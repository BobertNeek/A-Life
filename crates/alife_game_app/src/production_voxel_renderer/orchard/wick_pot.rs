//! A reversible unharvested planter on the existing Garden Hand.
//! Only the original World orchard bed travels; the pottery is its fitted skin.
use super::*;
use alife_world::orchard::OrchardBed;

const WICK_POT_REVIEW: bool = true;

#[derive(Clone, Copy)]
pub(in crate::production_voxel_renderer) struct WickPot {
    source: OrchardBed,
    cursor: VoxelTileCoord,
    shown: Option<Seat>,
}

/// Last successful inverse is only a suggested aim, not a saved action.
#[derive(Clone, Copy)]
pub(in crate::production_voxel_renderer) struct WickReturn {
    placed: OrchardBed,
    former: VoxelTileCoord,
}

#[derive(Clone, Copy, PartialEq)]
struct Seat {
    tile: VoxelTileCoord,
    fit: Option<Vec3f>,
}

fn ground_tile(ray: Vec3) -> Option<VoxelTileCoord> {
    (ray.is_finite() && (-512.0..512.0).contains(&ray.x) && (-512.0..512.0).contains(&ray.z))
        .then(|| VoxelTileCoord::new(ray.x.floor() as i32, ray.z.floor() as i32))
}

fn matches_shown(shown: Option<Seat>, now: Option<Seat>) -> Option<Seat> {
    now.filter(|s| s.fit.is_some() && Some(*s) == shown)
}

fn watched(
    world: &alife_world::HeadlessWorld,
    tile: VoxelTileCoord,
) -> Option<OrchardBed> {
    (world.orchard_gesture(tile) == Some(OrchardGesture::Watch))
        .then(|| world.movable_orchard_planter(tile))
        .flatten()
}

pub(super) fn observe_watch(
    dock: &mut care_dock::State,
    world: &alife_world::HeadlessWorld,
    tile: VoxelTileCoord,
) {
    if WICK_POT_REVIEW {
        dock.wick_watch = watched(world, tile);
    }
}

fn end(dock: &mut care_dock::State) {
    dock.wick_pot = None;
    dock.wick_watch = None;
    dock.garden_armed = false;
    dock.garden_cursor = None;
    dock.garden_pointer = false;
}

