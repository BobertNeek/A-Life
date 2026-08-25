use alife_core::{
    ActionCommand, ActionId, ActionKind, ActionTarget, BiochemistryState, BodyEventDelta,
    BrainCapacityClass, ChannelCommand, Confidence, CreatureGenome, DurationTicks,
    ExperienceSequenceId, FoundationGeneticIdentity, Intensity, MotorChannel, MotorCommandBundle,
    OrganismId, PhysicalContactKind, SpeechActKind, SpeechMotorPayload, Tick, Validate, Vec3f,
    WorldEntityId,
};
use alife_world::{
    HeadlessActionIds, HeadlessScenarioBuilder, HeadlessWorld, HeadlessWorldCommand,
    WorldEditorSpawnSpec, WorldObjectKind, WorldOrganismRecord,
};

const ORGANISM_ID: OrganismId = OrganismId(7);

fn record(world_entity_id: WorldEntityId) -> WorldOrganismRecord {
    let genome = CreatureGenome::early_mammal_founder(
        0xE10_32A0,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let biochemistry = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
    WorldOrganismRecord::new(
        ORGANISM_ID,
        world_entity_id,
        genome,
        phenotype,
        biochemistry,
        Tick::ZERO,
    )
    .unwrap()
}

fn move_command(target: Vec3f) -> ActionCommand {
    structured_command(
        ActionKind::Move.canonical_id(),
        ActionKind::Move,
        None,
        Some(target),
    )
}

fn inspect_command(target: WorldEntityId) -> ActionCommand {
    structured_command(
        ActionKind::Inspect.canonical_id(),
        ActionKind::Inspect,
        Some(target),
        None,
    )
}

fn grab_command(target: WorldEntityId) -> ActionCommand {
    structured_command(
        ActionKind::Hold.canonical_id(),
        ActionKind::Hold,
        Some(target),
        None,
    )
}

fn structured_command(
    action_id: ActionId,
    kind: ActionKind,
    target_entity: Option<WorldEntityId>,
    target_position: Option<Vec3f>,
) -> ActionCommand {
    ActionCommand::structured(
        ORGANISM_ID,
        action_id,
        kind,
        ActionTarget::new(target_entity, target_position),
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        Confidence::new(0.9).unwrap(),
        0,
        None,
        None,
        None,
    )
    .unwrap()
}

fn world_with_agent() -> (HeadlessWorld, WorldEntityId) {
    let world = HeadlessScenarioBuilder::new(32_001)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .build()
        .unwrap();
    let agent = world.entity_id("agent").unwrap();
    (world, agent)
}

fn register(world: &mut HeadlessWorld, agent: WorldEntityId) {
    world.register_organism_record(record(agent)).unwrap();
}

fn zero_only_event(event: BodyEventDelta, energy: f32) {
    assert_eq!(event.energy, energy);
    assert_eq!(event.damage, 0.0);
    assert_eq!(event.temperature_stress, 0.0);
    assert_eq!(event.nutrition, 0.0);
    assert_eq!(event.social_contact, 0.0);
    assert_eq!(event.sleep_recovery, 0.0);
    assert_eq!(event.mating_opportunity, 0.0);
}

#[test]
fn registered_move_transaction_advances_authoritative_biology_once() {
    let (mut world, agent) = world_with_agent();
    register(&mut world, agent);
    let before = *world
        .organism_registry()
        .get(ORGANISM_ID)
        .unwrap()
        .biochemistry();

    let receipt = world
        .apply_registered_command(&move_command(Vec3f::new(0.5, 0.0, 0.0)), agent, Tick(1))
        .unwrap();

    assert!(receipt.action_result.execution.succeeded);
    assert_eq!(receipt.outcome_tick, Tick(1));
    assert_eq!(receipt.biology_before, before);
    assert_eq!(receipt.biology_after.tick, Tick(1));
    assert_eq!(
        receipt.biology_after,
        before
            .advance(
                Tick(1),
                receipt.action_result.body_event,
                world
                    .organism_registry()
                    .get(ORGANISM_ID)
                    .unwrap()
                    .phenotype(),
            )
            .unwrap()
    );
    assert_eq!(
        world
            .organism_registry()
            .get(ORGANISM_ID)
            .unwrap()
            .biochemistry(),
        &receipt.biology_after
    );
    assert_eq!(
        world.entity(agent).unwrap().position,
        Vec3f::new(0.5, 0.0, 0.0)
    );
    assert_eq!(receipt.action_result.body_event.nutrition, 0.0);
    assert_eq!(receipt.action_result.body_event.damage, 0.0);
    assert_eq!(receipt.action_result.body_event.energy, -0.04);
    zero_only_event(receipt.action_result.body_event, -0.04);
}

#[test]
fn registered_food_transaction_reports_actual_nutrition() {
    let mut world = HeadlessScenarioBuilder::new(32_002)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.6)
        .build()
        .unwrap();
    let agent = world.entity_id("agent").unwrap();
    let food = world.entity_id("food").unwrap();
    register(&mut world, agent);

    let receipt = world
        .apply_registered_command(
            &HeadlessWorldCommand::eat(ORGANISM_ID, food).unwrap(),
            agent,
            Tick(1),
        )
        .unwrap();

    assert!(receipt.action_result.execution.succeeded);
    assert_eq!(receipt.action_result.body_event.nutrition, 0.6);
    assert_eq!(receipt.action_result.body_event.damage, 0.0);
    assert_eq!(receipt.action_result.body_event.energy, 0.6_f32 * 0.5);
    assert_eq!(receipt.action_result.observation.reward_valence.raw(), 0.0);
    assert_eq!(receipt.action_result.body_event.temperature_stress, 0.0);
    assert_eq!(receipt.action_result.body_event.social_contact, 0.0);
    assert_eq!(receipt.action_result.body_event.sleep_recovery, 0.0);
    assert_eq!(receipt.action_result.body_event.mating_opportunity, 0.0);
    assert_eq!(receipt.biology_after.tick, Tick(1));
    assert!(world.entity(food).unwrap().is_consumed());
}

