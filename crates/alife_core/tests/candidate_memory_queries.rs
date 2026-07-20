use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, CandidateActionFamily,
    CandidateMemoryContextV1, CandidateMemoryQueryV2, CandidateObservationRef, Confidence,
    DriveSnapshot, DurationTicks, EndocrineSnapshot, EpisodicRetrievalContextV1,
    GroundedObjectSlotV1, HomeostaticSnapshot, MemoryId, MemoryQueryEncoderV2, MemoryQueryVersion,
    NormalizedScalar, OrganismId, PerceptionContextBlock, PerceptionFrameDraft, Pose,
    SensorProfile, SensorProfileProvenance, SensoryAbiVersion, SensoryChannels, SensorySnapshot,
    Tick, TrackedObjectId, Vec3f, Velocity, WorldEntityId, MEMORY_ACTION_FAMILY_RANGE,
    MEMORY_ACTION_KIND_RANGE, MEMORY_BODY_RANGE, MEMORY_DRIVE_RANGE, MEMORY_HORMONE_RANGE,
    MEMORY_LATENT_V1_COUNT, MEMORY_PROFILE_RANGE, MEMORY_QUERY_V2_FEATURE_COUNT,
    MEMORY_RESERVED_RANGE, MEMORY_STATE_SENSORY_RANGE, MEMORY_TARGET_RANGE, MEMORY_VALUE_V1_COUNT,
};

const ORGANISM: OrganismId = OrganismId(42);
const TICK: Tick = Tick::new(17);

