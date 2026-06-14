use std::path::{Path, PathBuf};

use alife_tools::p35_playground::{
    run_gpu_fallback_demo, run_headless_cpu_demo, run_save_load_demo, run_school_teacher_demo,
    run_semantic_fake_provider_demo, validate_playground_manifest, PlaygroundExampleConfig,
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_tools should live under crates/")
        .to_path_buf()
}

fn p34_fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34")
}

#[test]
fn headless_cpu_playground_smoke_loads_p34_config_and_emits_patch_log_summary() {
    let config = PlaygroundExampleConfig::from_p34_fixture_root(p34_fixture_root()).unwrap();

    let report = run_headless_cpu_demo(config).unwrap();

    assert_eq!(report.seed, 4242);
    assert_eq!(report.backend_selected, "CpuReference");
    assert!(report.sealed_patch_count >= 1);
    assert!(report.packed_log_count >= 1);
    assert!(report
        .world_signature
        .iter()
        .any(|line| line.contains("berry")));
    assert!(report.drive_hormone_debug.contains("hunger"));
    assert!(report.action_debug.contains("Interact") || report.action_debug.contains("Rest"));
}

#[test]
fn save_load_demo_uses_stable_ids_and_rejects_engine_local_persistence() {
    let report = run_save_load_demo(p34_fixture_root()).unwrap();

    assert_eq!(report.save_id, "tiny-p34-fixture");
    assert_eq!(report.seed, 4242);
    assert!(report.world_entity_count >= 2);
    assert!(report.stable_id_remap_available);
    assert!(!report.engine_local_ids_serialized);
}

#[test]
fn school_demo_dispatches_perception_only_events_and_verifies_sealed_patches() {
    let report = run_school_teacher_demo().unwrap();

    assert!(report.perception_event_count > 0);
    assert!(report.verifier_passed);
    assert!(!report.direct_motor_bypass);
    assert!(!report.hidden_vector_injection);
}

#[test]
fn semantic_demo_tolerates_missing_provider_and_uses_fake_provider_when_enabled() {
    let report = run_semantic_fake_provider_demo().unwrap();

    assert!(report.missing_provider_tolerated);
    #[cfg(feature = "semantic-demo")]
    assert!(report.fake_provider_context_available);
    #[cfg(not(feature = "semantic-demo"))]
    assert!(!report.fake_provider_context_available);
    assert!(!report.provider_required_for_core_path);
}

#[test]
fn gpu_demo_falls_back_to_cpu_and_keeps_diagnostics_boundary_scoped() {
    let report = run_gpu_fallback_demo().unwrap();

    assert_eq!(report.requested_backend, "GpuStatic");
    assert_eq!(report.selected_backend, "CpuReference");
    assert!(report.cpu_fallback);
    assert!(!report.active_bulk_readback_allowed);
    assert!(report.diagnostic_export_boundary_allowed);
}

#[test]
fn playground_docs_manifest_references_existing_small_files_and_manual_optional_demos() {
    let root = workspace_root();
    let report =
        validate_playground_manifest(root.join("examples/p35/playground_manifest.json")).unwrap();

    assert!(report.checked_paths >= 4);
    assert!(report.manual_optional_commands >= 2);
    assert!(report.largest_committed_sample_bytes < 64 * 1024);
    assert!(report
        .documented_commands
        .iter()
        .any(|command| command.contains("p35_playground")));
}
