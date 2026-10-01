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
    let readiness = label == "readiness";
    let world_seed = if readiness { 539_363_617 } else { 31_117 };
    let founder = if readiness {
        crate::NewGameFounderSelection::N2048Candidate {
            asset_path: std::env::var_os("ALIFE_READINESS_FOUNDER_ASSET")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                        "../../assets/founders/terrain-care-n2048-v3/trained.alife-foundation",
                    )
                }),
        }
    } else {
        crate::NewGameFounderSelection::BuiltinNano512
    };
    // The launch request uses the canonical base configuration. The explicit
    // founder selection sets the resident class during New Game staging.
    let mut config = RuntimeConfig::deterministic_default(world_seed, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    let mut runtime = crate::create_canonical_new_game_runtime_with_founder(
        crate::CanonicalNewGameLaunchRequest {
            world_seed,
            population: if label == "readiness" { 1 } else { 4 },
            disable_age_death: false,
            save_path: root.join("live.json"),
            asset_root,
            config,
            assets,
        },
        founder,
    )
    .unwrap()
    .runtime;
    println!(
        "MANIFEST_ROOTS_GPU_ADAPTER={} EVIDENCE={}",
        runtime.authority_telemetry().adapter,
        root.display()
    );
    if !readiness {
        assert!(matches!(
            runtime.tick_outcome().unwrap(),
            GpuLiveTickOutcome::Progressed(_)
        ));
    }
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

#[test]
fn readiness_resume_preserves_external_actors_toys_and_private_prior() {
    let (root, mut runtime, _marker) = fixture("readiness");
    let organism = runtime.world.organism_entity_ids()[0].0;
    let position = runtime.world.object_snapshots()[0].position;
    let teacher = runtime
        .world
        .spawn_social_agent("readiness-teacher", OrganismId(9000001), position, 0.75)
        .unwrap();
    runtime
        .world
        .spawn_toy("readiness-ball", position, true)
        .unwrap();
    runtime
        .world
        .spawn_toy("readiness-station", position, false)
        .unwrap();
    // No server request is needed to exercise the real fading controller.
    let mut prior = semantic_prior::RuntimeSemanticPrior::from_environment(31117, false)
        .unwrap()
        .unwrap();
    prior.seed_resume_check(organism.raw(), runtime.world.tick().raw());
    runtime.semantic_prior = Some(prior);
    let handle = runtime.handles[&organism.raw()];
    assert_eq!(handle.class_id(), BrainCapacityClass::N2048_ID);
    let initial_fast = runtime
        .backend
        .read_active_fast_weights_for_test(handle)
        .unwrap();
    let mut learning_transactions = 0;
    let mut reported_fast_changes = 0;
    let acquisition_deadline = Instant::now() + Duration::from_secs(60);
    for _ in 0..8 {
        readiness_tick(&mut runtime, acquisition_deadline);
        for receipt in runtime.last_learning_receipts() {
            if receipt.handle.organism_id() == organism {
                learning_transactions += 1;
                reported_fast_changes += receipt.fast_weights_changed;
            }
        }
    }
    let learned_fast = runtime
        .backend
        .read_active_fast_weights_for_test(handle)
        .unwrap();
    let learned_lifetime = runtime
        .backend
        .read_active_lifetime_weights_for_test(handle)
        .unwrap();
    let learned_changes = initial_fast
        .iter()
        .zip(&learned_fast)
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    assert!(
        learning_transactions > 0 && reported_fast_changes > 0 && learned_changes > 0,
        "ordinary N2048 GPU outcomes must produce measured acquired weights: transactions={learning_transactions}, reported_changes={reported_fast_changes}, measured_changes={learned_changes}"
    );
    assert!(runtime.memories[&organism.raw()].bank().fast_len() > 0);

    // This is an explicit public recovery request, not induced biological sleep.
    let before_sleep = runtime.residents[&organism.raw()].sleep_scheduler.state();
    let transition = runtime.request_recovery_sleep(organism).unwrap();
    assert_eq!(transition.from, SleepPhase::Awake);
    let deadline = Instant::now() + Duration::from_secs(120);
    let mut progressed = 0;
    let mut saw_submitted = false;
    let mut saw_completed = false;
    let mut observations = Vec::new();
    let mut previous = None;
    while progressed < 96 && Instant::now() < deadline {
        // Poll as a live tick does. Shutdown finalizes an AwaitingJournal
        // checkpoint without promoting Completed sleep, consuming the permit
        // this running recovery test needs for its next normal tick.
        runtime.poll_sleep_journal_publication().unwrap();
        runtime.poll_exact_population_checkpoint().unwrap();
        let state = runtime.residents[&organism.raw()].sleep_scheduler.state();
        // Completed neural work waits for its normal durable publication permit.
        if matches!(state.consolidation, ConsolidationState::Completed { .. })
            && !runtime
                .durable_completed_sleep_permitted_ids_for_test()
                .contains(&organism)
        {
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        if matches!(
            runtime.tick_outcome().unwrap(),
            GpuLiveTickOutcome::Progressed(_)
        ) {
            progressed += 1;
        }
        let state = runtime.residents[&organism.raw()].sleep_scheduler.state();
        saw_submitted |= matches!(state.consolidation, ConsolidationState::Submitted { .. });
        saw_completed |= matches!(state.consolidation, ConsolidationState::Completed { .. });
        if previous != Some(state) {
            observations.push(serde_json::json!({"tick":runtime.world.tick(),"state":state}));
            previous = Some(state);
        }
        if state.phase == SleepPhase::Awake
            && state.last_consolidated_cycle_id == before_sleep.last_consolidated_cycle_id + 1
        {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let after_sleep = runtime.residents[&organism.raw()].sleep_scheduler.state();
    let slept_lifetime = runtime
        .backend
        .read_active_lifetime_weights_for_test(handle)
        .unwrap();
    let consolidated_changes = learned_lifetime
        .iter()
        .zip(&slept_lifetime)
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    std::fs::write(root.join("readiness-learning-sleep.json"), serde_json::to_vec_pretty(
        &serde_json::json!({
            "brain_class":"N2048", "sleep_trigger":"public request_recovery_sleep",
            "prior_coverage":"deterministically seeded private state; no service-delivery claim",
            "learning_transactions":learning_transactions, "reported_fast_changes":reported_fast_changes,
            "measured_fast_entries_changed":learned_changes,
            "measured_lifetime_entries_changed":consolidated_changes,
            "before_sleep":before_sleep,"after_sleep":after_sleep,"progressed_ticks":progressed,
            "saw_submitted":saw_submitted,"saw_completed":saw_completed,"observations":observations,
        })).unwrap()).unwrap();
    assert!(saw_submitted && saw_completed);
    assert_eq!(
        after_sleep.phase,
        SleepPhase::Awake,
        "bounded recovery must actually wake"
    );
    assert_eq!(
        after_sleep.cycles_completed,
        before_sleep.cycles_completed + 1
    );
    assert_eq!(
        after_sleep.last_consolidated_cycle_id,
        before_sleep.last_consolidated_cycle_id + 1
    );
    assert!(
        consolidated_changes > 0,
        "measured waking learning must reach lifetime GPU weights"
    );
    let world = runtime.world.canonical_signature_digest().unwrap();
    let biology = *runtime
        .world
        .organism_registry()
        .get(organism)
        .unwrap()
        .biochemistry();
    let prior = runtime
        .semantic_prior
        .as_ref()
        .unwrap()
        .snapshot(organism.raw())
        .unwrap();
    let saved = capture(&mut runtime, CaptureKind::Async);
    let neural = runtime
        .backend
        .snapshot_brain(handle, runtime.world.tick())
        .unwrap();
    let (manifest, loaded) =
        GpuDurableSaveManifest::open_loaded(root.join("live.json"), root.join("assets")).unwrap();
    assert_eq!(loaded.save, saved);
    let mut restored = GpuLiveBrainRuntime::restore_loaded_save(
        runtime.new_staging_like_live().unwrap(),
        manifest,
        loaded,
        saved.deterministic_seed,
        saved.config.brain_class,
    )
    .unwrap();
    let restored_handle = restored.handles[&organism.raw()];
    assert_eq!(restored_handle.class_id(), BrainCapacityClass::N2048_ID);
    assert_eq!(
        restored
            .backend
            .snapshot_brain(restored_handle, restored.world.tick())
            .unwrap(),
        neural
    );
    assert_eq!(restored.world.canonical_signature_digest().unwrap(), world);
    assert_eq!(
        *restored
            .world
            .organism_registry()
            .get(organism)
            .unwrap()
            .biochemistry(),
        biology
    );
    assert_eq!(
        restored
            .semantic_prior
            .as_ref()
            .unwrap()
            .snapshot(organism.raw())
            .unwrap(),
        prior
    );
    assert_eq!(restored.world.entity_id("readiness-teacher"), Some(teacher));
    assert!(!restored.handles.contains_key(&9000001));
    assert_eq!(restored.handles.len(), runtime.handles.len());
    assert!(
        runtime
            .memories
            .get(&organism.raw())
            .unwrap()
            .bank()
            .fast_len()
            > 0
    );
    assert_eq!(restored.memories, runtime.memories);
    assert_eq!(restored.topologies, runtime.topologies);
    assert_eq!(
        restored.residents[&organism.raw()].sleep_scheduler.state(),
        after_sleep
    );

    let next_deadline = Instant::now() + Duration::from_secs(60);
    readiness_tick(&mut runtime, next_deadline);
    readiness_tick(&mut restored, next_deadline);
    assert_eq!(
        restored.last_sealed_patches, runtime.last_sealed_patches,
        "the first ordinary post-wake response must match uninterrupted life"
    );
    assert_eq!(
        restored.world.canonical_signature_digest().unwrap(),
        runtime.world.canonical_signature_digest().unwrap()
    );
    assert_eq!(restored.memories, runtime.memories);
    assert_eq!(
        restored
            .backend
            .snapshot_brain(restored_handle, restored.world.tick())
            .unwrap(),
        runtime
            .backend
            .snapshot_brain(handle, runtime.world.tick())
            .unwrap()
    );
    assert!(!restored.handles.contains_key(&9000001));
    println!("READINESS_N2048 learned_fast_changes={learned_changes} consolidated_lifetime_changes={consolidated_changes} sleep_trigger=public_recovery exact_neural_world_memory_restore=true next_response_equal=true EVIDENCE={}", root.display());
}

fn readiness_tick(runtime: &mut GpuLiveBrainRuntime, deadline: Instant) {
    loop {
        assert!(
            Instant::now() < deadline,
            "bounded readiness tick did not progress"
        );
        match runtime.tick_outcome().unwrap() {
            GpuLiveTickOutcome::Progressed(_) => return,
            GpuLiveTickOutcome::NoProgress(_) => {
                runtime.poll_persistence_for_shutdown().unwrap();
                thread::sleep(Duration::from_millis(1));
            }
        }
    }
}
