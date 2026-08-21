//! The one production cognition/action transaction shared by every host.
//!
//! Hosts own cadence, sleep scheduling, sidecar observation, and world
//! advancement. This module owns the ordinary grounded GPU decision through
//! measured outcome, successor prediction, and sealed learning patch.

use alife_core::cognitive_work::CognitiveWorkCounters;
use alife_core::{
    ActionCommand, ActionKind, ActionTarget, BiochemistryState, BodyState,
    BoundedCoordinationSummary, BoundedMotorPayload, BrainCapacityClass, BrainGenome,
    BrainPhenotype, BrainWorkCounters, CandidateActionFamily, CandidateObservationRef,
    CognitiveCandidatePrediction, CognitiveContextFrame, CognitiveInteroceptiveView,
    CognitiveWorkReceipt, DevelopmentState, ExperiencePatch, ExperienceSequenceId,
    GroundedFocalDetail, GroundedOutcomeFeatures, GroundedSuccessorPredictor, HomeostaticSnapshot,
    JointMotorCondition, LanguageGroundingLedger, MemoryRecallReceipt, MotorChannel,
    MotorCommandBundle, MotorFamily, NeuralActionSelection, NormalizedScalar, PerceptionFrame,
    PerceptionFrameDigest, PostActionOutcome, PreActionSnapshot, PredictionTargetReceipt,
    ScaffoldContractError, SemanticStateVector, SignedValence, SpeechMotorPayload,
    StableFocusIdentity, Validate, Vec3f, WorldEntityId, MAX_FOCAL_FEATURE_WIDTH,
    MAX_FOCAL_TARGETS,
};
use alife_gpu_backend::{
    GpuBrainHandle, GpuClosedLoopTick, GpuSelectorDiagnosticReceipt, GpuV11WorkReceipt,
    PendingEligibilityReceipt, GPU_MOTOR_CHANNEL_SLOT_COUNT,
};
use alife_world::{HeadlessMotorTransactionError, HeadlessWorld};

use crate::GpuAuthoritativeSession;

const SINGLE_ACTION_COMPATIBILITY_ADAPTER_VERSION: u16 = 1;
const VOCAL_CHANNEL_PAYLOAD_MAGIC_V1: u32 = 0x5348_5031;

pub const MAX_PREDECISION_PREDICTIONS: usize = 8;

#[derive(Debug, Clone)]
pub struct PredecisionCandidatePrediction {
    pub candidate_index: u16,
    pub motor_condition: JointMotorCondition,
    pub prediction: alife_core::predictive::SuccessorPrediction,
}

#[derive(Debug, Clone)]
pub struct PredecisionPredictionPreparation {
    pub source_state: SemanticStateVector,
    pub candidates: Vec<PredecisionCandidatePrediction>,
}

