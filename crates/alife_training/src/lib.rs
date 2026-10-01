//! Offline-only exact-graph WGSL foundation training.

mod active_battery;
mod curriculum;
mod era1_trials;
mod evolution;
mod founder_demonstrations;
mod founder_merge;
mod founder_readout;
mod ppo;
mod program;
mod ranking;
mod trainer;
mod types;

pub use active_battery::*;
pub use curriculum::*;
pub use era1_trials::*;
pub use evolution::*;
pub use founder_demonstrations::*;
pub use founder_merge::*;
pub use founder_readout::*;
pub use ppo::*;
pub use program::*;
pub use ranking::*;
pub use trainer::FoundationTrainer;
pub use types::*;

#[derive(Debug, thiserror::Error)]
pub enum TrainingError {
    #[error("{pipeline} is blocked: canonical organism biology and receptor-gated learning are not integrated")]
    CanonicalBiologyUnavailable { pipeline: &'static str },
    #[error("training world organism error: {0}")]
    Organism(#[from] alife_world::OrganismRegistryError),
    #[error("training contract error: {0}")]
    Contract(#[from] alife_core::ScaffoldContractError),
    #[error("GPU training submission failed")]
    GpuSubmission,
    #[error("GPU training readback was malformed")]
    MalformedReadback,
}

/// These legacy evaluators still construct synthetic physiology. Fail before
/// creating a GPU session or mutating a caller-owned world; removing this gate
/// requires integrating organism-owned biology and sealed receptor evidence.
pub(crate) fn require_canonical_training_biology(
    pipeline: &'static str,
) -> Result<(), TrainingError> {
    Err(TrainingError::CanonicalBiologyUnavailable { pipeline })
}
