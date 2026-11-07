//! Native CPU tests of the production modal/input transitions, without a GPU.
use super::*;
use bevy::ecs::message::Messages;
use bevy::input::keyboard::Key;
use serde_json::json;

fn keys(codes: &[KeyCode]) -> ButtonInput<KeyCode> {
    let mut keys = ButtonInput::default();
    for &code in codes {
        keys.press(code);
    }
    keys
}
fn key_frame(state: &mut MenuState, codes: &[KeyCode]) -> InputOutcome {
    state.input(
        &keys(codes),
        false,
        false,
        codes
            .iter()
            .any(|code| matches!(code, KeyCode::Enter | KeyCode::NumpadEnter)),
        None,
        false,
        &[],
        false,
        false,
        LoadAvailability::Ready,
    )
}
fn release(state: &mut MenuState) {
    key_frame(state, &[]);
}
fn prompt(origin: Screen, paused: bool) -> MenuState {
    let mut state = MenuState::default();
    if origin != Screen::Closed {
        state.open(paused);
        state.screen = origin;
    }
    assert!(state.request_load(paused, LoadAvailability::Ready));
    state
}

#[test]
fn closed_l_and_menu_load_open_same_cancel_default_without_queueing() {
    let mut state = MenuState::default();
    let outcome = key_frame(&mut state, &[KeyCode::KeyL, KeyCode::Enter]);
    assert!(outcome.opening);
    assert_eq!(state.screen, Screen::ConfirmLoad);
    assert_eq!(state.selected, 0);
    assert_eq!(state.pending, None);
    assert!(state.captured && state.release_gate);
    let mut state = MenuState::default();
    state.open(false);
    state.input(
        &keys(&[]),
        true,
        true,
        false,
        None,
        false,
        &[
            (Screen::Main, Action::Load),
            (Screen::ConfirmLoad, Action::ConfirmLoad),
        ],
        false,
        false,
        LoadAvailability::Ready,
    );
    assert_eq!(state.screen, Screen::ConfirmLoad);
    assert_eq!(state.pending, None);
}

#[test]
fn opening_held_enter_and_repeated_l_never_confirm() {
    let mut state = prompt(Screen::Closed, false);
    for _ in 0..4 {
        key_frame(&mut state, &[KeyCode::KeyL, KeyCode::Enter]);
        assert_eq!(state.pending, None);
    }
    release(&mut state);
    for code in [KeyCode::KeyL, KeyCode::KeyS] {
        key_frame(&mut state, &[code]);
        assert_eq!(state.pending, None);
    }
    key_frame(&mut state, &[KeyCode::Enter]);
    assert_eq!(state.screen, Screen::Closed); // The initial selection was Cancel.
    assert!(state.captured && state.release_gate);
}

#[test]
fn cancel_returns_to_invoking_page_and_restores_only_owned_pause() {
    for origin in [
        Screen::Closed,
        Screen::Main,
        Screen::Speed,
        Screen::Settings,
    ] {
        for paused in [false, true] {
            let mut state = prompt(origin, paused);
            let outcome = key_frame(&mut state, &[KeyCode::Escape]);
            assert_eq!(state.screen, origin);
            assert_eq!(
                outcome.resume,
                (origin == Screen::Closed).then_some(!paused)
            );
            assert!(state.captured && state.release_gate);
            assert_eq!(state.pending, None);
            assert_eq!(state.load_phase, LoadPhase::Idle);
        }
    }
}

#[test]
fn stationary_double_click_and_release_frame_drag_cannot_arm_load() {
    let mut state = prompt(Screen::Main, false);
    state.last_cursor = Some(Vec2::ZERO);
    let target = Vec2::new(10.0, 20.0);
    let pressed = [(Screen::ConfirmLoad, Action::ConfirmLoad)];
    // Drag over the new target while still holding the opening click.
    state.input(
        &keys(&[]),
        true,
        false,
        false,
        Some(target),
        true,
        &pressed,
        true,
        false,
        LoadAvailability::Ready,
    );
    state.input(
        &keys(&[]),
        false,
        false,
        false,
        Some(target),
        true,
        &[],
        true,
        false,
        LoadAvailability::Ready,
    );
    state.input(
        &keys(&[]),
        true,
        true,
        false,
        Some(target),
        true,
        &pressed,
        true,
        false,
        LoadAvailability::Ready,
    );
    assert_eq!(state.pending, None);
    assert!(!state.pointer_confirm_armed);
    // Only an actual move on a later released frame arms the separate target.
    state.input(
        &keys(&[]),
        false,
        false,
        false,
        Some(target + Vec2::X),
        true,
        &[],
        true,
        false,
        LoadAvailability::Ready,
    );
    assert!(state.pointer_confirm_armed);
    state.input(
        &keys(&[]),
        true,
        false,
        false,
        Some(target + Vec2::X),
        true,
        &pressed,
        true,
        false,
        LoadAvailability::Ready,
    );
    assert_eq!(state.pending, None); // Held/reinserted Pressed is not a fresh click.
    state.input(
        &keys(&[]),
        true,
        true,
        false,
        Some(target + Vec2::X),
        true,
        &pressed,
        true,
        false,
        LoadAvailability::Ready,
    );
    assert_eq!(state.pending, Some(Action::ConfirmLoad));
}

#[test]
fn escape_wins_over_confirm_and_hidden_page_actions() {
    let mut state = prompt(Screen::Main, false);
    release(&mut state);
    state.selected = 1;
    state.pointer_confirm_armed = true;
    state.input(
        &keys(&[KeyCode::Escape, KeyCode::Enter]),
        true,
        true,
        true,
        Some(Vec2::ZERO),
        true,
        &[(Screen::ConfirmLoad, Action::ConfirmLoad)],
        true,
        false,
        LoadAvailability::Ready,
    );
    assert_eq!(state.screen, Screen::Main);
    assert_eq!(state.pending, None);
    let mut state = prompt(Screen::Main, false);
    release(&mut state);
    state.input(
        &keys(&[]),
        true,
        true,
        false,
        None,
        false,
        &[
            (Screen::Main, Action::Load),
            (Screen::Settings, Action::Focus),
        ],
        true,
        false,
        LoadAvailability::Ready,
    );
    assert_eq!(state.pending, None);
    assert_eq!(state.screen, Screen::ConfirmLoad);
}

