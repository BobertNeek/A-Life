use std::{env, path::PathBuf, process::ExitCode};

use alife_game_app::{
    load_visible_world_from_p34_save, run_cognition_debug_timeline_smoke,
    run_creature_inspector_smoke, run_creature_visual_smoke, run_feedback_polish_smoke,
    run_gpu_product_hardening_smoke, run_headless_app_shell_smoke, run_lifecycle_lineage_smoke,
    run_live_brain_loop_fixed_smoke, run_live_brain_loop_paused_smoke, run_live_brain_loop_smoke,
    run_longrun_balance_smoke, run_onboarding_help_smoke, run_platform_package_smoke,
    run_playable_survival_loop_smoke, run_population_performance_lod_smoke,
    run_population_social_loop_smoke, run_product_qa_hardening_smoke, run_release_candidate_smoke,
    run_runtime_controls_smoke, run_save_load_ux_smoke, run_school_mode_smoke,
    run_semantic_provider_smoke, run_world_ecology_loop_smoke, run_world_editor_smoke,
    validate_app_shell_config, AppShellLaunchConfig,
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
        [command, fixture_root] if command == "graphical-playground" => {
            run_graphical_playground_interactive(fixture_root)
        }
        [command, flag, seconds, fixture_root]
            if command == "graphical-playground-smoke" && flag == "--seconds" =>
        {
            let seconds = seconds
                .parse::<u32>()
                .map_err(|_| "graphical smoke seconds must be an unsigned integer".to_string())?;
            run_graphical_playground_smoke(fixture_root, seconds)
        }
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
        [command, fixture_root, ticks] if command == "runtime-controls-smoke" => {
            let ticks = ticks
                .parse::<u32>()
                .map_err(|_| "runtime control smoke ticks must be an unsigned integer".to_string())?;
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary = run_runtime_controls_smoke(&launch, ticks).map_err(|err| err.to_string())?;
            Ok(format_runtime_controls_summary(
                "S02 runtime controls",
                &summary,
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
        [command] if command == "population-social-loop-smoke" => {
            let summary = run_population_social_loop_smoke().map_err(|err| err.to_string())?;
            Ok(format_population_social_loop_summary(
                "G08 population social loop",
                &summary,
            ))
        }
        [command] if command == "lifecycle-lineage-smoke" => {
            let summary = run_lifecycle_lineage_smoke().map_err(|err| err.to_string())?;
            Ok(format_lifecycle_lineage_summary(
                "G09 lifecycle lineage",
                &summary,
            ))
        }
        [command] if command == "school-mode-smoke" => {
            let summary = run_school_mode_smoke().map_err(|err| err.to_string())?;
            Ok(format_school_mode_summary("G10 school mode", &summary))
        }
        [command] if command == "semantic-provider-smoke" => {
            let summary = run_semantic_provider_smoke().map_err(|err| err.to_string())?;
            Ok(format_semantic_provider_summary(
                "G11 semantic provider",
                &summary,
            ))
        }
        [command] if command == "gpu-product-smoke" => {
            let summary = run_gpu_product_hardening_smoke().map_err(|err| err.to_string())?;
            Ok(format_gpu_product_summary("G12 GPU product", &summary))
        }
        [command] if command == "world-editor-smoke" => {
            let summary = run_world_editor_smoke().map_err(|err| err.to_string())?;
            Ok(format_world_editor_summary("G13 world editor", &summary))
        }
        [command] if command == "cognition-debug-smoke" => {
            let panel = run_cognition_debug_timeline_smoke().map_err(|err| err.to_string())?;
            Ok(format_cognition_debug_summary(
                "G14 cognition debug",
                &panel,
            ))
        }
        [command, fixture_root] if command == "save-load-ux-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary = run_save_load_ux_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_save_load_ux_summary("G15 save/load UX", &summary))
        }
        [command, fixture_root] if command == "feedback-polish-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary = run_feedback_polish_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_feedback_polish_summary("G17 feedback polish", &summary))
        }
        [command, fixture_root] if command == "population-performance-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_population_performance_lod_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_population_performance_summary(
                "G18 population performance",
                &summary,
            ))
        }
        [command] if command == "longrun-balance-smoke" => {
            let summary = run_longrun_balance_smoke().map_err(|err| err.to_string())?;
            Ok(format_longrun_balance_summary(
                "G19 long-run balance",
                &summary,
            ))
        }
        [command] if command == "onboarding-help-smoke" => {
            let summary = run_onboarding_help_smoke().map_err(|err| err.to_string())?;
            Ok(format_onboarding_help_summary(
                "G20 onboarding help",
                &summary,
            ))
        }
        [command] if command == "platform-package-smoke" => {
            let summary = run_platform_package_smoke().map_err(|err| err.to_string())?;
            Ok(format_platform_package_summary(
                "G21 platform package",
                &summary,
            ))
        }
        [command] if command == "product-qa-smoke" => {
            let summary = run_product_qa_hardening_smoke().map_err(|err| err.to_string())?;
            Ok(format_product_qa_summary("G22 product QA", &summary))
        }
        [command] if command == "release-candidate-smoke" => {
            let summary = run_release_candidate_smoke().map_err(|err| err.to_string())?;
            Ok(format_release_candidate_summary(
                "G23 release candidate",
                &summary,
            ))
        }
        _ => Err("usage: alife_game_app headless-smoke <p34-fixture-root> | headless-paused-smoke <p34-fixture-root> | validate-config <config> <manifest> <asset-root> | bevy-smoke <p34-fixture-root> | graphical-playground <p34-fixture-root> | graphical-playground-smoke --seconds <N> <p34-fixture-root> | visible-signature <p34-fixture-root> | visible-world-smoke <p34-fixture-root> | live-brain-tick-smoke <p34-fixture-root> | live-brain-paused-smoke <p34-fixture-root> | live-brain-fixed-smoke <p34-fixture-root> <ticks> | runtime-controls-smoke <p34-fixture-root> <ticks> | creature-visual-smoke <p34-fixture-root> | creature-inspector-smoke <p34-fixture-root> | playable-survival-loop-smoke | world-ecology-loop-smoke | population-social-loop-smoke | lifecycle-lineage-smoke | school-mode-smoke | semantic-provider-smoke | gpu-product-smoke | world-editor-smoke | cognition-debug-smoke | save-load-ux-smoke <p34-fixture-root> | feedback-polish-smoke <p34-fixture-root> | population-performance-smoke <p34-fixture-root> | longrun-balance-smoke | onboarding-help-smoke | platform-package-smoke | product-qa-smoke | release-candidate-smoke".to_string()),
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

