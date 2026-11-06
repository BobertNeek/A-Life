use super::*;

use std::{
    cell::Cell,
    fs,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "alife-runtime-profile-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn record(output: &Path, count: u64, succeeded: bool) {
    let mut profile = RuntimeProfile::with_enabled(true, output, 7);
    profile.begin(100, 1, 2, GpuLivePerformanceMetrics::default);
    profile.complete(100 + count, succeeded, || GpuLivePerformanceMetrics {
        tick_calls: count,
        inference_rows: count,
        tick_wall_ns: count.saturating_mul(1_000_000),
        ..Default::default()
    });
}

#[test]
fn continuous_runtime_profile_disabled_does_no_capture_or_io() {
    let t = Temp::new();
    let missing = t.0.join("not-created");
    {
        let mut p = RuntimeProfile::with_enabled(false, &missing, 7);
        p.begin(0, 1, 2, || panic!("disabled getter"));
        p.complete(1, false, || panic!("disabled getter"));
    }
    assert!(!missing.exists());
    assert_eq!(fs::read_dir(&t.0).unwrap().count(), 0);
}

#[test]
fn continuous_runtime_profile_aggregates_two_samples_for_large_runs() {
    for count in [1, 1_000_000, 10_000_000] {
        let t = Temp::new();
        let calls = Cell::new(0);
        {
            let mut p = RuntimeProfile::with_enabled(true, &t.0, 7);
            let initial = || {
                calls.set(calls.get() + 1);
                GpuLivePerformanceMetrics::default()
            };
            p.begin(100, 1, 2, initial);
            // No window callback exists in the profiler. Progress is a synthetic
            // input, not a native trajectory or a training workload.
            p.complete(100 + count, true, || {
                calls.set(calls.get() + 1);
                GpuLivePerformanceMetrics {
                    tick_calls: count,
                    ..Default::default()
                }
            });
        }
        let bytes = fs::read(t.0.join(PROFILE_FILE)).unwrap();
        assert!(bytes.len() <= MAX_PROFILE_BYTES);
        assert_eq!(calls.get(), 2);
        let r: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(r["session_nonce"], 7);
        assert_eq!(r["measurement_status"], "complete");
        assert_eq!(r["initial"]["cumulative"]["tick_calls"], 0);
        assert_eq!(r["final"]["delta"]["tick_calls"], count);
        assert_eq!(fs::read_dir(&t.0).unwrap().count(), 1);
    }
}

#[test]
fn continuous_runtime_profile_serialization_and_saturated_counts_are_bounded() {
    assert!(bounded_json(&json!({"oversized": "x".repeat(MAX_PROFILE_BYTES)})).is_err());
    let t = Temp::new();
    let mut all_max = serde_json::to_value(GpuLivePerformanceMetrics::default()).unwrap();
    for value in all_max.as_object_mut().unwrap().values_mut() {
        *value = json!(u64::MAX);
    }
    {
        let mut observer = RuntimeMetrics::new(10);
        let initial = observer.observe_snapshot(10, all_max.clone());
        let final_snapshot = observer.observe_snapshot(10, all_max);
        let mut p = RuntimeProfile::with_enabled(true, &t.0, 7);
        p.0.as_mut().unwrap().measured = Some(Measurement {
            observer,
            organism_id: 1,
            world_seed: 2,
            initial,
            final_snapshot: Some(final_snapshot),
            succeeded: false,
        });
    }
    let bytes = fs::read(t.0.join(PROFILE_FILE)).unwrap();
    assert!(bytes.len() <= MAX_PROFILE_BYTES);
    let r: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(r["measurement_status"], "error");
    assert_eq!(r["final"]["delta_status"], "partial");
    assert!(r["final"]["delta"]["tick_calls"].is_null());
    assert_eq!(
        r["final"]["cumulative"]["tick_calls"].as_u64(),
        Some(u64::MAX)
    );
}

#[test]
fn continuous_runtime_profile_does_not_write_before_final_measurement() {
    let t = Temp::new();
    {
        let mut p = RuntimeProfile::with_enabled(true, &t.0, 7);
        p.begin(1, 1, 2, GpuLivePerformanceMetrics::default);
    }
    assert!(!t.0.join(PROFILE_FILE).exists());
    let missing = t.0.join("missing");
    record(&missing, 1, false);
    assert!(!missing.exists());
}

#[test]
fn continuous_runtime_profile_refuses_predecessors_and_keeps_original_error() {
    let t = Temp::new();
    let path = t.0.join(PROFILE_FILE);
    fs::write(&path, b"existing diagnostic").unwrap();
    let original: Box<dyn std::error::Error> = io::Error::other("original producer error").into();
    let address = (&*original as *const dyn std::error::Error).cast::<()>();
    let result: crate::foundation_training_cycle::Result<()> = {
        let mut p = RuntimeProfile::with_enabled(true, &t.0, 7);
        p.begin(1, 1, 2, GpuLivePerformanceMetrics::default);
        p.complete(2, false, GpuLivePerformanceMetrics::default);
        Err(original)
    };
    let error = result.unwrap_err();
    assert_eq!(
        (&*error as *const dyn std::error::Error).cast::<()>(),
        address
    );
    assert_eq!(error.to_string(), "original producer error");
    assert_eq!(fs::read(path).unwrap(), b"existing diagnostic");
}

