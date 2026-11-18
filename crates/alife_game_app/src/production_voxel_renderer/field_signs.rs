//! A selected Hearthling leaves a short, readable trail of its observed movement.
//! This is field notation: no paths are requested of the creature or written into a save.
pub(super) mod glass;
use super::*;
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use std::collections::VecDeque;

const LIFE: u64 = 360;
const LIMIT: usize = 36;
const SPACING: f32 = 0.38;

#[derive(Clone, Copy)]
struct Print {
    p: Vec2,
    forward: Vec2,
    left: bool,
    born: u64,
    serial: u64,
}
#[derive(Default, Resource)]
pub(super) struct Trail {
    who: Option<WorldEntityId>,
    last: Option<(u64, Vec2)>,
    remaining: f32,
    left: bool,
    serial: u64,
    marks: VecDeque<Print>,
}

impl Trail {
    fn choose(&mut self, id: WorldEntityId) {
        if self.who != Some(id) {
            self.clear();
            self.who = Some(id);
        }
    }
    pub(super) fn clear(&mut self) {
        self.who = None;
        self.last = None;
        self.remaining = SPACING;
        self.left = false;
        self.marks.clear();
    }
    fn sample(&mut self, tick: u64, position: Option<Vec2>) {
        if self.last.is_some_and(|(t, _)| t > tick) {
            self.marks.clear();
            self.last = None;
        }
        while self
            .marks
            .front()
            .is_some_and(|m| tick.saturating_sub(m.born) >= LIFE)
        {
            self.marks.pop_front();
        }
        let Some(p) = position.filter(|p| p.is_finite()) else {
            self.last = None;
            self.remaining = SPACING;
            return;
        };
        let Some((t, old)) = self.last else {
            self.last = Some((tick, p));
            self.remaining = SPACING;
            return;
        };
        if t == tick {
            return;
        }
        self.last = Some((tick, p));
        let delta = p - old;
        let d = delta.length();
        // Do not draw through a teleport, a loading gap, a Hand carry or an unseen jump.
        if tick - t > 8 || d > SPACING * 4.0 {
            self.remaining = SPACING;
            return;
        }
        if d < 0.0001 {
            return;
        }
        let forward = delta / d;
        let side = Vec2::new(-forward.y, forward.x);
        let mut along = self.remaining;
        while along <= d {
            let p = old + forward * along + side * if self.left { 0.092 } else { -0.092 };
            self.serial = self.serial.wrapping_add(1);
            self.marks.push_back(Print {
                p,
                forward,
                left: self.left,
                born: tick,
                serial: self.serial,
            });
            if self.marks.len() > LIMIT {
                self.marks.pop_front();
            }
            self.left = !self.left;
            along += SPACING;
        }
        self.remaining = along - d;
    }
}

#[derive(Component)]
pub(super) struct Impression(u64);
#[derive(Default)]
pub(super) struct Ink {
    art: Option<StampArt>,
    frame: Option<(u64, Option<u64>, Option<u64>)>,
}
struct StampArt {
    mesh: Handle<Mesh>,
    stains: [Handle<StandardMaterial>; 4],
}

/// The approved raccoon-foot sculpt has one heel and five connected toes. The
/// softer outer band keeps the sole looking pressed into the grass, not painted on it.
fn impression_mesh() -> Mesh {
    let mut pos = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();
    let mut ix = Vec::new();
    let mut oval = |cx: f32, cz: f32, rx: f32, rz: f32, angle: f32| {
        let start = pos.len() as u32;
        let emit = |p: Vec2,
                    opacity: f32,
                    pos: &mut Vec<[f32; 3]>,
                    normals: &mut Vec<[f32; 3]>,
                    colors: &mut Vec<[f32; 4]>| {
            let p = Vec2::from_angle(angle).rotate(p) + Vec2::new(cx, cz);
            pos.push([p.x, 0., p.y]);
            normals.push([0., 1., 0.]);
            colors.push([1., 1., 1., opacity]);
        };
        emit(Vec2::ZERO, 1., &mut pos, &mut normals, &mut colors);
        const N: u32 = 12;
        for (r, a) in [(0.70, 0.91), (1., 0.)] {
            for i in 0..N {
                let theta = -(i as f32) * std::f32::consts::TAU / N as f32;
                emit(
                    Vec2::new(rx * r * theta.cos(), rz * r * theta.sin()),
                    a,
                    &mut pos,
                    &mut normals,
                    &mut colors,
                );
            }
        }
        for i in 0..N {
            let j = (i + 1) % N;
            let (a, b, c, d) = (
                start + 1 + i,
                start + 1 + j,
                start + 1 + N + i,
                start + 1 + N + j,
            );
            ix.extend([start, a, b, a, c, d, a, d, b]);
        }
    };
    oval(0., -0.035, 0.082, 0.103, 0.10);
    for (x, z, rz, tilt) in [
        (-0.086, 0.040, 0.040, 0.52),
        (-0.052, 0.096, 0.047, 0.31),
        (-0.009, 0.117, 0.055, 0.07),
        (0.036, 0.105, 0.051, -0.16),
        (0.074, 0.074, 0.042, -0.35),
    ] {
        oval(x, z, 0.024, rz, tilt);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, pos)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(ix))
}

