//! Source-bound demonstration warm-up on the ordinary recurrent GPU trainer.

use std::{collections::HashSet, path::Path};

use alife_core::{
    BrainScaleTier, CompiledSynapseKind, DecoderHeadKind, FoundationWeightAsset, SensorProfile,
    TrainingStageManifest,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile, GpuTrainingSamplingConfig};
use alife_training::{
    train_recurrent_imitation, AdamWConfig, FoundationTrainer, ImitationExample, ImitationTarget,
    ImitationTrainingWindow, PpoTrainingState, StageTrainableMask,
};

use crate::{
    initial_n2048_care_asset, load_foundation_replay_window, FoundationPilotReceipt,
    FoundationReplayBudget, FoundationReplayRecordRef, FoundationReplaySource,
    FoundationTeacherLesson, GpuLiveBrainRuntime,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, serde::Deserialize)]
struct DemonstrationManifest {
    founder_seed_base: u64,
    pilots: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundationWarmupReceipt {
    pub founder_seed_base: u64,
    pub demonstration_count: usize,
    pub demonstration_records: usize,
    pub category_counts: [usize; 4],
    pub source_asset_digest: String,
    pub trained_asset_digest: String,
    pub actor_optimizer_step: u32,
    #[serde(default)]
    pub epochs: u32,
    pub losses: Vec<f32>,
    pub next_cohort_optimizer_rebound: bool,
}

fn asset_digest(asset: &FoundationWeightAsset) -> String {
    asset
        .digest()
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn lesson_index(lesson: FoundationTeacherLesson) -> usize {
    match lesson {
        FoundationTeacherLesson::Feeding => 0,
        FoundationTeacherLesson::HazardAvoidance => 1,
        FoundationTeacherLesson::ObstacleNavigation => 2,
        FoundationTeacherLesson::Recovery => 3,
    }
}

fn imitation_example(
    behavior: &alife_gpu_backend::GpuTrainingRolloutReceipt,
) -> Result<ImitationExample> {
    let representative = behavior.representative_index;
    let forced = behavior.forced_motor_slots[usize::from(representative)];
    let mut motors = [None; 6];
    for (slot, target) in motors.iter_mut().enumerate() {
        let index = behavior.motor_indices[slot];
        if index != u16::MAX && forced != slot as u8 {
            *target = Some(1u32 << index);
        }
    }
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
            representative: Some(1u32 << representative),
            motors,
        },
    };
    example.validate()?;
    Ok(example)
}

