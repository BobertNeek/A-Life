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

/// Create only the producer's NEW direct child of its already-owned .run.
/// Native registered the aggregate budget before dispatch; materialization's
/// own create-new directory is one level below this working directory.
fn prepare_live_work_directory(work: &Path) -> Result<std::path::PathBuf> {
    let parent = work
        .parent()
        .ok_or("continuous work lacks an owned parent")?;
    if work.file_name() != Some(std::ffi::OsStr::new("live"))
        || parent.file_name() != Some(std::ffi::OsStr::new(".run"))
    {
        return Err("continuous work must be the owned .run/live child".into());
    }
    let metadata = std::fs::symlink_metadata(parent)?;
    #[cfg(windows)]
    let alias = {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    };
    #[cfg(not(windows))]
    let alias = metadata.file_type().is_symlink();
    if !metadata.is_dir() || alias {
        return Err("continuous work parent must be a real owned directory".into());
    }
    match std::fs::symlink_metadata(work) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("continuous work already exists; preserving it".into()),
    }
    let budget = alife_storage::budget_for(parent)?
        .ok_or("continuous work requires the Native aggregate output budget")?;
    if alife_storage::budget_for(work)?.as_ref() != Some(&budget) {
        return Err("continuous work is outside its parent's output budget".into());
    }
    // No recursive creation, overwrite, predecessor cleanup or alternate root.
    std::fs::create_dir(work)?;
    Ok(work.join("live"))
}

/// A partial live rollout always keeps its actual last action as bootstrap.
/// No clock advance or fictitious next state is manufactured at a deadline.
fn loss_rows(captured: usize, requested: usize) -> usize {
    captured.saturating_sub(1).min(requested)
}

// The replay describes the currently installed policy; the PPO round is the
// next version, matching the serial trainer's monotonic update contract.
fn continuous_update_version(policy: u64, value_version: Option<u64>) -> Result<u64> {
    if value_version != Some(policy) {
        return Err(format!(
            "continuous value version mismatch: policy {policy}, value {value_version:?}"
        )
        .into());
    }
    policy
        .checked_add(1)
        .ok_or_else(|| "policy version overflow".into())
}

/// Passive bounded observations. Distance is geometric, not a path-reachability claim.
fn care_world_observation(world: &alife_world::HeadlessWorld, id: OrganismId) -> serde_json::Value {
    let Some(record) = world.organism_registry().get(id) else {
        return serde_json::json!({"organism_present": false});
    };
    let biology = record.biochemistry();
    let position = world
        .entity(record.world_entity_id())
        .map(|entity| entity.position);
    let food: Vec<_> = world
        .object_snapshots()
        .into_iter()
        .filter(|object| object.kind == alife_world::WorldObjectKind::Food)
        .map(|object| {
            let distance = position.map(|p| {
                let dx = p.x - object.position.x;
                let dy = p.y - object.position.y;
                let dz = p.z - object.position.z;
                (dx * dx + dy * dy + dz * dz).sqrt()
            });
            serde_json::json!({"id":object.id.raw(), "consumed":object.consumed,
                "carried_by":object.carried_by.map(|owner| owner.raw()),
                "nutrition":object.nutrition,"hazard_pain":object.hazard_pain,
                "geometric_distance":distance})
        })
        .collect();
    serde_json::json!({"organism_present":true,"world_tick":world.tick().raw(),
        "energy":biology.body.energy,"health":biology.body.health,"injury":biology.body.injury,
        "sleeping":biology.body.sleeping,"hunger":biology.homeostasis.drives.hunger,
        "fatigue":biology.homeostasis.drives.fatigue,"pain":biology.homeostasis.drives.pain,
        "food":food})
}

#[derive(Default)]
struct CareRowEvidence {
    food_candidate: bool,
    food_approach: bool,
    food_eat: bool,
    food_grab: bool,
    bundle_missing: bool,
    teacher_food_token_heard: bool,
}

fn care_row_evidence(
    step: &crate::FoundationTrainingStep,
    food: &BTreeMap<u64, (alife_core::WorldEntityId, crate::FoundationFoodOutcome)>,
) -> CareRowEvidence {
    let mut evidence = CareRowEvidence {
        food_candidate: step.frame.candidates().iter().any(|candidate| {
            candidate
                .target
                .entity
                .is_some_and(|target| food.contains_key(&target.raw()))
        }),
        bundle_missing: step.patch.selected_bundle().is_none(),
        teacher_food_token_heard: step
            .frame
            .sensory()
            .language_context
            .heard_tokens
            .iter()
            .flatten()
            .any(|heard| {
                heard.source_kind == alife_core::UtteranceSourceKind::Teacher
                    && heard.addressee == Some(step.frame.organism_id())
                    && matches!(heard.token_id, 1 | 8)
            }),
        ..CareRowEvidence::default()
    };
    if let Some(bundle) = step.patch.selected_bundle() {
        for command in &bundle.channels {
            let targets_food = command
                .target
                .and_then(|target| target.entity)
                .is_some_and(|target| food.contains_key(&target.raw()));
            if targets_food {
                evidence.food_approach |=
                    command.primitive == alife_world::HeadlessActionIds::APPROACH;
                evidence.food_eat |= command.primitive == alife_world::HeadlessActionIds::EAT;
                evidence.food_grab |= command.primitive == alife_world::HeadlessActionIds::GRAB;
            }
        }
    }
    evidence
}

fn care_row_summary(rows: &[CareRowEvidence]) -> serde_json::Value {
    serde_json::json!({"trained_rows":rows.len(),
        "food_candidate_rows":rows.iter().filter(|row| row.food_candidate).count(),
        "food_approach_command_rows":rows.iter().filter(|row| row.food_approach).count(),
        "food_eat_command_rows":rows.iter().filter(|row| row.food_eat).count(),
        "food_grab_command_rows":rows.iter().filter(|row| row.food_grab).count(),
        "missing_bundle_rows":rows.iter().filter(|row| row.bundle_missing).count(),
        "teacher_food_token_heard_rows":rows.iter().filter(|row| row.teacher_food_token_heard).count(),
        "scope":"candidate presence and selected commands, not visibility, reachability or execution"})
}

