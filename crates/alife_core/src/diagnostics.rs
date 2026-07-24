//! v0 scaffold: compact validation diagnostics for logs and tests.

use serde::{Deserialize, Serialize};

use crate::{ScaffoldContractError, SchemaKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticCode {
    BrainClassTooSmall,
    BrainClassAlignment,
    LobeTotalMismatch,
    LobeAlignment,
    LobeRangeCoverage,
    RoutingReferencesDisabledLobe,
    RoutingDuplicateMask,
    MissingCanonicalNeuronCount,
    InvalidId,
    UnknownBrainClass,
    NonFiniteFloat,
    ScalarOutOfRange,
    DenseAlphaRequiresOptIn,
    LamarckianInheritanceRequiresOptIn,
    NonMonotonicTick,
    InvalidBounds,
    MissingPhaseData,
    UnorderedExperiencePhase,
    MismatchedCreatureId,
    InvalidActionDecision,
    OutOfRangeDriveHormone,
    IncompatibleAbi,
    PackedLogSchemaMismatch,
    PackedLogSideBufferOverflow,
    PackedLogFrameCapacityExceeded,
    TopologyCapacityExceeded,
    InvalidSparseProjectionSchema,
    UnsupportedSparseTileFormat,
    BackendParity,
    InvalidPerceptionFrame,
    InvalidActionCandidate,
    InvalidDecisionEvidence,
    EvidenceKindMismatch,
    PhenotypeCompile,
    UnsupportedProductionBrainClass,
    GpuLayoutMismatch,
    SensorProfileMismatch,
    TrackedObjectIdentityExhausted,
    InvalidMemoryQuery,
    MemoryModeConflict,
    MemoryReplayRejected,
    MemoryCompactionConflict,
    BrainOwnershipMismatch,
    LearningEvidenceMismatch,
    LearningReplayRejected,
    ConsolidationGenerationMismatch,
    BrainActivityPolicyMismatch,
    BrainActivitySequenceMismatch,
    BrainAtpExhausted,
    GpuTimestampQueryUnavailable,
    NeuralBackendUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractDiagnostic {
    pub code: DiagnosticCode,
    pub schema: Option<SchemaKind>,
    pub expected: Option<u16>,
    pub actual: Option<u16>,
}

impl ContractDiagnostic {
    pub const fn new(code: DiagnosticCode) -> Self {
        Self {
            code,
            schema: None,
            expected: None,
            actual: None,
        }
    }
}

impl From<&ScaffoldContractError> for ContractDiagnostic {
    fn from(error: &ScaffoldContractError) -> Self {
        match *error {
            ScaffoldContractError::BrainClassTooSmall => {
                Self::new(DiagnosticCode::BrainClassTooSmall)
            }
            ScaffoldContractError::BrainClassAlignment => {
                Self::new(DiagnosticCode::BrainClassAlignment)
            }
            ScaffoldContractError::LobeTotalMismatch => {
                Self::new(DiagnosticCode::LobeTotalMismatch)
            }
            ScaffoldContractError::LobeAlignment => Self::new(DiagnosticCode::LobeAlignment),
            ScaffoldContractError::LobeRangeCoverage => {
                Self::new(DiagnosticCode::LobeRangeCoverage)
            }
            ScaffoldContractError::RoutingReferencesDisabledLobe => {
                Self::new(DiagnosticCode::RoutingReferencesDisabledLobe)
            }
            ScaffoldContractError::RoutingDuplicateMask => {
                Self::new(DiagnosticCode::RoutingDuplicateMask)
            }
            ScaffoldContractError::MissingCanonicalNeuronCount => {
                Self::new(DiagnosticCode::MissingCanonicalNeuronCount)
            }
            ScaffoldContractError::InvalidId => Self::new(DiagnosticCode::InvalidId),
            ScaffoldContractError::UnknownBrainClass => {
                Self::new(DiagnosticCode::UnknownBrainClass)
            }
            ScaffoldContractError::NonFiniteFloat => Self::new(DiagnosticCode::NonFiniteFloat),
            ScaffoldContractError::ScalarOutOfRange => Self::new(DiagnosticCode::ScalarOutOfRange),
            ScaffoldContractError::DenseAlphaRequiresOptIn => {
                Self::new(DiagnosticCode::DenseAlphaRequiresOptIn)
            }
            ScaffoldContractError::LamarckianInheritanceRequiresOptIn => {
                Self::new(DiagnosticCode::LamarckianInheritanceRequiresOptIn)
            }
            ScaffoldContractError::NonMonotonicTick => Self::new(DiagnosticCode::NonMonotonicTick),
            ScaffoldContractError::InvalidBounds => Self::new(DiagnosticCode::InvalidBounds),
            ScaffoldContractError::MissingPhaseData => Self::new(DiagnosticCode::MissingPhaseData),
            ScaffoldContractError::UnorderedExperiencePhase => {
                Self::new(DiagnosticCode::UnorderedExperiencePhase)
            }
            ScaffoldContractError::MismatchedCreatureId => {
                Self::new(DiagnosticCode::MismatchedCreatureId)
            }
            ScaffoldContractError::InvalidActionDecision => {
                Self::new(DiagnosticCode::InvalidActionDecision)
            }
            ScaffoldContractError::OutOfRangeDriveHormone => {
                Self::new(DiagnosticCode::OutOfRangeDriveHormone)
            }
            ScaffoldContractError::IncompatibleAbi {
                kind,
                expected,
                actual,
            } => Self {
                code: DiagnosticCode::IncompatibleAbi,
                schema: Some(kind),
                expected: Some(expected),
                actual: Some(actual),
            },
            ScaffoldContractError::PackedLogSchemaMismatch { expected, actual } => Self {
                code: DiagnosticCode::PackedLogSchemaMismatch,
                schema: Some(SchemaKind::PackedLog),
                expected: Some(expected),
                actual: Some(actual),
            },
            ScaffoldContractError::PackedLogSideBufferOverflow => {
                Self::new(DiagnosticCode::PackedLogSideBufferOverflow)
            }
            ScaffoldContractError::PackedLogFrameCapacityExceeded => {
                Self::new(DiagnosticCode::PackedLogFrameCapacityExceeded)
            }
            ScaffoldContractError::TopologyCapacityExceeded => {
                Self::new(DiagnosticCode::TopologyCapacityExceeded)
            }
            ScaffoldContractError::InvalidSparseProjectionSchema => {
                Self::new(DiagnosticCode::InvalidSparseProjectionSchema)
            }
            ScaffoldContractError::UnsupportedSparseTileFormat => {
                Self::new(DiagnosticCode::UnsupportedSparseTileFormat)
            }
            ScaffoldContractError::BackendParity => Self::new(DiagnosticCode::BackendParity),
            ScaffoldContractError::InvalidPerceptionFrame => {
                Self::new(DiagnosticCode::InvalidPerceptionFrame)
            }
            ScaffoldContractError::InvalidActionCandidate => {
                Self::new(DiagnosticCode::InvalidActionCandidate)
            }
            ScaffoldContractError::InvalidDecisionEvidence => {
                Self::new(DiagnosticCode::InvalidDecisionEvidence)
            }
            ScaffoldContractError::EvidenceKindMismatch => {
                Self::new(DiagnosticCode::EvidenceKindMismatch)
            }
            ScaffoldContractError::PhenotypeCompile => Self::new(DiagnosticCode::PhenotypeCompile),
            ScaffoldContractError::UnsupportedProductionBrainClass => {
                Self::new(DiagnosticCode::UnsupportedProductionBrainClass)
            }
            ScaffoldContractError::GpuLayoutMismatch => {
                Self::new(DiagnosticCode::GpuLayoutMismatch)
            }
            ScaffoldContractError::SensorProfileMismatch => {
                Self::new(DiagnosticCode::SensorProfileMismatch)
            }
            ScaffoldContractError::TrackedObjectIdentityExhausted => {
                Self::new(DiagnosticCode::TrackedObjectIdentityExhausted)
            }
            ScaffoldContractError::InvalidMemoryQuery => {
                Self::new(DiagnosticCode::InvalidMemoryQuery)
            }
            ScaffoldContractError::MemoryModeConflict => {
                Self::new(DiagnosticCode::MemoryModeConflict)
            }
            ScaffoldContractError::MemoryReplayRejected => {
                Self::new(DiagnosticCode::MemoryReplayRejected)
            }
            ScaffoldContractError::MemoryCompactionConflict => {
                Self::new(DiagnosticCode::MemoryCompactionConflict)
            }
            ScaffoldContractError::BrainOwnershipMismatch => {
                Self::new(DiagnosticCode::BrainOwnershipMismatch)
            }
            ScaffoldContractError::LearningEvidenceMismatch => {
                Self::new(DiagnosticCode::LearningEvidenceMismatch)
            }
            ScaffoldContractError::LearningReplayRejected => {
                Self::new(DiagnosticCode::LearningReplayRejected)
            }
            ScaffoldContractError::ConsolidationGenerationMismatch => {
                Self::new(DiagnosticCode::ConsolidationGenerationMismatch)
            }
            ScaffoldContractError::BrainActivityPolicyMismatch => {
                Self::new(DiagnosticCode::BrainActivityPolicyMismatch)
            }
            ScaffoldContractError::BrainActivitySequenceMismatch => {
                Self::new(DiagnosticCode::BrainActivitySequenceMismatch)
            }
            ScaffoldContractError::BrainAtpExhausted => {
                Self::new(DiagnosticCode::BrainAtpExhausted)
            }
            ScaffoldContractError::GpuTimestampQueryUnavailable => {
                Self::new(DiagnosticCode::GpuTimestampQueryUnavailable)
            }
            ScaffoldContractError::NeuralBackendUnavailable => {
                Self::new(DiagnosticCode::NeuralBackendUnavailable)
            }
        }
    }
}

impl From<ScaffoldContractError> for ContractDiagnostic {
    fn from(error: ScaffoldContractError) -> Self {
        Self::from(&error)
    }
}
