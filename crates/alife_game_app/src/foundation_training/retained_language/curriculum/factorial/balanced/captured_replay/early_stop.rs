//! Matched four/eight-update replay; no new labels or sampler exploration.
use super::*;
use std::io::{BufRead, BufReader};

const CORPUS_SHA256: &str = "2b473602c0f402d5a977292f423be18b35118ed47137b21fefa51f63fad21225";

pub(super) fn saved_pairs(source: &Path, arm: Arm) -> Result<Vec<Pair>> {
    let bytes = fs::read(source.join("native-trace.jsonl.zst"))?;
    if hash(&bytes) != CORPUS_SHA256 {
        return Err("not the preserved measured V21 witness corpus".into());
    }
    let reader = BufReader::new(zstd::stream::read::Decoder::new(bytes.as_slice())?);
    let mut pairs = vec![];
    for line in reader.lines() {
        let v: Value = serde_json::from_str(&line?)?;
        if serde_json::from_value::<Arm>(v["arm"].clone())? != arm {
            continue;
        }
        let failure: CpuCollectedDecision = serde_json::from_value(v["failure"].clone())?;
        let success: CpuCollectedDecision = serde_json::from_value(v["success_witness"].clone())?;
        let scene: Scene = serde_json::from_value(v["scene"].clone())?;
        let before = serde_json::from_value(v["failure_owners_before"].clone())?;
        let after = serde_json::from_value(v["failure_owners_after"].clone())?;
        let owned = serde_json::from_value(v["success_owners_after"].clone())?;
        let goal: Lesson = serde_json::from_value(v["requested"].clone())?;
        let target = u32::try_from(v["native_target_mask"].as_u64().ok_or("missing target")?)?;
        if failure.initial != success.initial
            || failure.replay_tick != success.replay_tick
            || failure.sampling_counter != success.sampling_counter
            || target != 1 << success.action.representative
            || physical_event(&success, scene, before, owned)?.is_none_or(|e| e.lesson != goal)
            || physical_event(&failure, scene, before, after)?.is_some_and(|e| e.lesson == goal)
        {
            return Err("stored witness is not a real input-matched physical correction".into());
        }
        pairs.push(Pair {
            failure,
            target,
            noun: usize::from(goal == Lesson::GetToy),
        });
    }
    if pairs.len() != if arm == Arm::Grounded { 3 } else { 11 } {
        return Err("changed measured witness count".into());
    }
    Ok(pairs)
}

// This is arithmetic accounting of actual native updates, not another scored actor.
fn zero_step_accounting(
    before: &FoundationTrainerCheckpoint,
    after: &FoundationTrainerCheckpoint,
) -> Result<Value> {
    let c = before.config;
    let (mut motion, mut momentum, mut decay) = (0f64, 0f64, 0f64);
    let mut changed = 0usize;
    for i in 0..before.weights.len() {
        if !before.stage_mask.is_trainable(i) {
            continue;
        }
        let m = c.beta1 * before.first_moment[i] + (1. - c.beta1) * 0.;
        let v = c.beta2 * before.second_moment[i] + (1. - c.beta2) * 0.;
        let age = after.update_ages[i];
        if after.first_moment[i].to_bits() != m.to_bits()
            || after.second_moment[i].to_bits() != v.to_bits()
            || age != before.update_ages[i] + 1
        {
            return Err("native zero-gradient moment/age accounting differs".into());
        }
        let mstep = c.learning_rate
            * ((m / (1. - c.beta1.powf(age as f32)))
                / ((v / (1. - c.beta2.powf(age as f32))).sqrt() + c.epsilon));
        let dstep = c.learning_rate * c.weight_decay * before.weights[i];
        let delta = f64::from(after.weights[i]) - f64::from(before.weights[i]);
        motion += delta * delta;
        momentum += f64::from(mstep).powi(2);
        decay += f64::from(dstep).powi(2);
        changed += usize::from(before.weights[i].to_bits() != after.weights[i].to_bits());
    }
    preserve_frozen(before, after)?;
    Ok(
        json!({"changed_action_weights":changed,"actual_native_weight_delta_l2":motion.sqrt(),"arithmetic_momentum_component_l2":momentum.sqrt(),"arithmetic_weight_decay_component_l2":decay.sqrt(),"zero_gradient_moments_and_integer_ages_verified":true,"not_a_separate_behavioral_decay_ablation":true}),
    )
}

struct Model {
    label: String,
    arm: Arm,
    actor: CpuFoundationTrainer,
    asset: FoundationWeightAsset,
    next: Vec<f32>,
    updates: Vec<Value>,
}

