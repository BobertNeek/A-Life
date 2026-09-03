//! FVR03 production voxel renderer.
//!
//! This module is Bevy-facing presentation code. It mirrors the persistent
//! voxel truth owned by `alife_world` into selectable chunk/tile meshes without
//! moving renderer handles, Bevy entities, or wgpu state into core/world data.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use alife_bevy_adapter::BevyEntityMap;
use alife_core::{OrganismId, Tick, Vec3f, WorldEntityId};
use alife_world::CreatureAppearanceGenome;
use alife_world::{
    persistence::{CreatureSaveState, GpuRuntimeSaveState, PortableSaveFile},
    CreatureWorldAnchor, PersistentVoxelWorldBackend, PersistentVoxelWorldSnapshot,
    PresentationOutcomeSnapshot, ProceduralTerrainMaterial, ProceduralTileCoord,
    ProceduralWorldConfig, StableVoxelObjectRef, StableVoxelRefKind, VoxelChunkCoord,
    VoxelTileCoord, WorldObjectKind, WorldOrganismPresentationRow,
    FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA,
};
use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    camera::ScalingMode,
    ecs::schedule::{IntoScheduleConfigs, SystemSet},
    math::primitives::InfinitePlane3d,
    mesh::Indices,
    prelude::{
        default, AlphaMode, App, Assets, BackgroundColor, ButtonInput, Camera, ChildOf, Children,
        Color, Commands, Component, Cuboid, DetectChanges, DirectionalLight, Entity, EulerRot,
        GlobalTransform, Handle, Image, KeyCode, Mat4, Mesh, Mesh3d, MeshMaterial3d, MessageReader,
        MessageWriter, MouseButton, Name, Node, NonSend, NonSendMut, ParamSet, PositionType,
        Projection, Quat, Res, ResMut, Resource, StandardMaterial, Text, Text2d, TextColor,
        TextFont, Time, Torus, Transform, Update, Val, Vec3, ViewVisibility, Visibility, Window,
        With, Without, World,
    },
    render::{
        render_resource::PrimitiveTopology,
        view::screenshot::{save_to_disk, Screenshot},
    },
    window::PrimaryWindow,
};

use crate::bevy_shell::{LiveBrainPresentationFrame, LiveBrainPresentationFrameResource};
#[cfg(feature = "gpu-runtime")]
use crate::bevy_shell::{
    ProductionCuratedFounderResetCommand, ProductionCuratedFounderResetResultResource,
    ProductionGpuBrainAuthorityResource, ProductionGpuBrainRuntimeResource,
    ProductionGpuBrainTickScheduleResource,
};
#[cfg(feature = "gpu-runtime")]
use crate::gpu_live_runtime::CuratedFounderResetRuntimePort;
use crate::terrain_mesh::{build_production_terrain_meshes, TerrainMeshBuild};
use crate::LiveBrainTickSummary;
#[cfg(feature = "gpu-runtime")]
use crate::ProductionConversationLineageUiState;
#[cfg(feature = "gpu-runtime")]
use crate::RuntimePlaybackState;
#[cfg(test)]
use crate::SocketFrame;
use crate::{
    creature_face_style_from_landmarks, creature_part_pose, creature_root_pose,
    grounded_root_height, load_geneforge_assembly_preparation_index,
    load_geneforge_creature_part_catalog, remap_creature_face_landmarks,
    resolve_geneforge_creature_assembly, CreatureAssemblyPartRecipe, CreatureAssemblyRecipe,
    CreatureCoatAssetHandles, CreatureCoatKey, CreaturePartAssetLibrary, CreaturePartLodId,
    CreaturePartSlot, CreatureVisualBounds, GeneForgeAssemblyPreparationIndex,
    GeneForgeCreaturePartCatalog,
};

#[cfg(feature = "gpu-runtime")]
mod phase31_performance_health;
#[cfg(feature = "gpu-runtime")]
mod phase31_slow_frame_ranking;
use crate::{
    creature_visual_snapshot_from_parts_with_appearance,
    production_terrain::{ProductionTerrainSample, ProductionTerrainSampleMap},
    production_voxel_save_with_population, CreatureAnimationState, CreatureExpressionState,
    CreatureVisualSnapshot, Fvr05ProductionDebugAuthorityReport, Fvr05ProductionInspectorTab,
    Fvr05ProductionOverlayKind, Fvr05ProductionUxSettings, Fvr11ProductionTerrainLayer,
    Fvr11ProductionTerrainSceneResource, Fvr11TerrainSurfaceRole, GameAppShellError,
    ProductionFrontendProfileBudget, ProductionFrontendProfileId, ProductionSaveMetadata,
    ProductionVoxelLaunchSummary, FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION,
    PRODUCTION_VOXEL_RENDERER_PROFILE,
};
use crate::{
    terrain_dressing::{
        create_terrain_dressing_library, plan_production_terrain_dressing,
        ProductionTerrainDressingSpawn, TerrainDressingLibrary, TerrainDressingTile,
    },
    terrain_lighting::{
        production_camera_extent, production_camera_transform, production_shadow_cascade_count,
        production_shadow_maximum_distance, spawn_production_terrain_camera,
    },
    terrain_materials::{create_production_terrain_material_library, TerrainMaterialLibrary},
    terrain_water::install_animated_water_material,
};
#[cfg(feature = "gpu-runtime")]
use phase31_performance_health::validate_phase31_performance_authority;
#[cfg(feature = "gpu-runtime")]
use phase31_slow_frame_ranking::{
    retain_ranked_slow_frame, RankedSlowFrame, PHASE31_SLOW_FRAME_THRESHOLD_NS,
};

pub const FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA: &str = "alife.fvr03.production_voxel_renderer.v3";
pub const FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA_VERSION: u16 = 3;
pub const FVR03_PERFORMANCE_ARTIFACT_DIR: &str = "target/artifacts/fvr03";
pub const FVR04_PRODUCTION_CREATURE_RENDERER_SCHEMA: &str =
    "alife.fvr04.production_creature_renderer.v1";
pub const FVR04_PRODUCTION_CREATURE_RENDERER_SCHEMA_VERSION: u16 = 1;
pub const FVR10_RENDERER_BACKEND_ID: &str =
    "fvr10-layered-grid-terrain+modular-heritable-creatures";
pub const FVR09_NATURAL_MATERIAL_PALETTE_VERSION: &str = "fvr09-natural-materials-v1";
pub const FVR09_CUTE_BIPED_VISUAL_PROFILE: &str = "fvr09-cute-biped-v1";
pub const FVR09_CUTE_BIPED_MATERIAL_VERSION: &str = "fvr09-soft-biped-materials-v1";
pub const FVR10_VISIBLE_SURFACE_VARIATION_VERSION: &str = "fvr10-visible-surface-variation-v1";
pub const FVR10_CUTE_BIPED_VISUAL_PROFILE: &str = "modular-heritable-part-assembly-v1";
pub const FVR10_CUTE_BIPED_MATERIAL_VERSION: &str = "modular-textured-part-material-v1";
pub const FVR10_SURFACE_DETAIL_VERSION: &str = "fvr10-screenshot-visible-surface-detail-v2";
/// Dynamic overlay geometry still comes from the scene snapshot. See `docs/STATUS.md`.
pub const FVR05_DYNAMIC_OVERLAYS_TRACK_LIVE_STATE: bool = false;
/// VFX positions follow live creatures, but effect selection is not event-driven yet.
pub const FVR07_VFX_TRIGGERS_TRACK_LIVE_STATE: bool = false;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fvr09MesherMode {
    LayeredGridQuads,
}

impl Fvr09MesherMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::LayeredGridQuads => "layered-grid-quads",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fvr09TerrainMeshStats {
    pub mode: Fvr09MesherMode,
    pub visible_voxels: usize,
    pub naive_visible_faces: usize,
    pub emitted_quads: usize,
    pub face_reduction_ratio: f32,
    pub remesh_time_micros: u128,
    pub dirty_chunks: usize,
    pub cached_chunks: usize,
    pub skipped_chunks: usize,
    pub remesh_budget_chunks_per_frame: usize,
    pub material_palette_version: &'static str,
    pub vertex_color_face_variation: bool,
    pub top_side_color_separation: bool,
    pub variation_bucket_count: usize,
    pub cache_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fvr03ProductionVoxelCameraMode {
    OrthographicIsometric,
    Orbit,
}

impl Fvr03ProductionVoxelCameraMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::OrthographicIsometric => "orthographic-isometric",
            Self::Orbit => "orbit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fvr03ProductionVoxelMaterialKind {
    SafeGrass,
    Soil,
    Resource,
    Hazard,
    Decay,
    Stone,
    Water,
    Sand,
    Creature,
    Selection,
}

impl Fvr03ProductionVoxelMaterialKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SafeGrass => "safe-grass",
            Self::Soil => "soil",
            Self::Resource => "resource",
            Self::Hazard => "hazard",
            Self::Decay => "decay",
            Self::Stone => "stone",
            Self::Water => "water",
            Self::Sand => "sand",
            Self::Creature => "creature",
            Self::Selection => "selection",
        }
    }

    pub const fn is_terrain_surface(self) -> bool {
        matches!(
            self,
            Self::SafeGrass
                | Self::Soil
                | Self::Resource
                | Self::Hazard
                | Self::Decay
                | Self::Stone
                | Self::Water
                | Self::Sand
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fvr03ProductionVoxelMaterialEntry {
    pub kind: Fvr03ProductionVoxelMaterialKind,
    pub label: &'static str,
    pub rgba: [f32; 4],
    pub roughness: f32,
    pub top_texture: &'static str,
    pub side_texture: &'static str,
    pub natural_variation_seed: &'static str,
    pub debug_primary_color: bool,
}

impl Fvr03ProductionVoxelMaterialEntry {
    fn standard_material(self) -> StandardMaterial {
        let base_color = if self.kind.is_terrain_surface() {
            Color::srgba(1.0, 1.0, 1.0, self.rgba[3])
        } else {
            Color::srgba(self.rgba[0], self.rgba[1], self.rgba[2], self.rgba[3])
        };
        StandardMaterial {
            base_color,
            perceptual_roughness: self.roughness,
            metallic: 0.0,
            cull_mode: None,
            alpha_mode: if self.rgba[3] < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
            unlit: self.kind == Fvr03ProductionVoxelMaterialKind::Selection,
            ..default()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fvr03ProductionVoxelRendererSettings {
    pub profile_id: ProductionFrontendProfileId,
    pub target_fps: u16,
    pub max_population: u16,
    pub draw_radius_chunks: u16,
    pub hot_radius_chunks: u16,
    pub resident_chunk_budget: u16,
    pub tile_stride: u16,
    pub estimated_tile_budget: usize,
    pub internal_render_scale: f32,
    pub shadow_quality: &'static str,
    pub label_density: &'static str,
    pub production_dressing_cap: usize,
    pub production_vfx_marker_cap: usize,
    pub production_vfx_budget_state: &'static str,
    pub minimum_floor: bool,
    pub min_spec_comfort_default: bool,
    pub research_scale: bool,
    pub default_camera_modes: Vec<Fvr03ProductionVoxelCameraMode>,
    pub material_palette_version: &'static str,
    pub debug_primary_colors: bool,
    pub remesh_budget_chunks_per_frame: usize,
}

impl Fvr03ProductionVoxelRendererSettings {
    pub fn for_profile(profile_id: ProductionFrontendProfileId) -> Self {
        let budget = profile_id.budget();
        let draw_radius_chunks = budget.chunk_activation_radius;
        let resident_chunk_budget = budget.active_chunk_cap;
        let tile_stride = match profile_id {
            ProductionFrontendProfileId::MinimumSettings30x30 => 2,
            ProductionFrontendProfileId::MinSpecComfort1080p => 2,
            ProductionFrontendProfileId::Balanced1080p => 2,
            ProductionFrontendProfileId::HighSpecScaleUp => 2,
            ProductionFrontendProfileId::ResearchScale => 4,
        };
        let diameter = usize::from(draw_radius_chunks) * 2 + 1;
        let visible_window_chunks = diameter
            .saturating_mul(diameter)
            .min(usize::from(resident_chunk_budget));
        let sampled_tiles_per_chunk = usize::from(budget.chunk_tile_size)
            .div_ceil(usize::from(tile_stride))
            .pow(2);
        let (production_dressing_cap, production_vfx_marker_cap) = match profile_id {
            ProductionFrontendProfileId::MinimumSettings30x30 => (64, 32),
            ProductionFrontendProfileId::MinSpecComfort1080p => (224, 96),
            ProductionFrontendProfileId::Balanced1080p => (288, 192),
            ProductionFrontendProfileId::HighSpecScaleUp => (384, 320),
            ProductionFrontendProfileId::ResearchScale => (160, 128),
        };
        let remesh_budget_chunks_per_frame = match profile_id {
            ProductionFrontendProfileId::MinimumSettings30x30 => 4,
            ProductionFrontendProfileId::MinSpecComfort1080p => 8,
            ProductionFrontendProfileId::Balanced1080p => 12,
            ProductionFrontendProfileId::HighSpecScaleUp => 24,
            ProductionFrontendProfileId::ResearchScale => 8,
        };
        Self {
            profile_id,
            target_fps: budget.target_fps,
            max_population: budget.maximum_profile_population,
            draw_radius_chunks,
            hot_radius_chunks: draw_radius_chunks.clamp(1, 3),
            resident_chunk_budget,
            tile_stride,
            estimated_tile_budget: visible_window_chunks.saturating_mul(sampled_tiles_per_chunk),
            internal_render_scale: budget.default_internal_render_scale,
            shadow_quality: budget.shadow_quality,
            label_density: budget.label_density,
            production_dressing_cap,
            production_vfx_marker_cap,
            production_vfx_budget_state: budget.vfx_budget,
            minimum_floor: budget.hard_floor,
            min_spec_comfort_default: budget.comfort_default,
            research_scale: budget.research_mode,
            default_camera_modes: vec![
                Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
                Fvr03ProductionVoxelCameraMode::Orbit,
            ],
            material_palette_version: FVR10_VISIBLE_SURFACE_VARIATION_VERSION,
            debug_primary_colors: false,
            remesh_budget_chunks_per_frame,
        }
    }

    pub fn material_palette(&self) -> Vec<Fvr03ProductionVoxelMaterialEntry> {
        vec![
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::SafeGrass,
                label: "safe-grass",
                rgba: [0.18, 0.38, 0.20, 1.0],
                roughness: 0.92,
                top_texture: "grass-moss-top",
                side_texture: "dirt-rooted-side",
                natural_variation_seed: "fvr10-grass-moss-temperate-vertex-face",
                debug_primary_color: false,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Soil,
                label: "soil",
                rgba: [0.33, 0.25, 0.17, 1.0],
                roughness: 0.96,
                top_texture: "soil-loam-variegated",
                side_texture: "soil-clay-side",
                natural_variation_seed: "fvr10-soil-loam-warm-vertex-face",
                debug_primary_color: false,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Resource,
                label: "resource",
                rgba: [0.22, 0.46, 0.29, 1.0],
                roughness: 0.74,
                top_texture: "clover-food-leaf",
                side_texture: "herb-root-side",
                natural_variation_seed: "fvr10-resource-clover-vertex-face",
                debug_primary_color: false,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Hazard,
                label: "hazard",
                rgba: [0.38, 0.16, 0.18, 1.0],
                roughness: 0.72,
                top_texture: "thorn-fungal-warning-top",
                side_texture: "thorn-dirt-side",
                natural_variation_seed: "fvr10-hazard-thorn-fungal-vertex-face",
                debug_primary_color: false,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Decay,
                label: "decay",
                rgba: [0.21, 0.16, 0.13, 1.0],
                roughness: 0.88,
                top_texture: "leaf-rot-duff-top",
                side_texture: "dark-humus-side",
                natural_variation_seed: "fvr10-decay-leaf-rot-vertex-face",
                debug_primary_color: false,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Stone,
                label: "stone",
                rgba: [0.40, 0.42, 0.38, 1.0],
                roughness: 0.98,
                top_texture: "lichen-rock-top",
                side_texture: "fractured-stone-side",
                natural_variation_seed: "fvr10-stone-lichen-vertex-face",
                debug_primary_color: false,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Water,
                label: "water",
                rgba: [0.12, 0.25, 0.32, 0.82],
                roughness: 0.34,
                top_texture: "wet-reed-water-top",
                side_texture: "dark-wet-bank-side",
                natural_variation_seed: "fvr10-water-reed-bank-vertex-face",
                debug_primary_color: false,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Sand,
                label: "sand",
                rgba: [0.53, 0.47, 0.31, 1.0],
                roughness: 0.90,
                top_texture: "dry-sand-ripple-top",
                side_texture: "dry-soil-side",
                natural_variation_seed: "fvr10-sand-dry-soil-vertex-face",
                debug_primary_color: false,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Creature,
                label: "creature",
                rgba: [0.74, 0.62, 0.42, 1.0],
                roughness: 0.66,
                top_texture: "soft-biped-fur-top",
                side_texture: "soft-biped-fur-side",
                natural_variation_seed: "fvr10-creature-soft-biped",
                debug_primary_color: false,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Selection,
                label: "selection",
                rgba: [1.0, 0.86, 0.18, 0.90],
                roughness: 0.48,
                top_texture: "selection-hover-ring",
                side_texture: "selection-hover-edge",
                natural_variation_seed: "fvr10-selection-hover",
                debug_primary_color: false,
            },
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fvr04CreatureLod {
    FullVoxel,
    CompactVoxel,
    ImpostorVoxel,
}

impl Fvr04CreatureLod {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FullVoxel => "full-voxel",
            Self::CompactVoxel => "compact-voxel",
            Self::ImpostorVoxel => "impostor-voxel",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fvr04ProductionCreatureRendererSettings {
    pub profile_id: ProductionFrontendProfileId,
    pub requested_population: u16,
    pub max_visible_creatures: u16,
    pub lod: Fvr04CreatureLod,
    pub selected_hover_label_only: bool,
    pub max_world_labels: u16,
    pub shared_mesh_material_buckets: bool,
    pub expression_buffer_channels: u8,
}

impl Fvr04ProductionCreatureRendererSettings {
    pub fn for_profile(profile_id: ProductionFrontendProfileId, requested_population: u16) -> Self {
        let budget = profile_id.budget();
        let lod = match (profile_id, requested_population) {
            (ProductionFrontendProfileId::MinimumSettings30x30, _) => {
                Fvr04CreatureLod::CompactVoxel
            }
            (ProductionFrontendProfileId::MinSpecComfort1080p, _) => Fvr04CreatureLod::CompactVoxel,
            (_, population) if population >= 250 => Fvr04CreatureLod::ImpostorVoxel,
            (_, population) if population >= 100 => Fvr04CreatureLod::CompactVoxel,
            _ => Fvr04CreatureLod::FullVoxel,
        };
        let max_world_labels = match profile_id {
            ProductionFrontendProfileId::MinimumSettings30x30 => 2,
            ProductionFrontendProfileId::MinSpecComfort1080p => 4,
            ProductionFrontendProfileId::Balanced1080p => 8,
            ProductionFrontendProfileId::HighSpecScaleUp => 12,
            ProductionFrontendProfileId::ResearchScale => 4,
        };
        Self {
            profile_id,
            requested_population,
            max_visible_creatures: requested_population.min(budget.maximum_profile_population),
            lod,
            selected_hover_label_only: matches!(
                profile_id,
                ProductionFrontendProfileId::MinimumSettings30x30
                    | ProductionFrontendProfileId::MinSpecComfort1080p
                    | ProductionFrontendProfileId::ResearchScale
            ),
            max_world_labels,
            shared_mesh_material_buckets: true,
            expression_buffer_channels: 8,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fvr04CreatureRenderBucket {
    pub expression: CreatureExpressionState,
    pub animation: CreatureAnimationState,
    pub lod: Fvr04CreatureLod,
    pub species_archetype: u8,
    pub palette_family: u8,
    pub fur_pattern: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fvr04CreatureExpressionSample {
    pub stable_id: alife_core::WorldEntityId,
    pub organism_id: alife_core::OrganismId,
    pub display_label: String,
    pub brain_class_id: Option<u16>,
    pub brain_neuron_count: Option<u32>,
    pub hunger: f32,
    pub fatigue: f32,
    pub fear: f32,
    pub cortisol: f32,
    pub dopamine: f32,
    pub reproductive_drive: f32,
    pub sleep_pressure: f32,
    pub social: f32,
    pub fast_memory_count: Option<u32>,
    pub lifetime_memory_count: Option<u32>,
    pub memory_record_count: Option<u32>,
    pub concept_count: Option<u32>,
    pub unresolved_gap_count: Option<u32>,
    pub lifetime_learning_enabled: Option<bool>,
    pub sleep_phase_raw: Option<u16>,
    pub consolidation_state_raw: Option<u16>,
    pub last_consolidated_tick: Option<u64>,
    pub topology_update_count: Option<u32>,
    pub expression: CreatureExpressionState,
    pub animation: CreatureAnimationState,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Fvr04ProductionCreatureSceneResource {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_population: u16,
    pub rendered_creature_count: usize,
    pub expression_buffer: Vec<Fvr04CreatureExpressionSample>,
    pub material_bucket_count: usize,
    pub mesh_pool_count: usize,
    pub lod: Fvr04CreatureLod,
    pub stable_lookup_by_raw_id: BTreeMap<u64, usize>,
    pub no_renderer_authority_over_actions_or_cognition: bool,
    pub expression_buffer_is_read_only_projection: bool,
    pub visual_profile: &'static str,
    pub mesh_material_version: &'static str,
    pub species_archetype_count: usize,
    pub creature_root_count: usize,
    pub creature_part_entity_count: usize,
    pub creature_join_cover_count: usize,
    pub creature_part_family_count: usize,
    pub creature_mixed_assembly_count: usize,
    pub creature_shared_mesh_handle_count: usize,
    pub production_visuals_display_only: bool,
}

impl Fvr04ProductionCreatureSceneResource {
    pub fn sample_for_stable_id(
        &self,
        stable_id: alife_core::WorldEntityId,
    ) -> Option<&Fvr04CreatureExpressionSample> {
        self.stable_lookup_by_raw_id
            .get(&stable_id.raw())
            .and_then(|index| self.expression_buffer.get(*index))
    }

    pub fn panel_text(&self, selection: Option<StableVoxelObjectRef>) -> String {
        let Some(selection) = selection else {
            return "Creature\nselection: none".to_string();
        };
        if selection.kind != StableVoxelRefKind::Creature {
            return "Creature\nselection: terrain".to_string();
        }
        let Some(stable_id) = selection.stable_id else {
            return "Creature\nselection: missing stable id".to_string();
        };
        let Some(sample) = self.sample_for_stable_id(stable_id) else {
            return format!("Creature\nstable: {}\nstate: unavailable", stable_id.raw());
        };
        format!(
            "Creature {}\norg: {}\nPRESENTATION METADATA (launch/save)\nexpression/body: {} / {}\nhunger {:.2} fatigue {:.2} fear {:.2}\ndopamine {:.2} cortisol {:.2} repro {:.2}\nsleep {:.2} social {:.2}",
            sample.stable_id.raw(),
            sample.organism_id.raw(),
            sample.expression.label(),
            sample.animation.label(),
            sample.hunger,
            sample.fatigue,
            sample.fear,
            sample.dopamine,
            sample.cortisol,
            sample.reproductive_drive,
            sample.sleep_pressure,
            sample.social,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Fvr03ProductionVoxelSceneResource {
    pub schema: &'static str,
    pub schema_version: u16,
    pub snapshot_schema: String,
    pub profile_id: ProductionFrontendProfileId,
    pub population: u16,
    pub renderer_profile: String,
    pub backend_id: &'static str,
    pub uses_internal_voxel_terrain_mesh: bool,
    pub visible_chunk_count: usize,
    pub resident_chunk_count: usize,
    pub tile_mesh_count: usize,
    pub creature_render_count: usize,
    pub creature_material_bucket_count: usize,
    pub creature_lod: Fvr04CreatureLod,
    pub creature_root_count: usize,
    pub creature_part_entity_count: usize,
    pub creature_join_cover_count: usize,
    pub creature_part_family_count: usize,
    pub creature_mixed_assembly_count: usize,
    pub creature_shared_mesh_handle_count: usize,
    pub selection_ref_count: usize,
    pub dirty_chunk_count: usize,
    pub estimated_resident_bytes: usize,
    pub draw_radius_chunks: u16,
    pub target_fps: u16,
    pub performance_artifact_path: Option<PathBuf>,
    pub no_renderer_authority_over_world_truth: bool,
    pub material_counts: BTreeMap<Fvr03ProductionVoxelMaterialKind, usize>,
    pub average_resource_bias: f32,
    pub average_hazard_pressure: f32,
    pub production_dressing_count: usize,
    pub production_vfx_marker_count: usize,
    pub production_gpu_vfx_emitter_count: usize,
    pub production_vfx_budget_state: &'static str,
    pub production_visuals_display_only: bool,
    pub production_vfx_uses_hanabi_gpu_particles: bool,
    pub mesh_stats: Fvr09TerrainMeshStats,
    visible_tiles: BTreeSet<VoxelTileCoord>,
    visible_chunks: BTreeSet<VoxelChunkCoord>,
    tile_summaries_by_tile: BTreeMap<VoxelTileCoord, Fvr05ProductionTileSummary>,
    creature_refs_by_tile: BTreeMap<VoxelTileCoord, StableVoxelObjectRef>,
    selection_positions_by_raw_id: BTreeMap<u64, Vec3>,
}

impl Fvr03ProductionVoxelSceneResource {
    pub fn selection_position(&self, stable_id: alife_core::WorldEntityId) -> Option<Vec3> {
        self.selection_positions_by_raw_id
            .get(&stable_id.raw())
            .copied()
    }

    pub fn contains_tile(&self, tile: VoxelTileCoord) -> bool {
        self.visible_tiles.contains(&tile)
    }

    pub fn contains_chunk(&self, chunk: VoxelChunkCoord) -> bool {
        self.visible_chunks.contains(&chunk)
    }

    pub fn selection_label(&self, selection: &StableVoxelObjectRef) -> String {
        let tile = selection
            .tile
            .map(|tile| format!("tile x={} z={}", tile.x, tile.z))
            .unwrap_or_else(|| "tile none".to_string());
        format!(
            "stable {} chunk x={} z={} {}",
            match selection.kind {
                StableVoxelRefKind::Chunk => "chunk",
                StableVoxelRefKind::Tile => "tile",
                StableVoxelRefKind::Creature => "creature",
                StableVoxelRefKind::Resource => "resource",
                StableVoxelRefKind::Hazard => "hazard",
            },
            selection.chunk.x,
            selection.chunk.z,
            tile
        )
    }

    fn tile_from_world_position(&self, world_position: Vec3) -> Option<VoxelTileCoord> {
        let tile = VoxelTileCoord::new(
            world_position.x.floor() as i32,
            world_position.z.floor() as i32,
        );
        self.contains_tile(tile).then_some(tile)
    }

    fn selectable_ref_at_tile(&self, tile: VoxelTileCoord) -> StableVoxelObjectRef {
        self.creature_refs_by_tile
            .get(&tile)
            .copied()
            .unwrap_or(StableVoxelObjectRef {
                kind: StableVoxelRefKind::Tile,
                stable_id: None,
                chunk: VoxelChunkCoord::for_tile(16, tile),
                tile: Some(tile),
            })
    }

    fn world_position_for_selection(&self, selection: StableVoxelObjectRef) -> Option<Vec3> {
        if let Some(stable_id) = selection.stable_id {
            if let Some(position) = self.selection_positions_by_raw_id.get(&stable_id.raw()) {
                return Some(*position);
            }
        }
        selection
            .tile
            .map(|tile| Vec3::new(tile.x as f32 + 0.5, 1.46, tile.z as f32 + 0.5))
    }

    fn tile_summary_for_selection(
        &self,
        selection: Option<StableVoxelObjectRef>,
    ) -> Option<&Fvr05ProductionTileSummary> {
        let tile = selection.and_then(|selection| selection.tile)?;
        self.tile_summaries_by_tile.get(&tile)
    }

    fn stable_sim_signature(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema,
            self.schema_version,
            self.profile_id.label(),
            self.population,
            self.visible_chunk_count,
            self.tile_mesh_count,
            self.creature_render_count,
            self.selection_ref_count,
            self.dirty_chunk_count
        )
    }

    fn tile_panel_text(&self, selection: Option<StableVoxelObjectRef>) -> String {
        let Some(tile) = self.tile_summary_for_selection(selection) else {
            return "Tile\nselection: none".to_string();
        };
        format!(
            "Tile\nx={} z={} | chunk {}:{}\nmaterial: {}\nheight {:.2}\nresource {:.2} | hazard {:.2}\nstable ref: {}",
            tile.tile.x,
            tile.tile.z,
            tile.chunk.x,
            tile.chunk.z,
            tile.material.label(),
            tile.height_units,
            tile.resource_bias,
            tile.hazard_pressure,
            self.selection_label(&tile.stable_ref)
        )
    }

    fn world_panel_text(&self) -> String {
        let material_line = self
            .material_counts
            .iter()
            .map(|(kind, count)| format!("{}={}", kind.label(), count))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "World / Ecology\nchunks visible {} resident {} dirty {}\ntiles sampled {} | creatures {}\nmesher {} | quads {} | face reduction {:.2}x | remesh {}/frame dirty {} cached {} skipped {}\nmaterial atlas {}\nresource avg {:.2} | hazard avg {:.2}\nmaterials {}\nproduction polish: dressing {} vfx {} gpu_emitters {} budget {} display_only {}\ncore authority: world/action legality only",
            self.visible_chunk_count,
            self.resident_chunk_count,
            self.dirty_chunk_count,
            self.tile_mesh_count,
            self.creature_render_count,
            self.mesh_stats.mode.label(),
            self.mesh_stats.emitted_quads,
            self.mesh_stats.face_reduction_ratio,
            self.mesh_stats.remesh_budget_chunks_per_frame,
            self.mesh_stats.dirty_chunks,
            self.mesh_stats.cached_chunks,
            self.mesh_stats.skipped_chunks,
            self.mesh_stats.material_palette_version,
            self.average_resource_bias,
            self.average_hazard_pressure,
            material_line,
            self.production_dressing_count,
            self.production_vfx_marker_count,
            self.production_gpu_vfx_emitter_count,
            self.production_vfx_budget_state,
            self.production_visuals_display_only
        )
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Fvr03ProductionVoxelSelectionResource {
    pub hovered: Option<StableVoxelObjectRef>,
    pub selected: Option<StableVoxelObjectRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct Fvr04ProductionCreatureFollowResource {
    pub enabled: bool,
    pub target_stable_id: Option<alife_core::WorldEntityId>,
}

#[derive(Debug, Clone, Resource)]
pub struct Fvr03ProductionVoxelScreenshotResource {
    pub frame: u32,
    pub capture_after_frame: u32,
    pub measurement_sample_frames: u32,
    pub measurement_start_frame: u32,
    pub measurement_started_at: Option<Instant>,
    pub measurement_written: bool,
    pub requested: bool,
    pub path: PathBuf,
    pub product_screenshot_captured: bool,
    pub fvr05_capture_index: usize,
    pub fvr05_next_capture_frame: u32,
    pub fvr05_sequence_complete: bool,
    pub developer_overlay: bool,
}

const PHASE31_PERFORMANCE_SCHEMA: &str = "alife.phase31.performance-baseline.v4";
const PHASE31_PERFORMANCE_SCHEMA_VERSION: u16 = 4;
const PHASE31_WARMUP_DURATION: Duration = Duration::from_secs(5);
const PHASE31_MEASUREMENT_DURATION: Duration = Duration::from_secs(60);
const PHASE31_PERSISTENCE_DRAIN_TIMEOUT: Duration = Duration::from_secs(20);
const PHASE31_PERFORMANCE_ARTIFACT_DIR: &str = "target/artifacts/phase31-performance";

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
struct Phase31FrameUpdateCpu {
    input_ns: u64,
    live_gpu_tick_ns: u64,
    authoritative_projection_ns: u64,
    procedural_animation_ns: u64,
    ui_root_readers_ns: u64,
}

#[cfg(feature = "gpu-runtime")]
impl Phase31FrameUpdateCpu {
    fn total_ns(self) -> u64 {
        self.input_ns
            .saturating_add(self.live_gpu_tick_ns)
            .saturating_add(self.authoritative_projection_ns)
            .saturating_add(self.procedural_animation_ns)
            .saturating_add(self.ui_root_readers_ns)
    }
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Clone, Copy, Default)]
struct Phase31FrameSnapshot {
    runtime: crate::gpu_live_runtime::GpuLivePerformanceMetrics,
    scheduler: crate::bevy_shell::ProductionGpuTickPerformanceCounters,
    checkpoint: crate::gpu_live_runtime::ExactCheckpointPerformanceState,
    world_tick: u64,
    world_objects: u64,
    organisms: u64,
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Clone, Default, serde::Serialize)]
struct Phase31SlowFrameSample {
    frame_index: u64,
    frame_duration_ns: u64,
    world_tick_before: u64,
    world_tick_after: u64,
    world_ticks_completed: u64,
    world_objects_before: u64,
    world_objects_after: u64,
    organisms_before: u64,
    organisms_after: u64,
    checkpoint_before: crate::gpu_live_runtime::ExactCheckpointPerformanceState,
    checkpoint_after: crate::gpu_live_runtime::ExactCheckpointPerformanceState,
    scheduler_attempts: u64,
    scheduler_completed_ticks: u64,
    checkpoint_publication_waits: u64,
    checkpoint_failed_waits: u64,
    deferred_catch_up_ticks: u64,
    catch_up_ticks_dropped: u64,
    scheduler_debt_micros_before: u64,
    scheduler_debt_micros_after: u64,
    update_cpu: Phase31FrameUpdateCpu,
    renderer_present_and_uninstrumented_residual_ns: u64,
    runtime: crate::gpu_live_runtime::GpuLivePerformanceMetrics,
}

#[cfg(feature = "gpu-runtime")]
impl RankedSlowFrame for Phase31SlowFrameSample {
    fn frame_duration_ns(&self) -> u64 {
        self.frame_duration_ns
    }

    fn frame_index(&self) -> u64 {
        self.frame_index
    }
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Resource)]
pub(crate) struct Phase31PerformanceMetricsResource {
    profile: String,
    population: u16,
    resolution: [u32; 2],
    backend: String,
    adapter: String,
    launched_at: Instant,
    last_frame_at: Instant,
    measurement_started_at: Option<Instant>,
    measurement_completed_at: Option<Instant>,
    measurement_start_world_tick: Option<u64>,
    runtime_baseline: Option<crate::gpu_live_runtime::GpuLivePerformanceMetrics>,
    scheduler_baseline: Option<crate::bevy_shell::ProductionGpuTickPerformanceCounters>,
    stage_mark: Option<Instant>,
    frame_snapshot: Option<Phase31FrameSnapshot>,
    current_frame_update_cpu: Phase31FrameUpdateCpu,
    frame_ns: Vec<u64>,
    slow_frame_count: u64,
    slow_frames: Vec<Phase31SlowFrameSample>,
    input_cpu_ns: u64,
    live_gpu_tick_cpu_ns: u64,
    authoritative_projection_cpu_ns: u64,
    procedural_animation_cpu_ns: u64,
    ui_root_readers_cpu_ns: u64,
    ui_updates: u64,
    gpu_samples: Vec<alife_gpu_backend::GpuNeuralTimingSample>,
    artifact_path: Option<PathBuf>,
    write_error: Option<String>,
}

#[cfg(feature = "gpu-runtime")]
impl Phase31PerformanceMetricsResource {
    fn new(summary: &ProductionVoxelLaunchSummary) -> Self {
        let now = Instant::now();
        Self {
            profile: summary.profile_id.label().to_string(),
            population: summary.effective_population,
            resolution: [summary.resolution.0, summary.resolution.1],
            backend: summary.diagnostics.selected_backend.clone(),
            adapter: summary
                .diagnostics
                .adapter_name
                .clone()
                .unwrap_or_else(|| "unavailable".to_string()),
            launched_at: now,
            last_frame_at: now,
            measurement_started_at: None,
            measurement_completed_at: None,
            measurement_start_world_tick: None,
            runtime_baseline: None,
            scheduler_baseline: None,
            stage_mark: None,
            frame_snapshot: None,
            current_frame_update_cpu: Phase31FrameUpdateCpu::default(),
            frame_ns: Vec::new(),
            slow_frame_count: 0,
            slow_frames: Vec::new(),
            input_cpu_ns: 0,
            live_gpu_tick_cpu_ns: 0,
            authoritative_projection_cpu_ns: 0,
            procedural_animation_cpu_ns: 0,
            ui_root_readers_cpu_ns: 0,
            ui_updates: 0,
            gpu_samples: Vec::new(),
            artifact_path: None,
            write_error: None,
        }
    }

    pub(crate) fn measuring(&self) -> bool {
        self.measurement_started_at.is_some_and(|started| {
            started.elapsed() < PHASE31_MEASUREMENT_DURATION
                && self.artifact_path.is_none()
                && self.write_error.is_none()
        })
    }

    pub(crate) fn draining(&self) -> bool {
        self.measurement_started_at.is_some_and(|started| {
            started.elapsed() >= PHASE31_MEASUREMENT_DURATION
                && self.artifact_path.is_none()
                && self.write_error.is_none()
        })
    }

    fn take_stage_elapsed_ns(&mut self) -> u64 {
        let now = Instant::now();
        self.stage_mark.replace(now).map_or(0, |started| {
            u64::try_from(now.duration_since(started).as_nanos()).unwrap_or(u64::MAX)
        })
    }

    pub(crate) fn record_gpu_sample(&mut self, sample: alife_gpu_backend::GpuNeuralTimingSample) {
        if self.measuring() {
            self.gpu_samples.push(sample);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr03ProductionVoxelCamera {
    pub mode: Fvr03ProductionVoxelCameraMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr03ProductionVoxelCreatureMarker {
    pub stable_id: alife_core::WorldEntityId,
    pub tile: VoxelTileCoord,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr04ProductionCreatureVisualMarker {
    pub stable_id: alife_core::WorldEntityId,
    pub organism_id: alife_core::OrganismId,
    pub tile: VoxelTileCoord,
    pub expression: CreatureExpressionState,
    pub animation: CreatureAnimationState,
    pub lod: Fvr04CreatureLod,
    pub base_translation: Vec3,
    pub local_offset: Vec3,
    pub base_scale: Vec3,
    pub local_bounds: CreatureVisualBounds,
    pub surface_height: f32,
    pub phase: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct ProductionCreatureAssemblyRoot {
    pub stable_id: alife_core::WorldEntityId,
    pub organism_id: OrganismId,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
struct Fvr04ProductionRuntimeSceneRoot;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ProductionVoxelPresentationSet {
    Input,
    LiveGpuTick,
    AuthoritativeProjection,
    ProceduralAnimation,
    RootReaders,
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct ProductionCreaturePartMarker {
    pub stable_id: alife_core::WorldEntityId,
    pub family: alife_world::CreaturePartFamilyId,
    pub asset_id: crate::CreaturePartAssetId,
    pub slot: CreaturePartSlot,
    pub runtime_group: String,
    pub authored_matrix: [f64; 16],
    pub animation: CreatureAnimationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
struct ProductionCreaturePartRestTransform(Transform);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct ProductionCreatureJoinCoverMarker {
    pub stable_id: alife_core::WorldEntityId,
    pub cover_kind: &'static str,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr09CuteBipedCreatureMarker {
    pub stable_id: alife_core::WorldEntityId,
    pub visual_profile: &'static str,
    pub two_legs: bool,
    pub visible_face: bool,
    pub eye_markers: u8,
    pub front_back_orientation: bool,
    pub real_state_driven: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr09CreatureFaceFeatureMarker {
    pub stable_id: alife_core::WorldEntityId,
    pub feature: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr10CreatureSurfaceDetailMarker {
    pub stable_id: alife_core::WorldEntityId,
    pub species_archetype: u8,
    pub detail_role: &'static str,
    pub anchor_slot: CreaturePartSlot,
    pub display_only: bool,
    pub no_renderer_authority_over_actions_or_cognition: bool,
    pub high_contrast_marking: bool,
    pub heritable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr10CreatureSpeciesMarker {
    pub stable_id: alife_core::WorldEntityId,
    pub species_archetype: u8,
    pub species_label: &'static str,
    pub body_plan_signature: &'static str,
    pub bipedal: bool,
    pub caveman_furry_design: bool,
    pub heritable_appearance: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr03ProductionVoxelSelectionMarker;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr04ProductionCreatureWorldLabel;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr04ProductionCreatureAffordanceCue {
    pub stable_id: alife_core::WorldEntityId,
    pub expression: CreatureExpressionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr03ProductionVoxelTerrainBatch {
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub tile_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr05ProductionOverlayBatch {
    pub kind: Fvr05ProductionOverlayKind,
    pub cell_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fvr07ProductionDressingKind {
    LeafPatch,
    MushroomCluster,
    PebbleCluster,
    NestMarker,
    FoodResource,
    CorpseMarker,
    FlowerPatch,
    ReedCluster,
    LichenRock,
    HazardFungus,
    DeadLeafPatch,
    AlienFern,
    CrimsonSpire,
    GlowBulbCluster,
}

impl Fvr07ProductionDressingKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::LeafPatch => "leaf-patch",
            Self::MushroomCluster => "mushroom-cluster",
            Self::PebbleCluster => "pebble-cluster",
            Self::NestMarker => "nest-marker",
            Self::FoodResource => "food-resource",
            Self::CorpseMarker => "corpse-marker",
            Self::FlowerPatch => "flower-patch",
            Self::ReedCluster => "reed-cluster",
            Self::LichenRock => "lichen-rock",
            Self::HazardFungus => "hazard-fungus",
            Self::DeadLeafPatch => "dead-leaf-patch",
            Self::AlienFern => "alien-fern",
            Self::CrimsonSpire => "crimson-spire",
            Self::GlowBulbCluster => "glow-bulb-cluster",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fvr07ProductionVfxKind {
    PheromoneTrail,
    SporeDrift,
    SleepGlow,
    DangerHazardParticles,
    EatingResourceEffect,
    BirthDeathEffect,
    WaterDecayAmbient,
    SelectedCreatureNeuralPulse,
}

impl Fvr07ProductionVfxKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::PheromoneTrail => "pheromone-trail",
            Self::SporeDrift => "spore-drift",
            Self::SleepGlow => "sleep-glow",
            Self::DangerHazardParticles => "danger-hazard-particles",
            Self::EatingResourceEffect => "eating-resource-effect",
            Self::BirthDeathEffect => "birth-death-effect",
            Self::WaterDecayAmbient => "water-decay-ambient",
            Self::SelectedCreatureNeuralPulse => "selected-creature-neural-pulse",
        }
    }

    #[cfg(not(feature = "vfx-hanabi"))]
    const fn pulse_speed(self) -> f32 {
        match self {
            Self::PheromoneTrail => 1.4,
            Self::SporeDrift => 0.9,
            Self::SleepGlow => 0.55,
            Self::DangerHazardParticles => 3.4,
            Self::EatingResourceEffect => 2.2,
            Self::BirthDeathEffect => 1.8,
            Self::WaterDecayAmbient => 0.8,
            Self::SelectedCreatureNeuralPulse => 2.7,
        }
    }

    #[cfg(not(feature = "vfx-hanabi"))]
    const fn bob_height(self) -> f32 {
        match self {
            Self::DangerHazardParticles => 0.16,
            Self::SelectedCreatureNeuralPulse => 0.11,
            Self::SleepGlow => 0.05,
            _ => 0.08,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr07ProductionVisualDressing {
    pub kind: Fvr07ProductionDressingKind,
    pub tile: VoxelTileCoord,
    pub display_only: bool,
    pub no_renderer_authority_over_actions_or_cognition: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr07ProductionGpuVfxMarker {
    pub kind: Fvr07ProductionVfxKind,
    pub tile: Option<VoxelTileCoord>,
    pub stable_id: Option<alife_core::WorldEntityId>,
    pub follows_creature: bool,
    pub display_only: bool,
    pub no_renderer_authority_over_actions_or_cognition: bool,
    pub budget_state: &'static str,
    pub base_translation: Vec3,
    pub base_scale: Vec3,
    pub phase: f32,
}

#[cfg(feature = "vfx-hanabi")]
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr07ProductionHanabiVfxEmitter {
    pub kind: Fvr07ProductionVfxKind,
    pub stable_id: Option<alife_core::WorldEntityId>,
    pub follows_creature: bool,
    pub display_only: bool,
    pub no_renderer_authority_over_actions_or_cognition: bool,
    pub budget_state: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr05ProductionTopRuntimeBar;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr05ProductionLeftControlPanel;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr05ProductionRightInspectorPanel;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr05ProductionBottomOverlayToolbar;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr05ProductionFooterStatusBar;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct V0PlayerStatusChip;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct V0PlayerCreaturePanel;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct V0PlayerControlStrip;

struct Fvr11TerrainSpawnReceipt {
    mesh_stats: Fvr09TerrainMeshStats,
    top_layer_count: usize,
    cliff_layer_count: usize,
    transition_edge_count: usize,
    water_layer_count: usize,
    confetti_detail_quad_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fvr05ProductionTileSummary {
    pub tile: VoxelTileCoord,
    pub chunk: VoxelChunkCoord,
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub height_units: f32,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
    pub stable_ref: StableVoxelObjectRef,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Fvr05OverlayCell {
    center_x: f32,
    center_z: f32,
    y: f32,
    footprint: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct Fvr04CreatureVisualRecord {
    stable_ref: StableVoxelObjectRef,
    tile: VoxelTileCoord,
    display_label: String,
    brain_class_id: Option<u16>,
    brain_neuron_count: Option<u32>,
    social_affinity: f32,
    reproductive_drive: f32,
    fast_memory_count: Option<u32>,
    lifetime_memory_count: Option<u32>,
    memory_record_count: Option<u32>,
    concept_count: Option<u32>,
    unresolved_gap_count: Option<u32>,
    lifetime_learning_enabled: Option<bool>,
    sleep_phase_raw: Option<u16>,
    consolidation_state_raw: Option<u16>,
    last_consolidated_tick: Option<u64>,
    topology_update_count: Option<u32>,
    visual: CreatureVisualSnapshot,
}

#[derive(Debug, Resource)]
struct Fvr04CreatureSpawnContext {
    settings: Fvr04ProductionCreatureRendererSettings,
    catalog: GeneForgeCreaturePartCatalog,
    preparations: GeneForgeAssemblyPreparationIndex,
    assets_root: PathBuf,
    creature_part_assets: CreaturePartAssetLibrary,
}

#[derive(Debug, Clone, PartialEq)]
struct Fvr04RuntimeSceneState {
    snapshot: PersistentVoxelWorldSnapshot,
    creatures: Vec<Fvr04CreatureVisualRecord>,
}

#[derive(Resource)]
struct Fvr04RuntimeSceneAssets {
    selection_material: Handle<StandardMaterial>,
    terrain_materials: TerrainMaterialLibrary,
    dressing_library: TerrainDressingLibrary,
    vfx_unit_mesh: Option<Handle<Mesh>>,
    vfx_materials: BTreeMap<Fvr07ProductionVfxKind, Handle<StandardMaterial>>,
    overlay_materials: BTreeMap<Fvr05ProductionOverlayKind, Handle<StandardMaterial>>,
    selection_mesh: Handle<Mesh>,
}

struct Fvr04OverlaySpawnPlan {
    kind: Fvr05ProductionOverlayKind,
    cells: Vec<Fvr05OverlayCell>,
    visible: bool,
}

#[derive(Resource)]
struct Fvr05OverlayGeometryCache {
    cells_by_kind: BTreeMap<Fvr05ProductionOverlayKind, Vec<Fvr05OverlayCell>>,
}

struct Fvr04RuntimeSceneCandidate {
    runtime_state: Fvr04RuntimeSceneState,
    settings: Fvr03ProductionVoxelRendererSettings,
    visible_tiles: BTreeSet<VoxelTileCoord>,
    visible_chunks: BTreeSet<VoxelChunkCoord>,
    tile_summaries_by_tile: BTreeMap<VoxelTileCoord, Fvr05ProductionTileSummary>,
    material_counts: BTreeMap<Fvr03ProductionVoxelMaterialKind, usize>,
    terrain_samples: ProductionTerrainSampleMap,
    terrain_build: TerrainMeshBuild,
    tile_mesh_count: usize,
    overlay_spawns: Vec<Fvr04OverlaySpawnPlan>,
    dressing_spawns: Vec<ProductionTerrainDressingSpawn>,
    vfx_spawns: Vec<Fvr07VfxSpawn>,
}

struct Fvr04PreparedCreaturePart {
    recipe: CreatureAssemblyPartRecipe,
    mesh: Handle<Mesh>,
    transform: Transform,
}

struct Fvr04PreparedCreature {
    record: Fvr04CreatureVisualRecord,
    recipe: CreatureAssemblyRecipe,
    coat: CreatureCoatAssetHandles,
    parts: Vec<Fvr04PreparedCreaturePart>,
    root_transform: Transform,
    root_visual: Fvr04ProductionCreatureVisualMarker,
}

struct Fvr04PreparedCreatureBatch {
    settings: Fvr04ProductionCreatureRendererSettings,
    creatures: Vec<Fvr04PreparedCreature>,
}

#[derive(Debug, Clone, Copy)]
struct Fvr04PreparedContactShadow {
    tile: VoxelTileCoord,
    translation: Vec3,
    scale: f32,
    source_kind: &'static str,
    stable_id: Option<alife_core::WorldEntityId>,
}

struct Fvr04PreparedLighting {
    directional_shadows: bool,
    shadow_cascades: usize,
    shadow_maximum_distance: f32,
    contact_shadow_mesh: Option<Handle<Mesh>>,
    contact_shadow_material: Option<Handle<StandardMaterial>>,
    contact_shadows: Vec<Fvr04PreparedContactShadow>,
}

struct Fvr04PreparedRuntimeScene {
    candidate: Fvr04RuntimeSceneCandidate,
    creatures: Fvr04PreparedCreatureBatch,
    lighting: Fvr04PreparedLighting,
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Default, Resource)]
struct ProductionRuntimeLoadRequest {
    pending: bool,
}

#[cfg(feature = "gpu-runtime")]
impl ProductionRuntimeLoadRequest {
    fn queue(&mut self) -> bool {
        if self.pending {
            return false;
        }
        self.pending = true;
        true
    }

    fn take(&mut self) -> bool {
        std::mem::replace(&mut self.pending, false)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Fvr07VfxSpawn {
    kind: Fvr07ProductionVfxKind,
    tile: Option<VoxelTileCoord>,
    stable_id: Option<alife_core::WorldEntityId>,
    follows_creature: bool,
    translation: Vec3,
    scale: Vec3,
    color: [f32; 4],
    phase: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Fvr07ProductionPolishSummary {
    dressing_count: usize,
    vfx_marker_count: usize,
    gpu_vfx_emitter_count: usize,
    vfx_budget_state: &'static str,
    display_only: bool,
    uses_hanabi_gpu_particles: bool,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Fvr05ProductionUxStateResource {
    pub settings: Fvr05ProductionUxSettings,
    pub ui_settings_path: PathBuf,
    pub source_save_path: PathBuf,
    pub asset_root: PathBuf,
    pub profile_id: ProductionFrontendProfileId,
    pub profile_budget: ProductionFrontendProfileBudget,
    pub population: u16,
    pub resolution: (u32, u32),
    pub save_metadata: ProductionSaveMetadata,
    pub selected_backend: String,
    pub adapter_name: String,
    pub backend_api: String,
    pub graphics_backend: String,
    pub unavailable_reason: String,
    pub renderer_profile: String,
    pub state_trace: String,
    pub authority: Fvr05ProductionDebugAuthorityReport,
    pub gpu_runtime_state: GpuRuntimeSaveState,
    pub last_action: String,
    pub last_error: Option<String>,
}

impl Fvr05ProductionUxStateResource {
    pub fn from_summary(summary: &ProductionVoxelLaunchSummary) -> Self {
        let mut settings = summary.ui_settings.clone();
        settings.show_menu = summary.developer_overlay;
        settings.show_settings = false;
        settings.show_overlays = false;
        if summary.developer_overlay {
            settings.active_inspector_tab = Fvr05ProductionInspectorTab::GpuRuntime;
        }
        Self {
            settings,
            ui_settings_path: summary.ui_settings_path.clone(),
            source_save_path: summary.save_path.clone(),
            asset_root: summary.asset_root.clone(),
            profile_id: summary.profile_id,
            profile_budget: summary.profile_budget,
            population: summary.effective_population,
            resolution: summary.resolution,
            save_metadata: summary.save_metadata.clone(),
            selected_backend: summary.diagnostics.selected_backend.clone(),
            adapter_name: summary
                .diagnostics
                .adapter_name
                .clone()
                .unwrap_or_else(|| "unavailable".to_string()),
            backend_api: summary
                .diagnostics
                .backend_api
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            graphics_backend: summary.diagnostics.graphics_backend.clone(),
            unavailable_reason: summary
                .diagnostics
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| "None".to_string()),
            renderer_profile: summary.renderer_profile.clone(),
            state_trace: summary.state_labels().join(">"),
            authority: summary.debug_authority.clone(),
            gpu_runtime_state: summary.gpu_runtime_state.clone(),
            last_action: "Ready: production voxel world loaded from validated save".to_string(),
            last_error: summary.ui_settings_load_error.clone(),
        }
    }

    fn active_overlay(&self, kind: Fvr05ProductionOverlayKind) -> bool {
        self.settings.show_overlays && self.settings.enabled_overlays.contains(&kind)
    }

    fn toggle_overlay(&mut self, kind: Fvr05ProductionOverlayKind) {
        if let Some(index) = self
            .settings
            .enabled_overlays
            .iter()
            .position(|overlay| *overlay == kind)
        {
            self.settings.enabled_overlays.remove(index);
            self.last_action = format!("Overlay hidden: {}", kind.label());
        } else {
            self.settings.enabled_overlays.push(kind);
            self.settings.enabled_overlays.sort();
            self.last_action = format!("Overlay shown: {}", kind.label());
        }
    }

    fn update_selection_snapshot(
        &mut self,
        selection: Option<StableVoxelObjectRef>,
        follow_enabled: bool,
    ) {
        self.settings.selected_stable_id =
            selection.and_then(|selected| selected.stable_id.map(|stable_id| stable_id.raw()));
        self.settings.follow_selection = follow_enabled;
    }

    fn write_runtime_save(&mut self, create_world: bool) {
        let target_path = if create_world {
            PathBuf::from(&self.settings.created_world_save_path)
        } else {
            PathBuf::from(&self.settings.runtime_save_path)
        };
        let result = (|| -> Result<PathBuf, GameAppShellError> {
            let save = PortableSaveFile::from_json_file(&self.source_save_path)?;
            let production_save = production_voxel_save_with_population(
                &save,
                &self.asset_root,
                self.profile_id,
                self.population,
            )?;
            let production_save =
                production_save.with_gpu_runtime_state(self.gpu_runtime_state.clone())?;
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
            production_save.to_json_file(&target_path)?;
            Ok(target_path.clone())
        })();
        match result {
            Ok(path) => {
                self.last_error = None;
                self.last_action = if create_world {
                    format!("Created production world save: {}", path.display())
                } else {
                    format!("Saved production runtime state: {}", path.display())
                };
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                self.last_action = "Save failed".to_string();
            }
        }
    }

    #[cfg(feature = "gpu-runtime")]
    fn write_gpu_runtime_save(
        &mut self,
        create_world: bool,
        runtime: &mut crate::GpuLiveBrainRuntime,
    ) {
        let target_path = if create_world {
            PathBuf::from(&self.settings.created_world_save_path)
        } else {
            PathBuf::from(&self.settings.runtime_save_path)
        };
        match runtime.request_manual_checkpoint(target_path.clone()) {
            Ok(_) => {
                self.last_error = None;
                self.last_action = if create_world {
                    format!("Queued exact GPU world save: {}", target_path.display())
                } else {
                    format!("Queued exact GPU runtime save: {}", target_path.display())
                };
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                self.last_action = "GPU checkpoint save failed; prior save retained".to_string();
            }
        }
    }

    #[cfg(feature = "gpu-runtime")]
    fn observe_gpu_runtime_save_status(&mut self, status: &crate::GpuManualCheckpointStatus) {
        match status {
            crate::GpuManualCheckpointStatus::Complete {
                destination,
                checkpoint_tick,
            } => {
                self.gpu_runtime_state.last_safe_checkpoint.world_tick = *checkpoint_tick;
                self.gpu_runtime_state.last_safe_checkpoint.checkpoint_label = format!(
                    "{}:GpuAuthoritative:checkpoint-tick={}",
                    self.profile_id.label(),
                    checkpoint_tick.raw()
                );
                self.last_error = None;
                self.last_action = format!(
                    "Saved exact GPU checkpoint asynchronously: {}",
                    destination.display()
                );
            }
            crate::GpuManualCheckpointStatus::Failed {
                destination,
                message,
            } => {
                self.last_error = Some(message.clone());
                self.last_action = format!(
                    "GPU checkpoint save failed; prior save retained: {}",
                    destination.display()
                );
            }
            crate::GpuManualCheckpointStatus::Idle
            | crate::GpuManualCheckpointStatus::Queued { .. } => {}
        }
    }

    fn persist_ui_settings(&mut self) {
        match self.settings.to_json_file(&self.ui_settings_path) {
            Ok(()) => {
                self.last_error = None;
                self.last_action =
                    format!("Saved UX settings: {}", self.ui_settings_path.display());
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
                self.last_action = "UX settings save failed".to_string();
            }
        }
    }
}

#[cfg(all(feature = "gpu-runtime", feature = "gpu-tests"))]
pub fn production_archive_birth_manifest_for_test(
    app: &mut App,
    organism_id: OrganismId,
) -> Option<alife_core::Blake3Digest> {
    app.world_mut()
        .get_non_send_resource_mut::<ProductionGpuBrainRuntimeResource>()
        .and_then(|runtime| runtime.runtime.archive_birth_manifest(organism_id))
}

fn configure_production_voxel_presentation_schedule(app: &mut App) {
    app.configure_sets(
        Update,
        (
            ProductionVoxelPresentationSet::Input,
            ProductionVoxelPresentationSet::LiveGpuTick,
            ProductionVoxelPresentationSet::AuthoritativeProjection,
            ProductionVoxelPresentationSet::ProceduralAnimation,
            ProductionVoxelPresentationSet::RootReaders,
        )
            .chain(),
    );
}

#[cfg(feature = "gpu-runtime")]
fn production_entities_with<T: Component>(world: &mut World) -> Vec<Entity> {
    let mut query = world.query_filtered::<Entity, With<T>>();
    query.iter(world).collect()
}

#[cfg(feature = "gpu-runtime")]
fn despawn_production_entity_hierarchy(world: &mut World, entity: Entity) {
    let children = world
        .get::<Children>(entity)
        .map(|children| children.iter().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    for child in children {
        despawn_production_entity_hierarchy(world, child);
    }
    let _ = world.despawn(entity);
}

#[cfg(feature = "gpu-runtime")]
fn despawn_fvr04_runtime_scene(world: &mut World) {
    let roots = production_entities_with::<Fvr04ProductionRuntimeSceneRoot>(world);
    for root in roots {
        if let Some(mut map) = world.get_resource_mut::<BevyEntityMap>() {
            map.remove_by_entity(root);
        }
        despawn_production_entity_hierarchy(world, root);
    }
    let mut labels = world
        .query_filtered::<(&mut Text2d, &mut Visibility), With<Fvr04ProductionCreatureWorldLabel>>(
        );
    for (mut text, mut visibility) in labels.iter_mut(world) {
        text.0 = "creature".to_string();
        *visibility = Visibility::Hidden;
    }
}

#[cfg(feature = "gpu-runtime")]
fn clear_production_load_focus(world: &mut World) {
    if let Some(mut selection) = world.get_resource_mut::<Fvr03ProductionVoxelSelectionResource>() {
        selection.hovered = None;
        selection.selected = None;
    }
    if let Some(mut follow) = world.get_resource_mut::<Fvr04ProductionCreatureFollowResource>() {
        follow.enabled = false;
        follow.target_stable_id = None;
    }
    if let Some(mut ux) = world.get_resource_mut::<Fvr05ProductionUxStateResource>() {
        ux.settings.selected_stable_id = None;
        ux.settings.follow_selection = false;
    }

    let profile_id = world
        .get_resource::<Fvr03ProductionVoxelSceneResource>()
        .map(|scene| scene.profile_id);
    if let Some(profile_id) = profile_id {
        let extent = production_camera_extent(profile_id);
        let mut cameras = world.query_filtered::<
            (&mut Transform, &mut Projection, &Fvr03ProductionVoxelCamera),
            Without<ProductionCreatureAssemblyRoot>,
        >();
        for (mut transform, mut projection, camera) in cameras.iter_mut(world) {
            *transform = production_camera_transform(camera.mode, extent);
            if let Projection::Orthographic(orthographic) = &mut *projection {
                orthographic.scaling_mode = ScalingMode::FixedVertical {
                    viewport_height: extent,
                };
            }
        }
    }
}

#[cfg(feature = "gpu-runtime")]
fn report_production_runtime_load_failure(world: &mut World, message: String) {
    if let Some(mut ux) = world.get_resource_mut::<Fvr05ProductionUxStateResource>() {
        ux.last_error = Some(message);
        ux.last_action = "Load failed; current world left unchanged".to_string();
    }
}

#[cfg(feature = "gpu-runtime")]
fn build_production_load_presentation_frame(
    save: &PortableSaveFile,
) -> Result<LiveBrainPresentationFrameResource, GameAppShellError> {
    let candidate_world = save.restore_headless_world()?;
    LiveBrainPresentationFrameResource::from_authoritative_world(&candidate_world).map_err(
        |error| GameAppShellError::InvalidProductionFrontend {
            message: format!("FVR04 presentation frame restore failed: {error:?}"),
        },
    )
}

#[cfg(feature = "gpu-runtime")]
fn apply_production_runtime_load(world: &mut World) {
    let requested = world
        .get_resource_mut::<ProductionRuntimeLoadRequest>()
        .is_some_and(|mut request| request.take());
    if !requested {
        return;
    }

    let result = (|| -> Result<(), GameAppShellError> {
        let current_ux = world
            .get_resource::<Fvr05ProductionUxStateResource>()
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: "FVR05 UX resource missing during runtime load".to_string(),
            })?
            .clone();
        let save_path = PathBuf::from(&current_ux.settings.runtime_save_path);
        let durable = crate::GpuDurableSaveManifest::open(&save_path, &current_ux.asset_root)?;
        let loaded = durable.load()?;
        let candidate_runtime_state = load_fvr04_runtime_state_from_save(
            &loaded.save,
            &current_ux.asset_root,
            current_ux.profile_id,
            current_ux.population,
        )?;
        let candidate_frame = build_production_load_presentation_frame(&loaded.save)?;
        let mut candidate_settings = current_ux.settings.clone();
        if current_ux.ui_settings_path.exists() {
            candidate_settings =
                Fvr05ProductionUxSettings::from_json_file(&current_ux.ui_settings_path)?;
            candidate_settings.refresh_runtime_context(&current_ux.settings);
            candidate_settings.validate()?;
        }
        let renderer_settings =
            Fvr03ProductionVoxelRendererSettings::for_profile(current_ux.profile_id);
        let candidate_scene = prepare_fvr04_runtime_scene_candidate(
            candidate_runtime_state,
            renderer_settings,
            &candidate_settings,
            world
                .get_resource::<Fvr04CreatureSpawnContext>()
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: "FVR04 spawn context missing during runtime load".to_string(),
                })?,
        )?;
        let prepared_scene = world.resource_scope(|world, mut context| {
            prepare_fvr04_runtime_scene(world, candidate_scene, &mut context)
        })?;
        for (present, message) in [
            (
                world.contains_resource::<Fvr04RuntimeSceneAssets>(),
                "FVR04 scene assets missing during runtime load",
            ),
            (
                world.contains_resource::<ProductionGpuBrainTickScheduleResource>(),
                "production GPU schedule missing during runtime load",
            ),
            (
                world.contains_resource::<Fvr03ProductionVoxelSelectionResource>(),
                "FVR03 selection resource missing during runtime load",
            ),
            (
                world.contains_resource::<Fvr04ProductionCreatureFollowResource>(),
                "FVR04 follow resource missing during runtime load",
            ),
        ] {
            if !present {
                return Err(GameAppShellError::InvalidProductionFrontend {
                    message: message.to_string(),
                });
            }
        }
        let staging_backend = {
            let live_runtime = world
                .get_non_send_resource_mut::<ProductionGpuBrainRuntimeResource>()
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: "production GPU runtime missing during runtime load".to_string(),
                })?;
            live_runtime
                .runtime
                .new_staging_like_live()
                .map_err(|error| GameAppShellError::InvalidProductionFrontend {
                    message: format!("same-device staging backend unavailable: {error}"),
                })?
        };
        let playback = if candidate_settings.paused {
            RuntimePlaybackState::Paused
        } else {
            RuntimePlaybackState::Running
        };
        let speed_ticks = candidate_settings.simulation_speed.round().clamp(1.0, 5.0) as u32;

        {
            let mut live_runtime = world
                .get_non_send_resource_mut::<ProductionGpuBrainRuntimeResource>()
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: "production GPU runtime missing at load commit".to_string(),
                })?;
            live_runtime
                .runtime
                .replace_from_durable_save(staging_backend, durable)?;
        }

        world.insert_resource(candidate_frame);
        world
            .resource_mut::<ProductionGpuBrainTickScheduleResource>()
            .reset_after_load(playback, speed_ticks);
        {
            let mut ux = world.resource_mut::<Fvr05ProductionUxStateResource>();
            ux.settings = candidate_settings;
            ux.settings.selected_stable_id = None;
            ux.settings.follow_selection = false;
            ux.source_save_path = save_path.clone();
            ux.last_error = None;
        }

        despawn_fvr04_runtime_scene(world);
        let assets = world
            .remove_resource::<Fvr04RuntimeSceneAssets>()
            .expect("FVR04 scene assets passed precommit validation");
        let (scene, creature_scene) =
            spawn_fvr04_runtime_scene_candidate(world, prepared_scene, &assets);
        world.insert_resource(assets);
        install_fvr04_runtime_scene_resources(world, scene, creature_scene);
        clear_production_load_focus(world);
        if let Some(mut ux) = world.get_resource_mut::<Fvr05ProductionUxStateResource>() {
            ux.last_action = format!(
                "Loaded authoritative production runtime: {}",
                save_path.display()
            );
            ux.last_error = None;
        }
        Ok(())
    })();

    if let Err(error) = result {
        report_production_runtime_load_failure(world, error.to_string());
    }
}

#[cfg(feature = "gpu-runtime")]
pub(crate) fn dispatch_production_curated_founder_reset_core<R: CuratedFounderResetRuntimePort>(
    commands: &[ProductionCuratedFounderResetCommand],
    runtime: &mut R,
    result: &mut ProductionCuratedFounderResetResultResource,
) {
    if commands.len() != 1 {
        result.outcome =
            crate::gpu_live_runtime::CuratedFounderResetDispatchResult::PreCommitRejected {
                rejection:
                    crate::gpu_live_runtime::CuratedFounderResetDispatchRejection::MultipleCommands,
            };
        return;
    }
    let runtime_result = match &commands[0] {
        ProductionCuratedFounderResetCommand::Attempt(intent) => {
            runtime.dispatch_attempt(intent.clone())
        }
        ProductionCuratedFounderResetCommand::Retry => runtime.dispatch_retry(),
    };
    result.outcome = crate::gpu_live_runtime::project_curated_founder_reset_result(runtime_result);
}

#[cfg(feature = "gpu-runtime")]
fn dispatch_production_curated_founder_reset(
    mut commands: MessageReader<ProductionCuratedFounderResetCommand>,
    mut runtime: NonSendMut<ProductionGpuBrainRuntimeResource>,
    mut result: ResMut<ProductionCuratedFounderResetResultResource>,
) {
    let pending = commands.read().cloned().collect::<Vec<_>>();
    if pending.is_empty() {
        return;
    }
    dispatch_production_curated_founder_reset_core(&pending, &mut runtime.runtime, &mut *result);
}

pub fn spawn_fvr03_production_voxel_scene(
    app: &mut App,
    summary: &ProductionVoxelLaunchSummary,
) -> Result<(), GameAppShellError> {
    app.init_resource::<Assets<Image>>();
    let settings = Fvr03ProductionVoxelRendererSettings::for_profile(summary.profile_id);
    let creature_settings = Fvr04ProductionCreatureRendererSettings::for_profile(
        summary.profile_id,
        summary.effective_population,
    );
    let runtime_state = load_fvr04_runtime_state(summary)?;
    let scene_assets = create_fvr04_runtime_scene_assets(app, &settings);
    let creature_part_catalog = load_geneforge_creature_part_catalog().map_err(|error| {
        GameAppShellError::InvalidProductionFrontend {
            message: error.to_string(),
        }
    })?;
    let creature_assets_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let creature_preparations =
        load_geneforge_assembly_preparation_index(&creature_assets_root, &creature_part_catalog)
            .map_err(|error| GameAppShellError::InvalidProductionFrontend {
                message: error.to_string(),
            })?;
    let creature_part_assets = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        let active_lod = match creature_settings.lod {
            Fvr04CreatureLod::FullVoxel => CreaturePartLodId::Full,
            Fvr04CreatureLod::CompactVoxel => CreaturePartLodId::Compact,
            Fvr04CreatureLod::ImpostorVoxel => CreaturePartLodId::Impostor,
        };
        CreaturePartAssetLibrary::load_geneforge_lod_for_profile(
            &creature_assets_root,
            &creature_part_catalog,
            &mut meshes,
            summary.profile_id,
            active_lod,
        )
        .map_err(|error| GameAppShellError::InvalidProductionFrontend {
            message: error.to_string(),
        })?
    };
    let mut creature_spawn_context = Fvr04CreatureSpawnContext {
        settings: creature_settings,
        catalog: creature_part_catalog,
        preparations: creature_preparations,
        assets_root: creature_assets_root,
        creature_part_assets,
    };
    let candidate = prepare_fvr04_runtime_scene_candidate(
        runtime_state,
        settings.clone(),
        &summary.ui_settings,
        &creature_spawn_context,
    )?;
    let selected =
        fvr04_runtime_scene_selection(&candidate.runtime_state, &candidate.visible_tiles);
    let prepared =
        prepare_fvr04_runtime_scene(app.world_mut(), candidate, &mut creature_spawn_context)?;
    let (mut scene, creature_scene) =
        spawn_fvr04_runtime_scene_candidate(app.world_mut(), prepared, &scene_assets);
    spawn_production_terrain_camera(app, &settings);

    if summary.record_performance {
        scene.performance_artifact_path = Some(write_fvr03_performance_artifact(&scene, None)?);
    }

    install_fvr04_runtime_scene_resources(app.world_mut(), scene, creature_scene);
    app.insert_resource(creature_spawn_context);
    app.insert_resource(scene_assets);
    app.insert_resource(Fvr05ProductionUxStateResource::from_summary(summary));
    app.insert_resource(Fvr03ProductionVoxelSelectionResource {
        hovered: selected,
        selected,
    });
    app.insert_resource(Fvr04ProductionCreatureFollowResource {
        enabled: false,
        target_stable_id: selected.and_then(|selection| {
            (selection.kind == StableVoxelRefKind::Creature)
                .then_some(selection.stable_id)
                .flatten()
        }),
    });
    #[cfg(feature = "gpu-runtime")]
    app.insert_resource(ProductionRuntimeLoadRequest::default());
    configure_production_voxel_presentation_schedule(app);
    app.add_systems(
        Update,
        project_live_world_to_fvr04_creature_roots
            .in_set(ProductionVoxelPresentationSet::AuthoritativeProjection),
    )
    .add_systems(
        Update,
        (
            handle_fvr03_mouse_selection,
            handle_fvr03_camera_mode_input,
            handle_fvr04_camera_follow_input,
            handle_fvr05_production_ux_input,
        )
            .in_set(ProductionVoxelPresentationSet::Input),
    )
    .add_systems(
        Update,
        (animate_fvr04_creatures, animate_fvr04_creature_parts)
            .in_set(ProductionVoxelPresentationSet::ProceduralAnimation),
    )
    .add_systems(
        Update,
        (
            sync_fvr04_selection_marker,
            sync_fvr11_creature_contact_shadows,
            sync_fvr04_camera_follow,
            sync_fvr04_creature_label,
            sync_fvr05_panel_visibility,
            sync_fvr05_overlay_visibility,
            sync_fvr05_top_runtime_bar,
            sync_fvr05_left_control_panel,
            sync_fvr05_right_inspector_panel,
            sync_fvr05_bottom_overlay_toolbar,
            sync_fvr05_footer_status_bar,
        )
            .in_set(ProductionVoxelPresentationSet::RootReaders),
    )
    .add_systems(
        Update,
        (
            sync_v0_player_status_chip,
            sync_v0_player_creature_panel,
            sync_v0_player_control_strip,
        )
            .in_set(ProductionVoxelPresentationSet::RootReaders),
    );
    #[cfg(not(feature = "vfx-hanabi"))]
    app.add_systems(
        Update,
        sync_fvr07_attached_fallback_vfx
            .in_set(ProductionVoxelPresentationSet::AuthoritativeProjection),
    )
    .add_systems(
        Update,
        animate_fvr07_production_vfx.in_set(ProductionVoxelPresentationSet::ProceduralAnimation),
    );
    #[cfg(feature = "vfx-hanabi")]
    app.add_systems(
        Update,
        sync_fvr07_attached_hanabi_vfx
            .in_set(ProductionVoxelPresentationSet::AuthoritativeProjection),
    );
    #[cfg(feature = "gpu-runtime")]
    app.add_systems(
        Update,
        apply_production_runtime_load
            .in_set(ProductionVoxelPresentationSet::Input)
            .after(handle_fvr05_production_ux_input)
            .after(dispatch_production_curated_founder_reset)
            .before(ProductionVoxelPresentationSet::LiveGpuTick),
    );
    #[cfg(feature = "gpu-runtime")]
    crate::install_production_conversation_lineage_ui(app, summary);
    #[cfg(feature = "gpu-runtime")]
    app.add_systems(
        Update,
        dispatch_production_curated_founder_reset
            .in_set(ProductionVoxelPresentationSet::Input)
            .after(crate::production_conversation_lineage_ui::handle_production_conversation_lineage_input)
            .before(ProductionVoxelPresentationSet::LiveGpuTick),
    );
    if summary.record_performance && !summary.dry_run {
        #[cfg(feature = "gpu-runtime")]
        {
            app.insert_resource(Phase31PerformanceMetricsResource::new(summary))
                .add_systems(
                    Update,
                    phase31_performance_frame_begin.before(ProductionVoxelPresentationSet::Input),
                )
                .add_systems(
                    Update,
                    phase31_performance_after_input
                        .after(ProductionVoxelPresentationSet::Input)
                        .before(ProductionVoxelPresentationSet::LiveGpuTick),
                )
                .add_systems(
                    Update,
                    phase31_performance_after_live_gpu_tick
                        .after(ProductionVoxelPresentationSet::LiveGpuTick)
                        .before(ProductionVoxelPresentationSet::AuthoritativeProjection),
                )
                .add_systems(
                    Update,
                    phase31_performance_after_authoritative_projection
                        .after(ProductionVoxelPresentationSet::AuthoritativeProjection)
                        .before(ProductionVoxelPresentationSet::ProceduralAnimation),
                )
                .add_systems(
                    Update,
                    phase31_performance_after_procedural_animation
                        .after(ProductionVoxelPresentationSet::ProceduralAnimation)
                        .before(ProductionVoxelPresentationSet::RootReaders),
                )
                .add_systems(
                    Update,
                    phase31_performance_after_ui
                        .after(ProductionVoxelPresentationSet::RootReaders)
                        .before(request_fvr03_recorded_screenshot),
                );
        }
        let screenshot_path = PathBuf::from(FVR03_PERFORMANCE_ARTIFACT_DIR).join(format!(
            "{}_runtime_screenshot.png",
            summary.profile_id.label()
        ));
        app.insert_resource(Fvr03ProductionVoxelScreenshotResource {
            frame: 0,
            capture_after_frame: fvr03_screenshot_capture_frame(&settings),
            measurement_sample_frames: 60,
            measurement_start_frame: 0,
            measurement_started_at: None,
            measurement_written: false,
            requested: false,
            path: screenshot_path,
            product_screenshot_captured: false,
            fvr05_capture_index: 0,
            fvr05_next_capture_frame: 0,
            fvr05_sequence_complete: false,
            developer_overlay: summary.developer_overlay,
        })
        .add_systems(Update, request_fvr03_recorded_screenshot);
    }
    spawn_fvr05_production_ux_ui(app);
    spawn_v0_player_experience_ui(app);
    spawn_fvr04_creature_world_label(app, selected);
    Ok(())
}

fn load_fvr04_runtime_state(
    summary: &ProductionVoxelLaunchSummary,
) -> Result<Fvr04RuntimeSceneState, GameAppShellError> {
    let save = PortableSaveFile::from_json_file(&summary.save_path)?;
    load_fvr04_runtime_state_from_save(
        &save,
        &summary.asset_root,
        summary.profile_id,
        summary.effective_population,
    )
}

fn load_fvr04_runtime_state_from_save(
    save: &PortableSaveFile,
    asset_root: &PathBuf,
    profile_id: ProductionFrontendProfileId,
    population: u16,
) -> Result<Fvr04RuntimeSceneState, GameAppShellError> {
    let production_save =
        production_voxel_save_with_population(save, asset_root, profile_id, population)?;
    let backend_state = production_save.require_voxel_backend()?.clone();
    let backend = PersistentVoxelWorldBackend::from_save_state(backend_state.clone())?;
    let anchors = backend_state
        .creature_anchors
        .iter()
        .map(|anchor| {
            CreatureWorldAnchor::new(
                anchor.stable_id,
                Vec3f::new(anchor.tile.x as f32, 0.0, anchor.tile.z as f32),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let snapshot = backend.snapshot_for_anchors(&anchors)?;
    if snapshot.schema != FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: format!(
                "FVR04 expected FVR02 snapshot schema, got {}",
                snapshot.schema
            ),
        });
    }
    let creatures = fvr04_creature_visual_records_from_save(&production_save, &snapshot)?;
    Ok(Fvr04RuntimeSceneState {
        snapshot,
        creatures,
    })
}

fn fvr04_creature_visual_records_from_save(
    save: &PortableSaveFile,
    snapshot: &PersistentVoxelWorldSnapshot,
) -> Result<Vec<Fvr04CreatureVisualRecord>, GameAppShellError> {
    let objects_by_stable_id = save
        .world
        .objects
        .iter()
        .filter(|object| object.kind == WorldObjectKind::Agent)
        .map(|object| (object.id.raw(), object))
        .collect::<BTreeMap<_, _>>();
    let creatures_by_organism = save
        .creatures
        .iter()
        .map(|creature| (creature.organism_id.raw(), creature))
        .collect::<BTreeMap<_, _>>();
    let mut records = Vec::with_capacity(snapshot.creatures.len());
    for anchor in &snapshot.creatures {
        let object = objects_by_stable_id
            .get(&anchor.stable_id.raw())
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "FVR04 voxel creature {} missing world object",
                    anchor.stable_id.raw()
                ),
            })?;
        let organism_id =
            object
                .organism_id
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: format!(
                        "FVR04 voxel creature {} missing organism_id",
                        anchor.stable_id.raw()
                    ),
                })?;
        let creature = creatures_by_organism
            .get(&organism_id.raw())
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "FVR04 organism {} missing creature save state",
                    organism_id.raw()
                ),
            })?;
        let position = Vec3f::new(
            anchor.tile.x as f32 + 0.5,
            object.position.y,
            anchor.tile.z as f32 + 0.5,
        );
        let visual = creature_visual_snapshot_from_parts_with_appearance(
            organism_id,
            anchor.stable_id,
            position,
            None,
            None,
            &creature.mind.homeostasis,
            fvr04_sleep_phase_from_creature_save(creature),
            None,
            creature.appearance,
        )?;
        records.push(Fvr04CreatureVisualRecord {
            stable_ref: StableVoxelObjectRef {
                kind: StableVoxelRefKind::Creature,
                stable_id: Some(anchor.stable_id),
                chunk: anchor.chunk,
                tile: Some(anchor.tile),
            },
            tile: anchor.tile,
            display_label: object.label.clone(),
            brain_class_id: Some(creature.brain_class.default_class_id().raw()),
            brain_neuron_count: creature.brain_class.neuron_count(),
            social_affinity: object.social_affinity,
            reproductive_drive: creature.mind.homeostasis.drives.reproductive_drive,
            fast_memory_count: None,
            lifetime_memory_count: None,
            memory_record_count: Some(creature.mind.memory_record_count),
            concept_count: Some(creature.mind.concept_count),
            unresolved_gap_count: Some(creature.mind.unresolved_gap_count),
            lifetime_learning_enabled: Some(creature.learning.lifetime_learning_enabled),
            sleep_phase_raw: Some(fvr04_sleep_phase_from_creature_save(creature).raw()),
            consolidation_state_raw: None,
            last_consolidated_tick: creature
                .learning
                .last_consolidated_tick
                .map(|tick| tick.raw()),
            topology_update_count: None,
            visual,
        });
    }
    records.sort_by_key(|record| record.visual.stable_id.raw());
    Ok(records)
}

fn fvr04_sleep_phase_from_creature_save(creature: &CreatureSaveState) -> alife_core::SleepPhase {
    match creature.mind.sleep_state_label.as_str() {
        "sleeping" | "consolidating" => alife_core::SleepPhase::Consolidating,
        "entering_sleep" => alife_core::SleepPhase::EnteringSleep,
        "waking" => alife_core::SleepPhase::Waking,
        "forced_recovery_sleep" => alife_core::SleepPhase::ForcedRecoverySleep,
        _ => alife_core::SleepPhase::Awake,
    }
}

fn validate_fvr04_creature_spawn_inputs(
    creatures: &[Fvr04CreatureVisualRecord],
    context: &Fvr04CreatureSpawnContext,
) -> Result<(), GameAppShellError> {
    let lod = match context.settings.lod {
        Fvr04CreatureLod::FullVoxel => CreaturePartLodId::Full,
        Fvr04CreatureLod::CompactVoxel => CreaturePartLodId::Compact,
        Fvr04CreatureLod::ImpostorVoxel => CreaturePartLodId::Impostor,
    };

    for creature in creatures {
        let visual = &creature.visual;
        let coat_key = CreatureCoatKey::new(
            visual.appearance.part_sources,
            visual.appearance.palette_family,
            visual.appearance.fur_pattern,
            visual.appearance.marking_density,
        );
        let recipe = resolve_geneforge_creature_assembly(
            visual.appearance.part_sources,
            lod,
            coat_key,
            &context.catalog,
            &context.preparations,
        )
        .map_err(|error| GameAppShellError::InvalidProductionFrontend {
            message: format!(
                "FVR04 saved creature {} assembly validation failed: {}",
                visual.stable_id.raw(),
                error
            ),
        })?;
        if recipe.parts.is_empty() {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "FVR04 saved creature {} has no visible assembly parts",
                    visual.stable_id.raw()
                ),
            });
        }
        for part in recipe.parts.values() {
            let key = part.mesh_key();
            if context.creature_part_assets.bounds(key.clone()).is_none()
                || context.creature_part_assets.mesh(key.clone()).is_none()
            {
                return Err(GameAppShellError::InvalidProductionFrontend {
                    message: format!(
                        "FVR04 saved creature {} has unloaded mesh {:?}",
                        visual.stable_id.raw(),
                        key
                    ),
                });
            }
        }

        if !matches!(context.settings.lod, Fvr04CreatureLod::ImpostorVoxel) {
            let head = recipe.parts.get(&CreaturePartSlot::Head).ok_or_else(|| {
                GameAppShellError::InvalidProductionFrontend {
                    message: format!(
                        "FVR04 saved creature {} assembly has no head",
                        visual.stable_id.raw()
                    ),
                }
            })?;
            let head_asset = context.catalog.asset(&head.asset_id).ok_or_else(|| {
                GameAppShellError::InvalidProductionFrontend {
                    message: format!(
                        "FVR04 saved creature {} head asset is missing",
                        visual.stable_id.raw()
                    ),
                }
            })?;
            let emitted_head_bounds = context
                .creature_part_assets
                .bounds(head.mesh_key())
                .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                    message: format!(
                        "FVR04 saved creature {} head bounds are missing",
                        visual.stable_id.raw()
                    ),
                })?;
            let face_landmarks = remap_creature_face_landmarks(
                head_asset.canonical_bounds,
                emitted_head_bounds,
                &head.landmarks,
            )
            .map_err(|error| GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "FVR04 saved creature {} face landmarks invalid: {}",
                    visual.stable_id.raw(),
                    error
                ),
            })?;
            creature_face_style_from_landmarks(visual.appearance, &face_landmarks).map_err(
                |error| GameAppShellError::InvalidProductionFrontend {
                    message: format!(
                        "FVR04 saved creature {} face style invalid: {}",
                        visual.stable_id.raw(),
                        error
                    ),
                },
            )?;
        }
    }
    Ok(())
}

fn procedural_config_from_snapshot(
    snapshot: &PersistentVoxelWorldSnapshot,
) -> ProceduralWorldConfig {
    ProceduralWorldConfig {
        schema_version: alife_world::PROCEDURAL_WORLD_CHUNKS_SCHEMA_VERSION,
        seed: snapshot.world_seed,
        chunk_tile_size: i32::from(snapshot.profile_budget.chunk_tile_size),
        activation_radius_chunks: i32::from(snapshot.profile_budget.activation_radius_chunks),
        max_active_chunks: usize::from(snapshot.profile_budget.active_chunk_cap),
        max_active_content_candidates: usize::from(snapshot.profile_budget.max_content_candidates),
        neighborhood_radius_tiles: i32::from(snapshot.profile_budget.neighborhood_radius_tiles),
        max_neighborhood_samples: usize::from(snapshot.profile_budget.max_neighborhood_samples),
        virtual_half_extent_chunks: snapshot.profile_budget.virtual_half_extent_chunks,
    }
}

fn fvr04_runtime_scene_selection(
    runtime_state: &Fvr04RuntimeSceneState,
    visible_tiles: &BTreeSet<VoxelTileCoord>,
) -> Option<StableVoxelObjectRef> {
    runtime_state
        .creatures
        .first()
        .map(|creature| creature.stable_ref)
        .or_else(|| {
            visible_tiles
                .iter()
                .copied()
                .find_map(|tile| runtime_state.snapshot.lookup_tile(tile))
        })
        .or_else(|| {
            runtime_state
                .snapshot
                .selection_refs
                .iter()
                .copied()
                .find(|reference| reference.tile.is_some())
        })
}

fn prepare_fvr04_runtime_scene_candidate(
    runtime_state: Fvr04RuntimeSceneState,
    settings: Fvr03ProductionVoxelRendererSettings,
    ux_settings: &Fvr05ProductionUxSettings,
    context: &Fvr04CreatureSpawnContext,
) -> Result<Fvr04RuntimeSceneCandidate, GameAppShellError> {
    validate_fvr04_creature_spawn_inputs(&runtime_state.creatures, context)?;
    let snapshot = &runtime_state.snapshot;
    let visible_chunks = snapshot
        .visible_chunks
        .iter()
        .map(|chunk| chunk.coord)
        .collect::<BTreeSet<_>>();
    let procedural_config = procedural_config_from_snapshot(snapshot);
    let mut visible_tiles = BTreeSet::new();
    let mut tile_summaries_by_tile = BTreeMap::new();
    let mut material_counts = BTreeMap::new();
    let mut terrain_samples = ProductionTerrainSampleMap::new();
    let mut tile_mesh_count = 0_usize;
    for chunk in &snapshot.visible_chunks {
        tile_mesh_count = tile_mesh_count.saturating_add(prepare_fvr03_chunk_tiles(
            snapshot,
            procedural_config,
            &settings,
            chunk.coord,
            &mut visible_tiles,
            &mut tile_summaries_by_tile,
            &mut material_counts,
            &mut terrain_samples,
        )?);
    }
    let terrain_build = build_production_terrain_meshes(
        &terrain_samples,
        f32::from(settings.tile_stride.max(1)),
        crate::production_terrain::TerrainAtlasLayout::PRODUCTION,
    );
    let selected = fvr04_runtime_scene_selection(&runtime_state, &visible_tiles);
    let overlay_spawns = Fvr05ProductionOverlayKind::all()
        .iter()
        .copied()
        .map(|kind| Fvr04OverlaySpawnPlan {
            kind,
            cells: fvr05_overlay_cells(
                kind,
                &settings,
                &tile_summaries_by_tile,
                &visible_chunks,
                &runtime_state.creatures,
                snapshot.profile_budget.chunk_tile_size,
            ),
            visible: ux_settings.show_overlays && ux_settings.enabled_overlays.contains(&kind),
        })
        .collect();
    let dressing_tiles = tile_summaries_by_tile
        .values()
        .map(|tile| {
            (
                tile.tile,
                TerrainDressingTile {
                    tile: tile.tile,
                    material: tile.material,
                    height: tile.height_units,
                    resource_bias: tile.resource_bias,
                    hazard_pressure: tile.hazard_pressure,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let occupied_tiles = runtime_state
        .creatures
        .iter()
        .map(|creature| creature.tile)
        .collect::<BTreeSet<_>>();
    let dressing_spawns = plan_production_terrain_dressing(
        &dressing_tiles,
        &occupied_tiles,
        settings.production_dressing_cap,
        settings.tile_stride,
        settings.minimum_floor,
    );
    let vfx_spawns = fvr07_vfx_spawns(
        &settings,
        &tile_summaries_by_tile,
        &runtime_state.creatures,
        selected,
    );
    Ok(Fvr04RuntimeSceneCandidate {
        runtime_state,
        settings,
        visible_tiles,
        visible_chunks,
        tile_summaries_by_tile,
        material_counts,
        terrain_samples,
        terrain_build,
        tile_mesh_count,
        overlay_spawns,
        dressing_spawns,
        vfx_spawns,
    })
}

fn fvr04_scene_preflight_error(message: impl Into<String>) -> GameAppShellError {
    GameAppShellError::InvalidProductionFrontend {
        message: message.into(),
    }
}

fn prepare_fvr04_creature_batch(
    world: &mut World,
    creatures: &[Fvr04CreatureVisualRecord],
    tile_summaries: &BTreeMap<VoxelTileCoord, Fvr05ProductionTileSummary>,
    context: &mut Fvr04CreatureSpawnContext,
) -> Result<Fvr04PreparedCreatureBatch, GameAppShellError> {
    let settings = context.settings.clone();
    let lod = match settings.lod {
        Fvr04CreatureLod::FullVoxel => CreaturePartLodId::Full,
        Fvr04CreatureLod::CompactVoxel => CreaturePartLodId::Compact,
        Fvr04CreatureLod::ImpostorVoxel => CreaturePartLodId::Impostor,
    };
    let mut prepared = Vec::new();
    for (index, creature) in creatures
        .iter()
        .take(usize::from(settings.max_visible_creatures))
        .enumerate()
    {
        let visual = &creature.visual;
        let coat_key = CreatureCoatKey::new(
            visual.appearance.part_sources,
            visual.appearance.palette_family,
            visual.appearance.fur_pattern,
            visual.appearance.marking_density,
        );
        let recipe = resolve_geneforge_creature_assembly(
            visual.appearance.part_sources,
            lod,
            coat_key,
            &context.catalog,
            &context.preparations,
        )
        .map_err(|error| {
            fvr04_scene_preflight_error(format!(
                "FVR04 saved creature {} assembly preparation failed: {error}",
                visual.stable_id.raw()
            ))
        })?;
        if recipe.parts.is_empty() {
            return Err(fvr04_scene_preflight_error(format!(
                "FVR04 saved creature {} has no visible assembly parts",
                visual.stable_id.raw()
            )));
        }
        let mut parts = Vec::with_capacity(recipe.parts.len());
        let mut local_bounds = None::<CreatureVisualBounds>;
        for part in recipe.parts.values() {
            let key = part.mesh_key();
            let bounds = context
                .creature_part_assets
                .bounds(key.clone())
                .ok_or_else(|| {
                    fvr04_scene_preflight_error(format!(
                        "FVR04 saved creature {} part {:?} has no finite bounds",
                        visual.stable_id.raw(),
                        part.slot
                    ))
                })?;
            let mesh = context.creature_part_assets.mesh(key).ok_or_else(|| {
                fvr04_scene_preflight_error(format!(
                    "FVR04 saved creature {} part {:?} mesh is not loaded",
                    visual.stable_id.raw(),
                    part.slot
                ))
            })?;
            let transform = geneforge_authored_transform_to_bevy(part.authored_transform);
            let transformed = transform_creature_visual_bounds(bounds, transform);
            if let Some(current) = &mut local_bounds {
                current.include(transformed);
            } else {
                local_bounds = Some(transformed);
            }
            parts.push(Fvr04PreparedCreaturePart {
                recipe: part.clone(),
                mesh,
                transform,
            });
        }
        let local_bounds = local_bounds.ok_or_else(|| {
            fvr04_scene_preflight_error(format!(
                "FVR04 saved creature {} produced no visible bounds",
                visual.stable_id.raw()
            ))
        })?;
        let coat = world
            .resource_scope(|world, mut images: bevy::prelude::Mut<Assets<Image>>| {
                world.resource_scope(
                    |_world, mut materials: bevy::prelude::Mut<Assets<StandardMaterial>>| {
                        context.creature_part_assets.acquire_geneforge_coat(
                            &context.assets_root,
                            &context.catalog,
                            &recipe,
                            &mut images,
                            &mut materials,
                        )
                    },
                )
            })
            .map_err(|error| {
                fvr04_scene_preflight_error(format!(
                    "FVR04 saved creature {} coat preparation failed: {error}",
                    visual.stable_id.raw()
                ))
            })?;
        let surface_height = tile_summaries
            .get(&creature.tile)
            .map(|tile| tile.height_units)
            .unwrap_or(0.44);
        let base_scale = fvr04_creature_scale(visual, settings.lod);
        let base_height = grounded_root_height(
            surface_height,
            0.04,
            local_bounds,
            base_scale.to_array(),
            bevy::math::Mat3::IDENTITY.to_cols_array(),
        );
        let base_translation = Vec3::new(
            creature.tile.x as f32 + 0.5,
            base_height,
            creature.tile.z as f32 + 0.5,
        );
        let root_transform = Transform::from_translation(base_translation)
            .with_rotation(Quat::from_rotation_y(std::f32::consts::PI))
            .with_scale(base_scale);
        let phase = (index as f32 * 0.37) + (visual.stable_id.raw() % 17) as f32 * 0.11;
        prepared.push(Fvr04PreparedCreature {
            record: creature.clone(),
            recipe,
            coat,
            parts,
            root_transform,
            root_visual: Fvr04ProductionCreatureVisualMarker {
                stable_id: visual.stable_id,
                organism_id: visual.organism_id,
                tile: creature.tile,
                expression: visual.expression,
                animation: visual.animation,
                lod: settings.lod,
                base_translation,
                local_offset: Vec3::ZERO,
                base_scale,
                local_bounds,
                surface_height,
                phase,
            },
        });
    }
    Ok(Fvr04PreparedCreatureBatch {
        settings,
        creatures: prepared,
    })
}

fn prepare_fvr04_lighting(
    world: &mut World,
    candidate: &Fvr04RuntimeSceneCandidate,
    creatures: &Fvr04PreparedCreatureBatch,
) -> Fvr04PreparedLighting {
    let shadow_cascades = production_shadow_cascade_count(&candidate.settings);
    let directional_shadows = shadow_cascades > 0;
    let shadow_maximum_distance = production_shadow_maximum_distance(&candidate.settings);
    if directional_shadows {
        return Fvr04PreparedLighting {
            directional_shadows,
            shadow_cascades,
            shadow_maximum_distance,
            contact_shadow_mesh: None,
            contact_shadow_material: None,
            contact_shadows: Vec::new(),
        };
    }
    let contact_shadow_mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(fvr04_contact_shadow_mesh());
    let contact_shadow_material =
        world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                base_color: Color::srgba(0.055, 0.075, 0.038, 0.24),
                alpha_mode: AlphaMode::Blend,
                perceptual_roughness: 1.0,
                cull_mode: None,
                unlit: true,
                ..default()
            });
    let height_for = |tile: VoxelTileCoord| {
        candidate
            .tile_summaries_by_tile
            .get(&tile)
            .map(|summary| summary.height_units)
            .unwrap_or(0.0)
            + 0.018
    };
    let mut contact_shadows = creatures
        .creatures
        .iter()
        .map(|creature| Fvr04PreparedContactShadow {
            tile: creature.record.tile,
            translation: Vec3::new(
                creature.root_transform.translation.x,
                height_for(creature.record.tile),
                creature.root_transform.translation.z,
            ),
            scale: 1.0,
            source_kind: "creature",
            stable_id: Some(creature.record.visual.stable_id),
        })
        .collect::<Vec<_>>();
    contact_shadows.extend(
        candidate
            .dressing_spawns
            .iter()
            .filter(|spawn| spawn.scale.y >= 1.0)
            .map(|spawn| Fvr04PreparedContactShadow {
                tile: spawn.tile,
                translation: Vec3::new(
                    spawn.translation.x,
                    height_for(spawn.tile),
                    spawn.translation.z,
                ),
                scale: 0.78,
                source_kind: "dressing",
                stable_id: None,
            }),
    );
    Fvr04PreparedLighting {
        directional_shadows,
        shadow_cascades,
        shadow_maximum_distance,
        contact_shadow_mesh: Some(contact_shadow_mesh),
        contact_shadow_material: Some(contact_shadow_material),
        contact_shadows,
    }
}

fn spawn_fvr04_prepared_lighting(world: &mut World, lighting: Fvr04PreparedLighting) {
    let light = DirectionalLight {
        color: Color::srgb(1.0, 0.86, 0.66),
        illuminance: 5800.0,
        shadows_enabled: lighting.directional_shadows,
        ..default()
    };
    let transform = Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.05, 0.62, -0.42));
    if lighting.directional_shadows {
        world.spawn((
            Name::new(format!(
                "A-Life FVR11 warm {}-cascade directional sun",
                lighting.shadow_cascades
            )),
            light,
            bevy::light::CascadeShadowConfigBuilder {
                num_cascades: lighting.shadow_cascades,
                minimum_distance: 0.1,
                maximum_distance: lighting.shadow_maximum_distance,
                first_cascade_far_bound: 28.0,
                overlap_proportion: 0.18,
            }
            .build(),
            transform,
            Fvr04ProductionRuntimeSceneRoot,
        ));
        return;
    }
    world.spawn((
        Name::new("A-Life FVR11 minimum-profile warm directional sun"),
        light,
        transform,
        Fvr04ProductionRuntimeSceneRoot,
    ));
    let (Some(mesh), Some(material)) = (
        lighting.contact_shadow_mesh,
        lighting.contact_shadow_material,
    ) else {
        return;
    };
    for shadow in lighting.contact_shadows {
        world.spawn((
            Name::new(format!(
                "A-Life FVR11 minimum contact shadow {} {}:{}",
                shadow.source_kind, shadow.tile.x, shadow.tile.z
            )),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(shadow.translation).with_scale(Vec3::splat(shadow.scale)),
            bevy::light::NotShadowCaster,
            bevy::picking::Pickable::IGNORE,
            crate::Fvr11ProductionContactShadow {
                source_kind: shadow.source_kind,
                tile: shadow.tile,
                stable_id: shadow.stable_id,
                display_only: true,
                no_renderer_authority_over_world_actions_or_cognition: true,
            },
            Fvr04ProductionRuntimeSceneRoot,
        ));
    }
}

fn fvr04_contact_shadow_mesh() -> Mesh {
    const SEGMENTS: u32 = 12;
    const RADIUS: f32 = 0.36;
    let mut positions = Vec::with_capacity((SEGMENTS + 1) as usize);
    let mut normals = Vec::with_capacity((SEGMENTS + 1) as usize);
    let mut uvs = Vec::with_capacity((SEGMENTS + 1) as usize);
    let mut indices = Vec::with_capacity((SEGMENTS * 3) as usize);
    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);
    for index in 0..SEGMENTS {
        let angle = index as f32 * std::f32::consts::TAU / SEGMENTS as f32;
        positions.push([angle.cos() * RADIUS, 0.0, angle.sin() * RADIUS]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([angle.cos() * 0.5 + 0.5, angle.sin() * 0.5 + 0.5]);
    }
    for index in 0..SEGMENTS {
        indices.extend([0, index + 1, ((index + 1) % SEGMENTS) + 1]);
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn prepare_fvr04_runtime_scene(
    world: &mut World,
    candidate: Fvr04RuntimeSceneCandidate,
    context: &mut Fvr04CreatureSpawnContext,
) -> Result<Fvr04PreparedRuntimeScene, GameAppShellError> {
    let creatures = prepare_fvr04_creature_batch(
        world,
        &candidate.runtime_state.creatures,
        &candidate.tile_summaries_by_tile,
        context,
    )?;
    let lighting = prepare_fvr04_lighting(world, &candidate, &creatures);
    Ok(Fvr04PreparedRuntimeScene {
        candidate,
        creatures,
        lighting,
    })
}

fn create_fvr04_runtime_scene_assets(
    app: &mut App,
    settings: &Fvr03ProductionVoxelRendererSettings,
) -> Fvr04RuntimeSceneAssets {
    let selection_material = create_fvr03_selection_material(app, &settings.material_palette());
    let terrain_materials = create_production_terrain_material_library(app);
    install_animated_water_material(app, terrain_materials.water.clone());
    let dressing_library = create_terrain_dressing_library(app);
    let (vfx_unit_mesh, selection_mesh) = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        (
            (!cfg!(feature = "vfx-hanabi")).then(|| meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            meshes.add(Torus::new(0.54, 0.70)),
        )
    };
    let vfx_materials = if cfg!(feature = "vfx-hanabi") {
        BTreeMap::new()
    } else {
        fvr07_vfx_materials(app.world_mut())
    };
    let overlay_materials = Fvr05ProductionOverlayKind::all()
        .iter()
        .copied()
        .map(|kind| {
            let handle = app
                .world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .add(fvr05_overlay_material(kind));
            (kind, handle)
        })
        .collect();
    Fvr04RuntimeSceneAssets {
        selection_material,
        terrain_materials,
        dressing_library,
        vfx_unit_mesh,
        vfx_materials,
        overlay_materials,
        selection_mesh,
    }
}

fn spawn_fvr04_runtime_scene_candidate(
    world: &mut World,
    prepared: Fvr04PreparedRuntimeScene,
    assets: &Fvr04RuntimeSceneAssets,
) -> (
    Fvr03ProductionVoxelSceneResource,
    Fvr04ProductionCreatureSceneResource,
) {
    let Fvr04PreparedRuntimeScene {
        candidate,
        creatures,
        lighting,
    } = prepared;
    let Fvr04RuntimeSceneCandidate {
        runtime_state,
        settings,
        visible_tiles,
        visible_chunks,
        tile_summaries_by_tile,
        material_counts,
        terrain_samples,
        terrain_build,
        tile_mesh_count,
        overlay_spawns,
        dressing_spawns,
        vfx_spawns,
    } = candidate;
    let snapshot = &runtime_state.snapshot;
    let selected = fvr04_runtime_scene_selection(&runtime_state, &visible_tiles);
    let terrain_receipt = spawn_fvr11_layered_terrain_meshes(
        world,
        &assets.terrain_materials,
        &settings,
        snapshot,
        &terrain_samples,
        terrain_build,
        tile_mesh_count,
    );
    let terrain_scene = Fvr11ProductionTerrainSceneResource {
        visual_version: FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION,
        sample_count: terrain_samples.len(),
        top_layer_count: terrain_receipt.top_layer_count,
        cliff_layer_count: terrain_receipt.cliff_layer_count,
        transition_edge_count: terrain_receipt.transition_edge_count,
        water_layer_count: terrain_receipt.water_layer_count,
        confetti_detail_quad_count: terrain_receipt.confetti_detail_quad_count,
        display_only: true,
        no_renderer_authority_over_world_actions_or_cognition: true,
    };
    let creature_scene = spawn_fvr04_prepared_creature_batch(world, creatures);
    spawn_fvr05_overlay_batches(world, overlay_spawns, &assets.overlay_materials);
    let polish = spawn_fvr07_production_visual_polish(
        world,
        &settings,
        dressing_spawns,
        vfx_spawns,
        &assets.dressing_library,
        &assets.vfx_unit_mesh,
        &assets.vfx_materials,
    );
    if let Some(selection) = selected {
        spawn_fvr03_selection_marker(
            world,
            assets.selection_material.clone(),
            assets.selection_mesh.clone(),
            selection,
        );
    }
    spawn_fvr04_prepared_lighting(world, lighting);
    let scene = Fvr03ProductionVoxelSceneResource {
        schema: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA,
        schema_version: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA_VERSION,
        snapshot_schema: snapshot.schema.clone(),
        profile_id: settings.profile_id,
        population: runtime_state.creatures.len().min(u16::MAX as usize) as u16,
        renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE.to_string(),
        backend_id: FVR10_RENDERER_BACKEND_ID,
        uses_internal_voxel_terrain_mesh: true,
        visible_chunk_count: snapshot.visible_chunks.len(),
        resident_chunk_count: snapshot.visible_chunks.len(),
        tile_mesh_count,
        creature_render_count: creature_scene.rendered_creature_count,
        creature_material_bucket_count: creature_scene.material_bucket_count,
        creature_lod: creature_scene.lod,
        creature_root_count: creature_scene.creature_root_count,
        creature_part_entity_count: creature_scene.creature_part_entity_count,
        creature_join_cover_count: creature_scene.creature_join_cover_count,
        creature_part_family_count: creature_scene.creature_part_family_count,
        creature_mixed_assembly_count: creature_scene.creature_mixed_assembly_count,
        creature_shared_mesh_handle_count: creature_scene.creature_shared_mesh_handle_count,
        selection_ref_count: snapshot.selection_refs.len(),
        dirty_chunk_count: snapshot.dirty_regions.len(),
        estimated_resident_bytes: fvr03_estimated_resident_bytes(
            tile_mesh_count,
            snapshot.visible_chunks.len(),
        ),
        draw_radius_chunks: settings.draw_radius_chunks,
        target_fps: settings.target_fps,
        performance_artifact_path: None,
        no_renderer_authority_over_world_truth: true,
        material_counts,
        average_resource_bias: fvr05_average_resource_bias(&tile_summaries_by_tile),
        average_hazard_pressure: fvr05_average_hazard_pressure(&tile_summaries_by_tile),
        production_dressing_count: polish.dressing_count,
        production_vfx_marker_count: polish.vfx_marker_count,
        production_gpu_vfx_emitter_count: polish.gpu_vfx_emitter_count,
        production_vfx_budget_state: polish.vfx_budget_state,
        production_visuals_display_only: polish.display_only,
        production_vfx_uses_hanabi_gpu_particles: polish.uses_hanabi_gpu_particles,
        mesh_stats: terrain_receipt.mesh_stats,
        visible_tiles,
        visible_chunks,
        tile_summaries_by_tile,
        creature_refs_by_tile: runtime_state
            .creatures
            .iter()
            .map(|creature| (creature.tile, creature.stable_ref))
            .collect(),
        selection_positions_by_raw_id: runtime_state
            .creatures
            .iter()
            .map(|creature| {
                (
                    creature.visual.stable_id.raw(),
                    Vec3::new(
                        creature.tile.x as f32 + 0.5,
                        1.52,
                        creature.tile.z as f32 + 0.5,
                    ),
                )
            })
            .collect(),
    };
    world.insert_resource(terrain_scene);
    (scene, creature_scene)
}

fn install_fvr04_runtime_scene_resources(
    world: &mut World,
    scene: Fvr03ProductionVoxelSceneResource,
    creatures: Fvr04ProductionCreatureSceneResource,
) {
    world.insert_resource(scene);
    world.insert_resource(creatures);
}

fn create_fvr03_selection_material(
    app: &mut App,
    palette: &[Fvr03ProductionVoxelMaterialEntry],
) -> Handle<StandardMaterial> {
    let material = palette
        .iter()
        .find(|entry| entry.kind == Fvr03ProductionVoxelMaterialKind::Selection)
        .expect("FVR03 selection material exists")
        .standard_material();
    app.world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(material)
}

fn prepare_fvr03_chunk_tiles(
    snapshot: &PersistentVoxelWorldSnapshot,
    procedural_config: ProceduralWorldConfig,
    settings: &Fvr03ProductionVoxelRendererSettings,
    chunk: VoxelChunkCoord,
    visible_tiles: &mut BTreeSet<VoxelTileCoord>,
    tile_summaries_by_tile: &mut BTreeMap<VoxelTileCoord, Fvr05ProductionTileSummary>,
    material_counts: &mut BTreeMap<Fvr03ProductionVoxelMaterialKind, usize>,
    terrain_samples: &mut ProductionTerrainSampleMap,
) -> Result<usize, GameAppShellError> {
    let chunk_tile_size = i32::from(snapshot.profile_budget.chunk_tile_size);
    let base_x = chunk.x * chunk_tile_size;
    let base_z = chunk.z * chunk_tile_size;
    let stride = usize::from(settings.tile_stride.max(1));
    let mut count = 0_usize;
    for dz in (0..chunk_tile_size).step_by(stride) {
        for dx in (0..chunk_tile_size).step_by(stride) {
            let tile = VoxelTileCoord::new(base_x + dx, base_z + dz);
            let sample = alife_world::sample_procedural_terrain_tile(
                procedural_config,
                ProceduralTileCoord::from(tile),
            )?;
            let material = fvr03_material_kind(sample.material, tile);
            let height = fvr03_tile_height(
                sample.material,
                sample.resource_bias,
                sample.hazard_pressure,
                sample.roughness,
            );
            let stable_ref = snapshot.lookup_tile(tile).unwrap_or(StableVoxelObjectRef {
                kind: StableVoxelRefKind::Tile,
                stable_id: None,
                chunk,
                tile: Some(tile),
            });
            visible_tiles.insert(tile);
            *material_counts.entry(material).or_default() += 1;
            tile_summaries_by_tile.insert(
                tile,
                Fvr05ProductionTileSummary {
                    tile,
                    chunk,
                    material,
                    height_units: height,
                    resource_bias: sample.resource_bias,
                    hazard_pressure: sample.hazard_pressure,
                    stable_ref,
                },
            );
            terrain_samples.insert(
                tile,
                ProductionTerrainSample {
                    tile,
                    material,
                    center_x: tile.x as f32 + 0.5,
                    center_z: tile.z as f32 + 0.5,
                    height: fvr09_visual_height_bucket(height),
                    resource_bias: sample.resource_bias,
                    hazard_pressure: sample.hazard_pressure,
                    visual_bucket: fvr10_terrain_variation_bucket(
                        material,
                        tile,
                        sample.resource_bias,
                        sample.hazard_pressure,
                    ),
                },
            );
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}

fn spawn_fvr11_layered_terrain_meshes(
    world: &mut World,
    materials: &TerrainMaterialLibrary,
    settings: &Fvr03ProductionVoxelRendererSettings,
    snapshot: &PersistentVoxelWorldSnapshot,
    terrain_samples: &ProductionTerrainSampleMap,
    build: TerrainMeshBuild,
    tile_mesh_count: usize,
) -> Fvr11TerrainSpawnReceipt {
    let started = Instant::now();
    let terrain_stats = build.stats.clone();
    let top_layer_count = build
        .layers
        .iter()
        .filter(|layer| layer.role == Fvr11TerrainSurfaceRole::Top)
        .count();
    let cliff_layer_count = build
        .layers
        .iter()
        .filter(|layer| layer.role == Fvr11TerrainSurfaceRole::Cliff)
        .count();
    let water_layer_count = build
        .layers
        .iter()
        .filter(|layer| layer.role == Fvr11TerrainSurfaceRole::Water)
        .count();
    for layer in build.layers {
        let role = layer.role;
        let material = layer.material;
        let material_handle = materials.handle_for(role, material);
        let source_tile_count = layer.source_tile_count;
        let mesh_handle = world.resource_mut::<Assets<Mesh>>().add(layer.mesh);
        let name = Name::new(format!(
            "A-Life FVR11 terrain {} {}",
            match role {
                Fvr11TerrainSurfaceRole::Top => "top",
                Fvr11TerrainSurfaceRole::Cliff => "cliff",
                Fvr11TerrainSurfaceRole::Transition => "transition",
                Fvr11TerrainSurfaceRole::Water => "water",
            },
            material.label()
        ));
        let marker = Fvr11ProductionTerrainLayer {
            role,
            material,
            source_tile_count,
            display_only: true,
            no_renderer_authority_over_world_actions_or_cognition: true,
        };
        if matches!(
            role,
            Fvr11TerrainSurfaceRole::Top | Fvr11TerrainSurfaceRole::Water
        ) {
            world.spawn((
                name,
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                Transform::default(),
                marker,
                Fvr03ProductionVoxelTerrainBatch {
                    material,
                    tile_count: source_tile_count,
                },
                Fvr04ProductionRuntimeSceneRoot,
            ));
        } else {
            world.spawn((
                name,
                Mesh3d(mesh_handle),
                MeshMaterial3d(material_handle),
                Transform::default(),
                marker,
                Fvr04ProductionRuntimeSceneRoot,
            ));
        }
    }
    let visible_voxels = terrain_samples
        .values()
        .map(fvr09_visible_voxels_for_tile)
        .sum::<usize>()
        .max(tile_mesh_count);
    let naive_visible_faces = tile_mesh_count.saturating_mul(6);
    let emitted_quads = terrain_stats
        .top_quads
        .saturating_add(terrain_stats.cliff_quads)
        .saturating_add(terrain_stats.transition_edges)
        .saturating_add(terrain_stats.water_quads)
        .clamp(1, naive_visible_faces.max(1));
    let face_reduction_ratio = if emitted_quads == 0 {
        0.0
    } else {
        naive_visible_faces as f32 / emitted_quads as f32
    };
    let dirty_source = snapshot.dirty_regions.len();
    let dirty_chunks = dirty_source.min(settings.remesh_budget_chunks_per_frame);
    let cached_chunks = snapshot.visible_chunks.len().saturating_sub(dirty_chunks);
    let skipped_chunks = dirty_source.saturating_sub(dirty_chunks);
    let variation_bucket_count = terrain_samples
        .values()
        .map(|sample| (sample.material, sample.visual_bucket))
        .collect::<BTreeSet<_>>()
        .len();
    Fvr11TerrainSpawnReceipt {
        mesh_stats: Fvr09TerrainMeshStats {
            mode: Fvr09MesherMode::LayeredGridQuads,
            visible_voxels,
            naive_visible_faces,
            emitted_quads,
            face_reduction_ratio,
            remesh_time_micros: started.elapsed().as_micros(),
            dirty_chunks,
            cached_chunks,
            skipped_chunks,
            remesh_budget_chunks_per_frame: settings.remesh_budget_chunks_per_frame,
            material_palette_version: settings.material_palette_version,
            vertex_color_face_variation: true,
            top_side_color_separation: true,
            variation_bucket_count,
            cache_key: fvr09_mesh_cache_key(snapshot, settings),
        },
        top_layer_count,
        cliff_layer_count,
        transition_edge_count: terrain_stats.transition_edges,
        water_layer_count,
        confetti_detail_quad_count: terrain_stats.confetti_detail_quads,
    }
}

fn spawn_fvr05_overlay_batches(
    world: &mut World,
    plans: Vec<Fvr04OverlaySpawnPlan>,
    materials: &BTreeMap<Fvr05ProductionOverlayKind, Handle<StandardMaterial>>,
) {
    let mut cells_by_kind = BTreeMap::new();
    for plan in plans {
        let Fvr04OverlaySpawnPlan {
            kind,
            cells,
            visible,
        } = plan;
        if visible && !cells.is_empty() {
            spawn_fvr05_overlay_batch(world, kind, &cells, materials, Visibility::Visible);
        }
        cells_by_kind.insert(kind, cells);
    }
    world.insert_resource(Fvr05OverlayGeometryCache { cells_by_kind });
}

fn spawn_fvr05_overlay_batch(
    world: &mut World,
    kind: Fvr05ProductionOverlayKind,
    cells: &[Fvr05OverlayCell],
    materials: &BTreeMap<Fvr05ProductionOverlayKind, Handle<StandardMaterial>>,
    visibility: Visibility,
) {
    if cells.is_empty() {
        return;
    }
    let mesh = fvr05_batched_overlay_mesh(cells);
    let mesh_handle = world.resource_mut::<Assets<Mesh>>().add(mesh);
    let material_handle = materials
        .get(&kind)
        .expect("prepared FVR05 overlay material exists")
        .clone();
    world.spawn((
        Name::new(format!("A-Life FVR05 overlay {}", kind.label())),
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
        Transform::default(),
        visibility,
        Fvr05ProductionOverlayBatch {
            kind,
            cell_count: cells.len(),
        },
        Fvr04ProductionRuntimeSceneRoot,
    ));
}

fn fvr05_overlay_cells(
    kind: Fvr05ProductionOverlayKind,
    settings: &Fvr03ProductionVoxelRendererSettings,
    tile_summaries: &BTreeMap<VoxelTileCoord, Fvr05ProductionTileSummary>,
    visible_chunks: &BTreeSet<VoxelChunkCoord>,
    creatures: &[Fvr04CreatureVisualRecord],
    chunk_tile_size: u16,
) -> Vec<Fvr05OverlayCell> {
    let tile_footprint = f32::from(settings.tile_stride.max(1)) * 0.96;
    match kind {
        Fvr05ProductionOverlayKind::Resources => tile_summaries
            .values()
            .filter(|tile| tile.resource_bias >= 0.38)
            .map(|tile| fvr05_tile_overlay_cell(tile, tile_footprint, 0.055))
            .collect(),
        Fvr05ProductionOverlayKind::Danger => tile_summaries
            .values()
            .filter(|tile| tile.hazard_pressure >= 0.30)
            .map(|tile| fvr05_tile_overlay_cell(tile, tile_footprint, 0.070))
            .collect(),
        Fvr05ProductionOverlayKind::Pheromones => tile_summaries
            .values()
            .filter(|tile| {
                (tile.resource_bias * 0.65 + tile.hazard_pressure * 0.35) >= 0.34
                    && (tile.tile.x + tile.tile.z).rem_euclid(2) == 0
            })
            .map(|tile| fvr05_tile_overlay_cell(tile, tile_footprint, 0.085))
            .collect(),
        Fvr05ProductionOverlayKind::Energy => creatures
            .iter()
            .filter(|creature| creature.visual.cues.energy.value >= 0.45)
            .map(|creature| fvr05_creature_overlay_cell(creature, 0.92, 1.88))
            .collect(),
        Fvr05ProductionOverlayKind::Age => creatures
            .iter()
            .filter(|creature| creature.visual.cues.sleep_pressure.value >= 0.35)
            .map(|creature| fvr05_creature_overlay_cell(creature, 0.74, 2.04))
            .collect(),
        Fvr05ProductionOverlayKind::Fertility => creatures
            .iter()
            .filter(|creature| creature.reproductive_drive >= 0.35)
            .map(|creature| fvr05_creature_overlay_cell(creature, 0.80, 2.18))
            .collect(),
        Fvr05ProductionOverlayKind::Territory => creatures
            .iter()
            .filter(|creature| creature.social_affinity.abs() >= 0.20)
            .map(|creature| fvr05_creature_overlay_cell(creature, 1.42, 0.10))
            .collect(),
        Fvr05ProductionOverlayKind::Neural => creatures
            .iter()
            .filter(|creature| {
                creature.visual.endocrine.dopamine >= 0.25
                    || creature.visual.endocrine.cortisol >= 0.25
            })
            .map(|creature| fvr05_creature_overlay_cell(creature, 0.58, 2.34))
            .collect(),
        Fvr05ProductionOverlayKind::Residency => creatures
            .iter()
            .map(|creature| fvr05_creature_overlay_cell(creature, 0.46, 2.50))
            .collect(),
        Fvr05ProductionOverlayKind::BackendTiming
        | Fvr05ProductionOverlayKind::ChunkBoundaries
        | Fvr05ProductionOverlayKind::LodBudget
        | Fvr05ProductionOverlayKind::Persistence => visible_chunks
            .iter()
            .map(|chunk| fvr05_chunk_overlay_cell(*chunk, chunk_tile_size, kind))
            .collect(),
    }
}

fn fvr05_tile_overlay_cell(
    tile: &Fvr05ProductionTileSummary,
    footprint: f32,
    y_offset: f32,
) -> Fvr05OverlayCell {
    Fvr05OverlayCell {
        center_x: tile.tile.x as f32 + 0.5,
        center_z: tile.tile.z as f32 + 0.5,
        y: tile.height_units + y_offset,
        footprint,
    }
}

fn fvr05_creature_overlay_cell(
    creature: &Fvr04CreatureVisualRecord,
    footprint: f32,
    y: f32,
) -> Fvr05OverlayCell {
    Fvr05OverlayCell {
        center_x: creature.tile.x as f32 + 0.5,
        center_z: creature.tile.z as f32 + 0.5,
        y,
        footprint,
    }
}

fn fvr05_chunk_overlay_cell(
    chunk: VoxelChunkCoord,
    chunk_tile_size: u16,
    kind: Fvr05ProductionOverlayKind,
) -> Fvr05OverlayCell {
    let size = f32::from(chunk_tile_size);
    let y = match kind {
        Fvr05ProductionOverlayKind::ChunkBoundaries => 0.05,
        Fvr05ProductionOverlayKind::LodBudget => 0.12,
        Fvr05ProductionOverlayKind::BackendTiming => 0.18,
        Fvr05ProductionOverlayKind::Persistence => 0.24,
        _ => 0.08,
    };
    Fvr05OverlayCell {
        center_x: chunk.x as f32 * size + size * 0.5,
        center_z: chunk.z as f32 * size + size * 0.5,
        y,
        footprint: size * 0.94,
    }
}

fn fvr05_overlay_material(kind: Fvr05ProductionOverlayKind) -> StandardMaterial {
    let rgba = match kind {
        Fvr05ProductionOverlayKind::Resources => [0.40, 1.00, 0.76, 0.34],
        Fvr05ProductionOverlayKind::Danger => [1.00, 0.15, 0.18, 0.36],
        Fvr05ProductionOverlayKind::Pheromones => [0.96, 0.42, 0.72, 0.28],
        Fvr05ProductionOverlayKind::Energy => [1.00, 0.86, 0.18, 0.40],
        Fvr05ProductionOverlayKind::Age => [0.62, 0.82, 1.00, 0.34],
        Fvr05ProductionOverlayKind::Fertility => [0.76, 0.54, 1.00, 0.36],
        Fvr05ProductionOverlayKind::Territory => [0.18, 0.95, 0.84, 0.30],
        Fvr05ProductionOverlayKind::Neural => [0.94, 0.28, 0.90, 0.38],
        Fvr05ProductionOverlayKind::Residency => [0.46, 0.72, 1.00, 0.36],
        Fvr05ProductionOverlayKind::BackendTiming => [0.20, 0.86, 1.00, 0.22],
        Fvr05ProductionOverlayKind::ChunkBoundaries => [1.00, 1.00, 1.00, 0.18],
        Fvr05ProductionOverlayKind::LodBudget => [0.54, 1.00, 0.38, 0.20],
        Fvr05ProductionOverlayKind::Persistence => [0.96, 0.96, 0.80, 0.22],
    };
    StandardMaterial {
        base_color: Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3]),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.52,
        cull_mode: None,
        ..default()
    }
}

fn fvr05_batched_overlay_mesh(cells: &[Fvr05OverlayCell]) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::with_capacity(cells.len() * 24);
    let mut normals = Vec::<[f32; 3]>::with_capacity(cells.len() * 24);
    let mut uvs = Vec::<[f32; 2]>::with_capacity(cells.len() * 24);
    let mut indices = Vec::<u32>::with_capacity(cells.len() * 36);
    for cell in cells {
        fvr03_append_cuboid(
            &mut positions,
            &mut normals,
            &mut uvs,
            &mut indices,
            Vec3::new(cell.center_x, cell.y, cell.center_z),
            Vec3::new(cell.footprint, 0.035, cell.footprint),
        );
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn spawn_fvr07_production_visual_polish(
    world: &mut World,
    settings: &Fvr03ProductionVoxelRendererSettings,
    dressing_spawns: Vec<ProductionTerrainDressingSpawn>,
    vfx_spawns: Vec<Fvr07VfxSpawn>,
    dressing_library: &TerrainDressingLibrary,
    unit_mesh: &Option<Handle<Mesh>>,
    vfx_materials: &BTreeMap<Fvr07ProductionVfxKind, Handle<StandardMaterial>>,
) -> Fvr07ProductionPolishSummary {
    for spawn in &dressing_spawns {
        let material = dressing_library.material(spawn.kind);
        let mesh = dressing_library.mesh(spawn.kind);
        let mut transform = Transform::from_translation(spawn.translation);
        transform.scale = spawn.scale;
        transform.rotation = Quat::from_rotation_y(spawn.yaw_radians);
        world.spawn((
            Name::new(format!(
                "A-Life FVR07 production dressing {} {}:{}",
                spawn.kind.label(),
                spawn.tile.x,
                spawn.tile.z
            )),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform,
            Fvr07ProductionVisualDressing {
                kind: spawn.kind,
                tile: spawn.tile,
                display_only: spawn.display_only,
                no_renderer_authority_over_actions_or_cognition: spawn
                    .no_renderer_authority_over_actions_or_cognition,
            },
            Fvr04ProductionRuntimeSceneRoot,
        ));
    }

    let vfx_marker_count = unit_mesh.as_ref().map_or(0, |unit_mesh| {
        let mut count = 0;
        for spawn in &vfx_spawns {
            let Some(material) = vfx_materials.get(&spawn.kind).cloned() else {
                continue;
            };
            let mut transform = Transform::from_translation(spawn.translation);
            transform.scale = spawn.scale;
            world.spawn((
                Name::new(format!(
                    "A-Life FVR07 display-only VFX {}",
                    spawn.kind.label()
                )),
                Mesh3d(unit_mesh.clone()),
                MeshMaterial3d(material),
                transform,
                bevy::light::NotShadowCaster,
                Fvr07ProductionGpuVfxMarker {
                    kind: spawn.kind,
                    tile: spawn.tile,
                    stable_id: spawn.stable_id,
                    follows_creature: spawn.follows_creature,
                    display_only: true,
                    no_renderer_authority_over_actions_or_cognition: true,
                    budget_state: settings.production_vfx_budget_state,
                    base_translation: spawn.translation,
                    base_scale: spawn.scale,
                    phase: spawn.phase,
                },
                Fvr04ProductionRuntimeSceneRoot,
            ));
            count += 1;
        }
        count
    });

    let gpu_vfx_emitter_count = spawn_fvr07_hanabi_gpu_vfx_emitters(world, settings, &vfx_spawns);
    Fvr07ProductionPolishSummary {
        dressing_count: dressing_spawns.len(),
        vfx_marker_count,
        gpu_vfx_emitter_count,
        vfx_budget_state: settings.production_vfx_budget_state,
        display_only: true,
        uses_hanabi_gpu_particles: cfg!(feature = "vfx-hanabi"),
    }
}

fn fvr07_vfx_materials(
    world: &mut World,
) -> BTreeMap<Fvr07ProductionVfxKind, Handle<StandardMaterial>> {
    [
        Fvr07ProductionVfxKind::PheromoneTrail,
        Fvr07ProductionVfxKind::SporeDrift,
        Fvr07ProductionVfxKind::SleepGlow,
        Fvr07ProductionVfxKind::DangerHazardParticles,
        Fvr07ProductionVfxKind::EatingResourceEffect,
        Fvr07ProductionVfxKind::BirthDeathEffect,
        Fvr07ProductionVfxKind::WaterDecayAmbient,
        Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse,
    ]
    .into_iter()
    .map(|kind| {
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(fvr07_vfx_material(kind));
        (kind, material)
    })
    .collect()
}

fn fvr07_vfx_material(kind: Fvr07ProductionVfxKind) -> StandardMaterial {
    let rgba = fvr07_vfx_color(kind);
    StandardMaterial {
        base_color: Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3]),
        alpha_mode: AlphaMode::Blend,
        perceptual_roughness: 0.42,
        cull_mode: None,
        unlit: true,
        ..default()
    }
}

fn fvr07_vfx_spawns(
    settings: &Fvr03ProductionVoxelRendererSettings,
    tile_summaries: &BTreeMap<VoxelTileCoord, Fvr05ProductionTileSummary>,
    creatures: &[Fvr04CreatureVisualRecord],
    selected: Option<StableVoxelObjectRef>,
) -> Vec<Fvr07VfxSpawn> {
    let mut spawns = Vec::with_capacity(settings.production_vfx_marker_cap);
    let per_kind = (settings.production_vfx_marker_cap / 8).clamp(1, 12);
    for kind in [
        Fvr07ProductionVfxKind::PheromoneTrail,
        Fvr07ProductionVfxKind::SporeDrift,
        Fvr07ProductionVfxKind::DangerHazardParticles,
        Fvr07ProductionVfxKind::EatingResourceEffect,
        Fvr07ProductionVfxKind::WaterDecayAmbient,
    ] {
        for tile in fvr07_tiles_for_vfx(kind, tile_summaries)
            .into_iter()
            .take(per_kind)
        {
            fvr07_push_tile_vfx(&mut spawns, settings, kind, tile);
        }
    }

    for creature in fvr07_creatures_for_vfx(Fvr07ProductionVfxKind::SleepGlow, creatures)
        .into_iter()
        .take(per_kind)
    {
        fvr07_push_creature_vfx(
            &mut spawns,
            settings,
            Fvr07ProductionVfxKind::SleepGlow,
            creature,
        );
    }
    for creature in fvr07_creatures_for_vfx(Fvr07ProductionVfxKind::BirthDeathEffect, creatures)
        .into_iter()
        .take(per_kind)
    {
        fvr07_push_creature_vfx(
            &mut spawns,
            settings,
            Fvr07ProductionVfxKind::BirthDeathEffect,
            creature,
        );
    }

    let neural_tile = selected
        .and_then(|selection| selection.tile)
        .and_then(|tile| tile_summaries.get(&tile))
        .or_else(|| tile_summaries.values().next());
    if let Some(tile) = neural_tile {
        fvr07_push_tile_vfx(
            &mut spawns,
            settings,
            Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse,
            tile,
        );
    }
    if spawns
        .iter()
        .all(|spawn| spawn.kind != Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse)
    {
        if let Some(creature) = creatures.first() {
            fvr07_push_creature_vfx(
                &mut spawns,
                settings,
                Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse,
                creature,
            );
        }
    }
    spawns.truncate(settings.production_vfx_marker_cap);
    spawns
}

fn fvr07_tiles_for_vfx<'a>(
    kind: Fvr07ProductionVfxKind,
    tile_summaries: &'a BTreeMap<VoxelTileCoord, Fvr05ProductionTileSummary>,
) -> Vec<&'a Fvr05ProductionTileSummary> {
    let mut tiles = tile_summaries
        .values()
        .filter(|tile| match kind {
            Fvr07ProductionVfxKind::PheromoneTrail => {
                tile.resource_bias * 0.65 + tile.hazard_pressure * 0.35 >= 0.34
                    && (tile.tile.x + tile.tile.z).rem_euclid(2) == 0
            }
            Fvr07ProductionVfxKind::SporeDrift => {
                matches!(
                    tile.material,
                    Fvr03ProductionVoxelMaterialKind::Decay
                        | Fvr03ProductionVoxelMaterialKind::Resource
                ) && fvr07_tile_hash(tile.tile) % 3 == 0
            }
            Fvr07ProductionVfxKind::DangerHazardParticles => {
                tile.hazard_pressure >= 0.30
                    || matches!(
                        tile.material,
                        Fvr03ProductionVoxelMaterialKind::Hazard
                            | Fvr03ProductionVoxelMaterialKind::Decay
                    )
            }
            Fvr07ProductionVfxKind::EatingResourceEffect => {
                matches!(tile.material, Fvr03ProductionVoxelMaterialKind::Resource)
                    || tile.resource_bias >= 0.42
            }
            Fvr07ProductionVfxKind::WaterDecayAmbient => {
                matches!(
                    tile.material,
                    Fvr03ProductionVoxelMaterialKind::Water
                        | Fvr03ProductionVoxelMaterialKind::Decay
                )
            }
            Fvr07ProductionVfxKind::SleepGlow
            | Fvr07ProductionVfxKind::BirthDeathEffect
            | Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse => false,
        })
        .collect::<Vec<_>>();
    if tiles.is_empty() {
        if let Some(tile) = tile_summaries.values().next() {
            tiles.push(tile);
        }
    }
    tiles
}

fn fvr07_creatures_for_vfx<'a>(
    kind: Fvr07ProductionVfxKind,
    creatures: &'a [Fvr04CreatureVisualRecord],
) -> Vec<&'a Fvr04CreatureVisualRecord> {
    let mut candidates = creatures
        .iter()
        .filter(|creature| match kind {
            Fvr07ProductionVfxKind::SleepGlow => {
                creature.visual.cues.sleep_pressure.value >= 0.25
                    || matches!(
                        creature.visual.animation,
                        CreatureAnimationState::Sleeping | CreatureAnimationState::Resting
                    )
            }
            Fvr07ProductionVfxKind::BirthDeathEffect => {
                creature.reproductive_drive >= 0.28
                    || matches!(
                        creature.visual.animation,
                        CreatureAnimationState::Hurt | CreatureAnimationState::Afraid
                    )
            }
            _ => false,
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        if let Some(creature) = creatures.first() {
            candidates.push(creature);
        }
    }
    candidates
}

fn fvr07_push_tile_vfx(
    spawns: &mut Vec<Fvr07VfxSpawn>,
    settings: &Fvr03ProductionVoxelRendererSettings,
    kind: Fvr07ProductionVfxKind,
    tile: &Fvr05ProductionTileSummary,
) {
    if spawns.len() >= settings.production_vfx_marker_cap {
        return;
    }
    let phase = fvr07_hash_phase(tile.tile);
    spawns.push(Fvr07VfxSpawn {
        kind,
        tile: Some(tile.tile),
        stable_id: tile.stable_ref.stable_id,
        follows_creature: false,
        translation: Vec3::new(
            tile.tile.x as f32 + 0.5,
            tile.height_units + fvr07_vfx_y_offset(kind),
            tile.tile.z as f32 + 0.5,
        ),
        scale: fvr07_vfx_scale(kind, settings),
        color: fvr07_vfx_color(kind),
        phase,
    });
}

fn fvr07_push_creature_vfx(
    spawns: &mut Vec<Fvr07VfxSpawn>,
    settings: &Fvr03ProductionVoxelRendererSettings,
    kind: Fvr07ProductionVfxKind,
    creature: &Fvr04CreatureVisualRecord,
) {
    if spawns.len() >= settings.production_vfx_marker_cap {
        return;
    }
    let phase = fvr07_hash_phase(creature.tile);
    spawns.push(Fvr07VfxSpawn {
        kind,
        tile: Some(creature.tile),
        stable_id: Some(creature.visual.stable_id),
        follows_creature: true,
        translation: Vec3::new(
            creature.tile.x as f32 + 0.5,
            2.08,
            creature.tile.z as f32 + 0.5,
        ),
        scale: fvr07_vfx_scale(kind, settings),
        color: fvr07_vfx_color(kind),
        phase,
    });
}

fn fvr07_vfx_y_offset(kind: Fvr07ProductionVfxKind) -> f32 {
    match kind {
        Fvr07ProductionVfxKind::PheromoneTrail => 0.28,
        Fvr07ProductionVfxKind::SporeDrift => 0.72,
        Fvr07ProductionVfxKind::DangerHazardParticles => 0.62,
        Fvr07ProductionVfxKind::EatingResourceEffect => 0.48,
        Fvr07ProductionVfxKind::WaterDecayAmbient => 0.34,
        Fvr07ProductionVfxKind::SleepGlow => 0.58,
        Fvr07ProductionVfxKind::BirthDeathEffect => 0.70,
        Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse => 0.76,
    }
}

fn fvr07_vfx_scale(
    kind: Fvr07ProductionVfxKind,
    settings: &Fvr03ProductionVoxelRendererSettings,
) -> Vec3 {
    let profile_scale = if settings.minimum_floor { 0.96 } else { 1.08 };
    let base = match kind {
        Fvr07ProductionVfxKind::PheromoneTrail => Vec3::new(1.00, 0.06, 0.30),
        Fvr07ProductionVfxKind::SporeDrift => Vec3::new(0.36, 0.36, 0.36),
        Fvr07ProductionVfxKind::SleepGlow => Vec3::new(0.14, 0.040, 0.14),
        Fvr07ProductionVfxKind::DangerHazardParticles => Vec3::new(0.46, 0.52, 0.46),
        Fvr07ProductionVfxKind::EatingResourceEffect => Vec3::new(0.34, 0.44, 0.34),
        Fvr07ProductionVfxKind::BirthDeathEffect => Vec3::new(0.22, 0.050, 0.22),
        Fvr07ProductionVfxKind::WaterDecayAmbient => Vec3::new(0.64, 0.07, 0.64),
        Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse => Vec3::new(0.28, 0.045, 0.28),
    };
    base * profile_scale
}

fn fvr07_vfx_color(kind: Fvr07ProductionVfxKind) -> [f32; 4] {
    match kind {
        Fvr07ProductionVfxKind::PheromoneTrail => [0.95, 0.30, 0.74, 0.58],
        Fvr07ProductionVfxKind::SporeDrift => [0.62, 0.95, 0.70, 0.54],
        Fvr07ProductionVfxKind::SleepGlow => [0.95, 0.66, 0.18, 0.24],
        Fvr07ProductionVfxKind::DangerHazardParticles => [1.00, 0.10, 0.24, 0.68],
        Fvr07ProductionVfxKind::EatingResourceEffect => [1.00, 0.82, 0.20, 0.62],
        Fvr07ProductionVfxKind::BirthDeathEffect => [0.74, 0.38, 0.86, 0.30],
        Fvr07ProductionVfxKind::WaterDecayAmbient => [0.18, 0.70, 0.92, 0.48],
        Fvr07ProductionVfxKind::SelectedCreatureNeuralPulse => [0.98, 0.82, 0.22, 0.34],
    }
}

fn fvr07_tile_hash(tile: VoxelTileCoord) -> u32 {
    let x = tile.x as i64 as u64;
    let z = tile.z as i64 as u64;
    x.wrapping_mul(0x9E37_79B1_85EB_CA87)
        .wrapping_add(z.wrapping_mul(0xC2B2_AE3D_27D4_EB4F))
        .rotate_left(17) as u32
}

fn fvr07_hash_phase(tile: VoxelTileCoord) -> f32 {
    (fvr07_tile_hash(tile) % 10_000) as f32 / 10_000.0
}

#[cfg(any(test, feature = "vfx-hanabi"))]
#[derive(Debug, Clone, Copy, PartialEq)]
struct Fvr07HanabiBudget {
    emitter_cap: usize,
    capacity: u32,
    rate: f32,
    alpha_scale: f32,
    particle_size: f32,
}

#[cfg(any(test, feature = "vfx-hanabi"))]
fn fvr07_hanabi_budget(profile_id: ProductionFrontendProfileId) -> Fvr07HanabiBudget {
    match profile_id {
        ProductionFrontendProfileId::MinimumSettings30x30 => Fvr07HanabiBudget {
            emitter_cap: 2,
            capacity: 64,
            rate: 2.5,
            alpha_scale: 0.42,
            particle_size: 0.085,
        },
        ProductionFrontendProfileId::MinSpecComfort1080p => Fvr07HanabiBudget {
            emitter_cap: 4,
            capacity: 128,
            rate: 5.0,
            alpha_scale: 0.45,
            particle_size: 0.10,
        },
        ProductionFrontendProfileId::Balanced1080p => Fvr07HanabiBudget {
            emitter_cap: 6,
            capacity: 192,
            rate: 8.0,
            alpha_scale: 0.48,
            particle_size: 0.11,
        },
        ProductionFrontendProfileId::HighSpecScaleUp => Fvr07HanabiBudget {
            emitter_cap: 8,
            capacity: 256,
            rate: 12.0,
            alpha_scale: 0.50,
            particle_size: 0.12,
        },
        ProductionFrontendProfileId::ResearchScale => Fvr07HanabiBudget {
            emitter_cap: 4,
            capacity: 128,
            rate: 4.0,
            alpha_scale: 0.44,
            particle_size: 0.095,
        },
    }
}

#[cfg(feature = "vfx-hanabi")]
fn spawn_fvr07_hanabi_gpu_vfx_emitters(
    world: &mut World,
    settings: &Fvr03ProductionVoxelRendererSettings,
    vfx_spawns: &[Fvr07VfxSpawn],
) -> usize {
    use bevy_hanabi::prelude::*;

    let budget = fvr07_hanabi_budget(settings.profile_id);
    let mut emitted = 0_usize;
    let mut effects_by_kind = BTreeMap::new();
    for spawn in vfx_spawns.iter().take(budget.emitter_cap) {
        let effect = if let Some(effect) = effects_by_kind.get(&spawn.kind) {
            effect.clone()
        } else {
            let writer = ExprWriter::new();
            let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());
            let init_lifetime =
                SetAttributeModifier::new(Attribute::LIFETIME, writer.lit(1.85).expr());
            let init_pos = SetPositionSphereModifier {
                center: writer.lit(Vec3::ZERO).expr(),
                radius: writer.lit(0.32).expr(),
                dimension: ShapeDimension::Surface,
            };
            let init_vel = SetVelocitySphereModifier {
                center: writer.lit(Vec3::ZERO).expr(),
                speed: writer.lit(0.38).expr(),
            };
            let mut gradient = bevy_hanabi::Gradient::new();
            gradient.add_key(
                0.0,
                bevy::prelude::Vec4::new(
                    spawn.color[0],
                    spawn.color[1],
                    spawn.color[2],
                    spawn.color[3] * budget.alpha_scale,
                ),
            );
            gradient.add_key(
                0.72,
                bevy::prelude::Vec4::new(
                    spawn.color[0],
                    spawn.color[1],
                    spawn.color[2],
                    spawn.color[3] * budget.alpha_scale * 0.62,
                ),
            );
            gradient.add_key(1.0, bevy::prelude::Vec4::splat(0.0));
            let size_gradient =
                bevy_hanabi::Gradient::constant(bevy::prelude::Vec3::splat(budget.particle_size));
            let effect = world.resource_mut::<Assets<EffectAsset>>().add(
                EffectAsset::new(
                    budget.capacity,
                    SpawnerSettings::rate(budget.rate.into()),
                    writer.finish(),
                )
                .with_name(format!("fvr07-{}", spawn.kind.label()))
                .init(init_pos)
                .init(init_vel)
                .init(init_age)
                .init(init_lifetime)
                .render(ColorOverLifetimeModifier::new(gradient))
                .render(SizeOverLifetimeModifier {
                    gradient: size_gradient,
                    screen_space_size: false,
                }),
            );
            effects_by_kind.insert(spawn.kind, effect.clone());
            effect
        };
        world.spawn((
            Name::new(format!(
                "A-Life FVR07 Hanabi GPU VFX {}",
                spawn.kind.label()
            )),
            ParticleEffect::new(effect),
            Transform::from_translation(spawn.translation),
            Fvr07ProductionHanabiVfxEmitter {
                kind: spawn.kind,
                stable_id: spawn.stable_id,
                follows_creature: spawn.follows_creature,
                display_only: true,
                no_renderer_authority_over_actions_or_cognition: true,
                budget_state: settings.production_vfx_budget_state,
            },
            Fvr04ProductionRuntimeSceneRoot,
        ));
        emitted = emitted.saturating_add(1);
    }
    emitted
}

#[cfg(not(feature = "vfx-hanabi"))]
fn spawn_fvr07_hanabi_gpu_vfx_emitters(
    _world: &mut World,
    _settings: &Fvr03ProductionVoxelRendererSettings,
    _vfx_spawns: &[Fvr07VfxSpawn],
) -> usize {
    0
}

fn fvr05_average_resource_bias(
    tile_summaries: &BTreeMap<VoxelTileCoord, Fvr05ProductionTileSummary>,
) -> f32 {
    if tile_summaries.is_empty() {
        return 0.0;
    }
    let total = tile_summaries
        .values()
        .map(|tile| tile.resource_bias)
        .sum::<f32>();
    total / tile_summaries.len() as f32
}

fn fvr05_average_hazard_pressure(
    tile_summaries: &BTreeMap<VoxelTileCoord, Fvr05ProductionTileSummary>,
) -> f32 {
    if tile_summaries.is_empty() {
        return 0.0;
    }
    let total = tile_summaries
        .values()
        .map(|tile| tile.hazard_pressure)
        .sum::<f32>();
    total / tile_summaries.len() as f32
}

fn fvr10_terrain_variation_bucket(
    material: Fvr03ProductionVoxelMaterialKind,
    tile: VoxelTileCoord,
    resource_bias: f32,
    hazard_pressure: f32,
) -> u8 {
    let hash = fvr10_coord_hash(
        tile.x,
        tile.z,
        material.label().bytes().fold(0_u32, |acc, byte| {
            acc.wrapping_mul(31).wrapping_add(u32::from(byte))
        }),
        ((resource_bias * 17.0) as u32) ^ ((hazard_pressure * 31.0) as u32),
    );
    (hash % 5) as u8
}
fn fvr10_coord_hash(x: i32, z: i32, salt_a: u32, salt_b: u32) -> u32 {
    let mut hash = 0x811c9dc5_u32;
    for value in [x as u32, z as u32, salt_a, salt_b] {
        hash ^= value;
        hash = hash.wrapping_mul(0x0100_0193);
        hash ^= hash >> 13;
    }
    hash
}

fn fvr09_visible_voxels_for_tile(tile: &ProductionTerrainSample) -> usize {
    (tile.height.ceil() as usize).max(1)
}

fn fvr09_visual_height_bucket(height: f32) -> f32 {
    (height * 4.0).round().max(1.0) / 4.0
}

fn fvr09_mesh_cache_key(
    snapshot: &PersistentVoxelWorldSnapshot,
    settings: &Fvr03ProductionVoxelRendererSettings,
) -> String {
    let version_sum = snapshot
        .visible_chunks
        .iter()
        .fold(0_u64, |acc, chunk| acc.wrapping_add(chunk.dirty_generation));
    let coord_sum = snapshot.visible_chunks.iter().fold(0_i64, |acc, chunk| {
        acc.wrapping_add((i64::from(chunk.coord.x) << 32) ^ i64::from(chunk.coord.z))
    });
    format!(
        "profile={};palette={};chunk_size={};stride={};chunks={};dirty_regions={};version_sum={};coord_sum={}",
        settings.profile_id.label(),
        settings.material_palette_version,
        snapshot.profile_budget.chunk_tile_size,
        settings.tile_stride,
        snapshot.visible_chunks.len(),
        snapshot.dirty_regions.len(),
        version_sum,
        coord_sum
    )
}

fn fvr03_append_cuboid(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    center: Vec3,
    size: Vec3,
) {
    let half = size * 0.5;
    let min_x = center.x - half.x;
    let max_x = center.x + half.x;
    let min_y = center.y - half.y;
    let max_y = center.y + half.y;
    let min_z = center.z - half.z;
    let max_z = center.z + half.z;
    let faces = [
        (
            [0.0, 1.0, 0.0],
            [
                [min_x, max_y, min_z],
                [max_x, max_y, min_z],
                [max_x, max_y, max_z],
                [min_x, max_y, max_z],
            ],
        ),
        (
            [0.0, -1.0, 0.0],
            [
                [min_x, min_y, max_z],
                [max_x, min_y, max_z],
                [max_x, min_y, min_z],
                [min_x, min_y, min_z],
            ],
        ),
        (
            [1.0, 0.0, 0.0],
            [
                [max_x, min_y, min_z],
                [max_x, min_y, max_z],
                [max_x, max_y, max_z],
                [max_x, max_y, min_z],
            ],
        ),
        (
            [-1.0, 0.0, 0.0],
            [
                [min_x, min_y, max_z],
                [min_x, min_y, min_z],
                [min_x, max_y, min_z],
                [min_x, max_y, max_z],
            ],
        ),
        (
            [0.0, 0.0, 1.0],
            [
                [max_x, min_y, max_z],
                [min_x, min_y, max_z],
                [min_x, max_y, max_z],
                [max_x, max_y, max_z],
            ],
        ),
        (
            [0.0, 0.0, -1.0],
            [
                [min_x, min_y, min_z],
                [max_x, min_y, min_z],
                [max_x, max_y, min_z],
                [min_x, max_y, min_z],
            ],
        ),
    ];
    for (normal, face_positions) in faces {
        let base = positions.len() as u32;
        positions.extend(face_positions);
        normals.extend([normal; 4]);
        uvs.extend([[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]);
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

fn fvr04_creature_root_bundle(
    stable_id: WorldEntityId,
    organism_id: OrganismId,
    tile: VoxelTileCoord,
    transform: Transform,
    mut visual: Fvr04ProductionCreatureVisualMarker,
    mut cute: Fvr09CuteBipedCreatureMarker,
    mut species: Fvr10CreatureSpeciesMarker,
    display_only: bool,
) -> impl bevy::ecs::bundle::Bundle {
    visual.stable_id = stable_id;
    visual.organism_id = organism_id;
    visual.tile = tile;
    cute.stable_id = stable_id;
    species.stable_id = stable_id;
    (
        Name::new(format!(
            "A-Life modular creature assembly stable {}",
            stable_id.raw()
        )),
        transform,
        Visibility::Inherited,
        Fvr03ProductionVoxelCreatureMarker { stable_id, tile },
        visual,
        ProductionCreatureAssemblyRoot {
            stable_id,
            organism_id,
            display_only,
        },
        Fvr04ProductionRuntimeSceneRoot,
        cute,
        species,
    )
}

fn spawn_fvr04_prepared_creature_batch(
    world: &mut World,
    prepared: Fvr04PreparedCreatureBatch,
) -> Fvr04ProductionCreatureSceneResource {
    let Fvr04PreparedCreatureBatch {
        settings,
        creatures,
    } = prepared;
    let mut expression_buffer = Vec::with_capacity(creatures.len());
    let mut stable_lookup_by_raw_id = BTreeMap::new();
    let mut part_families = BTreeSet::new();
    let mut species_archetypes = BTreeSet::new();
    let mut scene_mesh_handles = BTreeSet::new();
    let mut scene_material_handles = BTreeSet::new();
    let mut part_entity_count = 0_usize;
    let mut mixed_assembly_count = 0_usize;

    for creature in creatures {
        let visual = &creature.record.visual;
        species_archetypes.insert(visual.appearance.species_archetype);
        let recipe_families = creature
            .recipe
            .parts
            .values()
            .map(|part| part.source_family)
            .collect::<BTreeSet<_>>();
        mixed_assembly_count += usize::from(recipe_families.len() > 1);
        part_families.extend(recipe_families);
        let root = world
            .spawn(fvr04_creature_root_bundle(
                visual.stable_id,
                visual.organism_id,
                creature.record.tile,
                creature.root_transform,
                creature.root_visual,
                Fvr09CuteBipedCreatureMarker {
                    stable_id: visual.stable_id,
                    visual_profile: "modular-heritable-part-assembly-v1",
                    two_legs: true,
                    visible_face: true,
                    eye_markers: 2,
                    front_back_orientation: true,
                    real_state_driven: true,
                },
                Fvr10CreatureSpeciesMarker {
                    stable_id: visual.stable_id,
                    species_archetype: visual.appearance.species_archetype,
                    species_label: visual.appearance.species_label(),
                    body_plan_signature: visual.appearance.body_plan_signature(),
                    bipedal: true,
                    caveman_furry_design: true,
                    heritable_appearance: true,
                },
                creature.recipe.display_only,
            ))
            .id();
        world
            .resource_mut::<BevyEntityMap>()
            .bind(root, visual.stable_id)
            .expect("validated creature root stable ID must bind");
        let coat_material = creature.coat.material;
        scene_material_handles.insert(coat_material.id());
        for part in creature.parts {
            scene_mesh_handles.insert(part.mesh.id());
            world.spawn((
                Name::new(format!(
                    "A-Life creature part {} {:?}",
                    visual.stable_id.raw(),
                    part.recipe.slot
                )),
                Mesh3d(part.mesh),
                MeshMaterial3d(coat_material.clone()),
                part.transform,
                ChildOf(root),
                ProductionCreaturePartMarker {
                    stable_id: visual.stable_id,
                    family: part.recipe.source_family,
                    asset_id: part.recipe.asset_id.clone(),
                    slot: part.recipe.slot,
                    runtime_group: part.recipe.runtime_group.clone(),
                    authored_matrix: part.recipe.authored_transform,
                    animation: visual.animation,
                },
                ProductionCreaturePartRestTransform(part.transform),
            ));
            part_entity_count += 1;
        }
        stable_lookup_by_raw_id.insert(visual.stable_id.raw(), expression_buffer.len());
        expression_buffer.push(Fvr04CreatureExpressionSample {
            stable_id: visual.stable_id,
            organism_id: visual.organism_id,
            display_label: creature.record.display_label,
            brain_class_id: creature.record.brain_class_id,
            brain_neuron_count: creature.record.brain_neuron_count,
            hunger: visual.cues.hunger.value,
            fatigue: visual.cues.fatigue.value,
            fear: visual.cues.fear.value,
            cortisol: visual.endocrine.cortisol,
            dopamine: visual.endocrine.dopamine,
            reproductive_drive: creature.record.reproductive_drive,
            sleep_pressure: visual.cues.sleep_pressure.value,
            social: ((creature.record.social_affinity + 1.0) * 0.5).clamp(0.0, 1.0),
            fast_memory_count: creature.record.fast_memory_count,
            lifetime_memory_count: creature.record.lifetime_memory_count,
            memory_record_count: creature.record.memory_record_count,
            concept_count: creature.record.concept_count,
            unresolved_gap_count: creature.record.unresolved_gap_count,
            lifetime_learning_enabled: creature.record.lifetime_learning_enabled,
            sleep_phase_raw: creature.record.sleep_phase_raw,
            consolidation_state_raw: creature.record.consolidation_state_raw,
            last_consolidated_tick: creature.record.last_consolidated_tick,
            topology_update_count: creature.record.topology_update_count,
            expression: visual.expression,
            animation: visual.animation,
        });
    }
    Fvr04ProductionCreatureSceneResource {
        schema: FVR04_PRODUCTION_CREATURE_RENDERER_SCHEMA,
        schema_version: FVR04_PRODUCTION_CREATURE_RENDERER_SCHEMA_VERSION,
        requested_population: settings.requested_population,
        rendered_creature_count: expression_buffer.len(),
        material_bucket_count: scene_material_handles.len(),
        mesh_pool_count: scene_mesh_handles.len(),
        lod: settings.lod,
        stable_lookup_by_raw_id,
        no_renderer_authority_over_actions_or_cognition: true,
        expression_buffer_is_read_only_projection: true,
        visual_profile: "modular-heritable-part-assembly-v1",
        mesh_material_version: "modular-textured-part-material-v1",
        species_archetype_count: species_archetypes.len(),
        creature_root_count: expression_buffer.len(),
        creature_part_entity_count: part_entity_count,
        creature_join_cover_count: 0,
        creature_part_family_count: part_families.len(),
        creature_mixed_assembly_count: mixed_assembly_count,
        creature_shared_mesh_handle_count: scene_mesh_handles.len(),
        production_visuals_display_only: true,
        expression_buffer,
    }
}

#[cfg(test)]
fn socket_translation_to_bevy([x, depth, height]: [f32; 3]) -> Vec3 {
    Vec3::new(x, height, -depth)
}

fn geneforge_authored_transform_to_bevy(matrix: [f64; 16]) -> Transform {
    let matrix = matrix.map(|value| value as f32);
    Transform::from_matrix(Mat4::from_cols_array(&[
        matrix[0], matrix[4], matrix[8], matrix[12], matrix[1], matrix[5], matrix[9], matrix[13],
        matrix[2], matrix[6], matrix[10], matrix[14], matrix[3], matrix[7], matrix[11], matrix[15],
    ]))
}

#[cfg(test)]
fn canonical_vec_to_bevy(vector: Vec3) -> Vec3 {
    Vec3::new(vector.x, vector.z, -vector.y)
}

#[cfg(test)]
fn socket_rotation_to_bevy([x, depth, height, w]: [f32; 4]) -> Quat {
    let canonical = Quat::from_xyzw(x, depth, height, w);
    let rotate_basis = |basis| canonical_vec_to_bevy(canonical * basis);
    Quat::from_mat3(&bevy::math::Mat3::from_cols(
        rotate_basis(Vec3::X),
        rotate_basis(Vec3::Z),
        rotate_basis(Vec3::NEG_Y),
    ))
    .normalize()
}

#[cfg(test)]
fn socket_scale_to_bevy([x, depth, height]: [f32; 3]) -> Vec3 {
    Vec3::new(x, height, depth)
}

fn transform_creature_visual_bounds(
    bounds: CreatureVisualBounds,
    transform: Transform,
) -> CreatureVisualBounds {
    let affine = transform.compute_affine();
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for corner in bounds.corners() {
        let point = affine.transform_point3(Vec3::from_array(corner)).to_array();
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    CreatureVisualBounds::new(min, max)
}

#[cfg(test)]
fn socket_transform_to_bevy(
    _slot: CreaturePartSlot,
    socket: SocketFrame,
    local_scale: [f32; 3],
) -> Transform {
    Transform::from_translation(socket_translation_to_bevy(socket.translation))
        .with_rotation(socket_rotation_to_bevy(socket.rotation_xyzw))
        .with_scale(socket_scale_to_bevy(socket.scale) * socket_scale_to_bevy(local_scale))
}

fn fvr04_creature_scale(visual: &CreatureVisualSnapshot, lod: Fvr04CreatureLod) -> Vec3 {
    let fatigue_squash = 1.0 - visual.cues.fatigue.value * 0.18;
    let fear_narrow = 1.0 - visual.cues.fear.value * 0.10;
    let energy = 0.92 + visual.cues.energy.value * 0.14;
    match lod {
        Fvr04CreatureLod::FullVoxel => {
            Vec3::new(1.32 * fear_narrow, 1.32 * fatigue_squash * energy, 1.32)
        }
        Fvr04CreatureLod::CompactVoxel => {
            Vec3::new(1.22 * fear_narrow, 1.22 * fatigue_squash, 1.22)
        }
        Fvr04CreatureLod::ImpostorVoxel => {
            Vec3::new(0.98 * fear_narrow, 0.98 * fatigue_squash, 0.86)
        }
    }
}

fn project_authoritative_creature_root_transform(
    stable_id: WorldEntityId,
    organism_id: OrganismId,
    transform: &mut Transform,
    frame: &LiveBrainPresentationFrame,
) -> bool {
    let Some(object) = frame.object(stable_id) else {
        return false;
    };
    if object.kind != WorldObjectKind::Agent || object.organism_id != Some(organism_id) {
        return false;
    }

    let x = object.position.x.round() + 0.5;
    let z = object.position.z.round() + 0.5;
    if transform.translation.x != x {
        transform.translation.x = x;
    }
    if transform.translation.z != z {
        transform.translation.z = z;
    }
    true
}

fn project_live_world_to_fvr04_creature_roots(world: &mut World) {
    if !world.contains_resource::<LiveBrainPresentationFrameResource>() {
        return;
    }

    world.resource_scope(
        |world, frame: bevy::prelude::Mut<LiveBrainPresentationFrameResource>| {
            if !frame.is_changed() {
                return;
            }
            let mut roots = world.query::<(
                &ProductionCreatureAssemblyRoot,
                &Fvr04ProductionCreatureVisualMarker,
                &mut Transform,
            )>();
            for (root, visual, mut transform) in roots.iter_mut(world) {
                let mut projected = *transform;
                if project_authoritative_creature_root_transform(
                    root.stable_id,
                    visual.organism_id,
                    &mut projected,
                    &frame.current,
                ) && *transform != projected
                {
                    *transform = projected;
                }
            }

            #[cfg(feature = "gpu-runtime")]
            {
                let pending_newborns = {
                    let Some(entity_map) = world.get_resource::<BevyEntityMap>() else {
                        return;
                    };
                    frame
                        .current
                        .objects()
                        .filter(|object| {
                            if object.kind != WorldObjectKind::Agent {
                                return false;
                            }
                            let Some(organism_id) = object.organism_id else {
                                return false;
                            };
                            let Some(entity) = entity_map.bevy_entity(object.id) else {
                                return true;
                            };
                            let root = world.get::<ProductionCreatureAssemblyRoot>(entity);
                            let visual = world.get::<Fvr04ProductionCreatureVisualMarker>(entity);
                            !matches!(
                                (root, visual),
                                (Some(root), Some(visual))
                                    if root.stable_id == object.id
                                        && root.organism_id == organism_id
                                        && visual.stable_id == object.id
                                        && visual.organism_id == organism_id
                            )
                        })
                        .collect::<Vec<_>>()
                };
                if pending_newborns.is_empty() {
                    return;
                }

                let Some(world_seed) = world
                    .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
                    .map(|runtime| runtime.runtime.world_seed())
                else {
                    return;
                };
                let tile_summaries = world
                    .get_resource::<Fvr03ProductionVoxelSceneResource>()
                    .map(|scene| scene.tile_summaries_by_tile.clone())
                    .unwrap_or_default();
                let mut newborns = Vec::with_capacity(pending_newborns.len());
                for object in pending_newborns {
                    let tile = VoxelTileCoord::new(
                        object.position.x.round() as i32,
                        object.position.z.round() as i32,
                    );
                    let chunk = tile_summaries
                        .get(&tile)
                        .map(|summary| summary.chunk)
                        .unwrap_or_else(|| VoxelChunkCoord::new(0, 0));
                    if let Some(record) = fvr04_live_creature_visual_record(
                        &frame.current,
                        world_seed,
                        object,
                        tile,
                        chunk,
                    ) {
                        newborns.push(record);
                    }
                }
                let max_visible = world
                    .get_resource::<Fvr04CreatureSpawnContext>()
                    .map(|context| usize::from(context.settings.max_visible_creatures))
                    .unwrap_or(0);
                newborns.truncate(max_visible);
                if newborns.is_empty() {
                    return;
                }

                let prepared = {
                    let Some(mut context) = world.remove_resource::<Fvr04CreatureSpawnContext>()
                    else {
                        return;
                    };
                    let result = prepare_fvr04_creature_batch(
                        world,
                        &newborns,
                        &tile_summaries,
                        &mut context,
                    );
                    world.insert_resource(context);
                    let Ok(prepared) = result else {
                        return;
                    };
                    prepared
                };
                let added_scene = spawn_fvr04_prepared_creature_batch(world, prepared);
                let added_count = added_scene.rendered_creature_count;
                if let Some(mut scene) =
                    world.get_resource_mut::<Fvr04ProductionCreatureSceneResource>()
                {
                    append_fvr04_creature_scene_resource(&mut scene, added_scene);
                }
                if let Some(mut scene) =
                    world.get_resource_mut::<Fvr03ProductionVoxelSceneResource>()
                {
                    scene.creature_render_count =
                        scene.creature_render_count.saturating_add(added_count);
                    scene.creature_root_count =
                        scene.creature_root_count.saturating_add(added_count);
                    for newborn in newborns.into_iter().take(added_count) {
                        scene
                            .creature_refs_by_tile
                            .insert(newborn.tile, newborn.stable_ref);
                        scene.selection_positions_by_raw_id.insert(
                            newborn.visual.stable_id.raw(),
                            Vec3::new(
                                newborn.tile.x as f32 + 0.5,
                                1.52,
                                newborn.tile.z as f32 + 0.5,
                            ),
                        );
                    }
                }
            }
        },
    );
}

#[cfg(feature = "gpu-runtime")]
fn fvr04_live_creature_visual_record(
    frame: &LiveBrainPresentationFrame,
    world_seed: u64,
    object: &alife_world::WorldObject,
    tile: VoxelTileCoord,
    chunk: VoxelChunkCoord,
) -> Option<Fvr04CreatureVisualRecord> {
    let organism_id = object.organism_id?;
    let presentation = frame.organism(object.id)?;
    if presentation.organism_id != organism_id
        || presentation.world_entity_id != object.id
        || presentation.object.kind != WorldObjectKind::Agent
        || presentation.object.organism_id != Some(organism_id)
        || !presentation.lifecycle.is_alive()
    {
        return None;
    }
    let (selected_action_kind, target_entity) =
        presentation.motor.as_ref().map_or((None, None), |motor| {
            (motor.action_kind.clone(), motor.target_entity)
        });
    let target_position =
        target_entity.and_then(|target| frame.object(target).map(|object| object.position));
    let visual = creature_visual_snapshot_from_parts_with_appearance(
        presentation.organism_id,
        presentation.world_entity_id,
        presentation.object.position,
        target_entity,
        target_position,
        &presentation.biochemistry.homeostasis,
        presentation.sleep_phase,
        selected_action_kind,
        CreatureAppearanceGenome::from_ids(
            presentation.organism_id,
            presentation.genome.id,
            presentation.organism_id.raw() as usize,
            world_seed,
        ),
    )
    .ok()?;
    let cognitive = frame.cognitive_for_organism(organism_id);
    let memory_record_count = cognitive.and_then(|snapshot| {
        snapshot
            .fast_memory_count
            .zip(snapshot.lifetime_memory_count)
            .and_then(|(fast, lifetime)| fast.checked_add(lifetime))
    });
    Some(Fvr04CreatureVisualRecord {
        stable_ref: StableVoxelObjectRef {
            kind: StableVoxelRefKind::Creature,
            stable_id: Some(presentation.world_entity_id),
            chunk,
            tile: Some(tile),
        },
        tile,
        display_label: presentation.object.label.clone(),
        brain_class_id: cognitive.and_then(|snapshot| snapshot.brain_class_id),
        brain_neuron_count: cognitive.and_then(|snapshot| snapshot.brain_neuron_count),
        social_affinity: presentation.object.social_affinity,
        reproductive_drive: presentation
            .biochemistry
            .homeostasis
            .drives
            .reproductive_drive,
        fast_memory_count: cognitive.and_then(|snapshot| snapshot.fast_memory_count),
        lifetime_memory_count: cognitive.and_then(|snapshot| snapshot.lifetime_memory_count),
        memory_record_count,
        concept_count: cognitive.and_then(|snapshot| snapshot.concept_count),
        unresolved_gap_count: cognitive.and_then(|snapshot| snapshot.unresolved_gap_count),
        lifetime_learning_enabled: cognitive.and_then(|snapshot| snapshot.learning_active),
        sleep_phase_raw: cognitive.and_then(|snapshot| snapshot.sleep_phase_raw),
        consolidation_state_raw: cognitive.and_then(|snapshot| snapshot.consolidation_state_raw),
        last_consolidated_tick: cognitive.and_then(|snapshot| snapshot.last_consolidated_tick),
        topology_update_count: cognitive.and_then(|snapshot| snapshot.topology_update_count),
        visual,
    })
}

fn append_fvr04_creature_scene_resource(
    scene: &mut Fvr04ProductionCreatureSceneResource,
    added: Fvr04ProductionCreatureSceneResource,
) {
    let offset = scene.expression_buffer.len();
    for (index, sample) in added.expression_buffer.into_iter().enumerate() {
        scene
            .stable_lookup_by_raw_id
            .insert(sample.stable_id.raw(), offset + index);
        scene.expression_buffer.push(sample);
    }
    scene.rendered_creature_count = scene
        .rendered_creature_count
        .saturating_add(added.rendered_creature_count);
    scene.creature_root_count = scene
        .creature_root_count
        .saturating_add(added.creature_root_count);
    scene.creature_part_entity_count = scene
        .creature_part_entity_count
        .saturating_add(added.creature_part_entity_count);
    scene.creature_join_cover_count = scene
        .creature_join_cover_count
        .saturating_add(added.creature_join_cover_count);
}

fn animate_fvr04_creatures(
    time: Res<Time>,
    ux: Option<Res<Fvr05ProductionUxStateResource>>,
    mut creatures: bevy::prelude::Query<(&mut Transform, &Fvr04ProductionCreatureVisualMarker)>,
) {
    if ux.as_ref().is_some_and(|ux| ux.settings.paused) {
        return;
    }
    let speed = ux
        .as_ref()
        .map(|ux| ux.settings.simulation_speed)
        .unwrap_or(1.0);
    let seconds = time.elapsed_secs() * speed;
    for (mut transform, marker) in &mut creatures {
        let wave = (seconds * fvr04_animation_speed(marker.animation) + marker.phase).sin();
        let lateral = (seconds * 7.0 + marker.phase * 1.7).sin();
        let pose = creature_root_pose(marker.animation, wave, lateral);
        let pose_rotation = Quat::from_euler(
            EulerRot::XYZ,
            pose.rotation_xyz[0],
            pose.rotation_xyz[1],
            pose.rotation_xyz[2],
        );
        let rotation = Quat::from_rotation_y(std::f32::consts::PI) * pose_rotation;
        transform.rotation = rotation;
        transform.scale = marker.base_scale * Vec3::from_array(pose.scale);
    }
}

fn animate_fvr04_creature_parts(
    time: Res<Time>,
    ux: Option<Res<Fvr05ProductionUxStateResource>>,
    mut parts: bevy::prelude::Query<(
        &mut Transform,
        &ProductionCreaturePartMarker,
        &ProductionCreaturePartRestTransform,
        &ViewVisibility,
    )>,
) {
    if ux.as_ref().is_some_and(|ux| ux.settings.paused) {
        return;
    }
    let speed = ux
        .as_ref()
        .map(|ux| ux.settings.simulation_speed)
        .unwrap_or(1.0);
    let seconds = time.elapsed_secs() * speed;
    for (mut transform, marker, rest_transform, view_visibility) in &mut parts {
        if !view_visibility.get() {
            continue;
        }
        let phase = (marker.stable_id.raw() % 31) as f32 * 0.19;
        let wave = (seconds * 3.8 + phase).sin();
        let pose = creature_part_pose(marker.animation, marker.slot, wave);
        transform.translation = rest_transform.0.translation + Vec3::from_array(pose.translation);
        transform.rotation = rest_transform.0.rotation
            * Quat::from_euler(
                EulerRot::XYZ,
                pose.rotation_xyz[0],
                pose.rotation_xyz[1],
                pose.rotation_xyz[2],
            );
        transform.scale = rest_transform.0.scale * Vec3::from_array(pose.scale);
    }
}

fn fvr04_animation_speed(animation: CreatureAnimationState) -> f32 {
    match animation {
        CreatureAnimationState::Idle => 1.7,
        CreatureAnimationState::Moving => 5.8,
        CreatureAnimationState::Inspecting | CreatureAnimationState::Curious => 2.7,
        CreatureAnimationState::Interacting | CreatureAnimationState::Signaling => 3.4,
        CreatureAnimationState::Resting => 0.9,
        CreatureAnimationState::Sleeping => 0.45,
        CreatureAnimationState::Hurt | CreatureAnimationState::Afraid => 8.0,
    }
}

fn live_agent_ground_position(
    frame: &LiveBrainPresentationFrameResource,
    stable_id: WorldEntityId,
) -> Option<(VoxelTileCoord, Vec3)> {
    let object = frame.current.object(stable_id)?;
    (object.kind == WorldObjectKind::Agent).then(|| {
        let tile = VoxelTileCoord::new(
            object.position.x.round() as i32,
            object.position.z.round() as i32,
        );
        (
            tile,
            Vec3::new(tile.x as f32 + 0.5, 0.0, tile.z as f32 + 0.5),
        )
    })
}

fn sync_fvr11_creature_contact_shadows(
    mut commands: Commands,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    mut shadows: bevy::prelude::Query<(
        Entity,
        &mut Transform,
        &mut crate::Fvr11ProductionContactShadow,
    )>,
) {
    let Some(frame) = frame.filter(|frame| frame.is_changed()) else {
        return;
    };
    for (entity, mut transform, mut shadow) in &mut shadows {
        let Some(stable_id) = shadow.stable_id else {
            continue;
        };
        let Some((tile, position)) = live_agent_ground_position(&frame, stable_id) else {
            commands.entity(entity).despawn();
            continue;
        };
        shadow.tile = tile;
        transform.translation.x = position.x;
        transform.translation.z = position.z;
        if let Some(summary) = scene.tile_summaries_by_tile.get(&tile) {
            transform.translation.y = summary.height_units + 0.018;
        }
    }
}

#[cfg(not(feature = "vfx-hanabi"))]
fn sync_fvr07_attached_fallback_vfx(
    mut commands: Commands,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    mut markers: bevy::prelude::Query<(Entity, &mut Transform, &mut Fvr07ProductionGpuVfxMarker)>,
) {
    let Some(frame) = frame.filter(|frame| frame.is_changed()) else {
        return;
    };
    for (entity, mut transform, mut marker) in &mut markers {
        if !marker.follows_creature {
            continue;
        }
        let Some(stable_id) = marker.stable_id else {
            continue;
        };
        let Some((tile, position)) = live_agent_ground_position(&frame, stable_id) else {
            commands.entity(entity).despawn();
            continue;
        };
        marker.tile = Some(tile);
        marker.base_translation.x = position.x;
        marker.base_translation.z = position.z;
        transform.translation.x = position.x;
        transform.translation.z = position.z;
    }
}

#[cfg(feature = "vfx-hanabi")]
fn sync_fvr07_attached_hanabi_vfx(
    mut commands: Commands,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    mut emitters: bevy::prelude::Query<(Entity, &mut Transform, &Fvr07ProductionHanabiVfxEmitter)>,
) {
    let Some(frame) = frame.filter(|frame| frame.is_changed()) else {
        return;
    };
    for (entity, mut transform, emitter) in &mut emitters {
        if !emitter.follows_creature {
            continue;
        }
        let Some(stable_id) = emitter.stable_id else {
            continue;
        };
        let Some((_, position)) = live_agent_ground_position(&frame, stable_id) else {
            commands.entity(entity).despawn();
            continue;
        };
        transform.translation.x = position.x;
        transform.translation.z = position.z;
    }
}

#[cfg(not(feature = "vfx-hanabi"))]
fn animate_fvr07_production_vfx(
    time: Res<Time>,
    ux: Option<Res<Fvr05ProductionUxStateResource>>,
    mut markers: bevy::prelude::Query<(
        &mut Transform,
        &Fvr07ProductionGpuVfxMarker,
        &ViewVisibility,
    )>,
) {
    if ux.as_ref().is_some_and(|ux| ux.settings.paused) {
        return;
    }
    let speed = ux
        .as_ref()
        .map(|ux| ux.settings.simulation_speed)
        .unwrap_or(1.0);
    let seconds = time.elapsed_secs() * speed;
    for (mut transform, marker, view_visibility) in &mut markers {
        if !view_visibility.get() {
            continue;
        }
        let wave = (seconds * marker.kind.pulse_speed() + marker.phase).sin();
        let pulse = 1.0 + wave * 0.10;
        transform.translation =
            marker.base_translation + Vec3::Y * (wave.abs() * marker.kind.bob_height());
        transform.scale = Vec3::new(
            marker.base_scale.x * pulse.max(0.84),
            marker.base_scale.y * (1.0 + wave.abs() * 0.18),
            marker.base_scale.z * pulse.max(0.84),
        );
    }
}

fn selected_live_creature_object(
    selection: Option<StableVoxelObjectRef>,
    frame: Option<&LiveBrainPresentationFrameResource>,
) -> Option<(WorldEntityId, OrganismId, u64, Vec3f)> {
    let stable_id = selection
        .filter(|selection| selection.kind == StableVoxelRefKind::Creature)
        .and_then(|selection| selection.stable_id)?;
    let frame = frame?;
    let object = frame.current.object(stable_id)?;
    if object.kind != WorldObjectKind::Agent {
        return None;
    }
    let organism_id = object.organism_id?;
    Some((
        stable_id,
        organism_id,
        frame.current.authoritative_world_tick.raw(),
        Vec3f::new(object.position.x, object.position.y, object.position.z),
    ))
}

fn fvr04_live_creature_inspector_text(
    selection: Option<StableVoxelObjectRef>,
    creatures: &Fvr04ProductionCreatureSceneResource,
    live_state: Option<(u64, Vec3f)>,
    frame: Option<&LiveBrainPresentationFrameResource>,
) -> String {
    let live_text = selection
        .filter(|selection| selection.kind == StableVoxelRefKind::Creature)
        .and_then(|selection| selection.stable_id)
        .map(|stable_id| match live_state {
            Some((tick, position)) => format!(
                "LIVE AUTHORITATIVE WORLD\nworld tick: {tick}\nworld position: x={:.2} y={:.2} z={:.2}",
                position.x, position.y, position.z
            ),
            None => format!(
                "LIVE AUTHORITATIVE WORLD\nlive state: unavailable for selected stable {}",
                stable_id.raw()
            ),
        })
        .unwrap_or_else(|| "LIVE AUTHORITATIVE WORLD\nstate: unavailable".to_string());
    format!(
        "{live_text}\n\n{}\n\n{}",
        fvr04_live_learning_explanation(selection, frame),
        creatures.panel_text(selection)
    )
}

fn fvr04_live_learning_explanation(
    selection: Option<StableVoxelObjectRef>,
    frame: Option<&LiveBrainPresentationFrameResource>,
) -> String {
    let unavailable = || "LEARNING EXPLANATION\nstate: unavailable".to_string();
    let Some(stable_id) = selection
        .filter(|selection| selection.kind == StableVoxelRefKind::Creature)
        .and_then(|selection| selection.stable_id)
    else {
        return unavailable();
    };
    let Some(frame) = frame else {
        return unavailable();
    };
    let Some(current_row) = frame.current.organism(stable_id) else {
        return format!(
            "LEARNING EXPLANATION\nstable {}: unavailable",
            stable_id.raw()
        );
    };
    let organism_id = current_row.organism_id;
    let current_summary = frame
        .current
        .tick_summaries
        .iter()
        .find(|summary| summary.organism_id == organism_id);
    let previous_row = frame
        .previous
        .organism(stable_id)
        .filter(|row| row.organism_id == organism_id);
    let previous_summary = previous_row.and_then(|_| {
        frame
            .previous
            .tick_summaries
            .iter()
            .find(|summary| summary.organism_id == organism_id)
    });
    let previous_action = fvr04_live_action_text(previous_row, previous_summary);
    let current_action = fvr04_live_action_text(Some(current_row), current_summary);
    let outcome_change = fvr04_measured_joint_outcome_change(
        previous_row.and_then(|row| row.outcome.as_ref()),
        Some(current_row).and_then(|row| row.outcome.as_ref()),
    );
    let sleep_phase = fvr04_sleep_phase_text(current_row.sleep_phase);

    format!(
        "LEARNING EXPLANATION\norganism {} | stable world {}\naction: previous {} -> current {}\nmeasured joint outcome: {}\nsleep current: {} | work units: {}\nupdates previous: {}\nupdates current: {}",
        organism_id.raw(),
        stable_id.raw(),
        previous_action,
        current_action,
        outcome_change,
        sleep_phase,
        current_row.sleep_work_units,
        fvr04_live_update_counts(previous_summary),
        fvr04_live_update_counts(current_summary),
    )
}

fn fvr04_live_action_text(
    row: Option<&WorldOrganismPresentationRow>,
    summary: Option<&LiveBrainTickSummary>,
) -> String {
    let action = summary
        .map(|summary| (summary.selected_action_kind, summary.selected_action_id))
        .or_else(|| {
            row.and_then(|row| {
                row.motor
                    .as_ref()
                    .map(|motor| (motor.action_kind, motor.action_id))
            })
        });
    let Some((kind, action_id)) = action else {
        return "unavailable".to_string();
    };
    match (kind, action_id) {
        (Some(kind), Some(action_id)) => format!("{kind:?} (id {})", action_id.raw()),
        (Some(kind), None) => format!("{kind:?} (id unavailable)"),
        (None, Some(action_id)) => format!("kind unavailable (id {})", action_id.raw()),
        (None, None) => "none".to_string(),
    }
}

fn fvr04_live_update_counts(summary: Option<&LiveBrainTickSummary>) -> String {
    summary.map_or_else(
        || "memory=unavailable learning=unavailable topology=unavailable".to_string(),
        |summary| {
            format!(
                "memory={} learning={} topology={}",
                summary.memory_updates, summary.learning_updates, summary.topology_updates
            )
        },
    )
}

fn fvr04_measured_joint_outcome_change(
    previous: Option<&PresentationOutcomeSnapshot>,
    current: Option<&PresentationOutcomeSnapshot>,
) -> &'static str {
    let (Some(previous), Some(current)) = (previous, current) else {
        return "unavailable";
    };
    if !previous.patch_sealed || !current.patch_sealed {
        return "unavailable";
    }
    if previous.patch_success == current.patch_success
        && previous.physical_contact == current.physical_contact
        && previous.action_failure == current.action_failure
    {
        "unchanged"
    } else {
        "changed"
    }
}

fn fvr04_sleep_phase_text(phase: alife_core::SleepPhase) -> &'static str {
    match phase {
        alife_core::SleepPhase::Awake => "awake",
        alife_core::SleepPhase::EnteringSleep => "entering sleep",
        alife_core::SleepPhase::Consolidating => "consolidating",
        alife_core::SleepPhase::Waking => "waking",
        alife_core::SleepPhase::ForcedRecoverySleep => "recovery sleep",
    }
}

fn spawn_fvr03_selection_marker(
    world: &mut World,
    material: Handle<StandardMaterial>,
    mesh: Handle<Mesh>,
    selection: StableVoxelObjectRef,
) {
    let Some(tile) = selection.tile else {
        return;
    };
    world.spawn((
        Name::new(format!("A-Life FVR03 selected tile {}:{}", tile.x, tile.z)),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(tile.x as f32 + 0.5, 1.45, tile.z as f32 + 0.5),
        Visibility::Visible,
        Fvr03ProductionVoxelSelectionMarker,
        Fvr04ProductionRuntimeSceneRoot,
    ));
}

fn spawn_fvr04_creature_world_label(app: &mut App, selected: Option<StableVoxelObjectRef>) {
    let visible = selected.is_some_and(|selection| selection.kind == StableVoxelRefKind::Creature);
    app.world_mut().spawn((
        Name::new("A-Life FVR04 selected creature world label"),
        Text2d::new("creature"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgb(0.96, 0.93, 0.72)),
        Transform::from_xyz(0.0, 2.35, 0.0),
        if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        },
        Fvr04ProductionCreatureWorldLabel,
    ));
}

fn spawn_fvr05_production_ux_ui(app: &mut App) {
    app.world_mut().spawn((
        Name::new("A-Life FVR05 top runtime bar"),
        Text::new("A-Life"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.78, 0.98, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            height: Val::Px(38.0),
            padding: bevy::ui::UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.010, 0.018, 0.018, 0.92)),
        Visibility::Hidden,
        Fvr05ProductionTopRuntimeBar,
    ));
    app.world_mut().spawn((
        Name::new("A-Life FVR05 left production control rail"),
        Text::new("Simulation"),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.94, 0.90)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(46.0),
            left: Val::Px(12.0),
            width: Val::Px(270.0),
            max_width: Val::Px(270.0),
            padding: bevy::ui::UiRect::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.030, 0.032, 0.88)),
        Visibility::Hidden,
        Fvr05ProductionLeftControlPanel,
    ));
    app.world_mut().spawn((
        Name::new("A-Life FVR05 right inspector panel"),
        Text::new("Inspector"),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.90, 0.98, 0.90)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(46.0),
            right: Val::Px(12.0),
            width: Val::Px(360.0),
            max_width: Val::Px(360.0),
            padding: bevy::ui::UiRect::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.012, 0.026, 0.028, 0.90)),
        Visibility::Hidden,
        Fvr05ProductionRightInspectorPanel,
    ));
    app.world_mut().spawn((
        Name::new("A-Life FVR05 bottom overlay toolbar"),
        Text::new("Overlays"),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.86, 0.96, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(300.0),
            right: Val::Px(280.0),
            bottom: Val::Px(42.0),
            min_height: Val::Px(86.0),
            padding: bevy::ui::UiRect::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.012, 0.024, 0.028, 0.88)),
        Visibility::Hidden,
        Fvr05ProductionBottomOverlayToolbar,
    ));
    app.world_mut().spawn((
        Name::new("A-Life FVR05 footer status bar"),
        Text::new("Status"),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.76, 0.90, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            right: Val::Px(0.0),
            bottom: Val::Px(0.0),
            height: Val::Px(34.0),
            padding: bevy::ui::UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.010, 0.018, 0.020, 0.92)),
        Visibility::Hidden,
        Fvr05ProductionFooterStatusBar,
    ));
}

fn spawn_v0_player_experience_ui(app: &mut App) {
    app.world_mut().spawn((
        Name::new("A-Life V0 player status chip"),
        Text::new("A-LIFE"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.96, 0.91, 0.70)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(18.0),
            left: Val::Px(18.0),
            padding: bevy::ui::UiRect::axes(Val::Px(14.0), Val::Px(9.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.055, 0.075, 0.048, 0.88)),
        V0PlayerStatusChip,
    ));
    app.world_mut().spawn((
        Name::new("A-Life V0 selected creature panel"),
        Text::new("Select a creature"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.93, 0.82)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(62.0),
            right: Val::Px(18.0),
            width: Val::Px(318.0),
            max_width: Val::Px(318.0),
            padding: bevy::ui::UiRect::all(Val::Px(16.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.055, 0.070, 0.046, 0.91)),
        V0PlayerCreaturePanel,
    ));
    app.world_mut().spawn((
        Name::new("A-Life V0 player control strip"),
        Text::new("LMB Select  |  R Recover view"),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.89, 0.88, 0.74)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(92.0),
            right: Val::Px(92.0),
            bottom: Val::Px(18.0),
            padding: bevy::ui::UiRect::axes(Val::Px(16.0), Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.050, 0.066, 0.044, 0.88)),
        V0PlayerControlStrip,
    ));
}

