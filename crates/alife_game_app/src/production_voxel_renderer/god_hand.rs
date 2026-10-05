//! God hand presentation and pointer translation. Physical carrying stays in world.
use super::*;
use bevy::{gltf::Gltf, prelude::*, scene::SceneInstanceReady, window::CursorOptions};

const PATH: &str = "hand/wizard-god-hand.glb";
// Measured from the approved normalized five-pose source, in glTF model space.
const POSE_CONTACTS: [Vec3; 5] = [
    Vec3::new(0.429072, 2.320183, 0.345747),
    Vec3::new(0.447459, 2.356355, 0.230186),
    Vec3::new(0.320020, 1.459_97, 0.589970),
    Vec3::new(0.320020, 1.459_97, 0.589970),
    Vec3::new(0.439802, 2.342351, 0.279382),
];

fn pose_transform(camera_rotation: Quat, contact: Vec3, extent: f32, state: usize) -> Transform {
    let carrying = state == 3;
    let scale = pointer_scale(extent, carrying);
    // Right-hand asset: fingers +Y, dorsal surface -Z, thumb +X.
    let rotation = camera_rotation * Quat::from_rotation_y(std::f32::consts::PI);
    let lift = if matches!(state, 2 | 3) {
        0.0
    } else {
        extent * 0.012
    };
    Transform {
        translation: contact + Vec3::Y * lift - rotation * (POSE_CONTACTS[state] * scale),
        rotation,
        scale: Vec3::splat(scale),
    }
}

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

