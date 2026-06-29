//! Feature-gated Bevy playground shell split during R13 remediation.

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use alife_bevy_adapter::{
    core_vec3_to_bevy, AffordanceTags, AlifeBevyAdapterPlugin, BevyEntityMap, CreatureBody,
    SensoryEmitter,
};
use alife_core::{ActionKind, AffordanceBits, Vec3f, WorldEntityId};
use alife_world::{
    activate_procedural_chunks_around_creatures, generate_procedural_world_content,
    sample_procedural_terrain_tile, CreatureWorldAnchor, ProceduralChunkActivationReport,
    ProceduralChunkCoord, ProceduralTerrainSample, ProceduralTileCoord, ProceduralWorldConfig,
    ProceduralWorldContentCandidate, ProceduralWorldContentKind, ProceduralWorldContentReport,
    TerrainZoneKind, WorldObjectKind,
};
use bevy::{
    app::AppExit,
    asset::{AssetPlugin, Assets, Handle, RenderAssetUsages},
    image::{CompressedImageFormats, Image, ImageSampler, ImageType},
    prelude::{
        default, App, BackgroundColor, ButtonInput, Camera, Camera2d, ClearColor, Color, Commands,
        Component, DefaultPlugins, Entity, GlobalTransform, ImageNode, KeyCode, MessageWriter,
        MinimalPlugins, MouseButton, Name, Node, NonSendMut, PluginGroup, PositionType, Quat, Res,
        ResMut, Resource, Sprite, Text, Text2d, TextColor, TextFont, Time, Transform, Update, Val,
        Vec2, Vec3, Visibility, With, Without,
    },
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::{ExitCondition, PresentMode, PrimaryWindow, Window, WindowPlugin, WindowTheme},
};

use crate::{
    ca18_cycle_selected_creature, ca18_graphical_population_summary, ca18_social_proximity_cues,
    ca19_graphical_ecology_summary, ca20_graphical_lifecycle_summary, ca37_world_art_style_summary,
    load_visible_world_from_p34_save, run_advanced_gameplay_ux_smoke, run_creature_inspector_smoke,
    run_creature_visual_smoke, run_headless_app_shell_smoke, run_live_brain_loop_smoke,
    AdvancedGameplayUxSummary, AppShellLaunchConfig, AppStartupSummary,
    Ca18GraphicalPopulationSummary, Ca19GraphicalEcologySummary, Ca19TerrainZoneVisual,
    Ca20GraphicalLifecycleSummary, Ca23GraphicalSchoolSummary, Ca23TeacherCueMarker,
    Ca37WorldArtStyleSummary, Ca38CreatureAnimationSummary, CameraNavigationState,
    CreatureAnimationState, CreatureExpressionState, CreatureInspectorSnapshot,
    CreatureVisualSnapshot, EntitySelectionSnapshot, GameAppShellError, GameAppState,
    GraphicalGpuRuntimeController, GraphicalGpuRuntimeMode, GraphicalGpuRuntimeTelemetry,
    GraphicalPlaygroundLaunchConfig, GraphicalPlaygroundLaunchSummary, GraphicalPlaygroundMode,
    GraphicalPlaygroundViewMode, LiveBrainLoop, LiveBrainTickSummary, RuntimeControlCommand,
    RuntimeControlPanel, RuntimePlaybackState, VisibleMaterialKind, VisiblePlaceholderShape,
    VisibleWorldObjectPresentation, VisibleWorldPresentation, S02_MAX_SMOKE_TICKS,
};

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct BevyAppShellSummary {
    pub seed: u64,
    pub current_state: GameAppState,
    pub graphics_required_for_default_path: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct VisibleWorldObject {
    pub stable_id: WorldEntityId,
    pub kind: WorldObjectKind,
    pub shape: VisiblePlaceholderShape,
    pub material: VisibleMaterialKind,
    pub rgba: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct VisibleWorldDebugLabel(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct GraphicalAlphaArtBackedSprite {
    pub role: &'static str,
    pub stable_id: Option<WorldEntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct GraphicalAlphaArtFallbackSprite {
    pub role: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Resource)]
pub struct GraphicalAlphaArtHandles {
    pub creature_idle: Handle<Image>,
    pub creature_hurt: Handle<Image>,
    pub creature_moving: Handle<Image>,
    pub creature_eat: Handle<Image>,
    pub creature_sleep: Handle<Image>,
    pub creature_signal: Handle<Image>,
    pub selection_ring: Handle<Image>,
    pub selection_pulse: Handle<Image>,
    pub food: Handle<Image>,
    pub food_bloom: Handle<Image>,
    pub hazard: Handle<Image>,
    pub hazard_glow: Handle<Image>,
    pub ambient_canopy_shadow: Handle<Image>,
    pub ambient_light_pool: Handle<Image>,
    pub entity_shadow: Handle<Image>,
    pub rock_obstacle: Handle<Image>,
    pub terrain_safe_grass: Handle<Image>,
    pub terrain_soil_path: Handle<Image>,
    pub terrain_resource_grove: Handle<Image>,
    pub terrain_hazard_pressure: Handle<Image>,
    pub terrain_stone_rough: Handle<Image>,
    pub terrain_edge_blend: Handle<Image>,
    pub world_backdrop: Handle<Image>,
    pub prop_grass_tuft: Handle<Image>,
    pub prop_pebble_cluster: Handle<Image>,
    pub prop_warning_shard: Handle<Image>,
    pub prop_leaf_patch: Handle<Image>,
    pub prop_mushroom_cluster: Handle<Image>,
    pub ui_panel_frame: Handle<Image>,
    pub ui_inspector_frame: Handle<Image>,
    pub ui_status_chip: Handle<Image>,
    pub ui_meter_bar: Handle<Image>,
    pub ui_control_keycap: Handle<Image>,
}

impl GraphicalAlphaArtHandles {
    pub fn from_embedded_assets(images: &mut Assets<Image>) -> Result<Self, GameAppShellError> {
        Ok(Self {
            creature_idle: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/creature_idle.png"),
                "creature_idle.png",
            )?,
            creature_hurt: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/creature_hurt.png"),
                "creature_hurt.png",
            )?,
            creature_moving: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/creature_moving.png"),
                "creature_moving.png",
            )?,
            creature_eat: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/creature_eat.png"),
                "creature_eat.png",
            )?,
            creature_sleep: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/creature_sleep.png"),
                "creature_sleep.png",
            )?,
            creature_signal: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/creature_signal.png"),
                "creature_signal.png",
            )?,
            selection_ring: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/selection_ring.png"),
                "selection_ring.png",
            )?,
            selection_pulse: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/selection_pulse.png"),
                "selection_pulse.png",
            )?,
            food: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/food_sprout.png"),
                "food_sprout.png",
            )?,
            food_bloom: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/food_bloom.png"),
                "food_bloom.png",
            )?,
            hazard: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/hazard_crystal.png"),
                "hazard_crystal.png",
            )?,
            hazard_glow: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/hazard_glow.png"),
                "hazard_glow.png",
            )?,
            ambient_canopy_shadow: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/ambient_canopy_shadow.png"),
                "ambient_canopy_shadow.png",
            )?,
            ambient_light_pool: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/ambient_light_pool.png"),
                "ambient_light_pool.png",
            )?,
            entity_shadow: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/entity_shadow.png"),
                "entity_shadow.png",
            )?,
            rock_obstacle: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/rock_cluster.png"),
                "rock_cluster.png",
            )?,
            terrain_safe_grass: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/terrain_safe_grass.png"),
                "terrain_safe_grass.png",
            )?,
            terrain_soil_path: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/terrain_soil_path.png"),
                "terrain_soil_path.png",
            )?,
            terrain_resource_grove: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/terrain_resource_grove.png"),
                "terrain_resource_grove.png",
            )?,
            terrain_hazard_pressure: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/terrain_hazard_pressure.png"),
                "terrain_hazard_pressure.png",
            )?,
            terrain_stone_rough: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/terrain_stone_rough.png"),
                "terrain_stone_rough.png",
            )?,
            terrain_edge_blend: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/terrain_edge_blend.png"),
                "terrain_edge_blend.png",
            )?,
            world_backdrop: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/world_backdrop_gpu_alpha.png"),
                "world_backdrop_gpu_alpha.png",
            )?,
            prop_grass_tuft: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/prop_grass_tuft.png"),
                "prop_grass_tuft.png",
            )?,
            prop_pebble_cluster: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/prop_pebble_cluster.png"),
                "prop_pebble_cluster.png",
            )?,
            prop_warning_shard: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/prop_warning_shard.png"),
                "prop_warning_shard.png",
            )?,
            prop_leaf_patch: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/prop_leaf_patch.png"),
                "prop_leaf_patch.png",
            )?,
            prop_mushroom_cluster: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/prop_mushroom_cluster.png"),
                "prop_mushroom_cluster.png",
            )?,
            ui_panel_frame: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/ui_panel_frame.png"),
                "ui_panel_frame.png",
            )?,
            ui_inspector_frame: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/ui_inspector_frame.png"),
                "ui_inspector_frame.png",
            )?,
            ui_status_chip: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/ui_status_chip.png"),
                "ui_status_chip.png",
            )?,
            ui_meter_bar: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/ui_meter_bar.png"),
                "ui_meter_bar.png",
            )?,
            ui_control_keycap: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/ui_control_keycap.png"),
                "ui_control_keycap.png",
            )?,
        })
    }

    pub fn unloaded_for_validation() -> Self {
        Self {
            creature_idle: Handle::default(),
            creature_hurt: Handle::default(),
            creature_moving: Handle::default(),
            creature_eat: Handle::default(),
            creature_sleep: Handle::default(),
            creature_signal: Handle::default(),
            selection_ring: Handle::default(),
            selection_pulse: Handle::default(),
            food: Handle::default(),
            food_bloom: Handle::default(),
            hazard: Handle::default(),
            hazard_glow: Handle::default(),
            ambient_canopy_shadow: Handle::default(),
            ambient_light_pool: Handle::default(),
            entity_shadow: Handle::default(),
            rock_obstacle: Handle::default(),
            terrain_safe_grass: Handle::default(),
            terrain_soil_path: Handle::default(),
            terrain_resource_grove: Handle::default(),
            terrain_hazard_pressure: Handle::default(),
            terrain_stone_rough: Handle::default(),
            terrain_edge_blend: Handle::default(),
            world_backdrop: Handle::default(),
            prop_grass_tuft: Handle::default(),
            prop_pebble_cluster: Handle::default(),
            prop_warning_shard: Handle::default(),
            prop_leaf_patch: Handle::default(),
            prop_mushroom_cluster: Handle::default(),
            ui_panel_frame: Handle::default(),
            ui_inspector_frame: Handle::default(),
            ui_status_chip: Handle::default(),
            ui_meter_bar: Handle::default(),
            ui_control_keycap: Handle::default(),
        }
    }
}

