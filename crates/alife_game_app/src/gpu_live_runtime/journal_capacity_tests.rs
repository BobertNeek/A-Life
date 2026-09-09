//! Bounded journals must pause before a cognitive tick, then resume after drain.
use super::*;
use std::time::Duration;

fn two_edge_population(population: usize) -> Vec<GpuSleepTransactionJournalEntryV2> {
    let mut entries = Vec::with_capacity(population * 2);
    for raw in 1..=population as u64 {
        let source = SleepState {
            schema_version: alife_core::SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            phase: SleepPhase::ForcedRecoverySleep,
            phase_started_tick: Tick::new(1),
            entered_sleep_tick: Some(Tick::new(1)),
            cycles_completed: 0,
            last_trigger: Some(alife_core::SleepTrigger::RecoveryProtocol),
            active_cycle_id: 1,
            last_consolidated_cycle_id: 0,
            consolidation: ConsolidationState::None,
        };
        let intermediate = SleepState {
            phase: SleepPhase::Consolidating,
            phase_started_tick: Tick::new(2),
            ..source
        };
        let target = SleepState {
            consolidation: ConsolidationState::Pending {
                intent: ConsolidationIntent { cycle_id: 1 },
                replay_digest: [9; 4],
                replay_event_count: 1,
                replay_eligibility_sample_count: 0,
            },
            ..intermediate
        };
        entries.push(
            GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
                OrganismId(raw),
                Tick::new(2),
                0,
                source,
                intermediate,
            )
            .unwrap(),
        );
        entries.push(
            GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
                OrganismId(raw),
                Tick::new(2),
                1,
                intermediate,
                target,
            )
            .unwrap(),
        );
    }
    entries
}

#[test]
fn pending_journal_capacity_accepts_65_valid_entries_for_33_organisms() {
    let capacity = sleep_journal_pending_capacity_for_population(33).unwrap();
    let entries = two_edge_population(33).into_iter().take(65).collect();
    let mut pending = Vec::new();
    append_bounded_sleep_journal_entries(&mut pending, entries, capacity).unwrap();
    assert_eq!(pending.len(), 65);
}

#[test]
fn production_population_has_room_for_a_whole_1000_entry_tick() {
    let population = alife_gpu_backend::GpuRuntimeProfile::production_v1().max_hot_brains as usize;
    assert_eq!(population, 500);
    let capacity = sleep_journal_pending_capacity_for_population(population).unwrap();
    assert!(capacity >= 2 * population);
    assert!(capacity <= 4096);
    let mut pending = Vec::new();
    append_bounded_sleep_journal_entries(&mut pending, two_edge_population(population), capacity)
        .unwrap();
    assert_eq!(pending.len(), 1000);
    GpuSleepTransactionJournalV2::try_new(
        "fnv1a64:0123456789abcdef".to_string(),
        Tick::new(0),
        pending,
    )
    .unwrap();
}

#[test]
fn durable_journal_accepts_4096_entries_and_rejects_one_more() {
    let entries = two_edge_population(2048);
    let mut oversized = entries.clone();
    oversized.push(
        GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(2049),
            entries[0].transition_tick,
            entries[0].source,
            entries[0].target,
        )
        .unwrap(),
    );
    GpuSleepTransactionJournalV2::try_new(
        "fnv1a64:0123456789abcdef".to_string(),
        Tick::new(0),
        entries,
    )
    .unwrap();
    assert!(GpuSleepTransactionJournalV2::try_new(
        "fnv1a64:0123456789abcdef".to_string(),
        Tick::new(0),
        oversized,
    )
    .is_err());
}

#[test]
fn unsupported_single_tick_capacity_is_rejected() {
    assert_eq!(
        sleep_journal_pending_capacity_for_population(1024).unwrap(),
        4096
    );
    assert_eq!(sleep_journal_tick_reserve(1024).unwrap(), 2048);
    assert!(sleep_journal_pending_capacity_for_population(1025).is_err());
    assert!(sleep_journal_pending_capacity_for_population(2049).is_err());
    assert!(sleep_journal_pending_capacity_for_population(usize::MAX).is_err());
}