#[test]
fn busy_or_missing_runtime_never_opens_or_defers_a_prompt() {
    for availability in [LoadAvailability::Busy, LoadAvailability::Unavailable] {
        let mut state = MenuState::default();
        let outcome = state.input(
            &keys(&[KeyCode::KeyL]),
            false,
            false,
            false,
            None,
            false,
            &[],
            false,
            false,
            availability,
        );
        assert_eq!(state.screen, Screen::Closed);
        assert_eq!(state.pending, None);
        assert!(outcome.load_error.is_some());
        release(&mut state);
        assert_eq!(state.screen, Screen::Closed);
    }
}

#[test]
fn busy_keyboard_acceptance_is_discarded_until_another_explicit_action() {
    let mut state = prompt(Screen::Main, false);
    release(&mut state);
    state.selected = 1;
    let outcome = state.input(
        &keys(&[KeyCode::Enter]),
        false,
        false,
        true,
        None,
        false,
        &[],
        true,
        false,
        LoadAvailability::Busy,
    );
    assert!(outcome.load_error.is_some());
    assert_eq!(state.pending, None);
    assert_eq!(state.selected, 0);
    release(&mut state);
    assert_eq!(state.pending, None);
    assert_eq!(state.load_phase, LoadPhase::Confirming);
}

#[cfg(feature = "gpu-runtime")]
#[test]
fn acceptance_and_application_require_fresh_ready_consent_and_are_one_shot() {
    let mut state = prompt(Screen::Closed, false);
    assert!(!state.queue_confirmed_load(LoadAvailability::Ready)); // Opening gate.
    release(&mut state);
    assert!(!state.queue_confirmed_load(LoadAvailability::Busy));
    assert_eq!(state.load_phase, LoadPhase::Confirming);
    assert_eq!(state.selected, 0);
    assert!(state.release_gate);
    assert!(!state.begin_load_application(true)); // Becoming idle never queues.
    release(&mut state);
    assert!(state.queue_confirmed_load(LoadAvailability::Ready));
    assert!(!state.queue_confirmed_load(LoadAvailability::Ready));
    assert!(state.begin_load_application(true));
    assert!(!state.begin_load_application(true));
    assert!(state.busy());
    state.finish_load();
    assert_eq!(state.screen, Screen::Main);
    assert_eq!(state.load_phase, LoadPhase::Idle);
    assert!(state.captured && state.release_gate);
}

#[cfg(feature = "gpu-runtime")]
#[test]
fn unavailable_apply_and_failed_load_keep_previous_pause_ownership() {
    for paused in [false, true] {
        let mut state = prompt(Screen::Closed, paused);
        release(&mut state);
        assert!(state.queue_confirmed_load(LoadAvailability::Ready));
        assert!(!state.begin_load_application(false));
        assert_eq!(state.screen, Screen::Main);
        assert_eq!(state.close(false), !paused);
        let mut state = prompt(Screen::Closed, paused);
        release(&mut state);
        assert!(state.queue_confirmed_load(LoadAvailability::Ready));
        assert!(state.begin_load_application(true));
        state.finish_load(); // Failure does not call note_loaded_pause.
        assert_eq!(state.close(false), !paused);
    }
}

#[cfg(feature = "gpu-runtime")]
#[test]
fn simultaneous_focus_loss_survives_open_and_successful_running_save_load() {
    let mut state = MenuState::default();
    state.note_external_pause();
    assert!(state.request_load(true, LoadAvailability::Ready));
    assert!(state.external_pause);
    release(&mut state);
    assert!(state.queue_confirmed_load(LoadAvailability::Ready));
    assert!(state.begin_load_application(true));
    state.note_loaded_pause(false);
    state.finish_load();
    assert!(!state.close(false));
    let mut state = prompt(Screen::Closed, false);
    state.note_external_pause();
    assert_eq!(
        key_frame(&mut state, &[KeyCode::Escape]).resume,
        Some(false)
    );
}

#[cfg(feature = "gpu-runtime")]
#[test]
fn stale_or_newly_busy_queued_request_is_consumed_without_replacing_world() {
    for consent in [false, true] {
        let mut world = World::new();
        let old = world.spawn(Name::new("current unsaved world")).id();
        let mut state = prompt(Screen::Closed, false);
        release(&mut state);
        if consent {
            assert!(state.queue_confirmed_load(LoadAvailability::Ready));
        }
        world.insert_resource(state);
        let mut request = ProductionRuntimeLoadRequest::default();
        assert!(request.queue());
        world.insert_resource(request);
        apply_production_runtime_load(&mut world); // No runtime: application recheck fails.
        assert!(world.get::<Name>(old).is_some());
        assert!(!world.resource_mut::<ProductionRuntimeLoadRequest>().take());
        assert_ne!(
            world.resource::<MenuState>().load_phase,
            LoadPhase::Applying
        );
    }
}

#[derive(Resource, Default)]
struct ReaderRuns(usize);
fn ready_input(world: &mut World) {
    handle_input_with_availability(world, LoadAvailability::Ready);
}
fn input_app() -> App {
    let mut app = App::new();
    app.insert_resource(ux())
        .init_resource::<MenuState>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<Messages<KeyboardInput>>()
        .init_resource::<Messages<bevy::input::mouse::MouseWheel>>()
        .init_resource::<ReaderRuns>();
    #[cfg(feature = "gpu-runtime")]
    app.insert_resource(
        LiveBrainPresentationFrameResource::from_authoritative_world(
            &alife_world::HeadlessScenarioBuilder::new(21)
                .build()
                .unwrap(),
        )
        .unwrap(),
    );
    spawn_ui(app.world_mut());
    app.add_systems(
        Update,
        (
            ready_input,
            (|mut runs: ResMut<ReaderRuns>| runs.0 += 1).run_if(world_input_allowed),
        )
            .chain(),
    );
    app
}
fn frame(app: &mut App, codes: &[KeyCode], repeat: bool) {
    *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = keys(codes);
    let mut messages = app.world_mut().resource_mut::<Messages<KeyboardInput>>();
    messages.clear();
    for &code in codes {
        messages.write(KeyboardInput {
            key_code: code,
            logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
            state: ButtonState::Pressed,
            text: None,
            repeat,
            window: Entity::PLACEHOLDER,
        });
    }
    app.update();
}

