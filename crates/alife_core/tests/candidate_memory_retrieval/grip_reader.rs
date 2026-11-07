//! Synthetic storage/routing contracts only, not evidence of learned release.
use super::*;
use alife_core::memory_query::MEMORY_TERRAIN_VISION_PROFILE_LANE;
use alife_core::{
    CandidateMemoryQueryV2, GripOperationV1, GripOwnershipV1, GroundedGripSlotV1,
    MemoryQueryEncoderV2, MEMORY_ACTION_FAMILY_RANGE, MEMORY_PROFILE_RANGE,
};

const PROFILE: SensorProfile = SensorProfile::GroundedGripObjectSlotsV1;

fn draft(tick: u64, tracked: u64, operation: Option<GripOperationV1>) -> PerceptionFrameDraft {
    let object = slot(0, tracked, 0.4, [0.0, 0.8, 0.9]);
    let mut candidate = candidate_for_family(&object, 0, CandidateActionFamily::Ingest);
    let ownership = if let Some(operation) = operation {
        candidate.action_id = operation.action_id();
        candidate.family = operation.family();
        if operation == GripOperationV1::Release {
            GripOwnershipV1::SelfHeld
        } else {
            GripOwnershipV1::NotSelfHeld
        }
    } else {
        GripOwnershipV1::NotSelfHeld
    };
    custom_draft(PROFILE, tick, object, candidate, ownership).unwrap()
}

fn custom_draft(
    profile: SensorProfile,
    tick: u64,
    object: GroundedObjectSlotV1,
    candidate: ActionCandidate,
    ownership: GripOwnershipV1,
) -> Result<PerceptionFrameDraft, ScaffoldContractError> {
    let tick = Tick::new(tick);
    PerceptionFrameDraft::new_with_grip_observations(
        ORGANISM,
        tick,
        profile,
        SensorySnapshot::new(
            ORGANISM,
            tick,
            Vec3f::ZERO,
            SensoryChannels::ZERO,
            Default::default(),
        )
        .unwrap(),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![candidate],
        SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
        vec![object],
        if profile == PROFILE {
            vec![GroundedGripSlotV1 {
                slot_index: 0,
                ownership,
                confidence: Confidence::new(0.9).unwrap(),
            }]
        } else {
            Vec::new()
        },
    )
}

fn event(
    bank: &MemoryBank,
    sequence: u64,
    operation: Option<GripOperationV1>,
    contact: PhysicalContactKind,
    reward: f32,
) -> ExperiencePatch {
    observation_from_draft(
        bank,
        sequence,
        draft(sequence, 71, operation),
        reward,
        if reward < 0.0 { 0.9 } else { 0.0 },
        |outcome| {
            outcome.physical.target_entity = Some(WorldEntityId(9000));
            outcome.physical.contact = contact;
            outcome.prediction_error = NormalizedScalar::new(0.2).unwrap();
        },
    )
}

