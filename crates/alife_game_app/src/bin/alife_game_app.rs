use std::{env, path::PathBuf, process::ExitCode};

use alife_game_app::{
    default_app_bundle_manifest_path, load_visible_world_from_p34_save,
    run_advanced_gameplay_ux_smoke, run_affordance_loop_smoke, run_batched_gpu_runtime_smoke,
    run_behavior_comparison_lab_smoke, run_behavior_tuning_metrics_smoke,
    run_cognition_debug_timeline_smoke, run_content_authoring_smoke,
    run_creature_animation_state_machine_smoke, run_creature_inspector_smoke,
    run_creature_visual_smoke, run_curriculum_authoring_smoke,
    run_curriculum_authoring_smoke_with_manifest, run_double_buffered_scheduler_smoke,
    run_drive_coupled_audio_vfx_smoke, run_ecological_soak_smoke, run_environment_launcher_smoke,
    run_feedback_polish_smoke, run_full_gpu_runtime_smoke,
    run_gpu_graphics_performance_evidence_smoke, run_gpu_longrun_soak,
    run_gpu_product_hardening_smoke, run_gpu_sustained_learning_soak, run_graphical_controls_smoke,
    run_graphical_ecology_smoke, run_graphical_lifecycle_smoke, run_graphical_population_smoke,
    run_graphical_save_load_menu_smoke, run_graphical_school_mode_smoke, run_hazard_recovery_smoke,
    run_headless_app_shell_smoke, run_homeostasis_runtime_smoke, run_internal_slm_prior_smoke,
    run_lifecycle_lineage_smoke, run_live_brain_loop_fixed_smoke, run_live_brain_loop_paused_smoke,
    run_live_brain_loop_smoke, run_longrun_balance_smoke, run_memory_history_journal_smoke,
    run_motor_ring_arbitration_smoke, run_multi_hour_soak_isolation_smoke,
    run_neural_activity_profiler_smoke, run_onboarding_help_smoke, run_platform_package_smoke,
    run_playable_survival_loop_smoke, run_player_sandbox_editor_smoke,
    run_population_performance_lod_smoke, run_population_social_loop_smoke,
    run_product_qa_hardening_smoke, run_real_semantic_provider_smoke,
    run_realtime_wgsl_telemetry_smoke, run_release_candidate_smoke, run_runtime_controls_smoke,
    run_sampled_gpu_runtime_smoke, run_save_load_ux_smoke, run_school_mode_smoke,
    run_semantic_provider_smoke, run_teacher_world_cues_smoke,
    run_topological_concept_overlay_smoke, run_world_art_style_smoke, run_world_ecology_loop_smoke,
    run_world_editor_smoke, validate_app_bundle_manifest, validate_app_shell_config,
    write_behavior_comparison_lab_report, write_ca36_soak_isolation_report, AppShellLaunchConfig,
    BatchedGpuRuntimeOptions, EnvironmentManifest, FullGpuRuntimeSmokeMode,
    FullGpuRuntimeSmokeOptions, GpuLongrunSoakOptions, GpuSustainedLearningSoakOptions,
    SampledGpuRuntimeOptions,
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
        [command, rest @ ..] if command == "list-environments" => {
            run_list_environments_cli(rest)
        }
        [command, rest @ ..] if command == "environment-launch-smoke" => {
            run_environment_launch_smoke_cli(rest)
        }
        [command, fixture_root] if command == "bevy-smoke" => run_bevy_smoke(fixture_root),
        [command, rest @ ..] if command == "graphical-playground" => {
            run_graphical_playground_cli(rest)
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
        [command, fixture_root] if command == "graphical-controls-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_graphical_controls_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_graphical_controls_summary(
                "Alpha graphical controls",
                &summary,
            ))
        }
        [command, fixture_root] if command == "topological-concept-overlay-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_topological_concept_overlay_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format!(
                "CA28 topological concept overlay schema={} version={} nodes={} edges={} gaps={} events={} bypass_blocked={} direct_mutation={} signature={}",
                summary.snapshot.schema,
                summary.snapshot.schema_version,
                summary.snapshot.concept_count,
                summary.snapshot.edge_count,
                summary.snapshot.gap_count,
                summary.snapshot.event_links.len(),
                summary.topology_action_bypass_blocked,
                summary.direct_cognition_mutation_allowed,
                summary.snapshot.signature_line()
            ))
        }
        [command, fixture_root] if command == "memory-history-journal-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_memory_history_journal_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format!(
                "CA29 memory history journal schema={} version={} memories={} patches={} bias_rows={} action_replay_blocked={} direct_mutation={} signature={}",
                summary.snapshot.schema,
                summary.snapshot.schema_version,
                summary.snapshot.memory_record_count,
                summary.snapshot.recent_patches.len(),
                summary.snapshot.expectancy_rows.len(),
                summary.action_replay_blocked,
                summary.direct_cognition_mutation_allowed,
                summary.snapshot.signature_line()
            ))
        }
        [command, fixture_root] if command == "neural-activity-profiler-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_neural_activity_profiler_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format!(
                "CA30 neural activity profiler schema={} version={} lobes={} tiles={}/{} syn={}/{} backend={} bulk_readback_blocked={} action_authority_blocked={} weight_mutation_blocked={} signature={}",
                summary.snapshot.schema,
                summary.snapshot.schema_version,
                summary.snapshot.lobe_rows.len(),
                summary.snapshot.tile_summary.active_tiles,
                summary.snapshot.tile_summary.max_active_tiles,
                summary.snapshot.tile_summary.active_synapses,
                summary.snapshot.tile_summary.max_active_synapses,
                summary.snapshot.route_status.selected_backend,
                summary.bulk_readback_blocked,
                summary.action_authority_blocked,
                summary.weight_mutation_blocked,
                summary.snapshot.signature_line()
            ))
        }
        [command, fixture_root] if command == "realtime-wgsl-telemetry-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_realtime_wgsl_telemetry_smoke(&launch, 3).map_err(|err| err.to_string())?;
            Ok(format!(
                "CA32 realtime WGSL telemetry schema={} version={} ticks={}/{} backend={} fallback={:?} available={} up_ms={:.3} compute_ms={:.3} read_ms={:.3} cpu_shadow_ms={:.3} tiles={}/{} skipped={} syn={} readback={}B nonblocking={} cpu_shadow_gate={} parity={} scores={} h_shadow_apps={} no_bulk_readback={} full_action_authoritative=false",
                summary.schema,
                summary.schema_version,
                summary.ticks_completed,
                summary.requested_ticks,
                summary.selected_backend,
                summary.fallback_reason,
                summary.telemetry.timing_available,
                summary.telemetry.upload_ms,
                summary.telemetry.compute_submit_poll_ms,
                summary.telemetry.compact_readback_ms,
                summary.telemetry.cpu_shadow_ms,
                summary.telemetry.routing_active_tiles,
                summary.telemetry.routing_total_tiles,
                summary.telemetry.routing_skipped_tiles,
                summary.telemetry.routing_active_synapses,
                summary.telemetry.compact_readback_bytes,
                summary.telemetry.nonblocking_hot_path,
                summary.cpu_shadow_gate,
                summary.cpu_shadow_parity,
                summary.gpu_scores_used_for_proposals,
                summary.h_shadow_applications,
                summary.no_active_bulk_readback,
            ))
        }
        [command, rest @ ..] if command == "behavior-comparison-lab-smoke" => {
            run_behavior_comparison_lab_cli(rest)
        }
        [command, fixture_root] if command == "graphical-population-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_graphical_population_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format!(
                "CA18 graphical population schema={} version={} creatures={} cap={} selected={} cues={} claim={} signature={}",
                summary.schema,
                summary.schema_version,
                summary.creature_count,
                summary.population_cap,
                summary.selected_stable_id.raw(),
                summary.social_cues.len(),
                summary.product_runtime_claim,
                summary.signature_line()
            ))
        }
        [command, fixture_root] if command == "graphical-ecology-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary = run_graphical_ecology_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format!(
                "CA19 graphical ecology schema={} version={} zones={} resources={} active={} regrown={} spawned={} hazard_zones={} roundtrip={} claim={} signature={}",
                summary.schema,
                summary.schema_version,
                summary.terrain_zones.len(),
                summary.resources.len(),
                summary.cycled_metrics.active_resources,
                summary.cycled_metrics.resources_regrown,
                summary.cycled_metrics.resources_spawned,
                summary.hazard_pressure_zone_count,
                summary.save_load_roundtrip_preserved,
                summary.product_runtime_claim,
                summary.signature_line()
            ))
        }
        [command, fixture_root] if command == "world-art-style-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary = run_world_art_style_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format!(
                "CA37 world art schema={} version={} seed={} palette={} props={} tiles={} viewport={}x{} ratio={:.1} offscreen_objects={} span_world_units={:.1} large_world_exploration={} distributed_objects={} zones={} resource_materials={} hazard_materials={} manifest_validated={} placeholder_art_entries={} display_only={} claim={} signature={}",
                summary.schema,
                summary.schema_version,
                summary.seed,
                summary.palette.len(),
                summary.dressing_props.len(),
                summary.visual_map_tile_count,
                summary.viewport_width_tiles,
                summary.viewport_height_tiles,
                summary.map_to_viewport_tile_ratio,
                summary.offscreen_stable_world_object_count,
                summary.visual_map_span_world_units,
                summary.true_large_world_exploration,
                summary.distributed_stable_world_objects,
                summary.ecology_zone_count,
                summary.resource_zone_materials,
                summary.hazard_zone_materials,
                summary.app_bundle_manifest_validated,
                summary.placeholder_art_entries,
                summary.display_only,
                summary.product_runtime_claim,
                summary.signature_line()
            ))
        }
        [command] if command == "creature-animation-state-smoke" => {
            let summary =
                run_creature_animation_state_machine_smoke().map_err(|err| err.to_string())?;
            Ok(format!(
                "CA38 creature animation schema={} version={} states={} display_only={} fallback_visible={} stable_ids_only={} no_action_authority={} no_cognition_mutation={} claim={} signature={}",
                summary.schema,
                summary.schema_version,
                summary.states.len(),
                summary.display_only,
                summary.fallback_visible,
                summary.stable_ids_only,
                summary.no_action_authority,
                summary.no_cognition_mutation,
                summary.product_runtime_claim,
                summary.signature_line()
            ))
        }
        [command, fixture_root] if command == "double-buffered-scheduler-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_double_buffered_scheduler_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_double_buffered_scheduler_summary(
                "CA13 double-buffered scheduler",
                &summary,
            ))
        }
        [command, fixture_root] if command == "motor-ring-arbitration-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_motor_ring_arbitration_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_motor_ring_arbitration_summary(
                "CA14 motor ring arbitration",
                &summary,
            ))
        }
        [command, fixture_root] if command == "homeostasis-runtime-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_homeostasis_runtime_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_homeostasis_runtime_summary(
                "CA15 homeostasis runtime",
                &summary,
            ))
        }
        [command, fixture_root] if command == "affordance-loop-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_affordance_loop_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_affordance_loop_summary(
                "CA16 affordance loop",
                &summary,
            ))
        }
        [command, fixture_root] if command == "hazard-recovery-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_hazard_recovery_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_hazard_recovery_summary(
                "CA17 hazard recovery",
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
        [command] if command == "graphical-lifecycle-smoke" => {
            let summary = run_graphical_lifecycle_smoke().map_err(|err| err.to_string())?;
            Ok(format_graphical_lifecycle_summary(
                "CA20 graphical lifecycle",
                &summary,
            ))
        }
        [command] if command == "school-mode-smoke" => {
            let summary = run_school_mode_smoke().map_err(|err| err.to_string())?;
            Ok(format_school_mode_summary("G10 school mode", &summary))
        }
        [command] if command == "graphical-school-mode-smoke" => {
            let summary = run_graphical_school_mode_smoke().map_err(|err| err.to_string())?;
            Ok(format!(
                "CA23 graphical school schema={} version={} school={} toggle={} teacher={} learner={} lesson={} verifier_sealed={} verifier_passed={} cues={} perception_only={} bypass_blocked={} hidden_vectors_blocked={} display_only={} signature={}",
                summary.schema,
                summary.schema_version,
                summary.school_enabled,
                summary.toggle_key,
                summary.teacher_avatar_stable_id.raw(),
                summary.learner_stable_id.raw(),
                summary.active_lesson_id,
                summary.sealed_patch_count,
                summary.verifier_passed,
                summary.cue_markers.len(),
                summary.perception_only_boundary_visible,
                summary.direct_motor_bypass_blocked,
                summary.hidden_vector_injection_blocked,
                summary.display_only,
                summary.signature_line()
            ))
        }
        [command] if command == "teacher-world-cues-smoke" => {
            let summary = run_teacher_world_cues_smoke().map_err(|err| err.to_string())?;
            Ok(format!(
                "CA24 teacher world cues schema={} version={} teacher={} learner={} lesson={} cues={} visible_world={} audible_tokens={} gestures={} sensory_environmental={} sealed_verifier={} bypass_rejected={} hidden_vectors_blocked={} no_action_authority={} signature={}",
                summary.schema,
                summary.schema_version,
                summary.teacher_avatar_stable_id.raw(),
                summary.learner_stable_id.raw(),
                summary.active_lesson_id,
                summary.cue_objects.len(),
                summary.visible_world_events,
                summary.audible_token_events,
                summary.gesture_events,
                summary.lesson_conditions_are_sensory_environmental,
                summary.verifier_uses_sealed_patches,
                summary.direct_motor_bypass_rejected,
                summary.hidden_vector_injection_blocked,
                summary.no_action_authority,
                summary.signature_line()
            ))
        }
        [command] if command == "curriculum-authoring-smoke" => {
            let summary = run_curriculum_authoring_smoke().map_err(|err| err.to_string())?;
            Ok(format_curriculum_authoring_summary(
                "CA25 curriculum authoring",
                &summary,
            ))
        }
        [command, manifest_path] if command == "curriculum-authoring-smoke" => {
            let summary = run_curriculum_authoring_smoke_with_manifest(manifest_path)
                .map_err(|err| err.to_string())?;
            Ok(format_curriculum_authoring_summary(
                "CA25 curriculum authoring",
                &summary,
            ))
        }
        [command] if command == "semantic-provider-smoke" => {
            let summary = run_semantic_provider_smoke().map_err(|err| err.to_string())?;
            Ok(format_semantic_provider_summary(
                "G11 semantic provider",
                &summary,
            ))
        }
        [command] if command == "real-semantic-provider-smoke" => {
            let summary = run_real_semantic_provider_smoke().map_err(|err| err.to_string())?;
            Ok(format_real_semantic_provider_summary(
                "CA26 real semantic provider",
                &summary,
            ))
        }
        [command] if command == "llamacpp-semantic-provider-smoke" => {
            let summary = run_real_semantic_provider_smoke().map_err(|err| err.to_string())?;
            Ok(format_real_semantic_provider_summary(
                "llama.cpp semantic provider",
                &summary,
            ))
        }
        [command] if command == "internal-slm-prior-smoke" => {
            let summary = run_internal_slm_prior_smoke().map_err(|err| err.to_string())?;
            Ok(format_internal_slm_prior_summary(
                "CA27 internal SLM prior",
                &summary,
            ))
        }
        [command] if command == "llamacpp-slm-prior-smoke" => {
            let summary = run_internal_slm_prior_smoke().map_err(|err| err.to_string())?;
            Ok(format_internal_slm_prior_summary(
                "llama.cpp SLM prior",
                &summary,
            ))
        }
        [command] if command == "llamacpp-local-model-runtime-smoke" => {
            let semantic =
                run_real_semantic_provider_smoke().map_err(|err| err.to_string())?;
            let slm = run_internal_slm_prior_smoke().map_err(|err| err.to_string())?;
            Ok(format!(
                "llama.cpp local model runtime semantic=pass slm=pass runtime=llamacpp-server-gguf semantic_alias={} semantic_port={} slm_alias={} slm_port={} no_cloud=true no_actions=true no_weight_rewrite=true semantic_signature={} slm_signature={}",
                semantic.llamacpp_alias,
                semantic.llamacpp_port,
                slm.llamacpp_alias,
                slm.llamacpp_port,
                semantic.signature_line(),
                slm.signature_line()
            ))
        }
        [command] if command == "advanced-gameplay-ux-smoke" => {
            let summary = run_advanced_gameplay_ux_smoke().map_err(|err| err.to_string())?;
            Ok(format_advanced_gameplay_summary(
                "S07 advanced gameplay UX",
                &summary,
            ))
        }
        [command] if command == "gpu-product-smoke" => {
            let summary = run_gpu_product_hardening_smoke().map_err(|err| err.to_string())?;
            Ok(format_gpu_product_summary("G12 GPU product", &summary))
        }
        [command, rest @ ..] if command == "full-gpu-runtime-smoke" => {
            run_full_gpu_runtime_cli(rest)
        }
        [command, rest @ ..] if command == "batched-gpu-runtime-smoke" => {
            run_batched_gpu_runtime_cli(rest)
        }
        [command, rest @ ..] if command == "sampled-gpu-runtime-smoke" => {
            run_sampled_gpu_runtime_cli(rest)
        }
        [command, rest @ ..] if command == "gpu-longrun-soak" => run_gpu_longrun_soak_cli(rest),
        [command, rest @ ..] if command == "gpu-sustained-learning-soak" => {
            run_gpu_sustained_learning_soak_cli(rest)
        }
        [command, rest @ ..] if command == "multi-hour-soak-isolation-smoke" => {
            run_multi_hour_soak_isolation_cli(rest)
        }
        [command, fixture_root] if command == "gpu-graphics-performance-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary = run_gpu_graphics_performance_evidence_smoke(&launch)
                .map_err(|err| err.to_string())?;
            Ok(format_gpu_graphics_performance_summary(
                "S08 GPU graphics performance",
                &summary,
            ))
        }
        [command] if command == "world-editor-smoke" => {
            let summary = run_world_editor_smoke().map_err(|err| err.to_string())?;
            Ok(format_world_editor_summary("G13 world editor", &summary))
        }
        [command, rest @ ..] if command == "player-sandbox-editor-smoke" => {
            run_player_sandbox_editor_cli(rest)
        }
        [command, rest @ ..] if command == "app-bundle-smoke" => {
            run_app_bundle_smoke_cli(rest)
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
        [command, fixture_root] if command == "graphical-save-load-menu-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_graphical_save_load_menu_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_graphical_save_load_menu_summary(
                "CA09 graphical save/load menu",
                &summary,
            ))
        }
        [command, fixture_root] if command == "feedback-polish-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary = run_feedback_polish_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_feedback_polish_summary("G17 feedback polish", &summary))
        }
        [command, fixture_root] if command == "drive-coupled-audio-vfx-smoke" => {
            let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
            let summary =
                run_drive_coupled_audio_vfx_smoke(&launch).map_err(|err| err.to_string())?;
            Ok(format_drive_coupled_audio_vfx_summary(
                "CA39 drive-coupled audio/VFX",
                &summary,
            ))
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
        [command] if command == "behavior-tuning-metrics-smoke" => {
            let summary = run_behavior_tuning_metrics_smoke().map_err(|err| err.to_string())?;
            Ok(format_behavior_tuning_summary(
                "CA21 behavior tuning",
                &summary,
            ))
        }
        [command] if command == "ecological-soak-smoke" => {
            let summary = run_ecological_soak_smoke().map_err(|err| err.to_string())?;
            Ok(format_ecological_soak_summary(
                "CA22 ecological soak",
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
        [command] if command == "content-authoring-smoke" => {
            let summary = run_content_authoring_smoke().map_err(|err| err.to_string())?;
            Ok(format_content_tutorial_authoring_summary(
                "S09 content tutorial authoring",
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
        _ => Err("usage: alife_game_app headless-smoke <p34-fixture-root> | headless-paused-smoke <p34-fixture-root> | validate-config <config> <manifest> <asset-root> | list-environments [--manifest path] | environment-launch-smoke [--manifest path] [--scenario id] | bevy-smoke <p34-fixture-root> | graphical-playground [<fixture-root>|--scenario id] [--manifest path] [--gpu-mode cpu-reference|static-plastic-cpu-shadow-guarded|auto-with-cpu-fallback] [--smoke-seconds N] [--require-gpu] | graphical-playground-smoke --seconds <N> <p34-fixture-root> | visible-signature <p34-fixture-root> | visible-world-smoke <p34-fixture-root> | live-brain-tick-smoke <p34-fixture-root> | live-brain-paused-smoke <p34-fixture-root> | live-brain-fixed-smoke <p34-fixture-root> <ticks> | runtime-controls-smoke <p34-fixture-root> <ticks> | graphical-controls-smoke <p34-fixture-root> | topological-concept-overlay-smoke <p34-fixture-root> | memory-history-journal-smoke <p34-fixture-root> | neural-activity-profiler-smoke <p34-fixture-root> | realtime-wgsl-telemetry-smoke <p34-fixture-root> | behavior-comparison-lab-smoke [--manifest path] [--a scenario] [--b scenario] [--ticks N] [--out path] | graphical-population-smoke <p34-fixture-root> | graphical-ecology-smoke <p34-fixture-root> | world-art-style-smoke <p34-fixture-root> | graphical-lifecycle-smoke | double-buffered-scheduler-smoke <p34-fixture-root> | motor-ring-arbitration-smoke <p34-fixture-root> | homeostasis-runtime-smoke <p34-fixture-root> | affordance-loop-smoke <p34-fixture-root> | hazard-recovery-smoke <p34-fixture-root> | graphical-save-load-menu-smoke <p34-fixture-root> | creature-visual-smoke <p34-fixture-root> | creature-inspector-smoke <p34-fixture-root> | playable-survival-loop-smoke | world-ecology-loop-smoke | population-social-loop-smoke | lifecycle-lineage-smoke | school-mode-smoke | graphical-school-mode-smoke | teacher-world-cues-smoke | curriculum-authoring-smoke [manifest-path] | semantic-provider-smoke | real-semantic-provider-smoke | internal-slm-prior-smoke | llamacpp-semantic-provider-smoke | llamacpp-slm-prior-smoke | llamacpp-local-model-runtime-smoke | advanced-gameplay-ux-smoke | gpu-product-smoke | full-gpu-runtime-smoke <p34-fixture-root> [--mode static-shadow|static-action-authoritative|static-plastic-shadow|static-plastic-cpu-shadow-guarded|full-shadow|full-action-authoritative] [--ticks N] [--json path] | batched-gpu-runtime-smoke <p34-fixture-root> [--creatures N] [--ticks N] [--cpu-shadow-every 1] [--json path] | sampled-gpu-runtime-smoke <p34-fixture-root> [--creatures N] [--ticks N] [--warmup-ticks N] [--cpu-shadow-every N] [--json path] | gpu-longrun-soak <p34-fixture-root> [--ticks N] [--report-every N] [--json path] | gpu-sustained-learning-soak <p34-fixture-root> [--ticks N] [--report-every N] [--episode-ticks N] [--json path] | multi-hour-soak-isolation-smoke [--out path] | gpu-graphics-performance-smoke <p34-fixture-root> | world-editor-smoke | player-sandbox-editor-smoke [--manifest path] [--scenario id] [--output path] | app-bundle-smoke [--manifest path] | cognition-debug-smoke | save-load-ux-smoke <p34-fixture-root> | feedback-polish-smoke <p34-fixture-root> | drive-coupled-audio-vfx-smoke <p34-fixture-root> | population-performance-smoke <p34-fixture-root> | longrun-balance-smoke | behavior-tuning-metrics-smoke | ecological-soak-smoke | onboarding-help-smoke | content-authoring-smoke | platform-package-smoke | product-qa-smoke | release-candidate-smoke".to_string()),
    }
}

fn run_list_environments_cli(args: &[String]) -> Result<String, String> {
    let mut manifest_path = alife_game_app::default_environment_manifest_path();
    let mut index = 0_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--manifest requires a path".to_string())?;
                manifest_path = PathBuf::from(value);
                index += 2;
            }
            unknown => return Err(format!("unknown list-environments option: {unknown}")),
        }
    }
    let manifest =
        EnvironmentManifest::from_json_file(&manifest_path).map_err(|err| err.to_string())?;
    manifest
        .validate(&manifest_path)
        .map_err(|err| err.to_string())?;
    let scenario_lines = manifest
        .scenarios
        .iter()
        .map(|scenario| {
            format!(
                "{}:{}:visible={}:{}",
                scenario.id, scenario.title, scenario.player_visible, scenario.description
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    Ok(format!(
        "CA10 environments schema={} version={} default={} count={} scenarios={}",
        manifest.schema,
        manifest.schema_version,
        manifest.default_scenario_id,
        manifest.scenarios.len(),
        scenario_lines
    ))
}

fn run_environment_launch_smoke_cli(args: &[String]) -> Result<String, String> {
    let mut manifest_path = alife_game_app::default_environment_manifest_path();
    let mut scenario_id = None::<String>;
    let mut index = 0_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--manifest requires a path".to_string())?;
                manifest_path = PathBuf::from(value);
                index += 2;
            }
            "--scenario" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--scenario requires an environment id".to_string())?;
                scenario_id = Some(value.clone());
                index += 2;
            }
            unknown => {
                return Err(format!(
                    "unknown environment-launch-smoke option: {unknown}"
                ))
            }
        }
    }
    let summary = run_environment_launcher_smoke(&manifest_path, scenario_id.as_deref())
        .map_err(|err| err.to_string())?;
    Ok(format_environment_launcher_summary(
        "CA10 environment launcher",
        &summary,
    ))
}

fn run_behavior_comparison_lab_cli(args: &[String]) -> Result<String, String> {
    let mut manifest_path = alife_game_app::default_environment_manifest_path();
    let mut scenario_a = None::<String>;
    let mut scenario_b = None::<String>;
    let mut ticks = alife_game_app::CA31_DEFAULT_COMPARISON_TICKS;
    let mut output_path = None::<PathBuf>;
    let mut index = 0_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--manifest requires a path".to_string())?;
                manifest_path = PathBuf::from(value);
                index += 2;
            }
            "--a" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--a requires a scenario id".to_string())?;
                scenario_a = Some(value.clone());
                index += 2;
            }
            "--b" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--b requires a scenario id".to_string())?;
                scenario_b = Some(value.clone());
                index += 2;
            }
            "--ticks" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--ticks requires a value".to_string())?;
                ticks = value
                    .parse::<u32>()
                    .map_err(|_| "--ticks must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--out requires a path".to_string())?;
                output_path = Some(PathBuf::from(value));
                index += 2;
            }
            unknown => {
                return Err(format!(
                    "unknown behavior-comparison-lab-smoke option: {unknown}"
                ))
            }
        }
    }
    let summary = run_behavior_comparison_lab_smoke(
        &manifest_path,
        scenario_a.as_deref(),
        scenario_b.as_deref(),
        ticks,
    )
    .map_err(|err| err.to_string())?;
    if let Some(output_path) = output_path {
        write_behavior_comparison_lab_report(&summary, output_path)
            .map_err(|err| err.to_string())?;
    }
    Ok(format_behavior_comparison_lab_summary(
        "CA31 behavior comparison lab",
        &summary,
    ))
}

