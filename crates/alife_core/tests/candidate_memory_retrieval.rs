use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, BrainClassSpec, BrainGenome,
    BrainScaleTier, CandidateActionFamily, CandidateObservationRef, Confidence, DecisionSnapshot,
    DevelopmentState, DriveDelta, DriveSnapshot, DurationTicks, EndocrineDelta, EndocrineSnapshot,
    ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId, GroundedObjectSlotV1,
    HomeostaticDelta, HomeostaticSnapshot, LobeKind, MemoryBank, MemoryBankConfig,
    MemoryCompactionPhase, MemoryRecallChannel, MemoryRecallDegradation, MemorySidecarState,
    NeuralActionSelection, NormalizedScalar, OrganismId, PerceptionContextKind, PerceptionFrame,
    PerceptionFrameDraft, PhenotypeHash, PhysicalActionOutcome, PhysicalContactKind, Pose,
    PostActionOutcome, PreActionSnapshot, ScaffoldContractError, SensorProfile,
    SensorProfileProvenance, SensoryAbiVersion, SensoryChannels, SensorySnapshot, SignedValence,
    Tick, TrackedObjectId, Validate, Vec3f, Velocity, WorldEntityId,
    MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE, MEMORY_LATENT_V1_COUNT, MEMORY_VALUE_V1_COUNT,
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
                LobeKind::PerceptualIntegration,
                LobeKind::TemporalPredictive,
                LobeKind::ActionPlanning,
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

