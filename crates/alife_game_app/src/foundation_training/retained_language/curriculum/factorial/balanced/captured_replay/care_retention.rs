//! Retain real care experience while replaying witnessed language failures.
use super::*;

struct Care {
    row: CpuCollectedDecision,
    target: u32,
}
fn collect_care(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    c: LanguagePilotConfig,
    traces: &mut impl Write,
) -> Result<Vec<Care>> {
    let mut care = vec![];
    for variant in 0..2 {
        for case in 0..6 {
            let (mut h, scene) = if variant == 0 {
                unseen_transfer::prepared_fresh_case(actor, asset, c.seed, case)?
            } else {
                unseen_transfer::prepared_replay_case(actor, asset, c.seed, case)?
            };
            let seed = if variant == 0 {
                unseen_transfer::FRESH_SEEDS[case]
            } else {
                unseen_transfer::REPLAY_SEEDS[case]
            };
            for _ in 0..2 {
                h.step(actor, 1.)?;
            }
            unseen_transfer::place_case(h.world_mut(), scene, case)?;
            for tick in 0..24 {
                let before = owners(h.world(), scene);
                let row = h.step(actor, 1.)?.decisions.remove(0);
                let after = owners(h.world(), scene);
                if physical_event(&row, scene, before, after)?
                    .is_none_or(|e| e.lesson != Lesson::EatFood)
                    || row.patch.outcome().energy_delta.raw() <= 0.
                {
                    continue;
                }
                let target = candidate_mask(
                    row.patch.pre_action().perception(),
                    &[(HeadlessActionIds::EAT, scene.food)],
                );
                if target.count_ones() != 1 || target != 1 << row.action.representative {
                    continue;
                }
                if row
                    .patch
                    .pre_action()
                    .perception()
                    .sensory()
                    .language_context
                    .heard_tokens
                    .iter()
                    .flatten()
                    .any(|t| t.source_kind != UtteranceSourceKind::Creature)
                {
                    return Err("care replay heard an external command".into());
                }
                writeln!(
                    traces,
                    "{}",
                    json!({"phase":"genuine_silent_care_replay","world_seed":seed,"variant":variant,"case":case,"scene":scene,"tick":tick,"row":row,"owned_before":before,"owned_after":after,"actual_food_consumption":true,"actual_positive_energy_delta":row.patch.outcome().energy_delta,"no_external_speech":true,"native_target_mask":target,"biological_reward_unmodified":true})
                )?;
                care.push(Care { row, target });
                break;
            }
        }
    }
    if care.is_empty() {
        return Err("no genuine naturally sampled nourishing care row".into());
    }
    Ok(care)
}
fn care_gradient(actor: &CpuFoundationTrainer, care: &[Care]) -> Result<(Vec<f32>, Value)> {
    let mut sum = vec![0.; actor.weights().len()];
    let mut loss = 0f64;
    for sample in care {
        let q = readout::single(actor, &sample.row);
        let ev = actor.evaluate_replay(&q)?;
        let a = &sample.row.action;
        if sample.target != 1u32 << a.representative {
            return Err(
                "care imitation target is not the actual selected native candidate bitmask".into(),
            );
        }
        let objective = cpu_imitation_objective(
            &ev.candidate_logits[0],
            &ImitationExample {
                candidate_count: a.candidate_count,
                representative_mask: a.representative_mask,
                motor_masks: [0; 6],
                forced_slots: a.forced_slots.clone(),
                target: ImitationTarget {
                    representative: Some(sample.target),
                    motors: [None; 6],
                },
            },
            1.,
            1.,
        )?;
        let scale = 1. / care.len() as f32;
        let adj = objective
            .logit_gradients
            .iter()
            .map(|g| g * scale)
            .collect::<Vec<_>>();
        let g = actor.probe_replay_gradients(&q, &[adj])?;
        for (a, b) in sum.iter_mut().zip(g.gradients) {
            *a += b;
        }
        loss += f64::from(objective.loss * scale);
    }
    let norm = trainable_norm(actor, &sum);
    Ok((
        sum,
        json!({"actual_nourishing_eat_samples":care.len(),"native_success_imitation_loss":loss,"trainable_native_gradient_l2":norm,"only_actual_selected_successful_eat_candidate_supervised":true}),
    ))
}
fn trainable_norm(actor: &CpuFoundationTrainer, g: &[f32]) -> f64 {
    let mask = actor.checkpoint().stage_mask;
    g.iter()
        .enumerate()
        .filter(|(i, _)| mask.is_trainable(*i))
        .map(|(_, g)| f64::from(*g).powi(2))
        .sum::<f64>()
        .sqrt()
}
fn verify_care_gradient(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    care: &[Care],
) -> Result<Value> {
    let mut source_likelihood_checks = vec![];
    for sample in care {
        let q = readout::single(actor, &sample.row);
        let ev = actor.evaluate_replay(&q)?;
        let a = &sample.row.action;
        let objective = cpu_imitation_objective(
            &ev.candidate_logits[0],
            &ImitationExample {
                candidate_count: a.candidate_count,
                representative_mask: a.representative_mask,
                motor_masks: [0; 6],
                forced_slots: a.forced_slots.clone(),
                target: ImitationTarget {
                    representative: Some(sample.target),
                    motors: [None; 6],
                },
            },
            1.,
            1.,
        )?;
        if objective.loss > -a.old_joint_log_probability + 0.001 {
            return Err(
                "source care target likelihood contradicts actual sampled native joint action"
                    .into(),
            );
        }
        source_likelihood_checks.push(json!({"actual_selected_index":a.representative,"care_target_bitmask":sample.target,"native_representative_imitation_loss":objective.loss,"recorded_full_native_joint_negative_log_probability":-a.old_joint_log_probability}));
    }
    let (g, _) = care_gradient(actor, care)?;
    let before = actor.checkpoint();
    let i = (0..g.len())
        .filter(|i| before.stage_mask.is_trainable(*i))
        .max_by(|a, b| g[*a].abs().total_cmp(&g[*b].abs()))
        .ok_or("no care coordinate")?;
    let mut losses = vec![];
    for sign in [-1., 1.] {
        let mut cp = before.clone();
        cp.weights[i] += sign * 0.001;
        let fork =
            CpuFoundationTrainer::from_checkpoint(actor.phenotype().clone(), asset.clone(), cp)?;
        let (_, o) = care_gradient(&fork, care)?;
        losses.push(
            o["native_success_imitation_loss"]
                .as_f64()
                .ok_or("no care loss")?,
        );
    }
    let finite = (losses[1] - losses[0]) / 0.002;
    let analytical = f64::from(g[i]);
    let relative = (finite - analytical).abs() / analytical.abs().max(1e-6);
    if relative > 0.03 || analytical.abs() < 1e-5 || actor.checkpoint() != before {
        return Err(
            "actual care native gradient failed finite difference/source preservation".into(),
        );
    }
    Ok(
        json!({"source_care_target_likelihood_checks":source_likelihood_checks,"actual_care_coordinate":i,"native_analytical_gradient":analytical,"central_finite_difference":finite,"relative_error":relative,"epsilon":0.001,"source_unmodified":true,"numerical_probe_not_behavioral_learning":true}),
    )
}
#[derive(Clone, Copy)]
enum Mode {
    Failure,
    Care,
    Mixed,
    NoCommandMixed,
}
fn objective(
    actor: &CpuFoundationTrainer,
    g: &[Pair],
    n: &[Pair],
    care: &[Care],
    mode: Mode,
) -> Result<(Vec<f32>, Value)> {
    if matches!(mode, Mode::Failure) {
        return gradient(actor, g);
    }
    if matches!(mode, Mode::Care) {
        return care_gradient(actor, care);
    }
    let (fg, fo) = gradient(
        actor,
        if matches!(mode, Mode::NoCommandMixed) {
            n
        } else {
            g
        },
    )?;
    let (cg, co) = care_gradient(actor, care)?;
    let mixed = fg
        .iter()
        .zip(&cg)
        .map(|(a, b)| 0.5 * a + 0.5 * b)
        .collect::<Vec<_>>();
    let mask = actor.checkpoint().stage_mask;
    let dot = fg
        .iter()
        .zip(&cg)
        .enumerate()
        .filter(|(i, _)| mask.is_trainable(*i))
        .map(|(_, (a, b))| f64::from(*a) * f64::from(*b))
        .sum::<f64>();
    let cosine = dot / (trainable_norm(actor, &fg) * trainable_norm(actor, &cg)).max(1e-30);
    let norm = trainable_norm(actor, &mixed);
    Ok((
        mixed,
        json!({"language_failure":fo,"genuine_silent_care":co,"failure_weight":0.5,"care_weight":0.5,"trainable_gradient_cosine_failure_vs_care":cosine,"trainable_native_gradient_l2":norm}),
    ))
}
pub fn preflight_care_retention(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run(source, output, c, true)
}
pub fn compare_care_retention(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run(source, output, c, false)
}
fn run(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
    preflight_only: bool,
) -> Result<Value> {
    if c.llamacpp_translation
        || !c.experimental_word_bridge
        || c.rounds != 1
        || c.epochs != 4
        || c.decisions != 24
    {
        return Err("care replay requires scripted V21, rounds1 epochs4 decisions24".into());
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
        return Err("not unchanged public step432 actor".into());
    }
    let g = early_stop::saved_pairs(source, Arm::Grounded)?;
    let n = early_stop::saved_pairs(source, Arm::NoSpeech)?;
    let mut trace = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("care-native-trace.jsonl.zst"))?,
        3,
    )?;
    eprintln!("care-retention: collect natural silent nourishment in retired worlds");
    let care = collect_care(&original, &asset, c, &mut trace)?;
    trace.finish()?;
    let mut probe = CpuFoundationTrainer::from_checkpoint(
        original.phenotype().clone(),
        asset.clone(),
        original.checkpoint(),
    )?;
    action_mask(&mut probe)?;
    let modes = [
        ("FailureOnly4", Arm::Grounded, Mode::Failure),
        ("CareOnly4", Arm::NoSpeech, Mode::Care),
        ("MixedGrounded4", Arm::Grounded, Mode::Mixed),
        ("MixedNoCommand4", Arm::NoSpeech, Mode::NoCommandMixed),
    ];
    let mut pref = vec![];
    for (label, _, mode) in modes {
        let (_, o) = objective(&probe, &g, &n, &care, mode)?;
        if o["trainable_native_gradient_l2"]
            .as_f64()
            .is_none_or(|v| v <= 1e-5)
        {
            return Err("intended care/failure/mixed gradient is zero".into());
        }
        eprintln!("care-retention preflight {label}: {o}");
        pref.push(json!({"condition":label,"objective":o}));
    }
    let numerical_care_gradient = verify_care_gradient(&probe, &asset, &care)?;
    let preflight = json!({"kind":"native-genuine-care-retention-preflight-v1","source_actor_sha256":common,"source_step":original.optimizer_step(),"source_actor_unchanged":identity(&original.checkpoint())?==common,"actual_nourishing_care_samples":care.len(),"care_trace_sha256":hash(&fs::read(output.join("care-native-trace.jsonl.zst"))?),"actual_care_gradient_finite_difference":numerical_care_gradient,"no_forced_collection_actions_or_rewards":true,"no_parameter_update_or_scored_assessment":true,"conditions":pref});
    write_json(&output.join("preflight.json"), &preflight)?;
    if preflight_only {
        write_json(&output.join("results.json"), &preflight)?;
        return Ok(preflight);
    }
    let mut results = vec![];
    let mut specs = vec![("Untreated", Arm::Untreated, None)];
    specs.extend(modes.into_iter().map(|(a, b, c)| (a, b, Some(c))));
    for (label, arm, mode) in specs {
        let mut actor = CpuFoundationTrainer::from_checkpoint(
            original.phenotype().clone(),
            asset.clone(),
            original.checkpoint(),
        )?;
        let mut next = r.next_gradient.clone();
        let mut updates = vec![];
        if let Some(mode) = mode {
            action_mask(&mut actor)?;
            let frozen = actor.checkpoint();
            for epoch in 0..4 {
                let (grad, o) = objective(&actor, &g, &n, &care, mode)?;
                let norm = actor.apply_gradients(&grad)?;
                preserve_frozen(&frozen, &actor.checkpoint())?;
                updates.push(json!({"epoch":epoch,"step":actor.optimizer_step(),"objective":o,"gradient_norm":norm,"all_frozen_coordinates_exact":true}));
                next = grad;
            }
        }
        let current = if mode.is_some() {
            export(&mut actor, c.seed)?
        } else {
            asset.clone()
        };
        if label == "FailureOnly4" {
            if identity(&actor.checkpoint())?
                != "ab735a77671c3d7cd85e0b72ec039675974ed9ecb8d1af2ba04e374b7e2bb66e"
            {
                return Err("failure-only actor differs from preserved V22 Grounded4".into());
            }
            let old = raw["arms"]
                .as_array()
                .ok_or("no V21 arms")?
                .iter()
                .find(|a| a["arm"] == "grounded")
                .ok_or("no grounded source")?;
            if json!(updates) != json!(&old["updates"].as_array().ok_or("no source updates")?[..4])
            {
                return Err("failure-only updates changed".into());
            }
        }
        eprintln!("care-retention {label}: fresh native assessment and cold repeat");
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
            replay_early_stop_v1: false,
            care_retention_v1: true,
        };
        let cold = exact_reload_plan(
            &output.join(format!("recovery-{label}")),
            &actor,
            &current,
            RecoveredAssessment {
                config: c,
                arm,
                curriculum: Some(plan),
            },
            next,
        )?;
        results.push(json!({"condition":label,"arm":arm,"common_source_actor_sha256":common,"source_step":432,"actor_step":actor.optimizer_step(),"actor_sha256":identity(&actor.checkpoint())?,"updates":updates,"evaluation":cold["native_private_and_optimizer_hashes"]["assessment"]["evaluation"],"continuity":cold}));
    }
    let result = json!({"kind":"experimental-genuine-care-retention-v23","execution_domain":"native-cpu","config":c,"source_v21_results_sha256":identity(&raw)?,"held_out_world_seeds":unseen_transfer::CARE_SEEDS,"independent_seed_count":6,"pre_post_are_paired_not_independent":true,"evaluation_ticks":EVALUATION_TICKS,"teacher_and_player_cue_ticks":INSTRUCTION_TICKS,"models":results,"preflight":preflight,"elapsed_seconds":started.elapsed().as_secs_f64(),"semantic_prior_enabled":false,"teacher":"historical scripted grounded failures; new care experience is naturally sampled without external speech; no provider","limitations":["Four-update treatments share source, optimizer configuration, action mask and real silent care corpus. Mixed loss is fixed50/50, care-only is unscaled care CE; no parameter or optimizer reset.","Grounded/no-command failure corpora have3/11 rows; both retain earlier grounded learning. No-command mixture contains no external teacher tokens; natural self-speech is allowed.","Care uses retired6M/8M pre-sleep worlds with one native success at most per world and no sampling forks. New12M seeds/IDs reuse familiar layouts/words; one founder.","Fresh assessment cohorts retain actor/optimizer history, not original source private life. Paired induced sleep is not independent N12, spontaneous sleep or an awake-delay control.","No forced evaluation actions, altered cooldown, fabricated labels/transitions/rewards, source overwrite, default/live brain migration, promotion or Luna401 retry."]});
    write_json(&output.join("results.json"), &result)?;
    Ok(result)
}