#[test]
fn actual_input_blocks_opening_and_dismissal_frames_and_requires_raw_fresh_enter() {
    let mut app = input_app();
    frame(&mut app, &[], false);
    assert_eq!(app.world().resource::<ReaderRuns>().0, 1);
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .pending_food_move = Some(WorldEntityId(9));
    frame(&mut app, &[KeyCode::KeyL, KeyCode::Enter], false);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        Screen::ConfirmLoad
    );
    assert!(
        app.world()
            .resource::<Fvr05ProductionUxStateResource>()
            .settings
            .paused
    );
    assert!(app
        .world()
        .resource::<Fvr05ProductionUxStateResource>()
        .pending_food_move
        .is_none());
    frame(&mut app, &[KeyCode::Enter], true);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
    frame(&mut app, &[], false);
    frame(&mut app, &[KeyCode::ArrowDown], false);
    frame(&mut app, &[KeyCode::Enter], true);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
    frame(&mut app, &[KeyCode::Enter], false);
    assert_eq!(
        app.world().resource::<MenuState>().pending,
        Some(Action::ConfirmLoad)
    );
    frame(&mut app, &[KeyCode::Escape, KeyCode::Enter], false);
    assert_eq!(app.world().resource::<MenuState>().screen, Screen::Closed);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
    assert_eq!(app.world().resource::<ReaderRuns>().0, 1);
    frame(&mut app, &[], false); // Normal world input resumes after complete release.
    assert_eq!(app.world().resource::<ReaderRuns>().0, 2);
}

// UI-only resource metadata; this does not construct or select a GPU runtime.
fn ux() -> Fvr05ProductionUxStateResource {
    let profile = ProductionFrontendProfileId::MinimumSettings30x30;
    Fvr05ProductionUxStateResource{
        settings:Fvr05ProductionUxSettings{
            schema:"alife.fvr05.production_ux.v1".into(),schema_version:1,
            selected_profile:profile,preferred_profile_for_next_launch:profile,
            active_inspector_tab:Fvr05ProductionInspectorTab::Creature,enabled_overlays:vec![],
            camera_mode:"orbit".into(),paused:false,run_mode:None,simulation_speed:1.0,
            follow_selection:false,show_menu:false,show_settings:false,show_overlays:false,
            pause_on_focus_loss:true,selected_stable_id:None,source_save_path:String::new(),
            runtime_save_path:String::new(),created_world_save_path:String::new(),
            asset_manifest_path:String::new(),backend_descriptor:"UI-only fixture".into(),
            validation_receipt:String::new()},
        animation_speed:0.0,debug_mode:false,show_help:false,ui_settings_path:PathBuf::new(),
        source_save_path:PathBuf::new(),asset_root:PathBuf::new(),profile_id:profile,
        profile_budget:profile.budget(),population:0,resolution:(1280,720),
        save_metadata:ProductionSaveMetadata{
            save_id:"UI-only fixture".into(),deterministic_seed:21,selected_profile:profile.label().into(),
            profile_budget_version:1,save_schema:String::new(),save_schema_version:1,
            config_schema:String::new(),config_schema_version:1,object_count:0,creature_count:0,
            asset_count:0,voxel_backend_schema:None,voxel_visible_chunk_signatures:0,
            voxel_materialized_chunks:0,voxel_resource_hazard_refs:0,voxel_stable_selection_refs:0,
            voxel_dirty_region_count:0,gpu_runtime_schema:None,
            gpu_runtime_selected_backend:"Unavailable".into(),
            gpu_runtime_unavailable_reason:Some("CPU UI fixture".into()),
            gpu_runtime_no_active_bulk_readback:true,gpu_runtime_compact_action_readback_bytes:0,
            gpu_runtime_checkpoint_label:String::new()},
        selected_backend:"Unavailable".into(),adapter_name:String::new(),backend_api:String::new(),
        graphics_backend:String::new(),unavailable_reason:"CPU UI fixture".into(),
        renderer_profile:String::new(),state_trace:String::new(),
        gpu_runtime_state:serde_json::from_value(json!({
            "schema":"UI-only fixture","schema_version":1,"requested_backend_mode":"GpuAuthoritative",
            "selected_backend_mode":"Unavailable","adapter_identity":{"adapter_name":null,
                "backend_api":null,"adapter_type":null,"driver":null,"driver_info":null},
            "validation_profile":"UI-only fixture","brain_residency_slots":{"hot_slots":0,
                "warm_slots":0,"cold_slots":0},"class_bucket_allocations":[],
            "active_profile_caps":{"target_fps":0,"target_frame_ms":0.0,"renderer_reserve_ms":0.0,
                "gpu_neural_budget_ms":0.0,"neural_heap_mb":0,"staging_readback_budget_kib":0,
                "chunk_activation_radius":0,"active_chunk_cap":0,"vfx_budget":"none",
                "adaptive_throttling_order":[]},
            "shader_abi_versions":{"shader_manifest":[],"abi_manifest":[]},
            "authority":{"authoritative":false,"failure_stops_learned_actions":true,"finite_rejections":0},
            "last_safe_checkpoint":{"save_id":"UI-only fixture","world_tick":0,
                "sealed_patch_boundary":true,"checkpoint_label":"UI-only fixture"},
            "unavailable_reason":"CPU UI fixture","selected_scale_profile":"UI-only fixture",
            "compact_action_readback_bytes_per_creature":0,"no_active_bulk_readback":true})).unwrap(),
        last_action:String::new(),last_error:None,pending_food_move:None,
        #[cfg(feature="gpu-runtime")] last_manual_checkpoint_status:None,
    }
}

fn raw_frame(app: &mut App, events: &[(KeyCode, ButtonState, bool)]) {
    for &(key_code, state, repeat) in events {
        app.world_mut()
            .resource_mut::<Messages<KeyboardInput>>()
            .write(KeyboardInput {
                key_code,
                logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
                state,
                text: None,
                repeat,
                window: Entity::PLACEHOLDER,
            });
    }
    app.update();
}

