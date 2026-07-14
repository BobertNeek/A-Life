//! Contract-only facade for immutable production phenotype compilation records.

use serde::{Deserialize, Serialize};

mod budgets;
mod capacity;
mod compiled;
mod compiler;
mod construction;
mod decoder;
mod encoder;
mod inputs;
mod io_compile;
mod layout_compile;
mod record;
mod topology_compile;

pub use budgets::{CompiledBudgets, GlobalPhenotypeBudgetReceipt, RouteBudgetReceipt};
pub use capacity::{BrainCapacityClass, BrainExecutionBudget};
pub use compiled::{
    CompiledProjection, CompiledSynapse, CompiledSynapseKind, DecoderHeadKind,
    DecoderSynapseCoordinate, NeuronDynamics,
};
pub use compiler::PhenotypeCompiler;
pub use decoder::{CandidateDecoderFamilyPlan, CandidateDecoderPlan};
pub use encoder::{SensorEncoderAssignment, SensorEncoderPlan, SensorEncoderSourceGroup};
pub use inputs::PhenotypeCompilerInputs;
pub use record::BrainPhenotype;

/// Stable content hash of one compiled brain phenotype.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhenotypeHash(pub [u64; 4]);
