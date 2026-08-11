use alife_core::{
    BiochemistryState, Blake3Digest, BodyEventDelta, BrainCapacityClass, BrainScaleTier,
    CreatureGenome, FoundationGeneticIdentity, OrganismId, ScaffoldContractError, Tick, Vec3f,
    WorldEntityId,
};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessScenarioBuilder, HeadlessWorld, WorldOrganismRecord,
};

fn record(organism_id: u64, world_entity_id: u64) -> WorldOrganismRecord {
    let genome = CreatureGenome::early_mammal_founder(
        0xE10_3300_u64.wrapping_add(organism_id),
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

fn save(world: &HeadlessWorld) -> PortableSaveFile {
    PortableSaveFile::from_headless_world(
        "task-4-3a1-allocator",
        world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap()
}

fn world_with_two_agents() -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
    let world = HeadlessScenarioBuilder::new(44_003)
        .agent("resident-a", OrganismId(1), Vec3f::ZERO)
        .agent("resident-b", OrganismId(2), Vec3f::new(4.0, 0.0, 0.0))
        .build()
        .unwrap();
    let resident_a = world.entity_id("resident-a").unwrap();
    let resident_b = world.entity_id("resident-b").unwrap();
    (world, resident_a, resident_b)
}

#[test]
fn canonical_signature_distinguishes_same_seed_wrong_and_later_worlds() {
    let original = HeadlessScenarioBuilder::new(44_001)
        .agent("resident", OrganismId(1), Vec3f::ZERO)
        .food("resource", Vec3f::new(2.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let exact_clone = original.clone();
    let wrong_world = HeadlessScenarioBuilder::new(44_001)
        .agent("resident", OrganismId(1), Vec3f::ZERO)
        .food("resource", Vec3f::new(3.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let mut later_world = original.clone();
    later_world.advance_tick();

    assert_eq!(
        original.canonical_signature_digest().unwrap(),
        exact_clone.canonical_signature_digest().unwrap()
    );
    assert_ne!(
        original.canonical_signature_digest().unwrap(),
        wrong_world.canonical_signature_digest().unwrap()
    );
    assert_ne!(
        original.canonical_signature_digest().unwrap(),
        later_world.canonical_signature_digest().unwrap()
    );
}

#[test]
fn organism_registration_advances_allocator_and_overflow_is_atomic() {
    let mut world = HeadlessScenarioBuilder::new(44_004)
        .agent("organism-2", OrganismId(2), Vec3f::ZERO)
        .agent("organism-900", OrganismId(900), Vec3f::new(4.0, 0.0, 0.0))
        .build()
        .unwrap();
    let organism_2 = world.entity_id("organism-2").unwrap();
    let organism_900 = world.entity_id("organism-900").unwrap();

    world
        .register_organism_record(record(2, organism_2.raw()))
        .unwrap();
    assert_eq!(save(&world).world.next_organism_id, 3);
    world
        .register_organism_record(record(900, organism_900.raw()))
        .unwrap();
    assert_eq!(save(&world).world.next_organism_id, 901);

    let almost_exhausted = u64::MAX - 1;
    let exhausted = u64::MAX;
    let mut overflow_world = HeadlessScenarioBuilder::new(44_005)
        .agent("almost-exhausted", OrganismId(almost_exhausted), Vec3f::ZERO)
        .agent("exhausted", OrganismId(exhausted), Vec3f::new(4.0, 0.0, 0.0))
        .build()
        .unwrap();
    let almost_entity = overflow_world.entity_id("almost-exhausted").unwrap();
    let exhausted_entity = overflow_world.entity_id("exhausted").unwrap();
    overflow_world
        .register_organism_record(record(almost_exhausted, almost_entity.raw()))
        .unwrap();
    let before_signature = overflow_world.canonical_signature_digest().unwrap();
    let before_records: Vec<_> = overflow_world.organism_registry().iter().cloned().collect();

    assert_eq!(
        overflow_world.register_organism_record(record(exhausted, exhausted_entity.raw())),
        Err(ScaffoldContractError::InvalidId)
    );
    assert_eq!(
        overflow_world.canonical_signature_digest().unwrap(),
        before_signature
    );
    let after_records: Vec<_> = overflow_world.organism_registry().iter().cloned().collect();
    assert_eq!(after_records, before_records);
}

#[test]
fn organism_allocator_round_trip_preserves_retired_high_id_and_legacy_derives_max_plus_one() {
    let mut current = HeadlessScenarioBuilder::new(44_006)
        .agent("resident", OrganismId(900), Vec3f::ZERO)
        .build()
        .unwrap();
    let resident = current.entity_id("resident").unwrap();
    let mut dead = record(900, resident.raw());
    dead.advance_biology(Tick::new(1), BodyEventDelta::zero())
        .unwrap();
    dead.mark_dead(Tick::new(1)).unwrap();
    dead.link_birth_manifest(Blake3Digest::from_bytes([1; 32])).unwrap();
    dead.link_life_manifest(Blake3Digest::from_bytes([2; 32])).unwrap();
    current.register_organism_record(dead).unwrap();
    current.retire_dead_organism(OrganismId(900)).unwrap();

    let current_save = save(&current);
    assert_eq!(current_save.world.next_organism_id, 901);
    let restored = PortableSaveFile::from_json_str(&serde_json::to_string(&current_save).unwrap())
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(save(&restored).world.next_organism_id, 901);

    let legacy_world = HeadlessScenarioBuilder::new(44_007)
        .agent("resident", OrganismId(900), Vec3f::ZERO)
        .build()
        .unwrap();
    let mut legacy = serde_json::to_value(save(&legacy_world)).unwrap();
    legacy["world"]
        .as_object_mut()
        .unwrap()
        .remove("next_organism_id");
    let restored_legacy = PortableSaveFile::from_json_str(&legacy.to_string())
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(save(&restored_legacy).world.next_organism_id, 901);
}

#[test]
fn canonical_signature_includes_future_organism_identity_state() {
    let mut advanced = HeadlessScenarioBuilder::new(44_008)
        .agent("resident", OrganismId(900), Vec3f::ZERO)
        .build()
        .unwrap();
    let resident = advanced.entity_id("resident").unwrap();
    let mut dead = record(900, resident.raw());
    dead.advance_biology(Tick::new(1), BodyEventDelta::zero())
        .unwrap();
    dead.mark_dead(Tick::new(1)).unwrap();
    dead.link_birth_manifest(Blake3Digest::from_bytes([1; 32])).unwrap();
    dead.link_life_manifest(Blake3Digest::from_bytes([2; 32])).unwrap();
    advanced.register_organism_record(dead).unwrap();
    advanced.retire_dead_organism(OrganismId(900)).unwrap();

    let mut baseline = HeadlessScenarioBuilder::new(44_008)
        .agent("resident", OrganismId(900), Vec3f::ZERO)
        .build()
        .unwrap();
    baseline.remove_agent_entity(resident).unwrap();

    assert_eq!(advanced.object_snapshots(), baseline.object_snapshots());
    assert!(advanced.organism_registry().is_empty());
    assert!(baseline.organism_registry().is_empty());
    assert_ne!(
        advanced.canonical_signature_digest().unwrap(),
        baseline.canonical_signature_digest().unwrap()
    );
}

#[test]
fn canonical_signature_registry_is_v4_and_included_in_world_identity() {
    let (mut world, resident_a, resident_b) = world_with_two_agents();
    let empty = world.canonical_signature_digest().unwrap();

    world
        .replace_organism_registry_exact(
            [record(1, resident_a.raw()), record(2, resident_b.raw())].into_iter(),
        )
        .unwrap();
    let registered = world.canonical_signature_digest().unwrap();

    assert_eq!(empty.schema_version, 4);
    assert_eq!(registered.schema_version, 4);
    assert_ne!(empty, registered);
}

#[test]
fn canonical_signature_registry_order_is_input_order_independent() {
    let (world, resident_a, resident_b) = world_with_two_agents();
    let mut forward = world.clone();
    let mut reverse = world;

    forward
        .replace_organism_registry_exact(
            [record(1, resident_a.raw()), record(2, resident_b.raw())].into_iter(),
        )
        .unwrap();
    reverse
        .replace_organism_registry_exact(
            [record(2, resident_b.raw()), record(1, resident_a.raw())].into_iter(),
        )
        .unwrap();

    assert_eq!(
        forward.canonical_signature_digest().unwrap(),
        reverse.canonical_signature_digest().unwrap()
    );
}

#[test]
fn canonical_signature_registry_changes_when_only_biology_lifecycle_archive_changes() {
    let (mut base, resident_a, resident_b) = world_with_two_agents();
    let mut changed = base.clone();
    let initial = record(1, resident_a.raw());
    let unchanged = record(2, resident_b.raw());
    base.replace_organism_registry_exact([initial.clone(), unchanged.clone()].into_iter())
        .unwrap();

    let mut changed_record = initial;
    changed_record
        .advance_biology(Tick(1), BodyEventDelta::zero())
        .unwrap();
    changed_record.mark_dead(Tick(1)).unwrap();
    changed_record
        .link_birth_manifest(Blake3Digest::from_bytes([1; 32]))
        .unwrap();
    changed_record
        .link_life_manifest(Blake3Digest::from_bytes([2; 32]))
        .unwrap();
    changed
        .replace_organism_registry_exact([changed_record, unchanged].into_iter())
        .unwrap();

    assert_eq!(base.object_snapshots(), changed.object_snapshots());
    assert_eq!(base.tick(), changed.tick());
    assert_ne!(
        base.canonical_signature_digest().unwrap(),
        changed.canonical_signature_digest().unwrap()
    );
}

#[test]
fn canonical_signature_distinguishes_one_bit_radius_change_at_contact_threshold() {
    let contact_radius = 0.75_f32;
    let one_bit_larger = f32::from_bits(contact_radius.to_bits() + 1);
    let at_threshold = HeadlessScenarioBuilder::new(44_002)
        .agent("resident", OrganismId(1), Vec3f::ZERO)
        .obstacle(
            "contact-boundary",
            Vec3f::new(0.75, 0.0, 0.0),
            contact_radius,
        )
        .build()
        .unwrap();
    let outside_threshold = HeadlessScenarioBuilder::new(44_002)
        .agent("resident", OrganismId(1), Vec3f::ZERO)
        .obstacle(
            "contact-boundary",
            Vec3f::new(0.75, 0.0, 0.0),
            one_bit_larger,
        )
        .build()
        .unwrap();

    assert_eq!(at_threshold.seed(), outside_threshold.seed());
    assert_eq!(at_threshold.tick(), outside_threshold.tick());
    assert_ne!(
        at_threshold.canonical_signature_digest().unwrap(),
        outside_threshold.canonical_signature_digest().unwrap()
    );
}
