//! Exact legacy JSON storage bytes and a parent sentinel, captured before the
//! version3 implementation. The emitter refuses to overwrite an existing path.
use alife_core::{
    ConsolidationIntent, ConsolidationState, OrganismId, SleepPhase, SleepState, SleepTrigger,
    Tick, SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};
use alife_runtime::{GpuSleepTransactionJournalEntryV2, GpuSleepTransactionJournalV2};

fn legacy_fixture() -> GpuSleepTransactionJournalV2 {
    let source = SleepState {
        schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
        phase: SleepPhase::ForcedRecoverySleep,
        phase_started_tick: Tick::new(2),
        entered_sleep_tick: Some(Tick::new(2)),
        cycles_completed: 0,
        last_trigger: Some(SleepTrigger::RecoveryProtocol),
        active_cycle_id: 1,
        last_consolidated_cycle_id: 0,
        consolidation: ConsolidationState::None,
    };
    let middle = SleepState {
        phase: SleepPhase::Consolidating,
        phase_started_tick: Tick::new(3),
        ..source
    };
    let target = SleepState {
        consolidation: ConsolidationState::Pending {
            intent: ConsolidationIntent { cycle_id: 1 },
            replay_digest: [9; 4],
            replay_event_count: 1,
            replay_eligibility_sample_count: 0,
        },
        ..middle
    };
    let entries = vec![
        GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
            OrganismId(1),
            Tick::new(3),
            0,
            source,
            middle,
        )
        .unwrap(),
        GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
            OrganismId(1),
            Tick::new(3),
            1,
            middle,
            target,
        )
        .unwrap(),
    ];
    GpuSleepTransactionJournalV2::try_new(
        "fnv1a64:0123456789abcdef".to_string(),
        Tick::new(2),
        entries,
    )
    .unwrap()
}

#[test]
fn legacy_v2_journal_storage_bytes_and_nested_sentinel_are_preserved() {
    let journal = legacy_fixture();
    assert_eq!(journal.schema_version, 2);
    journal.validate().unwrap();
    let bytes = serde_json::to_vec(&(journal.clone(), 42_u8)).unwrap();
    if let Ok(path) = std::env::var("ALIFE_EMIT_LEGACY_JOURNAL_FIXTURE") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(&bytes).unwrap();
        println!(
            "captured legacy version2 journal fixture: {} bytes",
            bytes.len()
        );
        return;
    }
    assert_eq!(
        bytes.as_slice(),
        include_bytes!("fixtures/sleep_journal_legacy_v2.json")
    );
    let (decoded, sentinel): (GpuSleepTransactionJournalV2, u8) =
        serde_json::from_slice(include_bytes!("fixtures/sleep_journal_legacy_v2.json")).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded, journal);
    assert_eq!(sentinel, 42);
}
use alife_core::{
    ConsolidationJobId, ConsolidationStagedOutput, GpuConsolidationRequest, PhenotypeHash,
    GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
};
// Source-only pending tests. Append after the native legacy fixture is captured.
#[test]
fn interrupted_journal_edges_preserve_frozen_payload_and_reject_downgrade() {
    use alife_core::{SleepConsolidationConfig, SleepController};
    let legacy = legacy_fixture();
    let request = valid_request_for(1, [9; 4]);
    let staged = valid_staged(&request);
    let base = legacy.entries[0].target;
    let stages = [
        legacy.entries[0].source,
        base,
        legacy.entries[1].target,
        SleepState {
            consolidation: ConsolidationState::Prepared { request },
            ..base
        },
        SleepState {
            consolidation: ConsolidationState::Submitted {
                request,
                job_id: staged.job_id,
            },
            ..base
        },
        SleepState {
            consolidation: ConsolidationState::Completed { request, staged },
            ..base
        },
        SleepState {
            consolidation: ConsolidationState::Committed {
                cycle_id: 1,
                output_generation: staged.output_generation,
                output_digest: staged.output_digest,
            },
            ..base
        },
        SleepState {
            phase: SleepPhase::Waking,
            consolidation: ConsolidationState::Committed {
                cycle_id: 1,
                output_generation: staged.output_generation,
                output_digest: staged.output_digest,
            },
            ..base
        },
    ];
    for source in stages {
        let mut controller =
            SleepController::restore(SleepConsolidationConfig::reference(), source).unwrap();
        controller
            .interrupt_for_acute_pain(Tick::new(4))
            .unwrap()
            .unwrap();
        let interrupted = controller.state();
        assert_eq!(interrupted.consolidation, source.consolidation);
        let entry = GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            source,
            interrupted,
        )
        .unwrap();
        let journal = GpuSleepTransactionJournalV2::try_new(
            "fnv1a64:0123456789abcdef".to_string(),
            Tick::new(2),
            vec![entry],
        )
        .unwrap();
        assert_eq!(journal.schema_version, 3);
        let mut downgraded = journal.clone();
        downgraded.schema_version = 2;
        assert!(downgraded.validate().is_err());
        let bytes = serde_json::to_vec(&(journal.clone(), 42_u8)).unwrap();
        let (decoded, sentinel): (GpuSleepTransactionJournalV2, u8) =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, journal);
        assert_eq!(sentinel, 42);
        decoded.validate().unwrap();
        controller
            .resume_interrupted_sleep(Tick::new(5))
            .unwrap()
            .unwrap();
        let resumed = controller.state();
        assert_eq!(resumed.consolidation, source.consolidation);
        assert_eq!(resumed.active_cycle_id, source.active_cycle_id);
        GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(5),
            interrupted,
            resumed,
        )
        .unwrap();
        let mut rewritten = interrupted;
        rewritten.last_trigger = Some(SleepTrigger::ForcedRequest);
        assert!(GpuSleepTransactionJournalEntryV2::try_new(
            OrganismId(1),
            Tick::new(4),
            source,
            rewritten
        )
        .is_err());
    }
}

