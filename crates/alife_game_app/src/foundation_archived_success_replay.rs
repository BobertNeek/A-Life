//! Offline imitation of immutable successful learner records from older actors.
use std::{
    collections::HashSet,
    io::Read,
    path::{Path, PathBuf},
};

use crate::{
    FoundationCycleReceipt, FoundationReplayBudget, FoundationReplayRecordRef,
    FoundationReplaySource,
};
use alife_core::{
    Blake3Digest, BrainPhenotype, BrainScaleTier, FoundationWeightAsset, MotorChannel,
    PhysicalContactKind, ScaffoldContractError,
};
use alife_training::{
    train_recurrent_imitation, FoundationTrainer, FoundationTrainerCheckpoint, ImitationExample,
    ImitationTarget, ImitationTrainingWindow, PpoTrainingState,
};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MAX_EXAMPLES: usize = 128;
const MAX_EPOCHS: u32 = 128;
const BATCH_SIZE: usize = 8;

pub(crate) fn read_archive_bytes(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1024 * 1024 {
        return Err("archived success manifest exceeds 1 MiB".into());
    }
    Ok(bytes)
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ArchivedSuccessEntry {
    pub episode: PathBuf,
    pub previous: PathBuf,
    pub row: usize,
    pub action: u32,
    pub source: FoundationReplaySource,
    pub record_digest: [u8; 32],
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ArchivedSuccessManifest {
    pub schema: u32,
    pub founder_seed_base: u64,
    pub entries: Vec<ArchivedSuccessEntry>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ArchivedSuccessReplayReceipt {
    pub manifest_digest: Blake3Digest,
    pub requested_epochs: u32,
    pub completed_epochs: u32,
    pub grab_examples: usize,
    pub eating_examples: usize,
    pub effective_batch_size: usize,
    pub imitation_temperature: f32,
    pub actor_optimizer_step_before: u32,
    pub actor_optimizer_step_after: u32,
    pub value_checkpoint_unchanged: bool,
    pub original_rewards_preserved: bool,
    pub action_losses: Vec<f32>,
    pub examples: Vec<ArchivedSuccessEntry>,
}

/// Read-only native validation of an archive against one sealed trained source.
/// No GPU device, trainer, forward pass, or optimizer update is created.
#[derive(Debug, serde::Serialize)]
pub struct ArchivedSuccessInspection {
    pub manifest_digest: Blake3Digest,
    pub source_policy_version: u64,
    pub founder_seed_base: u64,
    pub checked_examples: usize,
    pub grab_examples: usize,
    pub eating_examples: usize,
    pub neural_execution: bool,
    pub weights_modified: bool,
}

/// Exact-file evidence from read-only checkpoint admission, not a promotion or
/// behavioral receipt. File digests bind the bytes that were actually checked.
#[derive(Debug, serde::Serialize)]
pub struct FoundationCheckpointInspection {
    pub expected_policy_version: u64,
    pub policy_version: u64,
    pub founder_seed_base: u64,
    pub foundation_digest: Blake3Digest,
    pub actor_optimizer_step: u32,
    pub value_optimizer_step: u32,
    pub file_digests: Vec<FoundationCheckpointFileDigest>,
    pub neural_execution: bool,
    pub weights_modified: bool,
    pub archive_checked: bool,
}

#[derive(Debug, serde::Serialize)]
pub struct FoundationCheckpointFileDigest {
    pub file_name: &'static str,
    pub encoded_bytes: usize,
    pub digest: Blake3Digest,
    /// Portable binding for report consumers using the standard SHA-256 API.
    pub sha256: String,
}

fn read_checkpoint_file(directory: &Path, file_name: &str) -> Result<Vec<u8>> {
    Ok(std::fs::read(directory.join(file_name)).map_err(|error| {
        std::io::Error::new(error.kind(), format!("cannot read {file_name}: {error}"))
    })?)
}

/// Validate one exact sealed-cycle checkpoint without an archive, GPU session,
/// neural execution, optimizer updates, runtime stepping or input writes.
/// Founder construction below compiles CPU metadata only, as in archive inspection.
pub fn inspect_foundation_checkpoint(
    source_cycle: &Path,
    expected_policy_version: u64,
) -> Result<FoundationCheckpointInspection> {
    for marker in ["adaptation.json", "warmup.json"] {
        match std::fs::symlink_metadata(source_cycle.join(marker)) {
            Ok(_) => {
                return Err(format!("checkpoint inspection rejects route marker {marker}").into())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    let cycle_bytes = read_checkpoint_file(source_cycle, "cycle.json")?;
    let cycle: FoundationCycleReceipt = serde_json::from_slice(&cycle_bytes)?;
    if cycle.policy_version != expected_policy_version {
        return Err(format!(
            "checkpoint policy mismatch: expected {expected_policy_version}, found {}",
            cycle.policy_version
        )
        .into());
    }
    cycle
        .policy_version
        .checked_add(1)
        .ok_or("policy version overflow")?;
    if cycle.biological_objective_version != 2
        || cycle.objective_state_reset
        || cycle.founder_seed_base == 0
    {
        return Err("checkpoint inspection requires an objective2 source with a nonzero founder and no state reset".into());
    }
    let asset_bytes = read_checkpoint_file(source_cycle, "trained.alife-foundation")?;
    let asset = FoundationWeightAsset::decode_canonical(&asset_bytes)?;
    let asset_digest = asset.digest();
    let digest_hex: String = asset_digest
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if digest_hex != cycle.new_asset_digest {
        return Err("checkpoint export digest disagrees with its cycle".into());
    }
    let actor_bytes = read_checkpoint_file(source_cycle, "actor-checkpoint.json")?;
    let actor: FoundationTrainerCheckpoint = serde_json::from_slice(&actor_bytes)?;
    let value_bytes = read_checkpoint_file(source_cycle, "value-checkpoint.json")?;
    let value: alife_training::PpoValueHeadCheckpoint = serde_json::from_slice(&value_bytes)?;
    crate::foundation_critic_correction::validate_cycle_optimizer_handoff_at(
        source_cycle,
        &cycle,
        &asset,
        &actor,
        &value,
    )?;
    let current = compile_source(asset, cycle.seed, cycle.founder_seed_base)?;
    actor.validate_for(&current, asset_digest)?;
    if value.feature_count != current.neuron_count() {
        return Err("checkpoint value head is bound to a different feature count".into());
    }
    let value_optimizer_step = value.optimizer_step;
    PpoTrainingState::from_checkpoint(value)?;
    let file_digests = [
        ("cycle.json", cycle_bytes),
        ("trained.alife-foundation", asset_bytes),
        ("actor-checkpoint.json", actor_bytes),
        ("value-checkpoint.json", value_bytes),
    ]
    .into_iter()
    .map(|(file_name, bytes)| FoundationCheckpointFileDigest {
        file_name,
        encoded_bytes: bytes.len(),
        digest: Blake3Digest::from_bytes(*blake3::hash(&bytes).as_bytes()),
        sha256: format!("{:x}", Sha256::digest(&bytes)),
    })
    .collect();
    Ok(FoundationCheckpointInspection {
        expected_policy_version,
        policy_version: cycle.policy_version,
        founder_seed_base: cycle.founder_seed_base,
        foundation_digest: asset_digest,
        actor_optimizer_step: actor.optimizer_step,
        value_optimizer_step,
        file_digests,
        neural_execution: false,
        weights_modified: false,
        archive_checked: false,
    })
}

/// Validate the same native source, graph, replay and action-label contracts
/// used by archived rehearsal, without allocating its GPU training state.
pub fn inspect_archived_successes(
    source_cycle: &Path,
    manifest_path: &Path,
) -> Result<ArchivedSuccessInspection> {
    let progress = std::env::var_os("ALIFE_ARCHIVE_INSPECT_PROGRESS").is_some();
    let started = std::time::Instant::now();
    if progress {
        eprintln!("archive inspection: reading selected source");
    }
    let bytes = read_archive_bytes(manifest_path)?;
    let manifest: ArchivedSuccessManifest = serde_json::from_slice(&bytes)?;
    validate_archive_request(&manifest, 1)?;
    let cycle: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(source_cycle.join("cycle.json"))?)?;
    if cycle.biological_objective_version != 2
        || cycle.objective_state_reset
        || cycle.founder_seed_base == 0
        || manifest.founder_seed_base != cycle.founder_seed_base
    {
        return Err(
            "archive inspection requires a compatible sealed objective2 source of the same founder"
                .into(),
        );
    }
    // The sealed source P resumes as P+1. Match rehearsal's episode ordering,
    // while keeping each archived record bound to its original collection P.
    let continuation_policy_version = cycle
        .policy_version
        .checked_add(1)
        .ok_or("policy version overflow")?;
    let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
        source_cycle.join("trained.alife-foundation"),
    )?)?;
    let asset_digest: String = asset
        .digest()
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    if asset_digest != cycle.new_asset_digest {
        return Err("inspection source export digest disagrees with its cycle".into());
    }
    let actor: FoundationTrainerCheckpoint =
        serde_json::from_slice(&std::fs::read(source_cycle.join("actor-checkpoint.json"))?)?;
    let value: alife_training::PpoValueHeadCheckpoint =
        serde_json::from_slice(&std::fs::read(source_cycle.join("value-checkpoint.json"))?)?;
    crate::foundation_critic_correction::validate_cycle_optimizer_handoff_at(
        source_cycle,
        &cycle,
        &asset,
        &actor,
        &value,
    )?;
    if progress {
        eprintln!(
            "archive inspection: source handoff checked at {:.3}s; compiling source graph",
            started.elapsed().as_secs_f64()
        );
    }
    let source_foundation_digest = asset.digest();
    let current = compile_source(asset, cycle.seed, cycle.founder_seed_base)?;
    actor.validate_for(&current, source_foundation_digest)?;
    if value.feature_count != current.neuron_count() {
        return Err("inspection source value head is bound to a different feature count".into());
    }
    // This validates the value payload and holds a pending restore only. No
    // replay buffers or GPU objective are created by this constructor.
    PpoTrainingState::from_checkpoint(value)?;
    let mut grabs = 0;
    for (index, entry) in manifest.entries.iter().enumerate() {
        if progress {
            eprintln!(
                "archive inspection: preparing example {}/{} at {:.3}s",
                index + 1,
                manifest.entries.len(),
                started.elapsed().as_secs_f64()
            );
        }
        let prepared = prepare(
            entry,
            cycle.founder_seed_base,
            &current,
            continuation_policy_version,
        )?;
        if progress {
            eprintln!(
                "archive inspection: loading example {} at {:.3}s",
                index + 1,
                started.elapsed().as_secs_f64()
            );
        }
        load(entry, &prepared, &current)?;
        grabs += usize::from(entry.action == 211);
    }
    Ok(ArchivedSuccessInspection {
        manifest_digest: Blake3Digest::from_bytes(*blake3::hash(&bytes).as_bytes()),
        source_policy_version: cycle.policy_version,
        founder_seed_base: cycle.founder_seed_base,
        checked_examples: manifest.entries.len(),
        grab_examples: grabs,
        eating_examples: manifest.entries.len() - grabs,
        neural_execution: false,
        weights_modified: false,
    })
}

pub(crate) fn validate_archive_request(
    manifest: &ArchivedSuccessManifest,
    epochs: u32,
) -> Result<()> {
    if manifest.schema != 1
        || manifest.founder_seed_base == 0
        || manifest.entries.is_empty()
        || manifest.entries.len() > MAX_EXAMPLES
        || !(1..=MAX_EPOCHS).contains(&epochs)
    {
        return Err(
            "archived success replay requires schema1, 1..128 examples and 1..128 epochs".into(),
        );
    }
    let mut identities = HashSet::new();
    for entry in &manifest.entries {
        if !matches!(entry.action, 210 | 211) || !identities.insert(entry.record_digest) {
            return Err("archived replay requires distinct genuine Grab/Eat records".into());
        }
    }
    Ok(())
}

fn graph_without_weights(phenotype: &BrainPhenotype) -> Result<serde_json::Value> {
    let mut value: serde_json::Value = serde_json::from_slice(&serde_json::to_vec(phenotype)?)?;
    let object = value.as_object_mut().ok_or("phenotype is not an object")?;
    object.remove("phenotype_hash");
    object.remove("compiler_inputs_digest");
    let abi = object
        .get_mut("foundation_abi_selection")
        .ok_or("foundation ABI missing")?;
    if abi["abi_selection"] != "CanonicalV2" {
        return Err("archived replay requires the native canonical foundation ABI".into());
    }
    // Payload identity changes with weights/training provenance. Its actual
    // original asset is checked separately; keep weight count and every ABI field.
    abi["contract"]["weight_asset"]
        .as_object_mut()
        .ok_or("foundation weight asset binding missing")?
        .remove("digest");
    for synapse in object
        .get_mut("synapses")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("synapses missing")?
    {
        synapse
            .as_object_mut()
            .ok_or("synapse is not an object")?
            .remove("genetic_weight");
    }
    Ok(value)
}

struct PreparedExample {
    phenotype: BrainPhenotype,
    references: Vec<FoundationReplayRecordRef>,
    cycle: FoundationCycleReceipt,
    burn_start: usize,
    end: usize,
    weight: f32,
}

pub(crate) fn compile_source(
    asset: FoundationWeightAsset,
    seed: u64,
    founder: u64,
) -> Result<BrainPhenotype> {
    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = founder;
    config.sensor_profile = asset.manifest().sensor_profile();
    let game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    let organism = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("archived founder missing")?;
    let genome = organism.phenotype().brain_genome.clone();
    let development = crate::gpu_live_runtime::foundation_construction_development(
        &genome,
        &alife_core::BrainCapacityClass::n2048(),
        &organism
            .phenotype()
            .development_state_at(alife_core::Tick::ZERO)?,
    )?;
    Ok(
        alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            asset,
        )?
        .0,
    )
}

fn prepare(
    entry: &ArchivedSuccessEntry,
    founder: u64,
    current: &BrainPhenotype,
    current_policy: u64,
) -> Result<PreparedExample> {
    let cycle: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(entry.episode.join("cycle.json"))?)?;
    let parent: FoundationCycleReceipt =
        serde_json::from_slice(&std::fs::read(entry.previous.join("cycle.json"))?)?;
    if cycle.biological_objective_version != 2
        || cycle.objective_state_reset
        || cycle.founder_seed_base != founder
        || !cycle.next_cohort_tick_captured
        || !cycle.next_cohort_optimizer_rebound
        || cycle.policy_version >= current_policy
        || entry.source.policy_version != cycle.policy_version
        || parent.policy_version.checked_add(1) != Some(cycle.policy_version)
    {
        return Err("archived episode is not a sealed older continuation of this founder".into());
    }
    let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
        entry.episode.join("initial.alife-foundation"),
    )?)?;
    let parent_asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
        entry.previous.join("trained.alife-foundation"),
    )?)?;
    let actor: FoundationTrainerCheckpoint = serde_json::from_slice(&std::fs::read(
        entry.previous.join("actor-checkpoint.json"),
    )?)?;
    if parent_asset.digest() != asset.digest()
        || entry.source.foundation_asset_digest != asset.digest()
        || entry.source.actor_checkpoint_digest.bytes()
            != blake3::hash(&serde_json::to_vec(&actor)?).as_bytes()
    {
        return Err("archived replay source disagrees with its actual frozen parent actor".into());
    }
    let phenotype = compile_source(asset, cycle.seed, founder)?;
    if entry.source.phenotype_hash != phenotype.phenotype_hash()
        || entry.source.compiler_inputs_digest != phenotype.compiler_inputs_digest()
        || graph_without_weights(&phenotype)? != graph_without_weights(current)?
    {
        return Err("archived replay requires the same complete graph, encoder and decoder, allowing only changed genetic weights".into());
    }
    let references: Vec<FoundationReplayRecordRef> =
        serde_json::from_slice(&std::fs::read(entry.episode.join("replay-manifest.json"))?)?;
    if references
        .get(entry.row)
        .is_none_or(|row| row.digest != entry.record_digest)
    {
        return Err("archived selection disagrees with the sealed replay manifest".into());
    }
    let (burn_start, end) = crate::foundation_acquisition_rehearsal::rehearsal_window(
        &references,
        cycle.trained_replay_rows,
        entry.row,
    )?;
    Ok(PreparedExample {
        phenotype,
        references,
        cycle,
        burn_start,
        end,
        weight: 1.0,
    })
}

