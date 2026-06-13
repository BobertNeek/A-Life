use alife_core::{
    cpu_reference_arbitrate, ActionArbitrationConfig, ActionKind, ActionProposal, ActionTarget,
    Confidence, NormalizedScalar, OrganismId, TeacherLessonResponseChannel,
    TeacherPerceptionChannel, WorldEntityId,
};
use alife_school::{
    Curriculum, CurriculumStepKind, ExpectedObservation, FeedbackPolarity,
    HeadlessCurriculumRunner, LessonId, LessonResponse, LessonResponseKind, PatchLogLessonVerifier,
    SchoolEvidence, TeacherChannelContract, TeacherInputKind, TeacherPerceptualEvent,
    TopologySummary, VerifierCheck, TEACHER_SCHOOL_SCHEMA_VERSION,
};
use alife_world::{ScenarioFixture, ScenarioName};

fn organism() -> OrganismId {
    OrganismId(23)
}

fn proposal(
    action_id: u32,
    kind: ActionKind,
    score: f32,
    target: Option<WorldEntityId>,
) -> ActionProposal {
    ActionProposal::new(
        alife_core::ActionId(action_id),
        kind,
        score,
        Confidence::new(0.9).unwrap(),
        None,
        0b101,
        ActionTarget::new(target, None),
        NormalizedScalar::new(0.7).unwrap(),
    )
    .unwrap()
}

#[test]
fn teacher_channel_contract_only_allows_perception_inputs() {
    let contract = TeacherChannelContract::grounded_default();

    assert_eq!(contract.schema_version, TEACHER_SCHOOL_SCHEMA_VERSION);
    assert!(!contract.hidden_vector_injection_allowed);
    assert!(!contract.direct_motor_bypass_allowed);
    assert_eq!(TeacherInputKind::PERCEPTION_ONLY.len(), 6);
    assert!(TeacherInputKind::PERCEPTION_ONLY
        .iter()
        .all(TeacherInputKind::is_perceptual));

    let events = [
        TeacherPerceptualEvent::spoken_token(LessonId::new(2301).unwrap(), 77),
        TeacherPerceptualEvent::gesture(LessonId::new(2301).unwrap(), 11),
        TeacherPerceptualEvent::object_highlight(
            LessonId::new(2301).unwrap(),
            WorldEntityId(7),
            NormalizedScalar::new(0.8).unwrap(),
        ),
        TeacherPerceptualEvent::social_feedback(
            LessonId::new(2301).unwrap(),
            FeedbackPolarity::Praise,
            Confidence::new(0.9).unwrap(),
        ),
        TeacherPerceptualEvent::visible_reward(
            LessonId::new(2301).unwrap(),
            NormalizedScalar::new(0.6).unwrap(),
        ),
        TeacherPerceptualEvent::visible_punishment(
            LessonId::new(2301).unwrap(),
            NormalizedScalar::new(0.4).unwrap(),
        ),
    ];

    assert!(events.iter().all(|event| contract.accepts_event(event)));
    assert!(events
        .iter()
        .all(|event| !event.hidden_vector_injection_allowed()));
    assert!(events.iter().all(|event| !event.direct_motor_bypass()));
    assert_eq!(events[0].channel(), TeacherPerceptionChannel::Hearing);
    assert_eq!(events[2].channel(), TeacherPerceptionChannel::Object);
}

#[test]
fn grounded_curriculum_defines_required_p23_steps_and_response_channels() {
    let curriculum = Curriculum::grounded_object_food_poison();

    assert_eq!(curriculum.schema_version, TEACHER_SCHOOL_SCHEMA_VERSION);
    assert_eq!(curriculum.steps.len(), 6);
    assert_eq!(
        curriculum
            .steps
            .iter()
            .map(|step| step.kind)
            .collect::<Vec<_>>(),
        vec![
            CurriculumStepKind::NameObject,
            CurriculumStepKind::OfferFood,
            CurriculumStepKind::DiscouragePoison,
            CurriculumStepKind::RequestApproach,
            CurriculumStepKind::RequestGrab,
            CurriculumStepKind::RequestVocalize,
        ]
    );
    assert!(curriculum.lesson_ids_are_unique());
    assert!(curriculum.steps.iter().all(|step| {
        !step.prompt_cues.is_empty()
            && !step.expected_observations.is_empty()
            && !step.verifier_checks.is_empty()
            && !step.feedback_events.is_empty()
            && !step.response_channels.is_empty()
    }));
    assert!(curriculum.steps.iter().any(|step| step
        .expected_observations
        .contains(&ExpectedObservation::PositiveReward)));
    assert!(curriculum.steps.iter().any(|step| step
        .expected_observations
        .contains(&ExpectedObservation::NegativeFeedback)));
}

