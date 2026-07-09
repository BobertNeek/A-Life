//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;
use alife_core::EndocrineSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CreatureAnimationState {
    Idle,
    Moving,
    Inspecting,
    Interacting,
    Resting,
    Sleeping,
    Signaling,
    Hurt,
    Afraid,
    Curious,
}

impl CreatureAnimationState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Moving => "moving",
            Self::Inspecting => "inspecting",
            Self::Interacting => "interacting",
            Self::Resting => "resting",
            Self::Sleeping => "sleeping",
            Self::Signaling => "signaling",
            Self::Hurt => "hurt",
            Self::Afraid => "afraid",
            Self::Curious => "curious",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CreatureExpressionState {
    Neutral,
    Hungry,
    Tired,
    Afraid,
    Pained,
    Curious,
    Energized,
}

impl CreatureExpressionState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Neutral => "neutral",
            Self::Hungry => "hungry",
            Self::Tired => "tired",
            Self::Afraid => "afraid",
            Self::Pained => "pained",
            Self::Curious => "curious",
            Self::Energized => "energized",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureVisualCue {
    pub value: f32,
    pub rgba: [f32; 4],
}

impl CreatureVisualCue {
    pub fn new(value: f32, rgba: [f32; 4]) -> Result<Self, ScaffoldContractError> {
        let value = NormalizedScalar::new(value)?.raw();
        validate_rgba(rgba)?;
        Ok(Self { value, rgba })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureVisualCueSet {
    pub hunger: CreatureVisualCue,
    pub fatigue: CreatureVisualCue,
    pub fear: CreatureVisualCue,
    pub pain: CreatureVisualCue,
    pub curiosity: CreatureVisualCue,
    pub energy: CreatureVisualCue,
    pub sleep_pressure: CreatureVisualCue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureVisualSnapshot {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub stable_id: WorldEntityId,
    pub position: Vec3f,
    pub facing: Vec3f,
    pub sleep_phase: SleepPhase,
    pub animation: CreatureAnimationState,
    pub expression: CreatureExpressionState,
    pub selected_action_kind: Option<ActionKind>,
    pub target_entity: Option<WorldEntityId>,
    pub base_rgba: [f32; 4],
    pub accent_rgba: [f32; 4],
    pub intent_rgba: [f32; 4],
    pub appearance: CreatureAppearanceGenome,
    pub cues: CreatureVisualCueSet,
    pub endocrine: EndocrineSnapshot,
    pub debug_summary: String,
}

impl CreatureVisualSnapshot {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.stable_id.validate()?;
        self.position.validate()?;
        self.facing.validate()?;
        if let Some(target) = self.target_entity {
            target.validate()?;
        }
        validate_rgba(self.base_rgba)?;
        validate_rgba(self.accent_rgba)?;
        validate_rgba(self.intent_rgba)?;
        self.appearance.validate()?;
        for cue in [
            self.cues.hunger,
            self.cues.fatigue,
            self.cues.fear,
            self.cues.pain,
            self.cues.curiosity,
            self.cues.energy,
            self.cues.sleep_pressure,
        ] {
            NormalizedScalar::new(cue.value)?;
            validate_rgba(cue.rgba)?;
        }
        self.endocrine.validate_contract()?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{:?}:{:?}:{:?}:{:?}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}",
            self.organism_id.raw(),
            self.stable_id.raw(),
            self.sleep_phase,
            self.animation,
            self.expression,
            self.selected_action_kind,
            self.position.x,
            self.position.y,
            self.position.z,
            self.facing.x,
            self.facing.y,
            self.facing.z,
            self.cues.energy.value
        )
    }
}

pub fn creature_visual_snapshot_from_presentation(
    presentation: &VisibleWorldPresentation,
    organism_id: OrganismId,
    mind: &CreatureMind,
    last_tick: Option<&LiveBrainTickSummary>,
) -> Result<CreatureVisualSnapshot, GameAppShellError> {
    let creature = presentation
        .objects
        .iter()
        .find(|object| {
            object.kind == WorldObjectKind::Agent && object.organism_id == Some(organism_id)
        })
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "visible presentation must include the creature agent",
        })?;
    let target = last_tick.and_then(|summary| summary.target_entity);
    let target_position = target.and_then(|target_id| {
        presentation
            .objects
            .iter()
            .find(|object| object.stable_id == target_id)
            .map(|object| object.position)
    });
    let snapshot = creature_visual_snapshot_from_parts(
        organism_id,
        creature.stable_id,
        creature.position,
        target,
        target_position,
        mind.homeostasis(),
        mind.sleep_state().phase,
        last_tick.and_then(|summary| summary.selected_action_kind),
    )?;
    snapshot.validate()?;
    Ok(snapshot)
}

