//! Raw keyboard boundary and Bevy input-edge contracts; no neural runtime fixture.
use super::*;
use bevy::ecs::message::{MessageCursor, Messages};
use bevy::input::{keyboard::Key, InputPlugin};
use bevy::prelude::Entity;

fn key(key_code: KeyCode, state: ButtonState, text: Option<&str>, repeat: bool) -> KeyboardInput {
    KeyboardInput {
        key_code,
        logical_key: match key_code {
            KeyCode::Enter => Key::Enter,
            KeyCode::Backspace => Key::Backspace,
            _ => Key::Character(text.unwrap_or_default().into()),
        },
        state,
        text: text.map(Into::into),
        repeat,
        window: Entity::PLACEHOLDER,
    }
}

fn enter(repeat: bool) -> KeyboardInput {
    key(KeyCode::Enter, ButtonState::Pressed, Some("\r"), repeat)
}

fn character(text: &str) -> KeyboardInput {
    key(KeyCode::KeyC, ButtonState::Pressed, Some(text), false)
}

#[test]
fn opening_frame_keeps_only_text_after_the_first_fresh_enter() {
    for (events, expected) in [
        (vec![character("c"), enter(false)], ""),
        (vec![enter(false), character("c")], "c"),
        (vec![character("c"), enter(false), character("é猫")], "é猫"),
        (vec![enter(false), character("c"), enter(true)], "c"),
        (vec![enter(false), character("c"), enter(false)], "c"),
    ] {
        let mut messages = Messages::default();
        let mut reader = MessageCursor::default();
        messages.write_batch(events);
        let mut input = String::new();
        assert!(!append_player_speech_key_text(
            &mut input,
            reader.read(&messages),
            true
        ));
        assert_eq!(input, expected);
        assert!(
            reader.is_empty(&messages),
            "opening frame must be fully consumed"
        );
    }
}

#[test]
fn an_enter_release_or_repeat_is_not_the_opening_text_boundary() {
    let mut messages = Messages::default();
    let mut reader = MessageCursor::default();
    messages.write_batch([
        key(KeyCode::Enter, ButtonState::Released, None, false),
        character("before release"),
        enter(true),
        character("before fresh press"),
        enter(false),
        character("after"),
    ]);
    let mut input = String::new();
    append_player_speech_key_text(&mut input, reader.read(&messages), true);
    assert_eq!(input, "after");
    assert!(reader.is_empty(&messages));
}

#[test]
fn a_missing_raw_opening_enter_discards_the_batch() {
    // A synthetic ButtonInput press must not turn unrelated unread text into speech.
    let mut messages = Messages::default();
    let mut reader = MessageCursor::default();
    messages.write_batch([character("c"), enter(true)]);
    let mut input = String::new();
    append_player_speech_key_text(&mut input, reader.read(&messages), true);
    assert!(input.is_empty());
    assert!(reader.is_empty(&messages));
}

#[test]
fn preopening_text_never_reappears_on_the_next_frame() {
    let mut messages = Messages::default();
    let mut reader = MessageCursor::default();
    messages.write_batch([character("c"), enter(false)]);
    let mut input = String::new();
    append_player_speech_key_text(&mut input, reader.read(&messages), true);
    messages.update();
    append_player_speech_key_text(&mut input, reader.read(&messages), false);
    assert!(input.is_empty());
    messages.write(character("é猫"));
    append_player_speech_key_text(&mut input, reader.read(&messages), false);
    assert_eq!(input, "é猫");
    assert!(reader.is_empty(&messages));
}

#[test]
fn active_typing_preserves_unicode_printable_repeats_and_the_existing_limit() {
    let events = [
        character("é猫\r\n\t"),
        key(KeyCode::KeyC, ButtonState::Released, Some("ignored"), false),
        key(KeyCode::KeyC, ButtonState::Pressed, Some("c"), true),
    ];
    let mut input = String::new();
    append_player_speech_key_text(&mut input, events.iter(), false);
    assert_eq!(input, "é猫c");
    input = "猫".repeat(MAX_TYPED_CHARS - 1);
    append_player_speech_key_text(&mut input, [character("é猫")].iter(), false);
    assert_eq!(input.chars().count(), MAX_TYPED_CHARS);
    assert!(input.ends_with('é'));
}