fn format_runtime_controls_summary(
    prefix: &str,
    summary: &alife_game_app::RuntimeControlSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} playback={} paused={} step={} run={} mind_tick={} world_tick={:?} action={:?}:{:?} target={:?} sealed={} sealed_patches={} packed_logs={} signature={}",
        summary.panel.schema,
        summary.panel.schema_version,
        summary.panel.playback.label(),
        summary.paused_produced,
        summary.step_produced,
        summary.run_produced,
        summary.panel.mind_tick,
        summary.panel.world_tick,
        summary.panel.selected_action_kind,
        summary.panel.selected_action_id,
        summary.panel.target_entity,
        summary.all_patches_sealed,
        summary.panel.sealed_patch_count,
        summary.panel.packed_record_count,
        summary.panel.signature_line()
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

#[cfg(feature = "bevy-app")]
fn format_graphical_playground_summary(
    prefix: &str,
    summary: &alife_game_app::bevy_shell::GraphicalPlaygroundRunSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} title='{}' mode={} timeout={:?} seed={} backend={:?} objects={} creatures={} food={} cpu_fallback={} stable_ids={} persistent={} playback={} mind_tick={} world_tick={:?} action={:?}:{:?} sealed_patches={} packed_logs={} signature={}",
        summary.launch.schema,
        summary.launch.schema_version,
        summary.launch.window_title,
        summary.launch.mode_label,
        summary.launch.smoke_seconds,
        summary.launch.seed,
        summary.launch.selected_backend,
        summary.launch.object_count,
        summary.launch.creature_marker_count,
        summary.launch.food_marker_count,
        summary.launch.cpu_fallback_visible,
        summary.launch.stable_id_overlay_visible,
        summary.launch.persistent_window,
        summary.runtime.playback.label(),
        summary.runtime.mind_tick,
        summary.runtime.world_tick,
        summary.runtime.selected_action_kind,
        summary.runtime.selected_action_id,
        summary.runtime.sealed_patch_count,
        summary.runtime.packed_record_count,
        summary.signature_line()
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
        "{prefix} schema={} version={} selected={} label={} camera_follow={:?} read_only={} action='{}' patch='{}' semantic='{}' memory_topology='{}' messages={} signature={}",
        inspector.schema,
        inspector.schema_version,
        inspector.selection.stable_id.raw(),
        inspector.selection.label,
        inspector.camera.follow_target.map(|id| id.raw()),
        inspector.read_only,
        inspector.action_summary,
        inspector.patch_summary,
        inspector.semantic_context_summary,
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

