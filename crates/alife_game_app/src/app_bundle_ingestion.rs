//! CA12 app bundle discovery and validation.
//!
//! This is metadata validation only. It makes the app's config, shader, and
//! placeholder art assets discoverable without generating or committing package
//! artifacts.

use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use serde::Deserialize;

use crate::prelude::*;
use crate::*;

pub const CA12_APP_BUNDLE_MANIFEST_FILE: &str = "app_bundle_manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AppBundleManifest {
    pub schema: String,
    pub schema_version: u16,
    pub bundle_id: String,
    pub environment_manifest: String,
    pub placeholder_art_manifest: String,
    pub alpha_art_manifest: String,
    pub true_25d_asset_manifest: String,
    pub production_voxel_asset_manifest: String,
    pub entries: Vec<AppBundleEntry>,
    pub shader_assets: Vec<ShaderAssetEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AppBundleEntry {
    pub id: String,
    pub kind: String,
    pub relative_path: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ShaderAssetEntry {
    pub id: String,
    pub relative_path: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlaceholderArtManifest {
    pub schema: String,
    pub schema_version: u16,
    pub manifest_id: String,
    pub entries: Vec<PlaceholderArtEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PlaceholderArtEntry {
    pub id: String,
    pub kind: String,
    pub shape: String,
    pub color: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppBundleIngestionSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub bundle_id: String,
    pub manifest_path: PathBuf,
    pub environment_scenarios: usize,
    pub config_entries: usize,
    pub shader_assets: usize,
    pub discovered_shader_assets: usize,
    pub placeholder_art_entries: usize,
    pub alpha_art_entries: usize,
    pub alpha_art_required_roles_present: bool,
    pub production_alpha_art: bool,
    pub true_25d_asset_entries: usize,
    pub true_25d_required_roles_present: bool,
    pub true_25d_endocrine_feedback_assets: usize,
    pub true_25d_endocrine_feedback_contract_validated: bool,
    pub production_true_25d_assets: bool,
    pub production_voxel_asset_entries: usize,
    pub production_voxel_generated_assets: usize,
    pub production_voxel_asset_manifest_validated: bool,
    pub required_entries: usize,
    pub largest_file_bytes: u64,
    pub missing_required_rejected: bool,
    pub shader_discovery_complete: bool,
    pub tiny_placeholder_art: bool,
    pub large_binary_assets_committed: bool,
    pub player_visible_status: Vec<String>,
}

impl AppBundleIngestionSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA12_APP_BUNDLE_MANIFEST_SCHEMA
            || self.schema_version != CA12_APP_BUNDLE_MANIFEST_SCHEMA_VERSION
            || self.bundle_id.trim().is_empty()
            || self.environment_scenarios == 0
            || self.config_entries == 0
            || self.shader_assets == 0
            || self.discovered_shader_assets == 0
            || self.shader_assets != self.discovered_shader_assets
            || self.placeholder_art_entries < 4
            || self.alpha_art_entries < CA44A_REQUIRED_ALPHA_ART_ROLES
            || !self.alpha_art_required_roles_present
            || !self.production_alpha_art
            || self.true_25d_asset_entries < TRUE_25D_ALPHA_MIN_REQUIRED_ROLES
            || !self.true_25d_required_roles_present
            || !self.true_25d_endocrine_feedback_contract_validated
            || !self.production_true_25d_assets
            || self.production_voxel_asset_entries < FVR07_REQUIRED_USAGE_CATEGORIES.len()
            || self.production_voxel_generated_assets == 0
            || !self.production_voxel_asset_manifest_validated
            || self.required_entries == 0
            || self.largest_file_bytes > CA44A_MAX_ALPHA_ART_BACKDROP_BYTES
            || !self.missing_required_rejected
            || !self.shader_discovery_complete
            || !self.tiny_placeholder_art
            || self.large_binary_assets_committed
            || self.player_visible_status.is_empty()
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:entries={}:shaders={}/{}:art={}:true25d={}:production_voxel_assets={}:endocrine={}:largest={}",
            self.schema,
            self.schema_version,
            self.bundle_id,
            self.config_entries,
            self.shader_assets,
            self.discovered_shader_assets,
            self.alpha_art_entries,
            self.true_25d_asset_entries,
            self.production_voxel_asset_entries,
            self.true_25d_endocrine_feedback_assets,
            self.largest_file_bytes
        )
    }
}

