//! Split from the original playable-sim app shell during R13 remediation.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraNavigationState {
    pub schema: &'static str,
    pub schema_version: u16,
    pub focus: Vec3f,
    pub zoom: f32,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
    pub follow_target: Option<WorldEntityId>,
}

impl CameraNavigationState {
    pub fn top_down_default() -> Self {
        Self {
            schema: G05_CAMERA_INSPECTOR_SCHEMA,
            schema_version: G05_CAMERA_INSPECTOR_SCHEMA_VERSION,
            focus: Vec3f::ZERO,
            zoom: 1.0,
            yaw_degrees: 0.0,
            pitch_degrees: 60.0,
            follow_target: None,
        }
    }

    pub fn with_follow_target(
        mut self,
        target: WorldEntityId,
    ) -> Result<Self, ScaffoldContractError> {
        target.validate()?;
        self.follow_target = Some(target);
        self.validate()?;
        Ok(self)
    }

    pub fn pan_by(mut self, dx: f32, dz: f32) -> Result<Self, ScaffoldContractError> {
        if !dx.is_finite() || !dz.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        self.focus.x = (self.focus.x + dx).clamp(-512.0, 512.0);
        self.focus.z = (self.focus.z + dz).clamp(-512.0, 512.0);
        self.validate()?;
        Ok(self)
    }

    pub fn zoom_by(mut self, delta: f32) -> Result<Self, ScaffoldContractError> {
        if !delta.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        self.zoom = (self.zoom + delta).clamp(0.25, 8.0);
        self.validate()?;
        Ok(self)
    }

    pub fn orbit_by(mut self, yaw_delta: f32) -> Result<Self, ScaffoldContractError> {
        if !yaw_delta.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        self.yaw_degrees = wrap_degrees(self.yaw_degrees + yaw_delta);
        self.validate()?;
        Ok(self)
    }