#[test]
fn grip_readouts_receive_same_target_ingest_valence_without_relabeling_keys_or_storage() {
    for reward in [-0.9, 0.9] {
        let mut bank = empty_bank();
        bank.observe_sealed_patch(&event(
            &empty_bank(),
            1,
            None,
            PhysicalContactKind::Consumed,
            reward,
        ))
        .unwrap();
        let original_bytes = serde_json::to_vec(&bank).unwrap();
        for operation in [GripOperationV1::Close, GripOperationV1::Release] {
            let probe = draft(2, 71, Some(operation));
            let query =
                MemoryQueryEncoderV2::encode_candidate(&probe, &probe.candidates()[0]).unwrap();
            let query_wire = serde_json::to_vec(&query).unwrap();
            assert_eq!(&query.features()[MEMORY_PROFILE_RANGE], &[1.0, 1.0]);
            assert_eq!(query.features()[MEMORY_TERRAIN_VISION_PROFILE_LANE], 0.0);
            let recalled = bank.recall_frame(&probe).unwrap();
            recalled.validate_for_draft(&probe).unwrap();
            let context = recalled.context().candidates[0];
            assert_eq!(
                context.family_value[0].is_sign_negative(),
                reward.is_sign_negative()
            );
            assert!(context.family_value[0].abs() > 0.8);
            assert_eq!(context.family_source_count, 1);
            // The target channel is still the ordinary cross-action lookup.
            assert_eq!(
                context.target_latent,
                bank.recall_frame(&draft(2, 71, None))
                    .unwrap()
                    .context()
                    .candidates[0]
                    .target_latent
            );
            let receipt = &recalled.receipt().candidates[0];
            assert_eq!(receipt.query_digest, query.canonical_digest());
            assert_eq!(
                receipt.family_bucket.family_raw,
                u16::from(operation.family().raw())
            );
            assert_eq!(
                receipt.family_bucket.other_action_id_raw,
                if operation == GripOperationV1::Release {
                    operation.action_id().raw()
                } else {
                    0
                }
            );
            let attention = recalled.attention_evidence_for_draft(&probe).unwrap();
            assert_eq!(attention[0].tracked_object_id, Some(TrackedObjectId(71)));
            let (frame, finalized) = recalled.finalize(probe).unwrap();
            finalized.validate_for_frame(&frame).unwrap();
            assert_eq!(finalized.candidate_keys()[0].query(), &query);
            assert_eq!(
                serde_json::to_vec(finalized.candidate_keys()[0].query()).unwrap(),
                query_wire
            );
            let restored: CandidateMemoryQueryV2 = serde_json::from_slice(&query_wire).unwrap();
            assert_eq!(restored, query);
        }
        assert_eq!(
            serde_json::to_vec(&bank).unwrap(),
            original_bytes,
            "recall is read-only"
        );
        for (sequence, operation, contact) in [
            (2, GripOperationV1::Close, PhysicalContactKind::Touch),
            (3, GripOperationV1::Release, PhysicalContactKind::Moved),
        ] {
            let patch = event(&bank, sequence, Some(operation), contact, 0.01);
            bank.observe_sealed_patch(&patch).unwrap();
        }
        let wire = serde_json::to_value(&bank).unwrap();
        let records = wire["candidate_store"]["records"].as_object().unwrap();
        assert_eq!(records.len(), 3);
        for operation in [GripOperationV1::Close, GripOperationV1::Release] {
            let record = records
                .values()
                .find(|r| r["action_id_raw"] == operation.action_id().raw())
                .unwrap();
            assert_eq!(record["family_raw"], u16::from(operation.family().raw()));
            assert!(
                record.get("outcome_contact").is_none(),
                "grip outcomes never become Consumed"
            );
            assert_eq!(
                record["query_features"]
                    [MEMORY_ACTION_FAMILY_RANGE.start + usize::from(operation.family().raw())],
                1.0
            );
        }
        assert_eq!(
            bank.recall_frame(&draft(4, 71, Some(GripOperationV1::Release)))
                .unwrap()
                .context()
                .candidates[0]
                .family_value[0]
                .is_sign_negative(),
            reward.is_sign_negative()
        );
    }
}

