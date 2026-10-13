//! Production keyboard and pointer input translation into runtime and presentation commands.

use super::*;

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS system parameters."
)]
pub(super) fn handle_fvr05_production_ux_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut windows: bevy::prelude::Query<&mut Window, With<PrimaryWindow>>,
    frontend: Res<crate::bevy_shell::ProductionVoxelFrontendResource>,
    #[cfg(feature = "gpu-runtime")] conversation: Option<
        Res<crate::ProductionConversationLineageUiState>,
    >,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    mut follow: ResMut<Fvr04ProductionCreatureFollowResource>,
    #[cfg(feature = "gpu-runtime")] mut frame: ResMut<LiveBrainPresentationFrameResource>,
    mut ux: ResMut<Fvr05ProductionUxStateResource>,
    mut menu_state: ResMut<menu::MenuState>,
    mut dock: ResMut<care_dock::State>,
    #[cfg(feature = "gpu-runtime")] mut gpu_runtime: Option<
        bevy::prelude::NonSendMut<crate::bevy_shell::ProductionGpuBrainRuntimeResource>,
    >,
    #[cfg(feature = "gpu-runtime")] mut schedule: Option<
        ResMut<crate::bevy_shell::ProductionGpuBrainTickScheduleResource>,
    >,
    #[cfg(feature = "gpu-runtime")] mut load_request: ResMut<ProductionRuntimeLoadRequest>,
) {
    let selected_stable_id = selection
        .selected
        .and_then(|selected| selected.stable_id.map(|stable_id| stable_id.raw()));
    if ux.settings.selected_stable_id != selected_stable_id
        || ux.settings.follow_selection != follow.enabled
    {
        ux.update_selection_snapshot(selection.selected, follow.enabled);
    }
    #[cfg(feature = "gpu-runtime")]
    if let Some(schedule) = schedule.as_deref() {
        let paused = schedule.is_paused();
        let mode = schedule.run_mode();
        ux.animation_speed = schedule.animation_speed();
        for mut window in &mut windows {
            window.present_mode =
                if mode == crate::ProductionRunMode::OneX && !frontend.summary.record_performance {
                    bevy::window::PresentMode::AutoVsync
                } else {
                    bevy::window::PresentMode::Immediate
                };
        }
        if ux.settings.paused != paused {
            ux.settings.paused = paused;
        }
        ux.settings.run_mode = Some(mode);
    }
    #[cfg(feature = "gpu-runtime")]
    if let Some(runtime) = gpu_runtime.as_ref() {
        let status = runtime.runtime.manual_checkpoint_status();
        if ux.last_manual_checkpoint_status.as_ref() != Some(status) {
            let status = status.clone();
            ux.observe_gpu_runtime_save_status(&status);
            ux.last_manual_checkpoint_status = Some(status);
        }
    }
    #[cfg(feature = "gpu-runtime")]
    if ux.pending_food_move.is_some_and(|source| {
        !gpu_runtime.as_ref().is_some_and(|runtime| {
            runtime
                .runtime
                .world()
                .entity(source)
                .is_some_and(|object| {
                    object.kind == WorldObjectKind::Food
                        && !object.consumed
                        && object.carried_by.is_none()
                })
        })
    }) {
        ux.pending_food_move = None;
        ux.last_error = Some("food move source unavailable".to_string());
        ux.last_action = "Food move cancelled; source is no longer loose".to_string();
    }
    if let Some(action) = menu_state.pending.take() {
        match action {
            menu::Action::Save => menu::save(
                &mut ux,
                #[cfg(feature = "gpu-runtime")]
                gpu_runtime
                    .as_deref_mut()
                    .map(|runtime| &mut runtime.runtime),
            ),
            menu::Action::Load => {
                #[cfg(feature = "gpu-runtime")]
                menu::queue_load(
                    &mut ux,
                    &mut load_request,
                    gpu_runtime.as_deref().map(|runtime| &runtime.runtime),
                );
                #[cfg(not(feature = "gpu-runtime"))]
                {
                    ux.last_error = Some("GPU runtime unavailable; load was not queued".into());
                    ux.last_action = "Load unavailable".into();
                }
            }
            menu::Action::OneX | menu::Action::Max => menu::speed(
                if action == menu::Action::OneX {
                    crate::ProductionRunMode::OneX
                } else {
                    crate::ProductionRunMode::MaxSpeed
                },
                &mut ux,
                #[cfg(feature = "gpu-runtime")]
                schedule.as_deref_mut(),
                frontend.summary.record_performance,
                true,
            ),
            menu::Action::Focus => {
                ux.settings.pause_on_focus_loss = !ux.settings.pause_on_focus_loss;
                ux.persist_ui_settings();
            }
            menu::Action::Quit => {
                #[cfg(feature = "gpu-runtime")]
                menu::begin_quit(
                    &mut menu_state,
                    &mut ux,
                    gpu_runtime
                        .as_deref_mut()
                        .map(|runtime| &mut runtime.runtime),
                );
                #[cfg(not(feature = "gpu-runtime"))]
                {
                    ux.last_error =
                        Some("GPU runtime unavailable; quit save was not queued".into());
                    ux.last_action = "Quit unavailable".into();
                }
            }
            _ => {}
        }
    }
    if menu::is_open(&menu_state) || menu::is_captured(&menu_state) {
        return;
    }
    #[cfg(feature = "gpu-runtime")]
    if conversation
        .as_ref()
        .is_some_and(|conversation| conversation.blocks_world_shortcuts())
    {
        return;
    }
    if keyboard.just_pressed(KeyCode::Escape) && ux.pending_food_move.take().is_some() {
        ux.last_error = None;
        ux.last_action = "Food move cancelled".to_string();
    }
    if keyboard.just_pressed(KeyCode::F1) {
        ux.show_help = !ux.show_help;
    }
    if keyboard.just_pressed(KeyCode::F3) {
        ux.debug_mode = !ux.debug_mode;
        ux.settings.show_menu = ux.debug_mode;
    }
    if keyboard.just_pressed(KeyCode::Space) || keyboard.just_pressed(KeyCode::KeyP) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = schedule.as_deref_mut() {
            schedule.toggle_playback();
            ux.settings.paused = schedule.is_paused();
        } else {
            ux.settings.paused = !ux.settings.paused;
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.settings.paused = !ux.settings.paused;
        }
        ux.last_action = if ux.settings.paused {
            "Paused production simulation".to_string()
        } else {
            "Resumed production simulation".to_string()
        };
    }
    if ux.debug_mode && keyboard.just_pressed(KeyCode::Tab) {
        ux.settings.active_inspector_tab = ux.settings.active_inspector_tab.next();
        ux.last_action = format!(
            "Inspector tab: {}",
            ux.settings.active_inspector_tab.label()
        );
    }
    if ux.debug_mode && keyboard.just_pressed(KeyCode::KeyM) {
        ux.settings.show_menu = !ux.settings.show_menu;
        ux.last_action = format!("Main menu visible: {}", ux.settings.show_menu);
    }
    if ux.debug_mode && keyboard.just_pressed(KeyCode::KeyG) {
        ux.settings.show_settings = !ux.settings.show_settings;
        ux.last_action = format!("Settings visible: {}", ux.settings.show_settings);
    }
    if !ux.debug_mode && keyboard.just_pressed(KeyCode::KeyG) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(runtime) = gpu_runtime.as_mut() {
            choose_or_move_food(&selection, &mut runtime.runtime, &mut frame, &mut ux);
        } else {
            ux.last_error = Some("GPU runtime unavailable".to_string());
            ux.last_action = "Food move unavailable".to_string();
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.last_error = Some("GPU runtime unavailable".to_string());
            ux.last_action = "Food move unavailable".to_string();
        }
    }
    if ux.debug_mode && keyboard.just_pressed(KeyCode::KeyH) {
        ux.settings.show_overlays = !ux.settings.show_overlays;
        ux.last_action = format!("Overlays visible: {}", ux.settings.show_overlays);
    }
    // These are the only playback modes. Brackets are aliases, not multipliers.
    for (key, mode) in [
        (KeyCode::Digit1, crate::ProductionRunMode::OneX),
        (KeyCode::Digit2, crate::ProductionRunMode::MaxSpeed),
        (KeyCode::Digit3, crate::ProductionRunMode::HeadlessMaxSpeed),
        (KeyCode::BracketLeft, crate::ProductionRunMode::OneX),
        (KeyCode::BracketRight, crate::ProductionRunMode::MaxSpeed),
    ] {
        if keyboard.just_pressed(key) && !fvr05_overlay_modifier_pressed(&keyboard) {
            menu::speed(
                mode,
                &mut ux,
                #[cfg(feature = "gpu-runtime")]
                schedule.as_deref_mut(),
                frontend.summary.record_performance,
                false,
            );
        }
    }
    dock.observe_selection(selection.selected);
    let pointer_action = dock.take_pending();
    for action in care_dock::ACTIONS
        .into_iter()
        .filter(|a| *a != care_dock::Action::Speech)
    {
        if care_dock::requested(action, pointer_action, &keyboard, ux.debug_mode) {
            let evidence = care_dock::dispatch(
                action,
                selection.selected,
                &mut ux,
                #[cfg(feature = "gpu-runtime")]
                gpu_runtime.as_deref_mut().map(|r| &mut r.runtime),
                #[cfg(feature = "gpu-runtime")]
                &mut frame,
            );
            dock.record(action, evidence);
        }
    }
    if keyboard.just_pressed(KeyCode::KeyS) {
        menu::save(
            &mut ux,
            #[cfg(feature = "gpu-runtime")]
            gpu_runtime
                .as_deref_mut()
                .map(|runtime| &mut runtime.runtime),
        );
    }
    if ux.debug_mode && keyboard.just_pressed(KeyCode::KeyN) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = schedule.as_deref_mut() {
            schedule.queue_step();
            ux.settings.paused = schedule.is_paused();
            ux.settings.run_mode = Some(schedule.run_mode());
            ux.last_action = "Queued one production simulation step".to_string();
        } else if let Some(runtime) = gpu_runtime.as_mut() {
            ux.write_gpu_runtime_save(true, &mut runtime.runtime);
        } else {
            ux.write_runtime_save(true);
        }
        #[cfg(not(feature = "gpu-runtime"))]
        ux.write_runtime_save(true);
        #[cfg(feature = "gpu-runtime")]
        if schedule.is_none() && ux.last_error.is_none() {
            ux.persist_ui_settings();
        }
        #[cfg(not(feature = "gpu-runtime"))]
        if ux.last_error.is_none() {
            ux.persist_ui_settings();
        }
    }
    if keyboard.just_pressed(KeyCode::KeyL) {
        #[cfg(feature = "gpu-runtime")]
        menu::queue_load(
            &mut ux,
            &mut load_request,
            gpu_runtime.as_deref().map(|runtime| &runtime.runtime),
        );
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.last_error = Some("GPU runtime unavailable; load was not queued".into());
            ux.last_action = "Load unavailable".into();
        }
    }
    if ux.debug_mode && keyboard.just_pressed(KeyCode::KeyQ) {
        ux.settings.preferred_profile_for_next_launch =
            fvr05_next_profile(ux.settings.preferred_profile_for_next_launch);
        ux.last_action = format!(
            "Preferred next-launch profile: {}",
            ux.settings.preferred_profile_for_next_launch.label()
        );
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        follow.enabled = false;
        ux.debug_mode = false;
        ux.settings.show_menu = false;
        ux.settings.show_settings = false;
        ux.settings.show_overlays = false;
        ux.last_action = "Recovered the player view".to_string();
    }
    #[cfg(feature = "gpu-runtime")]
    let scheduler_speed_key = schedule.is_some()
        && !fvr05_overlay_modifier_pressed(&keyboard)
        && [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
            .into_iter()
            .any(|key| keyboard.just_pressed(key));
    #[cfg(not(feature = "gpu-runtime"))]
    let scheduler_speed_key = false;
    if ux.debug_mode && !scheduler_speed_key {
        if let Some(kind) = fvr05_overlay_key_pressed(&keyboard) {
            ux.toggle_overlay(kind);
        }
    }
}

