//! Bounded authored layouts become world-owned solid terrain, not policy hints.
use super::*;
use alife_core::Vec3f;
use alife_world::{LocomotionLimits, TerrainData, WorldTerrain};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

/// Fixed two-unit cells, X right and Z down. L anchors the fresh founder body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCareLayout {
    pub rows: Vec<String>,
}

impl CpuCareLayout {
    fn cells(&self) -> Result<((usize, usize), (usize, usize))> {
        let height = self.rows.len();
        let width = self.rows.first().map_or(0, String::len);
        if !(3..=17).contains(&height) || !(3..=17).contains(&width) {
            return Err("layout dimensions must each be 3..=17 cells".into());
        }
        let mut learner = None;
        let mut food = None;
        for (z, row) in self.rows.iter().enumerate() {
            if row.len() != width {
                return Err("layout rows must be rectangular ASCII".into());
            }
            for (x, cell) in row.bytes().enumerate() {
                if (x == 0 || z == 0 || x + 1 == width || z + 1 == height) && cell != b'#' {
                    return Err("layout requires a closed wall border".into());
                }
                match cell {
                    b'#' | b'.' => {}
                    b'L' if learner.replace((x, z)).is_none() => {}
                    b'F' if food.replace((x, z)).is_none() => {}
                    _ => {
                        return Err(
                            "layout needs exactly one L and F, and only # . L F cells".into()
                        )
                    }
                }
            }
        }
        Ok((
            learner.ok_or("layout lacks learner L")?,
            food.ok_or("layout lacks food F")?,
        ))
    }

    fn geometry(&self, origin: Vec3f) -> Result<(WorldTerrain, Vec3f)> {
        let (learner, food) = self.cells()?;
        let position = |(x, z): (usize, usize)| {
            Vec3f::new(
                origin.x + 2.0 * (x as f32 - learner.0 as f32),
                0.0,
                origin.z + 2.0 * (z as f32 - learner.1 as f32),
            )
        };
        let mut obstacles = Vec::new();
        for (z, row) in self.rows.iter().enumerate() {
            for (x, cell) in row.bytes().enumerate() {
                if cell == b'#' {
                    let p = position((x, z));
                    obstacles.push([p.x - 1.0, p.z - 1.0, p.x + 1.0, p.z + 1.0, 0.0, 3.0]);
                }
            }
        }
        let terrain = WorldTerrain::new(
            TerrainData {
                width: 2,
                depth: 2,
                origin_x: -256.0,
                origin_z: -256.0,
                spacing: 512.0,
                heights: vec![0.0; 4],
                obstacles,
                water_level: None,
            },
            LocomotionLimits::default(),
        )?;
        // Every accepted route edge must be traversable under the engine's
        // swept body collision authority. No teleport or policy-visible route.
        let mut seen = BTreeSet::from([learner]);
        let mut queue = VecDeque::from([learner]);
        while let Some((x, z)) = queue.pop_front() {
            if (x, z) == food {
                return Ok((terrain, position(food)));
            }
            for (nx, nz) in [
                (x as isize - 1, z as isize),
                (x as isize + 1, z as isize),
                (x as isize, z as isize - 1),
                (x as isize, z as isize + 1),
            ] {
                if nx < 0
                    || nz < 0
                    || nz as usize >= self.rows.len()
                    || nx as usize >= self.rows[0].len()
                {
                    continue;
                }
                let next = (nx as usize, nz as usize);
                if self.rows[next.1].as_bytes()[next.0] == b'#' || seen.contains(&next) {
                    continue;
                }
                let target = position(next);
                if terrain
                    .resolve_move(position((x, z)), target)
                    .is_some_and(|p| (p.x - target.x).abs() < 1e-5 && (p.z - target.z).abs() < 1e-5)
                {
                    seen.insert(next);
                    queue.push_back(next);
                }
            }
        }
        Err("layout food is unreachable under native terrain collision".into())
    }

    pub(super) fn validate(
        &self,
        lesson: FoundationTeacherLesson,
        offset: Option<[f32; 2]>,
    ) -> Result<()> {
        if lesson != FoundationTeacherLesson::Feeding || offset.is_some() {
            return Err("layout requires feeding and cannot be combined with food_offset".into());
        }
        self.geometry(Vec3f::ZERO)?;
        Ok(())
    }

