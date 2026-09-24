//! A bounded, real-world actor update and next-cohort admission gate.
//! The ordinary GPU runtime owns perception, action, biology, and legality.

use std::{path::Path, time::Instant};

use alife_core::{
    BrainScaleTier, CompiledSynapseKind, DecoderHeadKind, FoundationWeightAsset,
    ScaffoldContractError, SensorProfile, TrainingStageManifest,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile, GpuTrainingSamplingConfig};
use alife_training::{
    train_recurrent_ppo_cohort, AdamWConfig, FoundationTrainer, PpoBatch, PpoBoundary, PpoConfig,
    PpoJointAction, PpoTrainingState, PpoTrainingWindow, PpoTransition, StageTrainableMask,
};

use crate::{
    foundation_replay_source, initial_n2048_care_asset, load_foundation_replay_window,
    verify_foundation_replay_step, FoundationReplayBudget, FoundationReplayWriter,
    FoundationWarmupReceipt, GpuDurableSaveManifest, GpuLiveBrainRuntime,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FoundationCycleReceipt {
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
    pub food_available_world_tick: Option<u64>,
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
    run_foundation_training_cycle_from(output, seed, training_ticks, None, None)
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
    run_foundation_training_cycle_from(output, seed, training_ticks, Some(previous), None)
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
    )
}

