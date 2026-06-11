use alife_core::{
    AffordanceBits, ChannelGroupKind, CompressedSemanticCode, ContextFeatureFlags, ContextStreams,
    EnvironmentStreamEntry, GaussianClusterId, GaussianContextRef, GaussianSalienceEntry,
    HeardToken, OrganismId, ScaffoldContractError, SchemaKind, SemanticContextRef,
    SemanticSalienceEntry, SensoryAbiDescriptor, SensoryAbiVersion, SensoryChannels,
    SensorySnapshot, SensorySnapshotFromAdapter, SensorySnapshotSource, SocialAgentSnapshot,
    TeacherPerceptionChannel, Tick, Validate, Vec3f, VocalizedToken, WorldEntityId,
    MAX_HEARD_TOKENS, MAX_OPTIONAL_ENVIRONMENT_STREAMS, MAX_SOCIAL_AGENTS,
    SENSORY_ABI_CHANNEL_COUNT, SENSORY_AUDITORY_CHANNEL_COUNT, SENSORY_SMELL_CHANNEL_COUNT,
    SENSORY_TACTILE_CHANNEL_COUNT, SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT,
};
use alife_core::{Confidence, NormalizedScalar, SignedValence};

fn valid_channels() -> SensoryChannels {
    SensoryChannels::try_from_groups(
        [0.2; SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT],
        [0.3; SENSORY_AUDITORY_CHANNEL_COUNT],
        [0.4; SENSORY_SMELL_CHANNEL_COUNT],
        [0.5; SENSORY_TACTILE_CHANNEL_COUNT],
        NormalizedScalar::new(0.1).unwrap(),
        NormalizedScalar::new(0.6).unwrap(),
        AffordanceBits::FOOD | AffordanceBits::SHELTER,
    )
    .unwrap()
}

#[test]
fn sensory_abi_descriptor_is_versioned_and_fixed_width() {
    let descriptor = SensoryAbiDescriptor::V1;

    assert_eq!(
        descriptor.version,
        SensoryAbiVersion::CURRENT,
        "P08 uses central schema versioning"
    );
    assert_eq!(
        descriptor.total_channel_count(),
        SENSORY_ABI_CHANNEL_COUNT,
        "fixed-size ABI width must match group totals"
    );
    assert!(descriptor
        .channel_groups
        .iter()
        .any(|group| group.kind == ChannelGroupKind::VisualAffordance));
    assert!(descriptor
        .channel_groups
        .iter()
        .any(|group| group.kind == ChannelGroupKind::AuditoryAcoustic));
    assert!(descriptor
        .channel_groups
        .iter()
        .any(|group| group.kind == ChannelGroupKind::SmellChemistry));
    assert!(descriptor
        .channel_groups
        .iter()
        .any(|group| group.kind == ChannelGroupKind::TactileContact));

    assert_eq!(
        valid_channels().as_flat_array().len(),
        SENSORY_ABI_CHANNEL_COUNT
    );
}

#[test]
fn sensory_snapshot_constructs_without_optional_semantic_or_gaussian_context() {
    let snapshot = SensorySnapshot::new(
        OrganismId(1),
        Tick(10),
        Vec3f::new(1.0, 2.0, 3.0),
        valid_channels(),
        ContextStreams::default(),
    )
    .unwrap();

    assert_eq!(snapshot.abi_version, SensoryAbiVersion::CURRENT);
    assert!(snapshot.semantic_context.is_none());
    assert!(snapshot.gaussian_context.is_none());
    assert!(snapshot.validate_contract().is_ok());
}

#[test]
fn sensory_channels_reject_nan_out_of_bounds_and_bad_versions() {
    let mut channels = valid_channels();
    channels.visual_affordance[0] = f32::NAN;
    assert_eq!(
        channels.validate_contract(),
        Err(ScaffoldContractError::NonFiniteFloat)
    );

    let mut channels = valid_channels();
    channels.auditory_acoustic[1] = 1.1;
    assert_eq!(
        channels.validate_contract(),
        Err(ScaffoldContractError::ScalarOutOfRange)
    );

    let mut snapshot = SensorySnapshot::new(
        OrganismId(1),
        Tick(10),
        Vec3f::ZERO,
        valid_channels(),
        ContextStreams::default(),
    )
    .unwrap();
    snapshot.abi_version = SensoryAbiVersion(99);
    assert_eq!(
        snapshot.validate_contract(),
        Err(ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::SensoryAbi,
            expected: 1,
            actual: 99,
        })
    );
}

#[test]
fn context_streams_validate_bounds_and_fixed_slots() {
    let mut streams = ContextStreams {
        atmospheric_temperature_celsius: 22.0,
        ambient_light: NormalizedScalar::new(0.8).unwrap(),
        energy_intake_trend: SignedValence::new(0.25).unwrap(),
        blood_sugar_trend: SignedValence::new(-0.15).unwrap(),
        ..Default::default()
    };
    streams.optional_environment[0] = Some(EnvironmentStreamEntry {
        stream_id: 7,
        value: NormalizedScalar::new(0.4).unwrap(),
        confidence: Confidence::new(0.9).unwrap(),
    });

    assert_eq!(streams.vocal_tokens.len(), MAX_HEARD_TOKENS);
    assert_eq!(streams.social_proximity.len(), MAX_SOCIAL_AGENTS);
    assert_eq!(
        streams.optional_environment.len(),
        MAX_OPTIONAL_ENVIRONMENT_STREAMS
    );
    assert!(streams.validate_contract().is_ok());

    streams.atmospheric_temperature_celsius = f32::INFINITY;
    assert_eq!(
        streams.validate_contract(),
        Err(ScaffoldContractError::NonFiniteFloat)
    );
}

