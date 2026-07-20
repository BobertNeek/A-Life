use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, BrainClassSpec, BrainGenome,
    BrainScaleTier, CandidateActionFamily, CandidateObservationRef, Confidence, DecisionSnapshot,
    DevelopmentState, DriveDelta, DriveSnapshot, DurationTicks, EndocrineDelta, EndocrineSnapshot,
    ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId, GroundedObjectSlotV1,
    HomeostaticDelta, HomeostaticSnapshot, LobeKind, MemoryBank, MemoryBankConfig,
    MemoryCompactionPhase, MemorySidecarState, NeuralActionSelection, NormalizedScalar, OrganismId,
    PerceptionContextKind, PerceptionFrame, PerceptionFrameDraft, PhenotypeHash,
    PhysicalActionOutcome, PhysicalContactKind, Pose, PostActionOutcome, PreActionSnapshot,
    ScaffoldContractError, SensorProfile, SensorProfileProvenance, SensoryAbiVersion,
    SensoryChannels, SensorySnapshot, SignedValence, Tick, TrackedObjectId, Validate, Vec3f,
    Velocity, WorldEntityId, MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE, MEMORY_LATENT_V1_COUNT,
    MEMORY_VALUE_V1_COUNT,
};

const ORGANISM: OrganismId = OrganismId(811);
const TICK: Tick = Tick::new(40);

fn slot(slot_index: u16, tracked: u64, distance: f32, color: [f32; 3]) -> GroundedObjectSlotV1 {
    GroundedObjectSlotV1 {
        slot_index,
        tracked_object_id: TrackedObjectId(tracked),
        bearing: [0.2 * (f32::from(slot_index) + 1.0), -0.1],
        distance,
        relative_velocity: [0.0, 0.0, 0.0],
        color,
        material: [0.2, 0.4, 0.6],
        shape: [0.1, 0.5, 0.9],
        chemical: [0.3, 0.0, -0.2],
        contact: 0.0,
        proprioception: [0.0, 0.0],
        temperature: 0.1,
        terrain: [0.5, 0.25],
        confidence: Confidence::new(0.9).unwrap(),
    }
}

fn candidate(slot: &GroundedObjectSlotV1, candidate_index: u16) -> ActionCandidate {
    candidate_for_family(slot, candidate_index, CandidateActionFamily::Ingest)
}

fn candidate_for_family(
    slot: &GroundedObjectSlotV1,
    candidate_index: u16,
    family: CandidateActionFamily,
) -> ActionCandidate {
    let kind = match family {
        CandidateActionFamily::Approach | CandidateActionFamily::Avoid => ActionKind::Move,
        CandidateActionFamily::Contact | CandidateActionFamily::Ingest => ActionKind::Interact,
        _ => unreachable!("fixture uses target-bearing families"),
    };
    ActionCandidate::new(
        candidate_index,
        ActionId(200 + u32::from(candidate_index)),
        kind,
        family,
        CandidateObservationRef::ObjectSlot(slot.slot_index),
        ActionTarget::new(
            Some(WorldEntityId(9_000 + u64::from(candidate_index))),
            Some(Vec3f::new(slot.distance, 0.0, 0.0)),
        ),
        slot.candidate_features().unwrap(),
        Confidence::new(0.9).unwrap(),
        NormalizedScalar::new(0.2).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(2),
    )
    .unwrap()
}

