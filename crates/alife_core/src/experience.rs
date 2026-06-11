//! v0 scaffold: causal three-phase ExperiencePatch runtime contract.

use serde::{Deserialize, Serialize};

use crate::{
    ensure_current_version, validate_finite, validate_optional_target, ActionArbitrationTrace,
    ActionCommand, ActionDecision, ActionDecisionStatus, ActionProposal, BrainClassId,
    BrainClassSpec, BrainGenome, BrainScaleTier, ConceptCellId, Confidence, DevelopmentState,
    DriveDelta, ExperienceSequenceId, GenomeId, HomeostaticDelta, HomeostaticSnapshot, LobeLayout,
    MemoryId, NormalizedScalar, OrganismId, Pose, RankedActionProposal, RoutingMatrix,
    ScaffoldContractError, SchemaKind, SchemaVersions, SensoryAbiVersion, SensorySnapshot,
    SignedValence, TeacherPerceptionChannel, Tick, Validate, Vec3f, Velocity, WeightSplitContract,
    WorldEntityId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperiencePatchPhase {
    PreActionSnapshot,
    DecisionSnapshot,
    PostActionOutcome,
    Sealed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperiencePatchHeader {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub world_tick: Tick,
    pub phase: ExperiencePatchPhase,
}

impl ExperiencePatchHeader {
    pub const ABI_VERSION: u16 = SchemaVersions::CURRENT.experience.0;

    pub fn new(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        world_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        Self::for_phase(
            organism_id,
            sequence_id,
            world_tick,
            ExperiencePatchPhase::PreActionSnapshot,
        )
    }

    pub fn for_phase(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        world_tick: Tick,
        phase: ExperiencePatchPhase,
    ) -> Result<Self, ScaffoldContractError> {
        let header = Self {
            abi_version: Self::ABI_VERSION,
            organism_id,
            sequence_id,
            world_tick,
            phase,
        };
        header.validate_contract()?;
        Ok(header)
    }
}

impl Validate for ExperiencePatchHeader {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::Experience, self.abi_version)?;
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryExpectancySnapshot {
    pub expected_valence: SignedValence,
    pub predicted_drive_delta: DriveDelta,
    pub affordance_bias: NormalizedScalar,
    pub danger_bias: NormalizedScalar,
    pub safety_bias: NormalizedScalar,
    pub salience_hint: NormalizedScalar,
}

impl MemoryExpectancySnapshot {
    pub const fn neutral() -> Self {
        Self {
            expected_valence: SignedValence(0.0),
            predicted_drive_delta: DriveDelta::zero(),
            affordance_bias: NormalizedScalar(0.0),
            danger_bias: NormalizedScalar(0.0),
            safety_bias: NormalizedScalar(0.0),
            salience_hint: NormalizedScalar(0.0),
        }
    }
}

impl Validate for MemoryExpectancySnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        SignedValence::new(self.expected_valence.raw())?;
        self.predicted_drive_delta.validate_contract()?;
        NormalizedScalar::new(self.affordance_bias.raw())?;
        NormalizedScalar::new(self.danger_bias.raw())?;
        NormalizedScalar::new(self.safety_bias.raw())?;
        NormalizedScalar::new(self.salience_hint.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreActionSnapshot {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub tick: Tick,
    pub brain_class_id: BrainClassId,
    pub brain_scale_tier: BrainScaleTier,
    pub brain_neuron_count: u32,
    pub max_active_synapses: u32,
    pub max_active_microtiles: u32,
    pub routing_schema_version: u16,
    pub lobe_layout: LobeLayout,
    pub routing_matrix: RoutingMatrix,
    pub genome_id: GenomeId,
    pub genome_schema_version: u16,
    pub development_state: DevelopmentState,
    pub weight_split: WeightSplitContract,
    pub sensory_abi_version: SensoryAbiVersion,
    pub chemistry_schema_version: u16,
    pub body_pose: Pose,
    pub body_velocity: Velocity,
    pub homeostasis: HomeostaticSnapshot,
    pub sensory: SensorySnapshot,
    pub memory_expectancy: MemoryExpectancySnapshot,
}

impl PreActionSnapshot {
    pub const ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        tick: Tick,
        brain_class: BrainClassSpec,
        genome: BrainGenome,
        development_state: DevelopmentState,
        weight_split: WeightSplitContract,
        body_pose: Pose,
        body_velocity: Velocity,
        homeostasis: HomeostaticSnapshot,
        sensory: SensorySnapshot,
        memory_expectancy: MemoryExpectancySnapshot,
    ) -> Result<Self, ScaffoldContractError> {
        let snapshot = Self {
            abi_version: Self::ABI_VERSION,
            organism_id,
            sequence_id,
            tick,
            brain_class_id: brain_class.id,
            brain_scale_tier: brain_class.tier,
            brain_neuron_count: brain_class.neuron_count,
            max_active_synapses: brain_class.max_active_synapses,
            max_active_microtiles: brain_class.max_active_microtiles,
            routing_schema_version: brain_class.routing_schema_version,
            lobe_layout: brain_class.lobe_layout,
            routing_matrix: brain_class.routing_matrix,
            genome_id: genome.id,
            genome_schema_version: genome.schema_version,
            development_state,
            weight_split,
            sensory_abi_version: sensory.abi_version,
            chemistry_schema_version: homeostasis.schema_version,
            body_pose,
            body_velocity,
            homeostasis,
            sensory,
            memory_expectancy,
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }
}

impl Validate for PreActionSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::Experience, self.abi_version)?;
        ensure_current_version(SchemaKind::SensoryAbi, self.sensory_abi_version.raw())?;
        ensure_current_version(SchemaKind::Chemistry, self.chemistry_schema_version)?;
        ensure_current_version(SchemaKind::Genome, self.genome_schema_version)?;
        ensure_current_version(SchemaKind::NeuralProjection, self.routing_schema_version)?;
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        self.brain_class_id.validate()?;
        self.genome_id.validate()?;
        self.development_state.validate_contract()?;
        self.weight_split.validate_contract()?;
        if self.weight_split.genetic_fixed.descriptor.brain_class_id != self.brain_class_id
            || self
                .weight_split
                .lifetime_consolidated
                .descriptor
                .brain_class_id
                != self.brain_class_id
            || self.weight_split.h_operational.descriptor.brain_class_id != self.brain_class_id
            || self.weight_split.h_shadow.descriptor.brain_class_id != self.brain_class_id
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.lobe_layout
            .validate_for_neuron_count(self.brain_neuron_count)?;
        self.routing_matrix.validate_for_layout(&self.lobe_layout)?;
        self.body_pose.validate()?;
        self.body_velocity.validate()?;
        self.homeostasis.validate_contract()?;
        self.sensory.validate_contract()?;
        self.memory_expectancy.validate_contract()?;
        if self.homeostasis.tick != self.tick || self.sensory.tick != self.tick {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        if self.sensory.organism_id != self.organism_id {
            return Err(ScaffoldContractError::MismatchedCreatureId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionSnapshot {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub decision_tick: Tick,
    pub action_abi_version: u16,
    pub proposals: Vec<ActionProposal>,
    pub selected_action: ActionCommand,
    pub rejected_top_proposal: Option<RankedActionProposal>,
    pub ranked_top_proposals: Vec<RankedActionProposal>,
    pub arbitration_trace: ActionArbitrationTrace,
    pub confidence: Confidence,
    pub status: ActionDecisionStatus,
}

impl DecisionSnapshot {
    pub const ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION;

    pub fn from_action_decision(
        sequence_id: ExperienceSequenceId,
        decision_tick: Tick,
        proposals: Vec<ActionProposal>,
        decision: ActionDecision,
    ) -> Result<Self, ScaffoldContractError> {
        let snapshot = Self {
            abi_version: Self::ABI_VERSION,
            organism_id: decision.selected.organism_id,
            sequence_id,
            decision_tick,
            action_abi_version: ActionCommand::ABI_VERSION,
            proposals,
            confidence: decision.selected.confidence,
            selected_action: decision.selected,
            rejected_top_proposal: decision.rejected_top_proposal,
            ranked_top_proposals: decision.ranked_top_proposals,
            arbitration_trace: decision.trace,
            status: decision.status,
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }
}

impl Validate for DecisionSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::Experience, self.abi_version)?;
        ensure_current_version(SchemaKind::ActionAbi, self.action_abi_version)?;
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        self.selected_action.validate_contract()?;
        if self.selected_action.organism_id != self.organism_id {
            return Err(ScaffoldContractError::MismatchedCreatureId);
        }
        Confidence::new(self.confidence.raw())?;
        validate_action_trace(&self.arbitration_trace)?;
        validate_action_decision_consistency(self)?;
        validate_action_proposals(&self.proposals)?;
        if let Some(proposal) = self.rejected_top_proposal {
            validate_ranked_proposal(proposal)?;
        }
        for proposal in &self.ranked_top_proposals {
            validate_ranked_proposal(*proposal)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhysicalContactKind {
    None,
    Touch,
    Collision,
    Blocked,
    Consumed,
    Moved,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhysicalActionOutcome {
    pub contact: PhysicalContactKind,
    pub target_entity: Option<WorldEntityId>,
    pub displacement: Vec3f,
    pub collision_normal: Option<Vec3f>,
    pub energy_cost: NormalizedScalar,
}

impl Validate for PhysicalActionOutcome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_optional_target(self.target_entity)?;
        self.displacement.validate()?;
        if let Some(normal) = self.collision_normal {
            normal.validate()?;
        }
        NormalizedScalar::new(self.energy_cost.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConceptHint {
    pub concept_id: ConceptCellId,
    pub salience: NormalizedScalar,
    pub contradiction_observed: bool,
}

impl Validate for ConceptHint {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.concept_id.validate()?;
        NormalizedScalar::new(self.salience.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MemoryHint {
    pub memory_id: MemoryId,
    pub salience: NormalizedScalar,
}

impl Validate for MemoryHint {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.memory_id.validate()?;
        NormalizedScalar::new(self.salience.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TeacherFeedbackObservation {
    pub channel: TeacherPerceptionChannel,
    pub source_entity: Option<WorldEntityId>,
    pub valence: SignedValence,
    pub confidence: Confidence,
}

impl Validate for TeacherFeedbackObservation {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_optional_target(self.source_entity)?;
        SignedValence::new(self.valence.raw())?;
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostActionOutcome {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub outcome_tick: Tick,
    pub success: bool,
    pub physical: PhysicalActionOutcome,
    pub homeostatic_delta: HomeostaticDelta,
    pub reward_valence: SignedValence,
    pub frustration_delta: NormalizedScalar,
    pub pain_delta: NormalizedScalar,
    pub energy_delta: SignedValence,
    pub prediction_error: NormalizedScalar,
    pub contradiction_observed: bool,
    pub concept_hints: Vec<ConceptHint>,
    pub memory_hints: Vec<MemoryHint>,
    pub teacher_feedback: Option<TeacherFeedbackObservation>,
}

impl PostActionOutcome {
    pub const ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION;

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        outcome_tick: Tick,
        success: bool,
        physical: PhysicalActionOutcome,
        homeostatic_delta: HomeostaticDelta,
        reward_valence: SignedValence,
        frustration_delta: NormalizedScalar,
        pain_delta: NormalizedScalar,
        energy_delta: SignedValence,
        prediction_error: NormalizedScalar,
    ) -> Result<Self, ScaffoldContractError> {
        let outcome = Self {
            abi_version: Self::ABI_VERSION,
            organism_id,
            sequence_id,
            outcome_tick,
            success,
            physical,
            homeostatic_delta,
            reward_valence,
            frustration_delta,
            pain_delta,
            energy_delta,
            prediction_error,
            contradiction_observed: false,
            concept_hints: Vec::new(),
            memory_hints: Vec::new(),
            teacher_feedback: None,
        };
        outcome.validate_contract()?;
        Ok(outcome)
    }
}

impl Validate for PostActionOutcome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::Experience, self.abi_version)?;
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        self.physical.validate_contract()?;
        self.homeostatic_delta.validate_contract()?;
        SignedValence::new(self.reward_valence.raw())?;
        NormalizedScalar::new(self.frustration_delta.raw())?;
        NormalizedScalar::new(self.pain_delta.raw())?;
        SignedValence::new(self.energy_delta.raw())?;
        NormalizedScalar::new(self.prediction_error.raw())?;
        for hint in &self.concept_hints {
            hint.validate_contract()?;
        }
        for hint in &self.memory_hints {
            hint.validate_contract()?;
        }
        if let Some(feedback) = self.teacher_feedback {
            feedback.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperiencePatchBuilder {
    sequence_id: ExperienceSequenceId,
    pre_action: Option<PreActionSnapshot>,
    decision: Option<DecisionSnapshot>,
    outcome: Option<PostActionOutcome>,
    next_phase: ExperiencePatchPhase,
}

impl ExperiencePatchBuilder {
    pub fn new(sequence_id: ExperienceSequenceId) -> Self {
        Self {
            sequence_id,
            pre_action: None,
            decision: None,
            outcome: None,
            next_phase: ExperiencePatchPhase::PreActionSnapshot,
        }
    }

    pub fn record_pre_action(
        mut self,
        pre_action: PreActionSnapshot,
    ) -> Result<Self, ScaffoldContractError> {
        if self.next_phase != ExperiencePatchPhase::PreActionSnapshot {
            return Err(ScaffoldContractError::UnorderedExperiencePhase);
        }
        self.sequence_id.validate()?;
        pre_action.validate_contract()?;
        if pre_action.sequence_id != self.sequence_id {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.pre_action = Some(pre_action);
        self.next_phase = ExperiencePatchPhase::DecisionSnapshot;
        Ok(self)
    }

    pub fn record_decision(
        mut self,
        decision: DecisionSnapshot,
    ) -> Result<Self, ScaffoldContractError> {
        if self.next_phase != ExperiencePatchPhase::DecisionSnapshot {
            return Err(ScaffoldContractError::UnorderedExperiencePhase);
        }
        let pre_action = self
            .pre_action
            .as_ref()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        decision.validate_contract()?;
        validate_same_sequence(self.sequence_id, decision.sequence_id)?;
        validate_same_creature(pre_action.organism_id, decision.organism_id)?;
        Tick::validate_monotonic(pre_action.tick, decision.decision_tick)?;
        self.decision = Some(decision);
        self.next_phase = ExperiencePatchPhase::PostActionOutcome;
        Ok(self)
    }

    pub fn record_outcome(
        mut self,
        outcome: PostActionOutcome,
    ) -> Result<Self, ScaffoldContractError> {
        if self.next_phase != ExperiencePatchPhase::PostActionOutcome {
            return Err(ScaffoldContractError::UnorderedExperiencePhase);
        }
        let pre_action = self
            .pre_action
            .as_ref()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let decision = self
            .decision
            .as_ref()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        outcome.validate_contract()?;
        validate_same_sequence(self.sequence_id, outcome.sequence_id)?;
        validate_same_creature(pre_action.organism_id, outcome.organism_id)?;
        Tick::validate_monotonic(decision.decision_tick, outcome.outcome_tick)?;
        self.outcome = Some(outcome);
        self.next_phase = ExperiencePatchPhase::Sealed;
        Ok(self)
    }

    pub fn seal(self) -> Result<ExperiencePatch, ScaffoldContractError> {
        let pre_action = self
            .pre_action
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let decision = self
            .decision
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let outcome = self
            .outcome
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let header = ExperiencePatchHeader::for_phase(
            pre_action.organism_id,
            self.sequence_id,
            pre_action.tick,
            ExperiencePatchPhase::Sealed,
        )?;
        let patch = ExperiencePatch {
            header,
            pre_action,
            decision,
            outcome,
        };
        patch.validate_contract()?;
        Ok(patch)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExperiencePatch {
    header: ExperiencePatchHeader,
    pre_action: PreActionSnapshot,
    decision: DecisionSnapshot,
    outcome: PostActionOutcome,
}

impl ExperiencePatch {
    pub const ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION;

    pub const fn header(&self) -> &ExperiencePatchHeader {
        &self.header
    }

    pub const fn pre_action(&self) -> &PreActionSnapshot {
        &self.pre_action
    }

    pub const fn decision(&self) -> &DecisionSnapshot {
        &self.decision
    }

    pub const fn outcome(&self) -> &PostActionOutcome {
        &self.outcome
    }

    pub const fn phase_sequence(&self) -> [ExperiencePatchPhase; 4] {
        [
            ExperiencePatchPhase::PreActionSnapshot,
            ExperiencePatchPhase::DecisionSnapshot,
            ExperiencePatchPhase::PostActionOutcome,
            ExperiencePatchPhase::Sealed,
        ]
    }

    pub const fn as_learning_view(&self) -> ExperiencePatchView<'_> {
        ExperiencePatchView { patch: self }
    }
}

impl Validate for ExperiencePatch {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.header.validate_contract()?;
        if self.header.phase != ExperiencePatchPhase::Sealed {
            return Err(ScaffoldContractError::UnorderedExperiencePhase);
        }
        self.pre_action.validate_contract()?;
        self.decision.validate_contract()?;
        self.outcome.validate_contract()?;
        validate_same_sequence(self.header.sequence_id, self.pre_action.sequence_id)?;
        validate_same_sequence(self.header.sequence_id, self.decision.sequence_id)?;
        validate_same_sequence(self.header.sequence_id, self.outcome.sequence_id)?;
        validate_same_creature(self.header.organism_id, self.pre_action.organism_id)?;
        validate_same_creature(self.header.organism_id, self.decision.organism_id)?;
        validate_same_creature(self.header.organism_id, self.outcome.organism_id)?;
        Tick::validate_monotonic(self.pre_action.tick, self.decision.decision_tick)?;
        Tick::validate_monotonic(self.decision.decision_tick, self.outcome.outcome_tick)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExperiencePatchView<'a> {
    patch: &'a ExperiencePatch,
}

impl<'a> ExperiencePatchView<'a> {
    pub const fn header(self) -> &'a ExperiencePatchHeader {
        &self.patch.header
    }

    pub const fn pre_action(self) -> &'a PreActionSnapshot {
        &self.patch.pre_action
    }

    pub const fn decision(self) -> &'a DecisionSnapshot {
        &self.patch.decision
    }

    pub const fn outcome(self) -> &'a PostActionOutcome {
        &self.patch.outcome
    }
}

fn validate_same_sequence(
    expected: ExperienceSequenceId,
    actual: ExperienceSequenceId,
) -> Result<(), ScaffoldContractError> {
    expected.validate()?;
    actual.validate()?;
    if expected == actual {
        Ok(())
    } else {
        Err(ScaffoldContractError::InvalidId)
    }
}

fn validate_same_creature(
    expected: OrganismId,
    actual: OrganismId,
) -> Result<(), ScaffoldContractError> {
    expected.validate()?;
    actual.validate()?;
    if expected == actual {
        Ok(())
    } else {
        Err(ScaffoldContractError::MismatchedCreatureId)
    }
}

fn validate_action_decision_consistency(
    snapshot: &DecisionSnapshot,
) -> Result<(), ScaffoldContractError> {
    let trace_ref = snapshot
        .selected_action
        .arbitration_trace
        .ok_or(ScaffoldContractError::InvalidActionDecision)?;
    if trace_ref != snapshot.arbitration_trace.trace_ref {
        return Err(ScaffoldContractError::InvalidActionDecision);
    }
    match snapshot.status {
        ActionDecisionStatus::Selected => {
            if snapshot.arbitration_trace.wta_result.selected_action_id
                != Some(snapshot.selected_action.action_id)
            {
                return Err(ScaffoldContractError::InvalidActionDecision);
            }
        }
        ActionDecisionStatus::FallbackSelected => {
            if snapshot
                .arbitration_trace
                .wta_result
                .selected_action_id
                .is_some()
            {
                return Err(ScaffoldContractError::InvalidActionDecision);
            }
        }
    }
    Ok(())
}

fn validate_action_trace(trace: &ActionArbitrationTrace) -> Result<(), ScaffoldContractError> {
    trace.trace_ref.validate()?;
    validate_finite(trace.wta_result.selected_score)?;
    validate_finite(trace.score_threshold)?;
    Confidence::new(trace.confidence_threshold)?;
    for sample in trace
        .inhibition_inputs
        .iter()
        .chain(trace.inhibition_outputs.iter())
    {
        validate_finite(sample.raw_score)?;
        validate_finite(sample.bias_delta)?;
        validate_finite(sample.output_score)?;
        Confidence::new(sample.confidence.raw())?;
    }
    for suppressed in &trace.suppressed_proposals {
        validate_finite(suppressed.proposal_index as f32)?;
    }
    if let Some(action_id) = trace.wta_result.selected_action_id {
        action_id.validate()?;
    }
    Ok(())
}

fn validate_action_proposals(proposals: &[ActionProposal]) -> Result<(), ScaffoldContractError> {
    for proposal in proposals {
        validate_action_proposal(*proposal)?;
    }
    Ok(())
}

fn validate_ranked_proposal(proposal: RankedActionProposal) -> Result<(), ScaffoldContractError> {
    validate_action_proposal(proposal.proposal)?;
    validate_finite(proposal.final_score)?;
    Ok(())
}

fn validate_action_proposal(proposal: ActionProposal) -> Result<(), ScaffoldContractError> {
    proposal.action_id.validate()?;
    validate_finite(proposal.score)?;
    Confidence::new(proposal.confidence.raw())?;
    if let Some(source_lobe) = proposal.source_lobe {
        if source_lobe.raw() == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
    }
    proposal.target.validate()?;
    NormalizedScalar::new(proposal.salience.raw())?;
    crate::Intensity::new(proposal.intensity.raw())?;
    if let Some(score_bias) = proposal.score_bias {
        score_bias.validate()?;
    }
    if let Some(teacher_lesson) = proposal.teacher_lesson {
        teacher_lesson.validate()?;
    }
    if let Some(motor_payload) = proposal.motor_payload {
        motor_payload.validate()?;
    }
    Ok(())
}