fn load(
    entry: &ArchivedSuccessEntry,
    prepared: &PreparedExample,
    current: &BrainPhenotype,
) -> Result<ImitationTrainingWindow> {
    let replay = crate::load_foundation_replay_window(
        &entry.episode.join("replay"),
        &prepared.references[prepared.burn_start..prepared.end],
        entry.row - prepared.burn_start,
        &entry.source,
        &prepared.phenotype,
        FoundationReplayBudget::default(),
    )?;
    let behavior = replay.behavior.last().ok_or("archived behavior missing")?;
    let patch = replay.patches.last().ok_or("archived patch missing")?;
    let example = if entry.action == 211 {
        let event = prepared
            .cycle
            .grab_food_acquisitions
            .iter()
            .find(|event| event.row == entry.row)
            .ok_or("selected row is not a genuine acquisition")?;
        if patch.header().sequence_id.raw() != event.sequence_id
            || patch.outcome().outcome_tick.raw() != event.outcome_tick
            || !patch.selected_bundle().is_some_and(|bundle| {
                bundle
                    .channels
                    .iter()
                    .any(|command| command == &event.selected_command)
            })
            || !patch.outcome().joint.as_ref().is_some_and(|joint| {
                joint.channel_outcomes.iter().any(|outcome| {
                    outcome.channel == MotorChannel::Manipulation
                        && outcome.physical == event.physical
                })
            })
        {
            return Err("archived Grab label disagrees with the original sealed outcome".into());
        }
        crate::foundation_acquisition_rehearsal::acquisition_target(behavior, event)?
    } else {
        behavior.on_policy_log_probability()?; // authenticates original collection; never used as current PPO data.
        let target = prepared
            .cycle
            .grab_food_setup
            .as_ref()
            .ok_or("archived eating has no acquisition setup")?
            .target;
        if !prepared
            .cycle
            .grab_food_acquisitions
            .iter()
            .any(|event| event.row < entry.row && event.target == target)
            || !patch.selected_bundle().is_some_and(|bundle| {
                bundle.channels.iter().any(|command| {
                    command.channel == MotorChannel::Manipulation
                        && command.primitive == alife_world::HeadlessActionIds::EAT
                        && command.target.and_then(|target| target.entity) == Some(target)
                })
            })
            || !patch.outcome().joint.as_ref().is_some_and(|joint| {
                joint.channel_outcomes.iter().any(|outcome| {
                    outcome.channel == MotorChannel::Manipulation
                        && outcome.physical.contact == PhysicalContactKind::Consumed
                        && outcome.physical.target_entity == Some(target)
                })
            })
        {
            return Err("archived eating label lacks a real acquisition followed by consumption of the same target".into());
        }
        let index = usize::from(behavior.motor_indices[2]);
        if index >= 32 || behavior.forced_motor_slots.get(index) != Some(&2) {
            return Err("archived meal has no corresponding manipulation action".into());
        }
        let mut motors = [None; 6];
        motors[2] = Some(1u32 << index);
        let example = ImitationExample {
            candidate_count: u16::try_from(behavior.forced_motor_slots.len())?,
            representative_mask: behavior.representative_mask,
            motor_masks: behavior.motor_masks,
            forced_slots: behavior
                .forced_motor_slots
                .iter()
                .copied()
                .map(Some)
                .collect(),
            target: ImitationTarget {
                representative: None,
                motors,
            },
        };
        example.validate()?;
        example
    };
    // Typed validation above uses the original phenotype and immutable source.
    // Only this in-memory training view is rebound after complete graph equality.
    let mut sequence = replay.sequence;
    sequence.phenotype_hash = current.phenotype_hash();
    sequence.validate_for(current)?;
    Ok(ImitationTrainingWindow {
        speech_targets: vec![None; replay.behavior.len()],
        sequence,
        examples: vec![example],
        episode_weight: prepared.weight,
    })
}

