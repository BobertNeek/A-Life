//! True 2.5D alpha asset manifest validation.
//!
//! This module validates the player-facing low-poly glTF asset lane. The
//! assets are presentation-only: they carry no action authority, no hidden
//! cognition state, and no portable engine IDs.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

use crate::prelude::*;
use crate::*;

pub const TRUE_25D_ALPHA_ASSET_MANIFEST_RELATIVE_PATH: &str =
    "crates/alife_game_app/assets/true_25d_alpha_v1/true_25d_manifest.json";
pub const TRUE_25D_ALPHA_ART_DIRECTION: &str = "true-2-5d-retro-futuristic-biological-v1";

pub const TRUE_25D_REQUIRED_ROLES: [&str; 15] = [
    "creature-idle",
    "creature-hurt",
    "selection-ring",
    "food",
    "hazard",
    "rock-obstacle",
    "plant-prop",
    "terrain-grass-island",
    "terrain-soil-island",
    "terrain-resource-grove",
    "terrain-hazard-pressure",
    "terrain-stone-island",
    "terrain-water-cell",
    "terrain-sand-island",
    "fog-of-war-cell",
];

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct True25dAssetManifest {
    pub schema: String,
    pub schema_version: u16,
    pub pack_id: String,
    pub art_direction: String,
    pub camera: True25dCameraContract,
    pub shader_stack: True25dShaderStackContract,
    pub entries: Vec<True25dAssetEntry>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct True25dCameraContract {
    pub projection: String,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub rotation_locked: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct True25dShaderStackContract {
    pub quantized_toon_bands: u8,
    pub sobel_outline_contract: bool,
    pub low_resolution_pixel_step_filter: bool,
    pub runtime_shader_contract_only: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct True25dAssetEntry {
    pub role: String,
    pub relative_path: String,
    pub node_count: u32,
    pub mesh_count: u32,
    pub material_count: u32,
    pub vertex_count: u32,
    pub index_count: u32,
    pub file_size_bytes: u64,
    pub blender_normalized: bool,
    pub origin_anchor: String,
    pub transform_applied: bool,
    pub max_dimension_units: f32,
    pub decimation_threshold_triangles: u32,
    pub triangle_count: u32,
    pub decimation_applied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct True25dAssetValidationSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub pack_id: String,
    pub manifest_path: PathBuf,
    pub entry_count: usize,
    pub required_roles_present: bool,
    pub gltf_files_validated: bool,
    pub orthographic_camera_locked: bool,
    pub shader_stack_declared: bool,
    pub largest_file_bytes: u64,
    pub total_file_bytes: u64,
    pub no_action_authority: bool,
}

impl True25dAssetValidationSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != TRUE_25D_ALPHA_ASSET_MANIFEST_SCHEMA
            || self.schema_version != TRUE_25D_ALPHA_ASSET_MANIFEST_SCHEMA_VERSION
            || self.entry_count < TRUE_25D_ALPHA_MIN_REQUIRED_ROLES
            || !self.required_roles_present
            || !self.gltf_files_validated
            || !self.orthographic_camera_locked
            || !self.shader_stack_declared
            || self.largest_file_bytes > TRUE_25D_ALPHA_MAX_ASSET_BYTES
            || self.total_file_bytes == 0
            || !self.no_action_authority
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:entries={}:roles={}:gltf={}:camera={}:shader={}:largest={}:authority={}",
            self.schema,
            self.schema_version,
            self.pack_id,
            self.entry_count,
            self.required_roles_present,
            self.gltf_files_validated,
            self.orthographic_camera_locked,
            self.shader_stack_declared,
            self.largest_file_bytes,
            self.no_action_authority
        )
    }
}

pub fn default_true_25d_asset_manifest_path() -> PathBuf {
    ca12_workspace_root().join(TRUE_25D_ALPHA_ASSET_MANIFEST_RELATIVE_PATH)
}

