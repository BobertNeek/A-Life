//! v0 scaffold: Bevy-independent world contracts.

use alife_core::{ActionAbiVersion, ActionCommand, SensoryAbiVersion};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionLegality {
    Legal,
    ImpossibleTarget,
    BlockedByWorldState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldContractManifest {
    pub sensory_abi_version: SensoryAbiVersion,
    pub action_abi_version: ActionAbiVersion,
    pub world_validates_actions: bool,
}

impl WorldContractManifest {
    pub const CURRENT: Self = Self {
        sensory_abi_version: SensoryAbiVersion::CURRENT,
        action_abi_version: ActionAbiVersion::CURRENT,
        world_validates_actions: true,
    };
}

pub trait ActionLegalityChecker {
    fn check_action(&self, action: &ActionCommand) -> ActionLegality;
}
