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
    last_tile_click: Option<(VoxelTileCoord, f64)>,
    resume_follow_after_drop: bool,
}
#[derive(Component)]
pub(super) struct GodHand;
#[derive(Component)]
struct HandSource(Handle<Gltf>);

fn hold_activation_delay_secs(kind: StableVoxelRefKind) -> f64 {
    match kind {
        StableVoxelRefKind::Resource => 0.08,
        _ => 0.25,
    }
}

fn should_recenter_camera_on_click(
    hovered: Option<StableVoxelObjectRef>,
    now: f64,
    last_tile_click: &mut Option<(VoxelTileCoord, f64)>,
) -> bool {
    let Some(tile) = hovered
        .filter(|s| s.kind == StableVoxelRefKind::Tile)
        .and_then(|s| s.tile)
    else {
        *last_tile_click = None;
        return false;
    };
    let is_double_click = last_tile_click.is_some_and(|(prev_tile, prev_at)| {
        prev_tile == tile && (0.0..=0.35).contains(&(now - prev_at))
    });
    if is_double_click {
        *last_tile_click = None;
        true
    } else {
        *last_tile_click = Some((tile, now));
        false
    }
}

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

#[cfg(any(feature = "gpu-runtime", test))]
/// Ring radii (metres) searched around the cursor when the exact carry point is unsupported.
const HOLD_SLIDE_RADII: [f32; 4] = [0.45, 0.9, 1.5, 2.2];

#[cfg(any(feature = "gpu-runtime", test))]
/// Ordered ground points for carrying an object toward `desired`.
///
/// The exact cursor point is tried first. When it lies on a boulder, shoreline, or steep bank
/// the world rejects it, so nearby rings are searched starting from the side facing the
/// object's current carry position (`current`). The held creature or food therefore slides
/// along the obstacle boundary under the hand instead of freezing on the far side of it.
/// The world remains the only authority that accepts or rejects each candidate.
fn hold_slide_candidates(
    desired: Vec3f,
    current: Option<Vec3f>,
    terrain_bound: bool,
) -> Vec<Vec3f> {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};
    let lateral = |p: Vec3f| if terrain_bound { p.z } else { p.y };
    let at = |dx: f32, dl: f32| {
        if terrain_bound {
            Vec3f::new(desired.x + dx, desired.y, desired.z + dl)
        } else {
            Vec3f::new(desired.x + dx, desired.y + dl, desired.z)
        }
    };
    let toward_current = current
        .map(|c| (lateral(c) - lateral(desired), c.x - desired.x))
        .filter(|(dl, dx)| dl.hypot(*dx) > 1.0e-4)
        .map_or(0.0, |(dl, dx)| dl.atan2(dx));
    let mut candidates = Vec::with_capacity(1 + HOLD_SLIDE_RADII.len() * 8);
    candidates.push(desired);
    for radius in HOLD_SLIDE_RADII {
        for offset in [
            0.0,
            FRAC_PI_4,
            -FRAC_PI_4,
            FRAC_PI_2,
            -FRAC_PI_2,
            3.0 * FRAC_PI_4,
            -3.0 * FRAC_PI_4,
            PI,
        ] {
            let angle = toward_current + offset;
            candidates.push(at(radius * angle.cos(), radius * angle.sin()));
        }
    }
    candidates
}

#[cfg(any(feature = "gpu-runtime", test))]
/// Moves the current player hold to the first world-accepted slide candidate.
/// Returns the accepted ground point, or `None` when the hold stays where it was.
fn slide_player_hold(
    desired: Vec3f,
    current: Option<Vec3f>,
    terrain_bound: bool,
    mut try_move: impl FnMut(Vec3f) -> bool,
) -> Option<Vec3f> {
    // Ordinary open-ground movement needs neither allocation nor a ring search.
    if try_move(desired) {
        return Some(desired);
    }
    hold_slide_candidates(desired, current, terrain_bound)
        .into_iter()
        .skip(1)
        .find(|candidate| try_move(*candidate))
}

