//! Read-only, evidence-backed world projections for presentation consumers.

use alife_core::{
    ActionId, ActionKind, BiochemistryState, BodyState, CreatureGenome, CreaturePhenotype,
    LanguageTokenId, NormalizedScalar, OrganismId, PhysicalContactKind, ReferenceActionFailure,
    SignedValence, SleepPhase, Tick, WorldEntityId,
};
use serde::{Deserialize, Serialize};

use crate::{
    HabitatId, HabitatMode, HeadlessWorld, OrganismLifecycle, WorldObject, WorldObjectKind,
};

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

#[derive(Debug, Clone, PartialEq)]
pub struct PresentationMotorSnapshot {
    pub action_kind: Option<ActionKind>,
    pub action_id: Option<ActionId>,
    pub target_entity: Option<WorldEntityId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PresentationOutcomeSnapshot {
    pub patch_sealed: bool,
    pub patch_sequence_id: Option<u64>,
    pub patch_success: Option<bool>,
    pub physical_contact: Option<PhysicalContactKind>,
    pub action_failure: Option<ReferenceActionFailure>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldOrganismPresentationRow {
    pub organism_id: OrganismId,
    pub world_entity_id: WorldEntityId,
    pub object: WorldObject,
    pub genome: CreatureGenome,
    pub phenotype: CreaturePhenotype,
    pub biochemistry: BiochemistryState,
    pub body: BodyState,
    pub birth_tick: Tick,
    pub lifecycle: OrganismLifecycle,
    pub sleep_phase: SleepPhase,
    pub sleep_phase_tick: Tick,
    pub sleep_cycle_id: u64,
    pub sleep_work_units: u64,
    pub motor: Option<PresentationMotorSnapshot>,
    pub outcome: Option<PresentationOutcomeSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldPresentationSnapshot {
    pub tick: Tick,
    pub organisms: Vec<WorldOrganismPresentationRow>,
}

impl WorldPresentationSnapshot {
    pub fn organism(&self, stable_id: WorldEntityId) -> Option<&WorldOrganismPresentationRow> {
        self.organisms
            .iter()
            .find(|row| row.world_entity_id == stable_id)
    }
}

impl HeadlessWorld {
    pub fn presentation_snapshot(&self) -> WorldPresentationSnapshot {
        let mut organisms = self
            .organism_registry()
            .iter()
            .filter_map(|record| {
                let object = self.entity(record.world_entity_id())?.clone();
                if object.kind != WorldObjectKind::Agent
                    || object.organism_id != Some(record.organism_id())
                {
                    return None;
                }

                let biochemistry = *record.biochemistry();
                Some(WorldOrganismPresentationRow {
                    organism_id: record.organism_id(),
                    world_entity_id: record.world_entity_id(),
                    object,
                    genome: record.genome().clone(),
                    phenotype: record.phenotype().clone(),
                    biochemistry,
                    body: biochemistry.body,
                    birth_tick: record.birth_tick(),
                    lifecycle: record.lifecycle().clone(),
                    sleep_phase: record.sleep_phase(),
                    sleep_phase_tick: record.sleep_phase_tick(),
                    sleep_cycle_id: record.sleep_cycle_id(),
                    sleep_work_units: record.sleep_work_units(),
                    motor: None,
                    outcome: None,
                })
            })
            .collect::<Vec<_>>();
        organisms.sort_by_key(|row| (row.world_entity_id.raw(), row.organism_id.raw()));
        WorldPresentationSnapshot {
            tick: self.tick(),
            organisms,
        }
    }
}
