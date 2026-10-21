//! Production art integrity and license metadata validation.

use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use alife_world::persistence::PortableAssetDigest;

use crate::prelude::*;
use crate::*;

pub const FVR07_PRODUCTION_ASSET_MANIFEST_RELATIVE_PATH: &str =
    "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json";
pub const FVR07_PRODUCTION_ASSET_MANIFEST_APP_RELATIVE_PATH: &str =
    "assets/production_voxel_v1/production_asset_manifest.json";
pub const FVR07_PRODUCTION_ASSET_PACK_ID: &str = "production-voxel-v1";
pub const FVR07_ART_DIRECTION: &str = "stylized-voxel-alife-production-v1";
// Editable Blender sources and full authoring-scene GLBs are not runtime assets.

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProductionVoxelAssetManifest {
    pub schema: String,
    pub schema_version: u16,
    pub pack_id: String,
    pub art_direction: String,
    pub generated_art_target: String,
    pub entries: Vec<ProductionVoxelAssetEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProductionVoxelAssetEntry {
    pub asset_id: String,
    pub usage_category: String,
    pub local_path: String,
    pub digest: String,
    pub size_bytes: u64,
    pub source: String,
    pub license: String,
    pub license_ref: String,
    pub author: String,
    pub generated: bool,
    pub generator: Option<GeneratedAssetSource>,
    pub external: bool,
    pub replacement_policy: String,
    pub final_art: bool,
    pub placeholder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct GeneratedAssetSource {
    pub tool: String,
    pub config_path: String,
    pub seed: String,
    pub date: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionAssetValidationSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub pack_id: String,
    pub manifest_path: PathBuf,
    pub asset_count: usize,
    pub generated_assets: usize,
    pub external_assets: usize,
    pub final_art_entries: usize,
    pub placeholder_final_entries: usize,
    pub unknown_license_entries: usize,
    pub missing_or_rejected_assets: usize,
    pub committed_asset_bytes: u64,
    pub largest_asset_bytes: u64,
    pub generated_art_target: String,
}

impl ProductionAssetValidationSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA
            || self.schema_version != FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA_VERSION
            || self.pack_id != FVR07_PRODUCTION_ASSET_PACK_ID
            || self.asset_count == 0
            || self.placeholder_final_entries != 0
            || self.unknown_license_entries != 0
            || self.missing_or_rejected_assets != 0
            || self.committed_asset_bytes == 0
            || self.generated_art_target.trim().is_empty()
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "FVR07 production asset manifest failed validation".to_string(),
            });
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:assets={}:generated={}:external={}:licenses={}:placeholder_final={}:bytes={}:largest={}",
            self.schema,
            self.schema_version,
            self.pack_id,
            self.asset_count,
            self.generated_assets,
            self.external_assets,
            self.unknown_license_entries,
            self.placeholder_final_entries,
            self.committed_asset_bytes,
            self.largest_asset_bytes,
        )
    }
}

pub fn default_production_asset_manifest_path() -> PathBuf {
    ca12_workspace_root().join(FVR07_PRODUCTION_ASSET_MANIFEST_RELATIVE_PATH)
}

pub fn production_asset_manifest_path_for_environment_manifest(
    environment_manifest_path: impl AsRef<Path>,
) -> PathBuf {
    let candidate = environment_manifest_path
        .as_ref()
        .parent()
        .map(|parent| parent.join(FVR07_PRODUCTION_ASSET_MANIFEST_APP_RELATIVE_PATH));
    match candidate {
        Some(path) if path.exists() => path,
        _ => default_production_asset_manifest_path(),
    }
}

fn production_asset_validation_root(manifest_path: &Path) -> PathBuf {
    manifest_path
        .ancestors()
        .nth(5)
        .map(Path::to_path_buf)
        .unwrap_or_else(ca12_workspace_root)
}

pub fn validate_production_assets(
    manifest_path: impl AsRef<Path>,
) -> Result<ProductionAssetValidationSummary, GameAppShellError> {
    let manifest_path = manifest_path.as_ref();
    let manifest: ProductionVoxelAssetManifest =
        serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
    let root = production_asset_validation_root(manifest_path);
    let summary = validate_production_assets_inner(&root, manifest_path, &manifest)?;
    summary.validate()?;
    Ok(summary)
}

