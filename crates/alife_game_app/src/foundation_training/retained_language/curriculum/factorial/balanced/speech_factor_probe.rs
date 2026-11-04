//! Read-only native-world gate/encoding/objective decomposition, not new training.
use super::super::token_probe::speech;
use super::*;
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
};

fn l2(a: &[f32], b: &[f32]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| f64::from(x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}
fn payload_audit(raw: &[Vec<f32>], target: Option<ReplaySpeechTarget>) -> Value {
    let words = [0usize, 1].map(|step| {
        (0..8).fold(0u32, |word, bit| {
            word | (u32::from(raw[step][8 + bit] > 0.) << bit)
        })
    });
    let act = (1..8).fold(0, |best, candidate| {
        if raw[0][candidate] > raw[0][best] {
            candidate
        } else {
            best
        }
    });
    let errors = target.map(|t| {
        (0..2)
            .map(|step| {
                (0..8)
                    .filter(|bit| (raw[step][8 + bit] > 0.) != (t.step_logits(step)[8 + bit] > 0.))
                    .count()
            })
            .sum::<usize>()
    });
    json!({"first_two_words_ignoring_gates_diagnostic_only":words,"first_act_argmax":act,"presence_raw":[raw[0][16],raw[1][16]],"stop_raw":[raw[0][17],raw[1][17]],"word_bit_errors_of_16":errors,"word_bits_correct":target.map(|_|errors==Some(0)),"presence_correct_for_two_words":raw[0][16]>0.&&raw[1][16]>0.,"stop_correct_for_two_words":raw[0][17]<=0.&&raw[1][17]>0.,"declare_act_correct":act==0,"first_two_raw_outputs":&raw[..2]})
}
fn conditional_correct(payload: &Option<SpeechMotorPayload>, target: ReplaySpeechTarget) -> bool {
    payload.as_ref().is_some_and(|p| {
        p.speech_act == SpeechActKind::Declare
            && p.tokens.iter().map(|t| t.raw()).collect::<Vec<_>>()
                == vec![target.token.unwrap(), target.continuation[0]]
    })
}
fn voice_probability(row: &CpuCollectedDecision, logits: &[f32]) -> f64 {
    let frame = row.patch.pre_action().perception();
    let max = logits
        .iter()
        .enumerate()
        .filter(|(i, _)| row.action.representative_mask & (1 << i) != 0)
        .map(|(_, x)| *x)
        .fold(f32::NEG_INFINITY, f32::max);
    let mut total = 0.;
    let mut voice = 0.;
    for c in frame.candidates() {
        if row.action.representative_mask & (1 << c.candidate_index) == 0 {
            continue;
        }
        let p =
            f64::from(((logits[c.candidate_index as usize] - max) / row.action.temperature).exp());
        total += p;
        if c.kind == ActionKind::Vocalize {
            voice += p;
        }
    }
    voice / total
}
fn prior_rows(path: &Path) -> BTreeMap<(usize, usize), Vec<String>> {
    let mut out = BTreeMap::<(usize, usize), Vec<String>>::new();
    let decoded = zstd::stream::read::Decoder::new(fs::File::open(path).unwrap()).unwrap();
    for line in BufReader::new(decoded).lines() {
        let e: Value = serde_json::from_str(&line.unwrap()).unwrap();
        if e["phase"] != "balanced_readout_evaluation" || e["post_sleep"] != false {
            continue;
        }
        let case = e["case"].as_u64().unwrap() as usize;
        if ![0, 2].contains(&case) {
            continue;
        }
        let request = e["requested"].as_str().map_or(3, |r| match r {
            "EatFood" => 0,
            "GetFood" => 1,
            "GetToy" => 2,
            _ => panic!("unexpected request"),
        });
        out.entry((case, request))
            .or_default()
            .push(e["native_full_row_sha256"].as_str().unwrap().to_owned());
    }
    out
}

#[test]
#[ignore = "explicit disposable V17 native speech-factor diagnostic; requires SOURCE and new OUT, no provider"]
fn diagnose_native_cue_payload_and_voice_factors() {
    let source = std::path::PathBuf::from(std::env::var("ALIFE_SPEECH_FACTOR_SOURCE").unwrap());
    let out = std::path::PathBuf::from(std::env::var("ALIFE_SPEECH_FACTOR_OUT").unwrap());
    assert!(!out.exists(), "preserve an existing diagnostic result");
    let measured: Value =
        serde_json::from_slice(&fs::read(source.join("results.json")).unwrap()).unwrap();
    assert!([
        "experimental-event-confirmed-speech-v17",
        "experimental-report-choice-noun-margin-v18"
    ]
    .iter()
    .any(|kind| measured["kind"] == *kind));
    let started = Instant::now();
    let mut records = vec![];
    let mut swaps = vec![];
    let mut shadows = vec![];
    let mut actors = vec![];
    for (arm, name) in [
        (Arm::Untreated, "Untreated"),
        (Arm::Grounded, "Grounded"),
        (Arm::Shuffled, "Shuffled"),
        (Arm::NoSpeech, "NoSpeech"),
    ] {
        let directory = source.join(format!("recovery-{name}"));
        let expected = prior_rows(&directory.join("reloaded-evaluation.jsonl.zst"));
        let r = read_recovery(&directory).unwrap();
        let p =
            PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048()).unwrap();
        assert_eq!(
            p.experimental_language_input(),
            Some(ExperimentalLanguageInputV1::LinearWordReadoutV3)
        );
        let asset = FoundationWeightAsset::decode_canonical(&r.foundation).unwrap();
        let actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor).unwrap();
        let before = identity(&actor.checkpoint()).unwrap();
        let recorded = measured["arms"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["arm"] == serde_json::to_value(arm).unwrap())
            .unwrap();
        assert_eq!(recorded["actor_sha256"], before);
        let motor = actor.phenotype().candidate_decoder().motor_start() as usize;
        for case in [0usize, 2] {
            let seed = HELD_OUT_SEEDS[case];
            let (mut parent, scene) = prepared(
                &actor,
                &asset,
                847501,
                seed,
                40 + case * 7,
                !case.is_multiple_of(2),
            )
            .unwrap();
            for _ in 0..2 {
                parent.step(&actor, 1.).unwrap();
            }
            place_balanced(parent.world_mut(), scene, case, true, true).unwrap();
            let id = parent.world().organism_entity_ids()[0].0;
            let parent_checkpoint = parent.checkpoint(&actor).unwrap();
            let parent_hash = identity(&parent_checkpoint).unwrap();
            let inherited_speech_tick = parent_checkpoint
                .world
                .world
                .last_creature_utterance_ticks
                .iter()
                .find(|(speaker, _)| *speaker == id)
                .map(|(_, tick)| *tick);
            let mut first = vec![];
            for (request, requested) in [
                Some(Lesson::EatFood),
                Some(Lesson::GetFood),
                Some(Lesson::GetToy),
                None,
            ]
            .into_iter()
            .enumerate()
            {
                let mut h = parent.snapshot();
                let mut last_actual_speech_tick = inherited_speech_tick;
                let mut pending = None::<Event>;
                let mut left = 0;
                let mut shadow_done = false;
                for (tick, expected_digest) in expected[&(case, request)].iter().take(8).enumerate()
                {
                    if tick % 2 == 0 {
                        if let Some(goal) = requested {
                            let pos = speech_position(h.world()).unwrap();
                            h.world_mut()
                                .emit_player_tokens(
                                    Some(id),
                                    pos,
                                    goal.words()
                                        .into_iter()
                                        .map(|w| LanguageTokenId::new(w).unwrap())
                                        .collect(),
                                )
                                .unwrap();
                        }
                    }
                    let owned_before = owners(h.world(), scene);
                    let step = h.step(&actor, 1.).unwrap();
                    let owned_after = owners(h.world(), scene);
                    assert_eq!(
                        step.decisions.len(),
                        1,
                        "bounded awake fixture must reproduce the saved row"
                    );
                    for row in step.decisions {
                        no_teacher_or_prior(&row).unwrap();
                        let digest = identity(&row).unwrap();
                        assert_eq!(&digest, expected_digest);
                        let target = confirmed_speech(pending.as_ref(), left, requested);
                        let q = readout::single(&actor, &row);
                        let ev = actor.evaluate_replay(&q).unwrap();
                        let logits = &ev.candidate_logits[0];
                        let sampled = sample_cpu_joint_action(
                            actor.phenotype(),
                            row.patch.pre_action().perception(),
                            logits,
                            seed as u32,
                            row.sampling_counter.unwrap(),
                            row.action.temperature,
                        )
                        .unwrap();
                        assert_eq!(sampled, row.action);
                        let raw = speech(
                            &actor,
                            &ev.final_activations[0],
                            &q.ticks[0].effective_weight_offsets,
                        );
                        let decoded = decode_cpu_speech(
                            &actor,
                            &ev.final_activations[0],
                            &q.ticks[0].effective_weight_offsets,
                        )
                        .unwrap();
                        assert_eq!(
                            row.action.motor_masks[3], 0,
                            "this profile uses representative-only vocal choice"
                        );
                        let vocal_choice = row.patch.pre_action().perception().candidates()
                            [row.action.representative as usize]
                            .kind
                            == ActionKind::Vocalize;
                        let vocal = row.patch.selected_bundle().is_some_and(|b| {
                            b.channels
                                .iter()
                                .any(|ch| ch.channel == MotorChannel::Vocal)
                        });
                        let emitted = h
                            .world()
                            .audible_utterances()
                            .into_iter()
                            .filter(|u| {
                                u.source_kind == UtteranceSourceKind::Creature
                                    && u.speaker_id == Some(id)
                                    && u.emitted_tick == row.patch.pre_action().perception().tick()
                            })
                            .map(|u| u.tokens.iter().map(|t| t.raw()).collect::<Vec<_>>())
                            .collect::<Vec<_>>();
                        let frame = row.patch.pre_action().perception();
                        let prompted = frame
                            .sensory()
                            .language_context
                            .heard_tokens
                            .iter()
                            .flatten()
                            .any(|t| t.source_kind == UtteranceSourceKind::Player);
                        // Match the authoritative world guard: prompted 8, spontaneous 32.
                        // The actual selected channel flag below verifies the host agrees.
                        let cooldown = if prompted {
                            8
                        } else {
                            SPONTANEOUS_SPEECH_COOLDOWN_TICKS
                        };
                        let cooldown_blocked = last_actual_speech_tick.is_some_and(|last: Tick| {
                            frame.tick().raw().saturating_sub(last.raw()) < cooldown
                        });
                        if let Some(channel) = row.patch.selected_bundle().and_then(|b| {
                            b.channels
                                .iter()
                                .find(|ch| ch.channel == MotorChannel::Vocal)
                        }) {
                            if let Some(payload) = decoded.as_ref() {
                                assert_eq!(
                                    channel.payload.values.get(2).copied(),
                                    Some(u32::from(prompted))
                                );
                                assert_eq!(
                                    &channel.payload.values[4..],
                                    &payload
                                        .tokens
                                        .iter()
                                        .map(|t| u32::from(t.raw()))
                                        .collect::<Vec<_>>()
                                );
                            } else {
                                assert!(channel.payload.values.is_empty());
                            }
                        }
                        let expected_tokens = if vocal && !cooldown_blocked {
                            decoded
                                .as_ref()
                                .map(|p| p.tokens.iter().map(|t| t.raw()).collect::<Vec<_>>())
                                .into_iter()
                                .collect::<Vec<_>>()
                        } else {
                            vec![]
                        };
                        assert_eq!(
                            emitted, expected_tokens,
                            "native payload and world cooldown must predict actual utterances"
                        );
                        records.push(json!({"arm":arm,"case":case,"world_seed":seed,"requested":requested,"tick_after_cue":tick,"native_row_sha256":digest,"row_matches_saved_source":true,"native_action_resampling_exact":true,"event_confirmed_target":target,"prior_actual_event":pending,"actual_food_consumption":consequences(&row,scene).0,"owners_before":owned_before,"owners_after":owned_after,"selected_vocal_channel":vocal,"native_vocal_choice_selected":vocal_choice,"last_actual_speech_tick_before":last_actual_speech_tick,"prompted_world_speech":prompted,"world_speech_cooldown_ticks":cooldown,"world_speech_cooldown_blocked":cooldown_blocked,"native_payload_matches_proposed_vocal_tokens":true,"world_payload_and_cooldown_predict_actual_utterance":true,"voice_choice_probability":voice_probability(&row,logits),"actual_utterance_tokens":emitted,"conditional_payload_if_vocal_diagnostic_only":decoded,"conditional_payload_correct":target.map(|t|conditional_correct(&decoded,t)),"payload_factors":payload_audit(&raw,target)}));
                        if !emitted.is_empty() {
                            last_actual_speech_tick = Some(frame.tick());
                        }
                        if tick == 0 {
                            first.push((
                                requested,
                                q.clone(),
                                ev.final_activations[0].clone(),
                                raw.clone(),
                            ));
                        }
                        if let Some(t) = target {
                            left -= 1;
                            if !shadow_done {
                                let g = actor
                                    .probe_replay_gradients_with_speech(
                                        &q,
                                        &[vec![0.; logits.len()]],
                                        &[Some(t)],
                                    )
                                    .unwrap();
                                let mut shadow = CpuFoundationTrainer::from_checkpoint(
                                    actor.phenotype().clone(),
                                    asset.clone(),
                                    actor.checkpoint(),
                                )
                                .unwrap();
                                let cp = shadow.checkpoint();
                                let norm = shadow.apply_gradients(&g.gradients).unwrap();
                                let after = shadow.checkpoint();
                                preserve_frozen(&cp, &after).unwrap();
                                let after_g = shadow
                                    .probe_replay_gradients_with_speech(
                                        &q,
                                        &[vec![0.; logits.len()]],
                                        &[Some(t)],
                                    )
                                    .unwrap();
                                let ev_after = shadow.evaluate_replay(&q).unwrap();
                                let raw_after = speech(
                                    &shadow,
                                    &ev_after.final_activations[0],
                                    &q.ticks[0].effective_weight_offsets,
                                );
                                let dot = cp
                                    .weights
                                    .iter()
                                    .zip(&after.weights)
                                    .zip(&g.gradients)
                                    .map(|((a, b), g)| f64::from(b - a) * f64::from(*g))
                                    .sum::<f64>();
                                shadows.push(json!({"arm":arm,"case":case,"requested":requested,"tick_after_cue":tick,"actual_target":t,"native_loss_before":g.objective,"native_loss_after_one_shadow_adam_step":after_g.objective,"native_gradient_norm":norm,"gradient_dot_native_adam_displacement":dot,"source_step":cp.optimizer_step,"shadow_step":after.optimizer_step,"frozen_weights_moments_ages_exact":true,"payload_factors_after_shadow":payload_audit(&raw_after,Some(t)),"no_world_action_or_learning_claim_from_shadow_step":true}));
                                shadow_done = true;
                            }
                        }
                        if let Some(event) =
                            physical_event(&row, scene, owned_before, owned_after).unwrap()
                        {
                            pending = Some(event);
                            left = 3;
                        }
                    }
                }
                let saved_last = h
                    .checkpoint(&actor)
                    .unwrap()
                    .world
                    .world
                    .last_creature_utterance_ticks
                    .iter()
                    .find(|(speaker, _)| *speaker == id)
                    .map(|(_, tick)| *tick);
                assert_eq!(last_actual_speech_tick, saved_last);
            }
            for (a, b) in [(0usize, 1usize), (1, 2), (1, 3)] {
                let (ra, qa, aa, sa) = &first[a];
                let (rb, qb, ab, sb) = &first[b];
                assert_eq!(qa.initial, qb.initial);
                let mut swap = qa.clone();
                swap.ticks[0].encoded_inputs[motor..motor + 160]
                    .copy_from_slice(&qb.ticks[0].encoded_inputs[motor..motor + 160]);
                let se = actor.evaluate_replay(&swap).unwrap();
                let ss = speech(
                    &actor,
                    &se.final_activations[0],
                    &swap.ticks[0].effective_weight_offsets,
                );
                let natural = l2(&sa[..2].concat(), &sb[..2].concat());
                let isolated = l2(&sa[..2].concat(), &ss[..2].concat());
                swaps.push(json!({"arm":arm,"case":case,"parent_private_sha256":parent_hash,"request_a":ra,"request_b":rb,"matched_initial_private_neural_state":true,"encoded_direct_bank_delta_l2":l2(&qa.ticks[0].encoded_inputs[motor..motor+160],&qb.ticks[0].encoded_inputs[motor..motor+160]),"natural_speech_source_delta_l2":l2(&aa[motor+128..motor+160],&ab[motor+128..motor+160]),"natural_first_two_output_delta_l2":natural,"cue_bank_swap_first_two_output_delta_l2":isolated,"cue_bank_swap_word_bit_outputs_delta_l2":l2(&sa[..2].iter().flat_map(|v|v[8..16].iter().copied()).collect::<Vec<_>>(),&ss[..2].iter().flat_map(|v|v[8..16].iter().copied()).collect::<Vec<_>>()),"cue_bank_swap_only_conditional_replay_not_world_transition":true}));
            }
        }
        assert_eq!(identity(&actor.checkpoint()).unwrap(), before);
        actors.push(json!({"arm":arm,"actor_sha256":before,"source_actor_private_and_optimizer_files_unmodified":true}));
    }
    assert_eq!(records.len(), 256);
    let report = json!({"kind":"native-speech-cue-payload-voice-factor-diagnostic-v1","execution_domain":"native-cpu","source_kind":measured["kind"],"actors":actors,"held_out_cases":[0,2],"world_seeds":[HELD_OUT_SEEDS[0],HELD_OUT_SEEDS[2]],"native_decisions":records.len(),"all_rows_match_existing_source":true,"teacher_and_prior_absent":true,"elapsed_seconds":started.elapsed().as_secs_f64(),"rows":records,"matched_cue_bank_swaps":swaps,"disposable_native_shadow_optimizer_probes":shadows,"limits":["Two previously observed held-out worlds and one founder, pre-sleep only; not an independent validation or new training experiment.","Conditional payloads, cue-bank swaps and shadow optimizer steps do not execute world actions and do not establish learning or competence.","The encoder bank contains native word and bodily contributions; swaps hold all other recorded replay context fixed but are not a pure semantic intervention.","No source checkpoint mutation, default-brain migration, promotion, real SLM request or Luna retry."]});
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)
        .unwrap();
    serde_json::to_writer_pretty(&mut file, &report).unwrap();
}
