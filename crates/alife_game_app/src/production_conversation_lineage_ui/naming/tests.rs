//! CPU state, real raw input, world naming and addressing contracts. No GPU or OS input.
use super::*;
use alife_core::{BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity};
use alife_world::{HeadlessScenarioBuilder, StableVoxelObjectRef, WorldOrganismRecord};
use bevy::{a11y::AccessibilityNode, input::keyboard::Key, prelude::*};

fn world() -> HeadlessWorld {
    let mut world = HeadlessScenarioBuilder::new(21)
        .agent("founder", OrganismId(1), Vec3f::ZERO)
        .agent("Élodie", OrganismId(2), Vec3f::new(3.0, 0.0, 0.0))
        .build()
        .unwrap();
    for (label, id) in [("founder", OrganismId(1)), ("Élodie", OrganismId(2))] {
        let genome = CreatureGenome::early_mammal_founder(
            21,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let chemistry =
            alife_core::BiochemistryState::new_with_age(&phenotype, Tick::ZERO, Tick::ZERO)
                .unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::new(
                    id,
                    world.entity_id(label).unwrap(),
                    genome,
                    phenotype,
                    chemistry,
                    Tick::ZERO,
                )
                .unwrap(),
            )
            .unwrap();
    }
    world
}
fn draft(world: &HeadlessWorld, opening: Option<bool>) -> Draft {
    Draft {
        stable_id: world.entity_id("founder").unwrap(),
        organism_id: OrganismId(1),
        original_name: "founder".into(),
        runtime_identity: SpeechRuntimeIdentity {
            authority: crate::gpu_live_runtime::LiveRuntimeSaveAuthorityView {
                save_id: "synthetic-naming".into(),
                deterministic_seed: 21,
                sensor_profile: SensorProfile::GroundedObjectSlotsV1,
                organism_ids: vec![OrganismId(1), OrganismId(2)],
            },
            started_at_tick: Tick::ZERO,
        },
        opening,
        error: None,
    }
}
fn state(world: &HeadlessWorld, opening: Option<bool>) -> ProductionConversationLineageUiState {
    let mut state = super::super::speech_completion_state_tests::state();
    open(&mut state, draft(world, opening));
    state
}
fn key(code: KeyCode, text: Option<&str>, repeat: bool) -> KeyboardInput {
    KeyboardInput {
        key_code: code,
        logical_key: Key::Character(text.unwrap_or_default().into()),
        state: ButtonState::Pressed,
        text: text.map(Into::into),
        repeat,
        window: Entity::PLACEHOLDER,
    }
}
fn selected(id: WorldEntityId) -> Fvr03ProductionVoxelSelectionResource {
    Fvr03ProductionVoxelSelectionResource {
        hovered: None,
        selected: Some(StableVoxelObjectRef {
            kind: StableVoxelRefKind::Creature,
            stable_id: Some(id),
            chunk: alife_world::VoxelChunkCoord::new(0, 0),
            tile: None,
        }),
    }
}

#[test]
fn opening_order_repeats_unicode_and_confirm_are_captured_without_speech() {
    let world = world();
    let mut state = state(&world, Some(true));
    let keys = ButtonInput::default();
    assert!(!edit(
        &mut state,
        &keys,
        [
            key(KeyCode::KeyC, Some("shortcut"), false),
            key(KeyCode::F2, None, true),
            key(KeyCode::F2, None, false),
            key(KeyCode::KeyC, Some("é猫"), false),
            key(KeyCode::Enter, Some("\r"), false)
        ]
        .iter(),
        None
    ));
    assert_eq!(state.input, "é猫");
    assert!(state.blocks_world_shortcuts());
    assert!(!edit(
        &mut state,
        &keys,
        [
            key(KeyCode::Enter, None, true),
            key(KeyCode::Backspace, None, true),
            key(KeyCode::KeyC, Some("🐾"), true)
        ]
        .iter(),
        None
    ));
    assert_eq!(state.input, "é🐾");
    assert!(edit(
        &mut state,
        &keys,
        [
            key(KeyCode::Enter, None, false),
            key(KeyCode::KeyC, Some("must not leak"), false),
            key(KeyCode::Enter, None, false)
        ]
        .iter(),
        None
    ));
    assert_eq!(state.input, "é🐾");
    let mut world = world;
    let result = world.rename_agent(state.naming.as_ref().unwrap().stable_id, &state.input);
    complete(&mut state, result);
    assert!(state.naming.is_none() && !state.input_open && state.input.is_empty());
    assert!(
        world.audible_utterances().is_empty(),
        "confirming a name emits no speech"
    );
    assert_eq!(world.entity_id("é🐾"), Some(WorldEntityId(1)));
}

