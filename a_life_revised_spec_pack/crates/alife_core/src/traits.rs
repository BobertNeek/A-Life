//! v0 scaffold: semantic-prior and compute-backend interfaces only.

use serde::{Deserialize, Serialize};

use crate::OrganismId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticPriorRequest {
    pub organism_id: OrganismId,
    pub sequence_id: u64,
    pub private_to_organism: bool,
}

impl SemanticPriorRequest {
    pub const fn new(organism_id: u64, sequence_id: u64) -> Self {
        Self {
            organism_id: OrganismId(organism_id),
            sequence_id,
            private_to_organism: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SemanticPriorPacket {
    pub request: SemanticPriorRequest,
    pub lexicon_bias_slots: Vec<u16>,
    pub plasticity_modulation: f32,
}

pub trait SemanticPriorProvider {
    fn provider_name(&self) -> &'static str;
}

pub trait NeuralComputeBackend {
    fn backend_name(&self) -> &'static str;
}
