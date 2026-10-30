//! Synthetic CPU selection over actual world/biology receipts; no GPU, accepted
//! learner, PPO, private neural restore or acquired competence is exercised.
use super::*;
use crate::foundation_training::held_food_lesson_tests as fixture;
use alife_core::{
    Confidence, ExperienceSequenceId, HomeostaticDelta, MeasuredPhysiologyTransition,
    NormalizedScalar, PostActionOutcome, SignedValence,
};
use alife_world::HeadlessActionIds;

fn source() -> (HeadlessWorld, CareEvaluationSetup) {
    let mut world = fixture::unconfigured(42);
    for _ in 0..8 {
        world.try_advance_tick().unwrap();
    }
    let organism = world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let target = world.entity_id("food-01").unwrap();
    let body = position(&world, organism).unwrap();
    let setup = CareEvaluationSetup {
        organism,
        target,
        held_out_food_position: Vec3f::new(body.x + 2.0, body.y, body.z),
        world_ticks: 50,
        start_boundary: CareEvaluationStartBoundary::ExactPrivatePolicyAndSampler,
        food_history: CareEvaluationFoodHistory::KnownUnassistedAndNeverCarried,
    };
    (world, setup)
}

fn forks(world: &HeadlessWorld, setup: CareEvaluationSetup) -> [CareEvaluationFork; 3] {
    let mut result = prepare_care_evaluation_forks(
        world,
        b"synthetic-private-fast-lifetime-memory",
        b"seed=91;counter=777;temperature=1",
        setup,
    )
    .unwrap();
    for arm in &mut result {
        arm.control.begin_scene(&mut arm.world).unwrap();
    }
    result
}

fn act(
    world: &mut HeadlessWorld,
    action: alife_core::ActionId,
    target: Option<WorldEntityId>,
) -> ExperiencePatch {
    let frame = fixture::frame(world);
    let organism = frame.organism_id();
    let command = frame
        .candidates()
        .iter()
        .find(|c| c.action_id == action && c.target.entity == target)
        .unwrap()
        .to_command(organism, Confidence::new(1.0).unwrap())
        .unwrap();
    let body = world
        .organism_registry()
        .get(organism)
        .unwrap()
        .world_entity_id();
    let result = world
        .apply_registered_command(&command, body, Tick(world.tick().raw() + 1))
        .unwrap();
    world.try_advance_tick().unwrap();
    let outcome = PostActionOutcome::new(
        organism,
        ExperienceSequenceId(1),
        world.tick(),
        result.action_result.execution.succeeded,
        result.action_result.execution.physical,
        HomeostaticDelta::zero(),
        SignedValence::ZERO,
        NormalizedScalar::ZERO,
        NormalizedScalar::ZERO,
        SignedValence::ZERO,
        NormalizedScalar::ZERO,
    )
    .unwrap()
    .with_measured_physiology(
        MeasuredPhysiologyTransition::new(result.biology_before, result.biology_after).unwrap(),
    )
    .unwrap();
    fixture::action_patch(frame, target, outcome, action, None)
}

#[test]
fn matched_forks_preserve_source_private_sampler_and_biology_on_success_and_failure() {
    let (world, setup) = source();
    let source_digest = world.canonical_signature_digest().unwrap();
    let source_biology = biology(&world, setup.organism);
    let private = b"synthetic-private-fast-lifetime-memory".to_vec();
    let sampler = b"seed=91;counter=777;temperature=1".to_vec();
    let mut arms = prepare_care_evaluation_forks(&world, &private, &sampler, setup).unwrap();
    for arm in &arms {
        assert_eq!(arm.private_checkpoint_bytes, private);
        assert_eq!(arm.sampler_checkpoint_bytes, sampler);
        assert_eq!(arm.world.tick(), world.tick());
        assert_eq!(biology(&arm.world, setup.organism), source_biology);
        assert_eq!(
            arm.world.canonical_signature_digest().unwrap(),
            source_digest
        );
    }
    for arm in &mut arms {
        arm.control.begin_scene(&mut arm.world).unwrap();
    }
    for arm in &arms {
        assert_eq!(
            arm.world.entity(setup.target).unwrap().position,
            setup.held_out_food_position
        );
        assert_eq!(
            arm.world.canonical_signature_digest().unwrap(),
            arms[0].world.canonical_signature_digest().unwrap()
        );
    }
    arms[0].private_checkpoint_bytes[0] ^= 1;
    arms[0].sampler_checkpoint_bytes[0] ^= 1;
    arms[0].world.try_advance_tick().unwrap();
    assert_eq!(arms[1].private_checkpoint_bytes, private);
    assert_eq!(arms[2].sampler_checkpoint_bytes, sampler);
    assert_eq!(arms[1].world.tick(), world.tick());
    assert_eq!(world.canonical_signature_digest().unwrap(), source_digest);
    let invalid = CareEvaluationSetup {
        held_out_food_position: Vec3f::new(f32::NAN, 0.0, 0.0),
        ..setup
    };
    assert!(prepare_care_evaluation_forks(&world, &private, &sampler, invalid).is_err());
    assert_eq!(world.canonical_signature_digest().unwrap(), source_digest);
    assert_eq!(private, b"synthetic-private-fast-lifetime-memory");
    assert_eq!(sampler, b"seed=91;counter=777;temperature=1");
}

