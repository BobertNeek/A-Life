use alife_core::{
    ActionKind, BiochemistryState, BodyEventDelta, BrainCapacityClass, ChannelCommand, Confidence,
    CreatureGenome, DurationTicks, ExperienceSequenceId, FoundationGeneticIdentity, Intensity,
    MotorChannel, MotorCommandBundle, OrganismId, TeacherPerceptionChannel, Tick, Vec3f,
    WorldEntityId,
};
use alife_world::{HeadlessScenarioBuilder, WorldOrganismRecord};

fn record() -> WorldOrganismRecord {
    let genome = CreatureGenome::early_mammal_founder(
        0xA0A2_0001,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let biochemistry = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    WorldOrganismRecord::new(
        OrganismId(44),
        WorldEntityId(144),
        genome,
        phenotype,
        biochemistry,
        Tick::ZERO,
    )
    .unwrap()
}

#[test]
fn canonical_record_advances_subsystems_only_through_explicit_boundaries() {
    let mut organism = record();
    let initial = organism.state_graph().clone();

    organism
        .advance_biology(Tick(1), BodyEventDelta::zero())
        .unwrap();
    assert_eq!(
        organism.state_graph().body_biochemistry.revision,
        initial.body_biochemistry.revision + 1
    );
    assert_ne!(
        organism.state_graph().body_biochemistry.content_digest,
        initial.body_biochemistry.content_digest
    );
    assert_eq!(organism.state_graph().brain, initial.brain);
    assert_eq!(organism.state_graph().memory, initial.memory);
    assert_eq!(
        organism.state_graph().genetics_development,
        initial.genetics_development
    );

    let mut embodiment = organism.embodiment().clone();
    let sensor_lanes = embodiment.sensor_calibration().len();
    let effector_lanes = embodiment.effector_controllability().len();
    let body_schema_lanes = embodiment.body_schema().len();
    embodiment
        .replace_calibration(
            Tick(1),
            vec![0.1; sensor_lanes],
            vec![0.2; effector_lanes],
            vec![0.3; body_schema_lanes],
        )
        .unwrap();
    organism.replace_embodiment_state(embodiment).unwrap();
    assert_eq!(
        organism.state_graph().embodiment.revision,
        initial.embodiment.revision + 1
    );

    organism
        .seal_cognitive_subsystems(Tick(1), [11, 12, 13, 14], [21, 22, 23, 24])
        .unwrap();
    assert_eq!(
        organism.state_graph().brain.content_digest,
        [11, 12, 13, 14]
    );
    assert_eq!(
        organism.state_graph().memory.content_digest,
        [21, 22, 23, 24]
    );
    organism.validate_contract().unwrap();
}

#[test]
fn lifecycle_and_persistence_changes_advance_only_their_owned_reference() {
    let mut organism = record();
    let initial = organism.state_graph().clone();
    organism
        .seal_sleep_phase(alife_core::SleepPhase::EnteringSleep, 1, Tick::ZERO, 3)
        .unwrap();
    assert_eq!(
        organism.state_graph().lifecycle_persistence.revision,
        initial.lifecycle_persistence.revision + 1
    );
    assert_ne!(
        organism.state_graph().lifecycle_persistence.content_digest,
        initial.lifecycle_persistence.content_digest
    );
    assert_eq!(
        organism.state_graph().genetics_development,
        initial.genetics_development
    );
    assert_eq!(
        organism.state_graph().body_biochemistry,
        initial.body_biochemistry
    );
}

#[test]
fn embodiment_replacement_rejects_foreign_or_skipped_revisions() {
    let mut organism = record();
    let before = organism.clone();
    let mut embodiment = organism.embodiment().clone();
    let sensor_lanes = embodiment.sensor_calibration().len();
    let effector_lanes = embodiment.effector_controllability().len();
    let body_schema_lanes = embodiment.body_schema().len();
    embodiment
        .replace_calibration(
            Tick::ZERO,
            vec![0.1; sensor_lanes],
            vec![0.2; effector_lanes],
            vec![0.3; body_schema_lanes],
        )
        .unwrap();
    embodiment
        .replace_calibration(
            Tick::ZERO,
            vec![0.2; sensor_lanes],
            vec![0.2; effector_lanes],
            vec![0.3; body_schema_lanes],
        )
        .unwrap();
    assert!(organism.replace_embodiment_state(embodiment).is_err());
    assert_eq!(organism, before);
}

fn registered_world() -> (alife_world::HeadlessWorld, WorldEntityId) {
    let organism_id = OrganismId(44);
    let mut world = HeadlessScenarioBuilder::new(0xA0A2_00E1)
        .agent("organism", organism_id, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 1.0)
        .build()
        .unwrap();
    let entity_id = world.entity_id("organism").unwrap();
    let mut organism = record();
    organism = WorldOrganismRecord::new(
        organism_id,
        entity_id,
        organism.genome().clone(),
        organism.phenotype().clone(),
        *organism.biochemistry(),
        Tick::ZERO,
    )
    .unwrap();
    world
        .replace_organism_registry_exact([organism].into_iter())
        .unwrap();
    (world, entity_id)
}

#[test]
fn production_sensing_and_motor_execution_consume_embodiment_state() {
    let (baseline_world, entity_id) = registered_world();
    let mut calibrated_world = baseline_world.clone();
    let mut calibrated_record = calibrated_world
        .organism_registry()
        .get(OrganismId(44))
        .unwrap()
        .clone();
    let mut embodiment = calibrated_record.embodiment().clone();
    embodiment
        .replace_calibration(
            Tick::ZERO,
            vec![-1.0; embodiment.sensor_calibration().len()],
            vec![-1.0; embodiment.effector_controllability().len()],
            vec![-1.0; embodiment.body_schema().len()],
        )
        .unwrap();
    calibrated_record
        .replace_embodiment_state(embodiment)
        .unwrap();
    calibrated_world
        .replace_organism_registry_exact([calibrated_record].into_iter())
        .unwrap();

    let baseline_sensory = baseline_world
        .sensory_report(OrganismId(44), Tick::ZERO)
        .unwrap();
    let calibrated_sensory = calibrated_world
        .sensory_report(OrganismId(44), Tick::ZERO)
        .unwrap();
    assert!(
        calibrated_sensory.core_snapshot.channels.visual_affordance[0]
            < baseline_sensory.core_snapshot.channels.visual_affordance[0]
    );

    let channel = ChannelCommand::new(
        MotorChannel::Locomotion,
        ActionKind::Move.canonical_id(),
        None,
        Vec3f::new(1.0, 0.0, 0.0),
        Intensity::new(0.8).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(1.0).unwrap(),
        0,
    )
    .unwrap();
    let bundle = MotorCommandBundle::new(
        OrganismId(44),
        ExperienceSequenceId(1),
        Tick::ZERO,
        vec![channel],
    )
    .unwrap();
    let mut baseline_motor_world = baseline_world;
    let baseline_receipt = baseline_motor_world
        .apply_registered_motor_bundle(&bundle, entity_id)
        .unwrap();
    let calibrated_receipt = calibrated_world
        .apply_registered_motor_bundle(&bundle, entity_id)
        .unwrap();
    assert!(
        calibrated_receipt.joint.execution.displacement.x
            < baseline_receipt.joint.execution.displacement.x
    );
}

#[test]
fn teacher_signals_require_a_grounded_actor_and_enter_normal_sensing() {
    let learner = OrganismId(501);
    let teacher = OrganismId(502);
    let mut world = HeadlessScenarioBuilder::new(0xA0A2_00E2)
        .agent("learner", learner, Vec3f::ZERO)
        .social_agent("teacher", teacher, Vec3f::new(0.5, 0.0, 0.0), 0.75)
        .token("not-an-actor", Vec3f::new(0.25, 0.0, 0.0), 9)
        .build()
        .unwrap();
    assert!(world
        .grounded_teacher_actor(world.entity_id("not-an-actor").unwrap())
        .is_err());
    let actor = world
        .grounded_teacher_actor(world.entity_id("teacher").unwrap())
        .unwrap();
    actor
        .speak(
            &mut world,
            Some(learner),
            vec![alife_core::LanguageTokenId::new(77).unwrap()],
            TeacherPerceptionChannel::Hearing,
        )
        .unwrap();
    let sensory = world.sensory_report(learner, Tick::ZERO).unwrap();
    assert!(sensory
        .core_snapshot
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .any(|token| token.token_id == 77
            && token.teacher_channel == Some(TeacherPerceptionChannel::Hearing)));
}
