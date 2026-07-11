use alife_core::{
    cpu_reference_arbitrate, ActionArbitrationConfig, ActionCandidate, ActionId, ActionKind,
    ActionProposal, ActionTarget, BodySnapshot, BrainClassSpec, BrainGenome, BrainScaleTier,
    CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef, Confidence,
    DevelopmentState, DriveDelta, DurationTicks, ExperiencePatch, ExperiencePatchBuilder,
    ExperienceSequenceId, HomeostaticDelta, HomeostaticSnapshot, Intensity, LobeKind, MemoryBank,
    MemoryBankConfig, MemoryExpectancy, MemoryOutcomeSummary, MemoryQuery, NormalizedScalar,
    OrganismId, PerceptionFrame, PhysicalActionOutcome, PhysicalContactKind, PostActionOutcome,
    ScaffoldContractError, SchemaKind, SensorProfile, SensoryChannels, SensorySnapshot,
    SignedValence, Tick, Validate, Vec3f, Velocity, WeightSplitContract, WorldEntityId,
};

fn organism() -> OrganismId {
    OrganismId(7)
}

fn sequence(raw: u64) -> ExperienceSequenceId {
    ExperienceSequenceId(raw)
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
        LobeKind::EpisodicMemory,
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

fn sensory(tick: Tick, organism_id: OrganismId, cue: f32) -> SensorySnapshot {
    let mut visual = [0.0; alife_core::SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT];
    visual[0] = cue;
    visual[1] = 1.0 - cue;
    let channels = SensoryChannels::try_from_groups(
        visual,
        [0.0; alife_core::SENSORY_AUDITORY_CHANNEL_COUNT],
        [0.0; alife_core::SENSORY_SMELL_CHANNEL_COUNT],
        [0.0; alife_core::SENSORY_TACTILE_CHANNEL_COUNT],
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(cue).unwrap(),
        Default::default(),
    )
    .unwrap();

    SensorySnapshot::new(
        organism_id,
        tick,
        Vec3f::new(1.0, 2.0, 3.0),
        channels,
        Default::default(),
    )
    .unwrap()
}

fn pre_action_at(
    tick: Tick,
    sequence_id: ExperienceSequenceId,
    organism_id: OrganismId,
    cue: f32,
) -> alife_core::PreActionSnapshot {
    let spec = brain_spec();
    let genome = genome(&spec);
    let body = BodySnapshot {
        pose: alife_core::Pose {
            translation: Vec3f::new(cue, 0.0, 1.0),
            rotation: alife_core::Quatf::IDENTITY,
        },
        velocity: Velocity::ZERO,
    };
    let homeostasis = HomeostaticSnapshot::baseline(tick);
    let perception = PerceptionFrame::new(
        organism_id,
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory(tick, organism_id, cue),
        body,
        homeostasis,
        vec![
            ActionCandidate::new(
                0,
                ActionId(300),
                ActionKind::Move,
                CandidateActionFamily::Approach,
                CandidateObservationRef::None,
                ActionTarget::new(Some(WorldEntityId(2)), Some(Vec3f::new(0.0, 0.0, 1.0))),
                CandidateFeatureVector::zero(),
                Confidence::new(0.8).unwrap(),
                NormalizedScalar::new(0.0).unwrap(),
                DurationTicks::new(4),
                DurationTicks::new(4),
            )
            .unwrap(),
            ActionCandidate::new(
                1,
                ActionId(400),
                ActionKind::Interact,
                CandidateActionFamily::Contact,
                CandidateObservationRef::None,
                ActionTarget::new(Some(WorldEntityId(2)), Some(Vec3f::new(0.0, 0.0, 1.0))),
                CandidateFeatureVector::zero(),
                Confidence::new(0.8).unwrap(),
                NormalizedScalar::new(0.0).unwrap(),
                DurationTicks::new(4),
                DurationTicks::new(4),
            )
            .unwrap(),
        ],
    )
    .unwrap();
    alife_core::PreActionSnapshot::from_heuristic_frame(
        sequence_id,
        perception,
        spec.clone(),
        genome.clone(),
        development(&genome),
        weight_split(&spec, &genome),
        alife_core::MemoryExpectancySnapshot::neutral(),
    )
    .unwrap()
}

fn proposal(action_id: u32, kind: ActionKind, score: f32) -> ActionProposal {
    ActionProposal::new(
        ActionId::new(action_id).unwrap(),
        kind,
        score,
        Confidence::new(0.8).unwrap(),
        None,
        0b101,
        ActionTarget::new(Some(WorldEntityId(2)), Some(Vec3f::new(0.0, 0.0, 1.0))),
        NormalizedScalar::new(0.5).unwrap(),
    )
    .unwrap()
    .with_intensity(Intensity::new(0.7).unwrap())
}

fn decision_at(
    tick: Tick,
    sequence_id: ExperienceSequenceId,
    organism_id: OrganismId,
    action_id: u32,
) -> alife_core::DecisionSnapshot {
    let kind = if action_id == 400 {
        ActionKind::Interact
    } else {
        ActionKind::Move
    };
    let proposals = vec![
        proposal(300, ActionKind::Move, 0.35),
        proposal(action_id, kind, 0.75),
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

    alife_core::DecisionSnapshot::from_action_decision(sequence_id, tick, proposals, decision)
        .unwrap()
}

fn outcome_at(
    tick: Tick,
    sequence_id: ExperienceSequenceId,
    organism_id: OrganismId,
    reward: f32,
    drive_delta: DriveDelta,
) -> PostActionOutcome {
    PostActionOutcome::new(
        organism_id,
        sequence_id,
        tick,
        reward >= 0.0,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::Touch,
            target_entity: Some(WorldEntityId(2)),
            displacement: Vec3f::new(0.0, 0.0, 0.25),
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta {
            drives: drive_delta,
            hormones: alife_core::EndocrineDelta::zero(),
        },
        SignedValence::new(reward).unwrap(),
        NormalizedScalar::new(if reward < 0.0 { 0.4 } else { 0.0 }).unwrap(),
        NormalizedScalar::new(if reward < 0.0 { 0.5 } else { 0.0 }).unwrap(),
        SignedValence::new(-0.1).unwrap(),
        NormalizedScalar::new(0.2).unwrap(),
    )
    .unwrap()
}

fn patch(
    sequence_id: u64,
    pre_tick: u64,
    cue: f32,
    action_id: u32,
    reward: f32,
    drive_delta: DriveDelta,
) -> ExperiencePatch {
    let sequence_id = sequence(sequence_id);
    ExperiencePatchBuilder::new(sequence_id)
        .record_pre_action(pre_action_at(
            Tick::new(pre_tick),
            sequence_id,
            organism(),
            cue,
        ))
        .unwrap()
        .record_decision(decision_at(
            Tick::new(pre_tick),
            sequence_id,
            organism(),
            action_id,
        ))
        .unwrap()
        .record_outcome(outcome_at(
            Tick::new(pre_tick + 1),
            sequence_id,
            organism(),
            reward,
            drive_delta,
        ))
        .unwrap()
        .seal()
        .unwrap()
}

fn config(capacity: usize, top_k: usize) -> MemoryBankConfig {
    MemoryBankConfig::new(capacity, 16, top_k, 0.01, Confidence::new(0.05).unwrap()).unwrap()
}

#[test]
fn empty_recall_returns_neutral_low_confidence_expectancy() {
    let bank = MemoryBank::new(config(4, 2)).unwrap();
    let pre = pre_action_at(Tick::new(20), sequence(20), organism(), 0.8);
    let query = MemoryQuery::from_pre_action(&pre, 16).unwrap();

    let expectancy = bank.recall(&query).unwrap();

    assert_eq!(
        expectancy.expected_valence,
        SignedValence::new(0.0).unwrap()
    );
    assert_eq!(expectancy.predicted_drive_delta, DriveDelta::zero());
    assert_eq!(
        expectancy.predicted_sensory_outcome,
        MemoryOutcomeSummary::neutral()
    );
    assert_eq!(expectancy.confidence, Confidence::new(0.05).unwrap());
    assert!(expectancy.source_memory_ids.is_empty());
}

#[test]
fn insertion_api_accepts_only_sealed_experience_patch() {
    fn accepts_only_sealed_patch(
        _: fn(
            &mut MemoryBank,
            &ExperiencePatch,
        ) -> Result<alife_core::MemoryId, ScaffoldContractError>,
    ) {
    }

    accepts_only_sealed_patch(MemoryBank::insert_from_patch);

    let mut bank = MemoryBank::new(config(2, 1)).unwrap();
    let inserted = bank
        .insert_from_patch(&patch(1, 10, 0.9, 400, 0.6, DriveDelta::zero()))
        .unwrap();

    assert_eq!(inserted, alife_core::MemoryId(1));
    assert_eq!(bank.len(), 1);
}

#[test]
fn top_k_matching_is_deterministic_with_stable_tie_breaking() {
    let mut bank = MemoryBank::new(config(4, 2)).unwrap();
    bank.insert_from_patch(&patch(1, 10, 0.8, 400, 0.2, DriveDelta::zero()))
        .unwrap();
    bank.insert_from_patch(&patch(2, 11, 0.8, 300, 0.4, DriveDelta::zero()))
        .unwrap();
    bank.insert_from_patch(&patch(3, 12, 0.1, 300, -0.4, DriveDelta::zero()))
        .unwrap();

    let query = MemoryQuery::from_pre_action(
        &pre_action_at(Tick::new(20), sequence(20), organism(), 0.8),
        16,
    )
    .unwrap();
    let matches = bank.query(&query).unwrap();

    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].memory_id, alife_core::MemoryId(1));
    assert_eq!(matches[1].memory_id, alife_core::MemoryId(2));
    assert!(matches[0].score >= matches[1].score);
}

