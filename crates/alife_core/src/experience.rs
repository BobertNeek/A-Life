//! v0 scaffold: versioned ExperiencePatch headers.

use serde::{Deserialize, Serialize};

use crate::{
    ensure_current_version, ExperienceSequenceId, OrganismId, SchemaKind, SchemaVersions, Tick,
    Validate,
};

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
    pub sequence_id: ExperienceSequenceId,
    pub world_tick: Tick,
    pub phase: ExperiencePatchPhase,
}

impl ExperiencePatchHeader {
    pub const ABI_VERSION: u16 = SchemaVersions::CURRENT.experience.0;

    pub fn new(
        organism_id: OrganismId,
        sequence_id: ExperienceSequenceId,
        world_tick: Tick,
    ) -> Result<Self, crate::ScaffoldContractError> {
        organism_id.validate()?;
        sequence_id.validate()?;
        Ok(Self {
            abi_version: Self::ABI_VERSION,
            organism_id,
            sequence_id,
            world_tick,
            phase: ExperiencePatchPhase::Ingest,
        })
    }
}

impl Validate for ExperiencePatchHeader {
    fn validate_contract(&self) -> Result<(), crate::ScaffoldContractError> {
        ensure_current_version(SchemaKind::Experience, self.abi_version)?;
        self.organism_id.validate()?;
        self.sequence_id.validate()?;
        Ok(())
    }
}
