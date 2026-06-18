//! Local packaging and platform smoke contract for G21.
//!
//! This module is validation metadata only. It records how to build/run local
//! package smoke commands and validates a tiny asset bundle manifest without
//! publishing releases, signing artifacts, or making graphics/GPU paths
//! mandatory.

use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageSmokeKind {
    Headless,
    GraphicalManual,
    AssetBundle,
    Validation,
}

impl PackageSmokeKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Headless => "headless",
            Self::GraphicalManual => "graphical-manual",
            Self::AssetBundle => "asset-bundle",
            Self::Validation => "validation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformPackageCommand {
    pub id: String,
    pub kind: PackageSmokeKind,
    pub windows_command: String,
    pub non_windows_command: String,
    pub manual: bool,
    pub requires_graphics: bool,
    pub requires_gpu: bool,
}

impl PlatformPackageCommand {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.id.is_empty()
            || self.windows_command.is_empty()
            || self.non_windows_command.is_empty()
            || self.windows_command.contains("bash scripts/check.sh")
            || self.non_windows_command.contains("bash scripts/check.sh")
            || self.windows_command.contains("gpu-report")
            || self.non_windows_command.contains("gpu-report")
            || self.windows_command.contains("ALIFE_GPU_BACKEND")
            || self.non_windows_command.contains("ALIFE_GPU_BACKEND")
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        if (self.requires_graphics || self.requires_gpu) && !self.manual {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphics and GPU package smoke commands must be manual",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetBundleEntry {
    pub asset_id: String,
    pub kind: String,
    pub relative_path: String,
    pub required: bool,
    pub max_size_bytes: u64,
}

impl AssetBundleEntry {
    pub fn validate_with_root(&self, root: &Path) -> Result<(), GameAppShellError> {
        if self.asset_id.is_empty()
            || self.kind.is_empty()
            || self.relative_path.is_empty()
            || self.max_size_bytes == 0
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        let relative = Path::new(&self.relative_path);
        if relative.is_absolute()
            || relative
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "asset bundle paths must be relative and portable",
            });
        }
        let path = root.join(relative);
        if !path.exists() {
            if self.required {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "required asset bundle entry is missing",
                });
            }
            return Ok(());
        }
        let len = std::fs::metadata(path)?.len();
        if len > self.max_size_bytes {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "asset bundle entry exceeds declared size cap",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetBundleManifest {
    pub schema: String,
    pub schema_version: u16,
    pub bundle_id: String,
    pub output_directory: String,
    pub entries: Vec<AssetBundleEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetBundleValidation {
    pub entry_count: usize,
    pub required_count: usize,
    pub optional_count: usize,
}

impl AssetBundleManifest {
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, GameAppShellError> {
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn validate_with_root(
        &self,
        root: impl AsRef<Path>,
    ) -> Result<AssetBundleValidation, GameAppShellError> {
        if self.schema != G21_ASSET_BUNDLE_SCHEMA
            || self.schema_version != G21_ASSET_BUNDLE_SCHEMA_VERSION
            || self.bundle_id.is_empty()
            || self.entries.is_empty()
            || self.entries.len() > G21_MAX_BUNDLE_ENTRIES
            || !self.output_directory.starts_with("target/artifacts/")
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        let mut ids = BTreeSet::new();
        let mut required_count = 0usize;
        for entry in &self.entries {
            if !ids.insert(entry.asset_id.as_str()) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "asset bundle IDs must be unique",
                });
            }
            if entry.required {
                required_count += 1;
            }
            entry.validate_with_root(root.as_ref())?;
        }
        Ok(AssetBundleValidation {
            entry_count: self.entries.len(),
            required_count,
            optional_count: self.entries.len() - required_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformPackageSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub output_directory: &'static str,
    pub commands: Vec<PlatformPackageCommand>,
    pub asset_bundle_entries: usize,
    pub required_asset_entries: usize,
    pub optional_asset_entries: usize,
    pub generated_artifacts_tracked: bool,
    pub windows_wrappers_used: bool,
    pub release_publishing_attempted: bool,
}

impl PlatformPackageSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != G21_PLATFORM_PACKAGE_SCHEMA
            || self.schema_version != G21_PLATFORM_PACKAGE_SCHEMA_VERSION
            || !self.output_directory.starts_with("target/artifacts/")
            || self.commands.is_empty()
            || self.asset_bundle_entries == 0
            || self.generated_artifacts_tracked
            || !self.windows_wrappers_used
            || self.release_publishing_attempted
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        let mut ids = BTreeSet::new();
        let mut has_headless = false;
        let mut has_graphical_manual = false;
        for command in &self.commands {
            if !ids.insert(command.id.as_str()) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "platform package command IDs must be unique",
                });
            }
            command.validate()?;
            has_headless |= command.kind == PackageSmokeKind::Headless && !command.manual;
            has_graphical_manual |=
                command.kind == PackageSmokeKind::GraphicalManual && command.manual;
        }
        if !has_headless || !has_graphical_manual {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "G21 package smoke must include CI headless and manual graphical paths",
            });
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.commands.len(),
            self.asset_bundle_entries,
            self.required_asset_entries,
            self.optional_asset_entries,
            self.output_directory
        )
    }
}

