//! Boundary-only export of existing runtime counters; never a pacing controller.
use crate::gpu_live_runtime::GpuLivePerformanceMetrics;
use serde_json::{json, Map, Value};

// This is a high-water mark, not an additive work/timing counter.
const PEAK: &str = "sleep_journal_pending_entries_peak";

pub(super) struct RuntimeMetrics {
    enabled_at_world_tick: u64,
    previous: Option<(u64, Value)>,
}

impl RuntimeMetrics {
    pub(super) fn new(enabled_at_world_tick: u64) -> Self {
        Self {
            enabled_at_world_tick,
            previous: None,
        }
    }

    pub(super) fn observe(&mut self, tick: u64, metrics: GpuLivePerformanceMetrics) -> Value {
        // The current type contains only u64 fields. Keep diagnostics nonfatal
        // if that serialization contract ever changes.
        match serde_json::to_value(metrics) {
            Ok(snapshot) => self.observe_snapshot(tick, snapshot),
            Err(_) => json!({"schema":1,"delta_status":"serialization_error",
                "cumulative":null,"delta":null}),
        }
    }

    fn observe_snapshot(&mut self, tick: u64, snapshot: Value) -> Value {
        let mut unavailable = Map::new();
        let mut delta = Map::new();
        let interval_start = self.previous.as_ref().map(|(tick, _)| *tick);
        let status = match &self.previous {
            None => "initial_snapshot",
            Some((before_tick, _)) if tick < *before_tick => "world_tick_regressed",
            Some((_, before)) => match (snapshot.as_object(), before.as_object()) {
                (Some(after), Some(before)) if after.keys().eq(before.keys()) => {
                    for (name, value) in after.iter().filter(|(name, _)| name.as_str() != PEAK) {
                        let difference = match (value.as_u64(), before[name].as_u64()) {
                            (Some(end), Some(start)) if end == u64::MAX || start == u64::MAX => {
                                Err("saturated_counter")
                            }
                            (Some(end), Some(start)) => end
                                .checked_sub(start)
                                .ok_or("counter_decreased_reset_or_wrap"),
                            _ => Err("non_u64_counter"),
                        };
                        match difference {
                            Ok(value) => {
                                delta.insert(name.clone(), json!(value));
                            }
                            Err(reason) => {
                                delta.insert(name.clone(), Value::Null);
                                unavailable.insert(name.clone(), json!(reason));
                            }
                        }
                    }
                    if unavailable.is_empty() {
                        "complete"
                    } else {
                        "partial"
                    }
                }
                _ => "counter_schema_changed",
            },
        };
        let valid_interval = matches!(status, "complete" | "partial");
        let receipt = json!({"schema":1,"delta_status":status,
            "measurement_enabled_at_world_tick":self.enabled_at_world_tick,
            "interval_start_world_tick":interval_start,"world_tick":tick,
            "committed_world_ticks":interval_start.and_then(|start| tick.checked_sub(start)),
            "cumulative_origin":"existing runtime-instance counters; initial snapshot is in continuous-started-v1",
            "cumulative":snapshot,"delta":valid_interval.then_some(delta),
            "delta_unavailable_fields":unavailable,"delta_excludes":[PEAK],
            "time_basis":"host wall nanoseconds around existing calls; inclusive and nested, not additive GPU kernel time"});
        self.previous = Some((tick, snapshot));
        receipt
    }
}

/// Emit once on an error boundary, including unsuccessful ticks and worker
/// polling since the last observation. Diagnostic delivery cannot replace the
/// original operation error. Setup before measurement has no invented timings.
pub(super) fn report_failure<T>(
    result: super::Result<T>,
    receipt: impl FnOnce() -> Value,
    emit: &mut impl FnMut(Value) -> super::Result<()>,
) -> super::Result<T> {
    if result.is_err() {
        let _ = emit(receipt());
    }
    result
}

#[cfg(test)]
mod tests;
