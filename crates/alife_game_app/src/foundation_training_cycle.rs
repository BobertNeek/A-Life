//! A bounded, real-world actor update and next-cohort admission gate.
//! The ordinary GPU runtime owns perception, action, biology, and legality.

use std::{path::Path, time::Instant};

use alife_core::{
    BrainScaleTier, FoundationWeightAsset, ScaffoldContractError, SensorProfile,
    TrainingStageManifest,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile, GpuTrainingSamplingConfig};
use alife_training::{
    train_recurrent_ppo_cohort, AdamWConfig, FoundationTrainer, PpoBatch, PpoBoundary, PpoConfig,
    PpoJointAction, PpoTrainingState, PpoTrainingWindow, PpoTransition, StageTrainableMask,
};

use crate::{
    configure_foundation_scenario, foundation_replay_source, initial_n2048_care_asset,
    load_foundation_replay_window, verify_foundation_replay_step, FoundationReplayBudget,
    FoundationReplayWriter, FoundationTeacherLesson, FoundationWarmupReceipt,
    GpuDurableSaveManifest, GpuLiveBrainRuntime,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const BIOLOGICAL_OBJECTIVE_VERSION: u16 = 2;
fn legacy_objective_version() -> u16 {
    1
}

fn default_sampling_temperature() -> f32 {
    1.0
}

fn cycle_ppo_config(temperature: f32) -> Result<PpoConfig> {
    if !temperature.is_finite() || !(0.0001..=128.0).contains(&temperature) {
        return Err("cycle sampling temperature must be finite in 0.0001..=128".into());
    }
    let config = PpoConfig {
        temperature,
        ..PpoConfig::default()
    };
    config.validate()?;
    Ok(config)
}

fn cycle_sampling_config(
    seed: u64,
    policy: u64,
    temperature: f32,
) -> Result<GpuTrainingSamplingConfig> {
    let config = GpuTrainingSamplingConfig {
        seed: (seed as u32) ^ (policy as u32).wrapping_mul(0x9e37_79b9),
        counter: 0,
        temperature: cycle_ppo_config(temperature)?.temperature,
        demonstrator: None,
    };
    config.validate()?;
    Ok(config)
}

/// An objective change may retain learned state only when the caller names the
/// actual older version. Ordinary resume never silently resets either head.
fn validate_objective_continuation(
    source_version: u16,
    preserve_from: Option<u16>,
) -> Result<Option<u16>> {
    if source_version == BIOLOGICAL_OBJECTIVE_VERSION && preserve_from.is_none() {
        return Ok(None);
    }
    if source_version == legacy_objective_version()
        && BIOLOGICAL_OBJECTIVE_VERSION == 2
        && preserve_from == Some(source_version)
    {
        return Ok(Some(source_version));
    }
    Err(format!(
        "objective continuation blocked: source version {source_version}, current version \
         {BIOLOGICAL_OBJECTIVE_VERSION}; a supported older objective requires an explicit \
         --preserve-objective-state-from matching its receipt"
    )
    .into())
}

fn training_receptor_profile(
    phenotype: &alife_core::CreaturePhenotype,
) -> alife_core::PlasticityReceptorProfile {
    let genes = phenotype.brain_genome.plasticity_parameters();
    genes.action_candidate_credit_profile().map_or_else(
        || genes.receptor_profile(),
        |profile| profile.receptor_profile(),
    )
}

fn training_biological_value(
    transition: &alife_core::MeasuredPhysiologyTransition,
    phenotype: &alife_core::CreaturePhenotype,
) -> Result<f32> {
    let receptors = transition.before.neural_receptor_frame(phenotype)?;
    let sample = alife_core::NeuromodulatorSample::from_components(
        0.0,
        transition.aversive_value(),
        transition.homeostatic_improvement(),
        0.0,
        0.0,
    )?
    .with_biochemical_receptors(&receptors)?;
    Ok(training_receptor_profile(phenotype).project(&sample.frame())?)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundationCycleReceipt {
    #[serde(default = "legacy_objective_version")]
    pub biological_objective_version: u16,
    #[serde(default)]
    pub objective_state_reset: bool,
    #[serde(default)]
    pub objective_transition_from: Option<u16>,
    #[serde(default = "default_sampling_temperature")]
    pub sampling_temperature: f32,
    #[serde(default)]
    pub acquisition_rehearsal: Option<crate::FoundationAcquisitionRehearsalReceipt>,
    pub seed: u64,
    #[serde(default)]
    pub founder_seed_base: u64,
    pub policy_version: u64,
    pub training_ticks: usize,
    #[serde(default)]
    pub requested_training_ticks: usize,
    #[serde(default)]
    pub terminal_death_tick: Option<u64>,
    #[serde(default)]
    pub delayed_food_gate_passed: bool,
    #[serde(default)]
    pub world_ticks_elapsed: u64,
    #[serde(default)]
    pub consumed_events: u64,
    #[serde(default)]
    pub blocked_actions: u64,
    #[serde(default)]
    pub collision_actions: u64,
    #[serde(default)]
    pub avoid_actions: u64,
    #[serde(default)]
    pub rest_recovery_actions: u64,
    #[serde(default)]
    pub food_available_world_tick: Option<u64>,
    #[serde(default)]
    pub lesson: Option<FoundationTeacherLesson>,
    #[serde(default)]
    pub first_consumed_world_tick: Option<u64>,
    #[serde(default)]
    pub food_available_elapsed_seconds: Option<f64>,
    #[serde(default)]
    pub first_consumed_elapsed_seconds: Option<f64>,
    #[serde(default)]
    pub initial_energy: f32,
    #[serde(default)]
    pub minimum_energy: f32,
    #[serde(default)]
    pub final_energy: f32,
    #[serde(default)]
    pub sleep_gap_reward_total: f32,
    #[serde(default)]
    pub semantic_prior: Option<crate::gpu_live_runtime::SemanticPriorMetrics>,
    #[serde(default)]
    pub speech_target_rows: usize,
    #[serde(default)]
    pub teacher_cue_frames: usize,
    #[serde(default)]
    pub teacher_cue_tokens: Vec<u16>,
    #[serde(default)]
    pub held_food_setup: Option<crate::foundation_training::FoundationHeldFoodSetup>,
    #[serde(default)]
    pub grab_food_setup: Option<crate::FoundationGrabFoodSetup>,
    #[serde(default)]
    pub grab_food_acquisitions: Vec<crate::FoundationGrabAcquisition>,
    #[serde(default)]
    pub trained_grab_food_acquisitions: usize,
    #[serde(default)]
    pub prepared_grab_food_opportunities: usize,
    #[serde(default)]
    pub grab_food_curriculum_version: Option<u16>,
    #[serde(default)]
    pub curriculum_reward_rows: Vec<crate::FoundationCurriculumRewardRow>,
    #[serde(default)]
    pub curriculum_reward_total: f32,
    #[serde(default)]
    pub held_food_consumption_events: usize,
    /// Actual captured meals whose action row participates in the update.
    #[serde(default)]
    pub trained_held_food_consumption_events: usize,
    /// Fresh world/lifetime setup opportunities, not replay rows or epochs.
    #[serde(default)]
    pub prepared_held_food_opportunities: usize,
    #[serde(default)]
    pub captured_replay_rows: usize,
    #[serde(default)]
    pub trained_replay_rows: usize,
    #[serde(default)]
    pub bootstrap_replay_rows: usize,
    #[serde(default)]
    pub actor_optimizer_updates: u32,
    #[serde(default)]
    pub value_optimizer_updates: u32,
    pub collection_seconds: f64,
    pub update_seconds: f64,
    pub old_asset_digest: String,
    pub new_asset_digest: String,
    pub actor_optimizer_step: u32,
    pub value_optimizer_step: u32,
    pub completed_epochs: u32,
    pub next_cohort_tick_captured: bool,
    pub next_cohort_optimizer_rebound: bool,
}

fn digest(asset: &FoundationWeightAsset) -> String {
    asset
        .digest()
        .bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn cycle_replay_dose(
    captured_rows: usize,
    trained_rows: usize,
    meal_rows: &[usize],
) -> Result<(usize, usize)> {
    let bootstrap_rows = captured_rows
        .checked_sub(trained_rows)
        .ok_or("trained replay row count exceeds captured rows")?;
    if bootstrap_rows > 1
        || meal_rows.iter().any(|row| *row >= captured_rows)
        || meal_rows.windows(2).any(|rows| rows[0] >= rows[1])
    {
        return Err("cycle replay dose has inconsistent bootstrap or meal row evidence".into());
    }
    Ok((
        bootstrap_rows,
        meal_rows.iter().filter(|row| **row < trained_rows).count(),
    ))
}

fn validate_cycle_optimizer_handoff(
    receipt: &FoundationCycleReceipt,
    asset: &FoundationWeightAsset,
    actor: &alife_training::FoundationTrainerCheckpoint,
    value: &alife_training::PpoValueHeadCheckpoint,
) -> Result<()> {
    if !receipt.next_cohort_optimizer_rebound || !receipt.next_cohort_tick_captured {
        return Err("previous cycle is not an exact sealed cohort handoff".into());
    }
    if actor.source_foundation_digest != asset.digest()
        || actor.optimizer_step != receipt.actor_optimizer_step
        || value.optimizer_step != receipt.value_optimizer_step
        || actor.weights.len() != asset.weights().len()
        || actor
            .weights
            .iter()
            .zip(asset.weights())
            .any(|(trained, exported)| trained.to_bits() != exported.to_bits())
        || value.last_updated_policy_version != Some(receipt.policy_version)
    {
        return Err("previous optimizer/value checkpoint does not match exported actor".into());
    }
    Ok(())
}

struct SealedCycleContinuation {
    asset: FoundationWeightAsset,
    policy_version: u64,
    founder_seed_base: u64,
    actor: alife_training::FoundationTrainerCheckpoint,
    value: alife_training::PpoValueHeadCheckpoint,
    objective_transition_from: Option<u16>,
}

/// CPU decoding/admission only. No output, world, backend or optimizer is created
/// until the existing receipt and both saved training heads have been accepted.
fn load_cycle_continuation(
    previous: &Path,
    preserve_objective_from: Option<u16>,
) -> Result<SealedCycleContinuation> {
    let receipt: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(previous.join("cycle.json"))?)?;
    let objective_transition_from = validate_objective_continuation(
        receipt.biological_objective_version,
        preserve_objective_from,
    )?;
    let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
        previous.join("trained.alife-foundation"),
    )?)?;
    if previous.join("terrain-continuation.json").is_file() {
        validate_terrain_continuation_package(previous, preserve_objective_from)?;
    } else if digest(&asset) != receipt.new_asset_digest {
        return Err("previous exported asset does not match its receipt".into());
    }
    let actor = serde_json::from_slice(&std::fs::read(previous.join("actor-checkpoint.json"))?)?;
    let value = serde_json::from_slice(&std::fs::read(previous.join("value-checkpoint.json"))?)?;
    validate_cycle_optimizer_handoff(&receipt, &asset, &actor, &value)?;
    Ok(SealedCycleContinuation {
        asset,
        policy_version: receipt
            .policy_version
            .checked_add(1)
            .ok_or("policy version overflow")?,
        founder_seed_base: if receipt.founder_seed_base == 0 {
            receipt.seed
        } else {
            receipt.founder_seed_base
        },
        actor,
        value,
        objective_transition_from,
    })
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundationTerrainContinuationReceipt {
    pub schema_version: u16,
    pub source_directory: std::path::PathBuf,
    pub source_file_blake3: std::collections::BTreeMap<String, String>,
    pub source_asset_digest: String,
    pub target_asset_digest: String,
    pub target_phenotype_hash: alife_core::PhenotypeHash,
    pub actor_optimizer_step: u32,
    pub value_optimizer_step: u32,
    pub biological_objective_version: u16,
    pub preserved_weight_count: usize,
    pub optimizer_reset: bool,
    pub value_state_reset: bool,
    pub personal_lifetime_restored: bool,
    #[serde(default)]
    pub legacy_source_export: Option<std::path::PathBuf>,
    #[serde(default)]
    pub terrain_template: Option<std::path::PathBuf>,
    #[serde(default)]
    pub source_phenotype_blake3: Option<String>,
    #[serde(default)]
    pub explicit_interface_changes: Vec<String>,
}

fn current_choice_credit_transition(
    old: &serde_json::Value,
    new: &serde_json::Value,
    head: Option<&str>,
) -> bool {
    let mut expected = old.clone();
    expected["receptor_profile"] = serde_json::json!([0.0, -1.0, 1.0, -0.5, 0.2, 0.0, 0.5, -0.5]);
    old["receptor_profile"] == serde_json::json!([0.2, -1.0, 1.0, -0.5, 0.2, 0.0, 0.5, -0.5])
        && expected == *new
        && matches!(
            head,
            Some("ActionCandidate" | "MemoryContext" | "CognitiveContext")
        )
}

fn prepare_legacy_terrain_binding(
    source: &FoundationWeightAsset,
    actor: &alife_training::FoundationTrainerCheckpoint,
    legacy: &Path,
    template_directory: &Path,
    seed: u64,
    founder: u64,
) -> Result<(
    FoundationWeightAsset,
    alife_core::BrainPhenotype,
    Vec<String>,
)> {
    let export: serde_json::Value =
        serde_json::from_slice(&std::fs::read(legacy.join("export.json"))?)?;
    let old: serde_json::Value =
        serde_json::from_slice(&std::fs::read(legacy.join("source-phenotype.json"))?)?;
    if export["producer_commit"] != "d4eb7892c8263e980314828f4273c271acab6a41"
        || export["actor_identity_verified"] != true
        || export["actor_phenotype_hash"] != serde_json::to_value(actor.phenotype_hash)?
        || old["phenotype_hash"] != export["actor_phenotype_hash"]
        || export["source_asset_digest"] != serde_json::to_value(source.digest())?
    {
        return Err("legacy terrain export does not authenticate the original actor".into());
    }
    let template = FoundationWeightAsset::decode_canonical(&std::fs::read(
        template_directory.join("trained.alife-foundation"),
    )?)?;
    if template.manifest().sensor_profile() != SensorProfile::GroundedTerrainVisionV1
        || template.weights().len() != source.weights().len()
        || template
            .weights()
            .iter()
            .zip(source.weights())
            .any(|(a, b)| a.to_bits() != b.to_bits())
    {
        return Err("terrain metadata template lacks the exact established trained weights".into());
    }
    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = founder;
    config.sensor_profile = template.manifest().sensor_profile();
    let game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &template)
        .map_err(|e| format!("trained terrain template admission: {e}"))?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("terrain founder missing")?;
    let genome = record.phenotype().brain_genome.clone();
    let development = crate::gpu_live_runtime::foundation_construction_development(
        &genome,
        &alife_core::BrainCapacityClass::n2048(),
        &record
            .phenotype()
            .development_state_at(alife_core::Tick::ZERO)?,
    )?;
    let (compiled, _) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development.clone(),
        template,
    )?;
    let target = FoundationWeightAsset::from_trained_weights(
        &compiled,
        source.weights().to_vec(),
        source.manifest().training_stage(),
    )?;
    let (next, _) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome,
        development,
        target.clone(),
    )?;
    // Serialize typed f32 fields before parsing, exactly as the source export
    // did. to_value widens f32 to f64 and invents decimal representation diffs.
    let new: serde_json::Value = serde_json::from_slice(&serde_json::to_vec(&next)?)?;
    // Receptor indices address a compiler-local table, not learned coordinates.
    // Preserve every synapse ordinal and require its resolved receptor parameters
    // to be identical before accepting any table relocation.
    let old_synapses = old["synapses"]
        .as_array()
        .ok_or("legacy synapses missing")?;
    let new_synapses = new["synapses"]
        .as_array()
        .ok_or("terrain synapses missing")?;
    let old_receptors = old["plasticity_receptors"]
        .as_array()
        .ok_or("legacy receptors missing")?;
    let new_receptors = new["plasticity_receptors"]
        .as_array()
        .ok_or("terrain receptors missing")?;
    if old_synapses.len() != new_synapses.len() {
        return Err("legacy terrain continuation changed synapse count".into());
    }
    let mut receptor_rebindings = std::collections::BTreeSet::new();
    let mut current_credit_readouts = 0_usize;
    for (index, (a, b)) in old_synapses.iter().zip(new_synapses).enumerate() {
        let ai = a["receptor_index"]
            .as_u64()
            .ok_or("legacy receptor index missing")? as usize;
        let bi = b["receptor_index"]
            .as_u64()
            .ok_or("terrain receptor index missing")? as usize;
        let ap = old_receptors
            .get(ai)
            .ok_or("legacy receptor index out of bounds")?;
        let bp = new_receptors
            .get(bi)
            .ok_or("terrain receptor index out of bounds")?;
        if ap != bp {
            // The merged 163457ca founder default deliberately removes surprise
            // reinforcement from choice readouts. Retain this current learning
            // rule while preserving accumulated weights and optimizer history.
            // Admit only that exact, documented coefficient change on its named
            // heads; every rate, bound and other receptor lane must still match.
            let head = a["kind"]["Decoder"]["head"].as_str();
            if !current_choice_credit_transition(ap, bp, head) {
                return Err(format!("legacy terrain continuation changed resolved receptor at synapse {index} ({ai} -> {bi}): source {ap}; target {bp}").into());
            }
            current_credit_readouts += 1;
        }
        let mut a = a.clone();
        let mut b = b.clone();
        a.as_object_mut()
            .ok_or("legacy synapse invalid")?
            .remove("receptor_index");
        b.as_object_mut()
            .ok_or("terrain synapse invalid")?
            .remove("receptor_index");
        if a != b {
            return Err(format!("legacy terrain continuation changed synapse coordinate {index}: source {a}; target {b}").into());
        }
        if ai != bi {
            receptor_rebindings.insert((ai, bi));
        }
    }
    for field in [
        "language_codebook",
        "brain_class_id",
        "neuron_count",
        "microstep_count",
        "lobe_layout",
        "projections",
        "neuron_dynamics",
        "cognitive_architecture",
        "replay_capture_plan",
        "sleep_consolidation_plan",
        "persistent_address_map",
        "route_abi_digest",
        "plasticity_abi_digest",
        "budgets",
        "speech_decoder",
        "memory_decoder",
    ] {
        if old[field] != new[field] {
            if let (Some(a), Some(b)) = (old[field].as_array(), new[field].as_array()) {
                if let Some(index) = a.iter().zip(b).position(|(x, y)| x != y) {
                    return Err(format!("legacy terrain continuation changed ordered neural field {field} index {index}: source {}; target {}", a[index], b[index]).into());
                }
            }
            return Err(format!(
                "legacy terrain continuation changed ordered neural field {field}"
            )
            .into());
        }
    }
    let old_assignments = old["sensor_encoder"]["assignments"]
        .as_array()
        .ok_or("legacy encoder missing")?;
    let new_assignments = new["sensor_encoder"]["assignments"]
        .as_array()
        .ok_or("terrain encoder missing")?;
    let changed: Vec<_> = old_assignments
        .iter()
        .filter(|a| !new_assignments.contains(a))
        .collect();
    if changed.iter().any(|a| {
        !matches!(
            a["source_group"].as_str(),
            Some("SensoryChannel" | "Body" | "Homeostasis")
        )
    }) {
        return Err("legacy terrain continuation changed an encoder binding outside the documented physical input coverage repair".into());
    }
    let mut old_decoder = old["decoder"].clone();
    let mut new_decoder = new["decoder"].clone();
    let mut changes = vec![
        "sensor_profile: grounded_object_slots_v1 -> grounded_terrain_vision_v1".to_owned(),
        "new hearing/private-prior encoder ports".to_owned(),
    ];
    if !changed.is_empty() {
        changes.push(format!("current physical input coverage from merged 163457ca replaces {} old sensory/body/homeostasis bindings; learned neural/critic coordinates retained, pre-update policy behavior may change", changed.len()));
    }
    // Keep the actual physical feature -> neuron maps, rather than describing
    // the intentional input repair as an invented bijective neural remapping.
    changes.push(format!(
        "source_sensor_encoder_assignments={}",
        serde_json::to_string(old_assignments)?
    ));
    changes.push(format!(
        "target_sensor_encoder_assignments={}",
        serde_json::to_string(new_assignments)?
    ));
    if current_credit_readouts != 0 {
        changes.push(format!("current SignedChoiceReadouts from merged 163457ca: surprise coefficient 0.2 -> 0.0 on {current_credit_readouts} action/memory/cognitive readout synapses; all other receptor parameters and accumulated learned/optimizer state retained"));
    }
    for (old_index, new_index) in receptor_rebindings {
        changes.push(format!("receptor table index {old_index} -> {new_index}; identical resolved parameters, synapse ordinals and optimizer arrays retained"));
    }
    if old["plasticity_plan_digest"] != new["plasticity_plan_digest"] {
        // This digest includes the entire compiler-local table, including unused
        // entries. All used entries and both replay/sleep plans were checked above.
        changes.push(
            "plasticity plan digest rebound after full per-synapse receptor validation and the explicit current credit transition"
                .to_owned(),
        );
    }
    for decoder in [&mut old_decoder, &mut new_decoder] {
        decoder
            .as_object_mut()
            .ok_or("decoder missing")?
            .remove("canonical_digest");
        for family in decoder["families"]
            .as_array_mut()
            .ok_or("decoder families missing")?
        {
            for field in [
                "innate_drive_mask",
                "innate_gain",
                "innate_cue_lane",
                "innate_cue_inverted",
                "innate_requires_reach",
            ] {
                family
                    .as_object_mut()
                    .ok_or("decoder family missing")?
                    .remove(field);
            }
        }
    }
    if old_decoder != new_decoder {
        return Err(
            "legacy terrain continuation changed action readout coordinates or bias".into(),
        );
    }
    if old["decoder"] != new["decoder"] {
        changes.push("current gene-compiled innate decoder dispositions; learned readout coordinates and bias retained".to_owned());
    }
    Ok((target, next, changes))
}

