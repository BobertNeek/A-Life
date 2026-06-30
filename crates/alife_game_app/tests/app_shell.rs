use alife_core::{
    ActionKind, ActionProposal, ActionTarget, BrainTickInput, BrainTickStatus, Confidence,
    CreatureMind, DurationTicks, NormalizedScalar, OrganismId, Tick, Validate, WorldEntityId,
};
use alife_game_app::{
    ca18_creature_selection_ids, ca18_cycle_selected_creature, ca39_drive_audio_vfx_summary,
    compare_visible_world_to_headless, g17_feedback_manifest_path, g17_workspace_root,
    load_visible_world_from_p34_save, project_lod_without_behavior_change,
    render_ca43_crash_summary, run_advanced_gameplay_ux_smoke, run_affordance_loop_smoke,
    run_batched_gpu_runtime_smoke, run_behavior_comparison_lab_smoke,
    run_behavior_tuning_metrics_smoke, run_behavior_tuning_metrics_with_config,
    run_cognition_debug_timeline_smoke, run_content_authoring_smoke,
    run_creature_animation_state_machine_smoke, run_creature_inspector_smoke,
    run_creature_visual_smoke, run_curriculum_authoring_smoke, run_double_buffered_scheduler_smoke,
    run_drive_coupled_audio_vfx_smoke, run_ecological_soak_smoke, run_ecological_soak_with_config,
    run_feedback_polish_smoke, run_full_gpu_runtime_smoke,
    run_gpu_graphics_performance_evidence_smoke, run_gpu_longrun_soak,
    run_gpu_product_hardening_smoke, run_gpu_sustained_learning_soak, run_graphical_controls_smoke,
    run_graphical_ecology_smoke, run_graphical_lifecycle_smoke, run_graphical_population_smoke,
    run_graphical_school_mode_smoke, run_hazard_recovery_smoke, run_headless_app_shell_smoke,
    run_homeostasis_runtime_smoke, run_internal_slm_prior_smoke, run_lifecycle_lineage_smoke,
    run_live_brain_loop_paused_smoke, run_live_brain_loop_smoke, run_longrun_balance_smoke,
    run_longrun_balance_with_config, run_memory_history_journal_smoke,
    run_motor_ring_arbitration_smoke, run_multi_hour_soak_isolation_smoke,
    run_neural_activity_profiler_smoke, run_onboarding_help_smoke, run_onboarding_tutorial_smoke,
    run_platform_package_smoke, run_playable_survival_loop_smoke,
    run_population_performance_lod_smoke, run_population_social_loop_smoke,
    run_procedural_world_travel_smoke, run_product_qa_hardening_smoke,
    run_real_semantic_provider_smoke, run_realtime_wgsl_telemetry_smoke,
    run_release_candidate_smoke, run_runtime_controls_smoke, run_runtime_prereq_diagnostics,
    run_sampled_gpu_runtime_smoke, run_save_load_ux_smoke, run_school_mode_smoke,
    run_semantic_provider_smoke, run_teacher_world_cues_smoke, run_tester_feedback_capture_smoke,
    run_topological_concept_overlay_smoke, run_world_art_style_smoke, run_world_ecology_loop_smoke,
    run_world_editor_smoke, select_visible_world_entity, validate_app_shell_config,
    write_behavior_comparison_lab_report, AppShellLaunchConfig, AutosavePolicy,
    BatchedGpuRuntimeOptions, BehaviorTuningConfig, BehaviorTuningFindingStatus, Ca13TickBuffer,
    Ca39DriveCueKind, Ca39RuntimeCueEvidence, Ca43LogDirectoryPolicy, CadenceTarget,
    CameraNavigationState, ConfigMenuState, CrashSummaryInput, CreatureAnimationState,
    CreatureExpressionState, CreatureLifeStage, CurriculumLessonSaveState,
    DoubleBufferedGraphicalScheduler, EcologicalSoakConfig, FeedbackAssetKind,
    FeedbackAssetManifest, FeedbackEventKind, FullGpuRuntimeSmokeMode, FullGpuRuntimeSmokeOptions,
    GpuLongrunSoakOptions, GpuSustainedLearningSoakOptions, GraphicalGpuRuntimeMode,
    GraphicalGpuRuntimeTelemetry, GraphicalPlaygroundViewMode, InspectorControlPanel,
    LessonManifest, LifecycleEventKind, LifecycleLiveLoop, LifecycleLoopConfig, LifecycleSaveState,
    LiveBrainLoop, LiveBrainTickControl, LodResidency, LongRunBalanceConfig, PackageSmokeKind,
    PlayableSurvivalEventKind, PopulationLiveLoop, PopulationLoopConfig,
    PopulationPerformancePolicy, PopulationSocialEventKind, ProductQaArea, ProductQaStatus,
    RealtimeWgslTelemetrySummary, ReleaseCandidateArea, ReleaseCandidateGateStatus,
    RenderDetailLevel, RuntimeControlCommand, RuntimeControlPanel, RuntimePlaybackState,
    RuntimePrereqDiagnosticsOptions, S08EvidenceStatus, SampledGpuRuntimeOptions,
    SaveSlotDescriptor, SaveSlotKind, SaveSlotManager, SchoolModeSaveState, VisibleMaterialKind,
    VisiblePlaceholderShape, WorldEditCommand, WorldEditorConfig, WorldEditorMode,
    WorldEditorSession, CA18_GRAPHICAL_POPULATION_SCHEMA, CA18_GRAPHICAL_POPULATION_SCHEMA_VERSION,
    CA18_MAX_GRAPHICAL_CREATURES, CA19_GRAPHICAL_ECOLOGY_SCHEMA,
    CA19_GRAPHICAL_ECOLOGY_SCHEMA_VERSION, CA20_GRAPHICAL_LIFECYCLE_SCHEMA,
    CA20_GRAPHICAL_LIFECYCLE_SCHEMA_VERSION, CA21_BEHAVIOR_TUNING_SCHEMA,
    CA21_BEHAVIOR_TUNING_SCHEMA_VERSION, CA21_REQUIRED_DETECTOR_COUNT, CA21_SCENARIO_SWEEP_COUNT,
    CA22_ECOLOGICAL_SOAK_SCHEMA, CA22_ECOLOGICAL_SOAK_SCHEMA_VERSION, CA22_FAST_HEADLESS_TICKS,
    CA22_MANUAL_HEADLESS_TICKS, CA23_GRAPHICAL_SCHOOL_SCHEMA, CA23_GRAPHICAL_SCHOOL_SCHEMA_VERSION,
    CA25_CURRICULUM_AUTHORING_SCHEMA, CA25_CURRICULUM_AUTHORING_SCHEMA_VERSION,
    CA26_REAL_SEMANTIC_PROVIDER_SCHEMA, CA26_REAL_SEMANTIC_PROVIDER_SCHEMA_VERSION,
    CA27_INTERNAL_SLM_PRIOR_SCHEMA, CA27_INTERNAL_SLM_PRIOR_SCHEMA_VERSION,
    CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA, CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA_VERSION,
    CA29_MEMORY_HISTORY_JOURNAL_SCHEMA, CA29_MEMORY_HISTORY_JOURNAL_SCHEMA_VERSION,
    CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA, CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA_VERSION,
    CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA, CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA_VERSION,
    CA31_MAX_REPORT_BYTES, CA32_REALTIME_WGSL_TELEMETRY_SCHEMA,
    CA32_REALTIME_WGSL_TELEMETRY_SCHEMA_VERSION, CA33_BATCHED_GPU_RUNTIME_SCHEMA,
    CA33_BATCHED_GPU_RUNTIME_SCHEMA_VERSION, CA34_SAMPLED_GPU_RUNTIME_SCHEMA,
    CA34_SAMPLED_GPU_RUNTIME_SCHEMA_VERSION, CA36_MIN_MANUAL_TICKS, CA36_SOAK_ISOLATION_SCHEMA,
    CA36_SOAK_ISOLATION_SCHEMA_VERSION, CA37_MIN_PALETTE_MATERIALS,
    CA37_MIN_PROCEDURAL_VISUAL_MAP_TILES, CA37_MIN_WORLD_DRESSING_PROPS,
    CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES, CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES,
    CA37_PROCEDURAL_VISUAL_MAP_HEIGHT_TILES, CA37_PROCEDURAL_VISUAL_MAP_WIDTH_TILES,
    CA37_WORLD_ART_STYLE_SCHEMA, CA37_WORLD_ART_STYLE_SCHEMA_VERSION,
    CA38_CREATURE_ANIMATION_SCHEMA, CA38_CREATURE_ANIMATION_SCHEMA_VERSION,
    CA38_REQUIRED_ANIMATION_STATES, CA39_DRIVE_AUDIO_VFX_SCHEMA,
    CA39_DRIVE_AUDIO_VFX_SCHEMA_VERSION, CA39_REQUIRED_DRIVE_CUE_COUNT,
    CA40_ONBOARDING_TUTORIAL_SCHEMA, CA40_ONBOARDING_TUTORIAL_SCHEMA_VERSION,
    CA40_REQUIRED_CHECKLIST_ITEMS, CA42A_MAX_PLAYER_TERRAIN_OVERLAY_ALPHA,
    CA42_RUNTIME_PREREQ_SCHEMA, CA42_RUNTIME_PREREQ_SCHEMA_VERSION, CA43_TESTER_FEEDBACK_SCHEMA,
    CA43_TESTER_FEEDBACK_SCHEMA_VERSION, G21_ASSET_BUNDLE_SCHEMA, G21_ASSET_BUNDLE_SCHEMA_VERSION,
    G21_PLATFORM_PACKAGE_SCHEMA, G21_PLATFORM_PACKAGE_SCHEMA_VERSION,
};
use alife_semantic::{
    parse_slm_prior_json, project_embedding_to_i8, LlamaCppEmbeddingConfig,
    LlamaCppEmbeddingProvider, LlamaCppSlmPriorConfig, LlamaCppSlmPriorProvider,
    LocalSemanticModelManifest, LocalSlmPriorAsyncQueue, LocalSlmPriorQueue, LocalSlmPriorRequest,
    SemanticProviderCapabilityManifest, CA26_DEFAULT_LLAMA_CPP_EMBEDDING_PORT,
    CA26_EMBEDDING_PROJECTION_DIMS, CA26_LOCAL_MODEL_MANIFEST_SCHEMA,
    CA26_LOCAL_MODEL_MANIFEST_SCHEMA_VERSION, CA26_LOCAL_SEMANTIC_PROVIDER_ID,
    CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS, CA27_DEFAULT_LLAMA_CPP_SLM_PORT,
    CA27_SLM_PRIOR_OUTPUT_SCHEMA, CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION,
};
use alife_world::persistence::{BackendSelection, PortableSaveFile, RuntimeConfig};
use alife_world::WorldObjectKind;
use alife_world::{
    EcologyZoneId, HeadlessActionIds, HeadlessBrainHarness, HeadlessScenarioBuilder, TerrainZone,
    TerrainZoneKind,
};
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

fn p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/alife_world/tests/fixtures/p34")
}

fn gpu_alpha_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/alife_world/tests/fixtures/gpu_alpha")
}

fn test_wgsl_telemetry() -> RealtimeWgslTelemetrySummary {
    RealtimeWgslTelemetrySummary {
        schema: CA32_REALTIME_WGSL_TELEMETRY_SCHEMA,
        schema_version: CA32_REALTIME_WGSL_TELEMETRY_SCHEMA_VERSION,
        tick_marker: 3,
        timing_available: true,
        timing_kind: "host-observed-active-wgsl-tick",
        upload_ms: 0.10,
        compute_submit_poll_ms: 0.80,
        compact_readback_ms: 0.20,
        cpu_shadow_ms: 0.15,
        total_gpu_runtime_ms: 1.10,
        routing_total_tiles: 8,
        routing_active_tiles: 3,
        routing_skipped_tiles: 5,
        routing_active_synapses: 384,
        compact_readback_bytes: 64,
        nonblocking_hot_path: true,
        unavailable_reason: None,
    }
}

fn gpu_plasticity_env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn s09_content_tutorial_authoring_pack_is_coherent_and_tiny() {
    let summary = run_content_authoring_smoke().unwrap();

    assert_eq!(summary.schema, alife_game_app::S09_CONTENT_TUTORIAL_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::S09_CONTENT_TUTORIAL_SCHEMA_VERSION
    );
    assert_eq!(summary.content.pack_id, "s09-first-run-tutorial-pack");
    assert_eq!(summary.content.world_presets, 1);
    assert_eq!(summary.content.lesson_packs, 1);
    assert_eq!(summary.content.creature_presets, 1);
    assert_eq!(summary.content.scenario_packs, 1);
    assert!(summary.content.largest_file_bytes < alife_game_app::S09_MAX_CONTENT_FILE_BYTES);
    assert!(summary.content.tiny_files_under_limit);
    assert!(summary.content.has_food);
    assert!(summary.content.has_hazard);
    assert!(summary.content.has_social_peer);
    assert!(summary.content.has_school_token);
    assert!(summary.content.has_resource_zone);
    assert!(summary.content.missing_required_rejected);
    assert!(summary.new_tester_headless_ready);
    assert!(!summary.hidden_provider_required);
    assert!(!summary.huge_assets_committed);
    summary.validate().unwrap();
}

#[test]
fn s09_tutorial_commands_are_current_and_school_cues_remain_perception_only() {
    let summary = run_content_authoring_smoke().unwrap();

    assert_eq!(summary.content.perception_only_lesson_steps, 3);
    assert!(summary.school_cues_perception_only);
    assert_eq!(summary.tutorial.graphical_manual_status, "manual-optional");
    assert!(summary
        .tutorial
        .recommended_commands
        .iter()
        .any(|command| command.contains("content-authoring-smoke")));
    assert!(summary
        .tutorial
        .recommended_commands
        .iter()
        .any(|command| command.contains("onboarding-help-smoke")));
    assert!(summary.tutorial.recommended_commands.iter().all(|command| {
        !command.contains("gpu-report")
            && !command.contains("ALIFE_GPU_BACKEND")
            && !command.contains("bash scripts/check.sh")
    }));
}

#[test]
fn s10_external_playtest_candidate_docs_are_current_and_artifact_safe() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = std::fs::read_to_string(
        root.join("docs/productization/S10_EXTERNAL_PLAYTEST_CANDIDATE_REPORT.md"),
    )
    .unwrap();
    let checklist =
        std::fs::read_to_string(root.join("docs/productization/S10_EXTERNAL_TESTER_CHECKLIST.md"))
            .unwrap();

    for text in [&report, &checklist] {
        assert!(text.contains(
            "cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke"
        ));
        assert!(
            text.contains("cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke")
        );
        assert!(text.contains(
            "cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke"
        ));
        assert!(text.contains(
            "cargo run -p alife_game_app --bin alife_game_app -- content-authoring-smoke"
        ));
        assert!(text.contains("cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json"));
        assert!(text.contains("powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1"));
        assert!(text.contains("ALIFE_GPU_RUNTIME_BACKEND=static"));
        assert!(text.contains("--gpu-runtime"));
        assert!(text.contains("git ls-files target dist target/artifacts graphify-out"));
        assert!(text.contains("not commit") || text.contains("not be committed"));
        assert!(!text.contains("gpu-report"));
        assert!(!text.contains("ALIFE_GPU_BACKEND"));
        assert!(!text.contains("bash scripts/check.sh"));
    }

    assert!(report.contains("No release blockers are known"));
    assert!(report.contains("No S12, G25, or P37 was created"));
    assert!(checklist.contains("S10 does not create a release tag"));
}

#[test]
fn s11_release_decision_docs_are_honest_and_stop_the_chain() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = std::fs::read_to_string(
        root.join("docs/productization/S11_FINAL_PRODUCTIZATION_REPORT.md"),
    )
    .unwrap();
    let decision =
        std::fs::read_to_string(root.join("docs/productization/S11_RELEASE_DECISION_PACKET.md"))
            .unwrap();
    let roadmap =
        std::fs::read_to_string(root.join("docs/productization/S11_NEXT_STAGE_ROADMAP.md"))
            .unwrap();

    for text in [&report, &decision, &roadmap] {
        assert!(!text.contains("gpu-report"));
        assert!(!text.contains("ALIFE_GPU_BACKEND"));
        assert!(!text.contains("bash scripts/check.sh"));
        assert!(text.contains("explicit") || text.contains("Do not"));
    }

    for required in [
        "cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke",
        "cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke",
        "cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke",
        "cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json",
        "cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime",
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -DryRun",
    ] {
        assert!(report.contains(required));
    }

    assert!(report.contains("alpha / external playtest candidate"));
    assert!(report.contains("No release tag was created"));
    assert!(report.contains("No S12, G25, P37"));
    assert!(decision.contains("defer release tag"));
    assert!(decision.contains("Do not run this without explicit user approval"));
    assert!(roadmap.contains("not an implementation plan"));
    assert!(roadmap.contains("Do not create S12 automatically"));
}

#[test]
fn first_graphical_alpha_playtest_docs_and_launcher_are_current() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let checklist = std::fs::read_to_string(
        root.join("docs/productization/FIRST_GRAPHICAL_ALPHA_PLAYTEST_CHECKLIST.md"),
    )
    .unwrap();
    let report = std::fs::read_to_string(
        root.join("docs/productization/FIRST_GRAPHICAL_ALPHA_PLAYTEST_REPORT.md"),
    )
    .unwrap();
    let launcher =
        std::fs::read_to_string(root.join("scripts/run_graphical_playground.ps1")).unwrap();

    for text in [&checklist, &report, &launcher] {
        assert!(text.contains("A-Life GPU Alpha Playground"));
        assert!(text.contains("-GpuMode static-plastic-cpu-shadow-guarded"));
        assert!(text.contains("CPU fallback") || text.contains("fallback"));
        assert!(!text.contains("gpu-report"));
        assert!(!text.contains("ALIFE_GPU_BACKEND"));
        assert!(!text.contains("bash scripts/check.sh"));
    }

    assert!(checklist.contains("No Bevy Entity IDs"));
    assert!(report.contains("CpuShadowGuardedStaticPlusLiveHShadow"));
    assert!(report.contains("not full action-authoritative"));
    assert!(launcher.contains("Reset/restart"));
    assert!(launcher.contains("[string]$ViewMode = \"player\""));
    assert!(launcher.contains("--view-mode"));
    assert!(launcher.contains("-ViewMode dev-overlay"));
    assert!(launcher.contains("[string]$GraphicsBackend"));
    assert!(launcher.contains("overriding inherited WGPU_BACKEND"));
    assert!(launcher.contains("-GraphicsBackend vulkan"));

    let app_cli =
        std::fs::read_to_string(root.join("crates/alife_game_app/src/bin/alife_game_app.rs"))
            .unwrap();
    assert!(app_cli.contains("configure_windows_graphical_playground_environment"));
    assert!(app_cli.contains("WGPU_BACKEND"));
    assert!(app_cli.contains("dx12 for clean alpha launch"));
    assert!(app_cli.contains("ALIFE_GRAPHICS_BACKEND=vulkan"));
}

#[test]
fn feedback_polish_maps_existing_outcomes_into_non_authoritative_cues() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_feedback_polish_smoke(&launch).unwrap();
    let labels = summary.event_labels();

    assert_eq!(summary.schema, alife_game_app::G17_FEEDBACK_POLISH_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G17_FEEDBACK_POLISH_SCHEMA_VERSION
    );
    assert_eq!(summary.sealed_outcome_event_count, 4);
    assert!(summary.non_authoritative);
    assert!(summary
        .events
        .iter()
        .all(|event| event.non_authoritative && !event.channels.is_empty()));
    assert!(labels.contains(&FeedbackEventKind::FoodReward.label()));
    assert!(labels.contains(&FeedbackEventKind::MissingAffordance.label()));
    assert!(labels.contains(&FeedbackEventKind::HazardPain.label()));
    assert!(labels.contains(&FeedbackEventKind::SleepTransition.label()));
    assert!(labels.contains(&FeedbackEventKind::TeacherCue.label()));
    assert!(labels.contains(&FeedbackEventKind::SaveCompleted.label()));
    assert!(labels.contains(&FeedbackEventKind::LoadCompleted.label()));
    assert!(labels.contains(&FeedbackEventKind::SelectionChanged.label()));
}

#[test]
fn feedback_polish_asset_manifest_validates_optional_fallbacks() {
    let manifest = FeedbackAssetManifest::from_json_file(g17_feedback_manifest_path()).unwrap();
    let validation = manifest.validate_with_root(g17_workspace_root()).unwrap();

    assert!(validation.entry_count >= 4);
    assert!(validation.optional_fallback_count > 0);
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.kind == FeedbackAssetKind::AudioCue));
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.kind == FeedbackAssetKind::VfxCue));
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.kind == FeedbackAssetKind::AnimationCurve));
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.kind == FeedbackAssetKind::NotificationStyle));
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.asset_id == "ca39-audio-learning-pulse"
            && entry.optional
            && entry.procedural_fallback));
    assert!(manifest
        .entries
        .iter()
        .any(|entry| entry.asset_id == "ca39-vfx-learning-pulse"
            && entry.optional
            && entry.procedural_fallback));
}

#[test]
fn feedback_polish_rejects_missing_required_asset() {
    let mut manifest = FeedbackAssetManifest::from_json_file(g17_feedback_manifest_path()).unwrap();
    let missing = manifest
        .entries
        .iter_mut()
        .find(|entry| entry.asset_id == "g17-audio-hazard-pulse")
        .expect("fixture should include a missing optional hazard pulse");
    missing.optional = false;
    missing.procedural_fallback = false;

    let err = manifest
        .validate_with_root(g17_workspace_root())
        .unwrap_err();
    assert!(err.to_string().contains("required feedback asset"));
}

#[test]
fn ca39_drive_coupled_audio_vfx_maps_drive_milestones_without_authority() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let feedback = run_feedback_polish_smoke(&launch).unwrap();
    let evidence = Ca39RuntimeCueEvidence {
        selected_backend: "GpuPlastic".to_string(),
        fallback_reason: None,
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow".to_string(),
        sealed_patches: 4,
        h_shadow_applications: 3,
        cpu_shadow_gate_preserved: true,
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };

    let summary = ca39_drive_audio_vfx_summary(&feedback, &evidence).unwrap();

    assert_eq!(summary.schema, CA39_DRIVE_AUDIO_VFX_SCHEMA);
    assert_eq!(summary.schema_version, CA39_DRIVE_AUDIO_VFX_SCHEMA_VERSION);
    assert_eq!(summary.cues.len(), CA39_REQUIRED_DRIVE_CUE_COUNT);
    assert_eq!(summary.active_cue_count, CA39_REQUIRED_DRIVE_CUE_COUNT);
    assert!(summary.no_action_authority);
    assert!(summary.no_weight_authority);
    assert!(summary.no_cognition_mutation);
    assert!(summary.no_large_assets_added);
    assert!(summary.cpu_shadow_gate_preserved);
    assert!(!summary.full_action_authoritative_claim);
    assert!(summary
        .cues
        .iter()
        .any(|cue| cue.kind == Ca39DriveCueKind::HungerSatisfaction
            && cue.audio_asset_id.as_deref() == Some("g17-audio-food-chime")));
    assert!(summary
        .cues
        .iter()
        .any(|cue| cue.kind == Ca39DriveCueKind::HazardPain
            && cue.vfx_asset_id.as_deref() == Some("g17-vfx-hazard-flash")));
    assert!(summary
        .cues
        .iter()
        .any(|cue| cue.kind == Ca39DriveCueKind::SleepRest
            && cue.audio_asset_id.as_deref() == Some("g17-audio-sleep-soft")));
    assert!(summary
        .cues
        .iter()
        .any(|cue| cue.kind == Ca39DriveCueKind::LearningPulse
            && cue.active
            && cue.vfx_asset_id.as_deref() == Some("ca39-vfx-learning-pulse")));
    assert!(!summary.compact_overlay_text().contains("Entity("));
}

#[test]
fn ca39_drive_coupled_audio_vfx_smoke_runs_with_honest_runtime_claim() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_drive_coupled_audio_vfx_smoke(&launch).unwrap();

    assert_eq!(summary.schema, CA39_DRIVE_AUDIO_VFX_SCHEMA);
    assert_eq!(summary.cues.len(), CA39_REQUIRED_DRIVE_CUE_COUNT);
    assert!(summary.sealed_feedback_sources >= 4);
    assert!(summary.no_action_authority);
    assert!(summary.no_weight_authority);
    assert!(summary.no_cognition_mutation);
    assert!(summary.no_active_bulk_readback);
    assert!(summary.cpu_shadow_gate_preserved);
    assert!(!summary.full_action_authoritative_claim);
    assert!(!summary
        .product_runtime_claim
        .contains("FullActionAuthoritative"));
}

#[test]
fn ci_headless_app_shell_smoke_uses_p34_config_without_graphics() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_headless_app_shell_smoke(&launch).unwrap();
    assert_eq!(summary.seed, 4242);
    assert_eq!(summary.requested_backend, BackendSelection::CpuReference);
    assert!(!summary.gpu_backend_enabled);
    assert!(!summary.semantic_enabled);
    assert!(!summary.school_enabled);
    assert!(summary.logging_enabled);
    assert!(!summary.graphics_required_for_default_path);
}

#[test]
fn explicit_config_validation_rejects_missing_required_manifest() {
    let mut launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    launch.asset_manifest_path = launch.fixture_root.join("missing_asset_manifest.json");
    assert!(validate_app_shell_config(&launch).is_err());
}

#[test]
fn visible_world_signature_matches_restored_headless_fixture_objects() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    compare_visible_world_to_headless(&presentation).unwrap();
    assert_eq!(presentation.object_count, 2);
    assert_eq!(presentation.kind_count(WorldObjectKind::Agent), 1);
    assert_eq!(presentation.kind_count(WorldObjectKind::Food), 1);
    assert_eq!(presentation.stable_ids()[0].raw(), 1);
    assert_eq!(presentation.stable_ids()[1].raw(), 2);
}

#[test]
fn gpu_alpha_fixture_adds_multi_creature_hazard_and_obstacle_markers_without_changing_stable_id_contract(
) {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    compare_visible_world_to_headless(&presentation).unwrap();
    assert_eq!(presentation.object_count, 12);
    assert_eq!(presentation.kind_count(WorldObjectKind::Agent), 3);
    assert_eq!(presentation.kind_count(WorldObjectKind::Food), 3);
    assert_eq!(presentation.kind_count(WorldObjectKind::Hazard), 3);
    assert_eq!(presentation.kind_count(WorldObjectKind::Obstacle), 3);
    assert_eq!(
        presentation
            .stable_ids()
            .iter()
            .map(|id| id.raw())
            .collect::<Vec<_>>(),
        (1_u64..=12).collect::<Vec<_>>()
    );
    let (min_x, max_x) = presentation
        .objects
        .iter()
        .map(|object| object.position.x)
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), x| {
            (min.min(x), max.max(x))
        });
    let (min_z, max_z) = presentation
        .objects
        .iter()
        .map(|object| object.position.z)
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), z| {
            (min.min(z), max.max(z))
        });
    assert!(max_x - min_x >= 30.0);
    assert!(max_z - min_z >= 18.0);
    let creature_ids = ca18_creature_selection_ids(&presentation)
        .iter()
        .map(|id| id.raw())
        .collect::<Vec<_>>();
    assert_eq!(creature_ids, vec![1, 5, 6]);
    assert_eq!(
        ca18_cycle_selected_creature(&presentation, WorldEntityId(1)),
        Some(WorldEntityId(5))
    );
    let obstacle = presentation
        .objects
        .iter()
        .find(|object| object.kind == WorldObjectKind::Obstacle)
        .expect("gpu alpha fixture should include a real obstacle marker");
    assert_eq!(obstacle.stable_id.raw(), 4);
    assert_eq!(obstacle.label, "stone");
    assert_eq!(obstacle.shape, VisiblePlaceholderShape::ObstacleCube);
    assert_eq!(obstacle.material, VisibleMaterialKind::Obstacle);
}

#[test]
fn ca18_graphical_population_smoke_reports_bounded_stable_id_creature_cycle() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_graphical_population_smoke(&launch).unwrap();

    assert_eq!(summary.schema, CA18_GRAPHICAL_POPULATION_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA18_GRAPHICAL_POPULATION_SCHEMA_VERSION
    );
    assert_eq!(summary.creature_count, 3);
    assert_eq!(summary.population_cap, CA18_MAX_GRAPHICAL_CREATURES);
    assert_eq!(
        summary
            .selectable_stable_ids
            .iter()
            .map(|id| id.raw())
            .collect::<Vec<_>>(),
        vec![1, 5, 6]
    );
    assert_eq!(summary.selected_stable_id, WorldEntityId(1));
    assert!(!summary.social_cues.is_empty());
    assert!(summary.bounded_performance);
    assert!(summary.stable_id_selection_only);
    assert!(summary.no_bevy_entity_ids_in_player_text);
    assert!(summary.cpu_shadow_gate_preserved);
    assert_eq!(
        summary.product_runtime_claim,
        "CpuShadowGuardedStaticPlusLiveHShadow"
    );
    let overlay = summary.compact_overlay_text();
    assert!(overlay.contains("Population: 3/"));
    assert!(overlay.contains("Tab stable IDs only"));
    assert!(overlay.contains("CPU shadow gate preserved"));
    assert!(!overlay.contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn ca19_graphical_ecology_smoke_reports_zones_resource_cycle_and_roundtrip() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_graphical_ecology_smoke(&launch).unwrap();

    assert_eq!(summary.schema, CA19_GRAPHICAL_ECOLOGY_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA19_GRAPHICAL_ECOLOGY_SCHEMA_VERSION
    );
    assert!(summary.terrain_zones.len() >= 4);
    assert!(summary.resources.len() >= 3);
    assert!(summary.hazard_pressure_zone_count >= 2);
    assert!(summary.initial_metrics.active_resources >= 1);
    assert!(summary.cycled_metrics.resources_regrown >= 1);
    assert!(summary.cycled_metrics.resources_spawned >= 1);
    assert!(summary.resource_regen_visible);
    assert!(summary.food_spawned_indicator_visible);
    assert!(summary.save_load_roundtrip_preserved);
    assert!(summary.stable_ids_only);
    assert!(summary.display_only);
    assert_eq!(
        summary.product_runtime_claim,
        "CpuShadowGuardedStaticPlusLiveHShadow"
    );

    let overlay = summary.compact_overlay_text();
    assert!(overlay.contains("Ecology: zones=4"));
    assert!(overlay.contains("hazard zones=3"));
    assert!(overlay.contains("regrown="));
    assert!(overlay.contains("spawned="));
    assert!(overlay.contains("roundtrip=true"));
    assert!(!overlay.contains("Entity("));
    summary.validate().unwrap();
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca19_graphical_ecology_overlay_text_is_display_only_and_stable_id_safe() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let ecology_summary = run_graphical_ecology_smoke(&launch).unwrap();
    assert!(ecology_summary.terrain_zones.len() >= 4);
    assert!(ecology_summary.resources.len() >= 3);
    assert!(ecology_summary.hazard_pressure_zone_count >= 2);
    assert!(ecology_summary.resource_regen_visible);
    assert!(ecology_summary.food_spawned_indicator_visible);
    assert!(ecology_summary.save_load_roundtrip_preserved);
    assert!(ecology_summary
        .terrain_zones
        .iter()
        .any(|zone| zone.kind == TerrainZoneKind::HazardField));

    let overlay = alife_game_app::bevy_shell::ca19_ecology_overlay_text(&ecology_summary);
    assert!(overlay.contains("Resource Ecology"));
    assert!(overlay.contains("Terrain: grove:berry-grove"));
    assert!(overlay.contains("hazard zones=3"));
    assert!(overlay.contains("Resource cycle"));
    assert!(overlay.contains("Boundary: terrain/resource visuals cannot emit actions"));
    assert!(!overlay.contains("Entity("));
    assert!(!overlay.contains("full action-authoritative"));
}

#[test]
fn ca37_world_art_style_smoke_validates_palette_props_and_manifest() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_world_art_style_smoke(&launch).unwrap();

    assert_eq!(summary.schema, CA37_WORLD_ART_STYLE_SCHEMA);
    assert_eq!(summary.schema_version, CA37_WORLD_ART_STYLE_SCHEMA_VERSION);
    assert!(summary.palette.len() >= CA37_MIN_PALETTE_MATERIALS);
    assert!(summary.dressing_props.len() >= CA37_MIN_WORLD_DRESSING_PROPS);
    assert!(summary.procedural_visual_map);
    assert_eq!(
        summary.visual_map_width_tiles,
        CA37_PROCEDURAL_VISUAL_MAP_WIDTH_TILES
    );
    assert_eq!(
        summary.visual_map_height_tiles,
        CA37_PROCEDURAL_VISUAL_MAP_HEIGHT_TILES
    );
    assert_eq!(
        summary.visual_map_tile_count,
        CA37_MIN_PROCEDURAL_VISUAL_MAP_TILES
    );
    assert!(summary.visual_map_span_world_units >= 60.0);
    assert!(summary.visual_map_width_tiles >= 4096);
    assert!(summary.visual_map_height_tiles >= 4096);
    assert!(summary.visual_map_span_world_units >= 4096.0);
    assert_eq!(
        summary.viewport_width_tiles,
        CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES
    );
    assert_eq!(
        summary.viewport_height_tiles,
        CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES
    );
    assert_eq!(
        summary.viewport_tile_count,
        CA37_PROCEDURAL_VIEWPORT_WIDTH_TILES * CA37_PROCEDURAL_VIEWPORT_HEIGHT_TILES
    );
    assert!(summary.map_to_viewport_tile_ratio > 10_000.0);
    assert!(summary.local_viewport_is_smaller_than_map);
    assert!(summary.offscreen_stable_world_object_count >= 4);
    assert!(summary.true_large_world_exploration);
    assert!(summary.camera_can_pan_large_world);
    assert!(summary.distributed_stable_world_objects);
    assert!(summary.generated_terrain_guides_resource_hazard_placement);
    assert!(summary.ecology_zone_count >= 4);
    assert!(summary.resource_zone_materials > 0);
    assert!(summary.hazard_zone_materials > 0);
    assert!(summary.app_bundle_manifest_validated);
    assert!(summary.placeholder_art_entries >= summary.palette.len() + 4);
    assert!(summary.display_only);
    assert!(summary.stable_ids_only);
    assert!(summary.no_runtime_tile_encoding);
    assert!(summary.no_physics_or_sensory_changes);
    assert_eq!(
        summary.product_runtime_claim,
        "CpuShadowGuardedStaticPlusLiveHShadow"
    );
    assert!(summary
        .palette
        .iter()
        .any(|material| material.id == "resource-grove"));
    assert!(summary
        .palette
        .iter()
        .any(|material| material.id == "hazard-pressure"));
    assert!(summary
        .dressing_props
        .iter()
        .any(|prop| prop.material_id == "stone-dressing"));

    let overlay = summary.compact_overlay_text();
    assert!(overlay.contains("World Map: seeded procedural terrain"));
    assert!(overlay.contains("Viewport: local camera slice"));
    assert!(overlay.contains("off-screen stable objects"));
    assert!(overlay.contains("Exploration: pan/follow to leave this slice"));
    assert!(overlay.contains("stable-ID creatures/resources/hazards distributed"));
    assert!(!overlay.contains("Entity("));
    assert!(!overlay.contains("full action-authoritative"));
    summary.validate().unwrap();
}

