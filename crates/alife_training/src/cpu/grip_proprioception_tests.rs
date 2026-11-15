//! Native sensor/cost/identity qualification only; not a behavioral efficacy assay.
use super::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;

fn fixture(grip: bool) -> (CpuCanonicalHost, CpuFoundationTrainer) {
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
        asset.manifest().sensor_profile(),
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

#[test]
fn grip_native_sensing_work_is_bound_and_charged_exactly_once() {
    for apply in [false, true] {
        let (mut host, trainer) = fixture(true);
        host.enable_joint_ingest_direct2295(&trainer, apply)
            .unwrap();
        let founder = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        for _ in 0..4 {
            for row in host.step(&trainer, 1.0).unwrap().decisions {
                let pre = row.patch.pre_action();
                let frame = pre.perception();
                let grip = frame.grip_proprioception().unwrap();
                assert_eq!(grip.work_units(), grip.inspected_objects() + 3);
                let frame_bytes = serde_json::to_vec(frame).unwrap();
                assert_eq!(
                    serde_json::from_slice::<PerceptionFrame>(&frame_bytes).unwrap(),
                    *frame
                );
                let context = pre.cognitive_context.as_ref().unwrap();
                let charged = row.patch.cognitive_work().unwrap();
                assert_eq!(
                    charged.focal_target_ops,
                    context.attention.budget_receipt.work_units + grip.work_units()
                );
                let record = host.world.organism_registry().get(row.organism_id).unwrap();
                assert_eq!(record.cognitive_work(), charged);
                let (policy, debit, before, after) = row.cognitive_cost_capture.unwrap();
                let p = record.phenotype();
                let expected = policy.energy_debit(charged).unwrap()
                    * p.body.metabolic_turnover
                    * (ContinuousLocus::midpoint_value(0.52, 0.60)
                        - p.chemistry.brain_atp_efficiency)
                        .exp2();
                assert_eq!(debit, expected.min(before));
                assert_eq!(record.cognitive_energy_debit(), debit);
                assert!((before - after - debit).abs() < 2e-6);
            }
        }
        assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), founder);
    }
}

#[test]
fn grip_native_cold_next_step_and_genetic_identity_boundaries() {
    let (legacy, old_trainer) = fixture(false);
    let old_cp = legacy.checkpoint(&old_trainer).unwrap();
    for apply in [false, true] {
        let (mut host, trainer) = fixture(true);
        let adam = serde_json::to_vec(&trainer.checkpoint()).unwrap();
        host.enable_joint_ingest_direct2295(&trainer, apply)
            .unwrap();
        host.step(&trainer, 1.0).unwrap();
        let cp = host.checkpoint(&trainer).unwrap();
        assert_eq!(cp.schema_version, 15);
        assert_eq!(cp.joint_membership2177, Some(if apply { 14 } else { 15 }));
        assert!(CpuCanonicalHost::from_checkpoint(cp.clone(), &old_trainer).is_err());
        assert!(CpuCanonicalHost::from_checkpoint(old_cp.clone(), &trainer).is_err());
        let wire = serde_json::to_vec(&cp).unwrap();
        let decoded = serde_json::from_slice(&wire).unwrap();
        let mut cold = CpuCanonicalHost::from_checkpoint(decoded, &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap(),
            wire
        );
        let next = host.step(&trainer, 1.0).unwrap();
        let cold_next = cold.step(&trainer, 1.0).unwrap();
        assert_eq!(
            serde_json::to_vec(&next.decisions).unwrap(),
            serde_json::to_vec(&cold_next.decisions).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
            serde_json::to_vec(&cold.checkpoint(&trainer).unwrap()).unwrap()
        );
        assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), adam);
    }
}

