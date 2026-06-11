use alife_core::{
    cpu_reference_arbitrate, ActionArbitrationConfig, ActionId, ActionKind, ActionProposal,
    ActionTarget, BrainClassSpec, BrainGenome, BrainScaleTier, Confidence, DevelopmentState,
    DurationTicks, ExperiencePatchBuilder, ExperiencePatchPhase, ExperienceSequenceId,
    HomeostaticDelta, HomeostaticSnapshot, Intensity, LobeKind, MemoryExpectancySnapshot,
    NormalizedScalar, OrganismId, PhysicalActionOutcome, PhysicalContactKind, Pose,
    PostActionOutcome, ScaffoldContractError, SchemaKind, SensoryChannels, SensorySnapshot,
    SignedValence, Tick, Validate, Vec3f, Velocity, WeightSplitContract, WorldEntityId,
};

fn organism() -> OrganismId {
    OrganismId(7)
}

fn sequence() -> ExperienceSequenceId {
    ExperienceSequenceId(99)
}

fn brain_spec() -> BrainClassSpec {
    BrainClassSpec::for_tier(BrainScaleTier::Standard2048)
}

fn genome(spec: &BrainClassSpec) -> BrainGenome {
    BrainGenome::scaffold(42, spec.id)
}

fn development(genome: &BrainGenome) -> DevelopmentState {
    DevelopmentState::new(
        genome.id,
        Tick::new(120),
        NormalizedScalar::new(0.35).unwrap(),
    )
    .with_enabled_lobes([
        LobeKind::SensoryGrounding,
        LobeKind::CoreAssociation,
        LobeKind::MotorArbitration,
    ])
}

fn weight_split(spec: &BrainClassSpec, genome: &BrainGenome) -> WeightSplitContract {
    WeightSplitContract::for_brain_class(
        spec.id,
        spec.max_active_synapses,
        spec.max_active_microtiles,
        genome.genetic_prior_seed,
    )
    .unwrap()
}

fn sensory(tick: Tick, organism_id: OrganismId) -> SensorySnapshot {
    SensorySnapshot::new(
        organism_id,
        tick,
        Vec3f::new(1.0, 2.0, 3.0),
        SensoryChannels::default(),
        Default::default(),
    )
    .unwrap()
}

fn pre_action_at(tick: Tick, organism_id: OrganismId) -> alife_core::PreActionSnapshot {
    let spec = brain_spec();
    let genome = genome(&spec);
    alife_core::PreActionSnapshot::new(
        organism_id,
        sequence(),
        tick,
        spec.clone(),
        genome.clone(),
        development(&genome),
        weight_split(&spec, &genome),
        Pose::IDENTITY,
        Velocity::ZERO,
        HomeostaticSnapshot::baseline(tick),
        sensory(tick, organism_id),
        MemoryExpectancySnapshot::neutral(),
    )
    .unwrap()
}

fn proposal(
    action_id: u32,
    kind: ActionKind,
    score: f32,
    target: Option<WorldEntityId>,
) -> ActionProposal {
    ActionProposal::new(
        ActionId::new(action_id).unwrap(),
        kind,
        score,
        Confidence::new(0.8).unwrap(),
        None,
        0b101,
        ActionTarget::new(target, Some(Vec3f::new(0.0, 0.0, 1.0))),
        NormalizedScalar::new(0.5).unwrap(),
    )
    .unwrap()
    .with_intensity(Intensity::new(0.7).unwrap())
}

fn decision_at(tick: Tick, organism_id: OrganismId) -> alife_core::DecisionSnapshot {
    let proposals = vec![
        proposal(300, ActionKind::Move, 0.35, Some(WorldEntityId(1))),
        proposal(400, ActionKind::Interact, 0.75, Some(WorldEntityId(2))),
    ];
    let decision = cpu_reference_arbitrate(
        organism_id,
        &proposals,
        ActionArbitrationConfig {
            default_duration_ticks: DurationTicks::new(4),
            ..ActionArbitrationConfig::default()
        },
    )
    .unwrap();

    alife_core::DecisionSnapshot::from_action_decision(sequence(), tick, proposals, decision)
        .unwrap()
}

fn outcome_at(tick: Tick, organism_id: OrganismId) -> PostActionOutcome {
    PostActionOutcome::new(
        organism_id,
        sequence(),
        tick,
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Touch,
            target_entity: Some(WorldEntityId(2)),
            displacement: Vec3f::new(0.0, 0.0, 0.25),
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta::zero(),
        SignedValence::new(0.25).unwrap(),
        NormalizedScalar::new(0.05).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        SignedValence::new(-0.1).unwrap(),
        NormalizedScalar::new(0.2).unwrap(),
    )
    .unwrap()
}

