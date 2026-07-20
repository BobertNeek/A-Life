use std::{
    fs,
    path::{Path, PathBuf},
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_tools should live under crates/")
        .to_path_buf()
}

fn read_workspace_file(relative: &str) -> String {
    let path = workspace_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("failed to read {relative}: {error}"))
}

#[test]
fn release_checklist_lists_required_gates_and_windows_wrappers() {
    let checklist = read_workspace_file("docs/release_checklist.md");

    for required in [
        "cargo fmt --all -- --check",
        "cargo check --workspace --all-targets",
        "cargo test --workspace --all-targets",
        "cargo clippy --workspace --all-targets -- -D warnings",
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1",
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1",
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1",
        "cargo tree -p alife_core",
        "cargo check --workspace --all-features --all-targets",
        "cargo test --workspace --all-features --all-targets",
        "cargo test -p alife_world --test golden_traces_determinism",
        "cargo test -p alife_world --test scenario_suite",
        "cargo test -p alife_world --test headless_soak",
        "cargo test -p alife_world --test save_load_roundtrip",
        "cargo test -p alife_tools --test playground_examples",
        "cargo test -p alife_tools --test benchmark_tiers benchmark_tiers_smoke_runs_tier_1_and_10_without_bevy_or_gpu",
        "cargo run -p alife_tools --bin benchmark_tiers",
    ] {
        assert!(
            checklist.contains(required),
            "release checklist missing gate command: {required}"
        );
    }
    assert!(checklist.contains("Manual"));
    assert!(checklist.contains("target/artifacts/"));
}

#[test]
fn final_status_report_records_honest_manual_gpu_and_performance_status() {
    let status = read_workspace_file("docs/final_status_report.md");

    for required in [
        "GPU compute paths are parity/diagnostic contracts",
        "Product GPU performance is not claimed",
        "CPU tiers 50, 100, 250, and 500 are manual expected-slow measurements",
        "The 60 FPS population target remains unknown unless measured",
        "No release blocker is recorded in this report before validation",
        "Backlog Notes",
    ] {
        assert!(
            status.contains(required),
            "final status report missing honest status text: {required}"
        );
    }
}

#[test]
fn gpu_soak_plan_marks_hardware_checks_manual_and_boundary_scoped() {
    let plan = read_workspace_file("docs/gpu_soak_performance_plan.md");

    for required in [
        "Production neural causality is GPU-authoritative",
        "cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored --nocapture",
        "cargo test -p alife_gpu_backend --features gpu-tests --test closed_loop_fast_plasticity -j 1 -- --nocapture",
        "cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime",
        "Unknown is preferable to fabricated data",
        "Neural release and promotion gates require current",
    ] {
        assert!(
            plan.contains(required),
            "GPU soak/performance plan missing required text: {required}"
        );
    }
}

#[test]
fn docs_do_not_reintroduce_windows_plain_bash_validation_commands() {
    for relative in [
        "docs/playground_examples.md",
        "docs/release_checklist.md",
        "docs/final_status_report.md",
        "docs/gpu_soak_performance_plan.md",
    ] {
        let text = read_workspace_file(relative);
        for forbidden in [
            "bash scripts/check.sh",
            "bash scripts/check_core_boundaries.sh",
            "bash scripts/docs_check.sh",
        ] {
            assert!(
                !text.contains(forbidden),
                "{relative} reintroduced ambiguous Windows validation command {forbidden}"
            );
        }
    }

    let validation = read_workspace_file("docs/codex_plan_pack/VALIDATION_PROTOCOL.md");
    assert!(validation.contains("do not run plain `bash scripts/check.sh`"));
    assert!(validation.contains("PowerShell wrappers"));
}

#[test]
fn release_docs_and_fixture_artifacts_stay_small_and_discoverable() {
    let root = workspace_root();
    for required in [
        "docs/release_checklist.md",
        "docs/final_status_report.md",
        "docs/gpu_soak_performance_plan.md",
        "crates/alife_world/tests/fixtures/p34/tiny_save.json",
        "crates/alife_world/tests/fixtures/p34/tiny_config.json",
        "crates/alife_world/tests/fixtures/p34/tiny_asset_manifest.json",
        "examples/p35/playground_manifest.json",
    ] {
        let path = root.join(required);
        assert!(path.is_file(), "missing release-gate artifact {required}");
        let metadata = fs::metadata(&path).expect("release artifact metadata");
        assert!(
            metadata.len() < 128 * 1024,
            "{required} is too large for a committed release-gate fixture"
        );
    }
}