fn cyan_amber_family_draft() -> PerceptionFrameDraft {
    let slots = vec![
        slot(0, 71, 0.4, [0.0, 0.8, 0.9]),
        slot(1, 72, 0.7, [0.9, 0.6, 0.1]),
    ];
    let candidates = vec![
        candidate_for_family(&slots[0], 0, CandidateActionFamily::Ingest),
        candidate_for_family(&slots[0], 1, CandidateActionFamily::Avoid),
        candidate_for_family(&slots[1], 2, CandidateActionFamily::Ingest),
    ];
    let sensory = SensorySnapshot::new(
        ORGANISM,
        TICK,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    PerceptionFrameDraft::new(
        ORGANISM,
        TICK,
        SensorProfile::GroundedObjectSlotsV1,
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::new(
            TICK,
            DriveSnapshot {
                hunger: 0.8,
                ..DriveSnapshot::baseline()
            },
            EndocrineSnapshot {
                learning_modulator: 0.7,
                ..EndocrineSnapshot::baseline()
            },
        )
        .unwrap(),
        candidates,
        SensorProfileProvenance::new(
            SensorProfile::GroundedObjectSlotsV1,
            SensoryAbiVersion::CURRENT,
            TICK,
        )
        .unwrap(),
        slots,
    )
    .unwrap()
}

fn grounded_draft(first_distance: f32) -> PerceptionFrameDraft {
    let slots = vec![
        slot(0, 71, first_distance, [0.0, 0.8, 0.9]),
        slot(1, 72, 0.7, [0.9, 0.6, 0.1]),
    ];
    let candidates = slots
        .iter()
        .enumerate()
        .map(|(index, slot)| candidate(slot, index as u16))
        .collect();
    let sensory = SensorySnapshot::new(
        ORGANISM,
        TICK,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    let drives = DriveSnapshot {
        hunger: 0.8,
        ..DriveSnapshot::baseline()
    };
    let hormones = EndocrineSnapshot {
        learning_modulator: 0.7,
        ..EndocrineSnapshot::baseline()
    };
    PerceptionFrameDraft::new(
        ORGANISM,
        TICK,
        SensorProfile::GroundedObjectSlotsV1,
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::new(TICK, drives, hormones).unwrap(),
        candidates,
        SensorProfileProvenance::new(
            SensorProfile::GroundedObjectSlotsV1,
            SensoryAbiVersion::CURRENT,
            TICK,
        )
        .unwrap(),
        slots,
    )
    .unwrap()
}

fn empty_bank() -> MemoryBank {
    MemoryBank::new(MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap())
        .unwrap()
}

fn grounded_sidecar(config: MemoryBankConfig) -> MemorySidecarState {
    MemorySidecarState::new_profiled(
        ORGANISM,
        SensorProfileProvenance::new(
            SensorProfile::GroundedObjectSlotsV1,
            SensoryAbiVersion::CURRENT,
            TICK,
        )
        .unwrap()
        .identity(),
        config,
    )
    .unwrap()
}

fn sequence() -> ExperienceSequenceId {
    ExperienceSequenceId(101)
}

fn neural_decision(frame: &PerceptionFrame, selected_index: usize) -> DecisionSnapshot {
    let candidate = &frame.candidates()[selected_index];
    let selection = NeuralActionSelection {
        candidate_index: candidate.candidate_index,
        logit: 0.75,
        confidence: Confidence::new(0.8).unwrap(),
        active_tiles: 8,
        active_synapses: 64,
    };
    DecisionSnapshot::from_neural_selection(
        sequence(),
        PhenotypeHash([1, 2, 3, 4]),
        9,
        1,
        frame,
        selection,
        candidate
            .to_command(ORGANISM, Confidence::new(0.8).unwrap())
            .unwrap(),
    )
    .unwrap()
}

fn pre_action(frame: PerceptionFrame) -> PreActionSnapshot {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let genome = BrainGenome::scaffold(321, spec.id);
    let development =
        DevelopmentState::new(genome.id, frame.tick(), NormalizedScalar::new(0.5).unwrap())
            .with_enabled_lobes([
                LobeKind::SensoryGrounding,
                LobeKind::CoreAssociation,
                LobeKind::MotorArbitration,
            ]);
    PreActionSnapshot::from_neural_frame(
        sequence(),
        spec.id,
        PhenotypeHash([1, 2, 3, 4]),
        genome.id,
        genome.schema_version,
        development,
        frame,
    )
    .unwrap()
}

fn outcome() -> PostActionOutcome {
    PostActionOutcome::new(
        ORGANISM,
        sequence(),
        Tick::new(TICK.raw() + 1),
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Consumed,
            target_entity: Some(WorldEntityId(9_000)),
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta::zero(),
        SignedValence::new(0.4).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        SignedValence::new(0.2).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
    )
    .unwrap()
}

fn poisoned_outcome() -> PostActionOutcome {
    PostActionOutcome::new(
        ORGANISM,
        sequence(),
        Tick::new(TICK.raw() + 1),
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Consumed,
            target_entity: Some(WorldEntityId(9_000)),
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.2).unwrap(),
        },
        HomeostaticDelta {
            drives: DriveDelta {
                hunger: -0.2,
                fear: 0.7,
                pain: 0.9,
                curiosity: -0.1,
                brain_atp: -0.3,
                ..DriveDelta::zero()
            },
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(-0.8).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
        NormalizedScalar::new(0.9).unwrap(),
        SignedValence::new(-0.3).unwrap(),
        NormalizedScalar::new(0.7).unwrap(),
    )
    .unwrap()
}

fn poisoned_cyan_ingest_patch() -> ExperiencePatch {
    let bank = empty_bank();
    let draft = grounded_draft(0.4);
    let (frame, finalized) = bank.recall_frame(&draft).unwrap().finalize(draft).unwrap();
    let decision = neural_decision(&frame, 0)
        .with_finalized_memory_recall(&frame, &finalized, 0)
        .unwrap();
    ExperiencePatchBuilder::new(sequence())
        .record_pre_action(pre_action(frame))
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(poisoned_outcome())
        .unwrap()
        .seal()
        .unwrap()
}

fn sequenced_patch(
    sequence_raw: u64,
    tick_raw: u64,
    tracked_raw: u64,
    distance: f32,
    reward: f32,
    pain: f32,
) -> ExperiencePatch {
    let tick = Tick::new(tick_raw);
    let sequence = ExperienceSequenceId(sequence_raw);
    let color = if tracked_raw == 71 {
        [0.0, 0.8, 0.9]
    } else {
        [0.2, 0.5, 0.8]
    };
    let object = slot(0, tracked_raw, distance, color);
    let candidate = candidate(&object, 0);
    let sensory = SensorySnapshot::new(
        ORGANISM,
        tick,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    let draft = PerceptionFrameDraft::new(
        ORGANISM,
        tick,
        SensorProfile::GroundedObjectSlotsV1,
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![candidate],
        SensorProfileProvenance::new(
            SensorProfile::GroundedObjectSlotsV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        vec![object],
    )
    .unwrap();
    let recall_bank = empty_bank();
    let (frame, finalized) = recall_bank
        .recall_frame(&draft)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let selected = &frame.candidates()[0];
    let decision = DecisionSnapshot::from_neural_selection(
        sequence,
        PhenotypeHash([1, 2, 3, 4]),
        sequence_raw,
        (sequence_raw & 1) as u8,
        &frame,
        NeuralActionSelection {
            candidate_index: 0,
            logit: 0.7,
            confidence: Confidence::new(0.8).unwrap(),
            active_tiles: 8,
            active_synapses: 64,
        },
        selected
            .to_command(ORGANISM, Confidence::new(0.8).unwrap())
            .unwrap(),
    )
    .unwrap()
    .with_finalized_memory_recall(&frame, &finalized, 0)
    .unwrap();
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let genome = BrainGenome::scaffold(321, spec.id);
    let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.5).unwrap())
        .with_enabled_lobes([
            LobeKind::SensoryGrounding,
            LobeKind::CoreAssociation,
            LobeKind::MotorArbitration,
        ]);
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence,
        spec.id,
        PhenotypeHash([1, 2, 3, 4]),
        genome.id,
        genome.schema_version,
        development,
        frame,
    )
    .unwrap();
    let outcome = PostActionOutcome::new(
        ORGANISM,
        sequence,
        Tick::new(tick_raw + 1),
        reward >= 0.0,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Consumed,
            target_entity: Some(WorldEntityId(9_000 + tracked_raw)),
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta {
            drives: DriveDelta {
                pain,
                fear: pain,
                brain_atp: -0.1,
                ..DriveDelta::zero()
            },
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(reward).unwrap(),
        NormalizedScalar::new(if reward < 0.0 { 0.5 } else { 0.0 }).unwrap(),
        NormalizedScalar::new(pain).unwrap(),
        SignedValence::new(-0.1).unwrap(),
        NormalizedScalar::new(pain.max(0.1)).unwrap(),
    )
    .unwrap();
    ExperiencePatchBuilder::new(sequence)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}

fn saturation_run() -> ([u64; 4], usize, u64, u64, u32) {
    let config = MemoryBankConfig::new(4, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
    let mut sidecar = grounded_sidecar(config);
    let mut compactions = 0_u32;
    let mut last_update = None;
    for sequence_raw in 1..=10_240_u64 {
        let retained = sequence_raw % 3 == 0;
        let (tracked, distance, reward, pain) = if retained {
            (71, 0.4, -1.0, 1.0)
        } else {
            (
                10_000 + sequence_raw,
                0.2 + (sequence_raw % 5) as f32 * 0.1,
                0.1,
                0.0,
            )
        };
        last_update = Some(
            sidecar
                .observe_sealed_patch(&sequenced_patch(
                    sequence_raw,
                    sequence_raw * 2,
                    tracked,
                    distance,
                    reward,
                    pain,
                ))
                .unwrap(),
        );
        if sequence_raw % 256 == 0 {
            let cycle = u64::from(compactions) + 1;
            let prepared = sidecar.prepare_compaction(cycle, 4, 1).unwrap();
            sidecar.commit_compaction(prepared).unwrap();
            compactions += 1;
        }
    }
    let update = last_update.unwrap();
    let probe = cyan_amber_family_draft();
    let digest = sidecar.recall_frame(&probe).unwrap().receipt().bank_digest;
    (
        digest,
        sidecar.bank().len(),
        update.merge_count,
        update.eviction_count,
        compactions,
    )
}

#[test]
fn empty_recall_finalizes_exact_candidate_keys_and_rejects_a_changed_draft() {
    let bank = empty_bank();
    let draft = grounded_draft(0.4);
    let prepared = bank.recall_frame(&draft).unwrap();
    assert_eq!(prepared.base_frame_digest(), draft.base_digest());
    assert_eq!(
        prepared.context().candidates.len(),
        draft.candidates().len()
    );
    for (index, context) in prepared.context().candidates.iter().enumerate() {
        assert_eq!(usize::from(context.candidate_index), index);
        assert_eq!(context.target_latent, [0.0; MEMORY_LATENT_V1_COUNT]);
        assert_eq!(context.family_value, [0.0; MEMORY_VALUE_V1_COUNT]);
        assert_eq!(context.target_source_count, 0);
        assert_eq!(context.family_source_count, 0);
    }
    assert_eq!(prepared.receipt().candidate_count, 2);
    assert_eq!(prepared.receipt().similarity_evaluations, 0);
    assert!(prepared.finalize(grounded_draft(0.2)).is_err());

    let draft = grounded_draft(0.4);
    let prepared = bank.recall_frame(&draft).unwrap();
    let (frame, finalized) = prepared.finalize(draft).unwrap();
    finalized.validate_for_frame(&frame).unwrap();
    assert_eq!(
        frame.context().context_kind(),
        PerceptionContextKind::EpisodicCandidateV1
    );
    assert_eq!(
        frame.context().values().len(),
        2 * MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE
    );
    assert_eq!(finalized.base_frame_digest(), frame.base_digest());
    assert_eq!(
        finalized.context_digest(),
        frame.context().canonical_digest()
    );
    assert_eq!(finalized.final_frame_digest(), frame.frame_digest());
    assert_eq!(finalized.candidate_keys().len(), frame.candidates().len());
    for (candidate, key) in frame.candidates().iter().zip(finalized.candidate_keys()) {
        key.validate_contract().unwrap();
        key.query()
            .validate_against_frame(&frame, candidate)
            .unwrap();
        assert_eq!(key.retrieval_context_digest(), finalized.context_digest());
        assert_eq!(key.final_frame_digest(), finalized.final_frame_digest());
    }
}

#[test]
fn sealed_neural_decision_accepts_only_its_selected_finalized_memory_key() {
    let bank = empty_bank();
    let draft = grounded_draft(0.4);
    let (frame, finalized) = bank.recall_frame(&draft).unwrap().finalize(draft).unwrap();

    let wrong_candidate =
        neural_decision(&frame, 0).with_finalized_memory_recall(&frame, &finalized, 1);
    assert!(wrong_candidate.is_err());

    let decision = neural_decision(&frame, 0)
        .with_finalized_memory_recall(&frame, &finalized, 0)
        .unwrap();
    let key = decision.episodic_key().unwrap();
    assert_eq!(key.query().candidate_index(), 0);
    assert_eq!(key.query().tracked_object_id(), Some(TrackedObjectId(71)));
    assert_eq!(key.final_frame_digest(), frame.frame_digest());
    let key_digest = key.canonical_digest();

    let patch = ExperiencePatchBuilder::new(sequence())
        .record_pre_action(pre_action(frame))
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome())
        .unwrap()
        .seal()
        .unwrap();
    patch.validate_contract().unwrap();
    assert_eq!(
        patch.decision().episodic_key().unwrap().canonical_digest(),
        key_digest
    );
    let roundtrip: alife_core::ExperiencePatch =
        serde_json::from_value(serde_json::to_value(&patch).unwrap()).unwrap();
    assert_eq!(roundtrip, patch);
}

#[test]
fn decision_deserialization_rejects_evidence_detached_from_episodic_key() {
    let bank = empty_bank();
    let draft = grounded_draft(0.4);
    let (frame, finalized) = bank.recall_frame(&draft).unwrap().finalize(draft).unwrap();
    let decision = neural_decision(&frame, 0)
        .with_finalized_memory_recall(&frame, &finalized, 0)
        .unwrap();
    let mut value = serde_json::to_value(decision).unwrap();
    value["evidence"]["NeuralClosedLoopGpu"]["candidate_index"] = serde_json::json!(1);

    assert!(serde_json::from_value::<DecisionSnapshot>(value).is_err());
}

#[test]
fn poisoned_target_memory_is_candidate_and_target_specific() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&poisoned_cyan_ingest_patch())
        .unwrap();
    let draft = cyan_amber_family_draft();
    let prepared = bank.recall_frame(&draft).unwrap();
    let cyan_ingest = &prepared.context().candidates[0];
    let cyan_avoid = &prepared.context().candidates[1];
    let amber_ingest = &prepared.context().candidates[2];

    assert!(cyan_ingest.family_value[0] < 0.0);
    assert!(cyan_ingest.family_value[2] > 0.0);
    assert_eq!(cyan_avoid.family_value, [0.0; MEMORY_VALUE_V1_COUNT]);
    assert_eq!(cyan_ingest.target_latent, cyan_avoid.target_latent);
    assert!(cyan_avoid.target_latent[2] > 0.0);
    assert_eq!(amber_ingest.family_value, [0.0; MEMORY_VALUE_V1_COUNT]);
    assert_eq!(amber_ingest.target_latent, [0.0; MEMORY_LATENT_V1_COUNT]);
    assert_eq!(bank.len(), 1);
}

