//! Optional requested-byte policy for the cache's bounded lifetime only.
//!
//! Pinned wgpu 29 Vulkan ordinary buffers, including mapped initialization and
//! queue staging, use the same allocator. Its reserved snapshot covers existing
//! managed buffers on the shared device, including the live runtime. The
//! controller must exclusively own allocations throughout the cache lifetime:
//! no raw/external imports or other allocations from threads, loader callbacks,
//! or custom window Borrow/Drop implementations. The continuous sealed loader
//! and owned/Arc procedural fixture handles perform CPU/storage work only.
//! This is NOT a bound on future reserved blocks, free VRAM, or a reservation:
//! allocator growth and driver/external memory remain separate concerns.
use super::*;

// Device::poll may return PollError or panic for a fatal HAL DeviceError.
// Cleanup must fail-stop the shared device without replacing an original
// loader/validation Result or causing a second escaping panic during unwind.
pub(super) fn retire_catching_failure<E>(
    retire: impl FnOnce() -> Result<(), E>,
    mark_failed: impl FnOnce(),
) -> bool {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(retire)) {
        Ok(Ok(())) => true,
        Ok(Err(_)) | Err(_) => {
            mark_failed();
            false
        }
    }
}

pub(super) fn window_fits(window: &PpoTrainingWindow, max_ticks: usize) -> bool {
    (1..=crate::MAX_TRAINING_SEQUENCE_TICKS).contains(&max_ticks)
        && window.sequence.ticks.len() <= max_ticks
        && window.auxiliary.is_none()
        && !window.speech_targets.iter().any(Option::is_some)
}

fn additional_requested_bytes(
    arena: u64,
    initialization_upload: u64,
    trainer_queue_upload: u64,
    max_ticks: usize,
    cache: u64,
) -> Option<u64> {
    let ticks = u64::try_from(max_ticks).ok()?;
    // Header/rows share their real upload extents. The scale write occurs in
    // accumulation; auxiliary/speech objectives are dropped before evaluation.
    let objective_upload = ticks
        .checked_mul(PPO_ROW_WORDS as u64)?
        .checked_mul(4)?
        .checked_add(PPO_HEADER_BYTES)?
        .checked_add(4)?;
    let readback = ticks
        .checked_mul(PPO_METRIC_WORDS as u64)?
        .checked_mul(4)?
        .max(8);
    // At most old+new replay arenas survive between metric waits. Counting
    // both again over the snapshot deliberately overcounts its current arena.
    arena
        .checked_mul(2)?
        .checked_add(initialization_upload)?
        .checked_add(trainer_queue_upload)?
        .checked_add(objective_upload)?
        .checked_add(readback)?
        .checked_add(cache)
}

fn fits_policy(reserved: u64, additional: u64, ceiling: u64) -> bool {
    ceiling > 0
        && reserved
            .checked_add(additional)
            .is_some_and(|peak| peak <= ceiling)
}

pub(super) fn evaluate(
    trainer: &crate::FoundationTrainer,
    max_ticks: usize,
    cache_bytes: u64,
) -> FrozenGradientAdmissionStats {
    let mut decision = FrozenGradientAdmissionStats {
        declared_profile_ceiling_bytes: Some(
            trainer
                .session()
                .backend()
                .runtime_budget()
                .physical_allocation_ceiling_bytes,
        ),
        ..Default::default()
    };
    if !(1..=crate::MAX_TRAINING_SEQUENCE_TICKS).contains(&max_ticks)
        || trainer.session().backend().adapter_info_for_test().backend != wgpu::Backend::Vulkan
    {
        return decision;
    }
    let Ok(bound) = trainer.replay_allocation_bound_for_ticks(max_ticks) else {
        return decision;
    };
    let Some(additional) = additional_requested_bytes(
        bound.arena_bytes,
        bound.initialization_upload_bytes,
        bound.queue_upload_bytes,
        max_ticks,
        cache_bytes,
    ) else {
        return decision;
    };
    decision.additional_requested_upper_bytes = Some(additional);
    let Ok((device, _)) = trainer.session().backend().offline_training_device_queue() else {
        return decision;
    };
    let Some(report) = device.generate_allocator_report() else {
        return decision;
    };
    if report.total_reserved_bytes == 0
        || report.total_allocated_bytes > report.total_reserved_bytes
    {
        return decision;
    }
    decision.allocator_reserved_baseline_bytes = Some(report.total_reserved_bytes);
    decision.admitted = fits_policy(
        report.total_reserved_bytes,
        additional,
        trainer
            .session()
            .backend()
            .runtime_budget()
            .physical_allocation_ceiling_bytes,
    );
    decision
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_policy_accepts_exact_boundary_and_refuses_overflow() {
        assert!(fits_policy(100, 200, 300));
        assert!(!fits_policy(100, 201, 300));
        assert!(!fits_policy(u64::MAX, 1, u64::MAX));
        assert!(!fits_policy(0, 0, 0));
    }

    #[test]
    fn cleanup_failure_marks_failed_without_replacing_result_or_unwind() {
        use std::sync::atomic::{AtomicBool, Ordering};
        struct Cleanup<'a>(&'a AtomicBool, bool);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                retire_catching_failure(
                    || {
                        assert!(!self.1, "cleanup failure");
                        Err(())
                    },
                    || self.0.store(true, Ordering::Release),
                );
            }
        }
        for cleanup_panics in [false, true] {
            let failed = AtomicBool::new(false);
            let result: Result<(), &str> = {
                let _cleanup = Cleanup(&failed, cleanup_panics);
                Err("original loader error")
            };
            assert_eq!(result, Err("original loader error"));
            assert!(failed.load(Ordering::Acquire));
            let failed = AtomicBool::new(false);
            let original = std::panic::catch_unwind(|| {
                let _cleanup = Cleanup(&failed, cleanup_panics);
                panic!("original unwind");
            })
            .unwrap_err();
            assert_eq!(original.downcast_ref::<&str>(), Some(&"original unwind"));
            assert!(failed.load(Ordering::Acquire));
        }
    }
}
