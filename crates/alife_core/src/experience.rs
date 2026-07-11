//! Contract-only causal three-phase ExperiencePatch and policy-evidence records.

use serde::{Deserialize, Serialize};

use crate::{
    ensure_current_version, validate_finite, validate_optional_target, ActionArbitrationTrace,
    ActionCandidate, ActionCommand, ActionDecision, ActionDecisionStatus, ActionProposal,
    BodySnapshot, BrainClassId, BrainClassSpec, BrainGenome, BrainScaleTier, CandidateActionFamily,
    CandidateFeatureDigest, CandidateFeatureVector, CandidateObservationRef, ConceptCellId,
    Confidence, DevelopmentState, DriveDelta, ExperienceSequenceId, GenomeId, HomeostaticDelta,
    HomeostaticSnapshot, LobeLayout, MemoryId, NeuralActionSelection, NormalizedScalar, OrganismId,
    PerceptionBaseDigest, PerceptionFrame, PerceptionFrameDigest, PhenotypeHash, PolicyBackend,
    Pose, RankedActionProposal, RoutingMatrix, ScaffoldContractError, SchemaKind, SchemaVersions,
    SensorProfile, SensoryAbiVersion, SensorySnapshot, SignedValence, TeacherPerceptionChannel,
    Tick, Validate, Vec3f, Velocity, WeightSplitContract, WorldEntityId, MAX_ACTION_CANDIDATES,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceKind {
    NeuralClosedLoopGpu,
    HeuristicBaseline,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeuristicPreActionEvidence {
    pub baseline_schema_version: u16,
    pub brain_class_id: BrainClassId,
    pub brain_scale_tier: BrainScaleTier,
    pub brain_neuron_count: u32,
    pub max_active_synapses: u32,
    pub max_active_microtiles: u32,
    pub routing_schema_version: u16,
    pub lobe_layout: LobeLayout,
    pub routing_matrix: RoutingMatrix,
    pub weight_split: WeightSplitContract,
    pub memory_expectancy: MemoryExpectancySnapshot,
}

impl HeuristicPreActionEvidence {
    pub const SCHEMA_VERSION: u16 = 1;
}

impl Validate for HeuristicPreActionEvidence {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.baseline_schema_version != Self::SCHEMA_VERSION {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        self.brain_class_id.validate()?;
        ensure_current_version(SchemaKind::NeuralProjection, self.routing_schema_version)?;
        self.lobe_layout
            .validate_for_neuron_count(self.brain_neuron_count)?;
        self.routing_matrix.validate_for_layout(&self.lobe_layout)?;
        self.weight_split.validate_contract()?;
        self.memory_expectancy.validate_contract()?;
        if self.max_active_synapses == 0
            || self.max_active_microtiles == 0
            || self.weight_split.genetic_fixed.descriptor.brain_class_id != self.brain_class_id
            || self
                .weight_split
                .lifetime_consolidated
                .descriptor
                .brain_class_id
                != self.brain_class_id
            || self.weight_split.h_operational.descriptor.brain_class_id != self.brain_class_id
            || self.weight_split.h_shadow.descriptor.brain_class_id != self.brain_class_id
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PreActionBrainEvidence {
    NeuralClosedLoopGpu {
        capacity_class_id: BrainClassId,
        phenotype_hash: PhenotypeHash,
        sensor_profile: SensorProfile,
        base_digest: PerceptionBaseDigest,
        frame_digest: PerceptionFrameDigest,
    },
    HeuristicBaseline {
        baseline_schema_version: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreActionSnapshot {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub tick: Tick,
    pub genome_id: GenomeId,
    pub genome_schema_version: u16,
    pub development_state: DevelopmentState,
    pub brain_evidence: PreActionBrainEvidence,
    perception: PerceptionFrame,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    heuristic_evidence: Option<HeuristicPreActionEvidence>,
}

impl PreActionSnapshot {
    pub const ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION;

    #[allow(clippy::too_many_arguments)]
    pub fn from_neural_frame(
        sequence_id: ExperienceSequenceId,
        capacity_class_id: BrainClassId,
        phenotype_hash: PhenotypeHash,
        genome_id: GenomeId,
        genome_schema_version: u16,
        development_state: DevelopmentState,
        perception: PerceptionFrame,
    ) -> Result<Self, ScaffoldContractError> {
        perception.validate_contract()?;
        let snapshot = Self {
            abi_version: Self::ABI_VERSION,
            organism_id: perception.organism_id(),
            sequence_id,
            tick: perception.tick(),
            genome_id,
            genome_schema_version,
            development_state,
            brain_evidence: PreActionBrainEvidence::NeuralClosedLoopGpu {
                capacity_class_id,
                phenotype_hash,
                sensor_profile: perception.sensor_profile(),
                base_digest: perception.base_digest(),
                frame_digest: perception.frame_digest(),
            },
            perception,
            heuristic_evidence: None,
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_heuristic_frame(
        sequence_id: ExperienceSequenceId,
        perception: PerceptionFrame,
        brain_class: BrainClassSpec,
        genome: BrainGenome,
        development_state: DevelopmentState,
        weight_split: WeightSplitContract,
        memory_expectancy: MemoryExpectancySnapshot,
    ) -> Result<Self, ScaffoldContractError> {
        let heuristic_evidence = HeuristicPreActionEvidence {
            baseline_schema_version: HeuristicPreActionEvidence::SCHEMA_VERSION,
            brain_class_id: brain_class.id,
            brain_scale_tier: brain_class.tier,
            brain_neuron_count: brain_class.neuron_count,
            max_active_synapses: brain_class.max_active_synapses,
            max_active_microtiles: brain_class.max_active_microtiles,
            routing_schema_version: brain_class.routing_schema_version,
            lobe_layout: brain_class.lobe_layout,
            routing_matrix: brain_class.routing_matrix,
            weight_split,
            memory_expectancy,
        };
        Self::from_heuristic_components(
            sequence_id,
            perception,
            genome.id,
            genome.schema_version,
            development_state,
            heuristic_evidence,
        )
    }

    fn from_heuristic_components(
        sequence_id: ExperienceSequenceId,
        perception: PerceptionFrame,
        genome_id: GenomeId,
        genome_schema_version: u16,
        development_state: DevelopmentState,
        heuristic_evidence: HeuristicPreActionEvidence,
    ) -> Result<Self, ScaffoldContractError> {
        perception.validate_contract()?;
        heuristic_evidence.validate_contract()?;
        let baseline_schema_version = heuristic_evidence.baseline_schema_version;
        let snapshot = Self {
            abi_version: Self::ABI_VERSION,
            organism_id: perception.organism_id(),
            sequence_id,
            tick: perception.tick(),
            genome_id,
            genome_schema_version,
            development_state,
            brain_evidence: PreActionBrainEvidence::HeuristicBaseline {
                baseline_schema_version,
            },
            perception,
            heuristic_evidence: Some(heuristic_evidence),
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }

    pub const fn perception(&self) -> &PerceptionFrame {
        &self.perception
    }

    pub const fn body(&self) -> BodySnapshot {
        self.perception.body()
    }

    pub const fn homeostasis(&self) -> &HomeostaticSnapshot {
        self.perception.homeostasis()
    }

    pub fn sensory(&self) -> &SensorySnapshot {
        self.perception.sensory()
    }

    pub fn base_digest(&self) -> Result<PerceptionBaseDigest, ScaffoldContractError> {
        self.validate_contract()?;
        Ok(self.perception.base_digest())
    }

    pub fn frame_digest(&self) -> Result<PerceptionFrameDigest, ScaffoldContractError> {
        self.validate_contract()?;
        Ok(self.perception.frame_digest())
    }

    pub const fn evidence_kind(&self) -> EvidenceKind {
        match self.brain_evidence {
            PreActionBrainEvidence::NeuralClosedLoopGpu { .. } => EvidenceKind::NeuralClosedLoopGpu,
            PreActionBrainEvidence::HeuristicBaseline { .. } => EvidenceKind::HeuristicBaseline,
        }
    }

    pub const fn policy_backend(&self) -> PolicyBackend {
        match self.evidence_kind() {
            EvidenceKind::NeuralClosedLoopGpu => PolicyBackend::NeuralClosedLoopGpu,
            EvidenceKind::HeuristicBaseline => PolicyBackend::HeuristicBaseline,
        }
    }

    pub fn heuristic_evidence(&self) -> Result<&HeuristicPreActionEvidence, ScaffoldContractError> {
        if !matches!(
            self.brain_evidence,
            PreActionBrainEvidence::HeuristicBaseline { .. }
        ) {
            return Err(ScaffoldContractError::EvidenceKindMismatch);
        }
        self.heuristic_evidence
            .as_ref()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)
    }

    pub fn brain_class_id(&self) -> Result<BrainClassId, ScaffoldContractError> {
        match self.brain_evidence {
            PreActionBrainEvidence::NeuralClosedLoopGpu {
                capacity_class_id, ..
            } => Ok(capacity_class_id),
            PreActionBrainEvidence::HeuristicBaseline { .. } => {
                Ok(self.heuristic_evidence()?.brain_class_id)
            }
        }
    }
}

impl Validate for PreActionSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::Experience, self.abi_version)?;
        ensure_current_version(SchemaKind::Genome, self.genome_schema_version)?;
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        self.genome_id.validate()?;
        self.development_state.validate_contract()?;
        self.perception.validate_contract()?;
        if self.development_state.genome_id != self.genome_id
            || self.organism_id != self.perception.organism_id()
            || self.tick != self.perception.tick()
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        match self.brain_evidence {
            PreActionBrainEvidence::NeuralClosedLoopGpu {
                capacity_class_id,
                sensor_profile,
                base_digest,
                frame_digest,
                ..
            } => {
                capacity_class_id.validate()?;
                if self.heuristic_evidence.is_some()
                    || sensor_profile != self.perception.sensor_profile()
                    || base_digest != self.perception.base_digest()
                    || frame_digest != self.perception.frame_digest()
                {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
            PreActionBrainEvidence::HeuristicBaseline {
                baseline_schema_version,
            } => {
                let evidence = self.heuristic_evidence()?;
                evidence.validate_contract()?;
                if baseline_schema_version != evidence.baseline_schema_version {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeuristicDecisionEvidence {
    pub baseline_schema_version: u16,
    pub proposals: Vec<ActionProposal>,
    pub rejected_top_proposal: Option<RankedActionProposal>,
    pub ranked_top_proposals: Vec<RankedActionProposal>,
    pub arbitration_trace: ActionArbitrationTrace,
    pub status: ActionDecisionStatus,
}

impl HeuristicDecisionEvidence {
    pub const SCHEMA_VERSION: u16 = 1;
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NeuralDecisionEvidence {
    pub phenotype_hash: PhenotypeHash,
    pub dispatch_generation: u64,
    pub base_digest: PerceptionBaseDigest,
    pub frame_digest: PerceptionFrameDigest,
    pub active_activation_side: u8,
    pub candidate_index: u16,
    pub action_id: crate::ActionId,
    pub action_family: CandidateActionFamily,
    pub candidate_feature_digest: CandidateFeatureDigest,
    pub logit: f32,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)] // The unboxed public shape is the versioned Task 2 ABI.
pub enum DecisionEvidence {
    NeuralClosedLoopGpu(NeuralDecisionEvidence),
    HeuristicBaseline(HeuristicDecisionEvidence),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecisionSnapshot {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub decision_tick: Tick,
    pub action_abi_version: u16,
    pub selected_action: ActionCommand,
    pub confidence: Confidence,
    pub evidence: DecisionEvidence,
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
            confidence: decision.selected.confidence,
            selected_action: decision.selected,
            evidence: DecisionEvidence::HeuristicBaseline(HeuristicDecisionEvidence {
                baseline_schema_version: HeuristicDecisionEvidence::SCHEMA_VERSION,
                proposals,
                rejected_top_proposal: decision.rejected_top_proposal,
                ranked_top_proposals: decision.ranked_top_proposals,
                arbitration_trace: decision.trace,
                status: decision.status,
            }),
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_neural_selection(
        sequence_id: ExperienceSequenceId,
        phenotype_hash: PhenotypeHash,
        dispatch_generation: u64,
        active_activation_side: u8,
        frame: &PerceptionFrame,
        selection: NeuralActionSelection,
        command: ActionCommand,
    ) -> Result<Self, ScaffoldContractError> {
        frame.validate_contract()?;
        selection.validate_contract()?;
        sequence_id.validate()?;
        command.validate_contract()?;
        if dispatch_generation == 0 || active_activation_side > 1 {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let candidate = frame
            .candidates()
            .get(usize::from(selection.candidate_index))
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        if candidate.candidate_index != selection.candidate_index
            || command.organism_id != frame.organism_id()
            || command.action_id != candidate.action_id
            || command.kind != candidate.kind
            || command.target_entity != candidate.target.entity
            || !same_optional_vec3_bits(command.target_position, candidate.target.position)
            || command.intensity.raw() != 1.0
            || command.duration_ticks != candidate.min_duration
            || !same_f32_bits(command.confidence.raw(), selection.confidence.raw())
            || command.source_mask != 0
            || command.teacher_lesson.is_some()
            || command.motor_payload.is_some()
            || command.arbitration_trace.is_some()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let snapshot = Self {
            abi_version: Self::ABI_VERSION,
            organism_id: frame.organism_id(),
            sequence_id,
            decision_tick: frame.tick(),
            action_abi_version: ActionCommand::ABI_VERSION,
            selected_action: command,
            confidence: selection.confidence,
            evidence: DecisionEvidence::NeuralClosedLoopGpu(NeuralDecisionEvidence {
                phenotype_hash,
                dispatch_generation,
                base_digest: frame.base_digest(),
                frame_digest: frame.frame_digest(),
                active_activation_side,
                candidate_index: selection.candidate_index,
                action_id: candidate.action_id,
                action_family: candidate.family,
                candidate_feature_digest: candidate.feature_digest(),
                logit: selection.logit,
                confidence: selection.confidence,
            }),
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }

    pub const fn evidence_kind(&self) -> EvidenceKind {
        match self.evidence {
            DecisionEvidence::NeuralClosedLoopGpu(_) => EvidenceKind::NeuralClosedLoopGpu,
            DecisionEvidence::HeuristicBaseline(_) => EvidenceKind::HeuristicBaseline,
        }
    }

    pub const fn policy_backend(&self) -> PolicyBackend {
        match self.evidence_kind() {
            EvidenceKind::NeuralClosedLoopGpu => PolicyBackend::NeuralClosedLoopGpu,
            EvidenceKind::HeuristicBaseline => PolicyBackend::HeuristicBaseline,
        }
    }

    pub fn neural_evidence(&self) -> Result<&NeuralDecisionEvidence, ScaffoldContractError> {
        match &self.evidence {
            DecisionEvidence::NeuralClosedLoopGpu(evidence) => Ok(evidence),
            DecisionEvidence::HeuristicBaseline(_) => {
                Err(ScaffoldContractError::EvidenceKindMismatch)
            }
        }
    }

    pub fn heuristic_evidence(&self) -> Result<&HeuristicDecisionEvidence, ScaffoldContractError> {
        match &self.evidence {
            DecisionEvidence::HeuristicBaseline(evidence) => Ok(evidence),
            DecisionEvidence::NeuralClosedLoopGpu(_) => {
                Err(ScaffoldContractError::EvidenceKindMismatch)
            }
        }
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
        match &self.evidence {
            DecisionEvidence::HeuristicBaseline(evidence) => {
                if evidence.baseline_schema_version != HeuristicDecisionEvidence::SCHEMA_VERSION {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
                validate_action_trace(&evidence.arbitration_trace)?;
                validate_action_decision_consistency(self, evidence)?;
                validate_action_proposals(&evidence.proposals)?;
                if let Some(proposal) = evidence.rejected_top_proposal {
                    validate_ranked_proposal(proposal)?;
                }
                for proposal in &evidence.ranked_top_proposals {
                    validate_ranked_proposal(*proposal)?;
                }
            }
            DecisionEvidence::NeuralClosedLoopGpu(evidence) => {
                evidence.action_id.validate()?;
                Confidence::new(evidence.confidence.raw())?;
                if evidence.dispatch_generation == 0
                    || evidence.active_activation_side > 1
                    || !evidence.logit.is_finite()
                    || evidence.action_id != self.selected_action.action_id
                    || !evidence
                        .action_family
                        .is_compatible_with(self.selected_action.kind)
                    || !same_f32_bits(evidence.confidence.raw(), self.confidence.raw())
                    || !same_f32_bits(
                        evidence.confidence.raw(),
                        self.selected_action.confidence.raw(),
                    )
                {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
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
        validate_decision_binding(pre_action, &decision)?;
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

#[derive(Debug, Clone, PartialEq, Serialize)]
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
        if self.header.world_tick != self.pre_action.tick {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Tick::validate_monotonic(self.pre_action.tick, self.decision.decision_tick)?;
        Tick::validate_monotonic(self.decision.decision_tick, self.outcome.outcome_tick)?;
        validate_decision_binding(&self.pre_action, &self.decision)?;
        Ok(())
    }
}

#[derive(Deserialize)]
struct CurrentExperiencePatchWire {
    header: ExperiencePatchHeader,
    pre_action: PreActionSnapshot,
    decision: DecisionSnapshot,
    outcome: PostActionOutcome,
}

#[derive(Deserialize)]
struct LegacyPreActionSnapshotV1 {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    tick: Tick,
    brain_class_id: BrainClassId,
    brain_scale_tier: BrainScaleTier,
    brain_neuron_count: u32,
    max_active_synapses: u32,
    max_active_microtiles: u32,
    routing_schema_version: u16,
    lobe_layout: LobeLayout,
    routing_matrix: RoutingMatrix,
    genome_id: GenomeId,
    genome_schema_version: u16,
    development_state: DevelopmentState,
    weight_split: WeightSplitContract,
    sensory_abi_version: SensoryAbiVersion,
    chemistry_schema_version: u16,
    body_pose: Pose,
    body_velocity: Velocity,
    homeostasis: HomeostaticSnapshot,
    sensory: SensorySnapshot,
    memory_expectancy: MemoryExpectancySnapshot,
}

#[derive(Deserialize)]
struct LegacyDecisionSnapshotV1 {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    decision_tick: Tick,
    action_abi_version: u16,
    proposals: Vec<ActionProposal>,
    selected_action: ActionCommand,
    rejected_top_proposal: Option<RankedActionProposal>,
    ranked_top_proposals: Vec<RankedActionProposal>,
    arbitration_trace: ActionArbitrationTrace,
    confidence: Confidence,
    status: ActionDecisionStatus,
}

#[derive(Deserialize)]
struct LegacyExperiencePatchV1 {
    header: ExperiencePatchHeader,
    pre_action: LegacyPreActionSnapshotV1,
    decision: LegacyDecisionSnapshotV1,
    outcome: PostActionOutcome,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ExperiencePatchWire {
    Current(Box<CurrentExperiencePatchWire>),
    LegacyV1(Box<LegacyExperiencePatchV1>),
}

impl<'de> Deserialize<'de> for ExperiencePatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match ExperiencePatchWire::deserialize(deserializer)? {
            ExperiencePatchWire::Current(wire) => Ok(Self {
                header: wire.header,
                pre_action: wire.pre_action,
                decision: wire.decision,
                outcome: wire.outcome,
            }),
            ExperiencePatchWire::LegacyV1(wire) => {
                Self::migrate_legacy_baseline_v1(*wire).map_err(serde::de::Error::custom)
            }
        }
    }
}

impl ExperiencePatch {
    fn migrate_legacy_baseline_v1(
        mut legacy: LegacyExperiencePatchV1,
    ) -> Result<Self, ScaffoldContractError> {
        if legacy.header.abi_version != 1
            || legacy.pre_action.abi_version != 1
            || legacy.decision.abi_version != 1
            || legacy.outcome.abi_version != 1
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        ensure_current_version(
            SchemaKind::SensoryAbi,
            legacy.pre_action.sensory_abi_version.raw(),
        )?;
        ensure_current_version(
            SchemaKind::Chemistry,
            legacy.pre_action.chemistry_schema_version,
        )?;
        ensure_current_version(SchemaKind::ActionAbi, legacy.decision.action_abi_version)?;

        let candidates = legacy_candidates(&legacy.decision)?;
        let perception = PerceptionFrame::new(
            legacy.pre_action.organism_id,
            legacy.pre_action.tick,
            SensorProfile::PrivilegedAffordanceV1,
            legacy.pre_action.sensory,
            BodySnapshot {
                pose: legacy.pre_action.body_pose,
                velocity: legacy.pre_action.body_velocity,
            },
            legacy.pre_action.homeostasis,
            candidates,
        )?;
        let heuristic_pre_action = HeuristicPreActionEvidence {
            baseline_schema_version: HeuristicPreActionEvidence::SCHEMA_VERSION,
            brain_class_id: legacy.pre_action.brain_class_id,
            brain_scale_tier: legacy.pre_action.brain_scale_tier,
            brain_neuron_count: legacy.pre_action.brain_neuron_count,
            max_active_synapses: legacy.pre_action.max_active_synapses,
            max_active_microtiles: legacy.pre_action.max_active_microtiles,
            routing_schema_version: legacy.pre_action.routing_schema_version,
            lobe_layout: legacy.pre_action.lobe_layout,
            routing_matrix: legacy.pre_action.routing_matrix,
            weight_split: legacy.pre_action.weight_split,
            memory_expectancy: legacy.pre_action.memory_expectancy,
        };
        let pre_action = PreActionSnapshot::from_heuristic_components(
            legacy.pre_action.sequence_id,
            perception,
            legacy.pre_action.genome_id,
            legacy.pre_action.genome_schema_version,
            legacy.pre_action.development_state,
            heuristic_pre_action,
        )?;
        let decision = DecisionSnapshot {
            abi_version: DecisionSnapshot::ABI_VERSION,
            organism_id: legacy.decision.organism_id,
            sequence_id: legacy.decision.sequence_id,
            decision_tick: legacy.decision.decision_tick,
            action_abi_version: legacy.decision.action_abi_version,
            selected_action: legacy.decision.selected_action,
            confidence: legacy.decision.confidence,
            evidence: DecisionEvidence::HeuristicBaseline(HeuristicDecisionEvidence {
                baseline_schema_version: HeuristicDecisionEvidence::SCHEMA_VERSION,
                proposals: legacy.decision.proposals,
                rejected_top_proposal: legacy.decision.rejected_top_proposal,
                ranked_top_proposals: legacy.decision.ranked_top_proposals,
                arbitration_trace: legacy.decision.arbitration_trace,
                status: legacy.decision.status,
            }),
        };
        legacy.header.abi_version = ExperiencePatchHeader::ABI_VERSION;
        legacy.outcome.abi_version = PostActionOutcome::ABI_VERSION;
        let patch = Self {
            header: legacy.header,
            pre_action,
            decision,
            outcome: legacy.outcome,
        };
        patch.validate_contract()?;
        Ok(patch)
    }
}

fn legacy_candidates(
    legacy: &LegacyDecisionSnapshotV1,
) -> Result<Vec<ActionCandidate>, ScaffoldContractError> {
    let selected_proposal_index = legacy_selected_proposal_index(legacy);
    let proposal_limit =
        if selected_proposal_index.is_some_and(|index| index < MAX_ACTION_CANDIDATES) {
            MAX_ACTION_CANDIDATES
        } else {
            MAX_ACTION_CANDIDATES.saturating_sub(1)
        };
    let retained_count = legacy.proposals.len().min(proposal_limit);
    let mut retained_indices = (0..retained_count).collect::<Vec<_>>();

    match selected_proposal_index {
        Some(index) if index >= MAX_ACTION_CANDIDATES => retained_indices.push(index),
        Some(_) => {}
        None => retained_indices.push(usize::MAX),
    }

    retained_indices
        .into_iter()
        .enumerate()
        .map(|(candidate_index, proposal_index)| {
            let candidate_index = u16::try_from(candidate_index)
                .map_err(|_| ScaffoldContractError::InvalidActionCandidate)?;
            match legacy.proposals.get(proposal_index) {
                Some(proposal) => legacy_candidate_from_proposal(
                    candidate_index,
                    *proposal,
                    (Some(proposal_index) == selected_proposal_index)
                        .then_some(legacy.selected_action.duration_ticks),
                ),
                None => legacy_candidate_from_command(candidate_index, legacy.selected_action),
            }
        })
        .collect()
}

fn legacy_selected_proposal_index(legacy: &LegacyDecisionSnapshotV1) -> Option<usize> {
    if legacy.status != ActionDecisionStatus::Selected {
        return None;
    }
    let index = legacy
        .arbitration_trace
        .wta_result
        .selected_proposal_index?;
    legacy
        .proposals
        .get(index)
        .filter(|proposal| legacy_proposal_matches_command(proposal, &legacy.selected_action))
        .map(|_| index)
}

fn legacy_proposal_matches_command(proposal: &ActionProposal, command: &ActionCommand) -> bool {
    proposal.action_id == command.action_id
        && proposal.kind == command.kind
        && proposal.target.entity == command.target_entity
        && same_optional_vec3_bits(proposal.target.position, command.target_position)
        && same_f32_bits(proposal.intensity.raw(), command.intensity.raw())
        && same_f32_bits(proposal.confidence.raw(), command.confidence.raw())
        && proposal.source_mask == command.source_mask
        && proposal.teacher_lesson == command.teacher_lesson
        && proposal.motor_payload == command.motor_payload
}

fn legacy_candidate_from_proposal(
    candidate_index: u16,
    proposal: ActionProposal,
    selected_duration: Option<crate::DurationTicks>,
) -> Result<ActionCandidate, ScaffoldContractError> {
    let duration = selected_duration.unwrap_or_else(|| crate::DurationTicks::new(1));
    ActionCandidate::new(
        candidate_index,
        proposal.action_id,
        proposal.kind,
        CandidateActionFamily::baseline_for_kind(proposal.kind),
        CandidateObservationRef::None,
        proposal.target,
        CandidateFeatureVector::zero(),
        proposal.confidence,
        NormalizedScalar::new(0.0)?,
        duration,
        duration,
    )
}

fn legacy_candidate_from_command(
    candidate_index: u16,
    command: ActionCommand,
) -> Result<ActionCandidate, ScaffoldContractError> {
    ActionCandidate::new(
        candidate_index,
        command.action_id,
        command.kind,
        CandidateActionFamily::baseline_for_kind(command.kind),
        CandidateObservationRef::None,
        crate::ActionTarget::new(command.target_entity, command.target_position),
        CandidateFeatureVector::zero(),
        command.confidence,
        NormalizedScalar::new(0.0)?,
        command.duration_ticks,
        command.duration_ticks,
    )
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

fn validate_decision_binding(
    pre_action: &PreActionSnapshot,
    decision: &DecisionSnapshot,
) -> Result<(), ScaffoldContractError> {
    if pre_action.policy_backend() != decision.policy_backend() {
        return Err(ScaffoldContractError::InvalidDecisionEvidence);
    }
    match (&pre_action.brain_evidence, &decision.evidence) {
        (
            PreActionBrainEvidence::NeuralClosedLoopGpu {
                phenotype_hash,
                base_digest,
                frame_digest,
                ..
            },
            DecisionEvidence::NeuralClosedLoopGpu(evidence),
        ) => {
            let frame = pre_action.perception();
            let candidate = frame
                .candidates()
                .get(usize::from(evidence.candidate_index))
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            if pre_action.tick != decision.decision_tick
                || phenotype_hash != &evidence.phenotype_hash
                || base_digest != &evidence.base_digest
                || frame_digest != &evidence.frame_digest
                || *base_digest != frame.base_digest()
                || *frame_digest != frame.frame_digest()
                || candidate.candidate_index != evidence.candidate_index
                || candidate.action_id != evidence.action_id
                || candidate.family != evidence.action_family
                || candidate.feature_digest() != evidence.candidate_feature_digest
                || candidate.action_id != decision.selected_action.action_id
                || candidate.kind != decision.selected_action.kind
                || candidate.target.entity != decision.selected_action.target_entity
                || !same_optional_vec3_bits(
                    candidate.target.position,
                    decision.selected_action.target_position,
                )
                || decision.selected_action.intensity.raw() != 1.0
                || decision.selected_action.duration_ticks != candidate.min_duration
                || decision.selected_action.source_mask != 0
                || decision.selected_action.teacher_lesson.is_some()
                || decision.selected_action.motor_payload.is_some()
                || decision.selected_action.arbitration_trace.is_some()
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        }
        (
            PreActionBrainEvidence::HeuristicBaseline { .. },
            DecisionEvidence::HeuristicBaseline(_),
        ) => {
            pre_action.heuristic_evidence()?.validate_contract()?;
            decision.heuristic_evidence()?;
        }
        _ => return Err(ScaffoldContractError::EvidenceKindMismatch),
    }
    Ok(())
}

fn validate_action_decision_consistency(
    snapshot: &DecisionSnapshot,
    evidence: &HeuristicDecisionEvidence,
) -> Result<(), ScaffoldContractError> {
    let trace_ref = snapshot
        .selected_action
        .arbitration_trace
        .ok_or(ScaffoldContractError::InvalidActionDecision)?;
    if trace_ref != evidence.arbitration_trace.trace_ref {
        return Err(ScaffoldContractError::InvalidActionDecision);
    }
    match evidence.status {
        ActionDecisionStatus::Selected => {
            if evidence.arbitration_trace.wta_result.selected_action_id
                != Some(snapshot.selected_action.action_id)
            {
                return Err(ScaffoldContractError::InvalidActionDecision);
            }
        }
        ActionDecisionStatus::FallbackSelected => {
            if evidence
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

fn same_f32_bits(left: f32, right: f32) -> bool {
    left.to_bits() == right.to_bits()
}

fn same_optional_vec3_bits(left: Option<Vec3f>, right: Option<Vec3f>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            same_f32_bits(left.x, right.x)
                && same_f32_bits(left.y, right.y)
                && same_f32_bits(left.z, right.z)
        }
        _ => false,
    }
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
