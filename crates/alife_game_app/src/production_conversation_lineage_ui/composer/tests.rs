//! Bounded CPU Bevy layout/hit-testing and shared control tests. No live GPU
//! input handler, OS accessibility service, neural execution or glyph rendering.
use super::*;
use crate::production_voxel_renderer::{
    care_dock::tests::{cpu_focus_fixture_stack, focus_cursor, layout_app},
    menu,
};
use bevy::{
    camera::RenderTarget,
    input::touch::Touches,
    ui::{ComputedNode, UiGlobalTransform},
};

fn state() -> ProductionConversationLineageUiState {
    super::super::speech_completion_state_tests::state()
}
fn app(size: Vec2, text: &str) -> App {
    let mut app = layout_app(size);
    let mut state = state();
    state.prepare_typed_speech_fixture(text);
    app.insert_resource(state);
    spawn(app.world_mut());
    adapt(app.world_mut());
    app.update();
    app.update();
    app
}
fn entity<T: Component>(world: &mut World) -> Entity {
    world
        .query_filtered::<Entity, With<T>>()
        .single(world)
        .unwrap()
}
fn bounds(world: &World, e: Entity) -> (Vec2, Vec2) {
    let center = world
        .get::<UiGlobalTransform>(e)
        .unwrap()
        .affine()
        .translation;
    let half = world.get::<ComputedNode>(e).unwrap().size() * 0.5;
    (center - half, center + half)
}
fn contains(parent: (Vec2, Vec2), child: (Vec2, Vec2)) -> bool {
    child.0.cmpge(parent.0 - Vec2::splat(0.1)).all()
        && child.1.cmple(parent.1 + Vec2::splat(0.1)).all()
}
fn control(world: &mut World, c: Control) -> Entity {
    world
        .query::<(Entity, &Control)>()
        .iter(world)
        .find_map(|(e, a)| (*a == c).then_some(e))
        .unwrap()
}
#[test]
fn wordless_controls_stay_fixed_above_long_drafts_at_four_viewports_and_error_gutters() {
    let mut fixtures = Vec::new();
    for size in [
        Vec2::new(1280.0, 720.0),
        Vec2::new(640.0, 360.0),
        Vec2::new(320.0, 480.0),
        Vec2::new(320.0, 240.0),
    ] {
        for error in [false, true] {
            let draft = "x".repeat(MAX_TYPED_CHARS);
            let mut app = app(size, &draft);
            app.world_mut()
                .resource_mut::<Fvr05ProductionUxStateResource>()
                .last_error = error.then(|| "synthetic rejection".into());
            adapt(app.world_mut());
            app.update();
            app.update();
            let root = entity::<ProductionSpeechComposerBounds>(app.world_mut());
            let root_box = bounds(app.world(), root);
            assert!(contains((Vec2::ZERO, size), root_box));
            assert!(root_box.1.y <= size.y - if error { 108.0 } else { 92.0 } + 0.1);
            let mut boxes = Vec::new();
            for c in CONTROLS {
                let e = control(app.world_mut(), c);
                let b = bounds(app.world(), e);
                assert!(contains(root_box, b));
                assert!(((b.1 - b.0) - Vec2::splat(48.0)).abs().max_element() < 0.1);
                assert!(!app
                    .world()
                    .get::<AccessibilityNode>(e)
                    .unwrap()
                    .label()
                    .unwrap()
                    .is_empty());
                assert!(app.world().get::<Text>(e).is_none());
                boxes.push(b);
            }
            for pair in boxes.windows(2) {
                assert!(pair[0].1.x <= pair[1].0.x);
            }
            let viewport = entity::<DraftViewport>(app.world_mut());
            let field = bounds(app.world(), viewport);
            assert!(contains(root_box, field));
            assert!(field.0.y >= boxes[0].1.y + 5.9);
            assert_eq!(
                app.world().get::<Node>(viewport).unwrap().overflow.y,
                bevy::ui::OverflowAxis::Scroll
            );
            let entry = entity::<ProductionSpeechEntryPanel>(app.world_mut());
            assert_eq!(
                app.world().get::<Text>(entry).unwrap().0,
                format!("{draft}|")
            );
            assert_eq!(
                app.world().get::<AccessibilityNode>(entry).unwrap().value(),
                Some(draft.as_str())
            );
            let texts = app
                .world_mut()
                .query::<&Text>()
                .iter(app.world())
                .map(|t| t.0.clone())
                .collect::<Vec<_>>();
            assert!(texts.iter().all(|t| *t == format!("{draft}|") || t == HELP));
            app.world_mut().clear_trackers();
            adapt(app.world_mut());
            assert!(
                !app.world()
                    .entity(entry)
                    .get_ref::<Text>()
                    .unwrap()
                    .is_changed(),
                "idle composer must not request text shaping again"
            );
            fixtures.push(serde_json::json!({"viewport":[size.x,size.y],"case":"long_draft","error":error,
                "nodes":crate::production_voxel_renderer::care_dock::tests::export_nodes(app.world_mut())}));
            app.world_mut()
                .resource_mut::<ProductionConversationLineageUiState>()
                .input_help_open = true;
            adapt(app.world_mut());
            app.update();
            app.update();
            let cancel = control(app.world_mut(), Control::Cancel);
            assert_eq!(bounds(app.world(), cancel), boxes[2]);
            assert_eq!(
                app.world()
                    .resource::<ProductionConversationLineageUiState>()
                    .input,
                draft
            );
            assert_eq!(
                app.world().get::<Node>(entry).unwrap().display,
                Display::None
            );
            assert_eq!(
                app.world().get::<ScrollPosition>(viewport).unwrap().0.y,
                0.0
            );
            fixtures.push(serde_json::json!({"viewport":[size.x,size.y],"case":"keyboard_help","error":error,
                "nodes":crate::production_voxel_renderer::care_dock::tests::export_nodes(app.world_mut())}));
            app.world_mut()
                .resource_mut::<ProductionConversationLineageUiState>()
                .input_help_open = false;
            app.world_mut()
                .resource_mut::<ProductionConversationLineageUiState>()
                .input = "what are you doing?".into();
            app.world_mut()
                .resource_mut::<ProductionConversationLineageUiState>()
                .address_selected = false;
            app.world_mut()
                .resource_mut::<ProductionConversationLineageUiState>()
                .input_cue = if error {
                SpeechInputCue::Failed
            } else {
                SpeechInputCue::Pending
            };
            adapt(app.world_mut());
            app.update();
            app.update();
            fixtures.push(serde_json::json!({"viewport":[size.x,size.y],"case":"broadcast_draft","error":error,
                "nodes":crate::production_voxel_renderer::care_dock::tests::export_nodes(app.world_mut())}));
        }
    }
    if let Ok(path) = std::env::var("ALIFE_SPEECH_COMPOSER_CPU_FIXTURE") {
        let receipt = serde_json::json!({"evidence_kind":"Actual CPU Bevy UiSurface geometry; synthetic draft and status state",
            "is_screenshot":false,"glyph_bounds_measured":false,"fixtures":fixtures});
        std::fs::write(path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    }
}

#[test]
fn keyboard_and_pointer_controls_preserve_addressing_and_cancel_wins() {
    let mut state = state();
    state.prepare_typed_speech_fixture("Ada, come here é猫");
    let draft = state.input.clone();
    let mut keys = ButtonInput::default();
    assert!(!apply_controls(&mut state, &keys, Some(Control::Address)));
    assert!(!state.address_selected);
    assert_eq!(state.input, draft);
    keys.press(KeyCode::Tab);
    assert!(!apply_controls(&mut state, &keys, Some(Control::Address)));
    assert!(
        state.address_selected,
        "simultaneous Tab and pointer toggle once"
    );
    keys.reset_all();
    keys.press(KeyCode::F1);
    assert!(!apply_controls(&mut state, &keys, Some(Control::Help)));
    assert!(state.input_help_open);
    assert_eq!(state.input, draft);
    keys.reset_all();
    keys.press(KeyCode::Escape);
    assert!(apply_controls(&mut state, &keys, Some(Control::Send)));
    assert!(!state.input_open);
    assert!(!state.input_help_open);
    assert!(state.input.is_empty());
    assert_eq!(state.input_cue, SpeechInputCue::None);
}
#[test]
fn status_cues_distinguish_pending_rejection_and_unavailable_without_success_claims() {
    let mut app = app(Vec2::new(320.0, 240.0), "new draft");
    let status = entity::<Status>(app.world_mut());
    let mut images = Vec::new();
    for cue in [
        SpeechInputCue::None,
        SpeechInputCue::Pending,
        SpeechInputCue::Failed,
        SpeechInputCue::Unavailable,
    ] {
        app.world_mut()
            .resource_mut::<ProductionConversationLineageUiState>()
            .input_cue = cue;
        adapt(app.world_mut());
        let a11y = app.world().get::<AccessibilityNode>(status).unwrap();
        assert!(!a11y.label().unwrap().contains("success"));
        let child = app.world().get::<Children>(status).unwrap()[0];
        assert_eq!(
            app.world().get::<Visibility>(child).unwrap() == &Visibility::Hidden,
            cue == SpeechInputCue::None
        );
        if cue != SpeechInputCue::None {
            images.push(app.world().get::<ImageNode>(child).unwrap().image.clone());
        }
        assert_eq!(
            app.world()
                .resource::<ProductionConversationLineageUiState>()
                .input,
            "new draft"
        );
    }
    assert!(images[0] != images[1] && images[1] != images[2] && images[0] != images[2]);
    let send = control(app.world_mut(), Control::Send);
    app.world_mut()
        .resource_mut::<ProductionConversationLineageUiState>()
        .input = " \t ".into();
    adapt(app.world_mut());
    assert!(app
        .world()
        .get::<AccessibilityNode>(send)
        .unwrap()
        .is_disabled());
}
fn install_focus_input(app: &mut App) {
    app.init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Touches>()
        .init_resource::<menu::MenuState>()
        .add_message::<ActionRequest>();
    let camera = entity::<Camera>(app.world_mut());
    app.world_mut()
        .entity_mut(camera)
        .insert(RenderTarget::default());
    app.add_systems(
        Update,
        (
            cpu_focus_fixture_stack,
            bevy::ui::ui_focus_system,
            collect_input,
        )
            .chain(),
    );
}
fn click(app: &mut App, c: Control) {
    let e = control(app.world_mut(), c);
    let b = bounds(app.world(), e);
    focus_cursor(app.world_mut(), (b.0 + b.1) * 0.5);
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .release(MouseButton::Left);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .clear();
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    assert_eq!(
        app.world().get::<Interaction>(e),
        Some(&Interaction::Pressed)
    );
}
#[test]
fn actual_ui_focus_clicks_repeated_cancel_and_held_dismissal_keep_world_capture() {
    let mut app = app(Vec2::new(320.0, 240.0), "é猫");
    install_focus_input(&mut app);
    let prior = app.world_mut().spawn_empty().id();
    app.world_mut().resource_mut::<InputState>().owns_focus = false;
    app.world_mut().resource_mut::<InputFocus>().0 = Some(prior);
    adapt(app.world_mut());
    let entry = entity::<ProductionSpeechEntryPanel>(app.world_mut());
    for _ in 0..4 {
        click(&mut app, Control::Address);
        let pending = app.world_mut().resource_mut::<InputState>().pending.take();
        assert_eq!(pending, Some(Control::Address));
        let keys = ButtonInput::default();
        apply_controls(
            &mut app
                .world_mut()
                .resource_mut::<ProductionConversationLineageUiState>(),
            &keys,
            pending,
        );
        adapt(app.world_mut());
        assert_eq!(app.world().resource::<InputFocus>().0, Some(entry));
        assert_eq!(
            app.world()
                .resource::<ProductionConversationLineageUiState>()
                .input,
            "é猫"
        );
        click(&mut app, Control::Cancel);
        let pending = app.world_mut().resource_mut::<InputState>().pending.take();
        assert_eq!(pending, Some(Control::Cancel));
        apply_controls(
            &mut app
                .world_mut()
                .resource_mut::<ProductionConversationLineageUiState>(),
            &keys,
            pending,
        );
        adapt(app.world_mut());
        assert_eq!(app.world().resource::<InputFocus>().0, Some(prior));
        assert!(app
            .world()
            .resource::<ProductionConversationLineageUiState>()
            .blocks_world_shortcuts());
        focus_cursor(app.world_mut(), Vec2::new(2.0, 2.0));
        app.update();
        assert!(
            app.world()
                .resource::<ProductionConversationLineageUiState>()
                .blocks_world_shortcuts(),
            "held dismissal outside composer stays captured"
        );
        assert!(app.world().resource::<InputState>().pending.is_none());
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert!(!app
            .world()
            .resource::<ProductionConversationLineageUiState>()
            .blocks_world_shortcuts());
        app.world_mut()
            .resource_mut::<ProductionConversationLineageUiState>()
            .prepare_typed_speech_fixture("é猫");
        adapt(app.world_mut());
        app.update();
    }
}
#[test]
fn accessibility_requests_are_bounded_and_do_not_replay_after_focus_loss_or_close() {
    let mut app = app(Vec2::new(640.0, 360.0), "draft");
    install_focus_input(&mut app);
    let entry = entity::<ProductionSpeechEntryPanel>(app.world_mut());
    let send = control(app.world_mut(), Control::Send);
    let request = |action, target: Entity, data| {
        ActionRequest(accesskit::ActionRequest {
            action,
            target: accesskit::NodeId(target.to_bits()),
            data,
        })
    };
    app.world_mut().write_message(request(
        AccessibleAction::SetValue,
        entry,
        Some(ActionData::Value(("é猫\n".repeat(300)).into())),
    ));
    app.update();
    let state = app
        .world()
        .resource::<ProductionConversationLineageUiState>();
    assert_eq!(state.input.chars().count(), 512);
    assert!(!state.input.contains('\n'));
    let window = entity::<bevy::window::PrimaryWindow>(app.world_mut());
    app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
    app.world_mut()
        .write_message(request(AccessibleAction::Click, send, None));
    app.update();
    assert!(app.world().resource::<InputState>().pending.is_none());
    app.world_mut().get_mut::<Window>(window).unwrap().focused = true;
    app.update();
    assert!(app.world().resource::<InputState>().pending.is_none());
    app.world_mut()
        .resource_mut::<ProductionConversationLineageUiState>()
        .input_open = false;
    app.world_mut()
        .write_message(request(AccessibleAction::Click, send, None));
    app.update();
    app.world_mut()
        .resource_mut::<ProductionConversationLineageUiState>()
        .open_player_speech_input();
    app.update();
    assert!(app.world().resource::<InputState>().pending.is_none());
    app.world_mut()
        .write_message(request(AccessibleAction::Click, send, None));
    app.update();
    assert_eq!(
        app.world().resource::<InputState>().pending,
        Some(Control::Send)
    );
}

#[test]
fn accepted_worker_completion_preserves_reopened_failed_or_cancelled_drafts() {
    let mut state = state();
    state.prepare_typed_speech_fixture("draft A");
    state.accept_player_speech_translation();
    state.prepare_typed_speech_fixture("é猫 draft B");
    state.input_cue = SpeechInputCue::Failed;
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    assert!(state.input_open);
    assert_eq!(state.input, "é猫 draft B");
    assert_eq!(state.input_cue, SpeechInputCue::Failed);
    let mut keys = ButtonInput::default();
    keys.press(KeyCode::Escape);
    assert!(apply_controls(&mut state, &keys, None));
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    assert!(!state.input_open);
    assert!(state.input.is_empty());
    assert_eq!(state.input_cue, SpeechInputCue::None);
    assert!(apply_controls(&mut state, &keys, None));
    assert!(!state.input_open);
    assert!(state.input.is_empty());
}
