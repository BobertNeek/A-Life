use std::path::PathBuf;

use alife_core::PolicyBackend;
use alife_game_app::{
    load_geneforge_creature_part_catalog, resolve_creature_part_display_sources,
    validate_app_shell_config, AppShellLaunchConfig, DoubleBufferedGraphicalScheduler,
    GpuBrainAuthorityTelemetry, GraphicalBrainPolicyMode, ProductionFrontendProfileId,
    ProductionVoxelLaunchConfig, RuntimePlaybackState,
};
use alife_world::{
    CreatureAppearanceGenome, CreaturePartFamilyId, CreaturePartSlotKey, CreaturePartSources,
};

#[test]
fn checkpoint_wait_preserves_only_unspent_planned_scheduler_debt() {
    let mut scheduler = DoubleBufferedGraphicalScheduler::default();
    let plan = scheduler
        .observe_render_frame(1.0, RuntimePlaybackState::Running, 1)
        .unwrap();
    let after_plan = scheduler.accumulator_micros;
    let completed_before = scheduler.ticks_executed;
    let unspent = plan.ticks_to_run - 1;

    scheduler.preserve_unspent_planned_ticks(unspent).unwrap();

    let restored = scheduler
        .config
        .fixed_tick_micros()
        .saturating_mul(u64::from(unspent));
    assert_eq!(
        scheduler.accumulator_micros,
        after_plan
            .saturating_add(restored)
            .min(scheduler.config.max_accumulator_micros)
    );
    assert_eq!(scheduler.ticks_executed, completed_before);
}

#[test]
fn production_catch_up_pacing_preserves_unspent_bounded_debt() {
    let mut scheduler = DoubleBufferedGraphicalScheduler::default();
    let plan = scheduler
        .observe_render_frame(1.0, RuntimePlaybackState::Running, 1)
        .unwrap();
    let after_plan = scheduler.accumulator_micros;

    let (ticks_to_run, deferred) = scheduler.pace_planned_ticks(plan.ticks_to_run, 1).unwrap();

    assert_eq!(ticks_to_run, 1);
    assert_eq!(deferred, 3);
    assert_eq!(
        scheduler.accumulator_micros,
        after_plan
            .saturating_add(scheduler.config.fixed_tick_micros() * u64::from(deferred))
            .min(scheduler.config.max_accumulator_micros)
    );
    assert_eq!(scheduler.ticks_executed, 0);
}

fn p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34")
}

#[test]
fn config_preflight_accepts_an_explicit_heuristic_baseline() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root())
        .with_brain_policy(PolicyBackend::HeuristicBaseline);
    validate_app_shell_config(&launch).unwrap();
}

#[test]
fn graphical_product_default_is_gpu_required() {
    let launch = ProductionVoxelLaunchConfig::from_manifest(
        alife_game_app::default_environment_manifest_path(),
        None,
        ProductionFrontendProfileId::MinSpecComfort1080p,
    )
    .unwrap();
    assert_eq!(
        launch.app_launch.brain_policy,
        PolicyBackend::NeuralClosedLoopGpu
    );
    assert_eq!(launch.gpu_mode, GraphicalBrainPolicyMode::GpuRequired);
    assert!(launch.app_launch.brain_policy.requires_gpu());
}

#[test]
fn authority_overlay_contains_the_blueprint_fields_without_a_switching_status() {
    let telemetry = GpuBrainAuthorityTelemetry {
        authoritative: true,
        adapter: "NVIDIA GeForce RTX 3050".to_string(),
        phenotype_hash_prefix: "7f3a91c2".to_string(),
        capacity_class: "N1024".to_string(),
        selected_candidate: Some(3),
        selected_logit: Some(0.742),
        checkpoint_tick: Some(18_432),
        checkpoint_sleep_phase: "Consolidating".to_string(),
        checkpoint_consolidation_state: "Submitted".to_string(),
        recovery_status: "GPU required".to_string(),
        ..GpuBrainAuthorityTelemetry::pending("N1024")
    };
    let text = telemetry.overlay_text();
    for required in [
        "GPU neural: authoritative",
        "Adapter: NVIDIA GeForce RTX 3050",
        "Phenotype: 7f3a91c2",
        "Class: N1024",
        "Selected: candidate 3  logit +0.742",
        "GPU BRAIN CHECKPOINT",
        "Checkpoint tick: 18432",
        "Sleep phase: Consolidating",
        "Consolidation: Submitted",
        "Recovery: GPU required",
        "Failure policy: stop learned actions",
    ] {
        assert!(
            text.contains(required),
            "missing {required:?} from {text:?}"
        );
    }
}

