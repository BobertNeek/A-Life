use alife_core::{
    cpu_reference_arbitrate, ActionArbitrationConfig, ActionId, ActionKind, ActionProposal,
    ActionTarget, AffordanceBits, BrainClassSpec, BrainGenome, BrainScaleTier, CognitiveEdge,
    ConceptBindings, ConceptCell, ConceptCellId, Confidence, ContextFeatureFlags,
    ContradictionType, CuriosityBias, DevelopmentState, DriveBinding, DriveChannel, DurationTicks,
    EdgeRelationKind, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    GapResolutionStatus, GaussianClusterId, GaussianContextRef, GaussianSalienceEntry, HeardToken,
    HomeostaticDelta, HomeostaticSnapshot, Intensity, LobeKind, NormalizedScalar, OrganismId,
    PhysicalActionOutcome, PhysicalContactKind, PostActionOutcome, ScaffoldContractError,
    SemanticContextRef, SemanticSalienceEntry, SensoryChannels, SensorySnapshot, SignedValence,
    SocialAgentSnapshot, TeacherFeedbackObservation, TeacherPerceptionChannel, Tick,
    TopologicalMap, TopologicalMapConfig, Validate, Vec3f, Velocity, WeightSplitContract,
    WorldEntityId, SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT,
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

fn sensory(tick: Tick, organism_id: OrganismId, target: WorldEntityId) -> SensorySnapshot {
    let mut visual = [0.0; SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT];
    visual[0] = 0.9;
    visual[7] = 0.35;
    let channels = SensoryChannels::try_from_groups(
        visual,
        [0.25; alife_core::SENSORY_AUDITORY_CHANNEL_COUNT],
        [0.1; alife_core::SENSORY_SMELL_CHANNEL_COUNT],
        [0.2; alife_core::SENSORY_TACTILE_CHANNEL_COUNT],
        NormalizedScalar::new(0.05).unwrap(),
        NormalizedScalar::new(0.7).unwrap(),
        AffordanceBits::FOOD | AffordanceBits::TOOL,
    )
    .unwrap();
    let mut snapshot = SensorySnapshot::new(
        organism_id,
        tick,
        Vec3f::new(1.0, 2.0, 3.0),
        channels,
        Default::default(),
    )
    .unwrap();
    snapshot.context_streams.vocal_tokens[0] = Some(HeardToken {
        speaker_id: Some(OrganismId(8)),
        source_entity: Some(target),
        token_id: 101,
        source_position: Vec3f::new(1.5, 2.0, 3.0),
        confidence: Confidence::new(0.8).unwrap(),
        teacher_channel: Some(TeacherPerceptionChannel::Hearing),
    });
    snapshot.language_context.heard_tokens[0] = Some(HeardToken {
        speaker_id: Some(OrganismId(8)),
        source_entity: Some(target),
        token_id: 202,
        source_position: Vec3f::new(1.25, 2.0, 3.0),
        confidence: Confidence::new(0.7).unwrap(),
        teacher_channel: Some(TeacherPerceptionChannel::Writing),
    });
    snapshot.social_context.nearest_agents[0] = Some(SocialAgentSnapshot {
        agent_id: OrganismId(8),
        body_entity: Some(WorldEntityId(70)),
        relative_position: Vec3f::new(0.5, 0.0, 1.0),
        gaze_direction: Vec3f::new(0.0, 0.0, 1.0),
        orientation_forward: Vec3f::new(0.0, 0.0, 1.0),
        affinity: SignedValence::new(0.3).unwrap(),
        proximity: NormalizedScalar::new(0.75).unwrap(),
    });
    snapshot.semantic_context = Some(SemanticContextRef {
        feature_flags: ContextFeatureFlags::SEMANTIC_CODES,
        confidence: Confidence::new(0.65).unwrap(),
        compressed_codes: vec![alife_core::CompressedSemanticCode {
            codebook_id: 1,
            code: 44,
            salience: NormalizedScalar::new(0.6).unwrap(),
        }],
        salience: vec![SemanticSalienceEntry {
            concept_id: ConceptCellId(33),
            salience: NormalizedScalar::new(0.7).unwrap(),
        }],
    });
    snapshot.gaussian_context = Some(GaussianContextRef {
        egocentric_bin_hash: 55,
        feature_flags: ContextFeatureFlags::GAUSSIAN_CLUSTERS,
        confidence: Confidence::new(0.75).unwrap(),
        clusters: vec![GaussianSalienceEntry {
            cluster_id: GaussianClusterId(66),
            salience: NormalizedScalar::new(0.5).unwrap(),
            distance_meters: 2.5,
        }],
    });
    snapshot.validate_contract().unwrap();
    snapshot
}

fn pre_action_at(
    tick: Tick,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    target: WorldEntityId,
) -> alife_core::PreActionSnapshot {
    let spec = brain_spec();
    let genome = genome(&spec);
    alife_core::PreActionSnapshot::new(
        organism_id,
        sequence_id,
        tick,
        spec.clone(),
        genome.clone(),
        development(&genome),
        weight_split(&spec, &genome),
        alife_core::Pose {
            translation: Vec3f::new(1.0, 2.0, 3.0),
            rotation: alife_core::Quatf::IDENTITY,
        },
        Velocity::ZERO,
        HomeostaticSnapshot::baseline(tick),
        sensory(tick, organism_id, target),
        alife_core::MemoryExpectancySnapshot {
            expected_valence: SignedValence::new(0.25).unwrap(),
            predicted_drive_delta: alife_core::DriveDelta::zero(),
            affordance_bias: NormalizedScalar::new(0.2).unwrap(),
            danger_bias: NormalizedScalar::new(0.1).unwrap(),
            safety_bias: NormalizedScalar::new(0.4).unwrap(),
            salience_hint: NormalizedScalar::new(0.3).unwrap(),
        },
    )
    .unwrap()
}

fn proposal(action_id: u32, kind: ActionKind, score: f32, target: WorldEntityId) -> ActionProposal {
    ActionProposal::new(
        ActionId::new(action_id).unwrap(),
        kind,
        score,
        Confidence::new(0.8).unwrap(),
        None,
        0b101,
        ActionTarget::new(Some(target), Some(Vec3f::new(0.0, 0.0, 1.0))),
        NormalizedScalar::new(0.5).unwrap(),
    )
    .unwrap()
    .with_intensity(Intensity::new(0.7).unwrap())
}

fn decision_at(
    tick: Tick,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    target: WorldEntityId,
) -> alife_core::DecisionSnapshot {
    let proposals = vec![
        proposal(300, ActionKind::Move, 0.35, target),
        proposal(400, ActionKind::Interact, 0.75, target),
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

#[allow(clippy::too_many_arguments)]
fn outcome_at(
    tick: Tick,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    target: WorldEntityId,
    success: bool,
    reward: f32,
    prediction_error: f32,
    contradiction: bool,
) -> PostActionOutcome {
    let mut outcome = PostActionOutcome::new(
        organism_id,
        sequence_id,
        tick,
        success,
        PhysicalActionOutcome {
            contact: if success {
                PhysicalContactKind::Touch
            } else {
                PhysicalContactKind::Blocked
            },
            target_entity: Some(target),
            displacement: Vec3f::new(0.0, 0.0, if success { 0.25 } else { 0.0 }),
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta::zero(),
        SignedValence::new(reward).unwrap(),
        NormalizedScalar::new(if success { 0.05 } else { 0.6 }).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        SignedValence::new(-0.1).unwrap(),
        NormalizedScalar::new(prediction_error).unwrap(),
    )
    .unwrap();
    outcome.contradiction_observed = contradiction;
    outcome.teacher_feedback = Some(TeacherFeedbackObservation {
        channel: TeacherPerceptionChannel::Hearing,
        source_entity: Some(WorldEntityId(77)),
        valence: SignedValence::new(reward).unwrap(),
        confidence: Confidence::new(0.9).unwrap(),
    });
    outcome.validate_contract().unwrap();
    outcome
}

fn patch(
    sequence_raw: u64,
    tick_raw: u64,
    target: WorldEntityId,
    success: bool,
    reward: f32,
    prediction_error: f32,
    contradiction: bool,
) -> ExperiencePatch {
    let seq = sequence(sequence_raw);
    ExperiencePatchBuilder::new(seq)
        .record_pre_action(pre_action_at(Tick::new(tick_raw), organism(), seq, target))
        .unwrap()
        .record_decision(decision_at(Tick::new(tick_raw), organism(), seq, target))
        .unwrap()
        .record_outcome(outcome_at(
            Tick::new(tick_raw + 1),
            organism(),
            seq,
            target,
            success,
            reward,
            prediction_error,
            contradiction,
        ))
        .unwrap()
        .seal()
        .unwrap()
}

fn map() -> TopologicalMap {
    TopologicalMap::new(TopologicalMapConfig {
        max_concepts: 16,
        max_edges: 32,
        max_simplexes: 16,
        max_unresolved_gaps: 8,
        edge_decay_per_tick: NormalizedScalar::new(0.1).unwrap(),
    })
    .unwrap()
}

#[test]
fn concept_creation_from_sealed_patch_records_grounded_multimodal_bindings() {
    let mut map = map();
    let update = map
        .apply_patch(&patch(99, 10, WorldEntityId(2), true, 0.25, 0.2, false))
        .unwrap();

    assert_eq!(update.primary_concept_id, ConceptCellId(1));
    assert_eq!(
        map.concepts().len(),
        2,
        "target and action concepts are tracked"
    );
    assert_eq!(map.simplexes().len(), 1);

    let concept = map.concept(update.primary_concept_id).unwrap();
    assert!(concept.bindings.objects.contains(&WorldEntityId(2)));
    assert!(concept.bindings.words.contains(&101));
    assert!(concept.bindings.words.contains(&202));
    assert!(concept
        .bindings
        .drives
        .iter()
        .any(|drive| drive.channel == DriveChannel::Curiosity));
    assert!(concept
        .bindings
        .actions
        .iter()
        .any(|action| action.action_id == ActionId(400) && action.kind == ActionKind::Interact));
    assert_eq!(concept.bindings.emotions.observation_count, 1);
    assert_eq!(concept.bindings.locations[0], Vec3f::new(1.0, 2.0, 3.0));
    assert!(concept.bindings.agents.contains(&OrganismId(8)));
    assert!(concept.bindings.affordances.contains(AffordanceBits::FOOD));
    assert!(concept.bindings.semantic_refs.contains(&ConceptCellId(33)));
    assert!(concept
        .bindings
        .cluster_refs
        .contains(&GaussianClusterId(66)));
    assert_eq!(concept.observation_count, 1);
}

#[test]
fn repeated_patch_strengthens_existing_concept_and_edge() {
    let mut map = map();
    let first = map
        .apply_patch(&patch(99, 10, WorldEntityId(2), true, 0.25, 0.2, false))
        .unwrap();
    let first_edge_strength = map.edge(first.edge_ids[0]).unwrap().strength.raw();

    let second = map
        .apply_patch(&patch(100, 20, WorldEntityId(2), true, 0.25, 0.2, false))
        .unwrap();
    let strengthened = map.edge(first.edge_ids[0]).unwrap();

    assert_eq!(first.primary_concept_id, second.primary_concept_id);
    assert_eq!(
        map.concept(first.primary_concept_id)
            .unwrap()
            .observation_count,
        2
    );
    assert!(strengthened.strength.raw() > first_edge_strength);
    assert_eq!(strengthened.evidence_count, 2);
}

#[test]
fn contradictory_outcome_creates_unresolved_gap_and_raises_curiosity_bias() {
    let mut map = map();
    map.apply_patch(&patch(99, 10, WorldEntityId(2), true, 0.35, 0.05, false))
        .unwrap();

    let update = map
        .apply_patch(&patch(100, 20, WorldEntityId(2), false, -0.6, 0.9, true))
        .unwrap();

    assert_eq!(update.gap_ids.len(), 1);
    let gap = map.gap(update.gap_ids[0]).unwrap();
    assert_eq!(
        gap.contradiction_type,
        ContradictionType::OutcomeContradiction
    );
    assert_eq!(gap.status, GapResolutionStatus::Open);
    assert!(gap.prediction_error.raw() >= 0.9);
    assert!(gap.curiosity_voltage.raw() > 0.5);

    let biases = map.curiosity_biases();
    assert_eq!(biases.len(), 1);
    assert_eq!(biases[0].gap_id, gap.id);
    assert!(biases[0].salience.raw() >= gap.salience.raw());
}

#[test]
fn bounded_map_behavior_rejects_growth_without_resizing() {
    let mut map = TopologicalMap::new(TopologicalMapConfig {
        max_concepts: 2,
        max_edges: 2,
        max_simplexes: 2,
        max_unresolved_gaps: 1,
        edge_decay_per_tick: NormalizedScalar::new(0.0).unwrap(),
    })
    .unwrap();

    map.apply_patch(&patch(99, 10, WorldEntityId(2), true, 0.25, 0.2, false))
        .unwrap();

    assert_eq!(
        map.apply_patch(&patch(100, 20, WorldEntityId(3), true, 0.25, 0.2, false)),
        Err(ScaffoldContractError::TopologyCapacityExceeded)
    );
}

#[test]
fn repeated_dynamic_observations_summarize_without_binding_capacity_failure() {
    let mut map = TopologicalMap::new(TopologicalMapConfig::default()).unwrap();

    for offset in 0..128 {
        map.apply_patch(&patch(
            200 + offset,
            20 + offset,
            WorldEntityId(2),
            true,
            0.25,
            0.2,
            false,
        ))
        .unwrap();
    }

    let target_concept = map
        .concepts()
        .iter()
        .find(|concept| concept.bindings.objects.contains(&WorldEntityId(2)))
        .expect("target object concept should be retained");
    assert!(
        target_concept.bindings.drives.len() <= 11,
        "drive bindings should be summarized by channel"
    );
    assert!(
        target_concept.bindings.locations.len() <= 32,
        "location samples should stay bounded"
    );
    assert!(
        target_concept.bindings.actions.len() <= 2,
        "action bindings should be summarized by action identity"
    );
}

#[test]
fn id_allocation_is_deterministic_for_same_patch_sequence() {
    let mut first = map();
    let mut second = map();

    for map in [&mut first, &mut second] {
        map.apply_patch(&patch(99, 10, WorldEntityId(2), true, 0.25, 0.2, false))
            .unwrap();
        map.apply_patch(&patch(100, 20, WorldEntityId(3), true, 0.3, 0.25, false))
            .unwrap();
    }

    let first_ids: Vec<_> = first.concepts().iter().map(|concept| concept.id).collect();
    let second_ids: Vec<_> = second.concepts().iter().map(|concept| concept.id).collect();
    assert_eq!(first_ids, second_ids);

    let first_edge_ids: Vec<_> = first.edges().iter().map(|edge| edge.id).collect();
    let second_edge_ids: Vec<_> = second.edges().iter().map(|edge| edge.id).collect();
    assert_eq!(first_edge_ids, second_edge_ids);
}

#[test]
fn edge_decay_and_strengthening_are_bounded_and_deterministic() {
    let mut map = map();
    let update = map
        .apply_patch(&patch(99, 10, WorldEntityId(2), true, 0.25, 0.2, false))
        .unwrap();
    let edge_id = update.edge_ids[0];
    let original = map.edge(edge_id).unwrap().strength.raw();

    map.decay_edges(3).unwrap();
    let decayed = map.edge(edge_id).unwrap().strength.raw();
    assert!(decayed < original);
    assert!(decayed >= 0.0);

    map.apply_patch(&patch(100, 20, WorldEntityId(2), true, 0.25, 0.2, false))
        .unwrap();
    let restrengthened = map.edge(edge_id).unwrap().strength.raw();
    assert!(restrengthened > decayed);
    assert!(restrengthened <= 1.0);
}

#[test]
fn invalid_ids_and_nan_or_out_of_range_values_are_rejected() {
    let bad_bindings = ConceptBindings {
        objects: vec![WorldEntityId::INVALID],
        ..ConceptBindings::default()
    };
    assert_eq!(
        ConceptCell::new(ConceptCellId(1), bad_bindings),
        Err(ScaffoldContractError::InvalidId)
    );

    assert_eq!(
        CognitiveEdge::new(
            ConceptCellId(1),
            ConceptCellId::INVALID,
            EdgeRelationKind::Predicts,
            NormalizedScalar::new(0.2).unwrap(),
            Tick::new(1),
        ),
        Err(ScaffoldContractError::InvalidId)
    );

    let nan_drive = DriveBinding {
        channel: DriveChannel::Hunger,
        value: f32::NAN,
    };
    assert_eq!(
        ConceptCell::new(
            ConceptCellId(1),
            ConceptBindings {
                drives: vec![nan_drive],
                ..ConceptBindings::default()
            },
        ),
        Err(ScaffoldContractError::NonFiniteFloat)
    );

    assert_eq!(
        TopologicalMap::new(TopologicalMapConfig {
            max_concepts: 1,
            max_edges: 1,
            max_simplexes: 1,
            max_unresolved_gaps: 1,
            edge_decay_per_tick: NormalizedScalar(1.5),
        }),
        Err(ScaffoldContractError::ScalarOutOfRange)
    );
}

#[test]
fn topology_curiosity_api_does_not_produce_or_bypass_action_commands() {
    fn returns_biases_only(_: fn(&TopologicalMap) -> Vec<CuriosityBias>) {}
    returns_biases_only(TopologicalMap::curiosity_biases);

    let mut map = map();
    map.apply_patch(&patch(99, 10, WorldEntityId(2), true, 0.35, 0.05, false))
        .unwrap();
    map.apply_patch(&patch(100, 20, WorldEntityId(2), false, -0.6, 0.9, true))
        .unwrap();

    let biases = map.curiosity_biases();
    let bias = &biases[0];
    assert_eq!(bias.source_concepts, vec![ConceptCellId(1)]);
    assert!(bias.salience.raw() > 0.0);
    assert!(bias.curiosity_voltage.raw() > 0.0);
}

#[test]
fn topology_source_uses_no_engine_types_or_action_command_shortcuts() {
    let source = include_str!("../src/topology.rs");

    let forbidden = [
        concat!("be", "vy"),
        concat!("wg", "pu"),
        concat!("Render", "Device"),
        concat!("Render", "Queue"),
        concat!("Action", "Command"),
    ];

    for forbidden in forbidden {
        assert!(
            !source.contains(forbidden),
            "topology core must not reference engine types or public action commands"
        );
    }
}
