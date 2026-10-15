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
        codes.contains(&KeyCode::Enter),
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
            voxel_dirty_region_count:0,voxel_roundtrip_signatures_match:false,
            no_renderer_tokens_in_voxel_save:true,gpu_runtime_schema:None,
            gpu_runtime_selected_backend:"Unavailable".into(),
            gpu_runtime_unavailable_reason:Some("CPU UI fixture".into()),
            gpu_runtime_no_active_bulk_readback:true,gpu_runtime_compact_action_readback_bytes:0,
            gpu_runtime_checkpoint_label:String::new()},
        selected_backend:"Unavailable".into(),adapter_name:String::new(),backend_api:String::new(),
        graphics_backend:String::new(),unavailable_reason:"CPU UI fixture".into(),
        renderer_profile:String::new(),state_trace:String::new(),
        authority:Fvr05ProductionDebugAuthorityReport::production_read_only(),
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