fn format_population_social_loop_summary(
    prefix: &str,
    summary: &alife_game_app::PopulationSocialLoopSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} seed={} creatures={} cap={} order={} steps={} social_samples={} heard_tokens={} collisions={} sealed_patches={} packed_logs={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.seed,
        summary.creature_count,
        summary.population_cap,
        summary
            .schedule_order
            .iter()
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join(">"),
        summary.metrics.scheduler_steps,
        summary.metrics.social_context_samples,
        summary.metrics.vocal_tokens_heard,
        summary.metrics.collision_feedback_count,
        summary.metrics.sealed_patch_count,
        summary.metrics.packed_record_count,
        summary.signature_line()
    )
}

fn format_lifecycle_lineage_summary(
    prefix: &str,
    summary: &alife_game_app::LifecycleLineageSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} seed={} living={} births={} deaths={} cap={} selected={:?} lineage={} events={} save_roundtrip={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.seed,
        summary.metrics.living_population,
        summary.metrics.births,
        summary.metrics.deaths,
        summary.population_cap,
        summary.selected_stable_id.map(|id| id.raw()),
        summary
            .lineage_records
            .iter()
            .map(alife_game_app::LifecycleLineageRecord::signature_line)
            .collect::<Vec<_>>()
            .join("|"),
        summary
            .events
            .iter()
            .map(alife_game_app::LifecycleEventRecord::signature_line)
            .collect::<Vec<_>>()
            .join("|"),
        summary.save_roundtrip_signature,
        summary.signature_line()
    )
}

fn format_school_mode_summary(prefix: &str, summary: &alife_game_app::SchoolModeSummary) -> String {
    format!(
        "{prefix} schema={} version={} seed={} curriculum={} lesson={} completed={}/{} cues={} verifier_passed={} sealed_patches={} bypass_blocked={} teacher_avatar={} learner={} sensory_tokens={} channels={} save_roundtrip={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.seed,
        summary.lesson_panel.curriculum_id,
        summary.lesson_panel.active_lesson_id.raw(),
        summary.lesson_panel.completed_steps,
        summary.lesson_panel.total_steps,
        summary
            .cues
            .iter()
            .map(alife_game_app::SchoolCuePresentation::signature_line)
            .collect::<Vec<_>>()
            .join("|"),
        summary.verifier_panel.passed,
        summary.verifier_panel.sealed_patch_count,
        summary.teacher_metadata_bypass_blocked,
        summary.teacher_avatar_stable_id.raw(),
        summary.learner_stable_id.raw(),
        summary
            .sensory_heard_tokens
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("+"),
        summary
            .sensory_teacher_channels
            .iter()
            .map(|channel| format!("{channel:?}"))
            .collect::<Vec<_>>()
            .join("+"),
        summary.save_roundtrip_signature,
        summary.signature_line()
    )
}

fn format_semantic_provider_summary(
    prefix: &str,
    summary: &alife_game_app::SemanticProviderSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} provider_schema={} provider_version={} disabled={} fake={} schema_rejected={} kind_rejected={} action_blocked={} weight_blocked={} absence_nonfatal={} failure_nonfatal={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.provider_schema,
        summary.provider_schema_version,
        summary.disabled_panel.signature_line(),
        summary.fake_panel.signature_line(),
        summary.unknown_schema_rejected,
        summary.unknown_provider_kind_rejected,
        summary.semantic_action_bypass_blocked,
        summary.weight_rewrite_blocked,
        summary.provider_absence_nonfatal,
        summary.provider_failure_nonfatal,
        summary.signature_line()
    )
}

fn format_gpu_product_summary(
    prefix: &str,
    summary: &alife_game_app::GpuProductHardeningSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} requested={} selected={} fallback={:?} feature_compiled={} no_readback={} measured_gpu={} cpu_fallback_default={} invalid_gpu_fallback={} manual_command='{}' performance_status={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.telemetry_overlay.requested_backend,
        summary.telemetry_overlay.selected_backend,
        summary.telemetry_overlay.fallback_reason,
        summary.telemetry_overlay.gpu_runtime_feature_compiled,
        summary.telemetry_overlay.no_active_gameplay_readback,
        summary.telemetry_overlay.measured_gpu_performance,
        summary.cpu_fallback_default,
        summary.invalid_gpu_config_falls_back,
        summary.manual_hardware_command,
        summary.performance_claim_status,
        summary.signature_line()
    )
}