#[test]
fn refuses_legacy_alignment_unknown_or_assisted_food_and_missing_sampler_without_reset() {
    let (world, setup) = source();
    let before = world.canonical_signature_digest().unwrap();
    for invalid in [
        CareEvaluationSetup {
            start_boundary: CareEvaluationStartBoundary::RequiresLegacySleepOrPolicyAdoption,
            ..setup
        },
        CareEvaluationSetup {
            food_history: CareEvaluationFoodHistory::Unknown,
            ..setup
        },
        CareEvaluationSetup {
            food_history: CareEvaluationFoodHistory::AssistedOrPreviouslyCarried,
            ..setup
        },
        CareEvaluationSetup {
            world_ticks: 0,
            ..setup
        },
        CareEvaluationSetup {
            world_ticks: 36_001,
            ..setup
        },
    ] {
        assert!(prepare_care_evaluation_forks(&world, b"private", b"sampler", invalid).is_err());
    }
    assert!(prepare_care_evaluation_forks(&world, b"private", b"", setup).is_err());
    assert!(prepare_care_evaluation_forks(&world, b"", b"sampler", setup).is_err());
    assert_eq!(world.canonical_signature_digest().unwrap(), before);
}

#[test]
fn refuses_lingering_speech_teacher_player_hold_and_already_carried_food() {
    let (world, setup) = source();
    let mut speech = world.clone();
    speech
        .emit_player_tokens(
            Some(setup.organism),
            Vec3f::ZERO,
            vec![alife_core::LanguageTokenId::new(1).unwrap()],
        )
        .unwrap();
    let speech_before = speech.audible_utterances();
    assert!(prepare_care_evaluation_forks(&speech, b"p", b"s", setup).is_err());
    assert_eq!(speech.audible_utterances(), speech_before);
    let mut teacher = world.clone();
    teacher
        .spawn_social_agent("nursery-teacher", OrganismId(9999), Vec3f::ZERO, 0.0)
        .unwrap();
    assert!(prepare_care_evaluation_forks(&teacher, b"p", b"s", setup).is_err());
    let mut held = world.clone();
    held.begin_player_hold(setup.target).unwrap();
    assert!(prepare_care_evaluation_forks(&held, b"p", b"s", setup).is_err());
    assert_eq!(held.player_held_object(), Some(setup.target));
    let mut carried = world.clone();
    let body = position(&carried, setup.organism).unwrap();
    carried.editor_move_object(setup.target, body).unwrap();
    act(&mut carried, HeadlessActionIds::EAT, Some(setup.target));
    assert_eq!(
        carried.entity(setup.target).unwrap().carried_by,
        Some(setup.organism)
    );
    assert!(prepare_care_evaluation_forks(&carried, b"p", b"s", setup).is_err());
    assert_eq!(
        carried.entity(setup.target).unwrap().carried_by,
        Some(setup.organism)
    );
}

