//! CA44A versioned alpha art manifest validation.
//!
//! The assets validated here are small committed product sprites/tiles. They
//! are distinct from target artifacts, screenshots, generated captures, and
//! debug placeholder metadata.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

use crate::prelude::*;
use crate::*;

pub const CA44A_ALPHA_ART_MANIFEST_RELATIVE_PATH: &str =
    "crates/alife_game_app/assets/alpha_art_v1/alpha_art_manifest.json";

pub const CA44A_REQUIRED_ALPHA_ART_ROLE_NAMES: [&str; CA44A_REQUIRED_ALPHA_ART_ROLES] = [
    "creature-idle",
    "creature-hurt",
    "creature-moving",
    "creature-eat",
    "creature-sleep",
    "creature-signal",
    "selection-ring",
    "selection-pulse",
    "food",
    "food-variant",
    "hazard",
    "hazard-active",
    "ambient-canopy-shadow",
    "ambient-light-pool",
    "entity-shadow",
    "rock-obstacle",
    "terrain-safe-grass",
    "terrain-soil-path",
    "terrain-resource-grove",
    "terrain-hazard-pressure",
    "terrain-stone-rough",
    "terrain-edge-blend",
    "world-backdrop",
    "prop-dressing",
    "ui-panel-frame",
    "ui-inspector-frame",
    "ui-status-chip",
    "ui-meter-bar",
    "ui-control-keycap",
];

pub const CA44A_ALPHA_ART_DIRECTION: &str = "production-alpha-generated-map-v15";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AlphaArtManifest {
    pub schema: String,
    pub schema_version: u16,
    pub pack_id: String,
    pub art_direction: String,
    pub entries: Vec<AlphaArtEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AlphaArtEntry {
    pub id: String,
    pub role: String,
    pub kind: String,
    pub relative_path: String,
    pub width: u32,
    pub height: u32,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlphaArtValidationSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub pack_id: String,
    pub manifest_path: PathBuf,
    pub entry_count: usize,
    pub required_roles_present: bool,
    pub prop_variant_count: usize,
    pub largest_file_bytes: u64,
    pub total_file_bytes: u64,
    pub png_dimensions_validated: bool,
    pub forbidden_artifact_paths_rejected: bool,
}

impl AlphaArtValidationSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA44A_ALPHA_ART_MANIFEST_SCHEMA
            || self.schema_version != CA44A_ALPHA_ART_MANIFEST_SCHEMA_VERSION
            || self.pack_id.trim().is_empty()
            || self.entry_count < CA44A_REQUIRED_ALPHA_ART_ROLES + 2
            || !self.required_roles_present
            || self.prop_variant_count < 3
            || self.largest_file_bytes > CA44A_MAX_ALPHA_ART_BACKDROP_BYTES
            || self.total_file_bytes == 0
            || !self.png_dimensions_validated
            || !self.forbidden_artifact_paths_rejected
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:entries={}:props={}:largest={}:total={}:roles={}:png={}",
            self.schema,
            self.schema_version,
            self.pack_id,
            self.entry_count,
            self.prop_variant_count,
            self.largest_file_bytes,
            self.total_file_bytes,
            self.required_roles_present,
            self.png_dimensions_validated
        )
    }
}

pub fn default_alpha_art_manifest_path() -> PathBuf {
    ca12_workspace_root().join(CA44A_ALPHA_ART_MANIFEST_RELATIVE_PATH)
}

pub fn validate_alpha_art_manifest(
    manifest_path: impl AsRef<Path>,
) -> Result<AlphaArtValidationSummary, GameAppShellError> {
    let manifest_path = manifest_path.as_ref();
    let manifest: AlphaArtManifest =
        serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
    let root = ca12_workspace_root();
    let summary = validate_alpha_art_manifest_inner(&root, manifest_path, &manifest, true)?;

    let mut broken = manifest.clone();
    if let Some(entry) = broken.entries.first_mut() {
        entry.relative_path = "target/artifacts/forbidden.png".to_string();
    }
    let forbidden_artifact_paths_rejected =
        validate_alpha_art_manifest_inner(&root, manifest_path, &broken, false).is_err();

    let summary = AlphaArtValidationSummary {
        forbidden_artifact_paths_rejected,
        ..summary
    };
    summary.validate()?;
    Ok(summary)
}