#[test]
fn continuous_runtime_profile_uses_released_recovery_capacity_and_budget_accounting() {
    let t = Temp::new();
    let budget = alife_storage::OutputBudget::register(&t.0, MAX_PROFILE_BYTES as u64).unwrap();
    {
        let mut p = RuntimeProfile::with_enabled(true, &t.0, 7);
        let _recovery = budget.reserve(MAX_PROFILE_BYTES as u64).unwrap();
        p.begin(1, 1, 2, GpuLivePerformanceMetrics::default);
        p.complete(2, true, GpuLivePerformanceMetrics::default);
        assert!(!t.0.join(PROFILE_FILE).exists());
    }
    let length = fs::metadata(t.0.join(PROFILE_FILE)).unwrap().len();
    assert!(length > 0 && length <= MAX_PROFILE_BYTES as u64);
    assert_eq!(budget.usage().unwrap().used, length);
}

#[test]
#[ignore = "isolated local component/IO measurement; not a training benchmark"]
fn continuous_runtime_profile_component_cost() {
    use std::{hint::black_box, time::Instant};
    let t = Temp::new();
    let _budget = alife_storage::OutputBudget::register(&t.0, 256 * 1024 * 1024).unwrap();
    let mut results = Vec::new();
    for count in [0, 1_000_000] {
        let sample = t.0.join(format!("sample-{count}"));
        fs::create_dir(&sample).unwrap();
        record(&sample, count, true);
        let sample_bytes = fs::read(sample.join(PROFILE_FILE)).unwrap();
        let mut baseline = Vec::new();
        let mut enabled = Vec::new();
        let run = |on: bool, dirs: &[PathBuf]| {
            let start = Instant::now();
            for dir in dirs {
                let mut p = RuntimeProfile::with_enabled(black_box(on), black_box(dir), 7);
                p.begin(100, 1, 2, GpuLivePerformanceMetrics::default);
                p.complete(100 + count, true, || GpuLivePerformanceMetrics {
                    tick_calls: count,
                    inference_rows: count,
                    ..Default::default()
                });
                black_box(&p);
                drop(p);
            }
            start.elapsed().as_nanos()
        };
        for pair in 0..8 {
            let dirs: Vec<_> = (0..100)
                .map(|i| {
                    let p = t.0.join(format!("{count}-{pair}-{i}"));
                    fs::create_dir(&p).unwrap();
                    p
                })
                .collect();
            let (off, on) = if pair % 2 == 0 {
                (run(false, &dirs), run(true, &dirs))
            } else {
                let on = run(true, &dirs);
                (run(false, &dirs), on)
            };
            if pair != 0 {
                baseline.push(off);
                enabled.push(on);
            }
        }
        results.push(json!({"synthetic_progress":count,"calls_per_pair_arm":100,
            "disabled_ns":baseline,"enabled_ns":enabled,"report_bytes":sample_bytes.len(),
            "sample":serde_json::from_slice::<Value>(&sample_bytes).unwrap()}));
    }
    println!(
        "ALIFE_RUNTIME_PROFILE_COMPONENT={}",
        json!({"profile":"Cargo native test profile; app opt-level=2",
        "method":"one warmup pair and seven alternating retained pairs, 100 profiles per arm; directory creation outside timing",
        "scope":"profile construction, two synthetic counter captures, serialization, exclusive file write and file sync; no world, GPU, training or checkpoints",
        "limits":"local disposable filesystem only; no CPU affinity/dedicated-host isolation; not an app throughput measurement",
        "rows":results})
    );
}

