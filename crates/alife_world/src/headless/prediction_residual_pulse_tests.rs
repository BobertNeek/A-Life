use super::*;

#[test]
fn late_tick_failure_preserves_each_creatures_pulse_for_exact_retry() {
    let ids = [OrganismId(1), OrganismId(2)];
    let mut world = HeadlessScenarioBuilder::new(921)
        .agent("a", ids[0], Vec3f::ZERO)
        .agent("b", ids[1], Vec3f::new(3.0, 0.0, 0.0))
        .build()
        .unwrap();
    let genomes = ids.map(|id| {
        alife_core::CreatureGenome::early_mammal_founder(
            921 + id.raw(),
            alife_core::FoundationGeneticIdentity::new(
                10,
                1,
                7,
                alife_core::BrainCapacityClass::N512_ID,
            )
            .unwrap(),
        )
        .unwrap()
    });
    let phenotypes = genomes.clone().map(|genome| genome.express().unwrap());
    let mature = phenotypes
        .iter()
        .map(|phenotype| u64::from(phenotype.development.maturation_duration_ticks))
        .max()
        .unwrap();
    let current = Tick(mature.div_ceil(120) * 120);
    world.tick = current;
    let mut expected = Vec::new();
    for (index, ((id, genome), phenotype)) in
        ids.into_iter().zip(genomes).zip(phenotypes).enumerate()
    {
        let entity = world.entity_id(if index == 0 { "a" } else { "b" }).unwrap();
        let biology = alife_core::BiochemistryState::new(&phenotype, current).unwrap();
        let activity = if index == 0 { 0.2 } else { 0.9 };
        let neural = alife_core::NeuralEmissionFrame::new(
            current,
            7,
            vec![alife_core::NeuralEmission::new(
                alife_core::NeuralEmissionClass::PredictionResidual,
                activity,
                1.0,
            )
            .unwrap()],
        )
        .unwrap();
        expected.push(
            biology
                .advance_with_neural_emission(
                    Tick(current.raw() + 1),
                    Tick(current.raw() + 1),
                    BodyEventDelta::zero(),
                    Some(&neural),
                    &phenotype,
                )
                .unwrap(),
        );
        let mut record =
            WorldOrganismRecord::new(id, entity, genome, phenotype, biology, Tick::ZERO).unwrap();
        record
            .stage_prediction_residual_pulse(
                alife_core::ExperienceSequenceId::new(id.raw()).unwrap(),
                7,
                activity,
                1.0,
            )
            .unwrap();
        world.register_organism_record(record).unwrap();
    }
    let before =
        ids.map(|id| serde_json::to_vec(world.organism_registry().get(id).unwrap()).unwrap());
    world.injected_tick_late_failure_after_first_organism = true;
    assert!(world.try_advance_tick().is_err());
    assert_eq!(world.tick(), current);
    for (index, id) in ids.into_iter().enumerate() {
        assert_eq!(
            serde_json::to_vec(world.organism_registry().get(id).unwrap()).unwrap(),
            before[index]
        );
    }
    world.injected_tick_late_failure_after_first_organism = false;
    world.try_advance_tick().unwrap();
    for (index, id) in ids.into_iter().enumerate() {
        assert_eq!(
            world.organism_registry().get(id).unwrap().biochemistry(),
            &expected[index]
        );
    }
    assert_ne!(
        expected[0].homeostasis.hormones.adrenaline,
        expected[1].homeostasis.hormones.adrenaline
    );
}
