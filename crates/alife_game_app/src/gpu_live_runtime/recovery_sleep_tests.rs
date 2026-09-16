//! Recovery requests must retain the transition across durable checkpoint boundaries.
use super::*;

pub(super) fn fixture(label: &str) -> GpuLiveBrainRuntime {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/recovery-journal-{}-{label}",
        std::process::id()
    ));
    assert!(!root.exists(), "preserve regression evidence");
    std::fs::create_dir_all(root.join("assets")).unwrap();
    let mut config =
        alife_world::RuntimeConfig::deterministic_default(31_117, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    let mut runtime =
        crate::create_canonical_new_game_runtime(crate::CanonicalNewGameLaunchRequest {
            world_seed: 31_117,
            population: alife_world::PHASE3_MIN_POPULATION,
            save_path: root.join("save.json"),
            asset_root: root.join("assets"),
            config,
            assets: alife_world::AssetManifest::empty(),
        })
        .unwrap()
        .runtime;
    println!(
        "RECOVERY_GPU_ADAPTER={} BACKEND=vulkan EVIDENCE={}",
        runtime.authority_telemetry().adapter,
        root.display()
    );
    runtime.tick().unwrap();
    assert_eq!(runtime.world.tick(), Tick::new(1));
    assert!(!runtime.last_learning_receipts().is_empty());
    assert_eq!(
        runtime.residents[&1].sleep_scheduler.state().phase,
        SleepPhase::Awake
    );
    runtime
}

#[test]
fn durable_public_recovery_journals_the_awake_source_after_learning() {
    let mut runtime = fixture("public");
    let before = runtime.residents[&1].sleep_scheduler.state();
    let transition = runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    assert_eq!(transition.from, SleepPhase::Awake);
    assert_eq!(
        runtime.residents[&1].sleep_scheduler.state().phase,
        SleepPhase::ForcedRecoverySleep
    );
    runtime.tick().unwrap();
    runtime.flush_sleep_journal_publication_blocking().unwrap();
    let durability = runtime.checkpoint_durability.as_ref().unwrap();
    let journal = durability
        .durable_manifest
        .load_sleep_transaction_journal(&durability.published)
        .unwrap();
    assert_eq!(
        journal.entries.len(),
        1,
        "the public recovery edge must reach the durable journal on the next tick"
    );
    assert_eq!(journal.entries[0].source, before);
    assert_eq!(
        journal.entries[0].target.phase,
        SleepPhase::ForcedRecoverySleep
    );
    assert_eq!(journal.entries[0].transition_tick, Tick::new(2));
    assert!(runtime.pending_recovery_sleep_edges.is_empty());
    runtime.tick().unwrap();
    runtime.flush_sleep_journal_publication_blocking().unwrap();
    assert_eq!(
        read_journal(&runtime).entries.len(),
        1,
        "the recovery edge is emitted once"
    );
}

fn read_journal(runtime: &GpuLiveBrainRuntime) -> GpuSleepTransactionJournalV2 {
    let durability = runtime.checkpoint_durability.as_ref().unwrap();
    durability
        .durable_manifest
        .load_sleep_transaction_journal(&durability.published)
        .unwrap()
}

