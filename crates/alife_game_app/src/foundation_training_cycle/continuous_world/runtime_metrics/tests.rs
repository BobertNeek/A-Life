use super::*;

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

#[test]
fn continuous_runtime_metrics_error_delivery_preserves_original_failure_and_counts() {
    #[derive(Debug)]
    struct Original;
    impl std::fmt::Display for Original {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("original")
        }
    }
    impl std::error::Error for Original {}
    let mut observer = RuntimeMetrics::new(10);
    observer.observe(10, GpuLivePerformanceMetrics::default());
    let mut emitted = Vec::new();
    let original: Box<dyn std::error::Error> = Box::new(Original);
    let pointer = (&*original as *const dyn std::error::Error).cast::<()>();
    let result: super::super::Result<()> = report_failure(
        Err(original),
        || {
            observer.observe(
                10,
                GpuLivePerformanceMetrics {
                    tick_calls: 1,
                    tick_wall_ns: 55,
                    rollback_clone_zero_progress_calls: 1,
                    ..Default::default()
                },
            )
        },
        &mut |receipt| {
            emitted.push(receipt);
            Err("diagnostic delivery failed".into())
        },
    );
    let error = result.unwrap_err();
    assert_eq!(
        (&*error as *const dyn std::error::Error).cast::<()>(),
        pointer
    );
    assert!(error.is::<Original>());
    assert_eq!(emitted.len(), 1);
    assert_eq!(emitted[0]["delta"]["tick_calls"], 1);
    assert_eq!(emitted[0]["delta"]["tick_wall_ns"], 55);
    assert_eq!(emitted[0]["committed_world_ticks"], 0);
    assert!(report_failure(
        Ok(17),
        || panic!("success must not emit failure"),
        &mut |_| panic!("unexpected emission")
    )
    .is_ok());
}
