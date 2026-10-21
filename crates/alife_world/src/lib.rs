//! Engine-independent world state, perception, action execution, and persistence.

use alife_core::ActionCommand;

pub mod appearance;
pub mod candidate_enumerator;
pub mod care_objects;
pub mod ecology;
pub mod grounded_sensing;
pub mod habitat;
pub mod headless;
pub mod new_game;
pub mod organism;
pub mod persistence;
pub mod persistent_voxel;
pub mod presentation;
pub mod procedural_chunks;
pub mod scenario;
pub mod speech;
pub mod terrain_surface;
pub mod tracked_objects;

pub use appearance::*;
pub use candidate_enumerator::*;
pub use care_objects::*;
pub use ecology::*;
pub use grounded_sensing::*;
pub use habitat::*;
pub use headless::*;
pub use terrain_surface::{TerrainBinding, TerrainSurface};
mod terrain;
pub use new_game::*;
pub use organism::*;
pub use persistence::*;
pub use persistent_voxel::*;
pub use presentation::*;
pub use procedural_chunks::*;
pub use scenario::*;
pub use speech::*;
pub use terrain::{LocomotionLimits, TerrainData, TerrainState, WorldTerrain};
mod island;
pub use island::island_terrain;
pub use tracked_objects::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionLegality {
    Legal,
    ImpossibleTarget,
    BlockedByWorldState,
}

pub trait ActionLegalityChecker {
    fn check_action(&self, action: &ActionCommand) -> ActionLegality;
}
