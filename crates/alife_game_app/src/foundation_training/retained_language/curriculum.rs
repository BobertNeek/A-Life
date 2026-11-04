//! Opt-in continuation of this pilot's own disposable founders, never a live save.
use super::*;

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(super) struct AssessmentPlan {
    recovery_sleep: bool,
}

#[derive(Clone, Serialize)]
struct CompletedActivity {
    goal: Goal,
    tick: u64,
    native_patch_sha256: String,
}

fn completion(
    row: &CpuCollectedDecision,
    scene: Scene,
    before: Option<OrganismId>,
    after: Option<OrganismId>,
) -> Result<Option<CompletedActivity>> {
    let (consumed, grabbed) = consequences(row, scene);
    let goal = if consumed {
        Some(Goal::EatFood)
    } else if grabbed && before != Some(row.organism_id) && after == Some(row.organism_id) {
        Some(Goal::GetToy)
    } else {
        None
    };
    goal.map(|goal| {
        Ok(CompletedActivity {
            goal,
            tick: row.patch.pre_action().tick.raw(),
            native_patch_sha256: identity(&row.patch)?,
        })
    })
    .transpose()
}

fn payload(frame: &PerceptionFrame, event: Option<&CompletedActivity>) -> ReplaySpeechTarget {
    let ready = frame
        .sensory()
        .language_context
        .heard_tokens
        .iter()
        .all(Option::is_none);
    let event = event.filter(|_| ready);
    let mut continuation = [0; 5];
    if let Some(event) = event {
        continuation[0] = event.goal.noun();
    }
    ReplaySpeechTarget {
        token: event.map(|e| e.goal.word()),
        continuation,
        act: SpeechActKind::Declare,
        weight: if event.is_some() { 1. } else { 0.25 },
    }
}

fn request_label(
    row: &CpuCollectedDecision,
    scene: Scene,
    goal: Goal,
    narrate: bool,
) -> Result<ImitationTarget> {
    let frame = row.patch.pre_action().perception();
    let chosen = if narrate {
        frame
            .candidates()
            .iter()
            .find(|c| c.kind == ActionKind::Vocalize)
    } else {
        vocabulary_reception_choice(
            frame,
            Some(u32::from(goal.word())),
            Some(scene.target(goal)),
            None,
        )
    };
    let mut label = candidate_label(row, chosen)?;
    // The native objective already supports both the representative and motors.
    // Teach the assembler's actual choice instead of allowing an unrelated
    // representative to accompany a correct conditional motor.
    label.representative = chosen.map(|c| 1u32 << c.candidate_index);
    Ok(label)
}

fn place(
    world: &mut HeadlessWorld,
    scene: Scene,
    layout: usize,
    distance: f32,
    reversed: bool,
) -> Result<()> {
    let body = world
        .entity(world.organism_entity_ids()[0].1)
        .ok_or("no body")?
        .clone();
    let side = if layout.is_multiple_of(2) { 1. } else { -1. } * if reversed { -1. } else { 1. };
    for (entity, bearing, radius) in [
        (scene.food, side * 0.58, distance),
        (scene.toy, -side * 0.58, distance + 0.05),
    ] {
        if world.entity(entity).is_some_and(|o| o.carried_by.is_some()) {
            return Err("fixture cannot relocate an acquired target".into());
        }
        world.editor_move_object(
            entity,
            Vec3f::new(
                body.position.x + radius * (body.body_yaw + bearing).cos(),
                body.position.y,
                body.position.z + radius * (body.body_yaw + bearing).sin(),
            ),
        )?;
    }
    Ok(())
}

fn base_host(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    seed: u64,
    founder: u64,
) -> Result<(CpuCanonicalHost, Scene)> {
    Ok(CpuCanonicalHost::new_prepared_cohort(
        game(seed, founder, asset)?,
        actor,
        seed as u32,
        |world| arrange(world, 1, true),
    )?)
}