#[test]
fn grip_route_keeps_consumed_priority_and_json_portable_provenance() {
    let profile = draft(1, 71, None).profile_provenance().identity();
    let mut sidecar = MemorySidecarState::new_profiled(
        ORGANISM,
        profile,
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let mut bank = empty_bank();
    for (sequence, contact, reward) in [
        (1, PhysicalContactKind::Consumed, -0.9),
        (2, PhysicalContactKind::Touch, 0.8),
    ] {
        let patch = event(&bank, sequence, None, contact, reward);
        bank.observe_sealed_patch(&patch).unwrap();
        sidecar.observe_sealed_patch(&patch).unwrap();
    }
    // A newer Unknown record also cannot outrank actual consumption.
    let unknown = observation_from_draft(&bank, 3, draft(3, 71, None), 0.8, 0.0, |outcome| {
        outcome.physical.target_entity = Some(WorldEntityId(99999))
    });
    bank.observe_sealed_patch(&unknown).unwrap();
    sidecar.observe_sealed_patch(&unknown).unwrap();
    let bytes = serde_json::to_vec(&bank).unwrap();
    let restored: MemoryBank = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    for operation in [GripOperationV1::Close, GripOperationV1::Release] {
        let probe = draft(4, 71, Some(operation));
        let audited = restored.recall_frame(&probe).unwrap();
        let runtime = restored
            .recall_frame_runtime(&probe, &Default::default(), true)
            .unwrap()
            .0;
        assert!(audited.context().candidates[0].family_value[0] < -0.8);
        assert_eq!(audited.context().candidates, runtime.context().candidates);
    }
    for (sequence, operation, contact) in [
        (4, GripOperationV1::Close, PhysicalContactKind::Touch),
        (5, GripOperationV1::Release, PhysicalContactKind::Moved),
    ] {
        let patch = event(&bank, sequence, Some(operation), contact, 0.01);
        sidecar.observe_sealed_patch(&patch).unwrap();
    }
    let portable = sidecar.export_active_bank().unwrap();
    for operation in [GripOperationV1::Close, GripOperationV1::Release] {
        let record = portable
            .records
            .iter()
            .find(|r| r.action_id_raw == operation.action_id().raw())
            .unwrap();
        assert_eq!(record.family_raw, u16::from(operation.family().raw()));
        assert_eq!(record.outcome_contact, None);
    }
    assert!(portable
        .records
        .iter()
        .any(|r| r.outcome_contact == Some(PhysicalContactKind::Consumed)));
    assert!(portable
        .records
        .iter()
        .any(|r| r.outcome_contact == Some(PhysicalContactKind::Touch)));
    assert!(portable.records.iter().any(|r| r.outcome_contact.is_none()));
    let restored = MemorySidecarState::restore_portable(
        profile,
        *sidecar.compaction_checkpoint(),
        portable.clone(),
        None,
    )
    .unwrap();
    assert_eq!(restored.export_active_bank().unwrap(), portable);
    assert!(
        restored
            .recall_frame(&draft(6, 71, Some(GripOperationV1::Release)))
            .unwrap()
            .context()
            .candidates[0]
            .family_value[0]
            < -0.8
    );
    let old_profile = probe_for_object(4, slot(0, 71, 0.4, [0.0, 0.8, 0.9]))
        .profile_provenance()
        .identity();
    assert!(MemorySidecarState::restore_portable(
        old_profile,
        *sidecar.compaction_checkpoint(),
        portable,
        None
    )
    .is_err());
}

#[test]
fn grip_route_rejects_foreign_targets_and_namespaces_without_category_fallback() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&event(
        &empty_bank(),
        1,
        None,
        PhysicalContactKind::Consumed,
        -0.9,
    ))
    .unwrap();
    for operation in [GripOperationV1::Close, GripOperationV1::Release] {
        let unrelated = draft(2, 72, Some(operation));
        for recalled in [
            bank.recall_frame(&unrelated).unwrap(),
            bank.recall_frame_runtime(&unrelated, &Default::default(), true)
                .unwrap()
                .0,
        ] {
            assert_eq!(recalled.context().candidates[0].family_source_count, 0);
            assert_eq!(
                recalled.context().candidates[0].family_value,
                [0.0; MEMORY_VALUE_V1_COUNT]
            );
        }
        // Same target number and appearance from the legacy profile are not an
        // acquired memory in the new namespace.
        let mut legacy = empty_bank();
        legacy
            .observe_sealed_patch(&observation_patch(
                &empty_bank(),
                1,
                1,
                slot(0, 71, 0.4, [0.0, 0.8, 0.9]),
                CandidateActionFamily::Ingest,
                -0.9,
                0.9,
                |outcome| outcome.physical.target_entity = Some(WorldEntityId(9000)),
            ))
            .unwrap();
        assert_eq!(
            legacy
                .recall_frame(&draft(2, 71, Some(operation)))
                .unwrap()
                .context()
                .candidates[0]
                .family_source_count,
            0
        );
    }
    // Same profile/target, wrong owner: also no family transfer.
    let mut foreign = serde_json::to_value(&bank).unwrap();
    foreign["candidate_store"]["records"]["1"]["organism_id_raw"] = serde_json::json!(812_u64);
    foreign["candidate_store"]["last_sequence_by_organism"] = serde_json::json!({"812":1});
    let foreign: MemoryBank = serde_json::from_value(foreign).unwrap();
    assert_eq!(
        foreign
            .recall_frame(&draft(2, 71, Some(GripOperationV1::Close)))
            .unwrap()
            .context()
            .candidates[0]
            .family_source_count,
        0
    );
}

