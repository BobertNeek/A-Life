//! Immutable per-owner CPU preparation, bounded to one scoped child and its caller.
use super::*;
use alife_gpu_backend::{GpuBrainSlot, GpuClosedLoopError};
use std::panic::{catch_unwind, AssertUnwindSafe};

pub(super) struct CapturedCpuPreparation {
    pub approximate_cache:
        Option<std::sync::Arc<super::approximate_cognition::ApproximateCognitionCache>>,
    pub draft: PerceptionFrameDraft,
    pub sequence_id: ExperienceSequenceId,
    pub homeostasis: HomeostaticSnapshot,
    pub hysteresis: alife_core::HysteresisState,
    pub policy: AttentionSelectionPolicy,
    pub neural_receptors: NeuralReceptorFrame,
    pub receptor_effects: NeuralReceptorEffects,
}

pub(super) struct CapturedLivePreparation {
    pub handle: GpuBrainHandle,
    pub world_entity_id: WorldEntityId,
    pub input: CapturedCpuPreparation,
}

pub(super) struct CpuPreparationJob<'a> {
    pub input: &'a CapturedCpuPreparation,
    pub slot: Option<&'a GpuBrainSlot>,
    pub memory: Option<&'a MemorySidecarState>,
    pub topology: Option<&'a TopologySidecar>,
    pub predictor: Option<&'a GroundedSuccessorPredictor>,
}

#[derive(Debug, PartialEq)]
pub(super) struct PreparedCpuFrame {
    pub frame: PerceptionFrame,
    pub memory_recall: FinalizedMemoryRecall,
    pub memory_upload: GpuMemoryContextUpload,
}

pub(super) use alife_runtime::canonical_preparation::CanonicalPreparationTiming as CpuPreparationTiming;

pub(super) struct CpuPreparationOutcome {
    pub approximate_cache:
        Option<std::sync::Arc<super::approximate_cognition::ApproximateCognitionCache>>,
    pub result: Result<PreparedCpuFrame, ScaffoldContractError>,
    pub stage: &'static str,
    pub hysteresis: Option<alife_core::HysteresisState>,
    pub timing: CpuPreparationTiming,
}

// Hints are captured serially into each owned draft before a worker sees it.
// No worker may request, poll or consume a hint, including serial spawn fallback.
pub(super) fn preparation_worker_count(rows: usize, available: usize) -> usize {
    if rows >= 8 && available >= 2 {
        2
    } else {
        1
    }
}

pub(super) fn upload_error(error: GpuClosedLoopError) -> ScaffoldContractError {
    match error {
        GpuClosedLoopError::LayoutMismatch => ScaffoldContractError::GpuLayoutMismatch,
        GpuClosedLoopError::StaleOrForeignHandle => ScaffoldContractError::BrainOwnershipMismatch,
        GpuClosedLoopError::MalformedUpload
        | GpuClosedLoopError::NonFinitePayload
        | GpuClosedLoopError::InvalidOffsetDomain => ScaffoldContractError::InvalidPerceptionFrame,
        GpuClosedLoopError::CapacityExceeded
        | GpuClosedLoopError::ArithmeticOverflow
        | GpuClosedLoopError::SubmissionFailed => ScaffoldContractError::NeuralBackendUnavailable,
    }
}

