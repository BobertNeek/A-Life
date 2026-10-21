//! Offline foundation training on the compiled neural graph, using WGSL or CPU.

mod backend_handoff;
#[cfg(feature = "cpu-backend")]
pub mod cpu;
mod founder_demonstrations;
mod founder_merge;
mod founder_readout;
mod ppo;
mod trainer;
mod types;

pub use backend_handoff::*;
pub use founder_demonstrations::*;
pub use founder_merge::*;
pub use founder_readout::*;
pub use ppo::*;
pub use trainer::FoundationTrainer;
pub use types::*;

#[derive(Debug, thiserror::Error)]
pub enum TrainingError {
    #[error("training world organism error: {0}")]
    Organism(#[from] alife_world::OrganismRegistryError),
    #[error("training contract error: {0}")]
    Contract(#[from] alife_core::ScaffoldContractError),
    #[error("GPU training submission failed")]
    GpuSubmission,
    #[error("GPU training readback was malformed")]
    MalformedReadback,
}

pub mod rehearsal;
