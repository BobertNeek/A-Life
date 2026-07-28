//! Engine-neutral checkpoint intent contracts.

use serde::{Deserialize, Serialize};

use crate::{ScaffoldContractError, Validate};

pub const BRAIN_CHECKPOINT_MODE_SCHEMA_VERSION: u16 = 1;

/// Declares which durable parts of a creature brain a checkpoint carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrainCheckpointMode {
    /// Rebuild immutable genetic state and clear all acquired state.
    GeneticRebuild,
    /// Carry consolidated individual learning into a healthy founder body.
    DurableLearnedFounder,
    /// Restore every mutable field required for same-save continuation.
    ExactResume,
}

impl BrainCheckpointMode {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::GeneticRebuild => "genetic-rebuild",
            Self::DurableLearnedFounder => "durable-learned-founder",
            Self::ExactResume => "exact-resume",
        }
    }

    pub fn try_from_slug(value: &str) -> Result<Self, ScaffoldContractError> {
        match value {
            "genetic-rebuild" => Ok(Self::GeneticRebuild),
            "durable-learned-founder" => Ok(Self::DurableLearnedFounder),
            "exact-resume" => Ok(Self::ExactResume),
            _ => Err(ScaffoldContractError::InvalidId),
        }
    }
}

impl Validate for BrainCheckpointMode {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        Ok(())
    }
}
