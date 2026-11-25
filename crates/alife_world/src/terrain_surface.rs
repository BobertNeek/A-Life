//! Immutable Y-up terrain shared by physical movement and graphical presentation.
//! The baked samples come from the same vertices as the visible near terrain.
use alife_core::{ScaffoldContractError, Vec3f};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) const CELL: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainBinding {
    pub version: u16,
    pub digest: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HeadlessScenarioBuilder, HeadlessWorld};
    use alife_core::{
        ActionCommand, ActionKind, ActionTarget, Confidence, DurationTicks, Intensity, OrganismId,
        PhysicalContactKind,
    };

    fn command(target: Vec3f) -> ActionCommand {
        ActionCommand::structured(
            OrganismId(1),
            ActionKind::Move.canonical_id(),
            ActionKind::Move,
            ActionTarget::new(None, Some(target)),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(0.9).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap()
    }
    fn world() -> HeadlessWorld {
        let mut world = HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        world.enable_island_for_new_game().unwrap();
        world
    }
    #[test]
    fn island_triangles_and_vertical_pick_agree() {
        let terrain = crate::island_terrain();
        let s = terrain.surface();
        // Interior samples on both sides of a deliberately non-planar grid cell.
        let fixture = TerrainSurface {
            width: 2,
            depth: 2,
            origin_x: 0.0,
            origin_z: 0.0,
            spacing: 1.0,
            digest: 0,
            water_level: Some(0.0),
            heights: vec![0.0, 2.0, 4.0, 10.0],
            obstacles: vec![],
            cells: HashMap::new(),
        };
        assert_eq!(fixture.height(0.75, 0.25), Some(3.5));
        assert_eq!(fixture.height(0.25, 0.75), Some(4.5));
        for (x, z) in [(32.0, 70.0), (-200.3, -301.7), (190.8, -410.1)] {
            let h = s.height(x, z).unwrap();
            let hit = s
                .ray_hit(Vec3f::new(x, 200.0, z), Vec3f::new(0.0, -1.0, 0.0), 300.0)
                .unwrap();
            assert!((hit.y - h).abs() < 0.001);
        }
        assert!(s.height(-501.0, 0.0).is_none());
    }
    #[test]
    fn ray_pick_does_not_skip_grid_or_triangle_ridges() {
        let surface = TerrainSurface::from_data(crate::TerrainData {
            width: 3,
            depth: 2,
            origin_x: 0.0,
            origin_z: 0.0,
            spacing: 1.0,
            heights: vec![0.0, 2.0, 0.0, 0.0, 2.0, 0.0],
            obstacles: Vec::new(),
            water_level: None,
        })
        .unwrap();
        let origin = Vec3f::new(0.0, 1.9, 0.25);
        let direction = Vec3f::new(1.0, 0.0, 0.0);
        let hit = surface.ray_hit(origin, direction, 2.0).unwrap();
        assert!((hit.x - 0.95).abs() < 0.00001);
        assert!(surface.ray_hit(origin, direction, 0.9).is_none());

        let surface = TerrainSurface::from_data(crate::TerrainData {
            width: 2,
            heights: vec![0.0, 0.0, 0.0, 4.0],
            ..surface.data()
        })
        .unwrap();
        let q = std::f32::consts::FRAC_1_SQRT_2;
        let hit = surface
            .ray_hit(Vec3f::new(0.0, 1.9, 1.0), Vec3f::new(q, 0.0, -q), 2.0)
            .unwrap();
        assert!((hit.x - 0.475).abs() < 0.00001);
        assert!((hit.z - 0.525).abs() < 0.00001);
        assert!((surface.height(hit.x, hit.z).unwrap() - hit.y).abs() < 0.00001);

        // This ray's clipped entry rounds just outside x = -500 in f32.
        let surface = TerrainSurface::from_data(crate::TerrainData {
            origin_x: -500.0,
            spacing: 10.0,
            heights: vec![0.0; 4],
            ..surface.data()
        })
        .unwrap();
        let dx = 0.19486223_f32;
        let hit = surface
            .ray_hit(
                Vec3f::new(-1310.7981, 4100.0, 0.25),
                Vec3f::new(dx, -(1.0 - dx * dx).sqrt(), 0.0),
                5000.0,
            )
            .unwrap();
        assert!((-500.0..=-490.0).contains(&hit.x));
        assert!(hit.y.abs() < 0.001);
        let hit = surface
            .ray_hit(
                Vec3f::new(-1310.7981, 0.0, 0.25),
                Vec3f::new(dx, -(1.0 - dx * dx).sqrt(), 0.0),
                5000.0,
            )
            .unwrap();
        assert!(surface.height(hit.x, hit.z).is_some());
    }
    #[test]
    fn rounded_mesh_boundary_supports_player_placement_and_restore() {
        let lo = 650.5266_f32;
        let spacing = 4.2522902_f32;
        let hi = lo + spacing;
        let terrain = crate::WorldTerrain::new(
            crate::TerrainData {
                width: 2,
                depth: 2,
                origin_x: lo,
                origin_z: lo,
                spacing,
                heights: vec![2.0; 4],
                obstacles: Vec::new(),
                water_level: None,
            },
            Default::default(),
        )
        .unwrap();
        for (x, z) in [(lo, lo), (hi, lo), (lo, hi), (hi, hi)] {
            assert_eq!(terrain.surface().sample(x, z), Some((2.0, [0.0; 2])));
        }
        for outside in [lo.next_down(), hi.next_up(), f32::NAN, f32::INFINITY] {
            assert!(terrain.surface().sample(outside, lo).is_none());
            assert!(terrain.surface().sample(lo, outside).is_none());
        }
        let mut world = HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        world
            .enable_terrain_for_new_game(terrain, Vec3f::new(lo, 0.0, lo))
            .unwrap();
        let id = world.entity_id("walker").unwrap();
        world.begin_player_hold(id).unwrap();
        world.move_player_hold(Vec3f::new(hi, 0.0, hi)).unwrap();
        let mut restored = HeadlessWorld::from_save_state(&world.save_state()).unwrap();
        restored.release_player_hold().unwrap();
        assert_eq!(
            restored.entity(id).unwrap().position,
            Vec3f::new(hi, 2.0, hi)
        );
    }
    #[test]
    fn island_actual_move_updates_authoritative_height_and_displacement() {
        let terrain = crate::island_terrain();
        let s = terrain.surface();
        let mut w = world();
        let id = w.entity_id("walker").unwrap();
        // Find a real sloping, unobstructed portion of the exported surface.
        let (start, end) = (-100..100)
            .flat_map(|z| (-100..100).map(move |x| (x as f32, z as f32)))
            .find_map(|(x, z)| {
                let a = Vec3f::new(x, s.height(x, z)?, z);
                let b = Vec3f::new(x + 0.3, s.height(x + 0.3, z)?, z);
                (s.walkable(x, z) && s.resolve_move(a, b).is_some() && (a.y - b.y).abs() > 0.01)
                    .then_some((a, b))
            })
            .expect("walkable hillside");
        w.editor_move_object(id, start).unwrap();
        let result = w.apply_command(&command(end)).unwrap();
        assert!(result.execution.succeeded);
        let actual = w.entity(id).unwrap().position;
        assert!((actual.y - s.height(actual.x, actual.z).unwrap()).abs() < 0.0001);
        assert!((actual.y - start.y).abs() > 0.005);
        assert!((result.execution.physical.displacement.y - (actual.y - start.y)).abs() < 0.0001);
    }
    #[test]
    fn island_actual_move_reports_blocked_at_a_baked_rock_or_trunk() {
        let terrain = crate::island_terrain();
        let s = terrain.surface();
        let mut w = world();
        let id = w.entity_id("walker").unwrap();
        let (start, end) = s
            .obstacles
            .iter()
            .find_map(|b| {
                let z = (b[1] + b[3]) * 0.5;
                let x = b[0] - crate::LocomotionLimits::default().body_radius - 0.04;
                let a = Vec3f::new(x, s.height(x, z)?, z);
                let e = Vec3f::new(x + 0.2, s.height(x + 0.2, z)?, z);
                (s.walkable(x, z)
                    && s.obstacle_at(e.x, e.z, crate::LocomotionLimits::default().body_radius))
                .then_some((a, e))
            })
            .expect("accessible collider boundary");
        w.editor_move_object(id, start).unwrap();
        let result = w.apply_command(&command(end)).unwrap();
        assert!(!result.execution.succeeded);
        assert_eq!(
            result.execution.physical.contact,
            PhysicalContactKind::Blocked
        );
        assert_eq!(result.execution.physical.displacement, Vec3f::ZERO);
        assert_eq!(w.entity(id).unwrap().position, start);
    }
    #[test]
    fn island_sweep_rejects_thin_obstacle_and_water_and_steep_ground() {
        let mut s = TerrainSurface {
            width: 2,
            depth: 2,
            origin_x: 0.0,
            origin_z: 0.0,
            spacing: 10.0,
            digest: 0,
            water_level: Some(0.0),
            heights: vec![1.0; 4],
            obstacles: vec![[4.95, 0.0, 5.05, 10.0, 1.0, 4.0]],
            cells: HashMap::from([((0, 0), vec![0])]),
        };
        assert!(s.walkable(1.0, 1.0) && s.walkable(9.0, 1.0));
        assert!(s
            .resolve_move(Vec3f::new(1.0, 1.0, 1.0), Vec3f::new(9.0, 1.0, 1.0))
            .is_none());
        s.obstacles.clear();
        s.cells.clear();
        s.heights.fill(-1.0);
        assert!(!s.walkable(1.0, 1.0));
        s.heights = vec![1.0, 15.0, 1.0, 15.0];
        assert!(!s.walkable(1.0, 1.0));
    }

    fn reference_ray_hit(
        surface: &TerrainSurface,
        origin: Vec3f,
        direction: Vec3f,
        max_distance: f32,
    ) -> Option<Vec3f> {
        if origin.validate().is_err()
            || direction.validate().is_err()
            || !max_distance.is_finite()
            || max_distance <= 0.0
        {
            return None;
        }
        let mut enter: f32 = 0.0;
        let mut exit = max_distance;
        let max_x = surface.origin_x + (surface.width - 1) as f32 * surface.spacing;
        let max_z = surface.origin_z + (surface.depth - 1) as f32 * surface.spacing;
        for (o, d, lo, hi) in [
            (origin.x, direction.x, surface.origin_x, max_x),
            (origin.z, direction.z, surface.origin_z, max_z),
        ] {
            if d == 0.0 {
                if o < lo || o > hi {
                    return None;
                }
            } else {
                let a = (lo - o) / d;
                let b = (hi - o) / d;
                enter = enter.max(a.min(b));
                exit = exit.min(a.max(b));
            }
        }
        if exit < enter {
            return None;
        }
        let point = |t: f32| {
            Vec3f::new(
                (origin.x + direction.x * t).clamp(surface.origin_x, max_x),
                origin.y + direction.y * t,
                (origin.z + direction.z * t).clamp(surface.origin_z, max_z),
            )
        };
        let mut cuts = vec![enter, exit];
        for (o, d, first, count) in [
            (
                origin.x - surface.origin_x,
                direction.x,
                1.0,
                surface.width - 2,
            ),
            (
                origin.z - surface.origin_z,
                direction.z,
                1.0,
                surface.depth - 2,
            ),
            (
                (origin.x - surface.origin_x) - (origin.z - surface.origin_z),
                direction.x - direction.z,
                -((surface.depth - 2) as f32),
                surface.width + surface.depth - 3,
            ),
        ] {
            if d == 0.0 {
                continue;
            }
            for i in 0..count {
                let t = ((first + i as f32) * surface.spacing - o) / d;
                if t > enter && t < exit {
                    cuts.push(t);
                }
            }
        }
        cuts.sort_by(f32::total_cmp);
        cuts.dedup();
        let mut previous = None;
        for t in cuts {
            let p = point(t);
            p.validate().ok()?;
            let u =
                ((p.x - surface.origin_x) / surface.spacing).clamp(0.0, (surface.width - 1) as f32);
            let v =
                ((p.z - surface.origin_z) / surface.spacing).clamp(0.0, (surface.depth - 1) as f32);
            let h = surface.sample_grid(u, v).0;
            let gap = f64::from(p.y) - f64::from(h);
            if gap <= 0.0 {
                if let Some((before, above)) = previous {
                    return Some(point(
                        before + (t - before) * (above / (above - gap)) as f32,
                    ));
                }
                return Some(Vec3f::new(p.x, h, p.z));
            }
            previous = Some((t, gap));
        }
        None
    }

    #[test]
    fn dda_ray_hit_matches_reference_cuts_on_grid_corners_diagonals_and_island() {
        let island = crate::island_terrain();
        let surface = island.surface();
        let mut seed = 0xDEAD_BEEF_CAFE_0001_u64;
        let mut next_f32 = |lo: f32, hi: f32| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let unit = ((seed >> 40) as f32) / ((1u32 << 24) as f32);
            lo + (hi - lo) * unit
        };
        let mut rays = vec![
            // Exact grid line & diagonal & corner cases
            (
                Vec3f::new(80.0, 200.0, 250.0),
                Vec3f::new(1.0, -0.2, 1.0),
                400.0,
            ),
            (
                Vec3f::new(80.0, 200.0, 250.0),
                Vec3f::new(-1.0, -0.2, -1.0),
                400.0,
            ),
            (
                Vec3f::new(80.0, 200.0, 250.0),
                Vec3f::new(1.0, -0.1, 0.0),
                400.0,
            ),
            (
                Vec3f::new(80.0, 200.0, 250.0),
                Vec3f::new(0.0, -0.1, -1.0),
                400.0,
            ),
            (
                Vec3f::new(-1000.0, 500.0, -1000.0),
                Vec3f::new(1.0, -0.3, 1.0),
                3000.0,
            ),
            (
                Vec3f::new(80.0, 300.0, 250.0),
                Vec3f::new(0.0, -1.0, 0.0),
                500.0,
            ),
        ];
        for _ in 0..400 {
            let ox = next_f32(-900.0, 900.0);
            let oy = next_f32(-20.0, 400.0);
            let oz = next_f32(-900.0, 900.0);
            let dx = next_f32(-1.0, 1.0);
            let dy = next_f32(-1.0, 0.2);
            let dz = next_f32(-1.0, 1.0);
            let len = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
            rays.push((
                Vec3f::new(ox, oy, oz),
                Vec3f::new(dx / len, dy / len, dz / len),
                next_f32(1.0, 1500.0),
            ));
        }
        for (origin, direction, max_dist) in rays {
            let actual = surface.ray_hit(origin, direction, max_dist);
            let expected = reference_ray_hit(surface, origin, direction, max_dist);
            assert_eq!(
                actual.map(|v| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]),
                expected.map(|v| [v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]),
                "ray_hit mismatch for origin={origin:?}, dir={direction:?}, max={max_dist}"
            );
        }
    }

    #[test]
    fn obstacle_ray_hit_matches_linear_scan_with_and_without_padding() {
        let island = crate::island_terrain();
        let surface = island.surface();
        let reference = |origin: Vec3f, direction: Vec3f, max_range: f32, padding: f32| {
            let mut nearest = max_range;
            let mut found = false;
            for b in &surface.obstacles {
                let bounds = if padding == 0.0 {
                    *b
                } else {
                    [
                        b[0] - padding,
                        b[1] - padding,
                        b[2] + padding,
                        b[3] + padding,
                        b[4],
                        b[5],
                    ]
                };
                if let Some(hit) = sight_box_hit(origin, direction, bounds, nearest) {
                    nearest = nearest.min(hit);
                    found = true;
                }
            }
            found.then_some(nearest)
        };

        let mut seed = 0xCAFE_BABE_1234_5678_u64;
        let mut next_f32 = |lo: f32, hi: f32| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let unit = ((seed >> 40) as f32) / ((1u32 << 24) as f32);
            lo + (hi - lo) * unit
        };
        let mut probes = Vec::new();
        for b in surface.obstacles.iter().take(40) {
            let cx = (b[0] + b[2]) * 0.5;
            let cy = (b[4] + b[5]) * 0.5;
            let cz = (b[1] + b[3]) * 0.5;
            // Point / vertical ray inside obstacle
            probes.push((Vec3f::new(cx, cy, cz), Vec3f::ZERO, 0.0, 0.0));
            probes.push((
                Vec3f::new(b[0] - 0.1, cy, cz),
                Vec3f::new(0.0, 1.0, 0.0),
                1.0,
                0.25,
            ));
            // Rays toward obstacle from nearby and far
            probes.push((
                Vec3f::new(cx - 25.0, cy, cz - 25.0),
                Vec3f::new(0.70710677, 0.0, 0.70710677),
                50.0,
                0.0,
            ));
            probes.push((
                Vec3f::new(cx - 1.2, cy, cz),
                Vec3f::new(1.0, 0.0, 0.0),
                1.5,
                0.35,
            ));
            probes.push((
                Vec3f::new(cx - 25.0, cy, cz - 15.0),
                Vec3f::new(0.8, 0.0, 0.6),
                50.0,
                0.5,
            ));
        }
        for _ in 0..300 {
            let ox = next_f32(-300.0, 300.0);
            let oy = next_f32(-5.0, 80.0);
            let oz = next_f32(-300.0, 300.0);
            let dx = next_f32(-1.0, 1.0);
            let dy = next_f32(-0.3, 0.3);
            let dz = next_f32(-1.0, 1.0);
            let len = (dx * dx + dy * dy + dz * dz).sqrt().max(0.001);
            let padding = if next_f32(0.0, 1.0) < 0.5 {
                0.0
            } else {
                next_f32(0.05, 1.5)
            };
            let range = if padding > 0.0 && next_f32(0.0, 1.0) < 0.5 {
                1.5
            } else {
                next_f32(1.0, 120.0)
            };
            probes.push((
                Vec3f::new(ox, oy, oz),
                Vec3f::new(dx / len, dy / len, dz / len),
                range,
                padding,
            ));
        }
        for (origin, dir, range, padding) in probes {
            let actual = surface.obstacle_ray_hit(origin, dir, range, padding);
            let expected = reference(origin, dir, range, padding);
            assert_eq!(
                actual.map(f32::to_bits),
                expected.map(f32::to_bits),
                "obstacle_ray_hit mismatch for origin={origin:?}, dir={dir:?}, range={range}, padding={padding}"
            );
        }
    }
}