fn sync_v0_player_status_chip(
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    ux: Res<Fvr05ProductionUxStateResource>,
    mut chips: bevy::prelude::Query<&mut Text, With<V0PlayerStatusChip>>,
) {
    if !scene.is_changed() && !ux.is_changed() {
        return;
    }
    let state = if ux.settings.paused {
        "PAUSED"
    } else {
        "LIVING"
    };
    let text = format!(
        "A-LIFE  |  {}  |  {} CREATURES",
        state, scene.creature_render_count
    );
    for mut chip in &mut chips {
        chip.0 = text.clone();
    }
}

fn sync_v0_player_creature_panel(
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    creatures: Res<Fvr04ProductionCreatureSceneResource>,
    follow: Res<Fvr04ProductionCreatureFollowResource>,
    mut panels: bevy::prelude::Query<&mut Text, With<V0PlayerCreaturePanel>>,
) {
    if !selection.is_changed() && !creatures.is_changed() && !follow.is_changed() {
        return;
    }
    let text = selection
        .selected
        .filter(|selected| selected.kind == StableVoxelRefKind::Creature)
        .and_then(|selected| selected.stable_id)
        .and_then(|stable_id| creatures.sample_for_stable_id(stable_id))
        .map(|sample| v0_selected_creature_text(sample, follow.enabled))
        .unwrap_or_else(|| {
            "CREATURES\nNo creature selected\n\nLMB selects a creature or terrain.\nR restores the default view."
                .to_string()
        });
    for mut panel in &mut panels {
        panel.0 = text.clone();
    }
}

