//! Three real commands separate noun choice from action choice; align narration timing.
use super::*;
pub(super) mod balanced;
pub use balanced::{
    continue_balanced_grounded_language, continue_event_confirmed_speech, continue_report_choice,
    continue_visible_balanced_grounded_language,
};
mod contrastive;
mod focus;
pub(super) mod readout;
pub use readout::continue_linear_grounded_language;
#[cfg(test)]
mod token_probe;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Lesson {
    EatFood,
    GetFood,
    GetToy,
}
impl Lesson {
    const ALL: [Self; 3] = [Self::EatFood, Self::GetFood, Self::GetToy];
    fn index(self) -> usize {
        match self {
            Self::EatFood => 0,
            Self::GetFood => 1,
            Self::GetToy => 2,
        }
    }
    fn words(self) -> [u16; 2] {
        match self {
            Self::EatFood => [8, 1],
            Self::GetFood => [9, 1],
            Self::GetToy => [9, 2],
        }
    }
    fn target(self, s: Scene) -> WorldEntityId {
        if self == Self::GetToy {
            s.toy
        } else {
            s.food
        }
    }
}
#[derive(Clone, Serialize)]
struct Event {
    lesson: Lesson,
    tick: u64,
    patch_sha256: String,
}
fn owners(w: &HeadlessWorld, s: Scene) -> [Option<OrganismId>; 2] {
    [s.food, s.toy].map(|id| w.entity(id).and_then(|o| o.carried_by))
}
fn activity(
    row: &CpuCollectedDecision,
    s: Scene,
    before: [Option<OrganismId>; 2],
    after: [Option<OrganismId>; 2],
) -> Result<Option<Event>> {
    let consumed = consequences(row, s).0;
    let mut lesson = consumed.then_some(Lesson::EatFood);
    for (i, target) in [s.food, s.toy].into_iter().enumerate() {
        let selected = row.patch.selected_bundle().is_some_and(|b| {
            b.channels.iter().any(|c| {
                c.primitive == HeadlessActionIds::GRAB
                    && c.target.and_then(|t| t.entity) == Some(target)
            })
        });
        if selected && before[i] != Some(row.organism_id) && after[i] == Some(row.organism_id) {
            lesson = Some(if i == 0 {
                Lesson::GetFood
            } else {
                Lesson::GetToy
            });
        }
    }
    lesson
        .map(|lesson| {
            Ok(Event {
                lesson,
                tick: row.patch.pre_action().tick.raw(),
                patch_sha256: identity(&row.patch)?,
            })
        })
        .transpose()
}
fn narration(
    frame: &PerceptionFrame,
    event: Option<&Event>,
    left: usize,
) -> Option<ReplaySpeechTarget> {
    let event = event.filter(|_| {
        left > 0
            && frame
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .all(Option::is_none)
    })?;
    let mut continuation = [0; 5];
    continuation[0] = event.lesson.words()[1];
    Some(ReplaySpeechTarget {
        token: Some(event.lesson.words()[0]),
        continuation,
        act: SpeechActKind::Declare,
        weight: 1.,
    })
}
fn instruction_choice(
    frame: &PerceptionFrame,
    s: Scene,
    requested: Lesson,
    grasp_legal: bool,
    requested_held: bool,
) -> Option<&ActionCandidate> {
    if requested != Lesson::EatFood && requested_held {
        return frame
            .candidates()
            .iter()
            .find(|c| c.family == CandidateActionFamily::Idle);
    }
    if grasp_legal {
        let primitive = if requested == Lesson::EatFood {
            HeadlessActionIds::EAT
        } else {
            HeadlessActionIds::GRAB
        };
        if let Some(candidate) = frame
            .candidates()
            .iter()
            .find(|c| c.action_id == primitive && c.target.entity == Some(requested.target(s)))
        {
            return Some(candidate);
        }
    }
    vocabulary_reception_choice(
        frame,
        Some(u32::from(requested.words()[0])),
        Some(requested.target(s)),
        None,
    )
}
fn action_target(
    row: &CpuCollectedDecision,
    s: Scene,
    requested: Lesson,
    narrate: bool,
    heard: bool,
    grasp_legal: bool,
    requested_held: bool,
) -> Result<ImitationTarget> {
    if row.action.motor_masks[3] != 0 {
        return Err("factorial experiment requires representative-only vocal arbitration".into());
    }
    let frame = row.patch.pre_action().perception();
    let chosen = if narrate {
        frame
            .candidates()
            .iter()
            .find(|c| c.kind == ActionKind::Vocalize)
    } else if heard {
        instruction_choice(frame, s, requested, grasp_legal, requested_held)
    } else {
        None
    };
    if let Some(c) = chosen {
        let mut target = candidate_label(row, Some(c))?;
        target.representative = Some(1 << c.candidate_index);
        Ok(target)
    } else {
        let voice = frame
            .candidates()
            .iter()
            .filter(|c| c.kind == ActionKind::Vocalize)
            .fold(0, |m, c| m | (1 << c.candidate_index));
        let allowed = row.action.representative_mask & !voice;
        if allowed == 0 {
            return Err("no quiet physical representative".into());
        }
        Ok(ImitationTarget {
            representative: Some(allowed),
            motors: [None; 6],
        })
    }
}
fn collect_factorial(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    c: LanguagePilotConfig,
    arm: Arm,
    round: usize,
    traces: &mut impl Write,
    focused: bool,
) -> Result<(Vec<Episode>, Value)> {
    let (mut episodes, mut counts, mut heard, mut positive) = (vec![], [0usize; 3], 0usize, 0usize);
    let mut lag = vec![];
    for layout in 0..2 {
        let (mut base, s) = base_host(
            actor,
            asset,
            c.seed + 80_000 + (round * 2 + layout) as u64,
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
            let (mut pending, mut left, mut complete) = (None::<Event>, 0usize, false);
            let mut e = Episode {
                rows: vec![],
                labels: vec![],
                speech: vec![],
            };
            for tick in 0..c.decisions {
                if arm != Arm::NoSpeech && tick < 16 && !complete && tick % 2 == 0 {
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
                let legal_grasp = h.world().can_grab_target(id, requested.target(s));
                let before = owners(h.world(), s);
                let requested_held = before[usize::from(requested == Lesson::GetToy)] == Some(id);
                let step = h.step(actor, 1.)?;
                let after = owners(h.world(), s);
                for row in step.decisions {
                    let f = row.patch.pre_action().perception();
                    let is_heard = f
                        .sensory()
                        .language_context
                        .heard_tokens
                        .iter()
                        .flatten()
                        .any(|t| t.source_kind == UtteranceSourceKind::Teacher);
                    heard += usize::from(is_heard);
                    let speech = narration(f, pending.as_ref(), left);
                    let label = action_target(
                        &row,
                        s,
                        requested,
                        speech.is_some(),
                        is_heard || (focused && tick == 0),
                        legal_grasp,
                        requested_held,
                    )?;
                    if speech.is_some() {
                        positive += 1;
                        left -= 1;
                        lag.push(
                            row.patch.pre_action().tick.raw() - pending.as_ref().unwrap().tick,
                        );
                    }
                    let event = activity(&row, s, before, after)?;
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"factorial_training","arm":arm,"round":round,"layout":layout,"requested":requested,"scene":s,"row":row,"owned_before":before,"owned_after":after,"label":{"representative":label.representative,"motors":label.motors},"speech_target":speech,"narration_evidence":pending,"completed":event})
                    )?;
                    if let Some(event) = event {
                        counts[event.lesson.index()] += 1;
                        complete = true;
                        pending = Some(event);
                        left = 3;
                    }
                    e.rows.push(row);
                    e.labels.push(label);
                    e.speech.push(speech);
                }
            }
            episodes.push(e);
        }
    }
    let native_decisions = episodes.iter().map(|e| e.rows.len()).sum::<usize>();
    Ok((
        episodes,
        json!({"round":round,"native_decisions":native_decisions,"teacher_heard_frames":heard,"actual_activity_counts":counts,"positive_narration_rows":positive,"positive_label_lags_ticks":lag,"training_distance":0.36+round as f32*0.08,"cue_stops_after_any_actual_activity":true}),
    ))
}
fn balanced_activity_payload(
    actor: &CpuFoundationTrainer,
    e: &[Episode],
) -> Result<(Vec<f32>, Value)> {
    let mut mass = [0f32; 3];
    for t in e.iter().flat_map(|e| e.speech.iter().flatten()) {
        let i = Lesson::ALL
            .iter()
            .position(|l| l.words() == [t.token.unwrap(), t.continuation[0]])
            .ok_or("unknown factual payload class")?;
        mass[i] += t.weight * t.len() as f32;
    }
    let classes = mass.iter().filter(|x| **x > 0.).count();
    let (mut g, mut losses) = (vec![0.; actor.weights().len()], [0f64; 3]);
    for episode in e {
        if episode.rows.is_empty() {
            continue;
        }
        let q = sequence(actor, episode, episode.rows.len(), 0);
        let zero = q
            .ticks
            .iter()
            .map(|t| vec![0.; t.candidates.len()])
            .collect::<Vec<_>>();
        for class in 0..3 {
            if mass[class] == 0. {
                continue;
            }
            let targets = episode
                .speech
                .iter()
                .map(|t| {
                    t.filter(|t| {
                        Lesson::ALL[class].words() == [t.token.unwrap(), t.continuation[0]]
                    })
                })
                .collect::<Vec<_>>();
            let amount = targets
                .iter()
                .flatten()
                .map(|t| t.weight * t.len() as f32)
                .sum::<f32>();
            if amount == 0. {
                continue;
            }
            let p = actor.probe_replay_gradients_with_speech(&q, &zero, &targets)?;
            let scale = amount / mass[class] / classes as f32;
            losses[class] += p.objective * f64::from(scale);
            for (dst, src) in g.iter_mut().zip(p.gradients) {
                *dst += src * scale;
            }
        }
    }
    Ok((
        g,
        json!({"observed_activity_classes":classes,"actual_token_mass_by_activity":mass,"positive_loss_by_activity":losses,"missing_classes_not_fabricated":true}),
    ))
}