#[test]
fn ca44a_procedural_world_travel_smoke_streams_seeded_chunks_without_rendering() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());

    let summary = run_procedural_world_travel_smoke(&launch).unwrap();

    summary.validate().unwrap();
    assert_eq!(summary.seed, 4242);
    assert_eq!(summary.stable_id, WorldEntityId(1));
    assert!(summary.route_steps >= 6);
    assert!(
        summary.total_unique_materialized_chunks > summary.max_active_chunk_count,
        "creature travel should materialize more chunks than one active camera window"
    );
    assert!(summary.total_content_candidates_seen > 0);
    assert!(summary.generated_without_rendering);
    assert!(!summary.rendering_required);
    assert!(!summary.chunks_exist_without_creature_presence);
    assert!(summary.materialized_only_near_creature_anchors);
    assert!(summary.bounded_for_creature_context);
    assert!(!summary.can_emit_actions);
    assert!(!summary.can_rewrite_weights);
    assert_eq!(
        summary.world_generation_claim,
        "SeededCreatureAnchoredNoRenderChunks"
    );
    assert!(
        summary
            .travel_report
            .steps
            .iter()
            .skip(1)
            .any(|step| step.newly_materialized_chunk_count > 0 && step.retired_chunk_count > 0),
        "travel route should stream active chunks as the creature leaves one area"
    );
}

#[test]
fn ca38_creature_animation_state_machine_maps_required_states_without_authority() {
    let summary = run_creature_animation_state_machine_smoke().unwrap();

    assert_eq!(summary.schema, CA38_CREATURE_ANIMATION_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA38_CREATURE_ANIMATION_SCHEMA_VERSION
    );
    assert!(summary.states.len() >= CA38_REQUIRED_ANIMATION_STATES);
    assert!(summary.display_only);
    assert!(summary.inspector_accurate);
    assert!(summary.fallback_visible);
    assert!(summary.stable_ids_only);
    assert!(summary.no_action_authority);
    assert!(summary.no_cognition_mutation);
    assert_eq!(
        summary.product_runtime_claim,
        "CpuShadowGuardedStaticPlusLiveHShadow"
    );
    for required in [
        "idle-breathe",
        "move-lean",
        "eat-reach",
        "flee-alert",
        "sleep-curl",
        "pain-flinch",
        "social-signal",
    ] {
        assert!(summary.states.iter().any(|pose| pose.pose_id == required));
    }
    assert!(summary.states.iter().all(|pose| pose.display_only));
    assert!(summary
        .states
        .iter()
        .all(|pose| pose.scale_x.is_finite() && pose.scale_y.is_finite()));
    assert!(!summary.signature_line().contains("Entity("));
    assert!(!summary
        .signature_line()
        .contains("full action-authoritative"));
    summary.validate().unwrap();
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca37_world_art_props_are_display_only_and_stable_id_safe() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch)
            .expect("CA37 graphical world art shell should build");
    app.update();

    let art_summary = app
        .world()
        .resource::<alife_game_app::bevy_shell::GraphicalWorldArtStyleResource>()
        .summary
        .clone();
    assert!(art_summary.display_only);
    assert!(art_summary.stable_ids_only);
    assert!(art_summary.no_physics_or_sensory_changes);
    assert!(art_summary.local_viewport_is_smaller_than_map);
    assert!(art_summary.map_to_viewport_tile_ratio > 20.0);
    assert!(art_summary.offscreen_stable_world_object_count >= 4);

    let mut query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalWorldArtProp>();
    let props = query.iter(app.world()).copied().collect::<Vec<_>>();
    assert_eq!(props.len(), art_summary.dressing_props.len());
    assert!(props.iter().all(|prop| prop.display_only));
    assert!(props
        .iter()
        .any(|prop| prop.material_id == "hazard-pressure"));
    assert!(props
        .iter()
        .any(|prop| prop.anchored_stable_id == Some(WorldEntityId(2))));
    assert!(props
        .iter()
        .any(|prop| prop.anchored_stable_id == Some(WorldEntityId(3))));
    let mut tile_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalWorldArtTerrainTile>();
    let tiles = tile_query.iter(app.world()).copied().collect::<Vec<_>>();
    let field = app
        .world()
        .resource::<alife_game_app::bevy_shell::GraphicalProceduralTerrainFieldResource>()
        .clone();
    assert!(
        field.virtual_map_width_tiles * field.virtual_map_height_tiles
            >= CA37_MIN_PROCEDURAL_VISUAL_MAP_TILES,
        "CA37 virtual terrain should keep at least its seeded map size"
    );
    assert!(field.generated_without_rendering);
    assert!(field.creature_anchor_count >= 1);
    assert!(!field.active_world_chunks.is_empty());
    assert!(
        tiles.len() >= 17 * 11,
        "default Player View should render the local procedural chunk window with asset-backed terrain tiles"
    );
    assert!(tiles.iter().all(|tile| {
        tile.display_only
            && tile.viewport_slice
            && tile.opacity >= 0.0
            && tile.opacity <= 0.02
            && tile.tile_size_pixels > 0.0
            && tile.material_id != "debug"
    }));
    assert!(
        field.materialized_only_near_active_views,
        "large terrain map should remain virtual until active view/creature anchors need chunks"
    );
    assert!(
        field.materialized_tiles.len() >= 17 * 11,
        "procedural terrain should be generated in the field ledger while Player View renders the active chunk slice"
    );
    let mut chunk_query =
        app.world_mut()
            .query::<&alife_game_app::bevy_shell::GraphicalProceduralTerrainChunkTile>();
    let chunks = chunk_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert_eq!(
        chunks.len(),
        tiles.len(),
        "Player View terrain tiles should carry chunk provenance for the rendered active slice"
    );
    assert!(chunks.iter().all(|chunk| {
        chunk.creature_authoritative_chunk
            && !chunk.rendering_required_for_generation
            && chunk.materialized_only_near_active_views
    }));
    assert!(field.virtual_map_width_tiles >= 97);
    assert!(field.virtual_map_height_tiles >= 73);
    assert!(
        field.chunk_radius_x < (field.virtual_map_width_tiles as i32 / 2)
            && field.chunk_radius_z < (field.virtual_map_height_tiles as i32 / 2),
        "active terrain chunks should be smaller than the full virtual map"
    );
    assert!(
        field
            .materialized_tiles
            .iter()
            .any(|(tile_x, _)| *tile_x < 0)
            && field
                .materialized_tiles
                .iter()
                .any(|(tile_x, _)| *tile_x > 0)
            && field
                .materialized_tiles
                .iter()
                .any(|(_, tile_z)| *tile_z < 0)
            && field
                .materialized_tiles
                .iter()
                .any(|(_, tile_z)| *tile_z > 0),
        "initial procedural ledger should cover the local world around the selected creature"
    );
    assert!(art_summary
        .palette
        .iter()
        .any(|material| material.id == "safe-grass"));
    assert!(art_summary
        .palette
        .iter()
        .any(|material| material.id == "resource-grove"));
    assert!(art_summary
        .palette
        .iter()
        .any(|material| material.id == "hazard-pressure"));

    let overlay = alife_game_app::bevy_shell::ca37_world_art_overlay_text(&art_summary);
    let legend = alife_game_app::bevy_shell::readability_legend_overlay_text();
    let controls = alife_game_app::bevy_shell::ca05_controls_bar_text();
    let mut live = LiveBrainLoop::from_p34_launch(&launch.app_launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    panel
        .apply_command(&mut live, RuntimeControlCommand::RunForTicks(3))
        .unwrap();
    let gpu = GraphicalGpuRuntimeTelemetry {
        requested_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        selected_backend: "GpuPlastic".to_string(),
        fallback_reason: None,
        hardware_identifier: Some("local-test".to_string()),
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow".to_string(),
        gpu_static_dispatched_ticks: 3,
        gpu_scores_used_for_proposals: true,
        cpu_shadow_parity: true,
        parity_failures: 0,
        sealed_patches: panel.sealed_patch_count,
        h_shadow_applications: 2,
        last_h_shadow_delta: 0.0125,
        compact_readback_bytes: 64,
        post_seal_readback_bytes: 64,
        total_gpu_runtime_ms: 1.25,
        wgsl: test_wgsl_telemetry(),
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };
    let player_hud = alife_game_app::bevy_shell::graphical_player_status_overlay_text(&panel, &gpu);
    assert!(overlay.contains("World Map: seeded procedural terrain"));
    assert!(overlay.contains("Viewport: local camera slice"));
    assert!(overlay.contains("off-screen stable objects"));
    assert!(overlay.contains("stable-ID creatures/resources/hazards distributed"));
    assert!(legend.contains("Viewport: local camera slice"));
    assert!(legend.contains("off-screen stable-ID food"));
    assert!(legend.contains("Terrain guides placement"));
    assert!(controls.contains("Controls: click"));
    assert!(controls.contains("[!] hazard"));
    assert!(player_hud.contains("A-Life GPU Alpha"));
    assert!(player_hud.contains("GPU ON"));
    assert!(player_hud.contains("L2"));
    assert!(player_hud.lines().count() <= 4);
    assert!(!player_hud.contains("stable:"));
    assert!(!player_hud.contains("Patch:"));
    assert!(!player_hud.contains("Concepts:"));
    assert!(!player_hud.contains("Memory:"));
    assert!(!player_hud.contains("Neural:"));
    assert!(!overlay.contains("Entity("));
    assert!(!legend.contains("Entity("));
    assert!(!controls.contains("Entity("));
    assert!(!player_hud.contains("Entity("));
    assert!(!overlay.contains("full action-authoritative"));
    assert!(!player_hud.contains("full action-authoritative"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca38_creature_animation_pose_is_display_only_and_readable() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch)
            .expect("CA38 graphical animation shell should build");
    app.update();

    let animation_summary = app
        .world()
        .resource::<alife_game_app::bevy_shell::GraphicalCreatureAnimationResource>()
        .summary
        .clone();
    assert!(animation_summary.display_only);
    assert!(animation_summary.fallback_visible);
    assert!(animation_summary.no_action_authority);
    assert!(animation_summary.no_cognition_mutation);

    let mut pose_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalCreatureAnimationPose>();
    let poses = pose_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert!(poses.len() >= 3);
    assert!(poses.iter().all(|pose| pose.display_only));
    assert!(poses.iter().all(|pose| pose.stable_id.raw() > 0));
    assert!(poses.iter().any(|pose| pose.pose_id == "idle-breathe"));

    let mut live = LiveBrainLoop::from_p34_launch(&launch.app_launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    panel
        .apply_command(&mut live, RuntimeControlCommand::RunForTicks(3))
        .unwrap();
    let gpu = GraphicalGpuRuntimeTelemetry {
        requested_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        selected_backend: "GpuPlastic".to_string(),
        fallback_reason: None,
        hardware_identifier: Some("local-test".to_string()),
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow".to_string(),
        gpu_static_dispatched_ticks: 3,
        gpu_scores_used_for_proposals: true,
        cpu_shadow_parity: true,
        parity_failures: 0,
        sealed_patches: panel.sealed_patch_count,
        h_shadow_applications: 2,
        last_h_shadow_delta: 0.0125,
        compact_readback_bytes: 64,
        post_seal_readback_bytes: 64,
        total_gpu_runtime_ms: 1.25,
        wgsl: test_wgsl_telemetry(),
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };
    let player_hud = alife_game_app::bevy_shell::graphical_player_status_overlay_text(&panel, &gpu);
    let debug_hud =
        alife_game_app::bevy_shell::graphical_full_debug_status_overlay_text(&panel, &gpu);
    assert!(!player_hud.contains("Pose:"));
    assert!(!player_hud.contains("Gate: CPU shadow; full_auth=false"));
    assert!(debug_hud.contains("Pose:"));
    assert!(debug_hud.contains("Gate: CPU shadow; full_auth=false"));
    assert!(!player_hud.contains("Entity("));
    assert!(!player_hud.contains("full action-authoritative"));
}

#[test]
fn ca20_graphical_lifecycle_smoke_reports_birth_death_lineage_and_roundtrip() {
    let summary = run_graphical_lifecycle_smoke().unwrap();

    assert_eq!(summary.schema, CA20_GRAPHICAL_LIFECYCLE_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA20_GRAPHICAL_LIFECYCLE_SCHEMA_VERSION
    );
    assert!(summary.living_population <= summary.population_cap);
    assert_eq!(summary.births, 1);
    assert_eq!(summary.deaths, 1);
    assert_eq!(summary.lineage_count, 1);
    assert!(summary.genetic_lifetime_separated);
    assert!(summary.birth_weight_assets_are_initializers);
    assert!(summary.save_load_lineages_roundtrip);
    assert!(summary
        .event_rows
        .iter()
        .any(|row| row.label == LifecycleEventKind::Birth.label()));
    assert!(summary
        .event_rows
        .iter()
        .any(|row| row.label == LifecycleEventKind::Death.label()));
    assert!(!summary.signature.contains("Entity("));

    let overlay = summary.compact_overlay_text();
    assert!(overlay.contains("Lifecycle"));
    assert!(overlay.contains("Births:1"));
    assert!(overlay.contains("Deaths:1"));
    assert!(overlay.contains("Genetic fixed separate from lifetime: true"));
    assert!(overlay.contains("Save/load lineages: true"));
    assert!(!overlay.contains("Entity("));
    summary.validate().unwrap();
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca20_lifecycle_overlay_is_player_facing_and_boundary_safe() {
    let summary = run_graphical_lifecycle_smoke().unwrap();
    let overlay = alife_game_app::bevy_shell::ca20_lifecycle_overlay_text(&summary);

    assert!(overlay.contains("Lifecycle"));
    assert!(overlay.contains("Birth/death events visible"));
    assert!(overlay.contains("population cap enforced"));
    assert!(overlay.contains("birth assets initialize only"));
    assert!(overlay.contains("lifetime state not inherited"));
    assert!(overlay.contains("Stable IDs only"));
    assert!(overlay.contains("lineage visuals cannot emit actions"));
    assert!(overlay.contains("gen"));
    assert!(!overlay.contains("Entity("));
    assert!(!overlay.contains("full action-authoritative"));
}

#[test]
fn live_brain_tick_smoke_seals_patch_and_updates_runtime_state() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_live_brain_loop_smoke(&launch).unwrap();

    assert_eq!(summary.schema, alife_game_app::G03_LIVE_BRAIN_LOOP_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION
    );
    assert_eq!(summary.organism_id, OrganismId(1));
    assert_eq!(summary.status, BrainTickStatus::Normal);
    assert_eq!(summary.selected_action_kind, Some(ActionKind::Interact));
    assert_eq!(summary.selected_action_id, Some(HeadlessActionIds::EAT));
    assert_eq!(summary.target_entity, Some(WorldEntityId(2)));
    assert!(summary.patch_sealed);
    assert_eq!(summary.patch_success, Some(true));
    assert_eq!(summary.sealed_patch_count, 1);
    assert_eq!(summary.packed_record_count, 1);
    assert_eq!(summary.memory_updates, 1);
    assert_eq!(summary.topology_updates, 1);
    assert_eq!(summary.tick_after.raw(), summary.tick_before.raw() + 1);
    assert_eq!(
        summary.world_tick_after.raw(),
        summary.world_tick_before.raw() + 1
    );
    assert_eq!(
        summary.causal_stages,
        vec![
            alife_game_app::LiveBrainCausalStage::GatherSensory,
            alife_game_app::LiveBrainCausalStage::CpuBrainTick,
            alife_game_app::LiveBrainCausalStage::ExecuteAction,
            alife_game_app::LiveBrainCausalStage::MeasureOutcome,
            alife_game_app::LiveBrainCausalStage::SealPatch,
            alife_game_app::LiveBrainCausalStage::UpdateLogs,
        ]
    );
}

#[test]
fn full_gpu_runtime_smoke_preserves_cpu_fallback_and_no_bulk_readback() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let default_summary =
        run_full_gpu_runtime_smoke(&launch, FullGpuRuntimeSmokeOptions::default()).unwrap();
    assert_eq!(default_summary.requested_mode, "cpu-reference");
    assert!(!default_summary.combined_mode);
    assert_eq!(default_summary.selected_backend, "CpuReference");
    assert!(!default_summary.gpu_static_dispatched);
    assert!(!default_summary.gpu_output_used_for_proposals);
    assert_eq!(default_summary.product_runtime_claim, "None");
    default_summary.validate().unwrap();

    let summary = run_full_gpu_runtime_smoke(
        &launch,
        FullGpuRuntimeSmokeOptions {
            mode: FullGpuRuntimeSmokeMode::StaticActionAuthoritative,
            ticks: 1,
            json_path: None,
        },
    )
    .unwrap();

    assert_eq!(
        summary.schema,
        alife_game_app::FULL_GPU_NEURAL_RUNTIME_SCHEMA
    );
    assert_eq!(
        summary.schema_version,
        alife_game_app::FULL_GPU_NEURAL_RUNTIME_SCHEMA_VERSION
    );
    assert_eq!(summary.ticks_run, 1);
    assert_eq!(summary.sealed_patches, 1);
    assert_eq!(summary.packed_logs, 1);
    assert!(summary.bulk_readback_forbidden);
    assert!(summary.per_synapse_readback_forbidden);
    assert!(summary.per_lobe_readback_forbidden);
    assert!(summary.weight_readback_forbidden);
    if summary.gpu_static_dispatched {
        assert_eq!(summary.compact_readback_bytes, 64);
        if summary.cpu_shadow_parity {
            assert!(summary.gpu_output_used_for_proposals);
            assert_eq!(summary.product_runtime_claim, "CpuShadowGuarded");
        } else {
            assert_eq!(summary.selected_backend, "CpuReference");
            assert!(summary.fallback_reason.is_some());
            assert!(!summary.gpu_output_used_for_proposals);
            assert_eq!(summary.product_runtime_claim, "None");
        }
    } else {
        assert_eq!(summary.selected_backend, "CpuReference");
        assert!(summary.fallback_reason.is_some());
        assert!(!summary.gpu_output_used_for_proposals);
    }
    summary.validate().unwrap();
}

#[test]
fn full_gpu_runtime_plasticity_report_is_post_seal_shadow_only_when_available() {
    let _guard = gpu_plasticity_env_lock();
    std::env::remove_var("ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE");
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_full_gpu_runtime_smoke(
        &launch,
        FullGpuRuntimeSmokeOptions {
            mode: FullGpuRuntimeSmokeMode::StaticPlasticShadow,
            ticks: 1,
            json_path: None,
        },
    )
    .unwrap();

    assert_eq!(summary.sealed_patches, 1);
    assert!(!summary.combined_mode);
    assert!(summary.w_genetic_fixed_unchanged);
    assert!(summary.lifetime_consolidated_unchanged);
    assert!(summary.h_operational_unchanged);
    assert!(summary.plasticity_post_seal_only);
    if summary.plasticity_dispatched {
        assert!(summary.experience_patch_sealed_before_plasticity);
        assert!(summary.post_seal_diagnostic_readback_bytes > 0);
        assert!(summary.post_seal_diagnostic_readback_boundary_scoped);
        assert!(summary.h_shadow_updated_values > 0);
        assert!(summary.plasticity_live_core_update_applied);
        assert!(summary.post_seal_hshadow_applied);
        assert!(summary.post_seal_replay_protected);
        assert!(summary.post_seal_delta_applied_records > 0);
        assert_eq!(summary.post_seal_delta_sequence_id, Some(1));
        assert!(!summary.gpu_output_used_for_proposals);
        assert_eq!(summary.product_runtime_claim, "ShadowOnly");
        assert!(summary.plasticity_live_gap.contains("alife_core contract"));
    } else {
        assert!(summary.plasticity_live_gap.contains("CPU reference"));
    }
    summary.validate().unwrap();
}

#[test]
fn full_gpu_runtime_combined_static_plastic_mode_is_cpu_shadow_guarded() {
    let _guard = gpu_plasticity_env_lock();
    std::env::remove_var("ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE");
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_full_gpu_runtime_smoke(
        &launch,
        FullGpuRuntimeSmokeOptions {
            mode: FullGpuRuntimeSmokeMode::StaticPlasticCpuShadowGuarded,
            ticks: 3,
            json_path: None,
        },
    )
    .unwrap();

    assert_eq!(summary.requested_mode, "static-plastic-cpu-shadow-guarded");
    assert!(summary.combined_mode);
    assert_eq!(summary.ticks_run, 3);
    assert_eq!(summary.sealed_patches, 3);
    assert!(summary.bulk_readback_forbidden);
    assert!(summary.per_synapse_readback_forbidden);
    assert!(summary.per_lobe_readback_forbidden);
    assert!(summary.weight_readback_forbidden);
    assert!(summary.w_genetic_fixed_unchanged);
    assert!(summary.lifetime_consolidated_unchanged);
    assert!(summary.h_operational_unchanged);
    assert!(summary.unsupported_full_runtime_gap_remaining);

    if summary.gpu_static_dispatched {
        assert_eq!(summary.compact_readback_bytes, 64);
        assert!(summary.cpu_shadow_parity);
        assert!(summary.gpu_output_used_for_proposals);
        assert!(summary.plasticity_dispatched);
        assert!(summary.plasticity_post_seal_only);
        assert!(summary.post_seal_diagnostic_readback_bytes > 0);
        assert!(summary.post_seal_diagnostic_readback_ms >= 0.0);
        assert!(summary.post_seal_diagnostic_readback_boundary_scoped);
        assert!(summary.experience_patch_sealed_before_plasticity);
        assert!(summary.plasticity_live_core_update_applied);
        assert!(summary.post_seal_hshadow_applied);
        assert!(summary.post_seal_replay_protected);
        assert!(summary.post_seal_delta_applied_records > 0);
        assert_eq!(summary.post_seal_delta_sequence_id, Some(1));
        assert_eq!(
            summary.product_runtime_claim,
            "CpuShadowGuardedStaticPlusLiveHShadow"
        );
        assert!(summary.plasticity_live_gap.contains("unsupported"));
    } else {
        assert_eq!(summary.selected_backend, "CpuReference");
        assert!(summary.fallback_reason.is_some());
        assert!(!summary.gpu_output_used_for_proposals);
        assert!(!summary.plasticity_live_core_update_applied);
        assert_eq!(summary.product_runtime_claim, "None");
    }
    summary.validate().unwrap();
}

#[test]
fn full_gpu_runtime_combined_mode_degrades_when_post_seal_plasticity_unavailable() {
    let _guard = gpu_plasticity_env_lock();
    std::env::set_var("ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE", "0");
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_full_gpu_runtime_smoke(
        &launch,
        FullGpuRuntimeSmokeOptions {
            mode: FullGpuRuntimeSmokeMode::StaticPlasticCpuShadowGuarded,
            ticks: 3,
            json_path: None,
        },
    )
    .unwrap();
    std::env::remove_var("ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE");

    assert!(summary.combined_mode);
    assert_eq!(summary.sealed_patches, 3);
    if summary.gpu_static_dispatched {
        assert!(summary.gpu_output_used_for_proposals);
        assert_eq!(summary.product_runtime_claim, "CpuShadowGuarded");
        assert!(!summary.plasticity_dispatched);
        assert!(!summary.plasticity_live_core_update_applied);
        assert!(!summary.post_seal_hshadow_applied);
        assert_eq!(summary.post_seal_diagnostic_readback_bytes, 0);
        assert!(summary.post_seal_diagnostic_readback_boundary_scoped);
        assert!(summary
            .plasticity_live_gap
            .contains("post-seal GPU plasticity unavailable"));
    }
    summary.validate().unwrap();
}

#[test]
fn full_gpu_runtime_combined_summary_validation_rejects_overclaimed_hshadow() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let mut summary =
        run_full_gpu_runtime_smoke(&launch, FullGpuRuntimeSmokeOptions::default()).unwrap();
    summary.combined_mode = true;
    summary.gpu_static_dispatched = true;
    summary.fallback_reason = None;
    summary.gpu_output_used_for_proposals = true;
    summary.cpu_shadow_parity = true;
    summary.product_runtime_claim = "CpuShadowGuardedStaticPlusLiveHShadow".to_string();
    summary.post_seal_hshadow_applied = false;
    summary.plasticity_live_core_update_applied = false;
    summary.post_seal_delta_applied_records = 0;

    assert!(summary.validate().is_err());
}

#[test]
fn gpu_longrun_soak_config_is_manual_bounded_and_keeps_smoke_cap() {
    let mut options = GpuLongrunSoakOptions::default();
    options.validate().unwrap();
    assert_eq!(
        options.ticks,
        alife_game_app::GPU_LONGRUN_SOAK_DEFAULT_TICKS
    );
    assert_eq!(
        alife_game_app::FULL_GPU_NEURAL_RUNTIME_MAX_TICKS,
        16,
        "normal full-gpu-runtime-smoke must stay CI-capped"
    );

    options.ticks = 0;
    assert!(options.validate().is_err());
    options.ticks = alife_game_app::GPU_LONGRUN_SOAK_MAX_TICKS_MANUAL + 1;
    assert!(options.validate().is_err());
    options.ticks = 1;
    options.report_every = 0;
    assert!(options.validate().is_err());
}

#[test]
fn gpu_longrun_soak_short_run_reports_no_overclaim() {
    let _guard = gpu_plasticity_env_lock();
    std::env::remove_var("ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE");
    std::env::remove_var("ALIFE_GPU_RUNTIME_AVAILABLE");
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_gpu_longrun_soak(
        &launch,
        GpuLongrunSoakOptions {
            ticks: 3,
            report_every: 1,
            stop_on_first_parity_failure: true,
            stop_on_first_hshadow_rejection: true,
            json_path: None,
        },
    )
    .unwrap();

    assert_eq!(summary.requested_ticks, 3);
    assert_eq!(summary.ticks_completed, 3);
    assert_eq!(summary.sealed_patches, 3);
    assert!(summary.no_active_bulk_readback);
    assert!(!summary.full_action_authoritative_claim);
    assert!(summary.w_genetic_fixed_unchanged);
    assert!(summary.lifetime_consolidated_unchanged);
    assert!(summary.h_operational_unchanged);
    if summary.selected_backend != "CpuReference" {
        assert_eq!(summary.cpu_shadow_parity_checks, 3);
        assert_eq!(summary.parity_failures, 0);
        assert_eq!(summary.gpu_proposal_ticks, 3);
        assert!(summary.compact_active_readback_bytes >= 64 * 3);
        assert_eq!(
            summary.product_runtime_claim,
            "CpuShadowGuardedStaticPlusLiveHShadow"
        );
        assert!(summary.h_shadow_applications > 0);
        assert!(summary.total_h_shadow_records_applied > 0);
        assert!(summary.post_seal_readback_bytes > 0);
    } else {
        assert_eq!(summary.product_runtime_claim, "None");
        assert_eq!(summary.gpu_proposal_ticks, 0);
        assert_eq!(summary.h_shadow_applications, 0);
    }
    summary.validate().unwrap();
}

#[test]
fn gpu_longrun_soak_forced_fallback_does_not_claim_gpu_work() {
    let _guard = gpu_plasticity_env_lock();
    std::env::set_var("ALIFE_GPU_RUNTIME_AVAILABLE", "0");
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_gpu_longrun_soak(
        &launch,
        GpuLongrunSoakOptions {
            ticks: 3,
            report_every: 1,
            stop_on_first_parity_failure: true,
            stop_on_first_hshadow_rejection: true,
            json_path: None,
        },
    )
    .unwrap();
    std::env::remove_var("ALIFE_GPU_RUNTIME_AVAILABLE");

    assert_eq!(summary.selected_backend, "CpuReference");
    assert!(summary.fallback_reason.is_some());
    assert_eq!(summary.sealed_patches, 3);
    assert_eq!(summary.gpu_static_dispatched_ticks, 0);
    assert_eq!(summary.gpu_proposal_ticks, 0);
    assert_eq!(summary.h_shadow_applications, 0);
    assert_eq!(summary.compact_active_readback_bytes, 0);
    assert_eq!(summary.post_seal_readback_bytes, 0);
    assert_eq!(summary.product_runtime_claim, "None");
    assert!(!summary.full_action_authoritative_claim);
    summary.validate().unwrap();
}

#[test]
fn gpu_longrun_soak_validation_rejects_full_action_authoritative_claim() {
    let _guard = gpu_plasticity_env_lock();
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let mut summary = run_gpu_longrun_soak(
        &launch,
        GpuLongrunSoakOptions {
            ticks: 1,
            report_every: 1,
            stop_on_first_parity_failure: true,
            stop_on_first_hshadow_rejection: true,
            json_path: None,
        },
    )
    .unwrap();
    summary.full_action_authoritative_claim = true;
    assert!(summary.validate().is_err());
}

#[test]
fn gpu_sustained_learning_soak_config_is_manual_bounded() {
    let mut options = GpuSustainedLearningSoakOptions::default();
    options.validate().unwrap();
    assert_eq!(
        options.ticks,
        alife_game_app::GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_TICKS
    );
    assert_eq!(
        options.episode_ticks,
        alife_game_app::GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_EPISODE_TICKS
    );

    options.ticks = 0;
    assert!(options.validate().is_err());
    options.ticks = alife_game_app::GPU_SUSTAINED_LEARNING_SOAK_MAX_TICKS_MANUAL + 1;
    assert!(options.validate().is_err());
    options.ticks = 1;
    options.report_every = 0;
    assert!(options.validate().is_err());
    options.report_every = 1;
    options.episode_ticks = 0;
    assert!(options.validate().is_err());
    options.episode_ticks = alife_game_app::GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_EPISODE_TICKS + 1;
    assert!(options.validate().is_err());
}

#[test]
fn gpu_sustained_learning_soak_rotates_episodes_for_aggregate_patches() {
    let _guard = gpu_plasticity_env_lock();
    std::env::remove_var("ALIFE_GPU_PLASTICITY_DIAGNOSTIC_AVAILABLE");
    std::env::remove_var("ALIFE_GPU_RUNTIME_AVAILABLE");
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_gpu_sustained_learning_soak(
        &launch,
        GpuSustainedLearningSoakOptions {
            ticks: 40,
            report_every: 10,
            episode_ticks: 20,
            stop_on_first_parity_failure: true,
            stop_on_first_hshadow_rejection: true,
            json_path: None,
        },
    )
    .unwrap();

    assert_eq!(summary.requested_ticks, 40);
    assert_eq!(summary.ticks_completed, 40);
    assert_eq!(summary.episodes, 2);
    assert!(
        summary.sealed_patches_total > 32,
        "episode rotation must collect evidence beyond the old single-fixture cap"
    );
    assert_eq!(summary.sealed_patches_total, summary.packed_logs_total);
    assert!(summary.replay_protection_active);
    assert!(summary.repeated_learning_uses_episode_rotation);
    assert!(summary.no_active_bulk_readback);
    assert!(!summary.full_action_authoritative_claim);
    assert!(summary.w_genetic_fixed_unchanged);
    assert!(summary.lifetime_consolidated_unchanged);
    assert!(summary.h_operational_unchanged);
    if summary.selected_backend != "CpuReference" {
        assert_eq!(summary.cpu_shadow_parity_checks, 40);
        assert_eq!(summary.parity_failures, 0);
        assert_eq!(summary.gpu_proposal_ticks, 40);
        assert!(summary.h_shadow_application_attempts >= 2);
        assert!(summary.h_shadow_applications_succeeded >= 2);
        assert!(summary.total_h_shadow_records_applied >= 4);
        assert_eq!(
            summary.product_runtime_claim,
            "CpuShadowGuardedStaticPlusLiveHShadow"
        );
    } else {
        assert_eq!(summary.product_runtime_claim, "None");
        assert_eq!(summary.gpu_proposal_ticks, 0);
        assert_eq!(summary.h_shadow_applications_succeeded, 0);
    }
    summary.validate().unwrap();
}

#[test]
fn gpu_sustained_learning_soak_forced_fallback_keeps_cpu_honest() {
    let _guard = gpu_plasticity_env_lock();
    std::env::set_var("ALIFE_GPU_RUNTIME_AVAILABLE", "0");
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_gpu_sustained_learning_soak(
        &launch,
        GpuSustainedLearningSoakOptions {
            ticks: 40,
            report_every: 10,
            episode_ticks: 20,
            stop_on_first_parity_failure: true,
            stop_on_first_hshadow_rejection: true,
            json_path: None,
        },
    )
    .unwrap();
    std::env::remove_var("ALIFE_GPU_RUNTIME_AVAILABLE");

    assert_eq!(summary.selected_backend, "CpuReference");
    assert!(summary.fallback_reason.is_some());
    assert_eq!(summary.ticks_completed, 40);
    assert_eq!(summary.episodes, 2);
    assert!(summary.sealed_patches_total > 32);
    assert_eq!(summary.gpu_static_dispatched_ticks, 0);
    assert_eq!(summary.gpu_proposal_ticks, 0);
    assert_eq!(summary.h_shadow_application_attempts, 0);
    assert_eq!(summary.h_shadow_applications_succeeded, 0);
    assert_eq!(summary.product_runtime_claim, "None");
    assert!(!summary.full_action_authoritative_claim);
    summary.validate().unwrap();
}

