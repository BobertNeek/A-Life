//! Player-tended props in the physical world. Nothing here drives an organism.
use crate::VoxelTileCoord;
use alife_core::{ScaffoldContractError, Tick, WorldEntityId};
use serde::{Deserialize, Serialize};

pub const MAX_ORCHARD_BEDS: usize = 12;
const BUD: u64 = 80;
const BLOOM: u64 = 205;
const RIPE: u64 = 440;
const WATER_BONUS: u64 = 140;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchardStage {
    Seed,
    Bud,
    Bloom,
    Ripe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchardGesture {
    Sow,
    Water,
    Watch,
    Pick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchardReceipt {
    Sown,
    Watered,
    Watching,
    Picked(WorldEntityId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrchardBed {
    pub tile: VoxelTileCoord,
    pub cycle_started: Tick,
    pub watered: bool,
    pub last_fruit: Option<WorldEntityId>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub offered: bool,
}

impl OrchardBed {
    pub fn stage(self, now: Tick) -> OrchardStage {
        let age = now
            .raw()
            .saturating_sub(self.cycle_started.raw())
            .saturating_add(if self.watered { WATER_BONUS } else { 0 });
        match age {
            n if n >= RIPE => OrchardStage::Ripe,
            n if n >= BLOOM => OrchardStage::Bloom,
            n if n >= BUD => OrchardStage::Bud,
            _ => OrchardStage::Seed,
        }
    }
    pub fn gesture(self, now: Tick) -> OrchardGesture {
        match self.stage(now) {
            OrchardStage::Ripe => OrchardGesture::Pick,
            OrchardStage::Bud | OrchardStage::Bloom if !self.watered => OrchardGesture::Water,
            _ => OrchardGesture::Watch,
        }
    }
}

pub(crate) fn valid_coordinate(t: VoxelTileCoord) -> bool {
    (-512..512).contains(&t.x) && (-512..512).contains(&t.z)
}

/// Canonical ordering allows binary search and rejects ambiguous save records.
pub(crate) fn validate_beds(beds: &[OrchardBed]) -> Result<(), ScaffoldContractError> {
    let mut fruit_ids = std::collections::BTreeSet::new();
    if beds.iter().any(|b| {
        (b.offered && b.last_fruit.is_none())
            || b.last_fruit.is_some_and(|id| !fruit_ids.insert(id.raw()))
    }) {
        return Err(ScaffoldContractError::InvalidId);
    }
    if beds.len() > MAX_ORCHARD_BEDS
        || beds.windows(2).any(|p| p[0].tile >= p[1].tile)
        || beds
            .iter()
            .any(|b| !valid_coordinate(b.tile) || b.last_fruit.is_some_and(|id| id.raw() == 0))
    {
        return Err(ScaffoldContractError::InvalidId);
    }
    Ok(())
}
