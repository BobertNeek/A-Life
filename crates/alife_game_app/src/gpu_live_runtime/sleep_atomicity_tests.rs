//! Failures inside one population tick cannot publish partial sleep authority.
use super::*;

#[derive(Debug, PartialEq)]
struct HostSleepSnapshot {
    world: String,
    residents: String,
    memories: BTreeMap<u64, MemorySidecarState>,
    topologies: BTreeMap<u64, TopologySidecar>,
    restored_replay: Vec<ExperiencePatch>,
    sealed: Vec<ExperiencePatch>,
    last_sealed: Vec<ExperiencePatch>,
    sealed_count: usize,
    journal_authorities: BTreeMap<u64, SleepJournalNeuralAuthority>,
    exact_queue: Vec<GpuSleepTransactionJournalEntryV2>,
    pending_queue: Vec<GpuSleepTransactionJournalEntryV2>,
    recovery_edges: Vec<(u64, SleepState, SleepState)>,
}

impl HostSleepSnapshot {
    fn capture(runtime: &GpuLiveBrainRuntime) -> Self {
        Self {
            world: format!("{:?}", runtime.world),
            residents: format!("{:?}", runtime.residents),
            memories: runtime.memories.clone(),
            topologies: runtime.topologies.clone(),
            restored_replay: runtime.restored_replay_patches.clone(),
            sealed: runtime.sealed_patches.clone(),
            last_sealed: runtime.last_sealed_patches.clone(),
            sealed_count: runtime.sealed_patch_count,
            journal_authorities: runtime.sleep_journal_neural_authorities.clone(),
            exact_queue: runtime.pending_exact_sleep_journal_entries.clone(),
            pending_queue: runtime.pending_sleep_journal_entries.clone(),
            recovery_edges: runtime
                .pending_recovery_sleep_edges
                .iter()
                .map(|(raw, edge)| (*raw, edge.source, edge.target))
                .collect(),
        }
    }
}

fn assert_stopped_operations_are_refused(runtime: &mut GpuLiveBrainRuntime) {
    let before = HostSleepSnapshot::capture(runtime);
    assert!(runtime.backend.ensure_neural_actions_available().is_err());
    assert!(runtime.tick_outcome().is_err());
    assert!(matches!(
        runtime.capture_portable_checkpoint(),
        Err(GameAppShellError::Core(
            ScaffoldContractError::NeuralBackendUnavailable
        ))
    ));
    let destination = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../target/founder-training-evidence/refused-sleep-save-{}.json",
        std::process::id()
    ));
    assert!(runtime
        .request_manual_checkpoint(destination.clone())
        .is_err());
    assert!(!destination.exists());
    assert_eq!(HostSleepSnapshot::capture(runtime), before);
}

fn production_progress(
    backend: &mut GpuClosedLoopBackend,
    handle: GpuBrainHandle,
    organism_id: OrganismId,
    state: SleepState,
    intent: Option<ConsolidationIntent>,
) -> SleepProgressResult {
    AuthoritativeGpuSleepDriver {
        backend,
        handle,
        sleep_config: None,
        context: None,
        replay_evidence_before_commit: None,
        last_sleep_work: None,
        fail_stop_armed: None,
    }
    .progress(organism_id, state, intent)
}

// One large staged-tick instantiation for all fault closures keeps this native
// test module inexpensive to rebuild without changing production dispatch.
fn tick_with_fault(
    runtime: &mut GpuLiveBrainRuntime,
    progress: &mut dyn FnMut(
        &mut GpuClosedLoopBackend,
        GpuBrainHandle,
        OrganismId,
        SleepState,
        Option<ConsolidationIntent>,
    ) -> SleepProgressResult,
) -> Result<GpuLiveTickOutcome, GameAppShellError> {
    runtime.tick_with_sleep_progress_outcome(progress)
}

fn poll_persistence(runtime: &mut GpuLiveBrainRuntime) {
    runtime.poll_sleep_journal_publication().unwrap();
    runtime.poll_exact_population_checkpoint().unwrap();
}

