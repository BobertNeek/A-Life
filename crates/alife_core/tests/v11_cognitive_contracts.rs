use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, BrainClassSpec, BrainGenome,
    BrainScaleTier, CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef,
    ChannelCommand, CognitiveContextFrame, CognitiveWorkReceipt, Confidence, DecisionSnapshot,
    DevelopmentState, DurationTicks, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    HomeostaticDelta, HomeostaticSnapshot, Intensity, JointPhysicalOutcome,
    MeasuredChannelObservation, MemoryExpectancySnapshot, MotorChannel, MotorCommandBundle,
    NormalizedScalar, OrganismId, PerceptionFrame, PhysicalActionOutcome, PhysicalContactKind,
    Pose, PostActionOutcome, PredictionTargetReceipt, ScaffoldContractError, SensorProfile,
    SensorProfileProvenance, SensoryAbiVersion, SensoryChannels, SensorySnapshot, SignedValence,
    Tick, Validate, Vec3f, Velocity, WeightSplitContract,
};

fn organism() -> OrganismId {
    OrganismId(7)
}

fn sequence() -> ExperienceSequenceId {
    ExperienceSequenceId(99)
}

fn context() -> CognitiveContextFrame {
    CognitiveContextFrame::empty(organism(), sequence(), Tick::new(10)).unwrap()
}

