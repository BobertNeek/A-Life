//! Translation from body-neutral core actions to Bevy-side action plans.
//!
//! This module does not report physical or biological outcomes. The authoritative
//! world and biology systems must measure those after they execute a plan.

use alife_core::{
    ActionCommand, ActionId, ActionKind, AffordanceBits, OrganismId, ScaffoldContractError,
    Validate, WorldEntityId,
};
use bevy::prelude::{Entity, Vec3};

use crate::math::{bevy_vec3_to_core, core_vec3_to_bevy};

pub const ACTION_APPROACH: ActionId = ActionId(101);
pub const ACTION_FLEE: ActionId = ActionId(102);
pub const ACTION_EAT: ActionId = ActionId(210);
pub const ACTION_GRAB: ActionId = ActionId(211);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BevyActionKind {
    Idle,
    Rest,
    Inspect,
    Move,
    Approach,
    Flee,
    Interact,
    Vocalize,
    Write,
    Gesture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BevyActionFailure {
    WrongOrganism {
        expected: OrganismId,
        actual: OrganismId,
    },
    MissingTarget(WorldEntityId),
    MissingTargetReference,
    MissingTargetPosition,
    MissingAffordance {
        target: WorldEntityId,
        required: AffordanceBits,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TargetAdapterState {
    pub entity: Entity,
    pub world_id: WorldEntityId,
    pub position: Vec3,
    pub affordances: AffordanceBits,
}

impl TargetAdapterState {
    pub const fn new(
        entity: Entity,
        world_id: WorldEntityId,
        position: Vec3,
        affordances: AffordanceBits,
    ) -> Self {
        Self {
            entity,
            world_id,
            position,
            affordances,
        }
    }

    fn validate(self) -> Result<Self, ScaffoldContractError> {
        self.world_id.validate()?;
        bevy_vec3_to_core(self.position)?;
        Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionAdapterContext {
    pub actor: Entity,
    pub organism_id: OrganismId,
    pub actor_world_id: WorldEntityId,
    pub actor_position: Vec3,
    pub movement_step_meters: f32,
    pub targets: Vec<TargetAdapterState>,
}

impl ActionAdapterContext {
    pub fn new(
        actor: Entity,
        organism_id: OrganismId,
        actor_world_id: WorldEntityId,
        actor_position: Vec3,
    ) -> Self {
        Self {
            actor,
            organism_id,
            actor_world_id,
            actor_position,
            movement_step_meters: 1.0,
            targets: Vec::new(),
        }
    }

    pub fn with_target(mut self, target: TargetAdapterState) -> Self {
        self.targets.push(target);
        self
    }

    pub fn target(&self, world_id: WorldEntityId) -> Option<TargetAdapterState> {
        self.targets
            .iter()
            .copied()
            .find(|target| target.world_id == world_id)
    }

    fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.actor_world_id.validate()?;
        bevy_vec3_to_core(self.actor_position)?;
        if !self.movement_step_meters.is_finite() || self.movement_step_meters < 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for (index, target) in self.targets.iter().copied().enumerate() {
            target.validate()?;
            if target.world_id == self.actor_world_id
                || self.targets[..index]
                    .iter()
                    .any(|previous| previous.world_id == target.world_id)
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevyActionPlan {
    pub actor: Entity,
    pub organism_id: OrganismId,
    pub action_id: ActionId,
    pub kind: BevyActionKind,
    pub target: Option<Entity>,
    pub target_world_id: Option<WorldEntityId>,
    /// Requested displacement for one simulation tick. The authoritative world
    /// still applies collision, actuator, and energy constraints.
    pub displacement: Vec3,
    pub rest_requested: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActionAdapterFeedback {
    pub plan: BevyActionPlan,
    pub failure: Option<BevyActionFailure>,
}

/// Translates an action without claiming that it succeeded in the world.
pub fn plan_action_command(
    command: &ActionCommand,
    context: &ActionAdapterContext,
) -> Result<ActionAdapterFeedback, ScaffoldContractError> {
    command.validate_contract()?;
    context.validate()?;
    if command.organism_id != context.organism_id {
        return Ok(failed_plan(
            command,
            context,
            BevyActionFailure::WrongOrganism {
                expected: context.organism_id,
                actual: command.organism_id,
            },
            None,
        ));
    }

    let kind = classify_action(command);
    let step = context.movement_step_meters * command.intensity.raw();
    match kind {
        BevyActionKind::Idle
        | BevyActionKind::Rest
        | BevyActionKind::Vocalize
        | BevyActionKind::Write
        | BevyActionKind::Gesture => Ok(successful_plan(
            command,
            context,
            kind,
            None,
            Vec3::ZERO,
            kind == BevyActionKind::Rest,
        )),
        BevyActionKind::Move => {
            let target = command.target_entity.and_then(|id| context.target(id));
            if let Some(target_id) = command.target_entity {
                if target.is_none() {
                    return Ok(failed_plan(
                        command,
                        context,
                        BevyActionFailure::MissingTarget(target_id),
                        Some(target_id),
                    ));
                }
            }
            let destination = if let Some(position) = command.target_position {
                core_vec3_to_bevy(position)?
            } else if let Some(target) = target {
                target.position
            } else {
                return Ok(failed_plan(
                    command,
                    context,
                    BevyActionFailure::MissingTargetPosition,
                    command.target_entity,
                ));
            };
            Ok(successful_plan(
                command,
                context,
                kind,
                target,
                step_toward(context.actor_position, destination, step),
                false,
            ))
        }
        BevyActionKind::Approach | BevyActionKind::Flee => {
            let Some(target_id) = command.target_entity else {
                return Ok(failed_plan(
                    command,
                    context,
                    BevyActionFailure::MissingTargetReference,
                    None,
                ));
            };
            let Some(target) = context.target(target_id) else {
                return Ok(failed_plan(
                    command,
                    context,
                    BevyActionFailure::MissingTarget(target_id),
                    Some(target_id),
                ));
            };
            let displacement = if kind == BevyActionKind::Approach {
                step_toward(context.actor_position, target.position, step)
            } else {
                step_away(context.actor_position, target.position, step)
            };
            Ok(successful_plan(
                command,
                context,
                kind,
                Some(target),
                displacement,
                false,
            ))
        }
        BevyActionKind::Inspect | BevyActionKind::Interact => {
            let Some(target_id) = command.target_entity else {
                return Ok(failed_plan(
                    command,
                    context,
                    BevyActionFailure::MissingTargetReference,
                    None,
                ));
            };
            let Some(target) = context.target(target_id) else {
                return Ok(failed_plan(
                    command,
                    context,
                    BevyActionFailure::MissingTarget(target_id),
                    Some(target_id),
                ));
            };
            if command.action_id == ACTION_EAT && !target.affordances.contains(AffordanceBits::FOOD)
            {
                return Ok(failed_plan(
                    command,
                    context,
                    BevyActionFailure::MissingAffordance {
                        target: target_id,
                        required: AffordanceBits::FOOD,
                    },
                    Some(target_id),
                ));
            }
            Ok(successful_plan(
                command,
                context,
                kind,
                Some(target),
                Vec3::ZERO,
                false,
            ))
        }
    }
}

/// Backward-compatible name. This only plans the command and never reports an
/// authoritative outcome.
pub fn execute_action_command(
    command: &ActionCommand,
    context: &ActionAdapterContext,
) -> Result<ActionAdapterFeedback, ScaffoldContractError> {
    plan_action_command(command, context)
}

fn classify_action(command: &ActionCommand) -> BevyActionKind {
    if command.action_id == ACTION_APPROACH {
        BevyActionKind::Approach
    } else if command.action_id == ACTION_FLEE {
        BevyActionKind::Flee
    } else if command.action_id == ACTION_EAT || command.action_id == ACTION_GRAB {
        BevyActionKind::Interact
    } else {
        match command.kind {
            ActionKind::Idle => BevyActionKind::Idle,
            ActionKind::Rest => BevyActionKind::Rest,
            ActionKind::Inspect => BevyActionKind::Inspect,
            ActionKind::Move => BevyActionKind::Move,
            ActionKind::Hold | ActionKind::Interact => BevyActionKind::Interact,
            ActionKind::Vocalize => BevyActionKind::Vocalize,
            ActionKind::Write => BevyActionKind::Write,
            ActionKind::Gesture => BevyActionKind::Gesture,
        }
    }
}

fn successful_plan(
    command: &ActionCommand,
    context: &ActionAdapterContext,
    kind: BevyActionKind,
    target: Option<TargetAdapterState>,
    displacement: Vec3,
    rest_requested: bool,
) -> ActionAdapterFeedback {
    ActionAdapterFeedback {
        plan: BevyActionPlan {
            actor: context.actor,
            organism_id: command.organism_id,
            action_id: command.action_id,
            kind,
            target: target.map(|target| target.entity),
            target_world_id: target.map(|target| target.world_id),
            displacement,
            rest_requested,
        },
        failure: None,
    }
}

fn failed_plan(
    command: &ActionCommand,
    context: &ActionAdapterContext,
    failure: BevyActionFailure,
    target_world_id: Option<WorldEntityId>,
) -> ActionAdapterFeedback {
    ActionAdapterFeedback {
        plan: BevyActionPlan {
            actor: context.actor,
            organism_id: command.organism_id,
            action_id: command.action_id,
            kind: classify_action(command),
            target: target_world_id.and_then(|id| context.target(id).map(|target| target.entity)),
            target_world_id,
            displacement: Vec3::ZERO,
            rest_requested: false,
        },
        failure: Some(failure),
    }
}

fn step_toward(start: Vec3, target: Vec3, step: f32) -> Vec3 {
    let delta = target - start;
    let length = delta.length();
    if length <= step || length == 0.0 {
        delta
    } else {
        delta / length * step
    }
}

fn step_away(start: Vec3, target: Vec3, step: f32) -> Vec3 {
    let delta = start - target;
    let length = delta.length();
    if length == 0.0 {
        Vec3::new(step, 0.0, 0.0)
    } else {
        delta / length * step
    }
}
