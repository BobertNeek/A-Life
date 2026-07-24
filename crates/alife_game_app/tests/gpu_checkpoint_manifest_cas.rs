//! Durable GPU checkpoint save-manifest compare-and-swap contracts.
#![cfg(feature = "gpu-runtime")]

use std::{fs, path::Path};

use alife_game_app::{GameAppShellError, GpuDurableSaveManifest, GpuSaveManifestCasOutcome};

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn save_manifest_compare_and_swap_is_atomic_idempotent_and_conflict_typed() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34");
    let root = std::env::temp_dir().join(format!("alife-gpu-save-cas-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    copy_tree(&fixture, &root);
    let durable = GpuDurableSaveManifest::open(root.join("tiny_save.json"), &root).unwrap();
    let loaded = durable.load().unwrap();
    let mut replacement = loaded.save.clone();
    replacement.save_id = "gpu-cas-replacement".to_string();

    let first = durable
        .compare_and_swap(&loaded.digest, &replacement)
        .unwrap();
    let replacement_digest = match first {
        GpuSaveManifestCasOutcome::Replaced { replacement_digest } => replacement_digest,
        other => panic!("first CAS must replace, got {other:?}"),
    };
    assert_eq!(durable.load().unwrap().save, replacement);

    assert_eq!(
        durable
            .compare_and_swap(&loaded.digest, &replacement)
            .unwrap(),
        GpuSaveManifestCasOutcome::AlreadyApplied {
            replacement_digest: replacement_digest.clone(),
        }
    );

    let mut conflicting = replacement.clone();
    conflicting.save_id = "gpu-cas-conflict".to_string();
    assert!(matches!(
        durable.compare_and_swap(&loaded.digest, &conflicting),
        Err(GameAppShellError::GpuCheckpointManifestConflict { .. })
    ));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn manual_checkpoint_publish_atomically_creates_a_new_portable_save() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34");
    let root = std::env::temp_dir().join(format!("alife-gpu-manual-save-{}", std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    copy_tree(&fixture, &root);
    let source = GpuDurableSaveManifest::open(root.join("tiny_save.json"), &root)
        .unwrap()
        .load()
        .unwrap()
        .save;
    let mut replacement = source;
    replacement.save_id = "manual-gpu-checkpoint".to_string();
    let target = root.join("manual_checkpoint.json");

    let published = GpuDurableSaveManifest::publish_snapshot(&target, &root, &replacement).unwrap();

    assert_eq!(published.save, replacement);
    assert_eq!(
        GpuDurableSaveManifest::open(&target, &root)
            .unwrap()
            .load()
            .unwrap(),
        published
    );
    assert!(
        fs::read_dir(&root).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("gpu-cas")),
        "atomic publication must not leave a temporary manifest"
    );

    fs::remove_dir_all(root).unwrap();
}