#[cfg(any(feature = "gpu-runtime", test))]
/// Live selection reference for a creature after it has been set down, so the HUD,
/// care dock, and speech target the creature at its landing tile.
fn dropped_creature_selection(
    id: WorldEntityId,
    ground: Vec3f,
    terrain_bound: bool,
) -> StableVoxelObjectRef {
    let tile = VoxelTileCoord::new(
        ground.x.floor() as i32,
        if terrain_bound { ground.z } else { ground.y }.floor() as i32,
    );
    StableVoxelObjectRef {
        kind: StableVoxelRefKind::Creature,
        stable_id: Some(id),
        chunk: VoxelChunkCoord::for_tile(16, tile),
        tile: Some(tile),
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
    mut selection: ResMut<Fvr03ProductionVoxelSelectionResource>,
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
    if !window.focused || keyboard.just_pressed(KeyCode::Escape) {
        hand.last_tile_click = None;
    }
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
            hand.last_tile_click = None;
            follow.target_stable_id = s.stable_id;
            follow.enabled = true;
        } else if should_recenter_camera_on_click(selection.hovered, now, &mut hand.last_tile_click)
        {
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
            hand.resume_follow_after_drop = false;
            hand.release_until = now + 0.18;
        }
        let cancel = !window.focused
            || keyboard.just_pressed(KeyCode::Escape)
            || !mouse.pressed(MouseButton::Left);
        if cancel {
            if hand.held.take().is_some() || runtime.runtime.world().player_held_object().is_some()
            {
                // A deliberate drop (button released in a focused window) keeps the creature in
                // the player's attention, as in Black & White: the HUD, care dock, and speech stay
                // on it at its landing tile, and the camera resumes following it.
                let natural_drop = window.focused && !keyboard.just_pressed(KeyCode::Escape);
                let resume_follow = std::mem::take(&mut hand.resume_follow_after_drop);
                if let Ok(Some(released)) = runtime.runtime.release_player_hold() {
                    let world = runtime.runtime.world();
                    let terrain_bound = world.terrain().is_some();
                    let landed = world
                        .entity(released)
                        .filter(|o| o.kind == WorldObjectKind::Agent && o.organism_id.is_some())
                        .map(|o| o.position);
                    if let (true, Some(ground)) = (natural_drop, landed) {
                        selection.selected =
                            Some(dropped_creature_selection(released, ground, terrain_bound));
                        follow.target_stable_id = Some(released);
                        if resume_follow {
                            follow.enabled = true;
                        }
                    }
                }
                if let Some(frame) = frame.as_deref_mut() {
                    frame.refresh_world_objects(runtime.runtime.world());
                }
                hand.release_until = now + 0.18;
            }
            hand.pressed = None;
        } else if let Some((target, started, _)) = hand.pressed {
            if hand.held.is_none() && now - started >= hold_activation_delay_secs(target.kind) {
                if let Some(id) = target.stable_id.filter(|_| {
                    matches!(
                        target.kind,
                        StableVoxelRefKind::Creature | StableVoxelRefKind::Resource
                    )
                }) {
                    if runtime.runtime.begin_player_hold(id).is_ok() {
                        hand.held = Some(id);
                        hand.resume_follow_after_drop = target.kind == StableVoxelRefKind::Creature
                            && follow.enabled
                            && follow.target_stable_id == Some(id);
                        if target.kind == StableVoxelRefKind::Creature {
                            follow.enabled = false;
                        }
                    }
                }
                hand.pressed = None;
            }
        }
        if hand.held.is_some() {
            if let Some(p) = hand.ground {
                let desired = Vec3f::new(p.x, p.y, p.z);
                let terrain_bound = runtime.runtime.world().terrain().is_some();
                let current = hand
                    .held
                    .and_then(|id| runtime.runtime.world().entity(id))
                    .map(|object| object.position);
                let _ = slide_player_hold(desired, current, terrain_bound, |candidate| {
                    runtime.runtime.move_player_hold(candidate).is_ok()
                });
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
        let _ = &mut selection;
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
    dock: Option<Res<care_dock::State>>,
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
        && !care_dock::captures_input(dock)
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
    clippy::type_complexity,
    reason = "Bevy injects independent ECS system parameters."
)]
pub(super) fn highlight(
    mut commands: Commands,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    children: Query<&Children>,
    roots: Query<(Entity, &ProductionCreatureAssemblyRoot)>,
    objects: Query<(Entity, &live_food_projection::LiveCareObject)>,
    mut meshes: Query<
        (
            Entity,
            &mut MeshMaterial3d<StandardMaterial>,
            Option<&SelectedTint>,
        ),
        Without<live_sleep_feedback::SleepBubble>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut markers: Query<&mut Visibility, With<Fvr03ProductionVoxelSelectionMarker>>,
    tinted: Query<Entity, With<SelectedTint>>,
) {
    for mut v in &mut markers {
        if *v != Visibility::Hidden {
            *v = Visibility::Hidden;
        }
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
    // Resolve the current hierarchy every frame so late glTF meshes, reparenting
    // and replacement roots need no separate invalidation cache. Unrelated
    // terrain and prop materials are never visited by this presentation pass.
    let descendants: BTreeSet<_> = target
        .into_iter()
        .flat_map(|root| std::iter::once(root).chain(children.iter_descendants(root)))
        .collect();
    for entity in &tinted {
        if !descendants.contains(&entity) {
            if let Ok((_, mut handle, Some(tint))) = meshes.get_mut(entity) {
                if handle.0 != tint.original {
                    handle.0 = tint.original.clone();
                }
            }
        }
    }
    for entity in descendants {
        let Ok((_, mut handle, tint)) = meshes.get_mut(entity) else {
            continue;
        };
        if let Some(tint) = tint {
            if handle.0 != tint.highlighted {
                handle.0 = tint.highlighted.clone();
            }
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
    }
}

#[cfg(test)]
#[path = "god_hand_highlight_tests.rs"]
mod highlight_tests;

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
        hand.resume_follow_after_drop = false;
        hand.last_tile_click = None;
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
    #[test]
    fn single_clicks_on_ground_or_resources_do_not_teleport_camera_until_double_click() {
        let tile_a = VoxelTileCoord::new(4, 7);
        let tile_b = VoxelTileCoord::new(5, 7);
        let ref_tile_a = StableVoxelObjectRef {
            kind: StableVoxelRefKind::Tile,
            stable_id: None,
            chunk: VoxelChunkCoord::for_tile(16, tile_a),
            tile: Some(tile_a),
        };
        let ref_tile_b = StableVoxelObjectRef {
            kind: StableVoxelRefKind::Tile,
            stable_id: None,
            chunk: VoxelChunkCoord::for_tile(16, tile_b),
            tile: Some(tile_b),
        };
        let ref_food = target(StableVoxelRefKind::Resource, Some(WorldEntityId(9)));
        let mut last = None;

        // Single click on ground records the tile without recentering.
        assert!(!should_recenter_camera_on_click(
            Some(ref_tile_a),
            1.0,
            &mut last
        ));
        assert_eq!(last, Some((tile_a, 1.0)));

        // Clicking a resource clears the double-click chain without recentering.
        assert!(!should_recenter_camera_on_click(
            Some(ref_food),
            1.1,
            &mut last
        ));
        assert_eq!(last, None);

        // Clicking two different tiles rapidly does not recenter.
        assert!(!should_recenter_camera_on_click(
            Some(ref_tile_a),
            2.0,
            &mut last
        ));
        assert!(!should_recenter_camera_on_click(
            Some(ref_tile_b),
            2.15,
            &mut last
        ));
        assert_eq!(last, Some((tile_b, 2.15)));

        // Double-clicking the same tile within 0.35s recenters and consumes the state.
        assert!(should_recenter_camera_on_click(
            Some(ref_tile_b),
            2.40,
            &mut last
        ));
        assert_eq!(last, None);
    }

    #[test]
    fn resource_pickup_activates_snappier_than_creature_lift() {
        assert!(
            hold_activation_delay_secs(StableVoxelRefKind::Resource)
                < hold_activation_delay_secs(StableVoxelRefKind::Creature)
        );
        assert_eq!(
            hold_activation_delay_secs(StableVoxelRefKind::Resource),
            0.08
        );
        assert_eq!(
            hold_activation_delay_secs(StableVoxelRefKind::Creature),
            0.25
        );
    }

    #[test]
    fn carried_objects_slide_around_rejected_points_toward_the_carry_side() {
        let desired = Vec3f::new(10.0, 2.0, 10.0);
        let candidates = hold_slide_candidates(desired, Some(Vec3f::new(7.0, 3.3, 10.0)), true);
        assert_eq!(
            candidates[0], desired,
            "exact cursor point is always tried first"
        );
        assert_eq!(candidates.len(), 1 + HOLD_SLIDE_RADII.len() * 8);
        // The first ring candidate faces the object's current (-X) side and stays on the ground plane.
        assert!((candidates[1].x - (10.0 - HOLD_SLIDE_RADII[0])).abs() < 1.0e-4);
        assert!((candidates[1].z - 10.0).abs() < 1.0e-4);
        assert_eq!(candidates[1].y, 2.0);

        // A boulder of radius 1.0 under the cursor: the hold slides to the nearest accepted point.
        let accepted = slide_player_hold(desired, Some(Vec3f::new(7.0, 3.3, 10.0)), true, |p| {
            (p.x - 10.0).hypot(p.z - 10.0) > 1.0
        })
        .unwrap();
        assert!((accepted.x - (10.0 - HOLD_SLIDE_RADII[2])).abs() < 1.0e-4);
        assert!((accepted.z - 10.0).abs() < 1.0e-4);

        // Legacy flat worlds slide on the X/Y plane and keep Z untouched.
        let flat = hold_slide_candidates(Vec3f::new(4.0, 6.0, 0.0), None, false);
        assert!(flat.iter().all(|p| p.z == 0.0));
        assert!(flat.iter().skip(1).any(|p| (p.y - 6.0).abs() > 0.1));

        // Nothing accepted (e.g. open water everywhere): the hold stays where it was.
        assert_eq!(slide_player_hold(desired, None, true, |_| false), None);
    }

    #[test]
    fn dropped_creature_stays_selected_at_its_landing_tile() {
        let id = WorldEntityId(42);
        let terrain = dropped_creature_selection(id, Vec3f::new(12.7, 3.1, -4.2), true);
        assert_eq!(terrain.kind, StableVoxelRefKind::Creature);
        assert_eq!(terrain.stable_id, Some(id));
        assert_eq!(terrain.tile, Some(VoxelTileCoord::new(12, -5)));
        assert_eq!(
            terrain.chunk,
            VoxelChunkCoord::for_tile(16, VoxelTileCoord::new(12, -5))
        );
        let flat = dropped_creature_selection(id, Vec3f::new(3.2, 9.9, 0.0), false);
        assert_eq!(flat.tile, Some(VoxelTileCoord::new(3, 9)));
    }
    #[test]
    fn tile_double_click_rejects_elapsed_and_rewound_time_and_consumes_pairs() {
        let tile = VoxelTileCoord::new(4, 7);
        let hovered = Some(StableVoxelObjectRef {
            kind: StableVoxelRefKind::Tile,
            stable_id: None,
            chunk: VoxelChunkCoord::for_tile(16, tile),
            tile: Some(tile),
        });
        let mut last = None;
        assert!(!should_recenter_camera_on_click(hovered, 2.0, &mut last));
        assert!(!should_recenter_camera_on_click(hovered, 1.9, &mut last));
        assert!(!should_recenter_camera_on_click(hovered, 3.0, &mut last));
        assert!(should_recenter_camera_on_click(hovered, 3.2, &mut last));
        assert!(!should_recenter_camera_on_click(hovered, 3.3, &mut last));
        assert!(!should_recenter_camera_on_click(None, 3.4, &mut last));
        assert!(!should_recenter_camera_on_click(hovered, 3.5, &mut last));
    }

    #[test]
    fn accepted_carry_target_stops_probing_immediately() {
        let desired = Vec3f::new(1.0, 2.0, 3.0);
        let mut attempts = 0;
        assert_eq!(
            slide_player_hold(desired, None, true, |candidate| {
                attempts += 1;
                assert_eq!(candidate, desired);
                true
            }),
            Some(desired)
        );
        assert_eq!(attempts, 1);
    }
    #[test]
    fn slide_uses_world_rejection_and_drops_at_accepted_ground() {
        let mut world = alife_world::HeadlessScenarioBuilder::new(17)
            .food("food", Vec3f::ZERO, 0.5)
            .obstacle("rock", Vec3f::new(4.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let id = world.entity_id("food").unwrap();
        world.begin_player_hold(id).unwrap();
        let current = world.entity(id).unwrap().position;
        let desired = Vec3f::new(4.0, 0.0, 0.0);
        let accepted = slide_player_hold(desired, Some(current), false, |point| {
            world.move_player_hold(point).is_ok()
        })
        .unwrap();
        assert_ne!(accepted, desired);
        assert!(accepted.x < desired.x);
        assert_eq!(world.player_held_object(), Some(id));
        assert_eq!(world.release_player_hold().unwrap(), Some(id));
        assert_eq!(world.entity(id).unwrap().position, accepted);
        assert_eq!(world.player_held_object(), None);
    }
}
