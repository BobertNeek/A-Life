use alife_core::{
    ConsolidationDriverEvent, ConsolidationIntent, ConsolidationJobId, ConsolidationStagedOutput,
    DurationTicks, ExperiencePatchPhase, GpuConsolidationRequest, PhenotypeHash,
    SleepConsolidationConfig, SleepController, SleepPhase, SleepTrigger, Tick, Validate,
    GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
};
use alife_world::{ScenarioAssertions, ScenarioFixture, ScenarioName, ScenarioRun};

const FAST_SOAK_CYCLES: usize = 3;
const EXTENDED_SOAK_CYCLES: usize = 24;
const MAX_MEMORY_RECORDS_PER_RUN: usize = 16;
const MAX_TOPOLOGY_CONCEPTS_PER_RUN: usize = 64;
const MAX_TOPOLOGY_GAPS_PER_RUN: usize = 16;

#[test]
fn fast_headless_soak_preserves_release_gate_invariants() {
    let runs = run_soak(FAST_SOAK_CYCLES);
    assert_eq!(runs.len(), FAST_SOAK_CYCLES * ScenarioName::ALL.len());

    let sleep_runs = runs
        .iter()
        .filter(|run| run.name == ScenarioName::FatigueSleep)
        .count();
    assert_eq!(sleep_runs, FAST_SOAK_CYCLES);
    assert!(runs.iter().all(|run| !run.sleep_transition_observed));

    for run in &runs {
        assert_release_invariants(run);
    }

    assert_replay_is_deterministic_after_soak();
    assert_repeated_sleep_wake_controller_sequence_is_deterministic();
}

#[test]
#[ignore = "manual extended P36 headless soak: cargo test -p alife_world --test headless_soak -- --ignored --nocapture"]
fn manual_extended_headless_soak_preserves_release_gate_invariants() {
    let runs = run_soak(EXTENDED_SOAK_CYCLES);
    assert_eq!(runs.len(), EXTENDED_SOAK_CYCLES * ScenarioName::ALL.len());
    for run in &runs {
        assert_release_invariants(run);
    }
}

fn run_soak(cycles: usize) -> Vec<ScenarioRun> {
    let mut runs = Vec::with_capacity(cycles * ScenarioName::ALL.len());
    for cycle in 0..cycles {
        for (scenario_index, name) in ScenarioName::ALL.into_iter().enumerate() {
            let seed = 360_000 + (cycle as u64 * 1_000) + scenario_index as u64;
            let fixture = ScenarioFixture::with_seed(name, seed).unwrap();
            ScenarioAssertions::assert_fixture_is_complete(&fixture);
            let run = fixture.run().unwrap();
            ScenarioAssertions::assert_run_matches_expectations(&fixture, &run);
            runs.push(run);
        }
    }
    runs
}

fn assert_release_invariants(run: &ScenarioRun) {
    assert!(!run.patches.is_empty(), "{:?}: no sealed patches", run.name);
    assert!(
        run.memory_record_count <= MAX_MEMORY_RECORDS_PER_RUN,
        "{:?}: memory grew past release soak bound: {}",
        run.name,
        run.memory_record_count
    );
    assert!(
        run.topology_concept_count <= MAX_TOPOLOGY_CONCEPTS_PER_RUN,
        "{:?}: topology concepts grew past release soak bound: {}",
        run.name,
        run.topology_concept_count
    );
    assert!(
        run.topology_gap_ids.len() <= MAX_TOPOLOGY_GAPS_PER_RUN,
        "{:?}: topology gaps grew past release soak bound: {}",
        run.name,
        run.topology_gap_ids.len()
    );
    assert_homeostasis_is_finite_and_bounded(run);
    assert_patch_sequence_is_causal_and_sealed(run);
    assert_memory_and_topology_stay_bias_only(run);
}

fn assert_homeostasis_is_finite_and_bounded(run: &ScenarioRun) {
    run.final_homeostasis.validate_contract().unwrap();
    for value in run.final_homeostasis.drives.to_array() {
        assert!(
            value.is_finite() && (0.0..=1.0).contains(&value),
            "{:?}: invalid final drive value {value}",
            run.name
        );
    }
    for value in run.final_homeostasis.hormones.to_array() {
        assert!(
            value.is_finite() && (0.0..=1.0).contains(&value),
            "{:?}: invalid final hormone value {value}",
            run.name
        );
    }
}

fn assert_patch_sequence_is_causal_and_sealed(run: &ScenarioRun) {
    let mut last_tick = None;
    for patch in &run.patches {
        ScenarioAssertions::assert_causal_patch_fields(patch);
        ScenarioAssertions::assert_selected_action_came_from_arbitration(patch);
        assert_eq!(patch.header().phase, ExperiencePatchPhase::Sealed);

        let tick = patch.pre_action().tick.raw();
        let outcome_tick = patch.outcome().outcome_tick.raw();
        assert!(
            outcome_tick >= tick,
            "{:?}: outcome tick precedes pre-action tick",
            run.name
        );
        if let Some(last_tick) = last_tick {
            assert!(
                tick >= last_tick,
                "{:?}: non-monotonic patch tick",
                run.name
            );
        }
        last_tick = Some(tick);

        patch
            .decision()
            .selected_action
            .validate_contract()
            .unwrap();
        assert!(patch.pre_action().organism_id.raw() > 0);
        if let Some(target) = patch.decision().selected_action.target_entity {
            assert!(target.raw() > 0, "{:?}: invalid target id", run.name);
        }
    }
}

