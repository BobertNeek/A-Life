#![cfg(feature = "gpu-tests")]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use alife_core::{
    BodyEventDelta, BoundedReplayBatch, BrainScaleTier, CanonicalDigestBuilder, ConsolidationState,
    OrganismId, ScaffoldContractError, SleepPhase, SleepTrigger, Tick,
};
use alife_game_app::{
    create_canonical_new_game_runtime, CanonicalNewGameLaunchRequest, GameAppShellError,
    GpuDurableSaveManifest, GpuLiveBrainRuntime,
};
use alife_world::persistence::PortableAssetDigest;
use alife_world::{AssetManifest, RuntimeConfig};

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

fn authority_journal_artifact_path(save_path: &Path) -> std::path::PathBuf {
    let pointer: serde_json::Value = serde_json::from_slice(&fs::read(save_path).unwrap()).unwrap();
    let file_name = pointer["gpu_checkpoint_authority"]["journal"]["file_name"]
        .as_str()
        .unwrap();
    save_path.parent().unwrap().join(file_name)
}

#[test]
fn phase31_pre_worker_checkpoint_failure_cannot_silently_return_to_idle() {
    let mut fixture = canonical_runtime(31_082_706, 6);
    fixture
        .runtime
        .force_exact_checkpoint_pre_worker_transition_failure_for_test()
        .unwrap();

    let mut failure = None;
    for _ in 0..256 {
        match fixture.runtime.tick() {
            Ok(_) => std::thread::yield_now(),
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }

    assert!(failure.is_some(), "forced pre-worker failure must surface");
    assert!(fixture.runtime.exact_checkpoint_failed_for_test());
    assert!(fixture.runtime.tick().is_err());
    drop(fixture.runtime);
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn phase31_prospective_permit_failure_precedes_the_public_checkpoint_cas() {
    let mut fixture = canonical_runtime(31_082_706, 6);
    let pointer_before = fs::read(&fixture.save_path).unwrap();
    let generation_before = GpuDurableSaveManifest::open(&fixture.save_path, &fixture.asset_root)
        .unwrap()
        .load()
        .unwrap()
        .authority_generation();
    fixture
        .runtime
        .force_exact_checkpoint_permit_prevalidation_failure_for_test()
        .unwrap();

    let mut failure = None;
    let started = Instant::now();
    let deadline = started + Duration::from_secs(90);
    while Instant::now() < deadline {
        match fixture.runtime.tick() {
            Ok(_) => {
                if let Err(error) = fixture.runtime.poll_exact_checkpoint_for_test() {
                    failure = Some(error);
                    break;
                }
                std::thread::park_timeout(Duration::from_millis(1));
            }
            Err(error) => {
                failure = Some(error);
                break;
            }
        }
    }

    let diagnostics =
        checkpoint_wait_diagnostics(&mut fixture.runtime, &fixture.organisms, started);
    assert!(
        matches!(
            failure.as_ref(),
            Some(GameAppShellError::Core(
                ScaffoldContractError::BrainActivitySequenceMismatch
            ))
        ),
        "prospective permit failure={failure:?}; {diagnostics}"
    );
    assert!(fixture.runtime.exact_checkpoint_failed_for_test());
    assert_eq!(fs::read(&fixture.save_path).unwrap(), pointer_before);
    let still_authoritative = GpuDurableSaveManifest::open(&fixture.save_path, &fixture.asset_root)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(
        still_authoritative.authority_generation(),
        generation_before
    );
    drop(fixture.runtime);
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn phase31_post_journal_authority_survives_finish_for_the_next_ordinary_edge() {
    fn tick_with_checkpoint_poll_opportunity(
        runtime: &mut GpuLiveBrainRuntime,
        organisms: &[OrganismId],
    ) {
        let active_before = runtime.exact_checkpoint_active_tick_for_test().is_some();
        let tick_before = runtime.world_tick_for_test();
        let sleep_before = sleep_generation_rows(runtime, organisms);
        if let Err(error) = runtime.tick() {
            let active = runtime.exact_checkpoint_active_tick_for_test();
            let state = runtime.exact_checkpoint_state_for_test();
            let pending = runtime
                .pending_exact_sleep_journal_entries_for_test()
                .to_vec();
            let sleep_after = sleep_generation_rows(runtime, organisms);
            panic!(
                "exact/journal runtime failed at world tick {}: {error:?}; state={state:?}; active={:?}; pending={:#?}; sleep_before={sleep_before:#?}; sleep_after={:#?}",
                tick_before.raw(),
                active,
                pending,
                sleep_after,
            );
        }
        if active_before || runtime.exact_checkpoint_active_tick_for_test().is_some() {
            let deadline = Instant::now() + Duration::from_millis(50);
            while Instant::now() < deadline {
                std::thread::yield_now();
            }
        }
    }

    let mut fixture = canonical_runtime(31_082_706, 6);
    let commit_deadline = Instant::now() + Duration::from_secs(30);
    let mut saw_all_committed = false;
    while Instant::now() < commit_deadline {
        tick_with_checkpoint_poll_opportunity(&mut fixture.runtime, &fixture.organisms);
        saw_all_committed = fixture.organisms.iter().all(|organism_id| {
            matches!(
                fixture
                    .runtime
                    .sleep_state_for_test(*organism_id)
                    .unwrap()
                    .consolidation,
                ConsolidationState::Committed { .. }
            )
        });
        if saw_all_committed {
            break;
        }
    }
    assert!(
        saw_all_committed,
        "the canonical cycle must reach Completed to Committed"
    );
    let drain_deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < drain_deadline
        && !fixture
            .runtime
            .persistence_idle_for_shutdown_for_test()
    {
        fixture
            .runtime
            .poll_persistence_for_shutdown_for_test()
            .unwrap();
        std::thread::park_timeout(Duration::from_millis(1));
    }
    assert!(fixture
        .runtime
        .persistence_idle_for_shutdown_for_test());
    let committed_generation =
        GpuDurableSaveManifest::open(&fixture.save_path, &fixture.asset_root)
            .unwrap()
            .load()
            .unwrap()
            .authority_generation();
    let metrics_before_ordinary_edge = fixture.runtime.performance_metrics();

    let mut ordinary_generation = None;
    let ordinary_edge_deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < ordinary_edge_deadline {
        tick_with_checkpoint_poll_opportunity(&mut fixture.runtime, &fixture.organisms);
        let generation = GpuDurableSaveManifest::open(&fixture.save_path, &fixture.asset_root)
            .unwrap()
            .load()
            .unwrap()
            .authority_generation();
        if generation > committed_generation {
            ordinary_generation = Some(generation);
            break;
        }
    }
    let ordinary_generation =
        ordinary_generation.expect("the next canonical Committed phase edge must publish");
    assert!(ordinary_generation > committed_generation);
    assert!(fixture
        .runtime
        .exact_checkpoint_active_tick_for_test()
        .is_none());
    let metrics_after_ordinary_edge = fixture.runtime.performance_metrics();
    assert_eq!(
        metrics_after_ordinary_edge.sleep_checkpoint_readback_calls,
        metrics_before_ordinary_edge.sleep_checkpoint_readback_calls,
        "the post-finish compact edge must not use the blocking checkpoint readback path"
    );
    assert_eq!(
        metrics_after_ordinary_edge.sleep_checkpoint_readback_bytes,
        metrics_before_ordinary_edge.sleep_checkpoint_readback_bytes,
        "the post-finish compact edge must remain readback-free"
    );

    drop(fixture.runtime);
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn phase31_shutdown_drain_finalizes_a_durable_completed_checkpoint_without_another_tick() {
    let mut fixture = canonical_runtime(31_082_706, 6);
    let permitted = drive_to_durable_completed(&mut fixture.runtime, &fixture.organisms);
    assert!(!permitted.is_empty());
    let quiesced_tick = fixture.runtime.world_tick_for_test();
    let durable_tick = GpuDurableSaveManifest::open(&fixture.save_path, &fixture.asset_root)
        .unwrap()
        .load()
        .unwrap()
        .save
        .world
        .tick;

    let started = Instant::now();
    let deadline = started + Duration::from_secs(30);
    while Instant::now() < deadline
        && !fixture
            .runtime
            .persistence_idle_for_shutdown_for_test()
    {
        fixture
            .runtime
            .poll_persistence_for_shutdown_for_test()
            .unwrap();
        std::thread::park_timeout(Duration::from_millis(1));
    }

    assert!(
        fixture
            .runtime
            .persistence_idle_for_shutdown_for_test(),
        "shutdown polling must drain AwaitingJournal without admitting another simulation tick; {}",
        checkpoint_wait_diagnostics(&mut fixture.runtime, &fixture.organisms, started)
    );
    assert_eq!(fixture.runtime.world_tick_for_test(), quiesced_tick);
    let durable = GpuDurableSaveManifest::open(&fixture.save_path, &fixture.asset_root)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(durable.save.world.tick, durable_tick);
    assert!(permitted.iter().all(|organism_id| {
        durable
            .save
            .creatures
            .iter()
            .find(|creature| creature.organism_id == *organism_id)
            .and_then(|creature| creature.gpu_brain.as_ref())
            .is_some_and(|brain| {
                matches!(brain.sleep.consolidation, ConsolidationState::Completed { .. })
            })
    }));

    drop(fixture.runtime);
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn phase31_async_journal_publication_survives_later_cycles_then_drains() {
    let mut fixture = canonical_runtime(31_082_706, 6);
    fixture.runtime.set_performance_measurement_enabled(true);
    let started = Instant::now();
    let tick_deadline = started + Duration::from_secs(180);
    while fixture.runtime.world_tick_for_test().raw() < 750 && Instant::now() < tick_deadline {
        if let Err(error) = fixture.runtime.tick() {
            panic!(
                "later-cycle async journal tick failed at {}: {error:?}; {}",
                fixture.runtime.world_tick_for_test().raw(),
                checkpoint_wait_diagnostics(&mut fixture.runtime, &fixture.organisms, started)
            );
        }
        // Match production's bounded catch-up shape: four immediate tick
        // attempts followed by one render-frame interval.
        if fixture.runtime.world_tick_for_test().raw() % 4 == 0 {
            std::thread::park_timeout(Duration::from_millis(16));
        }
    }
    assert!(fixture.runtime.world_tick_for_test().raw() >= 750);

    let quiesced_tick = fixture.runtime.world_tick_for_test();
    let drain_deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < drain_deadline
        && !fixture
            .runtime
            .persistence_idle_for_shutdown_for_test()
    {
        fixture
            .runtime
            .poll_persistence_for_shutdown_for_test()
            .unwrap();
        std::thread::park_timeout(Duration::from_millis(1));
    }
    assert!(
        fixture
            .runtime
            .persistence_idle_for_shutdown_for_test(),
        "later-cycle persistence must drain: {}",
        checkpoint_wait_diagnostics(&mut fixture.runtime, &fixture.organisms, started)
    );
    assert_eq!(fixture.runtime.world_tick_for_test(), quiesced_tick);

    drop(fixture.runtime);
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn phase31_shutdown_poll_releases_a_stranded_exact_journal_wait() {
    let mut fixture = canonical_runtime(31_082_706, 6);
    assert!(fixture
        .runtime
        .persistence_idle_for_shutdown_for_test());
    fixture
        .runtime
        .force_stranded_exact_journal_wait_for_test();
    assert!(!fixture
        .runtime
        .persistence_idle_for_shutdown_for_test());

    fixture
        .runtime
        .poll_persistence_for_shutdown_for_test()
        .unwrap();
    assert!(fixture
        .runtime
        .exact_checkpoint_active_tick_for_test()
        .is_some());

    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline
        && !fixture
            .runtime
            .persistence_idle_for_shutdown_for_test()
    {
        fixture
            .runtime
            .poll_persistence_for_shutdown_for_test()
            .unwrap();
        std::thread::park_timeout(Duration::from_millis(1));
    }
    assert!(fixture
        .runtime
        .persistence_idle_for_shutdown_for_test());

    drop(fixture.runtime);
    fs::remove_dir_all(fixture.root).unwrap();
}

#[test]
fn phase31_natural_later_journal_edge_forces_exactly_one_follow_up_capture() {
    let mut fixture = canonical_runtime(31_082_706, 6);
    let started = Instant::now();
    let deadline = started + Duration::from_secs(90);
    let mut observed_later = None;
    while Instant::now() < deadline {
        if let Some(capture_tick) = fixture.runtime.exact_checkpoint_active_tick_for_test() {
            if let Some(entry) = fixture
                .runtime
                .pending_exact_sleep_journal_entries_for_test()
                .iter()
                .find(|entry| entry.transition_tick > capture_tick)
                .cloned()
            {
                observed_later = Some((capture_tick, entry));
                break;
            }
        }
        fixture.runtime.tick().unwrap();
        fixture.runtime.poll_exact_checkpoint_for_test().unwrap();
        std::thread::park_timeout(Duration::from_millis(1));
    }
    let diagnostics =
        checkpoint_wait_diagnostics(&mut fixture.runtime, &fixture.organisms, started);
    let (first_capture_tick, later_entry) = observed_later.unwrap_or_else(|| {
        panic!(
            "the natural canonical lifecycle must queue a real journal edge newer than capture T: {diagnostics}"
        )
    });
    let follow_up_deadline = Instant::now() + Duration::from_secs(90);

    let mut saw_follow_up_bit = false;
    let mut follow_up = None;
    let mut capture_ticks = vec![first_capture_tick];
    while Instant::now() < follow_up_deadline {
        saw_follow_up_bit |= fixture.runtime.exact_checkpoint_follow_up_queued_for_test();
        if let Some(active_tick) = fixture.runtime.exact_checkpoint_active_tick_for_test() {
            if active_tick != *capture_ticks.last().unwrap() {
                capture_ticks.push(active_tick);
                assert_eq!(
                    capture_ticks.len(),
                    2,
                    "only one follow-up capture is bounded"
                );
                follow_up = Some((
                    active_tick,
                    fixture
                        .runtime
                        .sleep_state_for_test(later_entry.organism_id)
                        .unwrap(),
                ));
                break;
            }
        }
        let tick_before = fixture.runtime.world_tick_for_test();
        let sleep_before = sleep_generation_rows(&mut fixture.runtime, &fixture.organisms);
        if let Err(error) = fixture.runtime.tick() {
            let active = fixture.runtime.exact_checkpoint_active_tick_for_test();
            let follow_up_queued = fixture.runtime.exact_checkpoint_follow_up_queued_for_test();
            let pending = fixture
                .runtime
                .pending_exact_sleep_journal_entries_for_test()
                .to_vec();
            let sleep_after = sleep_generation_rows(&mut fixture.runtime, &fixture.organisms);
            panic!(
                "follow-up runtime failed at tick {}: {error:?}; first_capture={}; later_entry={later_entry:#?}; active={:?}; follow_up={}; pending={:#?}; sleep_before={sleep_before:#?}; sleep_after={:#?}",
                tick_before.raw(),
                first_capture_tick.raw(),
                active,
                follow_up_queued,
                pending,
                sleep_after,
            );
        }
        fixture.runtime.poll_exact_checkpoint_for_test().unwrap();
        std::thread::park_timeout(Duration::from_millis(1));
    }
    assert!(
        saw_follow_up_bit,
        "the later edge must set the one-bit follow-up request"
    );
    let (follow_up_tick, follow_up_sleep) =
        follow_up.expect("the first checkpoint completion must start one follow-up capture");
    assert!(follow_up_tick > first_capture_tick);

    let loaded_follow_up = loop {
        assert!(
            Instant::now() < follow_up_deadline,
            "the follow-up exact save must publish"
        );
        let loaded = GpuDurableSaveManifest::open(&fixture.save_path, &fixture.asset_root)
            .unwrap()
            .load()
            .unwrap();
        if loaded.save.world.tick == follow_up_tick {
            break loaded;
        }
        fixture.runtime.tick().unwrap();
        fixture.runtime.poll_exact_checkpoint_for_test().unwrap();
        std::thread::park_timeout(Duration::from_millis(1));
        if let Some(active_tick) = fixture.runtime.exact_checkpoint_active_tick_for_test() {
            assert!(
                active_tick == follow_up_tick || active_tick == first_capture_tick,
                "a second follow-up capture must not start"
            );
        }
    };
    let saved_sleep = loaded_follow_up
        .save
        .creatures
        .iter()
        .find(|creature| creature.organism_id == later_entry.organism_id)
        .and_then(|creature| creature.gpu_brain.as_ref())
        .map(|brain| brain.sleep)
        .unwrap();
    assert_eq!(saved_sleep, follow_up_sleep);
    let journal = GpuDurableSaveManifest::open(&fixture.save_path, &fixture.asset_root)
        .unwrap()
        .load_sleep_transaction_journal(&loaded_follow_up)
        .unwrap();
    assert!(
        journal
            .entries
            .iter()
            .filter(|entry| entry.entry_digest == later_entry.entry_digest)
            .count()
            <= 1,
        "the absorbed later edge must not be duplicated"
    );

    drop(fixture.runtime);
    fs::remove_dir_all(fixture.root).unwrap();
}

fn drive_to_durable_completed(
    runtime: &mut GpuLiveBrainRuntime,
    organisms: &[OrganismId],
) -> Vec<OrganismId> {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline && runtime.world_tick_for_test().raw() < 256 {
        runtime.poll_exact_checkpoint_for_test().unwrap();
        let permitted = runtime.durable_completed_sleep_permitted_ids_for_test();
        if !permitted.is_empty() {
            assert!(permitted
                .iter()
                .all(|organism_id| organisms.contains(organism_id)));
            return permitted;
        }
        runtime.tick().unwrap();
        if runtime.exact_checkpoint_active_tick_for_test().is_some() {
            let poll_deadline = Instant::now() + Duration::from_millis(50);
            while Instant::now() < poll_deadline {
                std::thread::yield_now();
            }
        }
    }
    panic!("a canonical organism must reach an exact durable Completed boundary");
}

#[allow(dead_code)]
#[derive(Debug)]
struct SleepGenerationRow {
    organism_id: u64,
    phase: SleepPhase,
    last_trigger: Option<SleepTrigger>,
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
                last_trigger: sleep.last_trigger,
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

fn checkpoint_wait_diagnostics(
    runtime: &mut GpuLiveBrainRuntime,
    organisms: &[OrganismId],
    started: Instant,
) -> String {
    format!(
        "elapsed={:?} world_tick={} coordinator={:?} active_tick={:?} follow_up={} pending_journal={} capture={:?} rows={:#?}",
        started.elapsed(),
        runtime.world_tick_for_test().raw(),
        runtime.exact_checkpoint_state_for_test(),
        runtime.exact_checkpoint_active_tick_for_test(),
        runtime.exact_checkpoint_follow_up_queued_for_test(),
        runtime.pending_exact_sleep_journal_entries_for_test().len(),
        runtime.exact_population_capture_metrics_for_test(),
        sleep_generation_rows(runtime, organisms),
    )
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
    let durable_before_drift = GpuDurableSaveManifest::open(save_path, asset_root)
        .unwrap()
        .load()
        .unwrap();
    let journal_before_drift = GpuDurableSaveManifest::open(save_path, asset_root)
        .unwrap()
        .load_sleep_transaction_journal(&durable_before_drift)
        .unwrap();
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
    let durable_after_drift = GpuDurableSaveManifest::open(save_path, asset_root)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(durable_after_drift, durable_before_drift);
    assert_eq!(
        GpuDurableSaveManifest::open(save_path, asset_root)
            .unwrap()
            .load_sleep_transaction_journal(&durable_after_drift)
            .unwrap(),
        journal_before_drift
    );
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
    let reopened = GpuDurableSaveManifest::open(save_path, asset_root).unwrap();
    let reopened_base = reopened.load().unwrap();
    assert_eq!(reopened_base.save, exact_base.save);
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
) -> bool {
    let started = Instant::now();
    let deadline = started + Duration::from_secs(30);
    let mut tick_attempts = 0_usize;
    let mut post_worker_ticks = 0_usize;
    let mut observed_checkpoint_transaction = false;
    let mut checkpoint_transaction_completed = false;
    let mut anchored_journal_rollback_proven = false;
    let mut completed_durability_hold_proven = false;
    loop {
        if tick_attempts >= 96 {
            if Instant::now() >= deadline {
                break;
            }
            if runtime.exact_checkpoint_active_tick_for_test().is_some() {
                std::thread::yield_now();
            } else if post_worker_ticks >= 24 {
                break;
            } else {
                post_worker_ticks = post_worker_ticks.saturating_add(1);
            }
        }
        tick_attempts = tick_attempts.saturating_add(1);
        if !anchored_journal_rollback_proven {
            if let Some((save_path, asset_root)) = journal_paths {
                anchored_journal_rollback_proven =
                    assert_anchored_journal_rolls_back_to_exact_base_if_present(
                        runtime, organisms, save_path, asset_root,
                    );
            }
        }
        let held_before = organisms.iter().find_map(|organism_id| {
            let sleep = runtime.sleep_state_for_test(*organism_id).ok()?;
            let permitted = runtime.durable_completed_sleep_permitted_ids_for_test();
            (matches!(sleep.consolidation, ConsolidationState::Completed { .. })
                && !permitted.contains(organism_id))
            .then(|| {
                let world_before = runtime.world_snapshot();
                let mut neutral_world = world_before.clone();
                neutral_world
                    .try_advance_tick_with_body_events(&BTreeMap::new())
                    .unwrap();
                let mut recovery_world = world_before;
                recovery_world
                    .try_advance_tick_with_body_events(&BTreeMap::from([(
                        organism_id.raw(),
                        BodyEventDelta {
                            sleep_recovery: 1.0,
                            ..BodyEventDelta::zero()
                        },
                    )]))
                    .unwrap();
                (
                    *organism_id,
                    sleep,
                    runtime.brain_atp_q16_for_test(*organism_id).unwrap(),
                    neutral_world
                        .organism_registry()
                        .get(*organism_id)
                        .unwrap()
                        .biochemistry()
                        .to_owned(),
                    recovery_world
                        .organism_registry()
                        .get(*organism_id)
                        .unwrap()
                        .biochemistry()
                        .to_owned(),
                )
            })
        });
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
        if let Some((held_organism, held_sleep, atp_before, neutral_biology, recovery_biology)) =
            held_before
        {
            let held_after = runtime.sleep_state_for_test(held_organism).unwrap();
            let still_unpermitted = !runtime
                .durable_completed_sleep_permitted_ids_for_test()
                .contains(&held_organism);
            if held_after == held_sleep && still_unpermitted {
                assert_eq!(
                    runtime.brain_atp_q16_for_test(held_organism).unwrap(),
                    atp_before,
                    "persistence latency must neither debit nor recover brain ATP"
                );
                let actual_biology = runtime
                    .world_snapshot()
                    .organism_registry()
                    .get(held_organism)
                    .unwrap()
                    .biochemistry()
                    .to_owned();
                assert_eq!(actual_biology, neutral_biology);
                assert_ne!(actual_biology, recovery_biology);
                assert!(runtime
                    .last_activity_work_receipts()
                    .iter()
                    .all(|receipt| receipt.organism_id_raw != held_organism.raw()));
                completed_durability_hold_proven = true;
            }
        }
        let active_checkpoint = runtime.exact_checkpoint_active_tick_for_test().is_some();
        observed_checkpoint_transaction |= active_checkpoint;
        if observed_checkpoint_transaction && !active_checkpoint {
            checkpoint_transaction_completed =
                runtime.exact_checkpoint_state_for_test() == ("Idle".to_string(), "Idle");
        }
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
        if authority_receipts.is_empty() || !checkpoint_transaction_completed {
            std::thread::yield_now();
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
        return completed_durability_hold_proven;
    }
    let world = runtime.world_snapshot();
    let coordinator = runtime.exact_checkpoint_state_for_test();
    let capture = runtime.exact_population_capture_metrics_for_test();
    let performance = runtime.performance_metrics();
    let active_tick = runtime.exact_checkpoint_active_tick_for_test();
    let follow_up = runtime.exact_checkpoint_follow_up_queued_for_test();
    let pending_journal = runtime.pending_exact_sleep_journal_entries_for_test().len();
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
        "canonical runtime emitted no compact GPU authority within the 96-tick plus bounded-worker window: elapsed={:?} tick_attempts={tick_attempts} post_worker_ticks={post_worker_ticks} world_tick={:?} rows={final_rows:?} inference_rows={} coordinator={coordinator:?} active_tick={active_tick:?} follow_up={follow_up} pending_journal={pending_journal} capture={capture:?} sleep_capture_calls={} sleep_persistence_calls={}",
        started.elapsed(),
        world.tick(),
        performance.inference_rows,
        performance.sleep_checkpoint_capture_calls,
        performance.sleep_persistence_calls,
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
    let durable_completed = drive_to_durable_completed(&mut runtime, &organisms);
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

    let durable = GpuDurableSaveManifest::open(&save_path, &asset_root).unwrap();
    let exact_base = durable.load().unwrap();
    let exact_anchor = exact_base.exact_save_anchor_digest().unwrap();
    assert!(durable_completed.iter().all(|organism_id| matches!(
        exact_base
            .save
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
    let committed_states = durable_completed
        .iter()
        .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
        .collect::<Vec<_>>();
    assert!(committed_states
        .iter()
        .all(|sleep| matches!(sleep.consolidation, ConsolidationState::Committed { .. })));

    let deadline = Instant::now() + Duration::from_secs(30);
    let exact_base = loop {
        runtime.poll_exact_checkpoint_for_test().unwrap();
        let loaded = GpuDurableSaveManifest::open(&save_path, &asset_root)
            .unwrap()
            .load()
            .unwrap();
        let journal = GpuDurableSaveManifest::open(&save_path, &asset_root)
            .unwrap()
            .load_sleep_transaction_journal(&loaded)
            .unwrap();
        if journal.entries.iter().any(|entry| {
            matches!(
                (entry.source.consolidation, entry.target.consolidation),
                (
                    ConsolidationState::Completed { .. },
                    ConsolidationState::Committed { .. }
                )
            )
        }) {
            break loaded;
        }
        assert!(Instant::now() < deadline, "promotion journal must publish");
        std::thread::yield_now();
    };
    assert_eq!(exact_base.exact_save_anchor_digest().unwrap(), exact_anchor);
    let durable = GpuDurableSaveManifest::open(&save_path, &asset_root).unwrap();
    let journal = durable.load_sleep_transaction_journal(&exact_base).unwrap();
    assert!(journal.entries.iter().any(|entry| matches!(
        (entry.source.consolidation, entry.target.consolidation),
        (
            ConsolidationState::Completed { .. },
            ConsolidationState::Committed { .. }
        )
    )));
    let journal_path = authority_journal_artifact_path(&save_path);
    let journal_bytes = fs::read(&journal_path).unwrap();
    let authority_bytes_before_tamper = fs::read(&save_path).unwrap();
    let mut tampered = journal.clone();
    tampered.entries[0].organism_id = OrganismId(999);
    fs::write(&journal_path, serde_json::to_vec_pretty(&tampered).unwrap()).unwrap();
    assert!(durable.load_sleep_transaction_journal(&exact_base).is_err());
    assert!(
        runtime.restored_clone_from_durability_for_test().is_err(),
        "malformed rollback journal provenance must fail closed at restore admission"
    );
    assert_eq!(
        fs::read(&save_path).unwrap(),
        authority_bytes_before_tamper,
        "rejected rollback provenance must not move the exact-save authority pointer"
    );
    fs::write(&journal_path, &journal_bytes).unwrap();

    let deadline = Instant::now() + Duration::from_secs(30);
    while runtime.exact_checkpoint_active_tick_for_test().is_some() {
        runtime.poll_exact_checkpoint_for_test().unwrap();
        assert!(
            Instant::now() < deadline,
            "checkpoint coordinator must return the durability lease after journal publication"
        );
        std::thread::yield_now();
    }

    let mut restored = runtime.restored_clone_from_durability_for_test().unwrap();
    let recommit_metrics_before = restored.performance_metrics();
    let batch_capture_before = restored.exact_population_capture_metrics_for_test();
    assert_eq!(batch_capture_before.gpu_copy_submissions, 0);
    assert_eq!(batch_capture_before.map_operations, 0);
    assert_eq!(batch_capture_before.bytes_copied, 0);
    assert_eq!(batch_capture_before.completed_captures, 0);
    assert!(durable_completed.iter().all(|organism_id| matches!(
        restored
            .sleep_state_for_test(*organism_id)
            .unwrap()
            .consolidation,
        ConsolidationState::Completed { .. }
    )));
    restored.tick().unwrap();
    let recommit_metrics_after = restored.performance_metrics();
    let batch_capture_after = restored.exact_population_capture_metrics_for_test();
    assert_eq!(
        recommit_metrics_after.sleep_checkpoint_capture_calls,
        recommit_metrics_before.sleep_checkpoint_capture_calls
    );
    assert_eq!(
        recommit_metrics_after.sleep_checkpoint_readback_calls,
        recommit_metrics_before.sleep_checkpoint_readback_calls
    );
    assert_eq!(
        recommit_metrics_after.sleep_checkpoint_readback_bytes,
        recommit_metrics_before.sleep_checkpoint_readback_bytes
    );
    assert_eq!(
        recommit_metrics_after.checkpoint_snapshot_calls,
        recommit_metrics_before.checkpoint_snapshot_calls
    );
    assert_eq!(
        recommit_metrics_after.checkpoint_snapshot_bytes,
        recommit_metrics_before.checkpoint_snapshot_bytes
    );
    assert_eq!(batch_capture_after, batch_capture_before);
    assert_eq!(
        GpuDurableSaveManifest::open(&save_path, &asset_root)
            .unwrap()
            .load()
            .unwrap()
            .exact_save_anchor_digest()
            .unwrap(),
        exact_base.exact_save_anchor_digest().unwrap(),
        "journal-only restart recommit must not replace the exact Completed save"
    );
    assert_eq!(
        durable_completed
            .iter()
            .map(|organism_id| restored.sleep_state_for_test(*organism_id).unwrap())
            .collect::<Vec<_>>(),
        committed_states,
        "restore must deterministically recommit from the exact Completed checkpoint"
    );
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
    let durable_completed = drive_to_durable_completed(&mut runtime, &organisms);
    let exact_completed = GpuDurableSaveManifest::open(&save_path, &asset_root)
        .unwrap()
        .load()
        .unwrap()
        .save;
    let expected_committed = durable_completed
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
    let durable_before_failure = GpuDurableSaveManifest::open(&save_path, &asset_root)
        .unwrap()
        .load()
        .unwrap();
    let journal_before_failure = GpuDurableSaveManifest::open(&save_path, &asset_root)
        .unwrap()
        .load_sleep_transaction_journal(&durable_before_failure)
        .unwrap();
    runtime.tick().unwrap();
    assert_eq!(
        durable_completed
            .iter()
            .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
            .collect::<Vec<_>>(),
        expected_committed,
        "runtime promotion may become visible only before the sole worker reports its CAS failure"
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    let publication_error = loop {
        match runtime.poll_exact_checkpoint_for_test() {
            Ok(()) => {
                assert!(
                    Instant::now() < deadline,
                    "journal CAS conflict must reach the runtime through the retained worker"
                );
                std::thread::yield_now();
            }
            Err(error) => break error,
        }
    };
    assert!(matches!(
        publication_error,
        GameAppShellError::GpuRuntime(
            alife_game_app::GpuRuntimeError::GpuCheckpointManifestConflict { .. }
        )
    ));
    assert_eq!(fs::read(&save_path).unwrap(), save_before_failure);
    let durable_after_failure = GpuDurableSaveManifest::open(&save_path, &asset_root)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(durable_after_failure, durable_before_failure);
    assert_eq!(
        GpuDurableSaveManifest::open(&save_path, &asset_root)
            .unwrap()
            .load_sleep_transaction_journal(&durable_after_failure)
            .unwrap(),
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
    assert!(durable_completed.iter().all(|organism_id| matches!(
        runtime
            .sleep_state_for_test(*organism_id)
            .unwrap()
            .consolidation,
        ConsolidationState::Completed { .. }
    )));
    runtime.tick().unwrap();
    assert_eq!(
        durable_completed
            .iter()
            .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
            .collect::<Vec<_>>(),
        expected_committed
    );
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

    assert!(drive_to_first_compact_authority_seal(
        &mut runtime,
        &organisms,
        Some((&continuity_save_path, &continuity_asset_root)),
    ));

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
    let _ =
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
    let async_capture_metrics = runtime.exact_population_capture_metrics_for_test();
    assert_eq!(
        async_capture_metrics.completed_captures,
        sleep_checkpoint_metrics.sleep_checkpoint_capture_calls
    );
    assert_eq!(
        async_capture_metrics.gpu_copy_submissions,
        async_capture_metrics.completed_captures
    );
    assert_eq!(
        async_capture_metrics.map_operations,
        async_capture_metrics.completed_captures
    );
    assert!(async_capture_metrics.bytes_copied > 0);
    assert_eq!(
        async_capture_metrics.released_staging_bytes,
        async_capture_metrics.bytes_copied
    );
    assert!(
        async_capture_metrics.bytes_copied
            <= async_capture_metrics
                .completed_captures
                .saturating_mul(u64::try_from(organisms.len()).unwrap())
                .saturating_mul(4 * 1024 * 1024),
        "Nano512 exact capture must remain bounded to the resident population"
    );
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
        asset_root: success_asset_root,
        save_path: success_save_path,
        mut runtime,
        organisms,
        ..
    } = canonical_runtime(31_103, 4);
    let durable_completed = drive_to_durable_completed(&mut runtime, &organisms);

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
    let failure_organism = *durable_completed
        .iter()
        .max_by_key(|organism_id| organism_id.raw())
        .unwrap();
    runtime.force_memory_preparation_failure_for_test(failure_organism);
    assert!(matches!(
        runtime.tick(),
        Err(GameAppShellError::Core(
            ScaffoldContractError::InvalidMemoryQuery
        ))
    ));
    assert_eq!(
        runtime.last_sleep_memory_compaction_preparation_count_for_test(),
        durable_completed.len() - 1,
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
    let capture_before_success = runtime.exact_population_capture_metrics_for_test();
    let exact_anchor_before_success =
        GpuDurableSaveManifest::open(&success_save_path, &success_asset_root)
            .unwrap()
            .load()
            .unwrap()
            .exact_save_anchor_digest()
            .unwrap();
    runtime.tick().unwrap();
    assert_eq!(
        runtime.performance_metrics().sleep_promotion_calls,
        metrics_before_success.sleep_promotion_calls + 1
    );
    assert_eq!(
        runtime.performance_metrics().sleep_promotion_publish_calls,
        metrics_before_success.sleep_promotion_publish_calls,
        "the retry tick must queue publication rather than write synchronously"
    );
    assert_eq!(
        runtime.last_memory_compaction_receipts().len(),
        durable_completed.len()
    );
    assert!(durable_completed.iter().all(|organism_id| matches!(
        runtime
            .sleep_state_for_test(*organism_id)
            .unwrap()
            .consolidation,
        ConsolidationState::Committed { .. }
    )));
    assert!(
        GpuDurableSaveManifest::open(&success_save_path, &success_asset_root)
            .unwrap()
            .load()
            .unwrap()
            .save
            .creatures
            .iter()
            .filter(|creature| durable_completed.contains(&creature.organism_id))
            .all(|creature| matches!(
                creature
                    .gpu_brain
                    .as_ref()
                    .map(|brain| brain.sleep.consolidation),
                Some(ConsolidationState::Completed { .. })
            ))
    );
    let publish_deadline = Instant::now() + Duration::from_secs(30);
    while runtime.exact_checkpoint_active_tick_for_test().is_some()
        && Instant::now() < publish_deadline
    {
        runtime.poll_exact_checkpoint_for_test().unwrap();
        std::thread::yield_now();
    }
    assert_eq!(
        runtime.exact_checkpoint_state_for_test(),
        ("Idle".to_string(), "Idle"),
        "the sole writer must finish the queued promotion journal"
    );
    assert_eq!(
        runtime.performance_metrics().sleep_promotion_publish_calls,
        metrics_before_success.sleep_promotion_publish_calls + 1
    );
    assert_eq!(
        runtime.exact_population_capture_metrics_for_test(),
        capture_before_success,
        "promotion publication must not start another exact GPU capture"
    );
    let exact_after_success = GpuDurableSaveManifest::open(&success_save_path, &success_asset_root)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(
        exact_after_success.exact_save_anchor_digest().unwrap(),
        exact_anchor_before_success
    );

    drop(runtime);
    fs::remove_dir_all(success_root).unwrap();

    let CanonicalRuntimeFixture {
        root: conflict_root,
        asset_root,
        save_path,
        mut runtime,
        organisms,
    } = canonical_runtime(31_102, 4);
    let _durable_completed = drive_to_durable_completed(&mut runtime, &organisms);

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
    let manifest_before = fs::read(&save_path).unwrap();
    let metrics_before = runtime.performance_metrics();
    runtime.tick().unwrap();
    assert!(organisms.iter().all(|organism_id| matches!(
        runtime
            .sleep_state_for_test(*organism_id)
            .unwrap()
            .consolidation,
        ConsolidationState::Committed { .. }
    )));
    assert_ne!(
        organisms
            .iter()
            .map(|organism_id| runtime.sleep_state_for_test(*organism_id).unwrap())
            .collect::<Vec<_>>(),
        scheduler_before,
        "a post-promotion worker failure must not roll host authority back to Completed"
    );
    assert_ne!(
        runtime
            .world_snapshot()
            .canonical_signature_digest()
            .unwrap(),
        world_digest_before,
        "the permitted promotion must remain installed when later publication fails"
    );
    let conflict_deadline = Instant::now() + Duration::from_secs(30);
    let conflict = loop {
        match runtime.poll_exact_checkpoint_for_test() {
            Ok(()) if Instant::now() < conflict_deadline => std::thread::yield_now(),
            Ok(()) => panic!("promotion journal CAS conflict did not surface before deadline"),
            Err(error) => break error,
        }
    };
    assert!(
        matches!(
            conflict,
            GameAppShellError::GpuRuntime(
                alife_game_app::GpuRuntimeError::GpuCheckpointManifestConflict { .. }
            )
        ),
        "expected promotion CAS conflict, got {conflict:?}"
    );
    assert_eq!(fs::read(&save_path).unwrap(), manifest_before);
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

    let target_tick = runtime.world_tick_for_test().raw().saturating_add(70);
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut measured_call = 0_u64;
    while runtime.world_tick_for_test().raw() < target_tick && Instant::now() < deadline {
        measured_call = measured_call.saturating_add(1);
        let world_tick = runtime.world_tick_for_test();
        let before = sleep_generation_rows(&mut runtime, &organisms);
        if let Err(error) = runtime.tick() {
            panic!(
                "six-founder production runtime failed during measured call {measured_call} at world tick {}: {error:?}\nbefore={before:#?}\nafter={:#?}",
                world_tick.raw(),
                sleep_generation_rows(&mut runtime, &organisms),
            );
        }
        if runtime.exact_checkpoint_active_tick_for_test().is_some() {
            let poll_deadline = Instant::now() + Duration::from_millis(50);
            while Instant::now() < poll_deadline {
                std::thread::yield_now();
            }
        }
    }
    assert_eq!(runtime.world_tick_for_test().raw(), target_tick);

    drop(runtime);
    fs::remove_dir_all(root).unwrap();

    let CanonicalRuntimeFixture {
        root,
        mut runtime,
        organisms,
        ..
    } = canonical_runtime(31_082_707, 4);
    let _durable_completed = drive_to_durable_completed(&mut runtime, &organisms);
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
fn phase31_six_founder_atp_safety_enters_recovery_before_dispatch_and_survives_old_boundary() {
    let CanonicalRuntimeFixture {
        root,
        asset_root,
        save_path,
        mut runtime,
        organisms,
    } = canonical_runtime(31_082_706, 6);
    let atp_preconditioned_organism = organisms[0];
    assert_eq!(
        runtime
            .sleep_state_for_test(atp_preconditioned_organism)
            .unwrap()
            .phase,
        SleepPhase::Awake
    );
    runtime
        .set_brain_atp_q16_for_test(atp_preconditioned_organism, 0)
        .unwrap();
    assert_eq!(
        runtime
            .brain_atp_q16_for_test(atp_preconditioned_organism)
            .unwrap(),
        0
    );
    let mut atp_recovery_organism = None;
    let mut atp_recovery_journal_count = None;
    let mut unaffected_dispatch_observed = false;
    let mut later_consolidation_observed = false;
    let newborn_id = OrganismId(7);
    let mut post_capture_birth = None;
    let mut first_exact_base_excluded_newborn = false;
    let mut population_follow_up_bit_observed = false;
    let mut follow_up_capture = None;
    let mut follow_up_save_proven = false;

    let started = Instant::now();
    let deadline = started + Duration::from_secs(150);
    let mut runtime_call = 0_u64;
    while Instant::now() < deadline {
        runtime_call = runtime_call.saturating_add(1);
        let world_tick = runtime.world_tick_for_test();
        let before = sleep_generation_rows(&mut runtime, &organisms);
        if let Err(error) = runtime.tick() {
            panic!(
                "six-founder ATP-safety runtime failed during call {runtime_call} at world tick {}: {error:?}\nbefore={before:#?}\nafter={:#?}",
                world_tick.raw(),
                sleep_generation_rows(&mut runtime, &organisms),
            );
        }
        assert!(runtime.authority_telemetry().authoritative);
        let active_capture_tick = runtime.exact_checkpoint_active_tick_for_test();
        if post_capture_birth.is_none() {
            if let Some(capture_tick) = active_capture_tick {
                let world = runtime.world_snapshot();
                if let Some(newborn) = world.organism_registry().get(newborn_id) {
                    if newborn.birth_tick() > capture_tick {
                        let membership = world
                            .habitat_authority()
                            .membership(newborn_id)
                            .expect("post-capture newborn must enter habitat authority atomically")
                            .clone();
                        assert_eq!(membership.entered_tick, newborn.birth_tick());
                        assert!(runtime.memory_sidecar_for_test(newborn_id).is_some());
                        assert!(runtime.topology_sidecar_for_test(newborn_id).is_some());
                        let capture_metrics = runtime.exact_population_capture_metrics_for_test();
                        post_capture_birth = Some((
                            capture_tick,
                            newborn.birth_tick(),
                            newborn.world_entity_id(),
                            newborn.genome().id,
                            newborn.genome().lineage_id,
                            membership,
                            capture_metrics.gpu_copy_submissions,
                        ));
                        population_follow_up_bit_observed =
                            runtime.exact_checkpoint_follow_up_queued_for_test();
                    }
                }
            }
        }
        if let Some((
            first_capture_tick,
            birth_tick,
            world_entity_id,
            genome_id,
            lineage_id,
            birth_membership,
            capture_submissions_at_birth,
        )) = post_capture_birth.as_ref()
        {
            population_follow_up_bit_observed |=
                runtime.exact_checkpoint_follow_up_queued_for_test();
            let durable = GpuDurableSaveManifest::open(&save_path, &asset_root).unwrap();
            let loaded = durable.load().unwrap();
            if loaded.save.world.tick == *first_capture_tick {
                assert!(loaded
                    .save
                    .world
                    .organism_records
                    .as_ref()
                    .is_some_and(|records| records
                        .iter()
                        .all(|record| record.organism_id() != newborn_id)));
                assert!(loaded
                    .save
                    .creatures
                    .iter()
                    .all(|creature| creature.organism_id != newborn_id));
                assert!(loaded.save.world.habitats.membership(newborn_id).is_none());
                first_exact_base_excluded_newborn = true;
            }
            if follow_up_capture.is_none() {
                if let Some(active_tick) =
                    active_capture_tick.filter(|tick| tick != first_capture_tick)
                {
                    assert!(active_tick >= *birth_tick);
                    assert_eq!(
                        runtime
                            .exact_population_capture_metrics_for_test()
                            .gpu_copy_submissions,
                        capture_submissions_at_birth.saturating_add(1),
                        "one post-admission signal must start exactly one follow-up capture"
                    );
                    let world = runtime.world_snapshot();
                    let record = world
                        .organism_registry()
                        .get(newborn_id)
                        .expect("follow-up capture must include the admitted newborn")
                        .clone();
                    let membership = world
                        .habitat_authority()
                        .membership(newborn_id)
                        .expect("follow-up capture must include newborn habitat authority")
                        .clone();
                    follow_up_capture = Some((active_tick, record, membership));
                }
            }
            if let Some((follow_up_tick, captured_record, captured_membership)) =
                follow_up_capture.as_ref()
            {
                if loaded.save.world.tick == *follow_up_tick {
                    let saved_record = loaded
                        .save
                        .world
                        .organism_records
                        .as_ref()
                        .and_then(|records| {
                            records
                                .iter()
                                .find(|record| record.organism_id() == newborn_id)
                        })
                        .expect("follow-up exact save must retain the newborn record");
                    assert_eq!(saved_record.organism_id(), captured_record.organism_id());
                    assert_eq!(saved_record.world_entity_id(), *world_entity_id);
                    assert_eq!(saved_record.genome().id, *genome_id);
                    assert_eq!(saved_record.genome().lineage_id, *lineage_id);
                    assert_eq!(saved_record.genome(), captured_record.genome());
                    assert_eq!(saved_record.birth_tick(), captured_record.birth_tick());
                    assert_eq!(saved_record.biochemistry().tick, *follow_up_tick);
                    assert_eq!(saved_record.biochemistry().source_genome_id, *genome_id);
                    assert_eq!(
                        loaded.save.world.habitats.membership(newborn_id),
                        Some(captured_membership)
                    );
                    assert_eq!(captured_membership, birth_membership);
                    let saved_brain = loaded
                        .save
                        .creatures
                        .iter()
                        .find(|creature| creature.organism_id == newborn_id)
                        .and_then(|creature| creature.gpu_brain.as_ref())
                        .expect("follow-up exact save must retain the newborn GPU brain");
                    assert_eq!(saved_brain.checkpoint_tick, *follow_up_tick);
                    assert_eq!(saved_brain.memory.summary.organism_id_raw, newborn_id.raw());
                    assert_eq!(saved_brain.topology.organism_id_raw, newborn_id.raw());
                    assert!(saved_brain.exact_cognitive_state.is_some());
                    assert!(saved_brain.live_structural_topology.is_some());
                    follow_up_save_proven = true;
                }
            }
        }
        let after = sleep_generation_rows(&mut runtime, &organisms);
        if atp_recovery_organism.is_none() {
            let before_row = before
                .iter()
                .find(|row| row.organism_id == atp_preconditioned_organism.raw())
                .unwrap();
            let after_row = after
                .iter()
                .find(|row| row.organism_id == atp_preconditioned_organism.raw())
                .unwrap();
            let transitioned = (before_row.phase == SleepPhase::Awake
                && after_row.phase == SleepPhase::ForcedRecoverySleep
                && after_row.last_trigger == Some(SleepTrigger::RecoveryProtocol))
            .then_some(atp_preconditioned_organism);
            if let Some(organism_id) = transitioned {
                let work = runtime.last_activity_work_receipts();
                assert!(
                    work.iter()
                        .all(|receipt| receipt.organism_id_raw != organism_id.raw()),
                    "the ATP-exhausted founder must be withheld before neural work submission"
                );
                atp_recovery_organism = Some(organism_id);
                atp_recovery_journal_count = Some(
                    runtime
                        .performance_metrics()
                        .sleep_compact_journal_organisms,
                );
            }
        } else if let Some(organism_id) = atp_recovery_organism {
            unaffected_dispatch_observed |= runtime
                .last_activity_work_receipts()
                .iter()
                .any(|receipt| receipt.organism_id_raw != organism_id.raw());
            let sleep = runtime.sleep_state_for_test(organism_id).unwrap();
            later_consolidation_observed |= sleep.phase == SleepPhase::Consolidating
                || !matches!(sleep.consolidation, ConsolidationState::None);
        }
        if runtime.exact_checkpoint_active_tick_for_test().is_some() {
            if let Err(error) = runtime.poll_exact_checkpoint_for_test() {
                panic!(
                    "ATP-safety checkpoint poll failed: {error:?}; recovery={atp_recovery_organism:?}; recovery_journal_count={atp_recovery_journal_count:?}; later_consolidation={later_consolidation_observed}; {}",
                    checkpoint_wait_diagnostics(&mut runtime, &organisms, started),
                );
            }
        }
        if atp_recovery_organism.is_some()
            && unaffected_dispatch_observed
            && later_consolidation_observed
            && runtime.world_tick_for_test().raw() > 246
            && runtime
                .performance_metrics()
                .sleep_compact_journal_organisms
                > atp_recovery_journal_count.unwrap_or(u64::MAX)
            && first_exact_base_excluded_newborn
            && population_follow_up_bit_observed
            && follow_up_save_proven
        {
            break;
        }
        std::thread::park_timeout(Duration::from_millis(1));
    }

    let atp_recovery_organism = atp_recovery_organism.unwrap_or_else(|| {
        panic!(
            "an exhausted founder must enter canonical forced-recovery sleep: {}",
            checkpoint_wait_diagnostics(&mut runtime, &organisms, started),
        )
    });
    assert!(unaffected_dispatch_observed);
    assert!(later_consolidation_observed);
    assert!(post_capture_birth.is_some());
    assert!(first_exact_base_excluded_newborn);
    assert!(population_follow_up_bit_observed);
    assert!(follow_up_save_proven);
    assert!(runtime.world_tick_for_test().raw() > 246);
    assert!(runtime.authority_telemetry().authoritative);
    assert!(
        runtime
            .performance_metrics()
            .sleep_compact_journal_organisms
            > atp_recovery_journal_count.unwrap(),
        "forced recovery must advance through the ordinary durable sleep journal"
    );
    assert!(organisms.contains(&atp_recovery_organism));
    assert_eq!(atp_recovery_organism, atp_preconditioned_organism);

    drop(runtime);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn phase31_receipt_rejects_dead_or_unaccounted_simulation() {
    use phase31_performance_health::validate_phase31_performance_authority;

    assert!(validate_phase31_performance_authority(false, true, 68, 68, 67, 67, 1).is_ok());
    assert!(validate_phase31_performance_authority(true, true, 68, 68, 67, 67, 1).is_err());
    assert!(validate_phase31_performance_authority(false, false, 68, 68, 67, 67, 1).is_err());
    assert!(validate_phase31_performance_authority(false, true, 69, 68, 67, 67, 1).is_err());
    assert!(validate_phase31_performance_authority(false, true, 68, 68, 66, 67, 1).is_err());
    assert!(validate_phase31_performance_authority(false, true, 68, 68, 67, 67, 0).is_err());
}
