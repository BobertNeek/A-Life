//! Canonical CPU collector. Biology, legality, perception, sleep and episodic
//! evidence are organism-owned; only founder parameters/optimizer are shared.
use super::*;
use alife_core::cognitive_work::{CognitiveWorkCostPolicy, CognitiveWorkCounters};
use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::sleep::{
    ReplayEligibilitySample, ReplaySynapseSpan, SleepController, SleepReplayEvent,
    SleepReplayEvidence,
};
use alife_core::*;
use alife_gpu_backend::{GpuV11CausalState, GpuV11Checkpoint};
use alife_runtime::canonical_preparation::*;
use alife_world::{
    AssetManifest, BrainPolicyConfig, CanonicalNewGame, CreatureSaveState, HeadlessWorld,
    PortableSaveFile, RuntimeConfig,
};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum CpuHostError {
    #[error(transparent)]
    Organism(#[from] alife_world::OrganismRegistryError),
    #[error(transparent)]
    Training(#[from] crate::TrainingError),
    #[error(transparent)]
    Contract(#[from] ScaffoldContractError),
    #[error(transparent)]
    Persistence(#[from] alife_world::PersistenceError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("CPU canonical host: {0}")]
    State(&'static str),
    #[error("CPU episode preparation: {0}")]
    Preparation(String),
}
#[derive(Clone)]
struct Resident {
    maze_blocked2346: Option<MazeBlockedHistory2346>,
    active_approach_intent2327: Option<ActiveApproachIntent2327>,
    joint_comparison_mode: Option<u16>,
    memory_readout: Option<FrozenMemoryReadoutConstructionV1>,
    memory_readout_digest: Option<[u64; 4]>,
    #[cfg(test)]
    last_terminal_capture: Option<(FinalizedMemoryRecall, ExperiencePatch)>,
    terminal_channel2135: Option<TerminalChannelState2135>,
    // Process-local proof of the last completed boundary. Private state is
    // invalidated before mutation and resealed before a staged host is committed.
    // Never restored from serialized data; cold admission recomputes the digest.
    sealed_authority: Option<ResidentAuthority>,
    neural: CpuOrganismState,
    activity: CpuActivityState,
    v11: GpuV11CausalState,
    memory: MemorySidecarState,
    topology: TopologySidecar,
    predictor: GroundedSuccessorPredictor,
    sleep: SleepController,
    hysteresis: HysteresisState,
    next_sequence: u64,
    replay: Vec<CpuReplayEvent>,
    language_grounding: LanguageGroundingLedger,
}
#[derive(Clone, Copy)]
struct ResidentAuthority {
    brain: [u64; 4],
    memory: [u64; 4],
    sampling_seed: u32,
    sampling_counter: u32,
}
impl ResidentAuthority {
    fn validate(
        self,
        record: &alife_world::WorldOrganismRecord,
        sampling_seed: u32,
        sampling_counter: u32,
    ) -> Result<(), CpuHostError> {
        if self.sampling_seed != sampling_seed
            || self.sampling_counter != sampling_counter
            || record.state_graph().brain.content_digest != self.brain
            || record.state_graph().memory.content_digest != self.memory
        {
            return Err(CpuHostError::State(
                "canonical organism authority differs from private CPU state",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CpuReplayEvent {
    event: SleepReplayEvent,
    eligibility: Vec<i16>,
    target: PredictionTargetReceipt,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuResidentCheckpoint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maze_blocked2346: Option<MazeBlockedHistory2346>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_approach_intent2327: Option<ActiveApproachIntent2327>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_readout: Option<FrozenMemoryReadoutConstructionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_channel2135: Option<TerminalChannelState2135>,
    pub neural: CpuOrganismCheckpoint,
    pub activity: CpuActivityState,
    pub neural_topology: GpuV11Checkpoint,
    pub memory: PortableMemoryBankAssetV2,
    pub memory_compaction: MemoryCompactionCheckpoint,
    pub staged_memory: Option<PortableMemoryBankAssetV2>,
    pub topology: PortableTopologySidecarAssetV1,
    pub predictor: GroundedSuccessorPredictor,
    pub sleep: SleepController,
    pub hysteresis: HysteresisState,
    pub next_sequence: u64,
    pub language_grounding: LanguageGroundingLedger,
    replay: Vec<CpuReplayEvent>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuCanonicalCheckpoint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maze_program2346: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_opportunity_coverage2313: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joint_membership2177: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_context2135: Option<TerminalContextConfig2135>,
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub founder_weight_digest: [u64; 4],
    pub sampling_seed: u32,
    pub sampling_counter: u32,
    pub world: PortableSaveFile,
    /// CPU saves have no GPU brain sidecar to carry world perception identity.
    /// Version1 omitted this registry and cannot establish exact object continuity.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracked_objects: Vec<alife_world::TrackedObjectRegistrySaveState>,
    pub residents: Vec<CpuResidentCheckpoint>,
}
/// Complete in-memory pre-success branch state. Restoring it restores the world,
/// chemistry, memories, predictor, neural traces, sleep, sequence and RNG counter.
#[derive(Clone)]
pub struct CpuCanonicalHost {
    maze_program2346: Option<u16>,
    candidate_opportunity_coverage2313: Option<u16>,
    #[cfg(test)]
    fail_joint_membership_after_admission: bool,
    #[cfg(test)]
    fail_ingest_outcome_binding2286: bool,
    joint_membership2177: Option<u16>,
    terminal_context2135: Option<TerminalContextConfig2135>,
    world: HeadlessWorld,
    creatures: Vec<CreatureSaveState>,
    phenotype_hash: PhenotypeHash,
    founder_weight_digest: [u64; 4],
    residents: BTreeMap<u64, Resident>,
    sampling_seed: u32,
    sampling_counter: u32,
}
#[derive(Clone)]
pub struct CpuCollectedDecision {
    pub active_approach_opportunity2327: Option<ActiveApproachOpportunityRow2327>,
    pub approach_attention_feedback2325: Option<ApproachAttentionFeedback2325>,
    /// Present only for the versioned plan-2 JSON wire; not an auxiliary reward.
    pub memory_plan2242: Option<MemoryPlanRow2242>,
    pub joint_membership2177: Option<JointMembershipLiveReceipt2177>,
    // Test-only capture from the exact call which charges this real native step.
    // Not persisted, imported, reported as component-specific physiology, or
    // included in diagnostic serialization.
    #[cfg(test)]
    pub(super) cognitive_cost_capture: Option<(CognitiveWorkCostPolicy, f32, f32, f32)>,
    #[cfg(test)]
    pub(super) cognitive_body_cost_capture: Option<(BodyState, BodyState)>,
    /// Explicitly requested cognitive action-phase/input facts, candidate aligned.
    /// Ordinary preparation does not request these for reporting. Offboard viewers
    /// use MemorySidecarState::inspect_execution_evidence instead. These are not
    /// isolated physiological effects or a policy override.
    pub executed_action_recall: Option<Vec<Option<alife_core::ExecutedActionRecallV1>>>,
    /// Already included once in patch cognitive memory_ops, not an extra debit.
    pub executed_action_recall_work: u64,
    pub memory_readout_research: Option<CpuMemoryReadoutReceiptV1>,
    pub terminal_context2135: Option<TerminalContextReceipt2135>,
    pub organism_id: OrganismId,
    /// Exact native sampling counter. Older diagnostic rows lack this evidence;
    /// they remain readable but cannot be admitted as cooperative shards.
    pub sampling_counter: Option<u32>,
    pub initial: crate::TrainingInitialState,
    pub replay_tick: crate::TrainingReplayTick,
    pub action: crate::PpoJointAction,
    pub final_features: Vec<f32>,
    pub brain_work: BrainWorkCounters,
    pub biological_reward: f32,
    pub patch: ExperiencePatch,
}

/// Final opportunity proof plus authenticated intent transition. No inferred reward.
#[derive(Debug,Clone,serde::Serialize,serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveApproachOpportunityRow2327 {
    pub frame_digest: PerceptionFrameDigest,
    pub before: Option<ActiveApproachIntent2327>,
    pub after: Option<ActiveApproachIntent2327>,
    pub selection: alife_world::ActiveApproachOpportunityReceipt,
    /// Actual intent reads/copies/update, separately from selection work.
    pub intent_work_units: u64,
}

/// Authenticated memory of one actual successful selected Approach, not a goal.
#[derive(Debug,Clone,PartialEq,serde::Serialize,serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveApproachIntent2327 {
    pub source_frame_digest: PerceptionFrameDigest,
    pub source_tick: Tick,
    pub organism_id: OrganismId,
    pub selected_candidate: u16,
    pub executed: ExecutedChannelEvidenceV1,
    pub tracked_object_id: TrackedObjectId,
    pub target_entity: WorldEntityId,
}
impl ActiveApproachIntent2327 {
    fn validate(&self)->Result<(),CpuHostError>{
        self.organism_id.validate()?;self.tracked_object_id.validate()?;self.target_entity.validate()?;self.executed.validate_contract()?;
        if self.selected_candidate as usize>=MAX_ACTION_CANDIDATES || self.executed.primitive.raw()!=101 || !self.executed.succeeded || self.executed.command_target!=Some(self.target_entity){return Err(CpuHostError::State("invalid active Approach source"));}Ok(())
    }
}

/// Native post-execution attention feedback, never a forced action or reward.
/// Work is a logical operation tariff: twelve fixed receipt field copies, plus
/// each actual option/index lookup, predicate, visited channel/slot and state write.
/// It is included once in focal-target work, not an additional energy debit.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApproachAttentionFeedback2325 {
    pub schema_version: u16,
    pub frame_digest: PerceptionFrameDigest,
    pub organism_id: OrganismId,
    pub source_tick: Tick,
    pub selected_candidate: Option<u16>,
    pub executed: Option<ExecutedChannelEvidenceV1>,
    pub tracked_object_id: Option<TrackedObjectId>,
    pub target_entity: Option<WorldEntityId>,
    pub before: HysteresisState,
    pub after: HysteresisState,
    pub nominated: bool,
    pub work_units: u64,
}
fn approach_attention_feedback(
    frame: &PerceptionFrame,
    action: &crate::PpoJointAction,
    joint: &JointPhysicalOutcome,
    world: &HeadlessWorld,
    before: HysteresisState,
) -> Result<ApproachAttentionFeedback2325, CpuHostError> {
    let mut r = ApproachAttentionFeedback2325 {
        schema_version: 1, frame_digest: frame.frame_digest(), organism_id: frame.organism_id(),
        source_tick: frame.tick(), selected_candidate: None, executed: None,
        tracked_object_id: None, target_entity: None, before, after: before,
        nominated: false, work_units: 12,
    };
    r.work_units += 1; // selected locomotion option
    let Some(index) = action.motor_candidates[0] else { return Ok(r); };
    r.selected_candidate = Some(index); r.work_units += 2; // receipt write and candidate lookup
    let candidate = frame.candidates().get(index as usize).ok_or(CpuHostError::State("feedback candidate missing"))?;
    r.work_units += 1;
    if candidate.family != CandidateActionFamily::Approach { return Ok(r); }
    r.work_units += 1;
    if candidate.action_id.raw() != 101 { return Ok(r); }
    for channel in &joint.channel_outcomes {
        r.work_units += 2; // channel visit and identity comparison
        if channel.channel != MotorChannel::Locomotion { continue; }
        r.work_units += 1;
        let Some(executed) = channel.execution_evidence else { return Ok(r); };
        r.executed = Some(executed); r.work_units += 2; // receipt copy and success test
        if !executed.succeeded { return Ok(r); }
        r.work_units += 1;
        if executed.primitive != candidate.action_id { return Err(CpuHostError::State("feedback executed primitive mismatch")); }
        r.work_units += 1;
        let Some(target) = candidate.target.entity else { return Ok(r); };
        r.target_entity = Some(target); r.work_units += 2; // receipt write and identity check
        if executed.command_target != Some(target) { return Err(CpuHostError::State("feedback executed target mismatch")); }
        r.work_units += 1;
        let CandidateObservationRef::ObjectSlot(slot_index) = candidate.observation else { return Ok(r); };
        for slot in frame.grounded_object_slots() {
            r.work_units += 2;
            if slot.slot_index != slot_index { continue; }
            r.work_units += 1;
            let Some(object) = world.entity(target) else { return Ok(r); };
            r.work_units += 1;
            if object.consumed { return Ok(r); }
            r.tracked_object_id = Some(slot.tracked_object_id); r.work_units += 2; // receipt write and incumbent compare
            let identity = Some(StableFocusIdentity::TrackedObject(slot.tracked_object_id));
            if r.after.previous_identity != identity {
                r.after.previous_identity = identity;
                r.after.retained_ticks = 1;
                r.work_units += 2;
            }
            r.nominated = true;
            r.work_units += 2; // nominated receipt and resident-state assignment
            return Ok(r);
        }
        return Ok(r);
    }
    Ok(r)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemoryPlanRow2242 {
    pub phenotype_hash: PhenotypeHash,
    pub memory_bound: f32,
}
#[derive(Clone, serde::Deserialize)]
struct CollectedDecisionFlat2242 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active_approach_opportunity2327: Option<ActiveApproachOpportunityRow2327>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    approach_attention_feedback2325: Option<ApproachAttentionFeedback2325>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joint_membership2177: Option<JointMembershipLiveReceipt2177>,
    // Test-only capture from the exact call which charges this real native step.
    // Not persisted, imported, reported as component-specific physiology, or
    // included in diagnostic serialization.
    #[cfg(test)]
    #[serde(skip)]
    pub(super) cognitive_cost_capture: Option<(CognitiveWorkCostPolicy, f32, f32, f32)>,
    /// Explicitly requested cognitive action-phase/input facts, candidate aligned.
    /// Ordinary preparation does not request these for reporting. Offboard viewers
    /// use MemorySidecarState::inspect_execution_evidence instead. These are not
    /// isolated physiological effects or a policy override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executed_action_recall: Option<Vec<Option<alife_core::ExecutedActionRecallV1>>>,
    /// Already included once in patch cognitive memory_ops, not an extra debit.
    #[serde(default, skip_serializing_if = "execution_work_is_zero")]
    pub executed_action_recall_work: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_readout_research: Option<CpuMemoryReadoutReceiptV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_context2135: Option<TerminalContextReceipt2135>,
    pub organism_id: OrganismId,
    /// Exact native sampling counter. Older diagnostic rows lack this evidence;
    /// they remain readable but cannot be admitted as cooperative shards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling_counter: Option<u32>,
    pub initial: crate::TrainingInitialState,
    pub replay_tick: crate::TrainingReplayTick,
    pub action: crate::PpoJointAction,
    pub final_features: Vec<f32>,
    pub brain_work: BrainWorkCounters,
    pub biological_reward: f32,
    pub patch: ExperiencePatch,
}
impl From<CollectedDecisionFlat2242> for CpuCollectedDecision {
    fn from(flat: CollectedDecisionFlat2242) -> Self {
        Self {
            memory_plan2242: None,
            active_approach_opportunity2327: flat.active_approach_opportunity2327,
            approach_attention_feedback2325: flat.approach_attention_feedback2325,
            joint_membership2177: flat.joint_membership2177,
            #[cfg(test)]
            cognitive_cost_capture: flat.cognitive_cost_capture,
            #[cfg(test)]
            cognitive_body_cost_capture: None,
            executed_action_recall: flat.executed_action_recall,
            executed_action_recall_work: flat.executed_action_recall_work,
            memory_readout_research: flat.memory_readout_research,
            terminal_context2135: flat.terminal_context2135,
            organism_id: flat.organism_id,
            sampling_counter: flat.sampling_counter,
            initial: flat.initial,
            replay_tick: flat.replay_tick,
            action: flat.action,
            final_features: flat.final_features,
            brain_work: flat.brain_work,
            biological_reward: flat.biological_reward,
            patch: flat.patch,
        }
    }
}
impl CpuCollectedDecision {
    pub(super) fn validate_memory_plan2242(
        &self,
        phenotype: &alife_core::BrainPhenotype,
    ) -> Result<(), crate::TrainingError> {
        self.patch.validate_contract()?;
        if self.patch.decision().neural_evidence()?.phenotype_hash != phenotype.phenotype_hash() {
            return Err(super::invalid());
        }
        match (
            phenotype
                .candidate_decoder()
                .memory_channel()
                .filter(|p| p.schema_version() == 2),
            &self.memory_plan2242,
        ) {
            (Some(plan), Some(record))
                if record.phenotype_hash == phenotype.phenotype_hash()
                    && record.memory_bound.to_bits() == plan.max_candidate_gain().to_bits() =>
            {
                Ok(())
            }
            (None, None) => Ok(()),
            _ => Err(super::invalid()),
        }
    }
}
impl<'de> serde::Deserialize<'de> for CpuCollectedDecision {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if !d.is_human_readable() {
            return CollectedDecisionFlat2242::deserialize(d).map(Into::into);
        }
        let value = serde_json::Value::deserialize(d)?;
        if value.get("memory_plan2_record_v1").is_some() {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Envelope {
                memory_plan2_record_v1: Payload,
            }
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Payload {
                plan: MemoryPlanRow2242,
                row: CollectedDecisionFlat2242,
            }
            let envelope: Envelope =
                serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            let payload = envelope.memory_plan2_record_v1;
            let mut row: Self = payload.row.into();
            let neural = row
                .patch
                .decision()
                .neural_evidence()
                .map_err(serde::de::Error::custom)?;
            if !payload.plan.memory_bound.is_finite()
                || payload.plan.memory_bound < 0.0
                || neural.phenotype_hash != payload.plan.phenotype_hash
            {
                return Err(serde::de::Error::custom(
                    "invalid plan-2 collected row identity",
                ));
            }
            row.memory_plan2242 = Some(payload.plan);
            Ok(row)
        } else {
            serde_json::from_value::<CollectedDecisionFlat2242>(value)
                .map(Into::into)
                .map_err(serde::de::Error::custom)
        }
    }
}

/// Shared-event research context and actual/reserved work, not isolated credit.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointMembershipLiveReceipt2177 {
    pub schema_version: u16,
    pub sources: Vec<alife_core::memory::JointMembershipSourceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison: Option<JointMembershipComparison2182>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success_projection2275: Option<JointContactSuccessProjection2275>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingest_outcome_projection2286: Option<JointIngestOutcomeProjection2286>,
    pub query_work: u64,
    pub admission_planning_work: u64,
    pub admission_reserved_work: u64,
    pub admission_actual_work: u64,
}

/// Queried evidence is not necessarily applied to the decision context.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointMembershipComparison2182 {
    pub apply: bool,
    /// Absent on historical all-family comparison rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_family2266: Option<CandidateActionFamily>,
    /// Explicit all-but-Ingest mask; absent on modes 0–9.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_family_mask2278: Option<u8>,
    pub queried_sources: Vec<alife_core::memory::JointMembershipSourceV1>,
}

/// Explicit CPU decoder policy. Raw joint recall and cognitive context are not
/// rewritten: only the ordinary success component is projected to Contact 33.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointContactSuccessProjection2275 {
    pub apply: bool,
    pub ordinary_query: MemoryRecallReceipt,
    pub work: u64,
    pub candidates: Vec<JointContactSuccessCandidate2275>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointContactSuccessCandidate2275 {
    pub candidate_index: u16,
    pub ordinary_success: f32,
    pub ordinary_confidence: f32,
    pub ordinary_source_count: u16,
    pub ordinary_best_source: Option<MemoryId>,
    pub ordinary_sources: MemoryFamilySourcesV1,
    pub joint_weighted_success: f32,
    pub projected_weighted_success: f32,
    pub output_weighted_success: f32,
}

fn project_joint_contact_success2275(
    frame: &PerceptionFrame,
    joint: &FinalizedMemoryRecall,
    ordinary_frame: &PerceptionFrame,
    ordinary: &FinalizedMemoryRecall,
    tick: &mut crate::TrainingReplayTick,
    apply: bool,
) -> Result<JointContactSuccessProjection2275, CpuHostError> {
    // Each recall validates against its OWN finalized context. Cross-binding
    // never accepts an arbitrary scalar or a basis from another private bank.
    joint.validate_for_frame(frame)?;
    ordinary.validate_for_frame(ordinary_frame)?;
    let a = joint.receipt();
    let b = ordinary.receipt();
    let sources = ordinary
        .family_sources()
        .ok_or(CpuHostError::State("projection lacks ordinary provenance"))?;
    if frame.base_digest() != ordinary_frame.base_digest()
        || frame.organism_id() != ordinary_frame.organism_id()
        || frame.tick() != ordinary_frame.tick()
        || a.bank_digest != b.bank_digest
        || a.input_generation != b.input_generation
        || a.organism_id_raw != b.organism_id_raw
        || joint.candidate_keys().len() != ordinary.candidate_keys().len()
        || joint
            .candidate_keys()
            .iter()
            .zip(ordinary.candidate_keys())
            .any(|(a, b)| a.query() != b.query())
        || frame.candidates().len() != tick.candidates.len()
        || sources.len() != tick.candidates.len()
        || joint.context().candidates.len() != tick.candidates.len()
        || ordinary.context().candidates.len() != tick.candidates.len()
    {
        return Err(CpuHostError::State(
            "projection ordinary/joint binding mismatch",
        ));
    }
    let mut candidates = Vec::new();
    // Logical tariff: binding checks, two immutable recall validations/context
    // reads, one candidate family check, component selection/copy, and complete
    // source provenance copying. Both apply/withhold perform the same assignment.
    let mut work = 16 + 4 * joint.context().candidates.len() as u64 * 16;
    for (i, (native, encoded)) in frame
        .candidates()
        .iter()
        .zip(&mut tick.candidates)
        .enumerate()
    {
        if native.family != encoded.family {
            return Err(CpuHostError::State("projection candidate family mismatch"));
        }
        work += 1;
        if native.family != CandidateActionFamily::Contact {
            continue;
        }
        let c = &ordinary.context().candidates[i];
        let j = &joint.context().candidates[i];
        let joint_value = j.family_value[1] * j.family_confidence.raw().clamp(0.0, 1.0);
        if c.candidate_index as usize != i
            || j.candidate_index as usize != i
            || encoded.decoder_inputs[33].to_bits() != joint_value.to_bits()
        {
            return Err(CpuHostError::State(
                "projection candidate component mismatch",
            ));
        }
        let value = c.family_value[1] * c.family_confidence.raw().clamp(0.0, 1.0);
        let output = if apply { value } else { joint_value };
        work += 12 + 16 * sources[i].sources.iter().flatten().count() as u64;
        candidates.push(JointContactSuccessCandidate2275 {
            candidate_index: c.candidate_index,
            ordinary_success: c.family_value[1],
            ordinary_confidence: c.family_confidence.raw(),
            ordinary_source_count: c.family_source_count,
            ordinary_best_source: c.best_family_source,
            ordinary_sources: sources[i].clone(),
            joint_weighted_success: joint_value,
            projected_weighted_success: value,
            output_weighted_success: output,
        });
        encoded.decoder_inputs[33] = output;
    }
    Ok(JointContactSuccessProjection2275 {
        apply,
        ordinary_query: b.clone(),
        work,
        candidates,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointIngestOutcomeProjection2286 {
    pub apply: bool,
    pub component_mask: u8,
    pub cost_revision: u16,
    pub requires_positive_ordinary_support: bool,
    pub ordinary: OrdinaryIngestOutcomeEvidence2286,
    pub actual_cpu_ops: u64,
    pub candidates: Vec<JointIngestOutcomeCandidate2286>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JointIngestOutcomeCandidate2286 {
    pub candidate_index: u16,
    pub support_eligible: bool,
    pub applied: bool,
    pub joint_values: [f32; 2],
    pub joint_confidence: f32,
    pub joint_source_count: u16,
    pub joint_best_source: Option<MemoryId>,
    pub joint_weighted: [f32; 2],
    pub ordinary_weighted: [f32; 2],
    pub output_weighted: [f32; 2],
}
fn project_joint_ingest_outcome2286(
    frame: &PerceptionFrame,
    joint: &FinalizedMemoryRecall,
    basis: OrdinaryIngestOutcomeBasis2286,
    tick: &mut crate::TrainingReplayTick,
    apply: bool,
) -> Result<JointIngestOutcomeProjection2286, CpuHostError> {
    if tick.candidates.len() != frame.candidates().len() {
        return Err(CpuHostError::State(
            "Ingest projection candidate count mismatch",
        ));
    }
    let mut actual = basis.validate_for_recall(frame, joint)?;
    let ordinary = basis.into_evidence();
    let mut candidates = Vec::with_capacity(ordinary.rows.len());
    for row in &ordinary.rows {
        let i = usize::from(row.candidate_index);
        let encoded = &mut tick.candidates[i];
        actual += 1; // Executed destination-family binding event.
        if encoded.family != CandidateActionFamily::Ingest {
            return Err(CpuHostError::State(
                "Ingest projection destination mismatch",
            ));
        }
        let eligible = row.source_count > 0 && row.confidence > 0.0 && row.provenance_valid;
        let apply_row = apply && eligible;
        actual += 1; // Actual fixed support-gate event in both arms.
        if eligible != row.support_eligible {
            return Err(CpuHostError::State("Ingest support eligibility mismatch"));
        }
        let raw = &joint.context().candidates[i];
        let mut old = [0.; 2];
        let mut weighted = [0.; 2];
        let mut output = [0.; 2];
        for (component, lane) in [32, 35].into_iter().enumerate() {
            old[component] = encoded.decoder_inputs[lane];
            actual += 1; // component read
            weighted[component] = row.values[component] * row.confidence;
            actual += 1; // product
            output[component] = if apply_row {
                weighted[component]
            } else {
                old[component]
            };
            actual += 1;
            encoded.decoder_inputs[lane] = output[component];
            actual += 1; // assignment even withheld
            actual += 1; // emitted component receipt event below
        }
        candidates.push(JointIngestOutcomeCandidate2286 {
            candidate_index: row.candidate_index,
            support_eligible: eligible,
            applied: apply_row,
            joint_values: [raw.family_value[0], raw.family_value[3]],
            joint_confidence: raw.family_confidence.raw(),
            joint_source_count: raw.family_source_count,
            joint_best_source: raw.best_family_source,
            joint_weighted: old,
            ordinary_weighted: weighted,
            output_weighted: output,
        });
    }
    if actual != ordinary.work.cpu_transform {
        return Err(CpuHostError::State(
            "Ingest projection actual/precharged work mismatch",
        ));
    }
    Ok(JointIngestOutcomeProjection2286 {
        apply,
        component_mask: 9,
        cost_revision: 1,
        requires_positive_ordinary_support: true,
        ordinary,
        actual_cpu_ops: actual,
        candidates,
    })
}

fn charge_joint_membership_work(
    activity: &mut CpuActivityBudget,
    work: u64,
) -> Result<(), CpuHostError> {
    activity.work.memory_context_ops = activity
        .work
        .memory_context_ops
        .checked_add(work)
        .ok_or(CpuHostError::State("joint membership work overflow"))?;
    let policy = BrainActivityPolicyV1::production_v1();
    activity.debit_atp_q16 = policy
        .cost
        .q24_to_atp_q16_round_half_up(policy.cost.neural_cost_q24(&activity.work)?)?;
    activity.affordable = activity.debit_atp_q16 <= activity.available_atp_q16;
    if !activity.affordable {
        return Err(CpuHostError::State("joint membership work exceeds ATP"));
    }
    Ok(())
}

fn execution_work_is_zero(work: &u64) -> bool {
    *work == 0
}

#[path = "terminal_row_wire.rs"]
mod terminal_row_wire;

impl serde::Serialize for CpuCollectedDecision {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Preserve the original flat field order and omissions without cloning
        // resident-derived arrays. Plan-2 envelopes support matching-reader training;
        // the existing auxiliary research modes remain diagnostic-only.
        #[derive(serde::Serialize)]
        struct Flat<'a> {
            #[serde(skip_serializing_if = "Option::is_none")]
            active_approach_opportunity2327: &'a Option<ActiveApproachOpportunityRow2327>,
            #[serde(skip_serializing_if = "Option::is_none")]
            approach_attention_feedback2325: &'a Option<ApproachAttentionFeedback2325>,
            #[serde(skip_serializing_if = "Option::is_none")]
            joint_membership2177: &'a Option<JointMembershipLiveReceipt2177>,
            #[serde(skip_serializing_if = "Option::is_none")]
            executed_action_recall: &'a Option<Vec<Option<alife_core::ExecutedActionRecallV1>>>,
            #[serde(skip_serializing_if = "execution_work_is_zero")]
            executed_action_recall_work: u64,
            #[serde(skip_serializing_if = "Option::is_none")]
            memory_readout_research: &'a Option<CpuMemoryReadoutReceiptV1>,
            #[serde(skip_serializing_if = "Option::is_none")]
            terminal_context2135: &'a Option<TerminalContextReceipt2135>,
            organism_id: &'a OrganismId,
            #[serde(skip_serializing_if = "Option::is_none")]
            sampling_counter: &'a Option<u32>,
            initial: &'a crate::TrainingInitialState,
            replay_tick: &'a crate::TrainingReplayTick,
            action: &'a crate::PpoJointAction,
            final_features: &'a Vec<f32>,
            brain_work: &'a BrainWorkCounters,
            biological_reward: &'a f32,
            patch: &'a ExperiencePatch,
        }
        let flat = Flat {
            active_approach_opportunity2327: &self.active_approach_opportunity2327,
            approach_attention_feedback2325: &self.approach_attention_feedback2325,
            joint_membership2177: &self.joint_membership2177,
            executed_action_recall: &self.executed_action_recall,
            executed_action_recall_work: self.executed_action_recall_work,
            memory_readout_research: &self.memory_readout_research,
            terminal_context2135: &self.terminal_context2135,
            organism_id: &self.organism_id,
            sampling_counter: &self.sampling_counter,
            initial: &self.initial,
            replay_tick: &self.replay_tick,
            action: &self.action,
            final_features: &self.final_features,
            brain_work: &self.brain_work,
            biological_reward: &self.biological_reward,
            patch: &self.patch,
        };
        if let Some(plan) = &self.memory_plan2242 {
            if !serializer.is_human_readable() {
                return Err(serde::ser::Error::custom(
                    "plan-2 collected rows require the versioned JSON envelope",
                ));
            }
            #[derive(serde::Serialize)]
            struct Payload<'a, T> {
                plan: &'a MemoryPlanRow2242,
                row: &'a T,
            }
            #[derive(serde::Serialize)]
            struct Envelope<'a, T> {
                memory_plan2_record_v1: Payload<'a, T>,
            }
            return Envelope {
                memory_plan2_record_v1: Payload { plan, row: &flat },
            }
            .serialize(serializer);
        }
        if self
            .joint_membership2177
            .as_ref()
            .is_some_and(|r| r.comparison.is_some())
        {
            #[derive(serde::Serialize)]
            struct ComparisonEnvelope<'a, T> {
                joint_membership2182_comparison_record_v1: &'a T,
            }
            ComparisonEnvelope {
                joint_membership2182_comparison_record_v1: &flat,
            }
            .serialize(serializer)
        } else if self.joint_membership2177.is_some() {
            #[derive(serde::Serialize)]
            struct Envelope<'a, T> {
                joint_membership2177_research_record_v1: &'a T,
            }
            Envelope {
                joint_membership2177_research_record_v1: &flat,
            }
            .serialize(serializer)
        } else if self.memory_readout_research.is_some() {
            super::memory_binding::serialize_research_row(&flat, serializer)
        } else {
            terminal_row_wire::serialize(&flat, self.terminal_context2135.is_some(), serializer)
        }
    }
}

pub struct CpuCanonicalStep {
    pub tick_before: Tick,
    pub tick_after: Tick,
    pub decisions: Vec<CpuCollectedDecision>,
    pub sleeping_organisms: usize,
    pub sleep_consolidations: usize,
}
fn weight_digest(founder: &CpuFoundationTrainer) -> Result<[u64; 4], ScaffoldContractError> {
    let mut d = CanonicalDigestBuilder::new(b"ALIFE-CPU-FOUNDER-WEIGHTS-V1");
    for &w in founder.weights() {
        d.write_f32(w)?;
    }
    Ok(d.finish256())
}
fn profile_identity(p: &BrainPhenotype) -> SensorProfileIdentity {
    SensorProfileIdentity {
        profile_id: p.sensor_profile().into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    }
}
fn json_digest<T: serde::Serialize>(domain: &[u8], v: &T) -> Result<[u64; 4], CpuHostError> {
    let mut d = CanonicalDigestBuilder::new(domain);
    d.write_bytes(&serde_json::to_vec(v)?);
    Ok(d.finish256())
}
impl CpuCanonicalHost {
    /// Fresh cohort only. Resuming an existing individual requires from_checkpoint;
    /// existing somatic state is never inferred from a founder asset.
    pub fn new_cohort(
        game: CanonicalNewGame,
        founder: &CpuFoundationTrainer,
        sampling_seed: u32,
    ) -> Result<Self, CpuHostError> {
        let world = game.world;
        let creatures = game.creatures;
        let p = founder.phenotype();
        let profile = profile_identity(p);
        let mut residents = BTreeMap::new();
        for record in world
            .organism_registry()
            .iter()
            .filter(|r| r.lifecycle().is_alive())
        {
            if record.biochemistry().tick != world.tick()
                || record.cognitive_work() != &CognitiveWorkReceipt::zero()
                || record.sleep_cycle_id() != 0
                || record.birth_tick() != world.tick()
            {
                return Err(CpuHostError::State(
                    "fresh cohort requires newborn canonical organism records; use exact restore for existing individuals",
                ));
            }
            validate_record_founder(record, founder)?;
            let id = record.organism_id();
            let n = p.neuron_count() as usize;
            let v11 = GpuV11CausalState::for_phenotype(p)?;
            let neural = CpuOrganismState::new(
                p,
                id,
                crate::TrainingInitialState {
                    activations: vec![0.; n],
                    activity_ema: vec![0.; n],
                    metabolic_load: vec![0.; n],
                    dendrites: v11.dendritic_branches().clone(),
                },
            )?;
            let policy = attention_selection_policy_for(p);
            residents.insert(
                id.raw(),
                Resident {
                    maze_blocked2346: None,
                    active_approach_intent2327: None,
                    joint_comparison_mode: None,
                    memory_readout: None,
                    memory_readout_digest: None,
                    #[cfg(test)]
                    last_terminal_capture: None,
                    terminal_channel2135: None,
                    sealed_authority: None,
                    neural,
                    activity: CpuActivityState::default(),
                    v11,
                    memory: MemorySidecarState::new_profiled(
                        id,
                        profile,
                        MemoryBankConfig::new(256, 64, 4, 0.72, Confidence::new(0.)?)?,
                    )?,
                    topology: TopologySidecar::new_profiled(
                        id,
                        profile,
                        TopologicalMapConfig::default(),
                    )?,
                    predictor: predictor_for_phenotype(p)?,
                    sleep: SleepController::new(sleep_consolidation_config_for(p)?)?,
                    hysteresis: HysteresisState {
                        previous_identity: None,
                        retained_ticks: 0,
                        switch_margin: policy.hysteresis_margin,
                    },
                    next_sequence: 1,
                    replay: Vec::new(),
                    language_grounding: LanguageGroundingLedger::default(),
                },
            );
        }
        if residents.is_empty() {
            return Err(CpuHostError::State(
                "cohort has no living canonical organisms",
            ));
        }
        let mut host = Self {
            #[cfg(test)]
            fail_joint_membership_after_admission: false,
            #[cfg(test)]
            fail_ingest_outcome_binding2286: false,
            maze_program2346: None,
            candidate_opportunity_coverage2313: None,
            joint_membership2177: None,
            terminal_context2135: None,
            world,
            creatures,
            phenotype_hash: p.phenotype_hash(),
            founder_weight_digest: weight_digest(founder)?,
            residents,
            sampling_seed,
            sampling_counter: 0,
        };
        host.seal_all_residents()?;
        Ok(host)
    }
    /// A new episode, prepared by ordinary world operations before the first
    /// neural dispatch. This first enforces newborn admission; it cannot admit
    /// an aged retained individual or substitute a founder for its private state.
    pub fn new_prepared_cohort<T, E: std::fmt::Display>(
        game: CanonicalNewGame,
        founder: &CpuFoundationTrainer,
        sampling_seed: u32,
        prepare: impl FnOnce(&mut HeadlessWorld) -> Result<T, E>,
    ) -> Result<(Self, T), CpuHostError> {
        let mut host = Self::new_cohort(game, founder, sampling_seed)?;
        let initial = host
            .world
            .organism_registry()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let entities = host.world.organism_entity_ids();
        let start = host.world.tick();
        let setup = prepare(&mut host.world)
            .map_err(|error| CpuHostError::Preparation(error.to_string()))?;
        if host.world.tick() < start
            || host.world.organism_entity_ids() != entities
            || host.world.organism_registry().iter().count() != initial.len()
        {
            return Err(CpuHostError::State(
                "episode setup changed the admitted roster or world clock",
            ));
        }
        for original in initial {
            let record = host
                .world
                .organism_registry()
                .get(original.organism_id())
                .ok_or(CpuHostError::State(
                    "episode setup lost its admitted learner",
                ))?;
            if record.genome() != original.genome()
                || record.phenotype() != original.phenotype()
                || record.birth_tick() != original.birth_tick()
                || !record.lifecycle().is_alive()
                || record.biochemistry().tick != host.world.tick()
                || record.sleep_cycle_id() != 0
                || record.biochemistry().body.sleeping
            {
                return Err(CpuHostError::State(
                    "episode setup changed learner identity or cannot begin an awake episode",
                ));
            }
            validate_record_founder(record, founder)?;
        }
        host.seal_all_residents()?;
        Ok((host, setup))
    }
    /// Read-only scene drafts for nonblocking semantic-prior priming. Priming
    /// advances no world tick, neural state, exposure or action sequence.
    pub fn semantic_prior_inputs(
        &mut self,
        founder: &CpuFoundationTrainer,
    ) -> Result<Vec<(PerceptionFrameDraft, ExperienceSequenceId)>, CpuHostError> {
        self.validate_founder(founder)?;
        let tick = self.world.tick();
        let mut perception_world = self.world.clone();
        let mut result = Vec::new();
        for (&raw, resident) in &self.residents {
            let id = OrganismId(raw);
            let record = self
                .world
                .organism_registry()
                .get(id)
                .ok_or(CpuHostError::State("semantic-prior learner missing"))?;
            if record.lifecycle().is_alive() && resident.sleep.state().phase == SleepPhase::Awake {
                let homeostasis = record.biochemistry().homeostasis;
                result.push((
                    perception_world.perception_frame_draft(
                        id,
                        tick,
                        founder.phenotype().sensor_profile(),
                        homeostasis,
                    )?,
                    ExperienceSequenceId(resident.next_sequence),
                ));
            }
        }
        Ok(result)
    }
    /// Fresh CPU research opt-in. It does not migrate an acquired life.
    pub fn enable_joint_membership2177(
        &mut self,
        founder: &CpuFoundationTrainer,
    ) -> Result<(), CpuHostError> {
        self.validate_joint_membership_enable(founder)?;
        let mut next = self.clone();
        next.joint_membership2177 = Some(1);
        for resident in next.residents.values_mut() {
            resident.joint_comparison_mode = Some(1);
        }
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }

    fn validate_joint_membership_enable(
        &self,
        founder: &CpuFoundationTrainer,
    ) -> Result<(), CpuHostError> {
        self.validate_founder(founder)?;
        if self.sampling_counter != 0
            || self.joint_membership2177.is_some()
            || self.terminal_context2135.is_some()
            || self
                .residents
                .values()
                .any(|r| r.next_sequence != 1 || r.memory_readout.is_some())
        {
            return Err(CpuHostError::State(
                "joint membership requires fresh exclusive construction",
            ));
        }
        Ok(())
    }

    /// Fresh equal-work route control; mode 2 applies and mode 3 withholds.
    pub fn enable_joint_membership_comparison2182(
        &mut self,
        founder: &CpuFoundationTrainer,
        apply: bool,
    ) -> Result<(), CpuHostError> {
        let mut next = self.clone();
        next.validate_joint_membership_enable(founder)?;
        next.joint_membership2177 = Some(if apply { 2 } else { 3 });
        for resident in next.residents.values_mut() {
            resident.joint_comparison_mode = next.joint_membership2177;
        }
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }

    /// Fresh-only Contact route candidate. Modes 4/5 are sealed into resident
    /// authority and schema 9; existing acquired checkpoints are never migrated.
    pub fn enable_joint_membership_contact_comparison2266(
        &mut self,
        founder: &CpuFoundationTrainer,
        apply: bool,
    ) -> Result<(), CpuHostError> {
        let mut next = self.clone();
        next.validate_joint_membership_enable(founder)?;
        next.joint_membership2177 = Some(if apply { 4 } else { 5 });
        for resident in next.residents.values_mut() {
            resident.joint_comparison_mode = next.joint_membership2177;
        }
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }

    /// Fresh-only Contact route candidate. Modes 6/7 are sealed into resident
    /// authority and schema 10; existing acquired checkpoints are never migrated.
    pub fn enable_joint_membership_contact_query_comparison2273(
        &mut self,
        founder: &CpuFoundationTrainer,
        apply: bool,
    ) -> Result<(), CpuHostError> {
        let mut next = self.clone();
        next.validate_joint_membership_enable(founder)?;
        next.joint_membership2177 = Some(if apply { 6 } else { 7 });
        for resident in next.residents.values_mut() {
            resident.joint_comparison_mode = next.joint_membership2177;
        }
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }

    /// Fresh-only full-query Contact success-component projection (schema 12, cost revision 2).
    /// Both modes apply joint Contact recall; mode 9 withholds only lane 33 projection.
    pub fn enable_joint_contact_success_projection2275(
        &mut self,
        founder: &CpuFoundationTrainer,
        apply_projection: bool,
    ) -> Result<(), CpuHostError> {
        let mut next = self.clone();
        next.validate_joint_membership_enable(founder)?;
        next.joint_membership2177 = Some(if apply_projection { 8 } else { 9 });
        for resident in next.residents.values_mut() {
            resident.joint_comparison_mode = next.joint_membership2177;
        }
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }

    /// Fresh-only full-query non-Ingest mask (191), checkpoint schema 13.
    pub fn enable_joint_membership_non_ingest_comparison2278(
        &mut self,
        founder: &CpuFoundationTrainer,
        apply: bool,
    ) -> Result<(), CpuHostError> {
        let mut next = self.clone();
        next.validate_joint_membership_enable(founder)?;
        next.joint_membership2177 = Some(if apply { 10 } else { 11 });
        for resident in next.residents.values_mut() {
            resident.joint_comparison_mode = next.joint_membership2177;
        }
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }

    /// Fresh global-joint Ingest valence/energy projection, component mask 9.
    pub fn enable_joint_ingest_outcome_projection2286(
        &mut self,
        founder: &CpuFoundationTrainer,
        apply_projection: bool,
    ) -> Result<(), CpuHostError> {
        let mut next = self.clone();
        next.validate_joint_membership_enable(founder)?;
        next.joint_membership2177 = Some(if apply_projection { 12 } else { 13 });
        for resident in next.residents.values_mut() {
            resident.joint_comparison_mode = next.joint_membership2177;
        }
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }

    /// Fresh direct global query, preserving supported projection semantics.
    pub fn enable_joint_ingest_direct2295(
        &mut self,
        founder: &CpuFoundationTrainer,
        apply_projection: bool,
    ) -> Result<(), CpuHostError> {
        let mut next = self.clone();
        next.validate_joint_membership_enable(founder)?;
        next.joint_membership2177 = Some(if apply_projection { 14 } else { 15 });
        for resident in next.residents.values_mut() {
            resident.joint_comparison_mode = next.joint_membership2177;
        }
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }

    /// Explicit research sensing/repertoire capability, never a genome migration.
    pub fn fork_with_maze_program_for_research2346(&self,founder:&CpuFoundationTrainer)->Result<Self,CpuHostError>{
        self.validate_founder(founder)?;let before=self.checkpoint(founder)?;
        if before.schema_version!=18||self.maze_program2346.is_some()||self.candidate_opportunity_coverage2313!=Some(3)||self.residents.values().any(|r|r.maze_blocked2346.is_some()){return Err(CpuHostError::State("maze program requires original CP18 cap3"));}
        let mut next=self.clone();next.maze_program2346=Some(1);let after=next.checkpoint(founder)?;let mut norm=after.clone();norm.schema_version=before.schema_version;norm.maze_program2346=None;
        if serde_json::to_vec(&norm)?!=serde_json::to_vec(&before)?{return Err(CpuHostError::State("maze attachment changed retained state"));}
        let cold=Self::from_checkpoint(after.clone(),founder)?;if serde_json::to_vec(&after)?!=serde_json::to_vec(&cold.checkpoint(founder)?)?{return Err(CpuHostError::State("maze cold attachment mismatch"));}Ok(next)
    }

    /// Explicit additive active-opportunity continuity, initially without intent.
    pub fn fork_with_active_approach_opportunity_for_research(&self, founder: &CpuFoundationTrainer) -> Result<Self,CpuHostError> {
        self.validate_founder(founder)?;
        if self.candidate_opportunity_coverage2313 != Some(2) || self.residents.values().any(|r|r.active_approach_intent2327.is_some()) {return Err(CpuHostError::State("active opportunity requires CP17 coverage2 without intent"));}
        let before=self.checkpoint(founder)?;let mut next=self.clone();next.candidate_opportunity_coverage2313=Some(3);let after=next.checkpoint(founder)?;let mut normalized=after.clone();normalized.schema_version=before.schema_version;normalized.candidate_opportunity_coverage2313=before.candidate_opportunity_coverage2313;
        if serde_json::to_vec(&normalized)?!=serde_json::to_vec(&before)? {return Err(CpuHostError::State("active opportunity attachment changed state"));}
        let cold=Self::from_checkpoint(after.clone(),founder)?;if serde_json::to_vec(&cold.checkpoint(founder)?)?!=serde_json::to_vec(&after)? {return Err(CpuHostError::State("active opportunity cold attachment changed state"));}Ok(next)
    }

    /// Explicit experimental action-to-attention feedback on an immutable copy.
    /// No intent is seeded at attachment; only the existing adapter identity changes.
    pub fn fork_with_approach_attention_feedback_for_research(
        &self, founder: &CpuFoundationTrainer,
    ) -> Result<Self, CpuHostError> {
        self.validate_founder(founder)?;
        if self.candidate_opportunity_coverage2313 != Some(1)
            || !matches!(self.joint_membership2177, Some(14 | 15))
            || founder.phenotype().sensor_profile() != SensorProfile::GroundedObjectSlotsV1 {
            return Err(CpuHostError::State("Approach feedback requires CP16 coverage1 mode14/15 Slots"));
        }
        let before = self.checkpoint(founder)?;
        let mut next = self.clone();
        next.candidate_opportunity_coverage2313 = Some(2);
        let after = next.checkpoint(founder)?;
        let mut normalized = after.clone();
        normalized.schema_version = before.schema_version;
        normalized.candidate_opportunity_coverage2313 = before.candidate_opportunity_coverage2313;
        if serde_json::to_vec(&normalized)? != serde_json::to_vec(&before)? {
            return Err(CpuHostError::State("Approach feedback fork changed acquired state"));
        }
        let cold = Self::from_checkpoint(after.clone(), founder)?;
        if serde_json::to_vec(&cold.checkpoint(founder)?)? != serde_json::to_vec(&after)? {
            return Err(CpuHostError::State("Approach feedback fork cold validation changed state"));
        }
        Ok(next)
    }

    /// Explicit retained-life experimental opportunity adapter. No genotype,
    /// phenotype, neural, memory, world, sampling or founder mutation occurs.
    /// Saved identity becomes CP16; disabling or hand-editing is not a migration.
    pub fn enable_candidate_opportunity_coverage2313(
        &mut self,
        founder: &CpuFoundationTrainer,
    ) -> Result<(), CpuHostError> {
        self.validate_founder(founder)?;
        if self.candidate_opportunity_coverage2313.is_some()
            || !matches!(self.joint_membership2177, Some(14 | 15))
            || founder.phenotype().sensor_profile() != SensorProfile::GroundedObjectSlotsV1
        {
            return Err(CpuHostError::State(
                "candidate adapter requires unchanged mode14/15 Slots policy",
            ));
        }
        let before = serde_json::to_value(self.checkpoint(founder)?)?;
        self.candidate_opportunity_coverage2313 = Some(1);
        let result = (|| {
            let mut old = before;
            let mut new = serde_json::to_value(self.checkpoint(founder)?)?;
            old.as_object_mut().unwrap().remove("schema_version");
            new.as_object_mut().unwrap().remove("schema_version");
            new.as_object_mut()
                .unwrap()
                .remove("candidate_opportunity_coverage2313");
            if serde_json::to_vec(&old)? != serde_json::to_vec(&new)? {
                return Err(CpuHostError::State(
                    "candidate adapter attachment changed retained state",
                ));
            }
            Ok(())
        })();
        if result.is_err() {
            self.candidate_opportunity_coverage2313 = None;
        }
        result
    }

    /// Explicit manual calibration of this simulator branch's readout policy.
    /// This does not claim learning and never migrates genes or retained biology.
    /// Stage on a clone; commit only after exact-state and native cold validation.
    pub fn rebase_readout_policy(
        &mut self,
        previous: &CpuFoundationTrainer,
        next: &CpuFoundationTrainer,
    ) -> Result<(), CpuHostError> {
        self.rebase_policy_checked2346(previous,next,false)
    }
    pub fn rebase_maze_program_policy2346(&mut self,previous:&CpuFoundationTrainer,next:&CpuFoundationTrainer)->Result<(),CpuHostError>{
        if self.maze_program2346!=Some(1){return Err(CpuHostError::State("maze rebase requires explicit CP19"));}
        self.rebase_policy_checked2346(previous,next,true)
    }
    fn rebase_policy_checked2346(&mut self,previous:&CpuFoundationTrainer,next:&CpuFoundationTrainer,maze:bool)->Result<(),CpuHostError>{
        self.validate_founder(previous)?;
        if serde_json::to_vec(previous.phenotype())? != serde_json::to_vec(next.phenotype())?
            || previous.source_foundation.encode_canonical()?
                != next.source_foundation.encode_canonical()?
            || previous.replay_execution() != next.replay_execution()
        {
            return Err(CpuHostError::State(
                "readout calibration changed source, phenotype or execution",
            ));
        }
        let old = previous.checkpoint();
        let mut normalized = next.checkpoint();
        if old.weights.len() != normalized.weights.len() {
            return Err(CpuHostError::State(
                "readout calibration weight length changed",
            ));
        }
        let mut changed = false;
        for (index, (a, b)) in old.weights.iter().zip(&normalized.weights).enumerate() {
            if a.to_bits() != b.to_bits() {
                if !b.is_finite()
                    || !(if maze {super::maze_program::program_coordinate(previous.phenotype(),index)&&b.abs()<=16.}else{super::readout_calibration_coordinate(previous.phenotype(), index)})
                {
                    return Err(CpuHostError::State(
                        "readout calibration changed forbidden coordinate",
                    ));
                }
                changed = true;
            }
        }
        if !changed {
            return Err(CpuHostError::State(
                "readout calibration requires changed weight bits",
            ));
        }
        normalized.weights.clone_from(&old.weights);
        if serde_json::to_vec(&old)? != serde_json::to_vec(&normalized)? {
            return Err(CpuHostError::State(
                "readout calibration changed optimizer or checkpoint metadata",
            ));
        }
        let before = self.checkpoint(previous)?;
        let mut staged = self.clone();
        staged.founder_weight_digest = weight_digest(next)?;
        if staged.founder_weight_digest == self.founder_weight_digest {
            return Err(CpuHostError::State(
                "readout calibration requires distinct policy digest",
            ));
        }
        let after = staged.checkpoint(next)?;
        let mut normalized_after = after.clone();
        normalized_after.founder_weight_digest = before.founder_weight_digest;
        if serde_json::to_vec(&before)? != serde_json::to_vec(&normalized_after)? {
            return Err(CpuHostError::State(
                "readout calibration changed retained state",
            ));
        }
        let cold = Self::from_checkpoint(after.clone(), next)?;
        if serde_json::to_vec(&cold.checkpoint(next)?)? != serde_json::to_vec(&after)? {
            return Err(CpuHostError::State(
                "readout calibration cold restore changed retained state",
            ));
        }
        *self = staged;
        Ok(())
    }

    /// Alive-only age-policy counterfactual copy, not learning or resurrection.
    /// Authentication and cold validation preserve the exact acquired branch;
    /// only the pre-existing persisted world age-death flag may differ.
    pub fn fork_without_age_death_for_research(
        &self,
        founder: &CpuFoundationTrainer,
    ) -> Result<Self, CpuHostError> {
        self.validate_founder(founder)?;
        if self.residents.is_empty() {
            return Err(CpuHostError::State(
                "age-policy fork requires living residents",
            ));
        }
        for raw in self.residents.keys() {
            let record = self
                .world
                .organism_registry()
                .get(OrganismId(*raw))
                .ok_or(CpuHostError::State("age-policy fork resident missing"))?;
            if !record.lifecycle().is_alive() {
                return Err(CpuHostError::State(
                    "age-policy fork cannot revive dead residents",
                ));
            }
        }
        let before = self.checkpoint(founder)?;
        let mut fork = self.clone();
        fork.world = self.world.fork_without_age_death_for_research()?;
        let after = fork.checkpoint(founder)?;
        let mut normalized = after.clone();
        normalized.world.world.disable_age_death = before.world.world.disable_age_death;
        if serde_json::to_vec(&before)? != serde_json::to_vec(&normalized)? {
            return Err(CpuHostError::State(
                "age-policy fork changed retained state beyond flag",
            ));
        }
        let cold = Self::from_checkpoint(after.clone(), founder)?;
        if serde_json::to_vec(&after)? != serde_json::to_vec(&cold.checkpoint(founder)?)? {
            return Err(CpuHostError::State(
                "age-policy fork cold restore changed state",
            ));
        }
        Ok(fork)
    }

    pub fn world(&self) -> &HeadlessWorld {
        &self.world
    }
    /// Scene variations use ordinary world APIs without reward or neural overrides.
    pub fn world_mut(&mut self) -> &mut HeadlessWorld {
        &mut self.world
    }
    pub fn snapshot(&self) -> Self {
        self.clone()
    }
    pub fn restore_snapshot(
        &mut self,
        snapshot: Self,
        founder: &CpuFoundationTrainer,
    ) -> Result<(), CpuHostError> {
        snapshot.validate_founder(founder)?;
        *self = snapshot;
        Ok(())
    }
    /// Fresh construction extension, separately versioned and frozen. It cannot
    /// be attached to an acquired life or silently exported as a legacy genome.
    pub fn enable_frozen_memory_readout(
        &mut self,
        config: FrozenMemoryReadoutConstructionV1,
        founder: &CpuFoundationTrainer,
    ) -> Result<(), CpuHostError> {
        if self.sampling_counter != 0
            || self.joint_membership2177.is_some()
            || self.terminal_context2135.is_some()
            || self.residents.values().any(|r| {
                r.memory_readout.is_some()
                    || r.neural
                        .checkpoint()
                        .is_ok_and(|c| c.last_decision_tick.is_some())
            })
        {
            return Err(CpuHostError::State(
                "memory readout requires a fresh exclusive construction",
            ));
        }
        config.validate()?;
        config.validate_parent(founder)?;
        for record in self.world.organism_registry().iter() {
            if record.genome() != &config.parent_genome {
                return Err(CpuHostError::State("memory readout full parent mismatch"));
            }
        }
        let digest = config.digest()?;
        for resident in self.residents.values_mut() {
            resident.memory_readout = Some(config.clone());
            resident.memory_readout_digest = Some(digest);
        }
        self.seal_all_residents()?;
        Ok(())
    }
    /// Opt-in only before the first decision. The complete candidate is the
    /// exact parent foundation plus this separately persisted research recipe.
    pub fn enable_terminal_context2135(
        &mut self,
        config: TerminalContextConfig2135,
        founder: &CpuFoundationTrainer,
    ) -> Result<(), CpuHostError> {
        if self.sampling_counter != 0
            || self.joint_membership2177.is_some()
            || self.terminal_context2135.is_some()
            || self.residents.values().any(|r| r.memory_readout.is_some())
        {
            return Err(CpuHostError::State(
                "terminal recipe can only be set before first decision",
            ));
        }
        config.validate().map_err(CpuHostError::Preparation)?;
        config
            .validate_parent(&founder.source_foundation, founder.phenotype())
            .map_err(CpuHostError::Preparation)?;
        for record in self.world.organism_registry().iter() {
            config
                .validate_genome(record.genome())
                .map_err(CpuHostError::Preparation)?;
        }
        let channel_policy = config.schema_version == 3;
        if channel_policy {
            for (&id, resident) in &mut self.residents {
                resident.terminal_channel2135 = Some(TerminalChannelState2135::new(OrganismId(id)));
            }
        }
        self.terminal_context2135 = Some(config);
        if channel_policy {
            self.seal_all_residents()?;
        }
        Ok(())
    }
    fn validate_founder(&self, founder: &CpuFoundationTrainer) -> Result<(), CpuHostError> {
        if self.maze_program2346!=Some(1) && founder.weights().iter().zip(founder.phenotype().synapses()).enumerate().any(|(i,(w,s))|super::maze_program::recurrent_coordinate(founder.phenotype(),i)&&w.to_bits()!=s.genetic_weight().to_bits()){return Err(CpuHostError::State("recurrent maze policy requires CP19"));}
        for resident in self.residents.values() {
            if let Some(config) = &resident.memory_readout {
                config.validate_parent(founder)?;
            }
        }
        if let Some(config) = &self.terminal_context2135 {
            config.validate().map_err(CpuHostError::Preparation)?;
            config
                .validate_parent(&founder.source_foundation, founder.phenotype())
                .map_err(CpuHostError::Preparation)?;
        }
        if self.phenotype_hash != founder.phenotype().phenotype_hash()
            || self.founder_weight_digest != weight_digest(founder)?
        {
            return Err(CpuHostError::State(
                "living collector policy changed: export/recompile a fresh cohort or explicitly rebase a simulator branch",
            ));
        }
        Ok(())
    }
    /// Explicit canonical recovery request, useful for recovery protocols and
    /// branch experiments. It changes sleep authority, never body chemistry.
    pub fn request_recovery_sleep(&mut self, id: OrganismId) -> Result<(), CpuHostError> {
        let mut next = self.clone();
        let tick = next.world.tick();
        let r = next
            .residents
            .get_mut(&id.raw())
            .ok_or(CpuHostError::State("unknown recovery organism"))?;
        r.sealed_authority = None;
        r.sleep
            .force_sleep(tick, alife_core::SleepTrigger::RecoveryProtocol)?;
        let state = r.sleep.state();
        let mut record = next
            .world
            .organism_registry()
            .get(id)
            .cloned()
            .ok_or(CpuHostError::State("recovery organism missing"))?;
        record.seal_sleep_phase(state.phase, state.active_cycle_id, tick, 0)?;
        next.world.replace_organism_record_exact(record)?;
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }
    pub fn checkpoint(
        &self,
        founder: &CpuFoundationTrainer,
    ) -> Result<CpuCanonicalCheckpoint, CpuHostError> {
        self.validate_founder(founder)?;
        let mut config =
            RuntimeConfig::deterministic_default(self.world.seed(), BrainScaleTier::Standard2048);
        config.brain_policy = BrainPolicyConfig::offline_cpu_training();
        config.validate()?;
        let mut creatures = self.creatures.clone();
        for creature in &mut creatures {
            let record = self
                .world
                .organism_registry()
                .get(creature.organism_id)
                .ok_or(CpuHostError::State(
                    "creature summary missing canonical record",
                ))?;
            creature.mind.tick = record.biochemistry().tick;
            creature.mind.homeostasis = record.biochemistry().homeostasis;
            creature.development_tick = record.biochemistry().development.last_update_tick;
            creature.gpu_brain = None;
            creature.lifetime_state_asset = None;
            creature.composite_genetics = None;
        }
        let world = PortableSaveFile::from_headless_world(
            "cpu-canonical-collector",
            &self.world,
            config,
            AssetManifest::empty(),
            creatures,
        )?;
        let residents = self
            .residents
            .values()
            .map(|r| {
                Ok(CpuResidentCheckpoint {
                    maze_blocked2346: r.maze_blocked2346.clone(),
                    active_approach_intent2327: r.active_approach_intent2327.clone(),
                    memory_readout: r.memory_readout.clone(),
                    terminal_channel2135: r.terminal_channel2135.clone(),
                    neural: r.neural.checkpoint()?,
                    activity: r.activity.clone(),
                    neural_topology: r.v11.checkpoint(),
                    memory: r.memory.export_active_bank()?,
                    memory_compaction: *r.memory.compaction_checkpoint(),
                    staged_memory: r.memory.export_staged_bank()?,
                    topology: r.topology.export_portable()?,
                    predictor: r.predictor.clone(),
                    sleep: r.sleep,
                    hysteresis: r.hysteresis,
                    next_sequence: r.next_sequence,
                    replay: r.replay.clone(),
                    language_grounding: r.language_grounding.clone(),
                })
            })
            .collect::<Result<_, CpuHostError>>()?;
        Ok(CpuCanonicalCheckpoint {
            maze_program2346: self.maze_program2346,
            candidate_opportunity_coverage2313: self.candidate_opportunity_coverage2313,
            joint_membership2177: self.joint_membership2177,
            terminal_context2135: self.terminal_context2135.clone(),
            schema_version: if self.maze_program2346==Some(1) {19} else if self.candidate_opportunity_coverage2313 == Some(3) {
                18
            } else if self.candidate_opportunity_coverage2313 == Some(2) {
                17
            } else if self.candidate_opportunity_coverage2313.is_some() {
                16
            } else if self.joint_membership2177.is_some() {
                match self.joint_membership2177 {
                    Some(1) => 8,
                    Some(4 | 5) => 9,
                    Some(6 | 7) => 10,
                    Some(8 | 9) => 12,
                    Some(10 | 11) => 13,
                    Some(12 | 13) => 14,
                    Some(14 | 15) => 15,
                    _ => 7,
                }
            } else if self.residents.values().any(|r| r.memory_readout.is_some()) {
                5
            } else {
                terminal_host_checkpoint_schema2135(self.terminal_context2135.is_some())
            },
            phenotype_hash: self.phenotype_hash,
            founder_weight_digest: self.founder_weight_digest,
            sampling_seed: self.sampling_seed,
            sampling_counter: self.sampling_counter,
            world,
            tracked_objects: self
                .residents
                .keys()
                .map(|&id| self.world.tracked_objects().save_state(OrganismId(id)))
                .collect::<Result<_, _>>()?,
            residents,
        })
    }
    pub fn from_checkpoint(
        checkpoint: CpuCanonicalCheckpoint,
        founder: &CpuFoundationTrainer,
    ) -> Result<Self, CpuHostError> {
        if checkpoint.schema_version == 1 {
            return Err(CpuHostError::State(
                "version1 CPU checkpoint lacks tracked-object identity; exact private-life restore requires version2",
            ));
        }
        if let Some(version) = checkpoint.candidate_opportunity_coverage2313 {
            if !matches!(version, 1..=3)
                || checkpoint.schema_version != if checkpoint.maze_program2346==Some(1){19}else{match version {3=>18,2=>17,_=>16}}
                || !matches!(checkpoint.joint_membership2177, Some(14 | 15))
                || founder.phenotype().sensor_profile() != SensorProfile::GroundedObjectSlotsV1
            {
                return Err(CpuHostError::State(
                    "invalid candidate opportunity adapter identity",
                ));
            }
        } else if matches!(checkpoint.schema_version, 16..=18) {
            return Err(CpuHostError::State(
                "CP16 requires explicit candidate opportunity adapter",
            ));
        }
        if checkpoint.maze_program2346.is_some() && (checkpoint.maze_program2346!=Some(1)||checkpoint.schema_version!=19||checkpoint.candidate_opportunity_coverage2313!=Some(3)) {return Err(CpuHostError::State("invalid maze program identity"));}
        if checkpoint.schema_version==19&&checkpoint.maze_program2346!=Some(1){return Err(CpuHostError::State("CP19 requires maze program"));}
        for resident in &checkpoint.residents {
            if let Some(h)=&resident.maze_blocked2346 {h.validate()?;if checkpoint.maze_program2346!=Some(1)||h.source_sequence.raw()>=resident.next_sequence {return Err(CpuHostError::State("invalid blocked history identity"));}}
            if let Some(intent) = &resident.active_approach_intent2327 {
                if checkpoint.candidate_opportunity_coverage2313 != Some(3) {return Err(CpuHostError::State("active intent requires CP18 coverage3"));}
                intent.validate()?;
            }
        }
        let has_readout = checkpoint
            .residents
            .iter()
            .any(|r| r.memory_readout.is_some());
        if let Some(version) = checkpoint.joint_membership2177 {
            if !matches!(version, 1..=15)
                || checkpoint.schema_version
                    != match version {
                        1 => 8,
                        4 | 5 => 9,
                        6 | 7 => 10,
                        8 | 9 => 12,
                        10 | 11 => 13,
                        12 | 13 => 14,
                        14 | 15 => {
                            if checkpoint.maze_program2346==Some(1) {19} else if checkpoint.candidate_opportunity_coverage2313 == Some(3) {
                                18
                            } else if checkpoint.candidate_opportunity_coverage2313 == Some(2) {
                                17
                            } else if checkpoint.candidate_opportunity_coverage2313.is_some() {
                                16
                            } else {
                                15
                            }
                        }
                        _ => 7,
                    }
                || has_readout
                || checkpoint.terminal_context2135.is_some()
            {
                return Err(CpuHostError::State(
                    "invalid joint membership research checkpoint",
                ));
            }
        } else if has_readout {
            if checkpoint.schema_version != 5 || checkpoint.terminal_context2135.is_some() {
                return Err(CpuHostError::State(
                    "unsupported combined/legacy memory readout checkpoint",
                ));
            }
        } else {
            validate_terminal_host_checkpoint_schema2135(
                checkpoint.schema_version,
                checkpoint.terminal_context2135.is_some(),
            )
            .map_err(CpuHostError::State)?;
        }
        let channel_policy = checkpoint
            .terminal_context2135
            .as_ref()
            .is_some_and(|c| c.schema_version == 3);
        if checkpoint.world.config.brain_policy.policy != PolicyBackend::NeuralClosedLoopCpu {
            return Err(CpuHostError::State(
                "not a versioned offline CPU checkpoint",
            ));
        }
        checkpoint.world.config.validate()?;
        let creatures = checkpoint.world.creatures.clone();
        let mut world = checkpoint.world.restore_headless_world()?;
        let owners = checkpoint
            .residents
            .iter()
            .map(|r| r.neural.organism_id.raw())
            .collect::<std::collections::BTreeSet<_>>();
        let tracked_owners = checkpoint
            .tracked_objects
            .iter()
            .map(|s| s.organism_id.raw())
            .collect::<std::collections::BTreeSet<_>>();
        if owners != tracked_owners
            || owners.len() != checkpoint.residents.len()
            || checkpoint
                .tracked_objects
                .iter()
                .any(|s| s.records.iter().any(|r| r.last_seen_tick > world.tick()))
        {
            return Err(CpuHostError::State(
                "checkpoint tracked-object owner or clock mismatch",
            ));
        }
        world.restore_tracked_object_states(checkpoint.tracked_objects)?;
        let mut residents = BTreeMap::new();
        let p = founder.phenotype();
        let profile = profile_identity(p);
        for r in checkpoint.residents {
            let id = r.neural.organism_id;
            let record = world
                .organism_registry()
                .get(id)
                .ok_or(CpuHostError::State(
                    "checkpoint resident has no canonical organism",
                ))?;
            if r.next_sequence == 0
                || r.sleep.state().phase != record.sleep_phase()
                || r.sleep
                    .state()
                    .active_cycle_id
                    .max(r.sleep.state().last_consolidated_cycle_id)
                    != record.sleep_cycle_id()
                || r.neural
                    .last_outcome_tick
                    .is_some_and(|t| t > world.tick().raw())
                || r.replay.len() > p.replay_capture_plan().event_capacity() as usize
            {
                return Err(CpuHostError::State("checkpoint causal boundary mismatch"));
            }
            if let Some(h)=&r.maze_blocked2346 {h.validate()?;if h.organism_id!=id||h.source_tick>=world.tick(){return Err(CpuHostError::State("blocked history causal boundary mismatch"));}}
            if let Some(intent)=&r.active_approach_intent2327 {
                if intent.organism_id!=id || intent.source_tick>=world.tick() {return Err(CpuHostError::State("active intent causal boundary mismatch"));}
            }
            validate_record_founder(record, founder)?;
            r.language_grounding.validate_contract()?;
            if r.memory.last_observed_sequence_id_raw != r.next_sequence - 1
                || r.language_grounding.last_sequence_id() != r.next_sequence - 1
            {
                return Err(CpuHostError::State(
                    "checkpoint memory/language sequence mismatch",
                ));
            }
            replay_evidence(p, &r.replay)?.validate_contract()?;
            if r.memory.organism_id_raw != id.raw()
                || r.memory_compaction.organism_id_raw != id.raw()
                || r.topology.organism_id_raw != id.raw()
                || r.topology.profile != profile
            {
                return Err(CpuHostError::State(
                    "checkpoint sidecar owner/profile mismatch",
                ));
            }
            let sleep =
                SleepController::restore(sleep_consolidation_config_for(p)?, r.sleep.state())?;
            if r.neural_topology.neuron_count != p.neuron_count()
                || r.neural_topology.dendritic_branches != r.neural.neural.dendrites
                || r.neural_topology.pending_lifetime_synapse.is_some()
                || r.neural_topology
                    .sparse_spans
                    .iter()
                    .flat_map(|s| &s.edges)
                    .any(|e| e.route as usize >= p.projections().len())
            {
                return Err(CpuHostError::State("neural topology checkpoint mismatch"));
            }
            if r.terminal_channel2135.is_some() != channel_policy {
                return Err(CpuHostError::State(
                    "terminal observation recipe/state mismatch",
                ));
            }
            if let Some(channels) = &r.terminal_channel2135 {
                channels
                    .validate(r.next_sequence)
                    .map_err(CpuHostError::Preparation)?;
                if channels.organism_id != id
                    || channels
                        .observations
                        .iter()
                        .any(|o| o.query().profile() != profile || o.outcome_tick() > world.tick())
                {
                    return Err(CpuHostError::State(
                        "terminal observation owner/profile/clock mismatch",
                    ));
                }
            }
            if let Some(config) = &r.memory_readout {
                config.validate()?;
                config.validate_parent(founder)?;
                if &config.parent_genome != record.genome() {
                    return Err(CpuHostError::State("memory readout parent genome differs"));
                }
            }
            let mut resident = Resident {
                maze_blocked2346: r.maze_blocked2346,
                active_approach_intent2327: r.active_approach_intent2327,
                joint_comparison_mode: checkpoint.joint_membership2177,
                memory_readout_digest: r.memory_readout.as_ref().map(|c| c.digest()).transpose()?,
                memory_readout: r.memory_readout,
                #[cfg(test)]
                last_terminal_capture: None,
                terminal_channel2135: r.terminal_channel2135,
                sealed_authority: None,
                v11: GpuV11CausalState::restore(r.neural_topology)?,
                neural: CpuOrganismState::from_checkpoint(p, r.neural)?,
                activity: r.activity,
                memory: (if checkpoint.joint_membership2177.is_some() {
                    MemorySidecarState::restore_portable_with_joint_membership
                } else {
                    MemorySidecarState::restore_portable
                })(profile, r.memory_compaction, r.memory, r.staged_memory)?,
                topology: TopologySidecar::restore_portable(r.topology)?,
                predictor: r.predictor,
                sleep,
                hysteresis: r.hysteresis,
                next_sequence: r.next_sequence,
                replay: r.replay,
                language_grounding: r.language_grounding,
            };
            resident.sealed_authority = Some(validate_resident_authority(
                record,
                &resident,
                checkpoint.sampling_seed,
                checkpoint.sampling_counter,
            )?);
            if residents.insert(id.raw(), resident).is_some() {
                return Err(CpuHostError::State("duplicate checkpoint organism"));
            }
        }
        for r in world
            .organism_registry()
            .iter()
            .filter(|r| r.lifecycle().is_alive())
        {
            if !residents.contains_key(&r.organism_id().raw()) {
                return Err(CpuHostError::State("checkpoint missing living organism"));
            }
        }
        let host = Self {
            #[cfg(test)]
            fail_joint_membership_after_admission: false,
            #[cfg(test)]
            fail_ingest_outcome_binding2286: false,
            maze_program2346: checkpoint.maze_program2346,
            candidate_opportunity_coverage2313: checkpoint.candidate_opportunity_coverage2313,
            joint_membership2177: checkpoint.joint_membership2177,
            terminal_context2135: checkpoint.terminal_context2135.clone(),
            world,
            creatures,
            phenotype_hash: checkpoint.phenotype_hash,
            founder_weight_digest: checkpoint.founder_weight_digest,
            residents,
            sampling_seed: checkpoint.sampling_seed,
            sampling_counter: checkpoint.sampling_counter,
        };
        host.validate_founder(founder)?;
        if let Some(config) = &host.terminal_context2135 {
            for record in host.world.organism_registry().iter() {
                config
                    .validate_genome(record.genome())
                    .map_err(CpuHostError::Preparation)?;
            }
        }
        Ok(host)
    }
    fn seal_all_residents(&mut self) -> Result<(), CpuHostError> {
        for (&raw, r) in &mut self.residents {
            let mut record = self
                .world
                .organism_registry()
                .get(OrganismId(raw))
                .cloned()
                .ok_or(CpuHostError::State(
                    "missing organism at CPU sealed boundary",
                ))?;
            let authority = resident_authority(r, self.sampling_seed, self.sampling_counter)?;
            record.seal_cognitive_subsystems(
                record.biochemistry().tick,
                authority.brain,
                authority.memory,
            )?;
            self.world.replace_organism_record_exact(record)?;
            r.sealed_authority = Some(authority);
        }
        Ok(())
    }
    /// Atomic whole-world tick: a failed seal/learning/checkpoint invariant rolls
    /// back body, biology, all cognitive rows and the sampling counter together.
    pub fn step(
        &mut self,
        founder: &CpuFoundationTrainer,
        temperature: f32,
    ) -> Result<CpuCanonicalStep, CpuHostError> {
        self.step_with_preparation(founder, temperature, |draft, _| Ok(draft))
    }
    /// The shared semantic prior may enrich only its bounded private context.
    /// Ordinary senses, candidates, body and identity are checked unchanged.
    /// Host rollback does not roll back an external provider. Its owner archives
    /// that state and reconciles a failed episode instead of retrying its seed.
    pub fn step_with_preparation(
        &mut self,
        founder: &CpuFoundationTrainer,
        temperature: f32,
        mut prepare: impl FnMut(
            PerceptionFrameDraft,
            ExperienceSequenceId,
        ) -> Result<PerceptionFrameDraft, ScaffoldContractError>,
    ) -> Result<CpuCanonicalStep, CpuHostError> {
        self.validate_founder(founder)?;
        for (&raw, r) in &self.residents {
            let record = self
                .world
                .organism_registry()
                .get(OrganismId(raw))
                .ok_or(CpuHostError::State("missing canonical organism"))?;
            r.sealed_authority
                .ok_or(CpuHostError::State(
                    "CPU resident has no completed private seal",
                ))?
                .validate(record, self.sampling_seed, self.sampling_counter)?;
        }
        let mut next = self.clone();
        // The global sampling counter is part of every resident digest, even
        // when a particular resident sleeps or makes no decision this tick.
        for resident in next.residents.values_mut() {
            resident.sealed_authority = None;
        }
        let result = next.step_inner(founder, temperature, &mut prepare)?;
        *self = next;
        Ok(result)
    }
    fn step_inner(
        &mut self,
        founder: &CpuFoundationTrainer,
        temperature: f32,
        prepare: &mut impl FnMut(
            PerceptionFrameDraft,
            ExperienceSequenceId,
        ) -> Result<PerceptionFrameDraft, ScaffoldContractError>,
    ) -> Result<CpuCanonicalStep, CpuHostError> {
        let p = founder.phenotype();
        let tick = self.world.tick();
        let mut decisions = Vec::new();
        let mut sleeping = 0;
        let mut consolidations = 0;
        let mut body_events = BTreeMap::new();
        let mut perception_world = self.world.clone();
        let ids = self
            .world
            .organism_registry()
            .iter()
            .filter(|r| r.lifecycle().is_alive())
            .map(|r| r.organism_id())
            .collect::<Vec<_>>();
        for id in ids {
            let mut record = self
                .world
                .organism_registry()
                .get(id)
                .cloned()
                .ok_or(CpuHostError::State("missing organism"))?;
            let r = self
                .residents
                .get_mut(&id.raw())
                .ok_or(CpuHostError::State("new organism requires brain admission"))?;
            let phase_before = r.sleep.state().phase;
            if r.activity
                .last_world_tick
                .is_some_and(|last| last.checked_add(1) != Some(tick.raw()))
            {
                return Err(CpuHostError::State("CPU activity world clock mismatch"));
            }
            r.activity.last_world_tick = Some(tick.raw());
            r.activity.remaining_atp_q16 =
                (f64::from(record.biochemistry().homeostasis.drives.brain_atp)
                    * f64::from(BRAIN_ATP_Q16_MAX))
                .floor() as u32;
            let neural_boundary = r.neural.checkpoint()?;
            let activity_anchor = CpuActivityAnchor {
                organism_id: id,
                tick,
                sequence_cursor: r.next_sequence,
                previous_dispatch_generation: neural_boundary.dispatch_generation,
                source_frame_digest: r.activity.source_frame_digest,
                frame_digest: p.phenotype_hash().0,
            };
            let capacity = BrainCapacityClass::production_for_id(p.brain_class_id())?;
            let lookahead = cpu_activity_budget(
                p,
                activity_anchor,
                record.biochemistry().homeostasis.drives.brain_atp,
                u32::from(capacity.execution().max_candidates()),
                u32::from(capacity.execution().max_memory_context_records()),
            )?;
            if phase_before == SleepPhase::Awake && !lookahead.affordable {
                r.sleep
                    .force_sleep(tick, alife_core::SleepTrigger::RecoveryProtocol)?;
            }

            let config = sleep_consolidation_config_for(p)?;
            if phase_before == SleepPhase::Awake {
                r.sleep.evaluate_homeostasis(
                    &record.biochemistry().homeostasis,
                    HomeostaticParameters::reference(),
                    tick,
                )?;
            } else if phase_before != SleepPhase::Waking
                || (record.biochemistry().homeostasis.drives.fatigue
                    < config.fatigue_threshold.raw()
                    && record.biochemistry().homeostasis.hormones.sleep_pressure
                        < config.sleep_pressure_threshold.raw())
            {
                r.sleep.advance(tick)?;
            }
            let mut sleep_work_units = 0;
            if r.sleep.state().phase == SleepPhase::Consolidating
                && matches!(r.sleep.state().consolidation, ConsolidationState::None)
            {
                let evidence = replay_evidence(p, &r.replay)?;
                if !evidence.batch.events.is_empty() {
                    // Same scheduled-work pressure projection used by the native
                    // scheduler; this is work admission, never reward/chemistry.
                    let mut due_homeostasis = record.biochemistry().homeostasis;
                    due_homeostasis.drives.fatigue = due_homeostasis
                        .drives
                        .fatigue
                        .max(config.fatigue_threshold.raw());
                    due_homeostasis.hormones.sleep_pressure = due_homeostasis
                        .hormones
                        .sleep_pressure
                        .max(config.sleep_pressure_threshold.raw());
                    let work = r.memory.run_bounded_sleep_transaction(
                        &SleepConsolidator::new(config)?,
                        &due_homeostasis,
                        tick,
                        &evidence,
                        &mut r.predictor,
                        &mut r.topology,
                    )?;
                    let structural = structural_sleep_evidence(p, &evidence.batch);
                    r.v11.apply_structural_phase(&structural)?;
                    let mut topology = r.v11.checkpoint();
                    topology.pending_lifetime_synapse = None;
                    r.v11 = GpuV11CausalState::restore(topology)?;
                    sleep_work_units = work.work_units;
                    let receipt = CognitiveWorkCounters::new(
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        u64::from(work.predictor_update_count),
                        u64::from(work.replay_event_count)
                            + u64::from(work.replay_eligibility_sample_count),
                        0,
                        0,
                        work.work_units,
                    )?
                    .into_receipt()?;
                    record.account_cognitive_work(
                        receipt,
                        CognitiveWorkCostPolicy::enabled(1.0e-6)?,
                    )?;
                }
                r.neural.consolidate_sleep(
                    founder,
                    &evidence,
                    &record.phenotype().development_state_at(tick)?,
                    record.phenotype().brain_genome.plasticity_parameters(),
                )?;
                r.replay.clear();
                let checkpoint = r.neural.checkpoint()?;
                let digest = json_digest(b"ALIFE-CPU-SLEEP-OUTPUT-V1", &checkpoint)?;
                let mut state = r.sleep.state();
                state.consolidation = ConsolidationState::Committed {
                    cycle_id: state.active_cycle_id,
                    output_generation: checkpoint.weight_generation,
                    output_digest: digest,
                };
                r.sleep = SleepController::restore(config, state)?;
                consolidations += 1;
            }
            let state = r.sleep.state();
            record.seal_sleep_phase(
                state.phase,
                state.active_cycle_id.max(state.last_consolidated_cycle_id),
                tick,
                sleep_work_units,
            )?;
            self.world.replace_organism_record_exact(record.clone())?;
            if phase_before != SleepPhase::Awake {
                body_events.insert(
                    id.raw(),
                    BodyEventDelta {
                        sleep_recovery: 1.,
                        ..BodyEventDelta::zero()
                    },
                );
            }
            if phase_before != SleepPhase::Awake || state.phase != SleepPhase::Awake {
                sleeping += 1;
                continue;
            }
            let sequence = ExperienceSequenceId(r.next_sequence);
            let receptors = record
                .biochemistry()
                .neural_receptor_frame(record.phenotype())?;
            let effects = NeuralReceptorEffects::from_frame(
                &receptors,
                &NeuralReceptorPhenotype::compile(p)?,
            )?;
            let (draft, candidate_binding) = if self.maze_program2346==Some(1) {
                perception_world.perception_frame_draft_with_maze_heading2346(id,tick,p.sensor_profile(),record.biochemistry().homeostasis,sequence.raw())?
            } else if self.candidate_opportunity_coverage2313.is_some() {
                perception_world.perception_frame_draft_with_candidate_coverage2313(
                    id,
                    tick,
                    p.sensor_profile(),
                    record.biochemistry().homeostasis,
                    sequence.raw(),
                )?
            } else {
                perception_world.perception_frame_draft_with_bindings(
                    id,
                    tick,
                    p.sensor_profile(),
                    record.biochemistry().homeostasis,
                )?
            };
            let mut maze_sense_work2346=0u64;
            let draft=if self.maze_program2346==Some(1) {
                maze_sense_work2346=1; // pending option read, once at this awake frame
                if let Some(history)=r.maze_blocked2346.take(){
                    history.validate()?;
                    if history.organism_id!=id||history.source_tick.raw()>=tick.raw(){return Err(CpuHostError::State("blocked history source mismatch"));}
                    maze_sense_work2346+=8; // seven source fields read, pending slot clear
                    let sense=alife_core::perception::MazeBlockedSense2346::new(id,history.source_tick,history.source_sequence,history.source_frame,history.primitive,history.blocked,record.embodiment().sensor_gain(SensorCapability::Touch))?;
                    draft.with_maze_blocked_sense2346(sense)?
                }else{draft}
            }else{draft};
            let mut active_intent_read_work2327=0;
            let candidate_binding=if self.candidate_opportunity_coverage2313==Some(3) {
                active_intent_read_work2327=1+r.active_approach_intent2327.as_ref().map_or(0, |_|1);
                candidate_binding.map(|b|b.with_active_approach(r.active_approach_intent2327.as_ref().map(|i|i.tracked_object_id)))
            }else{candidate_binding};
            let ordinary = draft.clone().with_semantic_context(None)?;
            let draft = prepare(draft, sequence)?;
            if draft.clone().with_semantic_context(None)? != ordinary {
                return Err(CpuHostError::State(
                    "semantic preparation changed ordinary perception",
                ));
            }
            let prepare_frame = if matches!(self.joint_membership2177, Some(14 | 15)) {
                prepare_canonical_frame_joint_ingest_direct2295
            } else if matches!(self.joint_membership2177, Some(12 | 13)) {
                prepare_canonical_frame_joint_ingest_outcome2286
            } else if self.joint_membership2177 == Some(10) {
                prepare_canonical_frame_joint_non_ingest_applied2278
            } else if self.joint_membership2177 == Some(11) {
                prepare_canonical_frame_joint_non_ingest_withheld2278
            } else if matches!(self.joint_membership2177, Some(8 | 9)) {
                prepare_canonical_frame_joint_contact_success2275
            } else if self.joint_membership2177 == Some(6) {
                prepare_canonical_frame_joint_contact_query_applied2273
            } else if self.joint_membership2177 == Some(7) {
                prepare_canonical_frame_joint_contact_query_withheld2273
            } else if self.joint_membership2177 == Some(4) {
                prepare_canonical_frame_joint_contact_applied2266
            } else if self.joint_membership2177 == Some(5) {
                prepare_canonical_frame_joint_contact_withheld2266
            } else if self.joint_membership2177 == Some(2) {
                prepare_canonical_frame_joint_applied
            } else if self.joint_membership2177 == Some(3) {
                prepare_canonical_frame_joint_withheld
            } else if self.joint_membership2177.is_some() {
                prepare_canonical_frame_with_joint_membership
            } else if r.memory_readout.is_some() {
                prepare_canonical_frame_with_family_sources
            } else {
                prepare_canonical_frame
            };
            let preparation = prepare_frame(
                &CanonicalPreparationInput {
                    draft,
                    candidate_binding,
                    sequence_id: sequence,
                    homeostasis: record.biochemistry().homeostasis,
                    hysteresis: r.hysteresis,
                    policy: attention_selection_policy_for(p),
                    neural_receptors: receptors.clone(),
                    receptor_effects: effects,
                },
                &r.memory,
                &r.topology,
                &r.predictor,
                false,
            );
            let prepared = preparation.result?;
            if let Some(h) = preparation.hysteresis {
                r.hysteresis = h;
            }
            let active_selection2327=prepared.active_approach_opportunity;
            let frame = prepared.frame;
            let recall = prepared.memory_recall;
            let mut activity = cpu_activity_budget(
                p,
                CpuActivityAnchor {
                    frame_digest: frame.frame_digest().0,
                    ..activity_anchor
                },
                record.biochemistry().homeostasis.drives.brain_atp,
                frame.candidates().len() as u32,
                recall.context().candidates.len() as u32,
            )?;
            if !activity.affordable {
                return Err(CpuHostError::State(
                    "CPU neural work exceeds canonical ATP availability",
                ));
            }
            let mut joint_membership2177 =
                self.joint_membership2177
                    .map(|_| JointMembershipLiveReceipt2177 {
                        schema_version: match self.joint_membership2177 {
                            Some(1) => 1,
                            Some(4 | 5) => 3,
                            Some(6 | 7) => 4,
                            Some(8 | 9) => 6,
                            Some(10 | 11) => 7,
                            Some(12 | 13) => 8,
                            Some(14 | 15) => 9,
                            _ => 2,
                        },
                        sources: recall.receipt().joint_membership_sources.clone(),
                        comparison: prepared.joint_comparison_sources.map(|queried_sources| {
                            JointMembershipComparison2182 {
                                apply: matches!(
                                    self.joint_membership2177,
                                    Some(2 | 4 | 6 | 8 | 9 | 10 | 12 | 13 | 14 | 15)
                                ),
                                applied_family2266: matches!(
                                    self.joint_membership2177,
                                    Some(4..=9)
                                )
                                .then_some(CandidateActionFamily::Contact),
                                applied_family_mask2278: matches!(
                                    self.joint_membership2177,
                                    Some(10 | 11)
                                )
                                .then_some(191),
                                queried_sources,
                            }
                        }),
                        success_projection2275: None,
                        ingest_outcome_projection2286: None,
                        query_work: recall.receipt().execution_evidence_work,
                        admission_planning_work: 0,
                        admission_reserved_work: 0,
                        admission_actual_work: 0,
                    });
            if let Some(receipt) = &joint_membership2177 {
                charge_joint_membership_work(&mut activity, receipt.query_work)?;
            }
            // Reserve the versioned worst-case research work before exporting
            // the bank or running any additional query. Default-off does none.
            let mut terminal_context2135 = if let Some(config) = &self.terminal_context2135 {
                Some(
                    if let Some(channels) = &r.terminal_channel2135 {
                        prepare_terminal_channels2135(
                            config,
                            &r.memory,
                            channels,
                            &recall,
                            activity
                                .available_atp_q16
                                .saturating_sub(activity.debit_atp_q16),
                        )
                    } else {
                        prepare_terminal2135(
                            config,
                            &r.memory,
                            &recall,
                            activity
                                .available_atp_q16
                                .saturating_sub(activity.debit_atp_q16),
                        )
                    }
                    .map_err(CpuHostError::Preparation)?,
                )
            } else {
                None
            };
            let mut context = encode_cpu_frame(
                p,
                &frame,
                &recall,
                &effects,
                activity.enabled_routes.clone(),
                r.v11
                    .sparse_spans()
                    .iter()
                    .flat_map(|s| {
                        s.edges.iter().map(|e| crate::TrainingFrozenSynapse {
                            source: e.source,
                            target: e.target,
                            route: e.route,
                            cadence: u32::from(
                                p.projections()[e.route as usize].update_cadence().raw(),
                            ),
                            effective_weight: e.weight,
                        })
                    })
                    .collect(),
            )?;
            if let Some((ordinary_frame, ordinary)) = prepared.joint_ordinary_success2275 {
                let projection = project_joint_contact_success2275(
                    &frame,
                    &recall,
                    &ordinary_frame,
                    &ordinary,
                    &mut context,
                    self.joint_membership2177 == Some(8),
                )?;
                charge_joint_membership_work(&mut activity, projection.work)?;
                let receipt = joint_membership2177
                    .as_mut()
                    .ok_or(CpuHostError::State("projection without joint mode"))?;
                receipt.query_work = receipt
                    .query_work
                    .checked_add(projection.work)
                    .ok_or(CpuHostError::State("projection work overflow"))?;
                receipt.success_projection2275 = Some(projection);
            }
            if let Some(basis) = prepared.joint_ingest_outcome_basis2286 {
                #[cfg(test)]
                if self.fail_ingest_outcome_binding2286 {
                    context.candidates.pop();
                }
                let mut projection = project_joint_ingest_outcome2286(
                    &frame,
                    &recall,
                    basis,
                    &mut context,
                    matches!(self.joint_membership2177, Some(12 | 14)),
                )?;
                if matches!(self.joint_membership2177, Some(14 | 15)) {
                    projection.cost_revision = 2;
                }
                // Complete deterministic transform work was precharged exactly
                // once by Core, in the same receipt used by actual physiology.
                joint_membership2177
                    .as_mut()
                    .ok_or(CpuHostError::State("Ingest projection without mode"))?
                    .ingest_outcome_projection2286 = Some(projection);
            }
            context.microstep_count = u32::from(activity.microsteps);
            let readout_work = if let Some(config) = &r.memory_readout {
                let work = super::memory_binding::bind_prepared_memory_readout(
                    config,
                    &frame,
                    &recall,
                    &mut context,
                )?;
                super::memory_binding::charge_memory_readout_work(&mut activity, work)?;
                work.total_ops
            } else {
                0
            };
            let mut output = r.neural.advance(founder, context, &effects)?;
            if let Some(receipt) = &terminal_context2135 {
                apply_terminal2135(receipt, &mut output.candidate_logits)
                    .map_err(CpuHostError::Preparation)?;
            }
            r.activity.remaining_atp_q16 = activity.available_atp_q16
                - activity.debit_atp_q16
                - terminal_context2135
                    .as_ref()
                    .map_or(0, |r| r.reserved_atp_q16);
            r.activity.source_frame_digest = frame.frame_digest().0;
            let sampling_counter = self.sampling_counter;
            let action = sample_cpu_joint_action(
                p,
                &frame,
                &output.candidate_logits,
                self.sampling_seed,
                sampling_counter,
                temperature,
            )?;
            self.sampling_counter = self
                .sampling_counter
                .checked_add(1)
                .ok_or(CpuHostError::State("sampling counter overflow"))?;
            let speech = if action.motor_candidates[3].is_some() {
                decode_cpu_speech(
                    founder,
                    &r.neural.neural_state().activations,
                    &output.replay_tick.effective_weight_offsets,
                )?
            } else {
                None
            };
            let index = action.representative;
            let candidate = &frame.candidates()[index as usize];
            let confidence =
                Confidence::new((candidate.sensor_confidence.raw() * 65535.).round() / 65535.)?;
            let command = candidate.to_command(id, confidence)?;
            let channels = p.candidate_decoder().factorized_motor_channels(p)?;
            let slots = action.motor_candidates.map(|i| i.map_or(0, |i| i + 1));
            let prompted = frame
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .any(|t| t.source_kind == UtteranceSourceKind::Player);
            let bundle = factorized_motor_bundle_for_candidates(
                id,
                sequence,
                tick,
                &frame,
                slots,
                &channels,
                &command,
                index,
                speech.as_ref(),
                prompted,
            )?;
            let joint = JointActionSelectionV1::new(slots, &bundle)?;
            let development = record.phenotype().development_state_at(tick)?;
            let pre = PreActionSnapshot::from_cpu_neural_frame(
                sequence,
                p.brain_class_id(),
                p.phenotype_hash(),
                record.phenotype().brain_genome.id,
                record.phenotype().brain_genome.schema_version,
                development.clone(),
                frame.clone(),
            )?;
            let work_neurons = activity.work.neuron_updates;
            let work_synapses = activity.work.synapse_ops;
            let decision = DecisionSnapshot::from_cpu_neural_selection(
                sequence,
                p.phenotype_hash(),
                output.dispatch_generation,
                output.active_activation_side,
                &frame,
                NeuralActionSelection {
                    candidate_index: index,
                    logit: output.candidate_logits[index as usize],
                    confidence,
                    active_tiles: u32::try_from(activity.work.tile_visits)
                        .map_err(|_| CpuHostError::State("CPU tile work overflow"))?,
                    active_synapses: u32::try_from(work_synapses)
                        .map_err(|_| CpuHostError::State("CPU synapse work overflow"))?,
                },
                command,
            )?
            .with_finalized_memory_recall(&frame, &recall, index)?;
            r.neural.stage_selection(
                p,
                &action,
                CpuDecisionBinding {
                    sequence_id: sequence,
                    frame_digest: frame.frame_digest(),
                    action_id: candidate.action_id,
                    candidate_feature_digest: candidate.feature_digest()?,
                    dispatch_generation: output.dispatch_generation,
                    active_activation_side: output.active_activation_side,
                    joint_selection: joint,
                },
                speech.is_some(),
            )?;
            let mut cognitive = recall
                .cognitive_context()
                .cloned()
                .ok_or(CpuHostError::State("missing canonical cognitive context"))?;
            let pre_context = cognitive.clone();
            let source = grounded_semantic_state_from_frame(&frame)?;
            let motor_condition = JointMotorCondition::from_bundle(&bundle)?;
            let mut forecast = r.predictor.predict(&source, &motor_condition)?;
            forecast.source_digest = frame.frame_digest().0;
            let mut neural_emissions = vec![
                NeuralEmission::new(
                    NeuralEmissionClass::RegionalArousal,
                    output.candidate_logits[index as usize].abs().tanh(),
                    confidence.raw(),
                )?,
                NeuralEmission::new(
                    NeuralEmissionClass::MotorCommitment,
                    confidence.raw(),
                    confidence.raw(),
                )?,
                NeuralEmission::new(NeuralEmissionClass::ExecutiveSustain, confidence.raw(), 1.)?,
            ];
            neural_emissions.extend(recall.recognized_social_emission());
            let emissions =
                NeuralEmissionFrame::new(tick, output.dispatch_generation, neural_emissions)?;
            let object_novelty = bundle
                .channels
                .iter()
                .filter_map(|c| c.target.and_then(|t| t.entity))
                .filter_map(|entity| {
                    let key = perception_world
                        .entity(entity)?
                        .tracking_provenance
                        .canonical_key();
                    let tracked = perception_world
                        .tracked_objects()
                        .records_for(id)?
                        .find(|r| r.tracking_key == key)?;
                    pre_context
                        .peripheral
                        .summaries
                        .iter()
                        .find(|s| {
                            s.identity
                                == StableFocusIdentity::TrackedObject(tracked.tracked_object_id)
                        })
                        .map(|s| (entity, s.salience.novelty.raw()))
                })
                .collect::<Vec<_>>();
            let receipt = self
                .world
                .apply_registered_motor_bundle_with_perceived_novelty_in_staged_tick(
                    &bundle,
                    record.world_entity_id(),
                    &emissions,
                    frame.sensory().channels.novelty_signal.raw(),
                    &object_novelty,
                )
                .map_err(|_| CpuHostError::State("canonical motor transaction rejected"))?;
            if let Some(evidence) = &mut joint_membership2177 {
                let (planning, reserved) = r.memory.reserve_joint_membership_admission_work(
                    &frame,
                    &bundle,
                    &receipt.joint,
                    sequence,
                    receipt.outcome_tick,
                )?;
                charge_joint_membership_work(&mut activity, planning + reserved)?;
                evidence.admission_planning_work = planning;
                evidence.admission_reserved_work = reserved;
                r.activity.remaining_atp_q16 = activity.available_atp_q16 - activity.debit_atp_q16;
            }
            let successor = grounded_successor_state(
                &self.world,
                record.world_entity_id(),
                &receipt.biology_after,
                p.sensor_profile(),
            )?;
            let target = PredictionTargetReceipt::for_successor(
                id,
                sequence,
                candidate.action_id,
                tick,
                frame.frame_digest().0,
                source,
                motor_condition,
                successor,
            )?;
            let prediction = r.predictor.observe_frozen(&target, &forecast)?;
            let error = apply_prediction_evidence(
                &mut cognitive,
                &target,
                &prediction.error,
                prediction.prediction.category_coverage,
            )?;
            let memory = recall.receipt();
            let dendritic_work = r
                .v11
                .dendritic_branches()
                .branches()
                .iter()
                .map(|b| 1 + b.inputs.len() as u64)
                .sum::<u64>()
                * u64::from(activity.microsteps);
            let structural_work = r
                .v11
                .sparse_spans()
                .iter()
                .flat_map(|s| &s.edges)
                .map(|e| {
                    if !activity.enabled_routes[e.route as usize] {
                        return 0;
                    }
                    (0..activity.microsteps as usize)
                        .filter(|&step| {
                            super::replay::route_fires(
                                p.projections()[e.route as usize].update_cadence().raw(),
                                step,
                            )
                        })
                        .count() as u64
                })
                .sum::<u64>();
            let approach_attention_feedback2325 = if matches!(self.candidate_opportunity_coverage2313,Some(2|3)) {
                let feedback = approach_attention_feedback(&frame, &action, &receipt.joint, &self.world, r.hysteresis)?;
                if feedback.nominated { r.hysteresis = feedback.after; }
                Some(feedback)
            } else { None };
            let active_approach_opportunity2327=if self.candidate_opportunity_coverage2313==Some(3) {
                let before=r.active_approach_intent2327.clone();
                // Intent presence/identity read, seven-field snapshot copy when present,
                // checked feedback/proof presence, nominated predicate, five row writes
                // and one resident assignment.
                let mut intent_work_units=active_intent_read_work2327+9+before.as_ref().map_or(0, |_|7);
                let feedback=approach_attention_feedback2325.as_ref().ok_or(CpuHostError::State("active mode lacks Approach feedback"))?;
                let after=if feedback.nominated {
                    // Seven source-field writes, four checked option extractions,
                    // and seven-field clone into resident state.
                    intent_work_units+=18;
                    Some(ActiveApproachIntent2327 {source_frame_digest:feedback.frame_digest,source_tick:feedback.source_tick,organism_id:feedback.organism_id,selected_candidate:feedback.selected_candidate.ok_or(CpuHostError::State("active source candidate missing"))?,executed:feedback.executed.ok_or(CpuHostError::State("active source execution missing"))?,tracked_object_id:feedback.tracked_object_id.ok_or(CpuHostError::State("active source identity missing"))?,target_entity:feedback.target_entity.ok_or(CpuHostError::State("active source target missing"))?})
                }else{None};
                r.active_approach_intent2327=after.clone();
                Some(ActiveApproachOpportunityRow2327 {frame_digest:frame.frame_digest(),before,after,selection:active_selection2327.ok_or(CpuHostError::State("active mode lacks selector proof"))?,intent_work_units})
            }else{None};
            let maze_outcome_work2346=if self.maze_program2346==Some(1){
                let (history,work)=MazeBlockedHistory2346::from_outcome(&frame,sequence,&receipt.joint)?;
                r.maze_blocked2346=history;work+1 // pending state assignment
            }else{0};
            let work = CognitiveWorkCounters::new(
                work_neurons,
                work_synapses,
                dendritic_work,
                cognitive.attention.budget_receipt.work_units
                    + maze_sense_work2346 + maze_outcome_work2346
                    + frame.maze_heading_work_units()
                    + frame.maze_blocked_sense2346().map_or(0, |s|s.work_units())
                    + active_approach_opportunity2327.as_ref().map_or(0,|r|r.intent_work_units+r.selection.work_units)
                    + approach_attention_feedback2325.as_ref().map_or(0, |r| r.work_units)
                    + frame.grip_proprioception().map_or(0, |r| r.work_units())
                    + frame
                        .candidate_opportunity_coverage()
                        .map_or(0, |r| r.work_units()),
                u64::from(memory.exact_bucket_reads)
                    + u64::from(memory.neighbor_bucket_reads)
                    + u64::from(memory.similarity_evaluations)
                    + memory.execution_evidence_work
                    + terminal_context2135
                        .as_ref()
                        .map_or(0, |r| r.reserved_work_units)
                    + readout_work
                    + joint_membership2177.as_ref().map_or(0, |r| {
                        r.admission_planning_work
                            + r.admission_reserved_work
                            + r.success_projection2275.as_ref().map_or(0, |p| p.work)
                    }),
                cognitive.concept.active_concepts.len() as u64,
                cognitive.gap.active_gaps.len() as u64,
                prediction.error.len() as u64,
                0,
                structural_work,
                1,
                0,
            )?
            .into_receipt()?;
            let mut after = self
                .world
                .organism_registry()
                .get(id)
                .cloned()
                .ok_or(CpuHostError::State("missing post-action organism"))?;
            let cost_policy = CognitiveWorkCostPolicy::enabled(1.0e-6)?;
            #[cfg(test)]
            let energy_before_cost = after.biochemistry().body.energy;
            #[cfg(test)]
            let body_before_cost = after.biochemistry().body;
            let _applied_cognitive_debit = after.account_cognitive_work(work, cost_policy)?;
            #[cfg(test)]
            let after_body_cost = after.biochemistry().body;
            #[cfg(test)]
            let cognitive_cost_capture = Some((
                cost_policy,
                _applied_cognitive_debit,
                energy_before_cost,
                after.biochemistry().body.energy,
            ));
            after.stage_prediction_residual_pulse(
                sequence,
                output.dispatch_generation,
                error,
                1.,
            )?;
            self.world.replace_organism_record_exact(after.clone())?;
            let physiology =
                MeasuredPhysiologyTransition::new(receipt.biology_before, *after.biochemistry())?;
            let mut outcome = PostActionOutcome::new(
                id,
                sequence,
                receipt.outcome_tick,
                receipt.succeeded,
                receipt.joint.execution,
                physiology.homeostatic_delta,
                SignedValence::ZERO,
                NormalizedScalar::new(if receipt.succeeded { 0. } else { 1. })?,
                NormalizedScalar::new(physiology.aversive_harm())?,
                physiology.energy_delta,
                NormalizedScalar::new(error)?,
            )?
            .with_measured_physiology(physiology)?;
            outcome.contradiction_observed = !receipt.succeeded;
            outcome = outcome.with_v12_joint(receipt.joint, work)?;
            let patch = ExperiencePatch::new_v12_with_decision(
                pre,
                decision,
                bundle,
                outcome,
                target.clone(),
                work,
                pre_context,
            )?;
            let credit_receptors = receptors;
            r.neural.apply_credit(
                founder,
                &patch,
                &credit_receptors,
                &development,
                after.phenotype().brain_genome.plasticity_parameters(),
            )?;
            r.language_grounding.observe_sealed(&patch)?;
            if let Some(evidence) = &mut joint_membership2177 {
                let admission = r
                    .memory
                    .observe_sealed_patch_with_joint_membership(&patch)?;
                evidence.admission_actual_work = admission.joint_membership_admission_work;
                #[cfg(test)]
                if self.fail_joint_membership_after_admission {
                    return Err(CpuHostError::State(
                        "test failure after actual membership admission",
                    ));
                }
                if evidence.admission_actual_work > evidence.admission_reserved_work {
                    return Err(CpuHostError::State(
                        "joint membership admission exceeded reservation",
                    ));
                }
            } else {
                r.memory.observe_sealed_patch(&patch)?;
            }
            if let Some(channels) = &mut r.terminal_channel2135 {
                let observation = terminal_context2135
                    .as_mut()
                    .and_then(|r| r.observation.as_mut())
                    .ok_or(CpuHostError::State(
                        "missing terminal observation admission",
                    ))?;
                channels
                    .observe(&patch, &recall, observation)
                    .map_err(CpuHostError::Preparation)?;
                #[cfg(test)]
                {
                    r.last_terminal_capture = Some((recall.clone(), patch.clone()));
                }
            }
            let topology_receipt = r.topology.observe_sealed_patch(&patch);
            if topology_receipt.rejected_invalid || topology_receipt.replay_rejected {
                return Err(CpuHostError::State("sealed topology observation rejected"));
            }
            r.topology.advance_lifecycle(receipt.outcome_tick)?;
            let checkpoint = r.neural.checkpoint()?;
            let credit = OutcomeCreditPacket::from_sealed_patch(&patch)?
                .with_biochemical_receptors(&credit_receptors)?;
            let selected = patch.decision().neural_evidence()?;
            r.replay.push(CpuReplayEvent {
                event: SleepReplayEvent {
                    sequence_id: sequence,
                    originating_tick: tick,
                    frame_digest: frame.frame_digest(),
                    candidate_feature_digest: candidate.feature_digest()?,
                    action_id: candidate.action_id,
                    family: selected.action_family,
                    modulator: credit.modulator(),
                },
                eligibility: p
                    .replay_capture_plan()
                    .global_synapse_ids()
                    .iter()
                    .map(|&id| {
                        let synapse = &p.synapses()[id as usize];
                        let decay = p.plasticity_receptors()[synapse.receptor_index() as usize]
                            .eligibility_decay();
                        let elapsed = (receipt.outcome_tick.raw() - tick.raw())
                            .min(u64::from(u32::MAX)) as f32;
                        (checkpoint.eligibility[id as usize] * decay.powf(elapsed))
                            .clamp(-1., 1.)
                            .mul_add(32767., 0.)
                            .round() as i16
                    })
                    .collect(),
                target,
            });
            if r.replay.len() > p.replay_capture_plan().event_capacity() as usize {
                r.replay.remove(0);
            }
            after.seal_cognitive_subsystems(
                receipt.outcome_tick,
                json_digest(b"ALIFE-CPU-NEURAL-STATE-V1", &checkpoint)?,
                r.memory.export_active_bank()?.active_bank_digest,
            )?;
            let parameters = after.phenotype().brain_genome.plasticity_parameters();
            let profile = parameters
                .action_candidate_credit_profile()
                .map_or_else(|| parameters.receptor_profile(), |p| p.receptor_profile());
            let biological_reward = profile.project(&credit.modulator().frame())?;
            self.world.replace_organism_record_exact(after)?;
            r.next_sequence = r
                .next_sequence
                .checked_add(1)
                .ok_or(CpuHostError::State("sequence overflow"))?;
            let memory_readout_research =
                r.memory_readout
                    .as_ref()
                    .map(|config| CpuMemoryReadoutReceiptV1 {
                        schema_version: 1,
                        readout_abi_version: config.readout_abi_version,
                        work_policy_version: config.work_policy_version,
                        construction_digest: r
                            .memory_readout_digest
                            .expect("configured readout was sealed"),
                        source_family: config.source_family,
                        destination_family: config.destination_family,
                        coefficient: config.coefficient,
                        additional_work_ops: readout_work,
                    });
            decisions.push(CpuCollectedDecision {
                active_approach_opportunity2327,
                approach_attention_feedback2325,
                memory_plan2242: founder
                    .phenotype()
                    .candidate_decoder()
                    .memory_channel()
                    .filter(|p| p.schema_version() == 2)
                    .map(|p| MemoryPlanRow2242 {
                        phenotype_hash: founder.phenotype().phenotype_hash(),
                        memory_bound: p.max_candidate_gain(),
                    }),
                joint_membership2177,
                #[cfg(test)]
                cognitive_cost_capture,
                #[cfg(test)]
                cognitive_body_cost_capture: Some((body_before_cost, after_body_cost)),
                executed_action_recall_work: memory.execution_evidence_work,
                executed_action_recall: recall
                    .executed_actions()
                    .iter()
                    .any(Option::is_some)
                    .then(|| recall.executed_actions().to_vec()),
                memory_readout_research,
                terminal_context2135,
                organism_id: id,
                sampling_counter: Some(sampling_counter),
                initial: output.initial,
                replay_tick: output.replay_tick,
                action,
                final_features: checkpoint.neural.activations.clone(),
                brain_work: activity.work,
                biological_reward,
                patch,
            });
        }
        // The batch uses one pre-action scene. Commit the identities actually
        // observed there along with its decisions; priming stays read-only.
        let tracked_objects = self
            .residents
            .keys()
            .map(|&id| {
                perception_world
                    .tracked_objects()
                    .save_state(OrganismId(id))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.world.restore_tracked_object_states(tracked_objects)?;
        let tick_after = self
            .world
            .try_advance_tick_with_body_events_in_staged_tick(&body_events)?;
        self.seal_all_residents()?;
        Ok(CpuCanonicalStep {
            tick_before: tick,
            tick_after,
            decisions,
            sleeping_organisms: sleeping,
            sleep_consolidations: consolidations,
        })
    }
}

#[cfg(test)]
#[path = "terminal_native_tests.rs"]
mod terminal_native_tests;
fn replay_evidence(
    p: &BrainPhenotype,
    events: &[CpuReplayEvent],
) -> Result<SleepReplayEvidence, ScaffoldContractError> {
    let mut spans = Vec::new();
    let mut samples = Vec::new();
    for (lane, &id) in p
        .replay_capture_plan()
        .global_synapse_ids()
        .iter()
        .enumerate()
    {
        let start = samples.len() as u32;
        for (index, event) in events.iter().enumerate() {
            samples.push(ReplayEligibilitySample {
                event_index: index as u16,
                eligibility_q15: *event
                    .eligibility
                    .get(lane)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?,
            });
        }
        spans.push(ReplaySynapseSpan {
            local_synapse_id: id,
            sample_start: start,
            sample_count: events.len() as u32,
            reserved: 0,
        });
    }
    let mut batch = BoundedReplayBatch {
        schema_version: 1,
        events: events.iter().map(|e| e.event).collect(),
        synapse_spans: spans,
        eligibility_samples: samples,
        canonical_digest: [0; 4],
    };
    batch.canonical_digest = batch.recompute_canonical_digest()?;
    SleepReplayEvidence::new(batch, events.iter().map(|e| e.target.clone()).collect())
}

pub(super) fn validate_record_founder(
    record: &alife_world::WorldOrganismRecord,
    founder: &CpuFoundationTrainer,
) -> Result<(), CpuHostError> {
    let p = founder.phenotype();
    let genome = &record.phenotype().brain_genome;
    if genome.brain_class_id != p.brain_class_id() {
        return Err(CpuHostError::State(
            "organism brain class differs from CPU founder",
        ));
    }
    let capacity = BrainCapacityClass::production_for_id(genome.brain_class_id)?;
    let development = foundation_construction_development(
        genome,
        &capacity,
        &record
            .phenotype()
            .development_state_at(record.biochemistry().tick)?,
    )?;
    let (compiled, _) = if p.foundation_abi().n2048_experimental_graph().is_some() {
        PhenotypeCompiler::compile_n2048_experimental_foundation_candidate(
            genome.clone(),
            development,
            founder.source_foundation.clone(),
        )?
    } else {
        PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome.clone(),
            development,
            founder.source_foundation.clone(),
        )?
    };
    if compiled.phenotype_hash() != p.phenotype_hash() {
        return Err(CpuHostError::State(
            "organism compiler identity differs from CPU founder; heterogeneous graphs require per-row compilation",
        ));
    }
    let inherited =
        record
            .genome()
            .n2048_foundation_candidate
            .as_ref()
            .ok_or(CpuHostError::State(
                "canonical organism lacks inherited candidate asset",
            ))?;
    if inherited.encode_canonical()? != founder.source_foundation.encode_canonical()? {
        return Err(CpuHostError::State(
            "organism inherited foundation differs from CPU founder",
        ));
    }
    Ok(())
}

fn resident_digest(
    r: &Resident,
    sampling_seed: u32,
    sampling_counter: u32,
) -> Result<[u64; 4], CpuHostError> {
    let ordinary = json_digest(
        b"ALIFE-CPU-CANONICAL-PRIVATE-STATE-V1",
        &(
            r.neural.checkpoint()?,
            r.v11.checkpoint(),
            &r.predictor,
            &r.activity,
            &r.sleep,
            &r.hysteresis,
            r.next_sequence,
            &r.replay,
            &r.language_grounding,
            r.topology.export_portable()?,
            *r.memory.compaction_checkpoint(),
            r.memory.export_staged_bank()?,
            r.memory.export_active_bank()?,
            sampling_seed,
            sampling_counter,
        ),
    )?;
    let ordinary=match &r.maze_blocked2346 {Some(history)=>json_digest(b"ALIFE-CPU-MAZE-BLOCKED-V1",&(ordinary,history))?,None=>ordinary};
    let ordinary=match &r.active_approach_intent2327 {Some(intent)=>json_digest(b"ALIFE-CPU-ACTIVE-APPROACH-INTENT-V1",&(ordinary,intent))?,None=>ordinary};
    let ordinary = match r.joint_comparison_mode {
        None => ordinary,
        // Cost revision 2 cannot resume a research life whose projection work
        // was absent from its biological intervals, even after header editing.
        Some(mode @ (8 | 9)) => json_digest(b"ALIFE-CPU-JOINT-SUCCESS-COST-V2", &(ordinary, mode))?,
        Some(mode @ (10 | 11)) => json_digest(
            b"ALIFE-CPU-JOINT-NON-INGEST-2278-V1",
            &(ordinary, mode, 191_u8),
        )?,
        Some(mode @ (12 | 13)) => json_digest(
            b"ALIFE-CPU-JOINT-INGEST-SUPPORTED-OUTCOME-2286-V1",
            &(
                ordinary,
                mode,
                CandidateActionFamily::Ingest,
                9_u8,
                1_u16,
                true,
            ),
        )?,
        Some(mode @ (14 | 15)) => json_digest(
            b"ALIFE-CPU-JOINT-INGEST-DIRECT-2295-V1",
            &(
                ordinary,
                mode,
                CandidateActionFamily::Ingest,
                9_u8,
                2_u16,
                true,
            ),
        )?,
        Some(mode) => json_digest(b"ALIFE-CPU-JOINT-COMPARISON-PRIVATE-V1", &(ordinary, mode))?,
    };
    let ordinary = match r.memory_readout_digest {
        None => ordinary,
        Some(digest) => json_digest(
            b"ALIFE-CPU-FROZEN-MEMORY-READOUT-PRIVATE-V1",
            &(ordinary, digest),
        )?,
    };
    match &r.terminal_channel2135 {
        None => Ok(ordinary),
        Some(channels) => json_digest(
            b"ALIFE-CPU-CHANNEL-TERMINAL-PRIVATE-V1",
            &(ordinary, channels),
        ),
    }
}
fn validate_resident_authority(
    record: &alife_world::WorldOrganismRecord,
    r: &Resident,
    sampling_seed: u32,
    sampling_counter: u32,
) -> Result<ResidentAuthority, CpuHostError> {
    let authority = resident_authority(r, sampling_seed, sampling_counter)?;
    authority.validate(record, sampling_seed, sampling_counter)?;
    Ok(authority)
}
fn resident_authority(
    r: &Resident,
    sampling_seed: u32,
    sampling_counter: u32,
) -> Result<ResidentAuthority, CpuHostError> {
    Ok(ResidentAuthority {
        brain: resident_digest(r, sampling_seed, sampling_counter)?,
        memory: r.memory.export_active_bank()?.active_bank_digest,
        sampling_seed,
        sampling_counter,
    })
}

fn structural_sleep_evidence(
    p: &BrainPhenotype,
    replay: &BoundedReplayBatch,
) -> Vec<alife_core::CoactivationEvidence> {
    let base_pairs = p
        .synapses()
        .iter()
        .map(|synapse| (synapse.source(), synapse.target()))
        .collect::<std::collections::BTreeSet<_>>();
    let active = replay
        .synapse_spans
        .iter()
        .filter_map(|span| {
            let start = usize::try_from(span.sample_start).ok()?;
            let count = usize::try_from(span.sample_count).ok()?;
            let samples = replay
                .eligibility_samples
                .get(start..start.checked_add(count)?)?;
            let eligibility = samples
                .iter()
                .map(|sample| u32::from(sample.eligibility_q15.unsigned_abs()))
                .max()
                .unwrap_or(0);
            if eligibility == 0 {
                return None;
            }
            let synapse = p
                .synapses()
                .get(usize::try_from(span.local_synapse_id).ok()?)?;
            Some((synapse.source(), synapse.target(), eligibility))
        })
        .take(32)
        .collect::<Vec<_>>();
    // Local growth proposals are anchored in replay eligibility. Unit
    // coactivation is proposal support, not an observed firing count.
    const CANONICAL_SLEEP_STRUCTURAL_REGION: u16 = 0;
    let mut structural_evidence = Vec::new();
    for pair in active.windows(2).take(32) {
        let (source, target, eligibility) = (pair[0].0, pair[1].1, pair[0].2);
        if source == target || base_pairs.contains(&(source, target)) {
            continue;
        }
        structural_evidence.push(CoactivationEvidence {
            region: CANONICAL_SLEEP_STRUCTURAL_REGION,
            source,
            target,
            coactivation: 1,
            eligibility,
            concept_gap_support: 0,
        });
    }
    if structural_evidence.is_empty() {
        if let Some((source, target, eligibility)) = active.first().copied() {
            for offset in 1..=8_u32 {
                let candidate_target =
                    (target % p.neuron_count()).wrapping_add(offset) % p.neuron_count();
                if candidate_target == source || base_pairs.contains(&(source, candidate_target)) {
                    continue;
                }
                structural_evidence.push(CoactivationEvidence {
                    region: CANONICAL_SLEEP_STRUCTURAL_REGION,
                    source,
                    target: candidate_target,
                    coactivation: 1,
                    eligibility,
                    concept_gap_support: 0,
                });
                break;
            }
        }
    }
    structural_evidence
}

#[cfg(test)]
#[path = "joint_membership_native_tests.rs"]
mod joint_membership_native_tests;

#[cfg(test)]
#[path = "grip_proprioception_tests.rs"]
mod grip_proprioception_tests;

#[cfg(test)]
#[path = "approach_attention_feedback_tests.rs"]
mod approach_attention_feedback_tests;

#[cfg(test)]
#[path="active_approach_opportunity_tests.rs"]
mod active_approach_opportunity_tests;
