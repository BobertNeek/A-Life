//! v0 scaffold: structured action ABI contracts.

use serde::{Deserialize, Serialize};

use crate::{OrganismId, WorldEntityId};

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
    pub confidence: f32,
    pub duration_ticks: u32,
    pub drive_source_mask: u32,
}

impl ActionCommand {
    pub const ABI_VERSION: u16 = 1;

    pub const fn new(
        organism_id: u64,
        kind: ActionKind,
        target_entity: Option<WorldEntityId>,
        confidence: f32,
        duration_ticks: u32,
    ) -> Self {
        Self {
            abi_version: Self::ABI_VERSION,
            organism_id: OrganismId(organism_id),
            kind,
            target_entity,
            confidence,
            duration_ticks,
            drive_source_mask: 0,
        }
    }
}
