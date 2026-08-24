use alife_core::{
    AlleleSide, BiochemistryState, BodyEventDelta, BrainCapacityClass, DevelopmentState,
    EffectorCapability, EmbodimentState, FoundationGeneticIdentity, LegacyLobeKindV1, LobeKind,
    NeuralReceptorActivation, NeuralReceptorClass, NeuralReceptorEffects, NeuralReceptorFrame,
    NeuralReceptorPhenotype, NeuromodulatoryFrame, NormalizedScalar, OrganKind,
    PlasticityReceptorProfile, SensorCapability, SensorProfile, Tick, WorldEntityId,
    BIOCHEMICAL_GRAPH_SCHEMA_VERSION, NEUROMODULATORY_LANE_COUNT,
};

#[test]
fn founder_region_abi_contains_exactly_the_nine_v2_homologues() {
    assert_eq!(
        LobeKind::ALL,
        [
            LobeKind::PerceptualIntegration,
            LobeKind::InteroceptiveMotivational,
            LobeKind::MultimodalAssociation,
            LobeKind::TemporalPredictive,
            LobeKind::WorkingContextExecutive,
            LobeKind::MemoryInterface,
            LobeKind::ActionPlanning,
            LobeKind::SocialCommunication,
            LobeKind::FlexibleReserve,
        ]
    );
    assert_eq!(LobeKind::ALL.len(), 9);
}

