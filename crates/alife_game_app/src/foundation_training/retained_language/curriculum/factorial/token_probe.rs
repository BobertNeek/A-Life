//! Paired interventions on real public worlds. Repeated frozen rows below are
//! gradient diagnostics, never biological trajectories or scored behavior.
use super::*;

fn norm(g: &[f32]) -> f64 {
    g.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>().sqrt()
}
fn delta(a: &[f32], b: &[f32]) -> Value {
    let d = a.iter().zip(b).map(|(a, b)| a - b).collect::<Vec<_>>();
    json!({"l2":norm(&d),"max_abs":d.iter().map(|x|x.abs()).fold(0f32,f32::max),"changed":d.iter().filter(|x|**x!=0.).count()})
}
fn cosine(a: &[f32], b: &[f32]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| f64::from(*a) * f64::from(*b))
        .sum::<f64>()
        / (norm(a) * norm(b)).max(1e-30)
}
fn speech(actor: &CpuFoundationTrainer, a: &[f32], offsets: &[f32]) -> Vec<Vec<f32>> {
    let source = actor.phenotype().candidate_decoder().motor_start() as usize + 128;
    let mut inputs = a[source..source + 32].to_vec();
    let mut result = vec![];
    for _ in 0..6 {
        let mut outputs = vec![0f32; 32];
        for (i, s) in actor.phenotype().synapses().iter().enumerate() {
            if matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload)
            {
                outputs[s.target() as usize - source - 32] +=
                    inputs[s.source() as usize - source] * (actor.weights()[i] + offsets[i]);
            }
        }
        result.push(outputs.clone());
        for (dst, src) in inputs[18..].iter_mut().zip(&outputs[18..]) {
            *dst = src.tanh();
        }
    }
    result
}
fn groups(actor: &CpuFoundationTrainer, g: &[f32]) -> Value {
    let cp = actor.checkpoint();
    let mut sq = [0f64; 4];
    let mut counts = [0usize; 4];
    for (i, s) in actor.phenotype().synapses().iter().enumerate() {
        if !cp.stage_mask.is_trainable(i) {
            continue;
        }
        let k = match s.kind() {
            CompiledSynapseKind::Recurrent => 0,
            CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate => 1,
            CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::SpeechPayload => 2,
            _ => 3,
        };
        sq[k] += f64::from(g[i]).powi(2);
        counts[k] += usize::from(g[i] != 0.);
    }
    json!({"order":["recurrent","action_head","speech_head","other"],"trainable_gradient_l2":sq.map(f64::sqrt),"nonzero_trainable_coordinates":counts})
}
fn action_gradient(
    actor: &CpuFoundationTrainer,
    q: &TrainingSequence,
    row: &CpuCollectedDecision,
    label: ImitationTarget,
) -> Result<Vec<f32>> {
    let evaluation = actor.evaluate_replay(q)?;
    let example = ImitationExample {
        candidate_count: row.action.candidate_count,
        representative_mask: row.action.representative_mask,
        motor_masks: row.action.motor_masks,
        forced_slots: row.action.forced_slots.clone(),
        target: label,
    };
    let o = cpu_imitation_objective(&evaluation.candidate_logits[0], &example, 1., 1.)?;
    let mut adj = q
        .ticks
        .iter()
        .map(|t| vec![0.; t.candidates.len()])
        .collect::<Vec<_>>();
    adj[0] = o.logit_gradients;
    Ok(actor.probe_replay_gradients(q, &adj)?.gradients)
}
#[test]
#[ignore = "explicit disposable native paired token diagnostic; no provider request"]
fn trace_paired_token_paths_and_teaching_gradients() {
    let out = std::path::PathBuf::from(std::env::var("ALIFE_TOKEN_PROBE_OUT").unwrap());
    let mut results = vec![];
    for (name, directory) in [
        (
            "common_v8",
            "/workspace/language-action-gated-run/recovery-Grounded",
        ),
        (
            "v11_grounded",
            "/workspace/language-factorial-hold-run/recovery-Grounded",
        ),
        (
            "v11_shuffled",
            "/workspace/language-factorial-hold-run/recovery-Shuffled",
        ),
        (
            "v11_no_speech",
            "/workspace/language-factorial-hold-run/recovery-NoSpeech",
        ),
    ] {
        let r = read_recovery(Path::new(directory)).unwrap();
        let p =
            PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048()).unwrap();
        let asset = FoundationWeightAsset::decode_canonical(&r.foundation).unwrap();
        let actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor).unwrap();
        let before = identity(&actor.checkpoint()).unwrap();
        let (mut base, s) = base_host(&actor, &asset, 947839, 847501).unwrap();
        place(base.world_mut(), s, 3, 0.41, true).unwrap();
        let id = base.world().organism_entity_ids()[0].0;
        let mut rows = vec![];
        for lesson in Lesson::ALL {
            let mut h = base.snapshot();
            let pos = speech_position(h.world()).unwrap();
            h.world_mut()
                .emit_player_tokens(
                    Some(id),
                    pos,
                    lesson
                        .words()
                        .into_iter()
                        .map(LanguageTokenId::new)
                        .collect::<std::result::Result<_, _>>()
                        .unwrap(),
                )
                .unwrap();
            let row = h.step(&actor, 1.).unwrap().decisions.remove(0);
            no_teacher_or_prior(&row).unwrap();
            rows.push(row);
        }
        let mut qs = vec![];
        let mut evaluations = vec![];
        let mut tapes = vec![];
        let mut gradients = vec![];
        let mut records = vec![];
        for (i, row) in rows.iter().enumerate() {
            let q = TrainingSequence {
                phenotype_hash: actor.phenotype().phenotype_hash(),
                initial: rows[0].initial.clone(),
                ticks: vec![row.replay_tick.clone(); 6],
                burn_in_ticks: 0,
                memory_candidate_gain: actor
                    .phenotype()
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0., |m| m.max_candidate_gain()),
            };
            let e = actor.evaluate_replay(&q).unwrap();
            let label = action_target(row, s, Lesson::ALL[i], false, true, true, false).unwrap();
            let g = action_gradient(&actor, &q, row, label.clone()).unwrap();
            let decoded = decode_cpu_speech(
                &actor,
                &e.final_activations[0],
                &q.ticks[0].effective_weight_offsets,
            )
            .unwrap();
            records.push(json!({"lesson":Lesson::ALL[i],"heard":row.patch.pre_action().perception().sensory().language_context,"candidates":row.patch.pre_action().perception().candidates(),"first_logits":e.candidate_logits[0],"label":{"representative":label.representative,"motors":label.motors},"gradient_norm":norm(&g),"trainable_gradients":groups(&actor,&g),"speech_logits":speech(&actor,&e.final_activations[0],&q.ticks[0].effective_weight_offsets),"decoded_payload_if_vocal":decoded,"actual_selected_action":row.action,"initial_sha256":identity(&row.initial).unwrap()}));
            tapes.push(actor.replay_activation_tape(&q).unwrap());
            evaluations.push(e);
            gradients.push(g);
            qs.push(q);
        }
        let mut pairs = vec![];
        let motor = actor.phenotype().candidate_decoder().motor_start() as usize;
        for (a, b, kind) in [(0, 1, "verb"), (1, 2, "noun")] {
            let label =
                action_target(&rows[a], s, Lesson::ALL[a], false, true, true, false).unwrap();
            let same_label = action_gradient(&actor, &qs[b], &rows[a], label).unwrap();
            let other_label =
                action_target(&rows[a], s, Lesson::ALL[b], false, true, true, false).unwrap();
            let same_input = action_gradient(&actor, &qs[a], &rows[a], other_label).unwrap();
            pairs.push(json!({"pair":kind,"encoded":delta(&qs[a].ticks[0].encoded_inputs,&qs[b].ticks[0].encoded_inputs),"candidate_context_identical":qs[a].ticks[0].candidates==qs[b].ticks[0].candidates,"initial_identical":rows[a].initial==rows[b].initial,"effective_offsets_identical":qs[a].ticks[0].effective_weight_offsets==qs[b].ticks[0].effective_weight_offsets,"microsteps":tapes[a].iter().zip(&tapes[b]).enumerate().map(|(i,(a,b))|json!({"index":i,"all":delta(a,b),"action_source":delta(&a[motor..motor+128],&b[motor..motor+128]),"speech_source":delta(&a[motor+128..motor+160],&b[motor+128..motor+160])})).collect::<Vec<_>>(),"action_logits_by_tick":evaluations[a].candidate_logits.iter().zip(&evaluations[b].candidate_logits).map(|(a,b)|delta(a,b)).collect::<Vec<_>>(),"speech_logits_by_tick":evaluations[a].final_activations.iter().zip(&evaluations[b].final_activations).map(|(aa,bb)| {let sa=speech(&actor,aa,&qs[a].ticks[0].effective_weight_offsets);let sb=speech(&actor,bb,&qs[b].ticks[0].effective_weight_offsets);sa.iter().zip(sb).map(|(a,b)|delta(a,&b)).collect::<Vec<_>>()}).collect::<Vec<_>>(),"matched_teaching_gradient_delta":delta(&gradients[a],&gradients[b]),"matched_teaching_gradient_cosine":cosine(&gradients[a],&gradients[b]),"first_action_logits_a":evaluations[a].candidate_logits[0],"first_action_logits_b":evaluations[b].candidate_logits[0],"token_only_same_label_gradient_delta":delta(&gradients[a],&same_label),"label_only_same_input_gradient_delta":delta(&gradients[a],&same_input)}));
        }
        #[derive(Deserialize)]
        struct Captured {
            phase: String,
            arm: Arm,
            row: CpuCollectedDecision,
            speech_target: Option<ReplaySpeechTarget>,
        }
        use std::io::{BufRead, BufReader};
        let mut positives = std::collections::BTreeMap::new();
        let path = "/workspace/language-factorial-hold-run/native-trace.jsonl.zst";
        for line in
            BufReader::new(zstd::stream::read::Decoder::new(fs::File::open(path).unwrap()).unwrap())
                .lines()
        {
            // Other phases have different schemas.
            let line = line.unwrap();
            if !line.contains("factorial_training") {
                continue;
            }
            let c: Captured = serde_json::from_str(&line).unwrap();
            if c.phase != "factorial_training" || c.arm != Arm::Grounded {
                continue;
            }
            if let Some(t) = c.speech_target.filter(|t| t.token.is_some()) {
                let key = (t.token.unwrap(), t.continuation[0]);
                positives.entry(key).or_insert((c.row, t));
                if positives.len() == 3 {
                    break;
                }
            }
        }
        let mut speech_gradients = vec![];
        for ((verb, noun), (r, t)) in positives {
            let q = TrainingSequence {
                phenotype_hash: actor.phenotype().phenotype_hash(),
                initial: r.initial.clone(),
                ticks: vec![r.replay_tick.clone()],
                burn_in_ticks: 0,
                memory_candidate_gain: actor
                    .phenotype()
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0., |m| m.max_candidate_gain()),
            };
            let zero = vec![vec![0.; q.ticks[0].candidates.len()]];
            let g = actor
                .probe_replay_gradients_with_speech(&q, &zero, &[Some(t)])
                .unwrap();
            let mut swapped = t;
            swapped.continuation[0] = if noun == 1 { 2 } else { 1 };
            let alt = actor
                .probe_replay_gradients_with_speech(&q, &zero, &[Some(swapped)])
                .unwrap();
            speech_gradients.push(json!({"factual_activity_words":[verb,noun],"actual_positive_tick":r.patch.pre_action().tick.raw(),"captured_native_positive_only":true,"gradient":groups(&actor,&g.gradients),"counterfactual_label_only_not_applied":{"gradient_delta":delta(&g.gradients,&alt.gradients),"cosine":cosine(&g.gradients,&alt.gradients)}}));
        }
        assert_eq!(before, identity(&actor.checkpoint()).unwrap());
        results.push(json!({"actor":name,"actor_sha256":before,"world_seed":947839,"distance":0.41,"layout":3,"repeated_frozen_context_diagnostic_ticks":6,"records":records,"pairs":pairs,"actual_positive_speech_gradients":speech_gradients}));
    }
    write_json(&out,&json!({"kind":"native-paired-token-diagnostic-v1","not_a_behavioral_learning_result":true,"actors":results})).unwrap();
}
