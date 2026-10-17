//! Bounded, explicitly historical/off-policy, frozen-feature GPU experiment.
//! Its output is not a campaign cycle or an admission to a live learner.
use crate::{
    load_foundation_replay_window, FoundationCycleReceipt, FoundationReplayBudget,
    FoundationReplayRecord, FoundationReplayRecordRef,
};
use alife_core::{
    BrainCapacityClass, BrainScaleTier, FoundationWeightAsset, PhenotypeCompiler, Tick,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_runtime::{GpuAuthoritativeSession, GpuSessionConsumerKind};
use alife_training::{
    AdamWConfig, CriticResidualRow, FoundationTrainer, FoundationTrainerCheckpoint,
    FrozenCriticResidualExperiment, PpoTrainingState, PpoValueHeadCheckpoint,
};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(serde::Deserialize)]
struct Corpus {
    directory: PathBuf,
    previous: PathBuf,
    fit: bool,
}
#[derive(serde::Deserialize)]
struct Manifest {
    checkpoint: PathBuf,
    corpus: Vec<Corpus>,
    steps: u32,
    learning_rate: f32,
}
fn hash(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn params_digest(values: &[f32]) -> String {
    hash(
        &values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )
}

fn metric_slice(rows: &[[f32; 4]]) -> serde_json::Value {
    let n = rows.len() as f64;
    let lower = -1731.7340972020208;
    let upper = 1731.9840972020208;
    serde_json::json!({"rows":rows.len(),"residual_mse":rows.iter().map(|r|f64::from(r[2]).powi(2)).sum::<f64>()/n,"residual_mean":rows.iter().map(|r|f64::from(r[2])).sum::<f64>()/n,"current_min":rows.iter().map(|r|r[0]).fold(f32::INFINITY,f32::min),"current_max":rows.iter().map(|r|r[0]).fold(f32::NEG_INFINITY,f32::max),"next_min":rows.iter().map(|r|r[1]).fold(f32::INFINITY,f32::min),"next_max":rows.iter().map(|r|r[1]).fold(f32::NEG_INFINITY,f32::max),"out_of_support_predictions":rows.iter().flat_map(|r|[r[0],r[1]]).filter(|v|f64::from(*v)<lower||f64::from(*v)>upper).count(),"prediction_count":rows.len()*2})
}
fn metrics(values: &[[f32; 4]], fit: usize) -> serde_json::Value {
    serde_json::json!({"fit":metric_slice(&values[..fit]),"heldout":metric_slice(&values[fit..])})
}

fn finite_fixture_close(
    actual: &[f32],
    expected: &[f32],
    tolerance: f32,
    label: &str,
) -> Result<()> {
    if actual.len() != expected.len()
        || !tolerance.is_finite()
        || tolerance < 0.0
        || actual
            .iter()
            .zip(expected)
            .any(|(a, b)| !a.is_finite() || !b.is_finite() || (a - b).abs() > tolerance)
    {
        return Err(format!("{label}: nonfinite or mismatched fixture values").into());
    }
    Ok(())
}

fn gpu_gradient_regression(session: &GpuAuthoritativeSession) -> Result<serde_json::Value> {
    // Analytic scalar fixture, developer test only, never a live neural shadow.
    let checkpoint = PpoValueHeadCheckpoint {
        feature_count: 2,
        optimizer_step: 7,
        parameters: vec![2., -1., 3., 0.1, 0.2, 0.3, 0.01, 0.02, 0.03],
        last_updated_policy_version: Some(5124),
    };
    let features = vec![vec![1., 2.], vec![4., -1.]];
    for discount in [0.99942255_f32, 0.5, 0.0] {
        let rows = vec![CriticResidualRow {
            current: 0,
            next: 1,
            reward: 0.25,
            discount,
        }];
        let probe = FrozenCriticResidualExperiment::new(session, &checkpoint, &features, &rows, 1)?;
        let v = probe.evaluate(session)?;
        let e = 3.0 - 0.25 - discount * 12.0;
        let expected = [
            e * (1.0 - discount * 4.0),
            e * (2.0 + discount),
            e * (1.0 - discount),
        ];
        let actual = probe.gradient_snapshot(session)?;
        finite_fixture_close(&actual, &expected, 0.0001, "GPU full-residual gradient")?;
        finite_fixture_close(
            &v[0],
            &[3.0, 12.0, e, discount],
            0.0001,
            "GPU residual forward",
        )?;
        let restored = probe.checkpoint(session)?;
        finite_fixture_close(
            &restored.parameters,
            &checkpoint.parameters,
            0.0,
            "GPU weights and Adam",
        )?;
        if restored != checkpoint {
            return Err("gradient evaluation changed critic/Adam".into());
        }
        let config = AdamWConfig::default();
        // The middle case remains unclipped, so omitting the loss coefficient
        // cannot accidentally pass through scale-invariant norm clipping.
        let coefficient = if discount == 0.5 { 0.01 } else { 0.5 };
        let mut updated = probe;
        updated.step_with_coefficient(session, config, coefficient)?;
        let actual = updated.checkpoint(session)?;
        let mut expected_state = checkpoint.parameters.clone();
        let scaled = expected.map(|g| g * coefficient);
        let norm = scaled.iter().map(|g| g * g).sum::<f32>().sqrt();
        let clip = (config.gradient_clip / norm.max(1e-20)).min(1.0);
        for p in 0..3 {
            let g = scaled[p] * clip;
            let m = config.beta1 * checkpoint.parameters[3 + p] + (1.0 - config.beta1) * g;
            let v = config.beta2 * checkpoint.parameters[6 + p] + (1.0 - config.beta2) * g * g;
            expected_state[p] -= config.learning_rate
                * (m / (1.0 - config.beta1.powi(8))
                    / ((v / (1.0 - config.beta2.powi(8))).sqrt() + config.epsilon)
                    + config.weight_decay * checkpoint.parameters[p]);
            expected_state[3 + p] = m;
            expected_state[6 + p] = v;
        }
        finite_fixture_close(
            &actual.parameters,
            &expected_state,
            0.00003,
            "continued residual AdamW weights/m/v",
        )?;
        if actual.optimizer_step != 8 {
            return Err("residual Adam age did not advance once".into());
        }
    }
    let overflow_features = vec![vec![f32::MAX, 2.0], vec![4.0, -1.0]];
    let overflow_rows = [CriticResidualRow {
        current: 0,
        next: 1,
        reward: 0.25,
        discount: 0.99942255,
    }];
    let mut rejected = FrozenCriticResidualExperiment::new(
        session,
        &checkpoint,
        &overflow_features,
        &overflow_rows,
        1,
    )?;
    if rejected.step(session, AdamWConfig::default()).is_ok()
        || rejected.checkpoint(session)? != checkpoint
    {
        return Err("nonfinite residual update mutated weights, moments, or age".into());
    }
    // A bad prospective final coordinate must not leave earlier coordinates
    // partially updated even when every input and the gradient are finite.
    let mut late_bad = checkpoint.clone();
    late_bad.parameters[5] = f32::MAX;
    let mut rejected =
        FrozenCriticResidualExperiment::new(session, &late_bad, &features, &overflow_rows, 1)?;
    if rejected.step(session, AdamWConfig::default()).is_ok()
        || rejected.checkpoint(session)? != late_bad
    {
        return Err("late-coordinate preflight rejection partially updated critic/Adam".into());
    }
    Ok(
        serde_json::json!({"passed":true,"discounts":[0.99942255,0.5,0.0],"checked":"current and next predictions; full feature gradient; bias e*(1-gamma); coefficient before clipping; AdamW weights/m/v and age; terminal gamma0; no-update checkpoint exact; rejected nonfinite update preserves every parameter and age"}),
    )
}

/// Bounded developer regression; no founder, world, or campaign checkpoint.
pub fn run_critic_gradient_regression() -> Result<serde_json::Value> {
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Training);
    gpu_gradient_regression(&session)
}