fn ca44a_register_embedded_alpha_art(
    images: &mut Assets<Image>,
    bytes: &[u8],
    _name: &'static str,
) -> Result<Handle<Image>, GameAppShellError> {
    let image = Image::from_buffer(
        bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::linear(),
        RenderAssetUsages::default(),
    )
    .map_err(|_| GameAppShellError::VisibleWorldMismatch {
        message: "failed to decode embedded alpha art PNG",
    })?;
    Ok(images.add(image))
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct VisibleGroundPlane {
    pub shape: VisiblePlaceholderShape,
    pub material: VisibleMaterialKind,
    pub rgba: [f32; 4],
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct VisibleWorldSceneResource {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub save_id: String,
    pub visible_signature: Vec<String>,
    pub headless_signature: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalVisibleWorldPresentationResource {
    pub presentation: VisibleWorldPresentation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibleWorldSpawnSummary {
    pub ground_spawned: bool,
    pub object_count: usize,
    pub stable_map_count: usize,
    pub visible_signature: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct LiveBrainLoopResource {
    pub last_summary: LiveBrainTickSummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CreatureVisualStateResource {
    pub snapshot: CreatureVisualSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct CameraNavigationResource {
    pub state: CameraNavigationState,
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct SelectionResource {
    pub stable_id: WorldEntityId,
    pub local_entity: Option<Entity>,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct CreatureInspectorResource {
    pub snapshot: CreatureInspectorSnapshot,
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct SelectedVisibleEntity {
    pub selection: EntitySelectionSnapshot,
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct VisibleCreatureState {
    pub animation: CreatureAnimationState,
    pub expression: CreatureExpressionState,
    pub base_rgba: [f32; 4],
    pub accent_rgba: [f32; 4],
    pub intent_rgba: [f32; 4],
    pub debug_summary: String,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalPlaygroundSceneResource {
    pub summary: GraphicalPlaygroundLaunchSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
pub struct GraphicalViewModeResource {
    pub mode: GraphicalPlaygroundViewMode,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalPopulationResource {
    pub summary: Ca18GraphicalPopulationSummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalEcologyResource {
    pub summary: Ca19GraphicalEcologySummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalLifecycleResource {
    pub summary: Ca20GraphicalLifecycleSummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalSchoolResource {
    pub summary: Ca23GraphicalSchoolSummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalWorldArtStyleResource {
    pub summary: Ca37WorldArtStyleSummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalCreatureAnimationResource {
    pub summary: Ca38CreatureAnimationSummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalPlaygroundRunSummary {
    pub launch: GraphicalPlaygroundLaunchSummary,
    pub runtime: RuntimeControlPanel,
    pub gpu: GraphicalGpuRuntimeTelemetry,
}

impl GraphicalPlaygroundRunSummary {
    pub fn signature_line(&self) -> String {
        format!(
            "{}|{}|gpu={}:{}",
            self.launch.signature_line(),
            self.runtime.signature_line(),
            self.gpu.requested_mode.label(),
            self.gpu.product_runtime_claim
        )
    }
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalRuntimeControlsResource {
    pub panel: RuntimeControlPanel,
    pub smoke_target_ticks: Option<u32>,
    pub smoke_ticks_done: u32,
}

struct GraphicalRuntimeLoopResource {
    launch: AppShellLaunchConfig,
    live: LiveBrainLoop,
    gpu: GraphicalGpuRuntimeController,
}

#[derive(Clone, Resource)]
struct GraphicalRuntimeCaptureSink(
    Arc<Mutex<Option<(RuntimeControlPanel, GraphicalGpuRuntimeTelemetry)>>>,
);

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalGpuTelemetryResource {
    pub telemetry: GraphicalGpuRuntimeTelemetry,
}

impl GraphicalRuntimeControlsResource {
    pub fn new(launch: &GraphicalPlaygroundLaunchConfig) -> Result<Self, GameAppShellError> {
        let live = LiveBrainLoop::from_p34_launch(&launch.app_launch)?;
        let panel = RuntimeControlPanel::from_live_loop(&live);
        panel.validate()?;
        Ok(Self {
            panel,
            smoke_target_ticks: launch
                .mode
                .smoke_seconds()
                .map(|seconds| seconds.min(S02_MAX_SMOKE_TICKS).max(1)),
            smoke_ticks_done: 0,
        })
    }
}

fn graphical_runtime_resources(
    launch: &GraphicalPlaygroundLaunchConfig,
) -> Result<
    (
        GraphicalRuntimeControlsResource,
        GraphicalRuntimeLoopResource,
        GraphicalGpuTelemetryResource,
    ),
    GameAppShellError,
> {
    let live = LiveBrainLoop::from_p34_launch(&launch.app_launch)?;
    let panel = RuntimeControlPanel::from_live_loop(&live);
    panel.validate()?;
    let controls = GraphicalRuntimeControlsResource {
        panel,
        smoke_target_ticks: launch
            .mode
            .smoke_seconds()
            .map(|seconds| seconds.min(S02_MAX_SMOKE_TICKS).max(1)),
        smoke_ticks_done: 0,
    };
    let gpu = GraphicalGpuRuntimeController::new(launch.gpu_mode);
    let telemetry = GraphicalGpuTelemetryResource {
        telemetry: gpu.telemetry().clone(),
    };
    Ok((
        controls,
        GraphicalRuntimeLoopResource {
            launch: launch.app_launch.clone(),
            live,
            gpu,
        },
        telemetry,
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalPlaygroundMarker {
    pub stable_id: WorldEntityId,
    pub kind: WorldObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct RuntimeStatusOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalMainCamera;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalSelectionRing;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct InspectorStatusOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct ReadabilityLegendOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct FeedbackCueOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalPopulationOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalEcologyOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalLifecycleOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalSchoolOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTopologyOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalMemoryJournalOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalNeuralActivityProfilerOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalOnboardingTutorialOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTeacherCueMarker {
    pub stable_id: WorldEntityId,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTerrainZoneMarker {
    pub zone_id: alife_world::EcologyZoneId,
    pub kind: TerrainZoneKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalWorldArtProp {
    pub prop_id: &'static str,
    pub material_id: &'static str,
    pub anchored_stable_id: Option<WorldEntityId>,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalWorldArtTerrainTile {
    pub tile_x: i32,
    pub tile_z: i32,
    pub material_id: &'static str,
    pub tile_size_pixels: f32,
    pub organic_rotation_degrees: f32,
    pub opacity: f32,
    pub viewport_slice: bool,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalProceduralTerrainChunkTile {
    pub anchor_stable_id: Option<WorldEntityId>,
    pub world_chunk_x: i32,
    pub world_chunk_z: i32,
    pub chunk_center_tile_x: i32,
    pub chunk_center_tile_z: i32,
    pub virtual_map_width_tiles: usize,
    pub virtual_map_height_tiles: usize,
    pub creature_authoritative_chunk: bool,
    pub rendering_required_for_generation: bool,
    pub materialized_only_near_active_views: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalProceduralWorldContentMarker {
    pub stable_id: WorldEntityId,
    pub kind: ProceduralWorldContentKind,
    pub anchor_stable_id: WorldEntityId,
    pub world_chunk_x: i32,
    pub world_chunk_z: i32,
    pub tile_x: i32,
    pub tile_z: i32,
    pub generated_without_rendering: bool,
    pub rendering_required: bool,
    pub creature_context_candidate: bool,
    pub can_emit_actions: bool,
    pub can_rewrite_weights: bool,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalProceduralTerrainFieldResource {
    pub seed: u64,
    pub virtual_map_width_tiles: usize,
    pub virtual_map_height_tiles: usize,
    pub chunk_radius_x: i32,
    pub chunk_radius_z: i32,
    pub active_world_chunks: BTreeSet<(i32, i32)>,
    pub creature_anchor_count: usize,
    pub generated_without_rendering: bool,
    pub materialized_tiles: BTreeSet<(i32, i32)>,
    pub materialized_content_stable_ids: BTreeSet<u64>,
    pub materialized_chunk_count: usize,
    pub active_content_count: usize,
    pub procedural_content_generated_without_rendering: bool,
    pub procedural_content_rendering_required: bool,
    pub materialized_only_near_active_views: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalProductionArtLayer {
    pub role: &'static str,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct GraphicalRuntimeProceduralBiomeMap {
    pub width_tiles: i32,
    pub height_tiles: i32,
    pub texture_width_px: u32,
    pub texture_height_px: u32,
    pub pixels_per_tile: u32,
    pub virtual_map_width_tiles: usize,
    pub virtual_map_height_tiles: usize,
    pub path_pixels: u32,
    pub resource_detail_pixels: u32,
    pub hazard_detail_pixels: u32,
    pub stone_detail_pixels: u32,
    pub dark_gap_pixels: u32,
    pub generated_from_procedural_sampler: bool,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalProductionHudSkinLayer {
    pub role: &'static str,
    pub panel_id: &'static str,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalCreatureAnimationPose {
    pub stable_id: WorldEntityId,
    pub pose_id: &'static str,
    pub action_label: &'static str,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalSocialProximityCue {
    pub from_stable_id: WorldEntityId,
    pub to_stable_id: WorldEntityId,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct SaveLoadMenuOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct AdvancedGameplayOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct BoundaryFooterOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalObjectBadge {
    pub stable_id: WorldEntityId,
    pub kind: WorldObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalObjectGlyph {
    pub stable_id: WorldEntityId,
    pub kind: WorldObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalIntentLine;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalActionBadge;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ca08SensoryCueKind {
    Reward,
    Pain,
    Sleep,
    Learning,
}

impl Ca08SensoryCueKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reward => "reward",
            Self::Pain => "pain",
            Self::Sleep => "sleep",
            Self::Learning => "learning",
        }
    }

    pub const fn audio_stub(self) -> &'static str {
        match self {
            Self::Reward => "soft-ping",
            Self::Pain => "warning-pulse",
            Self::Sleep => "rest-chime",
            Self::Learning => "learn-spark",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ca08SensoryCueRow {
    pub kind: Ca08SensoryCueKind,
    pub target: Option<WorldEntityId>,
    pub active: bool,
}

impl Ca08SensoryCueRow {
    pub fn panel_line(self) -> String {
        let target = self
            .target
            .map_or_else(|| "guide".to_string(), |id| format!("stable:{}", id.raw()));
        let state = if self.active { "on" } else { "armed" };
        format!(
            "{}: {} | audio={} | {}",
            self.kind.label(),
            state,
            self.kind.audio_stub(),
            target
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalSensoryCuePulse {
    pub kind: Ca08SensoryCueKind,
    pub target_stable_id: Option<WorldEntityId>,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalFeedbackCueResource {
    pub summary: crate::FeedbackPolishSummary,
}

#[derive(Debug, Clone, Resource)]
pub struct GraphicalSaveLoadMenuResource {
    pub session: crate::GraphicalSaveLoadMenuSession,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalAdvancedGameplayResource {
    pub summary: AdvancedGameplayUxSummary,
}

const GRAPHICAL_WORLD_SCALE: f32 = 36.0;
const CA37_TERRAIN_TILE_PIXEL_SIZE: f32 = GRAPHICAL_WORLD_SCALE;
const CA37_TERRAIN_TILE_JITTER_PIXELS: f32 = 2.0;
pub(crate) const CA37_EXPLORATION_CAMERA_ZOOM: f32 = 0.34;
const CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES: i32 = 128;
const CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES: i32 = 72;
const CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE: u32 = 12;
const CA44A_PLAYER_WORLD_BACKDROP_WIDTH: f32 = 3_840.0;
const CA44A_PLAYER_WORLD_BACKDROP_HEIGHT: f32 = 2_160.0;

#[derive(Debug, Resource)]
struct GraphicalPlaygroundSmokeTimer {
    started: Instant,
    duration: Duration,
}

pub fn build_minimal_bevy_app_shell(summary: AppStartupSummary) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AlifeBevyAdapterPlugin)
        .insert_resource(BevyAppShellSummary {
            seed: summary.seed,
            current_state: GameAppState::Boot,
            graphics_required_for_default_path: summary.graphics_required_for_default_path,
        });
    app
}

pub fn build_visible_world_app_shell(
    launch: &AppShellLaunchConfig,
) -> Result<(App, VisibleWorldSpawnSummary), GameAppShellError> {
    let startup = crate::run_headless_app_shell_smoke(launch)?;
    let presentation = load_visible_world_from_p34_save(launch)?;
    let mut app = build_minimal_bevy_app_shell(startup);
    let summary = spawn_visible_world(&mut app, &presentation)?;
    Ok((app, summary))
}

pub fn build_live_brain_world_app_shell(
    launch: &AppShellLaunchConfig,
) -> Result<(App, VisibleWorldSpawnSummary, LiveBrainTickSummary), GameAppShellError> {
    let (mut app, summary) = build_visible_world_app_shell(launch)?;
    let tick_summary = run_live_brain_loop_smoke(launch)?;
    app.insert_resource(LiveBrainLoopResource {
        last_summary: tick_summary.clone(),
    });
    Ok((app, summary, tick_summary))
}

pub fn build_creature_visual_world_app_shell(
    launch: &AppShellLaunchConfig,
) -> Result<(App, VisibleWorldSpawnSummary, CreatureVisualSnapshot), GameAppShellError> {
    let (mut app, summary) = build_visible_world_app_shell(launch)?;
    let visual = run_creature_visual_smoke(launch)?;
    if let Some(entity) = app
        .world()
        .resource::<BevyEntityMap>()
        .bevy_entity(visual.stable_id)
    {
        app.world_mut()
            .entity_mut(entity)
            .insert(VisibleCreatureState {
                animation: visual.animation,
                expression: visual.expression,
                base_rgba: visual.base_rgba,
                accent_rgba: visual.accent_rgba,
                intent_rgba: visual.intent_rgba,
                debug_summary: visual.debug_summary.clone(),
            });
    }
    app.insert_resource(CreatureVisualStateResource {
        snapshot: visual.clone(),
    });
    Ok((app, summary, visual))
}

pub fn build_creature_inspector_world_app_shell(
    launch: &AppShellLaunchConfig,
) -> Result<(App, VisibleWorldSpawnSummary, CreatureInspectorSnapshot), GameAppShellError> {
    let (mut app, summary, _visual) = build_creature_visual_world_app_shell(launch)?;
    let inspector = run_creature_inspector_smoke(launch)?;
    let local_entity = app
        .world()
        .resource::<BevyEntityMap>()
        .bevy_entity(inspector.selection.stable_id);
    if let Some(entity) = local_entity {
        app.world_mut()
            .entity_mut(entity)
            .insert(SelectedVisibleEntity {
                selection: inspector.selection.clone(),
            });
    }
    app.insert_resource(CameraNavigationResource {
        state: inspector.camera,
    });
    app.insert_resource(SelectionResource {
        stable_id: inspector.selection.stable_id,
        local_entity,
    });
    app.insert_resource(CreatureInspectorResource {
        snapshot: inspector.clone(),
    });
    Ok((app, summary, inspector))
}

pub fn build_graphical_playground_app_shell(
    launch: &GraphicalPlaygroundLaunchConfig,
) -> Result<(App, GraphicalPlaygroundLaunchSummary), GameAppShellError> {
    let summary = crate::validate_graphical_playground_launch(launch)?;
    let presentation = load_visible_world_from_p34_save(&launch.app_launch)?;
    crate::compare_visible_world_to_headless(&presentation)?;
    let population_summary = ca18_graphical_population_summary(&presentation).ok();
    let ecology_summary = ca19_graphical_ecology_summary(&launch.app_launch).ok();
    let world_art_summary = ca37_world_art_style_summary(&launch.app_launch).ok();
    let animation_summary = crate::ca38_creature_animation_summary().ok();
    let lifecycle_summary = ca20_graphical_lifecycle_summary().ok();
    let school_summary = crate::run_graphical_school_mode_smoke()?;

    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "crates/alife_game_app/assets".to_string(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: launch.window_title.clone(),
                    name: Some("alife.graphical_playground".to_string()),
                    resolution: (1280, 720).into(),
                    present_mode: PresentMode::AutoVsync,
                    window_theme: Some(WindowTheme::Dark),
                    ..default()
                }),
                exit_condition: ExitCondition::OnPrimaryClosed,
                ..default()
            }),
    )
    .add_plugins(AlifeBevyAdapterPlugin)
    .insert_resource(ClearColor(Color::srgb(0.39, 0.49, 0.25)))
    .insert_resource(GraphicalPlaygroundSceneResource {
        summary: summary.clone(),
    })
    .insert_resource(GraphicalViewModeResource {
        mode: summary.view_mode,
    });
    spawn_graphical_playground_scene(
        &mut app,
        &presentation,
        &summary,
        ecology_summary.as_ref(),
        world_art_summary.as_ref(),
        Some(&school_summary),
    )?;
    let inspector = run_creature_inspector_smoke(&launch.app_launch)?;
    let feedback = crate::run_feedback_polish_smoke(&launch.app_launch)?;
    let save_load = crate::GraphicalSaveLoadMenuSession::from_launch(&launch.app_launch)?;
    let advanced = run_advanced_gameplay_ux_smoke()?;
    let local_entity =
        inspector_local_entity(&mut app, &presentation, inspector.selection.stable_id)?;
    let (controls, live_loop, gpu_telemetry) = graphical_runtime_resources(launch)?;
    let camera_state = ca37_graphical_default_camera_state(&inspector, world_art_summary.as_ref())?;
    app.insert_resource(controls)
        .insert_resource(gpu_telemetry)
        .insert_non_send_resource(live_loop)
        .insert_resource(GraphicalVisibleWorldPresentationResource {
            presentation: presentation.clone(),
        })
        .insert_resource(GraphicalCreatureAnimationResource {
            summary: animation_summary
                .clone()
                .unwrap_or_else(|| crate::ca38_creature_animation_summary().unwrap()),
        })
        .insert_resource(CameraNavigationResource {
            state: camera_state,
        })
        .insert_resource(SelectionResource {
            stable_id: inspector.selection.stable_id,
            local_entity,
        })
        .insert_resource(CreatureInspectorResource {
            snapshot: inspector,
        })
        .insert_resource(GraphicalFeedbackCueResource { summary: feedback })
        .insert_resource(GraphicalSaveLoadMenuResource { session: save_load })
        .insert_resource(GraphicalAdvancedGameplayResource {
            summary: advanced.clone(),
        })
        .add_systems(
            Update,
            (
                handle_graphical_runtime_input,
                handle_graphical_camera_selection_input,
                handle_graphical_population_cycle_input,
                handle_graphical_mouse_selection,
                advance_graphical_runtime_loop,
                update_graphical_camera_transform,
                update_graphical_selection_ring,
                update_graphical_runtime_overlay,
                update_graphical_inspector_overlay,
                update_graphical_gpu_visual_cues,
                update_graphical_feedback_pulses,
                update_graphical_intent_feedback,
                update_graphical_procedural_terrain_field,
                update_graphical_feedback_overlay,
                update_graphical_population_overlay,
                update_graphical_ecology_overlay,
                update_graphical_lifecycle_overlay,
                update_graphical_topology_overlay,
                update_graphical_memory_journal_overlay,
                update_graphical_onboarding_tutorial_overlay,
            ),
        )
        .add_systems(
            Update,
            (
                update_graphical_neural_activity_profiler_overlay,
                update_graphical_boundary_footer_overlay,
                update_graphical_save_load_menu_overlay,
            ),
        )
        .add_systems(Update, update_graphical_advanced_gameplay_overlay)
        .add_systems(
            Update,
            (
                handle_graphical_school_toggle_input,
                update_graphical_school_overlay,
            ),
        );
    if let Some(summary) = population_summary {
        app.insert_resource(GraphicalPopulationResource { summary });
    }
    if let Some(summary) = ecology_summary {
        app.insert_resource(GraphicalEcologyResource { summary });
    }
    if let Some(summary) = world_art_summary {
        app.insert_resource(GraphicalWorldArtStyleResource { summary });
    }
    if let Some(summary) = animation_summary {
        app.insert_resource(GraphicalCreatureAnimationResource { summary });
    }
    if let Some(summary) = lifecycle_summary {
        app.insert_resource(GraphicalLifecycleResource { summary });
    }
    app.insert_resource(GraphicalSchoolResource {
        summary: school_summary,
    });

    if let GraphicalPlaygroundMode::Smoke { seconds } = launch.mode {
        app.insert_resource(GraphicalPlaygroundSmokeTimer {
            started: Instant::now(),
            duration: Duration::from_secs(u64::from(seconds)),
        })
        .add_systems(Update, close_after_graphical_smoke_timeout);
    }

    Ok((app, summary))
}

fn ca37_graphical_default_camera_state(
    inspector: &CreatureInspectorSnapshot,
    world_art: Option<&Ca37WorldArtStyleSummary>,
) -> Result<CameraNavigationState, GameAppShellError> {
    if world_art
        .map(|summary| summary.local_viewport_is_smaller_than_map)
        .unwrap_or(false)
    {
        let mut state = CameraNavigationState::top_down_default();
        state.zoom = CA37_EXPLORATION_CAMERA_ZOOM;
        state.validate()?;
        return Ok(state);
    }
    let state = inspector.camera;
    state.validate()?;
    Ok(state)
}

pub fn build_graphical_playground_preview_app_shell(
    launch: &GraphicalPlaygroundLaunchConfig,
) -> Result<(App, GraphicalPlaygroundLaunchSummary), GameAppShellError> {
    let startup = run_headless_app_shell_smoke(&launch.app_launch)?;
    let summary = crate::validate_graphical_playground_launch(launch)?;
    let presentation = load_visible_world_from_p34_save(&launch.app_launch)?;
    crate::compare_visible_world_to_headless(&presentation)?;
    let ecology_summary = ca19_graphical_ecology_summary(&launch.app_launch).ok();
    let world_art_summary = ca37_world_art_style_summary(&launch.app_launch).ok();
    let animation_summary = crate::ca38_creature_animation_summary().ok();
    let school_summary = crate::run_graphical_school_mode_smoke().ok();
    let mut app = build_minimal_bevy_app_shell(startup);
    app.insert_resource(GraphicalAlphaArtHandles::unloaded_for_validation());
    spawn_graphical_playground_scene(
        &mut app,
        &presentation,
        &summary,
        ecology_summary.as_ref(),
        world_art_summary.as_ref(),
        school_summary.as_ref(),
    )?;
    let inspector = run_creature_inspector_smoke(&launch.app_launch)?;
    let local_entity =
        inspector_local_entity(&mut app, &presentation, inspector.selection.stable_id)?;
    app.insert_resource(SelectionResource {
        stable_id: inspector.selection.stable_id,
        local_entity,
    })
    .insert_resource(CreatureInspectorResource {
        snapshot: inspector,
    });
    if let Some(summary) = world_art_summary {
        app.insert_resource(GraphicalWorldArtStyleResource { summary });
    }
    if let Some(summary) = animation_summary {
        app.insert_resource(GraphicalCreatureAnimationResource { summary });
    }
    Ok((app, summary))
}

pub fn build_ca03_intent_feedback_preview_app_shell(
    launch: &GraphicalPlaygroundLaunchConfig,
    panel: RuntimeControlPanel,
) -> Result<App, GameAppShellError> {
    let startup = run_headless_app_shell_smoke(&launch.app_launch)?;
    let summary = crate::validate_graphical_playground_launch(launch)?;
    let presentation = load_visible_world_from_p34_save(&launch.app_launch)?;
    crate::compare_visible_world_to_headless(&presentation)?;
    let mut app = build_minimal_bevy_app_shell(startup);
    app.insert_resource(GraphicalPlaygroundSceneResource {
        summary: summary.clone(),
    });
    app.insert_resource(GraphicalViewModeResource {
        mode: summary.view_mode,
    });
    spawn_graphical_playground_scene(&mut app, &presentation, &summary, None, None, None)?;
    app.insert_resource(GraphicalRuntimeControlsResource {
        panel,
        smoke_target_ticks: None,
        smoke_ticks_done: 0,
    })
    .insert_resource(SelectionResource {
        stable_id: WorldEntityId(1),
        local_entity: None,
    })
    .add_systems(Update, update_graphical_intent_feedback);
    Ok(app)
}

pub fn run_graphical_playground_window(
    launch: &GraphicalPlaygroundLaunchConfig,
) -> Result<GraphicalPlaygroundLaunchSummary, GameAppShellError> {
    Ok(run_graphical_playground_window_with_controls(launch)?.launch)
}

pub fn run_graphical_playground_window_with_controls(
    launch: &GraphicalPlaygroundLaunchConfig,
) -> Result<GraphicalPlaygroundRunSummary, GameAppShellError> {
    let (mut app, summary) = build_graphical_playground_app_shell(launch)?;
    let capture = Arc::new(Mutex::new(None));
    app.insert_resource(GraphicalRuntimeCaptureSink(capture.clone()))
        .add_systems(Update, capture_graphical_runtime_snapshot);
    app.run();
    let (runtime, gpu) = capture
        .lock()
        .map_err(|_| GameAppShellError::VisibleWorldMismatch {
            message: "graphical runtime capture sink was poisoned",
        })?
        .clone()
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "graphical runtime exited before telemetry could be captured",
        })?;
    if launch.require_gpu && gpu.fallback_reason.is_some() {
        return Err(GameAppShellError::InvalidGraphicalLaunch {
            message: "RequireGpu requested a real GPU path, but graphical runtime fell back to CPU",
        });
    }
    Ok(GraphicalPlaygroundRunSummary {
        launch: summary,
        runtime,
        gpu,
    })
}

pub fn spawn_visible_world(
    app: &mut App,
    presentation: &VisibleWorldPresentation,
) -> Result<VisibleWorldSpawnSummary, GameAppShellError> {
    crate::compare_visible_world_to_headless(presentation)?;
    let ground_material = presentation.ground_material;
    app.world_mut().spawn((
        Transform::from_xyz(0.0, 0.0, -0.05),
        VisibleGroundPlane {
            shape: presentation.ground_shape,
            material: ground_material,
            rgba: ground_material.rgba(),
        },
        VisibleWorldDebugLabel("ground:debug-plane".to_string()),
    ));

    for object in &presentation.objects {
        let material = object.material;
        let entity = app
            .world_mut()
            .spawn((
                Transform::from_translation(core_vec3_to_bevy(object.position)?),
                VisibleWorldObject {
                    stable_id: object.stable_id,
                    kind: object.kind,
                    shape: object.shape,
                    material,
                    rgba: material.rgba(),
                },
                VisibleWorldDebugLabel(object.debug_label.clone()),
            ))
            .id();
        {
            let mut entity_mut = app.world_mut().entity_mut(entity);
            match object.kind {
                WorldObjectKind::Agent => {
                    if let Some(organism_id) = object.organism_id {
                        entity_mut.insert(CreatureBody::new(organism_id, object.stable_id)?);
                    }
                }
                WorldObjectKind::Food => {
                    entity_mut.insert(AffordanceTags::food(object.nutrition));
                }
                WorldObjectKind::Hazard => {
                    entity_mut.insert(AffordanceTags::hazard(object.hazard_pain));
                }
                WorldObjectKind::Obstacle => {
                    entity_mut.insert(AffordanceTags {
                        bits: AffordanceBits::RESOURCE,
                        nutrition: 0.0,
                        hazard_pain: 0.0,
                        blocks_movement: true,
                    });
                }
                WorldObjectKind::Token => {
                    entity_mut.insert(SensoryEmitter {
                        audible_token: object.token_id,
                        ..SensoryEmitter::default()
                    });
                }
            }
        }
        app.world_mut()
            .resource_mut::<BevyEntityMap>()
            .bind(entity, object.stable_id)?;
    }

    app.insert_resource(VisibleWorldSceneResource {
        schema: presentation.schema,
        schema_version: presentation.schema_version,
        seed: presentation.seed,
        save_id: presentation.save_id.clone(),
        visible_signature: presentation.visible_signature.clone(),
        headless_signature: presentation.headless_signature.clone(),
    });
    app.update();

    let map_len = app.world().resource::<BevyEntityMap>().len();
    Ok(VisibleWorldSpawnSummary {
        ground_spawned: true,
        object_count: presentation.objects.len(),
        stable_map_count: map_len,
        visible_signature: presentation.visible_signature.clone(),
    })
}

fn spawn_graphical_playground_scene(
    app: &mut App,
    presentation: &VisibleWorldPresentation,
    summary: &GraphicalPlaygroundLaunchSummary,
    ecology: Option<&Ca19GraphicalEcologySummary>,
    world_art: Option<&Ca37WorldArtStyleSummary>,
    school: Option<&Ca23GraphicalSchoolSummary>,
) -> Result<(), GameAppShellError> {
    app.world_mut().spawn((Camera2d, GraphicalMainCamera));
    let alpha_art = if let Some(handles) = app.world().get_resource::<GraphicalAlphaArtHandles>() {
        Some(handles.clone())
    } else {
        if !app.world().contains_resource::<Assets<Image>>() {
            app.init_resource::<Assets<Image>>();
        }
        let handles = {
            let mut images = app.world_mut().resource_mut::<Assets<Image>>();
            GraphicalAlphaArtHandles::from_embedded_assets(&mut images)?
        };
        app.insert_resource(handles.clone());
        Some(handles)
    };
    let (ground_color, ground_size, ground_label) =
        if summary.view_mode == GraphicalPlaygroundViewMode::Player {
            (
                Color::srgb(0.45, 0.56, 0.25),
                Vec2::new(5_600.0, 5_600.0),
                "ground:production-player-backdrop",
            )
        } else {
            (
                rgba_to_color(presentation.ground_material.rgba()),
                Vec2::new(860.0, 460.0),
                "ground:p34-fixture",
            )
        };
    app.world_mut().spawn((
        Name::new("A-Life S01 ground plane"),
        Sprite {
            color: ground_color,
            custom_size: Some(ground_size),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -10.0),
        VisibleGroundPlane {
            shape: presentation.ground_shape,
            material: presentation.ground_material,
            rgba: presentation.ground_material.rgba(),
        },
        VisibleWorldDebugLabel(ground_label.to_string()),
    ));
    if let Some(ecology) = ecology {
        spawn_ca19_terrain_zone_visuals(app, ecology, summary.view_mode);
    }
    if let Some(world_art) = world_art {
        spawn_ca37_world_art_dressing(
            app,
            world_art,
            alpha_art.as_ref(),
            summary.view_mode,
            presentation,
        );
    }

    for object in &presentation.objects {
        spawn_graphical_object(app, object, summary.view_mode, alpha_art.as_ref())?;
    }
    if let Some(school) = school {
        spawn_ca23_school_teacher_markers(app, school, summary.view_mode);
    }
    spawn_graphical_intent_feedback(app, summary.view_mode);
    spawn_ca08_feedback_pulses(app, presentation, summary.view_mode, alpha_art.as_ref());
    spawn_ca18_social_proximity_cues(app, presentation, summary.view_mode);
    spawn_production_player_view_hud_skin(app, alpha_art.as_ref(), summary.view_mode);

    app.insert_resource(VisibleWorldSceneResource {
        schema: presentation.schema,
        schema_version: presentation.schema_version,
        seed: presentation.seed,
        save_id: presentation.save_id.clone(),
        visible_signature: presentation.visible_signature.clone(),
        headless_signature: presentation.headless_signature.clone(),
    });

    app.world_mut().spawn((
        Name::new("A-Life S02 runtime controls overlay"),
        Text::new(format!(
            "A-Life GPU Alpha\nlaunching | GPU {}\nSpace/N/R/Esc",
            summary.requested_gpu_mode.label(),
        )),
        TextFont {
            font_size: 8.0,
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.95, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            max_width: Val::Px(116.0),
            padding: bevy::ui::UiRect::all(Val::Px(4.0)),
            ..default()
        },
        player_hud_text_background(summary.view_mode, Color::srgba(0.02, 0.03, 0.025, 0.58)),
        RuntimeStatusOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA18 graphical population overlay"),
        Text::new("Population: loading stable-ID creatures..."),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.96, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(222.0),
            left: Val::Px(12.0),
            max_width: Val::Px(390.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.035, 0.04, 0.78)),
        Visibility::Hidden,
        GraphicalPopulationOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA19 graphical ecology overlay"),
        Text::new("Ecology: loading terrain zones..."),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.90, 1.0, 0.86)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(314.0),
            left: Val::Px(12.0),
            max_width: Val::Px(390.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.04, 0.025, 0.78)),
        Visibility::Hidden,
        GraphicalEcologyOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA20 graphical lifecycle overlay"),
        Text::new("Lifecycle: loading lineage events..."),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.93, 0.80)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(414.0),
            left: Val::Px(12.0),
            max_width: Val::Px(390.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.045, 0.030, 0.018, 0.78)),
        Visibility::Hidden,
        GraphicalLifecycleOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA23 graphical school panel"),
        Text::new(
            school
                .map(ca23_school_overlay_text)
                .unwrap_or_else(|| "School Mode: disabled".to_string()),
        ),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.88, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(516.0),
            left: Val::Px(12.0),
            max_width: Val::Px(390.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.022, 0.052, 0.80)),
        Visibility::Hidden,
        GraphicalSchoolOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA28 topological concept overlay"),
        Text::new(
            "Concept Map (read-only)\nnodes=0 edges=0 gaps=0 tick=0\nnode: waiting for sealed topology update\nedge: pending concept relation\ngap: none open\nevent link: pending sealed behavior\nBoundary: bias/context only; no actions",
        ),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgb(0.74, 1.0, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(362.0),
            right: Val::Px(12.0),
            max_width: Val::Px(380.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.012, 0.036, 0.032, 0.84)),
        Visibility::Hidden,
        GraphicalTopologyOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA29 creature memory history journal"),
        Text::new(
            "Memory Journal (read-only)\nmemories=0 tick=0 patches=0 bias_rows=0\npatch: waiting for sealed experience\nmemory: waiting for stored expectancy\nbias: neutral expectancy\nSave/load: stable memory IDs visible\nBoundary: expectancy bias only; no action replay",
        ),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.95, 0.72)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(488.0),
            right: Val::Px(12.0),
            max_width: Val::Px(430.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.048, 0.036, 0.012, 0.86)),
        Visibility::Hidden,
        GraphicalMemoryJournalOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA30 neural activity profiler"),
        Text::new(
            "Neural Profiler (compact)\nbrain=0 neurons=0 tick=0\nlobes: pending compact summary\ntiles 0/0 skip=0 syn 0/0\nroute pending backend=PendingFirstTick fallback=none\nBoundary: compact summary; offline export only",
        ),
        TextFont {
            font_size: 10.0,
            ..default()
        },
        TextColor(Color::srgb(0.72, 0.92, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(626.0),
            right: Val::Px(12.0),
            max_width: Val::Px(430.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.012, 0.024, 0.048, 0.86)),
        Visibility::Hidden,
        GraphicalNeuralActivityProfilerOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA05 controls and legend panel"),
        Text::new(
            if summary.view_mode == GraphicalPlaygroundViewMode::Player {
                ca42a_player_controls_bar_text()
            } else {
                ca05_controls_bar_text()
            },
        ),
        TextFont {
            font_size: 6.6,
            ..default()
        },
        TextColor(Color::srgba(0.95, 0.94, 0.86, 0.86)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            left: Val::Px(10.0),
            max_width: Val::Px(265.0),
            padding: bevy::ui::UiRect::all(Val::Px(4.0)),
            ..default()
        },
        player_hud_text_background(summary.view_mode, Color::srgba(0.025, 0.025, 0.02, 0.46)),
        ReadabilityLegendOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA05 event feed panel"),
        Text::new("Events: waiting for first tick"),
        TextFont {
            font_size: 6.4,
            ..default()
        },
        TextColor(Color::srgba(0.94, 0.98, 0.94, 0.82)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            right: Val::Px(10.0),
            max_width: Val::Px(170.0),
            padding: bevy::ui::UiRect::all(Val::Px(4.0)),
            ..default()
        },
        player_hud_text_background(summary.view_mode, Color::srgba(0.018, 0.03, 0.024, 0.44)),
        FeedbackCueOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA40 first-session tutorial panel"),
        Text::new(crate::ca40_first_session_tutorial_placeholder_text()),
        TextFont {
            font_size: 9.5,
            ..default()
        },
        TextColor(Color::srgb(0.88, 1.0, 0.82)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(18.0),
            left: Val::Px(500.0),
            right: Val::Px(330.0),
            max_width: Val::Px(390.0),
            padding: bevy::ui::UiRect::all(Val::Px(7.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.018, 0.036, 0.02, 0.62)),
        view_mode_visibility(summary.view_mode.dev_overlay_visible()),
        GraphicalOnboardingTutorialOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life S03 read-only creature inspector overlay"),
        Text::new("Inspector loading..."),
        TextFont {
            font_size: 6.8,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            max_width: Val::Px(140.0),
            padding: bevy::ui::UiRect::all(Val::Px(4.0)),
            ..default()
        },
        player_hud_text_background(summary.view_mode, Color::srgba(0.02, 0.025, 0.035, 0.74)),
        InspectorStatusOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA05 CPU-shadow boundary footer"),
        Text::new("Boundary: CPU shadow gate | Claim: pending | no bulk readback=true"),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.90, 0.84)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(226.0),
            left: Val::Px(12.0),
            right: Val::Px(12.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.02, 0.018, 0.86)),
        Visibility::Hidden,
        BoundaryFooterOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA09 player save/load menu"),
        Text::new("Save/Load: M menu | F5 save | F9 load"),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.98, 0.90)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(250.0),
            right: Val::Px(250.0),
            max_width: Val::Px(640.0),
            padding: bevy::ui::UiRect::all(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.015, 0.025, 0.018, 0.90)),
        Visibility::Hidden,
        SaveLoadMenuOverlay,
    ));

    Ok(())
}

fn spawn_production_player_view_hud_skin(
    app: &mut App,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    view_mode: GraphicalPlaygroundViewMode,
) {
    if view_mode != GraphicalPlaygroundViewMode::Player {
        return;
    }
    let Some(handles) = alpha_art else {
        return;
    };

    spawn_hud_skin_layer(
        app,
        "status-panel",
        "ui-status-chip",
        handles.ui_status_chip.clone(),
        HudSkinPlacement {
            top: Some(8.0),
            left: Some(8.0),
            width: 116.0,
            height: 29.0,
            alpha: 0.72,
            ..default()
        },
    );
    spawn_hud_skin_layer(
        app,
        "gpu-status-chip",
        "ui-status-chip",
        handles.ui_status_chip.clone(),
        HudSkinPlacement {
            top: Some(45.0),
            left: Some(14.0),
            width: 56.0,
            height: 17.0,
            alpha: 0.52,
            ..default()
        },
    );
    spawn_hud_skin_layer(
        app,
        "creature-inspector",
        "ui-status-chip",
        handles.ui_status_chip.clone(),
        HudSkinPlacement {
            top: Some(8.0),
            right: Some(8.0),
            width: 132.0,
            height: 29.0,
            alpha: 0.68,
            ..default()
        },
    );
    spawn_hud_skin_layer(
        app,
        "inspector-meter",
        "ui-meter-bar",
        handles.ui_meter_bar.clone(),
        HudSkinPlacement {
            top: Some(45.0),
            right: Some(74.0),
            width: 50.0,
            height: 8.0,
            alpha: 0.46,
            ..default()
        },
    );
    spawn_hud_skin_layer(
        app,
        "controls-panel",
        "ui-status-chip",
        handles.ui_status_chip.clone(),
        HudSkinPlacement {
            bottom: Some(10.0),
            left: Some(8.0),
            width: 248.0,
            height: 23.0,
            alpha: 0.54,
            ..default()
        },
    );
    spawn_hud_skin_layer(
        app,
        "event-feed-panel",
        "ui-status-chip",
        handles.ui_status_chip.clone(),
        HudSkinPlacement {
            bottom: Some(10.0),
            right: Some(8.0),
            width: 150.0,
            height: 23.0,
            alpha: 0.50,
            ..default()
        },
    );
}

#[derive(Debug, Clone, Copy)]
struct HudSkinPlacement {
    top: Option<f32>,
    bottom: Option<f32>,
    left: Option<f32>,
    right: Option<f32>,
    width: f32,
    height: f32,
    alpha: f32,
}

impl Default for HudSkinPlacement {
    fn default() -> Self {
        Self {
            top: None,
            bottom: None,
            left: None,
            right: None,
            width: 128.0,
            height: 128.0,
            alpha: 1.0,
        }
    }
}

fn spawn_hud_skin_layer(
    app: &mut App,
    panel_id: &'static str,
    role: &'static str,
    image: Handle<Image>,
    placement: HudSkinPlacement,
) {
    let mut node = Node {
        position_type: PositionType::Absolute,
        width: Val::Px(placement.width),
        height: Val::Px(placement.height),
        ..default()
    };
    if let Some(top) = placement.top {
        node.top = Val::Px(top);
    }
    if let Some(bottom) = placement.bottom {
        node.bottom = Val::Px(bottom);
    }
    if let Some(left) = placement.left {
        node.left = Val::Px(left);
    }
    if let Some(right) = placement.right {
        node.right = Val::Px(right);
    }

    app.world_mut().spawn((
        Name::new(format!("A-Life production HUD skin {panel_id}")),
        ImageNode::new(image).with_color(Color::srgba(1.0, 1.0, 1.0, placement.alpha)),
        node,
        GraphicalProductionHudSkinLayer {
            role,
            panel_id,
            display_only: true,
        },
    ));
}

fn player_hud_text_background(
    view_mode: GraphicalPlaygroundViewMode,
    debug_background: Color,
) -> BackgroundColor {
    if view_mode == GraphicalPlaygroundViewMode::Player {
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0))
    } else {
        BackgroundColor(debug_background)
    }
}

fn spawn_ca18_social_proximity_cues(
    app: &mut App,
    presentation: &VisibleWorldPresentation,
    view_mode: GraphicalPlaygroundViewMode,
) {
    let visibility = view_mode_visibility(view_mode.topology_lines_visible());
    for cue in ca18_social_proximity_cues(presentation) {
        let Some(from) = presentation
            .objects
            .iter()
            .find(|object| object.stable_id == cue.from_stable_id)
        else {
            continue;
        };
        let Some(to) = presentation
            .objects
            .iter()
            .find(|object| object.stable_id == cue.to_stable_id)
        else {
            continue;
        };
        let start = graphical_position(from);
        let end = graphical_position(to);
        let delta = end - start;
        let length = (delta.x * delta.x + delta.y * delta.y).sqrt();
        if length <= 1.0 {
            continue;
        }
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life CA18 social cue stable:{}-stable:{}",
                cue.from_stable_id.raw(),
                cue.to_stable_id.raw()
            )),
            Sprite {
                color: Color::srgba(0.30, 0.82, 1.0, 0.28),
                custom_size: Some(Vec2::new(length, 3.0)),
                ..default()
            },
            Transform {
                translation: Vec3::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5, 0.22),
                rotation: bevy::prelude::Quat::from_rotation_z(delta.y.atan2(delta.x)),
                ..default()
            },
            GraphicalSocialProximityCue {
                from_stable_id: cue.from_stable_id,
                to_stable_id: cue.to_stable_id,
            },
            visibility,
        ));
    }
}

fn view_mode_visibility(visible: bool) -> Visibility {
    if visible {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

fn spawn_ca19_terrain_zone_visuals(
    app: &mut App,
    ecology: &Ca19GraphicalEcologySummary,
    view_mode: GraphicalPlaygroundViewMode,
) {
    if view_mode == GraphicalPlaygroundViewMode::Player {
        return;
    }
    for zone in &ecology.terrain_zones {
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life CA19 terrain zone {} stable-zone:{}",
                zone.kind.label(),
                zone.zone_id.raw()
            )),
            Sprite {
                color: ca19_terrain_zone_color(zone),
                custom_size: Some(Vec2::splat(zone.radius * GRAPHICAL_WORLD_SCALE * 2.0)),
                ..default()
            },
            Transform::from_xyz(
                zone.center.x * GRAPHICAL_WORLD_SCALE,
                zone.center.z * GRAPHICAL_WORLD_SCALE,
                -1.0,
            ),
            GraphicalTerrainZoneMarker {
                zone_id: zone.zone_id,
                kind: zone.kind,
            },
        ));
    }
}

fn ca19_terrain_zone_color(zone: &Ca19TerrainZoneVisual) -> Color {
    match zone.kind {
        TerrainZoneKind::HazardField => Color::srgba(0.95, 0.12, 0.12, 0.045),
        TerrainZoneKind::Grove | TerrainZoneKind::Meadow => {
            Color::srgba(0.20, 0.72, 0.30, 0.04 + zone.resource_bias * 0.025)
        }
        TerrainZoneKind::Wetland => Color::srgba(0.18, 0.48, 0.84, 0.045),
        TerrainZoneKind::Rocky => Color::srgba(0.55, 0.52, 0.46, 0.045),
        TerrainZoneKind::Nest => Color::srgba(0.78, 0.62, 0.22, 0.045),
    }
}

fn spawn_ca37_world_art_dressing(
    app: &mut App,
    summary: &Ca37WorldArtStyleSummary,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    view_mode: GraphicalPlaygroundViewMode,
    presentation: &VisibleWorldPresentation,
) {
    spawn_ca37_world_art_terrain_canvas(app, summary, alpha_art, view_mode, presentation);
    spawn_production_player_view_ambient_layers(app, summary, alpha_art, view_mode);
    for prop in &summary.dressing_props {
        if view_mode == GraphicalPlaygroundViewMode::Player {
            if let Some(handles) = alpha_art {
                let (image, role) = ca44a_prop_art_for_material(handles, prop.material_id, prop.id);
                app.world_mut().spawn((
                    Name::new(format!(
                        "A-Life CA44A art prop {} {}",
                        prop.material_id, prop.id
                    )),
                    Sprite {
                        image,
                        color: Color::WHITE,
                        custom_size: Some(Vec2::new(
                            ca44a_player_dressing_prop_width(prop.width),
                            ca44a_player_dressing_prop_height(prop.height),
                        )),
                        ..default()
                    },
                    Transform::from_xyz(
                        prop.x * GRAPHICAL_WORLD_SCALE,
                        prop.z * GRAPHICAL_WORLD_SCALE,
                        -0.40 + prop.visual_depth,
                    ),
                    GraphicalAlphaArtBackedSprite {
                        role,
                        stable_id: prop.anchored_stable_id,
                    },
                    GraphicalWorldArtProp {
                        prop_id: prop.id,
                        material_id: prop.material_id,
                        anchored_stable_id: prop.anchored_stable_id,
                        display_only: true,
                    },
                ));
                continue;
            }
        }
        app.world_mut().spawn((
            Name::new(format!("A-Life CA37 {} {}", prop.material_id, prop.id)),
            Sprite {
                color: ca37_world_art_prop_color(prop.material_id),
                custom_size: Some(Vec2::new(
                    prop.width * GRAPHICAL_WORLD_SCALE,
                    prop.height * GRAPHICAL_WORLD_SCALE,
                )),
                ..default()
            },
            Transform::from_xyz(
                prop.x * GRAPHICAL_WORLD_SCALE,
                prop.z * GRAPHICAL_WORLD_SCALE,
                -0.52 + prop.visual_depth,
            ),
            GraphicalWorldArtProp {
                prop_id: prop.id,
                material_id: prop.material_id,
                anchored_stable_id: prop.anchored_stable_id,
                display_only: true,
            },
            GraphicalAlphaArtFallbackSprite {
                role: "prop-dressing",
                reason: "alpha art handles unavailable",
            },
        ));
    }
}

fn spawn_production_player_view_ambient_layers(
    _app: &mut App,
    _summary: &Ca37WorldArtStyleSummary,
    _alpha_art: Option<&GraphicalAlphaArtHandles>,
    view_mode: GraphicalPlaygroundViewMode,
) {
    if view_mode != GraphicalPlaygroundViewMode::Player {
        return;
    }
}

fn spawn_ca37_world_art_terrain_canvas(
    app: &mut App,
    summary: &Ca37WorldArtStyleSummary,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    view_mode: GraphicalPlaygroundViewMode,
    presentation: &VisibleWorldPresentation,
) {
    if matches!(
        view_mode,
        GraphicalPlaygroundViewMode::Player | GraphicalPlaygroundViewMode::FullDebug
    ) {
        if let Some(handles) = alpha_art {
            ca44a_spawn_world_backdrop_app(app, handles, view_mode);
        }
    }
    let config = ca44a_procedural_world_config(summary);
    let anchors = ca44a_procedural_world_anchors_from_presentation(presentation);
    let activation = activate_procedural_chunks_around_creatures(config, &anchors)
        .expect("graphical procedural world activation should validate");
    let mut field = GraphicalProceduralTerrainFieldResource::new(summary, &activation);
    if view_mode == GraphicalPlaygroundViewMode::Player {
        ca44a_spawn_runtime_procedural_biome_map_app(app, summary, &field);
    }
    for (center_x, center_z, anchor) in ca44a_initial_procedural_terrain_centers(&activation) {
        ca44a_materialize_terrain_chunk_app(
            app, summary, &mut field, alpha_art, view_mode, center_x, center_z, anchor,
        );
    }
    if let Ok(content) = generate_procedural_world_content(config, &activation) {
        field.record_content_report(&content);
        ca44a_spawn_procedural_world_content_app(app, &mut field, alpha_art, view_mode, &content);
    }
    app.insert_resource(field);
}

fn ca44a_spawn_world_backdrop_app(
    app: &mut App,
    handles: &GraphicalAlphaArtHandles,
    view_mode: GraphicalPlaygroundViewMode,
) {
    let alpha = if view_mode == GraphicalPlaygroundViewMode::Player {
        0.99
    } else {
        0.35
    };
    let custom_size = if view_mode == GraphicalPlaygroundViewMode::Player {
        Vec2::new(
            CA44A_PLAYER_WORLD_BACKDROP_WIDTH,
            CA44A_PLAYER_WORLD_BACKDROP_HEIGHT,
        )
    } else {
        Vec2::new(1_280.0, 720.0)
    };
    let layer_role = "world-painted-viewport";
    app.world_mut().spawn((
        Name::new("A-Life alpha procedural painted viewport"),
        Sprite {
            image: handles.world_backdrop.clone(),
            color: Color::srgba(1.0, 1.0, 1.0, alpha),
            custom_size: Some(custom_size),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -1.90),
        GraphicalAlphaArtBackedSprite {
            role: layer_role,
            stable_id: None,
        },
        GraphicalProductionArtLayer {
            role: layer_role,
            display_only: true,
        },
    ));
}

fn ca44a_spawn_runtime_procedural_biome_map_app(
    app: &mut App,
    summary: &Ca37WorldArtStyleSummary,
    field: &GraphicalProceduralTerrainFieldResource,
) {
    if !app.world().contains_resource::<Assets<Image>>() {
        app.init_resource::<Assets<Image>>();
    }
    let (image, metrics) = ca44a_generate_runtime_procedural_biome_map(summary.seed);
    let texture_width_px = image.texture_descriptor.size.width;
    let texture_height_px = image.texture_descriptor.size.height;
    let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.world_mut().spawn((
        Name::new("A-Life runtime procedural biome map"),
        Sprite {
            image: handle,
            color: Color::srgba(1.0, 1.0, 1.0, 0.12),
            custom_size: Some(Vec2::new(
                CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as f32 * GRAPHICAL_WORLD_SCALE,
                CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as f32 * GRAPHICAL_WORLD_SCALE,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -1.96),
        GraphicalRuntimeProceduralBiomeMap {
            width_tiles: CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES,
            height_tiles: CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES,
            texture_width_px,
            texture_height_px,
            pixels_per_tile: CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE,
            virtual_map_width_tiles: field.virtual_map_width_tiles,
            virtual_map_height_tiles: field.virtual_map_height_tiles,
            path_pixels: metrics.path_pixels,
            resource_detail_pixels: metrics.resource_detail_pixels,
            hazard_detail_pixels: metrics.hazard_detail_pixels,
            stone_detail_pixels: metrics.stone_detail_pixels,
            dark_gap_pixels: metrics.dark_gap_pixels,
            generated_from_procedural_sampler: true,
            display_only: true,
        },
        GraphicalProductionArtLayer {
            role: "runtime-procedural-biome-map",
            display_only: true,
        },
    ));
}

#[derive(Debug, Default, Clone, Copy)]
struct RuntimeProceduralBiomeMapMetrics {
    path_pixels: u32,
    resource_detail_pixels: u32,
    hazard_detail_pixels: u32,
    stone_detail_pixels: u32,
    dark_gap_pixels: u32,
}

#[derive(Debug, Clone, Copy)]
struct RuntimeBiomePixel {
    rgb: [u8; 3],
    is_path: bool,
    is_resource_detail: bool,
    is_hazard_detail: bool,
    is_stone_detail: bool,
}

fn ca44a_generate_runtime_procedural_biome_map(
    seed: u64,
) -> (Image, RuntimeProceduralBiomeMapMetrics) {
    let width_px =
        (CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as u32) * CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE;
    let height_px =
        (CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as u32) * CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE;
    let mut data = Vec::with_capacity((width_px * height_px * 4) as usize);
    let mut metrics = RuntimeProceduralBiomeMapMetrics::default();
    let half_width_tiles = CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES / 2;
    let half_height_tiles = CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES / 2;
    for y in 0..height_px {
        let tile_z = (height_px.saturating_sub(1) - y) as i32
            / CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE as i32
            - half_height_tiles;
        let local_y = y % CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE;
        for x in 0..width_px {
            let tile_x =
                x as i32 / CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE as i32 - half_width_tiles;
            let local_x = x % CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE;
            let sample = ca44a_procedural_terrain_sample(seed, tile_x, tile_z);
            let pixel = ca44a_runtime_biome_pixel(
                seed,
                sample.material.material_id(),
                tile_x,
                tile_z,
                local_x,
                local_y,
            );
            if pixel.is_path {
                metrics.path_pixels += 1;
            }
            if pixel.is_resource_detail {
                metrics.resource_detail_pixels += 1;
            }
            if pixel.is_hazard_detail {
                metrics.hazard_detail_pixels += 1;
            }
            if pixel.is_stone_detail {
                metrics.stone_detail_pixels += 1;
            }
            let [r, g, b] = pixel.rgb;
            if u16::from(r) + u16::from(g) + u16::from(b) < 92 {
                metrics.dark_gap_pixels += 1;
            }
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    let mut image = Image::new(
        Extent3d {
            width: width_px,
            height: height_px,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::linear();
    (image, metrics)
}

fn ca44a_runtime_biome_pixel(
    seed: u64,
    material_id: &str,
    tile_x: i32,
    tile_z: i32,
    local_x: u32,
    local_y: u32,
) -> RuntimeBiomePixel {
    let fine_hash = ca44a_runtime_pixel_hash(seed, tile_x, tile_z, local_x, local_y);
    let tile_hash = ca44a_runtime_pixel_hash(seed ^ 0x51A7_0001, tile_x, tile_z, 0, 0);
    let coarse_hash = ca44a_runtime_pixel_hash(seed ^ 0xBAD5_EED5, tile_x / 3, tile_z / 3, 0, 0);
    let local_progress_x = local_x as f32 / CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE as f32;
    let local_progress_y = local_y as f32 / CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE as f32;
    let world_x = tile_x as f32 + local_progress_x - 0.5;
    let world_z = tile_z as f32 + local_progress_y - 0.5;
    let resource_weight = ca44a_runtime_resource_weight(world_x, world_z);
    let hazard_weight = ca44a_runtime_hazard_weight(world_x, world_z);
    let stone_weight = ca44a_runtime_stone_weight(world_x, world_z);
    let [base_r, base_g, base_b] = ca44a_runtime_continuous_material_base_rgb(
        material_id,
        resource_weight,
        hazard_weight,
        stone_weight,
    );
    let noise = (fine_hash % 17) as i16 - 8;
    let broad = (coarse_hash % 13) as i16 - 6;
    let mut r = base_r as i16 + noise + broad;
    let mut g = base_g as i16 + noise + broad;
    let mut b = base_b as i16 + noise / 2 + broad;

    let primary_path =
        (world_z - (world_x * 0.22 + (world_x * 0.085 + seed as f32 * 0.0007).sin() * 5.8)).abs();
    let branch_path =
        (world_z * 0.82 + world_x * 0.34 - ((world_x * 0.055 + 2.1).cos() * 4.6)).abs();
    let is_path = primary_path < 1.05 || branch_path < 0.82;
    if is_path {
        let weight = if primary_path.min(branch_path) < 0.45 {
            0.72
        } else {
            0.46
        };
        [r, g, b] = ca44a_blend_rgb_i16([r, g, b], [139, 96, 52], weight);
    }

    let local_cx = (tile_hash % CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE) as i32;
    let local_cy = ((tile_hash / 17) % CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE) as i32;
    let dx = local_x as i32 - local_cx;
    let dy = local_y as i32 - local_cy;
    let radial = dx * dx + dy * dy;
    let mut is_resource_detail = false;
    let mut is_hazard_detail = false;
    let mut is_stone_detail = false;
    let leaf_detail = radial <= 3 && tile_hash % 7 == 0;
    let flower_detail = radial <= 2 && tile_hash % 29 == 0;
    let hazard_spike = radial <= 4
        && (dy.abs() <= 1 || (dx.abs() + dy.abs()) <= 3)
        && (material_id == "hazard-pressure" || hazard_weight > 0.38 || tile_hash % 47 == 0);
    let stone_detail = radial <= 5
        && (material_id == "stone-dressing" || stone_weight > 0.42 || tile_hash % 41 == 0);

    if (material_id == "resource-grove" || resource_weight > 0.34) && (leaf_detail || flower_detail)
    {
        is_resource_detail = true;
        [r, g, b] = ca44a_blend_rgb_i16(
            [r, g, b],
            if flower_detail {
                [234, 190, 96]
            } else {
                [92, 184, 72]
            },
            0.62,
        );
    } else if (material_id == "hazard-pressure" || hazard_weight > 0.30) && hazard_spike {
        is_hazard_detail = true;
        [r, g, b] = ca44a_blend_rgb_i16([r, g, b], [224, 64, 42], 0.70);
    } else if stone_detail {
        is_stone_detail = true;
        [r, g, b] = ca44a_blend_rgb_i16([r, g, b], [145, 145, 132], 0.58);
    } else if material_id == "safe-grass" && leaf_detail {
        is_resource_detail = true;
        [r, g, b] = ca44a_blend_rgb_i16([r, g, b], [116, 172, 72], 0.44);
    }

    RuntimeBiomePixel {
        rgb: [
            r.clamp(22, 235) as u8,
            g.clamp(32, 235) as u8,
            b.clamp(24, 225) as u8,
        ],
        is_path,
        is_resource_detail,
        is_hazard_detail,
        is_stone_detail,
    }
}

fn ca44a_runtime_continuous_material_base_rgb(
    material_id: &str,
    resource_weight: f32,
    hazard_weight: f32,
    stone_weight: f32,
) -> [u8; 3] {
    let mut rgb = ca44a_runtime_material_base_rgb("safe-grass").map(f32::from);
    let sampler_bias = match material_id {
        "neutral-soil" => 0.18,
        "resource-grove" => 0.26,
        "hazard-pressure" => 0.30,
        "stone-dressing" => 0.24,
        _ => 0.0,
    };
    if material_id != "safe-grass" {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb(material_id).map(f32::from),
            sampler_bias,
        );
    }
    if resource_weight > 0.05 {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb("resource-grove").map(f32::from),
            (resource_weight * 0.68).clamp(0.0, 0.68),
        );
    }
    if stone_weight > 0.05 {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb("stone-dressing").map(f32::from),
            (stone_weight * 0.76).clamp(0.0, 0.76),
        );
    }
    if hazard_weight > 0.05 {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb("hazard-pressure").map(f32::from),
            (hazard_weight * 0.82).clamp(0.0, 0.82),
        );
    }
    [
        rgb[0].round().clamp(0.0, 255.0) as u8,
        rgb[1].round().clamp(0.0, 255.0) as u8,
        rgb[2].round().clamp(0.0, 255.0) as u8,
    ]
}

fn ca44a_runtime_material_base_rgb(material_id: &str) -> [u8; 3] {
    match material_id {
        "neutral-soil" => [118, 88, 50],
        "resource-grove" => [64, 135, 58],
        "hazard-pressure" => [142, 63, 45],
        "stone-dressing" => [103, 110, 99],
        _ => [91, 145, 57],
    }
}

fn ca44a_runtime_resource_weight(world_x: f32, world_z: f32) -> f32 {
    (ca44a_smooth_blob(world_x, world_z, -19.0, -5.0, 22.0, 16.0)
        + ca44a_smooth_blob(world_x, world_z, 8.0, 12.0, 25.0, 18.0) * 0.95
        + ca44a_smooth_blob(world_x, world_z, 35.0, -17.0, 17.0, 12.0) * 0.74)
        .clamp(0.0, 1.0)
}

fn ca44a_runtime_hazard_weight(world_x: f32, world_z: f32) -> f32 {
    (ca44a_smooth_blob(world_x, world_z, 45.0, 3.0, 29.0, 22.0)
        + ca44a_smooth_blob(world_x, world_z, -48.0, -23.0, 19.0, 14.0) * 0.88
        + ca44a_smooth_blob(world_x, world_z, 18.0, 24.0, 13.0, 8.0) * 0.42)
        .clamp(0.0, 1.0)
}

fn ca44a_runtime_stone_weight(world_x: f32, world_z: f32) -> f32 {
    (ca44a_smooth_blob(world_x, world_z, -28.0, 22.0, 33.0, 19.0)
        + ca44a_smooth_blob(world_x, world_z, 10.0, 26.0, 24.0, 12.0) * 0.54
        + ca44a_smooth_blob(world_x, world_z, 52.0, -29.0, 14.0, 8.0) * 0.34)
        .clamp(0.0, 1.0)
}

fn ca44a_smooth_blob(
    world_x: f32,
    world_z: f32,
    center_x: f32,
    center_z: f32,
    radius_x: f32,
    radius_z: f32,
) -> f32 {
    let dx = (world_x - center_x) / radius_x.max(0.001);
    let dz = (world_z - center_z) / radius_z.max(0.001);
    let distance = (dx * dx + dz * dz).sqrt();
    (1.0 - distance).clamp(0.0, 1.0)
}

fn ca44a_blend_rgb_f32(base: [f32; 3], overlay: [f32; 3], weight: f32) -> [f32; 3] {
    let weight = weight.clamp(0.0, 1.0);
    let inverse = 1.0 - weight;
    [
        base[0] * inverse + overlay[0] * weight,
        base[1] * inverse + overlay[1] * weight,
        base[2] * inverse + overlay[2] * weight,
    ]
}

fn ca44a_blend_rgb_i16(base: [i16; 3], overlay: [i16; 3], weight: f32) -> [i16; 3] {
    let inverse = 1.0 - weight;
    [
        (base[0] as f32 * inverse + overlay[0] as f32 * weight).round() as i16,
        (base[1] as f32 * inverse + overlay[1] as f32 * weight).round() as i16,
        (base[2] as f32 * inverse + overlay[2] as f32 * weight).round() as i16,
    ]
}

fn ca44a_runtime_pixel_hash(
    seed: u64,
    tile_x: i32,
    tile_z: i32,
    local_x: u32,
    local_y: u32,
) -> u32 {
    let mut value = seed
        .wrapping_add((tile_x as i64 as u64).wrapping_mul(0x9E37_79B1))
        .wrapping_add((tile_z as i64 as u64).wrapping_mul(0x85EB_CA77))
        .wrapping_add((local_x as u64).wrapping_mul(0xC2B2_AE3D))
        .wrapping_add((local_y as u64).wrapping_mul(0x27D4_EB2F));
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51_afd7_ed55_8ccd);
    value ^= value >> 33;
    value as u32
}

impl GraphicalProceduralTerrainFieldResource {
    fn new(
        summary: &Ca37WorldArtStyleSummary,
        activation: &ProceduralChunkActivationReport,
    ) -> Self {
        let config = ca44a_procedural_world_config(summary);
        Self {
            seed: summary.seed,
            virtual_map_width_tiles: config.virtual_width_tiles(),
            virtual_map_height_tiles: config.virtual_height_tiles(),
            chunk_radius_x: (summary.viewport_width_tiles as i32).max(14),
            chunk_radius_z: (summary.viewport_height_tiles as i32).max(10),
            active_world_chunks: activation
                .active_chunks
                .iter()
                .map(|chunk| (chunk.coord.x, chunk.coord.z))
                .collect(),
            creature_anchor_count: activation.creature_anchor_count,
            generated_without_rendering: activation.generated_without_rendering,
            materialized_tiles: BTreeSet::new(),
            materialized_content_stable_ids: BTreeSet::new(),
            materialized_chunk_count: 0,
            active_content_count: 0,
            procedural_content_generated_without_rendering: true,
            procedural_content_rendering_required: false,
            materialized_only_near_active_views: true,
        }
    }

    fn record_content_report(&mut self, content: &ProceduralWorldContentReport) {
        self.active_content_count = content.candidate_count;
        self.procedural_content_generated_without_rendering = content.generated_without_rendering;
        self.procedural_content_rendering_required = content.rendering_required;
    }
}

fn ca44a_procedural_world_config(summary: &Ca37WorldArtStyleSummary) -> ProceduralWorldConfig {
    ProceduralWorldConfig::with_seed(summary.seed)
}

fn ca44a_procedural_world_anchors_from_presentation(
    presentation: &VisibleWorldPresentation,
) -> Vec<CreatureWorldAnchor> {
    presentation
        .objects
        .iter()
        .filter(|object| object.kind == WorldObjectKind::Agent)
        .filter_map(|object| CreatureWorldAnchor::new(object.stable_id, object.position).ok())
        .collect()
}

fn ca44a_initial_procedural_terrain_centers(
    activation: &ProceduralChunkActivationReport,
) -> Vec<(i32, i32, Option<WorldEntityId>)> {
    let mut centers = Vec::new();
    let mut seen_anchors = BTreeSet::new();
    for chunk in &activation.active_chunks {
        if seen_anchors.insert(chunk.anchor_stable_id.raw()) {
            centers.push((
                chunk.anchor_tile.x,
                chunk.anchor_tile.z,
                Some(chunk.anchor_stable_id),
            ));
        }
    }
    centers
}

fn ca44a_materialize_terrain_chunk_app(
    app: &mut App,
    summary: &Ca37WorldArtStyleSummary,
    field: &mut GraphicalProceduralTerrainFieldResource,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    view_mode: GraphicalPlaygroundViewMode,
    center_x: i32,
    center_z: i32,
    anchor: Option<WorldEntityId>,
) {
    field.materialized_chunk_count = field.materialized_chunk_count.saturating_add(1);
    for ix in center_x - field.chunk_radius_x..=center_x + field.chunk_radius_x {
        for iz in center_z - field.chunk_radius_z..=center_z + field.chunk_radius_z {
            if !ca44a_virtual_tile_in_bounds(field, ix, iz)
                || !field.materialized_tiles.insert((ix, iz))
            {
                continue;
            }
            ca44a_spawn_terrain_tile_app(
                app, summary, field, alpha_art, view_mode, ix, iz, center_x, center_z, anchor,
            );
        }
    }
}

fn ca44a_spawn_procedural_world_content_app(
    app: &mut App,
    field: &mut GraphicalProceduralTerrainFieldResource,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    view_mode: GraphicalPlaygroundViewMode,
    content: &ProceduralWorldContentReport,
) {
    for candidate in &content.candidates {
        if !field
            .materialized_content_stable_ids
            .insert(candidate.stable_id.raw())
        {
            continue;
        }
        ca44a_spawn_procedural_content_candidate_app(app, alpha_art, view_mode, candidate);
    }
}

fn ca44a_spawn_procedural_content_candidate_app(
    app: &mut App,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    view_mode: GraphicalPlaygroundViewMode,
    candidate: &ProceduralWorldContentCandidate,
) {
    let position = Vec3::new(
        candidate.position.x * GRAPHICAL_WORLD_SCALE,
        candidate.position.z * GRAPHICAL_WORLD_SCALE,
        ca44a_procedural_content_z(candidate.kind),
    );
    if view_mode == GraphicalPlaygroundViewMode::Player {
        if let Some(handles) = alpha_art {
            let (image, role) = ca44a_procedural_content_art_handle(handles, candidate);
            app.world_mut().spawn((
                Name::new(format!(
                    "A-Life procedural world content {role} stable:{}",
                    candidate.stable_id.raw()
                )),
                Sprite {
                    image,
                    color: Color::WHITE,
                    custom_size: Some(ca44a_procedural_content_sprite_size(candidate.kind)),
                    ..default()
                },
                Transform::from_translation(position),
                GraphicalAlphaArtBackedSprite {
                    role,
                    stable_id: Some(candidate.stable_id),
                },
                GraphicalProductionArtLayer {
                    role: "procedural-world-content",
                    display_only: true,
                },
                ca44a_procedural_content_marker(candidate),
            ));
            return;
        }
    }
    app.world_mut().spawn((
        Name::new(format!(
            "A-Life procedural content fallback stable:{}",
            candidate.stable_id.raw()
        )),
        Sprite {
            color: ca44a_procedural_content_fallback_color(candidate.kind),
            custom_size: Some(ca44a_procedural_content_sprite_size(candidate.kind) * 0.72),
            ..default()
        },
        Transform::from_translation(position),
        GraphicalAlphaArtFallbackSprite {
            role: candidate.kind.alpha_art_role(),
            reason: "alpha art handles unavailable or dev/full debug view",
        },
        ca44a_procedural_content_marker(candidate),
    ));
}

fn ca44a_procedural_content_marker(
    candidate: &ProceduralWorldContentCandidate,
) -> GraphicalProceduralWorldContentMarker {
    GraphicalProceduralWorldContentMarker {
        stable_id: candidate.stable_id,
        kind: candidate.kind,
        anchor_stable_id: candidate.anchor_stable_id,
        world_chunk_x: candidate.chunk.x,
        world_chunk_z: candidate.chunk.z,
        tile_x: candidate.tile.x,
        tile_z: candidate.tile.z,
        generated_without_rendering: candidate.generated_without_rendering,
        rendering_required: candidate.rendering_required,
        creature_context_candidate: candidate.bounded_for_creature_context,
        can_emit_actions: candidate.can_emit_actions,
        can_rewrite_weights: candidate.can_rewrite_weights,
    }
}

fn ca44a_virtual_tile_in_bounds(
    field: &GraphicalProceduralTerrainFieldResource,
    ix: i32,
    iz: i32,
) -> bool {
    let half_x = field.virtual_map_width_tiles as i32 / 2;
    let half_z = field.virtual_map_height_tiles as i32 / 2;
    (-half_x..=half_x).contains(&ix) && (-half_z..=half_z).contains(&iz)
}

fn ca44a_spawn_terrain_tile_app(
    app: &mut App,
    summary: &Ca37WorldArtStyleSummary,
    field: &GraphicalProceduralTerrainFieldResource,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    view_mode: GraphicalPlaygroundViewMode,
    ix: i32,
    iz: i32,
    center_x: i32,
    center_z: i32,
    anchor: Option<WorldEntityId>,
) {
    if view_mode == GraphicalPlaygroundViewMode::Player {
        if let Some(handles) = alpha_art {
            ca44a_spawn_alpha_terrain_tile_app(
                app, summary, field, handles, ix, iz, center_x, center_z, anchor,
            );
            return;
        }
    }
    ca44a_spawn_fallback_terrain_tile_app(app, summary, field, ix, iz, center_x, center_z, anchor);
}

fn ca44a_spawn_alpha_terrain_tile_app(
    app: &mut App,
    summary: &Ca37WorldArtStyleSummary,
    field: &GraphicalProceduralTerrainFieldResource,
    handles: &GraphicalAlphaArtHandles,
    ix: i32,
    iz: i32,
    center_x: i32,
    center_z: i32,
    anchor: Option<WorldEntityId>,
) {
    let sample = ca44a_procedural_terrain_sample(summary.seed, ix, iz);
    let material_id = sample.material.material_id();
    let hash = ca37_seeded_terrain_hash(summary.seed, ix, iz);
    let jitter_x = ca44a_terrain_jitter_x(hash);
    let jitter_y = ca44a_terrain_jitter_y(hash);
    let width = ca44a_terrain_tile_width(hash);
    let height = ca44a_terrain_tile_height(hash);
    let rotation_degrees = ca44a_terrain_rotation_degrees(hash);
    let opacity = ca44a_alpha_terrain_opacity(material_id);
    let (image, role) = ca44a_terrain_art_for_material(handles, material_id);
    app.world_mut().spawn((
        Name::new(format!(
            "A-Life procedural terrain chunk art {material_id} {ix}:{iz}"
        )),
        Sprite {
            image,
            color: Color::srgba(1.0, 1.0, 1.0, opacity),
            custom_size: Some(Vec2::new(width, height)),
            ..default()
        },
        Transform::from_xyz(
            ix as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + jitter_x,
            iz as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + jitter_y,
            -1.42,
        )
        .with_rotation(Quat::from_rotation_z(rotation_degrees.to_radians())),
        GraphicalAlphaArtBackedSprite {
            role,
            stable_id: None,
        },
        GraphicalWorldArtTerrainTile {
            tile_x: ix,
            tile_z: iz,
            material_id,
            tile_size_pixels: CA37_TERRAIN_TILE_PIXEL_SIZE,
            organic_rotation_degrees: rotation_degrees,
            opacity,
            viewport_slice: summary.local_viewport_is_smaller_than_map,
            display_only: true,
        },
        GraphicalProceduralTerrainChunkTile {
            anchor_stable_id: anchor,
            world_chunk_x: sample.chunk.x,
            world_chunk_z: sample.chunk.z,
            chunk_center_tile_x: center_x,
            chunk_center_tile_z: center_z,
            virtual_map_width_tiles: field.virtual_map_width_tiles,
            virtual_map_height_tiles: field.virtual_map_height_tiles,
            creature_authoritative_chunk: anchor.is_some(),
            rendering_required_for_generation: false,
            materialized_only_near_active_views: field.materialized_only_near_active_views,
        },
        GraphicalProductionArtLayer {
            role: "streamed-procedural-terrain",
            display_only: true,
        },
    ));
    if hash % 7 == 0 {
        let edge_rotation = (rotation_degrees + 38.0).to_radians();
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life production terrain edge blend {material_id} {ix}:{iz}"
            )),
            Sprite {
                image: handles.terrain_edge_blend.clone(),
                color: Color::srgba(1.0, 1.0, 1.0, 0.025),
                custom_size: Some(Vec2::new(
                    CA37_TERRAIN_TILE_PIXEL_SIZE * 0.88,
                    CA37_TERRAIN_TILE_PIXEL_SIZE * 0.54,
                )),
                ..default()
            },
            Transform::from_xyz(
                ix as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + jitter_x * 0.6,
                iz as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + jitter_y * 0.6,
                -1.32,
            )
            .with_rotation(Quat::from_rotation_z(edge_rotation)),
            GraphicalAlphaArtBackedSprite {
                role: "terrain-edge-blend",
                stable_id: None,
            },
            GraphicalProductionArtLayer {
                role: "terrain-edge-blend",
                display_only: true,
            },
        ));
    }
}

fn ca44a_spawn_fallback_terrain_tile_app(
    app: &mut App,
    summary: &Ca37WorldArtStyleSummary,
    field: &GraphicalProceduralTerrainFieldResource,
    ix: i32,
    iz: i32,
    center_x: i32,
    center_z: i32,
    anchor: Option<WorldEntityId>,
) {
    let sample = ca44a_procedural_terrain_sample(summary.seed, ix, iz);
    let material_id = sample.material.material_id();
    let hash = ca37_seeded_terrain_hash(summary.seed, ix, iz);
    app.world_mut().spawn((
        Name::new(format!("A-Life CA37 terrain wash {material_id} {ix}:{iz}")),
        Sprite {
            color: ca37_terrain_tile_color(summary.seed, material_id, ix, iz),
            custom_size: Some(Vec2::new(
                ca44a_terrain_tile_width(hash),
                ca44a_terrain_tile_height(hash),
            )),
            ..default()
        },
        Transform::from_xyz(
            ix as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + ca44a_terrain_jitter_x(hash),
            iz as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + ca44a_terrain_jitter_y(hash),
            -1.45,
        ),
        GraphicalWorldArtTerrainTile {
            tile_x: ix,
            tile_z: iz,
            material_id,
            tile_size_pixels: CA37_TERRAIN_TILE_PIXEL_SIZE,
            organic_rotation_degrees: 0.0,
            opacity: ca37_terrain_tile_alpha(material_id),
            viewport_slice: summary.local_viewport_is_smaller_than_map,
            display_only: true,
        },
        GraphicalProceduralTerrainChunkTile {
            anchor_stable_id: anchor,
            world_chunk_x: sample.chunk.x,
            world_chunk_z: sample.chunk.z,
            chunk_center_tile_x: center_x,
            chunk_center_tile_z: center_z,
            virtual_map_width_tiles: field.virtual_map_width_tiles,
            virtual_map_height_tiles: field.virtual_map_height_tiles,
            creature_authoritative_chunk: anchor.is_some(),
            rendering_required_for_generation: false,
            materialized_only_near_active_views: field.materialized_only_near_active_views,
        },
        GraphicalAlphaArtFallbackSprite {
            role: "terrain-fallback",
            reason: "alpha art handles unavailable",
        },
    ));
}

fn ca44a_terrain_jitter_x(hash: i32) -> f32 {
    ((hash % 7) as f32 - 3.0) * (CA37_TERRAIN_TILE_JITTER_PIXELS / 3.0)
}

fn ca44a_terrain_jitter_y(hash: i32) -> f32 {
    (((hash / 7) % 7) as f32 - 3.0) * (CA37_TERRAIN_TILE_JITTER_PIXELS / 3.0)
}

fn ca44a_terrain_tile_width(hash: i32) -> f32 {
    let variant = ((hash & 0x7) as f32) / 7.0;
    CA37_TERRAIN_TILE_PIXEL_SIZE * (1.12 + variant * 0.06)
}

fn ca44a_terrain_tile_height(hash: i32) -> f32 {
    let variant = (((hash >> 4) & 0x7) as f32) / 7.0;
    CA37_TERRAIN_TILE_PIXEL_SIZE * (1.08 + variant * 0.06)
}

fn ca44a_terrain_rotation_degrees(hash: i32) -> f32 {
    (((hash % 17) as f32) - 8.0) * 1.2
}

fn ca44a_alpha_terrain_opacity(material_id: &str) -> f32 {
    match material_id {
        "hazard-pressure" => 0.007,
        "resource-grove" => 0.006,
        "stone-dressing" => 0.005,
        "neutral-soil" => 0.004,
        _ => 0.004,
    }
}

fn update_graphical_procedural_terrain_field(
    mut commands: Commands,
    view_mode: Res<GraphicalViewModeResource>,
    _camera: Option<Res<CameraNavigationResource>>,
    world_art: Option<Res<GraphicalWorldArtStyleResource>>,
    alpha_art: Option<Res<GraphicalAlphaArtHandles>>,
    field: Option<ResMut<GraphicalProceduralTerrainFieldResource>>,
    markers: bevy::prelude::Query<(&GraphicalPlaygroundMarker, &Transform)>,
) {
    if view_mode.mode != GraphicalPlaygroundViewMode::Player {
        return;
    }
    let Some(world_art) = world_art else {
        return;
    };
    let Some(mut field) = field else {
        return;
    };

    let mut anchors = Vec::new();
    for (marker, transform) in &markers {
        if marker.kind == WorldObjectKind::Agent {
            let position = Vec3f::new(
                transform.translation.x / GRAPHICAL_WORLD_SCALE,
                0.0,
                transform.translation.y / GRAPHICAL_WORLD_SCALE,
            );
            if let Ok(anchor) = CreatureWorldAnchor::new(marker.stable_id, position) {
                anchors.push(anchor);
            }
        }
    }
    let config = ca44a_procedural_world_config(&world_art.summary);
    let Ok(activation) = activate_procedural_chunks_around_creatures(config, &anchors) else {
        return;
    };
    let content = generate_procedural_world_content(config, &activation).ok();
    field.active_world_chunks = activation
        .active_chunks
        .iter()
        .map(|chunk| (chunk.coord.x, chunk.coord.z))
        .collect();
    field.creature_anchor_count = activation.creature_anchor_count;
    field.generated_without_rendering = activation.generated_without_rendering;
    if let Some(content) = &content {
        field.record_content_report(content);
    }
    let centers = ca44a_initial_procedural_terrain_centers(&activation);
    if centers.is_empty() {
        return;
    }

    let half_x = field.virtual_map_width_tiles as i32 / 2;
    let half_z = field.virtual_map_height_tiles as i32 / 2;
    for (center_x, center_z, anchor) in centers {
        ca44a_materialize_terrain_chunk_commands(
            &mut commands,
            &world_art.summary,
            field.as_mut(),
            center_x.clamp(-half_x, half_x),
            center_z.clamp(-half_z, half_z),
            anchor,
            alpha_art.as_deref(),
        );
    }
    if let Some(content) = &content {
        ca44a_spawn_procedural_world_content_commands(
            &mut commands,
            field.as_mut(),
            alpha_art.as_deref(),
            content,
        );
    }
}

fn ca44a_materialize_terrain_chunk_commands(
    commands: &mut Commands,
    summary: &Ca37WorldArtStyleSummary,
    field: &mut GraphicalProceduralTerrainFieldResource,
    center_x: i32,
    center_z: i32,
    anchor: Option<WorldEntityId>,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
) {
    field.materialized_chunk_count = field.materialized_chunk_count.saturating_add(1);
    for ix in center_x - field.chunk_radius_x..=center_x + field.chunk_radius_x {
        for iz in center_z - field.chunk_radius_z..=center_z + field.chunk_radius_z {
            if !ca44a_virtual_tile_in_bounds(field, ix, iz)
                || !field.materialized_tiles.insert((ix, iz))
            {
                continue;
            }
            if let Some(handles) = alpha_art {
                ca44a_spawn_alpha_terrain_tile_commands(
                    commands, summary, field, handles, ix, iz, center_x, center_z, anchor,
                );
            } else {
                ca44a_spawn_fallback_terrain_tile_commands(
                    commands, summary, field, ix, iz, center_x, center_z, anchor,
                );
            }
        }
    }
}

fn ca44a_spawn_alpha_terrain_tile_commands(
    commands: &mut Commands,
    summary: &Ca37WorldArtStyleSummary,
    field: &GraphicalProceduralTerrainFieldResource,
    handles: &GraphicalAlphaArtHandles,
    ix: i32,
    iz: i32,
    center_x: i32,
    center_z: i32,
    anchor: Option<WorldEntityId>,
) {
    let sample = ca44a_procedural_terrain_sample(summary.seed, ix, iz);
    let material_id = sample.material.material_id();
    let hash = ca37_seeded_terrain_hash(summary.seed, ix, iz);
    let jitter_x = ca44a_terrain_jitter_x(hash);
    let jitter_y = ca44a_terrain_jitter_y(hash);
    let width = ca44a_terrain_tile_width(hash);
    let height = ca44a_terrain_tile_height(hash);
    let rotation_degrees = ca44a_terrain_rotation_degrees(hash);
    let opacity = ca44a_alpha_terrain_opacity(material_id);
    let (image, role) = ca44a_terrain_art_for_material(handles, material_id);
    commands.spawn((
        Name::new(format!(
            "A-Life streamed procedural terrain {material_id} {ix}:{iz}"
        )),
        Sprite {
            image,
            color: Color::srgba(1.0, 1.0, 1.0, opacity),
            custom_size: Some(Vec2::new(width, height)),
            ..default()
        },
        Transform::from_xyz(
            ix as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + jitter_x,
            iz as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + jitter_y,
            -1.42,
        )
        .with_rotation(Quat::from_rotation_z(rotation_degrees.to_radians())),
        GraphicalAlphaArtBackedSprite {
            role,
            stable_id: None,
        },
        GraphicalWorldArtTerrainTile {
            tile_x: ix,
            tile_z: iz,
            material_id,
            tile_size_pixels: CA37_TERRAIN_TILE_PIXEL_SIZE,
            organic_rotation_degrees: rotation_degrees,
            opacity,
            viewport_slice: summary.local_viewport_is_smaller_than_map,
            display_only: true,
        },
        GraphicalProceduralTerrainChunkTile {
            anchor_stable_id: anchor,
            world_chunk_x: sample.chunk.x,
            world_chunk_z: sample.chunk.z,
            chunk_center_tile_x: center_x,
            chunk_center_tile_z: center_z,
            virtual_map_width_tiles: field.virtual_map_width_tiles,
            virtual_map_height_tiles: field.virtual_map_height_tiles,
            creature_authoritative_chunk: anchor.is_some(),
            rendering_required_for_generation: false,
            materialized_only_near_active_views: field.materialized_only_near_active_views,
        },
        GraphicalProductionArtLayer {
            role: "streamed-procedural-terrain",
            display_only: true,
        },
    ));
    if hash % 7 == 0 {
        commands.spawn((
            Name::new(format!(
                "A-Life streamed terrain organic blend {material_id} {ix}:{iz}"
            )),
            Sprite {
                image: handles.terrain_edge_blend.clone(),
                color: Color::srgba(1.0, 1.0, 1.0, 0.025),
                custom_size: Some(Vec2::new(
                    CA37_TERRAIN_TILE_PIXEL_SIZE * 0.88,
                    CA37_TERRAIN_TILE_PIXEL_SIZE * 0.54,
                )),
                ..default()
            },
            Transform::from_xyz(
                ix as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + jitter_x * 0.6,
                iz as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + jitter_y * 0.6,
                -1.31,
            )
            .with_rotation(Quat::from_rotation_z(
                (rotation_degrees + 38.0).to_radians(),
            )),
            GraphicalAlphaArtBackedSprite {
                role: "terrain-edge-blend",
                stable_id: None,
            },
            GraphicalProductionArtLayer {
                role: "terrain-edge-blend",
                display_only: true,
            },
        ));
    }
}

fn ca44a_spawn_procedural_world_content_commands(
    commands: &mut Commands,
    field: &mut GraphicalProceduralTerrainFieldResource,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    content: &ProceduralWorldContentReport,
) {
    for candidate in &content.candidates {
        if !field
            .materialized_content_stable_ids
            .insert(candidate.stable_id.raw())
        {
            continue;
        }
        ca44a_spawn_procedural_content_candidate_commands(commands, alpha_art, candidate);
    }
}

fn ca44a_spawn_procedural_content_candidate_commands(
    commands: &mut Commands,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    candidate: &ProceduralWorldContentCandidate,
) {
    let position = Vec3::new(
        candidate.position.x * GRAPHICAL_WORLD_SCALE,
        candidate.position.z * GRAPHICAL_WORLD_SCALE,
        ca44a_procedural_content_z(candidate.kind),
    );
    if let Some(handles) = alpha_art {
        let (image, role) = ca44a_procedural_content_art_handle(handles, candidate);
        commands.spawn((
            Name::new(format!(
                "A-Life streamed procedural content {role} stable:{}",
                candidate.stable_id.raw()
            )),
            Sprite {
                image,
                color: Color::WHITE,
                custom_size: Some(ca44a_procedural_content_sprite_size(candidate.kind)),
                ..default()
            },
            Transform::from_translation(position),
            GraphicalAlphaArtBackedSprite {
                role,
                stable_id: Some(candidate.stable_id),
            },
            GraphicalProductionArtLayer {
                role: "procedural-world-content",
                display_only: true,
            },
            ca44a_procedural_content_marker(candidate),
        ));
    } else {
        commands.spawn((
            Name::new(format!(
                "A-Life streamed procedural content fallback stable:{}",
                candidate.stable_id.raw()
            )),
            Sprite {
                color: ca44a_procedural_content_fallback_color(candidate.kind),
                custom_size: Some(ca44a_procedural_content_sprite_size(candidate.kind) * 0.72),
                ..default()
            },
            Transform::from_translation(position),
            GraphicalAlphaArtFallbackSprite {
                role: candidate.kind.alpha_art_role(),
                reason: "alpha art handles unavailable",
            },
            ca44a_procedural_content_marker(candidate),
        ));
    }
}

fn ca44a_spawn_fallback_terrain_tile_commands(
    commands: &mut Commands,
    summary: &Ca37WorldArtStyleSummary,
    field: &GraphicalProceduralTerrainFieldResource,
    ix: i32,
    iz: i32,
    center_x: i32,
    center_z: i32,
    anchor: Option<WorldEntityId>,
) {
    let sample = ca44a_procedural_terrain_sample(summary.seed, ix, iz);
    let material_id = sample.material.material_id();
    let hash = ca37_seeded_terrain_hash(summary.seed, ix, iz);
    commands.spawn((
        Name::new(format!(
            "A-Life fallback streamed terrain {material_id} {ix}:{iz}"
        )),
        Sprite {
            color: ca37_terrain_tile_color(summary.seed, material_id, ix, iz),
            custom_size: Some(Vec2::new(
                ca44a_terrain_tile_width(hash),
                ca44a_terrain_tile_height(hash),
            )),
            ..default()
        },
        Transform::from_xyz(
            ix as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + ca44a_terrain_jitter_x(hash),
            iz as f32 * CA37_TERRAIN_TILE_PIXEL_SIZE + ca44a_terrain_jitter_y(hash),
            -1.45,
        ),
        GraphicalWorldArtTerrainTile {
            tile_x: ix,
            tile_z: iz,
            material_id,
            tile_size_pixels: CA37_TERRAIN_TILE_PIXEL_SIZE,
            organic_rotation_degrees: 0.0,
            opacity: ca37_terrain_tile_alpha(material_id),
            viewport_slice: summary.local_viewport_is_smaller_than_map,
            display_only: true,
        },
        GraphicalProceduralTerrainChunkTile {
            anchor_stable_id: anchor,
            world_chunk_x: sample.chunk.x,
            world_chunk_z: sample.chunk.z,
            chunk_center_tile_x: center_x,
            chunk_center_tile_z: center_z,
            virtual_map_width_tiles: field.virtual_map_width_tiles,
            virtual_map_height_tiles: field.virtual_map_height_tiles,
            creature_authoritative_chunk: anchor.is_some(),
            rendering_required_for_generation: false,
            materialized_only_near_active_views: field.materialized_only_near_active_views,
        },
        GraphicalAlphaArtFallbackSprite {
            role: "terrain-fallback",
            reason: "alpha art handles unavailable",
        },
    ));
}

fn ca44a_terrain_art_for_material(
    handles: &GraphicalAlphaArtHandles,
    material_id: &str,
) -> (Handle<Image>, &'static str) {
    match material_id {
        "neutral-soil" => (handles.terrain_soil_path.clone(), "terrain-soil-path"),
        "resource-grove" => (
            handles.terrain_resource_grove.clone(),
            "terrain-resource-grove",
        ),
        "hazard-pressure" => (
            handles.terrain_hazard_pressure.clone(),
            "terrain-hazard-pressure",
        ),
        "stone-dressing" => (handles.terrain_stone_rough.clone(), "terrain-stone-rough"),
        _ => (handles.terrain_safe_grass.clone(), "terrain-safe-grass"),
    }
}

fn ca44a_prop_art_for_material(
    handles: &GraphicalAlphaArtHandles,
    material_id: &str,
    prop_id: &str,
) -> (Handle<Image>, &'static str) {
    let variant = ca37_seeded_prop_variant(prop_id);
    match material_id {
        "hazard-pressure" => (handles.prop_warning_shard.clone(), "prop-dressing"),
        "stone-dressing" => (handles.prop_pebble_cluster.clone(), "prop-dressing"),
        "resource-grove" => match variant % 3 {
            0 => (handles.prop_leaf_patch.clone(), "prop-dressing"),
            1 => (handles.prop_mushroom_cluster.clone(), "prop-dressing"),
            _ => (handles.prop_grass_tuft.clone(), "prop-dressing"),
        },
        _ => {
            if variant % 5 == 0 {
                (handles.prop_mushroom_cluster.clone(), "prop-dressing")
            } else {
                (handles.prop_grass_tuft.clone(), "prop-dressing")
            }
        }
    }
}

fn ca44a_player_dressing_prop_width(width_world: f32) -> f32 {
    (width_world * GRAPHICAL_WORLD_SCALE * 0.16).clamp(3.5, 8.5)
}

fn ca44a_player_dressing_prop_height(height_world: f32) -> f32 {
    (height_world * GRAPHICAL_WORLD_SCALE * 0.16).clamp(3.0, 8.0)
}

fn ca44a_procedural_content_art_handle(
    handles: &GraphicalAlphaArtHandles,
    candidate: &ProceduralWorldContentCandidate,
) -> (Handle<Image>, &'static str) {
    match candidate.kind {
        ProceduralWorldContentKind::Food => (handles.food_bloom.clone(), "food"),
        ProceduralWorldContentKind::Hazard => (handles.hazard.clone(), "hazard"),
        ProceduralWorldContentKind::Obstacle => (handles.rock_obstacle.clone(), "rock-obstacle"),
        ProceduralWorldContentKind::DressingProp => {
            ca44a_prop_art_for_material(handles, candidate.material.material_id(), &candidate.label)
        }
    }
}

fn ca44a_procedural_content_sprite_size(kind: ProceduralWorldContentKind) -> Vec2 {
    match kind {
        ProceduralWorldContentKind::Food => Vec2::splat(6.0),
        ProceduralWorldContentKind::Hazard => Vec2::splat(8.0),
        ProceduralWorldContentKind::Obstacle => Vec2::splat(8.0),
        ProceduralWorldContentKind::DressingProp => Vec2::splat(4.5),
    }
}

fn ca44a_procedural_content_z(kind: ProceduralWorldContentKind) -> f32 {
    match kind {
        ProceduralWorldContentKind::DressingProp => -0.18,
        ProceduralWorldContentKind::Food => 0.06,
        ProceduralWorldContentKind::Hazard => 0.08,
        ProceduralWorldContentKind::Obstacle => 0.04,
    }
}

fn ca44a_procedural_content_fallback_color(kind: ProceduralWorldContentKind) -> Color {
    match kind {
        ProceduralWorldContentKind::Food => Color::srgba(0.25, 0.96, 0.30, 0.70),
        ProceduralWorldContentKind::Hazard => Color::srgba(1.0, 0.14, 0.12, 0.76),
        ProceduralWorldContentKind::Obstacle => Color::srgba(0.60, 0.56, 0.50, 0.74),
        ProceduralWorldContentKind::DressingProp => Color::srgba(0.48, 0.78, 0.28, 0.50),
    }
}

fn ca37_seeded_prop_variant(prop_id: &str) -> u32 {
    prop_id.bytes().fold(0_u32, |acc, value| {
        acc.wrapping_mul(33).wrapping_add(value as u32)
    })
}

fn ca44a_procedural_terrain_sample(seed: u64, ix: i32, iz: i32) -> ProceduralTerrainSample {
    let config = ProceduralWorldConfig::with_seed(seed);
    sample_procedural_terrain_tile(config, ProceduralTileCoord::new(ix, iz)).unwrap_or_else(|_| {
        ProceduralTerrainSample {
            tile: ProceduralTileCoord::new(0, 0),
            chunk: ProceduralChunkCoord::new(0, 0),
            biome: alife_world::ProceduralBiomeKind::SafeGrass,
            material: alife_world::ProceduralTerrainMaterial::SafeGrass,
            resource_bias: 0.24,
            hazard_pressure: 0.04,
            roughness: 0.20,
            traversal_cost: 0.18,
        }
    })
}

fn ca37_seeded_terrain_hash(seed: u64, ix: i32, iz: i32) -> i32 {
    let seed_part = (seed as i64 % 65_521) as i32;
    (ix * 73 + iz * 151 + ix * iz * 17 + seed_part).rem_euclid(193)
}

fn ca37_terrain_tile_color(seed: u64, material_id: &str, ix: i32, iz: i32) -> Color {
    let shade = ((ca37_seeded_terrain_hash(seed, ix, iz) % 7) as f32) * 0.008;
    match material_id {
        "neutral-soil" => Color::srgba(
            0.39 + shade,
            0.29 + shade,
            0.17,
            ca37_terrain_tile_alpha(material_id),
        ),
        "resource-grove" => Color::srgba(
            0.14 + shade,
            0.48 + shade,
            0.18,
            ca37_terrain_tile_alpha(material_id),
        ),
        "hazard-pressure" => Color::srgba(
            0.58 + shade,
            0.20,
            0.15,
            ca37_terrain_tile_alpha(material_id),
        ),
        "stone-dressing" => Color::srgba(
            0.31 + shade,
            0.34 + shade,
            0.27,
            ca37_terrain_tile_alpha(material_id),
        ),
        _ => Color::srgba(
            0.17 + shade,
            0.39 + shade,
            0.18,
            ca37_terrain_tile_alpha(material_id),
        ),
    }
}

fn ca37_terrain_tile_alpha(material_id: &str) -> f32 {
    match material_id {
        "hazard-pressure" => 0.12,
        "resource-grove" => 0.11,
        "stone-dressing" => 0.10,
        "neutral-soil" => 0.10,
        _ => 0.09,
    }
}

fn ca37_world_art_prop_color(material_id: &str) -> Color {
    match material_id {
        "neutral-soil" => Color::srgba(0.48, 0.34, 0.20, 0.26),
        "resource-grove" => Color::srgba(0.30, 0.86, 0.36, 0.24),
        "hazard-pressure" => Color::srgba(1.0, 0.18, 0.14, 0.30),
        "stone-dressing" => Color::srgba(0.64, 0.62, 0.56, 0.42),
        "school-accent" => Color::srgba(0.58, 0.38, 0.92, 0.28),
        _ => Color::srgba(0.22, 0.50, 0.24, 0.20),
    }
}

pub fn ca37_world_art_overlay_text(summary: &Ca37WorldArtStyleSummary) -> String {
    summary.compact_overlay_text()
}

fn spawn_graphical_intent_feedback(app: &mut App, view_mode: GraphicalPlaygroundViewMode) {
    let visibility = view_mode_visibility(view_mode.world_labels_visible());
    app.world_mut().spawn((
        Name::new("A-Life CA03 stable-ID intent line"),
        Sprite {
            color: Color::srgba(0.42, 1.0, 0.58, 0.0),
            custom_size: Some(Vec2::new(1.0, 5.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.35),
        visibility,
        GraphicalIntentLine,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA03 selected action badge"),
        Text2d::new("action: idle"),
        TextFont {
            font_size: 10.0,
            ..default()
        },
        TextColor(Color::srgb(0.98, 0.96, 0.72)),
        Transform::from_xyz(0.0, 56.0, 1.15),
        visibility,
        GraphicalActionBadge,
    ));
}

fn spawn_ca08_feedback_pulses(
    app: &mut App,
    presentation: &VisibleWorldPresentation,
    view_mode: GraphicalPlaygroundViewMode,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
) {
    for pulse in ca08_pulse_targets_for_presentation(presentation) {
        let Some(target_id) = pulse.target_stable_id else {
            continue;
        };
        let Some(target) = presentation
            .objects
            .iter()
            .find(|object| object.stable_id == target_id)
        else {
            continue;
        };
        let mut sprite = Sprite {
            color: ca08_pulse_color(pulse.kind, false),
            custom_size: Some(ca08_pulse_size(pulse.kind, view_mode)),
            ..default()
        };
        let mut role = None;
        if view_mode == GraphicalPlaygroundViewMode::Player {
            if let Some(handles) = alpha_art {
                let (image, art_role) = ca08_pulse_art_handle(handles, pulse.kind);
                sprite.image = image;
                role = Some(art_role);
            }
        }
        let mut entity = app.world_mut().spawn((
            Name::new(format!(
                "A-Life CA08 {} pulse stable:{}",
                pulse.kind.label(),
                target.stable_id.raw()
            )),
            sprite,
            Transform::from_translation(graphical_position(target) + Vec3::new(0.0, 0.0, 0.28)),
            pulse,
        ));
        if let Some(role) = role {
            entity.insert((
                GraphicalAlphaArtBackedSprite {
                    role,
                    stable_id: Some(target.stable_id),
                },
                GraphicalProductionArtLayer {
                    role: "feedback-pulse",
                    display_only: true,
                },
            ));
        }
    }
}

pub fn ca08_pulse_targets_for_presentation(
    presentation: &VisibleWorldPresentation,
) -> Vec<GraphicalSensoryCuePulse> {
    let agent = presentation
        .objects
        .iter()
        .find(|object| object.kind == WorldObjectKind::Agent);
    let food = presentation
        .objects
        .iter()
        .find(|object| object.kind == WorldObjectKind::Food);
    let hazard = presentation
        .objects
        .iter()
        .find(|object| object.kind == WorldObjectKind::Hazard);

    [
        (Ca08SensoryCueKind::Reward, food.or(agent)),
        (Ca08SensoryCueKind::Pain, hazard.or(agent)),
        (Ca08SensoryCueKind::Sleep, agent),
        (Ca08SensoryCueKind::Learning, agent),
    ]
    .into_iter()
    .filter_map(|(kind, target)| {
        target.map(|target| GraphicalSensoryCuePulse {
            kind,
            target_stable_id: Some(target.stable_id),
        })
    })
    .collect()
}

fn ca08_pulse_size(kind: Ca08SensoryCueKind, view_mode: GraphicalPlaygroundViewMode) -> Vec2 {
    if view_mode == GraphicalPlaygroundViewMode::Player {
        return match kind {
            Ca08SensoryCueKind::Reward => Vec2::splat(3.5),
            Ca08SensoryCueKind::Pain => Vec2::splat(4.5),
            Ca08SensoryCueKind::Sleep => Vec2::new(5.0, 3.4),
            Ca08SensoryCueKind::Learning => Vec2::splat(5.5),
        };
    }
    match kind {
        Ca08SensoryCueKind::Reward => Vec2::new(74.0, 74.0),
        Ca08SensoryCueKind::Pain => Vec2::new(86.0, 86.0),
        Ca08SensoryCueKind::Sleep => Vec2::new(102.0, 64.0),
        Ca08SensoryCueKind::Learning => Vec2::new(118.0, 74.0),
    }
}

fn ca08_pulse_art_handle(
    handles: &GraphicalAlphaArtHandles,
    kind: Ca08SensoryCueKind,
) -> (Handle<Image>, &'static str) {
    match kind {
        Ca08SensoryCueKind::Reward => (handles.ambient_light_pool.clone(), "feedback-reward"),
        Ca08SensoryCueKind::Pain => (handles.hazard_glow.clone(), "feedback-pain"),
        Ca08SensoryCueKind::Sleep => (handles.ambient_canopy_shadow.clone(), "feedback-sleep"),
        Ca08SensoryCueKind::Learning => (handles.selection_pulse.clone(), "feedback-learning"),
    }
}

fn spawn_graphical_object(
    app: &mut App,
    object: &VisibleWorldObjectPresentation,
    view_mode: GraphicalPlaygroundViewMode,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
) -> Result<(), GameAppShellError> {
    let material = object.material;
    let marker_position = graphical_position(object);
    let alpha_art_role = ca44a_object_art_role(object.kind);
    if view_mode == GraphicalPlaygroundViewMode::Player {
        if let Some(handles) = alpha_art {
            app.world_mut().spawn((
                Name::new(format!(
                    "A-Life production entity shadow stable:{}",
                    object.stable_id.raw()
                )),
                Sprite {
                    image: handles.entity_shadow.clone(),
                    color: Color::srgba(1.0, 1.0, 1.0, ca44a_entity_shadow_alpha(object.kind)),
                    custom_size: Some(ca44a_entity_shadow_size(object)),
                    ..default()
                },
                Transform::from_translation(marker_position + Vec3::new(0.0, -2.0, -0.08)),
                GraphicalAlphaArtBackedSprite {
                    role: "entity-shadow",
                    stable_id: Some(object.stable_id),
                },
                GraphicalProductionArtLayer {
                    role: "entity-shadow",
                    display_only: true,
                },
            ));
        }
    }
    let sprite = if view_mode == GraphicalPlaygroundViewMode::Player {
        if let Some(handles) = alpha_art {
            Sprite {
                image: ca44a_object_art_handle(handles, object.kind),
                color: Color::WHITE,
                custom_size: Some(ca44a_player_sprite_size(object)),
                ..default()
            }
        } else {
            Sprite {
                color: rgba_to_color(material.rgba()),
                custom_size: Some(graphical_size(object)),
                ..default()
            }
        }
    } else {
        Sprite {
            color: rgba_to_color(material.rgba()),
            custom_size: Some(graphical_size(object)),
            ..default()
        }
    };
    let entity = app
        .world_mut()
        .spawn((
            Name::new(format!(
                "A-Life {:?} stable:{} {}",
                object.kind,
                object.stable_id.raw(),
                object.label
            )),
            sprite,
            Transform::from_translation(marker_position),
            VisibleWorldObject {
                stable_id: object.stable_id,
                kind: object.kind,
                shape: object.shape,
                material,
                rgba: material.rgba(),
            },
            VisibleWorldDebugLabel(object.debug_label.clone()),
            GraphicalPlaygroundMarker {
                stable_id: object.stable_id,
                kind: object.kind,
            },
        ))
        .id();
    if view_mode == GraphicalPlaygroundViewMode::Player {
        if alpha_art.is_some() {
            app.world_mut()
                .entity_mut(entity)
                .insert(GraphicalAlphaArtBackedSprite {
                    role: alpha_art_role,
                    stable_id: Some(object.stable_id),
                });
        } else {
            app.world_mut()
                .entity_mut(entity)
                .insert(GraphicalAlphaArtFallbackSprite {
                    role: alpha_art_role,
                    reason: "alpha art handles unavailable",
                });
        }
    }
    {
        let mut entity_mut = app.world_mut().entity_mut(entity);
        match object.kind {
            WorldObjectKind::Agent => {
                if let Some(organism_id) = object.organism_id {
                    entity_mut.insert(CreatureBody::new(organism_id, object.stable_id)?);
                }
                let pose = crate::ca38_creature_pose_for_state(
                    CreatureAnimationState::Idle,
                    CreatureExpressionState::Neutral,
                );
                entity_mut.insert(GraphicalCreatureAnimationPose {
                    stable_id: object.stable_id,
                    pose_id: pose.pose_id,
                    action_label: pose.action_label,
                    display_only: true,
                });
            }
            WorldObjectKind::Food => {
                entity_mut.insert(AffordanceTags::food(object.nutrition));
            }
            WorldObjectKind::Hazard => {
                entity_mut.insert(AffordanceTags::hazard(object.hazard_pain));
            }
            WorldObjectKind::Obstacle => {
                entity_mut.insert(AffordanceTags {
                    bits: AffordanceBits::RESOURCE,
                    nutrition: 0.0,
                    hazard_pain: 0.0,
                    blocks_movement: true,
                });
            }
            WorldObjectKind::Token => {
                entity_mut.insert(SensoryEmitter {
                    audible_token: object.token_id,
                    ..SensoryEmitter::default()
                });
            }
        }
    }
    app.world_mut()
        .resource_mut::<BevyEntityMap>()
        .bind(entity, object.stable_id)?;
    if view_mode != GraphicalPlaygroundViewMode::Player || alpha_art.is_none() {
        spawn_graphical_object_glyphs(app, object, marker_position);
    }

    app.world_mut().spawn((
        Name::new(format!("A-Life label stable:{}", object.stable_id.raw())),
        Text2d::new(graphical_object_badge_text(object)),
        TextFont {
            font_size: 11.0,
            ..default()
        },
        TextColor(readability_label_color(object.kind)),
        Transform::from_translation(marker_position + graphical_badge_offset(object.kind)),
        view_mode_visibility(view_mode.world_labels_visible()),
        GraphicalObjectBadge {
            stable_id: object.stable_id,
            kind: object.kind,
        },
    ));
    Ok(())
}

fn ca44a_entity_shadow_size(object: &VisibleWorldObjectPresentation) -> Vec2 {
    let base = ca44a_player_sprite_size(object);
    match object.kind {
        WorldObjectKind::Agent => Vec2::new(base.x * 0.88, base.y * 0.36),
        WorldObjectKind::Food => Vec2::new(base.x * 0.72, base.y * 0.28),
        WorldObjectKind::Hazard => Vec2::new(base.x * 0.92, base.y * 0.34),
        WorldObjectKind::Obstacle => Vec2::new(base.x * 0.96, base.y * 0.34),
        WorldObjectKind::Token => Vec2::new(base.x * 0.74, base.y * 0.28),
    }
}

fn ca44a_player_sprite_size(object: &VisibleWorldObjectPresentation) -> Vec2 {
    match object.kind {
        WorldObjectKind::Agent => Vec2::new(9.0, 8.0),
        WorldObjectKind::Food => Vec2::splat(6.0),
        WorldObjectKind::Hazard => Vec2::splat(9.0),
        WorldObjectKind::Obstacle => Vec2::splat(9.0),
        WorldObjectKind::Token => Vec2::new(5.5, 4.5),
    }
}

fn ca44a_entity_shadow_alpha(kind: WorldObjectKind) -> f32 {
    match kind {
        WorldObjectKind::Agent => 0.24,
        WorldObjectKind::Hazard | WorldObjectKind::Obstacle => 0.22,
        WorldObjectKind::Food | WorldObjectKind::Token => 0.16,
    }
}

fn ca44a_object_art_role(kind: WorldObjectKind) -> &'static str {
    match kind {
        WorldObjectKind::Agent => "creature-idle",
        WorldObjectKind::Food => "food",
        WorldObjectKind::Hazard => "hazard",
        WorldObjectKind::Obstacle => "rock-obstacle",
        WorldObjectKind::Token => "prop-dressing",
    }
}

pub fn ca44a_creature_art_role_for_pose(pose: crate::Ca38CreaturePose) -> &'static str {
    match pose.pose_id {
        "pain-flinch" | "flee-alert" => "creature-hurt",
        "move-lean" => "creature-moving",
        "eat-reach" => "creature-eat",
        "sleep-curl" | "rest-low" => "creature-sleep",
        "social-signal" | "inspect-focus" | "curious-tilt" => "creature-signal",
        _ => "creature-idle",
    }
}

fn ca44a_object_art_handle(
    handles: &GraphicalAlphaArtHandles,
    kind: WorldObjectKind,
) -> Handle<Image> {
    match kind {
        WorldObjectKind::Agent => handles.creature_idle.clone(),
        WorldObjectKind::Food => handles.food.clone(),
        WorldObjectKind::Hazard => handles.hazard.clone(),
        WorldObjectKind::Obstacle => handles.rock_obstacle.clone(),
        WorldObjectKind::Token => handles.prop_warning_shard.clone(),
    }
}

fn ca44a_creature_art_handle_for_pose(
    handles: &GraphicalAlphaArtHandles,
    pose: crate::Ca38CreaturePose,
) -> Handle<Image> {
    match ca44a_creature_art_role_for_pose(pose) {
        "creature-hurt" => handles.creature_hurt.clone(),
        "creature-moving" => handles.creature_moving.clone(),
        "creature-eat" => handles.creature_eat.clone(),
        "creature-sleep" => handles.creature_sleep.clone(),
        "creature-signal" => handles.creature_signal.clone(),
        _ => handles.creature_idle.clone(),
    }
}

fn spawn_graphical_object_glyphs(
    app: &mut App,
    object: &VisibleWorldObjectPresentation,
    marker_position: Vec3,
) {
    match object.kind {
        WorldObjectKind::Agent => {
            spawn_object_glyph_rect(
                app,
                object,
                marker_position + Vec3::new(0.0, 5.0, 0.95),
                Vec2::new(42.0, 18.0),
                Color::srgba(0.78, 0.98, 0.92, 0.72),
                0.0,
            );
            spawn_object_glyph_rect(
                app,
                object,
                marker_position + Vec3::new(14.0, 5.0, 1.0),
                Vec2::new(6.0, 6.0),
                Color::srgba(0.02, 0.08, 0.07, 0.88),
                0.0,
            );
        }
        WorldObjectKind::Food => {
            spawn_object_glyph_rect(
                app,
                object,
                marker_position + Vec3::new(0.0, 0.0, 0.94),
                Vec2::new(12.0, 42.0),
                Color::srgba(0.62, 1.0, 0.42, 0.82),
                0.0,
            );
            spawn_object_glyph_rect(
                app,
                object,
                marker_position + Vec3::new(0.0, 0.0, 0.95),
                Vec2::new(42.0, 12.0),
                Color::srgba(0.62, 1.0, 0.42, 0.82),
                0.0,
            );
        }
        WorldObjectKind::Hazard => {
            spawn_object_glyph_rect(
                app,
                object,
                marker_position + Vec3::new(0.0, 0.0, 0.96),
                Vec2::new(52.0, 52.0),
                Color::srgba(1.0, 0.08, 0.10, 0.78),
                45.0,
            );
            spawn_object_glyph_rect(
                app,
                object,
                marker_position + Vec3::new(0.0, 4.0, 1.02),
                Vec2::new(8.0, 30.0),
                Color::srgba(1.0, 0.92, 0.70, 0.92),
                0.0,
            );
            spawn_object_glyph_rect(
                app,
                object,
                marker_position + Vec3::new(0.0, -18.0, 1.02),
                Vec2::new(9.0, 9.0),
                Color::srgba(1.0, 0.92, 0.70, 0.92),
                0.0,
            );
        }
        WorldObjectKind::Obstacle => {
            for (x, y, size) in [(-15.0, -8.0, 22.0), (3.0, 6.0, 30.0), (24.0, -3.0, 18.0)] {
                spawn_object_glyph_rect(
                    app,
                    object,
                    marker_position + Vec3::new(x, y, 0.94),
                    Vec2::splat(size),
                    Color::srgba(0.72, 0.70, 0.64, 0.70),
                    12.0,
                );
            }
        }
        WorldObjectKind::Token => {
            spawn_object_glyph_rect(
                app,
                object,
                marker_position + Vec3::new(0.0, 0.0, 0.94),
                Vec2::new(36.0, 10.0),
                Color::srgba(0.94, 0.82, 1.0, 0.70),
                0.0,
            );
            spawn_object_glyph_rect(
                app,
                object,
                marker_position + Vec3::new(0.0, 0.0, 0.95),
                Vec2::new(10.0, 36.0),
                Color::srgba(0.94, 0.82, 1.0, 0.70),
                0.0,
            );
        }
    }
}

fn spawn_object_glyph_rect(
    app: &mut App,
    object: &VisibleWorldObjectPresentation,
    translation: Vec3,
    size: Vec2,
    color: Color,
    rotation_degrees: f32,
) {
    app.world_mut().spawn((
        Name::new(format!(
            "A-Life player glyph {:?} stable:{}",
            object.kind,
            object.stable_id.raw()
        )),
        Sprite {
            color,
            custom_size: Some(size),
            ..default()
        },
        Transform {
            translation,
            rotation: bevy::prelude::Quat::from_rotation_z(rotation_degrees.to_radians()),
            ..default()
        },
        GraphicalObjectGlyph {
            stable_id: object.stable_id,
            kind: object.kind,
        },
    ));
}

fn spawn_ca23_school_teacher_markers(
    app: &mut App,
    school: &Ca23GraphicalSchoolSummary,
    view_mode: GraphicalPlaygroundViewMode,
) {
    let visibility = view_mode_visibility(view_mode.teacher_debug_labels_visible());
    app.world_mut().spawn((
        Name::new(format!(
            "A-Life CA23 teacher cue stable:{}",
            school.teacher_avatar_stable_id.raw()
        )),
        Text2d::new(format!("[T] cue {}", school.teacher_avatar_stable_id.raw())),
        TextFont {
            font_size: 10.0,
            ..default()
        },
        TextColor(Color::srgb(0.82, 0.66, 1.0)),
        Transform::from_translation(Vec3::new(-300.0, 132.0, 1.08)),
        visibility,
        GraphicalTeacherCueMarker {
            stable_id: school.teacher_avatar_stable_id,
        },
    ));

    for (index, cue) in school.cue_markers.iter().take(4).enumerate() {
        let row = index as f32;
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life CA24 teacher world cue stable:{}",
                cue.stable_id.raw()
            )),
            Text2d::new(ca24_teacher_cue_marker_text(cue)),
            TextFont {
                font_size: 10.0,
                ..default()
            },
            TextColor(Color::srgb(0.92, 0.78, 1.0)),
            Transform::from_translation(Vec3::new(-210.0 + row * 116.0, 96.0 - row * 18.0, 1.08)),
            visibility,
            GraphicalTeacherCueMarker {
                stable_id: cue.stable_id,
            },
        ));
    }
}

fn ca24_teacher_cue_marker_text(cue: &Ca23TeacherCueMarker) -> String {
    let kind = match cue.channel {
        alife_core::TeacherPerceptionChannel::Hearing => "speech",
        alife_core::TeacherPerceptionChannel::Gesture => "gesture",
        alife_core::TeacherPerceptionChannel::Object => "object",
        alife_core::TeacherPerceptionChannel::Writing => "writing",
        alife_core::TeacherPerceptionChannel::Vision => "feedback",
    };
    format!("[T] {kind} {}", cue.stable_id.raw())
}

fn inspector_local_entity(
    app: &mut App,
    presentation: &VisibleWorldPresentation,
    stable_id: WorldEntityId,
) -> Result<Option<Entity>, GameAppShellError> {
    let local_entity = app
        .world()
        .resource::<BevyEntityMap>()
        .bevy_entity(stable_id);
    if let Some(entity) = local_entity {
        let selection = crate::select_visible_world_entity(presentation, stable_id)?;
        app.world_mut()
            .entity_mut(entity)
            .insert(SelectedVisibleEntity { selection });
        let selection_has_alpha_art = app.world().contains_resource::<GraphicalAlphaArtHandles>();
        let sprite = app
            .world()
            .get_resource::<GraphicalAlphaArtHandles>()
            .map_or(
                Sprite {
                    color: Color::srgba(1.0, 0.86, 0.25, 0.42),
                    custom_size: Some(Vec2::new(7.0, 5.5)),
                    ..default()
                },
                |handles| Sprite {
                    image: handles.selection_ring.clone(),
                    color: Color::WHITE,
                    custom_size: Some(Vec2::new(8.5, 6.5)),
                    ..default()
                },
            );
        app.world_mut().spawn((
            Name::new("A-Life S03 stable-ID selection ring"),
            sprite,
            Transform::from_xyz(0.0, 0.0, 0.5),
            if selection_has_alpha_art {
                GraphicalAlphaArtBackedSprite {
                    role: "selection-ring",
                    stable_id: Some(stable_id),
                }
            } else {
                GraphicalAlphaArtBackedSprite {
                    role: "selection-ring-fallback-marker",
                    stable_id: Some(stable_id),
                }
            },
            GraphicalSelectionRing,
        ));
        let selection_pulse = app
            .world()
            .get_resource::<GraphicalAlphaArtHandles>()
            .map(|handles| handles.selection_pulse.clone());
        if let Some(selection_pulse) = selection_pulse {
            app.world_mut().spawn((
                Name::new("A-Life CA44A selected creature pulse"),
                Sprite {
                    image: selection_pulse,
                    color: Color::srgba(1.0, 1.0, 1.0, 0.18),
                    custom_size: Some(Vec2::new(10.0, 7.5)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, 0.46),
                GraphicalAlphaArtBackedSprite {
                    role: "selection-pulse",
                    stable_id: Some(stable_id),
                },
                GraphicalSelectionRing,
            ));
        }
    }
    Ok(local_entity)
}

fn graphical_position(object: &VisibleWorldObjectPresentation) -> Vec3 {
    Vec3::new(
        object.position.x * GRAPHICAL_WORLD_SCALE,
        object.position.z * GRAPHICAL_WORLD_SCALE,
        0.0,
    )
}

fn graphical_size(object: &VisibleWorldObjectPresentation) -> Vec2 {
    match object.kind {
        WorldObjectKind::Agent => Vec2::new(78.0, 46.0),
        WorldObjectKind::Food => Vec2::splat(42.0),
        WorldObjectKind::Hazard => Vec2::new(52.0, 52.0),
        WorldObjectKind::Obstacle => Vec2::new(64.0, 64.0),
        WorldObjectKind::Token => Vec2::new(72.0, 36.0),
    }
}

fn rgba_to_color(rgba: [f32; 4]) -> Color {
    Color::srgba(rgba[0], rgba[1], rgba[2], rgba[3])
}

fn close_after_graphical_smoke_timeout(
    timer: Res<GraphicalPlaygroundSmokeTimer>,
    mut exits: MessageWriter<AppExit>,
) {
    if timer.started.elapsed() >= timer.duration {
        exits.write(AppExit::Success);
    }
}

fn handle_graphical_runtime_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut runtime: ResMut<GraphicalRuntimeControlsResource>,
    mut live_loop: NonSendMut<GraphicalRuntimeLoopResource>,
    mut gpu_telemetry: ResMut<GraphicalGpuTelemetryResource>,
    mut save_load: ResMut<GraphicalSaveLoadMenuResource>,
    mut exits: MessageWriter<AppExit>,
) {
    let apply = |runtime: &mut GraphicalRuntimeControlsResource,
                 live_loop: &mut GraphicalRuntimeLoopResource,
                 gpu_telemetry: &mut GraphicalGpuTelemetryResource,
                 command| {
        match apply_graphical_runtime_command(&mut runtime.panel, live_loop, command) {
            Ok(_) => {
                gpu_telemetry.telemetry = live_loop.gpu.telemetry().clone();
            }
            Err(err) => {
                runtime
                    .panel
                    .record_terminal_recovery(format!("runtime command failed: {err}"));
            }
        }
    };

    if keyboard.just_pressed(KeyCode::Escape) {
        apply(
            &mut *runtime,
            &mut *live_loop,
            &mut *gpu_telemetry,
            RuntimeControlCommand::RequestExit,
        );
        exits.write(AppExit::Success);
    }
    if keyboard.just_pressed(KeyCode::Space) {
        apply(
            &mut *runtime,
            &mut *live_loop,
            &mut *gpu_telemetry,
            RuntimeControlCommand::TogglePause,
        );
    }
    if keyboard.just_pressed(KeyCode::KeyN) {
        apply(
            &mut *runtime,
            &mut *live_loop,
            &mut *gpu_telemetry,
            RuntimeControlCommand::StepOnce,
        );
    }
    if keyboard.just_pressed(KeyCode::Digit1) {
        apply(
            &mut *runtime,
            &mut *live_loop,
            &mut *gpu_telemetry,
            RuntimeControlCommand::SetRunSpeed(1),
        );
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        apply(
            &mut *runtime,
            &mut *live_loop,
            &mut *gpu_telemetry,
            RuntimeControlCommand::SetRunSpeed(2),
        );
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        apply(
            &mut *runtime,
            &mut *live_loop,
            &mut *gpu_telemetry,
            RuntimeControlCommand::SetRunSpeed(3),
        );
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        if reset_graphical_runtime(&mut runtime, &mut live_loop, &mut gpu_telemetry).is_err() {
            runtime
                .panel
                .record_terminal_recovery("reset failed; close and relaunch");
        }
    }
    if keyboard.just_pressed(KeyCode::KeyM) {
        let result = save_load
            .session
            .apply_command(crate::GraphicalSaveLoadMenuCommand::ToggleMenu);
        runtime.panel.record_control_event(format!(
            "Save/load menu {}.",
            if save_load.session.is_open() {
                "opened"
            } else {
                "closed"
            }
        ));
        if !result.success {
            runtime
                .panel
                .record_control_event("Save/load menu toggle failed.".to_string());
        }
    }
    if keyboard.just_pressed(KeyCode::F5) {
        save_load.session.open();
        let result = save_load
            .session
            .apply_command(crate::GraphicalSaveLoadMenuCommand::ManualSave);
        runtime.panel.record_control_event(if result.success {
            "Manual save wrote Slot 1 with stable IDs.".to_string()
        } else {
            format!(
                "Manual save failed: {}.",
                result
                    .error
                    .as_ref()
                    .map(|error| error.code.as_str())
                    .unwrap_or("unknown")
            )
        });
    }
    if keyboard.just_pressed(KeyCode::F9) {
        save_load.session.open();
        let result = save_load
            .session
            .apply_command(crate::GraphicalSaveLoadMenuCommand::LoadManualSlot);
        if result.success {
            runtime.panel.record_control_event(
                "Manual load restored Slot 1; stable IDs remapped.".to_string(),
            );
            if reset_graphical_runtime(&mut runtime, &mut live_loop, &mut gpu_telemetry).is_err() {
                runtime
                    .panel
                    .record_terminal_recovery("load reset failed; close and relaunch");
            }
        } else {
            runtime.panel.record_control_event(format!(
                "Manual load failed without partial load: {}.",
                result
                    .error
                    .as_ref()
                    .map(|error| error.code.as_str())
                    .unwrap_or("unknown")
            ));
        }
    }
}

fn handle_graphical_camera_selection_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera: ResMut<CameraNavigationResource>,
    selection: Res<SelectionResource>,
) {
    let mut next = camera.state;

    if keyboard.any_just_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
        next = next.pan_by(-0.2, 0.0).unwrap_or(next);
    }
    if keyboard.any_just_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
        next = next.pan_by(0.2, 0.0).unwrap_or(next);
    }
    if keyboard.any_just_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
        next = next.pan_by(0.0, 0.2).unwrap_or(next);
    }
    if keyboard.any_just_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
        next = next.pan_by(0.0, -0.2).unwrap_or(next);
    }
    if keyboard.just_pressed(KeyCode::KeyQ) {
        next = next.orbit_by(-15.0).unwrap_or(next);
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        next = next.orbit_by(15.0).unwrap_or(next);
    }
    if keyboard.just_pressed(KeyCode::Equal) {
        next = next.zoom_by(0.25).unwrap_or(next);
    }
    if keyboard.just_pressed(KeyCode::Minus) {
        next = next.zoom_by(-0.25).unwrap_or(next);
    }
    if keyboard.just_pressed(KeyCode::KeyF) {
        next = next.with_follow_target(selection.stable_id).unwrap_or(next);
    }

    camera.state = next;
}

fn handle_graphical_mouse_selection(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: bevy::prelude::Query<&Window, With<PrimaryWindow>>,
    cameras: bevy::prelude::Query<(&Camera, &GlobalTransform), With<GraphicalMainCamera>>,
    markers: bevy::prelude::Query<(Entity, &GraphicalPlaygroundMarker, &Transform)>,
    map: Res<BevyEntityMap>,
    presentation: Res<GraphicalVisibleWorldPresentationResource>,
    mut selection: ResMut<SelectionResource>,
    mut inspector: ResMut<CreatureInspectorResource>,
    mut camera_state: ResMut<CameraNavigationResource>,
    mut runtime: ResMut<GraphicalRuntimeControlsResource>,
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
    let Ok(world_position) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };

    let marker_data = markers.iter().filter_map(|(entity, marker, transform)| {
        if map.bevy_entity(marker.stable_id) == Some(entity) {
            Some((marker.stable_id, marker.kind, transform.translation))
        } else {
            None
        }
    });

    let Some(stable_id) = ca06_pick_stable_id_from_world_point(world_position, marker_data) else {
        return;
    };
    let local_entity = map.bevy_entity(stable_id);

    if apply_graphical_stable_selection(
        &presentation.presentation,
        stable_id,
        local_entity,
        &mut selection,
        &mut inspector,
        &mut camera_state,
        &mut runtime,
    )
    .is_err()
    {
        runtime
            .panel
            .record_terminal_recovery("mouse selection failed; stable ID not selectable");
    }
}

