//! A bounded, real-world actor update and next-cohort admission gate.
//! The ordinary GPU runtime owns perception, action, biology, and legality.

use std::{path::Path, time::Instant};

use alife_core::{
    BrainScaleTier, CompiledSynapseKind, DecoderHeadKind, FoundationWeightAsset, SensorProfile,
    TrainingStageManifest,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile, GpuTrainingSamplingConfig};
use alife_training::{
    train_recurrent_ppo, AdamWConfig, FoundationTrainer, PpoBatch, PpoBoundary, PpoConfig,
    PpoJointAction, PpoTrainingState, PpoTransition, StageTrainableMask,
};

use crate::{
    initial_n2048_care_asset, load_foundation_replay_window, FoundationReplayBudget,
    FoundationReplaySource, FoundationReplayWriter, GpuLiveBrainRuntime,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundationCycleReceipt {
    pub seed: u64,
    pub policy_version: u64,
    pub training_ticks: usize,
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
    run_foundation_training_cycle_from(output, seed, training_ticks, None)
}

/// Continue from an exactly rebound, sealed previous cycle. The saved optimizer
/// and value head are restored before using any newly collected decision.
pub fn resume_foundation_training_cycle(
    previous: &Path,
    output: &Path,
    seed: u64,
    training_ticks: usize,
) -> Result<FoundationCycleReceipt> {
    run_foundation_training_cycle_from(output, seed, training_ticks, Some(previous))
}

fn run_foundation_training_cycle_from(
    output: &Path,
    seed: u64,
    training_ticks: usize,
    previous: Option<&Path>,
) -> Result<FoundationCycleReceipt> {
    if seed == 0 || !(1..=511).contains(&training_ticks) {
        return Err("cycle needs a nonzero seed and 1..=511 training ticks".into());
    }
    std::fs::create_dir(output)?;
    let (asset, policy_version, restored_actor, restored_value) = if let Some(previous) = previous {
        let receipt: FoundationCycleReceipt =
            serde_json::from_slice(&std::fs::read(previous.join("cycle.json"))?)?;
        if receipt.seed != seed || !receipt.next_cohort_optimizer_rebound {
            return Err("previous cycle is not an exact sealed cohort handoff".into());
        }
        let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
            previous.join("trained.alife-foundation"),
        )?)?;
        if digest(&asset) != receipt.new_asset_digest {
            return Err("previous exported asset does not match its receipt".into());
        }
        let actor: alife_training::FoundationTrainerCheckpoint =
            serde_json::from_slice(&std::fs::read(previous.join("actor-checkpoint.json"))?)?;
        let value: alife_training::PpoValueHeadCheckpoint =
            serde_json::from_slice(&std::fs::read(previous.join("value-checkpoint.json"))?)?;
        if actor.source_foundation_digest != asset.digest()
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
        (
            asset,
            receipt
                .policy_version
                .checked_add(1)
                .ok_or("policy version overflow")?,
            Some(actor),
            Some(value),
        )
    } else {
        (initial_n2048_care_asset(seed)?, 0, None, None)
    };
    std::fs::write(
        output.join("initial.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    std::fs::write(output.join("phase.txt"), "source-written")?;

    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let mut game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    game.world.set_age_death_disabled_for_new_game(true)?;
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let mut runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        backend,
        game.world,
        seed,
        BrainScaleTier::Standard2048,
        SensorProfile::GroundedObjectSlotsV1,
        alife_archive::LineageLibraryConfig::profile_default(output.join("lineage")),
        format!("n2048-cycle-{seed}"),
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        GpuTrainingSamplingConfig {
            seed: (seed as u32) ^ (policy_version as u32).wrapping_mul(0x9e37_79b9),
            counter: 0,
            temperature: 1.0,
            demonstrator: None,
        },
    )?;
    std::fs::write(output.join("phase.txt"), "runtime-admitted")?;

    let started = Instant::now();
    runtime
        .tick()
        .map_err(|e| format!("cycle production tick 0: {e}"))?;
    let mut first = runtime.take_foundation_training_steps();
    if first.len() != 1 {
        return Err(format!(
            "cycle tick 0: expected one waking decision, got {}",
            first.len()
        )
        .into());
    }
    std::fs::write(output.join("phase.txt"), "first-capture")?;
    let phenotype = first[0].before.phenotype.clone();
    let trainable: Vec<u32> = phenotype.synapses().iter().enumerate().filter_map(|(i, synapse)| {
        (!matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload))
            .then_some(i as u32)
    }).collect();
    let mask = StageTrainableMask::from_synapse_indices(&phenotype, &trainable)?;
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
    }
    let initial_checkpoint = serde_json::to_vec(&trainer.checkpoint()?)?;
    let source_revision = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()?;
    if !source_revision.status.success() {
        return Err("could not identify training source revision".into());
    }
    let source = FoundationReplaySource {
        source_revision: String::from_utf8(source_revision.stdout)?.trim().to_owned(),
        policy_version,
        actor_checkpoint_digest: alife_core::Blake3Digest::from_bytes(
            *blake3::hash(&initial_checkpoint).as_bytes(),
        ),
        foundation_asset_digest: asset.digest(),
        phenotype_hash: phenotype.phenotype_hash(),
        compiler_inputs_digest: phenotype.compiler_inputs_digest(),
    };
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
    references.push(writer.append(&first.remove(0))?);
    for tick in 1..=training_ticks {
        budget.check_additional(64 * 1024 * 1024)?;
        runtime
            .tick()
            .map_err(|e| format!("cycle production tick {tick}: {e}"))?;
        let mut captured = runtime.take_foundation_training_steps();
        if captured.len() != 1 {
            return Err(format!(
                "cycle tick {tick}: expected one waking decision, got {}",
                captured.len()
            )
            .into());
        }
        references.push(writer.append(&captured.remove(0))?);
    }
    let collection_seconds = started.elapsed().as_secs_f64();
    std::fs::write(output.join("phase.txt"), "collection-complete")?;
    std::fs::write(
        output.join("replay-manifest.json"),
        serde_json::to_vec(&references)?,
    )?;
    let window =
        load_foundation_replay_window(&replay_dir, &references, 0, &source, &phenotype, budget)?;
    let mut sequence = window.sequence;
    let mut value = if let Some(checkpoint) = restored_value {
        PpoTrainingState::from_checkpoint(checkpoint)?
    } else {
        PpoTrainingState::default()
    };
    let values = value.predict_values(&mut trainer, &sequence)?;
    if values.len() != references.len() {
        return Err("GPU value prediction count does not match decisions".into());
    }
    let mut transitions = Vec::with_capacity(training_ticks);
    for tick in 0..training_ticks {
        let behavior = &window.behavior[tick];
        let modulator =
            alife_core::OutcomeCreditPacket::from_sealed_patch(&window.patches[tick])?.modulator();
        transitions.push(PpoTransition {
            policy_version,
            trajectory_id: seed,
            step: tick as u64,
            action: joint_action(behavior)?,
            reward: (modulator.homeostatic_improvement() - modulator.pain()).clamp(-1.0, 1.0),
            old_value: values[tick],
            next_value: values[tick + 1],
            elapsed_seconds: 1.0 / 20.0,
            boundary: if tick + 1 == training_ticks {
                PpoBoundary::Truncated
            } else {
                PpoBoundary::Continuing
            },
        });
    }
    sequence.ticks.pop(); // Bootstrap decision is context only, never an update row.
    let ppo_config = PpoConfig::default();
    let batch = PpoBatch::from_rollout(policy_version, transitions, ppo_config)?;
    let started = Instant::now();
    let update = train_recurrent_ppo(
        &mut trainer,
        &mut value,
        &sequence,
        &batch,
        ppo_config,
        policy_version,
    )?;
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
    let bytes = std::fs::read(output.join("trained.alife-foundation"))?;
    let admitted_asset = FoundationWeightAsset::decode_canonical(&bytes)?;
    if admitted_asset.digest() != trained.digest() {
        return Err("exported asset digest changed on disk".into());
    }
    let mut next_game =
        alife_world::create_canonical_new_game_with_n2048_candidate(&next_config, &admitted_asset)?;
    next_game.world.set_age_death_disabled_for_new_game(true)?;
    let next_backend = runtime.new_staging_like_live()?;
    let mut next_runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        next_backend,
        next_game.world,
        seed,
        BrainScaleTier::Standard2048,
        SensorProfile::GroundedObjectSlotsV1,
        alife_archive::LineageLibraryConfig::profile_default(output.join("next-lineage")),
        format!("n2048-cycle-next-{}", seed + 1),
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        GpuTrainingSamplingConfig {
            seed: (seed as u32)
                ^ ((policy_version as u32).wrapping_add(1)).wrapping_mul(0x9e37_79b9),
            counter: 0,
            temperature: 1.0,
            demonstrator: None,
        },
    )?;
    next_runtime.tick()?;
    let next_steps = next_runtime.take_foundation_training_steps();
    let next_cohort_tick_captured = next_steps.len() == 1;
    if !next_cohort_tick_captured {
        return Err("trained asset next cohort did not produce a decision".into());
    }
    trainer.rebind_for_next_cohort(next_steps[0].before.phenotype.clone(), admitted_asset)?;
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
        seed,
        policy_version,
        training_ticks,
        collection_seconds,
        update_seconds,
        old_asset_digest: digest(&asset),
        new_asset_digest: digest(&trained),
        actor_optimizer_step: update.actor_optimizer_step,
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