#[test]
fn input_plugin_delivery_blocks_repeats_after_focus_reset_and_accepts_fresh_enter() {
    let mut app = input_app();
    app.add_plugins(bevy::input::InputPlugin);
    raw_frame(&mut app, &[]);
    raw_frame(
        &mut app,
        &[
            (KeyCode::KeyL, ButtonState::Pressed, false),
            (KeyCode::Enter, ButtonState::Pressed, false),
        ],
    );
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        Screen::ConfirmLoad
    );
    raw_frame(
        &mut app,
        &[
            (KeyCode::KeyL, ButtonState::Released, false),
            (KeyCode::Enter, ButtonState::Released, false),
        ],
    );
    raw_frame(
        &mut app,
        &[(KeyCode::ArrowDown, ButtonState::Pressed, false)],
    );
    raw_frame(
        &mut app,
        &[(KeyCode::ArrowDown, ButtonState::Released, false)],
    );
    assert_eq!(app.world().resource::<MenuState>().selected, 1);
    raw_frame(&mut app, &[(KeyCode::Enter, ButtonState::Pressed, true)]);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
    app.world_mut()
        .resource_mut::<Messages<bevy::input::keyboard::KeyboardFocusLost>>()
        .write(bevy::input::keyboard::KeyboardFocusLost);
    raw_frame(&mut app, &[]);
    raw_frame(&mut app, &[(KeyCode::Enter, ButtonState::Pressed, true)]);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
    raw_frame(&mut app, &[(KeyCode::Enter, ButtonState::Released, false)]);
    raw_frame(&mut app, &[(KeyCode::Enter, ButtonState::Pressed, false)]);
    assert_eq!(
        app.world().resource::<MenuState>().pending,
        Some(Action::ConfirmLoad)
    );
    assert_eq!(app.world().resource::<ReaderRuns>().0, 1);
}
fn activate_play(code: KeyCode) {
    let mut state = MenuState::default();
    state.open(false); // Same ordinary launch path as install().
    let outcome = key_frame(&mut state, &[code]);
    assert_eq!(
        state.screen,
        Screen::Closed,
        "{code:?} did not activate Play"
    );
    assert_eq!(outcome.resume, Some(true));
    assert!(state.captured && state.release_gate);
}

fn navigate_speed(code: KeyCode) {
    let mut state = MenuState::default();
    key_frame(&mut state, &[KeyCode::Escape]);
    for _ in 0..3 {
        key_frame(&mut state, &[KeyCode::ArrowDown]);
    }
    key_frame(&mut state, &[code]);
    assert_eq!(state.screen, Screen::Speed, "{code:?} did not open Speed");
    key_frame(&mut state, &[KeyCode::ArrowDown]);
    key_frame(&mut state, &[code]);
    assert_eq!(state.pending, Some(Action::Max));
}

fn keypad_input_app() -> App {
    let mut app = input_app();
    app.add_plugins(bevy::input::InputPlugin);
    app
}

fn load_confirmation_with_input_plugin(code: KeyCode) {
    let mut app = keypad_input_app();
    raw_frame(
        &mut app,
        &[
            (KeyCode::KeyL, ButtonState::Pressed, false),
            (code, ButtonState::Pressed, false),
        ],
    );
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        Screen::ConfirmLoad
    );
    assert_eq!(app.world().resource::<MenuState>().selected, 0);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
    raw_frame(
        &mut app,
        &[
            (KeyCode::KeyL, ButtonState::Released, false),
            (code, ButtonState::Released, false),
        ],
    );
    raw_frame(
        &mut app,
        &[(KeyCode::ArrowDown, ButtonState::Pressed, false)],
    );
    raw_frame(
        &mut app,
        &[(KeyCode::ArrowDown, ButtonState::Released, false)],
    );
    assert_eq!(app.world().resource::<MenuState>().selected, 1);
    raw_frame(&mut app, &[(code, ButtonState::Pressed, true)]);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
    app.world_mut()
        .resource_mut::<Messages<bevy::input::keyboard::KeyboardFocusLost>>()
        .write(bevy::input::keyboard::KeyboardFocusLost);
    raw_frame(&mut app, &[]);
    raw_frame(&mut app, &[(code, ButtonState::Pressed, true)]);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
    raw_frame(&mut app, &[(code, ButtonState::Released, false)]);
    raw_frame(&mut app, &[(code, ButtonState::Pressed, false)]);
    assert_eq!(
        app.world().resource::<MenuState>().pending,
        Some(Action::ConfirmLoad),
        "{code:?} fresh confirmation ignored"
    );
    raw_frame(&mut app, &[(KeyCode::Escape, ButtonState::Pressed, false)]);
    assert_eq!(app.world().resource::<MenuState>().screen, Screen::Closed);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
}

fn load_cancel_with_input_plugin(code: KeyCode) {
    let mut app = keypad_input_app();
    raw_frame(&mut app, &[(KeyCode::KeyL, ButtonState::Pressed, false)]);
    raw_frame(&mut app, &[(KeyCode::KeyL, ButtonState::Released, false)]);
    raw_frame(&mut app, &[(code, ButtonState::Pressed, false)]);
    assert_eq!(
        app.world().resource::<MenuState>().screen,
        Screen::Closed,
        "{code:?} cannot activate default Cancel"
    );
    assert_eq!(app.world().resource::<MenuState>().pending, None);
}

#[test]
fn main_enter_activates_initial_play() {
    activate_play(KeyCode::Enter);
}
#[test]
fn keypad_enter_activates_initial_play() {
    activate_play(KeyCode::NumpadEnter);
}
#[test]
fn main_enter_navigates_speed_menu() {
    navigate_speed(KeyCode::Enter);
}
#[test]
fn keypad_enter_navigates_speed_menu() {
    navigate_speed(KeyCode::NumpadEnter);
}
#[test]
fn main_enter_confirms_load_after_release_and_fresh_press() {
    load_confirmation_with_input_plugin(KeyCode::Enter);
}
#[test]
fn keypad_enter_confirms_load_after_release_and_fresh_press() {
    load_confirmation_with_input_plugin(KeyCode::NumpadEnter);
}
#[test]
fn main_enter_activates_default_cancel() {
    load_cancel_with_input_plugin(KeyCode::Enter);
}
#[test]
fn keypad_enter_activates_default_cancel() {
    load_cancel_with_input_plugin(KeyCode::NumpadEnter);
}