#[test]
fn gpu_sustained_learning_soak_validation_rejects_overclaim() {
    let _guard = gpu_plasticity_env_lock();
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let mut summary = run_gpu_sustained_learning_soak(
        &launch,
        GpuSustainedLearningSoakOptions {
            ticks: 1,
            report_every: 1,
            episode_ticks: 1,
            stop_on_first_parity_failure: true,
            stop_on_first_hshadow_rejection: true,
            json_path: None,
        },
    )
    .unwrap();
    summary.full_action_authoritative_claim = true;
    assert!(summary.validate().is_err());

    summary.full_action_authoritative_claim = false;
    summary.selected_backend = "CpuReference".to_string();
    summary.product_runtime_claim = "CpuShadowGuardedStaticPlusLiveHShadow".to_string();
    summary.gpu_proposal_ticks = 1;
    assert!(summary.validate().is_err());
}

#[test]
fn ca36_soak_isolation_protocol_records_manual_untracked_artifacts() {
    let summary = run_multi_hour_soak_isolation_smoke().unwrap();

    assert_eq!(summary.schema, CA36_SOAK_ISOLATION_SCHEMA);
    assert_eq!(summary.schema_version, CA36_SOAK_ISOLATION_SCHEMA_VERSION);
    assert!(summary.artifact_root.starts_with("target/"));
    assert!(summary.default_report_path.starts_with("target/"));
    assert!(summary.report_artifacts_untracked);
    assert_eq!(summary.manual_10k_commands.len(), 3);
    assert!(summary
        .manual_10k_commands
        .iter()
        .all(|command| command.min_ticks >= Some(CA36_MIN_MANUAL_TICKS)));
    assert!(summary
        .manual_10k_commands
        .iter()
        .any(|command| command.command.contains("gpu-sustained-learning-soak")));
    assert!(summary
        .manual_10k_commands
        .iter()
        .any(|command| command.command.contains("gpu-longrun-soak")));
    assert!(summary
        .manual_10k_commands
        .iter()
        .any(|command| command.command.contains("ca22_manual_10k_ecological_soak")));
    assert!(summary.report_markdown.contains("Get-Process"));
    assert!(summary.report_markdown.contains("WorkingSet64"));
    assert!(summary
        .report_markdown
        .contains("target/ca36_soak_isolation"));
    summary.validate().unwrap();
}

#[test]
fn ca36_soak_isolation_protocol_preserves_gpu_truth_boundaries() {
    let summary = run_multi_hour_soak_isolation_smoke().unwrap();

    assert!(summary.cpu_fallback_preserved);
    assert!(summary.cpu_shadow_parity_preserved);
    assert!(summary.no_active_bulk_readback);
    assert!(!summary.full_action_authoritative_claim);
    assert!(!summary.release_tag_created);
    assert!(summary
        .report_markdown
        .contains("not full action-authoritative"));
    assert!(summary
        .precision_drift_counters
        .iter()
        .any(|counter| counter.name == "cpu_shadow_parity_checks"));
    assert!(summary
        .precision_drift_counters
        .iter()
        .any(|counter| counter.name == "h_shadow_delta_max"));
    assert!(summary
        .precision_drift_counters
        .iter()
        .any(|counter| counter.name == "working_set_private_memory"));
}

#[test]
fn full_gpu_runtime_unsupported_full_mode_falls_back_without_claiming_gpu_work() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_full_gpu_runtime_smoke(
        &launch,
        FullGpuRuntimeSmokeOptions {
            mode: FullGpuRuntimeSmokeMode::FullActionAuthoritative,
            ticks: 1,
            json_path: None,
        },
    )
    .unwrap();

    assert_eq!(summary.selected_backend, "CpuReference");
    assert!(summary.fallback_reason.is_some());
    assert!(!summary.gpu_static_dispatched);
    assert!(!summary.gpu_output_used_for_proposals);
    assert!(!summary.plasticity_dispatched);
    assert_eq!(summary.product_runtime_claim, "None");
    summary.validate().unwrap();
}

#[test]
fn app_bridge_and_manual_headless_tick_produce_compatible_patch_summary() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let mut bridge = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let proposals = bridge.current_context_proposals().unwrap();
    let bridge_summary = bridge.tick_with_proposals(proposals.clone());

    let save = PortableSaveFile::from_json_file(&launch.save_path).unwrap();
    let creature = save.creatures.first().unwrap();
    let mut mind = CreatureMind::scaffold(
        creature.organism_id,
        creature.brain_class,
        save.deterministic_seed,
        creature.mind.tick,
    )
    .unwrap();
    *mind.homeostasis_mut() = creature.mind.homeostasis;
    let mut harness = HeadlessBrainHarness::new(save.restore_headless_world().unwrap());
    let manual = harness.tick_mind(
        &mut mind,
        BrainTickInput::new(creature.mind.tick, proposals)
            .with_pack_experience(true)
            .with_action_duration(DurationTicks::new(1)),
    );

    let manual_patch = manual.brain.experience_patch.as_ref().unwrap();
    assert_eq!(bridge_summary.status, manual.brain.status);
    assert_eq!(
        bridge_summary.selected_action_kind,
        Some(manual_patch.decision().selected_action.kind)
    );
    assert_eq!(
        bridge_summary.selected_action_id,
        Some(manual_patch.decision().selected_action.action_id)
    );
    assert_eq!(
        bridge_summary.patch_success,
        Some(manual_patch.outcome().success)
    );
    assert_eq!(
        bridge_summary.physical_contact,
        Some(manual_patch.outcome().physical.contact)
    );
}

#[test]
fn invalid_live_action_is_recoverable_and_still_seals_failure_patch() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let mut bridge = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let invalid = ActionProposal::new(
        HeadlessActionIds::EAT,
        ActionKind::Interact,
        0.99,
        Confidence::new(0.99).unwrap(),
        None,
        0b11,
        ActionTarget::new(Some(WorldEntityId(99_999)), None),
        NormalizedScalar::new(0.9).unwrap(),
    )
    .unwrap();
    let summary = bridge.tick_with_proposals(vec![invalid]);

    assert_eq!(summary.status, BrainTickStatus::RecoverableActionFailure);
    assert!(summary.patch_sealed);
    assert_eq!(summary.patch_success, Some(false));
    assert!(summary.action_failure.is_some());
    assert_eq!(summary.sealed_patch_count, 1);
    assert_eq!(summary.memory_updates, 1);
    assert_eq!(summary.topology_updates, 1);
}

#[test]
fn pause_and_step_modes_do_not_advance_hidden_state_unexpectedly() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let (mind_tick, world_tick, produced) = run_live_brain_loop_paused_smoke(&launch).unwrap();
    assert_eq!(mind_tick, Tick::new(3));
    assert_eq!(world_tick, Tick::new(2));
    assert_eq!(produced, 0);

    let mut bridge = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let paused = bridge.update(LiveBrainTickControl::paused()).unwrap();
    assert!(paused.is_empty());
    assert_eq!(bridge.mind().current_tick(), Tick::new(3));
    let stepped = bridge.update(LiveBrainTickControl::step_once()).unwrap();
    assert_eq!(stepped.len(), 1);
    assert_eq!(bridge.mind().current_tick(), Tick::new(4));
}

#[test]
fn s02_runtime_controls_pause_step_and_run_through_sealed_live_loop() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_runtime_controls_smoke(&launch, 5).unwrap();

    assert_eq!(summary.paused_produced, 0);
    assert_eq!(summary.step_produced, 1);
    assert_eq!(summary.run_produced, 5);
    assert!(summary.all_patches_sealed);
    assert_eq!(summary.panel.playback, RuntimePlaybackState::Running);
    assert_eq!(summary.panel.run_speed_ticks, 2);
    assert!(summary.panel.selected_action_kind.is_some());
    assert!(summary.panel.sealed_patch_count >= 6);
    assert!(summary.panel.packed_record_count >= 6);
    assert!(summary.panel.status_overlay_text().contains("Controls:"));
    assert!(summary
        .panel
        .status_overlay_text()
        .contains("A-Life GPU Alpha Playground"));
    summary.validate().unwrap();
}

#[test]
fn s02_runtime_controls_cannot_mutate_cognition_directly() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);

    let stepped = panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    assert_eq!(stepped.len(), 1);
    assert!(stepped[0].patch_sealed);
    panel.direct_cognition_mutation_allowed = true;
    assert!(panel.validate().is_err());
}

#[test]
fn ca13_double_buffered_scheduler_uses_fixed_cadence_not_render_frames() {
    let mut scheduler = DoubleBufferedGraphicalScheduler::default();

    let paused = scheduler
        .observe_render_frame(1.0, RuntimePlaybackState::Paused, 1)
        .unwrap();
    assert_eq!(paused.ticks_to_run, 0);
    assert_eq!(scheduler.fixed_tick_index, 0);
    assert_eq!(scheduler.paused_frames, 1);

    let sub_tick = scheduler
        .observe_render_frame(0.016, RuntimePlaybackState::Running, 1)
        .unwrap();
    assert_eq!(sub_tick.ticks_to_run, 0);
    assert!(scheduler.render_alpha_milli > 0);

    let fixed_tick = scheduler
        .observe_render_frame(0.034, RuntimePlaybackState::Running, 1)
        .unwrap();
    assert_eq!(fixed_tick.ticks_to_run, 1);
    scheduler
        .record_executed_ticks(fixed_tick.ticks_to_run)
        .unwrap();
    assert_eq!(scheduler.fixed_tick_index, 1);
    assert_eq!(scheduler.front_buffer, Ca13TickBuffer::B);

    let catch_up = scheduler
        .observe_render_frame(1.0, RuntimePlaybackState::Running, 1)
        .unwrap();
    assert_eq!(
        catch_up.ticks_to_run,
        alife_game_app::CA13_MAX_CATCH_UP_TICKS_PER_FRAME
    );
    assert!(catch_up.catch_up_capped);
    assert!(scheduler.catch_up_ticks_dropped > 0);
    assert!(scheduler.overlay_line().contains("fixed=20Hz"));
    assert!(!scheduler.overlay_line().contains("Entity("));
}

#[test]
fn ca13_scheduler_smoke_proves_pause_step_and_catchup_bounds() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_double_buffered_scheduler_smoke(&launch).unwrap();

    assert_eq!(summary.paused_ticks, 0);
    assert_eq!(summary.sub_tick_due, 0);
    assert_eq!(summary.fixed_tick_due, 1);
    assert_eq!(summary.step_ticks, 1);
    assert!(summary.frame_driven_drift_prevented);
    assert!(summary
        .scheduler
        .signature_line()
        .contains("alife.ca13.double_buffered_graphical_scheduler.v1"));
    summary.validate().unwrap();
}

#[test]
fn ca14_motor_ring_arbitration_smoke_preserves_p09_boundary() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_motor_ring_arbitration_smoke(&launch).unwrap();

    assert!(summary.patch_sealed);
    assert!(!summary.direct_action_bypass);
    assert_eq!(
        summary.ring.schema,
        alife_game_app::CA14_MOTOR_RING_PRESENTATION_SCHEMA
    );
    assert_eq!(
        summary.ring.channels.len(),
        alife_game_app::CA14_MAX_MOTOR_RING_CHANNELS
    );
    assert_eq!(
        summary
            .ring
            .channels
            .iter()
            .filter(|channel| channel.selected)
            .count(),
        1
    );
    assert!(summary.ring.panel_text().contains("Motor Ring"));
    assert!(summary.ring.panel_text().contains("normal arbitration"));
    assert!(summary.ring.panel_text().contains("no direct bypass"));
    assert!(!summary.ring.panel_text().contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn ca14_runtime_panel_records_motor_ring_without_action_bypass() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let summaries = panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();

    assert_eq!(summaries.len(), 1);
    assert!(panel.motor_ring.selected_action_id.is_some());
    assert!(panel.motor_ring.structured_arbitration_preserved);
    assert!(panel.motor_ring.no_direct_action_bypass);
    assert!(panel.status_overlay_text().contains("Motor Ring: winner="));
    assert!(panel
        .signature_line()
        .contains(alife_game_app::CA14_MOTOR_RING_PRESENTATION_SCHEMA));
    assert!(!panel.status_overlay_text().contains("Entity("));
    panel.validate().unwrap();
}

#[test]
fn ca15_homeostasis_runtime_smoke_exposes_bounded_registers_and_modulation() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_homeostasis_runtime_smoke(&launch).unwrap();

    assert_eq!(
        summary.after.schema,
        alife_game_app::CA15_HOMEOSTASIS_RUNTIME_SCHEMA
    );
    assert_eq!(
        summary.after.registers.len(),
        alife_game_app::CA15_HOMEOSTASIS_REGISTER_COUNT
    );
    assert!(summary.patch_sealed);
    assert!(summary.finite_and_bounded);
    assert!(summary.fixed_register_count);
    assert!(summary.salience_learning_visible);
    assert!(summary.after.salience_modulation > 0.0);
    assert!(summary.after.learning_modulation > 0.0);
    assert!(summary.after.panel_text().contains("Homeostasis"));
    assert!(summary.after.panel_text().contains("Energy"));
    assert!(summary.after.panel_text().contains("Hunger"));
    assert!(summary.after.panel_text().contains("Fatigue"));
    assert!(summary.after.panel_text().contains("Pain"));
    assert!(summary.after.panel_text().contains("Stress"));
    assert!(summary.after.panel_text().contains("sal="));
    assert!(summary.after.panel_text().contains("learn="));
    assert!(!summary.after.panel_text().contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn ca15_runtime_panel_updates_homeostasis_after_live_step() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let before_tick = panel.homeostasis.tick;
    let summaries = panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();

    assert_eq!(summaries.len(), 1);
    assert!(panel.homeostasis.tick.raw() > before_tick.raw());
    assert!(panel.status_overlay_text().contains("Homeo E"));
    assert!(panel
        .structured_status_panel_text_with_backend("GPU: test")
        .contains("Mods: sal="));
    assert!(panel
        .signature_line()
        .contains(alife_game_app::CA15_HOMEOSTASIS_RUNTIME_SCHEMA));
    assert!(!panel.status_overlay_text().contains("Entity("));
    panel.validate().unwrap();
}

#[test]
fn ca16_affordance_loop_approaches_then_eats_without_scripted_forcing() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_affordance_loop_smoke(&launch).unwrap();

    assert_eq!(summary.schema, alife_game_app::CA16_AFFORDANCE_LOOP_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::CA16_AFFORDANCE_LOOP_SCHEMA_VERSION
    );
    assert_eq!(summary.food_entity, WorldEntityId(2));
    assert!(summary.moved_toward_food);
    assert!(summary.initial_food_distance > summary.after_approach_food_distance);
    assert_eq!(
        summary.approach_tick.selected_action_kind,
        Some(ActionKind::Move)
    );
    assert_eq!(
        summary.approach_tick.selected_action_id,
        Some(HeadlessActionIds::APPROACH)
    );
    assert_eq!(
        summary.eat_tick.selected_action_kind,
        Some(ActionKind::Interact)
    );
    assert_eq!(
        summary.eat_tick.selected_action_id,
        Some(HeadlessActionIds::EAT)
    );
    assert_eq!(
        summary.approach_tick.target_entity,
        Some(summary.food_entity)
    );
    assert_eq!(summary.eat_tick.target_entity, Some(summary.food_entity));
    assert_eq!(
        summary.eat_tick.physical_contact,
        Some(alife_core::PhysicalContactKind::Consumed)
    );
    assert!(summary.approach_tick.patch_sealed);
    assert!(summary.eat_tick.patch_sealed);
    assert!(summary.sealed_patches >= 2);
    assert!(summary.food_consumed);
    assert!(!summary.food_visible_after_eat);
    assert!(summary.hunger_after < summary.hunger_before);
    assert!(summary.energy_after > summary.energy_before);
    assert!(summary.normal_arbitration_preserved);
    assert!(summary.no_scripted_action_forcing);
    assert!(!summary.signature.contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn ca16_live_loop_uses_approach_when_food_is_outside_eat_radius() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let summaries = panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    let first = summaries
        .first()
        .expect("CA16 should produce one live tick");

    assert_eq!(first.selected_action_kind, Some(ActionKind::Move));
    assert_eq!(first.selected_action_id, Some(HeadlessActionIds::APPROACH));
    assert_eq!(first.target_entity, Some(WorldEntityId(2)));
    assert!(first.patch_sealed);
    assert_eq!(first.patch_success, Some(true));
    assert!(first.action_failure.is_none());
}

#[test]
fn ca17_hazard_recovery_smoke_covers_avoidance_pain_sleep_and_failure_recovery() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_hazard_recovery_smoke(&launch).unwrap();

    assert_eq!(summary.schema, alife_game_app::CA17_HAZARD_RECOVERY_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::CA17_HAZARD_RECOVERY_SCHEMA_VERSION
    );
    assert!(summary.fixture_hazard_visible);
    assert_eq!(summary.hazard_entity, WorldEntityId(2));
    assert!(summary.hazard_salience > 0.0);
    assert!(summary.visible_hazard_cue);
    assert!(summary.after_flee_hazard_distance > summary.initial_hazard_distance);
    assert_eq!(
        summary.flee_tick.selected_action_kind,
        Some(ActionKind::Move)
    );
    assert_eq!(
        summary.flee_tick.selected_action_id,
        Some(HeadlessActionIds::FLEE)
    );
    assert_eq!(summary.flee_tick.target_entity, Some(summary.hazard_entity));
    assert!(summary.flee_tick.patch_sealed);
    assert!(summary.flee_tick.action_failure.is_none());
    assert_eq!(
        summary.pain_tick.physical_contact,
        Some(alife_core::PhysicalContactKind::Collision)
    );
    assert!(summary.pain_after_contact > summary.pain_before);
    assert!(summary.fear_after_contact > summary.fear_before);
    assert_eq!(
        summary.sleep_tick.selected_action_kind,
        Some(ActionKind::Rest)
    );
    assert_ne!(summary.sleep_phase_after, alife_core::SleepPhase::Awake);
    assert!(summary.fatigue_after_sleep < summary.fatigue_before_sleep);
    assert!(summary.failure_tick.action_failure.is_some());
    assert!(summary.failure_tick.patch_sealed);
    assert!(summary.recovery_tick.patch_sealed);
    assert!(summary.failure_recovered_with_sealed_patch);
    assert!(summary.terminal_stagnation_avoided);
    assert!(summary.normal_arbitration_preserved);
    assert!(summary.no_scripted_terminal_escape);
    assert!(!summary.signature.contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn graphical_gpu_launch_config_defaults_gpu_first_and_preserves_cpu_choice() {
    let gpu_default =
        alife_game_app::GraphicalPlaygroundLaunchConfig::interactive(p34_fixture_root());
    assert_eq!(
        gpu_default.gpu_mode,
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded
    );
    let default_summary =
        alife_game_app::validate_graphical_playground_launch(&gpu_default).unwrap();
    assert!(default_summary.gpu_mode_visible);
    assert!(default_summary.cpu_fallback_visible);
    assert_eq!(
        default_summary.requested_gpu_mode,
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded
    );

    let cpu = alife_game_app::GraphicalPlaygroundLaunchConfig::interactive(p34_fixture_root())
        .with_gpu_mode(GraphicalGpuRuntimeMode::CpuReference);
    let cpu_summary = alife_game_app::validate_graphical_playground_launch(&cpu).unwrap();
    assert_eq!(
        cpu_summary.requested_gpu_mode,
        GraphicalGpuRuntimeMode::CpuReference
    );
    assert!(
        alife_game_app::GraphicalPlaygroundLaunchConfig::interactive(p34_fixture_root())
            .with_gpu_mode(GraphicalGpuRuntimeMode::CpuReference)
            .require_gpu(true)
            .validate()
            .is_err()
    );

    let gpu = alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(p34_fixture_root(), 5)
        .with_gpu_mode(GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded);
    let gpu_summary = alife_game_app::validate_graphical_playground_launch(&gpu).unwrap();
    assert_eq!(
        gpu_summary.requested_gpu_mode,
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded
    );
    assert!(gpu_summary.cpu_fallback_visible);
    assert!(!gpu_summary.require_gpu);
    assert!(gpu_summary
        .signature_line()
        .contains("gpu_mode=static-plastic"));
}

#[test]
fn ca42a_default_graphical_launch_uses_player_view_acceptance() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let summary = alife_game_app::validate_graphical_playground_launch(&launch).unwrap();
    let acceptance = &summary.player_view_acceptance;

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);
    assert!(!summary.stable_id_overlay_visible);
    assert!(acceptance.dev_overlay_hidden);
    assert!(acceptance.full_debug_hidden);
    assert!(acceptance.event_feed_collapsed);
    assert!(acceptance.stable_id_labels_hidden_except_selected);
    assert!(acceptance.internal_patch_gpu_claim_spam_hidden);
    assert!(acceptance.topology_lines_hidden);
    assert!(acceptance.teacher_debug_labels_hidden_unless_school);
    assert!(acceptance.terrain_overlay_max_opacity <= CA42A_MAX_PLAYER_TERRAIN_OVERLAY_ALPHA);
    assert!(summary.signature_line().contains("view_mode=player"));
    assert!(summary.signature_line().contains("dev_overlay_hidden=true"));
}

#[test]
fn ca42a_dev_overlay_and_full_debug_modes_remain_available() {
    let dev = alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5)
        .with_view_mode(GraphicalPlaygroundViewMode::DevOverlay);
    let dev_summary = alife_game_app::validate_graphical_playground_launch(&dev).unwrap();
    assert_eq!(
        dev_summary.view_mode,
        GraphicalPlaygroundViewMode::DevOverlay
    );
    assert!(dev_summary.stable_id_overlay_visible);
    assert!(!dev_summary.player_view_acceptance.dev_overlay_hidden);
    assert!(dev_summary.player_view_acceptance.full_debug_hidden);

    let full = alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5)
        .with_view_mode(GraphicalPlaygroundViewMode::FullDebug);
    let full_summary = alife_game_app::validate_graphical_playground_launch(&full).unwrap();
    assert_eq!(
        full_summary.view_mode,
        GraphicalPlaygroundViewMode::FullDebug
    );
    assert!(full_summary.stable_id_overlay_visible);
    assert!(!full_summary.player_view_acceptance.dev_overlay_hidden);
    assert!(!full_summary.player_view_acceptance.full_debug_hidden);

    assert!(GraphicalPlaygroundViewMode::parse("player").is_ok());
    assert!(GraphicalPlaygroundViewMode::parse("dev-overlay").is_ok());
    assert!(GraphicalPlaygroundViewMode::parse("full-debug").is_ok());
    assert!(GraphicalPlaygroundViewMode::parse("debug-dashboard").is_err());
}

#[test]
fn ca44a_committed_alpha_art_manifest_validates_required_roles_and_pngs() {
    let summary = alife_game_app::validate_alpha_art_manifest(
        alife_game_app::default_alpha_art_manifest_path(),
    )
    .unwrap();
    assert_eq!(
        summary.schema,
        alife_game_app::CA44A_ALPHA_ART_MANIFEST_SCHEMA
    );
    assert_eq!(
        summary.schema_version,
        alife_game_app::CA44A_ALPHA_ART_MANIFEST_SCHEMA_VERSION
    );
    assert!(summary.entry_count >= 32);
    assert!(summary.required_roles_present);
    assert!(summary.prop_variant_count >= 5);
    assert!(summary.largest_file_bytes <= alife_game_app::CA44A_MAX_ALPHA_ART_BACKDROP_BYTES);
    assert!(summary.png_dimensions_validated);
    assert!(summary.forbidden_artifact_paths_rejected);
    summary.validate().unwrap();

    let manifest_text =
        std::fs::read_to_string(alife_game_app::default_alpha_art_manifest_path()).unwrap();
    let manifest: alife_game_app::AlphaArtManifest = serde_json::from_str(&manifest_text).unwrap();
    let backdrop = manifest
        .entries
        .iter()
        .find(|entry| entry.role == "world-backdrop")
        .expect("world backdrop manifest entry");
    assert!(backdrop.file_size_bytes > alife_game_app::CA44A_MAX_ALPHA_ART_ASSET_BYTES);
    assert!(backdrop.file_size_bytes <= alife_game_app::CA44A_MAX_ALPHA_ART_BACKDROP_BYTES);
    assert!(backdrop.width <= alife_game_app::CA44A_MAX_PRODUCTION_BACKDROP_DIMENSION);
    assert!(backdrop.height <= alife_game_app::CA44A_MAX_PRODUCTION_BACKDROP_DIMENSION);
    for entry in manifest
        .entries
        .iter()
        .filter(|entry| entry.role != "world-backdrop")
    {
        assert!(
            entry.file_size_bytes <= alife_game_app::CA44A_MAX_ALPHA_ART_ASSET_BYTES,
            "{} exceeded ordinary alpha-art asset cap",
            entry.id
        );
        assert!(entry.width <= alife_game_app::CA44A_MAX_PRODUCTION_ART_DIMENSION);
        assert!(entry.height <= alife_game_app::CA44A_MAX_PRODUCTION_ART_DIMENSION);
    }
}

#[test]
fn ca44a_gpu_alpha_stability_regression_runs_past_tick_7() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = alife_game_app::run_ca44a_gpu_alpha_stability_smoke(&launch, 600).unwrap();
    assert_eq!(summary.requested_ticks, 600);
    assert_eq!(summary.completed_ticks, 600);
    assert!(summary.first_invalid_tick.is_none());
    assert_eq!(summary.terminal_invalid_count, 0);
    assert_eq!(summary.sealed_patches, 600);
    assert_eq!(summary.packed_records, 600);
    assert_eq!(summary.topology_simplexes, 600);
    assert!(summary.cpu_shadow_parity_preserved);
    summary.validate().unwrap();
}

#[cfg(feature = "bevy-app")]
#[test]
fn ca42a_player_hud_is_compact_and_debug_spam_free() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    let gpu = GraphicalGpuRuntimeTelemetry {
        requested_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        selected_backend: "GpuPlastic".to_string(),
        fallback_reason: None,
        hardware_identifier: Some("local-test".to_string()),
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow".to_string(),
        gpu_static_dispatched_ticks: 1,
        gpu_scores_used_for_proposals: true,
        cpu_shadow_parity: true,
        parity_failures: 0,
        sealed_patches: panel.sealed_patch_count,
        h_shadow_applications: 1,
        last_h_shadow_delta: 0.004,
        compact_readback_bytes: 64,
        post_seal_readback_bytes: 64,
        total_gpu_runtime_ms: 1.0,
        wgsl: test_wgsl_telemetry(),
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };

    let player_hud = alife_game_app::bevy_shell::graphical_player_status_overlay_text(&panel, &gpu);
    let debug_hud =
        alife_game_app::bevy_shell::graphical_full_debug_status_overlay_text(&panel, &gpu);

    assert!(player_hud.contains("A-Life GPU Alpha"));
    assert!(player_hud.contains("GPU ON"));
    assert!(alife_game_app::bevy_shell::ca42a_player_controls_bar_text().contains("Space"));
    assert!(alife_game_app::bevy_shell::ca42a_player_controls_bar_text().contains("N"));
    assert!(alife_game_app::bevy_shell::ca42a_player_controls_bar_text().contains("R"));
    assert!(alife_game_app::bevy_shell::ca42a_player_controls_bar_text().contains("Esc"));
    assert!(player_hud.lines().count() <= 4);
    assert!(!player_hud.contains("stable:"));
    assert!(!player_hud.contains("Patch:"));
    assert!(!player_hud.contains("sealed="));
    assert!(!player_hud.contains("CpuShadowGuardedStaticPlusLiveHShadow"));
    assert!(!player_hud.contains("full_auth"));
    assert!(debug_hud.contains("Creature: stable:1"));
    assert!(debug_hud.contains("Patch: sealed="));
    assert!(debug_hud.contains("Gate: CPU shadow; full_auth=false"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn ca44a_player_view_uses_alpha_art_sprites_not_default_rectangles() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch).unwrap();
    app.update();

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);
    assert!(!summary.stable_id_overlay_visible);

    let mut art_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalAlphaArtBackedSprite>();
    let roles = art_query
        .iter(app.world())
        .map(|sprite| sprite.role)
        .collect::<Vec<_>>();
    for role in [
        "creature-idle",
        "food",
        "hazard",
        "entity-shadow",
        "rock-obstacle",
        "selection-ring",
        "selection-pulse",
        "feedback-reward",
        "feedback-pain",
        "feedback-sleep",
        "feedback-learning",
        "prop-dressing",
        "terrain-safe-grass",
        "terrain-soil-path",
        "terrain-resource-grove",
        "terrain-hazard-pressure",
        "terrain-stone-rough",
        "terrain-water",
        "terrain-sand",
    ] {
        assert!(roles.contains(&role), "missing alpha art role {role}");
    }
    assert!(
        !roles.contains(&"world-painted-viewport"),
        "default Player View must use the live procedural biome map, not the baked painted viewport"
    );
    assert!(
        !roles.contains(&"world-atmospheric-underlay"),
        "default Player View must not restore the old atmospheric debug underlay"
    );

    let mut fallback_query =
        app.world_mut()
            .query::<&alife_game_app::bevy_shell::GraphicalAlphaArtFallbackSprite>();
    assert_eq!(
        fallback_query.iter(app.world()).count(),
        0,
        "default Player View must not use rectangle fallback sprites"
    );

    let mut badge_query = app.world_mut().query::<(
        &bevy::prelude::Visibility,
        &alife_game_app::bevy_shell::GraphicalObjectBadge,
    )>();
    assert!(badge_query
        .iter(app.world())
        .all(|(visibility, _)| *visibility == bevy::prelude::Visibility::Hidden));

    let mut pulse_query = app.world_mut().query_filtered::<
        &bevy::prelude::Sprite,
        bevy::prelude::With<alife_game_app::bevy_shell::GraphicalSensoryCuePulse>,
    >();
    for pulse in pulse_query.iter(app.world()) {
        let size = pulse
            .custom_size
            .expect("Player View feedback pulses should be bounded sprites");
        assert!(
            size.x <= 16.0 && size.y <= 16.0,
            "Player View feedback pulses must be bounded pings, not debug slabs: {size:?}"
        );
    }

    let mut selection_query = app.world_mut().query_filtered::<
        &bevy::prelude::Sprite,
        bevy::prelude::With<alife_game_app::bevy_shell::GraphicalSelectionRing>,
    >();
    for ring in selection_query.iter(app.world()) {
        let size = ring
            .custom_size
            .expect("Player View selection rings should be bounded sprites");
        assert!(
            size.x <= 92.0 && size.y <= 74.0,
            "selected creature rings must frame the sprite without covering the map: {size:?}"
        );
    }
}

#[cfg(feature = "bevy-app")]
#[test]
fn production_player_view_uses_runtime_map_and_readable_foreground_sprites() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch).unwrap();
    app.update();

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);

    let mut biome_map_query =
        app.world_mut()
            .query::<&alife_game_app::bevy_shell::GraphicalRuntimeProceduralBiomeMap>();
    let biome_maps = biome_map_query
        .iter(app.world())
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(biome_maps.len(), 1);
    assert!(
        biome_maps[0].primary_player_surface,
        "runtime-generated seeded biome map should be the continuous Player View surface; committed terrain tiles blend in as asset-backed detail"
    );
    assert!(
        biome_maps[0].generated_from_alpha_art_tiles,
        "runtime biome surface must stamp committed generated terrain PNGs, not only paint flat procedural colors"
    );
    assert_eq!(
        biome_maps[0].terrain_tile_source_count, 7,
        "runtime biome surface should use the complete grass/soil/grove/hazard/stone/water/sand tile set"
    );
    assert!(biome_maps[0].seed > 0);
    assert!(biome_maps[0].active_chunk_count > 0);
    assert!(
        biome_maps[0].fog_of_war_applied && biome_maps[0].fogged_pixels > 0,
        "runtime-generated biome map should apply fog outside active creature chunk windows"
    );

    let mut art_query = app.world_mut().query::<(
        &alife_game_app::bevy_shell::GraphicalAlphaArtBackedSprite,
        &bevy::prelude::Sprite,
    )>();
    let art_sprites = art_query.iter(app.world()).collect::<Vec<_>>();
    assert!(
        art_sprites
            .iter()
            .all(|(marker, _)| marker.role != "world-painted-viewport"),
        "Player View should not depend on the baked painted viewport"
    );

    let live_creature_max = art_sprites
        .iter()
        .filter(|(marker, _)| {
            matches!(
                marker.role,
                "creature-idle"
                    | "creature-hurt"
                    | "creature-moving"
                    | "creature-eat"
                    | "creature-sleep"
                    | "creature-signal"
            )
        })
        .filter_map(|(_, sprite)| sprite.custom_size)
        .map(|size| size.x.max(size.y))
        .fold(0.0_f32, f32::max);
    assert!(
        live_creature_max >= 34.0 && live_creature_max <= 64.0,
        "live creatures should be readable player-scale sprites without becoming giant foreground sprites: {live_creature_max}"
    );

    let required_world_role_max = art_sprites
        .iter()
        .filter(|(marker, _)| {
            matches!(
                marker.role,
                "food" | "hazard" | "rock-obstacle" | "prop-dressing"
            )
        })
        .filter_map(|(_, sprite)| sprite.custom_size)
        .map(|size| size.x.max(size.y))
        .fold(0.0_f32, f32::max);
    assert!(
        required_world_role_max >= 24.0 && required_world_role_max <= 52.0,
        "food, hazard, rock, and prop sprites must be readable but bounded player-scale sprites: {required_world_role_max}"
    );

    let generated_content_max = art_sprites
        .iter()
        .filter(|(marker, _)| {
            marker
                .stable_id
                .map(|stable_id| stable_id.raw() >= alife_world::PROCEDURAL_CONTENT_ID_BASE)
                .unwrap_or(false)
        })
        .filter_map(|(_, sprite)| sprite.custom_size)
        .map(|size| size.x.max(size.y))
        .fold(0.0_f32, f32::max);
    assert!(
        generated_content_max >= 12.0 && generated_content_max <= 28.0,
        "generated procedural content should be readable world dressing, not tiny specks or large overlays: {generated_content_max}"
    );
}

