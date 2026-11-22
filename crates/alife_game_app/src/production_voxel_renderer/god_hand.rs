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
    #[cfg(feature = "gpu-runtime")]
    direct_prey_aim: Option<DirectPreyAim>,
    release_until: f64,
    stroke: Option<(WorldEntityId, f64)>,
    pub position: Option<Vec3>,
    last_tile_click: Option<(VoxelTileCoord, f64)>,
    resume_follow_after_drop: bool,
    #[cfg(any(feature = "gpu-runtime", test))]
    reopened_chime: Option<ReopenedChimeGrip>,
}
const STROKE_SECONDS: f64 = 0.42;

/// A stroke starts and settles at the same point on the fur. A press causes
/// exactly one touch; holding the mouse cannot farm repeated care receipts.
fn stroke_offset(start: f64, now: f64, camera: Quat, extent: f32) -> Vec3 {
    let phase = (now - start) / STROKE_SECONDS;
    if !(0.0..1.0).contains(&phase) {
        return Vec3::ZERO;
    }
    let arc = (std::f32::consts::PI * phase as f32).sin();
    let sweep = (std::f32::consts::TAU * phase as f32).sin();
    pointer_scale(extent, false) * arc * (camera * Vec3::X * sweep * 0.22 - Vec3::Y * 0.045)
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
#[cfg(any(feature = "gpu-runtime", test))]
struct ReopenedChimeGrip {
    press: Vec2,
    moved: bool,
    direct: Option<ChimeDirectSeat>,
}

/// Review only. A retaken saved Chime is not ordinary first-party pickup.
#[cfg(any(feature = "gpu-runtime", test))]
const REOPENED_CHIME_COMMIT_REVIEW: bool = super::QUALIA_VILLAGE_REVIEW;

#[cfg(any(feature = "gpu-runtime", test))]
#[derive(Clone, Copy)]
struct ChimeDirectSeat {
    id: WorldEntityId,
    aim: Vec3f,
    raised: Vec3f,
}

#[cfg(any(feature = "gpu-runtime", test))]
struct ChimePointerEnd {
    focused: bool,
    escaped: bool,
    fresh_left_up: bool,
    cursor: Option<Vec2>,
    ground: Option<Vec3f>,
}

#[cfg(any(feature = "gpu-runtime", test))]
enum ReopenedChimeEnd<R> {
    Stock,
    Retained,
    Native(R),
}

#[cfg(any(feature = "gpu-runtime", test))]
fn live_reopened_chime_pose(world: &alife_world::HeadlessWorld) -> Option<(WorldEntityId, Vec3f)> {
    if world.terrain().is_none() || !world.player_holds_original_chime() {
        return None;
    }
    let id = world.player_held_object()?;
    Some((id, world.entity(id)?.position))
}

/// The existing exact native motion request; a denied or unsampled aim cannot
/// authorize a later release at a previously accepted World support.
#[cfg(any(feature = "gpu-runtime", test))]
fn submit_chime_direct_aim<E>(
    hand: &mut HandInteraction,
    aim: Vec3f,
    move_exact: impl FnOnce(Vec3f) -> Result<Vec3f, E>,
) {
    let raised = move_exact(aim).ok();
    if REOPENED_CHIME_COMMIT_REVIEW {
        if let Some(grip) = hand.reopened_chime.as_mut() {
            grip.direct =
                hand.held
                    .zip(raised)
                    .map(|(id, raised)| ChimeDirectSeat { id, aim, raised });
        }
    }
}

/// Reopened release exclusively uses a fresh focused pointer-up, an actual
/// crossed drag threshold, and the same accepted signed aim/raised pose.
/// The closure is the one native World release (and still rechecks its foot).
#[cfg(any(feature = "gpu-runtime", test))]
fn end_reopened_chime_pointer<R>(
    hand: &mut HandInteraction,
    live: Option<(WorldEntityId, Vec3f)>,
    input: ChimePointerEnd,
    release_exact: impl FnOnce() -> R,
) -> ReopenedChimeEnd<R> {
    if !REOPENED_CHIME_COMMIT_REVIEW {
        return ReopenedChimeEnd::Stock;
    }
    let Some(grip) = hand.reopened_chime.as_ref() else {
        return ReopenedChimeEnd::Stock;
    };
    let fits = input.focused
        && !input.escaped
        && input.fresh_left_up
        && input.cursor.is_some_and(|p| p.is_finite())
        && grip.moved
        && grip.direct.is_some_and(|w| {
            hand.held == Some(w.id) && live == Some((w.id, w.raised)) && input.ground == Some(w.aim)
        });
    if fits {
        return ReopenedChimeEnd::Native(release_exact());
    }
    reset_chime_hand(hand);
    hand.last_tile_click = None;
    ReopenedChimeEnd::Retained
}

#[cfg(feature = "gpu-runtime")]
fn keep_reopened_chime_on_menu(hand: &mut HandInteraction) -> bool {
    if !REOPENED_CHIME_COMMIT_REVIEW || hand.reopened_chime.is_none() {
        return false;
    }
    // Menu takes the pointer, not the saved raised World object.
    reset_chime_hand(hand);
    hand.last_tile_click = None;
    true
}

#[cfg(any(feature = "gpu-runtime", test))]
fn focused_left_press(
    focused: bool,
    mouse: &ButtonInput<MouseButton>,
    keyboard: &ButtonInput<KeyCode>,
) -> bool {
    focused
        && mouse.just_pressed(MouseButton::Left)
        && mouse.pressed(MouseButton::Left)
        && !keyboard.just_pressed(KeyCode::Escape)
}

/// Eligibility comes from the same feature-gated World Hand authority. No
/// generic ActivityToy is made selectable; a restored hold is not a new pickup.
#[cfg(any(feature = "gpu-runtime", test))]
pub(super) fn chime_body_pick_id(world: &alife_world::HeadlessWorld) -> Option<WorldEntityId> {
    if world.player_holds_original_chime() {
        return world.player_held_object();
    }
    world.entity_id("player-activity-toy").filter(|&id| {
        world
            .entity(id)
            .is_some_and(|object| object.kind == WorldObjectKind::ActivityToy)
            && world.can_begin_player_hold(id)
    })
}

#[cfg(any(feature = "gpu-runtime", test))]
fn adopt_reopened_chime(
    hand: &mut HandInteraction,
    world: &alife_world::HeadlessWorld,
    hovered: Option<StableVoxelObjectRef>,
    cursor: Option<Vec2>,
    clean_press: bool,
) -> bool {
    if !clean_press || hand.held.is_some() || !world.player_holds_original_chime() {
        return false;
    }
    let Some(id) = world.player_held_object() else {
        return false;
    };
    let Some(press) = cursor.filter(|p| p.is_finite()) else {
        return false;
    };
    if !hovered
        .is_some_and(|hit| hit.kind == StableVoxelRefKind::Resource && hit.stable_id == Some(id))
    {
        return false;
    }
    // Only the App gesture attaches here. World keeps its existing ID,
    // raised pose, last accepted support and pending release authority.
    hand.held = Some(id);
    hand.pressed = None;
    hand.stroke = None;
    hand.resume_follow_after_drop = false;
    hand.reopened_chime = Some(ReopenedChimeGrip {
        press,
        moved: false,
        direct: None,
    });
    true
}

#[cfg(any(feature = "gpu-runtime", test))]
fn allows_held_motion(hand: &mut HandInteraction, cursor: Option<Vec2>) -> bool {
    let Some(grip) = hand.reopened_chime.as_mut() else {
        return true;
    };
    let Some(cursor) = cursor.filter(|p| p.is_finite()) else {
        return false;
    };
    // A press or stationary button hold must not shift the seat toward the ray.
    grip.moved |= cursor.distance_squared(grip.press) >= 4.0 * 4.0;
    grip.moved
}

#[cfg(any(feature = "gpu-runtime", test))]
fn may_release_hand(local_id: Option<WorldEntityId>, world: &alife_world::HeadlessWorld) -> bool {
    let held = world.player_held_object();
    held.is_some() && (!world.player_holds_original_chime() || local_id == held)
}

#[cfg(any(feature = "gpu-runtime", test))]
fn reset_chime_hand(hand: &mut HandInteraction) {
    hand.reopened_chime = None;
    hand.held = None;
    hand.pressed = None;
    hand.resume_follow_after_drop = false;
}

#[cfg(any(feature = "gpu-runtime", test))]
fn detach_denied_reopened_chime(hand: &mut HandInteraction) {
    if hand.reopened_chime.is_some() {
        // Failed release leaves the World hold alive. A later deliberate
        // press may resume it; idle frames cannot retry or steer it.
        reset_chime_hand(hand);
    }
}

#[cfg(feature = "gpu-runtime")]
pub(super) fn clear_chime_hand_on_load(world: &mut World) {
    if !super::QUALIA_VILLAGE_REVIEW {
        return;
    }
    let restored = world
        .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
        .is_some_and(|runtime| runtime.runtime.world().player_holds_original_chime());
    if let Some(mut hand) = world.get_resource_mut::<HandInteraction>() {
        // Called only after the actual runtime/scene commit, not load staging.
        // Identical numeric ID, pose and ray can recur across loaded worlds.
        hand.direct_prey_aim = None;
        if restored || hand.reopened_chime.is_some() {
            // Even the same numeric ID from a committed reload needs a new
            // real press. Do not reset ordinary stock carry on load.
            reset_chime_hand(&mut hand);
        }
    }
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
        // Render layers are per entity, not inherited by a glTF hierarchy.
        commands
            .entity(entity)
            .insert(crate::terrain_lighting::player_guide_layer());
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
#[derive(Clone, Copy)]
struct DirectPreyAim {
    id: WorldEntityId,
    aim: Vec3f,
    held_pose: Vec3f,
}

#[cfg(any(feature = "gpu-runtime", test))]
impl DirectPreyAim {
    fn observe(
        id: WorldEntityId,
        aim: Vec3f,
        offered: Option<Vec3f>,
        held_pose: Vec3f,
    ) -> Option<Self> {
        // A nearby success is a carry preview, never consent to place there.
        (offered == Some(aim)).then_some(Self { id, aim, held_pose })
    }
}

#[cfg(any(feature = "gpu-runtime", test))]
fn terrain_prey_hold(
    local: Option<WorldEntityId>,
    world: &alife_world::HeadlessWorld,
) -> Option<(WorldEntityId, Vec3f)> {
    if !super::QUALIA_VILLAGE_REVIEW {
        return None;
    }
    let id = local?;
    if world.terrain().is_none() || world.player_held_object() != Some(id) {
        return None;
    }
    let body = world.entity(id)?;
    (body.kind == WorldObjectKind::Food
        && !body.consumed
        && body.carried_by.is_none()
        && !body.activity_hold
        && world.autonomous_critter(id).is_some())
    .then_some((id, body.position))
}

#[cfg(any(feature = "gpu-runtime", test))]
fn allows_prey_pointer_setdown(
    held: Option<(WorldEntityId, Vec3f)>,
    prior_direct: Option<DirectPreyAim>,
    pointer_up: bool,
    pointer_ground: Option<Vec3f>,
) -> bool {
    // This gate applies only to an ordinary focused pointer gesture with
    // matching custody. No retry on an idle frame after a refused release.
    let Some((id, pose)) = held else {
        return true;
    };
    pointer_up
        && prior_direct.is_some_and(|direct| {
            direct.id == id && direct.held_pose == pose && Some(direct.aim) == pointer_ground
        })
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
    #[cfg(feature = "gpu-runtime")] mut ux: Option<ResMut<Fvr05ProductionUxStateResource>>,
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
        {
            hand.direct_prey_aim = None;
        }
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
        if hand
            .stroke
            .is_some_and(|(_, start)| now - start >= STROKE_SECONDS)
            || !window.focused
            || keyboard.just_pressed(KeyCode::Escape)
            || hand.pressed.is_some()
        {
            hand.stroke = None;
        }
        let pet_press = window.focused
            && !keyboard.just_pressed(KeyCode::Escape)
            && !mouse.pressed(MouseButton::Left)
            && mouse.just_pressed(MouseButton::Right)
            && runtime.runtime.world().player_held_object().is_none();
        let target = selection
            .hovered
            .filter(|s| pet_press && s.kind == StableVoxelRefKind::Creature)
            .and_then(|s| s.stable_id.map(|id| (s, id)));
        if let (Some((target, id)), Some(ux), Some(frame)) =
            (target, ux.as_deref_mut(), frame.as_deref_mut())
        {
            let outcome = care_dock::dispatch(
                care_dock::Action::Pet,
                Some(target),
                ux,
                Some(&mut runtime.runtime),
                frame,
            );
            if outcome == care_dock::Evidence::Queued {
                selection.selected = Some(target);
                hand.stroke = Some((id, now));
            }
        }
        if hand.held.is_some() && runtime.runtime.world().player_held_object() != hand.held {
            hand.reopened_chime = None;
            hand.held = None;
            hand.direct_prey_aim = None;
            hand.resume_follow_after_drop = false;
            hand.release_until = now + 0.18;
        }
        let cancel = !window.focused
            || keyboard.just_pressed(KeyCode::Escape)
            || !mouse.pressed(MouseButton::Left);
        if !cancel {
            adopt_reopened_chime(
                &mut hand,
                runtime.runtime.world(),
                selection.hovered,
                window.cursor_position(),
                focused_left_press(window.focused, &mouse, &keyboard),
            );
        }
        if cancel {
            let verified_held = if REOPENED_CHIME_COMMIT_REVIEW && hand.reopened_chime.is_some() {
                live_reopened_chime_pose(runtime.runtime.world())
            } else {
                None
            };
            let end = ChimePointerEnd {
                focused: window.focused,
                escaped: keyboard.just_pressed(KeyCode::Escape),
                fresh_left_up: mouse.just_released(MouseButton::Left),
                cursor: window.cursor_position(),
                ground: hand.ground.map(|p| Vec3f::new(p.x, p.y, p.z)),
            };
            let reopened = end_reopened_chime_pointer(&mut hand, verified_held, end, || {
                runtime.runtime.release_player_hold()
            });
            if matches!(&reopened, ReopenedChimeEnd::Retained) {
                return;
            }
            if matches!(&reopened, ReopenedChimeEnd::Native(_))
                || may_release_hand(hand.held, runtime.runtime.world())
            {
                // A deliberate drop (button released in a focused window) keeps the creature in
                // the player's attention, as in Black & White: the HUD, care dock, and speech stay
                // on it at its landing tile, and the camera resumes following it.
                let natural_drop = window.focused && !keyboard.just_pressed(KeyCode::Escape);
                let scoped_prey = terrain_prey_hold(hand.held, runtime.runtime.world());
                let prior_direct = hand.direct_prey_aim.take();
                let pointer_ground = hand.ground.map(|p| Vec3f::new(p.x, p.y, p.z));
                if natural_drop
                    && !allows_prey_pointer_setdown(
                        scoped_prey,
                        prior_direct,
                        mouse.just_released(MouseButton::Left),
                        pointer_ground,
                    )
                {
                    // The approved nearby carry preview is not placement.
                    hand.pressed = None;
                    return;
                }
                let release = match reopened {
                    ReopenedChimeEnd::Native(receipt) => receipt,
                    ReopenedChimeEnd::Stock => runtime.runtime.release_player_hold(),
                    ReopenedChimeEnd::Retained => {
                        unreachable!("reopened detach returned before release")
                    }
                };
                let released = match release {
                    Ok(released) => released,
                    Err(error) => {
                        // The world still owns this hold. Retain local contact and follow
                        // intent so a later successful retry can finish the same gesture.
                        hand.pressed = None;
                        detach_denied_reopened_chime(&mut hand);
                        if let Some(ux) = ux.as_deref_mut() {
                            ux.last_error = Some(error.to_string());
                            ux.last_action = "Hand release failed; hold retained".into();
                        }
                        if let Some(frame) = frame.as_deref_mut() {
                            frame.refresh_world_objects(runtime.runtime.world());
                        }
                        return;
                    }
                };
                hand.held = None;
                hand.reopened_chime = None;
                let resume_follow = std::mem::take(&mut hand.resume_follow_after_drop);
                if let Some(ux) = ux.as_deref_mut() {
                    if ux.last_action == "Hand release failed; hold retained" {
                        ux.last_error = None;
                        ux.last_action = "Hand released".into();
                    }
                }
                if let Some(released) = released {
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
            hand.direct_prey_aim = None;
            if REOPENED_CHIME_COMMIT_REVIEW {
                if let Some(grip) = hand.reopened_chime.as_mut() {
                    // Every new held input sample must prove its own direct seat.
                    grip.direct = None;
                }
            }
            let ground = hand.ground;
            if let Some(p) =
                ground.filter(|_| allows_held_motion(&mut hand, window.cursor_position()))
            {
                let desired = Vec3f::new(p.x, p.y, p.z);
                let terrain_bound = runtime.runtime.world().terrain().is_some();
                let current = hand
                    .held
                    .and_then(|id| runtime.runtime.world().entity(id))
                    .map(|object| object.position);
                if runtime.runtime.world().player_holds_original_chime() {
                    // Keep the native retained support when this exact
                    // ground fails. Don't silently slide the one fixture.
                    submit_chime_direct_aim(&mut hand, desired, |aim| {
                        runtime.runtime.move_player_hold(aim)
                    });
                } else {
                    let offered = slide_player_hold(desired, current, terrain_bound, |candidate| {
                        runtime.runtime.move_player_hold(candidate).is_ok()
                    });
                    hand.direct_prey_aim = terrain_prey_hold(hand.held, runtime.runtime.world())
                        .and_then(|(id, pose)| DirectPreyAim::observe(id, desired, offered, pose));
                }
            }
            if let Some(frame) = frame.as_deref_mut() {
                frame.refresh_world_objects(runtime.runtime.world());
            }
        }
        // The pointing fingertip belongs on the picked body, not one metre above
        // the terrain behind it. A committed hold or stroke still takes priority.
        let contact = contact_id(&hand)
            .or_else(|| hand.stroke.map(|(id, _)| id))
            .or_else(|| selection.hovered.and_then(|target| target.stable_id));
        if let Some(id) = contact {
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
            let stroke = hand.stroke.map_or(Vec3::ZERO, |(_, start)| {
                stroke_offset(start, time.elapsed_secs_f64(), camera.rotation, extent)
            });
            *transform = pose_transform(camera.rotation, contact + stroke, extent, state);
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
        #[cfg(feature = "gpu-runtime")]
        {
            hand.direct_prey_aim = None;
            if keep_reopened_chime_on_menu(&mut hand) {
                return;
            }
        }
    }
    #[cfg(feature = "gpu-runtime")]
    let local_hold = world
        .get_resource::<HandInteraction>()
        .and_then(|hand| hand.held);
    #[cfg(feature = "gpu-runtime")]
    if let Some(mut runtime) =
        world.get_non_send_resource_mut::<ProductionGpuBrainRuntimeResource>()
    {
        if may_release_hand(local_hold, runtime.runtime.world()) {
            if let Err(error) = runtime.runtime.release_player_hold() {
                if let Some(mut hand) = world.get_resource_mut::<HandInteraction>() {
                    detach_denied_reopened_chime(&mut hand);
                }
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
        #[cfg(feature = "gpu-runtime")]
        {
            hand.reopened_chime = None;
        }
        hand.resume_follow_after_drop = false;
        hand.last_tile_click = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(all(feature = "gpu-runtime", feature = "qualia-village-review"))]
    #[test]
    fn reopened_chime_real_follow_shift_retains_unless_the_new_ray_is_explicitly_aimed() {
        use alife_core::{
            ActionCommand, ActionKind, ActionTarget, Confidence, DurationTicks, Intensity,
        };
        use alife_world::{HeadlessScenarioBuilder, HeadlessWorld};
        use bevy::{
            camera::{CameraProjection, OrthographicProjection, RenderTargetInfo, ScalingMode},
            math::UVec2,
            window::WindowResolution,
        };

        const WALKER: OrganismId = OrganismId(11);
        const MODE: Fvr03ProductionVoxelCameraMode =
            Fvr03ProductionVoxelCameraMode::OrthographicIsometric;
        const EXTENT: f32 = 9.8;

        fn p(v: Vec3f) -> Vec3 {
            Vec3::new(v.x, v.y, v.z)
        }
        fn ground_at(world: &HeadlessWorld, x: f32, z: f32) -> Vec3f {
            Vec3f::new(
                x,
                world.terrain().unwrap().surface().height(x, z).unwrap(),
                z,
            )
        }
        fn ray_ground(world: &HeadlessWorld, ray: &bevy::math::Ray3d) -> Vec3f {
            let o = ray.origin;
            let d = *ray.direction;
            world
                .terrain()
                .unwrap()
                .surface()
                .ray_hit(
                    Vec3f::new(o.x, o.y, o.z),
                    Vec3f::new(d.x, d.y, d.z),
                    10000.0,
                )
                .expect("signed Follow ray must hit the actual island")
        }

        assert!(REOPENED_CHIME_COMMIT_REVIEW);
        let mut world = HeadlessScenarioBuilder::new(1643)
            .agent("signed-follow-target", WALKER, Vec3f::ZERO)
            .build()
            .unwrap();
        world
            .enable_terrain_for_new_game(
                alife_world::island_terrain(),
                Vec3f::new(81.5, 0.0, 251.5),
            )
            .unwrap();
        let walker = world.entity_id("signed-follow-target").unwrap();
        // Keep the followed resident and signed destination fixed. Start far
        // enough south that the native grounded carry sweep clears that body.
        let old_chime = ground_at(&world, 80.0, 248.5);
        let chime = world
            .spawn_toy("player-activity-toy", old_chime, false)
            .expect("signed original Chime source is supported");
        assert_eq!(world.entity(chime).unwrap().position, old_chime);
        world
            .begin_player_hold(chime)
            .expect("native Chime lift at the source");
        let before_body = world.entity(walker).unwrap().position;
        let before_eye = GlobalTransform::from(production_follow_camera_transform(
            MODE,
            EXTENT,
            p(before_body),
        ));

        let mut projection = OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical {
                viewport_height: EXTENT,
            },
            scale: 1.0,
            near: -200.0,
            far: 500.0,
            ..OrthographicProjection::default_3d()
        };
        projection.update(1280.0, 720.0);
        let mut camera = Camera::default();
        camera.computed.clip_from_view = projection.get_clip_from_view();
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(1280, 720),
            scale_factor: 1.0,
        });
        let mut window = Window {
            focused: true,
            resolution: WindowResolution::new(1280, 720),
            ..default()
        };
        let mut surface = creature_grounding::RenderedTerrainSurface::from_meshes(
            std::iter::empty::<&Mesh>(),
            1.0,
        );
        surface.set_surface(Some(world.terrain().unwrap().surface().clone()));
        let chime_center = live_food_projection::care_object_center(
            world.entity(chime).unwrap().position,
            true,
            &surface,
        );
        let press = camera.world_to_viewport(&before_eye, chime_center).unwrap();
        window.set_cursor_position(Some(press));
        let press = window.cursor_position().unwrap();
        let press_ray = camera.viewport_to_world(&before_eye, press).unwrap();
        let frame =
            LiveBrainPresentationFrame::try_new(Vec::new(), world.tick(), world.object_snapshots())
                .unwrap();
        let (_, id, kind, _) = pick_live_object_body_with_chime(
            &frame,
            true,
            &surface,
            &press_ray,
            p(ray_ground(&world, &press_ray)),
            chime_body_pick_id(&world),
        )
        .expect("ranked Follow press must hit the raised original Chime");
        assert_eq!((id, kind), (chime, WorldObjectKind::ActivityToy));
        let tile = VoxelTileCoord::new(80, 248);
        let hit = StableVoxelObjectRef {
            kind: StableVoxelRefKind::Resource,
            stable_id: Some(id),
            chunk: VoxelChunkCoord::for_tile(16, tile),
            tile: Some(tile),
        };
        let aim = p(ground_at(&world, 83.5, 250.5));
        let seat_pixel = camera.world_to_viewport(&before_eye, aim).unwrap();
        window.set_cursor_position(Some(seat_pixel));
        let seat_pixel = window.cursor_position().unwrap();
        assert!(press.distance(seat_pixel) > 16.0);
        let held_ray = camera.viewport_to_world(&before_eye, seat_pixel).unwrap();
        let direct_ground = ray_ground(&world, &held_ray);
        assert!(p(direct_ground).distance(aim) < 0.002);
        let direct_seat = ground_at(&world, direct_ground.x, direct_ground.z);

        // The exact original movement command and terrain authority move the followed
        // body; the camera is the same production follow transform at both positions.
        let command = ActionCommand::structured(
            WALKER,
            ActionKind::Move.canonical_id(),
            ActionKind::Move,
            ActionTarget::new(
                None,
                Some(Vec3f::new(
                    before_body.x + 1.0,
                    before_body.y,
                    before_body.z,
                )),
            ),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(0.9).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap();
        let mut movement_probe = world.clone();
        let step = movement_probe.apply_command(&command).unwrap();
        assert!(step.execution.succeeded);
        let after_body = movement_probe.entity(walker).unwrap().position;
        assert!(((after_body.x - before_body.x) - 0.10).abs() < 0.0001);
        let after_eye = GlobalTransform::from(production_follow_camera_transform(
            MODE,
            EXTENT,
            p(after_body),
        ));
        let up_ray = camera.viewport_to_world(&after_eye, seat_pixel).unwrap();
        let shifted_ground = ray_ground(&world, &up_ray);
        let drift = p(shifted_ground).distance(p(direct_ground));
        assert!((0.06..0.25).contains(&drift));
        // Geometry control omitted the follow-tolerance proposal: the moved
        // eye genuinely has a different native ground while logical cursor stays.
        assert_ne!(direct_ground, shifted_ground);
        let accept_native_aim =
            |hand: &mut HandInteraction, world: &mut HeadlessWorld, ground: Vec3f, phase: &str| {
                submit_chime_direct_aim(hand, ground, |aim| {
                    let native = world.move_player_hold(aim);
                    assert!(
                        native.is_ok(),
                        "{phase}: native Chime sweep refused {aim:?}: {native:?}"
                    );
                    native
                });
                let raised = ground_at(world, ground.x, ground.z);
                let raised = Vec3f::new(raised.x, raised.y + 1.3, raised.z);
                assert_eq!(
                    live_reopened_chime_pose(world),
                    Some((chime, raised)),
                    "{phase}"
                );
                assert_eq!(
                    hand.reopened_chime
                        .as_ref()
                        .and_then(|grip| grip.direct)
                        .map(|stamp| (stamp.id, stamp.aim, stamp.raised)),
                    Some((chime, ground, raised)),
                    "{phase}: App must stamp that exact native acceptance",
                );
            };
        for fresh_held_sample in [false, true] {
            let mut world = world.clone();
            let mut hand = HandInteraction::default();
            let mut buttons = ButtonInput::<MouseButton>::default();
            let keys = ButtonInput::<KeyCode>::default();
            buttons.press(MouseButton::Left);
            assert!(adopt_reopened_chime(
                &mut hand,
                &world,
                Some(hit),
                Some(press),
                focused_left_press(true, &buttons, &keys),
            ));
            buttons.clear_just_pressed(MouseButton::Left);
            assert!(allows_held_motion(&mut hand, Some(seat_pixel)));
            accept_native_aim(&mut hand, &mut world, direct_ground, "initial held aim");
            // The exact native destination was legal; absent the App guard
            // old release would really commit, rather than fail on terrain.
            let mut old_release = world.clone();
            assert_eq!(old_release.release_player_hold().unwrap(), Some(chime));
            assert_eq!(old_release.entity(chime).unwrap().position, direct_seat);

            assert!(world.apply_command(&command).unwrap().execution.succeeded);
            assert_eq!(world.entity(walker).unwrap().position, after_body);
            if fresh_held_sample {
                // A still-held input frame genuinely resamples and accepts
                // that world point; never update the seat on button-up.
                assert!(buttons.pressed(MouseButton::Left));
                accept_native_aim(
                    &mut hand,
                    &mut world,
                    shifted_ground,
                    "fresh aim after Follow move",
                );
            }
            buttons.release(MouseButton::Left);
            let live = live_reopened_chime_pose(&world);
            let before = world.canonical_signature_digest().unwrap();
            let mut calls = 0;
            let decision = end_reopened_chime_pointer(
                &mut hand,
                live,
                ChimePointerEnd {
                    focused: true,
                    escaped: false,
                    fresh_left_up: buttons.just_released(MouseButton::Left),
                    cursor: Some(seat_pixel),
                    ground: Some(shifted_ground),
                },
                || {
                    calls += 1;
                    world.release_player_hold()
                },
            );
            match (fresh_held_sample, decision) {
                (true, ReopenedChimeEnd::Native(Ok(Some(id)))) => {
                    assert_eq!(id, chime);
                    assert_eq!(calls, 1);
                    assert!(world.player_held_object().is_none());
                    assert_eq!(
                        world.entity(chime).unwrap().position,
                        ground_at(&world, shifted_ground.x, shifted_ground.z)
                    );
                }
                (false, ReopenedChimeEnd::Retained) => {
                    assert_eq!(calls, 0);
                    assert!(hand.held.is_none());
                    assert_eq!(live_reopened_chime_pose(&world), live);
                    assert_eq!(world.canonical_signature_digest().unwrap(), before);
                }
                _ => panic!("unexpected production termination for follow motion"),
            }
        }
    }

    #[cfg(not(feature = "qualia-village-review"))]
    #[test]
    fn default_world_and_app_keep_stock_chime_ineligible_and_prey_setdown() {
        // Same signed boulder/control as the genuine activated case; no GPU.
        const CURSOR_X: f32 = f32::from_bits(0x42932d66);
        const START_X: f32 = f32::from_bits(0x4295_2847);
        const Z: f32 = f32::from_bits(0x4345_e09a);
        let terrain = alife_world::island_terrain();
        let floor = |x, z| Vec3f::new(x, terrain.surface().height(x, z).unwrap(), z);
        let cursor = floor(CURSOR_X, Z);
        let fallback = floor(CURSOR_X + 0.45, Z);
        assert!(!terrain.walkable(cursor.x, cursor.z) && terrain.walkable(fallback.x, fallback.z));
        let mut world = alife_world::HeadlessWorld::new(17);
        world
            .enable_terrain_for_new_game(terrain.clone(), Vec3f::ZERO)
            .unwrap();
        let chime = world
            .spawn_toy("player-activity-toy", floor(80.0, 251.5), false)
            .unwrap();
        assert!(chime_body_pick_id(&world).is_none());
        assert!(world.begin_player_hold(chime).is_err());
        let prey = world
            .editor_spawn_object(alife_world::WorldEditorSpawnSpec {
                label: "signed-default-prey".into(),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position: floor(START_X, Z),
                nutrition: 0.34,
                hazard_pain: 0.0,
                radius: 0.34,
                token_id: None,
            })
            .unwrap();
        assert_eq!(
            world.tend_orchard(VoxelTileCoord::new(79, 250)).unwrap(),
            alife_world::orchard::OrchardReceipt::Sown
        );
        assert!(world
            .movable_orchard_planter(VoxelTileCoord::new(79, 250))
            .is_none());
        assert!(world.orchard_court_care().is_none());
        assert!(world.orchard_court_home(chime).is_none());
        world
            .enable_autonomous_critter(prey, 17, 0.040, 4.0)
            .unwrap();
        world.begin_player_hold(prey).unwrap();
        let tick = world.tick();
        let held_pos = world.entity(prey).unwrap().position;
        let offered = slide_player_hold(cursor, Some(held_pos), true, |candidate| {
            world.move_player_hold(candidate).is_ok()
        });
        let stock_slide = Vec3f::new(fallback.x, cursor.y, Z);
        assert_eq!(offered, Some(stock_slide));
        let scoped = terrain_prey_hold(Some(prey), &world);
        assert!(
            scoped.is_none(),
            "default cannot activate the special direct-prey gate"
        );
        assert!(may_release_hand(Some(prey), &world));
        assert!(allows_prey_pointer_setdown(
            scoped,
            None,
            true,
            Some(cursor)
        ));
        assert_eq!(world.release_player_hold().unwrap(), Some(prey));
        assert_eq!(world.entity(prey).unwrap().position, fallback);
        assert_eq!(world.tick(), tick);
    }

    #[cfg(all(feature = "gpu-runtime", feature = "qualia-village-review"))]
    #[test]
    fn committed_load_focus_clears_prey_intent_even_when_id_pose_and_aim_repeat() {
        let terrain = alife_world::island_terrain();
        let x = 84.0;
        let z = 251.5;
        let aim = Vec3f::new(x, terrain.surface().height(x, z).unwrap(), z);
        let fixture = |seed| {
            let mut world = alife_world::HeadlessWorld::new(seed);
            world
                .enable_terrain_for_new_game(terrain.clone(), Vec3f::ZERO)
                .unwrap();
            let prey = world
                .spawn_player_food(aim, alife_world::FoodVariety::Fruit)
                .unwrap();
            world
                .enable_autonomous_critter(prey, seed, 0.04, 4.0)
                .unwrap();
            world.begin_player_hold(prey).unwrap();
            let scoped = terrain_prey_hold(Some(prey), &world).unwrap();
            (world, scoped)
        };
        let (previous_world, old_local) = fixture(17);
        let (new_world, same_local) = fixture(19);
        assert_ne!(previous_world.seed(), new_world.seed());
        assert_eq!(
            old_local, same_local,
            "numeric ID and raised pose genuinely repeat"
        );
        let (id, raised) = old_local;
        let stale = DirectPreyAim::observe(id, aim, Some(aim), raised);
        assert!(
            allows_prey_pointer_setdown(Some(same_local), stale, true, Some(aim)),
            "without committed-load invalidation the equal sample would appear current"
        );

        // Call the actual successful-load focus hook with only its local Bevy
        // resources. This is not a runtime/GPU load or a readiness simulation.
        let mut app_world = World::new();
        app_world.insert_resource(HandInteraction {
            held: Some(id),
            direct_prey_aim: stale,
            ..Default::default()
        });
        super::super::clear_production_load_focus(&mut app_world);
        let hand = app_world.resource::<HandInteraction>();
        assert_eq!(
            hand.held,
            Some(id),
            "ordinary local prey carry is not re-adopted"
        );
        assert!(hand.direct_prey_aim.is_none());
        assert!(!allows_prey_pointer_setdown(
            Some(same_local),
            hand.direct_prey_aim,
            true,
            Some(aim)
        ));
        assert_eq!(new_world.player_held_object(), Some(id));
        assert_eq!(new_world.entity(id).unwrap().position, raised);
    }

    #[cfg(feature = "qualia-village-review")]
    #[test]
    fn reopened_chime_requires_a_new_real_press_then_drag_and_keeps_the_world_hold() {
        use alife_world::{AssetManifest, PortableSaveFile, RuntimeConfig};

        let mut original = alife_world::HeadlessWorld::new(1643);
        original.enable_island_for_new_game().unwrap();
        let ground = |x, z| {
            let terrain = alife_world::island_terrain();
            Vec3f::new(x, terrain.surface().height(x, z).unwrap(), z)
        };
        let first = ground(80.0, 251.5);
        let block = ground(81.5, 251.5);
        let ball = original
            .spawn_toy("player-plaything", ground(78.5, 250.0), true)
            .unwrap();
        let chime = original
            .spawn_toy("player-activity-toy", first, false)
            .unwrap();
        assert_eq!(chime_body_pick_id(&original), Some(chime));
        original.begin_player_hold(chime).unwrap();
        let raised = original.move_player_hold(block).unwrap();
        let bed = VoxelTileCoord::new(81, 251);
        original.tend_orchard(bed).unwrap();

        let portable = PortableSaveFile::from_headless_world(
            "chime-reopen-app-handoff",
            &original,
            RuntimeConfig::deterministic_default(
                original.seed(),
                alife_core::BrainScaleTier::Nano512,
            ),
            AssetManifest::empty(),
            vec![],
        )
        .unwrap();
        let mut saved = PortableSaveFile::from_json_str(&portable.to_json_string_pretty().unwrap())
            .unwrap()
            .restore_headless_world()
            .unwrap();
        let sig = saved.canonical_signature_digest().unwrap();
        assert_eq!(chime_body_pick_id(&saved), Some(chime));
        assert!(
            !saved.can_begin_player_hold(chime),
            "retargeting must not begin a second hold"
        );

        let hit = |kind, id| {
            let tile = VoxelTileCoord::new(81, 251);
            Some(StableVoxelObjectRef {
                kind,
                stable_id: Some(id),
                chunk: VoxelChunkCoord::for_tile(16, tile),
                tile: Some(tile),
            })
        };
        let real_ref = hit(StableVoxelRefKind::Resource, chime);
        let cursor = Some(Vec2::new(415.0, 290.0));
        let mut hand = HandInteraction::default();
        let mut buttons = ButtonInput::<MouseButton>::default();
        let mut keys = ButtonInput::<KeyCode>::default();
        assert!(
            !may_release_hand(hand.held, &saved),
            "unclaimed reopen is idle"
        );
        assert!(!focused_left_press(true, &buttons, &keys));
        buttons.press(MouseButton::Left);
        assert!(!focused_left_press(false, &buttons, &keys));
        keys.press(KeyCode::Escape);
        assert!(!focused_left_press(true, &buttons, &keys));
        keys.clear_just_pressed(KeyCode::Escape);
        let press = focused_left_press(true, &buttons, &keys);
        assert!(press);
        assert!(!adopt_reopened_chime(
            &mut hand, &saved, real_ref, None, press
        ));
        assert!(!adopt_reopened_chime(
            &mut hand,
            &saved,
            hit(StableVoxelRefKind::Resource, ball),
            cursor,
            press
        ));
        assert!(!adopt_reopened_chime(
            &mut hand,
            &saved,
            hit(StableVoxelRefKind::Creature, chime),
            cursor,
            press
        ));
        hand.held = Some(ball);
        assert!(!may_release_hand(hand.held, &saved));
        assert!(!adopt_reopened_chime(
            &mut hand, &saved, real_ref, cursor, press
        ));
        reset_chime_hand(&mut hand);
        assert!(adopt_reopened_chime(
            &mut hand, &saved, real_ref, cursor, press
        ));
        assert_eq!(hand.held, Some(chime));
        assert!(!allows_held_motion(&mut hand, cursor));
        assert!(!allows_held_motion(
            &mut hand,
            Some(Vec2::new(417.0, 291.0))
        ));
        assert!(!allows_held_motion(&mut hand, None));
        assert_eq!(saved.canonical_signature_digest().unwrap(), sig);
        assert_eq!(saved.entity(chime).unwrap().position, raised);

        buttons.release(MouseButton::Left);
        assert!(may_release_hand(hand.held, &saved));
        assert!(saved.release_player_hold().is_err());
        detach_denied_reopened_chime(&mut hand);
        assert_eq!(hand.held, None);
        assert!(!may_release_hand(hand.held, &saved));
        assert!(!adopt_reopened_chime(
            &mut hand,
            &saved,
            real_ref,
            cursor,
            focused_left_press(true, &buttons, &keys)
        ));
        assert_eq!(saved.canonical_signature_digest().unwrap(), sig);

        buttons.clear();
        buttons.press(MouseButton::Left);
        assert!(adopt_reopened_chime(
            &mut hand,
            &saved,
            real_ref,
            cursor,
            focused_left_press(true, &buttons, &keys)
        ));
        buttons.clear_just_pressed(MouseButton::Left);
        assert!(!focused_left_press(true, &buttons, &keys));
        assert!(allows_held_motion(&mut hand, Some(Vec2::new(431.0, 292.0))));
        // The production branch submits this one chosen ray to World. Do not
        // pretend this CPU proof ran the actual window/event system.
        saved.move_player_hold(first).unwrap();
        assert!(may_release_hand(hand.held, &saved));
        assert_eq!(saved.release_player_hold().unwrap(), Some(chime));
        reset_chime_hand(&mut hand);
        assert_eq!(saved.entity(chime).unwrap().position, first);
        assert_eq!(
            saved.orchard_gesture(bed),
            Some(alife_world::orchard::OrchardGesture::Watch)
        );
        assert!(saved.last_chime_strike().is_none());
        assert!(!may_release_hand(hand.held, &saved));
    }

    #[cfg(all(feature = "gpu-runtime", feature = "qualia-village-review"))]
    #[test]
    fn reopened_chime_keeps_legal_old_support_on_cancel_and_commits_one_real_intentional_drop() {
        use alife_world::{AssetManifest, PortableSaveFile, RuntimeConfig};

        assert!(
            REOPENED_CHIME_COMMIT_REVIEW,
            "use only the separate review opt-in"
        );
        let mut base = alife_world::HeadlessWorld::new(1643);
        base.enable_island_for_new_game().unwrap();
        let ground = |world: &alife_world::HeadlessWorld, x, z| {
            Vec3f::new(
                x,
                world.terrain().unwrap().surface().height(x, z).unwrap(),
                z,
            )
        };
        let old = ground(&base, 80.0, 251.5);
        let target = ground(&base, 83.5, 250.5);
        let other_aim = ground(&base, 83.6, 250.5);
        let chime = base.spawn_toy("player-activity-toy", old, false).unwrap();
        base.begin_player_hold(chime).unwrap();
        let durable = PortableSaveFile::from_headless_world(
            "reopened-legal-chime-seat",
            &base,
            RuntimeConfig::deterministic_default(base.seed(), alife_core::BrainScaleTier::Nano512),
            AssetManifest::empty(),
            vec![],
        )
        .unwrap()
        .to_json_string_pretty()
        .unwrap();
        let reopen = || {
            PortableSaveFile::from_json_str(&durable)
                .unwrap()
                .restore_headless_world()
                .unwrap()
        };
        // The old support really is legal and unobstructed. No blocking planted
        // material can make either App guard appear to work by accident.
        let mut world_control = reopen();
        assert_eq!(world_control.release_player_hold().unwrap(), Some(chime));
        assert_eq!(world_control.entity(chime).unwrap().position, old);

        let initially_pressed = Vec2::new(415.0, 291.0);
        let dragged = Vec2::new(431.0, 291.0);
        let fresh = || {
            let world = reopen();
            let mut hand = HandInteraction::default();
            let mut buttons = ButtonInput::<MouseButton>::default();
            let keys = ButtonInput::<KeyCode>::default();
            let id = chime_body_pick_id(&world).unwrap();
            assert_eq!(id, chime);
            let tile = VoxelTileCoord::new(old.x.floor() as i32, old.z.floor() as i32);
            let hit = StableVoxelObjectRef {
                kind: StableVoxelRefKind::Resource,
                stable_id: Some(id),
                chunk: VoxelChunkCoord::for_tile(16, tile),
                tile: Some(tile),
            };
            buttons.press(MouseButton::Left);
            assert!(adopt_reopened_chime(
                &mut hand,
                &world,
                Some(hit),
                Some(initially_pressed),
                focused_left_press(true, &buttons, &keys),
            ));
            buttons.clear_just_pressed(MouseButton::Left);
            (world, hand, buttons)
        };

        // Stationary up, focus loss after a genuine move, Escape after a genuine
        // move, and a changed pointer aim each exercise the shared production
        // termination/real native release closure. None may call World release.
        for (moved, focused, escaped, pointer_up, wrong_current_aim) in [
            (false, true, false, true, false),
            (true, false, false, false, false),
            (true, true, true, true, false),
            (true, true, false, true, true),
        ] {
            let (mut world, mut hand, mut mouse) = fresh();
            if moved {
                assert!(allows_held_motion(&mut hand, Some(dragged)));
                submit_chime_direct_aim(&mut hand, target, |aim| world.move_player_hold(aim));
                assert!(hand.reopened_chime.as_ref().unwrap().direct.is_some());
            } else {
                assert!(!allows_held_motion(&mut hand, Some(initially_pressed)));
            }
            if pointer_up {
                mouse.release(MouseButton::Left);
            }
            let live = live_reopened_chime_pose(&world).unwrap();
            let frozen = world.canonical_signature_digest().unwrap();
            let mut called = false;
            let action = end_reopened_chime_pointer(
                &mut hand,
                Some(live),
                ChimePointerEnd {
                    focused,
                    escaped,
                    fresh_left_up: mouse.just_released(MouseButton::Left),
                    cursor: Some(if moved { dragged } else { initially_pressed }),
                    ground: Some(if wrong_current_aim {
                        other_aim
                    } else if moved {
                        target
                    } else {
                        old
                    }),
                },
                || {
                    called = true;
                    world.release_player_hold()
                },
            );
            assert!(matches!(action, ReopenedChimeEnd::Retained));
            assert!(!called);
            assert_eq!(hand.held, None);
            assert!(hand.reopened_chime.is_none());
            assert_eq!(live_reopened_chime_pose(&world), Some(live));
            assert_eq!(world.canonical_signature_digest().unwrap(), frozen);
        }

        // Test the exact menu cancellation system. The opt-in return occurs
        // before any live runtime access, so the actual old World hold remains
        // its legal self without initializing a GPU backend.
        let (world, mut hand, _) = fresh();
        let frozen = world.canonical_signature_digest().unwrap();
        assert_eq!(hand.held, Some(chime));
        hand.last_tile_click = Some((VoxelTileCoord::new(80, 251), 0.25));
        let mut ecs = World::new();
        ecs.insert_resource(hand);
        cancel_for_menu(&mut ecs);
        let local = ecs.resource::<HandInteraction>();
        assert!(
            local.held.is_none()
                && local.reopened_chime.is_none()
                && local.last_tile_click.is_none()
        );
        assert_eq!(world.player_held_object(), Some(chime));
        assert_eq!(world.canonical_signature_digest().unwrap(), frozen);

        // A denied exact World aim leaves the old legal saved seat in custody.
        let (mut world, mut hand, mut mouse) = fresh();
        world
            .spawn_toy("reopened-denied-aim-ball", target, true)
            .unwrap();
        assert!(allows_held_motion(&mut hand, Some(dragged)));
        submit_chime_direct_aim(&mut hand, target, |aim| world.move_player_hold(aim));
        assert!(hand.reopened_chime.as_ref().unwrap().direct.is_none());
        mouse.release(MouseButton::Left);
        let live = live_reopened_chime_pose(&world);
        let before = world.canonical_signature_digest().unwrap();
        let result = end_reopened_chime_pointer(
            &mut hand,
            live,
            ChimePointerEnd {
                focused: true,
                escaped: false,
                fresh_left_up: mouse.just_released(MouseButton::Left),
                cursor: Some(dragged),
                ground: Some(target),
            },
            || world.release_player_hold(),
        );
        assert!(matches!(result, ReopenedChimeEnd::Retained));
        assert_eq!(world.canonical_signature_digest().unwrap(), before);

        // Only the full real directed gesture invokes the same native release.
        let (mut world, mut hand, mut mouse) = fresh();
        assert!(allows_held_motion(&mut hand, Some(dragged)));
        submit_chime_direct_aim(&mut hand, target, |aim| world.move_player_hold(aim));
        let live = live_reopened_chime_pose(&world);
        assert!(hand.reopened_chime.as_ref().unwrap().direct.is_some());
        mouse.release(MouseButton::Left);
        let mut count = 0;
        let result = end_reopened_chime_pointer(
            &mut hand,
            live,
            ChimePointerEnd {
                focused: true,
                escaped: false,
                fresh_left_up: mouse.just_released(MouseButton::Left),
                cursor: Some(dragged),
                ground: Some(target),
            },
            || {
                count += 1;
                world.release_player_hold()
            },
        );
        let ReopenedChimeEnd::Native(receipt) = result else {
            panic!("intentional drop did not reach World");
        };
        assert_eq!(receipt.unwrap(), Some(chime));
        assert_eq!(count, 1);
        assert_eq!(world.player_held_object(), None);
        assert_eq!(world.entity(chime).unwrap().position, target);
    }

    #[test]
    fn stroke_makes_one_camera_relative_arc_and_settles_on_the_fur() {
        let camera = Quat::from_rotation_y(1.0);
        let start = 2.0;
        assert_eq!(stroke_offset(start, start - 0.1, camera, 1.0), Vec3::ZERO);
        assert!(stroke_offset(start, start, camera, 1.0).length() < 1e-6);
        assert!(stroke_offset(start, start + STROKE_SECONDS * 0.5, camera, 1.0).length() > 0.001);
        assert_eq!(
            stroke_offset(start, start + STROKE_SECONDS + 0.01, camera, 1.0),
            Vec3::ZERO
        );
    }

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
    #[cfg(feature = "qualia-village-review")]
    #[test]
    fn registered_prey_release_requires_a_current_same_body_direct_acceptance() {
        let id = WorldEntityId(41);
        let aim = Vec3f::new(10.0, 2.0, 12.0);
        let raised = Vec3f::new(10.0, 3.3, 12.0);
        let held = Some((id, raised));
        let proof = DirectPreyAim::observe(id, aim, Some(aim), raised);
        assert!(allows_prey_pointer_setdown(held, proof, true, Some(aim)));
        // Identical sample on an idle frame cannot release a refused hold.
        assert!(!allows_prey_pointer_setdown(held, proof, false, Some(aim)));
        assert!(!allows_prey_pointer_setdown(held, proof, true, None));
        let different_aim = Vec3f::new(10.1, 2.0, 12.0);
        assert!(!allows_prey_pointer_setdown(
            held,
            proof,
            true,
            Some(different_aim)
        ));
        assert!(!allows_prey_pointer_setdown(
            Some((WorldEntityId(42), raised)),
            proof,
            true,
            Some(aim),
        ));
        assert!(!allows_prey_pointer_setdown(
            Some((id, Vec3f::new(10.0, 3.4, 12.0))),
            proof,
            true,
            Some(aim),
        ));
        let alternate = DirectPreyAim::observe(id, aim, Some(different_aim), raised);
        assert!(!allows_prey_pointer_setdown(
            held,
            alternate,
            true,
            Some(aim)
        ));
        let rejected = DirectPreyAim::observe(id, aim, None, raised);
        assert!(!allows_prey_pointer_setdown(
            held,
            rejected,
            true,
            Some(aim)
        ));
        // A state not covered by this guard follows the existing policy.
        assert!(allows_prey_pointer_setdown(None, None, false, None));
    }

    #[cfg(feature = "qualia-village-review")]
    #[test]
    fn signed_registered_prey_fallback_keeps_custody_until_direct_reaim() {
        // The cursor is only 10 cm outside the signed baked rock. Even
        // stock World rejects it. This test requires no prior World patch.
        const CURSOR_X: f32 = f32::from_bits(0x42932d66);
        const START_X: f32 = f32::from_bits(0x4295_2847);
        const Z: f32 = f32::from_bits(0x4345_e09a);
        let terrain = alife_world::island_terrain();
        let floor = |x| Vec3f::new(x, terrain.surface().height(x, Z).unwrap(), Z);
        let cursor = floor(CURSOR_X);
        let start = floor(START_X);
        let alternate_ground = floor(CURSOR_X + 0.45);
        assert!(!terrain.walkable(cursor.x, cursor.z));
        assert!(terrain.walkable(alternate_ground.x, alternate_ground.z));
        let mut world = alife_world::HeadlessWorld::new(17);
        world
            .enable_terrain_for_new_game(terrain.clone(), Vec3f::ZERO)
            .unwrap();
        let id = world
            .editor_spawn_object(alife_world::WorldEditorSpawnSpec {
                label: "signed-fallback-prey".into(),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position: start,
                nutrition: 0.34,
                hazard_pain: 0.0,
                radius: 0.34,
                token_id: None,
            })
            .unwrap();
        world.begin_player_hold(id).unwrap();
        assert_eq!(terrain_prey_hold(Some(id), &world), None); // Ordinary food is stock.
        world.release_player_hold().unwrap();
        world.enable_autonomous_critter(id, 17, 0.040, 4.0).unwrap();
        world.begin_player_hold(id).unwrap();
        let tick = world.tick();
        let before = terrain_prey_hold(Some(id), &world).unwrap().1;
        let snapshots = world.object_snapshots();
        let mut attempts = Vec::new();
        let offered = slide_player_hold(cursor, Some(before), true, |point| {
            let ok = world.move_player_hold(point).is_ok();
            attempts.push((point, ok));
            if !ok {
                assert_eq!(world.object_snapshots(), snapshots);
            }
            ok
        });
        let alternate_proposal = Vec3f::new(alternate_ground.x, cursor.y, Z);
        assert_eq!(attempts, [(cursor, false), (alternate_proposal, true)]);
        assert_eq!(offered, Some(alternate_proposal));
        let landed_while_held = terrain_prey_hold(Some(id), &world);
        let pose = landed_while_held.unwrap().1;
        let preview = DirectPreyAim::observe(id, cursor, offered, pose);
        // These two decisions precede the one physical release call: an
        // invalid pointer-up and its following idle refresh retain custody.
        assert!(!allows_prey_pointer_setdown(
            landed_while_held,
            preview,
            true,
            Some(cursor)
        ));
        assert!(!allows_prey_pointer_setdown(
            landed_while_held,
            preview,
            false,
            Some(alternate_ground)
        ));
        assert_eq!(world.player_held_object(), Some(id));
        assert_eq!(world.entity(id).unwrap().position, pose);
        assert_eq!(world.tick(), tick);
        // A new drag and explicit re-aim at actual open ground may release
        // that same World ID. No substitute prey or auto-placement.
        let direct = slide_player_hold(alternate_ground, Some(pose), true, |point| {
            world.move_player_hold(point).is_ok()
        });
        let held = terrain_prey_hold(Some(id), &world);
        let proof = DirectPreyAim::observe(id, alternate_ground, direct, held.unwrap().1);
        assert!(allows_prey_pointer_setdown(
            held,
            proof,
            true,
            Some(alternate_ground)
        ));
        assert_eq!(world.release_player_hold().unwrap(), Some(id));
        assert_eq!(world.entity(id).unwrap().position, alternate_ground);
        assert_eq!(world.tick(), tick);
        assert_eq!(world.object_snapshots().len(), 1);
    }

    #[cfg(feature = "qualia-village-review")]
    #[test]
    fn exact_chime_and_registered_prey_never_share_local_release_authority() {
        let mut world = alife_world::HeadlessWorld::new(1643);
        world.enable_island_for_new_game().unwrap();
        let terrain = world.terrain().unwrap().clone();
        let ground = |x, z| Vec3f::new(x, terrain.surface().height(x, z).unwrap(), z);
        let chime = world
            .spawn_toy("player-activity-toy", ground(80.0, 251.5), false)
            .unwrap();
        let prey = world
            .spawn_player_food(ground(84.0, 251.5), alife_world::FoodVariety::Fruit)
            .unwrap();
        world
            .enable_autonomous_critter(prey, 17, 0.040, 4.0)
            .unwrap();
        let hit = |id| {
            let tile = VoxelTileCoord::new(80, 251);
            Some(StableVoxelObjectRef {
                kind: StableVoxelRefKind::Resource,
                stable_id: Some(id),
                chunk: VoxelChunkCoord::for_tile(16, tile),
                tile: Some(tile),
            })
        };
        let cursor = Some(Vec2::new(100.0, 100.0));
        let mut hand = HandInteraction::default();
        world.begin_player_hold(chime).unwrap();
        assert!(!may_release_hand(Some(prey), &world));
        assert!(!adopt_reopened_chime(
            &mut hand,
            &world,
            hit(prey),
            cursor,
            true
        ));
        assert!(adopt_reopened_chime(
            &mut hand,
            &world,
            hit(chime),
            cursor,
            true
        ));
        assert!(terrain_prey_hold(hand.held, &world).is_none());
        assert!(may_release_hand(hand.held, &world));
        assert_eq!(world.release_player_hold().unwrap(), Some(chime));
        reset_chime_hand(&mut hand);

        world.begin_player_hold(prey).unwrap();
        let pose = world.entity(prey).unwrap().position;
        assert_eq!(chime_body_pick_id(&world), None);
        assert!(!adopt_reopened_chime(
            &mut hand,
            &world,
            hit(prey),
            cursor,
            true
        ));
        assert_eq!(terrain_prey_hold(Some(prey), &world), Some((prey, pose)));
        assert!(!allows_prey_pointer_setdown(
            Some((prey, pose)),
            None,
            true,
            Some(ground(84.0, 251.5))
        ));
        assert_eq!(world.player_held_object(), Some(prey));
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
