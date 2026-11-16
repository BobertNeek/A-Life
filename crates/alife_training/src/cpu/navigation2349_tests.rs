//! Manual readout calibration preserves acquired state; synthetic tests, not efficacy.
use super::*;
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;
fn fixture(profile: SensorProfile) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    fixture_kind(profile, false)
}
fn fixture_kind(profile: SensorProfile, hazard: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let grip = true;
    let asset = FoundationWeightAsset::decode_canonical(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation"))).unwrap();
    let mut genome: CreatureGenome = serde_json::from_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../docs/experiments/astra-generalization-2026-11-13/pursuit-twohead-contact20.alife-foundation.genome.json"))).unwrap();
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
    let mask = alife_training::StageTrainableMask::recurrent_only(&p).unwrap();
    let trainer =
        CpuFoundationTrainer::new(p, asset, mask, alife_training::AdamWConfig::default()).unwrap();
    (
        CpuCanonicalHost::new_cohort(game, &trainer, 2306).unwrap(),
        trainer,
    )
}
fn cpbytes(h: &CpuCanonicalHost, t: &CpuFoundationTrainer) -> Vec<u8> {
    serde_json::to_vec(&h.checkpoint(t).unwrap()).unwrap()
}
fn interface() -> (CpuCanonicalHost, CpuFoundationTrainer) {
    let (mut h, t) = fixture(SensorProfile::GroundedObjectSlotsV1);
    let mut edits = vec![(26144u32, f32::from_bits(0x80000000)), (26146, 0.)];
    edits.extend((30048..30088).map(|i| (i, 0.4)));
    // Hermetic fresh synthetic policy: g0 distance pair and Inspect memory valence only.
    let calibrated = t.calibrated_readout_copy(&edits).unwrap();
    h.rebase_readout_policy(&t, &calibrated).unwrap();
    let t = calibrated;

    h.enable_joint_ingest_direct2295(&t, true).unwrap();
    h.enable_candidate_opportunity_coverage2313(&t).unwrap();
    let h = h
        .fork_with_approach_attention_feedback_for_research(&t)
        .unwrap()
        .fork_with_active_approach_opportunity_for_research(&t)
        .unwrap();
    (h, t)
}

#[test]
fn navigation_attach_cold_replay_and_private_preservation() {
    let (h, t) = interface();
    let mut old = h.fork_with_maze_program_for_research2346(&t).unwrap();
    old.step(&t, 1.).unwrap();
    let original = cpbytes(&old, &t);
    let adam = serde_json::to_vec(&t.checkpoint()).unwrap();
    let mut nav = old
        .fork_with_experienced_navigation_for_research2349(&t)
        .unwrap();
    assert_eq!(cpbytes(&old, &t), original);
    let cp = nav.checkpoint(&t).unwrap();
    assert_eq!(cp.schema_version, 20);
    assert_eq!(cp.experienced_navigation2349, Some(1));
    assert!(cp.residents[0]
        .navigation2349
        .as_ref()
        .unwrap()
        .previous_pose
        .is_none());
    let mut cold = CpuCanonicalHost::from_checkpoint(cp, &t).unwrap();
    for _ in 0..4 {
        let a = nav.step(&t, 1.).unwrap();
        let b = cold.step(&t, 1.).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(cpbytes(&nav, &t), cpbytes(&cold, &t));
        for row in a.decisions {
            let proof = row.navigation2349.as_ref().unwrap();
            proof.validate().unwrap();
            let q = alife_training::TrainingSequence {
                phenotype_hash: t.phenotype().phenotype_hash(),
                initial: row.initial.clone(),
                ticks: vec![row.replay_tick.clone()],
                burn_in_ticks: 0,
                memory_candidate_gain: 1.,
            };
            let eval = t.evaluate_replay(&q).unwrap();
            let sampled = sample_cpu_joint_action(
                t.phenotype(),
                row.patch.pre_action().perception(),
                &eval.candidate_logits[0],
                nav.checkpoint(&t).unwrap().sampling_seed,
                row.sampling_counter.unwrap(),
                1.,
            )
            .unwrap();
            assert_eq!(sampled, row.action);
        }
    }
    assert_eq!(serde_json::to_vec(&t.checkpoint()).unwrap(), adam);
    assert!(nav
        .fork_with_experienced_navigation_for_research2349(&t)
        .is_err());
}
#[test]
fn disabled_cp19_cold_remains_exact() {
    let (h, t) = interface();
    let mut a = h.fork_with_maze_program_for_research2346(&t).unwrap();
    let mut b = CpuCanonicalHost::from_checkpoint(a.checkpoint(&t).unwrap(), &t).unwrap();
    for _ in 0..8 {
        assert_eq!(
            serde_json::to_vec(&a.step(&t, 1.).unwrap().decisions).unwrap(),
            serde_json::to_vec(&b.step(&t, 1.).unwrap().decisions).unwrap()
        );
        assert_eq!(cpbytes(&a, &t), cpbytes(&b, &t));
    }
}

#[test]
fn navigation_row_binding_and_bad_checkpoint_are_rejected() {
    let (h, t) = interface();
    let h = h.fork_with_maze_program_for_research2346(&t).unwrap();
    let mut nav = h
        .fork_with_experienced_navigation_for_research2349(&t)
        .unwrap();
    let step = nav.step(&t, 1.).unwrap();
    let row = &step.decisions[0];
    let bytes = serde_json::to_vec(row).unwrap();
    let restored: CpuCollectedDecision = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let r = if v.get("memory_plan2_record_v1").is_some() {
        &mut v["memory_plan2_record_v1"]["row"]
    } else {
        &mut v
    };
    r["navigation2349"]["output_bits"][0] = serde_json::json!(0);
    assert!(serde_json::from_value::<CpuCollectedDecision>(v).is_err());
    let cp = nav.checkpoint(&t).unwrap();
    let mut v = serde_json::to_value(&cp).unwrap();
    v["residents"][0]["navigation2349"]["map"]["sequence"] = serde_json::json!(999999999);
    let bad: CpuCanonicalCheckpoint = serde_json::from_value(v).unwrap();
    assert!(CpuCanonicalHost::from_checkpoint(bad, &t).is_err());
    let mut bad = cp;
    bad.experienced_navigation2349 = None;
    assert!(CpuCanonicalHost::from_checkpoint(bad, &t).is_err());
}
#[test]
fn passive_self_transport_does_not_create_traversed_link() {
    let (h, t) = interface();
    let h = h.fork_with_maze_program_for_research2346(&t).unwrap();
    let mut nav = h
        .fork_with_experienced_navigation_for_research2349(&t)
        .unwrap();
    nav.step(&t, 1.).unwrap();
    let owner = nav
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let body = nav
        .world()
        .object_snapshots()
        .into_iter()
        .find(|o| o.organism_id == Some(owner))
        .unwrap();
    let before = nav.checkpoint(&t).unwrap().residents[0]
        .navigation2349
        .as_ref()
        .unwrap()
        .map
        .edges()
        .len();
    nav.world_mut()
        .editor_move_object(
            body.id,
            Vec3f::new(body.position.x + 10., body.position.y, body.position.z),
        )
        .unwrap();
    nav.step(&t, 1.).unwrap();
    let cp = nav.checkpoint(&t).unwrap();
    assert_eq!(
        cp.residents[0]
            .navigation2349
            .as_ref()
            .unwrap()
            .map
            .edges()
            .len(),
        before
    );
}
#[test]
fn dead_body_never_acquires_graph_or_restarts_control() {
    let (h, t) = interface();
    let mut old = h.fork_with_maze_program_for_research2346(&t).unwrap();
    let id = old
        .world()
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let events = std::collections::BTreeMap::from([(
        id.raw(),
        BodyEventDelta {
            damage: 1.,
            ..BodyEventDelta::zero()
        },
    )]);
    for _ in 0..32 {
        old.world_mut()
            .try_advance_tick_with_body_events(&events)
            .unwrap();
        if !old
            .world()
            .organism_registry()
            .get(id)
            .unwrap()
            .lifecycle()
            .is_alive()
        {
            break;
        }
    }
    assert!(!old
        .world()
        .organism_registry()
        .get(id)
        .unwrap()
        .lifecycle()
        .is_alive());
    assert!(old
        .fork_with_experienced_navigation_for_research2349(&t)
        .is_err());
}

#[test]
fn navigation_cognitive_work_and_charged_debit_match_native_policy() {
    let (h, t) = interface();
    let h = h.fork_with_maze_program_for_research2346(&t).unwrap();
    let mut nav = h
        .fork_with_experienced_navigation_for_research2349(&t)
        .unwrap();
    let step = nav.step(&t, 1.).unwrap();
    let row = &step.decisions[0];
    let proof = row.navigation2349.as_ref().unwrap();
    let frame = row.patch.pre_action().perception();
    let cognitive = row.patch.pre_action().cognitive_context.as_ref().unwrap();
    // Count the declared authenticated locomotion outcome scan via public receipts.
    let mut maze_outcome = 1u64;
    for channel in &row.patch.outcome().joint.as_ref().unwrap().channel_outcomes {
        maze_outcome += 2;
        if channel.channel == MotorChannel::Locomotion {
            maze_outcome += 2;
            if channel
                .execution_evidence
                .is_some_and(|e| matches!(e.primitive.raw(), 101 | 102 | 105))
            {
                maze_outcome += 7;
            }
        }
    }
    let base = cognitive.attention.budget_receipt.work_units
        + 1
        + maze_outcome
        + 1
        + frame.maze_heading_work_units()
        + frame.maze_blocked_sense2346().map_or(0, |s| s.work_units())
        + row
            .active_approach_opportunity2327
            .as_ref()
            .map_or(0, |r| r.intent_work_units + r.selection.work_units)
        + row
            .approach_attention_feedback2325
            .as_ref()
            .map_or(0, |r| r.work_units)
        + frame.grip_proprioception().map_or(0, |r| r.work_units())
        + frame
            .candidate_opportunity_coverage()
            .map_or(0, |r| r.work_units());
    let work = row.patch.cognitive_work().unwrap();
    assert_eq!(
        work.focal_target_ops,
        base + proof.work_units + proof.outcome_work_units
    );
    let record = nav
        .world()
        .organism_registry()
        .get(row.organism_id)
        .unwrap();
    let policy = alife_core::cognitive_work::CognitiveWorkCostPolicy::enabled(1.0e-6).unwrap();
    let p = record.phenotype();
    let expected = policy.energy_debit(work).unwrap()
        * p.body.metabolic_turnover
        * (ContinuousLocus::midpoint_value(0.52, 0.60) - p.chemistry.brain_atp_efficiency).exp2();
    assert_eq!(
        record.cognitive_energy_debit().to_bits(),
        expected.to_bits()
    );
    assert_eq!(record.cognitive_work(), work);
}

#[test]
fn zero_motion_blocking_records_only_native_step_heading() {
    use super::observed_map2349::{MeasuredMotion, ObservedMap};
    let still = MeasuredMotion {
        displacement: [0.; 2],
        yaw_delta: 0.,
        blocked: false,
        zero_step_blocked: false,
        executed_traversal: false,
    };
    let m = ObservedMap::default()
        .observe(ExperienceSequenceId(1), &[], 8., 0., still, None)
        .unwrap()
        .0;
    let mut step = still;
    step.blocked = true;
    step.zero_step_blocked = true;
    step.yaw_delta = std::f32::consts::FRAC_PI_2;
    let s = m
        .observe(ExperienceSequenceId(2), &[], 8., 0., step, None)
        .unwrap()
        .0;
    assert_eq!(s.visited_nodes()[0].exit_blocks[0], 1);
    assert_eq!(s.visited_nodes()[0].exit_blocks[2], 0);
    assert!(s.edges().is_empty());
    let mut other = still;
    other.blocked = true;
    let a = m
        .observe(ExperienceSequenceId(2), &[], 8., 0., other, None)
        .unwrap()
        .0;
    assert_eq!(a.visited_nodes()[0].exit_attempts, [0; 8]);
    assert!(a.visited_nodes()[0].confidence < 1.);
    let mut passive = still;
    passive.displacement = [1., 0.];
    let a = m
        .observe(ExperienceSequenceId(2), &[], 8., 0., passive, None)
        .unwrap()
        .0;
    assert!(a.edges().is_empty());
    assert_eq!(a.visited_nodes().len(), 2);
    let mut actual = passive;
    actual.executed_traversal = true;
    let a = m
        .observe(ExperienceSequenceId(2), &[], 8., 0., actual, None)
        .unwrap()
        .0;
    assert_eq!(a.edges().len(), 1);
}

#[test]
fn acquired_mode3_preserves_map_and_removes_only_redundant_validation() {
    let (h, t) = interface();
    let h = h.fork_with_maze_program_for_research2346(&t).unwrap();
    let mut old = h
        .fork_with_experienced_navigation_for_research2349(&t)
        .unwrap();
    for _ in 0..8 {
        old.step(&t, 1.).unwrap();
    }
    let original = cpbytes(&old, &t);
    let adam = serde_json::to_vec(&t.checkpoint()).unwrap();
    let mut fast = old
        .fork_acquired_graph_with_validated_map_for_research2356(&t)
        .unwrap();
    assert_eq!(cpbytes(&old, &t), original);
    let cp = fast.checkpoint(&t).unwrap();
    assert_eq!(cp.experienced_navigation2349, Some(3));
    let mut cold = CpuCanonicalHost::from_checkpoint(cp.clone(), &t).unwrap();
    assert_eq!(cpbytes(&fast, &t), cpbytes(&cold, &t));
    let a = old.step(&t, 1.).unwrap();
    let b = fast.step(&t, 1.).unwrap();
    let c = cold.step(&t, 1.).unwrap();
    assert_eq!(
        serde_json::to_vec(&b.decisions).unwrap(),
        serde_json::to_vec(&c.decisions).unwrap()
    );
    assert_eq!(cpbytes(&fast, &t), cpbytes(&cold, &t));
    let ar = &a.decisions[0];
    let br = &b.decisions[0];
    let ap = ar.navigation2349.as_ref().unwrap();
    let bp = br.navigation2349.as_ref().unwrap();
    assert_eq!(ap.map_before, bp.map_before);
    assert_eq!(ap.map_observed, bp.map_observed);
    assert_eq!(ap.candidate_digest, bp.candidate_digest);
    assert_eq!(ap.base_bits, bp.base_bits);
    assert_eq!(ap.output_bits, bp.output_bits);
    assert_eq!(ar.action, br.action);
    let scan = ap.route.work.validation_checks
        - ap.route.work.slot_visits
        - ap.route.work.candidate_visits;
    assert!(scan > 2);
    let mut norm = bp.route.clone();
    norm.work.validation_checks += scan - 1;
    assert_eq!(norm, ap.route);
    assert_eq!(ap.work_units - bp.work_units, scan - 2);
    assert_eq!(ap.outcome_work_units, bp.outcome_work_units);
    assert_eq!(
        ar.patch.cognitive_work().unwrap().weighted_total
            - br.patch.cognitive_work().unwrap().weighted_total,
        scan - 2
    );
    let bytes = serde_json::to_vec(br).unwrap();
    let reread: CpuCollectedDecision = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&reread).unwrap());
    for _ in 0..3 {
        let b = fast.step(&t, 1.).unwrap();
        let c = cold.step(&t, 1.).unwrap();
        assert_eq!(
            serde_json::to_vec(&b.decisions).unwrap(),
            serde_json::to_vec(&c.decisions).unwrap()
        );
        assert_eq!(cpbytes(&fast, &t), cpbytes(&cold, &t));
    }
    assert_eq!(adam, serde_json::to_vec(&t.checkpoint()).unwrap());
    assert!(fast
        .fork_acquired_graph_with_validated_map_for_research2356(&t)
        .is_err());
    let mut bad = cp;
    bad.experienced_navigation2349 = Some(1);
    assert!(CpuCanonicalHost::from_checkpoint(bad, &t).is_err());
    println!("PASS acquired preservation; mode3 cold/4nextsteps; exact route/candidate/bias/action; work saving {}=scan{}-2; Adam unchanged; mode mismatch rejected",scan-2,scan);
}
