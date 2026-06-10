//! v0 scaffold: versioned ExperiencePatch headers.

use serde::{Deserialize, Serialize};

use crate::OrganismId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExperiencePatchPhase {
    Ingest,
    Activate,
    Consolidate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperiencePatchHeader {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub sequence_id: u64,
    pub world_tick: u64,
    pub phase: ExperiencePatchPhase,
}

impl ExperiencePatchHeader {
    pub const ABI_VERSION: u16 = 1;

    pub const fn new(organism_id: u64, sequence_id: u64, world_tick: u64) -> Self {
        Self {
            abi_version: Self::ABI_VERSION,
            organism_id: OrganismId(organism_id),
            sequence_id,
            world_tick,
            phase: ExperiencePatchPhase::Ingest,
        }
    }
}
