use alife_core::*;
use alife_training::*;
fn handoff() -> FoundationTrainingHandoffV1 {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x74_00, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let base = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_trained_weights(
        &base,
        base.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::new(1, 1, 1),
    )
    .unwrap();
    let (p, inputs) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())
            .unwrap();
    let count = p.synapses().len();
    let actor = FoundationTrainerCheckpoint {
        schema_version: 2,
        phenotype_hash: p.phenotype_hash(),
        source_foundation_digest: asset.digest(),
        optimizer_step: 16_684,
        config: AdamWConfig::default(),
        stage_mask: StageTrainableMask::from_synapse_indices(
            &p,
            &(0..count as u32).collect::<Vec<_>>(),
        )
        .unwrap(),
        weights: p.synapses().iter().map(|s| s.genetic_weight()).collect(),
        first_moment: vec![0.01; count],
        second_moment: vec![0.02; count],
        update_ages: vec![16_684; count],
    };
    let value = PpoValueHeadCheckpoint {
        feature_count: p.neuron_count(),
        optimizer_step: 19_800,
        parameters: vec![0.; (p.neuron_count() as usize + 1) * 3],
        last_updated_policy_version: Some(7400),
    };
    FoundationTrainingHandoffV1::new(
        FoundationTrainingBackendV1::NativeCpu,
        7400,
        inputs,
        actor,
        value,
    )
    .unwrap()
}
#[test]
fn device_free_native_handoff_preserves_full_optimizer_and_asset_identity() {
    let source = handoff();
    let bytes = serde_json::to_vec(&source).unwrap();
    let restored: FoundationTrainingHandoffV1 = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(source, restored);
    let admitted = restored.admit().unwrap();
    assert_eq!(admitted.actor, source.actor);
    assert_eq!(admitted.value, source.value);
    assert_eq!(admitted.actor.optimizer_step, 16_684);
    assert_eq!(admitted.value.optimizer_step, 19_800);
    admitted
        .foundation
        .validate_against(&admitted.phenotype)
        .unwrap();
    assert_eq!(admitted.foundation.weights(), admitted.actor.weights);
    let mut wrong = source.clone();
    wrong.actor.update_ages[0] = 16_685;
    assert!(wrong.validate().is_err());
    let mut wrong = source.clone();
    wrong.value.last_updated_policy_version = Some(7399);
    assert!(wrong.validate().is_err());
    let mut wrong = source;
    wrong.actor.weights[0] += 0.01;
    assert!(wrong.validate().is_err());
}

