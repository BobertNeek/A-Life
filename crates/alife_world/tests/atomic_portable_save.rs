use std::{fs, path::PathBuf};

use alife_core::{BrainScaleTier, OrganismId, Vec3f};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessScenarioBuilder,
};

fn fixture(name: &str) -> PortableSaveFile {
    let world = HeadlessScenarioBuilder::new(4242)
        .agent("creature", OrganismId(1), Vec3f::ZERO)
        .food("berry", Vec3f::new(1.0, 0.0, 0.0), 0.75)
        .build()
        .unwrap();
    PortableSaveFile::from_headless_world(
        name,
        &world,
        RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512),
        AssetManifest::empty(),
        vec![],
    )
    .unwrap()
}

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "alife-atomic-portable-save-{name}-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    root
}

#[cfg(unix)]
#[test]
fn unix_save_permissions_under_controlled_umasks() {
    for mask in ["000", "022", "077"] {
        let status = std::process::Command::new("sh")
            .args([
                "-c",
                "umask \"$1\"; shift; exec \"$@\"",
                "alife-save-mode",
                mask,
            ])
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", "unix_save_mode_child", "--nocapture"])
            .env("ALIFE_SAVE_MODE_CHILD", "1")
            .status()
            .unwrap();
        assert!(status.success(), "save permissions under umask {mask}");
    }
}

#[cfg(unix)]
#[test]
fn unix_save_mode_child() {
    use std::os::unix::fs::PermissionsExt;
    if std::env::var_os("ALIFE_SAVE_MODE_CHILD").is_none() {
        return;
    }
    let root = temp_root("unix-modes");
    let path = root.join("creature.alife.json");
    let mut save = fixture("private");
    // Exercise the public API against an existing private save first.
    fs::write(&path, save.to_json_string_pretty().unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    save.save_id = "replacement".to_string();
    save.to_json_file(&path).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        PortableSaveFile::from_json_file(&path).unwrap().save_id,
        "replacement"
    );
    // Never broaden stricter owner permissions; replacement also removes
    // group/other access rather than enabling inherited ACL entries.
    for mode in [0o000, 0o400, 0o600, 0o640, 0o644] {
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        save.to_json_file(&path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            mode & 0o600
        );
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::remove_file(&path).unwrap();
    save.to_json_file(&path).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn unix_save_rejects_symlink_without_changing_target_or_link() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let root = temp_root("symlink");
    let target = root.join("private-original");
    fs::write(&target, b"recoverable original").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    let link = root.join("creature.alife.json");
    symlink(&target, &link).unwrap();
    assert!(fixture("replacement").to_json_file(&link).is_err());
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read(&target).unwrap(), b"recoverable original");
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn save_creation_and_replacement_preserve_exact_world_state_and_budget() {
    let root = temp_root("replace");
    let budget = alife_storage::OutputBudget::register(&root, 8 * 1024 * 1024).unwrap();
    let path = root.join("creature.alife.json");
    let mut save = fixture("first");
    save.to_json_file(&path).unwrap();
    save.save_id = "replacement".to_string();
    save.to_json_file(&path).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert_eq!(bytes, save.to_json_string_pretty().unwrap().as_bytes());
    let restored = PortableSaveFile::from_json_file(&path).unwrap();
    assert_eq!(restored.save_id, save.save_id);
    assert_eq!(restored.world, save.world);
    assert_eq!(
        restored
            .restore_headless_world()
            .unwrap()
            .canonical_signature_digest()
            .unwrap(),
        save.restore_headless_world()
            .unwrap()
            .canonical_signature_digest()
            .unwrap()
    );
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(budget.usage().unwrap().used, bytes.len() as u64);
    alife_storage::remove_dir_all(&root).unwrap();
    assert_eq!(budget.usage().unwrap().used, 0);
}

#[test]
fn failed_save_write_keeps_previous_save_readable_and_releases_staging_budget() {
    let root = temp_root("write-failure");
    let original = fixture("previous");
    let bytes = original.to_json_string_pretty().unwrap().into_bytes();
    let budget = alife_storage::OutputBudget::register(&root, bytes.len() as u64 + 1).unwrap();
    let path = root.join("creature.alife.json");
    alife_storage::write(&path, &bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o400)).unwrap();
    }
    let mut replacement = original.clone();
    replacement.save_id = "larger replacement".repeat(1024);
    assert!(replacement.to_json_file(&path).is_err());
    assert!(fs::read(&path).unwrap() == bytes, "previous save changed");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o400
        );
    }
    let restored = PortableSaveFile::from_json_file(&path).unwrap();
    assert_eq!(restored.save_id, original.save_id);
    assert_eq!(restored.world, original.world);
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    let usage = budget.usage().unwrap();
    assert_eq!(usage.used, bytes.len() as u64);
    assert_eq!(usage.reserved, 0);
    assert!(usage.high_water <= usage.limit);
    alife_storage::remove_dir_all(&root).unwrap();
}

#[test]
fn failed_save_publication_preserves_destination_and_cleans_its_staged_file() {
    let root = temp_root("publication-failure");
    let budget = alife_storage::OutputBudget::register(&root, 8 * 1024 * 1024).unwrap();
    let path = root.join("creature.alife.json");
    fs::create_dir(&path).unwrap();
    let sentinel = path.join("recoverable-state");
    alife_storage::write(&sentinel, b"preserve this").unwrap();
    assert!(fixture("replacement").to_json_file(&path).is_err());
    assert_eq!(fs::read(&sentinel).unwrap(), b"preserve this");
    assert_eq!(fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(budget.usage().unwrap().used, 13);
    alife_storage::remove_dir_all(&root).unwrap();
}
