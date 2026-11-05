//! Sixteen real decisions and sixteen read-only gradients, never optimizer steps.
use super::super::token_probe::speech;
use super::*;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
    path::PathBuf,
};

const TEST: &str = "foundation_training::retained_language::curriculum::factorial::balanced::bounded_tape_probe::diagnose_two_frozen_witnesses";

pub(super) fn source_actor(
    source: &Path,
    label: &str,
) -> Result<(CpuFoundationTrainer, FoundationWeightAsset)> {
    let r = read_recovery(&source.join(format!("recovery-{label}")))?;
    let p = PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048())?;
    let asset = FoundationWeightAsset::decode_canonical(&r.foundation)?;
    let actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor)?;
    Ok((actor, asset))
}
pub(super) fn write_compressed(path: &Path, v: &impl Serialize) -> Result<()> {
    let f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let mut w = zstd::stream::write::Encoder::new(f, 3)?;
    serde_json::to_writer(&mut w, v)?;
    w.finish()?;
    Ok(())
}
fn previous_rows(source: &Path, label: &str) -> Result<BTreeMap<(u64, usize), Vec<Value>>> {
    let f = fs::File::open(source.join(format!("recovery-{label}/reloaded-evaluation.jsonl.zst")))?;
    let mut rows = BTreeMap::<(u64, usize), Vec<Value>>::new();
    for line in BufReader::new(zstd::stream::read::Decoder::new(f)?).lines() {
        let e: Value = serde_json::from_str(&line?)?;
        if e["phase"] != "balanced_readout_evaluation" || e["post_sleep"] != false {
            continue;
        }
        let seed = e["world_seed"].as_u64().ok_or("missing seed")?;
        if ![16_309_347, 16_508_317].contains(&seed) {
            continue;
        }
        let request = match e["requested"].as_str() {
            Some("EatFood") => 0,
            Some("GetFood") => 1,
            _ => continue,
        };
        let a = rows.entry((seed, request)).or_default();
        if a.len() < 2 {
            a.push(e);
        }
    }
    if rows.len() != 4 || rows.values().any(|a| a.len() != 2) {
        return Err("missing witnessed prefix".into());
    }
    Ok(rows)
}
fn gradient_groups(actor: &CpuFoundationTrainer, g: &[f32]) -> Value {
    let cp = actor.checkpoint();
    let mut raw = [0f64; 4];
    let mut trainable = [0f64; 4];
    let mut active = [0usize; 4];
    for (i, s) in actor.phenotype().synapses().iter().enumerate() {
        let k = match s.kind() {
            CompiledSynapseKind::Recurrent => 0,
            CompiledSynapseKind::Decoder(d) if d.head() == DecoderHeadKind::ActionCandidate => 1,
            CompiledSynapseKind::Decoder(d) if d.head() == DecoderHeadKind::SpeechPayload => 2,
            _ => 3,
        };
        raw[k] += f64::from(g[i]).powi(2);
        if cp.stage_mask.is_trainable(i) {
            trainable[k] += f64::from(g[i]).powi(2);
            active[k] += usize::from(g[i] != 0.);
        }
    }
    json!({"order":["recurrent","action_head","speech_head","other"],"raw_l2":raw.map(f64::sqrt),"current_stage_trainable_l2":trainable.map(f64::sqrt),"nonzero_current_trainable_coordinates":active})
}
fn probe(
    actor: &CpuFoundationTrainer,
    q: &TrainingSequence,
    adj: Vec<f32>,
    target: Option<ReplaySpeechTarget>,
    count: &mut usize,
) -> Result<alife_training::TrainingGradientProbe> {
    if *count >= 16 {
        return Err("refusing gradient probe 17".into());
    }
    *count += 1;
    let out = PathBuf::from(std::env::var("ALIFE_BOUNDED_TAPE_OUT")?);
    write_json(
        &out.join(format!("probe-attempt-{count:02}.json")),
        &json!({"reserved_probe":count,"maximum":16,"target":target,"no_optimizer_step":true}),
    )?;
    let g = actor.probe_replay_gradients_with_speech(q, &[adj], &[target])?;
    write_compressed(
        &out.join(format!("native-probe-{count:02}.json.zst")),
        &json!({"objective":g.objective,"gradients":g.gradients}),
    )?;
    Ok(g)
}
fn dot_masked(actor: &CpuFoundationTrainer, a: &[f32], b: &[f32]) -> f64 {
    let cp = actor.checkpoint();
    a.iter()
        .zip(b)
        .enumerate()
        .filter(|(i, _)| cp.stage_mask.is_trainable(*i))
        .map(|(_, (a, b))| f64::from(*a) * f64::from(*b))
        .sum()
}
fn head_gate_direction(
    actor: &CpuFoundationTrainer,
    features: &[f32],
    g: &[f32],
    output: usize,
) -> f64 {
    let target = actor.phenotype().candidate_decoder().motor_start() as usize + 160;
    actor.phenotype().synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(d)if d.head()==DecoderHeadKind::SpeechPayload)&&s.target() as usize==target+output).map(|(i,s)|-f64::from(features[s.source() as usize])*f64::from(g[i])).sum()
}
fn verify_child(out: &Path, source: &Path) -> Result<()> {
    let index: Value = serde_json::from_slice(&fs::read(out.join("prefix-checkpoints.json"))?)?;
    let mut checks = vec![];
    for item in index.as_array().ok_or("bad prefix index")? {
        let label = item["condition"].as_str().ok_or("bad label")?;
        let (actor, _) = source_actor(source, label)?;
        let before = actor.checkpoint();
        let file = out.join(item["file"].as_str().ok_or("bad filename")?);
        let cp: CpuCanonicalCheckpoint =
            serde_json::from_reader(zstd::stream::read::Decoder::new(fs::File::open(file)?)?)?;
        assert_eq!(identity(&cp)?, item["host_sha256"]);
        let h = CpuCanonicalHost::from_checkpoint(cp, &actor)?;
        assert_eq!(identity(&h.checkpoint(&actor)?)?, item["host_sha256"]);
        assert_eq!(actor.checkpoint(), before);
        checks.push(json!({"condition":label,"file":item["file"],"host_exact_after_fresh_process_load":true,"actor_optimizer_exact":true}));
    }
    assert_eq!(checks.len(), 8);
    write_json(
        &out.join("fresh-process-state-verification.json"),
        &json!({"checks":checks,"native_decisions":0,"gradient_probes":0,"adam_or_shadow_adam_steps":0}),
    )
}

