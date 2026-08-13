//! Bounded cognitive context assembled before a v1.1 decision.

use serde::{Deserialize, Serialize};

use crate::{
    AttentionFrame, CanonicalDigestBuilder, ConceptCellId, ExperienceSequenceId, HysteresisState,
    MemoryId, NormalizedScalar, OrganismId, SalienceComponents, ScaffoldContractError,
    SignedValence, StableFocusIdentity, Tick, UnresolvedGapId, Validate, MAX_FOCAL_TARGETS,
    MAX_PERIPHERAL_SUMMARIES,
};

pub const COGNITIVE_CONTEXT_SCHEMA_VERSION: u16 = 1;
pub const MAX_CONTEXT_MEMORY_EXPECTANCIES: usize = 32;
pub const MAX_ACTIVE_CONCEPTS: usize = 32;
pub const MAX_ACTIVE_GAPS: usize = 32;
pub const MAX_PREDICTION_ERROR_FEATURES: usize = 64;

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
    pub hysteresis: HysteresisState,
}

impl Validate for CognitiveFocalView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.identities.len() > MAX_FOCAL_TARGETS || self.salience.len() > MAX_FOCAL_TARGETS {
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
        self.hysteresis.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CognitiveInteroceptiveView {
    pub hunger: NormalizedScalar,
    pub fatigue: NormalizedScalar,
    pub pain: NormalizedScalar,
    pub temperature_stress: NormalizedScalar,
    pub sleep_pressure: NormalizedScalar,
    pub energy: NormalizedScalar,
}

impl Default for CognitiveInteroceptiveView {
    fn default() -> Self {
        Self {
            hunger: NormalizedScalar(0.0),
            fatigue: NormalizedScalar(0.0),
            pain: NormalizedScalar(0.0),
            temperature_stress: NormalizedScalar(0.0),
            sleep_pressure: NormalizedScalar(0.0),
            energy: NormalizedScalar(1.0),
        }
    }
}

impl Validate for CognitiveInteroceptiveView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        for value in [
            self.hunger,
            self.fatigue,
            self.pain,
            self.temperature_stress,
            self.sleep_pressure,
            self.energy,
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
    pub topology_digest: [u64; 4],
}

impl Validate for CognitiveConceptView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.active_concepts.len() > MAX_ACTIVE_CONCEPTS {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        for concept in &self.active_concepts {
            concept.validate_contract()?;
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
    pub successor_feature_abi: u16,
    pub prediction_error: Vec<NormalizedScalar>,
    pub action_sensitivity: NormalizedScalar,
}

impl Default for CognitivePredictionView {
    fn default() -> Self {
        Self {
            source_digest: [0; 4],
            successor_feature_abi: 0,
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
        if self.successor_feature_abi == 0
            && (!self.prediction_error.is_empty() || self.source_digest != [0; 4])
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
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
    pub work_limit: u64,
    pub work_used: u64,
}

impl Default for CognitiveBudgetView {
    fn default() -> Self {
        Self {
            peripheral_capacity: MAX_PERIPHERAL_SUMMARIES as u16,
            focal_capacity: MAX_FOCAL_TARGETS as u8,
            work_limit: 0,
            work_used: 0,
        }
    }
}

impl Validate for CognitiveBudgetView {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if usize::from(self.peripheral_capacity) > MAX_PERIPHERAL_SUMMARIES
            || usize::from(self.focal_capacity) > MAX_FOCAL_TARGETS
            || self.work_used > self.work_limit
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
        builder.write_u16(self.prediction.successor_feature_abi);
        for word in self.prediction.source_digest {
            builder.write_u64(word);
        }
        builder.write_f32(self.prediction.action_sensitivity.raw())?;
        builder.write_u16(self.budget.peripheral_capacity);
        builder.write_u8(self.budget.focal_capacity);
        builder.write_u64(self.budget.work_limit);
        builder.write_u64(self.budget.work_used);
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
        view.temperature_stress,
        view.sleep_pressure,
        view.energy,
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
