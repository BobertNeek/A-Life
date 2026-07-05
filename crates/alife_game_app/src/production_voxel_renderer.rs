//! FVR03 production voxel renderer.
//!
//! This module is Bevy-facing presentation code. It mirrors the persistent
//! voxel truth owned by `alife_world` into selectable chunk/tile meshes without
//! moving renderer handles, Bevy entities, or wgpu state into core/world data.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    time::Instant,
};

use alife_core::Vec3f;
use alife_world::{
    persistence::{CreatureSaveState, PortableSaveFile},
    CreatureWorldAnchor, PersistentVoxelWorldBackend, PersistentVoxelWorldSnapshot,
    ProceduralTerrainMaterial, ProceduralTileCoord, ProceduralWorldConfig, StableVoxelObjectRef,
    StableVoxelRefKind, VoxelChunkCoord, VoxelTileCoord, WorldObjectKind,
    FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA,
};
use bevy::{
    app::AppExit,
    asset::RenderAssetUsages,
    camera::ScalingMode,
    core_pipeline::tonemapping::Tonemapping,
    math::primitives::InfinitePlane3d,
    mesh::Indices,
    prelude::{
        default, AlphaMode, App, Assets, BackgroundColor, ButtonInput, Camera, Camera3d,
        ClearColorConfig, Color, Commands, Component, Cuboid, DetectChanges, DirectionalLight,
        GlobalTransform, Handle, KeyCode, Mesh, Mesh3d, MeshMaterial3d, MessageWriter, MouseButton,
        Name, Node, OrthographicProjection, PositionType, Projection, Quat, Res, ResMut, Resource,
        StandardMaterial, Text, Text2d, TextColor, TextFont, Time, Transform, Update, Val, Vec3,
        Visibility, Window, With,
    },
    render::{
        render_resource::PrimitiveTopology,
        view::{
            screenshot::{save_to_disk, Screenshot},
            Msaa,
        },
    },
    window::PrimaryWindow,
};

use crate::{
    creature_visual_snapshot_from_parts, production_voxel_save_with_population,
    CreatureAnimationState, CreatureExpressionState, CreatureVisualSnapshot, GameAppShellError,
    ProductionFrontendProfileId, ProductionVoxelLaunchSummary, PRODUCTION_VOXEL_RENDERER_PROFILE,
};

pub const FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA: &str = "alife.fvr03.production_voxel_renderer.v1";
pub const FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA_VERSION: u16 = 1;
pub const FVR03_RENDERER_BACKEND_ID: &str = "bevy_voxel_world+fvr03_chunk_mesh";
pub const FVR03_PERFORMANCE_ARTIFACT_DIR: &str = "target/artifacts/fvr03";
pub const FVR04_PRODUCTION_CREATURE_RENDERER_SCHEMA: &str =
    "alife.fvr04.production_creature_renderer.v1";
pub const FVR04_PRODUCTION_CREATURE_RENDERER_SCHEMA_VERSION: u16 = 1;
pub const FVR04_RENDERER_BACKEND_ID: &str =
    "bevy_voxel_world+fvr03_chunk_mesh+fvr04_creature_interaction";

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
    ChunkBoundary,
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
            Self::ChunkBoundary => "chunk-boundary",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fvr03ProductionVoxelMaterialEntry {
    pub kind: Fvr03ProductionVoxelMaterialKind,
    pub label: &'static str,
    pub rgba: [f32; 4],
    pub roughness: f32,
}

impl Fvr03ProductionVoxelMaterialEntry {
    fn standard_material(self) -> StandardMaterial {
        StandardMaterial {
            base_color: Color::srgba(self.rgba[0], self.rgba[1], self.rgba[2], self.rgba[3]),
            perceptual_roughness: self.roughness,
            metallic: 0.0,
            cull_mode: None,
            alpha_mode: if self.rgba[3] < 1.0 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            },
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
    pub show_chunk_boundaries: bool,
    pub minimum_floor: bool,
    pub min_spec_comfort_default: bool,
    pub research_scale: bool,
    pub default_camera_modes: Vec<Fvr03ProductionVoxelCameraMode>,
}

impl Fvr03ProductionVoxelRendererSettings {
    pub fn for_profile(profile_id: ProductionFrontendProfileId) -> Self {
        let budget = profile_id.budget();
        let draw_radius_chunks = budget.chunk_activation_radius;
        let resident_chunk_budget = budget.active_chunk_cap;
        let tile_stride = match profile_id {
            ProductionFrontendProfileId::MinimumSettings30x30 => 4,
            ProductionFrontendProfileId::MinSpecComfort1080p => 16,
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
            show_chunk_boundaries: !matches!(
                profile_id,
                ProductionFrontendProfileId::MinSpecComfort1080p
                    | ProductionFrontendProfileId::HighSpecScaleUp
                    | ProductionFrontendProfileId::ResearchScale
            ),
            minimum_floor: budget.hard_floor,
            min_spec_comfort_default: budget.comfort_default,
            research_scale: budget.research_mode,
            default_camera_modes: vec![
                Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
                Fvr03ProductionVoxelCameraMode::Orbit,
            ],
        }
    }

