use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
    PhenotypeCompilerInputs, PhenotypeGrowthMigration, SensorProfile, Tick,
};

#[test]
fn n4096_growth_preserves_n2048_addresses_and_stays_unpromoted() {
    let source_capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(0x4096_2048, source_capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let inputs = PhenotypeCompilerInputs::try_new(
        genome,
        &source_capacity,
        development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    let source = PhenotypeCompiler::compile_validated(&inputs, &source_capacity).unwrap();
    let grown = PhenotypeGrowthMigration::compile_n2048_to_n4096(&source, &inputs).unwrap();
    let target = &grown.phenotype;

    assert_eq!(target.neuron_count(), 4_096);
    assert_eq!(target.budgets().global.recurrent_synapses, 49_152);
    assert_eq!(target.budgets().global.action_decoder_synapses, 8_192);
    assert_eq!(target.budgets().global.memory_decoder_synapses, 8_192);
    assert_eq!(target.candidate_decoder().decoder_synapse_count(), 7_168);
    assert_eq!(
        target.speech_decoder().unwrap().decoder_synapse_count(),
        1_024
    );
    assert_eq!(
        target.memory_decoder().unwrap().decoder_synapse_count(),
        8_192
    );
    assert_eq!(source.language_codebook(), target.language_codebook());
    assert!(!grown.receipt.promoted);
    assert!(BrainCapacityClass::production_for_id(BrainCapacityClass::N4096_RESEARCH_ID).is_err());

    for old in source.persistent_address_map().neurons() {
        let mapped = grown.receipt.source_to_target_neurons[old.packed_index() as usize];
        let new = target
            .persistent_address_map()
            .neurons()
            .iter()
            .find(|entry| entry.packed_index() == mapped)
            .unwrap();
        assert_eq!(old.address(), new.address());
    }
    for old in source.persistent_address_map().synapses() {
        let mapped = grown.receipt.source_to_target_synapses[old.packed_index() as usize];
        let new = target
            .persistent_address_map()
            .synapses()
            .iter()
            .find(|entry| entry.packed_index() == mapped)
            .unwrap();
        assert_eq!(old.address(), new.address());
        assert_eq!(
            source.synapses()[old.packed_index() as usize]
                .genetic_weight()
                .to_bits(),
            target.synapses()[mapped as usize]
                .genetic_weight()
                .to_bits(),
        );
    }
    let json = serde_json::to_vec(&grown).unwrap();
    let restored: PhenotypeGrowthMigration = serde_json::from_slice(&json).unwrap();
    assert_eq!(restored, grown);
}