/// Builds one stable source state and bounded categorical motor conditions.
/// This API only prepares consequence facts for the GPU context. It never
/// ranks, scores, or selects a candidate.
pub fn prepare_predecision_predictions(
    predictor: &GroundedSuccessorPredictor,
    organism_id: alife_core::OrganismId,
    sequence_id: ExperienceSequenceId,
    frame: &PerceptionFrame,
    phenotype: &BrainPhenotype,
    canonical_biochemistry: &BiochemistryState,
) -> Result<PredecisionPredictionPreparation, ScaffoldContractError> {
    canonical_biochemistry.validate_contract()?;
    let source_state = grounded_semantic_state_from_frame(frame, canonical_biochemistry)?;
    let limit = usize::from(phenotype.cognitive_architecture().predictor_capacity())
        .min(MAX_PREDECISION_PREDICTIONS);
    if limit == 0 || frame.candidates().is_empty() {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    let mut conditions = Vec::with_capacity(limit);
    for candidate in frame.candidates().iter().take(limit) {
        let command = candidate.to_command(organism_id, candidate.sensor_confidence)?;
        let bundle = compatibility_bundle_for_selected_action_v1(
            organism_id,
            sequence_id,
            frame.tick(),
            &command,
        )?;
        conditions.push((
            candidate.candidate_index,
            JointMotorCondition::from_bundle(&bundle)?,
        ));
    }
    let joint_conditions = conditions
        .iter()
        .map(|(_, condition)| condition.clone())
        .collect::<Vec<_>>();
    let predictions = predictor.predict_candidates(&source_state, &joint_conditions)?;
    let candidates = conditions
        .into_iter()
        .zip(predictions)
        .map(
            |((candidate_index, motor_condition), prediction)| PredecisionCandidatePrediction {
                candidate_index,
                motor_condition,
                prediction,
            },
        )
        .collect();
    Ok(PredecisionPredictionPreparation {
        source_state,
        candidates,
    })
}

/// Completes all predecision context before the memory-context upload. The
/// admission is canonical world state, while topology and memory context are
/// retained on the supplied frame.
pub fn prepare_predecision_context(
    world: &HeadlessWorld,
    predictor: &GroundedSuccessorPredictor,
    organism_id: alife_core::OrganismId,
    sequence_id: ExperienceSequenceId,
    frame: &PerceptionFrame,
    phenotype: &BrainPhenotype,
    canonical_biochemistry: &BiochemistryState,
    context: &mut CognitiveContextFrame,
) -> Result<PredecisionPredictionPreparation, ScaffoldContractError> {
    context.interoceptive = CognitiveInteroceptiveView::from_biochemistry(canonical_biochemistry)?;
    reacquire_focal_context(world, frame, phenotype, context)?;
    let preparation = prepare_predecision_predictions(
        predictor,
        organism_id,
        sequence_id,
        frame,
        phenotype,
        canonical_biochemistry,
    )?;
    context.apply_predecision_predictions(
        preparation.source_state.clone(),
        cognitive_predictions_for_predecision(&preparation)?,
    )?;
    Ok(preparation)
}

pub fn cognitive_predictions_for_predecision(
    preparation: &PredecisionPredictionPreparation,
) -> Result<Vec<CognitiveCandidatePrediction>, ScaffoldContractError> {
    preparation
        .candidates
        .iter()
        .map(
            |candidate| -> Result<CognitiveCandidatePrediction, ScaffoldContractError> {
                Ok(CognitiveCandidatePrediction {
                    candidate_index: candidate.candidate_index,
                    action_family: candidate_family_for_motor(
                        candidate
                            .motor_condition
                            .channels
                            .first()
                            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?
                            .family(),
                    ),
                    predicted_successor: candidate
                        .prediction
                        .predicted_successor
                        .iter()
                        .copied()
                        .map(NormalizedScalar::new)
                        .collect::<Result<Vec<_>, _>>()?,
                    uncertainty: NormalizedScalar::new(candidate.prediction.uncertainty)?,
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()
}

fn candidate_family_for_motor(family: MotorFamily) -> CandidateActionFamily {
    match family {
        MotorFamily::Locomotion => CandidateActionFamily::Approach,
        MotorFamily::Manipulation => CandidateActionFamily::Contact,
        MotorFamily::Posture => CandidateActionFamily::Rest,
        MotorFamily::Orientation | MotorFamily::Vocal | MotorFamily::SpeciesSpecific => {
            CandidateActionFamily::Other
        }
    }
}

/// Explicit mechanism controls crossing the shared production boundary.
/// Hosts may choose the mask, but they cannot substitute another cognition
/// implementation for a disabled mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductionCausalMechanismMask {
    pub sleep: bool,
    pub memory_observation: bool,
    pub topology_observation: bool,
    pub plasticity: bool,
}

/// Ordered stages owned by the shared ordinary transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionCausalStage {
    CanonicalSync,
    DueSleep,
    SensingAndContext,
    GpuDecisionAndEligibility,
    FactorizedWorldAction,
    SuccessorPredictionAndPatchSeal,
    GpuPlasticity,
    MemoryObservation,
    TopologyObservation,
    CognitiveWork,
    DueWorldAdvance,
    CanonicalResync,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionCausalStageReceipt {
    pub ordered: Vec<ProductionCausalStage>,
}

/// Narrow host-owned capabilities for stages whose cadence or sidecar storage
/// differs between gameplay and Era 1. The shared transaction still controls
/// their order and invokes a capability only when the explicit mask permits it.
pub struct ProductionCausalStageHooks<'a> {
    pub due_sleep: &'a mut dyn FnMut() -> Result<bool, ScaffoldContractError>,
    pub gpu_plasticity: &'a mut dyn FnMut(
        &mut GpuAuthoritativeSession,
        &ProductionCausalStep,
    ) -> Result<bool, ScaffoldContractError>,
    pub memory_observation:
        &'a mut dyn FnMut(&ProductionCausalStep) -> Result<(), ScaffoldContractError>,
    pub topology_observation:
        &'a mut dyn FnMut(&ProductionCausalStep) -> Result<(), ScaffoldContractError>,
    pub due_world_advance: &'a mut dyn FnMut(
        &mut HeadlessWorld,
        &ProductionCausalStep,
    ) -> Result<bool, ScaffoldContractError>,
    pub canonical_resync: &'a mut dyn FnMut(
        &HeadlessWorld,
        &ProductionCausalStep,
    ) -> Result<bool, ScaffoldContractError>,
}

impl ProductionCausalMechanismMask {
    pub const fn all_enabled() -> Self {
        Self {
            sleep: true,
            memory_observation: true,
            topology_observation: true,
            plasticity: true,
        }
    }
}

/// All mutable resident state needed by the ordinary production transaction.
/// The host may keep presentation and cadence state beside this contract, but
/// it cannot replace any of these authorities with a second policy path.
pub struct ProductionCausalStepInput<'a> {
    pub world: &'a mut HeadlessWorld,
    pub predictor: &'a mut GroundedSuccessorPredictor,
    pub language_grounding: &'a mut LanguageGroundingLedger,
    pub last_cognitive_context: &'a mut Option<CognitiveContextFrame>,
    pub last_selected_motor_bundle: &'a mut Option<MotorCommandBundle>,
    pub last_cognitive_work: &'a mut CognitiveWorkReceipt,
    pub next_sequence: &'a mut u64,
    pub organism_id: alife_core::OrganismId,
    pub world_entity_id: WorldEntityId,
    pub handle: GpuBrainHandle,
    pub phenotype: &'a BrainPhenotype,
    pub genome: &'a BrainGenome,
    pub development: DevelopmentState,
    pub frame: PerceptionFrame,
    pub memory_recall: alife_core::FinalizedMemoryRecall,
    pub gpu_tick: GpuClosedLoopTick,
    pub mechanisms: ProductionCausalMechanismMask,
    pub cognitive_work_cost_policy: &'a alife_core::cognitive_work::CognitiveWorkCostPolicy,
}

