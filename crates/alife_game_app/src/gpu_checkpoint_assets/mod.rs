//! Production content-addressed GPU-brain checkpoint assets.
//!
//! Active ticks never call this module. It is the explicit sealed save/restore
//! boundary between portable world persistence and GPU-authoritative state.

mod content_store;
mod durable_manifest;
mod replay_codec;
mod state_codec;

pub use content_store::{merge_gpu_checkpoint_manifest_entries, GpuCheckpointAssetStore};
pub use durable_manifest::{
    GpuDurableSaveManifest, GpuLoadedSaveManifest, GpuSaveManifestCasOutcome, GpuSaveManifestDigest,
};
pub use state_codec::{
    GpuBrainCheckpointWrite, GpuBrainSidecarCapture, RestoredGpuBrainCheckpoint,
    RestoredRetainedLearning, RetainedLearningCapture,
};
