#![cfg(feature = "cpu-backend")]
use std::{fs, path::Path, process::Command};

#[path = "cpu_cli/storage.rs"]
mod storage;
use storage::{measure_history, verify_recovery};
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
fn snapshot_tree(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>> {
    fn visit(
        root: &Path,
        path: &Path,
        files: &mut std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>>,
    ) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let key = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                files.insert(key, None);
                visit(root, &path, files);
            } else {
                files.insert(key, Some(fs::read(path).unwrap()));
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    visit(root, root, &mut files);
    files
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
    let mut compact_bytes = 0;
    let mut pretty_bytes = 0;
    for name in [
        "collection-compiler-inputs.json",
        "collection-actor-checkpoint.json",
        "initial-value-checkpoint.json",
        "collector-checkpoint.json",
        "training-handoff.json",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ] {
        let bytes = fs::read(first.join(name)).unwrap();
        let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let pretty = serde_json::to_vec_pretty(&document).unwrap();
        assert!(bytes.len() < pretty.len(), "{name} retains formatting bulk");
        compact_bytes += bytes.len();
        pretty_bytes += pretty.len();
    }
    println!(
        "CPU fixture recovery JSON: {compact_bytes} compact bytes; {pretty_bytes} pretty bytes"
    );
    let handoff: alife_training::FoundationTrainingHandoffV1 =
        serde_json::from_slice(&fs::read(first.join("training-handoff.json")).unwrap()).unwrap();
    handoff.validate().unwrap();
    let actor: alife_training::FoundationTrainerCheckpoint =
        serde_json::from_slice(&fs::read(first.join("actor-checkpoint.json")).unwrap()).unwrap();
    let value: alife_training::PpoValueHeadCheckpoint =
        serde_json::from_slice(&fs::read(first.join("value-checkpoint.json")).unwrap()).unwrap();
    assert_eq!(handoff.actor, actor);
    assert_eq!(handoff.value, value);
    let compiler_bytes = serde_json::to_vec(&handoff.compiler_inputs).unwrap();
    let sidecar = first.join("compiler-inputs.json");
    assert!(!sidecar.exists());
    let admitted = alife_training::cpu::read_cpu_founder_source(&first).unwrap();
    assert_eq!(admitted.handoff, handoff);
    // Historical serial outputs may still carry the optional duplicate. Keep
    // reading it and rejecting invalid copies instead of weakening admission.
    fs::write(&sidecar, &compiler_bytes).unwrap();
    assert_eq!(
        alife_training::cpu::read_cpu_founder_source(&first)
            .unwrap()
            .handoff,
        handoff
    );
    fs::write(&sidecar, b"{}").unwrap();
    assert!(alife_training::cpu::read_cpu_founder_source(&first).is_err());
    fs::remove_file(sidecar).unwrap();
    let source_before = snapshot_tree(&first);
    let files: Vec<_> = source_before.values().filter_map(Option::as_ref).collect();
    println!(
        "CPU fixture first episode: {} files, {} logical bytes; omitted duplicate compiler inputs: {} bytes, one file",
        files.len(), files.iter().map(|bytes| bytes.len()).sum::<usize>(), compiler_bytes.len(),
    );
    let forbidden_outputs = vec![
        first.clone(),
        first.join("new-parent/experiment"),
        root.join("new-parent/../first/another-parent/experiment"),
        root.clone(),
    ];
    #[cfg(unix)]
    let forbidden_outputs = {
        let mut forbidden_outputs = forbidden_outputs;
        let alias = root.join("source-alias");
        std::os::unix::fs::symlink(&first, &alias).unwrap();
        forbidden_outputs.push(alias.join("new-parent/experiment"));
        forbidden_outputs.push(alias);
        forbidden_outputs
    };
    for forbidden in forbidden_outputs {
        let result = run(&[
            "--previous",
            text(&first),
            "--output",
            text(&forbidden),
            "--seed",
            "799978",
            "--ticks",
            "6",
        ]);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("must not overlap"));
        assert_eq!(snapshot_tree(&first), source_before);
    }
    assert!(!root.join("new-parent").exists());
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
    assert_eq!(snapshot_tree(&first), source_before);
    // Compact recovery must restore all learned coordinates, moments and exact
    // integer ages, not merely the receipt's optimizer step counters.
    let restored_actor: alife_training::FoundationTrainerCheckpoint =
        serde_json::from_slice(&fs::read(second.join("collection-actor-checkpoint.json")).unwrap())
            .unwrap();
    let restored_value: alife_training::PpoValueHeadCheckpoint =
        serde_json::from_slice(&fs::read(second.join("initial-value-checkpoint.json")).unwrap())
            .unwrap();
    assert_eq!(restored_actor, actor);
    assert_eq!(restored_value, value);
    assert_eq!(serde_json::to_vec(&restored_actor).unwrap(), before[3]);
    assert_eq!(serde_json::to_vec(&restored_value).unwrap(), before[4]);
    let restored_inputs: alife_core::PhenotypeCompilerInputs =
        serde_json::from_slice(&fs::read(second.join("collection-compiler-inputs.json")).unwrap())
            .unwrap();
    assert_eq!(restored_inputs, handoff.compiler_inputs);
    assert_eq!(
        serde_json::to_vec(&restored_inputs).unwrap(),
        compiler_bytes
    );
    assert!(!second.join("compiler-inputs.json").exists());
    alife_training::cpu::read_cpu_founder_source(&second).unwrap();
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
    let second_before = snapshot_tree(&second);
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
    assert_eq!(snapshot_tree(&second), second_before);
    let c: serde_json::Value = serde_json::from_slice(&c_bytes).unwrap();
    assert!(c.get("source_receipt").is_none());
    assert!(c["seed_provenance"].get("native_source_receipt").is_none());
    assert!(c["seed_provenance"]["native_source_receipt_ref"].is_string());
    assert_eq!(c["initial_actor_optimizer_step"], b["actor_optimizer_step"]);
    assert!(c_bytes.len() < 2 * serde_json::to_vec_pretty(&b).unwrap().len());
    verify_recovery(&second, &third);
    let mut episodes = vec![first.clone(), second.clone(), third.clone()];
    for episode in 3..8 {
        let previous = episodes.last().unwrap();
        let preserved = snapshot_tree(previous);
        let next = root.join(format!("serial-{episode}"));
        let seed = (800000 + episode).to_string();
        let result = run(&[
            "--previous",
            text(previous),
            "--output",
            text(&next),
            "--seed",
            &seed,
            "--ticks",
            "4",
        ]);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        verify_recovery(previous, &next);
        assert_eq!(snapshot_tree(previous), preserved);
        episodes.push(next);
    }
    measure_history(&episodes.iter().map(|p| p.as_path()).collect::<Vec<_>>());
    // Synthetic group-output import fixture: these native bytes prove backend
    // continuity only; this test does not claim fifty GPU-collected organisms.
    let group = root.join("synthetic-group-source");
    fs::create_dir(&group).unwrap();
    alife_training::native_receipt::Lineage::load(&c, &third)
        .unwrap()
        .write_to(&group)
        .unwrap();
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
    storage::relocate_recovery(&root, &episodes);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn serial_source_requires_native_binding_and_rejects_finite_checkpoint_tampering() {
    use alife_training::{FoundationSerialHandoffBindingV1, SERIAL_HANDOFF_SOURCE_FILES};
    use sha2::{Digest, Sha256};
    let sha = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
    let root = std::env::temp_dir().join(format!(
        "alife-serial-cpu-admission-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let first = root.join("cpu-fixture");
    let output = run(&[
        "--fixture",
        "--output",
        text(&first),
        "--seed",
        "898001",
        "--ticks",
        "4",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // Synthetic portable serial boundary; native exporter/correction verification
    // is tested in alife_game_app without requiring GPU execution.
    let serial = root.join("serial");
    fs::create_dir(&serial).unwrap();
    for name in [
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ] {
        fs::copy(first.join(name), serial.join(name)).unwrap();
    }
    let mut receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(first.join("cpu-cycle.json")).unwrap()).unwrap();
    receipt["next_cohort_tick_captured"] = true.into();
    receipt["next_cohort_optimizer_rebound"] = true.into();
    fs::write(
        serial.join("cycle.json"),
        serde_json::to_vec(&receipt).unwrap(),
    )
    .unwrap();
    fs::write(serial.join("phase.txt"), "next-cohort-admitted").unwrap();
    let rejected = root.join("missing-binding");
    let output = run(&[
        "--previous",
        text(&serial),
        "--output",
        text(&rejected),
        "--seed",
        "898002",
        "--ticks",
        "4",
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("validated portable handoff"));
    assert!(!rejected.exists());
    let mut handoff: alife_training::FoundationTrainingHandoffV1 =
        serde_json::from_slice(&fs::read(first.join("training-handoff.json")).unwrap()).unwrap();
    handoff.origin = alife_training::FoundationTrainingBackendV1::NativeGpu;
    let handoff_bytes = serde_json::to_vec(&handoff).unwrap();
    fs::write(serial.join("training-handoff.json"), &handoff_bytes).unwrap();
    let source_sha256 = SERIAL_HANDOFF_SOURCE_FILES
        .into_iter()
        .map(|name| (name.to_owned(), sha(&fs::read(serial.join(name)).unwrap())))
        .collect();
    let binding = FoundationSerialHandoffBindingV1 {
        schema_version: 1,
        native_serial_continuity_validated: true,
        source_sha256,
        training_handoff_sha256: sha(&handoff_bytes),
    };
    fs::write(
        serial.join("native-training-handoff.json"),
        serde_json::to_vec(&binding).unwrap(),
    )
    .unwrap();
    for (index, (name, field)) in [
        ("actor-checkpoint.json", "first_moment"),
        ("actor-checkpoint.json", "update_ages"),
        ("actor-checkpoint.json", "stage_mask"),
        ("value-checkpoint.json", "parameters"),
    ]
    .into_iter()
    .enumerate()
    {
        let path = serial.join(name);
        let original = fs::read(&path).unwrap();
        let mut changed: serde_json::Value = serde_json::from_slice(&original).unwrap();
        match field {
            "stage_mask" => changed[field]["words"][0] = 0.into(),
            "update_ages" => changed[field][0] = 1.into(),
            _ => changed[field][0] = (changed[field][0].as_f64().unwrap() + 0.125).into(),
        }
        let changed_bytes = serde_json::to_vec(&changed).unwrap();
        fs::write(&path, &changed_bytes).unwrap();
        let rejected = root.join(format!("rejected-{index}"));
        let output = run(&[
            "--previous",
            text(&serial),
            "--output",
            text(&rejected),
            "--seed",
            "898002",
            "--ticks",
            "4",
        ]);
        assert!(!output.status.success(), "tampered {field} was admitted");
        assert!(!rejected.exists());
        assert_eq!(fs::read(&path).unwrap(), changed_bytes);
        fs::write(&path, original).unwrap();
    }
    let accepted = root.join("accepted");
    let output = run(&[
        "--previous",
        text(&serial),
        "--output",
        text(&accepted),
        "--seed",
        "898002",
        "--ticks",
        "4",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let continued: serde_json::Value =
        serde_json::from_slice(&fs::read(accepted.join("cpu-cycle.json")).unwrap()).unwrap();
    assert_eq!(
        continued["initial_actor_optimizer_step"],
        handoff.actor.optimizer_step
    );
    assert_eq!(
        continued["initial_value_optimizer_step"],
        handoff.value.optimizer_step
    );
    assert_eq!(
        fs::read(serial.join("training-handoff.json")).unwrap(),
        handoff_bytes
    );
    fs::remove_dir_all(root).unwrap();
}
