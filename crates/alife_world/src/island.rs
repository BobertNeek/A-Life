//! Approved island data. Sampling and movement use the ordinary world terrain solver.
use crate::{LocomotionLimits, TerrainData, WorldTerrain};
use std::sync::OnceLock;

pub fn island_terrain() -> WorldTerrain {
    static TERRAIN: OnceLock<WorldTerrain> = OnceLock::new();
    TERRAIN
        .get_or_init(|| {
            let bytes = include_bytes!("../assets/island-v1.bin");
            assert_eq!(&bytes[..8], b"HLAND001");
            let u32_at = |i| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap()) as usize;
            let f32_at = |i| f32::from_le_bytes(bytes[i..i + 4].try_into().unwrap());
            let width = u32_at(8);
            let depth = u32_at(12);
            let count = u32_at(28);
            assert_eq!(bytes.len(), 32 + width * depth * 4 + count * 24);
            WorldTerrain::new(
                TerrainData {
                    width,
                    depth,
                    origin_x: f32_at(16),
                    origin_z: f32_at(20),
                    spacing: f32_at(24),
                    heights: (0..width * depth).map(|i| f32_at(32 + i * 4)).collect(),
                    obstacles: (0..count)
                        .map(|i| {
                            std::array::from_fn(|j| f32_at(32 + width * depth * 4 + i * 24 + j * 4))
                        })
                        .collect(),
                    water_level: Some(0.0),
                },
                LocomotionLimits::default(),
            )
            .expect("validated approved island bake")
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn island_overview_ray_uses_the_requested_range() {
        let terrain = island_terrain();
        let surface = terrain.surface();
        let origin = Vec3f::new(80.0, 3500.0, 250.0);
        let direction = Vec3f::new(0.0, -1.0, 0.0);
        let hit = surface.ray_hit(origin, direction, 5000.0).unwrap();
        assert!((hit.y - surface.height(80.0, 250.0).unwrap()).abs() < 0.001);
        assert!(surface.ray_hit(origin, direction, 2000.0).is_none());
        assert!(surface
            .ray_hit(Vec3f::new(2000.0, 50.0, 250.0), direction, 5000.0)
            .is_none());
    }
    use crate::HeadlessScenarioBuilder;
    use alife_core::{OrganismId, Vec3f};

    #[test]
    fn island_new_game_and_restore_keep_the_selected_geometry() {
        let terrain = island_terrain();
        let surface = terrain.surface();
        assert_eq!((surface.width, surface.depth), (321, 385));
        assert!(surface.height(-500.0, -600.0).unwrap() < -5.0);
        assert!(surface.height(28.0, -275.0).unwrap() > 200.0);
        assert!(terrain.walkable(80.0, 250.0));
        assert_ne!(terrain.binding(), crate::TerrainBinding::highlands());
        let mut world = HeadlessScenarioBuilder::new(93026)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        world
            .enable_terrain_for_new_game(terrain.clone(), Vec3f::new(80.0, 0.0, 250.0))
            .unwrap();
        let object = world.entity(world.entity_id("walker").unwrap()).unwrap();
        assert_eq!(
            object.position.y,
            surface
                .height(object.position.x, object.position.z)
                .unwrap()
        );
        let restored = WorldTerrain::restore(Some(terrain.binding()), Some(&terrain.state()))
            .unwrap()
            .unwrap();
        assert_eq!(restored.binding(), terrain.binding());
        assert_eq!(
            restored.surface().height(28.0, -275.0),
            surface.height(28.0, -275.0)
        );
    }
}
