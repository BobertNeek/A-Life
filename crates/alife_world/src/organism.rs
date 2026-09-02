//! World-owned, Bevy-independent organism identity and biology records.

use std::collections::HashMap;

use alife_core::cognitive_work::{CognitiveWorkCostPolicy, CognitiveWorkReceipt};
use alife_core::{
    BiochemistryState, Blake3Digest, BodyEventDelta, CanonicalDigestBuilder, CreatureGenome,
    CreaturePhenotype, EmbodimentState, HomeostaticSnapshot, NeuralEmissionFrame, OrganismId,
    ScaffoldContractError, SchemaVersions, SleepPhase, Tick, Validate, WorldEntityId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

const ORGANISM_REGISTRY_SIGNATURE_ENCODING_VERSION: u16 = 1;

fn is_zero_cognitive_work(value: &CognitiveWorkReceipt) -> bool {
    *value == CognitiveWorkReceipt::zero()
}

fn is_zero_energy_debit(value: &f32) -> bool {
    *value == 0.0
}

fn default_sleep_phase() -> SleepPhase {
    SleepPhase::Awake
}

fn default_sleep_phase_tick() -> Tick {
    Tick::ZERO
}

fn body_biochemistry_digest(state: &BiochemistryState) -> Result<[u64; 4], ScaffoldContractError> {
    let mut digest = CanonicalDigestBuilder::new(b"alife.organism.body-biochemistry.v3");
    let bytes = serde_json::to_vec(state).map_err(|_| ScaffoldContractError::InvalidId)?;
    digest.write_bytes(&bytes);
    Ok(digest.finish256())
}

fn embodiment_digest(state: &EmbodimentState) -> Result<[u64; 4], ScaffoldContractError> {
    let mut digest = CanonicalDigestBuilder::new(b"alife.organism.embodiment.v3");
    let bytes = serde_json::to_vec(state).map_err(|_| ScaffoldContractError::InvalidId)?;
    digest.write_bytes(&bytes);
    Ok(digest.finish256())
}

fn genetics_development_digest(
    genome: &CreatureGenome,
    phenotype: &CreaturePhenotype,
) -> Result<[u64; 4], ScaffoldContractError> {
    let mut digest = CanonicalDigestBuilder::new(b"alife.organism.genetics-development.v3");
    let genome_bytes =
        serde_json::to_vec(genome).map_err(|_| ScaffoldContractError::InvalidGeneticBounds)?;
    let phenotype_bytes =
        serde_json::to_vec(phenotype).map_err(|_| ScaffoldContractError::InvalidGeneticBounds)?;
    digest.write_bytes(&genome_bytes);
    digest.write_bytes(&phenotype_bytes);
    Ok(digest.finish256())
}

fn initial_brain_digest(
    organism_id: OrganismId,
    phenotype: &CreaturePhenotype,
) -> Result<[u64; 4], ScaffoldContractError> {
    let mut digest = CanonicalDigestBuilder::new(b"alife.organism.brain.birth-state.v3");
    digest.write_u64(organism_id.raw());
    let brain_bytes = serde_json::to_vec(&phenotype.brain_genome)
        .map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
    digest.write_bytes(&brain_bytes);
    Ok(digest.finish256())
}

fn initial_memory_digest(organism_id: OrganismId) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(b"alife.organism.memory.empty-state.v3");
    digest.write_u64(organism_id.raw());
    digest.write_sequence_len(0);
    digest.finish256()
}

