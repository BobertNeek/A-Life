use alife_core::{
    heuristic_baseline_arbitrate, ActionArbitrationConfig, ActionCandidate, ActionCommand,
    ActionId, ActionKind, ActionProposal, ActionTarget, BodySnapshot, BrainClassSpec, BrainGenome,
    BrainScaleTier, CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef,
    Confidence, DecisionSnapshot, DevelopmentState, DurationTicks, ExperiencePatch,
    ExperiencePatchBuilder, ExperienceSequenceId, HomeostaticDelta, HomeostaticSnapshot, Intensity,
    LobeKind, NeuralActionSelection, NormalizedScalar, OrganismId, PerceptionFrame, PhenotypeHash,
    PhysicalActionOutcome, PhysicalContactKind, Pose, PostActionOutcome, PreActionSnapshot,
    ScaffoldContractError, SensorProfile, SensorProfileProvenance, SensoryAbiVersion,
    SensoryChannels, SensorySnapshot, SignedValence, TeacherLessonResponseChannel,
    TeacherPerceptionChannel, Tick, Vec3f, Velocity, WorldEntityId,
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

fn neural_patch() -> ExperiencePatch {
    let tick = Tick::new(7);
    let sequence_id = ExperienceSequenceId(991);
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let genome = BrainGenome::scaffold(42, spec.id);
    let sensory = SensorySnapshot::new(
        organism(),
        tick,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    let candidate = ActionCandidate::new(
        0,
        ActionId(810),
        ActionKind::Inspect,
        CandidateActionFamily::Inspect,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero(),
        Confidence::new(0.9).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(1),
    )
    .unwrap();
    let frame = PerceptionFrame::new(
        organism(),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![candidate],
        SensorProfileProvenance::new(
            SensorProfile::PrivilegedAffordanceV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        Vec::new(),
    )
    .unwrap();
    let selection = NeuralActionSelection {
        candidate_index: 0,
        logit: 0.75,
        confidence: Confidence::new(0.8).unwrap(),
        active_tiles: 12,
        active_synapses: 144,
    };
    let phenotype_hash = PhenotypeHash([11, 22, 33, 44]);
    let teacher_metadata = LessonResponse::new(
        LessonId::new(2_311).unwrap(),
        LessonResponseKind::CreatureVocalized,
        TeacherLessonResponseChannel::Speech,
    )
    .with_teacher_entity(WorldEntityId(42))
    .to_action_metadata()
    .unwrap();
    let teacher_bypass = ActionCommand::structured(
        organism(),
        candidate.action_id,
        candidate.kind,
        candidate.target,
        Intensity::new(1.0).unwrap(),
        candidate.min_duration,
        selection.confidence,
        0,
        Some(teacher_metadata),
        None,
        None,
    )
    .unwrap();
    assert!(matches!(
        DecisionSnapshot::from_neural_selection(
            sequence_id,
            phenotype_hash,
            7,
            1,
            &frame,
            selection,
            teacher_bypass,
        ),
        Err(ScaffoldContractError::InvalidDecisionEvidence)
    ));

    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id,
        phenotype_hash,
        7,
        1,
        &frame,
        selection,
        candidate
            .to_command(organism(), selection.confidence)
            .unwrap(),
    )
    .unwrap();
    let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.35).unwrap())
        .with_enabled_lobes([
            LobeKind::SensoryGrounding,
            LobeKind::CoreAssociation,
            LobeKind::MotorArbitration,
        ]);
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence_id,
        spec.id,
        phenotype_hash,
        genome.id,
        genome.schema_version,
        development,
        frame,
    )
    .unwrap();
    let outcome = PostActionOutcome::new(
        organism(),
        sequence_id,
        Tick::new(8),
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.0).unwrap(),
        },
        HomeostaticDelta::zero(),
        SignedValence::new(0.25).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        SignedValence::new(-0.1).unwrap(),
        NormalizedScalar::new(0.2).unwrap(),
    )
    .unwrap();

    ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
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
                VerifierCheck::MinimumTopologyConcepts(1),
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
                VerifierCheck::MinimumTopologyConcepts(1),
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
fn neural_patch_is_audited_from_typed_candidate_evidence_without_a_heuristic_trace() {
    let patch = neural_patch();
    assert!(patch.decision().selected_action.arbitration_trace.is_none());
    assert!(matches!(
        patch.decision().heuristic_evidence(),
        Err(ScaffoldContractError::EvidenceKindMismatch)
    ));

    let verification = PatchLogLessonVerifier
        .verify_checks(
            &[
                VerifierCheck::NoDirectTeacherActionSelection,
                VerifierCheck::SelectedByArbitration,
            ],
            &SchoolEvidence::new(std::slice::from_ref(&patch)),
        )
        .unwrap();

    assert!(verification.passed);
    assert!(verification.failed_checks.is_empty());
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
    let first = heuristic_baseline_arbitrate(
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
    let second = heuristic_baseline_arbitrate(
        organism(),
        &[teacher_tagged_high, ordinary_low],
        ActionArbitrationConfig::default(),
    )
    .unwrap();
    assert_eq!(second.selected.action_id, teacher_tagged_high.action_id);
    assert_eq!(second.selected.teacher_lesson, Some(metadata));
}