fn verify_terrain_coordinates(
    old: &alife_core::BrainPhenotype,
    next: &alife_core::BrainPhenotype,
) -> Result<()> {
    if old.sensor_profile() != SensorProfile::GroundedObjectSlotsV1
        || next.sensor_profile() != SensorProfile::GroundedTerrainVisionV1
        || old.schema_version() != next.schema_version()
        || old.brain_class_id() != next.brain_class_id()
        || old.neuron_count() != next.neuron_count()
        || old.microstep_count() != next.microstep_count()
        || old.lobe_layout() != next.lobe_layout()
        || old.language_codebook() != next.language_codebook()
        || old.cognitive_architecture() != next.cognitive_architecture()
        || old.projections() != next.projections()
        || old.neuron_dynamics() != next.neuron_dynamics()
        || old.candidate_decoder() != next.candidate_decoder()
        || old.speech_decoder() != next.speech_decoder()
        || old.memory_decoder() != next.memory_decoder()
        || old.cognitive_decoder() != next.cognitive_decoder()
        || old.cognitive_channel_plan() != next.cognitive_channel_plan()
        || old.plasticity_receptors() != next.plasticity_receptors()
        || old.replay_capture_plan() != next.replay_capture_plan()
        || old.sleep_consolidation_plan() != next.sleep_consolidation_plan()
        || old.plasticity_plan_digest() != next.plasticity_plan_digest()
        || old.persistent_address_map() != next.persistent_address_map()
        || old.route_abi_digest() != next.route_abi_digest()
        || old.plasticity_abi_digest() != next.plasticity_abi_digest()
        || old.budgets() != next.budgets()
        || old.synapses().len() != next.synapses().len()
        || old.synapses().iter().zip(next.synapses()).any(|(a, b)| {
            a.source() != b.source()
                || a.target() != b.target()
                || a.alpha().to_bits() != b.alpha().to_bits()
                || a.route_index() != b.route_index()
                || a.receptor_index() != b.receptor_index()
                || a.kind() != b.kind()
                || a.genetic_weight().to_bits() != b.genetic_weight().to_bits()
        })
    {
        return Err(
            "terrain continuation changed neural, synapse or ordered critic coordinates".into(),
        );
    }
    // Preserve every existing sensory/body binding. Only the explicitly named
    // hearing/private-prior ports may be added to the same learned neurons.
    let old_assignments = old.sensor_encoder().assignments();
    let next_assignments = next.sensor_encoder().assignments();
    if old_assignments
        .iter()
        .any(|a| !next_assignments.contains(a))
        || next_assignments.iter().any(|a| {
            !old_assignments.contains(a)
                && !matches!(
                    a.source_group(),
                    alife_core::SensorEncoderSourceGroup::HeardLanguage
                        | alife_core::SensorEncoderSourceGroup::SemanticPrior
                )
        })
    {
        return Err("terrain continuation changed existing encoder coordinates".into());
    }
    Ok(())
}

fn prepare_terrain_continuation(
    previous: &Path,
    preserve_from: Option<u16>,
    legacy: Option<&Path>,
    terrain_template: Option<&Path>,
) -> Result<(
    FoundationWeightAsset,
    alife_training::FoundationTrainerCheckpoint,
    alife_training::PpoValueHeadCheckpoint,
    FoundationTerrainContinuationReceipt,
)> {
    if previous.join("terrain-continuation.json").exists() {
        return Err(
            "terrain continuation requires the original sealed cycle, not another transfer".into(),
        );
    }
    let sealed = load_cycle_continuation(previous, preserve_from)?;
    let original_receipt: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(previous.join("cycle.json"))?)?;
    let source = sealed.asset;
    if source.manifest().sensor_profile() != SensorProfile::GroundedObjectSlotsV1 {
        return Err("terrain continuation requires a grounded-object checkpoint".into());
    }
    let (target, next, old_hash, interface_changes) = if let (Some(legacy), Some(template)) =
        (legacy, terrain_template)
    {
        let (target, next, changes) = prepare_legacy_terrain_binding(
            &source,
            &sealed.actor,
            legacy,
            template,
            original_receipt.seed,
            sealed.founder_seed_base,
        )?;
        (target, next, sealed.actor.phenotype_hash, changes)
    } else {
        if legacy.is_some() || terrain_template.is_some() {
            return Err("legacy continuation requires both authenticated export and trained terrain template".into());
        }
        let mut config = alife_world::CanonicalNewGameConfig::phase3(original_receipt.seed, 1)?;
        config.brain_class = BrainScaleTier::Standard2048;
        config.founder_seed_base = sealed.founder_seed_base;
        config.sensor_profile = source.manifest().sensor_profile();
        let game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &source)?;
        let record = game
            .world
            .organism_registry()
            .iter()
            .next()
            .ok_or("source founder missing")?;
        let genome = record.phenotype().brain_genome.clone();
        let capacity = alife_core::BrainCapacityClass::n2048();
        let development = crate::gpu_live_runtime::foundation_construction_development(
            &genome,
            &capacity,
            &record
                .phenotype()
                .development_state_at(alife_core::Tick::ZERO)?,
        )?;
        let (old, _) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome.clone(),
            development.clone(),
            source.clone(),
        )?;
        let template = alife_core::PhenotypeCompiler::compile_testing_procedural_baseline(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedTerrainVisionV1,
        )?;
        // The procedural compile supplies metadata only. Every exported weight is
        // taken from the existing trained source, then that actual asset is compiled.
        let target = FoundationWeightAsset::from_trained_weights(
            &template,
            source.weights().to_vec(),
            source.manifest().training_stage(),
        )?;
        let (next, _) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            target.clone(),
        )?;
        verify_terrain_coordinates(&old, &next)?;
        (target, next, old.phenotype_hash(), Vec::new())
    };
    let mut actor = sealed.actor;
    actor.config.validate()?;
    let n = source.weights().len();
    let indices = (0..actor.stage_mask.len())
        .filter(|index| actor.stage_mask.is_trainable(*index))
        .map(|index| index as u32)
        .collect::<Vec<_>>();
    if actor.stage_mask != StageTrainableMask::from_synapse_indices(&next, &indices)? {
        return Err("terrain continuation source stage mask is malformed".into());
    }
    if actor.schema_version != 2
        || actor.phenotype_hash != old_hash
        || actor.first_moment.len() != n
        || actor.second_moment.len() != n
        || actor.update_ages.len() != n
        || actor
            .weights
            .iter()
            .chain(&actor.first_moment)
            .chain(&actor.second_moment)
            .any(|v| !v.is_finite())
        || actor.second_moment.iter().any(|v| *v < 0.0)
        || actor
            .update_ages
            .iter()
            .zip(&actor.first_moment)
            .zip(&actor.second_moment)
            .any(|((age, m), v)| {
                *age > actor.optimizer_step || (*age == 0 && (*m != 0.0 || *v != 0.0))
            })
        || sealed.value.feature_count != next.neuron_count()
    {
        return Err(
            "terrain continuation source actor or ordered value features are incompatible".into(),
        );
    }
    PpoTrainingState::from_checkpoint(sealed.value.clone())?;
    actor.phenotype_hash = next.phenotype_hash();
    actor.source_foundation_digest = target.digest();
    let files = [
        "cycle.json",
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ];
    let source_file_blake3 = files
        .iter()
        .map(|name| {
            Ok((
                (*name).to_owned(),
                blake3::hash(&std::fs::read(previous.join(name))?)
                    .to_hex()
                    .to_string(),
            ))
        })
        .collect::<Result<_>>()?;
    let receipt = FoundationTerrainContinuationReceipt {
        schema_version: 1,
        source_directory: previous.canonicalize()?,
        source_file_blake3,
        source_asset_digest: digest(&source),
        target_asset_digest: digest(&target),
        target_phenotype_hash: next.phenotype_hash(),
        actor_optimizer_step: actor.optimizer_step,
        value_optimizer_step: sealed.value.optimizer_step,
        biological_objective_version: original_receipt.biological_objective_version,
        preserved_weight_count: n,
        optimizer_reset: false,
        value_state_reset: false,
        personal_lifetime_restored: false,
        legacy_source_export: legacy.map(|p| p.canonicalize()).transpose()?,
        terrain_template: terrain_template.map(|p| p.canonicalize()).transpose()?,
        source_phenotype_blake3: legacy
            .map(|p| -> Result<String> {
                Ok(
                    blake3::hash(&std::fs::read(p.join("source-phenotype.json"))?)
                        .to_hex()
                        .to_string(),
                )
            })
            .transpose()?,
        explicit_interface_changes: interface_changes,
    };
    Ok((target, actor, sealed.value, receipt))
}

