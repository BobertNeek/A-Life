//! Separate residual experiment: fixed public scene preparation, real sampled
//! consequences, train-only delta RMS scaling and whole-world heldout splits.
use super::*;
use crate::cpu::predictive::{
    artifact::{identity, ExperimentalPredictiveState, PredictiveRun},
    recording::*,
};
use alife_core::Validate;
use std::{borrow::Cow, collections::BTreeMap};

pub type ResidualResult<T> = Result<T, Box<dyn std::error::Error>>;
pub mod motion_coverage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResidualConfig {
    #[serde(default)]
    pub motion_coverage: bool,
    pub seed: u64,
    pub world_ticks: usize,
    pub temperature: f32,
    pub horizons: Vec<u16>,
    pub epochs: usize,
    pub head_learning_rate: f32,
    pub native_learning_rate: f32,
    pub scale_floor: f32,
}
impl Default for ResidualConfig {
    fn default() -> Self {
        Self {
            motion_coverage: false,
            seed: 71,
            world_ticks: 32,
            temperature: 4.,
            horizons: vec![1, 4],
            epochs: 16,
            head_learning_rate: 0.03,
            native_learning_rate: 0.0001,
            scale_floor: 0.001,
        }
    }
}

/// Geometry is prepared before any observation/action, using public world APIs.
/// No recorded outcome, reward, current perception or action is overwritten.
pub fn capture_diverse(config: &ResidualConfig) -> ResidualResult<Vec<PublicSimulationRecording>> {
    if config.motion_coverage {
        return motion_coverage::capture(config);
    }
    if !(5..=128).contains(&config.world_ticks) || config.seed == 0 {
        return Err("invalid residual capture configuration".into());
    }
    let mut captures = Vec::new();
    for scene in 0..3 {
        let base = config
            .seed
            .checked_add(1000 + scene * 10)
            .ok_or("seed overflow")?;
        let capture = capture_prepared_public(
            PublicCaptureConfig {
                founder_seed: config.seed,
                world_seeds: [base + 1, base + 2, base + 3],
                sampling_seeds: [
                    0x6a095101 + scene as u32,
                    0xbb675202 + scene as u32,
                    0x3c6e5303 + scene as u32,
                ],
                world_ticks: config.world_ticks,
            },
            Some(PublicPreparation {
                scenario: format!("residual-public-geometry-v1-scene{scene}"),
                temperature: config.temperature,
            }),
            |variation, game| {
                let agent = game
                    .world
                    .object_snapshots()
                    .into_iter()
                    .find(|o| o.kind == alife_world::WorldObjectKind::Agent)
                    .ok_or("public scene lacks agent")?
                    .position;
                let angle = [-0.6_f32, 0.25, 0.9][variation];
                let distance = [0.35_f32, 2., 4.][scene as usize];
                let at = |x: f32, y: f32| alife_core::Vec3f::new(agent.x + x, agent.y + y, agent.z);
                let food = game
                    .world
                    .entity_id("food-01")
                    .ok_or("missing public food")?;
                game.world
                    .editor_move_object(food, at(distance * angle.cos(), distance * angle.sin()))?;
                let ball = game
                    .world
                    .entity_id("meadow-ball")
                    .ok_or("missing public ball")?;
                game.world
                    .editor_move_object(ball, at(0.8 + 0.2 * variation as f32, -0.5))?;
                let obstacle = game
                    .world
                    .entity_id("obstacle-01")
                    .ok_or("missing public obstacle")?;
                game.world
                    .editor_move_object(obstacle, at(if scene == 1 { 1.2 } else { 6. }, 1.4))?;
                let hazard = game
                    .world
                    .entity_id("hazard-01")
                    .ok_or("missing public hazard")?;
                game.world.editor_move_object(
                    hazard,
                    at(
                        if scene == 2 { -0.9 } else { -6. },
                        0.6 + 0.2 * variation as f32,
                    ),
                )?;
                Ok(())
            },
        )?;
        if captures
            .first()
            .is_some_and(|first: &PublicSimulationRecording| {
                identity(&first.source).ok() != identity(&capture.source).ok()
            })
        {
            return Err("diverse worlds changed the frozen source".into());
        }
        captures.push(capture);
    }
    Ok(captures)
}