    pub fn focus_on(mut self, position: Vec3f) -> Result<Self, ScaffoldContractError> {
        position.validate()?;
        self.focus = position;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.focus.validate()?;
        if !(0.25..=8.0).contains(&self.zoom) || !self.zoom.is_finite() {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if !self.yaw_degrees.is_finite() || !self.pitch_degrees.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if !(15.0..=85.0).contains(&self.pitch_degrees) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if let Some(target) = self.follow_target {
            target.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{:.2}:{:.2}:{:.2}:{:.2}:{:?}",
            self.schema_version,
            self.schema,
            self.focus.x,
            self.focus.z,
            self.zoom,
            self.yaw_degrees,
            self.follow_target.map(|id| id.raw())
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorRunMode {
    Paused,
    StepOnce,
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectorControlPanel {
    pub schema: &'static str,
    pub schema_version: u16,
    pub mode: InspectorRunMode,
    pub fixed_ticks: u32,
    pub speed_percent: u16,
}

impl InspectorControlPanel {
    pub const fn paused() -> Self {
        Self {
            schema: G05_CAMERA_INSPECTOR_SCHEMA,
            schema_version: G05_CAMERA_INSPECTOR_SCHEMA_VERSION,
            mode: InspectorRunMode::Paused,
            fixed_ticks: 0,
            speed_percent: 0,
        }
    }

    pub const fn step_once() -> Self {
        Self {
            schema: G05_CAMERA_INSPECTOR_SCHEMA,
            schema_version: G05_CAMERA_INSPECTOR_SCHEMA_VERSION,
            mode: InspectorRunMode::StepOnce,
            fixed_ticks: 1,
            speed_percent: 100,
        }
    }

    pub const fn run_fixed(fixed_ticks: u32, speed_percent: u16) -> Self {
        Self {
            schema: G05_CAMERA_INSPECTOR_SCHEMA,
            schema_version: G05_CAMERA_INSPECTOR_SCHEMA_VERSION,
            mode: InspectorRunMode::Run,
            fixed_ticks,
            speed_percent,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.fixed_ticks > 16 || self.speed_percent > 400 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if matches!(self.mode, InspectorRunMode::Paused) && self.fixed_ticks != 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn to_live_control(self) -> Result<LiveBrainTickControl, ScaffoldContractError> {
        self.validate()?;
        Ok(match self.mode {
            InspectorRunMode::Paused => LiveBrainTickControl::paused(),
            InspectorRunMode::StepOnce => LiveBrainTickControl::step_once(),
            InspectorRunMode::Run => LiveBrainTickControl::run_fixed(self.fixed_ticks),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntitySelectionSnapshot {
    pub schema: &'static str,
    pub schema_version: u16,
    pub stable_id: WorldEntityId,
    pub label: String,
    pub kind: WorldObjectKind,
    pub organism_id: Option<OrganismId>,
    pub position: Vec3f,
    pub debug_label: String,
}

impl EntitySelectionSnapshot {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.stable_id.validate()?;
        self.position.validate()?;
        if let Some(organism_id) = self.organism_id {
            organism_id.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{:?}:{}:{:.2}:{:.2}:{:.2}",
            self.stable_id.raw(),
            self.label,
            self.kind,
            self.organism_id.map(|id| id.raw()).unwrap_or_default(),
            self.position.x,
            self.position.y,
            self.position.z
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureInspectorSnapshot {
    pub schema: &'static str,
    pub schema_version: u16,
    pub read_only: bool,
    pub selection: EntitySelectionSnapshot,
    pub camera: CameraNavigationState,
    pub visual: CreatureVisualSnapshot,
    pub tick_summary: Option<LiveBrainTickSummary>,
    pub drive_lines: Vec<String>,
    pub hormone_lines: Vec<String>,
    pub memory_topology_summary: String,
    pub action_summary: String,
    pub patch_summary: String,
    pub semantic_context_summary: String,
    pub fallback_summary: String,
    pub troubleshooting_messages: Vec<String>,
}

impl CreatureInspectorSnapshot {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if !self.read_only {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.selection.validate()?;
        self.camera.validate()?;
        self.visual.validate()?;
        if self.selection.stable_id != self.visual.stable_id {
            return Err(ScaffoldContractError::InvalidId);
        }
        if let Some(organism_id) = self.selection.organism_id {
            if organism_id != self.visual.organism_id {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.selection.signature_line(),
            self.visual.animation.label(),
            self.visual.expression.label(),
            self.action_summary,
            self.patch_summary,
            self.semantic_context_summary,
            self.memory_topology_summary
        )
    }
}

pub fn select_visible_world_entity(
    presentation: &VisibleWorldPresentation,
    stable_id: WorldEntityId,
) -> Result<EntitySelectionSnapshot, GameAppShellError> {
    stable_id.validate()?;
    let object = presentation
        .objects
        .iter()
        .find(|object| object.stable_id == stable_id)
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "selected stable ID must exist in visible world presentation",
        })?;
    let selection = EntitySelectionSnapshot {
        schema: G05_CAMERA_INSPECTOR_SCHEMA,
        schema_version: G05_CAMERA_INSPECTOR_SCHEMA_VERSION,
        stable_id: object.stable_id,
        label: object.label.clone(),
        kind: object.kind,
        organism_id: object.organism_id,
        position: object.position,
        debug_label: object.debug_label.clone(),
    };
    selection.validate()?;
    Ok(selection)
}

pub fn creature_inspector_snapshot(
    presentation: &VisibleWorldPresentation,
    organism_id: OrganismId,
    mind: &CreatureMind,
    last_tick: Option<&LiveBrainTickSummary>,
    camera: CameraNavigationState,
) -> Result<CreatureInspectorSnapshot, GameAppShellError> {
    let visual =
        creature_visual_snapshot_from_presentation(presentation, organism_id, mind, last_tick)?;
    let selection = select_visible_world_entity(presentation, visual.stable_id)?;
    let camera = camera
        .focus_on(selection.position)?
        .with_follow_target(selection.stable_id)?;
    let homeostasis = mind.homeostasis();
    let drive_lines = vec![
        format!("hunger={:.2}", homeostasis.drives.hunger),
        format!("fatigue={:.2}", homeostasis.drives.fatigue),
        format!("fear={:.2}", homeostasis.drives.fear),
        format!("pain={:.2}", homeostasis.drives.pain),
        format!("curiosity={:.2}", homeostasis.drives.curiosity),
        format!("brain_atp={:.2}", homeostasis.drives.brain_atp),
    ];
    let hormone_lines = vec![
        format!("adrenaline={:.2}", homeostasis.hormones.adrenaline),
        format!("cortisol={:.2}", homeostasis.hormones.cortisol),
        format!("dopamine={:.2}", homeostasis.hormones.dopamine),
        format!("serotonin={:.2}", homeostasis.hormones.serotonin),
        format!("sleep_pressure={:.2}", homeostasis.hormones.sleep_pressure),
    ];
    let memory_topology_summary = match last_tick {
        Some(summary) => format!(
            "memory_updates={} topology_updates={} learning_updates={}",
            summary.memory_updates, summary.topology_updates, summary.learning_updates
        ),
        None => "memory_updates=0 topology_updates=0 learning_updates=0".to_string(),
    };
    let action_summary = match last_tick {
        Some(summary) => format!(
            "action={:?} id={:?} target={:?} status={:?}",
            summary.selected_action_kind,
            summary.selected_action_id.map(|id| id.raw()),
            summary.target_entity.map(|id| id.raw()),
            summary.status
        ),
        None => "action=None id=None target=None status=NotTicked".to_string(),
    };
    let patch_summary = match last_tick {
        Some(summary) => format!(
            "sealed={} sequence={:?} success={:?} contact={:?} packed_logs={}",
            summary.patch_sealed,
            summary.patch_sequence_id,
            summary.patch_success,
            summary.physical_contact,
            summary.packed_record_count
        ),
        None => "sealed=false sequence=None success=None contact=None packed_logs=0".to_string(),
    };
    let mut troubleshooting_messages = vec![
        "backend=CpuReference fallback=not-required-for-headless-smoke".to_string(),
        "semantic_provider=optional missing_provider=nonfatal".to_string(),
        "gpu_runtime=optional no-active-neural-readback".to_string(),
    ];
    if let Some(summary) = last_tick {
        if summary.action_failure.is_some() {
            troubleshooting_messages.push(format!(
                "recoverable_action_failure={:?}",
                summary.action_failure
            ));
        }
    }
    let snapshot = CreatureInspectorSnapshot {
        schema: G05_CAMERA_INSPECTOR_SCHEMA,
        schema_version: G05_CAMERA_INSPECTOR_SCHEMA_VERSION,
        read_only: true,
        selection,
        camera,
        visual,
        tick_summary: last_tick.cloned(),
        drive_lines,
        hormone_lines,
        memory_topology_summary,
        action_summary,
        patch_summary,
        semantic_context_summary:
            "semantic_provider=disabled context=none bounded=true nonfatal=true".to_string(),
        fallback_summary: "CPU oracle active; GPU/semantic providers optional".to_string(),
        troubleshooting_messages,
    };
    snapshot.validate()?;
    Ok(snapshot)
}

pub fn run_creature_inspector_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<CreatureInspectorSnapshot, GameAppShellError> {
    let presentation = load_visible_world_from_p34_save(launch)?;
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut summaries = live.update(LiveBrainTickControl::step_once())?;
    let summary = summaries
        .pop()
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "step once must produce one live brain tick for G05 inspector",
        })?;
    creature_inspector_snapshot(
        &presentation,
        live.organism_id(),
        live.mind(),
        Some(&summary),
        CameraNavigationState::top_down_default(),
    )
}