    pub fn material_palette(&self) -> Vec<Fvr03ProductionVoxelMaterialEntry> {
        vec![
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::SafeGrass,
                label: "safe-grass",
                rgba: [0.22, 0.55, 0.30, 1.0],
                roughness: 0.92,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Soil,
                label: "soil",
                rgba: [0.42, 0.30, 0.20, 1.0],
                roughness: 0.96,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Resource,
                label: "resource",
                rgba: [0.18, 0.68, 0.52, 1.0],
                roughness: 0.74,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Hazard,
                label: "hazard",
                rgba: [0.67, 0.16, 0.43, 1.0],
                roughness: 0.72,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Decay,
                label: "decay",
                rgba: [0.32, 0.18, 0.39, 1.0],
                roughness: 0.88,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Stone,
                label: "stone",
                rgba: [0.46, 0.49, 0.47, 1.0],
                roughness: 0.98,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Water,
                label: "water",
                rgba: [0.12, 0.35, 0.62, 0.82],
                roughness: 0.34,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Sand,
                label: "sand",
                rgba: [0.71, 0.62, 0.39, 1.0],
                roughness: 0.90,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Creature,
                label: "creature",
                rgba: [0.90, 0.80, 0.36, 1.0],
                roughness: 0.66,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::Selection,
                label: "selection",
                rgba: [1.0, 0.86, 0.18, 0.58],
                roughness: 0.48,
            },
            Fvr03ProductionVoxelMaterialEntry {
                kind: Fvr03ProductionVoxelMaterialKind::ChunkBoundary,
                label: "chunk-boundary",
                rgba: [0.04, 0.05, 0.05, 0.52],
                roughness: 0.80,
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
    pub spawn_affordance_cues: bool,
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
            spawn_affordance_cues: matches!(
                profile_id,
                ProductionFrontendProfileId::MinimumSettings30x30
                    | ProductionFrontendProfileId::Balanced1080p
            ) && requested_population <= 100,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fvr04CreatureExpressionSample {
    pub stable_id: alife_core::WorldEntityId,
    pub organism_id: alife_core::OrganismId,
    pub hunger: f32,
    pub fatigue: f32,
    pub fear: f32,
    pub cortisol: f32,
    pub dopamine: f32,
    pub reproductive_drive: f32,
    pub sleep_pressure: f32,
    pub social: f32,
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
            "Creature {}\norg: {} | {} / {}\nhunger {:.2} fatigue {:.2} fear {:.2}\ndopamine {:.2} cortisol {:.2} repro {:.2}\nsleep {:.2} social {:.2}",
            sample.stable_id.raw(),
            sample.organism_id.raw(),
            sample.animation.label(),
            sample.expression.label(),
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
    pub uses_bevy_voxel_world_backend: bool,
    pub uses_internal_chunk_mesh_for_fvr02_contract: bool,
    pub visible_chunk_count: usize,
    pub resident_chunk_count: usize,
    pub tile_mesh_count: usize,
    pub creature_render_count: usize,
    pub creature_material_bucket_count: usize,
    pub creature_lod: Fvr04CreatureLod,
    pub selection_ref_count: usize,
    pub dirty_chunk_count: usize,
    pub estimated_resident_bytes: usize,
    pub draw_radius_chunks: u16,
    pub target_fps: u16,
    pub performance_artifact_path: Option<PathBuf>,
    pub no_renderer_authority_over_world_truth: bool,
    visible_tiles: BTreeSet<VoxelTileCoord>,
    visible_chunks: BTreeSet<VoxelChunkCoord>,
    creature_refs_by_tile: BTreeMap<VoxelTileCoord, StableVoxelObjectRef>,
    selection_positions_by_raw_id: BTreeMap<u64, Vec3>,
}

impl Fvr03ProductionVoxelSceneResource {
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
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct Fvr03ProductionVoxelChunk {
    pub coord: VoxelChunkCoord,
    pub signature: String,
    pub lod_level: u8,
    pub dirty_generation: u64,
    pub sampled_tiles: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr03ProductionVoxelTerrainTile {
    pub tile: VoxelTileCoord,
    pub chunk: VoxelChunkCoord,
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub height_units: f32,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
    pub stable_ref: StableVoxelObjectRef,
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
    pub phase: f32,
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
pub struct Fvr04ProductionCreatureInspectorPanel;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct Fvr03ProductionVoxelTerrainBatch {
    pub material: Fvr03ProductionVoxelMaterialKind,
    pub tile_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Fvr03BatchedTerrainTile {
    center_x: f32,
    center_z: f32,
    height: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct Fvr04CreatureVisualRecord {
    stable_ref: StableVoxelObjectRef,
    tile: VoxelTileCoord,
    social_affinity: f32,
    reproductive_drive: f32,
    visual: CreatureVisualSnapshot,
}

#[derive(Debug, Clone, PartialEq)]
struct Fvr04RuntimeSceneState {
    snapshot: PersistentVoxelWorldSnapshot,
    creatures: Vec<Fvr04CreatureVisualRecord>,
}

pub fn spawn_fvr03_production_voxel_scene(
    app: &mut App,
    summary: &ProductionVoxelLaunchSummary,
) -> Result<(), GameAppShellError> {
    let settings = Fvr03ProductionVoxelRendererSettings::for_profile(summary.profile_id);
    let creature_settings = Fvr04ProductionCreatureRendererSettings::for_profile(
        summary.profile_id,
        summary.effective_population,
    );
    let runtime_state = load_fvr04_runtime_state(summary)?;
    let snapshot = runtime_state.snapshot.clone();
    let visible_chunks = snapshot
        .visible_chunks
        .iter()
        .map(|chunk| chunk.coord)
        .collect::<BTreeSet<_>>();
    let procedural_config = procedural_config_from_snapshot(&snapshot);

    #[cfg(feature = "voxel-backend")]
    {
        let voxel_config = Fvr03BevyVoxelWorldConfig {
            seed: snapshot.world_seed,
            procedural_config,
            visible_chunks: visible_chunks.clone(),
            settings: settings.clone(),
        };
        app.add_plugins(bevy_voxel_world::prelude::VoxelWorldPlugin::<
            Fvr03BevyVoxelWorldConfig,
        >::minimal());
        app.insert_resource(voxel_config);
    }

    let palette = settings.material_palette();
    let materials = create_fvr03_materials(app, &palette);
    let boundary_mesh = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        meshes.add(Cuboid::new(
            f32::from(snapshot.profile_budget.chunk_tile_size),
            0.035,
            f32::from(snapshot.profile_budget.chunk_tile_size),
        ))
    };
    let creature_mesh = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        meshes.add(Cuboid::new(0.92, 1.28, 0.92))
    };
    let creature_cue_mesh = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        meshes.add(Cuboid::new(0.30, 0.30, 0.30))
    };
    let mut visible_tiles = BTreeSet::new();
    let mut terrain_batches =
        BTreeMap::<Fvr03ProductionVoxelMaterialKind, Vec<Fvr03BatchedTerrainTile>>::new();
    let mut tile_mesh_count = 0_usize;
    for chunk in &snapshot.visible_chunks {
        let sampled_tiles = spawn_fvr03_chunk_tiles(
            app,
            &snapshot,
            procedural_config,
            &settings,
            chunk.coord,
            &mut visible_tiles,
            &mut terrain_batches,
        )?;
        tile_mesh_count = tile_mesh_count.saturating_add(sampled_tiles);
        if settings.show_chunk_boundaries {
            spawn_fvr03_chunk_boundary(
                app,
                &materials,
                boundary_mesh.clone(),
                chunk.coord,
                snapshot.profile_budget.chunk_tile_size,
            );
        }
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life FVR03 resident voxel chunk {}:{}",
                chunk.coord.x, chunk.coord.z
            )),
            Transform::default(),
            Visibility::Hidden,
            Fvr03ProductionVoxelChunk {
                coord: chunk.coord,
                signature: chunk.signature.0.clone(),
                lod_level: fvr03_lod_for_chunk(chunk.coord),
                dirty_generation: chunk.dirty_generation,
                sampled_tiles,
            },
        ));
    }

    spawn_fvr03_batched_terrain_meshes(app, &materials, settings.tile_stride, &terrain_batches);
    let creature_scene = spawn_fvr04_creatures(
        app,
        &runtime_state.creatures,
        &creature_settings,
        creature_mesh,
        creature_cue_mesh,
    );
    spawn_fvr03_camera(app, &settings);
    spawn_fvr03_lighting(app, &settings);

    let selected = runtime_state
        .creatures
        .first()
        .map(|creature| creature.stable_ref)
        .or_else(|| {
            visible_tiles
                .iter()
                .copied()
                .find_map(|tile| snapshot.lookup_tile(tile))
        })
        .or_else(|| {
            snapshot
                .selection_refs
                .iter()
                .copied()
                .find(|reference| reference.tile.is_some())
        });
    if let Some(selection) = selected {
        spawn_fvr03_selection_marker(app, &materials, selection);
    }

    let mut scene = Fvr03ProductionVoxelSceneResource {
        schema: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA,
        schema_version: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA_VERSION,
        snapshot_schema: snapshot.schema.clone(),
        profile_id: summary.profile_id,
        population: summary.effective_population,
        renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE.to_string(),
        backend_id: FVR04_RENDERER_BACKEND_ID,
        uses_bevy_voxel_world_backend: cfg!(feature = "voxel-backend"),
        uses_internal_chunk_mesh_for_fvr02_contract: true,
        visible_chunk_count: snapshot.visible_chunks.len(),
        resident_chunk_count: snapshot.visible_chunks.len(),
        tile_mesh_count,
        creature_render_count: creature_scene.rendered_creature_count,
        creature_material_bucket_count: creature_scene.material_bucket_count,
        creature_lod: creature_scene.lod,
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
        visible_tiles,
        visible_chunks,
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

    if summary.record_performance {
        scene.performance_artifact_path = Some(write_fvr03_performance_artifact(&scene, None)?);
    }

    app.insert_resource(scene);
    app.insert_resource(creature_scene);
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
    app.add_systems(
        Update,
        (
            handle_fvr03_mouse_selection,
            handle_fvr03_camera_mode_input,
            animate_fvr04_creatures,
            sync_fvr04_selection_marker,
            handle_fvr04_camera_follow_input,
            sync_fvr04_camera_follow,
            sync_fvr04_creature_label,
            sync_fvr04_creature_inspector_panel,
        ),
    );
    if summary.record_performance && !summary.dry_run {
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
        })
        .add_systems(Update, request_fvr03_recorded_screenshot);
    }
    spawn_fvr03_diagnostics_ui(app, summary, &settings);
    spawn_fvr04_creature_inspector_panel(app);
    spawn_fvr04_creature_world_label(app, selected);
    Ok(())
}

fn load_fvr04_runtime_state(
    summary: &ProductionVoxelLaunchSummary,
) -> Result<Fvr04RuntimeSceneState, GameAppShellError> {
    let save = PortableSaveFile::from_json_file(&summary.save_path)?;
    let production_save = production_voxel_save_with_population(
        &save,
        &summary.asset_root,
        summary.profile_id,
        summary.effective_population,
    )?;
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
        let visual = creature_visual_snapshot_from_parts(
            organism_id,
            anchor.stable_id,
            position,
            None,
            None,
            &creature.mind.homeostasis,
            fvr04_sleep_phase_from_creature_save(creature),
            None,
        )?;
        records.push(Fvr04CreatureVisualRecord {
            stable_ref: StableVoxelObjectRef {
                kind: StableVoxelRefKind::Creature,
                stable_id: Some(anchor.stable_id),
                chunk: anchor.chunk,
                tile: Some(anchor.tile),
            },
            tile: anchor.tile,
            social_affinity: object.social_affinity,
            reproductive_drive: creature.mind.homeostasis.drives.reproductive_drive,
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

fn create_fvr03_materials(
    app: &mut App,
    palette: &[Fvr03ProductionVoxelMaterialEntry],
) -> BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>> {
    let mut assets = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
    palette
        .iter()
        .map(|entry| (entry.kind, assets.add(entry.standard_material())))
        .collect()
}

fn spawn_fvr03_chunk_tiles(
    app: &mut App,
    snapshot: &PersistentVoxelWorldSnapshot,
    procedural_config: ProceduralWorldConfig,
    settings: &Fvr03ProductionVoxelRendererSettings,
    chunk: VoxelChunkCoord,
    visible_tiles: &mut BTreeSet<VoxelTileCoord>,
    terrain_batches: &mut BTreeMap<Fvr03ProductionVoxelMaterialKind, Vec<Fvr03BatchedTerrainTile>>,
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
            terrain_batches
                .entry(material)
                .or_default()
                .push(Fvr03BatchedTerrainTile {
                    center_x: tile.x as f32 + 0.5,
                    center_z: tile.z as f32 + 0.5,
                    height,
                });
            app.world_mut().spawn((
                Name::new(format!("A-Life FVR03 voxel tile {}:{}", tile.x, tile.z)),
                Transform::from_xyz(tile.x as f32 + 0.5, height * 0.5, tile.z as f32 + 0.5),
                Visibility::Hidden,
                Fvr03ProductionVoxelTerrainTile {
                    tile,
                    chunk,
                    material,
                    height_units: height,
                    resource_bias: sample.resource_bias,
                    hazard_pressure: sample.hazard_pressure,
                    stable_ref,
                },
            ));
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}

fn spawn_fvr03_batched_terrain_meshes(
    app: &mut App,
    materials: &BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    tile_stride: u16,
    terrain_batches: &BTreeMap<Fvr03ProductionVoxelMaterialKind, Vec<Fvr03BatchedTerrainTile>>,
) {
    let footprint = f32::from(tile_stride.max(1)) * 0.98;
    for (material, tiles) in terrain_batches {
        if tiles.is_empty() {
            continue;
        }
        let Some(material_handle) = materials.get(material).cloned() else {
            continue;
        };
        let mesh = fvr03_batched_cuboid_mesh(tiles, footprint);
        let mesh_handle = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life FVR03 batched voxel terrain {}",
                material.label()
            )),
            Mesh3d(mesh_handle),
            MeshMaterial3d(material_handle),
            Transform::default(),
            Fvr03ProductionVoxelTerrainBatch {
                material: *material,
                tile_count: tiles.len(),
            },
        ));
    }
}