#[cfg(feature = "bevy-app")]
#[test]
fn production_world_art_atlas_v3_breaks_up_debug_checkerboard() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch).unwrap();
    app.update();

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);

    let field = app
        .world()
        .resource::<alife_game_app::bevy_shell::GraphicalProceduralTerrainFieldResource>()
        .clone();
    assert!(
        field.materialized_tiles.len() >= 17 * 11,
        "expected procedural terrain ledger evidence behind the rendered Player View chunk slice"
    );
    assert!(field.materialized_only_near_active_views);
    assert!(field.virtual_map_width_tiles >= 97);
    assert!(field.virtual_map_height_tiles >= 73);
    assert!(field.generated_without_rendering);
    assert!(field.creature_anchor_count >= 1);
    assert!(
        !field.active_world_chunks.is_empty(),
        "world chunks should be activated around stable-ID creature anchors before rendering"
    );
    assert!(
        field.materialized_tiles.len() >= 17 * 11,
        "at least one local viewport chunk should be generated"
    );
    let mut terrain_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalWorldArtTerrainTile>();
    let visible_terrain_count = terrain_query.iter(app.world()).count();
    assert!(
        visible_terrain_count >= 17 * 11,
        "default Player View must render an active procedural terrain chunk slice, not only a painted plate"
    );

    let mut layer_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalProductionArtLayer>();
    let layers = layer_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert!(
        layers.iter().all(|layer| layer.display_only),
        "world-art blend layers must not become simulation authority"
    );
    let baked_backdrop_count = layers
        .iter()
        .filter(|layer| {
            layer.role == "world-painted-viewport" || layer.role == "world-atmospheric-underlay"
        })
        .count();
    let streamed_terrain_count = layers
        .iter()
        .filter(|layer| layer.role == "streamed-procedural-terrain")
        .count();
    assert_eq!(
        baked_backdrop_count, 0,
        "Player View should use the live runtime procedural map, not a baked painted plate"
    );
    let runtime_biome_map_count = layers
        .iter()
        .filter(|layer| layer.role == "runtime-procedural-biome-map")
        .count();
    assert_eq!(
        runtime_biome_map_count, 1,
        "Player View should keep one live generated biome-map surface for fog/context support"
    );
    assert!(
        streamed_terrain_count >= visible_terrain_count,
        "streamed terrain chunks, not the underlay, must be the readable map layer"
    );

    let mut chunk_query =
        app.world_mut()
            .query::<&alife_game_app::bevy_shell::GraphicalProceduralTerrainChunkTile>();
    assert_eq!(
        chunk_query.iter(app.world()).count(),
        visible_terrain_count,
        "visible terrain entities should be the active chunk materialization, not unrelated debug blocks"
    );

    let mut zone_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalTerrainZoneMarker>();
    assert_eq!(
        zone_query.iter(app.world()).count(),
        0,
        "default Player View should not be dominated by CA19 debug zone blocks"
    );
}

#[cfg(feature = "bevy-app")]
#[test]
fn production_player_view_starts_with_rendered_procedural_chunk_window() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch).unwrap();
    app.update();

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);

    let mut biome_map_query =
        app.world_mut()
            .query::<&alife_game_app::bevy_shell::GraphicalRuntimeProceduralBiomeMap>();
    let biome_maps = biome_map_query
        .iter(app.world())
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(
        biome_maps.len(),
        1,
        "default Player View should render one generated biome-map underlay, not a blank ground plane"
    );
    let biome_map = biome_maps[0];
    assert!(biome_map.generated_from_procedural_sampler);
    assert!(biome_map.generated_from_alpha_art_tiles);
    assert_eq!(biome_map.terrain_tile_source_count, 7);
    assert!(biome_map.primary_player_surface);
    assert!(biome_map.display_only);
    assert_eq!(biome_map.seed, summary.seed);
    assert!(biome_map.active_chunk_count > 0);
    assert!(
        biome_map.fog_of_war_applied && biome_map.fogged_pixels > 0,
        "seeded Player View map should fog regions with no active creature/chunk presence"
    );
    assert!(biome_map.width_tiles >= 96);
    assert!(biome_map.height_tiles >= 64);
    assert_eq!(biome_map.pixels_per_tile, 20);
    assert_eq!(
        biome_map.texture_width_px,
        biome_map.width_tiles as u32 * biome_map.pixels_per_tile
    );
    assert_eq!(
        biome_map.texture_height_px,
        biome_map.height_tiles as u32 * biome_map.pixels_per_tile
    );
    assert!(biome_map.virtual_map_width_tiles > biome_map.width_tiles as usize);
    assert!(biome_map.virtual_map_height_tiles > biome_map.height_tiles as usize);
    assert!(
        biome_map.path_pixels > 10_000,
        "generated map should contain visible path networks like the target mockup"
    );
    assert!(
        biome_map.resource_detail_pixels > 500,
        "generated map should contain small resource/grove detail pixels"
    );
    assert!(
        biome_map.hazard_detail_pixels > 200,
        "generated map should contain hazard pressure detail pixels"
    );
    assert!(
        biome_map.stone_detail_pixels > 500,
        "generated map should contain stone/rough ground detail pixels"
    );
    assert_eq!(
        biome_map.dark_gap_pixels, 0,
        "generated map must not have black void gaps in the default player view"
    );

    let mut terrain_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalWorldArtTerrainTile>();
    let terrain_tiles = terrain_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert!(
        terrain_tiles.len() >= 29 * 21,
        "default Player View must show a dense active procedural chunk window"
    );
    assert!(
        terrain_tiles.iter().all(|tile| {
            tile.opacity >= 0.0 && tile.opacity <= 0.02
        }),
        "streamed terrain tiles should not read as square slabs; the generated alpha-art biome texture is the visible terrain surface"
    );
    assert!(
        terrain_tiles
            .iter()
            .all(|tile| tile.tile_size_pixels >= 34.0 && tile.tile_size_pixels <= 38.0),
        "terrain tiles should remain aligned map cells, not a single-screen plate"
    );
    assert!(terrain_tiles
        .iter()
        .any(|tile| tile.material_id == "resource-grove"));
    assert!(terrain_tiles
        .iter()
        .any(|tile| tile.material_id == "hazard-pressure"));
    assert!(terrain_tiles
        .iter()
        .any(|tile| tile.material_id == "stone-dressing"));
    assert!(terrain_tiles.iter().any(|tile| tile.material_id == "water"));
    assert!(terrain_tiles.iter().any(|tile| tile.material_id == "sand"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn player_view_streaming_keeps_live_creature_anchors_synced() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 12);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_graphical_playground_runtime_preview_app_shell(&launch)
            .unwrap();
    app.update();

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);

    let initial_field = app
        .world()
        .resource::<alife_game_app::bevy_shell::GraphicalProceduralTerrainFieldResource>()
        .clone();

    for _ in 0..180 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        app.update();
        let smoke_ticks_done = app
            .world()
            .resource::<alife_game_app::bevy_shell::GraphicalRuntimeControlsResource>()
            .smoke_ticks_done;
        if smoke_ticks_done >= 12 {
            break;
        }
    }

    let controls = app
        .world()
        .resource::<alife_game_app::bevy_shell::GraphicalRuntimeControlsResource>();
    assert!(
        controls.smoke_ticks_done >= 2,
        "smoke should advance enough live ticks for the graphical map to follow creature travel"
    );
    let selected_marker_position = marker_translation(&mut app, WorldEntityId(1));

    let field = app
        .world()
        .resource::<alife_game_app::bevy_shell::GraphicalProceduralTerrainFieldResource>()
        .clone();
    assert!(field.generated_without_rendering);
    assert!(field.materialized_only_near_active_views);
    assert_eq!(
        field.creature_anchor_count,
        initial_field.creature_anchor_count
    );
    assert!(
        !field.active_world_chunks.is_empty(),
        "procedural chunks should remain anchored around live creature positions"
    );
    assert!(
        field.materialized_tiles.len() >= initial_field.materialized_tiles.len(),
        "live creature motion should keep a materialized procedural chunk window available"
    );

    let inspector = app
        .world()
        .resource::<alife_game_app::bevy_shell::CreatureInspectorResource>();
    assert_eq!(inspector.snapshot.selection.stable_id, WorldEntityId(1));
    assert!(
        (inspector.snapshot.visual.position.x * 36.0 - selected_marker_position.x).abs() <= 0.1
            && (inspector.snapshot.visual.position.z * 36.0 - selected_marker_position.y).abs()
                <= 0.1,
        "read-only inspector should reflect the selected live-world stable-ID marker position"
    );
}

#[cfg(feature = "bevy-app")]
#[test]
fn runtime_biome_map_refreshes_when_active_chunk_window_changes() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_graphical_playground_runtime_preview_app_shell(&launch)
            .unwrap();
    app.update();

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);

    let before = runtime_biome_map(&mut app);
    assert_eq!(before.refresh_count, 0);
    assert!(before.active_chunk_signature > 0);

    app.world_mut()
        .resource_mut::<alife_game_app::bevy_shell::GraphicalViewModeResource>()
        .mode = GraphicalPlaygroundViewMode::DevOverlay;
    {
        let mut field = app
            .world_mut()
            .resource_mut::<alife_game_app::bevy_shell::GraphicalProceduralTerrainFieldResource>(
        );
        let chunk_tile_size = field.chunk_tile_size;
        field.active_world_chunks.clear();
        field
            .active_world_chunks
            .extend([(20, -12), (21, -12), (20, -11)]);
        field.creature_anchor_count = field.creature_anchor_count.saturating_add(1);
        field
            .materialized_tiles
            .insert((20 * chunk_tile_size, -12 * chunk_tile_size));
    }

    app.update();

    let after = runtime_biome_map(&mut app);
    assert_eq!(after.refresh_count, before.refresh_count + 1);
    assert_ne!(
        after.active_chunk_signature, before.active_chunk_signature,
        "runtime biome surface must regenerate when creature-anchored active chunks change"
    );
    assert_eq!(after.active_chunk_count, 3);
    assert_eq!(
        after.last_creature_anchor_count,
        before.last_creature_anchor_count + 1
    );
    assert!(after.fog_of_war_applied);
    assert!(
        after.last_materialized_tile_count >= before.last_materialized_tile_count,
        "refresh metadata should retain materialized procedural terrain evidence"
    );
    assert!(after.display_only);
    assert!(after.primary_player_surface);
}

#[cfg(feature = "bevy-app")]
fn marker_translation(
    app: &mut bevy::prelude::App,
    stable_id: WorldEntityId,
) -> bevy::prelude::Vec3 {
    let mut query = app.world_mut().query::<(
        &alife_game_app::bevy_shell::GraphicalPlaygroundMarker,
        &bevy::prelude::Transform,
    )>();
    query
        .iter(app.world())
        .find_map(|(marker, transform)| {
            (marker.stable_id == stable_id).then_some(transform.translation)
        })
        .unwrap_or_else(|| panic!("missing marker for stable ID {}", stable_id.raw()))
}

#[cfg(feature = "bevy-app")]
fn runtime_biome_map(
    app: &mut bevy::prelude::App,
) -> alife_game_app::bevy_shell::GraphicalRuntimeProceduralBiomeMap {
    let mut query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalRuntimeProceduralBiomeMap>();
    let maps = query.iter(app.world()).copied().collect::<Vec<_>>();
    assert_eq!(maps.len(), 1);
    maps[0]
}

#[cfg(feature = "bevy-app")]
#[test]
fn procedural_world_content_uses_alpha_art_and_no_action_authority() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch).unwrap();
    app.update();

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);

    let field = app
        .world()
        .resource::<alife_game_app::bevy_shell::GraphicalProceduralTerrainFieldResource>()
        .clone();
    assert!(
        field.active_content_count >= 160,
        "default Player View should show a dense creature-anchored ecology layer, not sparse fixture objects"
    );
    assert!(field.procedural_content_generated_without_rendering);
    assert!(!field.procedural_content_rendering_required);

    let mut marker_query =
        app.world_mut()
            .query::<&alife_game_app::bevy_shell::GraphicalProceduralWorldContentMarker>();
    let markers = marker_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert_eq!(markers.len(), field.active_content_count);
    assert!(markers.iter().all(|marker| {
        marker.generated_without_rendering
            && !marker.rendering_required
            && marker.creature_context_candidate
            && !marker.can_emit_actions
            && !marker.can_rewrite_weights
    }));
    for kind in [
        alife_world::ProceduralWorldContentKind::Food,
        alife_world::ProceduralWorldContentKind::Hazard,
        alife_world::ProceduralWorldContentKind::Obstacle,
        alife_world::ProceduralWorldContentKind::DressingProp,
    ] {
        assert!(
            markers.iter().any(|marker| marker.kind == kind),
            "missing generated procedural content kind {kind:?}"
        );
    }

    let mut art_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalAlphaArtBackedSprite>();
    let generated_art_roles = art_query
        .iter(app.world())
        .filter(|sprite| {
            sprite
                .stable_id
                .map(|id| id.raw() >= alife_world::PROCEDURAL_CONTENT_ID_BASE)
                .unwrap_or(false)
        })
        .map(|sprite| sprite.role)
        .collect::<Vec<_>>();
    for role in ["food", "hazard", "rock-obstacle", "prop-dressing"] {
        assert!(
            generated_art_roles.contains(&role),
            "generated content should use alpha-art role {role}"
        );
    }
    assert!(
        generated_art_roles.len() >= 160,
        "target-style player view needs many small asset-backed world objects"
    );
}

#[cfg(feature = "bevy-app")]
#[test]
fn production_player_view_composition_layers_are_asset_backed_and_display_only() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch).unwrap();
    app.update();

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);

    let mut layer_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::GraphicalProductionArtLayer>();
    let layers = layer_query.iter(app.world()).copied().collect::<Vec<_>>();
    assert!(
        layers.iter().all(|layer| layer.display_only),
        "production composition layers must remain presentation-only"
    );

    let shadow_count = layers
        .iter()
        .filter(|layer| layer.role == "entity-shadow")
        .count();
    let baked_backdrop_count = layers
        .iter()
        .filter(|layer| {
            layer.role == "world-painted-viewport" || layer.role == "world-atmospheric-underlay"
        })
        .count();
    let streamed_terrain_count = layers
        .iter()
        .filter(|layer| layer.role == "streamed-procedural-terrain")
        .count();
    let runtime_biome_map_count = layers
        .iter()
        .filter(|layer| layer.role == "runtime-procedural-biome-map")
        .count();
    let terrain_blend_count = layers
        .iter()
        .filter(|layer| layer.role == "terrain-edge-blend")
        .count();

    assert!(
        shadow_count >= 4,
        "expected asset-backed entity shadows for visible objects"
    );
    assert_eq!(
        baked_backdrop_count, 0,
        "default Player View should not use a baked/debug backdrop"
    );
    assert_eq!(
        runtime_biome_map_count, 1,
        "default Player View should use a runtime-generated biome map as the continuous world surface"
    );
    assert!(
        streamed_terrain_count >= 29 * 21,
        "default Player View should be built from streamed procedural terrain chunks"
    );
    assert!(
        terrain_blend_count >= 8,
        "expected organic blend layers to soften the visible procedural chunk tiles"
    );
}

#[cfg(feature = "bevy-app")]
#[test]
fn production_hud_skin_uses_committed_ui_assets_in_player_view() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch).unwrap();
    app.update();

    assert_eq!(summary.view_mode, GraphicalPlaygroundViewMode::Player);

    let mut skin_query = app.world_mut().query::<(
        &alife_game_app::bevy_shell::GraphicalProductionHudSkinLayer,
        &bevy::prelude::ImageNode,
    )>();
    let skins = skin_query
        .iter(app.world())
        .map(|(skin, _)| *skin)
        .collect::<Vec<_>>();

    assert!(
        skins.iter().all(|skin| skin.display_only),
        "HUD skin layers must be display-only"
    );
    for role in ["ui-status-chip", "ui-meter-bar"] {
        assert!(
            skins.iter().any(|skin| skin.role == role),
            "missing production HUD skin role {role}"
        );
    }

    let chip_count = skins
        .iter()
        .filter(|skin| skin.role == "ui-status-chip")
        .count();
    assert!(
        chip_count >= 4,
        "default HUD should use compact status chips instead of large debug frames"
    );
    assert!(
        skins
            .iter()
            .all(|skin| skin.role != "ui-panel-frame" && skin.role != "ui-inspector-frame"),
        "default Player View must not use large ornate debug HUD frames"
    );
}

#[cfg(feature = "bevy-app")]
#[test]
fn production_alpha_art_pose_mapping_uses_committed_animation_frames() {
    let cases = [
        (
            CreatureAnimationState::Idle,
            CreatureExpressionState::Neutral,
            "creature-idle",
        ),
        (
            CreatureAnimationState::Moving,
            CreatureExpressionState::Energized,
            "creature-moving",
        ),
        (
            CreatureAnimationState::Interacting,
            CreatureExpressionState::Hungry,
            "creature-eat",
        ),
        (
            CreatureAnimationState::Sleeping,
            CreatureExpressionState::Tired,
            "creature-sleep",
        ),
        (
            CreatureAnimationState::Signaling,
            CreatureExpressionState::Curious,
            "creature-signal",
        ),
        (
            CreatureAnimationState::Inspecting,
            CreatureExpressionState::Curious,
            "creature-signal",
        ),
        (
            CreatureAnimationState::Hurt,
            CreatureExpressionState::Pained,
            "creature-hurt",
        ),
    ];

    for (animation, expression, expected_role) in cases {
        let pose = alife_game_app::ca38_creature_pose_for_state(animation, expression);
        assert_eq!(
            alife_game_app::bevy_shell::ca44a_creature_art_role_for_pose(pose),
            expected_role
        );
    }
}

#[test]
fn graphical_gpu_telemetry_overlay_is_honest_and_bounded() {
    let telemetry = GraphicalGpuRuntimeTelemetry {
        requested_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        selected_backend: "GpuStatic".to_string(),
        fallback_reason: None,
        hardware_identifier: Some("local-test-adapter".to_string()),
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow".to_string(),
        gpu_static_dispatched_ticks: 3,
        gpu_scores_used_for_proposals: true,
        cpu_shadow_parity: true,
        parity_failures: 0,
        sealed_patches: 3,
        h_shadow_applications: 1,
        last_h_shadow_delta: 0.015,
        compact_readback_bytes: 64,
        post_seal_readback_bytes: 64,
        total_gpu_runtime_ms: 1.25,
        wgsl: test_wgsl_telemetry(),
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };
    telemetry.validate().unwrap();
    let overlay = telemetry.overlay_lines();
    let inspector = telemetry.inspector_lines();
    assert!(overlay.contains("scores=true"));
    assert!(overlay.contains("WGSL: tick=3"));
    assert!(overlay.contains("compute=0.80ms"));
    assert!(overlay.contains("No bulk neural readback=true"));
    assert!(inspector.contains("Claim:"));
    assert!(inspector.contains("CpuShadowGuardedStaticPlusLiveHShadow"));
    assert!(inspector.contains("Gate: CPU shadow"));
    assert!(inspector.contains("No full action-authoritative claim"));
    assert!(!inspector.contains("Entity("));

    let pending = GraphicalGpuRuntimeTelemetry::pending(
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
    );
    assert_eq!(pending.selected_backend, "PendingFirstTick");
    assert_eq!(pending.product_runtime_claim, "PendingTick");
    assert!(pending.fallback_reason.is_none());
}

#[test]
fn graphical_runtime_overlay_is_gpu_first_without_false_pretick_events() {
    let graphical_launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let launch = graphical_launch.app_launch.clone();
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let pending = GraphicalGpuRuntimeTelemetry::pending(
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
    );

    let initial =
        panel.status_overlay_text_with_backend(&pending.backend_line(), &pending.overlay_lines());
    assert!(initial.contains("A-Life GPU Alpha Playground"));
    assert!(initial.contains("Press Space to run, or N to step"));
    assert!(initial.contains("GPU path is armed"));
    assert!(initial.contains("Gate: CPU shadow"));
    assert!(initial.contains("Events (last 5):"));
    assert!(!initial.contains("GPU proposal accepted after CPU shadow parity"));
    assert!(!initial.contains("Patch sealed count=0"));
    assert!(!initial.contains("Entity("));

    let step = panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    assert_eq!(step.len(), 1);
    let stepped =
        panel.status_overlay_text_with_backend(&pending.backend_line(), &pending.overlay_lines());
    assert!(stepped.contains("Tick advanced with status Normal"));
    assert!(stepped.contains("Creature action APPROACH toward stable:2"));
    assert!(stepped.contains("Intent line stable:1 -> stable:2"));
    assert!(stepped.contains("Patch sealed count=1"));
    assert_eq!(
        panel.player_events.len(),
        alife_game_app::S02_MAX_PLAYER_EVENT_LINES
    );
    assert_eq!(
        panel.intent_marker_label(),
        "stable:1 -> stable:2 (APPROACH)"
    );
    panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    assert_eq!(panel.intent_marker_label(), "stable:1 -> stable:2 (EAT)");
    let eaten =
        panel.status_overlay_text_with_backend(&pending.backend_line(), &pending.overlay_lines());
    assert!(eaten.contains("Food interaction cue highlighted"));
    assert!(!stepped.contains("Entity("));
}

#[test]
fn graphical_runtime_event_feed_keeps_last_five_meaningful_events() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);

    panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    panel
        .apply_command(&mut live, RuntimeControlCommand::SetRunSpeed(1))
        .unwrap();
    panel
        .apply_command(&mut live, RuntimeControlCommand::SetRunSpeed(2))
        .unwrap();
    panel
        .apply_command(&mut live, RuntimeControlCommand::SetRunSpeed(3))
        .unwrap();

    assert_eq!(
        panel.player_events.len(),
        alife_game_app::S02_MAX_PLAYER_EVENT_LINES
    );
    assert!(panel
        .player_events
        .iter()
        .any(|event| event.contains("Run speed set to 3x")));
    assert!(panel
        .player_events
        .iter()
        .any(|event| event.contains("Patch sealed count=")));
    assert!(panel
        .player_events
        .iter()
        .all(|event| !event.contains("Entity(")));
}

#[test]
fn ca05_structured_status_and_event_panels_keep_player_ui_compact() {
    let graphical_launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let launch = graphical_launch.app_launch.clone();
    let live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let pending = GraphicalGpuRuntimeTelemetry::pending(
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
    );

    let status = panel.structured_status_panel_text_with_backend(&pending.backend_line());
    assert!(status.contains("Status"));
    assert!(status.contains("A-Life GPU Alpha Playground"));
    assert!(status.contains("GPU: GpuPlastic"));
    assert!(status.contains("Creature: stable:1"));
    assert!(status.contains("Goal: idle"));
    assert!(status.contains("Learning: H_shadow pulse"));
    assert!(!status.contains("Events (last 5):"));
    assert!(!status.contains("No full action-authoritative"));
    assert!(!status.contains("Entity("));

    panel.record_control_event("GPU proposal accepted after CPU shadow parity");
    let events = panel.event_feed_panel_text();
    assert!(events.contains("Event Feed"));
    assert!(events.contains("GPU proposal accepted after CPU shadow parity"));
    assert!(!events.contains("Entity("));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca05_controls_and_boundary_footer_are_player_facing() {
    let controls = alife_game_app::bevy_shell::ca05_controls_bar_text();
    assert!(controls.contains("Controls"));
    assert!(controls.contains("click"));
    assert!(controls.contains("Space run/pause"));
    assert!(controls.contains("N step"));
    assert!(controls.contains("R reset"));
    assert!(controls.contains("+/- zoom"));
    assert!(controls.contains("F follow"));
    assert!(controls.contains("[!] hazard"));
    assert!(!controls.contains("Entity("));

    let telemetry = GraphicalGpuRuntimeTelemetry {
        requested_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        selected_backend: "GpuStatic".to_string(),
        fallback_reason: None,
        hardware_identifier: Some("local-test-adapter".to_string()),
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow".to_string(),
        gpu_static_dispatched_ticks: 3,
        gpu_scores_used_for_proposals: true,
        cpu_shadow_parity: true,
        parity_failures: 0,
        sealed_patches: 3,
        h_shadow_applications: 1,
        last_h_shadow_delta: 0.015,
        compact_readback_bytes: 64,
        post_seal_readback_bytes: 64,
        total_gpu_runtime_ms: 1.25,
        wgsl: test_wgsl_telemetry(),
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };
    let footer = alife_game_app::bevy_shell::ca05_boundary_footer_text(&telemetry);
    assert!(footer.contains("Boundary: CPU shadow gate"));
    assert!(footer.contains("Claim: CpuShadowGuardedStaticPlusLiveHShadow"));
    assert!(footer.contains("no full action-authoritative"));
    assert!(footer.contains("no bulk readback=true"));
    assert!(!footer.contains("Entity("));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca06_mouse_selection_updates_stable_id_camera_and_inspector() {
    let graphical_launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let launch = graphical_launch.app_launch.clone();
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    let marker_records = presentation
        .objects
        .iter()
        .map(|object| {
            (
                object.stable_id,
                object.kind,
                bevy::prelude::Vec3::new(object.position.x * 125.0, object.position.z * 125.0, 0.0),
            )
        })
        .collect::<Vec<_>>();
    let (_, _, food_translation) = marker_records
        .iter()
        .copied()
        .find(|(stable_id, kind, _)| {
            *stable_id == WorldEntityId(2) && *kind == WorldObjectKind::Food
        })
        .expect("gpu alpha fixture should expose stable food marker");
    let picked = alife_game_app::bevy_shell::ca06_pick_stable_id_from_world_point(
        bevy::prelude::Vec2::new(food_translation.x, food_translation.y),
        marker_records
            .iter()
            .map(|(stable_id, kind, translation)| (*stable_id, *kind, *translation)),
    );
    assert_eq!(picked, Some(WorldEntityId(2)));
    let missed = alife_game_app::bevy_shell::ca06_pick_stable_id_from_world_point(
        bevy::prelude::Vec2::new(10_000.0, 10_000.0),
        marker_records
            .iter()
            .map(|(stable_id, kind, translation)| (*stable_id, *kind, *translation)),
    );
    assert_eq!(missed, None);

    let inspector_snapshot = run_creature_inspector_smoke(&launch).unwrap();
    let mut selection = alife_game_app::bevy_shell::SelectionResource {
        stable_id: inspector_snapshot.selection.stable_id,
        local_entity: None,
    };
    let mut inspector = alife_game_app::bevy_shell::CreatureInspectorResource {
        snapshot: inspector_snapshot.clone(),
    };
    let mut camera = alife_game_app::bevy_shell::CameraNavigationResource {
        state: inspector_snapshot.camera,
    };
    let mut runtime =
        alife_game_app::bevy_shell::GraphicalRuntimeControlsResource::new(&graphical_launch)
            .unwrap();
    let food_entity = bevy::prelude::Entity::PLACEHOLDER;

    alife_game_app::bevy_shell::apply_graphical_stable_selection(
        &presentation,
        WorldEntityId(2),
        Some(food_entity),
        &mut selection,
        &mut inspector,
        &mut camera,
        &mut runtime,
    )
    .unwrap();

    let expected = select_visible_world_entity(&presentation, WorldEntityId(2)).unwrap();
    assert_eq!(selection.stable_id, WorldEntityId(2));
    assert_eq!(selection.local_entity, Some(food_entity));
    assert_eq!(inspector.snapshot.selection.stable_id, WorldEntityId(2));
    assert_eq!(camera.state.focus, expected.position);
    assert!(runtime
        .panel
        .player_events
        .iter()
        .any(|event| event.contains("Mouse selected stable:2")));
    let gpu = alife_game_app::bevy_shell::GraphicalGpuTelemetryResource {
        telemetry: GraphicalGpuRuntimeTelemetry::pending(
            GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        ),
    };
    let overlay = alife_game_app::bevy_shell::graphical_inspector_overlay_text(
        &runtime, &camera, &selection, &inspector, &gpu,
    );
    assert!(overlay.contains("Creature Inspector"));
    assert!(overlay.contains("Stable ID: 2"));
    assert!(overlay.contains("Energy "));
    assert!(overlay.contains("Health "));
    assert!(overlay.contains("Hunger "));
    assert!(overlay.contains("Fatigue"));
    assert!(overlay.contains("Fear   "));
    assert!(overlay.contains("Learning: H_shadow="));
    assert!(overlay.contains("Read-only stable IDs"));
    assert!(!overlay.contains("Entity("));
}

#[test]
fn ca04_terminal_recovery_and_reset_are_player_visible() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);

    panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    panel.record_terminal_recovery("invalid action/state");
    let terminal = panel.status_overlay_text();
    assert!(terminal.contains("Simulation stopped: invalid action/state. Press R to restart."));
    assert!(terminal.contains("Controls: Space run/pause | N step | R reset | Esc quit"));
    assert!(panel
        .player_events
        .iter()
        .any(|event| event.contains("Press R to restart")));
    assert!(!terminal.contains("Entity("));

    panel
        .apply_command(&mut live, RuntimeControlCommand::RestartAlphaFixture)
        .unwrap();
    let reset = panel.status_overlay_text();
    assert!(reset.contains("Alpha fixture reset; stable IDs preserved"));
    assert!(!reset.contains("Simulation stopped:"));
    assert_eq!(panel.playback, RuntimePlaybackState::Paused);
    assert_eq!(panel.world_tick, None);
    assert_eq!(panel.sealed_patch_count, 0);
    assert_eq!(panel.packed_record_count, 0);
    assert_eq!(panel.intent_marker_label(), "pending");
}

