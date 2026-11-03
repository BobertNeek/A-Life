//! Explicit disposable teacher-supervised control, separate from the frozen-
//! decoder negative. Labels reuse the existing care teacher on actual frames.
use super::*;
use alife_core::{CompiledSynapseKind, DecoderHeadKind, WorldEntityId};
use alife_training::{ImitationExample, ImitationTarget, StageTrainableMask, TrainingSequence};

const TEACHER_EPOCHS: usize = 16;
const TEACHER_LR: f32 = 0.003;
const TEACHER_DOSE: usize = 64;

fn configure(actor: &mut CpuFoundationTrainer) -> Result<()> {
    let before = actor.checkpoint();
    let ids = actor.phenotype().synapses().iter().enumerate().filter_map(|(i,s)| {
        (matches!(s.kind(), CompiledSynapseKind::Recurrent)
            || matches!(s.kind(), CompiledSynapseKind::Decoder(d) if d.head()==DecoderHeadKind::ActionCandidate))
            .then_some(i as u32)
    }).collect::<Vec<_>>();
    let mut next = before.clone();
    next.stage_mask = StageTrainableMask::from_synapse_indices(actor.phenotype(), &ids)?;
    next.config.learning_rate = TEACHER_LR;
    actor.restore_checkpoint(next)?;
    let after = actor.checkpoint();
    if before.weights != after.weights
        || before.first_moment != after.first_moment
        || before.second_moment != after.second_moment
        || before.update_ages != after.update_ages
        || before.optimizer_step != after.optimizer_step
    {
        return Err("explicit disposable mask/config setup reset acquired state".into());
    }
    Ok(())
}

fn target(row: &CpuCollectedDecision, food: WorldEntityId) -> Result<Option<ImitationExample>> {
    let frame = row.patch.pre_action().perception();
    let Some(candidate) = crate::foundation_training::care_eat_teacher_candidate(frame, food)
    else {
        return Ok(None); // Food may already have been consumed. No invented label.
    };
    let example = ImitationExample {
        candidate_count: row.action.candidate_count,
        representative_mask: row.action.representative_mask,
        motor_masks: row.action.motor_masks,
        forced_slots: row.action.forced_slots.clone(),
        target: ImitationTarget {
            representative: Some(1 << candidate.candidate_index),
            motors: [None; 6],
        },
    };
    example.validate()?;
    Ok(Some(example))
}

fn sequence(actor: &CpuFoundationTrainer, rows: &[CpuCollectedDecision]) -> TrainingSequence {
    TrainingSequence {
        phenotype_hash: actor.phenotype().phenotype_hash(),
        initial: rows[0].initial.clone(),
        ticks: rows.iter().map(|r| r.replay_tick.clone()).collect(),
        burn_in_ticks: 0,
        memory_candidate_gain: actor
            .phenotype()
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    }
}

fn verify_update(
    before: &alife_training::FoundationTrainerCheckpoint,
    after: &alife_training::FoundationTrainerCheckpoint,
) -> Result<()> {
    if before.stage_mask != after.stage_mask
        || before.config != after.config
        || after.optimizer_step != before.optimizer_step + 1
    {
        return Err("teacher update changed optimizer identity/config/mask".into());
    }
    for i in 0..before.weights.len() {
        if after.update_ages[i]
            != before.update_ages[i] + u32::from(before.stage_mask.is_trainable(i))
        {
            return Err("teacher update changed native integer age continuity".into());
        }
        if !before.stage_mask.is_trainable(i)
            && (before.weights[i].to_bits() != after.weights[i].to_bits()
                || before.first_moment[i].to_bits() != after.first_moment[i].to_bits()
                || before.second_moment[i].to_bits() != after.second_moment[i].to_bits())
        {
            return Err("teacher update modified a frozen coordinate".into());
        }
    }
    Ok(())
}

