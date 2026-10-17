//! CPU-only exact-output/error-order regression against the frozen sync path.
use super::*;
use alife_world::HeadlessScenarioBuilder;

// Literal helper bodies from main69f00c05. Keep the independent owned admission
// snapshot in this oracle so allocation removal cannot validate itself.
fn legacy_synchronize_resident_from_record(
    resident: &mut ResidentCognition,
    record: &WorldOrganismRecord,
    world_tick: Tick,
) -> Result<(), ScaffoldContractError> {
    let admission = record.authoritative_admission_at(world_tick)?;
    if resident.genome != admission.phenotype.brain_genome {
        return Err(ScaffoldContractError::BrainOwnershipMismatch);
    }
    resident.homeostasis = admission.biochemistry.homeostasis;
    resident.development = admission.phenotype.development_state_at(admission.age)?;
    Ok(())
}

fn legacy_synchronize_residents_from_world(
    world: &HeadlessWorld,
    residents: &mut BTreeMap<u64, ResidentCognition>,
    world_tick: Tick,
) -> Result<(), ScaffoldContractError> {
    let organism_ids = residents.keys().copied().collect::<Vec<_>>();
    for raw in organism_ids {
        let record = world
            .organism_registry()
            .get(OrganismId(raw))
            .cloned()
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if !record.lifecycle().is_alive() {
            continue;
        }
        let resident = residents
            .get_mut(&raw)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        legacy_synchronize_resident_from_record(resident, &record, world_tick)?;
    }
    Ok(())
}

fn fixture() -> (HeadlessWorld, BTreeMap<u64, ResidentCognition>) {
    let mut world = HeadlessScenarioBuilder::new(77_112)
        .agent("first", OrganismId(1), Vec3f::ZERO)
        .agent("second", OrganismId(2), Vec3f::new(1.0, 0.0, 0.0))
        .build()
        .unwrap();
    let residents = [1, 2]
        .into_iter()
        .map(|raw| {
            super::tests::register_sealing_test_organism(&mut world, OrganismId(raw));
            let (_, resident) = GpuLiveBrainRuntime::compile_birth(
                &world,
                BrainScaleTier::Nano512,
                SensorProfile::GroundedObjectSlotsV1,
                OrganismId(raw),
            )
            .unwrap();
            (raw, resident)
        })
        .collect();
    (world, residents)
}

fn compare_one(
    record: &WorldOrganismRecord,
    residents: &BTreeMap<u64, ResidentCognition>,
    tick: Tick,
) -> Result<(), ScaffoldContractError> {
    let raw = record.organism_id().raw();
    let mut legacy = residents[&raw].clone();
    let mut actual = residents[&raw].clone();
    let record_before = serde_json::to_value(record).unwrap();
    let expected_result = legacy_synchronize_resident_from_record(&mut legacy, record, tick);
    let actual_result = synchronize_resident_from_record(&mut actual, record, tick);
    assert_eq!(actual_result, expected_result);
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(legacy).unwrap()
    );
    assert_eq!(serde_json::to_value(record).unwrap(), record_before);
    actual_result
}

fn compare_population(
    world: &HeadlessWorld,
    residents: &BTreeMap<u64, ResidentCognition>,
    tick: Tick,
) -> Result<(), ScaffoldContractError> {
    let mut legacy = residents.clone();
    let mut actual = residents.clone();
    let before = world.canonical_signature_digest().unwrap();
    let expected_result = legacy_synchronize_residents_from_world(world, &mut legacy, tick);
    let actual_result = synchronize_residents_from_world(world, &mut actual, tick);
    assert_eq!(actual_result, expected_result);
    assert_eq!(
        serde_json::to_value(actual).unwrap(),
        serde_json::to_value(legacy).unwrap()
    );
    assert_eq!(world.canonical_signature_digest().unwrap(), before);
    actual_result
}

#[test]
fn borrowed_sync_matches_full_residents_over_current_biology_and_development() {
    let (mut world, residents) = fixture();
    for _ in 0..3 {
        for record in world.organism_registry().iter() {
            compare_one(record, &residents, world.tick()).unwrap();
        }
        compare_population(&world, &residents, world.tick()).unwrap();
        world.try_advance_tick().unwrap();
    }
}

