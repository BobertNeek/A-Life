use alife_core::{
    heuristic_baseline_arbitrate, ActionArbitrationConfig, ActionCandidate, ActionId, ActionKind,
    ActionProposal, ActionTarget, BodySnapshot, BrainClassSpec, BrainGenome, BrainScaleTier,
    CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef, Confidence,
    DecisionSnapshot, DevelopmentState, DriveDelta, DurationTicks, EndocrineDelta, ExperiencePatch,
    ExperiencePatchBuilder, ExperiencePatchPhase, ExperienceSequenceId, FastWeightSemantics,
    HomeostaticDelta, HomeostaticSnapshot, Intensity, LearningSequenceGuard, LobeKind,
    MemoryExpectancySnapshot, NeuralActionSelection, NeuromodulatorSample, NormalizedScalar,
    OrganismId, OutcomeCreditPacket, OutcomeCreditReplayKey, PerceptionFrame, PhenotypeHash,
    PhysicalActionOutcome, PhysicalContactKind, PostActionOutcome, ScaffoldContractError,
    SchemaVersions, SensorProfile, SensorProfileProvenance, SensoryAbiVersion, SensoryChannels,
    SensorySnapshot, SignedValence, Tick, Vec3f, Velocity, WeightSplitContract, WorldEntityId,
};

const ORGANISM: OrganismId = OrganismId(7);
const PHENOTYPE: PhenotypeHash = PhenotypeHash([11, 22, 33, 44]);

fn sequence(raw: u64) -> ExperienceSequenceId {
    ExperienceSequenceId(raw)
}

