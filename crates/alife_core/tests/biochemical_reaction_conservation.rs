use alife_core::*;
use serde_json::json;

fn material(id: u16, minimum: f32) -> serde_json::Value {
    json!({
        "id": id,
        "kind": "Material",
        "compartment": "Circulation",
        "baseline": minimum,
        "decay_retention": 1.0,
        "minimum": minimum,
        "maximum": 1.0,
    })
}

fn regulatory(id: u16) -> serde_json::Value {
    json!({
        "id": id,
        "kind": "Regulatory",
        "compartment": "Circulation",
        "baseline": 0.0,
        "decay_retention": 1.0,
        "minimum": 0.0,
        "maximum": 1.0,
    })
}

fn reaction(reactants: &[(u16, f32)], products: &[(u16, f32)], rate: f32) -> serde_json::Value {
    let reactants = reactants
        .iter()
        .map(|&(species, amount)| json!({"species": species, "amount": amount}))
        .collect::<Vec<_>>();
    let products = products
        .iter()
        .map(|&(species, amount)| json!({"species": species, "amount": amount}))
        .collect::<Vec<_>>();
    json!({
        "reactants": reactants,
        "products": products,
        "rate": rate,
        "rate_control": null,
    })
}

fn graph(
    species: Vec<serde_json::Value>,
    reactions: Vec<serde_json::Value>,
) -> BiochemicalPhenotype {
    let species_budget = species.len();
    let reaction_budget = reactions.len();
    let graph: BiochemicalPhenotype = serde_json::from_value(json!({
        "schema_version": BIOCHEMICAL_GRAPH_SCHEMA_VERSION,
        "species_budget": species_budget,
        "reaction_budget": reaction_budget,
        "species": species,
        "reactions": reactions,
        "emitters": [],
        "receptors": [],
        "neuroemitters": [],
    }))
    .unwrap();
    graph.validate_contract().unwrap();
    graph
}

