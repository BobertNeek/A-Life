//! Flatten the latest admitted private save, not prior generations or episodes.
use super::*;
use alife_training::CurrentLifeCheckpointV1;
use std::{
    collections::BTreeSet,
    fs,
    io::{Read, Write},
    path::{Component, PathBuf},
};

fn relative(name: &str) -> Result<()> {
    if name.is_empty()
        || name.contains(['\\', ':'])
        || !Path::new(name)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        || name.split('/').any(|p| matches!(p, "" | "." | ".."))
    {
        return Err("private checkpoint asset path must be canonical and relative".into());
    }
    Ok(())
}
fn read_asset(root: &Path, name: &str) -> Result<Vec<u8>> {
    relative(name)?;
    let mut path = root.to_path_buf();
    for part in Path::new(name).components() {
        path.push(part);
        if fs::symlink_metadata(&path)?.file_type().is_symlink() {
            return Err("private checkpoint asset symlink refused".into());
        }
    }
    if !path.canonicalize()?.starts_with(root)
        || !fs::symlink_metadata(&path)?.is_file()
        || fs::metadata(&path)?.len() > alife_training::MAX_CURRENT_CHECKPOINT_BYTES
    {
        return Err("private checkpoint asset is outside its bounded closure".into());
    }
    Ok(fs::read(path)?)
}

pub(super) fn capture(source: &Path) -> Result<CurrentLifeCheckpointV1> {
    let root = source.join("world-assets").canonicalize()?;
    let save_path = source.join("world.json");
    let pointer = fs::read(&save_path)?;
    let (durable, loaded) = GpuDurableSaveManifest::open_loaded(&save_path, &root)?;
    let journal = durable.load_sleep_transaction_journal(&loaded)?;
    let mut snapshot = CurrentLifeCheckpointV1 {
        world_json: serde_json::to_string(&loaded.save)?,
        sleep_journal_json: serde_json::to_string(&journal)?,
        assets_base64: Default::default(),
    };
    for entry in &loaded.save.assets.entries {
        let bytes = read_asset(&root, &entry.relative_path)?;
        snapshot.insert_asset(entry.relative_path.clone(), &bytes);
    }
    for name in snapshot.assets_base64.keys() {
        if read_asset(&root, name)? != snapshot.asset_bytes(name)? {
            return Err("private checkpoint changed during capture".into());
        }
    }
    if fs::read(&save_path)? != pointer
        || durable.load_sleep_transaction_journal(&loaded)? != journal
    {
        return Err("private save authority changed during capture".into());
    }
    validate(&snapshot)?;
    Ok(snapshot)
}

fn materialize(snapshot: &CurrentLifeCheckpointV1, root: &Path) -> Result<()> {
    let save = alife_world::PortableSaveFile::from_json_str(&snapshot.world_json)?;
    let expected: BTreeSet<_> = save
        .assets
        .entries
        .iter()
        .map(|e| e.relative_path.as_str())
        .collect();
    if expected != snapshot.assets_base64.keys().map(String::as_str).collect() {
        return Err("private checkpoint asset closure is missing or has unreferenced files".into());
    }
    let assets = root.join("world-assets");
    fs::create_dir(&assets)?;
    for name in snapshot.assets_base64.keys() {
        relative(name)?;
        let path = assets.join(name);
        fs::create_dir_all(path.parent().ok_or("private asset has no parent")?)?;
        fs::write(path, snapshot.asset_bytes(name)?)?;
    }
    fs::write(root.join("world.json"), &snapshot.world_json)?;
    fs::write(
        root.join(".world.json.sleep-journal-v2.json"),
        &snapshot.sleep_journal_json,
    )?;
    let (durable, loaded) = GpuDurableSaveManifest::open_loaded(root.join("world.json"), &assets)?;
    if loaded.save != save
        || serde_json::to_string(&durable.load_sleep_transaction_journal(&loaded)?)?
            != snapshot.sleep_journal_json
    {
        return Err("private recovery changed the exact save or sleep journal".into());
    }
    // This invokes current world/biology, composite-genome and lifetime-state
    // readers, in addition to every declared asset digest/size check.
    loaded.save.validate_with_asset_root(&assets)?;
    loaded.save.restore_headless_world()?;
    Ok(())
}
pub(super) struct Temporary {
    path: PathBuf,
    _lease: fs::File,
}
impl AsRef<Path> for Temporary {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
fn real_directory(path: &Path) -> Result<bool> {
    let metadata = fs::symlink_metadata(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Ok(false);
        }
    }
    Ok(metadata.is_dir() && !metadata.file_type().is_symlink())
}
pub(super) fn workspace_at(root: &Path) -> Result<Temporary> {
    #[cfg(unix)]
    let created = {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new().mode(0o700).create(root)
    };
    #[cfg(not(unix))]
    let created = fs::create_dir(root);
    match created {
        Ok(()) => (),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(error) => return Err(error.into()),
    }
    if !real_directory(root)? {
        return Err("invalid private validation workspace".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(root)?.permissions().mode() & 0o077 != 0 {
            return Err("private validation workspace permissions are not private".into());
        }
    }
    let lock = root.join("lease");
    match fs::symlink_metadata(&lock) {
        Ok(metadata)
            if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 128 =>
        {
            return Err("invalid private validation lease".into())
        }
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
        Err(error) => return Err(error.into()),
    }
    let mut lease = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock)?;
    // The lease survives file-name reuse and is never unlinked. A killed
    // process releases it; another validator may then remove only this owned
    // workspace's interrupted materialization. Never sweep arbitrary temp files.
    lease.lock()?;
    let mut marker = String::new();
    lease.read_to_string(&mut marker)?;
    let expected = "alife-current-private-validation-v1\n";
    let entries = fs::read_dir(root)?
        .map(|e| e.map(|e| e.file_name()))
        .collect::<std::io::Result<BTreeSet<_>>>()?;
    if marker.is_empty() && entries == BTreeSet::from([std::ffi::OsString::from("lease")]) {
        lease.write_all(expected.as_bytes())?;
        lease.sync_all()?;
    } else if marker != expected || entries.iter().any(|name| name != "lease" && name != "work") {
        return Err("unknown private validation workspace contents; refusing cleanup".into());
    }
    let path = root.join("work");
    if path.exists() || path.is_symlink() {
        if !real_directory(&path)? {
            return Err("private validation scratch must be a real directory".into());
        }
        fs::remove_dir_all(&path)?;
    }
    fs::create_dir(&path)?;
    Ok(Temporary {
        path,
        _lease: lease,
    })
}
pub(super) fn validate(snapshot: &CurrentLifeCheckpointV1) -> Result<()> {
    let root = std::env::temp_dir().join("alife-current-private-validation-v1");
    let temporary = workspace_at(&root)?;
    materialize(snapshot, &temporary.path)
}

/// Restore a portable private world/organism bundle into a fresh directory.
/// It is distinct from the fresh-cohort founder/Adam continuation boundary.
pub fn extract_current_training_life(checkpoint: &Path, output: &Path) -> Result<()> {
    let checkpoint = checkpoint.canonicalize()?;
    let output = alife_training::resolve_isolated_training_output(output, [checkpoint.as_path()])?;
    let current = read_current_training_checkpoint(&checkpoint)?;
    let snapshot = current
        .life
        .ok_or("current checkpoint has no private life snapshot")?;
    fs::create_dir(&output)?;
    materialize(&snapshot, &output)
}
