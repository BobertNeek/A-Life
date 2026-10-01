use std::{path::PathBuf, process::Command};

#[path = "common/current_persistence.rs"]
mod current_persistence;

#[test]
fn p34_fixture_validator_accepts_committed_fixture_bundle() {
    let bundle = current_persistence::current_fixture_workspace();
    let root = bundle.path().join("crates/alife_world/tests/fixtures/p34");
    let binary = std::env::var("CARGO_BIN_EXE_p34_persistence").expect("p34 validator binary path");
    let output = Command::new(binary)
        .arg("validate-fixtures")
        .arg(&root)
        .output()
        .expect("run p34 validator");
    assert!(
        output.status.success(),
        "validator failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn p34_fixture_validator_rejects_legacy_save_missing_authoritative_graph() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../alife_world/tests/fixtures/p34")
        .canonicalize()
        .expect("legacy fixture root exists");
    let binary = std::env::var("CARGO_BIN_EXE_p34_persistence").expect("p34 validator binary path");
    let output = Command::new(binary)
        .arg("validate-fixtures")
        .arg(&root)
        .output()
        .expect("run p34 validator");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains(
        "legacy state is missing authoritative v3 subsystem data: genetic_biochemical_graph"
    ));
}
