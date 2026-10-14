use alife_core::{
    BrainCapacityClass, BrainPhenotype, BrainScaleTier, CandidateActionFamily as Family,
    CompiledSynapseKind, CreatureGenome, DecoderHeadKind as Head,
    DecoderLifetimeRatePolicyV1 as Policy, FoundationGeneticIdentity, FoundationWeightAsset,
    HomeostaticSnapshot, MemoryId, OrganismId, PhenotypeCompiler, SensorProfile, Tick, Vec3f,
};
use alife_world::{
    persist_composite_genetic_birth_assets, persist_creature_lifetime_state_asset, AssetManifest,
    CreatureLifetimeMemoryRecord, CreatureLifetimeStateAsset, CreatureLifetimeWeightValue,
    CreatureMindSaveSummary, CreatureSaveState, HeadlessScenarioBuilder, LearningTraceSaveSummary,
    PortableSaveFile, RuntimeConfig, WeightLayerSaveSummary, P34_ASSET_MANIFEST_SCHEMA,
    P34_ASSET_MANIFEST_SCHEMA_VERSION,
};
use std::fs;

fn compile(genome: &CreatureGenome, foundation: &FoundationWeightAsset) -> BrainPhenotype {
    let expressed = genome.express().unwrap();
    let development = expressed
        .development_state_at(Tick::new(u64::from(
            expressed.development.maturation_duration_ticks,
        )))
        .unwrap();
    PhenotypeCompiler::compile_from_foundation_asset(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &development,
        SensorProfile::GroundedObjectSlotsV1,
        foundation,
    )
    .unwrap()
}

fn assert_inherited_values_and_effective_rates(
    baseline: &BrainPhenotype,
    selected: &BrainPhenotype,
) {
    assert_eq!(baseline.projections(), selected.projections());
    assert_eq!(baseline.synapses().len(), selected.synapses().len());
    let mut counts = [0usize; 4]; // Contact, Approach, nonzero genetic weight, nonzero alpha
    for (index, (before, after)) in baseline
        .synapses()
        .iter()
        .zip(selected.synapses())
        .enumerate()
    {
        assert_eq!(before.source(), after.source());
        assert_eq!(before.target(), after.target());
        assert_eq!(before.kind(), after.kind());
        assert_eq!(before.route_index(), after.route_index());
        assert_eq!(
            before.genetic_weight().to_bits(),
            after.genetic_weight().to_bits()
        );
        assert_eq!(before.alpha().to_bits(), after.alpha().to_bits());
        counts[2] += usize::from(after.genetic_weight() != 0.0);
        counts[3] += usize::from(after.alpha() != 0.0);
        let left = baseline.plasticity_receptors()[usize::from(before.receptor_index())];
        let right = selected.plasticity_receptors()[usize::from(after.receptor_index())];
        let scale = match after.kind() {
            CompiledSynapseKind::Decoder(coordinate)
                if coordinate.head() == Head::ActionCandidate
                    && coordinate.family() == Family::Contact =>
            {
                counts[0] += 1;
                assert!(!right.is_delta_enabled());
                assert!(!selected
                    .replay_capture_plan()
                    .global_synapse_ids()
                    .contains(&(index as u32)));
                0.0
            }
            CompiledSynapseKind::Decoder(coordinate)
                if coordinate.head() == Head::ActionCandidate
                    && coordinate.family() == Family::Approach =>
            {
                counts[1] += 1;
                0.25
            }
            _ => 1.0,
        };
        if scale == 1.0 {
            assert_eq!(left, right);
        } else {
            for (a, b) in [
                (left.learning_rate(), right.learning_rate()),
                (left.normalization_rate(), right.normalization_rate()),
                (left.sleep_replay_rate(), right.sleep_replay_rate()),
            ] {
                assert_eq!((a * scale).to_bits(), b.to_bits());
            }
            assert_eq!(left.fast_weight_bounds(), right.fast_weight_bounds());
            assert_eq!(left.eligibility_decay(), right.eligibility_decay());
        }
    }
    assert!(
        counts.iter().all(|count| *count > 0),
        "non-vacuous coverage: {counts:?}"
    );
    println!(
        "compared {} synapses; Contact={} Approach={} nonzero genetic={} nonzero alpha={}",
        selected.synapses().len(),
        counts[0],
        counts[1],
        counts[2],
        counts[3]
    );
}

