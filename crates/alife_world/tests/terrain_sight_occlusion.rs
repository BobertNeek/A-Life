use alife_core::{BrainScaleTier, HomeostaticSnapshot, OrganismId, SensorProfile, Vec3f};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessActionIds, HeadlessScenarioBuilder, LocomotionLimits, TerrainData,
    WorldEditorSpawnSpec, WorldObjectKind, WorldTerrain,
};

fn check_ridge(fine_grid: bool, ridge_height: f32) {
    for height in [0.0, ridge_height] {
        let (width, depth, origin_x, origin_z, spacing, peak, target_x, radius) = if fine_grid {
            (53, 81, -0.5, -1.0, 0.025, 29, 0.6, 0.1)
        } else {
            (5, 3, -3.125, -3.125, 3.125, 2, 6.25, 0.75)
        };
        let mut row = vec![0.0; width];
        row[peak] = height;
        let terrain = WorldTerrain::new(
            TerrainData {
                width,
                depth,
                origin_x,
                origin_z,
                spacing,
                heights: row.repeat(depth),
                obstacles: vec![],
                water_level: None,
            },
            LocomotionLimits::default(),
        )
        .unwrap();
        assert!(terrain.walkable(0.0, 0.0));
        assert!(terrain.walkable(target_x, 0.0));
        let mut world = HeadlessScenarioBuilder::new(4250)
            .agent("creature", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        world
            .enable_terrain_for_new_game(terrain, Vec3f::ZERO)
            .unwrap();
        let agent = world.entity_id("creature").unwrap();
        assert_eq!(world.entity(agent).unwrap().position, Vec3f::ZERO);
        let target = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "food".into(),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position: Vec3f::new(target_x, 0.0, 0.0),
                radius,
                nutrition: 0.5,
                hazard_pain: 0.0,
                token_id: None,
            })
            .unwrap();
        assert_eq!(
            world.entity(target).unwrap().position,
            Vec3f::new(target_x, 0.0, 0.0)
        );
        let save = PortableSaveFile::from_headless_world(
            "terrain-sight",
            &world,
            RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
            AssetManifest::empty(),
            vec![],
        )
        .unwrap();
        let mut world = PortableSaveFile::from_json_str(&save.to_json_string_pretty().unwrap())
            .unwrap()
            .restore_headless_world()
            .unwrap();
        let eye = Vec3f::new(0.0, 1.0, 0.0);
        let vertical = radius * 0.5 - eye.y;
        let range = target_x.hypot(vertical);
        let direction = Vec3f::new(target_x / range, vertical / range, 0.0);
        let surface = world.terrain().unwrap().surface();
        let hit = surface.ray_hit(eye, direction, range);
        let blocked_walk = world
            .terrain()
            .unwrap()
            .resolve_move(Vec3f::ZERO, Vec3f::new(target_x, 0.0, 0.0))
            .is_none();
        if height == 0.0 {
            assert!(hit.is_none());
        } else {
            let point = hit.expect("the representative sight ray intersects the ridge");
            let distance = point.x.hypot(point.y - eye.y);
            assert!(distance < range - radius * 0.25);
        }
        let tick = world.tick();
        let draft = world
            .perception_frame_draft(
                OrganismId(1),
                tick,
                SensorProfile::GroundedTerrainVisionV1,
                HomeostaticSnapshot::baseline(tick),
            )
            .unwrap();
        let approach = draft.candidates().iter().any(|candidate| {
            candidate.action_id == HeadlessActionIds::APPROACH
                && candidate.target.entity == Some(target)
        });
        if fine_grid {
            let forward_depth = draft.sensory().channels.visual_affordance[7];
            if height == 0.0 {
                assert_eq!(forward_depth, 1.0);
            } else {
                assert!(
                    forward_depth < 0.04,
                    "the nearby ridge must appear in the depth fan"
                );
            }
        }
        println!("fine_grid={fine_grid} peak={height} walk_blocked={blocked_walk} exact_hit={hit:?} visible_slots={} approach={approach}", draft.grounded_object_slots().len());
        assert_eq!(
            approach,
            height == 0.0,
            "fine_grid={fine_grid}, peak={height}"
        );
    }
}

#[test]
fn fine_grid_ridge_blocks_food_visibility() {
    check_ridge(true, 2.0);
}

#[test]
fn island_spacing_ridge_blocks_the_representative_food_sight_ray() {
    check_ridge(false, 0.7);
}

#[test]
fn shipped_island_ridge_is_occluding_in_canonical_and_indexed_perception() {
    let mut world = HeadlessScenarioBuilder::new(4251)
        .agent("observer", OrganismId(1), Vec3f::ZERO)
        .build()
        .unwrap();
    world.enable_island_for_new_game().unwrap();
    let observer = world.entity_id("observer").unwrap();
    let spawn = world.entity(observer).unwrap().position;
    let agent_position = Vec3f::new(-96.875, 0.0, -384.375);
    let food_position = Vec3f::new(-90.625, 0.0, -384.375);
    world.editor_move_object(observer, agent_position).unwrap();
    let target = world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "hidden food".into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: food_position,
            radius: 0.75,
            nutrition: 0.5,
            hazard_pain: 0.0,
            token_id: None,
        })
        .unwrap();
    for i in 0..128 {
        world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: format!("remote-{i}"),
                kind: WorldObjectKind::Food,
                organism_id: None,
                position: spawn,
                radius: 0.75,
                nutrition: 0.5,
                hazard_pain: 0.0,
                token_id: None,
            })
            .unwrap();
    }
    let a = world.entity(observer).unwrap().position;
    let b = world.entity(target).unwrap().position;
    assert_eq!((a.x, a.z), (agent_position.x, agent_position.z));
    assert_eq!((b.x, b.z), (food_position.x, food_position.z));
    let dy = b.y + 0.375 - a.y - 1.0;
    let length = (b.x - a.x).hypot(dy);
    let hit = world
        .terrain()
        .unwrap()
        .surface()
        .ray_hit(
            Vec3f::new(a.x, a.y + 1.0, a.z),
            Vec3f::new((b.x - a.x) / length, dy / length, 0.0),
            length,
        )
        .expect("the shipped ridge intersects the representative sight ray");
    assert!(hit.x > a.x && hit.x < b.x - 0.1875);
    let tick = world.tick();
    let mut indexed_world = world.clone();
    let index = indexed_world.build_perception_batch_index().unwrap();
    let canonical = world
        .perception_frame_draft(
            OrganismId(1),
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            HomeostaticSnapshot::baseline(tick),
        )
        .unwrap();
    let indexed = indexed_world
        .perception_frame_draft_indexed(
            OrganismId(1),
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            HomeostaticSnapshot::baseline(tick),
            &index,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&canonical).unwrap(),
        serde_json::to_vec(&indexed).unwrap()
    );
    assert!(!canonical
        .candidates()
        .iter()
        .any(|c| c.target.entity == Some(target)));
    assert!(canonical.grounded_object_slots().is_empty());
}