fn care_food_ready(food: &alife_world::WorldObject) -> bool {
    food.kind == alife_world::WorldObjectKind::Food
        && !food.consumed
        && food.carried_by.is_none()
        && food.nutrition > 0.0
        && food.hazard_pain == 0.0
}

/// Inspect a disposable copy so checking assistance never advances tracking in
/// the live world or invents a target that the learner cannot perceive.
fn care_food_is_offered(
    world: &alife_world::HeadlessWorld,
    id: OrganismId,
    food: alife_core::WorldEntityId,
) -> bool {
    let mut trial = world.clone();
    care_food_is_offered_in_trial(&mut trial, id, food)
}

fn care_food_is_offered_in_trial(
    trial: &mut alife_world::HeadlessWorld,
    id: OrganismId,
    food: alife_core::WorldEntityId,
) -> bool {
    let Some(record) = trial.organism_registry().get(id) else {
        return false;
    };
    let homeostasis = record.biochemistry().homeostasis;
    let tick = trial.tick();
    trial
        .perception_frame(
            id,
            tick,
            alife_core::SensorProfile::GroundedTerrainVisionV1,
            homeostasis,
        )
        .is_ok_and(|frame| {
            frame.candidates().iter().any(|candidate| {
                candidate.action_id == alife_world::HeadlessActionIds::EAT
                    && candidate.target.entity == Some(food)
            })
        })
}

