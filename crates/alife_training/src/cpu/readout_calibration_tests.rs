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

fn decoder(trainer: &CpuFoundationTrainer) -> usize {
    trainer.phenotype().synapses().iter().position(|s|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if c.head()==DecoderHeadKind::ActionCandidate)).unwrap()
}
fn changed(trainer: &CpuFoundationTrainer) -> CpuFoundationTrainer {
    let i = decoder(trainer);
    trainer
        .calibrated_readout_copy(&[(i as u32, trainer.weights()[i] + 0.125)])
        .unwrap()
}
#[test]
fn calibration_changes_only_requested_bits_and_preserves_adam_and_execution() {
    let (_, mut trainer) = fixture(SensorProfile::GroundedObjectSlotsV1);
    trainer.set_replay_execution(CpuReplayExecution::CadenceCsr);
    let mut cp = trainer.checkpoint();
    cp.optimizer_step = 7;
    cp.update_ages[0] = 3;
    cp.first_moment[0] = 0.25;
    cp.second_moment[0] = 0.5;
    trainer.restore_checkpoint(cp).unwrap();
    let index = decoder(&trainer);
    let before = trainer.checkpoint();
    let value = if before.weights[index].to_bits() == 0x80000000 {
        0.0
    } else {
        f32::from_bits(0x80000000)
    };
    let next = trainer
        .calibrated_readout_copy(&[(index as u32, value)])
        .unwrap();
    assert_eq!(next.weights()[index].to_bits(), value.to_bits());
    let mut normalized = next.checkpoint();
    normalized.weights[index] = before.weights[index];
    assert_eq!(
        serde_json::to_vec(&normalized).unwrap(),
        serde_json::to_vec(&before).unwrap()
    );
    assert_eq!(trainer.replay_execution(), next.replay_execution());
    assert_eq!(
        trainer.source_foundation.encode_canonical().unwrap(),
        next.source_foundation.encode_canonical().unwrap()
    );
    assert_eq!(
        serde_json::to_vec(trainer.phenotype()).unwrap(),
        serde_json::to_vec(next.phenotype()).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&trainer.checkpoint()).unwrap(),
        serde_json::to_vec(&before).unwrap()
    );
}
#[test]
fn calibration_rejects_invalid_edits_without_mutation() {
    let (_, trainer) = fixture(SensorProfile::GroundedObjectSlotsV1);
    let i = decoder(&trainer) as u32;
    let recurrent = trainer
        .phenotype()
        .synapses()
        .iter()
        .position(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .unwrap() as u32;
    let before = serde_json::to_vec(&trainer.checkpoint()).unwrap();
    for edits in [
        vec![],
        vec![(i, trainer.weights()[i as usize])],
        vec![(i, 1.), (i, 2.)],
        vec![(u32::MAX, 1.)],
        vec![(recurrent, 1.)],
        vec![(i, f32::NAN)],
        vec![(i, f32::INFINITY)],
    ] {
        assert!(trainer.calibrated_readout_copy(&edits).is_err());
        assert_eq!(serde_json::to_vec(&trainer.checkpoint()).unwrap(), before);
    }
    for (i, s) in trainer.phenotype().synapses().iter().enumerate() {
        if matches!(s.kind(),CompiledSynapseKind::Decoder(c) if !matches!(c.head(),DecoderHeadKind::ActionCandidate|DecoderHeadKind::MemoryContext|DecoderHeadKind::CognitiveContext))
        {
            assert!(trainer.calibrated_readout_copy(&[(i as u32, 1.)]).is_err());
            break;
        }
    }
}
#[test]
fn rebase_retains_exact_state_and_cold_nextstep_under_new_policy() {
    for adapter in [false, true] {
        let (mut host, trainer) = fixture(SensorProfile::GroundedObjectSlotsV1);
        host.enable_joint_ingest_direct2295(&trainer, true).unwrap();
        if adapter {
            host.enable_candidate_opportunity_coverage2313(&trainer)
                .unwrap();
        }
        host.step(&trainer, 1.).unwrap();
        let before = host.checkpoint(&trainer).unwrap();
        let before_bytes = serialized(&host, &trainer);
        let next = changed(&trainer);
        assert!(host.checkpoint(&next).is_err());
        assert!(CpuCanonicalHost::from_checkpoint(before.clone(), &next).is_err());
        host.rebase_readout_policy(&trainer, &next).unwrap();
        assert!(host.checkpoint(&trainer).is_err());
        let after = host.checkpoint(&next).unwrap();
        assert_ne!(before.founder_weight_digest, after.founder_weight_digest);
        let mut normalized = after.clone();
        normalized.founder_weight_digest = before.founder_weight_digest;
        assert_eq!(serde_json::to_vec(&normalized).unwrap(), before_bytes);
        assert!(CpuCanonicalHost::from_checkpoint(after.clone(), &trainer).is_err());
        let mut cold = CpuCanonicalHost::from_checkpoint(after.clone(), &next).unwrap();
        assert_eq!(serialized(&cold, &next), serialized(&host, &next));
        let a = host.step(&next, 1.).unwrap();
        let b = cold.step(&next, 1.).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.decisions).unwrap(),
            serde_json::to_vec(&b.decisions).unwrap()
        );
        assert_eq!(serialized(&cold, &next), serialized(&host, &next));
        // Prior branch remains independently resumable under original unchanged policy.
        let mut old_a = CpuCanonicalHost::from_checkpoint(before.clone(), &trainer).unwrap();
        let mut old_b = CpuCanonicalHost::from_checkpoint(before, &trainer).unwrap();
        assert_eq!(
            serde_json::to_vec(&old_a.step(&trainer, 1.).unwrap().decisions).unwrap(),
            serde_json::to_vec(&old_b.step(&trainer, 1.).unwrap().decisions).unwrap()
        );
        assert_eq!(serialized(&old_a, &trainer), serialized(&old_b, &trainer));
    }
}
#[test]
fn rebase_rejects_wrong_policy_optimizer_or_recurrent_edits_atomically() {
    let (mut host, trainer) = fixture(SensorProfile::GroundedObjectSlotsV1);
    let next = changed(&trainer);
    let original = serialized(&host, &trainer);
    assert!(host.rebase_readout_policy(&next, &trainer).is_err());
    assert!(host.rebase_readout_policy(&trainer, &trainer).is_err());
    let recurrent = trainer
        .phenotype()
        .synapses()
        .iter()
        .position(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .unwrap();
    for kind in 0..3 {
        let mut bad = changed(&trainer);
        let mut cp = bad.checkpoint();
        match kind {
            0 => cp.weights[recurrent] += 0.125,
            1 => cp.optimizer_step += 1,
            _ => {
                bad.set_replay_execution(CpuReplayExecution::CadenceCsr);
            }
        }
        bad.restore_checkpoint(cp).unwrap();
        assert!(host.rebase_readout_policy(&trainer, &bad).is_err());
        assert_eq!(serialized(&host, &trainer), original);
    }
    // Canonical policy digest normalizes +/-0: a sign-only edit cannot create a new identity.
    let zero = trainer
        .phenotype()
        .synapses()
        .iter()
        .enumerate()
        .find(|(i, _)| {
            super::readout_calibration_coordinate(trainer.phenotype(), *i)
                && trainer.weights()[*i] == 0.
        })
        .map(|(i, _)| i)
        .unwrap();
    let signed = trainer
        .calibrated_readout_copy(&[(
            zero as u32,
            f32::from_bits(trainer.weights()[zero].to_bits() ^ 0x80000000),
        )])
        .unwrap();
    assert!(host.rebase_readout_policy(&trainer, &signed).is_err());
    assert_eq!(serialized(&host, &trainer), original);
}

#[test]
fn calibrated_native_g0_forward_and_sample_match_detached_same_history() {
    let (mut original, base) = fixture(SensorProfile::GroundedObjectSlotsV1);
    // This synthetic fixture starts with zero distance weights. Establish the
    // explicit g1024 policy before acquisition, using the public transition API.
    let trainer = base
        .calibrated_readout_copy(&[(26144, -1024.0), (26146, 1024.0)])
        .unwrap();
    original.rebase_readout_policy(&base, &trainer).unwrap();
    original
        .enable_joint_ingest_direct2295(&trainer, true)
        .unwrap();
    original.step(&trainer, 1.).unwrap();
    let next = trainer
        .calibrated_readout_copy(&[(26144, f32::from_bits(0x80000000)), (26146, 0.)])
        .unwrap();
    let before = original.checkpoint(&trainer).unwrap();
    let mut calibrated = original.snapshot();
    calibrated.rebase_readout_policy(&trainer, &next).unwrap();
    let baseline = original.step(&trainer, 1.).unwrap().decisions.remove(0);
    let actual = calibrated.step(&next, 1.).unwrap().decisions.remove(0);
    assert_eq!(baseline.initial, actual.initial);
    assert_eq!(
        baseline.replay_tick, actual.replay_tick,
        "readout-only policy does not change pre-action inputs or acquired offsets"
    );
    let detached = next
        .evaluate_replay(&crate::TrainingSequence {
            phenotype_hash: next.phenotype().phenotype_hash(),
            initial: baseline.initial,
            ticks: vec![baseline.replay_tick],
            burn_in_ticks: 0,
            memory_candidate_gain: next
                .phenotype()
                .candidate_decoder()
                .memory_channel()
                .unwrap()
                .max_candidate_gain(),
        })
        .unwrap();
    assert_eq!(
        detached.final_activations[0]
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>(),
        actual
            .final_features
            .iter()
            .map(|x| x.to_bits())
            .collect::<Vec<_>>()
    );
    let sampled = sample_cpu_joint_action(
        next.phenotype(),
        actual.patch.pre_action().perception(),
        &detached.candidate_logits[0],
        before.sampling_seed,
        actual.sampling_counter.unwrap(),
        actual.action.temperature,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&sampled).unwrap(),
        serde_json::to_vec(&actual.action).unwrap()
    );
}

