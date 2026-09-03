//! Production keyboard and pointer input translation into runtime and presentation commands.

use super::*;

pub(super) fn handle_fvr05_production_ux_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    #[cfg(feature = "gpu-runtime")] conversation: Option<
        Res<crate::ProductionConversationLineageUiState>,
    >,
    selection: Res<Fvr03ProductionVoxelSelectionResource>,
    mut follow: ResMut<Fvr04ProductionCreatureFollowResource>,
    mut ux: ResMut<Fvr05ProductionUxStateResource>,
    #[cfg(feature = "gpu-runtime")] mut gpu_runtime: Option<
        bevy::prelude::NonSendMut<crate::bevy_shell::ProductionGpuBrainRuntimeResource>,
    >,
    #[cfg(feature = "gpu-runtime")] mut schedule: Option<
        ResMut<crate::bevy_shell::ProductionGpuBrainTickScheduleResource>,
    >,
    #[cfg(feature = "gpu-runtime")] mut load_request: ResMut<ProductionRuntimeLoadRequest>,
) {
    #[cfg(feature = "gpu-runtime")]
    if conversation
        .as_ref()
        .is_some_and(|conversation| conversation.blocks_world_shortcuts())
    {
        return;
    }
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
        let speed = schedule.speed_ticks() as f32;
        if ux.settings.paused != paused {
            ux.settings.paused = paused;
        }
        if ux.settings.simulation_speed != speed {
            ux.settings.simulation_speed = speed;
        }
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
    if keyboard.just_pressed(KeyCode::Tab) {
        ux.settings.active_inspector_tab = ux.settings.active_inspector_tab.next();
        ux.last_action = format!(
            "Inspector tab: {}",
            ux.settings.active_inspector_tab.label()
        );
    }
    if keyboard.just_pressed(KeyCode::KeyM) {
        ux.settings.show_menu = !ux.settings.show_menu;
        ux.last_action = format!("Main menu visible: {}", ux.settings.show_menu);
    }
    if keyboard.just_pressed(KeyCode::KeyG) {
        ux.settings.show_settings = !ux.settings.show_settings;
        ux.last_action = format!("Settings visible: {}", ux.settings.show_settings);
    }
    if keyboard.just_pressed(KeyCode::KeyH) {
        ux.settings.show_overlays = !ux.settings.show_overlays;
        ux.last_action = format!("Overlays visible: {}", ux.settings.show_overlays);
    }
    if keyboard.just_pressed(KeyCode::BracketLeft) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = schedule.as_deref_mut() {
            let speed = schedule.speed_ticks().saturating_sub(1);
            schedule.set_running_speed(speed);
            ux.settings.paused = schedule.is_paused();
            ux.settings.simulation_speed = schedule.speed_ticks() as f32;
        } else {
            ux.settings.simulation_speed = (ux.settings.simulation_speed * 0.5).clamp(0.10, 5.0);
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.settings.simulation_speed = (ux.settings.simulation_speed * 0.5).clamp(0.10, 5.0);
        }
        ux.last_action = format!("Simulation speed {:.2}x", ux.settings.simulation_speed);
    }
    if keyboard.just_pressed(KeyCode::BracketRight) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = schedule.as_deref_mut() {
            let speed = schedule.speed_ticks().saturating_add(1);
            schedule.set_running_speed(speed);
            ux.settings.paused = schedule.is_paused();
            ux.settings.simulation_speed = schedule.speed_ticks() as f32;
        } else {
            ux.settings.simulation_speed = (ux.settings.simulation_speed * 2.0).clamp(0.10, 5.0);
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.settings.simulation_speed = (ux.settings.simulation_speed * 2.0).clamp(0.10, 5.0);
        }
        ux.last_action = format!("Simulation speed {:.2}x", ux.settings.simulation_speed);
    }
    #[cfg(feature = "gpu-runtime")]
    for (key, speed) in [
        (KeyCode::Digit1, 1),
        (KeyCode::Digit2, 2),
        (KeyCode::Digit3, 3),
    ] {
        if keyboard.just_pressed(key) {
            if let Some(schedule) = schedule.as_deref_mut() {
                schedule.set_running_speed(speed);
                ux.settings.paused = schedule.is_paused();
                ux.settings.simulation_speed = schedule.speed_ticks() as f32;
                ux.last_action = format!("Simulation speed {:.0}x", ux.settings.simulation_speed);
            }
        }
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        #[cfg(feature = "gpu-runtime")]
        let selected_tile = selection.selected.and_then(|selected| {
            (selected.kind == StableVoxelRefKind::Tile && selected.is_stable())
                .then_some(selected.tile)
                .flatten()
        });
        #[cfg(feature = "gpu-runtime")]
        match (gpu_runtime.as_mut(), selected_tile) {
            (Some(runtime), Some(tile)) => {
                let position = Vec3f::new(tile.x as f32 + 0.5, 0.0, tile.z as f32 + 0.5);
                match runtime.runtime.place_player_food(position) {
                    Ok(receipt) => {
                        ux.last_error = None;
                        ux.last_action = format!(
                            "Placed canonical food {} at tile x={} z={}",
                            receipt.world_entity_id.raw(),
                            tile.x,
                            tile.z
                        );
                    }
                    Err(error) => {
                        ux.last_error = Some(error.to_string());
                        ux.last_action =
                            "Food placement rejected; world left unchanged".to_string();
                    }
                }
            }
            (Some(_), None) => {
                ux.last_error = Some("select a visible terrain tile first".to_string());
                ux.last_action = "Food placement rejected; world left unchanged".to_string();
            }
            (None, _) => {
                ux.last_error = Some("GPU runtime unavailable".to_string());
                ux.last_action = "Food placement unavailable".to_string();
            }
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.last_error = Some("GPU runtime unavailable".to_string());
            ux.last_action = "Food placement unavailable".to_string();
        }
    }
    if keyboard.just_pressed(KeyCode::KeyS) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(runtime) = gpu_runtime.as_mut() {
            ux.write_gpu_runtime_save(false, &mut runtime.runtime);
        } else {
            ux.write_runtime_save(false);
        }
        #[cfg(not(feature = "gpu-runtime"))]
        ux.write_runtime_save(false);
        if ux.last_error.is_none() {
            ux.persist_ui_settings();
        }
    }
    if keyboard.just_pressed(KeyCode::KeyN) {
        #[cfg(feature = "gpu-runtime")]
        if let Some(schedule) = schedule.as_deref_mut() {
            schedule.queue_step();
            ux.settings.paused = schedule.is_paused();
            ux.settings.simulation_speed = schedule.speed_ticks() as f32;
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
        if load_request.queue() {
            ux.last_error = None;
            ux.last_action = "Queued authoritative production runtime load".to_string();
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            ux.last_error = Some("GPU runtime unavailable; load was not queued".to_string());
            ux.last_action = "Load unavailable without GPU runtime".to_string();
        }
    }
    if keyboard.just_pressed(KeyCode::KeyQ) {
        ux.settings.preferred_profile_for_next_launch =
            fvr05_next_profile(ux.settings.preferred_profile_for_next_launch);
        ux.last_action = format!(
            "Preferred next-launch profile: {}",
            ux.settings.preferred_profile_for_next_launch.label()
        );
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        follow.enabled = false;
        ux.settings.show_menu = false;
        ux.settings.show_settings = false;
        ux.settings.show_overlays = false;
        ux.last_action = "Recovered the player view".to_string();
    }
    #[cfg(feature = "gpu-runtime")]
    let scheduler_speed_key = schedule.is_some()
        && [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3]
            .into_iter()
            .any(|key| keyboard.just_pressed(key));
    #[cfg(not(feature = "gpu-runtime"))]
    let scheduler_speed_key = false;
    if !scheduler_speed_key {
        if let Some(kind) = fvr05_overlay_key_pressed(&keyboard) {
            ux.toggle_overlay(kind);
        }
    }
}