#[test]
fn measured_meal_and_blocked_attempt_keep_biological_value_in_memory() {
    let phenotype = alife_core::CreatureGenome::early_mammal_founder(
        321,
        alife_core::FoundationGeneticIdentity::new(
            10,
            1,
            7,
            alife_core::BrainCapacityClass::N512_ID,
        )
        .unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let mut hungry = alife_core::BiochemistryState::new(&phenotype, Tick(37)).unwrap();
    hungry.body.set_energy(0.1).unwrap();
    for tick in 38..=40 {
        hungry = hungry
            .advance(Tick(tick), alife_core::BodyEventDelta::zero(), &phenotype)
            .unwrap();
    }
    for meal in [true, false] {
        let after = hungry
            .advance(
                Tick(41),
                alife_core::BodyEventDelta {
                    nutrition: if meal { 0.6 } else { 0.0 },
                    ..alife_core::BodyEventDelta::zero()
                },
                &phenotype,
            )
            .unwrap();
        let physiology = alife_core::MeasuredPhysiologyTransition::new(hungry, after).unwrap();
        let mut outcome = outcome();
        outcome.success = meal;
        outcome.frustration_delta = NormalizedScalar::new(if meal { 0.0 } else { 1.0 }).unwrap();
        outcome.physical.contact = if meal {
            PhysicalContactKind::Consumed
        } else {
            PhysicalContactKind::None
        };
        let outcome = outcome.with_measured_physiology(physiology).unwrap();
        assert_eq!(outcome.reward_valence.raw(), 0.0);
        let mut bank = empty_bank();
        let draft = grounded_draft(0.4);
        let (frame, finalized) = bank.recall_frame(&draft).unwrap().finalize(draft).unwrap();
        let decision = neural_decision(&frame, 0)
            .with_finalized_memory_recall(&frame, &finalized, 0)
            .unwrap();
        let patch = ExperiencePatchBuilder::new(sequence())
            .record_pre_action(pre_action(frame))
            .unwrap()
            .record_decision(decision)
            .unwrap()
            .record_outcome(outcome)
            .unwrap()
            .seal()
            .unwrap();
        bank.observe_sealed_patch(&patch).unwrap();
        let recall = bank.recall_frame(&grounded_draft(0.4)).unwrap();
        let recalled = &recall.context().candidates[0];
        if meal {
            assert!(
                recalled.target_latent[0] < 0.0,
                "hunger relief stays a negative drive change"
            );
            assert!(
                recalled.family_value[0] > 0.0,
                "the remembered experience is positive"
            );
        } else {
            assert!(recalled.family_value[0] < 0.0);
            assert!(
                recalled.family_value[2] < 0.25,
                "a blocked attempt is disappointment, not injury"
            );
        }
        let restored: MemoryBank =
            serde_json::from_value(serde_json::to_value(&bank).unwrap()).unwrap();
        assert_eq!(
            restored
                .recall_frame(&grounded_draft(0.4))
                .unwrap()
                .context(),
            recall.context()
        );
    }
}

fn sequenced_patch(
    sequence_raw: u64,
    tick_raw: u64,
    tracked_raw: u64,
    distance: f32,
    reward: f32,
    pain: f32,
) -> ExperiencePatch {
    let color = if tracked_raw == 71 {
        [0.0, 0.8, 0.9]
    } else {
        [0.2, 0.5, 0.8]
    };
    let object = slot(0, tracked_raw, distance, color);
    sequenced_patch_for_object(sequence_raw, tick_raw, object, reward, pain)
}

fn sequenced_patch_for_object(
    sequence_raw: u64,
    tick_raw: u64,
    object: GroundedObjectSlotV1,
    reward: f32,
    pain: f32,
) -> ExperiencePatch {
    observation_patch(
        &empty_bank(),
        sequence_raw,
        tick_raw,
        object,
        CandidateActionFamily::Ingest,
        reward,
        pain,
        |_| {},
    )
}

#[allow(clippy::too_many_arguments)]
fn observation_patch(
    recall_bank: &MemoryBank,
    sequence_raw: u64,
    tick_raw: u64,
    object: GroundedObjectSlotV1,
    family: CandidateActionFamily,
    reward: f32,
    pain: f32,
    edit_outcome: impl FnOnce(&mut PostActionOutcome),
) -> ExperiencePatch {
    let tick = Tick::new(tick_raw);
    let candidate = candidate_for_family(&object, 0, family);
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
    observation_from_draft(recall_bank, sequence_raw, draft, reward, pain, edit_outcome)
}

fn observation_from_draft(
    recall_bank: &MemoryBank,
    sequence_raw: u64,
    draft: PerceptionFrameDraft,
    reward: f32,
    pain: f32,
    edit_outcome: impl FnOnce(&mut PostActionOutcome),
) -> ExperiencePatch {
    let tick = draft.tick();
    let tick_raw = tick.raw();
    let sequence = ExperienceSequenceId(sequence_raw);
    let tracked_raw = draft
        .grounded_object_slots()
        .first()
        .map_or(0, |object| object.tracked_object_id.raw());
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
            LobeKind::PerceptualIntegration,
            LobeKind::TemporalPredictive,
            LobeKind::ActionPlanning,
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
    let mut outcome = PostActionOutcome::new(
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
    edit_outcome(&mut outcome);
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
fn populated_recall_keys_match_singular_queries_and_reject_changed_context() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&poisoned_cyan_ingest_patch())
        .unwrap();
    let draft = cyan_amber_family_draft();
    let expected_queries = draft
        .candidates()
        .iter()
        .map(|candidate| {
            alife_core::MemoryQueryEncoderV2::encode_candidate(&draft, candidate).unwrap()
        })
        .collect::<Vec<_>>();
    let prepared = bank.recall_frame(&draft).unwrap();
    assert!(prepared.context().candidates[0].target_source_count > 0);
    let mut cognitive =
        alife_core::CognitiveContextFrame::empty(ORGANISM, sequence(), TICK).unwrap();
    let recalled = &prepared.context().candidates[0];
    cognitive
        .memory
        .expectancies
        .push(alife_core::CognitiveMemoryExpectancy {
            memory_id: recalled.best_family_source.unwrap(),
            expected_valence: SignedValence::new(recalled.family_value[0]).unwrap(),
            confidence: NormalizedScalar::new(recalled.family_confidence.raw()).unwrap(),
        });
    let prepared = prepared.with_cognitive_context(cognitive.clone()).unwrap();
    let (frame, finalized) = prepared.finalize(draft.clone()).unwrap();
    finalized.validate_for_frame(&frame).unwrap();
    assert_eq!(finalized.cognitive_context(), Some(&cognitive));
    for ((key, expected), candidate) in finalized
        .candidate_keys()
        .iter()
        .zip(&expected_queries)
        .zip(frame.candidates())
    {
        assert_eq!(key.query(), expected);
        key.query()
            .validate_against_frame(&frame, candidate)
            .unwrap();
    }

    let changed_draft = draft.with_remembered_novelty(0.75).unwrap();
    let (changed_frame, _) = bank
        .recall_frame(&changed_draft)
        .unwrap()
        .finalize(changed_draft)
        .unwrap();
    assert_eq!(
        finalized.validate_for_frame(&changed_frame),
        Err(ScaffoldContractError::InvalidMemoryQuery)
    );
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
    let profile = probe.profile_provenance().identity();
    assert_eq!(
        bank.object_familiarity(ORGANISM, TrackedObjectId(71), profile),
        0.5
    );
    assert_eq!(
        bank.object_familiarity(ORGANISM, TrackedObjectId(72), profile),
        0.0
    );
    assert_eq!(
        bank.object_familiarity(OrganismId(812), TrackedObjectId(71), profile),
        0.0
    );
    let restored: MemoryBank =
        serde_json::from_value(serde_json::to_value(&bank).unwrap()).unwrap();
    let after = restored.recall_frame(&probe).unwrap();

    assert_eq!(after.context(), before.context());
    assert_eq!(after.receipt(), before.receipt());
    assert_eq!(
        restored.object_familiarity(ORGANISM, TrackedObjectId(71), profile),
        0.5
    );
}

fn stopped_meal_fixture() -> (MemoryBank, PerceptionFrameDraft) {
    // Captured first-meal-selector-19160: the same object stops moving relative
    // to the creature. Candidate lanes 3 and 19 change by one quantization bin.
    let mut moving = slot(0, 112_164_366_696_106, 0.0, [1.0, 0.55, 0.1]);
    moving.bearing = [0.0, 1.0];
    moving.relative_velocity = [-0.125, 0.0, 0.0];
    moving.proprioception = [0.125, 0.0];
    moving.contact = 1.0;
    let patch = sequenced_patch_for_object(1, 1, moving, -0.12, 0.12);
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&patch).unwrap();
    let stopped = GroundedObjectSlotV1 {
        relative_velocity: [0.0; 3],
        proprioception: [0.0; 2],
        ..moving
    };
    let probe_patch = sequenced_patch_for_object(2, 2, stopped, 0.0, 0.0);
    let frame = probe_patch.pre_action().perception();
    let draft = PerceptionFrameDraft::new(
        frame.organism_id(),
        frame.tick(),
        frame.sensor_profile(),
        frame.sensory().clone(),
        frame.body(),
        *frame.homeostasis(),
        frame.candidates().to_vec(),
        frame.profile_provenance(),
        frame.grounded_object_slots().to_vec(),
    )
    .unwrap();
    (bank, draft)
}

