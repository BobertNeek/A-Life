//! v0 scaffold: validation errors for architecture contract checks.

use thiserror::Error;

use crate::SchemaKind;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ScaffoldContractError {
    #[error("brain class must contain at least 512 neurons")]
    BrainClassTooSmall,
    #[error("near-term GPU brain classes must align neuron count to 128")]
    BrainClassAlignment,
    #[error("lobe layout total does not match brain neuron count")]
    LobeTotalMismatch,
    #[error("lobe starts and lengths must align to 16")]
    LobeAlignment,
    #[error("lobe layout has a gap, overlap, or out-of-order enabled range")]
    LobeRangeCoverage,
    #[error("routing mask references a missing or disabled lobe")]
    RoutingReferencesDisabledLobe,
    #[error("routing mask duplicates an existing source-target projection")]
    RoutingDuplicateMask,
    #[error("requested brain tier has no canonical neuron count")]
    MissingCanonicalNeuronCount,
    #[error("ID value zero is reserved as invalid")]
    InvalidId,
    #[error("brain class ID is not known to the current scaffold registry")]
    UnknownBrainClass,
    #[error("float value must be finite")]
    NonFiniteFloat,
    #[error("scalar value is outside its allowed range")]
    ScalarOutOfRange,
    #[error("dense alpha storage requires an explicit debug/reference opt-in")]
    DenseAlphaRequiresOptIn,
    #[error("lifetime weight inheritance requires explicit Lamarckian opt-in")]
    LamarckianInheritanceRequiresOptIn,
    #[error("tick value moved backward")]
    NonMonotonicTick,
    #[error("axis-aligned bounds are invalid")]
    InvalidBounds,
    #[error("required phase data is missing")]
    MissingPhaseData,
    #[error("experience phases were recorded out of causal order")]
    UnorderedExperiencePhase,
    #[error("experience phases reference different creatures")]
    MismatchedCreatureId,
    #[error("action decision is internally inconsistent")]
    InvalidActionDecision,
    #[error("drive or hormone value is outside its allowed range")]
    OutOfRangeDriveHormone,
    #[error("incompatible {kind:?} version: expected {expected}, got {actual}")]
    IncompatibleAbi {
        kind: SchemaKind,
        expected: u16,
        actual: u16,
    },
    #[error("packed log schema mismatch: expected {expected}, got {actual}")]
    PackedLogSchemaMismatch { expected: u16, actual: u16 },
    #[error("packed log side buffer capacity exceeded")]
    PackedLogSideBufferOverflow,
    #[error("packed log frame capacity exceeded")]
    PackedLogFrameCapacityExceeded,
    #[error("backend parity check failed")]
    BackendParity,
}