#[test]
fn all_three_arms_gate_initial_and_periodic_interventions_without_tick_or_body_reset() {
    let (world, setup) = source();
    let [mut none, mut cue, mut placed] = forks(&world, setup);
    for arm in [&mut none, &mut cue, &mut placed] {
        let initial = biology(&arm.world, setup.organism);
        let tick = arm.world.tick();
        assert!(arm.control.before_tick(&mut arm.world).unwrap());
        assert_eq!(arm.world.tick(), tick);
        assert_eq!(biology(&arm.world, setup.organism), initial);
        let once = arm.world.canonical_signature_digest().unwrap();
        arm.control.before_tick(&mut arm.world).unwrap();
        assert_eq!(arm.world.canonical_signature_digest().unwrap(), once);
    }
    assert!(none.world.audible_utterances().is_empty());
    assert_eq!(
        none.world.entity(setup.target).unwrap().position,
        setup.held_out_food_position
    );
    assert_eq!(
        cue.world.entity(setup.target).unwrap().position,
        setup.held_out_food_position
    );
    assert!(cue.control.summary.cue_emissions > 0);
    assert_eq!(cue.control.summary.food_repositions, 0);
    assert!(placed.control.summary.food_repositions > 0);
    assert!(placed.control.summary.cue_emissions > 0);
    for _ in 0..24 {
        for arm in [&mut none, &mut cue, &mut placed] {
            arm.world.try_advance_tick().unwrap();
            arm.control
                .observe_committed_tick(&arm.world, None)
                .unwrap();
        }
    }
    let far = Vec3f::new(
        setup.held_out_food_position.x + 3.0,
        setup.held_out_food_position.y,
        setup.held_out_food_position.z,
    );
    for arm in [&mut none, &mut cue, &mut placed] {
        arm.world.editor_move_object(setup.target, far).unwrap();
    }
    for arm in [&mut none, &mut cue, &mut placed] {
        arm.control.before_tick(&mut arm.world).unwrap();
    }
    assert!(none.world.audible_utterances().is_empty());
    assert_eq!(none.control.summary.cue_emissions, 0);
    assert_eq!(none.control.summary.food_repositions, 0);
    assert_eq!(cue.control.summary.food_repositions, 0);
    assert_eq!(cue.world.entity(setup.target).unwrap().position, far);
    assert_eq!(cue.control.summary.cue_emissions, 2);
    assert_eq!(placed.control.summary.food_repositions, 2);
    assert_ne!(
        biology(&none.world, setup.organism),
        biology(&world, setup.organism)
    );
}

#[test]
fn no_help_does_not_emit_recovery_speech_for_an_overlapping_teacher() {
    let (world, setup) = source();
    let [mut none, _, _] = forks(&world, setup);
    let body = position(&none.world, setup.organism).unwrap();
    // Fault injected after clean admission to exercise the control gate itself.
    none.world
        .spawn_social_agent("nursery-teacher", OrganismId(9999), body, 0.0)
        .unwrap();
    let before = none.world.canonical_signature_digest().unwrap();
    none.control.before_tick(&mut none.world).unwrap();
    assert_eq!(none.world.canonical_signature_digest().unwrap(), before);
    assert!(none.world.audible_utterances().is_empty());
}

#[test]
fn exact_target_acquisition_and_nourished_meal_use_sealed_normal_biology_receipts() {
    let (world, mut setup) = source();
    setup.held_out_food_position = position(&world, setup.organism).unwrap();
    let [mut arm, _, _] = forks(&world, setup);
    let patch = act(&mut arm.world, HeadlessActionIds::EAT, Some(setup.target));
    arm.control
        .observe_committed_tick(&arm.world, Some(&patch))
        .unwrap();
    assert_eq!(arm.control.summary.first_acquisition_latency_ticks, Some(1));
    assert_eq!(arm.control.summary.first_nourished_meal_latency_ticks, None);
    let patch = act(&mut arm.world, HeadlessActionIds::EAT, Some(setup.target));
    arm.control
        .observe_committed_tick(&arm.world, Some(&patch))
        .unwrap();
    let summary = arm.control.finish();
    assert_eq!(summary.first_nourished_meal_latency_ticks, Some(2));
    assert_eq!(summary.food.nourished_meal_events, 1);
    assert!(
        summary.nourished_meal_energy_gain > 0.0 || summary.nourished_meal_hunger_reduction > 0.0
    );
    assert_eq!(summary.path_length_meters, 0.0);
    assert_eq!(summary.cue_emissions, 0);
    assert_eq!(summary.food_repositions, 0);
    assert_eq!(summary.stop, CareEvaluationStop::EarlyCensoring);
}

