//! v0 scaffold: structured action ABI contracts and CPU arbitration reference.

use core::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::{
    ensure_current_version, validate_finite, validate_optional_target, ActionId, Confidence,
    DurationTicks, Intensity, LobeIndex, NormalizedScalar, OrganismId, ScaffoldContractError,
    SchemaKind, SchemaVersions, Validate, Vec3f, WorldEntityId,
};

const DEFAULT_ACTION_DURATION_TICKS: DurationTicks = DurationTicks(1);
const DEFAULT_MIN_SCORE: f32 = 0.25;
const DEFAULT_MIN_CONFIDENCE: f32 = 0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    Idle,
    Hold,
    Rest,
    Inspect,
    Move,
    Interact,
    Vocalize,
    Write,
    Gesture,
}

impl ActionKind {
    pub const fn canonical_id(self) -> ActionId {
        match self {
            Self::Idle => ActionId(1),
            Self::Hold => ActionId(2),
            Self::Rest => ActionId(3),
            Self::Inspect => ActionId(4),
            Self::Move => ActionId(100),
            Self::Interact => ActionId(200),
            Self::Gesture => ActionId(300),
            Self::Vocalize => ActionId(400),
            Self::Write => ActionId(500),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionRegistryEntry {
    pub action_id: ActionId,
    pub kind: ActionKind,
}

impl ActionRegistryEntry {
    pub fn new(action_id: ActionId, kind: ActionKind) -> Result<Self, ScaffoldContractError> {
        action_id.validate()?;
        Ok(Self { action_id, kind })
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionArbitrationTraceRef(pub u64);

impl ActionArbitrationTraceRef {
    pub const INVALID: Self = Self(0);

    pub const fn new(raw: u64) -> Option<Self> {
        if raw == 0 {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn is_valid(self) -> bool {
        self.0 != 0
    }

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        if self.is_valid() {
            Ok(self)
        } else {
            Err(ScaffoldContractError::InvalidId)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeacherLessonResponseChannel {
    Speech,
    Writing,
    Gesture,
    Demonstration,
    Feedback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeacherLessonMetadata {
    pub teacher_entity: Option<WorldEntityId>,
    pub lesson_id: u64,
    pub response_channel: TeacherLessonResponseChannel,
}

impl TeacherLessonMetadata {
    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        validate_optional_target(self.teacher_entity)?;
        if self.lesson_id == 0 {
            Err(ScaffoldContractError::InvalidId)
        } else {
            Ok(self)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MotorPayloadKind {
    Speech,
    Writing,
    Vocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotorPayloadRef {
    pub kind: MotorPayloadKind,
    pub payload_id: u64,
    pub schema_version: u16,
}

impl MotorPayloadRef {
    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        if self.payload_id == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        if self.schema_version == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionTarget {
    pub entity: Option<WorldEntityId>,
    pub position: Option<Vec3f>,
}

impl ActionTarget {
    pub const NONE: Self = Self {
        entity: None,
        position: None,
    };

    pub const fn new(entity: Option<WorldEntityId>, position: Option<Vec3f>) -> Self {
        Self { entity, position }
    }

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        validate_optional_target(self.entity)?;
        if let Some(position) = self.position {
            position.validate()?;
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionBiasSource {
    MemoryExpectancy,
    EndocrineDrive,
    Salience,
    Debug,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionScoreBias {
    pub source: ActionBiasSource,
    pub score_delta: f32,
}

impl ActionScoreBias {
    pub fn memory_expectancy(score_delta: f32) -> Result<Self, ScaffoldContractError> {
        validate_finite(score_delta)?;
        Ok(Self {
            source: ActionBiasSource::MemoryExpectancy,
            score_delta,
        })
    }

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        validate_finite(self.score_delta)?;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InhibitionNeighborhood {
    pub ring_index: u32,
    pub radius: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionProposal {
    pub action_id: ActionId,
    pub kind: ActionKind,
    pub score: f32,
    pub confidence: Confidence,
    pub source_lobe: Option<LobeIndex>,
    pub source_mask: u32,
    pub target: ActionTarget,
    pub salience: NormalizedScalar,
    pub inhibition: Option<InhibitionNeighborhood>,
    pub intensity: Intensity,
    pub score_bias: Option<ActionScoreBias>,
    pub teacher_lesson: Option<TeacherLessonMetadata>,
    pub motor_payload: Option<MotorPayloadRef>,
    pub rationale_ref: Option<u32>,
}

impl ActionProposal {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        action_id: ActionId,
        kind: ActionKind,
        score: f32,
        confidence: Confidence,
        source_lobe: Option<LobeIndex>,
        source_mask: u32,
        target: ActionTarget,
        salience: NormalizedScalar,
    ) -> Result<Self, ScaffoldContractError> {
        action_id.validate()?;
        validate_finite(score)?;
        Confidence::new(confidence.raw())?;
        NormalizedScalar::new(salience.raw())?;
        Ok(Self {
            action_id,
            kind,
            score,
            confidence,
            source_lobe,
            source_mask,
            target,
            salience,
            inhibition: None,
            intensity: Intensity::new(1.0)?,
            score_bias: None,
            teacher_lesson: None,
            motor_payload: None,
            rationale_ref: None,
        })
    }

    pub const fn with_inhibition(mut self, inhibition: Option<InhibitionNeighborhood>) -> Self {
        self.inhibition = inhibition;
        self
    }

    pub fn with_intensity(mut self, intensity: Intensity) -> Self {
        self.intensity = intensity;
        self
    }

    pub fn with_score_bias(mut self, score_bias: ActionScoreBias) -> Self {
        self.score_bias = Some(score_bias);
        self
    }

    pub const fn with_teacher_lesson(
        mut self,
        teacher_lesson: Option<TeacherLessonMetadata>,
    ) -> Self {
        self.teacher_lesson = teacher_lesson;
        self
    }

    pub const fn with_motor_payload(mut self, motor_payload: Option<MotorPayloadRef>) -> Self {
        self.motor_payload = motor_payload;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionCommand {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub action_id: ActionId,
    pub kind: ActionKind,
    pub target_entity: Option<WorldEntityId>,
    pub target_position: Option<Vec3f>,
    pub intensity: Intensity,
    pub duration_ticks: DurationTicks,
    pub confidence: Confidence,
    pub source_mask: u32,
    pub teacher_lesson: Option<TeacherLessonMetadata>,
    pub motor_payload: Option<MotorPayloadRef>,
    pub arbitration_trace: Option<ActionArbitrationTraceRef>,
}

impl ActionCommand {
    pub const ABI_VERSION: u16 = SchemaVersions::CURRENT.action_abi.0;

    pub fn new(
        organism_id: OrganismId,
        kind: ActionKind,
        target_entity: Option<WorldEntityId>,
        confidence: Confidence,
        duration_ticks: DurationTicks,
    ) -> Result<Self, ScaffoldContractError> {
        Self::structured(
            organism_id,
            kind.canonical_id(),
            kind,
            ActionTarget::new(target_entity, None),
            Intensity::new(1.0)?,
            duration_ticks,
            confidence,
            0,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn structured(
        organism_id: OrganismId,
        action_id: ActionId,
        kind: ActionKind,
        target: ActionTarget,
        intensity: Intensity,
        duration_ticks: DurationTicks,
        confidence: Confidence,
        source_mask: u32,
        teacher_lesson: Option<TeacherLessonMetadata>,
        motor_payload: Option<MotorPayloadRef>,
        arbitration_trace: Option<ActionArbitrationTraceRef>,
    ) -> Result<Self, ScaffoldContractError> {
        let command = Self {
            abi_version: Self::ABI_VERSION,
            organism_id,
            action_id,
            kind,
            target_entity: target.entity,
            target_position: target.position,
            intensity,
            duration_ticks,
            confidence,
            source_mask,
            teacher_lesson,
            motor_payload,
            arbitration_trace,
        };
        command.validate_contract()?;
        Ok(command)
    }
}

impl Validate for ActionCommand {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::ActionAbi, self.abi_version)?;
        self.organism_id.validate()?;
        self.action_id.validate()?;
        ActionTarget::new(self.target_entity, self.target_position).validate()?;
        Intensity::new(self.intensity.raw())?;
        if self.duration_ticks.raw() == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Confidence::new(self.confidence.raw())?;
        if let Some(teacher_lesson) = self.teacher_lesson {
            teacher_lesson.validate()?;
        }
        if let Some(motor_payload) = self.motor_payload {
            motor_payload.validate()?;
        }
        if let Some(arbitration_trace) = self.arbitration_trace {
            arbitration_trace.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionDecisionStatus {
    Selected,
    FallbackSelected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionFallbackReason {
    NoEligibleProposal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SuppressionReason {
    InvalidActionId,
    InvalidTarget,
    InvalidConfidence,
    InvalidIntensity,
    InvalidTeacherLesson,
    InvalidMotorPayload,
    NonFiniteScore,
    BelowScoreThreshold,
    BelowConfidenceThreshold,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SuppressedProposal {
    pub proposal_index: usize,
    pub reason: SuppressionReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionInhibitionSample {
    pub proposal_index: usize,
    pub raw_score: f32,
    pub bias_delta: f32,
    pub output_score: f32,
    pub confidence: Confidence,
    pub source_lobe: Option<LobeIndex>,
    pub ring_index: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionWtaResult {
    pub selected_proposal_index: Option<usize>,
    pub selected_action_id: Option<ActionId>,
    pub selected_score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionArbitrationTrace {
    pub trace_ref: ActionArbitrationTraceRef,
    pub inhibition_inputs: Vec<ActionInhibitionSample>,
    pub inhibition_outputs: Vec<ActionInhibitionSample>,
    pub wta_result: ActionWtaResult,
    pub score_threshold: f32,
    pub confidence_threshold: f32,
    pub tied_proposal_indices: Vec<usize>,
    pub suppressed_proposals: Vec<SuppressedProposal>,
    pub tie_breaker_seed: u64,
    pub tie_breaker_index: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RankedActionProposal {
    pub proposal_index: usize,
    pub proposal: ActionProposal,
    pub final_score: f32,
    pub tie_break_key: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionDecision {
    pub selected: ActionCommand,
    pub rejected_top_proposal: Option<RankedActionProposal>,
    pub ranked_top_proposals: Vec<RankedActionProposal>,
    pub fallback_reason: Option<ActionFallbackReason>,
    pub status: ActionDecisionStatus,
    pub trace: ActionArbitrationTrace,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionArbitrationConfig {
    pub min_score: f32,
    pub min_confidence: Confidence,
    pub default_duration_ticks: DurationTicks,
    pub fallback_kind: ActionKind,
    pub fallback_confidence: Confidence,
    pub fallback_intensity: Intensity,
    pub trace_ref: ActionArbitrationTraceRef,
    pub tie_breaker_seed: u64,
}

impl Default for ActionArbitrationConfig {
    fn default() -> Self {
        Self {
            min_score: DEFAULT_MIN_SCORE,
            min_confidence: Confidence(DEFAULT_MIN_CONFIDENCE),
            default_duration_ticks: DEFAULT_ACTION_DURATION_TICKS,
            fallback_kind: ActionKind::Inspect,
            fallback_confidence: Confidence(0.25),
            fallback_intensity: Intensity(0.0),
            trace_ref: ActionArbitrationTraceRef(1),
            tie_breaker_seed: 0,
        }
    }
}

pub fn cpu_reference_arbitrate(
    organism_id: OrganismId,
    proposals: &[ActionProposal],
    config: ActionArbitrationConfig,
) -> Result<ActionDecision, ScaffoldContractError> {
    organism_id.validate()?;
    validate_finite(config.min_score)?;
    Confidence::new(config.min_confidence.raw())?;
    if config.default_duration_ticks.raw() == 0 {
        return Err(ScaffoldContractError::ScalarOutOfRange);
    }
    config.trace_ref.validate()?;

    let mut inhibition_inputs = Vec::with_capacity(proposals.len());
    let mut inhibition_outputs = Vec::with_capacity(proposals.len());
    let mut suppressed_proposals = Vec::new();
    let mut ranked_top_proposals = Vec::new();

    for (proposal_index, proposal) in proposals.iter().copied().enumerate() {
        let bias_delta = match proposal.score_bias {
            Some(score_bias) => {
                if score_bias.validate().is_err() {
                    suppressed_proposals.push(SuppressedProposal {
                        proposal_index,
                        reason: SuppressionReason::NonFiniteScore,
                    });
                    continue;
                }
                score_bias.score_delta
            }
            None => 0.0,
        };
        let output_score = proposal.score + bias_delta;
        if validate_finite(proposal.score).is_err() || validate_finite(output_score).is_err() {
            suppressed_proposals.push(SuppressedProposal {
                proposal_index,
                reason: SuppressionReason::NonFiniteScore,
            });
            continue;
        }

        let sample = ActionInhibitionSample {
            proposal_index,
            raw_score: proposal.score,
            bias_delta,
            output_score,
            confidence: proposal.confidence,
            source_lobe: proposal.source_lobe,
            ring_index: proposal.inhibition.map(|value| value.ring_index),
        };
        inhibition_inputs.push(sample);
        inhibition_outputs.push(sample);

        if proposal.action_id.validate().is_err() {
            suppressed_proposals.push(SuppressedProposal {
                proposal_index,
                reason: SuppressionReason::InvalidActionId,
            });
            continue;
        }
        if proposal.target.validate().is_err() {
            suppressed_proposals.push(SuppressedProposal {
                proposal_index,
                reason: SuppressionReason::InvalidTarget,
            });
            continue;
        }
        if Confidence::new(proposal.confidence.raw()).is_err() {
            suppressed_proposals.push(SuppressedProposal {
                proposal_index,
                reason: SuppressionReason::InvalidConfidence,
            });
            continue;
        }
        if Intensity::new(proposal.intensity.raw()).is_err() {
            suppressed_proposals.push(SuppressedProposal {
                proposal_index,
                reason: SuppressionReason::InvalidIntensity,
            });
            continue;
        }
        if let Some(teacher_lesson) = proposal.teacher_lesson {
            if teacher_lesson.validate().is_err() {
                suppressed_proposals.push(SuppressedProposal {
                    proposal_index,
                    reason: SuppressionReason::InvalidTeacherLesson,
                });
                continue;
            }
        }
        if let Some(motor_payload) = proposal.motor_payload {
            if motor_payload.validate().is_err() {
                suppressed_proposals.push(SuppressedProposal {
                    proposal_index,
                    reason: SuppressionReason::InvalidMotorPayload,
                });
                continue;
            }
        }
        if output_score < config.min_score {
            suppressed_proposals.push(SuppressedProposal {
                proposal_index,
                reason: SuppressionReason::BelowScoreThreshold,
            });
            continue;
        }
        if proposal.confidence.raw() < config.min_confidence.raw() {
            suppressed_proposals.push(SuppressedProposal {
                proposal_index,
                reason: SuppressionReason::BelowConfidenceThreshold,
            });
            continue;
        }

        ranked_top_proposals.push(RankedActionProposal {
            proposal_index,
            proposal,
            final_score: output_score,
            tie_break_key: deterministic_tie_key(config.tie_breaker_seed, proposal, proposal_index),
        });
    }

    ranked_top_proposals.sort_by(compare_ranked_proposals);

    let selected_ranked = ranked_top_proposals.first().copied();
    let tied_proposal_indices = tied_proposal_indices(&ranked_top_proposals);
    let tie_breaker_index = selected_ranked.map(|ranked| ranked.proposal_index);

    let (selected, fallback_reason, status) = if let Some(ranked) = selected_ranked {
        let proposal = ranked.proposal;
        (
            ActionCommand::structured(
                organism_id,
                proposal.action_id,
                proposal.kind,
                proposal.target,
                proposal.intensity,
                config.default_duration_ticks,
                proposal.confidence,
                proposal.source_mask,
                proposal.teacher_lesson,
                proposal.motor_payload,
                Some(config.trace_ref),
            )?,
            None,
            ActionDecisionStatus::Selected,
        )
    } else {
        (
            ActionCommand::structured(
                organism_id,
                config.fallback_kind.canonical_id(),
                config.fallback_kind,
                ActionTarget::NONE,
                config.fallback_intensity,
                config.default_duration_ticks,
                config.fallback_confidence,
                0,
                None,
                None,
                Some(config.trace_ref),
            )?,
            Some(ActionFallbackReason::NoEligibleProposal),
            ActionDecisionStatus::FallbackSelected,
        )
    };

    let rejected_top_proposal = selected_ranked.and_then(|selected_ranked| {
        ranked_top_proposals
            .iter()
            .copied()
            .find(|ranked| ranked.proposal_index != selected_ranked.proposal_index)
    });

    let trace = ActionArbitrationTrace {
        trace_ref: config.trace_ref,
        inhibition_inputs,
        inhibition_outputs,
        wta_result: ActionWtaResult {
            selected_proposal_index: selected_ranked.map(|ranked| ranked.proposal_index),
            selected_action_id: selected_ranked.map(|ranked| ranked.proposal.action_id),
            selected_score: selected_ranked.map_or(0.0, |ranked| ranked.final_score),
        },
        score_threshold: config.min_score,
        confidence_threshold: config.min_confidence.raw(),
        tied_proposal_indices,
        suppressed_proposals,
        tie_breaker_seed: config.tie_breaker_seed,
        tie_breaker_index,
    };

    Ok(ActionDecision {
        selected,
        rejected_top_proposal,
        ranked_top_proposals,
        fallback_reason,
        status,
        trace,
    })
}

fn compare_ranked_proposals(a: &RankedActionProposal, b: &RankedActionProposal) -> Ordering {
    b.final_score
        .total_cmp(&a.final_score)
        .then_with(|| {
            b.proposal
                .confidence
                .raw()
                .total_cmp(&a.proposal.confidence.raw())
        })
        .then_with(|| a.tie_break_key.cmp(&b.tie_break_key))
        .then_with(|| a.proposal_index.cmp(&b.proposal_index))
}

fn tied_proposal_indices(ranked: &[RankedActionProposal]) -> Vec<usize> {
    let Some(top) = ranked.first() else {
        return Vec::new();
    };
    ranked
        .iter()
        .filter(|candidate| {
            candidate.final_score == top.final_score
                && candidate.proposal.confidence.raw() == top.proposal.confidence.raw()
        })
        .map(|candidate| candidate.proposal_index)
        .collect()
}

fn deterministic_tie_key(seed: u64, proposal: ActionProposal, proposal_index: usize) -> u64 {
    let mut value = seed
        ^ ((proposal.action_id.raw() as u64) << 32)
        ^ proposal.kind.canonical_id().raw() as u64
        ^ proposal_index as u64;
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}