pub fn validate_true_25d_asset_manifest(
    manifest_path: impl AsRef<Path>,
) -> Result<True25dAssetValidationSummary, GameAppShellError> {
    let manifest_path = manifest_path.as_ref();
    let manifest: True25dAssetManifest =
        serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
    let summary =
        validate_true_25d_asset_manifest_inner(&ca12_workspace_root(), manifest_path, &manifest)?;
    summary.validate()?;
    Ok(summary)
}

pub(crate) fn validate_true_25d_asset_manifest_inner(
    root: &Path,
    manifest_path: &Path,
    manifest: &True25dAssetManifest,
) -> Result<True25dAssetValidationSummary, GameAppShellError> {
    if manifest.schema != TRUE_25D_ALPHA_ASSET_MANIFEST_SCHEMA
        || manifest.schema_version != TRUE_25D_ALPHA_ASSET_MANIFEST_SCHEMA_VERSION
        || manifest.pack_id.trim().is_empty()
        || manifest.art_direction != TRUE_25D_ALPHA_ART_DIRECTION
        || manifest.entries.len() < TRUE_25D_ALPHA_MIN_REQUIRED_ROLES
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }

    let orthographic_camera_locked = manifest.camera.projection == "orthographic"
        && manifest.camera.rotation_locked
        && manifest.camera.yaw_degrees.abs() <= 0.01
        && (manifest.camera.pitch_degrees + 45.0).abs() <= 0.01;
    let shader_stack_declared = manifest.shader_stack.quantized_toon_bands >= 3
        && manifest.shader_stack.sobel_outline_contract
        && manifest.shader_stack.low_resolution_pixel_step_filter;

    let mut roles = BTreeMap::new();
    let mut paths = BTreeSet::new();
    let mut largest_file_bytes = 0_u64;
    let mut total_file_bytes = 0_u64;
    for entry in &manifest.entries {
        validate_true_25d_asset_entry(entry, &mut paths)?;
        let path = root.join(&entry.relative_path);
        let file_size = validate_gltf_asset_file(&path)?;
        if file_size != entry.file_size_bytes {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        largest_file_bytes = largest_file_bytes.max(file_size);
        total_file_bytes = total_file_bytes.saturating_add(file_size);
        *roles.entry(entry.role.as_str()).or_insert(0_usize) += 1;
    }

    let required_roles_present = TRUE_25D_REQUIRED_ROLES
        .iter()
        .all(|role| roles.contains_key(role));
    Ok(True25dAssetValidationSummary {
        schema: TRUE_25D_ALPHA_ASSET_MANIFEST_SCHEMA,
        schema_version: TRUE_25D_ALPHA_ASSET_MANIFEST_SCHEMA_VERSION,
        pack_id: manifest.pack_id.clone(),
        manifest_path: manifest_path.to_path_buf(),
        entry_count: manifest.entries.len(),
        required_roles_present,
        gltf_files_validated: true,
        orthographic_camera_locked,
        shader_stack_declared,
        largest_file_bytes,
        total_file_bytes,
        no_action_authority: true,
    })
}