#[cfg(feature = "receipt-lineage")]
#[test]
fn current_checkpoint_exact_full_state_and_seed_union_are_self_contained() {
    let mut handoff = handoff();
    handoff.actor.first_moment[1] = -0.0;
    handoff.actor.second_moment[2] = f32::from_bits(1);
    handoff.value.parameters[0] = -0.0;
    let source = CurrentTrainingCheckpointV1 {
        schema_version: 1,
        seed: 7400,
        founder_seed_base: 7400,
        seeds: CheckpointSeedsV1 {
            historical_freshness: "unknown".into(),
            excluded_ranges: vec![[1, 7400], [9000, 9000]],
            pending_range: None,
        },
        handoff,
        life: None,
    };
    let restored = CurrentTrainingCheckpointV1::decode(&source.encode().unwrap()).unwrap();
    assert_eq!(
        restored.handoff.actor.first_moment[1].to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(restored.handoff.actor.second_moment[2].to_bits(), 1);
    assert_eq!(
        restored.handoff.value.parameters[0].to_bits(),
        (-0.0_f32).to_bits()
    );
    assert_eq!(
        serde_json::to_vec(&source).unwrap(),
        serde_json::to_vec(&restored).unwrap()
    );
    let mut next = restored;
    next.seeds.reserve(7401, 7500).unwrap();
    assert!(next.seeds.reserve(7501, 8999).is_err());
    for seed in 7401..=7500 {
        next.seeds.consume(seed).unwrap();
    }
    next.seeds.reserve(7501, 8999).unwrap();
    assert_eq!(next.seeds.excluded_ranges, [[1, 9000]]);
    assert_eq!(next.seeds.historical_freshness, "unknown");
    assert!(next.seeds.reserve(9000, 9001).is_err());
    assert!(next.seeds.reserve(0, 1).is_err());
    assert!(next.seeds.reserve(9100, 9099).is_err());
    assert_eq!(next.handoff, source.handoff);
    next.seeds.excluded_ranges = vec![[1, 3], [4, 7400]];
    assert!(next.validate().is_err());
}
#[cfg(feature = "cpu-backend")]
#[test]
fn cpu_updated_founder_exports_to_native_handoff_without_age_or_moment_reset() {
    use alife_training::cpu::*;
    let source = handoff();
    let original = source.clone();
    let components = source.admit().unwrap();
    let mut trainer = CpuFoundationTrainer::from_checkpoint(
        components.phenotype,
        components.foundation,
        components.actor,
    )
    .unwrap();
    let mut value = CpuValueHead::from_checkpoint(components.value).unwrap();
    trainer
        .apply_gradients(&vec![0.01; trainer.weights().len()])
        .unwrap();
    let n = trainer.phenotype().neuron_count() as usize;
    value
        .residual_step(
            &[vec![0.1; n], vec![0.2; n]],
            &[CriticResidualRow {
                current: 0,
                next: 1,
                reward: 0.2,
                discount: 0.9,
            }],
            AdamWConfig::default(),
            0.5,
            7401,
        )
        .unwrap();
    let asset = trainer
        .export_candidate(TrainingStageManifest::new(1, 1, 1))
        .unwrap();
    let (p, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        original.compiler_inputs.genome().clone(),
        original.compiler_inputs.development().clone(),
        asset.clone(),
    )
    .unwrap();
    trainer.rebind_for_next_cohort(p, asset).unwrap();
    let exported = FoundationTrainingHandoffV1::new(
        FoundationTrainingBackendV1::NativeCpu,
        7401,
        inputs,
        trainer.checkpoint(),
        value.checkpoint(),
    )
    .unwrap();
    let admitted = serde_json::from_slice::<FoundationTrainingHandoffV1>(
        &serde_json::to_vec(&exported).unwrap(),
    )
    .unwrap()
    .admit()
    .unwrap();
    assert_eq!(admitted.actor.optimizer_step, 16_685);
    assert!(admitted.actor.update_ages.iter().all(|a| *a == 16_685));
    assert_eq!(admitted.value.optimizer_step, 19_801);
    assert_eq!(
        admitted.actor.first_moment,
        trainer.checkpoint().first_moment
    );
    assert_eq!(admitted.value.parameters, value.checkpoint().parameters);
    assert_ne!(admitted.actor.weights, original.actor.weights);
    admitted
        .foundation
        .validate_against(&admitted.phenotype)
        .unwrap();
}
#[cfg(feature = "gpu-tests")]
#[test]
fn typed_cpu_handoff_restores_real_gpu_training_session_without_reset() {
    let source = handoff();
    let backend = alife_gpu_backend::GpuClosedLoopBackend::new_required(
        alife_gpu_backend::GpuRuntimeProfile::production_v1(),
    )
    .unwrap();
    let session = alife_runtime::GpuAuthoritativeSession::new(
        backend,
        alife_runtime::GpuSessionConsumerKind::Training,
    );
    let (mut trainer, mut value) = source.clone().into_gpu_training_session(session).unwrap();
    let p = trainer.phenotype();
    let n = p.neuron_count() as usize;
    let sequence = TrainingSequence {
        phenotype_hash: p.phenotype_hash(),
        initial: TrainingInitialState {
            activations: vec![0.; n],
            activity_ema: vec![0.; n],
            metabolic_load: vec![0.; n],
            dendrites: Default::default(),
        },
        ticks: vec![TrainingReplayTick {
            encoded_inputs: vec![0.; n],
            projection_gain: 1.,
            local_threshold_shift: 0.,
            microstep_count: u32::from(p.microstep_count()),
            enabled_routes: vec![true; p.projections().len()],
            effective_weight_offsets: vec![0.; p.synapses().len()],
            structural_synapses: vec![],
            candidates: vec![TrainingReplayCandidate {
                family: CandidateActionFamily::Other,
                decoder_inputs: [0.; 54],
                innate_bias: 0.,
            }],
        }],
        burn_in_ticks: 0,
        memory_candidate_gain: p
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    };
    trainer.prepare_replay(&sequence).unwrap();
    value.prepare_for_replay(&trainer).unwrap();
    assert_eq!(trainer.checkpoint().unwrap(), source.actor);
    assert_eq!(value.checkpoint(trainer.session()).unwrap(), source.value);
}
