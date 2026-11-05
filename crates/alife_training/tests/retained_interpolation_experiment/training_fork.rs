use super::*;
use alife_training::cpu::retained_training_fork::*;

#[test]
fn pc_optimizer_transfer_keeps_all_moments_ages_and_acquired_coordinates() {
    for active in [0, 1] {
        let mut inputs = fixture(active);
        inputs.cloud_fraction = 0.25;
        let experiment = prepare_retained_interpolation_experiment(inputs).unwrap();
        let before = serde_json::to_vec(&experiment).unwrap();
        let fork = initialize_pc_donor_retained_fork(&experiment).unwrap();
        let mut expected = fork.donor_actor.clone();
        expected.weights = fork.trainer.weights().to_vec();
        expected.phenotype_hash = fork.trainer.phenotype().phenotype_hash();
        expected.source_foundation_digest = experiment
            .compiler_inputs()
            .n2048_candidate_asset()
            .unwrap()
            .digest();
        assert_eq!(fork.trainer.checkpoint(), expected);
        assert_eq!(
            serde_json::to_vec(&fork.trainer.checkpoint()).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
        assert_eq!(fork.value.checkpoint(), fork.donor_value);
        assert_eq!(
            serde_json::to_vec(&fork.value.checkpoint()).unwrap(),
            serde_json::to_vec(&fork.donor_value).unwrap()
        );
        assert_eq!(fork.trainer.optimizer_step(), 16_777_219);
        assert_eq!(fork.retained.topology.total_synapse_count, 32769);
        assert_eq!(fork.trainer.weights().len(), 32768);
        assert_eq!(fork.retained.fast.active_bank, active);
        assert_eq!(fork.retained.fast.active_generation, 1234);
        assert_eq!(
            serde_json::to_vec(&fork.retained.predictor).unwrap(),
            serde_json::to_vec(&experiment.retained().predictor).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&fork.retained.topology).unwrap(),
            serde_json::to_vec(&experiment.retained().topology).unwrap()
        );
        for bank in [&fork.retained.fast, &fork.retained.lifetime] {
            assert!(bank
                .bank_0_bits
                .iter()
                .chain(&bank.bank_1_bits)
                .all(|v| *v == 0));
        }
        assert_eq!(serde_json::to_vec(&experiment).unwrap(), before);
    }
}

#[test]
fn retained_rebind_preserves_shifted_decoder_banks_and_rejects_unrepresentable_added_values() {
    let experiment = prepare_retained_interpolation_experiment(fixture(1)).unwrap();
    let pc = CurrentTrainingCheckpointV1::decode(&experiment.inputs().pc_current)
        .unwrap()
        .handoff
        .admit()
        .unwrap();
    let (source, _) = derive_retained_projection_from_current(
        &experiment.inputs().pc_current,
        &experiment.inputs().expected_pc_current_sha256,
    )
    .unwrap();
    let next = PhenotypeCompiler::compile_validated(
        experiment.compiler_inputs(),
        &BrainCapacityClass::n2048(),
    )
    .unwrap();
    let old_bytes = serde_json::to_vec(&source).unwrap();
    let rebound = source.rebind_weight_only(&pc.phenotype, &next).unwrap();
    assert_eq!(
        rebound.canonical_to_live(&next).unwrap(),
        source.canonical_to_live(&pc.phenotype).unwrap()
    );
    assert_eq!(rebound.fast.bank_0_bits, source.fast.bank_0_bits);
    assert_eq!(rebound.fast.bank_1_bits, source.fast.bank_1_bits);
    assert_eq!(rebound.lifetime.bank_0_bits, source.lifetime.bank_0_bits);
    assert_eq!(rebound.lifetime.bank_1_bits, source.lifetime.bank_1_bits);
    assert_eq!(rebound.topology.alpha_bits, source.topology.alpha_bits);
    assert_eq!(serde_json::to_vec(&source).unwrap(), old_bytes);
    let mut corrupt = rebound.clone();
    let map = corrupt.canonical_to_live(&next).unwrap();
    let extra = (0..corrupt.topology.total_synapse_count as usize)
        .find(|i| !map.contains(i))
        .unwrap();
    corrupt.fast.bank_1_bits[extra] = 0.2f32.to_bits();
    corrupt.fast.canonical_digest = corrupt.fast.recompute_canonical_digest().unwrap();
    assert!(corrupt.rebind_weight_only(&next, &next).is_err());
}