fn perception(organism_id: OrganismId, tick: Tick) -> PerceptionFrame {
    let sensory = SensorySnapshot::new(
        organism_id,
        tick,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    let candidate = ActionCandidate::new(
        0,
        ActionId(101),
        ActionKind::Move,
        CandidateActionFamily::Approach,
        CandidateObservationRef::None,
        ActionTarget::new(Some(WorldEntityId(55)), Some(Vec3f::new(1.0, 0.0, 2.0))),
        CandidateFeatureVector::zero(),
        Confidence::new(0.9).unwrap(),
        NormalizedScalar::new(0.2).unwrap(),
        DurationTicks::new(2),
        DurationTicks::new(4),
    )
    .unwrap();
    PerceptionFrame::new(
        organism_id,
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory,
        BodySnapshot {
            pose: alife_core::Pose::IDENTITY,
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
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn sealed_neural_patch(
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
    sequence_id: ExperienceSequenceId,
    reward: f32,
    pain: f32,
    frustration: f32,
    novelty: f32,
) -> ExperiencePatch {
    let tick = Tick::new(sequence_id.raw());
    let frame = perception(organism_id, tick);
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let genome = BrainGenome::scaffold(42, spec.id);
    let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.35).unwrap())
        .with_enabled_lobes([
            LobeKind::SensoryGrounding,
            LobeKind::CoreAssociation,
            LobeKind::MotorArbitration,
        ]);
    let pre_action = alife_core::PreActionSnapshot::from_neural_frame(
        sequence_id,
        spec.id,
        phenotype_hash,
        genome.id,
        genome.schema_version,
        development,
        frame.clone(),
    )
    .unwrap();
    let selection = NeuralActionSelection {
        candidate_index: 0,
        logit: 0.75,
        confidence: Confidence::new(0.8).unwrap(),
        active_tiles: 12,
        active_synapses: 144,
    };
    let command = frame.candidates()[0]
        .to_command(organism_id, selection.confidence)
        .unwrap();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id,
        phenotype_hash,
        7,
        1,
        &frame,
        selection,
        command,
    )
    .unwrap();
    let outcome = PostActionOutcome::new(
        organism_id,
        sequence_id,
        Tick::new(tick.raw() + 1),
        reward >= 0.0,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Moved,
            target_entity: Some(WorldEntityId(55)),
            displacement: Vec3f::new(0.25, 0.0, 0.0),
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta {
            drives: DriveDelta {
                hunger: -0.4,
                fatigue: -0.2,
                brain_atp: 0.2,
                ..DriveDelta::zero()
            },
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(reward).unwrap(),
        NormalizedScalar::new(frustration).unwrap(),
        NormalizedScalar::new(pain).unwrap(),
        SignedValence::new(0.1).unwrap(),
        NormalizedScalar::new(novelty).unwrap(),
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

fn sealed_heuristic_patch() -> ExperiencePatch {
    let sequence_id = sequence(77);
    let tick = Tick::new(77);
    let frame = perception(ORGANISM, tick);
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let genome = BrainGenome::scaffold(99, spec.id);
    let development = DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.35).unwrap())
        .with_enabled_lobes([
            LobeKind::SensoryGrounding,
            LobeKind::CoreAssociation,
            LobeKind::MotorArbitration,
        ]);
    let pre_action = alife_core::PreActionSnapshot::from_heuristic_frame(
        sequence_id,
        frame,
        spec.clone(),
        genome.clone(),
        development,
        WeightSplitContract::for_brain_class(
            spec.id,
            spec.max_active_synapses,
            spec.max_active_microtiles,
            genome.genetic_prior_seed,
        )
        .unwrap(),
        MemoryExpectancySnapshot::neutral(),
    )
    .unwrap();
    let proposal = ActionProposal::new(
        ActionId(101),
        ActionKind::Move,
        0.75,
        Confidence::new(0.8).unwrap(),
        None,
        0,
        ActionTarget::new(Some(WorldEntityId(55)), Some(Vec3f::new(1.0, 0.0, 2.0))),
        NormalizedScalar::new(0.0).unwrap(),
    )
    .unwrap()
    .with_intensity(Intensity::new(1.0).unwrap());
    let proposals = vec![proposal];
    let action_decision = heuristic_baseline_arbitrate(
        ORGANISM,
        &proposals,
        ActionArbitrationConfig {
            default_duration_ticks: DurationTicks::new(2),
            ..ActionArbitrationConfig::default()
        },
    )
    .unwrap();
    let decision =
        DecisionSnapshot::from_action_decision(sequence_id, tick, proposals, action_decision)
            .unwrap();
    let outcome = PostActionOutcome::new(
        ORGANISM,
        sequence_id,
        Tick::new(78),
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Moved,
            target_entity: Some(WorldEntityId(55)),
            displacement: Vec3f::new(0.25, 0.0, 0.0),
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
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
fn pain_and_reward_create_opposite_bounded_modulators() {
    let reward = NeuromodulatorSample::from_components(0.8, 0.0, 0.2, 0.0, 0.1).unwrap();
    let pain = NeuromodulatorSample::from_components(-0.2, 0.9, -0.4, 0.3, 0.0).unwrap();
    assert!(reward.value() > 0.0);
    assert!(pain.value() < 0.0);
    assert!((-1.0..=1.0).contains(&reward.value()));
    assert!((-1.0..=1.0).contains(&pain.value()));

    let saturated = NeuromodulatorSample::from_components(1.0, -1.0, 1.0, -1.0, 1.0).unwrap();
    assert_eq!(saturated.value(), 1.0);
    assert_eq!(
        NeuromodulatorSample::from_components(f32::NAN, 0.0, 0.0, 0.0, 0.0),
        Err(ScaffoldContractError::NonFiniteFloat)
    );
    assert_eq!(
        NeuromodulatorSample::from_components(1.01, 0.0, 0.0, 0.0, 0.0),
        Err(ScaffoldContractError::ScalarOutOfRange)
    );
}

#[test]
fn serialized_modulator_recomputes_and_rejects_a_forged_value() {
    let sample = NeuromodulatorSample::from_components(0.8, 0.0, 0.2, 0.0, 0.1).unwrap();
    let mut json = serde_json::to_value(sample).unwrap();
    json["value"] = serde_json::json!(-1.0);
    assert!(serde_json::from_value::<NeuromodulatorSample>(json).is_err());

    let round_trip: NeuromodulatorSample =
        serde_json::from_value(serde_json::to_value(sample).unwrap()).unwrap();
    assert_eq!(round_trip, sample);
}

#[test]
fn credit_packet_is_derived_exactly_from_matching_sealed_gpu_evidence() {
    let patch = sealed_neural_patch(ORGANISM, PHENOTYPE, sequence(99), 0.25, 0.1, 0.2, 0.3);
    let packet = OutcomeCreditPacket::from_sealed_patch(&patch).unwrap();
    let evidence = patch.decision().neural_evidence().unwrap();

    assert_eq!(
        packet.schema_version(),
        SchemaVersions::CURRENT.learning.raw()
    );
    assert_eq!(packet.organism_id(), ORGANISM);
    assert_eq!(packet.sequence_id(), patch.header().sequence_id);
    assert_eq!(packet.originating_tick(), patch.header().world_tick);
    assert_eq!(packet.outcome_tick(), patch.outcome().outcome_tick);
    assert_eq!(packet.phenotype_hash(), evidence.phenotype_hash);
    assert_eq!(packet.frame_digest(), evidence.frame_digest);
    assert_eq!(packet.active_activation_side(), 1);
    assert_eq!(packet.selected_candidate(), evidence.candidate_index);
    assert_eq!(packet.selected_family(), evidence.action_family);
    assert_eq!(packet.selected_action(), evidence.action_id);
    assert_eq!(
        packet.candidate_feature_digest(),
        evidence.candidate_feature_digest
    );
    assert_eq!(packet.dispatch_generation(), evidence.dispatch_generation);
    assert_eq!(packet.modulator().reward_prediction_error(), 0.25);
    assert_eq!(packet.modulator().pain(), 0.1);
    assert_eq!(packet.modulator().frustration(), 0.2);
    assert_eq!(packet.modulator().novelty(), 0.3);
    assert!(packet.modulator().homeostatic_improvement() > 0.0);
    assert_eq!(
        packet.replay_key(),
        OutcomeCreditReplayKey {
            organism_id: ORGANISM,
            phenotype_hash: PHENOTYPE,
            sequence_id: sequence(99),
        }
    );
}

#[test]
fn tampered_or_unsealed_patch_wire_cannot_become_credit_evidence() {
    let patch = sealed_neural_patch(ORGANISM, PHENOTYPE, sequence(99), 0.25, 0.1, 0.2, 0.3);
    let original = serde_json::to_value(&patch).unwrap();

    for (case, mutate) in [
        |value: &mut serde_json::Value| value["header"]["organism_id"] = serde_json::json!(8),
        |value: &mut serde_json::Value| value["header"]["sequence_id"] = serde_json::json!(100),
        |value: &mut serde_json::Value| value["header"]["world_tick"] = serde_json::json!(98),
        |value: &mut serde_json::Value| {
            value["header"]["phase"] = serde_json::json!(ExperiencePatchPhase::PostActionOutcome)
        },
        |value: &mut serde_json::Value| {
            value["decision"]["evidence"]["NeuralClosedLoopGpu"]["active_activation_side"] =
                serde_json::json!(2)
        },
        |value: &mut serde_json::Value| {
            value["decision"]["evidence"]["NeuralClosedLoopGpu"]["phenotype_hash"] =
                serde_json::json!([1, 2, 3, 4])
        },
        |value: &mut serde_json::Value| {
            value["decision"]["evidence"]["NeuralClosedLoopGpu"]["frame_digest"] =
                serde_json::json!([1, 2, 3, 4])
        },
        |value: &mut serde_json::Value| {
            value["decision"]["evidence"]["NeuralClosedLoopGpu"]["action_id"] =
                serde_json::json!(999)
        },
    ]
    .into_iter()
    .enumerate()
    {
        let mut tampered = original.clone();
        mutate(&mut tampered);
        if let Ok(tampered_patch) = serde_json::from_value::<ExperiencePatch>(tampered) {
            assert_eq!(
                OutcomeCreditPacket::from_sealed_patch(&tampered_patch),
                Err(ScaffoldContractError::LearningEvidenceMismatch),
                "tamper case {case} unexpectedly became credit evidence"
            );
        }
    }
}

#[test]
fn heuristic_evidence_cannot_create_neural_outcome_credit() {
    let patch = sealed_heuristic_patch();
    assert_eq!(
        OutcomeCreditPacket::from_sealed_patch(&patch),
        Err(ScaffoldContractError::LearningEvidenceMismatch)
    );
}

#[test]
fn sequence_guard_rejects_wrong_ownership_replay_and_stale_tokens_without_mutation() {
    let mut guard = LearningSequenceGuard::new(ORGANISM, PHENOTYPE);
    let pristine = guard.clone();
    let valid = OutcomeCreditReplayKey {
        organism_id: ORGANISM,
        phenotype_hash: PHENOTYPE,
        sequence_id: sequence(10),
    };
    let wrong_organism = OutcomeCreditReplayKey {
        organism_id: OrganismId(8),
        ..valid
    };
    let wrong_phenotype = OutcomeCreditReplayKey {
        phenotype_hash: PhenotypeHash([1, 2, 3, 4]),
        ..valid
    };
    assert_eq!(
        guard.validate_next(wrong_organism),
        Err(ScaffoldContractError::LearningEvidenceMismatch)
    );
    assert_eq!(guard, pristine);
    assert_eq!(
        guard.validate_next(wrong_phenotype),
        Err(ScaffoldContractError::LearningEvidenceMismatch)
    );
    assert_eq!(guard, pristine);

    let token = guard.validate_next(valid).unwrap();
    let stale = guard.validate_next(valid).unwrap();
    guard.commit_validated(token).unwrap();
    assert_eq!(guard.last_committed(), Some(valid));
    let committed = guard.clone();
    assert_eq!(
        guard.commit_validated(stale),
        Err(ScaffoldContractError::LearningReplayRejected)
    );
    assert_eq!(guard, committed);

    for raw in [9, 10] {
        let replay = OutcomeCreditReplayKey {
            sequence_id: sequence(raw),
            ..valid
        };
        assert_eq!(
            guard.validate_next(replay),
            Err(ScaffoldContractError::LearningReplayRejected)
        );
        assert_eq!(guard, committed);
    }
}

#[test]
fn sequence_guard_restore_validates_bound_replay_identity() {
    let last = OutcomeCreditReplayKey {
        organism_id: ORGANISM,
        phenotype_hash: PHENOTYPE,
        sequence_id: sequence(42),
    };
    let restored =
        LearningSequenceGuard::restore_validated(ORGANISM, PHENOTYPE, Some(last)).unwrap();
    assert_eq!(restored.last_committed(), Some(last));
    assert_eq!(
        LearningSequenceGuard::restore_validated(OrganismId(8), PHENOTYPE, Some(last)),
        Err(ScaffoldContractError::LearningEvidenceMismatch)
    );
    assert_eq!(
        LearningSequenceGuard::restore_validated(
            ORGANISM,
            PHENOTYPE,
            Some(OutcomeCreditReplayKey {
                phenotype_hash: PhenotypeHash([9, 9, 9, 9]),
                ..last
            }),
        ),
        Err(ScaffoldContractError::LearningEvidenceMismatch)
    );
}

#[test]
fn fast_weight_semantics_is_an_explicit_versioned_contract() {
    let value = FastWeightSemantics::ImmediateThreeFactor;
    let round_trip: FastWeightSemantics =
        serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap();
    assert_eq!(round_trip, value);
}
