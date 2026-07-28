//! Read-only, evidence-backed world projections for presentation consumers.

use alife_core::{
    LanguageTokenId, NormalizedScalar, OrganismId, SignedValence, Tick, WorldEntityId,
};
use serde::{Deserialize, Serialize};

use crate::{HabitatId, HabitatMode};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PresentationEvidence<T> {
    Observed { value: T, tick: Tick },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairwiseRelationshipProjection {
    pub source_organism_id: OrganismId,
    pub target_organism_id: OrganismId,
    pub target_stable_world_entity_id: Option<WorldEntityId>,
    pub affinity: PresentationEvidence<SignedValence>,
    pub trust: PresentationEvidence<SignedValence>,
    pub fear: PresentationEvidence<NormalizedScalar>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitatCreaturePresentation {
    pub organism_id: OrganismId,
    pub stable_world_entity_id: Option<WorldEntityId>,
    pub habitat_id: HabitatId,
    pub habitat_mode: HabitatMode,
    pub latest_grounded_utterance: PresentationEvidence<Vec<LanguageTokenId>>,
    pub relationships: Vec<PairwiseRelationshipProjection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HabitatPresentationProjection {
    pub tick: Tick,
    pub creatures: Vec<HabitatCreaturePresentation>,
}
