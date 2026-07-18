//! Engine-agnostic A-Life cognitive contracts.

pub mod action;
pub mod action_abi;
pub mod adapter;
mod blake3_digest;
pub mod brain_class;
pub mod canonical_digest;
pub mod chemistry;
pub mod diagnostics;
pub mod error;
pub mod experience;
pub mod foundation;
pub mod genome;
pub mod ids;
pub mod language;
pub mod lineage;
pub mod lobe;
pub mod math;
pub mod memory;
pub mod neural;
pub mod packed_log;
pub mod perception;
pub mod phenotype;
pub mod post_seal_lifetime;
pub mod reference_brain;
pub mod routing;
pub mod sensory_abi;
pub mod sleep;
pub mod topology;
pub mod traits;
pub mod units;
pub mod validation;
pub mod version;

pub use action::{
    cpu_reference_arbitrate, ActionArbitrationConfig, ActionArbitrationTrace,
    ActionArbitrationTraceRef, ActionBiasSource, ActionCommand, ActionDecision,
    ActionDecisionStatus, ActionFallbackReason, ActionInhibitionSample, ActionKind, ActionProposal,
    ActionRegistryEntry, ActionScoreBias, ActionTarget, ActionWtaResult, InhibitionNeighborhood,
    MotorPayloadKind, MotorPayloadRef, RankedActionProposal, SuppressedProposal, SuppressionReason,
    TeacherLessonMetadata, TeacherLessonResponseChannel,
};
pub use action_abi::ActionAbiVersion;
pub use adapter::{CoreFromAdapter, CoreIntoAdapter, WorldEntityIdMapper};
pub use blake3_digest::Blake3Digest;
pub use brain_class::{
    BrainClassRegistry, BrainClassSpec, BrainComputeBudget, BrainScaleTier, LegacyBrainClassAdapter,
};
pub use canonical_digest::CanonicalDigestBuilder;
pub use chemistry::{
    ChemistryModulation, DriveDelta, DriveSnapshot, EndocrineDelta, EndocrineProfile,
    EndocrineSnapshot, HomeostaticCadence, HomeostaticCadenceBand, HomeostaticDelta,
    HomeostaticParameters, HomeostaticSnapshot, RecoveryAssessment, RecoveryTrigger,
    DRIVE_EXTENSION_SLOTS, ENDOCRINE_EXTENSION_SLOTS,
};
pub use diagnostics::{ContractDiagnostic, DiagnosticCode};
pub use error::ScaffoldContractError;
pub use experience::{
    ConceptHint, DecisionEvidence, DecisionSnapshot, EvidenceKind, ExperiencePatch,
    ExperiencePatchBuilder, ExperiencePatchHeader, ExperiencePatchPhase, ExperiencePatchView,
    HeuristicDecisionEvidence, HeuristicPreActionEvidence, MemoryExpectancySnapshot, MemoryHint,
    NeuralDecisionEvidence, PhysicalActionOutcome, PhysicalContactKind, PostActionOutcome,
    PreActionBrainEvidence, PreActionSnapshot, TeacherFeedbackObservation,
};
pub use foundation::{
    FoundationAbiBinding, FoundationLayoutId, FoundationSectionPolicy, LifetimePlasticityBand,
    N2048FoundationLayoutV1, N2048FoundationRouteSpec,
};
pub use genome::{
    AlphaMask, AlphaStoragePolicy, BrainGenome, CriticalPeriod, CrossoverPolicy, DevelopmentStage,
    DevelopmentState, DevelopmentalMilestone, DevelopmentalSchedule, DriveThresholdGene,
    DriveThresholdKind, EffectiveWeightSample, EndocrineConstantGene, EndocrineConstantKind,
    GenomeSeedSet, HOperational, HShadow, InheritancePolicy, LifetimeConsolidationDelta,
    LobeAlphaOverride, LobeRatioOverride, LobeRatioPlan, LobeRatioRegistryRef, MacroConnectomeMask,
    MotorAffordanceGene, MotorAffordanceKind, MutationRates, PlasticityMask,
    ProjectionAlphaOverride, ProjectionKey, ProjectionPlasticityMask, SensorChannelGene,
    SensorChannelKind, SensorLayoutGene, SparseDensityPrior, SynapseAddress, SynapseAlphaOverride,
    TileAddress, TileAlphaOverride, WEffective, WGeneticFixed, WLifetimeConsolidated,
    WeightLayerDescriptor, WeightLayerKind, WeightSplitContract, WeightStorageSemantics,
};
pub use ids::{
    validate_optional_target, ActionId, BrainClassId, ConceptCellId, CreatureId,
    ExperienceSequenceId, GaussianClusterId, GenomeId, LineageId, LobeIndex, MemoryId, NeuronIndex,
    OrganismId, WorldEntityId,
};
pub use language::{
    LanguageCodebookId, LanguageCodebookV1, LanguageTokenClass, LanguageTokenId, SpeechActKind,
    SpeechDecoderLayoutV1,
};
pub use lineage::LineageExportManifest;
pub use lobe::{
    ActivationPolicy, LobeEssentiality, LobeKind, LobeLayout, LobeRegion, LobeThrottlePriority,
    PlasticityPolicy, UpdateCadence,
};
pub use math::{validate_finite, validate_finite_slice, Aabb, Pose, Quatf, Vec2f, Vec3f, Velocity};
pub use memory::{
    MemoryBank, MemoryBankConfig, MemoryConsolidationBatch, MemoryConsolidator, MemoryExpectancy,
    MemoryMatch, MemoryOutcomeSummary, MemoryQuery, MemoryRecord, MEMORY_BANK_MAX_CAPACITY,
    MEMORY_FEATURE_VECTOR_MAX_LEN,
};
pub use neural::{
    cpu_spmv_projection, finalize_cpu_activations, update_oja_shadow_traces, ActivationFunction,
    CooEntry, CooTile, CpuNeuralState, DecodedSynapse, DenseTile, LobeActivationView, Microtile,
    NeuralActivationConfig, NeuralDiagnostics, NeuralProjectionSchema, NeuralUpdateMetadata,
    NeuralUpdateReport, NeuronRange, OjaUpdateConfig, PlasticityTraceBuffers, ProjectionRoutingRef,
    ProjectionTile, SparseProjection, SparseTileCoord, SparseTilePayload, SparseTileType,
    SupertileMask, SynapseWeightSplit, TileMetadata, MICROTILE_CELLS, MICROTILE_EDGE,
    SUPERTILE_EDGE, SUPERTILE_MICROTILES,
};
pub use packed_log::{
    ExperiencePacker, InMemoryPackedExperienceLog, PackedExperienceFrame, PackedExperienceRecord,
    PackedExperienceSink, PackedExperienceSummary, PackedLogEntryRef, PackedSideBufferKind,
    PackedSideBufferRecord, PackedSideBufferSpans, PackedSideBuffers, SideBufferSpan,
    PACKED_DRIVE_SUMMARY_CHANNELS, PACKED_EXPERIENCE_FRAME_RESERVED_U32S,
    PACKED_EXPERIENCE_SCHEMA_VERSION, PACKED_FLAG_CONTRADICTION_OBSERVED,
    PACKED_FLAG_HAS_GAUSSIAN_CONTEXT, PACKED_FLAG_HAS_MOTOR_PAYLOAD,
    PACKED_FLAG_HAS_SEMANTIC_CONTEXT, PACKED_FLAG_HAS_TARGET_ENTITY,
    PACKED_FLAG_HAS_TARGET_POSITION, PACKED_FLAG_HAS_TEACHER_FEEDBACK,
    PACKED_FLAG_HAS_TEACHER_LESSON, PACKED_FLAG_SUCCESS, PACKED_HORMONE_SUMMARY_CHANNELS,
    PACKED_LOG_DEFAULT_SIDE_BUFFER_CAPACITY_RECORDS, PACKED_SIDE_BUFFER_GROUP_COUNT,
};
pub use perception::{
    ActionCandidate, BodySnapshot, CandidateActionFamily, CandidateFeatureDigest,
    CandidateFeatureVector, CandidateObservationRef, NeuralActionSelection, PerceptionBaseDigest,
    PerceptionContextBlock, PerceptionContextDigest, PerceptionContextKind, PerceptionFrame,
    PerceptionFrameDigest, PerceptionFrameDraft, PolicyBackend, SensorProfile,
    CANDIDATE_FEATURE_COUNT, MAX_ACTION_CANDIDATES,
};
pub use phenotype::{
    AuxiliaryDecoderPlan, BrainCapacityClass, BrainExecutionBudget, BrainPhenotype,
    CandidateDecoderFamilyPlan, CandidateDecoderPlan, CompiledBudgets, CompiledProjection,
    CompiledSynapse, CompiledSynapseKind, DecoderHeadKind, DecoderSynapseCoordinate,
    GlobalPhenotypeBudgetReceipt, NeuronDynamics, PersistentAddressMap, PersistentDecoderAddress,
    PersistentDecoderAddressEntry, PersistentNeuronAddress, PersistentNeuronAddressEntry,
    PersistentProjectionAddress, PersistentProjectionAddressEntry, PersistentProjectionRole,
    PersistentSynapseAddress, PersistentSynapseAddressEntry, PhenotypeCompiler,
    PhenotypeCompilerInputs, PhenotypeHash, RouteBudgetReceipt, SensorEncoderAssignment,
    SensorEncoderPlan, SensorEncoderSourceGroup,
};
pub use post_seal_lifetime::{
    PostSealHShadowDeltaTarget, PostSealLearningToken, PostSealLifetimeDeltaApplication,
    PostSealLifetimeDeltaBatch, PostSealLifetimeDeltaReceipt, PostSealLifetimeDeltaRecord,
    PostSealLifetimeDeltaRejectionReason, PostSealLifetimeDeltaSchemaVersion,
    PostSealLifetimeDeltaSourceKind, PostSealLifetimeLayer, POST_SEAL_HSHADOW_ABS_LIMIT,
    POST_SEAL_HSHADOW_VALUE_EPSILON, POST_SEAL_LIFETIME_DELTA_MAX_RECORDS,
    POST_SEAL_LIFETIME_DELTA_SCHEMA_VERSION,
};
pub use reference_brain::{
    BrainTickDiagnostics, BrainTickInput, BrainTickOutput, BrainTickStatus, CreatureActionState,
    CreatureBodyState, CreatureMind, ReferenceActionExecution, ReferenceActionExecutor,
    ReferenceActionFailure, ReferenceOutcomeObservation, ReferenceOutcomeObserver,
    ReferenceOutcomeRequest, ReferenceSensoryAdapter, ReferenceSensoryRequest,
};
pub use routing::{
    ActiveTilePolicy, BiologicalPriority, ProjectionType, RoutingMask, RoutingMatrix,
};
pub use sensory_abi::{
    AffordanceBits, ChannelBounds, ChannelExtensionPolicy, ChannelGroupKind, ChannelGroupSpec,
    CompressedSemanticCode, ContextFeatureFlags, ContextStreams, EnvironmentStreamEntry,
    GaussianContextRef, GaussianSalienceEntry, HeardToken, LanguageContextSnapshot,
    SemanticContextRef, SemanticSalienceEntry, SensoryAbiDescriptor, SensoryAbiVersion,
    SensoryChannels, SensorySnapshot, SensorySnapshotFromAdapter, SensorySnapshotSource,
    SocialAgentSnapshot, SocialContextSnapshot, SocialProximityEntry, TeacherPerceptionChannel,
    VocalizedToken, MAX_HEARD_TOKENS, MAX_OPTIONAL_ENVIRONMENT_STREAMS, MAX_SOCIAL_AGENTS,
    SENSORY_ABI_CHANNEL_COUNT, SENSORY_AUDITORY_CHANNEL_COUNT, SENSORY_PAIN_NOVELTY_CHANNEL_COUNT,
    SENSORY_SMELL_CHANNEL_COUNT, SENSORY_TACTILE_CHANNEL_COUNT,
    SENSORY_VISUAL_AFFORDANCE_CHANNEL_COUNT,
};
pub use sleep::{
    ConceptConsolidationReport, HTraceDrainReport, LifetimeTraitEvidence, LifetimeTraitLedger,
    MemoryCompressionReport, SleepConsolidationConfig, SleepConsolidationReport, SleepConsolidator,
    SleepController, SleepPhase, SleepState, SleepTransition, SleepTrigger, StableLifetimeTrait,
    StableLifetimeTraitKind, StructuralEditApplicationStatus, StructuralEditBatch,
    StructuralEditCandidate, StructuralEditKind, StructuralEditReason, TraitPromotionReport,
    SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};
pub use topology::{
    ActionObservationFact, CognitiveEdge, CognitiveEdgeId, CognitiveSimplex, CognitiveSimplexId,
    ConceptBindings, ConceptCell, ContradictionType, CuriosityBias, DriveBinding, DriveChannel,
    EdgeRelationKind, EmotionValenceSummary, GapResolutionStatus, TopologicalMap,
    TopologicalMapConfig, TopologyUpdate, UnresolvedGap, UnresolvedGapId,
};
pub use traits::{
    NeuralComputeBackend, SemanticPriorPacket, SemanticPriorProvider, SemanticPriorRequest,
};
pub use units::{
    Confidence, DurationTicks, FixedPointScale, Intensity, NormalizedScalar, Seconds,
    SignedValence, Tick,
};
pub use validation::{ensure_current_version, Validate, Validated};
pub use version::{
    require_current_version, require_version, ContractVersion, SchemaKind, SchemaVersions,
};
