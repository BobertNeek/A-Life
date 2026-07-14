use alife_core::{
    cpu_reference_arbitrate, ActionArbitrationConfig, ActionCandidate, ActionId, ActionKind,
    ActionProposal, ActionTarget, BodySnapshot, BrainClassSpec, BrainGenome, BrainScaleTier,
    CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef, ConceptCellId,
    Confidence, ContextFeatureFlags, DevelopmentState, DurationTicks, ExperiencePacker,
    ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId, GaussianClusterId,
    GaussianContextRef, GaussianSalienceEntry, HeardToken, HomeostaticDelta, HomeostaticSnapshot,
    InMemoryPackedExperienceLog, Intensity, LobeKind, MemoryId, MotorPayloadKind, MotorPayloadRef,
    NormalizedScalar, OrganismId, PackedExperienceFrame, PackedExperienceRecord,
    PackedExperienceSink, PackedLogEntryRef, PackedSideBufferKind, PackedSideBufferRecord,
    PerceptionFrame, PhysicalActionOutcome, PhysicalContactKind, PostActionOutcome,
    ScaffoldContractError, SchemaKind, SemanticContextRef, SemanticSalienceEntry, SensorProfile,
    SensoryChannels, SensorySnapshot, SignedValence, SocialAgentSnapshot,
    TeacherFeedbackObservation, TeacherLessonMetadata, TeacherLessonResponseChannel,
    TeacherPerceptionChannel, Tick, Validate, Vec3f, Velocity, VocalizedToken, WeightSplitContract,
    WorldEntityId, PACKED_EXPERIENCE_SCHEMA_VERSION, PACKED_FLAG_HAS_GAUSSIAN_CONTEXT,
    PACKED_FLAG_HAS_SEMANTIC_CONTEXT, PACKED_FLAG_SUCCESS,
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

fn rich_sensory(tick: Tick, organism_id: OrganismId) -> SensorySnapshot {
    let mut sensory = sensory(tick, organism_id);
    sensory.social_context.nearest_agents[0] = Some(SocialAgentSnapshot {
        agent_id: OrganismId(8),
        body_entity: Some(WorldEntityId(70)),
        relative_position: Vec3f::new(0.5, 0.0, 1.0),
        gaze_direction: Vec3f::new(0.0, 0.0, 1.0),
        orientation_forward: Vec3f::new(0.0, 0.0, 1.0),
        affinity: SignedValence::new(0.25).unwrap(),
        proximity: NormalizedScalar::new(0.75).unwrap(),
    });
    sensory.context_streams.vocal_tokens[0] = Some(HeardToken {
        speaker_id: Some(OrganismId(8)),
        source_entity: Some(WorldEntityId(70)),
        token_id: 101,
        source_position: Vec3f::new(0.5, 0.0, 1.0),
        confidence: Confidence::new(0.8).unwrap(),
        teacher_channel: Some(TeacherPerceptionChannel::Hearing),
    });
    sensory.language_context.heard_tokens[0] = Some(HeardToken {
        speaker_id: None,
        source_entity: Some(WorldEntityId(71)),
        token_id: 102,
        source_position: Vec3f::new(0.25, 0.0, 1.25),
        confidence: Confidence::new(0.7).unwrap(),
        teacher_channel: Some(TeacherPerceptionChannel::Writing),
    });
    sensory.language_context.vocalized_token = Some(VocalizedToken {
        token_id: 201,
        confidence: Confidence::new(0.55).unwrap(),
    });
    sensory.semantic_context = Some(SemanticContextRef {
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
    sensory.gaussian_context = Some(GaussianContextRef {
        egocentric_bin_hash: 55,
        feature_flags: ContextFeatureFlags::GAUSSIAN_CLUSTERS
            | ContextFeatureFlags::EGOCENTRIC_BIN_HASH,
        confidence: Confidence::new(0.75).unwrap(),
        clusters: vec![GaussianSalienceEntry {
            cluster_id: GaussianClusterId(66),
            salience: NormalizedScalar::new(0.5).unwrap(),
            distance_meters: 2.5,
        }],
    });
    sensory.validate_contract().unwrap();
    sensory
}

fn pre_action_at(tick: Tick, organism_id: OrganismId, rich: bool) -> alife_core::PreActionSnapshot {
    let spec = brain_spec();
    let genome = genome(&spec);
    let body = BodySnapshot {
        pose: alife_core::Pose {
            translation: Vec3f::new(1.0, 2.0, 3.0),
            rotation: alife_core::Quatf::IDENTITY,
        },
        velocity: Velocity::ZERO,
    };
    let homeostasis = HomeostaticSnapshot::baseline(tick);
    let perception = PerceptionFrame::new(
        organism_id,
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        if rich {
            rich_sensory(tick, organism_id)
        } else {
            sensory(tick, organism_id)
        },
        body,
        homeostasis,
        vec![
            ActionCandidate::new(
                0,
                ActionId(300),
                ActionKind::Move,
                CandidateActionFamily::Approach,
                CandidateObservationRef::None,
                ActionTarget::new(Some(WorldEntityId(1)), Some(Vec3f::new(0.0, 0.0, 1.0))),
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
        sequence(),
        perception,
        spec.clone(),
        genome.clone(),
        development(&genome),
        weight_split(&spec, &genome),
        alife_core::MemoryExpectancySnapshot {
            expected_valence: SignedValence::new(0.15).unwrap(),
            predicted_drive_delta: alife_core::DriveDelta::zero(),
            affordance_bias: NormalizedScalar::new(0.2).unwrap(),
            danger_bias: NormalizedScalar::new(0.1).unwrap(),
            safety_bias: NormalizedScalar::new(0.4).unwrap(),
            salience_hint: NormalizedScalar::new(0.3).unwrap(),
        },
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

fn decision_at(tick: Tick, organism_id: OrganismId, rich: bool) -> alife_core::DecisionSnapshot {
    let mut top = proposal(400, ActionKind::Interact, 0.75, Some(WorldEntityId(2)));
    if rich {
        top = top
            .with_teacher_lesson(Some(TeacherLessonMetadata {
                teacher_entity: Some(WorldEntityId(77)),
                lesson_id: 500,
                response_channel: TeacherLessonResponseChannel::Speech,
            }))
            .with_motor_payload(Some(MotorPayloadRef {
                kind: MotorPayloadKind::Vocal,
                payload_id: 600,
                schema_version: 1,
            }));
    }
    let proposals = vec![
        proposal(300, ActionKind::Move, 0.35, Some(WorldEntityId(1))),
        top,
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

fn outcome_at(tick: Tick, organism_id: OrganismId, rich: bool) -> PostActionOutcome {
    let mut outcome = PostActionOutcome::new(
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
    .unwrap();
    if rich {
        outcome.concept_hints.push(alife_core::ConceptHint {
            concept_id: ConceptCellId(33),
            salience: NormalizedScalar::new(0.6).unwrap(),
            contradiction_observed: false,
        });
        outcome.memory_hints.push(alife_core::MemoryHint {
            memory_id: MemoryId(88),
            salience: NormalizedScalar::new(0.5).unwrap(),
        });
        outcome.teacher_feedback = Some(TeacherFeedbackObservation {
            channel: TeacherPerceptionChannel::Hearing,
            source_entity: Some(WorldEntityId(77)),
            valence: SignedValence::new(0.4).unwrap(),
            confidence: Confidence::new(0.9).unwrap(),
        });
    }
    outcome.validate_contract().unwrap();
    outcome
}

fn patch(rich: bool) -> ExperiencePatch {
    ExperiencePatchBuilder::new(sequence())
        .record_pre_action(pre_action_at(Tick::new(10), organism(), rich))
        .unwrap()
        .record_decision(decision_at(Tick::new(10), organism(), rich))
        .unwrap()
        .record_outcome(outcome_at(Tick::new(11), organism(), rich))
        .unwrap()
        .seal()
        .unwrap()
}

#[test]
fn packed_frame_has_fixed_size_and_schema() {
    const EXPECTED_PACKED_FRAME_SIZE_BYTES: usize = 392;

    assert_eq!(
        core::mem::size_of::<PackedExperienceFrame>(),
        EXPECTED_PACKED_FRAME_SIZE_BYTES
    );
    assert_eq!(
        PackedExperienceFrame::SIZE_BYTES,
        EXPECTED_PACKED_FRAME_SIZE_BYTES
    );
    assert_eq!(
        PackedExperienceFrame::SCHEMA_VERSION,
        PACKED_EXPERIENCE_SCHEMA_VERSION
    );
}

#[test]
fn valid_sealed_patch_packs_into_lossy_frame_and_side_buffers() {
    let record = ExperiencePacker::default().pack(&patch(true)).unwrap();
    let frame = record.frame;

    assert_eq!(frame.schema_version, PACKED_EXPERIENCE_SCHEMA_VERSION);
    assert_eq!(frame.organism_id, organism().raw());
    assert_eq!(frame.sequence_id, sequence().raw());
    assert_eq!(frame.pre_action_tick, 10);
    assert_eq!(frame.decision_tick, 10);
    assert_eq!(frame.outcome_tick, 11);
    assert_eq!(frame.selected_action_id, 400);
    assert_eq!(frame.target_entity_id, 2);
    assert_ne!(frame.flags & PACKED_FLAG_SUCCESS, 0);
    assert_ne!(frame.flags & PACKED_FLAG_HAS_SEMANTIC_CONTEXT, 0);
    assert_ne!(frame.flags & PACKED_FLAG_HAS_GAUSSIAN_CONTEXT, 0);
    assert!(frame.salience_summary > 0.0);

    assert!(!record.side_buffers.is_empty());
    assert!(record
        .side_buffers
        .records()
        .iter()
        .any(|record| record.kind == PackedSideBufferKind::HeardToken));
    assert!(record
        .side_buffers
        .records()
        .iter()
        .any(|record| record.kind == PackedSideBufferKind::RankedActionProposal));
    assert!(record.validate_contract().is_ok());
}

#[test]
fn packer_public_api_requires_sealed_experience_patch() {
    fn accepts_only_sealed_patch(
        _: fn(
            &ExperiencePacker,
            &ExperiencePatch,
        ) -> Result<PackedExperienceRecord, ScaffoldContractError>,
    ) {
    }

    accepts_only_sealed_patch(ExperiencePacker::pack);
    assert_eq!(
        ExperiencePatchBuilder::new(sequence()).seal(),
        Err(ScaffoldContractError::MissingPhaseData)
    );
}

#[test]
fn unsupported_packed_schema_version_is_rejected() {
    let mut frame = ExperiencePacker::default()
        .pack(&patch(false))
        .unwrap()
        .frame;
    frame.schema_version = 999;

    assert_eq!(
        frame.validate_contract(),
        Err(ScaffoldContractError::PackedLogSchemaMismatch {
            expected: PACKED_EXPERIENCE_SCHEMA_VERSION,
            actual: 999,
        })
    );
}

#[test]
fn unsupported_embedded_action_abi_version_is_rejected() {
    let mut frame = ExperiencePacker::default()
        .pack(&patch(false))
        .unwrap()
        .frame;
    frame.action_abi_version = 999;

    assert_eq!(
        frame.validate_contract(),
        Err(ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::ActionAbi,
            expected: 2,
            actual: 999,
        })
    );
}

#[test]
fn unchanged_packed_v1_layout_accepts_legacy_experience_v1_records() {
    let mut frame = ExperiencePacker::default()
        .pack(&patch(false))
        .unwrap()
        .frame;
    frame.experience_schema_version = 1;

    assert!(frame.validate_contract().is_ok());
}

#[test]
fn side_buffer_offsets_and_counts_are_deterministic() {
    let packer = ExperiencePacker::default();
    let first = packer.pack(&patch(true)).unwrap();
    let second = packer.pack(&patch(true)).unwrap();

    assert_eq!(
        first.frame.side_buffer_spans,
        second.frame.side_buffer_spans
    );
    assert_eq!(first.side_buffers.records(), second.side_buffers.records());

    let mut expected_offset = 0;
    for span in first.frame.side_buffer_spans.all() {
        assert_eq!(span.offset, expected_offset);
        expected_offset = span.end().unwrap();
    }
    assert_eq!(expected_offset as usize, first.side_buffers.len());
}

#[test]
fn variable_payloads_are_side_buffered_without_changing_frame_size() {
    let minimal = ExperiencePacker::default().pack(&patch(false)).unwrap();
    let rich = ExperiencePacker::default().pack(&patch(true)).unwrap();

    assert_eq!(
        core::mem::size_of_val(&minimal.frame),
        core::mem::size_of_val(&rich.frame)
    );
    assert_eq!(
        core::mem::size_of::<PackedExperienceFrame>(),
        PackedExperienceFrame::SIZE_BYTES
    );
    assert!(rich.side_buffers.len() > minimal.side_buffers.len());
    assert!(rich
        .side_buffers
        .records()
        .iter()
        .any(|record| record.kind == PackedSideBufferKind::SemanticCode));
}

#[test]
fn side_buffer_capacity_overflow_is_rejected_cleanly() {
    let packer = ExperiencePacker::new(0).unwrap();

    assert_eq!(
        packer.pack(&patch(true)),
        Err(ScaffoldContractError::PackedLogSideBufferOverflow)
    );
}

#[test]
fn lossy_metadata_can_be_inspected_and_appended_in_memory() {
    let record = ExperiencePacker::default().pack(&patch(true)).unwrap();
    let summary = record.inspect_lossy().unwrap();

    assert_eq!(summary.organism_id, organism().raw());
    assert_eq!(summary.sequence_id, sequence().raw());
    assert_eq!(summary.selected_action_id, 400);
    assert_eq!(summary.target_entity_id, Some(2));
    assert!(summary.success);

    let mut log = InMemoryPackedExperienceLog::bounded(2, 128).unwrap();
    let entry = log.append(record.clone()).unwrap();
    assert_eq!(
        entry,
        PackedLogEntryRef {
            frame_index: 0,
            side_buffer_offset: 0,
            side_buffer_count: record.side_buffers.len() as u32,
        }
    );
    assert_eq!(log.frames()[0].inspect_lossy().unwrap(), summary);
    assert_eq!(log.side_records(), record.side_buffers.records());
}

#[test]
fn fixed_side_buffer_record_rejects_non_finite_variable_payload_summary() {
    let record = PackedSideBufferRecord::new(
        PackedSideBufferKind::DiagnosticExtra,
        1,
        0,
        [f32::NAN, 0.0, 0.0, 0.0],
        0,
    );

    assert_eq!(record, Err(ScaffoldContractError::NonFiniteFloat));
}
