use std::{fs, path::PathBuf};

use alife_core::{
    BrainScaleTier, GenomeId, HomeostaticSnapshot, OrganismId, PolicyBackend, Tick, Vec3f,
    WorldEntityId,
};
use alife_world::{
    persistence::{
        AdapterRemapEntry, AdapterRemapTable, AssetKind, AssetManifest, AssetManifestEntry,
        AssetPresence, BrainPolicyConfig, CreatureMindSaveSummary, CreatureSaveState,
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
        gpu_brain: None,
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

fn assert_no_runtime_fallback_keys(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            for forbidden in ["fallback_to_cpu", "gpu_feature_enabled", "require_gpu"] {
                assert!(
                    !fields.contains_key(forbidden),
                    "serialized policy intent contains forbidden runtime field {forbidden}"
                );
            }
            for child in fields.values() {
                assert_no_runtime_fallback_keys(child);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                assert_no_runtime_fallback_keys(child);
            }
        }
        _ => {}
    }
}

fn load_runtime_config_value(test_name: &str, value: &serde_json::Value) -> RuntimeConfig {
    let root = temp_root(test_name);
    let path = root.join("runtime_config.json");
    fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    RuntimeConfig::from_json_file(path).unwrap()
}

fn legacy_runtime_config(
    requested: &str,
    gpu_feature_enabled: bool,
    fallback_to_cpu: bool,
) -> serde_json::Value {
    let mut value: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/p34/tiny_config.json")).unwrap();
    let object = value.as_object_mut().unwrap();
    replace_brain_policy_with_legacy_backend(
        object,
        requested,
        gpu_feature_enabled,
        fallback_to_cpu,
    );
    value
}

fn replace_brain_policy_with_legacy_backend(
    config: &mut serde_json::Map<String, serde_json::Value>,
    requested: &str,
    gpu_feature_enabled: bool,
    fallback_to_cpu: bool,
) {
    config.remove("brain_policy");
    config.insert(
        "backend".to_string(),
        serde_json::json!({
            "requested": requested,
            "gpu_feature_enabled": gpu_feature_enabled,
            "fallback_to_cpu": fallback_to_cpu,
            "validation_required": true
        }),
    );
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
    assert!(loaded.creatures[0].gpu_brain.is_none());
    assert_eq!(
        loaded.creatures[0]
            .weights
            .generated_weight_asset_id
            .as_deref(),
        Some("tiny-generated-weights")
    );
}

#[test]
fn legacy_world_objects_migrate_by_canonical_order_and_resave_current_identity() {
    let save = PortableSaveFile::from_headless_world(
        "legacy-grounded-migration",
        &fixture_world(),
        RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512),
        fixture_manifest(),
        vec![fixture_creature()],
    )
    .unwrap();
    let mut canonical = serde_json::to_value(&save).unwrap();
    canonical["world"]
        .as_object_mut()
        .unwrap()
        .remove("next_spawn_sequence");
    for object in canonical["world"]["objects"].as_array_mut().unwrap() {
        let object = object.as_object_mut().unwrap();
        object.remove("schema_version");
        object.remove("grounded_physical");
        object.remove("tracking_provenance");
        object.remove("tracking_key");
    }
    let mut reversed = canonical.clone();
    reversed["world"]["objects"]
        .as_array_mut()
        .unwrap()
        .reverse();

    let canonical_loaded = PortableSaveFile::from_json_str(&canonical.to_string()).unwrap();
    let reversed_loaded = PortableSaveFile::from_json_str(&reversed.to_string()).unwrap();

    assert_eq!(canonical_loaded.world, reversed_loaded.world);
    assert_eq!(
        canonical_loaded.world.next_spawn_sequence,
        canonical_loaded.world.objects.len() as u64 + 1
    );
    for (index, object) in canonical_loaded.world.objects.iter().enumerate() {
        assert_eq!(object.tracking_provenance.spawn_sequence, index as u64 + 1);
        assert_eq!(
            object.tracking_key,
            object.tracking_provenance.canonical_key()
        );
    }
    let current = serde_json::to_value(&canonical_loaded).unwrap();
    assert!(current["world"].get("next_spawn_sequence").is_some());
    for object in current["world"]["objects"].as_array().unwrap() {
        assert!(object.get("schema_version").is_some());
        assert!(object.get("grounded_physical").is_some());
        assert!(object.get("tracking_provenance").is_some());
        assert!(object.get("tracking_key").is_some());
    }
}

#[test]
fn current_world_objects_reject_tampered_or_partial_tracking_identity() {
    let save = PortableSaveFile::from_headless_world(
        "grounded-tamper-rejection",
        &fixture_world(),
        RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512),
        fixture_manifest(),
        vec![fixture_creature()],
    )
    .unwrap();
    let current = serde_json::to_value(&save).unwrap();

    let mut tampered = current.clone();
    tampered["world"]["objects"][0]["tracking_key"][0] = serde_json::json!(0);
    assert!(PortableSaveFile::from_json_str(&tampered.to_string()).is_err());

    let mut partial = current;
    partial["world"]["objects"][0]
        .as_object_mut()
        .unwrap()
        .remove("tracking_key");
    assert!(PortableSaveFile::from_json_str(&partial.to_string()).is_err());
}

