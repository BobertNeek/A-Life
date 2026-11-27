//! Asset-free qualification harness: exact existing World/adapter Rust source, no saved life or GPU.
#[cfg(feature = "contact211-rank32-crossed")]
use sha2::{Digest, Sha256};

#[cfg(feature = "contact211-rank32-crossed")]
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(feature = "contact211-rank32-crossed")]
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// Exercise the very same adapter and its four real CPU tests, not a model of the adapter.
// This integration test avoids compiling the unrelated app-wide #[cfg(test)] asset includes.
#[cfg(feature = "contact211-rank32-crossed")]
#[path = "../src/foundation_training_cycle/saved_life_retention/language_pilot.rs"]
mod language_pilot;

#[cfg(not(feature = "contact211-rank32-crossed"))]
#[test]
fn actual_shared_admission_refuses_crossed_while_off_without_checkpoint() {
    use std::{fs, time::{SystemTime, UNIX_EPOCH}};
    let path = std::env::temp_dir().join(format!("crossed-cpu-refusal-{}-{}",
        std::process::id(), SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
    fs::create_dir(&path).unwrap();
    let root = path.canonicalize().unwrap();
    let manifest = root.join("experiment.json");
    let nonexistent_checkpoint = root.join("no-input-checkpoint");
    let output = root.join("must-remain-absent");
    fs::write(&manifest, b"{\"schema\":8}\n").unwrap();
    let result = alife_game_app::evaluate_saved_life_retention_experiment(
        &root, &nonexistent_checkpoint, &output, &manifest);
    assert_eq!(result.unwrap_err().to_string(), "Crossed Rank32 feature is disabled");
    assert!(!nonexistent_checkpoint.exists() && !output.exists());
    fs::remove_file(manifest).unwrap();
    fs::remove_dir(root).unwrap();
}