pub fn horizon_dataset(
    captures: &[PublicSimulationRecording],
    horizon: u16,
) -> Result<PredictiveDataset, TrainingError> {
    if captures.is_empty() || horizon == 0 || horizon > 32 {
        return Err(invalid());
    }
    let mut episodes = Vec::new();
    let mut schema = None;
    let mut all_worlds = BTreeSet::new();
    for capture in captures {
        let (base, _) = convert_recording(capture)?;
        if let Some(previous) = &schema {
            if previous != &base.schema {
                return Err(invalid());
            }
        } else {
            schema = Some(base.schema.clone());
        }
        let index = base
            .episodes
            .iter()
            .map(|e| {
                (
                    (&e.split_group, e.evidence.as_ref().unwrap().source_sequence),
                    e,
                )
            })
            .collect::<BTreeMap<_, _>>();
        for group in &capture.groups {
            if !all_worlds.insert(group.world_seed) {
                return Err(invalid());
            }
            for window in group.decisions.windows(horizon as usize + 1) {
                if window.windows(2).any(|p| {
                    p[0].patch.pre_action().tick.raw().checked_add(1)
                        != Some(p[1].patch.pre_action().tick.raw())
                        || p[0].patch.pre_action().sequence_id.raw().checked_add(1)
                            != Some(p[1].patch.pre_action().sequence_id.raw())
                }) {
                    continue;
                }
                let source = &window[0];
                let target = &window[horizon as usize];
                let Some(base) = index.get(&(
                    &group.collection_id,
                    source.patch.pre_action().sequence_id.raw(),
                )) else {
                    continue;
                };
                target.patch.validate_contract()?;
                let mut e = (*base).clone();
                e.episode_id = format!("{}-h{horizon}", e.episode_id);
                e.observations[1] = observation(target);
                let evidence = e.evidence.as_mut().unwrap();
                evidence.horizon_ticks = horizon;
                evidence.target_sequence = target.patch.pre_action().sequence_id.raw();
                evidence.target_tick = target.patch.pre_action().tick.raw();
                evidence.target_frame = target.patch.pre_action().frame_digest()?.0;
                evidence.target_patch_sha256 = identity(&target.patch)?;
                episodes.push(e);
            }
        }
    }
    let mut schema = schema.ok_or_else(invalid)?;
    schema.version = format!("{}-forecast-h{horizon}", schema.version);
    let data=PredictiveDataset {schema_version:1,source_description:format!("Actual diverse public simulation observations at horizon {horizon}; future actions follow the frozen collection policy and are not predictor inputs"),schema,episodes};
    let p = captures[0].source.clone().admit()?.phenotype;
    data.validate_for(&p)?;
    Ok(data)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeltaScaling {
    pub scales: Vec<f32>,
    pub train_delta_rms: Vec<f32>,
    pub train_pairs: usize,
    pub scale_floor: f32,
}
pub fn fit_training_scales(
    data: &PredictiveDataset,
    floor: f32,
) -> Result<DeltaScaling, TrainingError> {
    if !floor.is_finite() || floor <= 0. {
        return Err(invalid());
    }
    let width = data.schema.feature_count();
    let mut sums = vec![0_f64; width];
    let mut count = 0;
    for e in data
        .episodes
        .iter()
        .filter(|e| e.split == PredictiveSplit::Train)
    {
        if e.observations.len() != e.replay.ticks.len() + 1
            || e.observations.iter().any(|o| o.len() != width)
        {
            return Err(invalid());
        }
        for t in e.replay.burn_in_ticks..e.replay.ticks.len() {
            for (i, (a, b)) in e.observations[t]
                .iter()
                .zip(&e.observations[t + 1])
                .enumerate()
            {
                sums[i] += (f64::from(*b) - f64::from(*a)).powi(2);
            }
            count += 1;
        }
    }
    if count == 0 {
        return Err(invalid());
    }
    let rms = sums
        .iter()
        .map(|s| (s / count as f64).sqrt() as f32)
        .collect::<Vec<_>>();
    finite(&rms)?;
    Ok(DeltaScaling {
        scales: rms.iter().map(|s| s.max(floor)).collect(),
        train_delta_rms: rms,
        train_pairs: count,
        scale_floor: floor,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct ResidualScore {
    pub pairs: usize,
    pub raw_mse: f64,
    pub scaled_mse: f64,
    pub sensory_raw_mse: f64,
    pub body_raw_mse: f64,
    pub per_channel_raw_mse: Vec<f64>,
}

fn split_indices(data: &PredictiveDataset, split: PredictiveSplit) -> Vec<usize> {
    data.episodes
        .iter()
        .enumerate()
        .filter(|(_, e)| e.split == split)
        .map(|(i, _)| i)
        .collect()
}
fn next(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}
fn shuffled_order(mut indices: Vec<usize>, seed: u64) -> Vec<usize> {
    let mut state = seed;
    for i in (1..indices.len()).rev() {
        let j = next(&mut state) as usize % (i + 1);
        indices.swap(i, j);
    }
    indices
}
fn action_permutation(data: &PredictiveDataset, seed: u64) -> Vec<usize> {
    let mut permutation = (0..data.episodes.len()).collect::<Vec<_>>();
    for (salt, split) in [
        PredictiveSplit::Train,
        PredictiveSplit::Validation,
        PredictiveSplit::Test,
    ]
    .into_iter()
    .enumerate()
    {
        let order = shuffled_order(split_indices(data, split), seed ^ salt as u64);
        if order.len() > 1 {
            for i in 0..order.len() {
                permutation[order[i]] = order[(i + 1) % order.len()];
            }
        }
    }
    permutation
}
fn conditioned<'a>(
    data: &'a PredictiveDataset,
    index: usize,
    inputs: PredictiveInputs,
    permutation: &[usize],
) -> Cow<'a, PredictiveEpisode> {
    let e = &data.episodes[index];
    if inputs != PredictiveInputs::ShuffledAction {
        return Cow::Borrowed(e);
    }
    let mut changed = e.clone();
    let other = &data.episodes[permutation[index]];
    changed.actions.clone_from(&other.actions);
    changed.motor_values.clone_from(&other.motor_values);
    Cow::Owned(changed)
}

fn score(
    trainer: &CpuFoundationTrainer,
    head: Option<&PredictiveHead>,
    data: &PredictiveDataset,
    split: PredictiveSplit,
    scales: &[f32],
    inputs: PredictiveInputs,
    permutation: &[usize],
) -> Result<ResidualScore, TrainingError> {
    let width = data.schema.feature_count();
    let mut per = vec![0_f64; width];
    let mut scaled = 0.;
    let mut count = 0;
    for index in split_indices(data, split) {
        let e = conditioned(data, index, inputs, permutation);
        let prediction = if let Some(h) = head {
            let evaluation = trainer.evaluate_replay(&e.replay)?;
            h.pass(&e, &evaluation.final_activations, inputs)?
                .predictions
                .remove(0)
        } else {
            e.observations[0].clone()
        };
        for i in 0..width {
            let error = f64::from(prediction[i]) - f64::from(e.observations[1][i]);
            per[i] += error * error;
            scaled += (error / f64::from(scales[i])).powi(2);
        }
        count += 1;
    }
    if count == 0 {
        return Err(invalid());
    }
    for x in &mut per {
        *x /= count as f64;
    }
    let sensor = data.schema.sensors.len();
    Ok(ResidualScore {
        pairs: count,
        raw_mse: per.iter().sum::<f64>() / width as f64,
        scaled_mse: scaled / (count * width) as f64,
        sensory_raw_mse: per[..sensor].iter().sum::<f64>() / sensor as f64,
        body_raw_mse: per[sensor..].iter().sum::<f64>() / (width - sensor) as f64,
        per_channel_raw_mse: per,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct EpochScore {
    pub epoch: usize,
    pub train: ResidualScore,
    pub validation: ResidualScore,
}
#[derive(Debug, Clone, Serialize)]
pub struct ArmResult {
    pub inputs: PredictiveInputs,
    pub curve: Vec<EpochScore>,
    pub test: ResidualScore,
    pub optimizer_steps: u32,
    pub state_payload_sha256: [u8; 32],
    pub training_seconds: f64,
}
#[derive(Debug, Clone, Serialize)]
pub struct HorizonResult {
    pub horizon_ticks: u16,
    pub dataset_sha256: [u8; 32],
    pub scaling: DeltaScaling,
    pub channel_names: Vec<String>,
    pub persistence_train: ResidualScore,
    pub persistence_validation: ResidualScore,
    pub persistence_test: ResidualScore,
    pub shuffle_identity_sha256: [u8; 32],
    pub shuffled_action_changed_rows: usize,
    pub arms: Vec<ArmResult>,
}

pub fn run_horizon(
    config: &ResidualConfig,
    captures: &[PublicSimulationRecording],
    data: &PredictiveDataset,
    horizon: u16,
    output: &std::path::Path,
) -> ResidualResult<HorizonResult> {
    let scaling = fit_training_scales(data, config.scale_floor)?;
    let permutation = action_permutation(data, config.seed);
    let source = captures[0].source.clone();
    let admitted = source.clone().admit()?;
    let reference = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation,
        admitted.actor,
    )?;
    let dataset_sha256 = identity(data)?;
    let persistence_train = score(
        &reference,
        None,
        data,
        PredictiveSplit::Train,
        &scaling.scales,
        PredictiveInputs::Full,
        &permutation,
    )?;
    let persistence_validation = score(
        &reference,
        None,
        data,
        PredictiveSplit::Validation,
        &scaling.scales,
        PredictiveInputs::Full,
        &permutation,
    )?;
    let persistence_test = score(
        &reference,
        None,
        data,
        PredictiveSplit::Test,
        &scaling.scales,
        PredictiveInputs::Full,
        &permutation,
    )?;
    let mut arms = Vec::new();
    for inputs in [
        PredictiveInputs::Full,
        PredictiveInputs::ShuffledAction,
        PredictiveInputs::NoRecurrent,
    ] {
        let admitted = source.clone().admit()?;
        let mut checkpoint = admitted.actor;
        checkpoint.config.learning_rate = config.native_learning_rate;
        let mut trainer = CpuFoundationTrainer::from_checkpoint(
            admitted.phenotype,
            admitted.foundation,
            checkpoint,
        )?;
        let neurons = trainer
            .phenotype()
            .synapses()
            .iter()
            .filter(|s| s.kind() == CompiledSynapseKind::Recurrent)
            .map(|s| s.target() as usize)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .take(32)
            .collect();
        let mut head = PredictiveHead::new_residual(
            trainer.phenotype(),
            data.schema.clone(),
            neurons,
            16,
            config.seed,
            scaling.scales.clone(),
            horizon,
        )?;
        head.validate(&trainer, data)?;
        let mut curve = vec![EpochScore {
            epoch: 0,
            train: score(
                &trainer,
                Some(&head),
                data,
                PredictiveSplit::Train,
                &scaling.scales,
                inputs,
                &permutation,
            )?,
            validation: score(
                &trainer,
                Some(&head),
                data,
                PredictiveSplit::Validation,
                &scaling.scales,
                inputs,
                &permutation,
            )?,
        }];
        if curve[0].validation.raw_mse != persistence_validation.raw_mse
            || curve[0].train.raw_mse != persistence_train.raw_mse
        {
            return Err("residual initialization did not equal persistence".into());
        }
        let started = std::time::Instant::now();
        for epoch in 1..=config.epochs {
            for index in shuffled_order(
                split_indices(data, PredictiveSplit::Train),
                config.seed ^ epoch as u64,
            ) {
                let e = conditioned(data, index, inputs, &permutation);
                head.train_episode(&mut trainer, &e, config.head_learning_rate, inputs)?;
            }
            if epoch == 1 || epoch == 4 || epoch == config.epochs {
                curve.push(EpochScore {
                    epoch,
                    train: score(
                        &trainer,
                        Some(&head),
                        data,
                        PredictiveSplit::Train,
                        &scaling.scales,
                        inputs,
                        &permutation,
                    )?,
                    validation: score(
                        &trainer,
                        Some(&head),
                        data,
                        PredictiveSplit::Validation,
                        &scaling.scales,
                        inputs,
                        &permutation,
                    )?,
                });
            }
        }
        let training_seconds = started.elapsed().as_secs_f64();
        let test = score(
            &trainer,
            Some(&head),
            data,
            PredictiveSplit::Test,
            &scaling.scales,
            inputs,
            &permutation,
        )?;
        let run = PredictiveRun::from_dataset(
            config.seed,
            data,
            dataset_sha256,
            config.epochs,
            config.head_learning_rate,
            inputs,
        )?;
        let state =
            ExperimentalPredictiveState::capture(&trainer, &head, source.clone(), run, None)?;
        let label = match inputs {
            PredictiveInputs::Full => "full",
            PredictiveInputs::ShuffledAction => "shuffled_action",
            PredictiveInputs::NoRecurrent => "no_recurrent",
            _ => unreachable!(),
        };
        state.write_new(&output.join(format!("state-h{horizon}-{label}.json")))?;
        let restored = ExperimentalPredictiveState::read(
            &output.join(format!("state-h{horizon}-{label}.json")),
        )?;
        if restored.restore()?.0.checkpoint() != trainer.checkpoint()
            || restored.restore()?.1 != head
        {
            return Err("residual state failed exact restoration".into());
        }
        arms.push(ArmResult {
            inputs,
            curve,
            test,
            optimizer_steps: trainer.optimizer_step(),
            state_payload_sha256: state.payload_sha256,
            training_seconds,
        });
    }
    Ok(HorizonResult {
        horizon_ticks: horizon,
        dataset_sha256,
        scaling,
        channel_names: data
            .schema
            .sensors
            .iter()
            .chain(&data.schema.body)
            .cloned()
            .collect(),
        persistence_train,
        persistence_validation,
        persistence_test,
        shuffle_identity_sha256: identity(&permutation)?,
        shuffled_action_changed_rows: permutation
            .iter()
            .enumerate()
            .filter(|(i, j)| {
                data.episodes[*i].actions != data.episodes[**j].actions
                    || data.episodes[*i].motor_values != data.episodes[**j].motor_values
            })
            .count(),
        arms,
    })
}

pub fn action_diagnostics(captures: &[PublicSimulationRecording]) -> serde_json::Value {
    let mut primitive_counts = BTreeMap::<u32, usize>::new();
    let mut successes = 0;
    let mut failures = 0;
    let mut decisions = 0;
    for capture in captures {
        for group in &capture.groups {
            for d in &group.decisions {
                decisions += 1;
                if d.patch.outcome().success {
                    successes += 1;
                } else {
                    failures += 1;
                }
                if let Some(bundle) = d.patch.selected_bundle() {
                    for command in &bundle.channels {
                        *primitive_counts.entry(command.primitive.raw()).or_default() += 1;
                    }
                }
            }
        }
    }
    serde_json::json!({"actual_decisions":decisions,"world_success_receipts":successes,"world_failure_receipts":failures,"primitive_command_counts":primitive_counts})
}

#[cfg(test)]
mod tests;