impl TerrainBinding {
    /// Check the identity format. `WorldTerrain::restore` also verifies its data.
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if self.version != 2 || self.digest == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

pub(crate) fn sight_box_hit(
    origin: Vec3f,
    direction: Vec3f,
    bounds: [f32; 6],
    max_range: f32,
) -> Option<f32> {
    let mut entry = 0.0_f32;
    let mut exit = max_range;
    for (position, velocity, minimum, maximum) in [
        (origin.x, direction.x, bounds[0], bounds[2]),
        (origin.y, direction.y, bounds[4], bounds[5]),
        (origin.z, direction.z, bounds[1], bounds[3]),
    ] {
        if velocity.abs() <= f32::EPSILON {
            if position < minimum || position > maximum {
                return None;
            }
        } else {
            let near = (minimum - position) / velocity;
            let far = (maximum - position) / velocity;
            entry = entry.max(near.min(far));
            exit = exit.min(near.max(far));
            if entry > exit {
                return None;
            }
        }
    }
    Some(entry)
}

#[derive(Debug, Clone)]
pub struct TerrainSurface {
    pub width: usize,
    pub depth: usize,
    pub origin_x: f32,
    pub origin_z: f32,
    pub spacing: f32,
    pub digest: u64,
    pub(crate) heights: Vec<f32>,
    pub(crate) obstacles: Vec<[f32; 6]>,
    pub(crate) cells: HashMap<(i32, i32), Vec<usize>>,
    pub(crate) water_level: Option<f32>,
}

impl TerrainSurface {
    pub fn height(&self, x: f32, z: f32) -> Option<f32> {
        self.sample(x, z).map(|p| p.0)
    }
    /// Height and surface gradient on the exact near-mesh triangle diagonal.
    pub fn sample(&self, x: f32, z: f32) -> Option<(f32, [f32; 2])> {
        let max_u = (self.width - 1) as f32;
        let max_v = (self.depth - 1) as f32;
        if !x.is_finite()
            || !z.is_finite()
            || x < self.origin_x
            || z < self.origin_z
            || x > self.origin_x + max_u * self.spacing
            || z > self.origin_z + max_v * self.spacing
        {
            return None;
        }
        // A real mesh edge can normalize slightly beyond its last grid index.
        let u = ((x - self.origin_x) / self.spacing).clamp(0.0, max_u);
        let v = ((z - self.origin_z) / self.spacing).clamp(0.0, max_v);
        Some(self.sample_grid(u, v))
    }
    fn sample_grid(&self, u: f32, v: f32) -> (f32, [f32; 2]) {
        let i = (u.floor() as usize).min(self.width - 2);
        let j = (v.floor() as usize).min(self.depth - 2);
        let (u, v) = (u - i as f32, v - j as f32);
        let k = j * self.width + i;
        let (a, b, c, d) = (
            self.heights[k],
            self.heights[k + 1],
            self.heights[k + self.width + 1],
            self.heights[k + self.width],
        );
        if v >= u {
            (
                a * (1.0 - v) + d * (v - u) + c * u,
                [(c - d) / self.spacing, (d - a) / self.spacing],
            )
        } else {
            (
                a * (1.0 - u) + b * (u - v) + c * v,
                [(b - a) / self.spacing, (c - b) / self.spacing],
            )
        }
    }
    pub fn obstacle_at(&self, x: f32, z: f32, radius: f32) -> bool {
        // Search every cell touched by this body's clearance, not a baked radius.
        let min_x = ((x - radius) / CELL).floor() as i32;
        let max_x = ((x + radius) / CELL).floor() as i32;
        let min_z = ((z - radius) / CELL).floor() as i32;
        let max_z = ((z + radius) / CELL).floor() as i32;
        (min_z..=max_z).any(|cz| {
            (min_x..=max_x).any(|cx| {
                self.cells.get(&(cx, cz)).is_some_and(|rows| {
                    rows.iter().any(|i| {
                        let b = self.obstacles[*i];
                        let dx = (b[0] - x).max(0.0).max(x - b[2]);
                        let dz = (b[1] - z).max(0.0).max(z - b[3]);
                        dx * dx + dz * dz <= radius * radius
                    })
                })
            })
        })
    }
    pub fn walkable(&self, x: f32, z: f32) -> bool {
        self.walkable_with(x, z, crate::LocomotionLimits::default())
    }
    pub fn walkable_with(&self, x: f32, z: f32, limits: crate::LocomotionLimits) -> bool {
        if limits.validate().is_err() {
            return false;
        }
        self.sample(x, z).is_some_and(|(h, g)| {
            self.water_level
                .is_none_or(|water| h >= water - limits.max_wading_depth)
                && g[0].hypot(g[1]) <= limits.max_slope
                && !self.obstacle_at(x, z, limits.body_radius)
        })
    }
    /// Sweep the full move so a fast step cannot jump across a rock or water edge.
    pub fn resolve_move(&self, start: Vec3f, end: Vec3f) -> Option<Vec3f> {
        self.resolve_move_with(start, end, crate::LocomotionLimits::default())
    }
    pub fn resolve_move_with(
        &self,
        start: Vec3f,
        end: Vec3f,
        limits: crate::LocomotionLimits,
    ) -> Option<Vec3f> {
        let distance = (end.x - start.x).hypot(end.z - start.z);
        if !distance.is_finite() || distance > 64.0 || limits.validate().is_err() {
            return None;
        }
        let step = 0.15_f32.min(limits.body_radius.max(0.001));
        let steps = (distance / step).ceil().max(1.0) as usize;
        for i in 1..=steps {
            let t = i as f32 / steps as f32;
            if !self.walkable_with(
                start.x + (end.x - start.x) * t,
                start.z + (end.z - start.z) * t,
                limits,
            ) {
                return None;
            }
        }
        Some(Vec3f::new(end.x, self.height(end.x, end.z)?, end.z))
    }
    pub fn ray_hit(&self, origin: Vec3f, direction: Vec3f, max_distance: f32) -> Option<Vec3f> {
        if origin.validate().is_err()
            || direction.validate().is_err()
            || !max_distance.is_finite()
            || max_distance <= 0.0
        {
            return None;
        }
        // Clip against the horizontal sample bounds before intersecting. A distant
        // overview camera must honor its requested ray range, not a 2 km cap.
        let mut enter: f32 = 0.0;
        let mut exit = max_distance;
        let max_x = self.origin_x + (self.width - 1) as f32 * self.spacing;
        let max_z = self.origin_z + (self.depth - 1) as f32 * self.spacing;
        for (o, d, lo, hi) in [
            (origin.x, direction.x, self.origin_x, max_x),
            (origin.z, direction.z, self.origin_z, max_z),
        ] {
            if d == 0.0 {
                if o < lo || o > hi {
                    return None;
                }
            } else {
                let a = (lo - o) / d;
                let b = (hi - o) / d;
                enter = enter.max(a.min(b));
                exit = exit.min(a.max(b));
            }
        }
        if exit < enter {
            return None;
        }
        let point = |t: f32| {
            Vec3f::new(
                (origin.x + direction.x * t).clamp(self.origin_x, max_x),
                origin.y + direction.y * t,
                (origin.z + direction.z * t).clamp(self.origin_z, max_z),
            )
        };
        let mut previous = None;
        let eval = |t: f32, previous: &mut Option<(f32, f64)>| -> Result<Option<Vec3f>, ()> {
            let p = point(t);
            if p.validate().is_err() {
                return Err(());
            }
            // Slab-boundary rounding must not reject a valid clipped endpoint.
            let u = ((p.x - self.origin_x) / self.spacing).clamp(0.0, (self.width - 1) as f32);
            let v = ((p.z - self.origin_z) / self.spacing).clamp(0.0, (self.depth - 1) as f32);
            let h = self.sample_grid(u, v).0;
            let gap = f64::from(p.y) - f64::from(h);
            if gap <= 0.0 {
                if let Some((before, above)) = *previous {
                    return Ok(Some(point(
                        before + (t - before) * (above / (above - gap)) as f32,
                    )));
                }
                return Ok(Some(Vec3f::new(p.x, h, p.z)));
            }
            *previous = Some((t, gap));
            Ok(None)
        };

        if let Some(hit) = eval(enter, &mut previous).ok()? {
            return Some(hit);
        }
        if exit == enter {
            return None;
        }

        // Zero-allocation 2D grid DDA across cells and triangle diagonals U - V = i - j.
        let p_enter = point(enter);
        let u_enter =
            ((p_enter.x - self.origin_x) / self.spacing).clamp(0.0, (self.width - 1) as f32);
        let v_enter =
            ((p_enter.z - self.origin_z) / self.spacing).clamp(0.0, (self.depth - 1) as f32);
        let mut i = if direction.x < 0.0 && u_enter == u_enter.floor() && u_enter > 0.0 {
            (u_enter as usize - 1).min(self.width - 2)
        } else {
            (u_enter.floor() as usize).min(self.width - 2)
        };
        let mut j = if direction.z < 0.0 && v_enter == v_enter.floor() && v_enter > 0.0 {
            (v_enter as usize - 1).min(self.depth - 2)
        } else {
            (v_enter.floor() as usize).min(self.depth - 2)
        };

        let ox = origin.x - self.origin_x;
        let oz = origin.z - self.origin_z;
        let od = ox - oz;
        let dd = direction.x - direction.z;
        let mut last_t = enter;

        for _ in 0..(self.width + self.depth + 2) {
            let t_x = if direction.x > 0.0 && i < self.width - 2 {
                ((i + 1) as f32 * self.spacing - ox) / direction.x
            } else if direction.x < 0.0 && i >= 1 {
                (i as f32 * self.spacing - ox) / direction.x
            } else {
                f32::INFINITY
            };
            let t_z = if direction.z > 0.0 && j < self.depth - 2 {
                ((j + 1) as f32 * self.spacing - oz) / direction.z
            } else if direction.z < 0.0 && j >= 1 {
                (j as f32 * self.spacing - oz) / direction.z
            } else {
                f32::INFINITY
            };
            let t_cell_exit = t_x.min(t_z);
            let t_step = t_cell_exit.min(exit);

            if dd != 0.0 {
                let diag = (i as isize - j as isize) as f32;
                let t_diag = (diag * self.spacing - od) / dd;
                if t_diag > last_t && t_diag < t_step {
                    if let Some(hit) = eval(t_diag, &mut previous).ok()? {
                        return Some(hit);
                    }
                    last_t = t_diag;
                }
            }

            if t_step > last_t {
                if let Some(hit) = eval(t_step, &mut previous).ok()? {
                    return Some(hit);
                }
                last_t = t_step;
            }

            if t_cell_exit >= exit {
                break;
            }
            if t_x <= t_cell_exit {
                if direction.x > 0.0 {
                    if i + 1 > self.width - 2 {
                        break;
                    }
                    i += 1;
                } else {
                    if i == 0 {
                        break;
                    }
                    i -= 1;
                }
            }
            if t_z <= t_cell_exit {
                if direction.z > 0.0 {
                    if j + 1 > self.depth - 2 {
                        break;
                    }
                    j += 1;
                } else {
                    if j == 0 {
                        break;
                    }
                    j -= 1;
                }
            }
        }
        if last_t < exit {
            if let Some(hit) = eval(exit, &mut previous).ok()? {
                return Some(hit);
            }
        }
        None
    }