#[test]
fn optional_gaussian_and_semantic_contexts_are_metadata_refs() {
    let gaussian = GaussianContextRef {
        egocentric_bin_hash: 0x00AA_BBCC_DDEE_FF11,
        feature_flags: ContextFeatureFlags::GAUSSIAN_CLUSTERS
            | ContextFeatureFlags::EGOCENTRIC_BIN_HASH,
        confidence: Confidence::new(0.72).unwrap(),
        clusters: vec![GaussianSalienceEntry {
            cluster_id: GaussianClusterId(42),
            salience: NormalizedScalar::new(0.9).unwrap(),
            distance_meters: 3.5,
        }],
    };
    let semantic = SemanticContextRef {
        feature_flags: ContextFeatureFlags::SEMANTIC_CODES
            | ContextFeatureFlags::INTERNAL_SLM_MODULATION,
        confidence: Confidence::new(0.64).unwrap(),
        compressed_codes: vec![CompressedSemanticCode {
            codebook_id: 2,
            code: 19,
            salience: NormalizedScalar::new(0.5).unwrap(),
        }],
        salience: vec![SemanticSalienceEntry {
            concept_id: alife_core::ConceptCellId(9),
            salience: NormalizedScalar::new(0.8).unwrap(),
        }],
    };

    let mut snapshot = SensorySnapshot::new(
        OrganismId(1),
        Tick(10),
        Vec3f::ZERO,
        valid_channels(),
        ContextStreams::default(),
    )
    .unwrap();
    snapshot.gaussian_context = Some(gaussian);
    snapshot.semantic_context = Some(semantic);

    assert!(snapshot.validate_contract().is_ok());

    let mut bad = snapshot;
    bad.gaussian_context.as_mut().unwrap().clusters[0].cluster_id = GaussianClusterId::INVALID;
    assert_eq!(
        bad.validate_contract(),
        Err(ScaffoldContractError::InvalidId)
    );
}

#[test]
fn social_and_language_context_use_stable_ids_and_perception_markers() {
    let mut snapshot = SensorySnapshot::new(
        OrganismId(1),
        Tick(10),
        Vec3f::ZERO,
        valid_channels(),
        ContextStreams::default(),
    )
    .unwrap();

    snapshot.social_context.nearest_agents[0] = Some(SocialAgentSnapshot {
        agent_id: OrganismId(2),
        body_entity: Some(WorldEntityId(20)),
        relative_position: Vec3f::new(0.0, 0.0, 2.0),
        gaze_direction: Vec3f::new(0.0, 0.0, -1.0),
        orientation_forward: Vec3f::new(0.0, 0.0, -1.0),
        affinity: SignedValence::new(0.35).unwrap(),
        proximity: NormalizedScalar::new(0.7).unwrap(),
    });
    snapshot.language_context.heard_tokens[0] = Some(HeardToken {
        speaker_id: Some(OrganismId(2)),
        source_entity: Some(WorldEntityId(20)),
        token_id: 101,
        source_position: Vec3f::new(0.0, 1.0, 2.0),
        confidence: Confidence::new(0.95).unwrap(),
        teacher_channel: Some(TeacherPerceptionChannel::Hearing),
    });
    snapshot.language_context.vocalized_token = Some(VocalizedToken {
        token_id: 55,
        confidence: Confidence::new(0.8).unwrap(),
    });
    snapshot.language_context.word_confidence = Confidence::new(0.88).unwrap();
    snapshot.language_context.teacher_channel_marker = Some(TeacherPerceptionChannel::Writing);

    assert!(snapshot.validate_contract().is_ok());

    snapshot.social_context.nearest_agents[0]
        .as_mut()
        .unwrap()
        .agent_id = OrganismId::INVALID;
    assert_eq!(
        snapshot.validate_contract(),
        Err(ScaffoldContractError::InvalidId)
    );
}

#[test]
fn sensory_conversion_interfaces_are_adapter_implemented_only() {
    struct AdapterFrame;
    struct AdapterSource;

    impl SensorySnapshotFromAdapter<AdapterFrame> for SensorySnapshot {
        fn sensory_from_adapter(value: AdapterFrame) -> Result<Self, ScaffoldContractError> {
            let _ = value;
            SensorySnapshot::new(
                OrganismId(1),
                Tick(10),
                Vec3f::ZERO,
                valid_channels(),
                ContextStreams::default(),
            )
        }
    }

    impl SensorySnapshotSource for AdapterSource {
        fn sensory_snapshot(
            &self,
            organism_id: OrganismId,
            tick: Tick,
        ) -> Result<SensorySnapshot, ScaffoldContractError> {
            SensorySnapshot::new(
                organism_id,
                tick,
                Vec3f::ZERO,
                valid_channels(),
                ContextStreams::default(),
            )
        }
    }

    let converted = SensorySnapshot::sensory_from_adapter(AdapterFrame).unwrap();
    let sourced = AdapterSource
        .sensory_snapshot(OrganismId(5), Tick(12))
        .unwrap();

    assert_eq!(converted.organism_id, OrganismId(1));
    assert_eq!(sourced.organism_id, OrganismId(5));
}