fn write_keys(app: &mut App, events: impl IntoIterator<Item = KeyboardInput>) {
    app.world_mut()
        .resource_mut::<Messages<KeyboardInput>>()
        .write_batch(events);
}

#[test]
fn held_enter_repeats_do_not_retrigger_the_bevy_submit_edge() {
    let mut app = App::new();
    app.add_plugins(InputPlugin);
    let mut input = "c".to_string();
    let mut submit_edges = 0;
    let events = [enter(false)];
    write_keys(&mut app, events.clone());
    app.update();
    let submit_enter = append_player_speech_key_text(&mut input, events.iter(), false);
    let keyboard = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(keyboard.just_pressed(KeyCode::Enter));
    submit_edges += usize::from(
        submit_enter && keyboard.just_pressed(KeyCode::Enter) && !input.trim().is_empty(),
    );
    for _ in 0..3 {
        let events = [enter(true), enter(true)];
        write_keys(&mut app, events.clone());
        app.update();
        let submit_enter = append_player_speech_key_text(&mut input, events.iter(), false);
        assert!(!submit_enter);
        let keyboard = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(keyboard.pressed(KeyCode::Enter));
        assert!(!keyboard.just_pressed(KeyCode::Enter));
        submit_edges += usize::from(
            submit_enter && keyboard.just_pressed(KeyCode::Enter) && !input.trim().is_empty(),
        );
    }
    // A focus/reset-like lost held state can make a repeat look like an aggregate edge.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset(KeyCode::Enter);
    let events = [enter(true)];
    write_keys(&mut app, events.clone());
    app.update();
    let keyboard = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(keyboard.just_pressed(KeyCode::Enter));
    let submit_enter = append_player_speech_key_text(&mut input, events.iter(), false);
    assert!(!submit_enter);
    submit_edges += usize::from(
        submit_enter && keyboard.just_pressed(KeyCode::Enter) && !input.trim().is_empty(),
    );
    assert_eq!(submit_edges, 1);
    let events = [
        key(KeyCode::Enter, ButtonState::Released, None, false),
        enter(false),
        enter(true),
    ];
    write_keys(&mut app, events.clone());
    app.update();
    let submit_enter = append_player_speech_key_text(&mut input, events.iter(), false);
    let keyboard = app.world().resource::<ButtonInput<KeyCode>>();
    assert!(submit_enter && keyboard.just_pressed(KeyCode::Enter));
    submit_edges += usize::from(
        submit_enter && keyboard.just_pressed(KeyCode::Enter) && !input.trim().is_empty(),
    );
    assert_eq!(
        submit_edges, 2,
        "a new physical press remains a distinct request"
    );
}

fn backspace(repeat: bool) -> KeyboardInput {
    key(
        KeyCode::Backspace,
        ButtonState::Pressed,
        Some("\u{8}"),
        repeat,
    )
}

#[test]
fn same_frame_text_and_backspace_follow_raw_event_order() {
    for (events, expected) in [
        (vec![character("c"), backspace(false)], "ab"),
        (vec![backspace(false), character("c")], "ac"),
        (
            vec![character("c"), backspace(false), character("d")],
            "abd",
        ),
        (
            vec![
                backspace(false),
                backspace(true),
                character("é猫"),
                backspace(true),
            ],
            "é",
        ),
    ] {
        let mut messages = Messages::default();
        let mut reader = MessageCursor::default();
        messages.write_batch(events);
        let mut input = "ab".to_string();
        assert!(!append_player_speech_key_text(
            &mut input,
            reader.read(&messages),
            false
        ));
        assert_eq!(input, expected);
        assert!(reader.is_empty(&messages));
        append_player_speech_key_text(&mut input, reader.read(&messages), false);
        assert_eq!(input, expected, "edits cannot replay on the next read");
    }
}

