//! Feature-gated Bevy playground shell split during R13 remediation.

use std::sync::{Arc, Mutex};

use alife_bevy_adapter::{
    core_vec3_to_bevy, AffordanceTags, AlifeBevyAdapterPlugin, BevyEntityMap, CreatureBody,
    SensoryEmitter,
};
use alife_core::{ActionKind, AffordanceBits, WorldEntityId};
use alife_world::{TerrainZoneKind, WorldObjectKind};
use bevy::{
    app::AppExit,
    prelude::{
        default, App, BackgroundColor, ButtonInput, Camera, Camera2d, ClearColor, Color, Component,
        DefaultPlugins, Entity, GlobalTransform, KeyCode, MessageWriter, MinimalPlugins,
        MouseButton, Name, Node, NonSendMut, PluginGroup, PositionType, Res, ResMut, Resource,
        Sprite, Text, Text2d, TextColor, TextFont, Time, Timer, TimerMode, Transform, Update, Val,
        Vec2, Vec3, With, Without,
    },
    window::{ExitCondition, PresentMode, PrimaryWindow, Window, WindowPlugin, WindowTheme},
};

use crate::{
    ca18_cycle_selected_creature, ca18_graphical_population_summary, ca18_social_proximity_cues,
    ca19_graphical_ecology_summary, load_visible_world_from_p34_save,
    run_advanced_gameplay_ux_smoke, run_creature_inspector_smoke, run_creature_visual_smoke,
    run_headless_app_shell_smoke, run_live_brain_loop_smoke, AdvancedGameplayUxSummary,
    AppShellLaunchConfig, AppStartupSummary, Ca18GraphicalPopulationSummary,
    Ca19GraphicalEcologySummary, Ca19TerrainZoneVisual, CameraNavigationState,
    CreatureAnimationState, CreatureExpressionState, CreatureInspectorSnapshot,
    CreatureVisualSnapshot, EntitySelectionSnapshot, GameAppShellError, GameAppState,
    GraphicalGpuRuntimeController, GraphicalGpuRuntimeMode, GraphicalGpuRuntimeTelemetry,
    GraphicalPlaygroundLaunchConfig, GraphicalPlaygroundLaunchSummary, GraphicalPlaygroundMode,
    LiveBrainLoop, LiveBrainTickSummary, RuntimeControlCommand, RuntimeControlPanel,
    RuntimePlaybackState, VisibleMaterialKind, VisiblePlaceholderShape,
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

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalPopulationResource {
    pub summary: Ca18GraphicalPopulationSummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalEcologyResource {
    pub summary: Ca19GraphicalEcologySummary,
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
pub struct GraphicalTerrainZoneMarker {
    pub zone_id: alife_world::EcologyZoneId,
    pub kind: TerrainZoneKind,
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

const GRAPHICAL_WORLD_SCALE: f32 = 125.0;

#[derive(Debug, Resource)]
struct GraphicalPlaygroundSmokeTimer(Timer);

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

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: launch.window_title.clone(),
            name: Some("alife.graphical_playground".to_string()),
            resolution: (1180, 760).into(),
            present_mode: PresentMode::AutoVsync,
            window_theme: Some(WindowTheme::Dark),
            ..default()
        }),
        exit_condition: ExitCondition::OnPrimaryClosed,
        ..default()
    }))
    .add_plugins(AlifeBevyAdapterPlugin)
    .insert_resource(ClearColor(Color::srgb(0.045, 0.065, 0.055)))
    .insert_resource(GraphicalPlaygroundSceneResource {
        summary: summary.clone(),
    });
    spawn_graphical_playground_scene(&mut app, &presentation, &summary, ecology_summary.as_ref())?;
    let inspector = run_creature_inspector_smoke(&launch.app_launch)?;
    let feedback = crate::run_feedback_polish_smoke(&launch.app_launch)?;
    let save_load = crate::GraphicalSaveLoadMenuSession::from_launch(&launch.app_launch)?;
    let advanced = run_advanced_gameplay_ux_smoke()?;
    let local_entity =
        inspector_local_entity(&mut app, &presentation, inspector.selection.stable_id)?;
    let (controls, live_loop, gpu_telemetry) = graphical_runtime_resources(launch)?;
    app.insert_resource(controls)
        .insert_resource(gpu_telemetry)
        .insert_non_send_resource(live_loop)
        .insert_resource(GraphicalVisibleWorldPresentationResource {
            presentation: presentation.clone(),
        })
        .insert_resource(CameraNavigationResource {
            state: inspector.camera,
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
                update_graphical_feedback_overlay,
                update_graphical_population_overlay,
                update_graphical_ecology_overlay,
                update_graphical_boundary_footer_overlay,
                update_graphical_save_load_menu_overlay,
                update_graphical_advanced_gameplay_overlay,
            ),
        );
    if let Some(summary) = population_summary {
        app.insert_resource(GraphicalPopulationResource { summary });
    }
    if let Some(summary) = ecology_summary {
        app.insert_resource(GraphicalEcologyResource { summary });
    }

    if let GraphicalPlaygroundMode::Smoke { seconds } = launch.mode {
        app.insert_resource(GraphicalPlaygroundSmokeTimer(Timer::from_seconds(
            seconds as f32,
            TimerMode::Once,
        )))
        .add_systems(Update, close_after_graphical_smoke_timeout);
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
    spawn_graphical_playground_scene(&mut app, &presentation, &summary, None)?;
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
) -> Result<(), GameAppShellError> {
    app.world_mut().spawn((Camera2d, GraphicalMainCamera));
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
    if let Some(ecology) = ecology {
        spawn_ca19_terrain_zone_visuals(app, ecology);
    }

    for object in &presentation.objects {
        spawn_graphical_object(app, object)?;
    }
    spawn_graphical_intent_feedback(app);
    spawn_ca08_feedback_pulses(app, presentation);
    spawn_ca18_social_proximity_cues(app, presentation);

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
            "Status\nA-Life GPU Alpha Playground\nState: launch  Speed: 1x\nTick: pending  World: pending\nGPU: {} requested\nCreature: stable:1\nGoal: idle  Action: None\nTarget: none  Intent: pending\nPatch: sealed=false count=0\nLearning: H_shadow pulse\nFixture seed={}",
            summary.requested_gpu_mode.label(),
            summary.seed,
        )),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.95, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            max_width: Val::Px(390.0),
            padding: bevy::ui::UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.025, 0.82)),
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
        GraphicalEcologyOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA05 controls and legend panel"),
        Text::new(ca05_controls_bar_text()),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.94, 0.86)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(118.0),
            left: Val::Px(12.0),
            max_width: Val::Px(570.0),
            padding: bevy::ui::UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.025, 0.02, 0.84)),
        ReadabilityLegendOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA05 event feed panel"),
        Text::new("Event Feed\n- Waiting for first GPU-backed tick."),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.98, 0.94)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(118.0),
            right: Val::Px(12.0),
            max_width: Val::Px(460.0),
            padding: bevy::ui::UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.018, 0.03, 0.024, 0.86)),
        FeedbackCueOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life S03 read-only creature inspector overlay"),
        Text::new("Inspector loading..."),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(12.0),
            max_width: Val::Px(380.0),
            padding: bevy::ui::UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.025, 0.035, 0.86)),
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
        SaveLoadMenuOverlay,
    ));

    Ok(())
}