fn teacher_update(
    actor: &mut CpuFoundationTrainer,
    rows: &[CpuCollectedDecision],
    food: WorldEntityId,
) -> Result<Value> {
    let labels = rows
        .iter()
        .map(|r| target(r, food))
        .collect::<Result<Vec<_>>>()?;
    let count = labels.iter().flatten().count();
    if count == 0 {
        return Err("care teacher has no actual visible/held Eat candidate".into());
    }
    let before = actor.checkpoint();
    let mut gradients = vec![0.; before.weights.len()];
    let mut loss = 0.;
    for (part, targets) in rows.chunks(16).zip(labels.chunks(16)) {
        let seq = sequence(actor, part);
        let evaluated = actor.evaluate_replay(&seq)?;
        let mut adjoints = Vec::new();
        for (i, label) in targets.iter().enumerate() {
            if let Some(label) = label {
                let objective = cpu_imitation_objective(
                    &evaluated.candidate_logits[i],
                    label,
                    TEMPERATURE,
                    1.,
                )?;
                loss += f64::from(objective.loss) / count as f64;
                adjoints.push(
                    objective
                        .logit_gradients
                        .into_iter()
                        .map(|g| g / count as f32)
                        .collect(),
                );
            } else {
                adjoints.push(vec![0.; part[i].replay_tick.candidates.len()]);
            }
        }
        let probe = actor.probe_replay_gradients(&seq, &adjoints)?;
        for (g, p) in gradients.iter_mut().zip(probe.gradients) {
            *g += p;
        }
    }
    let ingest_gradient=actor.phenotype().synapses().iter().enumerate().filter(|(_,s)| {
        matches!(s.kind(),CompiledSynapseKind::Decoder(d) if d.head()==DecoderHeadKind::ActionCandidate && d.family()==alife_core::CandidateActionFamily::Ingest)
    }).map(|(i,_)|f64::from(gradients[i]).powi(2)).sum::<f64>().sqrt();
    let norm = actor.apply_gradients(&gradients)?;
    let after = actor.checkpoint();
    verify_update(&before, &after)?;
    let changed_ingest=actor.phenotype().synapses().iter().enumerate().filter(|(i,s)| {
        matches!(s.kind(),CompiledSynapseKind::Decoder(d) if d.head()==DecoderHeadKind::ActionCandidate && d.family()==alife_core::CandidateActionFamily::Ingest)
            && before.weights[*i].to_bits()!=after.weights[*i].to_bits()
    }).count();
    Ok(
        json!({"labelled_actual_rows":count,"teacher_loss_before":loss,"gradient_norm":norm,
        "ingest_action_decoder_gradient_l2":ingest_gradient,"ingest_action_decoder_coordinates_changed":changed_ingest,
        "optimizer_step_before":before.optimizer_step,"optimizer_step_after":after.optimizer_step}),
    )
}

fn trace_food(output: &Path) -> Result<WorldEntityId> {
    let rows: Value = serde_json::from_slice(&fs::read(output.join("raw-care-trace.json"))?)?;
    Ok(WorldEntityId(
        rows[0]["lesson_target"]
            .as_u64()
            .ok_or("trace target missing")?,
    ))
}

