use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, CandidateActionFamily,
    CandidateFeatureVector, CandidateObservationRef, Confidence, DurationTicks,
    GroundedObjectSlotV1, HomeostaticSnapshot, NormalizedScalar, OrganismId, PerceptionFrameDraft,
    Pose, SensorProfile, SensorProfileId, SensorProfileProvenance, SensoryAbiVersion,
    SensoryChannels, SensorySnapshot, Tick, TrackedObjectId, Validate, Vec3f, Velocity,
    WorldEntityId, MAX_GROUNDED_OBJECT_SLOTS,
};

fn slot_fixture(
    tracked_object_id: TrackedObjectId,
    slot_index: u16,
    color: [f32; 3],
) -> GroundedObjectSlotV1 {
    GroundedObjectSlotV1 {
        slot_index,
        tracked_object_id,
        bearing: [-0.75, 0.25],
        distance: 0.5,
        relative_velocity: [-0.3, 0.2, 0.1],
        color,
        material: [0.4, 0.5, 0.6],
        shape: [0.7, 0.8, 0.9],
        chemical: [-0.9, 0.25, 0.75],
        contact: 1.0,
        proprioception: [0.33, -0.2],
        temperature: -0.4,
        terrain: [0.6, 0.2],
        confidence: Confidence::new(0.8).unwrap(),
    }
}

fn grounded_candidate(
    index: u16,
    family: CandidateActionFamily,
    observation: CandidateObservationRef,
    features: CandidateFeatureVector,
) -> ActionCandidate {
    let (kind, action_id) = match family {
        CandidateActionFamily::Idle => (ActionKind::Idle, 1),
        CandidateActionFamily::Inspect => (ActionKind::Inspect, 2),
        CandidateActionFamily::Approach | CandidateActionFamily::Avoid => (ActionKind::Move, 3),
        CandidateActionFamily::Contact | CandidateActionFamily::Ingest => (ActionKind::Interact, 4),
        CandidateActionFamily::Rest => (ActionKind::Rest, 5),
        CandidateActionFamily::Other => (ActionKind::Hold, 6),
    };
    ActionCandidate::new(
        index,
        ActionId(action_id),
        kind,
        family,
        observation,
        if family == CandidateActionFamily::Idle {
            ActionTarget::NONE
        } else {
            ActionTarget::new(Some(WorldEntityId(99)), Some(Vec3f::new(1.0, 0.0, 0.0)))
        },
        features,
        Confidence::new(0.8).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(2),
    )
    .unwrap()
}

fn sensory(tick: Tick, channels: SensoryChannels) -> SensorySnapshot {
    SensorySnapshot::new(
        OrganismId(42),
        tick,
        Vec3f::ZERO,
        channels,
        Default::default(),
    )
    .unwrap()
}

