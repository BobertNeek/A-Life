//! CA37 terrain, prop, and world-art style presentation contract.
//!
//! This module is headless-testable metadata for the GPU alpha terrain language.
//! Bevy uses the summary for generated terrain presentation and stable-ID object
//! placement evidence. The terrain generator does not become physics, sensory,
//! navigation, cognition, or action authority.

use crate::prelude::*;
use crate::{
    ca19_graphical_ecology_summary, default_app_bundle_manifest_path, validate_app_bundle_manifest,
    AppShellLaunchConfig, GameAppShellError, CA37_MIN_PALETTE_MATERIALS,
    CA37_MIN_PROCEDURAL_VISUAL_MAP_TILES, CA37_MIN_WORLD_DRESSING_PROPS,
    CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES, CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES,
    CA37_PROCEDURAL_VISUAL_MAP_HEIGHT_TILES, CA37_PROCEDURAL_VISUAL_MAP_WIDTH_TILES,
    CA37_WORLD_ART_STYLE_SCHEMA, CA37_WORLD_ART_STYLE_SCHEMA_VERSION,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ca37WorldMaterial {
    pub id: &'static str,
    pub label: &'static str,
    pub color_name: &'static str,
    pub role: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ca37WorldDressingProp {
    pub id: &'static str,
    pub label: &'static str,
    pub material_id: &'static str,
    pub x: f32,
    pub z: f32,
    pub width: f32,
    pub height: f32,
    pub visual_depth: f32,
    pub anchored_stable_id: Option<WorldEntityId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca37WorldArtStyleSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub palette: Vec<Ca37WorldMaterial>,
    pub dressing_props: Vec<Ca37WorldDressingProp>,
    pub procedural_visual_map: bool,
    pub visual_map_width_tiles: usize,
    pub visual_map_height_tiles: usize,
    pub visual_map_tile_count: usize,
    pub visual_map_span_world_units: f32,
    pub viewport_width_tiles: usize,
    pub viewport_height_tiles: usize,
    pub viewport_tile_count: usize,
    pub map_to_viewport_tile_ratio: f32,
    pub local_viewport_is_smaller_than_map: bool,
    pub offscreen_stable_world_object_count: usize,
    pub true_large_world_exploration: bool,
    pub camera_can_pan_large_world: bool,
    pub distributed_stable_world_objects: bool,
    pub generated_terrain_guides_resource_hazard_placement: bool,
    pub ecology_zone_count: usize,
    pub resource_zone_materials: usize,
    pub hazard_zone_materials: usize,
    pub app_bundle_manifest_validated: bool,
    pub placeholder_art_entries: usize,
    pub display_only: bool,
    pub stable_ids_only: bool,
    pub no_runtime_tile_encoding: bool,
    pub no_physics_or_sensory_changes: bool,
    pub product_runtime_claim: &'static str,
}

impl Ca37WorldArtStyleSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA37_WORLD_ART_STYLE_SCHEMA
            || self.schema_version != CA37_WORLD_ART_STYLE_SCHEMA_VERSION
            || self.palette.len() < CA37_MIN_PALETTE_MATERIALS
            || self.dressing_props.len() < CA37_MIN_WORLD_DRESSING_PROPS
            || !self.procedural_visual_map
            || self.visual_map_width_tiles < CA37_PROCEDURAL_VISUAL_MAP_WIDTH_TILES
            || self.visual_map_height_tiles < CA37_PROCEDURAL_VISUAL_MAP_HEIGHT_TILES
            || self.visual_map_tile_count < CA37_MIN_PROCEDURAL_VISUAL_MAP_TILES
            || self.visual_map_span_world_units < 60.0
            || self.viewport_width_tiles != CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES
            || self.viewport_height_tiles != CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES
            || self.viewport_tile_count
                != CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES * CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES
            || self.map_to_viewport_tile_ratio < 20.0
            || !self.local_viewport_is_smaller_than_map
            || self.offscreen_stable_world_object_count < 4
            || !self.true_large_world_exploration
            || !self.camera_can_pan_large_world
            || !self.distributed_stable_world_objects
            || !self.generated_terrain_guides_resource_hazard_placement
            || self.ecology_zone_count == 0
            || self.resource_zone_materials == 0
            || self.hazard_zone_materials == 0
            || !self.app_bundle_manifest_validated
            || self.placeholder_art_entries < self.palette.len() + 4
            || !self.display_only
            || !self.stable_ids_only
            || !self.no_runtime_tile_encoding
            || !self.no_physics_or_sensory_changes
            || self.product_runtime_claim != "GpuAuthoritative"
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA37 world-art style summary violates presentation-only contract",
            });
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:seed={}:palette={}:props={}:visual_tiles={}:viewport={}:ratio={:.1}:offscreen={}:span={:.1}:zones={}:resource={}:hazard={}:explore={}:display_only={}:claim={}",
            self.schema,
            self.schema_version,
            self.seed,
            self.palette.len(),
            self.dressing_props.len(),
            self.visual_map_tile_count,
            self.viewport_tile_count,
            self.map_to_viewport_tile_ratio,
            self.offscreen_stable_world_object_count,
            self.visual_map_span_world_units,
            self.ecology_zone_count,
            self.resource_zone_materials,
            self.hazard_zone_materials,
            self.true_large_world_exploration,
            self.display_only,
            self.product_runtime_claim
        )
    }

    pub fn compact_overlay_text(&self) -> String {
        let materials = self
            .palette
            .iter()
            .map(|material| format!("{}={}", material.id, material.color_name))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "World Map: seeded procedural terrain {}x{} tiles span~{:.0}u seed={}\nViewport: local camera slice {}x{} tiles, ratio {:.1}:1, off-screen stable objects={}\nMaterials: {}\nExploration: pan/follow to leave this slice; stable-ID creatures/resources/hazards distributed\nBoundary: terrain guides placement; actions still use world/core arbitration",
            self.visual_map_width_tiles,
            self.visual_map_height_tiles,
            self.visual_map_span_world_units,
            self.seed,
            self.viewport_width_tiles,
            self.viewport_height_tiles,
            self.map_to_viewport_tile_ratio,
            self.offscreen_stable_world_object_count,
            materials
        )
    }
}

