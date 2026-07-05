//! FVR07 production voxel asset and VFX manifest validation.
//!
//! This validates production presentation metadata without giving renderer,
//! asset-loader, or VFX systems authority over simulation, actions, or
//! cognition.

use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

use alife_world::persistence::PortableAssetDigest;

use crate::prelude::*;
use crate::*;

pub const FVR07_PRODUCTION_ASSET_MANIFEST_RELATIVE_PATH: &str =
    "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json";
pub const FVR07_PRODUCTION_ASSET_PACK_ID: &str = "production-voxel-v1";
pub const FVR07_ART_DIRECTION: &str = "stylized-voxel-alife-production-v1";
pub const FVR07_MAX_COMMITTED_ASSET_BYTES: u64 = 256 * 1024;
pub const FVR07_MAX_TOTAL_COMMITTED_ASSET_BYTES: u64 = 2 * 1024 * 1024;

pub const FVR07_REQUIRED_USAGE_CATEGORIES: [&str; 15] = [
    "voxel-material-atlas",
    "terrain-materials",
    "water-materials",
    "decay-materials",
    "resource-materials",
    "hazard-materials",
    "creatures",
    "props",
    "nests",
    "corpses",
    "food-resources",
    "selection-hover-effects",
    "ui-icons",
    "environment-dressing",
    "stylization-lighting",
];

pub const FVR07_REQUIRED_VFX_EFFECTS: [&str; 8] = [
    "pheromone-trails",
    "spores",
    "sleep-consolidation-glows",
    "danger-hazard-particles",
    "eating-resource-effects",
    "birth-death-effects",
    "water-decay-ambient-motion",
    "selected-creature-neural-pulse",
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProductionVoxelAssetManifest {
    pub schema: String,
    pub schema_version: u16,
    pub pack_id: String,
    pub art_direction: String,
    pub generated_art_target: String,
    pub loader: ProductionAssetLoaderContract,
    pub entries: Vec<ProductionVoxelAssetEntry>,
    pub vfx_profiles: Vec<ProductionVfxProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProductionAssetLoaderContract {
    pub crate_name: String,
    pub crate_version: String,
    pub production_feature: String,
    pub missing_asset_policy: String,
    pub runtime_dependency: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ProductionVfxProfile {
    pub profile: String,
    pub budget_state: String,
    pub effect_ids: Vec<String>,
    pub particle_cap: u32,
    pub gpu_driven: bool,
    pub display_only: bool,
    pub adaptive: bool,
    pub density_scale_percent: u8,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
    pub no_cognition_mutation: bool,
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
    pub required_usage_categories_present: bool,
    pub final_art_entries: usize,
    pub placeholder_final_entries: usize,
    pub unknown_license_entries: usize,
    pub missing_or_rejected_assets: usize,
    pub committed_asset_bytes: u64,
    pub largest_asset_bytes: u64,
    pub generated_art_target: String,
    pub loader_crate: String,
    pub loader_version: String,
    pub missing_asset_policy: String,
    pub vfx_profile_count: usize,
    pub vfx_effects_present: bool,
    pub minimum_vfx_budget_state: String,
    pub comfort_vfx_budget_state: String,
    pub display_only_vfx: bool,
    pub adaptive_vfx: bool,
    pub no_large_artifacts_committed: bool,
    pub no_renderer_authority: bool,
    pub scale_up_profiles_present: bool,
}

impl ProductionAssetValidationSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA
            || self.schema_version != FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA_VERSION
            || self.pack_id != FVR07_PRODUCTION_ASSET_PACK_ID
            || self.asset_count < FVR07_REQUIRED_USAGE_CATEGORIES.len()
            || !self.required_usage_categories_present
            || self.final_art_entries != self.asset_count
            || self.placeholder_final_entries != 0
            || self.unknown_license_entries != 0
            || self.missing_or_rejected_assets != 0
            || self.committed_asset_bytes == 0
            || self.largest_asset_bytes > FVR07_MAX_COMMITTED_ASSET_BYTES
            || self.committed_asset_bytes > FVR07_MAX_TOTAL_COMMITTED_ASSET_BYTES
            || self.generated_art_target.trim().is_empty()
            || self.loader_crate != "bevy_asset_loader"
            || self.loader_version != "0.26.0"
            || self.missing_asset_policy != "clear-error-or-generated-fallback"
            || self.vfx_profile_count != ProductionFrontendProfileId::all().len()
            || !self.vfx_effects_present
            || self.minimum_vfx_budget_state != "conservative"
            || self.comfort_vfx_budget_state != "medium"
            || !self.display_only_vfx
            || !self.adaptive_vfx
            || !self.no_large_artifacts_committed
            || !self.no_renderer_authority
            || !self.scale_up_profiles_present
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "FVR07 production asset manifest failed validation".to_string(),
            });
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:assets={}:generated={}:external={}:licenses={}:placeholder_final={}:bytes={}:largest={}:vfx_profiles={}:effects={}:min_vfx={}:comfort_vfx={}:authority={}",
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
            self.vfx_profile_count,
            self.vfx_effects_present,
            self.minimum_vfx_budget_state,
            self.comfort_vfx_budget_state,
            self.no_renderer_authority,
        )
    }
}

