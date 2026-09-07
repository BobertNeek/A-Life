//! In-engine screenshots, also usable when Windows desktop capture is unavailable.
use super::*;

#[derive(Default)]
pub(super) struct CaptureSession {
    initialized: bool,
    directory: Option<PathBuf>,
    next_at: f64,
    count: u32,
}

pub(super) fn capture_player_view(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time<bevy::time::Real>>,
    mut session: Local<CaptureSession>,
    ux: Res<Fvr05ProductionUxStateResource>,
    roots: bevy::prelude::Query<(&Fvr04ProductionCreatureVisualMarker, &Transform)>,
    players: bevy::prelude::Query<&bevy::prelude::AnimationPlayer>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    mut commands: Commands,
) {
    if !session.initialized {
        session.directory = std::env::var_os("ALIFE_GRAPHICS_CAPTURE_DIR").map(PathBuf::from);
        session.next_at = 2.0;
        session.initialized = true;
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
        "animation_players": players.iter().count(),
        "clip_times": players.iter().map(|p| p.playing_animations().map(|(_,a)| a.seek_time()).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "creatures": roots.iter().map(|(v,t)| serde_json::json!({
            "stable_id":v.stable_id.raw(), "state":format!("{:?}",v.animation),
            "position":t.translation.to_array(), "rotation":t.rotation.to_array(),
            "ground_height":surface.height(t.translation),
        })).collect::<Vec<_>>()
    });
    let _ = fs::write(directory.join(format!("{name}.json")), receipt.to_string());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(directory.join(format!("{name}.png"))));
    session.count += 1;
    session.next_at = time.elapsed_secs_f64() + 0.125;
}
