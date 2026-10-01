//! Immutable per-owner CPU preparation, bounded to one scoped child and its caller.
use super::*;
use alife_gpu_backend::{GpuBrainSlot, GpuClosedLoopError};
use std::panic::{catch_unwind, AssertUnwindSafe};

pub(super) struct CapturedCpuPreparation {
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

#[derive(Default)]
pub(super) struct CpuPreparationTiming {
    pub episodic_retrieval_wall_ns: u64,
    pub attention_context_wall_ns: u64,
    pub topology_concept_wall_ns: u64,
    pub gpu_upload_wall_ns: u64,
}

pub(super) struct CpuPreparationOutcome {
    pub result: Result<PreparedCpuFrame, ScaffoldContractError>,
    pub stage: &'static str,
    pub hysteresis: Option<alife_core::HysteresisState>,
    pub timing: CpuPreparationTiming,
}

// A live asynchronous terrain prior retains interleaved polling, including the
// CPU interval between owners. No worker may request or consume a hint.
pub(super) fn preparation_worker_count(rows: usize, available: usize, live_prior: bool) -> usize {
    if rows >= 8 && available >= 2 && !live_prior {
        2
    } else {
        1
    }
}

fn upload_error(error: GpuClosedLoopError) -> ScaffoldContractError {
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

fn prepare_cpu_row(job: &CpuPreparationJob<'_>, measure: bool) -> CpuPreparationOutcome {
    let input = job.input;
    let draft = &input.draft;
    let receptor_effects = input.receptor_effects;
    let tick_before = draft.tick();
    let mut preparation_stage = "baseline recall";
    let mut hysteresis = None;
    let mut timing = CpuPreparationTiming::default();
    let result = (|| {
        let episodic_retrieval_started = measure.then(Instant::now);
        let memory = job
            .memory
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let topology = job
            .topology
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let predictor = job
            .predictor
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let sequence_id = input.sequence_id;
        sequence_id.validate()?;
        preparation_stage = "baseline recall";
        let prepared_recall = memory.recall_frame(draft)?;
        let baseline_context = cognitive_context_for_recall(
            input.draft.organism_id(),
            sequence_id,
            &prepared_recall,
            topology,
        )?;
        preparation_stage = "baseline attention evidence";
        let baseline_prepared = prepared_recall.with_cognitive_context(baseline_context.clone())?;
        let memory_evidence = baseline_prepared.attention_evidence_for_draft(draft)?;
        timing.episodic_retrieval_wall_ns = timing
            .episodic_retrieval_wall_ns
            .saturating_add(episodic_retrieval_started.map_or(0, elapsed_ns));
        let attention_context_started = measure.then(Instant::now);
        let mut peripheral_summaries =
            grounded_peripheral_summaries(draft.grounded_object_slots())?;
        let topology_evidence = topology_evidence_for_draft(draft, topology)?;
        let body_need = input
            .homeostasis
            .drives
            .to_array()
            .iter()
            .copied()
            .fold(0.0, f32::max);
        apply_predecision_attention_evidence(
            &mut peripheral_summaries,
            body_need,
            &memory_evidence,
            &baseline_context,
            &topology_evidence,
            receptor_effects,
        )?;
        preparation_stage = "attention selection";
        for summary in &mut peripheral_summaries {
            if let alife_core::StableFocusIdentity::TrackedObject(id) = summary.identity {
                summary.salience.novelty = NormalizedScalar::new(
                    1.0 - memory.bank().object_familiarity(
                        input.draft.organism_id(),
                        id,
                        memory.profile(),
                    ),
                )?;
            }
        }
        let attention = select_focal_targets(
            input.draft.organism_id(),
            sequence_id,
            tick_before,
            &peripheral_summaries,
            input.hysteresis,
            input.policy,
        )?;
        hysteresis = Some(attention.hysteresis);
        preparation_stage = "focal routing";
        let routed_draft = route_focal_candidates(draft.clone(), &attention)?;
        let novelty = attention
            .focal_targets
            .first()
            .and_then(|id| match id {
                alife_core::StableFocusIdentity::TrackedObject(object) => Some(
                    1.0 - memory.bank().object_familiarity(
                        input.draft.organism_id(),
                        *object,
                        memory.profile(),
                    ),
                ),
                _ => None,
            })
            .unwrap_or(0.0);
        let routed_draft = routed_draft.with_remembered_novelty(novelty)?;
        timing.attention_context_wall_ns = timing
            .attention_context_wall_ns
            .saturating_add(attention_context_started.map_or(0, elapsed_ns));
        let topology_concept_started = measure.then(Instant::now);
        preparation_stage = "routed recall";
        let routed_recall = memory.recall_frame(&routed_draft)?;
        let cognitive_context = cognitive_context_for_recall(
            input.draft.organism_id(),
            sequence_id,
            &routed_recall,
            topology,
        )?;
        let cognitive_context = cognitive_context_with_attention(cognitive_context, attention)?;
        preparation_stage = "cognitive projection";
        let cognitive_projection = cognitive_projection_for_draft(
            &routed_draft,
            &routed_recall,
            sequence_id,
            predictor,
            &topology_evidence,
        )?;
        let cognitive_context =
            cognitive_context_with_projection(cognitive_context, cognitive_projection)?;
        preparation_stage = "routed finalization";
        let prepared_recall = routed_recall.with_cognitive_context(cognitive_context)?;
        let (frame, memory_recall) = prepared_recall.finalize(routed_draft)?;
        memory_recall.validate_for_frame(&frame)?;
        timing.topology_concept_wall_ns = timing
            .topology_concept_wall_ns
            .saturating_add(topology_concept_started.map_or(0, elapsed_ns));
        let gpu_upload_started = measure.then(Instant::now);
        preparation_stage = "GPU memory upload";
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
        .bind_neural_receptor_effects(receptor_effects)
        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        timing.gpu_upload_wall_ns = timing
            .gpu_upload_wall_ns
            .saturating_add(gpu_upload_started.map_or(0, elapsed_ns));
        Ok(PreparedCpuFrame {
            frame,
            memory_recall,
            memory_upload,
        })
    })();
    CpuPreparationOutcome {
        result,
        stage: preparation_stage,
        hysteresis,
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