#[test]
fn registered_hazard_contact_reports_measured_damage() {
    let mut world = HeadlessScenarioBuilder::new(32_003)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .hazard("hazard", Vec3f::new(1.0, 0.0, 0.0), 0.7)
        .build()
        .unwrap();
    let agent = world.entity_id("agent").unwrap();
    let hazard = world.entity_id("hazard").unwrap();
    register(&mut world, agent);

    let receipt = world
        .apply_registered_command(
            &HeadlessWorldCommand::approach(ORGANISM_ID, hazard).unwrap(),
            agent,
            Tick(1),
        )
        .unwrap();

    assert!(receipt.action_result.execution.succeeded);
    assert_eq!(receipt.action_result.body_event.damage, 0.7);
    assert_eq!(receipt.action_result.body_event.energy, -0.08);
    assert_eq!(receipt.action_result.observation.reward_valence.raw(), 0.0);
    assert_eq!(receipt.action_result.body_event.nutrition, 0.0);
    assert_eq!(receipt.action_result.body_event.temperature_stress, 0.0);
    assert_eq!(receipt.action_result.body_event.social_contact, 0.0);
    assert_eq!(receipt.action_result.body_event.sleep_recovery, 0.0);
    assert_eq!(receipt.action_result.body_event.mating_opportunity, 0.0);
    assert_eq!(receipt.biology_after.tick, Tick(1));
}

#[test]
fn stored_object_radius_bounds_grounded_contact_and_movement_collision() {
    let world_for_radius = |radius| {
        let mut world = HeadlessScenarioBuilder::new(32_030)
            .agent("agent", ORGANISM_ID, Vec3f::ZERO)
            .build()
            .unwrap();
        let agent = world.entity_id("agent").unwrap();
        let hazard = world
            .editor_spawn_object(WorldEditorSpawnSpec {
                label: "radius-hazard".to_string(),
                kind: WorldObjectKind::Hazard,
                organism_id: None,
                position: Vec3f::new(1.0, 0.0, 0.0),
                nutrition: 0.0,
                hazard_pain: 0.2,
                radius,
                token_id: None,
            })
            .unwrap();
        register(&mut world, agent);
        (world, agent, hazard)
    };

    let (mut noncontact, noncontact_agent, noncontact_hazard) = world_for_radius(0.5);
    let noncontact_receipt = noncontact
        .apply_registered_command(
            &move_command(Vec3f::new(0.4, 0.0, 0.0)),
            noncontact_agent,
            Tick(1),
        )
        .unwrap();
    assert_eq!(
        noncontact_receipt.action_result.execution.physical.contact,
        alife_core::PhysicalContactKind::Moved
    );
    assert!(
        !noncontact
            .physical_observation_snapshot(ORGANISM_ID, Tick(1))
            .unwrap()
            .visible
            .iter()
            .find(|object| object.transport_entity == noncontact_hazard)
            .unwrap()
            .contact
    );

    let (mut contact, contact_agent, contact_hazard) = world_for_radius(0.8);
    let contact_receipt = contact
        .apply_registered_command(
            &move_command(Vec3f::new(0.4, 0.0, 0.0)),
            contact_agent,
            Tick(1),
        )
        .unwrap();
    assert_eq!(
        contact_receipt.action_result.execution.physical.contact,
        alife_core::PhysicalContactKind::Collision
    );
    assert!(
        contact
            .physical_observation_snapshot(ORGANISM_ID, Tick(1))
            .unwrap()
            .visible
            .iter()
            .find(|object| object.transport_entity == contact_hazard)
            .unwrap()
            .contact
    );
}

