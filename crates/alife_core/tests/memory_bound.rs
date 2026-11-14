use alife_core::*;

fn compiled(layout: u16, bound: Option<f32>) -> (BrainGenome, BrainPhenotype) {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(2242, capacity.id());
    let mut architecture = *genome.cognitive_architecture();
    architecture = match layout {
        1 => architecture,
        2 => architecture.with_independent_orientation_v2(2).unwrap(),
        3 => architecture.with_grounded_orientation_v3(3).unwrap(),
        _ => unreachable!(),
    };
    if let Some(bound) = bound {
        architecture = architecture.with_memory_candidate_bound_v4(bound).unwrap();
    }
    let genome = genome.with_cognitive_architecture(architecture).unwrap();
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.8).unwrap());
    let phenotype = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    (genome, phenotype)
}

#[test]
fn bound_is_compiled_without_rewiring_or_changing_motor_layout() {
    for layout in 1..=3 {
        let (_, old) = compiled(layout, None);
        let (_, new) = compiled(layout, Some(1.0));
        let channel = new.candidate_decoder().memory_channel().unwrap();
        assert_eq!(channel.schema_version(), 2);
        assert_eq!(channel.max_candidate_gain(), 1.0);
        assert_eq!(
            old.candidate_decoder()
                .memory_channel()
                .unwrap()
                .schema_version(),
            1
        );
        assert_eq!(old.synapses(), new.synapses());
        assert_eq!(old.neuron_dynamics(), new.neuron_dynamics());
        assert_eq!(old.sensor_encoder(), new.sensor_encoder());
        assert_eq!(
            old.candidate_decoder()
                .factorized_motor_channels(&old)
                .unwrap(),
            new.candidate_decoder()
                .factorized_motor_channels(&new)
                .unwrap()
        );
        assert_eq!(
            old.cognitive_architecture()
                .grounded_orientation_repertoire(),
            new.cognitive_architecture()
                .grounded_orientation_repertoire()
        );
        assert_ne!(old.phenotype_hash(), new.phenotype_hash());
    }
}

#[test]
fn extended_genome_and_phenotype_roundtrip_and_reject_changed_bound() {
    let (genome, phenotype) = compiled(2, Some(1.0));
    let genome_bytes = serde_json::to_vec(&genome).unwrap();
    let restored: BrainGenome = serde_json::from_slice(&genome_bytes).unwrap();
    assert_eq!(genome_bytes, serde_json::to_vec(&restored).unwrap());
    let bytes = serde_json::to_vec(&phenotype).unwrap();
    let restored: BrainPhenotype = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
    let mut altered = serde_json::to_value(&phenotype).unwrap();
    altered["cognitive_architecture"]["parameters"]["memory_candidate_bound"] = 2.0.into();
    assert!(serde_json::from_value::<BrainPhenotype>(altered).is_err());
}

#[test]
fn public_creature_expression_keeps_paternal_and_maternal_bound_alleles() {
    let original = CreatureGenome::early_mammal_founder(
        2242,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N2048_ID).unwrap(),
    )
    .unwrap();
    let mut bounds = Vec::new();
    for paternal in [false, true] {
        let mut genome = original.clone();
        let construction = &mut genome.brain.construction;
        let legacy = construction.maternal_architecture;
        let opted = legacy.with_memory_candidate_bound_v4(1.0).unwrap();
        construction.maternal_architecture = if paternal { legacy } else { opted };
        construction.paternal_architecture = if paternal { opted } else { legacy };
        let cold: CreatureGenome =
            serde_json::from_slice(&serde_json::to_vec(&genome).unwrap()).unwrap();
        let expressed = cold.express().unwrap();
        let architecture = expressed.brain_genome.cognitive_architecture();
        assert_eq!(architecture.schema_version(), 4);
        assert_eq!(
            architecture.motor_layout_schema_version(),
            legacy.motor_layout_schema_version()
        );
        let bound = architecture.memory_candidate_bound();
        assert!(bound > 0.5 && bound < 1.0);
        bounds.push(bound.to_bits());
    }
    assert_eq!(bounds[0], bounds[1]);
    assert_ne!(
        original
            .express()
            .unwrap()
            .brain_genome
            .cognitive_architecture()
            .schema_version(),
        4
    );
}
