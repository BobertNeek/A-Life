//! Independent-seed, dimension-balanced continuation of measured public V3.
use super::*;
mod captured_replay;
mod frozen_causal;
pub use frozen_causal::assess_frozen_commands;
#[cfg(test)]
mod bounded_tape_probe;
#[cfg(test)]
mod detached_speech_inputs;
#[cfg(test)]
mod physical_priority_probe;
#[cfg(test)]
mod speech_factor_probe;
mod unseen_transfer;
#[cfg(test)]
mod within_seed_gate_validation;
#[cfg(test)]
mod witnessed_gate_retention;
pub use captured_replay::{
    compare_care_retention, compare_witnessed_early_stop, continue_witnessed_replay,
    preflight_care_retention, preflight_witnessed_replay,
};
pub use unseen_transfer::assess_unseen_grounding;
const HELD_OUT_SEEDS: [u64; 6] = [
    2_404_507, 2_509_943, 2_608_889, 2_700_419, 2_805_623, 2_907_709,
];
const INSTRUCTION_TICKS: usize = 8;
const EVALUATION_TICKS: usize = 24;
#[derive(Clone, Copy, Debug, Serialize)]
enum Group {
    Concrete(usize),
    Fulfilled,
    Narration,
    Other,
}
struct Sample {
    row: CpuCollectedDecision,
    target: ImitationTarget,
    speech: Option<ReplaySpeechTarget>,
    group: Group,
    verb_mask: u32,
    noun_mask: u32,
    report_correction: bool,
    physical_ordering: bool,
}