/// Evidence and durable identifiers emitted by the shared transaction.
#[derive(Debug, Clone)]
pub struct ProductionCausalStep {
    pub handle: GpuBrainHandle,
    pub world_entity_id: WorldEntityId,
    pub frame: PerceptionFrame,
    pub memory_recall: MemoryRecallReceipt,
    pub memory_context_final_digest: PerceptionFrameDigest,
    pub frame_digest: PerceptionFrameDigest,
    pub pending_eligibility: PendingEligibilityReceipt,
    pub sequence_id: ExperienceSequenceId,
    pub outcome_tick: alife_core::Tick,
    pub selected_action: ActionCommand,
    pub selected_action_kind: ActionKind,
    pub selected_family: CandidateActionFamily,
    pub selected_observation: CandidateObservationRef,
    pub selected_candidate_index: u16,
    pub dispatch_generation: u64,
    pub speech_payload: Option<SpeechMotorPayload>,
    pub speech_prompted: bool,
    pub selector_diagnostic: Option<GpuSelectorDiagnosticReceipt>,
    pub cognitive_context_digest: [u64; 4],
    pub cognitive_work: CognitiveWorkReceipt,
    pub patch: ExperiencePatch,
    pub mechanisms: ProductionCausalMechanismMask,
    pub learning_applied: bool,
    pub memory_observed: bool,
    pub topology_observed: bool,
    pub stage_receipt: ProductionCausalStageReceipt,
}

/// Runs the complete ordinary production transaction around the shared
/// factorized cognition core. Host callbacks retain only cadence and sidecar
/// storage. They cannot reorder or replace the causal stages.
pub fn run_production_causal_transaction(
    session: &mut GpuAuthoritativeSession,
    mut input: ProductionCausalStepInput<'_>,
    hooks: &mut ProductionCausalStageHooks<'_>,
) -> Result<ProductionCausalStep, ScaffoldContractError> {
    let mechanisms = input.mechanisms;
    let mut ordered = vec![ProductionCausalStage::CanonicalSync];
    validate_canonical_sync(&input)?;

    if mechanisms.sleep {
        if (hooks.due_sleep)()? {
            ordered.push(ProductionCausalStage::DueSleep);
        }
    }
    validate_sensing_and_context(&input)?;
    ordered.push(ProductionCausalStage::SensingAndContext);
    validate_gpu_decision_and_eligibility(session, &input)?;
    ordered.push(ProductionCausalStage::GpuDecisionAndEligibility);

    let mut step = run_production_causal_step(session, &mut input)?;
    ordered.push(ProductionCausalStage::FactorizedWorldAction);
    ordered.push(ProductionCausalStage::SuccessorPredictionAndPatchSeal);

    let learning_applied = if mechanisms.plasticity {
        (hooks.gpu_plasticity)(session, &step)?
    } else {
        session.discard_pending_eligibility(step.handle, step.pending_eligibility.identity())?;
        false
    };
    step.learning_applied = learning_applied;
    ordered.push(ProductionCausalStage::GpuPlasticity);

    let mut memory_observed = false;
    if mechanisms.memory_observation {
        (hooks.memory_observation)(&step)?;
        input.language_grounding.observe_sealed(&step.patch)?;
        memory_observed = true;
        ordered.push(ProductionCausalStage::MemoryObservation);
    }
    let mut topology_observed = false;
    if mechanisms.topology_observation {
        (hooks.topology_observation)(&step)?;
        topology_observed = true;
        ordered.push(ProductionCausalStage::TopologyObservation);
    }
    apply_cognitive_work_cost(
        input.world,
        step.patch.header().organism_id,
        step.cognitive_work,
        input.cognitive_work_cost_policy,
    )?;
    ordered.push(ProductionCausalStage::CognitiveWork);

    if (hooks.due_world_advance)(input.world, &step)? {
        ordered.push(ProductionCausalStage::DueWorldAdvance);
        if (hooks.canonical_resync)(input.world, &step)? {
            ordered.push(ProductionCausalStage::CanonicalResync);
        }
    }
    step.stage_receipt = ProductionCausalStageReceipt { ordered };
    step.memory_observed = memory_observed;
    step.topology_observed = topology_observed;
    Ok(step)
}

