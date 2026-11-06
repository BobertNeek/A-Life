use super::*;
use alife_core::{BrainCapacityClass, FoundationGeneticIdentity};

fn record(id: u64) -> WorldOrganismRecord {
    let genome = CreatureGenome::early_mammal_founder(
        81_000 + id,
        FoundationGeneticIdentity::new(100 + id, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    WorldOrganismRecord::newborn(
        OrganismId(id),
        WorldEntityId(100 + id),
        genome.clone(),
        genome.express().unwrap(),
        Tick::ZERO,
    )
    .unwrap()
}

fn registry() -> WorldOrganismRegistry {
    WorldOrganismRegistry::from_exact_records([record(1), record(2)]).unwrap()
}

fn cached(registry: &WorldOrganismRegistry, id: u64) -> bool {
    registry.records_by_organism[&id].is_validated()
}

#[test]
fn registry_record_proof_invalidates_on_every_mutable_dereference() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<WorldOrganismRegistry>();
    for field in 0..9 {
        let mut registry = registry();
        registry.validate_contract().unwrap();
        assert!(cached(&registry, 1) && cached(&registry, 2));
        let before = serde_json::to_vec(registry.get(OrganismId(1)).unwrap()).unwrap();
        let peer = registry.records_by_organism.get_mut(&2).unwrap();
        // Shared methods on an exclusive wrapper still cannot change state.
        peer.biochemistry();
        peer.embodiment();
        assert!(peer.is_validated());
        match field {
            0 => peer.biochemistry.body.energy = f32::NAN,
            1 => peer.organism_id = OrganismId(3),
            2 => peer.world_entity_id = WorldEntityId(103),
            3 => peer.birth_tick = Tick::new(1),
            4 => peer.state_graph.body_biochemistry.content_digest[0] ^= 1,
            5 => peer.cognitive_energy_debit = -1.0,
            6 => peer.sleep_phase_tick = Tick::new(1),
            7 => {
                peer.lifecycle = OrganismLifecycle::Dead {
                    death_tick: Tick::new(1),
                }
            }
            _ => peer.archive.birth_manifest_digest = Some(Blake3Digest::from_bytes([3; 32])),
        }
        assert!(!peer.is_validated());
        let uncached = registry.get(OrganismId(2)).unwrap().validate_contract();
        assert!(uncached.is_err(), "field {field}");
        assert_eq!(
            registry.validate_contract(),
            uncached.map_err(OrganismRegistryError::from)
        );
        assert!(cached(&registry, 1));
        assert_eq!(
            serde_json::to_vec(registry.get(OrganismId(1)).unwrap()).unwrap(),
            before
        );
    }
}

#[test]
fn registry_record_proof_preserves_wire_clone_restore_and_replacement() {
    let mut original = registry();
    original.validate_contract().unwrap();
    let wire = serde_json::to_vec(original.get(OrganismId(1)).unwrap()).unwrap();
    assert_eq!(
        serde_json::to_vec(&original.records_by_organism[&1]).unwrap(),
        wire
    );
    let mut changed = original.clone();
    changed
        .records_by_organism
        .get_mut(&1)
        .unwrap()
        .biochemistry
        .body
        .energy = -1.0;
    assert!(!cached(&changed, 1));
    assert!(changed.validate_contract().is_err());
    original.validate_contract().unwrap();
    assert_eq!(
        serde_json::to_vec(original.get(OrganismId(1)).unwrap()).unwrap(),
        wire
    );

    let restored: WorldOrganismRecord = serde_json::from_slice(&wire).unwrap();
    original.replace_existing_exact(restored).unwrap();
    assert!(!cached(&original, 1));
    original.validate_contract().unwrap();
    let mut forged: WorldOrganismRecord = serde_json::from_slice(&wire).unwrap();
    forged.cognitive_energy_debit = f32::INFINITY;
    assert!(original.replace_existing_exact(forged).is_err());
    assert!(cached(&original, 1));
    assert_eq!(
        serde_json::to_vec(original.get(OrganismId(1)).unwrap()).unwrap(),
        wire
    );

    let restored: WorldOrganismRecord = serde_json::from_slice(&wire).unwrap();
    let mut fresh = WorldOrganismRegistry::default();
    fresh.insert(restored).unwrap();
    assert!(!cached(&fresh, 1));
    fresh.validate_contract().unwrap();
    assert!(cached(&fresh, 1));
}

#[test]
fn registry_record_proof_is_local_and_failed_mutations_restore_exact_state() {
    let mut registry = registry();
    registry.validate_contract().unwrap();
    registry
        .advance_biology(OrganismId(1), Tick::new(1), BodyEventDelta::zero())
        .unwrap();
    assert!(!cached(&registry, 1));
    assert!(cached(&registry, 2));
    registry.validate_contract().unwrap();
    let before = serde_json::to_vec(registry.get(OrganismId(1)).unwrap()).unwrap();
    let result = registry.with_biology_mut(OrganismId(1), |biology| {
        biology.body.energy = f32::NAN;
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(
        serde_json::to_vec(registry.get(OrganismId(1)).unwrap()).unwrap(),
        before
    );
    registry.validate_contract().unwrap();
    registry
        .with_biology_mut(OrganismId(1), |biology| {
            Ok(biology.body.set_energy(0.25)?)
        })
        .unwrap();
    registry
        .get(OrganismId(1))
        .unwrap()
        .validate_contract()
        .unwrap();
    registry.validate_contract().unwrap();
}

#[test]
fn registry_record_proofs_do_not_cache_registry_indices() {
    let mut registry = registry();
    registry.validate_contract().unwrap();
    registry.organism_by_world_entity.insert(102, OrganismId(1));
    assert!(cached(&registry, 1) && cached(&registry, 2));
    assert_eq!(
        registry.validate_contract(),
        Err(OrganismRegistryError::IndexMismatch)
    );
}

#[test]
fn registry_record_proof_is_invalidated_before_a_mutating_closure_can_unwind() {
    let mut registry = registry();
    registry.validate_contract().unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = registry.with_biology_mut(
            OrganismId(2),
            |biology| -> Result<(), OrganismRegistryError> {
                biology.body.energy = f32::NAN;
                panic!("synthetic interrupted mutation");
            },
        );
    }));
    assert!(result.is_err());
    assert!(!cached(&registry, 2));
    // The existing closure API does not undo a panic. It must still reject the
    // now-malformed record rather than reuse the pre-mutation success proof.
    assert!(registry.validate_contract().is_err());
}
