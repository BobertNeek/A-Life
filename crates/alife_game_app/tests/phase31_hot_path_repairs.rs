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

#[path = "../src/production_voxel_renderer/phase31_performance_health.rs"]
mod phase31_performance_health;

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

fn sleep_journal_path(save_path: &Path) -> PathBuf {
    let file_name = save_path.file_name().unwrap().to_string_lossy();
    save_path.with_file_name(format!(".{file_name}.sleep-journal-v2.json"))
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
    Progressed {
        tick: Tick,
        compact_seals: usize,
    },
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
    assert_eq!(
        PortableAssetDigest::for_bytes(&replay_bytes),
        replay_ref.digest
    );
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

    let mut control_runtime = runtime.restored_clone_from_durability_for_test().unwrap();
    let control_before = control_runtime.performance_metrics();
    let control_result = control_runtime.tick();
    let control_after = control_runtime.performance_metrics();
    let compact_transition_reached = control_result.is_ok()
        && matches!(
            control_runtime
                .sleep_state_for_test(organism_id)
                .unwrap()
                .consolidation,
            ConsolidationState::Prepared { .. }
        )
        && control_after.sleep_compact_journal_organisms
            > control_before.sleep_compact_journal_organisms
        && control_after.sleep_checkpoint_capture_calls
            == control_before.sleep_checkpoint_capture_calls
        && control_after.sleep_checkpoint_readback_calls
            == control_before.sleep_checkpoint_readback_calls
        && control_after.sleep_checkpoint_readback_bytes
            == control_before.sleep_checkpoint_readback_bytes;
    drop(control_runtime);

    GpuDurableSaveManifest::publish_snapshot(save_path, asset_root, &checkpoint).unwrap();
    let durable = GpuDurableSaveManifest::open(save_path, asset_root).unwrap();
    let staging = runtime.new_staging_like_live().unwrap();
    runtime.replace_from_durable_save(staging, durable).unwrap();
    if !compact_transition_reached {
        return false;
    }

    let mut drift_runtime = runtime.restored_clone_from_durability_for_test().unwrap();
    let exact_save_before_drift = fs::read(save_path).unwrap();
    let journal_path = sleep_journal_path(save_path);
    let journal_before_drift = fs::read(&journal_path).unwrap();
    let metrics_before_drift = drift_runtime.performance_metrics();
    drift_runtime
        .force_compact_checkpoint_identity_drift_for_test(organism_id)
        .unwrap();
    assert!(matches!(
        drift_runtime.tick(),
        Err(GameAppShellError::GpuRuntime(
            alife_runtime::GpuRuntimeError::Core(
                ScaffoldContractError::BrainActivitySequenceMismatch
            )
        ))
    ));
    assert_eq!(fs::read(save_path).unwrap(), exact_save_before_drift);
    assert_eq!(fs::read(&journal_path).unwrap(), journal_before_drift);
    let metrics_after_drift = drift_runtime.performance_metrics();
    assert_eq!(
        metrics_after_drift.sleep_persistence_calls,
        metrics_before_drift.sleep_persistence_calls
    );
    assert_eq!(
        metrics_after_drift.sleep_checkpoint_capture_calls,
        metrics_before_drift.sleep_checkpoint_capture_calls
    );
    assert_eq!(
        metrics_after_drift.sleep_checkpoint_readback_calls,
        metrics_before_drift.sleep_checkpoint_readback_calls
    );
    assert_eq!(
        metrics_after_drift.sleep_checkpoint_readback_bytes,
        metrics_before_drift.sleep_checkpoint_readback_bytes
    );
    drop(drift_runtime);
    true
}