#[test]
fn curriculum_runner_emits_perceptual_events_and_advances_on_verifier_pass() {
    let curriculum = Curriculum::grounded_object_food_poison();
    let mut runner = HeadlessCurriculumRunner::new(curriculum);
    let current = runner.current_step().unwrap();

    let dispatch = runner.dispatch_current().unwrap();
    assert_eq!(dispatch.lesson_id, current.lesson_id);
    assert!(!dispatch.perception_events.is_empty());
    assert!(dispatch
        .perception_events
        .iter()
        .all(|event| !event.direct_motor_bypass()));
    assert!(dispatch
        .perception_events
        .iter()
        .all(|event| !event.hidden_vector_injection_allowed()));

    let run = ScenarioFixture::named(ScenarioName::TeacherPerceptionEvent)
        .unwrap()
        .run()
        .unwrap();
    let evidence = SchoolEvidence::new(&run.patches)
        .with_memory_record_count(run.memory_record_count)
        .with_topology_summary(TopologySummary {
            concept_count: run.topology_concept_count,
            edge_count: run.topology_edge_count,
            simplex_count: run.topology_simplex_count,
            gap_count: run.topology_gap_ids.len(),
        });

    let passed = PatchLogLessonVerifier
        .verify_checks(
            &[
                VerifierCheck::HeardToken {
                    token_id: 77,
                    channel: TeacherPerceptionChannel::Hearing,
                },
                VerifierCheck::NoHiddenSemanticContext,
                VerifierCheck::SelectedByArbitration,
                VerifierCheck::MinimumMemoryRecords(1),
                VerifierCheck::MinimumTopologyConcepts(2),
            ],
            &evidence,
        )
        .unwrap();
    assert!(passed.passed);

    assert!(runner.observe_verification(&passed).unwrap());
    assert_eq!(runner.completed_step_count(), 1);
}

#[test]
fn patch_log_verifier_passes_and_fails_using_sealed_patch_memory_and_topology_evidence() {
    let run = ScenarioFixture::named(ScenarioName::TeacherPerceptionEvent)
        .unwrap()
        .run()
        .unwrap();
    let evidence = SchoolEvidence::new(&run.patches)
        .with_memory_record_count(run.memory_record_count)
        .with_topology_summary(TopologySummary {
            concept_count: run.topology_concept_count,
            edge_count: run.topology_edge_count,
            simplex_count: run.topology_simplex_count,
            gap_count: run.topology_gap_ids.len(),
        });
    let verifier = PatchLogLessonVerifier;

    let pass = verifier
        .verify_checks(
            &[
                VerifierCheck::HeardToken {
                    token_id: 77,
                    channel: TeacherPerceptionChannel::Hearing,
                },
                VerifierCheck::RewardAtLeast(0.01),
                VerifierCheck::NoDirectTeacherActionSelection,
                VerifierCheck::MinimumMemoryRecords(1),
                VerifierCheck::MinimumTopologyConcepts(2),
            ],
            &evidence,
        )
        .unwrap();
    assert!(pass.passed);
    assert!(pass.failed_checks.is_empty());

    let fail = verifier
        .verify_checks(
            &[VerifierCheck::HeardToken {
                token_id: 999,
                channel: TeacherPerceptionChannel::Hearing,
            }],
            &evidence,
        )
        .unwrap();
    assert!(!fail.passed);
    assert_eq!(fail.failed_checks.len(), 1);
}

#[test]
fn lesson_response_metadata_can_annotate_action_candidates_without_bypassing_arbitration() {
    let lesson_response = LessonResponse::new(
        LessonId::new(2310).unwrap(),
        LessonResponseKind::CreatureVocalized,
        TeacherLessonResponseChannel::Speech,
    )
    .with_teacher_entity(WorldEntityId(42));
    let metadata = lesson_response.to_action_metadata().unwrap();

    let teacher_tagged_low =
        proposal(700, ActionKind::Vocalize, 0.30, None).with_teacher_lesson(Some(metadata));
    let ordinary_high = proposal(701, ActionKind::Inspect, 0.90, None);
    let first = cpu_reference_arbitrate(
        organism(),
        &[teacher_tagged_low, ordinary_high],
        ActionArbitrationConfig::default(),
    )
    .unwrap();
    assert_eq!(first.selected.action_id, ordinary_high.action_id);
    assert_eq!(first.selected.teacher_lesson, None);

    let teacher_tagged_high =
        proposal(702, ActionKind::Vocalize, 0.95, None).with_teacher_lesson(Some(metadata));
    let ordinary_low = proposal(703, ActionKind::Inspect, 0.40, None);
    let second = cpu_reference_arbitrate(
        organism(),
        &[teacher_tagged_high, ordinary_low],
        ActionArbitrationConfig::default(),
    )
    .unwrap();
    assert_eq!(second.selected.action_id, teacher_tagged_high.action_id);
    assert_eq!(second.selected.teacher_lesson, Some(metadata));
}
