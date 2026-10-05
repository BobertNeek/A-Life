//! In-engine screenshots, also usable when Windows desktop capture is unavailable.
use super::*;
use bevy::ecs::system::SystemParam;
use bevy::prelude::{Query, ViewVisibility};
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
    interval: f64,
    maximum: u32,
}

#[derive(SystemParam)]
pub(super) struct CaptureView<'w, 's> {
    cameras:
        Query<'w, 's, (&'static Transform, &'static Projection), With<Fvr03ProductionVoxelCamera>>,
    hand: Option<Res<'w, god_hand::HandInteraction>>,
    selection: Res<'w, Fvr03ProductionVoxelSelectionResource>,
}

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS system parameters."
)]
pub(super) fn capture_player_view(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time<bevy::time::Real>>,
    mut session: Local<CaptureSession>,
    ux: Res<Fvr05ProductionUxStateResource>,
    roots: bevy::prelude::Query<(
        Entity,
        &Fvr04ProductionCreatureVisualMarker,
        &Transform,
        Option<&hearthling::HearthlingVisual>,
    )>,
    bones: Query<(
        &hearthling::HearthlingPoseBone,
        &Name,
        &Transform,
        &GlobalTransform,
    )>,
    players: bevy::prelude::Query<&bevy::prelude::AnimationPlayer>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    highlands: Option<Res<creature_grounding::SelectedTerrain>>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    #[cfg(feature = "gpu-runtime")] runtime: Option<NonSend<ProductionGpuBrainRuntimeResource>>,
    meshes: Res<Assets<Mesh>>,
    visible_meshes: Query<(&Mesh3d, &ViewVisibility)>,
    view: CaptureView,
    mut commands: Commands,
) {
    let CaptureView {
        cameras,
        hand,
        selection,
    } = view;
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
        session.maximum = std::env::var("ALIFE_GRAPHICS_CAPTURE_MAX_FRAMES")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(256)
            .clamp(1, 4096);
        session.interval = std::env::var("ALIFE_GRAPHICS_CAPTURE_INTERVAL_SECONDS")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v >= 0.125)
            .unwrap_or(0.125);
        session.initialized = true;
    }
    // Fetch existing world speech only when writing evidence, without GPU readback.
    let active_utterances = || {
        #[cfg(feature = "gpu-runtime")]
        {
            runtime
                .as_ref()
                .map_or_else(Vec::new, |r| r.runtime.active_utterances())
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            Vec::<alife_world::AudibleUtterance>::new()
        }
    };
    // Passive, bounded debug evidence. No extra neural readback or simulation changes.
    if session.action_trace.is_some() && session.traced_frames < 10_000 {
        if let Some(frame) = frame.as_ref() {
            let tick = frame.current.authoritative_world_tick.raw();
            if session.traced_tick != Some(tick) {
                let rows = frame.current.tick_summaries.iter().map(|s| {
                    let object = frame.current.objects().find(|o| o.organism_id == Some(s.organism_id));
                    let organism = object.and_then(|o| frame.current.organism(o.id));
                    let target = s.target_entity.and_then(|id| frame.current.object(id));
                    let motor_execution = s.motor_execution.as_ref().map(|trace| {
                        let receipts = trace.channel_receipts.iter().map(|receipt| {
                            let target = receipt.command.target
                                .and_then(|binding| binding.entity)
                                .and_then(|id| frame.current.object(id));
                            serde_json::json!({
                                "executed_command": &receipt.command,
                                "observation": receipt.observation,
                                "physical": receipt.physical,
                                // This is the presentation snapshot after the whole tick,
                                // not a claim about position at an individual channel boundary.
                                "target_state_after_tick": target.map(|target| serde_json::json!({
                                    "id": target.id.raw(), "kind": format!("{:?}", target.kind),
                                    "position": [target.position.x, target.position.y, target.position.z],
                                    "consumed": target.consumed,
                                    "carried_by": target.carried_by.map(|id| id.raw()),
                                })),
                            })
                        }).collect::<Vec<_>>();
                        serde_json::json!({
                            "requested_channels": &trace.requested_channels,
                            "channel_receipts": receipts,
                        })
                    });
                    serde_json::json!({
                        "organism_id": s.organism_id.raw(),
                        "stable_id": object.map(|o| o.id.raw()),
                        "tick_before": s.world_tick_before.raw(), "tick_after": s.world_tick_after.raw(),
                        "status": format!("{:?}", s.status),
                        "action": s.selected_action_kind.map(|v| format!("{v:?}")),
                        "action_id": s.selected_action_id.map(|v| v.raw()),
                        // Legacy action/target fields name the representative candidate;
                        // only motor_execution describes every executed channel.
                        "action_scope": "representative_candidate",
                        "motor_execution": motor_execution,
                        "target_id": s.target_entity.map(|v| v.raw()),
                        "target_kind": target.map(|o| format!("{:?}", o.kind)),
                        "target_position": target.map(|o| [o.position.x, o.position.y, o.position.z]),
                        "position": object.map(|o| [o.position.x, o.position.y, o.position.z]),
                        "sleep_phase": organism.map(|o| format!("{:?}", o.sleep_phase)),
                        "body_energy": organism.map(|o| o.biochemistry.body.energy),
                        "hunger": organism.map(|o| o.biochemistry.homeostasis.drives.hunger),
                        "comfort_signal": organism.map(|o| o.biochemistry.homeostasis.hormones.oxytocin),
                        "praise_signal": organism.map(|o| o.biochemistry.homeostasis.hormones.extension[0]),
                        "learning_updates": s.learning_updates,
                        "sealed": s.patch_sealed, "success": s.patch_success,
                        "contact": s.physical_contact.map(|v| format!("{v:?}")),
                        "outcome_scope": "aggregate_joint_motor_outcome",
                        "failure": s.action_failure.as_ref().map(|v| format!("{v:?}")),
                    })
                }).collect::<Vec<_>>();
                let receipt = serde_json::json!({
                    "world_tick": tick, "elapsed_seconds": time.elapsed_secs_f64(), "actions": rows,
                    "active_utterances": active_utterances(),
                    "food": frame.current.objects().filter(|o| o.kind == WorldObjectKind::Food).map(|o| serde_json::json!({
                        "id": o.id.raw(), "position": [o.position.x, o.position.y, o.position.z], "consumed": o.consumed,
                        "carried_by": o.carried_by.map(|id| id.raw()),
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
        && session.count < session.maximum
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
        "terrain_binding": highlands.as_ref().map(|t| t.0.binding()),
        "camera": cameras.iter().next().map(|(t,p)| serde_json::json!({
            "position":t.translation.to_array(),"rotation":t.rotation.to_array(),
            "view_height_m":island::view_height(p),
            "focus":highlands.as_ref().map(|s| island::focus_on_surface(t,s.0.surface()).to_array()),
        })),
        "selected_object": selection.selected.and_then(|s|s.stable_id).map(|id|id.raw()),
        "hand": hand.as_ref().map(|h|serde_json::json!({"held":h.held.map(|id|id.raw()),"contact":h.position.map(|p|p.to_array())})),
        "visible_mesh_primitives": visible_meshes.iter().filter(|(_,v)| v.get()).count(),
        "visible_mesh_triangles_before_batching": visible_meshes.iter().filter(|(_,v)| v.get())
            .filter_map(|(m,_)| meshes.get(&m.0)).map(|m| m.indices().map_or(m.count_vertices(),|i| i.len())/3).sum::<usize>(),
        "paused": ux.settings.paused, "elapsed_seconds": time.elapsed_secs_f64(),
        "last_player_action": ux.last_action,
        "last_player_error": ux.last_error,
        "world_tick": frame.as_ref().map(|f| f.current.authoritative_world_tick.raw()),
        "active_utterances": active_utterances(),
        "food": frame.as_ref().map(|f| f.current.objects().filter(|o| o.kind == WorldObjectKind::Food).map(|o| serde_json::json!({
            "id":o.id.raw(), "position":[o.position.x,o.position.y,o.position.z], "consumed":o.consumed,
        })).collect::<Vec<_>>()),
        "animation_players": players.iter().count(),
        "clip_times": players.iter().map(|p| p.playing_animations().map(|(_,a)| a.seek_time()).collect::<Vec<_>>()).collect::<Vec<_>>(),
        "creatures": roots.iter().map(|(root,v,t,visual)| serde_json::json!({
            "stable_id":v.stable_id.raw(), "organism_id":v.organism_id.raw(),
            "state":format!("{:?}",v.animation), "expression":v.expression.label(),
            "physiology":frame.as_ref().and_then(|f| f.current.organism(v.stable_id))
                .filter(|o| o.organism_id == v.organism_id).map(|o| {
                    let h = &o.biochemistry.homeostasis;
                    serde_json::json!({
                        "sleep_phase":format!("{:?}",o.sleep_phase),
                        "hunger":h.drives.hunger, "fatigue":h.drives.fatigue,
                        "fear":h.drives.fear, "pain":h.drives.pain,
                        "curiosity":h.drives.curiosity, "brain_atp":h.drives.brain_atp,
                        "sleep_pressure":h.hormones.sleep_pressure,
                    })
                }),
            "outcome_feedback":visual.map(hearthling::HearthlingVisual::capture_feedback),
            "overlays_suppressed_for_sleep":v.animation == CreatureAnimationState::Sleeping,
            "body_yaw":v.body_yaw, "head_yaw":v.head_yaw,
            "articulated_pose":bones.iter().filter(|(bone,_,_,_)| bone.capture_root() == root)
                .map(|(bone,name,local,global)| serde_json::json!({
                    "name":name.as_str(), "bind_rotation":bone.capture_bind_rotation(),
                    "local_translation":local.translation.to_array(),
                    "local_rotation":local.rotation.to_array(), "local_scale":local.scale.to_array(),
                    "world_matrix":global.to_matrix().to_cols_array_2d(),
                })).collect::<Vec<_>>(),
            "position":t.translation.to_array(), "rotation":t.rotation.to_array(),
            "authoritative_position":frame.as_ref().and_then(|f|f.current.object(v.stable_id))
                .map(|o|[o.position.x,o.position.y,o.position.z]),
            "ground_height":surface.height(t.translation),
            "world_position":frame.as_ref().and_then(|f| f.current.object(v.stable_id)).map(|o| [o.position.x,o.position.y,o.position.z]),
        })).collect::<Vec<_>>()
    });
    let _ = fs::write(directory.join(format!("{name}.json")), receipt.to_string());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(directory.join(format!("{name}.png"))));
    session.count += 1;
    session.next_at = time.elapsed_secs_f64() + session.interval;
}
