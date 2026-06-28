//! CA38 creature animation state-machine presentation contract.
//!
//! This module is app-facing and Bevy-free. It maps existing creature visual
//! state into bounded presentation poses. It does not alter cognition, action
//! arbitration, world execution, or persisted state.

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ca38CreaturePose {
    pub animation: CreatureAnimationState,
    pub pose_id: &'static str,
    pub action_label: &'static str,
    pub scale_x: f32,
    pub scale_y: f32,
    pub lean_x: f32,
    pub lean_y: f32,
    pub rotation_degrees: f32,
    pub pulse: f32,
    pub display_only: bool,
}

impl Ca38CreaturePose {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.pose_id.is_empty() || self.action_label.is_empty() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA38 creature pose must have readable labels",
            });
        }
        for value in [
            self.scale_x,
            self.scale_y,
            self.lean_x,
            self.lean_y,
            self.rotation_degrees,
            self.pulse,
        ] {
            if !value.is_finite() {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "CA38 creature pose values must be finite",
                });
            }
        }
        if !(0.45..=1.60).contains(&self.scale_x)
            || !(0.45..=1.60).contains(&self.scale_y)
            || self.lean_x.abs() > 18.0
            || self.lean_y.abs() > 18.0
            || self.rotation_degrees.abs() > 20.0
            || !(0.0..=1.0).contains(&self.pulse)
            || !self.display_only
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA38 creature pose must stay bounded and display-only",
            });
        }
        Ok(())
    }

    pub fn signature_part(self) -> String {
        format!(
            "{}:{:.2}:{:.2}:{:.1}:{:.1}:{:.1}:{:.2}:display_only={}",
            self.pose_id,
            self.scale_x,
            self.scale_y,
            self.lean_x,
            self.lean_y,
            self.rotation_degrees,
            self.pulse,
            self.display_only
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ca38CreatureAnimationSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub states: Vec<Ca38CreaturePose>,
    pub inspector_accurate: bool,
    pub fallback_visible: bool,
    pub stable_ids_only: bool,
    pub display_only: bool,
    pub no_action_authority: bool,
    pub no_cognition_mutation: bool,
    pub product_runtime_claim: &'static str,
}

impl Ca38CreatureAnimationSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA38_CREATURE_ANIMATION_SCHEMA
            || self.schema_version != CA38_CREATURE_ANIMATION_SCHEMA_VERSION
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA38 creature animation schema must be current",
            });
        }
        if self.states.len() < CA38_REQUIRED_ANIMATION_STATES {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA38 must cover required readable animation states",
            });
        }
        for pose in &self.states {
            pose.validate()?;
        }
        if !self.inspector_accurate
            || !self.fallback_visible
            || !self.stable_ids_only
            || !self.display_only
            || !self.no_action_authority
            || !self.no_cognition_mutation
            || self.product_runtime_claim != "CpuShadowGuardedStaticPlusLiveHShadow"
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA38 animation summary must preserve product boundaries",
            });
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:states={}:display_only={}:fallback_visible={}:claim={}:{}",
            self.schema,
            self.schema_version,
            self.states.len(),
            self.display_only,
            self.fallback_visible,
            self.product_runtime_claim,
            self.states
                .iter()
                .map(|pose| pose.signature_part())
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

