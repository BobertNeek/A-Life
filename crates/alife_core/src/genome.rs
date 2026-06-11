//! v0 scaffold: genome and developmental encoding contracts.

use serde::{Deserialize, Serialize};

use crate::{BrainClassId, GenomeId, SchemaVersions, Validate};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainGenome {
    pub id: GenomeId,
    pub species_seed: u64,
    pub brain_class_id: BrainClassId,
    pub genetic_prior_seed: u64,
    pub developmental_schedule_version: u16,
    pub mutable_lifetime_weights_allowed: bool,
}

impl BrainGenome {
    pub const SCHEMA_VERSION: u16 = SchemaVersions::CURRENT.genome.0;

    pub fn scaffold(species_seed: u64, brain_class_id: BrainClassId) -> Self {
        let genetic_prior_seed = species_seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(u64::from(brain_class_id.0))
            .max(1);
        Self {
            id: GenomeId(genetic_prior_seed),
            species_seed,
            brain_class_id,
            genetic_prior_seed,
            developmental_schedule_version: 1,
            mutable_lifetime_weights_allowed: true,
        }
    }
}

impl Validate for BrainGenome {
    fn validate_contract(&self) -> Result<(), crate::ScaffoldContractError> {
        self.id.validate()?;
        self.brain_class_id.validate()?;
        Ok(())
    }
}