pub(super) fn evaluate_factorial(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    c: LanguagePilotConfig,
    arm: Arm,
    plan: AssessmentPlan,
    traces: &mut impl Write,
) -> Result<Vec<Value>> {
    let mut result = vec![];
    for (layout, sleep) in [(0, false), (1, false), (1, true)] {
        if sleep && !plan.recovery_sleep {
            continue;
        }
        let (mut base, s) = base_host(
            actor,
            asset,
            c.seed
                + if plan.linear_readout {
                    290_000
                } else if plan.paired_request_focus {
                    190_000
                } else {
                    90_000
                }
                + layout as u64,
            c.seed,
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
                    json!({"phase":"factorial_recovery_sleep","arm":arm,"tick":base.world().tick(),"sleeping":step.sleeping_organisms,"consolidations":step.sleep_consolidations,"decisions":step.decisions})
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
                return Err("factorial native sleep did not consolidate and wake".into());
            }
        }
        if plan.paired_request_focus {
            focus::held_out_place(base.world_mut(), s, layout)?;
        } else {
            place(
                base.world_mut(),
                s,
                layout,
                if layout == 0 { 0.48 } else { 0.85 },
                true,
            )?;
        }
        let paired = identity(&base.checkpoint(actor)?)?;
        for requested in [
            Some(Lesson::EatFood),
            Some(Lesson::GetFood),
            Some(Lesson::GetToy),
            None,
        ] {
            let mut h = base.snapshot();
            let (mut pending, mut left, mut first) = (None::<Event>, 0usize, None::<Lesson>);
            let (
                mut counts,
                mut speech_count,
                mut correct,
                mut opportunities,
                mut outside,
                mut nourished,
            ) = ([0usize; 3], 0, 0, 0, 0, 0);
            let mut visible = false;
            let mut native_decisions = 0usize;
            for tick in 0..c.decisions {
                if tick < 16 && first.is_none() && tick % 2 == 0 {
                    if let Some(goal) = requested {
                        let pos = speech_position(h.world())?;
                        h.world_mut().emit_player_tokens(
                            Some(id),
                            pos,
                            goal.words()
                                .into_iter()
                                .map(LanguageTokenId::new)
                                .collect::<std::result::Result<_, _>>()?,
                        )?;
                    }
                }
                let before = owners(h.world(), s);
                let step = h.step(actor, 1.)?;
                let after = owners(h.world(), s);
                for row in step.decisions {
                    native_decisions += 1;
                    no_teacher_or_prior(&row)?;
                    if h.world().entity_id("nursery-teacher").is_some() {
                        return Err("teacher in factorial assessment".into());
                    }
                    let f = row.patch.pre_action().perception();
                    if tick == 0 {
                        visible = [s.food, s.toy, s.distractor]
                            .into_iter()
                            .all(|x| visible_target(f, x));
                    }
                    let target = narration(f, pending.as_ref(), left);
                    if target.is_some() {
                        opportunities += 1;
                        left -= 1;
                    }
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
                    speech_count += emitted.len();
                    outside += usize::from(target.is_none()) * emitted.len();
                    correct += emitted
                        .iter()
                        .filter(|u| {
                            target.is_some_and(|t| {
                                u.tokens.iter().map(|t| t.raw()).collect::<Vec<_>>()
                                    == vec![t.token.unwrap(), t.continuation[0]]
                            })
                        })
                        .count();
                    let event = activity(&row, s, before, after)?;
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"factorial_evaluation","arm":arm,"layout":layout,"recovery_sleep":sleep,"requested":requested,"scene":s,"row":row,"owned_before":before,"owned_after":after,"grounded_speech_target":target,"narration_evidence":pending,"completed":event,"emitted":emitted})
                    )?;
                    if let Some(event) = event {
                        first.get_or_insert(event.lesson);
                        counts[event.lesson.index()] += 1;
                        if event.lesson == Lesson::EatFood
                            && row.patch.outcome().energy_delta.raw() > 0.
                        {
                            nourished += 1
                        }
                        pending = Some(event);
                        left = 3;
                    }
                }
            }
            if !visible {
                return Err("factorial assessment lacks visible food/toy/distractor".into());
            }
            result.push(json!({"arm":arm,"layout":layout,"recovery_sleep":sleep,"sleep_trigger":if sleep{Some("RecoveryProtocol")}else{None},"sleeping_ticks":sleeping,"sleep_consolidations":consolidations,"paired_initial_private_sha256":paired,"requested":requested,"first_completed_activity":first,"first_completion_matches_request":requested.zip(first).map(|(a,b)|a==b),"requested_activity_completed":requested.map(|l|counts[l.index()]>0),"actual_activity_counts":counts,"nourished_meals":nourished,"utterances":speech_count,"correct_grounded_utterances":correct,"grounded_utterance_opportunities":opportunities,"outside_primary_window_utterances":outside,"native_decisions":native_decisions,"initial_targets_and_distractor_visible":visible,"teacher_present":false,"prior_enabled":false}));
        }
    }
    Ok(result)
}