#[test]
fn memory_bank_roundtrip_rebuilds_indices_and_preserves_recall() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&poisoned_cyan_ingest_patch())
        .unwrap();
    let probe = cyan_amber_family_draft();
    let before = bank.recall_frame(&probe).unwrap();
    let restored: MemoryBank =
        serde_json::from_value(serde_json::to_value(&bank).unwrap()).unwrap();
    let after = restored.recall_frame(&probe).unwrap();

    assert_eq!(after.context(), before.context());
    assert_eq!(after.receipt(), before.receipt());
}

#[test]
fn legacy_diagnostic_and_candidate_memory_modes_cannot_mix() {
    let patch = poisoned_cyan_ingest_patch();

    let mut candidate_bank = empty_bank();
    candidate_bank.observe_sealed_patch(&patch).unwrap();
    assert_eq!(
        candidate_bank.insert_from_patch(&patch).unwrap_err(),
        ScaffoldContractError::MemoryModeConflict
    );

    let mut legacy_bank = empty_bank();
    legacy_bank.insert_from_patch(&patch).unwrap();
    assert_eq!(
        legacy_bank
            .recall_frame(&cyan_amber_family_draft())
            .unwrap_err(),
        ScaffoldContractError::MemoryModeConflict
    );
    assert_eq!(
        legacy_bank.observe_sealed_patch(&patch).unwrap_err(),
        ScaffoldContractError::MemoryModeConflict
    );
}

