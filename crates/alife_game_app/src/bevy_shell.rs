//! Feature-gated Bevy playground shell split during R13 remediation.

use std::sync::{Arc, Mutex};

use alife_bevy_adapter::{
    core_vec3_to_bevy, AffordanceTags, AlifeBevyAdapterPlugin, BevyEntityMap, CreatureBody,
    SensoryEmitter,
};
use alife_core::{ActionKind, AffordanceBits, WorldEntityId};
use alife_world::WorldObjectKind;
use bevy::{
    app::AppExit,
    prelude::{
        default, App, BackgroundColor, ButtonInput, Camera2d, ClearColor, Color, Component,
        DefaultPlugins, Entity, KeyCode, MessageWriter, MinimalPlugins, Name, Node, NonSendMut,
        PluginGroup, PositionType, Res, ResMut, Resource, Sprite, Text, Text2d, TextColor,
        TextFont, Time, Timer, TimerMode, Transform, Update, Val, Vec2, Vec3, With, Without,
    },
    window::{ExitCondition, PresentMode, Window, WindowPlugin, WindowTheme},
};

use crate::{
    load_visible_world_from_p34_save, run_advanced_gameplay_ux_smoke, run_creature_inspector_smoke,
    run_creature_visual_smoke, run_headless_app_shell_smoke, run_live_brain_loop_smoke,
    AdvancedGameplayUxSummary, AppShellLaunchConfig, AppStartupSummary, CameraNavigationState,
    CreatureAnimationState, CreatureExpressionState, CreatureInspectorSnapshot,
    CreatureVisualSnapshot, EntitySelectionSnapshot, GameAppShellError, GameAppState,
    GraphicalGpuRuntimeController, GraphicalGpuRuntimeTelemetry, GraphicalPlaygroundLaunchConfig,
    GraphicalPlaygroundLaunchSummary, GraphicalPlaygroundMode, LiveBrainLoop, LiveBrainTickSummary,
    RuntimeControlCommand, RuntimeControlPanel, RuntimePlaybackState, VisibleMaterialKind,
    VisiblePlaceholderShape, VisibleWorldObjectPresentation, VisibleWorldPresentation,
    S02_MAX_SMOKE_TICKS,
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
pub struct SaveLoadMenuOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct AdvancedGameplayOverlay;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalObjectBadge {
    pub stable_id: WorldEntityId,
    pub kind: WorldObjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalIntentLine;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct GraphicalActionBadge;

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalFeedbackCueResource {
    pub summary: crate::FeedbackPolishSummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalSaveLoadMenuResource {
    pub summary: crate::SaveLoadUxSmokeSummary,
}

#[derive(Debug, Clone, PartialEq, Resource)]
pub struct GraphicalAdvancedGameplayResource {
    pub summary: AdvancedGameplayUxSummary,
}

const GRAPHICAL_WORLD_SCALE: f32 = 125.0;

#[derive(Debug, Resource)]
struct GraphicalPlaygroundSmokeTimer(Timer);

#[derive(Debug, Resource)]
struct GraphicalRuntimeTickTimer(Timer);

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
    spawn_graphical_playground_scene(&mut app, &presentation, &summary)?;
    let inspector = run_creature_inspector_smoke(&launch.app_launch)?;
    let feedback = crate::run_feedback_polish_smoke(&launch.app_launch)?;
    let save_load = crate::run_save_load_ux_smoke(&launch.app_launch)?;
    let advanced = run_advanced_gameplay_ux_smoke()?;
    let local_entity =
        inspector_local_entity(&mut app, &presentation, inspector.selection.stable_id)?;
    let (controls, live_loop, gpu_telemetry) = graphical_runtime_resources(launch)?;
    app.insert_resource(controls)
        .insert_resource(gpu_telemetry)
        .insert_non_send_resource(live_loop)
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
        .insert_resource(GraphicalSaveLoadMenuResource {
            summary: save_load.clone(),
        })
        .insert_resource(GraphicalAdvancedGameplayResource {
            summary: advanced.clone(),
        })
        .insert_resource(GraphicalRuntimeTickTimer(Timer::from_seconds(
            0.35,
            TimerMode::Repeating,
        )))
        .add_systems(
            Update,
            (
                handle_graphical_runtime_input,
                handle_graphical_camera_selection_input,
                advance_graphical_runtime_loop,
                update_graphical_camera_transform,
                update_graphical_selection_ring,
                update_graphical_runtime_overlay,
                update_graphical_inspector_overlay,
                update_graphical_gpu_visual_cues,
                update_graphical_intent_feedback,
                update_graphical_feedback_overlay,
                update_graphical_save_load_menu_overlay,
                update_graphical_advanced_gameplay_overlay,
            ),
        );

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
    spawn_graphical_playground_scene(&mut app, &presentation, &summary)?;
    app.insert_resource(GraphicalRuntimeControlsResource {
        panel,
        smoke_target_ticks: None,
        smoke_ticks_done: 0,
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

    for object in &presentation.objects {
        spawn_graphical_object(app, object)?;
    }
    spawn_graphical_intent_feedback(app);

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
            "A-Life GPU Alpha Playground\nFixture: GPU alpha stable-ID world  seed={}\nMode: {}  GPU={}\nRequire GPU: {}  timeout={:?}\n{}\nStable IDs: creature=1 food=2 hazard=3 obstacle=4\nControls: Space run/pause | N step | R reset | 1/2/3 speed | F follow | Esc quit\nMarkers: green/cyan creature, bright food, red hazard, stone obstacle",
            summary.seed,
            summary.mode_label,
            summary.requested_gpu_mode.label(),
            summary.require_gpu,
            summary.smoke_seconds,
            crate::s08_runtime_overlay_status_line(),
        )),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.88, 0.95, 0.88)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            max_width: Val::Px(540.0),
            padding: bevy::ui::UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.03, 0.025, 0.82)),
        RuntimeStatusOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life S04 readability legend overlay"),
        Text::new(readability_legend_overlay_text()),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.95, 0.94, 0.86)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            left: Val::Px(12.0),
            max_width: Val::Px(620.0),
            padding: bevy::ui::UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.025, 0.025, 0.02, 0.84)),
        ReadabilityLegendOverlay,
    ));

    app.world_mut().spawn((
        Name::new("A-Life S04 feedback cue overlay"),
        Text::new("Feedback cues loading..."),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::srgb(0.94, 0.98, 0.94)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            right: Val::Px(12.0),
            max_width: Val::Px(470.0),
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
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.92, 0.96, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(12.0),
            max_width: Val::Px(420.0),
            padding: bevy::ui::UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.025, 0.035, 0.86)),
        InspectorStatusOverlay,
    ));

    Ok(())
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

