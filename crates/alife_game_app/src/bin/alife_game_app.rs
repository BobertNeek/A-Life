use std::{env, path::PathBuf, process::ExitCode};

use alife_game_app::{
    load_visible_world_from_p34_save, run_creature_inspector_smoke, run_creature_visual_smoke,
    run_headless_app_shell_smoke, run_live_brain_loop_fixed_smoke,
    run_live_brain_loop_paused_smoke, run_live_brain_loop_smoke, run_playable_survival_loop_smoke,
    run_world_ecology_loop_smoke, validate_app_shell_config, AppShellLaunchConfig,
};

fn main() -> ExitCode {
    match run() {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, fixture_root] if command == "headless-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary = run_headless_app_shell_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_summary("G01 headless app shell", &summary))
        }
        [command, fixture_root] if command == "headless-paused-smoke" => {
            let mut launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            launch.start_paused = true;
            let summary = run_headless_app_shell_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_summary("G01 paused app shell", &summary))
        }
        [command, config, manifest, asset_root] if command == "validate-config" => {
            let launch = AppShellLaunchConfig {
                fixture_root: PathBuf::from(asset_root),
                config_path: PathBuf::from(config),
                asset_manifest_path: PathBuf::from(manifest),
                save_path: PathBuf::from(asset_root).join("tiny_save.json"),
                asset_root: PathBuf::from(asset_root),
                start_paused: false,
            };
            let summary = validate_app_shell_config(&launch).map_err(|err| err.to_string())?;
            Ok(format_summary("G01 validated app config", &summary))
        }
        [command, fixture_root] if command == "bevy-smoke" => run_bevy_smoke(fixture_root),
        [command, fixture_root] if command == "visible-signature" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let presentation =
                load_visible_world_from_p34_save(&launch).map_err(|err| err.to_string())?;
            Ok(format_visible_summary("G02 visible world signature", &presentation))
        }
        [command, fixture_root] if command == "visible-world-smoke" => {
            run_visible_world_smoke(fixture_root)
        }
        [command, fixture_root] if command == "live-brain-tick-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary = run_live_brain_loop_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_live_tick_summary("G03 live brain tick", &summary))
        }
        [command, fixture_root] if command == "live-brain-paused-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let (mind_tick, world_tick, produced) =
                run_live_brain_loop_paused_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format!(
                "G03 live brain paused mind_tick={} world_tick={} produced={}",
                mind_tick.raw(),
                world_tick.raw(),
                produced
            ))
        }
        [command, fixture_root, ticks] if command == "live-brain-fixed-smoke" => {
            let ticks = ticks
                .parse::<u32>()
                .map_err(|_| "ticks must be an unsigned integer".to_string())?;
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summaries =
                run_live_brain_loop_fixed_smoke(&launch, ticks).map_err(|err| err.to_string())?;
            Ok(format!(
                "G03 live brain fixed ticks={} sealed={} last_status={:?}",
                summaries.len(),
                summaries.last().map_or(0, |summary| summary.sealed_patch_count),
                summaries.last().map(|summary| summary.status)
            ))
        }
        [command, fixture_root] if command == "creature-visual-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let visual = run_creature_visual_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_creature_visual_summary("G04 creature visual", &visual))
        }
        [command, fixture_root] if command == "creature-inspector-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let inspector =
                run_creature_inspector_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_creature_inspector_summary("G05 creature inspector", &inspector))
        }
        [command] if command == "playable-survival-loop-smoke" => {
            let summary = run_playable_survival_loop_smoke().map_err(|err| err.to_string())?;
            Ok(format_playable_survival_loop_summary(
                "G06 playable survival loop",
                &summary,
            ))
        }
        [command] if command == "world-ecology-loop-smoke" => {
            let summary = run_world_ecology_loop_smoke().map_err(|err| err.to_string())?;
            Ok(format_world_ecology_loop_summary(
                "G07 world ecology loop",
                &summary,
            ))
        }
        _ => Err("usage: alife_game_app headless-smoke <p34-fixture-root> | headless-paused-smoke <p34-fixture-root> | validate-config <config> <manifest> <asset-root> | bevy-smoke <p34-fixture-root> | visible-signature <p34-fixture-root> | visible-world-smoke <p34-fixture-root> | live-brain-tick-smoke <p34-fixture-root> | live-brain-paused-smoke <p34-fixture-root> | live-brain-fixed-smoke <p34-fixture-root> <ticks> | creature-visual-smoke <p34-fixture-root> | creature-inspector-smoke <p34-fixture-root> | playable-survival-loop-smoke | world-ecology-loop-smoke".to_string()),
    }
}

fn format_summary(prefix: &str, summary: &alife_game_app::AppStartupSummary) -> String {
    format!(
        "{prefix} schema={} version={} seed={} brain={} backend={:?} assets={} states={} bevy_feature={} graphics_required={}",
        summary.schema,
        summary.schema_version,
        summary.seed,
        summary.brain_class,
        summary.requested_backend,
        summary.asset_count,
        summary.state_labels().join(">"),
        summary.bevy_feature_compiled,
        summary.graphics_required_for_default_path
    )
}