#[test]
fn borrowed_sync_preserves_stale_tick_and_genome_mismatch_errors() {
    let (world, mut residents) = fixture();
    let record = world.organism_registry().get(OrganismId(1)).unwrap();
    assert_eq!(
        compare_one(record, &residents, Tick(1)),
        Err(ScaffoldContractError::NonMonotonicTick)
    );
    residents.get_mut(&1).unwrap().genome.id.0 ^= 1;
    assert_eq!(
        compare_one(record, &residents, world.tick()),
        Err(ScaffoldContractError::BrainOwnershipMismatch)
    );
    assert_eq!(
        compare_one(record, &residents, Tick(1)),
        Err(ScaffoldContractError::NonMonotonicTick),
        "tick error must precede genome mismatch"
    );
    assert_eq!(
        compare_population(&world, &residents, world.tick()),
        Err(ScaffoldContractError::BrainOwnershipMismatch)
    );
}

#[test]
fn borrowed_sync_preserves_dead_direct_rejection_and_population_skip() {
    let (mut world, residents) = fixture();
    let mut record = world
        .organism_registry()
        .get(OrganismId(1))
        .unwrap()
        .clone();
    record.mark_dead(world.tick()).unwrap();
    assert_eq!(
        compare_one(&record, &residents, world.tick()),
        Err(ScaffoldContractError::NonMonotonicTick)
    );
    world.replace_organism_record_exact(record).unwrap();
    compare_population(&world, &residents, world.tick()).unwrap();
}

#[test]
fn borrowed_sync_preserves_population_missing_record_and_partial_progress_order() {
    let (mut world, mut residents) = fixture();
    world.try_advance_tick().unwrap();
    // The first row advances before the second row fails, exactly as before.
    residents.get_mut(&2).unwrap().genome.id.0 ^= 1;
    assert_eq!(
        compare_population(&world, &residents, world.tick()),
        Err(ScaffoldContractError::BrainOwnershipMismatch)
    );
    let missing = HeadlessScenarioBuilder::new(77_112).build().unwrap();
    assert_eq!(
        compare_population(&missing, &residents, Tick::ZERO),
        Err(ScaffoldContractError::BrainOwnershipMismatch)
    );
}

fn compare_cold_wire(
    wire: serde_json::Value,
    resident: &ResidentCognition,
    tick: Tick,
) -> Result<(), ScaffoldContractError> {
    // Each derived-deserialized record owns a distinct, initially empty proof.
    // The old path cannot warm the new path's process-local genetic cache.
    let old_record: WorldOrganismRecord = serde_json::from_value(wire.clone()).unwrap();
    let new_record: WorldOrganismRecord = serde_json::from_value(wire).unwrap();
    let mut old = resident.clone();
    let mut new = resident.clone();
    let expected = legacy_synchronize_resident_from_record(&mut old, &old_record, tick);
    let actual = synchronize_resident_from_record(&mut new, &new_record, tick);
    assert_eq!(actual, expected);
    assert_eq!(
        serde_json::to_value(new).unwrap(),
        serde_json::to_value(old).unwrap()
    );
    actual
}

#[test]
fn borrowed_sync_matches_independently_restored_records_with_cold_genetic_caches() {
    let (world, residents) = fixture();
    let record = world.organism_registry().get(OrganismId(1)).unwrap();
    compare_cold_wire(
        serde_json::to_value(record).unwrap(),
        &residents[&1],
        world.tick(),
    )
    .unwrap();
}

#[test]
fn borrowed_sync_preserves_cold_immutable_validation_before_tick_and_genome_errors() {
    let (world, mut residents) = fixture();
    let record = world.organism_registry().get(OrganismId(1)).unwrap();
    let mut wire = serde_json::to_value(record).unwrap();
    // The source fields have no mutation API. Corrupt the restored source and
    // keep its original recorded subsystem digest, as a malformed save might.
    wire["genome"]["id"] = serde_json::json!(0);
    residents.get_mut(&1).unwrap().genome.id.0 ^= 1;
    assert_eq!(
        compare_cold_wire(wire, &residents[&1], Tick(1)),
        Err(ScaffoldContractError::BrainOwnershipMismatch),
        "canonical source validation must precede stale tick and resident mismatch"
    );
}

#[test]
fn borrowed_sync_preserves_dynamic_validation_before_tick_and_genome_errors() {
    let (world, mut residents) = fixture();
    let record = world.organism_registry().get(OrganismId(1)).unwrap();
    let mut wire = serde_json::to_value(record).unwrap();
    wire["cognitive_energy_debit"] = serde_json::json!(-1.0);
    residents.get_mut(&1).unwrap().genome.id.0 ^= 1;
    assert_eq!(
        compare_cold_wire(wire, &residents[&1], Tick(1)),
        Err(ScaffoldContractError::ScalarOutOfRange),
        "dynamic record validation must precede stale tick and resident mismatch"
    );
}
