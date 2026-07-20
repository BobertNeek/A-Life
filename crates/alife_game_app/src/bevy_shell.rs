//! Feature-gated Bevy playground shell split during R13 remediation.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
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
    persistence::PortableSaveFile, sample_procedural_terrain_tile, CreatureWorldAnchor,
    ProceduralChunkActivationReport, ProceduralChunkCoord, ProceduralTerrainSample,
    ProceduralTileCoord, ProceduralWorldConfig, ProceduralWorldContentCandidate,
    ProceduralWorldContentKind, ProceduralWorldContentReport, TerrainZoneKind, WorldObjectKind,
};
use bevy::{
    app::AppExit,
    asset::{AssetPlugin, Assets, Handle, RenderAssetUsages},
    camera::ScalingMode,
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        tonemapping::Tonemapping,
        FullscreenShader,
    },
    ecs::{query::QueryItem, schedule::IntoScheduleConfigs},
    gltf::GltfAssetLabel,
    image::{BevyDefault, CompressedImageFormats, Image, ImageSampler, ImageType},
    prelude::{
        default, AlphaMode, App, AssetServer, BackgroundColor, ButtonInput, Camera, Camera2d,
        Camera3d, Capsule3d, Circle, ClearColor, ClearColorConfig, Color, Commands, Component,
        Cone, Cuboid, DefaultPlugins, DirectionalLight, Entity, GlobalTransform, ImageNode,
        KeyCode, Mesh, Mesh3d, MeshBuilder, MeshMaterial3d, Meshable, MessageWriter,
        MinimalPlugins, MouseButton, Name, Node, NonSend, NonSendMut, OrthographicProjection,
        Plane3d, Plugin, PluginGroup, PositionType, Projection, Quat, Res, ResMut, Resource,
        SceneRoot, Sphere, Sprite, StandardMaterial, Text, Text2d, TextColor, TextFont, Time,
        ToRing, Transform, Update, Val, Vec2, Vec3, Visibility, With, Without, World,
    },
    render::{
        extract_component::{
            ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
            UniformComponentPlugin,
        },
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel, ViewNode, ViewNodeRunner,
        },
        render_resource::{
            binding_types::{sampler, texture_2d, texture_depth_2d_multisampled, uniform_buffer},
            BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, Extent3d, FragmentState,
            Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
            RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
            ShaderType, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice},
        settings::{RenderCreation, WgpuSettings},
        view::{ViewDepthTexture, ViewTarget},
        RenderApp, RenderPlugin, RenderStartup,
    },
    shader::Shader,
    window::{ExitCondition, PresentMode, PrimaryWindow, Window, WindowPlugin, WindowTheme},
    winit::WinitSettings,
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
    GraphicalGpuRuntimeController, GraphicalGpuRuntimeTelemetry, GraphicalPlaygroundLaunchConfig,
    GraphicalPlaygroundLaunchSummary, GraphicalPlaygroundMode, GraphicalPlaygroundViewMode,
    LiveBrainLoop, LiveBrainTickSummary, ProductionVoxelLaunchConfig, ProductionVoxelLaunchSummary,
    RuntimeControlCommand, RuntimeControlPanel, RuntimePlaybackState, VisibleMaterialKind,
    VisiblePlaceholderShape, VisibleWorldObjectPresentation, VisibleWorldPresentation,
    CA13_FIXED_SIM_TICK_HZ, CA13_TARGET_RENDER_FRAME_HZ, S02_MAX_SMOKE_TICKS,
};

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct BevyAppShellSummary {
    pub seed: u64,
    pub current_state: GameAppState,
    pub graphics_required_for_default_path: bool,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct ProductionVoxelFrontendResource {
    pub summary: crate::ProductionVoxelLaunchSummary,
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
    pub terrain_water: Handle<Image>,
    pub terrain_sand: Handle<Image>,
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
            terrain_water: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/terrain_water.png"),
                "terrain_water.png",
            )?,
            terrain_sand: ca44a_register_embedded_alpha_art(
                images,
                include_bytes!("../assets/alpha_art_v1/terrain_sand.png"),
                "terrain_sand.png",
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
            terrain_water: Handle::default(),
            terrain_sand: Handle::default(),
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
            if self.gpu.authoritative {
                "GpuAuthoritative"
            } else {
                "Unavailable"
            }
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

#[derive(Debug, Clone, PartialEq, Resource)]
pub(crate) struct ProductionGpuBrainAuthorityResource {
    pub telemetry: crate::GpuBrainAuthorityTelemetry,
}

#[cfg(feature = "gpu-runtime")]
#[derive(Resource)]
pub(crate) struct ProductionGpuBrainRuntimeResource {
    pub(crate) runtime: crate::GpuLiveBrainRuntime,
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Resource)]
struct ProductionGpuBrainTickScheduleResource {
    startup_render_frames_remaining: u8,
}

#[cfg(feature = "gpu-runtime")]
impl ProductionGpuBrainTickScheduleResource {
    const fn new(startup_render_frames: u8) -> Self {
        Self {
            startup_render_frames_remaining: startup_render_frames,
        }
    }

    fn take_dispatch_permit(&mut self) -> bool {
        if self.startup_render_frames_remaining == 0 {
            true
        } else {
            self.startup_render_frames_remaining -= 1;
            false
        }
    }
}

#[cfg(feature = "gpu-runtime")]
const PRODUCTION_GPU_STARTUP_RENDER_FRAMES: u8 = 12;

#[cfg(feature = "gpu-runtime")]
fn prepare_production_gpu_runtime_launch(
    launch: &ProductionVoxelLaunchConfig,
    summary: &ProductionVoxelLaunchSummary,
) -> Result<AppShellLaunchConfig, GameAppShellError> {
    let runtime_save_path = PathBuf::from(&summary.ui_settings.runtime_save_path);
    if runtime_save_path.exists() {
        let existing = PortableSaveFile::from_json_file(&runtime_save_path)?;
        existing.validate_with_asset_root(&summary.asset_root)?;
        let existing_population = existing
            .world
            .objects
            .iter()
            .filter(|object| object.kind == alife_world::WorldObjectKind::Agent)
            .count();
        if existing_population != usize::from(summary.effective_population) {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "runtime save population {existing_population} does not match requested profile population {}; select a matching save or create a new world",
                    summary.effective_population
                ),
            });
        }
    } else {
        let source = PortableSaveFile::from_json_file(&summary.save_path)?;
        let production = crate::production_voxel_save_with_population(
            &source,
            &summary.asset_root,
            summary.profile_id,
            summary.effective_population,
        )?
        .with_gpu_runtime_state(summary.gpu_runtime_state.clone())?;
        crate::GpuDurableSaveManifest::publish_snapshot(
            &runtime_save_path,
            &summary.asset_root,
            &production,
        )?;
    }
    let mut runtime_launch = launch.app_launch.clone();
    runtime_launch.save_path = runtime_save_path;
    Ok(runtime_launch)
}

#[cfg(feature = "gpu-runtime")]
fn tick_production_gpu_brain(
    mut runtime: ResMut<ProductionGpuBrainRuntimeResource>,
    mut authority: ResMut<ProductionGpuBrainAuthorityResource>,
    mut schedule: ResMut<ProductionGpuBrainTickScheduleResource>,
) {
    if !schedule.take_dispatch_permit() {
        return;
    }
    match runtime.runtime.tick() {
        Ok(_) => authority.telemetry = runtime.runtime.authority_telemetry(),
        Err(error) => {
            authority.telemetry.authoritative = false;
            authority.telemetry.unavailable_reason = Some(error.to_string());
        }
    }
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
    let gpu = GraphicalGpuRuntimeController::new(&launch)?;
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
pub struct GraphicalTrue25dCamera {
    pub orthographic_locked: bool,
    pub pitch_degrees: f32,
    pub yaw_degrees: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTrue25dGroundPlane {
    pub texture_path: &'static str,
    pub width_world_units: f32,
    pub depth_world_units: f32,
    pub uv_repeat_x: f32,
    pub uv_repeat_z: f32,
    pub sampler_repeat_wrapped: bool,
    pub static_primitive_plane: bool,
    pub synchronous_runtime_texture_generation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTrue25dAsset {
    pub role: &'static str,
    pub stable_id: Option<WorldEntityId>,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTrue25dViewportRenderBypass {
    pub inside_locked_camera_viewport: bool,
    pub render_pass_bypassed: bool,
    pub render_extraction_bypassed: bool,
    pub presentation_draw_call_budget: u16,
    pub offscreen_animation_update_budget: u16,
    pub headless_update_continues: bool,
    pub zero_draw_call_contract: bool,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTrue25dStateCue {
    pub stable_id: WorldEntityId,
    pub pain_pose: bool,
    pub stress_desaturated: bool,
    pub learning_biolume: bool,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTrue25dCreatureEndocrinePresentation {
    pub stable_id: WorldEntityId,
    pub base_scale: f32,
    pub asset_scale_multiplier: f32,
    pub animation_speed_multiplier: f32,
    pub animation_phase_index: u8,
    pub posture_roll_degrees: f32,
    pub posture_lift: f32,
    pub adrenaline_proxy: f32,
    pub cortisol_desaturation: f32,
    pub hunger_satisfaction_biolume: f32,
    pub learning_biolume: f32,
    pub particle_trail_count: u8,
    pub biolume_particle_array_initialized: bool,
    pub creature_root_transform_applied: bool,
    pub material_shell_applied: bool,
    pub display_only: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
}

const TRUE_25D_ENDOCRINE_TENSOR_SCHEMA_VERSION: u16 = 1;
const TRUE_25D_ENDOCRINE_ADRENALINE_CHANNEL: usize = 0;
const TRUE_25D_ENDOCRINE_CORTISOL_CHANNEL: usize = 1;
const TRUE_25D_ENDOCRINE_DOPAMINE_CHANNEL: usize = 2;
const TRUE_25D_ENDOCRINE_SLEEP_PRESSURE_CHANNEL: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GraphicalTrue25dFlatEndocrineTensor {
    pub schema_version: u16,
    pub channel_count: usize,
    pub values: [f32; alife_core::EndocrineSnapshot::CHANNEL_COUNT],
    pub adrenaline_channel_index: usize,
    pub cortisol_channel_index: usize,
    pub dopamine_channel_index: usize,
    pub sleep_pressure_channel_index: usize,
    pub pain_drive_companion: f32,
    pub low_hunger_drive_companion: f32,
    pub learning_companion: f32,
    pub source: &'static str,
    pub values_bounded: bool,
    pub display_only: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicalTrue25dNeurochemicalCueKind {
    HungerGlow,
    PainSpike,
    StressDesaturation,
    EnergyTrail,
    SleepBloom,
    LearningBiolume,
}

impl GraphicalTrue25dNeurochemicalCueKind {
    pub const fn role(self) -> &'static str {
        match self {
            Self::HungerGlow => "neurochemical-hunger-glow",
            Self::PainSpike => "neurochemical-pain-spike",
            Self::StressDesaturation => "neurochemical-stress-desaturation",
            Self::EnergyTrail => "neurochemical-energy-trail",
            Self::SleepBloom => "neurochemical-sleep-bloom",
            Self::LearningBiolume => "neurochemical-learning-biolume",
        }
    }

    pub const fn player_label(self) -> &'static str {
        match self {
            Self::HungerGlow => "hunger",
            Self::PainSpike => "pain",
            Self::StressDesaturation => "stress",
            Self::EnergyTrail => "energy",
            Self::SleepBloom => "sleep",
            Self::LearningBiolume => "learning",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTrue25dNeurochemicalCue {
    pub stable_id: WorldEntityId,
    pub kind: GraphicalTrue25dNeurochemicalCueKind,
    pub intensity: f32,
    pub active: bool,
    pub anchored_to_selected_creature: bool,
    pub display_only: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTrue25dEndocrineParticleLane {
    pub stable_id: WorldEntityId,
    pub lane_index: u8,
    pub intensity: f32,
    pub active: bool,
    pub animation_phase_index: u8,
    pub initialized_from_endocrine_tensor: bool,
    pub display_only: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct GraphicalTrue25dNeurochemicalFeedbackResource {
    pub schema_version: u16,
    pub selected_stable_id: WorldEntityId,
    pub cue_count: usize,
    pub active_cue_count: usize,
    pub hunger: f32,
    pub pain: f32,
    pub stress: f32,
    pub energy: f32,
    pub sleep_pressure: f32,
    pub learning: f32,
    pub direct_mesh_presentation: bool,
    pub display_only: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
    pub gpu_authority_preserved: bool,
    pub no_active_bulk_readback: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct GraphicalTrue25dEndocrineAssetFeedbackResource {
    pub schema_version: u16,
    pub selected_stable_id: WorldEntityId,
    pub gltf_endocrine_feedback_contract_validated: bool,
    pub gltf_endocrine_feedback_assets: usize,
    pub direct_asset_feedback_contract: bool,
    pub applied_to_creature_root: bool,
    pub root_transform_posture: bool,
    pub material_shell_applied: bool,
    pub flat_endocrine_tensor_channels: usize,
    pub flat_endocrine_tensor_bounded: bool,
    pub derived_from_flat_endocrine_tensor: bool,
    pub endocrine_tensor_source: &'static str,
    pub pain_posture_active: bool,
    pub adrenaline_proxy: f32,
    pub cortisol_desaturation: f32,
    pub dopamine_biolume: f32,
    pub pain_drive_companion: f32,
    pub low_hunger_drive_companion: f32,
    pub hunger_satisfaction_biolume: f32,
    pub learning_biolume: f32,
    pub asset_scale_multiplier: f32,
    pub animation_speed_multiplier: f32,
    pub animation_phase_index: u8,
    pub animation_speed_layer_applied: bool,
    pub posture_roll_degrees: f32,
    pub posture_lift: f32,
    pub particle_trail_count: u8,
    pub biolume_particle_array_initialized: bool,
    pub biolume_particle_lanes_visible: u8,
    pub biolume_particle_lanes_max: u8,
    pub emissive_particle_array_initialized: bool,
    pub derived_from_visual_snapshot: bool,
    pub display_only: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
    pub tensor_action_authority: bool,
    pub tensor_weight_authority: bool,
    pub gpu_authority_preserved: bool,
    pub no_active_bulk_readback: bool,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalTrue25dPresentationResource {
    pub asset_manifest: crate::True25dAssetValidationSummary,
    pub versioned_gltf_pack_validated: bool,
    pub runtime_gltf_scene_rendering: bool,
    pub runtime_native_low_poly_fallback: bool,
    pub fixed_orthographic_camera: bool,
    pub preprocessed_repeating_ground_plane: bool,
    pub synchronous_runtime_ground_texture_generation: bool,
    pub ground_texture_path: &'static str,
    pub toon_bands: u8,
    pub sobel_outline_contract: bool,
    pub pixel_step_filter_contract: bool,
    pub procedural_micro_ecology_chunks: bool,
    pub offscreen_headless_chunks: bool,
    pub viewport_render_bypass: bool,
    pub offscreen_zero_draw_call_contract: bool,
    pub no_action_authority: bool,
}

pub const TRUE_25D_LAUNCH_BASELINE_SCHEMA: &str = "alife.ca44a.true25d_launch_baseline.v1";
pub const TRUE_25D_LAUNCH_BASELINE_SCHEMA_VERSION: u16 = 1;
pub const TRUE_25D_LAUNCH_BASELINE_MAX_MS: f64 = 50.0;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicalTrue25dLaunchBaselineSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub scope: &'static str,
    pub baseline_elapsed_ms: f64,
    pub baseline_under_50ms: bool,
    pub bevy_window_created: bool,
    pub cold_process_launch_measured: bool,
    pub cold_process_under_50ms_claim: bool,
    pub fixed_orthographic_camera: bool,
    pub camera_fixed_vertical_height: f32,
    pub camera_position: [f32; 3],
    pub camera_points_at_origin: bool,
    pub camera_non_rotating_locked: bool,
    pub single_static_primitive_ground_plane: bool,
    pub ground_tile_path: &'static str,
    pub ground_tile_width_px: u32,
    pub ground_tile_height_px: u32,
    pub texture_address_mode_repeat: bool,
    pub preprocessed_diffuse_tile: bool,
    pub synchronous_runtime_noise_generation: bool,
    pub synchronous_runtime_texture_generation: bool,
    pub zero_sync_runtime_noise_or_texture_generation: bool,
    pub procedural_chunk_data_ledger_only: bool,
    pub stylization_shader_embedded: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
    pub headless_path_preserved: bool,
    pub gpu_authority_preserved: bool,
    pub full_action_authoritative_claim: bool,
}

impl GraphicalTrue25dLaunchBaselineSummary {
    pub fn signature_line(&self) -> String {
        format!(
            "{}:v{}:scope={}:elapsed_ms={:.3}:under_50ms={}:window={}:cold_process_measured={}:camera={}:ground={}x{}:repeat={}:sync_noise={}:sync_texture={}:ledger_only={}:no_action_authority={}:no_weight_authority={}:gpu_authority={}:full_auth={}",
            self.schema,
            self.schema_version,
            self.scope,
            self.baseline_elapsed_ms,
            self.baseline_under_50ms,
            self.bevy_window_created,
            self.cold_process_launch_measured,
            self.fixed_orthographic_camera && self.camera_points_at_origin,
            self.ground_tile_width_px,
            self.ground_tile_height_px,
            self.texture_address_mode_repeat,
            self.synchronous_runtime_noise_generation,
            self.synchronous_runtime_texture_generation,
            self.procedural_chunk_data_ledger_only,
            self.no_action_authority,
            self.no_weight_authority,
            self.gpu_authority_preserved,
            self.full_action_authoritative_claim,
        )
    }

    pub fn contract_passed(&self) -> bool {
        !self.bevy_window_created
            && !self.cold_process_under_50ms_claim
            && self.fixed_orthographic_camera
            && (self.camera_fixed_vertical_height - TRUE_25D_VIEWPORT_VERTICAL_UNITS).abs()
                <= f32::EPSILON
            && self.camera_points_at_origin
            && self.camera_non_rotating_locked
            && self.single_static_primitive_ground_plane
            && self.ground_tile_width_px >= 256
            && self.ground_tile_height_px >= 144
            && !self.texture_address_mode_repeat
            && !self.preprocessed_diffuse_tile
            && self.synchronous_runtime_noise_generation
            && self.synchronous_runtime_texture_generation
            && !self.zero_sync_runtime_noise_or_texture_generation
            && self.procedural_chunk_data_ledger_only
            && self.stylization_shader_embedded
            && self.no_action_authority
            && self.no_weight_authority
            && self.headless_path_preserved
            && self.gpu_authority_preserved
            && !self.full_action_authoritative_claim
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct GraphicalTrue25dStylizationRenderPassResource {
    pub shader_path: &'static str,
    pub shader_source_embedded: bool,
    pub runtime_render_graph_registered: bool,
    pub attached_to_player_camera: bool,
    pub pixel_grid_width: u32,
    pub pixel_grid_height: u32,
    pub toon_bands: u8,
    pub depth_sobel_outline: bool,
    pub luminance_sobel_fallback: bool,
    pub low_resolution_pixel_step_filter: bool,
    pub display_only: bool,
    pub no_action_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component, ExtractComponent, ShaderType)]
pub struct GraphicalTrue25dStylizationSettings {
    pub pixel_grid: Vec2,
    pub toon_bands: f32,
    pub outline_threshold: f32,
    pub outline_strength: f32,
    pub depth_outline_strength: f32,
    pub _padding: Vec2,
}

impl Default for GraphicalTrue25dStylizationSettings {
    fn default() -> Self {
        Self {
            pixel_grid: TRUE_25D_STYLIZATION_PIXEL_GRID,
            toon_bands: TRUE_25D_STYLIZATION_TOON_BANDS,
            outline_threshold: TRUE_25D_STYLIZATION_OUTLINE_THRESHOLD,
            outline_strength: TRUE_25D_STYLIZATION_OUTLINE_STRENGTH,
            depth_outline_strength: TRUE_25D_STYLIZATION_DEPTH_OUTLINE_STRENGTH,
            _padding: Vec2::ZERO,
        }
    }
}

#[derive(Debug, Clone, Resource, ExtractResource)]
struct GraphicalTrue25dStylizationShaderHandle(pub Handle<Shader>);

struct True25dStylizationPostProcessPlugin;

impl Plugin for True25dStylizationPostProcessPlugin {
    fn build(&self, app: &mut App) {
        let shader_handle = register_true_25d_stylization_shader(app);
        app.insert_resource(GraphicalTrue25dStylizationShaderHandle(
            shader_handle.clone(),
        ));
        app.add_plugins((
            ExtractComponentPlugin::<GraphicalTrue25dStylizationSettings>::default(),
            UniformComponentPlugin::<GraphicalTrue25dStylizationSettings>::default(),
            ExtractResourcePlugin::<GraphicalTrue25dStylizationShaderHandle>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .world_mut()
            .insert_resource(GraphicalTrue25dStylizationShaderHandle(shader_handle));
        render_app.add_systems(RenderStartup, init_true_25d_stylization_pipeline);
        render_app
            .add_render_graph_node::<ViewNodeRunner<True25dStylizationNode>>(
                Core3d,
                True25dStylizationLabel,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::Tonemapping,
                    True25dStylizationLabel,
                    Node3d::EndMainPassPostProcessing,
                ),
            );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct True25dStylizationLabel;

#[derive(Default)]
struct True25dStylizationNode;

impl ViewNode for True25dStylizationNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewDepthTexture,
        &'static GraphicalTrue25dStylizationSettings,
        &'static DynamicUniformIndex<GraphicalTrue25dStylizationSettings>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (view_target, view_depth, _settings, settings_index): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline = world.resource::<True25dStylizationPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline.pipeline_id) else {
            return Ok(());
        };
        let settings_uniforms =
            world.resource::<ComponentUniforms<GraphicalTrue25dStylizationSettings>>();
        let Some(settings_binding) = settings_uniforms.uniforms().binding() else {
            return Ok(());
        };
        let post_process = view_target.post_process_write();
        let bind_group = render_context.render_device().create_bind_group(
            "true25d_stylization_postprocess_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                post_process.source,
                view_depth.view(),
                &pipeline.sampler,
                settings_binding.clone(),
            )),
        );

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("true25d_stylization_postprocess_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post_process.destination,
                depth_slice: None,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_render_pipeline(render_pipeline);
        render_pass.set_bind_group(0, &bind_group, &[settings_index.index()]);
        render_pass.draw(0..3, 0..1);
        Ok(())
    }
}

#[derive(Resource)]
struct True25dStylizationPipeline {
    layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    pipeline_id: CachedRenderPipelineId,
}

fn register_true_25d_stylization_shader(app: &mut App) -> Handle<Shader> {
    if !app.world().contains_resource::<Assets<Shader>>() {
        app.init_resource::<Assets<Shader>>();
    }
    app.world_mut()
        .resource_mut::<Assets<Shader>>()
        .add(Shader::from_wgsl(
            TRUE_25D_STYLIZATION_SHADER_SOURCE,
            TRUE_25D_STYLIZATION_SHADER_PATH,
        ))
}

pub fn true_25d_stylization_shader_source_is_complete() -> bool {
    TRUE_25D_STYLIZATION_SHADER_SOURCE.contains("@fragment")
        && TRUE_25D_STYLIZATION_SHADER_SOURCE.contains("toon_quantize")
        && TRUE_25D_STYLIZATION_SHADER_SOURCE.contains("sobel_depth")
        && TRUE_25D_STYLIZATION_SHADER_SOURCE.contains("sobel_luma")
        && TRUE_25D_STYLIZATION_SHADER_SOURCE.contains("texture_depth_multisampled_2d")
        && TRUE_25D_STYLIZATION_SHADER_SOURCE.contains("pixel_grid")
}

fn init_true_25d_stylization_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    pipeline_cache: Res<PipelineCache>,
    shader_handle: Res<GraphicalTrue25dStylizationShaderHandle>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "true25d_stylization_postprocess_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_depth_2d_multisampled(),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<GraphicalTrue25dStylizationSettings>(true),
            ),
        ),
    );
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("true25d_stylization_postprocess_sampler"),
        ..default()
    });
    let pipeline_id = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("true25d_stylization_postprocess_pipeline".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: shader_handle.0.clone(),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::bevy_default(),
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        ..default()
    });
    commands.insert_resource(True25dStylizationPipeline {
        layout,
        sampler,
        pipeline_id,
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Resource)]
pub struct GraphicalTrue25dRenderBypassSummaryResource {
    pub locked_camera_viewport_width_units: f32,
    pub locked_camera_viewport_height_units: f32,
    pub renderable_true_25d_entities: usize,
    pub visible_true_25d_entities: usize,
    pub bypassed_true_25d_entities: usize,
    pub ledger_only_true_25d_assets: usize,
    pub active_headless_chunks: usize,
    pub materialized_headless_tiles: usize,
    pub render_frame_hz: u32,
    pub fixed_headless_tick_hz: u32,
    pub presentation_headless_tick_hz: u32,
    pub authoritative_sim_tick_hz: u32,
    pub offscreen_presentation_draw_call_budget: u16,
    pub offscreen_animation_update_budget: u16,
    pub offscreen_render_extraction_bypassed: bool,
    pub offscreen_regions_zero_draw_calls: bool,
    pub headless_updates_continue: bool,
    pub procedural_generation_without_rendering: bool,
    pub authoritative_scheduler_unchanged: bool,
    pub no_action_authority: bool,
}

#[derive(Debug, Clone, Resource)]
struct GraphicalTrue25dNativeAssets {
    terrain_mesh: Handle<Mesh>,
    billboard_plane_mesh: Handle<Mesh>,
    crystal_mesh: Handle<Mesh>,
    rock_mesh: Handle<Mesh>,
    ring_mesh: Handle<Mesh>,
    creature_body_mesh: Handle<Mesh>,
    creature_eye_mesh: Handle<Mesh>,
    creature_antenna_mesh: Handle<Mesh>,
    food_mesh: Handle<Mesh>,
    reed_mesh: Handle<Mesh>,
    fog_material: Handle<StandardMaterial>,
    creature_material: Handle<StandardMaterial>,
    creature_hurt_material: Handle<StandardMaterial>,
    creature_eye_material: Handle<StandardMaterial>,
    creature_glow_material: Handle<StandardMaterial>,
    food_material: Handle<StandardMaterial>,
    hazard_crystal_material: Handle<StandardMaterial>,
    rock_material: Handle<StandardMaterial>,
    reed_material: Handle<StandardMaterial>,
    selection_material: Handle<StandardMaterial>,
    contact_shadow_material: Handle<StandardMaterial>,
    hunger_glow_material: Handle<StandardMaterial>,
    pain_spike_material: Handle<StandardMaterial>,
    stress_desaturation_material: Handle<StandardMaterial>,
    energy_trail_material: Handle<StandardMaterial>,
    sleep_bloom_material: Handle<StandardMaterial>,
    learning_biolume_material: Handle<StandardMaterial>,
}

#[derive(Debug, Clone, Resource)]
struct GraphicalTrue25dSceneAssets {
    creature_idle: Handle<bevy::scene::Scene>,
    creature_hurt: Handle<bevy::scene::Scene>,
    selection_ring: Handle<bevy::scene::Scene>,
    food: Handle<bevy::scene::Scene>,
    hazard: Handle<bevy::scene::Scene>,
    rock_obstacle: Handle<bevy::scene::Scene>,
    plant_prop: Handle<bevy::scene::Scene>,
    terrain_grass_island: Handle<bevy::scene::Scene>,
    terrain_soil_island: Handle<bevy::scene::Scene>,
    terrain_resource_grove: Handle<bevy::scene::Scene>,
    terrain_hazard_pressure: Handle<bevy::scene::Scene>,
    terrain_stone_island: Handle<bevy::scene::Scene>,
    terrain_water_cell: Handle<bevy::scene::Scene>,
    terrain_sand_island: Handle<bevy::scene::Scene>,
    fog_of_war_cell: Handle<bevy::scene::Scene>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub struct GraphicalTrue25dGltfScene {
    pub role: &'static str,
    pub scene_path: &'static str,
    pub display_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalTrue25dScaleNormalized {
    pub applied_scale: f32,
    pub min_scale: f32,
    pub max_scale: f32,
    pub authoring_space_clamped: bool,
}

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
    pub chunk_tile_size: i32,
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
    pub seed: u64,
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
    pub fogged_pixels: u32,
    pub active_chunk_count: usize,
    pub dark_gap_pixels: u32,
    pub generated_from_procedural_sampler: bool,
    pub generated_from_alpha_art_tiles: bool,
    pub rendered_from_preprocessed_ground_tile: bool,
    pub sampler_repeat_wrapped: bool,
    pub synchronous_texture_generation: bool,
    pub texture_source_path: &'static str,
    pub terrain_tile_source_count: u32,
    pub fog_of_war_applied: bool,
    pub primary_player_surface: bool,
    pub display_only: bool,
    pub active_chunk_signature: u64,
    pub refresh_count: u64,
    pub last_creature_anchor_count: usize,
    pub last_materialized_tile_count: usize,
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
pub(crate) const CA37_EXPLORATION_CAMERA_ZOOM: f32 = 1.0;
const CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES: i32 = 128;
const CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES: i32 = 72;
const CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE: u32 = 12;
const CA44A_PLAYER_WORLD_BACKDROP_WIDTH: f32 =
    CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as f32 * GRAPHICAL_WORLD_SCALE;
const CA44A_PLAYER_WORLD_BACKDROP_HEIGHT: f32 =
    CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as f32 * GRAPHICAL_WORLD_SCALE;
const TRUE_25D_VIEWPORT_VERTICAL_UNITS: f32 = 10.0;
const TRUE_25D_SIM_TO_VIEW_SCALE: f32 = 0.25;
const TRUE_25D_ACTIVE_CHUNK_SIM_WIDTH: f32 = 88.0;
const TRUE_25D_ACTIVE_CHUNK_SIM_DEPTH: f32 = 56.0;
const TRUE_25D_GROUND_WIDTH: f32 = TRUE_25D_ACTIVE_CHUNK_SIM_WIDTH * TRUE_25D_SIM_TO_VIEW_SCALE;
const TRUE_25D_GROUND_DEPTH: f32 = TRUE_25D_ACTIVE_CHUNK_SIM_DEPTH * TRUE_25D_SIM_TO_VIEW_SCALE;
const TRUE_25D_GROUND_UV_SPAN_X: f32 = 1.0;
const TRUE_25D_GROUND_UV_SPAN_Z: f32 = 1.0;
const TRUE_25D_RUNTIME_BIOME_TEXTURE_PATH: &str = "runtime-generated-seeded-biome-map";
pub const TRUE_25D_STYLIZATION_SHADER_PATH: &str =
    "crates/alife_gpu_backend/shaders/true25d_stylization_postprocess.wgsl";
pub const TRUE_25D_STYLIZATION_SHADER_SOURCE: &str =
    include_str!("../../alife_gpu_backend/shaders/true25d_stylization_postprocess.wgsl");
const TRUE_25D_STYLIZATION_PIXEL_GRID: Vec2 = Vec2::new(320.0, 240.0);
const TRUE_25D_STYLIZATION_TOON_BANDS: f32 = 4.0;
const TRUE_25D_STYLIZATION_OUTLINE_THRESHOLD: f32 = 0.012;
const TRUE_25D_STYLIZATION_OUTLINE_STRENGTH: f32 = 0.92;
const TRUE_25D_STYLIZATION_DEPTH_OUTLINE_STRENGTH: f32 = 1.0;
const TRUE_25D_MIN_NORMALIZED_SCALE: f32 = 0.08;
const TRUE_25D_MAX_NORMALIZED_SCALE: f32 = 1.0;
const TRUE_25D_VIEWPORT_ASPECT: f32 = 16.0 / 9.0;
const TRUE_25D_VIEWPORT_RENDER_MARGIN_UNITS: f32 = 0.65;
const TRUE_25D_VISIBLE_PRESENTATION_DRAW_CALL_BUDGET: u16 = 1;
const TRUE_25D_OFFSCREEN_PRESENTATION_DRAW_CALL_BUDGET: u16 = 0;
const TRUE_25D_OFFSCREEN_ANIMATION_UPDATE_BUDGET: u16 = 0;

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
                file_path: graphical_playground_asset_root(),
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
    .add_plugins((AlifeBevyAdapterPlugin, True25dStylizationPostProcessPlugin))
    .insert_resource(ClearColor(Color::srgb(0.110, 0.175, 0.105)))
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
    if summary.view_mode == GraphicalPlaygroundViewMode::Player {
        spawn_true_25d_neurochemical_visual_feedback(
            &mut app,
            &inspector,
            &gpu_telemetry.telemetry,
        );
    }
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
        });
    add_graphical_runtime_update_systems(&mut app);
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

fn graphical_playground_asset_root() -> String {
    crate::ca12_workspace_root()
        .join("crates/alife_game_app/assets")
        .to_string_lossy()
        .to_string()
}

fn add_graphical_runtime_update_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            handle_graphical_runtime_input,
            handle_graphical_camera_selection_input,
            handle_graphical_population_cycle_input,
            handle_graphical_mouse_selection,
            advance_graphical_runtime_loop,
            update_graphical_procedural_terrain_field,
            update_graphical_runtime_procedural_biome_map,
            normalize_true_25d_gltf_asset_scales,
            enforce_true_25d_camera_contract,
            update_true_25d_neurochemical_visual_feedback,
            update_true_25d_viewport_render_bypass,
            update_graphical_camera_transform,
            update_graphical_selection_ring,
            update_graphical_gpu_visual_cues,
            update_graphical_feedback_pulses,
            update_graphical_intent_feedback,
        )
            .chain(),
    )
    .add_systems(
        Update,
        (
            update_graphical_runtime_overlay,
            update_graphical_inspector_overlay,
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
}

fn add_graphical_runtime_core_update_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            advance_graphical_runtime_loop,
            update_graphical_procedural_terrain_field,
            update_graphical_runtime_procedural_biome_map,
            normalize_true_25d_gltf_asset_scales,
            enforce_true_25d_camera_contract,
            update_true_25d_neurochemical_visual_feedback,
            update_true_25d_viewport_render_bypass,
            update_graphical_camera_transform,
            update_graphical_selection_ring,
            update_graphical_gpu_visual_cues,
            update_graphical_feedback_pulses,
            update_graphical_intent_feedback,
        )
            .chain(),
    );
}

fn ca37_graphical_default_camera_state(
    inspector: &CreatureInspectorSnapshot,
    world_art: Option<&Ca37WorldArtStyleSummary>,
) -> Result<CameraNavigationState, GameAppShellError> {
    if world_art
        .map(|summary| summary.local_viewport_is_smaller_than_map)
        .unwrap_or(false)
    {
        let mut state = inspector.camera;
        state.zoom = CA37_EXPLORATION_CAMERA_ZOOM;
        state.validate()?;
        return Ok(state);
    }
    let state = inspector.camera;
    state.validate()?;
    Ok(state)
}

fn true_25d_native_assets(app: &mut App) -> GraphicalTrue25dNativeAssets {
    if let Some(existing) = app.world().get_resource::<GraphicalTrue25dNativeAssets>() {
        return existing.clone();
    }
    if !app.world().contains_resource::<Assets<Mesh>>() {
        app.init_resource::<Assets<Mesh>>();
    }
    if !app.world().contains_resource::<Assets<StandardMaterial>>() {
        app.init_resource::<Assets<StandardMaterial>>();
    }

    let (
        terrain_mesh,
        billboard_plane_mesh,
        crystal_mesh,
        rock_mesh,
        ring_mesh,
        creature_body_mesh,
        creature_eye_mesh,
        creature_antenna_mesh,
        food_mesh,
        reed_mesh,
    ) = {
        let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
        (
            meshes.add(Circle::new(1.0)),
            meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
            meshes.add(Cone::new(0.42, 1.28)),
            meshes.add(Sphere::new(1.0).mesh().ico(1).expect("valid rock sphere")),
            meshes.add(Circle::new(1.0).to_ring(0.26)),
            meshes.add(Sphere::new(1.0).mesh().ico(2).expect("valid ico sphere")),
            meshes.add(Sphere::new(1.0).mesh().ico(1).expect("valid eye sphere")),
            meshes.add(Cuboid::new(0.06, 0.52, 0.06)),
            meshes.add(Cone::new(0.26, 0.48)),
            meshes.add(Capsule3d::new(0.06, 0.54)),
        )
    };

    let (
        fog_material,
        creature_material,
        creature_hurt_material,
        creature_eye_material,
        creature_glow_material,
        food_material,
        hazard_crystal_material,
        rock_material,
        reed_material,
        selection_material,
        contact_shadow_material,
        hunger_glow_material,
        pain_spike_material,
        stress_desaturation_material,
        energy_trail_material,
        sleep_bloom_material,
        learning_biolume_material,
    ) = {
        let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        (
            true_25d_material(&mut materials, Color::srgba(0.02, 0.035, 0.032, 0.08), 0.08),
            true_25d_material(&mut materials, Color::srgb(0.12, 0.82, 0.90), 1.0),
            true_25d_material(&mut materials, Color::srgb(0.52, 0.88, 0.95), 1.0),
            true_25d_material(&mut materials, Color::srgb(0.015, 0.045, 0.055), 1.0),
            true_25d_material(&mut materials, Color::srgba(0.56, 1.0, 0.88, 0.78), 0.78),
            true_25d_material(&mut materials, Color::srgb(0.32, 0.94, 0.26), 1.0),
            true_25d_material(&mut materials, Color::srgb(1.0, 0.11, 0.14), 1.0),
            true_25d_material(&mut materials, Color::srgb(0.54, 0.55, 0.50), 1.0),
            true_25d_material(&mut materials, Color::srgb(0.42, 0.96, 0.32), 1.0),
            true_25d_material(&mut materials, Color::srgba(0.88, 1.0, 0.28, 0.86), 0.86),
            true_25d_material(
                &mut materials,
                Color::srgba(0.005, 0.010, 0.008, 0.30),
                0.30,
            ),
            true_25d_material(&mut materials, Color::srgba(1.0, 0.78, 0.24, 0.74), 0.74),
            true_25d_material(&mut materials, Color::srgba(1.0, 0.18, 0.22, 0.86), 0.86),
            true_25d_material(&mut materials, Color::srgba(0.58, 0.60, 0.66, 0.54), 0.54),
            true_25d_material(&mut materials, Color::srgba(0.24, 0.78, 1.0, 0.62), 0.62),
            true_25d_material(&mut materials, Color::srgba(0.46, 0.46, 1.0, 0.58), 0.58),
            true_25d_material(&mut materials, Color::srgba(0.38, 1.0, 0.82, 0.72), 0.72),
        )
    };

    let handles = GraphicalTrue25dNativeAssets {
        terrain_mesh,
        billboard_plane_mesh,
        crystal_mesh,
        rock_mesh,
        ring_mesh,
        creature_body_mesh,
        creature_eye_mesh,
        creature_antenna_mesh,
        food_mesh,
        reed_mesh,
        fog_material,
        creature_material,
        creature_hurt_material,
        creature_eye_material,
        creature_glow_material,
        food_material,
        hazard_crystal_material,
        rock_material,
        reed_material,
        selection_material,
        contact_shadow_material,
        hunger_glow_material,
        pain_spike_material,
        stress_desaturation_material,
        energy_trail_material,
        sleep_bloom_material,
        learning_biolume_material,
    };
    app.insert_resource(handles.clone());
    handles
}

fn true_25d_material(
    materials: &mut Assets<StandardMaterial>,
    base_color: Color,
    alpha: f32,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color,
        unlit: true,
        alpha_mode: if alpha < 0.995 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        },
        cull_mode: None,
        perceptual_roughness: 0.82,
        ..default()
    })
}