fn drain(runtime: &mut GpuLiveBrainRuntime) {
    let deadline = Instant::now() + std::time::Duration::from_secs(30);
    while !runtime.persistence_idle_for_shutdown() {
        assert!(
            Instant::now() < deadline,
            "{}",
            runtime.persistence_shutdown_diagnostics()
        );
        runtime.poll_persistence_for_shutdown().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[test]
fn recovery_before_exact_capture_is_absorbed_by_the_frozen_target() {
    let mut runtime = fixture("capture-after");
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    let target = runtime.residents[&1].sleep_scheduler.state();
    runtime.request_exact_population_checkpoint().unwrap();
    assert!(runtime.pending_recovery_sleep_edges.is_empty());
    drain(&mut runtime);
    assert_eq!(
        runtime
            .checkpoint_durability
            .as_ref()
            .unwrap()
            .published
            .save
            .creatures[0]
            .gpu_brain
            .as_ref()
            .unwrap()
            .sleep,
        target
    );
    runtime.tick().unwrap();
    runtime.flush_sleep_journal_publication_blocking().unwrap();
    assert!(
        read_journal(&runtime).entries.is_empty(),
        "the exact base already owns recovery"
    );
}

#[test]
fn recovery_after_exact_capture_preserves_the_frozen_awake_source() {
    let mut runtime = fixture("capture-before");
    runtime.request_exact_population_checkpoint().unwrap();
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    assert_eq!(
        runtime.pending_recovery_sleep_edges[&1].source.phase,
        SleepPhase::Awake
    );
    assert!(matches!(
        runtime.exact_checkpoint_work,
        ExactPopulationCheckpointRuntimeWorkV1::Capture { .. }
    ));
    let edge = runtime.pending_recovery_sleep_edges[&1];
    let entry = GpuSleepTransactionJournalEntryV2::try_new(
        OrganismId(1),
        Tick::new(2),
        edge.source,
        edge.target,
    )
    .unwrap();
    assert!(runtime
        .queue_exact_checkpoint_journal_entries(vec![entry.clone(), entry])
        .is_err());
    assert!(runtime.pending_exact_sleep_journal_entries.is_empty());
    assert!(runtime.pending_recovery_sleep_edges.contains_key(&1));
    // Each tick polls only one work-state arm, so this capture cannot finish
    // before both ticks. No worker timing assumption or artificial pause is needed.
    for _ in 0..2 {
        assert!(runtime.exact_checkpoint_coordinator.is_active());
        assert!(matches!(
            runtime.tick_outcome().unwrap(),
            GpuLiveTickOutcome::Progressed(_)
        ));
        assert!(runtime.exact_checkpoint_coordinator.is_active());
        assert_eq!(runtime.pending_exact_sleep_journal_entries.len(), 1);
        assert_eq!(
            runtime.pending_exact_sleep_journal_entries[0].source,
            edge.source
        );
        assert_eq!(
            runtime.pending_exact_sleep_journal_entries[0].target,
            edge.target
        );
        assert!(runtime.pending_recovery_sleep_edges.is_empty());
    }
    drain(&mut runtime);
    assert!(runtime.pending_exact_sleep_journal_entries.is_empty());
    assert!(runtime.pending_recovery_sleep_edges.is_empty());
    let durability = runtime.checkpoint_durability.as_ref().unwrap();
    assert_eq!(durability.published.save.world.tick, runtime.world.tick());
    assert_eq!(
        durability.published.save.creatures[0]
            .gpu_brain
            .as_ref()
            .unwrap()
            .sleep,
        edge.target
    );
    assert!(
        read_journal(&runtime).entries.is_empty(),
        "the bounded follow-up exact capture owns the queued edge"
    );
}

#[test]
fn portable_export_does_not_absorb_live_recovery_and_duplicate_request_is_inert() {
    let mut runtime = fixture("export");
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    let before = runtime.residents[&1].sleep_scheduler.state();
    let exported = runtime.capture_portable_checkpoint().unwrap();
    assert_eq!(
        exported.creatures[0].gpu_brain.as_ref().unwrap().sleep,
        before
    );
    assert!(runtime.pending_recovery_sleep_edges.contains_key(&1));
    assert!(matches!(
        runtime.request_recovery_sleep(OrganismId(1)),
        Err(GameAppShellError::InvalidProductionFrontend { .. })
    ));
    assert_eq!(runtime.residents[&1].sleep_scheduler.state(), before);
    assert_eq!(
        runtime.pending_recovery_sleep_edges[&1].source.phase,
        SleepPhase::Awake
    );
    runtime.tick().unwrap();
    runtime.flush_sleep_journal_publication_blocking().unwrap();
    assert_eq!(read_journal(&runtime).entries.len(), 1);
}

#[test]
fn failed_tick_and_failed_capture_keep_the_pending_recovery_source() {
    let mut runtime = fixture("failure");
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    let before = runtime.residents[&1].sleep_scheduler.state();
    let tick = runtime.world.tick();
    let memory = runtime.memories.remove(&1).unwrap();
    let result = runtime.tick_outcome();
    runtime.memories.insert(1, memory);
    assert!(result.is_err());
    assert_eq!(runtime.world.tick(), tick);
    assert_eq!(runtime.residents[&1].sleep_scheduler.state(), before);
    assert_eq!(
        runtime.pending_recovery_sleep_edges[&1].source.phase,
        SleepPhase::Awake
    );
    // Fail the existing host-freeze guard before GPU capture submission.
    let age = &mut runtime.residents.get_mut(&1).unwrap().development.age_ticks;
    *age = Tick::new(age.raw() + 1);
    assert!(runtime.request_exact_population_checkpoint().is_err());
    assert_eq!(
        runtime.pending_recovery_sleep_edges[&1].source.phase,
        SleepPhase::Awake
    );
}

#[test]
fn retained_retry_exhaustion_uses_the_same_edge_and_does_not_restart_sleep() {
    let mut runtime = fixture("retained");
    runtime.force_learning_rejections_for_test(1);
    runtime.tick().unwrap();
    assert!(runtime.retained_learning.contains_key(&1));
    let tick = runtime.world.tick();
    for _ in 0..MAX_RETAINED_LEARNING_RETRIES {
        runtime
            .record_retained_retry_failure(
                OrganismId(1),
                tick,
                RetainedLearningErrorCode::LearningEvidenceMismatch,
            )
            .unwrap();
    }
    let before = runtime.residents[&1].sleep_scheduler.state();
    assert_eq!(before.phase, SleepPhase::ForcedRecoverySleep);
    runtime
        .record_retained_retry_failure(
            OrganismId(1),
            Tick::new(tick.raw() + 1),
            RetainedLearningErrorCode::LearningEvidenceMismatch,
        )
        .unwrap();
    assert_eq!(runtime.residents[&1].sleep_scheduler.state(), before);
    runtime.tick().unwrap();
    runtime.flush_sleep_journal_publication_blocking().unwrap();
    let entries = read_journal(&runtime).entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source.phase, SleepPhase::Awake);
    assert_eq!(entries[0].target, before);
}

#[test]
fn durable_recovery_completes_one_sleep_cycle_after_learning() {
    let mut runtime = fixture("cycle");
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    let deadline = Instant::now() + std::time::Duration::from_secs(90);
    let mut progressed = 0;
    let mut saw_submitted = false;
    let mut saw_completed = false;
    let mut compactions = 0;
    while progressed < 96 && Instant::now() < deadline {
        if matches!(
            runtime.tick_outcome().unwrap(),
            GpuLiveTickOutcome::Progressed(_)
        ) {
            progressed += 1;
            compactions += runtime
                .last_memory_compaction_receipts()
                .iter()
                .filter(|receipt| receipt.identity.organism_id_raw == 1)
                .count();
        }
        let state = runtime.residents[&1].sleep_scheduler.state();
        saw_submitted |= matches!(state.consolidation, ConsolidationState::Submitted { .. });
        saw_completed |= matches!(state.consolidation, ConsolidationState::Completed { .. });
        if state.phase == SleepPhase::Awake && state.last_consolidated_cycle_id == 1 {
            assert_eq!(state.cycles_completed, 1);
            assert!(saw_submitted && saw_completed);
            assert_eq!(compactions, 1);
            drain(&mut runtime);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("recovery did not complete one cycle after {progressed} progressed ticks");
}
