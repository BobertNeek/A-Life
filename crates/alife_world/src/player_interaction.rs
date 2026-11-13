//! Four player experiences. Doses are design defaults, not reward verdicts.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerInteractionV1 {
    Praise,
    Scold,
    Pet,
    Slap,
}
impl PlayerInteractionV1 {
    pub(crate) fn is_social(self) -> bool {
        matches!(self, Self::Praise | Self::Scold)
    }
    pub(crate) fn apply(self, event: &mut alife_core::BodyEventDelta) {
        // Assignment, not addition: repeated/held input before delivery is capped.
        match self {
            Self::Praise => event.player_reward = 1.0,
            Self::Scold => event.social_disapproval_v1 = 1.0,
            Self::Pet => event.social_contact = 0.25,
            Self::Slap => event.damage = 0.02,
        }
    }
}

/// Request outcome, distinct from a later applied physiology receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerInteractionQueueOutcomeV1 {
    Queued,
    NoReceivedDose,
}
