//! v0 scaffold: engine-agnostic A-Life cognitive contracts.

pub mod action;
pub mod action_abi;
pub mod adapter;
pub mod brain_class;
pub mod chemistry;
pub mod diagnostics;
pub mod error;
pub mod experience;
pub mod genome;
pub mod ids;
pub mod lineage;
pub mod lobe;
pub mod math;
pub mod routing;
pub mod sensory_abi;
pub mod traits;
pub mod units;
pub mod validation;
pub mod version;

pub use action::{ActionCommand, ActionKind};
pub use action_abi::ActionAbiVersion;
pub use adapter::{CoreFromAdapter, CoreIntoAdapter, WorldEntityIdMapper};
pub use brain_class::{BrainClassRegistry, BrainClassSpec, BrainComputeBudget, BrainScaleTier};
pub use chemistry::EndocrineProfile;
pub use diagnostics::{ContractDiagnostic, DiagnosticCode};
pub use error::ScaffoldContractError;
pub use experience::{ExperiencePatchHeader, ExperiencePatchPhase};
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
pub use lineage::LineageExportManifest;
pub use lobe::{
    ActivationPolicy, LobeEssentiality, LobeKind, LobeLayout, LobeRegion, LobeThrottlePriority,
    PlasticityPolicy, UpdateCadence,
};
pub use math::{validate_finite, validate_finite_slice, Aabb, Pose, Quatf, Vec2f, Vec3f, Velocity};
pub use routing::{
    ActiveTilePolicy, BiologicalPriority, ProjectionType, RoutingMask, RoutingMatrix,
};
pub use sensory_abi::{SensoryAbiVersion, TeacherPerceptionChannel};
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
