//! v0 scaffold: semantic-prior and compute-backend interfaces only.

use serde::{Deserialize, Serialize};

use crate::{ExperienceSequenceId, OrganismId, Tick, Validate};

pub const SEMANTIC_PRIOR_MAX_GAIN: f32 = 0.20;
pub const SEMANTIC_PRIOR_MAX_PACKET_TICKS: u64 = 32;
pub const SEMANTIC_PRIOR_MAX_LEXICON_BIAS_SLOTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticPriorRequest {
    pub organism_id: OrganismId,
    pub sequence_id: ExperienceSequenceId,
    pub private_to_organism: bool,
}

impl SemanticPriorRequest {
    pub fn new(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
    ) -> Result<Self, crate::ScaffoldContractError> {
        organism_id.validate()?;
        sequence_id.validate()?;
        Ok(Self {
            organism_id,
            sequence_id,
            private_to_organism: true,
        })
    }
}

impl Validate for SemanticPriorRequest {
    fn validate_contract(&self) -> Result<(), crate::ScaffoldContractError> {
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        if self.private_to_organism {
            Ok(())
        } else {
            Err(crate::ScaffoldContractError::MissingPhaseData)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticPriorPacket {
    pub request: SemanticPriorRequest,
    pub lexicon_bias_slots: Vec<u16>,
    pub plasticity_modulation: f32,
    pub issued_at_tick: Tick,
    pub expires_at_tick: Tick,
    pub assisted: bool,
}

impl Validate for SemanticPriorPacket {
    fn validate_contract(&self) -> Result<(), crate::ScaffoldContractError> {
        self.request.validate_contract()?;
        if self.lexicon_bias_slots.len() > SEMANTIC_PRIOR_MAX_LEXICON_BIAS_SLOTS
            || self.lexicon_bias_slots.iter().any(|slot| *slot >= 256)
            || !self.plasticity_modulation.is_finite()
            || !(0.0..=SEMANTIC_PRIOR_MAX_GAIN).contains(&self.plasticity_modulation)
            || self.expires_at_tick.raw() < self.issued_at_tick.raw()
            || self
                .expires_at_tick
                .raw()
                .saturating_sub(self.issued_at_tick.raw())
                > SEMANTIC_PRIOR_MAX_PACKET_TICKS
        {
            return Err(crate::ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

pub trait SemanticPriorProvider {
    fn provider_name(&self) -> &'static str;
}

pub trait NeuralComputeBackend {
    fn backend_name(&self) -> &'static str;
}
