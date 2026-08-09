use alife_core::{
    BiochemistryState, Blake3Digest, BodyEventDelta, BrainCapacityClass, CreatureGenome,
    FoundationGeneticIdentity, GenomeId, HomeostaticSnapshot, OrganismId, Tick, Vec3f,
    WorldEntityId,
};
use alife_world::{
    persistence::{
        AssetManifest, CreatureMindSaveSummary, CreatureSaveState, LearningTraceSaveSummary,
        PortableSaveFile, RuntimeConfig, WeightLayerSaveSummary,
    },
    CreatureAppearanceGenome, Habitat, HabitatAuthority, HabitatId, HabitatMode,
    HeadlessScenarioBuilder, HeadlessWorld, WorldOrganismRecord,
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

fn world_with_nontrivial_registry() -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(73_201)
        .agent("agent-a", OrganismId(7), Vec3f::ZERO)
        .agent("agent-b", OrganismId(8), Vec3f::new(4.0, 0.0, 0.0))
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let agent_a = world.entity_id("agent-a").unwrap();
    let agent_b = world.entity_id("agent-b").unwrap();

    let mut dead = record(7, agent_a.raw());
    dead.advance_biology(Tick(2), BodyEventDelta::zero())
        .unwrap();
    dead.mark_dead(Tick(2)).unwrap();
    dead.link_birth_manifest(Blake3Digest::from_bytes([1; 32]))
        .unwrap();
    dead.link_life_manifest(Blake3Digest::from_bytes([2; 32]))
        .unwrap();

    world
        .replace_organism_registry_exact([dead, record(8, agent_b.raw())].into_iter())
        .unwrap();
    world
}

fn save(world: &HeadlessWorld) -> PortableSaveFile {
    save_with_creatures(world, Vec::new())
}

fn save_with_creatures(
    world: &HeadlessWorld,
    creatures: Vec<CreatureSaveState>,
) -> PortableSaveFile {
    PortableSaveFile::from_headless_world(
        "task-3-2b3c1-round-trip",
        world,
        RuntimeConfig::deterministic_default(world.seed(), alife_core::BrainScaleTier::Nano512),
        AssetManifest::empty(),
        creatures,
    )
    .unwrap()
}

fn creature_save_state() -> CreatureSaveState {
    CreatureSaveState {
        organism_id: OrganismId(7),
        genome_id: GenomeId(17),
        brain_class: alife_core::BrainScaleTier::Nano512,
        development_tick: Tick::new(3),
        appearance: CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick: Tick::new(3),
            homeostasis: HomeostaticSnapshot::baseline(Tick::new(3)),
            memory_record_count: 2,
            memory_source_ids: Vec::new(),
            concept_count: 1,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".to_string(),
            diagnostics: vec!["fixture".to_string()],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: Some("task-3-2b3c1-generated".to_string()),
            genetic_fixed_digest: "fnv1a64:0000000000000001".to_string(),
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: 3,
            h_operational_entries: 1,
            h_shadow_entries: 1,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: true,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: Some(Tick::new(2)),
        },
        composite_genetics: None,
        lifetime_state_asset: None,
        gpu_brain: None,
    }
}

fn registry_records(world: &HeadlessWorld) -> Vec<WorldOrganismRecord> {
    let mut records: Vec<_> = world.organism_registry().iter().cloned().collect();
    records.sort_by_key(|record| record.organism_id().raw());
    records
}

fn world_with_agents() -> HeadlessWorld {
    HeadlessScenarioBuilder::new(73_201)
        .agent("agent-a", OrganismId(7), Vec3f::ZERO)
        .agent("agent-b", OrganismId(8), Vec3f::new(4.0, 0.0, 0.0))
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap()
}

fn serialized_world(world: &HeadlessWorld) -> serde_json::Value {
    serde_json::to_value(save(world)).unwrap()
}

