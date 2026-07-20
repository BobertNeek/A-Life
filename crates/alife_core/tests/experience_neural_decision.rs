use alife_core::{
    heuristic_baseline_arbitrate, ActionArbitrationConfig, ActionCandidate, ActionCommand,
    ActionId, ActionKind, ActionProposal, ActionTarget, BodySnapshot, BrainClassId, BrainClassSpec,
    BrainGenome, BrainScaleTier, CandidateActionFamily, CandidateFeatureVector,
    CandidateObservationRef, Confidence, DecisionSnapshot, DevelopmentState, DurationTicks,
    EvidenceKind, ExperiencePacker, ExperiencePatch, ExperiencePatchBuilder, ExperiencePatchPhase,
    ExperienceSequenceId, HomeostaticDelta, HomeostaticSnapshot, Intensity, LobeKind, MemoryBank,
    MemoryBankConfig, MemoryExpectancySnapshot, NeuralActionSelection, NormalizedScalar,
    OrganismId, PerceptionFrame, PhenotypeHash, PhysicalActionOutcome, PhysicalContactKind,
    PolicyBackend, Pose, PostActionOutcome, PreActionSnapshot, ScaffoldContractError,
    SensorProfile, SensorProfileProvenance, SensoryAbiVersion, SensoryChannels, SensorySnapshot,
    SignedValence, Tick, TopologicalMapConfig, TopologySidecar, Validate, Vec3f, Velocity,
    WeightSplitContract, WorldEntityId,
};
use serde::Serialize;

fn organism() -> OrganismId {
    OrganismId(7)
}

fn sequence_id() -> ExperienceSequenceId {
    ExperienceSequenceId(99)
}

fn phenotype_hash() -> PhenotypeHash {
    PhenotypeHash([11, 22, 33, 44])
}

fn brain_spec() -> BrainClassSpec {
    BrainClassSpec::for_tier(BrainScaleTier::Nano512)
}

fn genome(spec: &BrainClassSpec) -> BrainGenome {
    BrainGenome::scaffold(42, spec.id)
}

fn development(genome: &BrainGenome, tick: Tick) -> DevelopmentState {
    DevelopmentState::new(genome.id, tick, NormalizedScalar::new(0.35).unwrap()).with_enabled_lobes(
        [
            LobeKind::SensoryGrounding,
            LobeKind::CoreAssociation,
            LobeKind::MotorArbitration,
        ],
    )
}

