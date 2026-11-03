//! Fresh public canonical simulation capture and strict one-step conversion.
//! The founder remains frozen during collection. Complete native commands and
//! world execution/failure receipts are retained; outcomes never become inputs.
use super::artifact::identity;
use super::*;
use crate::{
    cpu::{CpuCanonicalHost, CpuCollectedDecision},
    AdamWConfig, FoundationTrainingBackendV1, FoundationTrainingHandoffV1, PpoValueHeadCheckpoint,
    StageTrainableMask,
};
use alife_core::{
    ActionKind, BrainCapacityClass, BrainScaleTier, FoundationWeightAsset, MotorCommandBundle,
    PhenotypeCompiler, SensorProfile, Tick, Validate,
};
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::{
    create_canonical_new_game_with_n2048_candidate, CanonicalNewGame, CanonicalNewGameConfig,
    HeadlessActionIds,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CausalPair {
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub horizon_ticks: u16,
    pub collection_id: String,
    pub organism_id: u64,
    pub source_sequence: u64,
    pub target_sequence: u64,
    pub source_tick: u64,
    pub outcome_tick: u64,
    pub target_tick: u64,
    pub source_frame: [u64; 4],
    pub target_frame: [u64; 4],
    pub source_patch_sha256: [u8; 32],
    pub target_patch_sha256: [u8; 32],
    pub motor_bundle_sha256: [u8; 32],
}
fn one() -> u16 {
    1
}
fn is_one(h: &u16) -> bool {
    *h == 1
}
impl CausalPair {
    pub(super) fn validate(&self, e: &PredictiveEpisode) -> Result<(), TrainingError> {
        if self.collection_id != e.split_group
            || self.organism_id == 0
            || self.horizon_ticks == 0
            || self.horizon_ticks > 32
            || self
                .source_sequence
                .checked_add(u64::from(self.horizon_ticks))
                != Some(self.target_sequence)
            || self.source_tick.checked_add(u64::from(self.horizon_ticks)) != Some(self.target_tick)
            || self.source_tick.checked_add(1) != Some(self.outcome_tick)
            || e.replay.ticks.len() != 1
            || e.replay.burn_in_ticks != 0
        {
            return Err(invalid());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCaptureConfig {
    pub founder_seed: u64,
    pub world_seeds: [u64; 3],
    pub sampling_seeds: [u32; 3],
    pub world_ticks: usize,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordedGroup {
    pub collection_id: String,
    pub split: PredictiveSplit,
    pub world_seed: u64,
    pub sampling_seed: u32,
    pub decisions: Vec<CpuCollectedDecision>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicSimulationRecording {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preparation: Option<PublicPreparation>,
    pub schema_version: u16,
    pub config: PublicCaptureConfig,
    pub source: FoundationTrainingHandoffV1,
    /// Public world inventory is fixed before collection, never fitted to targets.
    pub entity_inventory: Vec<u64>,
    pub groups: Vec<RecordedGroup>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicPreparation {
    pub scenario: String,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConversionMetrics {
    pub captured_decisions: usize,
    pub converted_pairs: usize,
    pub skipped_nonadjacent_pairs: usize,
    pub bootstrap_only_decisions: usize,
    pub native_replay_max_absolute_error: f32,
}

fn game(
    world_seed: u64,
    founder_seed: u64,
    asset: &FoundationWeightAsset,
) -> Result<CanonicalNewGame, Box<dyn std::error::Error>> {
    let mut config = CanonicalNewGameConfig::phase3(world_seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = founder_seed;
    config.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
    Ok(create_canonical_new_game_with_n2048_candidate(
        &config, asset,
    )?)
}

/// These are actual simulator steps with sampled native actions and canonical
/// physics/biology, starting from a procedural untrained founder, not a saved
/// trained creature. There are no scripted actions, made-up targets or rewards.
pub fn capture_public_simulation(
    config: PublicCaptureConfig,
) -> Result<PublicSimulationRecording, Box<dyn std::error::Error>> {
    capture_prepared_public(config, None, |_, _| Ok(()))
}

pub(super) fn capture_prepared_public(
    config: PublicCaptureConfig,
    preparation: Option<PublicPreparation>,
    mut setup: impl FnMut(usize, &mut CanonicalNewGame) -> Result<(), Box<dyn std::error::Error>>,
) -> Result<PublicSimulationRecording, Box<dyn std::error::Error>> {
    let temperature = preparation.as_ref().map_or(0.9, |p| p.temperature);
    if !temperature.is_finite() || temperature <= 0. || temperature > 16. {
        return Err("invalid collection temperature".into());
    }
    if config.founder_seed == 0
        || config.world_seeds.contains(&0)
        || config.world_seeds.iter().collect::<BTreeSet<_>>().len() != 3
        || config.sampling_seeds.iter().collect::<BTreeSet<_>>().len() != 3
        || !(3..=128).contains(&config.world_ticks)
    {
        return Err("invalid public capture configuration".into());
    }
    let builtin = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let initial = game(config.world_seeds[0], config.founder_seed, &builtin)?;
    let record = initial
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("missing founder")?;
    let genome = record.phenotype().brain_genome.clone();
    let capacity = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        &genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    let baseline = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )?;
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&baseline)?;
    let (p, inputs) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())?;
    let trainer = CpuFoundationTrainer::new(
        p.clone(),
        asset.clone(),
        StageTrainableMask::recurrent_only(&p)?,
        AdamWConfig::default(),
    )?;
    let source = FoundationTrainingHandoffV1::new(
        FoundationTrainingBackendV1::NativeCpu,
        1,
        inputs,
        trainer.checkpoint(),
        PpoValueHeadCheckpoint {
            feature_count: p.neuron_count(),
            optimizer_step: 0,
            parameters: vec![0.; (p.neuron_count() as usize + 1) * 3],
            last_updated_policy_version: Some(1),
        },
    )?;
    let mut worlds = Vec::new();
    let mut inventory = BTreeSet::new();
    for (i, &seed) in config.world_seeds.iter().enumerate() {
        let mut g = game(seed, config.founder_seed, &asset)?;
        setup(i, &mut g)?;
        inventory.extend(g.world.object_snapshots().iter().map(|o| o.id.raw()));
        worlds.push(g);
    }
    let frozen = trainer.checkpoint();
    let mut groups = Vec::new();
    for (i, world) in worlds.into_iter().enumerate() {
        let mut host = CpuCanonicalHost::new_cohort(world, &trainer, config.sampling_seeds[i])?;
        let mut decisions = Vec::new();
        for _ in 0..config.world_ticks {
            decisions.extend(host.step(&trainer, temperature)?.decisions);
        }
        groups.push(RecordedGroup {
            collection_id: format!(
                "public-world-{}-sampling-{}",
                config.world_seeds[i], config.sampling_seeds[i]
            ),
            split: [
                PredictiveSplit::Train,
                PredictiveSplit::Validation,
                PredictiveSplit::Test,
            ][i],
            world_seed: config.world_seeds[i],
            sampling_seed: config.sampling_seeds[i],
            decisions,
        });
    }
    if trainer.checkpoint() != frozen {
        return Err("collection changed the founder optimizer".into());
    }
    Ok(PublicSimulationRecording {
        preparation,
        schema_version: 1,
        config,
        source,
        entity_inventory: inventory.into_iter().collect(),
        groups,
    })
}

fn primitive_ids() -> Vec<u32> {
    (0..10)
        .map(|i| ActionKind::try_from_raw(i).unwrap().canonical_id().raw())
        .chain(
            [
                HeadlessActionIds::APPROACH,
                HeadlessActionIds::FLEE,
                HeadlessActionIds::NO_LOCOMOTION,
                HeadlessActionIds::NO_POSTURE,
                HeadlessActionIds::STEP_FORWARD,
                HeadlessActionIds::EAT,
                HeadlessActionIds::GRAB,
                HeadlessActionIds::NO_MANIPULATION,
                HeadlessActionIds::PLAY,
                HeadlessActionIds::LOOK_LEFT,
                HeadlessActionIds::LOOK_RIGHT,
                HeadlessActionIds::LOOK_CENTER,
                HeadlessActionIds::TURN_LEFT,
                HeadlessActionIds::TURN_RIGHT,
                HeadlessActionIds::HOLD_GAZE,
            ]
            .iter()
            .map(|a| a.raw()),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn motor_slot(channel: alife_core::MotorChannel) -> Result<usize, TrainingError> {
    match channel {
        alife_core::MotorChannel::Locomotion => Ok(0),
        alife_core::MotorChannel::Orientation => Ok(1),
        alife_core::MotorChannel::Manipulation => Ok(2),
        alife_core::MotorChannel::Vocal => Ok(3),
        alife_core::MotorChannel::Posture => Ok(4),
        _ => Err(invalid()),
    }
}
fn signed(x: f32) -> f32 {
    x / (1. + x.abs())
}
const MOTOR_NUMERIC: [&str; 12] = [
    "target_present",
    "target_position_present",
    "target_x",
    "target_y",
    "target_z",
    "direction_x",
    "direction_y",
    "direction_z",
    "intensity",
    "duration",
    "stand_off",
    "confidence",
];

pub fn public_schema(inventory: &[u64]) -> PredictiveSchema {
    let mut motors = Vec::new();
    let mut continuous = Vec::new();
    for slot in 0..5 {
        motors.push(MotorChannel {
            name: format!("motor{slot}.primitive"),
            choices: std::iter::once("absent".into())
                .chain(primitive_ids().iter().map(|id| format!("action-{id}")))
                .collect(),
        });
        motors.push(MotorChannel {
            name: format!("motor{slot}.entity"),
            choices: std::iter::once("none".into())
                .chain(inventory.iter().map(|id| format!("entity-{id}")))
                .collect(),
        });
        motors.push(MotorChannel {
            name: format!("motor{slot}.coordination"),
            choices: (0..8).map(|id| format!("group-{id}")).collect(),
        });
        continuous.extend(
            MOTOR_NUMERIC
                .iter()
                .map(|name| format!("motor{slot}.{name}")),
        );
    }
    motors.push(MotorChannel {
        name: "speech.act".into(),
        choices: std::iter::once("absent".into())
            .chain((0..8).map(|i| format!("speech-act-{i}")))
            .collect(),
    });
    motors.push(MotorChannel {
        name: "speech.prompted".into(),
        choices: vec!["absent".into(), "false".into(), "true".into()],
    });
    for i in 0..6 {
        motors.push(MotorChannel {
            name: format!("speech.token{i}"),
            choices: std::iter::once("absent".into())
                .chain((0..256).map(|t| format!("token-{t}")))
                .collect(),
        });
    }
    continuous.push("speech.confidence".into());
    for group in 0..8 {
        for slot in 0..5 {
            continuous.push(format!("coordination.group{group}.motor{slot}"));
        }
    }
    PredictiveSchema {
        version: "native-public-sensor-body-motor-v2-fixed-signed-unit".into(),
        sensors: (0..alife_core::SENSORY_ABI_CHANNEL_COUNT)
            .map(|i| format!("sensory-abi1-lane{i}"))
            .collect(),
        body: [
            "position_x",
            "position_y",
            "position_z",
            "rotation_x",
            "rotation_y",
            "rotation_z",
            "rotation_w",
            "velocity_x",
            "velocity_y",
            "velocity_z",
            "angular_x",
            "angular_y",
            "angular_z",
        ]
        .iter()
        .map(|s| format!("body.{s}"))
        .chain(
            (0..alife_core::DriveSnapshot::baseline().to_array().len())
                .map(|i| format!("drive.{i}")),
        )
        .chain(
            (0..alife_core::EndocrineSnapshot::baseline().to_array().len())
                .map(|i| format!("hormone.{i}")),
        )
        .collect(),
        motors,
        continuous_motors: continuous,
    }
}

pub(super) fn observation(d: &CpuCollectedDecision) -> Vec<f32> {
    let pre = d.patch.pre_action();
    let b = pre
        .body()
        .neural_projection(SensorProfile::GroundedObjectSlotsV1);
    let t = b.pose.translation;
    let r = b.pose.rotation;
    let v = b.velocity.linear;
    let a = b.velocity.angular;
    pre.sensory()
        .channels
        .as_flat_array()
        .into_iter()
        .chain(
            [
                t.x, t.y, t.z, r.x, r.y, r.z, r.w, v.x, v.y, v.z, a.x, a.y, a.z,
            ]
            .map(signed),
        )
        .chain(pre.homeostasis().drives.to_array())
        .chain(pre.homeostasis().hormones.to_array())
        .collect()
}

fn encode_bundle(
    bundle: &MotorCommandBundle,
    inventory: &[u64],
) -> Result<(Vec<usize>, Vec<f32>), TrainingError> {
    bundle.validate_contract()?;
    let primitives = primitive_ids();
    let mut categorical = vec![0; 23];
    let mut numeric = vec![0.; 101];
    for command in &bundle.channels {
        let slot = motor_slot(command.channel)?;
        categorical[slot * 3] = primitives
            .iter()
            .position(|&id| id == command.primitive.raw())
            .ok_or_else(invalid)?
            + 1;
        categorical[slot * 3 + 1] = command
            .target
            .and_then(|t| t.entity)
            .map(|id| {
                inventory
                    .iter()
                    .position(|i| *i == id.raw())
                    .map(|i| i + 1)
                    .ok_or_else(invalid)
            })
            .transpose()?
            .unwrap_or(0);
        if command.coordination_group >= 8 {
            return Err(invalid());
        }
        categorical[slot * 3 + 2] = command.coordination_group as usize;
        let position = command.target.and_then(|t| t.position);
        let p = position.unwrap_or(alife_core::Vec3f::ZERO);
        let d = command.direction;
        numeric[slot * 12..slot * 12 + 12].copy_from_slice(&[
            f32::from(command.target.is_some()),
            f32::from(position.is_some()),
            signed(p.x),
            signed(p.y),
            signed(p.z),
            signed(d.x),
            signed(d.y),
            signed(d.z),
            command.intensity.raw(),
            signed(command.duration_ticks.raw() as f32),
            signed(command.stand_off_distance),
            command.confidence.raw(),
        ]);
        let payload = &command.payload.values;
        if !payload.is_empty() {
            if slot != 3
                || !(4..=10).contains(&payload.len())
                || payload[0] != alife_core::VOCAL_CHANNEL_PAYLOAD_MAGIC_V1
                || payload[1] >= 8
                || payload[2] >= 2
                || payload[3] > 65535
                || payload[4..].iter().any(|&t| t >= 256)
            {
                return Err(invalid());
            }
            categorical[15] = payload[1] as usize + 1;
            categorical[16] = payload[2] as usize + 1;
            for (i, &token) in payload[4..].iter().enumerate() {
                categorical[17 + i] = token as usize + 1;
            }
            numeric[60] = payload[3] as f32 / 65535.;
        }
    }
    for group in &bundle.coordination.groups {
        if group.group_id >= 8 {
            return Err(invalid());
        }
        for &channel in &group.channels {
            numeric[61 + group.group_id as usize * 5 + motor_slot(channel)?] = 1.;
        }
    }
    Ok((categorical, numeric))
}

/// Use actual adjacent pre-action observations. Sleep/time gaps and missing
/// terminal observations are excluded, never filled with repeated targets.
/// Each example uses its exact native initial state and one replay tick, so
/// private-state changes between decisions are never silently chained in BPTT.
pub fn convert_recording(
    recording: &PublicSimulationRecording,
) -> Result<(PredictiveDataset, ConversionMetrics), TrainingError> {
    if recording.schema_version != 1
        || recording.groups.len() != 3
        || recording.source.actor.optimizer_step != 0
        || recording.source.value.optimizer_step != 0
        || recording.source.actor.update_ages.iter().any(|&a| a != 0)
        || recording
            .source
            .actor
            .first_moment
            .iter()
            .chain(&recording.source.actor.second_moment)
            .any(|&m| m != 0.)
        || recording.source.compiler_inputs.sensor_profile() != SensorProfile::GroundedObjectSlotsV1
        || recording.entity_inventory.is_empty()
        || recording.entity_inventory.len() > 511
        || recording.entity_inventory.windows(2).any(|w| w[0] >= w[1])
        || recording.config.founder_seed == 0
        || !(3..=128).contains(&recording.config.world_ticks)
    {
        return Err(invalid());
    }
    let source = recording.source.clone().admit()?;
    let trainer =
        CpuFoundationTrainer::from_checkpoint(source.phenotype, source.foundation, source.actor)?;
    let mut episodes = Vec::new();
    let mut stats = ConversionMetrics {
        captured_decisions: 0,
        converted_pairs: 0,
        skipped_nonadjacent_pairs: 0,
        bootstrap_only_decisions: 0,
        native_replay_max_absolute_error: 0.,
    };
    let mut worlds = BTreeSet::new();
    for (group_index, group) in recording.groups.iter().enumerate() {
        if group.world_seed != recording.config.world_seeds[group_index]
            || group.sampling_seed != recording.config.sampling_seeds[group_index]
            || group.split
                != [
                    PredictiveSplit::Train,
                    PredictiveSplit::Validation,
                    PredictiveSplit::Test,
                ][group_index]
            || group.collection_id
                != format!(
                    "public-world-{}-sampling-{}",
                    group.world_seed, group.sampling_seed
                )
            || group.decisions.len() > recording.config.world_ticks
        {
            return Err(invalid());
        }
        if !worlds.insert(group.world_seed) || group.decisions.len() < 2 {
            return Err(invalid());
        }
        stats.captured_decisions += group.decisions.len();
        stats.bootstrap_only_decisions += 1;
        for pair in group.decisions.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            a.patch.validate_contract()?;
            b.patch.validate_contract()?;
            a.action.validate(a.action.temperature)?;
            let pre = a.patch.pre_action();
            let next = b.patch.pre_action();
            if !matches!(pre.brain_evidence,alife_core::PreActionBrainEvidence::NeuralClosedLoopCpu { phenotype_hash,sensor_profile,.. } if phenotype_hash==trainer.phenotype().phenotype_hash() && sensor_profile==SensorProfile::GroundedObjectSlotsV1)
            {
                return Err(invalid());
            }
            if a.organism_id != b.organism_id
                || pre.sequence_id.raw().checked_add(1) != Some(next.sequence_id.raw())
            {
                return Err(invalid());
            }
            if pre.tick.raw().checked_add(1) != Some(next.tick.raw())
                || a.patch.outcome().outcome_tick != next.tick
            {
                stats.skipped_nonadjacent_pairs += 1;
                continue;
            }
            let bundle = a.patch.selected_bundle().ok_or_else(invalid)?;
            let target = a.patch.prediction_target().ok_or_else(invalid)?;
            if pre.policy_backend() != alife_core::PolicyBackend::NeuralClosedLoopCpu
                || bundle.tick != pre.tick
                || bundle.sequence_id != pre.sequence_id
                || bundle.organism_id != a.organism_id
                || target.motor_condition != alife_core::JointMotorCondition::from_bundle(bundle)?
                || a.patch.outcome().joint.is_none()
            {
                return Err(invalid());
            }
            let (action, values) = encode_bundle(bundle, &recording.entity_inventory)?;
            let replay = TrainingSequence {
                phenotype_hash: trainer.phenotype().phenotype_hash(),
                initial: a.initial.clone(),
                ticks: vec![a.replay_tick.clone()],
                burn_in_ticks: 0,
                memory_candidate_gain: trainer
                    .phenotype()
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0., |m| m.max_candidate_gain()),
            };
            let evaluated = trainer.evaluate_replay(&replay)?;
            if a.final_features.len() != trainer.phenotype().neuron_count() as usize {
                return Err(invalid());
            }
            let error = evaluated.final_activations[0]
                .iter()
                .zip(&a.final_features)
                .map(|(a, b)| (a - b).abs())
                .fold(0_f32, f32::max);
            if error > 1e-5 {
                return Err(invalid());
            }
            stats.native_replay_max_absolute_error =
                stats.native_replay_max_absolute_error.max(error);
            let evidence = CausalPair {
                horizon_ticks: 1,
                collection_id: group.collection_id.clone(),
                organism_id: a.organism_id.raw(),
                source_sequence: pre.sequence_id.raw(),
                target_sequence: next.sequence_id.raw(),
                source_tick: pre.tick.raw(),
                outcome_tick: a.patch.outcome().outcome_tick.raw(),
                target_tick: next.tick.raw(),
                source_frame: pre.frame_digest()?.0,
                target_frame: next.frame_digest()?.0,
                source_patch_sha256: identity(&a.patch)?,
                target_patch_sha256: identity(&b.patch)?,
                motor_bundle_sha256: identity(bundle)?,
            };
            episodes.push(PredictiveEpisode {
                episode_id: format!(
                    "{}-organism{}-sequence{}",
                    group.collection_id,
                    a.organism_id.raw(),
                    pre.sequence_id.raw()
                ),
                split_group: group.collection_id.clone(),
                split: group.split,
                replay,
                observations: vec![observation(a), observation(b)],
                actions: vec![action],
                motor_values: vec![values],
                evidence: Some(evidence),
            });
            stats.converted_pairs += 1;
        }
    }
    let data=PredictiveDataset { schema_version:1, source_description:"Fresh public canonical CPU simulation, procedural untrained founder, sampled native motor bundles, actual physics/biology and adjacent pre-action observations; one-tick detached examples".into(), schema:public_schema(&recording.entity_inventory),episodes };
    data.validate_for(trainer.phenotype())?;
    Ok((data, stats))
}