#[test]
fn ca03_action_badges_are_player_facing_and_stable_id_safe() {
    assert_eq!(
        alife_game_app::action_badge_label(ActionKind::Move),
        "APPROACH"
    );
    assert_eq!(
        alife_game_app::action_badge_label_for_target(ActionKind::Move, Some(2)),
        "APPROACH"
    );
    assert_eq!(
        alife_game_app::action_badge_label_for_target(ActionKind::Move, Some(3)),
        "FLEE"
    );
    assert_eq!(
        alife_game_app::action_badge_label(ActionKind::Interact),
        "EAT"
    );
    assert_eq!(
        alife_game_app::action_badge_label(ActionKind::Inspect),
        "INSPECT"
    );
    assert_eq!(
        alife_game_app::action_badge_label(ActionKind::Rest),
        "SLEEP"
    );
    assert_eq!(alife_game_app::action_badge_label(ActionKind::Idle), "IDLE");

    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();

    assert_eq!(
        panel.intent_marker_label(),
        "stable:1 -> stable:2 (APPROACH)"
    );
    assert!(panel
        .status_overlay_text()
        .contains("Intent: stable:1 -> stable:2 (APPROACH)"));
    panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    assert_eq!(panel.intent_marker_label(), "stable:1 -> stable:2 (EAT)");
    assert!(!panel.status_overlay_text().contains("Entity("));

    panel.selected_action_kind = Some(ActionKind::Move);
    panel.target_entity = Some(3);
    assert_eq!(panel.intent_marker_label(), "stable:1 -> stable:3 (FLEE)");
    assert!(panel
        .status_overlay_text()
        .contains("Creature: stable:1  Goal: hazard  Action: FLEE"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca03_intent_line_and_action_badge_are_stable_id_presentation_only() {
    let graphical_launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let launch = graphical_launch.app_launch.clone();
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    panel.selected_action_kind = Some(ActionKind::Move);
    panel.target_entity = Some(3);

    let mut app = alife_game_app::bevy_shell::build_ca03_intent_feedback_preview_app_shell(
        &graphical_launch,
        panel,
    )
    .unwrap();
    app.update();

    let mut badge_query = app.world_mut().query_filtered::<
        (&bevy::prelude::Text2d, &bevy::prelude::TextColor),
        bevy::prelude::With<alife_game_app::bevy_shell::GraphicalActionBadge>,
    >();
    let badges = badge_query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(badges.len(), 1);
    assert_eq!(badges[0].0 .0.as_str(), "action: flee");

    let mut marker_query = app.world_mut().query::<(
        &alife_game_app::bevy_shell::GraphicalPlaygroundMarker,
        &bevy::prelude::Transform,
    )>();
    let markers = marker_query
        .iter(app.world())
        .map(|(marker, transform)| (marker.stable_id.raw(), transform.translation))
        .collect::<Vec<_>>();
    assert!(markers.iter().any(|(id, _)| *id == 1));
    assert!(markers.iter().any(|(id, _)| *id == 3));

    let mut line_query = app.world_mut().query_filtered::<
        (&bevy::prelude::Sprite, &bevy::prelude::Transform),
        bevy::prelude::With<alife_game_app::bevy_shell::GraphicalIntentLine>,
    >();
    let lines = line_query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    let line_size = lines[0]
        .0
        .custom_size
        .expect("CA03 intent line should expose a visible bounded sprite");
    assert!(
        line_size.x > 1.0,
        "expected visible intent line, size={line_size:?}, markers={markers:?}"
    );
    assert_eq!(line_size.y, 5.0);
    assert!(lines[0].1.translation.z > 0.0);
}

#[test]
fn graphical_controls_smoke_verifies_first_tester_controls_without_key_injection() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_graphical_controls_smoke(&launch).unwrap();

    assert!(summary.toggle_pause_run_verified);
    assert_eq!(summary.speed_sequence, [1, 2, 3]);
    assert_eq!(summary.follow_target, Some(WorldEntityId(1)));
    assert!(summary.reset_verified);
    assert!(summary.terminal_guidance_visible);
    assert!(summary.exit_requested);
    assert_eq!(
        summary.runtime.panel.playback,
        RuntimePlaybackState::ShutdownRequested
    );
    assert_eq!(summary.runtime.step_produced, 1);
    assert!(summary.runtime.run_produced > 0);
    assert!(summary.runtime.all_patches_sealed);
    assert!(summary.overlay_text.contains("Alpha fixture reset"));
    assert!(summary
        .overlay_text
        .contains("Simulation stopped: invalid action/state"));
    assert!(summary.overlay_text.contains("Press R to restart"));
    assert!(summary.overlay_text.contains("Controls:"));
    assert!(!summary
        .overlay_text
        .contains("full action-authoritative claim=true"));
    assert!(!summary.overlay_text.contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn creature_visual_smoke_maps_live_state_without_mutating_cognition() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let visual = run_creature_visual_smoke(&launch).unwrap();

    assert_eq!(visual.schema, alife_game_app::G04_CREATURE_VISUAL_SCHEMA);
    assert_eq!(
        visual.schema_version,
        alife_game_app::G04_CREATURE_VISUAL_SCHEMA_VERSION
    );
    assert_eq!(visual.organism_id, OrganismId(1));
    assert_eq!(visual.stable_id, WorldEntityId(1));
    assert_eq!(visual.selected_action_kind, Some(ActionKind::Interact));
    assert_eq!(visual.target_entity, Some(WorldEntityId(2)));
    assert_eq!(visual.animation, CreatureAnimationState::Interacting);
    assert!(matches!(
        visual.expression,
        CreatureExpressionState::Neutral
            | CreatureExpressionState::Hungry
            | CreatureExpressionState::Energized
    ));
    assert!(visual.cues.hunger.value <= 1.0);
    assert!(visual.cues.energy.value <= 1.0);
    assert!(visual.signature_line().contains("Interact"));
}

#[test]
fn inspector_smoke_selects_creature_and_reports_read_only_runtime_state() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let inspector = run_creature_inspector_smoke(&launch).unwrap();

    assert_eq!(
        inspector.schema,
        alife_game_app::G05_CAMERA_INSPECTOR_SCHEMA
    );
    assert_eq!(
        inspector.schema_version,
        alife_game_app::G05_CAMERA_INSPECTOR_SCHEMA_VERSION
    );
    assert!(inspector.read_only);
    assert_eq!(inspector.selection.stable_id, WorldEntityId(1));
    assert_eq!(inspector.selection.organism_id, Some(OrganismId(1)));
    assert_eq!(inspector.camera.follow_target, Some(WorldEntityId(1)));
    assert!(inspector.action_summary.contains("Interact"));
    assert!(inspector.patch_summary.contains("sealed=true"));
    assert!(inspector
        .memory_topology_summary
        .contains("topology_updates=1"));
    assert!(inspector
        .fallback_summary
        .contains("GPU/semantic providers optional"));
    assert!(inspector
        .semantic_context_summary
        .contains("semantic_provider=disabled"));
}

#[test]
fn selection_and_camera_controls_are_stable_id_based_and_deterministic() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    let selection = select_visible_world_entity(&presentation, WorldEntityId(1)).unwrap();
    let camera = CameraNavigationState::top_down_default()
        .focus_on(selection.position)
        .unwrap()
        .with_follow_target(selection.stable_id)
        .unwrap()
        .zoom_by(0.5)
        .unwrap()
        .orbit_by(90.0)
        .unwrap();

    assert_eq!(selection.kind, WorldObjectKind::Agent);
    assert_eq!(camera.follow_target, Some(selection.stable_id));
    assert_eq!(camera.zoom, 1.5);
    assert_eq!(camera.yaw_degrees, 90.0);
    assert!(camera.signature_line().contains("90.00"));
}

#[test]
fn inspector_pause_step_controls_preserve_existing_deterministic_scheduler() {
    assert_eq!(
        InspectorControlPanel::paused().to_live_control().unwrap(),
        LiveBrainTickControl::paused()
    );
    assert_eq!(
        InspectorControlPanel::step_once()
            .to_live_control()
            .unwrap(),
        LiveBrainTickControl::step_once()
    );
    assert_eq!(
        InspectorControlPanel::run_fixed(2, 100)
            .to_live_control()
            .unwrap(),
        LiveBrainTickControl::run_fixed(2)
    );
}

#[test]
fn playable_survival_loop_exercises_food_hazard_sleep_and_logs() {
    let summary = run_playable_survival_loop_smoke().unwrap();

    assert_eq!(
        summary.schema,
        alife_game_app::G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA
    );
    assert_eq!(
        summary.schema_version,
        alife_game_app::G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA_VERSION
    );
    assert_eq!(
        summary.event_labels(),
        vec![
            "food-consumed",
            "missing-affordance",
            "hazard-pain",
            "rest-sleep"
        ]
    );
    assert_eq!(summary.object_count, 5);
    assert_eq!(summary.events.len(), 4);
    assert_eq!(summary.tick_summaries.len(), 4);
    assert_eq!(summary.sealed_patch_count, 4);
    assert_eq!(summary.packed_record_count, 4);
    assert!(summary.memory_record_count >= 4);
    assert!(summary.topology_concept_count >= 1);
    assert!(summary
        .world_signature
        .iter()
        .any(|line| line.contains("Food")));
    assert!(summary
        .world_signature
        .iter()
        .any(|line| line.contains("Hazard")));
    summary.validate().unwrap();
}

#[test]
fn food_loop_reduces_hunger_and_failure_does_not_retry_infinitely() {
    let summary = run_playable_survival_loop_smoke().unwrap();
    let food = &summary.events[0];
    assert_eq!(food.kind, PlayableSurvivalEventKind::FoodConsumed);
    assert!(food.success);
    assert_eq!(food.action_kind, Some(ActionKind::Interact));
    assert!(food.hunger_after < food.hunger_before);

    let missing = &summary.events[1];
    assert_eq!(missing.kind, PlayableSurvivalEventKind::MissingAffordance);
    assert!(!missing.success);
    assert_eq!(
        summary.tick_summaries[1].status,
        BrainTickStatus::RecoverableActionFailure
    );
    assert!(summary.tick_summaries[1].action_failure.is_some());
    assert_eq!(summary.tick_summaries[1].sealed_patch_count, 2);
}

#[test]
fn hazard_loop_records_bias_only_memory_topology_evidence() {
    let summary = run_playable_survival_loop_smoke().unwrap();
    let hazard = &summary.events[2];
    assert_eq!(hazard.kind, PlayableSurvivalEventKind::HazardPain);
    assert!(hazard.success);
    assert_eq!(hazard.action_kind, Some(ActionKind::Move));
    assert!(hazard.pain_after > summary.events[1].pain_after);
    assert!(hazard.fear_after > summary.events[1].fear_after);
    assert!(summary.tick_summaries[2].memory_updates > 0);
    assert!(summary.tick_summaries[2].topology_updates > 0);
    assert!(summary.unresolved_gap_count >= 1);
    assert!(summary.events[2]
        .message
        .contains("topology gap remains bias-only"));
}

#[test]
fn rest_loop_enters_visible_sleep_state_after_sealed_patch() {
    let summary = run_playable_survival_loop_smoke().unwrap();
    let rest = &summary.events[3];
    assert_eq!(rest.kind, PlayableSurvivalEventKind::RestSleep);
    assert!(rest.success);
    assert_eq!(rest.action_kind, Some(ActionKind::Rest));
    assert!(matches!(
        rest.sleep_phase_after,
        alife_core::SleepPhase::EnteringSleep
            | alife_core::SleepPhase::Consolidating
            | alife_core::SleepPhase::ForcedRecoverySleep
    ));
    assert_eq!(summary.tick_summaries[3].sealed_patch_count, 4);
    assert_eq!(
        summary.final_visual.animation,
        CreatureAnimationState::Sleeping
    );
    assert_eq!(
        summary.final_visual.expression,
        CreatureExpressionState::Tired
    );
}

#[test]
fn world_ecology_loop_tracks_regrowth_spawn_pressure_and_logs() {
    let summary = run_world_ecology_loop_smoke().unwrap();

    assert_eq!(summary.schema, alife_game_app::G07_WORLD_ECOLOGY_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G07_WORLD_ECOLOGY_SCHEMA_VERSION
    );
    assert_eq!(summary.seed, 7070);
    assert_eq!(summary.tick_summaries.len(), 4);
    assert_eq!(summary.sealed_patch_count, 4);
    assert_eq!(summary.packed_record_count, 4);
    assert!(summary.metrics.resources_regrown >= 1);
    assert!(summary.metrics.resources_spawned >= 1);
    assert!(summary.metrics.active_resources >= 1);
    assert!(summary.hazard_pain > 0.0);
    assert_eq!(summary.sensory_zone_label.as_deref(), Some("ash-field"));
    assert!(summary
        .ecology_indicators
        .iter()
        .any(|indicator| indicator.label == "meadow"));
    assert!(summary
        .ecology_indicators
        .iter()
        .any(|indicator| indicator.label == "ash-field"));
    assert!(summary
        .world_signature
        .iter()
        .any(|line| line.contains("seed-berry")));
    summary.validate().unwrap();
}

#[test]
fn population_social_loop_runs_two_creatures_in_stable_order() {
    let summary = run_population_social_loop_smoke().unwrap();

    assert_eq!(summary.schema, alife_game_app::G08_POPULATION_SOCIAL_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G08_POPULATION_SOCIAL_SCHEMA_VERSION
    );
    assert_eq!(summary.seed, 8080);
    assert_eq!(summary.creature_count, 2);
    assert_eq!(summary.population_cap, 4);
    assert_eq!(
        summary.schedule_order,
        vec![OrganismId(801), OrganismId(802)]
    );
    assert_eq!(summary.tick_records.len(), 4);
    assert_eq!(summary.metrics.scheduler_steps, 4);
    assert_eq!(summary.metrics.sealed_patch_count, 4);
    assert_eq!(summary.metrics.packed_record_count, 4);
    assert!(summary.metrics.world_object_count >= summary.creature_count);
    summary.validate().unwrap();
}

#[test]
fn population_social_loop_exposes_vocal_tokens_and_social_context_as_perception_only() {
    let summary = run_population_social_loop_smoke().unwrap();

    assert!(summary.tick_records.iter().any(|record| record.event_kind
        == PopulationSocialEventKind::Vocalize
        && record.tick_summary.selected_action_kind == Some(ActionKind::Vocalize)));
    assert!(summary
        .tick_records
        .iter()
        .any(|record| record.heard_tokens > 0));
    assert!(summary
        .tick_records
        .iter()
        .any(|record| record.social_agents_seen > 0));
    assert!(summary
        .tick_records
        .iter()
        .any(|record| record.trust_cues_seen > 0));
    assert!(summary
        .tick_records
        .iter()
        .any(|record| record.fear_cues_seen > 0));
    assert_eq!(
        summary
            .tick_records
            .iter()
            .map(|record| record.social_direct_action_count)
            .sum::<usize>(),
        0
    );
    assert!(summary
        .world_signature
        .iter()
        .any(|line| line.contains("voice-token-801")));
}

#[test]
fn population_social_loop_records_bounded_collision_feedback_and_group_status() {
    let summary = run_population_social_loop_smoke().unwrap();

    assert!(summary.metrics.collision_feedback_count >= 1);
    assert!(summary
        .tick_records
        .iter()
        .any(|record| record.contacted_agents > 0
            && record.tick_summary.physical_contact
                == Some(alife_core::PhysicalContactKind::Collision)));
    assert_eq!(summary.creature_status.len(), 2);
    assert!(summary
        .creature_status
        .iter()
        .all(|status| status.visual.schema == alife_game_app::G04_CREATURE_VISUAL_SCHEMA));
    assert!(summary
        .creature_status
        .iter()
        .all(|status| status.last_action_kind.is_some()));
}

#[test]
fn population_cap_and_schedule_validation_are_strict_and_deterministic() {
    let mut config = PopulationLoopConfig::two_creature_smoke().unwrap();
    config.population_cap = 1;
    assert!(config.validate().is_err());

    let config_a = PopulationLoopConfig::two_creature_smoke().unwrap();
    let config_b = PopulationLoopConfig::two_creature_smoke().unwrap();
    let seed = config_a.seed;
    let rounds = config_a.rounds;
    let mut run_a = PopulationLiveLoop::from_config(config_a).unwrap();
    let mut run_b = PopulationLiveLoop::from_config(config_b).unwrap();
    let summary_a = run_a.run_rounds(rounds, seed).unwrap();
    let summary_b = run_b.run_rounds(rounds, seed).unwrap();
    assert_eq!(summary_a.signature_line(), summary_b.signature_line());
}

#[test]
fn population_performance_lod_smoke_documents_honest_tiers_and_gpu_status() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_population_performance_lod_smoke(&launch).unwrap();

    assert_eq!(
        summary.schema,
        alife_game_app::G18_POPULATION_PERFORMANCE_SCHEMA
    );
    assert_eq!(
        summary.schema_version,
        alife_game_app::G18_POPULATION_PERFORMANCE_SCHEMA_VERSION
    );
    assert_eq!(summary.population_creatures, 2);
    assert_eq!(summary.policy.minimum_playable_population, 10);
    assert_eq!(
        summary
            .policy
            .tier_targets
            .iter()
            .map(|target| target.population)
            .collect::<Vec<_>>(),
        vec![1, 10, 50, 100, 250, 500]
    );
    assert!(summary.tier_1_10_ci_smoke_documented);
    assert!(summary.manual_upper_tiers_documented);
    assert!(!summary.gpu_performance_measured);
    assert!(summary
        .performance_report_markdown
        .contains("GPU performance remains unknown"));
    assert!(summary
        .policy
        .gpu_runtime_manual_command
        .contains("ALIFE_GPU_RUNTIME_BACKEND=static"));
    assert!(summary
        .policy
        .gpu_runtime_manual_command
        .contains("--gpu-runtime"));
    assert!(!summary
        .policy
        .gpu_runtime_manual_command
        .contains("--gpu-report"));
    summary.validate().unwrap();
}

#[test]
fn population_performance_throttling_protects_sensory_motor_homeostasis() {
    let policy = PopulationPerformancePolicy::v1_defaults().unwrap();

    assert_eq!(
        policy.rate_hz(LodResidency::Hot, CadenceTarget::SensoryMotor),
        60.0
    );
    assert!(
        policy.rate_hz(LodResidency::Hot, CadenceTarget::ActionArbitration)
            >= policy.rate_hz(LodResidency::Hot, CadenceTarget::NonessentialCognition)
    );
    assert!(
        policy.rate_hz(LodResidency::Warm, CadenceTarget::SensoryMotor)
            >= policy.rate_hz(LodResidency::Warm, CadenceTarget::NonessentialCognition)
    );

    let under_budget = policy.throttling_decision(8.0, None).unwrap();
    assert_eq!(under_budget.throttle_level, 0);
    assert_eq!(under_budget.nonessential_decimation_factor, 1);

    let over_budget = policy
        .throttling_decision(alife_game_app::G18_TARGET_FRAME_MS * 1.2, None)
        .unwrap();
    assert_eq!(over_budget.throttle_level, 2);
    assert_eq!(over_budget.nonessential_decimation_factor, 4);
    assert!(over_budget.sensory_motor_protected);
    assert!(over_budget.homeostasis_protected);
    assert!(over_budget.action_arbitration_protected);
}

#[test]
fn population_lod_projection_preserves_behavior_signature() {
    let population = run_population_social_loop_smoke().unwrap();
    let policy = PopulationPerformancePolicy::v1_defaults().unwrap();
    let decision = policy
        .throttling_decision(alife_game_app::G18_TARGET_FRAME_MS * 1.2, None)
        .unwrap();
    let projection = project_lod_without_behavior_change(&population, &policy, &decision).unwrap();

    assert_eq!(
        projection.behavior_signature_before,
        projection.behavior_signature_after
    );
    assert_eq!(projection.render_detail, RenderDetailLevel::Full);
    assert!(projection.nonessential_cognition_decimated);
    assert!(projection.feedback_vfx_enabled);
}

#[test]
fn longrun_balance_smoke_reports_plausible_bounded_metrics() {
    let summary = run_longrun_balance_smoke().unwrap();

    assert_eq!(summary.schema, alife_game_app::G19_LONG_RUN_BALANCE_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G19_LONG_RUN_BALANCE_SCHEMA_VERSION
    );
    assert_eq!(
        summary.config.cycles,
        alife_game_app::G19_FAST_BALANCE_CYCLES
    );
    assert!(summary.metrics.survival_score > 0.0);
    assert!(summary.metrics.energy_stability > 0.0);
    assert_eq!(summary.metrics.food_success_rate, 1.0);
    assert!(summary.metrics.hazard_avoidance_score < 1.0);
    assert!(summary.metrics.sleep_cycle_count >= 1);
    assert!(summary.metrics.reproduction_births >= 1);
    assert!(summary.metrics.social_diversity_score > 0.0);
    assert!(summary.metrics.no_unsealed_learning);
    assert!(summary.metrics.invalid_id_rejected);
    assert!(summary.metrics.finite_values);
    assert!(summary.metrics.population_bounds_enforced);
    assert!(summary.metrics.resource_bounds_enforced);
    assert_eq!(summary.behavior_criteria.len(), 6);
    let criterion_ids = summary
        .behavior_criteria
        .iter()
        .map(|criterion| criterion.id)
        .collect::<Vec<_>>();
    assert_eq!(
        criterion_ids,
        vec![
            "food-seeking",
            "hazard-avoidance",
            "sleep-rest",
            "population-bounds",
            "resource-stability",
            "social-diversity"
        ]
    );
    assert!(summary
        .behavior_criteria
        .iter()
        .any(|criterion| criterion.status == "autonomous-ecology-signal"));
    assert!(summary
        .behavior_criteria
        .iter()
        .any(|criterion| criterion.status == "not-yet-emergent"));
    let resource = summary
        .behavior_criteria
        .iter()
        .find(|criterion| criterion.id == "resource-stability")
        .unwrap();
    assert!(resource.evidence.contains("resources_regrown="));
    assert!(resource.evidence.contains("resources_spawned="));
    assert!(summary
        .report_markdown
        .contains("Known degenerate behaviors"));
    assert!(summary
        .report_markdown
        .contains("S06 non-scripted criteria"));
    assert!(summary.report_markdown.contains("S06 assessment"));
    assert!(summary
        .s06_improvement_note
        .contains("autonomous resource regrowth"));
    assert!(summary.manual_extended_command.contains("--ignored"));
    summary.validate().unwrap();
}

#[test]
fn longrun_balance_is_reproducible_by_seed_and_config() {
    let first = run_longrun_balance_smoke().unwrap();
    let second = run_longrun_balance_smoke().unwrap();

    assert_eq!(first.signature_line(), second.signature_line());
    assert_eq!(first.report_markdown, second.report_markdown);
}

#[test]
fn ca21_behavior_tuning_metrics_detect_required_degeneracy_classes() {
    let summary = run_behavior_tuning_metrics_smoke().unwrap();

    assert_eq!(summary.schema, CA21_BEHAVIOR_TUNING_SCHEMA);
    assert_eq!(summary.schema_version, CA21_BEHAVIOR_TUNING_SCHEMA_VERSION);
    assert_eq!(summary.findings.len(), CA21_REQUIRED_DETECTOR_COUNT);
    assert_eq!(summary.scenario_sweeps.len(), CA21_SCENARIO_SWEEP_COUNT);

    let finding_ids = summary
        .findings
        .iter()
        .map(|finding| finding.id)
        .collect::<Vec<_>>();
    assert_eq!(
        finding_ids,
        vec![
            "stagnation",
            "catatonia",
            "overfeeding",
            "hazard-suicide",
            "population-collapse"
        ]
    );

    let overfeeding = summary
        .findings
        .iter()
        .find(|finding| finding.id == "overfeeding")
        .unwrap();
    assert_eq!(
        overfeeding.status,
        BehaviorTuningFindingStatus::KnownLimitation
    );
    let hazard = summary
        .findings
        .iter()
        .find(|finding| finding.id == "hazard-suicide")
        .unwrap();
    assert_eq!(hazard.status, BehaviorTuningFindingStatus::KnownLimitation);
    assert!(summary.no_hidden_overfitting);
    assert!(summary
        .report_markdown
        .contains("Known degenerate behavior list"));
    assert!(summary.report_markdown.contains("No hidden overfitting"));
    assert!(summary
        .known_degenerate_behaviors
        .iter()
        .any(|behavior| behavior.contains("overfeeding risk")));
    assert!(summary
        .known_degenerate_behaviors
        .iter()
        .any(|behavior| behavior.contains("hazard-suicide risk")));
    summary.validate().unwrap();
}

#[test]
fn ca21_behavior_tuning_config_bounds_are_enforced() {
    let mut config = BehaviorTuningConfig::fast_ci();
    config.minimum_sealed_patches = 0;
    assert!(config.validate().is_err());

    let mut config = BehaviorTuningConfig::fast_ci();
    config.overfeeding_watch_threshold = 1.5;
    assert!(config.validate().is_err());

    let mut config = BehaviorTuningConfig::fast_ci();
    config.minimum_population_observed = alife_game_app::G08_MAX_POPULATION_CAP + 1;
    assert!(config.validate().is_err());
}

#[test]
fn ca21_behavior_tuning_report_is_reproducible_and_not_overclaimed() {
    let first = run_behavior_tuning_metrics_with_config(BehaviorTuningConfig::fast_ci()).unwrap();
    let second = run_behavior_tuning_metrics_with_config(BehaviorTuningConfig::fast_ci()).unwrap();

    assert_eq!(first.signature_line(), second.signature_line());
    assert_eq!(first.report_markdown, second.report_markdown);
    assert!(first
        .report_markdown
        .contains("does not alter action arbitration"));
    assert!(first
        .report_markdown
        .contains("CPU fallback and CPU shadow parity remain separate"));
    assert!(!first.report_markdown.contains("full action-authoritative"));
    assert!(first.scenario_sweeps.iter().all(|sweep| sweep.bounded_ci));
    first.validate().unwrap();
}

#[test]
fn ca22_ecological_soak_smoke_records_bounds_and_remaining_issues() {
    let summary = run_ecological_soak_smoke().unwrap();

    assert_eq!(summary.schema, CA22_ECOLOGICAL_SOAK_SCHEMA);
    assert_eq!(summary.schema_version, CA22_ECOLOGICAL_SOAK_SCHEMA_VERSION);
    assert_eq!(
        summary.metrics.headless_ticks_completed,
        CA22_FAST_HEADLESS_TICKS
    );
    assert_eq!(
        summary.metrics.headless_ticks_requested,
        CA22_FAST_HEADLESS_TICKS
    );
    assert_eq!(summary.metrics.first_failure_tick, None);
    assert!(summary.metrics.ecology_metric_samples > 0);
    assert_eq!(summary.findings.len(), CA21_REQUIRED_DETECTOR_COUNT);
    assert!(summary.metrics.population_bounds_enforced);
    assert!(summary.metrics.resource_bounds_enforced);
    assert!(summary.metrics.no_unsealed_learning);
    assert!(summary.metrics.resources_regrown_or_spawned);
    assert!(summary.config_first_tuning);
    assert!(!summary.full_emergent_ecology_claim);
    assert_eq!(
        summary.gpu_product_claim,
        "CpuShadowGuardedStaticPlusLiveHShadow"
    );
    assert!(summary.cpu_shadow_parity_preserved);
    assert!(summary.manual_10k_command.contains("--ignored"));
    assert!(summary
        .manual_10k_command
        .contains("ca22_manual_10k_ecological_soak"));
    assert!(summary
        .graphical_bounded_command
        .contains("run_graphical_playground.ps1"));
    assert!(summary.report_markdown.contains("Remaining issues"));
    assert!(summary.report_markdown.contains("config-first"));
    assert!(summary
        .report_markdown
        .contains("full action-authoritative GPU runtime are not claimed"));
    summary.validate().unwrap();
}

#[test]
fn ca22_ecological_soak_config_bounds_are_enforced() {
    let mut config = EcologicalSoakConfig::fast_ci();
    config.headless_ticks = 0;
    assert!(config.validate().is_err());

    let mut config = EcologicalSoakConfig::fast_ci();
    config.headless_ticks = CA22_MANUAL_HEADLESS_TICKS + 1;
    assert!(config.validate().is_err());

    let mut config = EcologicalSoakConfig::fast_ci();
    config.population_cap = alife_game_app::G08_MAX_POPULATION_CAP + 1;
    assert!(config.validate().is_err());

    let mut config = EcologicalSoakConfig::fast_ci();
    config.report_every = config.headless_ticks + 1;
    assert!(config.validate().is_err());
}

#[test]
fn ca22_ecological_soak_report_is_reproducible_and_honest() {
    let first = run_ecological_soak_with_config(EcologicalSoakConfig::fast_ci()).unwrap();
    let second = run_ecological_soak_with_config(EcologicalSoakConfig::fast_ci()).unwrap();

    assert_eq!(first.signature_line(), second.signature_line());
    assert_eq!(first.report_markdown, second.report_markdown);
    assert!(first
        .findings
        .iter()
        .any(|finding| finding.status == BehaviorTuningFindingStatus::KnownLimitation));
    assert!(first.report_markdown.contains(
        "Full emergent ecology and full action-authoritative GPU runtime are not claimed"
    ));
    assert!(first
        .report_markdown
        .contains("CPU shadow parity and CPU fallback remain preserved"));
    assert!(!first.report_markdown.contains("release tag"));
    first.validate().unwrap();
}

#[test]
#[ignore = "manual CA22 10k ecology soak: cargo test -p alife_game_app --test app_shell ca22_manual_10k_ecological_soak -- --ignored --nocapture"]
fn ca22_manual_10k_ecological_soak() {
    let summary = run_ecological_soak_with_config(EcologicalSoakConfig::manual_10k()).unwrap();

    assert_eq!(
        summary.metrics.headless_ticks_completed,
        CA22_MANUAL_HEADLESS_TICKS
    );
    assert_eq!(
        summary.metrics.headless_ticks_requested,
        CA22_MANUAL_HEADLESS_TICKS
    );
    assert_eq!(summary.metrics.first_failure_tick, None);
    assert!(summary.metrics.ecology_metric_samples >= 10);
    assert!(summary.metrics.population_bounds_enforced);
    assert!(summary.metrics.resource_bounds_enforced);
    assert!(summary.metrics.no_unsealed_learning);
    assert!(!summary.full_emergent_ecology_claim);
    summary.validate().unwrap();
}

#[test]
fn onboarding_help_smoke_references_existing_controls_and_safe_commands() {
    let summary = run_onboarding_help_smoke().unwrap();

    assert_eq!(summary.schema, alife_game_app::G20_ONBOARDING_HELP_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G20_ONBOARDING_HELP_SCHEMA_VERSION
    );
    assert!(summary
        .controls
        .iter()
        .any(|control| control.label == "Pause"));
    assert!(summary
        .controls
        .iter()
        .any(|control| control.label == "Step"));
    assert!(summary
        .controls
        .iter()
        .any(|control| control.label == "Inspect"));
    assert!(summary
        .troubleshooting
        .iter()
        .any(|entry| entry.command.contains("scripts/check.ps1")));
    assert!(summary
        .troubleshooting
        .iter()
        .all(|entry| !entry.command.contains("bash scripts/check.sh")));
    assert!(summary.optional_systems_remain_optional);
    assert!(summary.windows_wrappers_documented);
    assert!(summary.tutorial_script_path.is_file());
    assert!(summary.docs_path.is_file());
    summary.validate().unwrap();
}

#[test]
fn tutorial_script_loads_food_hazard_sleep_inspection_flow() {
    let script = alife_game_app::load_g20_tutorial_script().unwrap();
    let step_ids = script
        .steps
        .iter()
        .map(|step| step.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(script.schema, alife_game_app::G20_TUTORIAL_SCRIPT_SCHEMA);
    assert_eq!(
        script.schema_version,
        alife_game_app::G20_TUTORIAL_SCRIPT_SCHEMA_VERSION
    );
    assert!(step_ids.contains(&"run-headless"));
    assert!(step_ids.contains(&"food-hazard-sleep"));
    assert!(step_ids.contains(&"inspect-readonly"));
    assert!(step_ids.contains(&"balance-report"));
    assert!(script
        .steps
        .iter()
        .all(|step| !step.command.contains("gpu-report")
            && !step.command.contains("ALIFE_GPU_BACKEND")
            && !step.command.contains("bash scripts/check.sh")));
    script.validate().unwrap();
}

#[test]
fn ca40_onboarding_tutorial_smoke_guides_first_gpu_alpha_session() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_onboarding_tutorial_smoke(&launch).unwrap();

    assert_eq!(summary.schema, CA40_ONBOARDING_TUTORIAL_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA40_ONBOARDING_TUTORIAL_SCHEMA_VERSION
    );
    assert!(summary.checklist.len() >= CA40_REQUIRED_CHECKLIST_ITEMS);
    assert!(summary
        .checklist
        .iter()
        .any(|item| item.id == "pause-run-step"));
    assert!(summary
        .checklist
        .iter()
        .any(|item| item.id == "read-food-hazard"));
    assert!(summary
        .checklist
        .iter()
        .any(|item| item.id == "read-gpu-fallback"));
    assert!(summary.tutorial_panel_text.contains("First Steps"));
    assert!(summary.tutorial_panel_text.contains("Space run/pause"));
    assert!(summary.tutorial_panel_text.contains("N step"));
    assert!(summary.tutorial_panel_text.contains("F follow"));
    assert!(summary.tutorial_panel_text.contains("[+] food"));
    assert!(summary.tutorial_panel_text.contains("[!] hazard"));
    assert!(summary.tutorial_panel_text.contains("GPU"));
    assert!(summary.tutorial_panel_text.contains("fallback"));
    assert!(summary.tutorial_panel_text.contains("CPU shadow gate"));
    assert!(summary.graphical_controls_verified);
    assert!(summary.has_food_marker);
    assert!(summary.has_hazard_marker);
    assert!(summary.stable_ids_only);
    assert!(summary.display_only);
    assert!(summary.no_action_authority);
    assert!(summary.no_weight_authority);
    assert!(summary.no_full_action_authoritative_claim);
    assert!(!summary.tutorial_panel_text.contains("Entity("));
    assert!(!summary
        .tutorial_panel_text
        .contains("full action-authoritative"));
    summary.validate().unwrap();
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca40_first_session_tutorial_panel_is_visible_and_bounded() {
    let launch =
        alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(gpu_alpha_fixture_root(), 5);
    let (mut app, _summary) =
        alife_game_app::bevy_shell::build_graphical_playground_preview_app_shell(&launch)
            .expect("CA40 graphical tutorial shell should build");
    app.update();

    let mut query = app.world_mut().query_filtered::<
        &bevy::prelude::Text,
        bevy::prelude::With<alife_game_app::bevy_shell::GraphicalOnboardingTutorialOverlay>,
    >();
    let text = query
        .iter(app.world())
        .next()
        .expect("CA40 tutorial panel should be spawned")
        .0
        .clone();
    assert!(text.contains("First Steps"));
    assert!(text.contains("Space run/pause"));
    assert!(text.contains("N step"));
    assert!(text.contains("F follow"));
    assert!(text.contains("[+] food"));
    assert!(text.contains("[!] hazard"));
    assert!(text.contains("GPU"));
    assert!(text.contains("fallback"));
    assert!(text.contains("CPU shadow gate"));
    assert!(text.contains("full_auth=false"));
    assert!(!text.contains("Entity("));
    assert!(!text.contains("full action-authoritative"));
}

#[test]
fn platform_package_smoke_validates_scripts_manifest_and_artifact_policy() {
    let summary = run_platform_package_smoke().unwrap();

    assert_eq!(summary.schema, G21_PLATFORM_PACKAGE_SCHEMA);
    assert_eq!(summary.schema_version, G21_PLATFORM_PACKAGE_SCHEMA_VERSION);
    assert_eq!(
        summary.output_directory,
        "target/artifacts/g21_local_package"
    );
    assert!(!summary.generated_artifacts_tracked);
    assert!(summary.windows_wrappers_used);
    assert!(!summary.release_publishing_attempted);
    assert!(summary.commands.iter().any(|command| {
        command.kind == PackageSmokeKind::Headless && !command.manual && !command.requires_graphics
    }));
    assert!(summary.commands.iter().any(|command| {
        command.kind == PackageSmokeKind::GraphicalManual
            && command.manual
            && command.requires_graphics
    }));
    assert!(summary.commands.iter().all(|command| {
        !command.windows_command.contains("bash scripts/check.sh")
            && !command
                .non_windows_command
                .contains("bash scripts/check.sh")
            && !command.windows_command.contains("gpu-report")
            && !command.windows_command.contains("ALIFE_GPU_BACKEND")
    }));
    assert!(summary.commands.iter().any(|command| command.id
        == "ca41-windows-alpha-package-dry-run"
        && command
            .windows_command
            .contains("scripts/package_windows_alpha.ps1 -DryRun")));
    assert!(summary.commands.iter().any(|command| command.id
        == "ca41-windows-alpha-package-runner-dry-run"
        && command
            .windows_command
            .contains("run_windows_alpha_package.ps1 -DryRun")));
    summary.validate().unwrap();
}

#[test]
fn ca41_windows_alpha_package_scripts_are_cargo_free_for_testers_and_artifact_safe() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let package_script =
        std::fs::read_to_string(root.join("scripts/package_windows_alpha.ps1")).unwrap();
    let runner_script =
        std::fs::read_to_string(root.join("scripts/run_windows_alpha_package.ps1")).unwrap();
    let status_doc = std::fs::read_to_string(
        root.join("docs/creatures_agi_roadmap_pack/status/CA41_WINDOWS_ZIP_PACKAGE.md"),
    )
    .unwrap();

    assert!(package_script.contains("cargo"));
    assert!(package_script.contains("--release"));
    assert!(package_script.contains("target/artifacts/ca41_windows_alpha"));
    assert!(package_script.contains("Compress-Archive"));
    assert!(package_script.contains("AllowedRootWithSeparator"));
    assert!(package_script.contains("PackageChildren"));
    assert!(package_script.contains("Select-Object -ExpandProperty FullName"));
    assert!(package_script.contains("package_metadata.json"));
    assert!(package_script.contains("working_tree_dirty"));
    assert!(package_script.contains("crates/alife_game_app/environment_manifest.json"));
    assert!(package_script.contains("crates/alife_game_app/app_bundle_manifest.json"));
    assert!(package_script.contains("crates/alife_gpu_backend/shaders"));
    assert!(package_script.contains("crates/alife_world/tests/fixtures/gpu_alpha"));
    assert!(package_script.contains("release_tag_created = $false"));
    assert!(!package_script.contains("git tag"));
    assert!(!package_script.contains("bash scripts/check.sh"));

    assert!(runner_script.contains("alife_game_app.exe"));
    assert!(runner_script.contains("graphical-playground"));
    assert!(runner_script.contains("--manifest"));
    assert!(runner_script.contains("--scenario"));
    assert!(runner_script.contains("--gpu-mode"));
    assert!(runner_script.contains("static-plastic-cpu-shadow-guarded"));
    assert!(runner_script.contains("CPU fallback is safety fallback"));
    assert!(runner_script.contains("Full action-authoritative GPU runtime claim: false"));
    assert!(!runner_script.contains("cargo run"));
    assert!(!runner_script.contains("bash scripts/check.sh"));
    assert!(!runner_script.contains("ALIFE_GPU_BACKEND"));

    assert!(status_doc.contains("CA41"));
    assert!(status_doc.contains("scripts/package_windows_alpha.ps1"));
    assert!(status_doc.contains("scripts/run_windows_alpha_package.ps1"));
    assert!(status_doc.contains("target/artifacts/ca41_windows_alpha"));
    assert!(status_doc.contains("No release tag"));
    assert!(status_doc.contains("CpuShadowGuardedStaticPlusLiveHShadow"));
    assert!(status_doc.contains("not full action-authoritative"));
}

