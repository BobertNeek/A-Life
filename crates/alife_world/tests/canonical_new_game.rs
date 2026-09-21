use alife_core::{
    BrainCapacityClass, FoundationGeneticIdentity, FoundationWeightAsset,
    N512FounderFoundationProjection, PhysicalContactKind, SensorProfile, Tick, Validate,
};
use alife_world::{
    create_canonical_new_game, CanonicalNewGameConfig, HeadlessWorldCommand, WorldObjectKind,
};

fn phase3_game(population: u16) -> alife_world::CanonicalNewGame {
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    create_canonical_new_game(
        &CanonicalNewGameConfig::phase3(240_824, population).unwrap(),
        &foundation,
    )
    .unwrap()
}

#[test]
fn canonical_new_game_creates_exact_requested_population() {
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();

    for population in [1, 4, 6, 8] {
        let game = create_canonical_new_game(
            &CanonicalNewGameConfig::phase3(240_824, population).unwrap(),
            &foundation,
        )
        .unwrap();

        assert_eq!(game.receipt.requested_population, population);
        assert_eq!(
            game.world.organism_registry().len(),
            usize::from(population)
        );
        assert_eq!(game.creatures.len(), usize::from(population));
        assert_eq!(game.receipt.founders.len(), usize::from(population));
    }
}

#[test]
fn canonical_new_game_rejects_population_outside_phase3_bounds() {
    assert!(CanonicalNewGameConfig::phase3(1, 0).is_err());
    assert!(CanonicalNewGameConfig::phase3(1, 9).is_err());
    assert!(CanonicalNewGameConfig::phase3(0, 6).is_err());
}

#[test]
fn canonical_new_game_binds_every_subsystem_before_admission() {
    let game = phase3_game(6);

    for founder in &game.receipt.founders {
        let record = game
            .world
            .organism_registry()
            .get(founder.organism_id)
            .unwrap();
        record.validate_contract().unwrap();
        assert_eq!(record.genome().id, founder.genome_id);
        assert_eq!(record.phenotype().source_genome_id, founder.genome_id);
        assert_eq!(record.state_graph().organism_id, founder.organism_id);
        assert_eq!(record.embodiment().entity_id(), founder.world_entity_id);
        assert!(game
            .world
            .habitat_authority()
            .membership(founder.organism_id)
            .is_some());
    }
}

#[test]
fn canonical_new_game_contains_live_ecology_not_frontend_fixtures() {
    let game = phase3_game(6);
    let objects = game.world.object_snapshots();

    assert_eq!(
        objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Food)
            .count(),
        1
    );
    assert_eq!(
        objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Hazard)
            .count(),
        1
    );
    assert_eq!(
        objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Obstacle)
            .count(),
        2
    );
    assert_eq!(game.world.ecology().resources.len(), 1);
    assert!(!game.world.ecology().zones.is_empty());
}

