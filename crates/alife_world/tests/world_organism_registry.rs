use alife_core::{
    BiochemistryState, Blake3Digest, BodyEventDelta, BrainCapacityClass, CreatureGenome,
    FoundationGeneticIdentity, GenomeId, LineageId, OrganismId, ScaffoldContractError, Tick,
    WorldEntityId,
};
use alife_world::{
    OrganismLifecycle, OrganismRegistryError, WorldOrganismRecord, WorldOrganismRegistry,
};

fn record(organism_id: u64, world_entity_id: u64) -> WorldOrganismRecord {
    record_at(organism_id, world_entity_id, Tick::ZERO)
}

fn record_at(organism_id: u64, world_entity_id: u64, birth_tick: Tick) -> WorldOrganismRecord {
    let genome = CreatureGenome::early_mammal_founder(
        0xE10_3100 + organism_id,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let biochemistry = BiochemistryState::new(&phenotype, birth_tick).unwrap();
    WorldOrganismRecord::new(
        OrganismId(organism_id),
        WorldEntityId(world_entity_id),
        genome,
        phenotype,
        biochemistry,
        birth_tick,
    )
    .unwrap()
}

fn digest(byte: u8) -> Blake3Digest {
    Blake3Digest::from_bytes([byte; 32])
}

fn malformed_record(
    record: WorldOrganismRecord,
    mutate: impl FnOnce(&mut serde_json::Value),
) -> WorldOrganismRecord {
    let mut value = serde_json::to_value(record).unwrap();
    mutate(&mut value);
    serde_json::from_value(value).unwrap()
}

#[test]
fn valid_record_inserts_and_resolves_by_both_stable_ids() {
    let mut registry = WorldOrganismRegistry::default();
    registry.insert(record(1, 101)).unwrap();

    let organism = registry.get(OrganismId(1)).unwrap();
    assert_eq!(organism.organism_id(), OrganismId(1));
    assert_eq!(organism.world_entity_id(), WorldEntityId(101));
    assert_eq!(organism.genome().id, organism.phenotype().source_genome_id);
    assert_eq!(
        organism.genome().id,
        organism.biochemistry().source_genome_id
    );

    let by_entity = registry.get_by_world_entity_id(WorldEntityId(101)).unwrap();
    assert_eq!(by_entity.organism_id(), OrganismId(1));
    registry.validate_contract().unwrap();
}

#[test]
fn registry_insert_rejects_identity_and_exact_expression_mismatches() {
    let phenotype_mismatch = malformed_record(record(1, 101), |value| {
        value["phenotype"]["source_genome_id"] = serde_json::json!(GenomeId(2));
    });
    let lineage_mismatch = malformed_record(record(2, 102), |value| {
        value["phenotype"]["lineage_id"] = serde_json::json!(LineageId(3));
    });
    let biochemistry_mismatch = malformed_record(record(3, 103), |value| {
        value["biochemistry"]["source_genome_id"] = serde_json::json!(GenomeId(4));
    });
    let expression_mismatch = malformed_record(record(4, 104), |value| {
        let size_scale = value["phenotype"]["body"]["size_scale"].as_f64().unwrap();
        value["phenotype"]["body"]["size_scale"] = serde_json::json!(size_scale + 0.01);
    });

    for invalid in [
        phenotype_mismatch,
        lineage_mismatch,
        biochemistry_mismatch,
        expression_mismatch,
    ] {
        let mut registry = WorldOrganismRegistry::default();
        assert!(matches!(
            registry.insert(invalid),
            Err(OrganismRegistryError::InvalidRecord(_))
        ));
        assert!(registry.is_empty());
    }
}

#[test]
fn duplicate_ids_are_rejected_without_corrupting_either_index() {
    let mut registry = WorldOrganismRegistry::default();
    registry.insert(record(1, 101)).unwrap();

    assert!(registry.insert(record(1, 102)).is_err());
    assert!(registry
        .get_by_world_entity_id(WorldEntityId(102))
        .is_none());
    assert_eq!(
        registry.get(OrganismId(1)).unwrap().world_entity_id(),
        WorldEntityId(101)
    );

    assert!(registry.insert(record(2, 101)).is_err());
    assert!(registry.get(OrganismId(2)).is_none());
    assert_eq!(
        registry
            .get_by_world_entity_id(WorldEntityId(101))
            .unwrap()
            .organism_id(),
        OrganismId(1)
    );
    registry.validate_contract().unwrap();
}

#[test]
fn biology_advance_keeps_identity_together_and_revalidates_the_record() {
    let mut organism = record(1, 101);
    let organism_id = organism.organism_id();
    let world_entity_id = organism.world_entity_id();
    let genome_id = organism.genome().id;
    let previous_energy = organism.biochemistry().body.energy;

    organism
        .advance_biology(
            Tick(12),
            BodyEventDelta {
                energy: -0.20,
                damage: 0.25,
                ..BodyEventDelta::zero()
            },
        )
        .unwrap();

    assert_eq!(organism.organism_id(), organism_id);
    assert_eq!(organism.world_entity_id(), world_entity_id);
    assert_eq!(organism.genome().id, genome_id);
    assert_eq!(organism.phenotype().source_genome_id, genome_id);
    assert_eq!(organism.biochemistry().source_genome_id, genome_id);
    assert_eq!(organism.biochemistry().tick, Tick(12));
    assert!(organism.biochemistry().body.energy < previous_energy);
    assert!(organism.validate_contract().is_ok());
}

#[test]
fn malformed_record_advance_rolls_back_biology_after_record_validation_fails() {
    let mut organism = malformed_record(record(1, 101), |value| {
        value["phenotype"]["lineage_id"] = serde_json::json!(LineageId(3));
    });
    let before = *organism.biochemistry();

    let result = organism.advance_biology(Tick(1), BodyEventDelta::zero());

    assert_eq!(
        result,
        Err(OrganismRegistryError::InvalidRecord(
            ScaffoldContractError::InvalidId,
        ))
    );
    assert_eq!(*organism.biochemistry(), before);
}

#[test]
fn biology_tick_regression_leaves_record_biology_unchanged() {
    let mut organism = record_at(1, 101, Tick(5));
    let before = *organism.biochemistry();

    let result = organism.advance_biology(Tick(4), BodyEventDelta::zero());

    assert_eq!(
        result,
        Err(OrganismRegistryError::InvalidRecord(
            ScaffoldContractError::NonMonotonicTick,
        ))
    );
    assert_eq!(*organism.biochemistry(), before);
    assert!(organism.validate_contract().is_ok());
}

#[test]
fn registry_biology_tick_regressions_leave_biology_unchanged() {
    let organism_id = OrganismId(1);
    let mut registry = WorldOrganismRegistry::default();
    registry.insert(record_at(1, 101, Tick(5))).unwrap();
    let before = *registry.get(organism_id).unwrap().biochemistry();

    let advance_result = registry.advance_biology(organism_id, Tick(4), BodyEventDelta::zero());
    assert_eq!(
        advance_result,
        Err(OrganismRegistryError::InvalidRecord(
            ScaffoldContractError::NonMonotonicTick,
        ))
    );
    assert_eq!(*registry.get(organism_id).unwrap().biochemistry(), before);

    let seam_result = registry.with_biology_mut(organism_id, |biology| {
        biology.tick = Tick(4);
        Ok(())
    });
    assert_eq!(
        seam_result,
        Err(OrganismRegistryError::InvalidRecord(
            ScaffoldContractError::NonMonotonicTick,
        ))
    );
    assert_eq!(*registry.get(organism_id).unwrap().biochemistry(), before);
    registry.validate_contract().unwrap();
}

#[test]
fn lifecycle_and_archive_links_enforce_ordering_without_revival() {
    let mut organism = record_at(1, 101, Tick(5));
    let birth_manifest = digest(1);
    let life_manifest = digest(2);

    assert!(organism.link_life_manifest(life_manifest).is_err());
    assert!(organism.mark_dead(Tick(4)).is_err());
    assert!(matches!(organism.lifecycle(), OrganismLifecycle::Alive));

    organism.mark_dead(Tick(5)).unwrap();
    assert!(organism.mark_dead(Tick(6)).is_err());
    assert!(organism.link_life_manifest(life_manifest).is_err());

    organism.link_birth_manifest(birth_manifest).unwrap();
    organism.link_life_manifest(life_manifest).unwrap();
    assert_eq!(
        organism.archive().birth_manifest_digest(),
        Some(birth_manifest)
    );
    assert_eq!(
        organism.archive().life_manifest_digest(),
        Some(life_manifest)
    );
    assert!(organism.validate_contract().is_ok());

    let live_with_life = malformed_record(record(2, 102), |value| {
        value["archive"]["life_manifest_digest"] = serde_json::to_value(life_manifest).unwrap();
    });
    assert!(live_with_life.validate_contract().is_err());
}

#[test]
fn insert_rejects_zero_ids_and_birth_after_current_biology_tick() {
    let zero_organism = malformed_record(record(1, 101), |value| {
        value["organism_id"] = serde_json::json!(0);
    });
    let zero_entity = malformed_record(record(2, 102), |value| {
        value["world_entity_id"] = serde_json::json!(0);
    });
    let birth_after_biology = malformed_record(record(3, 103), |value| {
        value["birth_tick"] = serde_json::json!(1);
    });

    for invalid in [zero_organism, zero_entity, birth_after_biology] {
        let mut registry = WorldOrganismRegistry::default();
        assert!(registry.insert(invalid).is_err());
        assert!(registry.is_empty());
    }

    let mut advanced = record(4, 104);
    advanced
        .advance_biology(Tick(5), BodyEventDelta::zero())
        .unwrap();
    assert!(advanced.mark_dead(Tick(4)).is_err());
}

#[test]
fn age_and_zero_archive_digest_contracts_are_checked() {
    let organism = record_at(1, 101, Tick(5));
    assert_eq!(organism.age_at(Tick(12)).unwrap(), Tick(7));
    assert!(organism.age_at(Tick(4)).is_err());

    let mut organism = organism;
    assert!(organism
        .link_birth_manifest(Blake3Digest::from_bytes([0; 32]))
        .is_err());
}

#[test]
fn same_digest_archive_retries_still_validate_record_ordering() {
    let birth_manifest = digest(1);
    let life_manifest = digest(2);

    let mut birth_linked = record(1, 101);
    birth_linked.link_birth_manifest(birth_manifest).unwrap();
    let mut malformed_birth_retry = malformed_record(birth_linked, |value| {
        value["archive"]["life_manifest_digest"] = serde_json::to_value(life_manifest).unwrap();
    });
    assert!(malformed_birth_retry
        .link_birth_manifest(birth_manifest)
        .is_err());

    let mut life_linked = record_at(2, 102, Tick(5));
    life_linked.mark_dead(Tick(5)).unwrap();
    life_linked.link_birth_manifest(birth_manifest).unwrap();
    life_linked.link_life_manifest(life_manifest).unwrap();
    let mut malformed_life_retry = malformed_record(life_linked, |value| {
        value["lifecycle"] = serde_json::json!("Alive");
    });
    assert!(malformed_life_retry
        .link_life_manifest(life_manifest)
        .is_err());
}

#[test]
fn malformed_death_transition_leaves_lifecycle_unchanged() {
    let mut organism = malformed_record(record(1, 101), |value| {
        value["phenotype"]["lineage_id"] = serde_json::json!(LineageId(3));
    });
    let before = organism.lifecycle();

    let result = organism.mark_dead(Tick::ZERO);

    assert_eq!(
        result,
        Err(OrganismRegistryError::InvalidRecord(
            ScaffoldContractError::InvalidId,
        ))
    );
    assert_eq!(organism.lifecycle(), before);
}

#[test]
fn malformed_birth_link_transition_leaves_archive_unchanged() {
    let mut organism = malformed_record(record(1, 101), |value| {
        value["phenotype"]["lineage_id"] = serde_json::json!(LineageId(3));
    });
    let before = *organism.archive();

    let result = organism.link_birth_manifest(digest(1));

    assert_eq!(
        result,
        Err(OrganismRegistryError::InvalidRecord(
            ScaffoldContractError::InvalidId,
        ))
    );
    assert_eq!(*organism.archive(), before);
}

#[test]
fn malformed_life_link_transition_leaves_archive_unchanged() {
    let mut valid = record(1, 101);
    valid.mark_dead(Tick::ZERO).unwrap();
    valid.link_birth_manifest(digest(1)).unwrap();
    let mut organism = malformed_record(valid, |value| {
        value["phenotype"]["lineage_id"] = serde_json::json!(LineageId(3));
    });
    let before = *organism.archive();

    let result = organism.link_life_manifest(digest(2));

    assert_eq!(
        result,
        Err(OrganismRegistryError::InvalidRecord(
            ScaffoldContractError::InvalidId,
        ))
    );
    assert_eq!(*organism.archive(), before);
}

#[test]
fn same_digest_archive_retries_are_idempotent_and_valid() {
    let organism_id = OrganismId(1);
    let birth_manifest = digest(1);
    let life_manifest = digest(2);
    let mut registry = WorldOrganismRegistry::default();
    registry.insert(record(1, 101)).unwrap();
    registry
        .link_birth_manifest(organism_id, birth_manifest)
        .unwrap();
    registry.mark_dead(organism_id, Tick::ZERO).unwrap();
    registry
        .link_life_manifest(organism_id, life_manifest)
        .unwrap();
    let before = *registry.get(organism_id).unwrap().archive();

    registry
        .link_birth_manifest(organism_id, birth_manifest)
        .unwrap();
    registry
        .link_life_manifest(organism_id, life_manifest)
        .unwrap();

    assert_eq!(*registry.get(organism_id).unwrap().archive(), before);
    registry.validate_contract().unwrap();
}

#[test]
fn registry_forwards_validated_biology_lifecycle_and_archive_operations() {
    let organism_id = OrganismId(1);
    let world_entity_id = WorldEntityId(101);
    let birth_manifest = digest(1);
    let life_manifest = digest(2);
    let mut registry = WorldOrganismRegistry::default();
    registry.insert(record(1, 101)).unwrap();

    registry
        .advance_biology(organism_id, Tick(3), BodyEventDelta::zero())
        .unwrap();
    registry.mark_dead(organism_id, Tick(3)).unwrap();
    registry
        .link_birth_manifest(organism_id, birth_manifest)
        .unwrap();
    registry
        .link_life_manifest(organism_id, life_manifest)
        .unwrap();

    let organism = registry.get(organism_id).unwrap();
    assert_eq!(organism.biochemistry().tick, Tick(3));
    assert_eq!(
        organism.lifecycle(),
        OrganismLifecycle::Dead {
            death_tick: Tick(3)
        }
    );
    assert_eq!(
        organism.archive().birth_manifest_digest(),
        Some(birth_manifest)
    );
    assert_eq!(
        organism.archive().life_manifest_digest(),
        Some(life_manifest)
    );
    assert_eq!(
        registry
            .get_by_world_entity_id(world_entity_id)
            .unwrap()
            .organism_id(),
        organism_id
    );
    registry.validate_contract().unwrap();
}

#[test]
fn dead_registry_rejects_both_biology_paths_without_mutating_biology() {
    let organism_id = OrganismId(1);
    let mut registry = WorldOrganismRegistry::default();
    registry.insert(record(1, 101)).unwrap();
    registry
        .advance_biology(organism_id, Tick(3), BodyEventDelta::zero())
        .unwrap();
    registry.mark_dead(organism_id, Tick(3)).unwrap();
    let before = *registry.get(organism_id).unwrap().biochemistry();

    let mut closure_called = false;
    let narrow_result = registry.with_biology_mut(organism_id, |biology| {
        closure_called = true;
        biology.body.energy = 0.0;
        Ok(())
    });

    assert_eq!(
        narrow_result,
        Err(OrganismRegistryError::DeadOrganism(organism_id))
    );
    assert!(!closure_called);
    assert_eq!(*registry.get(organism_id).unwrap().biochemistry(), before);

    let forwarding_result = registry.advance_biology(
        organism_id,
        Tick(4),
        BodyEventDelta {
            energy: -0.25,
            ..BodyEventDelta::zero()
        },
    );
    assert_eq!(
        forwarding_result,
        Err(OrganismRegistryError::DeadOrganism(organism_id))
    );
    assert_eq!(*registry.get(organism_id).unwrap().biochemistry(), before);
    registry.validate_contract().unwrap();
}

#[test]
fn biology_mutation_rolls_back_when_the_closure_returns_an_error() {
    let organism_id = OrganismId(1);
    let mut registry = WorldOrganismRegistry::default();
    registry.insert(record(1, 101)).unwrap();
    let before = *registry.get(organism_id).unwrap().biochemistry();

    let result: Result<(), OrganismRegistryError> =
        registry.with_biology_mut(organism_id, |biology| {
            biology.body.energy = 0.0;
            Err(OrganismRegistryError::InvalidRecord(
                ScaffoldContractError::InvalidId,
            ))
        });

    assert_eq!(
        result,
        Err(OrganismRegistryError::InvalidRecord(
            ScaffoldContractError::InvalidId,
        ))
    );
    assert_eq!(*registry.get(organism_id).unwrap().biochemistry(), before);
    registry.validate_contract().unwrap();
}

#[test]
fn biology_mutation_rolls_back_when_the_resulting_biology_is_invalid() {
    let organism_id = OrganismId(1);
    let mut registry = WorldOrganismRegistry::default();
    registry.insert(record(1, 101)).unwrap();
    let before = *registry.get(organism_id).unwrap().biochemistry();

    let result = registry.with_biology_mut(organism_id, |biology| {
        biology.body.energy = f32::NAN;
        Ok(())
    });

    assert!(matches!(
        result,
        Err(OrganismRegistryError::InvalidRecord(_))
    ));
    assert_eq!(*registry.get(organism_id).unwrap().biochemistry(), before);
    registry.validate_contract().unwrap();
}