#[test]
fn ca42_runtime_prereq_reports_cpu_mode_without_blocking() {
    let options = RuntimePrereqDiagnosticsOptions::new(
        GraphicalGpuRuntimeMode::CpuReference,
        false,
        "dx12",
        "target/artifacts/ca42_runtime_prereq/test_cpu.log",
    );
    let summary = run_runtime_prereq_diagnostics(&options).unwrap();

    assert_eq!(summary.schema, CA42_RUNTIME_PREREQ_SCHEMA);
    assert_eq!(summary.schema_version, CA42_RUNTIME_PREREQ_SCHEMA_VERSION);
    assert_eq!(
        summary.requested_gpu_mode,
        GraphicalGpuRuntimeMode::CpuReference
    );
    assert_eq!(summary.requested_backend, "CpuReference");
    assert_eq!(summary.selected_backend, "CpuReference");
    assert!(summary.fallback_reason.is_none());
    assert!(!summary.would_block_launch);
    assert!(summary.cpu_fallback_available);
    assert!(summary.missing_driver_guidance.contains("DirectX 12"));
    assert!(summary.missing_driver_guidance.contains("Vulkan"));
    assert!(summary.no_full_action_authoritative_claim);
    assert!(summary.cpu_shadow_gate_preserved);
    summary.validate().unwrap();
}

#[test]
fn ca42_runtime_prereq_forced_gpu_unavailable_is_clear_and_require_gpu_blocks() {
    let _guard = gpu_plasticity_env_lock();
    std::env::set_var("ALIFE_GPU_RUNTIME_AVAILABLE", "0");
    let options = RuntimePrereqDiagnosticsOptions::new(
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        false,
        "dx12",
        "target/artifacts/ca42_runtime_prereq/test_forced_fallback.log",
    );
    let summary = run_runtime_prereq_diagnostics(&options).unwrap();
    std::env::remove_var("ALIFE_GPU_RUNTIME_AVAILABLE");

    assert_eq!(
        summary.requested_gpu_mode,
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded
    );
    assert_eq!(summary.selected_backend, "CpuReference");
    assert!(summary.fallback_reason.is_some());
    assert!(!summary.would_block_launch);
    assert!(summary.cpu_fallback_degraded_visible);
    assert!(summary.cpu_shadow_gate_preserved);
    assert!(summary.no_full_action_authoritative_claim);
    summary.validate().unwrap();

    std::env::set_var("ALIFE_GPU_RUNTIME_AVAILABLE", "0");
    let require_options = RuntimePrereqDiagnosticsOptions::new(
        GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        true,
        "dx12",
        "target/artifacts/ca42_runtime_prereq/test_require_gpu.log",
    );
    let require_summary = run_runtime_prereq_diagnostics(&require_options).unwrap();
    std::env::remove_var("ALIFE_GPU_RUNTIME_AVAILABLE");

    assert!(require_summary.would_block_launch);
    assert!(require_summary.fallback_reason.is_some());
    assert!(require_summary
        .missing_driver_guidance
        .contains("-RequireGpu"));
    require_summary.validate().unwrap();
}

#[test]
fn ca42_launcher_scripts_run_preflight_and_keep_artifacts_untracked() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let repo_script =
        std::fs::read_to_string(root.join("scripts/run_graphical_playground.ps1")).unwrap();
    let package_runner =
        std::fs::read_to_string(root.join("scripts/run_windows_alpha_package.ps1")).unwrap();
    let platform_docs =
        std::fs::read_to_string(root.join("docs/playable_sim_spec/platform_packaging.md")).unwrap();

    for text in [&repo_script, &package_runner] {
        assert!(text.contains("runtime-prereq-smoke"));
        assert!(text.contains("--graphics-backend"));
        assert!(text.contains("--log"));
        assert!(text.contains("Runtime preflight log"));
        assert!(text.contains("PreflightExitCode"));
        assert!(text.contains("-RequireGpu") || text.contains("$RequireGpu"));
        assert!(!text.contains("bash scripts/check.sh"));
        assert!(!text.contains("Tee-Object -FilePath $PreflightLog"));
    }

    assert!(repo_script.contains("target/artifacts/ca42_runtime_prereq"));
    assert!(package_runner.contains("diagnostics/runtime_prereq.log"));
    assert!(platform_docs.contains("runtime-prereq-smoke"));
    assert!(platform_docs.contains("Runtime preflight log"));

    let summary = run_platform_package_smoke().unwrap();
    assert!(summary
        .commands
        .iter()
        .any(|command| command.id == "ca42-runtime-prereq-smoke"
            && command.windows_command.contains("runtime-prereq-smoke")));
    assert!(!summary.generated_artifacts_tracked);
}

#[test]
fn ca43_tester_feedback_capture_smoke_validates_policy_template_and_scripts() {
    let summary = run_tester_feedback_capture_smoke().unwrap();

    assert_eq!(summary.schema, CA43_TESTER_FEEDBACK_SCHEMA);
    assert_eq!(summary.schema_version, CA43_TESTER_FEEDBACK_SCHEMA_VERSION);
    assert!(summary
        .policy
        .repo_feedback_dir
        .starts_with("target/artifacts"));
    assert_eq!(
        summary.policy.package_feedback_dir,
        PathBuf::from("diagnostics/ca43_tester_feedback")
    );
    assert!(summary.policy.artifacts_must_remain_untracked);
    assert!(summary.policy.sanitize_paths_required);
    assert!(summary.launcher_script_wired);
    assert!(summary.package_script_wired);
    assert!(summary.docs_template_present);
    assert!(!summary.tracked_artifacts_present);
    assert!(summary.no_release_tag_claim);
    assert!(summary.no_core_dependency_change_required);
    assert!(summary.crash_summary.user_action_required);
    assert!(summary.crash_summary.commit_media_forbidden);
    assert!(summary
        .feedback_template
        .severity_labels
        .contains(&"BLOCKER"));
    summary
        .validate(&PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .unwrap();

    let package = run_platform_package_smoke().unwrap();
    assert!(package
        .commands
        .iter()
        .any(|command| command.id == "ca43-tester-feedback-smoke"
            && command.windows_command.contains("tester-feedback-smoke")
            && !command.manual
            && !command.requires_graphics
            && !command.requires_gpu));
}

#[test]
fn ca43_crash_summary_sanitizes_local_paths_and_forbids_committed_media() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let input = CrashSummaryInput::sample_for_workspace(&root);
    let summary = render_ca43_crash_summary(&input, &root);
    let markdown = summary.to_markdown();

    assert!(!summary
        .sanitized_command
        .contains(&root.display().to_string()));
    assert!(!summary
        .sanitized_log_path
        .contains(&root.display().to_string()));
    assert!(!summary
        .sanitized_stderr_tail
        .contains(&root.display().to_string()));
    assert!(summary.sanitized_command.contains("<local-path>"));
    assert!(markdown.contains("Commit media/log artifacts: `false`"));
    assert!(markdown.contains("User action required: `true`"));
    summary.validate(&root).unwrap();
}

#[test]
fn ca43_launcher_scripts_report_local_feedback_paths_without_tracking_artifacts() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let repo_script =
        std::fs::read_to_string(root.join("scripts/run_graphical_playground.ps1")).unwrap();
    let package_runner =
        std::fs::read_to_string(root.join("scripts/run_windows_alpha_package.ps1")).unwrap();
    let package_script =
        std::fs::read_to_string(root.join("scripts/package_windows_alpha.ps1")).unwrap();
    let status_doc = std::fs::read_to_string(
        root.join("docs/creatures_agi_roadmap_pack/status/CA43_CRASH_LOGS_TESTER_FEEDBACK.md"),
    )
    .unwrap();
    let template = std::fs::read_to_string(
        root.join("docs/creatures_agi_roadmap_pack/templates/CA43_TESTER_FEEDBACK_TEMPLATE.md"),
    )
    .unwrap();

    for text in [&repo_script, &package_runner] {
        assert!(text.contains("ca43_tester_feedback"));
        assert!(text.contains("crash_summary.md"));
        assert!(text.contains("tester_feedback_template.md"));
        assert!(text.contains("Write-Ca43CrashSummary"));
        assert!(text.contains("Convert-ToCa43SafeText"));
        assert!(text.contains("Commit media/log artifacts: false"));
        assert!(!text.contains("git tag"));
        assert!(!text.contains("bash scripts/check.sh"));
    }
    assert!(repo_script.contains("target/artifacts/ca43_tester_feedback"));
    assert!(package_runner.contains("diagnostics/ca43_tester_feedback"));
    assert!(package_script.contains("CA43_TESTER_FEEDBACK_TEMPLATE.md"));
    assert!(status_doc.contains("target/artifacts/ca43_tester_feedback"));
    assert!(status_doc.contains("diagnostics/ca43_tester_feedback"));
    assert!(status_doc.contains("Full action-authoritative GPU runtime is not claimed"));
    assert!(template.contains("A-Life Alpha Tester Feedback Template"));
    assert!(template.contains("Do not attach or commit screenshots"));
    assert!(template.contains("Release/tag recommendation: defer"));
    assert!(Ca43LogDirectoryPolicy::default().validate().is_ok());
}

#[test]
fn platform_asset_bundle_manifest_references_only_small_committed_fixtures() {
    let manifest = alife_game_app::load_g21_asset_bundle_manifest().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let validation = manifest.validate_with_root(&root).unwrap();

    assert_eq!(manifest.schema, G21_ASSET_BUNDLE_SCHEMA);
    assert_eq!(manifest.schema_version, G21_ASSET_BUNDLE_SCHEMA_VERSION);
    assert!(manifest.output_directory.starts_with("target/artifacts/"));
    assert_eq!(validation.entry_count, 6);
    assert_eq!(validation.required_count, 6);
    assert_eq!(validation.optional_count, 0);
    assert!(manifest
        .entries
        .iter()
        .all(|entry| entry.max_size_bytes <= 16_384));
}

#[test]
fn product_qa_smoke_aggregates_invalid_inputs_optional_gates_and_known_issues() {
    let summary = run_product_qa_hardening_smoke().unwrap();

    assert_eq!(summary.schema, alife_game_app::G22_PRODUCT_QA_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G22_PRODUCT_QA_SCHEMA_VERSION
    );
    assert_eq!(summary.release_blocker_count, 0);
    assert!(summary.known_limitation_count >= 3);
    assert!(summary.p36_gates_preserved);
    assert!(summary.no_p37_created);
    assert!(summary.no_generated_artifacts_tracked);
    assert!(summary.invalid_input.invalid_config_rejected);
    assert!(summary.invalid_input.invalid_save_schema_rejected);
    assert!(summary.invalid_input.missing_required_asset_rejected);
    assert!(summary.invalid_input.digest_mismatch_rejected);
    assert!(summary.invalid_input.invalid_app_state_transition_rejected);
    assert!(summary.invalid_input.stale_gpu_command_rejected);
    assert!(summary.invalid_input.no_partial_load_after_error);
    assert!(
        summary
            .optional_features
            .headless_default_has_no_graphics_requirement
    );
    assert!(summary.optional_features.semantic_absence_nonfatal);
    assert!(
        summary
            .optional_features
            .semantic_fake_provider_non_authoritative
    );
    assert!(
        summary
            .optional_features
            .school_verifier_uses_sealed_patches
    );
    assert!(summary.optional_features.gpu_default_falls_back_to_cpu);
    assert!(summary.optional_features.gpu_no_active_readback);
    assert!(summary.optional_features.graphical_smoke_manual);
    assert!(summary.ui_transitions.pause_resume_seen);
    assert!(summary.ui_transitions.save_load_menu_seen);
    assert!(summary.ui_transitions.cognition_debug_read_only);
    assert!(summary.ui_transitions.world_editor_resume_seen);
    assert!(summary.manual_gpu_command.contains("--gpu-runtime"));
    assert!(summary
        .extended_balance_command
        .contains("g19_manual_extended_balance_run"));
    summary.validate().unwrap();
}

#[test]
fn product_qa_checklist_covers_all_required_areas_without_stale_commands() {
    let summary = run_product_qa_hardening_smoke().unwrap();
    let areas = summary
        .checklist
        .iter()
        .map(|item| item.area)
        .collect::<std::collections::BTreeSet<_>>();

    for area in [
        ProductQaArea::AppLaunch,
        ProductQaArea::GameplayLoop,
        ProductQaArea::UiState,
        ProductQaArea::SaveLoad,
        ProductQaArea::School,
        ProductQaArea::Semantic,
        ProductQaArea::GpuFallback,
        ProductQaArea::Performance,
        ProductQaArea::Packaging,
        ProductQaArea::Docs,
    ] {
        assert!(areas.contains(&area), "missing QA area {}", area.label());
    }
    assert!(summary
        .checklist
        .iter()
        .all(|item| !item.command.contains("gpu-report")
            && !item.command.contains("ALIFE_GPU_BACKEND")
            && !item.command.contains("bash scripts/check.sh")));
    assert!(summary
        .checklist
        .iter()
        .any(|item| item.status == ProductQaStatus::Manual && item.manual));
    assert!(summary
        .findings
        .iter()
        .all(|finding| !finding.release_blocker));
}

#[test]
fn product_qa_docs_record_exact_manual_commands_and_no_hidden_blockers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let docs = std::fs::read_to_string(root.join("docs/playable_sim_spec/product_qa_hardening.md"))
        .unwrap();
    let known =
        std::fs::read_to_string(root.join("docs/playable_sim_spec/known_issues.md")).unwrap();

    for text in [&docs, &known] {
        assert!(!text.contains("bash scripts/check.sh"));
        assert!(!text.contains("gpu-report"));
        assert!(!text.contains("ALIFE_GPU_BACKEND"));
        assert!(text.contains("ALIFE_GPU_RUNTIME_BACKEND=static"));
        assert!(text.contains("--gpu-runtime"));
    }
    assert!(docs.contains("cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke"));
    assert!(docs.contains("cargo test -p alife_world --test headless_soak"));
    assert!(known.contains("None known after the G22 QA smoke"));
    assert!(known.contains("CPU fallback is not a GPU performance claim"));
}

#[test]
fn release_candidate_smoke_aggregates_playable_candidate_gates() {
    let summary = run_release_candidate_smoke().unwrap();
    let areas = summary
        .gates
        .iter()
        .map(|gate| gate.area)
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(summary.schema, alife_game_app::G23_RELEASE_CANDIDATE_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G23_RELEASE_CANDIDATE_SCHEMA_VERSION
    );
    assert_eq!(summary.candidate_id, "playable-sim-rc1");
    assert_eq!(summary.playable_supported_path, "headless-cpu-playground");
    assert_eq!(summary.release_blocker_count, 0);
    assert_eq!(summary.product_qa_release_blockers, 0);
    assert!(summary.known_limitation_count >= 1);
    assert!(summary.p36_gates_preserved);
    assert!(summary.no_p37_created);
    assert!(summary.no_generated_artifacts_tracked);
    assert!(!summary.release_tag_created);
    assert_eq!(
        summary.gpu_performance_status,
        "manual-unknown-unless-measured"
    );
    assert_eq!(summary.graphics_status, "manual-not-measured");
    assert!(summary.tag_proposal.contains("git tag -a playable-sim-rc1"));

    for area in [
        ReleaseCandidateArea::FullValidation,
        ReleaseCandidateArea::HeadlessPlayground,
        ReleaseCandidateArea::SaveLoad,
        ReleaseCandidateArea::Soak,
        ReleaseCandidateArea::Balance,
        ReleaseCandidateArea::ProductQa,
        ReleaseCandidateArea::Packaging,
        ReleaseCandidateArea::GpuManual,
        ReleaseCandidateArea::GraphicsManual,
        ReleaseCandidateArea::Docs,
    ] {
        assert!(areas.contains(&area), "missing G23 area {}", area.label());
    }
    assert!(summary
        .gates
        .iter()
        .any(|gate| gate.area == ReleaseCandidateArea::GpuManual
            && gate.status == ReleaseCandidateGateStatus::Manual
            && gate.command.contains("--gpu-runtime")));
    assert!(summary
        .gates
        .iter()
        .any(|gate| gate.area == ReleaseCandidateArea::GraphicsManual
            && gate.status == ReleaseCandidateGateStatus::Manual));
    assert!(summary.gates.iter().all(|gate| !gate.release_blocker
        && !gate.command.contains("gpu-report")
        && !gate.command.contains("ALIFE_GPU_BACKEND")
        && !gate.command.contains("bash scripts/check.sh")));
    summary.validate().unwrap();
}

#[test]
fn release_candidate_report_records_exact_commands_and_no_overclaims() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report = std::fs::read_to_string(root.join("docs/release_candidate.md")).unwrap();

    for required in [
        "cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke",
        "cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json",
        "cargo run -p alife_game_app --bin alife_game_app -- save-load-ux-smoke crates/alife_world/tests/fixtures/p34",
        "cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants",
        "cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke",
        "ALIFE_GPU_RUNTIME_BACKEND=static",
        "--gpu-runtime",
        "No release tag was created",
        "CPU fallback is not GPU performance",
    ] {
        assert!(report.contains(required), "missing report text: {required}");
    }
    assert!(!report.contains("gpu-report"));
    assert!(!report.contains("ALIFE_GPU_BACKEND"));
    assert!(!report.contains("bash scripts/check.sh"));
}

#[test]
#[ignore = "manual extended G19 balance run: cargo test -p alife_game_app --test app_shell g19_manual_extended_balance_run -- --ignored --nocapture"]
fn g19_manual_extended_balance_run() {
    let summary = run_longrun_balance_with_config(LongRunBalanceConfig::extended_manual()).unwrap();

    assert_eq!(
        summary.config.cycles,
        alife_game_app::G19_EXTENDED_BALANCE_CYCLES
    );
    assert!(summary.metrics.no_unsealed_learning);
    assert!(summary.metrics.population_bounds_enforced);
    assert!(summary.metrics.resource_bounds_enforced);
    summary.validate().unwrap();
    eprintln!("{}", summary.report_markdown);
}

#[test]
fn lifecycle_lineage_birth_creates_valid_offspring_genome() {
    let summary = run_lifecycle_lineage_smoke().unwrap();

    assert_eq!(summary.schema, alife_game_app::G09_LIFECYCLE_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G09_LIFECYCLE_SCHEMA_VERSION
    );
    assert_eq!(summary.metrics.births, 1);
    assert_eq!(summary.lineage_records.len(), 1);
    let birth = &summary.lineage_records[0];
    assert_eq!(birth.parent_genome_ids.len(), 2);
    assert!(summary.creatures.iter().any(|creature| creature.genome_id
        == birth.offspring_genome_id
        && creature.parent_genome_ids == birth.parent_genome_ids
        && creature.life_stage == CreatureLifeStage::Hatchling
        && creature.alive));
    summary.validate().unwrap();
}

#[test]
fn lifecycle_lineage_keeps_genetic_baseline_immutable_by_default() {
    let summary = run_lifecycle_lineage_smoke().unwrap();
    let offspring = summary
        .creatures
        .iter()
        .find(|creature| !creature.parent_genome_ids.is_empty())
        .expect("G09 smoke should create one offspring");

    assert_eq!(
        offspring.birth_weight_asset_id.as_deref(),
        Some("g09-tiny-birth-weight-asset")
    );
    assert!(!offspring.lamarckian_enabled);
    assert!(!offspring.inherited_lifetime_state);
    assert!(summary
        .lineage_records
        .iter()
        .all(|record| !record.lamarckian_enabled && !record.inherited_lifetime_state));
}

#[test]
fn lifecycle_lineage_death_cleanup_preserves_stable_selection() {
    let summary = run_lifecycle_lineage_smoke().unwrap();

    assert_eq!(summary.metrics.deaths, 1);
    assert!(summary
        .events
        .iter()
        .any(|event| event.kind == LifecycleEventKind::Death
            && event.message.contains("energy-failure")));
    let dead = summary
        .creatures
        .iter()
        .find(|creature| !creature.alive)
        .expect("G09 smoke should remove the low-energy elder");
    assert_eq!(dead.life_stage, CreatureLifeStage::Dead);
    assert_ne!(summary.selected_stable_id, Some(dead.stable_id));
    assert!(summary.selected_stable_id.is_some_and(|selected| summary
        .creatures
        .iter()
        .any(|creature| { creature.alive && creature.stable_id == selected })));
    assert!(!summary
        .world_signature
        .iter()
        .any(|line| line.contains("lineage-elder")));
}

#[test]
fn lifecycle_lineage_save_state_roundtrips_with_lineage_records() {
    let summary = run_lifecycle_lineage_smoke().unwrap();
    let save = LifecycleSaveState::from_summary(&summary).unwrap();
    let json = save.to_json_string_pretty().unwrap();
    let loaded = LifecycleSaveState::from_json_str(&json).unwrap();

    assert_eq!(save.signature_line(), loaded.signature_line());
    assert_eq!(loaded.lineages.len(), 1);
    assert_eq!(loaded.records.len(), summary.creatures.len());
    assert_eq!(loaded.selected_stable_id, summary.selected_stable_id);
    loaded.validate().unwrap();
}

#[test]
fn lifecycle_lineage_reproduction_cap_is_enforced() {
    let mut config = LifecycleLoopConfig::lineage_smoke().unwrap();
    config.population_cap = config.creatures.len() - 1;
    assert!(config.validate().is_err());

    let mut blocked = LifecycleLoopConfig::lineage_smoke().unwrap();
    blocked.population_cap = blocked.creatures.len();
    for creature in &mut blocked.creatures {
        creature.homeostasis.drives.brain_atp = 0.72;
        creature.homeostasis.drives.reproductive_drive = 0.76;
        creature.homeostasis.validate_contract().unwrap();
        creature.initial_age_ticks = Tick::new(5);
    }
    let mut live = LifecycleLiveLoop::from_config(blocked).unwrap();
    let summary = live.run_lifecycle_once().unwrap();
    assert_eq!(summary.metrics.births, 0);
    assert_eq!(summary.metrics.reproduction_blocked_count, 1);
    assert!(summary
        .events
        .iter()
        .any(|event| event.kind == LifecycleEventKind::ReproductionBlocked));
}

#[test]
fn school_mode_dispatches_teacher_cue_as_sensory_event() {
    let summary = run_school_mode_smoke().unwrap();

    assert_eq!(summary.schema, alife_game_app::G10_SCHOOL_MODE_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G10_SCHOOL_MODE_SCHEMA_VERSION
    );
    assert!(summary.sensory_heard_tokens.contains(&77));
    assert!(summary
        .sensory_teacher_channels
        .contains(&alife_core::TeacherPerceptionChannel::Hearing));
    assert!(summary.cues.iter().any(|cue| cue.token_id == Some(77)
        && cue.channel == alife_core::TeacherPerceptionChannel::Hearing
        && cue.perception_only
        && !cue.direct_motor_bypass));
    assert!(summary.cues.iter().any(|cue| cue.gesture_id == Some(24)
        && cue.channel == alife_core::TeacherPerceptionChannel::Gesture
        && cue.perception_only
        && !cue.direct_motor_bypass));
    assert!(summary
        .world_signature
        .iter()
        .any(|line| line.contains("teacher-word-food")));
    summary.validate().unwrap();
}

#[test]
fn ca24_teacher_world_cues_are_visible_audible_and_perception_only() {
    let summary = run_teacher_world_cues_smoke().unwrap();

    assert_eq!(
        summary.schema,
        alife_game_app::CA24_TEACHER_WORLD_CUES_SCHEMA
    );
    assert_eq!(
        summary.schema_version,
        alife_game_app::CA24_TEACHER_WORLD_CUES_SCHEMA_VERSION
    );
    assert_eq!(summary.teacher_avatar_stable_id, WorldEntityId(2));
    assert_eq!(summary.learner_stable_id, WorldEntityId(1));
    assert_eq!(summary.active_lesson_id, 10_100);
    assert!(summary.visible_world_events);
    assert!(summary.audible_token_events);
    assert!(summary.gesture_events);
    assert!(summary.lesson_conditions_are_sensory_environmental);
    assert!(summary.verifier_uses_sealed_patches);
    assert!(summary.direct_motor_bypass_rejected);
    assert!(summary.hidden_vector_injection_blocked);
    assert!(summary.no_action_authority);
    assert!(summary
        .cue_objects
        .iter()
        .all(|cue| cue.perception_only && !cue.direct_motor_bypass));
    assert!(summary
        .cue_objects
        .iter()
        .any(|cue| cue.compact_line().contains("speech token")));
    assert!(summary
        .cue_objects
        .iter()
        .any(|cue| cue.compact_line().contains("gesture marker")));
    let overlay = summary.compact_overlay_text();
    assert!(overlay.contains("Teacher Cues"));
    assert!(overlay.contains("Speech token: true"));
    assert!(overlay.contains("Gesture: true"));
    assert!(overlay.contains("sensory/environmental"));
    assert!(overlay.contains("no direct motor bypass"));
    assert!(!overlay.contains("Entity("));
    assert!(!overlay.contains("action-authoritative"));
    summary.validate().unwrap();
}