fn run_player_sandbox_editor_cli(args: &[String]) -> Result<String, String> {
    let mut manifest_path = alife_game_app::default_environment_manifest_path();
    let mut scenario_id = None::<String>;
    let mut output_path = None::<PathBuf>;
    let mut index = 0_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--manifest requires a path".to_string())?;
                manifest_path = PathBuf::from(value);
                index += 2;
            }
            "--scenario" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--scenario requires an environment id".to_string())?;
                scenario_id = Some(value.clone());
                index += 2;
            }
            "--output" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--output requires a save path".to_string())?;
                output_path = Some(PathBuf::from(value));
                index += 2;
            }
            unknown => {
                return Err(format!(
                    "unknown player-sandbox-editor-smoke option: {unknown}"
                ))
            }
        }
    }
    let summary = run_player_sandbox_editor_smoke(
        &manifest_path,
        scenario_id.as_deref(),
        output_path.as_deref(),
    )
    .map_err(|err| err.to_string())?;
    Ok(format_player_sandbox_editor_summary(
        "CA11 player sandbox editor",
        &summary,
    ))
}

fn run_app_bundle_smoke_cli(args: &[String]) -> Result<String, String> {
    let mut manifest_path = default_app_bundle_manifest_path();
    let mut index = 0_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--manifest requires a path".to_string())?;
                manifest_path = PathBuf::from(value);
                index += 2;
            }
            unknown => return Err(format!("unknown app-bundle-smoke option: {unknown}")),
        }
    }
    let summary = validate_app_bundle_manifest(&manifest_path).map_err(|err| err.to_string())?;
    Ok(format_app_bundle_summary(
        "CA12 app bundle ingestion",
        &summary,
    ))
}

