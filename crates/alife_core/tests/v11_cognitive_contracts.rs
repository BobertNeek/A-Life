use serde::Serialize;

use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, BrainClassSpec, BrainGenome,
    BrainScaleTier, CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef,
    ChannelCommand, CognitiveContextFrame, CognitiveWorkReceipt, Confidence, DecisionSnapshot,
    DevelopmentState, DurationTicks, ExperiencePatch, ExperienceSequenceId, HomeostaticDelta,
    HomeostaticSnapshot, Intensity, JointPhysicalOutcome, MeasuredChannelObservation,
    MemoryExpectancySnapshot, MotorChannel, MotorCommandBundle, NormalizedScalar, OrganismId,
    PerceptionFrame, PhysicalActionOutcome, PhysicalContactKind, Pose, PostActionOutcome,
    PredictionTargetReceipt, ScaffoldContractError, SensorProfile, SensorProfileProvenance,
    SensoryAbiVersion, SensoryChannels, SensorySnapshot, SignedValence, Tick, Validate, Vec3f,
    Velocity, WeightSplitContract,
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

fn measured_outcome(joint: JointPhysicalOutcome) -> PostActionOutcome {
    PostActionOutcome::new(
        organism(),
        sequence(),
        Tick::new(12),
        false,
        joint.execution,
        HomeostaticDelta::zero(),
        SignedValence::new(-0.25).unwrap(),
        NormalizedScalar::new(0.4).unwrap(),
        NormalizedScalar::new(0.3).unwrap(),
        SignedValence::new(-0.2).unwrap(),
        NormalizedScalar::new(0.6).unwrap(),
    )
    .unwrap()
    .with_v11_joint(joint, work())
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

#[derive(Serialize)]
struct LegacyV1PreAction {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    tick: Tick,
    brain_class_id: alife_core::BrainClassId,
    brain_scale_tier: BrainScaleTier,
    brain_neuron_count: u32,
    max_active_synapses: u32,
    max_active_microtiles: u32,
    routing_schema_version: u16,
    lobe_layout: alife_core::LobeLayout,
    routing_matrix: alife_core::RoutingMatrix,
    genome_id: alife_core::GenomeId,
    genome_schema_version: u16,
    development_state: DevelopmentState,
    weight_split: WeightSplitContract,
    sensory_abi_version: SensoryAbiVersion,
    chemistry_schema_version: u16,
    body_pose: Pose,
    body_velocity: Velocity,
    homeostasis: HomeostaticSnapshot,
    sensory: SensorySnapshot,
    memory_expectancy: MemoryExpectancySnapshot,
}

#[derive(Serialize)]
struct LegacyV1Decision {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    decision_tick: Tick,
    action_abi_version: u16,
    proposals: Vec<alife_core::ActionProposal>,
    selected_action: alife_core::ActionCommand,
    rejected_top_proposal: Option<alife_core::RankedActionProposal>,
    ranked_top_proposals: Vec<alife_core::RankedActionProposal>,
    arbitration_trace: alife_core::ActionArbitrationTrace,
    confidence: Confidence,
    status: alife_core::ActionDecisionStatus,
}

#[derive(Serialize)]
struct LegacyHeader {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    world_tick: Tick,
    phase: alife_core::ExperiencePatchPhase,
}

#[derive(Serialize)]
struct LegacyV1Patch {
    header: LegacyHeader,
    pre_action: LegacyV1PreAction,
    decision: LegacyV1Decision,
    outcome: PostActionOutcome,
}

#[derive(Serialize)]
struct LegacyV2Decision {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    decision_tick: Tick,
    action_abi_version: u16,
    selected_action: alife_core::ActionCommand,
    confidence: Confidence,
    evidence: alife_core::DecisionEvidence,
}

#[derive(Serialize)]
struct LegacyV2Patch {
    header: LegacyHeader,
    pre_action: serde_json::Value,
    decision: LegacyV2Decision,
    outcome: PostActionOutcome,
}

fn legacy_outcome(abi_version: u16) -> PostActionOutcome {
    let mut outcome = PostActionOutcome::new(
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
    outcome.abi_version = abi_version;
    outcome
}

fn legacy_v1_bytes() -> Vec<u8> {
    let tick = Tick::new(10);
    let pre = legacy_pre_action(tick);
    let evidence = pre.heuristic_evidence().unwrap();
    let sensory = pre.sensory().clone();
    let proposal = alife_core::ActionProposal::new(
        ActionId(300),
        ActionKind::Move,
        0.75,
        Confidence::new(0.8).unwrap(),
        None,
        0,
        ActionTarget::new(None, Some(Vec3f::new(0.0, 0.0, 1.0))),
        NormalizedScalar::new(0.4).unwrap(),
    )
    .unwrap();
    let proposals = vec![proposal];
    let action_decision = alife_core::heuristic_baseline_arbitrate(
        organism(),
        &proposals,
        alife_core::ActionArbitrationConfig::default(),
    )
    .unwrap();
    serde_json::to_vec(&LegacyV1Patch {
        header: LegacyHeader {
            abi_version: 1,
            organism_id: organism(),
            sequence_id: sequence(),
            world_tick: tick,
            phase: alife_core::ExperiencePatchPhase::Sealed,
        },
        pre_action: LegacyV1PreAction {
            abi_version: 1,
            organism_id: pre.organism_id,
            sequence_id: pre.sequence_id,
            tick: pre.tick,
            brain_class_id: evidence.brain_class_id,
            brain_scale_tier: evidence.brain_scale_tier,
            brain_neuron_count: evidence.brain_neuron_count,
            max_active_synapses: evidence.max_active_synapses,
            max_active_microtiles: evidence.max_active_microtiles,
            routing_schema_version: evidence.routing_schema_version,
            lobe_layout: evidence.lobe_layout.clone(),
            routing_matrix: evidence.routing_matrix.clone(),
            genome_id: pre.genome_id,
            genome_schema_version: pre.genome_schema_version,
            development_state: pre.development_state.clone(),
            weight_split: evidence.weight_split.clone(),
            sensory_abi_version: sensory.abi_version,
            chemistry_schema_version: pre.homeostasis().schema_version,
            body_pose: pre.body().pose,
            body_velocity: pre.body().velocity,
            homeostasis: *pre.homeostasis(),
            sensory,
            memory_expectancy: evidence.memory_expectancy,
        },
        decision: LegacyV1Decision {
            abi_version: 1,
            organism_id: organism(),
            sequence_id: sequence(),
            decision_tick: tick,
            action_abi_version: alife_core::ActionCommand::ABI_VERSION,
            proposals,
            selected_action: action_decision.selected,
            rejected_top_proposal: action_decision.rejected_top_proposal,
            ranked_top_proposals: action_decision.ranked_top_proposals,
            arbitration_trace: action_decision.trace,
            confidence: action_decision.selected.confidence,
            status: action_decision.status,
        },
        outcome: legacy_outcome(1),
    })
    .unwrap()
}

fn legacy_v2_bytes() -> Vec<u8> {
    let tick = Tick::new(10);
    let pre = legacy_pre_action(tick);
    let mut pre_value = serde_json::to_value(pre).unwrap();
    pre_value["abi_version"] = serde_json::json!(2u16);
    let action_decision = alife_core::heuristic_baseline_arbitrate(
        organism(),
        &[],
        alife_core::ActionArbitrationConfig::default(),
    )
    .unwrap();
    let decision =
        DecisionSnapshot::from_action_decision(sequence(), tick, Vec::new(), action_decision)
            .unwrap();
    serde_json::to_vec(&LegacyV2Patch {
        header: LegacyHeader {
            abi_version: 2,
            organism_id: organism(),
            sequence_id: sequence(),
            world_tick: tick,
            phase: alife_core::ExperiencePatchPhase::Sealed,
        },
        pre_action: pre_value,
        decision: LegacyV2Decision {
            abi_version: 2,
            organism_id: decision.organism_id,
            sequence_id: decision.sequence_id,
            decision_tick: decision.decision_tick,
            action_abi_version: decision.action_abi_version,
            selected_action: decision.selected_action,
            confidence: decision.confidence,
            evidence: decision.evidence,
        },
        outcome: legacy_outcome(2),
    })
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

    assert!(PredictionTargetReceipt::for_successor(
        organism(),
        sequence(),
        ActionId(300),
        Tick::new(10),
        [1, 2, 3, 4],
        2,
        vec![0.2, 0.8],
    )
    .is_err());
    assert!(PredictionTargetReceipt::for_successor(
        organism(),
        sequence(),
        ActionId(300),
        Tick::new(10),
        [1, 2, 3, 4],
        1,
        vec![1.2, 0.8],
    )
    .is_err());
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
        measured_outcome(joint_outcome()),
        prediction(),
        work(),
        context(),
    )
    .unwrap();
    let digest = patch.causal_digest().unwrap();
    assert_eq!(patch.outcome(), &measured_outcome(joint_outcome()));
    assert_eq!(patch.prediction_target(), Some(&prediction()));
    assert_eq!(patch.cognitive_work(), Some(&work()));
    assert_ne!(digest, [0; 4]);

    let mut changed_joint = joint_outcome();
    changed_joint.channel_observations[0].measured_intensity = NormalizedScalar::new(0.2).unwrap();
    let changed = ExperiencePatch::new_v11(
        legacy_pre_action(Tick::new(10)),
        bundle(),
        measured_outcome(changed_joint),
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

    let decoded_v1: ExperiencePatch = serde_json::from_slice(&legacy_v1_bytes()).unwrap();
    assert_eq!(
        decoded_v1.header().abi_version,
        ExperiencePatch::ABI_VERSION
    );
    assert!(decoded_v1.selected_bundle().is_none());
    assert_eq!(
        decoded_v1.pre_action().evidence_kind(),
        alife_core::EvidenceKind::HeuristicBaseline
    );
    assert!(decoded_v1.validate_contract().is_ok());

    let decoded_v2: ExperiencePatch = serde_json::from_slice(&legacy_v2_bytes()).unwrap();
    assert_eq!(
        decoded_v2.header().abi_version,
        ExperiencePatch::ABI_VERSION
    );
    assert!(decoded_v2.selected_bundle().is_none());
    assert!(decoded_v2.validate_contract().is_ok());
    assert_ne!(
        ExperiencePatch::ABI_VERSION,
        ExperiencePatch::V11_ABI_VERSION
    );
}
