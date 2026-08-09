use alife_core::{
    BiochemistryState, Blake3Digest, BodyEventDelta, BrainCapacityClass, CreatureGenome,
    FoundationGeneticIdentity, OrganismId, Tick, Vec3f, WorldEntityId,
};
use alife_world::{HeadlessScenarioBuilder, HeadlessWorld, WorldOrganismRecord};

fn record(organism_id: u64, world_entity_id: u64) -> WorldOrganismRecord {
    let genome = CreatureGenome::early_mammal_founder(
        0xE10_3300 + organism_id,
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
fn canonical_signature_registry_is_v3_and_included_in_world_identity() {
    let (mut world, resident_a, resident_b) = world_with_two_agents();
    let empty = world.canonical_signature_digest().unwrap();

    world
        .replace_organism_registry_exact(
            [record(1, resident_a.raw()), record(2, resident_b.raw())].into_iter(),
        )
        .unwrap();
    let registered = world.canonical_signature_digest().unwrap();

    assert_eq!(empty.schema_version, 3);
    assert_eq!(registered.schema_version, 3);
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