#[test]
fn eviction_and_capacity_are_bounded_and_deterministic() {
    let mut bank = MemoryBank::new(config(2, 2)).unwrap();
    bank.insert_from_patch(&patch(1, 10, 0.1, 300, -0.2, DriveDelta::zero()))
        .unwrap();
    bank.insert_from_patch(&patch(2, 11, 0.5, 300, 0.1, DriveDelta::zero()))
        .unwrap();
    bank.insert_from_patch(&patch(3, 12, 0.9, 400, 0.7, DriveDelta::zero()))
        .unwrap();

    assert_eq!(bank.len(), 2);
    assert_eq!(bank.capacity(), 2);
    let ids: Vec<_> = bank
        .records_chronological()
        .into_iter()
        .map(|record| record.memory_id)
        .collect();
    assert_eq!(ids, vec![alife_core::MemoryId(2), alife_core::MemoryId(3)]);
}

#[test]
fn bounded_values_validate_correctly() {
    assert!(MemoryBankConfig::new(0, 16, 1, 0.01, Confidence::new(0.05).unwrap()).is_err());
    assert!(MemoryBankConfig::new(2, 0, 1, 0.01, Confidence::new(0.05).unwrap()).is_err());
    assert!(MemoryBankConfig::new(2, 16, 0, 0.01, Confidence::new(0.05).unwrap()).is_err());

    let invalid = MemoryExpectancy {
        expected_valence: SignedValence::new(0.0).unwrap(),
        predicted_drive_delta: DriveDelta::zero(),
        predicted_sensory_outcome: MemoryOutcomeSummary::neutral(),
        affordance_bias: NormalizedScalar::new(0.0).unwrap(),
        danger_bias: NormalizedScalar::new(0.0).unwrap(),
        safety_bias: NormalizedScalar::new(0.0).unwrap(),
        social_trust_bias: NormalizedScalar::new(0.0).unwrap(),
        social_fear_bias: NormalizedScalar::new(0.0).unwrap(),
        novelty_bias: NormalizedScalar::new(0.0).unwrap(),
        curiosity_bias: NormalizedScalar::new(0.0).unwrap(),
        confidence: Confidence(1.2),
        source_memory_ids: Vec::new(),
    };

    assert_eq!(
        invalid.validate_contract(),
        Err(ScaffoldContractError::ScalarOutOfRange)
    );
}

