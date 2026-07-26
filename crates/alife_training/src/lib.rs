//! Offline-only exact-graph WGSL foundation training.

mod curriculum;
mod evolution;
mod program;
mod ranking;
mod trainer;
mod types;

pub use curriculum::*;
pub use evolution::*;
pub use program::*;
pub use ranking::*;
pub use trainer::FoundationTrainer;
pub use types::*;

#[derive(Debug, thiserror::Error)]
pub enum TrainingError {
    #[error("training contract error: {0}")]
    Contract(#[from] alife_core::ScaffoldContractError),
    #[error("GPU training submission failed")]
    GpuSubmission,
    #[error("GPU training readback was malformed")]
    MalformedReadback,
}
