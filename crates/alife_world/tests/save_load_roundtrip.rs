use std::{fs, path::PathBuf};

use alife_core::{
    BrainScaleTier, GenomeId, HomeostaticSnapshot, OrganismId, Tick, Vec3f, WorldEntityId,
};
use alife_world::{
    persistence::{
        AdapterRemapEntry, AdapterRemapTable, AssetKind, AssetManifest, AssetManifestEntry,
        AssetPresence, BackendSelection, CreatureMindSaveSummary, CreatureSaveState,
        FeatureFlagConfig, LearningTraceSaveSummary, MigrationHook, PersistenceError,
        PortableAssetDigest, PortableSaveFile, RuntimeConfig, SchoolConfig, WeightLayerSaveSummary,
        P34_ASSET_MANIFEST_SCHEMA, P34_ASSET_MANIFEST_SCHEMA_VERSION, P34_SAVE_FILE_SCHEMA,
        P34_SAVE_FILE_SCHEMA_VERSION,
    },
    HeadlessScenarioBuilder,
};

fn fixture_world() -> alife_world::HeadlessWorld {
    HeadlessScenarioBuilder::new(4242)
        .agent("agent", OrganismId(1), Vec3f::ZERO)
        .food("berry", Vec3f::new(1.0, 0.0, 0.0), 0.75)
        .hazard("thorn", Vec3f::new(3.0, 0.0, 0.0), 0.4)
        .token("word-food", Vec3f::new(0.0, 2.0, 0.0), 99)
        .build()
        .expect("fixture world builds")
}

fn fixture_creature() -> CreatureSaveState {
    CreatureSaveState {
        organism_id: OrganismId(1),
        genome_id: GenomeId(17),
        brain_class: BrainScaleTier::Nano512,
        development_tick: Tick::new(3),
        appearance: alife_world::CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick: Tick::new(3),
            homeostasis: HomeostaticSnapshot::baseline(Tick::new(3)),
            memory_record_count: 2,
            memory_source_ids: vec![],
            concept_count: 1,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".to_string(),
            diagnostics: vec!["fixture".to_string()],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: Some("tiny-generated-weights".to_string()),
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
    }
}

fn fixture_manifest() -> AssetManifest {
    AssetManifest {
        schema: P34_ASSET_MANIFEST_SCHEMA.to_string(),
        schema_version: P34_ASSET_MANIFEST_SCHEMA_VERSION,
        entries: vec![AssetManifestEntry {
            asset_id: "tiny-generated-weights".to_string(),
            kind: AssetKind::GeneratedWeights,
            relative_path: "assets/optional_tiny_weights.json".to_string(),
            digest: PortableAssetDigest("fnv1a64:0000000000000001".to_string()),
            presence: AssetPresence::Optional,
            schema_version: 1,
            size_bytes: None,
            provenance: Some("optional fixture reference only".to_string()),
        }],
    }
}

