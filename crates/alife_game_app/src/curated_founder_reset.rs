use std::collections::HashSet;

use alife_core::{
    Blake3Digest, BrainCapacityClass, CanonicalDigestBuilder, CreatureGenome,
    FoundationGeneticIdentity, FoundationWeightAsset, GenomeId, LineageId, OrganismId,
    SensorProfile, Tick, Validate, WorldEntityId,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const CURATED_FOUNDER_RESET_POLICY: &str = "curated-founder-reset:v1";
pub const CURATED_FOUNDER_SEED_DOMAIN_VERSION: &str = "curated-founder-reset-seed:v1";
pub const CURATED_FOUNDER_RECEIPT_DOMAIN_VERSION: &str = "curated-founder-reset-receipt:v1";

const CURATED_FOUNDER_SEED_DOMAIN: &[u8] = b"alife.curated-founder-reset.seed.v1";
const CURATED_FOUNDER_RECEIPT_DOMAIN: &[u8] = b"alife.curated-founder-reset.receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
pub enum CuratedFounderResetError {
    #[error("the exact curated founder reset policy is missing")]
    MissingExactPolicy,
    #[error("the curated founder reset policy is not exact: {label}")]
    PolicyMismatch { label: String },
    #[error("a required source identity is empty")]
    MissingSourceIdentity,
    #[error("the source or world seed is zero")]
    InvalidSourceSeed,
    #[error("the target population must be nonzero")]
    ZeroTargetPopulation,
    #[error("final Agent count {actual} does not equal target population {expected}")]
    AgentCountMismatch { expected: u32, actual: usize },
    #[error("a final Agent world entity ID is zero")]
    ZeroWorldEntityId,
    #[error("final Agent world entity IDs contain a duplicate")]
    DuplicateWorldEntityId,
    #[error("a final Agent organism identity is missing")]
    MissingOrganismId,
    #[error("a final Agent organism ID is zero")]
    ZeroOrganismId,
    #[error("final Agent organism IDs contain a duplicate")]
    DuplicateOrganismId,
    #[error("final population slots contain a duplicate")]
    DuplicatePopulationSlot,
    #[error("final population slots are not exactly 0..target_population")]
    PopulationSlotGap,
    #[error("a legacy genome ID was presented as a founder source")]
    LegacyGenomeSource,
    #[error("the checked Nano512 foundation identity or digest does not match")]
    FoundationMismatch,
    #[error("derived conception seeds contain a duplicate")]
    DuplicateConceptionSeed,
    #[error("a derived conception seed is zero")]
    ZeroConceptionSeed,
    #[error("derived founder genome identities contain a duplicate")]
    DuplicateGenomeId,
    #[error("a derived founder genome identity is zero")]
    ZeroGenomeId,
    #[error("derived founder lineage identities contain a duplicate")]
    DuplicateLineageId,
    #[error("a derived founder lineage identity is zero")]
    ZeroLineageId,
    #[error("serialized founder entries are not in canonical slot order")]
    NonCanonicalPopulationOrder,
    #[error("a conception seed does not match serialized provenance and identity")]
    ConceptionSeedMismatch,
    #[error("the derived founder genome failed its contract")]
    FounderGenomeContract,
    #[error("the serialized founder receipt digest is invalid")]
    ReceiptDigestMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratedFounderAgentInput {
    pub world_entity_id: WorldEntityId,
    pub organism_id: Option<OrganismId>,
    pub final_population_slot: u32,
    pub legacy_genome_id: Option<GenomeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratedFounderResetRequest {
    pub policy_label: Option<String>,
    pub source_save_identity: String,
    pub source_save_label: String,
    pub source_save_seed: u64,
    pub world_seed: u64,
    pub restored_tick: Tick,
    pub target_population: u32,
    pub sensor_profile: SensorProfile,
    pub foundation: FoundationGeneticIdentity,
    pub foundation_content_digest: Blake3Digest,
    pub source_run_identity: String,
    pub final_agents: Vec<CuratedFounderAgentInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratedFounderPlanEntry {
    pub final_population_slot: u32,
    pub world_entity_id: WorldEntityId,
    pub organism_id: OrganismId,
    pub conception_seed: u64,
    pub genome_id: GenomeId,
    pub lineage_id: LineageId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratedFounderAgentIdentity {
    pub final_population_slot: u32,
    pub world_entity_id: WorldEntityId,
    pub organism_id: OrganismId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratedFounderResetReceipt {
    pub policy_label: String,
    pub source_save_identity: String,
    pub source_save_label: String,
    pub source_save_seed: u64,
    pub world_seed: u64,
    pub restored_tick: Tick,
    pub target_population: u32,
    pub seed_domain_version: String,
    pub sensor_profile: SensorProfile,
    pub foundation: FoundationGeneticIdentity,
    pub foundation_content_digest: Blake3Digest,
    pub source_run_identity: String,
    pub ordered_agent_identities: Vec<CuratedFounderAgentIdentity>,
    pub derived_conception_seeds: Vec<u64>,
    pub derived_genome_ids: Vec<GenomeId>,
    pub derived_lineage_ids: Vec<LineageId>,
    pub receipt_digest: [u64; 4],
}

struct CuratedFounderValidation<'a> {
    policy_label: &'a str,
    source_save_identity: &'a str,
    source_save_label: &'a str,
    source_save_seed: u64,
    world_seed: u64,
    restored_tick: Tick,
    target_population: u32,
    seed_domain_version: &'a str,
    sensor_profile: SensorProfile,
    foundation: FoundationGeneticIdentity,
    foundation_content_digest: Blake3Digest,
    source_run_identity: &'a str,
    entries: &'a [CuratedFounderPlanEntry],
}

fn validate_curated_founder_contract(
    validation: CuratedFounderValidation<'_>,
) -> Result<(), CuratedFounderResetError> {
    validate_policy(Some(validation.policy_label))?;
    if validation.seed_domain_version != CURATED_FOUNDER_SEED_DOMAIN_VERSION {
        return Err(CuratedFounderResetError::ReceiptDigestMismatch);
    }
    if validation.source_save_identity.trim().is_empty()
        || validation.source_save_label.trim().is_empty()
        || validation.source_run_identity.trim().is_empty()
    {
        return Err(CuratedFounderResetError::MissingSourceIdentity);
    }
    if validation.source_save_seed == 0 || validation.world_seed == 0 {
        return Err(CuratedFounderResetError::InvalidSourceSeed);
    }
    if validation.target_population == 0 {
        return Err(CuratedFounderResetError::ZeroTargetPopulation);
    }
    if validation.entries.len() != validation.target_population as usize {
        return Err(CuratedFounderResetError::AgentCountMismatch {
            expected: validation.target_population,
            actual: validation.entries.len(),
        });
    }
    validate_checked_foundation(
        validation.sensor_profile,
        validation.foundation,
        validation.foundation_content_digest,
    )?;

    let mut world_entities = HashSet::with_capacity(validation.entries.len());
    let mut organisms = HashSet::with_capacity(validation.entries.len());
    let mut slots = HashSet::with_capacity(validation.entries.len());
    let mut conception_seeds = HashSet::with_capacity(validation.entries.len());
    let mut genome_ids = HashSet::with_capacity(validation.entries.len());
    let mut lineage_ids = HashSet::with_capacity(validation.entries.len());

    for (expected_slot, entry) in validation.entries.iter().enumerate() {
        let expected_slot = expected_slot as u32;
        if !slots.insert(entry.final_population_slot) {
            return Err(CuratedFounderResetError::DuplicatePopulationSlot);
        }
        if entry.final_population_slot != expected_slot {
            return Err(CuratedFounderResetError::NonCanonicalPopulationOrder);
        }
        if !entry.world_entity_id.is_valid() {
            return Err(CuratedFounderResetError::ZeroWorldEntityId);
        }
        if !entry.organism_id.is_valid() {
            return Err(CuratedFounderResetError::ZeroOrganismId);
        }
        if !world_entities.insert(entry.world_entity_id.raw()) {
            return Err(CuratedFounderResetError::DuplicateWorldEntityId);
        }
        if !organisms.insert(entry.organism_id.raw()) {
            return Err(CuratedFounderResetError::DuplicateOrganismId);
        }
        if entry.conception_seed == 0 {
            return Err(CuratedFounderResetError::ZeroConceptionSeed);
        }
        if !conception_seeds.insert(entry.conception_seed) {
            return Err(CuratedFounderResetError::DuplicateConceptionSeed);
        }
        if !entry.genome_id.is_valid() {
            return Err(CuratedFounderResetError::ZeroGenomeId);
        }
        if !genome_ids.insert(entry.genome_id.raw()) {
            return Err(CuratedFounderResetError::DuplicateGenomeId);
        }
        if !entry.lineage_id.is_valid() {
            return Err(CuratedFounderResetError::ZeroLineageId);
        }
        if !lineage_ids.insert(entry.lineage_id.raw()) {
            return Err(CuratedFounderResetError::DuplicateLineageId);
        }

        let expected_seed = derive_conception_seed(
            validation.policy_label,
            validation.seed_domain_version,
            validation.source_save_identity,
            validation.source_save_label,
            validation.source_run_identity,
            validation.source_save_seed,
            validation.world_seed,
            validation.restored_tick,
            validation.target_population,
            validation.sensor_profile,
            validation.foundation,
            validation.foundation_content_digest,
            entry.final_population_slot,
            entry.world_entity_id,
            entry.organism_id,
        );
        if expected_seed != entry.conception_seed {
            return Err(CuratedFounderResetError::ConceptionSeedMismatch);
        }

        let genome =
            CreatureGenome::early_mammal_founder(entry.conception_seed, validation.foundation)
                .map_err(|_| CuratedFounderResetError::FounderGenomeContract)?;
        if genome.id != entry.genome_id
            || genome.lineage_id != entry.lineage_id
            || !genome.parent_genome_ids.is_empty()
            || genome.provenance.ordinary_birth
            || !genome.provenance.recombination.is_empty()
            || !genome.provenance.mutations.is_empty()
        {
            return Err(CuratedFounderResetError::FounderGenomeContract);
        }
    }

    Ok(())
}

impl CuratedFounderResetReceipt {
    pub fn validate(&self) -> Result<(), CuratedFounderResetError> {
        if self.target_population == 0 {
            return Err(CuratedFounderResetError::ZeroTargetPopulation);
        }
        let expected_len = self.target_population as usize;
        if self.ordered_agent_identities.len() != expected_len
            || self.derived_conception_seeds.len() != expected_len
            || self.derived_genome_ids.len() != expected_len
            || self.derived_lineage_ids.len() != expected_len
        {
            return Err(CuratedFounderResetError::AgentCountMismatch {
                expected: self.target_population,
                actual: self.ordered_agent_identities.len(),
            });
        }

        let entries = self
            .ordered_agent_identities
            .iter()
            .zip(&self.derived_conception_seeds)
            .zip(&self.derived_genome_ids)
            .zip(&self.derived_lineage_ids)
            .map(
                |(((identity, conception_seed), genome_id), lineage_id)| CuratedFounderPlanEntry {
                    final_population_slot: identity.final_population_slot,
                    world_entity_id: identity.world_entity_id,
                    organism_id: identity.organism_id,
                    conception_seed: *conception_seed,
                    genome_id: *genome_id,
                    lineage_id: *lineage_id,
                },
            )
            .collect::<Vec<_>>();
        validate_curated_founder_contract(CuratedFounderValidation {
            policy_label: &self.policy_label,
            source_save_identity: &self.source_save_identity,
            source_save_label: &self.source_save_label,
            source_save_seed: self.source_save_seed,
            world_seed: self.world_seed,
            restored_tick: self.restored_tick,
            target_population: self.target_population,
            seed_domain_version: &self.seed_domain_version,
            sensor_profile: self.sensor_profile,
            foundation: self.foundation,
            foundation_content_digest: self.foundation_content_digest,
            source_run_identity: &self.source_run_identity,
            entries: &entries,
        })?;

        if self.receipt_digest == [0; 4] {
            return Err(CuratedFounderResetError::ReceiptDigestMismatch);
        }
        if self.receipt_digest != self.recompute_digest() {
            return Err(CuratedFounderResetError::ReceiptDigestMismatch);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(CURATED_FOUNDER_RECEIPT_DOMAIN);
        digest.write_utf8(&self.policy_label);
        digest.write_utf8(&self.source_save_identity);
        digest.write_utf8(&self.source_save_label);
        digest.write_u64(self.source_save_seed);
        digest.write_u64(self.world_seed);
        digest.write_u64(self.restored_tick.raw());
        digest.write_u32(self.target_population);
        digest.write_utf8(&self.seed_domain_version);
        digest.write_u16(self.sensor_profile.raw());
        write_foundation_identity(&mut digest, self.foundation);
        digest.write_bytes(self.foundation_content_digest.bytes());
        digest.write_utf8(&self.source_run_identity);
        digest.write_sequence_len(self.ordered_agent_identities.len());
        for identity in &self.ordered_agent_identities {
            digest.write_u32(identity.final_population_slot);
            digest.write_u64(identity.world_entity_id.raw());
            digest.write_u64(identity.organism_id.raw());
        }
        digest.write_sequence_len(self.derived_conception_seeds.len());
        for seed in &self.derived_conception_seeds {
            digest.write_u64(*seed);
        }
        digest.write_sequence_len(self.derived_genome_ids.len());
        for genome_id in &self.derived_genome_ids {
            digest.write_u64(genome_id.raw());
        }
        digest.write_sequence_len(self.derived_lineage_ids.len());
        for lineage_id in &self.derived_lineage_ids {
            digest.write_u64(lineage_id.raw());
        }
        nonzero_canonical_digest(digest.finish256())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CuratedFounderPlan {
    pub policy_label: String,
    pub source_save_identity: String,
    pub source_save_label: String,
    pub source_save_seed: u64,
    pub world_seed: u64,
    pub restored_tick: Tick,
    pub target_population: u32,
    pub seed_domain_version: String,
    pub sensor_profile: SensorProfile,
    pub foundation: FoundationGeneticIdentity,
    pub foundation_content_digest: Blake3Digest,
    pub source_run_identity: String,
    pub entries: Vec<CuratedFounderPlanEntry>,
    pub receipt: CuratedFounderResetReceipt,
}

impl CuratedFounderPlan {
    pub fn validate(&self) -> Result<(), CuratedFounderResetError> {
        validate_curated_founder_contract(CuratedFounderValidation {
            policy_label: &self.policy_label,
            source_save_identity: &self.source_save_identity,
            source_save_label: &self.source_save_label,
            source_save_seed: self.source_save_seed,
            world_seed: self.world_seed,
            restored_tick: self.restored_tick,
            target_population: self.target_population,
            seed_domain_version: &self.seed_domain_version,
            sensor_profile: self.sensor_profile,
            foundation: self.foundation,
            foundation_content_digest: self.foundation_content_digest,
            source_run_identity: &self.source_run_identity,
            entries: &self.entries,
        })?;

        self.receipt.validate()?;
        if self.receipt.policy_label != self.policy_label
            || self.receipt.source_save_identity != self.source_save_identity
            || self.receipt.source_save_label != self.source_save_label
            || self.receipt.source_save_seed != self.source_save_seed
            || self.receipt.world_seed != self.world_seed
            || self.receipt.restored_tick != self.restored_tick
            || self.receipt.target_population != self.target_population
            || self.receipt.seed_domain_version != self.seed_domain_version
            || self.receipt.sensor_profile != self.sensor_profile
            || self.receipt.foundation != self.foundation
            || self.receipt.foundation_content_digest != self.foundation_content_digest
            || self.receipt.source_run_identity != self.source_run_identity
            || self.receipt.ordered_agent_identities.len() != self.entries.len()
        {
            return Err(CuratedFounderResetError::ReceiptDigestMismatch);
        }
        for (entry, identity) in self
            .entries
            .iter()
            .zip(&self.receipt.ordered_agent_identities)
        {
            if entry.final_population_slot != identity.final_population_slot
                || entry.world_entity_id != identity.world_entity_id
                || entry.organism_id != identity.organism_id
            {
                return Err(CuratedFounderResetError::ReceiptDigestMismatch);
            }
        }
        if self.receipt.derived_conception_seeds
            != self
                .entries
                .iter()
                .map(|entry| entry.conception_seed)
                .collect::<Vec<_>>()
            || self.receipt.derived_genome_ids
                != self
                    .entries
                    .iter()
                    .map(|entry| entry.genome_id)
                    .collect::<Vec<_>>()
            || self.receipt.derived_lineage_ids
                != self
                    .entries
                    .iter()
                    .map(|entry| entry.lineage_id)
                    .collect::<Vec<_>>()
        {
            return Err(CuratedFounderResetError::ReceiptDigestMismatch);
        }
        Ok(())
    }
}

pub fn plan_curated_founder_reset(
    request: &CuratedFounderResetRequest,
) -> Result<CuratedFounderPlan, CuratedFounderResetError> {
    validate_policy(request.policy_label.as_deref())?;
    if request.source_save_identity.trim().is_empty()
        || request.source_save_label.trim().is_empty()
        || request.source_run_identity.trim().is_empty()
    {
        return Err(CuratedFounderResetError::MissingSourceIdentity);
    }
    if request.source_save_seed == 0 || request.world_seed == 0 {
        return Err(CuratedFounderResetError::InvalidSourceSeed);
    }
    if request.target_population == 0 {
        return Err(CuratedFounderResetError::ZeroTargetPopulation);
    }
    if request.final_agents.len() != request.target_population as usize {
        return Err(CuratedFounderResetError::AgentCountMismatch {
            expected: request.target_population,
            actual: request.final_agents.len(),
        });
    }
    validate_checked_foundation(
        request.sensor_profile,
        request.foundation,
        request.foundation_content_digest,
    )?;

    let mut world_entities = HashSet::with_capacity(request.final_agents.len());
    let mut organisms = HashSet::with_capacity(request.final_agents.len());
    let mut slots = HashSet::with_capacity(request.final_agents.len());
    for agent in &request.final_agents {
        if agent.legacy_genome_id.is_some() {
            return Err(CuratedFounderResetError::LegacyGenomeSource);
        }
        if !agent.world_entity_id.is_valid() {
            return Err(CuratedFounderResetError::ZeroWorldEntityId);
        }
        let organism_id = agent
            .organism_id
            .ok_or(CuratedFounderResetError::MissingOrganismId)?;
        if !organism_id.is_valid() {
            return Err(CuratedFounderResetError::MissingOrganismId);
        }
        if !world_entities.insert(agent.world_entity_id.raw()) {
            return Err(CuratedFounderResetError::DuplicateWorldEntityId);
        }
        if !organisms.insert(organism_id.raw()) {
            return Err(CuratedFounderResetError::DuplicateOrganismId);
        }
        if !slots.insert(agent.final_population_slot) {
            return Err(CuratedFounderResetError::DuplicatePopulationSlot);
        }
    }

    let mut ordered_agents = request.final_agents.iter().collect::<Vec<_>>();
    ordered_agents.sort_by_key(|agent| agent.final_population_slot);
    for (expected_slot, agent) in (0..request.target_population).zip(&ordered_agents) {
        if expected_slot != agent.final_population_slot {
            return Err(CuratedFounderResetError::PopulationSlotGap);
        }
    }

    let mut entries = Vec::with_capacity(ordered_agents.len());
    let mut conception_seeds = HashSet::with_capacity(ordered_agents.len());
    let mut genome_ids = HashSet::with_capacity(ordered_agents.len());
    let mut lineage_ids = HashSet::with_capacity(ordered_agents.len());
    for agent in ordered_agents {
        let organism_id = agent.organism_id.expect("validated organism identity");
        let conception_seed = derive_conception_seed(
            CURATED_FOUNDER_RESET_POLICY,
            CURATED_FOUNDER_SEED_DOMAIN_VERSION,
            &request.source_save_identity,
            &request.source_save_label,
            &request.source_run_identity,
            request.source_save_seed,
            request.world_seed,
            request.restored_tick,
            request.target_population,
            request.sensor_profile,
            request.foundation,
            request.foundation_content_digest,
            agent.final_population_slot,
            agent.world_entity_id,
            organism_id,
        );
        if !conception_seeds.insert(conception_seed) {
            return Err(CuratedFounderResetError::DuplicateConceptionSeed);
        }
        let genome = CreatureGenome::early_mammal_founder(conception_seed, request.foundation)
            .map_err(|_| CuratedFounderResetError::FounderGenomeContract)?;
        if !genome.parent_genome_ids.is_empty()
            || genome.provenance.ordinary_birth
            || !genome.provenance.recombination.is_empty()
            || !genome.provenance.mutations.is_empty()
        {
            return Err(CuratedFounderResetError::FounderGenomeContract);
        }
        if !genome_ids.insert(genome.id.raw()) {
            return Err(CuratedFounderResetError::DuplicateGenomeId);
        }
        if !lineage_ids.insert(genome.lineage_id.raw()) {
            return Err(CuratedFounderResetError::DuplicateLineageId);
        }
        entries.push(CuratedFounderPlanEntry {
            final_population_slot: agent.final_population_slot,
            world_entity_id: agent.world_entity_id,
            organism_id,
            conception_seed,
            genome_id: genome.id,
            lineage_id: genome.lineage_id,
        });
    }

    let receipt = CuratedFounderResetReceipt {
        policy_label: CURATED_FOUNDER_RESET_POLICY.to_string(),
        source_save_identity: request.source_save_identity.clone(),
        source_save_label: request.source_save_label.clone(),
        source_save_seed: request.source_save_seed,
        world_seed: request.world_seed,
        restored_tick: request.restored_tick,
        target_population: request.target_population,
        seed_domain_version: CURATED_FOUNDER_SEED_DOMAIN_VERSION.to_string(),
        sensor_profile: request.sensor_profile,
        foundation: request.foundation,
        foundation_content_digest: request.foundation_content_digest,
        source_run_identity: request.source_run_identity.clone(),
        ordered_agent_identities: entries
            .iter()
            .map(|entry| CuratedFounderAgentIdentity {
                final_population_slot: entry.final_population_slot,
                world_entity_id: entry.world_entity_id,
                organism_id: entry.organism_id,
            })
            .collect(),
        derived_conception_seeds: entries.iter().map(|entry| entry.conception_seed).collect(),
        derived_genome_ids: entries.iter().map(|entry| entry.genome_id).collect(),
        derived_lineage_ids: entries.iter().map(|entry| entry.lineage_id).collect(),
        receipt_digest: [0; 4],
    };
    let receipt = CuratedFounderResetReceipt {
        receipt_digest: receipt.recompute_digest(),
        ..receipt
    };
    let plan = CuratedFounderPlan {
        policy_label: CURATED_FOUNDER_RESET_POLICY.to_string(),
        source_save_identity: request.source_save_identity.clone(),
        source_save_label: request.source_save_label.clone(),
        source_save_seed: request.source_save_seed,
        world_seed: request.world_seed,
        restored_tick: request.restored_tick,
        target_population: request.target_population,
        seed_domain_version: CURATED_FOUNDER_SEED_DOMAIN_VERSION.to_string(),
        sensor_profile: request.sensor_profile,
        foundation: request.foundation,
        foundation_content_digest: request.foundation_content_digest,
        source_run_identity: request.source_run_identity.clone(),
        entries,
        receipt,
    };
    plan.validate()?;
    Ok(plan)
}

fn validate_policy(policy_label: Option<&str>) -> Result<(), CuratedFounderResetError> {
    match policy_label {
        None => Err(CuratedFounderResetError::MissingExactPolicy),
        Some(label) if label == CURATED_FOUNDER_RESET_POLICY => Ok(()),
        Some(label) if label.trim().is_empty() => Err(CuratedFounderResetError::MissingExactPolicy),
        Some(label) => Err(CuratedFounderResetError::PolicyMismatch {
            label: label.to_string(),
        }),
    }
}

fn validate_checked_foundation(
    sensor_profile: SensorProfile,
    foundation: FoundationGeneticIdentity,
    foundation_content_digest: Blake3Digest,
) -> Result<(), CuratedFounderResetError> {
    if foundation.validate_contract().is_err()
        || foundation.brain_class_id != BrainCapacityClass::N512_ID
    {
        return Err(CuratedFounderResetError::FoundationMismatch);
    }
    let asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile)
        .map_err(|_| CuratedFounderResetError::FoundationMismatch)?;
    let manifest = asset.manifest();
    if foundation.foundation_id != manifest.foundation_id().raw()
        || u32::from(foundation.version) != manifest.foundation_version().raw()
        || foundation.compatibility_family_id != manifest.compatibility_family_id().raw()
        || foundation_content_digest != asset.digest()
    {
        return Err(CuratedFounderResetError::FoundationMismatch);
    }
    Ok(())
}

fn derive_conception_seed(
    policy_label: &str,
    seed_domain_version: &str,
    source_save_identity: &str,
    source_save_label: &str,
    source_run_identity: &str,
    source_save_seed: u64,
    world_seed: u64,
    restored_tick: Tick,
    target_population: u32,
    sensor_profile: SensorProfile,
    foundation: FoundationGeneticIdentity,
    foundation_content_digest: Blake3Digest,
    final_population_slot: u32,
    world_entity_id: WorldEntityId,
    organism_id: OrganismId,
) -> u64 {
    let mut digest = CanonicalDigestBuilder::new(CURATED_FOUNDER_SEED_DOMAIN);
    digest.write_utf8(policy_label);
    digest.write_utf8(seed_domain_version);
    digest.write_utf8(source_save_identity);
    digest.write_utf8(source_save_label);
    digest.write_utf8(source_run_identity);
    digest.write_u64(source_save_seed);
    digest.write_u64(world_seed);
    digest.write_u64(restored_tick.raw());
    digest.write_u32(target_population);
    digest.write_u32(final_population_slot);
    digest.write_u64(world_entity_id.raw());
    digest.write_u64(organism_id.raw());
    digest.write_u16(sensor_profile.raw());
    write_foundation_identity(&mut digest, foundation);
    digest.write_bytes(foundation_content_digest.bytes());
    nonzero_word_mix(digest.finish256())
}

fn write_foundation_identity(
    digest: &mut CanonicalDigestBuilder,
    foundation: FoundationGeneticIdentity,
) {
    digest.write_u64(foundation.foundation_id);
    digest.write_u16(foundation.version);
    digest.write_u64(foundation.compatibility_family_id);
    digest.write_u16(foundation.brain_class_id.raw());
}

fn nonzero_word_mix(words: [u64; 4]) -> u64 {
    let mixed =
        words[0] ^ words[1].rotate_left(17) ^ words[2].rotate_right(11) ^ words[3].rotate_left(29);
    if mixed == 0 {
        1
    } else {
        mixed
    }
}

fn nonzero_canonical_digest(words: [u64; 4]) -> [u64; 4] {
    if words == [0; 4] {
        [1, 0, 0, 0]
    } else {
        words
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{
        Blake3Digest, BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity,
        FoundationWeightAsset, GenomeId, OrganismId, SensorProfile, Tick, WorldEntityId,
    };

    fn checked_foundation(
        sensor_profile: SensorProfile,
    ) -> (FoundationGeneticIdentity, Blake3Digest) {
        let asset = FoundationWeightAsset::builtin_nano512_v1(sensor_profile).unwrap();
        let manifest = asset.manifest();
        let foundation = FoundationGeneticIdentity::new(
            manifest.foundation_id().raw(),
            manifest.foundation_version().raw() as u16,
            manifest.compatibility_family_id().raw(),
            BrainCapacityClass::N512_ID,
        )
        .unwrap();
        (foundation, asset.digest())
    }

    fn request_with_profile(sensor_profile: SensorProfile) -> CuratedFounderResetRequest {
        let (foundation, foundation_content_digest) = checked_foundation(sensor_profile);
        CuratedFounderResetRequest {
            policy_label: Some(CURATED_FOUNDER_RESET_POLICY.to_string()),
            source_save_identity: "save:portable-p34-2026-08-08".to_string(),
            source_save_label: "P34 restored final population".to_string(),
            source_save_seed: 0x1111_2222_3333_4444,
            world_seed: 0x5555_6666_7777_8888,
            restored_tick: Tick::new(42_000),
            target_population: 3,
            sensor_profile,
            foundation,
            foundation_content_digest,
            source_run_identity: "run:portable-p34-2026-08-08".to_string(),
            final_agents: vec![
                CuratedFounderAgentInput {
                    world_entity_id: WorldEntityId(101),
                    organism_id: Some(OrganismId(201)),
                    final_population_slot: 0,
                    legacy_genome_id: None,
                },
                CuratedFounderAgentInput {
                    world_entity_id: WorldEntityId(102),
                    organism_id: Some(OrganismId(202)),
                    final_population_slot: 1,
                    legacy_genome_id: None,
                },
                CuratedFounderAgentInput {
                    world_entity_id: WorldEntityId(103),
                    organism_id: Some(OrganismId(203)),
                    final_population_slot: 2,
                    legacy_genome_id: None,
                },
            ],
        }
    }

    fn entry_for_world(
        plan: &CuratedFounderPlan,
        world_entity_id: u64,
    ) -> &CuratedFounderPlanEntry {
        plan.entries
            .iter()
            .find(|entry| entry.world_entity_id.raw() == world_entity_id)
            .unwrap()
    }

    fn serialized_plan(plan: &CuratedFounderPlan) -> CuratedFounderPlan {
        serde_json::from_str(&serde_json::to_string(plan).unwrap()).unwrap()
    }

    fn serialized_receipt(receipt: &CuratedFounderResetReceipt) -> CuratedFounderResetReceipt {
        serde_json::from_str(&serde_json::to_string(receipt).unwrap()).unwrap()
    }

    fn sync_receipt_to_plan(plan: &mut CuratedFounderPlan) {
        plan.receipt.policy_label = plan.policy_label.clone();
        plan.receipt.source_save_identity = plan.source_save_identity.clone();
        plan.receipt.source_save_label = plan.source_save_label.clone();
        plan.receipt.source_save_seed = plan.source_save_seed;
        plan.receipt.world_seed = plan.world_seed;
        plan.receipt.restored_tick = plan.restored_tick;
        plan.receipt.target_population = plan.target_population;
        plan.receipt.seed_domain_version = plan.seed_domain_version.clone();
        plan.receipt.sensor_profile = plan.sensor_profile;
        plan.receipt.foundation = plan.foundation;
        plan.receipt.foundation_content_digest = plan.foundation_content_digest;
        plan.receipt.source_run_identity = plan.source_run_identity.clone();
        plan.receipt.ordered_agent_identities = plan
            .entries
            .iter()
            .map(|entry| CuratedFounderAgentIdentity {
                final_population_slot: entry.final_population_slot,
                world_entity_id: entry.world_entity_id,
                organism_id: entry.organism_id,
            })
            .collect();
        plan.receipt.derived_conception_seeds = plan
            .entries
            .iter()
            .map(|entry| entry.conception_seed)
            .collect();
        plan.receipt.derived_genome_ids =
            plan.entries.iter().map(|entry| entry.genome_id).collect();
        plan.receipt.derived_lineage_ids =
            plan.entries.iter().map(|entry| entry.lineage_id).collect();
        plan.receipt.receipt_digest = plan.receipt.recompute_digest();
    }

    #[test]
    fn curated_reset_rejects_legacy_source_with_exact_policy() {
        let mut request = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        request.final_agents[0].legacy_genome_id = Some(GenomeId(0xDEAD_BEEF));

        assert_eq!(
            plan_curated_founder_reset(&request),
            Err(CuratedFounderResetError::LegacyGenomeSource)
        );
    }

    #[test]
    fn serialized_plan_validation_rejects_tampered_ids_order_provenance_and_seed() {
        let plan = plan_curated_founder_reset(&request_with_profile(
            SensorProfile::PrivilegedAffordanceV1,
        ))
        .unwrap();

        let mut zero_world = serialized_plan(&plan);
        zero_world.entries[0].world_entity_id = WorldEntityId(0);
        sync_receipt_to_plan(&mut zero_world);
        assert!(zero_world.validate().is_err());

        let mut zero_organism = serialized_plan(&plan);
        zero_organism.entries[0].organism_id = OrganismId(0);
        sync_receipt_to_plan(&mut zero_organism);
        assert!(zero_organism.validate().is_err());

        let mut duplicate_genome = serialized_plan(&plan);
        duplicate_genome.entries[1].genome_id = duplicate_genome.entries[0].genome_id;
        sync_receipt_to_plan(&mut duplicate_genome);
        assert!(duplicate_genome.validate().is_err());

        let mut duplicate_lineage = serialized_plan(&plan);
        duplicate_lineage.entries[1].lineage_id = duplicate_lineage.entries[0].lineage_id;
        sync_receipt_to_plan(&mut duplicate_lineage);
        assert!(duplicate_lineage.validate().is_err());

        let mut wrong_order = serialized_plan(&plan);
        wrong_order.entries.swap(0, 1);
        sync_receipt_to_plan(&mut wrong_order);
        assert!(wrong_order.validate().is_err());

        let mut changed_provenance = serialized_plan(&plan);
        changed_provenance.source_save_seed += 1;
        sync_receipt_to_plan(&mut changed_provenance);
        assert!(changed_provenance.validate().is_err());

        let mut changed_identity = serialized_plan(&plan);
        changed_identity.entries[0].world_entity_id = WorldEntityId(9_999);
        sync_receipt_to_plan(&mut changed_identity);
        assert!(changed_identity.validate().is_err());

        let mut changed_seed = serialized_plan(&plan);
        let old_seed = changed_seed.entries[0].conception_seed;
        let new_seed = old_seed.wrapping_add(1).max(1);
        let new_genome =
            CreatureGenome::early_mammal_founder(new_seed, changed_seed.foundation).unwrap();
        changed_seed.entries[0].conception_seed = new_seed;
        changed_seed.entries[0].genome_id = new_genome.id;
        changed_seed.entries[0].lineage_id = new_genome.lineage_id;
        sync_receipt_to_plan(&mut changed_seed);
        assert!(changed_seed.validate().is_err());
    }

    #[test]
    fn serialized_receipt_validation_rejects_tampered_contract_fields() {
        let plan = plan_curated_founder_reset(&request_with_profile(
            SensorProfile::PrivilegedAffordanceV1,
        ))
        .unwrap();
        let receipt = serialized_receipt(&plan.receipt);
        assert!(receipt.validate().is_ok());

        let mut zero_world = receipt.clone();
        zero_world.ordered_agent_identities[0].world_entity_id = WorldEntityId(0);
        zero_world.receipt_digest = zero_world.recompute_digest();
        assert!(zero_world.validate().is_err());

        let mut zero_organism = receipt.clone();
        zero_organism.ordered_agent_identities[0].organism_id = OrganismId(0);
        zero_organism.receipt_digest = zero_organism.recompute_digest();
        assert!(zero_organism.validate().is_err());

        let mut wrong_slot_set = receipt.clone();
        wrong_slot_set.ordered_agent_identities[0].final_population_slot = 3;
        wrong_slot_set.receipt_digest = wrong_slot_set.recompute_digest();
        assert!(wrong_slot_set.validate().is_err());

        let mut wrong_order = receipt.clone();
        wrong_order.ordered_agent_identities.swap(0, 1);
        wrong_order.receipt_digest = wrong_order.recompute_digest();
        assert!(wrong_order.validate().is_err());

        let mut duplicate_conception = receipt.clone();
        duplicate_conception.derived_conception_seeds[1] =
            duplicate_conception.derived_conception_seeds[0];
        duplicate_conception.receipt_digest = duplicate_conception.recompute_digest();
        assert!(duplicate_conception.validate().is_err());

        let mut duplicate_genome = receipt.clone();
        duplicate_genome.derived_genome_ids[1] = duplicate_genome.derived_genome_ids[0];
        duplicate_genome.receipt_digest = duplicate_genome.recompute_digest();
        assert!(duplicate_genome.validate().is_err());

        let mut duplicate_lineage = receipt.clone();
        duplicate_lineage.derived_lineage_ids[1] = duplicate_lineage.derived_lineage_ids[0];
        duplicate_lineage.receipt_digest = duplicate_lineage.recompute_digest();
        assert!(duplicate_lineage.validate().is_err());

        let mut zero_source_seed = receipt.clone();
        zero_source_seed.source_save_seed = 0;
        zero_source_seed.receipt_digest = zero_source_seed.recompute_digest();
        assert!(zero_source_seed.validate().is_err());

        let mut zero_world_seed = receipt.clone();
        zero_world_seed.world_seed = 0;
        zero_world_seed.receipt_digest = zero_world_seed.recompute_digest();
        assert!(zero_world_seed.validate().is_err());

        let mut changed_provenance = receipt.clone();
        changed_provenance.source_save_seed += 1;
        changed_provenance.receipt_digest = changed_provenance.recompute_digest();
        assert!(changed_provenance.validate().is_err());

        let mut changed_identity = receipt.clone();
        changed_identity.ordered_agent_identities[0].world_entity_id = WorldEntityId(9_999);
        changed_identity.receipt_digest = changed_identity.recompute_digest();
        assert!(changed_identity.validate().is_err());

        let mut changed_seed = receipt.clone();
        changed_seed.derived_conception_seeds[0] = changed_seed.derived_conception_seeds[0]
            .wrapping_add(1)
            .max(1);
        changed_seed.receipt_digest = changed_seed.recompute_digest();
        assert!(changed_seed.validate().is_err());

        let mut wrong_foundation = receipt.clone();
        wrong_foundation.foundation.foundation_id ^= 1;
        wrong_foundation.receipt_digest = wrong_foundation.recompute_digest();
        assert!(wrong_foundation.validate().is_err());

        let mut wrong_profile = receipt.clone();
        wrong_profile.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
        wrong_profile.receipt_digest = wrong_profile.recompute_digest();
        assert!(wrong_profile.validate().is_err());

        let mut wrong_digest = receipt;
        wrong_digest.foundation_content_digest = Blake3Digest::from_bytes([0xA5; 32]);
        wrong_digest.receipt_digest = wrong_digest.recompute_digest();
        assert!(wrong_digest.validate().is_err());
    }

    #[test]
    fn production_bootstrap_rejects_bare_p34_without_exact_curated_reset() {
        let mut request = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        request.policy_label = None;
        request.final_agents[0].legacy_genome_id = Some(GenomeId(0xDEAD_BEEF));
        assert_eq!(
            plan_curated_founder_reset(&request),
            Err(CuratedFounderResetError::MissingExactPolicy)
        );

        request.policy_label = Some("curated-founder-reset:v2".to_string());
        assert!(matches!(
            plan_curated_founder_reset(&request),
            Err(CuratedFounderResetError::PolicyMismatch { .. })
        ));
    }

    #[test]
    fn curated_reset_plan_is_stable_by_final_agent_identity() {
        let first_request = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        let mut reordered_request = first_request.clone();
        reordered_request.final_agents.reverse();

        let first = plan_curated_founder_reset(&first_request).unwrap();
        let reordered = plan_curated_founder_reset(&reordered_request).unwrap();

        assert_eq!(first.entries, reordered.entries);
        assert_eq!(first.receipt, reordered.receipt);
        assert_ne!(first.receipt.receipt_digest, [0; 4]);
    }

    #[test]
    fn curated_reset_changes_seed_when_source_slot_or_identity_changes() {
        let baseline_request = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        let baseline = plan_curated_founder_reset(&baseline_request).unwrap();
        let baseline_seed = entry_for_world(&baseline, 101).conception_seed;

        let mut source_changed = baseline_request.clone();
        source_changed.source_save_seed += 1;
        let source_plan = plan_curated_founder_reset(&source_changed).unwrap();
        assert_ne!(
            baseline_seed,
            entry_for_world(&source_plan, 101).conception_seed
        );

        let mut slot_changed = baseline_request.clone();
        slot_changed.final_agents[0].final_population_slot = 1;
        slot_changed.final_agents[1].final_population_slot = 0;
        let slot_plan = plan_curated_founder_reset(&slot_changed).unwrap();
        assert_ne!(
            baseline_seed,
            entry_for_world(&slot_plan, 101).conception_seed
        );

        let mut identity_changed = baseline_request;
        identity_changed.final_agents[0].world_entity_id = WorldEntityId(9_999);
        let identity_plan = plan_curated_founder_reset(&identity_changed).unwrap();
        assert_ne!(
            baseline_seed,
            identity_plan
                .entries
                .iter()
                .find(|entry| entry.organism_id == OrganismId(201))
                .unwrap()
                .conception_seed
        );
    }

    #[test]
    fn curated_reset_requires_exact_checked_n512_foundation() {
        for sensor_profile in [
            SensorProfile::PrivilegedAffordanceV1,
            SensorProfile::GroundedObjectSlotsV1,
        ] {
            assert!(plan_curated_founder_reset(&request_with_profile(sensor_profile)).is_ok());
        }

        let mut cross_class = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        cross_class.foundation.brain_class_id = BrainCapacityClass::N2048_ID;
        assert!(matches!(
            plan_curated_founder_reset(&cross_class),
            Err(CuratedFounderResetError::FoundationMismatch)
        ));

        let mut forged_identity = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        forged_identity.foundation.foundation_id ^= 1;
        assert!(matches!(
            plan_curated_founder_reset(&forged_identity),
            Err(CuratedFounderResetError::FoundationMismatch)
        ));

        let mut digest_mismatch = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        digest_mismatch.foundation_content_digest = Blake3Digest::from_bytes([0xA5; 32]);
        assert!(matches!(
            plan_curated_founder_reset(&digest_mismatch),
            Err(CuratedFounderResetError::FoundationMismatch)
        ));
    }

    #[test]
    fn curated_reset_rejects_duplicate_or_incomplete_final_population() {
        let mut duplicate_world = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        duplicate_world.final_agents[1].world_entity_id =
            duplicate_world.final_agents[0].world_entity_id;
        assert!(matches!(
            plan_curated_founder_reset(&duplicate_world),
            Err(CuratedFounderResetError::DuplicateWorldEntityId)
        ));

        let mut duplicate_organism = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        duplicate_organism.final_agents[1].organism_id =
            duplicate_organism.final_agents[0].organism_id;
        assert!(matches!(
            plan_curated_founder_reset(&duplicate_organism),
            Err(CuratedFounderResetError::DuplicateOrganismId)
        ));

        let mut duplicate_slot = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        duplicate_slot.final_agents[1].final_population_slot =
            duplicate_slot.final_agents[0].final_population_slot;
        assert!(matches!(
            plan_curated_founder_reset(&duplicate_slot),
            Err(CuratedFounderResetError::DuplicatePopulationSlot)
        ));

        let mut gap = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        gap.final_agents[2].final_population_slot = 3;
        assert!(matches!(
            plan_curated_founder_reset(&gap),
            Err(CuratedFounderResetError::PopulationSlotGap)
        ));

        let mut missing_organism = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        missing_organism.final_agents[0].organism_id = None;
        assert!(matches!(
            plan_curated_founder_reset(&missing_organism),
            Err(CuratedFounderResetError::MissingOrganismId)
        ));

        let mut incomplete = request_with_profile(SensorProfile::PrivilegedAffordanceV1);
        incomplete.final_agents.pop();
        assert!(matches!(
            plan_curated_founder_reset(&incomplete),
            Err(CuratedFounderResetError::AgentCountMismatch { .. })
        ));
    }

    #[test]
    fn curated_reset_founders_have_empty_parent_recombination_and_mutation_history() {
        let request = request_with_profile(SensorProfile::GroundedObjectSlotsV1);
        let plan = plan_curated_founder_reset(&request).unwrap();

        for entry in &plan.entries {
            let genome =
                CreatureGenome::early_mammal_founder(entry.conception_seed, plan.foundation)
                    .unwrap();
            assert!(genome.parent_genome_ids.is_empty());
            assert!(!genome.provenance.ordinary_birth);
            assert!(genome.provenance.recombination.is_empty());
            assert!(genome.provenance.mutations.is_empty());
            assert_eq!(genome.id, entry.genome_id);
            assert_eq!(genome.lineage_id, entry.lineage_id);
            assert_eq!(genome.foundation, plan.foundation);
        }
    }
}
