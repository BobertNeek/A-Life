use alife_core::{SleepPhase, SleepState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BrainAtpWorldTickMode {
    Charge { recover: bool },
    DurabilityHold,
}

pub(crate) fn brain_atp_world_tick_mode(
    phase_before: SleepPhase,
    schedule_sleep: bool,
    completed_waiting_for_durable_permit: bool,
) -> BrainAtpWorldTickMode {
    if completed_waiting_for_durable_permit {
        BrainAtpWorldTickMode::DurabilityHold
    } else {
        BrainAtpWorldTickMode::Charge {
            recover: phase_before.is_physical_sleep() || !schedule_sleep,
        }
    }
}

pub(crate) fn sleep_recovery_body_event_due(
    phase_before: SleepPhase,
    completed_waiting_for_durable_permit: bool,
) -> bool {
    phase_before.is_physical_sleep() && !completed_waiting_for_durable_permit
}

pub(crate) fn motor_eligible(phase: SleepPhase) -> bool {
    phase == SleepPhase::Awake
}

/// Explicit research mode only; ordinary dispatch remains Awake-only. Both
/// boundaries must already permit activity, so draining a frozen job cannot
/// immediately become a new aroused write in that same interval.
pub(crate) fn candidate_interrupted_dispatch_allowed(
    enabled: bool,
    durable: bool,
    before: SleepState,
    after: SleepState,
) -> bool {
    enabled
        && durable
        && before.phase.is_interrupted()
        && before.phase == after.phase
        && before.permits_neural_activity()
        && after.permits_neural_activity()
}

#[cfg(test)]
mod arousal_gate_tests {
    use super::*;
    use alife_core::{ConsolidationIntent, ConsolidationState, SleepTrigger, Tick, Validate};
    #[test]
    fn candidate_gate_requires_durable_clear_entry_and_unchanged_interrupted_phase() {
        let clear = SleepState {
            schema_version: 3,
            phase: SleepPhase::InterruptedConsolidating,
            phase_started_tick: Tick(1),
            entered_sleep_tick: Some(Tick(0)),
            cycles_completed: 0,
            last_trigger: Some(SleepTrigger::RecoveryProtocol),
            active_cycle_id: 1,
            last_consolidated_cycle_id: 0,
            consolidation: ConsolidationState::None,
        };
        clear.validate_contract().unwrap();
        assert!(candidate_interrupted_dispatch_allowed(
            true, true, clear, clear
        ));
        assert!(!candidate_interrupted_dispatch_allowed(
            false, true, clear, clear
        ));
        assert!(!candidate_interrupted_dispatch_allowed(
            true, false, clear, clear
        ));
        let committed = SleepState {
            consolidation: ConsolidationState::Committed {
                cycle_id: 1,
                output_generation: 10,
                output_digest: [1; 4],
            },
            ..clear
        };
        committed.validate_contract().unwrap();
        assert!(candidate_interrupted_dispatch_allowed(
            true, true, committed, committed
        ));
        let pending = SleepState {
            consolidation: ConsolidationState::Pending {
                intent: ConsolidationIntent { cycle_id: 1 },
                replay_digest: [2; 4],
                replay_event_count: 1,
                replay_eligibility_sample_count: 0,
            },
            ..clear
        };
        pending.validate_contract().unwrap();
        assert!(!candidate_interrupted_dispatch_allowed(
            true, true, pending, committed
        ));
        let resumed = SleepState {
            phase: SleepPhase::Consolidating,
            ..committed
        };
        assert!(!candidate_interrupted_dispatch_allowed(
            true, true, committed, resumed
        ));
        let awake = SleepState::awake_at(Tick(1));
        assert!(!candidate_interrupted_dispatch_allowed(
            true, true, awake, awake
        ));
    }
    #[test]
    fn legacy_recovery_is_unchanged_and_interrupted_phases_do_not_earn_sleep() {
        for phase in [
            SleepPhase::Awake,
            SleepPhase::EnteringSleep,
            SleepPhase::Consolidating,
            SleepPhase::Waking,
            SleepPhase::ForcedRecoverySleep,
        ] {
            assert_eq!(
                brain_atp_world_tick_mode(phase, true, false),
                BrainAtpWorldTickMode::Charge {
                    recover: phase != SleepPhase::Awake
                }
            );
            assert_eq!(
                sleep_recovery_body_event_due(phase, false),
                phase != SleepPhase::Awake
            );
        }
        for phase in [
            SleepPhase::InterruptedEnteringSleep,
            SleepPhase::InterruptedConsolidating,
            SleepPhase::InterruptedWaking,
            SleepPhase::InterruptedForcedRecoverySleep,
        ] {
            assert_eq!(
                brain_atp_world_tick_mode(phase, true, false),
                BrainAtpWorldTickMode::Charge { recover: false }
            );
            assert!(!sleep_recovery_body_event_due(phase, false));
            assert_eq!(
                brain_atp_world_tick_mode(phase, true, true),
                BrainAtpWorldTickMode::DurabilityHold
            );
        }
    }
}