fn temp_root(test_name: &str) -> PathBuf {
    let root = std::env::temp_dir().join("alife_p34_tests").join(format!(
        "{}_{}",
        test_name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("create temp root");
    root
}

#[test]
fn tiny_save_load_round_trip_restores_stable_world_and_summaries() {
    let mut world = fixture_world();
    world.advance_tick();
    world.advance_tick();

    let config = RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512);
    let save = PortableSaveFile::from_headless_world(
        "tiny-p34-save",
        &world,
        config,
        fixture_manifest(),
        vec![fixture_creature()],
    )
    .expect("create portable save");

    save.validate_with_asset_root(temp_root("round_trip"))
        .unwrap();
    let json = save.to_json_string_pretty().unwrap();
    let loaded = PortableSaveFile::from_json_str(&json).unwrap();
    loaded
        .validate_with_asset_root(temp_root("round_trip_load"))
        .unwrap();

    let restored = loaded.restore_headless_world().unwrap();
    assert_eq!(restored.seed(), world.seed());
    assert_eq!(restored.tick(), world.tick());
    assert_eq!(restored.stable_signature(), world.stable_signature());
    assert_eq!(loaded.creatures[0].mind.memory_record_count, 2);
    assert_eq!(
        loaded.creatures[0]
            .weights
            .generated_weight_asset_id
            .as_deref(),
        Some("tiny-generated-weights")
    );
}

#[test]
fn incompatible_schemas_reject_without_silent_migration() {
    let world = fixture_world();
    let mut save = PortableSaveFile::from_headless_world(
        "schema-reject",
        &world,
        RuntimeConfig::deterministic_default(7, BrainScaleTier::Nano512),
        fixture_manifest(),
        vec![fixture_creature()],
    )
    .unwrap();
    save.schema_version = P34_SAVE_FILE_SCHEMA_VERSION + 1;
    assert!(matches!(
        save.validate_with_asset_root(temp_root("future_save")),
        Err(PersistenceError::SchemaVersion { .. })
    ));

    let future_save_json = serde_json::json!({
        "schema": P34_SAVE_FILE_SCHEMA,
        "schema_version": P34_SAVE_FILE_SCHEMA_VERSION + 99
    })
    .to_string();
    assert!(matches!(
        PortableSaveFile::from_json_str(&future_save_json),
        Err(PersistenceError::SchemaVersion { .. })
    ));

    let mut config = RuntimeConfig::deterministic_default(8, BrainScaleTier::Nano512);
    config.schema_version += 1;
    assert!(matches!(
        config.validate(),
        Err(PersistenceError::SchemaVersion { .. })
    ));

    let mut manifest = fixture_manifest();
    manifest.schema_version += 1;
    assert!(matches!(
        manifest.validate_with_root(temp_root("future_manifest")),
        Err(PersistenceError::SchemaVersion { .. })
    ));

    let migration = MigrationHook {
        schema_version: 1,
        from_schema_version: P34_SAVE_FILE_SCHEMA_VERSION - 1,
        to_schema_version: P34_SAVE_FILE_SCHEMA_VERSION,
    };
    assert!(matches!(
        migration.reject_premature_migration(),
        Err(PersistenceError::MigrationUnsupported { .. })
    ));
}

#[test]
fn stable_id_remap_rejects_engine_local_id_leakage() {
    let valid = AdapterRemapTable {
        entries: vec![AdapterRemapEntry {
            stable_world_entity_id: WorldEntityId(7),
            adapter_namespace: "headless".to_string(),
            adapter_slot: "slot-7".to_string(),
        }],
    };
    valid.validate().unwrap();

    let invalid = AdapterRemapTable {
        entries: vec![AdapterRemapEntry {
            stable_world_entity_id: WorldEntityId(7),
            adapter_namespace: "headless".to_string(),
            adapter_slot: "Entity(7v1)".to_string(),
        }],
    };
    assert!(matches!(
        invalid.validate(),
        Err(PersistenceError::EngineLocalIdLeak { .. })
    ));
}

#[test]
fn runtime_config_defaults_are_deterministic_and_invalid_combinations_reject() {
    let left = RuntimeConfig::deterministic_default(99, BrainScaleTier::Nano512);
    let right = RuntimeConfig::deterministic_default(99, BrainScaleTier::Nano512);
    assert_eq!(left, right);
    left.validate().unwrap();

    let mut invalid_backend = left.clone();
    invalid_backend.backend.requested = BackendSelection::GpuFull;
    invalid_backend.backend.gpu_feature_enabled = false;
    assert!(matches!(
        invalid_backend.validate(),
        Err(PersistenceError::InvalidConfig { .. })
    ));

    let mut invalid_brain = left.clone();
    invalid_brain.brain_class = BrainScaleTier::ResearchCustom;
    assert!(matches!(
        invalid_brain.validate(),
        Err(PersistenceError::InvalidConfig { .. })
    ));

    let mut invalid_school = left;
    invalid_school.features = FeatureFlagConfig {
        school_enabled: false,
        ..invalid_school.features
    };
    invalid_school.school = SchoolConfig {
        teacher_enabled: true,
        ..invalid_school.school
    };
    assert!(matches!(
        invalid_school.validate(),
        Err(PersistenceError::InvalidConfig { .. })
    ));
}

#[test]
fn asset_manifest_validates_required_optional_and_digest_mismatch() {
    let root = temp_root("assets");
    let asset_path = root.join("assets/tiny_weights.json");
    fs::create_dir_all(asset_path.parent().unwrap()).unwrap();
    let payload = br#"{"tiny":true}"#;
    fs::write(&asset_path, payload).unwrap();

    let digest = PortableAssetDigest::for_bytes(payload);
    let manifest = AssetManifest {
        schema: P34_ASSET_MANIFEST_SCHEMA.to_string(),
        schema_version: P34_ASSET_MANIFEST_SCHEMA_VERSION,
        entries: vec![
            AssetManifestEntry {
                asset_id: "tiny-generated-weights".to_string(),
                kind: AssetKind::GeneratedWeights,
                relative_path: "assets/tiny_weights.json".to_string(),
                digest: digest.clone(),
                presence: AssetPresence::Required,
                schema_version: 1,
                size_bytes: Some(payload.len() as u64),
                provenance: Some("test fixture".to_string()),
            },
            AssetManifestEntry {
                asset_id: "optional-etf".to_string(),
                kind: AssetKind::EtfPrototypes,
                relative_path: "assets/missing_optional.json".to_string(),
                digest,
                presence: AssetPresence::Optional,
                schema_version: 1,
                size_bytes: None,
                provenance: None,
            },
        ],
    };
    manifest.validate_with_root(&root).unwrap();

    let mut missing_required = manifest.clone();
    missing_required.entries[0].relative_path = "assets/missing_required.json".to_string();
    assert!(matches!(
        missing_required.validate_with_root(&root),
        Err(PersistenceError::MissingRequiredAsset { .. })
    ));

    let mut mismatch = manifest;
    mismatch.entries[0].digest = PortableAssetDigest("fnv1a64:ffffffffffffffff".to_string());
    assert!(matches!(
        mismatch.validate_with_root(&root),
        Err(PersistenceError::DigestMismatch { .. })
    ));
}

#[test]
fn json_asset_digest_is_stable_across_line_endings() {
    let root = temp_root("json_asset_digest_line_endings");
    fs::create_dir_all(&root).unwrap();
    let asset_path = root.join("asset.json");
    fs::write(&asset_path, b"{\n  \"value\": 1\n}\n").unwrap();
    let lf_digest = PortableAssetDigest::for_file(&asset_path).unwrap();

    fs::write(&asset_path, b"{\r\n  \"value\": 1\r\n}\r\n").unwrap();
    let crlf_digest = PortableAssetDigest::for_file(&asset_path).unwrap();

    assert_eq!(lf_digest, crlf_digest);
}

#[test]
fn learning_values_and_genetic_lifetime_boundaries_validate() {
    let world = fixture_world();
    let mut save = PortableSaveFile::from_headless_world(
        "learning-boundaries",
        &world,
        RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512),
        fixture_manifest(),
        vec![fixture_creature()],
    )
    .unwrap();

    save.validate_with_asset_root(temp_root("learning_ok"))
        .unwrap();
    assert!(!save.creatures[0].weights.genetic_layer_mutable);
    assert!(!save.creatures[0].learning.lamarckian_mode_enabled);

    save.creatures[0].mind.homeostasis.drives.hunger = f32::NAN;
    assert!(matches!(
        save.validate_with_asset_root(temp_root("learning_nan")),
        Err(PersistenceError::Contract(_))
    ));

    save.creatures[0].mind.homeostasis.drives.hunger = 0.25;
    save.creatures[0].weights.genetic_layer_mutable = true;
    assert!(matches!(
        save.validate_with_asset_root(temp_root("genetic_mutable")),
        Err(PersistenceError::GeneticLayerMutable)
    ));
}

#[test]
fn committed_p34_fixture_files_load_and_validate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/p34");
    let save = PortableSaveFile::from_json_file(root.join("tiny_save.json")).unwrap();
    save.validate_with_asset_root(&root).unwrap();
    let config = RuntimeConfig::from_json_file(root.join("tiny_config.json")).unwrap();
    config.validate().unwrap();
    let manifest = AssetManifest::from_json_file(root.join("tiny_asset_manifest.json")).unwrap();
    manifest.validate_with_root(&root).unwrap();
}