#[test]
fn same_tick_journal_prefix_accepts_one_final_interrupt_not_a_second_edge() {
    use alife_core::{SleepConsolidationConfig, SleepController};
    let mut journal = legacy_fixture();
    let source = journal.entries[1].target;
    let mut controller =
        SleepController::restore(SleepConsolidationConfig::reference(), source).unwrap();
    controller
        .interrupt_for_acute_pain(Tick::new(3))
        .unwrap()
        .unwrap();
    let interrupted = controller.state();
    let edge = GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
        OrganismId(1),
        Tick::new(3),
        2,
        source,
        interrupted,
    )
    .unwrap();
    journal.entries.push(edge);
    let accepted = GpuSleepTransactionJournalV2::try_new(
        journal.exact_base_manifest_digest.clone(),
        journal.exact_base_checkpoint_tick,
        journal.entries.clone(),
    )
    .unwrap();
    assert_eq!(accepted.schema_version, 3);
    controller
        .resume_interrupted_sleep(Tick::new(3))
        .unwrap()
        .unwrap();
    assert!(GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
        OrganismId(1),
        Tick::new(3),
        3,
        interrupted,
        controller.state()
    )
    .is_err());
    let mut ordinal_gap = journal.entries;
    ordinal_gap.remove(1);
    assert!(GpuSleepTransactionJournalV2::try_new(
        accepted.exact_base_manifest_digest,
        accepted.exact_base_checkpoint_tick,
        ordinal_gap
    )
    .is_err());
}

fn valid_request_for(cycle_id: u64, replay_digest: [u64; 4]) -> GpuConsolidationRequest {
    let mut request = GpuConsolidationRequest {
        schema_version: GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
        request_flags: 0,
        cycle_id,
        phenotype_hash: PhenotypeHash([1, 2, 3, 4]),
        input_generation: 9,
        expected_output_generation: 10,
        input_digest: [5, 6, 7, 8],
        replay_digest,
        max_replay_events: 8,
        max_replay_eligibility_samples: 32,
        request_digest: [0; 4],
    };
    request.request_digest = request.recompute_request_digest().unwrap();
    request
}

fn valid_staged(request: &GpuConsolidationRequest) -> ConsolidationStagedOutput {
    let mut staged = ConsolidationStagedOutput {
        job_id: ConsolidationJobId::try_from_raw(3).unwrap(),
        output_generation: request.expected_output_generation,
        output_weight_bank: 1,
        output_digest: [13, 14, 15, 16],
        eligibility_reset_generation: 8,
        output_eligibility_bank: 0,
        eligibility_output_digest: [17, 18, 19, 20],
        replay_journal_generation: 7,
        replay_journal_cursor: 0,
        replay_journal_event_count: 0,
        replay_journal_output_digest: [21, 22, 23, 24],
        staging_digest: [0; 4],
        promoted_fast_l1_bits: 0.5_f32.to_bits(),
        replay_induced_fast_l1_bits: 0.25_f32.to_bits(),
    };
    staged.staging_digest = staged.recompute_staging_digest(request, 1, 1).unwrap();
    staged
}

#[test]
fn version3_phase_edge_requires_actual_transition_tick_without_changing_version2() {
    let legacy = legacy_fixture();
    let source = legacy.entries[0].source;
    let target = legacy.entries[0].target;
    GpuSleepTransactionJournalEntryV2::try_new(OrganismId(1), Tick::new(4), source, target)
        .unwrap();
    let source = SleepState {
        schema_version: 3,
        ..source
    };
    let target = SleepState {
        schema_version: 3,
        ..target
    };
    assert!(GpuSleepTransactionJournalEntryV2::try_new(
        OrganismId(1),
        Tick::new(4),
        source,
        target
    )
    .is_err());
    GpuSleepTransactionJournalEntryV2::try_new(OrganismId(1), Tick::new(3), source, target)
        .unwrap();
}