#[test]
fn mixed_enter_chord_activates_only_one_menu_row() {
    let mut app = keypad_input_app();
    app.world_mut().resource_mut::<MenuState>().open(false);
    for _ in 0..3 {
        raw_frame(
            &mut app,
            &[(KeyCode::ArrowDown, ButtonState::Pressed, false)],
        );
        raw_frame(
            &mut app,
            &[(KeyCode::ArrowDown, ButtonState::Released, false)],
        );
    }
    raw_frame(
        &mut app,
        &[
            (KeyCode::Enter, ButtonState::Pressed, false),
            (KeyCode::NumpadEnter, ButtonState::Pressed, false),
        ],
    );
    let state = app.world().resource::<MenuState>();
    assert_eq!(state.screen, Screen::Speed);
    assert_eq!(state.selected, 0);
    assert_eq!(state.pending, None);
    raw_frame(&mut app, &[]);
    assert_eq!(app.world().resource::<MenuState>().pending, None);
}

#[test]
fn mixed_opening_enters_require_complete_release_before_load_navigation() {
    for released in [KeyCode::Enter, KeyCode::NumpadEnter] {
        let held = if released == KeyCode::Enter {
            KeyCode::NumpadEnter
        } else {
            KeyCode::Enter
        };
        let mut app = keypad_input_app();
        raw_frame(
            &mut app,
            &[
                (KeyCode::KeyL, ButtonState::Pressed, false),
                (released, ButtonState::Pressed, false),
                (held, ButtonState::Pressed, false),
            ],
        );
        raw_frame(
            &mut app,
            &[
                (KeyCode::KeyL, ButtonState::Released, false),
                (released, ButtonState::Released, false),
            ],
        );
        raw_frame(
            &mut app,
            &[
                (KeyCode::ArrowDown, ButtonState::Pressed, false),
                (released, ButtonState::Pressed, false),
            ],
        );
        let state = app.world().resource::<MenuState>();
        assert_eq!(state.screen, Screen::ConfirmLoad);
        assert_eq!(state.selected, 0);
        assert_eq!(state.pending, None);
        assert!(state.release_gate);
        raw_frame(
            &mut app,
            &[
                (KeyCode::ArrowDown, ButtonState::Released, false),
                (released, ButtonState::Released, false),
                (held, ButtonState::Released, false),
            ],
        );
        assert!(!app.world().resource::<MenuState>().release_gate);
        raw_frame(
            &mut app,
            &[(KeyCode::ArrowDown, ButtonState::Pressed, false)],
        );
        assert_eq!(app.world().resource::<MenuState>().selected, 1);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
    }
}

fn selected_load_app() -> App {
    let mut app = keypad_input_app();
    raw_frame(&mut app, &[(KeyCode::KeyL, ButtonState::Pressed, false)]);
    raw_frame(&mut app, &[(KeyCode::KeyL, ButtonState::Released, false)]);
    raw_frame(
        &mut app,
        &[(KeyCode::ArrowDown, ButtonState::Pressed, false)],
    );
    raw_frame(
        &mut app,
        &[(KeyCode::ArrowDown, ButtonState::Released, false)],
    );
    assert_eq!(app.world().resource::<MenuState>().selected, 1);
    app
}

#[test]
fn repeat_cannot_borrow_freshness_from_the_other_held_enter() {
    for held in [KeyCode::Enter, KeyCode::NumpadEnter] {
        let repeated = if held == KeyCode::Enter {
            KeyCode::NumpadEnter
        } else {
            KeyCode::Enter
        };
        let mut app = selected_load_app();
        raw_frame(&mut app, &[(held, ButtonState::Pressed, true)]);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        raw_frame(
            &mut app,
            &[
                (held, ButtonState::Pressed, false),
                (repeated, ButtonState::Pressed, true),
            ],
        );
        assert!(!app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(held));
        assert!(app
            .world()
            .resource::<ButtonInput<KeyCode>>()
            .just_pressed(repeated));
        assert_eq!(app.world().resource::<MenuState>().pending, None);
    }
}

#[test]
fn released_enter_cannot_authorize_another_keys_repeat() {
    for released in [KeyCode::Enter, KeyCode::NumpadEnter] {
        let repeated = if released == KeyCode::Enter {
            KeyCode::NumpadEnter
        } else {
            KeyCode::Enter
        };
        let mut app = selected_load_app();
        raw_frame(
            &mut app,
            &[
                (released, ButtonState::Released, false),
                (repeated, ButtonState::Pressed, true),
            ],
        );
        assert_eq!(app.world().resource::<MenuState>().pending, None);
    }
}

#[test]
fn escape_wins_over_either_fresh_enter_or_both_together() {
    for codes in [
        vec![KeyCode::Enter],
        vec![KeyCode::NumpadEnter],
        vec![KeyCode::Enter, KeyCode::NumpadEnter],
    ] {
        let mut app = selected_load_app();
        let mut events = vec![(KeyCode::Escape, ButtonState::Pressed, false)];
        events.extend(
            codes
                .into_iter()
                .map(|code| (code, ButtonState::Pressed, false)),
        );
        raw_frame(&mut app, &events);
        assert_eq!(app.world().resource::<MenuState>().screen, Screen::Closed);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
    }
}

#[test]
fn conversation_capture_prevents_both_enter_keys_activating_the_menu() {
    for code in [KeyCode::Enter, KeyCode::NumpadEnter] {
        let mut state = MenuState::default();
        state.open(false);
        state.input(
            &keys(&[code]),
            false,
            false,
            true,
            None,
            false,
            &[],
            false,
            true,
            LoadAvailability::Ready,
        );
        assert_eq!(state.screen, Screen::Main);
        assert_eq!(state.pending, None);
    }
}

