//! World-owned, Bevy-independent organism identity and biology records.

use std::collections::HashMap;

use alife_core::{
    BiochemistryState, Blake3Digest, BodyEventDelta, CanonicalDigestBuilder, CreatureGenome,
    CreaturePhenotype, OrganismId, ScaffoldContractError, Tick, Validate, WorldEntityId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ORGANISM_REGISTRY_SIGNATURE_ENCODING_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganismArchiveIdentity {
    birth_manifest_digest: Option<Blake3Digest>,
    life_manifest_digest: Option<Blake3Digest>,
}

impl OrganismArchiveIdentity {
    pub const fn birth_manifest_digest(&self) -> Option<Blake3Digest> {
        self.birth_manifest_digest
    }

    pub const fn life_manifest_digest(&self) -> Option<Blake3Digest> {
        self.life_manifest_digest
    }

    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        for digest in self
            .birth_manifest_digest
            .iter()
            .chain(self.life_manifest_digest.iter())
        {
            if digest.bytes().iter().all(|byte| *byte == 0) {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        if self.life_manifest_digest.is_some() && self.birth_manifest_digest.is_none() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganismLifecycle {
    Alive,
    Dead { death_tick: Tick },
}

impl Default for OrganismLifecycle {
    fn default() -> Self {
        Self::Alive
    }
}

impl OrganismLifecycle {
    pub const fn is_alive(self) -> bool {
        matches!(self, Self::Alive)
    }

    pub const fn death_tick(self) -> Option<Tick> {
        match self {
            Self::Alive => None,
            Self::Dead { death_tick } => Some(death_tick),
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum OrganismRegistryError {
    #[error("organism record contract is invalid: {0}")]
    InvalidRecord(#[from] ScaffoldContractError),
    #[error("organism ID is already registered: {0:?}")]
    DuplicateOrganism(OrganismId),
    #[error("world entity ID is already registered: {0:?}")]
    DuplicateWorldEntity(WorldEntityId),
    #[error("organism ID is not registered: {0:?}")]
    UnknownOrganism(OrganismId),
    #[error("organism is already dead: {0:?}")]
    AlreadyDead(OrganismId),
    #[error("dead organism cannot advance biology: {0:?}")]
    DeadOrganism(OrganismId),
    #[error("organism death tick is invalid: {0:?}")]
    InvalidDeathTick(OrganismId),
    #[error("birth manifest is already linked: {0:?}")]
    BirthManifestAlreadyLinked(OrganismId),
    #[error("life manifest is already linked: {0:?}")]
    LifeManifestAlreadyLinked(OrganismId),
    #[error("life manifest requires a birth manifest: {0:?}")]
    LifeManifestRequiresBirth(OrganismId),
    #[error("life manifest requires a dead organism: {0:?}")]
    LifeManifestRequiresDead(OrganismId),
    #[error("dead organism is missing its linked life manifest: {0:?}")]
    LifeManifestNotLinked(OrganismId),
    #[error("organism registry indexes are not bijective")]
    IndexMismatch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldOrganismRecord {
    organism_id: OrganismId,
    world_entity_id: WorldEntityId,
    genome: CreatureGenome,
    phenotype: CreaturePhenotype,
    biochemistry: BiochemistryState,
    birth_tick: Tick,
    lifecycle: OrganismLifecycle,
    archive: OrganismArchiveIdentity,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldOrganismAdmissionSnapshot {
    pub organism_id: OrganismId,
    pub world_entity_id: WorldEntityId,
    pub genome: CreatureGenome,
    pub phenotype: CreaturePhenotype,
    pub biochemistry: BiochemistryState,
    pub age: Tick,
}

impl WorldOrganismRecord {
    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn world_entity_id(&self) -> WorldEntityId {
        self.world_entity_id
    }

    pub fn genome(&self) -> &CreatureGenome {
        &self.genome
    }

    pub fn phenotype(&self) -> &CreaturePhenotype {
        &self.phenotype
    }

    pub fn biochemistry(&self) -> &BiochemistryState {
        &self.biochemistry
    }

    pub const fn birth_tick(&self) -> Tick {
        self.birth_tick
    }

    pub const fn lifecycle(&self) -> OrganismLifecycle {
        self.lifecycle
    }

    pub fn archive(&self) -> &OrganismArchiveIdentity {
        &self.archive
    }

    pub fn authoritative_admission_at(
        &self,
        world_tick: Tick,
    ) -> Result<WorldOrganismAdmissionSnapshot, ScaffoldContractError> {
        self.validate_contract()?;
        if !self.lifecycle.is_alive() || self.biochemistry.tick != world_tick {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        Ok(WorldOrganismAdmissionSnapshot {
            organism_id: self.organism_id,
            world_entity_id: self.world_entity_id,
            genome: self.genome.clone(),
            phenotype: self.phenotype.clone(),
            biochemistry: self.biochemistry,
            age: self.age_at(world_tick)?,
        })
    }

    pub fn new(
        organism_id: OrganismId,
        world_entity_id: WorldEntityId,
        genome: CreatureGenome,
        phenotype: CreaturePhenotype,
        biochemistry: BiochemistryState,
        birth_tick: Tick,
    ) -> Result<Self, OrganismRegistryError> {
        let record = Self {
            organism_id,
            world_entity_id,
            genome,
            phenotype,
            biochemistry,
            birth_tick,
            lifecycle: OrganismLifecycle::Alive,
            archive: OrganismArchiveIdentity::default(),
        };
        record.validate_contract()?;
        Ok(record)
    }

    pub fn newborn(
        organism_id: OrganismId,
        world_entity_id: WorldEntityId,
        genome: CreatureGenome,
        phenotype: CreaturePhenotype,
        birth_tick: Tick,
    ) -> Result<Self, OrganismRegistryError> {
        let biochemistry = BiochemistryState::new_with_age(&phenotype, birth_tick, Tick(0))?;
        Self::new(
            organism_id,
            world_entity_id,
            genome,
            phenotype,
            biochemistry,
            birth_tick,
        )
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.world_entity_id.validate()?;
        self.genome.validate_contract()?;
        if self.phenotype.source_genome_id != self.genome.id
            || self.phenotype.lineage_id != self.genome.lineage_id
            || self.biochemistry.source_genome_id != self.genome.id
        {
            return Err(ScaffoldContractError::InvalidId);
        }

        let expressed = self.genome.express()?;
        if self.phenotype != expressed {
            return Err(ScaffoldContractError::InvalidId);
        }

        self.biochemistry.validate_contract()?;
        let expected_age = self.age_at(self.biochemistry.tick)?;
        if self.biochemistry.development.age_ticks != expected_age {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        if self.birth_tick.raw() > self.biochemistry.tick.raw() {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        match self.lifecycle {
            OrganismLifecycle::Alive => {
                if self.archive.life_manifest_digest.is_some() {
                    return Err(ScaffoldContractError::InvalidId);
                }
            }
            OrganismLifecycle::Dead { death_tick } => {
                if death_tick.raw() < self.birth_tick.raw()
                    || death_tick.raw() < self.biochemistry.tick.raw()
                {
                    return Err(ScaffoldContractError::NonMonotonicTick);
                }
            }
        }
        self.archive.validate_contract()?;
        Ok(())
    }

    pub fn age_at(&self, current_tick: Tick) -> Result<Tick, ScaffoldContractError> {
        Tick::validate_monotonic(self.birth_tick, current_tick)?;
        Ok(Tick(current_tick.raw() - self.birth_tick.raw()))
    }

    pub fn advance_biology(
        &mut self,
        next_tick: Tick,
        event: BodyEventDelta,
    ) -> Result<(), OrganismRegistryError> {
        if !self.lifecycle.is_alive() {
            return Err(OrganismRegistryError::DeadOrganism(self.organism_id));
        }
        let original_biochemistry = self.biochemistry;
        self.validate_contract()?;
        let next_age = self.age_at(next_tick)?;
        let next = self
            .biochemistry
            .advance_with_age(next_tick, next_age, event, &self.phenotype)
            .map_err(OrganismRegistryError::InvalidRecord)?;
        self.biochemistry = next;
        if let Err(error) = self.validate_contract() {
            self.biochemistry = original_biochemistry;
            return Err(error.into());
        }
        Ok(())
    }

    pub fn mark_dead(&mut self, death_tick: Tick) -> Result<(), OrganismRegistryError> {
        self.validate_contract()?;
        if !self.lifecycle.is_alive() {
            return Err(OrganismRegistryError::AlreadyDead(self.organism_id));
        }
        if death_tick.raw() < self.birth_tick.raw()
            || death_tick.raw() < self.biochemistry.tick.raw()
        {
            return Err(OrganismRegistryError::InvalidDeathTick(self.organism_id));
        }
        let original_lifecycle = self.lifecycle;
        self.lifecycle = OrganismLifecycle::Dead { death_tick };
        if let Err(error) = self.validate_contract() {
            self.lifecycle = original_lifecycle;
            return Err(error.into());
        }
        Ok(())
    }

    pub fn link_birth_manifest(
        &mut self,
        digest: Blake3Digest,
    ) -> Result<(), OrganismRegistryError> {
        self.validate_contract()?;
        if digest.bytes().iter().all(|byte| *byte == 0) {
            return Err(OrganismRegistryError::InvalidRecord(
                ScaffoldContractError::InvalidId,
            ));
        }
        if let Some(existing) = self.archive.birth_manifest_digest {
            if existing == digest {
                return Ok(());
            }
            return Err(OrganismRegistryError::BirthManifestAlreadyLinked(
                self.organism_id,
            ));
        }
        let original_archive = self.archive;
        self.archive.birth_manifest_digest = Some(digest);
        if let Err(error) = self.validate_contract() {
            self.archive = original_archive;
            return Err(error.into());
        }
        Ok(())
    }

    pub fn link_life_manifest(
        &mut self,
        digest: Blake3Digest,
    ) -> Result<(), OrganismRegistryError> {
        self.validate_contract()?;
        if digest.bytes().iter().all(|byte| *byte == 0) {
            return Err(OrganismRegistryError::InvalidRecord(
                ScaffoldContractError::InvalidId,
            ));
        }
        if let Some(existing) = self.archive.life_manifest_digest {
            if existing == digest {
                return Ok(());
            }
            return Err(OrganismRegistryError::LifeManifestAlreadyLinked(
                self.organism_id,
            ));
        }
        if self.archive.birth_manifest_digest.is_none() {
            return Err(OrganismRegistryError::LifeManifestRequiresBirth(
                self.organism_id,
            ));
        }
        if self.lifecycle.is_alive() {
            return Err(OrganismRegistryError::LifeManifestRequiresDead(
                self.organism_id,
            ));
        }
        let original_archive = self.archive;
        self.archive.life_manifest_digest = Some(digest);
        if let Err(error) = self.validate_contract() {
            self.archive = original_archive;
            return Err(error.into());
        }
        Ok(())
    }
}

impl Validate for WorldOrganismRecord {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        WorldOrganismRecord::validate_contract(self)
    }
}

#[derive(Debug, Clone, Default)]
pub struct WorldOrganismRegistry {
    records_by_organism: HashMap<u64, WorldOrganismRecord>,
    organism_by_world_entity: HashMap<u64, OrganismId>,
}

impl WorldOrganismRegistry {
    pub(crate) fn from_exact_records<I>(records: I) -> Result<Self, OrganismRegistryError>
    where
        I: IntoIterator<Item = WorldOrganismRecord>,
    {
        let mut registry = Self::default();
        for record in records {
            registry.insert(record)?;
        }
        registry.validate_contract()?;
        Ok(registry)
    }

    pub fn insert(&mut self, record: WorldOrganismRecord) -> Result<(), OrganismRegistryError> {
        record.validate_contract()?;
        let organism_id = record.organism_id;
        let world_entity_id = record.world_entity_id;
        if self.records_by_organism.contains_key(&organism_id.raw()) {
            return Err(OrganismRegistryError::DuplicateOrganism(organism_id));
        }
        if self
            .organism_by_world_entity
            .contains_key(&world_entity_id.raw())
        {
            return Err(OrganismRegistryError::DuplicateWorldEntity(world_entity_id));
        }
        self.records_by_organism.insert(organism_id.raw(), record);
        self.organism_by_world_entity
            .insert(world_entity_id.raw(), organism_id);
        Ok(())
    }

    pub fn get(&self, organism_id: OrganismId) -> Option<&WorldOrganismRecord> {
        self.records_by_organism.get(&organism_id.raw())
    }

    pub fn get_by_world_entity_id(
        &self,
        world_entity_id: WorldEntityId,
    ) -> Option<&WorldOrganismRecord> {
        self.organism_by_world_entity
            .get(&world_entity_id.raw())
            .and_then(|organism_id| self.get(*organism_id))
    }

    pub fn iter(&self) -> impl Iterator<Item = &WorldOrganismRecord> {
        self.records_by_organism.values()
    }

    pub fn with_biology_mut<R>(
        &mut self,
        organism_id: OrganismId,
        mutate: impl FnOnce(&mut BiochemistryState) -> Result<R, OrganismRegistryError>,
    ) -> Result<R, OrganismRegistryError> {
        let record = self
            .records_by_organism
            .get_mut(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?;
        if !record.lifecycle.is_alive() {
            return Err(OrganismRegistryError::DeadOrganism(organism_id));
        }
        record.validate_contract()?;
        let original_biochemistry = record.biochemistry;
        let result = mutate(&mut record.biochemistry);
        match result {
            Err(error) => {
                record.biochemistry = original_biochemistry;
                Err(error)
            }
            Ok(value) => {
                let valid_tick = record.biochemistry.tick.raw() >= original_biochemistry.tick.raw();
                if !valid_tick {
                    record.biochemistry = original_biochemistry;
                    return Err(OrganismRegistryError::InvalidRecord(
                        ScaffoldContractError::NonMonotonicTick,
                    ));
                }
                if let Err(error) = record.validate_contract() {
                    record.biochemistry = original_biochemistry;
                    Err(error.into())
                } else {
                    Ok(value)
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.records_by_organism.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records_by_organism.is_empty()
    }

    pub fn advance_biology(
        &mut self,
        organism_id: OrganismId,
        next_tick: Tick,
        event: BodyEventDelta,
    ) -> Result<(), OrganismRegistryError> {
        self
            .records_by_organism
            .get_mut(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?
            .advance_biology(next_tick, event)
    }

    pub fn mark_dead(
        &mut self,
        organism_id: OrganismId,
        death_tick: Tick,
    ) -> Result<(), OrganismRegistryError> {
        self.records_by_organism
            .get_mut(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?
            .mark_dead(death_tick)
    }

    pub fn link_birth_manifest(
        &mut self,
        organism_id: OrganismId,
        digest: Blake3Digest,
    ) -> Result<(), OrganismRegistryError> {
        self.records_by_organism
            .get_mut(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?
            .link_birth_manifest(digest)
    }

    pub fn link_life_manifest(
        &mut self,
        organism_id: OrganismId,
        digest: Blake3Digest,
    ) -> Result<(), OrganismRegistryError> {
        self.records_by_organism
            .get_mut(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?
            .link_life_manifest(digest)
    }

    pub fn remove_dead(
        &mut self,
        organism_id: OrganismId,
    ) -> Result<WorldOrganismRecord, OrganismRegistryError> {
        let record = self
            .records_by_organism
            .get(&organism_id.raw())
            .cloned()
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?;
        record.validate_contract()?;
        if record.lifecycle().is_alive() {
            return Err(OrganismRegistryError::LifeManifestRequiresDead(organism_id));
        }
        if record.archive().life_manifest_digest().is_none() {
            return Err(OrganismRegistryError::LifeManifestNotLinked(organism_id));
        }
        if self.organism_by_world_entity.get(&record.world_entity_id().raw())
            != Some(&organism_id)
        {
            return Err(OrganismRegistryError::IndexMismatch);
        }

        let removed_record = self
            .records_by_organism
            .remove(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?;
        let Some(removed_index) = self
            .organism_by_world_entity
            .remove(&record.world_entity_id().raw())
        else {
            self.records_by_organism
                .insert(organism_id.raw(), removed_record);
            return Err(OrganismRegistryError::IndexMismatch);
        };
        if let Err(error) = self.validate_contract() {
            self.records_by_organism
                .insert(organism_id.raw(), removed_record);
            self.organism_by_world_entity
                .insert(record.world_entity_id().raw(), removed_index);
            return Err(error);
        }
        Ok(removed_record)
    }

    pub(crate) fn write_canonical_signature(
        &self,
        digest: &mut CanonicalDigestBuilder,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_contract().map_err(|error| match error {
            OrganismRegistryError::InvalidRecord(error) => error,
            _ => ScaffoldContractError::InvalidId,
        })?;

        // The registry map is deliberately excluded from the payload. The
        // record graph is composed of serde structs, enums, arrays, and
        // sequences with no map fields, so serde_json preserves field order.
        let mut records: Vec<_> = self.records_by_organism.values().collect();
        records.sort_unstable_by_key(|record| record.organism_id().raw());
        digest.write_u16(ORGANISM_REGISTRY_SIGNATURE_ENCODING_VERSION);
        digest.write_sequence_len(records.len());
        for record in records {
            let payload =
                serde_json::to_vec(record).map_err(|_| ScaffoldContractError::InvalidId)?;
            digest.write_bytes(&payload);
        }
        Ok(())
    }

    pub fn validate_contract(&self) -> Result<(), OrganismRegistryError> {
        if self.records_by_organism.len() != self.organism_by_world_entity.len() {
            return Err(OrganismRegistryError::IndexMismatch);
        }
        for (organism_key, record) in &self.records_by_organism {
            record.validate_contract()?;
            if *organism_key != record.organism_id().raw()
                || self
                    .organism_by_world_entity
                    .get(&record.world_entity_id().raw())
                    != Some(&record.organism_id())
            {
                return Err(OrganismRegistryError::IndexMismatch);
            }
        }
        for (world_entity_key, organism_id) in &self.organism_by_world_entity {
            let Some(record) = self.records_by_organism.get(&organism_id.raw()) else {
                return Err(OrganismRegistryError::IndexMismatch);
            };
            if *world_entity_key != record.world_entity_id().raw() {
                return Err(OrganismRegistryError::IndexMismatch);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{BrainCapacityClass, FoundationGeneticIdentity, LineageId};

    fn malformed_registry() -> (WorldOrganismRegistry, OrganismId) {
        let genome = CreatureGenome::early_mammal_founder(
            0xE10_31FF,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let record = WorldOrganismRecord::newborn(
            OrganismId(1),
            WorldEntityId(101),
            genome,
            phenotype,
            Tick(5),
        )
        .unwrap();
        let mut value = serde_json::to_value(record).unwrap();
        value["phenotype"]["lineage_id"] = serde_json::json!(LineageId(3));
        let malformed: WorldOrganismRecord = serde_json::from_value(value).unwrap();
        let organism_id = malformed.organism_id();
        let world_entity_id = malformed.world_entity_id();
        let mut registry = WorldOrganismRegistry::default();
        registry
            .records_by_organism
            .insert(organism_id.raw(), malformed);
        registry
            .organism_by_world_entity
            .insert(world_entity_id.raw(), organism_id);
        (registry, organism_id)
    }

    #[test]
    fn malformed_registry_prestate_does_not_invoke_biology_closure() {
        let (mut registry, organism_id) = malformed_registry();
        let before = *registry.get(organism_id).unwrap().biochemistry();
        let mut closure_called = false;

        let result = registry.with_biology_mut(organism_id, |biology| {
            closure_called = true;
            biology.body.energy = 0.0;
            Ok(())
        });

        assert_eq!(
            result,
            Err(OrganismRegistryError::InvalidRecord(
                ScaffoldContractError::InvalidId,
            ))
        );
        assert!(!closure_called);
        assert_eq!(*registry.get(organism_id).unwrap().biochemistry(), before);
    }

    #[test]
    fn malformed_registry_advance_returns_prestate_error_without_mutation() {
        let (mut registry, organism_id) = malformed_registry();
        let before = *registry.get(organism_id).unwrap().biochemistry();

        let result = registry.advance_biology(organism_id, Tick(4), BodyEventDelta::zero());

        assert_eq!(
            result,
            Err(OrganismRegistryError::InvalidRecord(
                ScaffoldContractError::InvalidId,
            ))
        );
        assert_eq!(*registry.get(organism_id).unwrap().biochemistry(), before);
    }

    #[test]
    fn newborn_record_advance_uses_age_since_birth_not_absolute_world_tick() {
        let genome = CreatureGenome::early_mammal_founder(
            0xE10_31FF,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let mut record = WorldOrganismRecord::newborn(
            OrganismId(1),
            WorldEntityId(101),
            genome,
            phenotype,
            Tick(10_000),
        )
        .unwrap();

        assert_eq!(record.biochemistry().development.age_ticks, Tick(0));
        record
            .advance_biology(Tick(10_001), BodyEventDelta::zero())
            .unwrap();

        assert_eq!(record.biochemistry().development.age_ticks, Tick(1));
    }

    #[test]
    fn registry_failed_advance_preserves_exact_biochemistry() {
        let genome = CreatureGenome::early_mammal_founder(
            0xE10_31FF,
            FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let record = WorldOrganismRecord::newborn(
            OrganismId(1),
            WorldEntityId(101),
            genome,
            phenotype,
            Tick(10_000),
        )
        .unwrap();
        let organism_id = record.organism_id();
        let mut registry = WorldOrganismRegistry::default();
        registry.insert(record).unwrap();
        let before = *registry.get(organism_id).unwrap().biochemistry();

        let result = registry.advance_biology(organism_id, Tick(9_999), BodyEventDelta::zero());

        assert_eq!(
            result,
            Err(OrganismRegistryError::InvalidRecord(
                ScaffoldContractError::NonMonotonicTick,
            ))
        );
        assert_eq!(*registry.get(organism_id).unwrap().biochemistry(), before);
    }
}
