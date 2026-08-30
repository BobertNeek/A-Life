//! Durable GPU checkpoint save-manifest compare-and-swap contracts.
use std::{fs, path::Path};

use alife_core::BrainScaleTier;
use alife_runtime::{
    GpuDurableSaveManifest, GpuRuntimeError, GpuSaveManifestCasOutcome,
    GpuSleepTransactionJournalV2,
};
use alife_world::{
    persistence::{AssetManifest, PortableAssetDigest, PortableSaveFile, RuntimeConfig},
    HeadlessScenarioBuilder,
};

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

fn current_save(save_id: &str) -> PortableSaveFile {
    let world = HeadlessScenarioBuilder::new(73_127)
        .food("authority-fixture-food", alife_core::Vec3f::ZERO, 0.25)
        .build()
        .unwrap();
    PortableSaveFile::from_headless_world(
        save_id,
        &world,
        RuntimeConfig::deterministic_default(world.seed(), BrainScaleTier::Nano512),
        AssetManifest::empty(),
        Vec::new(),
    )
    .unwrap()
}

fn authority_artifact_names(save_path: &Path) -> (String, String, u64) {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(save_path).unwrap()).unwrap();
    let authority = &value["gpu_checkpoint_authority"];
    (
        authority["save"]["file_name"].as_str().unwrap().to_string(),
        authority["journal"]["file_name"]
            .as_str()
            .unwrap()
            .to_string(),
        authority["generation"].as_u64().unwrap(),
    )
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
        Err(GpuRuntimeError::GpuCheckpointManifestConflict { .. })
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

