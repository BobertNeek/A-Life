//! Ordinary play uses intermittent recall and concept summaries. Immediate
//! perception, neural action selection and outcome learning stay at tick rate.
use super::cpu_preparation::{
    CpuPreparationJob, CpuPreparationOutcome, CpuPreparationTiming, PreparedCpuFrame,
};
use super::*;
use std::sync::Arc;

pub(super) const RECALL_INTERVAL: u64 = 4;
pub(super) const CONCEPT_INTERVAL: u64 = 10;

#[derive(Debug, Clone, Default)]
pub(super) struct ApproximateCognitionCache {
    pub recall: Arc<alife_core::RuntimeMemoryRecallCache>,
    pub summary: Option<Arc<CognitiveContextFrame>>,
    pub object_evidence: Arc<ObjectTopologyEvidence>,
    pub last_full_recall_tick: Option<Tick>,
    pub last_summary_tick: Option<Tick>,
    pub last_topology_tick: Option<Tick>,
    pub targets: Arc<[alife_core::TrackedObjectId]>,
    pub drives: [f32; alife_core::DriveSnapshot::CHANNEL_COUNT],
    pub force_refresh: bool,
}

fn due(last: Option<Tick>, now: Tick, interval: u64) -> bool {
    last.is_none_or(|last| now.raw() < last.raw() || now.raw() - last.raw() >= interval)
}

pub(super) fn important_outcome(outcome: &PostActionOutcome) -> bool {
    outcome.pain_delta.raw() > 0.01
        || outcome.reward_valence.raw().abs() > 0.15
        || outcome
            .homeostatic_delta
            .drives
            .to_array()
            .iter()
            .any(|change| change.abs() > 0.03)
        || outcome.contradiction_observed
        || outcome.teacher_feedback.is_some()
}

fn bounded_best<'a, T>(
    items: impl Iterator<Item = &'a T>,
    cap: usize,
    compare: impl Fn(&T, &T) -> std::cmp::Ordering,
) -> Vec<&'a T> {
    let mut selected: Vec<&T> = Vec::with_capacity(cap);
    for item in items {
        let at = selected.partition_point(|old| compare(old, item).is_le());
        if at < cap {
            if selected.len() == cap {
                selected.pop();
            }
            selected.insert(at, item);
        }
    }
    selected
}

pub(super) fn summary_for_recall(
    owner: OrganismId,
    sequence: ExperienceSequenceId,
    recall: &PreparedMemoryRecall,
    topology: &TopologySidecar,
) -> Result<CognitiveContextFrame, ScaffoldContractError> {
    let mut context = cognitive_memory_context_for_recall(owner, sequence, recall)?;
    let concepts = bounded_best(
        topology.map().concepts().iter(),
        MAX_ACTIVE_CONCEPTS,
        |left, right| {
            (right.salience.raw() * right.confidence.raw())
                .total_cmp(&(left.salience.raw() * left.confidence.raw()))
                .then_with(|| left.id.raw().cmp(&right.id.raw()))
        },
    );
    context.concept.active_concepts = concepts
        .into_iter()
        .map(|concept| {
            Ok(CognitiveConceptActivation {
                concept_id: concept.id,
                activation: concept.salience,
                utility: NormalizedScalar::new(concept.confidence.raw())?,
            })
        })
        .collect::<Result<_, ScaffoldContractError>>()?;
    context.concept.topology_digest = topology.diagnostics().canonical_digest;
    let salience =
        |gap: &alife_core::UnresolvedGap| gap.salience.raw().max(gap.curiosity_voltage.raw());
    let gaps = bounded_best(
        topology.map().unresolved_gaps().iter().filter(|gap| {
            matches!(
                gap.status,
                alife_core::GapResolutionStatus::Open
                    | alife_core::GapResolutionStatus::BiasingCuriosity
            )
        }),
        MAX_ACTIVE_GAPS,
        |left, right| {
            salience(right)
                .total_cmp(&salience(left))
                .then_with(|| left.id.raw().cmp(&right.id.raw()))
        },
    );
    context.gap.gap_voltage = NormalizedScalar::new(gaps.first().map_or(0.0, |gap| salience(gap)))?;
    context.gap.active_gaps = gaps
        .into_iter()
        .map(|gap| {
            Ok(CognitiveGapActivation {
                gap_id: gap.id,
                voltage: NormalizedScalar::new(salience(gap))?,
                uncertainty: NormalizedScalar::new((1.0 - gap.confidence.raw()).clamp(0.0, 1.0))?,
            })
        })
        .collect::<Result<_, ScaffoldContractError>>()?;
    Ok(context)
}