/// CPU-only explicit sensor migration of an existing complete offline checkpoint.
/// The original cycle receipt remains byte-identical; no update or objective
/// relabel occurs here. A later first cycle still requires the objective flag.
pub fn continue_foundation_to_terrain(
    previous: &Path,
    output: &Path,
    preserve_from: Option<u16>,
) -> Result<FoundationTerrainContinuationReceipt> {
    continue_foundation_to_terrain_with_legacy(previous, output, preserve_from, None, None)
}

pub fn continue_foundation_to_terrain_with_legacy(
    previous: &Path,
    output: &Path,
    preserve_from: Option<u16>,
    legacy: Option<&Path>,
    terrain_template: Option<&Path>,
) -> Result<FoundationTerrainContinuationReceipt> {
    let (asset, actor, _value, receipt) =
        prepare_terrain_continuation(previous, preserve_from, legacy, terrain_template)?;
    crate::foundation_training_output::in_new_directory(
        output,
        || {
            std::fs::write(
                output.join("trained.alife-foundation"),
                asset.encode_canonical()?,
            )?;
            std::fs::write(
                output.join("actor-checkpoint.json"),
                serde_json::to_vec(&actor)?,
            )?;
            for name in ["cycle.json", "value-checkpoint.json"] {
                std::fs::copy(previous.join(name), output.join(name))?;
            }
            std::fs::write(
                output.join("terrain-continuation.json"),
                serde_json::to_vec_pretty(&receipt)?,
            )?;
            Ok(())
        },
        |error| {
            let _ = std::fs::write(output.join("failure.txt"), error.to_string());
        },
    )?;
    Ok(receipt)
}

#[cfg(test)]
mod sampling_temperature_tests {
    use super::*;

    #[test]
    fn collection_ppo_and_next_cohort_share_one_temperature() {
        for temperature in [0.0001_f32, 1.0, 2.0, 16.0, 32.0, 128.0] {
            let collection = cycle_sampling_config(202610061298, 624, temperature).unwrap();
            let next = cycle_sampling_config(202610061298, 625, temperature).unwrap();
            let ppo = cycle_ppo_config(temperature).unwrap();
            assert_eq!(collection.temperature.to_bits(), ppo.temperature.to_bits());
            assert_eq!(next.temperature.to_bits(), ppo.temperature.to_bits());
            assert_eq!(collection.counter, 0);
            assert!(collection.demonstrator.is_none());
            assert_ne!(collection.seed, next.seed);
            assert_eq!(
                ppo,
                PpoConfig {
                    temperature,
                    ..PpoConfig::default()
                }
            );
        }
    }

    #[test]
    fn incompatible_temperature_is_rejected_before_checkpoint_or_output_access() {
        for temperature in [0.0_f32, -1.0, 0.00001, 129.0, f32::NAN, f32::INFINITY] {
            let error = resume_foundation_training_cycle_with_lesson_temperature(
                Path::new("missing-source-for-temperature-fixture"),
                Path::new("unused-output-for-temperature-fixture"),
                1,
                16,
                FoundationTeacherLesson::GrabFood,
                temperature,
            )
            .unwrap_err();
            assert!(error.to_string().contains("temperature must be finite"));
        }
    }

    #[test]
    fn replay_action_rejects_collection_ppo_temperature_mismatch() {
        let action = PpoJointAction {
            candidate_count: 1,
            representative_mask: 1,
            motor_masks: [0; 6],
            representative: 0,
            forced_slots: vec![Some(4)],
            motor_candidates: [None, None, None, None, Some(0), None],
            old_joint_log_probability: 0.0,
            temperature: 2.0,
        };
        assert!(action
            .validate(cycle_ppo_config(2.0).unwrap().temperature)
            .is_ok());
        assert!(action
            .validate(cycle_ppo_config(1.0).unwrap().temperature)
            .is_err());
    }
}

fn validate_terrain_continuation_package(
    directory: &Path,
    preserve_from: Option<u16>,
) -> Result<()> {
    let receipt: FoundationTerrainContinuationReceipt =
        serde_json::from_slice(&std::fs::read(directory.join("terrain-continuation.json"))?)?;
    for (name, expected) in &receipt.source_file_blake3 {
        if !matches!(
            name.as_str(),
            "cycle.json"
                | "trained.alife-foundation"
                | "actor-checkpoint.json"
                | "value-checkpoint.json"
        ) || blake3::hash(&std::fs::read(receipt.source_directory.join(name))?)
            .to_hex()
            .as_str()
            != expected
        {
            return Err("terrain continuation source file changed".into());
        }
    }
    let (asset, actor, _value, expected) = prepare_terrain_continuation(
        &receipt.source_directory,
        preserve_from,
        receipt.legacy_source_export.as_deref(),
        receipt.terrain_template.as_deref(),
    )?;
    if serde_json::to_vec(&receipt)? != serde_json::to_vec(&expected)?
        || std::fs::read(directory.join("cycle.json"))?
            != std::fs::read(receipt.source_directory.join("cycle.json"))?
        || std::fs::read(directory.join("value-checkpoint.json"))?
            != std::fs::read(receipt.source_directory.join("value-checkpoint.json"))?
        || asset.encode_canonical()? != std::fs::read(directory.join("trained.alife-foundation"))?
        || serde_json::to_vec(&actor)?
            != serde_json::to_vec(&serde_json::from_slice::<
                alife_training::FoundationTrainerCheckpoint,
            >(&std::fs::read(
                directory.join("actor-checkpoint.json"),
            )?)?)?
    {
        return Err("terrain continuation package changed learned state or provenance".into());
    }
    Ok(())
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundationAdaptationReceipt {
    pub source_directory: std::path::PathBuf,
    pub source_asset_digest: String,
    pub adapted_asset_digest: String,
    pub founder_seed_base: u64,
    pub policy_version: u64,
    pub preserved_weight_count: usize,
    pub optimizer_reset: bool,
    pub founder_biology_calibration: u16,
}

/// Explicit newborn-prior revision; source bytes and personal saves are untouched.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundationFounderRefreshReceipt {
    pub schema_version: u16,
    pub source_asset_digest: String,
    pub target_asset_digest: String,
    pub source_decoder_digest: [u64; 4],
    pub target_decoder_digest: [u64; 4],
    pub source_compiler_inputs_digest: Option<[u64; 4]>,
    pub target_compiler_inputs_digest: [u64; 4],
    pub founder_seed_base: u64,
    pub preserved_neuron_count: u32,
    pub preserved_weight_count: usize,
    pub optimizer_reset: bool,
    pub value_state_reset: bool,
    pub promoted: bool,
    #[serde(default)]
    pub source_weight_only: bool,
    #[serde(default)]
    pub source_optimizer_reset: bool,
    #[serde(default)]
    pub metadata_rebuilt: bool,
    #[serde(default)]
    pub source_policy_version: u64,
    #[serde(default)]
    pub target_policy_version: u64,
    #[serde(default)]
    pub preserved_address_map_digest: Option<alife_core::Blake3Digest>,
}

const TERRAIN_REBIND_SOURCE_DIGEST: &str =
    "b83c0a689fd6c672c4826dd06f9ea4463587c840139579b2b3c063c91028dab4";
const TERRAIN_REBIND_SOURCE_PARENT_DIGEST: &str =
    "8a10a0a5eb350f4a24f1c9535e4e4b8f1e10240da905a32a9b0c1b5a0719c6d9";

/// Rebind the selected v2 weight-only prior to the current founder design.
/// Personal and optimizer state cannot enter this explicit metadata revision.
pub fn refresh_terrain_founder(
    previous: &Path,
    output: &Path,
) -> Result<FoundationFounderRefreshReceipt> {
    let receipt: FoundationAdaptationReceipt =
        serde_json::from_slice(&std::fs::read(previous.join("adaptation.json"))?)?;
    let source = FoundationWeightAsset::decode_canonical(&std::fs::read(
        previous.join("trained.alife-foundation"),
    )?)?;
    let (target, adaptation, refresh) =
        prepare_terrain_founder_refresh(previous, &receipt, &source)?;
    let asset_bytes = target.encode_canonical()?;
    let adaptation_bytes = serde_json::to_vec_pretty(&adaptation)?;
    let refresh_bytes = serde_json::to_vec_pretty(&refresh)?;
    // create_dir rejects existing destinations, including the immutable source.
    std::fs::create_dir(output)?;
    std::fs::write(output.join("trained.alife-foundation"), asset_bytes)?;
    std::fs::write(output.join("adaptation.json"), adaptation_bytes)?;
    std::fs::write(output.join("founder-refresh.json"), refresh_bytes)?;
    Ok(refresh)
}

fn prepare_terrain_founder_refresh(
    previous: &Path,
    receipt: &FoundationAdaptationReceipt,
    source: &FoundationWeightAsset,
) -> Result<(
    FoundationWeightAsset,
    FoundationAdaptationReceipt,
    FoundationFounderRefreshReceipt,
)> {
    let capacity = alife_core::BrainCapacityClass::n2048();
    let profile = SensorProfile::GroundedTerrainVisionV1;
    if receipt.founder_seed_base != 539_363_617
        || receipt.policy_version != 530
        || receipt.founder_biology_calibration != 2
        || !receipt.optimizer_reset
        || receipt.preserved_weight_count != 32_768
        || source.weights().len() != 32_768
        || source.manifest().capacity_class_id() != capacity.id()
        || source.manifest().sensor_profile() != profile
        || receipt.source_asset_digest != TERRAIN_REBIND_SOURCE_PARENT_DIGEST
        || receipt.adapted_asset_digest != TERRAIN_REBIND_SOURCE_DIGEST
        || digest(source) != receipt.adapted_asset_digest
        || source.manifest().promotion_receipt().is_promoted()
    {
        return Err("founder rebind requires the sealed weight-only terrain v2 source".into());
    }
    // This command accepts only the selected asset package. An actor, value,
    // personal save, or any other checkpoint needs a separate state transfer.
    for entry in std::fs::read_dir(previous)? {
        let entry = entry?;
        if !entry.file_type()?.is_file()
            || !matches!(
                entry.file_name().to_str(),
                Some(
                    "trained.alife-foundation"
                        | "adaptation.json"
                        | "founder-refresh.json"
                        | "README.md"
                )
            )
        {
            return Err("founder rebind rejects mutable checkpoints or extra package files".into());
        }
    }
    let source_revision: FoundationFounderRefreshReceipt =
        serde_json::from_slice(&std::fs::read(previous.join("founder-refresh.json"))?)?;
    if source_revision.schema_version != 1
        || source_revision.source_asset_digest != TERRAIN_REBIND_SOURCE_PARENT_DIGEST
        || source_revision.target_asset_digest != TERRAIN_REBIND_SOURCE_DIGEST
        || source_revision.target_decoder_digest != source.manifest().action_decoder_digest()
        || source_revision.founder_seed_base != receipt.founder_seed_base
        || source_revision.preserved_neuron_count != 2_048
        || source_revision.preserved_weight_count != source.weights().len()
        || !source_revision.optimizer_reset
        || !source_revision.value_state_reset
        || source_revision.promoted
    {
        return Err("founder rebind source revision receipt is not sealed".into());
    }
    let mut config = alife_world::CanonicalNewGameConfig::phase3(receipt.founder_seed_base, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = receipt.founder_seed_base;
    config.sensor_profile = profile;
    let game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, source)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("source founder missing")?;
    let genome = record.phenotype().brain_genome.clone();
    let development = crate::gpu_live_runtime::foundation_construction_development(
        &genome,
        &capacity,
        &record
            .phenotype()
            .development_state_at(alife_core::Tick::ZERO)?,
    )?;
    let target = alife_core::PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        profile,
    )?;
    let target_binding = alife_core::FoundationAbiBinding::canonical_for_capacity(&capacity)?;
    let manifest = source.manifest();
    if target.lobe_layout() != &alife_core::N2048FoundationLayoutV1::lobe_layout()
        || manifest.layout_digest() != target_binding.layout_digest()
        || manifest.route_abi_digest() != target.route_abi_digest()
        || manifest.plasticity_abi_digest() != target.plasticity_abi_digest()
        || manifest.address_map_digest() != target.persistent_address_map().digest()
        || manifest.weight_asset() != source.asset_ref()
        || manifest.weight_asset().weight_count() as usize != target.synapses().len()
        || target.neuron_count() != 2_048
        || target.synapses().len() != source.weights().len()
    {
        return Err(
            "founder rebind would change frozen N2048 coordinates or weight binding".into(),
        );
    }
    let revised = FoundationWeightAsset::from_trained_weights(
        &target,
        source.weights().to_vec(),
        source.manifest().training_stage(),
    )?;
    let (admitted, inputs) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome,
        development,
        revised.clone(),
    )?;
    let restored_inputs: alife_core::PhenotypeCompilerInputs =
        serde_json::from_slice(&serde_json::to_vec(&inputs)?)?;
    if restored_inputs != inputs
        || alife_core::PhenotypeCompiler::compile_validated(&restored_inputs, &capacity)?
            != admitted
        || revised.manifest().promotion_receipt().is_promoted()
        || admitted
            .synapses()
            .iter()
            .zip(source.weights())
            .any(|(synapse, weight)| synapse.genetic_weight().to_bits() != weight.to_bits())
        || revised
            .weights()
            .iter()
            .zip(source.weights())
            .any(|(a, b)| a.to_bits() != b.to_bits())
    {
        return Err("founder revision failed exact weight or compiler-resume acceptance".into());
    }
    let source_asset_digest = digest(source);
    let target_asset_digest = digest(&revised);
    let refresh = FoundationFounderRefreshReceipt {
        schema_version: 2,
        source_asset_digest: source_asset_digest.clone(),
        target_asset_digest: target_asset_digest.clone(),
        source_decoder_digest: source.manifest().action_decoder_digest(),
        target_decoder_digest: admitted.candidate_decoder().canonical_digest(),
        source_compiler_inputs_digest: None,
        target_compiler_inputs_digest: inputs.canonical_digest(),
        founder_seed_base: receipt.founder_seed_base,
        preserved_neuron_count: admitted.neuron_count(),
        preserved_weight_count: source.weights().len(),
        optimizer_reset: false,
        value_state_reset: false,
        promoted: false,
        source_weight_only: true,
        source_optimizer_reset: receipt.optimizer_reset,
        metadata_rebuilt: true,
        source_policy_version: receipt.policy_version,
        target_policy_version: receipt.policy_version + 1,
        preserved_address_map_digest: Some(admitted.persistent_address_map().digest()),
    };
    let adaptation = FoundationAdaptationReceipt {
        source_directory: previous.to_path_buf(),
        source_asset_digest,
        adapted_asset_digest: target_asset_digest,
        founder_seed_base: receipt.founder_seed_base,
        policy_version: receipt
            .policy_version
            .checked_add(1)
            .ok_or("policy version overflow")?,
        preserved_weight_count: source.weights().len(),
        // Retain the source's documented fresh-training handoff marker; this
        // rebind performed no optimizer reset or mutable-state transfer.
        optimizer_reset: receipt.optimizer_reset,
        founder_biology_calibration: receipt.founder_biology_calibration,
    };
    Ok((revised, adaptation, refresh))
}

#[cfg(test)]
mod founder_refresh_tests {
    use super::*;