fn validate_true_25d_asset_entry(
    entry: &True25dAssetEntry,
    paths: &mut BTreeSet<String>,
) -> Result<(), GameAppShellError> {
    require_true_25d_id(&entry.role)?;
    validate_true_25d_relative_path(&entry.relative_path)?;
    if !paths.insert(entry.relative_path.clone())
        || entry.node_count == 0
        || entry.mesh_count == 0
        || entry.material_count == 0
        || entry.vertex_count < 4
        || entry.index_count < 6
        || entry.file_size_bytes == 0
        || entry.file_size_bytes > TRUE_25D_ALPHA_MAX_ASSET_BYTES
        || !entry.blender_normalized
        || entry.origin_anchor != "base-center"
        || !entry.transform_applied
        || !entry.max_dimension_units.is_finite()
        || entry.max_dimension_units <= 0.0
        || entry.max_dimension_units > 1.001
        || entry.decimation_threshold_triangles == 0
        || entry.triangle_count == 0
        || entry.triangle_count > entry.decimation_threshold_triangles
        || entry.index_count != entry.triangle_count.saturating_mul(3)
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    if !entry.relative_path.ends_with(".gltf") && !entry.relative_path.ends_with(".glb") {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn validate_true_25d_relative_path(relative: &str) -> Result<(), GameAppShellError> {
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
                    "target"
                        | "artifacts"
                        | "logs"
                        | "captures"
                        | "screenshots"
                        | "cache"
                        | ".cache"
                        | "models"
                ) {
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

fn validate_gltf_asset_file(path: &Path) -> Result<u64, GameAppShellError> {
    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();
    if file_size == 0 || file_size > TRUE_25D_ALPHA_MAX_ASSET_BYTES {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    match path.extension().and_then(|value| value.to_str()) {
        Some("gltf") => {
            let text = std::fs::read_to_string(path)?;
            let value: serde_json::Value = serde_json::from_str(&text)?;
            if value.get("asset").and_then(|asset| asset.get("version"))
                != Some(&serde_json::Value::String("2.0".to_string()))
                || value.get("meshes").and_then(|v| v.as_array()).is_none()
                || value.get("materials").and_then(|v| v.as_array()).is_none()
            {
                return Err(ScaffoldContractError::MissingPhaseData.into());
            }
        }
        Some("glb") => {
            let bytes = std::fs::read(path)?;
            if bytes.len() < 12 || &bytes[0..4] != b"glTF" {
                return Err(ScaffoldContractError::MissingPhaseData.into());
            }
        }
        _ => return Err(ScaffoldContractError::MissingPhaseData.into()),
    }
    Ok(file_size)
}

fn require_true_25d_id(value: &str) -> Result<(), GameAppShellError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn true_25d_committed_manifest_validates_gltf_assets() {
        let summary =
            validate_true_25d_asset_manifest(default_true_25d_asset_manifest_path()).unwrap();
        assert!(summary.required_roles_present);
        assert!(summary.gltf_files_validated);
        assert!(summary.orthographic_camera_locked);
        assert!(summary.shader_stack_declared);
        assert!(summary.no_action_authority);
    }

    #[test]
    fn true_25d_manifest_rejects_missing_required_role_and_remote_paths() {
        let root = ca12_workspace_root();
        let path = default_true_25d_asset_manifest_path();
        let mut manifest: True25dAssetManifest =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let hazard = manifest
            .entries
            .iter_mut()
            .find(|entry| entry.role == "hazard")
            .unwrap();
        hazard.role = "hazard-renamed-out-of-required-set".to_string();
        let summary = validate_true_25d_asset_manifest_inner(&root, &path, &manifest).unwrap();
        assert!(!summary.required_roles_present);
        assert!(summary.validate().is_err());

        let mut manifest: True25dAssetManifest =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        manifest.entries[0].relative_path = "target/artifacts/fake.gltf".to_string();
        assert!(validate_true_25d_asset_manifest_inner(&root, &path, &manifest).is_err());
    }

    #[test]
    fn true_25d_manifest_rejects_assets_without_blender_normalization_contract() {
        let root = ca12_workspace_root();
        let path = default_true_25d_asset_manifest_path();
        let manifest: True25dAssetManifest =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        let mut missing_normalization = manifest.clone();
        missing_normalization.entries[0].blender_normalized = false;
        assert!(
            validate_true_25d_asset_manifest_inner(&root, &path, &missing_normalization).is_err()
        );

        let mut wrong_origin = manifest.clone();
        wrong_origin.entries[0].origin_anchor = "scene-center".to_string();
        assert!(validate_true_25d_asset_manifest_inner(&root, &path, &wrong_origin).is_err());

        let mut oversized = manifest.clone();
        oversized.entries[0].max_dimension_units = 1.25;
        assert!(validate_true_25d_asset_manifest_inner(&root, &path, &oversized).is_err());

        let mut stale_metrics = manifest;
        stale_metrics.entries[0].triangle_count =
            stale_metrics.entries[0].decimation_threshold_triangles + 1;
        assert!(validate_true_25d_asset_manifest_inner(&root, &path, &stale_metrics).is_err());
    }
}
