//! One restored individual/world with bounded frozen-policy PPO windows.
//! No world, recurrent state, biology or sampler reset occurs between updates.
use super::*;
use alife_core::{N2048LivePolicyTransition, OrganismId};
use alife_training::{
    CurrentTrainingCheckpointV1, FoundationTrainingBackendV1, FoundationTrainingHandoffV1,
};
use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}

/// A partial live rollout always keeps its actual last action as bootstrap.
/// No clock advance or fictitious next state is manufactured at a deadline.
fn loss_rows(captured: usize, requested: usize) -> usize {
    captured.saturating_sub(1).min(requested)
}

fn tick_limit(decisions: usize) -> Result<u64> {
    (decisions as u64)
        .checked_mul(8)
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(|| "continuous collection tick bound overflow".into())
}

fn alive(runtime: &GpuLiveBrainRuntime, id: OrganismId) -> bool {
    runtime
        .world()
        .organism_registry()
        .get(id)
        .is_some_and(|r| r.lifecycle().is_alive())
}

fn compact_prior(runtime: &GpuLiveBrainRuntime) -> serde_json::Value {
    runtime.semantic_prior_metrics().map_or(serde_json::Value::Null, |m| serde_json::json!({
        "backend":m.backend,"provider_identity":m.provider_identity,
        "decision_frames":m.decision_frames,"decision_frames_with_prior":m.decision_frames_with_prior,
        "decision_frames_without_prior":m.decision_frames_without_prior,
        "delivered_frames":m.delivered_frames,"failures":m.failures,
        "last_error":m.last_error,"teacher_cue_frames":0,
    }))
}

fn handoff(
    runtime: &GpuLiveBrainRuntime,
    id: OrganismId,
    trainer: &FoundationTrainer,
    value: &PpoTrainingState,
    policy: u64,
) -> Result<FoundationTrainingHandoffV1> {
    Ok(FoundationTrainingHandoffV1::new_same_individual(
        FoundationTrainingBackendV1::NativeGpu,
        policy,
        runtime.live_foundation_compiler_inputs(id)?.clone(),
        trainer.checkpoint()?,
        value.checkpoint(trainer.session())?,
    )?)
}