fn perception_fixture() -> PerceptionFrame {
    let tick = Tick::new(7);
    let sensory = SensorySnapshot::new(
        organism(),
        tick,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    let candidates = vec![
        ActionCandidate::new(
            0,
            ActionId(4),
            ActionKind::Inspect,
            CandidateActionFamily::Inspect,
            CandidateObservationRef::None,
            ActionTarget::NONE,
            CandidateFeatureVector::zero(),
            Confidence::new(0.8).unwrap(),
            NormalizedScalar::new(0.1).unwrap(),
            DurationTicks::new(1),
            DurationTicks::new(1),
        )
        .unwrap(),
        ActionCandidate::new(
            1,
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
        .unwrap(),
    ];
    PerceptionFrame::new(
        organism(),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        candidates,
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

fn neural_selection_fixture(frame: &PerceptionFrame, index: usize) -> NeuralActionSelection {
    NeuralActionSelection {
        candidate_index: frame.candidates()[index].candidate_index,
        logit: 0.75,
        confidence: Confidence::new(0.8).unwrap(),
        active_tiles: 12,
        active_synapses: 144,
    }
}

fn command_for_candidate(candidate: &ActionCandidate) -> ActionCommand {
    candidate
        .to_command(organism(), Confidence::new(0.8).unwrap())
        .unwrap()
}

fn neural_pre_action(frame: PerceptionFrame) -> PreActionSnapshot {
    let spec = brain_spec();
    let genome = genome(&spec);
    PreActionSnapshot::from_neural_frame(
        sequence_id(),
        spec.id,
        phenotype_hash(),
        genome.id,
        genome.schema_version,
        development(&genome, frame.tick()),
        frame,
    )
    .unwrap()
}

fn outcome() -> PostActionOutcome {
    PostActionOutcome::new(
        organism(),
        sequence_id(),
        Tick::new(8),
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
    .unwrap()
}

fn seal_with_decision(
    frame: PerceptionFrame,
    decision: DecisionSnapshot,
) -> Result<ExperiencePatch, ScaffoldContractError> {
    ExperiencePatchBuilder::new(sequence_id())
        .record_pre_action(neural_pre_action(frame))?
        .record_decision(decision)?
        .record_outcome(outcome())?
        .seal()
}

#[test]
fn sealed_patch_binds_gpu_selection_to_the_perception_frame() {
    let frame = perception_fixture();
    let selection = neural_selection_fixture(&frame, 1);
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        selection,
        command_for_candidate(&frame.candidates()[1]),
    )
    .unwrap();
    let patch = seal_with_decision(frame.clone(), decision).unwrap();
    let evidence = patch.decision().neural_evidence().unwrap();

    assert_eq!(
        evidence.base_digest,
        patch.pre_action().base_digest().unwrap()
    );
    assert_eq!(
        evidence.frame_digest,
        patch.pre_action().frame_digest().unwrap()
    );
    assert_eq!(evidence.action_id, frame.candidates()[1].action_id);
    assert_eq!(evidence.action_family, CandidateActionFamily::Approach);
    assert_eq!(
        patch.pre_action().policy_backend(),
        PolicyBackend::NeuralClosedLoopGpu
    );
}

#[test]
fn mismatched_candidate_or_command_cannot_be_sealed() {
    let frame = perception_fixture();
    let selection = neural_selection_fixture(&frame, 1);
    let wrong = command_for_candidate(&frame.candidates()[0]);
    assert!(DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        selection,
        wrong,
    )
    .is_err());

    let wrong_target = ActionCommand::structured(
        organism(),
        frame.candidates()[1].action_id,
        frame.candidates()[1].kind,
        ActionTarget::new(
            Some(WorldEntityId(99)),
            frame.candidates()[1].target.position,
        ),
        Intensity::new(1.0).unwrap(),
        frame.candidates()[1].min_duration,
        Confidence::new(0.8).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        selection,
        wrong_target,
    )
    .is_err());

    let wrong_duration = ActionCommand::structured(
        organism(),
        frame.candidates()[1].action_id,
        frame.candidates()[1].kind,
        frame.candidates()[1].target,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(3),
        Confidence::new(0.8).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        selection,
        wrong_duration,
    )
    .is_err());
}

#[test]
fn neural_selection_rejects_invalid_side_index_and_receipt_values() {
    let frame = perception_fixture();
    let command = command_for_candidate(&frame.candidates()[1]);

    assert!(DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        2,
        &frame,
        neural_selection_fixture(&frame, 1),
        command,
    )
    .is_err());

    let mut invalid = neural_selection_fixture(&frame, 1);
    invalid.logit = f32::NAN;
    assert!(DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        invalid,
        command,
    )
    .is_err());
}