pub fn g21_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn g21_asset_bundle_manifest_path() -> PathBuf {
    g21_workspace_root().join("examples/g21/platform_asset_bundle_manifest.json")
}

pub fn load_g21_asset_bundle_manifest() -> Result<AssetBundleManifest, GameAppShellError> {
    AssetBundleManifest::from_json_file(g21_asset_bundle_manifest_path())
}

pub fn run_platform_package_smoke() -> Result<PlatformPackageSummary, GameAppShellError> {
    let root = g21_workspace_root();
    let manifest = load_g21_asset_bundle_manifest()?;
    let validation = manifest.validate_with_root(&root)?;
    let docs = std::fs::read_to_string(root.join("docs/playable_sim_spec/platform_packaging.md"))?;
    if !docs.contains("scripts/run_headless_playground.ps1")
        || !docs.contains("scripts/run_graphical_playground.ps1")
        || !docs.contains("powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1")
        || docs.contains("bash scripts/check.sh")
    {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "platform packaging docs must reference wrapper-safe commands",
        });
    }
    let summary = PlatformPackageSummary {
        schema: G21_PLATFORM_PACKAGE_SCHEMA,
        schema_version: G21_PLATFORM_PACKAGE_SCHEMA_VERSION,
        output_directory: "target/artifacts/g21_local_package",
        commands: platform_package_commands(),
        asset_bundle_entries: validation.entry_count,
        required_asset_entries: validation.required_count,
        optional_asset_entries: validation.optional_count,
        generated_artifacts_tracked: tracked_generated_artifacts_present(&root)?,
        windows_wrappers_used: true,
        release_publishing_attempted: false,
    };
    summary.validate()?;
    Ok(summary)
}

pub fn platform_package_commands() -> Vec<PlatformPackageCommand> {
    vec![
        PlatformPackageCommand {
            id: "headless-run-script-dry-run".to_string(),
            kind: PackageSmokeKind::Headless,
            windows_command:
                "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_headless_playground.ps1 -DryRun"
                    .to_string(),
            non_windows_command: "./scripts/run_headless_playground.sh --dry-run".to_string(),
            manual: false,
            requires_graphics: false,
            requires_gpu: false,
        },
        PlatformPackageCommand {
            id: "headless-run-script".to_string(),
            kind: PackageSmokeKind::Headless,
            windows_command:
                "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_headless_playground.ps1"
                    .to_string(),
            non_windows_command: "./scripts/run_headless_playground.sh".to_string(),
            manual: false,
            requires_graphics: false,
            requires_gpu: false,
        },
        PlatformPackageCommand {
            id: "graphical-run-script-manual".to_string(),
            kind: PackageSmokeKind::GraphicalManual,
            windows_command:
                "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun"
                    .to_string(),
            non_windows_command: "./scripts/run_graphical_playground.sh --dry-run".to_string(),
            manual: true,
            requires_graphics: true,
            requires_gpu: false,
        },
        PlatformPackageCommand {
            id: "asset-bundle-manifest".to_string(),
            kind: PackageSmokeKind::AssetBundle,
            windows_command:
                "cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke"
                    .to_string(),
            non_windows_command:
                "cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke"
                    .to_string(),
            manual: false,
            requires_graphics: false,
            requires_gpu: false,
        },
        PlatformPackageCommand {
            id: "windows-validation-wrapper".to_string(),
            kind: PackageSmokeKind::Validation,
            windows_command:
                "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1"
                    .to_string(),
            non_windows_command: "./scripts/check.sh".to_string(),
            manual: false,
            requires_graphics: false,
            requires_gpu: false,
        },
    ]
}

fn tracked_generated_artifacts_present(root: &Path) -> Result<bool, GameAppShellError> {
    let output = std::process::Command::new("git")
        .args([
            "ls-files",
            "target",
            "dist",
            "target/artifacts",
            "graphify-out",
        ])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "git ls-files failed while checking generated artifacts",
        });
    }
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}
