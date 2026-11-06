use alife_core::{
    ActionCommand, ActionKind, ActionTarget, Confidence, DurationTicks, Intensity, OrganismId,
    Vec3f,
};
use alife_world::{
    HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld, LocomotionLimits, TerrainData,
    WorldTerrain,
};

fn fixture(terrain: bool, radius: f32) -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(4271)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .food("food", Vec3f::ZERO, 0.5)
        .toy("ball", Vec3f::ZERO, true)
        .token("token", Vec3f::ZERO, 1)
        .obstacle("rock", Vec3f::new(2.0, 0.0, 0.0), radius)
        .build()
        .unwrap();
    if terrain {
        world
            .enable_terrain_for_new_game(
                WorldTerrain::new(
                    TerrainData {
                        width: 17,
                        depth: 9,
                        origin_x: -2.0,
                        origin_z: -2.0,
                        spacing: 0.5,
                        heights: vec![0.0; 153],
                        obstacles: vec![],
                        water_level: None,
                    },
                    LocomotionLimits::default(),
                )
                .unwrap(),
                Vec3f::ZERO,
            )
            .unwrap();
    }
    world
}
fn step() -> ActionCommand {
    ActionCommand::structured(
        OrganismId(1),
        HeadlessActionIds::STEP_FORWARD,
        ActionKind::Move,
        ActionTarget::new(None, None),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(1.0).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap()
}

#[test]
fn player_rejects_new_rock_overlaps_without_mutating_the_hold_or_world() {
    for terrain in [false, true] {
        for radius in [0.01, 1.25] {
            for label in ["creature", "food", "ball", "token"] {
                let mut world = fixture(terrain, radius);
                let held = world.entity_id(label).unwrap();
                world.begin_player_hold(held).unwrap();
                let before = world.canonical_signature_digest().unwrap();
                for destination in [
                    Vec3f::new(2.0, 0.0, 0.0),
                    Vec3f::new(2.0 + radius.max(0.75), 0.0, 0.0),
                ] {
                    assert!(world.move_player_hold(destination).is_err());
                    assert_eq!(world.canonical_signature_digest().unwrap(), before);
                    assert_eq!(world.player_held_object(), Some(held));
                }
                // Releasing after a rejected move uses the previous safe ground.
                world.release_player_hold().unwrap();
                assert_eq!(world.entity(held).unwrap().position, Vec3f::ZERO);
            }
        }
    }
}

#[test]
fn carrying_can_cross_rocks_and_can_rescue_an_existing_overlap() {
    for terrain in [false, true] {
        for overlap in [false, true] {
            let mut world = fixture(terrain, 0.75);
            let creature = world.entity_id("creature").unwrap();
            if overlap {
                world
                    .editor_move_object(creature, Vec3f::new(2.0, 0.0, 0.1))
                    .unwrap();
            }
            world.begin_player_hold(creature).unwrap();
            let destination = Vec3f::new(4.0, 0.0, 0.0);
            world.move_player_hold(destination).unwrap();
            world.release_player_hold().unwrap();
            assert_eq!(world.entity(creature).unwrap().position, destination);
            assert!(world.apply_command(&step()).unwrap().execution.succeeded);
        }
    }
}

#[test]
fn positions_just_outside_the_existing_collision_radius_remain_legal() {
    for terrain in [false, true] {
        let mut world = fixture(terrain, 0.01);
        let creature = world.entity_id("creature").unwrap();
        world.begin_player_hold(creature).unwrap();
        let destination = Vec3f::new(f32::from_bits(2.75_f32.to_bits() + 1), 0.0, 0.0);
        world.move_player_hold(destination).unwrap();
        world.release_player_hold().unwrap();
        assert_eq!(world.entity(creature).unwrap().position, destination);
        assert!(world.apply_command(&step()).unwrap().execution.succeeded);
    }
}

#[test]
fn rejected_drop_preserves_registered_state_and_a_restorable_releasable_hold() {
    use alife_core::{
        BrainCapacityClass, BrainScaleTier, CreatureGenome, FoundationGeneticIdentity, Tick,
    };
    use alife_world::{AssetManifest, PortableSaveFile, RuntimeConfig, WorldOrganismRecord};
    for terrain in [false, true] {
        let mut world = fixture(terrain, 0.75);
        let creature = world.entity_id("creature").unwrap();
        let genome = CreatureGenome::early_mammal_founder(
            4271,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::newborn(
                    OrganismId(1),
                    creature,
                    genome,
                    phenotype,
                    Tick::ZERO,
                )
                .unwrap(),
            )
            .unwrap();
        let record = world
            .organism_registry()
            .get(OrganismId(1))
            .unwrap()
            .clone();
        world.begin_player_hold(creature).unwrap();
        let before = world.canonical_signature_digest().unwrap();
        assert!(world.move_player_hold(Vec3f::new(2.0, 0.0, 0.0)).is_err());
        let file = PortableSaveFile::from_headless_world(
            "player-drop-obstacle",
            &world,
            RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
            AssetManifest::empty(),
            vec![],
        )
        .unwrap();
        let mut restored = PortableSaveFile::from_json_str(&file.to_json_string_pretty().unwrap())
            .unwrap()
            .restore_headless_world()
            .unwrap();
        assert_eq!(restored.canonical_signature_digest().unwrap(), before);
        assert_eq!(
            restored.organism_registry().get(OrganismId(1)),
            Some(&record)
        );
        assert_eq!(restored.player_held_object(), Some(creature));
        assert_eq!(restored.release_player_hold().unwrap(), Some(creature));
        assert_eq!(restored.entity(creature).unwrap().position, Vec3f::ZERO);
        assert!(restored.apply_command(&step()).unwrap().execution.succeeded);
    }
}
