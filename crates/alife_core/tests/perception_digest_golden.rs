use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, AffordanceBits, BodySnapshot,
    CandidateActionFamily, CandidateFeatureDigest, CandidateFeatureVector, CandidateObservationRef,
    CompressedSemanticCode, ConceptCellId, Confidence, ContextFeatureFlags, DriveSnapshot,
    DurationTicks, EndocrineSnapshot, EnvironmentStreamEntry, GaussianClusterId,
    GaussianContextRef, GaussianSalienceEntry, HeardToken, HomeostaticSnapshot, NormalizedScalar,
    OrganismId, PerceptionBaseDigest, PerceptionContextBlock, PerceptionContextDigest,
    PerceptionContextKind, PerceptionFrameDigest, PerceptionFrameDraft, Pose, Quatf,
    SemanticContextRef, SemanticSalienceEntry, SensorProfile, SensorProfileProvenance,
    SensoryAbiVersion, SensoryChannels, SensorySnapshot, SignedValence, SocialAgentSnapshot,
    SocialProximityEntry, TeacherPerceptionChannel, Tick, Validate, Vec3f, Velocity,
    VocalizedToken, WorldEntityId,
};

fn organism() -> OrganismId {
    OrganismId(1)
}

fn rich_sensory(tick: Tick) -> SensorySnapshot {
    let mut channels = SensoryChannels::ZERO;
    channels.visual_affordance[0] = 0.1;
    channels.visual_affordance[15] = 0.2;
    channels.auditory_acoustic[0] = 0.3;
    channels.auditory_acoustic[7] = 0.4;
    channels.smell_chemistry[0] = 0.5;
    channels.smell_chemistry[7] = 0.6;
    channels.tactile_contact[0] = 0.7;
    channels.tactile_contact[7] = 0.8;
    channels.pain_signal = NormalizedScalar::new(0.15).unwrap();
    channels.novelty_signal = NormalizedScalar::new(0.25).unwrap();
    channels.nearby_affordances = AffordanceBits::FOOD | AffordanceBits::HAZARD;

    let mut streams = alife_core::ContextStreams {
        atmospheric_temperature_celsius: 18.5,
        ambient_light: NormalizedScalar::new(0.65).unwrap(),
        energy_intake_trend: SignedValence::new(-0.2).unwrap(),
        blood_sugar_trend: SignedValence::new(0.35).unwrap(),
        ..Default::default()
    };
    streams.vocal_tokens[0] = Some(HeardToken {
        speaker_id: Some(OrganismId(8)),
        source_entity: Some(WorldEntityId(70)),
        token_id: 101,
        source_position: Vec3f::new(0.5, -0.25, 1.5),
        confidence: Confidence::new(0.8).unwrap(),
        teacher_channel: Some(TeacherPerceptionChannel::Hearing),
    });
    streams.social_proximity[0] = Some(SocialProximityEntry {
        agent_id: OrganismId(8),
        proximity: NormalizedScalar::new(0.75).unwrap(),
        confidence: Confidence::new(0.85).unwrap(),
    });
    streams.optional_environment[0] = Some(EnvironmentStreamEntry {
        stream_id: 9,
        value: NormalizedScalar::new(0.45).unwrap(),
        confidence: Confidence::new(0.55).unwrap(),
    });

    let mut sensory = SensorySnapshot::new(
        organism(),
        tick,
        Vec3f::new(1.25, -2.5, 3.75),
        channels,
        streams,
    )
    .unwrap();
    sensory.social_context.nearest_agents[0] = Some(SocialAgentSnapshot {
        agent_id: OrganismId(8),
        body_entity: Some(WorldEntityId(70)),
        relative_position: Vec3f::new(0.5, 0.0, 1.0),
        gaze_direction: Vec3f::new(0.0, 0.0, 1.0),
        orientation_forward: Vec3f::new(0.0, 1.0, 0.0),
        affinity: SignedValence::new(-0.25).unwrap(),
        proximity: NormalizedScalar::new(0.75).unwrap(),
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
    sensory.language_context.word_confidence = Confidence::new(0.6).unwrap();
    sensory.language_context.teacher_channel_marker = Some(TeacherPerceptionChannel::Gesture);
    sensory.semantic_context = Some(SemanticContextRef {
        feature_flags: ContextFeatureFlags::SEMANTIC_CODES
            | ContextFeatureFlags::TEACHER_PERCEPTION_MARKER,
        confidence: Confidence::new(0.65).unwrap(),
        compressed_codes: vec![CompressedSemanticCode {
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

fn rich_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    HomeostaticSnapshot::new(
        tick,
        DriveSnapshot {
            hunger: 0.11,
            fatigue: 0.12,
            fear: 0.13,
            pain: 0.14,
            loneliness: 0.15,
            curiosity: 0.16,
            brain_atp: 0.17,
            temperature_stress: 0.18,
            reproductive_drive: 0.19,
            extension: [0.20, 0.21],
        },
        EndocrineSnapshot {
            adrenaline: 0.31,
            cortisol: 0.32,
            dopamine: 0.33,
            oxytocin: 0.34,
            serotonin: 0.35,
            acetylcholine: 0.36,
            learning_modulator: 0.37,
            developmental_hormone: 0.38,
            sleep_pressure: 0.39,
            extension: [0.40, 0.41],
        },
    )
    .unwrap()
}

fn rich_candidates() -> Vec<ActionCandidate> {
    let first = ActionCandidate::new(
        0,
        ActionId(4),
        ActionKind::Inspect,
        CandidateActionFamily::Inspect,
        CandidateObservationRef::None,
        ActionTarget::NONE,
        CandidateFeatureVector::zero(),
        Confidence::new(1.0).unwrap(),
        NormalizedScalar::new(0.1).unwrap(),
        DurationTicks::new(1),
        DurationTicks::new(2),
    )
    .unwrap();
    let mut features = CandidateFeatureVector::zero();
    features.0[0] = -0.5;
    features.0[1] = 0.25;
    features.0[23] = 0.75;
    let second = ActionCandidate::new(
        1,
        ActionId(101),
        ActionKind::Move,
        CandidateActionFamily::Approach,
        CandidateObservationRef::None,
        ActionTarget::new(Some(WorldEntityId(44)), Some(Vec3f::new(1.0, -1.0, 2.0))),
        features,
        Confidence::new(0.9).unwrap(),
        NormalizedScalar::new(0.2).unwrap(),
        DurationTicks::new(2),
        DurationTicks::new(4),
    )
    .unwrap();
    vec![first, second]
}

fn rich_draft() -> PerceptionFrameDraft {
    let tick = Tick::new(7);
    PerceptionFrameDraft::new(
        organism(),
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        rich_sensory(tick),
        BodySnapshot {
            pose: Pose {
                translation: Vec3f::new(4.0, -5.0, 6.0),
                rotation: Quatf::new(0.0, 0.5, 0.0, 0.5),
            },
            velocity: Velocity {
                linear: Vec3f::new(0.1, -0.2, 0.3),
                angular: Vec3f::new(0.4, 0.5, -0.6),
            },
        },
        rich_homeostasis(tick),
        rich_candidates(),
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

#[test]
fn perception_digest_golden_vectors_are_stable() {
    let draft = rich_draft();
    assert_eq!(
        draft.candidates()[1].feature_digest().unwrap(),
        CandidateFeatureDigest([0xb645_d16b_9d15_451e, 0xe695_6e1d_af56_0dc8])
    );
    assert_eq!(
        draft.base_digest(),
        PerceptionBaseDigest([
            0xb9e6_553a_8a7c_02a7,
            0xb1ed_a030_f362_93ca,
            0xc176_ff6c_05ca_f8c6,
            0x02c9_ac28_7401_2c99,
        ])
    );

    let mut context_values = vec![0.0; alife_core::MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE];
    context_values[..3].copy_from_slice(&[0.25, -0.5, 0.75]);
    let context = PerceptionContextBlock::try_new(
        1,
        PerceptionContextKind::EpisodicCandidateV1,
        context_values,
    )
    .unwrap();
    assert_eq!(
        context.canonical_digest(),
        PerceptionContextDigest([
            0xbe2b_a6cd_b50e_7e56,
            0x61bc_faa2_d819_ef59,
            0x63ac_c64d_a541_0f5a,
            0x8bb4_a389_18ad_051a,
        ])
    );

    let frame = draft.finalize(context).unwrap();
    assert_eq!(
        frame.frame_digest(),
        PerceptionFrameDigest([
            0x485c_3578_8fcb_d2b5,
            0x72df_599f_352d_a9fe,
            0xcce9_78b2_3cdd_7c38,
            0x5baf_03df_f1eb_ca98,
        ])
    );
}

#[test]
fn digest_enum_raw_mappings_are_stable_and_reject_unknown_values() {
    let kinds = [
        ActionKind::Idle,
        ActionKind::Hold,
        ActionKind::Rest,
        ActionKind::Inspect,
        ActionKind::Move,
        ActionKind::Interact,
        ActionKind::Vocalize,
        ActionKind::Write,
        ActionKind::Gesture,
    ];
    for (raw, kind) in kinds.into_iter().enumerate() {
        assert_eq!(kind.raw(), raw as u8);
        assert_eq!(ActionKind::try_from_raw(raw as u8).unwrap(), kind);
    }
    assert!(ActionKind::try_from_raw(9).is_err());

    for (raw, channel) in TeacherPerceptionChannel::ALL.into_iter().enumerate() {
        assert_eq!(channel.raw(), raw as u8);
        assert_eq!(
            TeacherPerceptionChannel::try_from_raw(raw as u8).unwrap(),
            channel
        );
    }
    assert!(TeacherPerceptionChannel::try_from_raw(5).is_err());

    assert_eq!(PerceptionContextKind::None.raw(), 0);
    assert_eq!(PerceptionContextKind::EpisodicCandidateV1.raw(), 1);
    assert_eq!(
        PerceptionContextKind::try_from_raw(0).unwrap(),
        PerceptionContextKind::None
    );
    assert_eq!(
        PerceptionContextKind::try_from_raw(1).unwrap(),
        PerceptionContextKind::EpisodicCandidateV1
    );
    assert!(PerceptionContextKind::try_from_raw(2).is_err());
}

#[test]
fn perception_digest_source_has_no_generic_serde_hash_path() {
    let source = include_str!("../src/perception.rs");
    assert!(!source.contains("hash_canonical"));
    assert!(!source.contains("CanonicalEncoder"));
    assert!(!source.contains("SerializeMap"));
    assert!(!source.contains("SerializeStruct"));
    assert!(!source.contains("serialize_field"));
}
