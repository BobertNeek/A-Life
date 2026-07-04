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
    persistence::PortableSaveFile, CreatureWorldAnchor, PersistentVoxelProfileId,
    PersistentVoxelWorldBackend, PersistentVoxelWorldSnapshot, ProceduralTerrainMaterial,
    ProceduralTileCoord, ProceduralWorldConfig, StableVoxelObjectRef, StableVoxelRefKind,
    VoxelChunkCoord, VoxelTileCoord, FVR02_PERSISTENT_VOXEL_WORLD_SCHEMA,
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
        ClearColorConfig, Color, Commands, Component, Cuboid, DirectionalLight, GlobalTransform,
        Handle, KeyCode, Mesh, Mesh3d, MeshMaterial3d, MessageWriter, MouseButton, Name, Node,
        OrthographicProjection, PositionType, Projection, Quat, Res, ResMut, Resource,
        StandardMaterial, Text, TextColor, TextFont, Transform, Update, Val, Vec3, Visibility,
        Window, With,
    },
    render::{
        render_resource::PrimitiveTopology,
        view::screenshot::{save_to_disk, Screenshot},
    },
    window::PrimaryWindow,
};

use crate::{
    GameAppShellError, ProductionFrontendProfileId, ProductionVoxelLaunchSummary,
    PRODUCTION_VOXEL_RENDERER_PROFILE,
};

pub const FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA: &str = "alife.fvr03.production_voxel_renderer.v1";
pub const FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA_VERSION: u16 = 1;
pub const FVR03_RENDERER_BACKEND_ID: &str = "bevy_voxel_world+fvr03_chunk_mesh";
pub const FVR03_PERFORMANCE_ARTIFACT_DIR: &str = "target/artifacts/fvr03";

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
            ProductionFrontendProfileId::MinSpecComfort1080p => 4,
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
    pub selection_ref_count: usize,
    pub dirty_chunk_count: usize,
    pub estimated_resident_bytes: usize,
    pub draw_radius_chunks: u16,
    pub target_fps: u16,
    pub performance_artifact_path: Option<PathBuf>,
    pub no_renderer_authority_over_world_truth: bool,
    visible_tiles: BTreeSet<VoxelTileCoord>,
    visible_chunks: BTreeSet<VoxelChunkCoord>,
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
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct Fvr03ProductionVoxelSelectionResource {
    pub hovered: Option<StableVoxelObjectRef>,
    pub selected: Option<StableVoxelObjectRef>,
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
pub struct Fvr03ProductionVoxelSelectionMarker;

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

pub fn spawn_fvr03_production_voxel_scene(
    app: &mut App,
    summary: &ProductionVoxelLaunchSummary,
) -> Result<(), GameAppShellError> {
    let settings = Fvr03ProductionVoxelRendererSettings::for_profile(summary.profile_id);
    let snapshot = load_fvr03_snapshot(summary)?;
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
        meshes.add(Cuboid::new(0.72, 1.15, 0.72))
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
        spawn_fvr03_chunk_boundary(
            app,
            &materials,
            boundary_mesh.clone(),
            chunk.coord,
            snapshot.profile_budget.chunk_tile_size,
        );
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
    spawn_fvr03_creatures(app, &snapshot, &materials, creature_mesh);
    spawn_fvr03_camera(app, &settings);
    spawn_fvr03_lighting(app, &settings);

    let selected = visible_tiles
        .iter()
        .copied()
        .find_map(|tile| snapshot.lookup_tile(tile))
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
        backend_id: FVR03_RENDERER_BACKEND_ID,
        uses_bevy_voxel_world_backend: cfg!(feature = "voxel-backend"),
        uses_internal_chunk_mesh_for_fvr02_contract: true,
        visible_chunk_count: snapshot.visible_chunks.len(),
        resident_chunk_count: snapshot.visible_chunks.len(),
        tile_mesh_count,
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
    };

    if summary.record_performance {
        scene.performance_artifact_path = Some(write_fvr03_performance_artifact(&scene, None)?);
    }

    app.insert_resource(scene);
    app.insert_resource(Fvr03ProductionVoxelSelectionResource {
        hovered: selected,
        selected,
    });
    app.add_systems(
        Update,
        (handle_fvr03_mouse_selection, handle_fvr03_camera_mode_input),
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
    Ok(())
}

fn load_fvr03_snapshot(
    summary: &ProductionVoxelLaunchSummary,
) -> Result<PersistentVoxelWorldSnapshot, GameAppShellError> {
    let save = PortableSaveFile::from_json_file(&summary.save_path)?;
    save.validate_with_asset_root(&summary.asset_root)?;
    let production_save =
        save.with_migrated_voxel_backend(persistent_profile_id_for_fvr03(summary.profile_id))?;
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
                "FVR03 expected FVR02 snapshot schema, got {}",
                snapshot.schema
            ),
        });
    }
    Ok(snapshot)
}