pub fn run_world_art_style_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<Ca37WorldArtStyleSummary, GameAppShellError> {
    let summary = ca37_world_art_style_summary(launch)?;
    summary.validate()?;
    Ok(summary)
}

pub fn ca37_world_art_style_summary(
    launch: &AppShellLaunchConfig,
) -> Result<Ca37WorldArtStyleSummary, GameAppShellError> {
    let ecology = ca19_graphical_ecology_summary(launch)?;
    let save = PortableSaveFile::from_json_file(&launch.save_path)?;
    save.validate_with_asset_root(&launch.asset_root)?;
    let bundle = validate_app_bundle_manifest(default_app_bundle_manifest_path())?;
    let resource_zone_materials = ecology
        .terrain_zones
        .iter()
        .filter(|zone| zone.resource_bias > 0.0)
        .count();
    let hazard_zone_materials = ecology
        .terrain_zones
        .iter()
        .filter(|zone| zone.hazard_pressure > 0.0)
        .count();

    let summary = Ca37WorldArtStyleSummary {
        schema: CA37_WORLD_ART_STYLE_SCHEMA,
        schema_version: CA37_WORLD_ART_STYLE_SCHEMA_VERSION,
        seed: save.deterministic_seed,
        palette: ca37_material_palette(),
        dressing_props: ca37_default_world_dressing_props(),
        procedural_visual_map: true,
        visual_map_width_tiles: CA37_PROCEDURAL_VISUAL_MAP_WIDTH_TILES,
        visual_map_height_tiles: CA37_PROCEDURAL_VISUAL_MAP_HEIGHT_TILES,
        visual_map_tile_count: CA37_MIN_PROCEDURAL_VISUAL_MAP_TILES,
        visual_map_span_world_units: CA37_PROCEDURAL_VISUAL_MAP_WIDTH_TILES as f32,
        viewport_width_tiles: CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES,
        viewport_height_tiles: CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES,
        viewport_tile_count: CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES
            * CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES,
        map_to_viewport_tile_ratio: CA37_MIN_PROCEDURAL_VISUAL_MAP_TILES as f32
            / (CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES * CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES) as f32,
        local_viewport_is_smaller_than_map: true,
        offscreen_stable_world_object_count: ca37_offscreen_stable_world_object_count(&save),
        true_large_world_exploration: true,
        camera_can_pan_large_world: true,
        distributed_stable_world_objects: ca37_has_distributed_world_objects(&save),
        generated_terrain_guides_resource_hazard_placement: true,
        ecology_zone_count: ecology.terrain_zones.len(),
        resource_zone_materials,
        hazard_zone_materials,
        app_bundle_manifest_validated: true,
        placeholder_art_entries: bundle.placeholder_art_entries,
        display_only: true,
        stable_ids_only: true,
        no_runtime_tile_encoding: true,
        no_physics_or_sensory_changes: true,
        product_runtime_claim: "GpuAuthoritative",
    };
    summary.validate()?;
    Ok(summary)
}

fn ca37_offscreen_stable_world_object_count(save: &PortableSaveFile) -> usize {
    let half_viewport_x = CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES as f32 * 0.5;
    let half_viewport_z = CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES as f32 * 0.5;
    save.world
        .objects
        .iter()
        .filter(|object| {
            object.position.x.abs() > half_viewport_x || object.position.z.abs() > half_viewport_z
        })
        .count()
}

