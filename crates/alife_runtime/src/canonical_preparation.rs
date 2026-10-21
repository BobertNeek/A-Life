//! Shared canonical cognition preparation used by explicit neural execution backends.
//! Processor placement never changes organism, memory or biochemical ownership.
use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::*;
use alife_world::{grounded_peripheral_summaries, HeadlessWorld};
use std::collections::BTreeMap;
pub fn cognitive_memory_context_for_recall(
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    recall: &PreparedMemoryRecall,
) -> Result<CognitiveContextFrame, ScaffoldContractError> {
    let mut context =
        CognitiveContextFrame::empty(organism_id, sequence_id, recall.context().tick)?;
    for candidate in recall
        .context()
        .candidates
        .iter()
        .take(MAX_CONTEXT_MEMORY_EXPECTANCIES)
    {
        // Target latent lane zero is hunger change, not signed valence:
        // relieving hunger is negative there, but a positive experience.
        let expectancy = candidate
            .best_family_source
            .zip(candidate.family_value.first().copied())
            .map(|(memory_id, value)| (memory_id, value, candidate.family_confidence.raw()));
        if let Some((memory_id, value, confidence)) = expectancy {
            context.memory.expectancies.push(CognitiveMemoryExpectancy {
                memory_id,
                expected_valence: SignedValence::new(value.clamp(-1.0, 1.0))?,
                confidence: NormalizedScalar::new(confidence.clamp(0.0, 1.0))?,
            });
        }
    }
    Ok(context)
}