fn sync_v0_player_control_strip(
    ux: Res<Fvr05ProductionUxStateResource>,
    follow: Res<Fvr04ProductionCreatureFollowResource>,
    mut strips: bevy::prelude::Query<&mut Text, With<V0PlayerControlStrip>>,
) {
    if !ux.is_changed() && !follow.is_changed() {
        return;
    }
    let playback = if ux.settings.paused {
        "Paused"
    } else {
        "Running"
    };
    let follow_state = if follow.enabled {
        "Following"
    } else {
        "Free camera"
    };
    let text = format!(
        "{}  {:.1}x  |  {}  |  LMB Select  E Place Food (Tile)  O Orbit  I Isometric  F Follow  R Recover  Space/P Pause  N Step  [ ] or 1/2/3 Speed",
        playback, ux.settings.simulation_speed, follow_state
    );
    for mut strip in &mut strips {
        strip.0 = text.clone();
    }
}

fn v0_selected_creature_text(sample: &Fvr04CreatureExpressionSample, following: bool) -> String {
    let display_name = v0_player_creature_name(&sample.display_label, sample.stable_id.raw());
    let brain = match (sample.brain_class_id, sample.brain_neuron_count) {
        (Some(class_id), Some(count)) => format!("class {class_id}, {count} neurons"),
        (None, Some(count)) => format!("{count} neurons"),
        _ => "brain unavailable".to_string(),
    };
    let learning = match sample.lifetime_learning_enabled {
        Some(true) => "active",
        Some(false) => "inactive",
        None => "unavailable",
    };
    let memories = sample
        .memory_record_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    let fast_memories = sample
        .fast_memory_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    let lifetime_memories = sample
        .lifetime_memory_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    let concepts = sample
        .concept_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    let gaps = sample
        .unresolved_gap_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| "unavailable".to_string());
    let last_sleep = sample
        .last_consolidated_tick
        .map(|tick| format!("Last sleep learning: tick {tick}"))
        .unwrap_or_else(|| "Last sleep learning: none yet".to_string());
    let consolidation = sample
        .consolidation_state_raw
        .map(|state| format!("state {state}"))
        .unwrap_or_else(|| "state unavailable".to_string());
    let follow_state = if following { "FOLLOWING" } else { "SELECTED" };
    format!(
        "{display_name}  |  {brain}\n{follow_state}  |  {}  |  {}\n\nNEEDS\nHunger   {} {:>3}%\nFatigue  {} {:>3}%\nSafety   {} {:>3}%\nSleep    {} {:>3}%\n\nSOCIAL\nReadiness {} {:>3}%\n\nLEARNING\n{}  |  memories {} (fast {} lifetime {})  |  concepts {}\nOpen curiosity gaps: {}\n{}\nConsolidation: {}",
        sample.animation.label(),
        sample.expression.label(),
        v0_need_bar(sample.hunger),
        v0_percent(sample.hunger),
        v0_need_bar(sample.fatigue),
        v0_percent(sample.fatigue),
        v0_need_bar(1.0 - sample.fear),
        v0_percent(1.0 - sample.fear),
        v0_need_bar(sample.sleep_pressure),
        v0_percent(sample.sleep_pressure),
        v0_need_bar(sample.social),
        v0_percent(sample.social),
        learning,
        memories,
        fast_memories,
        lifetime_memories,
        concepts,
        gaps,
        last_sleep,
        consolidation,
    )
}

