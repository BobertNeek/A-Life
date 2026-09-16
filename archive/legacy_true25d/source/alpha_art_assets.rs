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
    "terrain-water",
    "terrain-sand",
    "terrain-edge-blend",
    "ground-repeat-tile",
    "world-backdrop",
    "prop-dressing",
    "ui-panel-frame",
    "ui-inspector-frame",
    "ui-status-chip",
    "ui-meter-bar",
    "ui-control-keycap",
];

pub const CA44A_ALPHA_ART_DIRECTION: &str = "production-alpha-imagegen-ground-tiles-v41";

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
    pub production_pixel_quality_validated: bool,
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
            || !self.production_pixel_quality_validated
            || !self.forbidden_artifact_paths_rejected
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:entries={}:props={}:largest={}:total={}:roles={}:png={}:quality={}",
            self.schema,
            self.schema_version,
            self.pack_id,
            self.entry_count,
            self.prop_variant_count,
            self.largest_file_bytes,
            self.total_file_bytes,
            self.required_roles_present,
            self.png_dimensions_validated,
            self.production_pixel_quality_validated
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
    let mut production_pixel_quality_validated = true;
    for entry in &manifest.entries {
        validate_alpha_art_entry(entry, &mut ids)?;
        *roles.entry(entry.role.as_str()).or_insert(0) += 1;
        let path = root.join(&entry.relative_path);
        if check_files {
            let (width, height, file_size) = validate_png_asset_file(&path)?;
            validate_png_asset_pixel_quality(&path, entry)?;
            if width != entry.width || height != entry.height || file_size != entry.file_size_bytes
            {
                return Err(ScaffoldContractError::MissingPhaseData.into());
            }
            largest_file_bytes = largest_file_bytes.max(file_size);
            total_file_bytes = total_file_bytes.saturating_add(file_size);
            png_dimensions_validated &= width > 0 && height > 0;
            production_pixel_quality_validated &= true;
        } else {
            largest_file_bytes = largest_file_bytes.max(entry.file_size_bytes);
            total_file_bytes = total_file_bytes.saturating_add(entry.file_size_bytes);
            png_dimensions_validated &= entry.width > 0 && entry.height > 0;
            production_pixel_quality_validated &= true;
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
        production_pixel_quality_validated,
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

fn validate_png_asset_pixel_quality(
    path: &Path,
    entry: &AlphaArtEntry,
) -> Result<(), GameAppShellError> {
    let bytes = std::fs::read(path)?;
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|_| ScaffoldContractError::MissingPhaseData)?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let metrics = AlphaArtPixelMetrics::from_rgba(width, height, rgba.as_raw());
    let policy = ca44a_pixel_policy_for_role(&entry.role);
    if metrics.visible_pixels < policy.min_visible_pixels
        || metrics.quantized_color_count < policy.min_quantized_colors
        || metrics.luma_range < policy.min_luma_range
        || metrics.alpha_coverage < policy.min_alpha_coverage
        || metrics.alpha_coverage > policy.max_alpha_coverage
        || metrics.edge_alpha_coverage < policy.min_edge_alpha_coverage
        || metrics.bounding_box_area_ratio > policy.max_bounding_box_area_ratio
    {
        return Err(ScaffoldContractError::MissingPhaseData.into());
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct AlphaArtPixelPolicy {
    min_visible_pixels: u64,
    min_quantized_colors: usize,
    min_luma_range: u16,
    min_alpha_coverage: f32,
    max_alpha_coverage: f32,
    min_edge_alpha_coverage: f32,
    max_bounding_box_area_ratio: f32,
}

impl AlphaArtPixelPolicy {
    const fn lenient() -> Self {
        Self {
            min_visible_pixels: 1,
            min_quantized_colors: 1,
            min_luma_range: 0,
            min_alpha_coverage: 0.001,
            max_alpha_coverage: 1.0,
            min_edge_alpha_coverage: 0.0,
            max_bounding_box_area_ratio: 1.0,
        }
    }

    const fn opaque_terrain() -> Self {
        Self {
            min_visible_pixels: 9_216,
            min_quantized_colors: 12,
            min_luma_range: 18,
            min_alpha_coverage: 0.98,
            max_alpha_coverage: 1.0,
            min_edge_alpha_coverage: 0.94,
            max_bounding_box_area_ratio: 1.0,
        }
    }

    const fn world_sprite() -> Self {
        Self {
            min_visible_pixels: 420,
            min_quantized_colors: 10,
            min_luma_range: 20,
            min_alpha_coverage: 0.025,
            max_alpha_coverage: 0.80,
            min_edge_alpha_coverage: 0.0,
            max_bounding_box_area_ratio: 0.88,
        }
    }

    const fn backdrop() -> Self {
        Self {
            min_visible_pixels: 400_000,
            min_quantized_colors: 64,
            min_luma_range: 32,
            min_alpha_coverage: 0.98,
            max_alpha_coverage: 1.0,
            min_edge_alpha_coverage: 0.95,
            max_bounding_box_area_ratio: 1.0,
        }
    }
}

fn ca44a_pixel_policy_for_role(role: &str) -> AlphaArtPixelPolicy {
    match role {
        "terrain-safe-grass"
        | "terrain-soil-path"
        | "terrain-resource-grove"
        | "terrain-hazard-pressure"
        | "terrain-stone-rough"
        | "terrain-water"
        | "terrain-sand"
        | "ground-repeat-tile" => AlphaArtPixelPolicy::opaque_terrain(),
        "creature-idle" | "creature-hurt" | "creature-moving" | "creature-eat"
        | "creature-sleep" | "creature-signal" | "food" | "food-variant" | "hazard"
        | "hazard-active" | "rock-obstacle" | "prop-dressing" => {
            AlphaArtPixelPolicy::world_sprite()
        }
        "world-backdrop" => AlphaArtPixelPolicy::backdrop(),
        _ => AlphaArtPixelPolicy::lenient(),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct AlphaArtPixelMetrics {
    visible_pixels: u64,
    alpha_coverage: f32,
    edge_alpha_coverage: f32,
    bounding_box_area_ratio: f32,
    quantized_color_count: usize,
    luma_range: u16,
}

impl AlphaArtPixelMetrics {
    fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Self {
        let total_pixels = u64::from(width) * u64::from(height);
        let mut visible_pixels = 0_u64;
        let mut edge_visible_pixels = 0_u64;
        let edge_pixels = u64::from(width.saturating_mul(2) + height.saturating_mul(2))
            .saturating_sub(4)
            .max(1);
        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0_u32;
        let mut max_y = 0_u32;
        let mut quantized = BTreeSet::new();
        let mut min_luma = u16::MAX;
        let mut max_luma = 0_u16;

        for y in 0..height {
            for x in 0..width {
                let index = ((y * width + x) * 4) as usize;
                let Some(chunk) = rgba.get(index..index + 4) else {
                    continue;
                };
                let [r, g, b, a] = [chunk[0], chunk[1], chunk[2], chunk[3]];
                if a <= 16 {
                    continue;
                }
                visible_pixels += 1;
                if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
                    edge_visible_pixels += 1;
                }
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                quantized.insert((r / 16, g / 16, b / 16, a / 32));
                let luma = (u16::from(r) * 77 + u16::from(g) * 150 + u16::from(b) * 29) / 256;
                min_luma = min_luma.min(luma);
                max_luma = max_luma.max(luma);
            }
        }

        let bounding_box_area_ratio = if visible_pixels == 0 {
            0.0
        } else {
            let box_width = u64::from(max_x.saturating_sub(min_x) + 1);
            let box_height = u64::from(max_y.saturating_sub(min_y) + 1);
            (box_width * box_height) as f32 / total_pixels.max(1) as f32
        };
        let luma_range = if visible_pixels == 0 {
            0
        } else {
            max_luma.saturating_sub(min_luma)
        };

        Self {
            visible_pixels,
            alpha_coverage: visible_pixels as f32 / total_pixels.max(1) as f32,
            edge_alpha_coverage: edge_visible_pixels as f32 / edge_pixels as f32,
            bounding_box_area_ratio,
            quantized_color_count: quantized.len(),
            luma_range,
        }
    }
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

    fn valid_png_bytes_for_role(role: &str, index: usize) -> &'static [u8] {
        match role {
            "creature-idle" => include_bytes!("../assets/alpha_art_v1/creature_idle.png"),
            "creature-hurt" => include_bytes!("../assets/alpha_art_v1/creature_hurt.png"),
            "creature-moving" => include_bytes!("../assets/alpha_art_v1/creature_moving.png"),
            "creature-eat" => include_bytes!("../assets/alpha_art_v1/creature_eat.png"),
            "creature-sleep" => include_bytes!("../assets/alpha_art_v1/creature_sleep.png"),
            "creature-signal" => include_bytes!("../assets/alpha_art_v1/creature_signal.png"),
            "selection-ring" => include_bytes!("../assets/alpha_art_v1/selection_ring.png"),
            "selection-pulse" => include_bytes!("../assets/alpha_art_v1/selection_pulse.png"),
            "food" => include_bytes!("../assets/alpha_art_v1/food_sprout.png"),
            "food-variant" => include_bytes!("../assets/alpha_art_v1/food_bloom.png"),
            "hazard" => include_bytes!("../assets/alpha_art_v1/hazard_crystal.png"),
            "hazard-active" => include_bytes!("../assets/alpha_art_v1/hazard_glow.png"),
            "ambient-canopy-shadow" => {
                include_bytes!("../assets/alpha_art_v1/ambient_canopy_shadow.png")
            }
            "ambient-light-pool" => {
                include_bytes!("../assets/alpha_art_v1/ambient_light_pool.png")
            }
            "entity-shadow" => include_bytes!("../assets/alpha_art_v1/entity_shadow.png"),
            "rock-obstacle" => include_bytes!("../assets/alpha_art_v1/rock_cluster.png"),
            "terrain-safe-grass" => {
                include_bytes!("../assets/alpha_art_v1/terrain_safe_grass.png")
            }
            "terrain-soil-path" => {
                include_bytes!("../assets/alpha_art_v1/terrain_soil_path.png")
            }
            "terrain-resource-grove" => {
                include_bytes!("../assets/alpha_art_v1/terrain_resource_grove.png")
            }
            "terrain-hazard-pressure" => {
                include_bytes!("../assets/alpha_art_v1/terrain_hazard_pressure.png")
            }
            "terrain-stone-rough" => {
                include_bytes!("../assets/alpha_art_v1/terrain_stone_rough.png")
            }
            "terrain-water" => include_bytes!("../assets/alpha_art_v1/terrain_water.png"),
            "terrain-sand" => include_bytes!("../assets/alpha_art_v1/terrain_sand.png"),
            "terrain-edge-blend" => {
                include_bytes!("../assets/alpha_art_v1/terrain_edge_blend.png")
            }
            "ground-repeat-tile" => {
                include_bytes!("../assets/alpha_art_v1/ground_tile_repeat.png")
            }
            "world-backdrop" => {
                include_bytes!("../assets/alpha_art_v1/world_backdrop_gpu_alpha.png")
            }
            "prop-dressing" => match index % 5 {
                0 => include_bytes!("../assets/alpha_art_v1/prop_grass_tuft.png"),
                1 => include_bytes!("../assets/alpha_art_v1/prop_pebble_cluster.png"),
                2 => include_bytes!("../assets/alpha_art_v1/prop_warning_shard.png"),
                3 => include_bytes!("../assets/alpha_art_v1/prop_leaf_patch.png"),
                _ => include_bytes!("../assets/alpha_art_v1/prop_mushroom_cluster.png"),
            },
            "ui-panel-frame" => include_bytes!("../assets/alpha_art_v1/ui_panel_frame.png"),
            "ui-inspector-frame" => include_bytes!("../assets/alpha_art_v1/ui_inspector_frame.png"),
            "ui-status-chip" => include_bytes!("../assets/alpha_art_v1/ui_status_chip.png"),
            "ui-meter-bar" => include_bytes!("../assets/alpha_art_v1/ui_meter_bar.png"),
            "ui-control-keycap" => include_bytes!("../assets/alpha_art_v1/ui_control_keycap.png"),
            _ => include_bytes!("../assets/alpha_art_v1/creature_idle.png"),
        }
    }

    fn write_test_png(path: &Path, width: u32, height: u32, rgba: [u8; 4]) {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba(rgba));
        image
            .save_with_format(path, image::ImageFormat::Png)
            .unwrap();
    }

    fn required_manifest(root: &Path) -> AlphaArtManifest {
        let mut entries = Vec::new();
        for (index, role) in CA44A_REQUIRED_ALPHA_ART_ROLE_NAMES.iter().enumerate() {
            let path = root.join(format!("alpha_art/{role}-{index}.png"));
            std::fs::write(&path, valid_png_bytes_for_role(role, index)).unwrap();
            let (width, height, file_size_bytes) = validate_png_asset_file(&path).unwrap();
            entries.push(AlphaArtEntry {
                id: format!("{role}-{index}"),
                role: (*role).to_string(),
                kind: if *role == "prop-dressing" {
                    "prop".to_string()
                } else {
                    "sprite".to_string()
                },
                relative_path: format!("alpha_art/{role}-{index}.png"),
                width,
                height,
                file_size_bytes,
            });
        }
        for index in 0..2 {
            let path = root.join(format!("alpha_art/prop-extra-{index}.png"));
            std::fs::write(&path, valid_png_bytes_for_role("prop-dressing", index + 3)).unwrap();
            let (width, height, file_size_bytes) = validate_png_asset_file(&path).unwrap();
            entries.push(AlphaArtEntry {
                id: format!("prop-extra-{index}"),
                role: "prop-dressing".to_string(),
                kind: "prop".to_string(),
                relative_path: format!("alpha_art/prop-extra-{index}.png"),
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

    #[test]
    fn alpha_art_inner_validator_rejects_flat_terrain_swatch() {
        let root = temp_root("flat_terrain");
        let path = root.join("alpha_art/flat-terrain.png");
        write_test_png(&path, 128, 128, [72, 120, 54, 255]);
        let mut manifest = required_manifest(&root);
        manifest.entries[0].role = "terrain-safe-grass".to_string();
        manifest.entries[0].relative_path = "alpha_art/flat-terrain.png".to_string();
        let (width, height, file_size_bytes) = validate_png_asset_file(&path).unwrap();
        manifest.entries[0].width = width;
        manifest.entries[0].height = height;
        manifest.entries[0].file_size_bytes = file_size_bytes;
        assert!(validate_alpha_art_manifest_inner(
            &root,
            &root.join("manifest.json"),
            &manifest,
            true,
        )
        .is_err());
    }

    #[test]
    fn alpha_art_inner_validator_rejects_opaque_square_sprite() {
        let root = temp_root("square_sprite");
        let path = root.join("alpha_art/square-creature.png");
        write_test_png(&path, 128, 128, [82, 209, 225, 255]);
        let mut manifest = required_manifest(&root);
        manifest.entries[0].role = "creature-idle".to_string();
        manifest.entries[0].relative_path = "alpha_art/square-creature.png".to_string();
        let (width, height, file_size_bytes) = validate_png_asset_file(&path).unwrap();
        manifest.entries[0].width = width;
        manifest.entries[0].height = height;
        manifest.entries[0].file_size_bytes = file_size_bytes;
        assert!(validate_alpha_art_manifest_inner(
            &root,
            &root.join("manifest.json"),
            &manifest,
            true,
        )
        .is_err());
    }
}