pub(crate) fn validate_production_assets_inner(
    root: &Path,
    manifest_path: &Path,
    manifest: &ProductionVoxelAssetManifest,
) -> Result<ProductionAssetValidationSummary, GameAppShellError> {
    validate_manifest_header(manifest)?;
    validate_generated_target_path(&manifest.generated_art_target)?;

    let mut asset_ids = BTreeSet::new();
    let mut committed_asset_bytes = 0_u64;
    let mut largest_asset_bytes = 0_u64;
    let mut generated_assets = 0_usize;
    let mut external_assets = 0_usize;
    let mut final_art_entries = 0_usize;
    let mut placeholder_final_entries = 0_usize;
    let mut unknown_license_entries = 0_usize;
    let mut missing_or_rejected_assets = 0_usize;

    for entry in &manifest.entries {
        if validate_production_asset_entry(root, entry, &mut asset_ids).is_err() {
            missing_or_rejected_assets += 1;
            continue;
        }
        if !license_allowed(&entry.license) {
            unknown_license_entries += 1;
        }
        if entry.generated {
            generated_assets += 1;
        }
        if entry.external {
            external_assets += 1;
        }
        if entry.final_art {
            final_art_entries += 1;
        }
        if entry.placeholder && entry.final_art {
            placeholder_final_entries += 1;
        }
        committed_asset_bytes = committed_asset_bytes.saturating_add(entry.size_bytes);
        largest_asset_bytes = largest_asset_bytes.max(entry.size_bytes);
    }

    Ok(ProductionAssetValidationSummary {
        schema: FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA,
        schema_version: FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA_VERSION,
        pack_id: manifest.pack_id.clone(),
        manifest_path: manifest_path.to_path_buf(),
        asset_count: manifest.entries.len(),
        generated_assets,
        external_assets,
        final_art_entries,
        placeholder_final_entries,
        unknown_license_entries,
        missing_or_rejected_assets,
        committed_asset_bytes,
        largest_asset_bytes,
        generated_art_target: manifest.generated_art_target.clone(),
    })
}

fn validate_manifest_header(
    manifest: &ProductionVoxelAssetManifest,
) -> Result<(), GameAppShellError> {
    if manifest.schema != FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA
        || manifest.schema_version != FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA_VERSION
        || manifest.pack_id != FVR07_PRODUCTION_ASSET_PACK_ID
        || manifest.art_direction != FVR07_ART_DIRECTION
        || manifest.entries.is_empty()
    {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "invalid FVR07 production asset manifest header".to_string(),
        });
    }
    Ok(())
}

fn validate_production_asset_entry(
    root: &Path,
    entry: &ProductionVoxelAssetEntry,
    asset_ids: &mut BTreeSet<String>,
) -> Result<(), GameAppShellError> {
    require_production_asset_id(&entry.asset_id)?;
    require_production_asset_id(&entry.usage_category)?;
    if !asset_ids.insert(entry.asset_id.clone()) {
        return Err(invalid_asset("duplicate production asset id"));
    }
    validate_production_asset_local_path(&entry.local_path, false)?;
    let path = root.join(&entry.local_path);
    if !path.is_file() {
        return Err(invalid_asset("production asset local path is missing"));
    }
    let size = std::fs::metadata(&path)?.len();
    if size == 0 || size != entry.size_bytes {
        return Err(invalid_asset("production asset size is invalid"));
    }
    let digest = PortableAssetDigest(entry.digest.clone());
    digest.validate_format()?;
    let actual_digest = PortableAssetDigest::for_file(&path)?;
    if actual_digest != digest {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: format!(
                "digest mismatch for production asset {}: expected {}, got {}",
                entry.asset_id, digest.0, actual_digest.0
            ),
        });
    }
    if entry.source.trim().is_empty()
        || entry.license.trim().is_empty()
        || entry.license_ref.trim().is_empty()
        || entry.author.trim().is_empty()
        || entry.replacement_policy.trim().is_empty()
        || entry.placeholder
        || entry.generated == entry.external
    {
        return Err(invalid_asset("production asset metadata is incomplete"));
    }
    if entry.generated {
        let generator = entry
            .generator
            .as_ref()
            .ok_or_else(|| invalid_asset("generated asset is missing generator metadata"))?;
        validate_generated_asset_source(generator)?;
    } else if entry.generator.is_some() {
        return Err(invalid_asset(
            "external asset cannot carry generator metadata",
        ));
    }
    if !license_allowed(&entry.license) {
        return Err(invalid_asset(
            "production asset has unknown or rejected license",
        ));
    }
    Ok(())
}