fn v0_player_creature_name(label: &str, stable_id: u64) -> String {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.starts_with("production-creature-") {
        return format!("Creature #{stable_id}");
    }
    let mut words = trimmed
        .split(['-', '_', ' '])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        words.push("Creature".to_string());
    }
    format!("{}  #{}", words.join(" "), stable_id)
}

fn v0_need_bar(value: f32) -> String {
    let filled = (value.clamp(0.0, 1.0) * 8.0).round() as usize;
    format!("[{}{}]", "=".repeat(filled), "-".repeat(8 - filled))
}

fn v0_percent(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 100.0).round() as u32
}

fn handle_fvr03_mouse_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: bevy::prelude::Query<&Window, With<PrimaryWindow>>,
    cameras: bevy::prelude::Query<(&Camera, &GlobalTransform), With<Fvr03ProductionVoxelCamera>>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    mut selection: ResMut<Fvr03ProductionVoxelSelectionResource>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Some(cursor_position) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };
    let Some(distance) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::default()) else {
        return;
    };
    let world_position = ray.get_point(distance);
    let Some(tile) = scene.tile_from_world_position(world_position) else {
        return;
    };
    let hovered = scene.selectable_ref_at_tile(tile);
    selection.hovered = Some(hovered);
    if mouse.just_pressed(MouseButton::Left) {
        selection.selected = Some(hovered);
    }
}

