//! New disposable V3 representation, native decoder-only learning, real biology.
use super::*;
fn collect(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    c: LanguagePilotConfig,
    arm: Arm,
    round: usize,
    traces: &mut impl Write,
) -> Result<(Vec<Episode>, Value)> {
    let (mut data, mut count, mut positive, mut heard) = (vec![], [0usize; 3], 0usize, 0usize);
    for layout in 0..2 {
        let (mut base, s) = base_host(
            actor,
            asset,
            c.seed + 180_000 + (round * 2 + layout) as u64,
            c.seed,
        )?;
        place(
            base.world_mut(),
            s,
            layout,
            0.36 + round as f32 * 0.08,
            false,
        )?;
        for requested in Lesson::ALL {
            let mut h = base.snapshot();
            let id = h.world().organism_entity_ids()[0].0;
            let (mut pending, mut left, mut fulfilled) = (None::<Event>, 0usize, false);
            let mut e = Episode {
                rows: vec![],
                labels: vec![],
                speech: vec![],
            };
            for tick in 0..c.decisions {
                if arm != Arm::NoSpeech && tick < 8 && tick % 2 == 0 {
                    let shift = if arm == Arm::Shuffled {
                        1 + (layout + round) % 2
                    } else {
                        0
                    };
                    let cue = Lesson::ALL[(requested.index() + shift) % 3];
                    let pos = speech_position(h.world())?;
                    alife_school::LanguageNursery::speak_in_world(
                        h.world_mut(),
                        id,
                        pos,
                        cue.words()
                            .into_iter()
                            .map(LanguageTokenId::new)
                            .collect::<std::result::Result<_, _>>()?,
                    )?;
                }
                let legal = h.world().can_grab_target(id, requested.target(s));
                let before = owners(h.world(), s);
                let held = before[usize::from(requested == Lesson::GetToy)] == Some(id);
                let step = h.step(actor, 1.)?;
                let after = owners(h.world(), s);
                for row in step.decisions {
                    let f = row.patch.pre_action().perception();
                    heard += usize::from(
                        f.sensory()
                            .language_context
                            .heard_tokens
                            .iter()
                            .flatten()
                            .any(|t| t.source_kind == UtteranceSourceKind::Teacher),
                    );
                    let target = if tick >= 8 {
                        narration(f, pending.as_ref(), left)
                    } else {
                        None
                    };
                    let label = if tick < 8 && fulfilled {
                        let idle = f.candidates().iter().find(|c| c.kind == ActionKind::Idle);
                        let mut t = candidate_label(&row, idle)?;
                        t.representative = idle.map(|c| 1 << c.candidate_index);
                        t
                    } else {
                        action_target(&row, s, requested, target.is_some(), tick < 8, legal, held)?
                    };
                    if target.is_some() {
                        positive += 1;
                        left -= 1;
                    }
                    let event = activity(&row, s, before, after)?;
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"linear_readout_training","arm":arm,"round":round,"layout":layout,"requested":requested,"instruction_window":tick<8,"scene":s,"row":row,"owned_before":before,"owned_after":after,"label":{"representative":label.representative,"motors":label.motors},"speech_target":target,"narration_evidence":pending,"completed":event})
                    )?;
                    if let Some(event) = event {
                        count[event.lesson.index()] += 1;
                        fulfilled |= event.lesson == requested;
                        pending = Some(event);
                        left = 3;
                    }
                    e.rows.push(row);
                    e.labels.push(label);
                    e.speech.push(target);
                }
            }
            data.push(e);
        }
    }
    Ok((
        data,
        json!({"round":round,"native_decisions":c.decisions*6,"teacher_heard_rows":heard,"actual_activity_counts":count,"positive_narration_rows":positive,"instruction_window_rows":48,"fixed_eight_tick_instruction_window":true}),
    ))
}
pub(super) fn single(actor: &CpuFoundationTrainer, r: &CpuCollectedDecision) -> TrainingSequence {
    TrainingSequence {
        phenotype_hash: actor.phenotype().phenotype_hash(),
        initial: r.initial.clone(),
        ticks: vec![r.replay_tick.clone()],
        burn_in_ticks: 0,
        memory_candidate_gain: actor
            .phenotype()
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    }
}
fn update_gradient(
    actor: &CpuFoundationTrainer,
    episodes: &[Episode],
) -> Result<(Vec<f32>, Value)> {
    let mut counts = [0usize; 3];
    let mut payload_mass = [0f32; 3];
    for e in episodes {
        for (i, t) in e.speech.iter().enumerate() {
            counts[if i < 8 {
                0
            } else if t.is_some() {
                1
            } else {
                2
            }] += 1;
            if let Some(t) = t {
                let k = Lesson::ALL
                    .iter()
                    .position(|l| l.words() == [t.token.unwrap(), t.continuation[0]])
                    .ok_or("unexpected factual speech")?;
                payload_mass[k] += t.len() as f32;
            }
        }
    }
    let budgets = [0.8f32, 0.1, 0.1];
    let observed = payload_mass.iter().filter(|x| **x > 0.).count();
    let (mut sum, mut action_loss, mut speech_loss) = (vec![0.; actor.weights().len()], 0f64, 0f64);
    for e in episodes {
        for (i, row) in e.rows.iter().enumerate() {
            let q = single(actor, row);
            let ev = actor.evaluate_replay(&q)?;
            let k = if i < 8 {
                0
            } else if e.speech[i].is_some() {
                1
            } else {
                2
            };
            let scale = budgets[k] / counts[k] as f32;
            let ex = ImitationExample {
                candidate_count: row.action.candidate_count,
                representative_mask: row.action.representative_mask,
                motor_masks: row.action.motor_masks,
                forced_slots: row.action.forced_slots.clone(),
                target: e.labels[i].clone(),
            };
            let o = cpu_imitation_objective(&ev.candidate_logits[0], &ex, 1., 1.)?;
            action_loss += f64::from(o.loss * scale);
            let p = actor.probe_replay_gradients(
                &q,
                &[o.logit_gradients.into_iter().map(|v| v * scale).collect()],
            )?;
            for (dst, src) in sum.iter_mut().zip(p.gradients) {
                *dst += src;
            }
            if let Some(t) = e.speech[i] {
                let class = Lesson::ALL
                    .iter()
                    .position(|l| l.words() == [t.token.unwrap(), t.continuation[0]])
                    .ok_or("unknown payload")?;
                let scale = t.len() as f32 / payload_mass[class] / observed as f32;
                let p = actor.probe_replay_gradients_with_speech(
                    &q,
                    &[vec![0.; q.ticks[0].candidates.len()]],
                    &[Some(t)],
                )?;
                speech_loss += p.objective * f64::from(scale);
                for (dst, src) in sum.iter_mut().zip(p.gradients) {
                    *dst += src * scale;
                }
            }
        }
    }
    Ok((
        sum,
        json!({"action_loss":action_loss,"speech_loss":speech_loss,"action_class_rows":counts,"action_class_budgets":budgets,"factual_payload_token_mass":payload_mass,"native_one_step_readout_gradients":true,"recurrent_parameters_frozen":true}),
    ))
}