    pub(super) fn prepare(&self, world: &mut alife_world::HeadlessWorld) -> Result<[f32; 2]> {
        // This hook runs on a fresh episode body, never a retained private world.
        // Refuse rather than replace any pre-existing terrain/private life.
        if world.terrain().is_some() || world.tick().raw() != 0 {
            return Err(
                "authored layout requires a fresh ungrounded tick-zero episode world".into(),
            );
        }
        let mut prepared = world.clone();
        let founder = prepared
            .organism_entity_ids()
            .first()
            .ok_or("layout founder missing")?
            .1;
        let flat = world
            .entity(founder)
            .ok_or("layout founder missing")?
            .position;
        let origin = Vec3f::new(flat.x, 0.0, flat.y);
        let (terrain, food) = self.geometry(origin)?;
        // Move stock clutter before terrain conversion, so it cannot force a
        // relocation search inside an authored wall or an unrelated closed room.
        for (index, label) in [
            "obstacle-01",
            "obstacle-02",
            "hazard-01",
            "food-02",
            "meadow-ball",
            "meadow-activity-toy",
        ]
        .iter()
        .enumerate()
        {
            if let Some(id) = prepared.entity_id(label) {
                prepared.editor_move_object(
                    id,
                    Vec3f::new(origin.x - 80.0 - index as f32 * 3.0, origin.z - 80.0, 0.0),
                )?;
            }
        }
        if let Some(id) = prepared.entity_id("food-01") {
            prepared.editor_move_object(id, Vec3f::new(food.x, food.z, 0.0))?;
        }
        prepared.enable_terrain_for_new_game(terrain, Vec3f::ZERO)?;
        let actual = prepared
            .entity(founder)
            .ok_or("layout founder missing")?
            .position;
        if actual != origin {
            return Err("terrain conversion moved the authored layout learner anchor".into());
        }
        // Authored open cells must not retain the stock scene's hidden clutter.
        for (index, label) in [
            "obstacle-01",
            "obstacle-02",
            "hazard-01",
            "food-02",
            "meadow-ball",
            "meadow-activity-toy",
        ]
        .iter()
        .enumerate()
        {
            if let Some(id) = prepared.entity_id(label) {
                let mut physical = prepared
                    .entity(id)
                    .ok_or("layout clutter missing")?
                    .grounded_physical;
                prepared.editor_move_object(
                    id,
                    Vec3f::new(origin.x - 80.0 - index as f32 * 3.0, 0.0, origin.z - 80.0),
                )?;
                physical.velocity = Vec3f::ZERO;
                prepared.set_grounded_physical_properties(id, physical)?;
            }
        }
        *world = prepared;
        Ok([food.x, food.z])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn layout(rows: &[&str]) -> CpuCareLayout {
        CpuCareLayout {
            rows: rows.iter().map(|s| s.to_string()).collect(),
        }
    }
    #[test]
    fn authored_walls_block_native_motion_but_detour_remains_reachable() {
        let map = layout(&["#######", "#L#..F#", "#.....#", "#######"]);
        let (terrain, food) = map.geometry(Vec3f::ZERO).unwrap();
        assert_eq!(food, Vec3f::new(8.0, 0.0, 0.0));
        assert!(!terrain.walkable(2.0, 0.0));
        assert!(terrain.resolve_move(Vec3f::ZERO, food).is_none());
        assert_eq!(
            terrain.resolve_move(Vec3f::ZERO, Vec3f::new(0.0, 0.0, 2.0)),
            Some(Vec3f::new(0.0, 0.0, 2.0))
        );
        // Native terrain serialization retains exactly the same collision shape.
        let state = terrain.state();
        let restored = WorldTerrain::new(state.data, state.locomotion).unwrap();
        assert_eq!(restored.binding(), terrain.binding());
        assert!(restored.resolve_move(Vec3f::ZERO, food).is_none());
    }
    #[test]
    fn authored_layout_rejects_invalid_and_unreachable_maps() {
        for rows in [
            vec!["#####", "#L#F#", "#####"],
            vec!["#####", "#L.F.", "#####"],
            vec!["#####", "#LLF#", "#####"],
            vec!["#####", "#L?F#", "#####"],
            vec!["#####", "#L.F#", "####"],
        ] {
            assert!(layout(&rows).geometry(Vec3f::ZERO).is_err());
        }
        let valid = layout(&["#####", "#L.F#", "#####"]);
        assert!(valid
            .validate(FoundationTeacherLesson::RelocatedFood, None)
            .is_err());
        assert!(valid
            .validate(FoundationTeacherLesson::Feeding, Some([2.0, 0.0]))
            .is_err());
        valid
            .validate(FoundationTeacherLesson::Feeding, None)
            .unwrap();
    }
}