fn format_live_tick_summary(
    prefix: &str,
    summary: &alife_game_app::LiveBrainTickSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} organism={} tick={}->{} world_tick={}->{} status={:?} action={:?}:{:?} target={:?} sealed={} success={:?} contact={:?} patches={} packed_logs={}",
        summary.schema,
        summary.schema_version,
        summary.organism_id.raw(),
        summary.tick_before.raw(),
        summary.tick_after.raw(),
        summary.world_tick_before.raw(),
        summary.world_tick_after.raw(),
        summary.status,
        summary.selected_action_kind,
        summary.selected_action_id.map(|id| id.raw()),
        summary.target_entity.map(|id| id.raw()),
        summary.patch_sealed,
        summary.patch_success,
        summary.physical_contact,
        summary.sealed_patch_count,
        summary.packed_record_count
    )
}

fn format_visible_summary(
    prefix: &str,
    presentation: &alife_game_app::VisibleWorldPresentation,
) -> String {
    format!(
        "{prefix} schema={} version={} seed={} save={} objects={} signature={}",
        presentation.schema,
        presentation.schema_version,
        presentation.seed,
        presentation.save_id,
        presentation.object_count,
        presentation.visible_signature.join("|")
    )
}

fn format_creature_visual_summary(
    prefix: &str,
    visual: &alife_game_app::CreatureVisualSnapshot,
) -> String {
    format!(
        "{prefix} schema={} version={} organism={} stable={} animation={} expression={} action={:?} target={:?} cues=hunger:{:.2},fatigue:{:.2},fear:{:.2},pain:{:.2},curiosity:{:.2},energy:{:.2},sleep:{:.2} signature={}",
        visual.schema,
        visual.schema_version,
        visual.organism_id.raw(),
        visual.stable_id.raw(),
        visual.animation.label(),
        visual.expression.label(),
        visual.selected_action_kind,
        visual.target_entity.map(|id| id.raw()),
        visual.cues.hunger.value,
        visual.cues.fatigue.value,
        visual.cues.fear.value,
        visual.cues.pain.value,
        visual.cues.curiosity.value,
        visual.cues.energy.value,
        visual.cues.sleep_pressure.value,
        visual.signature_line()
    )
}

fn format_creature_inspector_summary(
    prefix: &str,
    inspector: &alife_game_app::CreatureInspectorSnapshot,
) -> String {
    format!(
        "{prefix} schema={} version={} selected={} label={} camera_follow={:?} read_only={} action='{}' patch='{}' memory_topology='{}' messages={} signature={}",
        inspector.schema,
        inspector.schema_version,
        inspector.selection.stable_id.raw(),
        inspector.selection.label,
        inspector.camera.follow_target.map(|id| id.raw()),
        inspector.read_only,
        inspector.action_summary,
        inspector.patch_summary,
        inspector.memory_topology_summary,
        inspector.troubleshooting_messages.join("|"),
        inspector.signature_line()
    )
}

fn format_playable_survival_loop_summary(
    prefix: &str,
    summary: &alife_game_app::PlayableSurvivalLoopSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} seed={} organism={} events={} sealed_patches={} packed_logs={} memory_records={} topology_concepts={} gaps={} final_animation={} final_expression={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.seed,
        summary.organism_id.raw(),
        summary.event_labels().join(">"),
        summary.sealed_patch_count,
        summary.packed_record_count,
        summary.memory_record_count,
        summary.topology_concept_count,
        summary.unresolved_gap_count,
        summary.final_visual.animation.label(),
        summary.final_visual.expression.label(),
        summary.signature_line()
    )
}

fn format_world_ecology_loop_summary(
    prefix: &str,
    summary: &alife_game_app::PlayableEcologyLoopSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} seed={} organism={} ticks={} active_resources={} regrown={} spawned={} hazard_pain={:.2} sensory_zone={:?} sealed_patches={} packed_logs={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.seed,
        summary.organism_id.raw(),
        summary.tick_summaries.len(),
        summary.metrics.active_resources,
        summary.metrics.resources_regrown,
        summary.metrics.resources_spawned,
        summary.hazard_pain,
        summary.sensory_zone_label,
        summary.sealed_patch_count,
        summary.packed_record_count,
        summary.signature_line()
    )
}

#[cfg(feature = "bevy-app")]
fn run_bevy_smoke(fixture_root: &str) -> Result<String, String> {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
    let summary = run_headless_app_shell_smoke(&launch).map_err(|err| err.to_string())?;
    let mut app = alife_game_app::bevy_shell::build_minimal_bevy_app_shell(summary.clone());
    app.update();
    Ok(format_summary("G01 Bevy app shell", &summary))
}

#[cfg(not(feature = "bevy-app"))]
fn run_bevy_smoke(_fixture_root: &str) -> Result<String, String> {
    Err("bevy-smoke requires feature `bevy-app`; run `cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- bevy-smoke crates/alife_world/tests/fixtures/p34`".to_string())
}

#[cfg(feature = "bevy-app")]
fn run_visible_world_smoke(fixture_root: &str) -> Result<String, String> {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
    let (_app, summary) = alife_game_app::bevy_shell::build_visible_world_app_shell(&launch)
        .map_err(|err| err.to_string())?;
    Ok(format!(
        "G02 visible world Bevy smoke objects={} stable_map={} ground={} signature={}",
        summary.object_count,
        summary.stable_map_count,
        summary.ground_spawned,
        summary.visible_signature.join("|")
    ))
}

#[cfg(not(feature = "bevy-app"))]
fn run_visible_world_smoke(_fixture_root: &str) -> Result<String, String> {
    Err("visible-world-smoke requires feature `bevy-app`; run `cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- visible-world-smoke crates/alife_world/tests/fixtures/p34`".to_string())
}