#[test]
fn only_validated_tracked_grip_operations_route_ingest_family_memory() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&event(
        &empty_bank(),
        1,
        None,
        PhysicalContactKind::Consumed,
        -0.9,
    ))
    .unwrap();
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let good = draft(2, 71, Some(GripOperationV1::Release)).candidates()[0];
    for fault in 0..5 {
        let mut bad = good;
        let mut profile = PROFILE;
        let mut ownership = GripOwnershipV1::SelfHeld;
        match fault {
            0 => bad.family = CandidateActionFamily::Contact,
            1 => bad.kind = ActionKind::Hold,
            2 => {
                bad.observation = CandidateObservationRef::None;
                bad.target = ActionTarget::NONE;
                bad.features = alife_core::CandidateFeatureVector::zero();
            }
            3 => profile = SensorProfile::GroundedObjectSlotsV1,
            4 => ownership = GripOwnershipV1::NotSelfHeld,
            _ => unreachable!(),
        }
        assert!(custom_draft(profile, 2, object, bad, ownership).is_err());
    }
    let mut ordinary = good;
    ordinary.action_id = ActionId(9876);
    let ordinary = custom_draft(PROFILE, 2, object, ordinary, GripOwnershipV1::SelfHeld).unwrap();
    assert_eq!(
        bank.recall_frame(&ordinary).unwrap().context().candidates[0].family_source_count,
        0
    );
    let play_patch = observation_from_draft(&bank, 2, ordinary.clone(), 0.8, 0.0, |outcome| {
        outcome.physical.target_entity = Some(WorldEntityId(9000));
        outcome.physical.contact = PhysicalContactKind::Touch;
    });
    bank.observe_sealed_patch(&play_patch).unwrap();
    assert!(
        bank.recall_frame(&ordinary).unwrap().context().candidates[0].family_value[0] > 0.7,
        "ordinary Other actions read their own acquired outcome, not the harmful meal"
    );
    assert!(
        bank.recall_frame(&draft(3, 71, Some(GripOperationV1::Release)))
            .unwrap()
            .context()
            .candidates[0]
            .family_value[0]
            < -0.8
    );
    let query = MemoryQueryEncoderV2::encode_candidate(
        &draft(2, 71, Some(GripOperationV1::Close)),
        &draft(2, 71, Some(GripOperationV1::Close)).candidates()[0],
    )
    .unwrap();
    for lane in [
        MEMORY_PROFILE_RANGE.start,
        MEMORY_PROFILE_RANGE.start + 1,
        MEMORY_TERRAIN_VISION_PROFILE_LANE,
    ] {
        let mut wire = serde_json::to_value(&query).unwrap();
        wire["features"][lane] = serde_json::json!(if lane == MEMORY_TERRAIN_VISION_PROFILE_LANE {
            1.0
        } else {
            0.0
        });
        assert!(serde_json::from_value::<CandidateMemoryQueryV2>(wire).is_err());
    }
}

#[test]
fn grip_runtime_cache_binds_operation_target_context_and_memory_generation() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&event(
        &empty_bank(),
        1,
        None,
        PhysicalContactKind::Consumed,
        -0.9,
    ))
    .unwrap();
    let first = draft(2, 71, Some(GripOperationV1::Close));
    let (original, cache) = bank
        .recall_frame_runtime(&first, &Default::default(), true)
        .unwrap();
    let cache = cache.unwrap();
    let later = draft(3, 71, Some(GripOperationV1::Close));
    let (reused, next) = bank.recall_frame_runtime(&later, &cache, false).unwrap();
    assert!(next.is_none());
    assert_eq!(reused.receipt().similarity_evaluations, 0);
    assert_eq!(reused.context().candidates, original.context().candidates);
    let (frame, finalized) = reused.finalize(later).unwrap();
    assert_eq!(finalized.candidate_keys()[0].query().tick(), Tick::new(3));
    finalized.validate_for_frame(&frame).unwrap();
    for changed in [
        draft(3, 72, Some(GripOperationV1::Close)),
        draft(3, 71, Some(GripOperationV1::Release)),
        draft(3, 71, Some(GripOperationV1::Close))
            .with_remembered_novelty(0.5)
            .unwrap(),
    ] {
        let (recalled, next) = bank.recall_frame_runtime(&changed, &cache, false).unwrap();
        assert!(
            next.is_some(),
            "changed target, operation or neural query context must search"
        );
        if changed.grounded_object_slots()[0].tracked_object_id == TrackedObjectId(72) {
            assert_eq!(recalled.context().candidates[0].family_source_count, 0);
        } else {
            assert!(recalled.receipt().similarity_evaluations > 0);
        }
    }
    let reversal = event(&bank, 2, None, PhysicalContactKind::Consumed, 0.9);
    bank.observe_sealed_patch(&reversal).unwrap();
    let (recalled, next) = bank.recall_frame_runtime(&first, &cache, false).unwrap();
    assert!(next.is_some());
    assert!(recalled.context().candidates[0].family_value[0] > 0.8);
}