// select_focal_targets copies identities and salience from the same selected
// indices, including hysteresis reordering. Tracked novelty was computed from
// this job's immutable memory bank before selection; reuse those exact bits.
fn prepare_cpu_row(job: &CpuPreparationJob<'_>, measure: bool) -> CpuPreparationOutcome {
    if job.input.approximate_cache.is_some() {
        return super::approximate_cognition::prepare_row(job, measure);
    }
    let input = job.input;
    let shared = alife_runtime::canonical_preparation::CanonicalPreparationInput {
        draft: input.draft.clone(),
        sequence_id: input.sequence_id,
        homeostasis: input.homeostasis,
        hysteresis: input.hysteresis,
        policy: input.policy,
        neural_receptors: input.neural_receptors.clone(),
        receptor_effects: input.receptor_effects,
    };
    let Some(memory) = job.memory else {
        return CpuPreparationOutcome {
            approximate_cache: None,
            result: Err(ScaffoldContractError::BrainOwnershipMismatch),
            stage: "baseline recall",
            hysteresis: None,
            timing: Default::default(),
        };
    };
    let Some(topology) = job.topology else {
        return CpuPreparationOutcome {
            approximate_cache: None,
            result: Err(ScaffoldContractError::BrainOwnershipMismatch),
            stage: "baseline recall",
            hysteresis: None,
            timing: Default::default(),
        };
    };
    let Some(predictor) = job.predictor else {
        return CpuPreparationOutcome {
            approximate_cache: None,
            result: Err(ScaffoldContractError::BrainOwnershipMismatch),
            stage: "baseline recall",
            hysteresis: None,
            timing: Default::default(),
        };
    };
    let outcome = alife_runtime::canonical_preparation::prepare_canonical_frame(
        &shared, memory, topology, predictor, measure,
    );
    let mut timing = outcome.timing;
    let mut stage = outcome.stage;
    let result = outcome.result.and_then(|prepared| {
        let frame = prepared.frame;
        let memory_recall = prepared.memory_recall;
        let started = measure.then(Instant::now);
        stage = "GPU memory upload";
        let slot = job
            .slot
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let perception = alife_gpu_backend::GpuPerceptionUpload::try_from_frame(&frame, slot, 0)
            .map_err(upload_error)?;
        let memory_upload = GpuMemoryContextUpload::try_from_finalized(
            &frame,
            &memory_recall,
            perception.frame_binding,
            slot,
        )
        .map_err(upload_error)?
        .bind_neural_receptor_effects(input.receptor_effects)
        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        timing.gpu_upload_wall_ns = timing
            .gpu_upload_wall_ns
            .saturating_add(started.map_or(0, elapsed_ns));
        Ok(PreparedCpuFrame {
            frame,
            memory_recall,
            memory_upload,
        })
    });
    CpuPreparationOutcome {
        approximate_cache: None,
        result,
        stage,
        hysteresis: outcome.hysteresis,
        timing,
    }
}

// Faults exist only in CPU tests, without environment switches or live hooks.
#[cfg(test)]
#[derive(Default)]
struct PreparationFaults {
    spawn_failure: bool,
    panic_row: Option<usize>,
    visits: Vec<std::sync::atomic::AtomicUsize>,
}

pub(super) fn prepare_cpu_rows(
    jobs: &[CpuPreparationJob<'_>],
    workers: usize,
    measure: bool,
) -> Result<Vec<CpuPreparationOutcome>, ScaffoldContractError> {
    prepare_cpu_rows_inner(
        jobs,
        workers,
        measure,
        #[cfg(test)]
        &PreparationFaults::default(),
    )
}

fn prepare_cpu_rows_inner(
    jobs: &[CpuPreparationJob<'_>],
    workers: usize,
    measure: bool,
    #[cfg(test)] faults: &PreparationFaults,
) -> Result<Vec<CpuPreparationOutcome>, ScaffoldContractError> {
    let compute = |rows: &[CpuPreparationJob<'_>], offset: usize| {
        rows.iter()
            .enumerate()
            .map(|(index, row)| {
                #[cfg(test)]
                {
                    if let Some(visits) = faults.visits.get(offset + index) {
                        visits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    }
                    assert_ne!(
                        faults.panic_row,
                        Some(offset + index),
                        "injected CPU preparation panic"
                    );
                }
                #[cfg(not(test))]
                let _ = (index, offset);
                prepare_cpu_row(row, measure)
            })
            .collect::<Vec<_>>()
    };
    // Both halves borrow retained inputs. A failed spawn drops no captured
    // draft/hint; fallback computes each row once and never repeats housekeeping.
    catch_unwind(AssertUnwindSafe(|| {
        if workers < 2 || jobs.len() < 2 {
            return Ok(compute(jobs, 0));
        }
        thread::scope(|scope| {
            let split = jobs.len().div_ceil(2);
            let (left, right) = jobs.split_at(split);
            #[cfg(test)]
            let forced_spawn_failure = faults.spawn_failure;
            #[cfg(not(test))]
            let forced_spawn_failure = false;
            let child = if forced_spawn_failure {
                Err(std::io::Error::other(
                    "injected CPU preparation spawn failure",
                ))
            } else {
                thread::Builder::new()
                    .name("alife-cpu-preparation".into())
                    .spawn_scoped(scope, move || {
                        catch_unwind(AssertUnwindSafe(|| compute(right, split)))
                    })
            };
            match child {
                Err(_) => Ok(compute(jobs, 0)),
                Ok(child) => {
                    let left = catch_unwind(AssertUnwindSafe(|| compute(left, 0)));
                    // Always join, including a panic on the calling thread.
                    let right = child
                        .join()
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                    let mut output =
                        left.map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                    output.extend(right);
                    Ok(output)
                }
            }
        })
    }))
    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?
}

#[cfg(test)]
#[path = "cpu_preparation_tests.rs"]
mod tests;