    /// Cell-bounded 2D DDA obstacle raycast over `TerrainSurface::cells`.
    /// When `padding > 0.0`, obstacle X/Z bounds are expanded by `padding`
    /// identically to `local_clearance_ranges` and `activity_placement`.
    pub(crate) fn obstacle_ray_hit(
        &self,
        origin: Vec3f,
        direction: Vec3f,
        max_range: f32,
        padding: f32,
    ) -> Option<f32> {
        if self.obstacles.is_empty()
            || self.cells.is_empty()
            || origin.validate().is_err()
            || direction.validate().is_err()
            || !max_range.is_finite()
            || max_range < 0.0
            || !padding.is_finite()
            || padding < 0.0
        {
            return None;
        }

        let eps_x = direction.x.abs() <= f32::EPSILON;
        let eps_z = direction.z.abs() <= f32::EPSILON;
        let mut nearest = max_range;
        let mut found = false;

        let test_cell = |cx: i32, cz: i32, nearest: &mut f32, found: &mut bool| {
            let Some(rows) = self.cells.get(&(cx, cz)) else {
                return;
            };
            for &idx in rows {
                let b = self.obstacles[idx];
                let bounds = if padding == 0.0 {
                    b
                } else {
                    [
                        b[0] - padding,
                        b[1] - padding,
                        b[2] + padding,
                        b[3] + padding,
                        b[4],
                        b[5],
                    ]
                };
                if let Some(hit) = sight_box_hit(origin, direction, bounds, *nearest) {
                    *nearest = (*nearest).min(hit);
                    *found = true;
                }
            }
        };

        let pad_margin = if padding == 0.0 {
            0.0_f64
        } else {
            f64::from(padding) * (1.0 + 8.0 * f64::from(f32::EPSILON)) + 1e-5
        };
        let cell_f64 = f64::from(CELL);

        if eps_x && eps_z {
            let x0 = ((f64::from(origin.x) - pad_margin) / cell_f64).floor() as i32;
            let x1 = ((f64::from(origin.x) + pad_margin) / cell_f64).floor() as i32;
            let z0 = ((f64::from(origin.z) - pad_margin) / cell_f64).floor() as i32;
            let z1 = ((f64::from(origin.z) + pad_margin) / cell_f64).floor() as i32;
            for cz in z0..=z1 {
                for cx in x0..=x1 {
                    test_cell(cx, cz, &mut nearest, &mut found);
                }
            }
            return found.then_some(nearest);
        }

        let end_x = f64::from(origin.x) + f64::from(direction.x) * f64::from(max_range);
        let end_z = f64::from(origin.z) + f64::from(direction.z) * f64::from(max_range);
        let min_x = f64::from(origin.x).min(end_x) - pad_margin;
        let max_x = f64::from(origin.x).max(end_x) + pad_margin;
        let min_z = f64::from(origin.z).min(end_z) - pad_margin;
        let max_z = f64::from(origin.z).max(end_z) + pad_margin;
        let x0 = (min_x / cell_f64).floor() as i32;
        let x1 = (max_x / cell_f64).floor() as i32;
        let z0 = (min_z / cell_f64).floor() as i32;
        let z1 = (max_z / cell_f64).floor() as i32;
        let span_x = i64::from(x1) - i64::from(x0) + 1;
        let span_z = i64::from(z1) - i64::from(z0) + 1;

        if span_x.saturating_mul(span_z) <= 16 {
            for cz in z0..=z1 {
                for cx in x0..=x1 {
                    test_cell(cx, cz, &mut nearest, &mut found);
                }
            }
            return found.then_some(nearest);
        }

        // 2D Amanatides-Woo DDA over CELL = 10.0.
        let mut cx = (f64::from(origin.x) / cell_f64).floor() as i32;
        let mut cz = (f64::from(origin.z) / cell_f64).floor() as i32;
        let (step_x, mut t_max_x, t_delta_x) = if eps_x {
            (0, f64::INFINITY, f64::INFINITY)
        } else {
            let dx = f64::from(direction.x);
            let step = if dx > 0.0 { 1 } else { -1 };
            let next_boundary = if step > 0 {
                f64::from(cx + 1) * cell_f64
            } else {
                f64::from(cx) * cell_f64
            };
            (
                step,
                ((next_boundary - f64::from(origin.x)) / dx).max(0.0),
                cell_f64 / dx.abs(),
            )
        };
        let (step_z, mut t_max_z, t_delta_z) = if eps_z {
            (0, f64::INFINITY, f64::INFINITY)
        } else {
            let dz = f64::from(direction.z);
            let step = if dz > 0.0 { 1 } else { -1 };
            let next_boundary = if step > 0 {
                f64::from(cz + 1) * cell_f64
            } else {
                f64::from(cz) * cell_f64
            };
            (
                step,
                ((next_boundary - f64::from(origin.z)) / dz).max(0.0),
                cell_f64 / dz.abs(),
            )
        };

        let pad_cells = if padding == 0.0 {
            0
        } else {
            ((f64::from(padding) + 1e-5) / cell_f64).ceil() as i32
        };
        let pad_t_margin = if padding == 0.0 {
            1e-4
        } else {
            let max_inv = [direction.x.abs(), direction.z.abs()]
                .into_iter()
                .filter(|v| *v > f32::EPSILON)
                .map(|v| 1.0 / f64::from(v))
                .fold(0.0_f64, f64::max);
            f64::from(padding) * max_inv + 1e-3
        };

        let mut t_enter = 0.0_f64;
        loop {
            if t_enter > f64::from(nearest) + pad_t_margin {
                break;
            }
            if pad_cells == 0 {
                test_cell(cx, cz, &mut nearest, &mut found);
            } else {
                for pz in (cz - pad_cells).max(z0)..=(cz + pad_cells).min(z1) {
                    for px in (cx - pad_cells).max(x0)..=(cx + pad_cells).min(x1) {
                        test_cell(px, pz, &mut nearest, &mut found);
                    }
                }
            }
            if step_x != 0 && t_max_x <= t_max_z {
                t_enter = t_max_x;
                cx += step_x;
                t_max_x += t_delta_x;
                if cx < x0 || cx > x1 {
                    break;
                }
            } else if step_z != 0 {
                t_enter = t_max_z;
                cz += step_z;
                t_max_z += t_delta_z;
                if cz < z0 || cz > z1 {
                    break;
                }
            } else {
                break;
            }
        }
        found.then_some(nearest)
    }
}