#[test]
fn ca25_curriculum_authoring_validates_manifest_progress_and_save_state() {
    let summary = run_curriculum_authoring_smoke().unwrap();

    assert_eq!(summary.schema, CA25_CURRICULUM_AUTHORING_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA25_CURRICULUM_AUTHORING_SCHEMA_VERSION
    );
    assert_eq!(summary.curriculum_id, "ca25-grounded-food-token");
    assert_eq!(summary.lesson_count, 1);
    assert_eq!(summary.active_lesson_id, 10_100);
    assert!(summary.verifier_uses_sealed_patches);
    assert!(summary.verifier_passed);
    assert_eq!(summary.completed_lesson_ids, vec![10_100]);
    assert!(summary
        .verifier_condition_labels
        .iter()
        .any(|label| label.contains("heard_token:77")));
    assert!(summary.progress_display.contains("Progress: 1/1"));
    assert!(summary.editor_panel_text.contains("validator-only JSON"));
    assert!(summary
        .editor_panel_text
        .contains("Boundary: perception-only"));
    assert!(!summary.model_inference_required);
    assert!(!summary.fake_model_output_used);
    assert!(!summary.can_issue_actions);
    assert!(!summary.can_rewrite_weights);
    assert!(!summary.progress_display.contains("Entity("));
    assert!(!summary.editor_panel_text.contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn ca25_lesson_manifest_rejects_fake_model_and_invalid_verifier_shape() {
    let mut manifest = LessonManifest::from_json_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/ca25/lesson_manifest.json"),
    )
    .unwrap();
    manifest.lessons[0].verifier_conditions.clear();
    let json = serde_json::to_string(&manifest).unwrap();
    assert!(LessonManifest::from_json_str(&json)
        .unwrap_err()
        .to_string()
        .contains("lesson entry"));

    let save = CurriculumLessonSaveState {
        schema: CA25_CURRICULUM_AUTHORING_SCHEMA.to_string(),
        schema_version: CA25_CURRICULUM_AUTHORING_SCHEMA_VERSION,
        curriculum_id: "ca25-grounded-food-token".to_string(),
        active_lesson_id: 10_100,
        completed_lesson_ids: vec![10_100],
        verifier_passed: true,
        editor_dirty: false,
        teacher_private_state_saved: false,
        model_inference_saved: false,
    };
    let loaded =
        CurriculumLessonSaveState::from_json_str(&save.to_json_string_pretty().unwrap()).unwrap();
    assert_eq!(save.signature_line(), loaded.signature_line());

    let mut bad = loaded;
    bad.model_inference_saved = true;
    assert!(bad.validate().is_err());
}

#[test]
fn school_mode_teacher_metadata_does_not_bypass_arbitration() {
    let summary = run_school_mode_smoke().unwrap();

    assert!(summary.teacher_metadata_bypass_blocked);
    assert_eq!(summary.teacher_selected_action_id, None);
    assert!(summary
        .cues
        .iter()
        .all(|cue| cue.perception_only && !cue.direct_motor_bypass));
}

#[test]
fn school_mode_verifier_uses_sealed_patches_and_progresses_lesson() {
    let summary = run_school_mode_smoke().unwrap();

    assert!(summary.verifier_panel.passed);
    assert_eq!(summary.verifier_panel.sealed_patch_count, 1);
    assert_eq!(summary.lesson_panel.completed_steps, 1);
    assert_eq!(summary.lesson_panel.total_steps, 1);
    assert!(summary
        .verifier_panel
        .observed_checks
        .iter()
        .any(|check| check.contains("HeardToken")));
    assert!(summary.verifier_panel.failed_checks.is_empty());
}

#[test]
fn school_mode_save_state_roundtrips_without_teacher_private_state() {
    let summary = run_school_mode_smoke().unwrap();
    let save = SchoolModeSaveState::from_summary(&summary).unwrap();
    let json = save.to_json_string_pretty().unwrap();
    let loaded = SchoolModeSaveState::from_json_str(&json).unwrap();

    assert_eq!(save.signature_line(), loaded.signature_line());
    assert!(loaded.p34_school.enabled);
    assert_eq!(
        loaded.p34_school.active_curriculum_id.as_deref(),
        Some("g10-grounded-object-food")
    );
    assert!(!loaded.p34_school.teacher_private_state_saved);
    assert_eq!(
        loaded.teacher_avatar_stable_id,
        summary.teacher_avatar_stable_id
    );
    assert!(!loaded.cue_entity_ids.is_empty());
    loaded.validate().unwrap();
}

#[test]
fn ca23_graphical_school_mode_smoke_reports_toggle_teacher_lesson_and_verifier_boundary() {
    let summary = run_graphical_school_mode_smoke().unwrap();

    assert_eq!(summary.schema, CA23_GRAPHICAL_SCHOOL_SCHEMA);
    assert_eq!(summary.schema_version, CA23_GRAPHICAL_SCHOOL_SCHEMA_VERSION);
    assert!(summary.school_enabled);
    assert_eq!(summary.toggle_key, "T");
    assert_eq!(summary.teacher_avatar_stable_id, WorldEntityId(2));
    assert_eq!(summary.learner_stable_id, WorldEntityId(1));
    assert_eq!(summary.active_lesson_id, 10_100);
    assert!(summary.verifier_uses_sealed_patches);
    assert!(summary.verifier_passed);
    assert_eq!(summary.sealed_patch_count, 1);
    assert!(!summary.cue_markers.is_empty());
    assert!(summary
        .cue_markers
        .iter()
        .all(|marker| marker.perception_only));
    assert!(summary.perception_only_boundary_visible);
    assert!(summary.direct_motor_bypass_blocked);
    assert!(summary.hidden_vector_injection_blocked);
    assert!(summary.display_only);

    let overlay = summary.compact_overlay_text();
    assert!(overlay.contains("School Mode: on"));
    assert!(overlay.contains("[T toggle]"));
    assert!(overlay.contains("Teacher: stable:2"));
    assert!(overlay.contains("Verifier: sealed patches=1 pass=true"));
    assert!(overlay.contains("Boundary: perception-only"));
    assert!(!overlay.contains("Entity("));

    let mut disabled = summary.clone();
    disabled.toggle_school_enabled();
    assert!(!disabled.school_enabled);
    let disabled_overlay = disabled.compact_overlay_text();
    assert!(disabled_overlay.contains("School Mode: off"));
    assert!(disabled_overlay.contains("[T toggle]"));
    assert!(disabled_overlay.contains("Teacher cues hidden"));
    assert!(disabled_overlay.contains("sealed-patch"));
    assert!(disabled_overlay.contains("no motor bypass"));
    assert!(!disabled_overlay.contains("Entity("));

    summary.validate().unwrap();
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca23_school_overlay_is_toggleable_and_perception_only() {
    let mut summary = run_graphical_school_mode_smoke().unwrap();
    let expanded = alife_game_app::bevy_shell::ca23_school_overlay_text(&summary);
    assert!(expanded.contains("School Mode: on"));
    assert!(expanded.contains("teacher stable:2") || expanded.contains("Teacher: stable:2"));
    assert!(expanded.contains("via Hearing"));
    assert!(expanded.contains("via Gesture"));
    assert!(expanded.contains("sealed patches=1"));
    assert!(expanded.contains("teacher cues cannot emit actions"));
    assert!(expanded.contains("no hidden vectors"));
    assert!(!expanded.contains("Entity("));
    assert!(!expanded.contains("full action-authoritative"));

    summary.toggle_school_enabled();
    let disabled = alife_game_app::bevy_shell::ca23_school_overlay_text(&summary);
    assert!(disabled.contains("School Mode: off"));
    assert!(disabled.contains("Teacher cues hidden"));
    assert!(disabled.contains("teacher cues cannot emit actions"));
    assert!(!disabled.contains("Entity("));
}

#[test]
fn semantic_provider_disabled_path_is_safe_and_nonfatal() {
    let summary = run_semantic_provider_smoke().unwrap();

    assert_eq!(
        summary.schema,
        alife_game_app::G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA
    );
    assert_eq!(
        summary.schema_version,
        alife_game_app::G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA_VERSION
    );
    assert_eq!(summary.disabled_panel.config.provider_id, "disabled");
    assert!(!summary.disabled_panel.context_visible);
    assert!(summary.provider_absence_nonfatal);
    assert!(summary.disabled_panel.display_lines.is_empty());
    summary.validate().unwrap();
}

#[test]
fn semantic_provider_fake_context_is_visible_bounded_and_optional() {
    let summary = run_semantic_provider_smoke().unwrap();
    let fake = &summary.fake_panel;

    assert_eq!(fake.config.provider_id, "fake-local-table");
    assert!(fake.manifest.available);
    assert!(fake.context_visible);
    assert!(fake.display_lines.len() <= fake.config.max_display_entries);
    assert!(fake.semantic_code_count > 0);
    assert!(fake.concept_binding_count > 0);
    assert!(fake.gaussian_cluster_count > 0);
    assert!(fake
        .display_lines
        .iter()
        .any(|line| line.source == "semantic-concept"));
    assert!(fake.extension_note.contains("extension point"));
}

#[test]
fn semantic_provider_cannot_issue_actions_or_mutate_weights() {
    let summary = run_semantic_provider_smoke().unwrap();

    assert!(summary.semantic_action_bypass_blocked);
    assert!(summary.weight_rewrite_blocked);
    assert!(!summary.fake_panel.manifest.can_issue_actions);
    assert!(!summary.fake_panel.manifest.can_rewrite_weights);
    assert!(!summary.disabled_panel.manifest.can_issue_actions);
    assert!(!summary.disabled_panel.manifest.can_rewrite_weights);
}

#[test]
fn semantic_provider_config_rejects_unknown_schema_and_kind() {
    let summary = run_semantic_provider_smoke().unwrap();

    assert!(summary.unknown_schema_rejected);
    assert!(summary.unknown_provider_kind_rejected);
}

#[test]
fn ca26_local_model_manifest_is_real_local_only_and_bounded() {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/model_manifests/local_semantic_models.json");
    let manifest = LocalSemanticModelManifest::from_json_file(&manifest_path).unwrap();
    manifest.validate().unwrap();
    assert_eq!(manifest.schema, CA26_LOCAL_MODEL_MANIFEST_SCHEMA);
    assert_eq!(
        manifest.schema_version,
        CA26_LOCAL_MODEL_MANIFEST_SCHEMA_VERSION
    );

    let model = manifest.semantic_embedding_model().unwrap();
    assert_eq!(model.repo_id, "Qwen/Qwen3-Embedding-0.6B-GGUF");
    assert_eq!(model.model_role, "semantic_embedding_provider");
    assert_eq!(model.license, "apache-2.0");
    assert_eq!(model.runtime_backend, "llamacpp-server-gguf");
    assert_eq!(model.llamacpp_alias, "alife-qwen3-embedding-0.6b");
    assert_eq!(model.llamacpp_host, "127.0.0.1");
    assert_eq!(model.llamacpp_port, CA26_DEFAULT_LLAMA_CPP_EMBEDDING_PORT);
    assert_eq!(model.sha256.len(), 64);
    assert!(model.downloaded_locally);
    assert!(model.inference_smoke_passed);
    assert!(model
        .limitations
        .iter()
        .any(|limitation| limitation.contains("perception-context-only")));
    assert!(!model.runtime_backend.contains("cloud"));
    assert!(!model.expected_local_path.contains("Entity("));
}

#[test]
fn ca26_local_embedding_projection_bounds_context_and_blocks_authority() {
    let raw = (0..1024)
        .map(|index| ((index % 31) as f32 / 31.0) - 0.5)
        .collect::<Vec<_>>();
    let projected = project_embedding_to_i8(&raw).unwrap();
    assert_eq!(projected.len(), CA26_EMBEDDING_PROJECTION_DIMS);
    assert!(projected.iter().any(|value| *value != 0));

    let capability = SemanticProviderCapabilityManifest::local_llamacpp_embedding(
        CA26_LOCAL_SEMANTIC_PROVIDER_ID,
        true,
    );
    capability.validate().unwrap();
    assert!(capability.available);
    assert!(capability.bounded_context);
    assert!(!capability.can_issue_actions);
    assert!(!capability.can_rewrite_weights);
}

#[test]
fn ca26_unavailable_local_model_is_user_action_required_not_fake_output() {
    let provider = LlamaCppEmbeddingProvider::new(LlamaCppEmbeddingConfig {
        port: 9,
        timeout_ms: 1_000,
        ..LlamaCppEmbeddingConfig::default()
    })
    .unwrap();
    let err = provider.embed_text("teacher token food").unwrap_err();
    assert!(err.contains("USER_ACTION_REQUIRED"));
    assert!(!err.contains("fake"));
}

#[test]
#[ignore = "manual CA26 real local llama.cpp smoke: cargo test -p alife_game_app --test app_shell ca26_real_local_semantic_provider_smoke -- --ignored --nocapture"]
fn ca26_real_local_semantic_provider_smoke() {
    let summary = run_real_semantic_provider_smoke().unwrap();
    assert_eq!(summary.schema, CA26_REAL_SEMANTIC_PROVIDER_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA26_REAL_SEMANTIC_PROVIDER_SCHEMA_VERSION
    );
    assert_eq!(summary.repo_id, "Qwen/Qwen3-Embedding-0.6B-GGUF");
    assert_eq!(summary.runtime_backend, "llamacpp-server-gguf");
    assert_eq!(summary.llamacpp_port, CA26_DEFAULT_LLAMA_CPP_EMBEDDING_PORT);
    assert!(summary.downloaded_locally);
    assert!(summary.inference_smoke_passed);
    assert!(summary.raw_embedding_dims > 0);
    assert_eq!(
        summary.projected_embedding_dims,
        CA26_EMBEDDING_PROJECTION_DIMS
    );
    assert!(summary.context_vectors_bounded);
    assert!(!summary.fake_model_output_used);
    assert!(!summary.can_issue_actions);
    assert!(!summary.can_rewrite_weights);
    assert!(!summary.hidden_vector_injection);
    summary.validate().unwrap();
}

#[test]
fn ca27_local_slm_manifest_is_real_local_only_and_bounded() {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/model_manifests/local_semantic_models.json");
    let manifest = LocalSemanticModelManifest::from_json_file(&manifest_path).unwrap();
    manifest.validate().unwrap();

    let model = manifest.slm_subconscious_prior_model().unwrap();
    assert_eq!(
        model.target_repo_id.as_deref(),
        Some("Qwen/Qwen3-4B-Instruct-2507")
    );
    assert_eq!(model.repo_id, "Qwen/Qwen3-4B-GGUF");
    assert_eq!(model.model_role, "slm_subconscious_prior");
    assert_eq!(model.license, "apache-2.0");
    assert_eq!(model.runtime_backend, "llamacpp-server-gguf");
    assert_eq!(model.llamacpp_alias, CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS);
    assert_eq!(model.llamacpp_host, "127.0.0.1");
    assert_eq!(model.llamacpp_port, CA27_DEFAULT_LLAMA_CPP_SLM_PORT);
    assert_eq!(model.sha256.len(), 64);
    assert!(model.downloaded_locally);
    assert!(model.inference_smoke_passed);
    assert!(model
        .limitations
        .iter()
        .any(|limitation| limitation.contains("perception-context-only")));
    assert!(model.limitations.iter().any(|limitation| limitation
        .contains("no selected GGUF artifact")
        && limitation.contains("Qwen/Qwen3-4B-GGUF")));
    assert!(!model.runtime_backend.contains("cloud"));
    assert!(!model.expected_local_path.contains("Entity("));
}

#[test]
fn ca27_slm_prior_parser_queue_and_boundaries_block_authority() {
    let output = parse_slm_prior_json(
        CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS,
        r#"{
            "salience_labels":["food","hazard"],
            "context_summary":"Creature sees food near a hazard.",
            "lexicon_associations":{"food":0.95,"hazard":0.82},
            "perception_tags":["near","sees"]
        }"#,
    )
    .unwrap();

    assert_eq!(output.schema, CA27_SLM_PRIOR_OUTPUT_SCHEMA);
    assert_eq!(output.schema_version, CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION);
    assert!(!output.can_issue_actions);
    assert!(!output.can_rewrite_weights);
    assert!(!output.can_bypass_arbitration);
    assert!(!output.hidden_vector_injection);
    assert!(output.bounded_context_only);

    let config = LlamaCppSlmPriorConfig {
        max_queue_depth: 1,
        ..LlamaCppSlmPriorConfig::default()
    };
    let mut queue = LocalSlmPriorQueue::new(config).unwrap();
    queue
        .enqueue(LocalSlmPriorRequest {
            request_id: 1,
            prompt: "teacher token food".to_string(),
        })
        .unwrap();
    assert!(queue
        .enqueue(LocalSlmPriorRequest {
            request_id: 2,
            prompt: "teacher token hazard".to_string(),
        })
        .is_err());
}

#[test]
fn ca27_slm_prior_malformed_output_and_unavailable_model_reject() {
    assert!(parse_slm_prior_json(
        CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS,
        r#"{
            "salience_labels":["food"],
            "context_summary":"Creature sees food.",
            "lexicon_associations":{"food":0.9},
            "perception_tags":["near"],
            "action":"eat"
        }"#,
    )
    .is_err());

    let provider = LlamaCppSlmPriorProvider::new(LlamaCppSlmPriorConfig {
        port: 9,
        timeout_ms: 1_000,
        ..LlamaCppSlmPriorConfig::default()
    })
    .unwrap();
    let err = provider.generate_prior("teacher token food").unwrap_err();
    assert!(err.contains("USER_ACTION_REQUIRED"));
    assert!(!err.contains("fake"));
}

#[test]
fn ca27_slm_prior_async_queue_reports_unavailable_without_fake_output() {
    let queue = LocalSlmPriorAsyncQueue::new(LlamaCppSlmPriorConfig {
        port: 9,
        timeout_ms: 1_000,
        ..LlamaCppSlmPriorConfig::default()
    })
    .unwrap();
    assert_eq!(queue.capacity(), 4);
    assert_eq!(queue.timeout_ms(), 1_000);
    let result = queue
        .submit(LocalSlmPriorRequest {
            request_id: 1,
            prompt: "teacher token food".to_string(),
        })
        .unwrap();
    let err = queue.wait_for(result).unwrap_err();
    assert!(err.contains("USER_ACTION_REQUIRED"));
    assert!(!err.contains("fake"));
}

#[test]
#[ignore = "manual CA27 real local llama.cpp smoke: cargo test -p alife_game_app --test app_shell ca27_real_local_slm_prior_smoke -- --ignored --nocapture"]
fn ca27_real_local_slm_prior_smoke() {
    let summary = run_internal_slm_prior_smoke().unwrap();
    assert_eq!(summary.schema, CA27_INTERNAL_SLM_PRIOR_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA27_INTERNAL_SLM_PRIOR_SCHEMA_VERSION
    );
    assert_eq!(summary.target_repo_id, "Qwen/Qwen3-4B-Instruct-2507");
    assert_eq!(summary.repo_id, "Qwen/Qwen3-4B-GGUF");
    assert_eq!(summary.model_role, "slm_subconscious_prior");
    assert_eq!(summary.runtime_backend, "llamacpp-server-gguf");
    assert_eq!(summary.llamacpp_alias, CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS);
    assert_eq!(summary.llamacpp_port, CA27_DEFAULT_LLAMA_CPP_SLM_PORT);
    assert!(summary.downloaded_locally);
    assert!(summary.inference_smoke_passed);
    assert!(summary.salience_label_count > 0);
    assert!(summary.lexicon_association_count > 0);
    assert!(summary.perception_tag_count > 0);
    assert!(!summary.can_issue_actions);
    assert!(!summary.can_rewrite_weights);
    assert!(!summary.can_bypass_arbitration);
    assert!(!summary.hidden_vector_injection);
    assert!(summary.malformed_output_rejected);
    assert!(summary.unavailable_is_user_action_required);
    summary.validate().unwrap();
}

#[test]
fn ca28_topological_concept_overlay_shows_read_only_nodes_edges_and_events() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_topological_concept_overlay_smoke(&launch).unwrap();

    assert_eq!(
        summary.snapshot.schema,
        CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA
    );
    assert_eq!(
        summary.snapshot.schema_version,
        CA28_TOPOLOGICAL_CONCEPT_OVERLAY_SCHEMA_VERSION
    );
    assert!(summary.snapshot.concept_count >= 1);
    assert!(summary.snapshot.edge_count >= 1);
    assert!(!summary.snapshot.nodes.is_empty());
    assert!(!summary.snapshot.edges.is_empty());
    assert!(!summary.snapshot.event_links.is_empty());
    assert!(summary.panel_text.contains("Concept Map (read-only)"));
    assert!(summary
        .panel_text
        .contains("Boundary: bias/context only; no actions"));
    assert!(summary.status_text.contains("Concepts:"));
    assert!(!summary.panel_text.contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn ca28_topological_concept_overlay_cannot_emit_actions_or_mutate_cognition() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_topological_concept_overlay_smoke(&launch).unwrap();

    assert!(summary.snapshot.read_only);
    assert!(summary.snapshot.bias_only);
    assert!(!summary.snapshot.can_emit_actions);
    assert!(!summary.snapshot.direct_cognition_mutation_allowed);
    assert!(summary.topology_action_bypass_blocked);
    assert!(!summary.direct_cognition_mutation_allowed);
    assert!(summary.panel_text.contains("event tick="));
    assert!(!summary.panel_text.contains("full action-authoritative"));
}

#[test]
fn ca29_memory_history_journal_shows_recent_patches_and_biases() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_memory_history_journal_smoke(&launch).unwrap();

    assert_eq!(summary.snapshot.schema, CA29_MEMORY_HISTORY_JOURNAL_SCHEMA);
    assert_eq!(
        summary.snapshot.schema_version,
        CA29_MEMORY_HISTORY_JOURNAL_SCHEMA_VERSION
    );
    assert!(summary.snapshot.memory_record_count >= 1);
    assert!(!summary.snapshot.recent_patches.is_empty());
    assert!(!summary.snapshot.recent_memories.is_empty());
    assert!(!summary.snapshot.expectancy_rows.is_empty());
    assert!(summary.panel_text.contains("Memory Journal (read-only)"));
    assert!(summary.panel_text.contains("patch tick="));
    assert!(summary.panel_text.contains("bias from m"));
    assert!(summary.status_text.contains("Memory:"));
    assert!(!summary.panel_text.contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn ca29_memory_history_journal_cannot_replay_actions_or_mutate_cognition() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_memory_history_journal_smoke(&launch).unwrap();

    assert!(summary.snapshot.read_only);
    assert!(summary.snapshot.expectancy_bias_only);
    assert!(summary.snapshot.save_load_visible);
    assert!(!summary.snapshot.can_replay_actions);
    assert!(!summary.snapshot.can_emit_actions);
    assert!(!summary.snapshot.direct_cognition_mutation_allowed);
    assert!(summary.action_replay_blocked);
    assert!(!summary.direct_cognition_mutation_allowed);
    assert!(summary
        .panel_text
        .contains("Boundary: expectancy bias only; no action replay"));
    assert!(summary
        .panel_text
        .contains("Save/load: stable memory IDs visible"));
    assert!(!summary.panel_text.contains("full action-authoritative"));
}

#[test]
fn ca30_neural_activity_profiler_shows_lobes_tiles_and_route_status() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_neural_activity_profiler_smoke(&launch).unwrap();

    assert_eq!(
        summary.snapshot.schema,
        CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA
    );
    assert_eq!(
        summary.snapshot.schema_version,
        CA30_NEURAL_ACTIVITY_PROFILER_SCHEMA_VERSION
    );
    assert!(summary.snapshot.neuron_count >= 512);
    assert!(!summary.snapshot.lobe_rows.is_empty());
    assert!(summary.snapshot.tile_summary.max_active_tiles > 0);
    assert!(summary.snapshot.tile_summary.max_active_synapses > 0);
    assert!(summary.snapshot.route_status.cpu_shadow_gate);
    assert!(summary.panel_text.contains("Neural Profiler (compact)"));
    assert!(summary.panel_text.contains("tiles "));
    assert!(summary.panel_text.contains("route "));
    assert!(summary.status_text.contains("Neural:"));
    assert!(!summary.panel_text.contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn ca30_neural_activity_profiler_blocks_bulk_readback_and_action_authority() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_neural_activity_profiler_smoke(&launch).unwrap();

    assert!(summary.snapshot.read_only);
    assert!(summary.snapshot.compact_summary_only);
    assert!(summary.snapshot.offline_export_boundary);
    assert!(!summary.snapshot.active_bulk_readback_allowed);
    assert!(!summary.snapshot.can_emit_actions);
    assert!(!summary.snapshot.can_mutate_weights);
    assert!(
        !summary
            .snapshot
            .route_status
            .full_action_authoritative_claim
    );
    assert!(summary.bulk_readback_blocked);
    assert!(summary.action_authority_blocked);
    assert!(summary.weight_mutation_blocked);
    assert!(summary
        .panel_text
        .contains("Boundary: compact summary; offline export only"));
    assert!(!summary.panel_text.contains("full action-authoritative"));
}

#[test]
fn ca32_realtime_wgsl_telemetry_exposes_timing_split_and_routing_counters() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_realtime_wgsl_telemetry_smoke(&launch, 3).unwrap();

    assert_eq!(summary.schema, CA32_REALTIME_WGSL_TELEMETRY_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA32_REALTIME_WGSL_TELEMETRY_SCHEMA_VERSION
    );
    assert_eq!(summary.requested_ticks, 3);
    assert_eq!(summary.ticks_completed, 3);
    assert!(summary.cpu_shadow_gate);
    assert!(summary.no_active_bulk_readback);
    assert!(!summary.full_action_authoritative_claim);
    assert!(summary.telemetry.nonblocking_hot_path);
    assert!(summary.ui_summary.contains("WGSL:"));
    assert!(summary.ui_summary.contains("compute="));
    assert!(summary.ui_summary.contains("tiles"));
    assert!(!summary.ui_summary.contains("Entity("));
    assert!(!summary.ui_summary.contains("full action-authoritative"));
    if summary.telemetry.timing_available {
        assert!(summary.telemetry.routing_total_tiles > 0);
        assert!(summary.telemetry.routing_active_tiles <= summary.telemetry.routing_total_tiles);
        assert!(summary.telemetry.compact_readback_bytes > 0);
    } else {
        assert!(
            summary.fallback_reason.is_some() || summary.telemetry.unavailable_reason.is_some(),
            "unavailable WGSL telemetry must report an explicit fallback reason"
        );
    }
    summary.validate().unwrap();
}

#[test]
fn ca33_batched_gpu_runtime_uses_stable_id_population_and_honest_claims() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_batched_gpu_runtime_smoke(
        &launch,
        BatchedGpuRuntimeOptions {
            max_creatures: 3,
            ticks: 1,
            cpu_shadow_every: 1,
            json_path: None,
        },
    )
    .unwrap();

    assert_eq!(summary.schema, CA33_BATCHED_GPU_RUNTIME_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA33_BATCHED_GPU_RUNTIME_SCHEMA_VERSION
    );
    assert_eq!(summary.batch_size, 3);
    assert_eq!(summary.per_creature.len(), 3);
    assert!(summary.shared_gpu_session);
    assert!(summary.cpu_shadow_checked_every_tick);
    assert!(summary.sampled_cpu_shadow_deferred_to_ca34);
    assert!(summary.no_active_bulk_readback);
    assert!(summary.stable_id_only);
    assert!(!summary.full_action_authoritative_claim);
    assert_ne!(summary.product_runtime_claim, "FullActionAuthoritative");
    assert_eq!(
        summary
            .per_creature
            .iter()
            .map(|creature| creature.stable_id)
            .collect::<Vec<_>>(),
        vec![WorldEntityId(1), WorldEntityId(5), WorldEntityId(6)]
    );
    if summary.selected_backend != "CpuReference" {
        assert!(summary.gpu_static_dispatched_creatures >= summary.batch_size as u32);
        assert_eq!(summary.parity_failures, 0);
        assert!(summary.cpu_shadow_parity_checks >= summary.batch_size as u32);
        assert!(summary.compact_readback_bytes >= summary.batch_size * 64);
        assert!(
            summary.product_runtime_claim == "CpuShadowGuarded"
                || summary.product_runtime_claim == "CpuShadowGuardedStaticPlusLiveHShadow"
        );
    } else {
        assert!(summary.fallback_reason.is_some());
        assert_eq!(summary.gpu_proposal_creatures, 0);
        assert_eq!(summary.product_runtime_claim, "None");
    }
    summary.validate().unwrap();
}

#[test]
fn ca33_batched_gpu_runtime_rejects_sampled_shadow_until_ca34() {
    let options = BatchedGpuRuntimeOptions {
        cpu_shadow_every: 2,
        ..Default::default()
    };
    assert!(options.validate().is_err());
}

#[test]
fn ca34_sampled_gpu_runtime_reports_sampled_policy_without_overclaim() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = run_sampled_gpu_runtime_smoke(
        &launch,
        SampledGpuRuntimeOptions {
            max_creatures: 3,
            ticks: 4,
            warmup_ticks: 1,
            cpu_shadow_every: 2,
            json_path: None,
        },
    )
    .unwrap();

    assert_eq!(summary.schema, CA34_SAMPLED_GPU_RUNTIME_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA34_SAMPLED_GPU_RUNTIME_SCHEMA_VERSION
    );
    assert_eq!(summary.batch_size, 3);
    assert_eq!(summary.ticks_run, 4);
    assert_eq!(summary.warmup_ticks, 1);
    assert_eq!(summary.cpu_shadow_every, 2);
    assert!(summary.sampled_cpu_shadow_enabled);
    assert!(summary.fallback_on_first_failure);
    assert!(summary.shared_gpu_session);
    assert!(summary.no_active_bulk_readback);
    assert!(summary.stable_id_only);
    assert!(!summary.full_action_authoritative_claim);
    assert_ne!(summary.product_runtime_claim, "FullActionAuthoritative");
    assert_ne!(summary.product_runtime_claim, "ActionAuthoritative");
    assert_eq!(summary.per_creature.len(), 3);
    assert_eq!(
        summary
            .per_creature
            .iter()
            .map(|creature| creature.stable_id)
            .collect::<Vec<_>>(),
        vec![WorldEntityId(1), WorldEntityId(5), WorldEntityId(6)]
    );
    if summary.selected_backend != "CpuReference" {
        assert!(summary.cpu_shadow_checks > 0);
        assert!(summary.cpu_shadow_skipped_creatures > 0);
        assert!(summary.gpu_static_dispatched_creatures >= summary.batch_size as u32);
        assert_eq!(summary.parity_failures, 0);
        assert!(!summary.forced_cpu_after_failure);
        assert!(summary
            .product_runtime_claim
            .starts_with("SampledCpuShadow"));
    } else {
        assert!(summary.fallback_reason.is_some());
        assert_eq!(summary.cpu_shadow_skipped_creatures, 0);
        assert_eq!(summary.product_runtime_claim, "None");
    }
    summary.validate().unwrap();
}

#[test]
fn ca34_sampled_gpu_runtime_requires_manual_sample_interval() {
    let options = SampledGpuRuntimeOptions {
        cpu_shadow_every: 1,
        ..Default::default()
    };
    assert!(options.validate().is_err());

    let options = SampledGpuRuntimeOptions {
        ticks: 0,
        ..Default::default()
    };
    assert!(options.validate().is_err());
}

#[test]
fn ca31_behavior_comparison_lab_compares_scenarios_and_exports_small_report() {
    let manifest = alife_game_app::default_environment_manifest_path();
    let summary =
        run_behavior_comparison_lab_smoke(&manifest, Some("gpu-alpha"), Some("p34"), 8).unwrap();

    assert_eq!(summary.schema, CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA);
    assert_eq!(
        summary.schema_version,
        CA31_BEHAVIOR_COMPARISON_LAB_SCHEMA_VERSION
    );
    assert_eq!(summary.scenario_a.scenario_id, "gpu-alpha");
    assert_eq!(summary.scenario_b.scenario_id, "p34");
    assert!(summary.scenario_a.creature_count > summary.scenario_b.creature_count);
    assert!(summary.panel.signatures_differ);
    assert!(summary.panel.panel_text.contains("A/B Scenario Runner"));
    assert!(summary.report_markdown.contains("Behavior Signatures"));
    assert!(summary.report_bytes <= CA31_MAX_REPORT_BYTES);
    assert!(!summary.report_markdown.contains("Entity("));

    let output = std::env::temp_dir().join("alife_ca31_behavior_comparison_report.md");
    write_behavior_comparison_lab_report(&summary, &output).unwrap();
    let exported = std::fs::read_to_string(&output).unwrap();
    let _ = std::fs::remove_file(&output);
    assert_eq!(exported, summary.report_markdown);
    summary.validate().unwrap();
}

#[test]
fn ca31_behavior_comparison_lab_is_read_only_and_has_no_action_authority() {
    let manifest = alife_game_app::default_environment_manifest_path();
    let summary =
        run_behavior_comparison_lab_smoke(&manifest, Some("gpu-alpha"), Some("p34"), 4).unwrap();

    assert!(summary.scenario_a.isolated_run);
    assert!(summary.scenario_b.isolated_run);
    assert!(summary.scenario_a.report_only);
    assert!(summary.scenario_b.report_only);
    assert!(summary.scenario_a.no_hidden_training_mutation);
    assert!(summary.scenario_b.no_hidden_training_mutation);
    assert!(summary.panel.read_only);
    assert!(summary.panel.stable_ids_only);
    assert!(!summary.direct_cognition_mutation_allowed);
    assert!(!summary.semantic_action_authority);
    assert!(!summary.gpu_action_authority_claim);
    assert!(summary
        .report_markdown
        .contains("CPU shadow parity remains the gate"));
    assert!(summary
        .report_markdown
        .contains("no full action-authoritative GPU runtime is claimed"));
    assert!(summary.validate().is_ok());
}

#[test]
fn s07_advanced_gameplay_ux_aggregates_optional_social_lifecycle_school_semantic_state() {
    let summary = run_advanced_gameplay_ux_smoke().unwrap();

    assert_eq!(
        summary.schema,
        alife_game_app::S07_ADVANCED_GAMEPLAY_UX_SCHEMA
    );
    assert_eq!(
        summary.schema_version,
        alife_game_app::S07_ADVANCED_GAMEPLAY_UX_SCHEMA_VERSION
    );
    assert_eq!(summary.social.creature_count, 2);
    assert_eq!(summary.lifecycle.births, 1);
    assert_eq!(summary.lifecycle.deaths, 1);
    assert!(summary.school.verifier_passed);
    assert!(summary.semantic.disabled_provider_nonfatal);
    assert!(summary.semantic.context_visible);
    assert!(summary.display_only);
    assert!(summary.optional_modes);
    summary.validate().unwrap();
}

#[test]
fn s07_advanced_gameplay_ux_keeps_authority_boundaries_blocked() {
    let summary = run_advanced_gameplay_ux_smoke().unwrap();

    assert!(summary.social.perception_only);
    assert_eq!(summary.social.direct_action_bypass_count, 0);
    assert!(summary.lifecycle.genetic_lifetime_separated);
    assert!(summary.lifecycle.birth_weight_assets_are_initializers);
    assert!(summary.school.perception_only);
    assert!(summary.school.direct_motor_bypass_blocked);
    assert!(summary.semantic.semantic_action_bypass_blocked);
    assert!(summary.semantic.weight_rewrite_blocked);
    assert!(summary.no_action_or_weight_bypass);
    assert!(!alife_game_app::advanced_gameplay_overlay_text(&summary).contains("Entity("));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_s07_advanced_gameplay_overlay_is_display_only() {
    let summary = run_advanced_gameplay_ux_smoke().unwrap();
    let overlay = alife_game_app::advanced_gameplay_overlay_text(&summary);

    assert!(overlay.contains("Advanced Systems (S07)"));
    assert!(overlay.contains("Social:"));
    assert!(overlay.contains("Lifecycle:"));
    assert!(overlay.contains("School:"));
    assert!(overlay.contains("Semantic:"));
    assert!(overlay.contains("display_only=true"));
    assert!(overlay.contains("no_action_or_weight_bypass=true"));
    assert!(overlay.contains("cannot act or rewrite weights"));
    assert!(!overlay.contains("Entity("));
}

#[test]
fn gpu_product_smoke_defaults_to_cpu_fallback_without_requiring_gpu() {
    let summary = run_gpu_product_hardening_smoke().unwrap();

    assert_eq!(
        summary.schema,
        alife_game_app::G12_GPU_PRODUCT_TELEMETRY_SCHEMA
    );
    assert_eq!(
        summary.schema_version,
        alife_game_app::G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION
    );
    assert!(summary.cpu_fallback_default);
    assert_eq!(summary.telemetry_overlay.selected_backend, "CpuReference");
    assert!(summary.telemetry_overlay.cpu_oracle_authoritative);
    assert!(!summary.telemetry_overlay.measured_gpu_performance);
    summary.validate().unwrap();
}

#[test]
fn gpu_product_smoke_invalid_gpu_config_falls_back_and_reports_reason() {
    let summary = run_gpu_product_hardening_smoke().unwrap();

    assert!(summary.invalid_gpu_config_falls_back);
    assert_eq!(summary.telemetry_overlay.requested_backend, "GpuStatic");
    assert!(summary.telemetry_overlay.fallback_reason.is_some());
    assert!(summary
        .telemetry_overlay
        .report_notes
        .contains("CPU fallback"));
}

#[test]
fn gpu_product_smoke_blocks_active_readback_and_allows_boundary_export() {
    let summary = run_gpu_product_hardening_smoke().unwrap();

    assert!(summary.active_readback_blocked);
    assert!(summary.diagnostic_export_boundary_allowed);
    assert!(summary.telemetry_overlay.no_active_gameplay_readback);
    assert_eq!(
        summary.telemetry_overlay.telemetry_boundary,
        "frame-boundary-diagnostic-export"
    );
}

#[test]
fn gpu_product_smoke_report_is_honest_and_manual_command_is_current() {
    let summary = run_gpu_product_hardening_smoke().unwrap();

    assert!(summary
        .report_markdown_preview
        .contains("CPU fallback is not GPU performance"));
    assert_eq!(summary.performance_claim_status, "unknown-unless-measured");
    assert!(summary
        .manual_hardware_command
        .contains("ALIFE_GPU_RUNTIME_BACKEND=static"));
    assert!(summary.manual_hardware_command.contains("--gpu-runtime"));
    assert!(!summary.manual_hardware_command.contains("--gpu-report"));
}

#[test]
fn s08_gpu_graphics_performance_evidence_keeps_gpu_claims_manual_or_measured() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_gpu_graphics_performance_evidence_smoke(&launch).unwrap();

    assert_eq!(
        summary.schema,
        alife_game_app::S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA
    );
    assert_eq!(
        summary.schema_version,
        alife_game_app::S08_GPU_GRAPHICS_PERFORMANCE_SCHEMA_VERSION
    );
    assert_eq!(
        summary.settings_panel.target_fps,
        alife_game_app::S08_TARGET_FPS
    );
    assert!(summary.cpu_fallback_works);
    assert!(summary.no_active_readback);
    assert!(summary.no_false_gpu_claims);
    assert_eq!(summary.settings_panel.selected_backend, "CpuReference");
    assert!(!summary.settings_panel.measured_gpu_performance);
    assert_ne!(
        summary.settings_panel.gpu_evidence_status,
        S08EvidenceStatus::Measured
    );
    assert_ne!(
        summary.settings_panel.fps_target_status,
        S08EvidenceStatus::Measured
    );
    summary.validate().unwrap();
}

#[test]
fn s08_status_surface_documents_fallback_fps_and_no_readback_boundaries() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_gpu_graphics_performance_evidence_smoke(&launch).unwrap();
    let status = &summary.settings_panel.status_line;

    assert!(status.contains("backend=CpuReference"));
    assert!(status.contains("CPU fallback is not GPU performance"));
    assert!(status.contains("60 FPS target=manual-unknown"));
    assert!(status.contains("no active neural readback"));
    assert!(summary.report_markdown.contains("manual/unknown"));
    assert!(summary
        .report_markdown
        .contains("ALIFE_GPU_RUNTIME_BACKEND=static"));
    assert!(summary.report_markdown.contains("--gpu-runtime"));
    assert!(!summary.report_markdown.contains("--gpu-report"));
    assert!(!summary.report_markdown.contains("ALIFE_GPU_BACKEND"));
}

