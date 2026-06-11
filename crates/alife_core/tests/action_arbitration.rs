use alife_core::{
    cpu_reference_arbitrate, ActionArbitrationConfig, ActionArbitrationTraceRef, ActionCommand,
    ActionDecisionStatus, ActionFallbackReason, ActionId, ActionKind, ActionProposal,
    ActionRegistryEntry, ActionScoreBias, ActionTarget, Confidence, DurationTicks, Intensity,
    LobeIndex, MotorPayloadKind, MotorPayloadRef, NormalizedScalar, OrganismId,
    ScaffoldContractError, SuppressionReason, TeacherLessonMetadata, TeacherLessonResponseChannel,
    Validate, Vec3f, WorldEntityId,
};

fn proposal(
    action_id: u32,
    kind: ActionKind,
    score: f32,
    confidence: f32,
    target_entity: Option<WorldEntityId>,
    source_mask: u32,
) -> ActionProposal {
    ActionProposal::new(
        ActionId::new(action_id).unwrap(),
        kind,
        score,
        Confidence::new(confidence).unwrap(),
        Some(LobeIndex(3)),
        source_mask,
        ActionTarget::new(target_entity, Some(Vec3f::new(1.0, 0.0, 2.0))),
        NormalizedScalar::new(0.5).unwrap(),
    )
    .unwrap()
}

#[test]
fn action_registry_keeps_wide_ids_separate_from_kinds() {
    let entry = ActionRegistryEntry::new(ActionId::new(1024).unwrap(), ActionKind::Interact)
        .expect("wide action IDs are valid public ABI values");

    assert_eq!(entry.action_id.raw(), 1024);
    assert_eq!(entry.kind, ActionKind::Interact);
    assert_ne!(
        ActionKind::Interact.canonical_id(),
        ActionKind::Move.canonical_id()
    );
}

#[test]
fn structured_command_preserves_required_fields_and_validates_ranges() {
    let teacher = TeacherLessonMetadata {
        teacher_entity: Some(WorldEntityId(7)),
        lesson_id: 42,
        response_channel: TeacherLessonResponseChannel::Speech,
    };
    let payload = MotorPayloadRef {
        kind: MotorPayloadKind::Speech,
        payload_id: 99,
        schema_version: 1,
    };
    let trace_ref = ActionArbitrationTraceRef::new(123).unwrap();

    let command = ActionCommand::structured(
        OrganismId(1),
        ActionId::new(4096).unwrap(),
        ActionKind::Vocalize,
        ActionTarget::new(Some(WorldEntityId(2)), Some(Vec3f::new(3.0, 4.0, 5.0))),
        Intensity::new(0.6).unwrap(),
        DurationTicks::new(12),
        Confidence::new(0.7).unwrap(),
        0b1010,
        Some(teacher),
        Some(payload),
        Some(trace_ref),
    )
    .unwrap();

    assert_eq!(command.action_id.raw(), 4096);
    assert_eq!(command.target_entity, Some(WorldEntityId(2)));
    assert_eq!(command.target_position, Some(Vec3f::new(3.0, 4.0, 5.0)));
    assert_eq!(command.source_mask, 0b1010);
    assert_eq!(command.teacher_lesson, Some(teacher));
    assert_eq!(command.motor_payload, Some(payload));
    assert_eq!(command.arbitration_trace, Some(trace_ref));
    assert!(command.validate_contract().is_ok());

    let mut bad_position = command;
    bad_position.target_position = Some(Vec3f::new(f32::NAN, 0.0, 0.0));
    assert_eq!(
        bad_position.validate_contract(),
        Err(ScaffoldContractError::NonFiniteFloat)
    );

    let mut bad_duration = command;
    bad_duration.duration_ticks = DurationTicks::ZERO;
    assert_eq!(
        bad_duration.validate_contract(),
        Err(ScaffoldContractError::ScalarOutOfRange)
    );

    let mut bad_intensity = command;
    bad_intensity.intensity = Intensity(1.5);
    assert_eq!(
        bad_intensity.validate_contract(),
        Err(ScaffoldContractError::ScalarOutOfRange)
    );
}

#[test]
fn cpu_arbitration_selects_deterministic_wta_and_preserves_source_mask() {
    let proposals = [
        proposal(
            300,
            ActionKind::Move,
            0.40,
            0.8,
            Some(WorldEntityId(10)),
            0b0001,
        ),
        proposal(
            400,
            ActionKind::Interact,
            0.80,
            0.7,
            Some(WorldEntityId(20)),
            0b1000,
        ),
    ];

    let decision = cpu_reference_arbitrate(
        OrganismId(1),
        &proposals,
        ActionArbitrationConfig::default(),
    )
    .unwrap();

    assert_eq!(decision.status, ActionDecisionStatus::Selected);
    assert_eq!(decision.selected.action_id, ActionId::new(400).unwrap());
    assert_eq!(decision.selected.kind, ActionKind::Interact);
    assert_eq!(decision.selected.target_entity, Some(WorldEntityId(20)));
    assert_eq!(decision.selected.source_mask, 0b1000);
    assert_eq!(decision.rejected_top_proposal.unwrap().proposal_index, 0);
    assert_eq!(decision.trace.wta_result.selected_proposal_index, Some(1));
    assert_eq!(decision.trace.inhibition_inputs.len(), 2);
    assert_eq!(decision.trace.inhibition_outputs.len(), 2);
}

