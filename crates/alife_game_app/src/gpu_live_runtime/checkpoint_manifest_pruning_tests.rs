//! Each new exact manifest retains current roots without deleting older saves.
use super::*;
use alife_world::persistence::{AssetKind, AssetManifestEntry, AssetPresence};
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
enum CaptureKind {
    Portable,
    Async,
}

fn fixture(label: &str) -> (PathBuf, GpuLiveBrainRuntime, AssetManifestEntry) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/manifest-roots-{}-{label}",
        std::process::id()
    ));
    assert!(
        !root.exists(),
        "preserve prior manifest regression evidence"
    );
    let asset_root = root.join("assets");
    std::fs::create_dir_all(&asset_root).unwrap();
    let marker_bytes = b"ordinary non-neural asset retained across checkpoints";
    std::fs::write(asset_root.join("ordinary-marker.txt"), marker_bytes).unwrap();
    let marker = AssetManifestEntry {
        asset_id: "ordinary-marker".to_string(),
        kind: AssetKind::Other,
        relative_path: "ordinary-marker.txt".to_string(),
        digest: PortableAssetDigest::for_bytes(marker_bytes),
        presence: AssetPresence::Required,
        schema_version: 1,
        size_bytes: Some(marker_bytes.len() as u64),
        provenance: Some("independent non-neural fixture asset".to_string()),
    };
    let mut assets = AssetManifest::empty();
    assets.entries.push(marker.clone());
    let mut config = RuntimeConfig::deterministic_default(31_117, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    let mut runtime =
        crate::create_canonical_new_game_runtime(crate::CanonicalNewGameLaunchRequest {
            world_seed: 31_117,
            population: 4,
            save_path: root.join("live.json"),
            asset_root,
            config,
            assets,
        })
        .unwrap()
        .runtime;
    println!(
        "MANIFEST_ROOTS_GPU_ADAPTER={} EVIDENCE={}",
        runtime.authority_telemetry().adapter,
        root.display()
    );
    assert!(matches!(
        runtime.tick_outcome().unwrap(),
        GpuLiveTickOutcome::Progressed(_)
    ));
    (root, runtime, marker)
}

fn capture(runtime: &mut GpuLiveBrainRuntime, kind: CaptureKind) -> PortableSaveFile {
    if matches!(kind, CaptureKind::Portable) {
        return runtime.capture_portable_checkpoint().unwrap();
    }
    let tick = runtime.world.tick();
    runtime.request_exact_population_checkpoint().unwrap();
    let deadline = Instant::now() + Duration::from_secs(90);
    while !runtime.persistence_idle_for_shutdown() {
        assert!(
            Instant::now() < deadline,
            "{}",
            runtime.persistence_shutdown_diagnostics()
        );
        runtime.poll_persistence_for_shutdown().unwrap();
        thread::sleep(Duration::from_millis(1));
    }
    let save = runtime
        .checkpoint_durability
        .as_ref()
        .unwrap()
        .published
        .save
        .clone();
    assert_eq!(save.world.tick, tick);
    save
}

fn neural_roots(save: &PortableSaveFile) -> BTreeSet<String> {
    save.creatures
        .iter()
        .flat_map(|creature| {
            creature
                .gpu_brain
                .as_ref()
                .unwrap()
                .asset_references()
                .into_iter()
                .map(|asset| asset.asset_id.clone())
        })
        .collect()
}

fn writer_ids(save: &PortableSaveFile) -> BTreeSet<String> {
    // These entries are emitted by the actual writer in this controlled fixture.
    // Library tests separately cover ambiguous or forged ownership metadata.
    save.assets
        .entries
        .iter()
        .filter(|entry| {
            entry
                .provenance
                .as_deref()
                .is_some_and(|value| value.starts_with("gpu-checkpoint:"))
        })
        .map(|entry| entry.asset_id.clone())
        .collect()
}

