use std::path::PathBuf;

use alife_core::PolicyBackend;
use alife_game_app::{
    run_headless_app_shell_smoke, run_lifecycle_lineage_smoke, AppShellLaunchConfig,
    GpuBrainAuthorityTelemetry, GraphicalBrainPolicyMode, GraphicalPlaygroundLaunchConfig,
    ProductionFrontendProfileId, ProductionVoxelLaunchConfig,
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

#[test]
fn lifecycle_lineage_birth_inherits_and_mutates_appearance_genes() {
    let summary = run_lifecycle_lineage_smoke().unwrap();
    let birth = &summary.lineage_records[0];
    let offspring = summary
        .creatures
        .iter()
        .find(|creature| creature.genome_id == birth.offspring_genome_id)
        .expect("G09 smoke should create one offspring");
    let parents = birth
        .parent_genome_ids
        .iter()
        .map(|parent_genome| {
            summary
                .creatures
                .iter()
                .find(|creature| creature.genome_id == *parent_genome)
                .expect("parent genome should stay visible in lifecycle summary")
        })
        .collect::<Vec<_>>();

    let part_catalog = alife_game_app::load_production_creature_part_catalog().unwrap();
    assert!(alife_game_app::part_sources_are_ordinary_compatible(
        &offspring.appearance.part_sources,
        &part_catalog
    ));
    assert!(
        offspring.appearance.part_sources.torso == parents[0].appearance.part_sources.torso
            || offspring.appearance.part_sources.torso == parents[1].appearance.part_sources.torso,
        "the inherited torso remains the assembly compatibility frame"
    );
    for ((slot, child_family), (_, parent_a_family), (_, parent_b_family)) in offspring
        .appearance
        .part_sources
        .iter_slots()
        .into_iter()
        .zip(parents[0].appearance.part_sources.iter_slots())
        .zip(parents[1].appearance.part_sources.iter_slots())
        .map(|((child, parent_a), parent_b)| (child, parent_a, parent_b))
    {
        if child_family != parent_a_family && child_family != parent_b_family {
            assert_ne!(
                slot,
                alife_world::CreaturePartSlotKey::Torso,
                "catalog normalization may only substitute attached parts"
            );
        }
    }
    assert!(
        offspring.appearance.mutation_count
            > parents[0]
                .appearance
                .mutation_count
                .max(parents[1].appearance.mutation_count),
        "offspring appearance should allow simple mutation across generations"
    );
    assert_ne!(
        offspring.appearance.signature_line(),
        parents[0].appearance.signature_line()
    );
    assert_ne!(
        offspring.appearance.signature_line(),
        parents[1].appearance.signature_line()
    );
}