fn serialized_world_with_registry() -> serde_json::Value {
    let world = world_with_nontrivial_registry();
    let mut value = serialized_world(&world);
    value["world"]["organism_records"] = serde_json::to_value(registry_records(&world)).unwrap();
    value
}

fn serialized_registry_ids(value: &serde_json::Value) -> Option<Vec<u64>> {
    value["world"]["organism_records"]
        .as_array()
        .map(|records| {
            records
                .iter()
                .map(|record| record["organism_id"].as_u64().unwrap())
                .collect()
        })
}

fn assert_rejected(value: serde_json::Value, label: &str) {
    let text = serde_json::to_string(&value).unwrap();
    assert!(
        PortableSaveFile::from_json_str(&text).is_err(),
        "JSON decoder accepted corruption case: {label}"
    );
}

#[test]
fn registered_agents_survive_portable_json_restore_with_exact_identity_and_signature() {
    let world = world_with_nontrivial_registry();
    let expected_records = registry_records(&world);
    let expected_signature = world.canonical_signature_digest().unwrap();

    let encoded = serde_json::to_string_pretty(&save(&world)).unwrap();
    let json: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    let serialized_ids = serialized_registry_ids(&json);
    assert_eq!(serialized_ids, Some(vec![7, 8]));

    let restored = PortableSaveFile::from_json_str(&encoded)
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(registry_records(&restored), expected_records);
    assert_eq!(
        restored.canonical_signature_digest().unwrap(),
        expected_signature
    );
}

#[test]
fn registry_insertion_order_has_identical_json_and_restored_identity() {
    let base = world_with_agents();
    let agent_a = base.entity_id("agent-a").unwrap();
    let agent_b = base.entity_id("agent-b").unwrap();
    let mut forward = base.clone();
    let mut reverse = base;

    forward
        .replace_organism_registry_exact(
            [record(7, agent_a.raw()), record(8, agent_b.raw())].into_iter(),
        )
        .unwrap();
    reverse
        .replace_organism_registry_exact(
            [record(8, agent_b.raw()), record(7, agent_a.raw())].into_iter(),
        )
        .unwrap();

    let forward_json = serde_json::to_string(&save(&forward)).unwrap();
    let reverse_json = serde_json::to_string(&save(&reverse)).unwrap();
    assert_eq!(forward_json, reverse_json);

    let forward_json_value: serde_json::Value = serde_json::from_str(&forward_json).unwrap();
    assert_eq!(
        serialized_registry_ids(&forward_json_value),
        Some(vec![7, 8])
    );
    let forward_restored = PortableSaveFile::from_json_str(&forward_json)
        .unwrap()
        .restore_headless_world()
        .unwrap();
    let reverse_restored = PortableSaveFile::from_json_str(&reverse_json)
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(
        registry_records(&forward_restored),
        registry_records(&reverse_restored)
    );
    assert_eq!(
        forward_restored.canonical_signature_digest().unwrap(),
        reverse_restored.canonical_signature_digest().unwrap()
    );
}

#[test]
fn replacing_headless_world_snapshot_retains_registry_records_and_archive_links() {
    let empty_world = world_with_agents();
    let registered_world = world_with_nontrivial_registry();
    let expected_records = registry_records(&registered_world);

    let mut save = save(&empty_world);
    save.replace_headless_world_snapshot(&registered_world)
        .unwrap();

    assert_eq!(
        serialized_registry_ids(&serde_json::to_value(&save).unwrap()),
        Some(vec![7, 8])
    );
    let restored = save.restore_headless_world().unwrap();
    assert_eq!(registry_records(&restored), expected_records);
}

