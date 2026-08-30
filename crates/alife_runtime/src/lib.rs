//! Shared GPU-authoritative brain runtime boundary.

mod checkpoint_assets;
mod session;
mod sleep_scheduler;

pub use checkpoint_assets::{
    current_backend_provenance, merge_gpu_checkpoint_manifest_entries, GpuBrainCheckpointWrite,
    GpuBrainSidecarCapture, GpuCheckpointAssetStore, GpuDurableFounderWrite,
    GpuDurableSaveManifest, GpuExactCheckpointTransactionContextV1, GpuLoadedSaveManifest,
    GpuSaveManifestCasOutcome, GpuSaveManifestDigest, GpuSleepJournalPublicationReceipt,
    GpuSleepJournalPublicationTiming, GpuSleepTransactionJournalEntryV2,
    GpuSleepTransactionJournalV2, RestoredGpuBrainCheckpoint, RestoredRetainedLearning,
    RetainedLearningCapture,
};
pub use session::*;
pub use sleep_scheduler::*;

#[derive(Debug, thiserror::Error)]
pub enum GpuRuntimeError {
    #[error("persistence/config error: {0}")]
    Persistence(#[from] alife_world::persistence::PersistenceError),
    #[error("core contract error: {0}")]
    Core(#[from] alife_core::ScaffoldContractError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error(
        "GPU checkpoint manifest compare-and-swap conflict: expected {expected}, found {actual}"
    )]
    GpuCheckpointManifestConflict { expected: String, actual: String },
    #[error(
        "GPU checkpoint authority generation {committed_generation} committed but post-commit validation failed; last known-good generation was {last_known_good_generation}: {message}"
    )]
    GpuCheckpointAuthorityPostCommitValidation {
        committed_generation: u64,
        last_known_good_generation: u64,
        message: String,
    },
    #[error("invalid GPU checkpoint boundary: {message}")]
    InvalidProductionFrontend { message: String },
}

// The moved codecs used this local name before the runtime crate existed.
// Keep it private so the implementation can remain byte-for-byte stable.
type GameAppShellError = GpuRuntimeError;