#[test]
fn stopped_meal_recall_survives_motion_changes_and_persistence_rebuild() {
    let (bank, draft) = stopped_meal_fixture();
    let bytes = serde_json::to_vec(&bank).unwrap();
    let before = bank.recall_frame(&draft).unwrap();
    let context = &before.context().candidates[0];
    assert_eq!(
        context.target_source_count, 1,
        "stored harmful meal must reach target similarity after stopping"
    );
    assert_eq!(context.family_source_count, 1);
    assert!(context.target_confidence.raw() > 0.72);
    assert!(context.family_confidence.raw() > 0.72);
    assert!(context.target_latent[2] > 0.0);
    assert!(context.family_value[2] > 0.0);
    let receipt = &before.receipt().candidates[0];
    assert_eq!(
        (
            receipt.target_eligible,
            receipt.target_searched,
            receipt.target_matches
        ),
        (1, 1, 1)
    );
    assert_eq!(
        (
            receipt.family_eligible,
            receipt.family_searched,
            receipt.family_matches
        ),
        (1, 1, 1)
    );
    assert_eq!(before.receipt().exact_bucket_reads, 2);
    assert_eq!(before.receipt().neighbor_bucket_reads, 0);
    assert_eq!(
        serde_json::to_vec(&bank).unwrap(),
        bytes,
        "recall is read-only"
    );
    let restored: MemoryBank = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        serde_json::to_vec(&restored).unwrap(),
        bytes,
        "derived indices must not change canonical stored records"
    );
    let after = restored.recall_frame(&draft).unwrap();
    assert_eq!(before.receipt(), after.receipt());
    assert_eq!(before.context(), after.context());
}