fn handle_graphical_population_cycle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    map: Res<BevyEntityMap>,
    presentation: Res<GraphicalVisibleWorldPresentationResource>,
    mut selection: ResMut<SelectionResource>,
    mut inspector: ResMut<CreatureInspectorResource>,
    mut camera_state: ResMut<CameraNavigationResource>,
    mut runtime: ResMut<GraphicalRuntimeControlsResource>,
) {
    if !keyboard.just_pressed(KeyCode::Tab) {
        return;
    }
    let Some(next_stable_id) =
        ca18_cycle_selected_creature(&presentation.presentation, selection.stable_id)
    else {
        runtime
            .panel
            .record_control_event("No next creature available for Tab cycle.");
        return;
    };
    let local_entity = map.bevy_entity(next_stable_id);
    if apply_graphical_stable_selection(
        &presentation.presentation,
        next_stable_id,
        local_entity,
        &mut selection,
        &mut inspector,
        &mut camera_state,
        &mut runtime,
    )
    .is_ok()
    {
        runtime.panel.record_control_event(format!(
            "Cycled selected creature to stable:{}.",
            next_stable_id.raw()
        ));
    }
}

pub fn ca06_marker_hit_radius(kind: WorldObjectKind) -> f32 {
    match kind {
        WorldObjectKind::Agent => 54.0,
        WorldObjectKind::Food => 42.0,
        WorldObjectKind::Hazard => 48.0,
        WorldObjectKind::Obstacle => 50.0,
        WorldObjectKind::Token => 44.0,
    }
}