fn fvr03_batched_cuboid_mesh(tiles: &[Fvr03BatchedTerrainTile], footprint: f32) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::with_capacity(tiles.len() * 24);
    let mut normals = Vec::<[f32; 3]>::with_capacity(tiles.len() * 24);
    let mut uvs = Vec::<[f32; 2]>::with_capacity(tiles.len() * 24);
    let mut indices = Vec::<u32>::with_capacity(tiles.len() * 36);
    for tile in tiles {
        fvr03_append_cuboid(
            &mut positions,
            &mut normals,
            &mut uvs,
            &mut indices,
            Vec3::new(tile.center_x, tile.height * 0.5, tile.center_z),
            Vec3::new(footprint, tile.height, footprint),
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

fn spawn_fvr03_chunk_boundary(
    app: &mut App,
    materials: &BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    mesh: Handle<Mesh>,
    coord: VoxelChunkCoord,
    chunk_tile_size: u16,
) {
    let size = f32::from(chunk_tile_size);
    let material = materials
        .get(&Fvr03ProductionVoxelMaterialKind::ChunkBoundary)
        .expect("FVR03 chunk boundary material exists")
        .clone();
    app.world_mut().spawn((
        Name::new(format!(
            "A-Life FVR03 chunk boundary {}:{}",
            coord.x, coord.z
        )),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(
            coord.x as f32 * size + size * 0.5,
            -0.02,
            coord.z as f32 * size + size * 0.5,
        ),
    ));
}

fn spawn_fvr04_creatures(
    app: &mut App,
    creatures: &[Fvr04CreatureVisualRecord],
    settings: &Fvr04ProductionCreatureRendererSettings,
    mesh: Handle<Mesh>,
    cue_mesh: Handle<Mesh>,
) -> Fvr04ProductionCreatureSceneResource {
    let mut material_handles =
        BTreeMap::<Fvr04CreatureRenderBucket, Handle<StandardMaterial>>::new();
    let mut expression_buffer = Vec::new();
    let mut stable_lookup_by_raw_id = BTreeMap::new();
    let max_visible = usize::from(settings.max_visible_creatures);
    for (index, creature) in creatures.iter().take(max_visible).enumerate() {
        let visual = &creature.visual;
        let bucket = Fvr04CreatureRenderBucket {
            expression: visual.expression,
            animation: visual.animation,
            lod: settings.lod,
        };
        let material = if let Some(handle) = material_handles.get(&bucket) {
            handle.clone()
        } else {
            let handle = app
                .world_mut()
                .resource_mut::<Assets<StandardMaterial>>()
                .add(fvr04_creature_material(visual));
            material_handles.insert(bucket, handle.clone());
            handle
        };
        let base_translation = Vec3::new(
            creature.tile.x as f32 + 0.5,
            fvr04_creature_base_height(settings.lod),
            creature.tile.z as f32 + 0.5,
        );
        let mut transform = Transform::from_translation(base_translation);
        transform.scale = fvr04_creature_scale(visual, settings.lod);
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life FVR04 creature stable {} {} {}",
                visual.stable_id.raw(),
                visual.animation.label(),
                visual.expression.label()
            )),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            transform,
            Fvr03ProductionVoxelCreatureMarker {
                stable_id: visual.stable_id,
                tile: creature.tile,
            },
            Fvr04ProductionCreatureVisualMarker {
                stable_id: visual.stable_id,
                organism_id: visual.organism_id,
                tile: creature.tile,
                expression: visual.expression,
                animation: visual.animation,
                lod: settings.lod,
                base_translation,
                phase: (index as f32 * 0.37) + (visual.stable_id.raw() % 17) as f32 * 0.11,
            },
        ));
        if settings.spawn_affordance_cues {
            app.world_mut().spawn((
                Name::new(format!(
                    "A-Life FVR04 creature cue stable {} {}",
                    visual.stable_id.raw(),
                    visual.expression.label()
                )),
                Mesh3d(cue_mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(
                    base_translation.x,
                    base_translation.y + 1.08,
                    base_translation.z,
                ),
                Fvr04ProductionCreatureAffordanceCue {
                    stable_id: visual.stable_id,
                    expression: visual.expression,
                },
            ));
        }
        stable_lookup_by_raw_id.insert(visual.stable_id.raw(), expression_buffer.len());
        expression_buffer.push(Fvr04CreatureExpressionSample {
            stable_id: visual.stable_id,
            organism_id: visual.organism_id,
            hunger: visual.cues.hunger.value,
            fatigue: visual.cues.fatigue.value,
            fear: visual.cues.fear.value,
            cortisol: visual.endocrine.cortisol,
            dopamine: visual.endocrine.dopamine,
            reproductive_drive: creature.reproductive_drive,
            sleep_pressure: visual.cues.sleep_pressure.value,
            social: ((creature.social_affinity + 1.0) * 0.5).clamp(0.0, 1.0),
            expression: visual.expression,
            animation: visual.animation,
        });
    }
    Fvr04ProductionCreatureSceneResource {
        schema: FVR04_PRODUCTION_CREATURE_RENDERER_SCHEMA,
        schema_version: FVR04_PRODUCTION_CREATURE_RENDERER_SCHEMA_VERSION,
        requested_population: settings.requested_population,
        rendered_creature_count: expression_buffer.len(),
        expression_buffer,
        material_bucket_count: material_handles.len(),
        mesh_pool_count: if settings.spawn_affordance_cues { 2 } else { 1 },
        lod: settings.lod,
        stable_lookup_by_raw_id,
        no_renderer_authority_over_actions_or_cognition: true,
        expression_buffer_is_read_only_projection: true,
    }
}