fn assert_memory_and_topology_stay_bias_only(run: &ScenarioRun) {
    assert_eq!(
        run.memory_record_count,
        run.patches.len(),
        "{:?}: memory did not consume one sealed record per patch",
        run.name
    );
    assert!(
        run.topology_simplex_count >= run.patches.len(),
        "{:?}: topology did not bind sealed patches",
        run.name
    );
    for patch in &run.patches {
        let memory_expectancy = &patch
            .pre_action()
            .heuristic_evidence()
            .expect("scenario patches use heuristic baseline evidence")
            .memory_expectancy;
        ScenarioAssertions::assert_memory_expectancy_snapshot_is_bias_only(memory_expectancy);
    }
}

fn assert_replay_is_deterministic_after_soak() {
    for name in ScenarioName::ALL {
        let first = ScenarioFixture::with_seed(name, 936_000)
            .unwrap()
            .run()
            .unwrap();
        let second = ScenarioFixture::with_seed(name, 936_000)
            .unwrap()
            .run()
            .unwrap();
        assert_eq!(first.summary, second.summary, "{name:?}: replay drift");
        assert_eq!(
            first.patch_summaries, second.patch_summaries,
            "{name:?}: patch summary drift"
        );
    }
}

fn assert_repeated_sleep_wake_controller_sequence_is_deterministic() {
    let mut controller = SleepController::new(fast_sleep_config()).unwrap();
    for cycle in 0..3 {
        let base = Tick::new(cycle * 10);
        let forced = controller
            .force_sleep(base, SleepTrigger::ForcedRequest)
            .unwrap();
        assert_eq!(forced.to, SleepPhase::ForcedRecoverySleep);
        assert_eq!(
            controller.advance(Tick::new(base.raw() + 1)).unwrap(),
            Some(transition(
                SleepPhase::ForcedRecoverySleep,
                SleepPhase::Consolidating,
                base.raw() + 1
            ))
        );
        complete_test_consolidation(&mut controller, cycle + 1);
        assert_eq!(
            controller.advance(Tick::new(base.raw() + 2)).unwrap(),
            Some(transition(
                SleepPhase::Consolidating,
                SleepPhase::Waking,
                base.raw() + 2
            ))
        );
        assert_eq!(
            controller.advance(Tick::new(base.raw() + 3)).unwrap(),
            Some(transition(
                SleepPhase::Waking,
                SleepPhase::Awake,
                base.raw() + 3
            ))
        );
        assert_eq!(controller.state().phase, SleepPhase::Awake);
        assert_eq!(controller.state().cycles_completed, cycle as u32 + 1);
    }
}

fn complete_test_consolidation(controller: &mut SleepController, cycle_id: u64) {
    let intent = ConsolidationIntent { cycle_id };
    let replay_digest = [cycle_id + 10, cycle_id + 11, cycle_id + 12, cycle_id + 13];
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::ReplayAssetPersisted {
            intent,
            replay_digest,
            replay_event_count: 1,
            replay_eligibility_sample_count: 1,
        })
        .unwrap();
    let mut request = GpuConsolidationRequest {
        schema_version: GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
        request_flags: 0,
        cycle_id,
        phenotype_hash: PhenotypeHash([1, 2, 3, 4]),
        input_generation: cycle_id,
        expected_output_generation: cycle_id + 1,
        input_digest: [21, 22, 23, 24],
        replay_digest,
        max_replay_events: 1,
        max_replay_eligibility_samples: 1,
        request_digest: [0; 4],
    };
    request.request_digest = request.recompute_request_digest().unwrap();
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Prepared { request })
        .unwrap();
    let job_id = ConsolidationJobId::try_from_raw(cycle_id + 1).unwrap();
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Submitted { request, job_id })
        .unwrap();
    let mut staged = ConsolidationStagedOutput {
        job_id,
        output_generation: request.expected_output_generation,
        output_weight_bank: (cycle_id & 1) as u8,
        output_digest: [31, 32, 33, cycle_id + 34],
        eligibility_reset_generation: cycle_id + 1,
        output_eligibility_bank: 0,
        eligibility_output_digest: [41, 42, 43, cycle_id + 44],
        replay_journal_generation: cycle_id + 1,
        replay_journal_cursor: 0,
        replay_journal_event_count: 0,
        replay_journal_output_digest: [51, 52, 53, cycle_id + 54],
        staging_digest: [0; 4],
        promoted_fast_l1_bits: 0.25_f32.to_bits(),
        replay_induced_fast_l1_bits: 0.125_f32.to_bits(),
    };
    staged.staging_digest = staged.recompute_staging_digest(&request, 1, 1).unwrap();
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Completed { request, staged })
        .unwrap();
    controller
        .apply_consolidation_driver_event(ConsolidationDriverEvent::Committed {
            cycle_id,
            output_generation: staged.output_generation,
            output_digest: staged.output_digest,
        })
        .unwrap();
}

fn fast_sleep_config() -> SleepConsolidationConfig {
    SleepConsolidationConfig {
        entering_duration: DurationTicks::new(1),
        consolidation_duration: DurationTicks::new(1),
        waking_duration: DurationTicks::new(1),
        forced_recovery_min_duration: DurationTicks::new(1),
        ..SleepConsolidationConfig::reference()
    }
}

fn transition(from: SleepPhase, to: SleepPhase, tick: u64) -> alife_core::SleepTransition {
    alife_core::SleepTransition {
        from,
        to,
        tick: Tick::new(tick),
        trigger: SleepTrigger::ForcedRequest,
    }
}