    #[test]
    fn current_credit_transition_preserves_all_other_lanes_and_learning_parameters() {
        let old = serde_json::json!({"receptor_profile": [0.2,-1.0,1.0,-0.5,0.2,0.0,0.5,-0.5], "learning_rate": 0.01, "eligibility_decay": 0.95});
        let mut new = old.clone();
        new["receptor_profile"][0] = serde_json::json!(0.0);
        assert!(current_choice_credit_transition(
            &old,
            &new,
            Some("ActionCandidate")
        ));
        assert!(current_choice_credit_transition(
            &old,
            &new,
            Some("MemoryContext")
        ));
        assert!(!current_choice_credit_transition(
            &old,
            &new,
            Some("SpeechPayload")
        ));
        assert!(!current_choice_credit_transition(&old, &new, None));
        new["learning_rate"] = serde_json::json!(0.02);
        assert!(!current_choice_credit_transition(
            &old,
            &new,
            Some("ActionCandidate")
        ));
        new["learning_rate"] = serde_json::json!(0.01);
        new["receptor_profile"][1] = serde_json::json!(-0.5);
        assert!(!current_choice_credit_transition(
            &old,
            &new,
            Some("ActionCandidate")
        ));
    }

    #[test]
    fn held_food_dose_excludes_bootstrap_meals_and_counts_terminal_trained_rows() {
        assert_eq!(cycle_replay_dose(1025, 1024, &[1024]).unwrap(), (1, 0));
        assert_eq!(cycle_replay_dose(1025, 1024, &[0]).unwrap(), (1, 1));
        assert_eq!(cycle_replay_dose(3, 3, &[2]).unwrap(), (0, 1));
        assert_eq!(cycle_replay_dose(3, 2, &[]).unwrap(), (1, 0));
        assert!(cycle_replay_dose(1, 2, &[]).is_err());
        assert!(cycle_replay_dose(3, 1, &[]).is_err());
        assert!(cycle_replay_dose(3, 2, &[3]).is_err());
        assert!(cycle_replay_dose(3, 2, &[1, 1]).is_err());
    }

    fn cycle_handoff() -> (
        FoundationCycleReceipt,
        FoundationWeightAsset,
        alife_training::FoundationTrainerCheckpoint,
        alife_training::PpoValueHeadCheckpoint,
    ) {
        let (directory, source_receipt, source) = sealed_source();
        let (asset, adaptation, _) =
            prepare_terrain_founder_refresh(&directory, &source_receipt, &source).unwrap();
        let mut config =
            alife_world::CanonicalNewGameConfig::phase3(adaptation.founder_seed_base, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.founder_seed_base = adaptation.founder_seed_base;
        config.sensor_profile = asset.manifest().sensor_profile();
        let game =
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
        let record = game.world.organism_registry().iter().next().unwrap();
        let genome = record.phenotype().brain_genome.clone();
        let development = crate::gpu_live_runtime::foundation_construction_development(
            &genome,
            &alife_core::BrainCapacityClass::n2048(),
            &record
                .phenotype()
                .development_state_at(alife_core::Tick::ZERO)
                .unwrap(),
        )
        .unwrap();
        let (phenotype, _) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            asset.clone(),
        )
        .unwrap();
        let receipt = serde_json::from_value(serde_json::json!({
            "biological_objective_version": BIOLOGICAL_OBJECTIVE_VERSION,
            "seed": 17,
            "founder_seed_base": adaptation.founder_seed_base,
            "policy_version": 4,
            "training_ticks": 32,
            "collection_seconds": 1.0,
            "update_seconds": 1.0,
            "old_asset_digest": digest(&asset),
            "new_asset_digest": digest(&asset),
            "actor_optimizer_step": 7,
            "value_optimizer_step": 5,
            "completed_epochs": 1,
            "next_cohort_tick_captured": true,
            "next_cohort_optimizer_rebound": true
        }))
        .unwrap();
        let count = asset.weights().len();
        let mask = StageTrainableMask::recurrent_only(&phenotype).unwrap();
        let ages = (0..count)
            .map(|index| if mask.is_trainable(index) { 7 } else { 0 })
            .collect::<Vec<_>>();
        let actor = alife_training::FoundationTrainerCheckpoint {
            schema_version: 2,
            phenotype_hash: phenotype.phenotype_hash(),
            source_foundation_digest: asset.digest(),
            optimizer_step: 7,
            config: AdamWConfig::default(),
            stage_mask: mask,
            weights: asset.weights().to_vec(),
            first_moment: ages
                .iter()
                .map(|age| if *age > 0 { 0.1 } else { 0.0 })
                .collect(),
            second_moment: ages
                .iter()
                .map(|age| if *age > 0 { 0.01 } else { 0.0 })
                .collect(),
            update_ages: ages,
        };
        let width = phenotype.neuron_count() as usize + 1;
        let mut parameters = vec![0.2; width];
        parameters.extend(vec![0.1; width]);
        parameters.extend(vec![0.01; width]);
        let value = alife_training::PpoValueHeadCheckpoint {
            feature_count: phenotype.neuron_count(),
            optimizer_step: 5,
            parameters,
            last_updated_policy_version: Some(4),
        };
        (receipt, asset, actor, value)
    }

    #[test]
    fn cycle_handoff_preserves_valid_learned_optimizer_state() {
        let (receipt, asset, actor, value) = cycle_handoff();
        let original_actor = actor.clone();
        let original_value = value.clone();
        let actor = serde_json::from_slice(&serde_json::to_vec(&actor).unwrap()).unwrap();
        let value = serde_json::from_slice(&serde_json::to_vec(&value).unwrap()).unwrap();
        validate_cycle_optimizer_handoff(&receipt, &asset, &actor, &value).unwrap();
        assert_eq!(actor, original_actor);
        assert_eq!(value, original_value);
        PpoTrainingState::from_checkpoint(value).unwrap();
    }

    #[test]
    fn objective_continuation_requires_the_exact_supported_transition() {
        assert_eq!(validate_objective_continuation(2, None).unwrap(), None);
        assert_eq!(
            validate_objective_continuation(1, Some(1)).unwrap(),
            Some(1)
        );
        for (source, requested) in [
            (1, None),
            (1, Some(2)),
            (2, Some(1)),
            (0, Some(0)),
            (3, Some(3)),
        ] {
            assert!(validate_objective_continuation(source, requested).is_err());
        }
    }