fn remap(world: &mut HeadlessWorld, s: Scene, padding: usize, toy_first: bool) -> Result<Scene> {
    remap_with_food(world, s, padding, toy_first, 1., 0.12)
}
fn remap_with_food(
    world: &mut HeadlessWorld,
    mut s: Scene,
    padding: usize,
    toy_first: bool,
    nutrition: f32,
    radius: f32,
) -> Result<Scene> {
    let b = world
        .entity(world.organism_entity_ids()[0].1)
        .ok_or("no body")?
        .clone();
    world.editor_remove_object(s.food)?;
    world.editor_remove_object(s.toy)?;
    for _ in 0..padding {
        let id = world.editor_spawn_object(WorldEditorSpawnSpec {
            label: "identity-padding".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(b.position.x + 30., b.position.y, b.position.z + 30.),
            nutrition: 0.,
            hazard_pain: 0.,
            radius: 0.1,
            token_id: None,
        })?;
        world.editor_remove_object(id)?;
    }
    let pos = Vec3f::new(b.position.x + 3., b.position.y, b.position.z);
    let food = |w: &mut HeadlessWorld| {
        w.editor_spawn_object(WorldEditorSpawnSpec {
            label: format!("balanced-food-{padding}"),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: pos,
            nutrition,
            hazard_pain: 0.,
            radius,
            token_id: None,
        })
    };
    if toy_first {
        s.toy = world.spawn_toy(&format!("balanced-toy-{padding}"), pos, true)?;
        s.food = food(world)?;
    } else {
        s.food = food(world)?;
        s.toy = world.spawn_toy(&format!("balanced-toy-{padding}"), pos, true)?;
    }
    Ok(s)
}
fn prepared(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    founder: u64,
    seed: u64,
    padding: usize,
    toy_first: bool,
) -> Result<(CpuCanonicalHost, Scene)> {
    Ok(CpuCanonicalHost::new_prepared_cohort(
        game(seed, founder, asset)?,
        actor,
        seed as u32,
        |w| {
            let s = arrange(w, 1, true)?;
            remap(w, s, padding, toy_first)
        },
    )?)
}
fn place_balanced(
    w: &mut HeadlessWorld,
    s: Scene,
    layout: usize,
    held_out: bool,
    visible: bool,
) -> Result<()> {
    let b = w
        .entity(w.organism_entity_ids()[0].1)
        .ok_or("no body")?
        .clone();
    let (food_angle, toy_angle, food_d, toy_d) = if visible && held_out {
        [
            (-0.39, 0.75, 0.40, 0.64),
            (0.58, -0.73, 0.45, 0.69),
            (-0.47, 0.74, 0.67, 0.73),
            (0.65, -0.72, 0.72, 0.78),
            (-0.52, 0.76, 0.56, 0.72),
            (0.44, -0.75, 0.62, 0.77),
        ][layout]
    } else if visible {
        [
            (0.42, -0.74, 0.38, 0.62),
            (-0.54, 0.72, 0.42, 0.65),
            (0.62, -0.76, 0.46, 0.68),
            (-0.46, 0.74, 0.48, 0.70),
        ][layout]
    } else if held_out {
        [
            (-0.31, 0.69, 0.39, 0.44),
            (0.71, -0.29, 0.43, 0.38),
            (-0.46, 0.63, 0.68, 0.74),
            (0.67, -0.35, 0.76, 0.69),
            (-0.39, 0.73, 0.52, 0.57),
            (0.33, -0.71, 0.61, 0.55),
        ][layout]
    } else {
        [
            (0.45, -0.65, 0.34, 0.40),
            (-0.65, 0.45, 0.38, 0.43),
            (0.65, -0.45, 0.40, 0.35),
            (-0.45, 0.65, 0.42, 0.37),
        ][layout]
    };
    for (id, a, d) in [(s.food, food_angle, food_d), (s.toy, toy_angle, toy_d)] {
        if w.entity(id).is_some_and(|e| e.carried_by.is_some()) {
            return Err("cannot relocate held target".into());
        }
        w.editor_move_object(
            id,
            Vec3f::new(
                b.position.x + d * (b.body_yaw + a).cos(),
                b.position.y,
                b.position.z + d * (b.body_yaw + a).sin(),
            ),
        )?;
    }
    Ok(())
}
fn physical_event(
    row: &CpuCollectedDecision,
    s: Scene,
    before: [Option<OrganismId>; 2],
    after: [Option<OrganismId>; 2],
) -> Result<Option<Event>> {
    let lesson = if consequences(row, s).0 {
        Some(Lesson::EatFood)
    } else if before[0] != Some(row.organism_id) && after[0] == Some(row.organism_id) {
        Some(Lesson::GetFood)
    } else if before[1] != Some(row.organism_id) && after[1] == Some(row.organism_id) {
        Some(Lesson::GetToy)
    } else {
        None
    };
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
fn confirmed_speech(
    event: Option<&Event>,
    left: usize,
    requested: Option<Lesson>,
) -> Option<ReplaySpeechTarget> {
    let event = event.filter(|e| left > 0 && Some(e.lesson) == requested)?;
    let mut continuation = [0; 5];
    continuation[0] = event.lesson.words()[1];
    Some(ReplaySpeechTarget {
        token: Some(event.lesson.words()[0]),
        continuation,
        act: SpeechActKind::Declare,
        weight: 1.,
    })
}
fn candidate_mask(frame: &PerceptionFrame, pairs: &[(ActionId, WorldEntityId)]) -> u32 {
    frame
        .candidates()
        .iter()
        .filter(|c| {
            pairs
                .iter()
                .any(|(p, t)| c.action_id == *p && c.target.entity == Some(*t))
        })
        .fold(0, |m, c| m | (1 << c.candidate_index))
}
#[derive(Clone, Copy)]
struct TeachingProtocol {
    visible: bool,
    event_speech: bool,
    report_correction: bool,
    physical_ordering: bool,
}
fn collect(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    c: LanguagePilotConfig,
    arm: Arm,
    round: usize,
    traces: &mut impl Write,
    protocol: TeachingProtocol,
) -> Result<(Vec<Sample>, Value)> {
    let TeachingProtocol {
        visible,
        event_speech,
        report_correction,
        physical_ordering,
    } = protocol;
    let (mut samples, mut events, mut heard) = (vec![], [0usize; 3], 0usize);
    let (mut speech_rows, mut speech_with_heard) = (0usize, 0usize);
    let (mut legal_reports, mut cooldown_suppressed, mut actual_reports) = (0usize, 0usize, 0usize);
    for layout in 0..4 {
        let seed = c.seed + 600_000 + (round * 4 + layout) as u64;
        let (mut base, s) = prepared(
            actor,
            asset,
            c.seed,
            seed,
            2 + round * 8 + layout,
            !layout.is_multiple_of(2),
        )?;
        place_balanced(base.world_mut(), s, layout, false, visible)?;
        for requested in Lesson::ALL {
            let mut h = base.snapshot();
            let id = h.world().organism_entity_ids()[0].0;
            let (mut pending, mut left, mut fulfilled) = (None::<Event>, 0usize, false);
            let mut reported = false;
            for tick in 0..c.decisions {
                if arm != Arm::NoSpeech && tick < INSTRUCTION_TICKS && tick % 2 == 0 {
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
                let ready_prompted = h.world().neural_speech_cooldown_ready(id, true);
                let ready_other = h.world().neural_speech_cooldown_ready(id, false);
                let before = owners(h.world(), s);
                let legal = h.world().can_grab_target(id, requested.target(s));
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
                    let speech = if report_correction && reported {
                        None
                    } else if event_speech {
                        (tick < INSTRUCTION_TICKS)
                            .then(|| confirmed_speech(pending.as_ref(), left, Some(requested)))
                            .flatten()
                    } else if tick >= INSTRUCTION_TICKS {
                        narration(f, pending.as_ref(), left)
                    } else {
                        None
                    };
                    let prompted = f
                        .sensory()
                        .language_context
                        .heard_tokens
                        .iter()
                        .flatten()
                        .any(|t| t.source_kind == UtteranceSourceKind::Player);
                    let ready = if prompted {
                        ready_prompted
                    } else {
                        ready_other
                    };
                    let report_due = speech.is_some() && (!report_correction || ready);
                    if report_correction && speech.is_some() {
                        legal_reports += usize::from(ready);
                        cooldown_suppressed += usize::from(!ready);
                    }
                    let target = if report_correction && speech.is_some() && !ready {
                        let idle = f.candidates().iter().find(|c| c.kind == ActionKind::Idle);
                        let mut t = candidate_label(&row, idle)?;
                        t.representative = idle.map(|c| 1 << c.candidate_index);
                        t
                    } else if event_speech && report_due {
                        action_target(&row, s, requested, true, true, legal, held)?
                    } else if tick < INSTRUCTION_TICKS && fulfilled {
                        let idle = f.candidates().iter().find(|c| c.kind == ActionKind::Idle);
                        let mut t = candidate_label(&row, idle)?;
                        t.representative = idle.map(|c| 1 << c.candidate_index);
                        t
                    } else {
                        action_target(
                            &row,
                            s,
                            requested,
                            speech.is_some(),
                            tick < INSTRUCTION_TICKS,
                            legal,
                            held,
                        )?
                    };
                    let verb_mask = candidate_mask(
                        f,
                        &[
                            (HeadlessActionIds::EAT, s.food),
                            (HeadlessActionIds::GRAB, s.food),
                        ],
                    );
                    let noun_mask = candidate_mask(
                        f,
                        &[
                            (HeadlessActionIds::GRAB, s.food),
                            (HeadlessActionIds::GRAB, s.toy),
                        ],
                    );
                    let desired = if requested == Lesson::EatFood {
                        HeadlessActionIds::EAT
                    } else {
                        HeadlessActionIds::GRAB
                    };
                    let group = if report_correction && speech.is_some() && !ready {
                        Group::Fulfilled
                    } else if event_speech && report_due {
                        Group::Narration
                    } else if tick < INSTRUCTION_TICKS {
                        let concrete = candidate_mask(f, &[(desired, requested.target(s))]);
                        if target
                            .representative
                            .is_some_and(|m| m.count_ones() == 1 && m == concrete)
                        {
                            Group::Concrete(requested.index())
                        } else if fulfilled || held {
                            Group::Fulfilled
                        } else {
                            Group::Other
                        }
                    } else if speech.is_some() {
                        Group::Narration
                    } else {
                        Group::Other
                    };
                    let event = physical_event(&row, s, before, after)?;
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"balanced_readout_training","arm":arm,"round":round,"layout":layout,"world_seed":seed,"requested":requested,"instruction_window":tick<INSTRUCTION_TICKS,"scene":s,"group":group,"verb_choice_mask":verb_mask,"noun_choice_mask":noun_mask,"row":row,"owned_before":before,"owned_after":after,"label":{"representative":target.representative,"motors":target.motors},"speech_target":speech,"narration_evidence":pending,"completed":event,"report_correction":report_correction,"physical_ordering":physical_ordering,"fulfilled_before":fulfilled,"world_cooldown_ready":ready,"report_already_completed":reported,"vocal_report_due":report_due})
                    )?;
                    if speech.is_some() {
                        speech_rows += 1;
                        speech_with_heard += usize::from(
                            f.sensory()
                                .language_context
                                .heard_tokens
                                .iter()
                                .any(Option::is_some),
                        );
                        left -= 1;
                    }
                    if report_correction
                        && !reported
                        && pending.as_ref().is_some_and(|e| e.lesson == requested)
                    {
                        let declare = row.patch.selected_bundle().is_some_and(|b| {
                            b.channels.iter().any(|ch| {
                                ch.channel == MotorChannel::Vocal
                                    && ch.payload.values.first() == Some(&0x53485031)
                                    && ch.payload.values.get(1) == Some(&0)
                            })
                        });
                        let correct = declare
                            && h.world().audible_utterances().iter().any(|u| {
                                u.source_kind == UtteranceSourceKind::Creature
                                    && u.speaker_id == Some(id)
                                    && u.emitted_tick == f.tick()
                                    && u.tokens.iter().map(|t| t.raw()).collect::<Vec<_>>()
                                        == requested.words()
                            });
                        if correct {
                            reported = true;
                            actual_reports += 1;
                        }
                    }
                    if let Some(event) = event {
                        events[event.lesson.index()] += 1;
                        fulfilled |= event.lesson == requested;
                        pending = Some(event);
                        left = 3;
                    }
                    samples.push(Sample {
                        row,
                        target,
                        speech,
                        group,
                        verb_mask,
                        noun_mask,
                        report_correction,
                        physical_ordering,
                    });
                }
            }
        }
    }
    Ok((
        samples,
        json!({"round":round,"native_decisions":c.decisions*12,"teacher_heard_rows":heard,"actual_activity_counts":events,"train_seeds":(0..4).map(|l|c.seed+600_000+(round*4+l)as u64).collect::<Vec<_>>(),"fixed_cue_ticks":INSTRUCTION_TICKS,"positive_speech_rows":speech_rows,"positive_speech_with_heard_tokens":speech_with_heard,"legal_report_rows":legal_reports,"cooldown_suppressed_report_rows":cooldown_suppressed,"actual_correct_reports":actual_reports}),
    ))
}

