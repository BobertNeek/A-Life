use std::{fs, path::Path, path::PathBuf};

use alife_core::{
    BrainCapacityClass, BrainScaleTier, CreatureGenome, FoundationGeneticIdentity,
    FoundationWeightAsset, HomeostaticSnapshot, MemoryId, OrganismId, PhenotypeCompiler,
    SensorProfile, Tick, Vec3f,
};
use alife_game_app::{CompositePopulationRuntime, MINIMUM_POST_RESTORE_TICKS};
use alife_world::{
    persist_composite_genetic_birth_assets, persist_creature_lifetime_state_asset, AssetManifest,
    CreatureAppearanceGenome, CreatureLifetimeMemoryRecord, CreatureLifetimeStateAsset,
    CreatureLifetimeWeightValue, CreatureMindSaveSummary, CreatureSaveState, Habitat, HabitatActor,
    HabitatAuthority, HabitatBreedingKind, HabitatId, HabitatMode, HeadlessScenarioBuilder,
    LearningTraceSaveSummary, PortableSaveFile, RuntimeConfig, WeightLayerSaveSummary,
    P34_ASSET_MANIFEST_SCHEMA, P34_ASSET_MANIFEST_SCHEMA_VERSION,
};

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "alife-composite-population-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

fn habitat(raw: u64) -> HabitatId {
    HabitatId::new(raw).unwrap()
}

fn write_managed_population(root: &Path) -> (PathBuf, [alife_core::GenomeId; 2]) {
    let seed = 90_001;
    let organisms = [OrganismId(11), OrganismId(12)];
    let foundation_identity = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let genomes = [
        CreatureGenome::early_mammal_founder(0xE10_611, foundation_identity).unwrap(),
        CreatureGenome::early_mammal_founder(0xE10_612, foundation_identity).unwrap(),
    ];
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let mut entries = Vec::new();
    let mut creatures = Vec::new();

    for (index, (organism_id, genome)) in organisms.into_iter().zip(&genomes).enumerate() {
        let expressed = genome.express().unwrap();
        let development = expressed
            .development_state_at(Tick::new(u64::from(
                expressed.development.maturation_duration_ticks,
            )))
            .unwrap();
        let phenotype = PhenotypeCompiler::compile_from_foundation_asset(
            &expressed.brain_genome,
            &BrainCapacityClass::n2048(),
            &development,
            SensorProfile::GroundedObjectSlotsV1,
            &foundation,
        )
        .unwrap();
        let (composite_ref, composite_entries) = persist_composite_genetic_birth_assets(
            root,
            genome,
            &foundation,
            phenotype.phenotype_hash(),
        )
        .unwrap();
        for entry in composite_entries {
            if !entries
                .iter()
                .any(|present: &alife_world::AssetManifestEntry| present.asset_id == entry.asset_id)
            {
                entries.push(entry);
            }
        }
        let lifetime = CreatureLifetimeStateAsset {
            schema_version: 1,
            organism_id,
            memory_records: vec![CreatureLifetimeMemoryRecord {
                memory_id: MemoryId(101 + index as u64),
                source_organism_id: organism_id,
                value_q16: 31_000 + index as u32,
            }],
            lifetime_weight_values: vec![CreatureLifetimeWeightValue {
                synapse_index: 41 + index as u32,
                value: 0.125 + index as f32 * 0.125,
            }],
        };
        let (lifetime_ref, lifetime_entry) =
            persist_creature_lifetime_state_asset(root, &lifetime).unwrap();
        entries.push(lifetime_entry);
        creatures.push(CreatureSaveState {
            organism_id,
            genome_id: genome.id,
            brain_class: BrainScaleTier::Standard2048,
            development_tick: Tick::ZERO,
            appearance: CreatureAppearanceGenome::default(),
            mind: CreatureMindSaveSummary {
                tick: Tick::ZERO,
                homeostasis: HomeostaticSnapshot::baseline(Tick::ZERO),
                memory_record_count: 1,
                memory_source_ids: vec![lifetime.memory_records[0].memory_id],
                concept_count: 0,
                edge_count: 0,
                simplex_count: 0,
                unresolved_gap_count: 0,
                sleep_state_label: "awake".to_string(),
                diagnostics: vec!["restored runtime fixture".to_string()],
            },
            weights: WeightLayerSaveSummary {
                generated_weight_asset_id: None,
                genetic_fixed_digest: "fnv1a64:0000000000000001".to_string(),
                genetic_layer_mutable: false,
                lifetime_consolidated_entries: 1,
                h_operational_entries: 1,
                h_shadow_entries: 0,
            },
            learning: LearningTraceSaveSummary {
                lifetime_learning_enabled: true,
                lamarckian_mode_enabled: false,
                last_consolidated_tick: Some(Tick::ZERO),
            },
            composite_genetics: Some(composite_ref),
            lifetime_state_asset: Some(lifetime_ref),
            gpu_brain: None,
        });
    }

    let managed = habitat(3);
    let mut authority = HabitatAuthority::new(vec![
        Habitat::new(HabitatId::DEFAULT_WILD, "Wild", HabitatMode::Wild).unwrap(),
        Habitat::new(managed, "Managed", HabitatMode::Managed).unwrap(),
    ])
    .unwrap();
    for organism_id in organisms {
        authority
            .register_creature(organism_id, managed, Tick::ZERO)
            .unwrap();
    }
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("parent-a", organisms[0], Vec3f::ZERO)
        .social_agent("parent-b", organisms[1], Vec3f::new(0.5, 0.0, 0.0), 0.8)
        .build()
        .unwrap();
    world.replace_habitat_authority(authority).unwrap();
    let save = PortableSaveFile::from_headless_world(
        "managed-restored-population",
        &world,
        RuntimeConfig::deterministic_default(seed, BrainScaleTier::Standard2048),
        AssetManifest {
            schema: P34_ASSET_MANIFEST_SCHEMA.to_string(),
            schema_version: P34_ASSET_MANIFEST_SCHEMA_VERSION,
            entries,
        },
        creatures,
    )
    .unwrap();
    let path = root.join("managed.alife.json");
    save.to_json_file(&path).unwrap();
    (path, [genomes[0].id, genomes[1].id])
}