pub fn ca06_pick_stable_id_from_world_point(
    world_point: Vec2,
    markers: impl IntoIterator<Item = (WorldEntityId, WorldObjectKind, Vec3)>,
) -> Option<WorldEntityId> {
    markers
        .into_iter()
        .filter_map(|(stable_id, kind, translation)| {
            let delta = Vec2::new(translation.x, translation.y) - world_point;
            let distance_squared = delta.length_squared();
            let radius = ca06_marker_hit_radius(kind);
            if distance_squared <= radius * radius {
                Some((stable_id, distance_squared))
            } else {
                None
            }
        })
        .min_by(|left, right| {
            left.1
                .partial_cmp(&right.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(stable_id, _)| stable_id)
}

pub fn apply_graphical_stable_selection(
    presentation: &VisibleWorldPresentation,
    stable_id: WorldEntityId,
    local_entity: Option<Entity>,
    selection: &mut SelectionResource,
    inspector: &mut CreatureInspectorResource,
    camera: &mut CameraNavigationResource,
    runtime: &mut GraphicalRuntimeControlsResource,
) -> Result<(), GameAppShellError> {
    let selection_snapshot = crate::select_visible_world_entity(presentation, stable_id)?;
    selection.stable_id = stable_id;
    selection.local_entity = local_entity;
    camera.state = camera.state.focus_on(selection_snapshot.position)?;
    inspector.snapshot.selection = selection_snapshot;
    runtime
        .panel
        .record_control_event(format!("Mouse selected stable:{}.", stable_id.raw()));
    runtime.panel.validate()?;
    Ok(())
}

fn advance_graphical_runtime_loop(
    time: Res<Time>,
    mut runtime: ResMut<GraphicalRuntimeControlsResource>,
    mut live_loop: NonSendMut<GraphicalRuntimeLoopResource>,
    mut gpu_telemetry: ResMut<GraphicalGpuTelemetryResource>,
) {
    if let Some(target) = runtime.smoke_target_ticks {
        if runtime.smoke_ticks_done < target {
            let delta_seconds = time.delta_secs();
            let speed = runtime.panel.run_speed_ticks;
            let plan = match runtime.panel.scheduler.observe_render_frame(
                delta_seconds,
                RuntimePlaybackState::Running,
                speed,
            ) {
                Ok(plan) => plan,
                Err(err) => {
                    runtime
                        .panel
                        .record_terminal_recovery(format!("scheduler failed: {err}"));
                    return;
                }
            };
            let remaining = target.saturating_sub(runtime.smoke_ticks_done);
            let ticks = plan.ticks_to_run.min(remaining);
            if ticks == 0 {
                return;
            }
            match apply_graphical_runtime_command(
                &mut runtime.panel,
                &mut live_loop,
                RuntimeControlCommand::RunForTicks(ticks),
            ) {
                Ok(summaries) => {
                    runtime.smoke_ticks_done = runtime
                        .smoke_ticks_done
                        .saturating_add(summaries.len() as u32);
                    gpu_telemetry.telemetry = live_loop.gpu.telemetry().clone();
                }
                Err(err) => {
                    runtime
                        .panel
                        .record_terminal_recovery(format!("runtime smoke tick failed: {err}"));
                }
            }
        }
        return;
    }

    let playback = runtime.panel.playback;
    let speed = runtime.panel.run_speed_ticks;
    let plan =
        match runtime
            .panel
            .scheduler
            .observe_render_frame(time.delta_secs(), playback, speed)
        {
            Ok(plan) => plan,
            Err(err) => {
                runtime
                    .panel
                    .record_terminal_recovery(format!("scheduler failed: {err}"));
                return;
            }
        };
    if runtime.panel.playback == RuntimePlaybackState::Running && plan.ticks_to_run > 0 {
        match apply_graphical_runtime_command(
            &mut runtime.panel,
            &mut live_loop,
            RuntimeControlCommand::RunForTicks(plan.ticks_to_run),
        ) {
            Ok(_) => {
                gpu_telemetry.telemetry = live_loop.gpu.telemetry().clone();
            }
            Err(err) => {
                runtime
                    .panel
                    .record_terminal_recovery(format!("runtime tick failed: {err}"));
            }
        }
    }
}

fn apply_graphical_runtime_command(
    panel: &mut RuntimeControlPanel,
    live_loop: &mut GraphicalRuntimeLoopResource,
    command: RuntimeControlCommand,
) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
    match command {
        RuntimeControlCommand::TogglePause => {
            panel.playback = match panel.playback {
                RuntimePlaybackState::Paused => RuntimePlaybackState::Running,
                RuntimePlaybackState::Running => RuntimePlaybackState::Paused,
                RuntimePlaybackState::ShutdownRequested => RuntimePlaybackState::ShutdownRequested,
            };
            panel.record_control_event(format!("Playback changed to {}.", panel.playback.label()));
            panel.validate()?;
            Ok(Vec::new())
        }
        RuntimeControlCommand::StepOnce => {
            panel.playback = RuntimePlaybackState::Paused;
            let (summary, motor_ring) = live_loop.gpu.tick_with_motor_ring(&mut live_loop.live)?;
            panel.record_motor_ring(motor_ring)?;
            panel.record_tick(&summary);
            panel.record_topology_overlay(&live_loop.live, std::slice::from_ref(&summary))?;
            panel.record_memory_journal(&live_loop.live, std::slice::from_ref(&summary))?;
            panel.record_neural_profiler(
                &live_loop.live,
                std::slice::from_ref(&summary),
                Some(live_loop.gpu.telemetry()),
            )?;
            panel
                .scheduler
                .record_executed_ticks(crate::executed_live_tick_count(std::slice::from_ref(
                    &summary,
                )))?;
            panel.mind_tick = live_loop.live.mind().current_tick().raw();
            panel.validate()?;
            Ok(vec![summary])
        }
        RuntimeControlCommand::SetRunSpeed(speed) => {
            panel.run_speed_ticks = speed.clamp(1, crate::S02_MAX_RUN_TICKS_PER_UPDATE);
            panel.playback = RuntimePlaybackState::Running;
            panel.record_control_event(format!("Run speed set to {}x.", panel.run_speed_ticks));
            panel.validate()?;
            Ok(Vec::new())
        }
        RuntimeControlCommand::RunForTicks(ticks) => {
            panel.playback = RuntimePlaybackState::Running;
            let bounded = ticks.min(S02_MAX_SMOKE_TICKS);
            let mut summaries = Vec::with_capacity(bounded as usize);
            for _ in 0..bounded {
                let (summary, motor_ring) =
                    live_loop.gpu.tick_with_motor_ring(&mut live_loop.live)?;
                panel.record_motor_ring(motor_ring)?;
                panel.record_tick(&summary);
                summaries.push(summary);
            }
            panel.record_topology_overlay(&live_loop.live, &summaries)?;
            panel.record_memory_journal(&live_loop.live, &summaries)?;
            panel.record_neural_profiler(
                &live_loop.live,
                &summaries,
                Some(live_loop.gpu.telemetry()),
            )?;
            panel
                .scheduler
                .record_executed_ticks(crate::executed_live_tick_count(&summaries))?;
            panel.mind_tick = live_loop.live.mind().current_tick().raw();
            panel.validate()?;
            Ok(summaries)
        }
        RuntimeControlCommand::RestartAlphaFixture => {
            live_loop.live = LiveBrainLoop::from_p34_launch(&live_loop.launch)?;
            live_loop.gpu = GraphicalGpuRuntimeController::new(live_loop.gpu.mode());
            panel.reset_to_alpha_fixture(&live_loop.live);
            panel.validate()?;
            Ok(Vec::new())
        }
        RuntimeControlCommand::RequestExit => {
            panel.playback = RuntimePlaybackState::ShutdownRequested;
            panel.record_control_event("Exit requested from graphical controls.");
            panel.validate()?;
            Ok(Vec::new())
        }
    }
}

fn reset_graphical_runtime(
    runtime: &mut GraphicalRuntimeControlsResource,
    live_loop: &mut GraphicalRuntimeLoopResource,
    gpu_telemetry: &mut GraphicalGpuTelemetryResource,
) -> Result<(), GameAppShellError> {
    live_loop.live = LiveBrainLoop::from_p34_launch(&live_loop.launch)?;
    live_loop.gpu = GraphicalGpuRuntimeController::new(live_loop.gpu.mode());
    runtime.panel.reset_to_alpha_fixture(&live_loop.live);
    runtime.smoke_ticks_done = 0;
    gpu_telemetry.telemetry = live_loop.gpu.telemetry().clone();
    runtime.panel.validate()?;
    Ok(())
}

fn update_graphical_camera_transform(
    camera: Res<CameraNavigationResource>,
    mut cameras: bevy::prelude::Query<&mut Transform, With<GraphicalMainCamera>>,
) {
    for mut transform in &mut cameras {
        transform.translation.x = camera.state.focus.x * GRAPHICAL_WORLD_SCALE;
        transform.translation.y = camera.state.focus.z * GRAPHICAL_WORLD_SCALE;
        transform.rotation =
            bevy::prelude::Quat::from_rotation_z(camera.state.yaw_degrees.to_radians());
        let zoom_scale = (1.0 / camera.state.zoom).clamp(0.125, 4.0);
        transform.scale = Vec3::splat(zoom_scale);
    }
}

fn update_graphical_selection_ring(
    inspector: Res<CreatureInspectorResource>,
    mut ring_query: bevy::prelude::Query<&mut Transform, With<GraphicalSelectionRing>>,
) {
    let selected_position = inspector.snapshot.selection.position;
    let ring_position = Vec3::new(
        selected_position.x * GRAPHICAL_WORLD_SCALE,
        selected_position.z * GRAPHICAL_WORLD_SCALE,
        0.2,
    );
    for mut ring in &mut ring_query {
        ring.translation = ring_position;
    }
}

fn update_graphical_runtime_overlay(
    runtime: Res<GraphicalRuntimeControlsResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    view_mode: Res<GraphicalViewModeResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<RuntimeStatusOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = if view_mode.mode == GraphicalPlaygroundViewMode::Player {
            graphical_player_status_overlay_text(&runtime.panel, &gpu.telemetry)
        } else {
            graphical_full_debug_status_overlay_text(&runtime.panel, &gpu.telemetry)
        };
    }
}