#[test]
fn predicted_drive_delta_behavior_is_stable() {
    let mut bank = MemoryBank::new(config(2, 1)).unwrap();
    let drive_delta = DriveDelta {
        hunger: -0.4,
        curiosity: 0.25,
        ..DriveDelta::zero()
    };
    bank.insert_from_patch(&patch(1, 10, 0.8, 400, 0.5, drive_delta))
        .unwrap();

    let query = MemoryQuery::from_pre_action(
        &pre_action_at(Tick::new(20), sequence(20), organism(), 0.8),
        16,
    )
    .unwrap();
    let expectancy = bank.recall(&query).unwrap();

    assert_eq!(expectancy.predicted_drive_delta.hunger, -0.4);
    assert_eq!(expectancy.predicted_drive_delta.curiosity, 0.25);
}

#[test]
fn confidence_thresholds_control_no_match_behavior() {
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(2, 16, 1, 0.95, Confidence::new(0.05).unwrap()).unwrap(),
    )
    .unwrap();
    bank.insert_from_patch(&patch(1, 10, 0.1, 400, 0.5, DriveDelta::zero()))
        .unwrap();

    let poor_query = MemoryQuery::from_pre_action(
        &pre_action_at(Tick::new(20), sequence(20), organism(), 0.9),
        16,
    )
    .unwrap();
    let no_match = bank.recall(&poor_query).unwrap();
    assert_eq!(no_match.confidence, Confidence::new(0.05).unwrap());
    assert!(no_match.source_memory_ids.is_empty());

    let good_query = MemoryQuery::from_pre_action(
        &pre_action_at(Tick::new(21), sequence(21), organism(), 0.1),
        16,
    )
    .unwrap();
    let matched = bank.recall(&good_query).unwrap();
    assert!(matched.confidence.raw() > 0.05);
    assert_eq!(matched.source_memory_ids, vec![alife_core::MemoryId(1)]);
}