fn distribution_counts(samples: &[Sample]) -> ([usize; 3], [usize; 3], [usize; 2], [usize; 2]) {
    let (mut concrete, mut rest, mut verbs, mut nouns) = ([0; 3], [0; 3], [0; 2], [0; 2]);
    for s in samples {
        match s.group {
            Group::Concrete(k) => {
                concrete[k] += 1;
                if k < 2 && s.verb_mask.count_ones() == 2 {
                    verbs[k] += 1;
                }
                if k > 0 && s.noun_mask.count_ones() == 2 {
                    nouns[k - 1] += 1;
                }
            }
            Group::Fulfilled => rest[0] += 1,
            Group::Narration => rest[1] += 1,
            Group::Other => rest[2] += 1,
        }
    }
    (concrete, rest, verbs, nouns)
}
fn add_objective(
    actor: &CpuFoundationTrainer,
    s: &Sample,
    logits: &[f32],
    mask: Option<u32>,
    scale: f32,
    adj: &mut [f32],
) -> Result<f64> {
    let conditional = mask.is_some();
    let ex = ImitationExample {
        candidate_count: s.row.action.candidate_count,
        representative_mask: mask.unwrap_or(s.row.action.representative_mask),
        motor_masks: if conditional {
            [0; 6]
        } else {
            s.row.action.motor_masks
        },
        forced_slots: s.row.action.forced_slots.clone(),
        target: if conditional {
            ImitationTarget {
                representative: s.target.representative,
                motors: [None; 6],
            }
        } else {
            s.target.clone()
        },
    };
    let objective = cpu_imitation_objective(logits, &ex, 1., 1.)?;
    for (dst, src) in adj.iter_mut().zip(objective.logit_gradients) {
        *dst += src * scale;
    }
    // Keep the actor argument explicit at this native-objective boundary.
    debug_assert_eq!(
        actor.phenotype().synapses().len(),
        s.row.replay_tick.effective_weight_offsets.len()
    );
    Ok(f64::from(objective.loss * scale))
}
fn wrong_noun_neighbor(target: ReplaySpeechTarget) -> Option<ReplaySpeechTarget> {
    if target.token != Some(9) || ![1, 2].contains(&target.continuation[0]) {
        return None;
    }
    let mut wrong = target;
    // Food=01 and Toy=10. The measured wrong noun is their OR-code 11.
    // It is a contrastive negative only, never a world event or a runtime remap.
    wrong.continuation[0] = 1 | 2;
    Some(wrong)
}
fn physical_margin(logits: &[f32], mask: u32, target: u32) -> Result<(f32, usize, usize)> {
    if target.count_ones() != 1 || target & mask != target {
        return Err("physical margin needs one legal target".into());
    }
    let desired = target.trailing_zeros() as usize;
    let competitor = (0..logits.len())
        .filter(|i| mask & (1 << i) != 0 && *i != desired)
        .max_by(|a, b| logits[*a].total_cmp(&logits[*b]))
        .ok_or("no competing representative")?;
    Ok((
        (0.5 + logits[competitor] - logits[desired]).max(0.),
        desired,
        competitor,
    ))
}
fn noun_margin_loss(true_loss: f64, wrong_loss: f64) -> f64 {
    (0.02 + true_loss - wrong_loss).max(0.)
}
fn gradient(actor: &CpuFoundationTrainer, samples: &[Sample]) -> Result<(Vec<f32>, Value)> {
    let (concrete, rest, verbs, nouns) = distribution_counts(samples);
    let correction = samples.first().is_some_and(|s| s.report_correction);
    assert!(samples.iter().all(|s| s.report_correction == correction));
    let (mut margin_loss, mut active_margin_rows) = (0f64, 0usize);
    let (mut physical_margin_loss, mut active_physical_rows, mut competing_voice_rows) =
        (0f64, 0usize, 0usize);
    if concrete.contains(&0) || verbs.contains(&0) || nouns.contains(&0) {
        return Err("missing real concrete classes; cannot claim balanced teaching".into());
    }
    let mut mass = [0f32; 3];
    for s in samples {
        if let Some(t) = s.speech {
            let k = Lesson::ALL
                .iter()
                .position(|l| l.words() == [t.token.unwrap(), t.continuation[0]])
                .ok_or("unknown factual speech")?;
            mass[k] += t.len() as f32;
        }
    }
    let classes = mass.iter().filter(|v| **v > 0.).count();
    let (mut sum, mut loss, mut speech_loss) = (vec![0.; actor.weights().len()], [0f64; 4], 0f64);
    for s in samples {
        let q = readout::single(actor, &s.row);
        let ev = actor.evaluate_replay(&q)?;
        let logits = &ev.candidate_logits[0];
        let mut adj = vec![0.; logits.len()];
        match s.group {
            Group::Concrete(k) => {
                loss[0] += add_objective(
                    actor,
                    s,
                    logits,
                    None,
                    0.4 / 3. / concrete[k] as f32,
                    &mut adj,
                )?;
                if s.physical_ordering {
                    let (hinge, desired, competitor) = physical_margin(
                        logits,
                        s.row.action.representative_mask,
                        s.target.representative.ok_or("missing concrete target")?,
                    )?;
                    if hinge > 0. {
                        let scale = 0.2 / 3. / concrete[k] as f32;
                        physical_margin_loss += f64::from(hinge * scale);
                        adj[desired] -= scale;
                        adj[competitor] += scale;
                        active_physical_rows += 1;
                        competing_voice_rows += usize::from(
                            s.row.patch.pre_action().perception().candidates()[competitor].kind
                                == ActionKind::Vocalize,
                        );
                    }
                }
                if k < 2 && s.verb_mask.count_ones() == 2 {
                    loss[1] += add_objective(
                        actor,
                        s,
                        logits,
                        Some(s.verb_mask),
                        0.15 / 2. / verbs[k] as f32,
                        &mut adj,
                    )?;
                }
                if k > 0 && s.noun_mask.count_ones() == 2 {
                    loss[2] += add_objective(
                        actor,
                        s,
                        logits,
                        Some(s.noun_mask),
                        0.15 / 2. / nouns[k - 1] as f32,
                        &mut adj,
                    )?;
                }
            }
            Group::Fulfilled => {
                loss[3] += add_objective(actor, s, logits, None, 0.1 / rest[0] as f32, &mut adj)?
            }
            Group::Narration => {
                loss[3] += add_objective(
                    actor,
                    s,
                    logits,
                    None,
                    (if correction { 0.4 } else { 0.1 }) / rest[1] as f32,
                    &mut adj,
                )?
            }
            Group::Other => {
                loss[3] += add_objective(actor, s, logits, None, 0.1 / rest[2] as f32, &mut adj)?
            }
        }
        let g = actor.probe_replay_gradients(&q, &[adj])?;
        for (dst, src) in sum.iter_mut().zip(g.gradients) {
            *dst += src;
        }
        if let Some(t) = s.speech {
            let k = Lesson::ALL
                .iter()
                .position(|l| l.words() == [t.token.unwrap(), t.continuation[0]])
                .ok_or("unknown actual speech")?;
            let scale = t.len() as f32 / mass[k] / classes as f32;
            let g = actor.probe_replay_gradients_with_speech(
                &q,
                &[vec![0.; logits.len()]],
                &[Some(t)],
            )?;
            speech_loss += g.objective * f64::from(scale);
            for (dst, src) in sum.iter_mut().zip(&g.gradients) {
                *dst += src * scale;
            }
            if correction {
                if let Some(wrong) = wrong_noun_neighbor(t) {
                    let other = actor.probe_replay_gradients_with_speech(
                        &q,
                        &[vec![0.; logits.len()]],
                        &[Some(wrong)],
                    )?;
                    let hinge = noun_margin_loss(g.objective, other.objective);
                    if hinge > 0. {
                        let noun_classes = mass[1..].iter().filter(|m| **m > 0.).count();
                        let pair_scale = t.len() as f32 / mass[k] / noun_classes as f32;
                        margin_loss += hinge * f64::from(pair_scale);
                        active_margin_rows += 1;
                        for ((dst, positive), negative) in
                            sum.iter_mut().zip(&g.gradients).zip(&other.gradients)
                        {
                            *dst += (positive - negative) * pair_scale;
                        }
                    }
                }
            }
        }
    }
    Ok((
        sum,
        json!({"concrete_counts_eat_food_get_food_get_toy":concrete,"full_concrete_total_mass_per_class":0.4/3.,"verb_pair_counts_eat_get":verbs,"verb_total_mass_per_class":0.075,"noun_pair_counts_food_toy":nouns,"noun_total_mass_per_class":0.075,"fulfilled_narration_other_rows":rest,"other_class_mass":0.1,"loss_full_verb_noun_other":loss,"factual_speech_loss":speech_loss,"report_action_mass":if correction{0.4}else{0.1},"physical_priority_margin_loss":physical_margin_loss,"active_physical_priority_rows":active_physical_rows,"top_competitor_vocal_rows":competing_voice_rows,"physical_priority_native_logit_margin":0.5,"physical_priority_total_mass":0.2,"noun_margin_loss":margin_loss,"active_noun_margin_rows":active_margin_rows,"noun_native_loss_gap_margin":0.02,"actual_payload_class_token_mass":mass,"production_marginals_not_claimed_balanced":true}),
    ))
}