fn true_25d_scene_assets(app: &mut App) -> Option<GraphicalTrue25dSceneAssets> {
    if let Some(existing) = app.world().get_resource::<GraphicalTrue25dSceneAssets>() {
        return Some(existing.clone());
    }
    let asset_server = app.world().get_resource::<AssetServer>()?.clone();
    let load_scene =
        |path: &'static str| asset_server.load(GltfAssetLabel::Scene(0).from_asset(path));
    let handles = GraphicalTrue25dSceneAssets {
        creature_idle: load_scene("true_25d_alpha_v1/creature_idle.glb"),
        creature_hurt: load_scene("true_25d_alpha_v1/creature_hurt.glb"),
        selection_ring: load_scene("true_25d_alpha_v1/selection_ring.glb"),
        food: load_scene("true_25d_alpha_v1/food_pod.glb"),
        hazard: load_scene("true_25d_alpha_v1/hazard_crystal.glb"),
        rock_obstacle: load_scene("true_25d_alpha_v1/rock_cluster.glb"),
        plant_prop: load_scene("true_25d_alpha_v1/bio_reed_prop.glb"),
        terrain_grass_island: load_scene("true_25d_alpha_v1/terrain_grass_island.glb"),
        terrain_soil_island: load_scene("true_25d_alpha_v1/terrain_soil_island.glb"),
        terrain_resource_grove: load_scene("true_25d_alpha_v1/terrain_resource_grove.glb"),
        terrain_hazard_pressure: load_scene("true_25d_alpha_v1/terrain_hazard_pressure.glb"),
        terrain_stone_island: load_scene("true_25d_alpha_v1/terrain_stone_island.glb"),
        terrain_water_cell: load_scene("true_25d_alpha_v1/terrain_water_cell.glb"),
        terrain_sand_island: load_scene("true_25d_alpha_v1/terrain_sand_island.glb"),
        fog_of_war_cell: load_scene("true_25d_alpha_v1/fog_of_war_cell.glb"),
    };
    app.insert_resource(handles.clone());
    Some(handles)
}

fn true_25d_scene_for_role(
    scenes: &GraphicalTrue25dSceneAssets,
    role: &'static str,
) -> (Handle<bevy::scene::Scene>, &'static str) {
    match role {
        "creature-idle" => (
            scenes.creature_idle.clone(),
            "true_25d_alpha_v1/creature_idle.glb",
        ),
        "creature-hurt" => (
            scenes.creature_hurt.clone(),
            "true_25d_alpha_v1/creature_hurt.glb",
        ),
        "selection-ring" => (
            scenes.selection_ring.clone(),
            "true_25d_alpha_v1/selection_ring.glb",
        ),
        "food" => (scenes.food.clone(), "true_25d_alpha_v1/food_pod.glb"),
        "hazard" => (
            scenes.hazard.clone(),
            "true_25d_alpha_v1/hazard_crystal.glb",
        ),
        "rock-obstacle" => (
            scenes.rock_obstacle.clone(),
            "true_25d_alpha_v1/rock_cluster.glb",
        ),
        "plant-prop" => (
            scenes.plant_prop.clone(),
            "true_25d_alpha_v1/bio_reed_prop.glb",
        ),
        "terrain-soil-island" => (
            scenes.terrain_soil_island.clone(),
            "true_25d_alpha_v1/terrain_soil_island.glb",
        ),
        "terrain-resource-grove" => (
            scenes.terrain_resource_grove.clone(),
            "true_25d_alpha_v1/terrain_resource_grove.glb",
        ),
        "terrain-hazard-pressure" => (
            scenes.terrain_hazard_pressure.clone(),
            "true_25d_alpha_v1/terrain_hazard_pressure.glb",
        ),
        "terrain-stone-island" => (
            scenes.terrain_stone_island.clone(),
            "true_25d_alpha_v1/terrain_stone_island.glb",
        ),
        "terrain-water-cell" => (
            scenes.terrain_water_cell.clone(),
            "true_25d_alpha_v1/terrain_water_cell.glb",
        ),
        "terrain-sand-island" => (
            scenes.terrain_sand_island.clone(),
            "true_25d_alpha_v1/terrain_sand_island.glb",
        ),
        "fog-of-war-cell" => (
            scenes.fog_of_war_cell.clone(),
            "true_25d_alpha_v1/fog_of_war_cell.glb",
        ),
        _ => (
            scenes.terrain_grass_island.clone(),
            "true_25d_alpha_v1/terrain_grass_island.glb",
        ),
    }
}

fn spawn_true_25d_player_view_layer(
    app: &mut App,
    presentation: &VisibleWorldPresentation,
    seed: u64,
    world_art: Option<&Ca37WorldArtStyleSummary>,
) -> Result<(), GameAppShellError> {
    let manifest =
        crate::validate_true_25d_asset_manifest(crate::default_true_25d_asset_manifest_path())?;
    app.insert_resource(GraphicalTrue25dPresentationResource {
        asset_manifest: manifest,
        versioned_gltf_pack_validated: true,
        runtime_gltf_scene_rendering: app.world().contains_resource::<AssetServer>(),
        runtime_native_low_poly_fallback: !app.world().contains_resource::<AssetServer>(),
        fixed_orthographic_camera: true,
        preprocessed_repeating_ground_plane: false,
        synchronous_runtime_ground_texture_generation: true,
        ground_texture_path: TRUE_25D_RUNTIME_BIOME_TEXTURE_PATH,
        toon_bands: 4,
        sobel_outline_contract: true,
        pixel_step_filter_contract: true,
        procedural_micro_ecology_chunks: true,
        offscreen_headless_chunks: true,
        viewport_render_bypass: true,
        offscreen_zero_draw_call_contract: true,
        no_action_authority: true,
    });
    app.insert_resource(GraphicalTrue25dStylizationRenderPassResource {
        shader_path: TRUE_25D_STYLIZATION_SHADER_PATH,
        shader_source_embedded: true_25d_stylization_shader_source_is_complete(),
        runtime_render_graph_registered: app
            .world()
            .contains_resource::<GraphicalTrue25dStylizationShaderHandle>(),
        attached_to_player_camera: true,
        pixel_grid_width: TRUE_25D_STYLIZATION_PIXEL_GRID.x as u32,
        pixel_grid_height: TRUE_25D_STYLIZATION_PIXEL_GRID.y as u32,
        toon_bands: TRUE_25D_STYLIZATION_TOON_BANDS as u8,
        depth_sobel_outline: true,
        luminance_sobel_fallback: true,
        low_resolution_pixel_step_filter: true,
        display_only: true,
        no_action_authority: true,
    });

    let native_assets = true_25d_native_assets(app);
    let scene_assets = true_25d_scene_assets(app);

    app.world_mut().spawn((
        Name::new("A-Life true 2.5D orthographic camera"),
        Camera3d {
            depth_texture_usages: (TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING)
                .into(),
            ..default()
        },
        Camera {
            order: 0,
            ..default()
        },
        Tonemapping::None,
        true_25d_camera_projection(),
        true_25d_camera_transform(),
        GraphicalTrue25dCamera {
            orthographic_locked: true,
            pitch_degrees: -45.0,
            yaw_degrees: 0.0,
        },
        GraphicalTrue25dStylizationSettings::default(),
    ));
    app.world_mut().spawn((
        Name::new("A-Life true 2.5D toon key light"),
        DirectionalLight {
            illuminance: 12_500.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-5.0, 8.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    spawn_true_25d_terrain_island_patch(
        app,
        &native_assets,
        scene_assets.as_ref(),
        seed,
        presentation,
        world_art,
    );
    for object in &presentation.objects {
        spawn_true_25d_object_scene(app, &native_assets, scene_assets.as_ref(), object)?;
    }
    normalize_existing_true_25d_gltf_asset_scales(app.world_mut());
    apply_existing_true_25d_viewport_render_bypass(app.world_mut());
    Ok(())
}

fn spawn_true_25d_terrain_island_patch(
    app: &mut App,
    native_assets: &GraphicalTrue25dNativeAssets,
    scene_assets: Option<&GraphicalTrue25dSceneAssets>,
    seed: u64,
    presentation: &VisibleWorldPresentation,
    world_art: Option<&Ca37WorldArtStyleSummary>,
) {
    spawn_true_25d_foundation_regions(
        app,
        native_assets,
        scene_assets,
        seed,
        presentation,
        world_art,
    );

    for index in 0..72 {
        let x = true_25d_region_coord(seed, index, 0xA11F, TRUE_25D_GROUND_WIDTH * 0.46);
        let z = true_25d_region_coord(seed, index, 0xC0DE, TRUE_25D_GROUND_DEPTH * 0.45);
        let ix = (x / TRUE_25D_SIM_TO_VIEW_SCALE).round() as i32;
        let iz = (z / TRUE_25D_SIM_TO_VIEW_SCALE).round() as i32;
        let material_id = true_25d_seeded_material_id(seed, ix, iz);
        let role = true_25d_terrain_role_for_material(material_id);
        spawn_true_25d_terrain_ledger(
            app,
            role,
            "true-25d-procedural-micro-ecology-ledger",
            index as usize,
        );

        spawn_true_25d_chunk_dressing(
            app,
            native_assets,
            scene_assets,
            seed,
            ix,
            iz,
            material_id,
            x,
            z,
        );
    }

    for index in 0..22 {
        let side = index % 4;
        let offset = true_25d_region_unit(seed, index, 0xF06) * 2.0 - 1.0;
        let (x, z) = match side {
            0 => (
                -TRUE_25D_GROUND_WIDTH * 0.56,
                offset * TRUE_25D_GROUND_DEPTH * 0.48,
            ),
            1 => (
                TRUE_25D_GROUND_WIDTH * 0.56,
                offset * TRUE_25D_GROUND_DEPTH * 0.48,
            ),
            2 => (
                offset * TRUE_25D_GROUND_WIDTH * 0.56,
                -TRUE_25D_GROUND_DEPTH * 0.56,
            ),
            _ => (
                offset * TRUE_25D_GROUND_WIDTH * 0.56,
                TRUE_25D_GROUND_DEPTH * 0.56,
            ),
        };
        if let Some(scenes) = scene_assets {
            let (scene, scene_path) = true_25d_scene_for_role(scenes, "fog-of-war-cell");
            app.world_mut().spawn((
                Name::new(format!(
                    "A-Life true 2.5D organic fog-of-war region {index}"
                )),
                SceneRoot(scene),
                Transform::from_xyz(x, 0.08, z).with_scale(true_25d_normalized_scale(0.34)),
                GraphicalTrue25dGltfScene {
                    role: "fog-of-war-cell",
                    scene_path,
                    display_only: true,
                },
                GraphicalTrue25dAsset {
                    role: "fog-of-war-cell",
                    stable_id: None,
                    display_only: true,
                },
                GraphicalProductionArtLayer {
                    role: "true-25d-fog-of-war",
                    display_only: true,
                },
            ));
        } else {
            app.world_mut().spawn((
                Name::new(format!(
                    "A-Life true 2.5D organic fog-of-war fallback {index}"
                )),
                Mesh3d(native_assets.terrain_mesh.clone()),
                MeshMaterial3d(native_assets.fog_material.clone()),
                Transform::from_xyz(x, 0.08, z)
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                    .with_scale(true_25d_normalized_scale(0.34)),
                GraphicalTrue25dAsset {
                    role: "fog-of-war-cell",
                    stable_id: None,
                    display_only: true,
                },
                GraphicalProductionArtLayer {
                    role: "true-25d-fog-of-war",
                    display_only: true,
                },
            ));
        }
    }
}

fn spawn_true_25d_foundation_regions(
    app: &mut App,
    native_assets: &GraphicalTrue25dNativeAssets,
    scene_assets: Option<&GraphicalTrue25dSceneAssets>,
    seed: u64,
    presentation: &VisibleWorldPresentation,
    world_art: Option<&Ca37WorldArtStyleSummary>,
) {
    spawn_true_25d_textured_micro_ecology_ground(app, seed, presentation, world_art);

    const FOUNDATION: [(&str, f32, f32, f32, f32, f32, f32); 18] = [
        ("terrain-soil-island", -4.80, 2.25, 0.92, 0.92, -0.42, 0.018),
        ("terrain-soil-island", -3.45, 1.45, 0.96, 0.96, -0.34, 0.019),
        ("terrain-soil-island", -2.10, 0.70, 0.96, 0.96, -0.24, 0.020),
        ("terrain-soil-island", -0.70, 0.12, 0.92, 0.92, -0.14, 0.021),
        ("terrain-soil-island", 0.85, -0.36, 0.94, 0.94, 0.04, 0.022),
        ("terrain-soil-island", 2.38, -0.94, 0.90, 0.90, 0.18, 0.023),
        ("terrain-soil-island", 3.70, -1.66, 0.82, 0.82, 0.32, 0.024),
        (
            "terrain-resource-grove",
            -4.65,
            -1.42,
            0.88,
            0.88,
            -0.16,
            0.022,
        ),
        (
            "terrain-resource-grove",
            -2.15,
            -2.70,
            0.94,
            0.94,
            0.24,
            0.024,
        ),
        (
            "terrain-hazard-pressure",
            4.55,
            1.10,
            1.00,
            1.00,
            -0.08,
            0.026,
        ),
        (
            "terrain-hazard-pressure",
            5.95,
            2.08,
            0.88,
            0.88,
            0.36,
            0.028,
        ),
        ("terrain-stone-island", 3.80, 3.00, 0.92, 0.92, 0.20, 0.024),
        ("terrain-stone-island", 5.20, 3.62, 0.78, 0.78, -0.20, 0.025),
        ("terrain-water-cell", -5.90, 3.28, 0.88, 0.88, -0.32, 0.022),
        ("terrain-water-cell", -6.55, 2.30, 0.78, 0.78, 0.12, 0.021),
        ("terrain-sand-island", -5.32, 1.58, 0.70, 0.70, 0.18, 0.020),
        ("terrain-grass-island", 0.45, 1.86, 0.94, 0.94, -0.08, 0.018),
        ("terrain-grass-island", 1.82, 1.82, 0.80, 0.80, 0.22, 0.019),
    ];

    for (index, (role, x, z, width, depth, rotation, y)) in FOUNDATION.iter().enumerate() {
        let jitter_x = (true_25d_region_unit(seed, index as i32, 0xB45E) - 0.5) * 0.18;
        let jitter_z = (true_25d_region_unit(seed, index as i32, 0xC411) - 0.5) * 0.14;
        let ix = ((*x + jitter_x) / TRUE_25D_SIM_TO_VIEW_SCALE).round() as i32;
        let iz = ((*z + jitter_z) / TRUE_25D_SIM_TO_VIEW_SCALE).round() as i32;
        let material_id = true_25d_material_id_for_role(role);
        spawn_true_25d_terrain_ledger(app, role, "true-25d-procedural-foundation-ledger", index);
        spawn_true_25d_chunk_dressing(
            app,
            native_assets,
            scene_assets,
            seed,
            ix,
            iz,
            material_id,
            *x + jitter_x,
            *z + jitter_z,
        );
        let _ = (width, depth, rotation, y);
    }
}

fn spawn_true_25d_terrain_ledger(
    app: &mut App,
    role: &'static str,
    layer_role: &'static str,
    index: usize,
) {
    app.world_mut().spawn((
        Name::new(format!(
            "A-Life true 2.5D virtual terrain ledger {index} {role}"
        )),
        GraphicalTrue25dAsset {
            role,
            stable_id: None,
            display_only: true,
        },
        GraphicalProductionArtLayer {
            role: layer_role,
            display_only: true,
        },
    ));
}

fn spawn_true_25d_textured_micro_ecology_ground(
    app: &mut App,
    seed: u64,
    presentation: &VisibleWorldPresentation,
    world_art: Option<&Ca37WorldArtStyleSummary>,
) {
    let mut field = true_25d_ground_texture_field(seed);
    let config = world_art
        .map(ca44a_procedural_world_config)
        .unwrap_or_else(|| ProceduralWorldConfig::with_seed(seed));
    let anchors = ca44a_procedural_world_anchors_from_presentation(presentation);
    if let Ok(activation) = activate_procedural_chunks_around_creatures(config, &anchors) {
        field = if let Some(summary) = world_art {
            GraphicalProceduralTerrainFieldResource::new(summary, &activation)
        } else {
            true_25d_ground_texture_field_from_activation(seed, config, &activation)
        };
        true_25d_materialize_terrain_chunk_ledger(&mut field, &activation);
        if let Ok(content) = generate_procedural_world_content(config, &activation) {
            field.record_content_report(&content);
            ca44a_spawn_procedural_world_content_app(
                app,
                &mut field,
                None,
                GraphicalPlaygroundViewMode::Player,
                false,
                &content,
            );
        }
    }
    if !app.world().contains_resource::<Assets<Image>>() {
        app.init_resource::<Assets<Image>>();
    }
    if !app.world().contains_resource::<Assets<Mesh>>() {
        app.init_resource::<Assets<Mesh>>();
    }
    if !app.world().contains_resource::<Assets<StandardMaterial>>() {
        app.init_resource::<Assets<StandardMaterial>>();
    }

    let mesh =
        app.world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(true_25d_repeating_ground_plane_mesh(
                TRUE_25D_GROUND_WIDTH,
                TRUE_25D_GROUND_DEPTH,
                TRUE_25D_GROUND_UV_SPAN_X,
                TRUE_25D_GROUND_UV_SPAN_Z,
            ));
    let (image, metrics) = ca44a_generate_runtime_procedural_biome_map(seed, &field);
    let texture_width_px = image.texture_descriptor.size.width;
    let texture_height_px = image.texture_descriptor.size.height;
    let image_handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color_texture: Some(image_handle),
            base_color: Color::srgb(0.74, 0.80, 0.68),
            unlit: true,
            cull_mode: None,
            perceptual_roughness: 0.86,
            ..default()
        });

    app.world_mut().spawn((
        Name::new("A-Life true 2.5D seeded biome ground substrate"),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -0.02, 0.0),
        GraphicalTrue25dGroundPlane {
            texture_path: TRUE_25D_RUNTIME_BIOME_TEXTURE_PATH,
            width_world_units: TRUE_25D_GROUND_WIDTH,
            depth_world_units: TRUE_25D_GROUND_DEPTH,
            uv_repeat_x: TRUE_25D_GROUND_UV_SPAN_X,
            uv_repeat_z: TRUE_25D_GROUND_UV_SPAN_Z,
            sampler_repeat_wrapped: false,
            static_primitive_plane: true,
            synchronous_runtime_texture_generation: true,
        },
        GraphicalTrue25dAsset {
            role: "terrain-seeded-biome-ground-plane",
            stable_id: None,
            display_only: true,
        },
        GraphicalProductionArtLayer {
            role: "true-25d-seeded-biome-ground-plane",
            display_only: true,
        },
        GraphicalRuntimeProceduralBiomeMap {
            seed,
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
            fogged_pixels: metrics.fogged_pixels,
            active_chunk_count: field.active_world_chunks.len(),
            dark_gap_pixels: metrics.dark_gap_pixels,
            generated_from_procedural_sampler: true,
            generated_from_alpha_art_tiles: metrics.alpha_art_tile_pixels > 0,
            rendered_from_preprocessed_ground_tile: false,
            sampler_repeat_wrapped: false,
            synchronous_texture_generation: true,
            texture_source_path: TRUE_25D_RUNTIME_BIOME_TEXTURE_PATH,
            terrain_tile_source_count: metrics.terrain_tile_source_count,
            fog_of_war_applied: metrics.fogged_pixels > 0,
            primary_player_surface: true,
            display_only: true,
            active_chunk_signature: ca44a_active_chunk_signature(&field),
            refresh_count: 0,
            last_creature_anchor_count: field.creature_anchor_count,
            last_materialized_tile_count: field.materialized_tiles.len(),
        },
    ));
    if let Some(handles) = app
        .world()
        .get_resource::<GraphicalAlphaArtHandles>()
        .cloned()
    {
        spawn_true_25d_alpha_painted_backdrop(app, &handles);
    }
    app.insert_resource(field);
}