#[test]
fn explicit_inherited_decoder_policy_survives_real_portable_file_save_and_restore() {
    let root = std::env::temp_dir().join(format!(
        "alife-decoder-policy-portable-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).unwrap();
    let identity = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .unwrap();
    let policy = Policy::default()
        .with_action_candidate_family_scale(Family::Contact, Some(0.0))
        .unwrap()
        .with_action_candidate_family_scale(Family::Approach, Some(0.25))
        .unwrap();
    let mut mother = CreatureGenome::early_mammal_founder(0xE10_701, identity).unwrap();
    let mut father = CreatureGenome::early_mammal_founder(0xE10_702, identity).unwrap();
    for parent in [&mut mother, &mut father] {
        parent.brain.construction.maternal_learning = parent
            .brain
            .construction
            .maternal_learning
            .with_decoder_lifetime_rate_policy(Some(policy))
            .unwrap();
        parent.brain.construction.paternal_learning = parent
            .brain
            .construction
            .paternal_learning
            .with_decoder_lifetime_rate_policy(Some(policy))
            .unwrap();
    }
    let child = CreatureGenome::reproduce(&mother, &father, 0xE10_704).unwrap();
    assert_eq!(child.parent_genome_ids, vec![mother.id, father.id]);
    let mut no_policy = child.clone();
    no_policy.brain.construction.maternal_learning = no_policy
        .brain
        .construction
        .maternal_learning
        .with_decoder_lifetime_rate_policy(None)
        .unwrap();
    no_policy.brain.construction.paternal_learning = no_policy
        .brain
        .construction
        .paternal_learning
        .with_decoder_lifetime_rate_policy(None)
        .unwrap();
    let foundation =
        FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1).unwrap();
    let baseline = compile(&no_policy, &foundation);
    let expected = compile(&child, &foundation);
    assert_ne!(baseline.phenotype_hash(), expected.phenotype_hash());
    assert_inherited_values_and_effective_rates(&baseline, &expected);
    let (composite_ref, mut entries) = persist_composite_genetic_birth_assets(
        &root,
        &child,
        &foundation,
        expected.phenotype_hash(),
    )
    .unwrap();
    let lifetime_state = CreatureLifetimeStateAsset {
        schema_version: 1,
        organism_id: OrganismId(1),
        memory_records: vec![CreatureLifetimeMemoryRecord {
            memory_id: MemoryId(41),
            source_organism_id: OrganismId(1),
            value_q16: 41_001,
        }],
        lifetime_weight_values: vec![
            CreatureLifetimeWeightValue {
                synapse_index: 17,
                value: 0.125,
            },
            CreatureLifetimeWeightValue {
                synapse_index: 29,
                value: -0.25,
            },
        ],
    };
    let (lifetime_ref, lifetime_entry) =
        persist_creature_lifetime_state_asset(&root, &lifetime_state).unwrap();
    entries.push(lifetime_entry);
    let creature = CreatureSaveState {
        organism_id: OrganismId(1),
        genome_id: child.id,
        brain_class: BrainScaleTier::Standard2048,
        development_tick: Tick::new(3),
        appearance: alife_world::CreatureAppearanceGenome::default(),
        mind: CreatureMindSaveSummary {
            tick: Tick::new(3),
            homeostasis: HomeostaticSnapshot::baseline(Tick::new(3)),
            memory_record_count: 1,
            memory_source_ids: vec![MemoryId(41)],
            concept_count: 1,
            edge_count: 0,
            simplex_count: 0,
            unresolved_gap_count: 0,
            sleep_state_label: "awake".into(),
            diagnostics: vec!["decoder-rate real-file fixture".into()],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: None,
            genetic_fixed_digest: "fnv1a64:0000000000000001".into(),
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: 2,
            h_operational_entries: 1,
            h_shadow_entries: 1,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: true,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: Some(Tick::new(2)),
        },
        composite_genetics: Some(composite_ref),
        lifetime_state_asset: Some(lifetime_ref),
        gpu_brain: None,
    };
    let mut world = HeadlessScenarioBuilder::new(4242)
        .agent("agent", OrganismId(1), Vec3f::ZERO)
        .food("berry", Vec3f::new(1.0, 0.0, 0.0), 0.75)
        .build()
        .unwrap();
    world.enable_island_for_new_game().unwrap();
    let save = PortableSaveFile::from_headless_world(
        "explicit-decoder-rate-real-save",
        &world,
        RuntimeConfig::deterministic_default(4242, BrainScaleTier::Standard2048),
        AssetManifest {
            schema: P34_ASSET_MANIFEST_SCHEMA.into(),
            schema_version: P34_ASSET_MANIFEST_SCHEMA_VERSION,
            entries,
        },
        vec![creature],
    )
    .unwrap();
    let path = root.join("population.alife.json");
    save.to_json_file(&path).unwrap();
    let restored = PortableSaveFile::from_json_file(&path).unwrap();
    restored.validate_with_asset_root(&root).unwrap();
    assert_eq!(restored.world, save.world);
    assert_eq!(restored.creatures, save.creatures);
    let restored_world = restored.restore_headless_world().unwrap();
    assert_eq!(
        restored_world.canonical_signature_digest().unwrap(),
        save.restore_headless_world()
            .unwrap()
            .canonical_signature_digest()
            .unwrap()
    );
    let birth = restored
        .load_composite_genetic_birth(OrganismId(1), &root)
        .unwrap();
    assert_eq!(birth.creature_genome, child);
    assert_eq!(
        birth.creature_genome.parent_genome_ids,
        vec![mother.id, father.id]
    );
    assert_eq!(birth.creature_genome.lineage_id, child.lineage_id);
    assert_eq!(birth.creature_genome.provenance, child.provenance);
    assert_eq!(birth.foundation.digest(), foundation.digest());
    assert_eq!(birth.phenotype_hash, expected.phenotype_hash());
    let expressed = birth.creature_genome.express().unwrap();
    assert_eq!(
        expressed
            .brain_genome
            .plasticity_parameters()
            .decoder_lifetime_rate_policy(),
        Some(policy)
    );
    let recompiled = compile(&birth.creature_genome, &birth.foundation);
    assert_eq!(recompiled, expected);
    assert_inherited_values_and_effective_rates(&baseline, &recompiled);
    assert_eq!(
        restored
            .load_creature_lifetime_state(OrganismId(1), &root)
            .unwrap(),
        lifetime_state
    );
    assert!(restored.gpu_runtime.is_none());
    assert!(restored.creatures[0].gpu_brain.is_none());
    println!(
        "real save={} genome={} lineage={} parents={:?}; GPU resume intentionally unqualified",
        path.display(),
        child.id.raw(),
        child.lineage_id.raw(),
        child.parent_genome_ids
    );
    fs::remove_dir_all(&root).unwrap();
}
