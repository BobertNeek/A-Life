//! Phase 3.1 performance receipt assembly and durable output.

use super::*;

pub(super) fn write_phase31_performance_receipt(
    metrics: &Phase31PerformanceMetricsResource,
    runtime: &crate::GpuLiveBrainRuntime,
    scheduler_final: crate::bevy_shell::ProductionGpuTickPerformanceCounters,
    final_world_tick: u64,
    elapsed: Duration,
    schedule_failed: bool,
    gpu_authoritative: bool,
) -> Result<PathBuf, GameAppShellError> {
    if cfg!(debug_assertions) {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "Phase 3.1 baseline requires an optimized release executable".to_string(),
        });
    }
    let source_head = std::env::var("ALIFE_PHASE31_SOURCE_HEAD").map_err(|_| {
        GameAppShellError::InvalidProductionFrontend {
            message: "Phase 3.1 baseline requires ALIFE_PHASE31_SOURCE_HEAD".to_string(),
        }
    })?;
    if source_head.len() != 40 || !source_head.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "Phase 3.1 source SHA must be a full hexadecimal Git object ID".to_string(),
        });
    }
    let executable_path = std::env::current_exe()?;
    let executable_blake3 = file_blake3_hex(&executable_path)?;
    let runtime_delta = runtime
        .performance_metrics()
        .delta_from(metrics.runtime_baseline.unwrap_or_default());
    let scheduler_before = metrics.scheduler_baseline.unwrap_or(scheduler_final);
    let frames_observed = scheduler_final
        .frames_observed
        .saturating_sub(scheduler_before.frames_observed);
    let scheduler_completed_ticks = scheduler_final
        .completed_ticks
        .saturating_sub(scheduler_before.completed_ticks);
    let scheduler_attempts = scheduler_final
        .scheduler_attempts
        .saturating_sub(scheduler_before.scheduler_attempts);
    let checkpoint_publication_waits = scheduler_final
        .checkpoint_publication_waits
        .saturating_sub(scheduler_before.checkpoint_publication_waits);
    let checkpoint_failed_waits = scheduler_final
        .checkpoint_failed_waits
        .saturating_sub(scheduler_before.checkpoint_failed_waits);
    let deferred_catch_up_ticks = scheduler_final
        .deferred_catch_up_ticks
        .saturating_sub(scheduler_before.deferred_catch_up_ticks);
    let dropped_ticks = scheduler_final
        .catch_up_ticks_dropped
        .saturating_sub(scheduler_before.catch_up_ticks_dropped);
    let completed_world_ticks = final_world_tick.saturating_sub(
        metrics
            .measurement_start_world_tick
            .unwrap_or(final_world_tick),
    );
    let elapsed_seconds = elapsed.as_secs_f64().max(0.001);
    validate_phase31_performance_authority(
        schedule_failed,
        gpu_authoritative,
        runtime_delta.tick_calls,
        scheduler_attempts,
        scheduler_completed_ticks,
        completed_world_ticks,
        checkpoint_publication_waits.saturating_add(checkpoint_failed_waits),
    )
    .map_err(|message| GameAppShellError::InvalidProductionFrontend { message })?;
    let gpu_inference_ns = metrics.gpu_samples.iter().fold(0_u64, |total, sample| {
        total.saturating_add(gpu_timestamp_ns(
            sample.inference_timestamp_ticks,
            sample.timestamp_period_ns_q24,
        ))
    });
    let gpu_plasticity_ns = metrics.gpu_samples.iter().fold(0_u64, |total, sample| {
        total.saturating_add(gpu_timestamp_ns(
            sample.plasticity_timestamp_ticks,
            sample.timestamp_period_ns_q24,
        ))
    });
    let frame_total_ns = metrics
        .frame_ns
        .iter()
        .fold(0_u64, |total, value| total.saturating_add(*value));
    let measured_update_ns = metrics
        .input_cpu_ns
        .saturating_add(metrics.live_gpu_tick_cpu_ns)
        .saturating_add(metrics.authoritative_projection_cpu_ns)
        .saturating_add(metrics.procedural_animation_cpu_ns)
        .saturating_add(metrics.ui_root_readers_cpu_ns);
    let renderer_present_residual_ns = frame_total_ns.saturating_sub(measured_update_ns);
    let measured_preparation_ns = runtime_delta
        .preparation_sleep_eligibility_replay_wall_ns
        .saturating_add(runtime_delta.preparation_grounded_perception_wall_ns)
        .saturating_add(runtime_delta.preparation_episodic_retrieval_wall_ns)
        .saturating_add(runtime_delta.preparation_attention_context_wall_ns)
        .saturating_add(runtime_delta.preparation_topology_concept_wall_ns)
        .saturating_add(runtime_delta.preparation_gpu_upload_wall_ns)
        .saturating_add(runtime_delta.preparation_checkpoint_publication_wall_ns);
    let preparation_residual_ns = runtime_delta
        .perception_sleep_preparation_wall_ns
        .saturating_sub(measured_preparation_ns);
    let sleep_journal_publication_stages = serde_json::json!({
        "current_journal_load_validation_ns": runtime_delta.sleep_journal_current_load_validation_wall_ns,
        "merge_ns": runtime_delta.sleep_journal_merge_wall_ns,
        "sort_ns": runtime_delta.sleep_journal_sort_wall_ns,
        "journal_build_validation_ns": runtime_delta.sleep_journal_build_validation_wall_ns,
        "input_validation_ns": runtime_delta.sleep_journal_input_validation_wall_ns,
        "cas_lock_wait_ns": runtime_delta.sleep_journal_cas_lock_wait_wall_ns,
        "cas_base_reload_ns": runtime_delta.sleep_journal_cas_base_reload_wall_ns,
        "save_encode_ns": runtime_delta.sleep_journal_save_encode_wall_ns,
        "save_artifact_write_ns": runtime_delta.sleep_journal_save_artifact_write_wall_ns,
        "journal_encode_ns": runtime_delta.sleep_journal_encode_wall_ns,
        "journal_artifact_write_ns": runtime_delta.sleep_journal_artifact_write_wall_ns,
        "pointer_build_validation_ns": runtime_delta.sleep_journal_pointer_build_validation_wall_ns,
        "prepared_artifact_reload_validation_ns": runtime_delta.sleep_journal_prepared_reload_validation_wall_ns,
        "manifest_encode_ns": runtime_delta.sleep_journal_manifest_encode_wall_ns,
        "manifest_write_ns": runtime_delta.sleep_journal_manifest_write_wall_ns,
        "manifest_reload_validation_ns": runtime_delta.sleep_journal_manifest_reload_validation_wall_ns,
        "final_journal_reload_validation_ns": runtime_delta.sleep_journal_final_reload_validation_wall_ns,
        "outer_manifest_reload_validation_ns": runtime_delta.sleep_journal_outer_manifest_reload_validation_wall_ns,
        "outer_journal_reload_validation_ns": runtime_delta.sleep_journal_outer_reload_validation_wall_ns,
        "worker_starts": runtime_delta.sleep_journal_worker_starts,
        "worker_completions": runtime_delta.sleep_journal_worker_completions,
        "worker_failures": runtime_delta.sleep_journal_worker_failures,
        "worker_poll_calls": runtime_delta.sleep_journal_worker_poll_calls,
        "worker_poll_ns": runtime_delta.sleep_journal_worker_poll_wall_ns,
        "worker_wall_ns": runtime_delta.sleep_journal_worker_wall_ns,
        "pending_entries_peak": runtime_delta.sleep_journal_pending_entries_peak,
        "update_thread_enqueue_ns": runtime_delta.sleep_journal_update_thread_enqueue_wall_ns
    });
    let persistence_shutdown = serde_json::json!({
        "idle": runtime.persistence_idle_for_shutdown(),
        "failed": runtime.persistence_failed_for_shutdown(),
        "checkpoint": runtime.exact_checkpoint_performance_state(),
        "outstanding": runtime.persistence_shutdown_diagnostics()
    });
    let mut receipt = serde_json::json!({
        "schema": PHASE31_PERFORMANCE_SCHEMA,
        "schema_version": PHASE31_PERFORMANCE_SCHEMA_VERSION,
        "source_head": source_head,
        "build": {
            "mode": if cfg!(debug_assertions) { "debug" } else { "release" },
            "debug_assertions": cfg!(debug_assertions),
            "optimized_release": !cfg!(debug_assertions),
            "executable_path": executable_path,
            "executable_blake3": executable_blake3
        },
        "profile": metrics.profile,
        "population": metrics.population,
        "resolution": metrics.resolution,
        "backend": metrics.backend,
        "adapter": metrics.adapter,
        "measurement_seconds": elapsed_seconds,
        "world_tick": {
            "start": metrics.measurement_start_world_tick,
            "end": final_world_tick
        },
        "frame": duration_summary(&metrics.frame_ns),
        "slow_frames": {
            "threshold_ms": PHASE31_SLOW_FRAME_THRESHOLD_NS as f64 / 1_000_000.0,
            "total_count": metrics.slow_frame_count,
            "retained_worst_count": metrics.slow_frames.len(),
            "ranked_worst_first": metrics.slow_frames
        },
        "simulation": {
            "configured_tps": scheduler_final.fixed_tick_hz,
            "achieved_tps": completed_world_ticks as f64 / elapsed_seconds,
            "completed_world_ticks": completed_world_ticks,
            "scheduler_completed_ticks": scheduler_completed_ticks,
            "scheduler_attempts": scheduler_attempts,
            "scheduler_attempts_per_second": scheduler_attempts as f64 / elapsed_seconds,
            "zero_progress_calls_by_reason": {
                "checkpoint_publication_pending": checkpoint_publication_waits,
                "checkpoint_failed": checkpoint_failed_waits
            },
            "checkpoint_polls": runtime_delta.exact_checkpoint_poll_calls,
            "deferred_catch_up_ticks": deferred_catch_up_ticks,
            "deferred_debt_micros_at_end": scheduler_final.deferred_debt_micros,
            "catch_up_ticks_dropped": dropped_ticks,
            "scheduler_frames_observed": frames_observed,
            "runtime_tick_calls": runtime_delta.tick_calls,
            "runtime_tick_wall_ns": runtime_delta.tick_wall_ns
        },
        "internal_tick_stages": {
            "tick_preamble_ns": runtime_delta.tick_preamble_wall_ns,
            "perception_sleep_preparation_ns": runtime_delta.perception_sleep_preparation_wall_ns,
            "sleep_promotion_ns": runtime_delta.sleep_promotion_wall_ns,
            "inference_transaction_ns": runtime_delta.inference_transaction_wall_ns,
            "selection_prepare_ns": runtime_delta.selection_prepare_wall_ns,
            "seal_world_body_biochemistry_ns": runtime_delta.seal_world_body_biochemistry_wall_ns,
            "sealed_commit_total_ns": runtime_delta.sealed_commit_total_wall_ns,
            "learning_transaction_ns": runtime_delta.learning_transaction_wall_ns,
            "sidecar_memory_ns": runtime_delta.sidecar_memory_wall_ns,
            "sidecar_topology_ns": runtime_delta.sidecar_topology_wall_ns,
            "cognitive_authority_seal_ns": runtime_delta.cognitive_authority_seal_wall_ns,
            "world_authority_advance_ns": runtime_delta.world_authority_advance_wall_ns,
            "resident_synchronize_ns": runtime_delta.resident_synchronize_wall_ns,
            "passive_observation_ns": runtime_delta.passive_observation_wall_ns,
            "population_reconcile_ns": runtime_delta.population_reconcile_wall_ns,
            "sleep_persistence_ns": runtime_delta.sleep_persistence_wall_ns
        },
        "preparation_substages": {
            "sleep_eligibility_replay_ns": runtime_delta.preparation_sleep_eligibility_replay_wall_ns,
            "sleep_phase_data_ns": runtime_delta.preparation_sleep_phase_data_wall_ns,
            "sleep_replay_progress_ns": runtime_delta.preparation_sleep_replay_progress_wall_ns,
            "sleep_consolidation_ns": runtime_delta.preparation_sleep_consolidation_wall_ns,
            "sleep_scheduler_other_ns": runtime_delta
                .preparation_sleep_eligibility_replay_wall_ns
                .saturating_sub(
                    runtime_delta
                        .preparation_sleep_phase_data_wall_ns
                        .saturating_add(runtime_delta.preparation_sleep_replay_progress_wall_ns)
                        .saturating_add(runtime_delta.preparation_sleep_consolidation_wall_ns)
                ),
            "grounded_perception_ns": runtime_delta.preparation_grounded_perception_wall_ns,
            "episodic_retrieval_ns": runtime_delta.preparation_episodic_retrieval_wall_ns,
            "attention_context_ns": runtime_delta.preparation_attention_context_wall_ns,
            "topology_concept_ns": runtime_delta.preparation_topology_concept_wall_ns,
            "gpu_upload_preparation_ns": runtime_delta.preparation_gpu_upload_wall_ns,
            "checkpoint_publication_preparation_ns": runtime_delta.preparation_checkpoint_publication_wall_ns,
            "other_and_instrumentation_residual_ns": preparation_residual_ns
        },
        "transactional_rollback_clone": {
            "calls": runtime_delta.rollback_clone_calls,
            "world_clone_ns": runtime_delta.rollback_world_clone_wall_ns,
            "residents_clone_ns": runtime_delta.rollback_residents_clone_wall_ns,
            "resident_rows": runtime_delta.rollback_resident_rows,
            "world_object_rows": runtime_delta.rollback_world_object_rows,
            "successful_progress_calls": runtime_delta.rollback_clone_progress_calls,
            "zero_progress_calls": runtime_delta.rollback_clone_zero_progress_calls
        },
        "cpu_stages": {
            "input_ns": metrics.input_cpu_ns,
            "live_gpu_tick_ns": metrics.live_gpu_tick_cpu_ns,
            "authoritative_projection_ns": metrics.authoritative_projection_cpu_ns,
            "procedural_animation_ns": metrics.procedural_animation_cpu_ns,
            "ui_root_readers_ns": metrics.ui_root_readers_cpu_ns,
            "renderer_present_and_uninstrumented_residual_ns": renderer_present_residual_ns
        },
        "gpu_stages": {
            "timestamp_samples": metrics.gpu_samples.len(),
            "inference_ns": gpu_inference_ns,
            "plasticity_ns": gpu_plasticity_ns
        },
        "blocking_transactions": {
            "count": runtime_delta.inference_batches
                .saturating_add(runtime_delta.learning_batches)
                .saturating_add(runtime_delta.ordinary_snapshot_calls),
            "inference_batch_wall_ns": runtime_delta.inference_transaction_wall_ns,
            "learning_batch_wall_ns": runtime_delta.learning_transaction_wall_ns,
            "ordinary_snapshot_poll_wait_ns": runtime_delta.ordinary_snapshot_poll_wait_ns,
            "ordinary_snapshot_map_receive_wait_ns": runtime_delta.ordinary_snapshot_map_receive_wait_ns
        },
        "readback": {
            "selection_calls": runtime_delta.selection_readback_calls,
            "selection_bytes": runtime_delta.selection_readback_bytes,
            "learning_calls": runtime_delta.learning_readback_calls,
            "learning_bytes": runtime_delta.learning_readback_bytes,
            "ordinary_full_snapshot_calls": runtime_delta.ordinary_snapshot_calls,
            "ordinary_full_snapshot_bytes": runtime_delta.ordinary_snapshot_bytes
        },
        "ordinary_full_snapshot": {
            "calls": runtime_delta.ordinary_snapshot_calls,
            "bytes": runtime_delta.ordinary_snapshot_bytes,
            "wall_ns": runtime_delta.ordinary_snapshot_wall_ns,
            "poll_wait_ns": runtime_delta.ordinary_snapshot_poll_wait_ns,
            "map_receive_wait_ns": runtime_delta.ordinary_snapshot_map_receive_wait_ns,
            "calls_per_runtime_tick": if runtime_delta.tick_calls == 0 {
                0.0
            } else {
                runtime_delta.ordinary_snapshot_calls as f64 / runtime_delta.tick_calls as f64
            }
        },
        "state_reference_hash": {
            "calls": runtime_delta.state_reference_hash_calls,
            "resident_json_bytes": runtime_delta.resident_json_bytes,
            "topology_json_bytes": runtime_delta.topology_json_bytes,
            "wall_ns": runtime_delta.state_reference_hash_wall_ns
        },
        "dispatch_batching": {
            "inference_batches": runtime_delta.inference_batches,
            "inference_rows": runtime_delta.inference_rows,
            "mean_inference_rows_per_batch": if runtime_delta.inference_batches == 0 {
                0.0
            } else {
                runtime_delta.inference_rows as f64 / runtime_delta.inference_batches as f64
            },
            "learning_batches": runtime_delta.learning_batches,
            "learning_rows": runtime_delta.learning_rows,
            "mean_learning_rows_per_batch": if runtime_delta.learning_batches == 0 {
                0.0
            } else {
                runtime_delta.learning_rows as f64 / runtime_delta.learning_batches as f64
            }
        },
        "ui": {
            "updates": metrics.ui_updates,
            "cadence_hz": metrics.ui_updates as f64 / elapsed_seconds
        },
        "checkpoint_activity": {
            "capture_calls": runtime_delta.checkpoint_capture_calls,
            "capture_wall_ns": runtime_delta.checkpoint_capture_wall_ns,
            "full_snapshot_calls": runtime_delta.checkpoint_snapshot_calls,
            "full_snapshot_bytes": runtime_delta.checkpoint_snapshot_bytes,
            "poll_wait_ns": runtime_delta.checkpoint_snapshot_poll_wait_ns,
            "map_receive_wait_ns": runtime_delta.checkpoint_snapshot_map_receive_wait_ns,
            "asynchronous_poll_calls": runtime_delta.exact_checkpoint_poll_calls,
            "asynchronous_poll_cpu_ns": runtime_delta.exact_checkpoint_poll_wall_ns,
            "asynchronous_transactions_started": runtime_delta.exact_checkpoint_transactions_started,
            "asynchronous_transactions_completed": runtime_delta.exact_checkpoint_transactions_completed,
            "asynchronous_transaction_wall_ns": runtime_delta.exact_checkpoint_transaction_wall_ns
        },
        "sleep_durable_activity": {
            "boundary_calls": runtime_delta.sleep_persistence_calls,
            "capture_calls": runtime_delta.sleep_checkpoint_capture_calls,
            "exact_neural_capture_organisms": runtime_delta.sleep_exact_neural_capture_organisms,
            "compact_journal_organisms": runtime_delta.sleep_compact_journal_organisms,
            "capture_wall_ns": runtime_delta.sleep_checkpoint_capture_wall_ns,
            "capture_readback_calls": runtime_delta.sleep_checkpoint_readback_calls,
            "capture_readback_bytes": runtime_delta.sleep_checkpoint_readback_bytes,
            "capture_readback_poll_wait_ns": runtime_delta.sleep_checkpoint_readback_poll_wait_ns,
            "capture_readback_map_receive_wait_ns": runtime_delta.sleep_checkpoint_readback_map_receive_wait_ns,
            "checkpoint_publish_calls": runtime_delta.sleep_checkpoint_publish_calls,
            "checkpoint_publish_wall_ns": runtime_delta.sleep_checkpoint_publish_wall_ns,
            "promotion_calls": runtime_delta.sleep_promotion_calls,
            "promotion_publish_calls": runtime_delta.sleep_promotion_publish_calls,
            "promotion_publish_wall_ns": runtime_delta.sleep_promotion_publish_wall_ns
        },
        "sleep_journal_publication_stages": sleep_journal_publication_stages
    });
    receipt
        .as_object_mut()
        .expect("performance receipt root is an object")
        .insert("persistence_shutdown".to_string(), persistence_shutdown);
    let root = PathBuf::from(PHASE31_PERFORMANCE_ARTIFACT_DIR);
    fs::create_dir_all(&root)?;
    let path = root.join(format!(
        "phase31-before-release-population-{}.json",
        metrics.population
    ));
    fs::write(&path, serde_json::to_string_pretty(&receipt)?)?;
    Ok(path)
}