#[test]
fn ordinary_other_keeps_bad_play_recall_separate_from_safe_ingest() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&event(
        &empty_bank(),
        1,
        None,
        PhysicalContactKind::Consumed,
        0.9,
    ))
    .unwrap();
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let mut candidate = draft(2, 71, Some(GripOperationV1::Release)).candidates()[0];
    candidate.action_id = ActionId(212);
    let play = custom_draft(PROFILE, 2, object, candidate, GripOwnershipV1::SelfHeld).unwrap();
    let patch = observation_from_draft(&bank, 2, play.clone(), -0.8, 0.8, |outcome| {
        outcome.physical.target_entity = Some(WorldEntityId(9000));
        outcome.physical.contact = PhysicalContactKind::Touch;
    });
    bank.observe_sealed_patch(&patch).unwrap();
    for recall in [
        bank.recall_frame(&play).unwrap(),
        bank.recall_frame_runtime(&play, &Default::default(), true)
            .unwrap()
            .0,
    ] {
        assert!(recall.context().candidates[0].family_value[0] < -0.7);
    }
    assert!(
        bank.recall_frame(&draft(3, 71, Some(GripOperationV1::Release)))
            .unwrap()
            .context()
            .candidates[0]
            .family_value[0]
            > 0.8
    );
}

#[test]
fn within_frame_contact_cache_cannot_alias_a_grip_ingest_read() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&event(
        &empty_bank(),
        1,
        None,
        PhysicalContactKind::Consumed,
        -0.9,
    ))
    .unwrap();
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let close = draft(2, 71, Some(GripOperationV1::Close));
    let mut contact = close.candidates()[0];
    contact.action_id = ActionId(9876);
    let contact_draft =
        custom_draft(PROFILE, 2, object, contact, GripOwnershipV1::NotSelfHeld).unwrap();
    let patch = observation_from_draft(&bank, 2, contact_draft, 0.8, 0.0, |outcome| {
        outcome.physical.target_entity = Some(WorldEntityId(9000));
        outcome.physical.contact = PhysicalContactKind::Touch;
    });
    bank.observe_sealed_patch(&patch).unwrap();
    for grip_first in [false, true] {
        let mut candidates = if grip_first {
            vec![close.candidates()[0], contact]
        } else {
            vec![contact, close.candidates()[0]]
        };
        for (index, candidate) in candidates.iter_mut().enumerate() {
            candidate.candidate_index = index as u16;
        }
        let mixed = PerceptionFrameDraft::new_with_grip_observations(
            ORGANISM,
            close.tick(),
            PROFILE,
            close.sensory().clone(),
            close.body(),
            *close.homeostasis(),
            candidates,
            close.profile_provenance(),
            vec![object],
            vec![GroundedGripSlotV1 {
                slot_index: 0,
                ownership: GripOwnershipV1::NotSelfHeld,
                confidence: Confidence::new(0.9).unwrap(),
            }],
        )
        .unwrap();
        let recalled = bank.recall_frame(&mixed).unwrap();
        recalled.validate_for_draft(&mixed).unwrap();
        for (candidate, context) in mixed
            .candidates()
            .iter()
            .zip(&recalled.context().candidates)
        {
            if candidate.action_id == GripOperationV1::Close.action_id() {
                assert!(context.family_value[0] < -0.8);
            } else {
                assert!(context.family_value[0] > 0.7);
            }
        }
    }
}