fn grab_intent_target(
    hovered: Option<StableVoxelObjectRef>,
    world: Option<&alife_world::HeadlessWorld>,
) -> Option<StableVoxelObjectRef> {
    let target = hovered?;
    let id = target.stable_id?;
    (matches!(
        target.kind,
        StableVoxelRefKind::Creature | StableVoxelRefKind::Resource
    ) && world?.can_begin_player_hold(id))
    .then_some(target)
}
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
        #[cfg(feature = "gpu-runtime")]
        let grab_target = grab_intent_target(
            selection.hovered,
            runtime.as_deref().map(|runtime| runtime.runtime.world()),
        );
        #[cfg(not(feature = "gpu-runtime"))]
        let grab_target = None::<StableVoxelObjectRef>;
        hand.pressed = grab_target.and_then(|s| window.cursor_position().map(|p| (s, now, p)));
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
        if hand.held.is_some() {
            if let Some(p) = hand.ground {
                let ground = Vec3f::new(p.x, p.y, p.z);
                let _ = runtime.runtime.move_player_hold(ground);
            }
            if let Some(frame) = frame.as_deref_mut() {
                frame.refresh_world_objects(runtime.runtime.world());
            }
        }
        // Pending Grab and Carry contact the same canonical creature/object.
        let contact_id = hand
            .held
            .or_else(|| hand.pressed.and_then(|(target, _, _)| target.stable_id));
        if let Some(id) = contact_id {
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
    for (mut transform, mut visibility) in &mut hands {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(contact) = hand.position {
            *transform = pose_transform(camera.rotation, contact, extent, state);
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
    fn all_poses_keep_right_dorsal_orientation_and_contact_through_camera_and_zoom_changes() {
        let contact = Vec3::new(12.0, 2.7, -8.0);
        for camera in [
            Quat::IDENTITY,
            Quat::from_euler(EulerRot::YXZ, 1.1, -0.6, 0.3),
            Quat::from_euler(EulerRot::YXZ, -2.2, -1.1, -0.4),
        ] {
            for extent in [2.4, 24.0, 240.0] {
                for (state, pose_contact) in POSE_CONTACTS.iter().enumerate() {
                    let transform = pose_transform(camera, contact, extent, state);
                    let camera_relative = camera.inverse() * transform.rotation;
                    assert!((camera_relative * Vec3::Y - Vec3::Y).length() < 1e-5);
                    assert!((camera_relative * -Vec3::Z - Vec3::Z).length() < 1e-5);
                    assert!((camera_relative * Vec3::X + Vec3::X).length() < 1e-5);
                    let actual = transform.transform_point(*pose_contact);
                    let lift = if matches!(state, 2 | 3) {
                        0.0
                    } else {
                        extent * 0.012
                    };
                    assert!((actual - (contact + Vec3::Y * lift)).length() < 1e-4);
                    assert!(transform.scale.cmpeq(Vec3::splat(transform.scale.x)).all());
                    if state == 3 {
                        assert_eq!(transform.scale, Vec3::ONE);
                    }
                }
            }
        }
    }

    fn target(kind: StableVoxelRefKind, id: Option<WorldEntityId>) -> StableVoxelObjectRef {
        let tile = VoxelTileCoord::new(0, 0);
        StableVoxelObjectRef {
            kind,
            stable_id: id,
            chunk: VoxelChunkCoord::for_tile(16, tile),
            tile: Some(tile),
        }
    }

    #[test]
    fn terrain_misses_and_ui_captured_pointer_samples_never_start_grab() {
        let world = alife_world::HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        let creature = target(StableVoxelRefKind::Creature, world.entity_id("walker"));
        let terrain = target(StableVoxelRefKind::Tile, None);
        assert_eq!(grab_intent_target(Some(terrain), Some(&world)), None);
        assert_eq!(grab_intent_target(None, Some(&world)), None);
        assert_eq!(grab_intent_target(Some(creature), None), None);

        let mut selection = Fvr03ProductionVoxelSelectionResource {
            hovered: Some(creature),
            selected: Some(terrain),
        };
        // UI capture and ray misses supply no hover; the prior selection remains.
        apply_fvr03_pointer_sample(&mut selection, None, true);
        assert_eq!(grab_intent_target(selection.hovered, Some(&world)), None);
        assert_eq!(selection.selected, Some(terrain));
        apply_fvr03_pointer_sample(&mut selection, Some(terrain), true);
        assert_eq!(selection.selected, Some(terrain));
        assert_eq!(grab_intent_target(selection.hovered, Some(&world)), None);
    }

    #[test]
    fn pending_grab_accepts_creatures_and_movable_objects_but_not_fixed_scenery() {
        let mut world = alife_world::HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(2.0, 0.0, 0.0), 0.5)
            .toy("ball", Vec3f::new(4.0, 0.0, 0.0), true)
            .token("token", Vec3f::new(6.0, 0.0, 0.0), 1)
            .toy("fixed toy", Vec3f::new(8.0, 0.0, 0.0), false)
            .build()
            .unwrap();
        for label in ["walker", "food", "ball", "token"] {
            let kind = if label == "walker" {
                StableVoxelRefKind::Creature
            } else {
                StableVoxelRefKind::Resource
            };
            let selected = target(kind, world.entity_id(label));
            assert_eq!(
                grab_intent_target(Some(selected), Some(&world)),
                Some(selected)
            );
        }
        assert_eq!(
            grab_intent_target(
                Some(target(
                    StableVoxelRefKind::Resource,
                    world.entity_id("fixed toy")
                )),
                Some(&world)
            ),
            None
        );
        assert_eq!(
            grab_intent_target(
                Some(target(
                    StableVoxelRefKind::Resource,
                    Some(WorldEntityId(u64::MAX))
                )),
                Some(&world)
            ),
            None
        );
        assert_eq!(
            grab_intent_target(
                Some(target(StableVoxelRefKind::Creature, None)),
                Some(&world)
            ),
            None
        );
        world
            .begin_player_hold(world.entity_id("walker").unwrap())
            .unwrap();
        assert_eq!(
            grab_intent_target(
                Some(target(
                    StableVoxelRefKind::Resource,
                    world.entity_id("ball")
                )),
                Some(&world)
            ),
            None
        );
    }

    #[test]
    fn zoom_keeps_pointer_coverage_and_carry_world_scale() {
        for extent in [9.8, 28.0, 400.0, 1600.0, 9.8] {
            assert!((3.0 * pointer_scale(extent, false) / extent - 0.125).abs() < 0.0001);
            assert_eq!(pointer_scale(extent, true), 1.0);
        }
    }
}
