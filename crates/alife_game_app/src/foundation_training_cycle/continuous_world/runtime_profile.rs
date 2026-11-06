//! Optional, fixed-size aggregate of existing counters. No native log output.
use crate::gpu_live_runtime::GpuLivePerformanceMetrics;
use serde_json::{json, Map, Value};
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
};

pub(crate) const MAX_PROFILE_BYTES: usize = 64 * 1024;
pub(crate) const PROFILE_FILE: &str = "runtime-profile.json";

/// Declared after the owning output budget but before validation/recovery
/// guards. Its drop writes only after core output has been prepared, those
/// guards are released, and the budget's recovery headroom is available again.
pub(crate) struct RuntimeProfile(Option<Profile>);

struct Profile {
    output: PathBuf,
    session_nonce: u64,
    measured: Option<Measurement>,
}

struct Measurement {
    observer: RuntimeMetrics,
    organism_id: u64,
    world_seed: u64,
    initial: Value,
    final_snapshot: Option<Value>,
    succeeded: bool,
}

impl RuntimeProfile {
    pub(crate) fn from_environment(output: &Path, session_nonce: u64) -> Self {
        // Match CycleStageTiming: presence enables profiling, including "0".
        Self::with_enabled(
            std::env::var_os("ALIFE_FOUNDATION_PROFILE").is_some(),
            output,
            session_nonce,
        )
    }

    fn with_enabled(enabled: bool, output: &Path, session_nonce: u64) -> Self {
        Self(enabled.then(|| Profile {
            output: output.to_owned(),
            session_nonce,
            measured: None,
        }))
    }

    pub(crate) fn begin(
        &mut self,
        tick: u64,
        organism_id: u64,
        world_seed: u64,
        metrics: impl FnOnce() -> GpuLivePerformanceMetrics,
    ) {
        let Some(profile) = &mut self.0 else { return };
        let mut observer = RuntimeMetrics::new(tick);
        let initial = observer.observe(tick, metrics());
        profile.measured = Some(Measurement {
            observer,
            organism_id,
            world_seed,
            initial,
            final_snapshot: None,
            succeeded: false,
        });
    }

    pub(crate) fn complete(
        &mut self,
        tick: u64,
        succeeded: bool,
        metrics: impl FnOnce() -> GpuLivePerformanceMetrics,
    ) {
        let Some(measured) = self.0.as_mut().and_then(|p| p.measured.as_mut()) else {
            return;
        };
        measured.final_snapshot = Some(measured.observer.observe(tick, metrics()));
        measured.succeeded = succeeded;
    }
}

impl Profile {
    fn encoded(&self) -> io::Result<Option<Vec<u8>>> {
        let Some(m) = &self.measured else {
            return Ok(None);
        };
        let Some(final_snapshot) = &m.final_snapshot else {
            return Ok(None);
        };
        let report = json!({"schema":1,"kind":"continuous-runtime-profile-v1",
            "session_nonce":self.session_nonce,"organism_id":m.organism_id,"source_world_seed":m.world_seed,
            "measurement_status":if m.succeeded {"complete"} else {"error"},
            "scope":"runtime after restore/adoption through final runtime boundary; excludes owner publication and this diagnostic write",
            "initial":m.initial,"final":final_snapshot});
        bounded_json(&report).map(Some)
    }
}

fn bounded_json(value: &Value) -> io::Result<Vec<u8>> {
    struct Buffer(Vec<u8>);
    impl Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if bytes.len() > MAX_PROFILE_BYTES.saturating_sub(self.0.len()) {
                return Err(io::Error::other("optional runtime profile byte limit"));
            }
            self.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut buffer = Buffer(Vec::new());
    serde_json::to_writer(&mut buffer, value).map_err(io::Error::other)?;
    Ok(buffer.0)
}

impl Drop for RuntimeProfile {
    fn drop(&mut self) {
        if std::thread::panicking() {
            return;
        }
        let Some(profile) = &self.0 else { return };
        let Ok(Some(bytes)) = profile.encoded() else {
            return;
        };
        // Only the already-owned producer directory is used. Never recreate a
        // missing parent, truncate a predecessor, or replace a training error.
        let _ = (|| -> io::Result<()> {
            let path = profile.output.join(PROFILE_FILE);
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            let mut writer = alife_storage::writer(&path, file)?;
            writer.write_all(&bytes)?;
            writer.flush()?;
            writer.sync_all()
        })();
    }
}

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
            "cumulative_origin":"existing runtime-instance counters; initial snapshot is in this aggregate profile",
            "cumulative":snapshot,"delta":valid_interval.then_some(delta),
            "delta_unavailable_fields":unavailable,"delta_excludes":[PEAK],
            "time_basis":"host wall nanoseconds around existing calls; inclusive and nested, not additive GPU kernel time"});
        self.previous = Some((tick, snapshot));
        receipt
    }
}

#[cfg(test)]
mod tests;
