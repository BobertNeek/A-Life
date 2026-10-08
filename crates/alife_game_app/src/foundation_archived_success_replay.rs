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

fn compile_source(asset: FoundationWeightAsset, seed: u64, founder: u64) -> Result<BrainPhenotype> {
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
mod tests {
    use super::*;
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