#[test]
fn failed_snapshot_replacement_leaves_the_entire_save_unchanged() {
    let initial_world = world_with_agents();
    let mut save = save_with_creatures(&initial_world, vec![creature_save_state()]);
    let mut replacement_world = world_with_agents();
    let custom_habitat = Habitat::new(
        HabitatId::new(2).unwrap(),
        "Replacement Reserve",
        HabitatMode::Reserve,
    )
    .unwrap();
    replacement_world
        .replace_habitat_authority(HabitatAuthority::new(vec![custom_habitat]).unwrap())
        .unwrap();
    let before = save.clone();

    assert!(save
        .replace_headless_world_snapshot(&replacement_world)
        .is_err());
    assert_eq!(save, before);
}

#[test]
fn absent_registry_field_is_legacy_empty_without_changing_world_object_identity() {
    let world = world_with_nontrivial_registry();
    let expected_objects = world.object_snapshots();
    let mut value = serialized_world(&world);
    value["world"]
        .as_object_mut()
        .unwrap()
        .remove("organism_records");

    let restored = PortableSaveFile::from_json_str(&serde_json::to_string(&value).unwrap())
        .unwrap()
        .restore_headless_world()
        .unwrap();
    assert_eq!(restored.object_snapshots(), expected_objects);
    assert!(restored.organism_registry().is_empty());
}

#[test]
fn present_empty_registry_is_authoritative_and_rejects_legacy_agents() {
    let mut value = serialized_world(&world_with_agents());
    value["world"]["organism_records"] = serde_json::json!([]);
    assert_rejected(value, "present empty registry with Agent objects");
}

#[test]
fn malformed_present_registry_records_are_rejected_before_restore() {
    let valid = serialized_world_with_registry();
    let cases: [(&str, Box<dyn Fn(&mut serde_json::Value)>); 11] = [
        (
            "explicit null registry field",
            Box::new(|value| {
                value["world"]["organism_records"] = serde_json::Value::Null;
            }),
        ),
        (
            "malformed genome",
            Box::new(|value| {
                value["world"]["organism_records"][0]["genome"]["id"] = serde_json::json!(0);
            }),
        ),
        (
            "genome phenotype mismatch",
            Box::new(|value| {
                value["world"]["organism_records"][0]["phenotype"]["source_genome_id"] =
                    serde_json::json!(0);
            }),
        ),
        (
            "invalid biochemistry",
            Box::new(|value| {
                value["world"]["organism_records"][0]["biochemistry"]["homeostasis"]["tick"] =
                    serde_json::json!(0);
            }),
        ),
        (
            "invalid lifecycle",
            Box::new(|value| {
                value["world"]["organism_records"][0]["lifecycle"]["death_tick"] =
                    serde_json::json!(1);
            }),
        ),
        (
            "invalid archive links",
            Box::new(|value| {
                value["world"]["organism_records"][0]["archive"]["birth_manifest_digest"] =
                    serde_json::Value::Null;
            }),
        ),
        (
            "duplicate organism id",
            Box::new(|value| {
                let first = value["world"]["organism_records"][0].clone();
                value["world"]["organism_records"]
                    .as_array_mut()
                    .unwrap()
                    .push(first);
            }),
        ),
        (
            "duplicate entity id",
            Box::new(|value| {
                let entity_id = value["world"]["organism_records"][0]["world_entity_id"].clone();
                value["world"]["organism_records"][1]["world_entity_id"] = entity_id;
            }),
        ),
        (
            "missing Agent binding",
            Box::new(|value| {
                value["world"]["organism_records"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }),
        ),
        (
            "extra Agent binding",
            Box::new(|value| {
                let extra = serde_json::to_value(record(99, 9_999)).unwrap();
                value["world"]["organism_records"]
                    .as_array_mut()
                    .unwrap()
                    .push(extra);
            }),
        ),
        (
            "entity identity mismatch",
            Box::new(|value| {
                value["world"]["organism_records"][0]["world_entity_id"] = serde_json::json!(9_999);
            }),
        ),
    ];

    for (label, mutate) in cases {
        let mut value = valid.clone();
        mutate(&mut value);
        assert_rejected(value, label);
    }
}