fn care_food_position_in_gaze(
    world: &alife_world::HeadlessWorld,
    id: OrganismId,
) -> Option<alife_core::Vec3f> {
    let record = world.organism_registry().get(id)?;
    let body = world.entity(record.world_entity_id())?;
    let gaze = body.body_yaw + body.head_yaw;
    let food = world.entity_id("food-01")?;
    if !world.entity(food).is_some_and(care_food_ready) {
        return None;
    }
    // Try real placements in the current gaze before falling back to contact.
    // Check the actual grounded frame on a disposable world clone: a nominally
    // visible target can be occluded or displaced from the three action slots.
    for distance in [1.0_f32, 0.5, 0.1, 0.0] {
        for offset in [0.0_f32, 0.35, -0.35, 0.7, -0.7] {
            let yaw = gaze + offset;
            let wanted = alife_core::Vec3f::new(
                body.position.x + distance * yaw.cos(),
                body.position.y,
                body.position.z + distance * yaw.sin(),
            );
            let position = match world.terrain() {
                Some(terrain) => match terrain.resolve_move(body.position, wanted) {
                    Some(position) => position,
                    None => continue,
                },
                None => wanted,
            };
            let dx = position.x - body.position.x;
            let dz = position.z - body.position.z;
            let length_squared = dx * dx + dz * dz;
            if world.object_snapshots().iter().any(|object| {
                if object.kind != alife_world::WorldObjectKind::Obstacle || object.consumed {
                    return false;
                }
                let t = if length_squared > 0.0 {
                    (((object.position.x - body.position.x) * dx
                        + (object.position.z - body.position.z) * dz)
                        / length_squared)
                        .clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let closest_x = body.position.x + dx * t;
                let closest_z = body.position.z + dz * t;
                (object.position.x - closest_x).hypot(object.position.z - closest_z)
                    <= object.radius
            }) {
                continue;
            }
            let mut trial = world.clone();
            if !matches!(
                crate::foundation_training::relocate_unheld_foundation_food(
                    &mut trial, food, position,
                ),
                Ok(true)
            ) {
                continue;
            }
            if care_food_is_offered_in_trial(&mut trial, id, food) {
                return Some(position);
            }
        }
    }
    None
}

struct ContinuousCareAssistance {
    food: alife_core::WorldEntityId,
    start_tick: u64,
}

fn prepare_care_assistance(
    world: &mut alife_world::HeadlessWorld,
    id: OrganismId,
) -> Result<ContinuousCareAssistance> {
    let position =
        care_food_position_in_gaze(world, id).ok_or("no walkable care placement in gaze")?;
    let food = world.entity_id("food-01").ok_or("care food missing")?;
    if !world.entity(food).is_some_and(care_food_ready)
        || !crate::foundation_training::relocate_unheld_foundation_food(world, food, position)?
    {
        return Err("care food is unavailable for placement".into());
    }
    Ok(ContinuousCareAssistance {
        food,
        start_tick: world.tick().raw(),
    })
}

fn expose_care_cue(
    world: &mut alife_world::HeadlessWorld,
    id: OrganismId,
    care: &ContinuousCareAssistance,
) -> Result<bool> {
    let position = world
        .entity(care.food)
        .ok_or("care cue food missing")?
        .position;
    let teacher_position = [(0.0_f32, 2.0_f32), (2.0, 0.0), (-2.0, 0.0), (0.0, -2.0)]
        .into_iter()
        .map(|(x, z)| alife_core::Vec3f::new(position.x + x, position.y, position.z + z))
        .find(|candidate| {
            world
                .terrain()
                .is_none_or(|terrain| terrain.walkable(candidate.x, candidate.z))
        });
    let Some(teacher_position) = teacher_position else {
        return Ok(false);
    };
    alife_school::LanguageNursery::speak_in_world(
        world,
        id,
        teacher_position,
        [8, 1]
            .into_iter()
            .map(alife_core::LanguageTokenId::new)
            .collect::<std::result::Result<Vec<_>, _>>()?,
    )?;
    Ok(true)
}

fn care_cue_due(tick: u64, start: u64, available: bool, last: Option<u64>) -> bool {
    available && tick.saturating_sub(start).is_multiple_of(24) && last != Some(tick)
}

fn care_food_is_far(
    world: &alife_world::HeadlessWorld,
    id: OrganismId,
    food: alife_core::WorldEntityId,
) -> bool {
    let Some(subject) = world
        .organism_registry()
        .get(id)
        .and_then(|record| world.entity(record.world_entity_id()))
    else {
        return false;
    };
    let Some(target) = world.entity(food).filter(|food| care_food_ready(food)) else {
        return false;
    };
    (subject.position.x - target.position.x).hypot(subject.position.z - target.position.z) > 4.0
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

/// Record the first observed terminal boundary independently of later replay
/// metrics or cleanup. The store binds this minimal event exactly once.
pub(super) fn observed_terminal_event(
    runtime: &GpuLiveBrainRuntime,
    id: OrganismId,
    session_nonce: u64,
    world_seed: u64,
) -> Option<serde_json::Value> {
    if alive(runtime, id) {
        return None;
    }
    let death_tick = runtime
        .world()
        .organism_registry()
        .get(id)
        .and_then(|record| record.lifecycle().death_tick())
        .map(|tick| tick.raw())
        .or_else(|| {
            runtime
                .archive_retirement_receipt(id)
                .map(|receipt| receipt.death_tick.raw())
        });
    let final_biology = runtime.foundation_terminal_biology(id).or_else(|| {
        runtime
            .world()
            .organism_registry()
            .get(id)
            .map(|record| *record.biochemistry())
    });
    let final_biology = final_biology.map(|biology| {
        serde_json::json!({
            "tick":biology.tick.raw(), "energy":biology.body.energy, "health":biology.body.health,
            "injury":biology.body.injury, "hunger":biology.homeostasis.drives.hunger,
            "fatigue":biology.homeostasis.drives.fatigue, "pain":biology.homeostasis.drives.pain,
        })
    });
    Some(
        serde_json::json!({"kind":"continuous-terminal-v1","terminal":true,
        "session_nonce":session_nonce,"source_world_seed":world_seed,"organism_id":id.raw(),
        "world_tick":runtime.world().tick().raw(),"world_ticks_per_second":alife_world::WORLD_TICKS_PER_SECOND,
        "terminal_death_tick":death_tick,"final_biology":final_biology}),
    )
}

pub(super) fn record_terminal_then<T>(
    event: serde_json::Value,
    on_update: &mut impl FnMut(serde_json::Value) -> Result<()>,
    postprocess: impl FnOnce() -> Result<T>,
) -> Result<T> {
    on_update(event)?;
    postprocess()
}

fn compact_prior(runtime: &GpuLiveBrainRuntime, teacher_cues_emitted: u64) -> serde_json::Value {
    runtime.semantic_prior_metrics().map_or(serde_json::Value::Null, |m| serde_json::json!({
        "backend":m.backend,"provider_identity":m.provider_identity,
        "decision_frames":m.decision_frames,"decision_frames_with_prior":m.decision_frames_with_prior,
        "decision_frames_without_prior":m.decision_frames_without_prior,
        "delivered_frames":m.delivered_frames,"failures":m.failures,
        "last_error":m.last_error,"teacher_cues_injected":teacher_cues_emitted,
        "teacher_cues_emitted":teacher_cues_emitted,
    }))
}

/// A legacy private life can be captured during an old-policy sleep transaction.
/// Finish that transaction through ordinary world ticks, never by clearing its
/// scheduler or installing the newer global heads into a pending request.
pub(crate) fn advance_legacy_policy_sleep(
    runtime: &mut GpuLiveBrainRuntime,
    id: OrganismId,
    session_nonce: u64,
    world_seed: u64,
    deadline: u64,
    on_update: &mut impl FnMut(serde_json::Value) -> Result<()>,
) -> Result<serde_json::Value> {
    let began = Instant::now();
    let first_tick = runtime.world().tick().raw();
    let mut captured_rows = 0u64;
    let mut no_progress_calls = 0u64;
    let mut last_progress = Instant::now();
    runtime.quiesce_live_foundation_checkpoint(deadline)?;
    while runtime.foundation_training_policy_install_blocked_by_sleep(id)? {
        if now()? >= deadline {
            return Err("legacy sleep did not reach an install boundary before collection cutoff; old state was not reset".into());
        }
        let before = runtime.world().tick();
        crate::foundation_training::prime_foundation_lesson_prior(runtime, None)?;
        let tick = runtime.tick_outcome();
        // Even a later runtime error must not hide an already observed death.
        if let Some(event) = observed_terminal_event(runtime, id, session_nonce, world_seed) {
            on_update(event)?;
            return Err(
                "legacy individual died while naturally completing old-policy sleep".into(),
            );
        }
        tick?;
        let steps = runtime.take_foundation_training_steps();
        let after = runtime.world().tick();
        if after == before {
            if !steps.is_empty() {
                return Err(
                    "legacy sleep returned training rows without a committed world tick".into(),
                );
            }
            no_progress_calls += 1;
            if last_progress.elapsed().as_secs() > 300 {
                return Err("legacy sleep made no world progress for five minutes".into());
            }
            continue;
        }
        if after.raw() != before.raw().checked_add(1).ok_or("world tick overflow")?
            || steps.len() > 1
            || steps
                .iter()
                .any(|s| s.frame.organism_id() != id || s.frame.tick() != before)
            || !runtime.last_memory_preparation_errors().is_empty()
            || !runtime.last_post_seal_learning_failures().is_empty()
        {
            return Err(
                "legacy sleep progression violated the ordinary sealed-tick contract".into(),
            );
        }
        // Ordinary lifetime learning has already occurred. These old-policy
        // rows are deliberately not fed to PPO under the newer accepted heads.
        captured_rows += steps.len() as u64;
        last_progress = Instant::now();
    }
    runtime.quiesce_live_foundation_policy_boundary(id, deadline)?;
    let event = serde_json::json!({"kind":"continuous-legacy-sleep-progress-v1",
        "session_nonce":session_nonce,"source_world_seed":world_seed,"organism_id":id.raw(),
        "first_world_tick":first_tick,"world_tick":runtime.world().tick().raw(),
        "unique_world_ticks":runtime.world().tick().raw().checked_sub(first_tick),
        "captured_old_policy_rows_not_trained":captured_rows,"ppo_updates":0,
        "no_progress_control_calls":no_progress_calls,"wall_seconds":began.elapsed().as_secs_f64(),
        "sleep_reset":false,"forced_wake":false});
    on_update(event.clone())?;
    Ok(event)
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
    request: &super::ResidentContinuousRequest,
    work: &Path,
    mut on_snapshot: impl FnMut(
        FoundationTrainingHandoffV1,
        &Path,
        SameIndividualProgressV1,
    ) -> Result<()>,
    mut on_update: impl FnMut(serde_json::Value) -> Result<()>,
) -> Result<serde_json::Value> {
    baseline.validate()?;
    baseline.require_living_continuation()?;
    let started = Instant::now();
    let life = prepare_live_work_directory(work)?;
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
    let legacy_sleep_progress = match inputs.live_policy() {
        Some(marker)
            if marker.policy_version == policy && inputs == baseline.handoff.compiler_inputs =>
        {
            serde_json::Value::Null
        }
        Some(_) => return Err("saved live policy is not the admitted global policy".into()),
        None => {
            // Legacy fresh-cohort Current paired a PRE-PPO individual with newer
            // global heads. Adopt those accepted heads once, preserving all
            // mutable private state; do not pretend the private actor was newer.
            let progress = advance_legacy_policy_sleep(
                &mut runtime,
                id,
                session_nonce,
                world_seed,
                request
                    .collection_cutoff_unix_seconds
                    .min(request.finalization_deadline_unix_seconds),
                &mut on_update,
            )?;
            let transition = N2048LivePolicyTransition::adopt_existing_policy(
                runtime.live_foundation_compiler_inputs(id)?,
                policy,
                asset.clone(),
            )?;
            runtime.install_live_foundation_policy(id, &transition)?;
            trainer.rebind_for_next_cohort(transition.next_phenotype().clone(), asset.clone())?;
            progress
        }
    };
    let sampling = runtime
        .foundation_training_sampling_state()
        .ok_or("training sampler absent after legacy sleep")?;
    let collection_start_tick = runtime.world().tick().raw();
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
    // Unassisted mode only observes actual food. Opt-in care assistance records
    // each movement of existing safe loose food; it never feeds the learner.
    let mut food = BTreeMap::new();
    let mut care_scenario: Option<ContinuousCareAssistance> = None;
    let mut care_cue_emissions = 0u64;
    let mut care_cue_deferrals = 0u64;
    let mut care_food_repositions = 0u64;
    let mut last_care_cue_tick = None;
    let restore_seconds = started.elapsed().as_secs_f64();
    on_update(
        serde_json::json!({"kind":"continuous-started-v1","session_nonce":session_nonce,
            "source_world_seed":world_seed,"organism_id":id.raw(),"policy_version":policy,
            "session_start_world_tick":start_tick,"world_tick":collection_start_tick,"world_ticks_per_second":alife_world::WORLD_TICKS_PER_SECOND,
            "legacy_sleep_progress":legacy_sleep_progress,
            "care_assistance_requested":request.care_assistance,
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
        // The life directory's parent owns the sealed cohort bundle. Use the
        // same directory for collection and both subsequent replay passes.
        let replay_dir = window_root.join("individual");
        let phenotype = alife_training::compile_validated_n2048_cached(
            runtime.live_foundation_compiler_inputs(id)?,
            &alife_core::BrainCapacityClass::n2048(),
        )?;
        let actor_before = trainer.optimizer_step();
        let value_checkpoint = value.checkpoint(trainer.session())?;
        let value_before = value_checkpoint.optimizer_step;
        let update_version =
            continuous_update_version(policy, value_checkpoint.last_updated_policy_version)?;
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
            &replay_dir,
            source.clone(),
            phenotype.clone(),
            &asset,
            budget,
        )?;
        let mut refs: Vec<crate::FoundationReplayRecordRef> = Vec::new();
        let mut features = Vec::new();
        let mut generic_rows = Vec::new();
        let mut meal_rows = Vec::new();
        let mut care_rows = Vec::new();
        let care_before = care_world_observation(runtime.world(), id);
        if let Some(scenario) = care_scenario.as_ref() {
            let observation = care_world_observation(runtime.world(), id);
            let far = observation["food"].as_array().is_some_and(|foods| {
                foods.iter().any(|food| {
                    food["id"].as_u64() == Some(scenario.food.raw())
                        && food["geometric_distance"]
                            .as_f64()
                            .is_some_and(|distance| distance > 4.0)
                })
            });
            let moved = if far
                && runtime
                    .world()
                    .entity(scenario.food)
                    .is_some_and(care_food_ready)
            {
                match care_food_position_in_gaze(runtime.world(), id) {
                    Some(position) => crate::foundation_training::relocate_unheld_foundation_food(
                        runtime.world_mut(),
                        scenario.food,
                        position,
                    )?,
                    None => false,
                }
            } else {
                false
            };
            if moved {
                care_food_repositions += 1;
                on_update(serde_json::json!({"kind":"continuous-care-reposition-v1",
                    "session_nonce":session_nonce,"world_tick":runtime.world().tick().raw(),
                    "food_target":scenario.food.raw(),"mode":"training_assistance_in_gaze",
                    "before":observation,"after":care_world_observation(runtime.world(), id),
                    "forced_action":false,"reward_override":false,"biology_override":false}))?;
            }
        }
        for object in runtime.world().object_snapshots() {
            if object.kind == alife_world::WorldObjectKind::Food {
                food.entry(object.id.raw()).or_insert_with(|| {
                    (object.id, crate::FoundationFoodOutcome::new(id, object.id))
                });
            }
        }
        crate::foundation_training::prime_foundation_lesson_prior(&mut runtime, None)?;
        let collection = Instant::now();
        let first_tick = runtime.world().tick().raw();
        let capture_before = runtime.foundation_training_capture_metrics();
        let mut no_progress = 0;
        let mut stalled = Instant::now();
        let mut first_control_call = true;
        let mut terminal_observed = false;
        loop {
            if now()? >= request.collection_cutoff_unix_seconds {
                stop = "collection_cutoff";
                break;
            }
            if runtime.world().tick().raw().saturating_sub(first_tick) >= tick_limit(wanted)? {
                stop = "world_tick_guard";
                break;
            }
            if request.care_assistance {
                if care_scenario.is_none() {
                    let ready = runtime
                        .world()
                        .entity_id("food-01")
                        .and_then(|target| runtime.world().entity(target))
                        .is_some_and(care_food_ready)
                        && care_food_position_in_gaze(runtime.world(), id).is_some();
                    if ready {
                        let scenario = prepare_care_assistance(runtime.world_mut(), id)?;
                        on_update(serde_json::json!({"kind":"continuous-care-assistance-v1",
                            "session_nonce":session_nonce,"world_tick":runtime.world().tick().raw(),
                            "mode":"ordinary_feeding_scene_and_spatial_eat_food_cues",
                            "food_target":scenario.food.raw(),"reward_override":false,
                            "biology_override":false,"forced_action":false,
                            "state":care_world_observation(runtime.world(), id)}))?;
                        care_scenario = Some(scenario);
                    }
                }
                if let Some(scenario) = care_scenario.as_ref() {
                    let tick = runtime.world().tick().raw();
                    let available = runtime
                        .world()
                        .entity(scenario.food)
                        .is_some_and(|food| !food.consumed);
                    if care_cue_due(tick, scenario.start_tick, available, last_care_cue_tick) {
                        // Ordinary food can regrow after an actual meal while the
                        // learner walks away within a 512-decision window. Keep
                        // the assistance explicit and bounded to the cue cadence.
                        let far = care_food_is_far(runtime.world(), id, scenario.food);
                        let not_offered = runtime
                            .world()
                            .entity(scenario.food)
                            .is_some_and(care_food_ready)
                            && !care_food_is_offered(runtime.world(), id, scenario.food);
                        if far || not_offered {
                            let before = care_world_observation(runtime.world(), id);
                            let moved = match care_food_position_in_gaze(runtime.world(), id) {
                                Some(position) => {
                                    crate::foundation_training::relocate_unheld_foundation_food(
                                        runtime.world_mut(),
                                        scenario.food,
                                        position,
                                    )?
                                }
                                None => false,
                            };
                            if moved {
                                care_food_repositions += 1;
                                on_update(
                                    serde_json::json!({"kind":"continuous-care-reposition-v1",
                                    "session_nonce":session_nonce,"world_tick":tick,
                                    "food_target":scenario.food.raw(),"mode":"training_assistance_in_gaze",
                                    "trigger":"periodic_cue", "reason":if far { "outside_range" } else { "not_offered" }, "before":before,
                                    "after":care_world_observation(runtime.world(), id),
                                    "forced_action":false,"reward_override":false,"biology_override":false}),
                                )?;
                            }
                        }
                        let emitted = expose_care_cue(runtime.world_mut(), id, scenario).map_err(
                            |error| {
                                format!("continuous care cue failed at world tick {tick}: {error}")
                            },
                        )?;
                        if emitted {
                            care_cue_emissions += 1;
                        } else {
                            care_cue_deferrals += 1;
                        }
                        last_care_cue_tick = Some(tick);
                    }
                }
            }
            let before_tick = runtime.world().tick();
            let before_food = food
                .iter()
                .map(|(raw, (target, _))| {
                    (
                        *raw,
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
            let tick_result = runtime.tick_outcome().map_err(|error| {
                format!(
                    "continuous runtime tick {} failed: {error}",
                    before_tick.raw()
                )
            });
            if let Some(event) = observed_terminal_event(&runtime, id, session_nonce, world_seed) {
                stop = "natural_death";
                terminal_death_tick = event["terminal_death_tick"].as_u64();
                terminal_observed = true;
                // Even a tick error may follow an observed death. Persist it
                // before propagating that error or processing returned rows.
                record_terminal_then(event, &mut on_update, || {
                    tick_result?;
                    Ok(())
                })?;
            } else {
                tick_result?;
            }
            let mut steps = runtime.take_foundation_training_steps();
            if runtime.world().tick() == before_tick {
                if !steps.is_empty() {
                    return Err("uncommitted world tick returned training rows".into());
                }
                if terminal_observed {
                    break;
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
                care_rows.push(care_row_evidence(&step, &food));
                for (raw, (target, summary)) in &mut food {
                    let observation =
                        crate::foundation_training::food_outcome::FoodOutcomeObservation {
                            organism: id,
                            target: *target,
                            decision_tick: before_tick,
                            outcome_tick: runtime.world().tick(),
                            before: before_food[raw],
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
            if terminal_observed {
                break;
            }
            if refs.len() > wanted {
                break;
            }
        }
        let care_after = care_world_observation(runtime.world(), id);
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
        if terminal_observed {
            // The terminal row/food observations above are included in capture
            // telemetry, while all this suffix remains explicitly untrained.
            // The immutable terminal event was already committed before metrics.
            drop(writer);
            drop(cohort);
            alife_storage::remove_dir_all(&window_root)?;
            on_update(serde_json::json!({"kind":"continuous-untrained-window-v1",
                "session_nonce":session_nonce,"captured_rows":refs.len(),"trained_rows":0,
                "stop_reason":stop,"causal_collection":causal,
                "generic_consumed_events":generic_rows.len(),"exact_nourished_meals":meal_rows.len(),
                "policy_version":policy,"semantic_prior":compact_prior(&runtime, care_cue_emissions)}))?;
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
                "policy_version":policy,"semantic_prior":compact_prior(&runtime, care_cue_emissions)}))?;
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
                    &replay_dir,
                    slice,
                    0,
                    &source,
                    &phenotype,
                    budget,
                )
                .map_err(|error| {
                    format!("continuous value replay {}: {error}", replay_dir.display())
                })?;
                values.extend(
                    value
                        .predict_values(&mut trainer, &window.sequence)
                        .map_err(|error| format!("continuous value prediction: {error}"))?,
                );
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
                        joint_action(behavior).map_err(|error| {
                            format!("continuous joint action row {index}: {error}")
                        })?,
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
                policy_version: update_version,
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
        let batch = PpoBatch::from_rollout(update_version, transitions, ppo)
            .map_err(|error| format!("continuous PPO batch: {error}"))?;
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
        // Bound immutable snapshots to this one policy/window. Repeated PPO
        // passes use admitted RAM, not a second claim about mutable disk state.
        // Subsequent external replay loads still verify their actual bytes.
        let mut cached_windows = PpoWindowSnapshotCache::new(spans.len());
        let update = train_recurrent_ppo_cohort(
            &mut trainer,
            &mut value,
            spans.len(),
            8,
            |index| {
                if let Some(cached) = cached_windows.get(index, budget)? {
                    return Ok(cached);
                }
                let (burn, start, end) = spans[index];
                let replay = load_foundation_replay_window(
                    &replay_dir,
                    &refs[burn..end],
                    start - burn,
                    &source,
                    &phenotype,
                    budget,
                )
                .map_err(|error| {
                    eprintln!("continuous PPO replay {}: {error}", replay_dir.display());
                    alife_training::TrainingError::from(
                        ScaffoldContractError::InvalidDecisionEvidence,
                    )
                })?;
                let window = PpoTrainingWindow {
                    speech_targets: vec![None; replay.sequence.ticks.len()],
                    sequence: replay.sequence,
                    batch: batch.window(start..end)?,
                    auxiliary: None,
                };
                cached_windows.retain(index, &window, budget);
                Ok(window)
            },
            PpoConfig {
                value_coefficient: 0.0,
                ..ppo
            },
            update_version,
        ).map_err(|error| format!("continuous PPO training: collection policy {policy}, update version {update_version}: {error}"))?;
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
        let next_policy = update_version;
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
        let value_updates = value_age
            .checked_sub(value_before)
            .ok_or("value age regressed")?;
        let install_seconds = install_started.elapsed().as_secs_f64();
        install_seconds_total += install_seconds;
        let event = serde_json::json!({"kind":"continuous-update-v1","session_nonce":session_nonce,
            "source_world_seed":world_seed,"organism_id":id.raw(),"update":completed_updates,
            "policy_before":policy-1,"policy_after":policy,"trained_rows":rows,
            "session_trained_rows":trained_rows,"captured_replay_rows":refs.len(),"bootstrap_replay_rows":1,
            "actor_optimizer_step":trainer.optimizer_step(),"value_optimizer_step":value_age,
            "actor_optimizer_updates":actor_updates,"value_optimizer_updates":value_updates,
            "completed_epochs":update.completed_epochs,"stopped_for_kl":update.stopped_for_kl,
            "causal_collection":causal,"ppo_seconds":ppo_seconds,"install_seconds":install_seconds,
            "mutable_words_written_during_install":install.mutable_words_written,
            "generic_consumed_events":generic_rows.len(),"trained_generic_consumed_events":generic_rows.iter().filter(|r| **r<rows).count(),
            "exact_nourished_meals":meal_rows.len(),"trained_exact_nourished_meals":meal_rows.iter().filter(|r| **r<rows).count(),
            "food_evidence_scope":"exact existing food entities observed at this window start; regrowth of same entity is ordinary ecology",
            "observed_food_entity_count":food.len(),"semantic_prior":compact_prior(&runtime, care_cue_emissions),
            "care_assistance_requested":request.care_assistance,
            "care_assistance_active":care_scenario.is_some(),"care_cue_emissions":care_cue_emissions,"care_cue_deferrals":care_cue_deferrals,
            "care_food_repositions":care_food_repositions,
            "care_before":care_before,"care_after":care_after,
            "care_training_rows":care_row_summary(&care_rows[..rows]),
            "food_outcomes_cumulative_including_bootstrap":food.values().map(|(_, summary)| summary).collect::<Vec<_>>(),
            "replay_bundle_bytes":replay.bundle_bytes,"replay_data_sync_calls":replay.data_sync_calls,
        });
        alife_storage::remove_dir_all(&window_root)?;
        on_update(event)?;
        if completed_updates.is_multiple_of(request.snapshot_every_updates as u64) {
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
        collection_start_tick,
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
            "semantic_prior":compact_prior(&runtime, care_cue_emissions),
            "care_assistance_requested":request.care_assistance,
            "care_assistance_active":care_scenario.is_some(),"care_cue_emissions":care_cue_emissions,"care_cue_deferrals":care_cue_deferrals,
            "care_food_repositions":care_food_repositions,
            "snapshot_seconds":snapshot_seconds,"producer_seconds":started.elapsed().as_secs_f64(),
            "collection_seconds":collection_seconds,"ppo_seconds":ppo_seconds_total,"install_seconds":install_seconds_total,
            "restore_seconds":restore_seconds,"replay_seal_seconds":replay_seal_seconds,"boundary_wait_seconds":boundary_wait_seconds,
            "legacy_sleep_progress":legacy_sleep_progress,
            "causal_collection":causal_collection,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct WorkFixture(std::path::PathBuf);
    impl WorkFixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "alife-continuous-work-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir(&root).unwrap();
            std::fs::create_dir(root.join(".run")).unwrap();
            Self(root)
        }
        fn run(&self) -> std::path::PathBuf {
            self.0.join(".run")
        }
    }
    impl Drop for WorkFixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn continuous_new_run_prepares_materialization_parent_under_active_quota() {
        let fixture = WorkFixture::new();
        let budget = alife_storage::OutputBudget::register(fixture.run(), 4).unwrap();
        let life = prepare_live_work_directory(&fixture.run().join("live")).unwrap();
        assert!(life.parent().unwrap().is_dir());
        // Same create-new boundary used by the native materializer. Previously
        // this failed because the parent .run/live had never been created.
        std::fs::create_dir(&life).unwrap();
        alife_storage::write(life.join("fixture"), b"life").unwrap();
        assert_eq!(budget.usage().unwrap().used, 4);
        assert!(alife_storage::write(life.join("overflow"), b"x").is_err());
        assert_eq!(std::fs::metadata(life.join("overflow")).unwrap().len(), 0);
        assert_eq!(budget.usage().unwrap().used, 4);
    }

    #[test]
    fn continuous_work_collision_preserves_existing_data() {
        let fixture = WorkFixture::new();
        let work = fixture.run().join("live");
        std::fs::create_dir(&work).unwrap();
        std::fs::write(work.join("sentinel"), b"original").unwrap();
        let budget = alife_storage::OutputBudget::register(fixture.run(), 4096).unwrap();
        let before = budget.usage().unwrap();
        assert!(prepare_live_work_directory(&work).is_err());
        assert_eq!(std::fs::read(work.join("sentinel")).unwrap(), b"original");
        assert_eq!(budget.usage().unwrap(), before);
        assert!(!work.join("live").exists());
    }

    #[test]
    fn continuous_work_requires_existing_owned_parent_and_active_quota() {
        let fixture = WorkFixture::new();
        let work = fixture.run().join("live");
        assert!(prepare_live_work_directory(&work).is_err());
        assert!(!work.exists());
        assert!(prepare_live_work_directory(&fixture.0.join("missing/.run/live")).is_err());
        assert!(!fixture.0.join("missing").exists());
    }
    #[test]
    fn continuous_deadline_uses_only_actual_next_state() {
        assert_eq!(loss_rows(0, 512), 0);
        assert_eq!(loss_rows(1, 512), 0);
        assert_eq!(loss_rows(11, 512), 10);
        assert_eq!(loss_rows(513, 512), 512);
    }
    #[test]
    fn continuous_update_advances_the_admitted_value_version() {
        assert_eq!(
            continuous_update_version(10977, Some(10977)).unwrap(),
            10978
        );
        assert!(continuous_update_version(10977, Some(10976)).is_err());
        assert!(continuous_update_version(10977, None).is_err());
        assert!(continuous_update_version(u64::MAX, Some(u64::MAX)).is_err());
    }
    #[test]
    fn assisted_feeding_uses_real_scene_and_cue_without_changing_biology() {
        let seed = 42;
        let asset = crate::foundation_training::initial_n2048_care_asset(seed).unwrap();
        let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.sensor_profile = asset.manifest().sensor_profile();
        let mut world =
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)
                .unwrap()
                .world;
        crate::foundation_training::ground_foundation_training_world(&mut world).unwrap();
        for label in ["hazard-01", "obstacle-02"] {
            let entity = world.entity_id(label).unwrap();
            world.editor_remove_object(entity).unwrap();
        }
        let id = world.organism_entity_ids()[0].0;
        let before = *world.organism_registry().get(id).unwrap().biochemistry();
        let tick = world.tick();
        let scenario = prepare_care_assistance(&mut world, id).unwrap();
        assert_eq!(
            *world.organism_registry().get(id).unwrap().biochemistry(),
            before
        );
        assert_eq!(world.tick(), tick);
        assert!(care_food_ready(world.entity(scenario.food).unwrap()));
        let subject = world
            .entity(world.organism_registry().get(id).unwrap().world_entity_id())
            .unwrap();
        let subject_position = subject.position;
        let target = world.entity(scenario.food).unwrap();
        let target_position = target.position;
        let distance =
            (subject.position.x - target.position.x).hypot(subject.position.z - target.position.z);
        assert!((distance - 1.0).abs() < 0.001);
        let report = world.sensory_report(id, world.tick()).unwrap();
        assert!(report
            .visible_entities
            .iter()
            .any(|entity| entity.id == scenario.food));
        let homeostasis = world
            .organism_registry()
            .get(id)
            .unwrap()
            .biochemistry()
            .homeostasis;
        let frame = world
            .perception_frame(
                id,
                tick,
                alife_core::SensorProfile::GroundedTerrainVisionV1,
                homeostasis,
            )
            .unwrap();
        assert!(
            frame.candidates().iter().any(|candidate| {
                candidate.action_id == alife_world::HeadlessActionIds::EAT
                    && candidate.target.entity == Some(scenario.food)
            }),
            "food {:?}; body {:?}; target {:?}; visible {:?}; candidate targets {:?}",
            scenario.food,
            subject_position,
            target_position,
            report.visible_entities,
            frame
                .candidates()
                .iter()
                .map(|candidate| (candidate.action_id, candidate.target.entity))
                .collect::<Vec<_>>()
        );
        let far = alife_core::Vec3f::new(
            subject_position.x + 10.0,
            subject_position.y,
            subject_position.z,
        );
        world.editor_move_object(scenario.food, far).unwrap();
        assert!(care_food_is_far(&world, id, scenario.food));
        let nearby = care_food_position_in_gaze(&world, id).unwrap();
        assert!(crate::foundation_training::relocate_unheld_foundation_food(
            &mut world,
            scenario.food,
            nearby
        )
        .unwrap());
        assert!(!care_food_is_far(&world, id, scenario.food));
        assert_eq!(
            *world.organism_registry().get(id).unwrap().biochemistry(),
            before
        );
        assert!(expose_care_cue(&mut world, id, &scenario).unwrap());
        assert!(world
            .audible_utterances()
            .iter()
            .any(
                |utterance| utterance.source_kind == alife_core::UtteranceSourceKind::Teacher
                    && utterance.addressee == Some(id)
                    && utterance
                        .tokens
                        .iter()
                        .map(|token| token.raw())
                        .collect::<Vec<_>>()
                        == [8, 1]
            ));
        assert!(!world.entity(scenario.food).unwrap().consumed);
        assert!(care_cue_due(tick.raw(), tick.raw(), true, None));
        assert!(!care_cue_due(
            tick.raw(),
            tick.raw(),
            true,
            Some(tick.raw())
        ));
        assert!(!care_cue_due(tick.raw(), tick.raw(), false, None));
        let body = world
            .entity(world.organism_registry().get(id).unwrap().world_entity_id())
            .unwrap();
        let yaw = body.body_yaw + body.head_yaw;
        let blocked = alife_core::Vec3f::new(
            body.position.x + 0.5 * yaw.cos(),
            body.position.y,
            body.position.z + 0.5 * yaw.sin(),
        );
        let obstacle = world.entity_id("obstacle-01").unwrap();
        world.editor_move_object(obstacle, blocked).unwrap();
        assert!(care_food_position_in_gaze(&world, id).is_none());
    }
    #[test]
    fn care_placement_and_teacher_stay_usable_at_terrain_edge() {
        let seed = 42;
        let asset = crate::foundation_training::initial_n2048_care_asset(seed).unwrap();
        let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.sensor_profile = asset.manifest().sensor_profile();
        let mut world =
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)
                .unwrap()
                .world;
        crate::foundation_training::ground_foundation_training_world(&mut world).unwrap();
        let id = world.organism_entity_ids()[0].0;
        let body = world.organism_registry().get(id).unwrap().world_entity_id();
        // The default gaze faces +X. A one-unit placement would leave terrain.
        world
            .editor_move_object(body, alife_core::Vec3f::new(255.999, 0.0, 0.0))
            .unwrap();
        let before = *world.organism_registry().get(id).unwrap().biochemistry();
        let care = prepare_care_assistance(&mut world, id).unwrap();
        let target = world.entity(care.food).unwrap().position;
        assert!((target.x - 255.999).abs() < 0.01);
        let tick = world.tick();
        let homeostasis = world
            .organism_registry()
            .get(id)
            .unwrap()
            .biochemistry()
            .homeostasis;
        let frame = world
            .perception_frame(
                id,
                tick,
                alife_core::SensorProfile::GroundedTerrainVisionV1,
                homeostasis,
            )
            .unwrap();
        assert!(frame
            .candidates()
            .iter()
            .any(
                |candidate| candidate.action_id == alife_world::HeadlessActionIds::EAT
                    && candidate.target.entity == Some(care.food)
            ));
        assert_eq!(
            *world.organism_registry().get(id).unwrap().biochemistry(),
            before
        );

        // The old teacher position (food.z + 2) lies outside this edge.
        world
            .editor_move_object(body, alife_core::Vec3f::new(20.0, 0.0, 255.9))
            .unwrap();
        let care = prepare_care_assistance(&mut world, id).unwrap();
        assert!(expose_care_cue(&mut world, id, &care).unwrap());
        let teacher = world
            .entity(world.entity_id("nursery-teacher").unwrap())
            .unwrap();
        assert!(world
            .terrain()
            .unwrap()
            .walkable(teacher.position.x, teacher.position.z));
        assert_eq!(
            *world.organism_registry().get(id).unwrap().biochemistry(),
            before
        );
    }
    #[test]
    fn cue_is_deferred_when_no_teacher_position_is_walkable() {
        let id = OrganismId(1);
        let mut world = alife_world::HeadlessScenarioBuilder::new(7)
            .agent("learner", id, alife_core::Vec3f::ZERO)
            .food("food", alife_core::Vec3f::ZERO, 0.45)
            .build()
            .unwrap();
        let terrain = alife_world::WorldTerrain::new(
            alife_world::TerrainData {
                width: 2,
                depth: 2,
                origin_x: -0.5,
                origin_z: -0.5,
                spacing: 1.0,
                heights: vec![0.0; 4],
                obstacles: Vec::new(),
                water_level: None,
            },
            alife_world::LocomotionLimits::default(),
        )
        .unwrap();
        world
            .enable_terrain_for_new_game(terrain, alife_core::Vec3f::ZERO)
            .unwrap();
        let care = ContinuousCareAssistance {
            food: world.entity_id("food").unwrap(),
            start_tick: world.tick().raw(),
        };
        let before_tick = world.tick();
        assert!(!expose_care_cue(&mut world, id, &care).unwrap());
        assert_eq!(world.tick(), before_tick);
        assert!(world.audible_utterances().is_empty());
        assert!(!world.entity(care.food).unwrap().consumed);
    }
    #[test]
    fn nearby_food_behind_gaze_is_not_treated_as_offered() {
        let seed = 42;
        let asset = crate::foundation_training::initial_n2048_care_asset(seed).unwrap();
        let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.sensor_profile = asset.manifest().sensor_profile();
        let mut world =
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)
                .unwrap()
                .world;
        crate::foundation_training::ground_foundation_training_world(&mut world).unwrap();
        let id = world.organism_entity_ids()[0].0;
        let food = world.entity_id("food-01").unwrap();
        world
            .editor_move_object(food, alife_core::Vec3f::new(1.0, 0.0, 0.0))
            .unwrap();
        let before = *world.organism_registry().get(id).unwrap().biochemistry();
        assert!(!care_food_is_far(&world, id, food));
        assert!(!care_food_is_offered(&world, id, food));
        prepare_care_assistance(&mut world, id).unwrap();
        assert!(care_food_is_offered(&world, id, food));
        assert_eq!(
            *world.organism_registry().get(id).unwrap().biochemistry(),
            before
        );
        assert!(!world.entity(food).unwrap().consumed);
    }
    #[test]
    fn care_setup_waits_for_a_safe_loose_unconsumed_food() {
        let world = alife_world::HeadlessScenarioBuilder::new(7)
            .food("food", alife_core::Vec3f::ZERO, 0.45)
            .build()
            .unwrap();
        let mut food = world
            .entity(world.entity_id("food").unwrap())
            .unwrap()
            .clone();
        assert!(care_food_ready(&food));
        food.consumed = true;
        assert!(!care_food_ready(&food));
        food.consumed = false;
        food.hazard_pain = 0.1;
        assert!(!care_food_ready(&food));
        food.hazard_pain = 0.0;
        food.carried_by = Some(OrganismId(1));
        assert!(!care_food_ready(&food));
    }
    #[test]
    fn care_row_summary_counts_only_the_supplied_training_slice() {
        let rows = [
            CareRowEvidence {
                food_candidate: true,
                food_approach: true,
                ..Default::default()
            },
            CareRowEvidence {
                food_eat: true,
                ..Default::default()
            },
            CareRowEvidence {
                food_grab: true,
                ..Default::default()
            },
        ];
        let summary = care_row_summary(&rows[..2]);
        assert_eq!(summary["trained_rows"], 2);
        assert_eq!(summary["food_candidate_rows"], 1);
        assert_eq!(summary["food_approach_command_rows"], 1);
        assert_eq!(summary["food_eat_command_rows"], 1);
        assert_eq!(summary["food_grab_command_rows"], 0);
    }
    #[test]
    fn continuous_tick_guard_is_relative_to_restored_world() {
        assert_eq!(tick_limit(512).unwrap(), 8192);
        let restored_tick = 1_000_000u64;
        assert!((restored_tick + 8191).saturating_sub(restored_tick) < tick_limit(512).unwrap());
        assert!((restored_tick + 8192).saturating_sub(restored_tick) >= tick_limit(512).unwrap());
    }
}