fn at_sleep_boundary(label: &str, completed: bool) -> GpuLiveBrainRuntime {
    let mut runtime = recovery_sleep_tests::fixture(label);
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    let deadline = Instant::now() + std::time::Duration::from_secs(120);
    loop {
        assert!(
            Instant::now() < deadline,
            "sleep boundary: {}",
            runtime.persistence_shutdown_diagnostics()
        );
        let state = runtime.residents[&1].sleep_scheduler.state();
        if !completed && matches!(state.consolidation, ConsolidationState::Prepared { .. }) {
            runtime.flush_sleep_journal_publication_blocking().unwrap();
            return runtime;
        }
        if completed && matches!(state.consolidation, ConsolidationState::Completed { .. }) {
            // Poll the normal coordinator without spending world ticks while
            // the exact captured Completed state becomes durable.
            poll_persistence(&mut runtime);
            if let ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, .. } =
                &runtime.exact_checkpoint_work
            {
                if permit.published().save.creatures.iter().any(|creature| {
                    creature.organism_id == OrganismId(1)
                        && creature
                            .gpu_brain
                            .as_ref()
                            .is_some_and(|brain| brain.sleep == state)
                }) {
                    return runtime;
                }
            }
        } else {
            runtime.tick_outcome().unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn sleep_sidecar_commit_rolls_back_when_replay_preparation_fails() {
    let mut runtime = recovery_sleep_tests::fixture("atomic-sidecars");
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    for _ in 0..64 {
        runtime.flush_sleep_journal_publication_blocking().unwrap();
        let before = HostSleepSnapshot::capture(&runtime);
        let mut injected = false;
        let result = tick_with_fault(&mut runtime, &mut |backend, handle, id, state, intent| {
            if id == OrganismId(1) && intent.is_some() {
                // scheduled_tick_with_organism runs the native bounded memory,
                // predictor and topology transaction before requesting replay.
                injected = true;
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
            production_progress(backend, handle, id, state, intent)
        });
        if injected {
            assert!(result.is_err());
            assert_eq!(HostSleepSnapshot::capture(&runtime), before);
            assert_stopped_operations_are_refused(&mut runtime);
            return;
        }
        result.unwrap();
    }
    panic!("did not reach the first bounded sleep transaction");
}

#[test]
fn sleep_submit_error_after_device_mutation_fail_stops() {
    assert_device_failure_stops(false);
}

#[test]
fn sleep_commit_error_after_device_mutation_fail_stops() {
    assert_device_failure_stops(true);
}

fn assert_device_failure_stops(completed: bool) {
    let mut runtime = at_sleep_boundary(
        if completed {
            "atomic-commit"
        } else {
            "atomic-submit"
        },
        completed,
    );
    let before = HostSleepSnapshot::capture(&runtime);
    let mut transitioned = false;
    let result = tick_with_fault(&mut runtime, &mut |backend, handle, id, state, intent| {
        let event = production_progress(backend, handle, id, state, intent)?;
        if id == OrganismId(1) {
            assert!(matches!(
                event,
                Some(
                    ConsolidationDriverEvent::Submitted { .. }
                        | ConsolidationDriverEvent::Committed { .. }
                )
            ));
            transitioned = true;
            // Model a fallible receipt step after the actual device operation.
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(event)
    });
    assert!(
        transitioned,
        "native device operation must precede the injected error"
    );
    assert!(result.is_err());
    assert_eq!(HostSleepSnapshot::capture(&runtime), before);
    assert!(
        runtime.backend.ensure_neural_actions_available().is_err(),
        "a partially committed device transaction cannot resume"
    );
    assert_stopped_operations_are_refused(&mut runtime);
}

#[test]
fn sleep_commit_followed_by_journal_preparation_failure_fail_stops() {
    let mut runtime = at_sleep_boundary("atomic-journal", true);
    let state = runtime.residents[&1].sleep_scheduler.state();
    runtime.pending_recovery_sleep_edges.insert(
        1,
        PendingRecoverySleepEdge {
            source: state,
            target: SleepState {
                phase: SleepPhase::Awake,
                ..state
            },
        },
    );
    let before = HostSleepSnapshot::capture(&runtime);
    let mut committed = false;
    let result = tick_with_fault(&mut runtime, &mut |backend, handle, id, state, intent| {
        let event = production_progress(backend, handle, id, state, intent)?;
        committed |= id == OrganismId(1)
            && matches!(event, Some(ConsolidationDriverEvent::Committed { .. }));
        Ok(event)
    });
    assert!(
        committed,
        "the journal mismatch must follow the native Completed commit"
    );
    assert!(result.is_err());
    assert_eq!(HostSleepSnapshot::capture(&runtime), before);
    assert_stopped_operations_are_refused(&mut runtime);
}

#[test]
fn stopped_sleep_session_refuses_portable_capture_and_manual_save() {
    let mut runtime = recovery_sleep_tests::fixture("atomic-export");
    runtime
        .backend
        .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
    assert_stopped_operations_are_refused(&mut runtime);
}

#[test]
fn later_organism_failure_restores_population_after_native_sleep_submit() {
    // The canonical launcher admits at least four founders. Exercise an ordered
    // pair within that existing fixture; the other founders remain untouched.
    let mut runtime = recovery_sleep_tests::fixture("atomic-population");
    for id in [OrganismId(1), OrganismId(2)] {
        runtime.request_recovery_sleep(id).unwrap();
    }
    for _ in 0..64 {
        runtime.flush_sleep_journal_publication_blocking().unwrap();
        if [1, 2].into_iter().all(|raw| {
            matches!(
                runtime.residents[&raw]
                    .sleep_scheduler
                    .state()
                    .consolidation,
                ConsolidationState::Prepared { .. }
            )
        }) {
            let before = HostSleepSnapshot::capture(&runtime);
            let mut first_submitted = false;
            let mut second_failed = false;
            let result =
                tick_with_fault(&mut runtime, &mut |backend, handle, id, state, intent| {
                    if id == OrganismId(2) {
                        assert!(
                            first_submitted,
                            "organism A must reach the native device first"
                        );
                        second_failed = true;
                        return Err(ScaffoldContractError::InvalidMemoryQuery);
                    }
                    let event = production_progress(backend, handle, id, state, intent)?;
                    if id == OrganismId(1) {
                        assert!(matches!(
                            event,
                            Some(ConsolidationDriverEvent::Submitted { .. })
                        ));
                        first_submitted = true;
                    }
                    Ok(event)
                });
            assert!(result.is_err() && first_submitted && second_failed);
            // Full maps cover both organisms, their world records and all sidecars.
            assert_eq!(HostSleepSnapshot::capture(&runtime), before);
            assert_stopped_operations_are_refused(&mut runtime);
            return;
        }
        runtime.tick_outcome().unwrap();
    }
    panic!("both organisms must reach Prepared in the same population tick");
}

#[test]
fn sleep_pre_device_retry_preserves_host_state_and_charges_atp_once() {
    let mut runtime = recovery_sleep_tests::fixture("atomic-retry");
    let handle = runtime.handles[&1];
    runtime
        .backend
        .set_brain_atp_q16_for_test(handle, alife_core::BRAIN_ATP_Q16_MAX / 4)
        .unwrap();
    runtime.request_recovery_sleep(OrganismId(1)).unwrap();
    for _ in 0..64 {
        if matches!(
            runtime.residents[&1].sleep_scheduler.state().consolidation,
            ConsolidationState::Pending { .. }
        ) {
            break;
        }
        runtime.tick_outcome().unwrap();
    }
    assert!(matches!(
        runtime.residents[&1].sleep_scheduler.state().consolidation,
        ConsolidationState::Pending { .. }
    ));
    runtime.flush_sleep_journal_publication_blocking().unwrap();
    let before = HostSleepSnapshot::capture(&runtime);
    let tick = runtime.world.tick();
    let atp_before = runtime.backend.snapshot_activity_state(handle).unwrap();
    let mut rejected = false;
    let result = tick_with_fault(&mut runtime, &mut |backend, handle, id, state, intent| {
        if id == OrganismId(1) {
            rejected = true;
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        production_progress(backend, handle, id, state, intent)
    });
    assert!(rejected && result.is_err());
    assert_eq!(HostSleepSnapshot::capture(&runtime), before);
    runtime.backend.ensure_neural_actions_available().unwrap();
    let charged = runtime.backend.snapshot_activity_state(handle).unwrap();
    assert_eq!(charged.last_world_atp_tick, Some(tick.raw()));
    assert!(charged.brain_atp_q16 > atp_before.brain_atp_q16);
    assert!(charged.brain_atp_q16 < alife_core::BRAIN_ATP_Q16_MAX);
    runtime.tick_outcome().unwrap();
    assert_eq!(runtime.world.tick().raw(), tick.raw() + 1);
    let retried = runtime.backend.snapshot_activity_state(handle).unwrap();
    assert_eq!(
        retried.brain_atp_q16, charged.brain_atp_q16,
        "same world tick cannot receive a second debit or sleep credit"
    );
    assert_eq!(retried.last_world_atp_tick, charged.last_world_atp_tick);
    runtime.flush_sleep_journal_publication_blocking().unwrap();
}
