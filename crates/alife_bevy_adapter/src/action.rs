//! v0 scaffold: core `ActionCommand` to Bevy/Avian-side action plans.

use alife_core::{
    ActionCommand, ActionId, ActionKind, AffordanceBits, DriveDelta, EndocrineDelta,
    HomeostaticDelta, NormalizedScalar, OrganismId, PhysicalActionOutcome, PhysicalContactKind,
    ReferenceActionExecution, ReferenceActionExecutor, ReferenceActionFailure,
    ReferenceOutcomeObservation, ReferenceOutcomeObserver, ReferenceOutcomeRequest,
    ScaffoldContractError, SignedValence, Validate, WorldEntityId,
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
    MissingTarget(WorldEntityId),
    MissingTargetPosition,
    MissingAffordance {
        target: WorldEntityId,
        required: AffordanceBits,
    },
    UnsupportedAction(ActionKind),
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionAdapterContext {
    pub actor: Entity,
    pub actor_world_id: WorldEntityId,
    pub actor_position: Vec3,
    pub movement_step_meters: f32,
    pub targets: Vec<TargetAdapterState>,
}

impl ActionAdapterContext {
    pub fn new(actor: Entity, actor_world_id: WorldEntityId, actor_position: Vec3) -> Self {
        Self {
            actor,
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
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BevyActionPlan {
    pub actor: Entity,
    pub organism_id: OrganismId,
    pub action_id: ActionId,
    pub kind: BevyActionKind,
    pub target: Option<Entity>,
    pub target_world_id: Option<WorldEntityId>,
    pub displacement: Vec3,
    pub rest_requested: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionAdapterFeedback {
    pub plan: BevyActionPlan,
    pub execution: ReferenceActionExecution,
    pub observation: ReferenceOutcomeObservation,
    pub failure: Option<BevyActionFailure>,
}

pub fn execute_action_command(
    command: &ActionCommand,
    context: &ActionAdapterContext,
) -> Result<ActionAdapterFeedback, ScaffoldContractError> {
    command.validate_contract()?;
    context.actor_world_id.validate()?;
    bevy_vec3_to_core(context.actor_position)?;

    let kind = classify_action(command);
    match kind {
        BevyActionKind::Idle => successful_feedback(
            command,
            context,
            SuccessfulFeedbackSpec {
                kind,
                target: None,
                displacement: Vec3::ZERO,
                contact: PhysicalContactKind::None,
                profile: OutcomeProfile::idle(),
                rest_requested: false,
            },
        ),
        BevyActionKind::Rest => successful_feedback(
            command,
            context,
            SuccessfulFeedbackSpec {
                kind,
                target: None,
                displacement: Vec3::ZERO,
                contact: PhysicalContactKind::None,
                profile: OutcomeProfile::rest(),
                rest_requested: true,
            },
        ),
        BevyActionKind::Move => {
            let displacement = if let Some(position) = command.target_position {
                core_vec3_to_bevy(position)? - context.actor_position
            } else if let Some(target_id) = command.target_entity {
                let Some(target) = context.target(target_id) else {
                    return failed_missing_target(command, context, kind, target_id);
                };
                target.position - context.actor_position
            } else {
                return failed_no_position(command, context, kind);
            };
            successful_feedback(
                command,
                context,
                SuccessfulFeedbackSpec {
                    kind,
                    target: command.target_entity.and_then(|id| context.target(id)),
                    displacement,
                    contact: PhysicalContactKind::Moved,
                    profile: OutcomeProfile::movement(),
                    rest_requested: false,
                },
            )
        }
        BevyActionKind::Approach | BevyActionKind::Flee => {
            let target_id = command
                .target_entity
                .ok_or(ScaffoldContractError::InvalidId)?;
            let Some(target) = context.target(target_id) else {
                return failed_missing_target(command, context, kind, target_id);
            };
            let displacement = if kind == BevyActionKind::Approach {
                step_toward(
                    context.actor_position,
                    target.position,
                    context.movement_step_meters,
                )
            } else {
                step_away(
                    context.actor_position,
                    target.position,
                    context.movement_step_meters,
                )
            };
            successful_feedback(
                command,
                context,
                SuccessfulFeedbackSpec {
                    kind,
                    target: Some(target),
                    displacement,
                    contact: PhysicalContactKind::Moved,
                    profile: OutcomeProfile::movement(),
                    rest_requested: false,
                },
            )
        }
        BevyActionKind::Inspect => {
            let target_id = command
                .target_entity
                .ok_or(ScaffoldContractError::InvalidId)?;
            let Some(target) = context.target(target_id) else {
                return failed_missing_target(command, context, kind, target_id);
            };
            successful_feedback(
                command,
                context,
                SuccessfulFeedbackSpec {
                    kind,
                    target: Some(target),
                    displacement: Vec3::ZERO,
                    contact: PhysicalContactKind::Touch,
                    profile: OutcomeProfile::inspect(),
                    rest_requested: false,
                },
            )
        }
        BevyActionKind::Interact => {
            let target_id = command
                .target_entity
                .ok_or(ScaffoldContractError::InvalidId)?;
            let Some(target) = context.target(target_id) else {
                return failed_missing_target(command, context, kind, target_id);
            };
            if command.action_id == ACTION_EAT && !target.affordances.contains(AffordanceBits::FOOD)
            {
                return failed_missing_affordance(
                    command,
                    context,
                    kind,
                    target,
                    AffordanceBits::FOOD,
                );
            }
            let contact = if command.action_id == ACTION_EAT {
                PhysicalContactKind::Consumed
            } else {
                PhysicalContactKind::Touch
            };
            successful_feedback(
                command,
                context,
                SuccessfulFeedbackSpec {
                    kind,
                    target: Some(target),
                    displacement: Vec3::ZERO,
                    contact,
                    profile: OutcomeProfile::interact(command.action_id == ACTION_EAT),
                    rest_requested: false,
                },
            )
        }
        BevyActionKind::Vocalize | BevyActionKind::Write | BevyActionKind::Gesture => {
            successful_feedback(
                command,
                context,
                SuccessfulFeedbackSpec {
                    kind,
                    target: None,
                    displacement: Vec3::ZERO,
                    contact: PhysicalContactKind::None,
                    profile: OutcomeProfile::vocalize(),
                    rest_requested: false,
                },
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BevyReferenceActionAdapter {
    context: ActionAdapterContext,
    last_feedback: Option<ActionAdapterFeedback>,
}

impl BevyReferenceActionAdapter {
    pub fn new(context: ActionAdapterContext) -> Self {
        Self {
            context,
            last_feedback: None,
        }
    }

    pub const fn last_feedback(&self) -> Option<&ActionAdapterFeedback> {
        self.last_feedback.as_ref()
    }
}

impl ReferenceActionExecutor for BevyReferenceActionAdapter {
    fn execute_action(
        &mut self,
        command: &ActionCommand,
    ) -> Result<ReferenceActionExecution, ScaffoldContractError> {
        let feedback = execute_action_command(command, &self.context)?;
        let execution = feedback.execution;
        self.last_feedback = Some(feedback);
        Ok(execution)
    }
}

impl ReferenceOutcomeObserver for BevyReferenceActionAdapter {
    fn observe_outcome(
        &mut self,
        request: ReferenceOutcomeRequest<'_>,
    ) -> Result<ReferenceOutcomeObservation, ScaffoldContractError> {
        let Some(feedback) = &self.last_feedback else {
            return Err(ScaffoldContractError::InvalidActionDecision);
        };
        if feedback.execution != *request.execution
            || feedback.plan.action_id != request.command.action_id
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        Ok(feedback.observation.clone())
    }
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

#[derive(Debug, Clone, Copy)]
struct SuccessfulFeedbackSpec {
    kind: BevyActionKind,
    target: Option<TargetAdapterState>,
    displacement: Vec3,
    contact: PhysicalContactKind,
    profile: OutcomeProfile,
    rest_requested: bool,
}

fn successful_feedback(
    command: &ActionCommand,
    context: &ActionAdapterContext,
    spec: SuccessfulFeedbackSpec,
) -> Result<ActionAdapterFeedback, ScaffoldContractError> {
    let target_world_id = spec.target.map(|target| target.world_id);
    let plan = plan_for(
        command,
        context,
        spec.kind,
        spec.target,
        spec.displacement,
        spec.rest_requested,
    );
    let execution = ReferenceActionExecution::succeeded(physical(
        spec.contact,
        target_world_id,
        spec.displacement,
        spec.profile.energy_cost,
    )?)?;
    Ok(ActionAdapterFeedback {
        plan,
        execution,
        observation: spec.profile.observation(true)?,
        failure: None,
    })
}

fn failed_missing_target(
    command: &ActionCommand,
    context: &ActionAdapterContext,
    kind: BevyActionKind,
    target: WorldEntityId,
) -> Result<ActionAdapterFeedback, ScaffoldContractError> {
    failed_feedback(
        command,
        context,
        FailedFeedbackSpec {
            kind,
            target: None,
            target_world_id: Some(target),
            failure: BevyActionFailure::MissingTarget(target),
            core_failure: ReferenceActionFailure::ActionRejected,
            profile: OutcomeProfile::invalid_target(),
        },
    )
}

fn failed_no_position(
    command: &ActionCommand,
    context: &ActionAdapterContext,
    kind: BevyActionKind,
) -> Result<ActionAdapterFeedback, ScaffoldContractError> {
    failed_feedback(
        command,
        context,
        FailedFeedbackSpec {
            kind,
            target: None,
            target_world_id: command.target_entity,
            failure: BevyActionFailure::MissingTargetPosition,
            core_failure: ReferenceActionFailure::ActionRejected,
            profile: OutcomeProfile::invalid_target(),
        },
    )
}

fn failed_missing_affordance(
    command: &ActionCommand,
    context: &ActionAdapterContext,
    kind: BevyActionKind,
    target: TargetAdapterState,
    required: AffordanceBits,
) -> Result<ActionAdapterFeedback, ScaffoldContractError> {
    failed_feedback(
        command,
        context,
        FailedFeedbackSpec {
            kind,
            target: Some(target),
            target_world_id: Some(target.world_id),
            failure: BevyActionFailure::MissingAffordance {
                target: target.world_id,
                required,
            },
            core_failure: ReferenceActionFailure::MissingAffordance,
            profile: OutcomeProfile::missing_affordance(),
        },
    )
}

#[derive(Debug, Clone, Copy)]
struct FailedFeedbackSpec {
    kind: BevyActionKind,
    target: Option<TargetAdapterState>,
    target_world_id: Option<WorldEntityId>,
    failure: BevyActionFailure,
    core_failure: ReferenceActionFailure,
    profile: OutcomeProfile,
}

fn failed_feedback(
    command: &ActionCommand,
    context: &ActionAdapterContext,
    spec: FailedFeedbackSpec,
) -> Result<ActionAdapterFeedback, ScaffoldContractError> {
    let plan = plan_for(command, context, spec.kind, spec.target, Vec3::ZERO, false);
    let execution = ReferenceActionExecution::failed(
        spec.core_failure,
        physical(
            PhysicalContactKind::Blocked,
            spec.target_world_id,
            Vec3::ZERO,
            spec.profile.energy_cost,
        )?,
    )?;
    Ok(ActionAdapterFeedback {
        plan,
        execution,
        observation: spec.profile.observation(false)?,
        failure: Some(spec.failure),
    })
}

fn plan_for(
    command: &ActionCommand,
    context: &ActionAdapterContext,
    kind: BevyActionKind,
    target: Option<TargetAdapterState>,
    displacement: Vec3,
    rest_requested: bool,
) -> BevyActionPlan {
    BevyActionPlan {
        actor: context.actor,
        organism_id: command.organism_id,
        action_id: command.action_id,
        kind,
        target: target.map(|target| target.entity),
        target_world_id: target.map(|target| target.world_id),
        displacement,
        rest_requested,
    }
}

fn physical(
    contact: PhysicalContactKind,
    target_entity: Option<WorldEntityId>,
    displacement: Vec3,
    energy_cost: f32,
) -> Result<PhysicalActionOutcome, ScaffoldContractError> {
    let outcome = PhysicalActionOutcome {
        contact,
        target_entity,
        displacement: bevy_vec3_to_core(displacement)?,
        collision_normal: None,
        energy_cost: NormalizedScalar::new(energy_cost.clamp(0.0, 1.0))?,
    };
    outcome.validate_contract()?;
    Ok(outcome)
}

#[derive(Debug, Clone, Copy)]
struct OutcomeProfile {
    homeostatic_delta: HomeostaticDelta,
    reward: f32,
    frustration: f32,
    pain: f32,
    energy: f32,
    prediction_error: f32,
    contradiction: bool,
    energy_cost: f32,
}

impl OutcomeProfile {
    fn idle() -> Self {
        Self::new(0.0, 0.0, 0.0, -0.01, 0.05, false, 0.0)
    }

    fn rest() -> Self {
        Self {
            homeostatic_delta: HomeostaticDelta {
                drives: DriveDelta {
                    fatigue: -0.25,
                    brain_atp: 0.08,
                    ..DriveDelta::zero()
                },
                hormones: EndocrineDelta {
                    sleep_pressure: -0.08,
                    serotonin: 0.03,
                    ..EndocrineDelta::zero()
                },
            },
            reward: 0.08,
            frustration: 0.0,
            pain: 0.0,
            energy: 0.06,
            prediction_error: 0.05,
            contradiction: false,
            energy_cost: 0.0,
        }
    }

    fn inspect() -> Self {
        Self::new(0.04, 0.0, 0.0, -0.01, 0.1, false, 0.02)
    }

    fn movement() -> Self {
        Self::new(0.0, 0.0, 0.0, -0.04, 0.08, false, 0.05)
    }

    fn interact(food: bool) -> Self {
        if food {
            Self {
                homeostatic_delta: HomeostaticDelta {
                    drives: DriveDelta {
                        hunger: -0.35,
                        brain_atp: 0.12,
                        ..DriveDelta::zero()
                    },
                    hormones: EndocrineDelta {
                        dopamine: 0.12,
                        ..EndocrineDelta::zero()
                    },
                },
                reward: 0.65,
                frustration: 0.0,
                pain: 0.0,
                energy: 0.15,
                prediction_error: 0.05,
                contradiction: false,
                energy_cost: 0.03,
            }
        } else {
            Self::new(0.06, 0.0, 0.0, -0.03, 0.1, false, 0.04)
        }
    }

    fn vocalize() -> Self {
        Self::new(0.04, 0.0, 0.0, -0.01, 0.1, false, 0.02)
    }

    fn missing_affordance() -> Self {
        Self::new(-0.35, 0.65, 0.0, -0.02, 0.85, true, 0.04)
    }

    fn invalid_target() -> Self {
        Self::new(-0.4, 0.7, 0.0, -0.01, 0.9, true, 0.02)
    }

    fn new(
        reward: f32,
        frustration: f32,
        pain: f32,
        energy: f32,
        prediction_error: f32,
        contradiction: bool,
        energy_cost: f32,
    ) -> Self {
        Self {
            homeostatic_delta: HomeostaticDelta {
                drives: DriveDelta::zero(),
                hormones: EndocrineDelta::zero(),
            },
            reward,
            frustration,
            pain,
            energy,
            prediction_error,
            contradiction,
            energy_cost,
        }
    }

    fn observation(
        self,
        success: bool,
    ) -> Result<ReferenceOutcomeObservation, ScaffoldContractError> {
        let mut observation = ReferenceOutcomeObservation::new(
            success,
            self.homeostatic_delta,
            SignedValence::new(self.reward.clamp(-1.0, 1.0))?,
            NormalizedScalar::new(self.frustration.clamp(0.0, 1.0))?,
            NormalizedScalar::new(self.pain.clamp(0.0, 1.0))?,
            SignedValence::new(self.energy.clamp(-1.0, 1.0))?,
            NormalizedScalar::new(self.prediction_error.clamp(0.0, 1.0))?,
        )?;
        observation.contradiction_observed = self.contradiction || !success;
        Ok(observation)
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
