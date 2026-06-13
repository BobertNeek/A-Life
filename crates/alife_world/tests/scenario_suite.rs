use alife_core::{
    ActionKind, AffordanceBits, BrainTickStatus, PhysicalContactKind, ReferenceActionFailure,
    SleepPhase, TeacherPerceptionChannel,
};
use alife_world::{
    ExpectedDirection, ScenarioAssertions, ScenarioFixture, ScenarioName, ScenarioRun,
};

fn run(name: ScenarioName) -> ScenarioRun {
    let fixture = ScenarioFixture::named(name).unwrap();
    ScenarioAssertions::assert_fixture_is_complete(&fixture);
    let run = fixture.run().unwrap();
    ScenarioAssertions::assert_run_matches_expectations(&fixture, &run);
    run
}

#[test]
fn food_seeking_scenario_links_hunger_food_salience_eating_and_reward() {
    let run = run(ScenarioName::FoodSeeking);
    let patch = run.first_patch();

    assert!(patch
        .pre_action()
        .sensory
        .channels
        .nearby_affordances
        .contains(AffordanceBits::FOOD));
    assert!(patch.pre_action().homeostasis.drives.hunger >= 0.8);
    assert_eq!(patch.decision().selected_action.kind, ActionKind::Interact);
    assert_eq!(
        patch.outcome().physical.contact,
        PhysicalContactKind::Consumed
    );
    assert!(patch.outcome().homeostatic_delta.drives.hunger < 0.0);
    assert!(patch.outcome().reward_valence.raw() > 0.0);
    assert!(patch.outcome().energy_delta.raw() > 0.0);
    assert!(run
        .world_signature
        .iter()
        .any(|line| line.contains("berry")));
    assert!(run.world_signature.iter().any(|line| line.contains("true")));
    assert!(run
        .memory_records
        .iter()
        .any(|record| record.safety_bias.raw() > 0.0));
}

#[test]
fn pain_poison_scenario_records_danger_expectancy_without_action_replay() {
    let run = run(ScenarioName::PoisonPainAvoidance);

    let painful = run.patch_at(0);
    assert_eq!(
        painful.outcome().physical.contact,
        PhysicalContactKind::Collision
    );
    assert!(painful.outcome().pain_delta.raw() > 0.0);
    assert!(painful.outcome().homeostatic_delta.drives.fear > 0.0);
    assert!(painful.outcome().homeostatic_delta.hormones.cortisol > 0.0);
    assert!(painful.outcome().reward_valence.raw() < 0.0);

    let repeat = run.patch_at(1);
    assert!(repeat.pre_action().memory_expectancy.danger_bias.raw() > 0.0);
    ScenarioAssertions::assert_memory_expectancy_snapshot_is_bias_only(
        &repeat.pre_action().memory_expectancy,
    );
    assert_eq!(repeat.decision().selected_action.kind, ActionKind::Move);
    assert!(run
        .memory_records
        .iter()
        .any(|record| record.danger_bias.raw() > 0.0));
}

#[test]
fn obstacle_frustration_scenario_records_failure_and_gap() {
    let run = run(ScenarioName::ObstacleFrustration);
    let patch = run.first_patch();

    assert_eq!(
        run.statuses,
        vec![BrainTickStatus::RecoverableActionFailure]
    );
    assert!(!patch.outcome().success);
    assert_eq!(
        patch.outcome().physical.contact,
        PhysicalContactKind::Blocked
    );
    assert!(patch.outcome().frustration_delta.raw() > 0.0);
    assert!(patch.outcome().contradiction_observed);
    assert_eq!(run.failures, vec![Some(ReferenceActionFailure::Blocked)]);
    assert!(!run.topology_gap_ids.is_empty());
    assert!(run
        .curiosity_biases
        .iter()
        .any(|bias| bias.curiosity_voltage.raw() > 0.0));
}

#[test]
fn fatigue_sleep_scenario_uses_p16_sleep_hooks_and_preserves_genetic_baseline() {
    let run = run(ScenarioName::FatigueSleep);
    let patch = run.first_patch();
    let report = run
        .sleep_report
        .as_ref()
        .expect("sleep consolidation report");

    assert_eq!(patch.decision().selected_action.kind, ActionKind::Rest);
    assert!(patch.pre_action().homeostasis.drives.fatigue >= 0.9);
    assert!(patch.outcome().homeostatic_delta.drives.fatigue < 0.0);
    assert_eq!(run.sleep_phase, SleepPhase::ForcedRecoverySleep);
    assert!(run.sleep_transition_observed);
    assert!(run.sleep_cycle_count >= 1);
    assert!(report.neural.genetic_layer_unchanged);
    assert!(!report.structural_edits.candidates().is_empty());
    assert!(run.pending_structural_edit_count >= 1);
}

