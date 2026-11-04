//! Opt-in payload-content learning with native action-policy utterance timing.
//! No new neural math, runtime thresholds or evaluation action overrides.
use super::*;

fn targets(
    episodes: &mut [Episode],
    labels: &mut impl Write,
    round: usize,
    arm: Arm,
) -> Result<Value> {
    let (mut positive, mut quiet) = (0, 0);
    for (episode_id, e) in episodes.iter_mut().enumerate() {
        for (i, row) in e.rows.iter().enumerate() {
            // These public founders abstain through representative selection. An enabled
            // conditional vocal sampler has no null category and needs a separate study.
            if row.action.motor_masks[3] != 0 {
                return Err(
                    "action-gated study requires representative-only native vocal arbitration"
                        .into(),
                );
            }
            let narrate = e.speech[i].is_some_and(|t| t.token.is_some());
            if narrate {
                positive += 1;
            } else {
                quiet += 1;
                e.speech[i] = None;
                if e.labels[i].representative.is_none() {
                    let excluded = row
                        .patch
                        .pre_action()
                        .perception()
                        .candidates()
                        .iter()
                        .filter(|c| c.kind == ActionKind::Vocalize)
                        .fold(0u32, |m, c| m | (1u32 << c.candidate_index));
                    let allowed = row.action.representative_mask & !excluded;
                    if allowed == 0 {
                        return Err(
                            "quiet narration label has no native physical representative".into(),
                        );
                    }
                    e.labels[i].representative = Some(allowed);
                }
            }
            writeln!(
                labels,
                "{}",
                json!({"arm":arm,"round":round,"episode":episode_id,"row":i,"native_patch_sha256":identity(&row.patch)?,"representative":e.labels[i].representative,"motors":e.labels[i].motors,"speech_target":e.speech[i]})
            )?;
        }
    }
    Ok(
        json!({"positive_payload_rows":positive,"quiet_rows_without_payload_loss":quiet,"utterance_timing":"native representative action; quiet leaves physical choices unconstrained"}),
    )
}
fn payload_gradient(
    actor: &CpuFoundationTrainer,
    episodes: &[Episode],
) -> Result<(Vec<f32>, Value)> {
    let mass = episodes
        .iter()
        .flat_map(|e| e.speech.iter().flatten())
        .map(|t| t.weight * t.len() as f32)
        .sum::<f32>();
    if mass == 0. {
        return Err(
            "no actual completed-activity payload supervision; preserving run prefix".into(),
        );
    }
    let (mut gradient, mut loss) = (vec![0.; actor.weights().len()], 0.0);
    for e in episodes {
        let amount = e
            .speech
            .iter()
            .flatten()
            .map(|t| t.weight * t.len() as f32)
            .sum::<f32>();
        if amount == 0. {
            continue;
        }
        let q = sequence(actor, e, e.rows.len(), 0);
        let zero = q
            .ticks
            .iter()
            .map(|t| vec![0.; t.candidates.len()])
            .collect::<Vec<_>>();
        let p = actor.probe_replay_gradients_with_speech(&q, &zero, &e.speech)?;
        let scale = amount / mass;
        loss += p.objective * f64::from(scale);
        for (dst, src) in gradient.iter_mut().zip(p.gradients) {
            *dst += src * scale;
        }
    }
    Ok((
        gradient,
        json!({"positive_payload_loss":loss,"positive_token_mass":mass,"negative_payload_targets":0,"quiet_is_supervised_by_native_action_mask":true}),
    ))
}