#[test]
fn s08_graphical_and_benchmark_commands_are_ci_safe_or_explicitly_manual() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_gpu_graphics_performance_evidence_smoke(&launch).unwrap();

    assert_eq!(
        summary.benchmark_smoke_command,
        "cargo run -p alife_tools --bin benchmark_tiers"
    );
    assert_eq!(
        summary.benchmark_gpu_runtime_command,
        "cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime"
    );
    assert!(summary.graphical_dry_run_command.contains("-DryRun"));
    assert!(summary.graphical_smoke_command.contains("-SmokeSeconds 5"));
    assert!(summary
        .graphics_smoke_evidence
        .contains("dry-run is not graphical proof"));
    assert!(summary
        .launch_window_smoke_status
        .contains("real window timing remains manual"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_s08_runtime_overlay_reports_honest_gpu_status() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let mut live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    panel
        .apply_command(&mut live, RuntimeControlCommand::StepOnce)
        .unwrap();
    let overlay = panel.status_overlay_text();

    assert!(overlay.contains("A-Life GPU Alpha Playground"));
    assert!(overlay.contains("GPU: CpuFallback"));
    assert!(overlay.contains("CPU shadow"));
    assert!(overlay.contains("Controls: Space run/pause"));
}

#[test]
fn world_editor_smoke_places_removes_moves_and_saves_stable_ids() {
    let summary = run_world_editor_smoke().unwrap();

    assert_eq!(summary.schema, alife_game_app::G13_WORLD_EDITOR_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G13_WORLD_EDITOR_SCHEMA_VERSION
    );
    assert_eq!(summary.mode_after_edits, WorldEditorMode::EditingPaused);
    assert_eq!(summary.placed_count, 4);
    assert_eq!(summary.removed_count, 1);
    assert_eq!(summary.moved_count, 1);
    assert_eq!(summary.resource_rate_changes, 1);
    assert!(summary.invalid_edit_rejected);
    assert!(summary.undo_available);
    assert_eq!(summary.stable_ids.len(), 3);
    assert!(summary
        .saved_roundtrip_signature
        .iter()
        .any(|line| line.contains("editor-food")));
    assert!(summary
        .saved_roundtrip_signature
        .iter()
        .any(|line| line.contains("editor-hazard")));
    assert!(summary
        .saved_roundtrip_signature
        .iter()
        .any(|line| line.contains("editor-creature")));
    assert!(!summary
        .saved_roundtrip_signature
        .iter()
        .any(|line| line.contains("editor-wall")));
    summary.validate().unwrap();
}

#[test]
fn world_editor_rejects_invalid_edits_and_enforces_caps() {
    let world = HeadlessScenarioBuilder::new(13_113)
        .agent("editor-agent", OrganismId(13_101), alife_core::Vec3f::ZERO)
        .build()
        .unwrap();
    let mut session = WorldEditorSession::new(
        world,
        WorldEditorConfig {
            max_objects: 2,
            world_bound: 4.0,
        },
    )
    .unwrap();

    assert!(session
        .apply_edit(WorldEditCommand::place_food(
            "not-paused",
            alife_core::Vec3f::new(0.2, 0.0, 0.0),
            0.5,
        ))
        .is_err());
    session.enter_editor();
    assert!(session
        .apply_edit(WorldEditCommand::place_food(
            "editor-food",
            alife_core::Vec3f::new(0.2, 0.0, 0.0),
            0.5,
        ))
        .unwrap()
        .is_some());
    assert!(session
        .apply_edit(WorldEditCommand::place_hazard(
            "over-cap-hazard",
            alife_core::Vec3f::new(0.4, 0.0, 0.0),
            0.2,
        ))
        .is_err());
    assert!(session
        .apply_edit(WorldEditCommand::place_food(
            "out-of-bounds",
            alife_core::Vec3f::new(8.0, 0.0, 0.0),
            0.5,
        ))
        .is_err());
    assert_eq!(session.world().object_count(), 2);
}

#[test]
fn world_editor_roundtrip_resource_rates_and_undo_are_stable() {
    let mut world = HeadlessScenarioBuilder::new(13_213)
        .agent("editor-agent", OrganismId(13_201), alife_core::Vec3f::ZERO)
        .build()
        .unwrap();
    world
        .add_terrain_zone(
            TerrainZone::new(
                EcologyZoneId(13),
                "editor-meadow",
                TerrainZoneKind::Meadow,
                alife_core::Vec3f::ZERO,
                8.0,
                0.8,
                0.1,
            )
            .unwrap(),
        )
        .unwrap();
    let before = world.stable_signature();
    let mut session = WorldEditorSession::new(world, WorldEditorConfig::default()).unwrap();
    session.enter_editor();
    let food = session
        .apply_edit(WorldEditCommand::place_food(
            "editor-food",
            alife_core::Vec3f::new(0.5, 0.0, 0.0),
            0.65,
        ))
        .unwrap()
        .unwrap();
    session
        .apply_edit(WorldEditCommand::SetFoodResourceRate {
            food_id: food,
            home_zone: EcologyZoneId(13),
            regrow_after_ticks: 2,
            decay_after_ticks: 4,
        })
        .unwrap();
    let save = session.save_portable("g13-test-save").unwrap();
    let json = save.to_json_string_pretty().unwrap();
    let loaded = PortableSaveFile::from_json_str(&json).unwrap();
    assert_eq!(
        loaded
            .restore_headless_world()
            .unwrap()
            .ecology()
            .resources
            .len(),
        1
    );

    session.undo_last().unwrap();
    session.undo_last().unwrap();
    assert_eq!(session.world().stable_signature(), before);
}

#[test]
fn world_editor_resume_uses_sealed_patch_without_direct_cognition_mutation() {
    let summary = run_world_editor_smoke().unwrap();

    assert!(summary.simulation_resumed);
    assert!(summary.resumed_patch_sealed);
    assert_eq!(summary.cognition_direct_mutation_count, 0);
    assert!(summary.edit_log.iter().any(|entry| entry == "place"));
    assert!(summary.edit_log.iter().any(|entry| entry == "remove"));
}

#[test]
fn cognition_debug_timeline_uses_sealed_patches_only_and_is_read_only() {
    let panel = run_cognition_debug_timeline_smoke().unwrap();

    assert_eq!(panel.schema, alife_game_app::G14_COGNITION_DEBUG_SCHEMA);
    assert_eq!(
        panel.schema_version,
        alife_game_app::G14_COGNITION_DEBUG_SCHEMA_VERSION
    );
    assert!(panel.read_only);
    assert!(!panel.mutation_controls_enabled);
    assert!(!panel.timeline_entries.is_empty());
    assert!(panel.timeline_entries.len() <= alife_game_app::G14_MAX_TIMELINE_ENTRIES);
    assert!(panel
        .timeline_entries
        .iter()
        .all(|entry| entry.sealed_patch_only && entry.packed_log_available));
    assert!(panel
        .timeline_entries
        .iter()
        .all(|entry| entry.summary_line.contains("sealed_patch=true")));
    assert!(panel
        .panel_notes
        .iter()
        .any(|note| note.contains("sealed ExperiencePatch")));
    panel.validate().unwrap();
}

#[test]
fn cognition_debug_keeps_memory_topology_gpu_and_exports_boundary_safe() {
    let panel = run_cognition_debug_timeline_smoke().unwrap();

    assert!(panel.bias_summary.action_replay_blocked);
    assert!(panel.bias_summary.topology_action_bypass_blocked);
    assert!(panel
        .bias_summary
        .memory_expectancy_line
        .contains("bias_only"));
    assert!(panel
        .bias_summary
        .memory_expectancy_line
        .contains("no_action_replay"));
    assert!(panel
        .bias_summary
        .topology_gap_line
        .contains("cannot_emit_action"));
    assert!(!panel
        .bias_summary
        .memory_expectancy_line
        .contains("ActionCommand"));
    assert!(!panel
        .bias_summary
        .topology_gap_line
        .contains("ActionCommand"));

    assert!(panel.no_active_neural_readback);
    assert!(panel.gpu_summary.no_active_gameplay_readback);
    assert_eq!(
        panel.gpu_summary.telemetry_boundary,
        "frame-boundary-diagnostic-export"
    );
    assert!(!panel.gpu_summary.measured_gpu_performance);
    assert!(panel.gpu_summary.report_notes.contains("CPU fallback"));

    assert!(panel.packed_log_export.offline_only);
    assert!(!panel.packed_log_export.mutates_runtime_state);
    assert!(panel
        .packed_log_export
        .export_command
        .contains("p30_offline"));
}

#[test]
fn cognition_debug_reports_arbitration_and_sleep_without_runtime_control() {
    let panel = run_cognition_debug_timeline_smoke().unwrap();

    assert!(!panel.proposal_lines.is_empty());
    assert!(panel.proposal_lines.len() <= alife_game_app::G14_MAX_PROPOSAL_LINES);
    assert_eq!(
        panel
            .proposal_lines
            .iter()
            .filter(|line| line.selected_by_arbitration)
            .count(),
        1
    );
    assert!(panel.proposal_lines.iter().all(|line| line
        .bias_only_sources
        .iter()
        .any(|source| source.contains("action_arbitration"))));

    assert!(panel.sleep_summary.rest_event_seen);
    assert!(panel.sleep_summary.consolidation_visible);
    assert!(!panel.sleep_summary.structural_edits_active_tick_applied);
    assert!(panel
        .sleep_summary
        .summary_line
        .contains("structural_edit_active_tick_applied=false"));
}

#[test]
fn save_load_ux_smoke_roundtrips_visible_world_with_stable_ids() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_save_load_ux_smoke(&launch).unwrap();

    assert_eq!(summary.schema, alife_game_app::G15_SAVE_LOAD_UX_SCHEMA);
    assert_eq!(
        summary.schema_version,
        alife_game_app::G15_SAVE_LOAD_UX_SCHEMA_VERSION
    );
    assert_eq!(summary.loaded_save_id, "g15-manual-slot");
    assert_eq!(summary.restored_object_count, 2);
    assert_eq!(
        summary.stable_world_ids,
        vec![WorldEntityId(1), WorldEntityId(2)]
    );
    assert!(summary.stable_id_remap_preserved);
    assert!(summary.engine_local_token_absent);
    assert!(summary
        .menu
        .stable_id_remap_summary
        .contains("stable_world_ids=2"));
    summary.validate().unwrap();
}

#[test]
fn save_load_ux_requires_overwrite_confirmation_and_autosave_is_deterministic() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let source_save = PortableSaveFile::from_json_file(&launch.save_path).unwrap();
    let mut manager = SaveSlotManager::new(alife_game_app::G15_MAX_SAVE_SLOTS).unwrap();
    let descriptor =
        SaveSlotDescriptor::new("slot-0", "Manual Save", SaveSlotKind::Manual).unwrap();

    let first = manager.save_slot(descriptor.clone(), &source_save, &launch.asset_root, false);
    assert!(first.success);
    let blocked = manager.save_slot(descriptor.clone(), &source_save, &launch.asset_root, false);
    assert!(!blocked.success);
    assert!(blocked.overwrite_confirmation_required);
    assert_eq!(
        blocked.error.as_ref().map(|error| error.code.as_str()),
        Some("overwrite-confirmation-required")
    );
    let confirmed = manager.save_slot(descriptor, &source_save, &launch.asset_root, true);
    assert!(confirmed.success);

    let autosave = AutosavePolicy::deterministic_default();
    assert!(autosave.should_autosave(None, source_save.world.tick));
    assert!(!autosave.should_autosave(
        Some(Tick::new(source_save.world.tick.raw().saturating_sub(1))),
        source_save.world.tick
    ));
}

#[test]
fn save_load_ux_displays_invalid_save_config_asset_errors_without_partial_load() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_save_load_ux_smoke(&launch).unwrap();

    assert_eq!(summary.invalid_schema_error.code, "schema-version");
    assert!(summary
        .invalid_schema_error
        .message
        .contains("schema version mismatch"));
    assert_eq!(summary.missing_asset_error.code, "missing-required-asset");
    assert!(summary.missing_asset_error.message.contains("missing"));
    assert_eq!(summary.digest_error.code, "digest-mismatch");
    assert!(summary.digest_error.message.contains("digest mismatch"));
    assert_eq!(summary.invalid_config_error.code, "invalid-config");
    assert!(summary
        .invalid_config_error
        .message
        .contains("deterministic_seed"));
    assert!(summary.no_partial_load_after_error);
    assert!(!summary.invalid_schema_error.partial_load_applied);
}

#[test]
fn config_menu_defaults_are_deterministic_and_keep_optional_features_safe() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let config = RuntimeConfig::from_json_file(&launch.config_path).unwrap();
    let menu = ConfigMenuState::validate_config(&config).unwrap();

    assert_eq!(menu.schema_version, 1);
    assert_eq!(menu.requested_backend, BackendSelection::CpuReference);
    assert_eq!(menu.deterministic_seed, 4242);
    assert!(!menu.school_enabled);
    assert!(!menu.semantic_enabled);
    assert!(!menu.gpu_enabled);
    assert!(menu.cpu_fallback_required);
    assert!(menu.no_active_readback);

    let mut invalid = config;
    invalid.backend.requested = BackendSelection::GpuStatic;
    invalid.backend.gpu_feature_enabled = false;
    let error = ConfigMenuState::validate_config(&invalid).unwrap_err();
    assert_eq!(error.code, "invalid-config");
    assert!(error.message.contains("GPU backend selection"));
}

#[test]
fn save_load_menu_text_exposes_player_flows_and_readable_errors() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_save_load_ux_smoke(&launch).unwrap();
    let text = alife_game_app::player_save_load_menu_text(&summary);

    assert!(text.contains("Save / Load Menu"));
    assert!(text.contains("Tabs: New | Save | Load | Settings"));
    assert!(text.contains("Manual Save 1"));
    assert!(text.contains("Autosave"));
    assert!(text.contains("Save: manual slot=slot-0"));
    assert!(text.contains("Load: slot-0 -> save=g15-manual-slot"));
    assert!(text.contains("Overwrite: confirm required=true"));
    assert!(text.contains("Cancel: keeps current slot"));
    assert!(text.contains("Errors: schema=schema-version"));
    assert!(text.contains("missing_asset=missing-required-asset"));
    assert!(text.contains("digest=digest-mismatch"));
    assert!(text.contains("config=invalid-config"));
    assert!(text.contains("partial_load_after_error=false"));
    assert!(text.contains("Settings: backend=CpuReference"));
    assert!(text.contains("cpu_fallback=true"));
    assert!(text.contains("no_active_readback=true"));
    assert!(text.contains("Stable IDs: [1, 2]"));
    assert!(text.contains("engine-local tokens=false"));
    assert!(!text.contains("Entity("));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_can_construct_shell_without_visible_world_content() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_headless_app_shell_smoke(&launch).unwrap();
    let mut app = alife_game_app::bevy_shell::build_minimal_bevy_app_shell(summary);
    app.update();
    assert!(app
        .world()
        .get_resource::<alife_bevy_adapter::AdapterWorldTick>()
        .is_some());
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_spawns_visible_world_with_adapter_local_stable_mapping() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let (mut app, summary) = alife_game_app::bevy_shell::build_visible_world_app_shell(&launch)
        .expect("visible world should load from the committed P34 fixture");
    assert_eq!(summary.object_count, 2);
    assert_eq!(summary.stable_map_count, 2);

    let mut visible_query = app
        .world_mut()
        .query::<&alife_game_app::bevy_shell::VisibleWorldObject>();
    let visible = visible_query.iter(app.world()).collect::<Vec<_>>();
    assert_eq!(visible.len(), 2);

    let map = app.world().resource::<alife_bevy_adapter::BevyEntityMap>();
    for object in visible {
        assert!(map.bevy_entity(object.stable_id).is_some());
    }
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_live_brain_bridge_records_last_tick_summary() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let (mut app, visible, live) =
        alife_game_app::bevy_shell::build_live_brain_world_app_shell(&launch)
            .expect("G03 live bridge should run on the P34 fixture");
    assert_eq!(visible.object_count, 2);
    assert!(live.patch_sealed);
    assert_eq!(live.selected_action_kind, Some(ActionKind::Interact));
    let target = live
        .target_entity
        .expect("P34 fixture live tick should select the visible food target");
    assert!(app
        .world()
        .resource::<alife_bevy_adapter::BevyEntityMap>()
        .bevy_entity(target)
        .is_some());
    app.update();
    let resource = app
        .world()
        .resource::<alife_game_app::bevy_shell::LiveBrainLoopResource>();
    assert_eq!(resource.last_summary.sealed_patch_count, 1);
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_creature_visual_state_attaches_to_visible_creature() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let (mut app, visible, visual) =
        alife_game_app::bevy_shell::build_creature_visual_world_app_shell(&launch)
            .expect("G04 creature visual shell should run on the P34 fixture");
    assert_eq!(visible.object_count, 2);
    assert_eq!(visual.animation, CreatureAnimationState::Interacting);
    app.update();

    let resource = app
        .world()
        .resource::<alife_game_app::bevy_shell::CreatureVisualStateResource>();
    assert_eq!(resource.snapshot.stable_id, WorldEntityId(1));

    let entity = app
        .world()
        .resource::<alife_bevy_adapter::BevyEntityMap>()
        .bevy_entity(WorldEntityId(1))
        .expect("visible creature should be mapped by stable ID");
    let state = app
        .world()
        .entity(entity)
        .get::<alife_game_app::bevy_shell::VisibleCreatureState>()
        .expect("G04 visual state component should be attached to the creature");
    assert_eq!(state.animation, visual.animation);
    assert_eq!(state.expression, visual.expression);
    assert!(state.debug_summary.contains("organism=1"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_creature_inspector_keeps_local_entity_mapping_out_of_model() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let (mut app, visible, inspector) =
        alife_game_app::bevy_shell::build_creature_inspector_world_app_shell(&launch)
            .expect("G05 inspector shell should run on the P34 fixture");
    assert_eq!(visible.object_count, 2);
    assert!(inspector.read_only);
    app.update();

    let selection = app
        .world()
        .resource::<alife_game_app::bevy_shell::SelectionResource>();
    assert_eq!(selection.stable_id, WorldEntityId(1));
    let local = selection
        .local_entity
        .expect("selected stable ID should map to a local Bevy entity");
    let selected_component = app
        .world()
        .entity(local)
        .get::<alife_game_app::bevy_shell::SelectedVisibleEntity>()
        .expect("selected visible entity component should be local only");
    assert_eq!(selected_component.selection.stable_id, WorldEntityId(1));

    let inspector_resource = app
        .world()
        .resource::<alife_game_app::bevy_shell::CreatureInspectorResource>();
    assert_eq!(
        inspector_resource.snapshot.selection.stable_id,
        WorldEntityId(1)
    );
    assert!(inspector_resource
        .snapshot
        .action_summary
        .contains("Interact"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_graphical_playground_carries_s02_runtime_controls() {
    let launch = alife_game_app::GraphicalPlaygroundLaunchConfig::smoke(p34_fixture_root(), 5);
    let runtime = alife_game_app::bevy_shell::GraphicalRuntimeControlsResource::new(&launch)
        .expect("S02 runtime controls should initialize from the P34 fixture");
    assert_eq!(runtime.smoke_target_ticks, Some(5));
    assert_eq!(runtime.panel.playback, RuntimePlaybackState::Paused);
    assert_eq!(
        runtime.panel.schema,
        alife_game_app::S02_RUNTIME_CONTROLS_SCHEMA
    );
    assert_eq!(
        runtime.panel.scheduler.schema,
        alife_game_app::CA13_DOUBLE_BUFFERED_SCHEDULER_SCHEMA
    );
    assert!(runtime
        .panel
        .structured_status_panel_text_with_backend("GPU: test")
        .contains("Scheduler: fixed=20Hz"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_s03_inspector_overlay_is_read_only_and_stable_id_based() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let (app, visible, inspector) =
        alife_game_app::bevy_shell::build_creature_inspector_world_app_shell(&launch)
            .expect("S03 inspector shell should load from the P34 fixture");

    assert_eq!(visible.object_count, 2);
    assert!(inspector.read_only);

    let camera = *app
        .world()
        .resource::<alife_game_app::bevy_shell::CameraNavigationResource>();
    let selection = *app
        .world()
        .resource::<alife_game_app::bevy_shell::SelectionResource>();
    let inspector_resource = app
        .world()
        .resource::<alife_game_app::bevy_shell::CreatureInspectorResource>()
        .clone();
    let gpu = alife_game_app::bevy_shell::GraphicalGpuTelemetryResource {
        telemetry: GraphicalGpuRuntimeTelemetry::cpu_reference(
            GraphicalGpuRuntimeMode::CpuReference,
            0,
        ),
    };
    let live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let runtime = alife_game_app::bevy_shell::GraphicalRuntimeControlsResource {
        panel: RuntimeControlPanel::from_live_loop(&live),
        smoke_target_ticks: None,
        smoke_ticks_done: 0,
    };

    let overlay = alife_game_app::bevy_shell::graphical_inspector_overlay_text(
        &runtime,
        &camera,
        &selection,
        &inspector_resource,
        &gpu,
    );

    assert_eq!(selection.stable_id, WorldEntityId(1));
    assert_eq!(camera.state.follow_target, Some(WorldEntityId(1)));
    assert!(overlay.contains("Creature Inspector"));
    assert!(overlay.contains("Stable ID: 1"));
    assert!(overlay.contains("State: Awake"));
    assert!(overlay.contains("Energy "));
    assert!(overlay.contains("Health "));
    assert!(overlay.contains("Hunger "));
    assert!(overlay.contains("Fatigue"));
    assert!(overlay.contains("Fear   "));
    assert!(overlay.contains("Action: EAT"));
    assert!(overlay.contains("Target: stable:2"));
    assert!(overlay.contains("Patch: sealed=true"));
    assert!(overlay.contains("Learning: H_shadow="));
    assert!(overlay.contains("Read-only stable IDs"));
    assert!(overlay.contains("Tech: CpuReference"));
    assert!(overlay.contains("gate=CPU shadow"));
    assert!(overlay.contains("full_auth=false"));
    assert!(!overlay.contains("Entity("));
    assert!(overlay.lines().all(|line| line.chars().count() <= 46));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca07_inspector_bars_are_readable_and_player_facing() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let (app, _visible, inspector) =
        alife_game_app::bevy_shell::build_creature_inspector_world_app_shell(&launch)
            .expect("inspector app builds");
    let camera = *app
        .world()
        .resource::<alife_game_app::bevy_shell::CameraNavigationResource>();
    let selection = *app
        .world()
        .resource::<alife_game_app::bevy_shell::SelectionResource>();
    let inspector_resource = alife_game_app::bevy_shell::CreatureInspectorResource {
        snapshot: inspector,
    };
    let gpu = alife_game_app::bevy_shell::GraphicalGpuTelemetryResource {
        telemetry: GraphicalGpuRuntimeTelemetry {
            requested_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
            selected_backend: "GpuPlastic".to_string(),
            fallback_reason: None,
            hardware_identifier: Some("local-test".to_string()),
            product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow".to_string(),
            gpu_static_dispatched_ticks: 3,
            gpu_scores_used_for_proposals: true,
            cpu_shadow_parity: true,
            parity_failures: 0,
            sealed_patches: 3,
            h_shadow_applications: 2,
            last_h_shadow_delta: 0.0125,
            compact_readback_bytes: 64,
            post_seal_readback_bytes: 64,
            total_gpu_runtime_ms: 1.25,
            wgsl: test_wgsl_telemetry(),
            no_active_bulk_readback: true,
            full_action_authoritative_claim: false,
        },
    };
    let live = LiveBrainLoop::from_p34_launch(&launch).unwrap();
    let runtime = alife_game_app::bevy_shell::GraphicalRuntimeControlsResource {
        panel: RuntimeControlPanel::from_live_loop(&live),
        smoke_target_ticks: None,
        smoke_ticks_done: 0,
    };

    let overlay = alife_game_app::bevy_shell::graphical_inspector_overlay_text(
        &runtime,
        &camera,
        &selection,
        &inspector_resource,
        &gpu,
    );
    let bars = alife_game_app::bevy_shell::ca07_creature_state_bars(&inspector_resource.snapshot);

    assert_eq!(bars.len(), 5);
    assert!(bars
        .iter()
        .all(|line| line.contains('[') && line.contains(']')));
    assert!(overlay.contains("Energy "));
    assert!(overlay.contains("Health "));
    assert!(overlay.contains("Hunger "));
    assert!(overlay.contains("Fatigue"));
    assert!(overlay.contains("Fear   "));
    assert!(overlay.contains("State: Awake"));
    assert!(overlay.contains("Learning: H_shadow=2 last=0.0125"));
    assert!(overlay.contains("Tech: GpuPlastic"));
    assert!(overlay.contains("gate=CPU shadow"));
    assert!(overlay.contains("full_auth=false"));
    assert!(!overlay.contains("GPU Runtime"));
    assert!(!overlay.contains("Entity("));
    assert!(overlay.lines().all(|line| line.chars().count() <= 46));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca08_sensory_feedback_cues_are_display_only_and_readable() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let feedback = run_feedback_polish_smoke(&launch).unwrap();
    let gpu = GraphicalGpuRuntimeTelemetry {
        requested_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        selected_backend: "GpuPlastic".to_string(),
        fallback_reason: None,
        hardware_identifier: Some("local-test".to_string()),
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow".to_string(),
        gpu_static_dispatched_ticks: 3,
        gpu_scores_used_for_proposals: true,
        cpu_shadow_parity: true,
        parity_failures: 0,
        sealed_patches: 3,
        h_shadow_applications: 2,
        last_h_shadow_delta: 0.0125,
        compact_readback_bytes: 64,
        post_seal_readback_bytes: 64,
        total_gpu_runtime_ms: 1.25,
        wgsl: test_wgsl_telemetry(),
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };

    let rows = alife_game_app::bevy_shell::ca08_sensory_feedback_cues(&feedback, &gpu);
    let panel = alife_game_app::bevy_shell::ca08_sensory_cue_panel_text(&feedback, &gpu);
    let legend = alife_game_app::bevy_shell::readability_legend_overlay_text();

    assert_eq!(rows.len(), 4);
    assert!(rows
        .iter()
        .any(|row| row.kind.label() == "reward" && row.active));
    assert!(rows
        .iter()
        .any(|row| row.kind.label() == "pain" && row.active));
    assert!(rows
        .iter()
        .any(|row| row.kind.label() == "sleep" && row.active));
    assert!(rows
        .iter()
        .any(|row| row.kind.label() == "learning" && row.active));
    assert!(panel.contains("Sensory Cues (display-only)"));
    assert!(panel.contains("soft-ping"));
    assert!(panel.contains("warning-pulse"));
    assert!(panel.contains("rest-chime"));
    assert!(panel.contains("learn-spark"));
    assert!(panel.contains("Boundary: no action/weight authority"));
    assert!(!panel.contains("Entity("));
    assert!(legend.contains("reward=green"));
    assert!(legend.contains("learning=teal"));
    assert!(legend.contains("Audio stubs"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca39_drive_cue_panel_is_player_readable_and_display_only() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let feedback = run_feedback_polish_smoke(&launch).unwrap();
    let gpu = GraphicalGpuRuntimeTelemetry {
        requested_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
        selected_backend: "GpuPlastic".to_string(),
        fallback_reason: None,
        hardware_identifier: Some("local-test".to_string()),
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow".to_string(),
        gpu_static_dispatched_ticks: 3,
        gpu_scores_used_for_proposals: true,
        cpu_shadow_parity: true,
        parity_failures: 0,
        sealed_patches: 3,
        h_shadow_applications: 2,
        last_h_shadow_delta: 0.0125,
        compact_readback_bytes: 64,
        post_seal_readback_bytes: 64,
        total_gpu_runtime_ms: 1.25,
        wgsl: test_wgsl_telemetry(),
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };

    let panel =
        alife_game_app::ca39_drive_audio_vfx_panel_text_from_graphical(&feedback, &gpu).unwrap();

    assert!(panel.contains("Drive Audio/VFX"));
    assert!(panel.contains("Food chime:on"));
    assert!(panel.contains("Hazard pulse:on"));
    assert!(panel.contains("Rest bloom:on"));
    assert!(panel.contains("Learning pulse:on"));
    assert!(panel.contains("H_shadow apps=2"));
    assert!(panel.contains("CPU shadow gate"));
    assert!(panel.contains("no actions/weights"));
    assert!(!panel.contains("full action-authoritative"));
    assert!(!panel.contains("Entity("));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca08_graphical_pulse_markers_spawn_without_model_authority() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    let pulses = alife_game_app::bevy_shell::ca08_pulse_targets_for_presentation(&presentation);
    let labels = pulses
        .iter()
        .map(|pulse| pulse.kind.label())
        .collect::<Vec<_>>();

    assert_eq!(pulses.len(), 4);
    assert!(labels.contains(&"reward"));
    assert!(labels.contains(&"pain"));
    assert!(labels.contains(&"sleep"));
    assert!(labels.contains(&"learning"));
    assert!(pulses.iter().all(|pulse| pulse.target_stable_id.is_some()));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_s04_readability_feedback_is_display_only() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let (app, visible, _inspector) =
        alife_game_app::bevy_shell::build_creature_inspector_world_app_shell(&launch)
            .expect("S04 readability helpers should use the existing headless-safe Bevy shell");
    assert_eq!(visible.object_count, 2);

    let feedback = run_feedback_polish_smoke(&launch).unwrap();
    assert!(feedback.non_authoritative);
    assert_eq!(feedback.sealed_outcome_event_count, 4);
    assert!(feedback
        .event_labels()
        .contains(&FeedbackEventKind::FoodReward.label()));
    assert!(feedback
        .event_labels()
        .contains(&FeedbackEventKind::HazardPain.label()));
    assert!(feedback
        .event_labels()
        .contains(&FeedbackEventKind::SleepTransition.label()));

    let inspector = app
        .world()
        .resource::<alife_game_app::bevy_shell::CreatureInspectorResource>();
    let feedback_text = alife_game_app::bevy_shell::feedback_cue_overlay_text(&feedback, inspector);
    assert!(feedback_text.contains("Play Feedback (display-only)"));
    assert!(feedback_text.contains("Food=true"));
    assert!(feedback_text.contains("hazard=true"));
    assert!(feedback_text.contains("sleep=true"));
    assert!(feedback_text.contains("failure=true"));
    assert!(feedback_text.contains("cannot act or mutate weights"));
    assert!(!feedback_text.contains("Entity("));

    let legend = alife_game_app::bevy_shell::readability_legend_overlay_text();
    assert!(legend.contains("[@] creature"));
    assert!(legend.contains("[+] food"));
    assert!(legend.contains("[!] hazard"));
    assert!(legend.contains("[#] obstacle/rock"));
    assert!(legend.contains("Viewport: local camera slice"));
    assert!(legend.contains("off-screen stable-ID food, hazards, obstacles"));
    assert!(legend.contains("world/core arbitration still owns actions"));

    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    let badges = presentation
        .objects
        .iter()
        .map(alife_game_app::bevy_shell::graphical_object_badge_text)
        .collect::<Vec<_>>();
    assert!(badges.iter().any(|badge| badge.contains("[@] creature")));
    assert!(badges.iter().any(|badge| badge.contains("[+] food")));
    assert!(badges.iter().all(|badge| badge.contains("stable:")));
}

#[test]
fn ca09_graphical_save_load_menu_opens_saves_loads_and_rejects_bad_saves() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let summary = alife_game_app::run_graphical_save_load_menu_smoke(&launch).unwrap();

    assert!(summary.menu_opened);
    assert!(summary.manual_save.success);
    assert!(summary.manual_load.success);
    assert!(!summary.invalid_load.success);
    assert_eq!(
        summary
            .invalid_load
            .error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("schema-version")
    );
    assert_eq!(
        summary.stable_world_ids,
        (1..=12).map(WorldEntityId).collect::<Vec<_>>()
    );
    assert!(summary.overlay_text.contains("Save / Load"));
    assert!(summary.overlay_text.contains("F5 save"));
    assert!(summary.overlay_text.contains("F9 load"));
    assert!(summary
        .overlay_text
        .contains("Stable IDs: [1, 2, 3, 4, 5, 6, 7, 8"));
    assert!(summary.overlay_text.contains("partial_load=false"));
    assert!(summary.engine_local_token_absent);
    assert!(!summary.overlay_text.contains("Entity("));
    summary.validate().unwrap();
}

#[test]
fn ca09_graphical_save_load_session_keeps_closed_bar_compact_and_stable_id_safe() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let mut session = alife_game_app::GraphicalSaveLoadMenuSession::from_launch(&launch).unwrap();
    session.validate().unwrap();

    let closed = alife_game_app::graphical_save_load_menu_text(&session);
    assert!(closed.contains("Save/Load: M menu"));
    assert!(closed.contains("F5 save"));
    assert!(closed.contains("F9 load"));
    assert!(closed.contains("Stable IDs [1, 2, 3, 4, 5, 6, 7, 8"));
    assert!(!closed.contains("Entity("));

    let result = session.apply_command(alife_game_app::GraphicalSaveLoadMenuCommand::ToggleMenu);
    assert!(result.success);
    let open = alife_game_app::graphical_save_load_menu_text(&session);
    assert!(open.contains("Save / Load"));
    assert!(open.contains("Boundary: stable IDs only"));
    assert!(open.contains("no partial load after errors"));
    assert!(!open.contains("Entity("));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_alpha_overlay_text_is_first_tester_readable() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let save_load = run_save_load_ux_smoke(&launch).unwrap();
    let advanced = run_advanced_gameplay_ux_smoke().unwrap();

    let save_note = alife_game_app::bevy_shell::alpha_save_load_note_text(&save_load);
    assert!(save_note.contains("Save/Load Alpha Note"));
    assert!(save_note.contains("Reset/restart"));
    assert!(save_note.contains("Stable IDs: [1, 2]"));
    assert!(!save_note.contains("Entity("));

    let playtest_note = alife_game_app::bevy_shell::alpha_playtest_status_note_text(&advanced);
    assert!(playtest_note.contains("Alpha Playtest Focus"));
    assert!(playtest_note.contains("GPU-first"));
    assert!(playtest_note.contains("CPU fallback is degraded safety mode"));
    assert!(playtest_note.contains("Record: window"));
    assert!(!playtest_note.contains("full action-authoritative"));

    let controls = alife_game_app::bevy_shell::alpha_controls_help_text();
    assert!(controls.contains("Space run/pause"));
    assert!(controls.contains("R reset"));
    assert!(controls.contains("M save/load"));
    assert!(controls.contains("F5 save"));
    assert!(controls.contains("F9 load"));
    assert!(controls.contains("Esc quit"));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_s05_save_load_menu_overlay_is_player_facing_and_stable_id_safe() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
    let summary = run_save_load_ux_smoke(&launch).unwrap();
    let overlay = alife_game_app::bevy_shell::save_load_menu_overlay_text(&summary);

    assert!(overlay.contains("Save / Load Menu"));
    assert!(overlay.contains("Tabs: New | Save | Load | Settings"));
    assert!(overlay.contains("Stable IDs: [1, 2]"));
    assert!(overlay.contains("Overwrite: confirm required=true"));
    assert!(overlay.contains("Cancel: keeps current slot"));
    assert!(overlay.contains("schema=schema-version"));
    assert!(overlay.contains("cpu_fallback=true"));
    assert!(overlay.contains("Boundary: stable IDs only"));
    assert!(!overlay.contains("Entity("));
}

#[cfg(feature = "bevy-app")]
#[test]
fn bevy_feature_ca09_graphical_save_load_overlay_uses_live_menu_session() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let mut session = alife_game_app::GraphicalSaveLoadMenuSession::from_launch(&launch).unwrap();
    session.apply_command(alife_game_app::GraphicalSaveLoadMenuCommand::ToggleMenu);
    let overlay = alife_game_app::graphical_save_load_menu_text(&session);

    assert!(overlay.contains("Save / Load"));
    assert!(overlay.contains("F5 save manual slot"));
    assert!(overlay.contains("F9 load manual slot"));
    assert!(overlay.contains("Stable IDs: [1, 2, 3, 4, 5, 6, 7, 8"));
    assert!(overlay.contains("Boundary: stable IDs only"));
    assert!(!overlay.contains("Entity("));

    let controls = alife_game_app::bevy_shell::ca05_controls_bar_text();
    assert!(controls.contains("M save/load"));
    assert!(controls.contains("F5 save"));
    assert!(controls.contains("F9 load"));
}