pub fn evaluate_critic_capture(
    directory: &Path,
    baseline_path: &Path,
    candidate_path: &Path,
    output: &Path,
) -> Result<serde_json::Value> {
    if output.exists() {
        return Err("heldout output must be new".into());
    }
    let receipt_bytes = std::fs::read(directory.join("critic-capture.json"))?;
    let receipt: serde_json::Value = serde_json::from_slice(&receipt_bytes)?;
    if receipt["purpose"] != "critic_capture_only"
        || receipt["actor_optimizer_updates"] != 0
        || receipt["value_optimizer_updates"] != 0
        || receipt["actor_policy"]
            .as_u64()
            .is_none_or(|policy| policy == 0)
        || receipt["actor_and_Adam_unchanged"] != true
        || receipt["critic_and_Adam_unchanged"] != true
        || receipt["feature_file"] != "critic-features.f32le"
    {
        return Err("heldout capture identity invalid".into());
    }
    let n = receipt["feature_count"]
        .as_u64()
        .ok_or("feature count missing")? as usize;
    let states = receipt["feature_states"]
        .as_u64()
        .ok_or("feature states missing")? as usize;
    let bytes = std::fs::read(directory.join("critic-features.f32le"))?;
    if n == 0
        || n > 16384
        || states > 4096
        || bytes.len() != n * states * 4
        || receipt["features_sha256"] != hash(&bytes)
    {
        return Err("heldout feature file identity invalid".into());
    }
    let floats = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .copied()
        .map(f32::from_le_bytes)
        .collect::<Vec<_>>();
    let features = floats
        .chunks_exact(n)
        .map(|r| r.to_vec())
        .collect::<Vec<_>>();
    let rows: Vec<CriticResidualRow> = serde_json::from_value(receipt["transitions"].clone())?;
    if rows.len() < 16 {
        return Err("heldout trajectory shorter than16 transitions".into());
    }
    let baseline: PpoValueHeadCheckpoint = read_json(baseline_path)?;
    let candidate: PpoValueHeadCheckpoint = read_json(candidate_path)?;
    if baseline.feature_count as usize != n || candidate.feature_count as usize != n {
        return Err("heldout head dimensions differ".into());
    }
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Training);
    let original =
        FrozenCriticResidualExperiment::new(&session, &baseline, &features, &rows, rows.len())?;
    let corrected =
        FrozenCriticResidualExperiment::new(&session, &candidate, &features, &rows, rows.len())?;
    let before = original.evaluate(&session)?;
    let after = corrected.evaluate(&session)?;
    if original.checkpoint(&session)? != baseline || corrected.checkpoint(&session)? != candidate {
        return Err("heldout evaluation mutated critic/Adam".into());
    }
    let mut parity = 0.0_f32;
    for (row, values) in receipt["transitions"]
        .as_array()
        .ok_or("transition array absent")?
        .iter()
        .zip(&before)
    {
        let current = row["current_value"]
            .as_f64()
            .ok_or("captured current value absent")? as f32;
        let next = if row["boundary"] == "Terminated" {
            values[1]
        } else {
            row["next_value"]
                .as_f64()
                .ok_or("captured next value absent")? as f32
        };
        parity = parity
            .max((values[0] - current).abs())
            .max((values[1] - next).abs());
    }
    if parity > 0.01 {
        return Err(format!(
            "production captured features and existing GPU replay head differ by {parity}"
        )
        .into());
    }
    let gae = |values: &[[f32; 4]]| -> Vec<f32> {
        let mut result = vec![0.; rows.len()];
        let mut tail = 0.0_f32;
        for i in (0..rows.len()).rev() {
            let elapsed = receipt["transitions"][i]["elapsed_seconds"]
                .as_f64()
                .unwrap() as f32;
            let lambda = (-std::f64::consts::LN_2 * f64::from(elapsed) / 10.0).exp() as f32;
            tail = -values[i][2] + rows[i].discount * lambda * tail;
            result[i] = tail;
        }
        result
    };
    let before_gae = gae(&before);
    let after_gae = gae(&after);
    let one = |values: &[[f32; 4]], advantages: &[f32]| serde_json::json!({"early16":metric_slice(&values[..16]),"whole_trajectory":metric_slice(values),"last16":metric_slice(&values[values.len()-16..]),"gae_min":advantages.iter().copied().fold(f32::INFINITY,f32::min),"gae_max":advantages.iter().copied().fold(f32::NEG_INFINITY,f32::max),"gae_positive":advantages.iter().filter(|v|**v>0.0).count()});
    let report = serde_json::json!({"schema":1,"seed":receipt["seed"],"capture":directory,"capture_sha256":hash(&receipt_bytes),"feature_sha256":hash(&bytes),"actor_policy":receipt["actor_policy"],"actor_source_sha256":receipt["source_file_sha256"],"rows":rows.len(),"source_actor_unchanged":receipt["actor_and_Adam_unchanged"],"normal_biology":receipt["normal_production_biology_and_lifetime_learning"],"no_fit_or_step_selection_from_this_seed":true,"baseline_age":baseline.optimizer_step,"candidate_age":candidate.optimizer_step,"baseline_value_path_parity_max_abs":parity,"before":one(&before,&before_gae),"after":one(&after,&after_gae),"before_values":before,"after_values":after,"before_gae":before_gae,"after_gae":after_gae,"limit":"Continuing finite trajectory has unknown real tail. Residual and range changes do not identify absolute return calibration."});
    std::fs::write(output, serde_json::to_vec_pretty(&report)?)?;
    Ok(
        serde_json::json!({"report":output,"seed":report["seed"],"before":report["before"],"after":report["after"],"parity_max_abs":parity}),
    )
}