#[test]
fn incidental_hazard_contact_preserves_factorized_action_outcome() {
    let mut world = HeadlessScenarioBuilder::new(32_031)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .food("food", Vec3f::ZERO, 0.6)
        .hazard("hazard", Vec3f::ZERO, 0.2)
        .build()
        .unwrap();
    let agent = world.entity_id("agent").unwrap();
    let food = world.entity_id("food").unwrap();
    register(&mut world, agent);
    let channel = ChannelCommand::new(
        MotorChannel::Manipulation,
        HeadlessActionIds::EAT,
        Some(ActionTarget::new(Some(food), None)),
        Vec3f::ZERO,
        Intensity::new(1.0).unwrap(),
        DurationTicks::new(1),
        0.0,
        Confidence::new(1.0).unwrap(),
        0,
    )
    .unwrap();
    let bundle = MotorCommandBundle::new(
        ORGANISM_ID,
        ExperienceSequenceId::new(1).unwrap(),
        Tick::ZERO,
        vec![channel],
    )
    .unwrap();

    let receipt = world.apply_registered_motor_bundle(&bundle, agent).unwrap();

    assert!(receipt.succeeded);
    assert_eq!(
        receipt.joint.execution.contact,
        PhysicalContactKind::Consumed
    );
    assert_eq!(receipt.joint.execution.target_entity, Some(food));
    assert_eq!(receipt.body_event.damage, 0.2);
    assert_eq!(receipt.body_event.nutrition, 0.6);
}

