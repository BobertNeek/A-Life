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
) -> Result<alife_world::PortableSaveFile> {
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
    Ok(exact)
}

/// Private to the adjacent setup -> decision-zero seam. This is not a cache:
/// the save is consumed once, and any later selection must export current state.
pub(super) fn save_for_pool_snapshot(
    setup: &mut Option<alife_world::PortableSaveFile>,
    decision: usize,
    live_tick: alife_core::Tick,
    export: impl FnOnce() -> Result<alife_world::PortableSaveFile>,
) -> Result<alife_world::PortableSaveFile> {
    let setup = setup.take();
    if decision == 0 {
        let save = setup.ok_or("decision-zero setup save already consumed")?;
        if save.world.tick != live_tick {
            return Err("setup save cannot be reused after simulator progress".into());
        }
        Ok(save)
    } else {
        export()
    }
}

#[cfg(all(test, feature = "gpu-tests"))]
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
    snapshot_from_save(save, store, id, target, lineage, output)
}

pub(super) fn snapshot_from_save(
    save: alife_world::PortableSaveFile,
    store: &GpuCheckpointAssetStore,
    id: u64,
    target: WorldEntityId,
    lineage: RehearsalLineage,
    output: &Path,
) -> Result<Snapshot> {
    save.validate_with_asset_root(store.root())?;
    let world = save.restore_headless_world()?;
    learner(&world)?;
    // Use the canonical on-disk save representation.
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
#[derive(Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
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
pub(super) fn capture_exact(
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

    fn scene() -> (alife_world::PortableSaveFile, WorldEntityId) {
        // Device-free serializer fixture; no claim about GPU/private-state equality.
        let asset = crate::initial_n2048_care_asset(77_777).unwrap();
        let (mut game, target) = fresh::prepare_scene(
            &asset,
            &fresh::SetupContext {
                founder: 77_777,
                seed: 99_001,
                rollout: 0,
            },
        )
        .unwrap();
        for creature in &mut game.creatures {
            let biology = game
                .world
                .organism_registry()
                .get(creature.organism_id)
                .unwrap()
                .biochemistry();
            creature.development_tick = biology.development.last_update_tick;
            creature.mind.tick = biology.tick;
            creature.mind.homeostasis = biology.homeostasis;
        }
        let save = alife_world::PortableSaveFile::from_headless_world(
            "setup-snapshot-fixture",
            &game.world,
            alife_world::RuntimeConfig::deterministic_default(
                game.world.seed(),
                BrainScaleTier::Standard2048,
            ),
            AssetManifest::empty(),
            game.creatures,
        )
        .unwrap();
        (save, target)
    }

    #[test]
    fn setup_is_consumed_once_and_later_boundaries_export_current_state() {
        let (save, _) = scene();
        let mut setup = Some(save.clone());
        let first = save_for_pool_snapshot(&mut setup, 0, save.world.tick, || {
            panic!("redundant GPU export")
        })
        .unwrap();
        assert_eq!(first, save);
        assert!(setup.is_none());
        assert!(
            save_for_pool_snapshot(&mut setup, 0, save.world.tick, || panic!(
                "missing setup must fail"
            ))
            .is_err()
        );
        let mut exports = 0;
        for decision in [8, 16, 24] {
            // Even a leftover setup save (e.g. a skipped pool admission at zero)
            // must not be reused at a later boundary.
            setup = Some(save.clone());
            save_for_pool_snapshot(
                &mut setup,
                decision,
                alife_core::Tick(decision as u64),
                || {
                    exports += 1;
                    Ok(save.clone())
                },
            )
            .unwrap();
            assert!(setup.is_none());
        }
        assert_eq!(exports, 3);
        let error = save_for_pool_snapshot(&mut setup, 32, alife_core::Tick(32), || {
            Err("export failed".into())
        })
        .unwrap_err();
        assert_eq!(error.to_string(), "export failed");
        setup = Some(save.clone());
        assert!(save_for_pool_snapshot(
            &mut setup,
            0,
            alife_core::Tick(save.world.tick.raw() + 1),
            || panic!("stale setup must fail")
        )
        .is_err());
        assert!(setup.is_none());
    }

    #[test]
    fn pool_publication_keeps_save_hash_lineage_and_asset_validation_without_an_export() {
        let (mut save, target) = scene();
        let output =
            std::env::temp_dir().join(format!("alife-rehearsal-pool-{}", std::process::id()));
        std::fs::create_dir(&output).unwrap();
        let assets = output.join("assets");
        std::fs::create_dir(&assets).unwrap();
        let store = GpuCheckpointAssetStore::new(&assets).unwrap();
        std::fs::write(assets.join("marker"), b"preserved").unwrap();
        save.assets
            .entries
            .push(alife_world::persistence::AssetManifestEntry {
                asset_id: "marker".into(),
                kind: alife_world::persistence::AssetKind::Other,
                relative_path: "marker".into(),
                digest: alife_world::PortableAssetDigest::for_bytes(b"preserved"),
                presence: alife_world::persistence::AssetPresence::Required,
                schema_version: 1,
                size_bytes: Some(9),
                provenance: None,
            });
        let lineage = RehearsalLineage {
            rollout: 0,
            parent_rollout: None,
            parent_snapshot: None,
            world_seed: save.world.seed,
            sampling_seed: 71_001,
            policy_version: 7401,
            foundation_digest: [1; 32],
            source: RehearsalSource::Fresh,
            variation: SceneVariation::Original,
            historically_fresh: false,
        };
        let snapshot =
            snapshot_from_save(save.clone(), &store, 7, target, lineage.clone(), &output).unwrap();
        assert_eq!(snapshot.save, save);
        assert_eq!(snapshot.lineage, lineage);
        let encoded = std::fs::read(output.join("snapshot-007.json")).unwrap();
        assert_eq!(
            serde_json::from_slice::<alife_world::PortableSaveFile>(&encoded).unwrap(),
            save
        );
        let recorded: serde_json::Value = serde_json::from_slice(
            &std::fs::read(output.join("snapshot-007-lineage.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(recorded["snapshot"], 7);
        assert_eq!(
            recorded["file_blake3"],
            blake3::hash(&encoded).to_hex().to_string()
        );
        assert_eq!(recorded["lineage"], serde_json::to_value(lineage).unwrap());
        assert_eq!(
            recorded["world_digest"],
            serde_json::to_value(snapshot.world.canonical_signature_digest().unwrap()).unwrap()
        );
        assert_eq!(capture_ledger(&output).unwrap().attempted, 0);
        assert!(!output.join("snapshot-capture-ledger.json").exists());
        std::fs::write(assets.join("marker"), b"corrupted").unwrap();
        assert!(snapshot_from_save(save, &store, 8, target, snapshot.lineage, &output).is_err());
        assert!(!output.join("snapshot-008.json").exists());
        std::fs::remove_dir_all(output).unwrap();
    }
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