#[derive(Default, Serialize)]
struct PhysicalMetrics {
    decisions: usize,
    food_acquisition_steps: Vec<usize>,
    toy_acquisition_steps: Vec<usize>,
    food_consumption_steps: Vec<usize>,
    nourished_meals: usize,
    first_acquisition: Option<String>,
    first_manipulation: Option<(u32, Option<WorldEntityId>)>,
    last_owners: [Option<OrganismId>; 2],
    utterance_tokens: Vec<Vec<u16>>,
    timely_factual_declarations: usize,
    eligible_speech_rows: usize,
    teacher_or_prior_present: bool,
}
pub(crate) fn evaluate(
    actor: &CpuFoundationTrainer,
    asset: &FoundationWeightAsset,
    c: LanguagePilotConfig,
    arm: Arm,
    plan: AssessmentPlan,
    traces: &mut impl Write,
) -> Result<Vec<Value>> {
    let mut results = vec![];
    let seeds = if plan.unseen_report_transfer_v1 {
        if plan.frozen_causal_v1 {
            frozen_causal::CASES.map(|c| c.seed)
        } else if plan.care_retention_v1 {
            unseen_transfer::CARE_SEEDS
        } else if plan.replay_early_stop_v1 {
            unseen_transfer::EARLY_STOP_SEEDS
        } else if plan.captured_failure_replay_v1 {
            unseen_transfer::REPLAY_SEEDS
        } else if plan.physical_before_speech_v1 {
            unseen_transfer::FRESH_SEEDS
        } else {
            unseen_transfer::SEEDS
        }
    } else {
        HELD_OUT_SEEDS
    };
    for (case, seed) in seeds.into_iter().enumerate() {
        let (mut base, s) = if plan.unseen_report_transfer_v1 {
            if plan.frozen_causal_v1 {
                unseen_transfer::prepared_frozen_case(actor, asset, c.seed, case)?
            } else if plan.care_retention_v1 {
                unseen_transfer::prepared_care_case(actor, asset, c.seed, case)?
            } else if plan.replay_early_stop_v1 {
                unseen_transfer::prepared_early_stop_case(actor, asset, c.seed, case)?
            } else if plan.captured_failure_replay_v1 {
                unseen_transfer::prepared_replay_case(actor, asset, c.seed, case)?
            } else if plan.physical_before_speech_v1 {
                unseen_transfer::prepared_fresh_case(actor, asset, c.seed, case)?
            } else {
                unseen_transfer::prepared_case(actor, asset, c.seed, case)?
            }
        } else {
            prepared(
                actor,
                asset,
                c.seed,
                seed,
                40 + case * 7,
                !case.is_multiple_of(2),
            )?
        };
        for _ in 0..2 {
            base.step(actor, 1.)?;
        }
        let id = base.world().organism_entity_ids()[0].0;
        for post_sleep in [false, true] {
            let mut parent = base.snapshot();
            let (mut sleeping, mut consolidations) = (0usize, 0usize);
            if post_sleep {
                parent.request_recovery_sleep(id)?;
                for _ in 0..600 {
                    let step = parent.step(actor, 1.)?;
                    sleeping += step.sleeping_organisms;
                    consolidations += step.sleep_consolidations;
                    writeln!(
                        traces,
                        "{}",
                        json!({"phase":"balanced_recovery_sleep","arm":arm,"case":case,"world_seed":seed,"sleeping":step.sleeping_organisms,"consolidations":step.sleep_consolidations,"decisions":step.decisions})
                    )?;
                    if consolidations > 0
                        && parent.world().organism_registry().get(id).is_some_and(|r| {
                            r.sleep_phase() == alife_core::sleep::SleepPhase::Awake
                        })
                    {
                        break;
                    }
                }
                if consolidations == 0
                    || parent
                        .world()
                        .organism_registry()
                        .get(id)
                        .is_none_or(|r| r.sleep_phase() != alife_core::sleep::SleepPhase::Awake)
                {
                    return Err("balanced assessment sleep failed to wake/consolidate".into());
                }
            }
            if plan.unseen_report_transfer_v1 {
                unseen_transfer::place_case(
                    parent.world_mut(),
                    s,
                    if plan.frozen_causal_v1 {
                        frozen_causal::CASES[case].fixture
                    } else {
                        case
                    },
                )?;
            } else {
                place_balanced(
                    parent.world_mut(),
                    s,
                    case,
                    true,
                    plan.balanced_grounding_v2,
                )?;
            }
            let paired = identity(&parent.checkpoint(actor)?)?;
            let mut causal_first = vec![];
            for requested in [
                Some(Lesson::EatFood),
                Some(Lesson::GetFood),
                Some(Lesson::GetToy),
                None,
            ] {
                let mut h = parent.snapshot();
                let (mut pending, mut left) = (None::<Event>, 0usize);
                let mut metric = PhysicalMetrics::default();
                for tick in 0..EVALUATION_TICKS {
                    let causal_before = if plan.frozen_causal_v1 && tick == 0 {
                        Some(frozen_causal::before_audio(&h, actor)?)
                    } else {
                        None
                    };
                    if tick < INSTRUCTION_TICKS && tick % 2 == 0 {
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
                    let causal_setup = causal_before
                        .map(|before| frozen_causal::after_audio(before, &h, actor))
                        .transpose()?;
                    let before = owners(h.world(), s);
                    let step = h.step(actor, 1.)?;
                    let after = owners(h.world(), s);
                    for row in step.decisions {
                        no_teacher_or_prior(&row)?;
                        if h.world().entity_id("nursery-teacher").is_some() {
                            return Err("teacher in balanced assessment".into());
                        }
                        let f = row.patch.pre_action().perception();
                        if tick == 0
                            && ![s.food, s.toy, s.distractor]
                                .into_iter()
                                .all(|t| visible_target(f, t))
                        {
                            return Err("balanced held-out referents/distractor not visible".into());
                        }
                        metric.decisions += 1;
                        let consumed = consequences(&row, s).0;
                        if before[0] != Some(id) && after[0] == Some(id) {
                            metric.food_acquisition_steps.push(tick);
                            metric.first_acquisition.get_or_insert("food".into());
                        }
                        if before[1] != Some(id) && after[1] == Some(id) {
                            metric.toy_acquisition_steps.push(tick);
                            metric.first_acquisition.get_or_insert("toy".into());
                        }
                        if consumed {
                            metric.food_consumption_steps.push(tick);
                            metric.nourished_meals +=
                                usize::from(row.patch.outcome().energy_delta.raw() > 0.);
                        }
                        if let Some(bundle) = row.patch.selected_bundle() {
                            for ch in &bundle.channels {
                                if ch.channel == MotorChannel::Manipulation {
                                    metric.first_manipulation.get_or_insert((
                                        ch.primitive.raw(),
                                        ch.target.and_then(|t| t.entity),
                                    ));
                                }
                            }
                        }
                        metric.last_owners = after;
                        let speech = if plan.event_confirmed_speech_v1 {
                            (tick < INSTRUCTION_TICKS)
                                .then(|| confirmed_speech(pending.as_ref(), left, requested))
                                .flatten()
                        } else if tick >= INSTRUCTION_TICKS {
                            narration(f, pending.as_ref(), left)
                        } else {
                            None
                        };
                        if speech.is_some() {
                            left -= 1;
                            metric.eligible_speech_rows += 1;
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
                        for u in &emitted {
                            let words = u.tokens.iter().map(|t| t.raw()).collect::<Vec<_>>();
                            metric.timely_factual_declarations += usize::from(
                                speech.is_some_and(|t| {
                                    words == vec![t.token.unwrap(), t.continuation[0]]
                                }) && row.patch.selected_bundle().is_some_and(|b| {
                                    b.channels.iter().any(|ch| {
                                        ch.channel == MotorChannel::Vocal
                                            && ch.payload.values.first() == Some(&0x53485031)
                                            && ch.payload.values.get(1) == Some(&0)
                                    })
                                }),
                            );
                            metric.utterance_tokens.push(words);
                        }
                        let event = physical_event(&row, s, before, after)?;
                        let trace_row = if plan.balanced_grounding_v2 && tick > 0 {
                            json!({"organism_id":row.organism_id,"sampling_counter":row.sampling_counter,"action":row.action,"patch":row.patch,"biological_reward":row.biological_reward,"brain_work":row.brain_work})
                        } else {
                            serde_json::to_value(&row)?
                        };
                        let mut record = json!({"phase":"balanced_readout_evaluation","arm":arm,"case":case,"world_seed":seed,"post_sleep":post_sleep,"requested":requested,"scene":s,"row":trace_row,"owned_before":before,"owned_after":after,"actual_food_consumption":consumed,"grounded_speech_target":speech,"narration_evidence":pending,"completed":event,"emitted":emitted});
                        if plan.frozen_causal_v1 {
                            record["causal_stratum"] = json!(frozen_causal::CASES[case].stratum);
                            record["fixture_case"] = json!(frozen_causal::CASES[case].fixture);
                            if tick == 0 {
                                record["causal_input_evidence"] = frozen_causal::input(
                                    &row,
                                    requested,
                                    causal_setup
                                        .clone()
                                        .ok_or("missing intervention setup proof")?,
                                )?;
                                causal_first.push((requested, row.clone()));
                            }
                        }
                        if plan.physical_before_speech_v1 {
                            let organism = h
                                .world()
                                .organism_registry()
                                .get(id)
                                .ok_or("missing assessed organism")?;
                            let bio = organism.biochemistry();
                            record["physical_safety_after_native_tick"] = json!({"alive":organism.lifecycle().is_alive(),"health":bio.body.health,"injury":bio.body.injury,"energy":bio.body.energy,"pain":bio.homeostasis.drives.pain});
                        }
                        if plan.balanced_grounding_v2 {
                            record["native_full_row_sha256"] = json!(identity(&row)?);
                            if plan.event_confirmed_speech_v1 {
                                record["speech_opportunity_protocol"] =
                                    json!("event-confirmed-cue-window-v1");
                            }
                            record["state_capture"] =
                                json!("full-first-row-and-full-state-digest-v1");
                        }
                        writeln!(traces, "{}", record)?;
                        if let Some(event) = event {
                            pending = Some(event);
                            left = 3;
                        }
                    }
                }
                results.push(json!({"arm":arm,"case":case,"world_seed":seed,"post_sleep":post_sleep,"independent_seed_unit":case,"sleep_is_paired_not_independent_sample":true,"sleeping_ticks":sleeping,"sleep_consolidations":consolidations,"requested":requested,"paired_initial_private_sha256":paired,"scene":s,"teacher_present":false,"prior_enabled":false,"cue_ticks":INSTRUCTION_TICKS,"metrics":metric}));
            }
            if plan.frozen_causal_v1 {
                let proof = frozen_causal::compare_inputs(actor, &causal_first)?;
                writeln!(
                    traces,
                    "{}",
                    json!({"phase":"frozen_causal_pair_inputs","case":case,"world_seed":seed,"post_sleep":post_sleep,"causal_stratum":frozen_causal::CASES[case].stratum,"proof":proof})
                )?;
            }
        }
    }
    Ok(results)
}

pub fn continue_balanced_grounded_language(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run_balanced(source, output, c, false, false, false, false)
}
pub fn continue_visible_balanced_grounded_language(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run_balanced(source, output, c, true, false, false, false)
}
pub fn continue_event_confirmed_speech(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run_balanced(source, output, c, true, true, false, false)
}
pub fn continue_report_choice(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run_balanced(source, output, c, true, true, true, false)
}
pub fn continue_physical_before_speech(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
) -> Result<Value> {
    run_balanced(source, output, c, true, true, true, true)
}
fn run_balanced(
    source: &Path,
    output: &Path,
    c: LanguagePilotConfig,
    visible: bool,
    event_speech: bool,
    report_correction: bool,
    physical_ordering: bool,
) -> Result<Value> {
    if c.llamacpp_translation
        || !c.experimental_word_bridge
        || !(1..=2).contains(&c.rounds)
        || !(1..=32).contains(&c.epochs)
        || c.decisions != 24
    {
        return Err(
            "balanced continuation requires scripted V3, rounds1..2 epochs1..32 decisions24".into(),
        );
    }
    let old: Value = serde_json::from_slice(&fs::read(source.join("results.json"))?)?;
    let admitted = if physical_ordering {
        "experimental-report-choice-noun-margin-v18"
    } else if report_correction {
        "experimental-event-confirmed-speech-v17"
    } else if event_speech {
        "experimental-balanced-grounding-visible-v16"
    } else {
        "experimental-linear-word-readout-v14"
    };
    if old["kind"] != admitted || old["config"]["seed"] != c.seed {
        return Err("continuation source must match the versioned measured pilot".into());
    }
    let recorded = old["arms"]
        .as_array()
        .ok_or("no V14 arms")?
        .iter()
        .find(|a| a["arm"] == "grounded")
        .ok_or("no source grounded actor")?;
    fs::create_dir(output)?;
    let start_time = Instant::now();
    let mut trace = zstd::stream::write::Encoder::new(
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join("native-trace.jsonl.zst"))?,
        3,
    )?;
    let mut arms = vec![];
    for arm in [Arm::Untreated, Arm::Grounded, Arm::Shuffled, Arm::NoSpeech] {
        let r = read_recovery(&source.join("recovery-Grounded"))?;
        if r.inputs.experimental_language_input()
            != Some(ExperimentalLanguageInputV1::LinearWordReadoutV3)
        {
            return Err("not V3 readout founder".into());
        }
        let p = PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048())?;
        let mut asset = FoundationWeightAsset::decode_canonical(&r.foundation)?;
        let mut actor = CpuFoundationTrainer::from_checkpoint(p, asset.clone(), r.actor)?;
        let initial = identity(&actor.checkpoint())?;
        if recorded["actor_sha256"] != initial {
            return Err("source differs from measured V14".into());
        }
        let original_private =
            CpuCanonicalHost::from_checkpoint(r.host, &actor)?.checkpoint(&actor)?;
        let private_sha = identity(&original_private)?;
        let before = actor.checkpoint();
        let mut collections = vec![];
        let mut updates = vec![];
        let mut next = r.next_gradient;
        if arm != Arm::Untreated {
            for round in 0..c.rounds {
                eprintln!("balanced {arm:?} round{round} native collection");
                let (samples, receipt) = collect(
                    &actor,
                    &asset,
                    c,
                    arm,
                    round,
                    &mut trace,
                    TeachingProtocol {
                        visible,
                        event_speech,
                        report_correction,
                        physical_ordering,
                    },
                )?;
                eprintln!("balanced collection {receipt}");
                collections.push(receipt);
                for epoch in 0..c.epochs {
                    let (g, objective) = gradient(&actor, &samples)?;
                    let frozen = actor.checkpoint();
                    let norm = actor.apply_gradients(&g)?;
                    preserve_frozen(&frozen, &actor.checkpoint())?;
                    updates.push(json!({"round":round,"epoch":epoch,"step":actor.optimizer_step(),"objective":objective,"gradient_norm":norm,"all_frozen_coordinates_exact":true}));
                    next = g;
                    if epoch % 8 == 7 {
                        eprintln!(
                            "balanced {arm:?} round{round} epoch{} step{}",
                            epoch + 1,
                            actor.optimizer_step()
                        );
                    }
                }
                asset = export(&mut actor, c.seed)?;
            }
        }
        let plan = AssessmentPlan {
            recovery_sleep: true,
            collect_utterance_timing: true,
            factorial_grounding: true,
            paired_request_focus: true,
            linear_readout: true,
            balanced_grounding_v1: !visible,
            balanced_grounding_v2: visible,
            event_confirmed_speech_v1: event_speech,
            unseen_report_transfer_v1: physical_ordering,
            physical_before_speech_v1: physical_ordering,
            captured_failure_replay_v1: false,
            replay_early_stop_v1: false,
            care_retention_v1: false,
            frozen_causal_v1: false,
        };
        eprintln!("balanced {arm:?} cold independent-seed assessment");
        let cold = exact_reload_plan(
            &output.join(format!("recovery-{arm:?}")),
            &actor,
            &asset,
            RecoveredAssessment {
                config: c,
                arm,
                curriculum: Some(plan),
            },
            next,
        )?;
        if arm == Arm::Untreated && identity(&actor.checkpoint())? != initial {
            return Err("untouched actor changed before assessment".into());
        }
        arms.push(json!({"arm":arm,"common_source_actor_sha256":initial,"source_private_sha256":private_sha,"source_step":before.optimizer_step,"actor_step":actor.optimizer_step(),"actor_sha256":identity(&actor.checkpoint())?,"collections":collections,"updates":updates,"evaluation":cold["native_private_and_optimizer_hashes"]["assessment"]["evaluation"],"continuity":cold,"source_history_retained":true}));
    }
    trace.finish()?;
    let result = json!({"kind":if physical_ordering{"experimental-physical-before-speech-v20"}else if report_correction{"experimental-report-choice-noun-margin-v18"}else if event_speech{"experimental-event-confirmed-speech-v17"}else if visible{"experimental-balanced-grounding-visible-v16"}else{"experimental-balanced-grounding-v15"},"execution_domain":"native-cpu","config":c,"source_report_sha256":identity(&old)?,"held_out_world_seeds":if physical_ordering{unseen_transfer::FRESH_SEEDS}else{HELD_OUT_SEEDS},"independent_seed_count":HELD_OUT_SEEDS.len(),"pre_post_are_paired_not_independent":true,"teacher_and_player_cue_ticks":INSTRUCTION_TICKS,"evaluation_ticks":EVALUATION_TICKS,"arms":arms,"elapsed_seconds":start_time.elapsed().as_secs_f64(),"semantic_prior_enabled":false,"teacher":"scripted ordinary spatial language","limitations":["One common retained source founder; independent held-out environment/sampling seeds are not independent trained founders.","Post-sleep is paired with the same pre-sleep seed and must not double independent N. RecoveryProtocol sleep is induced, with no matched awake-delay causal sleep control.","Native conditional action losses balance verb and noun choices separately; truthful narration activity classes cannot yield exactly balanced lexical marginals without impossible EatToy events.","No-request retains prior grounded history and natural self speech. No real SLM delivery or promotion."]});
    write_json(&output.join("results.json"), &result)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn eat_control(
        h: &mut CpuCanonicalHost,
        actor: &CpuFoundationTrainer,
        s: Scene,
    ) -> Option<(bool, f32)> {
        let w = h.world_mut();
        let (id, body) = w.organism_entity_ids()[0];
        let before = *w.organism_registry().get(id).unwrap().biochemistry();
        let frame = w
            .perception_frame(
                id,
                w.tick(),
                actor.phenotype().sensor_profile(),
                before.homeostasis,
            )
            .unwrap();
        let candidate = frame
            .candidates()
            .iter()
            .find(|c| c.action_id == HeadlessActionIds::EAT && c.target.entity == Some(s.food))?;
        let command = candidate
            .to_command(id, Confidence::new(1.).unwrap())
            .unwrap();
        let receipt = w
            .apply_registered_command(&command, body, Tick(w.tick().raw() + 1))
            .unwrap();
        let consumed =
            receipt.action_result.execution.physical.contact == PhysicalContactKind::Consumed;
        let gain = receipt.biology_after.body.energy - before.body.energy;
        w.try_advance_tick().unwrap();
        Some((consumed, gain))
    }
    #[test]
    fn v15_near_toy_occludes_carried_food_and_moving_toy_restores_eat() {
        let founder = 847501;
        let asset = initial_n2048_care_asset(founder).unwrap();
        let actor = new_actor_profile(founder, &asset, true).unwrap();
        let (mut h, s) = prepared(&actor, &asset, founder, HELD_OUT_SEEDS[0], 40, false).unwrap();
        place_balanced(h.world_mut(), s, 0, false, false).unwrap();
        assert!(!eat_control(&mut h, &actor, s).unwrap().0);
        let id = h.world().organism_entity_ids()[0].0;
        assert_eq!(h.world().entity(s.food).unwrap().carried_by, Some(id));
        assert!(eat_control(&mut h, &actor, s).is_none());
        let b = h
            .world()
            .entity(h.world().organism_entity_ids()[0].1)
            .unwrap()
            .clone();
        h.world_mut()
            .editor_move_object(
                s.toy,
                Vec3f::new(b.position.x + 4., b.position.y, b.position.z + 4.),
            )
            .unwrap();
        let (consumed, gain) = eat_control(&mut h, &actor, s).unwrap();
        assert!(consumed && gain > 0.);
    }
    #[test]
    fn v16_all_layouts_really_consume_and_nourish_without_toy_occlusion() {
        let founder = 847501;
        let asset = initial_n2048_care_asset(founder).unwrap();
        let actor = new_actor_profile(founder, &asset, true).unwrap();
        for (held_out, count) in [(false, 4), (true, 6)] {
            for layout in 0..count {
                let seed = if held_out {
                    *HELD_OUT_SEEDS.get(layout).unwrap()
                } else {
                    1_447_501 + layout as u64
                };
                let (mut h, s) = prepared(
                    &actor,
                    &asset,
                    founder,
                    seed,
                    40 + layout * 7,
                    layout % 2 == 1,
                )
                .unwrap();
                place_balanced(h.world_mut(), s, layout, held_out, true).unwrap();
                assert!(
                    !eat_control(&mut h, &actor, s)
                        .unwrap_or_else(|| panic!(
                            "first Eat absent: held_out={held_out} layout={layout}"
                        ))
                        .0
                );
                let (consumed, gain) = eat_control(&mut h, &actor, s).unwrap_or_else(|| {
                    panic!("held Eat absent: held_out={held_out} layout={layout}")
                });
                assert!(
                    consumed && gain > 0.,
                    "failed nutrition: held_out={held_out} layout={layout}"
                );
            }
        }
    }
    #[test]
    fn physical_priority_margin_native_gradient_matches_real_row_finite_difference() {
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor_profile(seed, &asset, true).unwrap();
        let (mut h, scene) = prepared(&actor, &asset, seed, 1447501, 2, false).unwrap();
        place_balanced(h.world_mut(), scene, 0, false, true).unwrap();
        let row = h.step(&actor, 1.).unwrap().decisions.remove(0);
        let frame = row.patch.pre_action().perception();
        let target = candidate_mask(frame, &[(HeadlessActionIds::GRAB, scene.food)]);
        let q = readout::single(&actor, &row);
        let ev = actor.evaluate_replay(&q).unwrap();
        let (loss, desired, competitor) = physical_margin(
            &ev.candidate_logits[0],
            row.action.representative_mask,
            target,
        )
        .unwrap();
        assert!(loss > 0.);
        let mut adj = vec![0.; ev.candidate_logits[0].len()];
        adj[desired] = -1.;
        adj[competitor] = 1.;
        let g = actor.probe_replay_gradients(&q, &[adj]).unwrap();
        let index = actor.phenotype().synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()!=DecoderHeadKind::SpeechPayload)).max_by(|(a,_),(b,_)|g.gradients[*a].abs().total_cmp(&g.gradients[*b].abs())).unwrap().0;
        assert!(g.gradients[index].abs() > 1e-5);
        let epsilon = 0.001;
        let mut losses = vec![];
        for delta in [-epsilon, epsilon] {
            let mut cp = actor.checkpoint();
            cp.weights[index] += delta;
            let changed =
                CpuFoundationTrainer::from_checkpoint(actor.phenotype().clone(), asset.clone(), cp)
                    .unwrap();
            let ev = changed.evaluate_replay(&q).unwrap();
            losses.push(
                physical_margin(
                    &ev.candidate_logits[0],
                    row.action.representative_mask,
                    target,
                )
                .unwrap()
                .0,
            );
        }
        let numerical = (losses[1] - losses[0]) / (2. * epsilon);
        assert!(
            (numerical - g.gradients[index]).abs() < 0.003,
            "native {} numerical {numerical}",
            g.gradients[index]
        );
    }
    #[test]
    fn native_noun_margin_gradient_matches_finite_difference_on_real_row() {
        // Numeric objective check on a real native row, not a learning/behavior score.
        let seed = 847501;
        let asset = initial_n2048_care_asset(seed).unwrap();
        let actor = new_actor_profile(seed, &asset, true).unwrap();
        let (mut h, scene) = prepared(&actor, &asset, seed, 1447501, 2, false).unwrap();
        place_balanced(h.world_mut(), scene, 0, false, true).unwrap();
        let id = h.world().organism_entity_ids()[0].0;
        let pos = speech_position(h.world()).unwrap();
        h.world_mut()
            .emit_player_tokens(
                Some(id),
                pos,
                [9, 2]
                    .into_iter()
                    .map(|w| LanguageTokenId::new(w).unwrap())
                    .collect(),
            )
            .unwrap();
        let row = h.step(&actor, 1.).unwrap().decisions.remove(0);
        let q = readout::single(&actor, &row);
        let n = row.action.candidate_count as usize;
        let mut continuation = [0; 5];
        continuation[0] = 2;
        let target = ReplaySpeechTarget {
            token: Some(9),
            continuation,
            act: SpeechActKind::Declare,
            weight: 1.,
        };
        let wrong = wrong_noun_neighbor(target).unwrap();
        assert_eq!(wrong.continuation[0], 3);
        let evaluate = |a: &CpuFoundationTrainer| {
            let g = a
                .probe_replay_gradients_with_speech(&q, &[vec![0.; n]], &[Some(target)])
                .unwrap();
            let b = a
                .probe_replay_gradients_with_speech(&q, &[vec![0.; n]], &[Some(wrong)])
                .unwrap();
            (
                0.02 + g.objective - b.objective,
                g.gradients
                    .iter()
                    .zip(b.gradients)
                    .map(|(a, b)| a - b)
                    .collect::<Vec<_>>(),
            )
        };
        // Check the native derivative of the active hinge branch even when
        // this fresh actor already satisfies the margin; no behavior claim.
        assert_eq!(noun_margin_loss(0., 1.), 0.);
        assert!((noun_margin_loss(1., 0.) - 1.02).abs() < 1e-12);
        let (_, g) = evaluate(&actor);
        let index=actor.phenotype().synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload)).max_by(|(a,_),(b,_)|g[*a].abs().total_cmp(&g[*b].abs())).unwrap().0;
        assert!(g[index].abs() > 1e-5);
        let epsilon = 0.001;
        let mut losses = vec![];
        for delta in [-epsilon, epsilon] {
            let mut cp = actor.checkpoint();
            cp.weights[index] += delta;
            let changed =
                CpuFoundationTrainer::from_checkpoint(actor.phenotype().clone(), asset.clone(), cp)
                    .unwrap();
            losses.push(evaluate(&changed).0);
        }
        let numerical = (losses[1] - losses[0]) / (2. * f64::from(epsilon));
        assert!(
            (numerical - f64::from(g[index])).abs() < 0.003,
            "native margin analytic {} numerical {numerical}",
            g[index]
        );
    }
    #[test]
    fn confirmed_speech_needs_actual_matching_event_and_constant_bits_prefer_get_food() {
        let event = Event {
            lesson: Lesson::GetToy,
            tick: 4,
            patch_sha256: "actual-world-receipt".into(),
        };
        assert!(confirmed_speech(None, 3, Some(Lesson::GetToy)).is_none());
        assert!(confirmed_speech(Some(&event), 3, Some(Lesson::GetFood)).is_none());
        assert!(confirmed_speech(Some(&event), 0, Some(Lesson::GetToy)).is_none());
        assert!(confirmed_speech(Some(&event), 3, None).is_none());
        assert_eq!(
            confirmed_speech(Some(&event), 3, Some(Lesson::GetToy))
                .unwrap()
                .continuation[0],
            2
        );
        // Diagnostic of the native bit-regression constant optimum, not behavior.
        let targets = Lesson::ALL.map(|lesson| {
            let e = Event {
                lesson,
                ..event.clone()
            };
            confirmed_speech(Some(&e), 1, Some(lesson)).unwrap()
        });
        let words = [0, 1].map(|step| {
            (0..8).fold(0u32, |word, bit| {
                let mean = targets
                    .iter()
                    .map(|t| t.step_logits(step)[8 + bit])
                    .sum::<f32>()
                    / 3.;
                word | (u32::from(mean > 0.) << bit)
            })
        });
        assert_eq!(words, [9, 1]);
    }
    #[test]
    #[ignore = "explicit saved public-row speech timing/gradient diagnosis; no provider request"]
    fn diagnose_actual_positive_speech_input_and_native_gradients() {
        let data: Value = serde_json::from_slice(
            &fs::read("/tmp/language-speech-timing-diagnostic.json").unwrap(),
        )
        .unwrap();
        let r = read_recovery(Path::new(
            "/workspace/language-balanced-visible-v16-run/recovery-Grounded",
        ))
        .unwrap();
        let p =
            PhenotypeCompiler::compile_validated(&r.inputs, &BrainCapacityClass::n2048()).unwrap();
        let asset = FoundationWeightAsset::decode_canonical(&r.foundation).unwrap();
        let actor = CpuFoundationTrainer::from_checkpoint(p, asset, r.actor).unwrap();
        let start = actor.phenotype().candidate_decoder().motor_start() as usize + 128;
        let speech_ids = actor.phenotype().synapses().iter().enumerate().filter_map(|(i,s)| matches!(s.kind(), CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::SpeechPayload).then_some(i)).collect::<Vec<_>>();
        let mut receipts = vec![];
        for key in ["8_1", "9_1", "9_2"] {
            let example = &data["examples"][key];
            let row: CpuCollectedDecision = serde_json::from_value(example["row"].clone()).unwrap();
            let target: ReplaySpeechTarget =
                serde_json::from_value(example["speech_target"].clone()).unwrap();
            let language = row
                .patch
                .pre_action()
                .perception()
                .sensory()
                .language_prior_neural_lanes();
            assert!(language[..16].iter().all(|v| *v == 0.));
            let q = readout::single(&actor, &row);
            let ev = actor.evaluate_replay(&q).unwrap();
            let zeros = vec![0.; ev.candidate_logits[0].len()];
            let g = actor
                .probe_replay_gradients_with_speech(
                    &q,
                    std::slice::from_ref(&zeros),
                    &[Some(target)],
                )
                .unwrap();
            let mut intervention = q.clone();
            intervention.ticks[0].encoded_inputs[start + 8] += 0.25;
            let changed = actor
                .probe_replay_gradients_with_speech(&intervention, &[zeros], &[Some(target)])
                .unwrap();
            let delta = speech_ids
                .iter()
                .map(|i| f64::from(g.gradients[*i] - changed.gradients[*i]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(delta > 0.);
            let decoded = alife_training::cpu::decode_cpu_speech(
                &actor,
                &ev.final_activations[0],
                &q.ticks[0].effective_weight_offsets,
            )
            .unwrap();
            receipts.push(json!({"target_words":[target.token.unwrap(),target.continuation[0]],"actual_heard_word_lane_l1":language[..16].iter().map(|v|v.abs()).sum::<f32>(),"native_speech_gradient_nonzero":speech_ids.iter().filter(|i|g.gradients[**i]!=0.).count(),"input_only_speech_noun_lane_pulse_gradient_delta_l2":delta,"decoded_if_vocal_diagnostic_only":decoded,"not_a_scored_world_or_learning_claim":true}));
        }
        write_json(
            Path::new("/tmp/language-speech-native-timing-proof.json"),
            &json!({"actual_training_counts":data["counts"],"receipts":receipts}),
        )
        .unwrap();
    }
    #[test]
    fn renamed_instances_and_balanced_native_masks_preserve_real_labels() {
        let seed = 847501;
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
        let (samples, _) = collect(
            &actor,
            &asset,
            config,
            Arm::Grounded,
            0,
            &mut std::io::sink(),
            TeachingProtocol {
                visible: false,
                event_speech: false,
                report_correction: false,
                physical_ordering: false,
            },
        )
        .unwrap();
        let (concrete, _, verb, noun) = distribution_counts(&samples);
        assert!(concrete.iter().all(|x| *x >= 4));
        assert!(verb.iter().all(|x| *x >= 4));
        assert!(noun.iter().all(|x| *x >= 4));
        let (g, receipt) = gradient(&actor, &samples).unwrap();
        assert!(g.iter().all(|x| x.is_finite()));
        assert_eq!(receipt["verb_total_mass_per_class"], 0.075);
        assert_eq!(receipt["noun_total_mass_per_class"], 0.075);
        let id=actor.phenotype().synapses().iter().enumerate().filter(|(_,s)|matches!(s.kind(),CompiledSynapseKind::Decoder(d)if d.head()==DecoderHeadKind::ActionCandidate)).max_by(|(i,_),(j,_)|g[*i].abs().total_cmp(&g[*j].abs())).unwrap().0;
        let epsilon = 0.001;
        let mut values = vec![];
        for d in [-epsilon, epsilon] {
            let mut cp = actor.checkpoint();
            cp.weights[id] += d;
            let changed =
                CpuFoundationTrainer::from_checkpoint(actor.phenotype().clone(), asset.clone(), cp)
                    .unwrap();
            let (_, v) = gradient(&changed, &samples).unwrap();
            values.push(
                v["loss_full_verb_noun_other"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .sum::<f64>()
                    + v["factual_speech_loss"].as_f64().unwrap(),
            );
        }
        let numeric = (values[1] - values[0]) / (2. * f64::from(epsilon));
        assert!(
            (numeric - f64::from(g[id])).abs() < 0.003,
            "native balanced analytic {} versus numeric {numeric}",
            g[id]
        );
        let source = actor.checkpoint();
        assert_eq!(actor.optimizer_step(), source.optimizer_step);
        let mut all_ids = std::collections::BTreeSet::new();
        for (case, world_seed) in HELD_OUT_SEEDS.into_iter().enumerate() {
            let (h, s) = prepared(
                &actor,
                &asset,
                seed,
                world_seed,
                40 + case * 7,
                !case.is_multiple_of(2),
            )
            .unwrap();
            assert!(all_ids.insert(s.food.raw()));
            assert!(all_ids.insert(s.toy.raw()));
            assert_ne!(s.food, WorldEntityId(8));
            assert_ne!(s.toy, WorldEntityId(9));
            assert!(h.world().entity(s.food).is_some());
        }
    }
}
