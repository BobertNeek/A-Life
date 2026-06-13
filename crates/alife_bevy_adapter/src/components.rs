//! v0 scaffold: Bevy ECS mirror components and telemetry resources.

use alife_core::{
    ActionCommand, ActionProposal, AffordanceBits, BrainTickOutput, CreatureMind, ExperiencePatch,
    HomeostaticSnapshot, OrganismId, SensorySnapshot, SleepPhase, Tick, WorldEntityId,
};
use bevy::prelude::{Component, Resource};

use crate::action::BevyActionFailure;

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct CreatureBody {
    pub organism_id: OrganismId,
    pub world_entity_id: WorldEntityId,
    pub radius_meters: f32,
    pub movement_step_meters: f32,
    pub vision_radius_meters: f32,
    pub hearing_radius_meters: f32,
}

impl CreatureBody {
    pub fn new(
        organism_id: OrganismId,
        world_entity_id: WorldEntityId,
    ) -> Result<Self, alife_core::ScaffoldContractError> {
        organism_id.validate()?;
        world_entity_id.validate()?;
        Ok(Self {
            organism_id,
            world_entity_id,
            radius_meters: 0.5,
            movement_step_meters: 1.0,
            vision_radius_meters: crate::sensory::DEFAULT_VISION_RADIUS,
            hearing_radius_meters: crate::sensory::DEFAULT_HEARING_RADIUS,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct AffordanceTags {
    pub bits: AffordanceBits,
    pub nutrition: f32,
    pub hazard_pain: f32,
    pub blocks_movement: bool,
}

impl AffordanceTags {
    pub const fn new(bits: AffordanceBits) -> Self {
        Self {
            bits,
            nutrition: 0.0,
            hazard_pain: 0.0,
            blocks_movement: false,
        }
    }

    pub const fn food(nutrition: f32) -> Self {
        Self {
            bits: AffordanceBits::FOOD,
            nutrition,
            hazard_pain: 0.0,
            blocks_movement: false,
        }
    }

    pub const fn hazard(pain: f32) -> Self {
        Self {
            bits: AffordanceBits::HAZARD,
            nutrition: 0.0,
            hazard_pain: pain,
            blocks_movement: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub struct SensoryEmitter {
    pub audible_token: Option<u32>,
    pub visual_salience_scale: f32,
    pub smell_salience_scale: f32,
    pub audible_radius_meters: f32,
}

impl Default for SensoryEmitter {
    fn default() -> Self {
        Self {
            audible_token: None,
            visual_salience_scale: 1.0,
            smell_salience_scale: 1.0,
            audible_radius_meters: crate::sensory::DEFAULT_HEARING_RADIUS,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct ActionSink {
    pub pending_command: Option<ActionCommand>,
    pub last_execution: Option<alife_core::ReferenceActionExecution>,
    pub last_failure: Option<BevyActionFailure>,
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct SleepDriveDebug {
    pub tick: Tick,
    pub sleep_phase: Option<SleepPhase>,
    pub homeostasis: Option<HomeostaticSnapshot>,
}

impl Default for SleepDriveDebug {
    fn default() -> Self {
        Self {
            tick: Tick::ZERO,
            sleep_phase: None,
            homeostasis: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct LatestSensorySnapshot(pub SensorySnapshot);

#[derive(Debug, Clone, PartialEq, Component)]
pub struct CoreBrainMind(pub CreatureMind);

#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct BrainTickProposals {
    pub proposals: Vec<ActionProposal>,
}

#[derive(Debug, Clone, Default, PartialEq, Resource)]
pub struct PatchTelemetry {
    pub sealed_patches: Vec<ExperiencePatch>,
    pub brain_outputs: Vec<BrainTickOutput>,
}
