//! Bounded cognitive context assembled before a v1.1 decision.

use serde::{Deserialize, Serialize};

use crate::{
    AttentionFrame, BiochemistryState, CandidateActionFamily,
    CanonicalDigestBuilder, ConceptCellId,
    ExperienceSequenceId, GroundedFocalDetail, HysteresisState, MemoryId, NormalizedScalar,
    OrganismId, SalienceComponents, ScaffoldContractError, SemanticStateVector, SignedValence,
    StableFocusIdentity, Tick, UnresolvedGapId, Validate, MAX_COGNITIVE_WORK_COUNTER,
    MAX_FOCAL_FEATURE_WIDTH, MAX_FOCAL_TARGETS, MAX_PERIPHERAL_SUMMARIES,
    MAX_SEMANTIC_STATE_VALUES,
};

pub const COGNITIVE_CONTEXT_SCHEMA_VERSION: u16 = 2;
pub const MAX_CONTEXT_MEMORY_EXPECTANCIES: usize = 32;
pub const MAX_ACTIVE_CONCEPTS: usize = 32;
pub const MAX_ACTIVE_GAPS: usize = 32;
pub const MAX_PREDICTION_ERROR_FEATURES: usize = MAX_SEMANTIC_STATE_VALUES;
pub const MAX_TOPOLOGY_TARGET_CONTEXTS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CognitiveTargetContext {
    pub target: Option<StableFocusIdentity>,
    pub action_family: CandidateActionFamily,
    pub concept_id: ConceptCellId,
    pub concept_signal: NormalizedScalar,
    pub gap_id: Option<UnresolvedGapId>,
    pub gap_signal: NormalizedScalar,
    pub causal_signal: NormalizedScalar,
    pub contradiction_signal: NormalizedScalar,
    pub uncertainty: NormalizedScalar,
}