#[cfg(feature = "gpu-runtime")]
fn choose_or_move_food(
    selection: &Fvr03ProductionVoxelSelectionResource,
    runtime: &mut crate::GpuLiveBrainRuntime,
    frame: &mut LiveBrainPresentationFrameResource,
    ux: &mut Fvr05ProductionUxStateResource,
) {
    let Some(source) = ux.pending_food_move else {
        let source = selection
            .selected
            .filter(|selected| selected.kind == StableVoxelRefKind::Resource)
            .and_then(|selected| selected.stable_id)
            .filter(|id| {
                runtime.world().entity(*id).is_some_and(|object| {
                    object.kind == WorldObjectKind::Food
                        && !object.consumed
                        && object.carried_by.is_none()
                })
            });
        if let Some(source) = source {
            ux.pending_food_move = Some(source);
            ux.last_error = None;
            ux.last_action = "Food chosen to move".to_string();
        } else {
            ux.last_error = Some("select loose food to move".to_string());
            ux.last_action = "Food move needs a source".to_string();
        }
        return;
    };
    let Some(tile) = selection
        .selected
        .filter(|selected| selected.kind == StableVoxelRefKind::Tile)
        .and_then(|selected| selected.tile)
    else {
        ux.last_error = Some("select ground for food move".to_string());
        ux.last_action = "Food move needs a destination".to_string();
        return;
    };
    let position = if runtime.world().terrain().is_some() {
        Vec3f::new(tile.x as f32 + 0.5, 0.0, tile.z as f32 + 0.5)
    } else {
        Vec3f::new(tile.x as f32 + 0.5, tile.z as f32 + 0.5, 0.0)
    };
    match runtime.move_player_food(source, position) {
        Ok(_) => {
            frame.refresh_world_objects(runtime.world());
            ux.pending_food_move = None;
            ux.last_error = None;
            ux.last_action = "Food moved".to_string();
        }
        Err(error) => {
            ux.last_error = Some(error.to_string());
            ux.last_action = "Food move rejected; world left unchanged".to_string();
        }
    }
}