fn update_graphical_inspector_overlay(
    runtime: Res<GraphicalRuntimeControlsResource>,
    camera: Res<CameraNavigationResource>,
    selection: Res<SelectionResource>,
    inspector: Res<CreatureInspectorResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    view_mode: Res<GraphicalViewModeResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<InspectorStatusOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = if view_mode.mode == GraphicalPlaygroundViewMode::Player {
            graphical_player_inspector_overlay_text(&selection, &inspector, &gpu)
        } else {
            graphical_inspector_overlay_text(&runtime, &camera, &selection, &inspector, &gpu)
        };
    }
}

fn update_graphical_gpu_visual_cues(
    runtime: Res<GraphicalRuntimeControlsResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    alpha_art: Option<Res<GraphicalAlphaArtHandles>>,
    mut markers: bevy::prelude::Query<(
        &GraphicalPlaygroundMarker,
        &mut Sprite,
        &mut Transform,
        Option<&mut GraphicalCreatureAnimationPose>,
        Option<&GraphicalAlphaArtBackedSprite>,
    )>,
) {
    let agent_color = if gpu.telemetry.fallback_reason.is_some() {
        Color::srgb(0.78, 0.78, 0.72)
    } else if gpu.telemetry.h_shadow_applications > 0 {
        Color::srgb(0.42, 1.0, 0.72)
    } else if gpu.telemetry.gpu_scores_used_for_proposals && gpu.telemetry.cpu_shadow_parity {
        Color::srgb(0.35, 0.86, 1.0)
    } else {
        Color::srgb(1.0, 0.88, 0.35)
    };
    let target = runtime.panel.target_entity.map(WorldEntityId);
    for (marker, mut sprite, mut transform, pose_component, art_backed) in &mut markers {
        let uses_alpha_art = art_backed.is_some();
        match marker.kind {
            WorldObjectKind::Agent => {
                let pose = ca38_pose_from_runtime(&runtime.panel, &gpu.telemetry);
                if uses_alpha_art {
                    if let Some(handles) = alpha_art.as_deref() {
                        sprite.image = ca44a_creature_art_handle_for_pose(handles, pose);
                    }
                    sprite.color = if gpu.telemetry.fallback_reason.is_some() {
                        Color::srgba(0.72, 0.72, 0.68, 1.0)
                    } else {
                        Color::WHITE
                    };
                } else {
                    sprite.color = agent_color;
                }
                sprite.custom_size = Some(ca38_graphical_creature_size(pose));
                transform.scale = Vec3::splat(ca38_graphical_creature_scale(pose, &gpu.telemetry));
                transform.rotation =
                    bevy::prelude::Quat::from_rotation_z(pose.rotation_degrees.to_radians());
                if let Some(mut pose_component) = pose_component {
                    pose_component.pose_id = pose.pose_id;
                    pose_component.action_label = pose.action_label;
                    pose_component.display_only = pose.display_only;
                }
            }
            WorldObjectKind::Food if target == Some(marker.stable_id) => {
                if uses_alpha_art {
                    if let Some(handles) = alpha_art.as_deref() {
                        sprite.image = handles.food_bloom.clone();
                    }
                    sprite.color = Color::WHITE;
                } else {
                    sprite.color = Color::srgb(1.0, 0.95, 0.28);
                }
                transform.scale = Vec3::splat(1.14);
            }
            WorldObjectKind::Food => {
                if uses_alpha_art {
                    if let Some(handles) = alpha_art.as_deref() {
                        sprite.image = handles.food.clone();
                    }
                    sprite.color = Color::WHITE;
                } else {
                    sprite.color = Color::srgb(0.62, 1.0, 0.42);
                }
                transform.scale = Vec3::splat(1.0);
            }
            WorldObjectKind::Hazard => {
                if uses_alpha_art {
                    if let Some(handles) = alpha_art.as_deref() {
                        sprite.image = handles.hazard_glow.clone();
                    }
                    sprite.color = Color::WHITE;
                } else {
                    sprite.color = Color::srgb(1.0, 0.16, 0.18);
                }
                transform.scale = Vec3::splat(1.06);
            }
            _ => {
                transform.scale = Vec3::splat(1.0);
            }
        }
    }
}