#[test]
fn managed_player_command_breeds_only_restored_parents_after_128_world_ticks() {
    let root = temp_root("managed");
    let (save_path, parent_genomes) = write_managed_population(&root);
    let mut runtime = CompositePopulationRuntime::restore_from_file(&save_path, &root).unwrap();

    runtime.advance_ticks(MINIMUM_POST_RESTORE_TICKS - 1);
    assert!(runtime
        .apply_player_breed_command(
            habitat(3),
            OrganismId(11),
            OrganismId(12),
            OrganismId(21),
            71
        )
        .is_err());
    runtime.advance_ticks(1);
    let receipt = runtime
        .apply_player_breed_command(
            habitat(3),
            OrganismId(11),
            OrganismId(12),
            OrganismId(21),
            71,
        )
        .unwrap();

    assert_eq!(receipt.breeding.actor, HabitatActor::Player);
    assert_eq!(receipt.breeding.kind, HabitatBreedingKind::Explicit);
    assert_eq!(receipt.post_restore_ticks, MINIMUM_POST_RESTORE_TICKS);
    assert_eq!(receipt.child_generation, 1);
    assert_eq!(receipt.parent_genome_ids, parent_genomes);
    assert!(receipt.first_parent_lifetime.memory_records > 0);
    assert!(receipt.first_parent_lifetime.lifetime_weights > 0);
    assert!(receipt.second_parent_lifetime.memory_records > 0);
    assert!(receipt.second_parent_lifetime.lifetime_weights > 0);
    assert_ne!(
        receipt.first_parent_lifetime.state_digest,
        receipt.second_parent_lifetime.state_digest
    );
    assert_eq!(receipt.child_lifetime.memory_records, 0);
    assert_eq!(receipt.child_lifetime.lifetime_weights, 0);
    assert_eq!(
        runtime
            .resident(OrganismId(21))
            .unwrap()
            .genome
            .parent_genome_ids,
        parent_genomes
    );

    fs::remove_dir_all(root).unwrap();
}
