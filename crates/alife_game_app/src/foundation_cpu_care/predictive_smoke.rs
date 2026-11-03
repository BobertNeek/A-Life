//! Disposable comparison under the pilot's original sensor profile. This is not
//! normal care-source admission, a retained lineage receipt or a promotion path.
use super::*;
use alife_core::{BrainScaleTier, PhenotypeCompiler, TrainingStageManifest};
use alife_training::{
    cpu::predictive::artifact::{identity, ExperimentalPredictiveState},
    resolve_isolated_training_output, FoundationTrainingBackendV1, FoundationTrainingHandoffV1,
    PpoConfig,
};
use alife_world::{create_canonical_new_game_with_n2048_candidate, CanonicalNewGameConfig};
use std::{fs, path::Path, time::Instant};

const DOSE: usize = 16;
const TEMPERATURE: f32 = 1.;
const TRAIN_SEEDS: [u64; 4] = [41001, 41002, 41003, 41004];
const EVAL_SEEDS: [u64; 4] = [51001, 51002, 51003, 51004];

fn write_new(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.sync_all()?;
    Ok(())
}

fn lesson(index: usize) -> FoundationTeacherLesson {
    if index % 2 == 0 {
        FoundationTeacherLesson::Feeding
    } else {
        FoundationTeacherLesson::RelocatedFood
    }
}

fn bind(
    trainer: &mut CpuFoundationTrainer,
    source: &FoundationTrainingHandoffV1,
) -> Result<alife_core::PhenotypeCompilerInputs> {
    let before = trainer.checkpoint();
    let asset = trainer.export_candidate(TrainingStageManifest::new(1, 1, 1))?;
    let inputs = &source.compiler_inputs;
    let (phenotype, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        inputs.genome().clone(),
        inputs.development().clone(),
        asset.clone(),
    )?;
    trainer.rebind_for_next_cohort(phenotype, asset)?;
    let after = trainer.checkpoint();
    if before.weights != after.weights
        || before.first_moment != after.first_moment
        || before.second_moment != after.second_moment
        || before.update_ages != after.update_ages
        || before.optimizer_step != after.optimizer_step
        || before.config != after.config
        || before.stage_mask != after.stage_mask
    {
        return Err("disposable rebind changed learned/optimizer state".into());
    }
    Ok(inputs)
}

fn save_founder(
    output: &Path,
    actor: &mut CpuFoundationTrainer,
    value: &CpuValueHead,
    source: &FoundationTrainingHandoffV1,
    version: u64,
) -> Result<()> {
    let inputs = bind(actor, source)?;
    let handoff = FoundationTrainingHandoffV1::new(
        FoundationTrainingBackendV1::NativeCpu,
        version,
        inputs,
        actor.checkpoint(),
        value.checkpoint(),
    )?;
    // Explicitly experimental; no cpu-cycle.json or normal source-admission seal.
    write_new(output, &handoff)?;
    let restored: FoundationTrainingHandoffV1 = serde_json::from_slice(&fs::read(output)?)?;
    let admitted = restored.admit()?;
    if admitted.actor != actor.checkpoint() || admitted.value != value.checkpoint() {
        return Err("disposable founder failed exact cold restoration".into());
    }
    Ok(())
}

struct Captured {
    rows: Vec<CpuCollectedDecision>,
    phenotype: alife_core::CreaturePhenotype,
    metrics: Value,
}