/// Read exactly eight measured lessons of each type, then update the same
/// foundation graph/optimizer later used by the production PPO cycle.
pub fn run_foundation_imitation_warmup(
    output: &Path,
    manifest_path: &Path,
    epochs: u32,
) -> Result<FoundationWarmupReceipt> {
    let manifest: DemonstrationManifest = serde_json::from_slice(&std::fs::read(manifest_path)?)?;
    if manifest.founder_seed_base == 0 || manifest.pilots.len() != 32 || !(1..=50).contains(&epochs)
    {
        return Err("warm-up requires 32 lessons and one nonzero founder seed".into());
    }
    std::fs::create_dir(output)?;
    let source_asset = initial_n2048_care_asset(manifest.founder_seed_base)?;
    let mut config = alife_world::CanonicalNewGameConfig::phase3(manifest.founder_seed_base, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = manifest.founder_seed_base;
    let mut game =
        alife_world::create_canonical_new_game_with_n2048_candidate(&config, &source_asset)?;
    game.world.set_age_death_disabled_for_new_game(true)?;
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let mut runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        backend,
        game.world,
        manifest.founder_seed_base,
        BrainScaleTier::Standard2048,
        SensorProfile::GroundedObjectSlotsV1,
        alife_archive::LineageLibraryConfig::profile_default(output.join("lineage")),
        "n2048-imitation-warmup",
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        GpuTrainingSamplingConfig {
            seed: manifest.founder_seed_base as u32,
            counter: 0,
            temperature: 1.0,
            demonstrator: None,
        },
    )?;
    runtime.tick()?;
    let steps = runtime.take_foundation_training_steps();
    let phenotype = steps
        .first()
        .ok_or("warm-up source cohort did not capture a decision")?
        .before
        .phenotype
        .clone();
    let ids = phenotype
        .synapses()
        .iter()
        .enumerate()
        .filter_map(|(index, synapse)| {
            (!matches!(synapse.kind(), CompiledSynapseKind::Decoder(decoder) if decoder.head() == DecoderHeadKind::SpeechPayload))
                .then_some(index as u32)
        })
        .collect::<Vec<_>>();
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &ids)?;
    let staging = runtime.new_staging_like_live()?;
    let mut trainer = FoundationTrainer::from_session(
        alife_runtime::GpuAuthoritativeSession::new(
            staging,
            alife_runtime::GpuSessionConsumerKind::Training,
        ),
        phenotype.clone(),
        source_asset.clone(),
        mask,
        AdamWConfig::default(),
    )?;
    let initial_checkpoint = serde_json::to_vec(&trainer.checkpoint()?)?;
    let checkpoint_digest =
        alife_core::Blake3Digest::from_bytes(*blake3::hash(&initial_checkpoint).as_bytes());
    let root = manifest_path
        .parent()
        .ok_or("manifest has no parent directory")?;
    let mut category_counts = [0usize; 4];
    let mut seen_seeds = HashSet::new();
    let mut demos = Vec::with_capacity(32);
    let mut record_count = 0;
    let mut expected_source: Option<FoundationReplaySource> = None;
    for entry in &manifest.pilots {
        let directory = root.join(entry);
        let receipt: FoundationPilotReceipt =
            serde_json::from_slice(&std::fs::read(directory.join("pilot.json"))?)?;
        let lesson = receipt.lesson.ok_or("warm-up pilot has no lesson")?;
        if !receipt.teacher_mode
            || receipt.lesson_completed != Some(true)
            || receipt.founder_seed_base != manifest.founder_seed_base
            || receipt.source_asset_digest != asset_digest(&source_asset)
            || receipt.ticks == 0
            || receipt.demonstration_replay_records != receipt.ticks
            || !seen_seeds.insert(receipt.seed)
        {
            return Err("warm-up pilot identity or measured outcome mismatch".into());
        }
        category_counts[lesson_index(lesson)] += 1;
        let source: FoundationReplaySource =
            serde_json::from_slice(&std::fs::read(directory.join("replay-source.json"))?)?;
        if source.policy_version != 0
            || source.actor_checkpoint_digest != checkpoint_digest
            || source.foundation_asset_digest != source_asset.digest()
            || source.phenotype_hash != phenotype.phenotype_hash()
            || source.compiler_inputs_digest != phenotype.compiler_inputs_digest()
            || expected_source
                .as_ref()
                .is_some_and(|expected| expected != &source)
            || std::fs::read(directory.join("actor-checkpoint.json"))? != initial_checkpoint
        {
            return Err("warm-up pilot is not bound to the same frozen actor".into());
        }
        expected_source.get_or_insert(source);
        let references: Vec<FoundationReplayRecordRef> =
            serde_json::from_slice(&std::fs::read(directory.join("replay-manifest.json"))?)?;
        if references.len() != receipt.ticks {
            return Err("warm-up pilot replay length mismatch".into());
        }
        record_count += references.len();
        demos.push((directory.join("replay"), references));
    }
    if category_counts != [8; 4] {
        return Err("warm-up lessons are not balanced eight per category".into());
    }
    let expected_source = expected_source.ok_or("warm-up has no replay source")?;
    let budget = FoundationReplayBudget::default();
    let mut state = PpoTrainingState::default();
    let losses = train_recurrent_imitation(
        &mut trainer,
        &mut state,
        demos.len(),
        8,
        epochs,
        1.0,
        1.0,
        |index| {
            let (directory, references) = &demos[index];
            let replay = load_foundation_replay_window(
                directory,
                references,
                0,
                &expected_source,
                &phenotype,
                budget,
            )
            .map_err(|_| {
                alife_training::TrainingError::from(
                    alife_core::ScaffoldContractError::InvalidDecisionEvidence,
                )
            })?;
            let examples = replay
                .behavior
                .iter()
                .map(imitation_example)
                .collect::<Result<Vec<_>>>()
                .map_err(|_| {
                    alife_training::TrainingError::from(
                        alife_core::ScaffoldContractError::InvalidDecisionEvidence,
                    )
                })?;
            Ok(ImitationTrainingWindow {
                sequence: replay.sequence,
                examples,
            })
        },
    )?;
    let value = state.checkpoint(trainer.session())?;
    let trained = trainer.export_candidate(TrainingStageManifest::new(1, 1, 1))?;
    std::fs::write(
        output.join("trained.alife-foundation"),
        trained.encode_canonical()?,
    )?;
    let mut next_game =
        alife_world::create_canonical_new_game_with_n2048_candidate(&config, &trained)?;
    next_game.world.set_age_death_disabled_for_new_game(true)?;
    let mut next_runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        runtime.new_staging_like_live()?,
        next_game.world,
        manifest.founder_seed_base,
        BrainScaleTier::Standard2048,
        SensorProfile::GroundedObjectSlotsV1,
        alife_archive::LineageLibraryConfig::profile_default(output.join("next-lineage")),
        "n2048-imitation-warmup-next",
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        GpuTrainingSamplingConfig {
            seed: manifest.founder_seed_base as u32 + 1,
            counter: 0,
            temperature: 1.0,
            demonstrator: None,
        },
    )?;
    next_runtime.tick()?;
    let next_steps = next_runtime.take_foundation_training_steps();
    let next_phenotype = next_steps
        .first()
        .ok_or("warm-up asset did not admit to the next cohort")?
        .before
        .phenotype
        .clone();
    trainer.rebind_for_next_cohort(next_phenotype, trained.clone())?;
    let actor = trainer.checkpoint()?;
    std::fs::write(
        output.join("actor-checkpoint.json"),
        serde_json::to_vec(&actor)?,
    )?;
    std::fs::write(
        output.join("value-checkpoint.json"),
        serde_json::to_vec(&value)?,
    )?;
    let receipt = FoundationWarmupReceipt {
        founder_seed_base: manifest.founder_seed_base,
        demonstration_count: demos.len(),
        demonstration_records: record_count,
        category_counts,
        source_asset_digest: asset_digest(&source_asset),
        trained_asset_digest: asset_digest(&trained),
        actor_optimizer_step: actor.optimizer_step,
        epochs,
        losses,
        next_cohort_optimizer_rebound: true,
    };
    std::fs::write(
        output.join("warmup.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
}
