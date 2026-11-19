//! Opt-in, in-memory compatibility receipt for the isolated study. No user profile,
//! training fixture, or default new-game mapping participates.
use alife_core::Vec3f;
use alife_world::{
    island_terrain, persistence::WorldSaveState, HeadlessScenarioBuilder, WorldTerrain,
};

fn prototype() -> WorldTerrain {
    let raw = include_bytes!("../../../studies/island-ridge-v2/island-ridge-v2.bin");
    assert_eq!(&raw[..8], b"HLAND001");
    let f = |i| f32::from_le_bytes(raw[i..i + 4].try_into().unwrap());
    let u = |i| u32::from_le_bytes(raw[i..i + 4].try_into().unwrap()) as usize;
    let old = island_terrain();
    let mut data = old.surface().data();
    assert_eq!((u(8), u(12)), (data.width, data.depth));
    assert_eq!((f(16), f(20), f(24)), (data.origin_x, data.origin_z, data.spacing));
    assert_eq!(u(28), data.obstacles.len());
    let off = 32 + data.heights.len() * 4;
    assert_eq!(raw.len(), off + data.obstacles.len() * 24);
    for (i, sample) in data.heights.iter_mut().enumerate() {
        *sample = f(32 + i * 4);
    }
    for (i, b) in data.obstacles.iter_mut().enumerate() {
        *b = std::array::from_fn(|j| f(off + i * 24 + j * 4));
    }
    WorldTerrain::new(data, old.limits()).unwrap()
}

fn save(terrain: Option<&WorldTerrain>) -> WorldSaveState {
    let mut world = HeadlessScenarioBuilder::new(412).build().unwrap();
    if let Some(map) = terrain {
        world
            .enable_terrain_for_new_game(map.clone(), Vec3f::new(80.0, 0.0, 250.0))
            .unwrap();
    } else {
        world.enable_island_for_new_game().unwrap();
    }
    serde_json::from_slice(&serde_json::to_vec(&world.save_state()).unwrap()).unwrap()
}

#[test]
#[ignore = "isolated physical terrain study"]
fn independent_ridge_and_original_terrain_survive_exact_save_restore() {
    let legacy = island_terrain();
    let original_save = save(None);
    let revised = prototype();
    let revised_save = save(Some(&revised));
    assert_ne!(legacy.binding(), revised.binding());
    assert_eq!(original_save.terrain, Some(legacy.binding()));
    assert_eq!(revised_save.terrain, Some(revised.binding()));
    for (source, expected) in [(&revised_save, &revised), (&original_save, &legacy)] {
        let restored_world = source.restore().unwrap();
        let restored = restored_world.terrain().unwrap();
        assert_eq!(restored.binding(), expected.binding());
        assert_eq!(restored.state(), expected.state());
        for (x, z) in [(80.0, 250.0), (-100.0, -203.125), (28.125, -275.0)] {
            let h = restored.surface().height(x, z).unwrap();
            let hit = restored
                .surface()
                .ray_hit(Vec3f::new(x, h + 10.0, z), Vec3f::new(0.0, -1.0, 0.0), 20.0)
                .unwrap();
            assert!((h - hit.y).abs() < 0.001);
        }
        assert!(restored.walkable(80.0, 250.0));
    }
    let mut mismatched = revised_save;
    mismatched.terrain = original_save.terrain;
    assert!(mismatched.restore().is_err());
    assert_eq!(save(None).terrain, Some(legacy.binding()));
}
