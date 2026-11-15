//! Native adapter identity/work tests; no efficacy claims.
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
fn adapter_attach_changes_only_explicit_checkpoint_identity_on_retained_life() {
    for apply in [false, true] {
        let (mut host, trainer) = fixture(SensorProfile::GroundedObjectSlotsV1);
        host.enable_joint_ingest_direct2295(&trainer, apply)
            .unwrap();
        host.step(&trainer, 1.).unwrap(); // Genuine acquired state before attachment.
        let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        let before = host.checkpoint(&trainer).unwrap();
        assert_eq!(before.schema_version, 15);
        assert_eq!(before.candidate_opportunity_coverage2313, None);
        let old_bytes = serialized(&host, &trainer);
        assert!(!std::str::from_utf8(&old_bytes)
            .unwrap()
            .contains("candidate_opportunity_coverage2313"));
        let cold = CpuCanonicalHost::from_checkpoint(before.clone(), &trainer).unwrap();
        assert_eq!(old_bytes, serialized(&cold, &trainer));
        host.enable_candidate_opportunity_coverage2313(&trainer)
            .unwrap();
        let after = host.checkpoint(&trainer).unwrap();
        assert_eq!(after.schema_version, 16);
        assert_eq!(after.candidate_opportunity_coverage2313, Some(1));
        assert_eq!(after.joint_membership2177, before.joint_membership2177);
        let mut normalized = after.clone();
        normalized.schema_version = 15;
        normalized.candidate_opportunity_coverage2313 = None;
        assert_eq!(
            serde_json::to_vec(&normalized).unwrap(),
            old_bytes,
            "all retained fields including residents,world,RNG,tracking unchanged"
        );
        let saved = serialized(&host, &trainer);
        assert!(host
            .enable_candidate_opportunity_coverage2313(&trainer)
            .is_err());
        assert_eq!(serialized(&host, &trainer), saved);
        assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), founder);
    }
}
#[test]
fn adapter_cold_next_step_and_malformed_identity_are_strict() {
    let (mut host, trainer) = fixture(SensorProfile::GroundedObjectSlotsV1);
    host.enable_joint_ingest_direct2295(&trainer, true).unwrap();
    host.enable_candidate_opportunity_coverage2313(&trainer)
        .unwrap();
    host.step(&trainer, 1.).unwrap();
    let cp = host.checkpoint(&trainer).unwrap();
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    let decoded = serde_json::from_slice(&serde_json::to_vec(&cp).unwrap()).unwrap();
    let mut cold = CpuCanonicalHost::from_checkpoint(decoded, &trainer).unwrap();
    assert_eq!(serialized(&host, &trainer), serialized(&cold, &trainer));
    let a = host.step(&trainer, 1.).unwrap();
    let b = cold.step(&trainer, 1.).unwrap();
    assert_eq!(
        serde_json::to_vec(&a.decisions).unwrap(),
        serde_json::to_vec(&b.decisions).unwrap()
    );
    assert_eq!(serialized(&host, &trainer), serialized(&cold, &trainer));
    for variant in 0..4 {
        let mut bad = cp.clone();
        match variant {
            0 => bad.schema_version = 15,
            1 => bad.candidate_opportunity_coverage2313 = None,
            2 => bad.candidate_opportunity_coverage2313 = Some(2),
            _ => bad.joint_membership2177 = Some(12),
        };
        assert!(CpuCanonicalHost::from_checkpoint(bad, &trainer).is_err());
    }
    assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), founder);
}
#[test]
fn adapter_invalid_mode_and_profile_do_not_mutate_host() {
    for profile in [
        SensorProfile::GroundedObjectSlotsV1,
        SensorProfile::GroundedTerrainVisionV1,
    ] {
        let (mut host, trainer) = fixture(profile);
        assert_eq!(trainer.phenotype().sensor_profile(), profile);
        let before = serialized(&host, &trainer);
        assert!(host
            .enable_candidate_opportunity_coverage2313(&trainer)
            .is_err());
        assert_eq!(serialized(&host, &trainer), before);
    }
    let (mut host, trainer) = fixture(SensorProfile::GroundedObjectSlotsV1);
    host.enable_joint_membership2177(&trainer).unwrap();
    let before = serialized(&host, &trainer);
    assert!(host
        .enable_candidate_opportunity_coverage2313(&trainer)
        .is_err());
    assert_eq!(serialized(&host, &trainer), before);
}
#[test]
fn crowded_adapter_receipt_and_actual_biology_charge_work_once() {
    let (mut host, trainer) = fixture(SensorProfile::GroundedObjectSlotsV1);
    for i in 0..8 {
        host.world_mut()
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: format!("coverage-{i}"),
                kind: WorldObjectKind::Ball,
                organism_id: None,
                position: Vec3f::new(0.4 + i as f32 * 0.25, 0., 0.2),
                nutrition: 0.,
                hazard_pain: 0.,
                radius: 0.3,
                token_id: None,
            })
            .unwrap();
    }
    host.enable_joint_ingest_direct2295(&trainer, true).unwrap();
    host.enable_candidate_opportunity_coverage2313(&trainer)
        .unwrap();
    let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    for _ in 0..4 {
        for row in host.step(&trainer, 1.).unwrap().decisions {
            let pre = row.patch.pre_action();
            let frame = pre.perception();
            let receipt = frame.candidate_opportunity_coverage().unwrap();
            assert!(receipt.observed_slots() > u16::from(receipt.capacity()));
            assert!(receipt.selected_slots().len() <= usize::from(receipt.capacity()));
            assert_eq!(receipt.phase(), pre.sequence_id.raw());
            assert_eq!(
                receipt.observed_slots() as usize,
                frame.grounded_object_slots().len()
            );
            let selected = frame
                .candidates()
                .iter()
                .filter_map(|c| match c.observation {
                    CandidateObservationRef::ObjectSlot(slot) => Some(slot),
                    _ => None,
                })
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(selected, receipt.selected_slots().iter().copied().collect());
            let charged = row.patch.cognitive_work().unwrap();
            assert_eq!(
                charged.focal_target_ops,
                pre.cognitive_context
                    .as_ref()
                    .unwrap()
                    .attention
                    .budget_receipt
                    .work_units
                    + frame.grip_proprioception().map_or(0, |r| r.work_units())
                    + receipt.work_units()
            );
            let record = host
                .world()
                .organism_registry()
                .get(row.organism_id)
                .unwrap();
            assert_eq!(record.cognitive_work(), charged);
            let (policy, debit, before, after) = row.cognitive_cost_capture.unwrap();
            let p = record.phenotype();
            let expected = policy.energy_debit(charged).unwrap()
                * p.body.metabolic_turnover
                * (ContinuousLocus::midpoint_value(0.52, 0.60) - p.chemistry.brain_atp_efficiency)
                    .exp2();
            assert_eq!(debit, expected.min(before));
            assert_eq!(record.cognitive_energy_debit(), debit);
            assert!((before - after - debit).abs() < 2e-6);
        }
    }
    assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), founder);
}
