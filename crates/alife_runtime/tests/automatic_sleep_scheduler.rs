//! Canonical no-action scheduler cycle before the GPU consolidation driver is wired.
use alife_core::sleep::{SleepWorkReceipt, SleepWorkStatus};
use alife_core::{
    BrainCapacityClass, ConsolidationDriverEvent, ConsolidationIntent, ConsolidationJobId,
    ConsolidationStagedOutput, CreatureGenome, FoundationGeneticIdentity, GpuConsolidationRequest,
    HomeostaticParameters, HomeostaticSnapshot, NormalizedScalar, OrganismId, PhenotypeHash,
    ScaffoldContractError, SleepConsolidationConfig, SleepPhase, SleepState, Tick, Validate,
    WorldEntityId, GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
    SLEEP_CONSOLIDATION_SCHEMA_VERSION,
};
use alife_runtime::{GpuSleepConsolidationDriver, GpuSleepScheduler, SleepWorkDue};
use alife_world::WorldOrganismRecord;

struct RecordingConsolidationDriver {
    intents: Vec<ConsolidationIntent>,
    expected_organism_id: OrganismId,
    has_phase_data: bool,
    bounded_calls: u32,
    persisted_replay_event_count: Option<u32>,
}

struct StructuralBeforePendingDriver {
    calls: Vec<&'static str>,
    replay_digest: [u64; 4],
}

impl StructuralBeforePendingDriver {
    fn new() -> Self {
        Self {
            calls: Vec::new(),
            replay_digest: [1, 2, 3, 4],
        }
    }

    fn skipped_receipt(tick: Tick) -> SleepWorkReceipt {
        let mut receipt = SleepWorkReceipt {
            schema_version: SLEEP_CONSOLIDATION_SCHEMA_VERSION,
            tick,
            status: SleepWorkStatus::SkippedLowPressure,
            fatigue: NormalizedScalar::new(0.0).unwrap(),
            sleep_pressure: NormalizedScalar::new(0.0).unwrap(),
            replay_digest: [0; 4],
            replay_event_count: 0,
            replay_eligibility_sample_count: 0,
            promoted_memory_ids: Vec::new(),
            predictor_update_count: 0,
            concept: None,
            work_units: 0,
            canonical_digest: [0; 4],
        };
        receipt.canonical_digest = receipt.recompute_canonical_digest().unwrap();
        receipt.validate_contract().unwrap();
        receipt
    }
}

impl GpuSleepConsolidationDriver for StructuralBeforePendingDriver {
    fn progress(
        &mut self,
        _organism_id: OrganismId,
        _state: SleepState,
        intent: Option<ConsolidationIntent>,
    ) -> Result<Option<ConsolidationDriverEvent>, ScaffoldContractError> {
        let Some(intent) = intent else {
            return Ok(None);
        };
        self.calls.push("ReplayAssetPersisted");
        Ok(Some(ConsolidationDriverEvent::ReplayAssetPersisted {
            intent,
            replay_digest: self.replay_digest,
            replay_event_count: 1,
            replay_eligibility_sample_count: 1,
        }))
    }

    fn run_bounded_sleep_transaction(
        &mut self,
        _organism_id: OrganismId,
        _state: SleepState,
        _homeostasis: &HomeostaticSnapshot,
        tick: Tick,
        due_work: SleepWorkDue,
    ) -> Result<Option<SleepWorkReceipt>, ScaffoldContractError> {
        assert!(due_work.contains(SleepWorkDue::STRUCTURAL_GROWTH_PRUNING));
        self.calls.push("bounded-structural-work");
        self.replay_digest = [11, 12, 13, 14];
        Ok(Some(Self::skipped_receipt(tick)))
    }
}

impl Default for RecordingConsolidationDriver {
    fn default() -> Self {
        Self::with_phase_data(OrganismId(1), true)
    }
}

#[derive(Default)]
struct FailFirstIntentDriver {
    calls_with_intent: u32,
}

