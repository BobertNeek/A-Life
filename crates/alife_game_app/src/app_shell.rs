//! Application launch state and shared errors.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Error)]
pub enum GameAppShellError {
    #[cfg(feature = "gpu-runtime")]
    #[error("lineage archive error: {0}")]
    Archive(#[from] alife_archive::ArchiveError),
    #[cfg(feature = "gpu-runtime")]
    #[error("GPU runtime error: {0}")]
    GpuRuntime(#[from] alife_runtime::GpuRuntimeError),
    #[error("persistence/config error: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("core contract error: {0}")]
    Core(#[from] alife_core::ScaffoldContractError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("visible world mismatch: {message}")]
    VisibleWorldMismatch { message: &'static str },
    #[error("invalid graphical playground launch: {message}")]
    InvalidGraphicalLaunch { message: &'static str },
    #[error("required neural GPU backend unavailable: {message}")]
    NeuralBackendUnavailable { message: String },
    #[error(
        "GPU checkpoint manifest compare-and-swap conflict: expected {expected}, found {actual}"
    )]
    GpuCheckpointManifestConflict { expected: String, actual: String },
    #[error("invalid production voxel frontend launch: {message}")]
    InvalidProductionFrontend { message: String },
    #[error("environment launcher error: {message}")]
    EnvironmentManifest { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppShellLaunchConfig {
    pub fixture_root: PathBuf,
    pub config_path: PathBuf,
    pub asset_manifest_path: PathBuf,
    pub save_path: PathBuf,
    pub asset_root: PathBuf,
    pub start_paused: bool,
    pub brain_policy: PolicyBackend,
}

impl AppShellLaunchConfig {
    pub fn from_p34_fixture_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            config_path: root.join("tiny_config.json"),
            asset_manifest_path: root.join("tiny_asset_manifest.json"),
            save_path: root.join("tiny_save.json"),
            asset_root: root.clone(),
            fixture_root: root,
            start_paused: false,
            brain_policy: PolicyBackend::HeuristicBaseline,
        }
    }

    pub const fn with_brain_policy(mut self, brain_policy: PolicyBackend) -> Self {
        self.brain_policy = brain_policy;
        self
    }
}

pub fn validate_app_shell_config(launch: &AppShellLaunchConfig) -> Result<(), GameAppShellError> {
    if launch.brain_policy == PolicyBackend::NeuralClosedLoopCpu {
        return Err(GameAppShellError::NeuralBackendUnavailable {
            message:
                "CPU neural execution is available only through explicit offline training hosts"
                    .into(),
        });
    }
    let config = RuntimeConfig::from_json_file(&launch.config_path)?;
    config.validate()?;
    if config.brain_policy.policy == PolicyBackend::NeuralClosedLoopCpu {
        return Err(GameAppShellError::NeuralBackendUnavailable {
            message: "CPU training configurations cannot launch gameplay".into(),
        });
    }
    let manifest = AssetManifest::from_json_file(&launch.asset_manifest_path)?;
    manifest.validate_with_root(&launch.asset_root)?;

    Ok(())
}