fn run_full_gpu_runtime_cli(args: &[String]) -> Result<String, String> {
    let Some(fixture_root) = args.first() else {
        return Err("full-gpu-runtime-smoke requires <p34-fixture-root>".to_string());
    };
    let mut options = FullGpuRuntimeSmokeOptions::default();
    let mut index = 1_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--mode" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--mode requires a value".to_string())?;
                options.mode = parse_full_gpu_runtime_mode(value)?;
                index += 2;
            }
            "--ticks" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--ticks requires a value".to_string())?;
                options.ticks = value
                    .parse::<u32>()
                    .map_err(|_| "--ticks must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--json" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--json requires a path".to_string())?;
                options.json_path = Some(PathBuf::from(value));
                index += 2;
            }
            unknown => return Err(format!("unknown full-gpu-runtime-smoke option: {unknown}")),
        }
    }
    let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
    let summary = run_full_gpu_runtime_smoke(&launch, options).map_err(|err| err.to_string())?;
    Ok(format_full_gpu_runtime_summary(
        "Full GPU neural runtime",
        &summary,
    ))
}

fn run_batched_gpu_runtime_cli(args: &[String]) -> Result<String, String> {
    let Some(fixture_root) = args.first() else {
        return Err("batched-gpu-runtime-smoke requires <p34-fixture-root>".to_string());
    };
    let mut options = BatchedGpuRuntimeOptions::default();
    let mut index = 1_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--creatures" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--creatures requires a value".to_string())?;
                options.max_creatures = value
                    .parse::<usize>()
                    .map_err(|_| "--creatures must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--ticks" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--ticks requires a value".to_string())?;
                options.ticks = value
                    .parse::<u32>()
                    .map_err(|_| "--ticks must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--cpu-shadow-every" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--cpu-shadow-every requires a value".to_string())?;
                options.cpu_shadow_every = value
                    .parse::<u32>()
                    .map_err(|_| "--cpu-shadow-every must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--json" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--json requires a path".to_string())?;
                options.json_path = Some(PathBuf::from(value));
                index += 2;
            }
            unknown => {
                return Err(format!(
                    "unknown batched-gpu-runtime-smoke option: {unknown}"
                ))
            }
        }
    }
    let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
    let summary = run_batched_gpu_runtime_smoke(&launch, options).map_err(|err| err.to_string())?;
    Ok(format_batched_gpu_runtime_summary(
        "CA33 batched GPU runtime",
        &summary,
    ))
}