impl GpuSleepConsolidationDriver for FailFirstIntentDriver {
    fn progress(
        &mut self,
        _organism_id: alife_core::OrganismId,
        _state: alife_core::SleepState,
        intent: Option<ConsolidationIntent>,
    ) -> Result<Option<ConsolidationDriverEvent>, alife_core::ScaffoldContractError> {
        let Some(intent) = intent else {
            return Ok(None);
        };
        self.calls_with_intent += 1;
        if self.calls_with_intent == 1 {
            return Err(alife_core::ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(Some(ConsolidationDriverEvent::ReplayAssetPersisted {
            intent,
            replay_digest: [71, 72, 73, 74],
            replay_event_count: 1,
            replay_eligibility_sample_count: 1,
        }))
    }
}

impl RecordingConsolidationDriver {
    fn with_phase_data(expected_organism_id: OrganismId, has_phase_data: bool) -> Self {
        Self {
            intents: Vec::new(),
            expected_organism_id,
            has_phase_data,
            bounded_calls: 0,
            persisted_replay_event_count: None,
        }
    }

    fn intents(&self) -> &[ConsolidationIntent] {
        &self.intents
    }
}

impl GpuSleepConsolidationDriver for RecordingConsolidationDriver {
    fn progress(
        &mut self,
        organism_id: alife_core::OrganismId,
        state: alife_core::SleepState,
        intent: Option<ConsolidationIntent>,
    ) -> Result<Option<ConsolidationDriverEvent>, alife_core::ScaffoldContractError> {
        assert_eq!(organism_id, self.expected_organism_id);
        if let Some(intent) = intent {
            self.intents.push(intent);
            let replay_event_count = u32::from(self.has_phase_data);
            self.persisted_replay_event_count = Some(replay_event_count);
            return Ok(Some(ConsolidationDriverEvent::ReplayAssetPersisted {
                intent,
                replay_digest: [11, 12, 13, 14],
                replay_event_count,
                replay_eligibility_sample_count: replay_event_count,
            }));
        }
        let event = match state.consolidation {
            alife_core::ConsolidationState::Pending {
                intent,
                replay_digest,
                replay_event_count,
                replay_eligibility_sample_count,
            } => {
                let mut request = GpuConsolidationRequest {
                    schema_version: GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
                    request_flags: 0,
                    cycle_id: intent.cycle_id,
                    phenotype_hash: PhenotypeHash([21, 22, 23, 24]),
                    input_generation: 1,
                    expected_output_generation: 2,
                    input_digest: [31, 32, 33, 34],
                    replay_digest,
                    max_replay_events: replay_event_count.max(1),
                    max_replay_eligibility_samples: replay_eligibility_sample_count.max(1),
                    request_digest: [0; 4],
                };
                request.request_digest = request.recompute_request_digest()?;
                ConsolidationDriverEvent::Prepared { request }
            }
            alife_core::ConsolidationState::Prepared { request } => {
                ConsolidationDriverEvent::Submitted {
                    request,
                    job_id: ConsolidationJobId::try_from_raw(1)?,
                }
            }
            alife_core::ConsolidationState::Submitted { request, job_id } => {
                let mut staged = ConsolidationStagedOutput {
                    job_id,
                    output_generation: request.expected_output_generation,
                    output_weight_bank: 1,
                    output_digest: [41, 42, 43, 44],
                    eligibility_reset_generation: 2,
                    output_eligibility_bank: 0,
                    eligibility_output_digest: [51, 52, 53, 54],
                    replay_journal_generation: 2,
                    replay_journal_cursor: 0,
                    replay_journal_event_count: 0,
                    replay_journal_output_digest: [61, 62, 63, 64],
                    staging_digest: [0; 4],
                    promoted_fast_l1_bits: 0.25_f32.to_bits(),
                    replay_induced_fast_l1_bits: 0.125_f32.to_bits(),
                };
                staged.staging_digest = staged.recompute_staging_digest(&request, 1, 1)?;
                ConsolidationDriverEvent::Completed { request, staged }
            }
            alife_core::ConsolidationState::Completed { request, staged } => {
                ConsolidationDriverEvent::Committed {
                    cycle_id: request.cycle_id,
                    output_generation: staged.output_generation,
                    output_digest: staged.output_digest,
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(event))
    }

    fn run_bounded_sleep_transaction(
        &mut self,
        _organism_id: OrganismId,
        _state: SleepState,
        _homeostasis: &HomeostaticSnapshot,
        _tick: Tick,
        _due_work: SleepWorkDue,
    ) -> Result<Option<SleepWorkReceipt>, ScaffoldContractError> {
        self.bounded_calls += 1;
        Ok(None)
    }

    fn has_bounded_sleep_phase_data(
        &mut self,
        _organism_id: OrganismId,
        _state: SleepState,
    ) -> Result<bool, ScaffoldContractError> {
        Ok(self.has_phase_data)
    }
}

fn fatigued_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    let mut drives = alife_core::DriveSnapshot::baseline();
    drives.fatigue = 0.99;
    let mut hormones = alife_core::EndocrineSnapshot::baseline();
    hormones.sleep_pressure = 0.99;
    HomeostaticSnapshot::new(tick, drives, hormones).unwrap()
}

fn newborn_record(organism_id: u64) -> WorldOrganismRecord {
    let genome = CreatureGenome::early_mammal_founder(
        0xE11_0000 + organism_id,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    WorldOrganismRecord::newborn(
        OrganismId(organism_id),
        WorldEntityId(100 + organism_id),
        genome,
        phenotype,
        Tick::ZERO,
    )
    .unwrap()
}

#[test]
fn bounded_structural_work_precedes_first_pending_replay_identity() {
    let config = SleepConsolidationConfig {
        entering_duration: alife_core::DurationTicks::new(1),
        ..SleepConsolidationConfig::reference()
    };
    let mut scheduler = GpuSleepScheduler::new(config).unwrap();
    let mut organism = newborn_record(13);
    let mut driver = StructuralBeforePendingDriver::new();
    scheduler.force_recovery_sleep(Tick::ZERO).unwrap();

    let mut event = None;
    for raw_tick in 1..=64 {
        let next = scheduler
            .scheduled_tick_with_organism(
                &mut organism,
                HomeostaticParameters::reference(),
                Tick::new(raw_tick),
                &mut driver,
                false,
            )
            .unwrap();
        event = Some(next);
        if !driver.calls.is_empty() {
            break;
        }
    }
    let event = event.unwrap();

    assert_eq!(
        driver.calls,
        vec!["bounded-structural-work", "ReplayAssetPersisted"]
    );
    assert!(event
        .phase_receipt
        .due_work
        .contains(SleepWorkDue::STRUCTURAL_GROWTH_PRUNING));
    assert!(matches!(
        scheduler.state().consolidation,
        alife_core::ConsolidationState::Pending {
            replay_digest: [11, 12, 13, 14],
            ..
        }
    ));
}

#[test]
fn fatigue_enters_sleep_requests_once_emits_no_actions_and_wakes_after_completion() {
    let config = SleepConsolidationConfig {
        fatigue_threshold: NormalizedScalar::new(0.8).unwrap(),
        sleep_pressure_threshold: NormalizedScalar::new(0.8).unwrap(),
        entering_duration: alife_core::DurationTicks::new(1),
        waking_duration: alife_core::DurationTicks::new(1),
        ..SleepConsolidationConfig::reference()
    };
    let mut scheduler = GpuSleepScheduler::new(config).unwrap();
    let mut driver = RecordingConsolidationDriver::default();
    let mut events = Vec::new();

    for raw_tick in 1..=64 {
        let tick = Tick::new(raw_tick);
        let event = scheduler
            .scheduled_tick(
                alife_core::OrganismId(1),
                &fatigued_homeostasis(tick),
                HomeostaticParameters::reference(),
                tick,
                &mut driver,
            )
            .unwrap();
        let completed_cycle = event.phase == SleepPhase::Awake && event.cycle_id > 0;
        events.push(event);
        if completed_cycle {
            break;
        }
    }

    assert!(events
        .iter()
        .any(|event| event.phase == SleepPhase::EnteringSleep));
    assert_eq!(driver.intents().len(), 1);
    let consolidating = events
        .iter()
        .find(|event| event.phase == SleepPhase::Consolidating)
        .unwrap();
    assert_eq!(driver.intents()[0].cycle_id, consolidating.cycle_id);
    assert_eq!(events.last().unwrap().phase, SleepPhase::Awake);
    assert!(events
        .iter()
        .filter(|event| event.phase != SleepPhase::Awake)
        .all(|event| event.selected_action.is_none()));
}

#[test]
fn failed_initial_driver_call_does_not_strand_the_sleep_cycle() {
    let config = SleepConsolidationConfig {
        fatigue_threshold: NormalizedScalar::new(0.8).unwrap(),
        sleep_pressure_threshold: NormalizedScalar::new(0.8).unwrap(),
        entering_duration: alife_core::DurationTicks::new(1),
        ..SleepConsolidationConfig::reference()
    };
    let mut scheduler = GpuSleepScheduler::new(config).unwrap();
    let mut driver = FailFirstIntentDriver::default();
    scheduler
        .scheduled_tick(
            alife_core::OrganismId(1),
            &fatigued_homeostasis(Tick::ZERO),
            HomeostaticParameters::reference(),
            Tick::ZERO,
            &mut driver,
        )
        .unwrap();

    let first = scheduler.scheduled_tick(
        alife_core::OrganismId(1),
        &fatigued_homeostasis(Tick::new(1)),
        HomeostaticParameters::reference(),
        Tick::new(1),
        &mut driver,
    );
    assert_eq!(
        first,
        Err(alife_core::ScaffoldContractError::NeuralBackendUnavailable)
    );
    assert_eq!(scheduler.state().phase, SleepPhase::Consolidating);
    assert_eq!(
        scheduler.state().consolidation,
        alife_core::ConsolidationState::None
    );

    scheduler
        .scheduled_tick(
            alife_core::OrganismId(1),
            &fatigued_homeostasis(Tick::new(2)),
            HomeostaticParameters::reference(),
            Tick::new(2),
            &mut driver,
        )
        .unwrap();
    assert_eq!(driver.calls_with_intent, 2);
    assert!(matches!(
        scheduler.state().consolidation,
        alife_core::ConsolidationState::Pending { .. }
    ));
}

#[test]
fn empty_replay_is_zero_work_but_nonempty_missing_phase_data_stays_fail_closed() {
    let config = SleepConsolidationConfig {
        entering_duration: alife_core::DurationTicks::new(1),
        waking_duration: alife_core::DurationTicks::new(1),
        ..SleepConsolidationConfig::reference()
    };
    let mut empty_scheduler = GpuSleepScheduler::new(config).unwrap();
    let mut empty_organism = newborn_record(11);
    let mut empty_driver = RecordingConsolidationDriver::with_phase_data(OrganismId(11), false);
    empty_scheduler.force_recovery_sleep(Tick::ZERO).unwrap();
    let mut empty_events = Vec::new();

    for raw_tick in 1..=64 {
        let event = empty_scheduler
            .scheduled_tick_with_organism(
                &mut empty_organism,
                HomeostaticParameters::reference(),
                Tick::new(raw_tick),
                &mut empty_driver,
                false,
            )
            .unwrap();
        let completed_cycle = event.phase == SleepPhase::Awake && event.cycle_id > 0;
        empty_events.push(event);
        if completed_cycle {
            break;
        }
    }

    assert_eq!(empty_driver.persisted_replay_event_count, Some(0));
    assert_eq!(empty_driver.bounded_calls, 0);
    assert!(empty_events.iter().all(|event| {
        event.sleep_work_units == 0
            && event.phase_receipt.work_units == 0
            && event.phase_receipt.due_work.is_empty()
    }));
    assert_eq!(empty_events.last().unwrap().phase, SleepPhase::Awake);

    let mut nonempty_scheduler = GpuSleepScheduler::new(config).unwrap();
    let mut nonempty_organism = newborn_record(12);
    let mut nonempty_driver = RecordingConsolidationDriver::with_phase_data(OrganismId(12), true);
    nonempty_scheduler.force_recovery_sleep(Tick::ZERO).unwrap();
    let mut result = Ok(None);
    for raw_tick in 1..=64 {
        match nonempty_scheduler.scheduled_tick_with_organism(
            &mut nonempty_organism,
            HomeostaticParameters::reference(),
            Tick::new(raw_tick),
            &mut nonempty_driver,
            false,
        ) {
            Ok(event) => result = Ok(Some(event)),
            Err(error) => {
                result = Err(error);
                break;
            }
        }
    }

    assert_eq!(result, Err(ScaffoldContractError::MissingPhaseData));
    assert_eq!(nonempty_driver.persisted_replay_event_count, None);
    assert_eq!(nonempty_driver.bounded_calls, 1);
    assert_eq!(
        nonempty_scheduler.state().consolidation,
        alife_core::ConsolidationState::None
    );
}