#[test]
fn signed_zero_cannot_change_digest_bound_command_bits() {
    let original = perception_fixture();
    let mut changed = original.candidates()[1];
    changed.target = ActionTarget::new(changed.target.entity, Some(Vec3f::new(-0.0, 0.0, 2.0)));
    let frame = PerceptionFrame::new(
        original.organism_id(),
        original.tick(),
        original.sensor_profile(),
        original.sensory().clone(),
        original.body(),
        *original.homeostasis(),
        vec![original.candidates()[0], changed],
        original.profile_provenance(),
        original.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let selection = NeuralActionSelection {
        confidence: Confidence::new(0.0).unwrap(),
        ..neural_selection_fixture(&frame, 1)
    };
    let command = ActionCommand::structured(
        organism(),
        changed.action_id,
        changed.kind,
        ActionTarget::new(changed.target.entity, Some(Vec3f::new(0.0, 0.0, 2.0))),
        Intensity::new(1.0).unwrap(),
        changed.min_duration,
        Confidence::new(-0.0).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();

    assert!(DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        selection,
        command,
    )
    .is_err());
}

#[test]
fn patch_sealing_rejects_phenotype_or_digest_mismatch() {
    let frame = perception_fixture();
    let selection = neural_selection_fixture(&frame, 1);
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        selection,
        command_for_candidate(&frame.candidates()[1]),
    )
    .unwrap();
    let mut value = serde_json::to_value(decision).unwrap();
    value["evidence"]["NeuralClosedLoopGpu"]["phenotype_hash"][0] = serde_json::json!(999u64);
    let tampered: DecisionSnapshot = serde_json::from_value(value).unwrap();

    assert!(seal_with_decision(frame, tampered).is_err());
}

#[test]
fn sealed_patch_rejects_header_tick_detached_from_perception() {
    let frame = perception_fixture();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        neural_selection_fixture(&frame, 1),
        command_for_candidate(&frame.candidates()[1]),
    )
    .unwrap();
    let patch = seal_with_decision(frame, decision).unwrap();
    let mut value = serde_json::to_value(patch).unwrap();
    value["header"]["world_tick"] = serde_json::json!(6u64);
    let detached: ExperiencePatch = serde_json::from_value(value).unwrap();

    assert!(detached.validate_contract().is_err());
}

#[test]
fn neural_evidence_rejects_baseline_only_accessors_with_typed_error() {
    let frame = perception_fixture();
    let pre = neural_pre_action(frame.clone());
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        neural_selection_fixture(&frame, 1),
        command_for_candidate(&frame.candidates()[1]),
    )
    .unwrap();

    assert_eq!(pre.evidence_kind(), EvidenceKind::NeuralClosedLoopGpu);
    assert_eq!(decision.evidence_kind(), EvidenceKind::NeuralClosedLoopGpu);
    assert_eq!(
        pre.heuristic_evidence(),
        Err(ScaffoldContractError::EvidenceKindMismatch)
    );
    assert_eq!(
        decision.heuristic_evidence(),
        Err(ScaffoldContractError::EvidenceKindMismatch)
    );
}

#[test]
fn neural_patch_serialization_omits_baseline_only_payloads() {
    let frame = perception_fixture();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        neural_selection_fixture(&frame, 1),
        command_for_candidate(&frame.candidates()[1]),
    )
    .unwrap();
    let patch = seal_with_decision(frame, decision).unwrap();
    let value = serde_json::to_value(patch).unwrap();

    assert!(value["pre_action"].get("heuristic_evidence").is_none());
    assert!(value["pre_action"].get("body").is_none());
    assert!(value["pre_action"].get("homeostasis").is_none());
    assert!(value["pre_action"]["perception"]["base"]
        .get("body")
        .is_some());
    assert!(value["pre_action"]["perception"]["base"]
        .get("homeostasis")
        .is_some());
    assert!(value["decision"]["evidence"]
        .get("NeuralClosedLoopGpu")
        .is_some());
    assert!(!value["decision"].to_string().contains("proposals"));
    assert!(!value["pre_action"].to_string().contains("weight_split"));
}