fn run_sampled_gpu_runtime_cli(args: &[String]) -> Result<String, String> {
    let Some(fixture_root) = args.first() else {
        return Err("sampled-gpu-runtime-smoke requires <p34-fixture-root>".to_string());
    };
    let mut options = SampledGpuRuntimeOptions::default();
    let mut index = 1_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--creatures" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--creatures requires a value".to_string())?;
                options.max_creatures = value
                    .parse::<usize>()
                    .map_err(|_| "--creatures must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--ticks" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--ticks requires a value".to_string())?;
                options.ticks = value
                    .parse::<u32>()
                    .map_err(|_| "--ticks must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--warmup-ticks" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--warmup-ticks requires a value".to_string())?;
                options.warmup_ticks = value
                    .parse::<u32>()
                    .map_err(|_| "--warmup-ticks must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--cpu-shadow-every" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--cpu-shadow-every requires a value".to_string())?;
                options.cpu_shadow_every = value
                    .parse::<u32>()
                    .map_err(|_| "--cpu-shadow-every must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--json" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--json requires a path".to_string())?;
                options.json_path = Some(PathBuf::from(value));
                index += 2;
            }
            unknown => {
                return Err(format!(
                    "unknown sampled-gpu-runtime-smoke option: {unknown}"
                ))
            }
        }
    }
    let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
    let summary = run_sampled_gpu_runtime_smoke(&launch, options).map_err(|err| err.to_string())?;
    Ok(format_sampled_gpu_runtime_summary(
        "CA34 sampled GPU runtime",
        &summary,
    ))
}

fn run_gpu_longrun_soak_cli(args: &[String]) -> Result<String, String> {
    let Some(fixture_root) = args.first() else {
        return Err("gpu-longrun-soak requires <p34-fixture-root>".to_string());
    };
    let mut options = GpuLongrunSoakOptions::default();
    let mut index = 1_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--ticks" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--ticks requires a value".to_string())?;
                options.ticks = value
                    .parse::<u32>()
                    .map_err(|_| "--ticks must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--report-every" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--report-every requires a value".to_string())?;
                options.report_every = value
                    .parse::<u32>()
                    .map_err(|_| "--report-every must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--json" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--json requires a path".to_string())?;
                options.json_path = Some(PathBuf::from(value));
                index += 2;
            }
            unknown => return Err(format!("unknown gpu-longrun-soak option: {unknown}")),
        }
    }
    let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
    let summary = run_gpu_longrun_soak(&launch, options).map_err(|err| err.to_string())?;
    Ok(format_gpu_longrun_soak_summary(
        "GPU long-run soak",
        &summary,
    ))
}

fn run_gpu_sustained_learning_soak_cli(args: &[String]) -> Result<String, String> {
    let Some(fixture_root) = args.first() else {
        return Err("gpu-sustained-learning-soak requires <p34-fixture-root>".to_string());
    };
    let mut options = GpuSustainedLearningSoakOptions::default();
    let mut index = 1_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--ticks" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--ticks requires a value".to_string())?;
                options.ticks = value
                    .parse::<u32>()
                    .map_err(|_| "--ticks must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--report-every" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--report-every requires a value".to_string())?;
                options.report_every = value
                    .parse::<u32>()
                    .map_err(|_| "--report-every must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--episode-ticks" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--episode-ticks requires a value".to_string())?;
                options.episode_ticks = value
                    .parse::<u32>()
                    .map_err(|_| "--episode-ticks must be an unsigned integer".to_string())?;
                index += 2;
            }
            "--json" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--json requires a path".to_string())?;
                options.json_path = Some(PathBuf::from(value));
                index += 2;
            }
            unknown => {
                return Err(format!(
                    "unknown gpu-sustained-learning-soak option: {unknown}"
                ));
            }
        }
    }
    let launch = AppShellLaunchConfig::from_p34_fixture_root(fixture_root);
    let summary =
        run_gpu_sustained_learning_soak(&launch, options).map_err(|err| err.to_string())?;
    Ok(format_gpu_sustained_learning_soak_summary(
        "GPU sustained-learning soak",
        &summary,
    ))
}

fn run_multi_hour_soak_isolation_cli(args: &[String]) -> Result<String, String> {
    let mut output_path = PathBuf::from(alife_game_app::CA36_DEFAULT_REPORT_PATH);
    let mut index = 0_usize;
    while index < args.len() {
        match args[index].as_str() {
            "--out" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--out requires a path".to_string())?;
                output_path = PathBuf::from(value);
                index += 2;
            }
            unknown => {
                return Err(format!(
                    "unknown multi-hour-soak-isolation-smoke option: {unknown}"
                ));
            }
        }
    }
    let summary = run_multi_hour_soak_isolation_smoke().map_err(|err| err.to_string())?;
    write_ca36_soak_isolation_report(&summary, &output_path).map_err(|err| err.to_string())?;
    Ok(format_soak_isolation_summary(
        "CA36 multi-hour soak isolation",
        &summary,
        &output_path,
    ))
}

fn parse_full_gpu_runtime_mode(value: &str) -> Result<FullGpuRuntimeSmokeMode, String> {
    match value {
        "cpu" | "cpu-reference" => Ok(FullGpuRuntimeSmokeMode::CpuReference),
        "static-shadow" => Ok(FullGpuRuntimeSmokeMode::StaticShadow),
        "static-action-authoritative" => Ok(FullGpuRuntimeSmokeMode::StaticActionAuthoritative),
        "static-plastic-shadow" => Ok(FullGpuRuntimeSmokeMode::StaticPlasticShadow),
        "static-plastic-cpu-shadow-guarded" => {
            Ok(FullGpuRuntimeSmokeMode::StaticPlasticCpuShadowGuarded)
        }
        "full-shadow" => Ok(FullGpuRuntimeSmokeMode::FullShadow),
        "full-action-authoritative" => Ok(FullGpuRuntimeSmokeMode::FullActionAuthoritative),
        _ => Err(format!("unknown full GPU runtime mode: {value}")),
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

fn format_environment_launcher_summary(
    prefix: &str,
    summary: &alife_game_app::EnvironmentLauncherSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} default={} selected={} title='{}' scenarios={} seed={} assets={} objects={} creatures={} food={} hazards={} obstacles={} fixture={} error_hint='{}' signature={}",
        summary.schema,
        summary.schema_version,
        summary.default_scenario_id,
        summary.selected_scenario_id,
        summary.title,
        summary.scenario_count,
        summary.seed,
        summary.asset_count,
        summary.object_count,
        summary.creature_count,
        summary.food_count,
        summary.hazard_count,
        summary.obstacle_count,
        summary.fixture_root.display(),
        summary.player_visible_error_sample,
        summary.signature_line()
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

fn format_graphical_controls_summary(
    prefix: &str,
    summary: &alife_game_app::GraphicalControlSmokeSummary,
) -> String {
    format!(
        "{prefix} toggle={} speed={:?} follow={:?} reset={} terminal_guidance={} exit={} playback={} step={} run={} sealed={} patches={} stable_id_only={} signature={}",
        summary.toggle_pause_run_verified,
        summary.speed_sequence,
        summary.follow_target.map(|id| id.raw()),
        summary.reset_verified,
        summary.terminal_guidance_visible,
        summary.exit_requested,
        summary.runtime.panel.playback.label(),
        summary.runtime.step_produced,
        summary.runtime.run_produced,
        summary.runtime.all_patches_sealed,
        summary.runtime.panel.sealed_patch_count,
        !summary.overlay_text.contains("Entity("),
        summary.runtime.panel.signature_line()
    )
}

fn format_double_buffered_scheduler_summary(
    prefix: &str,
    summary: &alife_game_app::DoubleBufferedSchedulerSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} fixed_hz={} render_hz={} paused={} sub_tick={} fixed_tick={} step={} catch_up={} alpha={:.3} buffers={}/{} drift_us={} frame_drift_prevented={} signature={}",
        summary.scheduler.schema,
        summary.scheduler.schema_version,
        summary.scheduler.config.fixed_tick_hz,
        summary.scheduler.config.target_render_hz,
        summary.paused_ticks,
        summary.sub_tick_due,
        summary.fixed_tick_due,
        summary.step_ticks,
        summary.catch_up_ticks,
        summary.scheduler.render_alpha(),
        summary.scheduler.front_buffer.label(),
        summary.scheduler.back_buffer.label(),
        summary.scheduler.accumulator_micros,
        summary.frame_driven_drift_prevented,
        summary.scheduler.signature_line()
    )
}