#[test]
fn exact_save_and_reset_journal_have_one_authority_commit_point() {
    let root = std::env::temp_dir().join(format!(
        "alife-gpu-save-journal-authority-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    let save_path = root.join("tiny_save.json");
    GpuDurableSaveManifest::publish_snapshot(&save_path, &root, &current_save("base")).unwrap();
    let durable = GpuDurableSaveManifest::open(&save_path, &root).unwrap();
    let loaded = durable.load().unwrap();
    let mut replacement = loaded.save.clone();
    replacement.save_id = "generation-backed-replacement".to_string();

    // A legacy sidecar destination that cannot be replaced models the old
    // save-first/journal-second crash seam. A generation-backed publication
    // must make a self-consistent pair authoritative through one pointer and
    // must not depend on this independently visible legacy path.
    let legacy_journal = root.join(".tiny_save.json.sleep-journal-v2.json");
    if legacy_journal.exists() {
        if legacy_journal.is_dir() {
            fs::remove_dir_all(&legacy_journal).unwrap();
        } else {
            fs::remove_file(&legacy_journal).unwrap();
        }
    }
    fs::create_dir(&legacy_journal).unwrap();

    durable
        .compare_and_swap(&loaded.digest, &replacement)
        .unwrap();
    let published = durable.load().unwrap();
    assert_eq!(published.save, replacement);
    let reset = durable.load_sleep_transaction_journal(&published).unwrap();
    assert!(reset.entries.is_empty());
    assert_eq!(
        reset.exact_base_manifest_digest,
        published.exact_save_anchor_digest().unwrap().0
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn journal_generations_advance_while_the_exact_save_anchor_stays_stable() {
    let root = std::env::temp_dir().join(format!(
        "alife-gpu-journal-generation-anchor-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    let save_path = root.join("current.json");
    let first = GpuDurableSaveManifest::publish_snapshot(
        &save_path,
        &root,
        &current_save("journal-generation-anchor"),
    )
    .unwrap();
    let durable = GpuDurableSaveManifest::open(&save_path, &root).unwrap();
    let anchor = first.exact_save_anchor_digest().unwrap().0;
    let save_artifact_count = || {
        fs::read_dir(&root)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".current.json.save-g")
            })
            .count()
    };
    let exact_save_artifacts_before = save_artifact_count();

    let journal_one = GpuSleepTransactionJournalV2::empty(&first).unwrap();
    durable
        .publish_sleep_transaction_journal(&first, &journal_one)
        .unwrap();
    let second = durable.load().unwrap();
    assert_eq!(second.authority_generation(), Some(2));
    assert_eq!(
        save_artifact_count(),
        exact_save_artifacts_before,
        "a journal-only generation must reuse the immutable exact-save artifact"
    );
    assert_eq!(second.exact_save_anchor_digest().unwrap().0, anchor);
    assert_eq!(
        durable
            .load_sleep_transaction_journal(&second)
            .unwrap()
            .exact_base_manifest_digest,
        anchor
    );

    let journal_two = GpuSleepTransactionJournalV2::empty(&second).unwrap();
    durable
        .publish_sleep_transaction_journal(&second, &journal_two)
        .unwrap();
    let third = durable.load().unwrap();
    assert_eq!(third.authority_generation(), Some(3));
    assert_eq!(
        save_artifact_count(),
        exact_save_artifacts_before,
        "later journal-only generations must keep reusing the exact-save artifact"
    );
    assert_eq!(third.exact_save_anchor_digest().unwrap().0, anchor);
    assert_eq!(
        durable
            .load_sleep_transaction_journal(&third)
            .unwrap()
            .exact_base_manifest_digest,
        anchor
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn authority_generations_keep_the_previous_exact_pair_reopenable() {
    let root = std::env::temp_dir().join(format!(
        "alife-gpu-authority-history-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    let save_path = root.join("current.json");
    let base = current_save("authority-generation-one");
    GpuDurableSaveManifest::publish_snapshot(&save_path, &root, &base).unwrap();
    let generation_one_envelope = fs::read(&save_path).unwrap();
    let durable = GpuDurableSaveManifest::open(&save_path, &root).unwrap();
    let loaded = durable.load().unwrap();
    let mut replacement = base.clone();
    replacement.save_id = "authority-generation-two".to_string();
    durable
        .compare_and_swap(&loaded.digest, &replacement)
        .unwrap();

    let old_pointer_path = root.join("generation-one-pointer.json");
    fs::write(&old_pointer_path, generation_one_envelope).unwrap();
    let reopened_old = GpuDurableSaveManifest::open(&old_pointer_path, &root)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(reopened_old.save, base);
    assert_eq!(reopened_old.authority_generation(), Some(1));
    assert_eq!(durable.load().unwrap().save, replacement);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn authority_missing_or_tampered_named_artifacts_fail_closed() {
    for (case, tamper_save, remove) in [
        ("missing-save", true, true),
        ("missing-journal", false, true),
        ("tampered-save", true, false),
        ("tampered-journal", false, false),
    ] {
        let root = std::env::temp_dir().join(format!(
            "alife-gpu-authority-tamper-{case}-{}",
            std::process::id()
        ));
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        fs::create_dir_all(&root).unwrap();
        let save_path = root.join("current.json");
        GpuDurableSaveManifest::publish_snapshot(
            &save_path,
            &root,
            &current_save("authority-tamper"),
        )
        .unwrap();
        let (save_artifact, journal_artifact, _) = authority_artifact_names(&save_path);
        let target = root.join(if tamper_save {
            save_artifact
        } else {
            journal_artifact
        });
        if remove {
            fs::remove_file(target).unwrap();
        } else {
            fs::write(target, b"tampered authority artifact").unwrap();
        }
        assert!(GpuDurableSaveManifest::open(&save_path, &root).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn journal_only_publication_advances_authority_and_rejects_the_stale_generation() {
    let root = std::env::temp_dir().join(format!(
        "alife-gpu-authority-journal-generation-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    let save_path = root.join("current.json");
    GpuDurableSaveManifest::publish_snapshot(
        &save_path,
        &root,
        &current_save("authority-journal-generation"),
    )
    .unwrap();
    let durable = GpuDurableSaveManifest::open(&save_path, &root).unwrap();
    let generation_one = durable.load().unwrap();
    let journal = GpuSleepTransactionJournalV2::empty(&generation_one).unwrap();

    durable
        .publish_sleep_transaction_journal(&generation_one, &journal)
        .unwrap();
    let generation_two = durable.load().unwrap();
    assert_eq!(generation_one.authority_generation(), Some(1));
    assert_eq!(generation_two.authority_generation(), Some(2));
    assert_eq!(generation_two.save, generation_one.save);
    assert_ne!(generation_two.digest, generation_one.digest);
    assert!(matches!(
        durable.publish_sleep_transaction_journal(&generation_one, &journal),
        Err(GpuRuntimeError::GpuCheckpointManifestConflict { .. })
    ));

    let refreshed_journal = GpuSleepTransactionJournalV2::empty(&generation_two).unwrap();
    durable
        .publish_sleep_transaction_journal(&generation_two, &refreshed_journal)
        .unwrap();
    assert_eq!(durable.load().unwrap().authority_generation(), Some(3));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn authority_unknown_pointer_version_fails_closed() {
    let root = std::env::temp_dir().join(format!(
        "alife-gpu-authority-version-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    let save_path = root.join("current.json");
    GpuDurableSaveManifest::publish_snapshot(&save_path, &root, &current_save("authority-version"))
        .unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&save_path).unwrap()).unwrap();
    value["gpu_checkpoint_authority"]["schema_version"] = serde_json::json!(99);
    fs::write(&save_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    assert!(GpuDurableSaveManifest::open(&save_path, &root).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn authority_identity_survives_complete_tree_relocation() {
    let source = std::env::temp_dir().join(format!(
        "alife-gpu-authority-relocate-source-{}",
        std::process::id()
    ));
    let target = std::env::temp_dir().join(format!(
        "alife-gpu-authority-relocate-target-{}",
        std::process::id()
    ));
    for root in [&source, &target] {
        if root.exists() {
            fs::remove_dir_all(root).unwrap();
        }
    }
    fs::create_dir_all(&source).unwrap();
    let save_path = source.join("current.json");
    let save = current_save("authority-relocation");
    let source_loaded =
        GpuDurableSaveManifest::publish_snapshot(&save_path, &source, &save).unwrap();
    copy_tree(&source, &target);
    let target_loaded = GpuDurableSaveManifest::open(target.join("current.json"), &target)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(target_loaded, source_loaded);
    fs::remove_dir_all(source).unwrap();
    fs::remove_dir_all(target).unwrap();
}

#[test]
fn pre_pointer_artifact_failure_leaves_old_authority_bytes_unchanged() {
    let root = std::env::temp_dir().join(format!(
        "alife-gpu-authority-precommit-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    let save_path = root.join("current.json");
    GpuDurableSaveManifest::publish_snapshot(
        &save_path,
        &root,
        &current_save("authority-precommit-base"),
    )
    .unwrap();
    let durable = GpuDurableSaveManifest::open(&save_path, &root).unwrap();
    let loaded = durable.load().unwrap();
    let old_pointer_bytes = fs::read(&save_path).unwrap();
    let mut replacement = loaded.save.clone();
    replacement.save_id = "authority-precommit-replacement".to_string();
    let save_bytes = serde_json::to_vec_pretty(&replacement).unwrap();
    let digest = PortableAssetDigest::for_bytes(&save_bytes);
    let digest_hex = digest.0.strip_prefix("fnv1a64:").unwrap();
    let blocked_artifact = root.join(format!(".current.json.save-g{:020}-{digest_hex}.json", 2));
    fs::create_dir(&blocked_artifact).unwrap();

    assert!(durable
        .compare_and_swap(&loaded.digest, &replacement)
        .is_err());
    assert_eq!(fs::read(&save_path).unwrap(), old_pointer_bytes);
    assert_eq!(durable.load().unwrap().save, loaded.save);

    fs::remove_dir_all(root).unwrap();
}