#[test]
fn valid_three_phase_patch_seals_and_exposes_learning_view() {
    let patch = ExperiencePatchBuilder::new(sequence())
        .record_pre_action(pre_action_at(Tick::new(10), organism()))
        .unwrap()
        .record_decision(decision_at(Tick::new(10), organism()))
        .unwrap()
        .record_outcome(outcome_at(Tick::new(11), organism()))
        .unwrap()
        .seal()
        .unwrap();

    assert_eq!(patch.header().phase, ExperiencePatchPhase::Sealed);
    assert_eq!(
        patch.phase_sequence(),
        [
            ExperiencePatchPhase::PreActionSnapshot,
            ExperiencePatchPhase::DecisionSnapshot,
            ExperiencePatchPhase::PostActionOutcome,
            ExperiencePatchPhase::Sealed,
        ]
    );

    let view = patch.as_learning_view();
    assert_eq!(view.pre_action().organism_id, organism());
    assert_eq!(view.pre_action().sensory.semantic_context, None);
    assert_eq!(view.pre_action().sensory.gaussian_context, None);
    assert_eq!(view.decision().selected_action.action_id, ActionId(400));
    assert_eq!(view.decision().proposals.len(), 2);
    assert!(view.outcome().success);
    assert_eq!(view.outcome().reward_valence.raw(), 0.25);
    assert!(patch.validate_contract().is_ok());
}

#[test]
fn builder_rejects_missing_and_unordered_phases() {
    assert_eq!(
        ExperiencePatchBuilder::new(sequence()).seal(),
        Err(ScaffoldContractError::MissingPhaseData)
    );

    assert_eq!(
        ExperiencePatchBuilder::new(sequence())
            .record_decision(decision_at(Tick::new(10), organism())),
        Err(ScaffoldContractError::UnorderedExperiencePhase)
    );

    assert_eq!(
        ExperiencePatchBuilder::new(sequence())
            .record_pre_action(pre_action_at(Tick::new(10), organism()))
            .unwrap()
            .record_outcome(outcome_at(Tick::new(11), organism())),
        Err(ScaffoldContractError::UnorderedExperiencePhase)
    );
}

#[test]
fn mismatched_creature_ids_and_non_monotonic_ticks_are_rejected() {
    assert_eq!(
        ExperiencePatchBuilder::new(sequence())
            .record_pre_action(pre_action_at(Tick::new(10), organism()))
            .unwrap()
            .record_decision(decision_at(Tick::new(9), organism())),
        Err(ScaffoldContractError::NonMonotonicTick)
    );

    assert_eq!(
        ExperiencePatchBuilder::new(sequence())
            .record_pre_action(pre_action_at(Tick::new(10), organism()))
            .unwrap()
            .record_decision(decision_at(Tick::new(10), OrganismId(8))),
        Err(ScaffoldContractError::MismatchedCreatureId)
    );
}

#[test]
fn incompatible_versions_and_invalid_learning_values_are_rejected() {
    let mut pre = pre_action_at(Tick::new(10), organism());
    pre.abi_version = 999;
    assert_eq!(
        ExperiencePatchBuilder::new(sequence()).record_pre_action(pre),
        Err(ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::Experience,
            expected: 1,
            actual: 999,
        })
    );

    let mut decision = decision_at(Tick::new(10), organism());
    decision.action_abi_version = 999;
    assert_eq!(
        ExperiencePatchBuilder::new(sequence())
            .record_pre_action(pre_action_at(Tick::new(10), organism()))
            .unwrap()
            .record_decision(decision),
        Err(ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::ActionAbi,
            expected: 2,
            actual: 999,
        })
    );

    let mut outcome = outcome_at(Tick::new(11), organism());
    outcome.homeostatic_delta.drives.pain = f32::NAN;
    assert_eq!(
        ExperiencePatchBuilder::new(sequence())
            .record_pre_action(pre_action_at(Tick::new(10), organism()))
            .unwrap()
            .record_decision(decision_at(Tick::new(10), organism()))
            .unwrap()
            .record_outcome(outcome),
        Err(ScaffoldContractError::NonFiniteFloat)
    );
}

#[test]
fn invalid_action_decisions_are_rejected_before_sealing() {
    let mut decision = decision_at(Tick::new(10), organism());
    decision.arbitration_trace.wta_result.selected_action_id = Some(ActionId(999));

    assert_eq!(
        ExperiencePatchBuilder::new(sequence())
            .record_pre_action(pre_action_at(Tick::new(10), organism()))
            .unwrap()
            .record_decision(decision),
        Err(ScaffoldContractError::InvalidActionDecision)
    );
}