#[test]
fn pointer_opening_cancel_and_invalid_names_keep_world_atomic_and_draft_editable() {
    let mut world = world();
    let id = world.entity_id("founder").unwrap();
    let before = world.canonical_signature_digest().unwrap();
    let mut state = state(&world, Some(false));
    let keys = ButtonInput::default();
    assert!(!edit(
        &mut state,
        &keys,
        [key(KeyCode::KeyC, Some("opening click text"), false)].iter(),
        Some(composer::Control::Send)
    ));
    assert!(state.input.is_empty());
    for name in ["", "  ", "ÉLODIE", &"猫".repeat(33)] {
        state.input = name.into();
        assert!(edit(
            &mut state,
            &keys,
            [].iter(),
            Some(composer::Control::Send)
        ));
        let result = world.rename_agent(id, &state.input);
        complete(&mut state, result);
        assert!(state.input_open && state.naming.as_ref().unwrap().error.is_some());
        assert_eq!(state.input, name);
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
    }
    edit(
        &mut state,
        &keys,
        [key(KeyCode::KeyC, Some("new"), false)].iter(),
        None,
    );
    assert!(state.naming.as_ref().unwrap().error.is_none());
    let mut keys = keys;
    keys.press(KeyCode::Escape);
    assert!(!edit(
        &mut state,
        &keys,
        [key(KeyCode::Enter, None, false)].iter(),
        Some(composer::Control::Send)
    ));
    assert!(state.naming.is_none() && state.input.is_empty() && !state.input_open);
    assert_eq!(world.canonical_signature_digest().unwrap(), before);
}

#[test]
fn target_is_bound_to_stable_id_organism_and_original_canonical_name() {
    let mut world = world();
    let draft = draft(&world, None);
    assert!(target_matches(
        &draft,
        &selected(draft.stable_id),
        Some(OrganismId(1)),
        &world
    ));
    assert!(!target_matches(
        &draft,
        &selected(WorldEntityId(2)),
        Some(OrganismId(2)),
        &world
    ));
    assert!(!target_matches(
        &draft,
        &selected(draft.stable_id),
        Some(OrganismId(2)),
        &world
    ));
    world
        .rename_agent(draft.stable_id, "changed elsewhere")
        .unwrap();
    assert!(!target_matches(
        &draft,
        &selected(draft.stable_id),
        Some(OrganismId(1)),
        &world
    ));
}

#[test]
fn name_composer_hides_address_and_captures_held_input_through_dismissal() {
    use crate::production_voxel_renderer::care_dock::tests::layout_app;
    let world = world();
    let mut app = layout_app(Vec2::new(640.0, 360.0));
    app.insert_resource(state(&world, None))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>();
    composer::spawn(app.world_mut());
    composer::adapt(app.world_mut());
    for (control, node, a11y) in app
        .world_mut()
        .query::<(&composer::Control, &Node, &AccessibilityNode)>()
        .iter(app.world())
    {
        if *control == composer::Control::Address {
            assert_eq!(node.display, Display::None);
            assert!(a11y.is_hidden());
        }
        if *control == composer::Control::Send {
            assert_eq!(a11y.label(), Some("Confirm individual name. Enter."));
        }
    }
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    composer::collect_input(app.world_mut());
    cancel(
        &mut app
            .world_mut()
            .resource_mut::<ProductionConversationLineageUiState>(),
    );
    composer::collect_input(app.world_mut());
    assert!(app
        .world()
        .resource::<ProductionConversationLineageUiState>()
        .blocks_world_shortcuts());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::Enter);
    composer::collect_input(app.world_mut());
    assert!(!app
        .world()
        .resource::<ProductionConversationLineageUiState>()
        .blocks_world_shortcuts());
}