pub fn compare_witnessed_early_stop(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    if c.llamacpp_translation
        || !c.experimental_word_bridge
        || c.rounds != 1
        || c.epochs != 8
        || c.decisions != 24
    {
        return Err(
            "early-stop comparison requires scripted V21, rounds1 epochs8 decisions24".into(),
        );
    }
    let raw: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if raw["kind"] != "experimental-success-witnessed-failure-replay-v21"
        || raw["config"]["seed"] != c.seed
    {
        return Err("not matching measured public V21".into());
    }
    fs::create_dir(output)?;
    let started = Instant::now();
    let r = read_recovery(&source.join("recovery-Untreated"))?;
    let p = PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048())?;
    let asset = FoundationWeightAsset::decode_canonical(&r.foundation)?;
    let original = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor)?;
    let common = identity(&original.checkpoint())?;
    if common != "ed1fdfa8bda84f7b8514d6e335f6bbab8eb165964e4cd7fa445ebe00fbabee6d" {
        return Err("not unchanged measured V18 step432 source".into());
    }
    let pref: Value = serde_json::from_slice(&fs::read(source.join("preflight.json"))?)?;
    let mut models = vec![Model {
        label: "Untreated".into(),
        arm: Arm::Untreated,
        actor: CpuFoundationTrainer::from_checkpoint(
            original.phenotype().clone(),
            asset.clone(),
            original.checkpoint(),
        )?,
        asset: asset.clone(),
        next: r.next_gradient,
        updates: vec![],
    }];
    let mut training = vec![];
    for (arm, name) in [(Arm::Grounded, "Grounded"), (Arm::NoSpeech, "NoSpeech")] {
        eprintln!("early-stop {name}: restore identical measured witness data");
        let pairs = saved_pairs(source, arm)?;
        let mut actor = CpuFoundationTrainer::from_checkpoint(
            original.phenotype().clone(),
            asset.clone(),
            original.checkpoint(),
        )?;
        action_mask(&mut actor)?;
        let frozen = actor.checkpoint();
        let (_, preflight) = gradient(&actor, &pairs)?;
        let expected = pref["arms"]
            .as_array()
            .ok_or("no preflight")?
            .iter()
            .find(|v| v["arm"] == json!(arm))
            .ok_or("no matching preflight")?;
        if preflight != expected["corrective_gradient_before_training"] {
            return Err("preserved replay changed corrective gradient".into());
        }
        let mut updates = vec![];
        let mut early = None;
        let mut next = vec![];
        let mut zero_receipts = vec![];
        for epoch in 0..8 {
            let (g, objective) = gradient(&actor, &pairs)?;
            let before = actor.checkpoint();
            let norm = actor.apply_gradients(&g)?;
            preserve_frozen(&frozen, &actor.checkpoint())?;
            if arm == Arm::Grounded && epoch >= 4 {
                if objective["trainable_native_gradient_l2"] != 0.
                    || objective["active_corrective_rows"] != 0
                {
                    return Err("later grounded update has a fresh nonzero objective".into());
                }
                zero_receipts.push(zero_step_accounting(&before, &actor.checkpoint())?);
            }
            updates.push(json!({"epoch":epoch,"step":actor.optimizer_step(),"objective":objective,"gradient_norm":norm,"all_frozen_coordinates_exact":true}));
            next = g;
            if epoch == 3 {
                early = Some((actor.checkpoint(), next.clone()));
            }
        }
        let (early_cp, early_next) = early.ok_or("missing four-step point")?;
        let mut early_actor = CpuFoundationTrainer::from_checkpoint(
            original.phenotype().clone(),
            asset.clone(),
            early_cp.clone(),
        )?;
        let (_, early_objective) = gradient(&early_actor, &pairs)?;
        let mut zero_exact = Value::Null;
        if arm == Arm::Grounded {
            let mut zero = CpuFoundationTrainer::from_checkpoint(
                original.phenotype().clone(),
                asset.clone(),
                early_cp,
            )?;
            for _ in 0..4 {
                zero.apply_gradients(&vec![0.; zero.weights().len()])?;
            }
            if zero.checkpoint() != actor.checkpoint() {
                return Err("four extra zero-gradient AdamW steps do not reproduce G8".into());
            }
            zero_exact = json!(true);
        }
        let early_asset = export(&mut early_actor, c.seed)?;
        let late_asset = export(&mut actor, c.seed)?;
        let saved = read_recovery(&source.join(format!("recovery-{name}")))?;
        if saved.actor != actor.checkpoint()
            || saved.foundation != late_asset.encode_canonical()?
            || saved.next_gradient != next
        {
            return Err(
                "eight-step replay differs from preserved V21 actor/optimizer/gradient".into(),
            );
        }
        let old = raw["arms"]
            .as_array()
            .ok_or("no measured source arms")?
            .iter()
            .find(|v| v["arm"] == json!(arm))
            .ok_or("no measured source arm")?;
        if old["updates"] != json!(updates) {
            return Err("native update receipts differ from V21".into());
        }
        training.push(json!({"arm":arm,"identical_preserved_failure_rows":pairs.len(),"training_corpus_sha256":CORPUS_SHA256,"preflight":preflight,"objective_at_four_step_stop":early_objective,"all_eight_updates_exactly_reproduce_preserved_v21":true,"grounded_eight_exactly_four_plus_four_zero_gradient_adamw_steps":zero_exact,"zero_gradient_step_accounting":zero_receipts,"updates":updates}));
        models.push(Model {
            label: format!("{name}4"),
            arm,
            actor: early_actor,
            asset: early_asset,
            next: early_next,
            updates: updates[..4].to_vec(),
        });
        models.push(Model {
            label: format!("{name}8"),
            arm,
            actor,
            asset: late_asset,
            next,
            updates,
        });
    }
    write_json(&output.join("training-replay.json"), &training)?;
    let mut results = vec![];
    for model in models {
        eprintln!(
            "early-stop {}: fresh native assessment and cold repeat",
            model.label
        );
        let plan = AssessmentPlan {
            recovery_sleep: true,
            collect_utterance_timing: true,
            factorial_grounding: true,
            paired_request_focus: true,
            linear_readout: true,
            balanced_grounding_v1: false,
            balanced_grounding_v2: true,
            event_confirmed_speech_v1: true,
            unseen_report_transfer_v1: true,
            physical_before_speech_v1: true,
            captured_failure_replay_v1: true,
            replay_early_stop_v1: true,
            care_retention_v1: false,
        };
        let cold = exact_reload_plan(
            &output.join(format!("recovery-{}", model.label)),
            &model.actor,
            &model.asset,
            RecoveredAssessment {
                config: c,
                arm: model.arm,
                curriculum: Some(plan),
            },
            model.next,
        )?;
        results.push(json!({"condition":model.label,"arm":model.arm,"common_source_actor_sha256":common,"source_step":432,"actor_step":model.actor.optimizer_step(),"actor_sha256":identity(&model.actor.checkpoint())?,"updates":model.updates,"evaluation":cold["native_private_and_optimizer_hashes"]["assessment"]["evaluation"],"continuity":cold}));
    }
    let result = json!({"kind":"experimental-witnessed-four-vs-eight-replay-v22","execution_domain":"native-cpu","config":c,"source_v21_results_sha256":identity(&raw)?,"training_corpus_sha256":CORPUS_SHA256,"held_out_world_seeds":unseen_transfer::EARLY_STOP_SEEDS,"independent_seed_count":6,"pre_post_are_paired_not_independent":true,"evaluation_ticks":EVALUATION_TICKS,"teacher_and_player_cue_ticks":INSTRUCTION_TICKS,"models":results,"training":training,"elapsed_seconds":started.elapsed().as_secs_f64(),"semantic_prior_enabled":false,"teacher":"no new teaching or provider calls; replay exact preserved scripted-command/native-success data","limitations":["Matched four/eight arms within each condition use identical data and source history. Grounded/no-command have3/11 admitted pairs; no shuffled fresh-seed arm.","Six new seeds/IDs reuse familiar geometry and words, one founder, paired induced sleep, no spontaneous-sleep/awake-delay control.","Later grounded steps have zero fresh gradient but nonzero retained momentum and weight decay. Arithmetic components are measured; behavior is not separately tested with decay-only optimizer.","No forced evaluation action, altered cooldown, fabricated physical outcome/reward, source overwrite, live/default migration, promotion or Luna401 retry."]});
    write_json(&output.join("results.json"), &result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_zero_gradient_steps_keep_momentum_and_freeze_other_history() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let mut actor = new_actor_profile(seed, &asset, true).unwrap();
        action_mask(&mut actor).unwrap();
        actor
            .apply_gradients(&vec![1.; actor.weights().len()])
            .unwrap();
        for _ in 0..4 {
            let before = actor.checkpoint();
            actor
                .apply_gradients(&vec![0.; actor.weights().len()])
                .unwrap();
            let r = zero_step_accounting(&before, &actor.checkpoint()).unwrap();
            assert!(r["changed_action_weights"].as_u64().unwrap() > 0);
            assert!(
                r["arithmetic_momentum_component_l2"].as_f64().unwrap()
                    > r["arithmetic_weight_decay_component_l2"].as_f64().unwrap()
            );
        }
    }
}
