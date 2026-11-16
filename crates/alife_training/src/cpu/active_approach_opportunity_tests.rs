//! Manual readout calibration preserves acquired state; synthetic tests, not efficacy.
use super::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;
fn fixture(profile: SensorProfile) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    fixture_kind(profile, false)
}
fn fixture_kind(profile: SensorProfile, hazard: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
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
    let builder = HeadlessScenarioBuilder::new(2306).agent("subject", owner, Vec3f::ZERO);
    let builder = if hazard {
        builder.hazard("near", Vec3f::new(0.25, 0., 0.), 0.05)
    } else {
        builder.food("near", Vec3f::new(2.5, 0., 0.), 0.4)
    };
    let mut world = builder.build().unwrap();
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
fn immutable_attach_cold_intent_lifecycle_and_real_cost() {
    for apply in [true, false] {
        let (mut base, t) = fixture(SensorProfile::GroundedObjectSlotsV1);
        base.enable_joint_ingest_direct2295(&t, apply).unwrap();
        base.enable_candidate_opportunity_coverage2313(&t).unwrap();
        assert!(base
            .fork_with_active_approach_opportunity_for_research(&t)
            .is_err());
        let mut old = base
            .fork_with_approach_attention_feedback_for_research(&t)
            .unwrap();
        let before = old.checkpoint(&t).unwrap();
        let original = serialized(&old, &t);
        let adam = serde_json::to_vec(&t.checkpoint()).unwrap();
        let mut new = old
            .fork_with_active_approach_opportunity_for_research(&t)
            .unwrap();
        let cp = new.checkpoint(&t).unwrap();
        assert_eq!(cp.schema_version, 18);
        assert_eq!(cp.candidate_opportunity_coverage2313, Some(3));
        assert!(cp.residents[0].active_approach_intent2327.is_none());
        let mut n = cp.clone();
        n.schema_version = 17;
        n.candidate_opportunity_coverage2313 = Some(2);
        assert_eq!(serde_json::to_vec(&n).unwrap(), original);
        assert!(new
            .fork_with_active_approach_opportunity_for_research(&t)
            .is_err());
        assert_eq!(serialized(&old, &t), original);
        let mut cold = CpuCanonicalHost::from_checkpoint(cp, &t).unwrap();
        let mut intents = 0;
        for i in 0..32 {
            let a = new.step(&t, 1.).unwrap();
            let b = cold.step(&t, 1.).unwrap();
            assert_eq!(
                serde_json::to_vec(&a.decisions).unwrap(),
                serde_json::to_vec(&b.decisions).unwrap()
            );
            assert_eq!(serialized(&new, &t), serialized(&cold, &t));
            if i == 0 {
                let first = old.step(&t, 1.).unwrap();
                assert_eq!(
                    serde_json::to_vec(&first.decisions[0].action).unwrap(),
                    serde_json::to_vec(&a.decisions[0].action).unwrap()
                );
                assert_eq!(
                    first.decisions[0].patch.pre_action().cognitive_context,
                    a.decisions[0].patch.pre_action().cognitive_context
                );
            }
            for row in &a.decisions {
                let extra = row.active_approach_opportunity2327.as_ref().unwrap();
                let feedback = row.approach_attention_feedback2325.as_ref().unwrap();
                let frame = row.patch.pre_action().perception();
                assert_eq!(extra.frame_digest, frame.frame_digest());
                assert_eq!(extra.after.is_some(), feedback.nominated);
                intents += usize::from(extra.after.is_some());
                if let Some(intent) = &extra.after {
                    assert_eq!(intent.source_frame_digest, feedback.frame_digest);
                    assert_eq!(intent.executed.primitive.raw(), 101);
                    assert!(intent.executed.succeeded);
                }
                let cp = new.checkpoint(&t).unwrap();
                assert_eq!(cp.residents[0].active_approach_intent2327, extra.after);
                let work = row.patch.cognitive_work().unwrap();
                let pre = row.patch.pre_action().cognitive_context.as_ref().unwrap();
                assert_eq!(
                    work.focal_target_ops,
                    pre.attention.budget_receipt.work_units
                        + frame.grip_proprioception().map_or(0, |r| r.work_units())
                        + frame
                            .candidate_opportunity_coverage()
                            .map_or(0, |r| r.work_units())
                        + feedback.work_units
                        + extra.selection.work_units
                        + extra.intent_work_units
                );
                let record = new
                    .world()
                    .organism_registry()
                    .get(row.organism_id)
                    .unwrap();
                assert_eq!(record.cognitive_work(), work);
                let (policy, debit, _, _) = row.cognitive_cost_capture.unwrap();
                let p = record.phenotype();
                let expected = policy.energy_debit(work).unwrap()
                    * p.body.metabolic_turnover
                    * (ContinuousLocus::midpoint_value(0.52, 0.60)
                        - p.chemistry.brain_atp_efficiency)
                        .exp2();
                let (mut oldbody, newbody) = row.cognitive_body_cost_capture.unwrap();
                assert_eq!(oldbody.debit_energy_pro_rata(expected).unwrap(), debit);
                assert_eq!(
                    serde_json::to_vec(&oldbody).unwrap(),
                    serde_json::to_vec(&newbody).unwrap()
                );
                let text = serde_json::to_vec(row).unwrap();
                assert_eq!(
                    text,
                    serde_json::to_vec(
                        &serde_json::from_slice::<CpuCollectedDecision>(&text).unwrap()
                    )
                    .unwrap()
                );
            }
        }
        assert!(intents > 0);
        assert_eq!(adam, serde_json::to_vec(&t.checkpoint()).unwrap());
        for (schema, version) in [(17, Some(3)), (18, Some(2)), (18, None), (18, Some(4))] {
            let mut bad = before.clone();
            bad.schema_version = schema;
            bad.candidate_opportunity_coverage2313 = version;
            assert!(CpuCanonicalHost::from_checkpoint(bad, &t).is_err());
        }
        let mut rollback = new.clone();
        rollback.fail_joint_membership_after_admission = true;
        let cp = serialized(&rollback, &t);
        assert!(rollback.step(&t, 1.).is_err());
        assert_eq!(serialized(&rollback, &t), cp);
    }
}
#[test]
fn active_mode_sleep_authority_and_cold_parity() {
    let (mut base, t) = fixture(SensorProfile::GroundedObjectSlotsV1);
    base.enable_joint_ingest_direct2295(&t, true).unwrap();
    base.enable_candidate_opportunity_coverage2313(&t).unwrap();
    let old = base
        .fork_with_approach_attention_feedback_for_research(&t)
        .unwrap();
    let mut h = old
        .fork_with_active_approach_opportunity_for_research(&t)
        .unwrap();
    let id = h
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    h.request_recovery_sleep(id).unwrap();
    let mut cold = CpuCanonicalHost::from_checkpoint(h.checkpoint(&t).unwrap(), &t).unwrap();
    let (mut sleeps, mut consolidations) = (0, 0);
    for _ in 0..256 {
        let a = h.step(&t, 1.).unwrap();
        let b = cold.step(&t, 1.).unwrap();
        sleeps += a.sleeping_organisms;
        consolidations += a.sleep_consolidations;
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
    }
    assert!(sleeps > 0 && consolidations > 0);
    assert_eq!(serialized(&h, &t), serialized(&cold, &t));
}