fn handle_fvr05_production_ux_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    #[cfg(feature = "gpu-runtime")] conversation: Option<
        Res<crate::ProductionConversationLineageUiState>,
    >,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    mut follow: ResMut<Fvr04ProductionCreatureFollowResource>,
    mut ux: ResMut<Fvr05ProductionUxStateResource>,
    #[cfg(feature = "gpu-runtime")] mut gpu_runtime: Option<
        bevy::prelude::NonSendMut<crate::bevy_shell::ProductionGpuBrainRuntimeResource>,
    >,
    #[cfg(feature = "gpu-runtime")] mut schedule: Option<
        ResMut<crate::bevy_shell::ProductionGpuBrainTickScheduleResource>,
    >,
    #[cfg(feature = "gpu-runtime")] mut load_request: ResMut<ProductionRuntimeLoadRequest>,
) {
    #[cfg(feature = "gpu-runtime")]
    if conversation
        .as_ref()
        .is_some_and(|conversation| conversation.blocks_world_shortcuts())
    {
        return;
    }
    ux.update_selection_snapshot(selection.selected, follow.enabled);
    #[cfg(feature = "gpu-runtime")]
    if let Some(schedule) = schedule.as_deref() {
        ux.settings.paused = schedule.is_paused();
        ux.settings.simulation_speed = schedule.speed_ticks() as f32;
    }
    #[cfg(feature = "gpu-runtime")]
    if let Some(runtime) = gpu_runtime.as_ref() {
        ux.observe_gpu_runtime_save_status(runtime.runtime.manual_checkpoint_status());
    }
    if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyP) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = schedule.as_deref_mut() {
            schedule.toggle_playback();
            ux.settings.paused = schedule.is_paused();
        } else {
            ux.settings.paused = !ux.settings.paused;
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.settings.paused = !ux.settings.paused;
        }
        ux.last_action = if ux.settings.paused {
            "Paused production simulation".to_string()
        } else {
            "Resumed production simulation".to_string()
        };
    }
    if keyboard.just_pressed(KeyCode::Tab) {
        ux.settings.active_inspector_tab = ux.settings.active_inspector_tab.next();
        ux.last_action = format!(
            "Inspector tab: {}",
            ux.settings.active_inspector_tab.label()
        );
    }
    if keyboard.just_pressed(KeyCode::KeyM) {
        ux.settings.show_menu = !ux.settings.show_menu;
        ux.last_action = format!("Main menu visible: {}", ux.settings.show_menu);
    }
    if keyboard.just_pressed(KeyCode::KeyG) {
        ux.settings.show_settings = !ux.settings.show_settings;
        ux.last_action = format!("Settings visible: {}", ux.settings.show_settings);
    }
    if keyboard.just_pressed(KeyCode::KeyH) {
        ux.settings.show_overlays = !ux.settings.show_overlays;
        ux.last_action = format!("Overlays visible: {}", ux.settings.show_overlays);
    }
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = schedule.as_deref_mut() {
            let speed = schedule.speed_ticks().saturating_sub(1);
            schedule.set_running_speed(speed);
            ux.settings.paused = schedule.is_paused();
            ux.settings.simulation_speed = schedule.speed_ticks() as f32;
        } else {
            ux.settings.simulation_speed = (ux.settings.simulation_speed * 0.5).clamp(0.10, 5.0);
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.settings.simulation_speed = (ux.settings.simulation_speed * 0.5).clamp(0.10, 5.0);
        }
        ux.last_action = format!("Simulation speed {:.2}x", ux.settings.simulation_speed);
    }
    if keyboard.just_pressed(KeyCode::BracketRight) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = schedule.as_deref_mut() {
            let speed = schedule.speed_ticks().saturating_add(1);
            schedule.set_running_speed(speed);
            ux.settings.paused = schedule.is_paused();
            ux.settings.simulation_speed = schedule.speed_ticks() as f32;
        } else {
            ux.settings.simulation_speed = (ux.settings.simulation_speed * 2.0).clamp(0.10, 5.0);
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.settings.simulation_speed = (ux.settings.simulation_speed * 2.0).clamp(0.10, 5.0);
        }
        ux.last_action = format!("Simulation speed {:.2}x", ux.settings.simulation_speed);
    }
    #[cfg(feature = "gpu-runtime")]
    for (key, speed) in [
        (KeyCode::Digit1, 1),
        (KeyCode::Digit2, 2),
        (KeyCode::Digit3, 3),
    ] {
        if keyboard.just_pressed(key) {
            if let Some(schedule) = schedule.as_deref_mut() {
                schedule.set_running_speed(speed);
                ux.settings.paused = schedule.is_paused();
                ux.settings.simulation_speed = schedule.speed_ticks() as f32;
                ux.last_action = format!("Simulation speed {:.0}x", ux.settings.simulation_speed);
            }
        }
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        #[cfg(feature = "gpu-runtime")]
        let selected_tile = selection.selected.and_then(|selected| {
            (selected.kind == StableVoxelRefKind::Tile && selected.is_stable())
                .then_some(selected.tile)
                .flatten()
        });
        #[cfg(feature = "gpu-runtime")]
        match (gpu_runtime.as_mut(), selected_tile) {
            (Some(runtime), Some(tile)) => {
                let position = Vec3f::new(tile.x as f32 + 0.5, 0.0, tile.z as f32 + 0.5);
                match runtime.runtime.place_player_food(position) {
                    Ok(receipt) => {
                        ux.last_error = None;
                        ux.last_action = format!(
                            "Placed canonical food {} at tile x={} z={}",
                            receipt.world_entity_id.raw(),
                            tile.x,
                            tile.z
                        );
                    }
                    Err(error) => {
                        ux.last_error = Some(error.to_string());
                        ux.last_action =
                            "Food placement rejected; world left unchanged".to_string();
                    }
                }
            }
            (Some(_), None) => {
                ux.last_error = Some("select a visible terrain tile first".to_string());
                ux.last_action = "Food placement rejected; world left unchanged".to_string();
            }
            (None, _) => {
                ux.last_error = Some("GPU runtime unavailable".to_string());
                ux.last_action = "Food placement unavailable".to_string();
            }
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.last_error = Some("GPU runtime unavailable".to_string());
            ux.last_action = "Food placement unavailable".to_string();
        }
    }
    if keyboard.just_pressed(KeyCode::KeyS) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(runtime) = gpu_runtime.as_mut() {
            ux.write_gpu_runtime_save(false, &mut runtime.runtime);
        } else {
            ux.write_runtime_save(false);
        }
        #[cfg(not(feature = "gpu-runtime"))]
        ux.write_runtime_save(false);
        if ux.last_error.is_none() {
            ux.persist_ui_settings();
        }
    }
    if keyboard.just_pressed(KeyCode::KeyN) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = schedule.as_deref_mut() {
            schedule.queue_step();
            ux.settings.paused = schedule.is_paused();
            ux.settings.simulation_speed = schedule.speed_ticks() as f32;
            ux.last_action = "Queued one production simulation step".to_string();
        } else if let Some(runtime) = gpu_runtime.as_mut() {
            ux.write_gpu_runtime_save(true, &mut runtime.runtime);
        } else {
            ux.write_runtime_save(true);
        }
        #[cfg(not(feature = "gpu-runtime"))]
        ux.write_runtime_save(true);
        #[cfg(feature = "gpu-runtime")]
        if schedule.is_none() && ux.last_error.is_none() {
            ux.persist_ui_settings();
        }
        #[cfg(not(feature = "gpu-runtime"))]
        if ux.last_error.is_none() {
            ux.persist_ui_settings();
        }
    }
    if keyboard.just_pressed(KeyCode::KeyL) {
        #[cfg(feature = "gpu-runtime")]
        if load_request.queue() {
            ux.last_error = None;
            ux.last_action = "Queued authoritative production runtime load".to_string();
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.last_error = Some("GPU runtime unavailable; load was not queued".to_string());
            ux.last_action = "Load unavailable without GPU runtime".to_string();
        }
    }
    if keyboard.just_pressed(KeyCode::KeyQ) {
        ux.settings.preferred_profile_for_next_launch =
            fvr05_next_profile(ux.settings.preferred_profile_for_next_launch);
        ux.last_action = format!(
            "Preferred next-launch profile: {}",
            ux.settings.preferred_profile_for_next_launch.label()
        );
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        follow.enabled = false;
        ux.settings.show_menu = false;
        ux.settings.show_settings = false;
        ux.settings.show_overlays = false;
        ux.last_action = "Recovered the player view".to_string();
    }
    #[cfg(feature = "gpu-runtime")]
    let scheduler_speed_key = schedule.is_some()
        && [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
            .into_iter()
            .any(|key| keyboard.just_pressed(key));
    #[cfg(not(feature = "gpu-runtime"))]
    let scheduler_speed_key = false;
    if !scheduler_speed_key {
        if let Some(kind) = fvr05_overlay_key_pressed(&keyboard) {
            ux.toggle_overlay(kind);
        }
    }
}

fn fvr05_next_profile(profile: ProductionFrontendProfileId) -> ProductionFrontendProfileId {
    let all = ProductionFrontendProfileId::all();
    let index = all
        .iter()
        .position(|candidate| *candidate == profile)
        .unwrap_or_default();
    all[(index + 1) % all.len()]
}

fn fvr05_overlay_key_pressed(
    keyboard: &ButtonInput<KeyCode>,
) -> Option<Fvr05ProductionOverlayKind> {
    let mappings = [
        (KeyCode::Digit1, Fvr05ProductionOverlayKind::Resources),
        (KeyCode::Digit2, Fvr05ProductionOverlayKind::Danger),
        (KeyCode::Digit3, Fvr05ProductionOverlayKind::Pheromones),
        (KeyCode::Digit4, Fvr05ProductionOverlayKind::Energy),
        (KeyCode::Digit5, Fvr05ProductionOverlayKind::Age),
        (KeyCode::Digit6, Fvr05ProductionOverlayKind::Fertility),
        (KeyCode::Digit7, Fvr05ProductionOverlayKind::Territory),
        (KeyCode::Digit8, Fvr05ProductionOverlayKind::Neural),
        (KeyCode::Digit9, Fvr05ProductionOverlayKind::Residency),
        (KeyCode::KeyB, Fvr05ProductionOverlayKind::BackendTiming),
        (KeyCode::KeyC, Fvr05ProductionOverlayKind::ChunkBoundaries),
        (KeyCode::KeyD, Fvr05ProductionOverlayKind::LodBudget),
        (KeyCode::KeyV, Fvr05ProductionOverlayKind::Persistence),
    ];
    mappings
        .iter()
        .find_map(|(key, kind)| keyboard.just_pressed(*key).then_some(*kind))
}