#[test]
fn continuous_runtime_metrics_keep_initial_and_window_final_scopes_exact() {
    let mut observer = RuntimeMetrics::new(100);
    let mut metrics = GpuLivePerformanceMetrics {
        tick_calls: 7,
        inference_rows: 3,
        sleep_journal_pending_entries_peak: 9,
        ..Default::default()
    };
    let initial = observer.observe(100, metrics);
    assert_eq!(initial["delta_status"], "initial_snapshot");
    assert!(initial["delta"].is_null());
    assert!(initial["committed_world_ticks"].is_null());
    assert_eq!(initial["cumulative"]["tick_calls"], 7);
    metrics.tick_calls = 20;
    metrics.tick_wall_ns = 123_456_789;
    metrics.inference_rows = 13;
    let window = observer.observe(105, metrics);
    assert_eq!(window["delta_status"], "complete");
    assert_eq!(window["interval_start_world_tick"], 100);
    assert_eq!(window["committed_world_ticks"], 5);
    assert_eq!(window["delta"]["tick_calls"], 13);
    assert_eq!(window["delta"]["inference_rows"], 10);
    assert_eq!(window["delta"]["tick_wall_ns"], 123_456_789);
    assert!(window["delta"].get(PEAK).is_none());
    assert_eq!(window["cumulative"][PEAK], 9);
    // A final unsuccessful poll advances counters but not world/active rows.
    metrics.tick_calls += 1;
    metrics.rollback_clone_zero_progress_calls = 1;
    let final_receipt = observer.observe(105, metrics);
    assert_eq!(final_receipt["committed_world_ticks"], 0);
    assert_eq!(final_receipt["delta"]["tick_calls"], 1);
    assert_eq!(final_receipt["delta"]["inference_rows"], 0);
    assert_eq!(
        final_receipt["delta"]["rollback_clone_zero_progress_calls"],
        1
    );
    assert_eq!(final_receipt["cumulative"]["tick_calls"], 21);
    assert_eq!(final_receipt["cumulative"]["inference_rows"], 13);
    assert_eq!(
        final_receipt["cumulative"],
        serde_json::to_value(metrics).unwrap()
    );
    assert_eq!(initial["cumulative"]["tick_calls"], 7);
}

#[test]
fn continuous_runtime_metrics_preserve_missing_zero_and_wire_integer_values() {
    let legacy = json!({"kind":"continuous-update-v1","trained_rows":512});
    assert!(legacy.get("runtime_performance").is_none());
    let mut observer = RuntimeMetrics::new(0);
    observer.observe(0, GpuLivePerformanceMetrics::default());
    let mut event = legacy.clone();
    event["runtime_performance"] = observer.observe(
        0,
        GpuLivePerformanceMetrics {
            tick_wall_ns: u64::MAX - 1,
            ..Default::default()
        },
    );
    let bytes = serde_json::to_vec(&event).unwrap();
    let mut decoded: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded, event);
    assert_eq!(decoded["runtime_performance"]["delta"]["tick_calls"], 0);
    assert_eq!(
        decoded["runtime_performance"]["cumulative"]["tick_wall_ns"].as_u64(),
        Some(u64::MAX - 1)
    );
    decoded
        .as_object_mut()
        .unwrap()
        .remove("runtime_performance");
    assert_eq!(decoded, legacy);
}

#[test]
fn continuous_runtime_metrics_do_not_turn_reset_wrap_or_saturation_into_zero() {
    let mut observer = RuntimeMetrics::new(4);
    observer.observe(
        4,
        GpuLivePerformanceMetrics {
            tick_calls: u64::MAX - 1,
            inference_rows: 12,
            ..Default::default()
        },
    );
    let receipt = observer.observe(
        5,
        GpuLivePerformanceMetrics {
            tick_calls: 1,
            inference_rows: 0,
            tick_wall_ns: u64::MAX,
            ..Default::default()
        },
    );
    assert_eq!(receipt["delta_status"], "partial");
    for field in ["tick_calls", "inference_rows", "tick_wall_ns"] {
        assert!(receipt["delta"][field].is_null());
        assert!(receipt["delta_unavailable_fields"][field].is_string());
    }
    assert_eq!(
        receipt["delta_unavailable_fields"]["tick_calls"],
        "counter_decreased_reset_or_wrap"
    );
    assert_eq!(
        receipt["delta_unavailable_fields"]["tick_wall_ns"],
        "saturated_counter"
    );
    assert_eq!(receipt["delta"]["learning_rows"], 0);
    assert_eq!(receipt["cumulative"]["tick_calls"], 1);
    assert_eq!(
        receipt["cumulative"]["tick_wall_ns"].as_u64(),
        Some(u64::MAX)
    );
}

#[test]
fn continuous_runtime_metrics_mark_schema_and_clock_changes_unavailable() {
    let mut observer = RuntimeMetrics::new(10);
    observer.observe_snapshot(10, json!({"tick_calls":1}));
    let missing = observer.observe_snapshot(11, json!({"inference_rows":0}));
    assert_eq!(missing["delta_status"], "counter_schema_changed");
    assert!(missing["delta"].is_null());
    assert_eq!(missing["cumulative"]["inference_rows"], 0);
    let invalid = observer.observe_snapshot(12, json!({"inference_rows":null}));
    assert_eq!(invalid["delta_status"], "partial");
    assert_eq!(
        invalid["delta_unavailable_fields"]["inference_rows"],
        "non_u64_counter"
    );
    let rewind = observer.observe_snapshot(9, json!({"inference_rows":0}));
    assert_eq!(rewind["delta_status"], "world_tick_regressed");
    assert!(rewind["delta"].is_null());
    assert!(rewind["committed_world_ticks"].is_null());
}