pub fn ca12_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_game_app should live under crates/")
        .to_path_buf()
}

pub fn default_app_bundle_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CA12_APP_BUNDLE_MANIFEST_FILE)
}

pub fn validate_app_bundle_manifest(
    manifest_path: impl AsRef<Path>,
) -> Result<AppBundleIngestionSummary, GameAppShellError> {
    let manifest_path = manifest_path.as_ref();
    let root = ca12_workspace_root();
    let manifest: AppBundleManifest = read_json(manifest_path)?;
    let summary = validate_app_bundle_manifest_inner(&root, manifest_path, &manifest)?;

    let mut broken = manifest.clone();
    if let Some(entry) = broken.entries.first_mut() {
        entry.relative_path =
            "crates/alife_world/tests/fixtures/gpu_alpha/missing_config.json".to_string();
    }
    let missing_required_rejected =
        validate_app_bundle_manifest_inner(&root, manifest_path, &broken).is_err();

    let summary = AppBundleIngestionSummary {
        missing_required_rejected,
        ..summary
    };
    summary.validate()?;
    Ok(summary)
}

fn validate_app_bundle_manifest_inner(
    root: &Path,
    manifest_path: &Path,
    manifest: &AppBundleManifest,
) -> Result<AppBundleIngestionSummary, GameAppShellError> {
    require_schema(
        &manifest.schema,
        manifest.schema_version,
        CA12_APP_BUNDLE_MANIFEST_SCHEMA,
        CA12_APP_BUNDLE_MANIFEST_SCHEMA_VERSION,
    )?;
    require_id(&manifest.bundle_id)?;
    if manifest.entries.is_empty()
        || manifest.entries.len() > CA12_MAX_BUNDLE_ENTRIES
        || manifest.shader_assets.is_empty()
        || manifest.shader_assets.len() > CA12_MAX_BUNDLE_ENTRIES
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }

    let mut largest_file_bytes = tiny_file_size(manifest_path)?;
    let environment_manifest_path = resolve_workspace_path(root, &manifest.environment_manifest)?;
    let environment_manifest = EnvironmentManifest::from_json_file(&environment_manifest_path)?;
    environment_manifest.validate(&environment_manifest_path)?;
    largest_file_bytes = largest_file_bytes.max(tiny_file_size(&environment_manifest_path)?);

    let placeholder_path = resolve_workspace_path(root, &manifest.placeholder_art_manifest)?;
    let placeholder_art = validate_placeholder_art_manifest(&placeholder_path)?;
    largest_file_bytes = largest_file_bytes.max(tiny_file_size(&placeholder_path)?);
    let alpha_art_path = resolve_workspace_path(root, &manifest.alpha_art_manifest)?;
    let alpha_art = validate_alpha_art_manifest_inner(
        root,
        &alpha_art_path,
        &read_json(&alpha_art_path)?,
        true,
    )?;
    largest_file_bytes = largest_file_bytes.max(alpha_art.largest_file_bytes);

    let true_25d_path = resolve_workspace_path(root, &manifest.true_25d_asset_manifest)?;
    let true_25d =
        validate_true_25d_asset_manifest_inner(root, &true_25d_path, &read_json(&true_25d_path)?)?;
    largest_file_bytes = largest_file_bytes.max(true_25d.largest_file_bytes);

    let production_voxel_asset_path =
        resolve_workspace_path(root, &manifest.production_voxel_asset_manifest)?;
    let production_voxel_assets = validate_production_assets(&production_voxel_asset_path)?;
    largest_file_bytes = largest_file_bytes
        .max(tiny_file_size(&production_voxel_asset_path)?)
        .max(production_voxel_assets.largest_asset_bytes);

    let mut ids = BTreeSet::new();
    let mut required_entries = 0;
    let mut large_binary_assets_committed = has_binary_like_extension(&placeholder_path);
    for entry in &manifest.entries {
        validate_entry(entry, &mut ids)?;
        if entry.required {
            required_entries += 1;
        }
        let path = resolve_workspace_path(root, &entry.relative_path)?;
        if !path.exists() {
            if entry.required {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "required app bundle entry is missing",
                });
            }
            continue;
        }
        validate_bundle_entry_kind(entry, &path)?;
        largest_file_bytes = largest_file_bytes.max(tiny_file_size(&path)?);
        large_binary_assets_committed |= has_binary_like_extension(&path);
    }

    let mut shader_ids = BTreeSet::new();
    for shader in &manifest.shader_assets {
        validate_shader_entry(shader, &mut shader_ids)?;
        let path = resolve_workspace_path(root, &shader.relative_path)?;
        if !path.exists() {
            if shader.required {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "required shader asset is missing",
                });
            }
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("wgsl") {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        largest_file_bytes = largest_file_bytes.max(tiny_file_size(&path)?);
    }
    let discovered_shader_assets = discover_workspace_shaders(root)?.len();

    Ok(AppBundleIngestionSummary {
        schema: CA12_APP_BUNDLE_MANIFEST_SCHEMA,
        schema_version: CA12_APP_BUNDLE_MANIFEST_SCHEMA_VERSION,
        bundle_id: manifest.bundle_id.clone(),
        manifest_path: manifest_path.to_path_buf(),
        environment_scenarios: environment_manifest.scenarios.len(),
        config_entries: manifest.entries.len(),
        shader_assets: manifest.shader_assets.len(),
        discovered_shader_assets,
        placeholder_art_entries: placeholder_art.entries.len(),
        alpha_art_entries: alpha_art.entry_count,
        alpha_art_required_roles_present: alpha_art.required_roles_present,
        production_alpha_art: alpha_art.required_roles_present
            && alpha_art.png_dimensions_validated
            && alpha_art.largest_file_bytes <= CA44A_MAX_ALPHA_ART_BACKDROP_BYTES
            && alpha_art.entry_count >= CA44A_REQUIRED_ALPHA_ART_ROLES
            && alpha_art.pack_id == "alpha-art-v1",
        true_25d_asset_entries: true_25d.entry_count,
        true_25d_required_roles_present: true_25d.required_roles_present,
        true_25d_endocrine_feedback_assets: true_25d.endocrine_feedback_assets,
        true_25d_endocrine_feedback_contract_validated: true_25d
            .endocrine_feedback_contract_validated,
        production_true_25d_assets: true_25d.required_roles_present
            && true_25d.gltf_files_validated
            && true_25d.orthographic_camera_locked
            && true_25d.shader_stack_declared
            && true_25d.endocrine_feedback_contract_validated
            && true_25d.no_action_authority
            && true_25d.largest_file_bytes <= TRUE_25D_ALPHA_MAX_ASSET_BYTES
            && true_25d.pack_id == "true-25d-alpha-v1",
        production_voxel_asset_entries: production_voxel_assets.asset_count,
        production_voxel_generated_assets: production_voxel_assets.generated_assets,
        production_voxel_asset_manifest_validated: production_voxel_assets
            .required_usage_categories_present
            && production_voxel_assets.placeholder_final_entries == 0
            && production_voxel_assets.unknown_license_entries == 0
            && production_voxel_assets.missing_or_rejected_assets == 0
            && production_voxel_assets.display_only_vfx
            && production_voxel_assets.no_renderer_authority,
        required_entries,
        largest_file_bytes,
        missing_required_rejected: false,
        shader_discovery_complete: discovered_shader_assets == manifest.shader_assets.len(),
        tiny_placeholder_art: placeholder_art.entries.iter().all(|entry| {
            !entry.id.trim().is_empty()
                && !entry.kind.trim().is_empty()
                && !entry.shape.trim().is_empty()
                && !entry.color.trim().is_empty()
        }),
        large_binary_assets_committed,
        player_visible_status: vec![
            "App bundle manifest is versioned and validated.".to_string(),
            "WGSL shader assets are discovered from the committed shader directory.".to_string(),
            "FVR08 production voxel route is the default environment entry and loads real saved config/assets."
                .to_string(),
            "Alpha art v1 PNG sprites/tiles remain manifest-validated as historical regression assets."
                .to_string(),
            "True 2.5D glTF assets remain manifest-validated as historical reference assets, not the FVR production default."
                .to_string(),
            "FVR07 production voxel assets are manifest-validated with license, digest, source, and VFX budget metadata."
                .to_string(),
        ],
    })
}

