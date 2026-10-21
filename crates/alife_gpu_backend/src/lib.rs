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
    probe_local_wgpu_runtime, probe_local_wgpu_runtime_for_graphics_backend,
    probe_local_wgpu_runtime_with_backends, required_storage_buffers, GpuRuntimeBackendConfig,
    GpuRuntimeBackendKind, GpuRuntimeBackendStatus, GpuRuntimeHardwareProbe,
    P29_RUNTIME_SCHEMA_VERSION,
};

/// BLAKE3 identity of every WGSL source used by the GPU-authoritative closed
/// loop. Names and lengths are included so concatenation cannot alias bundles.
pub fn closed_loop_shader_bundle_digest() -> alife_core::Blake3Digest {
    let sources = [
        ("abi", CLOSED_LOOP_ABI_WGSL),
        ("encode", CLOSED_LOOP_ENCODE_WGSL),
        ("clear-diagnostics", CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL),
        ("recurrent", CLOSED_LOOP_RECURRENT_WGSL),
        ("decode", CLOSED_LOOP_DECODE_WGSL),
        ("memory-context", CLOSED_LOOP_MEMORY_CONTEXT_WGSL),
        ("eligibility", CLOSED_LOOP_ELIGIBILITY_WGSL),
        ("plasticity", CLOSED_LOOP_PLASTICITY_WGSL),
        ("consolidate", CLOSED_LOOP_CONSOLIDATE_WGSL),
        ("replay-learning", CLOSED_LOOP_REPLAY_LEARNING_WGSL),
    ];
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"alife.gpu.closed-loop.wgsl-bundle.v1");
    for (name, source) in sources {
        #[cfg(feature = "training-rollout")]
        let source = training_rollout::shader_source(source);
        hasher.update(&(name.len() as u64).to_le_bytes());
        hasher.update(name.as_bytes());
        hasher.update(&(source.len() as u64).to_le_bytes());
        hasher.update(source.as_bytes());
    }
    alife_core::Blake3Digest::from_bytes(*hasher.finalize().as_bytes())
}
