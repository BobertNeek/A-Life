//! Production 2.5D art pipeline contract.
//!
//! This validates the local Blender-to-Bevy sprite pipeline without making
//! Blender a runtime dependency. Missing Blender is reported as user action
//! required; the active game continues to use the committed alpha PNG pack.

use std::{
    collections::BTreeSet,
    env,
    ffi::OsString,
    path::{Component, Path, PathBuf},
    process::Command,
};

use crate::prelude::*;
use crate::*;

pub const PRODUCTION_ASSET_PIPELINE_MANIFEST_RELATIVE_PATH: &str =
    "crates/alife_game_app/assets/alpha_art_v1/blender_pipeline_manifest.json";
pub const PRODUCTION_ASSET_PIPELINE_SCRIPT_RELATIVE_PATH: &str =
    "scripts/render_alpha_art_blender_sprites.ps1";
pub const PRODUCTION_ASSET_PIPELINE_BLENDER_PY_RELATIVE_PATH: &str =
    "tools/blender/render_alpha_art_v1.py";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProductionAssetPipelineManifest {
    pub schema: String,
    pub schema_version: u16,
    pub pipeline_id: String,
    pub art_direction: String,
    pub active_alpha_art_pack: String,
    pub blender_runtime_required_for_generation: bool,
    pub runtime_game_dependency: bool,
    pub render_backend: String,
    pub output_dir: String,
    pub source_script: String,
    pub launcher_script: String,
    pub roles: Vec<ProductionAssetPipelineRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProductionAssetPipelineRole {
    pub role: String,
    pub source_kind: String,
    pub active_png_path: String,
    pub blender_target_png_path: String,
    pub required_model_features: Vec<String>,
    pub expected_render_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductionAssetPipelineSmokeSummary {
    pub schema: String,
    pub schema_version: u16,
    pub pipeline_id: String,
    pub art_direction: String,
    pub role_count: usize,
    pub required_roles_present: bool,
    pub active_pngs_exist: bool,
    pub blender_script_present: bool,
    pub launcher_script_present: bool,
    pub output_dir_ignored_target: bool,
    pub blender_on_path: bool,
    pub blender_discovered: bool,
    pub blender_executable: Option<String>,
    pub local_render_status: String,
    pub user_action_required: bool,
    pub runtime_game_dependency: bool,
    pub can_emit_actions: bool,
    pub can_rewrite_weights: bool,
    pub can_change_simulation_semantics: bool,
}

impl ProductionAssetPipelineSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != PRODUCTION_ASSET_PIPELINE_SCHEMA
            || self.schema_version != PRODUCTION_ASSET_PIPELINE_SCHEMA_VERSION
            || self.pipeline_id.trim().is_empty()
            || self.role_count < PRODUCTION_ASSET_PIPELINE_MIN_ROLES
            || self.role_count > PRODUCTION_ASSET_PIPELINE_MAX_ROLES
            || !self.required_roles_present
            || !self.active_pngs_exist
            || !self.blender_script_present
            || !self.launcher_script_present
            || !self.output_dir_ignored_target
            || self.runtime_game_dependency
            || self.can_emit_actions
            || self.can_rewrite_weights
            || self.can_change_simulation_semantics
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "production asset pipeline smoke violates the Blender/2.5D boundary",
            });
        }
        if self.blender_discovered {
            if self.local_render_status != "READY_TO_RENDER" || self.user_action_required {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "Blender is available but pipeline did not report ready",
                });
            }
        } else if self.local_render_status != "USER_ACTION_REQUIRED" || !self.user_action_required {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "missing Blender must be reported as USER_ACTION_REQUIRED",
            });
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:roles={}:active_pngs={}:blender_on_path={}:blender_discovered={}:status={}:user_action_required={}:runtime_dependency={}:actions={}:weights={}:semantics={}",
            self.schema,
            self.schema_version,
            self.pipeline_id,
            self.role_count,
            self.active_pngs_exist,
            self.blender_on_path,
            self.blender_discovered,
            self.local_render_status,
            self.user_action_required,
            self.runtime_game_dependency,
            self.can_emit_actions,
            self.can_rewrite_weights,
            self.can_change_simulation_semantics
        )
    }
}

pub fn default_production_asset_pipeline_manifest_path() -> PathBuf {
    ca12_workspace_root().join(PRODUCTION_ASSET_PIPELINE_MANIFEST_RELATIVE_PATH)
}

