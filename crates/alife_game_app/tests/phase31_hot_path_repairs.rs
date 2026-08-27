#![cfg(feature = "gpu-tests")]

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::{
    BoundedReplayBatch, BrainScaleTier, CanonicalDigestBuilder, ConsolidationState, OrganismId,
    ScaffoldContractError, SleepPhase, Tick,
};
use alife_game_app::{
    create_canonical_new_game_runtime, CanonicalNewGameLaunchRequest, GameAppShellError,
    GpuDurableSaveManifest, GpuLiveBrainRuntime,
};
use alife_world::persistence::PortableAssetDigest;
use alife_world::{AssetManifest, PortableSaveFile, RuntimeConfig};

fn isolated_root() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "alife-phase31-hot-path-{}-{nonce}",
        std::process::id()
    ))
}

struct CanonicalRuntimeFixture {
    root: PathBuf,
    asset_root: PathBuf,
    save_path: PathBuf,
    runtime: GpuLiveBrainRuntime,
    organisms: Vec<OrganismId>,
}

fn canonical_runtime(seed: u64, population: u16) -> CanonicalRuntimeFixture {
    let root = isolated_root();
    let asset_root = root.join("assets");
    let save_path = root.join("save.json");
    fs::create_dir_all(&asset_root).unwrap();
    let mut config = RuntimeConfig::deterministic_default(seed, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    let created = create_canonical_new_game_runtime(CanonicalNewGameLaunchRequest {
        world_seed: seed,
        population,
        save_path: save_path.clone(),
        asset_root: asset_root.clone(),
        config,
        assets: AssetManifest::empty(),
    })
    .unwrap();
    println!(
        "PHASE31_GPU_ADAPTER={} BACKEND_API=vulkan PROFILE=production_v1",
        created.runtime.authority_telemetry().adapter
    );
    let mut organisms = created
        .runtime
        .world_snapshot()
        .organism_registry()
        .iter()
        .map(|record| record.organism_id())
        .collect::<Vec<_>>();
    organisms.sort_by_key(|organism_id| organism_id.raw());
    assert_eq!(organisms.len(), usize::from(population));
    CanonicalRuntimeFixture {
        root,
        asset_root,
        save_path,
        runtime: created.runtime,
        organisms,
    }
}

fn drive_to_completed(runtime: &mut GpuLiveBrainRuntime, organisms: &[OrganismId]) {
    for _ in 0..64 {
        runtime.tick().unwrap();
        if organisms.iter().all(|organism_id| {
            matches!(
                runtime
                    .sleep_state_for_test(*organism_id)
                    .unwrap()
                    .consolidation,
                ConsolidationState::Completed { .. }
            )
        }) {
            return;
        }
    }
    panic!("all organisms must reach the exact durable Completed boundary");
}

#[allow(dead_code)]
#[derive(Debug)]
struct SleepGenerationRow {
    organism_id: u64,
    phase: SleepPhase,
    active_cycle_id: u64,
    consolidation: ConsolidationState,
    backend_active_weight_generation: Option<u64>,
    backend_transaction_generation: Option<u64>,
}

#[allow(dead_code)]
#[derive(Debug)]
enum ContinuityOutcome {
    Progressed { tick: Tick, compact_seals: usize },
    Failed {
        tick: Tick,
        error: String,
        before: Vec<SleepGenerationRow>,
        after: Vec<SleepGenerationRow>,
    },
    BoundExceeded {
        tick: Tick,
        rows: Vec<SleepGenerationRow>,
    },
}

fn sleep_generation_rows(
    runtime: &mut GpuLiveBrainRuntime,
    organisms: &[OrganismId],
) -> Vec<SleepGenerationRow> {
    organisms
        .iter()
        .map(|organism_id| {
            let sleep = runtime.sleep_state_for_test(*organism_id).unwrap();
            let learning = runtime.learning_state_for_test(*organism_id).ok();
            SleepGenerationRow {
                organism_id: organism_id.raw(),
                phase: sleep.phase,
                active_cycle_id: sleep.active_cycle_id,
                consolidation: sleep.consolidation,
                backend_active_weight_generation: learning
                    .as_ref()
                    .map(|state| state.active_weight_generation),
                backend_transaction_generation: learning
                    .as_ref()
                    .map(|state| state.transaction_generation),
            }
        })
        .collect()
}

fn assert_pending_replay_checkpoint_roundtrip(
    runtime: &mut GpuLiveBrainRuntime,
    organisms: &[OrganismId],
    save_path: &Path,
    asset_root: &Path,
) -> bool {
    let Some((organism_id, pending)) = organisms.iter().find_map(|organism_id| {
        let state = runtime.sleep_state_for_test(*organism_id).ok()?;
        matches!(state.consolidation, ConsolidationState::Pending { .. })
            .then_some((*organism_id, state.consolidation))
    }) else {
        return false;
    };
    let ConsolidationState::Pending {
        replay_digest,
        replay_event_count,
        replay_eligibility_sample_count,
        ..
    } = pending
    else {
        unreachable!();
    };
    let source_replay = runtime.sleep_replay_for_test(organism_id).unwrap();
    assert_eq!(replay_digest, source_replay.canonical_digest);
    assert_eq!(replay_event_count, source_replay.events.len() as u32);
    assert_eq!(
        replay_eligibility_sample_count,
        source_replay.eligibility_samples.len() as u32
    );

    let checkpoint = runtime.capture_portable_checkpoint().unwrap();
    let brain = checkpoint
        .creatures
        .iter()
        .find(|creature| creature.organism_id == organism_id)
        .and_then(|creature| creature.gpu_brain.as_ref())
        .expect("Pending organism has an exact GPU checkpoint");
    let replay_ref = brain
        .sleep_assets
        .replay_batch
        .as_ref()
        .expect("Pending checkpoint retains its replay asset");
    let replay_entry = checkpoint
        .assets
        .entries
        .iter()
        .find(|entry| entry.asset_id == replay_ref.asset_id)
        .expect("Pending replay asset has a manifest entry");
    assert_eq!(replay_entry.digest, replay_ref.digest);
    let replay_bytes = fs::read(asset_root.join(&replay_entry.relative_path)).unwrap();
    assert_eq!(PortableAssetDigest::for_bytes(&replay_bytes), replay_ref.digest);
    let saved_replay: BoundedReplayBatch = serde_json::from_slice(&replay_bytes).unwrap();
    assert_eq!(saved_replay, source_replay);
    assert_eq!(saved_replay.canonical_digest, replay_digest);

    GpuDurableSaveManifest::publish_snapshot(save_path, asset_root, &checkpoint).unwrap();
    let durable = GpuDurableSaveManifest::open(save_path, asset_root).unwrap();
    let staging = runtime.new_staging_like_live().unwrap();
    runtime.replace_from_durable_save(staging, durable).unwrap();
    let restored_replay = runtime.sleep_replay_for_test(organism_id).unwrap();
    let ConsolidationState::Pending {
        replay_digest: restored_digest,
        ..
    } = runtime
        .sleep_state_for_test(organism_id)
        .unwrap()
        .consolidation
    else {
        panic!("exact restore did not retain Pending authority");
    };
    assert_eq!(restored_digest, replay_digest);
    assert_eq!(restored_replay, saved_replay);
    true
}

fn observe_checkpoint_continuity(
    runtime: &mut GpuLiveBrainRuntime,
    organisms: &[OrganismId],
) -> ContinuityOutcome {
    let mut compact_seals = 0_usize;
    for _ in 0..64 {
        let tick = runtime.world_tick_for_test();
        let before = sleep_generation_rows(runtime, organisms);
        if let Err(error) = runtime.tick() {
            return ContinuityOutcome::Failed {
                tick,
                error: format!("{error:?}"),
                before,
                after: sleep_generation_rows(runtime, organisms),
            };
        }
        compact_seals = compact_seals.saturating_add(runtime.last_gpu_authority_receipts().len());
        if runtime.world_tick_for_test().raw() > 109 && compact_seals != 0 {
            return ContinuityOutcome::Progressed {
                tick: runtime.world_tick_for_test(),
                compact_seals,
            };
        }
    }
    ContinuityOutcome::BoundExceeded {
        tick: runtime.world_tick_for_test(),
        rows: sleep_generation_rows(runtime, organisms),
    }
}

fn drive_to_first_compact_authority_seal(
    runtime: &mut GpuLiveBrainRuntime,
    organisms: &[OrganismId],
    pending_roundtrip_paths: Option<(&Path, &Path)>,
) {
    let mut pending_roundtrip_proven = false;
    for _ in 0..96 {
        if !pending_roundtrip_proven {
            if let Some((save_path, asset_root)) = pending_roundtrip_paths {
                pending_roundtrip_proven = assert_pending_replay_checkpoint_roundtrip(
                    runtime,
                    organisms,
                    save_path,
                    asset_root,
                );
            }
        }
        let brain_digests_before = organisms
            .iter()
            .map(|organism_id| {
                (
                    *organism_id,
                    runtime
                        .world_snapshot()
                        .organism_registry()
                        .get(*organism_id)
                        .unwrap()
                        .state_graph()
                        .brain
                        .content_digest,
                )
            })
            .collect::<Vec<_>>();
        let summaries = runtime.tick().unwrap();
        let authority_receipts = runtime.last_gpu_authority_receipts();
        if authority_receipts.is_empty() {
            continue;
        }
        let topology_receipts = runtime
            .last_topology_observations()
            .iter()
            .filter_map(|disposition| disposition.receipt())
            .collect::<Vec<_>>();
        assert_eq!(authority_receipts.len(), topology_receipts.len());
        let topology = topology_receipts[0];
        let sealed_organism = OrganismId(topology.organism_id_raw);
        assert!(summaries
            .iter()
            .find(|summary| summary.organism_id == sealed_organism)
            .expect("sealed topology organism has a tick summary")
            .patch_sealed);
        let authority = &authority_receipts[0];
        authority.validate().unwrap();
        assert_ne!(topology.before_digest, topology.after_digest);
        let mut expected_brain = CanonicalDigestBuilder::new(b"alife.live-brain-authority.v4");
        for word in authority.receipt_digest() {
            expected_brain.write_u64(word);
        }
        let topology_digest = runtime
            .topology_sidecar_for_test(sealed_organism)
            .unwrap()
            .diagnostics()
            .canonical_digest;
        for word in topology_digest {
            expected_brain.write_u64(word);
        }
        let brain_before = brain_digests_before
            .iter()
            .find(|(organism_id, _)| *organism_id == sealed_organism)
            .map(|(_, digest)| *digest)
            .expect("sealed organism has a pre-tick brain digest");
        let brain_after = runtime
            .world_snapshot()
            .organism_registry()
            .get(sealed_organism)
            .unwrap()
            .state_graph()
            .brain
            .content_digest;
        assert_ne!(brain_after, brain_before);
        assert_eq!(brain_after, expected_brain.finish256());
        assert_eq!(runtime.performance_metrics().ordinary_snapshot_calls, 0);
        assert_eq!(runtime.performance_metrics().ordinary_snapshot_bytes, 0);
        assert_eq!(runtime.performance_metrics().resident_json_bytes, 0);
        assert_eq!(runtime.performance_metrics().topology_json_bytes, 0);
        assert!(
            pending_roundtrip_paths.is_none() || pending_roundtrip_proven,
            "the canonical seal lifecycle never reached a Pending replay checkpoint"
        );
        return;
    }
    let world = runtime.world_snapshot();
    let final_rows = organisms
        .iter()
        .map(|organism_id| {
            let sleep = runtime.sleep_state_for_test(*organism_id).unwrap();
            let record = world.organism_registry().get(*organism_id).unwrap();
            (
                organism_id.raw(),
                sleep.phase,
                sleep.active_cycle_id,
                record.biochemistry().homeostasis.drives.brain_atp,
                record
                    .phenotype()
                    .chemistry
                    .endocrine
                    .parameters
                    .catatonia_brain_atp_threshold,
            )
        })
        .collect::<Vec<_>>();
    panic!(
        "canonical runtime emitted no compact GPU authority within 96 ticks: world_tick={:?} rows={final_rows:?} inference_rows={}",
        world.tick(),
        runtime.performance_metrics().inference_rows
    );
}

#[test]
fn phase31_hot_path_completed_sleep_batch_is_atomic_and_keeps_exact_snapshot_boundaries() {
    let CanonicalRuntimeFixture {
        root: continuity_root,
        asset_root: continuity_asset_root,
        save_path: continuity_save_path,
        mut runtime,
        organisms,
    } = canonical_runtime(31_082_706, 6);

    drive_to_first_compact_authority_seal(
        &mut runtime,
        &organisms,
        Some((&continuity_save_path, &continuity_asset_root)),
    );

    let checkpoint_before = runtime.performance_metrics();
    let _checkpoint = runtime.capture_portable_checkpoint().unwrap();
    let checkpoint_after = runtime.performance_metrics();
    assert!(
        checkpoint_after.checkpoint_snapshot_calls > checkpoint_before.checkpoint_snapshot_calls
    );
    assert!(
        checkpoint_after.checkpoint_snapshot_bytes > checkpoint_before.checkpoint_snapshot_bytes
    );

    let with_capture = observe_checkpoint_continuity(&mut runtime, &organisms);

    let CanonicalRuntimeFixture {
        root: no_capture_root,
        runtime: mut no_capture_runtime,
        organisms: no_capture_organisms,
        ..
    } = canonical_runtime(31_082_706, 6);
    drive_to_first_compact_authority_seal(
        &mut no_capture_runtime,
        &no_capture_organisms,
        None,
    );
    let without_capture = observe_checkpoint_continuity(&mut no_capture_runtime, &no_capture_organisms);
    drop(no_capture_runtime);
    fs::remove_dir_all(no_capture_root).unwrap();

    assert!(
        matches!(without_capture, ContinuityOutcome::Progressed { .. }),
        "no-capture control must pass the failing tick and seal again: {without_capture:#?}"
    );
    assert!(
        matches!(with_capture, ContinuityOutcome::Progressed { .. }),
        "successful checkpoint capture must pass the failing tick and seal again: {with_capture:#?}"
    );

    let sleep_checkpoint_metrics = runtime.performance_metrics();
    assert!(sleep_checkpoint_metrics.sleep_checkpoint_readback_calls > 0);
    assert!(sleep_checkpoint_metrics.sleep_checkpoint_readback_bytes > 0);

    drop(runtime);
    fs::remove_dir_all(continuity_root).unwrap();

    let CanonicalRuntimeFixture {
        root: success_root,
        save_path: success_save_path,
        mut runtime,
        organisms,
        ..
    } = canonical_runtime(31_103, 4);
    drive_to_completed(&mut runtime, &organisms);

    let scheduler_before_late_failure = organisms
        .iter()
        .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
        .collect::<Vec<_>>();
    let world_before_late_failure = runtime.world_snapshot();
    let world_digest_before_late_failure = world_before_late_failure
        .canonical_signature_digest()
        .unwrap();
    let memories_before_late_failure = organisms
        .iter()
        .map(|organism_id| {
            runtime
                .memory_sidecar_for_test(*organism_id)
                .unwrap()
                .clone()
        })
        .collect::<Vec<_>>();
    let replay_before_late_failure = runtime.restored_replay_patches_for_test().to_vec();
    let manifest_before_late_failure = fs::read(&success_save_path).unwrap();
    let metrics_before_late_failure = runtime.performance_metrics();
    runtime.force_memory_preparation_failure_for_test(*organisms.last().unwrap());
    assert!(matches!(
        runtime.tick(),
        Err(GameAppShellError::Core(
            ScaffoldContractError::InvalidMemoryQuery
        ))
    ));
    assert_eq!(
        runtime.last_sleep_memory_compaction_preparation_count_for_test(),
        organisms.len() - 1,
        "the injected last-organism failure must follow N-1 validated cloned compactions"
    );
    assert_eq!(
        organisms
            .iter()
            .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
            .collect::<Vec<_>>(),
        scheduler_before_late_failure
    );
    assert_eq!(
        runtime
            .world_snapshot()
            .canonical_signature_digest()
            .unwrap(),
        world_digest_before_late_failure
    );
    assert_eq!(
        organisms
            .iter()
            .map(|organism_id| runtime
                .memory_sidecar_for_test(*organism_id)
                .unwrap()
                .clone())
            .collect::<Vec<_>>(),
        memories_before_late_failure
    );
    assert_eq!(
        runtime.restored_replay_patches_for_test(),
        replay_before_late_failure
    );
    assert_eq!(fs::read(&success_save_path).unwrap(), manifest_before_late_failure);
    assert_eq!(
        runtime.performance_metrics().sleep_promotion_publish_calls,
        metrics_before_late_failure.sleep_promotion_publish_calls
    );

    let metrics_before_success = runtime.performance_metrics();
    runtime.tick().unwrap();
    assert_eq!(
        runtime.performance_metrics().sleep_promotion_publish_calls,
        metrics_before_success.sleep_promotion_publish_calls + 1
    );
    assert_eq!(
        runtime.last_memory_compaction_receipts().len(),
        organisms.len()
    );
    assert!(organisms.iter().all(|organism_id| matches!(
        runtime
            .sleep_state_for_test(*organism_id)
            .unwrap()
            .consolidation,
        ConsolidationState::Committed { .. }
    )));
    let published = PortableSaveFile::from_json_file(&success_save_path).unwrap();
    assert!(organisms.iter().all(|organism_id| matches!(
        published
            .creatures
            .iter()
            .find(|creature| creature.organism_id == *organism_id)
            .and_then(|creature| creature.gpu_brain.as_ref())
            .map(|brain| brain.sleep.consolidation),
        Some(ConsolidationState::Committed { .. })
    )));

    drop(runtime);
    fs::remove_dir_all(success_root).unwrap();

    let CanonicalRuntimeFixture {
        root: conflict_root,
        asset_root,
        save_path,
        mut runtime,
        organisms,
    } = canonical_runtime(31_102, 4);
    drive_to_completed(&mut runtime, &organisms);

    let durable = GpuDurableSaveManifest::open(&save_path, &asset_root).unwrap();
    let mut external_replacement = durable.load().unwrap().save;
    external_replacement.save_id = "phase31-external-valid-cas-replacement".to_string();
    GpuDurableSaveManifest::publish_snapshot(&save_path, &asset_root, &external_replacement)
        .unwrap();

    let scheduler_before = organisms
        .iter()
        .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
        .collect::<Vec<_>>();
    let world_before = runtime.world_snapshot();
    let world_digest_before = world_before.canonical_signature_digest().unwrap();
    let records_before = organisms
        .iter()
        .map(|organism_id| {
            world_before
                .organism_registry()
                .get(*organism_id)
                .unwrap()
                .clone()
        })
        .collect::<Vec<_>>();
    let memories_before = organisms
        .iter()
        .map(|organism_id| {
            runtime
                .memory_sidecar_for_test(*organism_id)
                .unwrap()
                .clone()
        })
        .collect::<Vec<_>>();
    let replay_before = runtime.restored_replay_patches_for_test().to_vec();
    let manifest_before = fs::read(&save_path).unwrap();
    let metrics_before = runtime.performance_metrics();
    let conflict = runtime.tick().unwrap_err();
    assert!(
        matches!(
            conflict,
            GameAppShellError::GpuRuntime(
                alife_game_app::GpuRuntimeError::GpuCheckpointManifestConflict { .. }
            )
        ),
        "expected promotion CAS conflict, got {conflict:?}"
    );
    assert_eq!(
        organisms
            .iter()
            .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
            .collect::<Vec<_>>(),
        scheduler_before
    );
    let world_after = runtime.world_snapshot();
    assert_eq!(
        world_after.canonical_signature_digest().unwrap(),
        world_digest_before
    );
    assert_eq!(
        organisms
            .iter()
            .map(|organism_id| world_after
                .organism_registry()
                .get(*organism_id)
                .unwrap()
                .clone())
            .collect::<Vec<_>>(),
        records_before
    );
    assert_eq!(fs::read(&save_path).unwrap(), manifest_before);
    assert_eq!(
        organisms
            .iter()
            .map(|organism_id| runtime
                .memory_sidecar_for_test(*organism_id)
                .unwrap()
                .clone())
            .collect::<Vec<_>>(),
        memories_before
    );
    assert_eq!(runtime.restored_replay_patches_for_test(), replay_before);
    assert_eq!(
        runtime.performance_metrics().sleep_promotion_publish_calls,
        metrics_before.sleep_promotion_publish_calls
    );
    assert!(matches!(
        runtime.tick(),
        Err(GameAppShellError::Core(
            ScaffoldContractError::NeuralBackendUnavailable
        ))
    ));
    drop(runtime);
    drop(durable);
    fs::remove_dir_all(conflict_root).unwrap();
}
