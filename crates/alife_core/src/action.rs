//! v0 scaffold: structured action ABI contracts.

use serde::{Deserialize, Serialize};

use crate::{validate_optional_target, Confidence, DurationTicks, OrganismId, WorldEntityId};

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
    pub const ABI_VERSION: u16 = 1;

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
