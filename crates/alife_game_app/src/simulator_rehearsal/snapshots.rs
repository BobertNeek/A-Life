//! Exact snapshots and one shared capture budget for setup, pool and holdouts.
use super::*;

#[derive(Clone)]
pub(super) struct Snapshot {
    pub(super) id: u64,
    pub(super) target: WorldEntityId,
    pub(super) world: HeadlessWorld,
    pub(super) save: alife_world::PortableSaveFile,
    pub(super) lineage: RehearsalLineage,
}

pub(super) fn attach_durable(
    runtime: &mut GpuLiveBrainRuntime,
    output: &Path,
    assets: &Path,
    rollout: u64,
    base: alife_world::PortableSaveFile,
    attach_archive: bool,
) -> Result<()> {
    let path = output.join(format!("world-{rollout:03}.json"));
    runtime.attach_durable_checkpoint_boundary(&path, assets, base)?;
    if attach_archive {
        runtime.attach_lineage_archive(
            alife_archive::LineageLibraryConfig::profile_default(output.join("lineage")),
            alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        )?;
    }
    let exact = capture_exact(runtime, output, assets)?;
    let published = crate::GpuDurableSaveManifest::publish_snapshot(&path, assets, &exact)?;
    if published.save != exact {
        return Err("rehearsal durable publication changed exact state".into());
    }
    runtime.rebind_durable_checkpoint_boundary(&path, assets, &exact)?;
    Ok(())
}
pub(super) fn capture(
    runtime: &mut GpuLiveBrainRuntime,
    store: &GpuCheckpointAssetStore,
    id: u64,
    target: WorldEntityId,
    lineage: RehearsalLineage,
    output: &Path,
) -> Result<Snapshot> {
    learner(runtime.world())?;
    let save = capture_exact(runtime, output, store.root())?;
    save.validate_with_asset_root(store.root())?;
    let world = save.restore_headless_world()?;
    // Match the canonical on-disk save serializer; the inline-text helper has a separate 64 KiB display limit.
    let bytes = serde_json::to_vec_pretty(&save)?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("snapshot world envelope exceeds 64 MiB".into());
    }
    let name = format!("snapshot-{id:03}.json");
    std::fs::write(output.join(&name), &bytes)?;
    std::fs::write(
        output.join(format!("snapshot-{id:03}-lineage.json")),
        serde_json::to_vec_pretty(
            &serde_json::json!({"snapshot":id,"file":name,"file_blake3":blake3::hash(&bytes).to_hex().to_string(),"world_tick":world.tick(),"world_digest":world.canonical_signature_digest()?,"target":target,"lineage":lineage}),
        )?,
    )?;
    Ok(Snapshot {
        id,
        target,
        world,
        save,
        lineage,
    })
}

pub(super) fn directory_bytes(root: &Path) -> Result<u64> {
    let mut bytes = 0_u64;
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        bytes = bytes
            .checked_add(if metadata.is_dir() {
                directory_bytes(&entry.path())?
            } else {
                metadata.len()
            })
            .ok_or("snapshot disk accounting overflow")?;
    }
    Ok(bytes)
}
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct CaptureLedger {
    pub(super) attempted: usize,
    pub(super) completed: usize,
    pub(super) elapsed_seconds: f64,
}
pub(super) fn capture_ledger(output: &Path) -> Result<CaptureLedger> {
    let path = output.join("snapshot-capture-ledger.json");
    match std::fs::read(&path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(CaptureLedger::default()),
        Err(error) => Err(error.into()),
    }
}
pub(super) fn can_capture(output: &Path, assets: &Path, reserve: usize) -> Result<bool> {
    let ledger = capture_ledger(output)?;
    Ok(ledger.attempted + reserve < 192
        && directory_bytes(assets)? + (reserve as u64 + 1) * 64 * 1024 * 1024 <= 512 * 1024 * 1024)
}
fn capture_exact(
    runtime: &mut GpuLiveBrainRuntime,
    output: &Path,
    assets: &Path,
) -> Result<alife_world::PortableSaveFile> {
    if !can_capture(output, assets, 0)? {
        return Err("snapshot capture admission budget exhausted".into());
    }
    let mut ledger = capture_ledger(output)?;
    ledger.attempted += 1;
    std::fs::write(
        output.join("snapshot-capture-ledger.json"),
        serde_json::to_vec_pretty(&ledger)?,
    )?;
    let started = Instant::now();
    let result = runtime.capture_portable_checkpoint();
    ledger.elapsed_seconds += started.elapsed().as_secs_f64();
    ledger.completed += usize::from(result.is_ok());
    std::fs::write(
        output.join("snapshot-capture-ledger.json"),
        serde_json::to_vec_pretty(&ledger)?,
    )?;
    let save = result?;
    if directory_bytes(assets)? > 512 * 1024 * 1024 {
        return Err("snapshot asset budget exceeded".into());
    }
    Ok(save)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_capture_class_shares_the_same_ceiling_and_holdout_reservation() {
        let output =
            std::env::temp_dir().join(format!("alife-rehearsal-budget-{}", std::process::id()));
        std::fs::create_dir(&output).unwrap();
        let assets = output.join("assets");
        std::fs::create_dir(&assets).unwrap();
        assert!(can_capture(&output, &assets, 2).unwrap());
        let file = std::fs::File::create(assets.join("sparse-test-budget")).unwrap();
        file.set_len(400 * 1024 * 1024).unwrap();
        assert!(!can_capture(&output, &assets, 2).unwrap());
        assert!(can_capture(&output, &assets, 0).unwrap());
        file.set_len(450 * 1024 * 1024).unwrap();
        assert!(!can_capture(&output, &assets, 0).unwrap());
        std::fs::remove_dir_all(output).unwrap();
    }
}