#[test]
fn gated_opening_batch_cannot_supply_later_confirmation() {
    for code in [KeyCode::Enter, KeyCode::NumpadEnter] {
        let mut app = input_app(); // Keep raw messages retained to exercise the reader cursor.
        *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() =
            keys(&[KeyCode::KeyL, KeyCode::Enter, KeyCode::NumpadEnter]);
        raw_frame(
            &mut app,
            &[
                (KeyCode::Enter, ButtonState::Pressed, false),
                (KeyCode::NumpadEnter, ButtonState::Pressed, false),
            ],
        );
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            Screen::ConfirmLoad
        );
        for codes in [&[][..], &[KeyCode::ArrowDown][..], &[code][..]] {
            *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = keys(codes);
            app.update();
        }
        assert_eq!(app.world().resource::<MenuState>().selected, 1);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        assert_eq!(app.world().resource::<Messages<KeyboardInput>>().len(), 2);
    }
}

#[test]
fn focus_reset_enter_repeats_cannot_activate_ordinary_menu_rows() {
    for code in [KeyCode::Enter, KeyCode::NumpadEnter] {
        for (screen, rows) in [
            (Screen::Main, MAIN.as_slice()),
            (Screen::Speed, SPEED.as_slice()),
            (Screen::Settings, SETTINGS.as_slice()),
        ] {
            for (selected, &(action, _)) in rows.iter().enumerate() {
                let mut app = keypad_input_app();
                // Hold Enter before opening the modal, as when opening it by pointer.
                raw_frame(&mut app, &[(code, ButtonState::Pressed, false)]);
                {
                    let mut state = app.world_mut().resource_mut::<MenuState>();
                    state.open(true);
                    state.screen = screen;
                    state.selected = selected;
                }
                raw_frame(&mut app, &[]);
                app.world_mut()
                    .resource_mut::<Messages<bevy::input::keyboard::KeyboardFocusLost>>()
                    .write(bevy::input::keyboard::KeyboardFocusLost);
                raw_frame(&mut app, &[]);
                for _ in 0..2 {
                    raw_frame(&mut app, &[(code, ButtonState::Pressed, true)]);
                    let state = app.world().resource::<MenuState>();
                    assert_eq!(state.screen, screen, "repeat activated {action:?}");
                    assert_eq!(state.selected, selected);
                    assert_eq!(state.pending, None, "repeat queued {action:?}");
                    assert!(state.captured);
                }
                // Release and a new physical press must still activate that exact row.
                raw_frame(&mut app, &[(code, ButtonState::Released, false)]);
                raw_frame(&mut app, &[(code, ButtonState::Pressed, false)]);
                let state = app.world().resource::<MenuState>();
                match action {
                    Action::Play => assert_eq!(state.screen, Screen::Closed),
                    Action::Load => assert_eq!(state.screen, Screen::ConfirmLoad),
                    Action::Speed => assert_eq!(state.screen, Screen::Speed),
                    Action::Settings => assert_eq!(state.screen, Screen::Settings),
                    Action::Back => assert_eq!(state.screen, Screen::Main),
                    _ => assert_eq!(state.pending, Some(action)),
                }
            }
        }
    }
}

#[test]
fn ordinary_menu_repeat_cannot_borrow_freshness_from_the_other_held_enter() {
    for held in [KeyCode::Enter, KeyCode::NumpadEnter] {
        let repeated = if held == KeyCode::Enter {
            KeyCode::NumpadEnter
        } else {
            KeyCode::Enter
        };
        let mut app = keypad_input_app();
        raw_frame(&mut app, &[(held, ButtonState::Pressed, false)]);
        {
            let mut state = app.world_mut().resource_mut::<MenuState>();
            state.open(true);
            state.selected = MAIN.len() - 1; // Quit must require a real new activation.
        }
        raw_frame(&mut app, &[]);
        raw_frame(
            &mut app,
            &[
                (held, ButtonState::Pressed, false),
                (repeated, ButtonState::Pressed, true),
            ],
        );
        let keyboard = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(!keyboard.just_pressed(held));
        assert!(keyboard.just_pressed(repeated));
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        raw_frame(&mut app, &[(repeated, ButtonState::Released, false)]);
        raw_frame(&mut app, &[(repeated, ButtonState::Pressed, false)]);
        assert_eq!(
            app.world().resource::<MenuState>().pending,
            Some(Action::Quit)
        );
    }
}

mod accessible_input {
    use super::*;
    use accesskit::{Action as AccessibleAction, ActionRequest as KitActionRequest, NodeId, Role};
    use bevy::a11y::{AccessibilityNode, ActionRequest};

    fn app() -> App {
        let mut app = input_app();
        app.init_resource::<Messages<ActionRequest>>()
            .add_systems(Update, sync_ui.after(ready_input));
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        frame(&mut app, &[], false);
        app
    }
    fn button(world: &mut World, screen: Screen, action: Action) -> Entity {
        world
            .query::<(Entity, &MenuButton)>()
            .iter(world)
            .find_map(|(entity, button)| {
                (button.screen == screen && button.action == action).then_some(entity)
            })
            .unwrap()
    }
    fn request(app: &mut App, screen: Screen, action: Action) {
        let target = button(app.world_mut(), screen, action);
        app.world_mut()
            .resource_mut::<Messages<ActionRequest>>()
            .write(ActionRequest(KitActionRequest {
                action: AccessibleAction::Click,
                target: NodeId(target.to_bits()),
                data: None,
            }));
    }
    fn activate(app: &mut App, screen: Screen, action: Action) {
        request(app, screen, action);
        frame(app, &[], false);
    }

    #[test]
    fn menu_entry_and_rows_expose_click_actions_and_only_the_active_page() {
        let mut app = app();
        install_cpu_ui(&mut app);
        // Bevy's automatic accessibility pass must not erase wordless names.
        frame(&mut app, &[], false);
        frame(&mut app, &[], false);
        for screen in [
            Screen::Closed,
            Screen::Main,
            Screen::Speed,
            Screen::Settings,
            Screen::ConfirmLoad,
        ] {
            app.world_mut().resource_mut::<MenuState>().screen = screen;
            frame(&mut app, &[], false);
            for (button, node) in app
                .world_mut()
                .query::<(&MenuButton, &AccessibilityNode)>()
                .iter(app.world())
            {
                assert_eq!(node.role(), Role::Button);
                assert!(node.label().is_some_and(|label| !label.is_empty()));
                assert!(node.supports_action(AccessibleAction::Click));
                assert_eq!(node.is_hidden(), button.screen != screen);
            }
            let entry = button(app.world_mut(), Screen::Closed, Action::Open);
            let node = app
                .world()
                .get::<AccessibilityNode>(entry)
                .expect("the wordless menu entry needs an accessible name");
            assert_eq!(node.label(), Some("Open player menu"));
        }
    }

