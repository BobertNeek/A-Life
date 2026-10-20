//! Training-only, bounded simulator branches. The current founder is frozen for
//! collection; all attempts enter one PPO batch before any genetic policy change.
use std::{path::Path, sync::Arc, time::Instant};

use crate::{FoundationTrainingStep, GpuCheckpointAssetStore, GpuLiveBrainRuntime};
use alife_core::{BrainScaleTier, FoundationWeightAsset, OrganismId, WorldEntityId};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile, GpuTrainingSamplingConfig};
use alife_training::{
    rehearsal::*, AdamWConfig, FoundationTrainer, PpoBatch, PpoBoundary, PpoConfig, PpoJointAction,
    PpoTrainingState, PpoTrainingWindow, PpoTransition,
};
use alife_world::{AssetManifest, HeadlessWorld};

mod continuation;
mod fresh;
mod snapshots;
use snapshots::*;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SimulatorRehearsalConfig {
    pub seed: u64,
    pub expected_source_policy: u64,
    pub attempts: usize,
    /// Includes burn-in and a real bootstrap decision in every attempt.
    pub decisions_per_attempt: usize,
    pub burn_in: usize,
    pub collection_wall_limit_seconds: u64,
    pub control_fresh: bool,
}
impl SimulatorRehearsalConfig {
    pub fn validate(&self) -> Result<()> {
        if self.seed == 0
            || !(4..=32).contains(&self.attempts)
            || !(8..=32).contains(&self.decisions_per_attempt)
            || !(1..=8).contains(&self.burn_in)
            || !(1..=3600).contains(&self.collection_wall_limit_seconds)
        {
            return Err("invalid rehearsal budget (4..32 attempts, 8..32 decisions, 1..8 burn-in, 1..3600 seconds)".into());
        }
        self.expected_source_policy
            .checked_add(2)
            .ok_or("policy version overflow")?;
        rehearsal_loss_range(self.decisions_per_attempt, self.burn_in, false)?;
        self.seed
            .checked_add(1_000_000 + self.attempts as u64)
            .ok_or("seed overflow")?;
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SimulatorAttemptReceipt {
    pub lineage: RehearsalLineage,
    pub variation: Option<AppliedSceneVariation>,
    pub decisions: usize,
    pub train_rows: usize,
    pub useful_successes: usize,
    pub world_ticks: usize,
    pub no_progress_polls: usize,
    pub stopped_for_sleep: bool,
    pub terminated: bool,
    /// Sum over actual loss rows only. Holdouts and untrainable attempts are
    /// not scored by this field; null must not be mistaken for measured zero.
    pub training_reward_sum: Option<f32>,
    pub elapsed_seconds: f64,
    pub retained_snapshot: Option<u64>,
}

#[derive(Debug, serde::Serialize)]
pub struct SimulatorRehearsalReceipt {
    pub schema: &'static str,
    pub config: SimulatorRehearsalConfig,
    pub source: crate::FoundationCheckpointInspection,
    pub collection_policy: u64,
    pub attempts: Vec<SimulatorAttemptReceipt>,
    pub holdouts: Vec<SimulatorAttemptReceipt>,
    pub success_pool_size: usize,
    pub difficult_failure_pool_size: usize,
    pub snapshot_captures: usize,
    pub snapshot_seconds: f64,
    pub snapshot_asset_bytes: u64,
    pub collection_seconds: f64,
    pub update_seconds: f64,
    pub holdout_seconds: f64,
    pub invalid_variations: usize,
    pub decisions: usize,
    pub holdout_decisions: usize,
    pub trained_decisions: usize,
    pub actor_optimizer_before: u32,
    pub actor_optimizer_after: u32,
    pub value_optimizer_before: u32,
    pub value_optimizer_after: u32,
    pub first_update_covers_all_trainable_attempts: bool,
    pub elapsed_seconds: f64,
    pub collection_stopped_for_budget: bool,
    pub historical_freshness: &'static str,
    pub generalization_evidence: &'static str,
}

/// The source must be a verified native objective2 cycle or exact simulator
/// continuation. Neither training head can be initialized or reset here.
/// The original source is read-only.
pub fn run_simulator_rehearsal(
    source: &Path,
    output: &Path,
    config: SimulatorRehearsalConfig,
) -> Result<SimulatorRehearsalReceipt> {
    config.validate()?;
    let started = Instant::now();
    let inspection = continuation::inspect(source, config.expected_source_policy)?;
    let asset = FoundationWeightAsset::decode_canonical(&continuation::read_verified(
        source,
        "trained.alife-foundation",
        &inspection,
    )?)?;
    let actor: alife_training::FoundationTrainerCheckpoint = serde_json::from_slice(
        &continuation::read_verified(source, "actor-checkpoint.json", &inspection)?,
    )?;
    let value_checkpoint: alife_training::PpoValueHeadCheckpoint = serde_json::from_slice(
        &continuation::read_verified(source, "value-checkpoint.json", &inspection)?,
    )?;
    let policy = config
        .expected_source_policy
        .checked_add(1)
        .ok_or("policy overflow")?;
    let digest = *asset.digest().bytes();
    let train_seeds: Vec<_> = (0..config.attempts)
        .map(|i| config.seed + i as u64)
        .collect();
    let holdout_seeds = [config.seed + 1_000_000, config.seed + 1_000_001];
    validate_scene_partition(&train_seeds, &holdout_seeds)?;
    // Check GPU availability before creating any run output.
    let device = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    std::fs::create_dir(output)?;
    let checkpoint_root = output.join("snapshot-assets");
    std::fs::create_dir(&checkpoint_root)?;
    let store = GpuCheckpointAssetStore::new(&checkpoint_root)?;
    let mut successes = DiverseRehearsalPool::new(16)?;
    let mut failures = DiverseRehearsalPool::new(8)?;
    let mut windows = Vec::new();
    let mut critic_features = Vec::new();
    let mut critic_rows = Vec::new();
    let mut receipts = Vec::new();
    let mut trainer = None;
    let mut value = PpoTrainingState::from_checkpoint(value_checkpoint.clone())?;
    let mut capture_count = 0;
    let mut invalid_variations = 0;
    let mut stopped = false;
    let ppo = PpoConfig {
        epochs: 1,
        value_coefficient: 0.0, // preserve the source residual-only critic objective
        ..PpoConfig::default()
    };
    for (attempt, seed) in train_seeds.iter().copied().enumerate() {
        if started.elapsed().as_secs() >= config.collection_wall_limit_seconds {
            stopped = true;
            break;
        }
        crate::FoundationReplayBudget::default().check_additional(128 * 1024 * 1024)?;
        if !can_capture(output, &checkpoint_root, 2)? {
            stopped = true;
            break;
        }
        let requested = rehearsal_source(attempt, config.control_fresh);
        let parent: Option<Arc<Snapshot>> = match requested {
            RehearsalSource::Success => successes.next_example().cloned(),
            RehearsalSource::DifficultFailure => failures.next_example().cloned(),
            _ => None,
        };
        let sampling = sampling(seed, attempt as u64);
        let (mut runtime, target, lineage, variation) = if let Some(parent) = parent {
            parent.lineage.require_policy(policy, digest)?;
            let organism = learner(&parent.world)?;
            let target = parent.target;
            let variant = SceneVariation::for_attempt(attempt);
            match perturb_rehearsal_world(&parent.world, organism, target, variant) {
                Ok((world, applied)) => {
                    let backend = device.new_staging_like_live()?;
                    let states = parent
                        .save
                        .creatures
                        .iter()
                        .filter_map(|creature| creature.gpu_brain.clone())
                        .collect::<Vec<_>>();
                    let mut runtime = GpuLiveBrainRuntime::restore_foundation_rehearsal(
                        backend,
                        world,
                        parent.world.seed(),
                        BrainScaleTier::Standard2048,
                        &store,
                        &parent.save.assets,
                        &states,
                        sampling,
                    )?;
                    let mut base = parent.save.clone();
                    base.save_id = format!("simulator-rehearsal-{attempt}");
                    base.replace_headless_world_snapshot(runtime.world())?;
                    attach_durable(
                        &mut runtime,
                        output,
                        &checkpoint_root,
                        attempt as u64,
                        base,
                        true,
                    )?;
                    let lineage = RehearsalLineage {
                        rollout: attempt as u64,
                        parent_rollout: Some(parent.lineage.rollout),
                        parent_snapshot: Some(parent.id),
                        world_seed: parent.world.seed(),
                        sampling_seed: u64::from(sampling.seed),
                        policy_version: policy,
                        foundation_digest: digest,
                        source: requested,
                        variation: variant,
                        historically_fresh: false,
                    };
                    (runtime, target, lineage, Some(applied))
                }
                Err(_) => {
                    invalid_variations += 1;
                    fresh_runtime(
                        &device,
                        &asset,
                        inspection.founder_seed_base,
                        seed,
                        attempt as u64,
                        policy,
                        sampling,
                        output,
                        RehearsalSource::Fresh,
                    )?
                }
            }
        } else {
            fresh_runtime(
                &device,
                &asset,
                inspection.founder_seed_base,
                seed,
                attempt as u64,
                policy,
                sampling,
                output,
                RehearsalSource::Fresh,
            )?
        };
        lineage.validate()?;
        let organism = learner(runtime.world())?;
        let organism_phenotype = runtime
            .world()
            .organism_registry()
            .get(organism)
            .ok_or("missing learner")?
            .phenotype()
            .clone();
        let attempt_started = Instant::now();
        let mut snapshot = None;
        let mut successful_snapshot = None;
        let mut steps = Vec::new();
        let mut useful = 0;
        let mut terminal_biology = None;
        let mut stopped_for_sleep = false;
        let mut world_ticks = 0;
        let mut no_progress_polls = 0;
        for decision in 0..config.decisions_per_attempt {
            if started.elapsed().as_secs() >= config.collection_wall_limit_seconds {
                stopped = true;
                break;
            }
            // Captures precede observed successes and contain the learned state
            // acquired in all preceding ticks. No success is manufactured.
            if decision.is_multiple_of(8)
                && capture_count < 128
                && can_capture(output, &checkpoint_root, 2)?
            {
                snapshot = Some(Arc::new(capture(
                    &mut runtime,
                    &store,
                    capture_count as u64,
                    target,
                    lineage.clone(),
                    output,
                )?));
                capture_count += 1;
                if directory_bytes(&checkpoint_root)? > 512 * 1024 * 1024 {
                    return Err("snapshot asset budget exceeded".into());
                }
            }
            let unheld_before = unheld(runtime.world(), target);
            let deadline =
                started + std::time::Duration::from_secs(config.collection_wall_limit_seconds);
            let Some(advanced) = progress_tick(&mut runtime, deadline, &mut no_progress_polls)?
            else {
                stopped = true;
                break;
            };
            world_ticks += advanced;
            terminal_biology = runtime
                .take_foundation_terminal_biology(organism)
                .or_else(|| {
                    runtime
                        .world()
                        .organism_registry()
                        .get(organism)
                        .and_then(|record| {
                            (!record.lifecycle().is_alive()).then_some(*record.biochemistry())
                        })
                });
            let mut captured = runtime.take_foundation_training_steps();
            if captured.is_empty() {
                stopped_for_sleep = terminal_biology.is_none();
                break; // train the completed prefix instead of discarding prior attempts
            }
            if captured.len() != 1 {
                return Err("rehearsal captured more than one private learner".into());
            }
            let step = captured.remove(0);
            if useful_success(&step, target, unheld_before, runtime.world()) {
                useful += 1;
                if successful_snapshot.is_none() {
                    successful_snapshot = snapshot.clone();
                }
            }
            steps.push(step);
            if terminal_biology.is_some() {
                break;
            }
        }
        let terminated = terminal_biology.is_some();
        if steps.len() < if terminated { 1 } else { 2 } {
            receipts.push(SimulatorAttemptReceipt {
                lineage,
                variation,
                decisions: steps.len(),
                train_rows: 0,
                useful_successes: useful,
                world_ticks,
                no_progress_polls,
                stopped_for_sleep,
                terminated,
                training_reward_sum: None,
                elapsed_seconds: attempt_started.elapsed().as_secs_f64(),
                retained_snapshot: None,
            });
            continue;
        }
        let burn_in = config
            .burn_in
            .min(steps.len() - if terminated { 1 } else { 2 });
        let mut sequence = crate::foundation_replay_sequence(&steps, burn_in)?;
        let phenotype = steps[0].before.phenotype.as_ref().clone();
        asset.validate_against(&phenotype)?;
        if trainer.is_none() {
            let mut next = FoundationTrainer::from_session(
                alife_runtime::GpuAuthoritativeSession::new(
                    device.new_staging_like_live()?,
                    alife_runtime::GpuSessionConsumerKind::Training,
                ),
                phenotype,
                asset.clone(),
                actor.stage_mask.clone(),
                AdamWConfig::default(),
            )?;
            next.restore_checkpoint(&actor)?;
            if serde_json::to_vec(&next.checkpoint()?)? != serde_json::to_vec(&actor)? {
                return Err("actor changed before training".into());
            }
            trainer = Some(next);
        }
        let trainer = trainer.as_mut().ok_or("missing trainer")?;
        // Predict every value with the unmodified source head before any update.
        let values = value.predict_values(trainer, &sequence)?;
        if windows.is_empty()
            && serde_json::to_vec(&value.checkpoint(trainer.session())?)?
                != serde_json::to_vec(&value_checkpoint)?
        {
            return Err("value head changed before training".into());
        }
        let replay_dir = output.join(format!("replay-{attempt:03}"));
        let replay_source = crate::foundation_training::foundation_replay_source(
            &steps[0].before.phenotype,
            &asset,
            policy,
            &serde_json::to_vec(&actor)?,
        )?;
        let mut writer = crate::FoundationReplayWriter::new(
            &replay_dir,
            replay_source,
            steps[0].before.phenotype.as_ref().clone(),
            &asset,
            crate::FoundationReplayBudget::default(),
        )?;
        let references = steps
            .iter()
            .map(|step| writer.append(step))
            .collect::<Result<Vec<_>>>()?;
        std::fs::write(
            replay_dir.join("manifest.json"),
            serde_json::to_vec_pretty(&references)?,
        )?;
        let feature_base = critic_features.len();
        for step in &steps {
            critic_features.push(crate::foundation_training::snapshot_activation(
                &step.after_inference,
            )?);
        }
        let range = rehearsal_loss_range(steps.len(), burn_in, terminated)?;
        let mut transitions = Vec::new();
        let mut reward_sum = 0.0;
        for index in range.clone() {
            let step = &steps[index];
            let mut reward = biological_reward(step, &organism_phenotype)?;
            let terminal = terminated && index + 1 == range.end;
            let mut elapsed_seconds = if terminal {
                0.05
            } else {
                (steps[index + 1].frame.tick().raw() - step.frame.tick().raw()) as f32 / 20.0
            };
            if terminal {
                let after = step
                    .patch
                    .outcome()
                    .measured_physiology
                    .ok_or("terminal action missing physiology")?
                    .after;
                let final_state = terminal_biology.ok_or("missing terminal biology")?;
                let (elapsed, tail_discount) = terminal_credit_timing(
                    step.frame.tick().raw(),
                    after.tick.raw(),
                    final_state.tick.raw(),
                    ppo.discount_half_life_seconds,
                )?;
                elapsed_seconds = elapsed;
                if final_state.tick > after.tick {
                    let gap = alife_core::MeasuredPhysiologyTransition::new(after, final_state)?;
                    reward = (reward
                        + biological_gap_reward(&gap, &organism_phenotype)? * tail_discount)
                        .clamp(-1.0, 1.0);
                }
            }
            reward_sum += reward;
            critic_rows.push(alife_training::CriticResidualRow {
                current: (feature_base + index) as u32,
                next: (feature_base + if terminal { index } else { index + 1 }) as u32,
                reward,
                discount: if terminal {
                    0.0
                } else {
                    (-std::f64::consts::LN_2 * f64::from(elapsed_seconds)
                        / f64::from(ppo.discount_half_life_seconds))
                    .exp() as f32
                },
            });
            transitions.push(PpoTransition {
                policy_version: policy,
                trajectory_id: attempt as u64,
                step: index as u64,
                action: joint_action(&step.behavior)?,
                reward,
                old_value: values[index],
                next_value: if terminal { 0.0 } else { values[index + 1] },
                elapsed_seconds,
                boundary: if terminal {
                    PpoBoundary::Terminated
                } else if index + 1 == range.end {
                    PpoBoundary::Truncated
                } else {
                    PpoBoundary::Continuing
                },
            });
        }
        // The final real decision bootstraps truncation; it has no loss.
        if !terminated {
            sequence.ticks.pop();
        }
        let batch = PpoBatch::from_rollout(policy, transitions, ppo)?;
        windows.push(PpoTrainingWindow {
            speech_targets: vec![None; sequence.ticks.len()],
            sequence,
            batch,
            auxiliary: None,
        });
        let difficult = terminated
            || steps.iter().any(|step| {
                let outcome = step.patch.outcome();
                let blocked = |physical: &alife_core::PhysicalActionOutcome| {
                    matches!(
                        physical.contact,
                        alife_core::PhysicalContactKind::Blocked
                            | alife_core::PhysicalContactKind::Collision
                    )
                };
                blocked(&outcome.physical)
                    || outcome.joint.as_ref().is_some_and(|joint| {
                        joint
                            .channel_outcomes
                            .iter()
                            .any(|channel| blocked(&channel.physical))
                    })
                    || outcome
                        .measured_physiology
                        .is_some_and(|transition| transition.aversive_value() > 0.0)
            });
        let retained = if useful > 0 {
            successful_snapshot
        } else if difficult {
            snapshot
        } else {
            None
        };
        let retained_id = retained.as_ref().map(|s| s.id);
        if let Some(snapshot) = retained {
            if useful > 0 {
                successes.retain(lineage.world_seed, lineage.variation, snapshot);
            } else {
                failures.retain(lineage.world_seed, lineage.variation, snapshot);
            }
        }
        let receipt = SimulatorAttemptReceipt {
            lineage,
            variation,
            decisions: steps.len(),
            world_ticks,
            no_progress_polls,
            stopped_for_sleep,
            terminated,
            train_rows: range.len(),
            useful_successes: useful,
            training_reward_sum: Some(reward_sum),
            elapsed_seconds: attempt_started.elapsed().as_secs_f64(),
            retained_snapshot: retained_id,
        };
        std::fs::write(
            output.join(format!("attempt-{attempt:03}.json")),
            serde_json::to_vec_pretty(&receipt)?,
        )?;
        receipts.push(receipt);
    }
    if windows.is_empty() {
        return Err("wall budget expired before any complete attempt".into());
    }
    let collection_seconds = started.elapsed().as_secs_f64();
    let update_started = Instant::now();
    let mut trainer = trainer.ok_or("missing actor")?;
    // One accumulation batch includes every attempt, including every failure,
    // before the first update. KL stopping therefore cannot omit later attempts.
    let update = alife_training::train_recurrent_ppo_cohort(
        &mut trainer,
        &mut value,
        windows.len(),
        windows.len(),
        |index| Ok(windows[index].clone()),
        ppo,
        policy,
    )?;
    if update.actor_optimizer_step <= actor.optimizer_step {
        return Err("all-attempt optimizer update was not applied".into());
    }
    let mut residual = alife_training::FrozenCriticResidualExperiment::new(
        trainer.session(),
        &value.checkpoint(trainer.session())?,
        &critic_features,
        &critic_rows,
        critic_rows.len(),
    )?;
    let critic_before = residual.evaluate(trainer.session())?;
    residual.step_with_coefficient(trainer.session(), ppo.value_optimizer, 0.5)?;
    let critic_after = residual.evaluate(trainer.session())?;
    let mut final_value = residual.checkpoint(trainer.session())?;
    final_value.last_updated_policy_version = Some(policy);
    if final_value.optimizer_step <= value_checkpoint.optimizer_step {
        return Err("all-attempt residual critic update was not applied".into());
    }
    std::fs::write(
        output.join("critic-residual-update.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"objective_version":1,"objective":"full_sampled_one_step_bellman_residual","feature_authority":"captured_production_GPU_after_inference","actor_GAE_unchanged":true,"loss_rows":critic_rows.len(),"bootstrap_is_loss_row":false,"initial_age":value_checkpoint.optimizer_step,"final_age":final_value.optimizer_step,"Adam_reset":false,"before":critic_before,"after":critic_after}),
        )?,
    )?;
    let trained = trainer.export_candidate(asset.manifest().training_stage())?;
    let next_phenotype = crate::foundation_archived_success_replay::compile_source(
        trained.clone(),
        config.seed,
        inspection.founder_seed_base,
    )?;
    trainer.rebind_for_next_cohort(next_phenotype, trained.clone())?;
    std::fs::write(
        output.join("trained.alife-foundation"),
        trained.encode_canonical()?,
    )?;
    std::fs::write(
        output.join("actor-checkpoint.json"),
        serde_json::to_vec(&trainer.checkpoint()?)?,
    )?;
    std::fs::write(
        output.join("value-checkpoint.json"),
        serde_json::to_vec(&final_value)?,
    )?;
    let update_seconds = update_started.elapsed().as_secs_f64();
    let holdout_started = Instant::now();
    let mut holdouts = Vec::new();
    let mut next_cohort_checked = false;
    for (index, seed) in holdout_seeds.into_iter().enumerate() {
        let (mut runtime, target, lineage, _) = fresh_runtime(
            &device,
            &trained,
            inspection.founder_seed_base,
            seed,
            config.attempts as u64 + index as u64,
            policy + 1,
            sampling(seed, 0),
            output,
            RehearsalSource::Holdout,
        )?;
        let time = Instant::now();
        let organism = learner(runtime.world())?;
        let mut useful = 0;
        let mut decisions = 0;
        let mut world_ticks = 0;
        let mut no_progress_polls = 0;
        let mut terminated = false;
        let mut stopped_for_sleep = false;
        for _ in 0..config.decisions_per_attempt {
            let unheld_before = unheld(runtime.world(), target);
            let deadline =
                time + std::time::Duration::from_secs(config.collection_wall_limit_seconds);
            let Some(advanced) = progress_tick(&mut runtime, deadline, &mut no_progress_polls)?
            else {
                break;
            };
            world_ticks += advanced;
            terminated = runtime.take_foundation_terminal_biology(organism).is_some();
            let steps = runtime.take_foundation_training_steps();
            if steps.is_empty() {
                stopped_for_sleep = !terminated;
                break;
            }
            for step in steps {
                trained.validate_against(&step.before.phenotype)?;
                if step.before.phenotype.as_ref() != trainer.phenotype() {
                    return Err("exported next cohort graph differs from rebound actor".into());
                }
                next_cohort_checked = true;
                decisions += 1;
                useful += usize::from(useful_success(
                    &step,
                    target,
                    unheld_before,
                    runtime.world(),
                ));
            }
            if terminated {
                break;
            }
        }
        holdouts.push(SimulatorAttemptReceipt {
            lineage,
            variation: None,
            decisions,
            world_ticks,
            no_progress_polls,
            stopped_for_sleep,
            terminated,
            train_rows: 0,
            useful_successes: useful,
            training_reward_sum: None,
            elapsed_seconds: time.elapsed().as_secs_f64(),
            retained_snapshot: None,
        });
    }
    if !next_cohort_checked {
        return Err("exported next cohort produced no real decision".into());
    }
    let receipt = SimulatorRehearsalReceipt {
        schema: "alife.simulator-rehearsal.v1",
        config,
        collection_policy: policy,
        attempts: receipts.clone(),
        holdouts: holdouts.clone(),
        source: inspection,
        success_pool_size: successes.len(),
        difficult_failure_pool_size: failures.len(),
        snapshot_captures: capture_ledger(output)?.completed,
        snapshot_seconds: capture_ledger(output)?.elapsed_seconds,
        snapshot_asset_bytes: directory_bytes(&checkpoint_root)?,
        collection_seconds,
        update_seconds,
        holdout_seconds: holdout_started.elapsed().as_secs_f64(),
        invalid_variations,
        decisions: receipts.iter().map(|attempt| attempt.decisions).sum(),
        holdout_decisions: holdouts.iter().map(|attempt| attempt.decisions).sum(),
        trained_decisions: windows.iter().map(|window| window.batch.len()).sum(),
        actor_optimizer_before: actor.optimizer_step,
        actor_optimizer_after: update.actor_optimizer_step,
        value_optimizer_before: value_checkpoint.optimizer_step,
        value_optimizer_after: final_value.optimizer_step,
        first_update_covers_all_trainable_attempts: true,
        elapsed_seconds: started.elapsed().as_secs_f64(),
        collection_stopped_for_budget: stopped,
        historical_freshness: "unknown",
        generalization_evidence:
            "within-run disjoint holdouts only; historical unseen-scene generalization unknown",
    };
    std::fs::write(
        output.join("rehearsal.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    // Publish last: an incomplete run has no admissible continuation marker.
    continuation::publish(
        output,
        policy,
        &receipt.source,
        trainer.optimizer_step(),
        final_value.optimizer_step,
    )?;
    Ok(receipt)
}

fn sampling(seed: u64, attempt: u64) -> GpuTrainingSamplingConfig {
    GpuTrainingSamplingConfig {
        seed: ((seed ^ (seed >> 32)) as u32)
            .wrapping_add((attempt as u32).wrapping_mul(0x9e37_79b9))
            .max(1),
        counter: 0,
        temperature: 1.0,
        demonstrator: None,
    }
}
fn learner(world: &HeadlessWorld) -> Result<OrganismId> {
    if world.organism_registry().len() != 1 {
        return Err("rehearsal currently requires one private organism".into());
    }
    Ok(world
        .organism_registry()
        .iter()
        .next()
        .ok_or("missing learner")?
        .organism_id())
}
#[allow(clippy::too_many_arguments)]
fn fresh_runtime(
    device: &GpuClosedLoopBackend,
    asset: &FoundationWeightAsset,
    founder: u64,
    seed: u64,
    rollout: u64,
    policy: u64,
    sampling: GpuTrainingSamplingConfig,
    output: &Path,
    source: RehearsalSource,
) -> Result<(
    GpuLiveBrainRuntime,
    WorldEntityId,
    RehearsalLineage,
    Option<AppliedSceneVariation>,
)> {
    let context = fresh::SetupContext {
        founder,
        seed,
        rollout,
    };
    let (game, target) = fresh::prepare_scene(asset, &context)?;
    let creatures = game.creatures;
    let staging = context.stage("GPU staging allocation", device.new_staging_like_live())?;
    let mut runtime = context.stage(
        "GPU resident construction",
        GpuLiveBrainRuntime::new_profiled_foundation_training(
            staging,
            game.world,
            seed,
            BrainScaleTier::Standard2048,
            asset.manifest().sensor_profile(),
            alife_archive::LineageLibraryConfig::profile_default(output.join("lineage")),
            format!("simulator-rehearsal-{rollout}"),
            alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
            sampling,
        ),
    )?;
    let mut creatures = creatures;
    for creature in &mut creatures {
        let biology = runtime
            .world()
            .organism_registry()
            .get(creature.organism_id)
            .ok_or("missing fresh biology")?
            .biochemistry();
        creature.development_tick = biology.development.last_update_tick;
        creature.mind.tick = biology.tick;
        creature.mind.homeostasis = biology.homeostasis;
    }
    let mut runtime_config =
        alife_world::RuntimeConfig::deterministic_default(seed, BrainScaleTier::Standard2048);
    runtime_config.features.gpu_backend_enabled = true;
    let base = context.stage(
        "portable base construction",
        alife_world::PortableSaveFile::from_headless_world(
            format!("simulator-rehearsal-{rollout}"),
            runtime.world(),
            runtime_config,
            AssetManifest::empty(),
            creatures,
        ),
    )?;
    context.stage(
        "initial durable snapshot",
        attach_durable(
            &mut runtime,
            output,
            &output.join("snapshot-assets"),
            rollout,
            base,
            false,
        ),
    )?;
    let lineage = RehearsalLineage {
        rollout,
        parent_rollout: None,
        parent_snapshot: None,
        world_seed: seed,
        sampling_seed: u64::from(sampling.seed),
        policy_version: policy,
        foundation_digest: *asset.digest().bytes(),
        source,
        variation: SceneVariation::Original,
        historically_fresh: false,
    };
    Ok((runtime, target, lineage, None))
}
fn biological_reward(
    step: &FoundationTrainingStep,
    phenotype: &alife_core::CreaturePhenotype,
) -> Result<f32> {
    let transition = step
        .patch
        .outcome()
        .measured_physiology
        .ok_or("missing canonical physiology")?;
    let receptors = transition.before.neural_receptor_frame(phenotype)?;
    let modulator = step
        .outcome_credit
        .with_biochemical_receptors(&receptors)?
        .modulator();
    let genes = phenotype.brain_genome.plasticity_parameters();
    let profile = genes.action_candidate_credit_profile().map_or_else(
        || genes.receptor_profile(),
        |profile| profile.receptor_profile(),
    );
    Ok(profile.project(&modulator.frame())?)
}
fn unheld(world: &HeadlessWorld, target: WorldEntityId) -> bool {
    world
        .entity(target)
        .is_some_and(|food| !food.consumed && food.carried_by.is_none())
}
fn useful_success(
    step: &FoundationTrainingStep,
    target: WorldEntityId,
    unheld_before: bool,
    world: &HeadlessWorld,
) -> bool {
    use alife_core::{MotorChannel, PhysicalContactKind};
    if step.behavior.on_policy_log_probability().is_err() {
        return false;
    }
    let outcome = step.patch.outcome();
    let Some(bundle) = step.patch.selected_bundle() else {
        return false;
    };
    let Some(command) = bundle.channels.iter().find(|command| {
        command.channel == MotorChannel::Manipulation
            && command.target.and_then(|target| target.entity) == Some(target)
    }) else {
        return false;
    };
    let Some(physical) = outcome
        .joint
        .as_ref()
        .and_then(|joint| {
            joint.channel_outcomes.iter().find(|channel| {
                channel.channel == MotorChannel::Manipulation
                    && channel.physical.target_entity == Some(target)
            })
        })
        .map(|channel| channel.physical)
    else {
        return false;
    };
    let acquired = command.primitive == alife_world::HeadlessActionIds::GRAB
        && physical.contact == PhysicalContactKind::Touch
        && unheld_before
        && world.entity(target).is_some_and(|food| {
            !food.consumed && food.carried_by == Some(step.frame.organism_id())
        });
    let nourished = command.primitive == alife_world::HeadlessActionIds::EAT
        && physical.contact == PhysicalContactKind::Consumed
        && world.entity(target).is_some_and(|food| food.consumed)
        && outcome.measured_physiology.is_some_and(|transition| {
            (transition.after.body.energy > transition.before.body.energy
                || transition.after.homeostasis.drives.hunger
                    < transition.before.homeostasis.drives.hunger)
                && transition.after.body.health >= transition.before.body.health
                && transition.after.body.injury <= transition.before.body.injury
                && transition.after.homeostasis.drives.pain
                    <= transition.before.homeostasis.drives.pain
        });
    acquired || nourished
}
fn joint_action(behavior: &alife_gpu_backend::GpuTrainingRolloutReceipt) -> Result<PpoJointAction> {
    let action = PpoJointAction {
        candidate_count: u16::try_from(behavior.forced_motor_slots.len())?,
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

fn biological_gap_reward(
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
    let genes = phenotype.brain_genome.plasticity_parameters();
    let profile = genes.action_candidate_credit_profile().map_or_else(
        || genes.receptor_profile(),
        |profile| profile.receptor_profile(),
    );
    Ok(profile.project(&sample.frame())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> SimulatorRehearsalConfig {
        SimulatorRehearsalConfig {
            seed: 71_001,
            expected_source_policy: 7400,
            attempts: 4,
            decisions_per_attempt: 8,
            burn_in: 2,
            collection_wall_limit_seconds: 60,
            control_fresh: false,
        }
    }
    #[test]
    fn invalid_budgets_and_missing_source_fail_before_output_or_device() {
        let mut invalid = config();
        invalid.seed = u64::MAX;
        assert!(invalid.validate().is_err());
        invalid = config();
        invalid.expected_source_policy = u64::MAX - 1;
        assert!(invalid.validate().is_err());
        invalid = config();
        invalid.burn_in = 7;
        assert!(invalid.validate().is_err());
        let output =
            std::env::temp_dir().join(format!("alife-rehearsal-missing-{}", std::process::id()));
        assert!(!output.exists());
        assert!(
            run_simulator_rehearsal(&output.join("missing-source"), &output, config()).is_err()
        );
        assert!(!output.exists());
    }
}

/// Matched decision/collection-time ceilings from the same immutable source.
/// Reports actual wall costs; it cannot establish a speedup when either arm hits
/// its time ceiling or receives fewer trainable decisions.
pub fn compare_simulator_rehearsal(
    source: &Path,
    output: &Path,
    mut config: SimulatorRehearsalConfig,
) -> Result<serde_json::Value> {
    config.validate()?;
    continuation::inspect(source, config.expected_source_policy)?;
    std::fs::create_dir(output)?;
    config.control_fresh = true;
    let control = run_simulator_rehearsal(source, &output.join("fresh-control"), config.clone())?;
    config.control_fresh = false;
    let varied = run_simulator_rehearsal(source, &output.join("varied-rehearsal"), config)?;
    let metrics = |receipt: &SimulatorRehearsalReceipt| {
        let discovered = receipt
            .attempts
            .iter()
            .filter(|attempt| {
                attempt.lineage.source == RehearsalSource::Fresh && attempt.useful_successes > 0
            })
            .count();
        let retries = receipt
            .attempts
            .iter()
            .filter(|attempt| attempt.lineage.source == RehearsalSource::Success)
            .collect::<Vec<_>>();
        let retained = retries
            .iter()
            .filter(|attempt| attempt.useful_successes > 0)
            .count();
        serde_json::json!({"fresh_scene_discoveries":discovered,"successful_rehearsal_attempts":retained,"success_rehearsal_attempts":retries.len(),"success_rehearsal_fraction":if retries.is_empty(){None}else{Some(retained as f64/retries.len() as f64)},"useful_successes":receipt.attempts.iter().map(|attempt|attempt.useful_successes).sum::<usize>(),"collection_decisions":receipt.decisions,"trained_decisions":receipt.trained_decisions,"holdout_decisions":receipt.holdout_decisions,"within_run_disjoint_holdout_successes":receipt.holdouts.iter().map(|attempt|attempt.useful_successes).sum::<usize>(),"collection_seconds":receipt.collection_seconds,"update_seconds":receipt.update_seconds,"holdout_seconds":receipt.holdout_seconds,"total_seconds":receipt.elapsed_seconds,"snapshot_seconds":receipt.snapshot_seconds,"invalid_variations":receipt.invalid_variations,"collection_stopped_for_budget":receipt.collection_stopped_for_budget})
    };
    let comparison = serde_json::json!({"schema":"alife.simulator-rehearsal-comparison.v1","same_source_files":control.source.file_digests.iter().map(|file|(&file.file_name,file.digest)).collect::<Vec<_>>()==varied.source.file_digests.iter().map(|file|(&file.file_name,file.digest)).collect::<Vec<_>>(),"same_collection_decisions":control.decisions==varied.decisions,"same_trainable_decisions":control.trained_decisions==varied.trained_decisions,"arm_order":["fresh_control","varied_rehearsal"],"fresh_control":metrics(&control),"varied_rehearsal":metrics(&varied),"historical_freshness":"unknown","unseen_scene_generalization":"unknown; only within-run disjoint holdouts are measured","rehearsal_is_correlated_evidence":true});
    std::fs::write(
        output.join("comparison.json"),
        serde_json::to_vec_pretty(&comparison)?,
    )?;
    Ok(comparison)
}

#[cfg(all(test, feature = "gpu-tests"))]
mod gpu_tests {
    use super::*;
    #[test]
    #[ignore = "requires an explicitly coordinated supported hardware GPU session"]
    fn exact_restore_preserves_private_context_and_next_real_consequence() {
        let output =
            std::env::temp_dir().join(format!("alife-simulator-restore-{}", std::process::id()));
        std::fs::create_dir(&output).unwrap();
        std::fs::create_dir(output.join("snapshot-assets")).unwrap();
        let device =
            GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
        let asset = crate::initial_n2048_care_asset(77_777).unwrap();
        let sampling = sampling(99_001, 0);
        let (mut original, target, lineage, _) = fresh_runtime(
            &device,
            &asset,
            77_777,
            99_001,
            0,
            7401,
            sampling,
            &output,
            RehearsalSource::Fresh,
        )
        .unwrap();
        let mut resume_sampling = sampling;
        for _ in 0..3 {
            let mut waits = 0;
            progress_tick(
                &mut original,
                Instant::now() + std::time::Duration::from_secs(60),
                &mut waits,
            )
            .unwrap()
            .unwrap();
            let steps = original.take_foundation_training_steps();
            assert_eq!(steps.len(), 1);
            resume_sampling.counter = steps[0].behavior.sampling.counter.checked_add(1).unwrap();
        }
        let store = GpuCheckpointAssetStore::new(output.join("snapshot-assets")).unwrap();
        let snapshot = capture(&mut original, &store, 0, target, lineage, &output).unwrap();
        let states = snapshot
            .save
            .creatures
            .iter()
            .filter_map(|creature| creature.gpu_brain.clone())
            .collect::<Vec<_>>();
        let mut restored = GpuLiveBrainRuntime::restore_foundation_rehearsal(
            device.new_staging_like_live().unwrap(),
            snapshot.world.clone(),
            snapshot.world.seed(),
            BrainScaleTier::Standard2048,
            &store,
            &snapshot.save.assets,
            &states,
            resume_sampling,
        )
        .unwrap();
        let mut base = snapshot.save.clone();
        base.save_id = "simulator-rehearsal-1".into();
        attach_durable(&mut restored, &output, store.root(), 1, base, true).unwrap();
        assert_eq!(
            original.world().canonical_signature_digest().unwrap(),
            restored.world().canonical_signature_digest().unwrap()
        );
        let mut waits = 0;
        progress_tick(
            &mut original,
            Instant::now() + std::time::Duration::from_secs(60),
            &mut waits,
        )
        .unwrap()
        .unwrap();
        progress_tick(
            &mut restored,
            Instant::now() + std::time::Duration::from_secs(60),
            &mut waits,
        )
        .unwrap()
        .unwrap();
        let expected = original.take_foundation_training_steps();
        let actual = restored.take_foundation_training_steps();
        assert_eq!(expected.len(), 1);
        assert_eq!(actual.len(), 1);
        assert_eq!(
            expected[0].frame.frame_digest(),
            actual[0].frame.frame_digest()
        );
        assert_eq!(expected[0].behavior.logits, actual[0].behavior.logits);
        assert_eq!(
            expected[0].behavior.joint_log_probability,
            actual[0].behavior.joint_log_probability
        );
        assert_eq!(
            expected[0].patch.outcome().measured_physiology,
            actual[0].patch.outcome().measured_physiology
        );
        let expected_sequence = crate::foundation_replay_sequence(&expected, 0).unwrap();
        let actual_sequence = crate::foundation_replay_sequence(&actual, 0).unwrap();
        assert_eq!(expected_sequence, actual_sequence);
        assert_eq!(
            original.world().canonical_signature_digest().unwrap(),
            restored.world().canonical_signature_digest().unwrap()
        );
        std::fs::remove_dir_all(output).unwrap();
    }
}

fn progress_tick(
    runtime: &mut GpuLiveBrainRuntime,
    deadline: Instant,
    no_progress: &mut usize,
) -> Result<Option<usize>> {
    while Instant::now() < deadline {
        let before = runtime.world().tick().raw();
        let outcome = runtime.tick_outcome()?;
        let after = runtime.world().tick().raw();
        match outcome {
            crate::GpuLiveTickOutcome::NoProgress(_) => {
                tick_delta(before, after, false)?;
                *no_progress += 1;
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            crate::GpuLiveTickOutcome::Progressed(_) => {
                return tick_delta(before, after, true);
            }
        }
    }
    Ok(None)
}

fn tick_delta(before: u64, after: u64, progressed: bool) -> Result<Option<usize>> {
    if !progressed {
        if after != before {
            return Err("no-progress checkpoint wait changed world time".into());
        }
        return Ok(None);
    }
    let delta = after
        .checked_sub(before)
        .filter(|delta| *delta > 0)
        .ok_or("progressed tick did not advance world time")?;
    Ok(Some(usize::try_from(delta)?))
}
#[cfg(test)]
mod progress_tests {
    use super::*;
    #[test]
    fn checkpoint_wait_is_neither_world_progress_nor_sleep() {
        assert_eq!(tick_delta(9, 9, false).unwrap(), None);
        assert_eq!(tick_delta(9, 10, true).unwrap(), Some(1));
        assert!(tick_delta(9, 10, false).is_err());
        assert!(tick_delta(9, 9, true).is_err());
        assert!(tick_delta(9, 8, true).is_err());
    }
}