fn validate_generated_asset_source(
    generator: &GeneratedAssetSource,
) -> Result<(), GameAppShellError> {
    if generator.tool.trim().is_empty()
        || generator.seed.trim().is_empty()
        || generator.date.trim().is_empty()
    {
        return Err(invalid_asset(
            "generated asset source metadata is incomplete",
        ));
    }
    validate_production_asset_local_path(&generator.config_path, false)?;
    Ok(())
}

fn validate_generated_target_path(relative: &str) -> Result<(), GameAppShellError> {
    validate_production_asset_local_path(relative, true)?;
    if !relative.starts_with("target/") {
        return Err(invalid_asset(
            "generated art target must live under target/",
        ));
    }
    Ok(())
}

fn license_allowed(license: &str) -> bool {
    matches!(
        license,
        "A-Life-Generated-Source"
            | "CC0-1.0"
            | "CC-BY-4.0"
            | "Public-Domain"
            | "MIT"
            | "Apache-2.0"
            | "BSD-2-Clause"
            | "BSD-3-Clause"
            | "Zlib"
    )
}

fn require_production_asset_id(value: &str) -> Result<(), GameAppShellError> {
    if value.is_empty()
        || value.len() > 80
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(invalid_asset("invalid production asset identifier"));
    }
    Ok(())
}

fn validate_production_asset_local_path(
    relative: &str,
    allow_target: bool,
) -> Result<(), GameAppShellError> {
    if relative.trim().is_empty() {
        return Err(invalid_asset("empty production asset path"));
    }
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err(invalid_asset(
            "absolute production asset paths are forbidden",
        ));
    }
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                let lower = name.to_string_lossy().to_ascii_lowercase();
                if matches!(
                    lower.as_str(),
                    "captures" | "screenshots" | ".cache" | "cache"
                ) || (!allow_target && matches!(lower.as_str(), "target" | "artifacts"))
                {
                    return Err(invalid_asset(
                        "production asset path points at a generated/cache artifact",
                    ));
                }
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(invalid_asset("production asset path escapes workspace"));
            }
            Component::CurDir => {}
        }
    }
    Ok(())
}