#[test]
fn stopped_meal_recall_preserves_identity_and_similarity_boundaries() {
    let (bank, draft) = stopped_meal_fixture();
    let original = serde_json::to_value(&bank).unwrap();
    for (field, value) in [
        ("tracked_object_id_raw", serde_json::json!(99_u64)),
        ("profile_id_raw", serde_json::json!(1_u16)),
        ("profile_schema_version", serde_json::json!(2_u16)),
        ("sensory_abi_version_raw", serde_json::json!(2_u16)),
    ] {
        let mut wire = original.clone();
        wire["candidate_store"]["records"]["1"][field] = value;
        let foreign: MemoryBank = serde_json::from_value(wire).unwrap();
        let recall = foreign.recall_frame(&draft).unwrap();
        let expected = u16::from(field == "tracked_object_id_raw");
        assert_eq!(
            recall.context().candidates[0].target_source_count,
            expected,
            "{field}"
        );
        assert_eq!(
            recall.context().candidates[0].family_source_count,
            expected,
            "{field}"
        );
        if expected != 0 {
            assert!(recall.context().candidates[0].family_confidence.raw() < 0.8);
        }
    }
    let mut wire = original.clone();
    wire["candidate_store"]["records"]["1"]["organism_id_raw"] = serde_json::json!(812_u64);
    wire["candidate_store"]["last_sequence_by_organism"] = serde_json::json!({"812":1});
    let foreign: MemoryBank = serde_json::from_value(wire).unwrap();
    assert_eq!(
        foreign.recall_frame(&draft).unwrap().context().candidates[0].target_source_count,
        0
    );
    let mut wire = original.clone();
    wire["candidate_store"]["records"]["1"]["query_version_raw"] = serde_json::json!(1_u16);
    assert!(serde_json::from_value::<MemoryBank>(wire).is_err());
    let mut wire = original.clone();
    wire["candidate_store"]["records"]["1"]["family_raw"] = serde_json::json!(4_u16);
    wire["candidate_store"]["records"]["1"]["action_kind_raw"] =
        serde_json::json!(ActionKind::Move.raw());
    let other_family: MemoryBank = serde_json::from_value(wire).unwrap();
    let recall = other_family.recall_frame(&draft).unwrap();
    assert!(
        recall.context().candidates[0].target_source_count > 0,
        "target evidence can cross action families"
    );
    assert_eq!(recall.context().candidates[0].family_source_count, 0);
    let mut wire = original;
    let features = wire["candidate_store"]["records"]["1"]["query_features"]
        .as_array_mut()
        .unwrap();
    for feature in &mut features[alife_core::MEMORY_TARGET_RANGE] {
        *feature = serde_json::json!(0.0);
    }
    features[alife_core::MEMORY_TARGET_RANGE.start + 1] = serde_json::json!(-1.0);
    let dissimilar: MemoryBank = serde_json::from_value(wire).unwrap();
    let recall = dissimilar.recall_frame(&draft).unwrap();
    assert_eq!(
        recall.receipt().candidates[0].target_searched,
        1,
        "same-object record must reach similarity"
    );
    assert_eq!(recall.receipt().candidates[0].family_searched, 1);
    assert_eq!(recall.context().candidates[0].target_source_count, 0);
    assert_eq!(recall.context().candidates[0].family_source_count, 0);
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
fn bounded_index_reports_eligible_records_above_the_similarity_search_cap() {
    let config = MemoryBankConfig::new(80, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
    let mut bank = MemoryBank::new(config).unwrap();
    bank.observe_sealed_patch(&poisoned_cyan_ingest_patch())
        .unwrap();

    let mut wire = serde_json::to_value(&bank).unwrap();
    let store = wire
        .get_mut("candidate_store")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap();
    let records = store
        .get_mut("records")
        .and_then(serde_json::Value::as_object_mut)
        .unwrap();
    let template = records.get("1").cloned().unwrap();
    for memory_id in 2..=65_u64 {
        let mut record = template.clone();
        record["memory_id"] = serde_json::json!(memory_id);
        records.insert(memory_id.to_string(), record);
    }
    store.insert("next_memory_id".to_owned(), serde_json::json!(66_u64));
    let restored: MemoryBank = serde_json::from_value(wire).unwrap();

    let prepared = restored.recall_frame(&cyan_amber_family_draft()).unwrap();
    let receipt = prepared.receipt();
    assert_eq!(receipt.candidates[0].family_eligible, 65);
    assert_eq!(receipt.candidates[0].family_searched, 64);
    assert!(receipt
        .degradations
        .contains(&MemoryRecallDegradation::SearchShortlisted {
            candidate_index: 0,
            channel: MemoryRecallChannel::FamilyValue,
            eligible: 65,
            searched: 64,
        }));
    assert!(
        receipt.similarity_evaluations
            <= u32::from(receipt.candidate_count) * alife_core::MEMORY_TOTAL_SEARCH_CAP as u32
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
fn consecutive_sleep_compactions_do_not_require_an_intervening_memory_write() {
    let config = MemoryBankConfig::new(4, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
    let mut sidecar = grounded_sidecar(config);
    sidecar
        .observe_sealed_patch(&sequenced_patch(1, 2, 71, 0.4, -1.0, 1.0))
        .unwrap();

    let first = sidecar.prepare_compaction(1, 4, 7).unwrap();
    sidecar.commit_compaction(first).unwrap();
    let second = sidecar.prepare_compaction(2, 4, 7).unwrap();
    let receipt = sidecar.commit_compaction(second).unwrap();

    assert_eq!(receipt.identity.cycle_id, 2);
    assert_eq!(sidecar.compaction_checkpoint().next_cycle_id, 3);
    assert_eq!(
        sidecar.compaction_checkpoint().last_committed_cycle_id,
        Some(2)
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
    let mut object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    // A valid physical reading can carry signed zero from ordinary arithmetic.
    object.relative_velocity[0] = -0.0;
    sidecar
        .observe_sealed_patch(&sequenced_patch_for_object(1, 2, object, -1.0, 1.0))
        .unwrap();
    let before = sidecar
        .recall_frame(&cyan_amber_family_draft())
        .unwrap()
        .receipt()
        .bank_digest;

    let active = sidecar.export_active_bank().unwrap();
    // Relative velocity occupies target feature lane 3 (query lane 60).
    assert_eq!(active.records[0].query_feature_bits[60], 0);
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

// A developer-only CPU measurement of the real recall API. This does not run
// neurons or training and cannot establish game FPS or accelerator performance.
#[test]
#[ignore = "CPU memory microbenchmark; run explicitly with --ignored --nocapture"]
fn memory_cpu_recall_microbenchmark() {
    for distinct in [false, true] {
        let config =
            MemoryBankConfig::new(256, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
        let mut bank = MemoryBank::new(config).unwrap();
        bank.observe_sealed_patch(&poisoned_cyan_ingest_patch())
            .unwrap();
        let mut wire = serde_json::to_value(&bank).unwrap();
        let template = wire["candidate_store"]["records"]["1"].clone();
        for id in 1..=256_u64 {
            let mut record = template.clone();
            record["memory_id"] = serde_json::json!(id);
            if distinct {
                record["tracked_object_id_raw"] = serde_json::json!(71 + (id - 1) % 16);
            }
            wire["candidate_store"]["records"][id.to_string()] = record;
        }
        wire["candidate_store"]["next_memory_id"] = serde_json::json!(257_u64);
        let bank: MemoryBank = serde_json::from_value(wire).unwrap();
        let source = grounded_draft(0.4);
        let original = source.grounded_object_slots()[0];
        let objects = if distinct {
            (0..16)
                .map(|index| GroundedObjectSlotV1 {
                    slot_index: index,
                    tracked_object_id: TrackedObjectId(71 + u64::from(index)),
                    ..original
                })
                .collect::<Vec<_>>()
        } else {
            vec![original]
        };
        let candidate_count = if distinct { 16 } else { 32 };
        let candidates = (0..candidate_count)
            .map(|index| candidate(&objects[if distinct { index as usize } else { 0 }], index))
            .collect();
        let draft = PerceptionFrameDraft::new(
            ORGANISM,
            TICK,
            source.sensor_profile(),
            source.sensory().clone(),
            source.body(),
            *source.homeostasis(),
            candidates,
            source.profile_provenance(),
            objects,
        )
        .unwrap();
        let mut samples = Vec::new();
        for _ in 0..5 {
            let started = std::time::Instant::now();
            for _ in 0..100 {
                std::hint::black_box(bank.recall_frame(std::hint::black_box(&draft)).unwrap());
            }
            samples.push(started.elapsed().as_micros());
        }
        samples.sort();
        let receipt = bank.recall_frame(&draft).unwrap().receipt().clone();
        eprintln!("memory CPU recall: 256 episodes, {candidate_count} {} candidates; median {} us/frame; samples_us={samples:?}; similarity_evaluations={}",
            if distinct { "distinct" } else { "repeated" }, samples[2] / 100, receipt.similarity_evaluations);
    }
}

#[test]
fn repeated_candidates_reuse_evidence_without_changing_candidate_keys() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&poisoned_cyan_ingest_patch())
        .unwrap();
    let source = grounded_draft(0.4);
    let object = source.grounded_object_slots()[0];
    let draft = PerceptionFrameDraft::new(
        ORGANISM,
        TICK,
        source.sensor_profile(),
        source.sensory().clone(),
        source.body(),
        *source.homeostasis(),
        vec![
            candidate(&object, 0),
            candidate(&object, 1),
            candidate_for_family(&object, 2, CandidateActionFamily::Avoid),
        ],
        source.profile_provenance(),
        source.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let prepared = bank.recall_frame(&draft).unwrap();
    assert_eq!(prepared.receipt().similarity_evaluations, 2);
    assert_eq!(prepared.receipt().exact_bucket_reads, 3);
    assert_eq!(prepared.receipt().candidates[1].target_reused_from, Some(0));
    assert_eq!(prepared.receipt().candidates[1].family_reused_from, Some(0));
    assert_eq!(prepared.receipt().candidates[2].target_reused_from, Some(0));
    assert_eq!(prepared.receipt().candidates[2].family_reused_from, None);
    assert_eq!(
        prepared.context().candidates[0].family_value,
        prepared.context().candidates[1].family_value
    );
    assert_eq!(
        prepared.context().candidates[0].target_latent,
        prepared.context().candidates[2].target_latent
    );
    let (frame, finalized) = prepared.finalize(draft).unwrap();
    assert_ne!(
        finalized.candidate_keys()[0].canonical_digest(),
        finalized.candidate_keys()[1].canonical_digest()
    );
    finalized.validate_for_frame(&frame).unwrap();
}

#[test]
fn boring_ticks_do_not_fill_or_refresh_memory_and_remain_replay_guarded() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, -0.8, 0.9))
        .unwrap();
    let retained = serde_json::to_value(&bank).unwrap()["candidate_store"]["records"].clone();
    let mut last = None;
    for sequence in 2..=300 {
        let patch = observation_patch(
            &bank,
            sequence,
            sequence * 2,
            slot(0, 10_000 + sequence, 0.4, [0.0, 0.8, 0.9]),
            CandidateActionFamily::Approach,
            0.0,
            0.0,
            |outcome| {
                // Actual world routine movement profile, not a perfectly zero
                // synthetic tick: movement cost alone is not an event.
                outcome.physical.contact = PhysicalContactKind::Moved;
                outcome.physical.energy_cost = NormalizedScalar::new(0.08).unwrap();
                outcome.homeostatic_delta = HomeostaticDelta {
                    drives: DriveDelta {
                        brain_atp: -0.04,
                        curiosity: 0.01,
                        ..DriveDelta::zero()
                    },
                    hormones: EndocrineDelta::zero(),
                };
                outcome.energy_delta = SignedValence::new(-0.04).unwrap();
                outcome.prediction_error = NormalizedScalar::new(0.08).unwrap();
            },
        );
        let receipt = bank.observe_sealed_patch(&patch).unwrap();
        assert_eq!(
            receipt.kind,
            alife_core::MemoryUpdateKind::IgnoredLowInformation
        );
        last = Some(patch);
    }
    assert_eq!(bank.len(), 1);
    assert_eq!(
        serde_json::to_value(&bank).unwrap()["candidate_store"]["records"],
        retained
    );
    let snapshot = serde_json::to_vec(&bank).unwrap();
    assert_eq!(
        bank.observe_sealed_patch(&last.unwrap()).unwrap_err(),
        ScaffoldContractError::MemoryReplayRejected
    );
    assert_eq!(serde_json::to_vec(&bank).unwrap(), snapshot);
    let restored: MemoryBank = serde_json::from_slice(&snapshot).unwrap();
    assert_eq!(
        restored
            .recall_frame(&cyan_amber_family_draft())
            .unwrap()
            .context(),
        bank.recall_frame(&cyan_amber_family_draft())
            .unwrap()
            .context()
    );
}

#[test]
fn contradictory_outcomes_keep_distinct_episodes_through_compaction() {
    let config = MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap();
    let mut sidecar = grounded_sidecar(config);
    sidecar
        .observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, 0.8, 0.0))
        .unwrap();
    sidecar
        .observe_sealed_patch(&sequenced_patch(2, 2, 71, 0.4, -0.8, 0.9))
        .unwrap();
    assert_eq!(
        sidecar.bank().len(),
        2,
        "matching cues do not merge opposite outcomes"
    );
    let probe = probe_for_object(3, slot(0, 71, 0.4, [0.0, 0.8, 0.9]));
    let recall = sidecar.recall_frame(&probe).unwrap();
    assert!(recall.context().candidates[0].family_value[0] < -0.7);
    assert!(recall.context().candidates[0].family_value[2] > 0.8);
    assert_eq!(recall.context().candidates[0].family_source_count, 1);
    let prepared = sidecar.prepare_compaction(1, 8, 1).unwrap();
    sidecar.commit_compaction(prepared).unwrap();
    assert_eq!(sidecar.bank().len(), 2);
    let records =
        serde_json::to_value(sidecar.bank()).unwrap()["candidate_store"]["records"].clone();
    assert!(records
        .as_object()
        .unwrap()
        .values()
        .any(|r| r["family_value"][0].as_f64().unwrap() < 0.0));
    assert!(records
        .as_object()
        .unwrap()
        .values()
        .any(|r| r["family_value"][0].as_f64().unwrap() > 0.0));
    sidecar
        .observe_sealed_patch(&sequenced_patch(3, 4, 71, 0.4, 0.8, 0.0))
        .unwrap();
    let reversed = sidecar
        .recall_frame(&probe_for_object(5, slot(0, 71, 0.4, [0.0, 0.8, 0.9])))
        .unwrap();
    assert!(
        reversed.context().candidates[0].family_value[0] > 0.7,
        "new direct evidence updates expectation while retaining the harm episode"
    );
    assert_eq!(sidecar.bank().len(), 2);
    let compression = sidecar.prepare_compaction(2, 1, 1).unwrap();
    sidecar.commit_compaction(compression).unwrap();
    assert_eq!(sidecar.bank().len(), 1);
    assert!(
        sidecar
            .recall_frame(&probe_for_object(5, slot(0, 71, 0.4, [0.0, 0.8, 0.9])))
            .unwrap()
            .context()
            .candidates[0]
            .family_value[0]
            > 0.7,
        "compression cannot resurrect the obsolete poison expectation"
    );
}

fn probe_for_object(tick: u64, object: GroundedObjectSlotV1) -> PerceptionFrameDraft {
    let patch = sequenced_patch_for_object(10_000_000 + tick, tick, object, 0.0, 0.0);
    let frame = patch.pre_action().perception();
    PerceptionFrameDraft::new(
        ORGANISM,
        frame.tick(),
        frame.sensor_profile(),
        frame.sensory().clone(),
        frame.body(),
        *frame.homeostasis(),
        frame.candidates().to_vec(),
        frame.profile_provenance(),
        frame.grounded_object_slots().to_vec(),
    )
    .unwrap()
}

#[test]
fn perceptual_category_fallback_preserves_individual_poison_and_does_not_equate_fruit() {
    let mut bank = empty_bank();
    for sequence in 1..=3 {
        bank.observe_sealed_patch(&sequenced_patch_for_object(
            sequence,
            sequence,
            slot(0, 100 + sequence, 0.4, [0.0, 0.8, 0.9]),
            0.8,
            0.0,
        ))
        .unwrap();
    }
    let novel = slot(0, 200, 0.4, [0.0, 0.8, 0.9]);
    let recall = bank.recall_frame(&probe_for_object(4, novel)).unwrap();
    assert!(recall.context().candidates[0].family_value[0] > 0.7);
    assert!(recall.context().candidates[0].family_confidence.raw() < 0.8);
    assert_eq!(
        bank.object_familiarity(
            ORGANISM,
            novel.tracked_object_id,
            probe_for_object(4, novel).profile_provenance().identity()
        ),
        0.0,
        "borrowing category evidence cannot create personal familiarity"
    );
    let poison = slot(0, 201, 0.4, [0.0, 0.8, 0.9]);
    bank.observe_sealed_patch(&observation_patch(
        &bank,
        4,
        4,
        poison,
        CandidateActionFamily::Ingest,
        -0.8,
        0.9,
        |_| {},
    ))
    .unwrap();
    let exception = bank.recall_frame(&probe_for_object(5, poison)).unwrap();
    assert!(exception.context().candidates[0].family_value[0] < -0.7);
    assert!(exception.context().candidates[0].family_value[2] > 0.8);
    assert_eq!(exception.context().candidates[0].family_source_count, 1);
    assert_eq!(exception.receipt().candidates[0].family_searched, 1);
    for unrelated in [
        slot(0, 202, 0.4, [0.9, 0.6, 0.1]),
        GroundedObjectSlotV1 {
            chemical: [-0.8, 0.5, 0.9],
            ..novel
        },
        GroundedObjectSlotV1 {
            shape: [0.9, 0.1, 0.1],
            ..novel
        },
    ] {
        let recalled = bank.recall_frame(&probe_for_object(5, unrelated)).unwrap();
        assert_eq!(recalled.context().candidates[0].family_source_count, 0);
        assert_eq!(recalled.context().candidates[0].target_source_count, 0);
    }
    let bytes = serde_json::to_vec(&bank).unwrap();
    let restored: MemoryBank = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        restored
            .recall_frame(&probe_for_object(5, poison))
            .unwrap()
            .context(),
        exception.context()
    );
}

#[test]
fn unused_low_value_evidence_fades_but_strong_danger_and_reward_survive() {
    let mut bank = empty_bank();
    for (sequence, tracked, reward, pain) in
        [(1, 71, 0.1, 0.0), (2, 72, -0.8, 0.9), (3, 73, 0.8, 0.0)]
    {
        bank.observe_sealed_patch(&sequenced_patch(sequence, 1, tracked, 0.4, reward, pain))
            .unwrap();
    }
    for tracked in [71, 72, 73] {
        let color = if tracked == 71 {
            [0.0, 0.8, 0.9]
        } else {
            [0.2, 0.5, 0.8]
        };
        let object = slot(0, tracked, 0.4, color);
        let recent = bank.recall_frame(&probe_for_object(2, object)).unwrap();
        let old = bank
            .recall_frame(&probe_for_object(65_537, object))
            .unwrap();
        let recent_confidence = recent.context().candidates[0].family_confidence.raw();
        let old_confidence = old.context().candidates[0].family_confidence.raw();
        if tracked == 71 {
            assert!(old_confidence < recent_confidence * 0.25);
        } else {
            assert_eq!(old_confidence, recent_confidence);
        }
        assert_eq!(
            old.context().candidates[0].family_value,
            recent.context().candidates[0].family_value
        );
    }
}

#[test]
fn corroborated_retrieval_refreshes_retention_without_amplifying_belief() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, 0.1, 0.0))
        .unwrap();
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let before = serde_json::to_value(&bank).unwrap()["candidate_store"]["records"]["1"].clone();
    let old = bank
        .recall_frame(&probe_for_object(65_537, object))
        .unwrap();
    let bytes = serde_json::to_vec(&bank).unwrap();
    for _ in 0..20 {
        bank.recall_frame(&probe_for_object(65_537, object))
            .unwrap();
    }
    assert_eq!(
        serde_json::to_vec(&bank).unwrap(),
        bytes,
        "recalling alone cannot strengthen evidence"
    );
    let patch = observation_patch(
        &bank,
        2,
        65_537,
        object,
        CandidateActionFamily::Ingest,
        0.1,
        0.0,
        |_| {},
    );
    bank.observe_sealed_patch(&patch).unwrap();
    let after = serde_json::to_value(&bank).unwrap()["candidate_store"]["records"]["1"].clone();
    assert!(after["salience_q16"].as_u64().unwrap() > before["salience_q16"].as_u64().unwrap());
    assert_eq!(after["confidence"], before["confidence"]);
    assert_eq!(after["family_value"], before["family_value"]);
    let refreshed = bank
        .recall_frame(&probe_for_object(65_538, object))
        .unwrap();
    assert!(
        refreshed.context().candidates[0].family_confidence.raw()
            > old.context().candidates[0].family_confidence.raw() * 3.0
    );
}

