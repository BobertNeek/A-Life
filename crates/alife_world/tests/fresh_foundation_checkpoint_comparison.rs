use alife_core::{
    Blake3Digest, BodyEventDelta, BrainCapacityClass, BrainGenome, BrainScaleTier,
    DevelopmentState, FoundationPromotionReceipt, FoundationWeightAsset, NormalizedScalar,
    OrganismId, PhenotypeCompiler, SensorProfile, Tick, TrainingStageManifest, Vec3f,
};
use alife_world::{
    create_canonical_new_game_with_n2048_candidate, CanonicalNewGameConfig, HeadlessWorld,
    WorldOrganismRecord,
};

fn asset(delta: f32, stage: TrainingStageManifest, promoted: bool) -> FoundationWeightAsset {
    asset_with_profile(
        delta,
        stage,
        promoted,
        SensorProfile::GroundedTerrainVisionV1,
    )
}

fn asset_with_profile(
    delta: f32,
    stage: TrainingStageManifest,
    promoted: bool,
    profile: SensorProfile,
) -> FoundationWeightAsset {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(539_363_617, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let phenotype = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        profile,
    )
    .unwrap();
    let mut weights: Vec<_> = phenotype
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect();
    weights[0] += delta;
    if promoted {
        let receipt = FoundationPromotionReceipt::promoted(
            stage.digest(),
            Blake3Digest::from_bytes([1; 32]),
            Blake3Digest::from_bytes([2; 32]),
        )
        .unwrap();
        FoundationWeightAsset::from_promoted_weights(&phenotype, weights, stage, receipt).unwrap()
    } else {
        FoundationWeightAsset::from_trained_weights(&phenotype, weights, stage).unwrap()
    }
}

fn world(asset: &FoundationWeightAsset, count: u16) -> HeadlessWorld {
    let mut config = CanonicalNewGameConfig::phase3(71_001, count).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = 539_363_617;
    config.sensor_profile = asset.manifest().sensor_profile();
    create_canonical_new_game_with_n2048_candidate(&config, asset)
        .unwrap()
        .world
}

fn signature(
    world: &HeadlessWorld,
) -> alife_world::HeadlessFreshFoundationCheckpointComparisonSignatureV1 {
    world
        .fresh_foundation_checkpoint_comparison_signature_v1(OrganismId(1))
        .unwrap()
}

#[test]
fn checkpoint_treatments_match_only_in_the_new_domain_without_mutation() {
    let source = asset(0.0, TrainingStageManifest::bootstrap(), false);
    let original = world(&source, 1);
    let canonical = original.canonical_signature_digest().unwrap();
    let bytes =
        serde_json::to_vec(original.organism_registry().get(OrganismId(1)).unwrap()).unwrap();
    for (delta, stage, promoted) in [
        (0.01, TrainingStageManifest::bootstrap(), false),
        (0.0, TrainingStageManifest::new(1, 2, 3), false),
        (0.0, TrainingStageManifest::bootstrap(), true),
        (0.01, TrainingStageManifest::new(1, 2, 3), true),
    ] {
        let treatment = asset(delta, stage, promoted);
        let changed = world(&treatment, 1);
        assert_ne!(
            source.encode_canonical().unwrap(),
            treatment.encode_canonical().unwrap()
        );
        assert_ne!(canonical, changed.canonical_signature_digest().unwrap());
        assert_eq!(signature(&original), signature(&changed));
        if stage != TrainingStageManifest::bootstrap() || promoted {
            assert_ne!(
                original
                    .policy_weight_comparison_signature_v1(OrganismId(1))
                    .unwrap(),
                changed
                    .policy_weight_comparison_signature_v1(OrganismId(1))
                    .unwrap()
            );
        }
    }
    assert_ne!(
        signature(&original).words,
        original
            .policy_weight_comparison_signature_v1(OrganismId(1))
            .unwrap()
            .words
    );
    assert_eq!(canonical, original.canonical_signature_digest().unwrap());
    assert_eq!(
        bytes,
        serde_json::to_vec(original.organism_registry().get(OrganismId(1)).unwrap()).unwrap()
    );
}

