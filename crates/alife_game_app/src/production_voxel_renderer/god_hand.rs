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

#[cfg(any(feature = "gpu-runtime", test))]
fn object_contact_position(
    object: &alife_world::WorldObject,
    terrain_bound: bool,
    surface: &creature_grounding::RenderedTerrainSurface,
    creature_scale: Vec3,
) -> Vec3 {
    let base = world_position_for_render(object.position, terrain_bound);
    match object.kind {
        WorldObjectKind::Agent => {
            base + Vec3::Y * creature_scale.y
                - Vec3::new(object.body_yaw.cos(), 0.0, object.body_yaw.sin())
                    * (0.18 * creature_scale.x)
        }
        WorldObjectKind::Food | WorldObjectKind::Ball | WorldObjectKind::ActivityToy => {
            live_food_projection::care_object_center(object.position, terrain_bound, surface)
        }
        _ => base + Vec3::Y * 0.30,
    }
}

#[cfg(any(feature = "gpu-runtime", test))]
fn contact_id(hand: &HandInteraction) -> Option<WorldEntityId> {
    hand.held
        .or_else(|| hand.pressed.and_then(|(target, _, _)| target.stable_id))
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
    #[cfg(feature = "gpu-runtime")] surface: Res<creature_grounding::RenderedTerrainSurface>,
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
                    let focus = island::focus_on_surface(&camera, terrain.0.surface());
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
        if let Some(id) = contact_id(&hand) {
            hand.position = runtime.runtime.world().entity(id).map(|object| {
                let scale = creatures
                    .iter()
                    .find(|(visual, _)| visual.stable_id == id)
                    .map_or(Vec3::ONE, |(_, transform)| transform.scale);
                object_contact_position(
                    object,
                    runtime.runtime.world().terrain().is_some(),
                    &surface,
                    scale,
                )
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

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS system parameters."
)]
pub(super) fn animate(
    time: Res<Time<bevy::time::Real>>,
    menu: Option<Res<menu::MenuState>>,
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
    let visible = !menu::captures_input(menu)
        && windows
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
    fn pending_and_carry_care_contacts_match_the_visual_center_across_drop_and_regrab() {
        let world = alife_world::HeadlessScenarioBuilder::new(17)
            .food("food", Vec3f::new(0.5, 2.0, 0.5), 0.5)
            .toy("ball", Vec3f::new(0.5, 2.0, 0.5), true)
            .build()
            .unwrap();
        let original = world.object_snapshots();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [0.0, 0.9, 0.0],
                [0.0, 0.9, 1.0],
                [1.0, 0.9, 1.0],
                [1.0, 0.9, 0.0],
            ],
        );
        for terrain_bound in [false, true] {
            for measured in [false, true] {
                let surface = creature_grounding::RenderedTerrainSurface::from_meshes(
                    measured.then_some(&mesh),
                    1.0,
                );
                for object in &original {
                    let selected = target(StableVoxelRefKind::Resource, Some(object.id));
                    let mut hand = HandInteraction::default();
                    for held in [false, true, true, false, true] {
                        hand.held = held.then_some(object.id);
                        hand.pressed = (!held).then_some((selected, 1.0, Vec2::ZERO));
                        assert_eq!(contact_id(&hand), Some(object.id));
                        let center = object_contact_position(
                            object,
                            terrain_bound,
                            &surface,
                            Vec3::splat(9.0),
                        );
                        assert_eq!(
                            center,
                            live_food_projection::care_object_center(
                                object.position,
                                terrain_bound,
                                &surface
                            )
                        );
                        let expected_y = 2.30
                            + if terrain_bound {
                                0.0
                            } else if measured {
                                0.9
                            } else {
                                0.44
                            };
                        assert!((center.y - expected_y).abs() < 1e-5);
                        for state in [2, 3] {
                            let transform = pose_transform(Quat::IDENTITY, center, 28.0, state);
                            assert!(
                                (transform.transform_point(POSE_CONTACTS[state]) - center).length()
                                    < 1e-5
                            );
                        }
                        hand.held = None;
                        hand.pressed = None;
                        assert_eq!(contact_id(&hand), None);
                    }
                }
            }
        }
        assert_eq!(world.object_snapshots(), original);
    }

    #[test]
    fn world_grab_move_drop_and_regrab_recompute_care_display_without_height_feedback() {
        // Compose the real world hold APIs, immediate frame refresh, care-object
        // ECS projection and hand contact helpers. The GPU-runtime-backed mouse
        // input system and actual GLB animation are deliberately not exercised.
        for terrain_bound in [false, true] {
            for measured in [false, true] {
                let mut world = alife_world::HeadlessScenarioBuilder::new(17)
                    .food("food", Vec3f::new(0.5, 2.0, 0.5), 0.5)
                    .toy("ball", Vec3f::new(0.5, 2.0, 0.5), true)
                    .build()
                    .unwrap();
                if terrain_bound {
                    let terrain = alife_world::WorldTerrain::new(
                        alife_world::TerrainData {
                            width: 2,
                            depth: 2,
                            origin_x: -5.0,
                            origin_z: -5.0,
                            spacing: 10.0,
                            heights: vec![2.0; 4],
                            obstacles: Vec::new(),
                            water_level: None,
                        },
                        alife_world::LocomotionLimits::default(),
                    )
                    .unwrap();
                    world
                        .enable_terrain_for_new_game(terrain, Vec3f::ZERO)
                        .unwrap();
                }
                let mut mesh = Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::default(),
                );
                mesh.insert_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    (0..4)
                        .flat_map(|z| {
                            (0..4).flat_map(move |x| {
                                let (x, z) = (x as f32, z as f32);
                                [
                                    [x, 0.9, z],
                                    [x, 0.9, z + 1.0],
                                    [x + 1.0, 0.9, z + 1.0],
                                    [x + 1.0, 0.9, z],
                                ]
                            })
                        })
                        .collect::<Vec<_>>(),
                );
                let mut app = App::new();
                app.insert_resource(
                    LiveBrainPresentationFrameResource::from_authoritative_world(&world).unwrap(),
                )
                .insert_resource(creature_grounding::RenderedTerrainSurface::from_meshes(
                    measured.then_some(&mesh),
                    1.0,
                ))
                .insert_resource(Assets::<Mesh>::default())
                .insert_resource(Assets::<StandardMaterial>::default())
                .add_systems(Update, live_food_projection::sync_care_objects);
                if let Some(terrain) = world.terrain() {
                    app.insert_resource(creature_grounding::SelectedTerrain(terrain.clone()));
                }
                let assert_projection = |app: &mut App,
                                         world: &alife_world::HeadlessWorld,
                                         id,
                                         expected: Vec3f| {
                    let snapshots = world.object_snapshots();
                    let object = world.entity(id).unwrap();
                    assert_eq!(object.position, expected);
                    app.world_mut()
                        .resource_mut::<LiveBrainPresentationFrameResource>()
                        .refresh_world_objects(world);
                    app.update();
                    let frame = app.world().resource::<LiveBrainPresentationFrameResource>();
                    assert_eq!(frame.current.object(id), Some(object));
                    assert_eq!(frame.current.authoritative_world_tick, world.tick());
                    let surface = app
                        .world()
                        .resource::<creature_grounding::RenderedTerrainSurface>();
                    let sampled = surface.height(Vec3::new(expected.x, 0.0, expected.z));
                    assert_eq!(sampled.is_some(), measured);
                    if let Some(height) = sampled {
                        assert!((height - 0.9).abs() < 1e-5);
                    }
                    let center = live_food_projection::care_object_center(
                        object.position,
                        terrain_bound,
                        surface,
                    );
                    let lift = if terrain_bound {
                        0.0
                    } else if measured {
                        0.9
                    } else {
                        0.44
                    };
                    let expected_center =
                        Vec3::new(expected.x, expected.y + lift + 0.30, expected.z);
                    assert!((center - expected_center).length() < 1e-5);
                    let contact =
                        object_contact_position(object, terrain_bound, surface, Vec3::ONE);
                    assert_eq!(contact, center);
                    let transform = pose_transform(Quat::IDENTITY, contact, 28.0, 3);
                    assert!((transform.transform_point(POSE_CONTACTS[3]) - center).length() < 1e-5);
                    let mut query = app
                        .world_mut()
                        .query::<(&live_food_projection::LiveCareObject, &Transform)>();
                    let displayed = query
                        .iter(app.world())
                        .find(|(marker, _)| marker.0 == id)
                        .unwrap()
                        .1;
                    assert_eq!(displayed.translation, center);
                    assert_eq!(world.object_snapshots(), snapshots);
                };
                for label in ["food", "ball"] {
                    let id = world.entity_id(label).unwrap();
                    let mut ground = world.entity(id).unwrap().position;
                    assert_projection(&mut app, &world, id, ground);
                    for destination in [Vec3f::new(1.5, 4.0, 2.0), Vec3f::new(2.5, 5.0, 3.0)] {
                        let mut raised = ground;
                        if terrain_bound {
                            raised.y += 1.3;
                        } else {
                            raised.z = 1.3;
                        }
                        assert_eq!(world.begin_player_hold(id).unwrap(), raised);
                        assert_eq!(world.player_held_object(), Some(id));
                        assert_projection(&mut app, &world, id, raised);
                        ground = if terrain_bound {
                            Vec3f::new(destination.x, 2.0, destination.z)
                        } else {
                            Vec3f::new(destination.x, destination.y, 0.0)
                        };
                        raised = ground;
                        if terrain_bound {
                            raised.y += 1.3;
                        } else {
                            raised.z += 1.3;
                        }
                        assert_eq!(world.move_player_hold(destination).unwrap(), raised);
                        assert_projection(&mut app, &world, id, raised);
                        assert_eq!(world.release_player_hold().unwrap(), Some(id));
                        assert_eq!(world.player_held_object(), None);
                        assert!(world.can_begin_player_hold(id));
                        assert_projection(&mut app, &world, id, ground);
                    }
                }
            }
        }
    }

    #[test]
    fn contact_helper_preserves_creature_and_token_rules_and_held_priority() {
        let world = alife_world::HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::new(0.5, 2.0, 0.5))
            .token("token", Vec3f::new(0.5, 2.0, 0.5), 1)
            .build()
            .unwrap();
        let objects = world.object_snapshots();
        let surface =
            creature_grounding::RenderedTerrainSurface::from_meshes(std::iter::empty(), 1.0);
        let scale = Vec3::new(2.0, 3.0, 4.0);
        let creature = &objects[0];
        let expected = world_position_for_render(creature.position, false) + Vec3::Y * scale.y
            - Vec3::new(creature.body_yaw.cos(), 0.0, creature.body_yaw.sin()) * (0.18 * scale.x);
        assert_eq!(
            object_contact_position(creature, false, &surface, scale),
            expected
        );
        let token = &objects[1];
        assert_eq!(
            object_contact_position(token, false, &surface, scale),
            world_position_for_render(token.position, false) + Vec3::Y * 0.30
        );
        let hand = HandInteraction {
            held: Some(creature.id),
            pressed: Some((
                target(StableVoxelRefKind::Resource, Some(token.id)),
                1.0,
                Vec2::ZERO,
            )),
            ..default()
        };
        assert_eq!(contact_id(&hand), Some(creature.id));
    }

    #[test]
    fn zoom_keeps_pointer_coverage_and_carry_world_scale() {
        for extent in [9.8, 28.0, 400.0, 1600.0, 9.8] {
            assert!((3.0 * pointer_scale(extent, false) / extent - 0.125).abs() < 0.0001);
            assert_eq!(pointer_scale(extent, true), 1.0);
        }
    }
}

/// Cancel a pointer gesture before the menu owns input. Carrying remains a world API.
pub(super) fn cancel_for_menu(world: &mut World) {
    if let Some(mut hand) = world.get_resource_mut::<HandInteraction>() {
        hand.pressed = None;
    }
    #[cfg(feature = "gpu-runtime")]
    if let Some(mut runtime) =
        world.get_non_send_resource_mut::<ProductionGpuBrainRuntimeResource>()
    {
        if runtime.runtime.world().player_held_object().is_some() {
            if let Err(error) = runtime.runtime.release_player_hold() {
                if let Some(mut ux) = world.get_resource_mut::<Fvr05ProductionUxStateResource>() {
                    ux.last_error = Some(error.to_string());
                    ux.last_action = "Hand release failed; menu remains paused".into();
                }
                return;
            }
        }
    }
    #[cfg(feature = "gpu-runtime")]
    world.resource_scope(
        |world, mut frame: Mut<LiveBrainPresentationFrameResource>| {
            if let Some(runtime) =
                world.get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
            {
                frame.refresh_world_objects(runtime.runtime.world());
            }
        },
    );
    if let Some(mut hand) = world.get_resource_mut::<HandInteraction>() {
        hand.held = None;
    }
}
