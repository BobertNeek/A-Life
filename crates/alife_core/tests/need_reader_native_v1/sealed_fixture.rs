//! Qualified store synthetic sealed-source builder, with test visibility only.
use alife_core::*;

#[derive(Clone)]
pub(super) struct Fixture {
    pub(super) sequence: u64,
    pub(super) tick: u64,
    pub(super) organism: OrganismId,
    pub(super) tracked: TrackedObjectId,
    pub(super) entity: WorldEntityId,
    pub(super) profile: SensorProfile,
    pub(super) family: CandidateActionFamily,
    pub(super) position: Vec3f,
    pub(super) loneliness: f32,
    pub(super) measured: bool,
    pub(super) observed: bool,
    pub(super) deltas: [f32; 11],
    pub(super) padding: usize,
    pub(super) utterance: u64,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            sequence: 1,
            tick: 10,
            organism: OrganismId(7),
            tracked: TrackedObjectId(71),
            entity: WorldEntityId(88),
            profile: SensorProfile::GroundedObjectSlotsV1,
            family: CandidateActionFamily::Inspect,
            position: Vec3f::ZERO,
            loneliness: 0.5,
            measured: true,
            observed: true,
            deltas: [0.; 11],
            padding: 0,
            utterance: 0,
        }
    }
}