pub(crate) fn evaluate(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    c: LanguagePilotConfig,
    arm: Arm,
    plan: AssessmentPlan,
    traces: &mut impl Write,
) -> Result<Vec<Value>> {
    let mut results = evaluate_factorial(actor, asset, c, arm, plan, traces)?;
    for layout in 0..2 {
        let (mut base, s) = base_host(actor, asset, c.seed + 390_000 + layout as u64, c.seed)?;
        place(
            base.world_mut(),
            s,
            layout,
            if layout == 0 { 0.39 } else { 0.67 },
            true,
        )?;
        let paired = identity(&base.checkpoint(actor)?)?;
        let id = base.world().organism_entity_ids()[0].0;
        for words in [
            Some([8u16, 1]),
            Some([9, 1]),
            Some([9, 2]),
            Some([8, 2]),
            None,
        ] {
            let mut h = base.snapshot();
            let (mut first, mut attempted, mut acquired, mut meals, mut nourished, mut utterances) = (
                None::<(u32, Option<WorldEntityId>)>,
                false,
                false,
                0usize,
                0usize,
                vec![],
            );
            let primitive = words.map(|w| {
                if w[0] == 8 {
                    HeadlessActionIds::EAT
                } else {
                    HeadlessActionIds::GRAB
                }
            });
            let target = words.map(|w| if w[1] == 1 { s.food } else { s.toy });
            for tick in 0..8 {
                if tick < 6 && tick % 2 == 0 {
                    if let Some(w) = words {
                        let pos = speech_position(h.world())?;
                        h.world_mut().emit_player_tokens(
                            Some(id),
                            pos,
                            w.into_iter()
                                .map(LanguageTokenId::new)
                                .collect::<std::result::Result<_, _>>()?,
                        )?;
                    }
                }
                let before = owners(h.world(), s);
                let step = h.step(actor, 1.)?;
                let after = owners(h.world(), s);
                for row in step.decisions {
                    no_teacher_or_prior(&row)?;
                    let f = row.patch.pre_action().perception();
                    if tick == 0
                        && ![s.food, s.toy, s.distractor]
                            .into_iter()
                            .all(|t| visible_target(f, t))
                    {
                        return Err("composition lacks visible referents".into());
                    }
                    if let Some(bundle) = row.patch.selected_bundle() {
                        for channel in &bundle.channels {
                            if channel.channel == MotorChannel::Manipulation {
                                let p = channel.primitive.raw();
                                let t = channel.target.and_then(|t| t.entity);
                                first.get_or_insert((p, t));
                                attempted |= Some(channel.primitive) == primitive && t == target;
                            }
                        }
                    }
                    if let Some(t) = target {
                        let i = usize::from(t == s.toy);
                        acquired |= before[i] != Some(id) && after[i] == Some(id);
                    }
                    let consumed = consequences(&row, s).0;
                    meals += usize::from(consumed);
                    nourished +=
                        usize::from(consumed && row.patch.outcome().energy_delta.raw() > 0.);
                    let emitted = h
                        .world()
                        .audible_utterances()
                        .into_iter()
                        .filter(|u| {
                            u.source_kind == UtteranceSourceKind::Creature
                                && u.speaker_id == Some(id)
                                && u.emitted_tick == f.tick()
                        })
                        .collect::<Vec<_>>();
                    utterances.extend(
                        emitted
                            .iter()
                            .map(|u| u.tokens.iter().map(|t| t.raw()).collect::<Vec<_>>()),
                    );
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"linear_composition_evaluation","arm":arm,"layout":layout,"requested_words":words,"scene":s,"row":row,"owned_before":before,"owned_after":after,"actual_food_consumption":consumed,"emitted":emitted})
                    )?;
                }
            }
            let first_matches = words.map(|_| {
                first.is_some_and(|(p, t)| Some(p) == primitive.map(|p| p.raw()) && t == target)
            });
            results.push(json!({"kind":"composition_evaluation","arm":arm,"layout":layout,"world_seed":c.seed+390_000+layout as u64,"requested_words":words,"unseen_word_combination":words==Some([8,2]),"paired_initial_private_sha256":paired,"first_manipulation":first,"first_requested_primitive_target":first_matches,"requested_primitive_target_attempted":attempted,"requested_target_acquired":acquired,"actual_food_consumptions":meals,"nourished_meals":nourished,"actual_utterance_tokens":utterances,"teacher_present":false,"prior_enabled":false,"native_decisions":8,"note":"Unseen eat-toy attempts are not successful ingestion or proof of knowing inedibility."}));
        }
    }
    Ok(results)
}