fn selection_metrics(
    actor: &CpuFoundationTrainer,
    rows: &[CpuCollectedDecision],
    food: WorldEntityId,
) -> Result<Value> {
    let mut probs = Vec::new();
    let mut selected = 0;
    let mut bundled = 0;
    let mut manipulation = 0;
    let mut error = 0_f32;
    for part in rows.chunks(16) {
        let evaluation = actor.evaluate_replay(&sequence(actor, part))?;
        for (i, row) in part.iter().enumerate() {
            let logits = &evaluation.candidate_logits[i];
            error = error.max(
                (logits[row.action.representative as usize]
                    - row.patch.decision().neural_evidence()?.logit)
                    .abs(),
            );
            let maximum = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let sum = logits
                .iter()
                .map(|x| ((x - maximum) / TEMPERATURE).exp())
                .sum::<f32>();
            let candidate = row
                .patch
                .pre_action()
                .perception()
                .candidates()
                .iter()
                .find(|c| {
                    c.target.entity == Some(food)
                        && c.family == alife_core::CandidateActionFamily::Ingest
                });
            if let Some(c) = candidate {
                probs.push(
                    ((logits[c.candidate_index as usize] - maximum) / TEMPERATURE).exp() / sum,
                );
                selected += usize::from(row.action.representative == c.candidate_index);
                manipulation +=
                    usize::from(row.action.motor_candidates[2] == Some(c.candidate_index));
            }
            bundled += usize::from(row.patch.selected_bundle().is_some_and(|b| {
                b.channels.iter().any(|c| {
                    c.channel == alife_core::MotorChannel::Manipulation
                        && c.primitive == alife_world::HeadlessActionIds::EAT
                        && c.target.and_then(|t| t.entity) == Some(food)
                })
            }));
        }
    }
    if error > 1e-4 {
        return Err("care replay logits differ from actual native collection".into());
    }
    Ok(
        json!({"rows":rows.len(),"ingest_candidate_rows":probs.len(),
        "ingest_probability_mean":if probs.is_empty(){0.}else{probs.iter().map(|x|f64::from(*x)).sum::<f64>()/probs.len() as f64},
        "ingest_probability_max":probs.into_iter().fold(0_f32,f32::max),"ingest_representative_selections":selected,
        "ingest_manipulation_selections":manipulation,"ingest_bundled":bundled,"max_native_replay_logit_error":error}),
    )
}

fn positive_control(reference: &Path, case: usize) -> Result<Value> {
    let root = reference.join("untrained");
    let dir = root.join(format!("eval-after-{case}"));
    let h: FoundationTrainingHandoffV1 = serde_json::from_slice(&fs::read(
        root.join("after-lesson-3-experimental-handoff.json"),
    )?)?;
    let a = h.admit()?;
    let actor = CpuFoundationTrainer::from_checkpoint(a.phenotype, a.foundation, a.actor)?;
    // Restore a disposable copy only. Never continue the source individual.
    let mut host = CpuCanonicalHost::from_checkpoint(read_cpu_private_checkpoint(&dir)?, &actor)?;
    let food = trace_food(&dir)?;
    let (organism, body) = host.world().organism_entity_ids()[0];
    let mut receipts = Vec::new();
    for _ in 0..2 {
        let world = host.world_mut();
        let before = *world
            .organism_registry()
            .get(organism)
            .ok_or("control learner missing")?
            .biochemistry();
        let draft = world.perception_frame_draft(
            organism,
            world.tick(),
            actor.phenotype().sensor_profile(),
            before.homeostasis,
        )?;
        let candidate = draft
            .candidates()
            .iter()
            .find(|c| {
                c.action_id == alife_world::HeadlessActionIds::EAT && c.target.entity == Some(food)
            })
            .ok_or("positive control lacks real Eat candidate")?;
        let command = candidate.to_command(organism, alife_core::Confidence::new(1.)?)?;
        let receipt = world.apply_registered_command(
            &command,
            body,
            alife_core::Tick(world.tick().raw() + 1),
        )?;
        world.try_advance_tick()?;
        receipts.push(json!({"contact":receipt.action_result.execution.physical.contact,"succeeded":receipt.action_result.execution.succeeded,
            "energy_before":receipt.biology_before.body.energy,"energy_after":receipt.biology_after.body.energy}));
        if receipt.action_result.execution.physical.contact
            == alife_core::PhysicalContactKind::Consumed
        {
            if receipt.biology_after.body.energy <= receipt.biology_before.body.energy {
                return Err("manual native Eat did not nourish".into());
            }
            return Ok(
                json!({"manual_control_not_learned_behavior":true,"case":case,"receipts":receipts}),
            );
        }
    }
    Err("native direct-action positive control failed to consume".into())
}