fn pose(foot: Print, surface: &creature_grounding::RenderedTerrainSurface) -> Option<Transform> {
    let p = Vec3::new(foot.p.x, 0., foot.p.y);
    let y = surface.height(p)?;
    let h = |delta| surface.height(p + delta).unwrap_or(y);
    let dx = Vec3::X * 0.08;
    let dz = Vec3::Z * 0.08;
    let normal = Vec3::new(h(-dx) - h(dx), 0.16, h(-dz) - h(dz)).normalize();
    let turn = Quat::from_rotation_y(foot.forward.x.atan2(foot.forward.y));
    Some(
        Transform::from_translation(Vec3::new(p.x, y, p.z) + normal * 0.011)
            .with_rotation(Quat::from_rotation_arc(Vec3::Y, normal) * turn)
            .with_scale(Vec3::new(if foot.left { 1. } else { -1. }, 1., 1.)),
    )
}

pub(super) fn sample(
    mut trail: ResMut<Trail>,
    frame: Res<LiveBrainPresentationFrameResource>,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    runtime: Option<NonSend<ProductionGpuBrainRuntimeResource>>,
) {
    let Some(runtime) = runtime else {
        trail.clear();
        return;
    };
    let world = runtime.runtime.world();
    if let Some(id) = selection
        .selected
        .filter(|s| s.kind == StableVoxelRefKind::Creature)
        .and_then(|s| s.stable_id)
    {
        trail.choose(id);
    }
    let position = trail.who.and_then(|id| {
        let o = frame.current.object(id)?;
        let p = Vec2::new(
            o.position.x,
            if world.terrain().is_some() {
                o.position.z
            } else {
                o.position.y
            },
        );
        (o.kind == WorldObjectKind::Agent
            && !o.consumed
            && o.carried_by.is_none()
            && world.player_held_object() != Some(id)
            && frame
                .current
                .organism(id)
                .is_some_and(|row| row.lifecycle.is_alive())
            && world.terrain().is_none_or(|t| t.walkable(p.x, p.y)))
        .then_some(p)
    });
    trail.sample(frame.current.authoritative_world_tick.raw(), position);
}

#[allow(
    clippy::too_many_arguments,
    reason = "A single bounded batch projects observed footprints onto the selected terrain."
)]
pub(super) fn draw(
    mut commands: Commands,
    trail: Res<Trail>,
    frame: Res<LiveBrainPresentationFrameResource>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    mut ink: Local<Ink>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut existing: Query<(Entity, &Impression, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    let tick = frame.current.authoritative_world_tick.raw();
    let signature = (
        tick,
        trail.marks.front().map(|p| p.serial),
        trail.marks.back().map(|p| p.serial),
    );
    if ink.frame == Some(signature) && !surface.is_changed() {
        return;
    }
    ink.frame = Some(signature);
    if trail.marks.is_empty() {
        for (e, _, _) in &existing {
            commands.entity(e).despawn();
        }
        return;
    }
    let art = ink.art.get_or_insert_with(|| StampArt {
        mesh: meshes.add(impression_mesh()),
        stains: [0.74, 0.54, 0.33, 0.14].map(|a| {
            materials.add(StandardMaterial {
                base_color: Color::srgba(0.19, 0.15, 0.11, a),
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                cull_mode: None,
                ..default()
            })
        }),
    });
    let stain =
        |foot: &Print| &art.stains[(tick.saturating_sub(foot.born) * 4 / LIFE).min(3) as usize];
    let first = trail.marks[0].serial;
    let mut present = [false; LIMIT];
    for (e, mark, mut material) in &mut existing {
        let index = mark.0.wrapping_sub(first);
        if let Some(foot) = usize::try_from(index).ok().and_then(|i| trail.marks.get(i)) {
            present[index as usize] = true;
            if material.0 != *stain(foot) {
                material.0 = stain(foot).clone();
            }
        } else {
            commands.entity(e).despawn();
        }
    }
    for (i, &foot) in trail.marks.iter().enumerate() {
        if present[i] {
            continue;
        }
        if let Some(t) = pose(foot, &surface) {
            commands.spawn((
                Name::new("Witnessed Hearthling footfall"),
                Impression(foot.serial),
                Mesh3d(art.mesh.clone()),
                MeshMaterial3d(stain(&foot).clone()),
                t,
                bevy::picking::Pickable::IGNORE,
                bevy::light::NotShadowCaster,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_pause_carry_load_or_teleport_never_draws_an_invented_route() {
        let mut t = Trail::default();
        t.choose(WorldEntityId(3));
        t.sample(10, Some(Vec2::ZERO));
        t.sample(10, Some(Vec2::X));
        assert!(t.marks.is_empty());
        t.sample(11, Some(Vec2::X * 0.85));
        assert_eq!(t.marks.len(), 2);
        assert_ne!(t.marks[0].left, t.marks[1].left);
        t.sample(12, Some(Vec2::X * 80.));
        assert_eq!(t.marks.len(), 2);
        t.sample(13, None);
        t.sample(14, Some(Vec2::X * 2.));
        assert_eq!(t.marks.len(), 2);
        t.sample(1, Some(Vec2::X));
        assert!(t.marks.is_empty());
    }
    #[test]
    fn footprints_age_in_simulation_time_and_remain_bounded() {
        let mut t = Trail::default();
        t.choose(WorldEntityId(1));
        for i in 0..150 {
            t.sample(i, Some(Vec2::X * i as f32 * 0.41));
        }
        assert_eq!(t.marks.len(), LIMIT);
        t.sample(151 + LIFE, None);
        assert!(t.marks.is_empty());
        t.sample(600, Some(Vec2::ZERO));
        t.sample(601, Some(Vec2::X));
        t.choose(WorldEntityId(2));
        assert!(t.marks.is_empty());
    }
}