#[test]
fn admission_checks_actual_profile_and_both_reserves_without_sizing_from_profile_maximum() {
    let profile = alife_gpu_backend::GpuRuntimeProfile::production_v1();
    assert_eq!(
        validate_sleep_journal_population(500, profile.max_hot_brains),
        Ok(1000)
    );
    assert_eq!(
        validate_sleep_journal_population(501, profile.max_hot_brains),
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    );
    assert_eq!(validate_sleep_journal_population(1024, 2048), Ok(2048));
    assert!(validate_sleep_journal_population(1025, 2048).is_err());
    assert_eq!(validate_sleep_journal_population(4, u32::MAX), Ok(8));
    assert_eq!(SleepJournalCapacity::default().prospective(4).unwrap(), 64);
}

#[test]
fn population_shrink_preserves_capacity_for_already_owned_entries() {
    let mut capacity = SleepJournalCapacity::default();
    // A proposed admission alone must not change the resource policy.
    assert!(capacity.prospective(500).unwrap() >= 1000);
    capacity.admit_population(1).unwrap();
    assert_eq!(capacity.prospective(1).unwrap(), 64);

    capacity.admit_population(33).unwrap();
    let mut pending = two_edge_population(33)
        .into_iter()
        .take(65)
        .collect::<Vec<_>>();
    let admitted_capacity = capacity.prospective(33).unwrap();
    capacity.admit_population(1).unwrap();
    let after_shrink = capacity.prospective(1).unwrap();
    assert_eq!(after_shrink, admitted_capacity);
    append_bounded_sleep_journal_entries(
        &mut pending,
        two_edge_population(34).into_iter().skip(65).collect(),
        after_shrink,
    )
    .unwrap();
    assert_eq!(pending.len(), 68);
    assert_eq!(sleep_journal_tick_reserve(1).unwrap(), 2);
}

#[test]
fn durable_budget_counts_worker_increment_once_and_reserves_the_next_tick() {
    assert!(!ordinary_sleep_journal_requires_rollover(2000, 10, 0, 2, 4096).unwrap());
    assert!(!ordinary_sleep_journal_requires_rollover(4080, 4, 4, 8, 4096).unwrap());
    assert!(ordinary_sleep_journal_requires_rollover(4081, 4, 4, 8, 4096).unwrap());
    assert!(ordinary_sleep_journal_requires_rollover(usize::MAX, 1, 0, 0, 4096).is_err());
}

#[test]
fn exact_finalization_reserve_survives_admitted_population_growth() {
    for (before_population, after_population) in [(8, 9), (16, 17), (1023, 1024)] {
        let mut capacity = SleepJournalCapacity::default();
        capacity.admit_population(before_population).unwrap();
        let before_capacity = capacity.prospective(before_population).unwrap();
        let before_reserve = sleep_journal_tick_reserve(before_population).unwrap();
        let pending = before_capacity / 2 - before_reserve;
        assert!(
            exact_sleep_journal_tick_fits(pending, before_reserve, before_capacity, false).unwrap()
        );
        let after_tick = pending + before_reserve;
        assert!(
            !exact_sleep_journal_tick_fits(after_tick, before_reserve, before_capacity, false)
                .unwrap()
        );

        capacity.admit_population(after_population).unwrap();
        let after_capacity = capacity.prospective(after_population).unwrap();
        let after_reserve = sleep_journal_tick_reserve(after_population).unwrap();
        assert!(after_capacity >= before_capacity);
        assert!(
            exact_sleep_journal_tick_fits(after_tick, after_reserve, after_capacity, true).unwrap()
        );
    }
    assert!(exact_sleep_journal_tick_fits(usize::MAX, 2, 64, false).is_err());
}