fn format_motor_ring_arbitration_summary(
    prefix: &str,
    summary: &alife_game_app::MotorRingArbitrationSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} winner={} margin={:.3} selected={:?}:{:?} patch_sealed={} no_direct_bypass={} panel={}",
        summary.ring.schema,
        summary.ring.schema_version,
        summary.ring.selected_label,
        summary.ring.winner_margin,
        summary.selected_action_kind,
        summary.selected_action_id.map(|id| id.raw()),
        summary.patch_sealed,
        summary.ring.no_direct_action_bypass,
        summary.ring.panel_text().replace('\n', " | ")
    )
}

fn format_homeostasis_runtime_summary(
    prefix: &str,
    summary: &alife_game_app::HomeostasisRuntimeSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} organism={} tick={}->{} fixed={} finite={} sealed={} salience={:.3} learning={:.3} bars={} signature={}",
        summary.after.schema,
        summary.after.schema_version,
        summary.after.organism_id.raw(),
        summary.before.tick.raw(),
        summary.after.tick.raw(),
        summary.fixed_register_count,
        summary.finite_and_bounded,
        summary.patch_sealed,
        summary.after.salience_modulation,
        summary.after.learning_modulation,
        summary
            .after
            .registers
            .iter()
            .map(alife_game_app::HomeostasisRegisterPresentation::bar_line)
            .collect::<Vec<_>>()
            .join("|"),
        summary.signature_line()
    )
}

fn format_affordance_loop_summary(
    prefix: &str,
    summary: &alife_game_app::AffordanceLoopSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} organism={} food=stable:{} distance={:.3}->{:.3} approach={:?}:{:?} eat={:?}:{:?} sealed={} consumed={} hunger={:.3}->{:.3} energy={:.3}->{:.3} normal_arbitration={} no_scripted={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.organism_id.raw(),
        summary.food_entity.raw(),
        summary.initial_food_distance,
        summary.after_approach_food_distance,
        summary.approach_tick.selected_action_kind,
        summary.approach_tick.selected_action_id.map(|id| id.raw()),
        summary.eat_tick.selected_action_kind,
        summary.eat_tick.selected_action_id.map(|id| id.raw()),
        summary.sealed_patches,
        summary.food_consumed,
        summary.hunger_before,
        summary.hunger_after,
        summary.energy_before,
        summary.energy_after,
        summary.normal_arbitration_preserved,
        summary.no_scripted_action_forcing,
        summary.signature
    )
}

fn format_hazard_recovery_summary(
    prefix: &str,
    summary: &alife_game_app::HazardRecoverySmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} organism={} hazard=stable:{} visible={} cue={} salience={:.3} distance={:.3}->{:.3} flee={:?}:{:?} pain={:.3}->{:.3} fear={:.3}->{:.3} sleep={:?} fatigue={:.3}->{:.3} failure={:?} recovered={} terminal_avoided={} sealed={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.organism_id.raw(),
        summary.hazard_entity.raw(),
        summary.fixture_hazard_visible,
        summary.visible_hazard_cue,
        summary.hazard_salience,
        summary.initial_hazard_distance,
        summary.after_flee_hazard_distance,
        summary.flee_tick.selected_action_kind,
        summary.flee_tick.selected_action_id.map(|id| id.raw()),
        summary.pain_before,
        summary.pain_after_contact,
        summary.fear_before,
        summary.fear_after_contact,
        summary.sleep_phase_after,
        summary.fatigue_before_sleep,
        summary.fatigue_after_sleep,
        summary.failure_tick.action_failure,
        summary.failure_recovered_with_sealed_patch,
        summary.terminal_stagnation_avoided,
        summary.sealed_patches,
        summary.signature
    )
}

