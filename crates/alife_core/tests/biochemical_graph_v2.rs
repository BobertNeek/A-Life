use alife_core::{
    BiochemistryState, BodyEventDelta, BrainCapacityClass, FoundationGeneticIdentity,
    NeuralEmission, NeuralEmissionClass, NeuralEmissionFrame, Tick, Validate,
};

fn founder_phenotype() -> alife_core::CreaturePhenotype {
    alife_core::CreatureGenome::early_mammal_founder(
        0xA0A_2001,
        FoundationGeneticIdentity::new(20, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap()
}

#[test]
fn founder_chemistry_is_a_bounded_sparse_active_graph() {
    let phenotype = founder_phenotype();
    let graph = &phenotype.chemistry.biochemical;

    graph.validate_contract().unwrap();
    assert!(!graph.species().is_empty());
    assert!(!graph.reactions().is_empty());
    assert!(!graph.emitters().is_empty());
    assert!(!graph.receptors().is_empty());
    assert!(!graph.neuroemitters().is_empty());
    assert!(graph.species().len() < graph.species_budget());
    assert!(graph.reactions().len() < graph.reaction_budget());
}

#[test]
fn neural_emission_changes_authoritative_chemistry_and_targeted_receptors() {
    let phenotype = founder_phenotype();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let before = state
        .neural_receptor_frame(&phenotype)
        .unwrap()
        .activation_for(alife_core::NeuralReceptorClass::RegionalExcitability);
    let emission = NeuralEmissionFrame::new(
        Tick::ZERO,
        1,
        vec![NeuralEmission::new(NeuralEmissionClass::PredictionResidual, 0.9, 0.8).unwrap()],
    )
    .unwrap();

    let next = state
        .advance_with_neural_emission(
            Tick(1),
            Tick(1),
            BodyEventDelta::zero(),
            Some(&emission),
            &phenotype,
        )
        .unwrap();
    let after = next
        .neural_receptor_frame(&phenotype)
        .unwrap()
        .activation_for(alife_core::NeuralReceptorClass::RegionalExcitability);

    assert_ne!(next.graph_state(), state.graph_state());
    assert!(after > before);
    assert_eq!(next.biochemical_work().neural_emitter_evaluations, 1);
}

#[test]
fn drive_frame_is_a_tick_bound_derivation_of_the_graph() {
    let phenotype = founder_phenotype();
    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    let next = state
        .advance_with_neural_emission(
            Tick(1),
            Tick(1),
            BodyEventDelta {
                nutrition: 0.8,
                ..BodyEventDelta::zero()
            },
            None,
            &phenotype,
        )
        .unwrap();

    assert_eq!(next.homeostasis.tick, next.tick);
    assert!(next.homeostasis.drives.hunger < state.homeostasis.drives.hunger);
    assert!(next.biochemical_work().emitter_evaluations > 0);
    assert_eq!(
        next.biochemical_work().species_updates,
        phenotype.chemistry.biochemical.species().len() as u32
    );
}
