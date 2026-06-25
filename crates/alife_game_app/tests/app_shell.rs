use alife_core::{
    ActionKind, ActionProposal, ActionTarget, BrainTickInput, BrainTickStatus, Confidence,
    CreatureMind, DurationTicks, NormalizedScalar, OrganismId, Tick, Validate, WorldEntityId,
};
use alife_game_app::{
    compare_visible_world_to_headless, g17_feedback_manifest_path, g17_workspace_root,
    load_visible_world_from_p34_save, project_lod_without_behavior_change,
    run_advanced_gameplay_ux_smoke, run_cognition_debug_timeline_smoke,
    run_content_authoring_smoke, run_creature_inspector_smoke, run_creature_visual_smoke,
    run_double_buffered_scheduler_smoke, run_feedback_polish_smoke, run_full_gpu_runtime_smoke,
    run_gpu_graphics_performance_evidence_smoke, run_gpu_longrun_soak,
    run_gpu_product_hardening_smoke, run_gpu_sustained_learning_soak, run_graphical_controls_smoke,
    run_headless_app_shell_smoke, run_homeostasis_runtime_smoke, run_lifecycle_lineage_smoke,
    run_live_brain_loop_paused_smoke, run_live_brain_loop_smoke, run_longrun_balance_smoke,
    run_longrun_balance_with_config, run_motor_ring_arbitration_smoke, run_onboarding_help_smoke,
    run_platform_package_smoke, run_playable_survival_loop_smoke,
    run_population_performance_lod_smoke, run_population_social_loop_smoke,
    run_product_qa_hardening_smoke, run_release_candidate_smoke, run_runtime_controls_smoke,
    run_save_load_ux_smoke, run_school_mode_smoke, run_semantic_provider_smoke,
    run_world_ecology_loop_smoke, run_world_editor_smoke, select_visible_world_entity,
    validate_app_shell_config, AppShellLaunchConfig, AutosavePolicy, Ca13TickBuffer, CadenceTarget,
    CameraNavigationState, ConfigMenuState, CreatureAnimationState, CreatureExpressionState,
    CreatureLifeStage, DoubleBufferedGraphicalScheduler, FeedbackAssetKind, FeedbackAssetManifest,
    FeedbackEventKind, FullGpuRuntimeSmokeMode, FullGpuRuntimeSmokeOptions, GpuLongrunSoakOptions,
    GpuSustainedLearningSoakOptions, GraphicalGpuRuntimeMode, GraphicalGpuRuntimeTelemetry,
    InspectorControlPanel, LifecycleEventKind, LifecycleLiveLoop, LifecycleLoopConfig,
    LifecycleSaveState, LiveBrainLoop, LiveBrainTickControl, LodResidency, LongRunBalanceConfig,
    PackageSmokeKind, PlayableSurvivalEventKind, PopulationLiveLoop, PopulationLoopConfig,
    PopulationPerformancePolicy, PopulationSocialEventKind, ProductQaArea, ProductQaStatus,
    ReleaseCandidateArea, ReleaseCandidateGateStatus, RenderDetailLevel, RuntimeControlCommand,
    RuntimeControlPanel, RuntimePlaybackState, S08EvidenceStatus, SaveSlotDescriptor, SaveSlotKind,
    SaveSlotManager, SchoolModeSaveState, VisibleMaterialKind, VisiblePlaceholderShape,
    WorldEditCommand, WorldEditorConfig, WorldEditorMode, WorldEditorSession,
    G21_ASSET_BUNDLE_SCHEMA, G21_ASSET_BUNDLE_SCHEMA_VERSION, G21_PLATFORM_PACKAGE_SCHEMA,
    G21_PLATFORM_PACKAGE_SCHEMA_VERSION,
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
fn gpu_alpha_fixture_adds_real_hazard_and_obstacle_markers_without_changing_stable_id_contract() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(gpu_alpha_fixture_root());
    let presentation = load_visible_world_from_p34_save(&launch).unwrap();
    compare_visible_world_to_headless(&presentation).unwrap();
    assert_eq!(presentation.object_count, 4);
    assert_eq!(presentation.kind_count(WorldObjectKind::Agent), 1);
    assert_eq!(presentation.kind_count(WorldObjectKind::Food), 1);
    assert_eq!(presentation.kind_count(WorldObjectKind::Hazard), 1);
    assert_eq!(presentation.kind_count(WorldObjectKind::Obstacle), 1);
    assert_eq!(
        presentation
            .stable_ids()
            .iter()
            .map(|id| id.raw())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
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
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
    };
    telemetry.validate().unwrap();
    let overlay = telemetry.overlay_lines();
    let inspector = telemetry.inspector_lines();
    assert!(overlay.contains("scores=true"));
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
    assert!(stepped.contains("Creature action EAT toward stable:2"));
    assert!(stepped.contains("Intent line stable:1 -> stable:2"));
    assert!(stepped.contains("Food interaction cue highlighted"));
    assert!(stepped.contains("Patch sealed count=1"));
    assert_eq!(
        panel.player_events.len(),
        alife_game_app::S02_MAX_PLAYER_EVENT_LINES
    );
    assert_eq!(panel.intent_marker_label(), "stable:1 -> stable:2 (EAT)");
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
    assert!(controls.contains("Left click select"));
    assert!(controls.contains("Space run/pause"));
    assert!(controls.contains("N step"));
    assert!(controls.contains("R reset"));
    assert!(controls.contains("+/- zoom"));
    assert!(controls.contains("F follow selected stable ID"));
    assert!(controls.contains("[!] hazard"));
    assert!(controls.contains("Stable IDs only"));
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
    assert!(overlay.contains("Stable ID: 2 (mapped)"));
    assert!(overlay.contains("Energy "));
    assert!(overlay.contains("Health "));
    assert!(overlay.contains("Hunger "));
    assert!(overlay.contains("Fatigue"));
    assert!(overlay.contains("Fear   "));
    assert!(overlay.contains("Learning: H_shadow="));
    assert!(overlay.contains("Cam:"));
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

    assert_eq!(panel.intent_marker_label(), "stable:1 -> stable:2 (EAT)");
    assert!(panel
        .status_overlay_text()
        .contains("Intent: stable:1 -> stable:2 (EAT)"));
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
    assert_eq!(badges[0].0 .0.as_str(), "Action: FLEE");

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
    summary.validate().unwrap();
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
    assert!(summary
        .world_signature
        .iter()
        .any(|line| line.contains("teacher-word-food")));
    summary.validate().unwrap();
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
    assert!(legend.contains("[#] obstacle"));
    assert!(legend.contains("P34 remains guide-only"));
    assert!(legend.contains("creature+food+real hazard+obstacle"));
    assert!(legend.contains("presentation only"));

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
        vec![
            WorldEntityId(1),
            WorldEntityId(2),
            WorldEntityId(3),
            WorldEntityId(4)
        ]
    );
    assert!(summary.overlay_text.contains("Save / Load"));
    assert!(summary.overlay_text.contains("F5 save"));
    assert!(summary.overlay_text.contains("F9 load"));
    assert!(summary.overlay_text.contains("Stable IDs: [1, 2, 3, 4]"));
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
    assert!(closed.contains("Stable IDs [1, 2, 3, 4]"));
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
    assert!(overlay.contains("Stable IDs: [1, 2, 3, 4]"));
    assert!(overlay.contains("Boundary: stable IDs only"));
    assert!(!overlay.contains("Entity("));

    let controls = alife_game_app::bevy_shell::ca05_controls_bar_text();
    assert!(controls.contains("M save/load"));
    assert!(controls.contains("F5 save"));
    assert!(controls.contains("F9 load"));
}
