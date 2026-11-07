use super::*;
use bevy::{
    ecs::message::{MessageCursor, Messages},
    input::{keyboard::Key, keyboard::KeyboardFocusLost, InputPlugin},
    prelude::Query,
};

#[derive(Default, Resource)]
struct Playback {
    paused: bool,
    toggles: usize,
    captured: bool,
}

// The production handler reads the request before its menu/conversation returns.
// This CPU fixture exercises that input boundary without a GPU runtime.
fn sample_playback(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut events: MessageReader<KeyboardInput>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut playback: ResMut<Playback>,
) {
    let requested = playback_toggle_requested(
        &keyboard,
        events.read(),
        windows
            .single()
            .ok()
            .filter(|(_, window)| window.focused)
            .map(|(entity, _)| entity),
    );
    if playback.captured {
        return;
    }
    if requested {
        playback.paused = !playback.paused;
        playback.toggles += 1;
    }
}

fn app() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(InputPlugin)
        .init_resource::<Playback>()
        .add_systems(Update, sample_playback);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    (app, window)
}

fn key(window: Entity, code: KeyCode, state: ButtonState, repeat: bool) -> KeyboardInput {
    KeyboardInput {
        key_code: code,
        logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
        state,
        text: None,
        repeat,
        window,
    }
}

fn frame(app: &mut App, window: Entity, events: &[(KeyCode, ButtonState, bool)]) {
    for &(code, state, repeat) in events {
        app.world_mut()
            .write_message(key(window, code, state, repeat));
    }
    app.update();
}

#[test]
fn focus_reset_repeats_cannot_resume_and_real_new_presses_still_work() {
    for code in [KeyCode::Space, KeyCode::KeyP] {
        let (mut app, window) = app();
        frame(&mut app, window, &[(code, ButtonState::Pressed, false)]);
        assert!(app.world().resource::<Playback>().paused);
        assert_eq!(app.world().resource::<Playback>().toggles, 1);
        app.world_mut().write_message(KeyboardFocusLost);
        frame(&mut app, window, &[]);
        assert!(!app.world().resource::<ButtonInput<KeyCode>>().pressed(code));
        frame(&mut app, window, &[(code, ButtonState::Pressed, true)]);
        assert!(app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(code));
        assert!(app.world().resource::<Playback>().paused, "{code:?}");
        assert_eq!(app.world().resource::<Playback>().toggles, 1);
        frame(&mut app, window, &[(code, ButtonState::Pressed, true)]);
        assert_eq!(app.world().resource::<Playback>().toggles, 1);
        frame(&mut app, window, &[(code, ButtonState::Released, false)]);
        frame(&mut app, window, &[(code, ButtonState::Pressed, false)]);
        assert!(!app.world().resource::<Playback>().paused);
        assert_eq!(app.world().resource::<Playback>().toggles, 2);
    }
}

#[test]
fn unfocused_fresh_keys_cannot_undo_autopause_or_replay_after_focus_returns() {
    for code in [KeyCode::Space, KeyCode::KeyP] {
        let (mut app, window) = app();
        app.world_mut().resource_mut::<Playback>().paused = true;
        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        frame(&mut app, window, &[(code, ButtonState::Pressed, false)]);
        assert!(app.world().resource::<Playback>().paused, "{code:?}");
        assert_eq!(app.world().resource::<Playback>().toggles, 0);
        app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
        frame(&mut app, window, &[]);
        frame(&mut app, window, &[(code, ButtonState::Pressed, true)]);
        assert!(app.world().resource::<Playback>().paused);
        frame(&mut app, window, &[(code, ButtonState::Released, false)]);
        frame(&mut app, window, &[(code, ButtonState::Pressed, false)]);
        assert!(!app.world().resource::<Playback>().paused);
        assert_eq!(app.world().resource::<Playback>().toggles, 1);
    }
}

#[test]
fn captured_playback_requests_are_drained_before_return_and_never_replayed() {
    for code in [KeyCode::Space, KeyCode::KeyP] {
        let (mut app, window) = app();
        app.world_mut().resource_mut::<Playback>().captured = true;
        frame(&mut app, window, &[(code, ButtonState::Pressed, false)]);
        app.world_mut().resource_mut::<Playback>().captured = false;
        frame(&mut app, window, &[]);
        assert_eq!(app.world().resource::<Playback>().toggles, 0);
        frame(&mut app, window, &[(code, ButtonState::Released, false)]);
        frame(&mut app, window, &[(code, ButtonState::Pressed, false)]);
        assert_eq!(app.world().resource::<Playback>().toggles, 1);
    }
}