    #[test]
    fn objective_continuation_loads_both_complete_heads_without_resetting_source() {
        let (receipt, asset, actor, value) = cycle_handoff();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "alife-objective-continuation-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let mut legacy = serde_json::to_value(receipt).unwrap();
        // A genuine old receipt omits this field; decoding keeps its v1 identity.
        legacy
            .as_object_mut()
            .unwrap()
            .remove("biological_objective_version");
        let files = [
            ("cycle.json", serde_json::to_vec(&legacy).unwrap()),
            (
                "trained.alife-foundation",
                asset.encode_canonical().unwrap(),
            ),
            ("actor-checkpoint.json", serde_json::to_vec(&actor).unwrap()),
            ("value-checkpoint.json", serde_json::to_vec(&value).unwrap()),
        ];
        for (name, bytes) in &files {
            std::fs::write(directory.join(name), bytes).unwrap();
        }
        assert!(load_cycle_continuation(&directory, None).is_err());
        assert!(load_cycle_continuation(&directory, Some(2)).is_err());
        let continued = load_cycle_continuation(&directory, Some(1)).unwrap();
        assert_eq!(continued.objective_transition_from, Some(1));
        assert_eq!(continued.policy_version, 5);
        assert_eq!(
            continued.asset.encode_canonical().unwrap(),
            asset.encode_canonical().unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&continued.actor).unwrap(),
            serde_json::to_vec(&actor).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&continued.value).unwrap(),
            serde_json::to_vec(&value).unwrap()
        );
        for (name, bytes) in &files {
            assert_eq!(std::fs::read(directory.join(name)).unwrap(), *bytes);
            std::fs::remove_file(directory.join(name)).unwrap();
        }
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn objective_continuation_rejects_legacy_resume_before_output_or_asset_reads() {
        let (mut receipt, _, _, _) = cycle_handoff();
        receipt.biological_objective_version = 1;
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "alife-objective-rejection-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(
            directory.join("cycle.json"),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();
        let output = directory.join("must-not-exist");
        let source_hash = blake3::hash(&std::fs::read(directory.join("cycle.json")).unwrap());
        std::fs::write(
            directory.join("failure.json"),
            b"preserved source diagnostic",
        )
        .unwrap();
        let error = resume_foundation_training_cycle(&directory, &output, 17, 1).unwrap_err();
        assert!(error.to_string().contains("objective continuation blocked"));
        assert!(!output.exists());
        let error = resume_foundation_training_cycle(&directory, &directory, 17, 1).unwrap_err();
        assert!(error.to_string().contains("objective continuation blocked"));
        assert_eq!(
            blake3::hash(&std::fs::read(directory.join("cycle.json")).unwrap()),
            source_hash
        );
        assert_eq!(
            std::fs::read(directory.join("failure.json")).unwrap(),
            b"preserved source diagnostic"
        );
        std::fs::remove_file(directory.join("failure.json")).unwrap();
        std::fs::remove_file(directory.join("cycle.json")).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn objective_continuation_preserves_existing_outputs_after_valid_admission() {
        let (receipt, asset, actor, value) = cycle_handoff();
        let files = [
            ("cycle.json", serde_json::to_vec(&receipt).unwrap()),
            (
                "trained.alife-foundation",
                asset.encode_canonical().unwrap(),
            ),
            ("actor-checkpoint.json", serde_json::to_vec(&actor).unwrap()),
            ("value-checkpoint.json", serde_json::to_vec(&value).unwrap()),
            ("failure.json", b"preserved source diagnostic".to_vec()),
        ];
        for alias in [true, false] {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let directory = std::env::temp_dir().join(format!(
                "alife-admitted-output-{}-{nonce}-{alias}",
                std::process::id()
            ));
            std::fs::create_dir(&directory).unwrap();
            let source = directory.join("source");
            let foreign = directory.join("foreign");
            std::fs::create_dir(&source).unwrap();
            std::fs::create_dir(&foreign).unwrap();
            for (name, bytes) in &files {
                std::fs::write(source.join(name), bytes).unwrap();
            }
            std::fs::write(foreign.join("failure.json"), b"foreign diagnostic").unwrap();
            let output = if alias { &source } else { &foreign };
            let error = resume_foundation_training_cycle(&source, output, 17, 1).unwrap_err();
            assert_eq!(
                error.downcast_ref::<std::io::Error>().unwrap().kind(),
                std::io::ErrorKind::AlreadyExists
            );
            for (name, bytes) in &files {
                assert_eq!(
                    blake3::hash(&std::fs::read(source.join(name)).unwrap()),
                    blake3::hash(bytes)
                );
                std::fs::remove_file(source.join(name)).unwrap();
            }
            assert_eq!(
                std::fs::read(foreign.join("failure.json")).unwrap(),
                b"foreign diagnostic"
            );
            std::fs::remove_file(foreign.join("failure.json")).unwrap();
            std::fs::remove_dir(&source).unwrap();
            std::fs::remove_dir(&foreign).unwrap();
            std::fs::remove_dir(&directory).unwrap();
        }
    }

    #[test]
    fn objective_continuation_owned_failure_keeps_cycle_diagnostic_fields() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let output = std::env::temp_dir().join(format!(
            "alife-owned-cycle-diagnostic-{}-{nonce}",
            std::process::id()
        ));
        let result: Result<()> = crate::foundation_training_output::in_new_directory(
            &output,
            || Err("CPU-only diagnostic failure".into()),
            |error| record_cycle_failure(&output, error, 17, 32, None, None),
        );
        assert!(result.is_err());
        let diagnostic: serde_json::Value =
            serde_json::from_slice(&std::fs::read(output.join("failure.json")).unwrap()).unwrap();
        assert_eq!(diagnostic["error"], "CPU-only diagnostic failure");
        assert_eq!(diagnostic["seed"], 17);
        assert_eq!(diagnostic["requested_waking_decisions"], 32);
        std::fs::remove_file(output.join("failure.json")).unwrap();
        std::fs::remove_dir(output).unwrap();
    }

    #[test]
    fn terrain_continuation_preserves_full_heads_and_rejects_tampering() {
        let (mut receipt, terrain, mut actor, value) = cycle_handoff();
        let mut config = alife_world::CanonicalNewGameConfig::phase3(receipt.seed, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.founder_seed_base = receipt.founder_seed_base;
        config.sensor_profile = terrain.manifest().sensor_profile();
        let game =
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &terrain).unwrap();
        let record = game.world.organism_registry().iter().next().unwrap();
        let genome = record.phenotype().brain_genome.clone();
        let capacity = alife_core::BrainCapacityClass::n2048();
        let development = crate::gpu_live_runtime::foundation_construction_development(
            &genome,
            &capacity,
            &record
                .phenotype()
                .development_state_at(alife_core::Tick::ZERO)
                .unwrap(),
        )
        .unwrap();
        let template = alife_core::PhenotypeCompiler::compile_testing_procedural_baseline(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let source = FoundationWeightAsset::from_trained_weights(
            &template,
            terrain.weights().to_vec(),
            terrain.manifest().training_stage(),
        )
        .unwrap();
        let (old, _) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            source.clone(),
        )
        .unwrap();
        actor.phenotype_hash = old.phenotype_hash();
        actor.source_foundation_digest = source.digest();
        receipt.new_asset_digest = digest(&source);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "alife-terrain-continuation-{}-{nonce}",
            std::process::id()
        ));
        let original = root.join("original");
        let output = root.join("continued");
        std::fs::create_dir_all(&original).unwrap();
        let files = [
            ("cycle.json", serde_json::to_vec(&receipt).unwrap()),
            (
                "trained.alife-foundation",
                source.encode_canonical().unwrap(),
            ),
            ("actor-checkpoint.json", serde_json::to_vec(&actor).unwrap()),
            ("value-checkpoint.json", serde_json::to_vec(&value).unwrap()),
        ];
        for (name, bytes) in &files {
            std::fs::write(original.join(name), bytes).unwrap();
        }
        let conversion = continue_foundation_to_terrain(&original, &output, None).unwrap();
        assert!(!conversion.optimizer_reset && !conversion.value_state_reset);
        let restored = load_cycle_continuation(&output, None).unwrap();
        let mut rebound = restored.actor;
        rebound.phenotype_hash = actor.phenotype_hash;
        rebound.source_foundation_digest = actor.source_foundation_digest;
        assert_eq!(rebound, actor);
        assert_eq!(restored.value, value);
        for name in ["cycle.json", "value-checkpoint.json"] {
            assert_eq!(
                std::fs::read(output.join(name)).unwrap(),
                std::fs::read(original.join(name)).unwrap()
            );
        }
        let mut corrupt: alife_training::FoundationTrainerCheckpoint =
            serde_json::from_slice(&std::fs::read(output.join("actor-checkpoint.json")).unwrap())
                .unwrap();
        corrupt.first_moment[0] += 0.125;
        std::fs::write(
            output.join("actor-checkpoint.json"),
            serde_json::to_vec(&corrupt).unwrap(),
        )
        .unwrap();
        assert!(load_cycle_continuation(&output, None).is_err());
        for (name, bytes) in &files {
            assert_eq!(std::fs::read(original.join(name)).unwrap(), *bytes);
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn cycle_handoff_value_age_mismatch_changes_next_adam_update() {
        let (receipt, asset, actor, value) = cycle_handoff();
        // Scalar bias-correction diagnostic of foundation_ppo.wgsl's value
        // update, not a CPU training or neural execution path.
        let next_weight = |checkpoint: &alife_training::PpoValueHeadCheckpoint| {
            let adam = PpoConfig::default().value_optimizer;
            let width = checkpoint.feature_count as usize + 1;
            let gradient = 0.25_f32;
            let moment = adam.beta1 * checkpoint.parameters[width] + (1.0 - adam.beta1) * gradient;
            let variance = adam.beta2 * checkpoint.parameters[width * 2]
                + (1.0 - adam.beta2) * gradient * gradient;
            let step = (checkpoint.optimizer_step + 1) as i32;
            checkpoint.parameters[0]
                - adam.learning_rate
                    * ((moment / (1.0 - adam.beta1.powi(step)))
                        / ((variance / (1.0 - adam.beta2.powi(step))).sqrt() + adam.epsilon)
                        + adam.weight_decay * checkpoint.parameters[0])
        };
        let mut altered = value.clone();
        altered.optimizer_step = 0;
        // The ordinary value checkpoint decoder admits both finite states.
        PpoTrainingState::from_checkpoint(value.clone()).unwrap();
        PpoTrainingState::from_checkpoint(altered.clone()).unwrap();
        assert!((next_weight(&value) - next_weight(&altered)).abs() > 1.0e-5);
        validate_cycle_optimizer_handoff(&receipt, &asset, &actor, &value).unwrap();
        assert!(validate_cycle_optimizer_handoff(&receipt, &asset, &actor, &altered).is_err());
    }

    #[test]
    fn cycle_handoff_rejects_optimizer_age_changes_with_identical_actor_weights() {
        let (receipt, asset, actor, value) = cycle_handoff();
        // Advancing the actor batch counter can pass the per-weight age bound;
        // at u32::MAX even preparing the next replay would overflow.
        for step in [0, 6, 8, u32::MAX] {
            let mut altered_actor = actor.clone();
            altered_actor.optimizer_step = step;
            assert!(
                validate_cycle_optimizer_handoff(&receipt, &asset, &altered_actor, &value).is_err()
            );
        }
        for step in [0, 4, 6] {
            let mut altered_value = value.clone();
            altered_value.optimizer_step = step;
            assert!(
                validate_cycle_optimizer_handoff(&receipt, &asset, &actor, &altered_value).is_err()
            );
        }
    }

    #[test]
    fn cycle_handoff_rejects_incomplete_next_cohort_admission() {
        let (mut receipt, asset, actor, value) = cycle_handoff();
        receipt.next_cohort_tick_captured = false;
        assert!(validate_cycle_optimizer_handoff(&receipt, &asset, &actor, &value).is_err());
        receipt.next_cohort_tick_captured = true;
        receipt.next_cohort_optimizer_rebound = false;
        assert!(validate_cycle_optimizer_handoff(&receipt, &asset, &actor, &value).is_err());
    }

    fn sealed_source() -> (
        std::path::PathBuf,
        FoundationAdaptationReceipt,
        FoundationWeightAsset,
    ) {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/founders/terrain-care-n2048-v2");
        let receipt =
            serde_json::from_slice(&std::fs::read(directory.join("adaptation.json")).unwrap())
                .unwrap();
        let source = FoundationWeightAsset::decode_canonical(
            &std::fs::read(directory.join("trained.alife-foundation")).unwrap(),
        )
        .unwrap();
        (directory, receipt, source)
    }

    #[test]
    fn n2048_terrain_founder_refresh_preserves_weights_and_strict_resume() {
        let (directory, receipt, source) = sealed_source();
        let original_bytes = std::fs::read(directory.join("trained.alife-foundation")).unwrap();
        let (revised, adaptation, refresh) =
            prepare_terrain_founder_refresh(&directory, &receipt, &source).unwrap();
        assert_eq!(source.encode_canonical().unwrap(), original_bytes);
        assert_eq!(
            std::fs::read(directory.join("trained.alife-foundation")).unwrap(),
            original_bytes
        );
        assert_eq!(refresh.source_asset_digest, receipt.adapted_asset_digest);
        assert_eq!(refresh.target_asset_digest, adaptation.adapted_asset_digest);
        assert_ne!(refresh.source_asset_digest, refresh.target_asset_digest);
        assert_ne!(refresh.source_decoder_digest, refresh.target_decoder_digest);
        assert_eq!(refresh.preserved_neuron_count, 2_048);
        assert_eq!(refresh.preserved_weight_count, 32_768);
        assert!(!refresh.optimizer_reset && !refresh.value_state_reset);
        assert!(refresh.source_weight_only && refresh.source_optimizer_reset);
        assert!(refresh.metadata_rebuilt);
        assert_eq!(refresh.schema_version, 2);
        assert_eq!(refresh.source_compiler_inputs_digest, None);
        assert_eq!(refresh.source_policy_version, 530);
        assert_eq!(refresh.target_policy_version, 531);
        assert_eq!(
            refresh.preserved_address_map_digest,
            Some(source.manifest().address_map_digest())
        );
        assert_eq!(
            source.manifest().layout_digest(),
            revised.manifest().layout_digest()
        );
        assert_eq!(
            source.manifest().route_abi_digest(),
            revised.manifest().route_abi_digest()
        );
        assert_eq!(
            source.manifest().plasticity_abi_digest(),
            revised.manifest().plasticity_abi_digest()
        );
        assert_eq!(
            source.manifest().address_map_digest(),
            revised.manifest().address_map_digest()
        );
        assert!(!refresh.promoted);
        assert!(source
            .weights()
            .iter()
            .zip(revised.weights())
            .all(|(a, b)| { a.to_bits() == b.to_bits() }));
        assert_eq!(
            FoundationWeightAsset::decode_canonical(&revised.encode_canonical().unwrap()).unwrap(),
            revised
        );
        // Only the explicitly selected source can enter this rebind operation.
        assert!(prepare_terrain_founder_refresh(&directory, &adaptation, &revised).is_err());
    }

    #[test]
    fn n2048_terrain_founder_refresh_rejects_unsealed_receipt() {
        let (directory, mut receipt, source) = sealed_source();
        receipt.adapted_asset_digest = "0".repeat(64);
        assert!(prepare_terrain_founder_refresh(&directory, &receipt, &source).is_err());
        receipt.adapted_asset_digest = digest(&source);
        receipt.founder_biology_calibration = 1;
        assert!(prepare_terrain_founder_refresh(&directory, &receipt, &source).is_err());
        receipt.founder_biology_calibration = 2;
        receipt.policy_version = 529;
        assert!(prepare_terrain_founder_refresh(&directory, &receipt, &source).is_err());
        receipt.policy_version = 530;
        receipt.founder_seed_base += 1;
        assert!(prepare_terrain_founder_refresh(&directory, &receipt, &source).is_err());
        receipt.founder_seed_base -= 1;

        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let package = std::env::temp_dir().join(format!(
            "alife-founder-rebind-rejection-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&package).unwrap();
        for name in [
            "trained.alife-foundation",
            "adaptation.json",
            "founder-refresh.json",
            "README.md",
        ] {
            std::fs::copy(directory.join(name), package.join(name)).unwrap();
        }
        for name in ["actor-checkpoint.json", "value-checkpoint.json"] {
            std::fs::write(package.join(name), b"{}").unwrap();
            let rejection = prepare_terrain_founder_refresh(&package, &receipt, &source)
                .unwrap_err()
                .to_string();
            assert!(rejection.contains("rejects mutable checkpoints"));
            std::fs::remove_file(package.join(name)).unwrap();
        }
        // This directory was created exclusively by this test under temp_dir.
        std::fs::remove_dir_all(package).unwrap();
    }
}

/// Explicit candidate migration. Personal state is absent, and the source is
/// immutable. Changing observations invalidates old optimizer/value statistics.
pub fn adapt_foundation_to_terrain(
    previous: &Path,
    output: &Path,
) -> Result<FoundationAdaptationReceipt> {
    let receipt: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(previous.join("cycle.json"))?)?;
    let source = FoundationWeightAsset::decode_canonical(&std::fs::read(
        previous.join("trained.alife-foundation"),
    )?)?;
    if !receipt.next_cohort_optimizer_rebound
        || digest(&source) != receipt.new_asset_digest
        || source.manifest().sensor_profile() != SensorProfile::GroundedObjectSlotsV1
    {
        return Err("adaptation requires a sealed grounded-object source cohort".into());
    }
    let founder_seed_base = if receipt.founder_seed_base == 0 {
        receipt.seed
    } else {
        receipt.founder_seed_base
    };
    let mut config = alife_world::CanonicalNewGameConfig::phase3(receipt.seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = founder_seed_base;
    let game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &source)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("source founder missing")?;
    let genome = record.phenotype().brain_genome.clone();
    let development = crate::gpu_live_runtime::foundation_construction_development(
        &genome,
        &alife_core::BrainCapacityClass::n2048(),
        &record
            .phenotype()
            .development_state_at(alife_core::Tick::ZERO)?,
    )?;
    let (old, _) = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development.clone(),
        source.clone(),
    )?;
    let target = alife_core::PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &alife_core::BrainCapacityClass::n2048(),
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )?;
    if old.persistent_address_map().digest() != target.persistent_address_map().digest()
        || old.synapses().len() != target.synapses().len()
        || old.synapses().iter().zip(target.synapses()).any(|(a, b)| {
            a.source() != b.source()
                || a.target() != b.target()
                || a.route_index() != b.route_index()
                || a.kind() != b.kind()
        })
    {
        return Err("terrain adaptation changed inherited synapse coordinates".into());
    }
    let adapted = FoundationWeightAsset::from_trained_weights(
        &target,
        source.weights().to_vec(),
        source.manifest().training_stage(),
    )?;
    if adapted.weights().len() != source.weights().len()
        || adapted
            .weights()
            .iter()
            .zip(source.weights())
            .any(|(a, b)| a.to_bits() != b.to_bits())
    {
        return Err("terrain adaptation changed inherited weight bits".into());
    }
    let mut target_config = config;
    target_config.sensor_profile = SensorProfile::GroundedTerrainVisionV1;
    alife_world::create_canonical_new_game_with_n2048_candidate(&target_config, &adapted)?;
    let result = FoundationAdaptationReceipt {
        source_directory: previous.to_path_buf(),
        source_asset_digest: digest(&source),
        adapted_asset_digest: digest(&adapted),
        founder_seed_base,
        policy_version: receipt
            .policy_version
            .checked_add(1)
            .ok_or("policy version overflow")?,
        preserved_weight_count: source.weights().len(),
        optimizer_reset: true,
        founder_biology_calibration: 2,
    };
    std::fs::create_dir(output)?;
    std::fs::write(
        output.join("trained.alife-foundation"),
        adapted.encode_canonical()?,
    )?;
    std::fs::write(
        output.join("adaptation.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    Ok(result)
}

fn organism_energy(runtime: &GpuLiveBrainRuntime, organism: alife_core::OrganismId) -> Result<f32> {
    Ok(runtime
        .world()
        .organism_registry()
        .get(organism)
        .ok_or("training organism missing from world")?
        .biochemistry()
        .body
        .energy)
}

fn consumed(step: &crate::FoundationTrainingStep) -> bool {
    let outcome = step.patch.outcome();
    outcome.physical.contact == alife_core::PhysicalContactKind::Consumed
        || outcome.joint.as_ref().is_some_and(|joint| {
            joint.channel_outcomes.iter().any(|channel| {
                channel.physical.contact == alife_core::PhysicalContactKind::Consumed
            })
        })
}

fn joint_action(behavior: &alife_gpu_backend::GpuTrainingRolloutReceipt) -> Result<PpoJointAction> {
    let count = u16::try_from(behavior.forced_motor_slots.len())?;
    let action = PpoJointAction {
        candidate_count: count,
        representative_mask: behavior.representative_mask,
        motor_masks: behavior.motor_masks,
        representative: behavior.representative_index,
        forced_slots: behavior
            .forced_motor_slots
            .iter()
            .copied()
            .map(Some)
            .collect(),
        motor_candidates: behavior.motor_indices.map(|i| (i != u16::MAX).then_some(i)),
        old_joint_log_probability: behavior.on_policy_log_probability()?,
        temperature: behavior.sampling.temperature,
    };
    action.validate(behavior.sampling.temperature)?;
    Ok(action)
}

/// A single pinned-policy cohort. The extra captured decision supplies the
/// actual next-state value for a time-limit truncation; it is not trained on.
pub fn run_foundation_training_cycle(
    output: &Path,
    seed: u64,
    training_ticks: usize,
) -> Result<FoundationCycleReceipt> {
    run_foundation_training_cycle_from(output, seed, training_ticks, None, None, None, None)
}

/// A scenario gate: food is out of reach until an explicit world tick. Biology
/// and the organism's policy are unchanged throughout the interval.
pub fn run_foundation_training_cycle_with_food_delay(
    output: &Path,
    seed: u64,
    training_ticks: usize,
    food_available_world_tick: u64,
) -> Result<FoundationCycleReceipt> {
    run_foundation_training_cycle_from(
        output,
        seed,
        training_ticks,
        None,
        Some(food_available_world_tick),
        None,
        None,
    )
}

/// Continue from an exactly rebound, sealed previous cycle. The saved optimizer
/// and value head are restored before using any newly collected decision.
pub fn resume_foundation_training_cycle(
    previous: &Path,
    output: &Path,
    seed: u64,
    training_ticks: usize,
) -> Result<FoundationCycleReceipt> {
    run_foundation_training_cycle_from(
        output,
        seed,
        training_ticks,
        Some(previous),
        None,
        None,
        None,
    )
}

pub fn resume_foundation_training_cycle_with_food_delay(
    previous: &Path,
    output: &Path,
    seed: u64,
    training_ticks: usize,
    food_available_world_tick: u64,
) -> Result<FoundationCycleReceipt> {
    run_foundation_training_cycle_from(
        output,
        seed,
        training_ticks,
        Some(previous),
        Some(food_available_world_tick),
        None,
        None,
    )
}

/// Collect policy actions in the same world layout used by a teacher lesson.
/// Configured teacher speech enters ordinary hearing; no demonstrator selects
/// learner actions. All decisions and outcomes come from the live brain.
pub fn run_foundation_training_cycle_with_lesson(
    previous: Option<&Path>,
    output: &Path,
    seed: u64,
    training_ticks: usize,
    lesson: FoundationTeacherLesson,
) -> Result<FoundationCycleReceipt> {
    run_foundation_training_cycle_from(
        output,
        seed,
        training_ticks,
        previous,
        None,
        Some(lesson),
        None,
    )
}

/// Bounded exploration diagnostic on an existing checkpoint. The same fixed
/// temperature governs collection, PPO likelihood/gradient, and next admission.
pub fn resume_foundation_training_cycle_with_lesson_temperature(
    previous: &Path,
    output: &Path,
    seed: u64,
    training_ticks: usize,
    lesson: FoundationTeacherLesson,
    sampling_temperature: f32,
) -> Result<FoundationCycleReceipt> {
    run_foundation_training_cycle_from_with_temperature(
        output,
        seed,
        training_ticks,
        Some(previous),
        None,
        Some(lesson),
        None,
        sampling_temperature,
        0,
    )
}

/// Rehearse only this continued life's first genuinely acquired food after PPO.
/// The actor's existing Adam history continues; the PPO value state is frozen
/// during the optional imitation phase. Ordinary runtime sampling is unchanged.
pub fn resume_foundation_grab_cycle_with_acquisition_rehearsal(
    previous: &Path,
    output: &Path,
    seed: u64,
    training_ticks: usize,
    sampling_temperature: f32,
    rehearsal_epochs: u32,
) -> Result<FoundationCycleReceipt> {
    crate::foundation_acquisition_rehearsal::validate_rehearsal_epochs(rehearsal_epochs)?;
    run_foundation_training_cycle_from_with_temperature(
        output,
        seed,
        training_ticks,
        Some(previous),
        None,
        Some(FoundationTeacherLesson::GrabFood),
        None,
        sampling_temperature,
        rehearsal_epochs,
    )
}

/// Explicitly move an existing sealed cycle to the current biological objective
/// while retaining its actor/value weights, moments, optimizer ages and masks.
/// This remains a new cohort; it does not restore individual lifetime state.
pub fn resume_foundation_training_cycle_with_objective_transition(
    previous: &Path,
    output: &Path,
    seed: u64,
    training_ticks: usize,
    source_objective_version: u16,
    lesson: Option<FoundationTeacherLesson>,
) -> Result<FoundationCycleReceipt> {
    run_foundation_training_cycle_from(
        output,
        seed,
        training_ticks,
        Some(previous),
        None,
        lesson,
        Some(source_objective_version),
    )
}

fn run_foundation_training_cycle_from(
    output: &Path,
    seed: u64,
    training_ticks: usize,
    previous: Option<&Path>,
    food_available_world_tick: Option<u64>,
    lesson: Option<FoundationTeacherLesson>,
    preserve_objective_from: Option<u16>,
) -> Result<FoundationCycleReceipt> {
    run_foundation_training_cycle_from_with_temperature(
        output,
        seed,
        training_ticks,
        previous,
        food_available_world_tick,
        lesson,
        preserve_objective_from,
        default_sampling_temperature(),
        0,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_foundation_training_cycle_from_with_temperature(
    output: &Path,
    seed: u64,
    training_ticks: usize,
    previous: Option<&Path>,
    food_available_world_tick: Option<u64>,
    lesson: Option<FoundationTeacherLesson>,
    preserve_objective_from: Option<u16>,
    sampling_temperature: f32,
    rehearsal_epochs: u32,
) -> Result<FoundationCycleReceipt> {
    cycle_ppo_config(sampling_temperature)?;
    if rehearsal_epochs != 0 {
        crate::foundation_acquisition_rehearsal::validate_rehearsal_epochs(rehearsal_epochs)?;
        if lesson != Some(FoundationTeacherLesson::GrabFood)
            || !previous.is_some_and(|path| {
                path.join("cycle.json").is_file()
                    && !path.join("adaptation.json").is_file()
                    && !path.join("warmup.json").is_file()
            })
        {
            return Err(
                "acquisition rehearsal requires an existing sealed GrabFood continuation".into(),
            );
        }
    }
    if seed == 0 || !(1..=36_000).contains(&training_ticks) {
        return Err("cycle needs a nonzero seed and 1..=36000 training ticks".into());
    }
    if food_available_world_tick == Some(0) {
        return Err("food availability tick must be positive".into());
    }
    if food_available_world_tick.is_some() && lesson.is_some() {
        return Err("a delayed food gate cannot be combined with a lesson layout".into());
    }
    let mut objective_state_reset = false;
    let mut objective_transition_from = None;
    if preserve_objective_from.is_some()
        && !previous.is_some_and(|path| {
            path.join("cycle.json").is_file()
                && !path.join("adaptation.json").is_file()
                && !path.join("warmup.json").is_file()
        })
    {
        return Err("objective transition requires an unambiguous sealed existing cycle".into());
    }
    let (asset, policy_version, restored_actor, restored_value, founder_seed_base) =
        if let Some(previous) = previous {
            if previous.join("adaptation.json").is_file() {
                let receipt: FoundationAdaptationReceipt =
                    serde_json::from_slice(&std::fs::read(previous.join("adaptation.json"))?)?;
                let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
                    previous.join("trained.alife-foundation"),
                )?)?;
                if digest(&asset) != receipt.adapted_asset_digest
                    || !receipt.optimizer_reset
                    || receipt.founder_biology_calibration != 2
                    || asset.manifest().sensor_profile() != SensorProfile::GroundedTerrainVisionV1
                    || receipt.preserved_weight_count != asset.weights().len()
                {
                    return Err("invalid explicit terrain adaptation handoff".into());
                }
                objective_state_reset = true;
                (
                    asset,
                    receipt.policy_version,
                    None,
                    None,
                    receipt.founder_seed_base,
                )
            } else if previous.join("warmup.json").is_file() {
                let receipt: FoundationWarmupReceipt =
                    serde_json::from_slice(&std::fs::read(previous.join("warmup.json"))?)?;
                if !receipt.next_cohort_optimizer_rebound
                    || receipt.demonstration_count == 0
                    || receipt.category_counts.iter().sum::<usize>() != receipt.demonstration_count
                {
                    return Err("previous warm-up is not a balanced sealed handoff".into());
                }
                let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
                    previous.join("trained.alife-foundation"),
                )?)?;
                if digest(&asset) != receipt.trained_asset_digest {
                    return Err("warm-up exported asset does not match its receipt".into());
                }
                let actor: alife_training::FoundationTrainerCheckpoint = serde_json::from_slice(
                    &std::fs::read(previous.join("actor-checkpoint.json"))?,
                )?;
                let value: alife_training::PpoValueHeadCheckpoint = serde_json::from_slice(
                    &std::fs::read(previous.join("value-checkpoint.json"))?,
                )?;
                if actor.source_foundation_digest != asset.digest()
                    || actor.optimizer_step != receipt.actor_optimizer_step
                    || actor.weights.len() != asset.weights().len()
                    || actor
                        .weights
                        .iter()
                        .zip(asset.weights())
                        .any(|(trained, exported)| trained.to_bits() != exported.to_bits())
                    || value.last_updated_policy_version.is_some()
                    || value.optimizer_step != 0
                {
                    return Err("warm-up optimizer/value checkpoint does not match actor".into());
                }
                (
                    asset,
                    1,
                    Some(actor),
                    Some(value),
                    receipt.founder_seed_base,
                )
            } else {
                let continued = load_cycle_continuation(previous, preserve_objective_from)?;
                objective_transition_from = continued.objective_transition_from;
                (
                    continued.asset,
                    continued.policy_version,
                    Some(continued.actor),
                    Some(continued.value),
                    continued.founder_seed_base,
                )
            }
        } else {
            (initial_n2048_care_asset(seed)?, 0, None, None, seed)
        };
    if lesson.is_some()
        && asset.manifest().sensor_profile() != SensorProfile::GroundedTerrainVisionV1
    {
        return Err("navigation curriculum requires explicit --adapt-terrain before resuming an old founder".into());
    }
    let admitted = AdmittedCycleState {
        asset,
        policy_version,
        restored_actor,
        restored_value,
        founder_seed_base,
        objective_state_reset,
        objective_transition_from,
    };
    crate::foundation_training_output::in_new_directory(
        output,
        || {
            run_foundation_training_cycle_in_owned_output(
                output,
                seed,
                training_ticks,
                food_available_world_tick,
                lesson,
                admitted,
                sampling_temperature,
                rehearsal_epochs,
            )
        },
        |error| {
            record_cycle_failure(
                output,
                error,
                seed,
                training_ticks,
                food_available_world_tick,
                lesson,
            );
        },
    )
}

