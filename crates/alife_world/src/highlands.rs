//! Immutable Y-up terrain shared by physical movement and graphical presentation.
//! The baked samples come from the same vertices as the visible near terrain.
use alife_core::{ScaffoldContractError, Vec3f};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::OnceLock};

const BYTES: &[u8] = include_bytes!("../assets/highlands-v1.bin");
pub(crate) const CELL: f32 = 10.0;
pub const HIGHLANDS_WATER_HEIGHT: f32 = 0.25;
pub const HIGHLANDS_BODY_RADIUS: f32 = 0.2;

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
        world.enable_highlands_for_new_game().unwrap();
        world
    }
    #[test]
    fn highlands_triangles_and_vertical_pick_agree() {
        let s = highlands();
        // Interior samples on both sides of a deliberately non-planar grid cell.
        let fixture = HighlandsSurface {
            width: 2,
            depth: 2,
            origin_x: 0.0,
            origin_z: 0.0,
            spacing: 1.0,
            digest: 0,
            water_level: Some(HIGHLANDS_WATER_HEIGHT),
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
        assert!(s.height(-401.0, 0.0).is_none());
    }
    #[test]
    fn highlands_actual_move_updates_authoritative_height_and_displacement() {
        let s = highlands();
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
    fn highlands_actual_move_reports_blocked_at_a_baked_rock_or_trunk() {
        let s = highlands();
        let mut w = world();
        let id = w.entity_id("walker").unwrap();
        let (start, end) = s
            .obstacles
            .iter()
            .find_map(|b| {
                let z = (b[1] + b[3]) * 0.5;
                let x = b[0] - HIGHLANDS_BODY_RADIUS - 0.04;
                let a = Vec3f::new(x, s.height(x, z)?, z);
                let e = Vec3f::new(x + 0.2, s.height(x + 0.2, z)?, z);
                (s.walkable(x, z) && s.obstacle_at(e.x, e.z, HIGHLANDS_BODY_RADIUS))
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
    fn highlands_sweep_rejects_thin_obstacle_and_water_and_steep_ground() {
        let mut s = HighlandsSurface {
            width: 2,
            depth: 2,
            origin_x: 0.0,
            origin_z: 0.0,
            spacing: 10.0,
            digest: 0,
            water_level: Some(HIGHLANDS_WATER_HEIGHT),
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
}

impl TerrainBinding {
    pub fn highlands() -> Self {
        Self {
            version: 1,
            digest: highlands().digest,
        }
    }
    /// Check the identity format. `WorldTerrain::restore` also verifies its data.
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if !matches!(self.version, 1 | 2) || self.digest == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
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

pub type HighlandsSurface = TerrainSurface;

pub fn highlands() -> &'static TerrainSurface {
    static SURFACE: OnceLock<HighlandsSurface> = OnceLock::new();
    SURFACE.get_or_init(|| {
        assert_eq!(&BYTES[..8], b"HLAND001");
        let u32_at = |i| u32::from_le_bytes(BYTES[i..i + 4].try_into().unwrap());
        let f32_at = |i| f32::from_le_bytes(BYTES[i..i + 4].try_into().unwrap());
        let width = u32_at(8) as usize;
        let depth = u32_at(12) as usize;
        let count = u32_at(28) as usize;
        assert_eq!(BYTES.len(), 32 + width * depth * 4 + count * 24);
        let heights = (0..width * depth)
            .map(|i| f32_at(32 + i * 4))
            .collect::<Vec<_>>();
        assert!(heights.iter().all(|h| h.is_finite()));
        let obstacles = (0..count)
            .map(|i| std::array::from_fn(|j| f32_at(32 + width * depth * 4 + i * 24 + j * 4)))
            .collect::<Vec<_>>();
        let mut cells: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, b) in obstacles.iter().enumerate() {
            for z in ((b[1] - 0.5) / CELL).floor() as i32..=((b[3] + 0.5) / CELL).floor() as i32 {
                for x in ((b[0] - 0.5) / CELL).floor() as i32..=((b[2] + 0.5) / CELL).floor() as i32
                {
                    cells.entry((x, z)).or_default().push(i);
                }
            }
        }
        let digest = BYTES.iter().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        });
        HighlandsSurface {
            width,
            depth,
            origin_x: f32_at(16),
            origin_z: f32_at(20),
            spacing: f32_at(24),
            digest,
            heights,
            water_level: Some(HIGHLANDS_WATER_HEIGHT),
            obstacles,
            cells,
        }
    })
}

impl TerrainSurface {
    pub fn height(&self, x: f32, z: f32) -> Option<f32> {
        self.sample(x, z).map(|p| p.0)
    }
    /// Height and surface gradient on the exact near-mesh triangle diagonal.
    pub fn sample(&self, x: f32, z: f32) -> Option<(f32, [f32; 2])> {
        let u = (x - self.origin_x) / self.spacing;
        let v = (z - self.origin_z) / self.spacing;
        if !u.is_finite()
            || !v.is_finite()
            || u < 0.0
            || v < 0.0
            || u > (self.width - 1) as f32
            || v > (self.depth - 1) as f32
        {
            return None;
        }
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
        Some(if v >= u {
            (
                a * (1.0 - v) + d * (v - u) + c * u,
                [(c - d) / self.spacing, (d - a) / self.spacing],
            )
        } else {
            (
                a * (1.0 - u) + b * (u - v) + c * v,
                [(b - a) / self.spacing, (c - b) / self.spacing],
            )
        })
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
        let point = |t: f32| {
            Vec3f::new(
                origin.x + direction.x * t,
                origin.y + direction.y * t,
                origin.z + direction.z * t,
            )
        };
        let mut previous = None;
        for i in 0..=((max_distance.min(2000.0) / 1.25).ceil() as usize) {
            let t = i as f32 * 1.25;
            let p = point(t);
            let Some(h) = self.height(p.x, p.z) else {
                previous = None;
                continue;
            };
            if p.y <= h {
                if let Some(mut lo) = previous {
                    let mut hi = t;
                    for _ in 0..14 {
                        let m = (lo + hi) * 0.5;
                        let p = point(m);
                        if p.y > self.height(p.x, p.z)? {
                            lo = m;
                        } else {
                            hi = m;
                        }
                    }
                    return Some(point(hi));
                }
                return Some(Vec3f::new(p.x, h, p.z));
            }
            previous = Some(t);
        }
        None
    }
}