fn spawn_true_25d_alpha_painted_backdrop(app: &mut App, handles: &GraphicalAlphaArtHandles) {
    let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(
        Plane3d::default()
            .mesh()
            .size(TRUE_25D_GROUND_WIDTH, TRUE_25D_GROUND_DEPTH),
    );
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color_texture: Some(handles.world_backdrop.clone()),
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.88),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            perceptual_roughness: 0.86,
            ..default()
        });
    app.world_mut().spawn((
        Name::new("A-Life true 2.5D painted biome art substrate"),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -0.015, 0.0),
        GraphicalTrue25dAsset {
            role: "terrain-painted-biome-art-substrate",
            stable_id: None,
            display_only: true,
        },
        GraphicalProductionArtLayer {
            role: "true-25d-painted-biome-art-substrate",
            display_only: true,
        },
    ));
}

fn true_25d_repeating_ground_plane_mesh(
    width: f32,
    depth: f32,
    repeat_x: f32,
    repeat_z: f32,
) -> Mesh {
    let mut mesh = Plane3d::default().mesh().size(width, depth).build();
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        vec![
            [0.0, 0.0],
            [repeat_x, 0.0],
            [0.0, repeat_z],
            [repeat_x, repeat_z],
        ],
    );
    mesh
}

fn true_25d_ground_texture_field(seed: u64) -> GraphicalProceduralTerrainFieldResource {
    let mut active_world_chunks = BTreeSet::new();
    for chunk_x in -5..=5 {
        for chunk_z in -3..=3 {
            active_world_chunks.insert((chunk_x, chunk_z));
        }
    }
    GraphicalProceduralTerrainFieldResource {
        seed,
        chunk_tile_size: 8,
        virtual_map_width_tiles: CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as usize,
        virtual_map_height_tiles: CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as usize,
        chunk_radius_x: 8,
        chunk_radius_z: 5,
        active_world_chunks,
        creature_anchor_count: 3,
        generated_without_rendering: true,
        materialized_tiles: BTreeSet::new(),
        materialized_content_stable_ids: BTreeSet::new(),
        materialized_chunk_count: 0,
        active_content_count: 0,
        procedural_content_generated_without_rendering: true,
        procedural_content_rendering_required: false,
        materialized_only_near_active_views: true,
    }
}

fn true_25d_ground_texture_field_from_activation(
    seed: u64,
    config: ProceduralWorldConfig,
    activation: &ProceduralChunkActivationReport,
) -> GraphicalProceduralTerrainFieldResource {
    GraphicalProceduralTerrainFieldResource {
        seed,
        chunk_tile_size: config.chunk_tile_size,
        virtual_map_width_tiles: config.virtual_width_tiles(),
        virtual_map_height_tiles: config.virtual_height_tiles(),
        chunk_radius_x: 14,
        chunk_radius_z: 10,
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

fn true_25d_materialize_terrain_chunk_ledger(
    field: &mut GraphicalProceduralTerrainFieldResource,
    activation: &ProceduralChunkActivationReport,
) {
    for (center_x, center_z, _) in
        ca44a_initial_procedural_terrain_centers(activation, field.chunk_tile_size)
    {
        field.materialized_chunk_count = field.materialized_chunk_count.saturating_add(1);
        for ix in center_x - field.chunk_radius_x..=center_x + field.chunk_radius_x {
            for iz in center_z - field.chunk_radius_z..=center_z + field.chunk_radius_z {
                if ca44a_virtual_tile_in_bounds(field, ix, iz) {
                    field.materialized_tiles.insert((ix, iz));
                }
            }
        }
    }
}

fn ca44a_launch_baseline_seed(launch: &GraphicalPlaygroundLaunchConfig) -> u64 {
    launch
        .app_launch
        .fixture_root
        .to_string_lossy()
        .bytes()
        .fold(0xCA44_A25D_u64, |acc, byte| {
            acc.rotate_left(5) ^ u64::from(byte)
        })
}

fn spawn_true_25d_chunk_dressing(
    app: &mut App,
    native_assets: &GraphicalTrue25dNativeAssets,
    scene_assets: Option<&GraphicalTrue25dSceneAssets>,
    seed: u64,
    ix: i32,
    iz: i32,
    material_id: &str,
    x: f32,
    z: f32,
) {
    let hash = ca37_seeded_terrain_hash(seed, ix, iz);
    let mut dressing: Vec<(&'static str, f32, f32, f32)> = Vec::new();
    match material_id {
        "resource-grove" => {
            if hash.rem_euclid(3) != 0 {
                dressing.push(("food", 0.16, -0.08, 0.68));
            }
        }
        "hazard-pressure" => {
            dressing.push(("hazard", 0.10, -0.08, 0.78));
            if hash.rem_euclid(2) == 0 {
                dressing.push(("hazard", -0.24, 0.14, 0.58));
            }
        }
        "stone-dressing" => {
            dressing.push(("rock-obstacle", -0.10, 0.04, 0.70));
        }
        "water" => {
            if hash.rem_euclid(4) == 0 {
                dressing.push(("rock-obstacle", 0.16, 0.18, 0.34));
            }
        }
        "sand" | "neutral-soil" => {
            if hash.rem_euclid(11) == 0 {
                dressing.push(("rock-obstacle", 0.16, -0.18, 0.34));
            }
        }
        _ => {}
    }
    for (role, ox, oz, scale) in dressing {
        let transform = Transform::from_xyz(x + ox * 0.34, 0.09, z + oz * 0.34)
            .with_scale(true_25d_normalized_scale(scale));
        if let Some(scenes) = scene_assets {
            let (scene, scene_path) = true_25d_scene_for_role(scenes, role);
            app.world_mut().spawn((
                Name::new(format!("A-Life true 2.5D glTF dressing {role} {ix}:{iz}")),
                SceneRoot(scene),
                transform,
                GraphicalTrue25dGltfScene {
                    role,
                    scene_path,
                    display_only: true,
                },
                GraphicalTrue25dAsset {
                    role,
                    stable_id: None,
                    display_only: true,
                },
                GraphicalProductionArtLayer {
                    role: "true-25d-procedural-dressing",
                    display_only: true,
                },
            ));
        } else {
            let (mesh, material) = true_25d_native_mesh_material_for_role(native_assets, role);
            app.world_mut().spawn((
                Name::new(format!("A-Life true 2.5D dressing {role} {ix}:{iz}")),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                transform,
                GraphicalTrue25dAsset {
                    role,
                    stable_id: None,
                    display_only: true,
                },
                GraphicalProductionArtLayer {
                    role: "true-25d-procedural-dressing",
                    display_only: true,
                },
            ));
        }
    }
}

fn true_25d_region_unit(seed: u64, index: i32, salt: u32) -> f32 {
    ca44a_runtime_pixel_hash(seed ^ u64::from(salt), index, salt as i32, 0, 0) as f32
        / u32::MAX as f32
}

fn true_25d_region_coord(seed: u64, index: i32, salt: u32, extent: f32) -> f32 {
    let unit = true_25d_region_unit(seed, index, salt);
    let cluster = ((index % 8) as f32 - 3.5) / 3.5;
    let clustered = (unit * 2.0 - 1.0) * 0.58 + cluster * 0.42;
    clustered.clamp(-1.0, 1.0) * extent
}

fn true_25d_normalized_scale(requested: f32) -> Vec3 {
    Vec3::splat(requested.clamp(TRUE_25D_MIN_NORMALIZED_SCALE, TRUE_25D_MAX_NORMALIZED_SCALE))
}

fn true_25d_normalize_transform_scale(
    transform: &mut Transform,
) -> GraphicalTrue25dScaleNormalized {
    let requested = transform
        .scale
        .x
        .abs()
        .max(transform.scale.y.abs())
        .max(transform.scale.z.abs());
    let applied_scale =
        requested.clamp(TRUE_25D_MIN_NORMALIZED_SCALE, TRUE_25D_MAX_NORMALIZED_SCALE);
    let authoring_space_clamped = (applied_scale - requested).abs() > f32::EPSILON
        || (transform.scale.x - transform.scale.y).abs() > f32::EPSILON
        || (transform.scale.y - transform.scale.z).abs() > f32::EPSILON;
    transform.scale = Vec3::splat(applied_scale);
    GraphicalTrue25dScaleNormalized {
        applied_scale,
        min_scale: TRUE_25D_MIN_NORMALIZED_SCALE,
        max_scale: TRUE_25D_MAX_NORMALIZED_SCALE,
        authoring_space_clamped,
    }
}

fn normalize_true_25d_gltf_asset_scales(
    mut commands: Commands,
    mut scenes: bevy::prelude::Query<
        (Entity, &mut Transform),
        (
            With<GraphicalTrue25dGltfScene>,
            Without<GraphicalTrue25dScaleNormalized>,
        ),
    >,
) {
    for (entity, mut transform) in &mut scenes {
        let receipt = true_25d_normalize_transform_scale(&mut transform);
        commands.entity(entity).insert(receipt);
    }
}

fn normalize_existing_true_25d_gltf_asset_scales(world: &mut bevy::prelude::World) {
    let mut receipts = Vec::new();
    {
        let mut query = world.query_filtered::<(Entity, &mut Transform), (
            With<GraphicalTrue25dGltfScene>,
            Without<GraphicalTrue25dScaleNormalized>,
        )>();
        for (entity, mut transform) in query.iter_mut(world) {
            let receipt = true_25d_normalize_transform_scale(&mut transform);
            receipts.push((entity, receipt));
        }
    }
    for (entity, receipt) in receipts {
        world.entity_mut(entity).insert(receipt);
    }
}

fn true_25d_camera_transform() -> Transform {
    Transform::from_xyz(0.0, 12.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y)
}

fn true_25d_camera_projection() -> Projection {
    Projection::from(OrthographicProjection {
        scaling_mode: ScalingMode::FixedVertical {
            viewport_height: TRUE_25D_VIEWPORT_VERTICAL_UNITS,
        },
        scale: 1.0,
        near: -100.0,
        far: 200.0,
        ..OrthographicProjection::default_3d()
    })
}

fn enforce_true_25d_camera_contract(
    mut cameras: bevy::prelude::Query<(&mut Transform, &mut Projection, &GraphicalTrue25dCamera)>,
) {
    for (mut transform, mut projection, camera) in &mut cameras {
        if !camera.orthographic_locked {
            continue;
        }
        *transform = true_25d_camera_transform();
        *projection = true_25d_camera_projection();
    }
}

fn true_25d_locked_camera_viewport_width_units() -> f32 {
    TRUE_25D_VIEWPORT_VERTICAL_UNITS * TRUE_25D_VIEWPORT_ASPECT
}

fn true_25d_inside_locked_camera_viewport(translation: Vec3) -> bool {
    let half_width =
        true_25d_locked_camera_viewport_width_units() * 0.5 + TRUE_25D_VIEWPORT_RENDER_MARGIN_UNITS;
    let half_depth = TRUE_25D_VIEWPORT_VERTICAL_UNITS * 0.5 + TRUE_25D_VIEWPORT_RENDER_MARGIN_UNITS;
    translation.x.abs() <= half_width && translation.z.abs() <= half_depth
}

fn true_25d_bypass_receipt(
    transform: Option<&Transform>,
    ground: Option<&GraphicalTrue25dGroundPlane>,
) -> Option<GraphicalTrue25dViewportRenderBypass> {
    let transform = transform?;
    let inside_locked_camera_viewport =
        ground.is_some() || true_25d_inside_locked_camera_viewport(transform.translation);
    let render_pass_bypassed = !inside_locked_camera_viewport;
    Some(GraphicalTrue25dViewportRenderBypass {
        inside_locked_camera_viewport,
        render_pass_bypassed,
        render_extraction_bypassed: render_pass_bypassed,
        presentation_draw_call_budget: if render_pass_bypassed {
            TRUE_25D_OFFSCREEN_PRESENTATION_DRAW_CALL_BUDGET
        } else {
            TRUE_25D_VISIBLE_PRESENTATION_DRAW_CALL_BUDGET
        },
        offscreen_animation_update_budget: if render_pass_bypassed {
            TRUE_25D_OFFSCREEN_ANIMATION_UPDATE_BUDGET
        } else {
            TRUE_25D_VISIBLE_PRESENTATION_DRAW_CALL_BUDGET
        },
        headless_update_continues: true,
        zero_draw_call_contract: render_pass_bypassed,
        display_only: true,
    })
}

fn true_25d_bypass_summary(
    renderable_true_25d_entities: usize,
    visible_true_25d_entities: usize,
    bypassed_true_25d_entities: usize,
    ledger_only_true_25d_assets: usize,
    field: Option<&GraphicalProceduralTerrainFieldResource>,
) -> GraphicalTrue25dRenderBypassSummaryResource {
    GraphicalTrue25dRenderBypassSummaryResource {
        locked_camera_viewport_width_units: true_25d_locked_camera_viewport_width_units(),
        locked_camera_viewport_height_units: TRUE_25D_VIEWPORT_VERTICAL_UNITS,
        renderable_true_25d_entities,
        visible_true_25d_entities,
        bypassed_true_25d_entities,
        ledger_only_true_25d_assets,
        active_headless_chunks: field
            .map(|field| field.active_world_chunks.len())
            .unwrap_or_default(),
        materialized_headless_tiles: field
            .map(|field| field.materialized_tiles.len())
            .unwrap_or_default(),
        render_frame_hz: CA13_TARGET_RENDER_FRAME_HZ,
        fixed_headless_tick_hz: CA13_FIXED_SIM_TICK_HZ,
        presentation_headless_tick_hz: CA13_TARGET_RENDER_FRAME_HZ,
        authoritative_sim_tick_hz: CA13_FIXED_SIM_TICK_HZ,
        offscreen_presentation_draw_call_budget: TRUE_25D_OFFSCREEN_PRESENTATION_DRAW_CALL_BUDGET,
        offscreen_animation_update_budget: TRUE_25D_OFFSCREEN_ANIMATION_UPDATE_BUDGET,
        offscreen_render_extraction_bypassed: bypassed_true_25d_entities > 0,
        offscreen_regions_zero_draw_calls: TRUE_25D_OFFSCREEN_PRESENTATION_DRAW_CALL_BUDGET == 0,
        headless_updates_continue: true,
        procedural_generation_without_rendering: field
            .map(|field| {
                field.generated_without_rendering
                    && field.procedural_content_generated_without_rendering
                    && !field.procedural_content_rendering_required
            })
            .unwrap_or(true),
        authoritative_scheduler_unchanged: CA13_FIXED_SIM_TICK_HZ == 20
            && CA13_TARGET_RENDER_FRAME_HZ == 60,
        no_action_authority: true,
    }
}

fn apply_existing_true_25d_viewport_render_bypass(world: &mut bevy::prelude::World) {
    let field = world
        .get_resource::<GraphicalProceduralTerrainFieldResource>()
        .cloned();
    let mut updates = Vec::new();
    let mut renderable_true_25d_entities = 0usize;
    let mut visible_true_25d_entities = 0usize;
    let mut bypassed_true_25d_entities = 0usize;
    let mut ledger_only_true_25d_assets = 0usize;
    {
        let mut query = world.query::<(
            Entity,
            &GraphicalTrue25dAsset,
            Option<&Transform>,
            Option<&GraphicalTrue25dGroundPlane>,
        )>();
        for (entity, _asset, transform, ground) in query.iter(world) {
            let Some(receipt) = true_25d_bypass_receipt(transform, ground) else {
                ledger_only_true_25d_assets = ledger_only_true_25d_assets.saturating_add(1);
                continue;
            };
            renderable_true_25d_entities = renderable_true_25d_entities.saturating_add(1);
            if receipt.render_pass_bypassed {
                bypassed_true_25d_entities = bypassed_true_25d_entities.saturating_add(1);
            } else {
                visible_true_25d_entities = visible_true_25d_entities.saturating_add(1);
            }
            let visibility = if receipt.render_pass_bypassed {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
            updates.push((entity, visibility, receipt));
        }
    }
    for (entity, visibility, receipt) in updates {
        world.entity_mut(entity).insert((visibility, receipt));
    }
    world.insert_resource(true_25d_bypass_summary(
        renderable_true_25d_entities,
        visible_true_25d_entities,
        bypassed_true_25d_entities,
        ledger_only_true_25d_assets,
        field.as_ref(),
    ));
}

fn update_true_25d_viewport_render_bypass(
    mut commands: Commands,
    field: Option<Res<GraphicalProceduralTerrainFieldResource>>,
    assets: bevy::prelude::Query<(
        Entity,
        &GraphicalTrue25dAsset,
        Option<&Transform>,
        Option<&GraphicalTrue25dGroundPlane>,
    )>,
) {
    let mut renderable_true_25d_entities = 0usize;
    let mut visible_true_25d_entities = 0usize;
    let mut bypassed_true_25d_entities = 0usize;
    let mut ledger_only_true_25d_assets = 0usize;
    for (entity, _asset, transform, ground) in &assets {
        let Some(receipt) = true_25d_bypass_receipt(transform, ground) else {
            ledger_only_true_25d_assets = ledger_only_true_25d_assets.saturating_add(1);
            continue;
        };
        renderable_true_25d_entities = renderable_true_25d_entities.saturating_add(1);
        let visibility = if receipt.render_pass_bypassed {
            bypassed_true_25d_entities = bypassed_true_25d_entities.saturating_add(1);
            Visibility::Hidden
        } else {
            visible_true_25d_entities = visible_true_25d_entities.saturating_add(1);
            Visibility::Visible
        };
        commands.entity(entity).insert((visibility, receipt));
    }
    commands.insert_resource(true_25d_bypass_summary(
        renderable_true_25d_entities,
        visible_true_25d_entities,
        bypassed_true_25d_entities,
        ledger_only_true_25d_assets,
        field.as_deref(),
    ));
}

fn true_25d_seeded_material_id(seed: u64, ix: i32, iz: i32) -> &'static str {
    let world_x = ix as f32 * 3.25;
    let world_z = iz as f32 * 3.25;
    let soil = ca44a_runtime_soil_weight(world_x, world_z);
    let resource = ca44a_runtime_resource_weight(world_x, world_z);
    let hazard = ca44a_runtime_hazard_weight(world_x, world_z);
    let stone = ca44a_runtime_stone_weight(world_x, world_z);
    let water = ca44a_runtime_water_weight(world_x, world_z);
    let sand = ca44a_runtime_sand_weight(world_x, world_z);
    let sampled = sample_procedural_terrain_tile(
        ProceduralWorldConfig::with_seed(seed),
        ProceduralTileCoord::new(ix, iz),
    )
    .map(|sample| sample.material.material_id())
    .unwrap_or("safe-grass");
    let mut ranked = [
        ("safe-grass", 0.30_f32),
        ("neutral-soil", soil * 1.08),
        ("resource-grove", resource * 1.34),
        ("hazard-pressure", hazard * 1.62),
        ("stone-dressing", stone * 1.24),
        ("water", water * 1.46),
        ("sand", sand * 1.24),
        (sampled, 0.38),
    ];
    ranked.sort_by(|(_, left), (_, right)| {
        right.partial_cmp(left).unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked[0].0
}

fn true_25d_terrain_role_for_material(material_id: &str) -> &'static str {
    match material_id {
        "neutral-soil" => "terrain-soil-island",
        "resource-grove" => "terrain-resource-grove",
        "hazard-pressure" => "terrain-hazard-pressure",
        "stone-dressing" => "terrain-stone-island",
        "water" => "terrain-water-cell",
        "sand" => "terrain-sand-island",
        _ => "terrain-grass-island",
    }
}

fn true_25d_material_id_for_role(role: &str) -> &'static str {
    match role {
        "terrain-soil-island" => "neutral-soil",
        "terrain-resource-grove" => "resource-grove",
        "terrain-hazard-pressure" => "hazard-pressure",
        "terrain-stone-island" => "stone-dressing",
        "terrain-water-cell" => "water",
        "terrain-sand-island" => "sand",
        _ => "safe-grass",
    }
}

fn true_25d_native_mesh_material_for_role(
    native_assets: &GraphicalTrue25dNativeAssets,
    role: &'static str,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    match role {
        "food" => (
            native_assets.food_mesh.clone(),
            native_assets.food_material.clone(),
        ),
        "hazard" => (
            native_assets.crystal_mesh.clone(),
            native_assets.hazard_crystal_material.clone(),
        ),
        "rock-obstacle" => (
            native_assets.rock_mesh.clone(),
            native_assets.rock_material.clone(),
        ),
        "plant-prop" => (
            native_assets.reed_mesh.clone(),
            native_assets.reed_material.clone(),
        ),
        _ => (
            native_assets.reed_mesh.clone(),
            native_assets.reed_material.clone(),
        ),
    }
}

fn true_25d_native_mesh_material_for_world_object(
    native_assets: &GraphicalTrue25dNativeAssets,
    kind: WorldObjectKind,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    match kind {
        WorldObjectKind::Agent => (
            native_assets.creature_body_mesh.clone(),
            native_assets.creature_material.clone(),
        ),
        WorldObjectKind::Food => (
            native_assets.food_mesh.clone(),
            native_assets.food_material.clone(),
        ),
        WorldObjectKind::Hazard => (
            native_assets.crystal_mesh.clone(),
            native_assets.hazard_crystal_material.clone(),
        ),
        WorldObjectKind::Obstacle => (
            native_assets.rock_mesh.clone(),
            native_assets.rock_material.clone(),
        ),
        WorldObjectKind::Token => (
            native_assets.reed_mesh.clone(),
            native_assets.reed_material.clone(),
        ),
    }
}

fn spawn_true_25d_creature_details(
    app: &mut App,
    native_assets: &GraphicalTrue25dNativeAssets,
    scene_assets: Option<&GraphicalTrue25dSceneAssets>,
    stable_id: WorldEntityId,
    base: Vec3,
) {
    let body_scale = 0.46;
    if let Some(scenes) = scene_assets {
        let (scene, scene_path) = true_25d_scene_for_role(scenes, "creature-hurt");
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life true 2.5D glTF creature pain/biolume cue stable:{}",
                stable_id.raw()
            )),
            SceneRoot(scene),
            Transform::from_translation(base + Vec3::new(0.0, 0.18, 0.0))
                .with_scale(true_25d_normalized_scale(0.12)),
            GraphicalTrue25dGltfScene {
                role: "creature-state-cue",
                scene_path,
                display_only: true,
            },
            GraphicalTrue25dAsset {
                role: "creature-state-cue",
                stable_id: Some(stable_id),
                display_only: true,
            },
            GraphicalProductionArtLayer {
                role: "true-25d-creature-expression",
                display_only: true,
            },
        ));
        return;
    }
    for (name, offset, scale, material) in [
        (
            "belly glow",
            Vec3::new(0.0, 0.02, -body_scale * 0.30),
            Vec3::new(body_scale * 0.30, body_scale * 0.08, body_scale * 0.20),
            native_assets.creature_glow_material.clone(),
        ),
        (
            "left eye",
            Vec3::new(-body_scale * 0.18, 0.18, -body_scale * 0.42),
            Vec3::splat(body_scale * 0.075),
            native_assets.creature_eye_material.clone(),
        ),
        (
            "right eye",
            Vec3::new(body_scale * 0.18, 0.18, -body_scale * 0.42),
            Vec3::splat(body_scale * 0.075),
            native_assets.creature_eye_material.clone(),
        ),
        (
            "pain crest",
            Vec3::new(0.0, 0.26, body_scale * 0.18),
            Vec3::new(body_scale * 0.12, body_scale * 0.045, body_scale * 0.16),
            native_assets.creature_hurt_material.clone(),
        ),
    ] {
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life true 2.5D creature {name} stable:{}",
                stable_id.raw()
            )),
            Mesh3d(native_assets.creature_eye_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(base + offset).with_scale(scale),
            GraphicalTrue25dAsset {
                role: "creature-state-cue",
                stable_id: Some(stable_id),
                display_only: true,
            },
            GraphicalProductionArtLayer {
                role: "true-25d-creature-expression",
                display_only: true,
            },
        ));
    }

    for (name, offset, rotation) in [
        (
            "left antenna",
            Vec3::new(-body_scale * 0.20, 0.30, -body_scale * 0.08),
            -0.35_f32,
        ),
        (
            "right antenna",
            Vec3::new(body_scale * 0.20, 0.30, -body_scale * 0.08),
            0.35_f32,
        ),
    ] {
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life true 2.5D creature {name} stable:{}",
                stable_id.raw()
            )),
            Mesh3d(native_assets.creature_antenna_mesh.clone()),
            MeshMaterial3d(native_assets.creature_material.clone()),
            Transform::from_translation(base + offset)
                .with_rotation(Quat::from_rotation_z(rotation))
                .with_scale(true_25d_normalized_scale(body_scale * 0.62)),
            GraphicalTrue25dAsset {
                role: "creature-state-cue",
                stable_id: Some(stable_id),
                display_only: true,
            },
            GraphicalProductionArtLayer {
                role: "true-25d-creature-expression",
                display_only: true,
            },
        ));
    }
}