fn collect(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    config: LanguagePilotConfig,
    arm: Arm,
    round: usize,
    translations: &[SpeechTranslationReceipt],
    traces: &mut impl Write,
) -> Result<(Vec<Episode>, Value)> {
    let mut episodes = Vec::new();
    let (mut positive, mut counter_goal, mut heard, mut actionable, mut events) = (0, 0, 0, 0, 0);
    let mut reward = 0f64;
    for layout in 0..4 {
        let seed = config.seed + 50_000 + (round * 4 + layout) as u64;
        let (mut base, scene) = base_host(actor, asset, seed, config.seed)?;
        place(
            base.world_mut(),
            scene,
            layout,
            0.36 + round as f32 * 0.08,
            false,
        )?;
        for requested in [Goal::EatFood, Goal::GetToy] {
            let mut host = base.snapshot();
            let id = host.world().organism_entity_ids()[0].0;
            let mut data = Episode {
                rows: Vec::new(),
                labels: Vec::new(),
                speech: Vec::new(),
            };
            let (mut pending, mut narration_left, mut request_done) =
                (None::<CompletedActivity>, 0usize, false);
            for tick in 0..config.decisions {
                if tick < 16 && !request_done && tick % 2 == 0 {
                    let crossed = arm == Arm::Shuffled && (layout / 2 + round).is_multiple_of(2);
                    let index = requested.index() ^ usize::from(crossed);
                    let position = speech_position(host.world())?;
                    alife_school::LanguageNursery::speak_in_world(
                        host.world_mut(),
                        id,
                        position,
                        translations[index].literal_tokens.clone(),
                    )?;
                }
                let before = host.world().entity(scene.toy).and_then(|o| o.carried_by);
                let step = host.step(actor, 1.)?;
                let after = host.world().entity(scene.toy).and_then(|o| o.carried_by);
                for row in step.decisions {
                    let frame = row.patch.pre_action().perception();
                    let is_heard = frame
                        .sensory()
                        .language_context
                        .heard_tokens
                        .iter()
                        .flatten()
                        .any(|h| h.source_kind == UtteranceSourceKind::Teacher);
                    heard += usize::from(is_heard);
                    let speech = payload(frame, pending.as_ref().filter(|_| narration_left > 0));
                    let narrate = speech.token.is_some();
                    let label = if narrate {
                        request_label(&row, scene, pending.as_ref().unwrap().goal, true)?
                    } else if is_heard {
                        request_label(&row, scene, requested, false)?
                    } else {
                        ImitationTarget {
                            representative: None,
                            motors: [None; 6],
                        }
                    };
                    positive += usize::from(narrate);
                    counter_goal += usize::from(
                        narrate && pending.as_ref().unwrap().goal.index() != requested.index(),
                    );
                    actionable += usize::from(label.representative.is_some());
                    if narrate {
                        narration_left -= 1;
                    }
                    let completed = completion(&row, scene, before, after)?;
                    if let Some(event) = &completed {
                        events += 1;
                        request_done |= event.goal.index() == requested.index();
                    }
                    reward += f64::from(row.biological_reward);
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"curriculum_training","arm":arm,"round":round,"layout":layout,"requested":requested,"scene":scene,"row":row,"owned_before":before,"owned_after":after,"label":{"representative":label.representative,"motors":label.motors},"speech_target":speech,"narration_evidence":pending,"completed":completed})
                    )?;
                    if completed.is_some() {
                        pending = completed;
                        narration_left = 3;
                    }
                    data.rows.push(row);
                    data.labels.push(label);
                    data.speech.push(Some(speech));
                }
            }
            episodes.push(data);
        }
    }
    Ok((
        episodes,
        json!({"round":round,"native_decisions":config.decisions*8,"teacher_heard_frames":heard,"action_label_rows":actionable,"positive_speech_label_rows":positive,"positive_labels_for_nonrequested_activity":counter_goal,"actual_completed_activities":events,"sum_native_biological_reward":reward}),
    ))
}