#[test]
fn path_length_is_actual_body_motion_and_failed_or_missing_events_are_censored() {
    let (world, mut setup) = source();
    setup.world_ticks = 2;
    let [mut arm, _, _] = forks(&world, setup);
    let initial = position(&arm.world, setup.organism).unwrap();
    let mut expected_path = 0.0;
    let mut previous = initial;
    for _ in 0..2 {
        assert!(arm.control.before_tick(&mut arm.world).unwrap());
        let patch = act(
            &mut arm.world,
            HeadlessActionIds::APPROACH,
            Some(setup.target),
        );
        let current = position(&arm.world, setup.organism).unwrap();
        expected_path += distance(previous, current);
        previous = current;
        arm.control
            .observe_committed_tick(&arm.world, Some(&patch))
            .unwrap();
    }
    assert!(!arm.control.before_tick(&mut arm.world).unwrap());
    let summary = arm.control.finish();
    assert!(expected_path > 0.0);
    assert_eq!(summary.path_length_meters, expected_path);
    assert_eq!(summary.displacement_meters, distance(initial, previous));
    assert_eq!(summary.stop, CareEvaluationStop::WorldTickBound);
    assert_eq!(summary.first_acquisition_latency_ticks, None);
    assert_eq!(summary.first_nourished_meal_latency_ticks, None);
}

#[test]
fn refuses_mismatched_or_repeated_tick_observations_without_committing_telemetry() {
    let (world, setup) = source();
    let [mut arm, _, _] = forks(&world, setup);
    assert!(arm
        .control
        .observe_committed_tick(&arm.world, None)
        .is_err());
    let patch = act(
        &mut arm.world,
        HeadlessActionIds::APPROACH,
        Some(setup.target),
    );
    let (_, other_setup) = source();
    let [mut other, _, _] = forks(&world, other_setup);
    other.world.try_advance_tick().unwrap();
    let wrong_patch = act(
        &mut other.world,
        HeadlessActionIds::APPROACH,
        Some(other_setup.target),
    );
    assert!(arm
        .control
        .observe_committed_tick(&arm.world, Some(&wrong_patch))
        .is_err());
    assert_eq!(arm.control.summary.end_tick, world.tick().raw());
    arm.control
        .observe_committed_tick(&arm.world, Some(&patch))
        .unwrap();
    assert!(arm
        .control
        .observe_committed_tick(&arm.world, Some(&patch))
        .is_err());
    arm.world.try_advance_tick().unwrap();
    arm.world.try_advance_tick().unwrap();
    assert!(arm
        .control
        .observe_committed_tick(&arm.world, None)
        .is_err());
}

#[test]
fn explicit_target_reposition_does_not_move_the_legacy_food_label() {
    let (mut world, mut setup) = source();
    let original = world.entity(setup.target).unwrap().position;
    let target = world
        .editor_spawn_object(alife_world::WorldEditorSpawnSpec {
            label: "held-out-target".into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: original,
            nutrition: 0.6,
            hazard_pain: 0.0,
            radius: 0.1,
            token_id: None,
        })
        .unwrap();
    let legacy = setup.target;
    setup.target = target;
    let [_, _, mut arm] = forks(&world, setup);
    arm.control.before_tick(&mut arm.world).unwrap();
    assert_eq!(arm.world.entity(legacy).unwrap().position, original);
    assert_ne!(
        arm.world.entity(target).unwrap().position,
        setup.held_out_food_position
    );
    assert_eq!(arm.control.summary.food_repositions, 1);
}