#[test]
fn direct_organism_rejects_nonreadout_policy_without_private_mutation() {
    let (mut host, trainer) = fixture(SensorProfile::GroundedObjectSlotsV1);
    let before = host.checkpoint(&trainer).unwrap();
    let row = host.step(&trainer, 1.).unwrap().decisions.remove(0);
    let effects = NeuralReceptorEffects {
        source_tick: Tick::new(8),
        source_chemistry_version: 1,
        interoceptive_gain: 1.,
        regional_excitability: 1.,
        projection_gain: 1.,
        local_threshold_shift: 0.,
        attention_gain: 1.,
        plasticity_appetitive: 0.,
        plasticity_aversive: 0.,
        structural_growth_gate: 0.,
        sleep_gate: 0.,
        consolidation_gate: 0.,
    };
    let recurrent = trainer
        .phenotype()
        .synapses()
        .iter()
        .position(|s| s.kind() == CompiledSynapseKind::Recurrent)
        .unwrap();
    let unsupported=trainer.phenotype().synapses().iter().position(|s|matches!(s.kind(),CompiledSynapseKind::Decoder(c) if !matches!(c.head(),DecoderHeadKind::ActionCandidate|DecoderHeadKind::MemoryContext|DecoderHeadKind::CognitiveContext))).expect("fixture has unsupported decoder head");
    for index in [recurrent, unsupported] {
        let mut bad = changed(&trainer);
        let mut cp = bad.checkpoint();
        cp.weights[index] += 0.125;
        bad.restore_checkpoint(cp).unwrap();
        assert!(!bad.live_readout_policy_is_valid());
        let mut state = CpuOrganismState::from_checkpoint(
            trainer.phenotype(),
            before.residents[0].neural.clone(),
        )
        .unwrap();
        let original = serde_json::to_vec(&state.checkpoint().unwrap()).unwrap();
        assert!(state
            .advance(&bad, row.replay_tick.clone(), &effects)
            .is_err());
        assert_eq!(
            serde_json::to_vec(&state.checkpoint().unwrap()).unwrap(),
            original
        );
    }
    assert!(trainer.live_readout_policy_is_valid());
    assert!(changed(&trainer).live_readout_policy_is_valid());
}