fn record_cycle_failure(
    output: &Path,
    error: &dyn std::error::Error,
    seed: u64,
    training_ticks: usize,
    food_available_world_tick: Option<u64>,
    lesson: Option<FoundationTeacherLesson>,
) {
    let _ = std::fs::write(
        output.join("failure.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "error": error.to_string(),
            "seed": seed,
            "requested_waking_decisions": training_ticks,
            "food_available_world_tick": food_available_world_tick,
            "lesson": lesson,
        }))
        .unwrap_or_default(),
    );
}

struct AdmittedCycleState {
    asset: FoundationWeightAsset,
    policy_version: u64,
    restored_actor: Option<alife_training::FoundationTrainerCheckpoint>,
    restored_value: Option<alife_training::PpoValueHeadCheckpoint>,
    founder_seed_base: u64,
    objective_state_reset: bool,
    objective_transition_from: Option<u16>,
}

fn run_foundation_training_cycle_in_owned_output(
    output: &Path,
    seed: u64,
    training_ticks: usize,
    food_available_world_tick: Option<u64>,
    lesson: Option<FoundationTeacherLesson>,
    admitted: AdmittedCycleState,
    sampling_temperature: f32,
    rehearsal_epochs: u32,
) -> Result<FoundationCycleReceipt> {
    let AdmittedCycleState {
        asset,
        policy_version,
        restored_actor,
        restored_value,
        founder_seed_base,
        objective_state_reset,
        objective_transition_from,
    } = admitted;
    std::fs::write(
        output.join("initial.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    std::fs::write(output.join("phase.txt"), "source-written")?;

    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = founder_seed_base;
    config.sensor_profile = asset.manifest().sensor_profile();
    let mut game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    crate::foundation_training::ground_foundation_training_world(&mut game.world)?;
    game.world.set_age_death_disabled_for_new_game(true)?;
    let mut scenario = if lesson.is_some() {
        Some(configure_foundation_scenario(
            &mut game.world,
            seed,
            lesson,
            None,
            true,
        )?)
    } else {
        None
    };
    if let Some(scenario) = &mut scenario {
        scenario.repeat_vocabulary = lesson != Some(FoundationTeacherLesson::VocabularyProduction);
    }
    let mut creatures = game.creatures;
    // Recovery preconditioning advances ordinary world biology before the
    // durable base is published. Keep the New Game save summaries in step
    // with that same organism record, as the normal checkpoint path does.
    for creature in &mut creatures {
        let biochemistry = game
            .world
            .organism_registry()
            .get(creature.organism_id)
            .ok_or("scenario founder is missing from the world")?
            .biochemistry();
        creature.development_tick = biochemistry.development.last_update_tick;
        creature.mind.tick = biochemistry.tick;
        creature.mind.homeostasis = biochemistry.homeostasis;
    }
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let mut runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        backend,
        game.world,
        seed,
        BrainScaleTier::Standard2048,
        config.sensor_profile,
        alife_archive::LineageLibraryConfig::profile_default(output.join("lineage")),
        format!("n2048-cycle-{seed}"),
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        cycle_sampling_config(seed, policy_version, sampling_temperature)?,
    )?;
    let delayed_food = if food_available_world_tick.is_some() {
        let food_id = runtime
            .world()
            .entity_id("food-01")
            .ok_or("training world is missing its food resource")?;
        let original_position = runtime
            .world()
            .entity(food_id)
            .ok_or("training food resource is missing")?
            .position;
        // Keep food in the legal world but far beyond the founder's practical
        // reach during the gate. Horizontal world coordinates are X and Z.
        let hidden_position =
            runtime.move_player_food(food_id, alife_core::Vec3f::new(390.0, 0.0, 340.0))?;
        Some((food_id, original_position, hidden_position))
    } else {
        None
    };
    let scenario_position = |label| -> Result<[f32; 3]> {
        let entity = runtime
            .world()
            .entity_id(label)
            .ok_or("scenario object is missing")?;
        Ok(runtime
            .world()
            .entity(entity)
            .ok_or("scenario object is missing")?
            .position
            .to_array())
    };
    std::fs::write(
        output.join("scenario.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "world_seed": seed,
            "founder_seed_base": founder_seed_base,
            "food_available_world_tick": food_available_world_tick,
            "sensor_profile": config.sensor_profile,
            "gate_closed_world_tick": scenario.as_ref().and_then(|setup| setup.closing_gate).map(|_| 2),
            "gate_closed_position": scenario.as_ref().and_then(|setup| setup.closing_gate).map(|(_, position)| position.to_array()),
            "lesson": lesson,
            "maze_walls": scenario.as_ref().map(|s|s.maze_walls.iter().map(|(id,p)|(id.raw(),p.to_array())).collect::<Vec<_>>()),
            "vocabulary_token": scenario.as_ref().and_then(|s|s.vocabulary_token),
            "teacher_cue_tokens": scenario.as_ref().map(|s| &s.teacher_cue_tokens),
            "teacher_cue_kind": "contextual_request_not_completed_action_narration",
            "held_food_setup": scenario.as_ref().and_then(|s|s.held_food_setup.as_ref()),
            "grab_food_setup": scenario.as_ref().and_then(|s|s.grab_food_setup.as_ref()),
            "vocabulary_target": scenario.as_ref().and_then(|s|s.vocabulary_target).map(|id|id.raw()),
            "food_position": scenario_position("food-01")?,
            "blocker_position": scenario_position("obstacle-01")?,
            "waypoint_position": scenario_position("obstacle-02")?,
            "hazard_position": scenario_position("hazard-01")?,
            "food_hidden_position": delayed_food.map(|(_, _, position)| position.to_array()),
            "food_original_position": delayed_food.map(|(_, position, _)| position.to_array()),
            "age_death_disabled": true,
        }))?,
    )?;
    // Sleep transitions use the same durable neural authority as an ordinary
    // New Game. Without an exact base, the sleep journal has no publisher.
    let asset_root = output.join("world-assets");
    std::fs::create_dir(&asset_root)?;
    let save_path = output.join("world.json");
    let mut runtime_config =
        alife_world::RuntimeConfig::deterministic_default(seed, BrainScaleTier::Standard2048);
    runtime_config.features.gpu_backend_enabled = true;
    let base = alife_world::PortableSaveFile::from_headless_world(
        format!("n2048-cycle-{seed}"),
        runtime.world(),
        runtime_config,
        alife_world::AssetManifest::empty(),
        creatures,
    )?;
    base.validate_with_asset_root(&asset_root)?;
    runtime.attach_durable_checkpoint_boundary(&save_path, &asset_root, base)?;
    let exact = runtime.capture_portable_checkpoint()?;
    let published = GpuDurableSaveManifest::publish_snapshot(&save_path, &asset_root, &exact)?;
    if published.save != exact {
        return Err("cycle exact checkpoint changed during publication".into());
    }
    runtime.rebind_durable_checkpoint_boundary(&save_path, &asset_root, &exact)?;
    if std::env::var_os("ALIFE_FOUNDATION_PROFILE").is_some() {
        runtime.set_performance_measurement_enabled(true);
    }
    std::fs::write(output.join("phase.txt"), "runtime-admitted")?;

    let organism_id = runtime
        .world()
        .organism_registry()
        .iter()
        .next()
        .ok_or("new training world has no founder")?
        .organism_id();
    let initial_energy = organism_energy(&runtime, organism_id)?;
    let organism_phenotype = runtime
        .world()
        .organism_registry()
        .get(organism_id)
        .ok_or("training founder is missing")?
        .phenotype()
        .clone();
    let mut minimum_energy = initial_energy;
    let mut consumed_events = 0_u64;
    let mut teacher_cue_frames = 0;
    let mut held_food_consumption_events = 0;
    let mut held_food_meal_rows = Vec::new();
    let mut grab_food_acquisitions = Vec::new();
    let mut grab_reward_gate = crate::foundation_grab_food::GrabRewardGate::default();
    let grab_setup = scenario.as_ref().and_then(|s| s.grab_food_setup.as_ref());
    let lesson_food_ownership = |runtime: &GpuLiveBrainRuntime| {
        grab_setup.and_then(|setup| {
            crate::foundation_grab_food::food_ownership(runtime.world(), setup.target)
        })
    };
    let lesson_food_held = |runtime: &GpuLiveBrainRuntime| {
        scenario
            .as_ref()
            .and_then(|s| s.held_food_setup.as_ref())
            .is_some_and(|setup| {
                runtime
                    .world()
                    .entity(setup.target)
                    .is_some_and(|food| food.carried_by == Some(setup.organism) && !food.consumed)
            })
    };
    let observe_lesson = |step: &crate::FoundationTrainingStep, held: bool| {
        scenario.as_ref().map_or((false, false), |s| {
            (
                crate::foundation_training::heard_foundation_teacher_cue(
                    &step.frame,
                    &s.teacher_cue_tokens,
                ),
                crate::foundation_training::held_food_step_consumed(step, s.food, held),
            )
        })
    };
    let mut first_consumed_world_tick = None;
    let mut food_available_elapsed_seconds = None;
    let mut first_consumed_elapsed_seconds = None;
    let mut food_hidden = delayed_food.is_some();

    let started = Instant::now();
    let mut production_tick_seconds = 0.0_f64;
    let mut replay_append_seconds = 0.0_f64;
    let tick_started = Instant::now();
    if let Some(scenario) = &scenario {
        crate::close_foundation_navigation_gate(&mut runtime, scenario)?;
    }
    crate::foundation_training::prime_foundation_lesson_prior(&mut runtime, output, lesson)?;
    let held_before = lesson_food_held(&runtime);
    let grab_before = lesson_food_ownership(&runtime);
    runtime.tick().map_err(|e| {
        let _ = std::fs::write(
            output.join("runtime-performance-failed.json"),
            serde_json::to_vec_pretty(&runtime.performance_metrics()).unwrap_or_default(),
        );
        format!("cycle production tick 0: {e}")
    })?;
    production_tick_seconds += tick_started.elapsed().as_secs_f64();
    let mut first = runtime.take_foundation_training_steps();
    if first.len() != 1 {
        return Err(format!(
            "cycle tick 0: expected one waking decision, got {}",
            first.len()
        )
        .into());
    }
    if first[0].frame.organism_id() != organism_id {
        return Err("first training capture belongs to a different organism".into());
    }
    if let Some(setup) = grab_setup {
        if let Some(acquisition) = crate::foundation_grab_food::observe_grab_acquisition(
            setup,
            grab_before,
            runtime.world(),
            &first[0],
            0,
        )? {
            grab_reward_gate.observe(0);
            grab_food_acquisitions.push(acquisition);
        }
    }
    let (heard_cue, consumed_held) = observe_lesson(&first[0], held_before);
    teacher_cue_frames += usize::from(heard_cue);
    held_food_consumption_events += usize::from(consumed_held);
    if consumed_held {
        held_food_meal_rows.push(0);
    }
    minimum_energy = minimum_energy.min(
        first[0]
            .patch
            .outcome()
            .measured_physiology
            .ok_or("first sealed training patch has no measured physiology")?
            .after
            .body
            .energy,
    );
    if consumed(&first[0]) {
        if food_hidden {
            return Err("food was consumed before the scenario made it available".into());
        }
        consumed_events += 1;
        first_consumed_world_tick = Some(first[0].patch.outcome().outcome_tick.raw());
        first_consumed_elapsed_seconds = Some(started.elapsed().as_secs_f64());
    }
    if food_hidden && runtime.world().tick().raw() >= food_available_world_tick.unwrap_or(u64::MAX)
    {
        if consumed_events != 0 {
            return Err("food was consumed before the scenario made it available".into());
        }
        let (food_id, original_position, _) = delayed_food.ok_or("missing delayed food")?;
        runtime.move_player_food(food_id, original_position)?;
        food_hidden = false;
        food_available_elapsed_seconds = Some(started.elapsed().as_secs_f64());
    }
    std::fs::write(output.join("phase.txt"), "first-capture")?;
    crate::foundation_training::validate_foundation_lesson_prior(&runtime, output, lesson)?;
    let phenotype = first[0].before.phenotype.as_ref().clone();
    let mask = if let Some(checkpoint) = &restored_actor {
        checkpoint.stage_mask.clone()
    } else {
        StageTrainableMask::from_synapse_indices(
            &phenotype,
            &(0..phenotype.synapses().len() as u32).collect::<Vec<_>>(),
        )?
    };
    let staging = runtime.new_staging_like_live()?;
    let mut trainer = FoundationTrainer::from_session(
        alife_runtime::GpuAuthoritativeSession::new(
            staging,
            alife_runtime::GpuSessionConsumerKind::Training,
        ),
        phenotype.clone(),
        asset.clone(),
        mask,
        AdamWConfig::default(),
    )?;
    if let Some(checkpoint) = &restored_actor {
        trainer.restore_checkpoint(checkpoint)?;
        if serde_json::to_vec(&trainer.checkpoint()?)? != serde_json::to_vec(checkpoint)? {
            return Err("restored actor weights/optimizer state changed before training".into());
        }
    }
    let actor_optimizer_step_before = trainer.optimizer_step();
    let initial_checkpoint = serde_json::to_vec(&trainer.checkpoint()?)?;
    let source = foundation_replay_source(&phenotype, &asset, policy_version, &initial_checkpoint)?;
    let budget = FoundationReplayBudget::default();
    let replay_dir = output.join("replay");
    let mut writer = FoundationReplayWriter::new(
        &replay_dir,
        source.clone(),
        phenotype.clone(),
        &asset,
        budget,
    )?;
    std::fs::write(output.join("phase.txt"), "replay-writer-ready")?;
    let mut references = Vec::with_capacity(training_ticks + 1);
    let speech_label = |frame: &alife_core::PerceptionFrame,
                        previous: Option<&alife_core::ExperiencePatch>| {
        crate::foundation_training::grounded_speech_label(
            frame,
            scenario.as_ref().and_then(|s| s.vocabulary_token),
            scenario.as_ref().and_then(|s| s.vocabulary_noun),
            scenario.as_ref().and_then(|s| s.vocabulary_target),
            previous,
            lesson == Some(FoundationTeacherLesson::VocabularyProduction),
        )
    };
    let mut speech_targets = Vec::with_capacity(training_ticks + 1);
    let append_started = Instant::now();
    speech_targets.push(speech_label(&first[0].frame, None));
    let mut last_speech_patch = first[0].patch.clone();
    references.push(writer.append(&first.remove(0))?);
    replay_append_seconds += append_started.elapsed().as_secs_f64();
    let mut gap = false;
    let (mut terminal_biology, mut terminal_death_tick) =
        if let Some(record) = runtime.world().organism_registry().get(organism_id) {
            (
                (!record.lifecycle().is_alive()).then_some(*record.biochemistry()),
                record.lifecycle().death_tick().map(|tick| tick.raw()),
            )
        } else {
            let death_tick = runtime
                .archive_retirement_receipt(organism_id)
                .ok_or("first captured organism missing without archived retirement")?
                .death_tick
                .raw();
            let biology = runtime
                .take_foundation_terminal_biology(organism_id)
                .ok_or("first archived death lacks terminal biology")?;
            if biology.tick.raw() != death_tick {
                return Err("first archived death tick disagrees with terminal biology".into());
            }
            (Some(biology), Some(death_tick))
        };
    let mut stalled_since = Instant::now();
    let mut last_stall_reason = None;
    let world_tick_limit = training_ticks
        .checked_mul(8)
        .and_then(|n| n.checked_add(4_096))
        .ok_or("cycle world-tick limit overflow")?;
    while references.len() <= training_ticks && terminal_biology.is_none() {
        budget.check_additional(64 * 1024 * 1024)?;
        let before = runtime.world().tick().raw();
        if before as usize >= world_tick_limit {
            return Err("cycle reached its world-tick limit before enough waking decisions".into());
        }
        let tick_started = Instant::now();
        if let Some(scenario) = &scenario {
            crate::close_foundation_navigation_gate(&mut runtime, scenario)?;
        }
        if matches!(
            lesson,
            Some(FoundationTeacherLesson::EatHeldFood | FoundationTeacherLesson::GrabFood)
        ) {
            crate::foundation_training::prime_foundation_lesson_prior(
                &mut runtime,
                output,
                lesson,
            )?;
        }
        let held_before = lesson_food_held(&runtime);
        let grab_before = lesson_food_ownership(&runtime);
        let tick_outcome = runtime.tick_outcome().map_err(|e| {
            let _ = std::fs::write(
                output.join("runtime-performance-failed.json"),
                serde_json::to_vec_pretty(&runtime.performance_metrics()).unwrap_or_default(),
            );
            format!("cycle production tick after {before}: {e}")
        })?;
        production_tick_seconds += tick_started.elapsed().as_secs_f64();
        let after = runtime.world().tick().raw();
        if after != before && after % 64 == 0 {
            std::fs::write(
                output.join("phase.txt"),
                format!(
                    "collecting world_tick={after} waking_records={}",
                    references.len()
                ),
            )?;
            std::fs::write(
                output.join("progress.json"),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "world_tick": after,
                    "waking_records": references.len(),
                    "biology": runtime.world().organism_registry().get(organism_id).map(|record| record.biochemistry()),
                    "checkpoint": runtime.exact_checkpoint_performance_state(),
                "retained_learning": format!("{:?}", runtime.retained_learning_recovery(organism_id)),
                "tick_outcome": format!("{tick_outcome:?}"),
                "preparation_errors": format!("{:?}", runtime.last_memory_preparation_errors()),
                "authority": format!("{:?}", runtime.authority_telemetry()),
                }))?,
            )?;
        }
        if after == before {
            let reason = format!("{tick_outcome:?}");
            if last_stall_reason.as_ref() != Some(&reason) {
                std::fs::write(
                    output.join("stall.json"),
                    serde_json::to_vec_pretty(&serde_json::json!({
                        "world_tick": before,
                        "reason": reason,
                    "performance": runtime.performance_metrics(),
                    "checkpoint": runtime.exact_checkpoint_performance_state(),
                    "biology": runtime.world().organism_registry().get(organism_id).map(|record| record.biochemistry()),
                    }))?,
                )?;
                last_stall_reason = Some(reason);
            }
            if stalled_since.elapsed().as_secs() > 300 {
                return Err(format!(
                    "cycle made no world progress for five minutes: {last_stall_reason:?}"
                )
                .into());
            }
            std::thread::yield_now();
            continue;
        }
        if after != before + 1 {
            return Err("cycle world advanced by more than one tick".into());
        }
        if let Some(record) = runtime.world().organism_registry().get(organism_id) {
            if !record.lifecycle().is_alive() {
                terminal_biology = Some(*record.biochemistry());
                terminal_death_tick = record.lifecycle().death_tick().map(|tick| tick.raw());
            }
            minimum_energy = minimum_energy.min(record.biochemistry().body.energy);
        } else {
            let death_tick = runtime
                .archive_retirement_receipt(organism_id)
                .ok_or("cycle organism missing without an archived retirement")?
                .death_tick
                .raw();
            let biology = runtime
                .take_foundation_terminal_biology(organism_id)
                .ok_or("archived death lacks the world-owned terminal biology")?;
            if biology.tick.raw() != death_tick {
                return Err("archived death tick disagrees with terminal biology".into());
            }
            minimum_energy = minimum_energy.min(biology.body.energy);
            terminal_biology = Some(biology);
            terminal_death_tick = Some(death_tick);
        }
        let was_food_hidden = food_hidden;
        if terminal_biology.is_none()
            && food_hidden
            && after >= food_available_world_tick.unwrap_or(u64::MAX)
        {
            if consumed_events != 0 {
                return Err("food was consumed before the scenario made it available".into());
            }
            let (food_id, original_position, _) = delayed_food.ok_or("missing delayed food")?;
            runtime.move_player_food(food_id, original_position)?;
            food_hidden = false;
            food_available_elapsed_seconds = Some(started.elapsed().as_secs_f64());
        }
        stalled_since = Instant::now();
        last_stall_reason = None;
        let mut captured = runtime.take_foundation_training_steps();
        if captured.is_empty() {
            if !runtime.last_memory_preparation_errors().is_empty() {
                return Err(format!(
                    "cycle perception preparation failed at world tick {after}: {:?}",
                    runtime.last_memory_preparation_errors()
                )
                .into());
            }
            if terminal_biology.is_some() {
                break;
            }
            gap = true;
            continue;
        }
        if captured.len() != 1 {
            return Err(format!("cycle world tick {after}: expected at most one decision").into());
        }
        if let Some(setup) = grab_setup {
            if let Some(acquisition) = crate::foundation_grab_food::observe_grab_acquisition(
                setup,
                grab_before,
                runtime.world(),
                &captured[0],
                references.len(),
            )? {
                grab_reward_gate.observe(references.len());
                grab_food_acquisitions.push(acquisition);
            }
        }
        let (heard_cue, consumed_held) = observe_lesson(&captured[0], held_before);
        teacher_cue_frames += usize::from(heard_cue);
        held_food_consumption_events += usize::from(consumed_held);
        if consumed_held {
            held_food_meal_rows.push(references.len());
        }
        if gap {
            verify_foundation_replay_step(&mut trainer, &captured[0])?;
            writer.start_segment()?;
            gap = false;
        }
        if consumed(&captured[0]) {
            if was_food_hidden {
                return Err("food was consumed before the scenario made it available".into());
            }
            consumed_events += 1;
            if first_consumed_world_tick.is_none() {
                first_consumed_world_tick = Some(captured[0].patch.outcome().outcome_tick.raw());
                first_consumed_elapsed_seconds = Some(started.elapsed().as_secs_f64());
            }
        }
        let append_started = Instant::now();
        speech_targets.push(speech_label(&captured[0].frame, Some(&last_speech_patch)));
        last_speech_patch = captured[0].patch.clone();
        references.push(writer.append(&captured.remove(0))?);
        replay_append_seconds += append_started.elapsed().as_secs_f64();
        if terminal_biology.is_some() {
            break;
        }
    }
    crate::foundation_training::validate_foundation_lesson_prior(&runtime, output, lesson)?;
    let collection_seconds = started.elapsed().as_secs_f64();
    let train_rows = references.len() - usize::from(terminal_biology.is_none());
    let (bootstrap_replay_rows, trained_held_food_consumption_events) =
        cycle_replay_dose(references.len(), train_rows, &held_food_meal_rows)?;
    if train_rows == 0 {
        return Err("cycle has no complete training transition".into());
    }
    let delayed_food_gate_passed = food_available_world_tick.is_some_and(|available| {
        terminal_biology.is_none()
            && !food_hidden
            && first_consumed_world_tick.is_some_and(|meal| meal > available)
            && food_available_elapsed_seconds.is_some()
    });
    let world_ticks_elapsed = runtime.world().tick().raw();
    let final_energy = if let Some(biology) = terminal_biology {
        biology.body.energy
    } else {
        organism_energy(&runtime, organism_id)?
    };
    std::fs::write(
        output.join("timing.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "production_tick_seconds": production_tick_seconds,
            "replay_append_seconds": replay_append_seconds,
            "collection_total_seconds": collection_seconds,
        }))?,
    )?;
    std::fs::write(
        output.join("runtime-performance.json"),
        serde_json::to_vec_pretty(&runtime.performance_metrics())?,
    )?;
    std::fs::write(output.join("phase.txt"), "collection-complete")?;
    std::fs::write(
        output.join("replay-manifest.json"),
        serde_json::to_vec(&references)?,
    )?;
    let value_optimizer_step_before = restored_value
        .as_ref()
        .map_or(0, |checkpoint| checkpoint.optimizer_step);
    let mut expected_value_checkpoint = restored_value
        .as_ref()
        .map(serde_json::to_vec)
        .transpose()?;
    let mut value = if let Some(checkpoint) = restored_value {
        PpoTrainingState::from_checkpoint(checkpoint)?
    } else {
        PpoTrainingState::default()
    };
    // The exact production start state is stored at every replay row. Predict
    // old values before any update in bounded chunks; keep only scalar targets.
    let mut values = Vec::with_capacity(references.len());
    let mut actions_rewards: Vec<(PpoJointAction, f32)> = Vec::with_capacity(train_rows);
    let mut previous_physiology_after = None;
    let mut sleep_gap_reward_total = 0.0_f32;
    let mut blocked_actions = 0_u64;
    let mut collision_actions = 0_u64;
    let mut avoid_actions = 0_u64;
    let mut rest_recovery_actions = 0_u64;
    let ppo_config = cycle_ppo_config(sampling_temperature)?;
    let mut segment_start = 0;
    while segment_start < references.len() {
        let segment = references[segment_start].segment;
        let segment_end = references[segment_start..]
            .iter()
            .position(|reference| reference.segment != segment)
            .map_or(references.len(), |offset| segment_start + offset);
        for (chunk, refs) in references[segment_start..segment_end]
            .chunks(512)
            .enumerate()
        {
            let window =
                load_foundation_replay_window(&replay_dir, refs, 0, &source, &phenotype, budget)?;
            values.extend(value.predict_values(&mut trainer, &window.sequence)?);
            if let Some(expected) = expected_value_checkpoint.take() {
                if serde_json::to_vec(&value.checkpoint(trainer.session())?)? != expected {
                    return Err(
                        "restored value weights/optimizer state changed before training".into(),
                    );
                }
            }
            for (row, (behavior, patch)) in window.behavior.iter().zip(&window.patches).enumerate()
            {
                let index = segment_start + chunk * 512 + row;
                let physiology = patch
                    .outcome()
                    .measured_physiology
                    .ok_or("sealed training patch has no measured physiology")?;
                if index > 0 && references[index].tick > references[index - 1].tick + 1 {
                    let gap = alife_core::MeasuredPhysiologyTransition::new(
                        previous_physiology_after.ok_or("sleep gap has no prior physiology")?,
                        physiology.before,
                    )?;
                    let gap_seconds =
                        (references[index].tick - references[index - 1].tick - 1) as f64 / 20.0;
                    let discount = (-std::f64::consts::LN_2 * gap_seconds
                        / f64::from(ppo_config.discount_half_life_seconds))
                    .exp() as f32;
                    let delayed = training_biological_value(&gap, &organism_phenotype)? * discount;
                    actions_rewards[index - 1].1 =
                        (actions_rewards[index - 1].1 + delayed).clamp(-1.0, 1.0);
                    sleep_gap_reward_total += delayed;
                }
                previous_physiology_after = Some(physiology.after);
                if index == train_rows {
                    break; // Real next state is a value bootstrap, never a loss row.
                }
                let outcome = patch.outcome();
                let contacts = std::iter::once(outcome.physical.contact).chain(
                    outcome.joint.iter().flat_map(|joint| {
                        joint
                            .channel_outcomes
                            .iter()
                            .map(|channel| channel.physical.contact)
                    }),
                );
                let mut blocked = false;
                let mut collision = false;
                for contact in contacts {
                    blocked |= contact == alife_core::PhysicalContactKind::Blocked;
                    collision |= contact == alife_core::PhysicalContactKind::Collision;
                }
                blocked_actions += u64::from(blocked);
                collision_actions += u64::from(collision);
                let family = patch.decision().neural_evidence()?.action_family;
                avoid_actions += u64::from(family == alife_core::CandidateActionFamily::Avoid);
                rest_recovery_actions += u64::from(
                    family == alife_core::CandidateActionFamily::Rest
                        && physiology.after.homeostasis.drives.fatigue
                            <= physiology.before.homeostasis.drives.fatigue - 0.02,
                );
                let receptors = physiology
                    .before
                    .neural_receptor_frame(&organism_phenotype)?;
                let modulator = alife_core::OutcomeCreditPacket::from_sealed_patch(patch)?
                    .with_biochemical_receptors(&receptors)?
                    .modulator();
                actions_rewards.push((
                    joint_action(behavior)?,
                    training_receptor_profile(&organism_phenotype).project(&modulator.frame())?,
                ));
            }
        }
        segment_start = segment_end;
    }
    if values.len() != references.len() {
        return Err("GPU value prediction count does not match decisions".into());
    }
    if actions_rewards.len() != train_rows {
        return Err("PPO action/reward count does not match training decisions".into());
    }
    if let Some(terminal) = terminal_biology {
        let before = previous_physiology_after.ok_or("terminal life has no sealed physiology")?;
        let transition = alife_core::MeasuredPhysiologyTransition::new(before, terminal)?;
        let elapsed = (terminal.tick.raw() - before.tick.raw()) as f64 / 20.0;
        let discount = (-std::f64::consts::LN_2 * elapsed
            / f64::from(ppo_config.discount_half_life_seconds))
        .exp() as f32;
        let delayed = training_biological_value(&transition, &organism_phenotype)? * discount;
        let final_reward = &mut actions_rewards
            .last_mut()
            .ok_or("terminal life has no trainable action")?
            .1;
        *final_reward = (*final_reward + delayed).clamp(-1.0, 1.0);
        sleep_gap_reward_total += delayed;
    }
    // Keep the ordinary measured biological value, including sleep/terminal
    // gaps, intact. Only a captured learner acquisition loss row adds this
    // bounded offline curriculum component; the real bootstrap row never does.
    let mut curriculum_reward_rows = Vec::new();
    if grab_setup.is_some() {
        for (row, (_, physiological_reward)) in actions_rewards.iter_mut().enumerate() {
            let component = crate::foundation_grab_food::curriculum_reward_row(
                row,
                *physiological_reward,
                grab_reward_gate.reward(row, train_rows),
            )?;
            *physiological_reward = component.combined_reward;
            curriculum_reward_rows.push(component);
        }
    }
    let curriculum_reward_total = curriculum_reward_rows
        .iter()
        .map(|row| row.curriculum_reward)
        .sum();
    let trained_grab_food_acquisitions = grab_food_acquisitions
        .iter()
        .filter(|event| event.row < train_rows)
        .count();
    let mut transitions = Vec::with_capacity(train_rows);
    for (tick, (action, reward)) in actions_rewards.into_iter().enumerate() {
        let terminal = terminal_death_tick.is_some() && tick + 1 == train_rows;
        transitions.push(PpoTransition {
            policy_version,
            trajectory_id: seed,
            step: tick as u64,
            action,
            reward,
            old_value: values[tick],
            next_value: if terminal { 0.0 } else { values[tick + 1] },
            elapsed_seconds: (if terminal {
                terminal_death_tick.ok_or("missing terminal death tick")?
            } else {
                references[tick + 1].tick
            } - references[tick].tick) as f32
                / 20.0,
            boundary: if terminal {
                PpoBoundary::Terminated
            } else if tick + 1 == train_rows {
                PpoBoundary::Truncated
            } else {
                PpoBoundary::Continuing
            },
        });
    }
    let batch = PpoBatch::from_rollout(policy_version, transitions, ppo_config)?;
    let started = Instant::now();
    let mut spans = Vec::new();
    let mut segment_start = 0;
    while segment_start < train_rows {
        let segment = references[segment_start].segment;
        let segment_end = references[segment_start..train_rows]
            .iter()
            .position(|reference| reference.segment != segment)
            .map_or(train_rows, |offset| segment_start + offset);
        for start in (segment_start..segment_end).step_by(256) {
            let end = (start + 256).min(segment_end);
            let burn_start = start.saturating_sub(128).max(segment_start);
            spans.push((burn_start, start, end));
        }
        segment_start = segment_end;
    }
    let update = train_recurrent_ppo_cohort(
        &mut trainer,
        &mut value,
        spans.len(),
        8,
        |index| {
            let (burn_start, start, end) = spans[index];
            let replay = load_foundation_replay_window(
                &replay_dir,
                &references[burn_start..end],
                start - burn_start,
                &source,
                &phenotype,
                budget,
            )
            .map_err(|_| {
                alife_training::TrainingError::from(ScaffoldContractError::InvalidDecisionEvidence)
            })?;
            Ok(PpoTrainingWindow {
                speech_targets: speech_targets[burn_start..end]
                    .iter()
                    .enumerate()
                    .map(|(i, t)| if i < start - burn_start { None } else { *t })
                    .collect(),
                sequence: replay.sequence,
                batch: batch.window(start..end)?,
                auxiliary: None,
            })
        },
        ppo_config,
        policy_version,
    )?;
    let acquisition_rehearsal = if rehearsal_epochs != 0 {
        std::fs::write(output.join("phase.txt"), "acquisition-rehearsal")?;
        let event = grab_food_acquisitions.iter().find(|event| {
            event.row < train_rows && grab_reward_gate.reward(event.row, train_rows) > 0.0
        });
        Some(
            crate::foundation_acquisition_rehearsal::rehearse_acquisition(
                &mut trainer,
                &mut value,
                rehearsal_epochs,
                event,
                crate::foundation_acquisition_rehearsal::AcquisitionRehearsalContext {
                    directory: &replay_dir,
                    references: &references,
                    train_rows,
                    source: &source,
                    phenotype: &phenotype,
                    budget,
                },
            )?,
        )
    } else {
        None
    };
    let final_actor_optimizer_step = trainer.optimizer_step();
    std::fs::write(output.join("phase.txt"), "update-complete")?;
    let update_seconds = started.elapsed().as_secs_f64();
    if update.actor_optimizer_step == 0 || update.value_optimizer_step == 0 {
        return Err("cycle completed without an actor and value update".into());
    }
    let completed_stage_count =
        u16::try_from(policy_version.checked_add(1).ok_or("stage overflow")?)?;
    let trained =
        trainer.export_candidate(TrainingStageManifest::new(1, 1, completed_stage_count))?;
    std::fs::write(
        output.join("trained.alife-foundation"),
        trained.encode_canonical()?,
    )?;
    // A newly born cohort must accept the exact exported native asset through
    // the same admission path as a regular game, then produce a GPU decision.
    // The exported asset is keyed to this native founder genome. A successor
    // world may be fresh, but must use the same genotype for exact admission.
    let mut next_config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    next_config.brain_class = BrainScaleTier::Standard2048;
    next_config.founder_seed_base = founder_seed_base;
    next_config.sensor_profile = trained.manifest().sensor_profile();
    let bytes = std::fs::read(output.join("trained.alife-foundation"))?;
    let admitted_asset = FoundationWeightAsset::decode_canonical(&bytes)?;
    if admitted_asset.digest() != trained.digest() {
        return Err("exported asset digest changed on disk".into());
    }
    let mut next_game =
        alife_world::create_canonical_new_game_with_n2048_candidate(&next_config, &admitted_asset)?;
    crate::foundation_training::ground_foundation_training_world(&mut next_game.world)?;
    next_game.world.set_age_death_disabled_for_new_game(true)?;
    let next_backend = runtime.new_staging_like_live()?;
    let mut next_runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        next_backend,
        next_game.world,
        seed,
        BrainScaleTier::Standard2048,
        next_config.sensor_profile,
        alife_archive::LineageLibraryConfig::profile_default(output.join("next-lineage")),
        format!("n2048-cycle-next-{}", seed + 1),
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        cycle_sampling_config(seed, policy_version.wrapping_add(1), sampling_temperature)?,
    )?;
    next_runtime.tick()?;
    let next_steps = next_runtime.take_foundation_training_steps();
    let next_cohort_tick_captured = next_steps.len() == 1;
    if !next_cohort_tick_captured {
        return Err("trained asset next cohort did not produce a decision".into());
    }
    trainer.rebind_for_next_cohort(
        next_steps[0].before.phenotype.as_ref().clone(),
        admitted_asset,
    )?;
    let next_cohort_optimizer_rebound = true;
    std::fs::write(
        output.join("actor-checkpoint.json"),
        serde_json::to_vec(&trainer.checkpoint()?)?,
    )?;
    std::fs::write(
        output.join("value-checkpoint.json"),
        serde_json::to_vec(&value.checkpoint(trainer.session())?)?,
    )?;
    std::fs::write(output.join("phase.txt"), "next-cohort-admitted")?;
    let receipt = FoundationCycleReceipt {
        biological_objective_version: BIOLOGICAL_OBJECTIVE_VERSION,
        objective_state_reset,
        objective_transition_from,
        sampling_temperature,
        acquisition_rehearsal,
        seed,
        founder_seed_base,
        policy_version,
        training_ticks: train_rows,
        requested_training_ticks: training_ticks,
        terminal_death_tick,
        delayed_food_gate_passed,
        world_ticks_elapsed,
        consumed_events,
        blocked_actions,
        collision_actions,
        avoid_actions,
        rest_recovery_actions,
        food_available_world_tick,
        lesson,
        first_consumed_world_tick,
        food_available_elapsed_seconds,
        first_consumed_elapsed_seconds,
        initial_energy,
        minimum_energy,
        final_energy,
        sleep_gap_reward_total,
        semantic_prior: runtime.semantic_prior_metrics().cloned(),
        speech_target_rows: speech_targets[..train_rows].iter().flatten().count(),
        teacher_cue_frames,
        teacher_cue_tokens: scenario
            .as_ref()
            .map_or_else(Vec::new, |s| s.teacher_cue_tokens.clone()),
        held_food_setup: scenario.as_ref().and_then(|s| s.held_food_setup.clone()),
        grab_food_setup: grab_setup.cloned(),
        grab_food_acquisitions,
        trained_grab_food_acquisitions,
        prepared_grab_food_opportunities: usize::from(grab_setup.is_some()),
        grab_food_curriculum_version: grab_setup
            .map(|_| crate::foundation_grab_food::GRAB_FOOD_CURRICULUM_VERSION),
        curriculum_reward_rows,
        curriculum_reward_total,
        held_food_consumption_events,
        trained_held_food_consumption_events,
        prepared_held_food_opportunities: usize::from(
            scenario
                .as_ref()
                .is_some_and(|s| s.held_food_setup.is_some()),
        ),
        captured_replay_rows: references.len(),
        trained_replay_rows: train_rows,
        bootstrap_replay_rows,
        actor_optimizer_updates: final_actor_optimizer_step
            .checked_sub(actor_optimizer_step_before)
            .ok_or("cycle actor optimizer step regressed")?,
        value_optimizer_updates: update
            .value_optimizer_step
            .checked_sub(value_optimizer_step_before)
            .ok_or("cycle value optimizer step regressed")?,
        collection_seconds,
        update_seconds,
        old_asset_digest: digest(&asset),
        new_asset_digest: digest(&trained),
        actor_optimizer_step: final_actor_optimizer_step,
        value_optimizer_step: update.value_optimizer_step,
        completed_epochs: update.completed_epochs,
        next_cohort_tick_captured,
        next_cohort_optimizer_rebound,
    };
    std::fs::write(
        output.join("cycle.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
}
