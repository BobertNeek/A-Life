use alife_core::{
    Confidence, MemoryBankConfig, MemorySidecarState, OrganismId, ScaffoldContractError,
    SensorProfile, SensorProfileIdentity, SensoryAbiVersion, Tick, TopologicalMapConfig,
    TopologySidecar,
};
use alife_world::{
    persistence::{
        GpuBrainAssetRef, MemorySidecarSaveState, PortableAssetDigest, TopologySidecarSaveSummary,
    },
    PhysicalTrackingProvenance, StablePhysicalDescriptor, TrackedObjectRegistry,
    TrackedObjectRegistrySaveState, TRACKED_OBJECT_REGISTRY_SAVE_SCHEMA_VERSION,
};

fn profile(profile: SensorProfile) -> SensorProfileIdentity {
    SensorProfileIdentity {
        profile_id: profile.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    }
}

fn asset(label: &str) -> GpuBrainAssetRef {
    GpuBrainAssetRef {
        asset_id: label.to_string(),
        digest: PortableAssetDigest::for_bytes(label.as_bytes()),
    }
}

fn provenance(world_seed: u64, spawn_sequence: u64) -> PhysicalTrackingProvenance {
    PhysicalTrackingProvenance {
        schema_version: 1,
        world_seed,
        zone_id: 3,
        spawn_sequence,
        lineage_key: 77 + spawn_sequence,
    }
}

fn descriptor(seed: f32) -> StablePhysicalDescriptor {
    StablePhysicalDescriptor([
        seed, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, -0.1, 0.2, -0.3, 0.4, 0.5, 0.6,
    ])
}

fn populated_registry() -> (TrackedObjectRegistry, OrganismId) {
    let organism = OrganismId(9);
    let mut registry = TrackedObjectRegistry::new(0x000A_11FE, 4).unwrap();
    registry
        .observe(
            organism,
            provenance(registry.world_seed(), 1),
            descriptor(0.1),
            Tick::new(10),
        )
        .unwrap();
    registry
        .observe(
            organism,
            provenance(registry.world_seed(), 2),
            descriptor(0.15),
            Tick::new(11),
        )
        .unwrap();
    (registry, organism)
}

#[test]
fn tracked_object_state_roundtrips_and_never_reuses_an_id() {
    let (registry, organism) = populated_registry();
    let saved = registry.save_state(organism).unwrap();

    assert_eq!(
        saved.schema_version,
        TRACKED_OBJECT_REGISTRY_SAVE_SCHEMA_VERSION
    );
    assert_eq!(saved.organism_id, organism);
    assert_eq!(saved.records.len(), 2);
    assert!(saved
        .records
        .windows(2)
        .all(|pair| pair[0].tracking_key < pair[1].tracking_key));

    let expected_next_id = saved.next_id;
    let mut restored = TrackedObjectRegistry::from_save_state(saved).unwrap();
    let receipt = restored
        .observe(
            organism,
            provenance(restored.world_seed(), 3),
            descriptor(0.2),
            Tick::new(12),
        )
        .unwrap();

    assert_eq!(receipt.tracked_object_id.raw(), expected_next_id);
    assert_eq!(receipt.next_id, expected_next_id + 1);
}

#[test]
fn tracker_restore_rejects_tampered_owner_key_and_next_id() {
    let (registry, organism) = populated_registry();
    let saved = registry.save_state(organism).unwrap();

    let mut wrong_owner = saved.clone();
    wrong_owner.organism_id = OrganismId(10);
    assert!(matches!(
        TrackedObjectRegistry::from_save_state(wrong_owner),
        Err(ScaffoldContractError::MismatchedCreatureId)
    ));

    let mut wrong_key = saved.clone();
    wrong_key.records[0].tracking_key.0[0] ^= 1;
    assert!(matches!(
        TrackedObjectRegistry::from_save_state(wrong_key),
        Err(ScaffoldContractError::InvalidId)
    ));

    let mut reused_next = saved;
    reused_next.next_id = reused_next.records[1].tracked_object_id.raw();
    assert!(matches!(
        TrackedObjectRegistry::from_save_state(reused_next),
        Err(ScaffoldContractError::TrackedObjectIdentityExhausted)
    ));
}

