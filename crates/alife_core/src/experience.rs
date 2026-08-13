//! Contract-only causal three-phase ExperiencePatch and policy-evidence records.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    ensure_current_version, validate_finite, validate_optional_target, ActionArbitrationTrace,
    ActionArbitrationTraceRef, ActionCandidate, ActionCommand, ActionDecision,
    ActionDecisionStatus, ActionProposal, ActionWtaResult, BodySnapshot, BrainClassId,
    BrainClassSpec, BrainGenome, BrainScaleTier, CandidateActionFamily, CandidateFeatureDigest,
    CandidateFeatureVector, CandidateObservationRef, CanonicalDigestBuilder, CognitiveContextFrame,
    CognitiveWorkReceipt, ConceptCellId, Confidence, DevelopmentState, DriveDelta,
    EpisodicDecisionKeyV2, ExperienceSequenceId, FinalizedMemoryRecall, GenomeId, HomeostaticDelta,
    HomeostaticSnapshot, LobeLayout, MeasuredChannelObservation, MemoryId, MotorChannel,
    MotorCommandBundle, NeuralActionSelection, NormalizedScalar, OrganismId, PerceptionBaseDigest,
    PerceptionFrame, PerceptionFrameDigest, PhenotypeHash, PolicyBackend, Pose,
    PredictionTargetReceipt, RankedActionProposal, RoutingMatrix, ScaffoldContractError,
    SchemaKind, SchemaVersions, SensorProfile, SensorProfileProvenance, SensoryAbiVersion,
    SensorySnapshot, SignedValence, TeacherPerceptionChannel, Tick, Validate, Vec3f, Velocity,
    WeightSplitContract, WorldEntityId, MAX_ACTION_CANDIDATES,
};

/// The v1.1 semantic spine is a deliberate ABI after the legacy/current
/// experience ABI. The central version registry remains owned by migration
/// work, so this source task keeps the new boundary explicit here.
pub const V11_EXPERIENCE_ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION + 1;

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
    pub sensor_profile: SensorProfileProvenance,
    pub phase: ExperiencePatchPhase,
}

impl ExperiencePatchHeader {
    pub const ABI_VERSION: u16 = SchemaVersions::CURRENT.experience.0;
    pub const V11_ABI_VERSION: u16 = V11_EXPERIENCE_ABI_VERSION;

    pub fn new(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        world_tick: Tick,
        sensor_profile: SensorProfileProvenance,
    ) -> Result<Self, ScaffoldContractError> {
        Self::for_phase(
            organism_id,
            sequence_id,
            world_tick,
            sensor_profile,
            ExperiencePatchPhase::PreActionSnapshot,
        )
    }

    pub fn for_phase(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        world_tick: Tick,
        sensor_profile: SensorProfileProvenance,
        phase: ExperiencePatchPhase,
    ) -> Result<Self, ScaffoldContractError> {
        let header = Self {
            abi_version: Self::ABI_VERSION,
            organism_id,
            sequence_id,
            world_tick,
            sensor_profile,
            phase,
        };
        header.validate_contract()?;
        Ok(header)
    }

    pub fn for_v11_phase(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        world_tick: Tick,
        sensor_profile: SensorProfileProvenance,
        phase: ExperiencePatchPhase,
    ) -> Result<Self, ScaffoldContractError> {
        let header = Self {
            abi_version: Self::V11_ABI_VERSION,
            organism_id,
            sequence_id,
            world_tick,
            sensor_profile,
            phase,
        };
        header.validate_contract()?;
        Ok(header)
    }
}

impl Validate for ExperiencePatchHeader {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_experience_abi(self.abi_version)?;
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        self.sensor_profile.validate_contract()?;
        if self.sensor_profile.source_tick != self.world_tick {
            return Err(ScaffoldContractError::SensorProfileMismatch);
        }
        Ok(())
    }
}

fn validate_experience_abi(actual: u16) -> Result<(), ScaffoldContractError> {
    if actual == ExperiencePatchHeader::ABI_VERSION || actual == V11_EXPERIENCE_ABI_VERSION {
        Ok(())
    } else {
        ensure_current_version(SchemaKind::Experience, actual)
    }
}