pub fn cognitive_context_for_recall(
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    recall: &PreparedMemoryRecall,
    topology: &TopologySidecar,
) -> Result<CognitiveContextFrame, ScaffoldContractError> {
    let mut context = cognitive_memory_context_for_recall(organism_id, sequence_id, recall)?;
    let mut concepts = topology.map().concepts().iter().collect::<Vec<_>>();
    concepts.sort_by(|left, right| {
        (right.salience.raw() * right.confidence.raw())
            .total_cmp(&(left.salience.raw() * left.confidence.raw()))
            .then_with(|| left.id.raw().cmp(&right.id.raw()))
    });
    context.concept.active_concepts = concepts
        .into_iter()
        .take(MAX_ACTIVE_CONCEPTS)
        .map(|concept| {
            Ok(CognitiveConceptActivation {
                concept_id: concept.id,
                activation: concept.salience,
                utility: NormalizedScalar::new(concept.confidence.raw())?,
            })
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    context.concept.topology_digest = topology.diagnostics().canonical_digest;

    let mut gaps = topology.map().curiosity_biases();
    gaps.sort_by(|left, right| {
        right
            .salience
            .raw()
            .total_cmp(&left.salience.raw())
            .then_with(|| left.gap_id.raw().cmp(&right.gap_id.raw()))
    });
    context.gap.gap_voltage = NormalizedScalar::new(
        gaps.iter()
            .map(|gap| gap.salience.raw())
            .fold(0.0, f32::max),
    )?;
    context.gap.active_gaps = gaps
        .into_iter()
        .take(MAX_ACTIVE_GAPS)
        .map(|gap| {
            Ok(CognitiveGapActivation {
                gap_id: gap.gap_id,
                voltage: gap.salience,
                uncertainty: NormalizedScalar::new((1.0 - gap.confidence.raw()).clamp(0.0, 1.0))?,
            })
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    context.validate_contract()?;
    Ok(context)
}

pub fn apply_predecision_attention_evidence(
    summaries: &mut [alife_core::PeripheralSummary],
    body_need: f32,
    memory_evidence: &[FinalizedMemoryAttentionEvidence],
    context: &CognitiveContextFrame,
    topology_evidence: &ObjectTopologyEvidence,
    receptors: NeuralReceptorEffects,
) -> Result<(), ScaffoldContractError> {
    receptors.validate_contract()?;
    // Concept and gap evidence stays bound to the candidate/object that
    // produced it. A global maximum would broadcast one object's evidence to
    // every peripheral summary and create false associations.
    context.validate_contract()?;
    for summary in summaries {
        if let alife_core::StableFocusIdentity::TrackedObject(tracked_object_id) = summary.identity
        {
            let (concept, gap) = topology_evidence
                .get(&tracked_object_id)
                .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
            summary.salience.concept = *concept;
            summary.salience.gap_voltage = *gap;
        }
        summary.salience.drive =
            NormalizedScalar::new((body_need * receptors.interoceptive_gain).clamp(0.0, 1.0))?;
        summary.salience.peripheral_intensity = NormalizedScalar::new(
            (summary.salience.peripheral_intensity.raw()
                * receptors.regional_excitability
                * receptors.attention_gain)
                .clamp(0.0, 1.0),
        )?;
        summary.salience.concept = NormalizedScalar::new(
            (summary.salience.concept.raw() * receptors.projection_gain).clamp(0.0, 1.0),
        )?;
        summary.salience.gap_voltage = NormalizedScalar::new(
            (summary.salience.gap_voltage.raw() - receptors.local_threshold_shift).clamp(0.0, 1.0),
        )?;
        summary.salience.novelty = NormalizedScalar::new(
            summary
                .salience
                .novelty
                .raw()
                .max(receptors.structural_growth_gate * 0.25),
        )?;
        summary.salience.uncertainty = NormalizedScalar::new(
            summary
                .salience
                .uncertainty
                .raw()
                .max(receptors.sleep_gate * 0.25),
        )?;
        if let alife_core::StableFocusIdentity::TrackedObject(tracked_object_id) = summary.identity
        {
            if let Some(memory) = memory_evidence
                .iter()
                .find(|memory| memory.tracked_object_id == Some(tracked_object_id))
            {
                summary.salience.memory_expectancy = NormalizedScalar::new(
                    (memory.salience.raw()
                        * (0.5 * receptors.projection_gain + 0.5 * receptors.consolidation_gate))
                        .clamp(0.0, 1.0),
                )?;
            }
        }
        summary.validate_contract()?;
    }
    Ok(())
}

pub fn route_focal_candidates(
    draft: &PerceptionFrameDraft,
    attention: &AttentionFrame,
) -> Result<PerceptionFrameDraft, ScaffoldContractError> {
    attention.validate_contract()?;
    let Some(alife_core::StableFocusIdentity::TrackedObject(focal_id)) =
        attention.focal_targets.first().copied()
    else {
        return Ok(draft.clone());
    };
    let Some(focal_slot) = draft
        .grounded_object_slots()
        .iter()
        .position(|slot| slot.tracked_object_id == focal_id)
        .and_then(|index| u16::try_from(index).ok())
    else {
        return Ok(draft.clone());
    };

    let mut candidates = draft.candidates().to_vec();
    candidates.sort_by_key(|candidate| {
        (
            !matches!(
                candidate.observation,
                CandidateObservationRef::ObjectSlot(slot) if slot == focal_slot
            ),
            candidate.candidate_index,
        )
    });
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.candidate_index =
            u16::try_from(index).map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
    }

    PerceptionFrameDraft::new(
        draft.organism_id(),
        draft.tick(),
        draft.sensor_profile(),
        draft.sensory().clone(),
        draft.body(),
        *draft.homeostasis(),
        candidates,
        draft.profile_provenance(),
        draft.grounded_object_slots().to_vec(),
    )
}

pub fn cognitive_context_with_attention(
    mut context: CognitiveContextFrame,
    attention: AttentionFrame,
) -> Result<CognitiveContextFrame, ScaffoldContractError> {
    context.peripheral.summaries = attention.peripheral_summaries.clone();
    context.focal.identities = attention.focal_targets.clone();
    context.focal.salience = attention.salience_components.clone();
    context.focal.hysteresis = attention.hysteresis;
    context.budget.peripheral_capacity = attention.budget_receipt.peripheral_capacity;
    context.budget.focal_capacity = attention.budget_receipt.focal_capacity;
    context.budget.work_used = attention.budget_receipt.work_units;
    context.budget.work_limit = attention.budget_receipt.work_units;
    context.attention = attention;
    context.validate_contract()?;
    Ok(context)
}

pub fn cognitive_context_with_projection(
    mut context: CognitiveContextFrame,
    projection: alife_core::cognitive_context::CognitiveProjectionFrame,
) -> Result<CognitiveContextFrame, ScaffoldContractError> {
    context.cognitive_projection = Some(projection);
    context.validate_contract()?;
    Ok(context)
}

pub fn tracked_object_id_for_candidate(
    draft: &PerceptionFrameDraft,
    candidate: &alife_core::ActionCandidate,
) -> Result<Option<alife_core::TrackedObjectId>, ScaffoldContractError> {
    match candidate.observation {
        CandidateObservationRef::None => Ok(None),
        CandidateObservationRef::ObjectSlot(slot_index) => {
            let slot = draft
                .grounded_object_slots()
                .get(usize::from(slot_index))
                .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
            if slot.slot_index != slot_index {
                return Err(ScaffoldContractError::InvalidPerceptionFrame);
            }
            slot.tracked_object_id.validate()?;
            Ok(Some(slot.tracked_object_id))
        }
    }
}

pub fn target_prior_residual(
    candidate: &alife_core::CandidateMemoryContextV1,
) -> Result<f32, ScaffoldContractError> {
    let mean_abs_latent = candidate
        .target_latent
        .iter()
        .map(|value| value.abs())
        .sum::<f32>()
        / candidate.target_latent.len() as f32;
    let residual = candidate.target_confidence.raw() * mean_abs_latent;
    if residual.is_finite() {
        Ok(residual.clamp(0.0, 1.0))
    } else {
        Err(ScaffoldContractError::NonFiniteFloat)
    }
}

pub type ObjectTopologyEvidence =
    BTreeMap<alife_core::TrackedObjectId, (NormalizedScalar, NormalizedScalar)>;

pub fn topology_evidence_for_draft(
    draft: &PerceptionFrameDraft,
    topology: &TopologySidecar,
) -> Result<ObjectTopologyEvidence, ScaffoldContractError> {
    draft.validate_contract()?;
    let mut evidence = BTreeMap::new();
    // Grounded slots and the topology map already have enforced capacities.
    // Compute once per stable identity before focus can reorder candidates.
    for slot in draft.grounded_object_slots() {
        if let std::collections::btree_map::Entry::Vacant(entry) =
            evidence.entry(slot.tracked_object_id)
        {
            entry.insert(target_bound_topology_scores(
                Some(slot.tracked_object_id),
                topology,
            )?);
        }
    }
    Ok(evidence)
}

pub fn target_bound_topology_scores(
    tracked_object_id: Option<alife_core::TrackedObjectId>,
    topology: &TopologySidecar,
) -> Result<(NormalizedScalar, NormalizedScalar), ScaffoldContractError> {
    let Some(tracked_object_id) = tracked_object_id else {
        return Ok((NormalizedScalar(0.0), NormalizedScalar(0.0)));
    };
    tracked_object_id.validate()?;
    let concept_ids = topology
        .map()
        .concepts()
        .iter()
        .filter(|concept| concept.bindings.objects.contains(&tracked_object_id))
        .map(|concept| concept.id)
        .collect::<Vec<_>>();
    let concept_match = topology
        .map()
        .concepts()
        .iter()
        .filter(|concept| concept.bindings.objects.contains(&tracked_object_id))
        .map(|concept| concept.confidence.raw() * concept.salience.raw())
        .fold(0.0, f32::max);
    let gap_match = topology
        .map()
        .unresolved_gaps()
        .iter()
        .filter(|gap| {
            matches!(
                gap.status,
                alife_core::GapResolutionStatus::Open
                    | alife_core::GapResolutionStatus::BiasingCuriosity
            )
        })
        .filter(|gap| {
            gap.source_concepts
                .iter()
                .any(|concept_id| concept_ids.contains(concept_id))
        })
        .map(|gap| gap.curiosity_voltage.raw() * gap.salience.raw())
        .fold(0.0, f32::max);
    Ok((
        NormalizedScalar::new(concept_match.clamp(0.0, 1.0))?,
        NormalizedScalar::new(gap_match.clamp(0.0, 1.0))?,
    ))
}

pub fn cognitive_projection_for_draft(
    draft: &PerceptionFrameDraft,
    recall: &PreparedMemoryRecall,
    sequence_id: ExperienceSequenceId,
    predictor: &GroundedSuccessorPredictor,
    topology_evidence: &ObjectTopologyEvidence,
) -> Result<alife_core::cognitive_context::CognitiveProjectionFrame, ScaffoldContractError> {
    draft.validate_contract()?;
    recall.validate_for_draft(draft)?;
    sequence_id.validate()?;
    let source_state = grounded_semantic_state_from_draft(draft)?;
    let source_digest = draft.base_digest().0;
    let memory_candidates = &recall.context().candidates;
    if memory_candidates.len() != draft.candidates().len() {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }

    let forecast_available = predictor.has_acquired_state();
    let mut object_priors = BTreeMap::<TrackedObjectId, f32>::new();
    let mut candidates = Vec::with_capacity(draft.candidates().len());
    for (index, candidate) in draft.candidates().iter().enumerate() {
        let expected_index =
            u16::try_from(index).map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
        if candidate.candidate_index != expected_index
            || memory_candidates[index].candidate_index != expected_index
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let tracked_object_id = tracked_object_id_for_candidate(draft, candidate)?;
        let (concept_match, gap_match) = match tracked_object_id {
            Some(id) => *topology_evidence
                .get(&id)
                .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?,
            None => (NormalizedScalar(0.0), NormalizedScalar(0.0)),
        };
        let prior_residual = NormalizedScalar::new(
            target_prior_residual(&memory_candidates[index])?.clamp(0.0, 1.0),
        )?;
        if let Some(id) = tracked_object_id {
            let object_prior = object_priors.entry(id).or_default();
            *object_prior = object_prior.max(prior_residual.raw());
        }
        let command = candidate.to_command(draft.organism_id(), candidate.sensor_confidence)?;
        let hypothetical_bundle =
            alife_core::arbitrate_gpu_selected_command_into_factorized_bundle(
                draft.organism_id(),
                sequence_id,
                draft.tick(),
                Vec::new(),
                &command,
                None,
                false,
            )?;
        let motor_condition = JointMotorCondition::from_bundle(&hypothetical_bundle)?;
        let mut prediction = predictor.predict(&source_state, &motor_condition)?;
        prediction.source_digest = source_digest;
        candidates.push(alife_core::cognitive_context::CognitiveCandidateInput {
            candidate_index: expected_index,
            candidate_feature_digest: candidate.feature_digest()?,
            tracked_object_id,
            prediction,
            forecast_available,
            concept_match,
            gap_match,
            prior_residual,
        });
    }

    let mut objects = Vec::with_capacity(object_priors.len());
    for (tracked_object_id, prior_residual) in object_priors {
        let (concept_match, gap_match) = *topology_evidence
            .get(&tracked_object_id)
            .ok_or(ScaffoldContractError::InvalidPerceptionFrame)?;
        objects.push(alife_core::cognitive_context::CognitiveObjectEvidence {
            tracked_object_id,
            concept_match,
            gap_match,
            prior_residual: NormalizedScalar::new(prior_residual.clamp(0.0, 1.0))?,
        });
    }

    let projection = alife_core::cognitive_context::CognitiveProjectionFrame {
        schema_version: 1,
        base_frame_digest: draft.base_digest(),
        candidates,
        objects,
    };
    projection.validate_contract()?;
    Ok(projection)
}

pub fn bounded_successor_scalar(value: f32) -> Result<f32, ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok((0.5 + 0.5 * (value / (1.0 + value.abs()))).clamp(0.0, 1.0))
}

pub fn unit_successor_scalar(value: f32) -> Result<f32, ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok(value.clamp(0.0, 1.0))
}

pub fn grounded_semantic_state_from_frame(
    frame: &PerceptionFrame,
) -> Result<SemanticStateVector, ScaffoldContractError> {
    let body = frame.body().neural_projection(frame.sensor_profile());
    grounded_semantic_state(
        body.pose.translation,
        body.velocity.linear,
        frame.homeostasis(),
    )
}

pub fn grounded_semantic_state_from_draft(
    draft: &PerceptionFrameDraft,
) -> Result<SemanticStateVector, ScaffoldContractError> {
    let body = draft.body().neural_projection(draft.sensor_profile());
    grounded_semantic_state(
        body.pose.translation,
        body.velocity.linear,
        draft.homeostasis(),
    )
}

// Both sides describe the same observable quantities: position, velocity and
// seven drives. Contact, success and physical damage remain outcome evidence.
pub fn grounded_semantic_state(
    position: Vec3f,
    velocity: Vec3f,
    homeostasis: &HomeostaticSnapshot,
) -> Result<SemanticStateVector, ScaffoldContractError> {
    let drives = homeostasis.drives.to_array();
    SemanticStateVector::new(vec![
        bounded_successor_scalar(position.x)?,
        bounded_successor_scalar(position.y)?,
        bounded_successor_scalar(position.z)?,
        bounded_successor_scalar(velocity.x)?,
        bounded_successor_scalar(velocity.y)?,
        bounded_successor_scalar(velocity.z)?,
        unit_successor_scalar(drives[0])?,
        unit_successor_scalar(drives[1])?,
        unit_successor_scalar(drives[2])?,
        unit_successor_scalar(drives[3])?,
        unit_successor_scalar(drives[4])?,
        unit_successor_scalar(drives[5])?,
        unit_successor_scalar(drives[6])?,
    ])
}

pub fn grounded_successor_state(
    world: &HeadlessWorld,
    world_entity_id: WorldEntityId,
    biology_after: &BiochemistryState,
    profile: SensorProfile,
) -> Result<SemanticStateVector, ScaffoldContractError> {
    let object = world
        .entity(world_entity_id)
        .ok_or(ScaffoldContractError::InvalidId)?;
    let velocity = match profile {
        SensorProfile::GroundedObjectSlotsV1 | SensorProfile::GroundedTerrainVisionV1 => {
            object.grounded_physical.velocity
        }
        SensorProfile::PrivilegedAffordanceV1 => Vec3f::ZERO,
    };
    let position = if profile == SensorProfile::GroundedTerrainVisionV1 {
        Vec3f::ZERO
    } else {
        object.position
    };
    grounded_semantic_state(position, velocity, &biology_after.homeostasis)
}

pub fn apply_prediction_evidence(
    context: &mut CognitiveContextFrame,
    target: &PredictionTargetReceipt,
    errors: &[f32],
    category_coverage: alife_core::predictive::PredictionCategoryCoverage,
) -> Result<f32, ScaffoldContractError> {
    let bounded_errors = errors
        .iter()
        .map(|error| error.abs().clamp(0.0, 1.0))
        .collect::<Vec<_>>();
    let mean_absolute_error = if bounded_errors.is_empty() {
        0.0
    } else {
        bounded_errors.iter().copied().sum::<f32>() / bounded_errors.len() as f32
    };
    context.prediction.source_digest = target.source_digest;
    context.prediction.semantic_state_abi = target.source_state.abi_version;
    context.prediction.source_state = Some(target.source_state.clone());
    context.prediction.prediction_error = bounded_errors
        .iter()
        .copied()
        .map(NormalizedScalar::new)
        .collect::<Result<Vec<_>, _>>()?;
    context.prediction.motor_condition_magnitude =
        NormalizedScalar::new(target.motor_condition_magnitude.clamp(0.0, 1.0))?;
    context.prediction.category_coverage = Some(category_coverage);

    context.validate_contract()?;
    Ok(mean_absolute_error)
}

pub fn attention_selection_policy_for(
    phenotype: &alife_core::BrainPhenotype,
) -> AttentionSelectionPolicy {
    let capacity = phenotype.cognitive_architecture().attention_capacity();
    AttentionSelectionPolicy {
        focal_capacity: capacity,
        requested_focal_count: capacity,
        ..AttentionSelectionPolicy::default()
    }
}

pub fn predictor_for_phenotype(
    phenotype: &alife_core::BrainPhenotype,
) -> Result<GroundedSuccessorPredictor, ScaffoldContractError> {
    GroundedSuccessorPredictor::with_learning_rate(
        phenotype.cognitive_architecture().predictor_learning_rate(),
    )
}

pub fn sleep_consolidation_config_for(
    phenotype: &alife_core::BrainPhenotype,
) -> Result<SleepConsolidationConfig, ScaffoldContractError> {
    let architecture = phenotype.cognitive_architecture();
    let plan = phenotype.sleep_consolidation_plan();
    let mut config = SleepConsolidationConfig::reference();
    config.sleep_pressure_threshold =
        NormalizedScalar::new(architecture.sleep_trigger_threshold())?;
    config.h_shadow_drain_rate = NormalizedScalar::new(plan.staging_rate())?;
    config.h_shadow_decay_rate = NormalizedScalar::new(plan.fast_decay_rate())?;
    config.lifetime_staging_rate = NormalizedScalar::new(architecture.sleep_consolidation_rate())?;
    config.structural_edit_candidate_limit =
        usize::from(architecture.structural_candidate_budget());
    config.weight_abs_limit = plan.weight_limit();
    config.validate_contract()?;
    Ok(config)
}

#[derive(Debug, Clone)]
pub struct CanonicalPreparationInput {
    pub draft: PerceptionFrameDraft,
    pub sequence_id: ExperienceSequenceId,
    pub homeostasis: HomeostaticSnapshot,
    pub hysteresis: HysteresisState,
    pub policy: AttentionSelectionPolicy,
    pub neural_receptors: NeuralReceptorFrame,
    pub receptor_effects: NeuralReceptorEffects,
}
#[derive(Debug, PartialEq)]
pub struct CanonicalPreparedFrame {
    pub frame: PerceptionFrame,
    pub memory_recall: FinalizedMemoryRecall,
}
pub struct CanonicalPreparationOutcome {
    pub result: Result<CanonicalPreparedFrame, ScaffoldContractError>,
    pub stage: &'static str,
    pub hysteresis: Option<HysteresisState>,
    pub timing: CanonicalPreparationTiming,
}
fn elapsed_ns(start: std::time::Instant) -> u64 {
    start.elapsed().as_nanos().min(u64::MAX as u128) as u64
}
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CanonicalPreparationTiming {
    pub episodic_retrieval_wall_ns: u64,
    pub attention_context_wall_ns: u64,
    pub topology_concept_wall_ns: u64,
    pub gpu_upload_wall_ns: u64,
    pub baseline_recall_wall_ns: u64,
    pub baseline_context_wall_ns: u64,
    pub baseline_attention_evidence_wall_ns: u64,
    pub routed_recall_wall_ns: u64,
    pub routed_context_wall_ns: u64,
    pub cognitive_projection_wall_ns: u64,
    pub routed_finalize_wall_ns: u64,
    pub routed_validate_wall_ns: u64,
}

impl CanonicalPreparationTiming {
    pub fn accumulate(&mut self, other: &Self) {
        macro_rules! add {
            ($field:ident) => {
                self.$field = self.$field.saturating_add(other.$field);
            };
        }
        add!(episodic_retrieval_wall_ns);
        add!(attention_context_wall_ns);
        add!(topology_concept_wall_ns);
        add!(gpu_upload_wall_ns);
        add!(baseline_recall_wall_ns);
        add!(baseline_context_wall_ns);
        add!(baseline_attention_evidence_wall_ns);
        add!(routed_recall_wall_ns);
        add!(routed_context_wall_ns);
        add!(cognitive_projection_wall_ns);
        add!(routed_finalize_wall_ns);
        add!(routed_validate_wall_ns);
    }
}

pub fn selected_focal_novelty(attention: &AttentionFrame) -> f32 {
    attention
        .focal_targets
        .first()
        .zip(attention.salience_components.first())
        .and_then(|(identity, salience)| {
            matches!(identity, alife_core::StableFocusIdentity::TrackedObject(_))
                .then_some(salience.novelty.raw())
        })
        .unwrap_or(0.0)
}

/// One preparation path for gameplay GPU collection and explicit offline CPU collection.
pub fn prepare_canonical_frame(
    input: &CanonicalPreparationInput,
    memory: &MemorySidecarState,
    topology: &TopologySidecar,
    predictor: &GroundedSuccessorPredictor,
    measure: bool,
) -> CanonicalPreparationOutcome {
    let draft = &input.draft;
    let receptor_effects = input.receptor_effects;
    let tick_before = draft.tick();
    let mut preparation_stage = "baseline recall";
    let mut hysteresis = None;
    let mut timing = CanonicalPreparationTiming::default();
    let result = (|| {
        input.neural_receptors.validate_contract()?;
        receptor_effects.validate_contract()?;
        if input.neural_receptors.source_tick != draft.tick()
            || receptor_effects.source_tick != draft.tick()
            || input.homeostasis.tick != draft.tick()
            || input.neural_receptors.source_chemistry_version
                != receptor_effects.source_chemistry_version
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let episodic_retrieval_started = measure.then(std::time::Instant::now);
        let sequence_id = input.sequence_id;
        sequence_id.validate()?;
        preparation_stage = "baseline recall";
        let leaf_started = measure.then(std::time::Instant::now);
        let prepared_recall = memory.recall_frame(draft)?;
        let baseline_recall_ns = leaf_started.map_or(0, elapsed_ns);
        let leaf_started = measure.then(std::time::Instant::now);
        let baseline_context = cognitive_context_for_recall(
            input.draft.organism_id(),
            sequence_id,
            &prepared_recall,
            topology,
        )?;
        let baseline_context_ns = leaf_started.map_or(0, elapsed_ns);
        preparation_stage = "baseline attention evidence";
        let leaf_started = measure.then(std::time::Instant::now);
        let baseline_prepared = prepared_recall.with_cognitive_context(baseline_context.clone())?;
        let memory_evidence = baseline_prepared.attention_evidence_for_draft(draft)?;
        if measure {
            timing.baseline_recall_wall_ns = baseline_recall_ns;
            timing.baseline_context_wall_ns = baseline_context_ns;
            timing.baseline_attention_evidence_wall_ns = leaf_started.map_or(0, elapsed_ns);
        }
        timing.episodic_retrieval_wall_ns = timing
            .episodic_retrieval_wall_ns
            .saturating_add(episodic_retrieval_started.map_or(0, elapsed_ns));
        let attention_context_started = measure.then(std::time::Instant::now);
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
        let routed_draft = route_focal_candidates(draft, &attention)?;
        let novelty = selected_focal_novelty(&attention);
        let routed_draft = routed_draft.with_remembered_novelty(novelty)?;
        timing.attention_context_wall_ns = timing
            .attention_context_wall_ns
            .saturating_add(attention_context_started.map_or(0, elapsed_ns));
        let topology_concept_started = measure.then(std::time::Instant::now);
        preparation_stage = "routed recall";
        let leaf_started = measure.then(std::time::Instant::now);
        let routed_recall = memory.recall_frame(&routed_draft)?;
        let routed_recall_ns = leaf_started.map_or(0, elapsed_ns);
        let leaf_started = measure.then(std::time::Instant::now);
        let cognitive_context = cognitive_context_for_recall(
            input.draft.organism_id(),
            sequence_id,
            &routed_recall,
            topology,
        )?;
        let cognitive_context = cognitive_context_with_attention(cognitive_context, attention)?;
        let routed_context_ns = leaf_started.map_or(0, elapsed_ns);
        preparation_stage = "cognitive projection";
        let leaf_started = measure.then(std::time::Instant::now);
        let cognitive_projection = cognitive_projection_for_draft(
            &routed_draft,
            &routed_recall,
            sequence_id,
            predictor,
            &topology_evidence,
        )?;
        let cognitive_context =
            cognitive_context_with_projection(cognitive_context, cognitive_projection)?;
        let cognitive_projection_ns = leaf_started.map_or(0, elapsed_ns);
        preparation_stage = "routed finalization";
        let leaf_started = measure.then(std::time::Instant::now);
        let prepared_recall = routed_recall.with_cognitive_context(cognitive_context)?;
        let (frame, memory_recall) = prepared_recall.finalize(routed_draft)?;
        let routed_finalize_ns = leaf_started.map_or(0, elapsed_ns);
        let leaf_started = measure.then(std::time::Instant::now);
        memory_recall.validate_for_frame(&frame)?;
        if measure {
            timing.routed_recall_wall_ns = routed_recall_ns;
            timing.routed_context_wall_ns = routed_context_ns;
            timing.cognitive_projection_wall_ns = cognitive_projection_ns;
            timing.routed_finalize_wall_ns = routed_finalize_ns;
            timing.routed_validate_wall_ns = leaf_started.map_or(0, elapsed_ns);
        }
        timing.topology_concept_wall_ns = timing
            .topology_concept_wall_ns
            .saturating_add(topology_concept_started.map_or(0, elapsed_ns));

        Ok(CanonicalPreparedFrame {
            frame,
            memory_recall,
        })
    })();
    CanonicalPreparationOutcome {
        result,
        stage: preparation_stage,
        hysteresis,
        timing,
    }
}

pub fn foundation_construction_development(
    genome: &BrainGenome,
    capacity: &BrainCapacityClass,
    development: &DevelopmentState,
) -> Result<DevelopmentState, ScaffoldContractError> {
    development.validate_contract()?;
    if development.genome_id != genome.id {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    if !matches!(
        capacity.id(),
        BrainCapacityClass::N512_ID | BrainCapacityClass::N1024_ID | BrainCapacityClass::N2048_ID
    ) {
        return Ok(development.clone());
    }

    // Production brain classes own a stable coordinate topology. World
    // development remains authoritative in ResidentCognition; the construction
    // input removes runtime chronology and dynamic gates that would reshape it.
    let mut construction = development.clone();
    construction.age_ticks = Tick::ZERO;
    construction.maturation = NormalizedScalar::new(1.0)?;
    construction.enabled_lobes.clear();
    construction.active_sensor_channels.clear();
    construction.active_motor_affordances.clear();
    construction.open_critical_periods.clear();
    construction.sleep_cycle_count = 0;
    construction.consolidation_cycle_count = 0;
    construction.last_sleep_tick = None;
    construction.validate_contract()?;
    Ok(construction)
}