fn validate_bundle_entry_kind(
    entry: &AppBundleEntry,
    path: &Path,
) -> Result<(), GameAppShellError> {
    match entry.kind.as_str() {
        "runtime-config" => {
            RuntimeConfig::from_json_file(path)?.validate()?;
        }
        "asset-manifest" => {
            let manifest = AssetManifest::from_json_file(path)?;
            let root = path
                .parent()
                .ok_or(ScaffoldContractError::MissingPhaseData)?;
            manifest.validate_with_root(root)?;
        }
        "portable-save" => {
            let save = PortableSaveFile::from_json_file(path)?;
            let root = path
                .parent()
                .ok_or(ScaffoldContractError::MissingPhaseData)?;
            save.validate_with_asset_root(root)?;
        }
        _ => return Err(ScaffoldContractError::MissingPhaseData.into()),
    }
    Ok(())
}

fn validate_placeholder_art_manifest(
    path: &Path,
) -> Result<PlaceholderArtManifest, GameAppShellError> {
    let manifest: PlaceholderArtManifest = read_json(path)?;
    require_schema(
        &manifest.schema,
        manifest.schema_version,
        CA12_PLACEHOLDER_ART_MANIFEST_SCHEMA,
        CA12_PLACEHOLDER_ART_MANIFEST_SCHEMA_VERSION,
    )?;
    require_id(&manifest.manifest_id)?;
    if manifest.entries.len() < 4 || manifest.entries.len() > CA12_MAX_BUNDLE_ENTRIES {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let mut ids = BTreeSet::new();
    let mut kinds = BTreeSet::new();
    for entry in &manifest.entries {
        require_id(&entry.id)?;
        require_id(&entry.kind)?;
        require_id(&entry.shape)?;
        require_id(&entry.color)?;
        require_id(&entry.description)?;
        if !ids.insert(entry.id.as_str()) {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        kinds.insert(entry.kind.as_str());
    }
    for required in ["creature", "food", "hazard", "obstacle"] {
        if !kinds.contains(required) {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
    }
    Ok(manifest)
}

fn validate_entry(
    entry: &AppBundleEntry,
    ids: &mut BTreeSet<String>,
) -> Result<(), GameAppShellError> {
    require_id(&entry.id)?;
    require_id(&entry.kind)?;
    validate_relative_path(&entry.relative_path)?;
    if !ids.insert(entry.id.clone()) {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn validate_shader_entry(
    entry: &ShaderAssetEntry,
    ids: &mut BTreeSet<String>,
) -> Result<(), GameAppShellError> {
    require_id(&entry.id)?;
    validate_relative_path(&entry.relative_path)?;
    if !entry.relative_path.ends_with(".wgsl") || !ids.insert(entry.id.clone()) {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn discover_workspace_shaders(root: &Path) -> Result<Vec<PathBuf>, GameAppShellError> {
    let shader_root = root.join("crates/alife_gpu_backend/shaders");
    let mut shaders = Vec::new();
    for entry in std::fs::read_dir(shader_root)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("wgsl") {
            shaders.push(path);
        }
    }
    shaders.sort();
    Ok(shaders)
}

fn resolve_workspace_path(root: &Path, relative: &str) -> Result<PathBuf, GameAppShellError> {
    validate_relative_path(relative)?;
    Ok(root.join(relative))
}

fn validate_relative_path(relative: &str) -> Result<(), GameAppShellError> {
    if relative.trim().is_empty() {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn require_schema(
    actual_schema: &str,
    actual_version: u16,
    expected_schema: &str,
    expected_version: u16,
) -> Result<(), GameAppShellError> {
    if actual_schema != expected_schema || actual_version != expected_version {
        Err(ScaffoldContractError::MissingPhaseData.into())
    } else {
        Ok(())
    }
}

fn require_id(value: &str) -> Result<(), GameAppShellError> {
    if value.trim().is_empty()
        || value.contains("..")
        || value.contains("Entity(")
        || value.contains("Bevy")
        || value.contains("wgpu::")
    {
        Err(ScaffoldContractError::MissingPhaseData.into())
    } else {
        Ok(())
    }
}

fn tiny_file_size(path: &Path) -> Result<u64, GameAppShellError> {
    let bytes = std::fs::metadata(path)?.len();
    if bytes > CA12_MAX_BUNDLE_FILE_BYTES {
        Err(ScaffoldContractError::MissingPhaseData.into())
    } else {
        Ok(bytes)
    }
}

fn has_binary_like_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase()),
        Some(ext)
            if matches!(
                ext.as_str(),
                "png" | "jpg" | "jpeg" | "dds" | "ktx" | "bin" | "mp4" | "wav"
            )
    )
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, GameAppShellError> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}