#[test]
fn grip_tampered_receipt_rejected_and_staged_failure_rolls_back() {
    let (mut host, trainer) = fixture(true);
    host.enable_joint_ingest_direct2295(&trainer, true).unwrap();
    let before = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    let mut failed = host.snapshot();
    failed.fail_ingest_outcome_binding2286 = true;
    assert!(failed.step(&trainer, 1.0).is_err());
    assert_eq!(
        serde_json::to_vec(&failed.checkpoint(&trainer).unwrap()).unwrap(),
        before
    );
    let row = host.step(&trainer, 1.0).unwrap().decisions.remove(0);
    let saved = serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap();
    let frame = serde_json::to_value(row.patch.pre_action().perception()).unwrap();
    for field in ["held", "inspected_objects", "touch_gain"] {
        let mut corrupt = frame.clone();
        let r = &mut corrupt["base"]["grip_proprioception"];
        assert!(r.is_object());
        r[field] = match field {
            "held" => (!r[field].as_bool().unwrap()).into(),
            "inspected_objects" => (r[field].as_u64().unwrap() + 1).into(),
            _ => (-1.0).into(),
        };
        assert!(serde_json::from_value::<PerceptionFrame>(corrupt).is_err());
    }
    // Decoder rejection above never mutates the resident host.
    assert_eq!(
        serde_json::to_vec(&host.checkpoint(&trainer).unwrap()).unwrap(),
        saved
    );
}

#[test]
fn grip_public_grab_reaches_unique_anchor_through_normal_encoder() {
    let (mut free, trainer) = fixture(true);
    free.enable_joint_ingest_direct2295(&trainer, true).unwrap();
    let mut held = free.snapshot();
    let id = held
        .world
        .organism_registry()
        .iter()
        .next()
        .unwrap()
        .organism_id();
    let target = held.world.entity_id("near").unwrap();
    let grab = ActionCommand::structured(
        id,
        HeadlessActionIds::GRAB,
        ActionKind::Interact,
        ActionTarget::new(Some(target), None),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(1.0).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap();
    assert!(held.world.apply_command(&grab).unwrap().execution.succeeded);
    assert_eq!(held.world.entity(target).unwrap().carried_by, Some(id));
    held.seal_all_residents().unwrap();
    let assignments = trainer
        .phenotype()
        .sensor_encoder()
        .assignments()
        .iter()
        .filter(|a| a.target_neuron() == 20)
        .collect::<Vec<_>>();
    assert_eq!(
        assignments.len(),
        1,
        "this witness requires an unaliased grip anchor"
    );
    let a = assignments[0];
    assert_eq!(
        a.source_group(),
        alife_core::phenotype::SensorEncoderSourceGroup::SensoryChannel
    );
    assert_eq!(a.source_index(), 34);
    let mut values = Vec::new();
    for (expected_held, host) in [(false, &mut free), (true, &mut held)] {
        let record = host.world.organism_registry().get(id).unwrap();
        let receptors = record
            .biochemistry()
            .neural_receptor_frame(record.phenotype())
            .unwrap();
        let effects = NeuralReceptorEffects::from_frame(
            &receptors,
            &NeuralReceptorPhenotype::compile(trainer.phenotype()).unwrap(),
        )
        .unwrap();
        let row = host.step(&trainer, 1.0).unwrap().decisions.remove(0);
        let frame = row.patch.pre_action().perception();
        assert_eq!(frame.grip_proprioception().unwrap().held(), expected_held);
        let input = frame.sensory().channels.tactile_contact[2];
        let (lo, hi) = a.clamp_range();
        let assigned = (input * effects.attention_gain * a.scale() + a.bias()).clamp(lo, hi);
        let gate =
            (effects.structural_growth_gate + effects.sleep_gate + effects.consolidation_gate)
                / 3.0;
        let expected =
            (assigned * effects.regional_excitability + (gate - 0.5) * 0.05).clamp(-1.0, 1.0);
        assert_eq!(
            row.replay_tick.encoded_inputs[20].to_bits(),
            expected.to_bits()
        );
        values.push(expected);
    }
    assert!(
        values[1] > values[0],
        "measured grip must change the real encoder input"
    );
}