/// The callback durably seals the returned heads with this exact live private
/// closure before it returns. Its owner owns WAL, publication and aggregate quota.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_same_individual_world(
    baseline: &CurrentTrainingCheckpointV1,
    session_nonce: u64,
    request: &super::continuous_store::ResidentContinuousRequest,
    work: &Path,
    mut on_snapshot: impl FnMut(
        FoundationTrainingHandoffV1,
        &Path,
        SameIndividualProgressV1,
    ) -> Result<()>,
    mut on_update: impl FnMut(serde_json::Value) -> Result<()>,
) -> Result<serde_json::Value> {
    let started = Instant::now();
    let life = work.join("live");
    materialize_current_training_life(baseline, &life)?;
    let save_path = life.join("world.json");
    let assets = life.join("world-assets");
    let (_, loaded) = GpuDurableSaveManifest::open_loaded(&save_path, &assets)?;
    let world = loaded.save.restore_headless_world()?;
    let residents = world.organism_registry().iter().collect::<Vec<_>>();
    if residents.len() != 1 || !residents[0].lifecycle().is_alive() {
        return Err(
            "same-individual training requires exactly one living saved organism; no resurrection"
                .into(),
        );
    }
    let id = residents[0].organism_id();
    let body = residents[0].phenotype().clone();
    let world_seed = world.seed();
    if world_seed != baseline.seed {
        return Err("saved world seed differs from admitted Current".into());
    }
    let start_tick = world.tick().raw();
    let initial_policy = baseline.handoff.policy_version;
    let sampling =
        cycle_sampling_config(session_nonce, initial_policy, request.sampling_temperature)?;
    let store = crate::GpuCheckpointAssetStore::new(&assets)?;
    let checkpoints = loaded
        .save
        .creatures
        .iter()
        .filter_map(|c| c.gpu_brain.clone())
        .collect::<Vec<_>>();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let hardware = backend.hardware_receipt().clone();
    let mut runtime = GpuLiveBrainRuntime::restore_foundation_rehearsal(
        backend,
        world,
        world_seed,
        BrainScaleTier::Standard2048,
        &store,
        &loaded.save.assets,
        &checkpoints,
        sampling,
    )?;
    runtime.rebind_durable_checkpoint_boundary(&save_path, &assets, &loaded.save)?;
    runtime.attach_lineage_archive(
        alife_archive::LineageLibraryConfig::profile_default(work.join("lineage")),
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
    )?;
    let sampler_restored = runtime.foundation_training_sampling_was_restored(id);
    let sampling = runtime
        .foundation_training_sampling_state()
        .ok_or("training sampler absent")?;
    // A present saved stream is authoritative, including temperature. The request
    // only initializes an explicitly new stream when legacy metadata is absent.
    let ppo = cycle_ppo_config(sampling.temperature)?;
    let staging = runtime.new_staging_like_live()?;
    let (mut trainer, mut value) = baseline.handoff.clone().into_gpu_training_session(
        alife_runtime::GpuAuthoritativeSession::new(
            staging,
            alife_runtime::GpuSessionConsumerKind::Training,
        ),
    )?;
    let mut asset = baseline
        .handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or("admitted Current has no foundation asset")?
        .clone();
    let mut policy = initial_policy;
    let inputs = runtime.live_foundation_compiler_inputs(id)?.clone();
    match inputs.live_policy() {
        Some(marker)
            if marker.policy_version == policy && inputs == baseline.handoff.compiler_inputs =>
        {
            ()
        }
        Some(_) => return Err("saved live policy is not the admitted global policy".into()),
        None => {
            // Legacy fresh-cohort Current paired a PRE-PPO individual with newer
            // global heads. Adopt those accepted heads once, preserving all
            // mutable private state; do not pretend the private actor was newer.
            let transition =
                N2048LivePolicyTransition::adopt_existing_policy(&inputs, policy, asset.clone())?;
            runtime.quiesce_live_foundation_policy_boundary(
                id,
                request.finalization_deadline_unix_seconds,
            )?;
            runtime.install_live_foundation_policy(id, &transition)?;
            trainer.rebind_for_next_cohort(transition.next_phenotype().clone(), asset.clone())?;
        }
    }
    runtime.set_performance_measurement_enabled(true);
    let mut trained_rows = 0u64;
    let mut completed_updates = 0u64;
    let mut saved_rows = 0u64;
    let mut saved_updates = 0u64;
    let mut saved_tick = start_tick;
    let mut saved_policy = initial_policy;
    let mut snapshot_seconds = 0.0;
    let mut collection_seconds = 0.0;
    let mut ppo_seconds_total = 0.0;
    let mut install_seconds_total = 0.0;
    let mut replay_seal_seconds = 0.0;
    let mut boundary_wait_seconds = 0.0;
    let mut no_progress_total = 0u64;
    let initial_capture_metrics = runtime.foundation_training_capture_metrics();
    let mut captured_total = 0u64;
    let mut generic_total = 0u64;
    let mut nourished_total = 0u64;
    let mut stop = "decision_horizon";
    let mut terminal_death_tick = None;
    // Observe only actual restored/current food entities. No food is spawned,
    // moved, replenished or assigned to the learner by this collector.
    let mut food = BTreeMap::new();
    let restore_seconds = started.elapsed().as_secs_f64();
    on_update(
        serde_json::json!({"kind":"continuous-started-v1","session_nonce":session_nonce,
            "source_world_seed":world_seed,"organism_id":id.raw(),"policy_version":policy,
            "world_tick":start_tick,"world_ticks_per_second":alife_world::WORLD_TICKS_PER_SECOND,
            "hardware":hardware,"sampling_stream":if sampler_restored {"restored_exact_private_stream"} else {"declared_new_legacy_stream"},
            "sampling_temperature":sampling.temperature,"sampling_seed":sampling.seed,"sampling_counter":sampling.counter,
            "actor_optimizer_step":trainer.optimizer_step(),"value_optimizer_step":value.checkpoint(trainer.session())?.optimizer_step,
        }),
    )?;
    while trained_rows < request.total_trained_decisions as u64 {
        if now()? >= request.collection_cutoff_unix_seconds {
            stop = "collection_cutoff";
            break;
        }
        if now()? >= request.finalization_deadline_unix_seconds {
            return Err("native finalization deadline reached before snapshot".into());
        }
        let wanted = request
            .decisions_per_update
            .min((request.total_trained_decisions as u64 - trained_rows) as usize);
        let window_root = work.join(format!("window-{completed_updates:08}"));
        let phenotype = alife_core::PhenotypeCompiler::compile_validated(
            runtime.live_foundation_compiler_inputs(id)?,
            &alife_core::BrainCapacityClass::n2048(),
        )?;
        let actor_before = trainer.optimizer_step();
        let value_before = value.checkpoint(trainer.session())?.optimizer_step;
        let source = foundation_replay_source(
            &phenotype,
            &asset,
            policy,
            &serde_json::to_vec(&trainer.checkpoint()?)?,
        )?;
        let budget = FoundationReplayBudget::default();
        let mut cohort =
            crate::foundation_training::CohortReplayWriter::new(&window_root, source.clone())?;
        let mut writer = FoundationReplayWriter::new(
            &window_root.join("individual"),
            source.clone(),
            phenotype.clone(),
            &asset,
            budget,
        )?;
        let mut refs: Vec<crate::FoundationReplayRecordRef> = Vec::new();
        let mut features = Vec::new();
        let mut generic_rows = Vec::new();
        let mut meal_rows = Vec::new();
        for object in runtime.world().object_snapshots() {
            if object.kind == alife_world::WorldObjectKind::Food {
                food.entry(object.id)
                    .or_insert_with(|| crate::FoundationFoodOutcome::new(id, object.id));
            }
        }
        crate::foundation_training::prime_foundation_lesson_prior(&mut runtime, None)?;
        let collection = Instant::now();
        let first_tick = runtime.world().tick().raw();
        let capture_before = runtime.foundation_training_capture_metrics();
        let mut no_progress = 0;
        let mut stalled = Instant::now();
        let mut first_control_call = true;
        loop {
            if now()? >= request.collection_cutoff_unix_seconds {
                stop = "collection_cutoff";
                break;
            }
            if runtime.world().tick().raw().saturating_sub(first_tick) >= tick_limit(wanted)? {
                stop = "world_tick_guard";
                break;
            }
            let before_tick = runtime.world().tick();
            let before_food = food
                .keys()
                .map(|target| {
                    (
                        *target,
                        crate::foundation_training::food_outcome::ownership(
                            runtime.world(),
                            *target,
                        ),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            if !first_control_call {
                crate::foundation_training::prime_foundation_lesson_prior(&mut runtime, None)?;
            }
            first_control_call = false;
            runtime.tick_outcome()?;
            let mut steps = runtime.take_foundation_training_steps();
            if runtime.world().tick() == before_tick {
                if !steps.is_empty() {
                    return Err("uncommitted world tick returned training rows".into());
                }
                no_progress += 1;
                if stalled.elapsed().as_secs() > 300 {
                    return Err("same-world runtime made no progress for five minutes".into());
                }
                continue;
            }
            if runtime.world().tick().raw()
                != before_tick
                    .raw()
                    .checked_add(1)
                    .ok_or("world tick overflow")?
                || steps.len() > 1
            {
                return Err("same-individual collector observed unexpected tick/population".into());
            }
            stalled = Instant::now();
            if !runtime.last_memory_preparation_errors().is_empty()
                || !runtime.last_post_seal_learning_failures().is_empty()
            {
                return Err("canonical memory preparation or sealed learning was rejected".into());
            }
            if let Some(step) = steps.pop() {
                if step.frame.organism_id() != id || step.frame.tick() != before_tick {
                    return Err("continuous row belongs to a different individual/tick".into());
                }
                let gap = refs
                    .last()
                    .is_some_and(|r| r.tick + 1 != step.frame.tick().raw());
                if refs.is_empty() || gap {
                    verify_foundation_replay_step(&mut trainer, &step)?;
                }
                if gap {
                    writer.start_segment()?;
                }
                if consumed(&step) {
                    generic_rows.push(refs.len());
                }
                for (target, summary) in &mut food {
                    let observation =
                        crate::foundation_training::food_outcome::FoodOutcomeObservation {
                            organism: id,
                            target: *target,
                            decision_tick: before_tick,
                            outcome_tick: runtime.world().tick(),
                            before: before_food[target],
                            after: crate::foundation_training::food_outcome::ownership(
                                runtime.world(),
                                *target,
                            ),
                        };
                    if !observation.matches_step(&step) {
                        return Err("continuous food/row identity mismatch".into());
                    }
                    let old = summary.nourished_meal_events;
                    summary.observe(observation, &step.patch);
                    if summary.nourished_meal_events > old {
                        meal_rows.push(refs.len());
                    }
                }
                features.push(crate::foundation_training::snapshot_activation(
                    &step.after_inference,
                )?);
                refs.push(writer.append_cohort(&step, &mut cohort)?);
            }
            if !alive(&runtime, id) {
                stop = "natural_death";
                terminal_death_tick = runtime
                    .world()
                    .organism_registry()
                    .get(id)
                    .and_then(|r| r.lifecycle().death_tick())
                    .map(|t| t.raw())
                    .or_else(|| {
                        runtime
                            .archive_retirement_receipt(id)
                            .map(|r| r.death_tick.raw())
                    });
                break;
            }
            if refs.len() >= wanted + 1 {
                break;
            }
        }
        let causal = FoundationCausalCollectionMetrics::measure(
            first_tick,
            runtime.world().tick().raw(),
            collection.elapsed().as_secs_f64(),
            refs.len(),
            no_progress,
            capture_before,
            runtime.foundation_training_capture_metrics(),
        )?;
        collection_seconds += causal.collection_wall_seconds;
        no_progress_total += no_progress;
        captured_total += refs.len() as u64;
        generic_total += generic_rows.len() as u64;
        nourished_total += meal_rows.len() as u64;
        // A dead body cannot be installed/saved as a living Current. Keep the
        // last valid snapshot and report this entire unretained suffix explicitly.
        if terminal_death_tick.is_some() || !alive(&runtime, id) {
            drop(writer);
            drop(cohort);
            alife_storage::remove_dir_all(&window_root)?;
            on_update(
                serde_json::json!({"kind":"continuous-terminal-v1", "session_nonce":session_nonce,
                "terminal":true,"terminal_death_tick":terminal_death_tick,"causal_collection":causal,
                "captured_rows":refs.len(),"trained_rows":0,"semantic_prior":compact_prior(&runtime)}),
            )?;
            break;
        }
        let rows = loss_rows(refs.len(), wanted);
        if rows == 0 {
            drop(writer);
            drop(cohort);
            alife_storage::remove_dir_all(&window_root)?;
            break;
        }
        let sealing = Instant::now();
        let replay = cohort.seal()?;
        drop(writer);
        replay_seal_seconds += sealing.elapsed().as_secs_f64();
        // This only drains persistence workers; it does not advance world,
        // cognition, physiology or the exploration stream.
        let waiting = Instant::now();
        runtime.quiesce_live_foundation_checkpoint(request.finalization_deadline_unix_seconds)?;
        boundary_wait_seconds += waiting.elapsed().as_secs_f64();
        if runtime.foundation_training_policy_install_blocked_by_sleep(id)? {
            // Retain the actual sleeping individual's exact state with its OLD
            // global heads/Adam. No wake, reset, clock advance or PPO/install is
            // invented to make a deadline. All this window's rows are untrained.
            stop = "sleep_consolidation_install_deferred";
            alife_storage::remove_dir_all(&window_root)?;
            on_update(serde_json::json!({"kind":"continuous-untrained-window-v1",
                "session_nonce":session_nonce,"captured_rows":refs.len(),"trained_rows":0,
                "stop_reason":stop,"causal_collection":causal,
                "generic_consumed_events":generic_rows.len(),"exact_nourished_meals":meal_rows.len(),
                "policy_version":policy,"semantic_prior":compact_prior(&runtime)}))?;
            break;
        }
        runtime.ensure_live_foundation_policy_boundary(id)?;
        let ppo_started = Instant::now();
        let mut values = Vec::with_capacity(refs.len());
        let mut rewards: Vec<(PpoJointAction, f32)> = Vec::with_capacity(rows);
        let mut previous_after = None;
        let mut segment_start = 0;
        while segment_start < refs.len() {
            let segment = refs[segment_start].segment;
            let end = refs[segment_start..]
                .iter()
                .position(|r| r.segment != segment)
                .map_or(refs.len(), |offset| segment_start + offset);
            for (chunk, slice) in refs[segment_start..end].chunks(512).enumerate() {
                let window = load_foundation_replay_window(
                    &window_root,
                    slice,
                    0,
                    &source,
                    &phenotype,
                    budget,
                )?;
                values.extend(value.predict_values(&mut trainer, &window.sequence)?);
                for (row, (behavior, patch)) in
                    window.behavior.iter().zip(&window.patches).enumerate()
                {
                    let index = segment_start + chunk * 512 + row;
                    let physiology = patch
                        .outcome()
                        .measured_physiology
                        .ok_or("continuous patch lacks measured physiology")?;
                    if index > 0 && refs[index].tick > refs[index - 1].tick + 1 {
                        let gap = alife_core::MeasuredPhysiologyTransition::new(
                            previous_after.ok_or("missing pre-sleep biology")?,
                            physiology.before,
                        )?;
                        let seconds = (refs[index].tick - refs[index - 1].tick - 1) as f64
                            / f64::from(alife_world::WORLD_TICKS_PER_SECOND);
                        let discount = (-std::f64::consts::LN_2 * seconds
                            / f64::from(ppo.discount_half_life_seconds))
                        .exp() as f32;
                        let delayed = training_biological_value(&gap, &body)? * discount;
                        rewards[index - 1].1 = (rewards[index - 1].1 + delayed).clamp(-1.0, 1.0);
                    }
                    previous_after = Some(physiology.after);
                    if index == rows {
                        break;
                    }
                    let receptors = physiology.before.neural_receptor_frame(&body)?;
                    let modulator = alife_core::OutcomeCreditPacket::from_sealed_patch(patch)?
                        .with_biochemical_receptors(&receptors)?
                        .modulator();
                    rewards.push((
                        joint_action(behavior)?,
                        training_receptor_profile(&body).project(&modulator.frame())?,
                    ));
                }
            }
            segment_start = end;
        }
        if values.len() != refs.len() || rewards.len() != rows {
            return Err("continuous target/capture length mismatch".into());
        }
        let transitions = rewards
            .into_iter()
            .enumerate()
            .map(|(i, (action, reward))| PpoTransition {
                policy_version: policy,
                trajectory_id: session_nonce,
                step: i as u64,
                action,
                reward,
                old_value: values[i],
                next_value: values[i + 1],
                elapsed_seconds: (refs[i + 1].tick - refs[i].tick) as f32
                    / alife_world::WORLD_TICKS_PER_SECOND as f32,
                boundary: if i + 1 == rows {
                    PpoBoundary::Truncated
                } else {
                    PpoBoundary::Continuing
                },
            })
            .collect();
        let batch = PpoBatch::from_rollout(policy, transitions, ppo)?;
        let mut spans = Vec::new();
        let mut start = 0;
        while start < rows {
            let segment = refs[start].segment;
            let end = refs[start..rows]
                .iter()
                .position(|r| r.segment != segment)
                .map_or(rows, |n| start + n);
            for loss_start in (start..end).step_by(256) {
                spans.push((
                    loss_start.saturating_sub(128).max(start),
                    loss_start,
                    (loss_start + 256).min(end),
                ));
            }
            start = end;
        }
        let update = train_recurrent_ppo_cohort(
            &mut trainer,
            &mut value,
            spans.len(),
            8,
            |index| {
                let (burn, start, end) = spans[index];
                let replay = load_foundation_replay_window(
                    &window_root,
                    &refs[burn..end],
                    start - burn,
                    &source,
                    &phenotype,
                    budget,
                )
                .map_err(|_| {
                    alife_training::TrainingError::from(
                        ScaffoldContractError::InvalidDecisionEvidence,
                    )
                })?;
                Ok(PpoTrainingWindow {
                    speech_targets: vec![None; replay.sequence.ticks.len()],
                    sequence: replay.sequence,
                    batch: batch.window(start..end)?,
                    auxiliary: None,
                })
            },
            PpoConfig {
                value_coefficient: 0.0,
                ..ppo
            },
            policy,
        )?;
        let actor_updates = update
            .actor_optimizer_step
            .checked_sub(actor_before)
            .ok_or("actor age regressed")?;
        // Zero accepted actor updates is not a new policy. No manufactured critic
        // update or policy age is used to satisfy a progress counter.
        if actor_updates == 0 {
            return Err(
                "continuous PPO accepted no actor update; recent live work remains unsnapshotted"
                    .into(),
            );
        }
        let next_policy = policy.checked_add(1).ok_or("policy version overflow")?;
        let (critic, _) = critic_residual::train(
            trainer.session(),
            value.checkpoint(trainer.session())?,
            &features,
            batch.transitions(),
            ppo,
            actor_updates,
            next_policy,
        )?;
        value = PpoTrainingState::from_checkpoint(critic)?;
        let ppo_seconds = ppo_started.elapsed().as_secs_f64();
        ppo_seconds_total += ppo_seconds;
        let install_started = Instant::now();
        // Curriculum metadata is independent of unbounded u64 policy updates.
        let trained = trainer.export_candidate(asset.manifest().training_stage())?;
        let transition = N2048LivePolicyTransition::prepare(
            runtime.live_foundation_compiler_inputs(id)?,
            policy,
            trained.clone(),
        )?;
        let install = runtime.install_live_foundation_policy(id, &transition)?;
        trainer.rebind_for_next_cohort(transition.next_phenotype().clone(), trained.clone())?;
        asset = trained;
        policy = next_policy;
        trained_rows = trained_rows
            .checked_add(rows as u64)
            .ok_or("trained-row overflow")?;
        completed_updates += 1;
        let value_age = value.checkpoint(trainer.session())?.optimizer_step;
        let install_seconds = install_started.elapsed().as_secs_f64();
        install_seconds_total += install_seconds;
        let event = serde_json::json!({"kind":"continuous-update-v1","session_nonce":session_nonce,
            "source_world_seed":world_seed,"organism_id":id.raw(),"update":completed_updates,
            "policy_before":policy-1,"policy_after":policy,"trained_rows":rows,
            "session_trained_rows":trained_rows,"captured_replay_rows":refs.len(),"bootstrap_replay_rows":1,
            "actor_optimizer_step":trainer.optimizer_step(),"value_optimizer_step":value_age,
            "actor_optimizer_updates":actor_updates,"value_optimizer_updates":value_age.checked_sub(value_before),
            "completed_epochs":update.completed_epochs,"stopped_for_kl":update.stopped_for_kl,
            "causal_collection":causal,"ppo_seconds":ppo_seconds,"install_seconds":install_seconds,
            "mutable_words_written_during_install":install.mutable_words_written,
            "generic_consumed_events":generic_rows.len(),"trained_generic_consumed_events":generic_rows.iter().filter(|r| **r<rows).count(),
            "exact_nourished_meals":meal_rows.len(),"trained_exact_nourished_meals":meal_rows.iter().filter(|r| **r<rows).count(),
            "food_evidence_scope":"exact existing food entities observed at this window start; regrowth of same entity is ordinary ecology",
            "observed_food_entity_count":food.len(),"semantic_prior":compact_prior(&runtime),
            "replay_bundle_bytes":replay.bundle_bytes,"replay_data_sync_calls":replay.data_sync_calls,
        });
        alife_storage::remove_dir_all(&window_root)?;
        on_update(event)?;
        if completed_updates % request.snapshot_every_updates as u64 == 0 {
            if now()? >= request.finalization_deadline_unix_seconds {
                return Err("deadline before periodic snapshot".into());
            }
            let snapshot = Instant::now();
            runtime
                .quiesce_live_foundation_checkpoint(request.finalization_deadline_unix_seconds)?;
            let exact = runtime.capture_portable_checkpoint()?;
            GpuDurableSaveManifest::publish_snapshot(&save_path, &assets, &exact)?;
            runtime.rebind_durable_checkpoint_boundary(&save_path, &assets, &exact)?;
            on_snapshot(
                handoff(&runtime, id, &trainer, &value, policy)?,
                &life,
                SameIndividualProgressV1 {
                    session_nonce,
                    organism_id: id.raw(),
                    session_start_world_tick: start_tick,
                    world_tick: runtime.world().tick().raw(),
                    trained_rows,
                    completed_updates,
                    sampling_temperature: sampling.temperature,
                },
            )?;
            snapshot_seconds += snapshot.elapsed().as_secs_f64();
            saved_rows = trained_rows;
            saved_updates = completed_updates;
            saved_tick = runtime.world().tick().raw();
            saved_policy = policy;
        }
        if stop != "decision_horizon" {
            break;
        }
    }
    if alive(&runtime, id) && now()? < request.finalization_deadline_unix_seconds {
        let snapshot = Instant::now();
        runtime.quiesce_live_foundation_checkpoint(request.finalization_deadline_unix_seconds)?;
        let exact = runtime.capture_portable_checkpoint()?;
        GpuDurableSaveManifest::publish_snapshot(&save_path, &assets, &exact)?;
        runtime.rebind_durable_checkpoint_boundary(&save_path, &assets, &exact)?;
        on_snapshot(
            handoff(&runtime, id, &trainer, &value, policy)?,
            &life,
            SameIndividualProgressV1 {
                session_nonce,
                organism_id: id.raw(),
                session_start_world_tick: start_tick,
                world_tick: runtime.world().tick().raw(),
                trained_rows,
                completed_updates,
                sampling_temperature: sampling.temperature,
            },
        )?;
        snapshot_seconds += snapshot.elapsed().as_secs_f64();
        saved_rows = trained_rows;
        saved_updates = completed_updates;
        saved_tick = runtime.world().tick().raw();
        saved_policy = policy;
    } else if alive(&runtime, id) {
        stop = "finalization_deadline_unsaved_suffix";
    }
    let causal_collection = FoundationCausalCollectionMetrics::measure(
        start_tick,
        runtime.world().tick().raw(),
        collection_seconds,
        usize::try_from(captured_total)?,
        no_progress_total,
        initial_capture_metrics,
        runtime.foundation_training_capture_metrics(),
    )?;
    Ok(
        serde_json::json!({"schema":1,"kind":"continuous-world-finished-v1", "session_nonce":session_nonce,
            "source_world_seed":world_seed,"organism_id":id.raw(),"world_ticks_per_second":alife_world::WORLD_TICKS_PER_SECOND,
            "hardware":hardware,
            "session_start_world_tick":start_tick,"world_tick":runtime.world().tick().raw(),
            "unique_world_ticks":runtime.world().tick().raw().checked_sub(start_tick),
            "policy_version":policy,"actor_optimizer_step":trainer.optimizer_step(),"value_optimizer_step":value.checkpoint(trainer.session())?.optimizer_step,
            "trained_rows":trained_rows,"completed_updates":completed_updates,"captured_rows":captured_total,
            "untrained_captured_rows_including_bootstrap":captured_total-trained_rows,
            "saved_trained_rows":saved_rows,"saved_completed_updates":saved_updates,"saved_world_tick":saved_tick,"saved_policy_version":saved_policy,
            "unsaved_trained_rows":trained_rows-saved_rows,"unsaved_world_ticks":runtime.world().tick().raw().saturating_sub(saved_tick),
            "stop_reason":stop,"terminal":!alive(&runtime,id),"terminal_death_tick":terminal_death_tick,
            "sampling_stream":if sampler_restored {"restored_exact_private_stream"} else {"declared_new_legacy_stream"},
            "sampling_temperature":sampling.temperature,"generic_consumed_events":generic_total,"exact_nourished_meals":nourished_total,
            "semantic_prior":compact_prior(&runtime),"snapshot_seconds":snapshot_seconds,"producer_seconds":started.elapsed().as_secs_f64(),
            "collection_seconds":collection_seconds,"ppo_seconds":ppo_seconds_total,"install_seconds":install_seconds_total,
            "restore_seconds":restore_seconds,"replay_seal_seconds":replay_seal_seconds,"boundary_wait_seconds":boundary_wait_seconds,
            "causal_collection":causal_collection,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn continuous_deadline_uses_only_actual_next_state() {
        assert_eq!(loss_rows(0, 512), 0);
        assert_eq!(loss_rows(1, 512), 0);
        assert_eq!(loss_rows(11, 512), 10);
        assert_eq!(loss_rows(513, 512), 512);
    }
    #[test]
    fn continuous_tick_guard_is_relative_to_restored_world() {
        assert_eq!(tick_limit(512).unwrap(), 8192);
        let restored_tick = 1_000_000u64;
        assert!((restored_tick + 8191).saturating_sub(restored_tick) < tick_limit(512).unwrap());
        assert!((restored_tick + 8192).saturating_sub(restored_tick) >= tick_limit(512).unwrap());
    }
}
