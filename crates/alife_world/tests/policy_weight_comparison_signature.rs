use alife_core::{
    Blake3Digest, BodyEventDelta, BrainCapacityClass, BrainGenome, BrainPhenotype, BrainScaleTier,
    DevelopmentState, FoundationPromotionReceipt, FoundationWeightAsset, NormalizedScalar,
    OrganismId, PhenotypeCompiler, SensorProfile, Tick, TrainingStageManifest, Vec3f,
};
use alife_world::{
    create_canonical_new_game_with_n2048_candidate, CanonicalNewGameConfig, HeadlessWorld,
    WorldOrganismRecord,
};

fn phenotype() -> BrainPhenotype {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(539_363_617, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap()
}

fn asset(delta: f32, stage: TrainingStageManifest) -> FoundationWeightAsset {
    let phenotype = phenotype();
    let mut weights: Vec<_> = phenotype
        .synapses()
        .iter()
        .map(|s| s.genetic_weight())
        .collect();
    weights[0] += delta;
    FoundationWeightAsset::from_trained_weights(&phenotype, weights, stage).unwrap()
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

fn subject(world: &HeadlessWorld) -> OrganismId {
    world.organism_entity_ids()[0].0
}

#[test]
fn policy_only_changes_match_without_changing_canonical_world_identity() {
    let original = world(&asset(0.0, TrainingStageManifest::bootstrap()), 1);
    let changed = world(&asset(0.01, TrainingStageManifest::bootstrap()), 1);
    let id = subject(&original);
    let before = original.canonical_signature_digest().unwrap();
    assert_ne!(before, changed.canonical_signature_digest().unwrap());
    assert_eq!(
        original.policy_weight_comparison_signature_v1(id).unwrap(),
        changed.policy_weight_comparison_signature_v1(id).unwrap()
    );
    assert_eq!(original.canonical_signature_digest().unwrap(), before);
}

#[test]
fn training_stage_and_promotion_provenance_are_not_treatments() {
    let stage = TrainingStageManifest::bootstrap();
    let original = world(&asset(0.0, stage), 1);
    let id = subject(&original);
    let baseline = original.policy_weight_comparison_signature_v1(id).unwrap();
    let staged = world(&asset(0.0, TrainingStageManifest::new(1, 2, 3)), 1);
    assert_ne!(
        baseline,
        staged.policy_weight_comparison_signature_v1(id).unwrap()
    );
    let source = asset(0.0, stage);
    let receipt = FoundationPromotionReceipt::promoted(
        stage.digest(),
        Blake3Digest::from_bytes([1; 32]),
        Blake3Digest::from_bytes([2; 32]),
    )
    .unwrap();
    let promoted = FoundationWeightAsset::from_promoted_weights(
        &phenotype(),
        source.weights().to_vec(),
        stage,
        receipt,
    )
    .unwrap();
    assert_ne!(
        baseline,
        world(&promoted, 1)
            .policy_weight_comparison_signature_v1(id)
            .unwrap()
    );
}

#[test]
fn another_organisms_policy_and_the_designated_subject_are_bound() {
    let original = world(&asset(0.0, TrainingStageManifest::bootstrap()), 2);
    let changed = world(&asset(0.01, TrainingStageManifest::bootstrap()), 2);
    let ids = original.organism_entity_ids();
    assert_ne!(
        original
            .policy_weight_comparison_signature_v1(ids[0].0)
            .unwrap(),
        changed
            .policy_weight_comparison_signature_v1(ids[0].0)
            .unwrap()
    );
    assert_ne!(
        original
            .policy_weight_comparison_signature_v1(ids[0].0)
            .unwrap(),
        original
            .policy_weight_comparison_signature_v1(ids[1].0)
            .unwrap()
    );
    assert!(original
        .policy_weight_comparison_signature_v1(OrganismId(u64::MAX))
        .is_err());
}

#[test]
fn world_tick_position_terrain_speech_tracking_and_lifecycle_rules_remain_bound() {
    let original = world(&asset(0.0, TrainingStageManifest::bootstrap()), 1);
    let id = subject(&original);
    let baseline = original.policy_weight_comparison_signature_v1(id).unwrap();
    let mut tick = original.clone();
    tick.advance_tick();
    let mut position = original.clone();
    position
        .editor_move_object(
            original.organism_entity_ids()[0].1,
            Vec3f::new(9.0, 0.0, 3.0),
        )
        .unwrap();
    let mut terrain = original.clone();
    terrain.enable_island_for_new_game().unwrap();
    let mut speech = original.clone();
    speech
        .emit_player_tokens(
            Some(id),
            Vec3f::ZERO,
            vec![alife_core::LanguageTokenId::new(9).unwrap()],
        )
        .unwrap();
    let mut tracking = original.clone();
    let mut tracked = tracking.tracked_objects().save_state(id).unwrap();
    tracked.capacity -= 1;
    tracking.restore_tracked_object_states([tracked]).unwrap();
    let mut lifecycle_rule = original.clone();
    lifecycle_rule
        .set_age_death_disabled_for_new_game(true)
        .unwrap();
    for (label, candidate) in [
        ("tick", tick),
        ("position", position),
        ("terrain", terrain),
        ("speech", speech),
        ("tracking", tracking),
        ("lifecycle rule", lifecycle_rule),
    ] {
        assert_ne!(
            baseline,
            candidate.policy_weight_comparison_signature_v1(id).unwrap(),
            "{label}"
        );
    }
}

#[test]
fn physiology_lifecycle_and_nonpolicy_genome_state_remain_bound() {
    let original = world(&asset(0.0, TrainingStageManifest::bootstrap()), 1);
    let id = subject(&original);
    let baseline = original.policy_weight_comparison_signature_v1(id).unwrap();
    let mut biology = original.clone();
    let mut record = biology.organism_registry().get(id).unwrap().clone();
    record
        .advance_biology(Tick(1), BodyEventDelta::zero())
        .unwrap();
    biology
        .replace_organism_record_exact(record.clone())
        .unwrap();
    assert_ne!(
        baseline,
        biology.policy_weight_comparison_signature_v1(id).unwrap()
    );
    record.mark_dead(Tick(1)).unwrap();
    record
        .link_birth_manifest(Blake3Digest::from_bytes([1; 32]))
        .unwrap();
    record
        .link_life_manifest(Blake3Digest::from_bytes([2; 32]))
        .unwrap();
    biology.replace_organism_record_exact(record).unwrap();
    assert_ne!(
        baseline,
        biology.policy_weight_comparison_signature_v1(id).unwrap()
    );
    let mut genome = original.clone();
    let mut value = serde_json::to_value(genome.organism_registry().get(id).unwrap()).unwrap();
    value["genome"]["lineage_id"] = serde_json::json!(999_999_u64);
    let record: WorldOrganismRecord = serde_json::from_value(value).unwrap();
    genome.replace_organism_record_exact(record).unwrap();
    assert_ne!(
        baseline,
        genome.policy_weight_comparison_signature_v1(id).unwrap()
    );
}

#[test]
fn missing_candidate_and_malformed_payloads_cannot_be_projected() {
    let source = asset(0.0, TrainingStageManifest::bootstrap());
    let original = world(&source, 1);
    let id = subject(&original);
    let mut missing = original.clone();
    let mut record = serde_json::to_value(missing.organism_registry().get(id).unwrap()).unwrap();
    record["genome"]
        .as_object_mut()
        .unwrap()
        .remove("n2048_foundation_candidate");
    let record: WorldOrganismRecord = serde_json::from_value(record).unwrap();
    // Either registry admission rejects absence or comparison explicitly does.
    if missing.replace_organism_record_exact(record).is_ok() {
        assert!(missing.policy_weight_comparison_signature_v1(id).is_err());
    }
    for path in ["digest", "count"] {
        let mut value = serde_json::to_value(&source).unwrap();
        if path == "digest" {
            let changed = value["digest"][0].as_u64().unwrap() ^ 1;
            value["digest"][0] = serde_json::json!(changed);
            value["manifest"]["weight_asset"]["digest"][0] = serde_json::json!(changed);
        } else {
            value["manifest"]["weight_asset"]["weight_count"] = serde_json::json!(1);
        }
        let malformed: FoundationWeightAsset = serde_json::from_value(value).unwrap();
        assert!(malformed.encode_canonical().is_err(), "{path}");
    }
}