pub fn run_production_asset_pipeline_smoke(
) -> Result<ProductionAssetPipelineSmokeSummary, GameAppShellError> {
    let root = ca12_workspace_root();
    let manifest_path = default_production_asset_pipeline_manifest_path();
    let manifest: ProductionAssetPipelineManifest =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    validate_production_asset_pipeline_manifest(&root, &manifest)?;
    let blender_on_path = blender_available_on_path();
    let blender_executable = discover_blender_executable();
    let blender_discovered = blender_executable.is_some();
    let summary = ProductionAssetPipelineSmokeSummary {
        schema: PRODUCTION_ASSET_PIPELINE_SCHEMA.to_string(),
        schema_version: PRODUCTION_ASSET_PIPELINE_SCHEMA_VERSION,
        pipeline_id: manifest.pipeline_id,
        art_direction: manifest.art_direction,
        role_count: manifest.roles.len(),
        required_roles_present: production_pipeline_required_roles_present(&manifest.roles),
        active_pngs_exist: manifest
            .roles
            .iter()
            .all(|role| root.join(&role.active_png_path).is_file()),
        blender_script_present: root.join(&manifest.source_script).is_file(),
        launcher_script_present: root.join(&manifest.launcher_script).is_file(),
        output_dir_ignored_target: manifest.output_dir.starts_with("target/")
            || manifest.output_dir == "target",
        blender_on_path,
        blender_discovered,
        blender_executable: blender_executable
            .as_ref()
            .map(|path| path.to_string_lossy().to_string()),
        local_render_status: if blender_discovered {
            "READY_TO_RENDER".to_string()
        } else {
            "USER_ACTION_REQUIRED".to_string()
        },
        user_action_required: !blender_discovered,
        runtime_game_dependency: manifest.runtime_game_dependency,
        can_emit_actions: false,
        can_rewrite_weights: false,
        can_change_simulation_semantics: false,
    };
    summary.validate()?;
    Ok(summary)
}

pub fn validate_production_asset_pipeline_manifest(
    root: &Path,
    manifest: &ProductionAssetPipelineManifest,
) -> Result<(), GameAppShellError> {
    if manifest.schema != PRODUCTION_ASSET_PIPELINE_SCHEMA
        || manifest.schema_version != PRODUCTION_ASSET_PIPELINE_SCHEMA_VERSION
        || manifest.pipeline_id.trim().is_empty()
        || manifest.art_direction != "production-blender-to-bevy-2-5d-v1"
        || manifest.active_alpha_art_pack != CA44A_ALPHA_ART_DIRECTION
        || !manifest.blender_runtime_required_for_generation
        || manifest.runtime_game_dependency
        || manifest.render_backend != "blender-eevee-toon-orthographic"
        || manifest.roles.len() < PRODUCTION_ASSET_PIPELINE_MIN_ROLES
        || manifest.roles.len() > PRODUCTION_ASSET_PIPELINE_MAX_ROLES
        || !production_pipeline_required_roles_present(&manifest.roles)
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    validate_pipeline_relative_path(&manifest.output_dir, true)?;
    validate_pipeline_relative_path(&manifest.source_script, false)?;
    validate_pipeline_relative_path(&manifest.launcher_script, false)?;
    if !root.join(&manifest.source_script).is_file()
        || !root.join(&manifest.launcher_script).is_file()
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let mut render_targets = BTreeSet::new();
    for role in &manifest.roles {
        validate_pipeline_role(root, role, &mut render_targets)?;
    }
    Ok(())
}

