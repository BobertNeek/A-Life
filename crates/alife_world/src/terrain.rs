//! World-owned heightfields. Geometry is immutable and shared; movement limits
//! describe the body, not the map or the neural policy.
use crate::terrain_surface::CELL;
use crate::{TerrainBinding, TerrainSurface};
use alife_core::{ScaffoldContractError, Vec3f};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainData {
    pub width: usize,
    pub depth: usize,
    pub origin_x: f32,
    pub origin_z: f32,
    pub spacing: f32,
    pub heights: Vec<f32>,
    /// Solid obstacle bounds: min x/z, max x/z, min/max height.
    pub obstacles: Vec<[f32; 6]>,
    pub water_level: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocomotionLimits {
    pub body_radius: f32,
    /// Maximum rise/run, not degrees.
    pub max_slope: f32,
    pub max_wading_depth: f32,
}
impl Default for LocomotionLimits {
    fn default() -> Self {
        Self {
            body_radius: 0.2,
            max_slope: 0.85,
            max_wading_depth: 0.6,
        }
    }
}
impl LocomotionLimits {
    pub fn validate(self) -> Result<(), ScaffoldContractError> {
        if [self.body_radius, self.max_slope, self.max_wading_depth]
            .into_iter()
            .any(|v| !v.is_finite() || v < 0.0)
            || self.body_radius > 64.0
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainState {
    pub data: TerrainData,
    pub locomotion: LocomotionLimits,
}

#[derive(Debug, Clone)]
pub struct WorldTerrain {
    surface: Arc<TerrainSurface>,
    binding: TerrainBinding,
    limits: LocomotionLimits,
}

struct Fnv1aWriter(u64);

impl std::io::Write for Fnv1aWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for &byte in buf {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl TerrainSurface {
    pub(crate) fn matches_data_bits(&self, data: &TerrainData) -> bool {
        self.width == data.width
            && self.depth == data.depth
            && self.origin_x.to_bits() == data.origin_x.to_bits()
            && self.origin_z.to_bits() == data.origin_z.to_bits()
            && self.spacing.to_bits() == data.spacing.to_bits()
            && match (self.water_level, data.water_level) {
                (Some(a), Some(b)) => a.to_bits() == b.to_bits(),
                (None, None) => true,
                _ => false,
            }
            && self.heights.len() == data.heights.len()
            && self
                .heights
                .iter()
                .zip(&data.heights)
                .all(|(a, b)| a.to_bits() == b.to_bits())
            && self.obstacles.len() == data.obstacles.len()
            && self.obstacles.iter().zip(&data.obstacles).all(|(a, b)| {
                a.iter()
                    .zip(b.iter())
                    .all(|(x, y)| x.to_bits() == y.to_bits())
            })
    }

    pub fn from_data(data: TerrainData) -> Result<Self, ScaffoldContractError> {
        let invalid = || ScaffoldContractError::InvalidId;
        let count = data.width.checked_mul(data.depth).ok_or_else(invalid)?;
        if data.width < 2
            || data.depth < 2
            || count != data.heights.len()
            || !data.spacing.is_finite()
            || data.spacing <= 0.0
            || !data.origin_x.is_finite()
            || !data.origin_z.is_finite()
            || data.origin_x + data.spacing <= data.origin_x
            || data.origin_z + data.spacing <= data.origin_z
            || !(data.origin_x + (data.width - 1) as f32 * data.spacing).is_finite()
            || !(data.origin_z + (data.depth - 1) as f32 * data.spacing).is_finite()
            || data.heights.iter().any(|h| !h.is_finite())
            || data.water_level.is_some_and(|v| !v.is_finite())
            || data.obstacles.iter().any(|b| {
                b.iter().any(|v| !v.is_finite()) || b[0] >= b[2] || b[1] >= b[3] || b[4] >= b[5]
            })
        {
            return Err(invalid());
        }
        let mut hasher = Fnv1aWriter(0xcbf29ce484222325u64);
        serde_json::to_writer(&mut hasher, &data).map_err(|_| invalid())?;
        let digest = hasher.0;
        let mut cells: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, b) in data.obstacles.iter().enumerate() {
            let (x0, x1, z0, z1) = (
                (b[0] / CELL).floor() as i32,
                (b[2] / CELL).floor() as i32,
                (b[1] / CELL).floor() as i32,
                (b[3] / CELL).floor() as i32,
            );
            // Bound index construction for malformed/extreme imported bounds.
            if (i64::from(x1) - i64::from(x0) + 1).saturating_mul(i64::from(z1) - i64::from(z0) + 1)
                > 1_000_000
            {
                return Err(invalid());
            }
            for z in z0..=z1 {
                for x in x0..=x1 {
                    cells.entry((x, z)).or_default().push(i);
                }
            }
        }
        Ok(Self {
            width: data.width,
            depth: data.depth,
            origin_x: data.origin_x,
            origin_z: data.origin_z,
            spacing: data.spacing,
            heights: data.heights,
            obstacles: data.obstacles,
            water_level: data.water_level,
            digest,
            cells,
        })
    }
    pub fn data(&self) -> TerrainData {
        TerrainData {
            width: self.width,
            depth: self.depth,
            origin_x: self.origin_x,
            origin_z: self.origin_z,
            spacing: self.spacing,
            heights: self.heights.clone(),
            obstacles: self.obstacles.clone(),
            water_level: self.water_level,
        }
    }
}

impl WorldTerrain {
    pub fn new(data: TerrainData, limits: LocomotionLimits) -> Result<Self, ScaffoldContractError> {
        limits.validate()?;
        let surface = Arc::new(TerrainSurface::from_data(data)?);
        Ok(Self {
            binding: TerrainBinding {
                version: 2,
                digest: surface.digest,
            },
            surface,
            limits,
        })
    }
    pub fn restore(
        binding: Option<TerrainBinding>,
        state: Option<&TerrainState>,
    ) -> Result<Option<Self>, ScaffoldContractError> {
        let Some(binding) = binding else {
            return if state.is_none() {
                Ok(None)
            } else {
                Err(ScaffoldContractError::InvalidId)
            };
        };
        binding.validate()?;
        let state = state.ok_or(ScaffoldContractError::InvalidId)?;
        state.locomotion.validate()?;
        let island = crate::island_terrain();
        if binding == island.binding() && island.surface().matches_data_bits(&state.data) {
            return Ok(Some(Self {
                surface: Arc::clone(island.surface()),
                binding,
                limits: state.locomotion,
            }));
        }
        let result = Self::new(state.data.clone(), state.locomotion)?;
        if result.binding != binding {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(Some(result))
    }
    pub fn binding(&self) -> TerrainBinding {
        self.binding
    }
    pub fn surface(&self) -> &Arc<TerrainSurface> {
        &self.surface
    }
    pub fn limits(&self) -> LocomotionLimits {
        self.limits
    }
    pub fn state(&self) -> TerrainState {
        TerrainState {
            data: self.surface.data(),
            locomotion: self.limits,
        }
    }
    pub fn walkable(&self, x: f32, z: f32) -> bool {
        self.surface.walkable_with(x, z, self.limits)
    }
    pub fn resolve_move(&self, a: Vec3f, b: Vec3f) -> Option<Vec3f> {
        self.surface.resolve_move_with(a, b, self.limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HeadlessScenarioBuilder;
    use alife_core::{
        ActionCommand, ActionKind, ActionTarget, Confidence, DurationTicks, Intensity, OrganismId,
        PhysicalContactKind,
    };

    pub(crate) fn data() -> TerrainData {
        TerrainData {
            width: 2,
            depth: 2,
            origin_x: 1000.0,
            origin_z: 2000.0,
            spacing: 10.0,
            heights: vec![2.0; 4],
            obstacles: vec![],
            water_level: None,
        }
    }
    fn command(x: f32) -> ActionCommand {
        ActionCommand::structured(
            OrganismId(1),
            ActionKind::Move.canonical_id(),
            ActionKind::Move,
            ActionTarget::new(None, Some(Vec3f::new(x, 10.35, 2001.0))),
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
    #[test]
    fn selected_terrain_controls_placement_movement_and_obstacles() {
        let base = HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        let mut a = base.clone();
        let mut b = base;
        let first = WorldTerrain::new(data(), LocomotionLimits::default()).unwrap();
        let mut other = data();
        other.heights = vec![10.0, 12.0, 10.0, 12.0];
        other.obstacles = vec![[1002.0, 2000.0, 1002.05, 2010.0, 10.0, 14.0]];
        let second = WorldTerrain::new(other, LocomotionLimits::default()).unwrap();
        a.enable_terrain_for_new_game(first, Vec3f::new(1001.0, 0.0, 2001.0))
            .unwrap();
        b.enable_terrain_for_new_game(second, Vec3f::new(1001.0, 0.0, 2001.0))
            .unwrap();
        let id = a.entity_id("walker").unwrap();
        assert_eq!(a.entity(id).unwrap().position.y, 2.0);
        assert!((b.entity(id).unwrap().position.y - 10.2).abs() < 0.001);
        for world in [&mut a, &mut b] {
            let result = world.apply_command(&command(1001.3)).unwrap();
            assert!(result.execution.succeeded);
            let p = world.entity(id).unwrap().position;
            assert_eq!(
                Some(p.y),
                world.terrain().unwrap().surface().height(p.x, p.z)
            );
        }
        for world in [&mut a, &mut b] {
            world
                .editor_move_object(id, Vec3f::new(1001.75, 0.0, 2001.0))
                .unwrap();
        }
        assert!(
            a.apply_command(&command(1002.0))
                .unwrap()
                .execution
                .succeeded
        );
        let before = b.entity(id).unwrap().position;
        let blocked = b.apply_command(&command(1002.0)).unwrap();
        assert!(!blocked.execution.succeeded);
        assert_eq!(
            blocked.execution.physical.contact,
            PhysicalContactKind::Blocked
        );
        assert_eq!(b.entity(id).unwrap().position, before);
        a.editor_move_object(id, Vec3f::new(1004.0, -999.0, 2004.0))
            .unwrap();
        assert_eq!(a.entity(id).unwrap().position.y, 2.0);
        assert!(b
            .editor_move_object(id, Vec3f::new(1002.0, 0.0, 2004.0))
            .is_err());
    }
    #[test]
    fn environment_water_and_body_limits_are_independent() {
        let mut d = data();
        d.heights = vec![0.0, 10.0, 0.0, 10.0];
        let surface = TerrainSurface::from_data(d.clone()).unwrap();
        assert!(!surface.walkable(1001.0, 2001.0));
        let limits = LocomotionLimits {
            max_slope: 1.1,
            ..Default::default()
        };
        assert!(surface.walkable_with(1001.0, 2001.0, limits));
        d.water_level = Some(2.0);
        let wet = TerrainSurface::from_data(d).unwrap();
        assert!(!wet.walkable_with(1001.0, 2001.0, limits));
        assert!(wet.walkable_with(
            1001.0,
            2001.0,
            LocomotionLimits {
                max_wading_depth: 1.1,
                ..limits
            }
        ));
        assert_ne!(wet.digest, surface.digest);
        let mut d = data();
        d.obstacles = vec![[1010.1, 2000.0, 1011.0, 2010.0, 2.0, 4.0]];
        let surface = TerrainSurface::from_data(d).unwrap();
        assert!(!surface.obstacle_at(1009.5, 2001.0, 0.2));
        assert!(surface.obstacle_at(1009.5, 2001.0, 0.8));
    }
    #[test]
    fn invalid_terrain_is_rejected_and_clones_share_samples() {
        let world = WorldTerrain::new(data(), Default::default()).unwrap();
        assert!(Arc::ptr_eq(world.surface(), world.clone().surface()));
        for broken in [
            TerrainData { width: 1, ..data() },
            TerrainData {
                spacing: 0.0,
                ..data()
            },
            TerrainData {
                heights: vec![f32::NAN; 4],
                ..data()
            },
            TerrainData {
                obstacles: vec![[0.0; 6]],
                ..data()
            },
        ] {
            assert!(WorldTerrain::new(broken, Default::default()).is_err());
        }
        assert!(WorldTerrain::restore(Some(world.binding()), None).is_err());
    }
    #[test]
    fn streaming_fnv1a_matches_vec_digest_and_restore_fast_path_checks_sign_bits() {
        let island = crate::island_terrain();
        let state = island.state();
        let bytes = serde_json::to_vec(&state.data).unwrap();
        let expected_digest = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        });
        assert_eq!(island.binding().digest, expected_digest);

        let restored = WorldTerrain::restore(Some(island.binding()), Some(&state))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(restored.surface(), island.surface()));

        // Custom locomotion limits still reuse the cached island surface Arc.
        let custom_state = TerrainState {
            locomotion: LocomotionLimits {
                body_radius: 0.3,
                ..state.locomotion
            },
            ..state.clone()
        };
        let restored_custom = WorldTerrain::restore(Some(island.binding()), Some(&custom_state))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(restored_custom.surface(), island.surface()));
        assert_eq!(restored_custom.limits().body_radius, 0.3);

        // -0.0 in water_level or heights must not match +0.0 via fast-path and must fail binding check
        // (since JSON encodes -0.0 differently from 0.0).
        let mut neg_zero_water = state.clone();
        neg_zero_water.data.water_level = Some(-0.0);
        assert!(WorldTerrain::restore(Some(island.binding()), Some(&neg_zero_water)).is_err());

        let mut neg_zero_height = state;
        if let Some(pos_zero_idx) = neg_zero_height
            .data
            .heights
            .iter()
            .position(|h| h.to_bits() == 0.0_f32.to_bits())
        {
            neg_zero_height.data.heights[pos_zero_idx] = -0.0;
            assert!(WorldTerrain::restore(Some(island.binding()), Some(&neg_zero_height)).is_err());
        }
    }
}