fn invalid_asset(message: &'static str) -> GameAppShellError {
    GameAppShellError::InvalidProductionFrontend {
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENEFORGE_RECIPE_PATH: &str =
        "crates/alife_game_app/assets/production_voxel_v1/creature_parts/geneforge_recipes.json";
    const GENEFORGE_OUTPUT_COUNT: usize = 168;

    fn collect_recipe_output_paths(value: &serde_json::Value, output: &mut BTreeSet<String>) {
        match value {
            serde_json::Value::Object(fields) => {
                for (key, value) in fields {
                    if matches!(
                        key.as_str(),
                        "generated_obj" | "socket_manifest" | "semantic_mask" | "anatomy_mask"
                    ) {
                        let path = value
                            .as_str()
                            .unwrap_or_else(|| panic!("{key} must contain a string path"));
                        output.insert(format!("crates/alife_game_app/assets/{path}"));
                    } else {
                        collect_recipe_output_paths(value, output);
                    }
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    collect_recipe_output_paths(value, output);
                }
            }
            _ => {}
        }
    }

    fn fixture_manifest() -> ProductionVoxelAssetManifest {
        serde_json::from_str(
            &std::fs::read_to_string(default_production_asset_manifest_path()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn production_asset_manifest_validates_license_and_digest() {
        let path = default_production_asset_manifest_path();
        let summary = validate_production_assets(&path).unwrap();
        summary.validate().unwrap();
        assert_eq!(summary.unknown_license_entries, 0);
        assert_eq!(summary.placeholder_final_entries, 0);
        assert_eq!(summary.missing_or_rejected_assets, 0);
    }

    #[test]
    fn retired_geneforge_outputs_are_not_required_by_the_production_pack() {
        let root = ca12_workspace_root();
        let recipe: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join(GENEFORGE_RECIPE_PATH)).unwrap(),
        )
        .unwrap();
        let mut retired_paths = BTreeSet::new();
        collect_recipe_output_paths(&recipe, &mut retired_paths);
        assert_eq!(retired_paths.len(), GENEFORGE_OUTPUT_COUNT);
        assert!(retired_paths.iter().all(|path| !root.join(path).exists()));
        let manifest = fixture_manifest();
        assert!(manifest
            .entries
            .iter()
            .all(|entry| !retired_paths.contains(&entry.local_path)));
        validate_production_assets(default_production_asset_manifest_path()).unwrap();
    }

    #[test]
    fn superseded_whole_body_pack_is_absent_from_production_assets() {
        let root = ca12_workspace_root();
        let generated =
            root.join("crates/alife_game_app/assets/production_voxel_v1/creature_parts/generated");
        let stale_generated = std::fs::read_dir(generated)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|entry| entry.path().is_file())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(
            stale_generated.is_empty(),
            "superseded generated family files remain: {stale_generated:?}"
        );

        let models = root.join("crates/alife_game_app/assets/production_voxel_v1/models");
        let stale_surfaces = std::fs::read_dir(models)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with("T_") && name.ends_with(".png"))
            .collect::<Vec<_>>();
        assert!(
            stale_surfaces.is_empty(),
            "superseded whole-body surfaces remain: {stale_surfaces:?}"
        );
    }

    #[test]
    fn production_asset_manifest_reports_every_rejected_entry() {
        let root = ca12_workspace_root();
        let manifest = fixture_manifest();
        let mut asset_ids = BTreeSet::new();
        let rejected = manifest
            .entries
            .iter()
            .filter_map(|entry| {
                validate_production_asset_entry(&root, entry, &mut asset_ids)
                    .err()
                    .map(|error| format!("{}: {error}", entry.asset_id))
            })
            .collect::<Vec<_>>();
        assert!(
            rejected.is_empty(),
            "rejected production assets:\n{}",
            rejected.join("\n")
        );
    }

    #[test]
    fn fvr11_terrain_atlases_are_manifested_and_compact() {
        let path = default_production_asset_manifest_path();
        let manifest: ProductionVoxelAssetManifest =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let required = [
            "terrain-albedo-atlas",
            "terrain-normal-atlas",
            "terrain-orm-atlas",
        ];
        for asset_id in required {
            let entry = manifest
                .entries
                .iter()
                .find(|entry| entry.asset_id == asset_id)
                .unwrap_or_else(|| panic!("missing {asset_id}"));
            assert!(entry.final_art);
            assert!(!entry.placeholder);
            assert!(entry.generated);
            assert!(!entry.external);
            assert!(entry
                .local_path
                .starts_with("crates/alife_game_app/assets/production_voxel_v1/terrain/"));
        }
        validate_production_assets(&path).unwrap();
    }

    #[test]
    fn production_asset_manifest_rejects_unknown_license() {
        let root = ca12_workspace_root();
        let path = default_production_asset_manifest_path();
        let mut manifest = fixture_manifest();
        manifest.entries[0].license = "unknown".to_string();
        assert!(validate_production_assets_inner(&root, &path, &manifest)
            .and_then(|summary| summary.validate())
            .is_err());
    }

    #[test]
    fn production_asset_manifest_rejects_placeholder_claimed_final() {
        let root = ca12_workspace_root();
        let path = default_production_asset_manifest_path();
        let mut manifest = fixture_manifest();
        manifest.entries[0].placeholder = true;
        assert!(validate_production_assets_inner(&root, &path, &manifest)
            .and_then(|summary| summary.validate())
            .is_err());
    }

    #[test]
    fn production_asset_manifest_rejects_target_local_assets() {
        let root = ca12_workspace_root();
        let path = default_production_asset_manifest_path();
        let mut manifest = fixture_manifest();
        manifest.entries[0].local_path =
            "target/generated_art/production_voxel_v1/bad.json".to_string();
        assert!(validate_production_assets_inner(&root, &path, &manifest)
            .and_then(|summary| summary.validate())
            .is_err());
    }
}