#[test]
fn low_value_events_cannot_displace_a_bank_of_strong_outcomes() {
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(2, 64, 2, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    bank.observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, -0.8, 0.9))
        .unwrap();
    bank.observe_sealed_patch(&sequenced_patch(2, 2, 72, 0.4, 0.8, 0.0))
        .unwrap();
    let retained = serde_json::to_value(&bank).unwrap()["candidate_store"]["records"].clone();
    for sequence in 3..=100 {
        let receipt = bank
            .observe_sealed_patch(&sequenced_patch(
                sequence,
                sequence * 10_000,
                10_000 + sequence,
                0.4,
                0.1,
                0.0,
            ))
            .unwrap();
        assert_eq!(
            receipt.kind,
            alife_core::MemoryUpdateKind::IgnoredLowerRetention
        );
    }
    assert_eq!(bank.len(), 2);
    assert_eq!(
        serde_json::to_value(&bank).unwrap()["candidate_store"]["records"],
        retained
    );
}

#[test]
fn later_approach_does_not_erase_ingest_target_evidence() {
    let mut bank = empty_bank();
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    bank.observe_sealed_patch(&sequenced_patch_for_object(1, 1, object, -0.8, 0.9))
        .unwrap();
    bank.observe_sealed_patch(&observation_patch(
        &bank,
        2,
        2,
        object,
        CandidateActionFamily::Approach,
        0.8,
        0.0,
        |_| {},
    ))
    .unwrap();
    let recall = bank.recall_frame(&probe_for_object(3, object)).unwrap();
    assert!(recall.context().candidates[0].target_latent[2] > 0.0);
    assert_eq!(recall.context().candidates[0].target_source_count, 2);
    assert!(recall.context().candidates[0].family_value[0] < -0.7);
}

