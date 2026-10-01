//! God hand presentation and pointer translation. Physical carrying stays in world.
use super::*;
use bevy::{gltf::Gltf, prelude::*, scene::SceneInstanceReady, window::CursorOptions};

const PATH: &str = "hand/wizard-god-hand.glb";

fn pointer_scale(extent: f32, carrying: bool) -> f32 {
    if carrying {
        1.0
    } else {
        (extent / 24.0).clamp(0.1, 70.0)
    }
}
#[derive(Resource, Default)]
pub(super) struct HandInteraction {
    pub ground: Option<Vec3>,
    pressed: Option<(StableVoxelObjectRef, f64, Vec2)>,
    pub held: Option<WorldEntityId>,
    release_until: f64,
    pub position: Option<Vec3>,
}
#[derive(Component)]
pub(super) struct GodHand;
#[derive(Component)]
struct HandSource(Handle<Gltf>);
#[derive(Component)]
pub(super) struct HandPlayer {
    clips: Vec<AnimationNodeIndex>,
    state: usize,
}
#[derive(Component)]
pub(super) struct SelectedTint {
    original: Handle<StandardMaterial>,
    highlighted: Handle<StandardMaterial>,
}

pub(super) fn spawn(world: &mut World) {
    world.init_resource::<HandInteraction>();
    let Some(server) = world.get_resource::<AssetServer>() else {
        return;
    };
    let scene = server.load(GltfAssetLabel::Scene(0).from_asset(PATH));
    let gltf = server.load(PATH);
    world
        .spawn((
            Name::new("Disembodied wizard god hand"),
            GodHand,
            HandSource(gltf),
            SceneRoot(scene),
            Transform::default(),
            Visibility::Hidden,
            Fvr04ProductionRuntimeSceneRoot,
        ))
        .observe(ready);
}
fn ready(
    event: On<SceneInstanceReady>,
    mut commands: Commands,
    sources: Query<&HandSource>,
    children: Query<&Children>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<&mut AnimationPlayer>,
) {
    let Ok(source) = sources.get(event.entity) else {
        return;
    };
    let Some(gltf) = gltfs.get(&source.0) else {
        return;
    };
    let (graph, clips) = AnimationGraph::from_clips(
        ["Hover", "Point", "Grab", "Carry", "Release"]
            .map(|name| gltf.named_animations[name].clone()),
    );
    let graph = graphs.add(graph);
    for entity in children.iter_descendants(event.entity) {
        if let Ok(mut player) = players.get_mut(entity) {
            player.play(clips[0]).repeat();
            commands.entity(entity).insert((
                AnimationGraphHandle(graph.clone()),
                HandPlayer {
                    clips: clips.clone(),
                    state: 0,
                },
            ));
        }
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS system parameters."
)]
pub(super) fn input(
    time: Res<Time<bevy::time::Real>>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut hand: ResMut<HandInteraction>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    mut follow: ResMut<Fvr04ProductionCreatureFollowResource>,
    #[cfg(feature = "gpu-runtime")] mut runtime: Option<
        NonSendMut<ProductionGpuBrainRuntimeResource>,
    >,
    mut frame: Option<ResMut<LiveBrainPresentationFrameResource>>,
    terrain: Option<Res<creature_grounding::SelectedTerrain>>,
    creatures: Query<
        (&Fvr04ProductionCreatureVisualMarker, &Transform),
        Without<Fvr03ProductionVoxelCamera>,
    >,
    mut cameras: Query<&mut Transform, With<Fvr03ProductionVoxelCamera>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let now = time.elapsed_secs_f64();
    if mouse.just_pressed(MouseButton::Left) && window.focused {
        hand.pressed = selection
            .hovered
            .and_then(|s| window.cursor_position().map(|p| (s, now, p)));
        if let Some(s) = selection
            .hovered
            .filter(|s| s.kind == StableVoxelRefKind::Creature)
        {
            follow.target_stable_id = s.stable_id;
            follow.enabled = true;
        } else if selection.hovered.is_some() {
            follow.enabled = false;
            if let (Some(target), Some(terrain)) = (hand.ground, terrain.as_ref()) {
                for mut camera in &mut cameras {
                    let focus = highlands::focus_on_surface(&camera, terrain.0.surface());
                    camera.translation += Vec3::new(target.x - focus.x, 0.0, target.z - focus.z);
                }
            }
        }
    }
    #[cfg(feature = "gpu-runtime")]
    if let Some(runtime) = runtime.as_deref_mut() {
        if hand.held.is_some() && runtime.runtime.world().player_held_object() != hand.held {
            hand.held = None;
            hand.release_until = now + 0.18;
        }
        let cancel = !window.focused
            || keyboard.just_pressed(KeyCode::Escape)
            || !mouse.pressed(MouseButton::Left);
        if cancel {
            if hand.held.take().is_some() || runtime.runtime.world().player_held_object().is_some()
            {
                let _ = runtime.runtime.release_player_hold();
                if let Some(frame) = frame.as_deref_mut() {
                    frame.refresh_world_objects(runtime.runtime.world());
                }
                hand.release_until = now + 0.18;
            }
            hand.pressed = None;
        } else if let Some((target, started, _)) = hand.pressed {
            if hand.held.is_none() && now - started >= 0.25 {
                if let Some(id) = target.stable_id.filter(|_| {
                    matches!(
                        target.kind,
                        StableVoxelRefKind::Creature | StableVoxelRefKind::Resource
                    )
                }) {
                    if runtime.runtime.begin_player_hold(id).is_ok() {
                        hand.held = Some(id);
                        follow.enabled = false;
                    }
                }
                hand.pressed = None;
            }
        }
        if let Some(id) = hand.held {
            if let Some(p) = hand.ground {
                let ground = if runtime.runtime.world().terrain().is_some() {
                    Vec3f::new(p.x, p.y, p.z)
                } else {
                    Vec3f::new(p.x, p.z, 0.0)
                };
                let _ = runtime.runtime.move_player_hold(ground);
            }
            hand.position = runtime.runtime.world().entity(id).map(|o| {
                let base = world_position_for_render(
                    o.position,
                    runtime.runtime.world().terrain().is_some(),
                );
                if o.kind == WorldObjectKind::Agent {
                    let scale = creatures
                        .iter()
                        .find(|(v, _)| v.stable_id == id)
                        .map_or(Vec3::ONE, |(_, t)| t.scale);
                    base + Vec3::Y * scale.y
                        - Vec3::new(o.body_yaw.cos(), 0.0, o.body_yaw.sin()) * (0.18 * scale.x)
                } else {
                    base + Vec3::Y * 0.30
                }
            });
            if let Some(frame) = frame.as_deref_mut() {
                frame.refresh_world_objects(runtime.runtime.world());
            }
        } else {
            hand.position = hand.ground.map(|p| p + Vec3::Y * 1.0);
        }
    }
    #[cfg(not(feature = "gpu-runtime"))]
    {
        hand.position = hand.ground.map(|p| p + Vec3::Y * 1.0);
        let _ = &mut frame;
    }
}