#[test]
fn canonical_new_game_meadow_is_safe_until_actual_hazard_contact() {
    // Every supported founder slot starts clear of danger, on both the flat
    // fixture and the terrain used by ordinary New Game. Food has a clear route.
    for elevated in [false, true] {
        let mut nursery = phase3_game(8);
        if elevated {
            nursery.world.enable_highlands_for_new_game().unwrap();
        }
        let hazard_id = nursery.world.entity_id("hazard-01").unwrap();
        let hazard = nursery.world.entity(hazard_id).unwrap().position;
        let food_id = nursery.world.entity_id("food-01").unwrap();
        let food = nursery.world.entity(food_id).unwrap().position;
        for founder in &nursery.receipt.founders {
            let position = nursery
                .world
                .entity(founder.world_entity_id)
                .unwrap()
                .position;
            let separation = ((position.x - hazard.x).powi(2)
                + (position.y - hazard.y).powi(2)
                + (position.z - hazard.z).powi(2))
            .sqrt();
            assert!(separation > 6.0, "hazard encroaches on a founder spawn");
            if let Some(terrain) = nursery.world.terrain() {
                assert!(terrain.walkable(position.x, position.z));
                assert!(
                    terrain.resolve_move(position, food).is_some(),
                    "food route blocked"
                );
            }
            let idle = HeadlessWorldCommand::idle(founder.organism_id).unwrap();
            let receipt = nursery
                .world
                .apply_registered_command(&idle, founder.world_entity_id, Tick(1))
                .unwrap();
            assert_eq!(receipt.action_result.body_event.damage, 0.0);
            assert_eq!(receipt.biology_after.body.health, 1.0);
        }
    }

    let mut game = phase3_game(1);
    let founder = &game.receipt.founders[0];
    let hazard = game.world.entity_id("hazard-01").unwrap();
    let command = HeadlessWorldCommand::approach(founder.organism_id, hazard).unwrap();

    let meadow_step = game
        .world
        .apply_registered_command(&command, founder.world_entity_id, Tick(1))
        .unwrap();
    assert!(meadow_step.action_result.execution.succeeded);
    assert_eq!(
        meadow_step.action_result.execution.physical.contact,
        PhysicalContactKind::Moved
    );
    assert_eq!(meadow_step.action_result.body_event.damage, 0.0);
    assert_eq!(meadow_step.biology_after.body.health, 1.0);
    game.world.try_advance_tick().unwrap();

    let mut hazard_step = None;
    for tick in 2..=32 {
        let step = game
            .world
            .apply_registered_command(&command, founder.world_entity_id, Tick(tick))
            .unwrap();
        assert!(step.action_result.execution.succeeded);
        if step.action_result.body_event.damage > 0.0 {
            hazard_step = Some(step);
            break;
        }
        assert_eq!(step.biology_after.body.health, 1.0);
        game.world.try_advance_tick().unwrap();
    }
    let hazard_step = hazard_step.expect("the remote hazard remains reachable");
    assert!(hazard_step.action_result.execution.succeeded);
    assert_eq!(
        hazard_step.action_result.execution.physical.contact,
        PhysicalContactKind::Collision
    );
    assert_eq!(hazard_step.action_result.body_event.damage, 0.12);
    assert!(hazard_step.biology_after.body.health < hazard_step.biology_before.body.health);
    game.world.try_advance_tick().unwrap();

    // Contact has a real consequence, but it does not trap the creature there.
    let food = game.world.entity_id("food-01").unwrap();
    let retreat = HeadlessWorldCommand::approach(founder.organism_id, food).unwrap();
    for _ in 0..3 {
        let next_tick = Tick(game.world.tick().raw() + 1);
        let step = game
            .world
            .apply_registered_command(&retreat, founder.world_entity_id, next_tick)
            .unwrap();
        assert!(step.action_result.execution.succeeded);
        game.world.try_advance_tick().unwrap();
    }
    let idle = HeadlessWorldCommand::idle(founder.organism_id).unwrap();
    let next_tick = Tick(game.world.tick().raw() + 1);
    let recovered = game
        .world
        .apply_registered_command(&idle, founder.world_entity_id, next_tick)
        .unwrap();
    assert_eq!(recovered.action_result.body_event.damage, 0.0);
}

#[test]
fn canonical_new_game_is_deterministic_and_rejects_the_wrong_foundation() {
    let first = phase3_game(6);
    let second = phase3_game(6);
    assert_eq!(first.receipt, second.receipt);
    assert_eq!(first.creatures, second.creatures);
    assert_eq!(
        first.world.canonical_signature_digest().unwrap(),
        second.world.canonical_signature_digest().unwrap()
    );

    let privileged =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::PrivilegedAffordanceV1).unwrap();
    let config = CanonicalNewGameConfig::phase3(240_824, 6).unwrap();
    assert!(create_canonical_new_game(&config, &privileged).is_err());
}

#[test]
fn canonical_new_game_founders_compile_against_the_checked_nano512_foundation() {
    let game = phase3_game(6);
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();

    for founder in &game.receipt.founders {
        let record = game
            .world
            .organism_registry()
            .get(founder.organism_id)
            .unwrap();
        assert_eq!(
            record.phenotype().foundation,
            FoundationGeneticIdentity::new(
                0x004E_3531_325F_5631,
                1,
                0x4E35_3132_5F00_FA11,
                BrainCapacityClass::N512_ID,
            )
            .unwrap()
        );
        assert_eq!(
            record.phenotype().source_genome_id,
            record.phenotype().brain_genome.id
        );
        assert_eq!(
            record.phenotype().brain_genome.lineage_id,
            Some(record.phenotype().lineage_id)
        );
        record.phenotype().brain_genome.validate_contract().unwrap();
        record
            .phenotype()
            .development_state_at(alife_core::Tick::ZERO)
            .unwrap();
        let projection = N512FounderFoundationProjection::compile(
            record.phenotype(),
            SensorProfile::GroundedObjectSlotsV1,
            &foundation,
        )
        .unwrap();
        assert_eq!(projection.source_genome_id(), founder.genome_id);
        assert_eq!(projection.lineage_id(), founder.lineage_id);
    }
}
