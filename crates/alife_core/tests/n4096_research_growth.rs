use alife_core::{
    BrainCapacityClass, BrainGenome, CandidateActionFamily, CandidateFeatureVector,
    DevelopmentState, DriveSnapshot, NormalizedScalar, PhenotypeCompiler, PhenotypeCompilerInputs,
    PhenotypeGrowthMigration, SensorProfile, Tick,
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

    assert_eq!(
        source.sensor_encoder().assignments().len(),
        target.sensor_encoder().assignments().len(),
    );
    for old in source.sensor_encoder().assignments() {
        let mapped = grown.receipt.source_to_target_neurons[old.target_neuron() as usize];
        let new = target
            .sensor_encoder()
            .assignments()
            .iter()
            .find(|entry| {
                entry.target_neuron() == mapped
                    && entry.source_group() == old.source_group()
                    && entry.source_index() == old.source_index()
            })
            .expect("growth must preserve each inherited sensory port");
        assert_eq!(old.scale().to_bits(), new.scale().to_bits());
        assert_eq!(old.bias().to_bits(), new.bias().to_bits());
        assert_eq!(old.clamp_range(), new.clamp_range());
    }
    let mut drives = DriveSnapshot::baseline();
    drives.hunger = 0.98;
    drives.pain = 0.98;
    let mut food = CandidateFeatureVector::zero();
    food.0[15] = 0.8;
    for old in source.candidate_decoder().families() {
        let new = target
            .candidate_decoder()
            .families()
            .iter()
            .find(|entry| entry.family() == old.family())
            .unwrap();
        assert_eq!(old.innate_drive_mask(), new.innate_drive_mask());
        assert_eq!(old.innate_gain().to_bits(), new.innate_gain().to_bits());
        assert_eq!(old.innate_cue_lane(), new.innate_cue_lane());
        assert_eq!(old.innate_cue_inverted(), new.innate_cue_inverted());
        assert_eq!(old.innate_requires_reach(), new.innate_requires_reach());
        assert_eq!(
            old.innate_contribution(drives, food).to_bits(),
            new.innate_contribution(drives, food).to_bits(),
        );
        if old.family() == CandidateActionFamily::Ingest {
            assert!(new.innate_contribution(drives, food) > 0.0);
        }
    }

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

    let mismatched_genome = BrainGenome::scaffold(0x4096_2049, source_capacity.id());
    let mismatched_development = DevelopmentState::new(
        mismatched_genome.id,
        Tick::ZERO,
        NormalizedScalar::new(1.0).unwrap(),
    );
    let mismatched_inputs = PhenotypeCompilerInputs::try_new(
        mismatched_genome,
        &source_capacity,
        mismatched_development,
        source.sensor_profile(),
    )
    .unwrap();
    assert!(PhenotypeGrowthMigration::compile_n2048_to_n4096(&source, &mismatched_inputs).is_err());
}