#[test]
fn missing_ambiguous_and_other_windows_cannot_request_playback() {
    for mode in 0..3 {
        let (mut app, window) = app();
        let other = app.world_mut().spawn(Window::default()).id();
        if mode == 0 {
            app.world_mut().entity_mut(window).remove::<PrimaryWindow>();
        } else if mode == 1 {
            app.world_mut().entity_mut(other).insert(PrimaryWindow);
        }
        frame(
            &mut app,
            other,
            &[(KeyCode::Space, ButtonState::Pressed, false)],
        );
        assert_eq!(app.world().resource::<Playback>().toggles, 0, "{mode}");
        app.world_mut().entity_mut(other).remove::<PrimaryWindow>();
        app.world_mut().entity_mut(window).insert(PrimaryWindow);
        frame(&mut app, window, &[]);
        assert_eq!(app.world().resource::<Playback>().toggles, 0);
        frame(
            &mut app,
            window,
            &[(KeyCode::Space, ButtonState::Released, false)],
        );
        frame(
            &mut app,
            window,
            &[(KeyCode::Space, ButtonState::Pressed, false)],
        );
        assert_eq!(app.world().resource::<Playback>().toggles, 1);
    }
}

#[test]
fn freshness_cannot_be_borrowed_from_another_held_key() {
    for (held, repeated) in [
        (KeyCode::Space, KeyCode::KeyP),
        (KeyCode::KeyP, KeyCode::Space),
    ] {
        let (mut app, window) = app();
        frame(&mut app, window, &[(held, ButtonState::Pressed, false)]);
        frame(
            &mut app,
            window,
            &[
                (held, ButtonState::Pressed, false),
                (repeated, ButtonState::Pressed, true),
            ],
        );
        assert_eq!(app.world().resource::<Playback>().toggles, 1);
    }
}

#[test]
fn fresh_keys_coalesce_and_the_entire_raw_batch_is_consumed() {
    let window = Entity::PLACEHOLDER;
    let mut keyboard = ButtonInput::default();
    keyboard.press(KeyCode::Space);
    keyboard.press(KeyCode::KeyP);
    let mut messages = Messages::default();
    messages.write_batch([
        key(window, KeyCode::Space, ButtonState::Pressed, false),
        key(window, KeyCode::KeyP, ButtonState::Pressed, false),
        key(window, KeyCode::Space, ButtonState::Pressed, true),
    ]);
    let mut reader = MessageCursor::default();
    assert!(playback_toggle_requested(
        &keyboard,
        reader.read(&messages),
        Some(window)
    ));
    assert!(reader.is_empty(&messages));
    assert!(!playback_toggle_requested(
        &keyboard,
        reader.read(&messages),
        Some(window)
    ));
    let (mut app, window) = app();
    frame(
        &mut app,
        window,
        &[
            (KeyCode::Space, ButtonState::Pressed, false),
            (KeyCode::KeyP, ButtonState::Pressed, false),
        ],
    );
    assert_eq!(app.world().resource::<Playback>().toggles, 1);
}

#[test]
fn aggregate_edges_need_the_matching_raw_fresh_press() {
    let window = Entity::PLACEHOLDER;
    let mut keyboard = ButtonInput::default();
    keyboard.press(KeyCode::Space);
    for events in [
        vec![],
        vec![key(window, KeyCode::Space, ButtonState::Released, false)],
        vec![key(window, KeyCode::Space, ButtonState::Pressed, true)],
        vec![key(window, KeyCode::KeyP, ButtonState::Pressed, false)],
        vec![key(window, KeyCode::KeyE, ButtonState::Pressed, false)],
    ] {
        assert!(!playback_toggle_requested(
            &keyboard,
            events.iter(),
            Some(window)
        ));
    }
    assert!(playback_toggle_requested(
        &keyboard,
        [key(window, KeyCode::Space, ButtonState::Pressed, false)].iter(),
        Some(window)
    ));
}

#[test]
fn same_frame_quick_taps_and_mixed_fresh_repeated_keys_toggle_once() {
    for code in [KeyCode::Space, KeyCode::KeyP] {
        let (mut app, window) = app();
        frame(
            &mut app,
            window,
            &[
                (code, ButtonState::Pressed, false),
                (code, ButtonState::Released, false),
            ],
        );
        assert!(app.world().resource::<Playback>().paused);
        assert_eq!(app.world().resource::<Playback>().toggles, 1);
        frame(&mut app, window, &[]);
        frame(&mut app, window, &[(code, ButtonState::Pressed, false)]);
        assert!(!app.world().resource::<Playback>().paused);
        assert_eq!(app.world().resource::<Playback>().toggles, 2);
    }
    for (fresh, repeated) in [
        (KeyCode::Space, KeyCode::KeyP),
        (KeyCode::KeyP, KeyCode::Space),
    ] {
        let (mut app, window) = app();
        frame(
            &mut app,
            window,
            &[
                (repeated, ButtonState::Pressed, true),
                (fresh, ButtonState::Pressed, false),
            ],
        );
        assert_eq!(app.world().resource::<Playback>().toggles, 1);
    }
}
