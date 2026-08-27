pub(crate) fn validate_phase31_performance_authority(
    schedule_failed: bool,
    gpu_authoritative: bool,
    runtime_tick_calls: u64,
    ticks_executed: u64,
) -> Result<(), String> {
    if schedule_failed {
        return Err("Phase 3.1 measurement rejected a failed production tick schedule".to_string());
    }
    if !gpu_authoritative {
        return Err("Phase 3.1 measurement requires live GPU authority".to_string());
    }
    if runtime_tick_calls != ticks_executed {
        return Err(format!(
            "Phase 3.1 runtime call/accounting mismatch: calls={runtime_tick_calls}, executed={ticks_executed}"
        ));
    }
    Ok(())
}
