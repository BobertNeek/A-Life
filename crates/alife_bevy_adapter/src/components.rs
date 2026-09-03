//! Bevy ECS components for derived adapter input and output.

use alife_core::{
    ActionCommand, AffordanceBits, OrganismId, ScaffoldContractError, SensorySnapshot,
    WorldEntityId,
};
use bevy::prelude::Component;

use crate::action::{BevyActionFailure, BevyActionPlan};

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

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        self.organism_id.validate()?;
        self.world_entity_id.validate()?;
        for value in [
            self.radius_meters,
            self.movement_step_meters,
            self.vision_radius_meters,
            self.hearing_radius_meters,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        Ok(self)
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

    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        if !self.nutrition.is_finite()
            || !self.hazard_pain.is_finite()
            || self.nutrition < 0.0
            || self.hazard_pain < 0.0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(self)
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

impl SensoryEmitter {
    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        if self.audible_token.is_some_and(|token| {
            token == 0 || token >= u32::from(alife_core::LanguageCodebookV1::CODE_COUNT)
        }) {
            return Err(ScaffoldContractError::InvalidId);
        }
        for value in [
            self.visual_salience_scale,
            self.smell_salience_scale,
            self.audible_radius_meters,
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct ActionSink {
    pub pending_command: Option<ActionCommand>,
    pub last_plan: Option<BevyActionPlan>,
    pub last_failure: Option<BevyActionFailure>,
    pub last_contract_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Component)]
pub struct LatestSensorySnapshot(pub SensorySnapshot);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterStage {
    GatherSensory,
    PlanAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Component)]
pub struct AdapterContractFailure {
    pub stage: AdapterStage,
    pub message: String,
}