fn grounded_draft(
    tick: Tick,
    slots: Vec<GroundedObjectSlotV1>,
    candidates: Vec<ActionCandidate>,
    channels: SensoryChannels,
) -> Result<PerceptionFrameDraft, alife_core::ScaffoldContractError> {
    PerceptionFrameDraft::new(
        OrganismId(42),
        tick,
        SensorProfile::GroundedObjectSlotsV1,
        sensory(tick, channels),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        candidates,
        SensorProfileProvenance::new(
            SensorProfile::GroundedObjectSlotsV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        slots,
    )
}

#[test]
fn grounded_slot_maps_every_named_group_into_exactly_twenty_four_features() {
    let slot = slot_fixture(TrackedObjectId(9), 0, [0.1, 0.2, 0.3]);
    assert_eq!(
        slot.candidate_features().unwrap().0,
        [
            -0.75, 0.25, 0.5, -0.3, 0.2, 0.1, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, -0.9,
            0.25, 0.75, 1.0, 0.33, -0.2, -0.4, 0.6, 0.2,
        ]
    );
}

#[test]
fn provenance_names_profile_version_and_sensory_abi() {
    let provenance = SensorProfileProvenance::new(
        SensorProfile::GroundedObjectSlotsV1,
        SensoryAbiVersion::CURRENT,
        Tick::new(17),
    )
    .unwrap();
    assert_eq!(provenance.schema_version, 1);
    assert_eq!(provenance.profile, SensorProfile::GroundedObjectSlotsV1);
    assert_eq!(provenance.source_tick, Tick::new(17));
    assert_eq!(provenance.identity().profile_id.raw(), 2);
}

#[test]
fn slot_and_provenance_validation_reject_invalid_boundaries() {
    assert!(slot_fixture(TrackedObjectId(0), 0, [0.0; 3])
        .validate_contract()
        .is_err());
    assert!(slot_fixture(
        TrackedObjectId(1),
        MAX_GROUNDED_OBJECT_SLOTS as u16,
        [0.0; 3]
    )
    .validate_contract()
    .is_err());
    assert!(slot_fixture(TrackedObjectId(1), 0, [-0.01, 0.0, 0.0])
        .validate_contract()
        .is_err());
    assert!(slot_fixture(TrackedObjectId(1), 0, [1.01, 0.0, 0.0])
        .validate_contract()
        .is_err());

    let mut wrong_abi = SensorProfileProvenance::new(
        SensorProfile::GroundedObjectSlotsV1,
        SensoryAbiVersion::CURRENT,
        Tick::new(17),
    )
    .unwrap();
    wrong_abi.sensory_abi_version = SensoryAbiVersion(999);
    assert!(wrong_abi.validate_contract().is_err());

    let mut wrong_schema = SensorProfileProvenance::new(
        SensorProfile::GroundedObjectSlotsV1,
        SensoryAbiVersion::CURRENT,
        Tick::new(17),
    )
    .unwrap();
    wrong_schema.schema_version = 999;
    assert!(wrong_schema.validate_contract().is_err());
}

#[test]
fn profile_ids_are_stable_and_unknown_values_are_rejected() {
    assert_eq!(
        SensorProfileId::from(SensorProfile::PrivilegedAffordanceV1).raw(),
        1
    );
    assert_eq!(
        SensorProfileId::from(SensorProfile::GroundedObjectSlotsV1).raw(),
        2
    );
    assert_eq!(
        SensorProfile::try_from(SensorProfileId(2)).unwrap(),
        SensorProfile::GroundedObjectSlotsV1
    );
    assert!(SensorProfile::try_from(SensorProfileId(99)).is_err());
    assert_eq!(OrganismId(42).raw(), 42);
}

#[test]
fn grounded_candidates_bind_exact_contiguous_slots_and_features() {
    let tick = Tick::new(7);
    let slot = slot_fixture(TrackedObjectId(9), 0, [0.1, 0.2, 0.3]);
    let frame = grounded_draft(
        tick,
        vec![slot],
        vec![
            grounded_candidate(
                0,
                CandidateActionFamily::Idle,
                CandidateObservationRef::None,
                CandidateFeatureVector::zero(),
            ),
            grounded_candidate(
                1,
                CandidateActionFamily::Approach,
                CandidateObservationRef::ObjectSlot(0),
                slot.candidate_features().unwrap(),
            ),
        ],
        SensoryChannels::ZERO,
    )
    .unwrap();

    assert_eq!(frame.profile_provenance().source_tick, tick);
    assert_eq!(frame.grounded_object_slots(), &[slot]);
}

#[test]
fn base_digest_authenticates_profile_provenance_and_complete_slot_identity() {
    let tick = Tick::new(7);
    let first_slot = slot_fixture(TrackedObjectId(9), 0, [0.1, 0.2, 0.3]);
    let second_slot = slot_fixture(TrackedObjectId(10), 0, [0.1, 0.2, 0.3]);
    let first = grounded_draft(
        tick,
        vec![first_slot],
        vec![
            grounded_candidate(
                0,
                CandidateActionFamily::Idle,
                CandidateObservationRef::None,
                CandidateFeatureVector::zero(),
            ),
            grounded_candidate(
                1,
                CandidateActionFamily::Approach,
                CandidateObservationRef::ObjectSlot(0),
                first_slot.candidate_features().unwrap(),
            ),
        ],
        SensoryChannels::ZERO,
    )
    .unwrap();
    let second = grounded_draft(
        tick,
        vec![second_slot],
        vec![
            grounded_candidate(
                0,
                CandidateActionFamily::Idle,
                CandidateObservationRef::None,
                CandidateFeatureVector::zero(),
            ),
            grounded_candidate(
                1,
                CandidateActionFamily::Approach,
                CandidateObservationRef::ObjectSlot(0),
                second_slot.candidate_features().unwrap(),
            ),
        ],
        SensoryChannels::ZERO,
    )
    .unwrap();

    assert_ne!(first.base_digest(), second.base_digest());

    let mut tampered_profile = serde_json::to_value(&first).unwrap();
    tampered_profile["profile_provenance"]["sensory_abi_version"] =
        serde_json::json!(SensoryAbiVersion::CURRENT.raw() + 1);
    assert!(serde_json::from_value::<PerceptionFrameDraft>(tampered_profile).is_err());

    let mut tampered_slot = serde_json::to_value(&first).unwrap();
    tampered_slot["grounded_object_slots"][0]["tracked_object_id"] = serde_json::json!(11);
    assert!(serde_json::from_value::<PerceptionFrameDraft>(tampered_slot).is_err());
}

#[test]
fn grounded_and_privileged_cross_profile_references_fail_closed() {
    let tick = Tick::new(7);
    let slot = slot_fixture(TrackedObjectId(9), 0, [0.1, 0.2, 0.3]);
    let idle = grounded_candidate(
        0,
        CandidateActionFamily::Idle,
        CandidateObservationRef::None,
        CandidateFeatureVector::zero(),
    );

    let dangling = grounded_candidate(
        1,
        CandidateActionFamily::Approach,
        CandidateObservationRef::ObjectSlot(1),
        slot.candidate_features().unwrap(),
    );
    assert!(grounded_draft(
        tick,
        vec![slot],
        vec![idle, dangling],
        SensoryChannels::ZERO
    )
    .is_err());

    let missing = grounded_candidate(
        1,
        CandidateActionFamily::Approach,
        CandidateObservationRef::None,
        slot.candidate_features().unwrap(),
    );
    assert!(grounded_draft(tick, vec![slot], vec![idle, missing], SensoryChannels::ZERO).is_err());

    let wrong_idle = grounded_candidate(
        0,
        CandidateActionFamily::Idle,
        CandidateObservationRef::ObjectSlot(0),
        slot.candidate_features().unwrap(),
    );
    assert!(grounded_draft(tick, vec![slot], vec![wrong_idle], SensoryChannels::ZERO).is_err());

    let mismatched = grounded_candidate(
        1,
        CandidateActionFamily::Approach,
        CandidateObservationRef::ObjectSlot(0),
        CandidateFeatureVector::zero(),
    );
    assert!(grounded_draft(
        tick,
        vec![slot],
        vec![idle, mismatched],
        SensoryChannels::ZERO
    )
    .is_err());

    let privileged_slot = PerceptionFrameDraft::new(
        OrganismId(42),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory(tick, SensoryChannels::ZERO),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![grounded_candidate(
            0,
            CandidateActionFamily::Approach,
            CandidateObservationRef::ObjectSlot(0),
            slot.candidate_features().unwrap(),
        )],
        SensorProfileProvenance::new(
            SensorProfile::PrivilegedAffordanceV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        Vec::new(),
    );
    assert!(privileged_slot.is_err());
}

#[test]
fn grounded_profile_rejects_privileged_affordance_channels() {
    let tick = Tick::new(7);
    let mut channels = SensoryChannels::ZERO;
    channels.visual_affordance[0] = 1.0;
    assert!(grounded_draft(
        tick,
        Vec::new(),
        vec![grounded_candidate(
            0,
            CandidateActionFamily::Idle,
            CandidateObservationRef::None,
            CandidateFeatureVector::zero(),
        )],
        channels,
    )
    .is_err());
}