pub(crate) fn validate_alpha_art_manifest_inner(
    root: &Path,
    manifest_path: &Path,
    manifest: &AlphaArtManifest,
    check_files: bool,
) -> Result<AlphaArtValidationSummary, GameAppShellError> {
    if manifest.schema != CA44A_ALPHA_ART_MANIFEST_SCHEMA
        || manifest.schema_version != CA44A_ALPHA_ART_MANIFEST_SCHEMA_VERSION
        || manifest.pack_id.trim().is_empty()
        || manifest.art_direction != CA44A_ALPHA_ART_DIRECTION
        || manifest.entries.is_empty()
        || manifest.entries.len() > CA44A_MAX_ALPHA_ART_ENTRIES
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }

    let mut ids = BTreeSet::new();
    let mut roles: BTreeMap<&str, usize> = BTreeMap::new();
    let mut largest_file_bytes = 0_u64;
    let mut total_file_bytes = 0_u64;
    let mut png_dimensions_validated = true;
    for entry in &manifest.entries {
        validate_alpha_art_entry(entry, &mut ids)?;
        *roles.entry(entry.role.as_str()).or_insert(0) += 1;
        let path = root.join(&entry.relative_path);
        if check_files {
            let (width, height, file_size) = validate_png_asset_file(&path)?;
            if width != entry.width || height != entry.height || file_size != entry.file_size_bytes
            {
                return Err(ScaffoldContractError::MissingPhaseData.into());
            }
            largest_file_bytes = largest_file_bytes.max(file_size);
            total_file_bytes = total_file_bytes.saturating_add(file_size);
            png_dimensions_validated &= width > 0 && height > 0;
        } else {
            largest_file_bytes = largest_file_bytes.max(entry.file_size_bytes);
            total_file_bytes = total_file_bytes.saturating_add(entry.file_size_bytes);
            png_dimensions_validated &= entry.width > 0 && entry.height > 0;
        }
    }
    let required_roles_present = CA44A_REQUIRED_ALPHA_ART_ROLE_NAMES
        .iter()
        .all(|role| roles.contains_key(role));
    let prop_variant_count = roles.get("prop-dressing").copied().unwrap_or(0);

    Ok(AlphaArtValidationSummary {
        schema: CA44A_ALPHA_ART_MANIFEST_SCHEMA,
        schema_version: CA44A_ALPHA_ART_MANIFEST_SCHEMA_VERSION,
        pack_id: manifest.pack_id.clone(),
        manifest_path: manifest_path.to_path_buf(),
        entry_count: manifest.entries.len(),
        required_roles_present,
        prop_variant_count,
        largest_file_bytes,
        total_file_bytes,
        png_dimensions_validated,
        forbidden_artifact_paths_rejected: false,
    })
}