#[test]
fn cognitive_energy_setter_preserves_routed_body_projection() {
    let genome = CreatureGenome::early_mammal_founder(
        0xE10_3200,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    let mut biology = BiochemistryState::new(&phenotype, Tick::ZERO)
        .unwrap()
        .advance(
            Tick(1),
            BodyEventDelta {
                damage: 0.2,
                ..BodyEventDelta::zero()
            },
            &phenotype,
        )
        .unwrap();

    biology
        .body
        .set_energy((biology.body.energy - 0.123_456_7).max(0.0))
        .unwrap();

    biology.body.validate_contract().unwrap();
}

#[test]
fn registered_hazard_and_agent_contact_preserves_hazard_observation_and_social_event() {
    let mut world = HeadlessScenarioBuilder::new(32_004)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .hazard("hazard", Vec3f::new(1.0, 0.0, 0.0), 0.7)
        .social_agent("other", OrganismId(8), Vec3f::new(1.0, 0.0, 0.0), -1.4)
        .build()
        .unwrap();
    let agent = world.entity_id("agent").unwrap();
    let hazard = world.entity_id("hazard").unwrap();
    register(&mut world, agent);

    let receipt = world
        .apply_registered_command(&move_command(Vec3f::new(1.0, 0.0, 0.0)), agent, Tick(1))
        .unwrap();

    assert!(receipt.action_result.execution.succeeded);
    assert_eq!(
        receipt.action_result.execution.physical.contact,
        alife_core::PhysicalContactKind::Collision
    );
    assert_eq!(
        receipt.action_result.execution.physical.target_entity,
        Some(hazard)
    );
    assert_eq!(receipt.action_result.observation.reward_valence.raw(), 0.0);
    assert_eq!(receipt.action_result.observation.pain_delta.raw(), 0.7);
    assert_eq!(receipt.action_result.body_event.energy, -0.08);
    assert_eq!(receipt.action_result.body_event.damage, 0.7);
    assert_eq!(receipt.action_result.body_event.social_contact, 1.0);
    assert_eq!(receipt.action_result.body_event.temperature_stress, 0.0);
    assert_eq!(receipt.action_result.body_event.nutrition, 0.0);
    assert_eq!(receipt.action_result.body_event.sleep_recovery, 0.0);
    assert_eq!(receipt.action_result.body_event.mating_opportunity, 0.0);
}

#[test]
fn registered_approach_contact_reports_social_affinity_magnitude() {
    let mut world = HeadlessScenarioBuilder::new(32_006)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .social_agent("other", OrganismId(8), Vec3f::new(0.5, 0.0, 0.0), -0.6)
        .build()
        .unwrap();
    let agent = world.entity_id("agent").unwrap();
    let other = world.entity_id("other").unwrap();
    register(&mut world, agent);

    let receipt = world
        .apply_registered_command(
            &HeadlessWorldCommand::approach(ORGANISM_ID, other).unwrap(),
            agent,
            Tick(1),
        )
        .unwrap();

    assert_eq!(receipt.action_result.body_event.social_contact, 0.6);
    assert_eq!(receipt.action_result.body_event.energy, -0.04);
    assert_eq!(receipt.action_result.observation.reward_valence.raw(), 0.0);
    assert_eq!(receipt.action_result.body_event.damage, 0.0);
    assert_eq!(receipt.action_result.body_event.temperature_stress, 0.0);
    assert_eq!(receipt.action_result.body_event.nutrition, 0.0);
    assert_eq!(receipt.action_result.body_event.sleep_recovery, 0.0);
    assert_eq!(receipt.action_result.body_event.mating_opportunity, 0.0);
}

#[test]
fn registered_specialized_events_preserve_physical_and_physiological_fields() {
    let (mut rest_world, rest_agent) = world_with_agent();
    register(&mut rest_world, rest_agent);
    let rest = rest_world
        .apply_registered_command(
            &HeadlessWorldCommand::rest(ORGANISM_ID).unwrap(),
            rest_agent,
            Tick(1),
        )
        .unwrap();
    assert_eq!(rest.action_result.body_event.energy, 0.08);
    assert_eq!(rest.action_result.observation.reward_valence.raw(), 0.0);
    assert_eq!(rest.action_result.body_event.damage, 0.0);
    assert_eq!(rest.action_result.body_event.temperature_stress, 0.0);
    assert_eq!(rest.action_result.body_event.nutrition, 0.0);
    assert_eq!(rest.action_result.body_event.social_contact, 0.0);
    assert_eq!(rest.action_result.body_event.sleep_recovery, 1.0);
    assert_eq!(rest.action_result.body_event.mating_opportunity, 0.0);

    let mut hazard_world = HeadlessScenarioBuilder::new(32_007)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .hazard("hazard", Vec3f::new(1.0, 0.0, 0.0), 0.7)
        .build()
        .unwrap();
    let hazard_agent = hazard_world.entity_id("agent").unwrap();
    let hazard = hazard_world.entity_id("hazard").unwrap();
    register(&mut hazard_world, hazard_agent);
    let hazard_receipt = hazard_world
        .apply_registered_command(
            &HeadlessWorldCommand::approach(ORGANISM_ID, hazard).unwrap(),
            hazard_agent,
            Tick(1),
        )
        .unwrap();
    assert_eq!(hazard_receipt.action_result.body_event.energy, -0.08);
    assert_eq!(
        hazard_receipt
            .action_result
            .observation
            .reward_valence
            .raw(),
        0.0
    );
    assert_eq!(hazard_receipt.action_result.body_event.damage, 0.7);
    assert_eq!(
        hazard_receipt.action_result.body_event.temperature_stress,
        0.0
    );
    assert_eq!(hazard_receipt.action_result.body_event.nutrition, 0.0);
    assert_eq!(hazard_receipt.action_result.body_event.social_contact, 0.0);
    assert_eq!(hazard_receipt.action_result.body_event.sleep_recovery, 0.0);
    assert_eq!(
        hazard_receipt.action_result.body_event.mating_opportunity,
        0.0
    );

    for (seed, affinity, expected_energy) in
        [(32_008, 0.6_f32, -0.02_f32), (32_009, -0.6_f32, -0.04_f32)]
    {
        let mut social_world = HeadlessScenarioBuilder::new(seed)
            .agent("agent", ORGANISM_ID, Vec3f::ZERO)
            .social_agent("other", OrganismId(8), Vec3f::new(0.5, 0.0, 0.0), affinity)
            .build()
            .unwrap();
        let social_agent = social_world.entity_id("agent").unwrap();
        register(&mut social_world, social_agent);
        let social = social_world
            .apply_registered_command(
                &move_command(Vec3f::new(0.5, 0.0, 0.0)),
                social_agent,
                Tick(1),
            )
            .unwrap();
        assert_eq!(social.action_result.body_event.energy, expected_energy);
        assert_eq!(social.action_result.observation.reward_valence.raw(), 0.0);
        assert_eq!(
            social.action_result.body_event.social_contact,
            affinity.abs()
        );
        assert_eq!(social.action_result.body_event.damage, 0.0);
        assert_eq!(social.action_result.body_event.temperature_stress, 0.0);
        assert_eq!(social.action_result.body_event.nutrition, 0.0);
        assert_eq!(social.action_result.body_event.sleep_recovery, 0.0);
        assert_eq!(social.action_result.body_event.mating_opportunity, 0.0);
    }
}

#[test]
fn registered_zero_only_event_profiles_keep_unmodeled_fields_zero() {
    let mut failed_world = HeadlessScenarioBuilder::new(32_010)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .hazard("hazard", Vec3f::new(1.0, 0.0, 0.0), 0.7)
        .build()
        .unwrap();
    let failed_agent = failed_world.entity_id("agent").unwrap();
    let hazard = failed_world.entity_id("hazard").unwrap();
    register(&mut failed_world, failed_agent);
    let failed = failed_world
        .apply_registered_command(
            &HeadlessWorldCommand::eat(ORGANISM_ID, hazard).unwrap(),
            failed_agent,
            Tick(1),
        )
        .unwrap();
    assert!(!failed.action_result.execution.succeeded);
    zero_only_event(failed.action_result.body_event, -0.02);
    assert_eq!(failed.action_result.observation.reward_valence.raw(), 0.0);

    let mut blocked_world = HeadlessScenarioBuilder::new(32_011)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .obstacle("obstacle", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let blocked_agent = blocked_world.entity_id("agent").unwrap();
    register(&mut blocked_world, blocked_agent);
    let blocked = blocked_world
        .apply_registered_command(
            &move_command(Vec3f::new(1.0, 0.0, 0.0)),
            blocked_agent,
            Tick(1),
        )
        .unwrap();
    assert!(!blocked.action_result.execution.succeeded);
    zero_only_event(blocked.action_result.body_event, -0.03);
    assert_eq!(blocked.action_result.observation.reward_valence.raw(), 0.0);

    let (mut invalid_world, invalid_agent) = world_with_agent();
    register(&mut invalid_world, invalid_agent);
    let invalid = invalid_world
        .apply_registered_command(
            &HeadlessWorldCommand::eat(ORGANISM_ID, WorldEntityId(999)).unwrap(),
            invalid_agent,
            Tick(1),
        )
        .unwrap();
    assert!(!invalid.action_result.execution.succeeded);
    zero_only_event(invalid.action_result.body_event, -0.01);
    assert_eq!(invalid.action_result.observation.reward_valence.raw(), 0.0);

    let (mut idle_world, idle_agent) = world_with_agent();
    register(&mut idle_world, idle_agent);
    let idle = idle_world
        .apply_registered_command(
            &HeadlessWorldCommand::idle(ORGANISM_ID).unwrap(),
            idle_agent,
            Tick(1),
        )
        .unwrap();
    zero_only_event(idle.action_result.body_event, -0.01);

    let mut inspect_world = HeadlessScenarioBuilder::new(32_012)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let inspect_agent = inspect_world.entity_id("agent").unwrap();
    let inspect_target = inspect_world.entity_id("food").unwrap();
    register(&mut inspect_world, inspect_agent);
    let inspect = inspect_world
        .apply_registered_command(&inspect_command(inspect_target), inspect_agent, Tick(1))
        .unwrap();
    zero_only_event(inspect.action_result.body_event, -0.01);

    let mut grab_world = HeadlessScenarioBuilder::new(32_013)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let grab_agent = grab_world.entity_id("agent").unwrap();
    let grab_target = grab_world.entity_id("food").unwrap();
    register(&mut grab_world, grab_agent);
    let grab = grab_world
        .apply_registered_command(&grab_command(grab_target), grab_agent, Tick(1))
        .unwrap();
    zero_only_event(grab.action_result.body_event, -0.03);
}

#[test]
fn registered_neural_command_seals_gpu_speech_with_biology() {
    let (mut world, agent) = world_with_agent();
    register(&mut world, agent);
    let command = HeadlessWorldCommand::vocalize(ORGANISM_ID).unwrap();
    let payload = SpeechMotorPayload::try_new(
        SpeechActKind::Declare,
        vec![alife_core::LanguageTokenId::new(9).unwrap()],
        Confidence::new(0.9).unwrap(),
    )
    .unwrap();

    let receipt = world
        .apply_registered_neural_command(&command, agent, Tick(1), Some(payload), false)
        .unwrap();

    assert!(receipt.action_result.execution.succeeded);
    assert_eq!(
        receipt
            .action_result
            .emitted_utterance
            .as_ref()
            .unwrap()
            .tokens
            .len(),
        1
    );
    assert_eq!(receipt.biology_after.tick, Tick(1));
    zero_only_event(receipt.action_result.body_event, -0.01);
}

#[test]
fn registered_transaction_rejects_wrong_tick_without_mutation() {
    let (mut world, agent) = world_with_agent();
    register(&mut world, agent);
    let before_signature = world.canonical_signature_digest().unwrap();
    let before_record = world.organism_registry().get(ORGANISM_ID).unwrap().clone();

    assert!(world
        .apply_registered_command(
            &HeadlessWorldCommand::idle(ORGANISM_ID).unwrap(),
            agent,
            Tick(2)
        )
        .is_err());

    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        before_signature
    );
    assert_eq!(
        world.organism_registry().get(ORGANISM_ID),
        Some(&before_record)
    );
}

#[test]
fn registered_transaction_rejects_mismatched_identity_without_mutation() {
    let mut world = HeadlessScenarioBuilder::new(32_004)
        .agent("agent", ORGANISM_ID, Vec3f::ZERO)
        .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.5)
        .build()
        .unwrap();
    let agent = world.entity_id("agent").unwrap();
    let food = world.entity_id("food").unwrap();
    register(&mut world, agent);
    let before_signature = world.canonical_signature_digest().unwrap();
    let before_record = world.organism_registry().get(ORGANISM_ID).unwrap().clone();

    assert!(world
        .apply_registered_command(
            &HeadlessWorldCommand::idle(ORGANISM_ID).unwrap(),
            food,
            Tick(1)
        )
        .is_err());

    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        before_signature
    );
    assert_eq!(
        world.organism_registry().get(ORGANISM_ID),
        Some(&before_record)
    );
}

#[test]
fn registered_transaction_rejects_missing_record_without_mutation() {
    let (mut world, agent) = world_with_agent();
    let before_signature = world.canonical_signature_digest().unwrap();

    assert!(world
        .apply_registered_command(
            &HeadlessWorldCommand::idle(ORGANISM_ID).unwrap(),
            agent,
            Tick(1)
        )
        .is_err());

    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        before_signature
    );
    assert!(world.organism_registry().is_empty());
}

#[test]
fn registered_transaction_rejects_dead_record_without_mutation() {
    let (mut world, agent) = world_with_agent();
    let mut dead = record(agent);
    dead.mark_dead(Tick::ZERO).unwrap();
    world.register_organism_record(dead).unwrap();
    let before_signature = world.canonical_signature_digest().unwrap();
    let before_record = world.organism_registry().get(ORGANISM_ID).unwrap().clone();

    assert!(world
        .apply_registered_command(
            &HeadlessWorldCommand::idle(ORGANISM_ID).unwrap(),
            agent,
            Tick(1)
        )
        .is_err());

    assert_eq!(
        world.canonical_signature_digest().unwrap(),
        before_signature
    );
    assert_eq!(
        world.organism_registry().get(ORGANISM_ID),
        Some(&before_record)
    );
}
