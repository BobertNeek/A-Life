use alife_core::{
    BiochemicalGraphState, BiochemicalPhenotype, BiochemistryState, BodyEventDelta,
    BrainCapacityClass, ChemicalSpeciesId, FoundationGeneticIdentity, NeuralEmission,
    NeuralEmissionClass, NeuralEmissionFrame, Tick, Validate, BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
};
use serde_json::json;

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
    assert_eq!(
        next.biochemical_work().neural_emitter_evaluations,
        phenotype.chemistry.biochemical.neuroemitters().len() as u32
    );
}

#[test]
fn neural_emitter_work_counts_configured_emitters_not_input_emissions() {
    for (configured_emitters, input_emissions) in [(0usize, 1usize), (1, 3), (3, 1)] {
        let mut wire = serde_json::to_value(founder_phenotype()).unwrap();
        let neuroemitters = wire["chemistry"]["biochemical"]["neuroemitters"]
            .as_array()
            .unwrap()
            .iter()
            .take(configured_emitters)
            .cloned()
            .collect::<Vec<_>>();
        wire["chemistry"]["biochemical"]["neuroemitters"] = json!(neuroemitters);
        let phenotype: alife_core::CreaturePhenotype = serde_json::from_value(wire).unwrap();
        let tick = mature_tick(&phenotype);
        let state = BiochemistryState::new(&phenotype, tick).unwrap();
        let emission =
            NeuralEmission::new(NeuralEmissionClass::PredictionResidual, 0.9, 0.8).unwrap();
        let frame = NeuralEmissionFrame::new(tick, 1, vec![emission; input_emissions]).unwrap();

        let next = state
            .advance_with_neural_emission(
                Tick(tick.raw() + 1),
                Tick(tick.raw() + 1),
                BodyEventDelta::zero(),
                Some(&frame),
                &phenotype,
            )
            .unwrap();

        assert_eq!(
            next.biochemical_work().neural_emitter_evaluations,
            configured_emitters as u32
        );
    }
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
fn emitter_catch_up_replays_persistent_sources_but_not_instantaneous_events() {
    for (source, event, expected) in [
        ("Basal", BodyEventDelta::zero(), 0.20),
        (
            "Nutrition",
            BodyEventDelta {
                nutrition: 0.5,
                ..BodyEventDelta::zero()
            },
            0.05,
        ),
    ] {
        let graph: BiochemicalPhenotype = serde_json::from_value(json!({
            "schema_version": BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
            "species_budget": 1,
            "reaction_budget": 0,
            "species": [{
                "id": 1,
                "kind": "Regulatory",
                "compartment": "Circulation",
                "baseline": 0.0,
                "decay_retention": 1.0,
                "minimum": 0.0,
                "maximum": 1.0,
            }],
            "reactions": [],
            "emitters": [{
                "source": source,
                "target": 1,
                "cadence_ticks": 1,
                "threshold": 0.0,
                "gain": 0.1,
                "response": "Analogue",
                "inverted": false,
                "developmental_expression_floor": 0.0,
            }],
            "receptors": [],
            "neuroemitters": [],
        }))
        .unwrap();
        let body = BiochemistryState::new(&founder_phenotype(), Tick::ZERO)
            .unwrap()
            .body;
        let initial = BiochemicalGraphState::new(&graph, Tick::ZERO, 1.0).unwrap();
        let (first, _) = initial
            .advance(Tick(1), body, event, None, &graph, 1.0)
            .unwrap();
        let (partitioned, _) = first
            .advance(Tick(2), body, BodyEventDelta::zero(), None, &graph, 1.0)
            .unwrap();
        let (jumped, _) = initial
            .advance(Tick(2), body, event, None, &graph, 1.0)
            .unwrap();

        assert!(
            (partitioned
                .concentration(&graph, ChemicalSpeciesId(1))
                .unwrap()
                - expected)
                .abs()
                < 1e-6
        );
        assert_eq!(
            jumped.concentration(&graph, ChemicalSpeciesId(1)).unwrap(),
            partitioned
                .concentration(&graph, ChemicalSpeciesId(1))
                .unwrap()
        );
    }
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
fn biochemistry_validation_requires_graph_and_owner_ticks_to_match() {
    let phenotype = founder_phenotype();
    let state = BiochemistryState::new(&phenotype, Tick(1)).unwrap();
    state.validate_contract().unwrap();

    for (label, graph_tick) in [("older", Tick::ZERO), ("newer", Tick(2))] {
        let mut wire = serde_json::to_value(state).unwrap();
        wire["graph_state"]["tick"] = json!(graph_tick.raw());
        let restored: BiochemistryState = serde_json::from_value(wire).unwrap();

        assert_eq!(
            restored.validate_contract(),
            Err(alife_core::ScaffoldContractError::NonMonotonicTick),
            "{label} graph tick must be rejected"
        );
    }
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

#[test]
fn grouped_receptors_use_inherited_baselines_and_responses_after_reload() {
    for target in [
        json!({"Drive":"Hunger"}),
        json!({"Endocrine":"Cortisol"}),
        json!({"Neural":"Sleep"}),
    ] {
        for digital in [false, true] {
            let receptors = vec![
                json!({"source":1,"target":target,"threshold":0.25,"gain":1.0,"nominal":0.2,"digital":digital}),
                json!({"source":1,"target":target,"threshold":0.5,"gain":0.5,"nominal":0.4}),
                json!({"source":1,"target":target,"threshold":0.5,"gain":-0.5,"nominal":0.3}),
            ];
            let mut wire = json!({"schema_version":BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
                "species_budget":1,"reaction_budget":0,
                "species":[{"id":1,"kind":"Regulatory","compartment":"Circulation","baseline":0.75,
                    "decay_retention":1.0,"minimum":0.0,"maximum":1.0}],
                "reactions":[],"emitters":[],"neuroemitters":[],"receptors":receptors});
            let expected = if digital { 0.7375 } else { 0.4875 };
            for _ in 0..2 {
                let graph: BiochemicalPhenotype = serde_json::from_value(wire.clone()).unwrap();
                let encoded = serde_json::to_value(&graph).unwrap();
                assert!(encoded.get("compiled").is_none());
                let reloaded: BiochemicalPhenotype = serde_json::from_value(encoded).unwrap();
                assert_eq!(graph, reloaded);
                let state = BiochemicalGraphState::new(&reloaded, Tick::ZERO, 1.0).unwrap();
                let home = state.derive_homeostasis(&reloaded).unwrap();
                let actual = if target.get("Drive").is_some() {
                    home.drives.hunger
                } else if target.get("Endocrine").is_some() {
                    home.hormones.cortisol
                } else {
                    state
                        .neural_receptor_frame(&reloaded)
                        .unwrap()
                        .activation_for(alife_core::NeuralReceptorClass::Sleep)
                };
                assert!((actual - expected).abs() < 1e-6, "{target}: {actual}");
                wire["receptors"].as_array_mut().unwrap().reverse();
            }
        }
    }
}