pub fn continue_action_gated_language(
    source: &Path,
    output: &Path,
    config: LanguagePilotConfig,
) -> Result<Value> {
    if config.llamacpp_translation
        || !config.experimental_word_bridge
        || !(1..=2).contains(&config.rounds)
        || !(1..=32).contains(&config.epochs)
        || !(24..=32).contains(&config.decisions)
    {
        return Err("action-gated study needs scripted teacher, explicit bridge, rounds 1..2, epochs 1..32, decisions 24..32".into());
    }
    let previous: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if previous["kind"] != "experimental-embodied-speech-input-v7"
        || previous["config"]["seed"] != config.seed
    {
        return Err(
            "action-gated study only accepts its matching measured disposable V2 source".into(),
        );
    }
    let recorded = previous["arms"]
        .as_array()
        .ok_or("no source arms")?
        .iter()
        .find(|v| v["arm"] == "grounded")
        .ok_or("missing common grounded source")?;
    fs::create_dir(output)?;
    let started = Instant::now();
    let translations = teacher_input(false)?;
    let mut traces = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output.join("native-collector-trace.jsonl.zst"))?,
        3,
    )?;
    let mut labels = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output.join("effective-native-training-targets.jsonl.zst"))?,
        3,
    )?;
    let (mut arms, mut baseline) = (vec![], Value::Null);
    for arm in [Arm::Grounded, Arm::Shuffled, Arm::NoSpeech] {
        let r = read_recovery(&source.join("recovery-Grounded"))?;
        if r.inputs.experimental_language_input()
            != Some(ExperimentalLanguageInputV1::TwoTokenAndEmbodiedSpeechV2)
        {
            return Err("common actor is not the measured V2 public founder".into());
        }
        let p = PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048())?;
        let mut asset = FoundationWeightAsset::decode_canonical(&r.foundation)?;
        let mut actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor)?;
        let retained = CpuCanonicalHost::from_checkpoint(r.host, &actor)?;
        let initial = identity(&actor.checkpoint())?;
        if initial != recorded["actor_sha256"] {
            return Err("common source differs from actual measured V2 actor".into());
        }
        let source_private = identity(&retained.checkpoint(&actor)?)?;
        let start = actor.optimizer_step();
        if arm == Arm::Grounded {
            eprintln!("action-gated: common untreated baseline");
            let file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output.join("untreated-common-baseline.jsonl.zst"))?;
            let mut trace = zstd::stream::write::Encoder::new(file, 3)?;
            baseline = json!(evaluate(
                &actor,
                &asset,
                config,
                Arm::Untreated,
                AssessmentPlan {
                    recovery_sleep: true,
                    collect_utterance_timing: true
                },
                &mut trace
            )?);
            trace.finish()?;
        }
        let (mut collections, mut updates, mut next) = (vec![], vec![], None);
        for round in 0..config.rounds {
            eprintln!("action-gated {arm:?} round {round}: actual collection");
            let (mut episodes, receipt) = collect(
                &actor,
                &asset,
                config,
                arm,
                round,
                &translations,
                &mut traces,
            )?;
            let effective = targets(&mut episodes, &mut labels, round, arm)?;
            eprintln!("action-gated {arm:?} collection {receipt} effective {effective}");
            collections.push(
                json!({"physical_collection":receipt,"effective_training_targets":effective}),
            );
            for epoch in 0..config.epochs {
                let (mut gradient, action) = gradients_with_speech(&actor, &episodes, false)?;
                let (speech, payload) = payload_gradient(&actor, &episodes)?;
                for (dst, src) in gradient.iter_mut().zip(speech) {
                    *dst += src;
                }
                let before = actor.checkpoint();
                let norm = actor.apply_gradients(&gradient)?;
                preserve_frozen(&before, &actor.checkpoint())?;
                updates.push(json!({"round":round,"epoch":epoch,"step":actor.optimizer_step(),"action":action,"payload":payload,"gradient_norm":norm,"frozen_coordinates_exact":true}));
                next = Some(gradient);
            }
            asset = export(&mut actor, config.seed)?;
        }
        eprintln!("action-gated {arm:?}: exact cold assessment and recovery sleep");
        let continuity = exact_reload_plan(
            &output.join(format!("recovery-{arm:?}")),
            &actor,
            &asset,
            RecoveredAssessment {
                config,
                arm,
                curriculum: Some(AssessmentPlan {
                    recovery_sleep: true,
                    collect_utterance_timing: true,
                }),
            },
            next.ok_or("no update")?,
        )?;
        let evaluation =
            continuity["native_private_and_optimizer_hashes"]["assessment"]["evaluation"].clone();
        arms.push(json!({"arm":arm,"common_source_actor_sha256":initial,"source_private_sha256":source_private,"source_step":start,"actor_step":actor.optimizer_step(),"actor_sha256":identity(&actor.checkpoint())?,"collections":collections,"updates":updates,"evaluation":evaluation,"continuity":continuity}));
    }
    traces.finish()?;
    labels.finish()?;
    let report = json!({"kind":"experimental-action-gated-payload-v8","execution_domain":"native-cpu","config":config,"common_source_arm":"grounded","source_report_sha256":identity(&previous)?,"untreated_common_baseline":baseline,"teacher_translation_deliveries":translations,"semantic_prior_enabled":false,"arms":arms,"elapsed_seconds":started.elapsed().as_secs_f64(),"limitations":["All three start from the same actual V2 actor/Adam history, which includes earlier grounded training. NoSpeech measures this incremental stage without teacher requests; it is not a language-naive brain.","Scripted teacher; no real model inference is attempted.","Fresh real cohorts with retained actor/Adam; no same-individual private-state optimizer rebase.","Raw collector trace retains its legacy labels for provenance; only effective-native-training-targets.jsonl.zst describes the actual offline supervision used here.","Payload content is supervised only after actual completed activity; native action choice learns whether to vocalize. Wrong/unsolicited utterances count as failures, not competence.","This bounded experiment requires representative-only vocal arbitration; it cannot silently apply to an enabled conditional vocal sampler.","Two contact-first collections and relocated contact/near assessment, one shared RecoveryProtocol sleep ancestor per cue trio. No spontaneous sleep or far-navigation claim."]});
    write_json(&output.join("results.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quiet_action_gate_preserves_physical_choices_and_has_no_payload_gradient() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor_profile(seed, &asset, true).unwrap();
        let (mut h, _) = host(seed, seed, &asset, &actor, 0, false).unwrap();
        let row = h.step(&actor, 1.).unwrap().decisions.remove(0);
        let mut e = vec![Episode {
            rows: vec![row],
            labels: vec![ImitationTarget {
                representative: None,
                motors: [None; 6],
            }],
            speech: vec![Some(ReplaySpeechTarget {
                token: None,
                continuation: [0; 5],
                act: SpeechActKind::Declare,
                weight: 0.25,
            })],
        }];
        targets(&mut e, &mut std::io::sink(), 0, Arm::NoSpeech).unwrap();
        assert!(e[0].speech[0].is_none());
        let allowed = e[0].labels[0].representative.unwrap();
        assert!(allowed.count_ones() > 1);
        for c in e[0].rows[0].patch.pre_action().perception().candidates() {
            assert_eq!(
                allowed & (1 << c.candidate_index) != 0,
                c.kind != ActionKind::Vocalize
            )
        }
        let (g, _) = gradients_with_speech(&actor, &e, false).unwrap();
        assert!(g.iter().any(|v| v.abs() > 0.));
        for (i, s) in actor.phenotype().synapses().iter().enumerate() {
            if matches!(s.kind(),CompiledSynapseKind::Decoder(c)if c.head()==DecoderHeadKind::SpeechPayload)
            {
                assert_eq!(g[i], 0.);
            }
        }
    }
}