pub fn run_critic_qualification(manifest_path: &Path, output: &Path) -> Result<serde_json::Value> {
    let started = Instant::now();
    let manifest: Manifest = read_json(manifest_path)?;
    if manifest.corpus.len() < 4
        || manifest.corpus.len() > 32
        || manifest.steps == 0
        || manifest.steps > 5000
        || !(0.00001..=0.003).contains(&manifest.learning_rate)
    {
        return Err("qualification bounds invalid".into());
    }
    if output.exists() {
        return Err("qualification output must be new".into());
    }
    std::fs::create_dir_all(output)?;
    let original: PpoValueHeadCheckpoint =
        read_json(&manifest.checkpoint.join("value-checkpoint.json"))?;
    let actor_bytes = std::fs::read(manifest.checkpoint.join("actor-checkpoint.json"))?;
    let actor: FoundationTrainerCheckpoint = serde_json::from_slice(&actor_bytes)?;
    let source_cycle: FoundationCycleReceipt = read_json(&manifest.checkpoint.join("cycle.json"))?;
    if source_cycle.policy_version != 5124
        || actor.optimizer_step != 12132
        || original.optimizer_step != 10248
    {
        return Err("unexpected pinned recovery state".into());
    }
    let foundation_bytes = std::fs::read(manifest.checkpoint.join("trained.alife-foundation"))?;
    let mut features = Vec::new();
    let mut reference_values = Vec::new();
    let mut fit_rows = Vec::new();
    let mut held_rows = Vec::new();
    let mut evidence = Vec::new();
    let mut seeds = std::collections::BTreeSet::new();
    for item in &manifest.corpus {
        let cycle: FoundationCycleReceipt = read_json(&item.directory.join("cycle.json"))?;
        if !seeds.insert(cycle.seed)
            || cycle.terminal_death_tick.is_some()
            || cycle.trained_replay_rows != 16
            || cycle.captured_replay_rows != 17
            || cycle.bootstrap_replay_rows != 1
            || cycle.curriculum_reward_rows.len() != 16
            || cycle.sleep_gap_reward_total != 0.0
        {
            return Err("corpus requires distinct sealed contiguous 16+1 Grab trajectories".into());
        }
        let refs: Vec<FoundationReplayRecordRef> =
            read_json(&item.directory.join("replay-manifest.json"))?;
        if refs.len() != 17
            || refs
                .windows(2)
                .any(|r| r[0].tick + 1 != r[1].tick || r[0].segment != r[1].segment)
        {
            return Err("corpus crosses actual gap".into());
        }
        let raw = std::fs::read(item.directory.join("replay").join(refs[0].file_name()))?;
        if blake3::hash(&raw).as_bytes() != &refs[0].digest {
            return Err("first replay digest mismatch".into());
        }
        let mut decoder = zstd::stream::read::Decoder::new(raw.as_slice())?.take(268435457);
        let first: FoundationReplayRecord = bincode::serde::decode_from_std_read(
            &mut decoder,
            bincode::config::standard().with_limit::<268435456>(),
        )?;
        let asset_bytes = std::fs::read(item.directory.join("initial.alife-foundation"))?;
        let asset = FoundationWeightAsset::decode_canonical(&asset_bytes)?;
        let previous_bytes = std::fs::read(item.previous.join("actor-checkpoint.json"))?;
        let previous: FoundationTrainerCheckpoint = serde_json::from_slice(&previous_bytes)?;
        if first.source.actor_checkpoint_digest.bytes() != blake3::hash(&previous_bytes).as_bytes()
            || first.source.foundation_asset_digest != asset.digest()
        {
            return Err("historical actor source binding mismatch".into());
        }
        let mut config = alife_world::CanonicalNewGameConfig::phase3(cycle.seed, 1)?;
        config.brain_class = BrainScaleTier::Standard2048;
        config.founder_seed_base = cycle.founder_seed_base;
        config.sensor_profile = asset.manifest().sensor_profile();
        let game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
        let record = game
            .world
            .organism_registry()
            .iter()
            .next()
            .ok_or("missing founder")?;
        let genome = record.phenotype().brain_genome.clone();
        let development = crate::gpu_live_runtime::foundation_construction_development(
            &genome,
            &BrainCapacityClass::n2048(),
            &record.phenotype().development_state_at(Tick::ZERO)?,
        )?;
        let (phenotype, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            asset.clone(),
        )?;
        let window = load_foundation_replay_window(
            &item.directory.join("replay"),
            &refs,
            0,
            &first.source,
            &phenotype,
            FoundationReplayBudget::default(),
        )?;
        let mut trainer = FoundationTrainer::new_required(
            phenotype,
            asset,
            previous.stage_mask.clone(),
            previous.config,
        )?;
        trainer.restore_checkpoint(&previous)?;
        let actor_before = trainer.checkpoint()?;
        let mut old_value = PpoTrainingState::from_checkpoint(original.clone())?;
        let baseline = old_value.predict_values(&mut trainer, &window.sequence)?;
        reference_values.extend(baseline.iter().copied());
        let evaluation = trainer.evaluate_replay(&window.sequence)?;
        if trainer.checkpoint()? != actor_before {
            return Err("GPU feature extraction mutated actor/Adam".into());
        }
        let offset = features.len() as u32;
        features.extend(evaluation.final_activations);
        let mut row_evidence = Vec::new();
        for i in 0..16 {
            let reward = &cycle.curriculum_reward_rows[i];
            if reward.row != i {
                return Err("reward identity mismatch".into());
            }
            let elapsed = (refs[i + 1].tick - refs[i].tick) as f32 / 20.0;
            let discount = (-std::f64::consts::LN_2 * f64::from(elapsed) / 60.0).exp() as f32;
            let row = CriticResidualRow {
                current: offset + i as u32,
                next: offset + i as u32 + 1,
                reward: reward.combined_reward,
                discount,
            };
            if item.fit {
                fit_rows.push(row);
            } else {
                held_rows.push(row);
            }
            row_evidence.push(serde_json::json!({"row":i,"reward":reward.combined_reward,"discount":discount,"elapsed_seconds":elapsed,"boundary":if i==15{"Truncated"}else{"Continuing"},"current_before":baseline[i],"next_before":baseline[i+1]}));
        }
        evidence.push(serde_json::json!({"directory":item.directory,"seed":cycle.seed,"fit":item.fit,"recorded_policy":cycle.policy_version,"recorded_initial_asset_sha256":hash(&asset_bytes),"recorded_actor_checkpoint_sha256":hash(&previous_bytes),"gpu_actor_checkpoint_unchanged":true,"rows":row_evidence}));
        std::fs::write(
            output.join("extraction-progress.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"completed_seeds":evidence.len(),"elapsed_seconds":started.elapsed().as_secs_f64()}),
            )?,
        )?;
    }
    if fit_rows.is_empty() || held_rows.is_empty() {
        return Err("whole-seed split empty".into());
    }
    let fit_count = fit_rows.len();
    let mut rows = fit_rows;
    rows.extend(held_rows);
    let feature_bytes = features
        .iter()
        .flatten()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<_>>();
    std::fs::write(output.join("actual-GPU-features.f32le"), &feature_bytes)?;
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let session = GpuAuthoritativeSession::new(backend, GpuSessionConsumerKind::Training);
    let gpu_regression = gpu_gradient_regression(&session)?;
    let mut experiment =
        FrozenCriticResidualExperiment::new(&session, &original, &features, &rows, fit_count)?;
    if experiment.checkpoint(&session)? != original {
        return Err("initial critic/Adam restore not exact".into());
    }
    let before_values = experiment.evaluate(&session)?;
    let mut max_prediction_difference = 0.0_f32;
    for (row, values) in rows.iter().zip(&before_values) {
        max_prediction_difference = max_prediction_difference
            .max((values[0] - reference_values[row.current as usize]).abs())
            .max((values[1] - reference_values[row.next as usize]).abs());
    }
    if max_prediction_difference > 0.01 {
        return Err(
            "cached feature value predictions differ from existing GPU PPO value path".into(),
        );
    }
    let before = metrics(&before_values, fit_count);
    let config = AdamWConfig {
        learning_rate: manifest.learning_rate,
        ..AdamWConfig::default()
    };
    let mut curve = vec![serde_json::json!({"critic_updates":0,"metrics":before})];
    for step in 1..=manifest.steps {
        experiment.step(&session, config)?;
        if step % 50 == 0 || step == manifest.steps {
            let scores = experiment.evaluate(&session)?;
            curve.push(
                serde_json::json!({"critic_updates":step,"metrics":metrics(&scores,fit_count)}),
            );
            std::fs::write(
                output.join("fit-progress.json"),
                serde_json::to_vec_pretty(&curve.last())?,
            )?;
        }
    }
    let after_values = experiment.evaluate(&session)?;
    let after = metrics(&after_values, fit_count);
    let candidate = experiment.checkpoint(&session)?;
    let cold =
        FrozenCriticResidualExperiment::new(&session, &candidate, &features, &rows, fit_count)?;
    if cold.checkpoint(&session)? != candidate || cold.evaluate(&session)? != after_values {
        return Err("critic cold restore changed values/Adam".into());
    }
    if actor_bytes != std::fs::read(manifest.checkpoint.join("actor-checkpoint.json"))?
        || foundation_bytes != std::fs::read(manifest.checkpoint.join("trained.alife-foundation"))?
    {
        return Err("pinned actor changed".into());
    }
    let width = original.feature_count as usize + 1;
    let report = serde_json::json!({"schema":1,"status":"historical_experiment_complete_not_live_qualified","candidate":"GPU full sampled one-step Bellman residual with frozen actual replay features","target_semantics":"V(s)-r-gamma(dt)*V(actual-next); continuing and truncated bootstrap retained; no artificial terminals or Monte Carlo labels","objective_support":[-1731.7340972020208,1731.9840972020208],"scope_limit":"Historical actor features from each sealed corpus, not fresh policy5124 trajectories. Short all-truncated residual improvement cannot establish constant-offset calibration or justify live adoption.","live_qualified":false,"source_checkpoint":manifest.checkpoint,"actor_policy":5124,"actor_optimizer_step":actor.optimizer_step,"actor_checkpoint_sha256":hash(&actor_bytes),"foundation_sha256":hash(&foundation_bytes),"actor_and_actor_Adam_byte_unchanged":true,"critic_before_age":original.optimizer_step,"critic_after_age":candidate.optimizer_step,"critic_only_updates":manifest.steps,"critic_Adam_reset":false,"initial_critic_weights_digest":params_digest(&original.parameters[..width]),"initial_critic_m_digest":params_digest(&original.parameters[width..width*2]),"initial_critic_v_digest":params_digest(&original.parameters[width*2..]),"final_critic_weights_digest":params_digest(&candidate.parameters[..width]),"final_critic_m_digest":params_digest(&candidate.parameters[width..width*2]),"final_critic_v_digest":params_digest(&candidate.parameters[width*2..]),"optimizer":config,"gpu_gradient_regression":gpu_regression,"before":before,"after":after,"curve":curve,"corpus":evidence,"feature_cache_sha256":hash(&feature_bytes),"feature_count":original.feature_count,"feature_states":features.len(),"fit_rows":fit_count,"heldout_rows":rows.len()-fit_count,"feature_rows_authority":"One exact production-graph GPU replay per historical saved window; source actor weights and full optimizer verified unchanged around extraction. No CPU neural execution.","wall_seconds":started.elapsed().as_secs_f64()});
    std::fs::write(
        output.join("candidate-value-checkpoint.json"),
        serde_json::to_vec(&candidate)?,
    )?;
    std::fs::write(
        output.join("gpu-row-values-before.json"),
        serde_json::to_vec(&before_values)?,
    )?;
    std::fs::write(
        output.join("gpu-row-values-after.json"),
        serde_json::to_vec(&after_values)?,
    )?;
    std::fs::write(
        output.join("transition-rows.json"),
        serde_json::to_vec(&rows)?,
    )?;
    std::fs::write(
        output.join("QUALIFICATION.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(
        serde_json::json!({"report":output.join("QUALIFICATION.json"),"live_qualified":false,"before":report["before"],"after":report["after"],"wall_seconds":report["wall_seconds"]}),
    )
}

#[cfg(test)]
mod fixture_tests {
    use super::finite_fixture_close;

    #[test]
    fn rejects_nonfinite_observed_or_expected_gradients_and_weights() {
        for label in ["gradient", "weights and Adam"] {
            for invalid in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                assert!(finite_fixture_close(&[invalid], &[1.0], 0.0001, label).is_err());
                assert!(finite_fixture_close(&[1.0], &[invalid], 0.0001, label).is_err());
            }
        }
    }

    #[test]
    fn checks_finite_tolerance_and_complete_dimensions() {
        assert!(finite_fixture_close(&[1.0, 2.0], &[1.00001, 2.0], 0.0001, "fixture").is_ok());
        assert!(finite_fixture_close(&[1.0], &[1.0], 0.0, "fixture").is_ok());
        assert!(finite_fixture_close(&[1.0], &[1.1], 0.0001, "fixture").is_err());
        assert!(finite_fixture_close(&[1.0, 2.0], &[1.0], 0.0001, "fixture").is_err());
        for invalid in [f32::NAN, f32::INFINITY, -1.0] {
            assert!(finite_fixture_close(&[1.0], &[1.0], invalid, "fixture").is_err());
        }
    }
}