#[test]
fn pre_action_signed_zero_body_copy_cannot_override_frame() {
    let frame = perception_fixture();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        neural_selection_fixture(&frame, 1),
        command_for_candidate(&frame.candidates()[1]),
    )
    .unwrap();
    let patch = seal_with_decision(frame, decision).unwrap();
    let mut value = serde_json::to_value(patch).unwrap();
    value["pre_action"]["body"]["pose"]["translation"]["x"] =
        serde_json::to_value(-0.0_f32).unwrap();

    let tampered: ExperiencePatch = serde_json::from_value(value).unwrap();

    assert!(tampered.validate_contract().is_ok());
    assert_eq!(
        tampered.pre_action().body().pose.translation.x.to_bits(),
        0.0_f32.to_bits()
    );
    assert_eq!(
        tampered.pre_action().body().pose.translation.x.to_bits(),
        tampered
            .pre_action()
            .perception()
            .body()
            .pose
            .translation
            .x
            .to_bits()
    );
}

#[test]
fn pre_action_signed_zero_homeostasis_copy_cannot_override_frame() {
    let frame = perception_fixture();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        neural_selection_fixture(&frame, 1),
        command_for_candidate(&frame.candidates()[1]),
    )
    .unwrap();
    let patch = seal_with_decision(frame, decision).unwrap();
    let mut value = serde_json::to_value(patch).unwrap();
    value["pre_action"]["homeostasis"]["drives"]["pain"] = serde_json::to_value(-0.0_f32).unwrap();

    let tampered: ExperiencePatch = serde_json::from_value(value).unwrap();

    assert!(tampered.validate_contract().is_ok());
    assert_eq!(
        tampered.pre_action().homeostasis().drives.pain.to_bits(),
        0.0_f32.to_bits()
    );
    assert_eq!(
        tampered.pre_action().homeostasis().drives.pain.to_bits(),
        tampered
            .pre_action()
            .perception()
            .homeostasis()
            .drives
            .pain
            .to_bits()
    );
}

