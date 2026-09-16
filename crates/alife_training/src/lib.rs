//! Offline-only exact-graph WGSL foundation training.

mod active_battery;
mod curriculum;
mod era1_trials;
mod evolution;
mod founder_demonstrations;
mod founder_readout;
mod program;
mod ranking;
mod trainer;
mod types;

pub use active_battery::*;
pub use curriculum::*;
pub use era1_trials::*;
pub use evolution::*;
pub use founder_demonstrations::*;
pub use founder_readout::*;
pub use program::*;
pub use ranking::*;
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
