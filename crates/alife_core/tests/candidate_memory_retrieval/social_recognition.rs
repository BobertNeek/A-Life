use super::*;
use alife_core::{
    AttentionFrame, BiochemistryState, BodyEventDelta, BrainCapacityClass, CandidateFeatureVector,
    CognitiveContextFrame, CreatureGenome, FoundationGeneticIdentity, NeuralEmission,
    NeuralEmissionClass, NeuralEmissionFrame, SocialAgentSnapshot, StableFocusIdentity,
};

fn social_draft(
    objects: Vec<GroundedObjectSlotV1>,
    observed: bool,
    tick: Tick,
) -> PerceptionFrameDraft {
    let mut sensory = SensorySnapshot::new(
        ORGANISM,
        tick,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    let mut candidates = vec![ActionCandidate::new(
        0,
        ActionKind::Idle.canonical_id(),
        ActionKind::Idle,
        CandidateActionFamily::Idle,
        CandidateObservationRef::None,
        ActionTarget::new(None, None),
        CandidateFeatureVector([0.0; 24]),
        Confidence::new(0.8).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(1),
    )
    .unwrap()];
    for (index, object) in objects.iter().enumerate() {
        let mut candidate =
            candidate_for_family(object, index as u16 + 1, CandidateActionFamily::Approach);
        candidate.target.entity = Some(WorldEntityId(9000 + index as u64));
        if observed {
            sensory.social_context.nearest_agents[index] = Some(SocialAgentSnapshot {
                agent_id: OrganismId(1000 + index as u64),
                body_entity: candidate.target.entity,
                relative_position: Vec3f::new(object.distance, 0.0, 0.0),
                gaze_direction: Vec3f::ZERO,
                orientation_forward: Vec3f::new(0.0, 1.0, 0.0),
                affinity: SignedValence::ZERO,
                proximity: NormalizedScalar::new(0.9).unwrap(),
            });
        }
        candidates.push(candidate);
    }
    PerceptionFrameDraft::new(
        ORGANISM,
        tick,
        SensorProfile::GroundedObjectSlotsV1,
        sensory,
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
        objects,
    )
    .unwrap()
}

fn finalize_social(
    bank: &MemoryBank,
    draft: PerceptionFrameDraft,
    attended: &[u64],
) -> (PerceptionFrame, alife_core::FinalizedMemoryRecall) {
    let mut attention = AttentionFrame::empty(ORGANISM, sequence(), draft.tick()).unwrap();
    attention.focal_targets = attended
        .iter()
        .map(|id| StableFocusIdentity::TrackedObject(TrackedObjectId(*id)))
        .collect();
    attention.budget_receipt.requested_focal_count = attended.len() as u8;
    attention.budget_receipt.granted_focal_count = attended.len() as u8;
    let mut context = CognitiveContextFrame::empty(ORGANISM, sequence(), draft.tick()).unwrap();
    context.focal.identities = attention.focal_targets.clone();
    context.budget.focal_capacity = attention.budget_receipt.focal_capacity;
    context.attention = attention;
    context.validate_contract().unwrap();
    bank.recall_frame(&draft)
        .unwrap()
        .with_cognitive_context(context)
        .unwrap()
        .finalize(draft)
        .unwrap()
}

fn remember(
    bank: &mut MemoryBank,
    seq: u64,
    object: GroundedObjectSlotV1,
    family: CandidateActionFamily,
    value: f32,
) {
    let patch = observation_patch(
        bank,
        seq,
        TICK.raw(),
        object,
        family,
        value,
        0.0,
        |outcome| {
            outcome.physical.contact = PhysicalContactKind::None;
        },
    );
    bank.observe_sealed_patch(&patch).unwrap();
}

fn recognize(
    bank: &MemoryBank,
    objects: Vec<GroundedObjectSlotV1>,
    observed: bool,
    attended: &[u64],
) -> Option<NeuralEmission> {
    finalize_social(bank, social_draft(objects, observed, TICK), attended)
        .1
        .recognized_social_emission()
}

#[test]
fn recognition_of_liked_individual_survives_an_idle_action_winner() {
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let mut bank = empty_bank();
    remember(&mut bank, 1, object, CandidateActionFamily::Contact, 0.8);
    let (frame, recall) = finalize_social(&bank, social_draft(vec![object], true, TICK), &[71]);
    assert_eq!(
        neural_decision(&frame, 0).selected_action.kind,
        ActionKind::Idle
    );
    let emission = recall.recognized_social_emission().unwrap();
    assert_eq!(emission.class, NeuralEmissionClass::SocialState);
    assert!((emission.activity - 0.8).abs() < 1e-6);
    assert!(emission.confidence > 0.0);
    assert_eq!(recall.receipt().candidates[1].target_searched, 1);
}

#[test]
fn unknown_negative_avoidance_and_ingestion_evidence_do_not_create_affection() {
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    assert!(recognize(&empty_bank(), vec![object], true, &[71]).is_none());
    for (family, value) in [
        (CandidateActionFamily::Contact, -0.8),
        (CandidateActionFamily::Avoid, 0.8),
        (CandidateActionFamily::Ingest, 0.8),
    ] {
        let mut bank = empty_bank();
        remember(&mut bank, 1, object, family, value);
        assert!(recognize(&bank, vec![object], true, &[71]).is_none());
    }
}

#[test]
fn stranger_category_borrowing_and_static_affinity_do_not_create_a_bond() {
    let known = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let stranger = GroundedObjectSlotV1 {
        tracked_object_id: TrackedObjectId(99),
        ..known
    };
    let mut bank = empty_bank();
    remember(&mut bank, 1, known, CandidateActionFamily::Contact, 0.8);
    let mut draft = social_draft(vec![stranger], true, TICK);
    // The existing social label can be positive without personal experience.
    let mut sensory = draft.sensory().clone();
    sensory.social_context.nearest_agents[0]
        .as_mut()
        .unwrap()
        .affinity = SignedValence::new(1.0).unwrap();
    draft = PerceptionFrameDraft::new(
        ORGANISM,
        TICK,
        draft.sensor_profile(),
        sensory,
        draft.body(),
        *draft.homeostasis(),
        draft.candidates().to_vec(),
        draft.profile_provenance(),
        draft.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let (_, recall) = finalize_social(&bank, draft, &[99]);
    assert!(
        recall.context().candidates[1].target_source_count > 0,
        "fixture must borrow category evidence"
    );
    assert!(recall.recognized_social_emission().is_none());
}

#[test]
fn signed_social_history_is_combined_before_the_positive_part() {
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let mut bank = empty_bank();
    remember(&mut bank, 1, object, CandidateActionFamily::Contact, 0.8);
    remember(&mut bank, 2, object, CandidateActionFamily::Approach, -0.4);
    let emission = recognize(&bank, vec![object], true, &[71]).unwrap();
    assert!((emission.activity - 0.2).abs() < 1e-6);
    remember(&mut bank, 3, object, CandidateActionFamily::Inspect, -0.8);
    assert!(recognize(&bank, vec![object], true, &[71]).is_none());
}

#[test]
fn multiple_individuals_choose_one_signal_and_unseen_or_unattended_friends_emit_none() {
    let a = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let b = slot(1, 72, 0.7, [0.9, 0.6, 0.1]);
    let mut bank = empty_bank();
    remember(&mut bank, 1, a, CandidateActionFamily::Contact, 0.3);
    remember(
        &mut bank,
        2,
        GroundedObjectSlotV1 { slot_index: 0, ..b },
        CandidateActionFamily::Contact,
        0.8,
    );
    let own_a = recognize(&bank, vec![a, b], true, &[71]).unwrap();
    let own_b = recognize(&bank, vec![a, b], true, &[72]).unwrap();
    assert_eq!(recognize(&bank, vec![a, b], true, &[71, 72]), Some(own_b));
    assert!(own_b.activity * own_b.confidence > own_a.activity * own_a.confidence);
    let weak_b = GroundedObjectSlotV1 {
        confidence: Confidence::new(0.1).unwrap(),
        ..b
    };
    let weak_b_signal = recognize(&bank, vec![a, weak_b], true, &[72]).unwrap();
    assert!(own_a.activity * own_a.confidence > weak_b_signal.activity * weak_b_signal.confidence);
    assert_eq!(
        recognize(&bank, vec![a, weak_b], true, &[71, 72]),
        Some(own_a)
    );
    assert!(recognize(&bank, vec![a, b], true, &[]).is_none());
    assert!(recognize(&bank, vec![a, b], false, &[71, 72]).is_none());
    assert!(recognize(&bank, vec![], true, &[71]).is_none());
}

#[test]
fn memory_roundtrip_and_failed_biology_retry_preserve_exact_once_consequence() {
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let mut bank = empty_bank();
    remember(&mut bank, 1, object, CandidateActionFamily::Contact, 0.8);
    let bank_bytes = serde_json::to_vec(&bank).unwrap();
    let restored: MemoryBank = serde_json::from_slice(&bank_bytes).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bank_bytes);
    assert_eq!(
        recognize(&bank, vec![object], true, &[71]),
        recognize(&restored, vec![object], true, &[71])
    );
    let phenotype = CreatureGenome::early_mammal_founder(
        778,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    let current = Tick(u64::from(phenotype.development.maturation_duration_ticks));
    let state = BiochemistryState::new(&phenotype, current).unwrap();
    let emission = finalize_social(&restored, social_draft(vec![object], true, current), &[71])
        .1
        .recognized_social_emission()
        .unwrap();
    let frame = NeuralEmissionFrame::new(current, 7, vec![emission]).unwrap();
    let mut malformed = frame.clone();
    malformed.schema_version += 1;
    let next = Tick(current.raw() + 1);
    assert!(state
        .advance_with_neural_emission(
            next,
            next,
            BodyEventDelta::zero(),
            Some(&malformed),
            &phenotype
        )
        .is_err());
    let actual = state
        .advance_with_neural_emission(next, next, BodyEventDelta::zero(), Some(&frame), &phenotype)
        .unwrap();
    let neutral = state
        .advance(next, BodyEventDelta::zero(), &phenotype)
        .unwrap();
    assert!(actual.homeostasis.hormones.oxytocin > neutral.homeostasis.hormones.oxytocin);
    let saved: BiochemistryState =
        serde_json::from_slice(&serde_json::to_vec(&actual).unwrap()).unwrap();
    let later = Tick(next.raw() + 1);
    assert_eq!(
        saved
            .advance(later, BodyEventDelta::zero(), &phenotype)
            .unwrap(),
        actual
            .advance(later, BodyEventDelta::zero(), &phenotype)
            .unwrap()
    );
    assert!(recognize(&restored, vec![], true, &[71]).is_none());
}

#[test]
fn recognition_reuses_the_existing_sixty_four_record_search_cap() {
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(128, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    for seq in 1..=90 {
        let mut draft = social_draft(vec![object], true, TICK);
        let mut hold = candidate_for_family(&object, 0, CandidateActionFamily::Other);
        hold.action_id = ActionId(20000 + seq as u32);
        draft = PerceptionFrameDraft::new(
            ORGANISM,
            TICK,
            draft.sensor_profile(),
            draft.sensory().clone(),
            draft.body(),
            *draft.homeostasis(),
            vec![hold],
            draft.profile_provenance(),
            vec![object],
        )
        .unwrap();
        let patch = observation_from_draft(&bank, seq, draft, 0.6, 0.0, |outcome| {
            outcome.physical.contact = PhysicalContactKind::None
        });
        bank.observe_sealed_patch(&patch).unwrap();
    }
    assert_eq!(bank.len(), 90);
    let (_, recall) = finalize_social(&bank, social_draft(vec![object], true, TICK), &[71]);
    assert_eq!(recall.receipt().candidates[1].target_searched, 64);
    assert!(recall.recognized_social_emission().is_some());
    assert_eq!(recall.receipt().similarity_evaluations, 64);
}

#[test]
fn duplicate_individual_queries_keep_one_paired_signal_independent_of_action_order() {
    let object = slot(0, 71, 0.4, [0.0, 0.8, 0.9]);
    let mut bank = empty_bank();
    remember(&mut bank, 1, object, CandidateActionFamily::Contact, 0.8);
    let base = social_draft(vec![object], true, TICK);
    let ordinary = base.candidates()[1];
    let mut play = ordinary;
    play.action_id = ActionId(22_000);
    play.kind = ActionKind::Interact;
    play.family = CandidateActionFamily::Contact;
    // The real grounded enumerator makes PLAY a distinct target-context query.
    play.features.0[alife_core::CONTACT_ACTIVATION_FEATURE_LANE] = 1.0;
    let recall = |mut candidates: Vec<ActionCandidate>| {
        for (index, candidate) in candidates.iter_mut().enumerate() {
            candidate.candidate_index = index as u16;
        }
        let draft = PerceptionFrameDraft::new(
            ORGANISM,
            TICK,
            base.sensor_profile(),
            base.sensory().clone(),
            base.body(),
            *base.homeostasis(),
            candidates,
            base.profile_provenance(),
            base.grounded_object_slots().to_vec(),
        )
        .unwrap();
        finalize_social(&bank, draft, &[71])
            .1
            .recognized_social_emission()
            .unwrap()
    };
    let ordinary_signal = recall(vec![ordinary]);
    let play_signal = recall(vec![play]);
    assert!(ordinary_signal.confidence > play_signal.confidence);
    assert_eq!(recall(vec![ordinary, play]), ordinary_signal);
    assert_eq!(recall(vec![play, ordinary]), ordinary_signal);
}
