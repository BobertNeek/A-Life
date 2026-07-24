//! v0 scaffold: CPU-side topological concept map and curiosity gap contracts.

use serde::{Deserialize, Serialize};

use crate::{
    validate_finite, ActionId, ActionKind, AffordanceBits, CandidateActionFamily,
    CanonicalDigestBuilder, ConceptCellId, Confidence, DriveSnapshot, ExperiencePatch,
    ExperienceSequenceId, GaussianClusterId, NormalizedScalar, OrganismId, ScaffoldContractError,
    SignedValence, Tick, TrackedObjectId, Validate, Vec3f,
};

const MAX_BINDING_REFS: usize = 32;
const MAX_SIMPLEX_CONCEPTS: usize = 8;
const CONTRADICTION_ERROR_THRESHOLD: f32 = 0.65;
const EDGE_STRENGTH_INCREMENT: f32 = 0.2;
const TOPOLOGY_MAP_DIGEST_DOMAIN: &[u8] = b"alife.topology.map.v2";
const PORTABLE_TOPOLOGY_SIDECAR_DIGEST_DOMAIN: &[u8] = b"ALIFE-PORTABLE-TOPOLOGY-SIDECAR-V1";

pub const PORTABLE_TOPOLOGY_SIDECAR_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableTopologyDriveBindingV1 {
    pub channel_raw: u8,
    pub value_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableTopologyActionBindingV1 {
    pub action_id_raw: u32,
    pub action_kind_raw: u8,
    pub confidence_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableTopologyBindingSetV1 {
    pub tracked_object_ids_raw: Vec<u64>,
    pub word_ids_raw: Vec<u32>,
    pub drives: Vec<PortableTopologyDriveBindingV1>,
    pub actions: Vec<PortableTopologyActionBindingV1>,
    pub action_families_raw: Vec<u8>,
    pub location_bits: Vec<[u32; 3]>,
    pub agent_ids_raw: Vec<u64>,
    pub semantic_concept_ids_raw: Vec<u64>,
    pub cluster_ids_raw: Vec<u64>,
    pub affordance_bits_raw: u32,
    pub mean_valence_bits: u32,
    pub mean_prediction_error_bits: u32,
    pub emotion_observation_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableTopologyConceptV1 {
    pub id_raw: u64,
    pub is_summary: bool,
    pub bindings: PortableTopologyBindingSetV1,
    pub observation_count: u32,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
    pub confidence_bits: u32,
    pub salience_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableTopologyEdgeV1 {
    pub id_raw: u64,
    pub from_raw: u64,
    pub to_raw: u64,
    pub relation_raw: u16,
    pub strength_bits: u32,
    pub evidence_count: u32,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
    pub confidence_bits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableTopologySimplexV1 {
    pub id_raw: u64,
    pub concept_ids_raw: Vec<u64>,
    pub observation_count: u32,
    pub mean_valence_bits: u32,
    pub mean_prediction_error_bits: u32,
    pub salience_bits: u32,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableTopologyGapV1 {
    pub id_raw: u64,
    pub source_concept_ids_raw: Vec<u64>,
    pub contradiction_raw: u16,
    pub prediction_error_bits: u32,
    pub curiosity_voltage_bits: u32,
    pub salience_bits: u32,
    pub first_tick_raw: u64,
    pub last_tick_raw: u64,
    pub confidence_bits: u32,
    pub status_raw: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableTopologySidecarAssetV1 {
    pub schema_version: u16,
    pub organism_id_raw: u64,
    pub profile: crate::SensorProfileIdentity,
    pub max_concepts: u32,
    pub max_edges: u32,
    pub max_simplexes: u32,
    pub max_unresolved_gaps: u32,
    pub max_bindings_per_kind: u32,
    pub edge_decay_bits: u32,
    pub last_observed_sequence_id_raw: u64,
    pub last_observed_key_digest: [u64; 4],
    pub next_concept_id_raw: u64,
    pub next_edge_id_raw: u64,
    pub next_simplex_id_raw: u64,
    pub next_gap_id_raw: u64,
    pub concepts: Vec<PortableTopologyConceptV1>,
    pub edges: Vec<PortableTopologyEdgeV1>,
    pub simplexes: Vec<PortableTopologySimplexV1>,
    pub gaps: Vec<PortableTopologyGapV1>,
    pub observation_count: u64,
    pub degradation_count: u64,
    pub invalid_rejection_count: u64,
    pub replay_rejection_count: u64,
    pub map_digest: [u64; 4],
    pub canonical_digest: [u64; 4],
}

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
    pub objects: Vec<TrackedObjectId>,
    pub words: Vec<u32>,
    pub drives: Vec<DriveBinding>,
    pub actions: Vec<ActionObservationFact>,
    #[serde(default)]
    pub action_families: Vec<CandidateActionFamily>,
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
            action_families: Vec::new(),
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
            || self.action_families.len() > MAX_BINDING_REFS
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
        for family in &self.action_families {
            CandidateActionFamily::try_from_raw(family.raw())?;
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
    #[serde(default)]
    pub is_summary: bool,
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
            is_summary: false,
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
    ) -> Result<bool, ScaffoldContractError> {
        bindings.validate_contract()?;
        NormalizedScalar::new(salience.raw())?;

        if self.observation_count == 0 {
            self.first_tick = tick;
        }
        Tick::validate_monotonic(self.last_tick, tick)?;
        self.last_tick = tick;
        self.observation_count = self.observation_count.saturating_add(1);

        let mut truncated = false;
        truncated |= append_unique_bounded(&mut self.bindings.objects, bindings.objects.drain(..));
        truncated |= append_unique_bounded(&mut self.bindings.words, bindings.words.drain(..));
        truncated |= merge_drive_bindings(&mut self.bindings.drives, bindings.drives.drain(..))?;
        truncated |=
            merge_action_observations(&mut self.bindings.actions, bindings.actions.drain(..))?;
        truncated |= append_unique_bounded(
            &mut self.bindings.action_families,
            bindings.action_families.drain(..),
        );
        truncated |= merge_location_samples(
            &mut self.bindings.locations,
            bindings.locations.drain(..),
            tick,
        )?;
        truncated |= append_unique_bounded(&mut self.bindings.agents, bindings.agents.drain(..));
        truncated |= append_unique_bounded(
            &mut self.bindings.semantic_refs,
            bindings.semantic_refs.drain(..),
        );
        truncated |= append_unique_bounded(
            &mut self.bindings.cluster_refs,
            bindings.cluster_refs.drain(..),
        );
        self.bindings.affordances |= bindings.affordances;
        self.bindings.emotions.record(
            bindings.emotions.mean_valence,
            bindings.emotions.mean_prediction_error,
        )?;

        let confidence = (self.observation_count as f32 / 4.0).min(1.0);
        self.confidence = Confidence::new(confidence)?;
        self.salience = NormalizedScalar::new(self.salience.raw().max(salience.raw()))?;
        self.validate_contract()?;
        Ok(truncated)
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
        mut concept_ids: Vec<ConceptCellId>,
        valence: SignedValence,
        prediction_error: NormalizedScalar,
        salience: NormalizedScalar,
        tick: Tick,
    ) -> Result<Self, ScaffoldContractError> {
        concept_ids.sort_by_key(|concept_id| concept_id.raw());
        concept_ids.dedup();
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
            max_simplexes: 1024,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyCounts {
    pub concepts: u32,
    pub edges: u32,
    pub simplexes: u32,
    pub unresolved_gaps: u32,
}

impl TopologyCounts {
    pub fn within(self, config: &TopologicalMapConfig) -> bool {
        usize::try_from(self.concepts).is_ok_and(|value| value <= config.max_concepts)
            && usize::try_from(self.edges).is_ok_and(|value| value <= config.max_edges)
            && usize::try_from(self.simplexes).is_ok_and(|value| value <= config.max_simplexes)
            && usize::try_from(self.unresolved_gaps)
                .is_ok_and(|value| value <= config.max_unresolved_gaps)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyIdCounters {
    pub next_concept_id: u64,
    pub next_edge_id: u64,
    pub next_simplex_id: u64,
    pub next_gap_id: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TopologyDegradationKind {
    ConceptMergedIntoSummary,
    EdgeEvicted,
    SimplexReplaced,
    GapReplaced,
    PrimaryBindingTruncated,
    ActionBindingTruncated,
    InvalidObservationRejected,
    ReplayRejected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologyObservationReceipt {
    pub organism_id_raw: u64,
    pub sealed_sequence_id: ExperienceSequenceId,
    pub update: Option<TopologyUpdate>,
    pub degradations: Vec<TopologyDegradationKind>,
    pub before_counts: TopologyCounts,
    pub after_counts: TopologyCounts,
    pub before_next_ids: TopologyIdCounters,
    pub after_next_ids: TopologyIdCounters,
    pub before_digest: [u64; 4],
    pub after_digest: [u64; 4],
    pub rejected_invalid: bool,
    pub replay_rejected: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologySidecarDiagnostics {
    pub organism_id_raw: u64,
    pub observations: u64,
    pub degradations: u64,
    pub invalid_rejections: u64,
    pub replay_rejections: u64,
    pub terminal_errors: u64,
    pub canonical_digest: [u64; 4],
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
    TrackedObject(TrackedObjectId),
    Action {
        family: CandidateActionFamily,
        action_id: ActionId,
    },
    Word(u32),
    Sequence(u64),
}

#[derive(Debug, Clone, PartialEq)]
struct TopologyObservationBindings {
    primary_bindings: ConceptBindings,
    action_bindings: ConceptBindings,
}

#[derive(Debug, Clone, PartialEq)]
enum TopologyReplacement {
    Concept {
        index: u32,
        expected_id: Option<ConceptCellId>,
        value: ConceptCell,
    },
    Edge {
        index: u32,
        expected_id: Option<CognitiveEdgeId>,
        value: CognitiveEdge,
    },
    Simplex {
        index: u32,
        expected_id: Option<CognitiveSimplexId>,
        value: CognitiveSimplex,
    },
    Gap {
        index: u32,
        expected_id: Option<UnresolvedGapId>,
        value: UnresolvedGap,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct TopologyMutationPlan {
    expected_digest: [u64; 4],
    final_digest: [u64; 4],
    expected_counts: TopologyCounts,
    final_counts: TopologyCounts,
    expected_next_ids: TopologyIdCounters,
    final_next_ids: TopologyIdCounters,
    primary_signature: ConceptSignature,
    action_signature: ConceptSignature,
    primary_bindings: ConceptBindings,
    action_bindings: ConceptBindings,
    replacements: Vec<TopologyReplacement>,
    degradations: Vec<TopologyDegradationKind>,
    update: TopologyUpdate,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TopologySidecar {
    organism_id: OrganismId,
    profile: crate::SensorProfileIdentity,
    map: TopologicalMap,
    diagnostics: TopologySidecarDiagnostics,
    last_observed_sequence_id: Option<ExperienceSequenceId>,
    last_observed_key_digest: Option<[u64; 4]>,
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

    fn plan_observation(
        &self,
        patch: &ExperiencePatch,
        require_episodic_key: bool,
    ) -> Result<TopologyMutationPlan, ScaffoldContractError> {
        patch.validate_contract()?;
        let key = patch.decision().episodic_key();
        if require_episodic_key && key.is_none() {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        if let Some(key) = key {
            key.validate_contract()?;
        }

        let observation_bindings = bindings_from_patch(patch)?;
        let primary_signature = primary_signature(patch);
        let action_signature = ConceptSignature::Action {
            family: key.map_or_else(
                || CandidateActionFamily::baseline_for_kind(patch.decision().selected_action.kind),
                |value| value.query().action_family(),
            ),
            action_id: patch.decision().selected_action.action_id,
        };
        let expected_digest = self.canonical_digest()?;
        let expected_counts = self.counts();
        let expected_next_ids = self.next_ids();
        let mut planned = self.clone();
        let mut degradations = Vec::new();
        let tick = patch.outcome().outcome_tick;
        let salience = patch_salience(patch)?;

        let primary_concept_id = planned.ensure_concept(
            primary_signature.clone(),
            observation_bindings.primary_bindings.clone(),
            tick,
            salience,
            TopologyDegradationKind::PrimaryBindingTruncated,
            &mut degradations,
        )?;
        let action_concept_id = planned.ensure_concept(
            action_signature.clone(),
            observation_bindings.action_bindings.clone(),
            tick,
            salience,
            TopologyDegradationKind::ActionBindingTruncated,
            &mut degradations,
        )?;
        let edge_id = planned.ensure_edge(
            primary_concept_id,
            action_concept_id,
            EdgeRelationKind::CoOccurs,
            salience,
            tick,
            &mut degradations,
        )?;
        let simplex_id = planned.push_simplex(
            vec![primary_concept_id, action_concept_id],
            patch.outcome().reward_valence,
            patch.outcome().prediction_error,
            salience,
            tick,
            &mut degradations,
        )?;
        let gap_ids = planned.detect_or_update_gap(
            primary_concept_id,
            patch,
            salience,
            tick,
            &mut degradations,
        )?;
        planned.validate_contract()?;
        let final_digest = planned.canonical_digest()?;
        let replacements = diff_replacements(self, &planned)?;
        let plan = TopologyMutationPlan {
            expected_digest,
            final_digest,
            expected_counts,
            final_counts: planned.counts(),
            expected_next_ids,
            final_next_ids: planned.next_ids(),
            primary_signature,
            action_signature,
            primary_bindings: observation_bindings.primary_bindings,
            action_bindings: observation_bindings.action_bindings,
            replacements,
            degradations,
            update: TopologyUpdate {
                primary_concept_id,
                edge_ids: vec![edge_id],
                simplex_id,
                gap_ids,
            },
        };
        plan.validate_against(self)?;
        Ok(plan)
    }

    pub fn config(&self) -> &TopologicalMapConfig {
        &self.config
    }

    pub fn counts(&self) -> TopologyCounts {
        TopologyCounts {
            concepts: u32::try_from(self.concepts.len()).unwrap_or(u32::MAX),
            edges: u32::try_from(self.edges.len()).unwrap_or(u32::MAX),
            simplexes: u32::try_from(self.simplexes.len()).unwrap_or(u32::MAX),
            unresolved_gaps: u32::try_from(self.unresolved_gaps.len()).unwrap_or(u32::MAX),
        }
    }

    pub const fn next_ids(&self) -> TopologyIdCounters {
        TopologyIdCounters {
            next_concept_id: self.next_concept_id,
            next_edge_id: self.next_edge_id,
            next_simplex_id: self.next_simplex_id,
            next_gap_id: self.next_gap_id,
        }
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
        truncation: TopologyDegradationKind,
        degradations: &mut Vec<TopologyDegradationKind>,
    ) -> Result<ConceptCellId, ScaffoldContractError> {
        if let Some(index) = self.find_concept_index(&signature) {
            let id = self.concepts[index].id;
            if self.concepts[index].observe(bindings, tick, salience)? {
                push_degradation(degradations, truncation);
            }
            return Ok(id);
        }

        if self.concepts.len() >= self.config.max_concepts {
            let index = self
                .concepts
                .iter()
                .enumerate()
                .min_by_key(|(_, concept)| {
                    (
                        q16(concept.salience.raw()),
                        concept.last_tick.raw(),
                        concept.id.raw(),
                    )
                })
                .map(|(index, _)| index)
                .ok_or(ScaffoldContractError::TopologyCapacityExceeded)?;
            let id = self.concepts[index].id;
            let mut summary_bindings = bindings;
            clear_identity_specific_bindings(&mut self.concepts[index].bindings);
            clear_identity_specific_bindings(&mut summary_bindings);
            self.concepts[index].is_summary = true;
            if self.concepts[index].observe(summary_bindings, tick, salience)? {
                push_degradation(degradations, truncation);
            }
            push_degradation(
                degradations,
                TopologyDegradationKind::ConceptMergedIntoSummary,
            );
            return Ok(id);
        }

        let id = ConceptCellId(self.next_concept_id);
        self.next_concept_id = self
            .next_concept_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let mut concept = ConceptCell::new(id, ConceptBindings::default())?;
        if concept.observe(bindings, tick, salience)? {
            push_degradation(degradations, truncation);
        }
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
        degradations: &mut Vec<TopologyDegradationKind>,
    ) -> Result<CognitiveEdgeId, ScaffoldContractError> {
        if let Some(edge) = self
            .edges
            .iter_mut()
            .find(|edge| edge.from == from && edge.to == to && edge.relation == relation)
        {
            return edge.strengthen(EDGE_STRENGTH_INCREMENT.max(salience.raw() * 0.2), tick);
        }

        let id = CognitiveEdgeId(self.next_edge_id);
        self.next_edge_id = self
            .next_edge_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let edge = CognitiveEdge::with_id(id, from, to, relation, salience, tick)?;
        if self.edges.len() >= self.config.max_edges {
            let index = self
                .edges
                .iter()
                .enumerate()
                .min_by_key(|(_, edge)| {
                    (
                        q16(edge.strength.raw()),
                        edge.last_tick.raw(),
                        edge.id.raw(),
                    )
                })
                .map(|(index, _)| index)
                .ok_or(ScaffoldContractError::TopologyCapacityExceeded)?;
            self.edges[index] = edge;
            push_degradation(degradations, TopologyDegradationKind::EdgeEvicted);
        } else {
            self.edges.push(edge);
        }
        Ok(id)
    }

    fn push_simplex(
        &mut self,
        concept_ids: Vec<ConceptCellId>,
        valence: SignedValence,
        prediction_error: NormalizedScalar,
        salience: NormalizedScalar,
        tick: Tick,
        degradations: &mut Vec<TopologyDegradationKind>,
    ) -> Result<CognitiveSimplexId, ScaffoldContractError> {
        let id = CognitiveSimplexId(self.next_simplex_id);
        self.next_simplex_id = self
            .next_simplex_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let simplex =
            CognitiveSimplex::new(id, concept_ids, valence, prediction_error, salience, tick)?;
        if self.simplexes.len() >= self.config.max_simplexes {
            let index = self
                .simplexes
                .iter()
                .enumerate()
                .min_by_key(|(_, simplex)| (simplex.last_tick.raw(), simplex.id.raw()))
                .map(|(index, _)| index)
                .ok_or(ScaffoldContractError::TopologyCapacityExceeded)?;
            self.simplexes[index] = simplex;
            push_degradation(degradations, TopologyDegradationKind::SimplexReplaced);
        } else {
            self.simplexes.push(simplex);
        }
        Ok(id)
    }

    fn detect_or_update_gap(
        &mut self,
        source_concept: ConceptCellId,
        patch: &ExperiencePatch,
        salience: NormalizedScalar,
        tick: Tick,
        degradations: &mut Vec<TopologyDegradationKind>,
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

        let id = UnresolvedGapId(self.next_gap_id);
        self.next_gap_id = self
            .next_gap_id
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
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
        if self.unresolved_gaps.len() >= self.config.max_unresolved_gaps {
            let index = self
                .unresolved_gaps
                .iter()
                .enumerate()
                .min_by_key(|(_, gap)| {
                    (
                        q16(gap.curiosity_voltage.raw()),
                        q16(gap.salience.raw()),
                        gap.last_tick.raw(),
                        gap.id.raw(),
                    )
                })
                .map(|(index, _)| index)
                .ok_or(ScaffoldContractError::TopologyCapacityExceeded)?;
            self.unresolved_gaps[index] = gap;
            push_degradation(degradations, TopologyDegradationKind::GapReplaced);
        } else {
            self.unresolved_gaps.push(gap);
        }
        Ok(vec![id])
    }

    fn edge_decay_per_tick_amount(&self, elapsed_ticks: u64) -> Result<f32, ScaffoldContractError> {
        let elapsed = elapsed_ticks.min(u64::from(u32::MAX)) as f32;
        validate_finite(self.config.edge_decay_per_tick.raw() * elapsed)
    }

    fn canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut builder = CanonicalDigestBuilder::new(TOPOLOGY_MAP_DIGEST_DOMAIN);
        builder.write_u64(
            u64::try_from(self.config.max_concepts)
                .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
        );
        builder.write_u64(
            u64::try_from(self.config.max_edges)
                .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
        );
        builder.write_u64(
            u64::try_from(self.config.max_simplexes)
                .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
        );
        builder.write_u64(
            u64::try_from(self.config.max_unresolved_gaps)
                .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
        );
        builder.write_f32(self.config.edge_decay_per_tick.raw())?;
        builder.write_u64(self.next_concept_id);
        builder.write_u64(self.next_edge_id);
        builder.write_u64(self.next_simplex_id);
        builder.write_u64(self.next_gap_id);
        builder.write_sequence_len(self.concepts.len());
        for concept in &self.concepts {
            encode_concept(&mut builder, concept)?;
        }
        builder.write_sequence_len(self.edges.len());
        for edge in &self.edges {
            encode_edge(&mut builder, edge)?;
        }
        builder.write_sequence_len(self.simplexes.len());
        for simplex in &self.simplexes {
            encode_simplex(&mut builder, simplex)?;
        }
        builder.write_sequence_len(self.unresolved_gaps.len());
        for gap in &self.unresolved_gaps {
            encode_gap(&mut builder, gap)?;
        }
        Ok(builder.finish256())
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
            if self.concept(edge.from).is_none() || self.concept(edge.to).is_none() {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        for simplex in &self.simplexes {
            simplex.validate_contract()?;
            if simplex
                .concept_ids
                .iter()
                .any(|id| self.concept(*id).is_none())
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        for gap in &self.unresolved_gaps {
            gap.validate_contract()?;
            if gap
                .source_concepts
                .iter()
                .any(|id| self.concept(*id).is_none())
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        if self.next_concept_id == 0
            || self.next_edge_id == 0
            || self.next_simplex_id == 0
            || self.next_gap_id == 0
            || self
                .concepts
                .iter()
                .any(|value| value.id.raw() >= self.next_concept_id)
            || self
                .edges
                .iter()
                .any(|value| value.id.raw() >= self.next_edge_id)
            || self
                .simplexes
                .iter()
                .any(|value| value.id.raw() >= self.next_simplex_id)
            || self
                .unresolved_gaps
                .iter()
                .any(|value| value.id.raw() >= self.next_gap_id)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

impl TopologyMutationPlan {
    fn validate_against(&self, map: &TopologicalMap) -> Result<(), ScaffoldContractError> {
        if map.canonical_digest()? != self.expected_digest
            || map.counts() != self.expected_counts
            || map.next_ids() != self.expected_next_ids
            || !self.expected_counts.within(map.config())
            || !self.final_counts.within(map.config())
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        self.primary_bindings.validate_contract()?;
        self.action_bindings.validate_contract()?;
        let _planned_signatures = (&self.primary_signature, &self.action_signature);
        let mut previous_target = None;
        for replacement in &self.replacements {
            let target = replacement_target(replacement);
            if previous_target.is_some_and(|previous| previous >= target) {
                return Err(ScaffoldContractError::InvalidMemoryQuery);
            }
            previous_target = Some(target);
        }
        let mut reconstructed = map.clone();
        apply_replacements_checked(&mut reconstructed, &self.replacements)?;
        reconstructed.assign_next_ids(self.final_next_ids);
        reconstructed.validate_contract()?;
        if reconstructed.counts() != self.final_counts
            || reconstructed.next_ids() != self.final_next_ids
            || reconstructed.canonical_digest()? != self.final_digest
            || reconstructed
                .concept(self.update.primary_concept_id)
                .is_none()
            || self
                .update
                .edge_ids
                .iter()
                .any(|id| reconstructed.edge(*id).is_none())
            || !reconstructed
                .simplexes
                .iter()
                .any(|simplex| simplex.id == self.update.simplex_id)
            || self
                .update
                .gap_ids
                .iter()
                .any(|id| reconstructed.gap(*id).is_none())
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }
}

impl TopologySidecar {
    pub fn new(
        organism_id: OrganismId,
        config: TopologicalMapConfig,
    ) -> Result<Self, ScaffoldContractError> {
        Self::new_profiled(
            organism_id,
            crate::SensorProfileIdentity {
                profile_id: crate::SensorProfile::PrivilegedAffordanceV1.into(),
                profile_schema_version: 1,
                sensory_abi_version: crate::SensoryAbiVersion::CURRENT.raw(),
            },
            config,
        )
    }

    pub fn new_profiled(
        organism_id: OrganismId,
        profile: crate::SensorProfileIdentity,
        config: TopologicalMapConfig,
    ) -> Result<Self, ScaffoldContractError> {
        organism_id.validate()?;
        profile.validate_contract()?;
        let map = TopologicalMap::new(config)?;
        let canonical_digest = map.canonical_digest()?;
        Ok(Self {
            organism_id,
            profile,
            map,
            diagnostics: TopologySidecarDiagnostics {
                organism_id_raw: organism_id.raw(),
                canonical_digest,
                ..TopologySidecarDiagnostics::default()
            },
            last_observed_sequence_id: None,
            last_observed_key_digest: None,
        })
    }

    pub fn observe_sealed_patch(&mut self, patch: &ExperiencePatch) -> TopologyObservationReceipt {
        self.observe_patch(patch, true)
    }

    pub(crate) fn observe_legacy_patch(
        &mut self,
        patch: &ExperiencePatch,
    ) -> TopologyObservationReceipt {
        self.observe_patch(patch, false)
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn profile(&self) -> crate::SensorProfileIdentity {
        self.profile
    }

    pub fn config(&self) -> &TopologicalMapConfig {
        self.map.config()
    }

    pub fn counts(&self) -> TopologyCounts {
        self.map.counts()
    }

    pub const fn next_ids(&self) -> TopologyIdCounters {
        self.map.next_ids()
    }

    pub const fn diagnostics(&self) -> TopologySidecarDiagnostics {
        self.diagnostics
    }

    pub const fn map(&self) -> &TopologicalMap {
        &self.map
    }

    pub(crate) fn map_mut(&mut self) -> &mut TopologicalMap {
        &mut self.map
    }

    pub fn decay_edges(&mut self, elapsed_ticks: u64) -> Result<(), ScaffoldContractError> {
        self.map.decay_edges(elapsed_ticks)?;
        self.refresh_diagnostics_after_map_mutation()
    }

    pub(crate) fn refresh_diagnostics_after_map_mutation(
        &mut self,
    ) -> Result<(), ScaffoldContractError> {
        self.map.validate_contract()?;
        self.diagnostics.canonical_digest = self.map.canonical_digest()?;
        Ok(())
    }

    fn observe_patch(
        &mut self,
        patch: &ExperiencePatch,
        require_episodic_key: bool,
    ) -> TopologyObservationReceipt {
        let sequence = patch.header().sequence_id;
        let before_counts = self.map.counts();
        let before_next_ids = self.map.next_ids();
        let before_digest = self.diagnostics.canonical_digest;
        let validation = patch.validate_contract().and_then(|()| {
            if patch.header().organism_id != self.organism_id {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            if patch.header().sensor_profile.identity() != self.profile {
                return Err(ScaffoldContractError::SensorProfileMismatch);
            }
            match patch.decision().episodic_key() {
                Some(key) => {
                    key.validate_contract()?;
                    if key.query().organism_id() != self.organism_id {
                        return Err(ScaffoldContractError::BrainOwnershipMismatch);
                    }
                    Ok(())
                }
                None if require_episodic_key => Err(ScaffoldContractError::InvalidMemoryQuery),
                None => Ok(()),
            }
        });
        if validation.is_err() {
            self.diagnostics.invalid_rejections =
                self.diagnostics.invalid_rejections.saturating_add(1);
            return rejected_topology_receipt(
                self.organism_id,
                sequence,
                before_counts,
                before_next_ids,
                before_digest,
                TopologyDegradationKind::InvalidObservationRejected,
                false,
            );
        }

        let key_digest = patch
            .decision()
            .episodic_key()
            .map_or([0; 4], |key| key.canonical_digest());
        if self
            .last_observed_sequence_id
            .is_some_and(|last| sequence.raw() <= last.raw())
        {
            self.diagnostics.replay_rejections =
                self.diagnostics.replay_rejections.saturating_add(1);
            return rejected_topology_receipt(
                self.organism_id,
                sequence,
                before_counts,
                before_next_ids,
                before_digest,
                TopologyDegradationKind::ReplayRejected,
                true,
            );
        }

        let plan = match self.map.plan_observation(patch, require_episodic_key) {
            Ok(plan) => plan,
            Err(_) => {
                self.diagnostics.invalid_rejections =
                    self.diagnostics.invalid_rejections.saturating_add(1);
                return rejected_topology_receipt(
                    self.organism_id,
                    sequence,
                    before_counts,
                    before_next_ids,
                    before_digest,
                    TopologyDegradationKind::InvalidObservationRejected,
                    false,
                );
            }
        };
        let receipt_degradations = plan.degradations.clone();
        let update = plan.update.clone();
        let after_counts = plan.final_counts;
        let after_next_ids = plan.final_next_ids;
        let after_digest = plan.final_digest;
        commit_prevalidated_plan(&mut self.map, plan);
        self.last_observed_sequence_id = Some(sequence);
        self.last_observed_key_digest = Some(key_digest);
        self.diagnostics.observations = self.diagnostics.observations.saturating_add(1);
        self.diagnostics.degradations = self
            .diagnostics
            .degradations
            .saturating_add(u64::try_from(receipt_degradations.len()).unwrap_or(u64::MAX));
        self.diagnostics.canonical_digest = after_digest;
        TopologyObservationReceipt {
            organism_id_raw: self.organism_id.raw(),
            sealed_sequence_id: sequence,
            update: Some(update),
            degradations: receipt_degradations,
            before_counts,
            after_counts,
            before_next_ids,
            after_next_ids,
            before_digest,
            after_digest,
            rejected_invalid: false,
            replay_rejected: false,
        }
    }

    pub fn export_portable(&self) -> Result<PortableTopologySidecarAssetV1, ScaffoldContractError> {
        self.validate_contract()?;
        let config = self.map.config;
        let mut asset = PortableTopologySidecarAssetV1 {
            schema_version: PORTABLE_TOPOLOGY_SIDECAR_SCHEMA_VERSION,
            organism_id_raw: self.organism_id.raw(),
            profile: self.profile,
            max_concepts: u32::try_from(config.max_concepts)
                .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
            max_edges: u32::try_from(config.max_edges)
                .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
            max_simplexes: u32::try_from(config.max_simplexes)
                .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
            max_unresolved_gaps: u32::try_from(config.max_unresolved_gaps)
                .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
            max_bindings_per_kind: MAX_BINDING_REFS as u32,
            edge_decay_bits: portable_topology_bits(config.edge_decay_per_tick.raw()),
            last_observed_sequence_id_raw: self
                .last_observed_sequence_id
                .map_or(0, ExperienceSequenceId::raw),
            last_observed_key_digest: self.last_observed_key_digest.unwrap_or([0; 4]),
            next_concept_id_raw: self.map.next_concept_id,
            next_edge_id_raw: self.map.next_edge_id,
            next_simplex_id_raw: self.map.next_simplex_id,
            next_gap_id_raw: self.map.next_gap_id,
            concepts: self.map.concepts.iter().map(portable_concept).collect(),
            edges: self.map.edges.iter().map(portable_edge).collect(),
            simplexes: self.map.simplexes.iter().map(portable_simplex).collect(),
            gaps: self.map.unresolved_gaps.iter().map(portable_gap).collect(),
            observation_count: self.diagnostics.observations,
            degradation_count: self.diagnostics.degradations,
            invalid_rejection_count: self.diagnostics.invalid_rejections,
            replay_rejection_count: self.diagnostics.replay_rejections,
            map_digest: self.diagnostics.canonical_digest,
            canonical_digest: [0; 4],
        };
        asset.canonical_digest = asset.recompute_canonical_digest()?;
        asset.validate_contract()?;
        Ok(asset)
    }

    pub fn restore_portable(
        asset: PortableTopologySidecarAssetV1,
    ) -> Result<Self, ScaffoldContractError> {
        asset
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        let map = topological_map_from_portable(&asset)?;
        let organism_id = OrganismId(asset.organism_id_raw);
        let last_observed_sequence_id = (asset.last_observed_sequence_id_raw != 0)
            .then_some(ExperienceSequenceId(asset.last_observed_sequence_id_raw));
        let last_observed_key_digest =
            (asset.last_observed_sequence_id_raw != 0).then_some(asset.last_observed_key_digest);
        let restored = Self {
            organism_id,
            profile: asset.profile,
            map,
            diagnostics: TopologySidecarDiagnostics {
                organism_id_raw: asset.organism_id_raw,
                observations: asset.observation_count,
                degradations: asset.degradation_count,
                invalid_rejections: asset.invalid_rejection_count,
                replay_rejections: asset.replay_rejection_count,
                terminal_errors: 0,
                canonical_digest: asset.map_digest,
            },
            last_observed_sequence_id,
            last_observed_key_digest,
        };
        restored.validate_contract()?;
        Ok(restored)
    }
}

impl std::ops::Deref for TopologySidecar {
    type Target = TopologicalMap;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl Validate for TopologySidecar {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.profile.validate_contract()?;
        self.map.validate_contract()?;
        if self.diagnostics.organism_id_raw != self.organism_id.raw()
            || self.diagnostics.canonical_digest != self.map.canonical_digest()?
            || self.diagnostics.terminal_errors != 0
            || self.last_observed_sequence_id.is_some() != self.last_observed_key_digest.is_some()
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        if let Some(sequence) = self.last_observed_sequence_id {
            sequence.validate()?;
        }
        Ok(())
    }
}

impl PortableTopologySidecarAssetV1 {
    pub fn recompute_canonical_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(PORTABLE_TOPOLOGY_SIDECAR_DIGEST_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u64(self.organism_id_raw);
        digest.write_u16(self.profile.profile_id.raw());
        digest.write_u16(self.profile.profile_schema_version);
        digest.write_u16(self.profile.sensory_abi_version);
        digest.write_u32(self.max_concepts);
        digest.write_u32(self.max_edges);
        digest.write_u32(self.max_simplexes);
        digest.write_u32(self.max_unresolved_gaps);
        digest.write_u32(self.max_bindings_per_kind);
        digest.write_f32(portable_topology_float(self.edge_decay_bits)?)?;
        digest.write_u64(self.last_observed_sequence_id_raw);
        for word in self.last_observed_key_digest {
            digest.write_u64(word);
        }
        digest.write_u64(self.next_concept_id_raw);
        digest.write_u64(self.next_edge_id_raw);
        digest.write_u64(self.next_simplex_id_raw);
        digest.write_u64(self.next_gap_id_raw);
        digest.write_u64(self.observation_count);
        digest.write_u64(self.degradation_count);
        digest.write_u64(self.invalid_rejection_count);
        digest.write_u64(self.replay_rejection_count);
        for word in self.map_digest {
            digest.write_u64(word);
        }
        Ok(digest.finish256())
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.profile
            .validate_contract()
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
        OrganismId(self.organism_id_raw).validate()?;
        let config = portable_topology_config(self)?;
        if self.schema_version != PORTABLE_TOPOLOGY_SIDECAR_SCHEMA_VERSION
            || self.max_bindings_per_kind != MAX_BINDING_REFS as u32
            || self.concepts.len() > config.max_concepts
            || self.edges.len() > config.max_edges
            || self.simplexes.len() > config.max_simplexes
            || self.gaps.len() > config.max_unresolved_gaps
            || (self.last_observed_sequence_id_raw == 0)
                != (self.last_observed_key_digest == [0; 4])
            || (self.observation_count == 0 && self.last_observed_sequence_id_raw != 0)
            || (self.observation_count != 0 && self.last_observed_sequence_id_raw == 0)
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        if self.last_observed_sequence_id_raw != 0 {
            ExperienceSequenceId(self.last_observed_sequence_id_raw).validate()?;
        }

        let map = topological_map_from_portable(self)?;
        if map.canonical_digest()? != self.map_digest
            || self.canonical_digest != self.recompute_canonical_digest()?
        {
            return Err(ScaffoldContractError::InvalidMemoryQuery);
        }
        Ok(())
    }
}

fn portable_concept(concept: &ConceptCell) -> PortableTopologyConceptV1 {
    PortableTopologyConceptV1 {
        id_raw: concept.id.raw(),
        is_summary: concept.is_summary,
        bindings: portable_bindings(&concept.bindings),
        observation_count: concept.observation_count,
        first_tick_raw: concept.first_tick.raw(),
        last_tick_raw: concept.last_tick.raw(),
        confidence_bits: portable_topology_bits(concept.confidence.raw()),
        salience_bits: portable_topology_bits(concept.salience.raw()),
    }
}

fn portable_bindings(bindings: &ConceptBindings) -> PortableTopologyBindingSetV1 {
    PortableTopologyBindingSetV1 {
        tracked_object_ids_raw: bindings.objects.iter().map(|value| value.raw()).collect(),
        word_ids_raw: bindings.words.clone(),
        drives: bindings
            .drives
            .iter()
            .map(|value| PortableTopologyDriveBindingV1 {
                channel_raw: drive_channel_raw(value.channel),
                value_bits: portable_topology_bits(value.value),
            })
            .collect(),
        actions: bindings
            .actions
            .iter()
            .map(|value| PortableTopologyActionBindingV1 {
                action_id_raw: value.action_id.raw(),
                action_kind_raw: value.kind.raw(),
                confidence_bits: portable_topology_bits(value.confidence.raw()),
            })
            .collect(),
        action_families_raw: bindings
            .action_families
            .iter()
            .map(|value| value.raw())
            .collect(),
        location_bits: bindings
            .locations
            .iter()
            .map(|value| {
                [
                    portable_topology_bits(value.x),
                    portable_topology_bits(value.y),
                    portable_topology_bits(value.z),
                ]
            })
            .collect(),
        agent_ids_raw: bindings.agents.iter().map(|value| value.raw()).collect(),
        semantic_concept_ids_raw: bindings
            .semantic_refs
            .iter()
            .map(|value| value.raw())
            .collect(),
        cluster_ids_raw: bindings
            .cluster_refs
            .iter()
            .map(|value| value.raw())
            .collect(),
        affordance_bits_raw: bindings.affordances.raw(),
        mean_valence_bits: portable_topology_bits(bindings.emotions.mean_valence.raw()),
        mean_prediction_error_bits: portable_topology_bits(
            bindings.emotions.mean_prediction_error.raw(),
        ),
        emotion_observation_count: bindings.emotions.observation_count,
    }
}

fn portable_edge(edge: &CognitiveEdge) -> PortableTopologyEdgeV1 {
    PortableTopologyEdgeV1 {
        id_raw: edge.id.raw(),
        from_raw: edge.from.raw(),
        to_raw: edge.to.raw(),
        relation_raw: edge_relation_raw(edge.relation),
        strength_bits: portable_topology_bits(edge.strength.raw()),
        evidence_count: edge.evidence_count,
        first_tick_raw: edge.first_tick.raw(),
        last_tick_raw: edge.last_tick.raw(),
        confidence_bits: portable_topology_bits(edge.confidence.raw()),
    }
}

fn portable_simplex(simplex: &CognitiveSimplex) -> PortableTopologySimplexV1 {
    PortableTopologySimplexV1 {
        id_raw: simplex.id.raw(),
        concept_ids_raw: simplex
            .concept_ids
            .iter()
            .map(|value| value.raw())
            .collect(),
        observation_count: simplex.observation_count,
        mean_valence_bits: portable_topology_bits(simplex.mean_valence.raw()),
        mean_prediction_error_bits: portable_topology_bits(simplex.mean_prediction_error.raw()),
        salience_bits: portable_topology_bits(simplex.salience.raw()),
        first_tick_raw: simplex.first_tick.raw(),
        last_tick_raw: simplex.last_tick.raw(),
    }
}

fn portable_gap(gap: &UnresolvedGap) -> PortableTopologyGapV1 {
    PortableTopologyGapV1 {
        id_raw: gap.id.raw(),
        source_concept_ids_raw: gap
            .source_concepts
            .iter()
            .map(|value| value.raw())
            .collect(),
        contradiction_raw: contradiction_raw(gap.contradiction_type),
        prediction_error_bits: portable_topology_bits(gap.prediction_error.raw()),
        curiosity_voltage_bits: portable_topology_bits(gap.curiosity_voltage.raw()),
        salience_bits: portable_topology_bits(gap.salience.raw()),
        first_tick_raw: gap.first_tick.raw(),
        last_tick_raw: gap.last_tick.raw(),
        confidence_bits: portable_topology_bits(gap.confidence.raw()),
        status_raw: gap_status_raw(gap.status),
    }
}

fn topological_map_from_portable(
    asset: &PortableTopologySidecarAssetV1,
) -> Result<TopologicalMap, ScaffoldContractError> {
    let config = portable_topology_config(asset)?;
    let concepts = asset
        .concepts
        .iter()
        .map(domain_concept)
        .collect::<Result<Vec<_>, _>>()?;
    let edges = asset
        .edges
        .iter()
        .map(domain_edge)
        .collect::<Result<Vec<_>, _>>()?;
    let simplexes = asset
        .simplexes
        .iter()
        .map(domain_simplex)
        .collect::<Result<Vec<_>, _>>()?;
    let unresolved_gaps = asset
        .gaps
        .iter()
        .map(domain_gap)
        .collect::<Result<Vec<_>, _>>()?;
    ensure_unique_raw(concepts.iter().map(|value| value.id.raw()))?;
    ensure_unique_raw(edges.iter().map(|value| value.id.raw()))?;
    ensure_unique_raw(simplexes.iter().map(|value| value.id.raw()))?;
    ensure_unique_raw(unresolved_gaps.iter().map(|value| value.id.raw()))?;
    let map = TopologicalMap {
        config,
        concepts,
        edges,
        simplexes,
        unresolved_gaps,
        next_concept_id: asset.next_concept_id_raw,
        next_edge_id: asset.next_edge_id_raw,
        next_simplex_id: asset.next_simplex_id_raw,
        next_gap_id: asset.next_gap_id_raw,
    };
    map.validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    Ok(map)
}

fn domain_concept(value: &PortableTopologyConceptV1) -> Result<ConceptCell, ScaffoldContractError> {
    let concept = ConceptCell {
        id: ConceptCellId(value.id_raw),
        bindings: domain_bindings(&value.bindings)?,
        is_summary: value.is_summary,
        observation_count: value.observation_count,
        first_tick: Tick::new(value.first_tick_raw),
        last_tick: Tick::new(value.last_tick_raw),
        confidence: Confidence::new(portable_topology_float(value.confidence_bits)?)?,
        salience: NormalizedScalar::new(portable_topology_float(value.salience_bits)?)?,
    };
    concept.validate_contract()?;
    Ok(concept)
}

fn domain_bindings(
    value: &PortableTopologyBindingSetV1,
) -> Result<ConceptBindings, ScaffoldContractError> {
    if value.affordance_bits_raw & !0x03ff != 0 {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    ensure_unique_raw(value.tracked_object_ids_raw.iter().copied())?;
    ensure_unique_raw(value.word_ids_raw.iter().copied().map(u64::from))?;
    ensure_unique_raw(value.agent_ids_raw.iter().copied())?;
    ensure_unique_raw(value.semantic_concept_ids_raw.iter().copied())?;
    ensure_unique_raw(value.cluster_ids_raw.iter().copied())?;
    let bindings = ConceptBindings {
        objects: value
            .tracked_object_ids_raw
            .iter()
            .copied()
            .map(TrackedObjectId)
            .collect(),
        words: value.word_ids_raw.clone(),
        drives: value
            .drives
            .iter()
            .map(|drive| {
                Ok(DriveBinding {
                    channel: drive_channel_from_raw(drive.channel_raw)?,
                    value: portable_topology_float(drive.value_bits)?,
                })
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?,
        actions: value
            .actions
            .iter()
            .map(|action| {
                Ok(ActionObservationFact {
                    action_id: ActionId(action.action_id_raw),
                    kind: ActionKind::try_from_raw(action.action_kind_raw)?,
                    confidence: Confidence::new(portable_topology_float(action.confidence_bits)?)?,
                })
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?,
        action_families: value
            .action_families_raw
            .iter()
            .copied()
            .map(CandidateActionFamily::try_from_raw)
            .collect::<Result<Vec<_>, _>>()?,
        emotions: EmotionValenceSummary {
            mean_valence: SignedValence::new(portable_topology_float(value.mean_valence_bits)?)?,
            mean_prediction_error: NormalizedScalar::new(portable_topology_float(
                value.mean_prediction_error_bits,
            )?)?,
            observation_count: value.emotion_observation_count,
        },
        locations: value
            .location_bits
            .iter()
            .map(|bits| {
                Ok(Vec3f::new(
                    portable_topology_float(bits[0])?,
                    portable_topology_float(bits[1])?,
                    portable_topology_float(bits[2])?,
                ))
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?,
        agents: value
            .agent_ids_raw
            .iter()
            .copied()
            .map(OrganismId)
            .collect(),
        affordances: AffordanceBits(value.affordance_bits_raw),
        semantic_refs: value
            .semantic_concept_ids_raw
            .iter()
            .copied()
            .map(ConceptCellId)
            .collect(),
        cluster_refs: value
            .cluster_ids_raw
            .iter()
            .copied()
            .map(GaussianClusterId)
            .collect(),
    };
    bindings.validate_contract()?;
    Ok(bindings)
}

fn domain_edge(value: &PortableTopologyEdgeV1) -> Result<CognitiveEdge, ScaffoldContractError> {
    let edge = CognitiveEdge {
        id: CognitiveEdgeId(value.id_raw),
        from: ConceptCellId(value.from_raw),
        to: ConceptCellId(value.to_raw),
        relation: edge_relation_from_raw(value.relation_raw)?,
        strength: NormalizedScalar::new(portable_topology_float(value.strength_bits)?)?,
        evidence_count: value.evidence_count,
        first_tick: Tick::new(value.first_tick_raw),
        last_tick: Tick::new(value.last_tick_raw),
        confidence: Confidence::new(portable_topology_float(value.confidence_bits)?)?,
    };
    edge.validate_contract()?;
    Ok(edge)
}

fn domain_simplex(
    value: &PortableTopologySimplexV1,
) -> Result<CognitiveSimplex, ScaffoldContractError> {
    ensure_unique_raw(value.concept_ids_raw.iter().copied())?;
    let simplex = CognitiveSimplex {
        id: CognitiveSimplexId(value.id_raw),
        concept_ids: value
            .concept_ids_raw
            .iter()
            .copied()
            .map(ConceptCellId)
            .collect(),
        observation_count: value.observation_count,
        mean_valence: SignedValence::new(portable_topology_float(value.mean_valence_bits)?)?,
        mean_prediction_error: NormalizedScalar::new(portable_topology_float(
            value.mean_prediction_error_bits,
        )?)?,
        salience: NormalizedScalar::new(portable_topology_float(value.salience_bits)?)?,
        first_tick: Tick::new(value.first_tick_raw),
        last_tick: Tick::new(value.last_tick_raw),
    };
    simplex.validate_contract()?;
    Ok(simplex)
}

fn domain_gap(value: &PortableTopologyGapV1) -> Result<UnresolvedGap, ScaffoldContractError> {
    ensure_unique_raw(value.source_concept_ids_raw.iter().copied())?;
    let gap = UnresolvedGap {
        id: UnresolvedGapId(value.id_raw),
        source_concepts: value
            .source_concept_ids_raw
            .iter()
            .copied()
            .map(ConceptCellId)
            .collect(),
        contradiction_type: contradiction_from_raw(value.contradiction_raw)?,
        prediction_error: NormalizedScalar::new(portable_topology_float(
            value.prediction_error_bits,
        )?)?,
        curiosity_voltage: NormalizedScalar::new(portable_topology_float(
            value.curiosity_voltage_bits,
        )?)?,
        salience: NormalizedScalar::new(portable_topology_float(value.salience_bits)?)?,
        first_tick: Tick::new(value.first_tick_raw),
        last_tick: Tick::new(value.last_tick_raw),
        confidence: Confidence::new(portable_topology_float(value.confidence_bits)?)?,
        status: gap_status_from_raw(value.status_raw)?,
    };
    gap.validate_contract()?;
    Ok(gap)
}

fn portable_topology_config(
    asset: &PortableTopologySidecarAssetV1,
) -> Result<TopologicalMapConfig, ScaffoldContractError> {
    let config = TopologicalMapConfig {
        max_concepts: usize::try_from(asset.max_concepts)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        max_edges: usize::try_from(asset.max_edges)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        max_simplexes: usize::try_from(asset.max_simplexes)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        max_unresolved_gaps: usize::try_from(asset.max_unresolved_gaps)
            .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?,
        edge_decay_per_tick: NormalizedScalar::new(portable_topology_float(
            asset.edge_decay_bits,
        )?)?,
    };
    config
        .validate_contract()
        .map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    Ok(config)
}

fn ensure_unique_raw<I>(values: I) -> Result<(), ScaffoldContractError>
where
    I: IntoIterator<Item = u64>,
{
    let mut seen = std::collections::BTreeSet::new();
    if values.into_iter().all(|value| seen.insert(value)) {
        Ok(())
    } else {
        Err(ScaffoldContractError::InvalidMemoryQuery)
    }
}

fn portable_topology_float(bits: u32) -> Result<f32, ScaffoldContractError> {
    let value = f32::from_bits(bits);
    if value.is_finite() && bits != (-0.0_f32).to_bits() {
        Ok(value)
    } else {
        Err(ScaffoldContractError::InvalidMemoryQuery)
    }
}

fn portable_topology_bits(value: f32) -> u32 {
    if value == 0.0 {
        0.0_f32.to_bits()
    } else {
        value.to_bits()
    }
}

fn drive_channel_raw(value: DriveChannel) -> u8 {
    match value {
        DriveChannel::Hunger => 1,
        DriveChannel::Fatigue => 2,
        DriveChannel::Fear => 3,
        DriveChannel::Pain => 4,
        DriveChannel::Loneliness => 5,
        DriveChannel::Curiosity => 6,
        DriveChannel::BrainAtp => 7,
        DriveChannel::TemperatureStress => 8,
        DriveChannel::ReproductiveDrive => 9,
        DriveChannel::Extension0 => 10,
        DriveChannel::Extension1 => 11,
    }
}

fn drive_channel_from_raw(value: u8) -> Result<DriveChannel, ScaffoldContractError> {
    match value {
        1 => Ok(DriveChannel::Hunger),
        2 => Ok(DriveChannel::Fatigue),
        3 => Ok(DriveChannel::Fear),
        4 => Ok(DriveChannel::Pain),
        5 => Ok(DriveChannel::Loneliness),
        6 => Ok(DriveChannel::Curiosity),
        7 => Ok(DriveChannel::BrainAtp),
        8 => Ok(DriveChannel::TemperatureStress),
        9 => Ok(DriveChannel::ReproductiveDrive),
        10 => Ok(DriveChannel::Extension0),
        11 => Ok(DriveChannel::Extension1),
        _ => Err(ScaffoldContractError::InvalidMemoryQuery),
    }
}

fn edge_relation_raw(value: EdgeRelationKind) -> u16 {
    match value {
        EdgeRelationKind::Predicts => 1,
        EdgeRelationKind::Causes => 2,
        EdgeRelationKind::SatisfiesDrive => 3,
        EdgeRelationKind::BelongsTo => 4,
        EdgeRelationKind::SociallyLiked => 5,
        EdgeRelationKind::SociallyFeared => 6,
        EdgeRelationKind::Contradicts => 7,
        EdgeRelationKind::CoOccurs => 8,
        EdgeRelationKind::Enables => 9,
        EdgeRelationKind::Blocks => 10,
        EdgeRelationKind::TeacherLabels => 11,
    }
}

fn edge_relation_from_raw(value: u16) -> Result<EdgeRelationKind, ScaffoldContractError> {
    match value {
        1 => Ok(EdgeRelationKind::Predicts),
        2 => Ok(EdgeRelationKind::Causes),
        3 => Ok(EdgeRelationKind::SatisfiesDrive),
        4 => Ok(EdgeRelationKind::BelongsTo),
        5 => Ok(EdgeRelationKind::SociallyLiked),
        6 => Ok(EdgeRelationKind::SociallyFeared),
        7 => Ok(EdgeRelationKind::Contradicts),
        8 => Ok(EdgeRelationKind::CoOccurs),
        9 => Ok(EdgeRelationKind::Enables),
        10 => Ok(EdgeRelationKind::Blocks),
        11 => Ok(EdgeRelationKind::TeacherLabels),
        _ => Err(ScaffoldContractError::InvalidMemoryQuery),
    }
}

fn contradiction_raw(value: ContradictionType) -> u16 {
    match value {
        ContradictionType::OutcomeContradiction => 1,
        ContradictionType::PredictionError => 2,
        ContradictionType::TeacherLabelConflict => 3,
        ContradictionType::SocialValenceConflict => 4,
    }
}

fn contradiction_from_raw(value: u16) -> Result<ContradictionType, ScaffoldContractError> {
    match value {
        1 => Ok(ContradictionType::OutcomeContradiction),
        2 => Ok(ContradictionType::PredictionError),
        3 => Ok(ContradictionType::TeacherLabelConflict),
        4 => Ok(ContradictionType::SocialValenceConflict),
        _ => Err(ScaffoldContractError::InvalidMemoryQuery),
    }
}

fn gap_status_raw(value: GapResolutionStatus) -> u16 {
    match value {
        GapResolutionStatus::Open => 1,
        GapResolutionStatus::BiasingCuriosity => 2,
        GapResolutionStatus::Resolved => 3,
        GapResolutionStatus::Dismissed => 4,
    }
}

fn gap_status_from_raw(value: u16) -> Result<GapResolutionStatus, ScaffoldContractError> {
    match value {
        1 => Ok(GapResolutionStatus::Open),
        2 => Ok(GapResolutionStatus::BiasingCuriosity),
        3 => Ok(GapResolutionStatus::Resolved),
        4 => Ok(GapResolutionStatus::Dismissed),
        _ => Err(ScaffoldContractError::InvalidMemoryQuery),
    }
}

fn diff_replacements(
    before: &TopologicalMap,
    after: &TopologicalMap,
) -> Result<Vec<TopologyReplacement>, ScaffoldContractError> {
    if after.concepts.len() < before.concepts.len()
        || after.edges.len() < before.edges.len()
        || after.simplexes.len() < before.simplexes.len()
        || after.unresolved_gaps.len() < before.unresolved_gaps.len()
    {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    let mut replacements = Vec::new();
    for (index, value) in after.concepts.iter().enumerate() {
        if before.concepts.get(index) != Some(value) {
            replacements.push(TopologyReplacement::Concept {
                index: u32::try_from(index)
                    .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
                expected_id: before.concepts.get(index).map(|current| current.id),
                value: value.clone(),
            });
        }
    }
    for (index, value) in after.edges.iter().enumerate() {
        if before.edges.get(index) != Some(value) {
            replacements.push(TopologyReplacement::Edge {
                index: u32::try_from(index)
                    .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
                expected_id: before.edges.get(index).map(|current| current.id),
                value: value.clone(),
            });
        }
    }
    for (index, value) in after.simplexes.iter().enumerate() {
        if before.simplexes.get(index) != Some(value) {
            replacements.push(TopologyReplacement::Simplex {
                index: u32::try_from(index)
                    .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
                expected_id: before.simplexes.get(index).map(|current| current.id),
                value: value.clone(),
            });
        }
    }
    for (index, value) in after.unresolved_gaps.iter().enumerate() {
        if before.unresolved_gaps.get(index) != Some(value) {
            replacements.push(TopologyReplacement::Gap {
                index: u32::try_from(index)
                    .map_err(|_| ScaffoldContractError::TopologyCapacityExceeded)?,
                expected_id: before.unresolved_gaps.get(index).map(|current| current.id),
                value: value.clone(),
            });
        }
    }
    Ok(replacements)
}

fn replacement_target(replacement: &TopologyReplacement) -> (u8, u32) {
    match replacement {
        TopologyReplacement::Concept { index, .. } => (0, *index),
        TopologyReplacement::Edge { index, .. } => (1, *index),
        TopologyReplacement::Simplex { index, .. } => (2, *index),
        TopologyReplacement::Gap { index, .. } => (3, *index),
    }
}

fn apply_replacements_checked(
    map: &mut TopologicalMap,
    replacements: &[TopologyReplacement],
) -> Result<(), ScaffoldContractError> {
    for replacement in replacements {
        match replacement {
            TopologyReplacement::Concept {
                index,
                expected_id,
                value,
            } => replace_or_append_checked(
                &mut map.concepts,
                *index,
                *expected_id,
                value.clone(),
                |current| current.id,
            )?,
            TopologyReplacement::Edge {
                index,
                expected_id,
                value,
            } => replace_or_append_checked(
                &mut map.edges,
                *index,
                *expected_id,
                value.clone(),
                |current| current.id,
            )?,
            TopologyReplacement::Simplex {
                index,
                expected_id,
                value,
            } => replace_or_append_checked(
                &mut map.simplexes,
                *index,
                *expected_id,
                value.clone(),
                |current| current.id,
            )?,
            TopologyReplacement::Gap {
                index,
                expected_id,
                value,
            } => replace_or_append_checked(
                &mut map.unresolved_gaps,
                *index,
                *expected_id,
                value.clone(),
                |current| current.id,
            )?,
        }
    }
    Ok(())
}

fn replace_or_append_checked<T, I: Copy + PartialEq>(
    values: &mut Vec<T>,
    index: u32,
    expected_id: Option<I>,
    value: T,
    id: impl Fn(&T) -> I,
) -> Result<(), ScaffoldContractError> {
    let index = usize::try_from(index).map_err(|_| ScaffoldContractError::InvalidMemoryQuery)?;
    match (index.cmp(&values.len()), expected_id) {
        (std::cmp::Ordering::Equal, None) => values.push(value),
        (std::cmp::Ordering::Less, Some(expected)) if id(&values[index]) == expected => {
            values[index] = value;
        }
        _ => return Err(ScaffoldContractError::InvalidMemoryQuery),
    }
    Ok(())
}

fn commit_prevalidated_plan(map: &mut TopologicalMap, plan: TopologyMutationPlan) {
    plan.validate_against(map)
        .expect("private topology mutation plan was validated before commit");
    apply_replacements_checked(map, &plan.replacements)
        .expect("prevalidated topology replacements have exact targets");
    map.assign_next_ids(plan.final_next_ids);
    debug_assert_eq!(map.counts(), plan.final_counts);
    debug_assert_eq!(map.canonical_digest().ok(), Some(plan.final_digest));
}

impl TopologicalMap {
    const fn assign_next_ids(&mut self, counters: TopologyIdCounters) {
        self.next_concept_id = counters.next_concept_id;
        self.next_edge_id = counters.next_edge_id;
        self.next_simplex_id = counters.next_simplex_id;
        self.next_gap_id = counters.next_gap_id;
    }
}

fn rejected_topology_receipt(
    organism_id: OrganismId,
    sequence: ExperienceSequenceId,
    counts: TopologyCounts,
    next_ids: TopologyIdCounters,
    digest: [u64; 4],
    degradation: TopologyDegradationKind,
    replay_rejected: bool,
) -> TopologyObservationReceipt {
    TopologyObservationReceipt {
        organism_id_raw: organism_id.raw(),
        sealed_sequence_id: sequence,
        update: None,
        degradations: vec![degradation],
        before_counts: counts,
        after_counts: counts,
        before_next_ids: next_ids,
        after_next_ids: next_ids,
        before_digest: digest,
        after_digest: digest,
        rejected_invalid: !replay_rejected,
        replay_rejected,
    }
}

fn clear_identity_specific_bindings(bindings: &mut ConceptBindings) {
    bindings.objects.clear();
    bindings.words.clear();
    bindings.locations.clear();
    bindings.agents.clear();
    bindings.semantic_refs.clear();
    bindings.cluster_refs.clear();
}

fn push_degradation(
    degradations: &mut Vec<TopologyDegradationKind>,
    degradation: TopologyDegradationKind,
) {
    if !degradations.contains(&degradation) {
        degradations.push(degradation);
    }
}

fn q16(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16
}

fn encode_concept(
    builder: &mut CanonicalDigestBuilder,
    concept: &ConceptCell,
) -> Result<(), ScaffoldContractError> {
    builder.write_u64(concept.id.raw());
    builder.write_bool(concept.is_summary);
    builder.write_u32(concept.observation_count);
    builder.write_u64(concept.first_tick.raw());
    builder.write_u64(concept.last_tick.raw());
    builder.write_f32(concept.confidence.raw())?;
    builder.write_f32(concept.salience.raw())?;
    encode_bindings(builder, &concept.bindings)
}

fn encode_bindings(
    builder: &mut CanonicalDigestBuilder,
    bindings: &ConceptBindings,
) -> Result<(), ScaffoldContractError> {
    builder.write_sequence_len(bindings.objects.len());
    for value in &bindings.objects {
        builder.write_u64(value.raw());
    }
    builder.write_sequence_len(bindings.words.len());
    for value in &bindings.words {
        builder.write_u32(*value);
    }
    builder.write_sequence_len(bindings.drives.len());
    for value in &bindings.drives {
        builder.write_u8(value.channel as u8);
        builder.write_f32(value.value)?;
    }
    builder.write_sequence_len(bindings.actions.len());
    for value in &bindings.actions {
        builder.write_u32(value.action_id.raw());
        builder.write_u8(value.kind as u8);
        builder.write_f32(value.confidence.raw())?;
    }
    builder.write_sequence_len(bindings.action_families.len());
    for value in &bindings.action_families {
        builder.write_u8(value.raw());
    }
    builder.write_f32(bindings.emotions.mean_valence.raw())?;
    builder.write_f32(bindings.emotions.mean_prediction_error.raw())?;
    builder.write_u32(bindings.emotions.observation_count);
    builder.write_sequence_len(bindings.locations.len());
    for value in &bindings.locations {
        builder.write_f32(value.x)?;
        builder.write_f32(value.y)?;
        builder.write_f32(value.z)?;
    }
    builder.write_sequence_len(bindings.agents.len());
    for value in &bindings.agents {
        builder.write_u64(value.raw());
    }
    builder.write_u32(bindings.affordances.raw());
    builder.write_sequence_len(bindings.semantic_refs.len());
    for value in &bindings.semantic_refs {
        builder.write_u64(value.raw());
    }
    builder.write_sequence_len(bindings.cluster_refs.len());
    for value in &bindings.cluster_refs {
        builder.write_u64(value.raw());
    }
    Ok(())
}

fn encode_edge(
    builder: &mut CanonicalDigestBuilder,
    edge: &CognitiveEdge,
) -> Result<(), ScaffoldContractError> {
    builder.write_u64(edge.id.raw());
    builder.write_u64(edge.from.raw());
    builder.write_u64(edge.to.raw());
    builder.write_u8(edge.relation as u8);
    builder.write_f32(edge.strength.raw())?;
    builder.write_u32(edge.evidence_count);
    builder.write_u64(edge.first_tick.raw());
    builder.write_u64(edge.last_tick.raw());
    builder.write_f32(edge.confidence.raw())
}

fn encode_simplex(
    builder: &mut CanonicalDigestBuilder,
    simplex: &CognitiveSimplex,
) -> Result<(), ScaffoldContractError> {
    builder.write_u64(simplex.id.raw());
    builder.write_sequence_len(simplex.concept_ids.len());
    for value in &simplex.concept_ids {
        builder.write_u64(value.raw());
    }
    builder.write_u32(simplex.observation_count);
    builder.write_f32(simplex.mean_valence.raw())?;
    builder.write_f32(simplex.mean_prediction_error.raw())?;
    builder.write_f32(simplex.salience.raw())?;
    builder.write_u64(simplex.first_tick.raw());
    builder.write_u64(simplex.last_tick.raw());
    Ok(())
}

fn encode_gap(
    builder: &mut CanonicalDigestBuilder,
    gap: &UnresolvedGap,
) -> Result<(), ScaffoldContractError> {
    builder.write_u64(gap.id.raw());
    builder.write_sequence_len(gap.source_concepts.len());
    for value in &gap.source_concepts {
        builder.write_u64(value.raw());
    }
    builder.write_u8(gap.contradiction_type as u8);
    builder.write_f32(gap.prediction_error.raw())?;
    builder.write_f32(gap.curiosity_voltage.raw())?;
    builder.write_f32(gap.salience.raw())?;
    builder.write_u64(gap.first_tick.raw());
    builder.write_u64(gap.last_tick.raw());
    builder.write_f32(gap.confidence.raw())?;
    builder.write_u8(gap.status as u8);
    Ok(())
}

fn primary_signature(patch: &ExperiencePatch) -> ConceptSignature {
    patch
        .decision()
        .episodic_key()
        .and_then(|key| key.query().tracked_object_id())
        .map(ConceptSignature::TrackedObject)
        .or_else(|| first_heard_word(patch).map(ConceptSignature::Word))
        .unwrap_or_else(|| ConceptSignature::Sequence(patch.header().sequence_id.raw()))
}

fn first_heard_word(patch: &ExperiencePatch) -> Option<u32> {
    patch
        .pre_action()
        .sensory()
        .context_streams
        .vocal_tokens
        .iter()
        .flatten()
        .map(|token| token.token_id)
        .chain(
            patch
                .pre_action()
                .sensory()
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
) -> Result<TopologyObservationBindings, ScaffoldContractError> {
    let mut primary_bindings = ConceptBindings::default();
    let mut action_bindings = ConceptBindings::default();
    let pre = patch.pre_action();
    let decision = patch.decision();
    let outcome = patch.outcome();

    let action = ActionObservationFact {
        action_id: decision.selected_action.action_id,
        kind: decision.selected_action.kind,
        confidence: decision.selected_action.confidence,
    };
    primary_bindings.actions.push(action);
    action_bindings.actions.push(action);
    let action_family = decision.episodic_key().map_or_else(
        || CandidateActionFamily::baseline_for_kind(decision.selected_action.kind),
        |key| key.query().action_family(),
    );
    primary_bindings.action_families.push(action_family);
    action_bindings.action_families.push(action_family);
    primary_bindings
        .emotions
        .record(outcome.reward_valence, outcome.prediction_error)?;
    action_bindings
        .emotions
        .record(outcome.reward_valence, outcome.prediction_error)?;

    if let Some(tracked) = decision
        .episodic_key()
        .and_then(|key| key.query().tracked_object_id())
    {
        push_unique(&mut primary_bindings.objects, tracked);
    }
    push_unique(&mut primary_bindings.locations, pre.body().pose.translation);
    primary_bindings.affordances = pre.sensory().channels.nearby_affordances;
    primary_bindings.drives = drive_bindings(pre.homeostasis().drives);

    for token in pre.sensory().context_streams.vocal_tokens.iter().flatten() {
        push_unique(&mut primary_bindings.words, token.token_id);
        if let Some(speaker) = token.speaker_id {
            push_unique(&mut primary_bindings.agents, speaker);
        }
    }
    for token in pre.sensory().language_context.heard_tokens.iter().flatten() {
        push_unique(&mut primary_bindings.words, token.token_id);
        if let Some(speaker) = token.speaker_id {
            push_unique(&mut primary_bindings.agents, speaker);
        }
    }
    if let Some(token) = pre.sensory().language_context.vocalized_token {
        push_unique(&mut primary_bindings.words, token.token_id);
    }
    for social in pre.sensory().social_context.nearest_agents.iter().flatten() {
        push_unique(&mut primary_bindings.agents, social.agent_id);
    }
    if let Some(semantic) = &pre.sensory().semantic_context {
        for entry in &semantic.salience {
            push_unique(&mut primary_bindings.semantic_refs, entry.concept_id);
        }
    }
    if let Some(gaussian) = &pre.sensory().gaussian_context {
        for entry in &gaussian.clusters {
            push_unique(&mut primary_bindings.cluster_refs, entry.cluster_id);
        }
    }

    primary_bindings.validate_contract()?;
    action_bindings.validate_contract()?;
    Ok(TopologyObservationBindings {
        primary_bindings,
        action_bindings,
    })
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
        .homeostasis()
        .drives
        .curiosity
        .max(pre.homeostasis().drives.fear);
    let sensory_salience = pre
        .sensory()
        .channels
        .novelty_signal
        .raw()
        .max(pre.sensory().channels.pain_signal.raw());
    let outcome_salience = outcome
        .prediction_error
        .raw()
        .max(outcome.reward_valence.raw().abs());
    NormalizedScalar::new(drive_salience.max(sensory_salience).max(outcome_salience))
}

fn concept_matches_signature(concept: &ConceptCell, signature: &ConceptSignature) -> bool {
    match signature {
        ConceptSignature::TrackedObject(id) => {
            !concept.is_summary && concept.bindings.objects.contains(id)
        }
        ConceptSignature::Action { family, action_id } => {
            concept.bindings.objects.is_empty()
                && !concept.is_summary
                && concept
                    .bindings
                    .actions
                    .iter()
                    .any(|action| action.action_id == *action_id)
                && concept.bindings.action_families.contains(family)
        }
        ConceptSignature::Word(id) => concept.bindings.words.contains(id),
        ConceptSignature::Sequence(sequence) => {
            concept.id.raw() == *sequence && concept.observation_count > 0
        }
    }
}

trait TopologyBindingKey {
    fn topology_binding_key(self) -> u128;
}

impl TopologyBindingKey for TrackedObjectId {
    fn topology_binding_key(self) -> u128 {
        u128::from(self.raw())
    }
}

impl TopologyBindingKey for u32 {
    fn topology_binding_key(self) -> u128 {
        u128::from(self)
    }
}

impl TopologyBindingKey for OrganismId {
    fn topology_binding_key(self) -> u128 {
        u128::from(self.raw())
    }
}

impl TopologyBindingKey for ConceptCellId {
    fn topology_binding_key(self) -> u128 {
        u128::from(self.raw())
    }
}

impl TopologyBindingKey for GaussianClusterId {
    fn topology_binding_key(self) -> u128 {
        u128::from(self.raw())
    }
}

impl TopologyBindingKey for CandidateActionFamily {
    fn topology_binding_key(self) -> u128 {
        u128::from(self.raw())
    }
}

fn append_unique_bounded<T: Copy + PartialEq + TopologyBindingKey>(
    target: &mut Vec<T>,
    values: impl Iterator<Item = T>,
) -> bool {
    let before_unique = target.len();
    for value in values {
        if !target.contains(&value) {
            target.push(value);
        }
    }
    target.sort_by_key(|value| value.topology_binding_key());
    target.dedup();
    let truncated = target.len() > MAX_BINDING_REFS;
    target.truncate(MAX_BINDING_REFS);
    truncated || before_unique > MAX_BINDING_REFS
}

fn merge_drive_bindings(
    target: &mut Vec<DriveBinding>,
    values: impl Iterator<Item = DriveBinding>,
) -> Result<bool, ScaffoldContractError> {
    for value in values {
        value.validate_contract()?;
        if let Some(existing) = target
            .iter_mut()
            .find(|existing| existing.channel == value.channel)
        {
            existing.value = value.value;
        } else {
            target.push(value);
        }
    }
    target.sort_by_key(|value| value.channel as u8);
    let truncated = target.len() > MAX_BINDING_REFS;
    target.truncate(MAX_BINDING_REFS);
    Ok(truncated)
}

fn merge_action_observations(
    target: &mut Vec<ActionObservationFact>,
    values: impl Iterator<Item = ActionObservationFact>,
) -> Result<bool, ScaffoldContractError> {
    for value in values {
        value.validate_contract()?;
        if let Some(existing) = target
            .iter_mut()
            .find(|existing| existing.action_id == value.action_id && existing.kind == value.kind)
        {
            existing.confidence =
                Confidence::new(existing.confidence.raw().max(value.confidence.raw()))?;
        } else {
            target.push(value);
        }
    }
    target.sort_by_key(|value| (value.action_id.raw(), value.kind as u8));
    let truncated = target.len() > MAX_BINDING_REFS;
    target.truncate(MAX_BINDING_REFS);
    Ok(truncated)
}

fn merge_location_samples(
    target: &mut Vec<Vec3f>,
    values: impl Iterator<Item = Vec3f>,
    tick: Tick,
) -> Result<bool, ScaffoldContractError> {
    let mut truncated = false;
    for value in values {
        value.validate()?;
        if target.contains(&value) {
            continue;
        }
        if target.len() < MAX_BINDING_REFS {
            target.push(value);
        } else {
            let index = (tick.raw() as usize) % MAX_BINDING_REFS;
            target[index] = value;
            truncated = true;
        }
    }
    target.sort_by_key(|value| (value.x.to_bits(), value.y.to_bits(), value.z.to_bits()));
    target.dedup();
    Ok(truncated)
}

fn push_unique<T: Copy + PartialEq>(target: &mut Vec<T>, value: T) {
    if !target.contains(&value) && target.len() < MAX_BINDING_REFS {
        target.push(value);
    }
}