pub(super) fn evaluate(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    config: LanguagePilotConfig,
    arm: Arm,
    plan: AssessmentPlan,
    traces: &mut impl Write,
) -> Result<Vec<Value>> {
    let mut results = Vec::new();
    for (layout, sleep) in [(0, false), (1, false), (1, true)] {
        if sleep && !plan.recovery_sleep {
            continue;
        }
        let (mut base, scene) = base_host(
            actor,
            asset,
            config.seed + 60_000 + layout as u64,
            config.seed,
        )?;
        for _ in 0..2 {
            base.step(actor, 1.)?;
        }
        let id = base.world().organism_entity_ids()[0].0;
        let (mut sleeping, mut consolidations) = (0, 0);
        if sleep {
            base.request_recovery_sleep(id)?;
            for _ in 0..600 {
                let step = base.step(actor, 1.)?;
                sleeping += step.sleeping_organisms;
                consolidations += step.sleep_consolidations;
                writeln!(
                    traces,
                    "{}",
                    json!({"phase":"curriculum_recovery_sleep","arm":arm,"tick":base.world().tick(),"sleeping":step.sleeping_organisms,"consolidations":step.sleep_consolidations,"decisions":step.decisions})
                )?;
                if consolidations > 0
                    && base
                        .world()
                        .organism_registry()
                        .get(id)
                        .is_some_and(|r| r.sleep_phase() == alife_core::sleep::SleepPhase::Awake)
                {
                    break;
                }
            }
            if consolidations == 0
                || base
                    .world()
                    .organism_registry()
                    .get(id)
                    .is_none_or(|r| r.sleep_phase() != alife_core::sleep::SleepPhase::Awake)
            {
                return Err(
                    "native recovery sleep did not consolidate and wake within 600 steps".into(),
                );
            }
        }
        // Ordinary fixture placement after preparation/sleep, not a claimed
        // learner transition. Carried objects cannot be moved by this fixture.
        place(
            base.world_mut(),
            scene,
            layout,
            if layout == 0 { 0.48 } else { 0.85 },
            true,
        )?;
        let base_private = identity(&base.checkpoint(actor)?)?;
        for requested in [Some(Goal::EatFood), Some(Goal::GetToy), None] {
            let mut host = base.snapshot();
            let (mut pending, mut narration_left, mut first) =
                (None::<CompletedActivity>, 0usize, None::<Goal>);
            let (
                mut meals,
                mut nourished,
                mut acquisitions,
                mut utterances,
                mut correct_utterances,
                mut opportunities,
                mut failures,
                mut decisions,
            ) = (0, 0, 0, 0, 0, 0, 0, 0);
            let mut initial_visible = Value::Null;
            for tick in 0..config.decisions {
                if tick < 16 && first.is_none() && tick % 2 == 0 {
                    if let Some(goal) = requested {
                        let pos = speech_position(host.world())?;
                        host.world_mut().emit_player_tokens(
                            Some(id),
                            pos,
                            vec![
                                LanguageTokenId::new(goal.word())?,
                                LanguageTokenId::new(goal.noun())?,
                            ],
                        )?;
                    }
                }
                let before = host.world().entity(scene.toy).and_then(|o| o.carried_by);
                let step = host.step(actor, 1.)?;
                let after = host.world().entity(scene.toy).and_then(|o| o.carried_by);
                for row in step.decisions {
                    no_teacher_or_prior(&row)?;
                    if host.world().entity_id("nursery-teacher").is_some() {
                        return Err("teacher actor in curriculum assessment".into());
                    }
                    let frame = row.patch.pre_action().perception();
                    if initial_visible.is_null() {
                        initial_visible = json!({"food":visible_target(frame,scene.food),"toy":visible_target(frame,scene.toy),"distractor":visible_target(frame,scene.distractor)});
                    }
                    let target = payload(frame, pending.as_ref().filter(|_| narration_left > 0));
                    if target.token.is_some() {
                        opportunities += 1;
                        narration_left -= 1;
                    }
                    let emitted = host
                        .world()
                        .audible_utterances()
                        .into_iter()
                        .filter(|u| {
                            u.source_kind == UtteranceSourceKind::Creature
                                && u.speaker_id == Some(id)
                                && u.emitted_tick == frame.tick()
                        })
                        .collect::<Vec<_>>();
                    utterances += emitted.len();
                    correct_utterances += emitted
                        .iter()
                        .filter(|u| {
                            target.token.is_some_and(|t| {
                                u.tokens.iter().map(|w| w.raw()).collect::<Vec<_>>()
                                    == vec![t, target.continuation[0]]
                            })
                        })
                        .count();
                    let completed = completion(&row, scene, before, after)?;
                    if let Some(event) = &completed {
                        first.get_or_insert(event.goal);
                        if event.goal.index() == 0 {
                            meals += 1;
                            nourished += usize::from(row.patch.outcome().energy_delta.raw() > 0.);
                        } else {
                            acquisitions += 1;
                        }
                    }
                    decisions += 1;
                    failures += usize::from(!row.patch.outcome().success);
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"curriculum_evaluation","arm":arm,"layout":layout,"recovery_sleep":sleep,"requested":requested,"scene":scene,"row":row,"owned_before":before,"owned_after":after,"completed":completed,"grounded_speech_target":target,"narration_evidence":pending,"emitted":emitted})
                    )?;
                    if completed.is_some() {
                        pending = completed;
                        narration_left = 3;
                    }
                }
            }
            if !initial_visible
                .as_object()
                .is_some_and(|o| o.values().all(|v| v == true))
            {
                return Err("curriculum assessment lacks visible targets/distractor".into());
            }
            results.push(json!({"arm":arm,"layout":if layout==0{"relocated_contact"}else{"relocated_near"},"recovery_sleep":sleep,"sleep_trigger":if sleep{Some("RecoveryProtocol")}else{None},"sleeping_ticks":sleeping,"sleep_consolidations":consolidations,"paired_initial_private_sha256":base_private,"requested":requested,"first_completed_activity":first,"first_completion_matches_request":requested.zip(first).map(|(g,f)|g.index()==f.index()),"native_decisions":decisions,"initial_visible":initial_visible,"teacher_present":false,"prior_enabled":false,"consumptions":meals,"nourished_meals":nourished,"toy_acquisitions":acquisitions,"utterances":utterances,"correct_grounded_utterances":correct_utterances,"grounded_utterance_opportunities":opportunities,"joint_failures":failures}));
        }
    }
    Ok(results)
}

