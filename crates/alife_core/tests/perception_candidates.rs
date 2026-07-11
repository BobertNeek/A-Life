use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, CandidateActionFamily,
    CandidateFeatureVector, CandidateObservationRef, Confidence, DurationTicks,
    HomeostaticSnapshot, NormalizedScalar, OrganismId, PerceptionContextBlock,
    PerceptionContextKind, PerceptionFrame, PerceptionFrameDraft, PolicyBackend, Pose,
    SensorProfile, SensoryChannels, SensorySnapshot, Tick, Vec3f, Velocity, WorldEntityId,
    MAX_ACTION_CANDIDATES,
};

fn organism() -> OrganismId {
    OrganismId(1)
}

fn sensory(tick: Tick) -> SensorySnapshot {
    SensorySnapshot::new(
        organism(),
        tick,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap()
}

fn candidate(
    candidate_index: u16,
    action_id: u32,
    kind: ActionKind,
    family: CandidateActionFamily,
) -> ActionCandidate {
    ActionCandidate::new(
        candidate_index,
        ActionId(action_id),
        kind,
        family,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero(),
        Confidence::new(1.0).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(2),
    )
    .unwrap()
}

fn perception_draft_fixture() -> PerceptionFrameDraft {
    let tick = Tick::new(7);
    PerceptionFrameDraft::new(
        organism(),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory(tick),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![
            candidate(0, 4, ActionKind::Inspect, CandidateActionFamily::Inspect),
            ActionCandidate::new(
                1,
                ActionId(101),
                ActionKind::Move,
                CandidateActionFamily::Approach,
                CandidateObservationRef::ObjectSlot(3),
                ActionTarget::new(Some(WorldEntityId(44)), Some(Vec3f::new(1.0, 0.0, 2.0))),
                CandidateFeatureVector::zero(),
                Confidence::new(0.9).unwrap(),
                NormalizedScalar::new(0.2).unwrap(),
                DurationTicks::new(2),
                DurationTicks::new(4),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

#[test]
fn candidate_is_unscored_and_frame_is_same_tick() {
    let tick = Tick::new(7);
    let sensory = sensory(tick);
    let candidate = ActionCandidate::new(
        0,
        ActionId(101),
        ActionKind::Move,
        CandidateActionFamily::Approach,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero(),
        Confidence::new(1.0).unwrap(),
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
    )
    .unwrap();

    assert_eq!(frame.tick(), frame.sensory().tick);
    assert_eq!(frame.candidates()[0].candidate_index, 0);
    assert_eq!(PolicyBackend::default(), PolicyBackend::NeuralClosedLoopGpu);
    let command = frame.candidates()[0]
        .to_command(organism(), Confidence::new(0.8).unwrap())
        .unwrap();
    assert_eq!(command.action_id, ActionId(101));
    assert_eq!(command.duration_ticks, DurationTicks::new(1));
}

#[test]
fn candidate_validation_rejects_duplicate_indices_and_non_finite_features() {
    let mut features = CandidateFeatureVector::zero();
    features.0[0] = f32::NAN;
    assert!(features.validate().is_err());
    features.0[0] = 1.01;
    assert!(features.validate().is_err());

    let tick = Tick::new(7);
    let duplicate = vec![
        candidate(0, 4, ActionKind::Inspect, CandidateActionFamily::Inspect),
        candidate(0, 100, ActionKind::Move, CandidateActionFamily::Approach),
    ];
    assert!(PerceptionFrame::new(
        organism(),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory(tick),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        duplicate,
    )
    .is_err());
}

#[test]
fn frame_validation_enforces_bounds_identity_and_family_compatibility() {
    let tick = Tick::new(7);
    let body = BodySnapshot {
        pose: Pose::IDENTITY,
        velocity: Velocity::ZERO,
    };
    let homeostasis = HomeostaticSnapshot::baseline(tick);

    assert!(PerceptionFrame::new(
        organism(),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory(tick),
        body,
        homeostasis,
        Vec::new(),
    )
    .is_err());

    let too_many = (0..=MAX_ACTION_CANDIDATES)
        .map(|index| {
            candidate(
                index as u16,
                100 + index as u32,
                ActionKind::Move,
                CandidateActionFamily::Approach,
            )
        })
        .collect();
    assert!(PerceptionFrame::new(
        organism(),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory(tick),
        body,
        homeostasis,
        too_many,
    )
    .is_err());

    assert!(ActionCandidate::new(
        0,
        ActionId(101),
        ActionKind::Interact,
        CandidateActionFamily::Approach,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero(),
        Confidence::new(1.0).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
        DurationTicks::new(2),
        DurationTicks::new(1),
    )
    .is_err());

    assert!(PerceptionFrame::new(
        organism(),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        SensorySnapshot::new(
            organism(),
            Tick::new(8),
            Vec3f::ZERO,
            SensoryChannels::ZERO,
            Default::default(),
        )
        .unwrap(),
        body,
        homeostasis,
        vec![candidate(
            0,
            4,
            ActionKind::Inspect,
            CandidateActionFamily::Inspect,
        )],
    )
    .is_err());
}

#[test]
fn candidate_family_raw_mapping_is_stable_and_total() {
    for raw in 0u8..=7 {
        let family = CandidateActionFamily::try_from_raw(raw).unwrap();
        assert_eq!(family.raw(), raw);
    }
    assert!(CandidateActionFamily::try_from_raw(8).is_err());
    assert_eq!(SensorProfile::PrivilegedAffordanceV1.raw(), 1);
    assert_eq!(SensorProfile::GroundedObjectSlotsV1.raw(), 2);
    assert!(SensorProfile::try_from_raw(0).is_err());
    assert!(SensorProfile::try_from_raw(3).is_err());
}

#[test]
fn base_digest_precedes_context_and_final_digest_without_a_cycle() {
    let draft = perception_draft_fixture();
    let base = draft.base_digest();
    let empty = draft
        .clone()
        .finalize(PerceptionContextBlock::empty())
        .unwrap();
    let recalled = draft
        .finalize(
            PerceptionContextBlock::try_new(
                1,
                PerceptionContextKind::EpisodicCandidateV1,
                vec![0.25, -0.5],
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(empty.base_digest(), base);
    assert_eq!(recalled.base_digest(), base);
    assert_ne!(empty.frame_digest(), recalled.frame_digest());
    assert_ne!(empty.base_digest().0, empty.frame_digest().0);
}

#[test]
fn candidate_feature_digest_excludes_transport_entity_but_frame_digest_binds_it() {
    let draft = perception_draft_fixture();
    let first = draft
        .clone()
        .finalize(PerceptionContextBlock::empty())
        .unwrap();
    let mut changed_candidate = first.candidates()[1];
    changed_candidate.target =
        ActionTarget::new(Some(WorldEntityId(45)), changed_candidate.target.position);
    let changed = PerceptionFrame::new(
        first.organism_id(),
        first.tick(),
        first.sensor_profile(),
        first.sensory().clone(),
        first.body(),
        *first.homeostasis(),
        vec![first.candidates()[0], changed_candidate],
    )
    .unwrap();

    assert_eq!(
        first.candidates()[1].feature_digest(),
        changed.candidates()[1].feature_digest()
    );
    assert_ne!(first.base_digest(), changed.base_digest());
    assert_ne!(first.frame_digest(), changed.frame_digest());
}

#[test]
fn tampered_serialized_base_context_or_final_digest_is_rejected() {
    let frame = perception_draft_fixture()
        .finalize(
            PerceptionContextBlock::try_new(
                1,
                PerceptionContextKind::EpisodicCandidateV1,
                vec![0.25, -0.5],
            )
            .unwrap(),
        )
        .unwrap();
    let original = serde_json::to_value(frame).unwrap();

    let mut base = original.clone();
    base["base"]["base_digest"][0] = serde_json::json!(u64::MAX);
    let mut context = original.clone();
    context["context"]["canonical_digest"][0] = serde_json::json!(u64::MAX);
    let mut final_digest = original;
    final_digest["frame_digest"][0] = serde_json::json!(u64::MAX);

    for tampered in [base, context, final_digest] {
        assert!(serde_json::from_value::<PerceptionFrame>(tampered).is_err());
    }
}

#[test]
fn action_candidate_source_has_no_score_field() {
    let source = include_str!("../src/perception.rs");
    let candidate = source.split("pub struct ActionCandidate").nth(1).unwrap();
    let candidate = candidate.split('}').next().unwrap();
    assert!(!candidate.contains("score:"));
    assert!(!candidate.contains("utility:"));
}