fn ca38_pose_from_runtime(
    panel: &RuntimeControlPanel,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> crate::Ca38CreaturePose {
    let animation = match panel.selected_action_kind {
        Some(ActionKind::Move) if panel.target_entity == Some(3) => CreatureAnimationState::Afraid,
        Some(ActionKind::Move) => CreatureAnimationState::Moving,
        Some(ActionKind::Interact) | Some(ActionKind::Hold) => CreatureAnimationState::Interacting,
        Some(ActionKind::Inspect) => CreatureAnimationState::Inspecting,
        Some(ActionKind::Rest) => CreatureAnimationState::Sleeping,
        Some(ActionKind::Vocalize) | Some(ActionKind::Write) | Some(ActionKind::Gesture) => {
            CreatureAnimationState::Signaling
        }
        Some(ActionKind::Idle) | None => {
            if gpu.fallback_reason.is_some() {
                CreatureAnimationState::Resting
            } else {
                CreatureAnimationState::Idle
            }
        }
    };
    let expression = if panel.terminal_recovery_cause.is_some() {
        CreatureExpressionState::Pained
    } else if panel.target_entity == Some(3) {
        CreatureExpressionState::Afraid
    } else if panel.target_entity == Some(2) {
        CreatureExpressionState::Hungry
    } else if gpu.fallback_reason.is_some() {
        CreatureExpressionState::Tired
    } else if gpu.h_shadow_applications > 0 {
        CreatureExpressionState::Energized
    } else {
        CreatureExpressionState::Neutral
    };
    crate::ca38_creature_pose_for_state(animation, expression)
}

fn ca38_graphical_creature_size(pose: crate::Ca38CreaturePose) -> Vec2 {
    Vec2::new(9.0 * pose.scale_x, 8.0 * pose.scale_y)
}

fn ca38_graphical_creature_scale(
    pose: crate::Ca38CreaturePose,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> f32 {
    let learning = if gpu.h_shadow_applications > 0 {
        1.045
    } else {
        1.0
    };
    1.0 + pose.pulse * 0.020 * learning
}

fn update_graphical_feedback_pulses(
    runtime: Res<GraphicalRuntimeControlsResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    markers: bevy::prelude::Query<
        (&GraphicalPlaygroundMarker, &Transform),
        Without<GraphicalSensoryCuePulse>,
    >,
    mut pulses: bevy::prelude::Query<(&GraphicalSensoryCuePulse, &mut Sprite, &mut Transform)>,
) {
    let phase = (runtime.panel.mind_tick % 4) as f32;
    let pulse_scale = 1.0 + phase * 0.035;
    for (pulse, mut sprite, mut transform) in &mut pulses {
        if let Some(target) = pulse.target_stable_id {
            if let Some((_, marker_transform)) = markers
                .iter()
                .find(|(marker, _)| marker.stable_id == target)
            {
                transform.translation = marker_transform.translation + Vec3::new(0.0, 0.0, 0.28);
            }
        }
        let active = ca08_pulse_active(pulse.kind, &runtime.panel, &gpu.telemetry);
        sprite.color = ca08_pulse_color(pulse.kind, active);
        transform.scale = Vec3::splat(if active { pulse_scale } else { 0.92 });
    }
}

fn update_graphical_population_overlay(
    population: Option<Res<GraphicalPopulationResource>>,
    mut overlays: bevy::prelude::Query<&mut Text, With<GraphicalPopulationOverlay>>,
) {
    let Some(population) = population else {
        return;
    };
    for mut text in &mut overlays {
        text.0 = population.summary.compact_overlay_text();
    }
}

fn update_graphical_ecology_overlay(
    ecology: Option<Res<GraphicalEcologyResource>>,
    mut overlays: bevy::prelude::Query<&mut Text, With<GraphicalEcologyOverlay>>,
) {
    let Some(ecology) = ecology else {
        return;
    };
    for mut text in &mut overlays {
        text.0 = ca19_ecology_overlay_text(&ecology.summary);
    }
}

fn update_graphical_lifecycle_overlay(
    lifecycle: Option<Res<GraphicalLifecycleResource>>,
    mut overlays: bevy::prelude::Query<&mut Text, With<GraphicalLifecycleOverlay>>,
) {
    let Some(lifecycle) = lifecycle else {
        return;
    };
    for mut text in &mut overlays {
        text.0 = ca20_lifecycle_overlay_text(&lifecycle.summary);
    }
}

fn update_graphical_topology_overlay(
    runtime: Res<GraphicalRuntimeControlsResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<GraphicalTopologyOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = runtime.panel.topology_overlay.panel_text();
    }
}

fn update_graphical_memory_journal_overlay(
    runtime: Res<GraphicalRuntimeControlsResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<GraphicalMemoryJournalOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = runtime.panel.memory_journal.panel_text();
    }
}