fn spawn_true_25d_neurochemical_visual_feedback(
    app: &mut App,
    inspector: &CreatureInspectorSnapshot,
    gpu: &GraphicalGpuRuntimeTelemetry,
) {
    let (gltf_contract_validated, gltf_contract_assets) = app
        .world()
        .get_resource::<GraphicalTrue25dPresentationResource>()
        .map(|presentation| {
            (
                presentation
                    .asset_manifest
                    .endocrine_feedback_contract_validated,
                presentation.asset_manifest.endocrine_feedback_assets,
            )
        })
        .unwrap_or((false, 0));
    let native_assets = true_25d_native_assets(app);
    let feedback = true_25d_neurochemical_feedback_from_snapshot(inspector, gpu);
    app.insert_resource(feedback);
    let endocrine = true_25d_endocrine_asset_feedback_from_snapshot(
        inspector,
        gpu,
        0,
        false,
        gltf_contract_validated,
        gltf_contract_assets,
    );
    app.insert_resource(endocrine);
    let base = true_25d_creature_visual_position(&inspector.visual);
    for kind in [
        GraphicalTrue25dNeurochemicalCueKind::HungerGlow,
        GraphicalTrue25dNeurochemicalCueKind::PainSpike,
        GraphicalTrue25dNeurochemicalCueKind::StressDesaturation,
        GraphicalTrue25dNeurochemicalCueKind::EnergyTrail,
        GraphicalTrue25dNeurochemicalCueKind::SleepBloom,
        GraphicalTrue25dNeurochemicalCueKind::LearningBiolume,
    ] {
        let intensity =
            true_25d_neurochemical_intensity(kind, &inspector.visual, gpu).clamp(0.0, 1.0);
        let active = true_25d_neurochemical_cue_active(kind, intensity);
        let (mesh, material) = true_25d_neurochemical_mesh_material(&native_assets, kind);
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life true 2.5D {} cue stable:{}",
                kind.player_label(),
                inspector.visual.stable_id.raw()
            )),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform::from_translation(base + true_25d_neurochemical_offset(kind))
                .with_scale(true_25d_neurochemical_scale(kind, intensity)),
            if active {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            GraphicalTrue25dNeurochemicalCue {
                stable_id: inspector.visual.stable_id,
                kind,
                intensity,
                active,
                anchored_to_selected_creature: true,
                display_only: true,
                no_action_authority: true,
                no_weight_authority: true,
            },
            GraphicalTrue25dAsset {
                role: kind.role(),
                stable_id: Some(inspector.visual.stable_id),
                display_only: true,
            },
            GraphicalProductionArtLayer {
                role: "true-25d-neurochemical-feedback",
                display_only: true,
            },
        ));
    }
    for lane_index in 0..3_u8 {
        let active = lane_index < endocrine.particle_trail_count;
        let intensity = endocrine
            .hunger_satisfaction_biolume
            .max(endocrine.learning_biolume)
            .clamp(0.0, 1.0);
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life true 2.5D endocrine particle lane {} stable:{}",
                lane_index,
                inspector.visual.stable_id.raw()
            )),
            Mesh3d(native_assets.creature_eye_mesh.clone()),
            MeshMaterial3d(native_assets.learning_biolume_material.clone()),
            Transform::from_translation(
                base + true_25d_endocrine_particle_offset(
                    lane_index,
                    intensity,
                    endocrine.animation_phase_index,
                ),
            )
            .with_scale(true_25d_endocrine_particle_scale(lane_index, intensity)),
            if active {
                Visibility::Visible
            } else {
                Visibility::Hidden
            },
            GraphicalTrue25dEndocrineParticleLane {
                stable_id: inspector.visual.stable_id,
                lane_index,
                intensity,
                active,
                animation_phase_index: endocrine.animation_phase_index,
                initialized_from_endocrine_tensor: true,
                display_only: true,
                no_action_authority: true,
                no_weight_authority: true,
            },
            GraphicalTrue25dAsset {
                role: "endocrine-biolume-particle",
                stable_id: Some(inspector.visual.stable_id),
                display_only: true,
            },
            GraphicalProductionArtLayer {
                role: "true-25d-endocrine-particle-array",
                display_only: true,
            },
        ));
    }
}

impl GraphicalTrue25dCreatureEndocrinePresentation {
    fn neutral(stable_id: WorldEntityId, base_scale: f32) -> Self {
        Self {
            stable_id,
            base_scale,
            asset_scale_multiplier: 1.0,
            animation_speed_multiplier: 1.0,
            animation_phase_index: 0,
            posture_roll_degrees: 0.0,
            posture_lift: 0.0,
            adrenaline_proxy: 0.0,
            cortisol_desaturation: 0.0,
            hunger_satisfaction_biolume: 0.0,
            learning_biolume: 0.0,
            particle_trail_count: 0,
            biolume_particle_array_initialized: false,
            creature_root_transform_applied: false,
            material_shell_applied: false,
            display_only: true,
            no_action_authority: true,
            no_weight_authority: true,
        }
    }
}

fn true_25d_creature_visual_position(visual: &CreatureVisualSnapshot) -> Vec3 {
    Vec3::new(
        visual.position.x * TRUE_25D_SIM_TO_VIEW_SCALE,
        true_25d_height_for_kind(WorldObjectKind::Agent) + 0.10,
        visual.position.z * TRUE_25D_SIM_TO_VIEW_SCALE,
    )
}

fn true_25d_endocrine_asset_feedback_from_snapshot(
    snapshot: &CreatureInspectorSnapshot,
    gpu: &GraphicalGpuRuntimeTelemetry,
    mind_tick: u64,
    applied_to_creature_root: bool,
    gltf_endocrine_feedback_contract_validated: bool,
    gltf_endocrine_feedback_assets: usize,
) -> GraphicalTrue25dEndocrineAssetFeedbackResource {
    let visual = &snapshot.visual;
    let tensor = true_25d_flat_endocrine_tensor_from_snapshot(visual, gpu);
    let pain = tensor.pain_drive_companion;
    let adrenaline = tensor.values[tensor.adrenaline_channel_index];
    let cortisol = tensor.values[tensor.cortisol_channel_index];
    let dopamine = tensor.values[tensor.dopamine_channel_index];
    let learning = tensor.learning_companion;
    let adrenaline_proxy = adrenaline.max(pain).clamp(0.0, 1.0);
    let dopamine_biolume = dopamine
        .max(tensor.low_hunger_drive_companion)
        .clamp(0.0, 1.0);
    let hunger_satisfaction_biolume = dopamine_biolume.max(learning).clamp(0.0, 1.0);
    let pain_posture_active = pain >= 0.08 || adrenaline_proxy >= 0.18;
    let animation_speed_multiplier = (1.0 + adrenaline_proxy * 0.65 + pain * 0.45).clamp(1.0, 2.20);
    let animation_phase_index =
        ((mind_tick as f32 * animation_speed_multiplier).floor() as u64 % 4) as u8;
    let phase = if animation_phase_index % 2 == 0 {
        1.0
    } else {
        -1.0
    };
    let posture_roll_degrees = if pain_posture_active {
        phase * (2.0 + adrenaline_proxy * 5.0)
    } else {
        0.0
    };
    let posture_lift = (pain * 0.035 + learning * 0.020).clamp(0.0, 0.07);
    let asset_scale_multiplier = (1.0 + hunger_satisfaction_biolume * 0.025 + pain * 0.015
        - cortisol * 0.025)
        .clamp(0.94, 1.045);
    let particle_trail_count = if hunger_satisfaction_biolume >= 0.72 || learning >= 0.50 {
        2
    } else if hunger_satisfaction_biolume >= 0.32 || learning > 0.0 {
        1
    } else {
        0
    };

    GraphicalTrue25dEndocrineAssetFeedbackResource {
        schema_version: 1,
        selected_stable_id: visual.stable_id,
        gltf_endocrine_feedback_contract_validated,
        gltf_endocrine_feedback_assets,
        direct_asset_feedback_contract: gltf_endocrine_feedback_contract_validated
            && gltf_endocrine_feedback_assets >= crate::TRUE_25D_ENDOCRINE_FEEDBACK_ROLES.len(),
        applied_to_creature_root,
        root_transform_posture: pain_posture_active || posture_lift > f32::EPSILON,
        material_shell_applied: cortisol >= 0.22
            || hunger_satisfaction_biolume >= 0.20
            || learning > 0.0,
        flat_endocrine_tensor_channels: tensor.channel_count,
        flat_endocrine_tensor_bounded: tensor.values_bounded,
        derived_from_flat_endocrine_tensor: true,
        endocrine_tensor_source: tensor.source,
        pain_posture_active,
        adrenaline_proxy,
        cortisol_desaturation: cortisol,
        dopamine_biolume,
        pain_drive_companion: tensor.pain_drive_companion,
        low_hunger_drive_companion: tensor.low_hunger_drive_companion,
        hunger_satisfaction_biolume,
        learning_biolume: learning,
        asset_scale_multiplier,
        animation_speed_multiplier,
        animation_phase_index,
        animation_speed_layer_applied: animation_speed_multiplier > 1.01,
        posture_roll_degrees,
        posture_lift,
        particle_trail_count,
        biolume_particle_array_initialized: particle_trail_count > 0,
        biolume_particle_lanes_visible: particle_trail_count,
        biolume_particle_lanes_max: 3,
        emissive_particle_array_initialized: hunger_satisfaction_biolume >= 0.20 || learning > 0.0,
        derived_from_visual_snapshot: true,
        display_only: true,
        no_action_authority: true,
        no_weight_authority: true,
        tensor_action_authority: false,
        tensor_weight_authority: false,
        gpu_authority_preserved: gpu.authoritative && gpu.no_active_bulk_readback,
        no_active_bulk_readback: gpu.no_active_bulk_readback,
    }
}

fn true_25d_flat_endocrine_tensor_from_snapshot(
    visual: &CreatureVisualSnapshot,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> GraphicalTrue25dFlatEndocrineTensor {
    let values = visual
        .endocrine
        .to_array()
        .map(|value| value.clamp(0.0, 1.0));
    GraphicalTrue25dFlatEndocrineTensor {
        schema_version: TRUE_25D_ENDOCRINE_TENSOR_SCHEMA_VERSION,
        channel_count: alife_core::EndocrineSnapshot::CHANNEL_COUNT,
        values,
        adrenaline_channel_index: TRUE_25D_ENDOCRINE_ADRENALINE_CHANNEL,
        cortisol_channel_index: TRUE_25D_ENDOCRINE_CORTISOL_CHANNEL,
        dopamine_channel_index: TRUE_25D_ENDOCRINE_DOPAMINE_CHANNEL,
        sleep_pressure_channel_index: TRUE_25D_ENDOCRINE_SLEEP_PRESSURE_CHANNEL,
        pain_drive_companion: visual.cues.pain.value.clamp(0.0, 1.0),
        low_hunger_drive_companion: (1.0 - visual.cues.hunger.value).clamp(0.0, 1.0),
        learning_companion: true_25d_learning_cue_intensity(gpu),
        source: "alife_core.EndocrineSnapshot::to_array plus bounded drive companions",
        values_bounded: values.iter().all(|value| (0.0..=1.0).contains(value)),
        display_only: true,
        no_action_authority: true,
        no_weight_authority: true,
    }
}

fn true_25d_neurochemical_feedback_from_snapshot(
    snapshot: &CreatureInspectorSnapshot,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> GraphicalTrue25dNeurochemicalFeedbackResource {
    let visual = &snapshot.visual;
    let stress = true_25d_neurochemical_stress(visual);
    let sleep_pressure = visual
        .cues
        .sleep_pressure
        .value
        .max(visual.cues.fatigue.value)
        .clamp(0.0, 1.0);
    let learning = true_25d_learning_cue_intensity(gpu);
    let intensities = [
        visual.cues.hunger.value,
        visual.cues.pain.value,
        stress,
        visual.cues.energy.value,
        sleep_pressure,
        learning,
    ];
    let active_cue_count = intensities
        .into_iter()
        .zip([
            GraphicalTrue25dNeurochemicalCueKind::HungerGlow,
            GraphicalTrue25dNeurochemicalCueKind::PainSpike,
            GraphicalTrue25dNeurochemicalCueKind::StressDesaturation,
            GraphicalTrue25dNeurochemicalCueKind::EnergyTrail,
            GraphicalTrue25dNeurochemicalCueKind::SleepBloom,
            GraphicalTrue25dNeurochemicalCueKind::LearningBiolume,
        ])
        .filter(|(intensity, kind)| true_25d_neurochemical_cue_active(*kind, *intensity))
        .count();
    GraphicalTrue25dNeurochemicalFeedbackResource {
        schema_version: 1,
        selected_stable_id: visual.stable_id,
        cue_count: 6,
        active_cue_count,
        hunger: visual.cues.hunger.value,
        pain: visual.cues.pain.value,
        stress,
        energy: visual.cues.energy.value,
        sleep_pressure,
        learning,
        direct_mesh_presentation: true,
        display_only: true,
        no_action_authority: true,
        no_weight_authority: true,
        gpu_authority_preserved: gpu.authoritative && gpu.no_active_bulk_readback,
        no_active_bulk_readback: gpu.no_active_bulk_readback,
    }
}

fn true_25d_neurochemical_intensity(
    kind: GraphicalTrue25dNeurochemicalCueKind,
    visual: &CreatureVisualSnapshot,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> f32 {
    match kind {
        GraphicalTrue25dNeurochemicalCueKind::HungerGlow => visual.cues.hunger.value,
        GraphicalTrue25dNeurochemicalCueKind::PainSpike => visual.cues.pain.value,
        GraphicalTrue25dNeurochemicalCueKind::StressDesaturation => {
            true_25d_neurochemical_stress(visual)
        }
        GraphicalTrue25dNeurochemicalCueKind::EnergyTrail => visual.cues.energy.value,
        GraphicalTrue25dNeurochemicalCueKind::SleepBloom => visual
            .cues
            .sleep_pressure
            .value
            .max(visual.cues.fatigue.value),
        GraphicalTrue25dNeurochemicalCueKind::LearningBiolume => {
            true_25d_learning_cue_intensity(gpu)
        }
    }
    .clamp(0.0, 1.0)
}

fn true_25d_neurochemical_stress(visual: &CreatureVisualSnapshot) -> f32 {
    visual
        .cues
        .fear
        .value
        .max(visual.cues.pain.value * 0.85)
        .max(visual.cues.fatigue.value * 0.45)
        .clamp(0.0, 1.0)
}

fn true_25d_learning_cue_intensity(gpu: &GraphicalGpuRuntimeTelemetry) -> f32 {
    if gpu.learning_updates == 0 {
        0.0
    } else {
        (0.35_f32 + gpu.last_learning_delta.abs() * 18.0).clamp(0.35, 1.0)
    }
}

fn true_25d_neurochemical_cue_active(
    kind: GraphicalTrue25dNeurochemicalCueKind,
    intensity: f32,
) -> bool {
    let threshold = match kind {
        GraphicalTrue25dNeurochemicalCueKind::HungerGlow => 0.18,
        GraphicalTrue25dNeurochemicalCueKind::PainSpike => 0.08,
        GraphicalTrue25dNeurochemicalCueKind::StressDesaturation => 0.22,
        GraphicalTrue25dNeurochemicalCueKind::EnergyTrail => 0.42,
        GraphicalTrue25dNeurochemicalCueKind::SleepBloom => 0.48,
        GraphicalTrue25dNeurochemicalCueKind::LearningBiolume => f32::EPSILON,
    };
    intensity >= threshold
}

fn true_25d_neurochemical_mesh_material(
    native_assets: &GraphicalTrue25dNativeAssets,
    kind: GraphicalTrue25dNeurochemicalCueKind,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    match kind {
        GraphicalTrue25dNeurochemicalCueKind::HungerGlow => (
            native_assets.terrain_mesh.clone(),
            native_assets.hunger_glow_material.clone(),
        ),
        GraphicalTrue25dNeurochemicalCueKind::PainSpike => (
            native_assets.crystal_mesh.clone(),
            native_assets.pain_spike_material.clone(),
        ),
        GraphicalTrue25dNeurochemicalCueKind::StressDesaturation => (
            native_assets.rock_mesh.clone(),
            native_assets.stress_desaturation_material.clone(),
        ),
        GraphicalTrue25dNeurochemicalCueKind::EnergyTrail => (
            native_assets.reed_mesh.clone(),
            native_assets.energy_trail_material.clone(),
        ),
        GraphicalTrue25dNeurochemicalCueKind::SleepBloom => (
            native_assets.terrain_mesh.clone(),
            native_assets.sleep_bloom_material.clone(),
        ),
        GraphicalTrue25dNeurochemicalCueKind::LearningBiolume => (
            native_assets.ring_mesh.clone(),
            native_assets.learning_biolume_material.clone(),
        ),
    }
}

fn true_25d_neurochemical_offset(kind: GraphicalTrue25dNeurochemicalCueKind) -> Vec3 {
    match kind {
        GraphicalTrue25dNeurochemicalCueKind::HungerGlow => Vec3::new(-0.34, 0.06, -0.28),
        GraphicalTrue25dNeurochemicalCueKind::PainSpike => Vec3::new(0.32, 0.30, 0.16),
        GraphicalTrue25dNeurochemicalCueKind::StressDesaturation => Vec3::new(0.00, -0.02, 0.08),
        GraphicalTrue25dNeurochemicalCueKind::EnergyTrail => Vec3::new(-0.62, 0.04, 0.42),
        GraphicalTrue25dNeurochemicalCueKind::SleepBloom => Vec3::new(0.52, 0.05, -0.40),
        GraphicalTrue25dNeurochemicalCueKind::LearningBiolume => Vec3::new(0.0, 0.14, 0.0),
    }
}

fn true_25d_neurochemical_scale(
    kind: GraphicalTrue25dNeurochemicalCueKind,
    intensity: f32,
) -> Vec3 {
    let i = intensity.clamp(0.0, 1.0);
    match kind {
        GraphicalTrue25dNeurochemicalCueKind::HungerGlow => Vec3::splat(0.06 + i * 0.06),
        GraphicalTrue25dNeurochemicalCueKind::PainSpike => Vec3::splat(0.05 + i * 0.06),
        GraphicalTrue25dNeurochemicalCueKind::StressDesaturation => Vec3::splat(0.08 + i * 0.06),
        GraphicalTrue25dNeurochemicalCueKind::EnergyTrail => {
            Vec3::new(0.045 + i * 0.04, 0.045 + i * 0.04, 0.13 + i * 0.10)
        }
        GraphicalTrue25dNeurochemicalCueKind::SleepBloom => {
            Vec3::new(0.08 + i * 0.07, 0.08 + i * 0.07, 0.05)
        }
        GraphicalTrue25dNeurochemicalCueKind::LearningBiolume => Vec3::splat(0.10 + i * 0.07),
    }
}

fn true_25d_endocrine_particle_offset(
    lane_index: u8,
    intensity: f32,
    animation_phase_index: u8,
) -> Vec3 {
    let lane = lane_index.min(2) as f32;
    let i = intensity.clamp(0.0, 1.0);
    let phase = animation_phase_index as f32 * 0.18;
    Vec3::new(
        -0.42 + lane * 0.28,
        0.24 + lane * 0.035 + i * 0.10,
        0.34 + phase - lane * 0.12,
    )
}

fn true_25d_endocrine_particle_scale(lane_index: u8, intensity: f32) -> Vec3 {
    let i = intensity.clamp(0.0, 1.0);
    Vec3::splat(0.034 + i * 0.036 + lane_index.min(2) as f32 * 0.008)
}

fn spawn_true_25d_object_scene(
    app: &mut App,
    native_assets: &GraphicalTrue25dNativeAssets,
    scene_assets: Option<&GraphicalTrue25dSceneAssets>,
    object: &VisibleWorldObjectPresentation,
) -> Result<(), GameAppShellError> {
    if object.kind == WorldObjectKind::Token {
        let entity = app
            .world_mut()
            .spawn((
                Name::new(format!(
                    "A-Life true 2.5D hidden school token stable:{}",
                    object.stable_id.raw()
                )),
                Transform::from_translation(true_25d_position(object)),
                GraphicalTrue25dAsset {
                    role: "school-token-hidden",
                    stable_id: Some(object.stable_id),
                    display_only: true,
                },
                GraphicalProductionArtLayer {
                    role: "true-25d-world-entity-hidden-token",
                    display_only: true,
                },
            ))
            .id();
        attach_visible_world_runtime_components(app, entity, object)?;
        return Ok(());
    }
    let role = true_25d_role_for_world_object(object.kind);
    let base = true_25d_position(object);
    if let Some(alpha_art) = app
        .world()
        .get_resource::<GraphicalAlphaArtHandles>()
        .cloned()
    {
        let entity =
            spawn_true_25d_alpha_art_billboard(app, native_assets, &alpha_art, object, role, base);
        if object.kind == WorldObjectKind::Agent {
            app.world_mut().entity_mut(entity).insert(
                GraphicalTrue25dCreatureEndocrinePresentation::neutral(
                    object.stable_id,
                    true_25d_alpha_billboard_scale_for_kind(object.kind),
                ),
            );
        }
        spawn_true_25d_entity_contact_shadow(
            app,
            native_assets,
            object.kind,
            base,
            object.stable_id,
        );
        attach_visible_world_runtime_components(app, entity, object)?;
        if object.kind == WorldObjectKind::Agent {
            spawn_true_25d_alpha_art_selection_ring(
                app,
                native_assets,
                &alpha_art,
                base,
                object.stable_id,
            );
        }
        return Ok(());
    }
    let entity = if let Some(scenes) = scene_assets {
        let (scene, scene_path) = true_25d_scene_for_role(scenes, role);
        app.world_mut()
            .spawn((
                Name::new(format!(
                    "A-Life true 2.5D glTF {role} stable:{}",
                    object.stable_id.raw()
                )),
                SceneRoot(scene),
                Transform::from_translation(base).with_scale(true_25d_normalized_scale(
                    true_25d_gltf_scale_for_kind(object.kind),
                )),
                GraphicalTrue25dGltfScene {
                    role,
                    scene_path,
                    display_only: true,
                },
                GraphicalTrue25dAsset {
                    role,
                    stable_id: Some(object.stable_id),
                    display_only: true,
                },
                GraphicalTrue25dStateCue {
                    stable_id: object.stable_id,
                    pain_pose: object.kind == WorldObjectKind::Hazard,
                    stress_desaturated: false,
                    learning_biolume: object.kind == WorldObjectKind::Agent,
                    display_only: true,
                },
                GraphicalProductionArtLayer {
                    role: "true-25d-world-entity",
                    display_only: true,
                },
            ))
            .id()
    } else {
        let (mesh, material) =
            true_25d_native_mesh_material_for_world_object(native_assets, object.kind);
        app.world_mut()
            .spawn((
                Name::new(format!(
                    "A-Life true 2.5D {role} stable:{}",
                    object.stable_id.raw()
                )),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_translation(base).with_scale(true_25d_normalized_scale(
                    true_25d_scale_for_kind(object.kind),
                )),
                GraphicalTrue25dAsset {
                    role,
                    stable_id: Some(object.stable_id),
                    display_only: true,
                },
                GraphicalTrue25dStateCue {
                    stable_id: object.stable_id,
                    pain_pose: object.kind == WorldObjectKind::Hazard,
                    stress_desaturated: false,
                    learning_biolume: object.kind == WorldObjectKind::Agent,
                    display_only: true,
                },
                GraphicalProductionArtLayer {
                    role: "true-25d-world-entity",
                    display_only: true,
                },
            ))
            .id()
    };
    if object.kind == WorldObjectKind::Agent {
        let base_scale = if scene_assets.is_some() {
            true_25d_gltf_scale_for_kind(object.kind)
        } else {
            true_25d_scale_for_kind(object.kind)
        };
        app.world_mut().entity_mut(entity).insert(
            GraphicalTrue25dCreatureEndocrinePresentation::neutral(object.stable_id, base_scale),
        );
    }
    spawn_true_25d_entity_contact_shadow(app, native_assets, object.kind, base, object.stable_id);
    attach_visible_world_runtime_components(app, entity, object)?;
    if object.kind == WorldObjectKind::Agent {
        spawn_true_25d_creature_details(app, native_assets, scene_assets, object.stable_id, base);
    }
    if object.kind == WorldObjectKind::Agent {
        if let Some(scenes) = scene_assets {
            let (scene, scene_path) = true_25d_scene_for_role(scenes, "selection-ring");
            app.world_mut().spawn((
                Name::new(format!(
                    "A-Life true 2.5D glTF selection ring stable:{}",
                    object.stable_id.raw()
                )),
                SceneRoot(scene),
                Transform::from_translation(base + Vec3::new(0.0, 0.035, 0.0))
                    .with_scale(true_25d_normalized_scale(0.42)),
                GraphicalTrue25dGltfScene {
                    role: "selection-ring",
                    scene_path,
                    display_only: true,
                },
                GraphicalTrue25dAsset {
                    role: "selection-ring",
                    stable_id: Some(object.stable_id),
                    display_only: true,
                },
            ));
        } else {
            app.world_mut().spawn((
                Name::new(format!(
                    "A-Life true 2.5D selection ring stable:{}",
                    object.stable_id.raw()
                )),
                Mesh3d(native_assets.ring_mesh.clone()),
                MeshMaterial3d(native_assets.selection_material.clone()),
                Transform::from_translation(base + Vec3::new(0.0, 0.035, 0.0))
                    .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
                    .with_scale(true_25d_normalized_scale(0.42)),
                GraphicalTrue25dAsset {
                    role: "selection-ring",
                    stable_id: Some(object.stable_id),
                    display_only: true,
                },
            ));
        }
    }
    Ok(())
}

fn spawn_true_25d_alpha_art_billboard(
    app: &mut App,
    native_assets: &GraphicalTrue25dNativeAssets,
    alpha_art: &GraphicalAlphaArtHandles,
    object: &VisibleWorldObjectPresentation,
    role: &'static str,
    base: Vec3,
) -> Entity {
    let texture = true_25d_alpha_art_handle_for_world_object(alpha_art, object.kind);
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color_texture: Some(texture),
            base_color: true_25d_alpha_art_tint_for_world_object(object.kind),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            perceptual_roughness: 0.88,
            ..default()
        });
    app.world_mut()
        .spawn((
            Name::new(format!(
                "A-Life true 2.5D alpha-art {role} stable:{}",
                object.stable_id.raw()
            )),
            Mesh3d(native_assets.billboard_plane_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(base + true_25d_alpha_billboard_offset(object.kind))
                .with_rotation(true_25d_alpha_billboard_rotation())
                .with_scale(true_25d_normalized_scale(
                    true_25d_alpha_billboard_scale_for_kind(object.kind),
                )),
            GraphicalAlphaArtBackedSprite {
                role,
                stable_id: Some(object.stable_id),
            },
            GraphicalTrue25dAsset {
                role,
                stable_id: Some(object.stable_id),
                display_only: true,
            },
            GraphicalTrue25dStateCue {
                stable_id: object.stable_id,
                pain_pose: object.kind == WorldObjectKind::Hazard,
                stress_desaturated: false,
                learning_biolume: object.kind == WorldObjectKind::Agent,
                display_only: true,
            },
            GraphicalProductionArtLayer {
                role: "true-25d-alpha-art-world-entity",
                display_only: true,
            },
        ))
        .id()
}

fn spawn_true_25d_alpha_art_selection_ring(
    app: &mut App,
    native_assets: &GraphicalTrue25dNativeAssets,
    alpha_art: &GraphicalAlphaArtHandles,
    base: Vec3,
    stable_id: WorldEntityId,
) {
    let material = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color_texture: Some(alpha_art.selection_ring.clone()),
            base_color: Color::srgba(1.0, 1.0, 1.0, 0.92),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            cull_mode: None,
            perceptual_roughness: 0.9,
            ..default()
        });
    app.world_mut().spawn((
        Name::new(format!(
            "A-Life true 2.5D alpha-art selection ring stable:{}",
            stable_id.raw()
        )),
        Mesh3d(native_assets.billboard_plane_mesh.clone()),
        MeshMaterial3d(material),
        Transform::from_translation(base + Vec3::new(0.0, -0.01, 0.0))
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(true_25d_normalized_scale(0.38)),
        GraphicalAlphaArtBackedSprite {
            role: "selection-ring",
            stable_id: Some(stable_id),
        },
        GraphicalTrue25dAsset {
            role: "selection-ring",
            stable_id: Some(stable_id),
            display_only: true,
        },
        GraphicalProductionArtLayer {
            role: "true-25d-alpha-art-selection-ring",
            display_only: true,
        },
    ));
}

fn true_25d_alpha_art_handle_for_world_object(
    alpha_art: &GraphicalAlphaArtHandles,
    kind: WorldObjectKind,
) -> Handle<Image> {
    match kind {
        WorldObjectKind::Agent => alpha_art.creature_idle.clone(),
        WorldObjectKind::Food => alpha_art.food.clone(),
        WorldObjectKind::Hazard => alpha_art.hazard.clone(),
        WorldObjectKind::Obstacle => alpha_art.rock_obstacle.clone(),
        WorldObjectKind::Token => alpha_art.prop_warning_shard.clone(),
    }
}