/// Executes the canonical GPU selection-to-sealed-patch transaction.
///
/// The GPU tick is supplied by the host's shared session after grounded
/// sensing and context upload. This function validates that binding, decodes
/// the factorized motor bundle, performs one world/body/biology transaction,
/// observes the measured successor, and seals the patch. Learning credit is
/// intentionally left pending for the host to apply or discard after sealing.
pub fn run_production_causal_step(
    session: &GpuAuthoritativeSession,
    input: &mut ProductionCausalStepInput<'_>,
) -> Result<ProductionCausalStep, ScaffoldContractError> {
    let world = &mut *input.world;
    let predictor = &mut *input.predictor;
    let last_cognitive_context = &mut *input.last_cognitive_context;
    let last_selected_motor_bundle = &mut *input.last_selected_motor_bundle;
    let last_cognitive_work = &mut *input.last_cognitive_work;
    let next_sequence = &mut *input.next_sequence;
    let organism_id = input.organism_id;
    let world_entity_id = input.world_entity_id;
    let handle = input.handle;
    let phenotype = input.phenotype;
    let genome = input.genome;
    let development = input.development.clone();
    let frame = input.frame.clone();
    let memory_recall = input.memory_recall.clone();
    let gpu_tick = input.gpu_tick.clone();
    let mechanisms = input.mechanisms;

    session.ensure_neural_actions_available()?;
    let memory_binding = gpu_tick
        .memory_context_binding
        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
    if gpu_tick.handle != handle
        || gpu_tick.base_digest != frame.base_digest()
        || gpu_tick.frame_digest != frame.frame_digest()
        || gpu_tick.hardware_receipt_generation != session.hardware_receipt().generation
        || memory_binding.slot != handle.slot()
        || memory_binding.slot_generation != handle.generation()
        || memory_binding.base_frame_digest != memory_recall.base_frame_digest()
        || memory_binding.context_digest != memory_recall.context_digest()
        || memory_binding.final_frame_digest != memory_recall.final_frame_digest()
        || usize::from(memory_binding.candidate_count) != frame.candidates().len()
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    memory_recall.validate_for_frame(&frame)?;
    let mut cognitive_context = memory_recall
        .cognitive_context()
        .cloned()
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let sequence_id = ExperienceSequenceId(*next_sequence);
    sequence_id.validate()?;
    let selected_candidate = *frame
        .candidates()
        .get(usize::from(gpu_tick.selection.candidate_index))
        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
    let pending_identity = gpu_tick.pending_eligibility.identity();
    if pending_identity.handle_generation() != handle.generation()
        || pending_identity.phenotype_hash() != handle.phenotype_hash()
        || pending_identity.dispatch_generation() != gpu_tick.dispatch_generation
        || pending_identity.originating_tick() != frame.tick()
        || pending_identity.frame_digest() != frame.frame_digest()
        || pending_identity.active_activation_side() != gpu_tick.active_activation_side
        || pending_identity.candidate_index() != gpu_tick.selection.candidate_index
        || pending_identity.action_id() != selected_candidate.action_id
        || pending_identity.action_family() != selected_candidate.family
        || pending_identity.candidate_feature_digest() != selected_candidate.feature_digest()?
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }

    let selected_action =
        selected_candidate.to_command(organism_id, gpu_tick.selection.confidence)?;
    let speech_prompted = frame
        .sensory()
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .any(|token| token.source_kind == alife_core::UtteranceSourceKind::Player);
    let factorized_channels = phenotype
        .candidate_decoder()
        .factorized_motor_channels(phenotype)?;
    let motor_bundle = factorized_motor_bundle_for_candidates(
        organism_id,
        sequence_id,
        frame.tick(),
        &frame,
        gpu_tick.factorized_motor_candidates,
        &factorized_channels,
        &selected_action,
        gpu_tick.selection.candidate_index,
        gpu_tick.speech_payload.as_ref(),
        speech_prompted,
    )?;
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence_id,
        handle.class_id(),
        handle.phenotype_hash(),
        genome.id,
        genome.schema_version,
        development,
        frame.clone(),
    )?;
    let decision = alife_core::DecisionSnapshot::from_neural_selection(
        sequence_id,
        handle.phenotype_hash(),
        gpu_tick.dispatch_generation,
        gpu_tick.active_activation_side,
        &frame,
        NeuralActionSelection {
            candidate_index: gpu_tick.selection.candidate_index,
            logit: gpu_tick.selection.logit,
            confidence: gpu_tick.selection.confidence,
            active_tiles: gpu_tick.selection.active_tiles,
            active_synapses: gpu_tick.selection.active_synapses,
        },
        selected_action.clone(),
    )?
    .with_finalized_memory_recall(
        &frame,
        &memory_recall,
        gpu_tick.selection.candidate_index,
    )?;

    let source_state = cognitive_context
        .prediction
        .source_state
        .clone()
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let motor_condition = JointMotorCondition::from_bundle(&motor_bundle)?;
    let motor_receipt = world
        .apply_registered_motor_bundle(&motor_bundle, world_entity_id)
        .map_err(map_motor_transaction_error)?;
    let physical = motor_receipt.joint.execution;
    let succeeded = motor_receipt.succeeded;
    let outcome_features =
        grounded_outcome_features(physical, succeeded, motor_receipt.body_event)?;
    let action_sensitivity_score = prediction_action_sensitivity(
        predictor,
        &frame,
        organism_id,
        sequence_id,
        &source_state,
        &motor_condition,
    )?;
    let successor_separability_score = prediction_successor_separability(
        predictor,
        &frame,
        organism_id,
        sequence_id,
        &source_state,
        &motor_condition,
    )?;
    let target_state =
        grounded_successor_state(world, world_entity_id, &motor_receipt.biology_after)?;
    let prediction_target = PredictionTargetReceipt::for_successor_with_outcome(
        organism_id,
        sequence_id,
        selected_action.action_id,
        frame.tick(),
        frame.frame_digest().0,
        source_state,
        motor_condition,
        target_state,
        outcome_features,
    )?
    .with_information_diagnostics(action_sensitivity_score, successor_separability_score)?;
    let prediction_update = predictor.observe(&prediction_target)?;
    let grounded_prediction_error = apply_prediction_evidence(
        &mut cognitive_context,
        &prediction_target,
        &prediction_update.error,
        candidate_target_identity(&frame, &selected_candidate),
    )?;
    let cognitive_context_digest = cognitive_context.canonical_digest()?;
    let cognitive_work = cognitive_work_receipt(
        &cognitive_context,
        memory_recall.receipt(),
        &gpu_tick.work.counters,
        &gpu_tick.v11_work,
        prediction_update.error.len() as u64,
    )?;
    *last_cognitive_context = Some(cognitive_context.clone());
    *last_selected_motor_bundle = Some(motor_bundle.clone());
    *last_cognitive_work = cognitive_work;
    let mut outcome = PostActionOutcome::new(
        organism_id,
        sequence_id,
        motor_receipt.outcome_tick,
        succeeded,
        physical,
        alife_core::HomeostaticDelta::zero(),
        SignedValence::new(motor_receipt.body_event.reward_outcome)?,
        NormalizedScalar::new(if succeeded { 0.0 } else { 1.0 })?,
        NormalizedScalar::new(motor_receipt.body_event.damage)?,
        SignedValence::new(motor_receipt.body_event.energy)?,
        NormalizedScalar::new(grounded_prediction_error)?,
    )?;
    outcome.contradiction_observed = !succeeded;
    outcome = outcome.with_v11_joint(motor_receipt.joint, cognitive_work)?;
    let patch = ExperiencePatch::new_v11_with_decision(
        pre_action,
        decision,
        motor_bundle,
        outcome,
        prediction_target,
        cognitive_work,
        cognitive_context,
    )?;
    *next_sequence = (*next_sequence)
        .checked_add(1)
        .ok_or(ScaffoldContractError::InvalidId)?;

    Ok(ProductionCausalStep {
        handle,
        world_entity_id,
        frame_digest: frame.frame_digest(),
        memory_context_final_digest: memory_binding.final_frame_digest,
        memory_recall: memory_recall.receipt().clone(),
        frame,
        pending_eligibility: gpu_tick.pending_eligibility,
        sequence_id,
        outcome_tick: motor_receipt.outcome_tick,
        selected_action,
        selected_action_kind: selected_candidate.kind,
        selected_family: selected_candidate.family,
        selected_observation: selected_candidate.observation,
        selected_candidate_index: gpu_tick.selection.candidate_index,
        dispatch_generation: gpu_tick.dispatch_generation,
        speech_payload: gpu_tick.speech_payload,
        speech_prompted,
        selector_diagnostic: gpu_tick.selector_diagnostic,
        cognitive_context_digest,
        cognitive_work,
        patch,
        mechanisms,
        learning_applied: false,
        memory_observed: false,
        topology_observed: false,
        stage_receipt: ProductionCausalStageReceipt {
            ordered: Vec::new(),
        },
    })
}