pub(super) fn prepare_row(job: &CpuPreparationJob<'_>, measure: bool) -> CpuPreparationOutcome {
    let input = job.input;
    let old = input.approximate_cache.as_ref().expect("approximate row");
    let mut next = (**old).clone();
    let mut timing = CpuPreparationTiming::default();
    let mut hysteresis = None;
    let mut stage = "cached attention";
    let result = (|| {
        let memory = job
            .memory
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let topology = job
            .topology
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let predictor = job
            .predictor
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let draft = &input.draft;
        let tick = draft.tick();
        let owner = draft.organism_id();
        let slots = draft.grounded_object_slots();
        let new_target = slots
            .iter()
            .any(|slot| !old.targets.contains(&slot.tracked_object_id));
        let drives = input.homeostasis.drives.to_array();
        let drive_change = drives
            .iter()
            .zip(&old.drives)
            .any(|(now, old)| (now - old).abs() > 0.15);
        let urgent = old.force_refresh || new_target || drive_change;
        let refresh_recall = urgent || due(old.last_full_recall_tick, tick, RECALL_INTERVAL);
        let refresh_summary = urgent || due(old.last_summary_tick, tick, CONCEPT_INTERVAL);
        let started = measure.then(Instant::now);
        if refresh_summary {
            next.object_evidence = Arc::new(topology_evidence_for_draft(draft, topology)?);
        }
        let mut context = CognitiveContextFrame::empty(owner, input.sequence_id, tick)?;
        if let Some(summary) = &old.summary {
            context.memory = summary.memory.clone();
            context.concept = summary.concept.clone();
            context.gap = summary.gap.clone();
        }
        let evidence = old.recall.attention_evidence();
        let mut summaries = grounded_peripheral_summaries(draft.grounded_object_slots())?;
        let body_need = drives.iter().copied().fold(0.0, f32::max);
        apply_predecision_attention_evidence(
            &mut summaries,
            body_need,
            evidence,
            &context,
            &next.object_evidence,
            input.receptor_effects,
        )?;
        // Familiarity is a cheap indexed cue. It does not trigger a second recall.
        for summary in &mut summaries {
            if let alife_core::StableFocusIdentity::TrackedObject(id) = summary.identity {
                summary.salience.novelty = NormalizedScalar::new(
                    1.0 - memory
                        .bank()
                        .object_familiarity(owner, id, memory.profile()),
                )?;
            }
        }
        let attention = select_focal_targets(
            owner,
            input.sequence_id,
            tick,
            &summaries,
            input.hysteresis,
            input.policy,
        )?;
        hysteresis = Some(attention.hysteresis);
        let routed = route_focal_candidates(draft, &attention)?;
        let novelty = attention
            .focal_targets
            .first()
            .and_then(|id| match id {
                alife_core::StableFocusIdentity::TrackedObject(id) => Some(
                    1.0 - memory
                        .bank()
                        .object_familiarity(owner, *id, memory.profile()),
                ),
                _ => None,
            })
            .unwrap_or(0.0);
        let routed = routed.with_remembered_novelty(novelty)?;
        timing.attention_context_wall_ns = started.map_or(0, elapsed_ns);
        stage = "focused runtime recall";
        let started = measure.then(Instant::now);
        let (recall, refreshed) =
            memory.recall_frame_runtime(&routed, &old.recall, refresh_recall)?;
        let recall_changed = refreshed.is_some();
        if let Some(refreshed) = refreshed {
            next.recall = Arc::new(refreshed);
            // Missing candidate keys refresh only those entries. They must not
            // postpone the deadline for refreshing the retained expectancies.
            if refresh_recall {
                next.last_full_recall_tick = Some(tick);
            }
        }
        timing.episodic_retrieval_wall_ns = started.map_or(0, elapsed_ns);
        timing.routed_recall_wall_ns = timing.episodic_retrieval_wall_ns;
        let started = measure.then(Instant::now);
        let leaf_started = started;
        let mut context = if refresh_summary {
            summary_for_recall(owner, input.sequence_id, &recall, topology)?
        } else {
            let mut fresh = cognitive_memory_context_for_recall(owner, input.sequence_id, &recall)?;
            if let Some(summary) = &old.summary {
                fresh.concept = summary.concept.clone();
                fresh.gap = summary.gap.clone();
            }
            fresh
        };
        if refresh_summary {
            next.last_summary_tick = Some(tick);
        }
        // Keep small summary buffers shared until the next recall/graph refresh.
        if refresh_summary || recall_changed {
            next.summary = Some(Arc::new(context.clone()));
        }
        context = cognitive_context_with_attention(context, attention)?;
        timing.routed_context_wall_ns = leaf_started.map_or(0, elapsed_ns);
        let leaf_started = measure.then(Instant::now);
        let projection = cognitive_projection_for_draft(
            &routed,
            &recall,
            input.sequence_id,
            predictor,
            &next.object_evidence,
        )?;
        context = cognitive_context_with_projection(context, projection)?;
        timing.cognitive_projection_wall_ns = leaf_started.map_or(0, elapsed_ns);
        let leaf_started = measure.then(Instant::now);
        let (frame, memory_recall) = recall.with_cognitive_context(context)?.finalize(routed)?;
        timing.routed_finalize_wall_ns = leaf_started.map_or(0, elapsed_ns);
        timing.topology_concept_wall_ns = started.map_or(0, elapsed_ns);
        stage = "GPU memory upload";
        let started = measure.then(Instant::now);
        let slot = job
            .slot
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        // The old path encoded an entire perception payload solely to discard
        // it and keep this binding. The real backend still packs the payload.
        let record = slot.record();
        let binding = alife_gpu_backend::GpuPerceptionFrameBinding {
            perception_header_index: 0,
            slot: record.slot,
            slot_generation: record.slot_generation,
            tick: frame.tick(),
            candidate_count: frame.candidates().len() as u16,
            base_frame_digest: frame.base_digest(),
            context_digest: frame.context().canonical_digest(),
            final_frame_digest: frame.frame_digest(),
        };
        let memory_upload =
            GpuMemoryContextUpload::try_from_finalized(&frame, &memory_recall, binding, slot)
                .map_err(super::cpu_preparation::upload_error)?
                .bind_neural_receptor_effects(input.receptor_effects)
                .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
        timing.gpu_upload_wall_ns = started.map_or(0, elapsed_ns);
        if !slots
            .iter()
            .map(|slot| slot.tracked_object_id)
            .eq(old.targets.iter().copied())
        {
            next.targets = slots.iter().map(|slot| slot.tracked_object_id).collect();
        }
        next.drives = drives;
        next.force_refresh = false;
        Ok(PreparedCpuFrame {
            frame,
            memory_recall,
            memory_upload,
        })
    })();
    CpuPreparationOutcome {
        result,
        stage,
        hysteresis,
        timing,
        approximate_cache: Some(Arc::new(next)),
    }
}

pub(super) fn should_observe_topology(
    cache: &ApproximateCognitionCache,
    outcome: &PostActionOutcome,
) -> bool {
    important_outcome(outcome)
        || due(
            cache.last_topology_tick,
            outcome.outcome_tick,
            CONCEPT_INTERVAL,
        )
}