fn validate_alpha_art_entry(
    entry: &AlphaArtEntry,
    ids: &mut BTreeSet<String>,
) -> Result<(), GameAppShellError> {
    require_alpha_id(&entry.id)?;
    require_alpha_id(&entry.role)?;
    require_alpha_id(&entry.kind)?;
    validate_alpha_art_relative_path(&entry.relative_path)?;
    if !ids.insert(entry.id.clone())
        || entry.width < CA44A_MIN_PRODUCTION_ART_DIMENSION
        || entry.height < CA44A_MIN_PRODUCTION_ART_DIMENSION
        || entry.file_size_bytes == 0
        || entry.file_size_bytes > ca44a_max_bytes_for_role(&entry.role)
        || entry.width > ca44a_max_dimension_for_role(&entry.role)
        || entry.height > ca44a_max_dimension_for_role(&entry.role)
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    if !entry.relative_path.ends_with(".png") {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

fn validate_alpha_art_relative_path(relative: &str) -> Result<(), GameAppShellError> {
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

pub fn validate_png_asset_file(path: &Path) -> Result<(u32, u32, u64), GameAppShellError> {
    let bytes = std::fs::read(path)?;
    let file_size = bytes.len() as u64;
    if file_size == 0 || file_size > CA44A_MAX_ALPHA_ART_BACKDROP_BYTES {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if bytes.len() < 24 || bytes[0..8] != PNG_SIGNATURE {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if width < CA44A_MIN_PRODUCTION_ART_DIMENSION
        || height < CA44A_MIN_PRODUCTION_ART_DIMENSION
        || width > CA44A_MAX_PRODUCTION_BACKDROP_DIMENSION
        || height > CA44A_MAX_PRODUCTION_BACKDROP_DIMENSION
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok((width, height, file_size))
}

fn ca44a_max_bytes_for_role(role: &str) -> u64 {
    if role == "world-backdrop" {
        CA44A_MAX_ALPHA_ART_BACKDROP_BYTES
    } else {
        CA44A_MAX_ALPHA_ART_ASSET_BYTES
    }
}

fn ca44a_max_dimension_for_role(role: &str) -> u32 {
    if role == "world-backdrop" {
        CA44A_MAX_PRODUCTION_BACKDROP_DIMENSION
    } else {
        CA44A_MAX_PRODUCTION_ART_DIMENSION
    }
}

fn require_alpha_id(value: &str) -> Result<(), GameAppShellError> {
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

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "alife_ca44a_alpha_art_{}_{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("alpha_art")).unwrap();
        root
    }

    fn valid_png_bytes() -> &'static [u8] {
        include_bytes!("../assets/alpha_art_v1/creature_idle.png")
    }

    fn required_manifest(root: &Path) -> AlphaArtManifest {
        let path = root.join("alpha_art/sprite.png");
        std::fs::write(&path, valid_png_bytes()).unwrap();
        let (width, height, file_size_bytes) = validate_png_asset_file(&path).unwrap();
        let mut entries = Vec::new();
        for (index, role) in CA44A_REQUIRED_ALPHA_ART_ROLE_NAMES.iter().enumerate() {
            entries.push(AlphaArtEntry {
                id: format!("{role}-{index}"),
                role: (*role).to_string(),
                kind: if *role == "prop-dressing" {
                    "prop".to_string()
                } else {
                    "sprite".to_string()
                },
                relative_path: "alpha_art/sprite.png".to_string(),
                width,
                height,
                file_size_bytes,
            });
        }
        for index in 0..2 {
            entries.push(AlphaArtEntry {
                id: format!("prop-extra-{index}"),
                role: "prop-dressing".to_string(),
                kind: "prop".to_string(),
                relative_path: "alpha_art/sprite.png".to_string(),
                width,
                height,
                file_size_bytes,
            });
        }
        AlphaArtManifest {
            schema: CA44A_ALPHA_ART_MANIFEST_SCHEMA.to_string(),
            schema_version: CA44A_ALPHA_ART_MANIFEST_SCHEMA_VERSION,
            pack_id: "unit-alpha-art".to_string(),
            art_direction: CA44A_ALPHA_ART_DIRECTION.to_string(),
            entries,
        }
    }

    #[test]
    fn alpha_art_inner_validator_accepts_complete_png_manifest() {
        let root = temp_root("valid");
        let manifest = required_manifest(&root);
        let summary =
            validate_alpha_art_manifest_inner(&root, &root.join("manifest.json"), &manifest, true)
                .unwrap();
        assert!(summary.required_roles_present);
        assert_eq!(summary.prop_variant_count, 3);
        assert!(summary.largest_file_bytes <= CA44A_MAX_ALPHA_ART_BACKDROP_BYTES);
        assert!(summary.png_dimensions_validated);
    }

    #[test]
    fn alpha_art_inner_validator_rejects_non_backdrop_asset_over_sprite_cap() {
        let root = temp_root("oversize_non_backdrop");
        let mut manifest = required_manifest(&root);
        manifest.entries[0].file_size_bytes = CA44A_MAX_ALPHA_ART_ASSET_BYTES + 1;
        assert!(validate_alpha_art_manifest_inner(
            &root,
            &root.join("manifest.json"),
            &manifest,
            true,
        )
        .is_err());
    }

    #[test]
    fn alpha_art_inner_validator_rejects_missing_required_role() {
        let root = temp_root("missing_role");
        let mut manifest = required_manifest(&root);
        manifest.entries.retain(|entry| entry.role != "hazard");
        let summary =
            validate_alpha_art_manifest_inner(&root, &root.join("manifest.json"), &manifest, true)
                .unwrap();
        assert!(!summary.required_roles_present);
        assert!(summary.validate().is_err());
    }

    #[test]
    fn alpha_art_inner_validator_rejects_dimension_mismatch() {
        let root = temp_root("dimension");
        let mut manifest = required_manifest(&root);
        manifest.entries[0].width = 32;
        assert!(validate_alpha_art_manifest_inner(
            &root,
            &root.join("manifest.json"),
            &manifest,
            true,
        )
        .is_err());
    }

    #[test]
    fn alpha_art_inner_validator_rejects_malformed_png() {
        let root = temp_root("malformed");
        let path = root.join("alpha_art/bad.png");
        std::fs::write(&path, b"not a png").unwrap();
        let mut manifest = required_manifest(&root);
        manifest.entries[0].relative_path = "alpha_art/bad.png".to_string();
        manifest.entries[0].file_size_bytes = std::fs::metadata(&path).unwrap().len();
        assert!(validate_alpha_art_manifest_inner(
            &root,
            &root.join("manifest.json"),
            &manifest,
            true,
        )
        .is_err());
    }

    #[test]
    fn alpha_art_inner_validator_rejects_forbidden_artifact_path() {
        let root = temp_root("forbidden");
        let mut manifest = required_manifest(&root);
        manifest.entries[0].relative_path = "target/artifacts/sprite.png".to_string();
        assert!(validate_alpha_art_manifest_inner(
            &root,
            &root.join("manifest.json"),
            &manifest,
            false,
        )
        .is_err());
    }
}