#[test]
fn exact_finalization_reserve_survives_shrink_and_regrowth_at_historical_capacity() {
    let mut capacity = SleepJournalCapacity::default();
    capacity.admit_population(1024).unwrap();
    capacity.admit_population(1).unwrap();
    let limit = capacity.prospective(1).unwrap();
    assert_eq!(limit, 4096);
    let small_reserve = sleep_journal_tick_reserve(1).unwrap();
    let pending = limit / 2 - small_reserve;
    assert!(exact_sleep_journal_tick_fits(pending, small_reserve, limit, false).unwrap());
    let after_tick = pending + small_reserve;
    assert!(!exact_sleep_journal_tick_fits(after_tick, small_reserve, limit, false).unwrap());

    capacity.admit_population(1024).unwrap();
    assert!(exact_sleep_journal_tick_fits(
        after_tick,
        sleep_journal_tick_reserve(1024).unwrap(),
        capacity.prospective(1024).unwrap(),
        true
    )
    .unwrap());
}

#[test]
fn pending_journal_rejects_duplicate_and_disconnected_edges_atomically() {
    let entries = two_edge_population(1);
    let mut pending = vec![entries[0].clone()];
    let before = pending.clone();
    assert!(
        append_bounded_sleep_journal_entries(&mut pending, vec![entries[0].clone()], 64).is_err()
    );
    assert_eq!(pending, before);

    let source = SleepState::awake_at(Tick::new(3));
    let target = SleepState {
        phase: SleepPhase::ForcedRecoverySleep,
        phase_started_tick: Tick::new(4),
        entered_sleep_tick: Some(Tick::new(4)),
        last_trigger: Some(alife_core::SleepTrigger::RecoveryProtocol),
        active_cycle_id: 1,
        ..source
    };
    let disconnected =
        GpuSleepTransactionJournalEntryV2::try_new(OrganismId(1), Tick::new(4), source, target)
            .unwrap();
    assert!(append_bounded_sleep_journal_entries(&mut pending, vec![disconnected], 64).is_err());
    assert_eq!(pending, before);
}

struct WorkerHold(Option<SyncSender<()>>);

impl WorkerHold {
    fn release(&mut self) {
        self.0.take();
    }
}

impl Drop for WorkerHold {
    fn drop(&mut self) {
        self.release();
    }
}

fn hold_next_worker(runtime: &mut GpuLiveBrainRuntime) -> WorkerHold {
    assert!(runtime.next_checkpoint_worker_start_gate.is_none());
    let (sender, receiver) = mpsc::sync_channel(1);
    runtime.next_checkpoint_worker_start_gate = Some(receiver);
    WorkerHold(Some(sender))
}

#[derive(Debug, PartialEq)]
struct BudgetSnapshot {
    host: sleep_atomicity_tests::HostSleepSnapshot,
    gpu_activity: Vec<(u64, String)>,
    readbacks: String,
    captures: String,
    available: bool,
}

impl BudgetSnapshot {
    fn capture(runtime: &GpuLiveBrainRuntime) -> Self {
        Self {
            host: sleep_atomicity_tests::HostSleepSnapshot::capture(runtime),
            gpu_activity: runtime
                .handles
                .keys()
                .map(|raw| {
                    (
                        *raw,
                        format!(
                            "{:?}",
                            runtime
                                .evidence_activity_snapshot(OrganismId(*raw))
                                .unwrap()
                        ),
                    )
                })
                .collect(),
            readbacks: format!("{:?}", runtime.backend.mutable_slot_readback_metrics()),
            captures: format!(
                "{:?}",
                runtime.backend.backend().exact_population_capture_metrics()
            ),
            available: runtime.backend.ensure_neural_actions_available().is_ok(),
        }
    }
}

