use super::*;
use crate::inspect_foundation_checkpoint;

pub(crate) struct Fixture {
    pub root: PathBuf,
    pub source: PathBuf,
    pub target: PathBuf,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

fn write_json(path: &Path, data: &serde_json::Value) {
    std::fs::write(path, serde_json::to_vec(data).unwrap()).unwrap();
}

fn edit(path: &Path, change: impl FnOnce(&mut serde_json::Value)) {
    let mut data = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    change(&mut data);
    write_json(path, &data);
}

pub(crate) fn fixture() -> Fixture {
    fixture_with_residual(None, 0)
}

pub(crate) fn residual_fixture() -> Fixture {
    fixture_with_residual(Some(b"{\"optimizer_updates\":2}"), 0)
}

pub(crate) fn full_residual_correction_fixture() -> Fixture {
    fixture_with_residual(Some(b"{\"optimizer_updates\":2}"), 1)
}
fn fixture_with_residual(residual: Option<&[u8]>, critic_objective: u16) -> Fixture {
    let source = crate::foundation_archived_success_replay::tests::checkpoint_fixture(5124);
    let root = source.with_file_name(format!(
        "{}-correction",
        source.file_name().unwrap().to_str().unwrap()
    ));
    std::fs::create_dir(&root).unwrap();
    let preserved = root.join("source");
    std::fs::rename(source, &preserved).unwrap();
    let source = preserved;
    if critic_objective == 1 {
        edit(&source.join("cycle.json"), |data| {
            data["critic_objective_version"] = 1.into();
        });
    }
    if let Some(bytes) = residual {
        std::fs::write(source.join(RESIDUAL_UPDATE), bytes).unwrap();
        edit(&source.join("cycle.json"), |data| {
            data["critic_residual_update_sha256"] = sha(bytes).into();
        });
    }
    std::fs::create_dir(source.join("nested")).unwrap();
    std::fs::write(source.join("nested/preserved.bin"), b"exact acquired state").unwrap();
    let target = root.join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::create_dir(target.join("nested")).unwrap();
    let source_files = snapshot(&source).unwrap();
    for name in source_files.keys() {
        std::fs::copy(source.join(name), target.join(name)).unwrap();
    }
    edit(&target.join("value-checkpoint.json"), |data| {
        data["optimizer_step"] = 6.into();
        data["parameters"][0] = 0.25.into();
        // Nonzero carried Adam lanes make moment preservation observable.
        let width = data["feature_count"].as_u64().unwrap() as usize + 1;
        data["parameters"][width] = 0.125.into();
        data["parameters"][2 * width] = 0.015625.into();
    });
    let manifest_path = root.join("RECOVERY-PRESERVED.json");
    write_json(
        &manifest_path,
        &serde_json::json!({
            "latest_clone": source, "latest_all_file_sha256": source_files,
        }),
    );
    let evidence = root.join("historical.json");
    write_json(&evidence, &serde_json::json!({"live_qualified": false}));
    let marker = serde_json::json!({
        "schema_version": 1, "purpose": "critic_only_correction_candidate_clone_validation",
        "created_utc": "2026-10-17T12:00:00Z", "source_directory": source,
        "source_file_sha256": source_files,
        "target_value_checkpoint_sha256": sha(&std::fs::read(target.join("value-checkpoint.json")).unwrap()),
        "source_policy_version": 5124, "actor_optimizer_step": 1,
        "source_value_optimizer_step": 1, "value_optimizer_step": 6,
        "critic_only_updates": 5, "optimizer_reset": false, "value_state_reset": false,
        "biological_objective_version": 2, "cycle_receipt_unchanged": true,
        "no_PPO_cycles_claimed": true, "live_qualified": false,
        "qualification_evidence_sha256": {evidence.to_str().unwrap(): sha(&std::fs::read(&evidence).unwrap())},
        "correction_objective": "full sampled Bellman residual, not Monte Carlo",
        "source_recovery_manifest_sha256": sha(&std::fs::read(manifest_path).unwrap()),
    });
    write_json(&target.join(MARKER), &marker);
    Fixture {
        root,
        source,
        target,
    }
}

fn rejected_unchanged(f: &Fixture) {
    let before = snapshot(&f.target).unwrap();
    assert!(inspect_foundation_checkpoint(&f.target, 5124).is_err());
    assert_eq!(snapshot(&f.target).unwrap(), before);
}

#[test]
fn critic_correction_preserves_original_bytes_and_admits_actual_critic_age() {
    let f = fixture();
    let source_before = snapshot(&f.source).unwrap();
    let target_before = snapshot(&f.target).unwrap();
    let ordinary = inspect_foundation_checkpoint(&f.source, 5124).unwrap();
    let corrected = inspect_foundation_checkpoint(&f.target, 5124).unwrap();
    assert_eq!(
        (ordinary.actor_optimizer_step, ordinary.value_optimizer_step),
        (1, 1)
    );
    assert_eq!(
        (
            corrected.actor_optimizer_step,
            corrected.value_optimizer_step
        ),
        (1, 6)
    );
    assert!(!corrected.neural_execution && !corrected.weights_modified);
    assert_eq!(snapshot(&f.source).unwrap(), source_before);
    assert_eq!(snapshot(&f.target).unwrap(), target_before);
    assert_eq!(
        std::fs::read(f.source.join("cycle.json")).unwrap(),
        std::fs::read(f.target.join("cycle.json")).unwrap()
    );
}

#[test]
fn critic_correction_rejects_every_preserved_actor_lane_and_cycle_mutation() {
    let f = fixture();
    let path = f.target.join("actor-checkpoint.json");
    let original = std::fs::read(&path).unwrap();
    for field in ["weights", "first_moment", "second_moment", "update_ages"] {
        edit(&path, |data| {
            data[field][0] = 0.5.into();
        });
        rejected_unchanged(&f);
        std::fs::write(&path, &original).unwrap();
    }
    edit(&f.target.join("cycle.json"), |data| {
        data["value_optimizer_step"] = 6.into();
    });
    rejected_unchanged(&f);
}

#[test]
fn critic_correction_rejects_false_reset_qualification_count_and_identity_claims() {
    let f = fixture();
    let path = f.target.join(MARKER);
    let original = std::fs::read(&path).unwrap();
    for (field, value) in [
        ("optimizer_reset", true.into()),
        ("value_state_reset", true.into()),
        ("live_qualified", true.into()),
        ("no_PPO_cycles_claimed", false.into()),
        ("cycle_receipt_unchanged", false.into()),
        ("critic_only_updates", 0.into()),
        ("critic_only_updates", 5001.into()),
        ("critic_only_updates", 4.into()),
        ("source_policy_version", 5125.into()),
        ("actor_optimizer_step", 2.into()),
        ("source_value_optimizer_step", 2.into()),
        ("value_optimizer_step", 7.into()),
        ("biological_objective_version", 1.into()),
    ] {
        edit(&path, |data| {
            data[field] = value;
        });
        rejected_unchanged(&f);
        std::fs::write(&path, &original).unwrap();
    }
}

#[test]
fn critic_correction_rejects_wrong_value_age_policy_shape_and_invalid_adam() {
    let f = fixture();
    let path = f.target.join("value-checkpoint.json");
    let original = std::fs::read(&path).unwrap();
    for lane in ["age", "policy", "shape", "variance"] {
        edit(&path, |data| match lane {
            "age" => data["optimizer_step"] = 7.into(),
            "policy" => data["last_updated_policy_version"] = 5125.into(),
            "shape" => data["feature_count"] = 1024.into(),
            _ => {
                let n = data["parameters"].as_array().unwrap().len();
                data["parameters"][n - 1] = (-0.1).into();
            }
        });
        // Updating the declared target digest cannot bypass native validation.
        edit(&f.target.join(MARKER), |data| {
            data["target_value_checkpoint_sha256"] = sha(&std::fs::read(&path).unwrap()).into();
        });
        rejected_unchanged(&f);
        std::fs::write(&path, &original).unwrap();
    }
}

#[test]
fn critic_correction_rejects_source_evidence_manifest_and_relative_path_tampering() {
    let f = fixture();
    let marker = f.target.join(MARKER);
    let original = std::fs::read(&marker).unwrap();
    for field in [
        "source_recovery_manifest_sha256",
        "target_value_checkpoint_sha256",
    ] {
        edit(&marker, |data| {
            data[field] = "0".repeat(64).into();
        });
        rejected_unchanged(&f);
        std::fs::write(&marker, &original).unwrap();
    }
    for name in [
        "../outside",
        "nested/../../outside",
        "C:\\outside",
        "nested:stream",
        "/outside",
        "nested//bad",
    ] {
        edit(&marker, |data| {
            data["source_file_sha256"][name] = "0".repeat(64).into();
        });
        rejected_unchanged(&f);
        std::fs::write(&marker, &original).unwrap();
    }
    std::fs::write(f.root.join("historical.json"), b"changed evidence").unwrap();
    rejected_unchanged(&f);
    std::fs::write(
        f.source.join("nested/preserved.bin"),
        b"changed acquired state",
    )
    .unwrap();
    rejected_unchanged(&f);
}

#[test]
fn critic_correction_rejects_nested_derivative_missing_extra_and_unreceipted_value() {
    let f = fixture();
    std::fs::write(f.source.join(MARKER), b"{}").unwrap();
    rejected_unchanged(&f);
    std::fs::remove_file(f.source.join(MARKER)).unwrap();
    std::fs::write(f.target.join("extra"), b"unexpected").unwrap();
    rejected_unchanged(&f);
    std::fs::remove_file(f.target.join("extra")).unwrap();
    std::fs::remove_file(f.target.join("nested/preserved.bin")).unwrap();
    rejected_unchanged(&f);
    std::fs::copy(
        f.source.join("nested/preserved.bin"),
        f.target.join("nested/preserved.bin"),
    )
    .unwrap();
    std::fs::remove_file(f.target.join(MARKER)).unwrap();
    rejected_unchanged(&f);
}

#[test]
fn critic_correction_rejects_unsealed_original_even_with_consistent_hashes() {
    let f = fixture();
    edit(&f.source.join("cycle.json"), |data| {
        data["next_cohort_optimizer_rebound"] = false.into();
    });
    std::fs::copy(f.source.join("cycle.json"), f.target.join("cycle.json")).unwrap();
    let files = snapshot(&f.source).unwrap();
    let manifest = f.root.join("RECOVERY-PRESERVED.json");
    edit(&manifest, |data| {
        data["latest_all_file_sha256"] = serde_json::to_value(&files).unwrap();
    });
    edit(&f.target.join(MARKER), |data| {
        data["source_file_sha256"] = serde_json::to_value(files).unwrap();
        data["source_recovery_manifest_sha256"] = sha(&std::fs::read(&manifest).unwrap()).into();
    });
    rejected_unchanged(&f);
}

#[cfg(unix)]
#[test]
fn critic_correction_rejects_colliding_normalized_source_names() {
    let f = fixture();
    let bytes = std::fs::read(f.source.join("nested/preserved.bin")).unwrap();
    std::fs::write(f.source.join(r"nested\preserved.bin"), &bytes).unwrap();
    assert!(inspect_foundation_checkpoint(&f.target, 5124).is_err());
    assert_eq!(
        std::fs::read(f.source.join(r"nested\preserved.bin")).unwrap(),
        bytes
    );
}

#[cfg(unix)]
#[test]
fn critic_correction_rejects_colliding_normalized_target_names() {
    let f = fixture();
    let bytes = std::fs::read(f.target.join("nested/preserved.bin")).unwrap();
    std::fs::write(f.target.join(r"nested\preserved.bin"), &bytes).unwrap();
    assert!(inspect_foundation_checkpoint(&f.target, 5124).is_err());
    assert_eq!(
        std::fs::read(f.target.join(r"nested\preserved.bin")).unwrap(),
        bytes
    );
}

#[cfg(unix)]
#[test]
fn preservation_snapshot_rejects_literal_backslash_components() {
    let f = fixture();
    std::fs::write(f.source.join(r"other\unambiguous.bin"), b"preserved").unwrap();
    assert!(snapshot(&f.source).is_err());
}