fn true_25d_alpha_art_tint_for_world_object(kind: WorldObjectKind) -> Color {
    match kind {
        WorldObjectKind::Agent => Color::srgba(1.0, 1.0, 1.0, 1.0),
        WorldObjectKind::Food => Color::srgba(1.0, 1.0, 1.0, 0.96),
        WorldObjectKind::Hazard => Color::srgba(1.0, 0.96, 0.92, 0.98),
        WorldObjectKind::Obstacle => Color::srgba(1.0, 1.0, 1.0, 0.96),
        WorldObjectKind::Token => Color::srgba(1.0, 1.0, 1.0, 0.74),
    }
}

fn true_25d_alpha_billboard_offset(kind: WorldObjectKind) -> Vec3 {
    match kind {
        WorldObjectKind::Agent => Vec3::new(0.0, 0.05, 0.0),
        WorldObjectKind::Food => Vec3::new(0.0, 0.04, 0.0),
        WorldObjectKind::Hazard => Vec3::new(0.0, 0.04, 0.0),
        WorldObjectKind::Obstacle => Vec3::new(0.0, 0.03, 0.0),
        WorldObjectKind::Token => Vec3::new(0.0, 0.02, 0.0),
    }
}

fn true_25d_alpha_billboard_scale_for_kind(kind: WorldObjectKind) -> f32 {
    match kind {
        WorldObjectKind::Agent => 0.42,
        WorldObjectKind::Food => 0.38,
        WorldObjectKind::Hazard => 0.56,
        WorldObjectKind::Obstacle => 0.54,
        WorldObjectKind::Token => 0.24,
    }
}

fn true_25d_alpha_billboard_rotation() -> Quat {
    Quat::from_rotation_x(std::f32::consts::FRAC_PI_4)
}

fn spawn_true_25d_entity_contact_shadow(
    app: &mut App,
    native_assets: &GraphicalTrue25dNativeAssets,
    kind: WorldObjectKind,
    base: Vec3,
    stable_id: WorldEntityId,
) {
    let (width, depth) = match kind {
        WorldObjectKind::Agent => (0.46, 0.26),
        WorldObjectKind::Food => (0.24, 0.16),
        WorldObjectKind::Hazard => (0.34, 0.22),
        WorldObjectKind::Obstacle => (0.38, 0.24),
        WorldObjectKind::Token => return,
    };
    app.world_mut().spawn((
        Name::new(format!(
            "A-Life true 2.5D contact shadow stable:{}",
            stable_id.raw()
        )),
        Mesh3d(native_assets.terrain_mesh.clone()),
        MeshMaterial3d(native_assets.contact_shadow_material.clone()),
        Transform::from_translation(Vec3::new(base.x, 0.026, base.z))
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::new(width, depth, 1.0)),
        GraphicalTrue25dAsset {
            role: "entity-contact-shadow",
            stable_id: Some(stable_id),
            display_only: true,
        },
        GraphicalProductionArtLayer {
            role: "true-25d-entity-contact-shadow",
            display_only: true,
        },
    ));
}

fn attach_visible_world_runtime_components(
    app: &mut App,
    entity: Entity,
    object: &VisibleWorldObjectPresentation,
) -> Result<(), GameAppShellError> {
    {
        let mut entity_mut = app.world_mut().entity_mut(entity);
        entity_mut.insert((
            VisibleWorldObject {
                stable_id: object.stable_id,
                kind: object.kind,
                shape: object.shape,
                material: object.material,
                rgba: object.material.rgba(),
            },
            VisibleWorldDebugLabel(object.debug_label.clone()),
            GraphicalPlaygroundMarker {
                stable_id: object.stable_id,
                kind: object.kind,
            },
        ));
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
    Ok(())
}

fn true_25d_position(object: &VisibleWorldObjectPresentation) -> Vec3 {
    Vec3::new(
        object.position.x * TRUE_25D_SIM_TO_VIEW_SCALE,
        true_25d_height_for_kind(object.kind),
        object.position.z * TRUE_25D_SIM_TO_VIEW_SCALE,
    ) + true_25d_display_separation_offset(object.stable_id, object.kind)
}

fn true_25d_display_separation_offset(stable_id: WorldEntityId, kind: WorldObjectKind) -> Vec3 {
    let radius = match kind {
        WorldObjectKind::Agent => 0.58,
        WorldObjectKind::Food => 0.08,
        WorldObjectKind::Hazard => 0.08,
        WorldObjectKind::Obstacle => 0.06,
        WorldObjectKind::Token => 0.0,
    };
    if radius == 0.0 {
        return Vec3::ZERO;
    }
    let raw = stable_id.raw();
    let angle_index = raw.wrapping_mul(1_103_515_245).wrapping_add(12_345) % 360;
    let angle = (angle_index as f32).to_radians();
    let ring = 0.68 + ((raw.wrapping_mul(37) % 32) as f32 / 31.0) * 0.32;
    Vec3::new(
        angle.cos() * radius * ring,
        0.0,
        angle.sin() * radius * ring,
    )
}

fn true_25d_height_for_kind(kind: WorldObjectKind) -> f32 {
    match kind {
        WorldObjectKind::Agent => 0.34,
        WorldObjectKind::Food => 0.18,
        WorldObjectKind::Hazard => 0.30,
        WorldObjectKind::Obstacle => 0.20,
        WorldObjectKind::Token => 0.16,
    }
}

fn true_25d_scale_for_kind(kind: WorldObjectKind) -> f32 {
    match kind {
        WorldObjectKind::Agent => 0.24,
        WorldObjectKind::Food => 0.34,
        WorldObjectKind::Hazard => 0.48,
        WorldObjectKind::Obstacle => 0.46,
        WorldObjectKind::Token => 0.08,
    }
}

fn true_25d_gltf_scale_for_kind(kind: WorldObjectKind) -> f32 {
    match kind {
        WorldObjectKind::Agent => 0.24,
        WorldObjectKind::Food => 0.34,
        WorldObjectKind::Hazard => 0.48,
        WorldObjectKind::Obstacle => 0.46,
        WorldObjectKind::Token => 0.08,
    }
}

fn true_25d_role_for_world_object(kind: WorldObjectKind) -> &'static str {
    match kind {
        WorldObjectKind::Agent => "creature-idle",
        WorldObjectKind::Food => "food",
        WorldObjectKind::Hazard => "hazard",
        WorldObjectKind::Obstacle => "rock-obstacle",
        WorldObjectKind::Token => "plant-prop",
    }
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
    if summary.view_mode == GraphicalPlaygroundViewMode::Player {
        let gpu = GraphicalGpuRuntimeTelemetry::pending("N2048");
        spawn_true_25d_neurochemical_visual_feedback(&mut app, &inspector, &gpu);
    }
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

pub fn run_true25d_launch_baseline_smoke(
    launch: &GraphicalPlaygroundLaunchConfig,
) -> Result<GraphicalTrue25dLaunchBaselineSummary, GameAppShellError> {
    let mut app = App::new();
    let started = Instant::now();
    let manifest = crate::True25dAssetValidationSummary {
        schema: crate::TRUE_25D_ALPHA_ASSET_MANIFEST_SCHEMA,
        schema_version: crate::TRUE_25D_ALPHA_ASSET_MANIFEST_SCHEMA_VERSION,
        pack_id: "true25d-camera-ground-baseline".to_string(),
        manifest_path: crate::default_true_25d_asset_manifest_path(),
        entry_count: crate::TRUE_25D_REQUIRED_ROLES.len(),
        required_roles_present: true,
        gltf_files_validated: false,
        orthographic_camera_locked: true,
        shader_stack_declared: true,
        largest_file_bytes: 0,
        total_file_bytes: 0,
        endocrine_feedback_assets: 0,
        endocrine_feedback_contract_validated: false,
        no_action_authority: true,
    };
    app.insert_resource(GraphicalTrue25dPresentationResource {
        asset_manifest: manifest,
        versioned_gltf_pack_validated: false,
        runtime_gltf_scene_rendering: false,
        runtime_native_low_poly_fallback: true,
        fixed_orthographic_camera: true,
        preprocessed_repeating_ground_plane: false,
        synchronous_runtime_ground_texture_generation: true,
        ground_texture_path: TRUE_25D_RUNTIME_BIOME_TEXTURE_PATH,
        toon_bands: 4,
        sobel_outline_contract: true,
        pixel_step_filter_contract: true,
        procedural_micro_ecology_chunks: true,
        offscreen_headless_chunks: true,
        viewport_render_bypass: true,
        offscreen_zero_draw_call_contract: true,
        no_action_authority: true,
    });
    app.insert_resource(GraphicalTrue25dStylizationRenderPassResource {
        shader_path: TRUE_25D_STYLIZATION_SHADER_PATH,
        shader_source_embedded: true_25d_stylization_shader_source_is_complete(),
        runtime_render_graph_registered: false,
        attached_to_player_camera: true,
        pixel_grid_width: TRUE_25D_STYLIZATION_PIXEL_GRID.x as u32,
        pixel_grid_height: TRUE_25D_STYLIZATION_PIXEL_GRID.y as u32,
        toon_bands: TRUE_25D_STYLIZATION_TOON_BANDS as u8,
        depth_sobel_outline: true,
        luminance_sobel_fallback: true,
        low_resolution_pixel_step_filter: true,
        display_only: true,
        no_action_authority: true,
    });

    app.world_mut().spawn((
        Name::new("A-Life true 2.5D launch-baseline locked camera"),
        Camera3d::default(),
        Camera {
            order: 0,
            ..default()
        },
        true_25d_camera_projection(),
        true_25d_camera_transform(),
        GraphicalTrue25dCamera {
            orthographic_locked: true,
            pitch_degrees: -45.0,
            yaw_degrees: 0.0,
        },
        GraphicalTrue25dStylizationSettings::default(),
    ));
    spawn_true_25d_launch_baseline_ground_plane(&mut app, ca44a_launch_baseline_seed(launch));
    let baseline_elapsed_ms = started.elapsed().as_secs_f64() * 1_000.0;

    let presentation = app
        .world()
        .get_resource::<GraphicalTrue25dPresentationResource>()
        .cloned()
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "true 2.5D presentation receipt missing from launch baseline smoke",
        })?;
    let stylization = app
        .world()
        .get_resource::<GraphicalTrue25dStylizationRenderPassResource>()
        .copied()
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "true 2.5D stylization receipt missing from launch baseline smoke",
        })?;

    let (
        camera_count,
        fixed_orthographic_camera,
        camera_fixed_vertical_height,
        camera_position,
        camera_points_at_origin,
        camera_non_rotating_locked,
    ) = {
        let mut camera_query = app
            .world_mut()
            .query::<(&GraphicalTrue25dCamera, &Transform, &Projection)>();
        let cameras = camera_query
            .iter(app.world())
            .map(|(camera, transform, projection)| (*camera, *transform, projection.clone()))
            .collect::<Vec<_>>();
        let expected_transform = true_25d_camera_transform();
        let Some((camera, transform, projection)) = cameras.first() else {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "true 2.5D launch baseline found no locked camera",
            });
        };
        let (height, projection_locked) = if let Projection::Orthographic(orthographic) = projection
        {
            match orthographic.scaling_mode {
                ScalingMode::FixedVertical { viewport_height } => (
                    viewport_height,
                    (viewport_height - TRUE_25D_VIEWPORT_VERTICAL_UNITS).abs() <= f32::EPSILON
                        && (orthographic.scale - 1.0).abs() <= f32::EPSILON,
                ),
                _ => (0.0, false),
            }
        } else {
            (0.0, false)
        };
        let rotation_matches = transform.rotation.dot(expected_transform.rotation).abs() > 0.9999;
        (
            cameras.len(),
            projection_locked,
            height,
            [
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
            ],
            transform.translation == expected_transform.translation && rotation_matches,
            camera.orthographic_locked
                && camera.pitch_degrees == -45.0
                && camera.yaw_degrees == 0.0,
        )
    };

    let (ground_plane_count, ground_plane) = {
        let mut ground_query = app.world_mut().query::<&GraphicalTrue25dGroundPlane>();
        let grounds = ground_query.iter(app.world()).copied().collect::<Vec<_>>();
        (grounds.len(), grounds.first().copied())
    };
    let Some(ground_plane) = ground_plane else {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "true 2.5D launch baseline found no ground plane",
        });
    };

    let biome_map = {
        let mut biome_query = app
            .world_mut()
            .query::<&GraphicalRuntimeProceduralBiomeMap>();
        biome_query.iter(app.world()).copied().next().ok_or(
            GameAppShellError::VisibleWorldMismatch {
                message: "true 2.5D launch baseline found no biome map receipt",
            },
        )?
    };
    let field = app
        .world()
        .get_resource::<GraphicalProceduralTerrainFieldResource>()
        .cloned()
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "true 2.5D launch baseline found no terrain field receipt",
        })?;

    let synchronous_runtime_noise_generation = biome_map.generated_from_procedural_sampler;
    let synchronous_runtime_texture_generation = ground_plane
        .synchronous_runtime_texture_generation
        || biome_map.synchronous_texture_generation
        || presentation.synchronous_runtime_ground_texture_generation;
    let summary = GraphicalTrue25dLaunchBaselineSummary {
        schema: TRUE_25D_LAUNCH_BASELINE_SCHEMA,
        schema_version: TRUE_25D_LAUNCH_BASELINE_SCHEMA_VERSION,
        scope: "camera-ground-baseline-no-window",
        baseline_elapsed_ms,
        baseline_under_50ms: baseline_elapsed_ms <= TRUE_25D_LAUNCH_BASELINE_MAX_MS,
        bevy_window_created: false,
        cold_process_launch_measured: false,
        cold_process_under_50ms_claim: false,
        fixed_orthographic_camera: camera_count == 1
            && fixed_orthographic_camera
            && presentation.fixed_orthographic_camera,
        camera_fixed_vertical_height,
        camera_position,
        camera_points_at_origin,
        camera_non_rotating_locked,
        single_static_primitive_ground_plane: ground_plane_count == 1
            && ground_plane.static_primitive_plane,
        ground_tile_path: ground_plane.texture_path,
        ground_tile_width_px: biome_map.texture_width_px,
        ground_tile_height_px: biome_map.texture_height_px,
        texture_address_mode_repeat: ground_plane.sampler_repeat_wrapped
            && biome_map.sampler_repeat_wrapped,
        preprocessed_diffuse_tile: biome_map.rendered_from_preprocessed_ground_tile
            && biome_map.generated_from_alpha_art_tiles
            && ground_plane.texture_path == TRUE_25D_RUNTIME_BIOME_TEXTURE_PATH,
        synchronous_runtime_noise_generation,
        synchronous_runtime_texture_generation,
        zero_sync_runtime_noise_or_texture_generation: !synchronous_runtime_noise_generation
            && !synchronous_runtime_texture_generation,
        procedural_chunk_data_ledger_only: field.generated_without_rendering
            && field.procedural_content_generated_without_rendering
            && !field.procedural_content_rendering_required,
        stylization_shader_embedded: stylization.shader_source_embedded
            && stylization.low_resolution_pixel_step_filter
            && stylization.depth_sobel_outline,
        no_action_authority: presentation.no_action_authority,
        no_weight_authority: true,
        headless_path_preserved: true,
        gpu_authority_preserved: true,
        full_action_authoritative_claim: false,
    };
    Ok(summary)
}

fn spawn_true_25d_launch_baseline_ground_plane(app: &mut App, seed: u64) {
    let field = true_25d_ground_texture_field(seed);
    if !app.world().contains_resource::<Assets<Mesh>>() {
        app.init_resource::<Assets<Mesh>>();
    }
    let mesh =
        app.world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(true_25d_repeating_ground_plane_mesh(
                TRUE_25D_GROUND_WIDTH,
                TRUE_25D_GROUND_DEPTH,
                TRUE_25D_GROUND_UV_SPAN_X,
                TRUE_25D_GROUND_UV_SPAN_Z,
            ));
    let (image, metrics) =
        ca44a_generate_runtime_procedural_biome_map_with_pixels_per_tile(seed, &field, 2);
    let texture_width_px = image.texture_descriptor.size.width;
    let texture_height_px = image.texture_descriptor.size.height;

    app.world_mut().spawn((
        Name::new("A-Life true 2.5D launch-baseline seeded biome substrate"),
        Mesh3d(mesh),
        Transform::from_xyz(0.0, -0.02, 0.0),
        GraphicalTrue25dGroundPlane {
            texture_path: TRUE_25D_RUNTIME_BIOME_TEXTURE_PATH,
            width_world_units: TRUE_25D_GROUND_WIDTH,
            depth_world_units: TRUE_25D_GROUND_DEPTH,
            uv_repeat_x: TRUE_25D_GROUND_UV_SPAN_X,
            uv_repeat_z: TRUE_25D_GROUND_UV_SPAN_Z,
            sampler_repeat_wrapped: false,
            static_primitive_plane: true,
            synchronous_runtime_texture_generation: true,
        },
        GraphicalTrue25dAsset {
            role: "terrain-seeded-biome-ground-plane",
            stable_id: None,
            display_only: true,
        },
        GraphicalProductionArtLayer {
            role: "true-25d-seeded-biome-ground-plane-launch-baseline",
            display_only: true,
        },
        GraphicalRuntimeProceduralBiomeMap {
            seed,
            width_tiles: CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES,
            height_tiles: CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES,
            texture_width_px,
            texture_height_px,
            pixels_per_tile: 2,
            virtual_map_width_tiles: field.virtual_map_width_tiles,
            virtual_map_height_tiles: field.virtual_map_height_tiles,
            path_pixels: metrics.path_pixels,
            resource_detail_pixels: metrics.resource_detail_pixels,
            hazard_detail_pixels: metrics.hazard_detail_pixels,
            stone_detail_pixels: metrics.stone_detail_pixels,
            fogged_pixels: metrics.fogged_pixels,
            active_chunk_count: field.active_world_chunks.len(),
            dark_gap_pixels: metrics.dark_gap_pixels,
            generated_from_procedural_sampler: true,
            generated_from_alpha_art_tiles: metrics.alpha_art_tile_pixels > 0,
            rendered_from_preprocessed_ground_tile: false,
            sampler_repeat_wrapped: false,
            synchronous_texture_generation: true,
            texture_source_path: TRUE_25D_RUNTIME_BIOME_TEXTURE_PATH,
            terrain_tile_source_count: metrics.terrain_tile_source_count,
            fog_of_war_applied: metrics.fogged_pixels > 0,
            primary_player_surface: true,
            display_only: true,
            active_chunk_signature: ca44a_active_chunk_signature(&field),
            refresh_count: 0,
            last_creature_anchor_count: field.creature_anchor_count,
            last_materialized_tile_count: field.materialized_tiles.len(),
        },
    ));
    app.insert_resource(field);
}

pub fn build_graphical_playground_runtime_preview_app_shell(
    launch: &GraphicalPlaygroundLaunchConfig,
) -> Result<(App, GraphicalPlaygroundLaunchSummary), GameAppShellError> {
    let startup = run_headless_app_shell_smoke(&launch.app_launch)?;
    let summary = crate::validate_graphical_playground_launch(launch)?;
    let presentation = load_visible_world_from_p34_save(&launch.app_launch)?;
    crate::compare_visible_world_to_headless(&presentation)?;
    let population_summary = ca18_graphical_population_summary(&presentation).ok();
    let ecology_summary = ca19_graphical_ecology_summary(&launch.app_launch).ok();
    let world_art_summary = ca37_world_art_style_summary(&launch.app_launch).ok();
    let animation_summary = crate::ca38_creature_animation_summary().ok();
    let lifecycle_summary = ca20_graphical_lifecycle_summary().ok();
    let school_summary = crate::run_graphical_school_mode_smoke()?;
    let mut app = build_minimal_bevy_app_shell(startup);
    app.insert_resource(GraphicalAlphaArtHandles::unloaded_for_validation());
    app.insert_resource(GraphicalPlaygroundSceneResource {
        summary: summary.clone(),
    });
    app.insert_resource(GraphicalViewModeResource {
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
    if summary.view_mode == GraphicalPlaygroundViewMode::Player {
        spawn_true_25d_neurochemical_visual_feedback(
            &mut app,
            &inspector,
            &gpu_telemetry.telemetry,
        );
    }
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
        });
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
    add_graphical_runtime_core_update_systems(&mut app);
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
    spawn_graphical_intent_feedback(&mut app, summary.view_mode);
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
    if launch.require_gpu && !gpu.authoritative {
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

pub fn build_production_voxel_frontend_app_shell(
    launch: &crate::ProductionVoxelLaunchConfig,
) -> Result<(App, crate::ProductionVoxelLaunchSummary), GameAppShellError> {
    let summary = crate::run_production_voxel_frontend_preflight(launch)?;
    let mut app = App::new();
    if launch.dry_run {
        app.add_plugins(MinimalPlugins);
        app.init_resource::<Assets<Mesh>>();
        app.init_resource::<Assets<StandardMaterial>>();
        #[cfg(feature = "vfx-hanabi")]
        app.init_resource::<Assets<bevy_hanabi::prelude::EffectAsset>>();
        app.init_resource::<ButtonInput<KeyCode>>();
        app.init_resource::<ButtonInput<MouseButton>>();
    } else {
        let present_mode = if launch.record_performance {
            PresentMode::Immediate
        } else {
            PresentMode::AutoVsync
        };
        app.add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: production_voxel_asset_root(),
                    ..default()
                })
                .set(production_voxel_render_plugin(launch.record_performance))
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: summary.window_title.clone(),
                        name: Some("alife.production_voxel_frontend".to_string()),
                        resolution: summary.resolution.into(),
                        present_mode,
                        window_theme: Some(WindowTheme::Dark),
                        ..default()
                    }),
                    exit_condition: ExitCondition::OnPrimaryClosed,
                    ..default()
                }),
        );
        #[cfg(feature = "vfx-hanabi")]
        app.add_plugins(bevy_hanabi::prelude::HanabiPlugin);
    }
    app.add_plugins(AlifeBevyAdapterPlugin)
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(Color::srgb(0.065, 0.105, 0.090)))
        .insert_resource(ProductionVoxelFrontendResource {
            summary: summary.clone(),
        });
    let initial_authority = crate::GpuBrainAuthorityTelemetry::pending(
        summary
            .gpu_runtime_state
            .class_bucket_allocations
            .first()
            .and_then(|allocation| allocation.brain_class.neuron_count())
            .map_or_else(|| "unknown".to_string(), |count| format!("N{count}")),
    );
    app.insert_resource(ProductionGpuBrainAuthorityResource {
        telemetry: initial_authority,
    });
    #[cfg(feature = "gpu-runtime")]
    {
        let runtime_launch = prepare_production_gpu_runtime_launch(launch, &summary)?;
        let backend = alife_gpu_backend::GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .map_err(|error| GameAppShellError::NeuralBackendUnavailable {
            message: error.to_string(),
        })?;
        let runtime = crate::GpuLiveBrainRuntime::from_p34_launch(backend, &runtime_launch)?;
        let telemetry = runtime.authority_telemetry();
        app.insert_resource(ProductionGpuBrainAuthorityResource { telemetry })
            .insert_resource(ProductionGpuBrainTickScheduleResource::new(
                PRODUCTION_GPU_STARTUP_RENDER_FRAMES,
            ))
            .insert_resource(ProductionGpuBrainRuntimeResource { runtime })
            .add_systems(Update, tick_production_gpu_brain);
    }
    crate::spawn_fvr03_production_voxel_scene(&mut app, &summary)?;
    if let Some(seconds) = launch.smoke_seconds {
        app.insert_resource(GraphicalPlaygroundSmokeTimer {
            started: Instant::now(),
            duration: Duration::from_secs(seconds as u64),
        })
        .add_systems(Update, close_after_graphical_smoke_timeout);
    }
    Ok((app, summary))
}

fn production_voxel_render_plugin(record_performance: bool) -> RenderPlugin {
    let mut wgpu_settings = WgpuSettings::default();
    if record_performance {
        wgpu_settings.instance_flags = wgpu::InstanceFlags::empty();
    }
    RenderPlugin {
        render_creation: RenderCreation::Automatic(wgpu_settings),
        synchronous_pipeline_compilation: false,
        debug_flags: default(),
    }
}

fn production_voxel_asset_root() -> String {
    graphical_playground_asset_root()
}

#[cfg(test)]
mod fvr11_asset_root_tests {
    use std::path::PathBuf;

    use super::production_voxel_asset_root;

    #[test]
    fn production_voxel_asset_root_contains_the_terrain_atlas() {
        let root = PathBuf::from(production_voxel_asset_root());
        assert!(root.ends_with("crates/alife_game_app/assets"));
        assert!(root
            .join("production_voxel_v1/terrain/terrain_albedo_atlas.png")
            .is_file());
    }
}

#[cfg(all(test, feature = "gpu-runtime"))]
mod production_gpu_tick_schedule_tests {
    use super::ProductionGpuBrainTickScheduleResource;

    #[test]
    fn first_gpu_world_tick_waits_for_the_startup_render_barrier() {
        let mut schedule = ProductionGpuBrainTickScheduleResource::new(12);
        for _ in 0..12 {
            assert!(!schedule.take_dispatch_permit());
        }
        assert!(schedule.take_dispatch_permit());
        assert!(schedule.take_dispatch_permit());
    }
}

