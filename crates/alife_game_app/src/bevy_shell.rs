//! Feature-gated Bevy playground shell split during R13 remediation.

use alife_bevy_adapter::{
    core_vec3_to_bevy, AffordanceTags, AlifeBevyAdapterPlugin, BevyEntityMap, CreatureBody,
    SensoryEmitter,
};
use alife_core::{AffordanceBits, WorldEntityId};
use alife_world::WorldObjectKind;
use bevy::prelude::{App, Component, Entity, MinimalPlugins, Resource, Transform};

use crate::{
    load_visible_world_from_p34_save, run_creature_inspector_smoke, run_creature_visual_smoke,
    run_live_brain_loop_smoke, AppShellLaunchConfig, AppStartupSummary, CameraNavigationState,
    CreatureAnimationState, CreatureExpressionState, CreatureInspectorSnapshot,
    CreatureVisualSnapshot, EntitySelectionSnapshot, GameAppShellError, GameAppState,
    LiveBrainTickSummary, VisibleMaterialKind, VisiblePlaceholderShape, VisibleWorldPresentation,
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