#[test]
fn world_body_genome_graph_and_private_references_remain_controls() {
    let original = world(&asset(0.0, TrainingStageManifest::bootstrap(), false), 1);
    let baseline = signature(&original);
    let changed_abi = world(
        &asset_with_profile(
            0.0,
            TrainingStageManifest::bootstrap(),
            false,
            SensorProfile::PrivilegedAffordanceV1,
        ),
        1,
    );
    assert_ne!(baseline, signature(&changed_abi));
    let mut tick = original.clone();
    tick.advance_tick();
    let mut position = original.clone();
    position
        .editor_move_object(
            original.organism_entity_ids()[0].1,
            Vec3f::new(9.0, 0.0, 3.0),
        )
        .unwrap();
    let mut biology = original.clone();
    let mut record = biology
        .organism_registry()
        .get(OrganismId(1))
        .unwrap()
        .clone();
    record
        .advance_biology(Tick(1), BodyEventDelta::zero())
        .unwrap();
    biology.replace_organism_record_exact(record).unwrap();
    let mut terrain = original.clone();
    terrain.enable_island_for_new_game().unwrap();
    let mut tracking = original.clone();
    let mut tracked = tracking
        .tracked_objects()
        .save_state(OrganismId(1))
        .unwrap();
    tracked.next_id += 1;
    tracking.restore_tracked_object_states([tracked]).unwrap();
    let mut speech = original.clone();
    speech
        .emit_player_tokens(
            Some(OrganismId(1)),
            Vec3f::ZERO,
            vec![alife_core::LanguageTokenId::new(9).unwrap()],
        )
        .unwrap();
    let mut rules = original.clone();
    rules.set_age_death_disabled_for_new_game(true).unwrap();
    for changed in [tick, position, biology, terrain, tracking, speech, rules] {
        assert_ne!(baseline, signature(&changed));
    }
    for pointer in [
        "/state_graph/transaction_revision",
        "/state_graph/genetics_development/schema_version",
        "/state_graph/genetics_development/revision",
        "/state_graph/genetics_development/causal_tick",
        "/state_graph/brain/revision",
        "/state_graph/memory/revision",
        "/state_graph/brain/content_digest/0",
        "/state_graph/memory/content_digest/0",
    ] {
        let mut changed = original.clone();
        let mut value =
            serde_json::to_value(changed.organism_registry().get(OrganismId(1)).unwrap()).unwrap();
        let field = value.pointer_mut(pointer).unwrap();
        *field = serde_json::json!(field.as_u64().unwrap() + 1);
        changed
            .replace_organism_record_exact(serde_json::from_value(value).unwrap())
            .unwrap();
        assert_ne!(baseline, signature(&changed), "{pointer}");
    }
    let mut changed = original.clone();
    let record = original.organism_registry().get(OrganismId(1)).unwrap();
    let mut genome = record.genome().clone();
    genome.lineage_id = alife_core::LineageId(999_999);
    let phenotype = genome.express().unwrap();
    changed
        .replace_organism_record_exact(
            WorldOrganismRecord::new(
                OrganismId(1),
                record.world_entity_id(),
                genome,
                phenotype,
                *record.biochemistry(),
                record.birth_tick(),
            )
            .unwrap(),
        )
        .unwrap();
    assert_ne!(baseline, signature(&changed));
}

#[test]
fn other_subjects_and_invalid_assets_cannot_be_projected_away() {
    let source = asset(0.0, TrainingStageManifest::bootstrap(), false);
    let original = world(&source, 2);
    let changed = world(&asset(0.01, TrainingStageManifest::new(1, 2, 3), true), 2);
    assert_ne!(signature(&original), signature(&changed));
    assert_ne!(
        signature(&original),
        original
            .fresh_foundation_checkpoint_comparison_signature_v1(OrganismId(2))
            .unwrap()
    );
    assert!(original
        .fresh_foundation_checkpoint_comparison_signature_v1(OrganismId(u64::MAX))
        .is_err());
    for pointer in [
        "/digest/0",
        "/manifest/weight_asset/weight_count",
        "/manifest/training_stage/completed_stage_count",
    ] {
        let mut value = serde_json::to_value(&source).unwrap();
        let field = value.pointer_mut(pointer).unwrap();
        *field = serde_json::json!(field.as_u64().unwrap() + 1);
        let malformed: FoundationWeightAsset = serde_json::from_value(value).unwrap();
        assert!(malformed.encode_canonical().is_err());
        assert!(create_canonical_new_game_with_n2048_candidate(
            &{
                let mut c = CanonicalNewGameConfig::phase3(71_001, 1).unwrap();
                c.brain_class = BrainScaleTier::Standard2048;
                c.founder_seed_base = 539_363_617;
                c.sensor_profile = source.manifest().sensor_profile();
                c
            },
            &malformed
        )
        .is_err());
    }
}