#[test]
fn appearance_schema_v1_migrates_and_schema_v2_roundtrips_in_saves() {
    let mut save = PortableSaveFile::from_headless_world(
        "appearance-migration",
        &fixture_world(),
        RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512),
        fixture_manifest(),
        vec![fixture_creature()],
    )
    .unwrap();
    let legacy = serde_json::json!({
        "schema_version": 1,
        "species_archetype": 5,
        "palette_family": 3,
        "fur_pattern": 4,
        "marking_density": 8,
        "accessory_trait": 2,
        "ear_muzzle_trait": 6,
        "tail_trait": 7,
        "body_mass_trait": 9,
        "mutation_count": 0,
        "bipedal_caveman_furry": true
    });
    let mut value = serde_json::to_value(&save).unwrap();
    value["creatures"][0]["appearance"] = legacy;

    let loaded = PortableSaveFile::from_json_str(&value.to_string()).unwrap();
    assert_eq!(
        loaded.creatures[0].appearance.part_sources,
        alife_world::CreaturePartSources::coherent(alife_world::CreaturePartFamilyId(5))
    );

    save.creatures[0].appearance.part_sources = alife_world::CreaturePartSources {
        head: alife_world::CreaturePartFamilyId(1),
        torso: alife_world::CreaturePartFamilyId(5),
        arms: alife_world::CreaturePartFamilyId(6),
        legs: alife_world::CreaturePartFamilyId(5),
        tail: alife_world::CreaturePartFamilyId(7),
    };
    let roundtrip =
        PortableSaveFile::from_json_str(&save.to_json_string_pretty().unwrap()).unwrap();
    assert_eq!(
        roundtrip.creatures[0].appearance,
        save.creatures[0].appearance
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

    assert_eq!(left.brain_policy.policy, PolicyBackend::NeuralClosedLoopGpu);
    assert!(left.brain_policy.policy.requires_gpu());

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
fn brain_policy_config_vnext_round_trips_without_runtime_fallback_state() {
    for policy in [
        PolicyBackend::NeuralClosedLoopGpu,
        PolicyBackend::HeuristicBaseline,
    ] {
        let expected = BrainPolicyConfig {
            schema_version: 1,
            policy,
        };
        let value = serde_json::to_value(expected).unwrap();
        assert_no_runtime_fallback_keys(&value);
        assert_eq!(
            serde_json::from_value::<BrainPolicyConfig>(value).unwrap(),
            expected
        );
    }
}

#[test]
fn requires_gpu_is_derived_only_from_the_explicit_policy() {
    assert!(PolicyBackend::NeuralClosedLoopGpu.requires_gpu());
    assert!(!PolicyBackend::HeuristicBaseline.requires_gpu());
}

#[test]
fn current_runtime_config_serializes_only_explicit_policy_intent() {
    let config = RuntimeConfig::deterministic_default(99, BrainScaleTier::Nano512);
    let value = serde_json::to_value(&config).unwrap();
    assert_no_runtime_fallback_keys(&value);
    assert_eq!(
        value.pointer("/brain_policy/policy"),
        Some(&serde_json::Value::String(
            "NeuralClosedLoopGpu".to_string()
        ))
    );
}

#[test]
fn legacy_cpu_reference_migrates_to_explicit_heuristic_policy() {
    let legacy = legacy_runtime_config("CpuReference", true, false);
    let migrated = load_runtime_config_value("legacy_cpu_policy", &legacy);

    assert_eq!(
        migrated.brain_policy.policy,
        PolicyBackend::HeuristicBaseline
    );
    assert!(!migrated.brain_policy.policy.requires_gpu());
    assert_no_runtime_fallback_keys(&serde_json::to_value(migrated).unwrap());
}

#[test]
fn legacy_gpu_selections_migrate_to_neural_without_runtime_switching() {
    for requested in ["GpuStatic", "GpuPlastic", "GpuFull"] {
        let legacy = legacy_runtime_config(requested, false, true);
        let migrated = load_runtime_config_value(&format!("legacy_{requested}"), &legacy);

        assert_eq!(
            migrated.brain_policy.policy,
            PolicyBackend::NeuralClosedLoopGpu,
            "legacy selection {requested}"
        );
        assert!(migrated.brain_policy.policy.requires_gpu());
        assert_no_runtime_fallback_keys(&serde_json::to_value(migrated).unwrap());
    }
}

#[test]
fn legacy_policy_nested_in_portable_save_migrates_without_runtime_switching() {
    for (requested, expected) in [
        ("CpuReference", PolicyBackend::HeuristicBaseline),
        ("GpuFull", PolicyBackend::NeuralClosedLoopGpu),
    ] {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("fixtures/p34/tiny_save.json")).unwrap();
        let config = value
            .get_mut("config")
            .and_then(serde_json::Value::as_object_mut)
            .unwrap();
        replace_brain_policy_with_legacy_backend(config, requested, false, true);

        let migrated = PortableSaveFile::from_json_str(&value.to_string()).unwrap();
        assert_eq!(
            migrated.config.brain_policy.policy, expected,
            "legacy nested selection {requested}"
        );
        assert_eq!(
            migrated.config.brain_policy.policy.requires_gpu(),
            expected == PolicyBackend::NeuralClosedLoopGpu
        );
        assert_no_runtime_fallback_keys(&serde_json::to_value(migrated).unwrap());
    }
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
    assert_eq!(
        config.brain_policy.policy,
        PolicyBackend::NeuralClosedLoopGpu
    );
    assert_no_runtime_fallback_keys(&serde_json::to_value(&config).unwrap());
    let fixture_config: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/p34/tiny_config.json")).unwrap();
    let fixture_save: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/p34/tiny_save.json")).unwrap();
    assert_no_runtime_fallback_keys(&fixture_config);
    assert_no_runtime_fallback_keys(&fixture_save);
    let manifest = AssetManifest::from_json_file(root.join("tiny_asset_manifest.json")).unwrap();
    manifest.validate_with_root(&root).unwrap();
}
