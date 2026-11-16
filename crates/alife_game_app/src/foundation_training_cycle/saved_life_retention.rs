//! Opt-in, bounded saved-life observation. No offline optimizer is constructed.
use super::*;
use crate::foundation_training::food_outcome::{ownership, FoodOutcomeObservation};
use alife_core::Validate;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_TRACE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedLifeRetentionRequest {
    pub schema: u16,
    /// Assessment identity only; never changes the saved sampling seed or stream.
    pub evaluation_nonce: u64,
    pub max_decisions: usize,
    pub max_world_ticks: u64,
    pub deadline_unix_seconds: u64,
    pub max_output_bytes: u64,
}

impl SavedLifeRetentionRequest {
    pub fn validate(&self, now: u64) -> Result<()> {
        if self.schema != 2
            || self.evaluation_nonce == 0
            || !((self.max_decisions == 0 && self.max_world_ticks == 0)
                || ((1..=256).contains(&self.max_decisions)
                    && (1..=8192).contains(&self.max_world_ticks)))
            || self.deadline_unix_seconds <= now
            || self.deadline_unix_seconds > now.saturating_add(600)
            || !(64 * 1024 * 1024..=8 * 1024 * 1024 * 1024).contains(&self.max_output_bytes)
        {
            return Err("invalid saved-life retention bounds".into());
        }
        Ok(())
    }
}

fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[derive(Debug, Default, serde::Serialize)]
struct TeacherCues {
    object_ids: Vec<u64>,
    passive_teacher_object_ids: Vec<u64>,
    utterances: usize,
}
impl TeacherCues {
    fn observe(world: &alife_world::HeadlessWorld) -> Self {
        Self {
            object_ids: world
                .object_snapshots()
                .iter()
                .filter(|object| object.teacher_channel.is_some())
                .map(|object| object.id.raw())
                .collect(),
            passive_teacher_object_ids: world
                .object_snapshots()
                .iter()
                .filter(|object| {
                    object.teacher_channel.is_none() && object.label == "nursery-teacher"
                })
                .map(|object| object.id.raw())
                .collect(),
            utterances: world
                .audible_utterances()
                .iter()
                .filter(|utterance| {
                    utterance.source_kind == alife_core::UtteranceSourceKind::Teacher
                })
                .count(),
        }
    }
    fn clear(&self) -> bool {
        self.object_ids.is_empty() && self.utterances == 0
    }
}

fn prior_off(metrics: Option<&crate::gpu_live_runtime::SemanticPriorMetrics>) -> bool {
    metrics.is_none_or(|m| {
        m.backend == "off"
            && m.requests == 0
            && m.delivered_frames == 0
            && m.decision_frames_with_prior == 0
    })
}

fn physiology(transition: &alife_core::MeasuredPhysiologyTransition) -> serde_json::Value {
    let state = |s: &alife_core::BiochemistryState| {
        serde_json::json!({
            "tick":s.tick.raw(), "energy":s.body.energy,"health":s.body.health,
            "injury":s.body.injury,"hunger":s.homeostasis.drives.hunger,
            "pain":s.homeostasis.drives.pain,"sleeping":s.body.sleeping,
        })
    };
    serde_json::json!({"authority":"sealed measured_physiology", "before":state(&transition.before),
        "after":state(&transition.after),"energy_delta":transition.energy_delta,
        "pain_delta":transition.pain_delta,"homeostatic_delta":transition.homeostatic_delta})
}

fn observation_seeds(
    baseline: &alife_training::CurrentTrainingCheckpointV1,
    nonce: u64,
) -> Result<alife_training::CheckpointSeedsV1> {
    baseline.require_training_session_nonce(nonce)?;
    if baseline.same_individual.as_ref().is_some_and(|tag| {
        tag.progress.session_nonce == nonce
            && baseline.seeds.contains(nonce)
            && !baseline
                .seeds
                .pending_range
                .is_some_and(|[a, b]| a <= nonce && nonce <= b)
    }) {
        // A frozen observation may continue the admitted session with zero PPO
        // rows/updates. No new seed allocation or sampling stream is invented.
        return Ok(baseline.seeds.clone());
    }
    if nonce == baseline.seed || baseline.seeds.contains(nonce) {
        return Err(
            "retention assessment nonce must be fresh and distinct from the world seed".into(),
        );
    }
    // Account for an assessment identity without consuming an unrelated pending
    // training allocation. This never changes the world's saved RNG stream.
    let mut seeds = baseline.seeds.clone();
    let pending = seeds.pending_range.take();
    seeds.reserve(nonce, nonce)?;
    seeds.consume(nonce)?;
    seeds.pending_range = pending;
    seeds.validate()?;
    Ok(seeds)
}