#[test]
fn terminal_death_is_observed_without_resurrection_or_any_finalization_callback() {
    let (world, setup) = source();
    let [mut arm, _, _] = forks(&world, setup);
    arm.world.try_advance_tick().unwrap();
    let mut record = arm
        .world
        .organism_registry()
        .get(setup.organism)
        .unwrap()
        .clone();
    record.mark_dead(arm.world.tick()).unwrap();
    arm.world.replace_organism_record_exact(record).unwrap();
    arm.control
        .observe_committed_tick(&arm.world, None)
        .unwrap();
    assert!(!arm.control.before_tick(&mut arm.world).unwrap());
    let summary = arm.control.finish();
    assert_eq!(summary.stop, CareEvaluationStop::TerminalDeath);
    assert_eq!(summary.terminal_death_tick, Some(world.tick().raw() + 1));
    assert_eq!(summary.world_ticks_without_sealed_outcome, 1);
    assert_eq!(summary.first_nourished_meal_latency_ticks, None);
    assert!(world
        .organism_registry()
        .get(setup.organism)
        .unwrap()
        .lifecycle()
        .is_alive());
}

#[test]
fn selected_approach_at_the_current_position_counts_no_body_motion() {
    let (world, mut setup) = source();
    setup.held_out_food_position = position(&world, setup.organism).unwrap();
    let [mut arm, _, _] = forks(&world, setup);
    let before = position(&arm.world, setup.organism).unwrap();
    let patch = act(
        &mut arm.world,
        HeadlessActionIds::APPROACH,
        Some(setup.target),
    );
    assert_eq!(
        patch.decision().selected_action.action_id,
        HeadlessActionIds::APPROACH
    );
    assert_eq!(position(&arm.world, setup.organism).unwrap(), before);
    arm.control
        .observe_committed_tick(&arm.world, Some(&patch))
        .unwrap();
    let summary = arm.control.finish();
    assert_eq!(summary.path_length_meters, 0.0);
    assert_eq!(summary.displacement_meters, 0.0);
}

#[test]
fn a_real_meal_from_another_target_does_not_count_as_target_nourishment() {
    let (mut world, setup) = source();
    let other = world
        .editor_spawn_object(alife_world::WorldEditorSpawnSpec {
            label: "other-meal".into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: position(&world, setup.organism).unwrap(),
            nutrition: 0.6,
            hazard_pain: 0.0,
            radius: 0.1,
            token_id: None,
        })
        .unwrap();
    let [mut arm, _, _] = forks(&world, setup);
    for _ in 0..2 {
        let patch = act(&mut arm.world, HeadlessActionIds::EAT, Some(other));
        arm.control
            .observe_committed_tick(&arm.world, Some(&patch))
            .unwrap();
    }
    assert!(arm.world.entity(other).unwrap().consumed);
    let summary = arm.control.finish();
    assert_eq!(summary.food.nourished_meal_events, 0);
    assert_eq!(summary.first_acquisition_latency_ticks, None);
    assert_eq!(summary.first_nourished_meal_latency_ticks, None);
    assert_eq!(summary.nourished_meal_energy_gain, 0.0);
    assert_eq!(summary.sealed_outcomes_observed, 2);
}

#[test]
fn common_placement_requires_one_unchanged_post_restore_boundary() {
    let (world, setup) = source();
    let [mut early, mut changed, mut valid] =
        prepare_care_evaluation_forks(&world, b"p", b"s", setup).unwrap();
    assert!(early.control.before_tick(&mut early.world).is_err());
    early.world.try_advance_tick().unwrap();
    let after_tick = early.world.canonical_signature_digest().unwrap();
    assert!(early.control.begin_scene(&mut early.world).is_err());
    assert_eq!(
        early.world.canonical_signature_digest().unwrap(),
        after_tick
    );
    changed
        .world
        .editor_move_object(setup.target, setup.held_out_food_position)
        .unwrap();
    let after_move = changed.world.canonical_signature_digest().unwrap();
    assert!(changed.control.begin_scene(&mut changed.world).is_err());
    assert_eq!(
        changed.world.canonical_signature_digest().unwrap(),
        after_move
    );
    valid.control.begin_scene(&mut valid.world).unwrap();
    let begun = valid.world.canonical_signature_digest().unwrap();
    assert!(valid.control.begin_scene(&mut valid.world).is_err());
    assert_eq!(valid.world.canonical_signature_digest().unwrap(), begun);
    assert_eq!(
        early.control.finish().stop,
        CareEvaluationStop::EarlyCensoring
    );
    assert_eq!(
        changed.control.finish().stop,
        CareEvaluationStop::EarlyCensoring
    );
}
