//! Immutable genetic-weight sign contract. Runtime learned offsets are separate.

use super::{BrainPhenotype, CompiledSynapseKind};
use crate::{FoundationAbiSelection, ProjectionType};

/// The numeric values are the existing GPU trainer metadata ABI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum GeneticWeightSignPolicy {
    Signed = 0,
    StrictNegative = 1,
    Nonnegative = 2,
}

impl GeneticWeightSignPolicy {
    pub fn accepts(self, weight: f32) -> bool {
        weight.is_finite()
            && match self {
                Self::Signed => true,
                Self::StrictNegative => weight < 0.0,
                Self::Nonnegative => weight >= 0.0,
            }
    }
}

impl FoundationAbiSelection {
    /// No caller flag can override the sign policy of an admitted foundation.
    pub fn genetic_weight_sign_policy(
        &self,
        kind: CompiledSynapseKind,
        projection: ProjectionType,
    ) -> GeneticWeightSignPolicy {
        use GeneticWeightSignPolicy::*;
        match projection {
            ProjectionType::LateralInhibition => StrictNegative,
            ProjectionType::Homeostatic if kind == CompiledSynapseKind::Recurrent => Nonnegative,
            ProjectionType::MotorProposal if kind == CompiledSynapseKind::Recurrent => {
                if matches!(self, Self::N2048ExperimentalGraphV3(_)) {
                    Signed
                } else {
                    Nonnegative
                }
            }
            _ => Signed,
        }
    }
}

impl BrainPhenotype {
    pub fn genetic_weight_sign_policy(
        &self,
        kind: CompiledSynapseKind,
        projection: ProjectionType,
    ) -> GeneticWeightSignPolicy {
        self.foundation_abi()
            .genetic_weight_sign_policy(kind, projection)
    }
}