fn format_graphical_save_load_menu_summary(
    prefix: &str,
    summary: &alife_game_app::GraphicalSaveLoadMenuSmokeSummary,
) -> String {
    format!(
        "{prefix} opened={} save={} load={} invalid={} load_count={} invalid_errors={} stable_ids={} no_partial_load={} engine_tokens_absent={} overlay={}",
        summary.menu_opened,
        summary.manual_save.success,
        summary.manual_load.success,
        !summary.invalid_load.success,
        summary.load_applied_count,
        summary.invalid_error_count,
        summary
            .stable_world_ids
            .iter()
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join("+"),
        summary.no_partial_load_after_error,
        summary.engine_local_token_absent,
        summary.overlay_text.replace('\n', " | ")
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
        "{prefix} schema={} version={} title='{}' mode={} timeout={:?} seed={} backend={:?} gpu_mode={} require_gpu={} gpu_selected={} gpu_claim={} gpu_scores={} cpu_shadow_parity={} h_shadow_apps={} fallback={:?} objects={} creatures={} food={} hazards={} cpu_fallback={} stable_ids={} persistent={} playback={} mind_tick={} world_tick={:?} action={:?}:{:?} sealed_patches={} packed_logs={} signature={}",
        summary.launch.schema,
        summary.launch.schema_version,
        summary.launch.window_title,
        summary.launch.mode_label,
        summary.launch.smoke_seconds,
        summary.launch.seed,
        summary.launch.selected_backend,
        summary.launch.requested_gpu_mode.label(),
        summary.launch.require_gpu,
        summary.gpu.selected_backend,
        summary.gpu.product_runtime_claim,
        summary.gpu.gpu_scores_used_for_proposals,
        summary.gpu.cpu_shadow_parity,
        summary.gpu.h_shadow_applications,
        summary.gpu.fallback_reason,
        summary.launch.object_count,
        summary.launch.creature_marker_count,
        summary.launch.food_marker_count,
        summary.launch.hazard_marker_count,
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

fn format_graphical_lifecycle_summary(
    prefix: &str,
    summary: &alife_game_app::Ca20GraphicalLifecycleSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} living={} cap={} births={} deaths={} blocked={} selected={:?} lineages={} genetic_lifetime_separated={} birth_assets_initialize_only={} save_load_lineages={} events={} lineage_rows={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.living_population,
        summary.population_cap,
        summary.births,
        summary.deaths,
        summary.reproduction_blocked_count,
        summary.selected_stable_id.map(|id| id.raw()),
        summary.lineage_count,
        summary.genetic_lifetime_separated,
        summary.birth_weight_assets_are_initializers,
        summary.save_load_lineages_roundtrip,
        summary
            .event_rows
            .iter()
            .map(alife_game_app::Ca20LifecyclePanelRow::signature_line)
            .collect::<Vec<_>>()
            .join("|"),
        summary
            .lineage_rows
            .iter()
            .map(alife_game_app::Ca20LifecyclePanelRow::signature_line)
            .collect::<Vec<_>>()
            .join("|"),
        summary.signature,
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

fn format_curriculum_authoring_summary(
    prefix: &str,
    summary: &alife_game_app::CurriculumAuthoringSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} manifest={} curriculum={} lessons={} active={} verifier_conditions={} sealed_verifier={} verifier_passed={} completed={} model_required={} fake_model={} can_issue_actions={} can_rewrite_weights={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.manifest_path,
        summary.curriculum_id,
        summary.lesson_count,
        summary.active_lesson_id,
        summary.verifier_condition_labels.len(),
        summary.verifier_uses_sealed_patches,
        summary.verifier_passed,
        summary.completed_lesson_ids.len(),
        summary.model_inference_required,
        summary.fake_model_output_used,
        summary.can_issue_actions,
        summary.can_rewrite_weights,
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

fn format_real_semantic_provider_summary(
    prefix: &str,
    summary: &alife_game_app::RealSemanticProviderSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} model_manifest={} model_version={} repo={} role={} license={} backend={} endpoint={}:{} alias={} downloaded={} inference_smoke={} raw_dims={} projected_dims={} semantic_codes={} bounded={} fake_model={} can_issue_actions={} can_rewrite_weights={} hidden_vector={} user_action_required={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.model_manifest_schema,
        summary.model_manifest_schema_version,
        summary.repo_id,
        summary.model_role,
        summary.license,
        summary.runtime_backend,
        summary.llamacpp_host,
        summary.llamacpp_port,
        summary.llamacpp_alias,
        summary.downloaded_locally,
        summary.inference_smoke_passed,
        summary.raw_embedding_dims,
        summary.projected_embedding_dims,
        summary.semantic_code_count,
        summary.context_vectors_bounded,
        summary.fake_model_output_used,
        summary.can_issue_actions,
        summary.can_rewrite_weights,
        summary.hidden_vector_injection,
        summary.unavailable_is_user_action_required,
        summary.signature_line()
    )
}

fn format_internal_slm_prior_summary(
    prefix: &str,
    summary: &alife_game_app::InternalSlmPriorSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} output_schema={} output_version={} target_repo={} repo={} role={} license={} backend={} endpoint={}:{} alias={} downloaded={} inference_smoke={} queue={}/{} timeout_ms={} labels={} summary_chars={} lexicon={} tags={} can_issue_actions={} can_rewrite_weights={} bypass={} hidden_vector={} malformed_rejected={} user_action_required={} disabled_nonfatal={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.output_schema,
        summary.output_schema_version,
        summary.target_repo_id,
        summary.repo_id,
        summary.model_role,
        summary.license,
        summary.runtime_backend,
        summary.llamacpp_host,
        summary.llamacpp_port,
        summary.llamacpp_alias,
        summary.downloaded_locally,
        summary.inference_smoke_passed,
        summary.processed_requests,
        summary.queue_capacity,
        summary.timeout_ms,
        summary.salience_label_count,
        summary.context_summary_chars,
        summary.lexicon_association_count,
        summary.perception_tag_count,
        summary.can_issue_actions,
        summary.can_rewrite_weights,
        summary.can_bypass_arbitration,
        summary.hidden_vector_injection,
        summary.malformed_output_rejected,
        summary.unavailable_is_user_action_required,
        summary.feature_disabled_is_nonfatal,
        summary.signature_line()
    )
}

fn format_advanced_gameplay_summary(
    prefix: &str,
    summary: &alife_game_app::AdvancedGameplayUxSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} social='{}' lifecycle='{}' school='{}' semantic='{}' display_only={} optional={} bypass_blocked={} screenshot_status={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.social.signature_line(),
        summary.lifecycle.signature_line(),
        summary.school.signature_line(),
        summary.semantic.signature_line(),
        summary.display_only,
        summary.optional_modes,
        summary.no_action_or_weight_bypass,
        summary.manual_screenshot_status,
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

fn format_full_gpu_runtime_summary(
    prefix: &str,
    summary: &alife_game_app::FullGpuRuntimeSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} mode={} combined={} selected={} fallback={:?} hardware={} ticks={} actions={} sealed_patches={} packed_logs={} gpu_static={} gpu_used_for_proposals={} cpu_shadow_parity={} routing=tiles:{}/{} skipped:{} synapses:{} compact_readback_bytes={} bulk_readback_forbidden={} plasticity={} live_h_shadow_applied={} post_seal_hshadow_applied={} replay_protected={} post_seal_diag_readback_bytes={} post_seal_diag_readback_ms={:.4} post_seal_diag_boundary={} h_shadow_changed={} h_updates={} delta_records={}/{} delta_max={:.6} seq={:?} w_genetic_fixed_unchanged={} lifetime_unchanged={} h_operational_unchanged={} timing=upload:{:.4},gpu:{:.4},readback:{:.4},cpu_shadow:{:.4},total:{:.4} claim={} unsupported_full_gap_remaining={} gap='{}'",
        summary.schema,
        summary.schema_version,
        summary.requested_mode,
        summary.combined_mode,
        summary.selected_backend,
        summary.fallback_reason,
        summary.hardware_identifier.as_deref().unwrap_or("none"),
        summary.ticks_run,
        summary.actions_selected.join("|"),
        summary.sealed_patches,
        summary.packed_logs,
        summary.gpu_static_dispatched,
        summary.gpu_output_used_for_proposals,
        summary.cpu_shadow_parity,
        summary.routing_active_tiles,
        summary.routing_total_tiles,
        summary.routing_skipped_tiles,
        summary.routing_active_synapses,
        summary.compact_readback_bytes,
        summary.bulk_readback_forbidden,
        summary.plasticity_dispatched,
        summary.plasticity_live_core_update_applied,
        summary.post_seal_hshadow_applied,
        summary.post_seal_replay_protected,
        summary.post_seal_diagnostic_readback_bytes,
        summary.post_seal_diagnostic_readback_ms,
        summary.post_seal_diagnostic_readback_boundary_scoped,
        summary.h_shadow_changed,
        summary.h_shadow_updated_values,
        summary.post_seal_delta_applied_records,
        summary.post_seal_delta_changed_records,
        summary.post_seal_delta_max_abs_delta,
        summary.post_seal_delta_sequence_id,
        summary.w_genetic_fixed_unchanged,
        summary.lifetime_consolidated_unchanged,
        summary.h_operational_unchanged,
        summary.upload_ms,
        summary.gpu_submit_poll_ms,
        summary.compact_readback_ms,
        summary.cpu_shadow_ms,
        summary.total_gpu_runtime_ms,
        summary.product_runtime_claim,
        summary.unsupported_full_runtime_gap_remaining,
        summary.plasticity_live_gap,
    )
}

fn format_batched_gpu_runtime_summary(
    prefix: &str,
    summary: &alife_game_app::BatchedGpuRuntimeSummary,
) -> String {
    let creatures = summary
        .per_creature
        .iter()
        .map(|creature| {
            format!(
                "stable:{}:org:{}:backend={}:gpu_scores={}:parity={}:sealed={}:h={}:bytes={}",
                creature.stable_id.raw(),
                creature.organism_id.raw(),
                creature.selected_backend,
                creature.gpu_scores_used_for_proposals,
                creature.cpu_shadow_parity,
                creature.sealed_patches,
                creature.post_seal_hshadow_applied,
                creature.compact_readback_bytes
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "{prefix} schema={} version={} mode={} batch={}/{} ticks={} selected={} fallback={:?} hardware={} shared_session={} compact_record_bytes={} compact_readback_bytes={} post_seal_readback_bytes={} gpu_static_creatures={} gpu_proposal_creatures={} cpu_shadow_checks={} cpu_shadow_every={} cpu_shadow_every_tick={} sampled_deferred_to_ca34={} parity_failures={} fallback_creatures={} plasticity_creatures={} h_apps={} h_records={} h_delta_max={:.6} w_genetic_fixed_unchanged={} lifetime_unchanged={} h_operational_unchanged={} timing=upload:{:.4},gpu:{:.4},compact_readback:{:.4},post_seal_readback:{:.4},cpu_shadow:{:.4} claim={} full_action_authoritative_claim={} no_active_bulk_readback={} stable_id_only={} creatures={}",
        summary.schema,
        summary.schema_version,
        summary.requested_mode,
        summary.batch_size,
        summary.max_batch_size,
        summary.ticks_run,
        summary.selected_backend,
        summary.fallback_reason,
        summary.hardware_identifier.as_deref().unwrap_or("none"),
        summary.shared_gpu_session,
        summary.per_creature_compact_record_bytes,
        summary.compact_readback_bytes,
        summary.post_seal_readback_bytes,
        summary.gpu_static_dispatched_creatures,
        summary.gpu_proposal_creatures,
        summary.cpu_shadow_parity_checks,
        summary.cpu_shadow_every,
        summary.cpu_shadow_checked_every_tick,
        summary.sampled_cpu_shadow_deferred_to_ca34,
        summary.parity_failures,
        summary.fallback_creatures,
        summary.plasticity_dispatched_creatures,
        summary.post_seal_hshadow_applications,
        summary.h_shadow_delta_records,
        summary.max_h_shadow_abs_delta,
        summary.w_genetic_fixed_unchanged,
        summary.lifetime_consolidated_unchanged,
        summary.h_operational_unchanged,
        summary.total_upload_ms,
        summary.total_submit_poll_ms,
        summary.total_compact_readback_ms,
        summary.total_post_seal_readback_ms,
        summary.total_cpu_shadow_ms,
        summary.product_runtime_claim,
        summary.full_action_authoritative_claim,
        summary.no_active_bulk_readback,
        summary.stable_id_only,
        creatures,
    )
}

fn format_sampled_gpu_runtime_summary(
    prefix: &str,
    summary: &alife_game_app::SampledGpuRuntimeSummary,
) -> String {
    let creatures = summary
        .per_creature
        .iter()
        .map(|creature| {
            format!(
                "stable:{}:org:{}:backend={}:gpu_scores={}:checks={}:skipped={}:failures={}:sealed={}:h_apps={}",
                creature.stable_id.raw(),
                creature.organism_id.raw(),
                creature.selected_backend,
                creature.gpu_scores_used_for_proposals,
                creature.cpu_shadow_checks,
                creature.cpu_shadow_skipped,
                creature.parity_failures,
                creature.sealed_patches,
                creature.post_seal_hshadow_applications
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "{prefix} schema={} version={} mode={} batch={}/{} ticks={} warmup={} cpu_shadow_every={} sampled={} fallback_on_first_failure={} selected={} fallback={:?} hardware={} shared_session={} gpu_static_creatures={} gpu_proposal_creatures={} cpu_shadow_checks={} cpu_shadow_skipped={} parity_failures={} first_parity_failure={:?} forced_cpu_after_failure={} fallback_creatures={} compact_readback_bytes={} post_seal_readback_bytes={} h_apps={} h_records={} h_delta_max={:.6} w_genetic_fixed_unchanged={} lifetime_unchanged={} h_operational_unchanged={} timing=upload:{:.4},gpu:{:.4},compact_readback:{:.4},post_seal_readback:{:.4},cpu_shadow:{:.4} claim={} full_action_authoritative_claim={} no_active_bulk_readback={} stable_id_only={} creatures={}",
        summary.schema,
        summary.schema_version,
        summary.requested_mode,
        summary.batch_size,
        summary.max_batch_size,
        summary.ticks_run,
        summary.warmup_ticks,
        summary.cpu_shadow_every,
        summary.sampled_cpu_shadow_enabled,
        summary.fallback_on_first_failure,
        summary.selected_backend,
        summary.fallback_reason,
        summary.hardware_identifier.as_deref().unwrap_or("none"),
        summary.shared_gpu_session,
        summary.gpu_static_dispatched_creatures,
        summary.gpu_proposal_creatures,
        summary.cpu_shadow_checks,
        summary.cpu_shadow_skipped_creatures,
        summary.parity_failures,
        summary.first_parity_failure_tick,
        summary.forced_cpu_after_failure,
        summary.fallback_creatures,
        summary.compact_readback_bytes,
        summary.post_seal_readback_bytes,
        summary.post_seal_hshadow_applications,
        summary.h_shadow_delta_records,
        summary.max_h_shadow_abs_delta,
        summary.w_genetic_fixed_unchanged,
        summary.lifetime_consolidated_unchanged,
        summary.h_operational_unchanged,
        summary.total_upload_ms,
        summary.total_submit_poll_ms,
        summary.total_compact_readback_ms,
        summary.total_post_seal_readback_ms,
        summary.total_cpu_shadow_ms,
        summary.product_runtime_claim,
        summary.full_action_authoritative_claim,
        summary.no_active_bulk_readback,
        summary.stable_id_only,
        creatures,
    )
}

fn format_gpu_longrun_soak_summary(
    prefix: &str,
    summary: &alife_game_app::GpuLongrunSoakSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} requested={} completed={} report_every={} selected={} fallback={:?} hardware={} sealed_patches={} packed_logs={} gpu_static_ticks={} gpu_proposal_ticks={} parity_checks={} parity_failures={} first_parity_failure={:?} h_apps={} h_rejections={} first_h_rejection={:?} h_records={} h_delta_max={:.6} w_genetic_fixed_unchanged={} lifetime_unchanged={} h_operational_unchanged={} compact_readback_bytes={} post_seal_readback_bytes={} no_active_bulk_readback={} timing=upload:{:.4},gpu:{:.4},compact_readback:{:.4},post_seal_readback:{:.4},cpu_shadow:{:.4},wall:{:.4},avg_ms_tick:{:.4},ticks_sec:{:.2} claim={} full_action_authoritative_claim={}",
        summary.schema,
        summary.schema_version,
        summary.requested_ticks,
        summary.ticks_completed,
        summary.report_every,
        summary.selected_backend,
        summary.fallback_reason,
        summary.hardware_identifier.as_deref().unwrap_or("none"),
        summary.sealed_patches,
        summary.packed_logs,
        summary.gpu_static_dispatched_ticks,
        summary.gpu_proposal_ticks,
        summary.cpu_shadow_parity_checks,
        summary.parity_failures,
        summary.first_parity_failure_tick,
        summary.h_shadow_applications,
        summary.h_shadow_rejected_applications,
        summary.first_h_shadow_rejection_tick,
        summary.total_h_shadow_records_applied,
        summary.max_h_shadow_abs_delta,
        summary.w_genetic_fixed_unchanged,
        summary.lifetime_consolidated_unchanged,
        summary.h_operational_unchanged,
        summary.compact_active_readback_bytes,
        summary.post_seal_readback_bytes,
        summary.no_active_bulk_readback,
        summary.total_upload_ms,
        summary.total_submit_poll_ms,
        summary.total_compact_readback_ms,
        summary.total_post_seal_readback_ms,
        summary.total_cpu_shadow_ms,
        summary.total_wall_ms,
        summary.average_ms_per_tick,
        summary.ticks_per_second,
        summary.product_runtime_claim,
        summary.full_action_authoritative_claim,
    )
}

fn format_gpu_sustained_learning_soak_summary(
    prefix: &str,
    summary: &alife_game_app::GpuSustainedLearningSoakSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} requested={} completed={} episodes={} episode_ticks={} report_every={} selected={} fallback={:?} hardware={} sealed_patches_total={} packed_logs_total={} gpu_static_ticks={} gpu_proposal_ticks={} parity_checks={} parity_failures={} first_parity_failure={:?} h_attempts={} h_success={} h_rejections={} first_h_rejection={:?} h_records={} h_delta_max={:.6} replay_protection={} episode_rotation={} w_genetic_fixed_unchanged={} lifetime_unchanged={} h_operational_unchanged={} compact_readback_bytes={} post_seal_readback_bytes={} no_active_bulk_readback={} timing=upload:{:.4},gpu:{:.4},compact_readback:{:.4},post_seal_readback:{:.4},cpu_shadow:{:.4},wall:{:.4},avg_ms_tick:{:.4},ticks_sec:{:.2} claim={} full_action_authoritative_claim={}",
        summary.schema,
        summary.schema_version,
        summary.requested_ticks,
        summary.ticks_completed,
        summary.episodes,
        summary.episode_ticks,
        summary.report_every,
        summary.selected_backend,
        summary.fallback_reason,
        summary.hardware_identifier.as_deref().unwrap_or("none"),
        summary.sealed_patches_total,
        summary.packed_logs_total,
        summary.gpu_static_dispatched_ticks,
        summary.gpu_proposal_ticks,
        summary.cpu_shadow_parity_checks,
        summary.parity_failures,
        summary.first_parity_failure_tick,
        summary.h_shadow_application_attempts,
        summary.h_shadow_applications_succeeded,
        summary.h_shadow_applications_rejected,
        summary.first_h_shadow_rejection_tick,
        summary.total_h_shadow_records_applied,
        summary.max_h_shadow_abs_delta,
        summary.replay_protection_active,
        summary.repeated_learning_uses_episode_rotation,
        summary.w_genetic_fixed_unchanged,
        summary.lifetime_consolidated_unchanged,
        summary.h_operational_unchanged,
        summary.compact_active_readback_bytes,
        summary.post_seal_readback_bytes,
        summary.no_active_bulk_readback,
        summary.total_upload_ms,
        summary.total_submit_poll_ms,
        summary.total_compact_readback_ms,
        summary.total_post_seal_readback_ms,
        summary.total_cpu_shadow_ms,
        summary.total_wall_ms,
        summary.average_ms_per_tick,
        summary.ticks_per_second,
        summary.product_runtime_claim,
        summary.full_action_authoritative_claim,
    )
}

fn format_soak_isolation_summary(
    prefix: &str,
    summary: &alife_game_app::SoakIsolationSummary,
    output_path: &std::path::Path,
) -> String {
    format!(
        "{prefix} schema={} version={} artifact_root={} report={} manual_10k_commands={} multi_hour_commands={} graphical_commands={} counters={} reports_untracked={} cpu_fallback={} cpu_shadow_parity={} no_bulk_readback={} full_action_authoritative_claim={} release_tag_created={}",
        summary.schema,
        summary.schema_version,
        summary.artifact_root,
        output_path.display(),
        summary.manual_10k_commands.len(),
        summary.multi_hour_commands.len(),
        summary.graphical_commands.len(),
        summary.precision_drift_counters.len(),
        summary.report_artifacts_untracked,
        summary.cpu_fallback_preserved,
        summary.cpu_shadow_parity_preserved,
        summary.no_active_bulk_readback,
        summary.full_action_authoritative_claim,
        summary.release_tag_created,
    )
}

fn format_gpu_graphics_performance_summary(
    prefix: &str,
    summary: &alife_game_app::GpuGraphicsPerformanceEvidenceSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} selected={} gpu_evidence={} graphics_evidence={} fps_target={} fallback={:?} cpu_fallback={} no_readback={} launch_smoke='{}' signature={}",
        summary.schema,
        summary.schema_version,
        summary.settings_panel.selected_backend,
        summary.settings_panel.gpu_evidence_status.label(),
        summary.settings_panel.graphics_evidence_status.label(),
        summary.settings_panel.fps_target_status.label(),
        summary.settings_panel.fallback_reason,
        summary.cpu_fallback_works,
        summary.no_active_readback,
        summary.launch_window_smoke_status,
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

fn format_player_sandbox_editor_summary(
    prefix: &str,
    summary: &alife_game_app::PlayerSandboxEditorSmokeSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} scenario={} initial_objects={} final_objects={} place_remove_food={}/{} place_remove_hazard={}/{} place_remove_obstacle={}/{} pause_required={} save_bytes={} output_written={} stable_ids={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.scenario_id,
        summary.initial_object_count,
        summary.final_object_count,
        summary.placed_food,
        summary.removed_food,
        summary.placed_hazard,
        summary.removed_hazard,
        summary.placed_obstacle,
        summary.removed_obstacle,
        summary.edit_mode_required,
        summary.saved_json_bytes,
        summary.output_written,
        summary
            .stable_ids
            .iter()
            .map(|id| id.raw().to_string())
            .collect::<Vec<_>>()
            .join("+"),
        summary.signature_line()
    )
}

