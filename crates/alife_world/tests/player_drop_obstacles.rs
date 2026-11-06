use alife_core::{
    ActionCommand, ActionId, ActionKind, ActionTarget, Confidence, DurationTicks, Intensity,
    OrganismId, Vec3f, WorldEntityId,
};
use alife_world::{
    HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld, LocomotionLimits, TerrainData,
    WorldTerrain,
};

fn fixture(terrain: bool, radius: f32) -> HeadlessWorld {
    fixture_with_boxes(terrain, radius, vec![])
}
fn fixture_with_boxes(terrain: bool, radius: f32, obstacles: Vec<[f32; 6]>) -> HeadlessWorld {
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
                        obstacles,
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
    command(HeadlessActionIds::STEP_FORWARD, ActionKind::Move, None)
}
fn command(action: ActionId, kind: ActionKind, target: Option<WorldEntityId>) -> ActionCommand {
    ActionCommand::structured(
        OrganismId(1),
        action,
        kind,
        ActionTarget::new(target, None),
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
fn equivalent_projected_ground_requests_preserve_existing_overlap_recovery() {
    for terrain in [false, true] {
        let mut world = fixture(terrain, 0.75);
        let creature = world.entity_id("creature").unwrap();
        world
            .editor_move_object(creature, Vec3f::new(2.0, 0.0, 0.1))
            .unwrap();
        world.begin_player_hold(creature).unwrap();
        let before = world.canonical_signature_digest().unwrap();
        for discarded in [-3.0, 0.1, 2.0] {
            let input = if terrain {
                Vec3f::new(2.0, discarded, 0.1)
            } else {
                Vec3f::new(2.0, 0.0, discarded)
            };
            world.move_player_hold(input).unwrap();
            assert_eq!(world.canonical_signature_digest().unwrap(), before);
        }
        world.move_player_hold(Vec3f::new(4.0, 0.0, 0.0)).unwrap();
        world.release_player_hold().unwrap();
        assert!(world.apply_command(&step()).unwrap().execution.succeeded);
    }
}

fn grab_food(world: &mut HeadlessWorld) -> (WorldEntityId, WorldEntityId) {
    let creature = world.entity_id("creature").unwrap();
    let food = world.entity_id("food").unwrap();
    assert!(
        world
            .apply_command(&command(
                HeadlessActionIds::GRAB,
                ActionKind::Hold,
                Some(food)
            ))
            .unwrap()
            .execution
            .succeeded
    );
    (creature, food)
}

#[test]
fn new_ground_must_leave_carried_food_outside_solid_rock() {
    for terrain in [false, true] {
        // The body alone fits here and can turn. Its held-food grip does not.
        let mut control = fixture(terrain, 0.75);
        let creature = control.entity_id("creature").unwrap();
        control.begin_player_hold(creature).unwrap();
        control.move_player_hold(Vec3f::new(1.2, 0.0, 0.0)).unwrap();
        control.release_player_hold().unwrap();
        assert!(
            control
                .apply_command(&command(
                    HeadlessActionIds::TURN_LEFT,
                    ActionKind::Look,
                    None
                ))
                .unwrap()
                .execution
                .succeeded
        );

        let mut world = fixture(terrain, 0.75);
        let (creature, food) = grab_food(&mut world);
        world.begin_player_hold(creature).unwrap();
        let before = world.canonical_signature_digest().unwrap();
        assert!(world.move_player_hold(Vec3f::new(1.2, 0.0, 0.0)).is_err());
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
        assert_eq!(world.entity(food).unwrap().carried_by, Some(OrganismId(1)));
        world.release_player_hold().unwrap();
        assert!(
            world
                .apply_command(&command(
                    HeadlessActionIds::TURN_LEFT,
                    ActionKind::Look,
                    None
                ))
                .unwrap()
                .execution
                .succeeded
        );
    }
}

#[test]
fn carried_reach_crossing_a_sphere_is_rejected_even_when_both_endpoints_fit() {
    for terrain in [false, true] {
        let mut world = fixture(terrain, 0.75);
        let rock = world.entity_id("rock").unwrap();
        world
            .editor_move_object(rock, Vec3f::new(1.45, 0.0, 0.74))
            .unwrap();
        let (creature, _) = grab_food(&mut world);
        world.begin_player_hold(creature).unwrap();
        let before = world.canonical_signature_digest().unwrap();
        assert!(world.move_player_hold(Vec3f::new(1.2, 0.0, 0.0)).is_err());
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
    }
}

#[test]
fn carried_reach_respects_terrain_boxes_without_sweeping_the_air_carry() {
    let mut world = fixture_with_boxes(true, 0.75, vec![[1.65, -0.25, 1.66, 0.25, -0.5, 2.0]]);
    let rock = world.entity_id("rock").unwrap();
    world
        .editor_move_object(rock, Vec3f::new(5.5, 0.0, 1.0))
        .unwrap();
    let (creature, food) = grab_food(&mut world);
    world.begin_player_hold(creature).unwrap();
    let before = world.canonical_signature_digest().unwrap();
    assert!(world.move_player_hold(Vec3f::new(1.2, 0.0, 0.0)).is_err());
    assert_eq!(world.canonical_signature_digest().unwrap(), before);
    world.move_player_hold(Vec3f::new(4.0, 0.0, 0.0)).unwrap();
    world.release_player_hold().unwrap();
    assert_eq!(
        world.entity(food).unwrap().position,
        Vec3f::new(4.5, 0.0, 0.0)
    );
    assert!(world.apply_command(&step()).unwrap().execution.succeeded);
    assert!(
        world
            .apply_command(&command(
                HeadlessActionIds::TURN_LEFT,
                ActionKind::Look,
                None
            ))
            .unwrap()
            .execution
            .succeeded
    );
}

#[test]
fn player_can_carry_food_across_rocks_and_rescue_an_existing_grip_overlap() {
    for terrain in [false, true] {
        for existing_overlap in [false, true] {
            let mut world = fixture(terrain, 0.75);
            let (creature, food) = grab_food(&mut world);
            if existing_overlap {
                world
                    .editor_move_object(creature, Vec3f::new(1.2, 0.0, 0.0))
                    .unwrap();
            }
            world.begin_player_hold(creature).unwrap();
            if existing_overlap {
                let before = world.canonical_signature_digest().unwrap();
                let equivalent = if terrain {
                    Vec3f::new(1.2, 1.0, 0.0)
                } else {
                    Vec3f::new(1.2, 0.0, 1.0)
                };
                world.move_player_hold(equivalent).unwrap();
                assert_eq!(world.canonical_signature_digest().unwrap(), before);
            }
            world.move_player_hold(Vec3f::new(4.0, 0.0, 0.0)).unwrap();
            world.release_player_hold().unwrap();
            assert_eq!(
                world.entity(food).unwrap().position,
                Vec3f::new(4.5, 0.0, 0.0)
            );
            assert_eq!(world.entity(food).unwrap().carried_by, Some(OrganismId(1)));
            assert!(world.apply_command(&step()).unwrap().execution.succeeded);
            assert!(
                world
                    .apply_command(&command(
                        HeadlessActionIds::TURN_LEFT,
                        ActionKind::Look,
                        None
                    ))
                    .unwrap()
                    .execution
                    .succeeded
            );
        }
    }
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
        let (_, food) = grab_food(&mut world);
        let record = world
            .organism_registry()
            .get(OrganismId(1))
            .unwrap()
            .clone();
        world.begin_player_hold(creature).unwrap();
        let before = world.canonical_signature_digest().unwrap();
        assert!(world.move_player_hold(Vec3f::new(1.2, 0.0, 0.0)).is_err());
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
        assert_eq!(
            restored.entity(food).unwrap().carried_by,
            Some(OrganismId(1))
        );
        assert_eq!(restored.release_player_hold().unwrap(), Some(creature));
        assert_eq!(restored.entity(creature).unwrap().position, Vec3f::ZERO);
        assert_eq!(
            restored.entity(food).unwrap().position,
            Vec3f::new(0.5, 0.0, 0.0)
        );
        assert!(restored.apply_command(&step()).unwrap().execution.succeeded);
    }
}
