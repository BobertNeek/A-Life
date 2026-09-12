//! In-engine screenshots, also usable when Windows desktop capture is unavailable.
use super::*;
use std::io::Write;

#[derive(Default)]
pub(super) struct CaptureSession {
    initialized: bool,
    directory: Option<PathBuf>,
    next_at: f64,
    count: u32,
    action_trace: Option<fs::File>,
    traced_tick: Option<u64>,
    traced_frames: u32,
}

pub(super) fn capture_player_view(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time<bevy::time::Real>>,
    mut session: Local<CaptureSession>,
    ux: Res<Fvr05ProductionUxStateResource>,
    roots: bevy::prelude::Query<(&Fvr04ProductionCreatureVisualMarker, &Transform)>,
    players: bevy::prelude::Query<&bevy::prelude::AnimationPlayer>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    mut commands: Commands,
) {
    if !session.initialized {
        session.directory = std::env::var_os("ALIFE_GRAPHICS_CAPTURE_DIR").map(PathBuf::from);
        if let Some(path) = std::env::var_os("ALIFE_ACTION_TRACE_PATH") {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(file) => session.action_trace = Some(file),
                Err(error) => eprintln!("Action trace could not open: {error}"),
            }
        }
        session.next_at = 2.0;
        session.initialized = true;
    }
    // Passive, bounded debug evidence. No extra neural readback or simulation changes.
    if session.action_trace.is_some() && session.traced_frames < 10_000 {
        if let Some(frame) = frame.as_ref() {
            let tick = frame.current.authoritative_world_tick.raw();
            if session.traced_tick != Some(tick) {
                let rows = frame.current.tick_summaries.iter().map(|s| {
                    let object = frame.current.objects().find(|o| o.organism_id == Some(s.organism_id));
                    let organism = object.and_then(|o| frame.current.organism(o.id));
                    let target = s.target_entity.and_then(|id| frame.current.object(id));
                    serde_json::json!({
                        "organism_id": s.organism_id.raw(),
                        "tick_before": s.world_tick_before.raw(), "tick_after": s.world_tick_after.raw(),
                        "status": format!("{:?}", s.status),
                        "action": s.selected_action_kind.map(|v| format!("{v:?}")),
                        "action_id": s.selected_action_id.map(|v| v.raw()),
                        "target_id": s.target_entity.map(|v| v.raw()),
                        "target_kind": target.map(|o| format!("{:?}", o.kind)),
                        "target_position": target.map(|o| [o.position.x, o.position.y, o.position.z]),
                        "position": object.map(|o| [o.position.x, o.position.y, o.position.z]),
                        "sleep_phase": organism.map(|o| format!("{:?}", o.sleep_phase)),
                        "sealed": s.patch_sealed, "success": s.patch_success,
                        "contact": s.physical_contact.map(|v| format!("{v:?}")),
                        "failure": s.action_failure.as_ref().map(|v| format!("{v:?}")),
                    })
                }).collect::<Vec<_>>();
                let receipt = serde_json::json!({
                    "world_tick": tick, "elapsed_seconds": time.elapsed_secs_f64(), "actions": rows,
                    "food": frame.current.objects().filter(|o| o.kind == WorldObjectKind::Food).map(|o| serde_json::json!({
                        "id": o.id.raw(), "position": [o.position.x, o.position.y, o.position.z], "consumed": o.consumed,
                    })).collect::<Vec<_>>(),
                });
                if let Err(error) = writeln!(session.action_trace.as_mut().unwrap(), "{receipt}") {
                    eprintln!("Action trace stopped: {error}");
                    session.action_trace = None;
                }
                session.traced_tick = Some(tick);
                session.traced_frames += 1;
            }
        }
    }
    let automatic = session.directory.is_some()
        && session.count < 96
        && time.elapsed_secs_f64() >= session.next_at
        && !players.is_empty();
    if !keyboard.just_pressed(KeyCode::F12) && !automatic {
        return;
    }
    let directory = session
        .directory
        .clone()
        .unwrap_or_else(|| crate::ca12_workspace_root().join("target/artifacts/player-captures"));
    if fs::create_dir_all(&directory).is_err() {
        return;
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let name = if automatic {
        format!("frame-{:04}", session.count)
    } else {
        format!("view-{stamp}")
    };
    let receipt = serde_json::json!({
        "paused": ux.settings.paused, "elapsed_seconds": time.elapsed_secs_f64(),
        "world_tick": frame.as_ref().map(|f| f.current.authoritative_world_tick.raw()),
        "food": frame.as_ref().map(|f| f.current.objects().filter(|o| o.kind == WorldObjectKind::Food).map(|o| serde_json::json!({
            "id":o.id.raw(), "position":[o.position.x,o.position.y,o.position.z], "consumed":o.consumed,
        })).collect::<Vec<_>>()),
        "animation_players": players.iter().count(),
        "clip_times": players.iter().map(|p| p.playing_animations().map(|(_,a)| a.seek_time()).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "creatures": roots.iter().map(|(v,t)| serde_json::json!({
            "stable_id":v.stable_id.raw(), "state":format!("{:?}",v.animation),
            "position":t.translation.to_array(), "rotation":t.rotation.to_array(),
            "ground_height":surface.height(t.translation),
            "world_position":frame.as_ref().and_then(|f| f.current.object(v.stable_id)).map(|o| [o.position.x,o.position.y,o.position.z]),
        })).collect::<Vec<_>>()
    });
    let _ = fs::write(directory.join(format!("{name}.json")), receipt.to_string());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(directory.join(format!("{name}.png"))));
    session.count += 1;
    session.next_at = time.elapsed_secs_f64() + 0.125;
}