pub fn run_production_voxel_frontend_window(
    launch: &crate::ProductionVoxelLaunchConfig,
) -> Result<crate::ProductionVoxelLaunchSummary, GameAppShellError> {
    let (mut app, mut summary) = build_production_voxel_frontend_app_shell(launch)?;
    app.run();
    if summary.state_trace.last() != Some(&crate::ProductionAppState::Shutdown) {
        summary
            .state_trace
            .push(crate::ProductionAppState::Shutdown);
    }
    Ok(summary)
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
    let true_25d_player_view = summary.view_mode == GraphicalPlaygroundViewMode::Player;
    if true_25d_player_view {
        app.world_mut().spawn((
            Camera2d,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            Tonemapping::None,
            GraphicalMainCamera,
        ));
    } else {
        app.world_mut().spawn((Camera2d, GraphicalMainCamera));
    }
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
    if true_25d_player_view {
        spawn_true_25d_player_view_layer(app, presentation, summary.seed, world_art)?;
    }
    if !true_25d_player_view {
        app.world_mut().spawn((
            Name::new("A-Life S01 ground plane"),
            Sprite {
                color: rgba_to_color(presentation.ground_material.rgba()),
                custom_size: Some(Vec2::new(860.0, 460.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -10.0),
            VisibleGroundPlane {
                shape: presentation.ground_shape,
                material: presentation.ground_material,
                rgba: presentation.ground_material.rgba(),
            },
            VisibleWorldDebugLabel("ground:p34-fixture".to_string()),
        ));
    }
    if !true_25d_player_view {
        if let Some(ecology) = ecology {
            spawn_ca19_terrain_zone_visuals(app, ecology, summary.view_mode);
        }
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

    if !true_25d_player_view {
        for object in &presentation.objects {
            spawn_graphical_object(app, object, summary.view_mode, alpha_art.as_ref())?;
        }
    }
    if !true_25d_player_view {
        if let Some(school) = school {
            spawn_ca23_school_teacher_markers(app, school, summary.view_mode);
        }
    }
    if !true_25d_player_view {
        spawn_graphical_intent_feedback(app, summary.view_mode);
        spawn_ca08_feedback_pulses(app, presentation, summary.view_mode, alpha_art.as_ref());
        spawn_ca18_social_proximity_cues(app, presentation, summary.view_mode);
    }
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
    let true_25d_player_view = view_mode == GraphicalPlaygroundViewMode::Player
        && app
            .world()
            .contains_resource::<GraphicalTrue25dPresentationResource>();
    if true_25d_player_view {
        return;
    }
    spawn_ca37_world_art_terrain_canvas(
        app,
        summary,
        alpha_art,
        view_mode,
        presentation,
        !true_25d_player_view,
    );
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
    render_player_surface: bool,
) {
    if view_mode == GraphicalPlaygroundViewMode::FullDebug {
        if let Some(handles) = alpha_art {
            ca44a_spawn_world_backdrop_app(app, handles, view_mode);
        }
    }
    let config = ca44a_procedural_world_config(summary);
    let anchors = ca44a_procedural_world_anchors_from_presentation(presentation);
    let activation = activate_procedural_chunks_around_creatures(config, &anchors)
        .expect("graphical procedural world activation should validate");
    let mut field = GraphicalProceduralTerrainFieldResource::new(summary, &activation);
    if view_mode == GraphicalPlaygroundViewMode::Player && render_player_surface {
        ca44a_spawn_runtime_procedural_biome_map_app(app, summary, &field);
    }
    for (center_x, center_z, anchor) in
        ca44a_initial_procedural_terrain_centers(&activation, field.chunk_tile_size)
    {
        ca44a_materialize_terrain_chunk_app(
            app,
            summary,
            &mut field,
            alpha_art,
            view_mode,
            render_player_surface,
            center_x,
            center_z,
            anchor,
        );
    }
    if let Ok(content) = generate_procedural_world_content(config, &activation) {
        field.record_content_report(&content);
        ca44a_spawn_procedural_world_content_app(
            app,
            &mut field,
            alpha_art,
            view_mode,
            render_player_surface,
            &content,
        );
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
    let (image, metrics) = ca44a_generate_runtime_procedural_biome_map(summary.seed, field);
    let texture_width_px = image.texture_descriptor.size.width;
    let texture_height_px = image.texture_descriptor.size.height;
    let handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.world_mut().spawn((
        Name::new("A-Life runtime procedural biome map"),
        Sprite {
            image: handle,
            color: Color::WHITE,
            custom_size: Some(Vec2::new(
                CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as f32 * GRAPHICAL_WORLD_SCALE,
                CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as f32 * GRAPHICAL_WORLD_SCALE,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, -1.96),
        GraphicalRuntimeProceduralBiomeMap {
            seed: summary.seed,
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
            fogged_pixels: metrics.fogged_pixels,
            active_chunk_count: field.active_world_chunks.len(),
            dark_gap_pixels: metrics.dark_gap_pixels,
            generated_from_procedural_sampler: true,
            generated_from_alpha_art_tiles: metrics.alpha_art_tile_pixels > 0,
            rendered_from_preprocessed_ground_tile: false,
            sampler_repeat_wrapped: false,
            synchronous_texture_generation: true,
            texture_source_path: "runtime-generated-procedural-biome-map",
            terrain_tile_source_count: metrics.terrain_tile_source_count,
            fog_of_war_applied: metrics.fogged_pixels > 0,
            primary_player_surface: true,
            display_only: true,
            active_chunk_signature: ca44a_active_chunk_signature(field),
            refresh_count: 0,
            last_creature_anchor_count: field.creature_anchor_count,
            last_materialized_tile_count: field.materialized_tiles.len(),
        },
        GraphicalProductionArtLayer {
            role: "runtime-procedural-biome-map",
            display_only: true,
        },
    ));
}

fn ca44a_active_chunk_signature(field: &GraphicalProceduralTerrainFieldResource) -> u64 {
    let mut hash = field.seed.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (field.creature_anchor_count as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    for (chunk_x, chunk_z) in &field.active_world_chunks {
        let x = (*chunk_x as i64 as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
        let z = (*chunk_z as i64 as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
        hash ^= x.rotate_left(13) ^ z.rotate_right(7);
        hash = hash.rotate_left(17).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    }
    hash
}

fn update_graphical_runtime_procedural_biome_map(
    field: Option<Res<GraphicalProceduralTerrainFieldResource>>,
    images: Option<ResMut<Assets<Image>>>,
    mut biome_maps: bevy::prelude::Query<(&mut GraphicalRuntimeProceduralBiomeMap, &Sprite)>,
) {
    let Some(field) = field else {
        return;
    };
    let Some(mut images) = images else {
        return;
    };
    let signature = ca44a_active_chunk_signature(&field);
    for (mut biome_map, sprite) in &mut biome_maps {
        if biome_map.active_chunk_signature == signature
            && biome_map.last_creature_anchor_count == field.creature_anchor_count
        {
            continue;
        }
        let (image, metrics) = ca44a_generate_runtime_procedural_biome_map(field.seed, &field);
        let texture_width_px = image.texture_descriptor.size.width;
        let texture_height_px = image.texture_descriptor.size.height;
        if let Some(existing) = images.get_mut(&sprite.image) {
            *existing = image;
        } else {
            continue;
        }
        biome_map.texture_width_px = texture_width_px;
        biome_map.texture_height_px = texture_height_px;
        biome_map.path_pixels = metrics.path_pixels;
        biome_map.resource_detail_pixels = metrics.resource_detail_pixels;
        biome_map.hazard_detail_pixels = metrics.hazard_detail_pixels;
        biome_map.stone_detail_pixels = metrics.stone_detail_pixels;
        biome_map.fogged_pixels = metrics.fogged_pixels;
        biome_map.active_chunk_count = field.active_world_chunks.len();
        biome_map.dark_gap_pixels = metrics.dark_gap_pixels;
        biome_map.generated_from_alpha_art_tiles = metrics.alpha_art_tile_pixels > 0;
        biome_map.terrain_tile_source_count = metrics.terrain_tile_source_count;
        biome_map.fog_of_war_applied = metrics.fogged_pixels > 0;
        biome_map.active_chunk_signature = signature;
        biome_map.refresh_count = biome_map.refresh_count.saturating_add(1);
        biome_map.last_creature_anchor_count = field.creature_anchor_count;
        biome_map.last_materialized_tile_count = field.materialized_tiles.len();
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct RuntimeProceduralBiomeMapMetrics {
    path_pixels: u32,
    resource_detail_pixels: u32,
    hazard_detail_pixels: u32,
    stone_detail_pixels: u32,
    fogged_pixels: u32,
    dark_gap_pixels: u32,
    alpha_art_tile_pixels: u32,
    terrain_tile_source_count: u32,
}

#[derive(Debug, Clone, Copy)]
struct RuntimeBiomeBasePixel {
    rgb: [u8; 3],
    is_path: bool,
}

#[derive(Debug, Clone)]
struct RuntimeBiomeTileImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone)]
struct RuntimeBiomeTileSet {
    safe_grass: RuntimeBiomeTileImage,
    soil_path: RuntimeBiomeTileImage,
    resource_grove: RuntimeBiomeTileImage,
    hazard_pressure: RuntimeBiomeTileImage,
    stone_rough: RuntimeBiomeTileImage,
    water: RuntimeBiomeTileImage,
    sand: RuntimeBiomeTileImage,
}

impl RuntimeBiomeTileSet {
    fn source_count(&self) -> u32 {
        7
    }

    fn for_material(&self, material_id: &str) -> &RuntimeBiomeTileImage {
        match material_id {
            "neutral-soil" => &self.soil_path,
            "resource-grove" => &self.resource_grove,
            "hazard-pressure" => &self.hazard_pressure,
            "stone-dressing" => &self.stone_rough,
            "water" => &self.water,
            "sand" => &self.sand,
            _ => &self.safe_grass,
        }
    }
}

fn ca44a_runtime_biome_tile_set() -> RuntimeBiomeTileSet {
    RuntimeBiomeTileSet {
        safe_grass: ca44a_decode_runtime_biome_tile(
            include_bytes!("../assets/alpha_art_v1/terrain_safe_grass.png"),
            "terrain_safe_grass.png",
        ),
        soil_path: ca44a_decode_runtime_biome_tile(
            include_bytes!("../assets/alpha_art_v1/terrain_soil_path.png"),
            "terrain_soil_path.png",
        ),
        resource_grove: ca44a_decode_runtime_biome_tile(
            include_bytes!("../assets/alpha_art_v1/terrain_resource_grove.png"),
            "terrain_resource_grove.png",
        ),
        hazard_pressure: ca44a_decode_runtime_biome_tile(
            include_bytes!("../assets/alpha_art_v1/terrain_hazard_pressure.png"),
            "terrain_hazard_pressure.png",
        ),
        stone_rough: ca44a_decode_runtime_biome_tile(
            include_bytes!("../assets/alpha_art_v1/terrain_stone_rough.png"),
            "terrain_stone_rough.png",
        ),
        water: ca44a_decode_runtime_biome_tile(
            include_bytes!("../assets/alpha_art_v1/terrain_water.png"),
            "terrain_water.png",
        ),
        sand: ca44a_decode_runtime_biome_tile(
            include_bytes!("../assets/alpha_art_v1/terrain_sand.png"),
            "terrain_sand.png",
        ),
    }
}

fn ca44a_decode_runtime_biome_tile(bytes: &[u8], name: &'static str) -> RuntimeBiomeTileImage {
    let mut image = Image::from_buffer(
        bytes,
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::linear(),
        RenderAssetUsages::default(),
    )
    .unwrap_or_else(|_| panic!("failed to decode committed alpha terrain tile {name}"));
    let size = image.texture_descriptor.size;
    let data = image
        .data
        .take()
        .unwrap_or_else(|| panic!("decoded alpha terrain tile {name} had no pixel data"));
    let expected = size.width as usize * size.height as usize * 4;
    assert!(
        data.len() >= expected,
        "decoded alpha terrain tile {name} has {} bytes, expected at least {expected}",
        data.len()
    );
    RuntimeBiomeTileImage {
        data,
        width: size.width,
        height: size.height,
    }
}

fn ca44a_generate_runtime_procedural_biome_map(
    seed: u64,
    field: &GraphicalProceduralTerrainFieldResource,
) -> (Image, RuntimeProceduralBiomeMapMetrics) {
    ca44a_generate_runtime_procedural_biome_map_with_pixels_per_tile(
        seed,
        field,
        CA44A_RUNTIME_BIOME_MAP_PIXELS_PER_TILE,
    )
}

fn ca44a_generate_runtime_procedural_biome_map_with_pixels_per_tile(
    seed: u64,
    field: &GraphicalProceduralTerrainFieldResource,
    pixels_per_tile: u32,
) -> (Image, RuntimeProceduralBiomeMapMetrics) {
    let pixels_per_tile = pixels_per_tile.max(2);
    let width_px = (CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as u32) * pixels_per_tile;
    let height_px = (CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as u32) * pixels_per_tile;
    let mut data = Vec::with_capacity((width_px * height_px * 4) as usize);
    let mut metrics = RuntimeProceduralBiomeMapMetrics::default();
    let terrain_tiles = ca44a_runtime_biome_tile_set();
    metrics.terrain_tile_source_count = terrain_tiles.source_count();
    let half_width_tiles = CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES / 2;
    let half_height_tiles = CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES / 2;
    for y in 0..height_px {
        let tile_z =
            (height_px.saturating_sub(1) - y) as i32 / pixels_per_tile as i32 - half_height_tiles;
        for x in 0..width_px {
            let tile_x = x as i32 / pixels_per_tile as i32 - half_width_tiles;
            let local_x = (x % pixels_per_tile) as f32 / pixels_per_tile as f32;
            let local_y = (y % pixels_per_tile) as f32 / pixels_per_tile as f32;
            let world_x = tile_x as f32 + local_x - 0.5;
            let world_z = tile_z as f32 + (1.0 - local_y) - 0.5;
            let mut pixel =
                ca44a_runtime_biome_tile_pixel(seed, &terrain_tiles, world_x, world_z, x, y);
            metrics.alpha_art_tile_pixels += 1;
            if ca44a_apply_runtime_fog_of_war(field, world_x, world_z, &mut pixel) {
                metrics.fogged_pixels += 1;
            }
            if u16::from(pixel.rgb[0]) + u16::from(pixel.rgb[1]) + u16::from(pixel.rgb[2]) < 92 {
                pixel.rgb = ca44a_blend_rgb_u8(pixel.rgb, [78, 86, 68], 0.68);
            }
            if pixel.is_path {
                metrics.path_pixels += 1;
            }
            let [r, g, b] = pixel.rgb;
            if u16::from(r) + u16::from(g) + u16::from(b) < 92 {
                metrics.dark_gap_pixels += 1;
            }
            data.extend_from_slice(&[r, g, b, 255]);
        }
    }
    ca44a_paint_runtime_biome_regions(seed, field, width_px, height_px, &mut data);
    ca44a_paint_runtime_biome_tile_edges(seed, field, width_px, height_px, &mut data);
    ca44a_paint_runtime_biome_trails(seed, field, width_px, height_px, &mut data, &mut metrics);
    ca44a_paint_runtime_biome_dressing(seed, field, width_px, height_px, &mut data, &mut metrics);
    ca44a_apply_runtime_biome_color_grade(width_px, height_px, &mut data);
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
    image.sampler = ImageSampler::nearest();
    (image, metrics)
}

fn ca44a_runtime_biome_tile_pixel(
    seed: u64,
    terrain_tiles: &RuntimeBiomeTileSet,
    world_x: f32,
    world_z: f32,
    pixel_x: u32,
    pixel_y: u32,
) -> RuntimeBiomeBasePixel {
    let tile_x = world_x.floor() as i32;
    let tile_z = world_z.floor() as i32;
    let local_x = (world_x - tile_x as f32).rem_euclid(1.0);
    let local_y = (world_z - tile_z as f32).rem_euclid(1.0);

    let soil = ca44a_runtime_soil_weight(world_x, world_z);
    let resource = ca44a_runtime_resource_weight(world_x, world_z);
    let hazard = ca44a_runtime_hazard_weight(world_x, world_z);
    let stone = ca44a_runtime_stone_weight(world_x, world_z);
    let water = ca44a_runtime_water_weight(world_x, world_z);
    let sand = ca44a_runtime_sand_weight(world_x, world_z);
    let pressure = soil
        .max(resource)
        .max(hazard)
        .max(stone)
        .max(water)
        .max(sand);
    let hazard_gate = (1.0 - hazard * 1.35).clamp(0.0, 1.0);
    let stone_gate = (1.0 - stone * 1.18).clamp(0.0, 1.0);
    let water_gate = (1.0 - water * 1.25).clamp(0.0, 1.0);
    let safe = ((0.48 - pressure * 0.38) * hazard_gate * stone_gate * water_gate).clamp(0.0, 0.52);

    let mut sources = [
        ("safe-grass", safe),
        ("neutral-soil", soil * 1.22 * hazard_gate * water_gate),
        ("resource-grove", resource * 1.34 * hazard_gate * stone_gate),
        ("hazard-pressure", hazard * (1.56 + hazard * 0.36)),
        ("stone-dressing", stone * 1.42 * water_gate),
        ("water", water * 1.48),
        ("sand", sand * 1.16 * (1.0 - hazard * 0.38).clamp(0.34, 1.0)),
    ];
    sources.sort_by(|(_, left), (_, right)| {
        right.partial_cmp(left).unwrap_or(std::cmp::Ordering::Equal)
    });
    let (dominant_material_id, dominant_weight) = sources[0];
    let dominant_material_id = if dominant_weight > 0.01 {
        dominant_material_id
    } else {
        "safe-grass"
    };
    let tile_rgb = ca44a_runtime_weighted_tile_rgb(
        seed,
        terrain_tiles,
        &sources,
        tile_x,
        tile_z,
        pixel_x,
        pixel_y,
        local_x,
        local_y,
    );
    let base_pixel = ca44a_runtime_biome_base_pixel(seed, world_x, world_z, pixel_x, pixel_y);
    let mut rgb = ca44a_blend_rgb_u8(
        base_pixel.rgb,
        tile_rgb,
        ca44a_runtime_tile_blend(dominant_material_id),
    );
    let micro_noise =
        (ca44a_runtime_pixel_hash(seed ^ 0xA117_71E5, tile_x, tile_z, pixel_x, pixel_y) % 3) as i16
            - 1;
    for channel in &mut rgb {
        *channel = ((*channel as i16) + micro_noise).clamp(24, 238) as u8;
    }
    RuntimeBiomeBasePixel {
        rgb,
        is_path: dominant_material_id == "neutral-soil" || soil > 0.46,
    }
}

#[allow(clippy::too_many_arguments)]
fn ca44a_runtime_weighted_tile_rgb(
    seed: u64,
    terrain_tiles: &RuntimeBiomeTileSet,
    ranked_sources: &[(&str, f32); 7],
    tile_x: i32,
    tile_z: i32,
    pixel_x: u32,
    pixel_y: u32,
    local_x: f32,
    local_y: f32,
) -> [u8; 3] {
    let (primary_id, primary_weight) = ranked_sources[0];
    let primary_id = if primary_weight > 0.01 {
        primary_id
    } else {
        "safe-grass"
    };
    let mut rgb = ca44a_sample_runtime_tile_rgb(
        primary_id,
        terrain_tiles.for_material(primary_id),
        seed,
        tile_x,
        tile_z,
        pixel_x,
        pixel_y,
        local_x,
        local_y,
    );
    for (material_id, weight) in ranked_sources.iter().skip(1).take(2) {
        if *weight <= 0.10 || primary_weight <= 0.001 {
            continue;
        }
        let blend = ((*weight / primary_weight) * 0.16).clamp(0.0, 0.12);
        let secondary = ca44a_sample_runtime_tile_rgb(
            material_id,
            terrain_tiles.for_material(material_id),
            seed ^ 0x9E37_4A7C,
            tile_x,
            tile_z,
            pixel_x,
            pixel_y,
            local_x,
            local_y,
        );
        rgb = ca44a_blend_rgb_u8(rgb, secondary, blend);
    }
    rgb
}

fn ca44a_sample_runtime_tile_rgb(
    material_id: &str,
    tile: &RuntimeBiomeTileImage,
    seed: u64,
    tile_x: i32,
    tile_z: i32,
    _pixel_x: u32,
    _pixel_y: u32,
    local_x: f32,
    local_y: f32,
) -> [u8; 3] {
    let material_salt = match material_id {
        "neutral-soil" => 0x15,
        "resource-grove" => 0x27,
        "hazard-pressure" => 0x39,
        "stone-dressing" => 0x4B,
        "water" => 0x5D,
        "sand" => 0x6F,
        _ => 0x7A,
    };
    let tile_hash = ca44a_runtime_pixel_hash(seed ^ 0x51A7_E111, material_salt, 0, 0, 0);
    let world_u = tile_x as f32 + local_x;
    let world_v = tile_z as f32 + local_y;
    let offset_u = ((tile_hash & 0xFF) as f32) / 255.0;
    let offset_v = (((tile_hash >> 8) & 0xFF) as f32) / 255.0;
    let material_scale = match material_id {
        "water" => 0.52,
        "sand" => 0.50,
        "stone-dressing" => 0.49,
        "hazard-pressure" => 0.54,
        "resource-grove" => 0.58,
        "neutral-soil" => 0.47,
        _ => 0.56,
    };
    let u = (world_u * material_scale + world_v * 0.031 + offset_u).rem_euclid(1.0);
    let v = (world_v * material_scale - world_u * 0.027 + offset_v).rem_euclid(1.0);
    let sample_x = ((u.clamp(0.0, 0.999) * tile.width as f32) as u32) % tile.width.max(1);
    let sample_y = ((v.clamp(0.0, 0.999) * tile.height as f32) as u32) % tile.height.max(1);
    let index = ((sample_y * tile.width + sample_x) * 4) as usize;
    let alpha = tile.data.get(index + 3).copied().unwrap_or(255) as f32 / 255.0;
    let mut source = [
        tile.data.get(index).copied().unwrap_or(0),
        tile.data.get(index + 1).copied().unwrap_or(0),
        tile.data.get(index + 2).copied().unwrap_or(0),
    ];
    let max_channel = source[0].max(source[1]).max(source[2]);
    let min_channel = source[0].min(source[1]).min(source[2]);
    if max_channel > 188 && max_channel.saturating_sub(min_channel) < 92 {
        source = ca44a_blend_rgb_u8(source, ca44a_runtime_material_base_rgb(material_id), 0.52);
    }
    ca44a_blend_rgb_u8(ca44a_runtime_material_base_rgb(material_id), source, alpha)
}

fn ca44a_runtime_tile_blend(material_id: &str) -> f32 {
    match material_id {
        "hazard-pressure" => 0.20,
        "water" => 0.20,
        "resource-grove" => 0.18,
        "stone-dressing" => 0.18,
        "neutral-soil" | "sand" => 0.16,
        _ => 0.12,
    }
}

fn ca44a_runtime_biome_base_pixel(
    seed: u64,
    world_x: f32,
    world_z: f32,
    pixel_x: u32,
    pixel_y: u32,
) -> RuntimeBiomeBasePixel {
    let tile_x = world_x.floor() as i32;
    let tile_z = world_z.floor() as i32;
    let fine_hash = ca44a_runtime_pixel_hash(seed, tile_x, tile_z, pixel_x, pixel_y);
    let sampled_material = sample_procedural_terrain_tile(
        ProceduralWorldConfig::with_seed(seed),
        ProceduralTileCoord::new(tile_x, tile_z),
    )
    .map(|sample| sample.material)
    .unwrap_or(alife_world::ProceduralTerrainMaterial::SafeGrass);
    let material_id = sampled_material.material_id();
    let sampled_rgb = ca44a_runtime_material_base_rgb(material_id);
    let continuous_rgb = ca44a_runtime_continuous_material_base_rgb(
        ca44a_runtime_soil_weight(world_x, world_z),
        ca44a_runtime_resource_weight(world_x, world_z),
        ca44a_runtime_hazard_weight(world_x, world_z),
        ca44a_runtime_stone_weight(world_x, world_z),
        ca44a_runtime_water_weight(world_x, world_z),
        ca44a_runtime_sand_weight(world_x, world_z),
    );
    let [base_r, base_g, base_b] = ca44a_blend_rgb_u8(sampled_rgb, continuous_rgb, 1.0);
    let local_x = world_x - tile_x as f32;
    let local_y = world_z - tile_z as f32;
    let noise = (fine_hash % 7) as i16 - 3;
    let organic_noise = ((world_x * 0.18).sin() * 2.0
        + (world_z * 0.21).cos() * 2.0
        + ((local_x - 0.5).abs() * -1.5)
        + ((local_y - 0.5).abs() * -1.5))
        .round() as i16;
    let material_lift = ((fine_hash % 5) as i16) - 2;
    let r = base_r as i16 + noise + organic_noise + material_lift / 2;
    let g = base_g as i16 + noise + organic_noise + material_lift;
    let b = base_b as i16 + noise / 2 + material_lift;

    RuntimeBiomeBasePixel {
        rgb: [
            r.clamp(46, 232) as u8,
            g.clamp(58, 234) as u8,
            b.clamp(42, 220) as u8,
        ],
        is_path: material_id == "neutral-soil",
    }
}

fn ca44a_apply_runtime_fog_of_war(
    field: &GraphicalProceduralTerrainFieldResource,
    world_x: f32,
    world_z: f32,
    pixel: &mut RuntimeBiomeBasePixel,
) -> bool {
    if field.creature_anchor_count == 0 || field.active_world_chunks.is_empty() {
        pixel.rgb = ca44a_blend_rgb_u8(pixel.rgb, [25, 31, 28], 0.55);
        return true;
    }
    let Some(min_distance) = ca44a_distance_to_active_chunk_center(field, world_x, world_z) else {
        pixel.rgb = ca44a_blend_rgb_u8(pixel.rgb, [25, 31, 28], 0.55);
        return true;
    };
    let chunk = field.chunk_tile_size.max(1) as f32;
    let clear_radius = chunk * 1.45;
    if min_distance <= clear_radius {
        return false;
    }
    let fade_radius = chunk * 3.35;
    let t = ((min_distance - clear_radius) / (fade_radius - clear_radius)).clamp(0.0, 1.0);
    let smooth_t = t * t * (3.0 - 2.0 * t);
    let alpha = 0.02 + smooth_t * 0.10;
    pixel.rgb = ca44a_blend_rgb_u8(pixel.rgb, [28, 35, 31], alpha);
    true
}

fn ca44a_distance_to_active_chunk_center(
    field: &GraphicalProceduralTerrainFieldResource,
    world_x: f32,
    world_z: f32,
) -> Option<f32> {
    let chunk = field.chunk_tile_size.max(1) as f32;
    field
        .active_world_chunks
        .iter()
        .map(|(chunk_x, chunk_z)| {
            let center_x = (*chunk_x as f32 + 0.5) * chunk;
            let center_z = (*chunk_z as f32 + 0.5) * chunk;
            let dx = world_x - center_x;
            let dz = world_z - center_z;
            (dx * dx + dz * dz).sqrt()
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
}

fn ca44a_paint_runtime_biome_regions(
    seed: u64,
    field: &GraphicalProceduralTerrainFieldResource,
    width_px: u32,
    height_px: u32,
    data: &mut [u8],
) -> u32 {
    let regions = [
        (-38.0, -23.0, 35.0, 13.0, [30, 142, 164], 0.68, 0xE9u32),
        (-33.0, -16.0, 29.0, 13.0, [228, 191, 103], 0.58, 0x23u32),
        (-36.0, 20.0, 41.0, 19.0, [99, 108, 101], 0.58, 0x51u32),
        (2.0, 13.0, 34.0, 17.0, [45, 158, 55], 0.52, 0xC3u32),
        (38.0, 0.0, 39.0, 24.0, [211, 54, 38], 0.72, 0xA7u32),
        (50.0, -24.0, 24.0, 13.0, [187, 158, 100], 0.52, 0x77u32),
        (-24.0, 2.0, 22.0, 9.0, [32, 148, 169], 0.58, 0xB4u32),
        (18.0, 6.0, 24.0, 15.0, [218, 61, 37], 0.56, 0x91u32),
        (-8.0, 21.0, 24.0, 11.0, [105, 115, 107], 0.50, 0x64u32),
        (0.0, -16.0, 28.0, 13.0, [44, 152, 52], 0.48, 0xD8u32),
        (-48.0, 5.0, 18.0, 11.0, [228, 191, 106], 0.46, 0x47u32),
        (45.0, 21.0, 21.0, 10.0, [88, 96, 92], 0.46, 0x33u32),
    ];
    regions
        .iter()
        .map(|(cx, cz, rx, rz, color, alpha, salt)| {
            ca44a_paint_runtime_biome_region(
                seed, field, width_px, height_px, data, *cx, *cz, *rx, *rz, *color, *alpha, *salt,
            )
        })
        .sum()
}

fn ca44a_paint_runtime_biome_tile_edges(
    seed: u64,
    field: &GraphicalProceduralTerrainFieldResource,
    width_px: u32,
    height_px: u32,
    data: &mut [u8],
) -> u32 {
    let pixels_per_tile = width_px / CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES.max(1) as u32;
    if pixels_per_tile < 4 {
        return 0;
    }
    let mut painted = 0;
    for tile_z in
        -CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES / 2..CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES / 2
    {
        for tile_x in
            -CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES / 2..CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES / 2
        {
            let center_x = tile_x as f32 + 0.5;
            let center_z = tile_z as f32 + 0.5;
            if !ca44a_world_point_in_active_fog_window(field, center_x, center_z) {
                continue;
            }
            let hash = ca44a_runtime_pixel_hash(seed ^ 0x771E_5EED, tile_x, tile_z, 0, 0);
            if hash % 4 != 0 {
                continue;
            }
            let (px, py) = ca44a_world_to_biome_pixel(center_x, center_z, width_px, height_px);
            let radius_x = (pixels_per_tile as i32 / 2).max(3);
            let radius_y = (pixels_per_tile as i32 / 4).max(2);
            let material_color = ca44a_runtime_continuous_material_base_rgb(
                ca44a_runtime_soil_weight(center_x, center_z),
                ca44a_runtime_resource_weight(center_x, center_z),
                ca44a_runtime_hazard_weight(center_x, center_z),
                ca44a_runtime_stone_weight(center_x, center_z),
                ca44a_runtime_water_weight(center_x, center_z),
                ca44a_runtime_sand_weight(center_x, center_z),
            );
            let highlight = ca44a_blend_rgb_u8(material_color, [235, 228, 158], 0.10);
            let shade = ca44a_blend_rgb_u8(material_color, [24, 32, 25], 0.14);
            painted += ca44a_paint_ellipse_rgba(
                data,
                width_px,
                height_px,
                px - radius_x / 3,
                py - radius_y / 3,
                radius_x,
                radius_y,
                highlight,
                0.018,
            );
            painted += ca44a_paint_line_rgba(
                data,
                width_px,
                height_px,
                px - radius_x,
                py + radius_y,
                px + radius_x,
                py + radius_y / 2,
                1,
                shade,
                0.020,
            );
        }
    }
    painted
}

#[allow(clippy::too_many_arguments)]
fn ca44a_paint_runtime_biome_region(
    seed: u64,
    field: &GraphicalProceduralTerrainFieldResource,
    width_px: u32,
    height_px: u32,
    data: &mut [u8],
    center_x: f32,
    center_z: f32,
    radius_x: f32,
    radius_z: f32,
    color: [u8; 3],
    max_alpha: f32,
    salt: u32,
) -> u32 {
    let (center_px, center_py) =
        ca44a_world_to_biome_pixel(center_x, center_z, width_px, height_px);
    let rx_px =
        (radius_x / CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as f32 * width_px as f32).round() as i32;
    let ry_px =
        (radius_z / CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as f32 * height_px as f32).round() as i32;
    let mut painted = 0;
    for y in (center_py - ry_px).max(0)..=(center_py + ry_px).min(height_px as i32 - 1) {
        for x in (center_px - rx_px).max(0)..=(center_px + rx_px).min(width_px as i32 - 1) {
            let world_x = (x as f32 / width_px as f32) * CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as f32
                - CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as f32 * 0.5;
            let world_z = CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as f32 * 0.5
                - (y as f32 / height_px as f32) * CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as f32;
            let fogged = !ca44a_world_point_in_active_fog_window(field, world_x, world_z);
            let dx = (world_x - center_x) / radius_x.max(0.001);
            let dz = (world_z - center_z) / radius_z.max(0.001);
            let distance = (dx * dx + dz * dz).sqrt();
            if distance > 1.0 {
                continue;
            }
            let hash = ca44a_runtime_pixel_hash(seed ^ u64::from(salt), x, y, salt as u32, 0);
            let edge = (1.0 - distance).clamp(0.0, 1.0).powf(0.62);
            let texture_breakup = ((world_x * 0.31 + salt as f32).sin()
                * (world_z * 0.27 - salt as f32).cos()
                * 0.07)
                + ((hash % 97) as f32 / 96.0 - 0.5) * 0.06;
            let fog_factor = if fogged { 0.72 } else { 1.0 };
            let alpha = (max_alpha * fog_factor * (0.22 + edge * 0.78 + texture_breakup))
                .clamp(0.0, max_alpha * fog_factor);
            if alpha <= 0.01 {
                continue;
            }
            ca44a_alpha_blend_pixel(data, width_px, x as u32, y as u32, color, alpha);
            painted += 1;
        }
    }
    painted
}

fn ca44a_paint_runtime_biome_trails(
    seed: u64,
    field: &GraphicalProceduralTerrainFieldResource,
    width_px: u32,
    height_px: u32,
    data: &mut [u8],
    metrics: &mut RuntimeProceduralBiomeMapMetrics,
) {
    let paths: [&[(f32, f32)]; 3] = [
        &[
            (-62.0, -12.0),
            (-44.0, -8.5),
            (-24.0, -5.0),
            (-3.0, -4.0),
            (22.0, -1.5),
            (61.0, 5.0),
        ],
        &[
            (-57.0, 17.0),
            (-39.0, 13.0),
            (-18.0, 9.0),
            (3.0, 3.0),
            (29.0, -7.0),
            (60.0, -17.0),
        ],
        &[
            (8.0, -34.0),
            (15.0, -22.0),
            (11.0, -12.0),
            (0.0, -5.0),
            (-15.0, 2.0),
            (-31.0, 8.5),
        ],
    ];
    for (index, path) in paths.iter().enumerate() {
        metrics.path_pixels += ca44a_paint_runtime_biome_trail(
            seed ^ (index as u64).wrapping_mul(0x9E37),
            field,
            width_px,
            height_px,
            data,
            path,
        );
    }
}

fn ca44a_paint_runtime_biome_trail(
    seed: u64,
    field: &GraphicalProceduralTerrainFieldResource,
    width_px: u32,
    height_px: u32,
    data: &mut [u8],
    path: &[(f32, f32)],
) -> u32 {
    let mut samples = Vec::new();
    for (segment_index, segment) in path.windows(2).enumerate() {
        let (x1, z1) = segment[0];
        let (x2, z2) = segment[1];
        let dx = x2 - x1;
        let dz = z2 - z1;
        let length = (dx * dx + dz * dz).sqrt().max(1.0);
        let nx = -dz / length;
        let nz = dx / length;
        let steps = ((length * 1.6).round() as usize).clamp(7, 42);
        for step in 0..steps {
            let t = step as f32 / steps as f32;
            let hash = ca44a_runtime_pixel_hash(seed, segment_index as i32, step as i32, 0, 0);
            let wobble = ((segment_index as f32 + t) * 3.4).sin() * 0.42
                + ((hash % 100) as f32 - 50.0) * 0.004;
            samples.push((x1 + dx * t + nx * wobble, z1 + dz * t + nz * wobble));
        }
    }
    if let Some(last) = path.last() {
        samples.push(*last);
    }

    let mut painted = 0;
    for (width, alpha, color) in [
        (10, 0.045, [83, 61, 39]),
        (7, 0.105, [148, 104, 61]),
        (3, 0.235, [215, 169, 91]),
    ] {
        for (a, b) in samples.iter().zip(samples.iter().skip(1)) {
            let (ax, ay) = ca44a_world_to_biome_pixel(a.0, a.1, width_px, height_px);
            let (bx, by) = ca44a_world_to_biome_pixel(b.0, b.1, width_px, height_px);
            if ca44a_world_point_in_active_fog_window(field, a.0, a.1)
                || ca44a_world_point_in_active_fog_window(field, b.0, b.1)
            {
                painted += ca44a_paint_line_rgba(
                    data, width_px, height_px, ax, ay, bx, by, width, color, alpha,
                );
            }
        }
    }
    for (sample_index, (x, z)) in samples.iter().step_by(5).enumerate() {
        let hash = ca44a_runtime_pixel_hash(seed ^ 0x7A11, sample_index as i32, 0, 0, 0);
        if hash % 3 != 0 {
            let (px, py) = ca44a_world_to_biome_pixel(*x, *z, width_px, height_px);
            if ca44a_world_point_in_active_fog_window(field, *x, *z) {
                painted += ca44a_paint_ellipse_rgba(
                    data,
                    width_px,
                    height_px,
                    px,
                    py,
                    7 + (hash % 7) as i32,
                    3 + ((hash >> 9) % 4) as i32,
                    [211, 169, 89],
                    0.16 + (hash % 17) as f32 / 120.0,
                );
            }
        }
    }
    painted
}

fn ca44a_paint_runtime_biome_dressing(
    seed: u64,
    field: &GraphicalProceduralTerrainFieldResource,
    width_px: u32,
    height_px: u32,
    data: &mut [u8],
    metrics: &mut RuntimeProceduralBiomeMapMetrics,
) {
    for index in 0..720 {
        let (world_x, world_z) = ca44a_runtime_resource_grove_point(seed, index);
        let resource_weight = ca44a_runtime_resource_weight(world_x, world_z);
        let hazard_weight = ca44a_runtime_hazard_weight(world_x, world_z);
        let stone_weight = ca44a_runtime_stone_weight(world_x, world_z);
        if resource_weight > 0.25
            && hazard_weight < 0.22
            && stone_weight < 0.58
            && ca44a_world_point_in_active_fog_window(field, world_x, world_z)
        {
            metrics.resource_detail_pixels +=
                ca44a_paint_tree_cluster(data, width_px, height_px, world_x, world_z, index, seed);
        }
    }
    for index in 0..260 {
        let (world_x, world_z) = ca44a_runtime_stone_cluster_point(seed, index);
        if (ca44a_runtime_stone_weight(world_x, world_z) > 0.42
            || ca44a_runtime_pixel_hash(seed, index, 17, 0, 0) % 23 == 0)
            && ca44a_world_point_in_active_fog_window(field, world_x, world_z)
        {
            metrics.stone_detail_pixels +=
                ca44a_paint_rock_cluster(data, width_px, height_px, world_x, world_z, index, seed);
        }
    }
    for index in 0..360 {
        let (world_x, world_z) = ca44a_runtime_hazard_cluster_point(seed, index);
        if ca44a_runtime_hazard_weight(world_x, world_z) > 0.28
            && ca44a_world_point_in_active_fog_window(field, world_x, world_z)
        {
            metrics.hazard_detail_pixels += ca44a_paint_crystal_cluster(
                data, width_px, height_px, world_x, world_z, index, seed,
            );
        }
    }
    for index in 0..760 {
        let (world_x, world_z) = ca44a_runtime_random_world_point(seed, index, 0xF10A_0E51);
        let resource_weight = ca44a_runtime_resource_weight(world_x, world_z);
        if resource_weight > 0.22
            && ca44a_runtime_hazard_weight(world_x, world_z) < 0.18
            && ca44a_world_point_in_active_fog_window(field, world_x, world_z)
        {
            metrics.resource_detail_pixels +=
                ca44a_paint_flower_sprout(data, width_px, height_px, world_x, world_z, index, seed);
        }
    }
}

fn ca44a_apply_runtime_biome_color_grade(width_px: u32, height_px: u32, data: &mut [u8]) {
    let width = width_px.max(1) as f32;
    let height = height_px.max(1) as f32;
    for y in 0..height_px {
        let v = y as f32 / height;
        for x in 0..width_px {
            let u = x as f32 / width;
            let dx = (u - 0.50).abs();
            let dy = (v - 0.50).abs();
            let edge = ((dx * 1.42).powi(2) + (dy * 1.24).powi(2))
                .sqrt()
                .clamp(0.0, 1.0);
            let sun = (1.09 - v * 0.07 - u * 0.025).clamp(0.96, 1.13);
            let center_pop = (1.0 + (1.0 - edge).powf(1.7) * 0.05).clamp(1.0, 1.05);
            let index = ((y * width_px + x) * 4) as usize;
            let mut r = data[index] as f32;
            let mut g = data[index + 1] as f32;
            let mut b = data[index + 2] as f32;
            let luminance = r * 0.30 + g * 0.55 + b * 0.15;
            let saturation = 1.12;
            r = luminance + (r - luminance) * saturation;
            g = luminance + (g - luminance) * 1.02;
            b = luminance + (b - luminance) * saturation;
            for channel in 0..3 {
                let value = match channel {
                    0 => r,
                    1 => g,
                    _ => b,
                };
                data[index + channel] = (value * sun * center_pop).clamp(0.0, 255.0) as u8;
            }
            if edge > 0.68 {
                let fog_alpha = ((edge - 0.68) / 0.32).clamp(0.0, 1.0) * 0.28;
                let fog_color = if v < 0.28 { [54, 65, 66] } else { [45, 55, 50] };
                ca44a_alpha_blend_pixel(data, width_px, x, y, fog_color, fog_alpha);
            }
        }
    }
}

fn ca44a_world_point_in_active_fog_window(
    field: &GraphicalProceduralTerrainFieldResource,
    world_x: f32,
    world_z: f32,
) -> bool {
    ca44a_distance_to_active_chunk_center(field, world_x, world_z)
        .map(|distance| distance <= field.chunk_tile_size.max(1) as f32 * 5.25)
        .unwrap_or(false)
}

fn ca44a_runtime_resource_grove_point(seed: u64, index: i32) -> (f32, f32) {
    let centers = [
        (-43.0, 16.0, 16.0, 11.0),
        (-26.0, -12.0, 19.0, 9.0),
        (2.0, 13.0, 18.0, 10.0),
        (18.0, -22.0, 19.0, 8.0),
        (-7.0, -26.0, 15.0, 7.0),
        (39.0, -18.0, 13.0, 7.0),
    ];
    ca44a_runtime_clustered_world_point(seed, index, 0xA11C_E001, &centers)
}

fn ca44a_runtime_hazard_cluster_point(seed: u64, index: i32) -> (f32, f32) {
    let centers = [
        (44.0, -6.0, 22.0, 15.0),
        (-43.0, -24.0, 13.0, 9.0),
        (22.0, 21.0, 9.0, 6.0),
    ];
    ca44a_runtime_clustered_world_point(seed, index, 0xF113_7A91, &centers)
}

fn ca44a_runtime_stone_cluster_point(seed: u64, index: i32) -> (f32, f32) {
    let centers = [
        (-30.0, 23.0, 21.0, 11.0),
        (9.0, 27.0, 20.0, 8.0),
        (52.0, -27.0, 13.0, 7.0),
        (-7.0, -5.0, 22.0, 9.0),
    ];
    ca44a_runtime_clustered_world_point(seed, index, 0xC742_9913, &centers)
}

fn ca44a_runtime_clustered_world_point(
    seed: u64,
    index: i32,
    salt: u32,
    centers: &[(f32, f32, f32, f32)],
) -> (f32, f32) {
    let hash = ca44a_runtime_pixel_hash(seed ^ u64::from(salt), index, salt as i32, 0, 0);
    let center = centers[(hash as usize) % centers.len()];
    let angle_hash = ca44a_runtime_pixel_hash(seed ^ 0x51C5_A11D, index, salt as i32, 1, 0);
    let radius_hash = ca44a_runtime_pixel_hash(seed ^ 0xA11D_5EED, index, salt as i32, 2, 0);
    let angle = (angle_hash as f32 / u32::MAX as f32) * std::f32::consts::TAU;
    let radius = (radius_hash as f32 / u32::MAX as f32).sqrt();
    (
        center.0 + angle.cos() * center.2 * radius,
        center.1 + angle.sin() * center.3 * radius,
    )
}

fn ca44a_runtime_random_world_point(seed: u64, index: i32, salt: u32) -> (f32, f32) {
    let hash_a = ca44a_runtime_pixel_hash(seed ^ u64::from(salt), index, salt as i32, 0, 0);
    let hash_b = ca44a_runtime_pixel_hash(
        seed ^ u64::from(salt.rotate_left(13)),
        index * 17 + 3,
        salt as i32,
        1,
        0,
    );
    let x_unit = (hash_a & 0xffff) as f32 / 65_535.0;
    let z_unit = (hash_b & 0xffff) as f32 / 65_535.0;
    let x = (x_unit - 0.5) * CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as f32;
    let z = (z_unit - 0.5) * CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as f32;
    (x, z)
}

fn ca44a_world_to_biome_pixel(
    world_x: f32,
    world_z: f32,
    width_px: u32,
    height_px: u32,
) -> (i32, i32) {
    let width_tiles = CA44A_RUNTIME_BIOME_MAP_WIDTH_TILES as f32;
    let height_tiles = CA44A_RUNTIME_BIOME_MAP_HEIGHT_TILES as f32;
    let x = ((world_x + width_tiles * 0.5) / width_tiles * width_px as f32).round() as i32;
    let y = ((height_tiles * 0.5 - world_z) / height_tiles * height_px as f32).round() as i32;
    (x, y)
}

fn ca44a_paint_tree_cluster(
    data: &mut [u8],
    width_px: u32,
    height_px: u32,
    world_x: f32,
    world_z: f32,
    index: i32,
    seed: u64,
) -> u32 {
    let (cx, cy) = ca44a_world_to_biome_pixel(world_x, world_z, width_px, height_px);
    let hash = ca44a_runtime_pixel_hash(seed, index, 31, 0, 0);
    let scale = 0.86 + (hash % 7) as f32 * 0.07;
    let mut painted = 0;
    painted += ca44a_paint_ellipse_rgba(
        data,
        width_px,
        height_px,
        cx + 1,
        cy + 5,
        (8.0 * scale) as i32,
        (4.0 * scale) as i32,
        [48, 78, 37],
        0.18,
    );
    for lobe in 0..4 {
        let offset_hash = ca44a_runtime_pixel_hash(seed ^ 0xBEE5, index, lobe, 0, 0);
        let ox = (offset_hash % 13) as i32 - 6;
        let oy = ((offset_hash / 17) % 11) as i32 - 5;
        let radius = (4 + (offset_hash % 4) as i32).max(4);
        painted += ca44a_paint_ellipse_rgba(
            data,
            width_px,
            height_px,
            cx + ox,
            cy + oy,
            (radius as f32 * scale) as i32,
            ((radius - 1) as f32 * scale) as i32,
            if lobe % 2 == 0 {
                [42, 111, 45]
            } else {
                [67, 139, 48]
            },
            0.56,
        );
    }
    if hash % 3 == 0 {
        painted += ca44a_paint_ellipse_rgba(
            data,
            width_px,
            height_px,
            cx + 3,
            cy - 2,
            2,
            2,
            [205, 71, 48],
            1.0,
        );
    }
    painted
}

fn ca44a_paint_rock_cluster(
    data: &mut [u8],
    width_px: u32,
    height_px: u32,
    world_x: f32,
    world_z: f32,
    index: i32,
    seed: u64,
) -> u32 {
    let (cx, cy) = ca44a_world_to_biome_pixel(world_x, world_z, width_px, height_px);
    let hash = ca44a_runtime_pixel_hash(seed ^ 0x510E_A11, index, 19, 0, 0);
    let mut painted = 0;
    painted += ca44a_paint_ellipse_rgba(
        data,
        width_px,
        height_px,
        cx + 2,
        cy + 5,
        16,
        7,
        [58, 66, 54],
        0.24,
    );
    for rock in 0..3 {
        let offset_hash = ca44a_runtime_pixel_hash(seed ^ 0xA70C, index, rock, 0, 0);
        let ox = (offset_hash % 13) as i32 - 6;
        let oy = ((offset_hash / 11) % 9) as i32 - 4;
        let rx = 6 + ((offset_hash / 3) % 6) as i32;
        let ry = 5 + ((offset_hash / 7) % 5) as i32;
        painted += ca44a_paint_ellipse_rgba(
            data,
            width_px,
            height_px,
            cx + ox,
            cy + oy,
            rx,
            ry,
            if (hash + rock as u32) % 2 == 0 {
                [139, 146, 132]
            } else {
                [101, 111, 99]
            },
            0.96,
        );
        painted += ca44a_paint_ellipse_rgba(
            data,
            width_px,
            height_px,
            cx + ox - 1,
            cy + oy - 1,
            (rx / 2).max(2),
            (ry / 2).max(2),
            [175, 182, 164],
            0.56,
        );
    }
    painted
}

fn ca44a_paint_crystal_cluster(
    data: &mut [u8],
    width_px: u32,
    height_px: u32,
    world_x: f32,
    world_z: f32,
    index: i32,
    seed: u64,
) -> u32 {
    let (cx, cy) = ca44a_world_to_biome_pixel(world_x, world_z, width_px, height_px);
    let hash = ca44a_runtime_pixel_hash(seed ^ 0xC425_7A1, index, 23, 0, 0);
    let mut painted = ca44a_paint_ellipse_rgba(
        data,
        width_px,
        height_px,
        cx + 2,
        cy + 7,
        16,
        5,
        [82, 34, 30],
        0.34,
    );
    for spike in 0..3 {
        let offset_hash = ca44a_runtime_pixel_hash(seed ^ 0xCAFE, index, spike, 0, 0);
        let ox = (offset_hash % 15) as i32 - 7;
        let height = 11 + ((offset_hash / 13) % 10) as i32;
        let half_width = 3 + ((offset_hash / 7) % 3) as i32;
        painted += ca44a_paint_diamond_rgba(
            data,
            width_px,
            height_px,
            cx + ox,
            cy + 1,
            half_width,
            height,
            if (hash + spike as u32) % 2 == 0 {
                [236, 42, 55]
            } else {
                [255, 95, 57]
            },
            1.0,
        );
        painted += ca44a_paint_diamond_rgba(
            data,
            width_px,
            height_px,
            cx + ox - 1,
            cy - 2,
            1,
            (height / 2).max(4),
            [255, 179, 105],
            0.72,
        );
    }
    painted
}

fn ca44a_paint_flower_sprout(
    data: &mut [u8],
    width_px: u32,
    height_px: u32,
    world_x: f32,
    world_z: f32,
    index: i32,
    seed: u64,
) -> u32 {
    let (cx, cy) = ca44a_world_to_biome_pixel(world_x, world_z, width_px, height_px);
    let hash = ca44a_runtime_pixel_hash(seed ^ 0xF10A, index, 29, 0, 0);
    let leaf_color = if hash % 2 == 0 {
        [98, 187, 70]
    } else {
        [73, 159, 66]
    };
    let mut painted = 0;
    painted += ca44a_paint_ellipse_rgba(
        data,
        width_px,
        height_px,
        cx - 2,
        cy + 1,
        3,
        2,
        leaf_color,
        0.70,
    );
    painted += ca44a_paint_ellipse_rgba(
        data,
        width_px,
        height_px,
        cx + 2,
        cy,
        3,
        2,
        leaf_color,
        0.70,
    );
    if hash % 5 == 0 {
        painted += ca44a_paint_ellipse_rgba(
            data,
            width_px,
            height_px,
            cx,
            cy - 3,
            2,
            2,
            [237, 123, 134],
            0.86,
        );
    }
    painted
}

fn ca44a_paint_line_rgba(
    data: &mut [u8],
    width_px: u32,
    height_px: u32,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    width: i32,
    color: [u8; 3],
    alpha: f32,
) -> u32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let steps = dx.abs().max(dy.abs()).max(1);
    let mut painted = 0;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = (x1 as f32 + dx as f32 * t).round() as i32;
        let y = (y1 as f32 + dy as f32 * t).round() as i32;
        painted += ca44a_paint_ellipse_rgba(
            data,
            width_px,
            height_px,
            x,
            y,
            width.max(1),
            (width / 2).max(1),
            color,
            alpha,
        );
    }
    painted
}

fn ca44a_paint_ellipse_rgba(
    data: &mut [u8],
    width_px: u32,
    height_px: u32,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: [u8; 3],
    alpha: f32,
) -> u32 {
    let rx = rx.max(1);
    let ry = ry.max(1);
    let mut painted = 0;
    for y in (cy - ry).max(0)..=(cy + ry).min(height_px as i32 - 1) {
        for x in (cx - rx).max(0)..=(cx + rx).min(width_px as i32 - 1) {
            let dx = (x - cx) as f32 / rx as f32;
            let dy = (y - cy) as f32 / ry as f32;
            let distance = dx * dx + dy * dy;
            if distance <= 1.0 {
                let edge = (1.0 - distance).clamp(0.0, 1.0);
                ca44a_alpha_blend_pixel(
                    data,
                    width_px,
                    x as u32,
                    y as u32,
                    color,
                    (alpha * (0.38 + edge * 0.62)).clamp(0.0, 1.0),
                );
                painted += 1;
            }
        }
    }
    painted
}

fn ca44a_paint_diamond_rgba(
    data: &mut [u8],
    width_px: u32,
    height_px: u32,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: [u8; 3],
    alpha: f32,
) -> u32 {
    let rx = rx.max(1);
    let ry = ry.max(1);
    let mut painted = 0;
    for y in (cy - ry).max(0)..=(cy + ry).min(height_px as i32 - 1) {
        for x in (cx - rx).max(0)..=(cx + rx).min(width_px as i32 - 1) {
            let dx = (x - cx).abs() as f32 / rx as f32;
            let dy = (y - cy).abs() as f32 / ry as f32;
            let distance = dx + dy;
            if distance <= 1.0 {
                ca44a_alpha_blend_pixel(
                    data,
                    width_px,
                    x as u32,
                    y as u32,
                    color,
                    (alpha * (1.0 - distance * 0.25)).clamp(0.0, 1.0),
                );
                painted += 1;
            }
        }
    }
    painted
}

fn ca44a_alpha_blend_pixel(
    data: &mut [u8],
    width_px: u32,
    x: u32,
    y: u32,
    color: [u8; 3],
    alpha: f32,
) {
    let index = ((y * width_px + x) * 4) as usize;
    if index + 2 >= data.len() {
        return;
    }
    let inverse = 1.0 - alpha.clamp(0.0, 1.0);
    for channel in 0..3 {
        data[index + channel] = (data[index + channel] as f32 * inverse
            + color[channel] as f32 * alpha)
            .round()
            .clamp(0.0, 255.0) as u8;
    }
}

fn ca44a_runtime_continuous_material_base_rgb(
    soil_weight: f32,
    resource_weight: f32,
    hazard_weight: f32,
    stone_weight: f32,
    water_weight: f32,
    sand_weight: f32,
) -> [u8; 3] {
    let mut rgb = ca44a_runtime_material_base_rgb("safe-grass").map(f32::from);
    if soil_weight > 0.05 {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb("neutral-soil").map(f32::from),
            (soil_weight * 0.80).clamp(0.0, 0.80),
        );
    }
    if resource_weight > 0.05 {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb("resource-grove").map(f32::from),
            (resource_weight * 0.82).clamp(0.0, 0.82),
        );
    }
    if stone_weight > 0.05 {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb("stone-dressing").map(f32::from),
            (stone_weight * 0.78).clamp(0.0, 0.78),
        );
    }
    if sand_weight > 0.05 {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb("sand").map(f32::from),
            (sand_weight * 0.78).clamp(0.0, 0.78),
        );
    }
    if water_weight > 0.05 {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb("water").map(f32::from),
            (water_weight * 0.86).clamp(0.0, 0.86),
        );
    }
    if hazard_weight > 0.05 {
        rgb = ca44a_blend_rgb_f32(
            rgb,
            ca44a_runtime_material_base_rgb("hazard-pressure").map(f32::from),
            (hazard_weight * 0.72).clamp(0.0, 0.72),
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
        "neutral-soil" => [143, 96, 55],
        "resource-grove" => [55, 136, 52],
        "hazard-pressure" => [188, 66, 43],
        "stone-dressing" => [97, 105, 100],
        "water" => [35, 123, 150],
        "sand" => [212, 179, 99],
        _ => [112, 158, 64],
    }
}

fn ca44a_runtime_soil_weight(world_x: f32, world_z: f32) -> f32 {
    let texture_gate = ca44a_macro_biome_gate(world_x, world_z, 0.21, 0.27);
    ((ca44a_smooth_blob(world_x, world_z, -48.0, -8.0, 20.0, 28.0) * 0.68
        + ca44a_smooth_blob(world_x, world_z, -17.0, -29.0, 20.0, 10.0) * 0.46
        + ca44a_smooth_blob(world_x, world_z, 12.0, -11.0, 16.0, 9.0) * 0.32
        + ca44a_smooth_blob(world_x, world_z, 53.0, 12.0, 13.0, 13.0) * 0.30)
        * texture_gate)
        .clamp(0.0, 1.0)
}

fn ca44a_runtime_resource_weight(world_x: f32, world_z: f32) -> f32 {
    let texture_gate = ca44a_macro_biome_gate(world_x, world_z, 0.17, 0.23);
    ((ca44a_smooth_blob(world_x, world_z, -20.0, -7.0, 23.0, 16.0) * 0.80
        + ca44a_smooth_blob(world_x, world_z, 4.0, 14.0, 24.0, 15.0) * 0.70
        + ca44a_smooth_blob(world_x, world_z, 0.0, -16.0, 24.0, 12.0) * 0.50
        + ca44a_smooth_blob(world_x, world_z, 34.0, -18.0, 18.0, 11.0) * 0.58
        + ca44a_smooth_blob(world_x, world_z, -42.0, 22.0, 15.0, 9.0) * 0.38)
        * texture_gate)
        .clamp(0.0, 1.0)
}

fn ca44a_runtime_hazard_weight(world_x: f32, world_z: f32) -> f32 {
    let texture_gate = ca44a_macro_biome_gate(world_x, world_z, 0.25, 0.19);
    ((ca44a_smooth_blob(world_x, world_z, 45.0, -5.0, 27.0, 20.0) * 0.88
        + ca44a_smooth_blob(world_x, world_z, 20.0, 6.0, 22.0, 14.0) * 0.62
        + ca44a_smooth_blob(world_x, world_z, 54.0, -28.0, 17.0, 11.0) * 0.50
        + ca44a_smooth_blob(world_x, world_z, -48.0, -23.0, 14.0, 10.0) * 0.50
        + ca44a_smooth_blob(world_x, world_z, 20.0, 23.0, 10.0, 7.0) * 0.28)
        * texture_gate)
        .clamp(0.0, 1.0)
}

fn ca44a_runtime_stone_weight(world_x: f32, world_z: f32) -> f32 {
    let texture_gate = ca44a_macro_biome_gate(world_x, world_z, 0.14, 0.22);
    ((ca44a_smooth_blob(world_x, world_z, -28.0, 22.0, 32.0, 18.0) * 0.84
        + ca44a_smooth_blob(world_x, world_z, 12.0, 26.0, 26.0, 13.0) * 0.74
        + ca44a_smooth_blob(world_x, world_z, -8.0, 21.0, 24.0, 11.0) * 0.54
        + ca44a_smooth_blob(world_x, world_z, 46.0, 24.0, 20.0, 10.0) * 0.52
        + ca44a_smooth_blob(world_x, world_z, 52.0, -29.0, 13.0, 8.0) * 0.34)
        * texture_gate)
        .clamp(0.0, 1.0)
}

fn ca44a_runtime_water_weight(world_x: f32, world_z: f32) -> f32 {
    let channel = world_z + world_x / 5.0 + 18.0;
    ((1.0 - channel.abs() / 4.0).clamp(0.0, 1.0)
        + ca44a_smooth_blob(world_x, world_z, -22.0, 2.0, 21.0, 9.0) * 0.76
        + ca44a_smooth_blob(world_x, world_z, -34.0, -21.0, 22.0, 10.0) * 0.98
        + ca44a_smooth_blob(world_x, world_z, -18.0, -34.0, 28.0, 9.0) * 0.46)
        .clamp(0.0, 1.0)
}

fn ca44a_runtime_sand_weight(world_x: f32, world_z: f32) -> f32 {
    let channel = world_z + world_x / 5.0 + 18.0;
    ((1.0 - channel.abs() / 7.0).clamp(0.0, 1.0) * 0.72
        + ca44a_smooth_blob(world_x, world_z, -21.0, 1.0, 25.0, 12.0) * 0.40
        + ca44a_smooth_blob(world_x, world_z, -34.0, -18.0, 23.0, 11.0) * 0.55)
        .clamp(0.0, 1.0)
}

fn ca44a_macro_biome_gate(world_x: f32, world_z: f32, x_freq: f32, z_freq: f32) -> f32 {
    let wave_a = (world_x * x_freq).sin() * (world_z * z_freq).cos();
    let wave_b = ((world_x + world_z) * x_freq * 0.47).sin();
    (0.66 + (wave_a + wave_b) * 0.12).clamp(0.48, 0.88)
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
    let t = (1.0 - distance).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
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

fn ca44a_blend_rgb_u8(base: [u8; 3], overlay: [u8; 3], weight: f32) -> [u8; 3] {
    let blended = ca44a_blend_rgb_f32(
        base.map(f32::from),
        overlay.map(f32::from),
        weight.clamp(0.0, 1.0),
    );
    [
        blended[0].round().clamp(0.0, 255.0) as u8,
        blended[1].round().clamp(0.0, 255.0) as u8,
        blended[2].round().clamp(0.0, 255.0) as u8,
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
            chunk_tile_size: config.chunk_tile_size,
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
    chunk_tile_size: i32,
) -> Vec<(i32, i32, Option<WorldEntityId>)> {
    let mut centers = Vec::new();
    let mut seen_chunks = BTreeSet::new();
    for chunk in &activation.active_chunks {
        if seen_chunks.insert((chunk.coord.x, chunk.coord.z)) {
            centers.push((
                chunk.coord.x * chunk_tile_size + chunk_tile_size / 2,
                chunk.coord.z * chunk_tile_size + chunk_tile_size / 2,
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
    render_player_surface: bool,
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
            if view_mode == GraphicalPlaygroundViewMode::Player && !render_player_surface {
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
    render_player_surface: bool,
    content: &ProceduralWorldContentReport,
) {
    for candidate in &content.candidates {
        if !field
            .materialized_content_stable_ids
            .insert(candidate.stable_id.raw())
        {
            continue;
        }
        ca44a_spawn_procedural_content_candidate_app(
            app,
            alpha_art,
            view_mode,
            render_player_surface,
            candidate,
        );
    }
}

fn ca44a_spawn_procedural_content_candidate_app(
    app: &mut App,
    alpha_art: Option<&GraphicalAlphaArtHandles>,
    view_mode: GraphicalPlaygroundViewMode,
    render_player_surface: bool,
    candidate: &ProceduralWorldContentCandidate,
) {
    let position = Vec3::new(
        candidate.position.x * GRAPHICAL_WORLD_SCALE,
        candidate.position.z * GRAPHICAL_WORLD_SCALE,
        ca44a_procedural_content_z(candidate.kind),
    );
    if view_mode == GraphicalPlaygroundViewMode::Player && !render_player_surface {
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life true 2.5D procedural content ledger stable:{}",
                candidate.stable_id.raw()
            )),
            GraphicalProductionArtLayer {
                role: "true-25d-procedural-content-ledger",
                display_only: true,
            },
            ca44a_procedural_content_marker(candidate),
        ));
        return;
    }
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
    let _ = hash;
    0.0
}

fn ca44a_terrain_jitter_y(hash: i32) -> f32 {
    let _ = hash;
    0.0
}

fn ca44a_terrain_tile_width(hash: i32) -> f32 {
    let _ = hash;
    CA37_TERRAIN_TILE_PIXEL_SIZE * 1.02
}

fn ca44a_terrain_tile_height(hash: i32) -> f32 {
    let _ = hash;
    CA37_TERRAIN_TILE_PIXEL_SIZE * 1.02
}

fn ca44a_terrain_rotation_degrees(hash: i32) -> f32 {
    let _ = hash;
    0.0
}

fn ca44a_alpha_terrain_opacity(material_id: &str) -> f32 {
    let _ = material_id;
    0.0
}

fn ca44a_sync_graphical_markers_from_live_world(
    snapshots: &[alife_world::WorldObject],
    markers: &mut bevy::prelude::Query<(
        &GraphicalPlaygroundMarker,
        &mut Transform,
        Option<&GraphicalTrue25dAsset>,
    )>,
    badges: &mut bevy::prelude::Query<
        (&GraphicalObjectBadge, &mut Transform),
        Without<GraphicalPlaygroundMarker>,
    >,
    art_layers: &mut bevy::prelude::Query<
        (&GraphicalAlphaArtBackedSprite, &mut Transform),
        (
            Without<GraphicalPlaygroundMarker>,
            Without<GraphicalObjectBadge>,
        ),
    >,
    selection: Option<&SelectionResource>,
    inspector: Option<&mut CreatureInspectorResource>,
    camera: Option<&mut CameraNavigationResource>,
) {
    let by_id = snapshots
        .iter()
        .map(|object| (object.id.raw(), object))
        .collect::<BTreeMap<_, _>>();

    for (marker, mut transform, true_25d) in markers.iter_mut() {
        if let Some(object) = by_id.get(&marker.stable_id.raw()) {
            let next = if true_25d.is_some() {
                Vec3::new(
                    object.position.x * TRUE_25D_SIM_TO_VIEW_SCALE,
                    transform.translation.y,
                    object.position.z * TRUE_25D_SIM_TO_VIEW_SCALE,
                )
            } else {
                let mut next = ca44a_world_object_graphical_position(object);
                next.z = transform.translation.z;
                next
            };
            transform.translation = next;
        }
    }

    for (badge, mut transform) in badges.iter_mut() {
        if let Some(object) = by_id.get(&badge.stable_id.raw()) {
            transform.translation =
                ca44a_world_object_graphical_position(object) + graphical_badge_offset(badge.kind);
        }
    }

    for (sprite, mut transform) in art_layers.iter_mut() {
        let Some(stable_id) = sprite.stable_id else {
            continue;
        };
        let Some(object) = by_id.get(&stable_id.raw()) else {
            continue;
        };
        if sprite.role == "entity-shadow" {
            transform.translation =
                ca44a_world_object_graphical_position(object) + Vec3::new(0.0, -2.0, -0.08);
        }
    }

    let selected = selection.map(|selection| selection.stable_id);
    if let (Some(selected), Some(inspector)) = (selected, inspector) {
        if let Some(object) = by_id.get(&selected.raw()) {
            inspector.snapshot.selection.position = object.position;
            inspector.snapshot.visual.position = object.position;
        }
    }

    if let Some(camera) = camera {
        if let Some(target) = camera.state.follow_target {
            if let Some(object) = by_id.get(&target.raw()) {
                if let Ok(next) = camera.state.focus_on(object.position) {
                    camera.state = next;
                }
            }
        }
    }
}

fn ca44a_world_object_graphical_position(object: &alife_world::WorldObject) -> Vec3 {
    Vec3::new(
        object.position.x * GRAPHICAL_WORLD_SCALE,
        object.position.z * GRAPHICAL_WORLD_SCALE,
        0.0,
    )
}

fn update_graphical_procedural_terrain_field(
    mut commands: Commands,
    view_mode: Res<GraphicalViewModeResource>,
    live_loop: Option<NonSend<GraphicalRuntimeLoopResource>>,
    mut camera: Option<ResMut<CameraNavigationResource>>,
    selection: Option<Res<SelectionResource>>,
    mut inspector: Option<ResMut<CreatureInspectorResource>>,
    world_art: Option<Res<GraphicalWorldArtStyleResource>>,
    true_25d: Option<Res<GraphicalTrue25dPresentationResource>>,
    alpha_art: Option<Res<GraphicalAlphaArtHandles>>,
    field: Option<ResMut<GraphicalProceduralTerrainFieldResource>>,
    mut markers: bevy::prelude::Query<(
        &GraphicalPlaygroundMarker,
        &mut Transform,
        Option<&GraphicalTrue25dAsset>,
    )>,
    mut badges: bevy::prelude::Query<
        (&GraphicalObjectBadge, &mut Transform),
        Without<GraphicalPlaygroundMarker>,
    >,
    mut art_layers: bevy::prelude::Query<
        (&GraphicalAlphaArtBackedSprite, &mut Transform),
        (
            Without<GraphicalPlaygroundMarker>,
            Without<GraphicalObjectBadge>,
        ),
    >,
) {
    if view_mode.mode != GraphicalPlaygroundViewMode::Player {
        return;
    }
    let Some(mut field) = field else {
        return;
    };
    let true_25d_player_view = true_25d.is_some();

    if let Some(live_loop) = live_loop.as_ref() {
        let snapshots = live_loop.live.world_object_snapshots();
        ca44a_sync_graphical_markers_from_live_world(
            &snapshots,
            &mut markers,
            &mut badges,
            &mut art_layers,
            selection.as_deref(),
            inspector.as_deref_mut(),
            camera.as_deref_mut(),
        );
    }

    let mut anchors = Vec::new();
    for (marker, transform, true_25d) in &mut markers {
        if marker.kind == WorldObjectKind::Agent {
            let world_z = if true_25d.is_some() {
                transform.translation.z / TRUE_25D_SIM_TO_VIEW_SCALE
            } else {
                transform.translation.y / GRAPHICAL_WORLD_SCALE
            };
            let position = Vec3f::new(
                transform.translation.x
                    / if true_25d.is_some() {
                        TRUE_25D_SIM_TO_VIEW_SCALE
                    } else {
                        GRAPHICAL_WORLD_SCALE
                    },
                0.0,
                world_z,
            );
            if let Ok(anchor) = CreatureWorldAnchor::new(marker.stable_id, position) {
                anchors.push(anchor);
            }
        }
    }
    let config = world_art
        .as_ref()
        .map(|world_art| ca44a_procedural_world_config(&world_art.summary))
        .unwrap_or_else(|| ProceduralWorldConfig::with_seed(field.seed));
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
    if true_25d_player_view {
        true_25d_materialize_terrain_chunk_ledger(field.as_mut(), &activation);
        if let Some(content) = &content {
            ca44a_spawn_procedural_world_content_ledger_commands(
                &mut commands,
                field.as_mut(),
                content,
            );
        }
        return;
    }
    let Some(world_art) = world_art else {
        return;
    };
    let centers = ca44a_initial_procedural_terrain_centers(&activation, field.chunk_tile_size);
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

fn ca44a_spawn_procedural_world_content_ledger_commands(
    commands: &mut Commands,
    field: &mut GraphicalProceduralTerrainFieldResource,
    content: &ProceduralWorldContentReport,
) {
    for candidate in &content.candidates {
        if !field
            .materialized_content_stable_ids
            .insert(candidate.stable_id.raw())
        {
            continue;
        }
        commands.spawn((
            Name::new(format!(
                "A-Life true 2.5D procedural content ledger stable:{}",
                candidate.stable_id.raw()
            )),
            GraphicalProductionArtLayer {
                role: "true-25d-procedural-content-ledger",
                display_only: true,
            },
            ca44a_procedural_content_marker(candidate),
        ));
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
        "water" => (handles.terrain_water.clone(), "terrain-water"),
        "sand" => (handles.terrain_sand.clone(), "terrain-sand"),
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
        "water" => (handles.prop_leaf_patch.clone(), "prop-dressing"),
        "sand" => (handles.prop_pebble_cluster.clone(), "prop-dressing"),
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
        ProceduralWorldContentKind::Food => Vec2::splat(13.0),
        ProceduralWorldContentKind::Hazard => Vec2::splat(18.0),
        ProceduralWorldContentKind::Obstacle => Vec2::splat(20.0),
        ProceduralWorldContentKind::DressingProp => Vec2::splat(9.0),
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
        "water" => Color::srgba(
            0.22 + shade,
            0.54 + shade,
            0.70,
            ca37_terrain_tile_alpha(material_id),
        ),
        "sand" => Color::srgba(
            0.74 + shade,
            0.62 + shade,
            0.35,
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
        "water" => 0.12,
        "sand" => 0.11,
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
        WorldObjectKind::Agent => Vec2::new(24.0, 19.0),
        WorldObjectKind::Food => Vec2::splat(15.0),
        WorldObjectKind::Hazard => Vec2::splat(19.0),
        WorldObjectKind::Obstacle => Vec2::splat(21.0),
        WorldObjectKind::Token => Vec2::new(13.0, 10.0),
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
                    custom_size: Some(Vec2::new(52.0, 41.0)),
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
                    color: Color::srgba(1.0, 1.0, 1.0, 0.28),
                    custom_size: Some(Vec2::new(60.0, 47.0)),
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
    mut commands: Commands,
    primary_windows: bevy::prelude::Query<Entity, With<PrimaryWindow>>,
    mut exits: MessageWriter<AppExit>,
) {
    if timer.started.elapsed() >= timer.duration {
        for entity in primary_windows.iter() {
            commands.entity(entity).despawn();
        }
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
            live_loop.gpu = GraphicalGpuRuntimeController::new(&GraphicalPlaygroundLaunchConfig {
                app_launch: live_loop.launch.clone(),
                mode: GraphicalPlaygroundMode::Interactive,
                brain_policy: live_loop.launch.brain_policy,
                gpu_mode: live_loop.gpu.mode(),
                view_mode: GraphicalPlaygroundViewMode::Player,
                window_title: crate::S01_GRAPHICAL_WINDOW_TITLE.to_string(),
                require_gpu: live_loop.launch.brain_policy.requires_gpu(),
            })?;
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
    live_loop.gpu = GraphicalGpuRuntimeController::new(&GraphicalPlaygroundLaunchConfig {
        app_launch: live_loop.launch.clone(),
        mode: GraphicalPlaygroundMode::Interactive,
        brain_policy: live_loop.launch.brain_policy,
        gpu_mode: live_loop.gpu.mode(),
        view_mode: GraphicalPlaygroundViewMode::Player,
        window_title: crate::S01_GRAPHICAL_WINDOW_TITLE.to_string(),
        require_gpu: live_loop.launch.brain_policy.requires_gpu(),
    })?;
    runtime.panel.reset_to_alpha_fixture(&live_loop.live);
    runtime.smoke_ticks_done = 0;
    gpu_telemetry.telemetry = live_loop.gpu.telemetry().clone();
    runtime.panel.validate()?;
    Ok(())
}

fn update_graphical_camera_transform(
    camera: Res<CameraNavigationResource>,
    mut cameras: bevy::prelude::Query<(&mut Transform, &mut Projection), With<GraphicalMainCamera>>,
) {
    for (mut transform, mut projection) in &mut cameras {
        transform.translation.x = camera.state.focus.x * GRAPHICAL_WORLD_SCALE;
        transform.translation.y = camera.state.focus.z * GRAPHICAL_WORLD_SCALE;
        transform.rotation =
            bevy::prelude::Quat::from_rotation_z(camera.state.yaw_degrees.to_radians());
        transform.scale = Vec3::ONE;
        if let Projection::Orthographic(ref mut orthographic) = *projection {
            orthographic.scale = (1.0 / camera.state.zoom).clamp(0.36, 3.2);
        }
    }
}

fn update_graphical_selection_ring(
    inspector: Res<CreatureInspectorResource>,
    selection: Res<SelectionResource>,
    markers: bevy::prelude::Query<
        (&GraphicalPlaygroundMarker, &Transform),
        Without<GraphicalSelectionRing>,
    >,
    mut ring_query: bevy::prelude::Query<&mut Transform, With<GraphicalSelectionRing>>,
) {
    let marker_position = markers
        .iter()
        .find(|(marker, _)| marker.stable_id == selection.stable_id)
        .map(|(_, transform)| transform.translation);
    let ring_position = marker_position.unwrap_or_else(|| {
        let selected_position = inspector.snapshot.selection.position;
        Vec3::new(
            selected_position.x * GRAPHICAL_WORLD_SCALE,
            selected_position.z * GRAPHICAL_WORLD_SCALE,
            0.2,
        )
    });
    for mut ring in &mut ring_query {
        ring.translation = Vec3::new(ring_position.x, ring_position.y, ring.translation.z);
    }
}

fn update_true_25d_neurochemical_visual_feedback(
    view_mode: Res<GraphicalViewModeResource>,
    runtime: Res<GraphicalRuntimeControlsResource>,
    inspector: Res<CreatureInspectorResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    presentation: Option<Res<GraphicalTrue25dPresentationResource>>,
    mut feedback: Option<ResMut<GraphicalTrue25dNeurochemicalFeedbackResource>>,
    mut endocrine_feedback: Option<ResMut<GraphicalTrue25dEndocrineAssetFeedbackResource>>,
    mut creature_presentations: bevy::prelude::Query<
        (
            &mut GraphicalTrue25dCreatureEndocrinePresentation,
            &mut GraphicalTrue25dStateCue,
            &mut Transform,
            &GraphicalTrue25dAsset,
            Option<&GraphicalAlphaArtBackedSprite>,
        ),
        (
            Without<GraphicalTrue25dNeurochemicalCue>,
            Without<GraphicalTrue25dEndocrineParticleLane>,
        ),
    >,
    mut cues: bevy::prelude::Query<
        (
            &mut GraphicalTrue25dNeurochemicalCue,
            &mut Transform,
            &mut Visibility,
        ),
        Without<GraphicalTrue25dEndocrineParticleLane>,
    >,
    mut particle_lanes: bevy::prelude::Query<
        (
            &mut GraphicalTrue25dEndocrineParticleLane,
            &mut Transform,
            &mut Visibility,
        ),
        (
            Without<GraphicalTrue25dNeurochemicalCue>,
            Without<GraphicalTrue25dCreatureEndocrinePresentation>,
        ),
    >,
) {
    if view_mode.mode != GraphicalPlaygroundViewMode::Player {
        return;
    }
    let visual = &inspector.snapshot.visual;
    let base = true_25d_creature_visual_position(visual);
    let (gltf_contract_validated, gltf_contract_assets) = presentation
        .as_ref()
        .map(|presentation| {
            (
                presentation
                    .asset_manifest
                    .endocrine_feedback_contract_validated,
                presentation.asset_manifest.endocrine_feedback_assets,
            )
        })
        .unwrap_or((false, 0));
    let endocrine = true_25d_endocrine_asset_feedback_from_snapshot(
        &inspector.snapshot,
        &gpu.telemetry,
        runtime.panel.mind_tick,
        true,
        gltf_contract_validated,
        gltf_contract_assets,
    );
    let mut selected_root_updated = false;
    for (mut presentation, mut state_cue, mut transform, asset, alpha_art) in
        &mut creature_presentations
    {
        if asset.role != "creature-idle" {
            continue;
        }
        let base_rotation = if alpha_art.is_some() {
            true_25d_alpha_billboard_rotation()
        } else {
            Quat::IDENTITY
        };
        if presentation.stable_id != visual.stable_id || asset.stable_id != Some(visual.stable_id) {
            presentation.asset_scale_multiplier = 1.0;
            presentation.posture_roll_degrees = 0.0;
            presentation.posture_lift = 0.0;
            presentation.animation_speed_multiplier = 1.0;
            presentation.animation_phase_index = 0;
            presentation.adrenaline_proxy = 0.0;
            presentation.cortisol_desaturation = 0.0;
            presentation.hunger_satisfaction_biolume = 0.0;
            presentation.learning_biolume = 0.0;
            presentation.particle_trail_count = 0;
            presentation.biolume_particle_array_initialized = false;
            presentation.creature_root_transform_applied = false;
            presentation.material_shell_applied = false;
            state_cue.pain_pose = false;
            state_cue.stress_desaturated = false;
            state_cue.learning_biolume = false;
            transform.rotation = base_rotation;
            transform.scale = true_25d_normalized_scale(presentation.base_scale);
            continue;
        }
        presentation.asset_scale_multiplier = endocrine.asset_scale_multiplier;
        presentation.animation_speed_multiplier = endocrine.animation_speed_multiplier;
        presentation.animation_phase_index = endocrine.animation_phase_index;
        presentation.posture_roll_degrees = endocrine.posture_roll_degrees;
        presentation.posture_lift = endocrine.posture_lift;
        presentation.adrenaline_proxy = endocrine.adrenaline_proxy;
        presentation.cortisol_desaturation = endocrine.cortisol_desaturation;
        presentation.hunger_satisfaction_biolume = endocrine.hunger_satisfaction_biolume;
        presentation.learning_biolume = endocrine.learning_biolume;
        presentation.particle_trail_count = endocrine.particle_trail_count;
        presentation.biolume_particle_array_initialized =
            endocrine.biolume_particle_array_initialized;
        presentation.creature_root_transform_applied = true;
        presentation.material_shell_applied = endocrine.material_shell_applied;
        state_cue.pain_pose = endocrine.pain_posture_active;
        state_cue.stress_desaturated = endocrine.cortisol_desaturation >= 0.22;
        state_cue.learning_biolume =
            endocrine.learning_biolume > 0.0 || endocrine.hunger_satisfaction_biolume >= 0.20;
        transform.translation = base + Vec3::Y * endocrine.posture_lift;
        transform.rotation =
            base_rotation * Quat::from_rotation_z(endocrine.posture_roll_degrees.to_radians());
        transform.scale =
            true_25d_normalized_scale(presentation.base_scale * endocrine.asset_scale_multiplier);
        selected_root_updated = true;
    }
    let mut active_cue_count = 0usize;
    let mut cue_count = 0usize;
    for (mut cue, mut transform, mut visibility) in &mut cues {
        if cue.stable_id != visual.stable_id {
            *visibility = Visibility::Hidden;
            cue.active = false;
            continue;
        }
        let intensity = true_25d_neurochemical_intensity(cue.kind, visual, &gpu.telemetry);
        let active = true_25d_neurochemical_cue_active(cue.kind, intensity);
        cue.intensity = intensity;
        cue.active = active;
        cue_count += 1;
        if active {
            active_cue_count += 1;
        }
        transform.translation = base + true_25d_neurochemical_offset(cue.kind);
        transform.scale = true_25d_neurochemical_scale(cue.kind, intensity);
        *visibility = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    let particle_intensity = endocrine
        .hunger_satisfaction_biolume
        .max(endocrine.learning_biolume)
        .clamp(0.0, 1.0);
    let mut visible_particle_lanes = 0_u8;
    for (mut lane, mut transform, mut visibility) in &mut particle_lanes {
        if lane.stable_id != visual.stable_id {
            lane.active = false;
            *visibility = Visibility::Hidden;
            continue;
        }
        let active = lane.lane_index < endocrine.particle_trail_count;
        lane.intensity = particle_intensity;
        lane.active = active;
        lane.animation_phase_index = endocrine.animation_phase_index;
        lane.initialized_from_endocrine_tensor = true;
        transform.translation = base
            + true_25d_endocrine_particle_offset(
                lane.lane_index,
                particle_intensity,
                endocrine.animation_phase_index,
            );
        transform.scale = true_25d_endocrine_particle_scale(lane.lane_index, particle_intensity);
        *visibility = if active {
            visible_particle_lanes = visible_particle_lanes.saturating_add(1);
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Some(feedback) = feedback.as_deref_mut() {
        *feedback =
            true_25d_neurochemical_feedback_from_snapshot(&inspector.snapshot, &gpu.telemetry);
        feedback.cue_count = cue_count.max(feedback.cue_count);
        feedback.active_cue_count = active_cue_count;
    }
    if let Some(endocrine_feedback) = endocrine_feedback.as_deref_mut() {
        *endocrine_feedback = GraphicalTrue25dEndocrineAssetFeedbackResource {
            applied_to_creature_root: selected_root_updated,
            root_transform_posture: selected_root_updated && endocrine.root_transform_posture,
            biolume_particle_lanes_visible: visible_particle_lanes,
            biolume_particle_array_initialized: endocrine.biolume_particle_array_initialized
                && visible_particle_lanes > 0,
            emissive_particle_array_initialized: endocrine.emissive_particle_array_initialized
                && visible_particle_lanes > 0,
            ..endocrine
        };
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
    let agent_color = if !gpu.telemetry.authoritative {
        Color::srgb(0.78, 0.78, 0.72)
    } else if gpu.telemetry.learning_updates > 0 {
        Color::srgb(0.42, 1.0, 0.72)
    } else if gpu.telemetry.authoritative {
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
                    sprite.color = if !gpu.telemetry.authoritative {
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
            if !gpu.authoritative {
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
    } else if !gpu.authoritative {
        CreatureExpressionState::Tired
    } else if gpu.learning_updates > 0 {
        CreatureExpressionState::Energized
    } else {
        CreatureExpressionState::Neutral
    };
    crate::ca38_creature_pose_for_state(animation, expression)
}

fn ca38_graphical_creature_size(pose: crate::Ca38CreaturePose) -> Vec2 {
    Vec2::new(42.0 * pose.scale_x, 34.0 * pose.scale_y)
}

fn ca38_graphical_creature_scale(
    pose: crate::Ca38CreaturePose,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> f32 {
    let learning = if gpu.learning_updates > 0 { 1.045 } else { 1.0 };
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
        Ca08SensoryCueKind::Learning => gpu.learning_updates > 0,
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
        (
            &GraphicalPlaygroundMarker,
            &Transform,
            Option<&GraphicalTrue25dAsset>,
        ),
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

    for (marker, transform, true_25d) in &markers {
        let display_position = intent_feedback_display_position(transform, true_25d.is_some());
        if marker.kind == WorldObjectKind::Agent && marker.stable_id == selection.stable_id {
            creature_position = Some(display_position);
        }
        if Some(marker.stable_id) == target {
            target_position = Some(display_position);
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
            if length > 0.05 {
                transform.translation =
                    Vec3::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5, 0.32);
                transform.rotation = bevy::prelude::Quat::from_rotation_z(delta.y.atan2(delta.x));
                sprite.custom_size = Some(Vec2::new(length.max(1.25), 5.0));
                sprite.color = intent_line_color(action, runtime.panel.target_entity);
            }
        } else {
            sprite.color = Color::srgba(0.42, 1.0, 0.58, 0.0);
            sprite.custom_size = Some(Vec2::new(1.0, 5.0));
        }
    }
}

fn intent_feedback_display_position(transform: &Transform, true_25d: bool) -> Vec3 {
    if true_25d {
        Vec3::new(
            transform.translation.x,
            transform.translation.z,
            transform.translation.y,
        )
    } else {
        transform.translation
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
                gpu.telemetry.learning_updates
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
        gpu.telemetry
            .unavailable_reason
            .as_deref()
            .unwrap_or("none"),
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
        gpu.learning_updates,
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
    let availability = if let Some(reason) = gpu.unavailable_reason.as_deref() {
        format!("GPU unavailable: {reason}")
    } else {
        "GPU authority: available".to_string()
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
        availability,
        goal,
        action,
        target,
        pose.action_label,
        pose.pose_id,
        panel.last_patch_sealed,
        panel.sealed_patch_count,
        gpu.learning_updates,
        gpu.last_learning_delta,
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
    if let Some(reason) = gpu.unavailable_reason.as_deref() {
        format!("unavailable ({})", compact_overlay_line(reason, 18))
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
        panel.mind_tick, last_event, gpu.learning_updates
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
        gpu.learning_updates, gpu.last_learning_delta
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
        "GPU neural: {} | failure policy: stop learned actions | no bulk readback={}",
        if gpu.authoritative {
            "authoritative"
        } else {
            "unavailable"
        },
        gpu.no_active_bulk_readback
    )
}

pub fn feedback_cue_overlay_text(
    feedback: &crate::FeedbackPolishSummary,
    inspector: &CreatureInspectorResource,
) -> String {
    let snapshot = &inspector.snapshot;
    let evidence = crate::Ca39RuntimeCueEvidence {
        selected_backend: "GpuAuthoritative".to_string(),
        unavailable_reason: None,
        authoritative: true,
        sealed_patches: feedback.sealed_outcome_event_count,
        learning_updates: 0,
        no_active_bulk_readback: true,
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
        ca08_sensory_cue_panel_text(feedback, &GraphicalGpuRuntimeTelemetry::pending("N2048"),),
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
            active: gpu.learning_updates > 0,
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
            "GPU-authoritative creature loop visible; advanced systems optional={}. Unavailability stops learned actions.\n",
            "Record: window, controls, inspector, unavailable warning, confusing text."
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