type HandCameraFilter = (With<Fvr03ProductionVoxelCamera>, Without<GodHand>);
type HandTransformFilter = (With<GodHand>, Without<Fvr03ProductionVoxelCamera>);

pub(super) fn animate(
    time: Res<Time<bevy::time::Real>>,
    hand: Res<HandInteraction>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    cameras: Query<(&Transform, &Projection), HandCameraFilter>,
    mut hands: Query<(&mut Transform, &mut Visibility), HandTransformFilter>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut players: Query<(&mut AnimationPlayer, &mut HandPlayer)>,
) {
    let Some((camera, projection)) = cameras.iter().next() else {
        return;
    };
    let visible = windows
        .single()
        .is_ok_and(|(w, _)| w.focused && w.cursor_position().is_some())
        && hand.position.is_some();
    let state = if hand.held.is_some() {
        3
    } else if time.elapsed_secs_f64() < hand.release_until {
        4
    } else if hand.pressed.is_some() {
        2
    } else if selection.hovered.is_some_and(|s| s.stable_id.is_some()) {
        1
    } else {
        0
    };
    for (mut player, mut pose) in &mut players {
        if state != pose.state {
            player.stop_all();
            player.play(pose.clips[state]).repeat();
            pose.state = state;
        }
    }
    let extent = match projection {
        Projection::Orthographic(p) => match p.scaling_mode {
            bevy::camera::ScalingMode::FixedVertical { viewport_height } => {
                viewport_height * p.scale
            }
            _ => 10.0,
        },
        _ => 10.0,
    };
    // Keep the pointer readable at overview zoom. Carrying retains actual model scale.
    let scale = pointer_scale(extent, hand.held.is_some());
    // Exported hand points +Y; tip-to-wrist orientation sits above/behind the target.
    let rotation = if hand.held.is_some() {
        // Approach from above/left in the camera view. The pinch stays on the
        // upper back while the palm and unused fingers leave the face readable.
        camera.rotation
            * Quat::from_rotation_z(0.75)
            * Quat::from_rotation_y(0.30)
            * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
    } else {
        Quat::from_rotation_y(-0.45) * Quat::from_rotation_x(2.25)
    };
    for (mut transform, mut visibility) in &mut hands {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(contact) = hand.position {
            let tip_local = if hand.held.is_some() {
                Vec3::new(-0.32, 1.46, 0.59)
            } else {
                Vec3::new(-0.345, 2.8, 0.05)
            };
            let lift = if hand.held.is_some() {
                0.0
            } else {
                extent * 0.012
            };
            transform.rotation = rotation;
            transform.scale = Vec3::splat(scale);
            transform.translation = contact + Vec3::Y * lift - rotation * (tip_local * scale);
        }
    }
    if let Ok((_, mut cursor)) = windows.single_mut() {
        cursor.visible = !visible;
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS system parameters."
)]
pub(super) fn highlight(
    mut commands: Commands,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    children: Query<&Children>,
    roots: Query<(Entity, &ProductionCreatureAssemblyRoot)>,
    objects: Query<(Entity, &live_food_projection::LiveCareObject)>,
    mut meshes: Query<(
        Entity,
        &mut MeshMaterial3d<StandardMaterial>,
        Option<&SelectedTint>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut markers: Query<&mut Visibility, With<Fvr03ProductionVoxelSelectionMarker>>,
) {
    for mut v in &mut markers {
        *v = Visibility::Hidden;
    }
    let selected = selection.selected.and_then(|s| s.stable_id);
    let target = roots
        .iter()
        .find(|(_, r)| Some(r.stable_id) == selected)
        .map(|(e, _)| e)
        .or_else(|| {
            objects
                .iter()
                .find(|(_, o)| Some(o.0) == selected)
                .map(|(e, _)| e)
        });
    let descendants: Vec<_> = target
        .into_iter()
        .flat_map(|root| std::iter::once(root).chain(children.iter_descendants(root)))
        .collect();
    for (entity, mut handle, tint) in &mut meshes {
        if descendants.contains(&entity) {
            if let Some(tint) = tint {
                handle.0 = tint.highlighted.clone();
            } else if let Some(source) = materials.get(&handle.0) {
                let mut highlighted = source.clone();
                highlighted.emissive = bevy::color::LinearRgba::rgb(0.12, 0.095, 0.025);
                let highlighted = materials.add(highlighted);
                let original = handle.0.clone();
                handle.0 = highlighted.clone();
                commands.entity(entity).insert(SelectedTint {
                    original,
                    highlighted,
                });
            }
        } else if let Some(tint) = tint {
            handle.0 = tint.original.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_keeps_pointer_coverage_and_carry_world_scale() {
        for extent in [9.8, 28.0, 400.0, 1600.0, 9.8] {
            assert!((3.0 * pointer_scale(extent, false) / extent - 0.125).abs() < 0.0001);
            assert_eq!(pointer_scale(extent, true), 1.0);
        }
    }
}
