//! Observations of the existing causal collection, never a pacing controller.
use super::Result;
use alife_gpu_backend::GpuTrainingCaptureMetrics;

/// Counter deltas exclude setup/initial checkpointing and final save/PPO. Each
/// committed world tick runs ordinary physics, biology and cognition. Waking
/// captures and unsuccessful no-progress calls are separate observations.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FoundationCausalCollectionMetrics {
    pub schema: u16,
    pub control_start_world_tick: u64,
    pub control_end_world_tick: u64,
    pub committed_world_ticks: u64,
    pub simulation_seconds: f64,
    /// Wall interval from immediately before the first control tick through the
    /// end of collection, including intervening replay/trainer preparation.
    pub collection_wall_seconds: f64,
    pub simulation_seconds_per_wall_second: Option<f64>,
    /// Includes the actual final bootstrap capture when one is collected.
    pub waking_decision_records: usize,
    pub no_progress_calls: u64,
    pub capture: FoundationCausalCaptureMetrics,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FoundationCausalCaptureMetrics {
    pub batch_captures: u64,
    pub captured_rows: u64,
    pub readback_submissions: u64,
    pub bytes_copied: u64,
}

impl FoundationCausalCollectionMetrics {
    pub(super) fn measure(
        start: u64,
        end: u64,
        wall_seconds: f64,
        waking_decision_records: usize,
        no_progress_calls: u64,
        before: GpuTrainingCaptureMetrics,
        after: GpuTrainingCaptureMetrics,
    ) -> Result<Self> {
        if !wall_seconds.is_finite() || wall_seconds < 0.0 {
            return Err("invalid causal collection wall interval".into());
        }
        let ticks = end
            .checked_sub(start)
            .ok_or("causal collection world clock moved backward")?;
        let delta = |end: u64, start: u64| {
            end.checked_sub(start)
                .ok_or("causal collection capture counter moved backward")
        };
        let simulation_seconds = ticks as f64 / f64::from(alife_world::WORLD_TICKS_PER_SECOND);
        let rate = (wall_seconds > 0.0).then(|| simulation_seconds / wall_seconds);
        if rate.is_some_and(|rate| !rate.is_finite()) {
            return Err("causal collection speed ratio overflow".into());
        }
        Ok(Self {
            schema: 1,
            control_start_world_tick: start,
            control_end_world_tick: end,
            committed_world_ticks: ticks,
            simulation_seconds,
            collection_wall_seconds: wall_seconds,
            simulation_seconds_per_wall_second: rate,
            waking_decision_records,
            no_progress_calls,
            capture: FoundationCausalCaptureMetrics {
                batch_captures: delta(after.batch_captures, before.batch_captures)?,
                captured_rows: delta(after.captured_rows, before.captured_rows)?,
                readback_submissions: delta(
                    after.readback_submissions,
                    before.readback_submissions,
                )?,
                bytes_copied: delta(after.bytes_copied, before.bytes_copied)?,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resident_historical_receipt_keeps_unmeasured_causal_speed_unknown() {
        let legacy: super::super::FoundationCycleReceipt =
            serde_json::from_value(serde_json::json!({
                "seed": 17, "policy_version": 4, "training_ticks": 32,
                "collection_seconds": 1.0, "update_seconds": 1.0,
                "old_asset_digest": "old", "new_asset_digest": "new",
                "actor_optimizer_step": 7, "value_optimizer_step": 5,
                "completed_epochs": 1, "next_cohort_tick_captured": true,
                "next_cohort_optimizer_rebound": true
            }))
            .unwrap();
        assert!(legacy.causal_collection.is_none());
        assert!(serde_json::to_value(&legacy)
            .unwrap()
            .get("causal_collection")
            .is_none());
    }

    #[test]
    fn resident_causal_metrics_exclude_setup_and_count_sleep_ticks_separately() {
        let before = GpuTrainingCaptureMetrics {
            batch_captures: 20,
            captured_rows: 20,
            readback_submissions: 40,
            bytes_copied: 1000,
        };
        let after = GpuTrainingCaptureMetrics {
            batch_captures: 28,
            captured_rows: 28,
            readback_submissions: 56,
            bytes_copied: 1400,
        };
        // The first 4,000 ordinary setup ticks are excluded. Only four waking
        // records were captured during 200 committed control ticks; sleep and
        // no-progress polls must not be inferred from the record count.
        let metrics =
            FoundationCausalCollectionMetrics::measure(4000, 4200, 2.0, 4, 7, before, after)
                .unwrap();
        assert_eq!(metrics.committed_world_ticks, 200);
        assert_eq!(metrics.simulation_seconds, 10.0);
        assert_eq!(metrics.simulation_seconds_per_wall_second, Some(5.0));
        assert_eq!(metrics.waking_decision_records, 4);
        assert_eq!(metrics.no_progress_calls, 7);
        assert_eq!(metrics.capture.batch_captures, 8);
        assert_eq!(metrics.capture.captured_rows, 8);
        assert_eq!(metrics.capture.readback_submissions, 16);
        assert_eq!(metrics.capture.bytes_copied, 400);
        let encoded = serde_json::to_vec(&metrics).unwrap();
        assert_eq!(
            serde_json::from_slice::<FoundationCausalCollectionMetrics>(&encoded).unwrap(),
            metrics
        );
    }

    #[test]
    fn resident_causal_metrics_do_not_invent_a_rate_or_saturate_corrupt_counters() {
        let zero = GpuTrainingCaptureMetrics::default();
        let empty =
            FoundationCausalCollectionMetrics::measure(9, 9, 0.0, 0, 3, zero, zero).unwrap();
        assert_eq!(empty.simulation_seconds, 0.0);
        assert_eq!(empty.simulation_seconds_per_wall_second, None);
        for seconds in [-1.0, f64::NAN, f64::INFINITY] {
            assert!(
                FoundationCausalCollectionMetrics::measure(0, 1, seconds, 1, 0, zero, zero)
                    .is_err()
            );
        }
        assert!(FoundationCausalCollectionMetrics::measure(2, 1, 1.0, 1, 0, zero, zero).is_err());
        let previous = GpuTrainingCaptureMetrics {
            bytes_copied: 1,
            ..zero
        };
        assert!(
            FoundationCausalCollectionMetrics::measure(0, 1, 1.0, 1, 0, previous, zero).is_err()
        );
        assert!(FoundationCausalCollectionMetrics::measure(
            0,
            1,
            f64::from_bits(1),
            1,
            0,
            zero,
            zero
        )
        .is_err());
    }
}