#[test]
fn production_developer_overlay_is_opt_in() {
    let manifest = alife_game_app::default_environment_manifest_path();
    let launch = ProductionVoxelLaunchConfig::from_manifest(
        manifest,
        None,
        ProductionFrontendProfileId::MinimumSettings30x30,
    )
    .unwrap();
    assert!(!launch.developer_overlay);
    assert_eq!(launch.gpu_mode, GraphicalBrainPolicyMode::GpuRequired);
    assert_eq!(
        launch.app_launch.brain_policy,
        PolicyBackend::NeuralClosedLoopGpu
    );
}

#[test]
fn offspring_appearance_inherits_and_mutates_compatible_parts() {
    let mut parent_a = CreatureAppearanceGenome::founder_for_species(0, 101);
    parent_a.part_sources = CreaturePartSources {
        head: CreaturePartFamilyId(8),
        torso: CreaturePartFamilyId(11),
        arms: CreaturePartFamilyId(9),
        legs: CreaturePartFamilyId(10),
        tail: CreaturePartFamilyId(11),
    };
    let mut parent_b = CreatureAppearanceGenome::founder_for_species(1, 202);
    parent_b.part_sources = CreaturePartSources {
        head: CreaturePartFamilyId(11),
        torso: CreaturePartFamilyId(11),
        arms: CreaturePartFamilyId(10),
        legs: CreaturePartFamilyId(8),
        tail: CreaturePartFamilyId(9),
    };
    let child = CreatureAppearanceGenome::offspring_from_parents(parent_a, parent_b, 303);
    assert!(child.inherited_from(parent_a, parent_b));

    child.validate().unwrap();

    assert_eq!(child.part_sources.torso, CreaturePartFamilyId(11));
    for ((slot, child_family), (_, parent_a_family), (_, parent_b_family)) in child
        .part_sources
        .iter_slots()
        .into_iter()
        .zip(parent_a.part_sources.iter_slots())
        .zip(parent_b.part_sources.iter_slots())
        .map(|((child, parent_a), parent_b)| (child, parent_a, parent_b))
    {
        if child_family != parent_a_family && child_family != parent_b_family {
            assert_ne!(slot, CreaturePartSlotKey::Torso);
        }
        assert!(child_family.0 <= 11);
    }
    assert!(child.mutation_count > parent_a.mutation_count.max(parent_b.mutation_count));
    assert_ne!(child.signature_line(), parent_a.signature_line());
    assert_ne!(child.signature_line(), parent_b.signature_line());
}

#[test]
fn display_resolution_keeps_unknown_saved_part_id_byte_for_byte() {
    let mut appearance = CreatureAppearanceGenome::founder_for_species(0, 73);
    appearance.part_sources.head = CreaturePartFamilyId(999);
    let before = serde_json::to_vec(&appearance).unwrap();

    let display = resolve_creature_part_display_sources(
        appearance.part_sources,
        &load_geneforge_creature_part_catalog().unwrap(),
    )
    .unwrap();

    assert!(display.warning.is_some());
    assert_eq!(display.saved_sources.head, CreaturePartFamilyId(999));
    assert_eq!(
        display.displayed_sources.head,
        display.displayed_sources.torso
    );
    assert!(display.display_only);
    assert!(display.warning.unwrap().visible_message().contains("999"));
    assert_eq!(serde_json::to_vec(&appearance).unwrap(), before);
    let roundtrip: CreatureAppearanceGenome = serde_json::from_slice(&before).unwrap();
    assert_eq!(roundtrip.part_sources.head, CreaturePartFamilyId(999));
    assert_eq!(serde_json::to_vec(&roundtrip).unwrap(), before);
}