#[test]
fn deserialization_rejects_a_mixed_legacy_and_candidate_bank() {
    let patch = poisoned_cyan_ingest_patch();
    let mut candidate_bank = empty_bank();
    candidate_bank.observe_sealed_patch(&patch).unwrap();
    let mut legacy_bank = empty_bank();
    legacy_bank.insert_from_patch(&patch).unwrap();

    let mut mixed = serde_json::to_value(candidate_bank).unwrap();
    let legacy = serde_json::to_value(legacy_bank).unwrap();
    mixed["records"][0] = legacy["records"][0].clone();
    mixed["len"] = serde_json::json!(1_u64);
    mixed["next_write_index"] = serde_json::json!(1_u64);

    assert!(serde_json::from_value::<MemoryBank>(mixed).is_err());
}

#[test]
fn bounded_index_retrieves_the_only_match_after_memory_id_sixty_four() {
    let config = MemoryBankConfig::new(80, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
    let mut sidecar = grounded_sidecar(config);
    for sequence_raw in 1..=80_u64 {
        let (tracked, distance, reward, pain) = if sequence_raw == 65 {
            (71, 0.4, -1.0, 1.0)
        } else {
            (20_000 + sequence_raw, 0.7, 0.1, 0.0)
        };
        sidecar
            .observe_sealed_patch(&sequenced_patch(
                sequence_raw,
                sequence_raw * 2,
                tracked,
                distance,
                reward,
                pain,
            ))
            .unwrap();
    }
    let probe = cyan_amber_family_draft();
    let prepared = sidecar.recall_frame(&probe).unwrap();
    assert_eq!(
        prepared.context().candidates[0].best_family_source,
        Some(alife_core::MemoryId(65))
    );
    assert!(prepared.context().candidates[0].family_confidence.raw() > 0.0);
    assert!(
        prepared.receipt().similarity_evaluations
            <= u32::from(prepared.receipt().candidate_count)
                * alife_core::MEMORY_TOTAL_SEARCH_CAP as u32
    );
}

#[test]
fn million_record_capacity_index_keeps_similarity_work_bounded() {
    let config =
        MemoryBankConfig::new(1_000_000, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
    let mut bank = MemoryBank::new(config).unwrap();
    bank.observe_sealed_patch(&poisoned_cyan_ingest_patch())
        .unwrap();

    // Exercise the production persistence/index rebuild path with a stable ID
    // near the configured million-record ceiling; no test-only lookup path is
    // involved in recall.
    let mut wire = serde_json::to_value(&bank).unwrap();
    let store = wire
        .get_mut("candidate_store")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap();
    let records = store
        .get_mut("records")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap();
    let mut record = records.remove("1").unwrap();
    record["memory_id"] = serde_json::json!(900_001_u64);
    records.insert("900001".to_owned(), record);
    store.insert("next_memory_id".to_owned(), serde_json::json!(900_002_u64));
    let restored: MemoryBank = serde_json::from_value(wire).unwrap();

    let prepared = restored.recall_frame(&cyan_amber_family_draft()).unwrap();
    assert_eq!(
        prepared.context().candidates[0].best_family_source,
        Some(alife_core::MemoryId(900_001))
    );
    assert!(
        prepared.receipt().similarity_evaluations
            <= u32::from(prepared.receipt().candidate_count)
                * alife_core::MEMORY_TOTAL_SEARCH_CAP as u32
    );
}

#[test]
fn compaction_is_prepared_offline_committed_once_and_replay_safe() {
    let config = MemoryBankConfig::new(4, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
    let mut sidecar = grounded_sidecar(config);
    sidecar
        .observe_sealed_patch(&sequenced_patch(1, 2, 71, 0.4, -1.0, 1.0))
        .unwrap();
    let probe = cyan_amber_family_draft();
    let before_prepare = sidecar.recall_frame(&probe).unwrap().receipt().bank_digest;
    let first = sidecar.prepare_compaction(1, 4, 7).unwrap();
    assert_eq!(sidecar.compaction_checkpoint().next_cycle_id, 2);
    assert!(matches!(
        sidecar.compaction_checkpoint().phase,
        MemoryCompactionPhase::Staged { cycle_id: 1, .. }
    ));
    assert_eq!(
        sidecar.prepare_compaction(1, 4, 8).unwrap_err(),
        ScaffoldContractError::MemoryCompactionConflict
    );
    let concurrent_retry = sidecar.prepare_compaction(1, 4, 7).unwrap();
    assert_eq!(
        sidecar.recall_frame(&probe).unwrap().receipt().bank_digest,
        before_prepare
    );
    let receipt = sidecar.commit_compaction(first).unwrap();
    assert_eq!(
        sidecar.commit_compaction(concurrent_retry).unwrap(),
        receipt
    );
    let committed_retry = sidecar.prepare_compaction(1, 4, 7).unwrap();
    assert_eq!(sidecar.commit_compaction(committed_retry).unwrap(), receipt);
    assert_eq!(
        sidecar.prepare_compaction(1, 4, 8).unwrap_err(),
        ScaffoldContractError::MemoryCompactionConflict
    );

    sidecar
        .observe_sealed_patch(&sequenced_patch(2, 4, 72, 0.7, 0.2, 0.0))
        .unwrap();
    assert_eq!(
        sidecar.prepare_compaction(1, 4, 7).unwrap_err(),
        ScaffoldContractError::MemoryCompactionConflict
    );
    let second = sidecar.prepare_compaction(2, 4, 7).unwrap();
    assert_eq!(
        sidecar.commit_compaction(second).unwrap().identity.cycle_id,
        2
    );
}

#[test]
fn duplicate_sealed_sequence_is_rejected_without_mutating_memory() {
    let mut bank = empty_bank();
    let patch = poisoned_cyan_ingest_patch();
    bank.observe_sealed_patch(&patch).unwrap();
    let probe = cyan_amber_family_draft();
    let before = bank.recall_frame(&probe).unwrap().receipt().clone();
    assert_eq!(
        bank.observe_sealed_patch(&patch).unwrap_err(),
        ScaffoldContractError::MemoryReplayRejected
    );
    let after = bank.recall_frame(&probe).unwrap().receipt().clone();
    assert_eq!(after, before);
}

#[test]
fn saturation_merge_evict_and_compaction_are_bounded_and_deterministic() {
    let first = saturation_run();
    let second = saturation_run();
    assert_eq!(first, second);
    assert_eq!(first.1, 4);
    assert!(first.2 > 0);
    assert!(first.3 > 0);
    assert!(first.4 > 0);
}

#[test]
fn portable_memory_assets_roundtrip_private_indices_and_reject_tampering() {
    let config = MemoryBankConfig::new(4, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
    let profile = SensorProfileProvenance::new(
        SensorProfile::GroundedObjectSlotsV1,
        SensoryAbiVersion::CURRENT,
        TICK,
    )
    .unwrap()
    .identity();
    let mut sidecar = MemorySidecarState::new_profiled(ORGANISM, profile, config).unwrap();
    sidecar
        .observe_sealed_patch(&sequenced_patch(1, 2, 71, 0.4, -1.0, 1.0))
        .unwrap();
    let before = sidecar
        .recall_frame(&cyan_amber_family_draft())
        .unwrap()
        .receipt()
        .bank_digest;

    let active = sidecar.export_active_bank().unwrap();
    assert_eq!(active.organism_id_raw, ORGANISM.raw());
    assert_eq!(active.profile, profile);
    assert_eq!(active.records.len(), 1);
    assert_eq!(active.records[0].sealed_sequence_id_raw, 1);
    let restored = MemorySidecarState::restore_portable(
        profile,
        *sidecar.compaction_checkpoint(),
        active.clone(),
        None,
    )
    .unwrap();
    assert_eq!(
        restored
            .recall_frame(&cyan_amber_family_draft())
            .unwrap()
            .receipt()
            .bank_digest,
        before
    );

    let mut tampered = active;
    tampered.records[0].confidence_bits ^= 1;
    assert_eq!(
        MemorySidecarState::restore_portable(
            profile,
            *sidecar.compaction_checkpoint(),
            tampered,
            None,
        )
        .unwrap_err(),
        ScaffoldContractError::InvalidMemoryQuery
    );
}

#[test]
fn portable_memory_restore_finishes_each_crash_phase_exactly_once() {
    let config = MemoryBankConfig::new(4, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
    let profile = SensorProfileProvenance::new(
        SensorProfile::GroundedObjectSlotsV1,
        SensoryAbiVersion::CURRENT,
        TICK,
    )
    .unwrap()
    .identity();
    let mut source = MemorySidecarState::new_profiled(ORGANISM, profile, config).unwrap();
    source
        .observe_sealed_patch(&sequenced_patch(1, 2, 71, 0.4, -1.0, 1.0))
        .unwrap();
    let active = source.export_active_bank().unwrap();

    let pending = alife_core::MemoryCompactionCheckpoint {
        schema_version: alife_core::MEMORY_RECALL_SCHEMA_VERSION,
        organism_id_raw: ORGANISM.raw(),
        active_generation: active.generation,
        active_digest: active.active_bank_digest,
        last_committed_cycle_id: None,
        next_cycle_id: 2,
        phase: MemoryCompactionPhase::Pending {
            cycle_id: 1,
            input_generation: active.generation,
            input_digest: active.active_bank_digest,
            max_records_after: 4,
            policy_version: 7,
        },
    };
    let pending_restored =
        MemorySidecarState::restore_portable(profile, pending, active.clone(), None).unwrap();
    assert!(matches!(
        pending_restored.compaction_checkpoint().phase,
        MemoryCompactionPhase::Committed { cycle_id: 1, .. }
    ));

    source.prepare_compaction(1, 4, 7).unwrap();
    let staged_checkpoint = *source.compaction_checkpoint();
    let staged = source.export_staged_bank().unwrap().unwrap();
    let staged_input_restored = MemorySidecarState::restore_portable(
        profile,
        staged_checkpoint,
        active,
        Some(staged.clone()),
    )
    .unwrap();
    assert_eq!(
        staged_input_restored.compaction_checkpoint(),
        pending_restored.compaction_checkpoint()
    );

    let staged_output_restored = MemorySidecarState::restore_portable(
        profile,
        staged_checkpoint,
        staged.clone(),
        Some(staged),
    )
    .unwrap();
    assert_eq!(
        staged_output_restored.compaction_checkpoint(),
        staged_input_restored.compaction_checkpoint()
    );

    let committed_restored = MemorySidecarState::restore_portable(
        profile,
        *staged_output_restored.compaction_checkpoint(),
        staged_output_restored.export_active_bank().unwrap(),
        None,
    )
    .unwrap();
    assert_eq!(
        committed_restored.compaction_checkpoint(),
        staged_output_restored.compaction_checkpoint()
    );
}