pub(crate) fn rehearse_archived_successes(
    trainer: &mut FoundationTrainer,
    value: &mut PpoTrainingState,
    manifest_path: &Path,
    epochs: u32,
    founder: u64,
    current: &BrainPhenotype,
    current_policy: u64,
) -> Result<ArchivedSuccessReplayReceipt> {
    let bytes = read_archive_bytes(manifest_path)?;
    let manifest: ArchivedSuccessManifest = serde_json::from_slice(&bytes)?;
    validate_archive_request(&manifest, epochs)?;
    if manifest.founder_seed_base != founder {
        return Err("archived manifest belongs to another founder".into());
    }
    let mut prepared = manifest
        .entries
        .iter()
        .map(|entry| prepare(entry, founder, current, current_policy))
        .collect::<Result<Vec<_>>>()?;
    let grabs = manifest
        .entries
        .iter()
        .filter(|entry| entry.action == 211)
        .count();
    let eating = manifest.entries.len() - grabs;
    for (entry, prepared) in manifest.entries.iter().zip(&mut prepared) {
        // Equal aggregate weight for acquisition and eating when both exist.
        prepared.weight = if grabs > 0 && eating > 0 {
            manifest.entries.len() as f32
                / (2 * if entry.action == 211 { grabs } else { eating }) as f32
        } else {
            1.0
        };
        load(entry, prepared, current)?; // reject false labels before any extra actor update.
    }
    let before = trainer.optimizer_step();
    let value_before = serde_json::to_vec(&value.checkpoint(trainer.session())?)?;
    let batch = BATCH_SIZE.min(manifest.entries.len());
    let losses = train_recurrent_imitation(
        trainer,
        value,
        manifest.entries.len(),
        batch,
        epochs,
        1.0,
        1.0,
        |index| {
            load(&manifest.entries[index], &prepared[index], current).map_err(|_| {
                alife_training::TrainingError::from(ScaffoldContractError::InvalidDecisionEvidence)
            })
        },
    )?;
    let expected = epochs
        .checked_mul(u32::try_from(manifest.entries.len().div_ceil(batch))?)
        .ok_or("archive update overflow")?;
    let after = trainer.optimizer_step();
    if after.checked_sub(before) != Some(expected)
        || losses.len() != expected as usize
        || losses.iter().any(|loss| !loss.is_finite())
        || serde_json::to_vec(&value.checkpoint(trainer.session())?)? != value_before
    {
        return Err(
            "archived replay did not preserve value state or complete its reported actor dose"
                .into(),
        );
    }
    Ok(ArchivedSuccessReplayReceipt {
        manifest_digest: Blake3Digest::from_bytes(*blake3::hash(&bytes).as_bytes()),
        requested_epochs: epochs,
        completed_epochs: epochs,
        grab_examples: grabs,
        eating_examples: eating,
        effective_batch_size: batch,
        imitation_temperature: 1.0,
        actor_optimizer_step_before: before,
        actor_optimizer_step_after: after,
        value_checkpoint_unchanged: true,
        original_rewards_preserved: true,
        action_losses: losses,
        examples: manifest.entries,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn selected_source_fixture() -> (
        PathBuf,
        FoundationTrainerCheckpoint,
        alife_training::PpoValueHeadCheckpoint,
    ) {
        let founder = 539363617;
        let provisional = crate::initial_n2048_care_asset(founder).unwrap();
        let mut config = alife_world::CanonicalNewGameConfig::phase3(founder, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.founder_seed_base = founder;
        config.sensor_profile = provisional.manifest().sensor_profile();
        let game =
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &provisional)
                .unwrap();
        let organism = game.world.organism_registry().iter().next().unwrap();
        let genome = &organism.phenotype().brain_genome;
        let capacity = alife_core::BrainCapacityClass::n2048();
        let development = crate::gpu_live_runtime::foundation_construction_development(
            genome,
            &capacity,
            &organism
                .phenotype()
                .development_state_at(alife_core::Tick::ZERO)
                .unwrap(),
        )
        .unwrap();
        // Fixture weights match the canonical founder's expressed graph.
        // This constructs metadata only, without stepping a world or a GPU.
        let native = alife_core::PhenotypeCompiler::compile_testing_procedural_baseline(
            genome,
            &capacity,
            &development,
            config.sensor_profile,
        )
        .unwrap();
        let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&native).unwrap();
        let current = compile_source(asset.clone(), founder, founder).unwrap();
        let n = current.synapses().len();
        let actor = FoundationTrainerCheckpoint {
            schema_version: 2,
            phenotype_hash: current.phenotype_hash(),
            source_foundation_digest: asset.digest(),
            optimizer_step: 1,
            config: alife_training::AdamWConfig::default(),
            stage_mask: alife_training::StageTrainableMask::from_synapse_indices(&current, &[17])
                .unwrap(),
            weights: asset.weights().to_vec(),
            first_moment: vec![0.0; n],
            second_moment: vec![0.0; n],
            update_ages: vec![0; n],
        };
        let value = alife_training::PpoValueHeadCheckpoint {
            feature_count: current.neuron_count(),
            optimizer_step: 1,
            parameters: vec![0.0; (current.neuron_count() as usize + 1) * 3],
            last_updated_policy_version: Some(2),
        };
        // Keep newly created fixtures separate from all genuine source data.
        let directory = std::env::temp_dir().join(format!(
            "alife-inspection-checkpoints-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        let write_json = |name: &str, data: &serde_json::Value| {
            std::fs::write(directory.join(name), serde_json::to_vec(data).unwrap()).unwrap();
        };
        let asset_digest: String = asset
            .digest()
            .bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        write_json(
            "cycle.json",
            &serde_json::json!({
                "biological_objective_version": 2, "seed": founder,
                "founder_seed_base": founder, "policy_version": 2,
                "training_ticks": 1, "collection_seconds": 0.0, "update_seconds": 0.0,
                "old_asset_digest": asset_digest, "new_asset_digest": asset_digest,
                "actor_optimizer_step": 1, "value_optimizer_step": 1, "completed_epochs": 1,
                "next_cohort_tick_captured": true, "next_cohort_optimizer_rebound": true
            }),
        );
        std::fs::write(
            directory.join("trained.alife-foundation"),
            asset.encode_canonical().unwrap(),
        )
        .unwrap();
        write_json(
            "actor-checkpoint.json",
            &serde_json::to_value(&actor).unwrap(),
        );
        write_json(
            "value-checkpoint.json",
            &serde_json::to_value(&value).unwrap(),
        );
        let mut archive = manifest();
        archive.founder_seed_base = founder;
        archive.entries[0].episode = directory.join("missing-archive-entry");
        write_json("manifest.json", &serde_json::to_value(&archive).unwrap());
        (directory, actor, value)
    }

    const CHECKPOINT_FILES: [&str; 4] = [
        "cycle.json",
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ];

    pub(crate) fn checkpoint_fixture(policy: u64) -> PathBuf {
        let (directory, _, _) = selected_source_fixture();
        for (name, field) in [
            ("cycle.json", "policy_version"),
            ("value-checkpoint.json", "last_updated_policy_version"),
        ] {
            let path = directory.join(name);
            let mut data: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            data[field] = serde_json::json!(policy);
            std::fs::write(path, serde_json::to_vec(&data).unwrap()).unwrap();
        }
        directory
    }

    fn checkpoint_bytes(directory: &Path) -> Vec<(String, Vec<u8>)> {
        CHECKPOINT_FILES
            .into_iter()
            .filter_map(|name| {
                std::fs::read(directory.join(name))
                    .ok()
                    .map(|bytes| (name.to_string(), bytes))
            })
            .collect()
    }

    fn checkpoint_error_without_mutation(directory: &Path, expected_policy: u64) -> String {
        let before = checkpoint_bytes(directory);
        let error = inspect_foundation_checkpoint(directory, expected_policy)
            .unwrap_err()
            .to_string();
        assert_eq!(checkpoint_bytes(directory), before);
        error
    }

    #[test]
    fn checkpoint_only_accepts_expected_policy_without_archive_or_mutation() {
        let directory = checkpoint_fixture(3238);
        // Even an unusable archive manifest must not become a prerequisite.
        std::fs::write(directory.join("manifest.json"), b"invalid archive JSON").unwrap();
        let before = checkpoint_bytes(&directory);
        let receipt = inspect_foundation_checkpoint(&directory, 3238).unwrap();
        assert_eq!(checkpoint_bytes(&directory), before);
        assert_eq!(
            std::fs::read(directory.join("manifest.json")).unwrap(),
            b"invalid archive JSON"
        );
        assert_eq!(receipt.policy_version, 3238);
        assert_eq!(receipt.expected_policy_version, 3238);
        assert_eq!(receipt.actor_optimizer_step, 1);
        assert_eq!(receipt.value_optimizer_step, 1);
        assert!(!receipt.neural_execution && !receipt.weights_modified && !receipt.archive_checked);
        assert_eq!(receipt.file_digests.len(), 4);
        for ((name, bytes), actual) in before.iter().zip(&receipt.file_digests) {
            assert_eq!(actual.file_name, name);
            assert_eq!(actual.encoded_bytes, bytes.len());
            assert_eq!(actual.digest.bytes(), blake3::hash(bytes).as_bytes());
            assert_eq!(actual.sha256, format!("{:x}", Sha256::digest(bytes)));
        }
    }

    #[test]
    fn checkpoint_only_preserves_optional_diagnostic_binding_without_requiring_sidecar() {
        let directory = checkpoint_fixture(3238);
        let path = directory.join("cycle.json");
        let mut cycle: FoundationCycleReceipt =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert!(cycle.ppo_rollout_diagnostics_sha256.is_none());
        assert!(serde_json::to_value(&cycle)
            .unwrap()
            .get("ppo_rollout_diagnostics_sha256")
            .is_none());
        let digest = "01".repeat(32);
        cycle.ppo_rollout_diagnostics_sha256 = Some(digest.clone());
        let encoded = serde_json::to_vec(&cycle).unwrap();
        let decoded: FoundationCycleReceipt = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.ppo_rollout_diagnostics_sha256, Some(digest));
        std::fs::write(path, encoded).unwrap();
        // Derived diagnostics must not become a prerequisite for resuming learning.
        assert!(!directory.join("ppo-rollout-diagnostics.json").exists());
        assert!(inspect_foundation_checkpoint(&directory, 3238).is_ok());
    }

    #[test]
    fn checkpoint_only_rejects_wrong_policy_and_all_route_marker_forms() {
        let directory = checkpoint_fixture(3237);
        assert_eq!(
            checkpoint_error_without_mutation(&directory, 3238),
            "checkpoint policy mismatch: expected 3238, found 3237"
        );
        let overflow = checkpoint_fixture(u64::MAX);
        assert_eq!(
            checkpoint_error_without_mutation(&overflow, u64::MAX),
            "policy version overflow"
        );
        for marker in ["adaptation.json", "warmup.json"] {
            let path = directory.join(marker);
            std::fs::write(&path, b"marker").unwrap();
            assert!(checkpoint_error_without_mutation(&directory, 3237).contains(marker));
            assert_eq!(std::fs::read(&path).unwrap(), b"marker");
            std::fs::remove_file(&path).unwrap();
            std::fs::create_dir(&path).unwrap();
            assert!(checkpoint_error_without_mutation(&directory, 3237).contains(marker));
            assert!(path.is_dir());
            std::fs::remove_dir(&path).unwrap();
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink("intentionally-absent-marker-target", &path).unwrap();
                assert!(checkpoint_error_without_mutation(&directory, 3237).contains(marker));
                assert!(std::fs::symlink_metadata(&path)
                    .unwrap()
                    .file_type()
                    .is_symlink());
                std::fs::remove_file(&path).unwrap();
            }
        }
    }

    #[test]
    fn checkpoint_only_rejects_corruption_and_incomplete_handoffs_without_mutation() {
        let directory = checkpoint_fixture(3238);
        let originals = checkpoint_bytes(&directory);
        for (file, field) in [
            ("actor-checkpoint.json", "first_moment"),
            ("actor-checkpoint.json", "phenotype_hash"),
            ("actor-checkpoint.json", "weights"),
            ("value-checkpoint.json", "parameters"),
            ("value-checkpoint.json", "last_updated_policy_version"),
            ("cycle.json", "next_cohort_tick_captured"),
            ("cycle.json", "next_cohort_optimizer_rebound"),
            ("cycle.json", "biological_objective_version"),
            ("cycle.json", "objective_state_reset"),
            ("cycle.json", "founder_seed_base"),
            ("cycle.json", "new_asset_digest"),
        ] {
            let path = directory.join(file);
            let original = &originals.iter().find(|(name, _)| name == file).unwrap().1;
            let mut data: serde_json::Value = serde_json::from_slice(original).unwrap();
            match field {
                "first_moment" => {
                    data[field].as_array_mut().unwrap().pop();
                }
                "phenotype_hash" => {
                    data[field][0] = serde_json::json!(data[field][0].as_u64().unwrap() ^ 1)
                }
                "weights" => {
                    data[field][0] = serde_json::json!(data[field][0].as_f64().unwrap() + 0.5)
                }
                "parameters" => data[field] = serde_json::json!([]),
                "last_updated_policy_version" => data[field] = serde_json::json!(3237),
                "next_cohort_tick_captured" | "next_cohort_optimizer_rebound" => {
                    data[field] = serde_json::json!(false)
                }
                "biological_objective_version" => data[field] = serde_json::json!(1),
                "objective_state_reset" => data[field] = serde_json::json!(true),
                "founder_seed_base" => data[field] = serde_json::json!(0),
                "new_asset_digest" => data[field] = serde_json::json!("wrong"),
                _ => unreachable!(),
            }
            std::fs::write(&path, serde_json::to_vec(&data).unwrap()).unwrap();
            assert!(
                !checkpoint_error_without_mutation(&directory, 3238).is_empty(),
                "{field}"
            );
            std::fs::write(path, original).unwrap();
        }
    }

    #[test]
    fn checkpoint_only_rejects_truncated_swapped_and_stale_optimizer_artifacts() {
        let directory = checkpoint_fixture(3238);
        let originals = checkpoint_bytes(&directory);
        for (name, bytes) in &originals {
            for end in [0, bytes.len() / 2] {
                std::fs::write(directory.join(name), &bytes[..end]).unwrap();
                assert!(!checkpoint_error_without_mutation(&directory, 3238).is_empty());
                std::fs::write(directory.join(name), bytes).unwrap();
            }
        }
        for (left, right) in [
            ("actor-checkpoint.json", "value-checkpoint.json"),
            ("trained.alife-foundation", "actor-checkpoint.json"),
        ] {
            let left_bytes = &originals.iter().find(|(name, _)| name == left).unwrap().1;
            let right_bytes = &originals.iter().find(|(name, _)| name == right).unwrap().1;
            std::fs::write(directory.join(left), right_bytes).unwrap();
            std::fs::write(directory.join(right), left_bytes).unwrap();
            assert!(!checkpoint_error_without_mutation(&directory, 3238).is_empty());
            std::fs::write(directory.join(left), left_bytes).unwrap();
            std::fs::write(directory.join(right), right_bytes).unwrap();
        }
        for name in ["actor-checkpoint.json", "value-checkpoint.json"] {
            let bytes = &originals.iter().find(|(file, _)| file == name).unwrap().1;
            let mut stale: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            stale["optimizer_step"] = serde_json::json!(0);
            std::fs::write(directory.join(name), serde_json::to_vec(&stale).unwrap()).unwrap();
            assert!(checkpoint_error_without_mutation(&directory, 3238)
                .contains("does not match exported actor"));
            std::fs::write(directory.join(name), bytes).unwrap();
        }
        assert!(inspect_foundation_checkpoint(&directory, 3238).is_ok());
        assert_eq!(checkpoint_bytes(&directory), originals);
    }

    #[test]
    fn checkpoint_only_reports_each_missing_file_without_rewriting_remaining_inputs() {
        let directory = checkpoint_fixture(3238);
        for file in CHECKPOINT_FILES {
            let path = directory.join(file);
            let original = std::fs::read(&path).unwrap();
            std::fs::remove_file(&path).unwrap();
            assert!(checkpoint_error_without_mutation(&directory, 3238).contains(file));
            assert!(!path.exists());
            std::fs::write(path, original).unwrap();
        }
    }

    #[test]
    fn selected_checkpoint_payloads_are_checked_before_loading_any_archive_entry() {
        let (directory, actor, value) = selected_source_fixture();
        let write_json = |name: &str, data: &serde_json::Value| {
            std::fs::write(directory.join(name), serde_json::to_vec(data).unwrap()).unwrap();
        };
        let inspect = || inspect_archived_successes(&directory, &directory.join("manifest.json"));
        // A valid selected checkpoint reaches the first (deliberately absent)
        // archive entry. A malformed selected checkpoint must fail earlier.
        let error = inspect().unwrap_err();
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::NotFound
        );
        let mut malformed_actor = actor.clone();
        malformed_actor.first_moment.clear();
        write_json(
            "actor-checkpoint.json",
            &serde_json::to_value(&malformed_actor).unwrap(),
        );
        assert!(inspect()
            .unwrap_err()
            .downcast_ref::<ScaffoldContractError>()
            .is_some());
        write_json(
            "actor-checkpoint.json",
            &serde_json::to_value(&actor).unwrap(),
        );
        let mut malformed_value = value.clone();
        malformed_value.parameters.clear();
        write_json(
            "value-checkpoint.json",
            &serde_json::to_value(&malformed_value).unwrap(),
        );
        assert!(inspect()
            .unwrap_err()
            .downcast_ref::<alife_training::TrainingError>()
            .is_some());
        let mut incompatible_value = value;
        incompatible_value.feature_count += 1;
        incompatible_value.parameters =
            vec![0.0; (incompatible_value.feature_count as usize + 1) * 3];
        write_json(
            "value-checkpoint.json",
            &serde_json::to_value(&incompatible_value).unwrap(),
        );
        assert_eq!(
            inspect().unwrap_err().to_string(),
            "inspection source value head is bound to a different feature count"
        );
    }

    #[test]
    fn inspection_binds_declared_residual_sidecar_before_archive_loading() {
        let (directory, _, _) = selected_source_fixture();
        let manifest_path = directory.join("manifest.json");
        let mut archive: ArchivedSuccessManifest =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        // A known archive-ordering sentinel distinguishes source admission from
        // archive loading, without constructing a GPU session or replay evidence.
        archive.entries[0].episode = directory.clone();
        archive.entries[0].previous = directory.clone();
        let manifest_bytes = serde_json::to_vec(&archive).unwrap();
        std::fs::write(&manifest_path, &manifest_bytes).unwrap();
        let inspect = || inspect_archived_successes(&directory, &manifest_path);
        let assert_source_admitted = || {
            let before = checkpoint_bytes(&directory);
            assert_eq!(
                inspect().unwrap_err().to_string(),
                "archived episode is not a sealed older continuation of this founder"
            );
            assert_eq!(checkpoint_bytes(&directory), before);
        };
        // Historical cycles remain admissible without a residual sidecar.
        assert_source_admitted();
        let sidecar = directory.join(crate::foundation_critic_correction::RESIDUAL_UPDATE);
        let bytes = b"{\"optimizer_updates\":2}";
        std::fs::write(&sidecar, bytes).unwrap();
        let mut cycle: FoundationCycleReceipt =
            serde_json::from_slice(&std::fs::read(directory.join("cycle.json")).unwrap()).unwrap();
        cycle.critic_residual_update_sha256 = Some(format!("{:x}", Sha256::digest(bytes)));
        std::fs::write(
            directory.join("cycle.json"),
            serde_json::to_vec(&cycle).unwrap(),
        )
        .unwrap();
        assert_source_admitted();
        let checkpoint_before = checkpoint_bytes(&directory);
        std::fs::write(&sidecar, b"tampered residual evidence").unwrap();
        assert_eq!(
            inspect().unwrap_err().to_string(),
            "critic residual update SHA256 mismatch"
        );
        assert_eq!(
            std::fs::read(&sidecar).unwrap(),
            b"tampered residual evidence"
        );
        std::fs::remove_file(&sidecar).unwrap();
        assert_eq!(
            inspect()
                .unwrap_err()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .kind(),
            std::io::ErrorKind::NotFound
        );
        assert!(!sidecar.exists());
        assert_eq!(checkpoint_bytes(&directory), checkpoint_before);
        assert_eq!(std::fs::read(&manifest_path).unwrap(), manifest_bytes);
        std::fs::write(&sidecar, bytes).unwrap();
        assert_source_admitted();
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn inspection_orders_archive_episodes_against_the_next_continuation() {
        let (directory, _, _) = selected_source_fixture();
        let source_cycle: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("cycle.json")).unwrap()).unwrap();
        let mut archive: ArchivedSuccessManifest =
            serde_json::from_slice(&std::fs::read(directory.join("manifest.json")).unwrap())
                .unwrap();
        let episode = directory.join("archive-episode");
        let parent = directory.join("archive-parent");
        std::fs::create_dir(&episode).unwrap();
        std::fs::create_dir(&parent).unwrap();
        archive.entries[0].episode = episode.clone();
        archive.entries[0].previous = parent.clone();
        let write_episode = |archive: &mut ArchivedSuccessManifest, policy: u64| {
            let mut cycle = source_cycle.clone();
            cycle["policy_version"] = serde_json::json!(policy);
            std::fs::write(
                episode.join("cycle.json"),
                serde_json::to_vec(&cycle).unwrap(),
            )
            .unwrap();
            cycle["policy_version"] = serde_json::json!(policy - 1);
            std::fs::write(
                parent.join("cycle.json"),
                serde_json::to_vec(&cycle).unwrap(),
            )
            .unwrap();
            archive.entries[0].source.policy_version = policy;
            std::fs::write(
                directory.join("manifest.json"),
                serde_json::to_vec(archive).unwrap(),
            )
            .unwrap();
        };
        // The selected source is P=2. Historical E=1 and source episode E=2
        // pass ordering for continuation 3 and reach the absent asset sentinel.
        // This is boundary admission only, not a genuine replay/label fixture.
        for policy in [1, 2] {
            write_episode(&mut archive, policy);
            let error = inspect_archived_successes(&directory, &directory.join("manifest.json"))
                .unwrap_err();
            assert_eq!(
                error.downcast_ref::<std::io::Error>().unwrap().kind(),
                std::io::ErrorKind::NotFound,
                "episode {policy} must pass policy ordering"
            );
        }
        write_episode(&mut archive, 3);
        assert_eq!(
            inspect_archived_successes(&directory, &directory.join("manifest.json"))
                .unwrap_err()
                .to_string(),
            "archived episode is not a sealed older continuation of this founder"
        );
    }

    #[test]
    fn inspection_rejects_source_policy_overflow_before_archive_loading() {
        let (directory, _, mut value) = selected_source_fixture();
        let mut cycle: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("cycle.json")).unwrap()).unwrap();
        cycle["policy_version"] = serde_json::json!(u64::MAX);
        value.last_updated_policy_version = Some(u64::MAX);
        std::fs::write(
            directory.join("cycle.json"),
            serde_json::to_vec(&cycle).unwrap(),
        )
        .unwrap();
        std::fs::write(
            directory.join("value-checkpoint.json"),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        assert_eq!(
            inspect_archived_successes(&directory, &directory.join("manifest.json"))
                .unwrap_err()
                .to_string(),
            "policy version overflow"
        );
    }

    fn manifest() -> ArchivedSuccessManifest {
        ArchivedSuccessManifest {
            schema: 1,
            founder_seed_base: 1,
            entries: vec![ArchivedSuccessEntry {
                episode: "old".into(),
                previous: "parent".into(),
                row: 0,
                action: 211,
                source: FoundationReplaySource {
                    source_revision: "old".into(),
                    policy_version: 1,
                    actor_checkpoint_digest: Blake3Digest::default(),
                    foundation_asset_digest: Blake3Digest::default(),
                    phenotype_hash: alife_core::PhenotypeHash([0; 4]),
                    compiler_inputs_digest: [0; 4],
                },
                record_digest: [1; 32],
            }],
        }
    }
    #[test]
    fn bounded_archive_rejects_empty_duplicate_foreign_actions_and_bad_epochs() {
        let mut value = manifest();
        assert!(validate_archive_request(&value, 1).is_ok());
        assert!(validate_archive_request(&value, 0).is_err());
        assert!(validate_archive_request(&value, 129).is_err());
        value.entries[0].action = 101;
        assert!(validate_archive_request(&value, 1).is_err());
        value.entries[0].action = 211;
        value
            .entries
            .push(serde_json::from_slice(&serde_json::to_vec(&value.entries[0]).unwrap()).unwrap());
        assert!(validate_archive_request(&value, 1).is_err());
        value.entries.clear();
        assert!(validate_archive_request(&value, 1).is_err());
    }

    #[test]
    fn graph_comparison_allows_new_weights_but_preserves_every_other_contract() {
        let founder = 539363617;
        let asset = crate::initial_n2048_care_asset(founder).unwrap();
        let genome =
            alife_core::BrainGenome::scaffold(founder, alife_core::BrainCapacityClass::N2048_ID);
        let development = alife_core::DevelopmentState::new(
            genome.id,
            alife_core::Tick::ZERO,
            alife_core::NormalizedScalar::new(1.0).unwrap(),
        );
        let old = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome.clone(),
            development.clone(),
            asset.clone(),
        )
        .unwrap()
        .0;
        let mut weights = asset.weights().to_vec();
        weights[1024] += 0.001;
        let next_asset = FoundationWeightAsset::from_trained_weights(
            &old,
            weights,
            alife_core::TrainingStageManifest::new(1, 1, 1),
        )
        .unwrap();
        let next = alife_core::PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            next_asset,
        )
        .unwrap()
        .0;
        assert_ne!(old.phenotype_hash(), next.phenotype_hash());
        let before = graph_without_weights(&old).unwrap();
        let mut after = graph_without_weights(&next).unwrap();
        let changed = before
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| before[*key] != after[*key])
            .collect::<Vec<_>>();
        assert!(
            before == after,
            "weight-only update changed graph fields: {changed:?}"
        );
        after["microstep_count"] = serde_json::json!(999);
        assert_ne!(before, after);
    }
}