fn validate_pipeline_role(
    root: &Path,
    role: &ProductionAssetPipelineRole,
    render_targets: &mut BTreeSet<String>,
) -> Result<(), GameAppShellError> {
    require_pipeline_id(&role.role)?;
    if !render_targets.insert(role.blender_target_png_path.clone())
        || role.source_kind != "low-poly-toon-blender"
        || role.expected_render_count == 0
        || role.required_model_features.is_empty()
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    validate_pipeline_relative_path(&role.active_png_path, false)?;
    validate_pipeline_relative_path(&role.blender_target_png_path, true)?;
    if !role.active_png_path.ends_with(".png")
        || !role.blender_target_png_path.ends_with(".png")
        || !root.join(&role.active_png_path).is_file()
        || !role.blender_target_png_path.starts_with("target/")
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn require_pipeline_id(value: &str) -> Result<(), GameAppShellError> {
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn production_pipeline_required_roles_present(roles: &[ProductionAssetPipelineRole]) -> bool {
    let role_names = roles
        .iter()
        .map(|role| role.role.as_str())
        .collect::<BTreeSet<_>>();
    [
        "creature-idle",
        "creature-hurt",
        "selection-ring",
        "food",
        "hazard",
        "rock-obstacle",
        "terrain-safe-grass",
        "terrain-soil-path",
        "terrain-resource-grove",
        "terrain-hazard-pressure",
        "terrain-stone-rough",
        "terrain-water",
        "terrain-sand",
        "prop-dressing",
    ]
    .iter()
    .all(|role| role_names.contains(role))
}

fn validate_pipeline_relative_path(
    relative: &str,
    allow_target: bool,
) -> Result<(), GameAppShellError> {
    if relative.trim().is_empty() {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                let lower = name.to_string_lossy().to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "logs" | "captures" | "screenshots" | ".cache" | "models"
                ) || (!allow_target
                    && matches!(lower.as_str(), "target" | "artifacts" | "cache"))
                {
                    return Err(ScaffoldContractError::MissingPhaseData.into());
                }
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ScaffoldContractError::MissingPhaseData.into())
            }
            Component::CurDir => {}
        }
    }
    Ok(())
}

fn blender_available_on_path() -> bool {
    blender_command_succeeds(OsString::from("blender"))
}

fn discover_blender_executable() -> Option<PathBuf> {
    if let Some(path) = env::var_os("BLENDER_EXE").map(PathBuf::from) {
        if path.is_file() && blender_command_succeeds(path.as_os_str().to_os_string()) {
            return Some(path);
        }
    }

    if blender_available_on_path() {
        return Some(PathBuf::from("blender"));
    }

    standard_windows_blender_candidates()
        .into_iter()
        .find(|candidate| blender_command_succeeds(candidate.as_os_str().to_os_string()))
}

fn standard_windows_blender_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for root in [
        env::var_os("ProgramFiles").map(|path| PathBuf::from(path).join("Blender Foundation")),
        env::var_os("LOCALAPPDATA").map(|path| {
            PathBuf::from(path)
                .join("Programs")
                .join("Blender Foundation")
        }),
    ]
    .into_iter()
    .flatten()
    {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten() {
            let candidate = entry.path().join("blender.exe");
            if candidate.is_file() {
                candidates.push(candidate);
            }
        }
    }
    candidates.sort_by(|left, right| {
        right
            .parent()
            .and_then(Path::file_name)
            .cmp(&left.parent().and_then(Path::file_name))
            .then_with(|| right.cmp(left))
    });
    candidates
}

fn blender_command_succeeds(command: OsString) -> bool {
    Command::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_asset_pipeline_manifest_validates_without_blender_runtime_dependency() {
        let root = ca12_workspace_root();
        let manifest_path = default_production_asset_pipeline_manifest_path();
        let manifest: ProductionAssetPipelineManifest =
            serde_json::from_str(&std::fs::read_to_string(manifest_path).unwrap()).unwrap();
        validate_production_asset_pipeline_manifest(&root, &manifest).unwrap();
        assert!(!manifest.runtime_game_dependency);
        assert_eq!(manifest.render_backend, "blender-eevee-toon-orthographic");
        assert!(production_pipeline_required_roles_present(&manifest.roles));
    }

    #[test]
    fn production_asset_pipeline_smoke_reports_discovered_or_missing_blender() {
        let summary = run_production_asset_pipeline_smoke().unwrap();
        summary.validate().unwrap();
        assert!(!summary.runtime_game_dependency);
        assert!(!summary.can_emit_actions);
        assert!(!summary.can_rewrite_weights);
        assert!(!summary.can_change_simulation_semantics);
        if !summary.blender_discovered {
            assert_eq!(summary.local_render_status, "USER_ACTION_REQUIRED");
            assert!(summary.user_action_required);
        }
    }

    #[test]
    fn production_asset_pipeline_rejects_target_as_active_png_source() {
        let root = ca12_workspace_root();
        let manifest_path = default_production_asset_pipeline_manifest_path();
        let mut manifest: ProductionAssetPipelineManifest =
            serde_json::from_str(&std::fs::read_to_string(manifest_path).unwrap()).unwrap();
        manifest.roles[0].active_png_path =
            "target/generated_art/alpha_blender_v1/bad.png".to_string();
        assert!(validate_production_asset_pipeline_manifest(&root, &manifest).is_err());
    }
}