#[test]
fn curiosity_contradiction_scenario_raises_gap_bias_without_bypassing_arbitration() {
    let run = run(ScenarioName::CuriosityContradiction);
    let patch = run.first_patch();

    assert!(patch.outcome().prediction_error.raw() >= 0.8);
    assert!(patch.outcome().contradiction_observed);
    assert!(patch.outcome().homeostatic_delta.drives.curiosity > 0.0);
    assert!(!run.topology_gap_ids.is_empty());
    assert!(run
        .curiosity_biases
        .iter()
        .any(|bias| { bias.salience.raw() > 0.0 && bias.curiosity_voltage.raw() > 0.0 }));
    ScenarioAssertions::assert_selected_action_came_from_arbitration(patch);
}

#[test]
fn word_token_grounding_scenario_binds_words_through_sensory_context_without_slm() {
    let run = run(ScenarioName::WordTokenGrounding);
    let patch = run.first_patch();

    assert_eq!(
        patch.pre_action().sensory.language_context.heard_tokens[0]
            .unwrap()
            .token_id,
        41
    );
    assert!(patch.pre_action().sensory.semantic_context.is_none());
    assert!(patch.pre_action().sensory.gaussian_context.is_none());
    assert!(run
        .topology_concepts
        .iter()
        .any(|concept| concept.bindings.words.contains(&41)));
}

#[test]
fn social_trust_fear_scenario_records_social_context_as_bias_only() {
    let run = run(ScenarioName::SimpleSocialTrustFear);
    let patch = run.first_patch();

    let social_agents = patch
        .pre_action()
        .sensory
        .social_context
        .nearest_agents
        .iter()
        .flatten()
        .collect::<Vec<_>>();
    assert!(social_agents.iter().any(|agent| agent.affinity.raw() > 0.0));
    assert!(social_agents.iter().any(|agent| agent.affinity.raw() < 0.0));
    assert!(run
        .memory_records
        .iter()
        .any(|record| record.social_trust_bias.raw() > 0.0));
    assert!(run
        .memory_records
        .iter()
        .any(|record| record.social_fear_bias.raw() > 0.0));
    ScenarioAssertions::assert_selected_action_came_from_arbitration(patch);
}

#[test]
fn teacher_perception_event_is_perceptual_and_modulatory_only() {
    let run = run(ScenarioName::TeacherPerceptionEvent);
    let patch = run.first_patch();
    let heard = patch.pre_action().sensory.language_context.heard_tokens[0].unwrap();

    assert_eq!(heard.token_id, 77);
    assert_eq!(
        heard.teacher_channel,
        Some(TeacherPerceptionChannel::Hearing)
    );
    assert_eq!(
        patch
            .pre_action()
            .sensory
            .language_context
            .teacher_channel_marker,
        Some(TeacherPerceptionChannel::Hearing)
    );
    assert!(patch.decision().selected_action.teacher_lesson.is_none());
    assert!(patch.pre_action().sensory.semantic_context.is_none());
    ScenarioAssertions::assert_selected_action_came_from_arbitration(patch);
}

#[test]
fn combined_scenario_smoke_covers_all_named_scenarios() {
    for name in ScenarioName::ALL {
        let run = run(name);
        assert_eq!(run.name, name);
        assert!(!run.patch_summaries.is_empty());
        assert!(run.memory_record_count >= run.fixture_expectations.memory.min_records);
        assert!(run.topology_concept_count >= run.fixture_expectations.topology.min_concepts);
    }
}

#[test]
fn same_seed_and_scenario_produce_same_broad_patch_summary() {
    for name in ScenarioName::ALL {
        let first = ScenarioFixture::with_seed(name, 9901)
            .unwrap()
            .run()
            .unwrap();
        let second = ScenarioFixture::with_seed(name, 9901)
            .unwrap()
            .run()
            .unwrap();

        assert_eq!(first.patch_summaries, second.patch_summaries, "{name:?}");
        assert_eq!(first.world_signature, second.world_signature, "{name:?}");
        assert_eq!(first.summary, second.summary, "{name:?}");
    }
}

#[test]
fn expected_direction_helper_checks_direction_not_exact_scores() {
    ScenarioAssertions::assert_direction("positive", 0.1, ExpectedDirection::Positive);
    ScenarioAssertions::assert_direction("negative", -0.1, ExpectedDirection::Negative);
    ScenarioAssertions::assert_direction("nonnegative", 0.0, ExpectedDirection::NonNegative);
    ScenarioAssertions::assert_direction("nonpositive", 0.0, ExpectedDirection::NonPositive);
}