fn zero_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    let mut wire = serde_json::to_value(HomeostaticSnapshot::baseline(tick)).unwrap();
    for name in ["drives", "hormones"] {
        for value in wire[name].as_object_mut().unwrap().values_mut() {
            if let Some(values) = value.as_array_mut() {
                for lane in values {
                    *lane = serde_json::json!(0.0);
                }
            } else {
                *value = serde_json::json!(0.0);
            }
        }
    }
    serde_json::from_value(wire).unwrap()
}

#[test]
fn changed_context_cannot_replace_a_known_individual_exception_with_category_reward() {
    let mut bank = empty_bank();
    let poison = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    bank.observe_sealed_patch(&sequenced_patch_for_object(1, 1, poison, -0.8, 0.9))
        .unwrap();
    let source = probe_for_object(2, poison);
    let mut selected = source.candidates()[0];
    selected.sensor_confidence = Confidence::new(0.0).unwrap();
    let shifted = PerceptionFrameDraft::new(
        ORGANISM,
        source.tick(),
        source.sensor_profile(),
        source.sensory().clone(),
        source.body(),
        zero_homeostasis(source.tick()),
        vec![selected],
        source.profile_provenance(),
        source.grounded_object_slots().to_vec(),
    )
    .unwrap();
    // A different individual has good evidence in exactly this shifted state.
    let mut good = poison;
    good.tracked_object_id = TrackedObjectId(72);
    let mut candidate = candidate(&good, 0);
    candidate.sensor_confidence = Confidence::new(0.0).unwrap();
    let good_draft = PerceptionFrameDraft::new(
        ORGANISM,
        source.tick(),
        source.sensor_profile(),
        source.sensory().clone(),
        source.body(),
        zero_homeostasis(source.tick()),
        vec![candidate],
        source.profile_provenance(),
        vec![good],
    )
    .unwrap();
    bank.observe_sealed_patch(&observation_from_draft(
        &bank,
        2,
        good_draft,
        0.8,
        0.0,
        |_| {},
    ))
    .unwrap();
    let recall = bank.recall_frame(&shifted).unwrap();
    assert_eq!(recall.context().candidates[0].family_source_count, 0);
    assert_eq!(
        recall.context().candidates[0].family_value,
        [0.0; MEMORY_VALUE_V1_COUNT]
    );
    assert_eq!(
        recall.receipt().candidates[0].family_searched,
        1,
        "known individual's namespace is checked without category scans"
    );
}

