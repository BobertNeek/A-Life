use alife_core::{
    ActionCommand, ActionId, ActionKind, ActionProposal, ActionTarget, BrainScaleTier,
    BrainTickInput, BrainTickStatus, Confidence, CreatureMind, DurationTicks, Intensity,
    NormalizedScalar, OrganismId, PhysicalContactKind, ReferenceActionFailure, SignedValence,
    SleepPhase, Tick, Vec3f, WorldEntityId,
};
use alife_world::{
    HeadlessActionIds, HeadlessBrainHarness, HeadlessScenarioBuilder, HeadlessWorld,
    HeadlessWorldCommand, WorldObjectKind,
};

fn organism() -> OrganismId {
    OrganismId(17)
}

fn pos(x: f32, y: f32) -> Vec3f {
    Vec3f::new(x, y, 0.0)
}

fn command(
    action_id: ActionId,
    kind: ActionKind,
    target_entity: Option<WorldEntityId>,
    target_position: Option<Vec3f>,
) -> ActionCommand {
    ActionCommand::structured(
        organism(),
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

fn proposal(
    action_id: ActionId,
    kind: ActionKind,
    target_entity: Option<WorldEntityId>,
    target_position: Option<Vec3f>,
    score: f32,
) -> ActionProposal {
    ActionProposal::new(
        action_id,
        kind,
        score,
        Confidence::new(0.9).unwrap(),
        None,
        0b11,
        ActionTarget::new(target_entity, target_position),
        NormalizedScalar::new(0.8).unwrap(),
    )
    .unwrap()
}

fn world_with_food_and_hazard() -> HeadlessWorld {
    HeadlessScenarioBuilder::new(123)
        .agent("agent", organism(), pos(0.0, 0.0))
        .food("berry", pos(1.0, 0.0), 0.6)
        .hazard("thorn", pos(0.0, 1.0), 0.7)
        .obstacle("stone", pos(2.0, 0.0), 0.5)
        .token("word_food", pos(1.5, 0.0), 41)
        .build()
        .unwrap()
}

#[test]
fn sensory_gathering_is_bounded_ordered_and_maps_affordances_to_core_snapshot() {
    let world = world_with_food_and_hazard();

    let report = world.sensory_report(organism(), Tick::ZERO).unwrap();

    assert_eq!(report.visible_entities.len(), 4);
    assert_eq!(report.visible_entities[0].kind, WorldObjectKind::Food);
    assert!(report
        .core_snapshot
        .channels
        .nearby_affordances
        .contains(alife_core::AffordanceBits::FOOD));
    assert!(report
        .core_snapshot
        .channels
        .nearby_affordances
        .contains(alife_core::AffordanceBits::HAZARD));
    assert!(report.core_snapshot.channels.visual_affordance[0] > 0.0);
    assert!(report.core_snapshot.channels.visual_affordance[2] > 0.0);
    assert_eq!(
        report.core_snapshot.context_streams.vocal_tokens[0]
            .unwrap()
            .token_id,
        41
    );
}

#[test]
fn stable_world_entity_ids_are_replayable_for_same_seed_and_script() {
    let first = world_with_food_and_hazard();
    let second = world_with_food_and_hazard();

    assert_eq!(first.entity_id("agent"), second.entity_id("agent"));
    assert_eq!(first.entity_id("berry"), second.entity_id("berry"));
    assert_eq!(first.entity_id("thorn"), second.entity_id("thorn"));
    assert_eq!(first.entity_id("stone"), second.entity_id("stone"));
    assert_eq!(first.stable_signature(), second.stable_signature());
}

#[test]
fn seeded_random_builder_replays_same_world_state() {
    let first = HeadlessScenarioBuilder::new(9)
        .agent("agent", organism(), pos(0.0, 0.0))
        .random_food("seeded_food", 0.4)
        .random_hazard("seeded_hazard", 0.3)
        .build()
        .unwrap();
    let second = HeadlessScenarioBuilder::new(9)
        .agent("agent", organism(), pos(0.0, 0.0))
        .random_food("seeded_food", 0.4)
        .random_hazard("seeded_hazard", 0.3)
        .build()
        .unwrap();

    assert_eq!(first.stable_signature(), second.stable_signature());
}

#[test]
fn headless_world_tick_progression_is_explicit_and_deterministic() {
    let mut first = world_with_food_and_hazard();
    let mut second = world_with_food_and_hazard();

    assert_eq!(first.tick(), Tick::ZERO);
    assert_eq!(first.advance_tick(), Tick::new(1));
    assert_eq!(first.advance_tick(), Tick::new(2));
    second.advance_tick();
    second.advance_tick();
    assert_eq!(first.tick(), second.tick());
}

#[test]
fn action_execution_supports_move_inspect_eat_rest_and_idle() {
    let mut world = world_with_food_and_hazard();
    let berry = world.entity_id("berry").unwrap();

    let moved = world
        .apply_command(&command(
            ActionKind::Move.canonical_id(),
            ActionKind::Move,
            None,
            Some(pos(0.5, 0.0)),
        ))
        .unwrap();
    assert!(moved.execution.succeeded);
    assert_eq!(moved.execution.physical.contact, PhysicalContactKind::Moved);

    let inspected = world
        .apply_command(&command(
            ActionKind::Inspect.canonical_id(),
            ActionKind::Inspect,
            Some(berry),
            None,
        ))
        .unwrap();
    assert!(inspected.execution.succeeded);
    assert_eq!(
        inspected.observation.reward_valence,
        SignedValence::new(0.05).unwrap()
    );

    let eaten = world
        .apply_command(&HeadlessWorldCommand::eat(organism(), berry).unwrap())
        .unwrap();
    assert!(eaten.execution.succeeded);
    assert_eq!(
        eaten.execution.physical.contact,
        PhysicalContactKind::Consumed
    );
    assert!(eaten.observation.reward_valence.raw() > 0.5);
    assert!(world.entity(berry).unwrap().is_consumed());

    let rested = world
        .apply_command(&command(
            ActionKind::Rest.canonical_id(),
            ActionKind::Rest,
            None,
            None,
        ))
        .unwrap();
    assert!(rested.execution.succeeded);
    assert!(rested.observation.homeostatic_delta.drives.fatigue < 0.0);

    let idle = world
        .apply_command(&command(
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            None,
            None,
        ))
        .unwrap();
    assert!(idle.execution.succeeded);
}

#[test]
fn missing_affordance_and_invalid_target_failures_are_distinct() {
    let mut world = world_with_food_and_hazard();
    let stone = world.entity_id("stone").unwrap();

    let missing = world
        .apply_command(&HeadlessWorldCommand::eat(organism(), stone).unwrap())
        .unwrap();
    assert!(!missing.execution.succeeded);
    assert_eq!(
        missing.execution.failure,
        Some(ReferenceActionFailure::MissingAffordance)
    );
    assert!(missing.observation.contradiction_observed);

    let invalid = world
        .apply_command(&HeadlessWorldCommand::eat(organism(), WorldEntityId(999)).unwrap())
        .unwrap();
    assert!(!invalid.execution.succeeded);
    assert_eq!(
        invalid.execution.failure,
        Some(ReferenceActionFailure::ActionRejected)
    );
}

#[test]
fn food_reward_and_hazard_pain_are_measured_in_outcomes() {
    let mut world = world_with_food_and_hazard();
    let berry = world.entity_id("berry").unwrap();
    let thorn = world.entity_id("thorn").unwrap();

    let food = world
        .apply_command(&HeadlessWorldCommand::eat(organism(), berry).unwrap())
        .unwrap();
    assert!(food.observation.reward_valence.raw() > 0.5);
    assert!(food.observation.homeostatic_delta.drives.hunger < 0.0);
    assert!(food.observation.energy_delta.raw() > 0.0);

    let pain = world
        .apply_command(&command(
            HeadlessActionIds::APPROACH,
            ActionKind::Move,
            Some(thorn),
            None,
        ))
        .unwrap();
    assert!(pain.observation.reward_valence.raw() < 0.0);
    assert!(pain.observation.pain_delta.raw() > 0.0);
    assert!(pain.observation.homeostatic_delta.drives.fear > 0.0);
}

#[test]
fn action_execution_supports_approach_flee_grab_and_vocalize() {
    let mut world = world_with_food_and_hazard();
    let berry = world.entity_id("berry").unwrap();
    let thorn = world.entity_id("thorn").unwrap();

    let approached = world
        .apply_command(&command(
            HeadlessActionIds::APPROACH,
            ActionKind::Move,
            Some(berry),
            None,
        ))
        .unwrap();
    assert!(approached.execution.succeeded);
    assert_eq!(
        approached.execution.physical.contact,
        PhysicalContactKind::Moved
    );

    let before_flee = world
        .entity(world.entity_id("agent").unwrap())
        .unwrap()
        .position;
    let fled = world
        .apply_command(&command(
            HeadlessActionIds::FLEE,
            ActionKind::Move,
            Some(thorn),
            None,
        ))
        .unwrap();
    assert!(fled.execution.succeeded);
    assert_ne!(
        before_flee,
        world
            .entity(world.entity_id("agent").unwrap())
            .unwrap()
            .position
    );

    let grabbed = world
        .apply_command(&command(
            HeadlessActionIds::GRAB,
            ActionKind::Hold,
            Some(berry),
            None,
        ))
        .unwrap();
    assert!(grabbed.execution.succeeded);
    assert_eq!(world.entity(berry).unwrap().carried_by, Some(organism()));

    let vocalized = world
        .apply_command(&command(
            ActionKind::Vocalize.canonical_id(),
            ActionKind::Vocalize,
            None,
            None,
        ))
        .unwrap();
    assert!(vocalized.execution.succeeded);
    assert!(vocalized.observation.reward_valence.raw() > 0.0);
}

#[test]
fn headless_harness_collects_sealed_patches_and_triggers_p16_sleep_on_rest() {
    let world = HeadlessScenarioBuilder::new(5)
        .agent("agent", organism(), pos(0.0, 0.0))
        .build()
        .unwrap();
    let mut harness = HeadlessBrainHarness::new(world);
    let mut mind =
        CreatureMind::scaffold(organism(), BrainScaleTier::Nano512, 99, Tick::ZERO).unwrap();

    let first = harness.tick_mind(
        &mut mind,
        BrainTickInput::new(
            Tick::ZERO,
            vec![proposal(
                ActionKind::Rest.canonical_id(),
                ActionKind::Rest,
                None,
                None,
                0.9,
            )],
        )
        .with_pack_experience(true),
    );

    assert_eq!(first.brain.status, BrainTickStatus::Normal);
    assert_eq!(harness.telemetry().sealed_patches.len(), 1);
    assert_eq!(harness.world().tick(), Tick::new(1));
    assert_eq!(mind.sleep_state().phase, SleepPhase::ForcedRecoverySleep);
    assert!(first.sleep_transition.is_some());
}

#[test]
fn multi_tick_cpu_reference_brain_stepping_is_headless_and_deterministic() {
    let berry_script = |seed| {
        HeadlessScenarioBuilder::new(seed)
            .agent("agent", organism(), pos(0.0, 0.0))
            .food("berry", pos(1.0, 0.0), 0.5)
            .build()
            .unwrap()
    };
    let mut first = HeadlessBrainHarness::new(berry_script(44));
    let mut second = HeadlessBrainHarness::new(berry_script(44));
    let mut first_mind =
        CreatureMind::scaffold(organism(), BrainScaleTier::Nano512, 7, Tick::ZERO).unwrap();
    let mut second_mind =
        CreatureMind::scaffold(organism(), BrainScaleTier::Nano512, 7, Tick::ZERO).unwrap();

    for tick in 0..2 {
        let target = first.world().entity_id("berry").unwrap();
        let input = BrainTickInput::new(
            Tick::new(tick),
            vec![proposal(
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                Some(target),
                None,
                0.9,
            )],
        )
        .with_pack_experience(true);
        let a = first.tick_mind(&mut first_mind, input.clone());
        let b = second.tick_mind(&mut second_mind, input);

        assert_eq!(a.brain.status, b.brain.status);
        assert_eq!(a.brain.selected_action, b.brain.selected_action);
        assert_eq!(
            first.world().stable_signature(),
            second.world().stable_signature()
        );
    }

    assert_eq!(first.telemetry().sealed_patches.len(), 2);
    assert_eq!(second.telemetry().sealed_patches.len(), 2);
    assert_eq!(first.world().tick(), Tick::new(2));
    assert_eq!(second.world().tick(), Tick::new(2));
}

#[test]
fn alife_world_headless_harness_does_not_require_bevy_or_gpu() {
    let manifest = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("crate manifest should be readable");
    for forbidden in [
        concat!("be", "vy"),
        concat!("av", "ian"),
        concat!("wg", "pu"),
    ] {
        assert!(
            !manifest.to_ascii_lowercase().contains(forbidden),
            "alife_world manifest must not depend on {forbidden}"
        );
    }

    let source = include_str!("../src/lib.rs");
    for forbidden in [
        concat!("Render", "Device"),
        concat!("Render", "Queue"),
        concat!("Ent", "ity<"),
    ] {
        assert!(
            !source.contains(forbidden),
            "headless world source must not embed engine type {forbidden}"
        );
    }
}