#[test]
fn unicode_names_and_aliases_resolve_and_ambiguous_legacy_names_fail_closed() {
    let mut world = world();
    let id = world.entity_id("founder").unwrap();
    world.rename_agent(id, "猫 Élan").unwrap();
    for text in ["猫 ÉLAN, come", "creature 1: come"] {
        assert_eq!(named_addressee(text, &world), Ok(Some(OrganismId(1))));
    }
    assert_eq!(named_addressee("cat come", &world), Ok(None));
    let ambiguous = HeadlessScenarioBuilder::new(21)
        .agent("Pip", OrganismId(1), Vec3f::ZERO)
        .agent("PIP", OrganismId(2), Vec3f::ZERO)
        .agent("Pip Junior", OrganismId(3), Vec3f::ZERO)
        .agent("creature 1", OrganismId(4), Vec3f::ZERO)
        .build()
        .unwrap();
    for text in ["pip, come", "pip junior: come", "creature 1 come"] {
        assert!(named_addressee(text, &ambiguous).is_err(), "{text}");
    }
    assert_eq!(
        named_addressee("creature 3, come", &ambiguous),
        Ok(Some(OrganismId(3)))
    );
    assert!(ambiguous.audible_utterances().is_empty());
}

#[test]
fn player_toy_labels_cannot_be_taken_by_a_name_before_the_toy_exists() {
    let mut world = world();
    let id = world.entity_id("founder").unwrap();
    let before = world.canonical_signature_digest().unwrap();
    for name in ["player-plaything", " PLAYER-ACTIVITY-TOY "] {
        assert_eq!(
            crate::gpu_live_runtime::rename_player_creature_in_world(&mut world, id, name),
            Err(alife_world::RenameAgentError::ReservedName)
        );
        assert_eq!(world.canonical_signature_digest().unwrap(), before);
    }
}

#[test]
fn actual_opening_collector_handles_pointer_f2_accessibility_and_gates() {
    use crate::production_voxel_renderer::{
        care_dock::tests::layout_app, menu, status_hud::RenameButton,
    };
    let mut app = layout_app(Vec2::new(640.0, 360.0));
    app.insert_resource(super::super::speech_completion_state_tests::state())
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<OpenInput>()
        .init_resource::<menu::MenuState>()
        .add_message::<ActionRequest>();
    let button = app
        .world_mut()
        .spawn((
            RenameButton,
            Node::default(),
            Visibility::Visible,
            Interaction::Hovered,
        ))
        .id();
    assert_eq!(collect_opening(app.world_mut()), None);
    assert!(
        app.world()
            .resource::<ProductionConversationLineageUiState>()
            .blocks_world_shortcuts(),
        "hover captures world clicks and camera input"
    );
    *app.world_mut().get_mut::<Interaction>(button).unwrap() = Interaction::Pressed;
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    assert_eq!(collect_opening(app.world_mut()), Some(false));
    assert_eq!(
        collect_opening(app.world_mut()),
        None,
        "held click cannot reopen"
    );
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F2);
    assert_eq!(collect_opening(app.world_mut()), Some(true));
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    let request = || {
        ActionRequest(accesskit::ActionRequest {
            action: AccessibleAction::Click,
            target: accesskit::NodeId(button.to_bits()),
            data: None,
        })
    };
    app.world_mut().write_message(request());
    assert_eq!(collect_opening(app.world_mut()), Some(false));
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .debug_mode = true;
    app.world_mut().write_message(request());
    assert_eq!(collect_opening(app.world_mut()), None);
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .debug_mode = false;
    assert_eq!(
        collect_opening(app.world_mut()),
        None,
        "blocked accessibility input is drained"
    );
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())
        .unwrap();
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    app.world_mut().write_message(request());
    assert_eq!(collect_opening(app.world_mut()), None);
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    *app.world_mut().get_mut::<Visibility>(button).unwrap() = Visibility::Hidden;
    app.world_mut().write_message(request());
    assert_eq!(collect_opening(app.world_mut()), None);
    *app.world_mut().get_mut::<Visibility>(button).unwrap() = Visibility::Visible;
    app.insert_resource(
        LiveBrainPresentationFrameResource::from_authoritative_world(&world()).unwrap(),
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    menu::handle_input(app.world_mut());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut().write_message(request());
    assert_eq!(
        collect_opening(app.world_mut()),
        None,
        "menu captures rename input"
    );
}