pub(super) fn fixture(spec: Fixture) -> ExperiencePatch {
    let genome = CreatureGenome::early_mammal_founder(
        301,
        FoundationGeneticIdentity::new(22, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let tick = Tick(spec.tick);
    // Keep developmental age fixed so clock-only fixtures really retain the same hormones.
    let mut before = BiochemistryState::new_with_age(&phenotype, tick, Tick::ZERO).unwrap();
    before.homeostasis.drives = DriveSnapshot {
        hunger: 0.5,
        fatigue: 0.5,
        fear: 0.5,
        pain: 0.5,
        loneliness: spec.loneliness,
        curiosity: 0.5,
        brain_atp: 0.5,
        temperature_stress: 0.5,
        reproductive_drive: 0.5,
        extension: [0.5, 0.5],
    };
    let mut after =
        BiochemistryState::new_with_age(&phenotype, Tick(spec.tick + 1), Tick::ZERO).unwrap();
    after.homeostasis = before.homeostasis;
    after.homeostasis.tick = after.tick;
    let b = before.homeostasis.drives.to_array();
    let d = spec.deltas;
    after.homeostasis.drives = DriveSnapshot {
        hunger: b[0] + d[0],
        fatigue: b[1] + d[1],
        fear: b[2] + d[2],
        pain: b[3] + d[3],
        loneliness: b[4] + d[4],
        curiosity: b[5] + d[5],
        brain_atp: b[6] + d[6],
        temperature_stress: b[7] + d[7],
        reproductive_drive: b[8] + d[8],
        extension: [b[9] + d[9], b[10] + d[10]],
    };
    let slot = GroundedObjectSlotV1 {
        slot_index: 0,
        tracked_object_id: spec.tracked,
        bearing: [0., 0.],
        distance: 0.1,
        relative_velocity: [0.; 3],
        color: [0.2; 3],
        material: [0.3; 3],
        shape: [0.4; 3],
        chemical: [0.; 3],
        contact: 0.,
        proprioception: [0.; 2],
        temperature: 0.,
        terrain: [0.; 2],
        confidence: Confidence::new(0.8).unwrap(),
    };
    let (kind, action) = match spec.family {
        CandidateActionFamily::Contact => (ActionKind::Interact, ActionId(201)),
        CandidateActionFamily::Rest => (ActionKind::Rest, ActionId(2)),
        _ => (ActionKind::Inspect, ActionId(4)),
    };
    let candidate = ActionCandidate::new(
        0,
        action,
        kind,
        spec.family,
        if spec.observed {
            CandidateObservationRef::ObjectSlot(0)
        } else {
            CandidateObservationRef::None
        },
        if spec.family == CandidateActionFamily::Rest {
            ActionTarget::NONE
        } else {
            ActionTarget::new(Some(spec.entity), Some(Vec3f::new(1., 0., 0.)))
        },
        if spec.observed {
            slot.candidate_features().unwrap()
        } else {
            CandidateFeatureVector::zero()
        },
        Confidence::new(0.8).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(1),
    )
    .unwrap();
    let mut sensory = SensorySnapshot::new(
        spec.organism,
        tick,
        spec.position,
        SensoryChannels::ZERO,
        Default::default(),
    )
    .unwrap();
    if spec.utterance != 0 {
        sensory.language_context.heard_tokens[0] = Some(HeardToken {
            utterance_id: UtteranceId::new(spec.utterance).unwrap(),
            sequence_position: 0,
            source_kind: UtteranceSourceKind::Creature,
            speaker_id: Some(OrganismId(8)),
            addressee: Some(spec.organism),
            source_entity: Some(spec.entity),
            token_id: 7,
            source_position: Vec3f::new(1., 0., 0.),
            confidence: Confidence::new(0.8).unwrap(),
            teacher_channel: None,
        });
        sensory.social_context.nearest_agents[0] = Some(SocialAgentSnapshot {
            agent_id: OrganismId(8),
            body_entity: Some(spec.entity),
            relative_position: Vec3f::new(1., 0., 0.),
            gaze_direction: Vec3f::new(-1., 0., 0.),
            orientation_forward: Vec3f::new(-1., 0., 0.),
            affinity: SignedValence::ZERO,
            proximity: NormalizedScalar::new(0.5).unwrap(),
        });
        sensory.semantic_context = Some(SemanticContextRef {
            feature_flags: ContextFeatureFlags::SEMANTIC_CODES,
            confidence: Confidence::new(0.8).unwrap(),
            compressed_codes: vec![CompressedSemanticCode {
                codebook_id: 1,
                code: 7,
                salience: NormalizedScalar::new(0.5).unwrap(),
            }],
            salience: vec![SemanticSalienceEntry {
                concept_id: ConceptCellId(1),
                salience: NormalizedScalar::new(0.5).unwrap(),
            }],
        });
    }
    let frame = PerceptionFrame::new(
        spec.organism,
        tick,
        spec.profile,
        sensory,
        BodySnapshot {
            pose: Pose {
                translation: spec.position,
                ..Pose::IDENTITY
            },
            velocity: Velocity::ZERO,
        },
        before.homeostasis,
        vec![candidate],
        SensorProfileProvenance::new(spec.profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
        if spec.observed { vec![slot] } else { vec![] },
    )
    .unwrap();
    frame.validate_contract().unwrap();
    let sequence = ExperienceSequenceId(spec.sequence);
    let hash = PhenotypeHash([1, 2, 3, 4]);
    let decision = DecisionSnapshot::from_cpu_neural_selection(
        sequence,
        hash,
        spec.sequence,
        0,
        &frame,
        NeuralActionSelection {
            candidate_index: 0,
            logit: 0.,
            confidence: Confidence::new(0.9).unwrap(),
            active_tiles: 0,
            active_synapses: 0,
        },
        candidate
            .to_command(spec.organism, Confidence::new(0.9).unwrap())
            .unwrap(),
    )
    .unwrap();
    let pre = PreActionSnapshot::from_cpu_neural_frame(
        sequence,
        BrainClassSpec::for_tier(BrainScaleTier::Nano512).id,
        hash,
        phenotype.brain_genome.id,
        phenotype.brain_genome.schema_version,
        phenotype.development_state_at(tick).unwrap(),
        frame,
    )
    .unwrap();
    let mut outcome = PostActionOutcome::new(
        spec.organism,
        sequence,
        after.tick,
        true,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: Some(spec.entity),
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::ZERO,
        },
        HomeostaticDelta::zero(),
        SignedValence::new(0.9).unwrap(),
        NormalizedScalar::ZERO,
        NormalizedScalar::ZERO,
        SignedValence::ZERO,
        NormalizedScalar::ZERO,
    )
    .unwrap();
    if spec.measured {
        outcome = outcome
            .with_measured_physiology(MeasuredPhysiologyTransition::new(before, after).unwrap())
            .unwrap();
    }
    outcome.memory_hints = vec![
        MemoryHint {
            memory_id: MemoryId(1),
            salience: NormalizedScalar::ZERO
        };
        spec.padding
    ];
    ExperiencePatchBuilder::new(sequence)
        .record_pre_action(pre)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}