#[allow(clippy::too_many_arguments)]
fn preserve_endpoint(
    runtime: &mut GpuLiveBrainRuntime,
    baseline: &alife_training::CurrentTrainingCheckpointV1,
    baseline_bytes: &[u8],
    life: &Path,
    save_path: &Path,
    assets: &Path,
    output: &Path,
    organism: alife_core::OrganismId,
    start_tick: u64,
    nonce: u64,
    temperature: f32,
    request_sha256: &str,
    seeds: alife_training::CheckpointSeedsV1,
) -> Result<serde_json::Value> {
    let exact = runtime.capture_portable_checkpoint()?;
    let published = crate::GpuDurableSaveManifest::publish_snapshot(save_path, assets, &exact)?;
    if published.save != exact {
        return Err("retention final save publication changed state".into());
    }
    runtime.rebind_durable_checkpoint_boundary(save_path, assets, &exact)?;
    let mut handoff = baseline.handoff.clone();
    handoff.continuation = alife_training::FoundationContinuationKindV1::SameIndividual;
    let mut checkpoint = seal_same_individual_training_checkpoint(
        baseline,
        baseline_bytes,
        handoff,
        life,
        alife_training::SameIndividualProgressV1 {
            session_nonce: nonce,
            organism_id: organism.raw(),
            session_start_world_tick: start_tick,
            world_tick: runtime.world().tick().raw(),
            trained_rows: 0,
            completed_updates: 0,
            sampling_temperature: temperature,
        },
        seeds,
    )?;
    if checkpoint.handoff.actor != baseline.handoff.actor
        || checkpoint.handoff.value != baseline.handoff.value
    {
        return Err("retention final endpoint changed frozen actor/value Adam".into());
    }
    let alive = runtime
        .world()
        .organism_registry()
        .get(organism)
        .is_some_and(|record| record.lifecycle().is_alive());
    if !alive {
        checkpoint = checkpoint.with_terminal_disposition(
            &digest(baseline_bytes),
            request_sha256,
            nonce,
            serde_json::json!({"kind":"continuous-terminal-v1","terminal":true,
                "session_nonce":nonce,"source_world_seed":runtime.world().seed(),
                "organism_id":organism.raw(),"world_tick":runtime.world().tick().raw(),
                "terminal_death_tick":null,"observation_scope":"frozen-head-retention"}),
        )?;
    }
    let zero_step_equality = if runtime.world().tick().raw() == start_tick {
        let before = baseline
            .life
            .as_ref()
            .ok_or("retention baseline life missing")?;
        let after = checkpoint
            .life
            .as_ref()
            .ok_or("retention endpoint life missing")?;
        Some(serde_json::json!({
            "world_json_byteequal":before.world_json == after.world_json,
            "sleep_journal_byteequal":before.sleep_journal_json == after.sleep_journal_json,
            "private_asset_closure_byteequal":before.assets_base64 == after.assets_base64,
            "saved_sampling_equal":baseline.saved_training_sampling()? == checkpoint.saved_training_sampling()?,
            "scope":"Any false field requires explicit compatibility review before biological evaluation"
        }))
    } else {
        None
    };
    let bytes = checkpoint.encode()?;
    let path = output.join("final.alife-checkpoint");
    let file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)?;
    let sync = file.try_clone()?;
    let mut writer = alife_storage::writer(&path, file)?;
    writer.write_all(&bytes)?;
    writer.flush()?;
    sync.sync_all()?;
    drop(writer);
    if read_current_training_checkpoint(&path)? != checkpoint {
        return Err("retention final Current readback differs".into());
    }
    Ok(
        serde_json::json!({"path":"final.alife-checkpoint","sha256":digest(&bytes),
        "world_tick":runtime.world().tick().raw(),"same_individual":true,
        "actor_value_Adam_unchanged":true,"offline_trained_rows":0,"offline_updates":0,
        "private_world_memory_neural_and_sampling_preserved":true,"living_continuation":alive,
        "source_current_sha256":digest(baseline_bytes),"zero_step_equality":zero_step_equality}),
    )
}
fn zero_step_preservation_valid(endpoint: &serde_json::Value) -> bool {
    let Some(equal) = endpoint
        .get("zero_step_equality")
        .filter(|value| !value.is_null())
    else {
        return true;
    };
    [
        "world_json_byteequal",
        "sleep_journal_byteequal",
        "private_asset_closure_byteequal",
        "saved_sampling_equal",
    ]
    .iter()
    .all(|field| equal.get(*field).and_then(serde_json::Value::as_bool) == Some(true))
}
fn qualified(complete: bool, unchanged: bool, cue_free: bool, endpoint_preserved: bool) -> bool {
    complete && unchanged && cue_free && endpoint_preserved
}

