#![cfg(feature = "cpu-backend")]
use std::{fs, path::Path, process::Command};
fn run(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_train_cpu_foundation"))
        .env("VK_ICD_FILENAMES", "/no-vulkan-icd.json")
        .args(args)
        .output()
        .unwrap()
}
fn text(path: &Path) -> &str {
    path.to_str().unwrap()
}
#[test]
fn executable_continues_native_weights_and_ages_and_rejects_incomplete_source() {
    let root = std::env::temp_dir().join(format!(
        "alife-cpu-cli-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    let first = root.join("first");
    let second = root.join("second");
    let result = run(&[
        "--fixture",
        "--output",
        text(&first),
        "--seed",
        "799977",
        "--ticks",
        "6",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let names = [
        "phase.txt",
        "cpu-cycle.json",
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ];
    let before = names.map(|name| fs::read(first.join(name)).unwrap());
    let result = run(&[
        "--previous",
        text(&first),
        "--output",
        text(&second),
        "--seed",
        "799978",
        "--ticks",
        "6",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for (name, bytes) in names.iter().zip(&before) {
        assert_eq!(&fs::read(first.join(name)).unwrap(), bytes);
    }
    let a: serde_json::Value = serde_json::from_slice(&before[1]).unwrap();
    let b: serde_json::Value =
        serde_json::from_slice(&fs::read(second.join("cpu-cycle.json")).unwrap()).unwrap();
    assert_eq!(b["initial_actor_optimizer_step"], a["actor_optimizer_step"]);
    assert_eq!(b["initial_value_optimizer_step"], a["value_optimizer_step"]);
    assert_eq!(b["old_asset_digest"], a["new_asset_digest"]);
    assert_eq!(b["fixture"], true);
    assert_eq!(b["objective_state_reset"], false);
    assert!(b["canonical_valid_decisions"].as_u64().unwrap() > 0);
    assert!(
        b["actor_optimizer_step"].as_u64().unwrap() > a["actor_optimizer_step"].as_u64().unwrap()
    );
    let rejected = root.join("rejected");
    fs::write(second.join("phase.txt"), "collecting-canonical-cpu").unwrap();
    let result = run(&[
        "--previous",
        text(&second),
        "--output",
        text(&rejected),
        "--seed",
        "799979",
        "--ticks",
        "6",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("completed"));
    assert!(!rejected.exists());
    fs::remove_dir_all(root).unwrap();
}