fn update_graphical_neural_activity_profiler_overlay(
    runtime: Res<GraphicalRuntimeControlsResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<GraphicalNeuralActivityProfilerOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = runtime.panel.neural_profiler.panel_text();
    }
}

fn update_graphical_onboarding_tutorial_overlay(
    runtime: Res<GraphicalRuntimeControlsResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<GraphicalOnboardingTutorialOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = crate::ca40_first_session_tutorial_panel_text(&runtime.panel, &gpu.telemetry);
    }
}

fn handle_graphical_school_toggle_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    school: Option<ResMut<GraphicalSchoolResource>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyT) {
        return;
    }
    if let Some(mut school) = school {
        school.summary.toggle_school_enabled();
    }
}

fn update_graphical_school_overlay(
    school: Option<Res<GraphicalSchoolResource>>,
    view_mode: Res<GraphicalViewModeResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<GraphicalSchoolOverlay>>,
    mut cue_markers: bevy::prelude::Query<&mut Visibility, With<GraphicalTeacherCueMarker>>,
) {
    let Some(school) = school else {
        return;
    };
    for mut text in &mut overlays {
        text.0 = ca23_school_overlay_text(&school.summary);
    }
    let visibility =
        if school.summary.school_enabled && view_mode.mode.teacher_debug_labels_visible() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    for mut marker_visibility in &mut cue_markers {
        *marker_visibility = visibility;
    }
}

fn ca08_pulse_active(
    kind: Ca08SensoryCueKind,
    runtime: &RuntimeControlPanel,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> bool {
    match kind {
        Ca08SensoryCueKind::Reward => {
            matches!(
                runtime.selected_action_kind,
                Some(ActionKind::Interact | ActionKind::Hold)
            ) && runtime.target_entity == Some(2)
        }
        Ca08SensoryCueKind::Pain => runtime.target_entity == Some(3),
        Ca08SensoryCueKind::Sleep => matches!(runtime.selected_action_kind, Some(ActionKind::Rest)),
        Ca08SensoryCueKind::Learning => gpu.h_shadow_applications > 0,
    }
}

fn ca08_pulse_color(kind: Ca08SensoryCueKind, active: bool) -> Color {
    let alpha = if active { 0.30 } else { 0.070 };
    match kind {
        Ca08SensoryCueKind::Reward => Color::srgba(0.34, 1.0, 0.46, alpha),
        Ca08SensoryCueKind::Pain => Color::srgba(1.0, 0.20, 0.18, alpha),
        Ca08SensoryCueKind::Sleep => Color::srgba(0.48, 0.68, 1.0, alpha),
        Ca08SensoryCueKind::Learning => Color::srgba(0.26, 1.0, 0.82, alpha),
    }
}

fn update_graphical_intent_feedback(
    runtime: Res<GraphicalRuntimeControlsResource>,
    selection: Res<SelectionResource>,
    markers: bevy::prelude::Query<
        (&GraphicalPlaygroundMarker, &Transform),
        (Without<GraphicalIntentLine>, Without<GraphicalActionBadge>),
    >,
    mut lines: bevy::prelude::Query<
        (&mut Sprite, &mut Transform),
        (With<GraphicalIntentLine>, Without<GraphicalActionBadge>),
    >,
    mut badges: bevy::prelude::Query<
        (&mut Text2d, &mut TextColor, &mut Transform),
        (With<GraphicalActionBadge>, Without<GraphicalIntentLine>),
    >,
) {
    let mut creature_position = None;
    let mut target_position = None;
    let target = runtime.panel.target_entity.map(WorldEntityId);

    for (marker, transform) in &markers {
        if marker.kind == WorldObjectKind::Agent && marker.stable_id == selection.stable_id {
            creature_position = Some(transform.translation);
        }
        if Some(marker.stable_id) == target {
            target_position = Some(transform.translation);
        }
    }

    let action_kind = runtime.panel.selected_action_kind;
    let action_label = action_kind
        .map(|action| crate::action_badge_label_for_target(action, runtime.panel.target_entity))
        .unwrap_or("idle");
    for (mut text, mut color, mut transform) in &mut badges {
        text.0 = format!("action: {}", action_label.to_ascii_lowercase());
        color.0 = action_badge_color(action_kind, runtime.panel.target_entity);
        if let Some(position) = creature_position {
            transform.translation = position + Vec3::new(0.0, 44.0, 1.15);
        }
    }

    for (mut sprite, mut transform) in &mut lines {
        if let (Some(start), Some(end), Some(action)) =
            (creature_position, target_position, action_kind)
        {
            let delta = end - start;
            let length = (delta.x * delta.x + delta.y * delta.y).sqrt();
            if length > 1.0 {
                transform.translation =
                    Vec3::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5, 0.32);
                transform.rotation = bevy::prelude::Quat::from_rotation_z(delta.y.atan2(delta.x));
                sprite.custom_size = Some(Vec2::new(length, 5.0));
                sprite.color = intent_line_color(action, runtime.panel.target_entity);
            }
        } else {
            sprite.color = Color::srgba(0.42, 1.0, 0.58, 0.0);
            sprite.custom_size = Some(Vec2::new(1.0, 5.0));
        }
    }
}

