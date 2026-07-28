use alife_core::{
    BrainCapacityClass, BrainGenome, CompiledSynapseKind, DecoderHeadKind, DevelopmentState,
    NormalizedScalar, PhenotypeCompiler, PlasticityGenomeParameters, ScaffoldContractError,
    SensorProfile, Tick,
};

fn compile(genome: &BrainGenome) -> alife_core::BrainPhenotype {
    let capacity = BrainCapacityClass::production_for_id(genome.brain_class_id).unwrap();
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    PhenotypeCompiler::compile(
        genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
}

#[test]
fn every_compiled_synapse_has_a_valid_receptor_and_replay_is_bounded() {
    for capacity in BrainCapacityClass::production_classes() {
        let phenotype = compile(&BrainGenome::scaffold(0xB002, capacity.id()));
        assert!(!phenotype.plasticity_receptors().is_empty());
        assert!(phenotype.synapses().iter().all(|synapse| {
            usize::from(synapse.receptor_index()) < phenotype.plasticity_receptors().len()
        }));
        let replay = phenotype.replay_capture_plan();
        replay.validate_against(&phenotype, &capacity).unwrap();
        assert!(replay
            .global_synapse_ids()
            .windows(2)
            .all(|pair| pair[0] < pair[1]));
        assert!(replay
            .global_synapse_ids()
            .iter()
            .all(|id| *id < phenotype.synapses().len() as u32));
        assert_eq!(
            phenotype.budgets().global.replay_capture_synapse_count,
            replay.global_synapse_ids().len() as u32
        );
        phenotype
            .sleep_consolidation_plan()
            .validate_contract()
            .unwrap();
    }
}

#[test]
fn bounded_replay_capture_covers_every_action_family() {
    let phenotype = compile(&BrainGenome::scaffold(0xB002, BrainCapacityClass::N512_ID));
    let mut captured_families = [false; 8];
    for id in phenotype.replay_capture_plan().global_synapse_ids() {
        if let CompiledSynapseKind::Decoder(coordinate) = phenotype.synapses()[*id as usize].kind()
        {
            if coordinate.head() == DecoderHeadKind::ActionCandidate {
                captured_families[usize::from(coordinate.family().raw())] = true;
            }
        }
    }
    let mut expected_families = [false; 8];
    for family in phenotype.candidate_decoder().families() {
        expected_families[usize::from(family.family().raw())] = family.decoder_synapse_count() > 0;
    }

    assert_eq!(captured_families, expected_families);
}

#[test]
fn procedural_sensorimotor_routes_learn_slowly_relative_to_action_decoders() {
    for class_id in [BrainCapacityClass::N512_ID, BrainCapacityClass::N1024_ID] {
        let phenotype = compile(&BrainGenome::scaffold(0xB005, class_id));
        let recurrent_max = phenotype
            .synapses()
            .iter()
            .filter(|synapse| matches!(synapse.kind(), CompiledSynapseKind::Recurrent))
            .map(|synapse| {
                phenotype.plasticity_receptors()[usize::from(synapse.receptor_index())]
                    .learning_rate()
            })
            .fold(0.0_f32, f32::max);
        let decoder_min = phenotype
            .synapses()
            .iter()
            .filter_map(|synapse| match synapse.kind() {
                CompiledSynapseKind::Decoder(coordinate)
                    if coordinate.head() == DecoderHeadKind::ActionCandidate =>
                {
                    Some(
                        phenotype.plasticity_receptors()[usize::from(synapse.receptor_index())]
                            .learning_rate(),
                    )
                }
                _ => None,
            })
            .fold(f32::INFINITY, f32::min);

        assert!(recurrent_max > 0.0);
        assert!(decoder_min.is_finite());
        assert!(
            decoder_min >= recurrent_max * 8.0,
            "class={} recurrent_max={} decoder_min={}",
            class_id.raw(),
            recurrent_max,
            decoder_min,
        );
    }
}

#[test]
fn synapse_and_decoder_discriminants_are_explicit_and_checked() {
    assert_eq!(CompiledSynapseKind::Recurrent.kind_raw(), 1);
    assert_eq!(
        CompiledSynapseKind::validate_kind_raw(0),
        Err(ScaffoldContractError::PhenotypeCompile)
    );
    assert!(CompiledSynapseKind::validate_kind_raw(1).is_ok());
    assert!(CompiledSynapseKind::validate_kind_raw(2).is_ok());
    assert!(CompiledSynapseKind::validate_kind_raw(3).is_err());

    for (raw, expected) in [
        (1, DecoderHeadKind::ActionCandidate),
        (2, DecoderHeadKind::MemoryContext),
        (3, DecoderHeadKind::SpeechPayload),
    ] {
        assert_eq!(DecoderHeadKind::try_from_raw(raw).unwrap(), expected);
        assert_eq!(expected.raw(), raw);
    }
    assert!(DecoderHeadKind::try_from_raw(0).is_err());
    assert!(DecoderHeadKind::try_from_raw(4).is_err());
    assert!(DecoderHeadKind::try_from_raw(u32::MAX).is_err());
}

#[test]
fn every_genome_learning_lane_changes_the_compiled_plasticity_identity() {
    let baseline_genome = BrainGenome::scaffold(0xB003, BrainCapacityClass::N2048_ID);
    let baseline = compile(&baseline_genome);
    let mutations = [
        ("eligibility_decay", serde_json::json!(0.81)),
        ("base_learning_rate", serde_json::json!(0.02)),
        ("normalization_rate", serde_json::json!(0.003)),
        ("sleep_replay_rate", serde_json::json!(0.4)),
        ("modulator_sign", serde_json::json!(-1.0)),
        ("fast_min", serde_json::json!(-3.0)),
        ("fast_max", serde_json::json!(3.0)),
        ("sleep_staging_rate", serde_json::json!(0.6)),
        ("sleep_weight_limit", serde_json::json!(5.0)),
        ("sleep_fast_decay_rate", serde_json::json!(0.4)),
    ];
    for (field, replacement) in mutations {
        let mut json = serde_json::to_value(&baseline_genome).unwrap();
        json["plasticity_parameters"][field] = replacement;
        let mutated: BrainGenome = serde_json::from_value(json).unwrap();
        let compiled = compile(&mutated);
        assert_ne!(
            compiled.plasticity_plan_digest(),
            baseline.plasticity_plan_digest(),
            "field={field}"
        );
        assert_ne!(
            compiled.phenotype_hash(),
            baseline.phenotype_hash(),
            "field={field}"
        );
    }
}

#[test]
fn invalid_genome_learning_parameters_and_stale_phenotypes_are_rejected() {
    let genome = BrainGenome::scaffold(0xB004, BrainCapacityClass::N2048_ID);
    for (field, replacement) in [
        ("eligibility_decay", serde_json::json!(f32::NAN)),
        ("base_learning_rate", serde_json::json!(0.0)),
        ("modulator_sign", serde_json::json!(0.0)),
        ("fast_min", serde_json::json!(9.0)),
        ("sleep_weight_limit", serde_json::json!(0.0)),
    ] {
        let mut json = serde_json::to_value(&genome).unwrap();
        json["plasticity_parameters"][field] = replacement;
        assert!(
            serde_json::from_value::<BrainGenome>(json).is_err(),
            "field={field}"
        );
    }

    let phenotype = compile(&genome);
    let mut json = serde_json::to_value(&phenotype).unwrap();
    json["plasticity_receptors"][1]["learning_rate"] = serde_json::json!(0.123);
    assert!(serde_json::from_value::<alife_core::BrainPhenotype>(json).is_err());
}

#[test]
fn plasticity_genome_parameters_round_trip_only_through_validated_construction() {
    let parameters = PlasticityGenomeParameters::try_new_v1(
        0.95, 0.01, 0.001, 0.25, 1.0, -2.0, 2.0, 0.5, 4.0, 0.5,
    )
    .unwrap();
    let round_trip: PlasticityGenomeParameters =
        serde_json::from_str(&serde_json::to_string(&parameters).unwrap()).unwrap();
    assert_eq!(round_trip, parameters);
}