#[test]
fn tracker_save_dto_is_portable_and_world_entity_free() {
    fn assert_portable(_: &TrackedObjectRegistrySaveState) {}

    let (registry, organism) = populated_registry();
    let saved = registry.save_state(organism).unwrap();
    assert_portable(&saved);
    let json = serde_json::to_string(&saved).unwrap();
    assert!(!json.contains("WorldEntityId"));
    assert!(!include_str!("../src/tracked_objects.rs").contains("WorldEntityId"));
}

#[test]
fn tracker_restores_multiple_organism_local_streams_without_collision() {
    let world_seed = 0x000A_11FE;
    let first = OrganismId(9);
    let second = OrganismId(10);
    let mut registry = TrackedObjectRegistry::new(world_seed, 4).unwrap();
    let first_id = registry
        .observe(
            first,
            provenance(world_seed, 1),
            descriptor(0.1),
            Tick::new(1),
        )
        .unwrap()
        .tracked_object_id;
    let second_id = registry
        .observe(
            second,
            provenance(world_seed, 1),
            descriptor(0.2),
            Tick::new(1),
        )
        .unwrap()
        .tracked_object_id;
    let saves = [
        registry.save_state(first).unwrap(),
        registry.save_state(second).unwrap(),
    ];

    let restored = TrackedObjectRegistry::from_save_states(world_seed, 4, saves).unwrap();
    assert_eq!(
        restored.record(first, first_id).unwrap().tracked_object_id,
        first_id
    );
    assert_eq!(
        restored
            .record(second, second_id)
            .unwrap()
            .tracked_object_id,
        second_id
    );
    assert!(restored.record(first, second_id).is_none());
    assert!(restored.record(second, first_id).is_none());
}

#[test]
fn memory_and_topology_summaries_bind_exact_owner_and_profile() {
    for sensor_profile in [
        SensorProfile::PrivilegedAffordanceV1,
        SensorProfile::GroundedObjectSlotsV1,
    ] {
        let organism = OrganismId(41);
        let identity = profile(sensor_profile);
        let memory = MemorySidecarState::new_profiled(
            organism,
            identity,
            MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
        )
        .unwrap();
        let saved_memory =
            MemorySidecarSaveState::from_sidecar(&memory, asset("memory-active"), None, None)
                .unwrap();
        saved_memory.validate_for(organism, identity).unwrap();
        assert_eq!(saved_memory.summary.profile, identity);
        assert_eq!(saved_memory.summary.record_count, 0);

        let topology =
            TopologySidecar::new_profiled(organism, identity, TopologicalMapConfig::default())
                .unwrap();
        let saved_topology =
            TopologySidecarSaveSummary::from_sidecar(&topology, asset("topology")).unwrap();
        saved_topology.validate_for(organism, identity).unwrap();
        assert_eq!(saved_topology.profile, identity);

        let other = profile(match sensor_profile {
            SensorProfile::PrivilegedAffordanceV1 => SensorProfile::GroundedObjectSlotsV1,
            SensorProfile::GroundedObjectSlotsV1 => SensorProfile::PrivilegedAffordanceV1,
        });
        assert!(saved_memory.validate_for(organism, other).is_err());
        assert!(saved_topology.validate_for(organism, other).is_err());
        assert!(saved_memory
            .validate_for(OrganismId(organism.raw() + 1), identity)
            .is_err());
        assert!(saved_topology
            .validate_for(OrganismId(organism.raw() + 1), identity)
            .is_err());
    }
}

#[test]
fn memory_summary_rejects_staged_asset_without_staged_checkpoint() {
    let organism = OrganismId(42);
    let identity = profile(SensorProfile::GroundedObjectSlotsV1);
    let memory = MemorySidecarState::new_profiled(
        organism,
        identity,
        MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();

    assert!(MemorySidecarSaveState::from_sidecar(
        &memory,
        asset("memory-active"),
        Some(asset("memory-staged")),
        None,
    )
    .is_err());
}