#[test]
fn memory_of_action_biases_affordance_and_valence_but_cannot_replay_action() {
    let mut bank = MemoryBank::new(config(2, 1)).unwrap();
    bank.insert_from_patch(&patch(1, 10, 0.8, 400, 0.75, DriveDelta::zero()))
        .unwrap();

    let query = MemoryQuery::from_pre_action(
        &pre_action_at(Tick::new(20), sequence(20), organism(), 0.8),
        16,
    )
    .unwrap();
    let expectancy = bank.recall(&query).unwrap();

    let MemoryExpectancy {
        expected_valence,
        predicted_drive_delta,
        predicted_sensory_outcome,
        affordance_bias,
        danger_bias,
        safety_bias,
        social_trust_bias,
        social_fear_bias,
        novelty_bias,
        curiosity_bias,
        confidence,
        source_memory_ids,
    } = expectancy;

    assert!(expected_valence.raw() > 0.5);
    assert!(affordance_bias.raw() > 0.0);
    assert_eq!(predicted_drive_delta, DriveDelta::zero());
    assert!(predicted_sensory_outcome.success_likelihood.raw() > 0.0);
    assert!(danger_bias.raw() <= 1.0);
    assert!(safety_bias.raw() <= 1.0);
    assert!(social_trust_bias.raw() <= 1.0);
    assert!(social_fear_bias.raw() <= 1.0);
    assert!(novelty_bias.raw() <= 1.0);
    assert!(curiosity_bias.raw() <= 1.0);
    assert!(confidence.raw() > 0.05);
    assert_eq!(source_memory_ids, vec![alife_core::MemoryId(1)]);
}

#[test]
fn invalid_creature_tick_and_schema_inputs_are_rejected() {
    let pre = pre_action_at(Tick::new(20), sequence(20), organism(), 0.8);

    let mut invalid_creature = pre.clone();
    invalid_creature.organism_id = OrganismId::INVALID;
    assert_eq!(
        MemoryQuery::from_pre_action(&invalid_creature, 16),
        Err(ScaffoldContractError::InvalidId)
    );

    let mut invalid_schema = pre;
    invalid_schema.abi_version = 999;
    assert_eq!(
        MemoryQuery::from_pre_action(&invalid_schema, 16),
        Err(ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::Experience,
            expected: 2,
            actual: 999,
        })
    );

    let mut bank = MemoryBank::new(config(2, 1)).unwrap();
    bank.insert_from_patch(&patch(1, 20, 0.8, 400, 0.2, DriveDelta::zero()))
        .unwrap();
    assert_eq!(
        bank.insert_from_patch(&patch(2, 19, 0.8, 400, 0.2, DriveDelta::zero())),
        Err(ScaffoldContractError::NonMonotonicTick)
    );
}

#[test]
fn alife_core_memory_contract_stays_engine_independent() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("crate manifest should be readable");

    let forbidden_names = [
        String::from("be") + "vy",
        String::from("av") + "ian",
        String::from("wg") + "pu",
    ];
    for forbidden in forbidden_names {
        assert!(
            !manifest.to_ascii_lowercase().contains(&forbidden),
            "alife_core manifest must not depend on {forbidden}"
        );
    }
}