fn persistent_profile_id_for_fvr03(
    profile_id: ProductionFrontendProfileId,
) -> PersistentVoxelProfileId {
    match profile_id {
        ProductionFrontendProfileId::MinimumSettings30x30 => {
            PersistentVoxelProfileId::MinimumSettings30x30
        }
        ProductionFrontendProfileId::MinSpecComfort1080p => {
            PersistentVoxelProfileId::MinSpecComfort1080p
        }
        ProductionFrontendProfileId::Balanced1080p => PersistentVoxelProfileId::Balanced1080p,
        ProductionFrontendProfileId::HighSpecScaleUp => PersistentVoxelProfileId::HighSpecScaleUp,
        ProductionFrontendProfileId::ResearchScale => PersistentVoxelProfileId::ResearchScale,
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

fn spawn_fvr03_creatures(
    app: &mut App,
    snapshot: &PersistentVoxelWorldSnapshot,
    materials: &BTreeMap<Fvr03ProductionVoxelMaterialKind, Handle<StandardMaterial>>,
    mesh: Handle<Mesh>,
) {
    let material = materials
        .get(&Fvr03ProductionVoxelMaterialKind::Creature)
        .expect("FVR03 creature material exists")
        .clone();
    for creature in &snapshot.creatures {
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life FVR03 creature stable {}",
                creature.stable_id.raw()
            )),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(
                creature.tile.x as f32 + 0.5,
                1.05,
                creature.tile.z as f32 + 0.5,
            ),
            Fvr03ProductionVoxelCreatureMarker {
                stable_id: creature.stable_id,
                tile: creature.tile,
            },
        ));
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
            "A-Life Voxel Frontend\nprofile: {} | population: {}\nrenderer: {} | backend: {}\ntarget: {} FPS | chunks radius: {} | stride: {}\nbackend: {} | fallback: {}\nsave: {}",
            summary.profile_id.label(),
            summary.effective_population,
            summary.renderer_profile,
            FVR03_RENDERER_BACKEND_ID,
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
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
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
    let selected = StableVoxelObjectRef {
        kind: StableVoxelRefKind::Tile,
        stable_id: None,
        chunk: VoxelChunkCoord::for_tile(16, tile),
        tile: Some(tile),
    };
    selection.hovered = Some(selected);
    selection.selected = Some(selected);
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

fn request_fvr03_recorded_screenshot(
    mut commands: Commands,
    mut capture: ResMut<Fvr03ProductionVoxelScreenshotResource>,
    scene: Res<Fvr03ProductionVoxelSceneResource>,
    mut exits: MessageWriter<AppExit>,
) {
    capture.frame = capture.frame.saturating_add(1);
    if capture.requested
        && capture.measurement_start_frame > 0
        && capture.measurement_started_at.is_none()
        && capture.frame >= capture.measurement_start_frame
    {
        capture.measurement_started_at = Some(Instant::now());
    }
    if capture.requested
        && !capture.measurement_written
        && capture.measurement_start_frame > 0
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
    if capture.requested || capture.frame < capture.capture_after_frame {
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
    capture.measurement_start_frame = capture.frame.saturating_add(10);
    capture.measurement_started_at = None;
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
        "{{\n  \"schema\": \"{}\",\n  \"profile\": \"{}\",\n  \"backend\": \"{}\",\n  \"target_fps\": {},\n  \"visible_chunks\": {},\n  \"resident_chunks\": {},\n  \"tile_mesh_count\": {},\n  \"estimated_resident_bytes\": {},\n  \"measured_fps\": {},\n  \"measured_frame_count\": {},\n  \"measured_seconds\": {},\n  \"performance_claim_status\": \"{}\"\n}}\n",
        scene.schema,
        scene.profile_id.label(),
        scene.backend_id,
        scene.target_fps,
        scene.visible_chunk_count,
        scene.resident_chunk_count,
        scene.tile_mesh_count,
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