fn assert_anchored_journal_rolls_back_to_exact_base_if_present(
    runtime: &mut GpuLiveBrainRuntime,
    organisms: &[OrganismId],
    save_path: &Path,
    asset_root: &Path,
) -> bool {
    let durable = GpuDurableSaveManifest::open(save_path, asset_root).unwrap();
    let exact_base = durable.load().unwrap();
    let journal = durable.load_sleep_transaction_journal(&exact_base).unwrap();
    if journal.entries.is_empty() {
        return false;
    }
    let exact_save_bytes = fs::read(save_path).unwrap();
    let exact_sleep_states = organisms
        .iter()
        .map(|organism_id| {
            let sleep = exact_base
                .save
                .creatures
                .iter()
                .find(|creature| creature.organism_id == *organism_id)
                .and_then(|creature| creature.gpu_brain.as_ref())
                .map(|brain| brain.sleep)
                .expect("anchored exact base contains every resident brain");
            (*organism_id, sleep)
        })
        .collect::<Vec<_>>();
    for organism_id in organisms {
        if let Some(first) = journal
            .entries
            .iter()
            .find(|entry| entry.organism_id == *organism_id)
        {
            assert_eq!(
                first.source,
                exact_sleep_states
                    .iter()
                    .find(|(candidate, _)| candidate == organism_id)
                    .map(|(_, sleep)| *sleep)
                    .unwrap()
            );
        }
    }

    let staging = runtime.new_staging_like_live().unwrap();
    runtime.replace_from_durable_save(staging, durable).unwrap();
    assert_eq!(
        organisms
            .iter()
            .map(|organism_id| (
                *organism_id,
                runtime.sleep_state_for_test(*organism_id).unwrap()
            ))
            .collect::<Vec<_>>(),
        exact_sleep_states,
        "restore must discard journal-only future phases and install the exact base"
    );
    assert_eq!(fs::read(save_path).unwrap(), exact_save_bytes);
    let reopened = GpuDurableSaveManifest::open(save_path, asset_root).unwrap();
    let reopened_base = reopened.load().unwrap();
    assert!(reopened
        .load_sleep_transaction_journal(&reopened_base)
        .unwrap()
        .entries
        .is_empty());
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
    journal_paths: Option<(&Path, &Path)>,
) {
    let mut anchored_journal_rollback_proven = false;
    for _ in 0..96 {
        if !anchored_journal_rollback_proven {
            if let Some((save_path, asset_root)) = journal_paths {
                anchored_journal_rollback_proven =
                    assert_anchored_journal_rolls_back_to_exact_base_if_present(
                        runtime, organisms, save_path, asset_root,
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
        let metrics_before_tick = runtime.performance_metrics();
        let summaries = runtime.tick().unwrap();
        let metrics_after_tick = runtime.performance_metrics();
        if metrics_after_tick.sleep_checkpoint_capture_calls
            == metrics_before_tick.sleep_checkpoint_capture_calls
        {
            assert_eq!(
                metrics_after_tick.sleep_checkpoint_readback_calls,
                metrics_before_tick.sleep_checkpoint_readback_calls,
                "a non-exact sleep tick must not perform a hidden mutable readback"
            );
            assert_eq!(
                metrics_after_tick.sleep_checkpoint_readback_bytes,
                metrics_before_tick.sleep_checkpoint_readback_bytes,
                "a non-exact sleep tick must not move hidden mutable bytes"
            );
        }
        if metrics_after_tick.sleep_compact_journal_organisms
            > metrics_before_tick.sleep_compact_journal_organisms
        {
            assert_eq!(
                metrics_after_tick.sleep_checkpoint_capture_calls,
                metrics_before_tick.sleep_checkpoint_capture_calls
            );
            assert_eq!(
                metrics_after_tick.sleep_checkpoint_readback_calls,
                metrics_before_tick.sleep_checkpoint_readback_calls
            );
            assert_eq!(
                metrics_after_tick.sleep_checkpoint_readback_bytes,
                metrics_before_tick.sleep_checkpoint_readback_bytes
            );
        }
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
        assert!(
            summaries
                .iter()
                .find(|summary| summary.organism_id == sealed_organism)
                .expect("sealed topology organism has a tick summary")
                .patch_sealed
        );
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
            journal_paths.is_none() || anchored_journal_rollback_proven,
            "the canonical lifecycle never published and rolled back a compact sleep journal"
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
fn phase31_pending_compact_transition_is_reachable_and_drift_fails_closed() {
    let CanonicalRuntimeFixture {
        root,
        asset_root,
        save_path,
        mut runtime,
        organisms,
    } = canonical_runtime(31_082_707, 4);
    for _ in 0..64 {
        if assert_pending_replay_checkpoint_roundtrip(
            &mut runtime,
            &organisms,
            &save_path,
            &asset_root,
        ) {
            drop(runtime);
            fs::remove_dir_all(root).unwrap();
            return;
        }
        runtime.tick().unwrap();
    }
    panic!("no single-organism Pending to compact Prepared transition was reachable");
}

#[test]
fn phase31_one_sleep_cycle_has_one_exact_capture_and_journaled_promotion() {
    let CanonicalRuntimeFixture {
        root,
        asset_root,
        save_path,
        mut runtime,
        organisms,
        ..
    } = canonical_runtime(31_104, 4);
    let metrics_before = runtime.performance_metrics();
    drive_to_completed(&mut runtime, &organisms);
    let completed_metrics = runtime.performance_metrics();
    assert_eq!(
        completed_metrics
            .sleep_checkpoint_capture_calls
            .saturating_sub(metrics_before.sleep_checkpoint_capture_calls),
        1,
        "one sleep cycle must capture the exact population only at Submitted -> Completed"
    );
    assert_eq!(
        completed_metrics
            .sleep_exact_neural_capture_organisms
            .saturating_sub(metrics_before.sleep_exact_neural_capture_organisms),
        u64::try_from(organisms.len()).unwrap(),
        "the single exact boundary must capture each resident once"
    );

    let exact_completed_bytes = fs::read(&save_path).unwrap();
    let published = PortableSaveFile::from_json_file(&save_path).unwrap();
    assert!(organisms.iter().all(|organism_id| matches!(
        published
            .creatures
            .iter()
            .find(|creature| creature.organism_id == *organism_id)
            .and_then(|creature| creature.gpu_brain.as_ref())
            .map(|brain| brain.sleep.consolidation),
        Some(ConsolidationState::Completed { .. })
    )));

    let promotion_before = runtime.performance_metrics();
    runtime.tick().unwrap();
    let promotion_after = runtime.performance_metrics();
    assert_eq!(
        promotion_after.sleep_promotion_publish_calls,
        promotion_before.sleep_promotion_publish_calls,
        "Completed -> Committed must publish only the rollback journal"
    );
    assert_eq!(
        fs::read(&save_path).unwrap(),
        exact_completed_bytes,
        "journaled promotion must leave the exact Completed save byte-identical"
    );
    let committed_states = organisms
        .iter()
        .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
        .collect::<Vec<_>>();
    assert!(committed_states
        .iter()
        .all(|sleep| matches!(sleep.consolidation, ConsolidationState::Committed { .. })));

    let durable = GpuDurableSaveManifest::open(&save_path, &asset_root).unwrap();
    let exact_base = durable.load().unwrap();
    let journal = durable.load_sleep_transaction_journal(&exact_base).unwrap();
    assert!(journal.entries.iter().any(|entry| matches!(
        (entry.source.consolidation, entry.target.consolidation),
        (
            ConsolidationState::Completed { .. },
            ConsolidationState::Committed { .. }
        )
    )));
    let journal_path = sleep_journal_path(&save_path);
    let journal_bytes = fs::read(&journal_path).unwrap();
    let mut tampered = journal.clone();
    tampered.entries[0].organism_id = OrganismId(999);
    fs::write(&journal_path, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();
    assert!(durable.load_sleep_transaction_journal(&exact_base).is_err());
    assert_eq!(fs::read(&save_path).unwrap(), exact_completed_bytes);
    fs::write(&journal_path, &journal_bytes).unwrap();

    let mut restored = runtime.restored_clone_from_durability_for_test().unwrap();
    assert!(organisms.iter().all(|organism_id| matches!(
        restored
            .sleep_state_for_test(*organism_id)
            .unwrap()
            .consolidation,
        ConsolidationState::Completed { .. }
    )));
    restored.tick().unwrap();
    assert_eq!(
        organisms
            .iter()
            .map(|organism_id| restored.sleep_state_for_test(*organism_id).unwrap())
            .collect::<Vec<_>>(),
        committed_states,
        "restore must deterministically recommit from the exact Completed checkpoint"
    );
    assert_eq!(fs::read(&save_path).unwrap(), exact_completed_bytes);
    drop(restored);

    drop(runtime);
    fs::remove_dir_all(root).unwrap();

    let CanonicalRuntimeFixture {
        root,
        asset_root,
        save_path,
        mut runtime,
        organisms,
    } = canonical_runtime(31_105, 4);
    drive_to_completed(&mut runtime, &organisms);
    let exact_completed = PortableSaveFile::from_json_file(&save_path).unwrap();
    let expected_committed = organisms
        .iter()
        .map(|organism_id| {
            exact_completed
                .creatures
                .iter()
                .find(|creature| creature.organism_id == *organism_id)
                .and_then(|creature| creature.gpu_brain.as_ref())
                .unwrap()
                .promoted_completed_sleep_state()
                .unwrap()
                .sleep
        })
        .collect::<Vec<_>>();
    let durable = GpuDurableSaveManifest::open(&save_path, &asset_root).unwrap();
    let mut external = durable.load().unwrap().save;
    let recovery_staging = runtime.new_staging_like_live().unwrap();
    external.save_id = "phase31-valid-external-completed-base".to_string();
    GpuDurableSaveManifest::publish_snapshot(&save_path, &asset_root, &external).unwrap();
    let save_before_failure = fs::read(&save_path).unwrap();
    let journal_before_failure = fs::read(sleep_journal_path(&save_path)).unwrap();
    let completed_before_failure = organisms
        .iter()
        .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
        .collect::<Vec<_>>();
    assert!(matches!(
        runtime.tick(),
        Err(GameAppShellError::GpuRuntime(
            alife_game_app::GpuRuntimeError::GpuCheckpointManifestConflict { .. }
        ))
    ));
    assert_eq!(
        organisms
            .iter()
            .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
            .collect::<Vec<_>>(),
        completed_before_failure
    );
    assert_eq!(fs::read(&save_path).unwrap(), save_before_failure);
    assert_eq!(
        fs::read(sleep_journal_path(&save_path)).unwrap(),
        journal_before_failure
    );
    assert!(matches!(
        runtime.tick(),
        Err(GameAppShellError::Core(
            ScaffoldContractError::NeuralBackendUnavailable
        ))
    ));

    let durable = GpuDurableSaveManifest::open(&save_path, &asset_root).unwrap();
    runtime
        .replace_from_durable_save(recovery_staging, durable)
        .unwrap();
    assert!(organisms.iter().all(|organism_id| matches!(
        runtime
            .sleep_state_for_test(*organism_id)
            .unwrap()
            .consolidation,
        ConsolidationState::Completed { .. }
    )));
    runtime.tick().unwrap();
    assert_eq!(
        organisms
            .iter()
            .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
            .collect::<Vec<_>>(),
        expected_committed
    );
    assert_eq!(fs::read(&save_path).unwrap(), save_before_failure);
    drop(runtime);
    fs::remove_dir_all(root).unwrap();
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
    drive_to_first_compact_authority_seal(&mut no_capture_runtime, &no_capture_organisms, None);
    let without_capture =
        observe_checkpoint_continuity(&mut no_capture_runtime, &no_capture_organisms);
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
    assert!(
        sleep_checkpoint_metrics.sleep_persistence_calls
            > sleep_checkpoint_metrics.sleep_checkpoint_capture_calls,
        "non-neural journal boundaries must outnumber exact whole-save captures in the canonical lifecycle: {sleep_checkpoint_metrics:#?}"
    );
    assert!(
        sleep_checkpoint_metrics.sleep_compact_journal_organisms > 0,
        "non-neural sleep transitions must use compact journal publication"
    );
    assert_eq!(
        sleep_checkpoint_metrics.sleep_exact_neural_capture_organisms,
        sleep_checkpoint_metrics
            .sleep_checkpoint_capture_calls
            .saturating_mul(u64::try_from(organisms.len()).unwrap()),
        "every exact sleep boundary captures the current population once; journals capture no neural slots"
    );

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
    assert_eq!(
        fs::read(&success_save_path).unwrap(),
        manifest_before_late_failure
    );
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

#[test]
fn phase31_six_founder_runtime_survives_seventy_ticks_after_player_measurement_baseline() {
    let CanonicalRuntimeFixture {
        root,
        mut runtime,
        organisms,
        ..
    } = canonical_runtime(31_082_706, 6);
    while runtime.world_tick_for_test().raw() < 47 {
        runtime.tick().unwrap();
    }

    let mut mixed_promotion_and_journal_tick_observed = false;
    for measured_call in 1..=70 {
        let world_tick = runtime.world_tick_for_test();
        let before = sleep_generation_rows(&mut runtime, &organisms);
        if world_tick.raw() == 113 {
            assert_eq!(
                before
                    .iter()
                    .filter(|row| matches!(row.consolidation, ConsolidationState::Completed { .. }))
                    .count(),
                1
            );
            assert_eq!(
                before
                    .iter()
                    .filter(|row| matches!(row.consolidation, ConsolidationState::Prepared { .. }))
                    .count(),
                2
            );
        }
        if let Err(error) = runtime.tick() {
            panic!(
                "six-founder production runtime failed during measured call {measured_call} at world tick {}: {error:?}\nbefore={before:#?}\nafter={:#?}",
                world_tick.raw(),
                sleep_generation_rows(&mut runtime, &organisms),
            );
        }
        if world_tick.raw() == 113 {
            let after = sleep_generation_rows(&mut runtime, &organisms);
            assert_eq!(
                after
                    .iter()
                    .filter(|row| matches!(row.consolidation, ConsolidationState::Committed { .. }))
                    .count(),
                1
            );
            assert_eq!(
                after
                    .iter()
                    .filter(|row| matches!(row.consolidation, ConsolidationState::Submitted { .. }))
                    .count(),
                2
            );
            mixed_promotion_and_journal_tick_observed = true;
        }
    }
    assert!(mixed_promotion_and_journal_tick_observed);

    drop(runtime);
    fs::remove_dir_all(root).unwrap();

    let CanonicalRuntimeFixture {
        root,
        mut runtime,
        organisms,
        ..
    } = canonical_runtime(31_082_707, 4);
    drive_to_completed(&mut runtime, &organisms);
    runtime.force_late_advance_failure_for_test();
    assert!(matches!(
        runtime.tick(),
        Err(GameAppShellError::Core(
            ScaffoldContractError::NonMonotonicTick
        ))
    ));
    assert!(matches!(
        runtime.tick(),
        Err(GameAppShellError::Core(
            ScaffoldContractError::NeuralBackendUnavailable
        ))
    ));
    drop(runtime);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn phase31_receipt_rejects_dead_or_unaccounted_simulation() {
    use phase31_performance_health::validate_phase31_performance_authority;

    assert!(validate_phase31_performance_authority(false, true, 67, 67).is_ok());
    assert!(validate_phase31_performance_authority(true, true, 67, 67).is_err());
    assert!(validate_phase31_performance_authority(false, false, 67, 67).is_err());
    assert!(validate_phase31_performance_authority(false, true, 68, 67).is_err());
}