#[test]
fn tie_breaking_is_seeded_and_repeatable() {
    let proposals = [
        proposal(300, ActionKind::Move, 0.75, 0.8, Some(WorldEntityId(10)), 1),
        proposal(
            400,
            ActionKind::Gesture,
            0.75,
            0.8,
            Some(WorldEntityId(11)),
            2,
        ),
    ];
    let config = ActionArbitrationConfig {
        tie_breaker_seed: 55,
        ..ActionArbitrationConfig::default()
    };

    let first = cpu_reference_arbitrate(OrganismId(1), &proposals, config).unwrap();
    let second = cpu_reference_arbitrate(OrganismId(1), &proposals, config).unwrap();

    assert_eq!(first.selected, second.selected);
    assert_eq!(first.trace.tie_breaker_seed, 55);
    assert_eq!(
        first.trace.tie_breaker_index,
        second.trace.tie_breaker_index
    );
    assert_eq!(first.trace.tied_proposal_indices, vec![0, 1]);
}

#[test]
fn fallback_command_is_used_when_no_proposal_passes_threshold() {
    let proposals = [proposal(
        300,
        ActionKind::Move,
        0.10,
        0.8,
        Some(WorldEntityId(10)),
        1,
    )];
    let config = ActionArbitrationConfig {
        min_score: 0.5,
        fallback_kind: ActionKind::Rest,
        ..ActionArbitrationConfig::default()
    };

    let decision = cpu_reference_arbitrate(OrganismId(1), &proposals, config).unwrap();

    assert_eq!(decision.status, ActionDecisionStatus::FallbackSelected);
    assert_eq!(
        decision.fallback_reason,
        Some(ActionFallbackReason::NoEligibleProposal)
    );
    assert_eq!(decision.selected.kind, ActionKind::Rest);
    assert_eq!(decision.selected.action_id, ActionKind::Rest.canonical_id());
    assert_eq!(decision.trace.wta_result.selected_proposal_index, None);
}

#[test]
fn invalid_target_is_suppressed_without_blocking_valid_candidates() {
    let proposals = [
        proposal(
            300,
            ActionKind::Move,
            0.90,
            0.8,
            Some(WorldEntityId::INVALID),
            1,
        ),
        proposal(
            400,
            ActionKind::Interact,
            0.60,
            0.8,
            Some(WorldEntityId(10)),
            2,
        ),
    ];

    let decision = cpu_reference_arbitrate(
        OrganismId(1),
        &proposals,
        ActionArbitrationConfig::default(),
    )
    .unwrap();

    assert_eq!(decision.selected.action_id, ActionId::new(400).unwrap());
    assert_eq!(decision.trace.suppressed_proposals.len(), 1);
    assert_eq!(
        decision.trace.suppressed_proposals[0].reason,
        SuppressionReason::InvalidTarget
    );
}

#[test]
fn teacher_lesson_metadata_does_not_bypass_selection() {
    let teacher_metadata = TeacherLessonMetadata {
        teacher_entity: Some(WorldEntityId(99)),
        lesson_id: 7,
        response_channel: TeacherLessonResponseChannel::Gesture,
    };
    let teacher_tagged = proposal(
        300,
        ActionKind::Gesture,
        0.30,
        0.9,
        Some(WorldEntityId(10)),
        1,
    )
    .with_teacher_lesson(Some(teacher_metadata));
    let ordinary = proposal(400, ActionKind::Move, 0.70, 0.8, Some(WorldEntityId(11)), 2);

    let decision = cpu_reference_arbitrate(
        OrganismId(1),
        &[teacher_tagged, ordinary],
        ActionArbitrationConfig::default(),
    )
    .unwrap();

    assert_eq!(decision.selected.action_id, ActionId::new(400).unwrap());
    assert_eq!(decision.selected.teacher_lesson, None);
    assert_eq!(
        decision
            .rejected_top_proposal
            .unwrap()
            .proposal
            .teacher_lesson,
        Some(teacher_metadata)
    );
}

#[test]
fn memory_expectancy_bias_modulates_scores_without_replay_actions() {
    let biased = proposal(
        300,
        ActionKind::Inspect,
        0.30,
        0.8,
        Some(WorldEntityId(10)),
        1,
    )
    .with_score_bias(ActionScoreBias::memory_expectancy(0.40).unwrap());
    let unbiassed = proposal(400, ActionKind::Move, 0.60, 0.8, Some(WorldEntityId(11)), 2);

    let decision = cpu_reference_arbitrate(
        OrganismId(1),
        &[biased, unbiassed],
        ActionArbitrationConfig::default(),
    )
    .unwrap();

    assert_eq!(decision.selected.action_id, ActionId::new(300).unwrap());

    let fallback = cpu_reference_arbitrate(
        OrganismId(1),
        &[],
        ActionArbitrationConfig {
            fallback_kind: ActionKind::Inspect,
            ..ActionArbitrationConfig::default()
        },
    )
    .unwrap();

    assert_eq!(fallback.status, ActionDecisionStatus::FallbackSelected);
    assert_eq!(fallback.selected.kind, ActionKind::Inspect);
}
