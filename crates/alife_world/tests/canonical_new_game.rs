use alife_core::{FoundationWeightAsset, SensorProfile};
use alife_world::{create_canonical_new_game, CanonicalNewGameConfig, WorldObjectKind};

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

    for population in [4, 6, 8] {
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
    assert!(CanonicalNewGameConfig::phase3(1, 3).is_err());
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
        8
    );
    assert_eq!(
        objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Hazard)
            .count(),
        2
    );
    assert_eq!(
        objects
            .iter()
            .filter(|object| object.kind == WorldObjectKind::Obstacle)
            .count(),
        2
    );
    assert_eq!(game.world.ecology().resources.len(), 2);
    assert!(!game.world.ecology().zones.is_empty());
    assert!(!game.world.ecology().spawn_policies.is_empty());
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
