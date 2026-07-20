//! Engine-agnostic A-Life cognitive contracts.

pub mod action;
pub mod action_abi;
pub mod activity;
pub mod adapter;
mod blake3_digest;
pub mod brain_class;
pub mod canonical_digest;
pub mod chemistry;
pub mod diagnostics;
pub mod error;
pub mod evidence_digest;
pub mod experience;
pub mod foundation;
pub mod genome;
pub mod grounding;
pub mod ids;
pub mod language;
pub mod learning;
pub mod lineage;
pub mod lobe;
pub mod math;
pub mod memory;
pub mod memory_query;
pub mod neural;
pub mod packed_log;
pub mod perception;
pub mod phenotype;
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
    heuristic_baseline_arbitrate, ActionArbitrationConfig, ActionArbitrationTrace,
    ActionArbitrationTraceRef, ActionBiasSource, ActionCommand, ActionDecision,
    ActionDecisionStatus, ActionFallbackReason, ActionInhibitionSample, ActionKind, ActionProposal,
    ActionRegistryEntry, ActionScoreBias, ActionTarget, ActionWtaResult, InhibitionNeighborhood,
    MotorPayloadKind, MotorPayloadRef, RankedActionProposal, SuppressedProposal, SuppressionReason,
    TeacherLessonMetadata, TeacherLessonResponseChannel,
};
pub use action_abi::ActionAbiVersion;
pub use activity::{
    BrainActivityPolicyV1, BrainAtpCostModel, BrainDispatchIdentity, BrainWorkCounters,
    BrainWorkReceipt, GpuPressureSample, GpuPressureSampleInput, NeuralThrottleDecision,
    NeuralThrottleLevel, BRAIN_ACTIVITY_POLICY_VERSION, BRAIN_ACTIVITY_SCHEMA_VERSION,
    BRAIN_ATP_BASAL_DEBIT_Q16, BRAIN_ATP_Q16_MAX, BRAIN_ATP_SLEEP_RECOVERY_Q16,
};
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
pub use evidence_digest::{
    lobe_layout_evidence_digest, projection_plan_evidence_digest, synapse_payload_evidence_digest,
    PhenotypeEvidenceManifest, GPU_PHENOTYPE_EVIDENCE_MANIFEST_SCHEMA,
};
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
    MotorAffordanceGene, MotorAffordanceKind, MutationRates, PlasticityGenomeParameters,
    PlasticityMask, ProjectionAlphaOverride, ProjectionKey, ProjectionPlasticityMask,
    SensorChannelGene, SensorChannelKind, SensorLayoutGene, SparseDensityPrior, SynapseAddress,
    SynapseAlphaOverride, TileAddress, TileAlphaOverride, WEffective, WGeneticFixed,
    WLifetimeConsolidated, WeightLayerDescriptor, WeightLayerKind, WeightSplitContract,
    WeightStorageSemantics,
};
pub use grounding::{
    GroundedObjectSlotV1, SensorProfileId, SensorProfileIdentity, SensorProfileProvenance,
    GROUNDED_OBJECT_SLOT_SCHEMA_VERSION, MAX_GROUNDED_OBJECT_SLOTS,
};
pub use ids::{
    validate_optional_target, ActionId, BrainClassId, ConceptCellId, CreatureId,
    ExperienceSequenceId, GaussianClusterId, GenomeId, LineageId, LobeIndex, MemoryId, NeuronIndex,
    OrganismId, TrackedObjectId, WorldEntityId,
};
pub use language::{
    LanguageCodebookId, LanguageCodebookV1, LanguageTokenClass, LanguageTokenId, SpeechActKind,
    SpeechDecoderLayoutV1,
};
pub use learning::{
    validate_outcome_credit_schema, FastWeightSemantics, LearningCommitToken,
    LearningSequenceGuard, NeuromodulatorSample, OutcomeCreditPacket, OutcomeCreditReplayKey,
};
pub use lineage::LineageExportManifest;
pub use lobe::{
    ActivationPolicy, LobeEssentiality, LobeKind, LobeLayout, LobeRegion, LobeThrottlePriority,
    PlasticityPolicy, UpdateCadence,
};
pub use math::{validate_finite, validate_finite_slice, Aabb, Pose, Quatf, Vec2f, Vec3f, Velocity};
pub use memory::{
    CandidateMemoryRecallReceipt, FinalizedMemoryRecall, MemoryBank, MemoryBankConfig,
    MemoryBucketReceiptKey, MemoryCompactionCheckpoint, MemoryCompactionIdentity,
    MemoryCompactionPhase, MemoryCompactionReceipt, MemoryConsolidationBatch, MemoryConsolidator,
    MemoryExpectancy, MemoryMatch, MemoryOutcomeSummary, MemoryQuery, MemoryRecallChannel,
    MemoryRecallDegradation, MemoryRecallReceipt, MemoryRecord, MemorySidecarState,
    MemoryUpdateKind, MemoryUpdateReceipt, PortableMemoryBankAssetV2, PortableMemoryRecordV2,
    PreparedMemoryCompaction, PreparedMemoryRecall, TargetMemoryBucketReceiptKey,
    MEMORY_BANK_MAX_CAPACITY, MEMORY_FAMILY_SEARCH_CAP, MEMORY_FEATURE_VECTOR_MAX_LEN,
    MEMORY_MERGE_SIMILARITY, MEMORY_MIN_SIMILARITY, MEMORY_RECALL_SCHEMA_VERSION,
    MEMORY_RECALL_TOP_K, MEMORY_TARGET_SEARCH_CAP, MEMORY_TOTAL_SEARCH_CAP,
};
pub use memory_query::{
    CandidateMemoryContextV1, CandidateMemoryQueryV2, EpisodicDecisionKeyV2,
    EpisodicRetrievalContextV1, MemoryQueryEncoderV2, MemoryQueryVersion,
    EPISODIC_RETRIEVAL_CONTEXT_SCHEMA_VERSION, MEMORY_ACTION_FAMILY_RANGE,
    MEMORY_ACTION_KIND_RANGE, MEMORY_BODY_RANGE, MEMORY_CONTEXT_V1_LANES_PER_CANDIDATE,
    MEMORY_CONTEXT_V1_MAX_SOURCES, MEMORY_DRIVE_RANGE, MEMORY_HORMONE_RANGE,
    MEMORY_LATENT_V1_COUNT, MEMORY_PROFILE_RANGE, MEMORY_QUERY_V2_FEATURE_COUNT,
    MEMORY_RESERVED_RANGE, MEMORY_STATE_SENSORY_RANGE, MEMORY_TARGET_RANGE, MEMORY_VALUE_V1_COUNT,
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
    GlobalPhenotypeBudgetReceipt, MemoryChannelPlan, NeuronDynamics, PersistentAddressMap,
    PersistentDecoderAddress, PersistentDecoderAddressEntry, PersistentNeuronAddress,
    PersistentNeuronAddressEntry, PersistentProjectionAddress, PersistentProjectionAddressEntry,
    PersistentProjectionRole, PersistentSynapseAddress, PersistentSynapseAddressEntry,
    PhenotypeCompiler, PhenotypeCompilerInputs, PhenotypeHash, PlasticityReceptorPlan,
    ReplayCapturePlan, RouteBudgetReceipt, SensorEncoderAssignment, SensorEncoderPlan,
    SensorEncoderSourceGroup, SleepConsolidationPlan, MAX_REPLAY_CAPTURE_SYNAPSES,
    REQUIRED_GPU_FEATURE_MASK,
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
    compute_gpu_sleep_commit_digest, compute_gpu_sleep_input_weight_digest,
    compute_gpu_sleep_mutable_state_digest, compute_gpu_sleep_output_weight_digest,
    decode_replay_eligibility_q15, encode_replay_eligibility_q15, BoundedReplayBatch,
    ConceptConsolidationReport, ConsolidationDriverEvent, ConsolidationIntent, ConsolidationJobId,
    ConsolidationStagedOutput, ConsolidationState, GpuConsolidationRequest, HTraceDrainReport,
    LifetimeTraitEvidence, LifetimeTraitLedger, MemoryCompressionReport, ReplayEligibilitySample,
    ReplaySynapseSpan, SleepConsolidationConfig, SleepConsolidationReport, SleepConsolidator,
    SleepController, SleepPhase, SleepReplayEvent, SleepReplayJournal, SleepState, SleepTransition,
    SleepTrigger, StableLifetimeTrait, StableLifetimeTraitKind, StructuralEditApplicationStatus,
    StructuralEditBatch, StructuralEditCandidate, StructuralEditKind, StructuralEditReason,
    TraitPromotionReport, BOUNDED_REPLAY_BATCH_SCHEMA_VERSION,
    GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION, SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};
pub use topology::{
    ActionObservationFact, CognitiveEdge, CognitiveEdgeId, CognitiveSimplex, CognitiveSimplexId,
    ConceptBindings, ConceptCell, ContradictionType, CuriosityBias, DriveBinding, DriveChannel,
    EdgeRelationKind, EmotionValenceSummary, GapResolutionStatus, PortableTopologyActionBindingV1,
    PortableTopologyBindingSetV1, PortableTopologyConceptV1, PortableTopologyDriveBindingV1,
    PortableTopologyEdgeV1, PortableTopologyGapV1, PortableTopologySidecarAssetV1,
    PortableTopologySimplexV1, TopologicalMap, TopologicalMapConfig, TopologyCounts,
    TopologyDegradationKind, TopologyIdCounters, TopologyObservationReceipt, TopologySidecar,
    TopologySidecarDiagnostics, TopologyUpdate, UnresolvedGap, UnresolvedGapId,
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
