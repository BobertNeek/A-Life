pub(crate) fn validate_phase31_performance_authority(
    schedule_failed: bool,
    gpu_authoritative: bool,
    runtime_tick_calls: u64,
    scheduler_attempts: u64,
    scheduler_completed_ticks: u64,
    completed_world_ticks: u64,
    typed_zero_progress_calls: u64,
) -> Result<(), String> {
    if schedule_failed {
        return Err("Phase 3.1 measurement rejected a failed production tick schedule".to_string());
    }
    if !gpu_authoritative {
        return Err("Phase 3.1 measurement requires live GPU authority".to_string());
    }
    if runtime_tick_calls != scheduler_attempts {
        return Err(format!(
            "Phase 3.1 runtime call/accounting mismatch: calls={runtime_tick_calls}, attempts={scheduler_attempts}"
        ));
    }
    if scheduler_completed_ticks != completed_world_ticks {
        return Err(format!(
            "Phase 3.1 completed tick mismatch: scheduler={scheduler_completed_ticks}, world={completed_world_ticks}"
        ));
    }
    if scheduler_attempts != scheduler_completed_ticks.saturating_add(typed_zero_progress_calls) {
        return Err(format!(
            "Phase 3.1 typed outcome mismatch: attempts={scheduler_attempts}, completed={scheduler_completed_ticks}, zero_progress={typed_zero_progress_calls}"
        ));
    }
    Ok(())
}