fn reacquire_focal_context(
    world: &HeadlessWorld,
    frame: &PerceptionFrame,
    phenotype: &BrainPhenotype,
    context: &mut CognitiveContextFrame,
) -> Result<(), ScaffoldContractError> {
    let class = BrainCapacityClass::supported_for_id(phenotype.brain_class_id())?;
    let class_focal_capacity =
        usize::from(class.execution().max_object_slots()).min(MAX_FOCAL_TARGETS);
    let phenotype_focal_capacity =
        usize::from(phenotype.cognitive_architecture().attention_capacity());
    let current_focal_capacity = usize::from(context.budget.focal_capacity)
        .min(usize::from(context.attention.budget_receipt.focal_capacity));
    let granted_focal_capacity = usize::from(context.attention.budget_receipt.granted_focal_count);
    let focal_capacity = class_focal_capacity
        .min(phenotype_focal_capacity)
        .min(current_focal_capacity)
        .min(granted_focal_capacity);
    let selected_identities = context
        .attention
        .focal_targets
        .iter()
        .copied()
        .take(focal_capacity)
        .collect::<Vec<_>>();
    let tracked_targets = selected_identities
        .iter()
        .filter_map(|identity| match identity {
            StableFocusIdentity::TrackedObject(tracked_object_id) => Some(*tracked_object_id),
            _ => None,
        })
        .collect::<Vec<_>>();
    let observations =
        world.reacquire_grounded_focal(frame.organism_id(), frame.tick(), &tracked_targets)?;
    let current_feature_width = match context.budget.focal_feature_width {
        0 => MAX_FOCAL_FEATURE_WIDTH,
        width => width,
    };
    let feature_width = phenotype
        .cognitive_architecture()
        .motor_head_width()
        .min(class.execution().max_decoder_input_lanes())
        .min(current_feature_width)
        .min(MAX_FOCAL_FEATURE_WIDTH);
    let mut details = Vec::with_capacity(observations.len());
    for observation in observations {
        let Some(slot) = frame
            .grounded_object_slots()
            .iter()
            .find(|slot| slot.tracked_object_id == observation.tracked_object_id)
        else {
            continue;
        };
        details.push(GroundedFocalDetail::new(
            observation.transport_entity,
            observation.relative_position,
            observation.properties.velocity,
            *slot,
            observation.confidence,
            feature_width,
        )?);
    }
    let peripheral_work_units = context.attention.peripheral_summaries.len() as u64;
    let focal_work_units = (details.len() as u64).saturating_mul(u64::from(feature_width));
    let work_used = peripheral_work_units.saturating_add(focal_work_units);
    context.focal.identities = selected_identities;
    context
        .focal
        .salience
        .truncate(context.focal.identities.len());
    context.focal.grounded_details = details;
    context.budget.focal_capacity = focal_capacity as u8;
    context.budget.focal_feature_width = if context.focal.grounded_details.is_empty() {
        0
    } else {
        feature_width
    };
    context.budget.peripheral_work_units = peripheral_work_units;
    context.budget.focal_work_units = focal_work_units;
    context.budget.work_used = work_used;
    context.budget.work_limit = context.budget.work_limit.max(work_used);
    context.attention.budget_receipt.work_units = work_used;
    context.validate_contract()
}