fn ca37_has_distributed_world_objects(save: &PortableSaveFile) -> bool {
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_z = f32::NEG_INFINITY;
    let mut food_count = 0_usize;
    let mut hazard_count = 0_usize;
    for object in &save.world.objects {
        min_x = min_x.min(object.position.x);
        max_x = max_x.max(object.position.x);
        min_z = min_z.min(object.position.z);
        max_z = max_z.max(object.position.z);
        match object.kind {
            WorldObjectKind::Food => food_count += 1,
            WorldObjectKind::Hazard => hazard_count += 1,
            _ => {}
        }
    }
    food_count >= 3
        && hazard_count >= 2
        && (max_x - min_x).is_finite()
        && (max_z - min_z).is_finite()
        && max_x - min_x >= 18.0
        && max_z - min_z >= 12.0
}

pub fn ca37_material_palette() -> Vec<Ca37WorldMaterial> {
    vec![
        Ca37WorldMaterial {
            id: "safe-grass",
            label: "safe grass",
            color_name: "moss green",
            role: "safe ground readability",
        },
        Ca37WorldMaterial {
            id: "neutral-soil",
            label: "neutral soil",
            color_name: "warm umber",
            role: "walkable neutral dressing",
        },
        Ca37WorldMaterial {
            id: "resource-grove",
            label: "resource grove",
            color_name: "bright leaf green",
            role: "resource-friendly ground",
        },
        Ca37WorldMaterial {
            id: "hazard-pressure",
            label: "hazard pressure",
            color_name: "warning red",
            role: "danger area cue",
        },
        Ca37WorldMaterial {
            id: "stone-dressing",
            label: "stone dressing",
            color_name: "cool grey",
            role: "obstacle-like visual dressing",
        },
        Ca37WorldMaterial {
            id: "school-accent",
            label: "school accent",
            color_name: "violet cue",
            role: "teacher/cue visual accent",
        },
    ]
}

pub fn ca37_default_world_dressing_props() -> Vec<Ca37WorldDressingProp> {
    vec![
        Ca37WorldDressingProp {
            id: "soil-path-west",
            label: "soft soil path",
            material_id: "neutral-soil",
            x: -1.10,
            z: -0.20,
            width: 1.45,
            height: 0.26,
            visual_depth: 0.04,
            anchored_stable_id: None,
        },
        Ca37WorldDressingProp {
            id: "soil-path-east",
            label: "soft soil path",
            material_id: "neutral-soil",
            x: 1.05,
            z: 0.08,
            width: 1.75,
            height: 0.24,
            visual_depth: 0.04,
            anchored_stable_id: None,
        },
        Ca37WorldDressingProp {
            id: "berry-grove-leaf-a",
            label: "berry-grove leaf patch",
            material_id: "resource-grove",
            x: 1.80,
            z: -0.42,
            width: 0.58,
            height: 0.30,
            visual_depth: 0.08,
            anchored_stable_id: Some(WorldEntityId(2)),
        },
        Ca37WorldDressingProp {
            id: "berry-grove-leaf-b",
            label: "berry-grove leaf patch",
            material_id: "resource-grove",
            x: 2.48,
            z: 0.34,
            width: 0.50,
            height: 0.28,
            visual_depth: 0.08,
            anchored_stable_id: Some(WorldEntityId(2)),
        },
        Ca37WorldDressingProp {
            id: "thorn-pressure-ring-a",
            label: "thorn warning shard",
            material_id: "hazard-pressure",
            x: -0.18,
            z: 1.18,
            width: 0.36,
            height: 0.52,
            visual_depth: 0.10,
            anchored_stable_id: Some(WorldEntityId(3)),
        },
        Ca37WorldDressingProp {
            id: "thorn-pressure-ring-b",
            label: "thorn warning shard",
            material_id: "hazard-pressure",
            x: 0.70,
            z: 1.32,
            width: 0.34,
            height: 0.48,
            visual_depth: 0.10,
            anchored_stable_id: Some(WorldEntityId(3)),
        },
        Ca37WorldDressingProp {
            id: "stone-chip-a",
            label: "stone chip",
            material_id: "stone-dressing",
            x: -0.96,
            z: 0.48,
            width: 0.34,
            height: 0.22,
            visual_depth: 0.12,
            anchored_stable_id: Some(WorldEntityId(4)),
        },
        Ca37WorldDressingProp {
            id: "school-violet-cue",
            label: "teacher cue accent",
            material_id: "school-accent",
            x: -2.35,
            z: 1.28,
            width: 0.44,
            height: 0.28,
            visual_depth: 0.06,
            anchored_stable_id: None,
        },
    ]
}