fn format_world_editor_summary(
    prefix: &str,
    summary: &alife_game_app::WorldEditorSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} seed={} mode={} placed={} removed={} moved={} resource_rates={} invalid_rejected={} stable_ids={} resumed={} sealed={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.seed,
        summary.mode_after_edits.label(),
        summary.placed_count,
        summary.removed_count,
        summary.moved_count,
        summary.resource_rate_changes,
        summary.invalid_edit_rejected,
        summary
            .stable_ids
            .iter()
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join("+"),
        summary.simulation_resumed,
        summary.resumed_patch_sealed,
        summary.signature_line()
    )
}

fn format_cognition_debug_summary(
    prefix: &str,
    panel: &alife_game_app::CognitionDebugTimelinePanel,
) -> String {
    format!(
        "{prefix} schema={} version={} organism={} read_only={} timeline={} proposals={} sealed_only={} memory='{}' topology='{}' sleep='{}' gpu_boundary={} no_readback={} export='{}' signature={}",
        panel.schema,
        panel.schema_version,
        panel.organism_id.raw(),
        panel.read_only,
        panel.timeline_entries.len(),
        panel.proposal_lines.len(),
        panel
            .timeline_entries
            .iter()
            .all(|entry| entry.sealed_patch_only),
        panel.bias_summary.memory_expectancy_line,
        panel.bias_summary.topology_gap_line,
        panel.sleep_summary.summary_line,
        panel.gpu_summary.telemetry_boundary,
        panel.no_active_neural_readback,
        panel.packed_log_export.export_command,
        panel.signature_line()
    )
}

fn format_save_load_ux_summary(
    prefix: &str,
    summary: &alife_game_app::SaveLoadUxSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} manual_slot={} autosave_slot={} loaded={} restored_objects={} stable_ids={} overwrite_confirm={} invalid_schema={} missing_asset={} digest_error={} invalid_config={} no_partial_load={} engine_tokens_absent={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.manual_save_slot,
        summary.autosave_slot,
        summary.loaded_save_id,
        summary.restored_object_count,
        summary
            .stable_world_ids
            .iter()
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join("+"),
        summary.overwrite_confirmation_visible,
        summary.invalid_schema_error.code,
        summary.missing_asset_error.code,
        summary.digest_error.code,
        summary.invalid_config_error.code,
        summary.no_partial_load_after_error,
        summary.engine_local_token_absent,
        summary.signature_line()
    )
}

fn format_feedback_polish_summary(
    prefix: &str,
    summary: &alife_game_app::FeedbackPolishSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} events={} sealed_sources={} manifest_entries={} optional_fallbacks={} non_authoritative={} labels={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.events.len(),
        summary.sealed_outcome_event_count,
        summary.asset_manifest_entries,
        summary.optional_asset_fallbacks,
        summary.non_authoritative,
        summary.event_labels().join(">"),
        summary.signature_line()
    )
}

fn format_population_performance_summary(
    prefix: &str,
    summary: &alife_game_app::PopulationPerformanceOverlaySummary,
) -> String {
    format!(
        "{prefix} schema={} version={} creatures={} steps={} sealed={} backend={} throttle={} decimation={} lod={} golden_preserved={} tier_smoke={} manual_upper={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.population_creatures,
        summary.scheduler_steps,
        summary.sealed_patch_count,
        summary.gpu_selected_backend,
        summary.throttle_decision.throttle_level,
        summary.throttle_decision.nonessential_decimation_factor,
        summary.lod_projection.render_detail.label(),
        summary.golden_behavior_preserved,
        summary.tier_1_10_ci_smoke_documented,
        summary.manual_upper_tiers_documented,
        summary.signature_line()
    )
}