#[test]
fn repeated_untracked_look_is_not_perpetually_novel() {
    let mut bank = empty_bank();
    for sequence in 1..=20 {
        let tick = Tick::new(sequence);
        let mut channels = SensoryChannels::ZERO;
        channels.novelty_signal = NormalizedScalar::new(0.7).unwrap();
        let mut features = alife_core::CandidateFeatureVector::zero();
        features.0[18] = 1.0;
        let candidate = ActionCandidate::new(
            0,
            ActionId(200),
            ActionKind::Look,
            CandidateActionFamily::Inspect,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            features,
            Confidence::new(0.9).unwrap(),
            NormalizedScalar::new(0.0).unwrap(),
            DurationTicks::new(1),
            DurationTicks::new(2),
        )
        .unwrap();
        let draft = PerceptionFrameDraft::new(
            ORGANISM,
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            SensorySnapshot::new(ORGANISM, tick, Vec3f::ZERO, channels, Default::default())
                .unwrap(),
            BodySnapshot {
                pose: Pose::IDENTITY,
                velocity: Velocity::ZERO,
            },
            HomeostaticSnapshot::baseline(tick),
            vec![candidate],
            SensorProfileProvenance::new(
                SensorProfile::GroundedTerrainVisionV1,
                SensoryAbiVersion::CURRENT,
                tick,
            )
            .unwrap(),
            vec![],
        )
        .unwrap();
        let patch = observation_from_draft(&bank, sequence, draft, 0.0, 0.0, |outcome| {
            outcome.physical.contact = PhysicalContactKind::None;
            outcome.homeostatic_delta = HomeostaticDelta::zero();
            outcome.energy_delta = SignedValence::new(0.0).unwrap();
            outcome.prediction_error = NormalizedScalar::new(0.0).unwrap();
        });
        let receipt = bank.observe_sealed_patch(&patch).unwrap();
        if sequence > 1 {
            assert_eq!(
                receipt.kind,
                alife_core::MemoryUpdateKind::IgnoredLowInformation
            );
        }
    }
    assert_eq!(bank.len(), 1);
    assert_eq!(
        serde_json::to_value(&bank).unwrap()["candidate_store"]["records"]["1"]
            ["observation_count"],
        1
    );
}