pub fn run_predictive_care_teacher_control(
    state_path: &Path,
    upstream_path: &Path,
    reference: &Path,
    output: &Path,
    check_only: bool,
) -> Result<Value> {
    let started = Instant::now();
    let output = resolve_isolated_training_output(output, [state_path, upstream_path, reference])?;
    if output.exists() {
        return Err("teacher-control output must be fresh".into());
    }
    let state_bytes = fs::read(state_path)?;
    let upstream_bytes = fs::read(upstream_path)?;
    let state = ExperimentalPredictiveState::read(state_path)?;
    let source = state.payload.source.clone();
    let upstream: Value = serde_json::from_slice(&upstream_bytes)?;
    let ref_plan: Value = serde_json::from_slice(&fs::read(reference.join("fixed-plan.json"))?)?;
    if ref_plan["state_identity"] != json!(state.payload_sha256)
        || ref_plan["source_identity"] != json!(state.payload.source_sha256)
        || upstream["source_identity_sha256"] != json!(state.payload.source_sha256)
    {
        return Err("teacher-control source identities mismatch".into());
    }
    let a = source.clone().admit()?;
    let mut baseline = CpuFoundationTrainer::from_checkpoint(a.phenotype, a.foundation, a.actor)?;
    let (mut pretrained, _) = state.into_disposable_founder()?;
    bind(&mut baseline, &source)?;
    bind(&mut pretrained, &source)?;
    configure(&mut baseline)?;
    configure(&mut pretrained)?;
    if baseline.checkpoint().stage_mask != pretrained.checkpoint().stage_mask
        || baseline.checkpoint().config != pretrained.checkpoint().config
    {
        return Err("teacher-control masks/configs differ".into());
    }
    let positive = [
        positive_control(reference, 0)?,
        positive_control(reference, 1)?,
    ];
    let mut checks = Vec::new();
    for (name, actor) in [("untrained", &baseline), ("pretrained", &pretrained)] {
        let dir = reference.join(name).join("train-0");
        let rows: Vec<CpuCollectedDecision> =
            serde_json::from_reader(zstd::stream::read::Decoder::new(fs::File::open(
                dir.join("raw-native-decisions.json.zst"),
            )?)?)?;
        let h: FoundationTrainingHandoffV1 = serde_json::from_slice(&fs::read(
            reference
                .join(name)
                .join("initial-experimental-handoff.json"),
        )?)?;
        let original = h.admit()?;
        let mut copy = CpuFoundationTrainer::from_checkpoint(
            original.phenotype,
            original.foundation,
            actor.checkpoint(),
        )?;
        let before = copy.checkpoint();
        let check = teacher_update(&mut copy, &rows[..16], trace_food(&dir)?)?;
        if check["ingest_action_decoder_coordinates_changed"]
            .as_u64()
            .unwrap_or(0)
            == 0
            || check["ingest_action_decoder_gradient_l2"]
                .as_f64()
                .unwrap_or(0.)
                <= 0.
        {
            return Err("targeted native decoder correctness check failed".into());
        }
        if actor.checkpoint() != before {
            return Err("preflight mutated the real experimental learner".into());
        }
        checks.push(json!({"arm":name,"read_only_source_copy_update":check}));
    }
    fs::create_dir_all(&output)?;
    eprintln!("Native direct-action controls and disposable decoder-update checks passed.");
    write_new(
        &output.join("correctness-checks.json"),
        &json!({"native_positive_controls":positive,"copy_update_checks":checks}),
    )?;
    if check_only {
        return Ok(
            json!({"check_only":true,"passed":true,"elapsed_seconds":started.elapsed().as_secs_f64()}),
        );
    }
    write_new(
        &output.join("fixed-plan.json"),
        &json!({"experiment":"disposable-native-care-teacher-control-v1","disposable_only":true,
        "train_seeds":[41001,41002],"eval_seeds":[51001,51002],"decisions":TEACHER_DOSE,"teacher_epochs":TEACHER_EPOCHS,
        "learning_rate":TEACHER_LR,"temperature":TEMPERATURE,"mask":"native recurrent + ActionCandidate decoder only",
        "teaching":"existing grounded care Eat demonstrator on actual perception; offline native imitation",
        "prior":"unsupported profile; not used or claimed","eval":"teacher/prior removed","source_identity":state.payload.source_sha256,"state_identity":state.payload_sha256}),
    )?;
    let mut arms = Vec::new();
    for (name, mut actor) in [("untrained", baseline), ("pretrained", pretrained)] {
        let dir = output.join(name);
        fs::create_dir(&dir)?;
        let critic = CpuValueHead::from_checkpoint(source.value.clone())?;
        save_founder(
            &dir.join("initial-experimental-handoff.json"),
            &mut actor,
            &critic,
            &source,
            1,
        )?;
        let initial_step = actor.optimizer_step();
        let mut evaluations = Vec::new();
        let mut training = Vec::new();
        for phase in ["before", "after"] {
            if phase == "after" {
                for case in 0..2 {
                    let lesson = if case == 0 {
                        FoundationTeacherLesson::EatHeldFood
                    } else {
                        FoundationTeacherLesson::EatReachableFood
                    };
                    let episode = dir.join(format!("train-{case}"));
                    let c = capture(
                        &actor,
                        &critic,
                        41001 + case,
                        lesson,
                        false,
                        TEACHER_DOSE,
                        &episode,
                    )?;
                    let food = trace_food(&episode)?;
                    let probabilities = selection_metrics(&actor, &c.rows[..TEACHER_DOSE], food)?;
                    let mut curve = Vec::new();
                    for epoch in 0..TEACHER_EPOCHS {
                        let update = teacher_update(&mut actor, &c.rows[..TEACHER_DOSE], food)?;
                        curve.push(json!({"epoch":epoch+1,"update":update}));
                    }
                    save_founder(
                        &dir.join(format!("after-lesson-{case}-experimental-handoff.json")),
                        &mut actor,
                        &critic,
                        &source,
                        2 + case,
                    )?;
                    training.push(json!({"behavior":c.metrics,"collection_policy":probabilities,"teacher_updates":curve}));
                }
            }
            for case in 0..2 {
                let lesson = if case == 0 {
                    FoundationTeacherLesson::EatHeldFood
                } else {
                    FoundationTeacherLesson::EatReachableFood
                };
                let episode = dir.join(format!("eval-{phase}-{case}"));
                let c = capture(
                    &actor,
                    &critic,
                    51001 + case,
                    lesson,
                    true,
                    TEACHER_DOSE,
                    &episode,
                )?;
                evaluations.push(json!({"phase":phase,"behavior":c.metrics,"selection":selection_metrics(&actor,&c.rows[..TEACHER_DOSE],trace_food(&episode)?)?}));
            }
        }
        if critic.checkpoint() != source.value {
            return Err("imitation modified the source critic".into());
        }
        arms.push(json!({"arm":name,"initial_actor_step":initial_step,"final_actor_step":actor.optimizer_step(),
            "critic_unchanged":true,"training":training,"evaluation":evaluations}));
    }
    if fs::read(state_path)? != state_bytes || fs::read(upstream_path)? != upstream_bytes {
        return Err("teacher control changed original inputs".into());
    }
    let seconds = started.elapsed().as_secs_f64();
    let result = json!({"experiment":"disposable-native-care-teacher-control-v1","backend":"native-cpu","arms":arms,
        "elapsed_seconds":seconds,"upstream_prediction_seconds":upstream["elapsed_seconds"],"includes":"native direct controls/copy-update checks/collection/teacher training/evaluation/local recovery; Cargo builds separate",
        "accepted_state_written":false,"trained_artifacts_published":false,"promotion":"none"});
    write_new(&output.join("result.json"), &result)?;
    Ok(result)
}