fn validate_canonical_sync(
    input: &ProductionCausalStepInput<'_>,
) -> Result<(), ScaffoldContractError> {
    let record = input
        .world
        .organism_registry()
        .get(input.organism_id)
        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
    let admission = record.authoritative_admission_at(input.frame.tick())?;
    if admission.organism_id != input.organism_id
        || admission.world_entity_id != input.world_entity_id
        || admission.biochemistry.tick != input.frame.tick()
    {
        return Err(ScaffoldContractError::BrainOwnershipMismatch);
    }
    Ok(())
}

fn validate_sensing_and_context(
    input: &ProductionCausalStepInput<'_>,
) -> Result<(), ScaffoldContractError> {
    input.frame.validate_contract()?;
    input.memory_recall.validate_for_frame(&input.frame)?;
    input
        .memory_recall
        .cognitive_context()
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    Ok(())
}

fn validate_gpu_decision_and_eligibility(
    session: &GpuAuthoritativeSession,
    input: &ProductionCausalStepInput<'_>,
) -> Result<(), ScaffoldContractError> {
    if input.gpu_tick.handle != input.handle
        || input.gpu_tick.base_digest != input.frame.base_digest()
        || input.gpu_tick.frame_digest != input.frame.frame_digest()
        || input.gpu_tick.hardware_receipt_generation != session.hardware_receipt().generation
    {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    Ok(())
}

fn map_motor_transaction_error(error: HeadlessMotorTransactionError) -> ScaffoldContractError {
    match error {
        HeadlessMotorTransactionError::Contract(error) => error,
        HeadlessMotorTransactionError::UnsupportedChannel(_) => {
            ScaffoldContractError::InvalidActionDecision
        }
    }
}

fn bounded_successor_scalar(value: f32) -> Result<f32, ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok((0.5 + 0.5 * (value / (1.0 + value.abs()))).clamp(0.0, 1.0))
}

fn unit_successor_scalar(value: f32) -> Result<f32, ScaffoldContractError> {
    if !value.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    Ok(value.clamp(0.0, 1.0))
}

fn grounded_semantic_state_from_frame(
    frame: &PerceptionFrame,
    canonical_biochemistry: &BiochemistryState,
) -> Result<SemanticStateVector, ScaffoldContractError> {
    let body = frame.body();
    grounded_semantic_state(
        body.pose.translation,
        body.velocity.linear,
        &canonical_biochemistry.body,
        &canonical_biochemistry.homeostasis,
    )
}

fn grounded_successor_state(
    world: &HeadlessWorld,
    world_entity_id: WorldEntityId,
    biology_after: &BiochemistryState,
) -> Result<SemanticStateVector, ScaffoldContractError> {
    let object = world
        .entity(world_entity_id)
        .ok_or(ScaffoldContractError::InvalidId)?;
    grounded_semantic_state(
        object.position,
        object.grounded_physical.velocity,
        &biology_after.body,
        &biology_after.homeostasis,
    )
}

fn grounded_semantic_state(
    position: Vec3f,
    velocity: Vec3f,
    body: &BodyState,
    homeostasis: &HomeostaticSnapshot,
) -> Result<SemanticStateVector, ScaffoldContractError> {
    let drives = homeostasis.drives.to_array();
    // This order is the append-only SemanticStateMeaning V2 prefix:
    // body position, body velocity, four drive lanes, then canonical body
    // energy/health/temperature/injury. Both pre and post call this builder.
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
        unit_successor_scalar(body.energy)?,
        unit_successor_scalar(body.health)?,
        unit_successor_scalar(body.temperature_stress)?,
        unit_successor_scalar(body.injury)?,
    ])
}

fn grounded_contact_intensity(contact: alife_core::PhysicalContactKind) -> f32 {
    match contact {
        alife_core::PhysicalContactKind::None => 0.0,
        alife_core::PhysicalContactKind::Touch => 0.2,
        alife_core::PhysicalContactKind::Collision => 0.4,
        alife_core::PhysicalContactKind::Blocked => 0.6,
        alife_core::PhysicalContactKind::Consumed => 0.8,
        alife_core::PhysicalContactKind::Moved => 1.0,
    }
}

fn grounded_outcome_features(
    physical: alife_core::PhysicalActionOutcome,
    succeeded: bool,
    body_event: alife_core::BodyEventDelta,
) -> Result<GroundedOutcomeFeatures, ScaffoldContractError> {
    GroundedOutcomeFeatures::from_parts(
        physical.displacement,
        grounded_contact_intensity(physical.contact),
        succeeded,
        body_event.damage,
        body_event.energy,
        0.0,
    )
}