fn baseline_body() -> BodyState {
    let phenotype = CreatureGenome::early_mammal_founder(
        0xA0A_2001,
        FoundationGeneticIdentity::new(20, 1, 1, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap()
    .express()
    .unwrap();
    BiochemistryState::new(&phenotype, Tick::ZERO).unwrap().body
}

fn state_with(graph: &BiochemicalPhenotype, concentrations: &[f32]) -> BiochemicalGraphState {
    assert_eq!(concentrations.len(), graph.species().len());
    let state = BiochemicalGraphState::new(graph, Tick::ZERO, 1.0).unwrap();
    let mut wire = serde_json::to_value(state).unwrap();
    let mut padded = vec![0.0_f32; MAX_ACTIVE_CHEMICAL_SPECIES];
    padded[..concentrations.len()].copy_from_slice(concentrations);
    wire["concentrations"] = json!(padded);
    serde_json::from_value(wire).unwrap()
}

fn advance_to(
    graph: &BiochemicalPhenotype,
    state: BiochemicalGraphState,
    next_tick: u64,
) -> BiochemicalGraphState {
    state
        .advance(
            Tick(next_tick),
            baseline_body(),
            BodyEventDelta::zero(),
            None,
            graph,
            1.0,
        )
        .unwrap()
        .0
}

fn concentration(graph: &BiochemicalPhenotype, state: &BiochemicalGraphState, id: u16) -> f32 {
    state.concentration(graph, ChemicalSpeciesId(id)).unwrap()
}

fn total_material(graph: &BiochemicalPhenotype, state: &BiochemicalGraphState) -> f32 {
    graph
        .species()
        .iter()
        .filter(|species| species.kind == ChemicalSpeciesKind::Material)
        .map(|species| concentration(graph, state, species.id.0))
        .sum()
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 2.0e-6,
        "actual={actual}, expected={expected}"
    );
}

fn assert_limits(graph: &BiochemicalPhenotype, state: &BiochemicalGraphState) {
    for species in graph.species() {
        let value = concentration(graph, state, species.id.0);
        assert!(
            (species.minimum..=species.maximum).contains(&value),
            "species {:?} value {} outside [{}, {}]",
            species.id,
            value,
            species.minimum,
            species.maximum
        );
    }
}

#[test]
fn long_step_uses_one_feasible_extent_instead_of_creating_product_material() {
    let graph = graph(
        vec![material(1, 0.0), material(2, 0.0)],
        vec![reaction(&[(1, 1.0)], &[(2, 1.0)], 1.0)],
    );
    let state = state_with(&graph, &[0.2, 0.0]);
    let initial_total = total_material(&graph, &state);

    let next = advance_to(&graph, state, 2);

    assert_close(concentration(&graph, &next, 1), 0.0);
    assert_close(concentration(&graph, &next, 2), 0.2);
    assert_close(total_material(&graph, &next), initial_total);
    assert_limits(&graph, &next);
}

#[test]
fn nearly_full_product_caps_extent_without_losing_reactant_material() {
    let graph = graph(
        vec![material(1, 0.0), material(2, 0.0)],
        vec![reaction(&[(1, 1.0)], &[(2, 1.0)], 1.0)],
    );
    let state = state_with(&graph, &[0.2, 0.9]);
    let initial_total = total_material(&graph, &state);

    let next = advance_to(&graph, state, 1);

    assert_close(concentration(&graph, &next, 1), 0.1);
    assert_close(concentration(&graph, &next, 2), 1.0);
    assert_close(total_material(&graph, &next), initial_total);
    assert_limits(&graph, &next);
}

#[test]
fn proportionally_scaled_nonzero_stoichiometry_preserves_reaction_extent() {
    let unit_graph = graph(
        vec![material(1, 0.0), material(2, 0.0)],
        vec![reaction(&[(1, 1.0)], &[(2, 1.0)], 1.0)],
    );
    let scaled_graph = graph(
        vec![material(1, 0.0), material(2, 0.0)],
        vec![reaction(&[(1, 1.0e-13)], &[(2, 1.0e-13)], 1.0)],
    );
    let unit_next = advance_to(&unit_graph, state_with(&unit_graph, &[0.2, 0.0]), 1);
    let scaled_next = advance_to(&scaled_graph, state_with(&scaled_graph, &[0.2, 0.0]), 1);

    for id in [1, 2] {
        assert_close(
            concentration(&scaled_graph, &scaled_next, id),
            concentration(&unit_graph, &unit_next, id),
        );
    }
    assert_limits(&scaled_graph, &scaled_next);
}

#[test]
fn nonzero_minimum_is_reserved_when_reaction_consumes_material() {
    let graph = graph(
        vec![material(1, 0.2), material(2, 0.0)],
        vec![reaction(&[(1, 1.0)], &[(2, 1.0)], 1.0)],
    );
    let state = state_with(&graph, &[0.5, 0.1]);
    let initial_total = total_material(&graph, &state);

    let next = advance_to(&graph, state, 2);

    assert_close(concentration(&graph, &next, 1), 0.2);
    assert_close(concentration(&graph, &next, 2), 0.4);
    assert_close(total_material(&graph, &next), initial_total);
    assert_limits(&graph, &next);
}

#[test]
fn multiple_reactants_and_products_share_one_limiting_extent() {
    let graph = graph(
        vec![
            material(1, 0.0),
            material(2, 0.0),
            material(3, 0.0),
            material(4, 0.0),
        ],
        vec![reaction(&[(1, 1.0), (2, 2.0)], &[(3, 1.0), (4, 2.0)], 1.0)],
    );
    let state = state_with(&graph, &[0.8, 0.6, 0.0, 0.0]);
    let initial_total = total_material(&graph, &state);

    let next = advance_to(&graph, state, 1);

    assert_close(concentration(&graph, &next, 1), 0.5);
    assert_close(concentration(&graph, &next, 2), 0.0);
    assert_close(concentration(&graph, &next, 3), 0.3);
    assert_close(concentration(&graph, &next, 4), 0.6);
    assert_close(total_material(&graph, &next), initial_total);
    assert_limits(&graph, &next);
}

#[test]
fn duplicate_reactants_are_aggregated_before_rate_limiting() {
    let graph = graph(
        vec![material(1, 0.0), material(2, 0.0)],
        vec![reaction(&[(1, 1.0), (1, 1.0)], &[(1, 1.0), (2, 1.0)], 1.0)],
    );
    let state = state_with(&graph, &[0.8, 0.0]);
    let initial_total = total_material(&graph, &state);

    let next = advance_to(&graph, state, 1);

    assert_close(concentration(&graph, &next, 1), 0.4);
    assert_close(concentration(&graph, &next, 2), 0.4);
    assert_close(total_material(&graph, &next), initial_total);
    assert_limits(&graph, &next);
}

#[test]
fn net_zero_catalyst_stays_exactly_unchanged_at_its_maximum() {
    let graph = graph(
        vec![material(1, 0.0), material(2, 0.0), regulatory(3)],
        vec![reaction(&[(1, 1.0), (3, 1.0)], &[(2, 1.0), (3, 1.0)], 1.0)],
    );
    let state = state_with(&graph, &[0.2, 0.9, 1.0]);
    let initial_total = total_material(&graph, &state);

    let next = advance_to(&graph, state, 1);

    assert_close(concentration(&graph, &next, 1), 0.1);
    assert_close(concentration(&graph, &next, 2), 1.0);
    assert_eq!(concentration(&graph, &next, 3).to_bits(), 1.0_f32.to_bits());
    assert_close(total_material(&graph, &next), initial_total);
    assert_limits(&graph, &next);
}

#[test]
fn zero_rate_leaves_a_valid_reaction_state_unchanged() {
    let graph = graph(
        vec![material(1, 0.0), material(2, 0.0)],
        vec![reaction(&[(1, 1.0)], &[(2, 1.0)], 0.0)],
    );
    let state = state_with(&graph, &[0.8, 0.2]);

    let next = advance_to(&graph, state, 1);

    assert_eq!(concentration(&graph, &next, 1).to_bits(), 0.8_f32.to_bits());
    assert_eq!(concentration(&graph, &next, 2).to_bits(), 0.2_f32.to_bits());
    assert_limits(&graph, &next);
}

#[test]
fn full_product_provides_no_feasible_reaction_space() {
    let graph = graph(
        vec![material(1, 0.0), material(2, 0.0)],
        vec![reaction(&[(1, 1.0)], &[(2, 1.0)], 1.0)],
    );
    let state = state_with(&graph, &[0.8, 1.0]);

    let next = advance_to(&graph, state, 1);

    assert_eq!(concentration(&graph, &next, 1).to_bits(), 0.8_f32.to_bits());
    assert_eq!(concentration(&graph, &next, 2).to_bits(), 1.0_f32.to_bits());
    assert_limits(&graph, &next);
}
