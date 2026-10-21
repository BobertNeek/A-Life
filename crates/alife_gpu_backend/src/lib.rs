//! wgpu backend for sparse GPU-authoritative neural compute.

pub mod closed_loop_activity;
pub mod closed_loop_admission;
pub mod closed_loop_buffers;
pub mod closed_loop_checkpoint;
pub mod closed_loop_development;
pub mod closed_loop_exact_capture;
mod closed_loop_exploration;
pub mod closed_loop_learning;
pub mod closed_loop_memory;
pub mod closed_loop_pipeline;
pub mod closed_loop_runtime;
pub mod closed_loop_sleep;
pub mod closed_loop_v11;
pub mod runtime;
#[cfg(feature = "training-rollout")]
pub mod training_rollout;
#[cfg(feature = "training-rollout")]
pub use training_rollout::{
    GpuTrainingBehaviorKind, GpuTrainingDemonstratorAction, GpuTrainingRolloutReceipt,
    GpuTrainingSamplingConfig,
};

pub use closed_loop_activity::*;
pub use closed_loop_admission::*;
pub use closed_loop_buffers::*;
pub use closed_loop_checkpoint::*;
pub use closed_loop_development::*;
pub use closed_loop_exact_capture::*;
pub(crate) use closed_loop_exploration::{
    game_candidate_exploration_identity, GAME_EXPLORATION_IDENTITY_WORDS,
};
pub use closed_loop_learning::*;
pub use closed_loop_memory::*;
pub use closed_loop_pipeline::*;
pub use closed_loop_runtime::*;
pub use closed_loop_sleep::*;
pub use closed_loop_v11::*;
pub use runtime::{
    probe_local_wgpu_runtime_for_graphics_backend, required_storage_buffers, GpuRuntimeBackendKind,
    GpuRuntimeHardwareProbe, P29_RUNTIME_SCHEMA_VERSION,
};
