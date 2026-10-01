//! World-owned heightfields. Geometry is immutable and shared; movement limits
//! describe the body, not the map or the neural policy.
use crate::highlands::CELL;
use crate::{highlands, TerrainBinding, TerrainSurface};
use alife_core::{ScaffoldContractError, Vec3f};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

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
    /// None is only accepted for the explicitly identified legacy baked asset.
    pub data: Option<TerrainData>,
    pub locomotion: LocomotionLimits,
}

#[derive(Debug, Clone)]
pub struct WorldTerrain {
    surface: Arc<TerrainSurface>,
    binding: TerrainBinding,
    limits: LocomotionLimits,
}

impl TerrainSurface {
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
        let bytes = serde_json::to_vec(&data).map_err(|_| invalid())?;
        let digest = bytes.iter().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
        });
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
    pub fn highlands() -> Self {
        static SURFACE: OnceLock<Arc<TerrainSurface>> = OnceLock::new();
        Self {
            surface: SURFACE
                .get_or_init(|| Arc::new(highlands().clone()))
                .clone(),
            binding: TerrainBinding::highlands(),
            limits: LocomotionLimits::default(),
        }
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
        let limits = state.map_or_else(LocomotionLimits::default, |s| s.locomotion);
        let result = if let Some(data) = state.and_then(|s| s.data.as_ref()) {
            Self::new(data.clone(), limits)?
        } else {
            // Compatibility loader for old saves; never a fallback for other identities.
            if binding != TerrainBinding::highlands() {
                return Err(ScaffoldContractError::InvalidId);
            }
            limits.validate()?;
            Self {
                limits,
                ..Self::highlands()
            }
        };
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
            data: (self.binding.version == 2).then(|| self.surface.data()),
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
}
