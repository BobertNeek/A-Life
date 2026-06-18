use alife_core::{
    ActionKind, ActionProposal, ActionTarget, BrainTickInput, BrainTickStatus, Confidence,
    CreatureMind, DurationTicks, NormalizedScalar, OrganismId, Tick, Validate, WorldEntityId,
};
use alife_game_app::{
    compare_visible_world_to_headless, g17_feedback_manifest_path, g17_workspace_root,
    load_visible_world_from_p34_save, project_lod_without_behavior_change,
    run_cognition_debug_timeline_smoke, run_creature_inspector_smoke, run_creature_visual_smoke,
    run_feedback_polish_smoke, run_gpu_product_hardening_smoke, run_headless_app_shell_smoke,
    run_lifecycle_lineage_smoke, run_live_brain_loop_paused_smoke, run_live_brain_loop_smoke,
    run_longrun_balance_smoke, run_longrun_balance_with_config, run_playable_survival_loop_smoke,
    run_population_performance_lod_smoke, run_population_social_loop_smoke, run_save_load_ux_smoke,
    run_school_mode_smoke, run_semantic_provider_smoke, run_world_ecology_loop_smoke,
    run_world_editor_smoke, select_visible_world_entity, validate_app_shell_config,
    AppShellLaunchConfig, AutosavePolicy, CadenceTarget, CameraNavigationState, ConfigMenuState,
    CreatureAnimationState, CreatureExpressionState, CreatureLifeStage, FeedbackAssetKind,
    FeedbackAssetManifest, FeedbackEventKind, InspectorControlPanel, LifecycleEventKind,
    LifecycleLiveLoop, LifecycleLoopConfig, LifecycleSaveState, LiveBrainLoop,
    LiveBrainTickControl, LodResidency, LongRunBalanceConfig, PlayableSurvivalEventKind,
    PopulationLiveLoop, PopulationLoopConfig, PopulationPerformancePolicy,
    PopulationSocialEventKind, RenderDetailLevel, SaveSlotDescriptor, SaveSlotKind,
    SaveSlotManager, SchoolModeSaveState, WorldEditCommand, WorldEditorConfig, WorldEditorMode,
    WorldEditorSession,
};
use alife_world::persistence::{BackendSelection, PortableSaveFile, RuntimeConfig};
use alife_world::WorldObjectKind;
use alife_world::{
    EcologyZoneId, HeadlessActionIds, HeadlessBrainHarness, HeadlessScenarioBuilder, TerrainZone,
    TerrainZoneKind,
};
use std::path::PathBuf;

fn p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/alife_world/tests/fixtures/p34")
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
    assert!(summary
        .report_markdown
        .contains("Known degenerate behaviors"));
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