fn bundle() -> MotorCommandBundle {
    MotorCommandBundle::new(
        organism(),
        sequence(),
        Tick::new(10),
        vec![
            ChannelCommand::new(
                MotorChannel::Locomotion,
                ActionId(300),
                None,
                Vec3f::new(0.0, 0.0, 1.0),
                Intensity::new(0.8).unwrap(),
                DurationTicks::new(2),
                0.0,
                Confidence::new(0.9).unwrap(),
                1,
            )
            .unwrap(),
            ChannelCommand::new(
                MotorChannel::Vocal,
                ActionId(400),
                None,
                Vec3f::ZERO,
                Intensity::new(0.4).unwrap(),
                DurationTicks::new(1),
                0.0,
                Confidence::new(0.7).unwrap(),
                1,
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

fn joint_outcome() -> JointPhysicalOutcome {
    JointPhysicalOutcome::new(
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Moved,
            target_entity: None,
            displacement: Vec3f::new(0.5, 0.0, 0.0),
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        vec![MeasuredChannelObservation::new(
            MotorChannel::Locomotion,
            true,
            NormalizedScalar::new(0.8).unwrap(),
            Vec3f::new(0.5, 0.0, 0.0),
        )
        .unwrap()],
    )
    .unwrap()
}

fn prediction() -> PredictionTargetReceipt {
    PredictionTargetReceipt::for_successor(
        organism(),
        sequence(),
        ActionId(300),
        Tick::new(10),
        [1, 2, 3, 4],
        1,
        vec![0.2, 0.8],
    )
    .unwrap()
}

fn work() -> CognitiveWorkReceipt {
    CognitiveWorkReceipt::from_counters(3, 2, 1, 4, 2, 1, 1, 3, 0, 2, 1, 0).unwrap()
}

fn legacy_pre_action(tick: Tick) -> alife_core::PreActionSnapshot {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Standard2048);
    let genome = BrainGenome::scaffold(42, spec.id);
    let perception = PerceptionFrame::new(
        organism(),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        SensorySnapshot::new(
            organism(),
            tick,
            Vec3f::new(1.0, 2.0, 3.0),
            SensoryChannels::default(),
            Default::default(),
        )
        .unwrap(),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        vec![ActionCandidate::new(
            0,
            ActionId(300),
            ActionKind::Move,
            CandidateActionFamily::Approach,
            CandidateObservationRef::None,
            ActionTarget::new(None, Some(Vec3f::new(0.0, 0.0, 1.0))),
            CandidateFeatureVector::zero(),
            Confidence::new(0.8).unwrap(),
            NormalizedScalar::new(0.0).unwrap(),
            DurationTicks::new(1),
            DurationTicks::new(1),
        )
        .unwrap()],
        SensorProfileProvenance::new(
            SensorProfile::PrivilegedAffordanceV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        Vec::new(),
    )
    .unwrap();
    alife_core::PreActionSnapshot::from_heuristic_frame(
        sequence(),
        perception,
        spec.clone(),
        genome.clone(),
        DevelopmentState::new(genome.id, Tick::new(1), NormalizedScalar::new(0.1).unwrap()),
        WeightSplitContract::for_brain_class(
            spec.id,
            spec.max_active_synapses,
            spec.max_active_microtiles,
            genome.genetic_prior_seed,
        )
        .unwrap(),
        MemoryExpectancySnapshot::neutral(),
    )
    .unwrap()
}

fn legacy_patch() -> ExperiencePatch {
    let pre = legacy_pre_action(Tick::new(10));
    let action_decision = alife_core::heuristic_baseline_arbitrate(
        organism(),
        &[],
        alife_core::ActionArbitrationConfig::default(),
    )
    .unwrap();
    let decision = DecisionSnapshot::from_action_decision(
        sequence(),
        Tick::new(10),
        Vec::new(),
        action_decision,
    )
    .unwrap();
    let outcome = PostActionOutcome::new(
        organism(),
        sequence(),
        Tick::new(11),
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Moved,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.0).unwrap(),
        },
        HomeostaticDelta::zero(),
        SignedValence::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        SignedValence::new(0.0).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
    )
    .unwrap();
    ExperiencePatchBuilder::new(sequence())
        .record_pre_action(pre)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}

#[test]
fn bounded_contracts_reject_overflow_and_mismatched_identity() {
    let mut frame = context();
    frame.attention.peripheral_summaries = (0..=alife_core::MAX_PERIPHERAL_SUMMARIES)
        .map(|_| Default::default())
        .collect();
    assert!(frame.validate_contract().is_err());

    let mut mismatched = context();
    mismatched.attention.organism_id = OrganismId(8);
    assert!(matches!(
        mismatched.validate_contract(),
        Err(ScaffoldContractError::MismatchedCreatureId)
    ));
}

#[test]
fn motor_channels_and_one_joint_outcome_round_trip_without_channel_reward() {
    let bundle = bundle();
    let outcome = joint_outcome();
    let bytes = serde_json::to_vec(&(bundle.clone(), outcome.clone())).unwrap();
    let (decoded_bundle, decoded_outcome): (MotorCommandBundle, JointPhysicalOutcome) =
        serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded_bundle, bundle);
    assert_eq!(decoded_outcome, outcome);
    assert_eq!(decoded_outcome.channel_observations.len(), 1);
    assert!(decoded_outcome.joint_reward().is_none());
}

#[test]
fn v11_patch_binds_exact_prediction_and_work_receipts_and_reads_legacy_explicitly() {
    let patch = ExperiencePatch::new_v11(
        legacy_pre_action(Tick::new(10)),
        bundle(),
        joint_outcome(),
        prediction(),
        work(),
        context(),
    )
    .unwrap();
    let digest = patch.causal_digest().unwrap();
    assert_eq!(patch.prediction_target(), Some(&prediction()));
    assert_eq!(patch.cognitive_work(), Some(&work()));
    assert_ne!(digest, [0; 4]);

    let changed = ExperiencePatch::new_v11(
        legacy_pre_action(Tick::new(10)),
        bundle(),
        joint_outcome(),
        PredictionTargetReceipt::for_successor(
            organism(),
            sequence(),
            ActionId(300),
            Tick::new(10),
            [9, 2, 3, 4],
            1,
            vec![0.2, 0.8],
        )
        .unwrap(),
        work(),
        context(),
    )
    .unwrap();
    assert_ne!(digest, changed.causal_digest().unwrap());

    let legacy = legacy_patch();
    let bytes = serde_json::to_vec(&legacy).unwrap();
    let decoded: ExperiencePatch = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.header().abi_version, ExperiencePatch::ABI_VERSION);
    assert!(decoded.selected_bundle().is_none());
    assert_ne!(
        ExperiencePatch::ABI_VERSION,
        ExperiencePatch::V11_ABI_VERSION
    );
}