fn lifecycle_persistence_digest(
    organism_id: OrganismId,
    world_entity_id: WorldEntityId,
    birth_tick: Tick,
    lifecycle: OrganismLifecycle,
    sleep_phase: SleepPhase,
    sleep_phase_tick: Tick,
    sleep_cycle_id: u64,
    sleep_work_units: u64,
    archive: &OrganismArchiveIdentity,
) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(b"alife.organism.lifecycle-persistence.v3");
    digest.write_u64(organism_id.raw());
    digest.write_u64(world_entity_id.raw());
    digest.write_u64(birth_tick.raw());
    match lifecycle {
        OrganismLifecycle::Alive => digest.write_u8(0),
        OrganismLifecycle::Dead { death_tick } => {
            digest.write_u8(1);
            digest.write_u64(death_tick.raw());
        }
    }
    digest.write_u8(sleep_phase as u8);
    digest.write_u64(sleep_phase_tick.raw());
    digest.write_u64(sleep_cycle_id);
    digest.write_u64(sleep_work_units);
    for value in [
        archive.birth_manifest_digest(),
        archive.life_manifest_digest(),
    ] {
        match value {
            Some(value) => {
                digest.write_some();
                digest.write_bytes(value.bytes());
            }
            None => digest.write_none(),
        }
    }
    digest.finish256()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganismSleepSeal {
    pub phase: SleepPhase,
    pub cycle_id: u64,
    pub tick: Tick,
    pub work_units: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrganismSleepInput {
    pub organism_id: OrganismId,
    pub lifecycle: OrganismLifecycle,
    pub biological_tick: Tick,
    pub energy: f32,
    pub homeostasis: HomeostaticSnapshot,
    pub body_sleeping: bool,
    pub seal: OrganismSleepSeal,
}

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
pub struct SubsystemStateRef {
    pub organism_id: OrganismId,
    pub schema_version: u16,
    pub revision: u64,
    pub causal_tick: Tick,
    pub content_digest: [u64; 4],
}

impl Validate for SubsystemStateRef {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.schema_version == 0 || self.revision == 0 || self.content_digest == [0; 4] {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganismSubsystemStateGraph {
    pub organism_id: OrganismId,
    pub transaction_revision: u64,
    pub genetics_development: SubsystemStateRef,
    pub body_biochemistry: SubsystemStateRef,
    pub brain: SubsystemStateRef,
    pub memory: SubsystemStateRef,
    pub embodiment: SubsystemStateRef,
    pub lifecycle_persistence: SubsystemStateRef,
}

impl Validate for OrganismSubsystemStateGraph {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.transaction_revision == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        for subsystem in [
            &self.genetics_development,
            &self.body_biochemistry,
            &self.brain,
            &self.memory,
            &self.embodiment,
            &self.lifecycle_persistence,
        ] {
            subsystem.validate_contract()?;
            if subsystem.organism_id != self.organism_id {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldOrganismRecord {
    organism_id: OrganismId,
    world_entity_id: WorldEntityId,
    genome: CreatureGenome,
    phenotype: CreaturePhenotype,
    biochemistry: BiochemistryState,
    state_graph: OrganismSubsystemStateGraph,
    embodiment: EmbodimentState,
    birth_tick: Tick,
    lifecycle: OrganismLifecycle,
    #[serde(default = "default_sleep_phase")]
    sleep_phase: SleepPhase,
    #[serde(default = "default_sleep_phase_tick")]
    sleep_phase_tick: Tick,
    #[serde(default)]
    sleep_cycle_id: u64,
    #[serde(default)]
    sleep_work_units: u64,
    archive: OrganismArchiveIdentity,
    #[serde(default, skip_serializing_if = "is_zero_cognitive_work")]
    cognitive_work: CognitiveWorkReceipt,
    #[serde(default, skip_serializing_if = "is_zero_energy_debit")]
    cognitive_energy_debit: f32,
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

    pub const fn state_graph(&self) -> &OrganismSubsystemStateGraph {
        &self.state_graph
    }

    pub const fn embodiment(&self) -> &EmbodimentState {
        &self.embodiment
    }

    pub const fn birth_tick(&self) -> Tick {
        self.birth_tick
    }

    pub const fn lifecycle(&self) -> OrganismLifecycle {
        self.lifecycle
    }

    pub const fn sleep_phase(&self) -> SleepPhase {
        self.sleep_phase
    }

    pub const fn sleep_phase_tick(&self) -> Tick {
        self.sleep_phase_tick
    }

    pub const fn sleep_cycle_id(&self) -> u64 {
        self.sleep_cycle_id
    }

    pub const fn sleep_work_units(&self) -> u64 {
        self.sleep_work_units
    }

    pub const fn sleep_seal(&self) -> OrganismSleepSeal {
        OrganismSleepSeal {
            phase: self.sleep_phase,
            cycle_id: self.sleep_cycle_id,
            tick: self.sleep_phase_tick,
            work_units: self.sleep_work_units,
        }
    }

    pub fn authoritative_sleep_input(&self) -> Result<OrganismSleepInput, ScaffoldContractError> {
        self.validate_contract()?;
        Ok(OrganismSleepInput {
            organism_id: self.organism_id,
            lifecycle: self.lifecycle,
            biological_tick: self.biochemistry.tick,
            energy: self.biochemistry.body.energy,
            homeostasis: self.biochemistry.homeostasis,
            body_sleeping: self.biochemistry.body.sleeping,
            seal: self.sleep_seal(),
        })
    }

    pub fn archive(&self) -> &OrganismArchiveIdentity {
        &self.archive
    }

    pub fn cognitive_work(&self) -> &CognitiveWorkReceipt {
        &self.cognitive_work
    }

    pub const fn cognitive_energy_debit(&self) -> f32 {
        self.cognitive_energy_debit
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
        let embodiment =
            EmbodimentState::from_phenotype(world_entity_id, biochemistry.tick, &phenotype)?;
        let genome_id = genome.id.0;
        let brain_schema_version = phenotype.brain_genome.schema_version;
        let genetics_development_digest = genetics_development_digest(&genome, &phenotype)?;
        let brain_birth_digest = initial_brain_digest(organism_id, &phenotype)?;
        let initial_ref = |tag: u64, schema_version: u16| SubsystemStateRef {
            organism_id,
            schema_version,
            revision: 1,
            causal_tick: biochemistry.tick,
            content_digest: [organism_id.raw(), genome_id, tag, biochemistry.tick.raw()],
        };
        let record = Self {
            organism_id,
            world_entity_id,
            genome,
            phenotype,
            biochemistry,
            state_graph: OrganismSubsystemStateGraph {
                organism_id,
                transaction_revision: 1,
                genetics_development: SubsystemStateRef {
                    content_digest: genetics_development_digest,
                    ..initial_ref(10, SchemaVersions::CURRENT.genome.raw())
                },
                body_biochemistry: SubsystemStateRef {
                    content_digest: body_biochemistry_digest(&biochemistry)?,
                    ..initial_ref(1, SchemaVersions::CURRENT.chemistry.raw())
                },
                brain: SubsystemStateRef {
                    content_digest: brain_birth_digest,
                    ..initial_ref(2, brain_schema_version)
                },
                memory: SubsystemStateRef {
                    content_digest: initial_memory_digest(organism_id),
                    ..initial_ref(3, 1)
                },
                embodiment: SubsystemStateRef {
                    content_digest: embodiment_digest(&embodiment)?,
                    ..initial_ref(4, embodiment.schema_version())
                },
                lifecycle_persistence: SubsystemStateRef {
                    content_digest: lifecycle_persistence_digest(
                        organism_id,
                        world_entity_id,
                        birth_tick,
                        OrganismLifecycle::Alive,
                        SleepPhase::Awake,
                        biochemistry.tick,
                        0,
                        0,
                        &OrganismArchiveIdentity::default(),
                    ),
                    ..initial_ref(5, SchemaVersions::CURRENT.save.raw())
                },
            },
            embodiment,
            birth_tick,
            lifecycle: OrganismLifecycle::Alive,
            sleep_phase: SleepPhase::Awake,
            sleep_phase_tick: biochemistry.tick,
            sleep_cycle_id: 0,
            sleep_work_units: 0,
            archive: OrganismArchiveIdentity::default(),
            cognitive_work: CognitiveWorkReceipt::zero(),
            cognitive_energy_debit: 0.0,
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
        self.state_graph.validate_contract()?;
        self.embodiment.validate_contract()?;
        if self.state_graph.organism_id != self.organism_id
            || self.embodiment.entity_id() != self.world_entity_id
            || self.state_graph.body_biochemistry.causal_tick != self.biochemistry.tick
            || self.state_graph.embodiment.revision != self.embodiment.revision()
            || self.state_graph.embodiment.causal_tick != self.embodiment.source_tick()
            || self.state_graph.body_biochemistry.content_digest
                != body_biochemistry_digest(&self.biochemistry)?
            || self.state_graph.embodiment.content_digest != embodiment_digest(&self.embodiment)?
            || self.state_graph.genetics_development.content_digest
                != genetics_development_digest(&self.genome, &self.phenotype)?
            || self.state_graph.lifecycle_persistence.content_digest
                != lifecycle_persistence_digest(
                    self.organism_id,
                    self.world_entity_id,
                    self.birth_tick,
                    self.lifecycle,
                    self.sleep_phase,
                    self.sleep_phase_tick,
                    self.sleep_cycle_id,
                    self.sleep_work_units,
                    &self.archive,
                )
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
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
        self.cognitive_work.validate_contract()?;
        if !self.cognitive_energy_debit.is_finite() || self.cognitive_energy_debit < 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let expected_age = self.age_at(self.biochemistry.tick)?;
        if self.biochemistry.development.age_ticks != expected_age {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        if self.birth_tick.raw() > self.biochemistry.tick.raw() {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        if self.sleep_phase_tick.raw() > self.biochemistry.tick.raw() {
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

    pub fn account_cognitive_work(
        &mut self,
        receipt: CognitiveWorkReceipt,
        policy: CognitiveWorkCostPolicy,
    ) -> Result<f32, OrganismRegistryError> {
        if !self.lifecycle.is_alive() {
            return Err(OrganismRegistryError::DeadOrganism(self.organism_id));
        }
        self.validate_contract()?;
        receipt.validate_contract()?;
        let requested_debit = policy.energy_debit(&receipt)?;
        let original_biochemistry = self.biochemistry;
        let original_state_graph = self.state_graph.clone();
        let original_work = self.cognitive_work;
        let original_debit = self.cognitive_energy_debit;
        let applied_debit = requested_debit.min(self.biochemistry.body.energy);

        self.biochemistry
            .body
            .set_energy(self.biochemistry.body.energy - applied_debit)?;
        self.advance_body_state_ref()?;
        self.cognitive_work = receipt;
        self.cognitive_energy_debit = applied_debit;
        if let Err(error) = self.validate_contract() {
            self.biochemistry = original_biochemistry;
            self.state_graph = original_state_graph;
            self.cognitive_work = original_work;
            self.cognitive_energy_debit = original_debit;
            return Err(error.into());
        }
        Ok(applied_debit)
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
        let original_state_graph = self.state_graph.clone();
        self.validate_contract()?;
        let next_age = self.age_at(next_tick)?;
        let next = self
            .biochemistry
            .advance_with_age(next_tick, next_age, event, &self.phenotype)
            .map_err(OrganismRegistryError::InvalidRecord)?;
        self.biochemistry = next;
        self.advance_body_state_ref()?;
        if let Err(error) = self.validate_contract() {
            self.biochemistry = original_biochemistry;
            self.state_graph = original_state_graph;
            return Err(error.into());
        }
        Ok(())
    }

    pub fn advance_biology_with_neural_emission(
        &mut self,
        next_tick: Tick,
        event: BodyEventDelta,
        neural: &NeuralEmissionFrame,
    ) -> Result<(), OrganismRegistryError> {
        if !self.lifecycle.is_alive() {
            return Err(OrganismRegistryError::DeadOrganism(self.organism_id));
        }
        let original_biochemistry = self.biochemistry;
        let original_state_graph = self.state_graph.clone();
        self.validate_contract()?;
        let next_age = self.age_at(next_tick)?;
        self.biochemistry = self
            .biochemistry
            .advance_with_neural_emission(next_tick, next_age, event, Some(neural), &self.phenotype)
            .map_err(OrganismRegistryError::InvalidRecord)?;
        self.advance_body_state_ref()?;
        if let Err(error) = self.validate_contract() {
            self.biochemistry = original_biochemistry;
            self.state_graph = original_state_graph;
            return Err(error.into());
        }
        Ok(())
    }

    fn advance_body_state_ref(&mut self) -> Result<(), ScaffoldContractError> {
        let content_digest = body_biochemistry_digest(&self.biochemistry)?;
        self.state_graph.transaction_revision = self
            .state_graph
            .transaction_revision
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let state = &mut self.state_graph.body_biochemistry;
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        state.causal_tick = self.biochemistry.tick;
        state.content_digest = content_digest;
        Ok(())
    }

    fn advance_lifecycle_persistence_ref(&mut self) -> Result<(), ScaffoldContractError> {
        let content_digest = lifecycle_persistence_digest(
            self.organism_id,
            self.world_entity_id,
            self.birth_tick,
            self.lifecycle,
            self.sleep_phase,
            self.sleep_phase_tick,
            self.sleep_cycle_id,
            self.sleep_work_units,
            &self.archive,
        );
        self.state_graph.transaction_revision = self
            .state_graph
            .transaction_revision
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let state = &mut self.state_graph.lifecycle_persistence;
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        state.causal_tick = self.biochemistry.tick;
        state.content_digest = content_digest;
        Ok(())
    }

    pub fn replace_embodiment_state(
        &mut self,
        candidate: EmbodimentState,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_contract()?;
        candidate.validate_contract()?;
        if candidate.entity_id() != self.world_entity_id
            || candidate.revision() != self.embodiment.revision().saturating_add(1)
            || candidate.source_tick().raw() > self.biochemistry.tick.raw()
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let original_embodiment = self.embodiment.clone();
        let original_graph = self.state_graph.clone();
        self.embodiment = candidate;
        self.state_graph.transaction_revision = self
            .state_graph
            .transaction_revision
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        self.state_graph.embodiment.revision = self.embodiment.revision();
        self.state_graph.embodiment.causal_tick = self.embodiment.source_tick();
        self.state_graph.embodiment.content_digest = embodiment_digest(&self.embodiment)?;
        if let Err(error) = self.validate_contract() {
            self.embodiment = original_embodiment;
            self.state_graph = original_graph;
            return Err(error);
        }
        Ok(())
    }

    pub fn seal_cognitive_subsystems(
        &mut self,
        tick: Tick,
        brain_digest: [u64; 4],
        memory_digest: [u64; 4],
    ) -> Result<(), ScaffoldContractError> {
        self.validate_contract()?;
        if tick != self.biochemistry.tick || brain_digest == [0; 4] || memory_digest == [0; 4] {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let original = self.state_graph.clone();
        self.state_graph.transaction_revision = self
            .state_graph
            .transaction_revision
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        for (subsystem, digest) in [
            (&mut self.state_graph.brain, brain_digest),
            (&mut self.state_graph.memory, memory_digest),
        ] {
            subsystem.revision = subsystem
                .revision
                .checked_add(1)
                .ok_or(ScaffoldContractError::InvalidId)?;
            subsystem.causal_tick = tick;
            subsystem.content_digest = digest;
        }
        if let Err(error) = self.validate_contract() {
            self.state_graph = original;
            return Err(error);
        }
        Ok(())
    }

    /// Advances world biology once for a causal tick. Repeated scheduling of
    /// the same tick is an intentional no-op, so sleep cannot double-charge
    /// recovery or biological time.
    pub fn advance_biology_once(
        &mut self,
        next_tick: Tick,
        event: BodyEventDelta,
    ) -> Result<bool, ScaffoldContractError> {
        self.validate_contract()?;
        if !self.lifecycle.is_alive() {
            return Err(ScaffoldContractError::InvalidId);
        }
        if self.biochemistry.tick == next_tick {
            return Ok(false);
        }
        self.advance_biology(next_tick, event)
            .map(|_| true)
            .map_err(map_organism_error)
    }

    /// Seals the current sleep phase into the authoritative organism record.
    /// Presentation and persistence consumers can read this boundary without
    /// consulting a renderer or a second biological clock.
    pub fn seal_sleep_phase(
        &mut self,
        phase: SleepPhase,
        cycle_id: u64,
        tick: Tick,
        work_units: u64,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_contract()?;
        if !self.lifecycle.is_alive()
            || tick != self.biochemistry.tick
            || tick.raw() < self.sleep_phase_tick.raw()
            || (phase != SleepPhase::Awake && cycle_id == 0)
            || cycle_id < self.sleep_cycle_id
        {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        let original = (
            self.sleep_phase,
            self.sleep_phase_tick,
            self.sleep_cycle_id,
            self.sleep_work_units,
        );
        self.sleep_phase = phase;
        self.sleep_phase_tick = tick;
        self.sleep_cycle_id = cycle_id;
        self.sleep_work_units = self.sleep_work_units.saturating_add(work_units);
        let original_graph = self.state_graph.clone();
        if let Err(error) = self.advance_lifecycle_persistence_ref() {
            (
                self.sleep_phase,
                self.sleep_phase_tick,
                self.sleep_cycle_id,
                self.sleep_work_units,
            ) = original;
            self.state_graph = original_graph;
            return Err(error);
        }
        if let Err(error) = self.validate_contract() {
            (
                self.sleep_phase,
                self.sleep_phase_tick,
                self.sleep_cycle_id,
                self.sleep_work_units,
            ) = original;
            self.state_graph = original_graph;
            return Err(error);
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
        let original_graph = self.state_graph.clone();
        self.lifecycle = OrganismLifecycle::Dead { death_tick };
        if let Err(error) = self.advance_lifecycle_persistence_ref() {
            self.lifecycle = original_lifecycle;
            self.state_graph = original_graph;
            return Err(error.into());
        }
        if let Err(error) = self.validate_contract() {
            self.lifecycle = original_lifecycle;
            self.state_graph = original_graph;
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
        let original_graph = self.state_graph.clone();
        self.archive.birth_manifest_digest = Some(digest);
        if let Err(error) = self.advance_lifecycle_persistence_ref() {
            self.archive = original_archive;
            self.state_graph = original_graph;
            return Err(error.into());
        }
        if let Err(error) = self.validate_contract() {
            self.archive = original_archive;
            self.state_graph = original_graph;
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
        let original_graph = self.state_graph.clone();
        self.archive.life_manifest_digest = Some(digest);
        if let Err(error) = self.advance_lifecycle_persistence_ref() {
            self.archive = original_archive;
            self.state_graph = original_graph;
            return Err(error.into());
        }
        if let Err(error) = self.validate_contract() {
            self.archive = original_archive;
            self.state_graph = original_graph;
            return Err(error.into());
        }
        Ok(())
    }
}

fn map_organism_error(error: OrganismRegistryError) -> ScaffoldContractError {
    match error {
        OrganismRegistryError::InvalidRecord(error) => error,
        OrganismRegistryError::UnknownOrganism(_)
        | OrganismRegistryError::DuplicateOrganism(_)
        | OrganismRegistryError::DuplicateWorldEntity(_)
        | OrganismRegistryError::AlreadyDead(_)
        | OrganismRegistryError::InvalidDeathTick(_)
        | OrganismRegistryError::DeadOrganism(_)
        | OrganismRegistryError::BirthManifestAlreadyLinked(_)
        | OrganismRegistryError::LifeManifestAlreadyLinked(_)
        | OrganismRegistryError::LifeManifestRequiresBirth(_)
        | OrganismRegistryError::LifeManifestRequiresDead(_)
        | OrganismRegistryError::LifeManifestNotLinked(_)
        | OrganismRegistryError::IndexMismatch => ScaffoldContractError::InvalidId,
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

    pub(crate) fn replace_existing_exact(
        &mut self,
        replacement: WorldOrganismRecord,
    ) -> Result<(), OrganismRegistryError> {
        replacement.validate_contract()?;
        let organism_id = replacement.organism_id;
        let world_entity_id = replacement.world_entity_id;
        let current = self
            .records_by_organism
            .get(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?;
        if current.world_entity_id != world_entity_id
            || self.organism_by_world_entity.get(&world_entity_id.raw()) != Some(&organism_id)
        {
            return Err(OrganismRegistryError::IndexMismatch);
        }

        self.records_by_organism
            .insert(organism_id.raw(), replacement);
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
        let original_state_graph = record.state_graph.clone();
        let result = mutate(&mut record.biochemistry);
        match result {
            Err(error) => {
                record.biochemistry = original_biochemistry;
                record.state_graph = original_state_graph;
                Err(error)
            }
            Ok(value) => {
                let valid_tick = record.biochemistry.tick.raw() >= original_biochemistry.tick.raw();
                if !valid_tick {
                    record.biochemistry = original_biochemistry;
                    record.state_graph = original_state_graph;
                    return Err(OrganismRegistryError::InvalidRecord(
                        ScaffoldContractError::NonMonotonicTick,
                    ));
                }
                if let Err(error) = record
                    .advance_body_state_ref()
                    .and_then(|_| record.validate_contract())
                {
                    record.biochemistry = original_biochemistry;
                    record.state_graph = original_state_graph;
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
        self.records_by_organism
            .get_mut(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?
            .advance_biology(next_tick, event)
    }

    pub fn advance_biology_with_neural_emission(
        &mut self,
        organism_id: OrganismId,
        next_tick: Tick,
        event: BodyEventDelta,
        neural: &NeuralEmissionFrame,
    ) -> Result<(), OrganismRegistryError> {
        self.records_by_organism
            .get_mut(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?
            .advance_biology_with_neural_emission(next_tick, event, neural)
    }

    pub fn account_cognitive_work(
        &mut self,
        organism_id: OrganismId,
        receipt: CognitiveWorkReceipt,
        policy: CognitiveWorkCostPolicy,
    ) -> Result<f32, OrganismRegistryError> {
        self.records_by_organism
            .get_mut(&organism_id.raw())
            .ok_or(OrganismRegistryError::UnknownOrganism(organism_id))?
            .account_cognitive_work(receipt, policy)
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
        if self
            .organism_by_world_entity
            .get(&record.world_entity_id().raw())
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
            biology.body.set_energy(0.0)?;
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
