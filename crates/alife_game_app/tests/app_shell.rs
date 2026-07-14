use std::path::PathBuf;

use alife_core::PolicyBackend;
use alife_game_app::{
    run_headless_app_shell_smoke, AppShellLaunchConfig, GpuBrainAuthorityTelemetry,
    GraphicalBrainPolicyMode, GraphicalPlaygroundLaunchConfig, ProductionFrontendProfileId,
    ProductionVoxelLaunchConfig,
};

fn p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34")
}

#[test]
fn headless_smoke_requires_an_explicit_heuristic_baseline() {
    let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root())
        .with_brain_policy(PolicyBackend::HeuristicBaseline);
    let summary = run_headless_app_shell_smoke(&launch).unwrap();
    assert_eq!(summary.requested_backend, PolicyBackend::HeuristicBaseline);
    assert!(!summary.graphics_required_for_default_path);
}

#[test]
fn graphical_product_default_is_gpu_required() {
    let launch = GraphicalPlaygroundLaunchConfig::interactive(p34_fixture_root());
    assert_eq!(launch.brain_policy, PolicyBackend::NeuralClosedLoopGpu);
    assert_eq!(launch.gpu_mode, GraphicalBrainPolicyMode::GpuRequired);
    assert!(launch.brain_policy.requires_gpu());
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
        ..GpuBrainAuthorityTelemetry::pending("N1024")
    };
    let text = telemetry.overlay_text();
    for required in [
        "GPU neural: authoritative",
        "Adapter: NVIDIA GeForce RTX 3050",
        "Phenotype: 7f3a91c2",
        "Class: N1024",
        "Selected: candidate 3  logit +0.742",
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