#[test]
fn typed_organs_can_fail_locally_without_rewriting_the_brain_genome() {
    let genome = alife_core::CreatureGenome::early_mammal_founder(
        0xA0A2_0002,
        FoundationGeneticIdentity::new(92, 2, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let brain_before = serde_json::to_vec(&phenotype.brain_genome).unwrap();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let damaged = state
        .advance(
            Tick::new(1),
            BodyEventDelta {
                damage: 0.5,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();

    assert!(
        damaged.body.organ(OrganKind::Locomotor).damage
            > damaged.body.organ(OrganKind::Digestive).damage
    );
    assert_ne!(
        damaged
            .body
            .organ(OrganKind::NeuralSupport)
            .energy
            .to_bits(),
        damaged.body.organ(OrganKind::Digestive).energy.to_bits()
    );
    assert_eq!(
        brain_before,
        serde_json::to_vec(&phenotype.brain_genome).unwrap()
    );
}

#[test]
fn embodiment_calibration_advances_independently_of_cognitive_state() {
    let mut embodiment = EmbodimentState::reference(WorldEntityId(77), Tick::ZERO).unwrap();
    let original_revision = embodiment.revision();
    embodiment
        .replace_calibration(Tick::new(1), vec![0.1; 26], vec![0.2; 11], vec![-0.1; 16])
        .unwrap();
    assert_eq!(embodiment.revision(), original_revision + 1);
    assert_eq!(embodiment.source_tick(), Tick::new(1));
    assert_eq!(
        embodiment.sensor_gain(SensorCapability::Vision).to_bits(),
        1.05_f32.to_bits()
    );
    assert_eq!(
        embodiment
            .effector_gain(EffectorCapability::Translation)
            .to_bits(),
        1.1_f32.to_bits()
    );
    assert!(embodiment
        .replace_calibration(Tick::new(2), vec![0.1; 25], vec![0.2; 11], vec![0.0; 16])
        .is_err());
    assert_eq!(embodiment.source_tick(), Tick::new(1));
}

#[test]
fn biochemical_graph_topology_is_expressed_from_heritable_graph_genes() {
    let mut genome = alife_core::CreatureGenome::early_mammal_founder(
        0xA0A2_0001,
        FoundationGeneticIdentity::new(91, 2, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let original = genome.express().unwrap().chemistry.biochemical.reactions()[0].rate;
    genome.chemistry.graph = genome
        .chemistry
        .graph
        .clone()
        .with_reaction_rate(AlleleSide::Maternal, 0, original * 0.5)
        .unwrap();
    let expressed = genome.express().unwrap();

    assert_eq!(
        expressed.chemistry.biochemical.reactions()[0]
            .rate
            .to_bits(),
        (original * 0.5).to_bits()
    );
}

#[test]
fn heritable_development_schedule_causally_gates_biochemical_expression() {
    let genome = alife_core::CreatureGenome::early_mammal_founder(
        0xA0A2_0011,
        FoundationGeneticIdentity::new(93, 2, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let newborn = BiochemistryState::new_with_age(&phenotype, Tick::ZERO, Tick::ZERO).unwrap();
    let adult_tick = Tick::new(u64::from(phenotype.development.maturation_duration_ticks));
    let adult = BiochemistryState::new_with_age(&phenotype, adult_tick, adult_tick).unwrap();

    assert_eq!(newborn.development.biochemical_expression, 0.0);
    assert_eq!(adult.development.biochemical_expression, 1.0);
    let species = phenotype.chemistry.biochemical.species()[0].id;
    assert!(
        adult
            .graph_state()
            .concentration(&phenotype.chemistry.biochemical, species)
            .unwrap()
            > newborn
                .graph_state()
                .concentration(&phenotype.chemistry.biochemical, species)
                .unwrap()
    );
}

#[test]
fn legacy_region_ids_migrate_deterministically_without_becoming_founder_ids() {
    let expected = [
        LobeKind::PerceptualIntegration,
        LobeKind::InteroceptiveMotivational,
        LobeKind::SocialCommunication,
        LobeKind::SocialCommunication,
        LobeKind::MultimodalAssociation,
        LobeKind::TemporalPredictive,
        LobeKind::MemoryInterface,
        LobeKind::WorkingContextExecutive,
        LobeKind::ActionPlanning,
        LobeKind::FlexibleReserve,
        LobeKind::SocialCommunication,
        LobeKind::MultimodalAssociation,
        LobeKind::TemporalPredictive,
        LobeKind::SocialCommunication,
        LobeKind::WorkingContextExecutive,
        LobeKind::TemporalPredictive,
        LobeKind::ActionPlanning,
    ];
    for (raw, founder) in (1_u16..=17).zip(expected) {
        let legacy = LegacyLobeKindV1::try_from_raw(raw).unwrap();
        assert_eq!(legacy.raw(), raw);
        assert_eq!(legacy.migrate_to_founder(), founder);
    }
    assert!(LobeKind::try_from_raw(10).is_err());
}

#[test]
fn receptor_profiles_project_the_same_lane_frame_to_different_local_factors() {
    assert_eq!(NEUROMODULATORY_LANE_COUNT, 8);
    let frame = NeuromodulatoryFrame::try_new([0.7, 0.2, 0.8, -0.4, 0.3, 0.5, -0.6, 0.1]).unwrap();
    let appetitive =
        PlasticityReceptorProfile::try_new([0.0, 0.0, 1.0, 0.0, 0.2, 0.0, 0.0, 0.0]).unwrap();
    let aversive =
        PlasticityReceptorProfile::try_new([0.0, -1.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0]).unwrap();

    assert!(appetitive.project(&frame).unwrap() > 0.0);
    assert!(aversive.project(&frame).unwrap() < 0.0);
    assert_ne!(
        appetitive.project(&frame).unwrap().to_bits(),
        aversive.project(&frame).unwrap().to_bits()
    );
}

#[test]
fn neuromodulatory_lanes_fail_closed_on_non_finite_or_out_of_range_values() {
    assert!(NeuromodulatoryFrame::try_new([f32::NAN; 8]).is_err());
    assert!(NeuromodulatoryFrame::try_new([1.01; 8]).is_err());
    assert!(PlasticityReceptorProfile::try_new([2.01; 8]).is_err());
}

#[test]
fn neuromodulatory_serde_preserves_every_lane_and_rejects_missing_frames() {
    let sample = alife_core::NeuromodulatorSample::from_frame(
        NeuromodulatoryFrame::try_new([0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, -0.8]).unwrap(),
    );
    let encoded = serde_json::to_vec(&sample).unwrap();
    let decoded: alife_core::NeuromodulatorSample = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded.frame(), sample.frame());

    let legacy_without_frame =
        br#"{"prediction_residual":0.1,"pain":0.2,"homeostatic_improvement":0.3,"frustration":0.4,"novelty":0.5}"#;
    assert!(
        serde_json::from_slice::<alife_core::NeuromodulatorSample>(legacy_without_frame).is_err()
    );
}

#[test]
fn every_targeted_chemistry_receptor_class_has_a_bounded_neural_effect() {
    let genome = alife_core::BrainGenome::scaffold(0xA0A2_0003, BrainCapacityClass::N512_ID);
    let brain = alife_core::PhenotypeCompiler::compile(
        &genome,
        &BrainCapacityClass::production_for_id(BrainCapacityClass::N512_ID).unwrap(),
        &DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap()),
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap();
    let receptor_phenotype = NeuralReceptorPhenotype::compile(&brain).unwrap();
    let activations = [
        NeuralReceptorClass::InteroceptiveInput,
        NeuralReceptorClass::RegionalExcitability,
        NeuralReceptorClass::ProjectionGain,
        NeuralReceptorClass::LocalThreshold,
        NeuralReceptorClass::AttentionGate,
        NeuralReceptorClass::PlasticityAppetitive,
        NeuralReceptorClass::PlasticityAversive,
        NeuralReceptorClass::StructuralGrowth,
        NeuralReceptorClass::Sleep,
        NeuralReceptorClass::Consolidation,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, class)| NeuralReceptorActivation {
        class,
        signal: 0.1 + index as f32 * 0.08,
    })
    .collect();
    let frame = NeuralReceptorFrame {
        schema_version: BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
        source_chemistry_version: BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
        source_tick: Tick(5),
        activations,
    };
    let effects = NeuralReceptorEffects::from_frame(&frame, &receptor_phenotype).unwrap();
    assert_eq!(effects.source_tick, Tick(5));
    assert!(effects.interoceptive_gain != effects.regional_excitability);
    assert!(effects.projection_gain != effects.attention_gain);
    assert!(effects.local_threshold_shift != 0.0);
    assert!(effects.plasticity_appetitive != effects.plasticity_aversive);
    assert!(effects.structural_growth_gate > 0.0);
    assert!(effects.sleep_gate > 0.0);
    assert!(effects.consolidation_gate > 0.0);
}
