//! v0 scaffold: CPU-side topological concept map and curiosity gap contracts.

use serde::{Deserialize, Serialize};

use crate::{
    validate_finite, ActionId, ActionKind, AffordanceBits, ConceptCellId, Confidence,
    DriveSnapshot, ExperiencePatch, GaussianClusterId, NormalizedScalar, OrganismId,
    ScaffoldContractError, SignedValence, Tick, Validate, Vec3f, WorldEntityId,
};

const MAX_BINDING_REFS: usize = 32;
const MAX_SIMPLEX_CONCEPTS: usize = 8;
const CONTRADICTION_ERROR_THRESHOLD: f32 = 0.65;
const EDGE_STRENGTH_INCREMENT: f32 = 0.2;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CognitiveEdgeId(pub u64);

impl CognitiveEdgeId {
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

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CognitiveSimplexId(pub u64);

impl CognitiveSimplexId {
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

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnresolvedGapId(pub u64);

impl UnresolvedGapId {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriveChannel {
    Hunger,
    Fatigue,
    Fear,
    Pain,
    Loneliness,
    Curiosity,
    BrainAtp,
    TemperatureStress,
    ReproductiveDrive,
    Extension0,
    Extension1,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DriveBinding {
    pub channel: DriveChannel,
    pub value: f32,
}

impl Validate for DriveBinding {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_finite(self.value)?;
        if (0.0..=1.0).contains(&self.value) {
            Ok(())
        } else {
            Err(ScaffoldContractError::ScalarOutOfRange)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionObservationFact {
    pub action_id: ActionId,
    pub kind: ActionKind,
    pub confidence: Confidence,
}

impl Validate for ActionObservationFact {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.action_id.validate()?;
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EmotionValenceSummary {
    pub mean_valence: SignedValence,
    pub mean_prediction_error: NormalizedScalar,
    pub observation_count: u32,
}

impl Default for EmotionValenceSummary {
    fn default() -> Self {
        Self {
            mean_valence: SignedValence(0.0),
            mean_prediction_error: NormalizedScalar(0.0),
            observation_count: 0,
        }
    }
}

impl EmotionValenceSummary {
    fn record(
        &mut self,
        valence: SignedValence,
        prediction_error: NormalizedScalar,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_contract()?;
        SignedValence::new(valence.raw())?;
        NormalizedScalar::new(prediction_error.raw())?;

        let count = self.observation_count.saturating_add(1);
        let previous = self.observation_count as f32;
        let next = count as f32;
        self.mean_valence =
            SignedValence::new(((self.mean_valence.raw() * previous) + valence.raw()) / next)?;
        self.mean_prediction_error = NormalizedScalar::new(
            ((self.mean_prediction_error.raw() * previous) + prediction_error.raw()) / next,
        )?;
        self.observation_count = count;
        Ok(())
    }
}

impl Validate for EmotionValenceSummary {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        SignedValence::new(self.mean_valence.raw())?;
        NormalizedScalar::new(self.mean_prediction_error.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConceptBindings {
    pub objects: Vec<WorldEntityId>,
    pub words: Vec<u32>,
    pub drives: Vec<DriveBinding>,
    pub actions: Vec<ActionObservationFact>,
    pub emotions: EmotionValenceSummary,
    pub locations: Vec<Vec3f>,
    pub agents: Vec<OrganismId>,
    pub affordances: AffordanceBits,
    pub semantic_refs: Vec<ConceptCellId>,
    pub cluster_refs: Vec<GaussianClusterId>,
}

impl Default for ConceptBindings {
    fn default() -> Self {
        Self {
            objects: Vec::new(),
            words: Vec::new(),
            drives: Vec::new(),
            actions: Vec::new(),
            emotions: EmotionValenceSummary::default(),
            locations: Vec::new(),
            agents: Vec::new(),
            affordances: AffordanceBits::NONE,
            semantic_refs: Vec::new(),
            cluster_refs: Vec::new(),
        }
    }
}

impl Validate for ConceptBindings {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.objects.len() > MAX_BINDING_REFS
            || self.words.len() > MAX_BINDING_REFS
            || self.drives.len() > MAX_BINDING_REFS
            || self.actions.len() > MAX_BINDING_REFS
            || self.locations.len() > MAX_BINDING_REFS
            || self.agents.len() > MAX_BINDING_REFS
            || self.semantic_refs.len() > MAX_BINDING_REFS
            || self.cluster_refs.len() > MAX_BINDING_REFS
        {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }
        for id in &self.objects {
            id.validate()?;
        }
        for word in &self.words {
            if *word == 0 {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        for drive in &self.drives {
            drive.validate_contract()?;
        }
        for action in &self.actions {
            action.validate_contract()?;
        }
        self.emotions.validate_contract()?;
        for location in &self.locations {
            location.validate()?;
        }
        for agent in &self.agents {
            agent.validate()?;
        }
        for concept in &self.semantic_refs {
            concept.validate()?;
        }
        for cluster in &self.cluster_refs {
            cluster.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConceptCell {
    pub id: ConceptCellId,
    pub bindings: ConceptBindings,
    pub observation_count: u32,
    pub first_tick: Tick,
    pub last_tick: Tick,
    pub confidence: Confidence,
    pub salience: NormalizedScalar,
}

impl ConceptCell {
    pub fn new(
        id: ConceptCellId,
        bindings: ConceptBindings,
    ) -> Result<Self, ScaffoldContractError> {
        id.validate()?;
        bindings.validate_contract()?;
        Ok(Self {
            id,
            bindings,
            observation_count: 0,
            first_tick: Tick::ZERO,
            last_tick: Tick::ZERO,
            confidence: Confidence(0.0),
            salience: NormalizedScalar(0.0),
        })
    }

    fn observe(
        &mut self,
        mut bindings: ConceptBindings,
        tick: Tick,
        salience: NormalizedScalar,
    ) -> Result<(), ScaffoldContractError> {
        bindings.validate_contract()?;
        NormalizedScalar::new(salience.raw())?;

        if self.observation_count == 0 {
            self.first_tick = tick;
        }
        Tick::validate_monotonic(self.last_tick, tick)?;
        self.last_tick = tick;
        self.observation_count = self.observation_count.saturating_add(1);

        append_unique_bounded(&mut self.bindings.objects, bindings.objects.drain(..))?;
        append_unique_bounded(&mut self.bindings.words, bindings.words.drain(..))?;
        append_unique_bounded(&mut self.bindings.drives, bindings.drives.drain(..))?;
        append_unique_bounded(&mut self.bindings.actions, bindings.actions.drain(..))?;
        append_unique_bounded(&mut self.bindings.locations, bindings.locations.drain(..))?;
        append_unique_bounded(&mut self.bindings.agents, bindings.agents.drain(..))?;
        append_unique_bounded(
            &mut self.bindings.semantic_refs,
            bindings.semantic_refs.drain(..),
        )?;
        append_unique_bounded(
            &mut self.bindings.cluster_refs,
            bindings.cluster_refs.drain(..),
        )?;
        self.bindings.affordances |= bindings.affordances;
        self.bindings.emotions.record(
            bindings.emotions.mean_valence,
            bindings.emotions.mean_prediction_error,
        )?;

        let confidence = (self.observation_count as f32 / 4.0).min(1.0);
        self.confidence = Confidence::new(confidence)?;
        self.salience = NormalizedScalar::new(self.salience.raw().max(salience.raw()))?;
        self.validate_contract()
    }
}

impl Validate for ConceptCell {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.id.validate()?;
        self.bindings.validate_contract()?;
        Confidence::new(self.confidence.raw())?;
        NormalizedScalar::new(self.salience.raw())?;
        Tick::validate_monotonic(self.first_tick, self.last_tick)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeRelationKind {
    Predicts,
    Causes,
    SatisfiesDrive,
    BelongsTo,
    SociallyLiked,
    SociallyFeared,
    Contradicts,
    CoOccurs,
    Enables,
    Blocks,
    TeacherLabels,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveEdge {
    pub id: CognitiveEdgeId,
    pub from: ConceptCellId,
    pub to: ConceptCellId,
    pub relation: EdgeRelationKind,
    pub strength: NormalizedScalar,
    pub evidence_count: u32,
    pub first_tick: Tick,
    pub last_tick: Tick,
    pub confidence: Confidence,
}

impl CognitiveEdge {
    pub fn new(
        from: ConceptCellId,
        to: ConceptCellId,
        relation: EdgeRelationKind,
        strength: NormalizedScalar,
        tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        Self::with_id(CognitiveEdgeId(1), from, to, relation, strength, tick)
    }

    fn with_id(
        id: CognitiveEdgeId,
        from: ConceptCellId,
        to: ConceptCellId,
        relation: EdgeRelationKind,
        strength: NormalizedScalar,
        tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        id.validate()?;
        from.validate()?;
        to.validate()?;
        NormalizedScalar::new(strength.raw())?;
        Ok(Self {
            id,
            from,
            to,
            relation,
            strength,
            evidence_count: 1,
            first_tick: tick,
            last_tick: tick,
            confidence: Confidence::new(strength.raw())?,
        })
    }

    fn strengthen(
        &mut self,
        amount: f32,
        tick: Tick,
    ) -> Result<CognitiveEdgeId, ScaffoldContractError> {
        validate_finite(amount)?;
        Tick::validate_monotonic(self.last_tick, tick)?;
        let next = (self.strength.raw() + amount).clamp(0.0, 1.0);
        self.strength = NormalizedScalar::new(next)?;
        self.evidence_count = self.evidence_count.saturating_add(1);
        self.last_tick = tick;
        self.confidence = Confidence::new((self.evidence_count as f32 / 4.0).min(1.0))?;
        Ok(self.id)
    }

    fn decay(&mut self, amount: f32) -> Result<(), ScaffoldContractError> {
        validate_finite(amount)?;
        self.strength = NormalizedScalar::new((self.strength.raw() - amount).max(0.0))?;
        Ok(())
    }
}

impl Validate for CognitiveEdge {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.id.validate()?;
        self.from.validate()?;
        self.to.validate()?;
        NormalizedScalar::new(self.strength.raw())?;
        Confidence::new(self.confidence.raw())?;
        Tick::validate_monotonic(self.first_tick, self.last_tick)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitiveSimplex {
    pub id: CognitiveSimplexId,
    pub concept_ids: Vec<ConceptCellId>,
    pub observation_count: u32,
    pub mean_valence: SignedValence,
    pub mean_prediction_error: NormalizedScalar,
    pub salience: NormalizedScalar,
    pub first_tick: Tick,
    pub last_tick: Tick,
}

impl CognitiveSimplex {
    pub fn new(
        id: CognitiveSimplexId,
        concept_ids: Vec<ConceptCellId>,
        valence: SignedValence,
        prediction_error: NormalizedScalar,
        salience: NormalizedScalar,
        tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        let simplex = Self {
            id,
            concept_ids,
            observation_count: 1,
            mean_valence: valence,
            mean_prediction_error: prediction_error,
            salience,
            first_tick: tick,
            last_tick: tick,
        };
        simplex.validate_contract()?;
        Ok(simplex)
    }
}

impl Validate for CognitiveSimplex {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.id.validate()?;
        if self.concept_ids.is_empty() || self.concept_ids.len() > MAX_SIMPLEX_CONCEPTS {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }
        for id in &self.concept_ids {
            id.validate()?;
        }
        SignedValence::new(self.mean_valence.raw())?;
        NormalizedScalar::new(self.mean_prediction_error.raw())?;
        NormalizedScalar::new(self.salience.raw())?;
        Tick::validate_monotonic(self.first_tick, self.last_tick)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContradictionType {
    OutcomeContradiction,
    PredictionError,
    TeacherLabelConflict,
    SocialValenceConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GapResolutionStatus {
    Open,
    BiasingCuriosity,
    Resolved,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnresolvedGap {
    pub id: UnresolvedGapId,
    pub source_concepts: Vec<ConceptCellId>,
    pub contradiction_type: ContradictionType,
    pub prediction_error: NormalizedScalar,
    pub curiosity_voltage: NormalizedScalar,
    pub salience: NormalizedScalar,
    pub first_tick: Tick,
    pub last_tick: Tick,
    pub confidence: Confidence,
    pub status: GapResolutionStatus,
}

impl UnresolvedGap {
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: UnresolvedGapId,
        source_concepts: Vec<ConceptCellId>,
        contradiction_type: ContradictionType,
        prediction_error: NormalizedScalar,
        curiosity_voltage: NormalizedScalar,
        salience: NormalizedScalar,
        tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        let gap = Self {
            id,
            source_concepts,
            contradiction_type,
            prediction_error,
            curiosity_voltage,
            salience,
            first_tick: tick,
            last_tick: tick,
            confidence: Confidence::new(salience.raw())?,
            status: GapResolutionStatus::Open,
        };
        gap.validate_contract()?;
        Ok(gap)
    }

    fn reinforce(
        &mut self,
        prediction_error: NormalizedScalar,
        salience: NormalizedScalar,
        tick: Tick,
    ) -> Result<UnresolvedGapId, ScaffoldContractError> {
        Tick::validate_monotonic(self.last_tick, tick)?;
        self.prediction_error =
            NormalizedScalar::new(self.prediction_error.raw().max(prediction_error.raw()))?;
        self.curiosity_voltage = NormalizedScalar::new(
            (self.curiosity_voltage.raw() + prediction_error.raw() * 0.2).clamp(0.0, 1.0),
        )?;
        self.salience = NormalizedScalar::new(self.salience.raw().max(salience.raw()))?;
        self.last_tick = tick;
        self.confidence = Confidence::new((self.confidence.raw() + 0.2).min(1.0))?;
        Ok(self.id)
    }
}

impl Validate for UnresolvedGap {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.id.validate()?;
        if self.source_concepts.is_empty() || self.source_concepts.len() > MAX_SIMPLEX_CONCEPTS {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }
        for id in &self.source_concepts {
            id.validate()?;
        }
        NormalizedScalar::new(self.prediction_error.raw())?;
        NormalizedScalar::new(self.curiosity_voltage.raw())?;
        NormalizedScalar::new(self.salience.raw())?;
        Confidence::new(self.confidence.raw())?;
        Tick::validate_monotonic(self.first_tick, self.last_tick)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TopologicalMapConfig {
    pub max_concepts: usize,
    pub max_edges: usize,
    pub max_simplexes: usize,
    pub max_unresolved_gaps: usize,
    pub edge_decay_per_tick: NormalizedScalar,
}

impl Default for TopologicalMapConfig {
    fn default() -> Self {
        Self {
            max_concepts: 256,
            max_edges: 512,
            max_simplexes: 256,
            max_unresolved_gaps: 64,
            edge_decay_per_tick: NormalizedScalar(0.01),
        }
    }
}

impl Validate for TopologicalMapConfig {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.max_concepts == 0
            || self.max_edges == 0
            || self.max_simplexes == 0
            || self.max_unresolved_gaps == 0
        {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }
        NormalizedScalar::new(self.edge_decay_per_tick.raw())?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologyUpdate {
    pub primary_concept_id: ConceptCellId,
    pub edge_ids: Vec<CognitiveEdgeId>,
    pub simplex_id: CognitiveSimplexId,
    pub gap_ids: Vec<UnresolvedGapId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CuriosityBias {
    pub gap_id: UnresolvedGapId,
    pub source_concepts: Vec<ConceptCellId>,
    pub salience: NormalizedScalar,
    pub curiosity_voltage: NormalizedScalar,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConceptSignature {
    Object(WorldEntityId),
    Action(ActionId),
    Word(u32),
    Sequence(u64),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologicalMap {
    config: TopologicalMapConfig,
    concepts: Vec<ConceptCell>,
    edges: Vec<CognitiveEdge>,
    simplexes: Vec<CognitiveSimplex>,
    unresolved_gaps: Vec<UnresolvedGap>,
    next_concept_id: u64,
    next_edge_id: u64,
    next_simplex_id: u64,
    next_gap_id: u64,
}

impl TopologicalMap {
    pub fn new(config: TopologicalMapConfig) -> Result<Self, ScaffoldContractError> {
        config.validate_contract()?;
        Ok(Self {
            config,
            concepts: Vec::new(),
            edges: Vec::new(),
            simplexes: Vec::new(),
            unresolved_gaps: Vec::new(),
            next_concept_id: 1,
            next_edge_id: 1,
            next_simplex_id: 1,
            next_gap_id: 1,
        })
    }

    pub fn concepts(&self) -> &[ConceptCell] {
        &self.concepts
    }

    pub fn edges(&self) -> &[CognitiveEdge] {
        &self.edges
    }

    pub fn simplexes(&self) -> &[CognitiveSimplex] {
        &self.simplexes
    }

    pub fn unresolved_gaps(&self) -> &[UnresolvedGap] {
        &self.unresolved_gaps
    }

    pub fn concept(&self, id: ConceptCellId) -> Option<&ConceptCell> {
        self.concepts.iter().find(|concept| concept.id == id)
    }

    pub fn edge(&self, id: CognitiveEdgeId) -> Option<&CognitiveEdge> {
        self.edges.iter().find(|edge| edge.id == id)
    }

    pub fn gap(&self, id: UnresolvedGapId) -> Option<&UnresolvedGap> {
        self.unresolved_gaps.iter().find(|gap| gap.id == id)
    }

    pub fn apply_patch(
        &mut self,
        patch: &ExperiencePatch,
    ) -> Result<TopologyUpdate, ScaffoldContractError> {
        patch.validate_contract()?;

        let primary_signature = primary_signature(patch);
        let action_signature = ConceptSignature::Action(patch.decision().selected_action.action_id);
        let primary_bindings = bindings_from_patch(patch, false)?;
        let action_bindings = bindings_from_patch(patch, true)?;
        let tick = patch.outcome().outcome_tick;
        let salience = patch_salience(patch)?;

        let primary_concept_id =
            self.ensure_concept(primary_signature, primary_bindings, tick, salience)?;
        let action_concept_id =
            self.ensure_concept(action_signature, action_bindings, tick, salience)?;
        let edge_id = self.ensure_edge(
            primary_concept_id,
            action_concept_id,
            EdgeRelationKind::CoOccurs,
            salience,
            tick,
        )?;
        let simplex_id = self.push_simplex(
            vec![primary_concept_id, action_concept_id],
            patch.outcome().reward_valence,
            patch.outcome().prediction_error,
            salience,
            tick,
        )?;
        let gap_ids = self.detect_or_update_gap(primary_concept_id, patch, salience, tick)?;

        Ok(TopologyUpdate {
            primary_concept_id,
            edge_ids: vec![edge_id],
            simplex_id,
            gap_ids,
        })
    }

    pub fn decay_edges(&mut self, elapsed_ticks: u64) -> Result<(), ScaffoldContractError> {
        let amount = self.edge_decay_per_tick_amount(elapsed_ticks)?;
        for edge in &mut self.edges {
            edge.decay(amount)?;
        }
        Ok(())
    }

    pub fn curiosity_biases(&self) -> Vec<CuriosityBias> {
        self.unresolved_gaps
            .iter()
            .filter(|gap| {
                matches!(
                    gap.status,
                    GapResolutionStatus::Open | GapResolutionStatus::BiasingCuriosity
                )
            })
            .map(|gap| CuriosityBias {
                gap_id: gap.id,
                source_concepts: gap.source_concepts.clone(),
                salience: NormalizedScalar(gap.salience.raw().max(gap.curiosity_voltage.raw())),
                curiosity_voltage: gap.curiosity_voltage,
                confidence: gap.confidence,
            })
            .collect()
    }

    fn ensure_concept(
        &mut self,
        signature: ConceptSignature,
        bindings: ConceptBindings,
        tick: Tick,
        salience: NormalizedScalar,
    ) -> Result<ConceptCellId, ScaffoldContractError> {
        if let Some(index) = self.find_concept_index(&signature) {
            let id = self.concepts[index].id;
            self.concepts[index].observe(bindings, tick, salience)?;
            return Ok(id);
        }

        if self.concepts.len() >= self.config.max_concepts {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }

        let id = ConceptCellId(self.next_concept_id);
        self.next_concept_id = self.next_concept_id.saturating_add(1);
        let mut concept = ConceptCell::new(id, ConceptBindings::default())?;
        concept.observe(bindings, tick, salience)?;
        self.concepts.push(concept);
        Ok(id)
    }

    fn find_concept_index(&self, signature: &ConceptSignature) -> Option<usize> {
        self.concepts
            .iter()
            .position(|concept| concept_matches_signature(concept, signature))
    }

    fn ensure_edge(
        &mut self,
        from: ConceptCellId,
        to: ConceptCellId,
        relation: EdgeRelationKind,
        salience: NormalizedScalar,
        tick: Tick,
    ) -> Result<CognitiveEdgeId, ScaffoldContractError> {
        if let Some(edge) = self
            .edges
            .iter_mut()
            .find(|edge| edge.from == from && edge.to == to && edge.relation == relation)
        {
            return edge.strengthen(EDGE_STRENGTH_INCREMENT.max(salience.raw() * 0.2), tick);
        }

        if self.edges.len() >= self.config.max_edges {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }

        let id = CognitiveEdgeId(self.next_edge_id);
        self.next_edge_id = self.next_edge_id.saturating_add(1);
        let edge = CognitiveEdge::with_id(id, from, to, relation, salience, tick)?;
        self.edges.push(edge);
        Ok(id)
    }

    fn push_simplex(
        &mut self,
        concept_ids: Vec<ConceptCellId>,
        valence: SignedValence,
        prediction_error: NormalizedScalar,
        salience: NormalizedScalar,
        tick: Tick,
    ) -> Result<CognitiveSimplexId, ScaffoldContractError> {
        if self.simplexes.len() >= self.config.max_simplexes {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }

        let id = CognitiveSimplexId(self.next_simplex_id);
        self.next_simplex_id = self.next_simplex_id.saturating_add(1);
        let simplex =
            CognitiveSimplex::new(id, concept_ids, valence, prediction_error, salience, tick)?;
        self.simplexes.push(simplex);
        Ok(id)
    }

    fn detect_or_update_gap(
        &mut self,
        source_concept: ConceptCellId,
        patch: &ExperiencePatch,
        salience: NormalizedScalar,
        tick: Tick,
    ) -> Result<Vec<UnresolvedGapId>, ScaffoldContractError> {
        let prediction_error = patch.outcome().prediction_error;
        let contradiction_type = if patch.outcome().contradiction_observed {
            Some(ContradictionType::OutcomeContradiction)
        } else if prediction_error.raw() >= CONTRADICTION_ERROR_THRESHOLD {
            Some(ContradictionType::PredictionError)
        } else {
            None
        };

        let Some(contradiction_type) = contradiction_type else {
            return Ok(Vec::new());
        };

        if let Some(gap) = self.unresolved_gaps.iter_mut().find(|gap| {
            gap.source_concepts.contains(&source_concept)
                && gap.contradiction_type == contradiction_type
                && matches!(
                    gap.status,
                    GapResolutionStatus::Open | GapResolutionStatus::BiasingCuriosity
                )
        }) {
            let id = gap.reinforce(prediction_error, salience, tick)?;
            return Ok(vec![id]);
        }

        if self.unresolved_gaps.len() >= self.config.max_unresolved_gaps {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }

        let id = UnresolvedGapId(self.next_gap_id);
        self.next_gap_id = self.next_gap_id.saturating_add(1);
        let curiosity_voltage =
            NormalizedScalar::new((prediction_error.raw() * 0.8 + salience.raw() * 0.2).min(1.0))?;
        let gap = UnresolvedGap::new(
            id,
            vec![source_concept],
            contradiction_type,
            prediction_error,
            curiosity_voltage,
            salience,
            tick,
        )?;
        self.unresolved_gaps.push(gap);
        Ok(vec![id])
    }

    fn edge_decay_per_tick_amount(&self, elapsed_ticks: u64) -> Result<f32, ScaffoldContractError> {
        let elapsed = elapsed_ticks.min(u64::from(u32::MAX)) as f32;
        validate_finite(self.config.edge_decay_per_tick.raw() * elapsed)
    }
}

impl Validate for TopologicalMap {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.config.validate_contract()?;
        if self.concepts.len() > self.config.max_concepts
            || self.edges.len() > self.config.max_edges
            || self.simplexes.len() > self.config.max_simplexes
            || self.unresolved_gaps.len() > self.config.max_unresolved_gaps
        {
            return Err(ScaffoldContractError::TopologyCapacityExceeded);
        }
        for concept in &self.concepts {
            concept.validate_contract()?;
        }
        for edge in &self.edges {
            edge.validate_contract()?;
        }
        for simplex in &self.simplexes {
            simplex.validate_contract()?;
        }
        for gap in &self.unresolved_gaps {
            gap.validate_contract()?;
        }
        Ok(())
    }
}

fn primary_signature(patch: &ExperiencePatch) -> ConceptSignature {
    patch
        .decision()
        .selected_action
        .target_entity
        .or(patch.outcome().physical.target_entity)
        .map_or_else(
            || {
                first_heard_word(patch).map_or(
                    ConceptSignature::Sequence(patch.header().sequence_id.raw()),
                    ConceptSignature::Word,
                )
            },
            ConceptSignature::Object,
        )
}

fn first_heard_word(patch: &ExperiencePatch) -> Option<u32> {
    patch
        .pre_action()
        .sensory
        .context_streams
        .vocal_tokens
        .iter()
        .flatten()
        .map(|token| token.token_id)
        .chain(
            patch
                .pre_action()
                .sensory
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .map(|token| token.token_id),
        )
        .find(|token_id| *token_id != 0)
}

fn bindings_from_patch(
    patch: &ExperiencePatch,
    action_only: bool,
) -> Result<ConceptBindings, ScaffoldContractError> {
    let mut bindings = ConceptBindings::default();
    let pre = patch.pre_action();
    let decision = patch.decision();
    let outcome = patch.outcome();

    bindings.actions.push(ActionObservationFact {
        action_id: decision.selected_action.action_id,
        kind: decision.selected_action.kind,
        confidence: decision.selected_action.confidence,
    });
    bindings
        .emotions
        .record(outcome.reward_valence, outcome.prediction_error)?;

    if !action_only {
        if let Some(target) = decision.selected_action.target_entity {
            push_unique(&mut bindings.objects, target);
        }
        if let Some(target) = outcome.physical.target_entity {
            push_unique(&mut bindings.objects, target);
        }
        push_unique(&mut bindings.locations, pre.body_pose.translation);
        bindings.affordances = pre.sensory.channels.nearby_affordances;
        bindings.drives = drive_bindings(pre.homeostasis.drives);

        for token in pre.sensory.context_streams.vocal_tokens.iter().flatten() {
            push_unique(&mut bindings.words, token.token_id);
            if let Some(entity) = token.source_entity {
                push_unique(&mut bindings.objects, entity);
            }
            if let Some(speaker) = token.speaker_id {
                push_unique(&mut bindings.agents, speaker);
            }
        }
        for token in pre.sensory.language_context.heard_tokens.iter().flatten() {
            push_unique(&mut bindings.words, token.token_id);
            if let Some(entity) = token.source_entity {
                push_unique(&mut bindings.objects, entity);
            }
            if let Some(speaker) = token.speaker_id {
                push_unique(&mut bindings.agents, speaker);
            }
        }
        if let Some(token) = pre.sensory.language_context.vocalized_token {
            push_unique(&mut bindings.words, token.token_id);
        }
        for social in pre.sensory.social_context.nearest_agents.iter().flatten() {
            push_unique(&mut bindings.agents, social.agent_id);
            if let Some(entity) = social.body_entity {
                push_unique(&mut bindings.objects, entity);
            }
        }
        if let Some(semantic) = &pre.sensory.semantic_context {
            for entry in &semantic.salience {
                push_unique(&mut bindings.semantic_refs, entry.concept_id);
            }
        }
        if let Some(gaussian) = &pre.sensory.gaussian_context {
            for entry in &gaussian.clusters {
                push_unique(&mut bindings.cluster_refs, entry.cluster_id);
            }
        }
    }

    bindings.validate_contract()?;
    Ok(bindings)
}

fn drive_bindings(drives: DriveSnapshot) -> Vec<DriveBinding> {
    [
        (DriveChannel::Hunger, drives.hunger),
        (DriveChannel::Fatigue, drives.fatigue),
        (DriveChannel::Fear, drives.fear),
        (DriveChannel::Pain, drives.pain),
        (DriveChannel::Loneliness, drives.loneliness),
        (DriveChannel::Curiosity, drives.curiosity),
        (DriveChannel::BrainAtp, drives.brain_atp),
        (DriveChannel::TemperatureStress, drives.temperature_stress),
        (DriveChannel::ReproductiveDrive, drives.reproductive_drive),
        (DriveChannel::Extension0, drives.extension[0]),
        (DriveChannel::Extension1, drives.extension[1]),
    ]
    .into_iter()
    .map(|(channel, value)| DriveBinding { channel, value })
    .collect()
}

fn patch_salience(patch: &ExperiencePatch) -> Result<NormalizedScalar, ScaffoldContractError> {
    let pre = patch.pre_action();
    let outcome = patch.outcome();
    let drive_salience = pre
        .homeostasis
        .drives
        .curiosity
        .max(pre.homeostasis.drives.fear);
    let sensory_salience = pre
        .sensory
        .channels
        .novelty_signal
        .raw()
        .max(pre.sensory.channels.pain_signal.raw());
    let outcome_salience = outcome
        .prediction_error
        .raw()
        .max(outcome.reward_valence.raw().abs());
    NormalizedScalar::new(drive_salience.max(sensory_salience).max(outcome_salience))
}

fn concept_matches_signature(concept: &ConceptCell, signature: &ConceptSignature) -> bool {
    match signature {
        ConceptSignature::Object(id) => concept.bindings.objects.contains(id),
        ConceptSignature::Action(id) => {
            concept.bindings.objects.is_empty()
                && concept
                    .bindings
                    .actions
                    .iter()
                    .any(|action| action.action_id == *id)
        }
        ConceptSignature::Word(id) => concept.bindings.words.contains(id),
        ConceptSignature::Sequence(sequence) => {
            concept.id.raw() == *sequence && concept.observation_count > 0
        }
    }
}

fn append_unique_bounded<T: Copy + PartialEq>(
    target: &mut Vec<T>,
    values: impl Iterator<Item = T>,
) -> Result<(), ScaffoldContractError> {
    for value in values {
        if !target.contains(&value) {
            if target.len() >= MAX_BINDING_REFS {
                return Err(ScaffoldContractError::TopologyCapacityExceeded);
            }
            target.push(value);
        }
    }
    Ok(())
}

fn push_unique<T: Copy + PartialEq>(target: &mut Vec<T>, value: T) {
    if !target.contains(&value) && target.len() < MAX_BINDING_REFS {
        target.push(value);
    }
}
