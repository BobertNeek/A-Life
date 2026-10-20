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
    let third = root.join("third");
    let result = run(&[
        "--previous",
        text(&second),
        "--output",
        text(&third),
        "--seed",
        "799979",
        "--ticks",
        "4",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let c_bytes = fs::read(third.join("cpu-cycle.json")).unwrap();
    let c: serde_json::Value = serde_json::from_slice(&c_bytes).unwrap();
    assert!(c.get("source_receipt").is_none());
    assert!(c["seed_provenance"]["native_source_receipt"]
        .get("source_receipt")
        .is_none());
    assert_eq!(c["initial_actor_optimizer_step"], b["actor_optimizer_step"]);
    assert!(c_bytes.len() < 2 * serde_json::to_vec_pretty(&b).unwrap().len());
    // Synthetic group-output import fixture: these native bytes prove backend
    // continuity only; this test does not claim fifty GPU-collected organisms.
    let group = root.join("synthetic-group-source");
    fs::create_dir(&group).unwrap();
    for name in [
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ] {
        fs::copy(third.join(name), group.join(name)).unwrap();
    }
    let mut group_receipt = c.clone();
    group_receipt.as_object_mut().unwrap().remove("fixture");
    group_receipt["schema"] = 1.into();
    group_receipt["mode"] = "group50_shared_habitat".into();
    group_receipt["population"] = 50.into();
    group_receipt["next_cohort_optimizer_rebound"] = true.into();
    group_receipt["actor_checkpoint_sha256"] =
        c["native_output_sha256"]["actor-checkpoint.json"].clone();
    group_receipt["value_checkpoint_sha256"] =
        c["native_output_sha256"]["value-checkpoint.json"].clone();
    fs::write(
        group.join("group50.json"),
        serde_json::to_vec(&group_receipt).unwrap(),
    )
    .unwrap();
    let back = root.join("group-back-to-cpu");
    let result = run(&[
        "--previous",
        text(&group),
        "--output",
        text(&back),
        "--seed",
        "799980",
        "--ticks",
        "4",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let d: serde_json::Value =
        serde_json::from_slice(&fs::read(back.join("cpu-cycle.json")).unwrap()).unwrap();
    assert_eq!(d["fixture"], true);
    assert_eq!(d["initial_actor_optimizer_step"], c["actor_optimizer_step"]);
    assert_eq!(d["initial_value_optimizer_step"], c["value_optimizer_step"]);
    assert!(d["seed_provenance"]["source_sha256"]
        .get("group50.json")
        .is_some());
    assert_eq!(d["seed_admission"]["historical_freshness"], "unknown");
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
