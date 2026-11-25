//! Immutable per-owner CPU preparation, bounded to one scoped child and its caller.
use super::*;
use alife_gpu_backend::{GpuBrainSlot, GpuClosedLoopError};
use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex, OnceLock};

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
        available.min(rows)
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
    let shared = alife_runtime::canonical_preparation::CanonicalPreparationInputRef {
        draft: &input.draft,
        sequence_id: input.sequence_id,
        homeostasis: input.homeostasis,
        hysteresis: input.hysteresis,
        policy: input.policy,
        neural_receptors: &input.neural_receptors,
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
    let outcome = alife_runtime::canonical_preparation::prepare_canonical_frame_borrowed(
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
pub(super) struct PreparationFaults {
    spawn_failure: bool,
    panic_row: Option<usize>,
    visits: Vec<std::sync::atomic::AtomicUsize>,
}

struct PoolTask {
    func: Box<dyn FnOnce() + Send + 'static>,
    completion: Arc<BatchCompletion>,
}

struct PoolState {
    queue: VecDeque<PoolTask>,
    shutdown: bool,
}

struct PoolInner {
    state: Mutex<PoolState>,
    worker_cv: Condvar,
}

struct BatchCompletion {
    remaining: Mutex<usize>,
    done: Condvar,
}

struct InFlightGuard {
    completion: Arc<BatchCompletion>,
}

impl Drop for InFlightGuard {
    fn drop(&mut self) {
        let mut remaining = self
            .completion
            .remaining
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        while *remaining > 0 {
            remaining = self
                .completion
                .done
                .wait(remaining)
                .unwrap_or_else(|poison| poison.into_inner());
        }
    }
}

struct TaskCompletionGuard(Arc<BatchCompletion>);

impl Drop for TaskCompletionGuard {
    fn drop(&mut self) {
        let mut remaining = self
            .0
            .remaining
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        *remaining -= 1;
        if *remaining == 0 {
            self.0.done.notify_all();
        }
    }
}

pub(super) struct CpuPreparationPool {
    inner: Arc<PoolInner>,
    workers: Mutex<Vec<thread::JoinHandle<()>>>,
}

impl CpuPreparationPool {
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(PoolInner {
                state: Mutex::new(PoolState {
                    queue: VecDeque::new(),
                    shutdown: false,
                }),
                worker_cv: Condvar::new(),
            }),
            workers: Mutex::new(Vec::new()),
        }
    }

    fn worker_loop(inner: Arc<PoolInner>) {
        loop {
            let task = {
                let mut state = inner
                    .state
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                loop {
                    if let Some(task) = state.queue.pop_front() {
                        break task;
                    }
                    if state.shutdown {
                        return;
                    }
                    state = inner
                        .worker_cv
                        .wait(state)
                        .unwrap_or_else(|poison| poison.into_inner());
                }
            };
            let _done = TaskCompletionGuard(task.completion);
            let _ = catch_unwind(AssertUnwindSafe(task.func));
        }
    }

    fn ensure_workers(&self, needed: usize) -> usize {
        let mut workers = self
            .workers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        while workers.len() < needed {
            let inner = Arc::clone(&self.inner);
            match thread::Builder::new()
                .name("alife-cpu-preparation".into())
                .spawn(move || Self::worker_loop(inner))
            {
                Ok(handle) => workers.push(handle),
                Err(_) => break,
            }
        }
        workers.len()
    }

    #[cfg(test)]
    pub(super) fn worker_count(&self) -> usize {
        self.workers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .len()
    }

    pub(super) fn prepare_rows(
        &self,
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

        catch_unwind(AssertUnwindSafe(|| {
            let requested_chunks = workers.min(jobs.len());
            if requested_chunks < 2 {
                return Ok(compute(jobs, 0));
            }
            #[cfg(test)]
            let forced_spawn_failure = faults.spawn_failure;
            #[cfg(not(test))]
            let forced_spawn_failure = false;
            if forced_spawn_failure {
                return Ok(compute(jobs, 0));
            }
            let available_workers = self.ensure_workers(requested_chunks - 1);
            if available_workers == 0 {
                return Ok(compute(jobs, 0));
            }
            let effective_chunks = (available_workers + 1).min(jobs.len());
            let base = jobs.len() / effective_chunks;
            let rem = jobs.len() % effective_chunks;
            let mut chunk_slices: Vec<(usize, &[CpuPreparationJob<'_>])> =
                Vec::with_capacity(effective_chunks);
            let mut start = 0_usize;
            for chunk_idx in 0..effective_chunks {
                let len = base + usize::from(chunk_idx < rem);
                chunk_slices.push((start, &jobs[start..start + len]));
                start += len;
            }

            let mut chunk_results: Vec<Option<Result<Vec<CpuPreparationOutcome>, ()>>> =
                (0..effective_chunks).map(|_| None).collect();
            let (first_slot, worker_slots) = chunk_results.split_at_mut(1);
            let (first_offset, first_rows) = chunk_slices[0];

            let completion = Arc::new(BatchCompletion {
                remaining: Mutex::new(worker_slots.len()),
                done: Condvar::new(),
            });
            let mut tasks = Vec::with_capacity(worker_slots.len());
            for ((offset, rows), slot) in chunk_slices[1..].iter().copied().zip(worker_slots) {
                let task: Box<dyn FnOnce() + Send + '_> = Box::new(move || {
                    let result =
                        catch_unwind(AssertUnwindSafe(|| compute(rows, offset))).map_err(|_| ());
                    *slot = Some(result);
                });
                // SAFETY: `guard` (`InFlightGuard`) below waits in `Drop` until
                // `completion.remaining == 0`. Every worker moves `task` into
                // `catch_unwind` and drops it before `TaskCompletionGuard::drop`
                // decrements `completion.remaining`. Therefore all references
                // captured by `task` (`rows`, `slot`, `faults`) strictly outlive
                // any worker access, even if chunk 0 panics or unwinds.
                let erased: Box<dyn FnOnce() + Send + 'static> =
                    unsafe { std::mem::transmute(task) };
                tasks.push(PoolTask {
                    func: erased,
                    completion: Arc::clone(&completion),
                });
            }

            let guard = InFlightGuard {
                completion: Arc::clone(&completion),
            };
            {
                let mut state = self
                    .inner
                    .state
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner());
                state.queue.extend(tasks);
                self.inner.worker_cv.notify_all();
            }

            first_slot[0] = Some(
                catch_unwind(AssertUnwindSafe(|| compute(first_rows, first_offset)))
                    .map_err(|_| ()),
            );
            drop(guard);

            let mut output = Vec::with_capacity(jobs.len());
            for slot in chunk_results {
                match slot {
                    Some(Ok(rows)) => output.extend(rows),
                    Some(Err(())) | None => {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                }
            }
            Ok(output)
        }))
        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?
    }
}

impl Drop for CpuPreparationPool {
    fn drop(&mut self) {
        {
            let mut state = self
                .inner
                .state
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            state.shutdown = true;
            self.inner.worker_cv.notify_all();
        }
        let mut workers = self
            .workers
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for handle in workers.drain(..) {
            let _ = handle.join();
        }
    }
}

fn global_cpu_preparation_pool() -> &'static CpuPreparationPool {
    static POOL: OnceLock<CpuPreparationPool> = OnceLock::new();
    POOL.get_or_init(CpuPreparationPool::new)
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
    global_cpu_preparation_pool().prepare_rows(
        jobs,
        workers,
        measure,
        #[cfg(test)]
        faults,
    )
}

#[cfg(test)]
#[path = "cpu_preparation_tests.rs"]
mod tests;
