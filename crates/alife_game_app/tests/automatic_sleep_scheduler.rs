//! Canonical no-action scheduler cycle before the GPU consolidation driver is wired.
#![cfg(feature = "gpu-runtime")]

use alife_core::{
    ConsolidationDriverEvent, ConsolidationIntent, ConsolidationJobId, ConsolidationStagedOutput,
    GpuConsolidationRequest, HomeostaticParameters, HomeostaticSnapshot, NormalizedScalar,
    PhenotypeHash, SleepConsolidationConfig, SleepPhase, Tick,
    GPU_CONSOLIDATION_REQUEST_SCHEMA_VERSION,
};
use alife_game_app::{GpuSleepConsolidationDriver, GpuSleepScheduler};

#[derive(Default)]
struct RecordingConsolidationDriver {
    intents: Vec<ConsolidationIntent>,
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
        assert_eq!(organism_id, alife_core::OrganismId(1));
        if let Some(intent) = intent {
            self.intents.push(intent);
            return Ok(Some(ConsolidationDriverEvent::ReplayAssetPersisted {
                intent,
                replay_digest: [11, 12, 13, 14],
                replay_event_count: 1,
                replay_eligibility_sample_count: 1,
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
}

fn fatigued_homeostasis(tick: Tick) -> HomeostaticSnapshot {
    let mut drives = alife_core::DriveSnapshot::baseline();
    drives.fatigue = 0.99;
    let mut hormones = alife_core::EndocrineSnapshot::baseline();
    hormones.sleep_pressure = 0.99;
    HomeostaticSnapshot::new(tick, drives, hormones).unwrap()
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