    #[test]
    fn accessible_click_opens_navigates_saves_once_and_restores_world_input() {
        let mut app = app();
        let before = app.world().resource::<ReaderRuns>().0;
        activate(&mut app, Screen::Closed, Action::Open);
        assert_eq!(app.world().resource::<MenuState>().screen, Screen::Main);
        assert_eq!(app.world().resource::<ReaderRuns>().0, before);
        activate(&mut app, Screen::Main, Action::Save);
        assert_eq!(
            app.world_mut().resource_mut::<MenuState>().pending.take(),
            Some(Action::Save)
        );
        frame(&mut app, &[], false);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        activate(&mut app, Screen::Main, Action::Speed);
        assert_eq!(app.world().resource::<MenuState>().screen, Screen::Speed);
        activate(&mut app, Screen::Speed, Action::Max);
        assert_eq!(
            app.world_mut().resource_mut::<MenuState>().pending.take(),
            Some(Action::Max)
        );
        activate(&mut app, Screen::Speed, Action::Back);
        activate(&mut app, Screen::Main, Action::Settings);
        assert_eq!(app.world().resource::<MenuState>().screen, Screen::Settings);
        activate(&mut app, Screen::Settings, Action::Focus);
        assert_eq!(
            app.world_mut().resource_mut::<MenuState>().pending.take(),
            Some(Action::Focus)
        );
        activate(&mut app, Screen::Settings, Action::Back);
        activate(&mut app, Screen::Main, Action::Play);
        assert_eq!(app.world().resource::<MenuState>().screen, Screen::Closed);
        assert_eq!(app.world().resource::<ReaderRuns>().0, before);
        frame(&mut app, &[], false);
        frame(&mut app, &[], false);
        assert!(app.world().resource::<ReaderRuns>().0 > before);
    }

    #[test]
    fn accessible_load_needs_a_separate_ready_confirmation_and_escape_wins() {
        let mut app = app();
        activate(&mut app, Screen::Closed, Action::Open);
        request(&mut app, Screen::Main, Action::Load);
        request(&mut app, Screen::ConfirmLoad, Action::ConfirmLoad);
        frame(&mut app, &[], false);
        assert_eq!(
            app.world().resource::<MenuState>().screen,
            Screen::ConfirmLoad
        );
        assert_eq!(app.world().resource::<MenuState>().selected, 0);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        // A request during the opening release boundary is discarded, not deferred.
        activate(&mut app, Screen::ConfirmLoad, Action::ConfirmLoad);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        frame(&mut app, &[], false);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        activate(&mut app, Screen::ConfirmLoad, Action::ConfirmLoad);
        assert_eq!(
            app.world_mut().resource_mut::<MenuState>().pending.take(),
            Some(Action::ConfirmLoad)
        );
        request(&mut app, Screen::ConfirmLoad, Action::ConfirmLoad);
        frame(&mut app, &[KeyCode::Escape], false);
        assert_eq!(app.world().resource::<MenuState>().screen, Screen::Main);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
    }

    #[test]
    fn unfocused_busy_hidden_and_unknown_accessibility_requests_never_replay() {
        let mut app = app();
        app.world_mut().resource_mut::<MenuState>().open(false);
        frame(&mut app, &[], false);
        app.world_mut()
            .query_filtered::<&mut Window, With<PrimaryWindow>>()
            .single_mut(app.world_mut())
            .unwrap()
            .focused = false;
        activate(&mut app, Screen::Main, Action::Quit);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        app.world_mut()
            .query_filtered::<&mut Window, With<PrimaryWindow>>()
            .single_mut(app.world_mut())
            .unwrap()
            .focused = true;
        frame(&mut app, &[], false);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        activate(&mut app, Screen::Speed, Action::Max);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        let unknown = app.world_mut().spawn_empty().id();
        app.world_mut()
            .resource_mut::<Messages<ActionRequest>>()
            .write(ActionRequest(KitActionRequest {
                action: AccessibleAction::Click,
                target: NodeId(unknown.to_bits()),
                data: None,
            }));
        frame(&mut app, &[], false);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
        app.world_mut().resource_mut::<MenuState>().load_phase = LoadPhase::Queued;
        activate(&mut app, Screen::Main, Action::Save);
        app.world_mut().resource_mut::<MenuState>().load_phase = LoadPhase::Idle;
        frame(&mut app, &[], false);
        assert_eq!(app.world().resource::<MenuState>().pending, None);
    }

    #[test]
    fn menu_accessibility_updates_preserve_other_ui_and_report_busy_and_toggled_states() {
        let mut app = app();
        let other = app
            .world_mut()
            .spawn(AccessibilityNode(accesskit::Node::new(Role::Toolbar)))
            .id();
        {
            let mut ux = app
                .world_mut()
                .resource_mut::<Fvr05ProductionUxStateResource>();
            ux.settings.pause_on_focus_loss = true;
        }
        app.world_mut().resource_mut::<MenuState>().open(false);
        frame(&mut app, &[], false);
        assert!(!app
            .world()
            .get::<AccessibilityNode>(other)
            .unwrap()
            .is_hidden());
        let focus = button(app.world_mut(), Screen::Settings, Action::Focus);
        assert_eq!(
            app.world()
                .get::<AccessibilityNode>(focus)
                .unwrap()
                .toggled(),
            Some(accesskit::Toggled::True)
        );
        let save = button(app.world_mut(), Screen::Main, Action::Save);
        app.world_mut().resource_mut::<MenuState>().load_phase = LoadPhase::Queued;
        frame(&mut app, &[], false);
        assert!(app
            .world()
            .get::<AccessibilityNode>(save)
            .unwrap()
            .is_disabled());
        assert!(!app
            .world()
            .get::<AccessibilityNode>(other)
            .unwrap()
            .is_hidden());
        app.world_mut().resource_mut::<MenuState>().load_phase = LoadPhase::Idle;
        app.world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>()
            .settings
            .pause_on_focus_loss = false;
        frame(&mut app, &[], false);
        assert!(!app
            .world()
            .get::<AccessibilityNode>(save)
            .unwrap()
            .is_disabled());
        assert_eq!(
            app.world()
                .get::<AccessibilityNode>(focus)
                .unwrap()
                .toggled(),
            Some(accesskit::Toggled::False)
        );
    }

