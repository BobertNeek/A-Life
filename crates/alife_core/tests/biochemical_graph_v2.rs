use alife_core::{
    BiochemistryState, BodyEventDelta, BrainCapacityClass, ChemicalSpeciesId,
    FoundationGeneticIdentity, NeuralEmission, NeuralEmissionClass, NeuralEmissionFrame, Tick,
    Validate,
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

fn mature_tick(phenotype: &alife_core::CreaturePhenotype) -> Tick {
    let maturation = u64::from(phenotype.development.maturation_duration_ticks);
    Tick(((maturation + 119) / 120) * 120)
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
    let tick = mature_tick(&phenotype);
    let state = BiochemistryState::new(&phenotype, tick).unwrap();
    let before = state
        .neural_receptor_frame(&phenotype)
        .unwrap()
        .activation_for(alife_core::NeuralReceptorClass::RegionalExcitability);
    let emission = NeuralEmissionFrame::new(
        tick,
        1,
        vec![NeuralEmission::new(NeuralEmissionClass::PredictionResidual, 0.9, 0.8).unwrap()],
    )
    .unwrap();

    let next = state
        .advance_with_neural_emission(
            Tick(tick.raw() + 1),
            Tick(tick.raw() + 1),
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
fn same_tick_without_event_does_not_advance_reactions() {
    let phenotype = founder_phenotype();
    let tick = mature_tick(&phenotype);
    let state = BiochemistryState::new(&phenotype, tick).unwrap();
    let active = state
        .advance(
            Tick(tick.raw() + 1),
            BodyEventDelta {
                nutrition: 0.1,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();
    let graph = &phenotype.chemistry.biochemical;
    let nutrient = active
        .graph_state()
        .concentration(graph, ChemicalSpeciesId(19))
        .unwrap();
    let brain_atp = active
        .graph_state()
        .concentration(graph, ChemicalSpeciesId(7))
        .unwrap();
    assert!(nutrient > 0.0);
    assert!(brain_atp + 0.5 * nutrient < 1.0);

    let same_tick = active
        .advance(active.tick, BodyEventDelta::zero(), &phenotype)
        .unwrap();

    assert_eq!(same_tick.graph_state(), active.graph_state());
}

#[test]
fn drive_frame_is_a_tick_bound_derivation_of_the_graph() {
    let phenotype = founder_phenotype();
    let tick = mature_tick(&phenotype);
    let state = BiochemistryState::new(&phenotype, tick).unwrap();
    let next = state
        .advance_with_neural_emission(
            Tick(tick.raw() + 1),
            Tick(tick.raw() + 1),
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

#[test]
fn non_unit_species_ranges_survive_advance_and_restore_validation() {
    let mut wire = serde_json::to_value(founder_phenotype()).unwrap();
    let nutrient = wire["chemistry"]["biochemical"]["species"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"].as_u64() == Some(19))
        .unwrap();
    nutrient["minimum"] = serde_json::json!(1.0);
    nutrient["baseline"] = serde_json::json!(2.0);
    nutrient["maximum"] = serde_json::json!(3.0);
    let phenotype: alife_core::CreaturePhenotype = serde_json::from_value(wire).unwrap();

    let state = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    state.validate_contract().unwrap();
    state.validate_against(&phenotype).unwrap();
    let next = state
        .advance(Tick(1), BodyEventDelta::zero(), &phenotype)
        .unwrap();
    next.validate_against(&phenotype).unwrap();

    let restored: BiochemistryState =
        serde_json::from_slice(&serde_json::to_vec(&next).unwrap()).unwrap();
    restored.validate_contract().unwrap();
    restored.validate_against(&phenotype).unwrap();
    let concentration = restored
        .graph_state()
        .concentration(
            &phenotype.chemistry.biochemical,
            alife_core::ChemicalSpeciesId(19),
        )
        .unwrap();
    assert!(concentration > 1.0);
}