#[test]
fn saturated_bank_accepts_a_direct_reversal_without_displacing_unrelated_harm() {
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(2, 64, 2, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    bank.observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, -0.9, 0.9))
        .unwrap();
    bank.observe_sealed_patch(&sequenced_patch(2, 2, 72, 0.4, -0.9, 0.9))
        .unwrap();
    bank.observe_sealed_patch(&sequenced_patch(3, 3, 71, 0.4, 0.8, 0.0))
        .unwrap();
    let reversal = bank
        .recall_frame(&probe_for_object(4, slot(0, 71, 0.4, [0.0, 0.8, 0.9])))
        .unwrap();
    assert!(reversal.context().candidates[0].family_value[0] > 0.7);
    let unrelated = bank
        .recall_frame(&probe_for_object(4, slot(0, 72, 0.4, [0.2, 0.5, 0.8])))
        .unwrap();
    assert!(unrelated.context().candidates[0].family_value[0] < -0.8);
    assert_eq!(bank.len(), 2);
}

#[test]
fn same_tick_outcome_order_uses_causal_sequence_after_merging() {
    let mut bank = empty_bank();
    bank.observe_sealed_patch(&sequenced_patch(1, 1, 71, 0.4, 0.8, 0.0))
        .unwrap();
    bank.observe_sealed_patch(&sequenced_patch(2, 2, 71, 0.4, -0.8, 0.9))
        .unwrap();
    bank.observe_sealed_patch(&sequenced_patch(3, 2, 71, 0.4, 0.8, 0.0))
        .unwrap();
    let recall = bank
        .recall_frame(&probe_for_object(3, slot(0, 71, 0.4, [0.0, 0.8, 0.9])))
        .unwrap();
    assert!(recall.context().candidates[0].family_value[0] > 0.7);
    assert_eq!(
        recall.context().candidates[0].best_family_source,
        Some(alife_core::MemoryId(1))
    );
}