impl Validate for CognitiveTargetContext {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if let Some(target) = self.target {
            validate_focus_identity(target)?;
        }
        CandidateActionFamily::try_from_raw(self.action_family.raw())?;
        self.concept_id.validate()?;
        if let Some(gap_id) = self.gap_id {
            gap_id.validate()?;
        }
        for value in [
            self.concept_signal,
            self.gap_signal,
            self.causal_signal,
            self.contradiction_signal,
            self.uncertainty,
        ] {
            NormalizedScalar::new(value.raw())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CognitivePeripheralView {
    pub summaries: Vec<crate::PeripheralSummary>,
}

impl Validate for CognitivePeripheralView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.summaries.len() > MAX_PERIPHERAL_SUMMARIES {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for summary in &self.summaries {
            summary.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CognitiveFocalView {
    pub identities: Vec<StableFocusIdentity>,
    pub salience: Vec<SalienceComponents>,
    #[serde(default)]
    pub grounded_details: Vec<GroundedFocalDetail>,
    pub hysteresis: HysteresisState,
}

impl Validate for CognitiveFocalView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.identities.len() > MAX_FOCAL_TARGETS
            || self.salience.len() > MAX_FOCAL_TARGETS
            || self.grounded_details.len() > MAX_FOCAL_TARGETS
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for identity in &self.identities {
            validate_focus_identity(*identity)?;
        }
        if self.identities.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for salience in &self.salience {
            salience.validate_contract()?;
        }
        for (index, detail) in self.grounded_details.iter().enumerate() {
            detail.validate_contract()?;
            if !self.identities.contains(&detail.identity)
                || self
                    .grounded_details
                    .iter()
                    .take(index)
                    .any(|candidate| candidate.identity == detail.identity)
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence);
            }
        }
        self.hysteresis.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CognitiveInteroceptiveView {
    pub hunger: NormalizedScalar,
    pub fatigue: NormalizedScalar,
    pub pain: NormalizedScalar,
    pub injury: NormalizedScalar,
    pub temperature_stress: NormalizedScalar,
    pub sleep_pressure: NormalizedScalar,
    pub energy: NormalizedScalar,
    pub brain_atp: NormalizedScalar,
}

impl Default for CognitiveInteroceptiveView {
    fn default() -> Self {
        Self {
            hunger: NormalizedScalar(0.0),
            fatigue: NormalizedScalar(0.0),
            pain: NormalizedScalar(0.0),
            injury: NormalizedScalar(0.0),
            temperature_stress: NormalizedScalar(0.0),
            sleep_pressure: NormalizedScalar(0.0),
            energy: NormalizedScalar(1.0),
            brain_atp: NormalizedScalar(1.0),
        }
    }
}

impl CognitiveInteroceptiveView {
    pub fn from_biochemistry(
        state: &BiochemistryState,
    ) -> Result<Self, ScaffoldContractError> {
        state.validate_contract()?;
        let view = Self {
            hunger: NormalizedScalar::new(state.homeostasis.drives.hunger)?,
            fatigue: NormalizedScalar::new(state.homeostasis.drives.fatigue)?,
            pain: NormalizedScalar::new(state.homeostasis.drives.pain)?,
            injury: NormalizedScalar::new(state.body.injury)?,
            temperature_stress: NormalizedScalar::new(
                state
                    .homeostasis
                    .drives
                    .temperature_stress
                    .max(state.body.temperature_stress),
            )?,
            sleep_pressure: NormalizedScalar::new(state.homeostasis.hormones.sleep_pressure)?,
            energy: NormalizedScalar::new(state.body.energy)?,
            brain_atp: NormalizedScalar::new(state.homeostasis.drives.brain_atp)?,
        };
        view.validate_contract()?;
        Ok(view)
    }
}

impl Validate for CognitiveInteroceptiveView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        for value in [
            self.hunger,
            self.fatigue,
            self.pain,
            self.injury,
            self.temperature_stress,
            self.sleep_pressure,
            self.energy,
            self.brain_atp,
        ] {
            NormalizedScalar::new(value.raw())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CognitiveMemoryExpectancy {
    pub memory_id: MemoryId,
    pub expected_valence: SignedValence,
    pub confidence: NormalizedScalar,
}

impl Validate for CognitiveMemoryExpectancy {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.memory_id.validate()?;
        SignedValence::new(self.expected_valence.raw())?;
        NormalizedScalar::new(self.confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CognitiveMemoryView {
    pub expectancies: Vec<CognitiveMemoryExpectancy>,
}

impl Validate for CognitiveMemoryView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.expectancies.len() > MAX_CONTEXT_MEMORY_EXPECTANCIES {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for expectancy in &self.expectancies {
            expectancy.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CognitiveConceptActivation {
    pub concept_id: ConceptCellId,
    pub activation: NormalizedScalar,
    pub utility: NormalizedScalar,
}

impl Validate for CognitiveConceptActivation {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.concept_id.validate()?;
        NormalizedScalar::new(self.activation.raw())?;
        NormalizedScalar::new(self.utility.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CognitiveConceptView {
    pub active_concepts: Vec<CognitiveConceptActivation>,
    #[serde(default)]
    pub target_contexts: Vec<CognitiveTargetContext>,
    pub topology_digest: [u64; 4],
}

impl Validate for CognitiveConceptView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.active_concepts.len() > MAX_ACTIVE_CONCEPTS
            || self.target_contexts.len() > MAX_TOPOLOGY_TARGET_CONTEXTS
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for concept in &self.active_concepts {
            concept.validate_contract()?;
        }
        for context in &self.target_contexts {
            context.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CognitiveGapActivation {
    pub gap_id: UnresolvedGapId,
    pub voltage: NormalizedScalar,
    pub uncertainty: NormalizedScalar,
}

impl Validate for CognitiveGapActivation {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.gap_id.validate()?;
        NormalizedScalar::new(self.voltage.raw())?;
        NormalizedScalar::new(self.uncertainty.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveGapView {
    pub active_gaps: Vec<CognitiveGapActivation>,
    pub gap_voltage: NormalizedScalar,
}

impl Default for CognitiveGapView {
    fn default() -> Self {
        Self {
            active_gaps: Vec::new(),
            gap_voltage: NormalizedScalar(0.0),
        }
    }
}

impl Validate for CognitiveGapView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.active_gaps.len() > MAX_ACTIVE_GAPS {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        NormalizedScalar::new(self.gap_voltage.raw())?;
        for gap in &self.active_gaps {
            gap.validate_contract()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitivePredictionView {
    pub source_digest: [u64; 4],
    pub semantic_state_abi: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_state: Option<SemanticStateVector>,
    pub prediction_error: Vec<NormalizedScalar>,
    pub action_sensitivity: NormalizedScalar,
}

impl Default for CognitivePredictionView {
    fn default() -> Self {
        Self {
            source_digest: [0; 4],
            semantic_state_abi: 0,
            source_state: None,
            prediction_error: Vec::new(),
            action_sensitivity: NormalizedScalar(0.0),
        }
    }
}

impl Validate for CognitivePredictionView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.prediction_error.len() > MAX_PREDICTION_ERROR_FEATURES {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        match (&self.source_state, self.semantic_state_abi) {
            (None, 0) if self.prediction_error.is_empty() && self.source_digest == [0; 4] => {}
            (Some(state), abi) => {
                state.validate_contract()?;
                if abi != state.abi_version || self.source_digest == [0; 4] {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
            }
            _ => return Err(ScaffoldContractError::InvalidDecisionEvidence),
        }
        NormalizedScalar::new(self.action_sensitivity.raw())?;
        for value in &self.prediction_error {
            NormalizedScalar::new(value.raw())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveBudgetView {
    pub peripheral_capacity: u16,
    pub focal_capacity: u8,
    #[serde(default)]
    pub focal_feature_width: u16,
    pub work_limit: u64,
    pub work_used: u64,
    #[serde(default)]
    pub peripheral_work_units: u64,
    #[serde(default)]
    pub focal_work_units: u64,
}

impl Default for CognitiveBudgetView {
    fn default() -> Self {
        Self {
            peripheral_capacity: MAX_PERIPHERAL_SUMMARIES as u16,
            focal_capacity: MAX_FOCAL_TARGETS as u8,
            focal_feature_width: 0,
            work_limit: 0,
            work_used: 0,
            peripheral_work_units: 0,
            focal_work_units: 0,
        }
    }
}

impl Validate for CognitiveBudgetView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if usize::from(self.peripheral_capacity) > MAX_PERIPHERAL_SUMMARIES
            || usize::from(self.focal_capacity) > MAX_FOCAL_TARGETS
            || self.focal_feature_width > MAX_FOCAL_FEATURE_WIDTH
            || self.work_used > self.work_limit
            || self.peripheral_work_units > MAX_COGNITIVE_WORK_COUNTER
            || self.focal_work_units > MAX_COGNITIVE_WORK_COUNTER
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveContextFrame {
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub world_tick: Tick,
    pub attention: AttentionFrame,
    pub peripheral: CognitivePeripheralView,
    pub focal: CognitiveFocalView,
    pub interoceptive: CognitiveInteroceptiveView,
    pub memory: CognitiveMemoryView,
    pub concept: CognitiveConceptView,
    pub gap: CognitiveGapView,
    pub prediction: CognitivePredictionView,
    pub budget: CognitiveBudgetView,
}

impl CognitiveContextFrame {
    pub fn empty(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        world_tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        let attention = AttentionFrame::empty(organism_id, sequence_id, world_tick)?;
        let frame = Self {
            schema_version: COGNITIVE_CONTEXT_SCHEMA_VERSION,
            organism_id,
            sequence_id,
            world_tick,
            attention,
            peripheral: CognitivePeripheralView::default(),
            focal: CognitiveFocalView::default(),
            interoceptive: CognitiveInteroceptiveView::default(),
            memory: CognitiveMemoryView::default(),
            concept: CognitiveConceptView::default(),
            gap: CognitiveGapView::default(),
            prediction: CognitivePredictionView::default(),
            budget: CognitiveBudgetView::default(),
        };
        frame.validate_contract()?;
        Ok(frame)
    }

    pub fn apply_topology_context(
        &mut self,
        contribution: &crate::topology::TopologyContextContribution,
    ) -> Result<(), ScaffoldContractError> {
        contribution.validate_contract()?;
        self.concept = CognitiveConceptView {
            active_concepts: contribution.active_concepts.clone(),
            target_contexts: contribution.target_contexts.clone(),
            topology_digest: contribution.topology_digest,
        };
        self.gap = CognitiveGapView {
            active_gaps: contribution.active_gaps.clone(),
            gap_voltage: contribution.gap_voltage,
        };
        self.validate_contract()
    }

    pub fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        self.validate_contract()?;
        let mut builder = CanonicalDigestBuilder::new(b"ALIFE-V11-COGNITIVE-CONTEXT");
        builder.write_u16(self.schema_version);
        builder.write_u64(self.organism_id.raw());
        builder.write_u64(self.sequence_id.raw());
        builder.write_u64(self.world_tick.raw());
        for word in self.attention.canonical_digest()? {
            builder.write_u64(word);
        }
        builder.write_sequence_len(self.peripheral.summaries.len());
        for summary in &self.peripheral.summaries {
            let summary_digest = {
                let mut nested = CanonicalDigestBuilder::new(b"ALIFE-V11-PERIPHERAL-SUMMARY");
                nested.write_u8(summary.identity.kind_tag());
                nested.write_u64(summary.identity.raw());
                nested.write_f32(summary.confidence.raw())?;
                for value in [
                    summary.salience.peripheral_intensity,
                    summary.salience.drive,
                    summary.salience.memory_expectancy,
                    summary.salience.concept,
                    summary.salience.novelty,
                    summary.salience.uncertainty,
                    summary.salience.gap_voltage,
                ] {
                    nested.write_f32(value.raw())?;
                }
                nested.finish256()
            };
            for word in summary_digest {
                builder.write_u64(word);
            }
        }
        builder.write_sequence_len(self.focal.identities.len());
        for identity in &self.focal.identities {
            builder.write_u8(identity.kind_tag());
            builder.write_u64(identity.raw());
        }
        builder.write_sequence_len(self.focal.salience.len());
        for salience in &self.focal.salience {
            write_salience(&mut builder, *salience)?;
        }
        builder.write_u16(self.focal.hysteresis.retained_ticks);
        builder.write_f32(self.focal.hysteresis.switch_margin.raw())?;
        builder.write_sequence_len(self.focal.grounded_details.len());
        for detail in &self.focal.grounded_details {
            builder.write_u8(detail.identity.kind_tag());
            builder.write_u64(detail.identity.raw());
            builder.write_u64(detail.transport_entity.raw());
            for value in [
                detail.relative_position.x,
                detail.relative_position.y,
                detail.relative_position.z,
                detail.velocity.x,
                detail.velocity.y,
                detail.velocity.z,
            ] {
                builder.write_f32(value)?;
            }
            builder.write_u16(detail.feature_width);
            for value in detail
                .feature_values()
                .into_iter()
                .take(usize::from(detail.feature_width))
            {
                builder.write_f32(value)?;
            }
            builder.write_f32(detail.confidence.raw())?;
        }
        write_interoceptive(&mut builder, self.interoceptive)?;
        builder.write_sequence_len(self.memory.expectancies.len());
        for entry in &self.memory.expectancies {
            builder.write_u64(entry.memory_id.raw());
            builder.write_f32(entry.expected_valence.raw())?;
            builder.write_f32(entry.confidence.raw())?;
        }
        builder.write_sequence_len(self.concept.active_concepts.len());
        for concept in &self.concept.active_concepts {
            builder.write_u64(concept.concept_id.raw());
            builder.write_f32(concept.activation.raw())?;
            builder.write_f32(concept.utility.raw())?;
        }
        builder.write_sequence_len(self.concept.target_contexts.len());
        for context in &self.concept.target_contexts {
            match context.target {
                Some(target) => {
                    builder.write_some();
                    builder.write_u8(target.kind_tag());
                    builder.write_u64(target.raw());
                }
                None => builder.write_none(),
            }
            builder.write_u8(context.action_family.raw());
            builder.write_u64(context.concept_id.raw());
            builder.write_f32(context.concept_signal.raw())?;
            match context.gap_id {
                Some(gap_id) => {
                    builder.write_some();
                    builder.write_u64(gap_id.raw());
                }
                None => builder.write_none(),
            }
            builder.write_f32(context.gap_signal.raw())?;
            builder.write_f32(context.causal_signal.raw())?;
            builder.write_f32(context.contradiction_signal.raw())?;
            builder.write_f32(context.uncertainty.raw())?;
        }
        for word in self.concept.topology_digest {
            builder.write_u64(word);
        }
        builder.write_sequence_len(self.gap.active_gaps.len());
        for gap in &self.gap.active_gaps {
            builder.write_u64(gap.gap_id.raw());
            builder.write_f32(gap.voltage.raw())?;
            builder.write_f32(gap.uncertainty.raw())?;
        }
        builder.write_f32(self.gap.gap_voltage.raw())?;
        builder.write_sequence_len(self.prediction.prediction_error.len());
        for value in &self.prediction.prediction_error {
            builder.write_f32(value.raw())?;
        }
        builder.write_u16(self.prediction.semantic_state_abi);
        for word in self.prediction.source_digest {
            builder.write_u64(word);
        }
        match &self.prediction.source_state {
            Some(state) => {
                builder.write_some();
                for word in state.canonical_digest()? {
                    builder.write_u64(word);
                }
            }
            None => builder.write_none(),
        }
        builder.write_f32(self.prediction.action_sensitivity.raw())?;
        builder.write_u16(self.budget.peripheral_capacity);
        builder.write_u8(self.budget.focal_capacity);
        builder.write_u16(self.budget.focal_feature_width);
        builder.write_u64(self.budget.work_limit);
        builder.write_u64(self.budget.work_used);
        builder.write_u64(self.budget.peripheral_work_units);
        builder.write_u64(self.budget.focal_work_units);
        Ok(builder.finish256())
    }
}

impl Validate for CognitiveContextFrame {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != COGNITIVE_CONTEXT_SCHEMA_VERSION {
            return Err(ScaffoldContractError::IncompatibleAbi {
                kind: crate::SchemaKind::Experience,
                expected: COGNITIVE_CONTEXT_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        self.attention.validate_contract()?;
        if self.attention.organism_id != self.organism_id
            || self.attention.sequence_id != self.sequence_id
            || self.attention.world_tick != self.world_tick
        {
            return Err(ScaffoldContractError::MismatchedCreatureId);
        }
        self.peripheral.validate_contract()?;
        self.focal.validate_contract()?;
        self.interoceptive.validate_contract()?;
        self.memory.validate_contract()?;
        self.concept.validate_contract()?;
        self.gap.validate_contract()?;
        self.prediction.validate_contract()?;
        self.budget.validate_contract()?;
        if self.peripheral.summaries.len() > usize::from(self.budget.peripheral_capacity)
            || self.focal.identities.len() > usize::from(self.budget.focal_capacity)
            || self.focal.grounded_details.len() > self.focal.identities.len()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }
}

fn validate_focus_identity(identity: StableFocusIdentity) -> Result<(), ScaffoldContractError> {
    match identity {
        StableFocusIdentity::TrackedObject(id) => id.validate().map(|_| ()),
        StableFocusIdentity::Organism(id) => id.validate().map(|_| ()),
        StableFocusIdentity::Concept(id) => id.validate().map(|_| ()),
        StableFocusIdentity::NeuralStructural(id) => id.validate().map(|_| ()),
    }
}

fn write_salience(
    builder: &mut CanonicalDigestBuilder,
    salience: SalienceComponents,
) -> Result<(), ScaffoldContractError> {
    for value in [
        salience.peripheral_intensity,
        salience.drive,
        salience.memory_expectancy,
        salience.concept,
        salience.novelty,
        salience.uncertainty,
        salience.gap_voltage,
    ] {
        builder.write_f32(value.raw())?;
    }
    Ok(())
}

fn write_interoceptive(
    builder: &mut CanonicalDigestBuilder,
    view: CognitiveInteroceptiveView,
) -> Result<(), ScaffoldContractError> {
    for value in [
        view.hunger,
        view.fatigue,
        view.pain,
        view.injury,
        view.temperature_stress,
        view.sleep_pressure,
        view.energy,
        view.brain_atp,
    ] {
        builder.write_f32(value.raw())?;
    }
    Ok(())
}

trait FocusIdentityEncoding {
    fn kind_tag(self) -> u8;
    fn raw(self) -> u64;
}

impl FocusIdentityEncoding for StableFocusIdentity {
    fn kind_tag(self) -> u8 {
        match self {
            StableFocusIdentity::TrackedObject(_) => 0,
            StableFocusIdentity::Organism(_) => 1,
            StableFocusIdentity::Concept(_) => 2,
            StableFocusIdentity::NeuralStructural(_) => 3,
        }
    }

    fn raw(self) -> u64 {
        match self {
            StableFocusIdentity::TrackedObject(id) => id.raw(),
            StableFocusIdentity::Organism(id) => id.raw(),
            StableFocusIdentity::Concept(id) => id.raw(),
            StableFocusIdentity::NeuralStructural(id) => id.raw(),
        }
    }
}
