//! v0 scaffold: structured action ABI contracts.

use serde::{Deserialize, Serialize};

use crate::{
    ensure_current_version, validate_optional_target, Confidence, DurationTicks, OrganismId,
    SchemaKind, SchemaVersions, Validate, WorldEntityId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    Hold,
    Move,
    Interact,
    Vocalize,
    Write,
    Gesture,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActionCommand {
    pub abi_version: u16,
    pub organism_id: OrganismId,
    pub kind: ActionKind,
    pub target_entity: Option<WorldEntityId>,
    pub confidence: Confidence,
    pub duration_ticks: DurationTicks,
    pub drive_source_mask: u32,
}

impl ActionCommand {
    pub const ABI_VERSION: u16 = SchemaVersions::CURRENT.action_abi.0;

    pub fn new(
        organism_id: OrganismId,
        kind: ActionKind,
        target_entity: Option<WorldEntityId>,
        confidence: Confidence,
        duration_ticks: DurationTicks,
    ) -> Result<Self, crate::ScaffoldContractError> {
        organism_id.validate()?;
        validate_optional_target(target_entity)?;
        Ok(Self {
            abi_version: Self::ABI_VERSION,
            organism_id,
            kind,
            target_entity,
            confidence,
            duration_ticks,
            drive_source_mask: 0,
        })
    }
}

impl Validate for ActionCommand {
    fn validate_contract(&self) -> Result<(), crate::ScaffoldContractError> {
        ensure_current_version(SchemaKind::ActionAbi, self.abi_version)?;
        self.organism_id.validate()?;
        validate_optional_target(self.target_entity)?;
        Confidence::new(self.confidence.raw())?;
        Ok(())
    }
}
