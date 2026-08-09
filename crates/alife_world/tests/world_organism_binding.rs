use alife_core::{
    BiochemistryState, Blake3Digest, BodyEventDelta, BrainCapacityClass, CreatureGenome,
    FoundationGeneticIdentity, OrganismId, Tick, Vec3f, WorldEntityId,
};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessScenarioBuilder, HeadlessWorld, HeadlessWorldSignatureDigest, WorldEditorSpawnSpec,
    WorldObject, WorldObjectKind, WorldOrganismRecord,
};

#[derive(Debug, PartialEq)]
struct WorldReceipt {
    objects: Vec<WorldObject>,
    registry: Vec<(
        WorldOrganismRecord,
        Option<WorldOrganismRecord>,
        Option<WorldOrganismRecord>,
    )>,
    tick: Tick,
    signature: HeadlessWorldSignatureDigest,
}

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

fn world_with_two_agents_and_food() -> (HeadlessWorld, WorldEntityId, WorldEntityId, WorldEntityId)
{
    let world = HeadlessScenarioBuilder::new(3_104)
        .agent("agent-a", OrganismId(7), Vec3f::ZERO)
        .agent("agent-b", OrganismId(8), Vec3f::new(4.0, 0.0, 0.0))
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let agent_a = world.entity_id("agent-a").unwrap();
    let agent_b = world.entity_id("agent-b").unwrap();
    let food = world.entity_id("food").unwrap();
    (world, agent_a, agent_b, food)
}

fn registry_receipt(
    world: &HeadlessWorld,
) -> Vec<(
    WorldOrganismRecord,
    Option<WorldOrganismRecord>,
    Option<WorldOrganismRecord>,
)> {
    let mut records: Vec<_> = world.organism_registry().iter().cloned().collect();
    records.sort_by_key(|record| record.organism_id().raw());
    records
        .into_iter()
        .map(|record| {
            let by_organism = world.organism_registry().get(record.organism_id()).cloned();
            let by_entity = world
                .organism_registry()
                .get_by_world_entity_id(record.world_entity_id())
                .cloned();
            (record, by_organism, by_entity)
        })
        .collect()
}

fn receipt(world: &HeadlessWorld) -> WorldReceipt {
    WorldReceipt {
        objects: world.object_snapshots(),
        registry: registry_receipt(world),
        tick: world.tick(),
        signature: world.canonical_signature_digest().unwrap(),
    }
}

fn assert_unchanged(world: &HeadlessWorld, before: &WorldReceipt) {
    assert_eq!(
        receipt(world),
        *before,
        "failed exact replacement must not publish partial state"
    );
}

