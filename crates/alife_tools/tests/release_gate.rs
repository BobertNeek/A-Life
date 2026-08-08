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
fn development_guide_lists_required_gates_and_windows_wrappers() {
    let development = read_workspace_file("docs/DEVELOPMENT.md");

    for required in [
        "cargo fmt --all -- --check",
        "cargo check --workspace --all-targets",
        "cargo test --workspace --all-targets",
        "cargo clippy --workspace --all-targets -- -D warnings",
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1",
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check_core_boundaries.ps1",
        "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/docs_check.ps1",
        "cargo check --workspace --all-features --all-targets",
        "cargo test --workspace --all-features --all-targets",
        "cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime",
    ] {
        assert!(
            development.contains(required),
            "development guide missing gate command: {required}"
        );
    }
    assert!(development.contains("manual"));
    assert!(development.contains("target/artifacts/"));
}

#[test]
fn status_records_honest_gpu_product_and_scale_limits() {
    let status = read_workspace_file("docs/STATUS.md");

    for required in [
        "active voxel renderer remains a save-derived projection",
        "Pause currently stops procedural animation, not the GPU tick",
        "Its promotion verdict is `Blocked`",
        "N4096 is research-only",
        "Not ready.",
    ] {
        assert!(
            status.contains(required),
            "current status missing honest limitation: {required}"
        );
    }
}

#[test]
fn evidence_guide_requires_source_bound_physical_gpu_receipts() {
    let evidence = read_workspace_file("docs/EVIDENCE.md");

    for required in [
        "source commit and tree",
        "adapter, backend, driver-visible identity",
        "A report bound to an older source remains valid historical evidence for that source.",
        "Missing hardware is `Unavailable`, not fallback success.",
        "promotion verdict `Blocked`",
    ] {
        assert!(
            evidence.contains(required),
            "evidence guide missing required rule: {required}"
        );
    }
}

#[test]
fn active_docs_do_not_reintroduce_windows_plain_bash_validation_commands() {
    for relative in [
        "README.md",
        "docs/VISION.md",
        "docs/STATUS.md",
        "docs/ARCHITECTURE.md",
        "docs/ROADMAP.md",
        "docs/DEVELOPMENT.md",
        "docs/EVIDENCE.md",
        "docs/REFERENCE.md",
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
}

#[test]
fn current_docs_and_fixture_artifacts_stay_small_and_discoverable() {
    let root = workspace_root();
    for required in [
        "docs/DEVELOPMENT.md",
        "docs/STATUS.md",
        "docs/EVIDENCE.md",
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