#[test]
fn neural_patch_consumers_use_common_evidence_without_baseline_payloads() {
    let frame = perception_fixture();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence_id(),
        phenotype_hash(),
        7,
        1,
        &frame,
        neural_selection_fixture(&frame, 1),
        command_for_candidate(&frame.candidates()[1]),
    )
    .unwrap();
    let patch = seal_with_decision(frame, decision).unwrap();

    assert_eq!(
        patch.decision().neural_evidence().unwrap().frame_digest,
        patch.pre_action().frame_digest().unwrap()
    );

    let mut memory = MemoryBank::new(
        MemoryBankConfig::new(4, 16, 2, 0.0, Confidence::new(0.05).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(memory.insert_from_patch(&patch).unwrap().raw(), 1);

    let mut topology = TopologySidecar::new(organism(), TopologicalMapConfig::default()).unwrap();
    let rejected = topology.observe_sealed_patch(&patch);
    assert!(rejected.rejected_invalid);
    assert_eq!(rejected.before_digest, rejected.after_digest);

    let packed = ExperiencePacker::default().pack(&patch).unwrap();
    assert_eq!(packed.frame.selected_action_id, 101);
    assert_eq!(
        packed.frame.side_buffer_spans.ranked_action_proposals.count,
        0
    );
}

#[derive(Serialize)]
struct LegacyPreActionSnapshotV1 {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    tick: Tick,
    brain_class_id: BrainClassId,
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
    sensory_abi_version: alife_core::SensoryAbiVersion,
    chemistry_schema_version: u16,
    body_pose: Pose,
    body_velocity: Velocity,
    homeostasis: HomeostaticSnapshot,
    sensory: SensorySnapshot,
    memory_expectancy: MemoryExpectancySnapshot,
}

#[derive(Serialize)]
struct LegacyDecisionSnapshotV1 {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    decision_tick: Tick,
    action_abi_version: u16,
    proposals: Vec<ActionProposal>,
    selected_action: ActionCommand,
    rejected_top_proposal: Option<alife_core::RankedActionProposal>,
    ranked_top_proposals: Vec<alife_core::RankedActionProposal>,
    arbitration_trace: alife_core::ActionArbitrationTrace,
    confidence: Confidence,
    status: alife_core::ActionDecisionStatus,
}

#[derive(Serialize)]
struct LegacyExperiencePatchHeaderV1 {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    world_tick: Tick,
    phase: ExperiencePatchPhase,
}

#[derive(Serialize)]
struct LegacyExperiencePatchV1 {
    header: LegacyExperiencePatchHeaderV1,
    pre_action: LegacyPreActionSnapshotV1,
    decision: LegacyDecisionSnapshotV1,
    outcome: PostActionOutcome,
}

fn legacy_patch_v1() -> LegacyExperiencePatchV1 {
    let tick = Tick::new(7);
    let spec = brain_spec();
    let genome = genome(&spec);
    let sensory = SensorySnapshot::new(
        organism(),
        tick,
        Vec3f::ZERO,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    let homeostasis = HomeostaticSnapshot::baseline(tick);
    let proposal = ActionProposal::new(
        ActionId(101),
        ActionKind::Move,
        0.75,
        Confidence::new(0.8).unwrap(),
        None,
        0,
        ActionTarget::new(Some(WorldEntityId(55)), Some(Vec3f::new(1.0, 0.0, 2.0))),
        NormalizedScalar::new(0.4).unwrap(),
    )
    .unwrap();
    let proposals = vec![proposal];
    let action_decision = heuristic_baseline_arbitrate(
        organism(),
        &proposals,
        ActionArbitrationConfig {
            default_duration_ticks: DurationTicks::new(2),
            ..ActionArbitrationConfig::default()
        },
    )
    .unwrap();
    let mut outcome = outcome();
    outcome.abi_version = 1;

    LegacyExperiencePatchV1 {
        header: LegacyExperiencePatchHeaderV1 {
            abi_version: 1,
            organism_id: organism(),
            sequence_id: sequence_id(),
            world_tick: tick,
            phase: ExperiencePatchPhase::Sealed,
        },
        pre_action: LegacyPreActionSnapshotV1 {
            abi_version: 1,
            organism_id: organism(),
            sequence_id: sequence_id(),
            tick,
            brain_class_id: spec.id,
            brain_scale_tier: spec.tier,
            brain_neuron_count: spec.neuron_count,
            max_active_synapses: spec.max_active_synapses,
            max_active_microtiles: spec.max_active_microtiles,
            routing_schema_version: spec.routing_schema_version,
            lobe_layout: spec.lobe_layout.clone(),
            routing_matrix: spec.routing_matrix.clone(),
            genome_id: genome.id,
            genome_schema_version: genome.schema_version,
            development_state: development(&genome, tick),
            weight_split: WeightSplitContract::for_brain_class(
                spec.id,
                spec.max_active_synapses,
                spec.max_active_microtiles,
                genome.genetic_prior_seed,
            )
            .unwrap(),
            sensory_abi_version: sensory.abi_version,
            chemistry_schema_version: homeostasis.schema_version,
            body_pose: Pose::IDENTITY,
            body_velocity: Velocity::ZERO,
            homeostasis,
            sensory,
            memory_expectancy: MemoryExpectancySnapshot::neutral(),
        },
        decision: LegacyDecisionSnapshotV1 {
            abi_version: 1,
            organism_id: organism(),
            sequence_id: sequence_id(),
            decision_tick: tick,
            action_abi_version: ActionCommand::ABI_VERSION,
            proposals,
            selected_action: action_decision.selected,
            rejected_top_proposal: action_decision.rejected_top_proposal,
            ranked_top_proposals: action_decision.ranked_top_proposals,
            arbitration_trace: action_decision.trace,
            confidence: action_decision.selected.confidence,
            status: action_decision.status,
        },
        outcome,
    }
}

fn legacy_proposals(count: u32) -> Vec<ActionProposal> {
    (0..count)
        .map(|index| {
            ActionProposal::new(
                ActionId(1_000 + index),
                ActionKind::Move,
                index as f32 / count.max(1) as f32,
                Confidence::new(0.8).unwrap(),
                None,
                0,
                ActionTarget::new(Some(WorldEntityId(55)), Some(Vec3f::new(1.0, 0.0, 2.0))),
                NormalizedScalar::new(0.4).unwrap(),
            )
            .unwrap()
        })
        .collect()
}

fn install_legacy_decision(
    legacy: &mut LegacyExperiencePatchV1,
    proposals: Vec<ActionProposal>,
    config: ActionArbitrationConfig,
) -> alife_core::ActionDecision {
    let action_decision = heuristic_baseline_arbitrate(organism(), &proposals, config).unwrap();
    legacy.decision.proposals = proposals;
    legacy.decision.selected_action = action_decision.selected;
    legacy.decision.rejected_top_proposal = action_decision.rejected_top_proposal;
    legacy.decision.ranked_top_proposals = action_decision.ranked_top_proposals.clone();
    legacy.decision.arbitration_trace = action_decision.trace.clone();
    legacy.decision.confidence = action_decision.selected.confidence;
    legacy.decision.status = action_decision.status;
    action_decision
}

fn migrate_legacy_patch(legacy: LegacyExperiencePatchV1) -> ExperiencePatch {
    let value = serde_json::to_value(legacy).unwrap();
    serde_json::from_value(value).unwrap()
}

fn assert_contiguous_candidates(patch: &ExperiencePatch) {
    assert!(patch
        .pre_action()
        .perception()
        .candidates()
        .iter()
        .enumerate()
        .all(|(index, candidate)| candidate.candidate_index as usize == index));
}

#[test]
fn legacy_v1_patch_deserializes_as_explicit_heuristic_baseline_evidence() {
    let value = serde_json::to_value(legacy_patch_v1()).unwrap();
    let migrated: ExperiencePatch = serde_json::from_value(value).unwrap();

    assert_eq!(migrated.header().abi_version, 3);
    assert_eq!(
        migrated.header().sensor_profile,
        migrated.pre_action().perception().profile_provenance()
    );
    assert_eq!(
        migrated.pre_action().evidence_kind(),
        EvidenceKind::HeuristicBaseline
    );
    assert_eq!(
        migrated.decision().evidence_kind(),
        EvidenceKind::HeuristicBaseline
    );
    assert_eq!(
        migrated
            .pre_action()
            .heuristic_evidence()
            .unwrap()
            .brain_class_id,
        brain_spec().id
    );
    assert_eq!(
        migrated
            .decision()
            .heuristic_evidence()
            .unwrap()
            .proposals
            .len(),
        1
    );
    assert_eq!(migrated.pre_action().perception().candidates().len(), 1);
    assert_eq!(
        migrated.pre_action().body().pose.translation.x.to_bits(),
        0.0_f32.to_bits()
    );
    assert_eq!(
        migrated.pre_action().homeostasis().drives.pain.to_bits(),
        0.0_f32.to_bits()
    );
    let reserialized = serde_json::to_value(&migrated).unwrap();
    assert!(reserialized["pre_action"].get("body").is_none());
    assert!(reserialized["pre_action"].get("homeostasis").is_none());
    assert!(migrated.validate_contract().is_ok());
}

#[test]
fn legacy_v1_migration_preserves_all_33_proposals_with_bounded_selected_candidate() {
    let mut legacy = legacy_patch_v1();
    let proposals = legacy_proposals(33);
    let action_decision = install_legacy_decision(
        &mut legacy,
        proposals.clone(),
        ActionArbitrationConfig {
            default_duration_ticks: DurationTicks::new(2),
            ..ActionArbitrationConfig::default()
        },
    );
    let selected_action_id = action_decision.selected.action_id;
    assert_eq!(selected_action_id, proposals[32].action_id);

    let migrated = migrate_legacy_patch(legacy);
    let heuristic = migrated.decision().heuristic_evidence().unwrap();
    let candidates = migrated.pre_action().perception().candidates();

    assert_eq!(heuristic.proposals, proposals);
    assert_eq!(
        heuristic.ranked_top_proposals,
        action_decision.ranked_top_proposals
    );
    assert_eq!(heuristic.arbitration_trace, action_decision.trace);
    assert_eq!(candidates.len(), 32);
    assert_contiguous_candidates(&migrated);
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.action_id)
            .collect::<Vec<_>>(),
        proposals[..31]
            .iter()
            .map(|proposal| proposal.action_id)
            .chain(std::iter::once(selected_action_id))
            .collect::<Vec<_>>()
    );
    assert!(candidates
        .iter()
        .any(|candidate| candidate.action_id == selected_action_id));
    assert!(migrated.validate_contract().is_ok());
}