fn drain(runtime: &mut GpuLiveBrainRuntime) -> Result<(), GameAppShellError> {
    let deadline = Instant::now() + Duration::from_secs(90);
    while !runtime.persistence_idle_for_shutdown() {
        if Instant::now() >= deadline {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "journal test drain timed out: {}",
                    runtime.persistence_shutdown_diagnostics()
                ),
            });
        }
        runtime.poll_persistence_for_shutdown()?;
        thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn assert_pause_then_drain_and_retry(
    runtime: &mut GpuLiveBrainRuntime,
    hold: &mut WorkerHold,
    kind: &str,
) {
    let before = BudgetSnapshot::capture(runtime);
    let tick_before = runtime.world.tick();
    let outcome = runtime.tick_outcome();
    let after = BudgetSnapshot::capture(runtime);
    println!("SCALED_JOURNAL_CAPACITY kind={kind} population={} capacity=16 reserve=8 tick_before={} tick_after={}",
        runtime.handles.len(), tick_before.raw(), runtime.world.tick().raw());
    // Release before assertions and perform the real publication cleanup even on RED.
    hold.release();
    let drained = drain(runtime);
    assert!(
        matches!(
            outcome,
            Ok(GpuLiveTickOutcome::NoProgress(
                GpuLiveNoProgressReason::CheckpointPublicationPending
            ))
        ),
        "capacity pressure must pause before cognitive mutation"
    );
    assert_eq!(after, before);
    drained.unwrap();
    runtime.backend.ensure_neural_actions_available().unwrap();
    assert!(matches!(
        runtime.tick_outcome().unwrap(),
        GpuLiveTickOutcome::Progressed(_)
    ));
    assert!(runtime.world.tick() > tick_before);
}

#[test]
fn scaled_ordinary_journal_backpressure_preserves_state_then_real_drain_allows_progress() {
    let mut runtime = recovery_sleep_tests::fixture("capacity-ordinary");
    runtime.sleep_journal_capacity_override = Some(16);
    // Declared after runtime so panic unwinding releases the worker before runtime Drop.
    let mut hold = hold_next_worker(&mut runtime);
    let ids = runtime.handles.keys().copied().collect::<Vec<_>>();
    assert_eq!(ids.len(), 4);
    for raw in ids {
        runtime.request_recovery_sleep(OrganismId(raw)).unwrap();
    }
    for _ in 0..64 {
        if runtime.pending_sleep_journal_entries.len() > 8 {
            break;
        }
        assert!(matches!(
            runtime.tick_outcome().unwrap(),
            GpuLiveTickOutcome::Progressed(_)
        ));
    }
    assert!(runtime.sleep_journal_publication_worker.is_some());
    assert!(runtime.pending_sleep_journal_entries.len() > 8);
    assert!(runtime.pending_sleep_journal_entries.len() <= 16);
    assert!(!runtime.exact_checkpoint_waiting_for_sleep_journal);
    assert_pause_then_drain_and_retry(&mut runtime, &mut hold, "ordinary");
}

#[test]
fn scaled_exact_journal_backpressure_preserves_state_then_real_drain_allows_progress() {
    let mut runtime = recovery_sleep_tests::fixture("capacity-exact");
    runtime.sleep_journal_capacity_override = Some(16);
    let mut hold = hold_next_worker(&mut runtime);
    runtime.request_exact_population_checkpoint().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while !matches!(
        runtime.exact_checkpoint_work,
        ExactPopulationCheckpointRuntimeWorkV1::Worker { .. }
    ) {
        assert!(Instant::now() < deadline, "exact worker was not admitted");
        runtime.poll_exact_population_checkpoint().unwrap();
        thread::sleep(Duration::from_millis(1));
    }
    assert!(runtime.next_checkpoint_worker_start_gate.is_none());
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    assert!(matches!(
        runtime.tick_outcome().unwrap(),
        GpuLiveTickOutcome::Progressed(_)
    ));
    assert_eq!(runtime.pending_exact_sleep_journal_entries.len(), 1);
    // Four founders reserve eight entries. Eight more remain for finalization,
    // so the existing one entry prevents another ordinary overlapping tick.
    assert_pause_then_drain_and_retry(&mut runtime, &mut hold, "exact");
}