pub fn continue_linear_grounded_language(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    if c.llamacpp_translation
        || !c.experimental_word_bridge
        || !(1..=2).contains(&c.rounds)
        || !(1..=64).contains(&c.epochs)
        || !(24..=32).contains(&c.decisions)
    {
        return Err("linear readout requires scripted explicit experiment, rounds1..2 epochs1..64 decisions24..32".into());
    }
    let old: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if old["kind"] != "experimental-action-gated-payload-v8" || old["config"]["seed"] != c.seed {
        return Err("linear readout only admits measured disposable V8".into());
    }
    let recorded = old["arms"]
        .as_array()
        .ok_or("no source arms")?
        .iter()
        .find(|a| a["arm"] == "grounded")
        .ok_or("no common source")?;
    fs::create_dir(output)?;
    let started = Instant::now();
    let mut traces = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("native-trace.jsonl.zst"))?,
        3,
    )?;
    let mut arms = vec![];
    let mut baseline = Value::Null;
    for arm in [Arm::Grounded, Arm::Shuffled, Arm::NoSpeech] {
        let r = read_recovery(&source.join("recovery-Grounded"))?;
        let p = PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048())?;
        let mut asset = FoundationWeightAsset::decode_canonical(&r.foundation)?;
        let prior = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor)?;
        let original = prior.checkpoint();
        let original_hash = identity(&original)?;
        if recorded["actor_sha256"] != original_hash {
            return Err("source not measured V8 actor".into());
        }
        let old_private = CpuCanonicalHost::from_checkpoint(r.host, &prior)?.checkpoint(&prior)?;
        let (mut actor, _) = prior.fork_linear_language_readout_experiment(&r.inputs)?;
        let copied = actor.checkpoint();
        if copied.weights != original.weights
            || copied.first_moment != original.first_moment
            || copied.second_moment != original.second_moment
            || copied.update_ages != original.update_ages
            || copied.optimizer_step != original.optimizer_step
        {
            return Err("readout fork reset native history".into());
        }
        if CpuCanonicalHost::from_checkpoint(old_private, &actor).is_ok() {
            return Err("old private life admitted into new profile".into());
        }
        let ids=actor.phenotype().synapses().iter().enumerate().filter_map(|(i,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(d)if matches!(d.head(),DecoderHeadKind::ActionCandidate|DecoderHeadKind::SpeechPayload)).then_some(i as u32)).collect::<Vec<_>>();
        actor.set_stage_mask(StageTrainableMask::from_synapse_indices(
            actor.phenotype(),
            &ids,
        )?)?;
        let initial = identity(&actor.checkpoint())?;
        let start = actor.optimizer_step();
        let plan = AssessmentPlan {
            recovery_sleep: true,
            collect_utterance_timing: true,
            factorial_grounding: true,
            paired_request_focus: true,
            linear_readout: true,
            balanced_grounding_v1: false,
            balanced_grounding_v2: false,
            event_confirmed_speech_v1: false,
            unseen_report_transfer_v1: false,
            physical_before_speech_v1: false,
            captured_failure_replay_v1: false,
            replay_early_stop_v1: false,
        };
        if arm == Arm::Grounded {
            eprintln!("linear profile untreated baseline");
            let mut t = zstd::stream::write::Encoder::new(
                fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(output.join("common-baseline.jsonl.zst"))?,
                3,
            )?;
            baseline = json!(evaluate(&actor, &asset, c, Arm::Untreated, plan, &mut t)?);
            t.finish()?;
        }
        let (mut collections, mut updates, mut next) = (vec![], vec![], None);
        for round in 0..c.rounds {
            eprintln!("linear {arm:?} round{round} native collection");
            let (e, receipt) = collect(&actor, &asset, c, arm, round, &mut traces)?;
            eprintln!("linear collection {receipt}");
            collections.push(receipt);
            for epoch in 0..c.epochs {
                let (g, loss) = update_gradient(&actor, &e)?;
                let before = actor.checkpoint();
                let norm = actor.apply_gradients(&g)?;
                preserve_frozen(&before, &actor.checkpoint())?;
                updates.push(json!({"round":round,"epoch":epoch,"step":actor.optimizer_step(),"objective":loss,"gradient_norm":norm,"all_frozen_coordinates_exact":true}));
                next = Some(g);
            }
            asset = export(&mut actor, c.seed)?;
        }
        eprintln!("linear {arm:?} cold assessment");
        let cold = exact_reload_plan(
            &output.join(format!("recovery-{arm:?}")),
            &actor,
            &asset,
            RecoveredAssessment {
                config: c,
                arm,
                curriculum: Some(plan),
            },
            next.ok_or("no update")?,
        )?;
        arms.push(json!({"arm":arm,"source_actor_sha256":original_hash,"profile_initial_actor_sha256":initial,"source_step":start,"actor_step":actor.optimizer_step(),"actor_sha256":identity(&actor.checkpoint())?,"profile":actor.phenotype().experimental_language_input(),"collections":collections,"updates":updates,"evaluation":cold["native_private_and_optimizer_hashes"]["assessment"]["evaluation"],"continuity":cold,"old_private_state_rejected":true,"source_history_copied_exactly":true}));
        if identity(&prior.checkpoint())? != original_hash {
            return Err("source mutated".into());
        }
    }
    traces.finish()?;
    let report = json!({"kind":"experimental-linear-word-readout-v14","execution_domain":"native-cpu","config":c,"source_report_sha256":identity(&old)?,"common_baseline":baseline,"arms":arms,"elapsed_seconds":started.elapsed().as_secs_f64(),"teacher":"scripted ordinary spatial language","semantic_prior_enabled":false,"limitations":["New disposable V3 identity; no accepted brain/private migration or promotion.","Existing native action/speech decoder coordinates train; recurrent weights/moments/ages freeze. No separate text classifier or forced evaluation.","Common V8 prior history; no-request removes new teacher cues, not prior language or factual narration labels.","Unseen eat-toy is a composition/attempt probe; native inedibility and physical outcomes remain authoritative.","Induced RecoveryProtocol sleep, not spontaneous. No real SLM request or delivery."]});
    write_json(&output.join("results.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn linear_fork_keeps_native_history_rejects_old_private_and_freezes_recurrence() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let mut v1 = new_actor_profile(seed, &asset, true).unwrap();
        let (mut h, s) = host(seed, seed, &asset, &v1, 0, false).unwrap();
        let id = h.world().organism_entity_ids()[0].0;
        let pos = speech_position(h.world()).unwrap();
        h.world_mut()
            .emit_player_tokens(
                Some(id),
                pos,
                vec![
                    LanguageTokenId::new(8).unwrap(),
                    LanguageTokenId::new(1).unwrap(),
                ],
            )
            .unwrap();
        let row = h.step(&v1, 1.).unwrap().decisions.remove(0);
        let label = action_label(&row, Goal::EatFood, s, false).unwrap();
        let e = Episode {
            rows: vec![row],
            labels: vec![label],
            speech: vec![None],
        };
        let (g, _) = gradients_with_speech(&v1, &[e], false).unwrap();
        v1.apply_gradients(&g).unwrap();
        let asset = export(&mut v1, seed).unwrap();
        let (_, inputs) = compile_profile(seed, &asset, true).unwrap();
        let (v2, inputs) = v1.fork_embodied_language_input_experiment(&inputs).unwrap();
        let before = v2.checkpoint();
        let (old, _) = host(seed, seed, &asset, &v2, 0, false).unwrap();
        let (mut v3, _) = v2.fork_linear_language_readout_experiment(&inputs).unwrap();
        let copied = v3.checkpoint();
        assert_eq!(copied.weights, before.weights);
        assert_eq!(copied.first_moment, before.first_moment);
        assert_eq!(copied.second_moment, before.second_moment);
        assert_eq!(copied.update_ages, before.update_ages);
        assert!(copied.update_ages.iter().any(|x| *x > 0));
        assert_ne!(
            v2.phenotype().phenotype_hash(),
            v3.phenotype().phenotype_hash()
        );
        assert!(CpuCanonicalHost::from_checkpoint(old.checkpoint(&v2).unwrap(), &v3).is_err());
        assert!(v1.fork_linear_language_readout_experiment(&inputs).is_err());
        let ids=v3.phenotype().synapses().iter().enumerate().filter_map(|(i,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(d)if matches!(d.head(),DecoderHeadKind::ActionCandidate|DecoderHeadKind::SpeechPayload)).then_some(i as u32)).collect::<Vec<_>>();
        v3.set_stage_mask(StageTrainableMask::from_synapse_indices(v3.phenotype(), &ids).unwrap())
            .unwrap();
        let (e, _) = collect(
            &v3,
            &asset,
            LanguagePilotConfig {
                seed,
                epochs: 1,
                rounds: 1,
                decisions: 24,
                llamacpp_translation: false,
                experimental_word_bridge: true,
            },
            Arm::Grounded,
            0,
            &mut std::io::sink(),
        )
        .unwrap();
        let (g, _) = update_gradient(&v3, &e).unwrap();
        let frozen = v3.checkpoint();
        v3.apply_gradients(&g).unwrap();
        preserve_frozen(&frozen, &v3.checkpoint()).unwrap();
        assert_eq!(v3.optimizer_step(), before.optimizer_step + 1);
    }
}