fn advance_graphical_runtime_loop(
    time: Res<Time>,
    mut timer: ResMut<GraphicalRuntimeTickTimer>,
    mut runtime: ResMut<GraphicalRuntimeControlsResource>,
    mut live_loop: NonSendMut<GraphicalRuntimeLoopResource>,
    mut gpu_telemetry: ResMut<GraphicalGpuTelemetryResource>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    if let Some(target) = runtime.smoke_target_ticks {
        if runtime.smoke_ticks_done < target {
            match apply_graphical_runtime_command(
                &mut runtime.panel,
                &mut live_loop,
                RuntimeControlCommand::StepOnce,
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

    if runtime.panel.playback == RuntimePlaybackState::Running {
        let ticks = runtime.panel.run_speed_ticks;
        match apply_graphical_runtime_command(
            &mut runtime.panel,
            &mut live_loop,
            RuntimeControlCommand::RunForTicks(ticks),
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
            let summary = live_loop.gpu.tick(&mut live_loop.live)?;
            panel.record_tick(&summary);
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
                let summary = live_loop.gpu.tick(&mut live_loop.live)?;
                panel.record_tick(&summary);
                summaries.push(summary);
            }
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
        text.0 = runtime.panel.status_overlay_text_with_backend(
            &gpu.telemetry.backend_line(),
            &gpu.telemetry.overlay_lines(),
        );
    }
}

fn update_graphical_inspector_overlay(
    camera: Res<CameraNavigationResource>,
    selection: Res<SelectionResource>,
    inspector: Res<CreatureInspectorResource>,
    gpu: Res<GraphicalGpuTelemetryResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<InspectorStatusOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = graphical_inspector_overlay_text(&camera, &selection, &inspector, &gpu);
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

fn update_graphical_intent_feedback(
    runtime: Res<GraphicalRuntimeControlsResource>,
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
        if marker.kind == WorldObjectKind::Agent && marker.stable_id == WorldEntityId(1) {
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
    feedback: Res<GraphicalFeedbackCueResource>,
    inspector: Res<CreatureInspectorResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<FeedbackCueOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = feedback_cue_overlay_text(&feedback.summary, &inspector);
    }
}

fn update_graphical_save_load_menu_overlay(
    menu: Res<GraphicalSaveLoadMenuResource>,
    mut overlays: bevy::prelude::Query<&mut Text, With<SaveLoadMenuOverlay>>,
) {
    for mut text in &mut overlays {
        text.0 = alpha_save_load_note_text(&menu.summary);
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
    camera: &CameraNavigationResource,
    selection: &SelectionResource,
    inspector: &CreatureInspectorResource,
    gpu: &GraphicalGpuTelemetryResource,
) -> String {
    let snapshot = &inspector.snapshot;
    let action = compact_overlay_line(&snapshot.action_summary, 36);
    let patch = compact_overlay_line(&snapshot.patch_summary, 36);
    let drives = compact_overlay_line(&snapshot.drive_lines.join(", "), 36);
    let follow = camera
        .state
        .follow_target
        .map_or_else(|| "none".to_string(), |id| id.raw().to_string());
    format!(
        concat!(
            "Read-only Inspector\n",
            "Stable ID: {} map={}\n",
            "Org: {:?} kind={:?}\n",
            "Action: {}\n",
            "Patch: {}\n",
            "Drives: {}\n",
            "Visual: {}/{}\n",
            "Cam: ({:.1},{:.1}) z={:.1}\n",
            "Follow: {}\n",
            "Read-only stable IDs\n\n{}"
        ),
        selection.stable_id.raw(),
        selection.local_entity.map(|_| "mapped").unwrap_or("none"),
        snapshot.selection.organism_id.map(|id| id.raw()),
        snapshot.selection.kind,
        action,
        patch,
        drives,
        snapshot.visual.animation.label(),
        snapshot.visual.expression.label(),
        camera.state.focus.x,
        camera.state.focus.z,
        camera.state.zoom,
        follow,
        gpu.telemetry.inspector_lines()
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

pub fn readability_legend_overlay_text() -> String {
    [
        "Visual Guide: [@] creature | [+] food | [!] hazard",
        "Other guide markers: [#] obstacle | [T] token",
        "GPU alpha fixture: creature+food+real hazard+obstacle. P34 remains guide-only.",
        "Creature colors: cyan GPU proposals | green learned H_shadow | gray CPU fallback",
        "All markers are presentation only. Stable IDs stay portable.",
    ]
    .join("\n")
}

pub fn alpha_controls_help_text() -> &'static str {
    "Controls: Space run/pause | N step | R reset | 1/2/3 speed | F follow | Esc quit"
}

pub fn feedback_cue_overlay_text(
    feedback: &crate::FeedbackPolishSummary,
    inspector: &CreatureInspectorResource,
) -> String {
    let snapshot = &inspector.snapshot;
    format!(
        concat!(
            "Play Feedback (display-only)\n",
            "Cues: {}\n",
            "Food={} hazard={} sleep={} failure={}\n",
            "Creature: {}/{} curiosity={:.2}\n",
            "Boundary: cues cannot act or mutate weights"
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