fn fvr04_creature_material(visual: &CreatureVisualSnapshot) -> StandardMaterial {
    let base = visual.base_rgba;
    let accent = visual.accent_rgba;
    let fear_boost = visual.cues.fear.value * 0.18;
    StandardMaterial {
        base_color: Color::srgba(
            (base[0] * 0.62 + accent[0] * 0.38 + fear_boost).clamp(0.0, 1.0),
            (base[1] * 0.62 + accent[1] * 0.38).clamp(0.0, 1.0),
            (base[2] * 0.62 + accent[2] * 0.38).clamp(0.0, 1.0),
            1.0,
        ),
        perceptual_roughness: 0.72,
        ..default()
    }
}

fn fvr04_creature_base_height(lod: Fvr04CreatureLod) -> f32 {
    match lod {
        Fvr04CreatureLod::FullVoxel => 1.16,
        Fvr04CreatureLod::CompactVoxel => 1.04,
        Fvr04CreatureLod::ImpostorVoxel => 0.92,
    }
}

fn fvr04_creature_scale(visual: &CreatureVisualSnapshot, lod: Fvr04CreatureLod) -> Vec3 {
    let fatigue_squash = 1.0 - visual.cues.fatigue.value * 0.18;
    let fear_narrow = 1.0 - visual.cues.fear.value * 0.10;
    let energy = 0.92 + visual.cues.energy.value * 0.14;
    let mut scale = match lod {
        Fvr04CreatureLod::FullVoxel => {
            Vec3::new(1.22 * fear_narrow, 1.14 * fatigue_squash * energy, 1.22)
        }
        Fvr04CreatureLod::CompactVoxel => {
            Vec3::new(1.02 * fear_narrow, 0.96 * fatigue_squash, 1.02)
        }
        Fvr04CreatureLod::ImpostorVoxel => {
            Vec3::new(0.82 * fear_narrow, 0.72 * fatigue_squash, 0.42)
        }
    };
    if matches!(
        visual.animation,
        CreatureAnimationState::Sleeping | CreatureAnimationState::Resting
    ) {
        scale.y *= 0.52;
        scale.x *= 1.24;
        scale.z *= 1.10;
    }
    scale
}