#[allow(clippy::too_many_arguments, reason = "Use the existing Garden Hand's input and World authority.")]
pub(super) fn drive(
    keyboard: &ButtonInput<KeyCode>,
    hand_ground: Option<Vec3>,
    shortcut: bool,
    pointer_tool: bool,
    selection: &mut Fvr03ProductionVoxelSelectionResource,
    follow: &mut Fvr04ProductionCreatureFollowResource,
    dock: &mut care_dock::State,
    runtime: Option<&mut crate::GpuLiveBrainRuntime>,
) -> bool {
    if !WICK_POT_REVIEW {
        return false;
    }
    let shift = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    if let Some(mut pot) = dock.wick_pot.take() {
        if !dock.garden_armed || keyboard.just_pressed(KeyCode::Escape) || pointer_tool {
            end(dock);
            return true;
        }
        let Some(runtime) = runtime else {
            end(dock);
            return true;
        };
        if runtime.world().movable_orchard_planter(pot.source.tile) != Some(pot.source) {
            end(dock);
            return true;
        }
        let dx = i32::from(keyboard.just_pressed(KeyCode::ArrowRight))
            - i32::from(keyboard.just_pressed(KeyCode::ArrowLeft));
        let dz = i32::from(keyboard.just_pressed(KeyCode::ArrowDown))
            - i32::from(keyboard.just_pressed(KeyCode::ArrowUp));
        if dx != 0 || dz != 0 {
            dock.garden_pointer = false;
            pot.cursor = VoxelTileCoord::new(
                (pot.cursor.x + dx).clamp(-512, 511),
                (pot.cursor.z + dz).clamp(-512, 511),
            );
        }
        let target = if dock.garden_pointer || dock.garden_ground_click {
            hand_ground.and_then(ground_tile)
        } else {
            Some(pot.cursor)
        };
        if let Some(tile) = target {
            pot.cursor = tile;
        }
        // Hovering the actual source is a no-op, not a second object or
        // an obstacle. Only a different aimed support makes an outline.
        let seat = target.filter(|tile| *tile != pot.source.tile).map(|tile| Seat {
            tile,
            fit: runtime.world().survey_orchard_planter(pot.source, tile),
        });
        let committed = if shortcut || dock.garden_ground_click {
            matches_shown(pot.shown, seat).and_then(|accepted| {
                runtime.reseat_player_orchard_planter(pot.source, accepted.tile)
                    .ok()
                    .map(|placed| (accepted, placed))
            })
        } else {
            None
        };
        if let Some((accepted, placed)) = committed {
            dock.wick_return = Some(WickReturn { placed, former: pot.source.tile });
            selection.selected = Some(selected_soil(accepted.tile));
            follow.enabled = false;
            end(dock);
            return true;
        }
        // No paid or durable lay-down is fabricated by a blocked/stale preview.
        pot.shown = seat;
        dock.wick_pot = Some(pot);
        return true;
    }
    let world = runtime.as_deref().map(|r| r.world());
    let tile = soil_tile(selection.selected);
    let watch = world.zip(tile).and_then(|(world, t)| watched(world, t));
    if dock.wick_watch != watch || keyboard.just_pressed(KeyCode::Escape) {
        dock.wick_watch = None;
    }
    // First ordinary Garden Watch retains its existing result. Its lifted
    // clay grips expose a second deliberate use of that same control.
    let seen_grip = watch.is_some() && watch == dock.wick_watch;
    if (shift || seen_grip) && (shortcut || pointer_tool) {
        if keyboard.just_pressed(KeyCode::Escape) {
            return true;
        }
        let available = world.zip(tile).and_then(|(world, tile)| {
            world.movable_orchard_planter(tile).map(|source| (world, source))
        });
        if let Some((world, source)) = available {
            let cursor = dock.wick_return.filter(|p| p.placed == source)
                .map_or(source.tile, |p| p.former);
            dock.garden_armed = true;
            dock.garden_pointer = false;
            dock.wick_watch = None;
            // Show the actual former seat when available. Even if blocked,
            // no alternate is chosen. This activation never commits.
            dock.wick_pot = Some(WickPot {
                source,
                cursor,
                shown: (cursor != source.tile).then(|| Seat {
                    tile: cursor, fit: world.survey_orchard_planter(source, cursor),
                }),
            });
        }
        return true;
    }
    false
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind { Built, Offered }

#[derive(Component)]
pub(in crate::production_voxel_renderer) struct WickPiece(VoxelTileCoord, Kind, usize);

#[derive(Default)]
pub(in crate::production_voxel_renderer) struct PotArt(
    Option<(Handle<Mesh>, [Handle<StandardMaterial>; 2])>,
);

#[derive(Clone, Copy)]
enum RimPose { Seated, Grips, Refused }

fn rims(x: f32, z: f32, y: [f32; 2], posture: RimPose) -> [Transform; 2] {
    let (height, roll) = match posture {
        RimPose::Seated => (0.0, 0.0),
        RimPose::Grips => (0.19, -0.58),
        RimPose::Refused => (0.22, 0.86),
    };
    [-1.0, 1.0].map(|side| {
        let i = usize::from(side > 0.0);
        Transform::from_xyz(x + side * 0.33, y[i] + 0.018 + height, z)
            .with_rotation(Quat::from_rotation_z(side * roll))
            .with_scale(Vec3::splat(0.38))
    })
}

/// Existing signed ceramic geometry, same two forms laid down or curled away.
/// Visuals never create a collision/selection authority or mutate orchard state.
#[allow(clippy::too_many_arguments, reason = "Independent renderer and authoritative world Resources.")]
pub(in crate::production_voxel_renderer) fn sync(
    live: Option<&alife_world::HeadlessWorld>,
    dock: Option<&care_dock::State>,
    server: Option<&AssetServer>,
    materials: Option<&mut Assets<StandardMaterial>>,
    cache: &mut PotArt,
    commands: &mut Commands,
    pieces: &mut Query<(Entity, &WickPiece, &mut Transform), Without<super::plants::Plant>>,
) {
    if !WICK_POT_REVIEW {
        return;
    }
    let Some((world, terrain)) = live.and_then(|world| world.terrain().map(|t| (world, t))) else {
        for (id, ..) in pieces.iter() {
            commands.entity(id).despawn();
        }
        return;
    };
    let (Some(server), Some(materials)) = (server, materials) else { return; };
    let surface = terrain.surface();
    let (mesh, skin) = cache.0.get_or_insert_with(|| (
        server.load(bevy::gltf::GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }
            .from_asset("care/ceramic-waystone.glb")),
        [
            materials.add(StandardMaterial { perceptual_roughness: 0.78, ..default() }),
            materials.add(StandardMaterial { base_color: Color::srgba(1.0, 0.94, 0.77, 0.53),
                alpha_mode: AlphaMode::Blend, perceptual_roughness: 0.78, ..default() }),
        ],
    ));
    let mut wanted = std::collections::BTreeMap::new();
    let mut add = |tile: VoxelTileCoord, kind, posture| {
        let x = tile.x as f32 + 0.5;
        let z = tile.z as f32 + 0.5;
        let (Some(left), Some(right)) = (surface.height(x - 0.33, z), surface.height(x + 0.33, z)) else {
            return;
        };
        let mut heights = [left, right];
        if matches!(posture, RimPose::Refused) {
            if let Some(water) = terrain.water_level() {
                heights = heights.map(|ground| ground.max(water));
            }
        }
        for (i, pose) in rims(x, z, heights, posture).into_iter().enumerate() {
            wanted.insert((tile, kind, i), pose);
        }
    };
    let primed = dock
        .and_then(|d| {
            d.wick_watch.filter(|bed| watched(world, bed.tile) == Some(*bed))
                .or_else(|| d.wick_pot.filter(|pot| pot.shown.is_none()).map(|pot| pot.source))
        })
        .filter(|bed| world.movable_orchard_planter(bed.tile) == Some(*bed));
    for bed in &world.ecology().orchard_beds {
        let pose = if primed == Some(*bed) { RimPose::Grips } else { RimPose::Seated };
        // The original actual bed stays drawn, fitted and untouched while
        // aiming elsewhere. The optional preview is only its two outlines.
        add(bed.tile, Kind::Built, pose);
    }
    if let Some(seat) = dock.filter(|d| d.garden_armed).and_then(|d| d.wick_pot).and_then(|p| p.shown) {
        let pose = if seat.fit.is_some() { RimPose::Seated } else { RimPose::Refused };
        add(seat.tile, Kind::Offered, pose);
    }
    for (id, part, mut pose) in pieces.iter_mut() {
        if let Some(next) = wanted.remove(&(part.0, part.1, part.2)) {
            pose.set_if_neq(next);
        } else {
            commands.entity(id).despawn();
        }
    }
    for ((tile, kind, i), pose) in wanted {
        let mut e = commands.spawn((
            WickPiece(tile, kind, i), Mesh3d(mesh.clone()),
            MeshMaterial3d(skin[usize::from(kind == Kind::Offered)].clone()),
            pose, Fvr04ProductionRuntimeSceneRoot, bevy::picking::Pickable::IGNORE,
        ));
        if kind == Kind::Offered {
            e.insert(bevy::light::NotShadowCaster);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wick_pot_preview_matches_exact_shown_fit_and_blocked_pottery_stands_open() {
        let tile = VoxelTileCoord::new(82, 252);
        let legal = Seat { tile, fit: Some(Vec3f::new(82.5, 6.675, 252.5)) };
        let blocked = Seat { tile, fit: None };
        assert_eq!(matches_shown(None, Some(legal)).map(|s| s.tile), None);
        assert_eq!(matches_shown(Some(legal), Some(blocked)).map(|s| s.tile), None);
        assert_eq!(matches_shown(Some(legal), Some(legal)).map(|s| s.tile), Some(tile));
        let [l, r] = rims(82.5, 252.5, [6.7, 6.65], RimPose::Seated);
        let [ll, rr] = rims(82.5, 252.5, [6.7, 6.65], RimPose::Refused);
        let [g, _] = rims(82.5, 252.5, [6.7, 6.65], RimPose::Grips);
        assert!(ll.translation.y - l.translation.y > 0.20);
        assert!(rr.translation.y - r.translation.y > 0.20);
        assert_ne!(ll.rotation, rr.rotation);
        assert_eq!(l.rotation, r.rotation);
        assert!((g.translation.y - l.translation.y - 0.19).abs() < 0.0001);
        assert!(g.rotation.z.signum() != ll.rotation.z.signum());
    }
}