#[test]
#[ignore = "explicit bounded V24 witness diagnostic; requires SOURCE and new OUT; zero Adam"]
fn diagnose_two_frozen_witnesses() -> Result<()> {
    let source = PathBuf::from(std::env::var("ALIFE_BOUNDED_TAPE_SOURCE")?);
    let out = PathBuf::from(std::env::var("ALIFE_BOUNDED_TAPE_OUT")?);
    if std::env::var_os("ALIFE_BOUNDED_TAPE_VERIFY_ONLY").is_some() {
        return verify_child(&out, &source);
    }
    let old: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if old["kind"] != "experimental-frozen-command-causal-retention-v24"
        || old["config"]["seed"] != 847501u64
    {
        return Err("not measured public V24".into());
    }
    fs::create_dir(&out)?;
    let start = Instant::now();
    let mut scored = 0usize;
    let mut probes = 0usize;
    let mut records = vec![];
    let mut gradients = vec![];
    let mut checkpoints = vec![];
    let mut models = vec![];
    let mut trace = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out.join("native-tapes.jsonl.zst"))?,
        3,
    )?;
    for label in ["Untreated", "MixedGrounded4"] {
        let (actor, asset) = source_actor(&source, label)?;
        let before = actor.checkpoint();
        let before_hash = identity(&before)?;
        let expected = previous_rows(&source, label)?;
        let model = old["models"]
            .as_array()
            .ok_or("no models")?
            .iter()
            .find(|m| m["condition"] == label)
            .ok_or("missing model")?;
        assert_eq!(model["source_actor_sha256"], before_hash);
        for case in [3usize, 4] {
            let scene_case = frozen_causal::CASES[case];
            let (mut parent, s) =
                unseen_transfer::prepared_frozen_case(&actor, &asset, 847501, case)?;
            for _ in 0..2 {
                parent.step(&actor, 1.)?;
            }
            unseen_transfer::place_case(parent.world_mut(), s, scene_case.fixture)?;
            let id = parent.world().organism_entity_ids()[0].0;
            for requested in [Lesson::EatFood, Lesson::GetFood] {
                let previous = &expected[&(scene_case.seed, requested.index())];
                let mut h = parent.snapshot();
                // V24 before_audio/after_audio hash JSON Values, whose object-key
                // ordering differs from typed checkpoint serialization.
                let parent_state = frozen_causal::before_audio(&h, &actor)?;
                let parent_hash = identity(&parent_state)?;
                write_compressed(
                    &out.join(format!(
                        "parent-{label}-{}-{}.json.zst",
                        scene_case.seed,
                        requested.index()
                    )),
                    &parent_state,
                )?;
                assert_eq!(
                    previous[0]["causal_input_evidence"]["setup"]
                        ["before_audio_full_checkpoint_sha256"],
                    parent_hash
                );
                let mut pending = None::<Event>;
                let mut left = 0usize;
                let mut fulfilled = false;
                for (tick, prior) in previous.iter().enumerate().take(2) {
                    if scored >= 16 {
                        return Err("refusing scored decision 17".into());
                    }
                    if tick == 0 {
                        let pos = speech_position(h.world())?;
                        h.world_mut().emit_player_tokens(
                            Some(id),
                            pos,
                            requested
                                .words()
                                .into_iter()
                                .map(LanguageTokenId::new)
                                .collect::<std::result::Result<_, _>>()?,
                        )?;
                        assert_eq!(
                            identity(&frozen_causal::before_audio(&h, &actor)?)?,
                            prior["causal_input_evidence"]["setup"]
                                ["after_audio_full_checkpoint_sha256"]
                        );
                    }
                    let owned_before = owners(h.world(), s);
                    let held = owned_before[0] == Some(id);
                    let legal = h.world().can_grab_target(id, requested.target(s));
                    let ready = h.world().neural_speech_cooldown_ready(id, true);
                    let private_before = identity(&h.checkpoint(&actor)?)?;
                    write_json(
                        &out.join(format!("decision-attempt-{:02}.json", scored + 1)),
                        &json!({"reserved_scored_decision":scored+1,"maximum":16,"condition":label,"seed":scene_case.seed,"requested":requested,"step":tick,"pre_private_sha256":private_before}),
                    )?;
                    let step = h.step(&actor, 1.)?;
                    scored += 1;
                    assert_eq!(step.decisions.len(), 1);
                    let row = &step.decisions[0];
                    write_compressed(
                        &out.join(format!("native-decision-{scored:02}.json.zst")),
                        &json!({"condition":label,"seed":scene_case.seed,"requested":requested,"step":tick,"full_native_row":row,"full_host_after":h.checkpoint(&actor)?}),
                    )?;
                    let owned_after = owners(h.world(), s);
                    no_teacher_or_prior(row)?;
                    assert_eq!(identity(row)?, prior["native_full_row_sha256"]);
                    assert_eq!(serde_json::to_value(owned_before)?, prior["owned_before"]);
                    assert_eq!(serde_json::to_value(owned_after)?, prior["owned_after"]);
                    let private_after = identity(&h.checkpoint(&actor)?)?;
                    let q = readout::single(&actor, row);
                    let ev = actor.evaluate_replay(&q)?;
                    let logits = &ev.candidate_logits[0];
                    assert_eq!(ev.final_activations[0], row.final_features);
                    assert_eq!(
                        sample_cpu_joint_action(
                            actor.phenotype(),
                            row.patch.pre_action().perception(),
                            logits,
                            scene_case.seed as u32,
                            row.sampling_counter.ok_or("no sampler counter")?,
                            row.action.temperature
                        )?,
                        row.action
                    );
                    let raw = speech(
                        &actor,
                        &ev.final_activations[0],
                        &q.ticks[0].effective_weight_offsets,
                    );
                    let payload = decode_cpu_speech(
                        &actor,
                        &ev.final_activations[0],
                        &q.ticks[0].effective_weight_offsets,
                    )?;
                    let target = confirmed_speech(pending.as_ref(), left, Some(requested));
                    assert_eq!(
                        serde_json::to_value(target)?,
                        prior["grounded_speech_target"]
                    );
                    let emitted = h
                        .world()
                        .audible_utterances()
                        .into_iter()
                        .filter(|u| {
                            u.source_kind == UtteranceSourceKind::Creature
                                && u.speaker_id == Some(id)
                                && u.emitted_tick == row.patch.pre_action().tick
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(serde_json::to_value(&emitted)?, prior["emitted"]);
                    let event = physical_event(row, s, owned_before, owned_after)?;
                    assert_eq!(serde_json::to_value(&event)?, prior["completed"]);
                    let candidates = row.patch.pre_action().perception().candidates();
                    let actions=candidates.iter().map(|c|json!({"index":c.candidate_index,"action_id":c.action_id,"kind":c.kind,"target":c.target,"logit":logits[c.candidate_index as usize],"representative_allowed":row.action.representative_mask&(1<<c.candidate_index)!=0})).collect::<Vec<_>>();
                    let mut diagnostic = json!({"condition":label,"world_seed":scene_case.seed,"requested":requested,"step":tick,"native_full_row_sha256":identity(row)?,"exact_prior_full_row_outcome_emission_and_sampling":true,"parent_full_state_matches_prior":true,"pre_private_sha256":private_before,"post_private_sha256":private_after,"owned_before":owned_before,"owned_after":owned_after,"actual_food_consumption":consequences(row,s).0,"energy_delta":row.patch.outcome().energy_delta,"biological_reward":row.biological_reward,"actual_emitted":emitted,"prior_actual_event":pending,"event_confirmed_speech_target":target,"prompted_cooldown_ready":ready,"native_action_logits":actions,"raw_speech_outputs_six_steps":raw,"decoded_conditional_payload":payload});
                    let tape = if tick == 1 {
                        Some(actor.replay_activation_tape(&q)?)
                    } else {
                        None
                    };
                    if tick == 1 {
                        let label_target = if target.is_some() && !ready {
                            let c = candidates.iter().find(|c| c.kind == ActionKind::Idle);
                            let mut t = candidate_label(row, c)?;
                            t.representative = c.map(|c| 1 << c.candidate_index);
                            t
                        } else if fulfilled && target.is_none() {
                            let c = candidates.iter().find(|c| c.kind == ActionKind::Idle);
                            let mut t = candidate_label(row, c)?;
                            t.representative = c.map(|c| 1 << c.candidate_index);
                            t
                        } else {
                            action_target(row, s, requested, target.is_some(), true, legal, held)?
                        };
                        let obj = cpu_imitation_objective(
                            logits,
                            &ImitationExample {
                                candidate_count: row.action.candidate_count,
                                representative_mask: row.action.representative_mask,
                                motor_masks: row.action.motor_masks,
                                forced_slots: row.action.forced_slots.clone(),
                                target: label_target.clone(),
                            },
                            1.,
                            1.,
                        )?;
                        let existing =
                            probe(&actor, &q, obj.logit_gradients.clone(), target, &mut probes)?;
                        diagnostic
                            ["existing_unit_native_imitation_and_factual_speech_components"] = json!({"target":{"representative":label_target.representative,"motors":label_target.motors},"action_ce":obj.loss,"speech_loss":existing.objective,"gradient":gradient_groups(&actor,&existing.gradients),"normalization":"unit per-row components; historical batch normalization, conditional and noun-margin terms are not reproduced"});
                        gradients.push(json!({"condition":label,"world_seed":scene_case.seed,"requested":requested,"kind":"existing_per_row_components","gradient":existing.gradients}));
                        if scene_case.seed == 16_309_347 {
                            assert!(pending.is_none() && event.is_none() && target.is_none());
                            assert_eq!(owned_before, [None, None]);
                            assert_eq!(owned_after, [None, None]);
                            assert!(!emitted.is_empty());
                            assert!(row.patch.selected_bundle().is_some_and(|b| b
                                .channels
                                .iter()
                                .any(|c| c.channel == MotorChannel::Vocal
                                    && c.payload.values.get(1) == Some(&0))));
                            assert!(payload.as_ref().is_some_and(|p| p
                                .tokens
                                .iter()
                                .map(|t| t.raw())
                                .collect::<Vec<_>>()
                                == requested.words()));
                            let silence = ReplaySpeechTarget {
                                token: None,
                                continuation: [0; 5],
                                act: SpeechActKind::Declare,
                                weight: 1.,
                            };
                            let g = probe(
                                &actor,
                                &q,
                                vec![0.; logits.len()],
                                Some(silence),
                                &mut probes,
                            )?;
                            diagnostic["witnessed_silence_only"] = json!({"eligibility":"actual matching Declare emitted before any activity; both owners absent; no physical event, not arbitrary missing target","native_target":silence,"loss":g.objective,"gradient":gradient_groups(&actor,&g.gradients),"speech_head_only_negative_gradient_direction_presence_raw0":head_gate_direction(&actor,&ev.final_activations[0],&g.gradients,16),"speech_head_only_negative_gradient_direction_stop_raw0":head_gate_direction(&actor,&ev.final_activations[0],&g.gradients,17),"current_stage_mask_unmodified":true,"no_update":true});
                            gradients.push(json!({"condition":label,"world_seed":scene_case.seed,"requested":requested,"kind":"witnessed_silence_only","gradient":g.gradients}));
                        } else {
                            assert!(
                                held && pending
                                    .as_ref()
                                    .is_some_and(|p| p.lesson == Lesson::GetFood)
                            );
                            let eat = candidates
                                .iter()
                                .find(|c| {
                                    c.action_id == HeadlessActionIds::EAT
                                        && c.target.entity == Some(s.food)
                                })
                                .ok_or("missing held EAT")?
                                .candidate_index as usize;
                            let rival = candidates
                                .iter()
                                .filter(|c| {
                                    matches!(c.kind, ActionKind::Idle | ActionKind::Vocalize)
                                        && row.action.representative_mask & (1 << c.candidate_index)
                                            != 0
                                })
                                .max_by(|a, b| {
                                    logits[a.candidate_index as usize]
                                        .total_cmp(&logits[b.candidate_index as usize])
                                })
                                .ok_or("no legal quiet/report competitor")?
                                .candidate_index as usize;
                            let mut adj = vec![0.; logits.len()];
                            adj[eat] = 1.;
                            adj[rival] = -1.;
                            let g = probe(&actor, &q, adj, None, &mut probes)?;
                            diagnostic["held_food_margin"] = json!({"eat_index":eat,"non_eat_index":rival,"eat_minus_best_idle_or_voice":logits[eat]-logits[rival],"current_stage_negative_existing_gradient_direction":-dot_masked(&actor,&g.gradients,&existing.gradients),"margin_gradient":gradient_groups(&actor,&g.gradients),"infinitesimal_direction_not_an_optimizer_step":true});
                            gradients.push(json!({"condition":label,"world_seed":scene_case.seed,"requested":requested,"kind":"held_eat_minus_best_quiet_or_voice_jacobian","gradient":g.gradients}));
                        }
                    }
                    writeln!(
                        trace,
                        "{}",
                        json!({"diagnostic":diagnostic,"full_native_row":row,"full_neural_activation_tape":tape})
                    )?;
                    records.push(diagnostic);
                    assert_eq!(
                        identity(&h.checkpoint(&actor)?)?,
                        private_after,
                        "read-only probes changed host"
                    );
                    assert_eq!(actor.checkpoint(), before);
                    if target.is_some() {
                        left -= 1;
                    }
                    if let Some(e) = event {
                        fulfilled |= e.lesson == requested;
                        pending = Some(e);
                        left = 3;
                    }
                }
                let cp = h.checkpoint(&actor)?;
                let file = format!(
                    "prefix-{label}-{}-{}.json.zst",
                    scene_case.seed,
                    requested.index()
                );
                write_compressed(&out.join(&file), &cp)?;
                checkpoints
                    .push(json!({"condition":label,"file":file,"host_sha256":identity(&cp)?}));
            }
        }
        assert_eq!(actor.checkpoint(), before);
        models.push(json!({"condition":label,"source_actor_sha256":before_hash,"final_actor_sha256":identity(&actor.checkpoint())?,"optimizer_step":actor.optimizer_step(),"all_weights_moments_ages_config_mask_and_source_asset_exact":true}));
    }
    trace.finish()?;
    assert_eq!(scored, 16);
    assert_eq!(probes, 16);
    write_compressed(&out.join("native-gradients.json.zst"), &gradients)?;
    write_json(&out.join("prefix-checkpoints.json"), &checkpoints)?;
    let status = Command::new(std::env::current_exe()?)
        .args([TEST, "--exact", "--ignored", "--nocapture"])
        .env("ALIFE_BOUNDED_TAPE_VERIFY_ONLY", "1")
        .status()?;
    assert!(status.success(), "fresh-process state verification failed");
    let cold: Value = serde_json::from_slice(&fs::read(
        out.join("fresh-process-state-verification.json"),
    )?)?;
    assert_eq!(cold["checks"].as_array().ok_or("no cold checks")?.len(), 8);
    write_json(
        &out.join("results.json"),
        &json!({"kind":"experimental-two-witness-native-tape-gradient-v25","execution_domain":"native-cpu","models":models,"scored_decisions":scored,"necessary_preparation_steps":8,"gradient_probes":probes,"adam_or_shadow_adam_steps":0,"semantic_prior":false,"teacher":false,"fresh_process_load_has_zero_decisions_or_probes":true,"elapsed_seconds":start.elapsed().as_secs_f64(),"records":records,"limitations":["Post-hoc diagnosis of two previously witnessed pre-sleep failures, not held-out behavioral improvement.","Existing per-row native imitation and factual-speech components use unit weights; historical batch normalization, conditional and noun-margin terms are not reproduced.","Raw gradient directions are infinitesimal diagnostics; current stage mask is retained, no optimizer update or behavioral correction is performed.","Only actual pre-event premature Declare states receive the native silence target; other missing speech targets remain unlabeled.","Full parent states and native rows match earlier cold traces. Eight prefix states are loaded exactly in a new process without taking extra decisions."]}),
    )?;
    Ok(())
}