/// Restore the exact admitted living individual and its saved sampling stream.
/// Global founder/critic weights, moments and ages are only read. Ordinary
/// waking/private learning, sleep and biology remain active; no source save is
/// published. Active teacher cues cause refusal rather than world editing; passive labels and all learned history remain.
pub fn evaluate_saved_life_retention(
    source: &Path,
    output: &Path,
    request: SavedLifeRetentionRequest,
) -> Result<serde_json::Value> {
    request.validate(now()?)?;
    if std::env::var("ALIFE_SLM_PRIOR").ok().as_deref() != Some("off")
        || std::env::var("ALIFE_SLM_PRIOR_BACKEND").ok().as_deref() != Some("off")
    {
        return Err(
            "saved-life retention requires ALIFE_SLM_PRIOR=off and ALIFE_SLM_PRIOR_BACKEND=off"
                .into(),
        );
    }
    let output = resolve_current_training_destination(source, output)?;
    let source = source.canonicalize()?;
    let metadata = fs::metadata(&source)?;
    if !metadata.is_file() || metadata.len() > alife_training::MAX_CURRENT_CHECKPOINT_BYTES {
        return Err("saved-life retention requires a bounded Current file".into());
    }
    let original = fs::read(&source)?;
    let source_sha = digest(&original);
    fs::create_dir(&output)?; // Never overwrite, resume or remove previous evidence.
    let budget = alife_storage::OutputBudget::register(&output, request.max_output_bytes)?;
    // Protect two bounded private-life/Current copies before any biological step.
    let final_headroom = budget.reserve(
        (original.len() as u64)
            .checked_mul(2)
            .and_then(|n| n.checked_add(1024 * 1024))
            .ok_or("retention endpoint size overflow")?,
    )?;
    let _validation = current::validation_scope(&output)?;
    alife_storage::write(
        output.join("request.json"),
        serde_json::to_vec_pretty(&request)?,
    )?;
    let baseline = read_current_training_checkpoint(&source)?;
    baseline.require_living_continuation()?;
    let actor_sha = digest(&serde_json::to_vec(&baseline.handoff.actor)?);
    let value_sha = digest(&serde_json::to_vec(&baseline.handoff.value)?);
    let life = output.join("life");
    materialize_current_training_life(&baseline, &life)?;
    let save_path = life.join("world.json");
    let assets = life.join("world-assets");
    let (_, loaded) = crate::GpuDurableSaveManifest::open_loaded(&save_path, &assets)?;
    let world = loaded.save.restore_headless_world()?;
    let id = {
        let residents = world.organism_registry().iter().collect::<Vec<_>>();
        if residents.len() != 1 || !residents[0].lifecycle().is_alive() {
            return Err("retention requires exactly one living saved individual".into());
        }
        residents[0].organism_id()
    };
    let source_world_tick = world.tick().raw();
    if world.seed() != baseline.seed {
        return Err("retention source world seed mismatch".into());
    }
    let cues = TeacherCues::observe(&world);
    let mut receipt = serde_json::json!({
        "schema":2,"kind":"saved-life-retention-v2","request":request,
        "source":source,"source_sha256":source_sha,"source_world_tick":source_world_tick,
        "organism_id":id.raw(),"policy_version":baseline.handoff.policy_version,
        "actor_sha256":actor_sha,"value_sha256":value_sha,
        "actor_optimizer_step":baseline.handoff.actor.optimizer_step,
        "value_optimizer_step":baseline.handoff.value.optimizer_step,
        "offline_actor_updates":0,"offline_value_updates":0,"offline_optimizer_constructed":false,
        "ordinary_private_learning_active":true,"prior_enabled":false,"new_teacher_cues_emitted_by_evaluator":0,
        "teacher_emission_scope":"No emitter is constructed; current-world channels, utterances and actual hearing are audited independently",
        "existing_teacher_cues":cues,"qualified":false,
        "future_teacher_emitter_constructed":false,"offline_optimizer_constructed":false,
    });
    if !cues.clear() {
        receipt["status"] = serde_json::json!("rejected_existing_teacher_cues_before_gpu");
        receipt["neural_execution"] = serde_json::json!(false);
        receipt["source_unchanged"] = serde_json::json!(fs::read(&source)? == original);
        drop(final_headroom);
        alife_storage::write(
            output.join("retention.json"),
            serde_json::to_vec_pretty(&receipt)?,
        )?;
        return Ok(receipt);
    }
    let endpoint_seeds = observation_seeds(&baseline, request.evaluation_nonce)?;
    let sampling_temperature = baseline
        .saved_training_sampling()?
        .ok_or("retention source lacks exact saved sampling")?
        .temperature;
    if now()? >= request.deadline_unix_seconds {
        return Err("retention deadline before GPU restore".into());
    }
    let world_seed = world.seed();
    let store = crate::GpuCheckpointAssetStore::new(&assets)?;
    let checkpoints = loaded
        .save
        .creatures
        .iter()
        .filter_map(|c| c.gpu_brain.clone())
        .collect::<Vec<_>>();
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    receipt["hardware"] = serde_json::to_value(backend.hardware_receipt())?;
    let mut runtime = GpuLiveBrainRuntime::restore_foundation_observation(
        backend,
        world,
        world_seed,
        BrainScaleTier::Standard2048,
        &store,
        &loaded.save.assets,
        &checkpoints,
        cycle_sampling_config(world_seed, baseline.handoff.policy_version, 1.0)?,
    )?;
    runtime.rebind_durable_checkpoint_boundary(&save_path, &assets, &loaded.save)?;
    if !runtime.foundation_training_sampling_was_restored(id) {
        return Err("retention refuses an invented replacement sampling stream".into());
    }
    if runtime.live_foundation_compiler_inputs(id)? != &baseline.handoff.compiler_inputs {
        return Err(
            "saved private policy differs from Current; retention will not adopt or replace it"
                .into(),
        );
    }
    if !prior_off(runtime.semantic_prior_metrics()) {
        return Err("retention prior is not dormant and off".into());
    }
    receipt["sampling_before"] =
        serde_json::to_value(runtime.foundation_training_sampling_state())?;
    let rows_path = output.join("rows.jsonl");
    let rows_file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&rows_path)?;
    let sync_rows = rows_file.try_clone()?;
    let mut rows_writer = alife_storage::writer(&rows_path, rows_file)?;
    let mut rows_hash = Sha256::new();
    let mut rows_bytes = 0usize;
    let mut rows = 0usize;
    let mut calls = 0u64;
    let mut meals = 0usize;
    let mut cue_free = true;
    let mut error = None;
    let mut outcomes = BTreeMap::<u64, crate::FoundationFoodOutcome>::new();
    let stop = loop {
        let alive = runtime
            .world()
            .organism_registry()
            .get(id)
            .is_some_and(|r| r.lifecycle().is_alive());
        if !alive {
            break "natural_death";
        }
        if rows == request.max_decisions {
            break if request.max_decisions == 0 {
                "restore_only"
            } else {
                "decision_limit"
            };
        }
        if now()? >= request.deadline_unix_seconds {
            break "deadline";
        }
        if runtime
            .world()
            .tick()
            .raw()
            .saturating_sub(source_world_tick)
            >= request.max_world_ticks
        {
            break "world_tick_limit";
        }
        if calls
            >= request
                .max_world_ticks
                .saturating_mul(4096)
                .saturating_add(4096)
        {
            break "control_call_limit";
        }
        if !TeacherCues::observe(runtime.world()).clear() {
            cue_free = false;
            break "teacher_cue_observed";
        }
        let before_tick = runtime.world().tick();
        let before_pose = runtime
            .world()
            .organism_registry()
            .get(id)
            .and_then(|record| runtime.world().entity(record.world_entity_id()))
            .map(|object| (object.position, object.body_yaw));
        let foods = runtime
            .world()
            .object_snapshots()
            .into_iter()
            .filter(|o| o.kind == alife_world::WorldObjectKind::Food)
            .map(|o| {
                (
                    o.id.raw(),
                    (
                        o.id,
                        ownership(runtime.world(), o.id),
                        o.nutrition,
                        o.hazard_pain,
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if foods.len() > 1024 {
            return Err("retention food observation bound exceeded".into());
        }
        calls += 1;
        let tick_error = runtime.tick_outcome().err().map(|e| e.to_string());
        let after_tick = runtime.world().tick();
        let after_pose = runtime
            .world()
            .organism_registry()
            .get(id)
            .and_then(|record| runtime.world().entity(record.world_entity_id()))
            .map(|object| (object.position, object.body_yaw));
        let mut steps = runtime.take_foundation_training_steps();
        if steps.len() > 1
            || (after_tick != before_tick
                && after_tick.raw()
                    != before_tick
                        .raw()
                        .checked_add(1)
                        .ok_or("retention tick overflow")?)
            || (after_tick == before_tick && !steps.is_empty())
        {
            return Err("retention observed a non-canonical tick/row boundary".into());
        }
        if let Some(step) = steps.pop() {
            if step.frame.organism_id() != id || step.frame.tick() != before_tick {
                return Err("retention row identity mismatch".into());
            }
            step.patch.validate_contract()?;
            let measured = step
                .patch
                .outcome()
                .measured_physiology
                .ok_or("retention row lacks sealed physiology")?;
            let heard = step
                .frame
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .any(|h| h.source_kind == alife_core::UtteranceSourceKind::Teacher);
            cue_free &= !heard && TeacherCues::observe(runtime.world()).clear();
            let mut events = Vec::new();
            for (raw, (target, before, nutrition, hazard_pain)) in &foods {
                let observation = FoodOutcomeObservation {
                    organism: id,
                    target: *target,
                    decision_tick: before_tick,
                    outcome_tick: after_tick,
                    before: *before,
                    after: ownership(runtime.world(), *target),
                };
                if !observation.matches_step(&step) {
                    return Err("retention food/row identity mismatch".into());
                }
                let outcome = outcomes
                    .entry(*raw)
                    .or_insert_with(|| crate::FoundationFoodOutcome::new(id, *target));
                let consumed_before = outcome.consumption_events;
                let event = outcome.observe(observation, &step.patch);
                if event.acquisition.is_some() || outcome.consumption_events != consumed_before {
                    meals += usize::from(event.nourished);
                    events.push(serde_json::json!({"target":raw,"before":before,"after":observation.after,
                        "nutrition_before":nutrition,"hazard_pain_before":hazard_pain,
                        "acquired":event.acquisition.is_some(),"consumed":outcome.consumption_events != consumed_before,
                        "nourished":event.nourished}));
                }
            }
            let row = serde_json::json!({"row":rows,"organism":id.raw(),"decision_tick":before_tick.raw(),
                "outcome_tick":after_tick.raw(),"frame_digest":step.frame.frame_digest(),
                "selected_action":step.patch.decision().selected_action,"selected_bundle":step.patch.selected_bundle(),
                "physical":step.patch.outcome().physical,"joint_physical":step.patch.outcome().joint,
                "physiology":physiology(&measured),"food_events":events,"teacher_heard":heard,
                "on_policy_log_probability":step.behavior.on_policy_log_probability()?,
                "cognitive_work":step.patch.cognitive_work(),"full_outcome":step.patch.outcome(),
                "observer_pose_before":before_pose,"observer_pose_after":after_pose,
                "observer_diagnostics_not_biologically_charged":true});
            let mut bytes = serde_json::to_vec(&row)?;
            bytes.push(b'\n');
            rows_bytes = rows_bytes
                .checked_add(bytes.len())
                .ok_or("retention trace size overflow")?;
            if rows_bytes > MAX_TRACE_BYTES {
                return Err("retention trace byte bound exceeded".into());
            }
            rows_writer.write_all(&bytes)?;
            rows_writer.flush()?;
            rows_hash.update(&bytes);
            rows += 1;
        }
        if let Some(e) = tick_error {
            error = Some(e);
            break "runtime_error";
        }
        if !runtime.last_memory_preparation_errors().is_empty()
            || !runtime.last_post_seal_learning_failures().is_empty()
        {
            error = Some("ordinary sealed private learning rejected".into());
            break "private_learning_error";
        }
        if !cue_free {
            break "teacher_cue_observed";
        }
        if !prior_off(runtime.semantic_prior_metrics()) {
            error = Some("prior delivery observed".into());
            break "prior_error";
        }
        if after_tick == before_tick {
            // External GPU polling backoff: this is not organism rest or a world tick.
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    };
    rows_writer.flush()?;
    sync_rows.sync_all()?;
    drop(rows_writer);
    let alive = runtime
        .world()
        .organism_registry()
        .get(id)
        .is_some_and(|r| r.lifecycle().is_alive());
    let mut base_unchanged =
        alive && runtime.live_foundation_compiler_inputs(id)? == &baseline.handoff.compiler_inputs;
    let prior_clear = prior_off(runtime.semantic_prior_metrics());
    let unchanged = fs::read(&source)? == original
        && actor_sha == digest(&serde_json::to_vec(&baseline.handoff.actor)?)
        && value_sha == digest(&serde_json::to_vec(&baseline.handoff.value)?);
    receipt["status"] = serde_json::json!("finished");
    receipt["stop_reason"] = serde_json::json!(stop);
    receipt["error"] = serde_json::json!(error);
    receipt["gpu_restored"] = serde_json::json!(true);
    receipt["neural_execution"] = serde_json::json!(calls > 0);
    receipt["captured_decisions"] = serde_json::json!(rows);
    receipt["world_ticks"] = serde_json::json!(runtime
        .world()
        .tick()
        .raw()
        .saturating_sub(source_world_tick));
    receipt["control_calls"] = serde_json::json!(calls);
    receipt["alive"] = serde_json::json!(alive);
    receipt["cue_free"] = serde_json::json!(cue_free);
    receipt["source_unchanged"] = serde_json::json!(unchanged);
    receipt["saved_policy_base_unchanged"] = serde_json::json!(base_unchanged);
    receipt["prior_remained_off"] = serde_json::json!(prior_clear);
    receipt["prior_metrics"] = serde_json::to_value(runtime.semantic_prior_metrics())?;
    receipt["sampling_after"] = serde_json::to_value(runtime.foundation_training_sampling_state())?;
    receipt["nourished_meals"] = serde_json::json!(meals);
    receipt["food_outcomes"] = serde_json::to_value(outcomes.values().collect::<Vec<_>>())?;
    receipt["rows_file"] = serde_json::json!("rows.jsonl");
    receipt["rows_sha256"] = serde_json::json!(format!("{:x}", rows_hash.finalize()));
    // Release the pre-reserved capacity only for final durable publication.
    drop(final_headroom);
    let endpoint = preserve_endpoint(
        &mut runtime,
        &baseline,
        &original,
        &life,
        &save_path,
        &assets,
        &output,
        id,
        source_world_tick,
        request.evaluation_nonce,
        sampling_temperature,
        &digest(&serde_json::to_vec(&request)?),
        endpoint_seeds,
    );
    let endpoint_preserved = endpoint
        .as_ref()
        .is_ok_and(|value| zero_step_preservation_valid(value));
    // A terminal copy is accepted only after the native final closure binds
    // its exact frozen compiler/heads; death itself is not an integrity veto.
    if !alive && endpoint_preserved {
        base_unchanged = true;
    }
    receipt["saved_policy_base_unchanged"] = serde_json::json!(base_unchanged);
    match endpoint {
        Ok(value) => receipt["final_endpoint"] = value,
        Err(e) => receipt["final_endpoint_error"] = serde_json::json!(e.to_string()),
    }
    receipt["coverage_complete"] = serde_json::json!(matches!(
        stop,
        "decision_limit" | "world_tick_limit" | "natural_death" | "restore_only"
    ));
    receipt["measurement_valid"] = serde_json::json!(
        error.is_none()
            && base_unchanged
            && prior_clear
            && unchanged
            && cue_free
            && endpoint_preserved
    );
    receipt["retention_scope"] = serde_json::json!("Frozen-head feeding observation only; not memory causality, maze mastery or generalization");
    let complete = matches!(
        stop,
        "decision_limit" | "world_tick_limit" | "natural_death" | "restore_only"
    ) && error.is_none()
        && base_unchanged
        && prior_clear;
    receipt["qualified"] =
        serde_json::json!(qualified(complete, unchanged, cue_free, endpoint_preserved));
    receipt["retention_demonstrated"] = serde_json::json!(
        qualified(complete, unchanged, cue_free, endpoint_preserved) && alive && meals > 0
    );
    alife_storage::write(
        output.join("retention.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_retention_preserves_fixture_heads_private_bytes_and_rejects_teacher_before_gpu() {
        // Synthetic storage/admission fixture only. No GPU or biological claim.
        fn with_teacher(source: &Path) {
            portable_handoff::tests::private_fixture(source);
            let assets = source.join("world-assets");
            let (_, loaded) =
                crate::GpuDurableSaveManifest::open_loaded(source.join("world.json"), &assets)
                    .unwrap();
            let mut world = loaded.save.restore_headless_world().unwrap();
            alife_school::LanguageNursery::speak_in_world(
                &mut world,
                alife_core::OrganismId(1),
                alife_core::Vec3f::new(1.0, 0.0, 0.0),
                vec![alife_core::LanguageTokenId::new(8).unwrap()],
            )
            .unwrap();
            let save = alife_world::PortableSaveFile::from_headless_world(
                "retention-synthetic-cued",
                &world,
                loaded.save.config.clone(),
                loaded.save.assets.clone(),
                vec![],
            )
            .unwrap();
            crate::GpuDurableSaveManifest::publish_snapshot(
                source.join("world.json"),
                &assets,
                &save,
            )
            .unwrap();
            let path = source.join("cycle.json");
            let mut receipt: FoundationCycleReceipt =
                serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            receipt.world_ticks_elapsed = 0;
            fs::write(path, serde_json::to_vec(&receipt).unwrap()).unwrap();
        }
        let f = crate::foundation_critic_correction::tests::full_residual_correction_fixture_with_source(with_teacher);
        let checkpoint = migrate_current_training_checkpoint(
            &f.target,
            alife_training::CheckpointSeedsV1 {
                historical_freshness: "unknown".into(),
                excluded_ranges: vec![[1, u64::MAX]],
                pending_range: None,
            },
        )
        .unwrap();
        let current = f.root.join("retention-current.alife-checkpoint");
        let original = checkpoint.encode().unwrap();
        fs::write(&current, &original).unwrap();
        assert_eq!(
            read_current_training_checkpoint(&current).unwrap(),
            checkpoint
        );
        // Explicitly off for this serial, device-free refusal test; restore caller settings.
        let prior = std::env::var_os("ALIFE_SLM_PRIOR");
        let backend = std::env::var_os("ALIFE_SLM_PRIOR_BACKEND");
        std::env::set_var("ALIFE_SLM_PRIOR", "off");
        std::env::set_var("ALIFE_SLM_PRIOR_BACKEND", "off");
        let output = f.root.join("retention-evidence");
        let result = evaluate_saved_life_retention(
            &current,
            &output,
            SavedLifeRetentionRequest {
                schema: 2,
                evaluation_nonce: 71,
                max_decisions: 2,
                max_world_ticks: 8,
                deadline_unix_seconds: now().unwrap() + 600,
                max_output_bytes: 512 * 1024 * 1024,
            },
        );
        let evidence_before = fs::read(output.join("retention.json")).unwrap();
        let request = SavedLifeRetentionRequest {
            schema: 2,
            evaluation_nonce: 71,
            max_decisions: 2,
            max_world_ticks: 8,
            deadline_unix_seconds: now().unwrap() + 600,
            max_output_bytes: 512 * 1024 * 1024,
        };
        assert!(evaluate_saved_life_retention(&current, &output, request.clone()).is_err());
        assert_eq!(
            fs::read(output.join("retention.json")).unwrap(),
            evidence_before
        );
        let corrupt = f.root.join("corrupt.alife-checkpoint");
        fs::write(&corrupt, b"not a Current checkpoint").unwrap();
        let corrupt_output = f.root.join("corrupt-evidence");
        assert!(evaluate_saved_life_retention(&corrupt, &corrupt_output, request).is_err());
        assert!(!corrupt_output.join("retention.json").exists());
        assert!(!corrupt_output.join("rows.jsonl").exists());
        assert_eq!(fs::read(&corrupt).unwrap(), b"not a Current checkpoint");
        for (key, value) in [
            ("ALIFE_SLM_PRIOR", prior),
            ("ALIFE_SLM_PRIOR_BACKEND", backend),
        ] {
            match value {
                Some(v) => std::env::set_var(key, v),
                None => std::env::remove_var(key),
            }
        }
        let receipt = result.unwrap();
        assert_eq!(
            receipt["status"],
            "rejected_existing_teacher_cues_before_gpu"
        );
        assert_eq!(receipt["neural_execution"], false);
        assert_eq!(receipt["qualified"], false);
        assert_eq!(receipt["source_unchanged"], true);
        assert!(!output.join("rows.jsonl").exists());
        assert_eq!(fs::read(&current).unwrap(), original);
        let restored = read_current_training_checkpoint(&current).unwrap();
        assert_eq!(restored.handoff.actor, checkpoint.handoff.actor);
        assert_eq!(restored.handoff.value, checkpoint.handoff.value);
        assert_eq!(restored.life, checkpoint.life);
        assert!(evaluate_saved_life_retention(
            &current,
            &output,
            SavedLifeRetentionRequest {
                schema: 2,
                evaluation_nonce: 71,
                max_decisions: 0,
                max_world_ticks: 8,
                deadline_unix_seconds: now().unwrap() + 600,
                max_output_bytes: 512 * 1024 * 1024,
            }
        )
        .is_err());
    }
    #[test]
    fn passive_teacher_label_is_observed_not_deleted_or_treated_as_emission() {
        let id = alife_core::OrganismId(17);
        let mut world = alife_world::HeadlessScenarioBuilder::new(71)
            .agent("learner", id, alife_core::Vec3f::ZERO)
            .build()
            .unwrap();
        let marker = world
            .editor_spawn_object(alife_world::WorldEditorSpawnSpec {
                label: "nursery-teacher".into(),
                kind: alife_world::WorldObjectKind::Obstacle,
                organism_id: None,
                position: alife_core::Vec3f::new(3.0, 0.0, 0.0),
                nutrition: 0.0,
                hazard_pain: 0.0,
                radius: 0.2,
                token_id: None,
            })
            .unwrap();
        let before = world.canonical_signature_digest().unwrap();
        let cues = TeacherCues::observe(&world);
        assert!(cues.clear());
        assert_eq!(cues.passive_teacher_object_ids, vec![marker.raw()]);
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
        assert!(world.entity(marker).is_some());
    }
    #[test]
    fn restore_only_is_explicitly_zero_zero_and_nonce_is_not_a_new_sampling_seed() {
        assert!(zero_step_preservation_valid(&serde_json::json!({})));
        assert!(!zero_step_preservation_valid(
            &serde_json::json!({"zero_step_equality":{"world_json_byteequal":false}})
        ));
        assert!(zero_step_preservation_valid(
            &serde_json::json!({"zero_step_equality":{"world_json_byteequal":true,"sleep_journal_byteequal":true,"private_asset_closure_byteequal":true,"saved_sampling_equal":true}})
        ));
        let r = SavedLifeRetentionRequest {
            schema: 2,
            evaluation_nonce: 71,
            max_decisions: 0,
            max_world_ticks: 0,
            deadline_unix_seconds: 200,
            max_output_bytes: 512 * 1024 * 1024,
        };
        r.validate(100).unwrap();
        let mut bad = r.clone();
        bad.max_world_ticks = 1;
        assert!(bad.validate(100).is_err());
        bad = r.clone();
        bad.max_decisions = 1;
        assert!(bad.validate(100).is_err());
        bad = r;
        bad.evaluation_nonce = 0;
        assert!(bad.validate(100).is_err());
    }
    #[test]
    fn saved_retention_request_is_finite_and_bounded() {
        let r = SavedLifeRetentionRequest {
            schema: 2,
            evaluation_nonce: 71,
            max_decisions: 128,
            max_world_ticks: 2048,
            deadline_unix_seconds: 200,
            max_output_bytes: 2 * 1024 * 1024 * 1024,
        };
        r.validate(100).unwrap();
        let mut unknown = serde_json::to_value(&r).unwrap();
        unknown["train"] = serde_json::json!(true);
        assert!(serde_json::from_value::<SavedLifeRetentionRequest>(unknown).is_err());
        for variant in 0..10 {
            let mut bad = r.clone();
            match variant {
                0 => bad.schema = 0,
                1 => bad.max_decisions = 0,
                2 => bad.max_decisions = 257,
                3 => bad.max_world_ticks = 8193,
                4 => bad.deadline_unix_seconds = 100,
                5 => bad.deadline_unix_seconds = 701,
                6 => bad.max_output_bytes = 1024,
                7 => bad.max_world_ticks = 0,
                8 => bad.max_output_bytes = 8 * 1024 * 1024 * 1024 + 1,
                _ => bad.deadline_unix_seconds = 0,
            }
            assert!(bad.validate(100).is_err());
        }
    }
    #[test]
    fn saved_retention_refuses_real_teacher_cues_without_editing_them() {
        let id = alife_core::OrganismId(17);
        let mut world = alife_world::HeadlessScenarioBuilder::new(71)
            .agent("learner", id, alife_core::Vec3f::ZERO)
            .build()
            .unwrap();
        assert!(TeacherCues::observe(&world).clear());
        alife_school::LanguageNursery::speak_in_world(
            &mut world,
            id,
            alife_core::Vec3f::new(1.0, 0.0, 0.0),
            vec![alife_core::LanguageTokenId::new(8).unwrap()],
        )
        .unwrap();
        let before = world.canonical_signature_digest().unwrap();
        let cues = TeacherCues::observe(&world);
        assert!(!cues.clear());
        assert!(cues.utterances > 0);
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
    }
    #[test]
    fn saved_retention_validity_is_independent_of_meals_and_requires_preserved_endpoint() {
        let mut metrics = crate::gpu_live_runtime::SemanticPriorMetrics {
            backend: "off".into(),
            ..Default::default()
        };
        assert!(prior_off(Some(&metrics)));
        metrics.requests = 1;
        assert!(!prior_off(Some(&metrics)));
        metrics.requests = 0;
        metrics.decision_frames_with_prior = 1;
        assert!(!prior_off(Some(&metrics)));
        assert!(qualified(true, true, true, true));
        assert!(!qualified(false, true, true, true));
        assert!(!qualified(true, false, true, true));
        assert!(!qualified(true, true, false, true));
        assert!(!qualified(true, true, true, false));
        assert!(qualified(true, true, true, true));
    }
}