fn capture(
    actor: &CpuFoundationTrainer,
    value: &CpuValueHead,
    _source: &FoundationTrainingHandoffV1,
    seed: u64,
    lesson: FoundationTeacherLesson,
    assessment: bool,
    output: &Path,
) -> Result<Captured> {
    let started = Instant::now();
    // Direct typed experimental source: no fabricated normal-care receipt,
    // historical-admission claim, sensor conversion or normal guard relaxation.
    if actor.phenotype().sensor_profile() != SensorProfile::GroundedObjectSlotsV1 {
        return Err(
            "this disposable pilot adapter requires its original object-slot sensors".into(),
        );
    }
    let before = actor.checkpoint();
    let critic_before = value.checkpoint();
    let asset = actor.export_candidate(TrainingStageManifest::new(1, 1, 1))?;
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = 71;
    config.sensor_profile = actor.phenotype().sensor_profile();
    let game = create_canonical_new_game_with_n2048_candidate(&config, &asset)?;
    let (_, mut episode) = prepare_care_episode(CpuCareRequest {
        previous: None,
        output: output.into(),
        seed,
        decisions: DOSE,
        temperature: TEMPERATURE,
        lesson,
        food_need_condition: FoundationFoodNeedCondition::NewbornV1,
        seed_history: None,
        known_exclusions: false,
        record_diagnostic_replay: false,
        fixture: true,
    })?;
    episode.assessment = assessment;
    // No semantic prior in either arm. Training exposes the ordinary teacher
    // cue through before_step; assessment removes it through existing hooks.
    episode.prior = None;
    let (mut host, ()) =
        CpuCanonicalHost::new_prepared_cohort(game, actor, seed as u32 ^ 0xa54ff53a, |world| {
            episode.prepare_world(world)
        })?;
    fs::create_dir(output)?;
    fs::write(
        output.join("collection-policy.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    write_new(&output.join("collection-actor-checkpoint.json"), &before)?;
    write_new(
        &output.join("initial-value-checkpoint.json"),
        &critic_before,
    )?;
    let mut rows = Vec::new();
    let result = (|| -> Result<()> {
        for _ in 0..1024 {
            episode.before_step(&mut host, actor)?;
            let step = host.step_with_preparation(actor, TEMPERATURE, |draft, sequence| {
                episode.prepare_draft(draft, sequence)
            })?;
            let before_food = episode.summary.as_ref().unwrap().clone();
            episode.after_step(&host, actor, &step)?;
            if let Some(trace) = episode
                .traces
                .last_mut()
                .filter(|_| !step.decisions.is_empty())
            {
                let after = episode.summary.as_ref().unwrap();
                trace["acquisition_event"] = json!(
                    after.ordinary_acquisition_events > before_food.ordinary_acquisition_events
                );
                trace["consumption_event"] =
                    json!(after.consumption_events > before_food.consumption_events);
                trace["moved"] = json!(teacher_outcome_moved(step.decisions[0].patch.outcome()));
                trace["blocked"] =
                    json!(teacher_outcome_blocked(step.decisions[0].patch.outcome()));
            }
            rows.extend(step.decisions);
            if rows.len() == DOSE + 1 {
                break;
            }
            if rows.len() > DOSE + 1
                || host
                    .world()
                    .organism_registry()
                    .iter()
                    .any(|r| !r.lifecycle().is_alive())
            {
                return Err("disposable care dose interrupted; prefix retained".into());
            }
        }
        if rows.len() != DOSE + 1 {
            return Err("disposable care dose incomplete; prefix retained".into());
        }
        Ok(())
    })();
    write_cpu_private_checkpoint(output, &host.checkpoint(actor)?)?;
    for (i, trace) in episode.traces.iter_mut().enumerate() {
        trace["bootstrap_row"] = json!(i >= DOSE);
        trace["loss_row"] = json!(!assessment && i < DOSE);
        trace["evaluation_row"] = json!(assessment && i < DOSE);
    }
    write_new(&output.join("raw-care-trace.json"), &episode.traces)?;
    // Exact decisions are retained compressed, including failures and bootstrap.
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("raw-native-decisions.json.zst"))?;
    let mut encoder = zstd::stream::write::Encoder::new(file, 3)?;
    serde_json::to_writer(&mut encoder, &rows)?;
    encoder.finish()?.sync_all()?;
    result?;
    if actor.checkpoint() != before || value.checkpoint() != critic_before {
        return Err("collection changed offline learner state".into());
    }
    let scored = &episode.traces[..DOSE];
    let count = |field: &str| scored.iter().filter(|row| row[field] == true).count();
    let mut actions = BTreeMap::<String, usize>::new();
    for row in scored {
        *actions.entry(row["action"].to_string()).or_default() += 1;
    }
    let success = rows[..DOSE]
        .iter()
        .filter(|r| r.patch.outcome().success)
        .count();
    let metrics = json!({"seed":seed,"lesson":lesson,"assessment":assessment,
        "decisions":DOSE,"bootstrap_decisions":1,"successes":success,"failures":DOSE-success,
        "nourished_meals":count("nourished_meal"),"acquisitions":count("acquisition_event"),
        "consumptions":count("consumption_event"),"selected_actions":actions,
        "moved":count("moved"),"blocked":count("blocked"),
        "teacher_cue_frames_all_including_bootstrap":episode.cue_frames,
        "semantic_prior_enabled":false,"world_tick":host.world().tick().raw(),
        "collection_seconds":started.elapsed().as_secs_f64(),"offline_checkpoints_unchanged":true});
    write_new(&output.join("metrics.json"), &metrics)?;
    let phenotype = host
        .world()
        .organism_registry()
        .iter()
        .next()
        .ok_or("missing learner")?
        .phenotype()
        .clone();
    Ok(Captured {
        rows,
        phenotype,
        metrics,
    })
}

/// Fixed small exploratory trial; outputs are recoverable disposable-only files.
pub fn run_predictive_care_smoke(
    state_path: &Path,
    upstream_path: &Path,
    output: &Path,
) -> Result<Value> {
    let started = Instant::now();
    let output = resolve_isolated_training_output(output, [state_path, upstream_path])?;
    if output.exists() {
        return Err("smoke output must be fresh".into());
    }
    let state_bytes = fs::read(state_path)?;
    let upstream_bytes = fs::read(upstream_path)?;
    let state = ExperimentalPredictiveState::read(state_path)?;
    let upstream: Value = serde_json::from_slice(&upstream_bytes)?;
    let expected = upstream["horizons"]
        .as_array()
        .ok_or("upstream horizons missing")?
        .iter()
        .find(|h| h["horizon_ticks"] == 1)
        .and_then(|h| h["arms"].as_array())
        .and_then(|arms| arms.iter().find(|a| a["inputs"] == "full"))
        .ok_or("upstream must contain the preselected one-tick full arm")?;
    if expected["state_payload_sha256"] != json!(state.payload_sha256)
        || upstream["source_identity_sha256"] != json!(state.payload.source_sha256)
        || upstream["config"]["motion_coverage"] != true
        || upstream["config"]["seed"] != 71
    {
        return Err("experimental state/upstream fixed trial identity mismatch".into());
    }
    let upstream_seconds = upstream["elapsed_seconds"]
        .as_f64()
        .filter(|s| s.is_finite() && *s > 0.)
        .ok_or("missing upstream wall time")?;
    let source = state.payload.source.clone();
    let source_identity = identity(&source)?;
    let admitted = source.clone().admit()?;
    let mut baseline_checkpoint = admitted.actor;
    let trained_config = state.payload.actor.config;
    // Match downstream Adam configuration, preserving all source moments/ages.
    baseline_checkpoint.config.learning_rate = trained_config.learning_rate;
    if baseline_checkpoint.config != trained_config {
        return Err("matched downstream Adam differs beyond the explicit learning rate".into());
    }
    let baseline = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation,
        baseline_checkpoint,
    )?;
    let (pretrained, _) = state.into_disposable_founder()?;
    fs::create_dir_all(&output)?;
    write_new(
        &output.join("fixed-plan.json"),
        &json!({"disposable_only":true,
        "source_identity":source_identity,"state_identity":state.payload_sha256,
        "train_seeds":TRAIN_SEEDS,"eval_seeds":EVAL_SEEDS,"decisions":DOSE,
        "temperature":TEMPERATURE,"ppo_epochs_per_lesson":1,
        "schedule":["feeding","relocated_food","feeding","relocated_food"],
        "sensor_profile":"original-grounded-object-slots-v1","semantic_prior":false,
        "baseline_downstream_adam_config":trained_config,"upstream_seconds":upstream_seconds}),
    )?;
    let ppo = PpoConfig {
        epochs: 1,
        temperature: TEMPERATURE,
        ..PpoConfig::default()
    };
    let mut arms = Vec::new();
    for (name, mut actor) in [("untrained", baseline), ("pretrained", pretrained)] {
        let arm_output = output.join(name);
        fs::create_dir(&arm_output)?;
        let mut critic = CpuValueHead::from_checkpoint(source.value.clone())?;
        save_founder(
            &arm_output.join("initial-experimental-handoff.json"),
            &mut actor,
            &critic,
            &source,
            1,
        )?;
        let before_actor = actor.checkpoint();
        let mut evaluations = Vec::new();
        let mut updates = Vec::new();
        for phase in ["before", "after"] {
            if phase == "after" {
                for (index, seed) in TRAIN_SEEDS.into_iter().enumerate() {
                    bind(&mut actor, &source)?;
                    let captured = capture(
                        &actor,
                        &critic,
                        &source,
                        seed,
                        lesson(index),
                        false,
                        &arm_output.join(format!("train-{index}")),
                    )?;
                    let version = 2 + index as u64;
                    let before = actor.checkpoint();
                    let windows = cpu_training_windows(
                        &captured.rows,
                        None,
                        &actor,
                        &critic,
                        &captured.phenotype,
                        version,
                        ppo,
                        16,
                        0,
                    )?;
                    if windows.iter().map(|w| w.actor.batch.len()).sum::<usize>() != DOSE {
                        return Err("native care training dose mismatch".into());
                    }
                    let receipt = train_cpu_residual_cohort(
                        &mut actor,
                        &mut critic,
                        windows.len(),
                        windows.len(),
                        |i| Ok(windows[i].clone()),
                        ppo,
                        version,
                    )?;
                    if actor.optimizer_step() != before.optimizer_step + 1
                        || receipt.completed_epochs != 1
                        || receipt.stopped_for_kl
                    {
                        return Err("fixed native care update did not complete".into());
                    }
                    save_founder(
                        &arm_output.join(format!("after-lesson-{index}-experimental-handoff.json")),
                        &mut actor,
                        &critic,
                        &source,
                        version,
                    )?;
                    updates.push(json!({"behavior":captured.metrics,"update":receipt,
                        "weights_changed":actor.weights() != before.weights,
                        "actor_optimizer_step_before":before.optimizer_step}));
                }
            }
            for (index, seed) in EVAL_SEEDS.into_iter().enumerate() {
                let captured = capture(
                    &actor,
                    &critic,
                    &source,
                    seed,
                    lesson(index),
                    true,
                    &arm_output.join(format!("eval-{phase}-{index}")),
                )?;
                evaluations.push(json!({"phase":phase,"behavior":captured.metrics}));
            }
        }
        arms.push(json!({"arm":name,"initial_actor_step":before_actor.optimizer_step,
            "final_actor_step":actor.optimizer_step(),"final_critic_step":critic.checkpoint().optimizer_step,
            "training":updates,"evaluation":evaluations}));
    }
    if fs::read(state_path)? != state_bytes
        || fs::read(upstream_path)? != upstream_bytes
        || identity(&source)? != source_identity
    {
        return Err("input/source changed during disposable trial".into());
    }
    let elapsed = started.elapsed().as_secs_f64();
    let result = json!({"experiment":"fixed-disposable-native-care-smoke-v1","backend":"native-cpu",
        "upstream_capture_pretraining_and_controls_seconds":upstream_seconds,
        "downstream_load_finetune_evaluation_export_seconds":elapsed,
        "total_pipeline_seconds":upstream_seconds+elapsed,
        "wall_scope":"includes upstream capture/conversion/all pretraining controls/scoring and downstream evaluation/fine-tuning/local recovery output; excludes Cargo builds",
        "arms":arms,"inputs_unchanged":true,"accepted_state_written":false,
        "trained_artifacts_published":false,"promotion":"none","sample_scope":"one fixed exploratory trial; no test-selected checkpoint"});
    write_new(&output.join("result.json"), &result)?;
    Ok(result)
}