pub fn continue_language_curriculum(
    source: &Path,
    output: &Path,
    config: LanguagePilotConfig,
) -> Result<Value> {
    if !config.experimental_word_bridge
        || !(1..=2).contains(&config.rounds)
        || !(1..=64).contains(&config.epochs)
        || !(24..=64).contains(&config.decisions)
    {
        return Err(
            "curriculum needs explicit word bridge, rounds 1..2, epochs 1..64, decisions 24..64"
                .into(),
        );
    }
    let previous: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if previous["kind"] != "experimental-retained-grounded-language-v3"
        || previous["config"]["seed"] != config.seed
        || previous["config"]["experimental_word_bridge"] != true
    {
        return Err(
            "only the matching disposable word-bridge pilot can seed this continuation".into(),
        );
    }
    fs::create_dir(output)?;
    let started = Instant::now();
    let translations = teacher_input(config.llamacpp_translation)?;
    let mut traces = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output.join("native-trace.jsonl.zst"))?,
        3,
    )?;
    let mut arms = Vec::new();
    for arm in [Arm::Grounded, Arm::Shuffled] {
        let recovery = read_recovery(&source.join(format!("recovery-{arm:?}")))?;
        if recovery.kind != "disposable-retained-language-pilot-v1"
            || recovery.inputs.experimental_language_input().is_none()
        {
            return Err("source is not an explicit experimental public-founder recovery".into());
        }
        let p =
            PhenotypeCompiler::compile_validated(&recovery.inputs, &BrainCapacityClass::n2048())?;
        let mut asset = FoundationWeightAsset::decode_canonical(&recovery.foundation)?;
        let mut actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), recovery.actor)?;
        let retained = CpuCanonicalHost::from_checkpoint(recovery.host, &actor)?;
        let prior = previous["arms"]
            .as_array()
            .ok_or("missing source arms")?
            .iter()
            .find(|v| v["arm"] == json!(arm))
            .ok_or("missing matched source arm")?;
        if identity(&actor.checkpoint())? != prior["actor_sha256"] {
            return Err("source actor differs from recorded measured arm".into());
        }
        let source_actor = identity(&actor.checkpoint())?;
        let source_private = identity(&retained.checkpoint(&actor)?)?;
        let start_step = actor.optimizer_step();
        let (mut collections, mut updates, mut next_gradient) = (Vec::new(), Vec::new(), None);
        for round in 0..config.rounds {
            eprintln!("grounded curriculum {arm:?} round {round}: paired native collection");
            let (episodes, receipt) = collect(
                &actor,
                &asset,
                config,
                arm,
                round,
                &translations,
                &mut traces,
            )?;
            eprintln!("collection receipt {receipt}");
            collections.push(receipt);
            for epoch in 0..config.epochs {
                let (gradient, objective) = gradients(&actor, &episodes)?;
                let before = actor.checkpoint();
                let norm = actor.apply_gradients(&gradient)?;
                preserve_frozen(&before, &actor.checkpoint())?;
                updates.push(json!({"round":round,"epoch":epoch,"step":actor.optimizer_step(),"objective":objective,"gradient_norm":norm,"frozen_coordinates_exact":true}));
                next_gradient = Some(gradient);
            }
            asset = export(&mut actor, config.seed)?;
        }
        eprintln!("grounded curriculum {arm:?}: cold assessment and recovery sleep");
        let continuity = exact_reload_plan(
            &output.join(format!("recovery-{arm:?}")),
            &actor,
            &asset,
            RecoveredAssessment {
                config,
                arm,
                curriculum: Some(AssessmentPlan {
                    recovery_sleep: true,
                }),
            },
            next_gradient.ok_or("no native update")?,
        )?;
        let evaluation =
            continuity["native_private_and_optimizer_hashes"]["assessment"]["evaluation"].clone();
        arms.push(json!({"arm":arm,"source_actor_sha256":source_actor,"source_private_sha256":source_private,"source_step":start_step,"actor_step":actor.optimizer_step(),"actor_sha256":identity(&actor.checkpoint())?,"collections":collections,"updates":updates,"evaluation":evaluation,"continuity":continuity}));
    }
    traces.finish()?;
    let report = json!({"kind":"experimental-request-contingent-language-v4","execution_domain":"native-cpu","config":config,"source_report_sha256":identity(&previous)?,"teacher_translation_deliveries":translations,"semantic_prior_enabled":false,"arms":arms,"elapsed_seconds":started.elapsed().as_secs_f64(),"limitations":["Scripted teacher unless verified optional provider receipts exist.","Continues both previously measured treatment histories; fresh cohorts preserve the actual native actor/Adam state, not same-individual online training.","Only audible requests or genuine completed-activity narration have action labels. Quiet behavior remains unconstrained.","Narration labels actual completed activity, including nonrequested activity; this is separate from request compliance.","Contact-first lessons and relocated contact/near assessment; no new far-navigation claim.","Sleep assessment uses explicit native RecoveryProtocol sleep, not spontaneous natural sleep.","Conditional recurrent replay retains recorded fast/lifetime offsets and memory context."]});
    write_json(&output.join("results.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_native_requests_share_world_context_and_quiet_rows_have_no_hidden_action_goal() {
        let seed = 847_501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor_profile(seed, &asset, true).unwrap();
        let config = LanguagePilotConfig {
            seed,
            epochs: 1,
            rounds: 1,
            decisions: 24,
            llamacpp_translation: false,
            experimental_word_bridge: true,
        };
        let (episodes, receipt) = collect(
            &actor,
            &asset,
            config,
            Arm::Grounded,
            0,
            &teacher_input(false).unwrap(),
            &mut std::io::sink(),
        )
        .unwrap();
        assert_eq!(episodes.len(), 8);
        assert!(receipt["teacher_heard_frames"].as_u64().unwrap() > 0);
        assert!(receipt["action_label_rows"].as_u64().unwrap() > 0);
        for pair in episodes.chunks_exact(2) {
            let left = &pair[0].rows[0];
            let right = &pair[1].rows[0];
            assert_eq!(left.initial, right.initial);
            assert_eq!(left.replay_tick.candidates, right.replay_tick.candidates);
            assert_eq!(
                left.replay_tick.enabled_routes,
                right.replay_tick.enabled_routes
            );
            assert_ne!(
                left.replay_tick.encoded_inputs,
                right.replay_tick.encoded_inputs
            );
        }
        for episode in &episodes {
            for (i, row) in episode.rows.iter().enumerate() {
                let heard = row
                    .patch
                    .pre_action()
                    .perception()
                    .sensory()
                    .language_context
                    .heard_tokens
                    .iter()
                    .flatten()
                    .any(|h| h.source_kind == UtteranceSourceKind::Teacher);
                let narration = episode.speech[i].is_some_and(|s| s.token.is_some());
                assert!(!episode.labels[i].representative.is_some() || heard || narration);
                if !heard && !narration {
                    assert!(episode.labels[i].motors.iter().all(Option::is_none));
                }
            }
        }
        let (gradient, _) = gradients(&actor, &episodes).unwrap();
        assert!(gradient.iter().any(|g| g.abs() > 0.));
    }
}