    fn install_cpu_ui(app: &mut App) {
        app.add_plugins((
            bevy::app::TaskPoolPlugin::default(),
            bevy::asset::AssetPlugin::default(),
            bevy::text::TextPlugin,
            bevy::ui::UiPlugin,
        ))
        .init_resource::<Assets<bevy::image::TextureAtlasLayout>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<bevy::input::touch::Touches>()
        .init_resource::<bevy::picking::hover::HoverMap>()
        .init_resource::<bevy::picking::events::PointerState>()
        .init_resource::<Messages<bevy::picking::pointer::PointerInput>>()
        .init_resource::<Messages<bevy::picking::backend::PointerHits>>();
    }

    #[test]
    fn actual_ui_focus_keeps_pointer_menu_open_and_save_functional() {
        let mut app = app();
        install_cpu_ui(&mut app);
        app.world_mut().spawn((
            Camera2d,
            Camera {
                computed: bevy::camera::ComputedCameraValues {
                    target_info: Some(bevy::camera::RenderTargetInfo {
                        physical_size: UVec2::new(1280, 720),
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                viewport: Some(bevy::camera::Viewport {
                    physical_size: UVec2::new(1280, 720),
                    ..default()
                }),
                ..default()
            },
            bevy::camera::RenderTarget::default(),
        ));
        // Renderer-free fixture: supply ancestor visibility and hierarchy stack;
        // geometry, cursor conversion and Interaction use actual Bevy systems.
        fn stack(world: &mut World) {
            fn walk(world: &World, entity: Entity, shown: bool, nodes: &mut Vec<(Entity, bool)>) {
                let shown = shown
                    && world
                        .get::<Node>(entity)
                        .is_none_or(|node| node.display != Display::None);
                nodes.push((entity, shown));
                if let Some(children) = world.get::<Children>(entity) {
                    for child in children.iter() {
                        walk(world, child, shown, nodes);
                    }
                }
            }
            let roots = world
                .query_filtered::<Entity, With<MenuRoot>>()
                .iter(world)
                .collect::<Vec<_>>();
            let mut nodes = vec![];
            for root in roots {
                walk(world, root, true, &mut nodes);
            }
            for &(entity, shown) in &nodes {
                world.entity_mut(entity).insert(if shown {
                    bevy::camera::visibility::InheritedVisibility::VISIBLE
                } else {
                    bevy::camera::visibility::InheritedVisibility::HIDDEN
                });
            }
            world.insert_resource(bevy::ui::UiStack {
                partition: std::iter::once(0..nodes.len()).collect(),
                uinodes: nodes.into_iter().map(|(entity, _)| entity).collect(),
            });
        }
        app.add_systems(
            Update,
            (stack, bevy::ui::ui_focus_system)
                .chain()
                .before(ready_input),
        );
        frame(&mut app, &[], false);
        frame(&mut app, &[], false);
        for (screen, action) in [(Screen::Closed, Action::Open), (Screen::Main, Action::Save)] {
            let entity = button(app.world_mut(), screen, action);
            let point = app
                .world()
                .get::<bevy::ui::UiGlobalTransform>(entity)
                .unwrap()
                .translation;
            assert!(app.world().get::<ComputedNode>(entity).unwrap().size.x > 0.0);
            app.world_mut()
                .query_filtered::<&mut Window, With<PrimaryWindow>>()
                .single_mut(app.world_mut())
                .unwrap()
                .set_cursor_position(Some(point));
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .reset_all();
            frame(&mut app, &[], false);
            assert_eq!(
                app.world().get::<Interaction>(entity),
                Some(&Interaction::Hovered)
            );
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .press(MouseButton::Left);
            frame(&mut app, &[], false);
            assert_eq!(
                app.world().get::<Interaction>(entity),
                Some(&Interaction::Pressed)
            );
            assert_eq!(app.world().resource::<MenuState>().screen, Screen::Main);
            if action == Action::Save {
                assert_eq!(
                    app.world_mut().resource_mut::<MenuState>().pending.take(),
                    Some(Action::Save)
                );
                frame(&mut app, &[], false);
                assert_eq!(app.world().resource::<MenuState>().pending, None);
            }
            app.world_mut()
                .resource_mut::<ButtonInput<MouseButton>>()
                .release(MouseButton::Left);
            frame(&mut app, &[], false);
        }
    }

    #[test]
    fn menu_rows_have_direct_accessible_dialog_parent_with_load_warning() {
        let mut app = app();
        for screen in [
            Screen::Main,
            Screen::Speed,
            Screen::Settings,
            Screen::ConfirmLoad,
        ] {
            app.world_mut().resource_mut::<MenuState>().screen = screen;
            frame(&mut app, &[], false);
            for (entity, button, parent) in app
                .world_mut()
                .query::<(Entity, &MenuButton, &ChildOf)>()
                .iter(app.world())
            {
                if button.screen != screen {
                    continue;
                }
                // Bevy's AccessKit exporter uses the immediate accessible parent
                // and immediate accessible children, not a search up the hierarchy.
                let dialog = app
                    .world()
                    .get::<AccessibilityNode>(parent.parent())
                    .expect("menu rows must not be orphaned from their dialog");
                assert_eq!(dialog.role(), Role::Dialog);
                assert!(dialog.is_modal() && !dialog.is_hidden());
                assert!(app
                    .world()
                    .get::<Children>(parent.parent())
                    .unwrap()
                    .contains(&entity));
                if screen == Screen::ConfirmLoad {
                    assert_eq!(dialog.label(), Some(LOAD_WARNING));
                }
            }
        }
    }
}
