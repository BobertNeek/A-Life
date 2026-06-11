//! v0 scaffold: compact validation diagnostics for logs and tests.

use serde::{Deserialize, Serialize};

use crate::{ScaffoldContractError, SchemaKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagnosticCode {
    BrainClassTooSmall,
    BrainClassAlignment,
    LobeTotalMismatch,
    LobeAlignment,
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
    OutOfRangeDriveHormone,
    IncompatibleAbi,
    PackedLogSchemaMismatch,
    BackendParity,
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
            ScaffoldContractError::BackendParity => Self::new(DiagnosticCode::BackendParity),
        }
    }
}

impl From<ScaffoldContractError> for ContractDiagnostic {
    fn from(error: ScaffoldContractError) -> Self {
        Self::from(&error)
    }
}