#[test]
fn backspace_release_and_attached_text_do_not_edit_or_insert_twice() {
    let mut input = "é猫".to_string();
    let events = [key(
        KeyCode::Backspace,
        ButtonState::Released,
        Some("ignored"),
        false,
    )];
    assert!(!append_player_speech_key_text(
        &mut input,
        events.iter(),
        false
    ));
    assert_eq!(input, "é猫");
    let events = [key(
        KeyCode::Backspace,
        ButtonState::Pressed,
        Some("not inserted"),
        false,
    )];
    assert!(!append_player_speech_key_text(
        &mut input,
        events.iter(),
        false
    ));
    assert_eq!(
        input, "é",
        "one pressed Backspace removes one Unicode scalar, regardless of its text payload"
    );
}

#[test]
fn held_backspace_repeats_edit_without_a_new_aggregate_edge() {
    let mut app = App::new();
    app.add_plugins(InputPlugin);
    // Establish the held key. The draft below represents the contents after
    // that initial press; later repeats must work without another Bevy edge.
    write_keys(&mut app, [backspace(false)]);
    app.update();
    assert!(app
        .world()
        .resource::<ButtonInput<KeyCode>>()
        .just_pressed(KeyCode::Backspace));
    let mut input = "é猫ab".to_string();
    for (events, expected) in [
        (vec![backspace(true)], "é猫a"),
        (vec![backspace(true), backspace(true)], "é"),
    ] {
        write_keys(&mut app, events.clone());
        app.update();
        let keyboard = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(keyboard.pressed(KeyCode::Backspace));
        assert!(!keyboard.just_pressed(KeyCode::Backspace));
        assert!(!append_player_speech_key_text(
            &mut input,
            events.iter(),
            false
        ));
        assert_eq!(input, expected);
    }
    // Even when held state is lost, a raw repeat edits exactly once rather
    // than being applied once through ButtonInput and again through the stream.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset(KeyCode::Backspace);
    let events = [backspace(true)];
    write_keys(&mut app, events.clone());
    app.update();
    assert!(app
        .world()
        .resource::<ButtonInput<KeyCode>>()
        .just_pressed(KeyCode::Backspace));
    append_player_speech_key_text(&mut input, events.iter(), false);
    assert!(input.is_empty());
    let events = [backspace(true), backspace(true)];
    write_keys(&mut app, events.clone());
    app.update();
    assert!(!append_player_speech_key_text(
        &mut input,
        events.iter(),
        false
    ));
    assert!(input.is_empty());
    input = "ab".into();
    let events = [
        key(KeyCode::Backspace, ButtonState::Released, None, false),
        backspace(false),
    ];
    write_keys(&mut app, events.clone());
    app.update();
    assert!(app
        .world()
        .resource::<ButtonInput<KeyCode>>()
        .just_pressed(KeyCode::Backspace));
    append_player_speech_key_text(&mut input, events.iter(), false);
    assert_eq!(input, "a");
}

#[test]
fn opening_boundary_discards_old_edits_and_applies_new_edits_in_order() {
    let events = [
        character("old"),
        backspace(false),
        enter(true),
        backspace(true),
        enter(false),
        character("é猫"),
        backspace(true),
        character("c"),
        enter(false),
    ];
    let mut messages = Messages::default();
    let mut reader = MessageCursor::default();
    messages.write_batch(events);
    let mut input = String::new();
    assert!(!append_player_speech_key_text(
        &mut input,
        reader.read(&messages),
        true
    ));
    assert_eq!(input, "éc");
    assert!(reader.is_empty(&messages));
}

#[test]
fn ordered_backspace_and_text_keep_the_character_limit_without_replaying_rejected_text() {
    let full = "猫".repeat(MAX_TYPED_CHARS);
    let mut input = full.clone();
    append_player_speech_key_text(&mut input, [character("é"), backspace(false)].iter(), false);
    assert_eq!(
        input,
        "猫".repeat(MAX_TYPED_CHARS - 1),
        "text rejected at the limit cannot return after deletion"
    );
    input = full.clone();
    append_player_speech_key_text(
        &mut input,
        [backspace(false), character("é猫")].iter(),
        false,
    );
    assert_eq!(input.chars().count(), MAX_TYPED_CHARS);
    assert!(input.ends_with('é'));
    input = full;
    append_player_speech_key_text(
        &mut input,
        [backspace(false), backspace(true), character("é猫")].iter(),
        false,
    );
    assert_eq!(input.chars().count(), MAX_TYPED_CHARS);
    assert!(input.ends_with("é猫"));
}