fn slot() -> GroundedObjectSlotV1 {
    GroundedObjectSlotV1 {
        slot_index: 0,
        tracked_object_id: TrackedObjectId(7001),
        bearing: [-0.75, 0.25],
        distance: 0.5,
        relative_velocity: [-0.3, 0.2, 0.1],
        color: [0.1, 0.2, 0.3],
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

fn candidate(index: u16, family: CandidateActionFamily) -> ActionCandidate {
    let (kind, action_id) = match family {
        CandidateActionFamily::Approach => (ActionKind::Move, ActionId(101)),
        CandidateActionFamily::Avoid => (ActionKind::Move, ActionId(102)),
        CandidateActionFamily::Contact => (ActionKind::Interact, ActionId(201)),
        CandidateActionFamily::Ingest => (ActionKind::Interact, ActionId(202)),
        _ => unreachable!("fixture uses target-bearing candidate families"),
    };
    ActionCandidate::new(
        index,
        action_id,
        kind,
        family,
        CandidateObservationRef::ObjectSlot(0),
        ActionTarget::new(Some(WorldEntityId(88)), Some(Vec3f::new(1.0, 0.5, -0.25))),
        slot().candidate_features().unwrap(),
        Confidence::new(0.85).unwrap(),
        NormalizedScalar::new(0.3).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(3),
    )
    .unwrap()
}

fn fully_populated_grounded_draft() -> PerceptionFrameDraft {
    let mut channels = SensoryChannels::ZERO;
    for (index, value) in channels.auditory_acoustic.iter_mut().enumerate() {
        *value = 0.1 + index as f32 * 0.05;
    }
    for (index, value) in channels.smell_chemistry.iter_mut().enumerate() {
        *value = 0.2 + index as f32 * 0.04;
    }
    for (index, value) in channels.tactile_contact.iter_mut().enumerate() {
        *value = 0.3 + index as f32 * 0.03;
    }
    channels.pain_signal = NormalizedScalar::new(0.45).unwrap();
    channels.novelty_signal = NormalizedScalar::new(0.65).unwrap();
    let sensory =
        SensorySnapshot::new(ORGANISM, TICK, Vec3f::ZERO, channels, Default::default()).unwrap();
    let drives = DriveSnapshot {
        hunger: 0.91,
        fatigue: 0.82,
        fear: 0.73,
        pain: 0.64,
        loneliness: 0.55,
        curiosity: 0.46,
        brain_atp: 0.37,
        temperature_stress: 0.28,
        reproductive_drive: 0.19,
        extension: [0.11, 0.22],
    };
    let hormones = EndocrineSnapshot {
        adrenaline: 0.12,
        cortisol: 0.23,
        dopamine: 0.34,
        oxytocin: 0.45,
        serotonin: 0.56,
        acetylcholine: 0.67,
        learning_modulator: 0.78,
        developmental_hormone: 0.89,
        sleep_pressure: 0.76,
        extension: [0.31, 0.42],
    };
    PerceptionFrameDraft::new(
        ORGANISM,
        TICK,
        SensorProfile::GroundedObjectSlotsV1,
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity {
                linear: Vec3f::new(0.1, -0.2, 0.3),
                angular: Vec3f::new(-0.4, 0.5, -0.6),
            },
        },
        HomeostaticSnapshot::new(TICK, drives, hormones).unwrap(),
        vec![
            candidate(0, CandidateActionFamily::Approach),
            candidate(1, CandidateActionFamily::Avoid),
            candidate(2, CandidateActionFamily::Contact),
            candidate(3, CandidateActionFamily::Ingest),
        ],
        SensorProfileProvenance::new(
            SensorProfile::GroundedObjectSlotsV1,
            SensoryAbiVersion::CURRENT,
            TICK,
        )
        .unwrap(),
        vec![slot()],
    )
    .unwrap()
}

#[test]
fn late_drive_hormone_action_and_target_strata_survive_full_input() {
    let draft = fully_populated_grounded_draft();
    let candidate = draft.candidates()[0];
    let query = MemoryQueryEncoderV2::encode_candidate(&draft, &candidate).unwrap();

    assert_eq!(query.features().len(), MEMORY_QUERY_V2_FEATURE_COUNT);
    assert_eq!(query.version(), MemoryQueryVersion::StateActionTargetV2);
    assert!(query.features()[MEMORY_STATE_SENSORY_RANGE]
        .iter()
        .any(|value| *value != 0.0));
    assert_eq!(
        &query.features()[MEMORY_DRIVE_RANGE],
        &draft.homeostasis().drives.to_array()
    );
    assert_eq!(
        &query.features()[MEMORY_HORMONE_RANGE],
        &draft.homeostasis().hormones.to_array()
    );
    assert_eq!(
        &query.features()[MEMORY_BODY_RANGE],
        &[0.1, -0.2, 0.3, -0.4, 0.5, -0.6]
    );
    assert_eq!(
        query.features()[MEMORY_ACTION_KIND_RANGE]
            .iter()
            .filter(|value| **value == 1.0)
            .count(),
        1
    );
    assert_eq!(
        query.features()[MEMORY_ACTION_FAMILY_RANGE]
            .iter()
            .filter(|value| **value == 1.0)
            .count(),
        1
    );
    assert_eq!(
        &query.features()[MEMORY_TARGET_RANGE],
        &candidate.features.0
    );
    assert_eq!(&query.features()[MEMORY_PROFILE_RANGE], &[0.0, 1.0]);
    assert!(query.features()[MEMORY_RESERVED_RANGE]
        .iter()
        .all(|value| *value == 0.0));
    assert_eq!(query.tracked_object_id(), Some(TrackedObjectId(7001)));
    assert_eq!(query.base_frame_digest(), draft.base_digest());
}

#[test]
fn opposite_families_with_the_same_action_kind_have_different_queries() {
    let draft = fully_populated_grounded_draft();
    let approach = MemoryQueryEncoderV2::encode_candidate(&draft, &draft.candidates()[0]).unwrap();
    let avoid = MemoryQueryEncoderV2::encode_candidate(&draft, &draft.candidates()[1]).unwrap();
    let contact = MemoryQueryEncoderV2::encode_candidate(&draft, &draft.candidates()[2]).unwrap();
    let ingest = MemoryQueryEncoderV2::encode_candidate(&draft, &draft.candidates()[3]).unwrap();

    assert_eq!(approach.action_kind(), avoid.action_kind());
    assert_eq!(contact.action_kind(), ingest.action_kind());
    assert_ne!(approach.features(), avoid.features());
    assert_ne!(contact.features(), ingest.features());
    assert_ne!(approach.canonical_digest(), avoid.canonical_digest());
    assert_ne!(contact.canonical_digest(), ingest.canonical_digest());
}

#[test]
fn target_and_action_change_queries_but_transport_entity_id_does_not() {
    let draft = fully_populated_grounded_draft();
    let candidate = draft.candidates()[0];
    let mut different_target = candidate;
    different_target.features.0[6] = 0.95;
    let mut different_transport = candidate;
    different_transport.target =
        ActionTarget::new(Some(WorldEntityId(999)), candidate.target.position);

    let original = MemoryQueryEncoderV2::encode_candidate(&draft, &candidate).unwrap();
    let target = MemoryQueryEncoderV2::encode_candidate(&draft, &different_target).unwrap();
    let transport = MemoryQueryEncoderV2::encode_candidate(&draft, &different_transport).unwrap();
    assert_ne!(original.features(), target.features());
    assert_ne!(original.canonical_digest(), target.canonical_digest());
    assert_eq!(original.features(), transport.features());
    assert_eq!(original.canonical_digest(), transport.canonical_digest());
    let original_wire = serde_json::to_value(&original).unwrap();
    let transport_wire = serde_json::to_value(&transport).unwrap();
    assert_eq!(transport_wire, original_wire);
    let transport_fields = transport_wire.as_object().unwrap();
    assert!(!transport_fields.contains_key("target"));
    assert!(!transport_fields.contains_key("world_entity_id"));
    assert!(!transport_fields.contains_key("target_entity_id"));
}

#[test]
fn finalized_frame_validation_reencodes_the_exact_candidate_query() {
    let draft = fully_populated_grounded_draft();
    let query = MemoryQueryEncoderV2::encode_candidate(&draft, &draft.candidates()[0]).unwrap();
    let frame = draft.finalize(PerceptionContextBlock::empty()).unwrap();
    query
        .validate_against_frame(&frame, &frame.candidates()[0])
        .unwrap();
    assert!(query
        .validate_against_frame(&frame, &frame.candidates()[1])
        .is_err());
}

#[test]
fn serialized_queries_recompute_and_authenticate_every_private_field() {
    let draft = fully_populated_grounded_draft();
    let query = MemoryQueryEncoderV2::encode_candidate(&draft, &draft.candidates()[0]).unwrap();
    let value = serde_json::to_value(&query).unwrap();
    let roundtrip: CandidateMemoryQueryV2 = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(roundtrip, query);

    let mut feature_tamper = value.clone();
    feature_tamper["features"][57] = serde_json::json!(0.123);
    assert!(serde_json::from_value::<CandidateMemoryQueryV2>(feature_tamper).is_err());
    let mut digest_tamper = value;
    digest_tamper["canonical_digest"][0] = serde_json::json!(0);
    assert!(serde_json::from_value::<CandidateMemoryQueryV2>(digest_tamper).is_err());
}

fn context(candidate_index: u16) -> CandidateMemoryContextV1 {
    CandidateMemoryContextV1 {
        candidate_index,
        target_latent: [0.1; MEMORY_LATENT_V1_COUNT],
        family_value: [-0.2; MEMORY_VALUE_V1_COUNT],
        target_confidence: Confidence::new(0.7).unwrap(),
        family_confidence: Confidence::new(0.8).unwrap(),
        target_source_count: 3,
        family_source_count: 4,
        best_target_source: Some(MemoryId(11)),
        best_family_source: Some(MemoryId(12)),
    }
}

#[test]
fn retrieval_context_emits_exact_candidate_local_gpu_rows_without_source_ids() {
    let draft = fully_populated_grounded_draft();
    let retrieval = EpisodicRetrievalContextV1::new(
        TICK,
        draft.profile_provenance().identity(),
        (0..4).map(context).collect(),
    )
    .unwrap();
    let block = retrieval.to_perception_context_block().unwrap();
    assert_eq!(block.values().len(), 4 * 16);
    assert_eq!(
        &block.values()[0..16],
        &[0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1, -0.2, -0.2, -0.2, -0.2, 0.7, 0.8, 3.0, 4.0,]
    );

    let mut diagnostics_changed = retrieval.clone();
    diagnostics_changed.candidates[0].best_target_source = Some(MemoryId(91));
    diagnostics_changed.candidates[0].best_family_source = Some(MemoryId(92));
    let changed = diagnostics_changed.to_perception_context_block().unwrap();
    assert_eq!(changed.canonical_digest(), block.canonical_digest());
}

#[test]
fn retrieval_context_rejects_noncontiguous_rows_and_nonfinite_values() {
    let draft = fully_populated_grounded_draft();
    assert!(EpisodicRetrievalContextV1::new(
        TICK,
        draft.profile_provenance().identity(),
        vec![context(0), context(2)],
    )
    .is_err());
    let mut invalid = context(0);
    invalid.target_latent[0] = f32::NAN;
    assert!(EpisodicRetrievalContextV1::new(
        TICK,
        draft.profile_provenance().identity(),
        vec![invalid],
    )
    .is_err());

    let mut contradictory = context(0);
    contradictory.target_source_count = 0;
    assert!(EpisodicRetrievalContextV1::new(
        TICK,
        draft.profile_provenance().identity(),
        vec![contradictory],
    )
    .is_err());

    let mut missing_source = context(0);
    missing_source.best_family_source = None;
    assert!(EpisodicRetrievalContextV1::new(
        TICK,
        draft.profile_provenance().identity(),
        vec![missing_source],
    )
    .is_err());
}
