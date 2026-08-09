use alife_core::{
    BiochemistryState, BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity, OrganismId,
    Tick, Vec3f, WorldEntityId,
};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessScenarioBuilder, HeadlessWorld, WorldEditorSpawnSpec, WorldObjectKind,
    WorldOrganismRecord,
};

fn record(organism_id: u64, world_entity_id: u64) -> WorldOrganismRecord {
    let genome = CreatureGenome::early_mammal_founder(
        0xE10_3200 + organism_id,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let biochemistry = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    WorldOrganismRecord::new(
        OrganismId(organism_id),
        WorldEntityId(world_entity_id),
        genome,
        phenotype,
        biochemistry,
        Tick::ZERO,
    )
    .unwrap()
}

fn malformed_record(record: WorldOrganismRecord) -> WorldOrganismRecord {
    let mut value = serde_json::to_value(record).unwrap();
    value["phenotype"]["source_genome_id"] = serde_json::json!(0);
    serde_json::from_value(value).unwrap()
}

fn world_with_agent_and_food() -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
    let world = HeadlessScenarioBuilder::new(3_101)
        .agent("agent", OrganismId(7), Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let agent = world.entity_id("agent").unwrap();
    let food = world.entity_id("food").unwrap();
    (world, agent, food)
}

#[test]
fn headless_world_registers_and_resolves_a_matching_record() {
    let (mut world, agent, _) = world_with_agent_and_food();
    assert!(world.organism_registry().is_empty());

    world
        .register_organism_record(record(7, agent.raw()))
        .unwrap();

    let registry = world.organism_registry();
    assert_eq!(registry.len(), 1);
    assert_eq!(
        registry.get(OrganismId(7)).unwrap().world_entity_id(),
        agent
    );
    assert_eq!(
        registry
            .get_by_world_entity_id(agent)
            .unwrap()
            .organism_id(),
        OrganismId(7)
    );
    world.validate_organism_bindings().unwrap();
}

#[test]
fn registration_rejects_invalid_bindings_and_duplicate_stable_ids_atomically() {
    let (mut world, agent, food) = world_with_agent_and_food();

    for invalid in [
        record(7, 999),
        record(7, food.raw()),
        record(8, agent.raw()),
        malformed_record(record(7, agent.raw())),
    ] {
        assert!(world.register_organism_record(invalid).is_err());
        assert!(world.organism_registry().is_empty());
    }

    world
        .register_organism_record(record(7, agent.raw()))
        .unwrap();
    let before = world
        .organism_registry()
        .get(OrganismId(7))
        .unwrap()
        .clone();

    for duplicate in [record(7, 999), record(8, agent.raw())] {
        assert!(world.register_organism_record(duplicate).is_err());
        assert_eq!(world.organism_registry().len(), 1);
        assert_eq!(world.organism_registry().get(OrganismId(7)), Some(&before));
    }
    world.validate_organism_bindings().unwrap();
}

#[test]
fn registered_agent_cannot_be_removed_through_world_or_editor_paths() {
    let (mut world, agent, _) = world_with_agent_and_food();
    world
        .register_organism_record(record(7, agent.raw()))
        .unwrap();
    let before_objects = world.object_snapshots();
    let before_record = world
        .organism_registry()
        .get(OrganismId(7))
        .unwrap()
        .clone();

    assert!(world.remove_organism(OrganismId(7)).is_err());
    assert_eq!(world.object_snapshots(), before_objects);
    assert_eq!(
        world.organism_registry().get(OrganismId(7)),
        Some(&before_record)
    );

    assert!(world.remove_agent_entity(agent).is_err());
    assert_eq!(world.object_snapshots(), before_objects);
    assert_eq!(
        world.organism_registry().get(OrganismId(7)),
        Some(&before_record)
    );

    assert!(world.editor_remove_object(agent).is_err());
    assert_eq!(world.object_snapshots(), before_objects);
    assert_eq!(
        world.organism_registry().get(OrganismId(7)),
        Some(&before_record)
    );
    world.validate_organism_bindings().unwrap();
}

#[test]
fn an_unregistered_legacy_agent_can_still_be_removed() {
    let mut world = HeadlessScenarioBuilder::new(3_102)
        .agent("legacy-agent", OrganismId(7), Vec3f::ZERO)
        .build()
        .unwrap();
    let removed = world.remove_organism(OrganismId(7)).unwrap();

    assert_eq!(removed.kind, WorldObjectKind::Agent);
    assert_eq!(removed.organism_id, Some(OrganismId(7)));
    assert_eq!(world.object_count(), 0);
    assert!(world.organism_registry().is_empty());
}

#[test]
fn duplicate_agent_spawns_reject_without_consuming_world_or_spawn_identity() {
    let mut world = HeadlessScenarioBuilder::new(3_103)
        .agent("agent", OrganismId(7), Vec3f::ZERO)
        .build()
        .unwrap();
    let before_objects = world.object_snapshots();
    let before_signature = world.canonical_signature_digest().unwrap();

    assert!(world
        .spawn_social_agent(
            "duplicate-raw",
            OrganismId(7),
            Vec3f::new(1.0, 0.0, 0.0),
            0.0
        )
        .is_err());
    assert!(world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "duplicate-editor".to_string(),
            kind: WorldObjectKind::Agent,
            organism_id: Some(OrganismId(7)),
            position: Vec3f::new(2.0, 0.0, 0.0),
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 1.0,
            token_id: None,
        })
        .is_err());

    assert_eq!(world.object_snapshots(), before_objects);
    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        before_signature
    );

    let next_agent = world
        .spawn_social_agent("next-agent", OrganismId(8), Vec3f::new(1.0, 0.0, 0.0), 0.0)
        .unwrap();
    assert_eq!(next_agent, WorldEntityId(2));
    assert_eq!(
        world
            .entity(next_agent)
            .unwrap()
            .tracking_provenance
            .spawn_sequence,
        2
    );

    let food = world
        .editor_spawn_object(WorldEditorSpawnSpec {
            label: "editor-food".to_string(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position: Vec3f::new(2.0, 0.0, 0.0),
            nutrition: 0.5,
            hazard_pain: 0.0,
            radius: 1.0,
            token_id: None,
        })
        .unwrap();
    assert_eq!(food, WorldEntityId(3));
}

#[test]
fn legacy_persistence_restore_starts_with_an_empty_registry() {
    let (mut world, agent, _) = world_with_agent_and_food();
    world
        .register_organism_record(record(7, agent.raw()))
        .unwrap();
    let before_objects = world.object_snapshots();
    let save = PortableSaveFile::from_headless_world(
        "task-3-1b-legacy",
        &world,
        RuntimeConfig::deterministic_default(3_101, alife_core::BrainScaleTier::Nano512),
        AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap();

    let restored = save.restore_headless_world().unwrap();
    assert_eq!(restored.object_snapshots(), before_objects);
    assert!(restored.organism_registry().is_empty());
    restored.validate_organism_bindings().unwrap();
}