fn spawn_ca18_social_proximity_cues(app: &mut App, presentation: &VisibleWorldPresentation) {
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
        ));
    }
}

fn spawn_ca19_terrain_zone_visuals(app: &mut App, ecology: &Ca19GraphicalEcologySummary) {
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
        TerrainZoneKind::HazardField => Color::srgba(0.95, 0.12, 0.12, 0.22),
        TerrainZoneKind::Grove | TerrainZoneKind::Meadow => {
            Color::srgba(0.20, 0.72, 0.30, 0.20 + zone.resource_bias * 0.10)
        }
        TerrainZoneKind::Wetland => Color::srgba(0.18, 0.48, 0.84, 0.22),
        TerrainZoneKind::Rocky => Color::srgba(0.55, 0.52, 0.46, 0.22),
        TerrainZoneKind::Nest => Color::srgba(0.78, 0.62, 0.22, 0.22),
    }
}

fn spawn_graphical_intent_feedback(app: &mut App) {
    app.world_mut().spawn((
        Name::new("A-Life CA03 stable-ID intent line"),
        Sprite {
            color: Color::srgba(0.42, 1.0, 0.58, 0.0),
            custom_size: Some(Vec2::new(1.0, 5.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.35),
        GraphicalIntentLine,
    ));

    app.world_mut().spawn((
        Name::new("A-Life CA03 selected action badge"),
        Text2d::new("Action: IDLE"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.98, 0.96, 0.72)),
        Transform::from_xyz(0.0, 88.0, 1.15),
        GraphicalActionBadge,
    ));
}

fn spawn_ca08_feedback_pulses(app: &mut App, presentation: &VisibleWorldPresentation) {
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
        app.world_mut().spawn((
            Name::new(format!(
                "A-Life CA08 {} pulse stable:{}",
                pulse.kind.label(),
                target.stable_id.raw()
            )),
            Sprite {
                color: ca08_pulse_color(pulse.kind, false),
                custom_size: Some(ca08_pulse_size(pulse.kind)),
                ..default()
            },
            Transform::from_translation(graphical_position(target) + Vec3::new(0.0, 0.0, 0.28)),
            pulse,
        ));
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

fn ca08_pulse_size(kind: Ca08SensoryCueKind) -> Vec2 {
    match kind {
        Ca08SensoryCueKind::Reward => Vec2::new(74.0, 74.0),
        Ca08SensoryCueKind::Pain => Vec2::new(86.0, 86.0),
        Ca08SensoryCueKind::Sleep => Vec2::new(102.0, 64.0),
        Ca08SensoryCueKind::Learning => Vec2::new(118.0, 74.0),
    }
}

fn spawn_graphical_object(
    app: &mut App,
    object: &VisibleWorldObjectPresentation,
) -> Result<(), GameAppShellError> {
    let material = object.material;
    let marker_position = graphical_position(object);
    let entity = app
        .world_mut()
        .spawn((
            Name::new(format!(
                "A-Life {:?} stable:{} {}",
                object.kind,
                object.stable_id.raw(),
                object.label
            )),
            Sprite {
                color: rgba_to_color(material.rgba()),
                custom_size: Some(graphical_size(object)),
                ..default()
            },
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

    app.world_mut().spawn((
        Name::new(format!("A-Life label stable:{}", object.stable_id.raw())),
        Text2d::new(graphical_object_badge_text(object)),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(readability_label_color(object.kind)),
        Transform::from_translation(marker_position + graphical_badge_offset(object.kind)),
        GraphicalObjectBadge {
            stable_id: object.stable_id,
            kind: object.kind,
        },
    ));
    Ok(())
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
        app.world_mut().spawn((
            Name::new("A-Life S03 stable-ID selection ring"),
            Sprite {
                color: Color::srgba(1.0, 0.86, 0.25, 0.42),
                custom_size: Some(Vec2::new(104.0, 66.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, 0.5),
            GraphicalSelectionRing,
        ));
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
    time: Res<Time>,
    mut timer: ResMut<GraphicalPlaygroundSmokeTimer>,
    mut exits: MessageWriter<AppExit>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
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
    mut overlays: bevy::prelude::Query<&mut Text, With<RuntimeStatusOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = runtime
            .panel
            .structured_status_panel_text_with_backend(&gpu.telemetry.backend_line());
    }
}

fn update_graphical_inspector_overlay(
    runtime: Res<GraphicalRuntimeControlsResource>,
    camera: Res<CameraNavigationResource>,
    selection: Res<SelectionResource>,
    inspector: Res<CreatureInspectorResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<InspectorStatusOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = graphical_inspector_overlay_text(&runtime, &camera, &selection, &inspector, &gpu);
    }
}

fn update_graphical_gpu_visual_cues(
    runtime: Res<GraphicalRuntimeControlsResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    mut markers: bevy::prelude::Query<(&GraphicalPlaygroundMarker, &mut Sprite, &mut Transform)>,
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
    let learning_scale = if gpu.telemetry.h_shadow_applications > 0 {
        1.18
    } else {
        1.0
    };
    let action_scale = if runtime.panel.playback == RuntimePlaybackState::Running {
        1.08
    } else {
        1.0
    };
    let target = runtime.panel.target_entity.map(WorldEntityId);
    for (marker, mut sprite, mut transform) in &mut markers {
        match marker.kind {
            WorldObjectKind::Agent => {
                sprite.color = agent_color;
                transform.scale = Vec3::splat(learning_scale * action_scale);
            }
            WorldObjectKind::Food if target == Some(marker.stable_id) => {
                sprite.color = Color::srgb(1.0, 0.95, 0.28);
                transform.scale = Vec3::splat(1.14);
            }
            WorldObjectKind::Hazard => {
                sprite.color = Color::srgb(1.0, 0.16, 0.18);
                transform.scale = Vec3::splat(1.06);
            }
            _ => {
                transform.scale = Vec3::splat(1.0);
            }
        }
    }
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
    let alpha = if active { 0.54 } else { 0.20 };
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
        .unwrap_or("IDLE");
    for (mut text, mut color, mut transform) in &mut badges {
        text.0 = format!("Action: {action_label}");
        color.0 = action_badge_color(action_kind, runtime.panel.target_entity);
        if let Some(position) = creature_position {
            transform.translation = position + Vec3::new(0.0, 72.0, 1.15);
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
    mut overlays: bevy::prelude::Query<&mut Text, With<FeedbackCueOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = format!(
            "{}\n{}",
            runtime.panel.event_feed_panel_text(),
            ca08_sensory_cue_panel_text(&feedback.summary, &gpu.telemetry)
        );
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
    runtime: &GraphicalRuntimeControlsResource,
    camera: &CameraNavigationResource,
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
    let follow = camera
        .state
        .follow_target
        .map_or_else(|| "none".to_string(), |id| id.raw().to_string());
    let sleep = ca07_awake_sleep_status(snapshot);
    let bars = ca07_creature_state_bars(snapshot).join("\n");
    let learning = ca07_learning_summary(&gpu.telemetry);
    let tech = ca07_compact_technical_summary(&gpu.telemetry);
    let fallback = compact_overlay_line(
        gpu.telemetry.fallback_reason.as_deref().unwrap_or("none"),
        30,
    );
    format!(
        concat!(
            "Creature Inspector\n",
            "Stable ID: {} ({})\n",
            "Kind: {:?}  Org: {}\n",
            "State: {}  {}/{}\n",
            "{}\n",
            "{}\n",
            "{}\n",
            "Action: {}  Target: {}\n",
            "Patch: {}\n",
            "Learning: {}\n",
            "Cam: ({:.1},{:.1}) z={:.1} follow={}\n",
            "Read-only stable IDs\n",
            "{}\n",
            "Fallback: {}\n",
            "Tech: {}\n",
            "Claim: full_auth=false"
        ),
        selection.stable_id.raw(),
        selection.local_entity.map(|_| "mapped").unwrap_or("none"),
        snapshot.selection.kind,
        snapshot
            .selection
            .organism_id
            .map_or_else(|| "none".to_string(), |id| id.raw().to_string()),
        sleep,
        snapshot.visual.animation.label(),
        snapshot.visual.expression.label(),
        bars,
        runtime.panel.homeostasis.compact_line(),
        runtime.panel.homeostasis.modulation_line(),
        action,
        target,
        patch,
        learning,
        camera.state.focus.x,
        camera.state.focus.z,
        camera.state.zoom,
        follow,
        runtime.panel.motor_ring.panel_text(),
        fallback,
        tech,
    )
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
        "Visual Guide: [@] creature | [+] food | [!] hazard",
        "Other guide markers: [#] obstacle | [T] token",
        "GPU alpha fixture: creature+food+real hazard+obstacle + terrain zones. P34 remains guide-only.",
        "Creature colors: cyan GPU proposals | green learned H_shadow | gray CPU fallback",
        "Sensory pulses: reward=green pain=red sleep=blue learning=teal.",
        "Terrain zones: green resource bias | red hazard pressure.",
        "Audio stubs: soft-ping warning-pulse rest-chime learn-spark.",
        "All markers are presentation only. Stable IDs stay portable.",
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

pub fn ca05_controls_bar_text() -> &'static str {
    concat!(
        "Controls\n",
        "Left click select | Tab cycle creatures | Space run/pause | N step | R reset\n",
        "1/2/3 speed | WASD/arrows pan | +/- zoom | Q/E orbit\n",
        "F follow selected stable ID | M save/load | F5 save | F9 load | Esc quit\n",
        "Guide: [@] creature | [+] food | [!] hazard | [#] obstacle | blue social cue\n",
        "Visuals mirror model state. Stable IDs only."
    )
}

pub fn alpha_controls_help_text() -> &'static str {
    "Controls: Left click select | Tab cycle creatures | Space run/pause | N step | R reset | M save/load | F5 save | F9 load | +/- zoom | F follow | Esc quit"
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
    format!(
        concat!(
            "Play Feedback (display-only)\n",
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
        WorldObjectKind::Obstacle => "[#] obstacle",
        WorldObjectKind::Token => "[T] token",
    };
    let detail = match object.kind {
        WorldObjectKind::Agent => object
            .organism_id
            .map(|id| format!("organism={}", id.raw()))
            .unwrap_or_else(|| "organism=unknown".to_string()),
        WorldObjectKind::Food => format!("nutrition={:.2}", object.nutrition),
        WorldObjectKind::Hazard => format!("pain={:.2}", object.hazard_pain),
        WorldObjectKind::Obstacle => format!("radius={:.2}", object.radius),
        WorldObjectKind::Token => format!("token={:?}", object.token_id),
    };
    format!(
        "{} stable:{} {}\n{}",
        marker,
        object.stable_id.raw(),
        object.label,
        detail
    )
}

fn graphical_badge_offset(kind: WorldObjectKind) -> Vec3 {
    match kind {
        WorldObjectKind::Agent => Vec3::new(-118.0, 72.0, 1.0),
        WorldObjectKind::Food => Vec3::new(-70.0, 84.0, 1.0),
        WorldObjectKind::Hazard => Vec3::new(92.0, -70.0, 1.0),
        WorldObjectKind::Obstacle => Vec3::new(96.0, 36.0, 1.0),
        WorldObjectKind::Token => Vec3::new(92.0, 42.0, 1.0),
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