fn run_foundation_training_cycle_from(
    output: &Path,
    seed: u64,
    training_ticks: usize,
    previous: Option<&Path>,
    food_available_world_tick: Option<u64>,
) -> Result<FoundationCycleReceipt> {
    if seed == 0 || !(1..=36_000).contains(&training_ticks) {
        return Err("cycle needs a nonzero seed and 1..=36000 training ticks".into());
    }
    if food_available_world_tick == Some(0) {
        return Err("food availability tick must be positive".into());
    }
    std::fs::create_dir(output)?;
    let (asset, policy_version, restored_actor, restored_value, founder_seed_base) =
        if let Some(previous) = previous {
            if previous.join("warmup.json").is_file() {
                let receipt: FoundationWarmupReceipt =
                    serde_json::from_slice(&std::fs::read(previous.join("warmup.json"))?)?;
                if !receipt.next_cohort_optimizer_rebound
                    || receipt.demonstration_count != 32
                    || receipt.category_counts != [8; 4]
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
                let receipt: FoundationCycleReceipt =
                    serde_json::from_slice(&std::fs::read(previous.join("cycle.json"))?)?;
                if !receipt.next_cohort_optimizer_rebound {
                    return Err("previous cycle is not an exact sealed cohort handoff".into());
                }
                let asset = FoundationWeightAsset::decode_canonical(&std::fs::read(
                    previous.join("trained.alife-foundation"),
                )?)?;
                if digest(&asset) != receipt.new_asset_digest {
                    return Err("previous exported asset does not match its receipt".into());
                }
                let actor: alife_training::FoundationTrainerCheckpoint = serde_json::from_slice(
                    &std::fs::read(previous.join("actor-checkpoint.json"))?,
                )?;
                let value: alife_training::PpoValueHeadCheckpoint = serde_json::from_slice(
                    &std::fs::read(previous.join("value-checkpoint.json"))?,
                )?;
                if actor.source_foundation_digest != asset.digest()
                    || actor.weights.len() != asset.weights().len()
                    || actor
                        .weights
                        .iter()
                        .zip(asset.weights())
                        .any(|(trained, exported)| trained.to_bits() != exported.to_bits())
                    || value.last_updated_policy_version != Some(receipt.policy_version)
                {
                    return Err(
                        "previous optimizer/value checkpoint does not match exported actor".into(),
                    );
                }
                let founder_seed_base = if receipt.founder_seed_base == 0 {
                    receipt.seed
                } else {
                    receipt.founder_seed_base
                };
                (
                    asset,
                    receipt
                        .policy_version
                        .checked_add(1)
                        .ok_or("policy version overflow")?,
                    Some(actor),
                    Some(value),
                    founder_seed_base,
                )
            }
        } else {
            (initial_n2048_care_asset(seed)?, 0, None, None, seed)
        };
    std::fs::write(
        output.join("initial.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    std::fs::write(output.join("phase.txt"), "source-written")?;

    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = founder_seed_base;
    let mut game = alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    game.world.set_age_death_disabled_for_new_game(true)?;
    let creatures = game.creatures;
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
    std::fs::write(
        output.join("scenario.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "world_seed": seed,
            "founder_seed_base": founder_seed_base,
            "food_available_world_tick": food_available_world_tick,
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
    let mut minimum_energy = initial_energy;
    let mut consumed_events = 0_u64;
    let mut first_consumed_world_tick = None;
    let mut food_available_elapsed_seconds = None;
    let mut first_consumed_elapsed_seconds = None;
    let mut food_hidden = delayed_food.is_some();

    let started = Instant::now();
    let mut production_tick_seconds = 0.0_f64;
    let mut replay_append_seconds = 0.0_f64;
    let tick_started = Instant::now();
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
    let append_started = Instant::now();
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
        runtime.tick().map_err(|e| {
            let _ = std::fs::write(
                output.join("runtime-performance-failed.json"),
                serde_json::to_vec_pretty(&runtime.performance_metrics()).unwrap_or_default(),
            );
            format!("cycle production tick after {before}: {e}")
        })?;
        production_tick_seconds += tick_started.elapsed().as_secs_f64();
        let after = runtime.world().tick().raw();
        if after == before {
            if stalled_since.elapsed().as_secs() > 300 {
                return Err("cycle made no world progress for five minutes".into());
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
        let mut captured = runtime.take_foundation_training_steps();
        if captured.is_empty() {
            if terminal_biology.is_some() {
                break;
            }
            gap = true;
            continue;
        }
        if captured.len() != 1 {
            return Err(format!("cycle world tick {after}: expected at most one decision").into());
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
        references.push(writer.append(&captured.remove(0))?);
        replay_append_seconds += append_started.elapsed().as_secs_f64();
        if terminal_biology.is_some() {
            break;
        }
    }
    let collection_seconds = started.elapsed().as_secs_f64();
    let train_rows = references.len() - usize::from(terminal_biology.is_none());
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
    let ppo_config = PpoConfig::default();
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
                    let delayed = (gap.homeostatic_improvement() - gap.aversive_harm())
                        .clamp(-1.0, 1.0)
                        * discount;
                    actions_rewards[index - 1].1 =
                        (actions_rewards[index - 1].1 + delayed).clamp(-1.0, 1.0);
                    sleep_gap_reward_total += delayed;
                }
                previous_physiology_after = Some(physiology.after);
                if index == train_rows {
                    break; // Real next state is a value bootstrap, never a loss row.
                }
                let modulator =
                    alife_core::OutcomeCreditPacket::from_sealed_patch(patch)?.modulator();
                actions_rewards.push((
                    joint_action(behavior)?,
                    (modulator.homeostatic_improvement() - modulator.pain()).clamp(-1.0, 1.0),
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
        let delayed = (transition.homeostatic_improvement() - transition.aversive_harm())
            .clamp(-1.0, 1.0)
            * discount;
        let final_reward = &mut actions_rewards
            .last_mut()
            .ok_or("terminal life has no trainable action")?
            .1;
        *final_reward = (*final_reward + delayed).clamp(-1.0, 1.0);
        sleep_gap_reward_total += delayed;
    }
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
                sequence: replay.sequence,
                batch: batch.window(start..end)?,
                auxiliary: None,
            })
        },
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
    next_config.founder_seed_base = founder_seed_base;
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
        founder_seed_base,
        policy_version,
        training_ticks: train_rows,
        requested_training_ticks: training_ticks,
        terminal_death_tick,
        delayed_food_gate_passed,
        world_ticks_elapsed,
        consumed_events,
        food_available_world_tick,
        first_consumed_world_tick,
        food_available_elapsed_seconds,
        first_consumed_elapsed_seconds,
        initial_energy,
        minimum_energy,
        final_energy,
        sleep_gap_reward_total,
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