fn world_with_agent_without_organism_id() -> HeadlessWorld {
    let (world, _, _) = world_with_agent_and_food();
    let save = PortableSaveFile::from_headless_world(
        "task-3-2b3b1-agent-none",
        &world,
        RuntimeConfig::deterministic_default(3_101, alife_core::BrainScaleTier::Nano512),
        AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap();
    let mut value = serde_json::to_value(save).unwrap();
    let objects = value["world"]["objects"].as_array_mut().unwrap();
    let agent = objects
        .iter_mut()
        .find(|object| object["label"] == "agent")
        .unwrap();
    agent["organism_id"] = serde_json::Value::Null;
    let save = PortableSaveFile::from_json_str(&serde_json::to_string(&value).unwrap()).unwrap();
    save.restore_headless_world().unwrap()
}

fn world_with_non_agent_organism_id() -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
    let (world, _, _) = world_with_agent_and_food();
    let save = PortableSaveFile::from_headless_world(
        "task-3-2b3b1-food-id",
        &world,
        RuntimeConfig::deterministic_default(3_101, alife_core::BrainScaleTier::Nano512),
        AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap();
    let mut value = serde_json::to_value(save).unwrap();
    let objects = value["world"]["objects"].as_array_mut().unwrap();
    let food = objects
        .iter_mut()
        .find(|object| object["label"] == "food")
        .unwrap();
    food["organism_id"] = serde_json::json!(99);
    let save = PortableSaveFile::from_json_str(&serde_json::to_string(&value).unwrap()).unwrap();
    let world = save.restore_headless_world().unwrap();
    let agent = world.entity_id("agent").unwrap();
    let food = world.entity_id("food").unwrap();
    (world, agent, food)
}

fn world_with_duplicate_agent_organism_id() -> HeadlessWorld {
    let (world, _, _, _) = world_with_two_agents_and_food();
    let save = PortableSaveFile::from_headless_world(
        "task-3-2b3b1-duplicate-agent-id",
        &world,
        RuntimeConfig::deterministic_default(3_104, alife_core::BrainScaleTier::Nano512),
        AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap();
    let mut value = serde_json::to_value(save).unwrap();
    let objects = value["world"]["objects"].as_array_mut().unwrap();
    let second_agent = objects
        .iter_mut()
        .find(|object| object["label"] == "agent-b")
        .unwrap();
    second_agent["organism_id"] = serde_json::json!(7);
    let save = PortableSaveFile::from_json_str(&serde_json::to_string(&value).unwrap()).unwrap();
    save.restore_headless_world().unwrap()
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
fn replace_registry_accepts_the_exact_complete_agent_set() {
    let (mut world, agent_a, agent_b, food) = world_with_two_agents_and_food();
    let empty_signature = world.canonical_signature_digest().unwrap();

    world
        .replace_organism_registry_exact(
            [record(7, agent_a.raw()), record(8, agent_b.raw())].into_iter(),
        )
        .unwrap();

    assert_eq!(world.organism_registry().len(), 2);
    assert_eq!(
        world
            .organism_registry()
            .get(OrganismId(7))
            .unwrap()
            .world_entity_id(),
        agent_a
    );
    assert_eq!(
        world
            .organism_registry()
            .get_by_world_entity_id(agent_b)
            .unwrap()
            .organism_id(),
        OrganismId(8)
    );
    assert_eq!(world.entity(food).unwrap().kind, WorldObjectKind::Food);
    assert_ne!(world.canonical_signature_digest().unwrap(), empty_signature);
}

#[test]
fn replace_registry_rejects_malformed_record_atomically() {
    let (mut world, agent, _) = world_with_agent_and_food();
    let before = receipt(&world);

    assert!(world
        .replace_organism_registry_exact([malformed_record(record(7, agent.raw()))].into_iter())
        .is_err());
    assert_unchanged(&world, &before);
}

#[test]
fn replace_registry_rejects_duplicate_organism_atomically() {
    let (mut world, agent_a, agent_b, _) = world_with_two_agents_and_food();
    let before = receipt(&world);

    assert!(world
        .replace_organism_registry_exact(
            [record(7, agent_a.raw()), record(7, agent_b.raw())].into_iter(),
        )
        .is_err());
    assert_unchanged(&world, &before);
}

#[test]
fn replace_registry_rejects_duplicate_world_entity_atomically() {
    let (mut world, agent_a, _, _) = world_with_two_agents_and_food();
    let before = receipt(&world);

    assert!(world
        .replace_organism_registry_exact(
            [record(7, agent_a.raw()), record(8, agent_a.raw())].into_iter(),
        )
        .is_err());
    assert_unchanged(&world, &before);
}

#[test]
fn replace_registry_rejects_missing_agent_record_atomically() {
    let (mut world, agent_a, _, _) = world_with_two_agents_and_food();
    let before = receipt(&world);

    assert!(world
        .replace_organism_registry_exact([record(7, agent_a.raw())].into_iter())
        .is_err());
    assert_unchanged(&world, &before);
}

#[test]
fn replace_registry_rejects_extra_record_atomically() {
    let (mut world, agent, _) = world_with_agent_and_food();
    let before = receipt(&world);

    assert!(world
        .replace_organism_registry_exact([record(7, agent.raw()), record(8, 999)].into_iter(),)
        .is_err());
    assert_unchanged(&world, &before);
}

#[test]
fn replace_registry_rejects_missing_object_wrong_kind_and_wrong_binding_atomically() {
    let (mut world, _, agent_b, food) = world_with_two_agents_and_food();
    let before = receipt(&world);

    for invalid in [
        [record(7, 999)],
        [record(7, food.raw())],
        [record(7, agent_b.raw())],
    ] {
        assert!(world
            .replace_organism_registry_exact(invalid.into_iter())
            .is_err());
        assert_unchanged(&world, &before);
    }
}

#[test]
fn replace_registry_rejects_agent_without_organism_id_atomically() {
    let mut world = world_with_agent_without_organism_id();
    let before = receipt(&world);

    assert!(world
        .replace_organism_registry_exact(std::iter::empty::<WorldOrganismRecord>())
        .is_err());
    assert_unchanged(&world, &before);
}

#[test]
fn replace_registry_rejects_duplicate_agent_organism_identity_atomically() {
    let mut world = world_with_duplicate_agent_organism_id();
    let agent_a = world.entity_id("agent-a").unwrap();
    let before = receipt(&world);

    assert!(world
        .replace_organism_registry_exact([record(7, agent_a.raw())].into_iter())
        .is_err());
    assert_unchanged(&world, &before);
}

#[test]
fn replace_registry_excludes_non_agent_organism_ids_from_reverse_cohort() {
    let (mut world, agent, food) = world_with_non_agent_organism_id();

    world
        .replace_organism_registry_exact([record(7, agent.raw())].into_iter())
        .unwrap();

    assert_eq!(
        world.entity(food).unwrap().organism_id,
        Some(OrganismId(99))
    );
    assert_eq!(world.organism_registry().len(), 1);
    world.validate_organism_bindings().unwrap();
}

#[test]
fn replace_registry_accepts_valid_biology_lifecycle_archive_state() {
    let (mut world, agent, _) = world_with_agent_and_food();
    let mut changed = record(7, agent.raw());
    changed
        .advance_biology(Tick(1), BodyEventDelta::zero())
        .unwrap();
    changed.mark_dead(Tick(1)).unwrap();
    changed
        .link_birth_manifest(Blake3Digest::from_bytes([1; 32]))
        .unwrap();
    changed
        .link_life_manifest(Blake3Digest::from_bytes([2; 32]))
        .unwrap();

    world
        .replace_organism_registry_exact([changed].into_iter())
        .unwrap();
    assert_eq!(world.organism_registry().len(), 1);
    assert_eq!(
        world
            .organism_registry()
            .get(OrganismId(7))
            .unwrap()
            .lifecycle(),
        alife_world::OrganismLifecycle::Dead {
            death_tick: Tick(1)
        }
    );
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
