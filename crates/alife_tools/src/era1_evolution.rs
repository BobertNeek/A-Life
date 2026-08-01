//! Bounded, deterministic reproduction receipts for the Era 1 evolution program.

use std::collections::BTreeSet;

use alife_core::{
    BrainCapacityClass, CreatureGenome, Era1Control, GenomeId, LanguageTokenId,
    ScaffoldContractError, Validate,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const ERA1_EVOLUTION_SCHEMA_VERSION: u16 = 1;
const BOUNDED_LINEAGES: usize = 4;
const BOUNDED_EVALUATION_SEEDS: usize = 3;
const BOUNDED_HELD_OUT_TRANSFORMS: usize = 2;
const BOUNDED_ORDINARY_GENERATIONS: u32 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1EvolutionConfig {
    pub schema_version: u16,
    pub evolution_seed: u64,
    pub lineage_count: usize,
    pub evaluation_seeds: Vec<u64>,
    pub held_out_world_transforms: Vec<u64>,
    pub controls: Vec<Era1Control>,
    pub ordinary_birth_generations: u32,
}

impl Era1EvolutionConfig {
    pub fn bounded_default(evolution_seed: u64) -> Result<Self, Era1EvolutionError> {
        if evolution_seed == 0 {
            return Err(Era1EvolutionError::InvalidConfig("evolution seed is zero"));
        }
        let config = Self {
            schema_version: ERA1_EVOLUTION_SCHEMA_VERSION,
            evolution_seed,
            lineage_count: BOUNDED_LINEAGES,
            evaluation_seeds: (0..BOUNDED_EVALUATION_SEEDS)
                .map(|index| derived_seed(evolution_seed, 0xE1A1_0000, index as u64))
                .collect(),
            held_out_world_transforms: (0..BOUNDED_HELD_OUT_TRANSFORMS)
                .map(|index| derived_seed(evolution_seed, 0xE1A1_1000, index as u64))
                .collect(),
            controls: Era1Control::ALL.to_vec(),
            ordinary_birth_generations: BOUNDED_ORDINARY_GENERATIONS,
        };
        config.validate_contract()?;
        Ok(config)
    }

    pub fn validate_contract(&self) -> Result<(), Era1EvolutionError> {
        if self.schema_version != ERA1_EVOLUTION_SCHEMA_VERSION
            || self.evolution_seed == 0
            || self.lineage_count != BOUNDED_LINEAGES
            || self.evaluation_seeds.len() != BOUNDED_EVALUATION_SEEDS
            || self.held_out_world_transforms.len() != BOUNDED_HELD_OUT_TRANSFORMS
            || self.controls != Era1Control::ALL
            || self.ordinary_birth_generations != BOUNDED_ORDINARY_GENERATIONS
            || !all_unique_nonzero(&self.evaluation_seeds)
            || !all_unique_nonzero(&self.held_out_world_transforms)
        {
            return Err(Era1EvolutionError::InvalidConfig(
                "bounded Era 1 matrix changed",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1AcquiredStateEvidence {
    pub lifetime_weight_digest: Option<[u64; 4]>,
    pub memory_digests: Vec<[u64; 4]>,
    pub learned_vocabulary: Vec<LanguageTokenId>,
    pub pending_eligibility_digest: Option<[u64; 4]>,
    pub transient_state_digest: Option<[u64; 4]>,
}

impl Era1AcquiredStateEvidence {
    pub fn is_empty(&self) -> bool {
        self.lifetime_weight_digest.is_none()
            && self.memory_digests.is_empty()
            && self.learned_vocabulary.is_empty()
            && self.pending_eligibility_digest.is_none()
            && self.transient_state_digest.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1BirthReceipt {
    pub generation: u32,
    pub lineage_slot: usize,
    pub genome: CreatureGenome,
    pub inherited_starter_tokens: Vec<LanguageTokenId>,
    pub acquired_state: Era1AcquiredStateEvidence,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1GenerationReceipt {
    pub generation: u32,
    pub births: Vec<Era1BirthReceipt>,
    pub preserved_wild_genome_ids: Vec<GenomeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1LineageReceipt {
    pub lineage_slot: usize,
    pub founder_genome_id: GenomeId,
    pub genome_ids: Vec<GenomeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1EvolutionReceipt {
    pub schema_version: u16,
    pub config: Era1EvolutionConfig,
    pub wild_reservoir: Vec<CreatureGenome>,
    pub generations: Vec<Era1GenerationReceipt>,
    pub lineages: Vec<Era1LineageReceipt>,
}

impl Era1EvolutionReceipt {
    pub fn validate_contract(&self) -> Result<(), Era1EvolutionError> {
        self.config.validate_contract()?;
        if self.schema_version != ERA1_EVOLUTION_SCHEMA_VERSION
            || self.wild_reservoir.len() != self.config.lineage_count
            || self.generations.len()
                != usize::try_from(self.config.ordinary_birth_generations + 1)
                    .map_err(|_| Era1EvolutionError::InvalidEvidence("generation overflow"))?
            || self.lineages.len() != self.config.lineage_count
        {
            return Err(Era1EvolutionError::InvalidEvidence(
                "evolution receipt shape changed",
            ));
        }

        validate_founders(&self.wild_reservoir)?;
        let wild_ids = self
            .wild_reservoir
            .iter()
            .map(|genome| genome.id)
            .collect::<Vec<_>>();

        for (generation_index, generation) in self.generations.iter().enumerate() {
            let expected_generation = u32::try_from(generation_index)
                .map_err(|_| Era1EvolutionError::InvalidEvidence("generation overflow"))?;
            if generation.generation != expected_generation
                || generation.births.len() != self.config.lineage_count
                || generation.preserved_wild_genome_ids != wild_ids
            {
                return Err(Era1EvolutionError::InvalidEvidence(
                    "generation receipt shape changed",
                ));
            }

            for (slot, birth) in generation.births.iter().enumerate() {
                validate_birth(birth, expected_generation, slot)?;
                if generation_index == 0 {
                    if birth.genome != self.wild_reservoir[slot]
                        || birth.genome.provenance.ordinary_birth
                        || !birth.genome.parent_genome_ids.is_empty()
                    {
                        return Err(Era1EvolutionError::InvalidEvidence(
                            "founder birth receipt changed",
                        ));
                    }
                } else {
                    let parents = &self.generations[generation_index - 1].births;
                    let maternal = &parents[slot].genome;
                    let paternal = &parents[(slot + 1) % parents.len()].genome;
                    if birth.genome.parent_genome_ids != [maternal.id, paternal.id]
                        || birth.genome
                            != CreatureGenome::reproduce(
                                maternal,
                                paternal,
                                birth.genome.conception_seed,
                            )?
                    {
                        return Err(Era1EvolutionError::InvalidEvidence(
                            "offspring does not match authoritative reproduction",
                        ));
                    }
                }
            }
        }

        for (slot, lineage) in self.lineages.iter().enumerate() {
            let expected = self
                .generations
                .iter()
                .map(|generation| generation.births[slot].genome.id)
                .collect::<Vec<_>>();
            if lineage.lineage_slot != slot
                || lineage.founder_genome_id != self.wild_reservoir[slot].id
                || lineage.genome_ids != expected
            {
                return Err(Era1EvolutionError::InvalidEvidence(
                    "lineage receipt does not match generations",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Era1EvolutionError {
    #[error("invalid Era 1 evolution configuration: {0}")]
    InvalidConfig(&'static str),
    #[error("invalid Era 1 evolution evidence: {0}")]
    InvalidEvidence(&'static str),
    #[error("authoritative genome operation failed: {0}")]
    Genome(#[from] ScaffoldContractError),
}

pub fn run_era1_evolution(
    config: &Era1EvolutionConfig,
    founders: &[CreatureGenome],
) -> Result<Era1EvolutionReceipt, Era1EvolutionError> {
    config.validate_contract()?;
    validate_founders(founders)?;
    if founders.len() != config.lineage_count {
        return Err(Era1EvolutionError::InvalidEvidence(
            "founder count does not match bounded lineages",
        ));
    }

    let wild_reservoir = founders.to_vec();
    let wild_ids = founders.iter().map(|genome| genome.id).collect::<Vec<_>>();
    let founder_births = founders
        .iter()
        .cloned()
        .enumerate()
        .map(|(lineage_slot, genome)| birth_receipt(0, lineage_slot, genome))
        .collect::<Result<Vec<_>, _>>()?;
    let mut generations = vec![Era1GenerationReceipt {
        generation: 0,
        births: founder_births,
        preserved_wild_genome_ids: wild_ids.clone(),
    }];

    for generation in 1..=config.ordinary_birth_generations {
        let parents = &generations
            .last()
            .expect("founder generation is always present")
            .births;
        let mut births = Vec::with_capacity(config.lineage_count);
        for lineage_slot in 0..config.lineage_count {
            let maternal = &parents[lineage_slot].genome;
            let paternal = &parents[(lineage_slot + 1) % parents.len()].genome;
            let conception_seed = derived_seed(
                config.evolution_seed ^ maternal.id.0 ^ paternal.id.0.rotate_left(23),
                u64::from(generation),
                lineage_slot as u64,
            );
            let genome = CreatureGenome::reproduce(maternal, paternal, conception_seed)?;
            births.push(birth_receipt(generation, lineage_slot, genome)?);
        }
        generations.push(Era1GenerationReceipt {
            generation,
            births,
            preserved_wild_genome_ids: wild_ids.clone(),
        });
    }

    let lineages = (0..config.lineage_count)
        .map(|lineage_slot| Era1LineageReceipt {
            lineage_slot,
            founder_genome_id: founders[lineage_slot].id,
            genome_ids: generations
                .iter()
                .map(|generation| generation.births[lineage_slot].genome.id)
                .collect(),
        })
        .collect();
    let receipt = Era1EvolutionReceipt {
        schema_version: ERA1_EVOLUTION_SCHEMA_VERSION,
        config: config.clone(),
        wild_reservoir,
        generations,
        lineages,
    };
    receipt.validate_contract()?;
    Ok(receipt)
}

fn birth_receipt(
    generation: u32,
    lineage_slot: usize,
    genome: CreatureGenome,
) -> Result<Era1BirthReceipt, Era1EvolutionError> {
    let inherited_starter_tokens = genome.express()?.predisposition.starter_tokens;
    let receipt = Era1BirthReceipt {
        generation,
        lineage_slot,
        genome,
        inherited_starter_tokens,
        acquired_state: Era1AcquiredStateEvidence::default(),
    };
    validate_birth(&receipt, generation, lineage_slot)?;
    Ok(receipt)
}

fn validate_birth(
    birth: &Era1BirthReceipt,
    generation: u32,
    lineage_slot: usize,
) -> Result<(), Era1EvolutionError> {
    birth.genome.validate_contract()?;
    let expressed = birth.genome.express()?;
    if birth.generation != generation
        || birth.lineage_slot != lineage_slot
        || !birth.acquired_state.is_empty()
        || birth.inherited_starter_tokens.is_empty()
        || birth.inherited_starter_tokens != expressed.predisposition.starter_tokens
        || birth
            .inherited_starter_tokens
            .iter()
            .any(|token| token.raw() == 0)
    {
        return Err(Era1EvolutionError::InvalidEvidence(
            "birth inherited copied or fabricated state",
        ));
    }
    Ok(())
}

fn validate_founders(founders: &[CreatureGenome]) -> Result<(), Era1EvolutionError> {
    let mut genome_ids = BTreeSet::new();
    let mut lineage_ids = BTreeSet::new();
    for founder in founders {
        founder.validate_contract()?;
        let phenotype = founder.express()?;
        if founder.foundation.brain_class_id != BrainCapacityClass::N2048_ID
            || founder.provenance.ordinary_birth
            || !founder.parent_genome_ids.is_empty()
            || !genome_ids.insert(founder.id.0)
            || !lineage_ids.insert(founder.lineage_id.0)
            || phenotype.brain_genome.brain_class_id != BrainCapacityClass::N2048_ID
        {
            return Err(Era1EvolutionError::InvalidEvidence(
                "founders must be distinct viable N2048 lineages",
            ));
        }
    }
    Ok(())
}

fn all_unique_nonzero(values: &[u64]) -> bool {
    let unique = values.iter().copied().collect::<BTreeSet<_>>();
    !unique.contains(&0) && unique.len() == values.len()
}

fn derived_seed(root: u64, domain: u64, index: u64) -> u64 {
    let mut value = root ^ domain.rotate_left(17) ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    if value == 0 {
        1
    } else {
        value
    }
}
