//! Fresh, bounded actual software-runtime fixtures. Never ordinary hardware
//! admission or a claim that automatic pain-to-cognitive-wake is enabled.
use super::*;

fn fixture(label: &str) -> (PathBuf, GpuLiveBrainRuntime) {
    let root = std::env::temp_dir().join(format!(
        "alife-arousal-epoch-{}-{label}",
        std::process::id()
    ));
    assert!(!root.exists(), "preserve disposable fixture evidence");
    std::fs::create_dir_all(root.join("assets")).unwrap();
    let mut config =
        alife_world::RuntimeConfig::deterministic_default(31_118, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    let runtime = crate::new_game_lifecycle::create_canonical_new_game_software_runtime_test(
        crate::CanonicalNewGameLaunchRequest {
            world_seed: 31_118,
            population: 1,
            disable_age_death: false,
            save_path: root.join("live.json"),
            asset_root: root.join("assets"),
            config,
            assets: AssetManifest::empty(),
        },
    )
    .unwrap()
    .runtime;
    assert!(runtime.backend.backend().is_software_runtime_test());
    assert!(!runtime.backend.backend().is_software_training_test());
    let info = runtime
        .backend
        .backend()
        .software_runtime_test_adapter_info()
        .unwrap();
    assert_eq!(format!("{:?}", info.device_type), "Cpu");
    assert_eq!(format!("{:?}", info.backend), "Vulkan");
    println!("AROUSAL_EPOCH_EXPLICIT_OFFLINE_ADAPTER={info:?}");
    assert!(runtime.checkpoint_durability.is_some());
    (root, runtime)
}

fn finish_publication(runtime: &mut GpuLiveBrainRuntime) {
    let deadline = Instant::now() + std::time::Duration::from_secs(90);
    while !runtime.persistence_idle_for_shutdown() {
        assert!(
            Instant::now() < deadline,
            "{}",
            runtime.persistence_shutdown_diagnostics()
        );
        runtime.poll_persistence_for_shutdown().unwrap();
        assert!(!runtime.persistence_failed_for_shutdown());
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn restore_published(runtime: &mut GpuLiveBrainRuntime, root: &Path) -> GpuLiveBrainRuntime {
    // Read the real worker's publication, never publish a replacement snapshot.
    let (manifest, loaded) =
        GpuDurableSaveManifest::open_loaded(root.join("live.json"), root.join("assets")).unwrap();
    let seed = loaded.save.deterministic_seed;
    GpuLiveBrainRuntime::restore_loaded_save(
        runtime.new_staging_like_live().unwrap(),
        manifest,
        loaded,
        seed,
        BrainScaleTier::Nano512,
    )
    .unwrap()
}

fn setup_clear_interruption(runtime: &mut GpuLiveBrainRuntime) {
    let tick = runtime.world.tick();
    let scheduler = &mut runtime.residents.get_mut(&1).unwrap().sleep_scheduler;
    scheduler.force_recovery_sleep(tick).unwrap();
    scheduler.interrupt_for_acute_pain(tick).unwrap().unwrap();
    let state = scheduler.state();
    assert!(state.phase.is_interrupted());
    assert_eq!(state.consolidation, ConsolidationState::None);
    let mut record = runtime
        .world
        .organism_registry()
        .get(OrganismId(1))
        .unwrap()
        .clone();
    record
        .seal_sleep_phase(state.phase, state.active_cycle_id, tick, 0)
        .unwrap();
    runtime.world.replace_organism_record_exact(record).unwrap();
    // Fixture setup uses the real core transition and a genuine exact baseline.
    // This does not claim that world pain caused the transition automatically.
    runtime.request_exact_population_checkpoint().unwrap();
    finish_publication(runtime);
}

#[test]
fn arousal_epoch_software_real_rejected_credit_retry_and_same_tick_older_capture() {
    let (root, mut runtime) = fixture("retry-same-tick-gate-v3");
    setup_clear_interruption(&mut runtime);
    let handle = runtime.handle_for(OrganismId(1)).unwrap();
    let start_sequence = runtime.residents[&1].next_sequence;
    let start_patches = runtime.sealed_patch_count;
    let initial = capture_sleep_journal_neural_authority(&mut runtime.backend, handle).unwrap();

    // These are the existing real worker-start and real sealed-credit rejection
    // hooks. Senders drop before runtime on panic, so teardown cannot deadlock.
    let (old_release, old_gate) = mpsc::sync_channel(1);
    runtime.next_checkpoint_worker_start_gate = Some(old_gate);
    runtime.force_learning_rejections_for_test(1);
    let summaries = runtime
        .tick_with_contact_withdrawal_candidate_for_test()
        .unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(runtime.world.tick(), Tick(1));
    assert_eq!(runtime.sealed_patch_count, start_patches + 1);
    assert_eq!(runtime.residents[&1].next_sequence, start_sequence + 1);
    assert_eq!(runtime.retained_learning.len(), 1);
    assert!(runtime.last_learning_receipts.is_empty());
    assert_eq!(runtime.last_post_seal_learning_failures.len(), 1);
    assert!(runtime.arousal_neural_epochs.is_dirty(1));
    assert!(runtime.exact_checkpoint_coordinator.is_active());
    let inference = capture_sleep_journal_neural_authority(&mut runtime.backend, handle).unwrap();
    assert!(!same_neural_authority(&initial, &inference));
    let pending = runtime.backend.pending_eligibility(handle).unwrap();
    assert!(pending.is_some());

    // Submission creates a GPU capture first; only its Ready poll moves the
    // start gate into a real immutable worker. Do not overwrite that gate with
    // the follow-up gate before this ownership transfer actually happened.
    let publication_deadline = Instant::now() + std::time::Duration::from_secs(90);
    while !matches!(
        runtime.exact_checkpoint_work,
        ExactPopulationCheckpointRuntimeWorkV1::Worker { .. }
    ) {
        assert!(
            Instant::now() < publication_deadline,
            "{}",
            runtime.persistence_shutdown_diagnostics()
        );
        runtime.poll_exact_population_checkpoint().unwrap();
        assert!(!runtime.persistence_failed_for_shutdown());
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(runtime.next_checkpoint_worker_start_gate.is_none());
    println!("AROUSAL_EPOCH_OLD_WORKER_OWNS_GATE capture_tick=1");
    let (new_release, new_gate) = mpsc::sync_channel(1);
    runtime.next_checkpoint_worker_start_gate = Some(new_gate);
    // Ordinary tick: no new aroused candidate batch. The real retained-credit
    // retry acknowledges at tick1, the same tick as the older immutable capture.
    assert!(matches!(
        runtime.tick_outcome().unwrap(),
        GpuLiveTickOutcome::Progressed(_)
    ));
    assert_eq!(runtime.world.tick(), Tick(2));
    assert!(runtime.retained_learning.is_empty());
    assert_eq!(runtime.last_learning_receipts.len(), 1);
    assert_eq!(runtime.sealed_patch_count, start_patches + 1);
    assert_eq!(runtime.residents[&1].next_sequence, start_sequence + 1);
    assert!(runtime.last_gpu_authority_receipts.is_empty());
    assert!(runtime
        .backend
        .pending_eligibility(handle)
        .unwrap()
        .is_none());
    let credited = capture_sleep_journal_neural_authority(&mut runtime.backend, handle).unwrap();
    assert!(
        credited.compact.active_weight_generation() > inference.compact.active_weight_generation()
    );
    assert!(!same_neural_authority(&inference, &credited));
    assert!(runtime.arousal_neural_epochs.is_dirty(1));

    drop(old_release);
    let deadline = publication_deadline;
    // Encoding/worker phases own the durability object. A missing host Option
    // is not a publication failure; poll the real worker until it returns the
    // published authority. The follow-up capture takes ownership on a later poll.
    while runtime
        .checkpoint_durability
        .as_ref()
        .is_none_or(|durability| durability.published.save.world.tick < Tick(1))
    {
        assert!(
            Instant::now() < deadline,
            "{}",
            runtime.persistence_shutdown_diagnostics()
        );
        runtime.poll_persistence_for_shutdown().unwrap();
        assert!(!runtime.persistence_failed_for_shutdown());
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(
        runtime
            .checkpoint_durability
            .as_ref()
            .unwrap()
            .published
            .save
            .world
            .tick,
        Tick(1)
    );
    assert!(
        runtime.arousal_neural_epochs.is_dirty(1),
        "older same-tick capture cleared newer credit"
    );
    let mut old = restore_published(&mut runtime, &root);
    let old_handle = old.handle_for(OrganismId(1)).unwrap();
    let old_authority =
        capture_sleep_journal_neural_authority(&mut old.backend, old_handle).unwrap();
    assert!(
        same_neural_authority(&inference, &old_authority),
        "old publication did not restore its captured authority"
    );
    assert_eq!(old.retained_learning.len(), 1);
    drop(old);

    drop(new_release);
    finish_publication(&mut runtime);
    assert!(!runtime.arousal_neural_epochs.is_dirty(1));
    assert_eq!(
        runtime
            .checkpoint_durability
            .as_ref()
            .unwrap()
            .published
            .save
            .world
            .tick,
        Tick(2)
    );
    let expected_sleep = runtime.residents[&1].sleep_scheduler.state();
    let expected_world = runtime.world.canonical_signature_digest().unwrap();
    let mut restored = restore_published(&mut runtime, &root);
    let restored_handle = restored.handle_for(OrganismId(1)).unwrap();
    let restored_authority =
        capture_sleep_journal_neural_authority(&mut restored.backend, restored_handle).unwrap();
    assert!(
        same_neural_authority(&credited, &restored_authority),
        "new publication lost actual acknowledged authority"
    );
    assert_eq!(
        restored.world.canonical_signature_digest().unwrap(),
        expected_world
    );
    assert_eq!(
        restored.residents[&1].sleep_scheduler.state(),
        expected_sleep
    );
    assert_eq!(restored.residents[&1].next_sequence, start_sequence + 1);
    assert!(restored.retained_learning.is_empty());
    assert_eq!(
        restored.residents[&1]
            .sleep_scheduler
            .state()
            .cycles_completed,
        0
    );
    println!("AROUSAL_EPOCH_REAL_WORKER_PASS decisions=1 intervals=2 genuine_rejected_credit=1 retry_ack_no_batch=1 old_same_tick_capture_rejected=1 exact_restores=2 fabricated_publications=0");
}
