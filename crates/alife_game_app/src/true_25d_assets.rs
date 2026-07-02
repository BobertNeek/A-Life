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
pub const TRUE_25D_ENDOCRINE_FEEDBACK_SCHEMA: &str =
    "alife.ca44a.true25d_endocrine_asset_feedback.v1";
pub const TRUE_25D_ENDOCRINE_FEEDBACK_SCHEMA_VERSION: u16 = 1;
pub const TRUE_25D_ENDOCRINE_FEEDBACK_EXTRAS_KEY: &str = "alife_true25d_endocrine_feedback";

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

pub const TRUE_25D_ENDOCRINE_FEEDBACK_ROLES: [&str; 2] = ["creature-idle", "creature-hurt"];

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
    #[serde(default)]
    pub endocrine_feedback: Option<True25dEndocrineFeedbackContract>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct True25dEndocrineFeedbackContract {
    pub schema: String,
    pub schema_version: u16,
    pub display_only: bool,
    pub flat_endocrine_tensor_driven: bool,
    pub source: String,
    pub posture_channels: Vec<String>,
    pub animation_speed_channels: Vec<String>,
    pub material_channels: Vec<String>,
    pub particle_channels: Vec<String>,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
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
    pub endocrine_feedback_assets: usize,
    pub endocrine_feedback_contract_validated: bool,
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
            || !self.endocrine_feedback_contract_validated
            || !self.no_action_authority
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:entries={}:roles={}:gltf={}:camera={}:shader={}:endocrine_assets={}:endocrine_contract={}:largest={}:authority={}",
            self.schema,
            self.schema_version,
            self.pack_id,
            self.entry_count,
            self.required_roles_present,
            self.gltf_files_validated,
            self.orthographic_camera_locked,
            self.shader_stack_declared,
            self.endocrine_feedback_assets,
            self.endocrine_feedback_contract_validated,
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
        && manifest.shader_stack.low_resolution_pixel_step_filter
        && !manifest.shader_stack.runtime_shader_contract_only;

    let mut roles = BTreeMap::new();
    let mut paths = BTreeSet::new();
    let mut largest_file_bytes = 0_u64;
    let mut total_file_bytes = 0_u64;
    let mut endocrine_feedback_assets = 0_usize;
    for entry in &manifest.entries {
        validate_true_25d_asset_entry(entry, &mut paths)?;
        let path = root.join(&entry.relative_path);
        let file_size = validate_gltf_asset_file(&path, entry)?;
        if file_size != entry.file_size_bytes {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        largest_file_bytes = largest_file_bytes.max(file_size);
        total_file_bytes = total_file_bytes.saturating_add(file_size);
        if true_25d_role_requires_endocrine_feedback(&entry.role) {
            endocrine_feedback_assets += 1;
        }
        *roles.entry(entry.role.as_str()).or_insert(0_usize) += 1;
    }

    let required_roles_present = TRUE_25D_REQUIRED_ROLES
        .iter()
        .all(|role| roles.contains_key(role));
    let endocrine_feedback_contract_validated =
        endocrine_feedback_assets == TRUE_25D_ENDOCRINE_FEEDBACK_ROLES.len();
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
        endocrine_feedback_assets,
        endocrine_feedback_contract_validated,
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

fn validate_gltf_asset_file(
    path: &Path,
    entry: &True25dAssetEntry,
) -> Result<u64, GameAppShellError> {
    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();
    if file_size == 0 || file_size > TRUE_25D_ALPHA_MAX_ASSET_BYTES {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let json = match path.extension().and_then(|value| value.to_str()) {
        Some("gltf") => {
            let text = std::fs::read_to_string(path)?;
            serde_json::from_str(&text)?
        }
        Some("glb") => {
            let bytes = std::fs::read(path)?;
            glb_json_chunk(&bytes)?
        }
        _ => return Err(ScaffoldContractError::MissingPhaseData.into()),
    };
    if json.get("asset").and_then(|asset| asset.get("version"))
        != Some(&serde_json::Value::String("2.0".to_string()))
        || json.get("meshes").and_then(|v| v.as_array()).is_none()
        || json.get("materials").and_then(|v| v.as_array()).is_none()
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    validate_true_25d_gltf_endocrine_feedback_contract(entry, &json)?;
    Ok(file_size)
}

fn glb_json_chunk(bytes: &[u8]) -> Result<serde_json::Value, GameAppShellError> {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let total_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    if version != 2 || total_len != bytes.len() {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let mut offset = 12_usize;
    while offset + 8 <= bytes.len() {
        let chunk_len = u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ]) as usize;
        let chunk_type = u32::from_le_bytes([
            bytes[offset + 4],
            bytes[offset + 5],
            bytes[offset + 6],
            bytes[offset + 7],
        ]);
        offset += 8;
        if offset + chunk_len > bytes.len() {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        if chunk_type == 0x4E4F534A {
            let json_bytes = &bytes[offset..offset + chunk_len];
            let text = std::str::from_utf8(json_bytes)
                .map_err(|_| ScaffoldContractError::MissingPhaseData)?
                .trim_end_matches([' ', '\0', '\n', '\r', '\t']);
            return Ok(serde_json::from_str(text)?);
        }
        offset += chunk_len;
    }
    Err(ScaffoldContractError::MissingPhaseData.into())
}

fn true_25d_role_requires_endocrine_feedback(role: &str) -> bool {
    TRUE_25D_ENDOCRINE_FEEDBACK_ROLES.contains(&role)
}

fn validate_true_25d_gltf_endocrine_feedback_contract(
    entry: &True25dAssetEntry,
    gltf: &serde_json::Value,
) -> Result<(), GameAppShellError> {
    let manifest_contract = if true_25d_role_requires_endocrine_feedback(&entry.role) {
        entry
            .endocrine_feedback
            .as_ref()
            .ok_or(ScaffoldContractError::MissingPhaseData)?
    } else if let Some(contract) = &entry.endocrine_feedback {
        validate_endocrine_feedback_contract(contract)?;
        return Ok(());
    } else {
        return Ok(());
    };
    validate_endocrine_feedback_contract(manifest_contract)?;
    let gltf_contract_value = gltf
        .get("asset")
        .and_then(|asset| asset.get("extras"))
        .and_then(|extras| extras.get(TRUE_25D_ENDOCRINE_FEEDBACK_EXTRAS_KEY))
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let gltf_contract: True25dEndocrineFeedbackContract =
        serde_json::from_value(gltf_contract_value.clone())?;
    validate_endocrine_feedback_contract(&gltf_contract)?;
    if &gltf_contract != manifest_contract {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn validate_endocrine_feedback_contract(
    contract: &True25dEndocrineFeedbackContract,
) -> Result<(), GameAppShellError> {
    if contract.schema != TRUE_25D_ENDOCRINE_FEEDBACK_SCHEMA
        || contract.schema_version != TRUE_25D_ENDOCRINE_FEEDBACK_SCHEMA_VERSION
        || !contract.display_only
        || !contract.flat_endocrine_tensor_driven
        || contract.source != "alife_core.EndocrineSnapshot::to_array plus bounded drive companions"
        || !contract.no_action_authority
        || !contract.no_weight_authority
        || !contains_channel(&contract.posture_channels, "adrenaline")
        || !contains_channel(&contract.posture_channels, "pain_drive_companion")
        || !contains_channel(&contract.animation_speed_channels, "adrenaline")
        || !contains_channel(&contract.animation_speed_channels, "pain_drive_companion")
        || !contains_channel(&contract.material_channels, "cortisol")
        || !contains_channel(&contract.material_channels, "dopamine")
        || !contains_channel(&contract.material_channels, "low_hunger_drive_companion")
        || !contains_channel(&contract.material_channels, "learning_companion")
        || !contains_channel(&contract.particle_channels, "dopamine")
        || !contains_channel(&contract.particle_channels, "learning_companion")
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn contains_channel(channels: &[String], expected: &str) -> bool {
    channels.iter().any(|channel| channel == expected)
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
        assert_eq!(
            summary.endocrine_feedback_assets,
            TRUE_25D_ENDOCRINE_FEEDBACK_ROLES.len()
        );
        assert!(summary.endocrine_feedback_contract_validated);
        assert!(summary.no_action_authority);
    }

    #[test]
    fn true_25d_manifest_rejects_contract_only_shader_stack() {
        let root = ca12_workspace_root();
        let path = default_true_25d_asset_manifest_path();
        let mut manifest: True25dAssetManifest =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        manifest.shader_stack.runtime_shader_contract_only = true;
        let summary = validate_true_25d_asset_manifest_inner(&root, &path, &manifest).unwrap();
        assert!(!summary.shader_stack_declared);
        assert!(summary.validate().is_err());
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

    #[test]
    fn true_25d_manifest_rejects_missing_or_malformed_endocrine_asset_contract() {
        let root = ca12_workspace_root();
        let path = default_true_25d_asset_manifest_path();
        let manifest: True25dAssetManifest =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

        let mut missing = manifest.clone();
        missing.entries[0].endocrine_feedback = None;
        assert!(validate_true_25d_asset_manifest_inner(&root, &path, &missing).is_err());

        let mut wrong_authority = manifest.clone();
        wrong_authority.entries[0]
            .endocrine_feedback
            .as_mut()
            .unwrap()
            .no_action_authority = false;
        assert!(validate_true_25d_asset_manifest_inner(&root, &path, &wrong_authority).is_err());

        let mut missing_channel = manifest.clone();
        missing_channel.entries[0]
            .endocrine_feedback
            .as_mut()
            .unwrap()
            .material_channels
            .retain(|channel| channel != "cortisol");
        assert!(validate_true_25d_asset_manifest_inner(&root, &path, &missing_channel).is_err());

        let mut manifest_gltf_mismatch = manifest;
        manifest_gltf_mismatch.entries[0]
            .endocrine_feedback
            .as_mut()
            .unwrap()
            .particle_channels
            .push("diagnostic-extra-channel".to_string());
        assert!(
            validate_true_25d_asset_manifest_inner(&root, &path, &manifest_gltf_mismatch).is_err()
        );
    }
}