fn format_longrun_balance_summary(
    prefix: &str,
    summary: &alife_game_app::LongRunBalanceSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} cycles={} survival={:.3} energy={:.3} food={:.3} hazard_avoidance={:.3} sleep={} births={} social={:.3} sealed={} population_bound={} resource_bound={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.config.cycles,
        summary.metrics.survival_score,
        summary.metrics.energy_stability,
        summary.metrics.food_success_rate,
        summary.metrics.hazard_avoidance_score,
        summary.metrics.sleep_cycle_count,
        summary.metrics.reproduction_births,
        summary.metrics.social_diversity_score,
        summary.metrics.sealed_patch_count,
        summary.metrics.population_bounds_enforced,
        summary.metrics.resource_bounds_enforced,
        summary.signature_line()
    )
}

fn format_onboarding_help_summary(
    prefix: &str,
    summary: &alife_game_app::OnboardingHelpSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} controls={} troubleshooting={} tutorial_steps={} optional={} wrappers={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.controls.len(),
        summary.troubleshooting.len(),
        summary.tutorial_step_count,
        summary.optional_systems_remain_optional,
        summary.windows_wrappers_documented,
        summary.signature_line()
    )
}

fn format_platform_package_summary(
    prefix: &str,
    summary: &alife_game_app::PlatformPackageSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} output={} commands={} assets={} required={} optional={} artifacts_tracked={} wrappers={} release_attempted={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.output_directory,
        summary.commands.len(),
        summary.asset_bundle_entries,
        summary.required_asset_entries,
        summary.optional_asset_entries,
        summary.generated_artifacts_tracked,
        summary.windows_wrappers_used,
        summary.release_publishing_attempted,
        summary.signature_line()
    )
}

fn format_product_qa_summary(prefix: &str, summary: &alife_game_app::ProductQaSummary) -> String {
    format!(
        "{prefix} schema={} version={} checklist={} findings={} blockers={} limitations={} p36={} no_p37={} artifacts_clean={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.checklist.len(),
        summary.findings.len(),
        summary.release_blocker_count,
        summary.known_limitation_count,
        summary.p36_gates_preserved,
        summary.no_p37_created,
        summary.no_generated_artifacts_tracked,
        summary.signature_line()
    )
}

fn format_release_candidate_summary(
    prefix: &str,
    summary: &alife_game_app::ReleaseCandidateSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} candidate={} path={} gates={} automated={} manual={} blockers={} gpu={} graphics={} tag_created={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.candidate_id,
        summary.playable_supported_path,
        summary.gates.len(),
        summary.automated_gate_count,
        summary.manual_gate_count,
        summary.release_blocker_count,
        summary.gpu_performance_status,
        summary.graphics_status,
        summary.release_tag_created,
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
fn run_graphical_playground_interactive(fixture_root: &str) -> Result<String, String> {
    let launch = alife_game_app::GraphicalPlaygroundLaunchConfig::interactive(fixture_root);
    let summary =
        alife_game_app::bevy_shell::run_graphical_playground_window_with_controls(&launch)
            .map_err(|err| err.to_string())?;
    Ok(format_graphical_playground_summary(
        "S02 graphical playground closed",
        &summary,
    ))
}

#[cfg(not(feature = "bevy-app"))]
fn run_graphical_playground_interactive(_fixture_root: &str) -> Result<String, String> {
    Err("graphical-playground requires feature `bevy-app`; run `cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/p34`".to_string())
}

#[cfg(feature = "bevy-app")]
fn run_graphical_playground_smoke(fixture_root: &str, seconds: u32) -> Result<String, String> {
    let launch = alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(fixture_root, seconds);
    let summary =
        alife_game_app::bevy_shell::run_graphical_playground_window_with_controls(&launch)
            .map_err(|err| err.to_string())?;
    Ok(format_graphical_playground_summary(
        "S02 graphical playground smoke",
        &summary,
    ))
}

#[cfg(not(feature = "bevy-app"))]
fn run_graphical_playground_smoke(_fixture_root: &str, _seconds: u32) -> Result<String, String> {
    Err("graphical-playground-smoke requires feature `bevy-app`; run `cargo run -p alife_game_app --features bevy-app --bin alife_game_app -- graphical-playground-smoke --seconds 5 crates/alife_world/tests/fixtures/p34`".to_string())
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
fn run_visible_world_smoke(fixture_root: &str) -> Result<String, String> {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
    let presentation = load_visible_world_from_p34_save(&launch).map_err(|err| err.to_string())?;
    alife_game_app::compare_visible_world_to_headless(&presentation)
        .map_err(|err| err.to_string())?;
    Ok(format_visible_summary(
        "G02 visible world headless smoke",
        &presentation,
    ))
}
