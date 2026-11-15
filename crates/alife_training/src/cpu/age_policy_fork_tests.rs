//! Manual readout calibration preserves acquired state; synthetic tests, not efficacy.
use super::*;
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;
fn fixture(profile: SensorProfile) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let grip = true;
    let asset = FoundationWeightAsset::decode_canonical(include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation")).unwrap();
    let mut genome: CreatureGenome = serde_json::from_slice(include_bytes!("../../../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation.genome.json")).unwrap();
    for architecture in [
        &mut genome.brain.construction.maternal_architecture,
        &mut genome.brain.construction.paternal_architecture,
    ] {
        *architecture = architecture.with_memory_candidate_bound_v4(1.0).unwrap();
        if grip {
            *architecture = architecture.with_grip_proprioception_v5().unwrap();
        }
    }
    // Fresh hereditary decoder-plan identity requires a matching foundation ABI.
    // Preserve every weight bit and coordinate; never reinterpret a plan-one asset.
    let seed_expression = genome.express().unwrap();
    let seed_development = foundation_construction_development(
        &seed_expression.brain_genome,
        &BrainCapacityClass::n2048(),
        &seed_expression.development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let (fresh, _) = PhenotypeCompiler::compile_n2048_anchored_v2_graph(
        seed_expression.brain_genome.clone(),
        seed_development,
        profile,
    )
    .expect("fresh inherited architecture graph");
    let rebound = FoundationWeightAsset::from_trained_weights(
        &fresh,
        asset.weights().to_vec(),
        asset.manifest().training_stage(),
    )
    .expect("fresh foundation ABI binding");
    assert_eq!(asset.weights().len(), rebound.weights().len());
    assert!(asset
        .weights()
        .iter()
        .zip(rebound.weights())
        .all(|(a, b)| a.to_bits() == b.to_bits()));
    assert_eq!(
        asset.manifest().address_map_digest(),
        rebound.manifest().address_map_digest()
    );
    genome = genome
        .with_n2048_foundation_candidate(rebound.clone())
        .unwrap();
    let asset = rebound;
    let mut config = CanonicalNewGameConfig::phase3(2306, 1).unwrap();
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = asset.manifest().sensor_profile();
    let mut game =
        create_canonical_new_game_with_n2048_experimental_candidate(&config, &asset).unwrap();
    let owner = game
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let expressed = genome.express().unwrap();
    let development = foundation_construction_development(
        &expressed.brain_genome,
        &BrainCapacityClass::n2048(),
        &expressed.development_state_at(Tick::ZERO).unwrap(),
    )
    .unwrap();
    let (p, _) = PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
        expressed.brain_genome.clone(),
        development,
        asset.clone(),
    )
    .unwrap();
    assert_eq!(p.cognitive_architecture().grip_proprioception(), grip);
    // Entirely fresh synthetic world and biological record, not an acquired-body conversion.
    let mut world = HeadlessScenarioBuilder::new(2306)
        .agent("subject", owner, Vec3f::ZERO)
        .food("near", Vec3f::new(0.25, 0.0, 0.0), 0.4)
        .build()
        .unwrap();
    let biology = BiochemistryState::new(&expressed, Tick::ZERO).unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::new(
                owner,
                world.entity_id("subject").unwrap(),
                genome.clone(),
                expressed,
                biology,
                Tick::ZERO,
            )
            .unwrap(),
        )
        .unwrap();
    game.world = world;
    game.creatures[0].genome_id = genome.id;
    game.creatures[0].mind.tick = biology.tick;
    game.creatures[0].mind.homeostasis = biology.homeostasis;
    game.creatures[0].development_tick = biology.development.last_update_tick;
    game.creatures[0].weights.genetic_fixed_digest =
        PortableAssetDigest::for_bytes(&serde_json::to_vec(&genome).unwrap()).0;
    let mask = crate::StageTrainableMask::recurrent_only(&p).unwrap();
    let trainer = CpuFoundationTrainer::new(p, asset, mask, crate::AdamWConfig::default()).unwrap();
    (
        CpuCanonicalHost::new_cohort(game, &trainer, 2306).unwrap(),
        trainer,
    )
}
fn serialized(host: &CpuCanonicalHost, trainer: &CpuFoundationTrainer) -> Vec<u8> {
    serde_json::to_vec(&host.checkpoint(trainer).unwrap()).unwrap()
}