fn format_app_bundle_summary(
    prefix: &str,
    summary: &alife_game_app::AppBundleIngestionSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} bundle={} scenarios={} entries={} shaders={}/{} placeholder_art={} required={} largest_bytes={} missing_required_rejected={} shader_discovery={} tiny_art={} large_binary_assets={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.bundle_id,
        summary.environment_scenarios,
        summary.config_entries,
        summary.shader_assets,
        summary.discovered_shader_assets,
        summary.placeholder_art_entries,
        summary.required_entries,
        summary.largest_file_bytes,
        summary.missing_required_rejected,
        summary.shader_discovery_complete,
        summary.tiny_placeholder_art,
        summary.large_binary_assets_committed,
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

fn format_drive_coupled_audio_vfx_summary(
    prefix: &str,
    summary: &alife_game_app::Ca39DriveAudioVfxSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} cues={} active={} audio={} vfx={} sealed_sources={} backend={} fallback={:?} h_shadow_apps={} gate={} no_readback={} no_actions={} no_weights={} claim={} full_action_authoritative={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.cues.len(),
        summary.active_cue_count,
        summary.audio_cue_count,
        summary.vfx_cue_count,
        summary.sealed_feedback_sources,
        summary.selected_backend,
        summary.fallback_reason,
        summary.h_shadow_applications,
        summary.cpu_shadow_gate_preserved,
        summary.no_active_bulk_readback,
        summary.no_action_authority,
        summary.no_weight_authority,
        summary.product_runtime_claim,
        summary.full_action_authoritative_claim,
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

fn format_behavior_tuning_summary(
    prefix: &str,
    summary: &alife_game_app::BehaviorTuningSummary,
) -> String {
    let finding_statuses = summary
        .findings
        .iter()
        .map(|finding| format!("{}={}", finding.id, finding.status.label()))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{prefix} schema={} version={} sweeps={} findings={} no_hidden_overfitting={} survival={:.3} food={:.3} hazard={:.3} population={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.scenario_sweeps.len(),
        finding_statuses,
        summary.no_hidden_overfitting,
        summary.metrics.survival_score,
        summary.metrics.food_success_rate,
        summary.metrics.hazard_avoidance_score,
        summary.metrics.max_population_observed,
        summary.signature_line()
    )
}

fn format_behavior_comparison_lab_summary(
    prefix: &str,
    summary: &alife_game_app::BehaviorComparisonLabSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} a={} b={} ticks={} report_bytes={} signatures_differ={} export_small={} no_hidden_training={} direct_mutation={} semantic_actions={} gpu_action_claim={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.scenario_a.scenario_id,
        summary.scenario_b.scenario_id,
        summary.ticks,
        summary.report_bytes,
        summary.panel.signatures_differ,
        summary.export_small_report_supported,
        summary.scenario_a.no_hidden_training_mutation
            && summary.scenario_b.no_hidden_training_mutation,
        summary.direct_cognition_mutation_allowed,
        summary.semantic_action_authority,
        summary.gpu_action_authority_claim,
        summary.signature_line()
    )
}

fn format_ecological_soak_summary(
    prefix: &str,
    summary: &alife_game_app::EcologicalSoakSummary,
) -> String {
    let finding_statuses = summary
        .findings
        .iter()
        .map(|finding| format!("{}={}", finding.id, finding.status.label()))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{prefix} schema={} version={} mode={} ticks={}/{} graphical_ticks={} findings={} survival={:.3} food={:.3} hazard={:.3} population={} resources={} sealed={} config_first={} full_emergent_claim={} gpu_claim={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.config.mode.label(),
        summary.metrics.headless_ticks_completed,
        summary.metrics.headless_ticks_requested,
        summary.metrics.graphical_ticks_bounded,
        finding_statuses,
        summary.metrics.survival_score,
        summary.metrics.food_success_rate,
        summary.metrics.hazard_avoidance_score,
        summary.metrics.max_population_observed,
        summary.metrics.max_resources_observed,
        summary.metrics.sealed_patch_count,
        summary.config_first_tuning,
        summary.full_emergent_ecology_claim,
        summary.gpu_product_claim,
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

fn format_content_tutorial_authoring_summary(
    prefix: &str,
    summary: &alife_game_app::ContentTutorialAuthoringSummary,
) -> String {
    format!(
        "{prefix} schema={} version={} pack={} worlds={} lessons={} creatures={} scenarios={} checked_files={} largest_bytes={} tutorial_steps={} perception_steps={} food={} hazard={} social={} school_token={} resource_zone={} missing_rejected={} headless_ready={} graphics={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.content.pack_id,
        summary.content.world_presets,
        summary.content.lesson_packs,
        summary.content.creature_presets,
        summary.content.scenario_packs,
        summary.content.checked_files,
        summary.content.largest_file_bytes,
        summary.onboarding_tutorial_steps,
        summary.content.perception_only_lesson_steps,
        summary.content.has_food,
        summary.content.has_hazard,
        summary.content.has_social_peer,
        summary.content.has_school_token,
        summary.content.has_resource_zone,
        summary.content.missing_required_rejected,
        summary.new_tester_headless_ready,
        summary.tutorial.graphical_manual_status,
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
fn run_graphical_playground_cli(args: &[String]) -> Result<String, String> {
    use alife_game_app::{GraphicalGpuRuntimeMode, GraphicalPlaygroundMode};

    configure_windows_graphical_playground_environment();

    let mut fixture_root = None::<PathBuf>;
    let mut manifest_path = alife_game_app::default_environment_manifest_path();
    let mut scenario_id = None::<String>;
    let mut gpu_mode = GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded;
    let mut smoke_seconds = None;
    let mut require_gpu = false;
    let mut index = 0_usize;
    while index < args.len() {
        match args[index].as_str() {
            value if !value.starts_with("--") && fixture_root.is_none() => {
                fixture_root = Some(PathBuf::from(value));
                index += 1;
            }
            "--manifest" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--manifest requires a path".to_string())?;
                manifest_path = PathBuf::from(value);
                index += 2;
            }
            "--scenario" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--scenario requires an environment id".to_string())?;
                scenario_id = Some(value.clone());
                index += 2;
            }
            "--gpu-mode" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--gpu-mode requires a value".to_string())?;
                gpu_mode = GraphicalGpuRuntimeMode::parse(value).map_err(|err| err.to_string())?;
                index += 2;
            }
            "--smoke-seconds" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--smoke-seconds requires a value".to_string())?;
                smoke_seconds = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| "--smoke-seconds must be an unsigned integer".to_string())?,
                );
                index += 2;
            }
            "--require-gpu" => {
                require_gpu = true;
                index += 1;
            }
            unknown => return Err(format!("unknown graphical-playground option: {unknown}")),
        }
    }

    let app_launch = if let Some(root) = fixture_root {
        AppShellLaunchConfig::from_p34_fixture_root(root)
    } else {
        alife_game_app::select_environment_scenario(&manifest_path, scenario_id.as_deref())
            .map_err(|err| err.to_string())?
            .launch
    };
    let mode = smoke_seconds
        .map(|seconds| GraphicalPlaygroundMode::Smoke { seconds })
        .unwrap_or(GraphicalPlaygroundMode::Interactive);
    let window_title = if let Some(seconds) = smoke_seconds {
        format!(
            "{} - smoke {}s",
            alife_game_app::S01_GRAPHICAL_WINDOW_TITLE,
            seconds
        )
    } else {
        alife_game_app::S01_GRAPHICAL_WINDOW_TITLE.to_string()
    };
    let launch = alife_game_app::GraphicalPlaygroundLaunchConfig {
        app_launch,
        mode,
        gpu_mode,
        window_title,
        require_gpu,
    };
    let summary =
        alife_game_app::bevy_shell::run_graphical_playground_window_with_controls(&launch)
            .map_err(|err| err.to_string())?;
    Ok(format_graphical_playground_summary(
        if smoke_seconds.is_some() {
            "S02 graphical playground smoke"
        } else {
            "S02 graphical playground closed"
        },
        &summary,
    ))
}