#[test]
fn awaiting_journal_can_spend_finalization_reserve_and_complete() {
    let mut runtime = sleep_atomicity_tests::at_sleep_boundary("capacity-finalize", true);
    assert!(matches!(
        runtime.exact_checkpoint_work,
        ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { .. }
    ));
    assert!(runtime.pending_exact_sleep_journal_entries.is_empty());
    runtime.sleep_journal_capacity_override = Some(8);
    for raw in runtime
        .handles
        .keys()
        .copied()
        .filter(|raw| *raw != 1)
        .collect::<Vec<_>>()
    {
        runtime.request_recovery_sleep(OrganismId(raw)).unwrap();
    }
    let tick_before = runtime.world.tick();
    // At capacity eight there is no ordinary overlap allowance, but the
    // captured Completed state must still be able to finalize in one tick.
    assert!(matches!(
        runtime.tick_outcome().unwrap(),
        GpuLiveTickOutcome::Progressed(_)
    ));
    assert!(matches!(
        runtime.residents[&1].sleep_scheduler.state().consolidation,
        ConsolidationState::Committed { .. }
    ));
    drain(&mut runtime).unwrap();
    assert!(runtime.world.tick() > tick_before);
    runtime.backend.ensure_neural_actions_available().unwrap();
    println!("SCALED_JOURNAL_FINALIZATION capacity=8 reserve=8 completed_and_drained=true");
}

fn assert_durable_journal_count(runtime: &GpuLiveBrainRuntime, expected: usize) {
    let durability = runtime.checkpoint_durability.as_ref().unwrap();
    let journal = durability
        .durable_manifest
        .load_sleep_transaction_journal(&durability.published)
        .unwrap();
    assert_eq!(journal.entries.len(), expected);
    assert_eq!(durability.sleep_journal_entry_count, journal.entries.len());
}

#[test]
fn preflight_rollover_preserves_cognition_and_refreshes_cached_count() {
    let mut runtime = recovery_sleep_tests::fixture("capacity-durable-cache");
    assert_durable_journal_count(&runtime, 0);
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    runtime.tick_outcome().unwrap();
    runtime.flush_sleep_journal_publication_blocking().unwrap();
    assert_durable_journal_count(&runtime, 1);

    let checkpoint_tick = runtime.world.tick();
    assert!(!runtime.exact_checkpoint_coordinator.is_active());
    // Scale only the admission budget. The actual journal and cached count
    // come from the real publication above; the storage limit remains 4096.
    runtime.sleep_journal_durable_capacity_override = Some(8);
    let before = BudgetSnapshot::capture(&runtime);
    assert!(matches!(
        runtime.tick_outcome().unwrap(),
        GpuLiveTickOutcome::NoProgress(GpuLiveNoProgressReason::CheckpointPublicationPending)
    ));
    let after = BudgetSnapshot::capture(&runtime);
    before.host.assert_cognition_matches(&after.host);
    assert_eq!(before.gpu_activity, after.gpu_activity);
    assert_eq!(before.available, after.available);
    assert_eq!(runtime.world.tick(), checkpoint_tick);
    assert!(matches!(
        runtime.exact_checkpoint_work,
        ExactPopulationCheckpointRuntimeWorkV1::Capture { .. }
    ));
    assert_eq!(
        runtime
            .exact_checkpoint_coordinator
            .active_identity()
            .unwrap()
            .checkpoint_tick,
        checkpoint_tick
    );
    drain(&mut runtime).unwrap();
    assert_eq!(runtime.world.tick(), checkpoint_tick);
    assert_durable_journal_count(&runtime, 0);

    runtime.request_recovery_sleep(OrganismId(2)).unwrap();
    runtime.tick_outcome().unwrap();
    runtime.flush_sleep_journal_publication_blocking().unwrap();
    assert!(runtime.world.tick() > checkpoint_tick);
    assert_durable_journal_count(&runtime, 1);
    runtime.backend.ensure_neural_actions_available().unwrap();
    println!("JOURNAL_CAPACITY_ROLLOVER durable_limit=8 real_publication=1 preflight_rollover=0 new_publication=1 checkpoint_tick={} retry_tick={}", checkpoint_tick.raw(), runtime.world.tick().raw());
}