#[allow(clippy::too_many_arguments)]
pub fn creature_visual_snapshot_from_parts(
    organism_id: OrganismId,
    stable_id: WorldEntityId,
    position: Vec3f,
    target_entity: Option<WorldEntityId>,
    target_position: Option<Vec3f>,
    homeostasis: &HomeostaticSnapshot,
    sleep_phase: SleepPhase,
    selected_action_kind: Option<ActionKind>,
) -> Result<CreatureVisualSnapshot, GameAppShellError> {
    creature_visual_snapshot_from_parts_with_appearance(
        organism_id,
        stable_id,
        position,
        target_entity,
        target_position,
        homeostasis,
        sleep_phase,
        selected_action_kind,
        CreatureAppearanceGenome::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn creature_visual_snapshot_from_parts_with_appearance(
    organism_id: OrganismId,
    stable_id: WorldEntityId,
    position: Vec3f,
    target_entity: Option<WorldEntityId>,
    target_position: Option<Vec3f>,
    homeostasis: &HomeostaticSnapshot,
    sleep_phase: SleepPhase,
    selected_action_kind: Option<ActionKind>,
    appearance: CreatureAppearanceGenome,
) -> Result<CreatureVisualSnapshot, GameAppShellError> {
    homeostasis.validate_contract()?;
    appearance.validate()?;
    position.validate()?;
    if let Some(target) = target_entity {
        target.validate()?;
    }
    if let Some(target_position) = target_position {
        target_position.validate()?;
    }

    let cues = creature_visual_cues(homeostasis)?;
    let animation = creature_animation_state(sleep_phase, homeostasis, selected_action_kind);
    let expression = creature_expression_state(sleep_phase, homeostasis);
    let base_rgba = creature_base_rgba(homeostasis)?;
    let accent_rgba = creature_expression_rgba(expression);
    let intent_rgba = action_intent_rgba(selected_action_kind);
    validate_rgba(base_rgba)?;
    validate_rgba(accent_rgba)?;
    validate_rgba(intent_rgba)?;

    let snapshot = CreatureVisualSnapshot {
        schema: G04_CREATURE_VISUAL_SCHEMA,
        schema_version: G04_CREATURE_VISUAL_SCHEMA_VERSION,
        organism_id,
        stable_id,
        position,
        facing: facing_from_target(position, target_position)?,
        sleep_phase,
        animation,
        expression,
        selected_action_kind,
        target_entity,
        base_rgba,
        accent_rgba,
        intent_rgba,
        appearance,
        cues,
        endocrine: homeostasis.hormones,
        debug_summary: format!(
            "organism={} animation={} expression={} action={:?} sleep={:?}",
            organism_id.raw(),
            animation.label(),
            expression.label(),
            selected_action_kind,
            sleep_phase
        ),
    };
    snapshot.validate()?;
    Ok(snapshot)
}

fn creature_visual_cues(
    homeostasis: &HomeostaticSnapshot,
) -> Result<CreatureVisualCueSet, ScaffoldContractError> {
    Ok(CreatureVisualCueSet {
        hunger: CreatureVisualCue::new(homeostasis.drives.hunger, [0.18, 0.78, 0.30, 1.0])?,
        fatigue: CreatureVisualCue::new(homeostasis.drives.fatigue, [0.48, 0.52, 0.90, 1.0])?,
        fear: CreatureVisualCue::new(homeostasis.drives.fear, [0.92, 0.62, 0.18, 1.0])?,
        pain: CreatureVisualCue::new(homeostasis.drives.pain, [0.92, 0.16, 0.18, 1.0])?,
        curiosity: CreatureVisualCue::new(homeostasis.drives.curiosity, [0.96, 0.84, 0.20, 1.0])?,
        energy: CreatureVisualCue::new(homeostasis.drives.brain_atp, [0.20, 0.62, 0.95, 1.0])?,
        sleep_pressure: CreatureVisualCue::new(
            homeostasis.hormones.sleep_pressure,
            [0.52, 0.44, 0.86, 1.0],
        )?,
    })
}

fn creature_animation_state(
    sleep_phase: SleepPhase,
    homeostasis: &HomeostaticSnapshot,
    action_kind: Option<ActionKind>,
) -> CreatureAnimationState {
    match sleep_phase {
        SleepPhase::EnteringSleep | SleepPhase::Consolidating | SleepPhase::ForcedRecoverySleep => {
            return CreatureAnimationState::Sleeping;
        }
        SleepPhase::Waking => return CreatureAnimationState::Resting,
        SleepPhase::Awake => {}
    }

    if homeostasis.drives.pain >= 0.55 {
        return CreatureAnimationState::Hurt;
    }
    if homeostasis.drives.fear >= 0.65 {
        return CreatureAnimationState::Afraid;
    }

    match action_kind {
        Some(ActionKind::Move) => CreatureAnimationState::Moving,
        Some(ActionKind::Inspect) => CreatureAnimationState::Inspecting,
        Some(ActionKind::Interact) | Some(ActionKind::Hold) => CreatureAnimationState::Interacting,
        Some(ActionKind::Rest) => CreatureAnimationState::Resting,
        Some(ActionKind::Vocalize) | Some(ActionKind::Write) | Some(ActionKind::Gesture) => {
            CreatureAnimationState::Signaling
        }
        Some(ActionKind::Idle) | None => {
            if homeostasis.drives.curiosity >= 0.72 {
                CreatureAnimationState::Curious
            } else {
                CreatureAnimationState::Idle
            }
        }
    }
}

fn creature_expression_state(
    sleep_phase: SleepPhase,
    homeostasis: &HomeostaticSnapshot,
) -> CreatureExpressionState {
    if sleep_phase != SleepPhase::Awake {
        return CreatureExpressionState::Tired;
    }
    if homeostasis.drives.pain >= 0.45 {
        CreatureExpressionState::Pained
    } else if homeostasis.drives.fear >= 0.55 {
        CreatureExpressionState::Afraid
    } else if homeostasis.drives.hunger >= 0.70 {
        CreatureExpressionState::Hungry
    } else if homeostasis.drives.fatigue >= 0.65 || homeostasis.hormones.sleep_pressure >= 0.65 {
        CreatureExpressionState::Tired
    } else if homeostasis.drives.curiosity >= 0.70 {
        CreatureExpressionState::Curious
    } else if homeostasis.drives.brain_atp >= 0.80 {
        CreatureExpressionState::Energized
    } else {
        CreatureExpressionState::Neutral
    }
}

fn creature_base_rgba(
    homeostasis: &HomeostaticSnapshot,
) -> Result<[f32; 4], ScaffoldContractError> {
    Ok([
        bounded01(0.22 + homeostasis.drives.brain_atp * 0.20 - homeostasis.drives.pain * 0.08)?,
        bounded01(0.40 + homeostasis.drives.curiosity * 0.18 - homeostasis.drives.fear * 0.10)?,
        bounded01(
            0.64 + homeostasis.hormones.serotonin * 0.14 - homeostasis.drives.fatigue * 0.12,
        )?,
        1.0,
    ])
}

const fn creature_expression_rgba(expression: CreatureExpressionState) -> [f32; 4] {
    match expression {
        CreatureExpressionState::Neutral => [0.74, 0.78, 0.82, 1.0],
        CreatureExpressionState::Hungry => [0.20, 0.86, 0.34, 1.0],
        CreatureExpressionState::Tired => [0.50, 0.46, 0.86, 1.0],
        CreatureExpressionState::Afraid => [0.96, 0.66, 0.20, 1.0],
        CreatureExpressionState::Pained => [0.96, 0.18, 0.20, 1.0],
        CreatureExpressionState::Curious => [0.96, 0.86, 0.18, 1.0],
        CreatureExpressionState::Energized => [0.18, 0.68, 0.96, 1.0],
    }
}

const fn action_intent_rgba(action_kind: Option<ActionKind>) -> [f32; 4] {
    match action_kind {
        Some(ActionKind::Move) => [0.40, 0.74, 0.96, 1.0],
        Some(ActionKind::Interact) | Some(ActionKind::Hold) => [0.20, 0.88, 0.38, 1.0],
        Some(ActionKind::Inspect) => [0.96, 0.84, 0.28, 1.0],
        Some(ActionKind::Rest) => [0.50, 0.46, 0.86, 1.0],
        Some(ActionKind::Vocalize) | Some(ActionKind::Write) | Some(ActionKind::Gesture) => {
            [0.76, 0.58, 0.96, 1.0]
        }
        Some(ActionKind::Idle) | None => [0.62, 0.66, 0.70, 1.0],
    }
}

fn facing_from_target(
    position: Vec3f,
    target_position: Option<Vec3f>,
) -> Result<Vec3f, ScaffoldContractError> {
    let Some(target) = target_position else {
        return Ok(Vec3f::new(1.0, 0.0, 0.0));
    };
    let dx = target.x - position.x;
    let dz = target.z - position.z;
    let length = (dx.mul_add(dx, dz * dz)).sqrt();
    if !length.is_finite() {
        return Err(ScaffoldContractError::NonFiniteFloat);
    }
    if length <= f32::EPSILON {
        Ok(Vec3f::new(1.0, 0.0, 0.0))
    } else {
        Ok(Vec3f::new(dx / length, 0.0, dz / length))
    }
}

fn bounded01(value: f32) -> Result<f32, ScaffoldContractError> {
    NormalizedScalar::new(value.clamp(0.0, 1.0)).map(|bounded| bounded.raw())
}

pub(crate) fn wrap_degrees(value: f32) -> f32 {
    let wrapped = value.rem_euclid(360.0);
    if wrapped == 360.0 {
        0.0
    } else {
        wrapped
    }
}

fn validate_rgba(rgba: [f32; 4]) -> Result<(), ScaffoldContractError> {
    for channel in rgba {
        NormalizedScalar::new(channel)?;
    }
    Ok(())
}

pub fn run_creature_visual_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<CreatureVisualSnapshot, GameAppShellError> {
    let presentation = load_visible_world_from_p34_save(launch)?;
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut summaries = live.update(LiveBrainTickControl::step_once())?;
    let summary = summaries
        .pop()
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "step once must produce one live brain tick for G04 visuals",
        })?;
    live.creature_visual_snapshot(&presentation, Some(&summary))
}