fn is_v11_abi(actual: u16) -> bool {
    actual == V11_EXPERIENCE_ABI_VERSION
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cognitive_context: Option<CognitiveContextFrame>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prediction_target: Option<PredictionTargetReceipt>,
}

impl PreActionSnapshot {
    pub const ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION;
    pub const V11_ABI_VERSION: u16 = V11_EXPERIENCE_ABI_VERSION;

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
            cognitive_context: None,
            prediction_target: None,
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
            cognitive_context: None,
            prediction_target: None,
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }

    pub const fn perception(&self) -> &PerceptionFrame {
        &self.perception
    }

    pub fn with_v11_context(
        mut self,
        cognitive_context: CognitiveContextFrame,
        prediction_target: PredictionTargetReceipt,
    ) -> Result<Self, ScaffoldContractError> {
        cognitive_context.validate_contract()?;
        prediction_target.validate_contract()?;
        self.abi_version = V11_EXPERIENCE_ABI_VERSION;
        self.cognitive_context = Some(cognitive_context);
        self.prediction_target = Some(prediction_target);
        self.validate_contract()?;
        Ok(self)
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
        validate_experience_abi(self.abi_version)?;
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
        match (&self.cognitive_context, &self.prediction_target) {
            (Some(context), Some(prediction)) => {
                context.validate_contract()?;
                prediction.validate_contract()?;
                if self.abi_version != V11_EXPERIENCE_ABI_VERSION
                    || context.organism_id != self.organism_id
                    || context.sequence_id != self.sequence_id
                    || context.world_tick != self.tick
                    || prediction.organism_id != self.organism_id
                    || prediction.experience_sequence != self.sequence_id
                    || prediction.world_tick != self.tick
                {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
            (None, None) if self.abi_version == ExperiencePatchHeader::ABI_VERSION => {}
            _ => return Err(ScaffoldContractError::InvalidDecisionEvidence),
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

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DecisionSnapshot {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub decision_tick: Tick,
    pub action_abi_version: u16,
    pub selected_action: ActionCommand,
    pub confidence: Confidence,
    pub evidence: DecisionEvidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    episodic_key: Option<EpisodicDecisionKeyV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_bundle: Option<MotorCommandBundle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prediction_target: Option<PredictionTargetReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cognitive_work: Option<CognitiveWorkReceipt>,
}

impl<'de> Deserialize<'de> for DecisionSnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            abi_version: u16,
            organism_id: OrganismId,
            sequence_id: ExperienceSequenceId,
            decision_tick: Tick,
            action_abi_version: u16,
            selected_action: ActionCommand,
            confidence: Confidence,
            evidence: DecisionEvidence,
            #[serde(default)]
            episodic_key: Option<EpisodicDecisionKeyV2>,
            #[serde(default)]
            selected_bundle: Option<MotorCommandBundle>,
            #[serde(default)]
            prediction_target: Option<PredictionTargetReceipt>,
            #[serde(default)]
            cognitive_work: Option<CognitiveWorkReceipt>,
        }

        let wire = Wire::deserialize(deserializer)?;
        let snapshot = Self {
            abi_version: wire.abi_version,
            organism_id: wire.organism_id,
            sequence_id: wire.sequence_id,
            decision_tick: wire.decision_tick,
            action_abi_version: wire.action_abi_version,
            selected_action: wire.selected_action,
            confidence: wire.confidence,
            evidence: wire.evidence,
            episodic_key: wire.episodic_key,
            selected_bundle: wire.selected_bundle,
            prediction_target: wire.prediction_target,
            cognitive_work: wire.cognitive_work,
        };
        snapshot.validate_contract().map_err(D::Error::custom)?;
        Ok(snapshot)
    }
}

impl DecisionSnapshot {
    pub const ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION;
    pub const V11_ABI_VERSION: u16 = V11_EXPERIENCE_ABI_VERSION;

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
            episodic_key: None,
            selected_bundle: None,
            prediction_target: None,
            cognitive_work: None,
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
                candidate_feature_digest: candidate.feature_digest()?,
                logit: selection.logit,
                confidence: selection.confidence,
            }),
            episodic_key: None,
            selected_bundle: None,
            prediction_target: None,
            cognitive_work: None,
        };
        snapshot.validate_contract()?;
        Ok(snapshot)
    }

    pub fn from_v11_bundle(
        sequence_id: ExperienceSequenceId,
        bundle: MotorCommandBundle,
        prediction_target: PredictionTargetReceipt,
        cognitive_work: CognitiveWorkReceipt,
    ) -> Result<Self, ScaffoldContractError> {
        bundle.validate_contract()?;
        prediction_target.validate_contract()?;
        cognitive_work.validate_contract()?;
        if bundle.sequence_id != sequence_id
            || prediction_target.organism_id != bundle.organism_id
            || prediction_target.experience_sequence != sequence_id
            || prediction_target.world_tick != bundle.tick
            || prediction_target.decision.raw() == 0
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let selected_action = legacy_action_for_bundle(&bundle, prediction_target.decision)?;
        let snapshot = Self {
            abi_version: V11_EXPERIENCE_ABI_VERSION,
            organism_id: bundle.organism_id,
            sequence_id,
            decision_tick: bundle.tick,
            action_abi_version: ActionCommand::ABI_VERSION,
            confidence: bundle
                .channels
                .iter()
                .map(|command| command.confidence.raw())
                .reduce(f32::min)
                .map_or_else(|| Confidence::new(0.0), Confidence::new)?,
            selected_action,
            evidence: DecisionEvidence::HeuristicBaseline(HeuristicDecisionEvidence {
                baseline_schema_version: HeuristicDecisionEvidence::SCHEMA_VERSION,
                proposals: Vec::new(),
                rejected_top_proposal: None,
                ranked_top_proposals: Vec::new(),
                arbitration_trace: v11_compatibility_trace(),
                status: ActionDecisionStatus::FallbackSelected,
            }),
            episodic_key: None,
            selected_bundle: Some(bundle),
            prediction_target: Some(prediction_target),
            cognitive_work: Some(cognitive_work),
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

    pub fn with_finalized_memory_recall(
        mut self,
        frame: &PerceptionFrame,
        recall: &FinalizedMemoryRecall,
        selected_candidate_index: u16,
    ) -> Result<Self, ScaffoldContractError> {
        self.validate_contract()?;
        recall.validate_for_frame(frame)?;
        if self.episodic_key.is_some() {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let evidence = self.neural_evidence()?;
        if selected_candidate_index != evidence.candidate_index {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        let index = usize::from(selected_candidate_index);
        let candidate = frame
            .candidates()
            .get(index)
            .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
        let key = recall
            .candidate_keys()
            .get(index)
            .ok_or(ScaffoldContractError::InvalidMemoryQuery)?;
        key.validate_contract()?;
        key.query().validate_against_frame(frame, candidate)?;
        if candidate.candidate_index != selected_candidate_index
            || key.query().organism_id() != frame.organism_id()
            || key.query().tick() != frame.tick()
            || key.query().profile() != frame.profile_provenance().identity()
            || key.query().action_id() != evidence.action_id
            || key.query().action_kind() != candidate.kind
            || key.query().action_family() != evidence.action_family
            || key.query().candidate_feature_digest() != evidence.candidate_feature_digest
            || key.query().base_frame_digest() != evidence.base_digest
            || key.retrieval_context_digest() != frame.context().canonical_digest()
            || key.final_frame_digest() != evidence.frame_digest
            || recall.base_frame_digest() != evidence.base_digest
            || recall.context_digest() != frame.context().canonical_digest()
            || recall.final_frame_digest() != evidence.frame_digest
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        self.episodic_key = Some(key.clone());
        self.validate_contract()?;
        Ok(self)
    }

    pub const fn episodic_key(&self) -> Option<&EpisodicDecisionKeyV2> {
        self.episodic_key.as_ref()
    }
}

fn v11_compatibility_trace() -> ActionArbitrationTrace {
    ActionArbitrationTrace {
        trace_ref: ActionArbitrationTraceRef(1),
        inhibition_inputs: Vec::new(),
        inhibition_outputs: Vec::new(),
        wta_result: ActionWtaResult {
            selected_proposal_index: None,
            selected_action_id: None,
            selected_score: 0.0,
        },
        score_threshold: 0.0,
        confidence_threshold: 0.0,
        tied_proposal_indices: Vec::new(),
        suppressed_proposals: Vec::new(),
        tie_breaker_seed: 0,
        tie_breaker_index: None,
    }
}

fn legacy_action_for_bundle(
    bundle: &MotorCommandBundle,
    action_id: crate::ActionId,
) -> Result<ActionCommand, ScaffoldContractError> {
    let command = bundle
        .channels
        .iter()
        .find(|command| command.primitive == action_id)
        .or_else(|| bundle.channels.first());
    let (kind, target, intensity, duration_ticks, confidence) = match command {
        Some(command) => {
            let kind = match command.channel {
                MotorChannel::Locomotion => crate::ActionKind::Move,
                MotorChannel::Orientation => crate::ActionKind::Inspect,
                MotorChannel::Manipulation => crate::ActionKind::Interact,
                MotorChannel::Vocal => crate::ActionKind::Vocalize,
                MotorChannel::Posture => crate::ActionKind::Gesture,
                MotorChannel::SpeciesSpecific(_) => crate::ActionKind::Gesture,
            };
            (
                kind,
                command
                    .target
                    .unwrap_or_else(|| crate::ActionTarget::new(None, None)),
                command.intensity,
                command.duration_ticks,
                command.confidence,
            )
        }
        None => (
            crate::ActionKind::Idle,
            crate::ActionTarget::new(None, None),
            crate::Intensity::new(0.0)?,
            crate::DurationTicks::new(1),
            crate::Confidence::new(0.0)?,
        ),
    };
    ActionCommand::structured(
        bundle.organism_id,
        action_id,
        kind,
        target,
        intensity,
        duration_ticks,
        confidence,
        0,
        None,
        None,
        Some(ActionArbitrationTraceRef(1)),
    )
}

impl Validate for DecisionSnapshot {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_experience_abi(self.abi_version)?;
        ensure_current_version(SchemaKind::ActionAbi, self.action_abi_version)?;
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        self.selected_action.validate_contract()?;
        if self.selected_action.organism_id != self.organism_id {
            return Err(ScaffoldContractError::MismatchedCreatureId);
        }
        Confidence::new(self.confidence.raw())?;
        match (
            self.abi_version == V11_EXPERIENCE_ABI_VERSION,
            &self.selected_bundle,
            &self.prediction_target,
            &self.cognitive_work,
        ) {
            (true, Some(bundle), Some(prediction), Some(work)) => {
                bundle.validate_contract()?;
                prediction.validate_contract()?;
                work.validate_contract()?;
                if bundle.organism_id != self.organism_id
                    || bundle.sequence_id != self.sequence_id
                    || bundle.tick != self.decision_tick
                    || prediction.organism_id != self.organism_id
                    || prediction.experience_sequence != self.sequence_id
                    || prediction.world_tick != self.decision_tick
                    || prediction.decision != self.selected_action.action_id
                {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
            (false, None, None, None) => {}
            _ => return Err(ScaffoldContractError::InvalidDecisionEvidence),
        }
        match &self.evidence {
            DecisionEvidence::HeuristicBaseline(evidence) => {
                if self.episodic_key.is_some() {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
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
                if let Some(key) = &self.episodic_key {
                    key.validate_contract()?;
                    let query = key.query();
                    if query.organism_id() != self.organism_id
                        || query.tick() != self.decision_tick
                        || query.candidate_index() != evidence.candidate_index
                        || query.action_id() != evidence.action_id
                        || query.action_kind() != self.selected_action.kind
                        || query.action_family() != evidence.action_family
                        || query.candidate_feature_digest() != evidence.candidate_feature_digest
                        || query.base_frame_digest() != evidence.base_digest
                        || key.final_frame_digest() != evidence.frame_digest
                    {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointPhysicalOutcome {
    pub execution: PhysicalActionOutcome,
    pub channel_observations: Vec<MeasuredChannelObservation>,
}

impl JointPhysicalOutcome {
    pub fn new(
        execution: PhysicalActionOutcome,
        channel_observations: Vec<MeasuredChannelObservation>,
    ) -> Result<Self, ScaffoldContractError> {
        let outcome = Self {
            execution,
            channel_observations,
        };
        outcome.validate_contract()?;
        Ok(outcome)
    }

    pub const fn joint_reward(&self) -> Option<SignedValence> {
        None
    }
}

impl Validate for JointPhysicalOutcome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.execution.validate_contract()?;
        if self.channel_observations.len() > crate::MAX_MOTOR_CHANNELS {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        for observation in &self.channel_observations {
            observation.validate_contract()?;
        }
        let mut channels = self
            .channel_observations
            .iter()
            .map(|observation| format!("{:?}", observation.channel))
            .collect::<Vec<_>>();
        channels.sort();
        if channels.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joint: Option<JointPhysicalOutcome>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cognitive_work: Option<CognitiveWorkReceipt>,
}

impl PostActionOutcome {
    pub const ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION;
    pub const V11_ABI_VERSION: u16 = V11_EXPERIENCE_ABI_VERSION;

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
            joint: None,
            cognitive_work: None,
        };
        outcome.validate_contract()?;
        Ok(outcome)
    }

    pub fn with_v11_joint(
        mut self,
        joint: JointPhysicalOutcome,
        cognitive_work: CognitiveWorkReceipt,
    ) -> Result<Self, ScaffoldContractError> {
        joint.validate_contract()?;
        cognitive_work.validate_contract()?;
        self.abi_version = V11_EXPERIENCE_ABI_VERSION;
        self.physical = joint.execution;
        self.joint = Some(joint);
        self.cognitive_work = Some(cognitive_work);
        self.validate_contract()?;
        Ok(self)
    }
}

impl Validate for PostActionOutcome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_experience_abi(self.abi_version)?;
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
        match (&self.joint, &self.cognitive_work) {
            (Some(joint), Some(work)) => {
                if self.abi_version != V11_EXPERIENCE_ABI_VERSION {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
                joint.validate_contract()?;
                work.validate_contract()?;
                if joint.execution != self.physical {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
            (None, None) if self.abi_version == ExperiencePatchHeader::ABI_VERSION => {}
            _ => return Err(ScaffoldContractError::InvalidDecisionEvidence),
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

    /// Returns the exact pre-action/decision pair only while this transaction
    /// is waiting for its measured world outcome. Checkpoint adapters use this
    /// to bind a pending GPU eligibility receipt to causal builder state.
    pub fn pending_decision(
        &self,
    ) -> Result<(&PreActionSnapshot, &DecisionSnapshot), ScaffoldContractError> {
        if self.next_phase != ExperiencePatchPhase::PostActionOutcome || self.outcome.is_some() {
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
        pre_action.validate_contract()?;
        decision.validate_contract()?;
        validate_same_sequence(self.sequence_id, pre_action.sequence_id)?;
        validate_same_sequence(self.sequence_id, decision.sequence_id)?;
        validate_same_creature(pre_action.organism_id, decision.organism_id)?;
        validate_decision_binding(pre_action, decision)?;
        Ok((pre_action, decision))
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
            pre_action.perception().profile_provenance(),
            ExperiencePatchPhase::Sealed,
        )?;
        let patch = ExperiencePatch {
            header,
            pre_action,
            decision,
            outcome,
            prediction_target: None,
            cognitive_work: None,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    prediction_target: Option<PredictionTargetReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cognitive_work: Option<CognitiveWorkReceipt>,
}

impl ExperiencePatch {
    pub const ABI_VERSION: u16 = ExperiencePatchHeader::ABI_VERSION;
    pub const V11_ABI_VERSION: u16 = V11_EXPERIENCE_ABI_VERSION;

    pub fn new_v11(
        pre_action: PreActionSnapshot,
        bundle: MotorCommandBundle,
        joint: JointPhysicalOutcome,
        prediction_target: PredictionTargetReceipt,
        cognitive_work: CognitiveWorkReceipt,
        cognitive_context: CognitiveContextFrame,
    ) -> Result<Self, ScaffoldContractError> {
        let pre_action =
            pre_action.with_v11_context(cognitive_context, prediction_target.clone())?;
        let decision = DecisionSnapshot::from_v11_bundle(
            pre_action.sequence_id,
            bundle,
            prediction_target.clone(),
            cognitive_work,
        )?;
        let work = decision
            .cognitive_work
            .as_ref()
            .copied()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let outcome = PostActionOutcome::new(
            pre_action.organism_id,
            pre_action.sequence_id,
            Tick::new(pre_action.tick.raw().saturating_add(1)),
            true,
            joint.execution,
            HomeostaticDelta::zero(),
            SignedValence::new(0.0)?,
            NormalizedScalar::new(0.0)?,
            NormalizedScalar::new(0.0)?,
            SignedValence::new(0.0)?,
            NormalizedScalar::new(0.0)?,
        )?
        .with_v11_joint(joint, work)?;
        let patch = Self {
            header: ExperiencePatchHeader::for_v11_phase(
                pre_action.organism_id,
                pre_action.sequence_id,
                pre_action.tick,
                pre_action.perception().profile_provenance(),
                ExperiencePatchPhase::Sealed,
            )?,
            pre_action,
            decision,
            outcome,
            prediction_target: Some(prediction_target),
            cognitive_work: Some(work),
        };
        patch.validate_contract()?;
        Ok(patch)
    }

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

    pub const fn selected_bundle(&self) -> Option<&MotorCommandBundle> {
        self.decision.selected_bundle.as_ref()
    }

    pub const fn prediction_target(&self) -> Option<&PredictionTargetReceipt> {
        self.prediction_target.as_ref()
    }

    pub const fn cognitive_work(&self) -> Option<&CognitiveWorkReceipt> {
        self.cognitive_work.as_ref()
    }

    pub fn causal_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-EXPERIENCE-PATCH");
        builder.write_u16(self.header.abi_version);
        builder.write_u64(self.header.organism_id.raw());
        builder.write_u64(self.header.sequence_id.raw());
        builder.write_u64(self.header.world_tick.raw());
        if let Some(context) = &self.pre_action.cognitive_context {
            for word in context.canonical_digest()? {
                builder.write_u64(word);
            }
        }
        if let Some(bundle) = self.selected_bundle() {
            for word in bundle.canonical_digest()? {
                builder.write_u64(word);
            }
        }
        if let Some(prediction) = self.prediction_target() {
            for word in prediction.canonical_digest()? {
                builder.write_u64(word);
            }
        }
        if let Some(work) = self.cognitive_work() {
            for word in work.canonical_digest()? {
                builder.write_u64(word);
            }
        }
        write_physical_outcome(&mut builder, self.outcome.physical)?;
        Ok(builder.finish256())
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
        if self.header.sensor_profile != self.pre_action.perception().profile_provenance() {
            return Err(ScaffoldContractError::SensorProfileMismatch);
        }
        Tick::validate_monotonic(self.pre_action.tick, self.decision.decision_tick)?;
        Tick::validate_monotonic(self.decision.decision_tick, self.outcome.outcome_tick)?;
        validate_decision_binding(&self.pre_action, &self.decision)?;
        if is_v11_abi(self.header.abi_version) {
            let prediction = self
                .prediction_target
                .as_ref()
                .ok_or(ScaffoldContractError::MissingPhaseData)?;
            let work = self
                .cognitive_work
                .as_ref()
                .ok_or(ScaffoldContractError::MissingPhaseData)?;
            if self.pre_action.abi_version != V11_EXPERIENCE_ABI_VERSION
                || self.decision.abi_version != V11_EXPERIENCE_ABI_VERSION
                || self.outcome.abi_version != V11_EXPERIENCE_ABI_VERSION
                || self.decision.prediction_target.as_ref() != Some(prediction)
                || self.decision.cognitive_work.as_ref() != Some(work)
                || self.pre_action.prediction_target.as_ref() != Some(prediction)
                || self.outcome.cognitive_work.as_ref() != Some(work)
                || self.decision.selected_bundle.is_none()
                || prediction.organism_id != self.header.organism_id
                || prediction.experience_sequence != self.header.sequence_id
                || prediction.world_tick != self.header.world_tick
                || self.outcome.joint.is_none()
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
            let bundle = self
                .decision
                .selected_bundle
                .as_ref()
                .ok_or(ScaffoldContractError::MissingPhaseData)?;
            if bundle.organism_id != self.header.organism_id
                || bundle.sequence_id != self.header.sequence_id
                || bundle.tick != self.header.world_tick
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        } else if self.prediction_target.is_some() || self.cognitive_work.is_some() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[derive(Deserialize)]
struct CurrentExperiencePatchWire {
    header: ExperiencePatchHeader,
    pre_action: PreActionSnapshot,
    decision: DecisionSnapshot,
    outcome: PostActionOutcome,
    #[serde(default)]
    prediction_target: Option<PredictionTargetReceipt>,
    #[serde(default)]
    cognitive_work: Option<CognitiveWorkReceipt>,
}

#[derive(Deserialize)]
struct LegacyExperiencePatchHeader {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    world_tick: Tick,
    phase: ExperiencePatchPhase,
}

#[derive(Deserialize)]
struct LegacyDecisionSnapshotV2 {
    abi_version: u16,
    organism_id: OrganismId,
    sequence_id: ExperienceSequenceId,
    decision_tick: Tick,
    action_abi_version: u16,
    selected_action: ActionCommand,
    confidence: Confidence,
    evidence: DecisionEvidence,
    #[serde(default)]
    episodic_key: Option<EpisodicDecisionKeyV2>,
}

#[derive(Deserialize)]
struct LegacyExperiencePatchV2 {
    header: LegacyExperiencePatchHeader,
    pre_action: PreActionSnapshot,
    decision: LegacyDecisionSnapshotV2,
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
    header: LegacyExperiencePatchHeader,
    pre_action: LegacyPreActionSnapshotV1,
    decision: LegacyDecisionSnapshotV1,
    outcome: PostActionOutcome,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ExperiencePatchWire {
    Current(Box<CurrentExperiencePatchWire>),
    LegacyV2(Box<LegacyExperiencePatchV2>),
    LegacyV1(Box<LegacyExperiencePatchV1>),
}

impl<'de> Deserialize<'de> for ExperiencePatch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match ExperiencePatchWire::deserialize(deserializer)? {
            ExperiencePatchWire::Current(wire) => {
                let patch = Self {
                    header: wire.header,
                    pre_action: wire.pre_action,
                    decision: wire.decision,
                    outcome: wire.outcome,
                    prediction_target: wire.prediction_target,
                    cognitive_work: wire.cognitive_work,
                };
                patch
                    .validate_contract()
                    .map_err(serde::de::Error::custom)?;
                Ok(patch)
            }
            ExperiencePatchWire::LegacyV2(wire) => {
                Self::migrate_unprofiled_v2(*wire).map_err(serde::de::Error::custom)
            }
            ExperiencePatchWire::LegacyV1(wire) => {
                Self::migrate_legacy_baseline_v1(*wire).map_err(serde::de::Error::custom)
            }
        }
    }
}

impl ExperiencePatch {
    fn migrate_unprofiled_v2(
        legacy: LegacyExperiencePatchV2,
    ) -> Result<Self, ScaffoldContractError> {
        if legacy.header.abi_version != 2
            || legacy.pre_action.abi_version != 2
            || legacy.decision.abi_version != 2
            || legacy.outcome.abi_version != 2
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        let sensor_profile = legacy.pre_action.perception().profile_provenance();
        let header = ExperiencePatchHeader::for_phase(
            legacy.header.organism_id,
            legacy.header.sequence_id,
            legacy.header.world_tick,
            sensor_profile,
            legacy.header.phase,
        )?;
        let mut pre_action = legacy.pre_action;
        pre_action.abi_version = ExperiencePatchHeader::ABI_VERSION;
        let decision = DecisionSnapshot {
            abi_version: ExperiencePatchHeader::ABI_VERSION,
            organism_id: legacy.decision.organism_id,
            sequence_id: legacy.decision.sequence_id,
            decision_tick: legacy.decision.decision_tick,
            action_abi_version: legacy.decision.action_abi_version,
            selected_action: legacy.decision.selected_action,
            confidence: legacy.decision.confidence,
            evidence: legacy.decision.evidence,
            episodic_key: legacy.decision.episodic_key,
            selected_bundle: None,
            prediction_target: None,
            cognitive_work: None,
        };
        let mut outcome = legacy.outcome;
        outcome.abi_version = ExperiencePatchHeader::ABI_VERSION;
        let patch = Self {
            header,
            pre_action,
            decision,
            outcome,
            prediction_target: None,
            cognitive_work: None,
        };
        patch.validate_contract()?;
        Ok(patch)
    }

    fn migrate_legacy_baseline_v1(
        legacy: LegacyExperiencePatchV1,
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
            crate::SensorProfileProvenance::new(
                SensorProfile::PrivilegedAffordanceV1,
                crate::SensoryAbiVersion::CURRENT,
                legacy.pre_action.tick,
            )?,
            Vec::new(),
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
            episodic_key: None,
            selected_bundle: None,
            prediction_target: None,
            cognitive_work: None,
        };
        let header = ExperiencePatchHeader::for_phase(
            legacy.header.organism_id,
            legacy.header.sequence_id,
            legacy.header.world_tick,
            pre_action.perception().profile_provenance(),
            legacy.header.phase,
        )?;
        let mut outcome = legacy.outcome;
        outcome.abi_version = PostActionOutcome::ABI_VERSION;
        let patch = Self {
            header,
            pre_action,
            decision,
            outcome,
            prediction_target: None,
            cognitive_work: None,
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
                || candidate.feature_digest()? != evidence.candidate_feature_digest
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
            if let Some(key) = decision.episodic_key() {
                key.validate_contract()?;
                key.query().validate_against_frame(frame, candidate)?;
                if key.query().organism_id() != pre_action.organism_id
                    || key.query().tick() != pre_action.tick
                    || key.query().profile() != frame.profile_provenance().identity()
                    || key.query().candidate_index() != evidence.candidate_index
                    || key.query().action_id() != candidate.action_id
                    || key.query().action_kind() != candidate.kind
                    || key.query().action_family() != candidate.family
                    || key.query().candidate_feature_digest() != candidate.feature_digest()?
                    || key.query().base_frame_digest() != frame.base_digest()
                    || key.retrieval_context_digest() != frame.context().canonical_digest()
                    || key.final_frame_digest() != frame.frame_digest()
                {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
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

fn write_physical_outcome(
    builder: &mut CanonicalDigestBuilder,
    outcome: PhysicalActionOutcome,
) -> Result<(), ScaffoldContractError> {
    builder.write_u8(outcome.contact as u8);
    match outcome.target_entity {
        Some(entity) => {
            builder.write_some();
            builder.write_u64(entity.raw());
        }
        None => builder.write_none(),
    }
    builder.write_f32(outcome.displacement.x)?;
    builder.write_f32(outcome.displacement.y)?;
    builder.write_f32(outcome.displacement.z)?;
    match outcome.collision_normal {
        Some(normal) => {
            builder.write_some();
            builder.write_f32(normal.x)?;
            builder.write_f32(normal.y)?;
            builder.write_f32(normal.z)?;
        }
        None => builder.write_none(),
    }
    builder.write_f32(outcome.energy_cost.raw())
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
