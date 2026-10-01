use alife_core::{
    BrainScaleTier, CreatureGenome, FoundationGeneticIdentity, FoundationWeightAsset, OrganismId,
    SensorProfile, Tick, Vec3f,
};
use alife_world::{
    AssetManifest, CreatureAppearanceGenome, CreatureMindSaveSummary, CreatureSaveState,
    HeadlessScenarioBuilder, LearningTraceSaveSummary, PortableAssetDigest, PortableSaveFile,
    RuntimeConfig, WeightLayerSaveSummary, WorldObjectKind, WorldOrganismRecord,
};

/// Construct an existing test population through individual birth authority.
/// This is independent of the product's bounded Phase 3 New Game cohort API.
pub fn untrained_population_save(config: RuntimeConfig, population: u16) -> PortableSaveFile {
    assert_eq!(config.brain_class, BrainScaleTier::Nano512);
    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let manifest = foundation.manifest();
    let identity = FoundationGeneticIdentity::new(
        manifest.foundation_id().raw(),
        u16::try_from(manifest.foundation_version().raw()).unwrap(),
        manifest.compatibility_family_id().raw(),
        config.brain_class.default_class_id(),
    )
    .unwrap();
    let mut builder = HeadlessScenarioBuilder::new(config.deterministic_seed)
        .food("population-food", Vec3f::new(2.0, 0.0, 0.0), 0.7)
        .hazard("population-hazard", Vec3f::new(0.0, 0.0, 2.0), 0.2)
        .obstacle("population-obstacle", Vec3f::new(-2.0, 0.0, 0.0), 0.8);
    for slot in 0..population {
        builder = builder.agent(
            &format!("population-founder-{}", slot + 1),
            OrganismId(u64::from(slot) + 1),
            Vec3f::new(f32::from(slot % 10) - 5.0, 0.0, f32::from(slot / 10)),
        );
    }
    let mut world = builder.build().unwrap();
    let mut creatures = Vec::with_capacity(usize::from(population));
    for object in world.object_snapshots() {
        if object.kind != WorldObjectKind::Agent {
            continue;
        }
        let organism_id = object.organism_id.unwrap();
        let founder_seed = config
            .deterministic_seed
            .checked_mul(64)
            .and_then(|seed| seed.checked_add(organism_id.raw()))
            .unwrap();
        let genome = CreatureGenome::early_mammal_founder(founder_seed, identity).unwrap();
        let phenotype = genome.express().unwrap();
        let record =
            WorldOrganismRecord::newborn(organism_id, object.id, genome, phenotype, Tick::ZERO)
                .unwrap();
        let biology = record.biochemistry();
        creatures.push(CreatureSaveState {
            organism_id,
            genome_id: record.genome().id,
            brain_class: config.brain_class,
            development_tick: biology.development.last_update_tick,
            appearance: CreatureAppearanceGenome::founder_for_species(
                u8::try_from(organism_id.raw() - 1).unwrap(),
                founder_seed,
            )
            .with_body_phenotype(&record.phenotype().body),
            mind: CreatureMindSaveSummary {
                tick: biology.tick,
                homeostasis: biology.homeostasis,
                memory_record_count: 0,
                memory_source_ids: Vec::new(),
                concept_count: 0,
                edge_count: 0,
                simplex_count: 0,
                unresolved_gap_count: 0,
                sleep_state_label: "awake".to_string(),
                diagnostics: vec!["test founder awaiting GPU admission".to_string()],
            },
            weights: WeightLayerSaveSummary {
                generated_weight_asset_id: None,
                genetic_fixed_digest: PortableAssetDigest::for_bytes(
                    &serde_json::to_vec(record.genome()).unwrap(),
                )
                .0,
                genetic_layer_mutable: false,
                lifetime_consolidated_entries: 0,
                h_operational_entries: 0,
                h_shadow_entries: 0,
            },
            learning: LearningTraceSaveSummary {
                lifetime_learning_enabled: true,
                lamarckian_mode_enabled: false,
                last_consolidated_tick: None,
            },
            composite_genetics: None,
            lifetime_state_asset: None,
            gpu_brain: None,
        });
        world.register_organism_record(record).unwrap();
    }
    world.validate_organism_bindings().unwrap();
    PortableSaveFile::from_headless_world(
        "current-untrained-test-population",
        &world,
        config,
        AssetManifest::empty(),
        creatures,
    )
    .unwrap()
}