#[test]
fn acquired_calibrated_policy_fork_only_flag_cold_nextstep_and_original_unchanged() {
    for adapter in [false, true] {
        let (mut host, base) = fixture(SensorProfile::GroundedObjectSlotsV1);
        host.enable_joint_ingest_direct2295(&base, true).unwrap();
        if adapter {
            host.enable_candidate_opportunity_coverage2313(&base)
                .unwrap();
        }
        let high = base
            .calibrated_readout_copy(&[(26144, -1024.), (26146, 1024.)])
            .unwrap();
        host.rebase_readout_policy(&base, &high).unwrap();
        let t = high
            .calibrated_readout_copy(&[(26144, f32::from_bits(0x80000000)), (26146, 0.)])
            .unwrap();
        host.rebase_readout_policy(&high, &t).unwrap();
        host.step(&t, 1.).unwrap();
        let before = host.checkpoint(&t).unwrap();
        let original = serialized(&host, &t);
        let trainer = serde_json::to_vec(&t.checkpoint()).unwrap();
        let mut fork = host.fork_without_age_death_for_research(&t).unwrap();
        let after = fork.checkpoint(&t).unwrap();
        let mut normalized = after.clone();
        normalized.world.world.disable_age_death = false;
        assert_eq!(serde_json::to_vec(&normalized).unwrap(), original);
        assert_eq!(before.schema_version, after.schema_version);
        assert!(fork.world().age_death_disabled());
        let mut cold = CpuCanonicalHost::from_checkpoint(after, &t).unwrap();
        assert_eq!(serialized(&cold, &t), serialized(&fork, &t));
        let a = fork.step(&t, 1.).unwrap();
        let b = cold.step(&t, 1.).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(serialized(&cold, &t), serialized(&fork, &t));
        assert_eq!(serialized(&host, &t), original);
        assert_eq!(serde_json::to_vec(&t.checkpoint()).unwrap(), trainer);
        let mut old_cold = CpuCanonicalHost::from_checkpoint(before, &t).unwrap();
        let mut old_warm = host.snapshot();
        assert_eq!(
            serde_json::to_vec(&old_warm.step(&t, 1.).unwrap().decisions).unwrap(),
            serde_json::to_vec(&old_cold.step(&t, 1.).unwrap().decisions).unwrap()
        );
        assert_eq!(serialized(&old_warm, &t), serialized(&old_cold, &t));
        assert!(fork.fork_without_age_death_for_research(&t).is_err());
        assert!(host.fork_without_age_death_for_research(&high).is_err());
        assert_eq!(serialized(&host, &t), original);
    }
}
#[test]
fn age_policy_fork_preserves_sleep_authority_and_consolidation() {
    let (mut h, t) = fixture(SensorProfile::GroundedObjectSlotsV1);
    let own = h
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    // Synthetic compatibility setup only, not an actual research intervention.
    h.request_recovery_sleep(own).unwrap();
    let before = serialized(&h, &t);
    let mut warm = h.fork_without_age_death_for_research(&t).unwrap();
    let mut cold = CpuCanonicalHost::from_checkpoint(warm.checkpoint(&t).unwrap(), &t).unwrap();
    let mut sleeps = 0;
    let mut consolidations = 0;
    for _ in 0..256 {
        let a = warm.step(&t, 1.).unwrap();
        let b = cold.step(&t, 1.).unwrap();
        sleeps += a.sleeping_organisms;
        consolidations += a.sleep_consolidations;
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
    }
    assert!(sleeps > 0 && consolidations > 0);
    assert_eq!(serialized(&warm, &t), serialized(&cold, &t));
    assert_eq!(serialized(&h, &t), before);
}
