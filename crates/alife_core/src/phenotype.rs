//! Contract-only facade for immutable production phenotype compilation records.

use serde::{Deserialize, Serialize};

mod abi_validation;
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
mod learning;
mod memory_channel;
mod persistent_address;
mod record;
mod topology_compile;

pub use budgets::{CompiledBudgets, GlobalPhenotypeBudgetReceipt, RouteBudgetReceipt};
pub use capacity::{BrainCapacityClass, BrainExecutionBudget};
pub use compiled::{
    CompiledProjection, CompiledSynapse, CompiledSynapseKind, DecoderHeadKind,
    DecoderSynapseCoordinate, NeuronDynamics,
};
pub use compiler::PhenotypeCompiler;
pub use decoder::{AuxiliaryDecoderPlan, CandidateDecoderFamilyPlan, CandidateDecoderPlan};
pub use encoder::{SensorEncoderAssignment, SensorEncoderPlan, SensorEncoderSourceGroup};
pub use inputs::PhenotypeCompilerInputs;
pub use learning::{
    PlasticityReceptorPlan, ReplayCapturePlan, SleepConsolidationPlan, MAX_REPLAY_CAPTURE_SYNAPSES,
};
pub use memory_channel::MemoryChannelPlan;
pub use persistent_address::{
    PersistentAddressMap, PersistentDecoderAddress, PersistentDecoderAddressEntry,
    PersistentNeuronAddress, PersistentNeuronAddressEntry, PersistentProjectionAddress,
    PersistentProjectionAddressEntry, PersistentProjectionRole, PersistentSynapseAddress,
    PersistentSynapseAddressEntry,
};
pub use record::BrainPhenotype;

/// Stable content hash of one compiled brain phenotype.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhenotypeHash(pub [u64; 4]);