#[cfg(feature = "bevy-app")]
fn configure_windows_graphical_playground_environment() {
    if !cfg!(windows) {
        return;
    }

    let backend_request = env::var("ALIFE_GRAPHICS_BACKEND")
        .or_else(|_| env::var("ALIFE_GRAPHICAL_BACKEND"))
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "dx12".to_string());

    match backend_request.as_str() {
        "existing" => {
            eprintln!(
                "Windows graphical backend: respecting existing WGPU_BACKEND={}.",
                env::var("WGPU_BACKEND").unwrap_or_else(|_| "<unset>".to_string())
            );
        }
        "vulkan" | "vk" => {
            env::set_var("WGPU_BACKEND", "vulkan");
            eprintln!(
                "Windows graphical backend: Vulkan diagnostics requested; injected overlay loader warnings may appear."
            );
        }
        "dx12" | "d3d12" | "auto" => {
            let previous = env::var("WGPU_BACKEND").ok();
            env::set_var("WGPU_BACKEND", "dx12");
            if matches!(previous.as_deref(), Some("dx12")) {
                eprintln!("Windows graphical backend: WGPU_BACKEND=dx12.");
            } else {
                eprintln!(
                    "Windows graphical backend: WGPU_BACKEND=dx12 for clean alpha launch; set ALIFE_GRAPHICS_BACKEND=vulkan for Vulkan diagnostics."
                );
            }
        }
        other => {
            eprintln!(
                "Windows graphical backend: unknown ALIFE_GRAPHICS_BACKEND={other}; using dx12 for clean alpha launch."
            );
            env::set_var("WGPU_BACKEND", "dx12");
        }
    }

    if env::var("ALIFE_SHOW_VULKAN_LOADER_LOGS").is_ok() {
        return;
    }

    const VULKAN_LOADER_FILTER: &str = "wgpu_hal::vulkan::instance=off";
    match env::var("RUST_LOG") {
        Ok(value) if value.contains("wgpu_hal::vulkan::instance") => {}
        Ok(value) if !value.trim().is_empty() => {
            env::set_var("RUST_LOG", format!("{value},{VULKAN_LOADER_FILTER}"));
        }
        _ => {
            env::set_var("RUST_LOG", format!("warn,{VULKAN_LOADER_FILTER}"));
        }
    }
}

#[cfg(not(feature = "bevy-app"))]
fn run_graphical_playground_cli(_args: &[String]) -> Result<String, String> {
    Err("graphical-playground requires feature `bevy-app`; run `cargo run -p alife_game_app --features \"bevy-app gpu-runtime\" --bin alife_game_app -- graphical-playground crates/alife_world/tests/fixtures/gpu_alpha --gpu-mode static-plastic-cpu-shadow-guarded`".to_string())
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