fn prediction_action_sensitivity(
    predictor: &GroundedSuccessorPredictor,
    frame: &PerceptionFrame,
    organism_id: alife_core::OrganismId,
    sequence_id: ExperienceSequenceId,
    source_state: &SemanticStateVector,
    selected: &JointMotorCondition,
) -> Result<f32, ScaffoldContractError> {
    for candidate in frame.candidates().iter().take(MAX_PREDECISION_PREDICTIONS) {
        let command = candidate.to_command(organism_id, candidate.sensor_confidence)?;
        let bundle = compatibility_bundle_for_selected_action_v1(
            organism_id,
            sequence_id,
            frame.tick(),
            &command,
        )?;
        let other = JointMotorCondition::from_bundle(&bundle)?;
        if other.canonical_digest()? == selected.canonical_digest()? {
            continue;
        }
        if let Ok(evidence) = predictor.action_sensitivity(source_state, selected, &other) {
            return Ok(evidence.predicted_successor_distance.clamp(0.0, 1.0));
        }
    }
    Ok(0.0)
}

fn prediction_successor_separability(
    predictor: &GroundedSuccessorPredictor,
    frame: &PerceptionFrame,
    organism_id: alife_core::OrganismId,
    sequence_id: ExperienceSequenceId,
    source_state: &SemanticStateVector,
    selected: &JointMotorCondition,
) -> Result<f32, ScaffoldContractError> {
    let first = predictor.predict(source_state, selected)?;
    let first_state = SemanticStateVector::new(first.predicted_successor)?;
    for candidate in frame.candidates().iter().take(MAX_PREDECISION_PREDICTIONS) {
        let command = candidate.to_command(organism_id, candidate.sensor_confidence)?;
        let bundle = compatibility_bundle_for_selected_action_v1(
            organism_id,
            sequence_id,
            frame.tick(),
            &command,
        )?;
        let other = JointMotorCondition::from_bundle(&bundle)?;
        if other.canonical_digest()? == selected.canonical_digest()? {
            continue;
        }
        let second = predictor.predict(source_state, &other)?;
        let second_state = SemanticStateVector::new(second.predicted_successor)?;
        let evidence = predictor.successor_separability(&first_state, &second_state)?;
        if evidence.materially_different {
            return Ok(evidence.successor_distance.clamp(0.0, 1.0));
        }
    }
    Ok(0.0)
}

fn channel_command_for_action(
    channel: MotorChannel,
    command: &ActionCommand,
) -> Result<alife_core::ChannelCommand, ScaffoldContractError> {
    let target = (command.target_entity.is_some() || command.target_position.is_some())
        .then(|| ActionTarget::new(command.target_entity, command.target_position));
    alife_core::ChannelCommand::new(
        channel,
        command.action_id,
        target,
        command.target_position.unwrap_or(Vec3f::ZERO),
        command.intensity,
        command.duration_ticks,
        0.0,
        command.confidence,
        0,
    )
}

fn factorized_motor_channel_for_action(kind: ActionKind) -> Option<MotorChannel> {
    match kind {
        ActionKind::Move => Some(MotorChannel::Locomotion),
        ActionKind::Interact | ActionKind::Write => Some(MotorChannel::Manipulation),
        ActionKind::Vocalize => Some(MotorChannel::Vocal),
        ActionKind::Hold | ActionKind::Rest | ActionKind::Inspect => Some(MotorChannel::Posture),
        ActionKind::Idle | ActionKind::Gesture => None,
    }
}

fn compatibility_bundle_for_selected_action_v1(
    organism_id: alife_core::OrganismId,
    sequence_id: ExperienceSequenceId,
    tick: alife_core::Tick,
    command: &ActionCommand,
) -> Result<MotorCommandBundle, ScaffoldContractError> {
    debug_assert_eq!(SINGLE_ACTION_COMPATIBILITY_ADAPTER_VERSION, 1);
    let channel = match command.kind {
        ActionKind::Idle | ActionKind::Hold | ActionKind::Rest | ActionKind::Inspect => {
            MotorChannel::Posture
        }
        ActionKind::Move => MotorChannel::Locomotion,
        ActionKind::Interact | ActionKind::Write => MotorChannel::Manipulation,
        ActionKind::Vocalize => MotorChannel::Vocal,
        ActionKind::Gesture => MotorChannel::Posture,
    };
    let channel_command = channel_command_for_action(channel, command)?;
    MotorCommandBundle::new(organism_id, sequence_id, tick, vec![channel_command])
}