#[test]
fn legacy_v1_migration_handles_candidate_boundaries_table() {
    #[derive(Clone, Copy)]
    enum SelectionCase {
        Highest,
        Preferred(usize),
        Fallback,
        NotRepresented,
    }

    let cases = [
        ("empty", 0, SelectionCase::Fallback),
        ("fewer", 3, SelectionCase::Highest),
        ("exactly-32", 32, SelectionCase::Highest),
        ("selected-in-first-32", 33, SelectionCase::Preferred(7)),
        ("fallback", 33, SelectionCase::Fallback),
        ("not-represented", 33, SelectionCase::NotRepresented),
    ];

    for (name, count, selection_case) in cases {
        let mut legacy = legacy_patch_v1();
        let unrepresented_command = legacy.decision.selected_action;
        let mut proposals = legacy_proposals(count);
        if let SelectionCase::Preferred(index) = selection_case {
            proposals[index].score = 2.0;
        }
        let decision = install_legacy_decision(
            &mut legacy,
            proposals.clone(),
            ActionArbitrationConfig {
                min_score: if matches!(selection_case, SelectionCase::Fallback) {
                    2.0
                } else {
                    ActionArbitrationConfig::default().min_score
                },
                ..ActionArbitrationConfig::default()
            },
        );
        let selected_command = if matches!(selection_case, SelectionCase::NotRepresented) {
            legacy.decision.selected_action = unrepresented_command;
            legacy.decision.confidence = unrepresented_command.confidence;
            legacy.decision.status = alife_core::ActionDecisionStatus::Selected;
            legacy
                .decision
                .arbitration_trace
                .wta_result
                .selected_proposal_index = None;
            legacy
                .decision
                .arbitration_trace
                .wta_result
                .selected_action_id = Some(unrepresented_command.action_id);
            unrepresented_command
        } else {
            decision.selected
        };

        let expected_action_ids = match selection_case {
            SelectionCase::Highest => proposals
                .iter()
                .take(32)
                .map(|proposal| proposal.action_id)
                .collect::<Vec<_>>(),
            SelectionCase::Preferred(index) => {
                assert_eq!(
                    decision.trace.wta_result.selected_proposal_index,
                    Some(index),
                    "{name}"
                );
                proposals[..32]
                    .iter()
                    .map(|proposal| proposal.action_id)
                    .collect::<Vec<_>>()
            }
            SelectionCase::Fallback | SelectionCase::NotRepresented => proposals
                .iter()
                .take(31)
                .map(|proposal| proposal.action_id)
                .chain(std::iter::once(selected_command.action_id))
                .collect::<Vec<_>>(),
        };

        let migrated = migrate_legacy_patch(legacy);
        let heuristic = migrated.decision().heuristic_evidence().unwrap();
        let candidates = migrated.pre_action().perception().candidates();
        assert_eq!(heuristic.proposals, proposals, "{name}");
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate.action_id)
                .collect::<Vec<_>>(),
            expected_action_ids,
            "{name}"
        );
        assert!(candidates.len() <= 32, "{name}");
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.action_id == selected_command.action_id),
            "{name}"
        );
        assert_contiguous_candidates(&migrated);
        assert!(migrated.validate_contract().is_ok(), "{name}");
    }
}