pub fn ca38_creature_pose_for_state(
    animation: CreatureAnimationState,
    expression: CreatureExpressionState,
) -> Ca38CreaturePose {
    let mut pose = match animation {
        CreatureAnimationState::Idle => Ca38CreaturePose {
            animation,
            pose_id: "idle-breathe",
            action_label: "idle",
            scale_x: 1.00,
            scale_y: 0.96,
            lean_x: 0.0,
            lean_y: 0.0,
            rotation_degrees: 0.0,
            pulse: 0.18,
            display_only: true,
        },
        CreatureAnimationState::Moving => Ca38CreaturePose {
            animation,
            pose_id: "move-lean",
            action_label: "moving",
            scale_x: 1.16,
            scale_y: 0.82,
            lean_x: 11.0,
            lean_y: 0.0,
            rotation_degrees: -5.0,
            pulse: 0.42,
            display_only: true,
        },
        CreatureAnimationState::Interacting => Ca38CreaturePose {
            animation,
            pose_id: "eat-reach",
            action_label: "eat",
            scale_x: 1.06,
            scale_y: 0.86,
            lean_x: 5.0,
            lean_y: -8.0,
            rotation_degrees: 0.0,
            pulse: 0.52,
            display_only: true,
        },
        CreatureAnimationState::Afraid => Ca38CreaturePose {
            animation,
            pose_id: "flee-alert",
            action_label: "flee",
            scale_x: 1.20,
            scale_y: 0.78,
            lean_x: -13.0,
            lean_y: 4.0,
            rotation_degrees: 7.5,
            pulse: 0.68,
            display_only: true,
        },
        CreatureAnimationState::Sleeping => Ca38CreaturePose {
            animation,
            pose_id: "sleep-curl",
            action_label: "sleep",
            scale_x: 0.86,
            scale_y: 0.72,
            lean_x: 0.0,
            lean_y: -6.0,
            rotation_degrees: 0.0,
            pulse: 0.24,
            display_only: true,
        },
        CreatureAnimationState::Hurt => Ca38CreaturePose {
            animation,
            pose_id: "pain-flinch",
            action_label: "pain",
            scale_x: 0.94,
            scale_y: 1.16,
            lean_x: -6.0,
            lean_y: 5.0,
            rotation_degrees: -10.0,
            pulse: 0.82,
            display_only: true,
        },
        CreatureAnimationState::Signaling => Ca38CreaturePose {
            animation,
            pose_id: "social-signal",
            action_label: "social",
            scale_x: 1.04,
            scale_y: 1.02,
            lean_x: 0.0,
            lean_y: 8.0,
            rotation_degrees: 0.0,
            pulse: 0.58,
            display_only: true,
        },
        CreatureAnimationState::Inspecting => Ca38CreaturePose {
            animation,
            pose_id: "inspect-focus",
            action_label: "inspect",
            scale_x: 0.98,
            scale_y: 1.06,
            lean_x: 6.0,
            lean_y: 4.0,
            rotation_degrees: 3.0,
            pulse: 0.46,
            display_only: true,
        },
        CreatureAnimationState::Resting => Ca38CreaturePose {
            animation,
            pose_id: "rest-low",
            action_label: "rest",
            scale_x: 0.92,
            scale_y: 0.78,
            lean_x: 0.0,
            lean_y: -4.0,
            rotation_degrees: 0.0,
            pulse: 0.22,
            display_only: true,
        },
        CreatureAnimationState::Curious => Ca38CreaturePose {
            animation,
            pose_id: "curious-tilt",
            action_label: "curious",
            scale_x: 0.98,
            scale_y: 1.08,
            lean_x: 4.0,
            lean_y: 6.0,
            rotation_degrees: 8.0,
            pulse: 0.50,
            display_only: true,
        },
    };

    match expression {
        CreatureExpressionState::Pained => {
            pose.pose_id = "pain-flinch";
            pose.action_label = "pain";
            pose.pulse = pose.pulse.max(0.82);
        }
        CreatureExpressionState::Afraid => {
            pose.pose_id = "flee-alert";
            pose.action_label = "flee";
            pose.pulse = pose.pulse.max(0.68);
        }
        CreatureExpressionState::Tired if animation != CreatureAnimationState::Sleeping => {
            pose.scale_y = pose.scale_y.min(0.86);
            pose.lean_y = pose.lean_y.min(-3.0);
        }
        CreatureExpressionState::Energized => {
            pose.pulse = pose.pulse.max(0.55);
            pose.scale_x = (pose.scale_x + 0.04).min(1.60);
        }
        CreatureExpressionState::Hungry
        | CreatureExpressionState::Tired
        | CreatureExpressionState::Curious
        | CreatureExpressionState::Neutral => {}
    }
    pose
}

pub fn ca38_animation_label_line(snapshot: &CreatureVisualSnapshot) -> String {
    let pose = ca38_creature_pose_for_state(snapshot.animation, snapshot.expression);
    format!(
        "{} / {} / pose={}",
        snapshot.animation.label(),
        snapshot.expression.label(),
        pose.pose_id
    )
}

pub fn ca38_creature_animation_summary() -> Result<Ca38CreatureAnimationSummary, GameAppShellError>
{
    let cases = [
        (
            CreatureAnimationState::Idle,
            CreatureExpressionState::Neutral,
        ),
        (
            CreatureAnimationState::Moving,
            CreatureExpressionState::Energized,
        ),
        (
            CreatureAnimationState::Interacting,
            CreatureExpressionState::Hungry,
        ),
        (
            CreatureAnimationState::Afraid,
            CreatureExpressionState::Afraid,
        ),
        (
            CreatureAnimationState::Sleeping,
            CreatureExpressionState::Tired,
        ),
        (
            CreatureAnimationState::Hurt,
            CreatureExpressionState::Pained,
        ),
        (
            CreatureAnimationState::Signaling,
            CreatureExpressionState::Curious,
        ),
        (
            CreatureAnimationState::Inspecting,
            CreatureExpressionState::Curious,
        ),
    ];
    let summary = Ca38CreatureAnimationSummary {
        schema: CA38_CREATURE_ANIMATION_SCHEMA,
        schema_version: CA38_CREATURE_ANIMATION_SCHEMA_VERSION,
        states: cases
            .into_iter()
            .map(|(animation, expression)| ca38_creature_pose_for_state(animation, expression))
            .collect(),
        inspector_accurate: true,
        fallback_visible: true,
        stable_ids_only: true,
        display_only: true,
        no_action_authority: true,
        no_cognition_mutation: true,
        product_runtime_claim: "CpuShadowGuardedStaticPlusLiveHShadow",
    };
    summary.validate()?;
    Ok(summary)
}

pub fn run_creature_animation_state_machine_smoke(
) -> Result<Ca38CreatureAnimationSummary, GameAppShellError> {
    ca38_creature_animation_summary()
}