fn animate_fvr04_creatures(
    time: Res<Time>,
    mut creatures: bevy::prelude::Query<(&mut Transform, &Fvr04ProductionCreatureVisualMarker)>,
) {
    let seconds = time.elapsed_secs();
    for (mut transform, marker) in &mut creatures {
        let wave = (seconds * fvr04_animation_speed(marker.animation) + marker.phase).sin();
        let lateral = (seconds * 7.0 + marker.phase * 1.7).sin();
        transform.translation = marker.base_translation;
        match marker.animation {
            CreatureAnimationState::Sleeping | CreatureAnimationState::Resting => {
                transform.translation.y -= 0.22;
                transform.rotation = Quat::from_rotation_y(0.10 * wave);
            }
            CreatureAnimationState::Afraid | CreatureAnimationState::Hurt => {
                transform.translation.x += lateral * 0.035;
                transform.translation.z += wave * 0.025;
                transform.rotation = Quat::from_rotation_y(lateral * 0.12);
            }
            CreatureAnimationState::Curious | CreatureAnimationState::Inspecting => {
                transform.translation.y += wave.abs() * 0.08;
                transform.rotation = Quat::from_rotation_y(wave * 0.24);
            }
            CreatureAnimationState::Moving => {
                transform.translation.y += wave.abs() * 0.14;
                transform.rotation = Quat::from_rotation_y(wave * 0.16);
            }
            CreatureAnimationState::Interacting | CreatureAnimationState::Signaling => {
                transform.translation.y += wave.abs() * 0.10;
                transform.rotation = Quat::from_rotation_y(wave * 0.32);
            }
            CreatureAnimationState::Idle => {
                transform.translation.y += wave * 0.035;
                transform.rotation = Quat::from_rotation_y(wave * 0.05);
            }
        }
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

fn spawn_fvr03_selection_marker(
    app: &mut App,
    materials: &BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    selection: StableVoxelObjectRef,
) {
    let Some(tile) = selection.tile else {
        return;
    };
    let mesh = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        meshes.add(Cuboid::new(1.28, 0.08, 1.28))
    };
    let material = materials
        .get(&Fvr03ProductionVoxelMaterialKind::Selection)
        .expect("FVR03 selection material exists")
        .clone();
    app.world_mut().spawn((
        Name::new(format!("A-Life FVR03 selected tile {}:{}", tile.x, tile.z)),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(tile.x as f32 + 0.5, 1.42, tile.z as f32 + 0.5),
        Fvr03ProductionVoxelSelectionMarker,
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

fn spawn_fvr04_creature_inspector_panel(app: &mut App) {
    app.world_mut().spawn((
        Name::new("A-Life FVR04 creature inspector panel"),
        Text::new("Creature\nselection: none"),
        TextFont {
            font_size: 15.0,
            ..default()
        },
        TextColor(Color::srgb(0.90, 0.96, 0.86)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(18.0),
            right: Val::Px(18.0),
            max_width: Val::Px(420.0),
            padding: bevy::ui::UiRect::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.014, 0.020, 0.017, 0.84)),
        Fvr04ProductionCreatureInspectorPanel,
    ));
}

fn spawn_fvr03_camera(app: &mut App, settings: &Fvr03ProductionVoxelRendererSettings) {
    let camera_extent = 18.0 + f32::from(settings.draw_radius_chunks) * 9.0;
    let transform = fvr03_camera_transform(
        Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
        camera_extent,
    );
    app.world_mut().spawn((
        Name::new("A-Life FVR03 production voxel camera"),
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.065, 0.105, 0.090)),
            ..default()
        },
        Projection::from(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: camera_extent,
            },
            scale: 1.0,
            near: -200.0,
            far: 500.0,
            ..OrthographicProjection::default_3d()
        }),
        Tonemapping::None,
        Msaa::Off,
        transform,
        Fvr03ProductionVoxelCamera {
            mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
        },
    ));
}

