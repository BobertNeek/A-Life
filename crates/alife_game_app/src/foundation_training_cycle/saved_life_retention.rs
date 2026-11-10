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
    pub max_decisions: usize,
    pub max_world_ticks: u64,
    pub deadline_unix_seconds: u64,
    pub max_output_bytes: u64,
}

impl SavedLifeRetentionRequest {
    pub fn validate(&self, now: u64) -> Result<()> {
        if self.schema != 1
            || !(1..=256).contains(&self.max_decisions)
            || !(1..=8192).contains(&self.max_world_ticks)
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
    utterances: usize,
}
impl TeacherCues {
    fn observe(world: &alife_world::HeadlessWorld) -> Self {
        Self {
            object_ids: world
                .object_snapshots()
                .iter()
                .filter(|object| {
                    object.teacher_channel.is_some() || object.label == "nursery-teacher"
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

fn qualified(complete: bool, alive: bool, unchanged: bool, cue_free: bool, meals: usize) -> bool {
    complete && alive && unchanged && cue_free && meals > 0
}

/// Restore the exact admitted living individual and its saved sampling stream.
/// Global founder/critic weights, moments and ages are only read. Ordinary
/// waking/private learning, sleep and biology remain active; no source save is
/// published. Existing teacher cues cause refusal rather than world editing.
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
    let final_headroom = budget.reserve(1024 * 1024)?;
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
        "schema":1,"kind":"saved-life-retention-v1","request":request,
        "source":source,"source_sha256":source_sha,"source_world_tick":source_world_tick,
        "organism_id":id.raw(),"policy_version":baseline.handoff.policy_version,
        "actor_sha256":actor_sha,"value_sha256":value_sha,
        "actor_optimizer_step":baseline.handoff.actor.optimizer_step,
        "value_optimizer_step":baseline.handoff.value.optimizer_step,
        "offline_actor_updates":0,"offline_value_updates":0,"offline_optimizer_constructed":false,
        "ordinary_private_learning_active":true,"prior_enabled":false,"new_teacher_cues_emitted":0,
        "existing_teacher_cues":cues,"qualified":false,
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
    let mut runtime = GpuLiveBrainRuntime::restore_foundation_rehearsal(
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
            break "decision_limit";
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
        if calls >= request.max_world_ticks * 4 + 4 {
            break "control_call_limit";
        }
        if !TeacherCues::observe(runtime.world()).clear() {
            cue_free = false;
            break "teacher_cue_observed";
        }
        let before_tick = runtime.world().tick();
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
                "on_policy_log_probability":step.behavior.on_policy_log_probability()?});
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
            std::thread::yield_now();
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
    let base_unchanged =
        alive && runtime.live_foundation_compiler_inputs(id)? == &baseline.handoff.compiler_inputs;
    let prior_clear = prior_off(runtime.semantic_prior_metrics());
    let unchanged = fs::read(&source)? == original
        && actor_sha == digest(&serde_json::to_vec(&baseline.handoff.actor)?)
        && value_sha == digest(&serde_json::to_vec(&baseline.handoff.value)?);
    receipt["status"] = serde_json::json!("finished");
    receipt["stop_reason"] = serde_json::json!(stop);
    receipt["error"] = serde_json::json!(error);
    receipt["neural_execution"] = serde_json::json!(true);
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
    receipt["qualified"] = serde_json::json!(qualified(
        stop == "decision_limit" && error.is_none() && base_unchanged && prior_clear,
        alive,
        unchanged,
        cue_free,
        meals
    ));
    drop(final_headroom);
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
    fn saved_retention_request_is_finite_and_bounded() {
        let r = SavedLifeRetentionRequest {
            schema: 1,
            max_decisions: 128,
            max_world_ticks: 2048,
            deadline_unix_seconds: 200,
            max_output_bytes: 2 * 1024 * 1024 * 1024,
        };
        r.validate(100).unwrap();
        for variant in 0..7 {
            let mut bad = r.clone();
            match variant {
                0 => bad.schema = 0,
                1 => bad.max_decisions = 0,
                2 => bad.max_decisions = 257,
                3 => bad.max_world_ticks = 8193,
                4 => bad.deadline_unix_seconds = 100,
                5 => bad.deadline_unix_seconds = 701,
                _ => bad.max_output_bytes = 1024,
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
    fn saved_retention_requires_zero_prior_delivery_and_actual_complete_meal_evidence() {
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
        assert!(!qualified(true, true, true, true, 0));
        assert!(!qualified(false, true, true, true, 1));
        assert!(!qualified(true, false, true, true, 1));
        assert!(!qualified(true, true, false, true, 1));
        assert!(!qualified(true, true, true, false, 1));
        assert!(qualified(true, true, true, true, 1));
    }
}
