use alife_core::{
    ActionKind, ActionProposal, ActionTarget, BrainTickInput, BrainTickStatus, Confidence,
    CreatureMind, DurationTicks, NormalizedScalar, OrganismId, Tick, WorldEntityId,
};
use alife_game_app::{
    compare_visible_world_to_headless, load_visible_world_from_p34_save,
    run_creature_inspector_smoke, run_creature_visual_smoke, run_headless_app_shell_smoke,
    run_live_brain_loop_paused_smoke, run_live_brain_loop_smoke, run_playable_survival_loop_smoke,
    select_visible_world_entity, validate_app_shell_config, AppShellLaunchConfig,
    CameraNavigationState, CreatureAnimationState, CreatureExpressionState, InspectorControlPanel,
    LiveBrainLoop, LiveBrainTickControl, PlayableSurvivalEventKind,
};
use alife_world::persistence::{BackendSelection, PortableSaveFile};
use alife_world::WorldObjectKind;
use alife_world::{HeadlessActionIds, HeadlessBrainHarness};
use std::path::PathBuf;

fn p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../crates/alife_world/tests/fixtures/p34")
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