fn spawn_fvr03_lighting(app: &mut App, settings: &Fvr03ProductionVoxelRendererSettings) {
    app.world_mut().spawn((
        Name::new("A-Life FVR03 warm directional sun"),
        DirectionalLight {
            illuminance: 9800.0,
            shadows_enabled: !(settings.minimum_floor || settings.min_spec_comfort_default),
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            bevy::prelude::EulerRot::XYZ,
            -1.05,
            0.62,
            -0.42,
        )),
    ));
}

fn spawn_fvr03_diagnostics_ui(
    app: &mut App,
    summary: &ProductionVoxelLaunchSummary,
    settings: &Fvr03ProductionVoxelRendererSettings,
) {
    app.world_mut().spawn((
        Name::new("A-Life FVR03 production voxel diagnostics"),
        Text::new(format!(
            "A-Life Voxel Frontend\nprofile: {} | population: {}\nrenderer: {} | backend: {}\ntarget: {} FPS | chunks radius: {} | stride: {}\ncreatures: FVR04 stable selection + expression buffer\nruntime: {} | fallback: {}\nsave: {}",
            summary.profile_id.label(),
            summary.effective_population,
            summary.renderer_profile,
            FVR04_RENDERER_BACKEND_ID,
            settings.target_fps,
            settings.draw_radius_chunks,
            settings.tile_stride,
            summary.diagnostics.selected_backend,
            summary
                .diagnostics
                .fallback_reason
                .as_deref()
                .unwrap_or("None"),
            summary
                .save_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown"),
        )),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.86, 0.96, 0.90)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(18.0),
            left: Val::Px(18.0),
            max_width: Val::Px(650.0),
            padding: bevy::ui::UiRect::all(Val::Px(12.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.026, 0.020, 0.82)),
    ));
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
    } else if keyboard.just_pressed(KeyCode::KeyI) {
        Some(Fvr03ProductionVoxelCameraMode::OrthographicIsometric)
    } else {
        None
    };
    let Some(next_mode) = next_mode else {
        return;
    };
    let extent = 18.0 + f32::from(scene.draw_radius_chunks) * 9.0;
    for (mut transform, mut projection, mut camera) in &mut cameras {
        camera.mode = next_mode;
        *transform = fvr03_camera_transform(next_mode, extent);
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
    mut markers: bevy::prelude::Query<
        (&mut Transform, &mut Visibility),
        With<Fvr03ProductionVoxelSelectionMarker>,
    >,
) {
    if !selection.is_changed() && !scene.is_changed() {
        return;
    }
    let Some(selected) = selection.selected else {
        for (_, mut visibility) in &mut markers {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let Some(position) = scene.world_position_for_selection(selected) else {
        return;
    };
    for (mut transform, mut visibility) in &mut markers {
        transform.translation = Vec3::new(position.x, 1.42, position.z);
        *visibility = Visibility::Visible;
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
    mut cameras: bevy::prelude::Query<(
        &mut Transform,
        &mut Projection,
        &Fvr03ProductionVoxelCamera,
    )>,
) {
    if !follow.enabled {
        return;
    }
    let Some(target) = follow.target_stable_id else {
        return;
    };
    let Some(position) = scene.selection_positions_by_raw_id.get(&target.raw()) else {
        return;
    };
    let target = Vec3::new(position.x, 0.0, position.z);
    let extent = 18.0 + f32::from(scene.draw_radius_chunks) * 9.0;
    for (mut transform, mut projection, camera) in &mut cameras {
        *transform = fvr04_follow_camera_transform(camera.mode, extent, target);
        if let Projection::Orthographic(orthographic) = &mut *projection {
            orthographic.scaling_mode = ScalingMode::FixedVertical {
                viewport_height: extent,
            };
        }
    }
}

fn sync_fvr04_creature_label(
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    creatures: Res<Fvr04ProductionCreatureSceneResource>,
    mut labels: bevy::prelude::Query<
        (&mut Text2d, &mut Transform, &mut Visibility),
        With<Fvr04ProductionCreatureWorldLabel>,
    >,
) {
    if !selection.is_changed() && !scene.is_changed() && !creatures.is_changed() {
        return;
    }
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
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let Some(stable_id) = target.stable_id else {
        return;
    };
    let Some(sample) = creatures.sample_for_stable_id(stable_id) else {
        return;
    };
    let Some(position) = scene.world_position_for_selection(target) else {
        return;
    };
    for (mut text, mut transform, mut visibility) in &mut labels {
        text.0 = format!(
            "#{} {} {}",
            sample.stable_id.raw(),
            sample.animation.label(),
            sample.expression.label()
        );
        transform.translation = Vec3::new(position.x, 2.35, position.z);
        *visibility = Visibility::Visible;
    }
}

fn sync_fvr04_creature_inspector_panel(
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    creatures: Res<Fvr04ProductionCreatureSceneResource>,
    follow: Res<Fvr04ProductionCreatureFollowResource>,
    mut panels: bevy::prelude::Query<&mut Text, With<Fvr04ProductionCreatureInspectorPanel>>,
) {
    if !selection.is_changed() && !creatures.is_changed() && !follow.is_changed() {
        return;
    }
    let suffix = if follow.enabled {
        "follow: on"
    } else {
        "follow: off"
    };
    let text = format!("{}\n{}", creatures.panel_text(selection.selected), suffix);
    for mut panel in &mut panels {
        panel.0 = text.clone();
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

fn request_fvr03_recorded_screenshot(
    mut commands: Commands,
    mut capture: ResMut<Fvr03ProductionVoxelScreenshotResource>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    mut exits: MessageWriter<AppExit>,
) {
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
    if capture.requested || !capture.measurement_written {
        return;
    }
    if let Some(parent) = capture.path.parent() {
        if fs::create_dir_all(parent).is_err() {
            capture.requested = true;
            exits.write(AppExit::Success);
            return;
        }
    }
    let path = capture.path.clone();
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    capture.requested = true;
}

fn fvr03_screenshot_capture_frame(_settings: &Fvr03ProductionVoxelRendererSettings) -> u32 {
    48
}

fn fvr03_camera_transform(mode: Fvr03ProductionVoxelCameraMode, extent: f32) -> Transform {
    match mode {
        Fvr03ProductionVoxelCameraMode::OrthographicIsometric => {
            Transform::from_xyz(extent * 0.56, extent * 0.82, extent * 0.58)
                .looking_at(Vec3::new(8.0, 0.0, -4.0), Vec3::Y)
        }
        Fvr03ProductionVoxelCameraMode::Orbit => {
            Transform::from_xyz(extent * 0.72, extent * 0.52, extent * 0.94)
                .looking_at(Vec3::new(8.0, 0.0, -4.0), Vec3::Y)
        }
    }
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

fn fvr03_voxel_material_index(material: ProceduralTerrainMaterial, tile: VoxelTileCoord) -> u8 {
    match fvr03_material_kind(material, tile) {
        Fvr03ProductionVoxelMaterialKind::SafeGrass => 1,
        Fvr03ProductionVoxelMaterialKind::Soil => 2,
        Fvr03ProductionVoxelMaterialKind::Resource => 3,
        Fvr03ProductionVoxelMaterialKind::Hazard => 4,
        Fvr03ProductionVoxelMaterialKind::Decay => 5,
        Fvr03ProductionVoxelMaterialKind::Stone => 6,
        Fvr03ProductionVoxelMaterialKind::Water => 7,
        Fvr03ProductionVoxelMaterialKind::Sand => 8,
        Fvr03ProductionVoxelMaterialKind::Creature
        | Fvr03ProductionVoxelMaterialKind::Selection
        | Fvr03ProductionVoxelMaterialKind::ChunkBoundary => 9,
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

fn fvr03_lod_for_chunk(coord: VoxelChunkCoord) -> u8 {
    let distance = coord.x.abs().max(coord.z.abs());
    if distance <= 2 {
        0
    } else if distance <= 5 {
        1
    } else {
        2
    }
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
        "{{\n  \"schema\": \"{}\",\n  \"profile\": \"{}\",\n  \"backend\": \"{}\",\n  \"target_fps\": {},\n  \"visible_chunks\": {},\n  \"resident_chunks\": {},\n  \"tile_mesh_count\": {},\n  \"creature_render_count\": {},\n  \"creature_material_bucket_count\": {},\n  \"creature_lod\": \"{}\",\n  \"estimated_resident_bytes\": {},\n  \"measured_fps\": {},\n  \"measured_frame_count\": {},\n  \"measured_seconds\": {},\n  \"performance_claim_status\": \"{}\"\n}}\n",
        scene.schema,
        scene.profile_id.label(),
        scene.backend_id,
        scene.target_fps,
        scene.visible_chunk_count,
        scene.resident_chunk_count,
        scene.tile_mesh_count,
        scene.creature_render_count,
        scene.creature_material_bucket_count,
        scene.creature_lod.label(),
        scene.estimated_resident_bytes,
        measured_fps,
        measured_frame_count,
        measured_seconds,
        performance_status
    );
    fs::write(&path, contents)?;
    Ok(path)
}

#[cfg(feature = "voxel-backend")]
#[derive(Debug, Clone, Resource)]
pub struct Fvr03BevyVoxelWorldConfig {
    pub seed: u64,
    pub procedural_config: ProceduralWorldConfig,
    pub visible_chunks: BTreeSet<VoxelChunkCoord>,
    pub settings: Fvr03ProductionVoxelRendererSettings,
}

#[cfg(feature = "voxel-backend")]
impl Default for Fvr03BevyVoxelWorldConfig {
    fn default() -> Self {
        let settings = Fvr03ProductionVoxelRendererSettings::for_profile(
            ProductionFrontendProfileId::MinimumSettings30x30,
        );
        Self {
            seed: 4_242,
            procedural_config: ProceduralWorldConfig::with_seed(4_242),
            visible_chunks: BTreeSet::new(),
            settings,
        }
    }
}

#[cfg(feature = "voxel-backend")]
impl bevy_voxel_world::prelude::VoxelWorldConfig for Fvr03BevyVoxelWorldConfig {
    type MaterialIndex = u8;
    type ChunkUserBundle = ();

    fn spawning_distance(&self) -> u32 {
        u32::from(self.settings.draw_radius_chunks.max(1))
    }

    fn min_despawn_distance(&self) -> u32 {
        u32::from(self.settings.hot_radius_chunks.max(1))
    }

    fn chunk_despawn_strategy(&self) -> bevy_voxel_world::prelude::ChunkDespawnStrategy {
        bevy_voxel_world::prelude::ChunkDespawnStrategy::FarAway
    }

    fn chunk_spawn_strategy(&self) -> bevy_voxel_world::prelude::ChunkSpawnStrategy {
        bevy_voxel_world::prelude::ChunkSpawnStrategy::Close
    }

    fn max_spawn_per_frame(&self) -> usize {
        usize::from(self.settings.resident_chunk_budget).min(96)
    }

    fn spawning_rays(&self) -> usize {
        match self.settings.profile_id {
            ProductionFrontendProfileId::MinimumSettings30x30 => 12,
            ProductionFrontendProfileId::MinSpecComfort1080p => 20,
            ProductionFrontendProfileId::Balanced1080p => 28,
            ProductionFrontendProfileId::HighSpecScaleUp => 36,
            ProductionFrontendProfileId::ResearchScale => 20,
        }
    }

    fn chunk_lod(
        &self,
        chunk_position: bevy::prelude::IVec3,
        _previous_lod: Option<bevy_voxel_world::prelude::LodLevel>,
        camera_position: Vec3,
    ) -> bevy_voxel_world::prelude::LodLevel {
        let center = Vec3::new(
            chunk_position.x as f32 * 32.0 + 16.0,
            chunk_position.y as f32 * 32.0 + 16.0,
            chunk_position.z as f32 * 32.0 + 16.0,
        );
        let distance = camera_position.distance(center);
        if distance < 64.0 {
            0
        } else if distance < 128.0 {
            1
        } else {
            2
        }
    }

    fn voxel_lookup_delegate(
        &self,
    ) -> bevy_voxel_world::prelude::VoxelLookupDelegate<Self::MaterialIndex> {
        let procedural_config = self.procedural_config;
        let visible_chunks = self.visible_chunks.clone();
        Box::new(move |_, _, _| {
            let visible_chunks = visible_chunks.clone();
            Box::new(move |position, _existing| {
                let tile = VoxelTileCoord::new(position.x, position.z);
                let chunk =
                    VoxelChunkCoord::for_tile(procedural_config.chunk_tile_size as u16, tile);
                if !visible_chunks.contains(&chunk) {
                    return bevy_voxel_world::prelude::WorldVoxel::Air;
                }
                let Ok(sample) = alife_world::sample_procedural_terrain_tile(
                    procedural_config,
                    ProceduralTileCoord::from(tile),
                ) else {
                    return bevy_voxel_world::prelude::WorldVoxel::Air;
                };
                let surface_height = fvr03_tile_height(
                    sample.material,
                    sample.resource_bias,
                    sample.hazard_pressure,
                    sample.roughness,
                )
                .ceil() as i32;
                if position.y < 0 || position.y > surface_height {
                    bevy_voxel_world::prelude::WorldVoxel::Air
                } else {
                    bevy_voxel_world::prelude::WorldVoxel::Solid(fvr03_voxel_material_index(
                        sample.material,
                        tile,
                    ))
                }
            })
        })
    }
}