fn factorized_motor_bundle_for_candidates(
    organism_id: alife_core::OrganismId,
    sequence_id: ExperienceSequenceId,
    tick: alife_core::Tick,
    frame: &PerceptionFrame,
    candidate_slots: [u16; GPU_MOTOR_CHANNEL_SLOT_COUNT],
    channels: &[MotorChannel],
    compatibility_command: &ActionCommand,
    selected_candidate_index: u16,
    speech_payload: Option<&SpeechMotorPayload>,
    speech_prompted: bool,
) -> Result<MotorCommandBundle, ScaffoldContractError> {
    let mut channel_commands = Vec::with_capacity(channels.len());
    for head_channel in channels {
        let slot = match head_channel {
            MotorChannel::Locomotion => 0,
            MotorChannel::Orientation => 1,
            MotorChannel::Manipulation => 2,
            MotorChannel::Vocal => 3,
            MotorChannel::Posture => 4,
            MotorChannel::SpeciesSpecific(_) => 5,
        };
        let encoded = candidate_slots
            .get(slot)
            .copied()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if encoded == 0 {
            continue;
        }
        let candidate_index = encoded - 1;
        let candidate = *frame
            .candidates()
            .get(usize::from(candidate_index))
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let command = candidate.to_command(organism_id, candidate.sensor_confidence)?;
        let channel = factorized_motor_channel_for_action(command.kind)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if channel != *head_channel {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let mut channel_command = channel_command_for_action(channel, &command)?;
        if channel == MotorChannel::Vocal && candidate_index == selected_candidate_index {
            if let Some(payload) = speech_payload {
                let mut values = Vec::with_capacity(payload.tokens.len() + 4);
                values.push(VOCAL_CHANNEL_PAYLOAD_MAGIC_V1);
                values.push(u32::from(payload.speech_act.raw()));
                values.push(if speech_prompted { 1 } else { 0 });
                values.push((payload.confidence.raw() * 65_535.0).round() as u32);
                values.extend(payload.tokens.iter().map(|token| u32::from(token.raw())));
                channel_command =
                    channel_command.with_payload(BoundedMotorPayload::new(values)?)?;
            }
        }
        channel_commands.push(channel_command);
    }
    if channel_commands.is_empty() {
        return compatibility_bundle_for_selected_action_v1(
            organism_id,
            sequence_id,
            tick,
            compatibility_command,
        );
    }
    let coordination = (channel_commands.len() > 1).then(|| BoundedCoordinationSummary {
        groups: vec![alife_core::CoordinationGroup {
            group_id: 0,
            channels: channel_commands
                .iter()
                .map(|command| command.channel)
                .collect(),
        }],
    });
    let bundle = MotorCommandBundle::new(organism_id, sequence_id, tick, channel_commands)?;
    if let Some(coordination) = coordination {
        bundle.with_coordination(coordination)
    } else {
        Ok(bundle)
    }
}

fn apply_prediction_evidence(
    context: &mut CognitiveContextFrame,
    target: &PredictionTargetReceipt,
    errors: &[f32],
    selected_target: Option<StableFocusIdentity>,
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
    context.prediction.action_sensitivity =
        NormalizedScalar::new(target.action_sensitivity_score.clamp(0.0, 1.0))?;
    let uncertainty = NormalizedScalar::new(mean_absolute_error)?;
    for summary in &mut context.attention.peripheral_summaries {
        if selected_target == Some(summary.identity) {
            summary.salience.uncertainty =
                NormalizedScalar::new(summary.salience.uncertainty.raw().max(mean_absolute_error))?;
            summary.salience.gap_voltage =
                NormalizedScalar::new(summary.salience.gap_voltage.raw().max(mean_absolute_error))?;
        }
    }
    for (index, salience) in context.attention.salience_components.iter_mut().enumerate() {
        if context.focal.identities.get(index).copied() == selected_target {
            salience.uncertainty = uncertainty;
            salience.gap_voltage =
                NormalizedScalar::new(salience.gap_voltage.raw().max(mean_absolute_error))?;
        }
    }
    context.peripheral.summaries = context.attention.peripheral_summaries.clone();
    context.focal.salience = context.attention.salience_components.clone();
    context.validate_contract()?;
    Ok(mean_absolute_error)
}

fn candidate_target_identity(
    frame: &PerceptionFrame,
    candidate: &alife_core::ActionCandidate,
) -> Option<StableFocusIdentity> {
    match candidate.observation {
        alife_core::CandidateObservationRef::ObjectSlot(slot) => frame
            .grounded_object_slots()
            .get(usize::from(slot))
            .map(|object| StableFocusIdentity::TrackedObject(object.tracked_object_id)),
        alife_core::CandidateObservationRef::None => None,
    }
}

fn cognitive_work_receipt(
    context: &CognitiveContextFrame,
    memory: &MemoryRecallReceipt,
    neural_work: &BrainWorkCounters,
    v11_work: &GpuV11WorkReceipt,
    prediction_ops: u64,
) -> Result<CognitiveWorkReceipt, ScaffoldContractError> {
    let memory_ops = u64::from(memory.exact_bucket_reads)
        .saturating_add(u64::from(memory.neighbor_bucket_reads))
        .saturating_add(u64::from(memory.similarity_evaluations));
    CognitiveWorkCounters::new(
        neural_work.neuron_updates,
        neural_work.synapse_ops,
        v11_work.cognitive.dendritic_ops,
        context.budget.work_used,
        memory_ops,
        context.concept.active_concepts.len() as u64,
        context.gap.active_gaps.len() as u64,
        prediction_ops,
        0,
        v11_work.cognitive.structural_ops,
        1,
        0,
    )?
    .into_receipt()
}

fn apply_cognitive_work_cost(
    world: &mut HeadlessWorld,
    organism_id: alife_core::OrganismId,
    receipt: CognitiveWorkReceipt,
    policy: &alife_core::cognitive_work::CognitiveWorkCostPolicy,
) -> Result<(), ScaffoldContractError> {
    let mut records = world
        .organism_registry()
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let record = records
        .iter_mut()
        .find(|record| record.organism_id() == organism_id)
        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
    record
        .account_cognitive_work(receipt, *policy)
        .map_err(|error| match error {
            alife_world::OrganismRegistryError::InvalidRecord(error) => error,
            _ => ScaffoldContractError::InvalidDecisionEvidence,
        })?;
    world.replace_organism_registry_exact(records)
}
