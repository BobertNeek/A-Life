use alife_core::*;

fn phenotype() -> CreaturePhenotype {
    CreatureGenome::early_mammal_founder(
        940513,
        FoundationGeneticIdentity::new(
            FoundationId::N512_V1.raw(),
            1,
            FoundationCompatibilityFamilyId::N512_FOUNDATION.raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap(),
    )
    .unwrap()
    .express()
    .unwrap()
}

#[test]
fn newborn_material_reserves_follow_genes_while_regulators_remain_gated() {
    let phenotype = phenotype();
    let newborn = BiochemistryState::new_with_age(&phenotype, Tick::ZERO, Tick::ZERO).unwrap();
    assert_eq!(newborn.development.biochemical_expression, 0.0);
    let graph = &phenotype.chemistry.biochemical;
    let mut material_count = 0;
    for species in graph.species() {
        let concentration = newborn
            .graph_state()
            .concentration(graph, species.id)
            .unwrap();
        match species.kind {
            ChemicalSpeciesKind::Material => {
                material_count += 1;
                assert_eq!(
                    concentration.to_bits(),
                    species.baseline.to_bits(),
                    "material {:?}",
                    species.id
                );
            }
            ChemicalSpeciesKind::Regulatory => assert_eq!(concentration, species.minimum),
        }
    }
    assert!(
        material_count > 1,
        "verify per-species reserves, not an ATP special case"
    );
    assert!(
        !ChemistryModulation::recovery_triggers(
            &newborn.homeostasis,
            HomeostaticParameters::reference()
        )
        .unwrap()
        .catatonia_energy_hypoplasia
    );
}

#[test]
fn relaxation_preserves_material_baselines_without_activating_regulators() {
    let phenotype = phenotype();
    let graph = &phenotype.chemistry.biochemical;
    let body = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap().body;
    let state = BiochemicalGraphState::new(graph, Tick::ZERO, 1.0).unwrap();
    let (next, _) = state
        .advance(
            Tick(12),
            body,
            BodyEventDelta {
                nutrition: 1.0,
                sleep_recovery: 1.0,
                ..BodyEventDelta::zero()
            },
            None,
            graph,
            0.0,
        )
        .unwrap();
    let mut regulatory_decay = false;
    for species in graph.species() {
        let concentration = next.concentration(graph, species.id).unwrap();
        match species.kind {
            ChemicalSpeciesKind::Material => assert_eq!(
                concentration.to_bits(),
                species.baseline.to_bits(),
                "material {:?}",
                species.id
            ),
            ChemicalSpeciesKind::Regulatory => {
                assert!(concentration <= species.baseline);
                regulatory_decay |= concentration < species.baseline;
            }
        }
    }
    assert!(regulatory_decay);
}

#[test]
fn stored_concentrations_and_experienced_material_changes_are_not_reset() {
    let phenotype = phenotype();
    let graph = &phenotype.chemistry.biochemical;
    let body = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap().body;
    let initial = BiochemicalGraphState::new(graph, Tick::ZERO, 1.0).unwrap();
    let (experienced, _) = initial
        .advance(
            Tick(1),
            body,
            BodyEventDelta {
                nutrition: 0.5,
                ..BodyEventDelta::zero()
            },
            None,
            graph,
            1.0,
        )
        .unwrap();
    let bytes = serde_json::to_vec(&experienced).unwrap();
    let restored: BiochemicalGraphState = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    let (continued, _) = restored
        .advance(Tick(2), body, BodyEventDelta::zero(), None, graph, 0.0)
        .unwrap();
    let mut preserved_difference = false;
    for species in graph
        .species()
        .iter()
        .filter(|s| s.kind == ChemicalSpeciesKind::Material)
    {
        let before = experienced.concentration(graph, species.id).unwrap();
        let after = continued.concentration(graph, species.id).unwrap();
        if before != species.baseline && species.decay_retention > 0.0 {
            assert_ne!(
                after, species.baseline,
                "ordinary relaxation must not reset stored reserves"
            );
            assert!((after - species.baseline).abs() < (before - species.baseline).abs());
            preserved_difference = true;
        }
    }
    assert!(
        preserved_difference,
        "real nutrition/reaction effects must alter material concentrations"
    );
}

#[test]
fn old_zero_reserve_save_loads_exactly_and_recovers_only_by_ordinary_relaxation() {
    let phenotype = phenotype();
    let graph = &phenotype.chemistry.biochemical;
    let initial = BiochemicalGraphState::new(graph, Tick::ZERO, 1.0).unwrap();
    let mut wire = serde_json::to_value(initial).unwrap();
    for concentration in wire["concentrations"].as_array_mut().unwrap() {
        *concentration = serde_json::json!(0.0);
    }
    let restored: BiochemicalGraphState = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), wire);
    restored.validate_against(graph).unwrap();
    let body = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap().body;
    let (advanced, _) = restored
        .advance(Tick(1), body, BodyEventDelta::zero(), None, graph, 0.0)
        .unwrap();
    let reserve = graph
        .species()
        .iter()
        .find(|species| {
            species.kind == ChemicalSpeciesKind::Material
                && species.baseline > 0.0
                && species.decay_retention > 0.0
                && species.decay_retention < 1.0
        })
        .unwrap();
    assert_eq!(restored.concentration(graph, reserve.id).unwrap(), 0.0);
    let concentration = advanced.concentration(graph, reserve.id).unwrap();
    assert!(concentration > 0.0 && concentration < reserve.baseline);
}