pub fn default_production_asset_manifest_path() -> PathBuf {
    ca12_workspace_root().join(FVR07_PRODUCTION_ASSET_MANIFEST_RELATIVE_PATH)
}

#[cfg(feature = "production-assets")]
pub fn production_asset_loader_runtime_type_names() -> Vec<&'static str> {
    vec![std::any::type_name::<
        bevy_asset_loader::prelude::DynamicAssets,
    >()]
}

#[cfg(not(feature = "production-assets"))]
pub fn production_asset_loader_runtime_type_names() -> Vec<&'static str> {
    Vec::new()
}

#[cfg(feature = "vfx-hanabi")]
pub fn production_hanabi_runtime_type_names() -> Vec<&'static str> {
    vec![
        std::any::type_name::<bevy_hanabi::EffectAsset>(),
        std::any::type_name::<bevy_hanabi::ParticleEffect>(),
    ]
}

#[cfg(not(feature = "vfx-hanabi"))]
pub fn production_hanabi_runtime_type_names() -> Vec<&'static str> {
    Vec::new()
}

pub fn validate_production_assets(
    manifest_path: impl AsRef<Path>,
) -> Result<ProductionAssetValidationSummary, GameAppShellError> {
    let manifest_path = manifest_path.as_ref();
    let manifest: ProductionVoxelAssetManifest =
        serde_json::from_str(&std::fs::read_to_string(manifest_path)?)?;
    let summary =
        validate_production_assets_inner(&ca12_workspace_root(), manifest_path, &manifest)?;
    summary.validate()?;
    Ok(summary)
}

pub(crate) fn validate_production_assets_inner(
    root: &Path,
    manifest_path: &Path,
    manifest: &ProductionVoxelAssetManifest,
) -> Result<ProductionAssetValidationSummary, GameAppShellError> {
    validate_manifest_header(manifest)?;
    validate_asset_loader_contract(&manifest.loader)?;
    validate_generated_target_path(&manifest.generated_art_target)?;

    let mut asset_ids = BTreeSet::new();
    let mut usage_categories = BTreeSet::new();
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
        usage_categories.insert(entry.usage_category.as_str());
        committed_asset_bytes = committed_asset_bytes.saturating_add(entry.size_bytes);
        largest_asset_bytes = largest_asset_bytes.max(entry.size_bytes);
    }

    let required_usage_categories_present = FVR07_REQUIRED_USAGE_CATEGORIES
        .iter()
        .all(|category| usage_categories.contains(category));
    let vfx_state = validate_vfx_profiles(&manifest.vfx_profiles)?;
    Ok(ProductionAssetValidationSummary {
        schema: FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA,
        schema_version: FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA_VERSION,
        pack_id: manifest.pack_id.clone(),
        manifest_path: manifest_path.to_path_buf(),
        asset_count: manifest.entries.len(),
        generated_assets,
        external_assets,
        required_usage_categories_present,
        final_art_entries,
        placeholder_final_entries,
        unknown_license_entries,
        missing_or_rejected_assets,
        committed_asset_bytes,
        largest_asset_bytes,
        generated_art_target: manifest.generated_art_target.clone(),
        loader_crate: manifest.loader.crate_name.clone(),
        loader_version: manifest.loader.crate_version.clone(),
        missing_asset_policy: manifest.loader.missing_asset_policy.clone(),
        vfx_profile_count: manifest.vfx_profiles.len(),
        vfx_effects_present: vfx_state.effects_present,
        minimum_vfx_budget_state: vfx_state.minimum_budget_state,
        comfort_vfx_budget_state: vfx_state.comfort_budget_state,
        display_only_vfx: vfx_state.display_only,
        adaptive_vfx: vfx_state.adaptive,
        no_large_artifacts_committed: largest_asset_bytes <= FVR07_MAX_COMMITTED_ASSET_BYTES
            && committed_asset_bytes <= FVR07_MAX_TOTAL_COMMITTED_ASSET_BYTES,
        no_renderer_authority: vfx_state.no_renderer_authority,
        scale_up_profiles_present: vfx_state.scale_up_profiles_present,
    })
}

fn validate_manifest_header(
    manifest: &ProductionVoxelAssetManifest,
) -> Result<(), GameAppShellError> {
    if manifest.schema != FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA
        || manifest.schema_version != FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA_VERSION
        || manifest.pack_id != FVR07_PRODUCTION_ASSET_PACK_ID
        || manifest.art_direction != FVR07_ART_DIRECTION
        || manifest.entries.len() < FVR07_REQUIRED_USAGE_CATEGORIES.len()
    {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "invalid FVR07 production asset manifest header".to_string(),
        });
    }
    Ok(())
}