fn sync_fvr05_panel_visibility(
    ux: Res<Fvr05ProductionUxStateResource>,
    mut panels: ParamSet<(
        bevy::prelude::Query<&mut Visibility, With<Fvr05ProductionTopRuntimeBar>>,
        bevy::prelude::Query<&mut Visibility, With<Fvr05ProductionLeftControlPanel>>,
        bevy::prelude::Query<&mut Visibility, With<Fvr05ProductionRightInspectorPanel>>,
        bevy::prelude::Query<&mut Visibility, With<Fvr05ProductionBottomOverlayToolbar>>,
        bevy::prelude::Query<&mut Visibility, With<Fvr05ProductionFooterStatusBar>>,
        bevy::prelude::Query<&mut Visibility, With<V0PlayerCreaturePanel>>,
    )>,
) {
    if !ux.is_changed() {
        return;
    }
    let menu_chrome = ux.settings.show_menu || ux.settings.show_settings;
    let overlay_chrome = ux.settings.show_overlays;
    let menu_visibility = if menu_chrome {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let overlay_visibility = if overlay_chrome {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    let footer_visibility = if menu_chrome || overlay_chrome {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in &mut panels.p0() {
        *visibility = menu_visibility;
    }
    for mut visibility in &mut panels.p1() {
        *visibility = menu_visibility;
    }
    for mut visibility in &mut panels.p2() {
        *visibility = menu_visibility;
    }
    for mut visibility in &mut panels.p3() {
        *visibility = overlay_visibility;
    }
    for mut visibility in &mut panels.p4() {
        *visibility = footer_visibility;
    }
    for mut visibility in &mut panels.p5() {
        *visibility = if menu_chrome {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
}

fn sync_fvr05_overlay_visibility(
    mut commands: Commands,
    ux: Res<Fvr05ProductionUxStateResource>,
    cache: Res<Fvr05OverlayGeometryCache>,
    assets: Res<Fvr04RuntimeSceneAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut overlays: bevy::prelude::Query<(&Fvr05ProductionOverlayBatch, &mut Visibility)>,
) {
    if !ux.is_changed() {
        return;
    }
    let mut spawned = BTreeSet::new();
    for (overlay, mut visibility) in &mut overlays {
        spawned.insert(overlay.kind);
        *visibility = if ux.active_overlay(overlay.kind) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    for kind in Fvr05ProductionOverlayKind::all().iter().copied() {
        if !ux.active_overlay(kind) || spawned.contains(&kind) {
            continue;
        }
        let Some(cells) = cache.cells_by_kind.get(&kind) else {
            continue;
        };
        if cells.is_empty() {
            continue;
        }
        let mesh = meshes.add(fvr05_batched_overlay_mesh(cells));
        let material = assets
            .overlay_materials
            .get(&kind)
            .expect("prepared FVR05 overlay material exists")
            .clone();
        commands.spawn((
            Name::new(format!("A-Life FVR05 overlay {}", kind.label())),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::default(),
            Visibility::Visible,
            Fvr05ProductionOverlayBatch {
                kind,
                cell_count: cells.len(),
            },
            Fvr04ProductionRuntimeSceneRoot,
        ));
    }
}

fn sync_fvr05_top_runtime_bar(
    ux: Res<Fvr05ProductionUxStateResource>,
    mut bars: bevy::prelude::Query<&mut Text, With<Fvr05ProductionTopRuntimeBar>>,
) {
    if !ux.is_changed() {
        return;
    }
    let status = if ux.settings.paused {
        "Paused"
    } else {
        "Running"
    };
    let runtime_save_path = PathBuf::from(&ux.settings.runtime_save_path);
    let save_name = runtime_save_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("runtime_save.json")
        .to_string();
    let text = format!(
        "A-Life | Profile: {} | Backend: {} | GPU: {} | Runtime: {} | Target FPS: {} | Frame: {:.1} ms | {} | Save: {}",
        ux.profile_id.label(),
        ux.graphics_backend,
        ux.adapter_name,
        ux.selected_backend,
        ux.profile_budget.target_fps,
        ux.profile_budget.target_frame_ms,
        status,
        save_name
    );
    for mut bar in &mut bars {
        bar.0 = text.clone();
    }
}

fn sync_fvr05_left_control_panel(
    ux: Res<Fvr05ProductionUxStateResource>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    mut panels: bevy::prelude::Query<&mut Text, With<Fvr05ProductionLeftControlPanel>>,
) {
    if !ux.is_changed() && !scene.is_changed() {
        return;
    }
    let menu = if ux.settings.show_menu {
        "open"
    } else {
        "closed"
    };
    let settings = if ux.settings.show_settings {
        format!(
            "QUALITY PROFILE\nactive: {}\npreferred: {}\nrender scale {:.2}\nchunks radius {}\nlabels {}\n\n",
            ux.profile_id.label(),
            ux.settings.preferred_profile_for_next_launch.label(),
            ux.profile_budget.default_internal_render_scale,
            scene.draw_radius_chunks,
            ux.profile_budget.label_density
        )
    } else {
        String::new()
    };
    let error = ux
        .last_error
        .as_deref()
        .map(|error| format!("\nERROR\n{error}\n"))
        .unwrap_or_default();
    let text = format!(
        "SIMULATION ({menu})\nSpace/P play-pause: {}\nN step once | 1/2/3 speed\n[ ] adjust speed\nS save world + UX | L load\nM menu | G settings | H overlays\nTab inspector | Q next profile\n4-9/B/C/D/V overlays\n\nQUICK CONTROLS\nfollow selection: {}\npause on focus loss: {}\noverlays: {}\n\nSIM SPEED\n{:.2}x\n\nSTATS (REAL RUNTIME)\ncreatures {}\nchunks loaded {}\nchunks resident {}\ntiles sampled {}\nmesher {} quads {} face reduction {:.2}x\nremesh budget {} dirty {} cached {} skipped {}\nmaterial atlas {}\ncreature visual {}\nbackend {}\n{}LAST ACTION\n{}{}",
        if ux.settings.paused { "paused" } else { "running" },
        ux.settings.follow_selection,
        ux.settings.pause_on_focus_loss,
        ux.settings.show_overlays,
        ux.settings.simulation_speed,
        scene.creature_render_count,
        scene.visible_chunk_count,
        scene.resident_chunk_count,
        scene.tile_mesh_count,
        scene.mesh_stats.mode.label(),
        scene.mesh_stats.emitted_quads,
        scene.mesh_stats.face_reduction_ratio,
        scene.mesh_stats.remesh_budget_chunks_per_frame,
        scene.mesh_stats.dirty_chunks,
        scene.mesh_stats.cached_chunks,
        scene.mesh_stats.skipped_chunks,
        scene.mesh_stats.material_palette_version,
        FVR10_CUTE_BIPED_VISUAL_PROFILE,
        ux.selected_backend,
        settings,
        ux.last_action,
        error
    );
    for mut panel in &mut panels {
        panel.0 = text.clone();
    }
}

fn sync_fvr05_right_inspector_panel(
    ux: Res<Fvr05ProductionUxStateResource>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    creatures: Res<Fvr04ProductionCreatureSceneResource>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    entity_map: Res<BevyEntityMap>,
    roots: bevy::prelude::Query<(
        &ProductionCreatureAssemblyRoot,
        &Fvr04ProductionCreatureVisualMarker,
    )>,
    authority: Option<Res<crate::bevy_shell::ProductionGpuBrainAuthorityResource>>,
    mut panels: bevy::prelude::Query<&mut Text, With<Fvr05ProductionRightInspectorPanel>>,
) {
    if !ux.is_changed()
        && !scene.is_changed()
        && !selection.is_changed()
        && !creatures.is_changed()
        && !frame.as_ref().is_some_and(|frame| frame.is_changed())
        && !authority
            .as_ref()
            .is_some_and(|authority| authority.is_changed())
    {
        return;
    }
    let tabs = Fvr05ProductionInspectorTab::all()
        .iter()
        .map(|tab| {
            if *tab == ux.settings.active_inspector_tab {
                format!("[{}]", tab.label())
            } else {
                tab.label().to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" | ");
    let selected_live =
        selected_live_creature_object(selection.selected, frame.as_ref().map(|frame| &**frame));
    let live_state = selected_live.and_then(|(stable_id, organism_id, tick, position)| {
        entity_map
            .bevy_entity(stable_id)
            .and_then(|entity| roots.get(entity).ok())
            .is_some_and(|(root, visual)| {
                root.stable_id == stable_id
                    && visual.stable_id == stable_id
                    && root.stable_id == visual.stable_id
                    && visual.organism_id == organism_id
            })
            .then_some((tick, position))
    });
    let body = match ux.settings.active_inspector_tab {
        Fvr05ProductionInspectorTab::Creature => format!(
            "{}\n\nDEBUG AUTHORITY\n{}",
            fvr04_live_creature_inspector_text(
                selection.selected,
                &creatures,
                live_state,
                frame.as_ref().map(|frame| &**frame),
            ),
            ux.authority.compact_line()
        ),
        Fvr05ProductionInspectorTab::Tile => {
            scene.tile_panel_text(selection.selected.or(selection.hovered))
        }
        Fvr05ProductionInspectorTab::World => scene.world_panel_text(),
        Fvr05ProductionInspectorTab::GpuRuntime => authority.as_ref().map_or_else(
            || "GPU neural: unavailable\nFailure policy: stop learned actions".to_string(),
            |authority| authority.telemetry.overlay_text(),
        ),
    };
    let text = format!("{tabs}\n\n{body}");
    for mut panel in &mut panels {
        panel.0 = text.clone();
    }
}

fn sync_fvr05_bottom_overlay_toolbar(
    ux: Res<Fvr05ProductionUxStateResource>,
    mut panels: bevy::prelude::Query<&mut Text, With<Fvr05ProductionBottomOverlayToolbar>>,
) {
    if !ux.is_changed() {
        return;
    }
    let labels = Fvr05ProductionOverlayKind::all()
        .iter()
        .map(|kind| {
            let marker = if ux.settings.enabled_overlays.contains(kind) {
                "on"
            } else {
                "off"
            };
            format!("{}={}", kind.label(), marker)
        })
        .collect::<Vec<_>>();
    let first = labels[..labels.len().min(7)].join(" | ");
    let second = labels[labels.len().min(7)..].join(" | ");
    let text = format!(
        "OVERLAYS\n{}\n{}\nkeys: 1 Resources 2 Danger 3 Pheromones 4 Energy 5 Age 6 Fertility 7 Territory 8 Neural 9 Residency B Backend C Chunks D LOD V Persistence",
        first, second
    );
    for mut panel in &mut panels {
        panel.0 = text.clone();
    }
}

fn sync_fvr05_footer_status_bar(
    ux: Res<Fvr05ProductionUxStateResource>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    mut bars: bevy::prelude::Query<&mut Text, With<Fvr05ProductionFooterStatusBar>>,
) {
    if !ux.is_changed() && !scene.is_changed() {
        return;
    }
    let text = format!(
        "Select LMB | Camera O orbit / I iso / F follow | chunks {} | LOD {} | mesher {} {:.2}x | resident bytes {} | backend {} | config {} | sim signature {}",
        scene.visible_chunk_count,
        scene.creature_lod.label(),
        scene.mesh_stats.mode.label(),
        scene.mesh_stats.face_reduction_ratio,
        scene.estimated_resident_bytes,
        ux.selected_backend,
        ux.ui_settings_path.display(),
        scene.stable_sim_signature()
    );
    for mut bar in &mut bars {
        bar.0 = text.clone();
    }
}

fn handle_fvr03_camera_mode_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut cameras: bevy::prelude::Query<(
        &mut Transform,
        &mut Projection,
        &mut Fvr03ProductionVoxelCamera,
    )>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
) {
    let next_mode = if keyboard.just_pressed(KeyCode::KeyO) {
        Some(Fvr03ProductionVoxelCameraMode::Orbit)
    } else if keyboard.just_pressed(KeyCode::KeyI) || keyboard.just_pressed(KeyCode::KeyR) {
        Some(Fvr03ProductionVoxelCameraMode::OrthographicIsometric)
    } else {
        None
    };
    let Some(next_mode) = next_mode else {
        return;
    };
    let extent = production_camera_extent(scene.profile_id);
    for (mut transform, mut projection, mut camera) in &mut cameras {
        camera.mode = next_mode;
        *transform = production_camera_transform(next_mode, extent);
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scaling_mode = ScalingMode::FixedVertical {
                viewport_height: extent,
            };
        }
    }
}

fn sync_fvr04_selection_marker(
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    entity_map: Res<BevyEntityMap>,
    roots: bevy::prelude::Query<
        (&ProductionCreatureAssemblyRoot, &Transform),
        Without<Fvr03ProductionVoxelSelectionMarker>,
    >,
    mut markers: bevy::prelude::Query<
        (&mut Transform, &mut Visibility),
        With<Fvr03ProductionVoxelSelectionMarker>,
    >,
) {
    let Some(selected) = selection.selected else {
        for (_, mut visibility) in &mut markers {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };
    let position = if selected.kind == StableVoxelRefKind::Creature {
        selected.stable_id.and_then(|stable_id| {
            let entity = entity_map.bevy_entity(stable_id)?;
            roots
                .get(entity)
                .ok()
                .filter(|(root, _)| root.stable_id == stable_id)
                .map(|(_, transform)| transform.translation)
        })
    } else {
        scene.world_position_for_selection(selected)
    };
    let Some(position) = position else {
        for (_, mut visibility) in &mut markers {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };
    for (mut transform, mut visibility) in &mut markers {
        let next_translation = Vec3::new(position.x, 1.45, position.z);
        if transform.translation != next_translation {
            transform.translation = next_translation;
        }
        if *visibility != Visibility::Visible {
            *visibility = Visibility::Visible;
        }
    }
}

fn handle_fvr04_camera_follow_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    mut follow: ResMut<Fvr04ProductionCreatureFollowResource>,
) {
    if !keyboard.just_pressed(KeyCode::KeyF) {
        return;
    }
    let selected_creature = selection.selected.and_then(|selection| {
        (selection.kind == StableVoxelRefKind::Creature)
            .then_some(selection.stable_id)
            .flatten()
    });
    if let Some(stable_id) = selected_creature {
        follow.enabled = follow.target_stable_id != Some(stable_id) || !follow.enabled;
        follow.target_stable_id = Some(stable_id);
    } else {
        follow.enabled = false;
        follow.target_stable_id = None;
    }
}

fn sync_fvr04_camera_follow(
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    follow: Res<Fvr04ProductionCreatureFollowResource>,
    entity_map: Res<BevyEntityMap>,
    roots: bevy::prelude::Query<
        (&ProductionCreatureAssemblyRoot, &Transform),
        Without<Fvr03ProductionVoxelCamera>,
    >,
    mut cameras: bevy::prelude::Query<
        (&mut Transform, &Fvr03ProductionVoxelCamera),
        Without<ProductionCreatureAssemblyRoot>,
    >,
) {
    if !follow.enabled {
        return;
    }
    let Some(target) = follow.target_stable_id else {
        return;
    };
    let Some(position) = entity_map
        .bevy_entity(target)
        .and_then(|entity| roots.get(entity).ok())
        .filter(|(root, _)| root.stable_id == target)
        .map(|(_, transform)| transform.translation)
    else {
        return;
    };
    let target = Vec3::new(position.x, 0.0, position.z);
    let extent = production_camera_extent(scene.profile_id);
    for (mut transform, camera) in &mut cameras {
        let next_transform = fvr04_follow_camera_transform(camera.mode, extent, target);
        if *transform != next_transform {
            *transform = next_transform;
        }
    }
}

fn sync_fvr04_creature_label(
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    creatures: Res<Fvr04ProductionCreatureSceneResource>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    entity_map: Res<BevyEntityMap>,
    roots: bevy::prelude::Query<
        (
            &ProductionCreatureAssemblyRoot,
            &Fvr04ProductionCreatureVisualMarker,
            &Transform,
        ),
        Without<Fvr04ProductionCreatureWorldLabel>,
    >,
    mut labels: bevy::prelude::Query<
        (&mut Text2d, &mut Transform, &mut Visibility),
        With<Fvr04ProductionCreatureWorldLabel>,
    >,
) {
    let refresh_text = selection.is_changed() || creatures.is_changed();
    let target = selection
        .hovered
        .filter(|hovered| hovered.kind == StableVoxelRefKind::Creature)
        .or_else(|| {
            selection
                .selected
                .filter(|selected| selected.kind == StableVoxelRefKind::Creature)
        });
    let Some(target) = target else {
        for (_, _, mut visibility) in &mut labels {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };
    let Some(stable_id) = target.stable_id else {
        for (_, _, mut visibility) in &mut labels {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };
    let Some(sample) = creatures.sample_for_stable_id(stable_id) else {
        for (_, _, mut visibility) in &mut labels {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };
    let Some((_, _, position)) = entity_map
        .bevy_entity(stable_id)
        .and_then(|entity| roots.get(entity).ok())
        .filter(|(root, visual, _)| {
            let Some(frame) = frame.as_ref().map(|frame| &**frame) else {
                return false;
            };
            let Some(object) = frame.current.object(stable_id) else {
                return false;
            };
            object.kind == WorldObjectKind::Agent
                && object.organism_id == Some(visual.organism_id)
                && root.stable_id == stable_id
                && visual.stable_id == stable_id
                && root.stable_id == visual.stable_id
        })
    else {
        for (_, _, mut visibility) in &mut labels {
            if *visibility != Visibility::Hidden {
                *visibility = Visibility::Hidden;
            }
        }
        return;
    };
    let label_text = refresh_text.then(|| {
        format!(
            "{}\n{}  |  {}",
            v0_player_creature_name(&sample.display_label, sample.stable_id.raw()),
            sample.animation.label(),
            sample.expression.label()
        )
    });
    for (mut text, mut transform, mut visibility) in &mut labels {
        if let Some(label_text) = &label_text {
            text.0.clone_from(label_text);
        }
        let next_translation = Vec3::new(position.translation.x, 2.35, position.translation.z);
        if transform.translation != next_translation {
            transform.translation = next_translation;
        }
        if *visibility != Visibility::Visible {
            *visibility = Visibility::Visible;
        }
    }
}

fn fvr04_follow_camera_transform(
    mode: Fvr03ProductionVoxelCameraMode,
    extent: f32,
    target: Vec3,
) -> Transform {
    let offset = match mode {
        Fvr03ProductionVoxelCameraMode::OrthographicIsometric => {
            Vec3::new(extent * 0.56, extent * 0.82, extent * 0.58)
        }
        Fvr03ProductionVoxelCameraMode::Orbit => {
            Vec3::new(extent * 0.72, extent * 0.52, extent * 0.94)
        }
    };
    Transform::from_translation(target + offset).looking_at(target, Vec3::Y)
}

#[cfg(feature = "gpu-runtime")]
fn phase31_frame_snapshot(
    runtime: &ProductionGpuBrainRuntimeResource,
    schedule: &ProductionGpuBrainTickScheduleResource,
    presentation: &LiveBrainPresentationFrameResource,
) -> Phase31FrameSnapshot {
    Phase31FrameSnapshot {
        runtime: runtime.runtime.performance_metrics(),
        scheduler: schedule.performance_counters(),
        checkpoint: runtime.runtime.exact_checkpoint_performance_state(),
        world_tick: presentation.current.authoritative_world_tick.raw(),
        world_objects: u64::try_from(presentation.current.object_count()).unwrap_or(u64::MAX),
        organisms: u64::try_from(presentation.current.organism_count()).unwrap_or(u64::MAX),
    }
}

#[cfg(feature = "gpu-runtime")]
fn phase31_performance_frame_begin(
    mut metrics: ResMut<Phase31PerformanceMetricsResource>,
    runtime: NonSend<ProductionGpuBrainRuntimeResource>,
    schedule: Res<ProductionGpuBrainTickScheduleResource>,
    presentation: Res<LiveBrainPresentationFrameResource>,
) {
    let now = Instant::now();
    let end_snapshot = phase31_frame_snapshot(&runtime, &schedule, &presentation);
    if metrics.measurement_started_at.is_none()
        && metrics.launched_at.elapsed() >= PHASE31_WARMUP_DURATION
        && presentation.current.authoritative_world_tick > Tick::ZERO
    {
        metrics.measurement_started_at = Some(now);
        metrics.measurement_completed_at = None;
        metrics.measurement_start_world_tick =
            Some(presentation.current.authoritative_world_tick.raw());
        metrics.runtime_baseline = Some(runtime.runtime.performance_metrics());
        metrics.scheduler_baseline = Some(schedule.performance_counters());
        metrics.last_frame_at = now;
        metrics.frame_snapshot = Some(end_snapshot);
    } else if metrics.measuring() {
        let frame_ns =
            u64::try_from(now.duration_since(metrics.last_frame_at).as_nanos()).unwrap_or(u64::MAX);
        metrics.frame_ns.push(frame_ns);
        if let Some(start_snapshot) = metrics.frame_snapshot.replace(end_snapshot) {
            let runtime_delta = end_snapshot.runtime.delta_from(start_snapshot.runtime);
            let update_cpu = std::mem::take(&mut metrics.current_frame_update_cpu);
            if frame_ns > PHASE31_SLOW_FRAME_THRESHOLD_NS {
                metrics.slow_frame_count = metrics.slow_frame_count.saturating_add(1);
            }
            let sample = Phase31SlowFrameSample {
                frame_index: u64::try_from(metrics.frame_ns.len()).unwrap_or(u64::MAX),
                frame_duration_ns: frame_ns,
                world_tick_before: start_snapshot.world_tick,
                world_tick_after: end_snapshot.world_tick,
                world_ticks_completed: end_snapshot
                    .world_tick
                    .saturating_sub(start_snapshot.world_tick),
                world_objects_before: start_snapshot.world_objects,
                world_objects_after: end_snapshot.world_objects,
                organisms_before: start_snapshot.organisms,
                organisms_after: end_snapshot.organisms,
                checkpoint_before: start_snapshot.checkpoint,
                checkpoint_after: end_snapshot.checkpoint,
                scheduler_attempts: end_snapshot
                    .scheduler
                    .scheduler_attempts
                    .saturating_sub(start_snapshot.scheduler.scheduler_attempts),
                scheduler_completed_ticks: end_snapshot
                    .scheduler
                    .completed_ticks
                    .saturating_sub(start_snapshot.scheduler.completed_ticks),
                checkpoint_publication_waits: end_snapshot
                    .scheduler
                    .checkpoint_publication_waits
                    .saturating_sub(start_snapshot.scheduler.checkpoint_publication_waits),
                checkpoint_failed_waits: end_snapshot
                    .scheduler
                    .checkpoint_failed_waits
                    .saturating_sub(start_snapshot.scheduler.checkpoint_failed_waits),
                deferred_catch_up_ticks: end_snapshot
                    .scheduler
                    .deferred_catch_up_ticks
                    .saturating_sub(start_snapshot.scheduler.deferred_catch_up_ticks),
                catch_up_ticks_dropped: end_snapshot
                    .scheduler
                    .catch_up_ticks_dropped
                    .saturating_sub(start_snapshot.scheduler.catch_up_ticks_dropped),
                scheduler_debt_micros_before: start_snapshot.scheduler.deferred_debt_micros,
                scheduler_debt_micros_after: end_snapshot.scheduler.deferred_debt_micros,
                update_cpu,
                renderer_present_and_uninstrumented_residual_ns: frame_ns
                    .saturating_sub(update_cpu.total_ns()),
                runtime: runtime_delta,
            };
            retain_ranked_slow_frame(&mut metrics.slow_frames, sample);
        }
        metrics.last_frame_at = now;
    }
    if metrics.measurement_completed_at.is_none()
        && metrics
            .measurement_started_at
            .is_some_and(|started| now.duration_since(started) >= PHASE31_MEASUREMENT_DURATION)
    {
        metrics.measurement_completed_at = Some(now);
    }
    metrics.stage_mark = metrics.measuring().then_some(now);
}

#[cfg(feature = "gpu-runtime")]
fn phase31_performance_after_input(mut metrics: ResMut<Phase31PerformanceMetricsResource>) {
    if metrics.measuring() {
        let elapsed = metrics.take_stage_elapsed_ns();
        metrics.input_cpu_ns = metrics.input_cpu_ns.saturating_add(elapsed);
        metrics.current_frame_update_cpu.input_ns = elapsed;
    }
}

#[cfg(feature = "gpu-runtime")]
fn phase31_performance_after_live_gpu_tick(mut metrics: ResMut<Phase31PerformanceMetricsResource>) {
    if metrics.measuring() {
        let elapsed = metrics.take_stage_elapsed_ns();
        metrics.live_gpu_tick_cpu_ns = metrics.live_gpu_tick_cpu_ns.saturating_add(elapsed);
        metrics.current_frame_update_cpu.live_gpu_tick_ns = elapsed;
    }
}

#[cfg(feature = "gpu-runtime")]
fn phase31_performance_after_authoritative_projection(
    mut metrics: ResMut<Phase31PerformanceMetricsResource>,
) {
    if metrics.measuring() {
        let elapsed = metrics.take_stage_elapsed_ns();
        metrics.authoritative_projection_cpu_ns = metrics
            .authoritative_projection_cpu_ns
            .saturating_add(elapsed);
        metrics.current_frame_update_cpu.authoritative_projection_ns = elapsed;
    }
}

#[cfg(feature = "gpu-runtime")]
fn phase31_performance_after_procedural_animation(
    mut metrics: ResMut<Phase31PerformanceMetricsResource>,
) {
    if metrics.measuring() {
        let elapsed = metrics.take_stage_elapsed_ns();
        metrics.procedural_animation_cpu_ns =
            metrics.procedural_animation_cpu_ns.saturating_add(elapsed);
        metrics.current_frame_update_cpu.procedural_animation_ns = elapsed;
    }
}

#[cfg(feature = "gpu-runtime")]
fn phase31_performance_after_ui(
    mut metrics: ResMut<Phase31PerformanceMetricsResource>,
    runtime: NonSend<ProductionGpuBrainRuntimeResource>,
    schedule: Res<ProductionGpuBrainTickScheduleResource>,
    authority: Res<ProductionGpuBrainAuthorityResource>,
    presentation: Res<LiveBrainPresentationFrameResource>,
    mut exits: MessageWriter<AppExit>,
) {
    let measuring = metrics.measuring();
    let draining = metrics.draining();
    if !measuring && !draining {
        return;
    }
    if measuring {
        let elapsed = metrics.take_stage_elapsed_ns();
        metrics.ui_root_readers_cpu_ns = metrics.ui_root_readers_cpu_ns.saturating_add(elapsed);
        metrics.current_frame_update_cpu.ui_root_readers_ns = elapsed;
        metrics.ui_updates = metrics.ui_updates.saturating_add(1);
    }
    let Some(started) = metrics.measurement_started_at else {
        return;
    };
    if !draining {
        return;
    }
    let drain_timed_out = metrics.measurement_completed_at.is_some_and(|completed| {
        Instant::now().duration_since(completed) >= PHASE31_PERSISTENCE_DRAIN_TIMEOUT
    });
    if !runtime.runtime.persistence_terminal_for_shutdown() && !drain_timed_out {
        return;
    }
    if drain_timed_out {
        let diagnostics = runtime.runtime.persistence_shutdown_diagnostics();
        eprintln!("PHASE31_PERSISTENCE_DRAIN_TIMEOUT {diagnostics}");
        metrics.write_error = Some(diagnostics);
        exits.write(AppExit::Error(std::num::NonZeroU8::new(1).unwrap()));
        return;
    }
    let persistence_failed = runtime.runtime.persistence_failed_for_shutdown();
    let performance_failed = schedule.performance_failed() || persistence_failed || drain_timed_out;
    match write_phase31_performance_receipt(
        &metrics,
        &runtime.runtime,
        schedule.performance_counters(),
        presentation.current.authoritative_world_tick.raw(),
        metrics
            .measurement_completed_at
            .unwrap_or_else(Instant::now)
            .duration_since(started),
        performance_failed,
        authority.telemetry.authoritative,
    ) {
        Ok(path) => metrics.artifact_path = Some(path),
        Err(error) => {
            let error = error.to_string();
            eprintln!(
                "PHASE31_PERFORMANCE_RECEIPT_ERROR error={error}; authority_reason={:?}; scheduler={:?}; persistence={}",
                authority.telemetry.unavailable_reason,
                schedule.performance_counters(),
                runtime.runtime.persistence_shutdown_diagnostics()
            );
            metrics.write_error = Some(error);
            exits.write(AppExit::Error(std::num::NonZeroU8::new(1).unwrap()));
            return;
        }
    }
    if performance_failed {
        exits.write(AppExit::Error(std::num::NonZeroU8::new(1).unwrap()));
    } else {
        exits.write(AppExit::Success);
    }
}

#[cfg(feature = "gpu-runtime")]
fn duration_summary(samples: &[u64]) -> serde_json::Value {
    if samples.is_empty() {
        return serde_json::json!({
            "count": 0,
            "total_ns": 0,
            "p50_ms": null,
            "p95_ms": null,
            "p99_ms": null,
            "max_ms": null,
            "hitches_over_100ms": 0
        });
    }
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let percentile = |numerator: usize| {
        let rank = sorted
            .len()
            .saturating_mul(numerator)
            .div_ceil(100)
            .saturating_sub(1)
            .min(sorted.len() - 1);
        sorted[rank] as f64 / 1_000_000.0
    };
    serde_json::json!({
        "count": sorted.len(),
        "total_ns": sorted.iter().fold(0_u64, |total, value| total.saturating_add(*value)),
        "p50_ms": percentile(50),
        "p95_ms": percentile(95),
        "p99_ms": percentile(99),
        "max_ms": *sorted.last().unwrap_or(&0) as f64 / 1_000_000.0,
        "hitches_over_100ms": sorted.iter().filter(|value| **value > 100_000_000).count()
    })
}

#[cfg(feature = "gpu-runtime")]
fn gpu_timestamp_ns(ticks: u64, period_ns_q24: u64) -> u64 {
    let scaled = u128::from(ticks).saturating_mul(u128::from(period_ns_q24));
    u64::try_from(scaled >> 24).unwrap_or(u64::MAX)
}

#[cfg(feature = "gpu-runtime")]
fn file_blake3_hex(path: &Path) -> Result<String, GameAppShellError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(feature = "gpu-runtime")]
fn write_phase31_performance_receipt(
    metrics: &Phase31PerformanceMetricsResource,
    runtime: &crate::GpuLiveBrainRuntime,
    scheduler_final: crate::bevy_shell::ProductionGpuTickPerformanceCounters,
    final_world_tick: u64,
    elapsed: Duration,
    schedule_failed: bool,
    gpu_authoritative: bool,
) -> Result<PathBuf, GameAppShellError> {
    if cfg!(debug_assertions) {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "Phase 3.1 baseline requires an optimized release executable".to_string(),
        });
    }
    let source_head = std::env::var("ALIFE_PHASE31_SOURCE_HEAD").map_err(|_| {
        GameAppShellError::InvalidProductionFrontend {
            message: "Phase 3.1 baseline requires ALIFE_PHASE31_SOURCE_HEAD".to_string(),
        }
    })?;
    if source_head.len() != 40 || !source_head.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "Phase 3.1 source SHA must be a full hexadecimal Git object ID".to_string(),
        });
    }
    let executable_path = std::env::current_exe()?;
    let executable_blake3 = file_blake3_hex(&executable_path)?;
    let runtime_delta = runtime
        .performance_metrics()
        .delta_from(metrics.runtime_baseline.unwrap_or_default());
    let scheduler_before = metrics.scheduler_baseline.unwrap_or(scheduler_final);
    let frames_observed = scheduler_final
        .frames_observed
        .saturating_sub(scheduler_before.frames_observed);
    let scheduler_completed_ticks = scheduler_final
        .completed_ticks
        .saturating_sub(scheduler_before.completed_ticks);
    let scheduler_attempts = scheduler_final
        .scheduler_attempts
        .saturating_sub(scheduler_before.scheduler_attempts);
    let checkpoint_publication_waits = scheduler_final
        .checkpoint_publication_waits
        .saturating_sub(scheduler_before.checkpoint_publication_waits);
    let checkpoint_failed_waits = scheduler_final
        .checkpoint_failed_waits
        .saturating_sub(scheduler_before.checkpoint_failed_waits);
    let deferred_catch_up_ticks = scheduler_final
        .deferred_catch_up_ticks
        .saturating_sub(scheduler_before.deferred_catch_up_ticks);
    let dropped_ticks = scheduler_final
        .catch_up_ticks_dropped
        .saturating_sub(scheduler_before.catch_up_ticks_dropped);
    let completed_world_ticks = final_world_tick.saturating_sub(
        metrics
            .measurement_start_world_tick
            .unwrap_or(final_world_tick),
    );
    let elapsed_seconds = elapsed.as_secs_f64().max(0.001);
    validate_phase31_performance_authority(
        schedule_failed,
        gpu_authoritative,
        runtime_delta.tick_calls,
        scheduler_attempts,
        scheduler_completed_ticks,
        completed_world_ticks,
        checkpoint_publication_waits.saturating_add(checkpoint_failed_waits),
    )
    .map_err(|message| GameAppShellError::InvalidProductionFrontend { message })?;
    let gpu_inference_ns = metrics.gpu_samples.iter().fold(0_u64, |total, sample| {
        total.saturating_add(gpu_timestamp_ns(
            sample.inference_timestamp_ticks,
            sample.timestamp_period_ns_q24,
        ))
    });
    let gpu_plasticity_ns = metrics.gpu_samples.iter().fold(0_u64, |total, sample| {
        total.saturating_add(gpu_timestamp_ns(
            sample.plasticity_timestamp_ticks,
            sample.timestamp_period_ns_q24,
        ))
    });
    let frame_total_ns = metrics
        .frame_ns
        .iter()
        .fold(0_u64, |total, value| total.saturating_add(*value));
    let measured_update_ns = metrics
        .input_cpu_ns
        .saturating_add(metrics.live_gpu_tick_cpu_ns)
        .saturating_add(metrics.authoritative_projection_cpu_ns)
        .saturating_add(metrics.procedural_animation_cpu_ns)
        .saturating_add(metrics.ui_root_readers_cpu_ns);
    let renderer_present_residual_ns = frame_total_ns.saturating_sub(measured_update_ns);
    let measured_preparation_ns = runtime_delta
        .preparation_sleep_eligibility_replay_wall_ns
        .saturating_add(runtime_delta.preparation_grounded_perception_wall_ns)
        .saturating_add(runtime_delta.preparation_episodic_retrieval_wall_ns)
        .saturating_add(runtime_delta.preparation_attention_context_wall_ns)
        .saturating_add(runtime_delta.preparation_topology_concept_wall_ns)
        .saturating_add(runtime_delta.preparation_gpu_upload_wall_ns)
        .saturating_add(runtime_delta.preparation_checkpoint_publication_wall_ns);
    let preparation_residual_ns = runtime_delta
        .perception_sleep_preparation_wall_ns
        .saturating_sub(measured_preparation_ns);
    let sleep_journal_publication_stages = serde_json::json!({
        "current_journal_load_validation_ns": runtime_delta.sleep_journal_current_load_validation_wall_ns,
        "merge_ns": runtime_delta.sleep_journal_merge_wall_ns,
        "sort_ns": runtime_delta.sleep_journal_sort_wall_ns,
        "journal_build_validation_ns": runtime_delta.sleep_journal_build_validation_wall_ns,
        "input_validation_ns": runtime_delta.sleep_journal_input_validation_wall_ns,
        "cas_lock_wait_ns": runtime_delta.sleep_journal_cas_lock_wait_wall_ns,
        "cas_base_reload_ns": runtime_delta.sleep_journal_cas_base_reload_wall_ns,
        "save_encode_ns": runtime_delta.sleep_journal_save_encode_wall_ns,
        "save_artifact_write_ns": runtime_delta.sleep_journal_save_artifact_write_wall_ns,
        "journal_encode_ns": runtime_delta.sleep_journal_encode_wall_ns,
        "journal_artifact_write_ns": runtime_delta.sleep_journal_artifact_write_wall_ns,
        "pointer_build_validation_ns": runtime_delta.sleep_journal_pointer_build_validation_wall_ns,
        "prepared_artifact_reload_validation_ns": runtime_delta.sleep_journal_prepared_reload_validation_wall_ns,
        "manifest_encode_ns": runtime_delta.sleep_journal_manifest_encode_wall_ns,
        "manifest_write_ns": runtime_delta.sleep_journal_manifest_write_wall_ns,
        "manifest_reload_validation_ns": runtime_delta.sleep_journal_manifest_reload_validation_wall_ns,
        "final_journal_reload_validation_ns": runtime_delta.sleep_journal_final_reload_validation_wall_ns,
        "outer_manifest_reload_validation_ns": runtime_delta.sleep_journal_outer_manifest_reload_validation_wall_ns,
        "outer_journal_reload_validation_ns": runtime_delta.sleep_journal_outer_reload_validation_wall_ns,
        "worker_starts": runtime_delta.sleep_journal_worker_starts,
        "worker_completions": runtime_delta.sleep_journal_worker_completions,
        "worker_failures": runtime_delta.sleep_journal_worker_failures,
        "worker_poll_calls": runtime_delta.sleep_journal_worker_poll_calls,
        "worker_poll_ns": runtime_delta.sleep_journal_worker_poll_wall_ns,
        "worker_wall_ns": runtime_delta.sleep_journal_worker_wall_ns,
        "pending_entries_peak": runtime_delta.sleep_journal_pending_entries_peak,
        "update_thread_enqueue_ns": runtime_delta.sleep_journal_update_thread_enqueue_wall_ns
    });
    let persistence_shutdown = serde_json::json!({
        "idle": runtime.persistence_idle_for_shutdown(),
        "failed": runtime.persistence_failed_for_shutdown(),
        "checkpoint": runtime.exact_checkpoint_performance_state(),
        "outstanding": runtime.persistence_shutdown_diagnostics()
    });
    let mut receipt = serde_json::json!({
        "schema": PHASE31_PERFORMANCE_SCHEMA,
        "schema_version": PHASE31_PERFORMANCE_SCHEMA_VERSION,
        "source_head": source_head,
        "build": {
            "mode": if cfg!(debug_assertions) { "debug" } else { "release" },
            "debug_assertions": cfg!(debug_assertions),
            "optimized_release": !cfg!(debug_assertions),
            "executable_path": executable_path,
            "executable_blake3": executable_blake3
        },
        "profile": metrics.profile,
        "population": metrics.population,
        "resolution": metrics.resolution,
        "backend": metrics.backend,
        "adapter": metrics.adapter,
        "measurement_seconds": elapsed_seconds,
        "world_tick": {
            "start": metrics.measurement_start_world_tick,
            "end": final_world_tick
        },
        "frame": duration_summary(&metrics.frame_ns),
        "slow_frames": {
            "threshold_ms": PHASE31_SLOW_FRAME_THRESHOLD_NS as f64 / 1_000_000.0,
            "total_count": metrics.slow_frame_count,
            "retained_worst_count": metrics.slow_frames.len(),
            "ranked_worst_first": metrics.slow_frames
        },
        "simulation": {
            "configured_tps": scheduler_final.fixed_tick_hz,
            "achieved_tps": completed_world_ticks as f64 / elapsed_seconds,
            "completed_world_ticks": completed_world_ticks,
            "scheduler_completed_ticks": scheduler_completed_ticks,
            "scheduler_attempts": scheduler_attempts,
            "scheduler_attempts_per_second": scheduler_attempts as f64 / elapsed_seconds,
            "zero_progress_calls_by_reason": {
                "checkpoint_publication_pending": checkpoint_publication_waits,
                "checkpoint_failed": checkpoint_failed_waits
            },
            "checkpoint_polls": runtime_delta.exact_checkpoint_poll_calls,
            "deferred_catch_up_ticks": deferred_catch_up_ticks,
            "deferred_debt_micros_at_end": scheduler_final.deferred_debt_micros,
            "catch_up_ticks_dropped": dropped_ticks,
            "scheduler_frames_observed": frames_observed,
            "runtime_tick_calls": runtime_delta.tick_calls,
            "runtime_tick_wall_ns": runtime_delta.tick_wall_ns
        },
        "internal_tick_stages": {
            "tick_preamble_ns": runtime_delta.tick_preamble_wall_ns,
            "perception_sleep_preparation_ns": runtime_delta.perception_sleep_preparation_wall_ns,
            "sleep_promotion_ns": runtime_delta.sleep_promotion_wall_ns,
            "inference_transaction_ns": runtime_delta.inference_transaction_wall_ns,
            "selection_prepare_ns": runtime_delta.selection_prepare_wall_ns,
            "seal_world_body_biochemistry_ns": runtime_delta.seal_world_body_biochemistry_wall_ns,
            "sealed_commit_total_ns": runtime_delta.sealed_commit_total_wall_ns,
            "learning_transaction_ns": runtime_delta.learning_transaction_wall_ns,
            "sidecar_memory_ns": runtime_delta.sidecar_memory_wall_ns,
            "sidecar_topology_ns": runtime_delta.sidecar_topology_wall_ns,
            "cognitive_authority_seal_ns": runtime_delta.cognitive_authority_seal_wall_ns,
            "world_authority_advance_ns": runtime_delta.world_authority_advance_wall_ns,
            "resident_synchronize_ns": runtime_delta.resident_synchronize_wall_ns,
            "passive_observation_ns": runtime_delta.passive_observation_wall_ns,
            "population_reconcile_ns": runtime_delta.population_reconcile_wall_ns,
            "sleep_persistence_ns": runtime_delta.sleep_persistence_wall_ns
        },
        "preparation_substages": {
            "sleep_eligibility_replay_ns": runtime_delta.preparation_sleep_eligibility_replay_wall_ns,
            "sleep_phase_data_ns": runtime_delta.preparation_sleep_phase_data_wall_ns,
            "sleep_replay_progress_ns": runtime_delta.preparation_sleep_replay_progress_wall_ns,
            "sleep_consolidation_ns": runtime_delta.preparation_sleep_consolidation_wall_ns,
            "sleep_scheduler_other_ns": runtime_delta
                .preparation_sleep_eligibility_replay_wall_ns
                .saturating_sub(
                    runtime_delta
                        .preparation_sleep_phase_data_wall_ns
                        .saturating_add(runtime_delta.preparation_sleep_replay_progress_wall_ns)
                        .saturating_add(runtime_delta.preparation_sleep_consolidation_wall_ns)
                ),
            "grounded_perception_ns": runtime_delta.preparation_grounded_perception_wall_ns,
            "episodic_retrieval_ns": runtime_delta.preparation_episodic_retrieval_wall_ns,
            "attention_context_ns": runtime_delta.preparation_attention_context_wall_ns,
            "topology_concept_ns": runtime_delta.preparation_topology_concept_wall_ns,
            "gpu_upload_preparation_ns": runtime_delta.preparation_gpu_upload_wall_ns,
            "checkpoint_publication_preparation_ns": runtime_delta.preparation_checkpoint_publication_wall_ns,
            "other_and_instrumentation_residual_ns": preparation_residual_ns
        },
        "transactional_rollback_clone": {
            "calls": runtime_delta.rollback_clone_calls,
            "world_clone_ns": runtime_delta.rollback_world_clone_wall_ns,
            "residents_clone_ns": runtime_delta.rollback_residents_clone_wall_ns,
            "resident_rows": runtime_delta.rollback_resident_rows,
            "world_object_rows": runtime_delta.rollback_world_object_rows,
            "successful_progress_calls": runtime_delta.rollback_clone_progress_calls,
            "zero_progress_calls": runtime_delta.rollback_clone_zero_progress_calls
        },
        "cpu_stages": {
            "input_ns": metrics.input_cpu_ns,
            "live_gpu_tick_ns": metrics.live_gpu_tick_cpu_ns,
            "authoritative_projection_ns": metrics.authoritative_projection_cpu_ns,
            "procedural_animation_ns": metrics.procedural_animation_cpu_ns,
            "ui_root_readers_ns": metrics.ui_root_readers_cpu_ns,
            "renderer_present_and_uninstrumented_residual_ns": renderer_present_residual_ns
        },
        "gpu_stages": {
            "timestamp_samples": metrics.gpu_samples.len(),
            "inference_ns": gpu_inference_ns,
            "plasticity_ns": gpu_plasticity_ns
        },
        "blocking_transactions": {
            "count": runtime_delta.inference_batches
                .saturating_add(runtime_delta.learning_batches)
                .saturating_add(runtime_delta.ordinary_snapshot_calls),
            "inference_batch_wall_ns": runtime_delta.inference_transaction_wall_ns,
            "learning_batch_wall_ns": runtime_delta.learning_transaction_wall_ns,
            "ordinary_snapshot_poll_wait_ns": runtime_delta.ordinary_snapshot_poll_wait_ns,
            "ordinary_snapshot_map_receive_wait_ns": runtime_delta.ordinary_snapshot_map_receive_wait_ns
        },
        "readback": {
            "selection_calls": runtime_delta.selection_readback_calls,
            "selection_bytes": runtime_delta.selection_readback_bytes,
            "learning_calls": runtime_delta.learning_readback_calls,
            "learning_bytes": runtime_delta.learning_readback_bytes,
            "ordinary_full_snapshot_calls": runtime_delta.ordinary_snapshot_calls,
            "ordinary_full_snapshot_bytes": runtime_delta.ordinary_snapshot_bytes
        },
        "ordinary_full_snapshot": {
            "calls": runtime_delta.ordinary_snapshot_calls,
            "bytes": runtime_delta.ordinary_snapshot_bytes,
            "wall_ns": runtime_delta.ordinary_snapshot_wall_ns,
            "poll_wait_ns": runtime_delta.ordinary_snapshot_poll_wait_ns,
            "map_receive_wait_ns": runtime_delta.ordinary_snapshot_map_receive_wait_ns,
            "calls_per_runtime_tick": if runtime_delta.tick_calls == 0 {
                0.0
            } else {
                runtime_delta.ordinary_snapshot_calls as f64 / runtime_delta.tick_calls as f64
            }
        },
        "state_reference_hash": {
            "calls": runtime_delta.state_reference_hash_calls,
            "resident_json_bytes": runtime_delta.resident_json_bytes,
            "topology_json_bytes": runtime_delta.topology_json_bytes,
            "wall_ns": runtime_delta.state_reference_hash_wall_ns
        },
        "dispatch_batching": {
            "inference_batches": runtime_delta.inference_batches,
            "inference_rows": runtime_delta.inference_rows,
            "mean_inference_rows_per_batch": if runtime_delta.inference_batches == 0 {
                0.0
            } else {
                runtime_delta.inference_rows as f64 / runtime_delta.inference_batches as f64
            },
            "learning_batches": runtime_delta.learning_batches,
            "learning_rows": runtime_delta.learning_rows,
            "mean_learning_rows_per_batch": if runtime_delta.learning_batches == 0 {
                0.0
            } else {
                runtime_delta.learning_rows as f64 / runtime_delta.learning_batches as f64
            }
        },
        "ui": {
            "updates": metrics.ui_updates,
            "cadence_hz": metrics.ui_updates as f64 / elapsed_seconds
        },
        "checkpoint_activity": {
            "capture_calls": runtime_delta.checkpoint_capture_calls,
            "capture_wall_ns": runtime_delta.checkpoint_capture_wall_ns,
            "full_snapshot_calls": runtime_delta.checkpoint_snapshot_calls,
            "full_snapshot_bytes": runtime_delta.checkpoint_snapshot_bytes,
            "poll_wait_ns": runtime_delta.checkpoint_snapshot_poll_wait_ns,
            "map_receive_wait_ns": runtime_delta.checkpoint_snapshot_map_receive_wait_ns,
            "asynchronous_poll_calls": runtime_delta.exact_checkpoint_poll_calls,
            "asynchronous_poll_cpu_ns": runtime_delta.exact_checkpoint_poll_wall_ns,
            "asynchronous_transactions_started": runtime_delta.exact_checkpoint_transactions_started,
            "asynchronous_transactions_completed": runtime_delta.exact_checkpoint_transactions_completed,
            "asynchronous_transaction_wall_ns": runtime_delta.exact_checkpoint_transaction_wall_ns
        },
        "sleep_durable_activity": {
            "boundary_calls": runtime_delta.sleep_persistence_calls,
            "capture_calls": runtime_delta.sleep_checkpoint_capture_calls,
            "exact_neural_capture_organisms": runtime_delta.sleep_exact_neural_capture_organisms,
            "compact_journal_organisms": runtime_delta.sleep_compact_journal_organisms,
            "capture_wall_ns": runtime_delta.sleep_checkpoint_capture_wall_ns,
            "capture_readback_calls": runtime_delta.sleep_checkpoint_readback_calls,
            "capture_readback_bytes": runtime_delta.sleep_checkpoint_readback_bytes,
            "capture_readback_poll_wait_ns": runtime_delta.sleep_checkpoint_readback_poll_wait_ns,
            "capture_readback_map_receive_wait_ns": runtime_delta.sleep_checkpoint_readback_map_receive_wait_ns,
            "checkpoint_publish_calls": runtime_delta.sleep_checkpoint_publish_calls,
            "checkpoint_publish_wall_ns": runtime_delta.sleep_checkpoint_publish_wall_ns,
            "promotion_calls": runtime_delta.sleep_promotion_calls,
            "promotion_publish_calls": runtime_delta.sleep_promotion_publish_calls,
            "promotion_publish_wall_ns": runtime_delta.sleep_promotion_publish_wall_ns
        },
        "sleep_journal_publication_stages": sleep_journal_publication_stages
    });
    receipt
        .as_object_mut()
        .expect("performance receipt root is an object")
        .insert("persistence_shutdown".to_string(), persistence_shutdown);
    let root = PathBuf::from(PHASE31_PERFORMANCE_ARTIFACT_DIR);
    fs::create_dir_all(&root)?;
    let path = root.join(format!(
        "phase31-before-release-population-{}.json",
        metrics.population
    ));
    fs::write(&path, serde_json::to_string_pretty(&receipt)?)?;
    Ok(path)
}

fn request_fvr03_recorded_screenshot(
    mut commands: Commands,
    mut capture: ResMut<Fvr03ProductionVoxelScreenshotResource>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    presentation: Option<Res<LiveBrainPresentationFrameResource>>,
    mut ux: Option<ResMut<Fvr05ProductionUxStateResource>>,
    #[cfg(feature = "gpu-runtime")] mut conversation: Option<
        ResMut<ProductionConversationLineageUiState>,
    >,
    #[cfg(feature = "gpu-runtime")] phase31: Option<Res<Phase31PerformanceMetricsResource>>,
    mut overlay_batches: bevy::prelude::Query<&mut Visibility, With<Fvr05ProductionOverlayBatch>>,
    mut exits: MessageWriter<AppExit>,
) {
    #[cfg(feature = "gpu-runtime")]
    let legacy_capture_controls_lifetime =
        fvr03_legacy_capture_controls_lifetime(phase31.is_some());
    #[cfg(not(feature = "gpu-runtime"))]
    let legacy_capture_controls_lifetime = true;
    capture.frame = capture.frame.saturating_add(1);
    if capture.measurement_started_at.is_none() && capture.frame >= capture.capture_after_frame {
        capture.measurement_start_frame = capture.frame;
        capture.measurement_started_at = Some(Instant::now());
    }
    if !capture.measurement_written
        && capture.measurement_started_at.is_some()
        && capture.frame
            >= capture
                .measurement_start_frame
                .saturating_add(capture.measurement_sample_frames)
    {
        if let Some(started_at) = capture.measurement_started_at {
            let elapsed_seconds = started_at.elapsed().as_secs_f64().max(0.001);
            let measured_fps = f64::from(capture.measurement_sample_frames) / elapsed_seconds;
            let _ = write_fvr03_performance_artifact(
                &scene,
                Some((
                    measured_fps,
                    capture.measurement_sample_frames,
                    elapsed_seconds,
                )),
            );
        }
        capture.measurement_written = true;
    }
    if !fvr03_visual_capture_ready(
        capture.frame,
        capture.capture_after_frame,
        selection.selected,
        presentation.as_deref(),
    ) {
        return;
    }
    if !capture.product_screenshot_captured {
        if let Some(parent) = capture.path.parent() {
            if fs::create_dir_all(parent).is_err() {
                capture.requested = true;
                if legacy_capture_controls_lifetime {
                    exits.write(AppExit::Success);
                }
                return;
            }
        }
        if let Some(ux) = ux.as_mut() {
            ux.settings.show_menu = false;
            ux.settings.show_settings = false;
            ux.settings.show_overlays = false;
            ux.last_action =
                "Capture-mode selection (non-input evidence); recorded clean product screenshot"
                    .to_string();
        }
        for mut visibility in &mut overlay_batches {
            *visibility = Visibility::Hidden;
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(capture.path.clone()));
        capture.product_screenshot_captured = true;
        capture.fvr05_next_capture_frame =
            capture.frame.saturating_add(FVR05_SCREENSHOT_SETTLE_FRAMES);
        if !capture.developer_overlay {
            capture.fvr05_capture_index = 4;
        }
        return;
    }
    if capture.fvr05_sequence_complete {
        if legacy_capture_controls_lifetime
            && capture.measurement_written
            && capture.frame >= capture.fvr05_next_capture_frame
        {
            capture.requested = true;
            exits.write(AppExit::Success);
        }
        return;
    }
    if capture.frame < capture.fvr05_next_capture_frame {
        return;
    }
    let Some((suffix, tab)) = fvr05_screenshot_step(capture.fvr05_capture_index) else {
        capture.fvr05_sequence_complete = true;
        capture.fvr05_next_capture_frame = capture.frame.saturating_add(24);
        return;
    };
    if let Some(parent) = capture.path.parent() {
        if fs::create_dir_all(parent).is_err() {
            capture.requested = true;
            if legacy_capture_controls_lifetime {
                exits.write(AppExit::Success);
            }
            return;
        }
    }
    if let Some(ux) = ux.as_mut() {
        let show_developer_surfaces = capture.fvr05_capture_index < 4 || capture.developer_overlay;
        ux.settings.show_menu = show_developer_surfaces;
        ux.settings.show_settings = show_developer_surfaces;
        ux.settings.show_overlays = show_developer_surfaces;
        ux.settings.active_inspector_tab = tab;
        ux.last_action = format!(
            "Capture-mode selection (non-input evidence); recorded {} tab",
            tab.label()
        );
    }
    #[cfg(feature = "gpu-runtime")]
    if let Some(conversation) = conversation.as_mut() {
        match capture.fvr05_capture_index {
            4 => conversation.prepare_recorded_speech_capture(),
            5 => conversation.prepare_recorded_lineage_capture(),
            _ => conversation.clear_recorded_capture(),
        }
    }
    let path = fvr05_screenshot_path(&capture.path, suffix);
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    capture.fvr05_capture_index = capture.fvr05_capture_index.saturating_add(1);
    capture.fvr05_next_capture_frame = capture.frame.saturating_add(FVR05_SCREENSHOT_SETTLE_FRAMES);
    if fvr05_screenshot_step(capture.fvr05_capture_index).is_none() {
        capture.fvr05_sequence_complete = true;
    }
}

fn fvr03_legacy_capture_controls_lifetime(phase31_measurement_mode: bool) -> bool {
    !phase31_measurement_mode
}

const FVR03_VISUAL_CAPTURE_AFTER_FRAMES: u32 = 8;
const FVR05_SCREENSHOT_SETTLE_FRAMES: u32 = 2;

fn fvr03_screenshot_capture_frame(_settings: &Fvr03ProductionVoxelRendererSettings) -> u32 {
    FVR03_VISUAL_CAPTURE_AFTER_FRAMES
}

fn fvr03_visual_capture_ready(
    frame: u32,
    capture_after_frame: u32,
    selected: Option<StableVoxelObjectRef>,
    presentation: Option<&LiveBrainPresentationFrameResource>,
) -> bool {
    if frame < capture_after_frame {
        return false;
    }
    let Some(selected) = selected.filter(|selected| selected.kind == StableVoxelRefKind::Creature)
    else {
        return false;
    };
    let Some(stable_id) = selected.stable_id else {
        return false;
    };
    let Some(current) = presentation.map(|presentation| &presentation.current) else {
        return false;
    };
    if current.authoritative_world_tick == Tick::ZERO {
        return false;
    }
    let Some(organism) = current.organism(stable_id) else {
        return false;
    };
    current
        .cognitive_for_organism(organism.organism_id)
        .is_some()
        && current
            .tick_summaries
            .iter()
            .any(|summary| summary.organism_id == organism.organism_id)
}

fn fvr05_screenshot_step(index: usize) -> Option<(&'static str, Fvr05ProductionInspectorTab)> {
    match index {
        0 => Some(("fvr05_gpu_panel", Fvr05ProductionInspectorTab::GpuRuntime)),
        1 => Some((
            "fvr05_menu_settings_creature",
            Fvr05ProductionInspectorTab::Creature,
        )),
        2 => Some(("fvr05_tile_inspector", Fvr05ProductionInspectorTab::Tile)),
        3 => Some(("fvr05_world_inspector", Fvr05ProductionInspectorTab::World)),
        4 => Some(("m14_player_speech", Fvr05ProductionInspectorTab::Creature)),
        5 => Some(("m14_lineage_library", Fvr05ProductionInspectorTab::Creature)),
        _ => None,
    }
}

fn fvr05_screenshot_path(base_path: &PathBuf, suffix: &str) -> PathBuf {
    let parent = base_path
        .parent()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = base_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("production_voxel");
    parent.join(format!("{stem}_{suffix}.png"))
}

fn fvr03_material_kind(
    material: ProceduralTerrainMaterial,
    tile: VoxelTileCoord,
) -> Fvr03ProductionVoxelMaterialKind {
    match material {
        ProceduralTerrainMaterial::SafeGrass => Fvr03ProductionVoxelMaterialKind::SafeGrass,
        ProceduralTerrainMaterial::NeutralSoil => Fvr03ProductionVoxelMaterialKind::Soil,
        ProceduralTerrainMaterial::ResourceGrove => Fvr03ProductionVoxelMaterialKind::Resource,
        ProceduralTerrainMaterial::HazardPressure => {
            if (tile.x + tile.z).rem_euclid(3) == 0 {
                Fvr03ProductionVoxelMaterialKind::Decay
            } else {
                Fvr03ProductionVoxelMaterialKind::Hazard
            }
        }
        ProceduralTerrainMaterial::StoneRough => Fvr03ProductionVoxelMaterialKind::Stone,
        ProceduralTerrainMaterial::Water => Fvr03ProductionVoxelMaterialKind::Water,
        ProceduralTerrainMaterial::Sand => Fvr03ProductionVoxelMaterialKind::Sand,
    }
}

fn fvr03_tile_height(
    material: ProceduralTerrainMaterial,
    resource_bias: f32,
    hazard_pressure: f32,
    roughness: f32,
) -> f32 {
    let base = match material {
        ProceduralTerrainMaterial::Water => 0.18,
        ProceduralTerrainMaterial::Sand => 0.24,
        ProceduralTerrainMaterial::SafeGrass => 0.44,
        ProceduralTerrainMaterial::NeutralSoil => 0.38,
        ProceduralTerrainMaterial::ResourceGrove => 0.64 + resource_bias * 0.18,
        ProceduralTerrainMaterial::HazardPressure => 0.72 + hazard_pressure * 0.24,
        ProceduralTerrainMaterial::StoneRough => 0.82 + roughness * 0.46,
    };
    base.clamp(0.16, 1.28)
}

fn fvr03_estimated_resident_bytes(tile_count: usize, chunk_count: usize) -> usize {
    tile_count
        .saturating_mul(192)
        .saturating_add(chunk_count.saturating_mul(512))
        .saturating_add(128 * 1024)
}

fn write_fvr03_performance_artifact(
    scene: &Fvr03ProductionVoxelSceneResource,
    measurement: Option<(f64, u32, f64)>,
) -> Result<PathBuf, GameAppShellError> {
    let root = PathBuf::from(FVR03_PERFORMANCE_ARTIFACT_DIR);
    fs::create_dir_all(&root)?;
    let path = root.join(format!(
        "{}_renderer_diagnostics.json",
        scene.profile_id.label()
    ));
    let (measured_fps, measured_frame_count, measured_seconds, performance_status) =
        if let Some((fps, frame_count, seconds)) = measurement {
            (
                format!("{fps:.2}"),
                frame_count.to_string(),
                format!("{seconds:.3}"),
                "measured-local-smoke-no-broad-claim",
            )
        } else {
            (
                "null".to_string(),
                "null".to_string(),
                "null".to_string(),
                "not-measured-no-performance-claim",
            )
        };
    let contents = format!(
        "{{\n  \"schema\": \"{}\",\n  \"profile\": \"{}\",\n  \"backend\": \"{}\",\n  \"target_fps\": {},\n  \"visible_chunks\": {},\n  \"resident_chunks\": {},\n  \"tile_mesh_count\": {},\n  \"mesher_mode\": \"{}\",\n  \"material_palette_version\": \"{}\",\n  \"visible_voxels\": {},\n  \"naive_visible_faces\": {},\n  \"emitted_quads\": {},\n  \"face_reduction_ratio\": {:.3},\n  \"remesh_time_micros\": {},\n  \"dirty_chunks\": {},\n  \"cached_chunks\": {},\n  \"skipped_chunks\": {},\n  \"remesh_budget_chunks_per_frame\": {},\n  \"mesh_cache_key\": \"{}\",\n  \"creature_render_count\": {},\n  \"creature_visual_profile\": \"{}\",\n  \"creature_mesh_material_version\": \"{}\",\n  \"creature_material_bucket_count\": {},\n  \"creature_lod\": \"{}\",\n  \"creature_root_count\": {},\n  \"creature_part_entity_count\": {},\n  \"creature_join_cover_count\": {},\n  \"creature_part_family_count\": {},\n  \"creature_mixed_assembly_count\": {},\n  \"creature_shared_mesh_handle_count\": {},\n  \"production_dressing_count\": {},\n  \"production_vfx_marker_count\": {},\n  \"production_gpu_vfx_emitter_count\": {},\n  \"production_vfx_budget_state\": \"{}\",\n  \"production_visuals_display_only\": {},\n  \"production_vfx_uses_hanabi_gpu_particles\": {},\n  \"estimated_resident_bytes\": {},\n  \"measured_fps\": {},\n  \"measured_frame_count\": {},\n  \"measured_seconds\": {},\n  \"performance_claim_status\": \"{}\"\n}}\n",
        scene.schema,
        scene.profile_id.label(),
        scene.backend_id,
        scene.target_fps,
        scene.visible_chunk_count,
        scene.resident_chunk_count,
        scene.tile_mesh_count,
        scene.mesh_stats.mode.label(),
        scene.mesh_stats.material_palette_version,
        scene.mesh_stats.visible_voxels,
        scene.mesh_stats.naive_visible_faces,
        scene.mesh_stats.emitted_quads,
        scene.mesh_stats.face_reduction_ratio,
        scene.mesh_stats.remesh_time_micros,
        scene.mesh_stats.dirty_chunks,
        scene.mesh_stats.cached_chunks,
        scene.mesh_stats.skipped_chunks,
        scene.mesh_stats.remesh_budget_chunks_per_frame,
        scene.mesh_stats.cache_key,
        scene.creature_render_count,
        FVR10_CUTE_BIPED_VISUAL_PROFILE,
        FVR10_CUTE_BIPED_MATERIAL_VERSION,
        scene.creature_material_bucket_count,
        scene.creature_lod.label(),
        scene.creature_root_count,
        scene.creature_part_entity_count,
        scene.creature_join_cover_count,
        scene.creature_part_family_count,
        scene.creature_mixed_assembly_count,
        scene.creature_shared_mesh_handle_count,
        scene.production_dressing_count,
        scene.production_vfx_marker_count,
        scene.production_gpu_vfx_emitter_count,
        scene.production_vfx_budget_state,
        scene.production_visuals_display_only,
        scene.production_vfx_uses_hanabi_gpu_particles,
        scene.estimated_resident_bytes,
        measured_fps,
        measured_frame_count,
        measured_seconds,
        performance_status
    );
    fs::write(&path, contents)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{OrganismId, Tick, WorldEntityId};
    use alife_world::{HeadlessScenarioBuilder, WorldObjectKind};

    #[cfg(feature = "gpu-runtime")]
    use crate::bevy_shell::{
        ProductionCuratedFounderResetCommand, ProductionCuratedFounderResetResultResource,
    };
    #[cfg(feature = "gpu-runtime")]
    use crate::curated_founder_staging::{
        CuratedFounderPublicationStatus, CuratedFounderSaveState,
    };
    #[cfg(feature = "gpu-runtime")]
    use crate::gpu_live_runtime::{
        CuratedFounderGpuResidencyState, CuratedFounderResetDispatchRejection,
        CuratedFounderResetDispatchResult, CuratedFounderResetRuntimeError,
        CuratedFounderResetRuntimeEvidence, CuratedFounderResetRuntimePort,
        CuratedFounderResetRuntimeResult, LiveAgentResetIntent,
    };
    #[cfg(feature = "gpu-runtime")]
    use crate::CuratedFounderAgentInput;

    #[test]
    #[cfg(feature = "gpu-runtime")]
    fn phase31_measurement_deadline_stops_admitting_simulation_work() {
        let now = Instant::now();
        let metrics = Phase31PerformanceMetricsResource {
            profile: "test".to_string(),
            population: 6,
            resolution: [1920, 1080],
            backend: "GpuAuthoritative".to_string(),
            adapter: "test-adapter".to_string(),
            launched_at: now,
            last_frame_at: now,
            measurement_started_at: Some(now - PHASE31_MEASUREMENT_DURATION),
            measurement_completed_at: None,
            measurement_start_world_tick: Some(1),
            runtime_baseline: None,
            scheduler_baseline: None,
            stage_mark: None,
            frame_snapshot: None,
            current_frame_update_cpu: Phase31FrameUpdateCpu::default(),
            frame_ns: Vec::new(),
            slow_frame_count: 0,
            slow_frames: Vec::new(),
            input_cpu_ns: 0,
            live_gpu_tick_cpu_ns: 0,
            authoritative_projection_cpu_ns: 0,
            procedural_animation_cpu_ns: 0,
            ui_root_readers_cpu_ns: 0,
            ui_updates: 0,
            gpu_samples: Vec::new(),
            artifact_path: None,
            write_error: None,
        };

        assert!(
            !metrics.measuring(),
            "an expired measurement must stop admitting simulation work before LiveGpuTick"
        );
        assert!(
            metrics.draining(),
            "an expired measurement must enter persistence drain mode"
        );
    }

    #[test]
    #[cfg(feature = "gpu-runtime")]
    fn phase31_duration_summary_reports_nearest_rank_percentiles_and_hitches() {
        let samples = [
            10_000_000_u64,
            20_000_000,
            30_000_000,
            40_000_000,
            120_000_000,
        ];
        let summary = duration_summary(&samples);
        assert_eq!(summary["count"], 5);
        assert_eq!(summary["p50_ms"], 30.0);
        assert_eq!(summary["p95_ms"], 120.0);
        assert_eq!(summary["p99_ms"], 120.0);
        assert_eq!(summary["hitches_over_100ms"], 1);
    }

    #[test]
    fn phase31_measurement_exclusively_owns_process_lifetime() {
        assert!(fvr03_legacy_capture_controls_lifetime(false));
        assert!(!fvr03_legacy_capture_controls_lifetime(true));
    }

    #[derive(Resource, Default)]
    struct ProjectionScheduleOrder(Vec<&'static str>);

    fn record_live_gpu_tick(mut order: ResMut<ProjectionScheduleOrder>) {
        order.0.push("live-gpu-tick");
    }

    fn record_authoritative_projection(mut order: ResMut<ProjectionScheduleOrder>) {
        order.0.push("authoritative-projection");
    }

    fn record_procedural_animation(mut order: ResMut<ProjectionScheduleOrder>) {
        order.0.push("procedural-animation");
    }

    fn record_root_reader(mut order: ResMut<ProjectionScheduleOrder>) {
        order.0.push("root-reader");
    }

    #[cfg(feature = "gpu-runtime")]
    #[derive(Resource)]
    struct ResetInputFrames {
        frames: std::collections::VecDeque<Vec<ProductionCuratedFounderResetCommand>>,
    }

    #[cfg(feature = "gpu-runtime")]
    #[derive(Resource)]
    struct BorrowedResetRuntimePort {
        calls: Vec<&'static str>,
        attempt_results: std::collections::VecDeque<CuratedFounderResetRuntimeResult>,
        retry_results: std::collections::VecDeque<CuratedFounderResetRuntimeResult>,
    }

    #[cfg(feature = "gpu-runtime")]
    impl CuratedFounderResetRuntimePort for BorrowedResetRuntimePort {
        fn dispatch_attempt(
            &mut self,
            _intent: LiveAgentResetIntent,
        ) -> CuratedFounderResetRuntimeResult {
            self.calls.push("attempt");
            self.attempt_results
                .pop_front()
                .expect("test attempt result must be available")
        }

        fn dispatch_retry(&mut self) -> CuratedFounderResetRuntimeResult {
            self.calls.push("retry");
            self.retry_results
                .pop_front()
                .expect("test retry result must be available")
        }
    }

    #[cfg(feature = "gpu-runtime")]
    fn runtime_evidence(
        status: CuratedFounderPublicationStatus,
        save_state: CuratedFounderSaveState,
        proposed_save_digest: &str,
        archive_count: usize,
    ) -> CuratedFounderResetRuntimeEvidence {
        CuratedFounderResetRuntimeEvidence {
            status,
            save_state,
            gpu_residency: CuratedFounderGpuResidencyState::Pending,
            expected_save_digest: (status
                == CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict)
                .then(|| "expected-digest".to_string()),
            actual_save_digest: (status
                == CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict)
                .then(|| "actual-digest".to_string()),
            proposed_save_digest: proposed_save_digest.to_string(),
            cause: (status == CuratedFounderPublicationStatus::ArchiveCommittedSaveFailure)
                .then(|| "save publication state unknown".to_string()),
            archive_count,
        }
    }

    #[cfg(feature = "gpu-runtime")]
    fn record_lineage_reset_input(
        mut frames: ResMut<ResetInputFrames>,
        mut commands: bevy::prelude::MessageWriter<ProductionCuratedFounderResetCommand>,
        mut order: ResMut<ProjectionScheduleOrder>,
    ) {
        let Some(frame) = frames.frames.pop_front() else {
            return;
        };
        order.0.push("lineage-input");
        for command in frame {
            commands.write(command);
        }
    }

    #[cfg(feature = "gpu-runtime")]
    fn dispatch_test_reset_commands(
        mut commands: bevy::prelude::MessageReader<ProductionCuratedFounderResetCommand>,
        mut runtime: ResMut<BorrowedResetRuntimePort>,
        mut result: ResMut<ProductionCuratedFounderResetResultResource>,
        mut order: ResMut<ProjectionScheduleOrder>,
    ) {
        let pending = commands.read().cloned().collect::<Vec<_>>();
        if pending.is_empty() {
            return;
        }
        order.0.push("reset-dispatch");
        dispatch_production_curated_founder_reset_core(&pending, &mut *runtime, &mut *result);
    }

    #[cfg(feature = "gpu-runtime")]
    fn reset_test_live_gpu_tick(mut order: ResMut<ProjectionScheduleOrder>) {
        order.0.push("live-gpu-tick");
    }

    #[cfg(feature = "gpu-runtime")]
    fn reset_test_intent() -> LiveAgentResetIntent {
        LiveAgentResetIntent {
            final_agents: vec![CuratedFounderAgentInput {
                world_entity_id: WorldEntityId(7),
                organism_id: Some(OrganismId(11)),
                final_population_slot: 0,
                legacy_genome_id: None,
            }],
        }
    }

    fn empty_scene() -> Fvr03ProductionVoxelSceneResource {
        Fvr03ProductionVoxelSceneResource {
            schema: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA,
            schema_version: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA_VERSION,
            snapshot_schema: FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA.to_string(),
            profile_id: ProductionFrontendProfileId::MinimumSettings30x30,
            population: 30,
            renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE.to_string(),
            backend_id: FVR10_RENDERER_BACKEND_ID,
            uses_internal_voxel_terrain_mesh: true,
            visible_chunk_count: 1,
            resident_chunk_count: 1,
            tile_mesh_count: 4,
            creature_render_count: 1,
            creature_material_bucket_count: 1,
            creature_lod: Fvr04CreatureLod::CompactVoxel,
            creature_root_count: 1,
            creature_part_entity_count: CreaturePartSlot::ALL.len(),
            creature_join_cover_count: 0,
            creature_part_family_count: 1,
            creature_mixed_assembly_count: 0,
            creature_shared_mesh_handle_count: CreaturePartSlot::ALL.len(),
            selection_ref_count: 1,
            dirty_chunk_count: 0,
            estimated_resident_bytes: 128 * 1024,
            draw_radius_chunks: 2,
            target_fps: 30,
            performance_artifact_path: None,
            no_renderer_authority_over_world_truth: true,
            material_counts: BTreeMap::new(),
            average_resource_bias: 0.0,
            average_hazard_pressure: 0.0,
            production_dressing_count: 4,
            production_vfx_marker_count: 8,
            production_gpu_vfx_emitter_count: 0,
            production_vfx_budget_state: "conservative",
            production_visuals_display_only: true,
            production_vfx_uses_hanabi_gpu_particles: cfg!(feature = "vfx-hanabi"),
            mesh_stats: Fvr09TerrainMeshStats {
                mode: Fvr09MesherMode::LayeredGridQuads,
                visible_voxels: 4,
                naive_visible_faces: 24,
                emitted_quads: 18,
                face_reduction_ratio: 1.333,
                remesh_time_micros: 0,
                dirty_chunks: 0,
                cached_chunks: 1,
                skipped_chunks: 0,
                remesh_budget_chunks_per_frame: 4,
                material_palette_version: FVR10_VISIBLE_SURFACE_VARIATION_VERSION,
                vertex_color_face_variation: true,
                top_side_color_separation: true,
                variation_bucket_count: 4,
                cache_key: "test-profile;palette=fvr10-visible-surface-variation-v1".to_string(),
            },
            visible_tiles: BTreeSet::new(),
            visible_chunks: BTreeSet::from([VoxelChunkCoord { x: 0, z: 0 }]),
            tile_summaries_by_tile: BTreeMap::new(),
            creature_refs_by_tile: BTreeMap::new(),
            selection_positions_by_raw_id: BTreeMap::new(),
        }
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn production_runtime_load_request_is_one_shot() {
        let mut request = ProductionRuntimeLoadRequest::default();

        assert!(request.queue());
        assert!(!request.queue());
        assert!(request.take());
        assert!(!request.take());
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn production_runtime_load_commit_cleanup_removes_old_roots_and_vfx() {
        let mut world = World::new();
        world.spawn((
            ProductionCreatureAssemblyRoot {
                stable_id: WorldEntityId(1),
                organism_id: OrganismId(1),
                display_only: true,
            },
            Fvr04ProductionRuntimeSceneRoot,
        ));
        world.spawn((
            Fvr07ProductionGpuVfxMarker {
                kind: Fvr07ProductionVfxKind::SleepGlow,
                tile: None,
                stable_id: None,
                follows_creature: false,
                display_only: true,
                no_renderer_authority_over_actions_or_cognition: true,
                budget_state: "test",
                base_translation: Vec3::ZERO,
                base_scale: Vec3::ONE,
                phase: 0.0,
            },
            Fvr04ProductionRuntimeSceneRoot,
        ));

        despawn_fvr04_runtime_scene(&mut world);

        assert_eq!(
            world
                .query::<&ProductionCreatureAssemblyRoot>()
                .iter(&world)
                .count(),
            0
        );
        assert_eq!(
            world
                .query::<&Fvr07ProductionGpuVfxMarker>()
                .iter(&world)
                .count(),
            0
        );
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn production_runtime_load_commit_clears_selection_and_follow_focus() {
        let mut world = World::new();
        let tile = VoxelTileCoord::new(0, 0);
        let selected = StableVoxelObjectRef {
            kind: StableVoxelRefKind::Tile,
            stable_id: None,
            chunk: VoxelChunkCoord::for_tile(16, tile),
            tile: Some(tile),
        };
        world.insert_resource(Fvr03ProductionVoxelSelectionResource {
            hovered: Some(selected),
            selected: Some(selected),
        });
        world.insert_resource(Fvr04ProductionCreatureFollowResource {
            enabled: true,
            target_stable_id: Some(WorldEntityId(1)),
        });

        clear_production_load_focus(&mut world);

        let selection = world
            .get_resource::<Fvr03ProductionVoxelSelectionResource>()
            .expect("selection resource");
        assert!(selection.hovered.is_none());
        assert!(selection.selected.is_none());
        let follow = world
            .get_resource::<Fvr04ProductionCreatureFollowResource>()
            .expect("follow resource");
        assert!(!follow.enabled);
        assert!(follow.target_stable_id.is_none());
    }

    #[test]
    fn fvr05_overlay_toggles_do_not_change_scene_signature() {
        let scene = empty_scene();
        let before = scene.stable_sim_signature();
        let mut overlays = Fvr05ProductionOverlayKind::default_enabled_for_profile(
            ProductionFrontendProfileId::MinimumSettings30x30,
        );
        overlays.retain(|kind| *kind != Fvr05ProductionOverlayKind::Danger);
        overlays.push(Fvr05ProductionOverlayKind::ChunkBoundaries);
        overlays.sort();
        assert_eq!(scene.stable_sim_signature(), before);
        assert!(scene.no_renderer_authority_over_world_truth);
        assert!(scene.production_visuals_display_only);
    }

    #[test]
    fn ambient_gpu_vfx_budget_stays_subtle_at_floor_profiles() {
        let minimum = fvr07_hanabi_budget(ProductionFrontendProfileId::MinimumSettings30x30);
        let comfort = fvr07_hanabi_budget(ProductionFrontendProfileId::MinSpecComfort1080p);

        assert!(minimum.emitter_cap <= 2);
        assert!(minimum.rate <= 3.0);
        assert!(comfort.emitter_cap <= 4);
        assert!(comfort.rate <= 6.0);
        assert!(minimum.alpha_scale <= 0.50);
        assert!(comfort.alpha_scale <= 0.50);
        assert!(minimum.particle_size <= 0.10);
        assert!(comfort.particle_size <= 0.11);
    }

    #[test]
    fn floor_profiles_allocate_a_lush_but_bounded_dressing_budget() {
        let minimum = Fvr03ProductionVoxelRendererSettings::for_profile(
            ProductionFrontendProfileId::MinimumSettings30x30,
        );
        let comfort = Fvr03ProductionVoxelRendererSettings::for_profile(
            ProductionFrontendProfileId::MinSpecComfort1080p,
        );
        let high = Fvr03ProductionVoxelRendererSettings::for_profile(
            ProductionFrontendProfileId::HighSpecScaleUp,
        );

        assert_eq!(minimum.production_dressing_cap, 64);
        assert_eq!(comfort.production_dressing_cap, 224);
        assert!(comfort.production_dressing_cap < high.production_dressing_cap);
        assert!(high.production_dressing_cap <= 384);
    }

    #[test]
    fn visual_capture_readiness_rejects_missing_canonical_presentation() {
        assert!(!fvr03_visual_capture_ready(7, 8, None, None));
        assert!(!fvr03_visual_capture_ready(8, 8, None, None));
        assert!(!fvr03_visual_capture_ready(u32::MAX, 8, None, None));
    }
    #[test]
    fn developer_capture_prioritizes_gpu_runtime_evidence() {
        assert_eq!(
            fvr05_screenshot_step(0),
            Some(("fvr05_gpu_panel", Fvr05ProductionInspectorTab::GpuRuntime))
        );
    }

    #[test]
    fn production_presentation_schedule_orders_tick_projection_animation_and_readers() {
        let mut app = App::new();
        configure_production_voxel_presentation_schedule(&mut app);
        app.insert_resource(ProjectionScheduleOrder::default())
            .add_systems(
                Update,
                (
                    record_live_gpu_tick.in_set(ProductionVoxelPresentationSet::LiveGpuTick),
                    record_authoritative_projection
                        .in_set(ProductionVoxelPresentationSet::AuthoritativeProjection),
                    record_procedural_animation
                        .in_set(ProductionVoxelPresentationSet::ProceduralAnimation),
                    record_root_reader.in_set(ProductionVoxelPresentationSet::RootReaders),
                ),
            );

        app.update();

        assert_eq!(
            app.world().resource::<ProjectionScheduleOrder>().0,
            vec![
                "live-gpu-tick",
                "authoritative-projection",
                "procedural-animation",
                "root-reader",
            ]
        );
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn production_curated_reset_dispatch_orders_input_before_live_gpu_tick() {
        let mut app = App::new();
        app.add_message::<ProductionCuratedFounderResetCommand>()
            .insert_resource(ProductionCuratedFounderResetResultResource::default())
            .insert_resource(ProjectionScheduleOrder::default())
            .insert_resource(ResetInputFrames {
                frames: std::collections::VecDeque::from([
                    vec![ProductionCuratedFounderResetCommand::Attempt(
                        reset_test_intent(),
                    )],
                    vec![ProductionCuratedFounderResetCommand::Retry],
                    vec![
                        ProductionCuratedFounderResetCommand::Attempt(reset_test_intent()),
                        ProductionCuratedFounderResetCommand::Retry,
                    ],
                    vec![ProductionCuratedFounderResetCommand::Attempt(
                        reset_test_intent(),
                    )],
                    vec![ProductionCuratedFounderResetCommand::Retry],
                ]),
            })
            .insert_resource(BorrowedResetRuntimePort {
                calls: Vec::new(),
                attempt_results: std::collections::VecDeque::from([
                    Ok(runtime_evidence(
                        CuratedFounderPublicationStatus::ArchiveCommittedSaveConflict,
                        CuratedFounderSaveState::Conflict,
                        "proposed-digest",
                        3,
                    )),
                    Err(CuratedFounderResetRuntimeError::DurableRefresh {
                        evidence: runtime_evidence(
                            CuratedFounderPublicationStatus::Published,
                            CuratedFounderSaveState::Verified,
                            "refresh-proposed-digest",
                            4,
                        ),
                        error: crate::GameAppShellError::InvalidProductionFrontend {
                            message: "refresh failed".to_string(),
                        },
                    }),
                ]),
                retry_results: std::collections::VecDeque::from([
                    Ok(runtime_evidence(
                        CuratedFounderPublicationStatus::ArchiveCommittedSaveFailure,
                        CuratedFounderSaveState::Unknown,
                        "retry-proposed-digest",
                        3,
                    )),
                    Err(
                        CuratedFounderResetRuntimeError::DurableCheckpointNotification {
                            evidence: runtime_evidence(
                                CuratedFounderPublicationStatus::AlreadyApplied,
                                CuratedFounderSaveState::Verified,
                                "checkpoint-proposed-digest",
                                5,
                            ),
                            error: crate::GameAppShellError::InvalidProductionFrontend {
                                message: "checkpoint notification failed".to_string(),
                            },
                        },
                    ),
                ]),
            });
        configure_production_voxel_presentation_schedule(&mut app);
        app.add_systems(
            Update,
            (
                record_lineage_reset_input.in_set(ProductionVoxelPresentationSet::Input),
                dispatch_test_reset_commands
                    .in_set(ProductionVoxelPresentationSet::Input)
                    .after(record_lineage_reset_input)
                    .before(ProductionVoxelPresentationSet::LiveGpuTick),
                reset_test_live_gpu_tick.in_set(ProductionVoxelPresentationSet::LiveGpuTick),
            ),
        );

        app.update();
        assert_eq!(
            app.world().resource::<BorrowedResetRuntimePort>().calls,
            vec!["attempt"]
        );
        assert_eq!(
            app.world().resource::<ProjectionScheduleOrder>().0,
            vec!["lineage-input", "reset-dispatch", "live-gpu-tick"]
        );
        assert_eq!(
            app.world()
                .resource::<ProductionCuratedFounderResetResultResource>()
                .outcome,
            CuratedFounderResetDispatchResult::Conflict {
                expected_save_digest: "expected-digest".to_string(),
                actual_save_digest: "actual-digest".to_string(),
                proposed_save_digest: "proposed-digest".to_string(),
                archive_count: 3,
                save_state: CuratedFounderSaveState::Conflict,
                gpu_residency: CuratedFounderGpuResidencyState::Pending,
                retryable: true,
            }
        );

        app.update();
        assert_eq!(
            app.world().resource::<BorrowedResetRuntimePort>().calls,
            vec!["attempt", "retry"]
        );
        assert_eq!(
            app.world().resource::<ProjectionScheduleOrder>().0,
            vec![
                "lineage-input",
                "reset-dispatch",
                "live-gpu-tick",
                "lineage-input",
                "reset-dispatch",
                "live-gpu-tick",
            ]
        );
        assert_eq!(
            app.world()
                .resource::<ProductionCuratedFounderResetResultResource>()
                .outcome,
            CuratedFounderResetDispatchResult::Unknown {
                cause: "save publication state unknown".to_string(),
                proposed_save_digest: "retry-proposed-digest".to_string(),
                archive_count: 3,
                save_state: CuratedFounderSaveState::Unknown,
                gpu_residency: CuratedFounderGpuResidencyState::Pending,
                retryable: true,
            }
        );

        app.update();
        assert_eq!(
            app.world().resource::<BorrowedResetRuntimePort>().calls,
            vec!["attempt", "retry"]
        );
        assert_eq!(
            app.world()
                .resource::<ProductionCuratedFounderResetResultResource>()
                .outcome,
            CuratedFounderResetDispatchResult::PreCommitRejected {
                rejection: CuratedFounderResetDispatchRejection::MultipleCommands,
            }
        );
        assert_eq!(
            app.world().resource::<ProjectionScheduleOrder>().0,
            vec![
                "lineage-input",
                "reset-dispatch",
                "live-gpu-tick",
                "lineage-input",
                "reset-dispatch",
                "live-gpu-tick",
                "lineage-input",
                "reset-dispatch",
                "live-gpu-tick",
            ]
        );

        app.update();
        assert_eq!(
            app.world().resource::<BorrowedResetRuntimePort>().calls,
            vec!["attempt", "retry", "attempt"]
        );
        match &app
            .world()
            .resource::<ProductionCuratedFounderResetResultResource>()
            .outcome
        {
            CuratedFounderResetDispatchResult::Unknown {
                cause,
                proposed_save_digest,
                archive_count,
                save_state,
                gpu_residency,
                retryable,
            } => {
                assert!(cause.contains("durable publication refresh failed"));
                assert!(cause.contains("retry the retained operation"));
                assert_eq!(proposed_save_digest, "refresh-proposed-digest");
                assert_eq!(*archive_count, 4);
                assert_eq!(*save_state, CuratedFounderSaveState::Unknown);
                assert_eq!(*gpu_residency, CuratedFounderGpuResidencyState::Pending);
                assert!(*retryable);
            }
            other => panic!("durable refresh must not project as pre-commit: {other:?}"),
        }

        app.update();
        assert_eq!(
            app.world().resource::<BorrowedResetRuntimePort>().calls,
            vec!["attempt", "retry", "attempt", "retry"]
        );
        match &app
            .world()
            .resource::<ProductionCuratedFounderResetResultResource>()
            .outcome
        {
            CuratedFounderResetDispatchResult::Unknown {
                cause,
                proposed_save_digest,
                archive_count,
                save_state,
                gpu_residency,
                retryable,
            } => {
                assert!(cause.contains("durable checkpoint notification failed"));
                assert!(cause.contains("manual recovery is required"));
                assert_eq!(proposed_save_digest, "checkpoint-proposed-digest");
                assert_eq!(*archive_count, 5);
                assert_eq!(*save_state, CuratedFounderSaveState::Verified);
                assert_eq!(*gpu_residency, CuratedFounderGpuResidencyState::Pending);
                assert!(!retryable);
            }
            other => {
                panic!("durable checkpoint notification must not project as pre-commit: {other:?}")
            }
        }
    }

    #[test]
    fn socket_transform_preserves_catalog_translation_rotation_and_scale() {
        let half_turn = std::f32::consts::FRAC_PI_4;
        let transform = socket_transform_to_bevy(
            CreaturePartSlot::LeftArm,
            SocketFrame {
                translation: [-0.08, 0.2, 0.3],
                rotation_xyzw: [0.0, 0.0, half_turn.sin(), half_turn.cos()],
                scale: [2.0, 3.0, 4.0],
            },
            [0.5, 0.25, 0.125],
        );

        assert!((transform.translation - Vec3::new(-0.08, 0.3, -0.2)).length() < 1.0e-5);
        assert!((transform.scale - Vec3::new(1.0, 0.5, 0.75)).length() < 1.0e-5);
        assert!((transform.rotation * Vec3::X - Vec3::NEG_Z).length() < 1.0e-5);
    }

    fn presentation_frame(
        kind: WorldObjectKind,
        stable_id: WorldEntityId,
        organism_id: Option<OrganismId>,
        position: Vec3f,
    ) -> LiveBrainPresentationFrame {
        let world = HeadlessScenarioBuilder::new(13)
            .agent("agent", OrganismId(7), Vec3f::ZERO)
            .build()
            .expect("projection fixture world must build");
        let mut object = world
            .object_snapshots()
            .into_iter()
            .next()
            .expect("projection fixture must contain an agent");
        object.id = stable_id;
        object.kind = kind;
        object.organism_id = organism_id;
        object.position = position;
        LiveBrainPresentationFrame::try_new(Vec::new(), Tick::new(3), vec![object])
            .expect("projection fixture frame must be valid")
    }

    #[test]
    fn authoritative_projection_maps_stable_agent_to_voxel_center_and_preserves_root_state() {
        let stable_id = WorldEntityId(41);
        let organism_id = OrganismId(7);
        let frame = presentation_frame(
            WorldObjectKind::Agent,
            stable_id,
            Some(organism_id),
            Vec3f::new(1.6, 99.0, -2.6),
        );
        let rotation = Quat::from_rotation_x(0.4);
        let scale = Vec3::new(2.0, 3.0, 4.0);
        let mut transform = Transform {
            translation: Vec3::new(-8.0, 1.75, 9.0),
            rotation,
            scale,
        };

        assert!(project_authoritative_creature_root_transform(
            stable_id,
            organism_id,
            &mut transform,
            &frame,
        ));
        assert_eq!(transform.translation, Vec3::new(2.5, 1.75, -2.5));
        assert_eq!(transform.rotation, rotation);
        assert_eq!(transform.scale, scale);
    }

    #[test]
    fn authoritative_projection_ignores_unmatched_ids_and_non_agents() {
        let stable_id = WorldEntityId(41);
        let organism_id = OrganismId(7);
        let original = Transform {
            translation: Vec3::new(4.0, 1.75, 5.0),
            rotation: Quat::from_rotation_z(0.2),
            scale: Vec3::splat(1.5),
        };

        let unmatched_frame = presentation_frame(
            WorldObjectKind::Agent,
            WorldEntityId(42),
            Some(organism_id),
            Vec3f::new(8.0, 0.0, 9.0),
        );
        let mut unmatched_transform = original;
        assert!(!project_authoritative_creature_root_transform(
            stable_id,
            organism_id,
            &mut unmatched_transform,
            &unmatched_frame,
        ));
        assert_eq!(unmatched_transform, original);

        let wrong_identity_frame = presentation_frame(
            WorldObjectKind::Agent,
            stable_id,
            Some(OrganismId(8)),
            Vec3f::new(8.0, 0.0, 9.0),
        );
        let mut wrong_identity_transform = original;
        assert!(!project_authoritative_creature_root_transform(
            stable_id,
            organism_id,
            &mut wrong_identity_transform,
            &wrong_identity_frame,
        ));
        assert_eq!(wrong_identity_transform, original);

        let non_agent_frame = presentation_frame(
            WorldObjectKind::Food,
            stable_id,
            None,
            Vec3f::new(-8.0, 0.0, -9.0),
        );
        let mut non_agent_transform = original;
        assert!(!project_authoritative_creature_root_transform(
            stable_id,
            organism_id,
            &mut non_agent_transform,
            &non_agent_frame,
        ));
        assert_eq!(non_agent_transform, original);
    }
}