fn restore_retained(
    live: &GpuLiveBrainRuntime,
    path: &Path,
    asset_root: &Path,
    expected: &PortableSaveFile,
    marker: &AssetManifestEntry,
) {
    let (manifest, loaded) = GpuDurableSaveManifest::open_loaded(path, asset_root).unwrap();
    assert_eq!(&loaded.save, expected);
    assert!(loaded.save.assets.entries.contains(marker));
    loaded.save.validate_with_asset_root(asset_root).unwrap();
    let restored = GpuLiveBrainRuntime::restore_loaded_save(
        live.new_staging_like_live().unwrap(),
        manifest,
        loaded,
        expected.deterministic_seed,
        expected.config.brain_class,
    )
    .unwrap();
    assert_eq!(restored.world.tick(), expected.world.tick);
    assert_eq!(restored.residents.len(), 4);
    assert_eq!(restored.memories.len(), 4);
    assert_eq!(restored.topologies.len(), 4);
    restored.world.validate_organism_bindings().unwrap();
    restored.backend.ensure_neural_actions_available().unwrap();
    drop(restored);
}

fn assert_current_roots_and_retained_generation(kind: CaptureKind, label: &str) {
    let (root, mut runtime, marker) = fixture(label);
    let asset_root = root.join("assets");
    let first = capture(&mut runtime, kind);
    let first_path = root.join("retained-first.json");
    GpuDurableSaveManifest::publish_snapshot(&first_path, &asset_root, &first).unwrap();
    if matches!(kind, CaptureKind::Portable) {
        // Save-as adopts A as the next portable capture's base, so B must shed
        // A-only entries rather than repeatedly branching from the initial save.
        let live_base_path = root.join("portable-live-base.json");
        GpuDurableSaveManifest::publish_snapshot(&live_base_path, &asset_root, &first).unwrap();
        runtime
            .rebind_durable_checkpoint_boundary(&live_base_path, &asset_root, &first)
            .unwrap();
    }
    assert!(matches!(
        runtime.tick_outcome().unwrap(),
        GpuLiveTickOutcome::Progressed(_)
    ));
    let second = capture(&mut runtime, kind);
    assert!(second.world.tick > first.world.tick);
    let first_roots = neural_roots(&first);
    let second_roots = neural_roots(&second);
    let obsolete = first_roots
        .difference(&second_roots)
        .cloned()
        .collect::<BTreeSet<_>>();
    let shared = first_roots
        .intersection(&second_roots)
        .cloned()
        .collect::<BTreeSet<_>>();
    assert!(
        !obsolete.is_empty(),
        "the real tick must change at least one neural asset"
    );
    assert!(
        !shared.is_empty(),
        "unchanged content-addressed roots must also be exercised"
    );
    assert!(obsolete.is_subset(&writer_ids(&first)));

    let second_path = root.join("retained-second.json");
    GpuDurableSaveManifest::publish_snapshot(&second_path, &asset_root, &second).unwrap();
    // Restore and drop each staging runtime serially. Neither save's CAS files
    // are deleted, rewritten, or replaced by a test fixture.
    restore_retained(&runtime, &first_path, &asset_root, &first, &marker);
    restore_retained(&runtime, &second_path, &asset_root, &second, &marker);
    for entry in &first.assets.entries {
        assert!(asset_root.join(&entry.relative_path).exists());
    }
    let manifested = writer_ids(&second);
    println!("MANIFEST_ROOTS kind={kind:?} old={} current={} obsolete={} shared={} manifest={} old_and_new_gpu_restore=true",
        first_roots.len(), second_roots.len(), obsolete.len(), shared.len(), manifested.len());
    assert_eq!(
        manifested, second_roots,
        "new manifest must contain exactly current neural roots"
    );
    assert!(obsolete.is_disjoint(&manifested));
    assert!(shared.is_subset(&manifested));
}

#[test]
fn portable_checkpoint_manifest_drops_stale_roots_and_preserves_old_save() {
    assert_current_roots_and_retained_generation(CaptureKind::Portable, "portable");
}

#[test]
fn async_checkpoint_manifest_drops_stale_roots_and_preserves_old_save() {
    assert_current_roots_and_retained_generation(CaptureKind::Async, "async");
}