fn validate_asset_loader_contract(
    loader: &ProductionAssetLoaderContract,
) -> Result<(), GameAppShellError> {
    if loader.crate_name != "bevy_asset_loader"
        || loader.crate_version != "0.26.0"
        || loader.production_feature != "production-assets"
        || loader.missing_asset_policy != "clear-error-or-generated-fallback"
        || !loader.runtime_dependency
    {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "invalid FVR07 production asset loader contract".to_string(),
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
    if size == 0 || size != entry.size_bytes || size > FVR07_MAX_COMMITTED_ASSET_BYTES {
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
        || !entry.final_art
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
        validate_generated_asset_source(root, generator)?;
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
    root: &Path,
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
    if !root.join(&generator.config_path).is_file() {
        return Err(invalid_asset("generated asset config path is missing"));
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct VfxValidationState {
    effects_present: bool,
    minimum_budget_state: String,
    comfort_budget_state: String,
    display_only: bool,
    adaptive: bool,
    no_renderer_authority: bool,
    scale_up_profiles_present: bool,
}

fn validate_vfx_profiles(
    profiles: &[ProductionVfxProfile],
) -> Result<VfxValidationState, GameAppShellError> {
    let expected_profiles = ProductionFrontendProfileId::labels()
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut seen_profiles = BTreeSet::new();
    let mut effect_ids = BTreeSet::new();
    let mut minimum_budget_state = String::new();
    let mut comfort_budget_state = String::new();
    let mut display_only = true;
    let mut adaptive = true;
    let mut no_renderer_authority = true;

    for profile in profiles {
        if !seen_profiles.insert(profile.profile.as_str()) {
            return Err(invalid_asset("duplicate FVR07 VFX profile"));
        }
        if profile.particle_cap == 0
            || profile.density_scale_percent == 0
            || profile.density_scale_percent > 200
            || !profile.gpu_driven
            || !profile.display_only
            || !profile.adaptive
            || !profile.no_action_authority
            || !profile.no_weight_authority
            || !profile.no_cognition_mutation
            || profile.effect_ids.is_empty()
        {
            return Err(invalid_asset("invalid FVR07 VFX profile metadata"));
        }
        for effect_id in &profile.effect_ids {
            require_production_asset_id(effect_id)?;
            effect_ids.insert(effect_id.as_str());
        }
        if profile.profile == ProductionFrontendProfileId::MinimumSettings30x30.label() {
            minimum_budget_state = profile.budget_state.clone();
            if profile.particle_cap > 512 || profile.density_scale_percent > 50 {
                return Err(invalid_asset(
                    "minimum VFX budget exceeds conservative floor",
                ));
            }
        }
        if profile.profile == ProductionFrontendProfileId::MinSpecComfort1080p.label() {
            comfort_budget_state = profile.budget_state.clone();
        }
        display_only &= profile.display_only;
        adaptive &= profile.adaptive;
        no_renderer_authority &= profile.no_action_authority
            && profile.no_weight_authority
            && profile.no_cognition_mutation;
    }

    let effects_present = FVR07_REQUIRED_VFX_EFFECTS
        .iter()
        .all(|effect| effect_ids.contains(effect));
    let scale_up_profiles_present = seen_profiles
        .contains(ProductionFrontendProfileId::Balanced1080p.label())
        && seen_profiles.contains(ProductionFrontendProfileId::HighSpecScaleUp.label())
        && seen_profiles.contains(ProductionFrontendProfileId::ResearchScale.label());
    if seen_profiles != expected_profiles || !effects_present {
        return Err(invalid_asset("FVR07 VFX profiles are incomplete"));
    }
    Ok(VfxValidationState {
        effects_present,
        minimum_budget_state,
        comfort_budget_state,
        display_only,
        adaptive,
        no_renderer_authority,
        scale_up_profiles_present,
    })
}

fn license_allowed(license: &str) -> bool {
    matches!(
        license,
        "A-Life-Generated-Source"
            | "CC0-1.0"
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

    fn fixture_manifest() -> ProductionVoxelAssetManifest {
        serde_json::from_str(
            &std::fs::read_to_string(default_production_asset_manifest_path()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn production_asset_manifest_validates_license_digest_and_vfx_contract() {
        let path = default_production_asset_manifest_path();
        let summary = validate_production_assets(&path).unwrap();
        summary.validate().unwrap();
        assert_eq!(summary.unknown_license_entries, 0);
        assert_eq!(summary.placeholder_final_entries, 0);
        assert_eq!(summary.missing_or_rejected_assets, 0);
        assert_eq!(summary.minimum_vfx_budget_state, "conservative");
        assert_eq!(summary.comfort_vfx_budget_state, "medium");
        assert!(summary.display_only_vfx);
        assert!(summary.adaptive_vfx);
        assert!(summary.no_renderer_authority);
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

    #[test]
    fn production_asset_manifest_rejects_non_display_only_vfx() {
        let root = ca12_workspace_root();
        let path = default_production_asset_manifest_path();
        let mut manifest = fixture_manifest();
        manifest.vfx_profiles[0].no_action_authority = false;
        assert!(validate_production_assets_inner(&root, &path, &manifest).is_err());
    }
}