pub fn continue_factorial_grounded_language(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run_factorial(source, output, c, false, false)
}
pub fn continue_focused_grounded_language(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run_factorial(source, output, c, true, false)
}
pub fn continue_contrastive_grounded_language(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run_factorial(source, output, c, true, true)
}
fn run_factorial(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
    focused: bool,
    paired_margin: bool,
) -> Result<Value> {
    if c.llamacpp_translation
        || !c.experimental_word_bridge
        || !(1..=2).contains(&c.rounds)
        || !(1..=32).contains(&c.epochs)
        || !(24..=32).contains(&c.decisions)
    {
        return Err("factorial pilot needs scripted teacher, explicit bridge, rounds1..2 epochs1..32 decisions24..32".into());
    }
    let old: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    if old["kind"] != "experimental-action-gated-payload-v8" || old["config"]["seed"] != c.seed {
        return Err("factorial continuation only admits matching measured disposable V8".into());
    }
    let recorded = old["arms"]
        .as_array()
        .ok_or("no measured arms")?
        .iter()
        .find(|a| a["arm"] == "grounded")
        .ok_or("missing common grounded source")?;
    fs::create_dir(output)?;
    let started = Instant::now();
    let mut trace = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output.join("native-trace.jsonl.zst"))?,
        3,
    )?;
    let (mut arms, mut baseline) = (vec![], Value::Null);
    for arm in [Arm::Grounded, Arm::Shuffled, Arm::NoSpeech] {
        let r = read_recovery(&source.join("recovery-Grounded"))?;
        if r.kind != "disposable-retained-language-pilot-v1"
            || r.inputs.experimental_language_input()
                != Some(ExperimentalLanguageInputV1::TwoTokenAndEmbodiedSpeechV2)
        {
            return Err("not a measured public V2 founder".into());
        }
        let p = PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048())?;
        let mut asset = FoundationWeightAsset::decode_canonical(&r.foundation)?;
        let mut actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor)?;
        let retained = CpuCanonicalHost::from_checkpoint(r.host, &actor)?;
        let initial = identity(&actor.checkpoint())?;
        if initial != recorded["actor_sha256"] {
            return Err("source differs from measured V8".into());
        }
        let source_private = identity(&retained.checkpoint(&actor)?)?;
        let start = actor.optimizer_step();
        let plan = AssessmentPlan {
            recovery_sleep: true,
            collect_utterance_timing: true,
            factorial_grounding: true,
            paired_request_focus: focused,
            linear_readout: false,
            balanced_grounding_v1: false,
            balanced_grounding_v2: false,
            event_confirmed_speech_v1: false,
        };
        if arm == Arm::Grounded {
            eprintln!("factorial common baseline");
            let file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(output.join("common-baseline.jsonl.zst"))?;
            let mut t = zstd::stream::write::Encoder::new(file, 3)?;
            baseline = json!(evaluate_factorial(
                &actor,
                &asset,
                c,
                Arm::Untreated,
                plan,
                &mut t
            )?);
            t.finish()?;
        }
        let (mut collections, mut updates, mut next) = (vec![], vec![], None);
        for round in 0..c.rounds {
            eprintln!("factorial {arm:?} round{round} native collection");
            let (e, receipt) =
                collect_factorial(&actor, &asset, c, arm, round, &mut trace, focused)?;
            eprintln!("factorial collection {receipt}");
            collections.push(receipt);
            for epoch in 0..c.epochs {
                let (mut g, action) = if focused {
                    focus::gradients(&actor, &e)?
                } else {
                    gradients_with_speech(&actor, &e, false)?
                };
                let pair_objective = if paired_margin {
                    let (pair, receipt) = contrastive::gradients(&actor, &e)?;
                    for (dst, src) in g.iter_mut().zip(pair) {
                        *dst += 2. * src;
                    }
                    Some(receipt)
                } else {
                    None
                };
                let (payload, objective) = balanced_activity_payload(&actor, &e)?;
                for (dst, src) in g.iter_mut().zip(payload) {
                    *dst += src
                }
                let before = actor.checkpoint();
                let norm = actor.apply_gradients(&g)?;
                preserve_frozen(&before, &actor.checkpoint())?;
                updates.push(json!({"round":round,"epoch":epoch,"step":actor.optimizer_step(),"action":action,"paired_margin":pair_objective,"payload":objective,"gradient_norm":norm,"frozen_coordinates_exact":true}));
                next = Some(g);
            }
            asset = export(&mut actor, c.seed)?;
        }
        eprintln!("factorial {arm:?} cold assessment");
        let continuity = exact_reload_plan(
            &output.join(format!("recovery-{arm:?}")),
            &actor,
            &asset,
            RecoveredAssessment {
                config: c,
                arm,
                curriculum: Some(plan),
            },
            next.ok_or("no native update")?,
        )?;
        let evaluation =
            continuity["native_private_and_optimizer_hashes"]["assessment"]["evaluation"].clone();
        arms.push(json!({"arm":arm,"common_source_actor_sha256":initial,"source_private_sha256":source_private,"source_step":start,"actor_step":actor.optimizer_step(),"actor_sha256":identity(&actor.checkpoint())?,"collections":collections,"updates":updates,"evaluation":evaluation,"continuity":continuity}));
    }
    trace.finish()?;
    let report = json!({"kind":if paired_margin {"experimental-paired-margin-v13"} else if focused {"experimental-paired-request-focus-v12"} else {"experimental-factorial-hold-aware-v11"},"paired_request_focus":focused,"execution_domain":"native-cpu","config":c,"source_report_sha256":identity(&old)?,"lessons":Lesson::ALL,"training_distances":[0.36,0.44],"native_grasp_legality_for_teaching":true,"common_baseline":baseline,"semantic_prior_enabled":false,"teacher":"scripted ordinary spatial speech","arms":arms,"elapsed_seconds":started.elapsed().as_secs_f64(),"limitations":["One common retained V8 history and seed; NoSpeech removes incremental teacher requests, not prior learning or truthful offline activity labels.","Training and assessment both stop request speech after any genuine completed activity; narration keeps the original three eligible quiet-row criterion.","GetFood ownership distinguishes noun choice from GetToy and action choice from EatFood. Missing physical activity classes receive no invented payload labels.","First-event discrimination is reported separately from eventual requested completion: grabbing food can be a legitimate prefix to eating food.","Native actor/Adam continuation across fresh cohorts; exact private/cold/sleep recovery is measured, not same-individual live optimizer rebase.","Induced RecoveryProtocol sleep, relocated contact/near fixtures and visible distractor; no spontaneous sleep or far-navigation claim.","Multiple targeted curriculum changes; no equal-dose old-objective arm, no real SLM delivery claim."]});
    write_json(&output.join("results.json"), &report)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn get_food_is_a_distinct_real_acquisition_without_ingestion() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor_profile(seed, &asset, true).unwrap();
        let (mut h, s) = host(seed, seed, &asset, &actor, 0, false).unwrap();
        place(h.world_mut(), s, 0, 0.36, false).unwrap();
        let (id, body) = h.world().organism_entity_ids()[0];
        let bio = *h
            .world()
            .organism_registry()
            .get(id)
            .unwrap()
            .biochemistry();
        let perception_tick = h.world().tick();
        let frame = h
            .world_mut()
            .perception_frame(
                id,
                perception_tick,
                actor.phenotype().sensor_profile(),
                bio.homeostasis,
            )
            .unwrap();
        assert!(h.world().can_grab_target(id, s.food));
        let legacy = vocabulary_reception_choice(&frame, Some(9), Some(s.food), None).unwrap();
        assert_eq!(legacy.action_id, HeadlessActionIds::APPROACH);
        let chosen = instruction_choice(&frame, s, Lesson::GetFood, true, false).unwrap();
        assert_eq!(chosen.action_id, HeadlessActionIds::GRAB);
        assert_eq!(chosen.target.entity, Some(s.food));
        let cmd = chosen.to_command(id, Confidence::new(1.).unwrap()).unwrap();
        let tick = h.world().tick();
        let receipt = h
            .world_mut()
            .apply_registered_command(&cmd, body, Tick(tick.raw() + 1))
            .unwrap();
        assert_eq!(
            receipt.action_result.execution.physical.contact,
            PhysicalContactKind::Touch
        );
        assert_eq!(h.world().entity(s.food).unwrap().carried_by, Some(id));
        assert!(receipt.biology_after.body.energy <= bio.body.energy);
        assert!(h.world().can_grab_target(id, s.food)); // native same-target Grab releases
        assert!(!h.world().can_grab_target(id, s.toy)); // other object cannot occupy the grip
        h.world_mut().try_advance_tick().unwrap();
        let tick = h.world().tick();
        let after = *h
            .world()
            .organism_registry()
            .get(id)
            .unwrap()
            .biochemistry();
        let f = h
            .world_mut()
            .perception_frame(
                id,
                tick,
                actor.phenotype().sensor_profile(),
                after.homeostasis,
            )
            .unwrap();
        assert_eq!(
            instruction_choice(&f, s, Lesson::GetFood, true, true)
                .unwrap()
                .kind,
            ActionKind::Idle
        );
        assert_ne!(Lesson::GetFood.words(), Lesson::GetToy.words());
        assert_eq!(Lesson::GetFood.words()[0], Lesson::GetToy.words()[0]);
    }
}
