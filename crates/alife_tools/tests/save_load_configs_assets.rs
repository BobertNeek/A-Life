use std::{path::PathBuf, process::Command};

#[test]
fn p34_fixture_validator_accepts_committed_fixture_bundle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../alife_world/tests/fixtures/p34")
        .canonicalize()
        .expect("fixture root exists");
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