fn capture_graphical_runtime_snapshot(
    runtime: Res<GraphicalRuntimeControlsResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    sink: Option<Res<GraphicalRuntimeCaptureSink>>,
) {
    if let Some(sink) = sink {
        if let Ok(mut slot) = sink.0.lock() {
            *slot = Some((runtime.panel.clone(), gpu.telemetry.clone()));
        }
    }
}

fn action_badge_color(action_kind: Option<ActionKind>, target_entity: Option<u64>) -> Color {
    match action_kind {
        Some(ActionKind::Move) if target_entity == Some(3) => Color::srgb(1.0, 0.34, 0.32),
        Some(ActionKind::Move) => Color::srgb(0.48, 0.78, 1.0),
        Some(ActionKind::Interact) | Some(ActionKind::Hold) => Color::srgb(0.42, 1.0, 0.58),
        Some(ActionKind::Inspect) => Color::srgb(1.0, 0.88, 0.36),
        Some(ActionKind::Rest) => Color::srgb(0.72, 0.65, 1.0),
        Some(ActionKind::Vocalize) | Some(ActionKind::Write) | Some(ActionKind::Gesture) => {
            Color::srgb(0.82, 0.68, 1.0)
        }
        Some(ActionKind::Idle) | None => Color::srgb(0.86, 0.88, 0.82),
    }
}

fn intent_line_color(action_kind: ActionKind, target_entity: Option<u64>) -> Color {
    match action_kind {
        ActionKind::Move if target_entity == Some(3) => Color::srgba(1.0, 0.34, 0.32, 0.88),
        ActionKind::Move => Color::srgba(0.48, 0.78, 1.0, 0.86),
        ActionKind::Interact | ActionKind::Hold => Color::srgba(0.42, 1.0, 0.58, 0.88),
        ActionKind::Inspect => Color::srgba(1.0, 0.88, 0.36, 0.84),
        ActionKind::Rest => Color::srgba(0.72, 0.65, 1.0, 0.78),
        ActionKind::Vocalize | ActionKind::Write | ActionKind::Gesture => {
            Color::srgba(0.82, 0.68, 1.0, 0.82)
        }
        ActionKind::Idle => Color::srgba(0.86, 0.88, 0.82, 0.70),
    }
}

fn update_graphical_feedback_overlay(
    runtime: Res<GraphicalRuntimeControlsResource>,
    feedback: Res<GraphicalFeedbackCueResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    view_mode: Res<GraphicalViewModeResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<FeedbackCueOverlay>>,
) {
    for mut text in &mut overlays {
        if view_mode.mode == GraphicalPlaygroundViewMode::Player {
            text.0 = ca42a_collapsed_event_chip_text(&runtime.panel, &gpu.telemetry);
            continue;
        }
        let cue_panel = crate::ca39_drive_audio_vfx_panel_text_from_graphical(
            &feedback.summary,
            &gpu.telemetry,
        )
        .unwrap_or_else(|_| {
            format!(
                "Drive Audio/VFX\nCues unavailable; H_shadow apps={}\nBoundary: display-only",
                gpu.telemetry.h_shadow_applications
            )
        });
        text.0 = format!("{}\n{}", runtime.panel.event_feed_panel_text(), cue_panel);
    }
}

fn update_graphical_boundary_footer_overlay(
    gpu: Res<GraphicalGpuTelemetryResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<BoundaryFooterOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = ca05_boundary_footer_text(&gpu.telemetry);
    }
}

fn update_graphical_save_load_menu_overlay(
    menu: Res<GraphicalSaveLoadMenuResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<SaveLoadMenuOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = crate::graphical_save_load_menu_text(&menu.session);
    }
}

fn update_graphical_advanced_gameplay_overlay(
    advanced: Res<GraphicalAdvancedGameplayResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<AdvancedGameplayOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = alpha_playtest_status_note_text(&advanced.summary);
    }
}

pub fn save_load_menu_overlay_text(summary: &crate::SaveLoadUxSmokeSummary) -> String {
    crate::player_save_load_menu_text(summary)
}

pub fn graphical_inspector_overlay_text(
    _runtime: &GraphicalRuntimeControlsResource,
    _camera: &CameraNavigationResource,
    selection: &SelectionResource,
    inspector: &CreatureInspectorResource,
    gpu: &GraphicalGpuTelemetryResource,
) -> String {
    let snapshot = &inspector.snapshot;
    let action = snapshot
        .visual
        .selected_action_kind
        .map(|kind| {
            crate::action_badge_label_for_target(
                kind,
                snapshot.visual.target_entity.map(|id| id.raw()),
            )
        })
        .unwrap_or("IDLE");
    let target = snapshot
        .visual
        .target_entity
        .map_or_else(|| "none".to_string(), |id| format!("stable:{}", id.raw()));
    let patch = compact_overlay_line(&snapshot.patch_summary, 34);
    let sleep = ca07_awake_sleep_status(snapshot);
    let bars = ca07_creature_state_bars(snapshot).join("\n");
    let learning = ca07_learning_summary(&gpu.telemetry);
    let tech = ca07_compact_technical_summary(&gpu.telemetry);
    let pose = crate::ca38_animation_label_line(&snapshot.visual);
    let fallback = compact_overlay_line(
        gpu.telemetry.fallback_reason.as_deref().unwrap_or("none"),
        22,
    );
    format!(
        concat!(
            "Creature Inspector\n",
            "Stable ID: {}\n",
            "State: {}  {}/{}\n",
            "{}\n",
            "Action: {}  Target: {}\n",
            "Pose: {}\n",
            "Patch: {}\n",
            "Learning: {}\n",
            "GPU: {}  fallback={}\n",
            "Gate: CPU shadow\n",
            "Tech: {}\n",
            "Read-only stable IDs\n",
            "Claim: full_auth=false"
        ),
        selection.stable_id.raw(),
        sleep,
        snapshot.visual.animation.label(),
        snapshot.visual.expression.label(),
        bars,
        action,
        target,
        compact_overlay_line(&pose, 34),
        patch,
        learning,
        compact_overlay_line(&gpu.telemetry.selected_backend, 14),
        fallback,
        tech,
    )
}

pub fn graphical_player_status_overlay_text(
    panel: &RuntimeControlPanel,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> String {
    let status = panel.terminal_recovery_cause.as_ref().map_or_else(
        || match panel.playback {
            RuntimePlaybackState::Running => "RUN".to_string(),
            RuntimePlaybackState::Paused => "PAUSE".to_string(),
            RuntimePlaybackState::ShutdownRequested => "EXIT".to_string(),
        },
        |_| "STOP".to_string(),
    );
    format!(
        concat!("A-Life GPU Alpha\n", "{}  {}x  t{}\n", "GPU {}  L{}"),
        status,
        panel.run_speed_ticks,
        panel.mind_tick,
        ca42a_gpu_status_chip(gpu),
        gpu.h_shadow_applications,
    )
}

pub fn graphical_full_debug_status_overlay_text(
    panel: &RuntimeControlPanel,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> String {
    let action = panel.selected_action_kind.map_or("None", |kind| {
        crate::action_badge_label_for_target(kind, panel.target_entity)
    });
    let goal = graphical_goal_label(panel.selected_action_kind, panel.target_entity);
    let target = panel
        .target_entity
        .map_or_else(|| "none".to_string(), |id| format!("stable:{id}"));
    let world = panel
        .world_tick
        .map_or_else(|| "pending".to_string(), |tick| tick.to_string());
    let fallback = if let Some(reason) = gpu.fallback_reason.as_deref() {
        format!("CPU fallback: {reason}")
    } else {
        "fallback: none".to_string()
    };
    let pose = ca38_pose_from_runtime(panel, gpu);
    let status = panel.terminal_recovery_cause.as_ref().map_or_else(
        || panel.playback.label().to_string(),
        |_| "paused: reset available".to_string(),
    );
    format!(
        concat!(
            "A-Life GPU Alpha Playground\n",
            "State: {}  speed={}x  tick={} world={}\n",
            "GPU: {}  {}\n",
            "Creature: stable:1  Goal: {}\n",
            "Action: {}  Target: {}\n",
            "Pose: {} ({})\n",
            "Patch: sealed={} count={}\n",
            "Learning: H_shadow apps={} delta={:.4}\n",
            "Gate: CPU shadow; full_auth=false\n",
            "Controls: Space run/pause | N step | R reset | Esc quit"
        ),
        status,
        panel.run_speed_ticks,
        panel.mind_tick,
        world,
        gpu.selected_backend,
        fallback,
        goal,
        action,
        target,
        pose.action_label,
        pose.pose_id,
        panel.last_patch_sealed,
        panel.sealed_patch_count,
        gpu.h_shadow_applications,
        gpu.last_h_shadow_delta,
    )
}

pub fn graphical_player_inspector_overlay_text(
    selection: &SelectionResource,
    inspector: &CreatureInspectorResource,
    gpu: &GraphicalGpuTelemetryResource,
) -> String {
    let snapshot = &inspector.snapshot;
    let action = snapshot
        .visual
        .selected_action_kind
        .map(|kind| {
            crate::action_badge_label_for_target(
                kind,
                snapshot.visual.target_entity.map(|id| id.raw()),
            )
        })
        .unwrap_or("IDLE");
    let cues = snapshot.visual.cues;
    format!(
        concat!(
            "Creature\n",
            "{}  {}\n",
            "E{:02} H{:02} Hu{:02}\n",
            "Act {}\n",
            "GPU {}"
        ),
        if selection.stable_id == snapshot.visual.stable_id {
            "active"
        } else {
            "creature"
        },
        snapshot.visual.animation.label(),
        (cues.energy.value * 100.0).round() as i32,
        (ca07_health_value(cues.pain.value, cues.fear.value) * 100.0).round() as i32,
        (cues.hunger.value * 100.0).round() as i32,
        action,
        ca42a_gpu_status_chip(&gpu.telemetry),
    )
}

fn ca42a_gpu_status_chip(gpu: &GraphicalGpuRuntimeTelemetry) -> String {
    if let Some(reason) = gpu.fallback_reason.as_deref() {
        format!("CPU fallback ({})", compact_overlay_line(reason, 18))
    } else if gpu.selected_backend == "PendingFirstTick" {
        "arming".to_string()
    } else if gpu.selected_backend.to_ascii_lowercase().contains("gpu") {
        "ON".to_string()
    } else {
        compact_overlay_line(&gpu.selected_backend, 20)
    }
}

fn ca42a_collapsed_event_chip_text(
    panel: &RuntimeControlPanel,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> String {
    if let Some(cause) = panel.terminal_recovery_cause.as_deref() {
        return format!("Events: stopped - {cause}. Press R");
    }
    let last_event = panel
        .player_events
        .last()
        .map(|event| compact_overlay_line(event, 44))
        .unwrap_or_else(|| "waiting for first tick".to_string());
    format!(
        "Events: tick {} | {} | H_shadow {}",
        panel.mind_tick, last_event, gpu.h_shadow_applications
    )
}

fn graphical_goal_label(action: Option<ActionKind>, target_entity: Option<u64>) -> &'static str {
    match (action, target_entity) {
        (Some(ActionKind::Interact | ActionKind::Hold), Some(2)) => "food",
        (Some(ActionKind::Move), Some(3)) => "hazard",
        (Some(ActionKind::Inspect), _) => "inspect",
        (Some(ActionKind::Rest), _) => "rest",
        (Some(ActionKind::Gesture | ActionKind::Vocalize | ActionKind::Write), _) => "cue",
        (Some(ActionKind::Move), _) => "move",
        _ => "idle",
    }
}

fn compact_overlay_line(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(3);
    let mut compact = value.chars().take(keep).collect::<String>();
    compact.push_str("...");
    compact
}

pub fn ca07_creature_state_bars(snapshot: &CreatureInspectorSnapshot) -> Vec<String> {
    let cues = snapshot.visual.cues;
    let health = ca07_health_value(cues.pain.value, cues.fear.value);
    vec![
        ca07_bar_line("Energy", cues.energy.value),
        ca07_bar_line("Health", health),
        ca07_bar_line("Hunger", cues.hunger.value),
        ca07_bar_line("Fatigue", cues.fatigue.value),
        ca07_bar_line("Fear", cues.fear.value),
    ]
}

fn ca07_bar_line(label: &str, value: f32) -> String {
    let value = value.clamp(0.0, 1.0);
    let filled = (value * 10.0).round().clamp(0.0, 10.0) as usize;
    let empty = 10usize.saturating_sub(filled);
    format!(
        "{:<7}[{}{}] {:>3}%",
        label,
        "#".repeat(filled),
        ".".repeat(empty),
        (value * 100.0).round() as u32
    )
}

fn ca07_health_value(pain: f32, fear: f32) -> f32 {
    (1.0 - pain.clamp(0.0, 1.0).max(fear.clamp(0.0, 1.0) * 0.5)).clamp(0.0, 1.0)
}

fn ca07_awake_sleep_status(snapshot: &CreatureInspectorSnapshot) -> &'static str {
    match snapshot.visual.sleep_phase {
        alife_core::SleepPhase::Awake => "Awake",
        alife_core::SleepPhase::EnteringSleep => "Entering sleep",
        alife_core::SleepPhase::Consolidating => "Consolidating",
        alife_core::SleepPhase::Waking => "Waking",
        alife_core::SleepPhase::ForcedRecoverySleep => "Recovery sleep",
    }
}

fn ca07_learning_summary(gpu: &GraphicalGpuRuntimeTelemetry) -> String {
    format!(
        "H_shadow={} last={:.4}",
        gpu.h_shadow_applications, gpu.last_h_shadow_delta
    )
}

fn ca07_compact_technical_summary(gpu: &GraphicalGpuRuntimeTelemetry) -> String {
    format!("{} gate=CPU shadow", gpu.selected_backend)
}

pub fn readability_legend_overlay_text() -> String {
    [
        "Visual Guide: [@] creature | [+] food | [!] hazard | [#] obstacle/rock | [T] cue",
        "Viewport: local camera slice of a larger seeded terrain map; pan/follow to explore.",
        "GPU alpha map: off-screen stable-ID food, hazards, obstacles, and social cues exist.",
        "World art: procedural terrain, soil paths, groves, hazard pressure, stone props.",
        "Cues: reward=green pain=red sleep=blue learning=teal. Audio stubs are display-only.",
        "Terrain guides placement; world/core arbitration still owns actions. Stable IDs stay portable.",
    ]
    .join("\n")
}

pub fn ca19_ecology_overlay_text(summary: &Ca19GraphicalEcologySummary) -> String {
    format!(
        concat!(
            "Resource Ecology\n",
            "{}\n",
            "Hazard pressure zones visible={} | stable IDs only\n",
            "Boundary: terrain/resource visuals cannot emit actions"
        ),
        summary.compact_overlay_text(),
        summary.hazard_pressure_zone_count,
    )
}

pub fn ca20_lifecycle_overlay_text(summary: &Ca20GraphicalLifecycleSummary) -> String {
    format!(
        concat!(
            "{}\n",
            "Birth/death events visible; population cap enforced.\n",
            "Boundary: birth assets initialize only; lifetime state not inherited.\n",
            "Stable IDs only; lineage visuals cannot emit actions."
        ),
        summary.compact_overlay_text(),
    )
}

pub fn ca23_school_overlay_text(summary: &Ca23GraphicalSchoolSummary) -> String {
    format!(
        "{}\nSchool visuals are display-only; teacher cues cannot emit actions.",
        summary.compact_overlay_text()
    )
}

pub fn ca05_controls_bar_text() -> &'static str {
    concat!(
        "Controls: click | Tab | Space run/pause | N step | R reset | Esc\n",
        "View: WASD | +/- zoom | F follow | M save/load | F5 save | F9 load | [!] hazard"
    )
}

pub fn ca42a_player_controls_bar_text() -> &'static str {
    "Click select | Space | N | R | Esc | WASD | +/-"
}

pub fn alpha_controls_help_text() -> &'static str {
    "Controls: Left click select | Tab cycle creatures | Space run/pause | N step | R reset | T school | M save/load | F5 save | F9 load | +/- zoom | F follow | Esc quit"
}

pub fn ca05_boundary_footer_text(gpu: &GraphicalGpuRuntimeTelemetry) -> String {
    format!(
        "Boundary: CPU shadow gate | Claim: {} | no full action-authoritative | no bulk readback={}",
        gpu.product_runtime_claim, gpu.no_active_bulk_readback
    )
}

pub fn feedback_cue_overlay_text(
    feedback: &crate::FeedbackPolishSummary,
    inspector: &CreatureInspectorResource,
) -> String {
    let snapshot = &inspector.snapshot;
    let evidence = crate::Ca39RuntimeCueEvidence {
        selected_backend: "CpuReference".to_string(),
        fallback_reason: None,
        product_runtime_claim: "None".to_string(),
        sealed_patches: feedback.sealed_outcome_event_count,
        h_shadow_applications: 0,
        cpu_shadow_gate_preserved: true,
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };
    let drive_panel = crate::ca39_drive_audio_vfx_panel_text(feedback, &evidence)
        .unwrap_or_else(|_| "Drive Audio/VFX unavailable".to_string());
    format!(
        concat!(
            "Play Feedback (display-only)\n",
            "{}\n",
            "{}\n",
            "Cues: {}\n",
            "Food={} hazard={} sleep={} failure={}\n",
            "Creature: {}/{} curiosity={:.2}\n",
            "Boundary: cues cannot act or mutate weights"
        ),
        ca08_sensory_cue_panel_text(
            feedback,
            &GraphicalGpuRuntimeTelemetry::cpu_reference(GraphicalGpuRuntimeMode::CpuReference, 0),
        ),
        drive_panel,
        feedback.event_labels().join(">"),
        feedback
            .event_labels()
            .iter()
            .any(|label| *label == crate::FeedbackEventKind::FoodReward.label()),
        feedback
            .event_labels()
            .iter()
            .any(|label| *label == crate::FeedbackEventKind::HazardPain.label()),
        feedback
            .event_labels()
            .iter()
            .any(|label| *label == crate::FeedbackEventKind::SleepTransition.label()),
        feedback
            .event_labels()
            .iter()
            .any(|label| *label == crate::FeedbackEventKind::MissingAffordance.label()),
        snapshot.visual.animation.label(),
        snapshot.visual.expression.label(),
        snapshot.visual.cues.curiosity.value,
    )
}

pub fn ca08_sensory_feedback_cues(
    feedback: &crate::FeedbackPolishSummary,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> Vec<Ca08SensoryCueRow> {
    let mut reward = None;
    let mut pain = None;
    let mut sleep = None;
    for event in &feedback.events {
        match event.kind {
            crate::FeedbackEventKind::FoodReward => reward = Some(event.stable_entity),
            crate::FeedbackEventKind::HazardPain => pain = Some(event.stable_entity),
            crate::FeedbackEventKind::SleepTransition => sleep = Some(event.stable_entity),
            _ => {}
        }
    }
    vec![
        Ca08SensoryCueRow {
            kind: Ca08SensoryCueKind::Reward,
            target: reward.flatten().or(Some(WorldEntityId(2))),
            active: reward.is_some(),
        },
        Ca08SensoryCueRow {
            kind: Ca08SensoryCueKind::Pain,
            target: pain.flatten().or(Some(WorldEntityId(3))),
            active: pain.is_some(),
        },
        Ca08SensoryCueRow {
            kind: Ca08SensoryCueKind::Sleep,
            target: sleep.flatten().or(Some(WorldEntityId(1))),
            active: sleep.is_some(),
        },
        Ca08SensoryCueRow {
            kind: Ca08SensoryCueKind::Learning,
            target: Some(WorldEntityId(1)),
            active: gpu.h_shadow_applications > 0,
        },
    ]
}

pub fn ca08_sensory_cue_panel_text(
    feedback: &crate::FeedbackPolishSummary,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> String {
    let lines = ca08_sensory_feedback_cues(feedback, gpu)
        .into_iter()
        .map(Ca08SensoryCueRow::panel_line)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Sensory Cues (display-only)\n{}\nBoundary: no action/weight authority",
        lines
    )
}

pub fn alpha_save_load_note_text(summary: &crate::SaveLoadUxSmokeSummary) -> String {
    format!(
        concat!(
            "Save/Load Alpha Note\n",
            "Manual slot: {}  autosave: {}\n",
            "Stable IDs: [{}]  schema={}\n",
            "Reset/restart: press R or close and relaunch this fixture."
        ),
        summary.manual_save_slot,
        summary.autosave_slot,
        summary
            .stable_world_ids
            .iter()
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        summary.schema_version,
    )
}

pub fn alpha_playtest_status_note_text(summary: &AdvancedGameplayUxSummary) -> String {
    format!(
        concat!(
            "Alpha Playtest Focus\n",
            "GPU-first creature loop visible; advanced systems optional={}. CPU fallback is degraded safety mode.\n",
            "Record: window, controls, inspector, fallback warning, confusing text."
        ),
        summary.optional_modes,
    )
}

pub fn graphical_object_badge_text(object: &VisibleWorldObjectPresentation) -> String {
    let marker = match object.kind {
        WorldObjectKind::Agent => "[@] creature",
        WorldObjectKind::Food => "[+] food",
        WorldObjectKind::Hazard => "[!] hazard",
        WorldObjectKind::Obstacle => "[#] rock",
        WorldObjectKind::Token => "[T] cue",
    };
    format!("{} stable:{}", marker, object.stable_id.raw())
}

fn graphical_badge_offset(kind: WorldObjectKind) -> Vec3 {
    match kind {
        WorldObjectKind::Agent => Vec3::new(-54.0, 52.0, 1.0),
        WorldObjectKind::Food => Vec3::new(-38.0, 58.0, 1.0),
        WorldObjectKind::Hazard => Vec3::new(54.0, -48.0, 1.0),
        WorldObjectKind::Obstacle => Vec3::new(56.0, 34.0, 1.0),
        WorldObjectKind::Token => Vec3::new(54.0, 34.0, 1.0),
    }
}

fn readability_label_color(kind: WorldObjectKind) -> Color {
    match kind {
        WorldObjectKind::Agent => Color::srgb(1.0, 0.88, 0.35),
        WorldObjectKind::Food => Color::srgb(0.34, 1.0, 0.46),
        WorldObjectKind::Hazard => Color::srgb(1.0, 0.28, 0.22),
        WorldObjectKind::Obstacle => Color::srgb(0.78, 0.76, 0.70),
        WorldObjectKind::Token => Color::srgb(0.82, 0.68, 1.0),
    }
}
