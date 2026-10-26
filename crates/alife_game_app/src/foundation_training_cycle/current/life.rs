//! Flatten the latest admitted private save, not prior generations or episodes.
use super::*;
use alife_training::CurrentLifeCheckpointV1;
use std::{
    collections::BTreeSet,
    fs,
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
struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
pub(super) fn validate(snapshot: &CurrentLifeCheckpointV1) -> Result<()> {
    let root = std::env::temp_dir().join(format!(
        "alife-current-life-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    ));
    fs::create_dir(&root)?;
    let temporary = Temporary(root);
    materialize(snapshot, &temporary.0)
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
