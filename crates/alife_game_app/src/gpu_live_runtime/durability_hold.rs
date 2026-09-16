use alife_core::SleepPhase;

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
            recover: phase_before != SleepPhase::Awake || !schedule_sleep,
        }
    }
}

pub(crate) fn sleep_recovery_body_event_due(
    phase_before: SleepPhase,
    completed_waiting_for_durable_permit: bool,
) -> bool {
    phase_before != SleepPhase::Awake && !completed_waiting_for_durable_permit
}

pub(crate) fn motor_eligible(phase: SleepPhase) -> bool {
    phase == SleepPhase::Awake
}
