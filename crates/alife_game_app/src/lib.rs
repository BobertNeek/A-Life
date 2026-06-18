//! G01 playable-sim app shell.
//!
//! This crate owns product app startup policy. The default path remains
//! headless and CI-safe; Bevy construction is behind the `bevy-app` feature.

use std::path::{Path, PathBuf};

use alife_core::{
    ActionId, ActionKind, ActionProposal, ActionTarget, BrainTickInput, BrainTickStatus,
    Confidence, CreatureMind, DurationTicks, HomeostaticSnapshot, Intensity, NormalizedScalar,
    OrganismId, PhysicalContactKind, ReferenceActionFailure, ScaffoldContractError, SleepPhase,
    Tick, Validate, Vec3f, WorldEntityId,
};
use alife_world::persistence::{
    AssetManifest, BackendSelection, PersistenceError, PortableSaveFile, RuntimeConfig,
    WorldObjectSaveState,
};
use alife_world::{HeadlessActionIds, HeadlessBrainHarness, HeadlessWorld, WorldObjectKind};
use thiserror::Error;

pub const G01_APP_SHELL_SCHEMA: &str = "alife.g01.app_shell.v1";
pub const G01_APP_SHELL_SCHEMA_VERSION: u16 = 1;
pub const G02_VISIBLE_WORLD_SCHEMA: &str = "alife.g02.visible_world.v1";
pub const G02_VISIBLE_WORLD_SCHEMA_VERSION: u16 = 1;
pub const G03_LIVE_BRAIN_LOOP_SCHEMA: &str = "alife.g03.live_brain_loop.v1";
pub const G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION: u16 = 1;
pub const G04_CREATURE_VISUAL_SCHEMA: &str = "alife.g04.creature_visual_state.v1";
pub const G04_CREATURE_VISUAL_SCHEMA_VERSION: u16 = 1;
pub const G05_CAMERA_INSPECTOR_SCHEMA: &str = "alife.g05.camera_selection_inspector.v1";
pub const G05_CAMERA_INSPECTOR_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Error)]
pub enum GameAppShellError {
    #[error("persistence/config error: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("core contract error: {0}")]
    Core(#[from] alife_core::ScaffoldContractError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid app shell state transition: {from:?} -> {to:?}")]
    InvalidTransition {
        from: GameAppState,
        to: GameAppState,
    },
    #[error("visible world mismatch: {message}")]
    VisibleWorldMismatch { message: &'static str },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameAppState {
    Boot,
    LoadConfig,
    DevMenu,
    Running,
    Paused,
    Shutdown,
}

impl GameAppState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Boot => "Boot",
            Self::LoadConfig => "LoadConfig",
            Self::DevMenu => "DevMenu",
            Self::Running => "Running",
            Self::Paused => "Paused",
            Self::Shutdown => "Shutdown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppShellStateTrace {
    states: Vec<GameAppState>,
}

impl Default for AppShellStateTrace {
    fn default() -> Self {
        Self {
            states: vec![GameAppState::Boot],
        }
    }
}

impl AppShellStateTrace {
    pub fn states(&self) -> &[GameAppState] {
        &self.states
    }

    pub fn labels(&self) -> Vec<&'static str> {
        self.states.iter().map(|state| state.label()).collect()
    }

    pub fn current(&self) -> GameAppState {
        *self
            .states
            .last()
            .expect("state trace always starts at Boot")
    }

    pub fn transition(&mut self, to: GameAppState) -> Result<(), GameAppShellError> {
        let from = self.current();
        if !valid_transition(from, to) {
            return Err(GameAppShellError::InvalidTransition { from, to });
        }
        self.states.push(to);
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppShellLaunchConfig {
    pub fixture_root: PathBuf,
    pub config_path: PathBuf,
    pub asset_manifest_path: PathBuf,
    pub save_path: PathBuf,
    pub asset_root: PathBuf,
    pub start_paused: bool,
}

impl AppShellLaunchConfig {
    pub fn from_p34_fixture_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        Self {
            config_path: root.join("tiny_config.json"),
            asset_manifest_path: root.join("tiny_asset_manifest.json"),
            save_path: root.join("tiny_save.json"),
            asset_root: root.clone(),
            fixture_root: root,
            start_paused: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisiblePlaceholderShape {
    GroundPlane,
    CreatureCapsule,
    FoodSphere,
    HazardCone,
    ObstacleCube,
    TokenBillboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleMaterialKind {
    Ground,
    Creature,
    Food,
    Hazard,
    Obstacle,
    Token,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub cues: CreatureVisualCueSet,
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
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.selection.signature_line(),
            self.visual.animation.label(),
            self.visual.expression.label(),
            self.action_summary,
            self.patch_summary,
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
        fallback_summary: "CPU oracle active; GPU/semantic providers optional".to_string(),
        troubleshooting_messages,
    };
    snapshot.validate()?;
    Ok(snapshot)
}

impl VisibleMaterialKind {
    pub const fn rgba(self) -> [f32; 4] {
        match self {
            Self::Ground => [0.18, 0.23, 0.18, 1.0],
            Self::Creature => [0.30, 0.55, 0.95, 1.0],
            Self::Food => [0.24, 0.78, 0.34, 1.0],
            Self::Hazard => [0.90, 0.20, 0.18, 1.0],
            Self::Obstacle => [0.42, 0.38, 0.33, 1.0],
            Self::Token => [0.72, 0.62, 0.95, 1.0],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibleWorldObjectPresentation {
    pub stable_id: alife_core::WorldEntityId,
    pub label: String,
    pub kind: WorldObjectKind,
    pub organism_id: Option<alife_core::OrganismId>,
    pub position: alife_core::Vec3f,
    pub radius: f32,
    pub nutrition: f32,
    pub hazard_pain: f32,
    pub token_id: Option<u32>,
    pub shape: VisiblePlaceholderShape,
    pub material: VisibleMaterialKind,
    pub debug_label: String,
}

impl VisibleWorldObjectPresentation {
    pub fn from_save_object(object: &WorldObjectSaveState) -> Self {
        let (shape, material) = placeholder_for_kind(object.kind);
        Self {
            stable_id: object.id,
            label: object.label.clone(),
            kind: object.kind,
            organism_id: object.organism_id,
            position: object.position,
            radius: object.radius,
            nutrition: object.nutrition,
            hazard_pain: object.hazard_pain,
            token_id: object.token_id,
            shape,
            material,
            debug_label: format!("{:04}:{:?}:{}", object.id.raw(), object.kind, object.label),
        }
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{:?}:{}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{:?}:{:?}:{:?}",
            self.stable_id.raw(),
            self.kind,
            self.label,
            self.position.x,
            self.position.y,
            self.position.z,
            self.radius,
            self.nutrition,
            self.hazard_pain,
            self.token_id,
            self.shape,
            self.material
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisibleWorldPresentation {
    pub schema: &'static str,
    pub schema_version: u16,
    pub save_id: String,
    pub seed: u64,
    pub object_count: usize,
    pub ground_shape: VisiblePlaceholderShape,
    pub ground_material: VisibleMaterialKind,
    pub objects: Vec<VisibleWorldObjectPresentation>,
    pub headless_signature: Vec<String>,
    pub visible_signature: Vec<String>,
}

impl VisibleWorldPresentation {
    pub fn stable_ids(&self) -> Vec<alife_core::WorldEntityId> {
        self.objects.iter().map(|object| object.stable_id).collect()
    }

    pub fn kind_count(&self, kind: WorldObjectKind) -> usize {
        self.objects
            .iter()
            .filter(|object| object.kind == kind)
            .count()
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
    homeostasis.validate_contract()?;
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
        cues,
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

fn wrap_degrees(value: f32) -> f32 {
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

pub fn load_visible_world_from_p34_save(
    launch: &AppShellLaunchConfig,
) -> Result<VisibleWorldPresentation, GameAppShellError> {
    let config = RuntimeConfig::from_json_file(&launch.config_path)?;
    config.validate()?;
    let manifest = AssetManifest::from_json_file(&launch.asset_manifest_path)?;
    manifest.validate_with_root(&launch.asset_root)?;

    let save = PortableSaveFile::from_json_file(&launch.save_path)?;
    save.validate_with_asset_root(&launch.asset_root)?;
    if save.deterministic_seed != config.deterministic_seed {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "runtime config seed must match portable save seed",
        });
    }

    visible_world_from_save(&save)
}

pub fn visible_world_from_save(
    save: &PortableSaveFile,
) -> Result<VisibleWorldPresentation, GameAppShellError> {
    let restored = save.restore_headless_world()?;
    let headless_signature = restored.stable_signature();
    let mut objects = save
        .world
        .objects
        .iter()
        .map(VisibleWorldObjectPresentation::from_save_object)
        .collect::<Vec<_>>();
    objects.sort_by_key(|object| object.stable_id.raw());
    let visible_signature = objects
        .iter()
        .map(VisibleWorldObjectPresentation::signature_line)
        .collect::<Vec<_>>();
    if objects.len() != headless_signature.len() {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "visible object count must match restored headless world",
        });
    }
    Ok(VisibleWorldPresentation {
        schema: G02_VISIBLE_WORLD_SCHEMA,
        schema_version: G02_VISIBLE_WORLD_SCHEMA_VERSION,
        save_id: save.save_id.clone(),
        seed: save.deterministic_seed,
        object_count: objects.len(),
        ground_shape: VisiblePlaceholderShape::GroundPlane,
        ground_material: VisibleMaterialKind::Ground,
        objects,
        headless_signature,
        visible_signature,
    })
}

pub fn compare_visible_world_to_headless(
    presentation: &VisibleWorldPresentation,
) -> Result<(), GameAppShellError> {
    if presentation.object_count != presentation.objects.len()
        || presentation.object_count != presentation.headless_signature.len()
    {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "presentation, visible signature, and headless signature counts must match",
        });
    }
    let mut stable_ids = presentation.stable_ids();
    stable_ids.sort_by_key(|id| id.raw());
    stable_ids.dedup();
    if stable_ids.len() != presentation.objects.len() {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "visible objects must have unique stable IDs",
        });
    }
    Ok(())
}

pub const fn placeholder_for_kind(
    kind: WorldObjectKind,
) -> (VisiblePlaceholderShape, VisibleMaterialKind) {
    match kind {
        WorldObjectKind::Agent => (
            VisiblePlaceholderShape::CreatureCapsule,
            VisibleMaterialKind::Creature,
        ),
        WorldObjectKind::Food => (
            VisiblePlaceholderShape::FoodSphere,
            VisibleMaterialKind::Food,
        ),
        WorldObjectKind::Hazard => (
            VisiblePlaceholderShape::HazardCone,
            VisibleMaterialKind::Hazard,
        ),
        WorldObjectKind::Obstacle => (
            VisiblePlaceholderShape::ObstacleCube,
            VisibleMaterialKind::Obstacle,
        ),
        WorldObjectKind::Token => (
            VisiblePlaceholderShape::TokenBillboard,
            VisibleMaterialKind::Token,
        ),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppStartupSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub brain_class: String,
    pub requested_backend: BackendSelection,
    pub gpu_feature_enabled: bool,
    pub gpu_backend_enabled: bool,
    pub semantic_enabled: bool,
    pub school_enabled: bool,
    pub logging_enabled: bool,
    pub asset_count: usize,
    pub state_trace: Vec<GameAppState>,
    pub bevy_feature_compiled: bool,
    pub graphics_required_for_default_path: bool,
}

impl AppStartupSummary {
    pub fn state_labels(&self) -> Vec<&'static str> {
        self.state_trace.iter().map(|state| state.label()).collect()
    }
}

pub fn run_headless_app_shell_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<AppStartupSummary, GameAppShellError> {
    let config = RuntimeConfig::from_json_file(&launch.config_path)?;
    config.validate()?;
    let manifest = AssetManifest::from_json_file(&launch.asset_manifest_path)?;
    manifest.validate_with_root(&launch.asset_root)?;

    let mut trace = AppShellStateTrace::default();
    trace.transition(GameAppState::LoadConfig)?;
    trace.transition(GameAppState::DevMenu)?;
    trace.transition(GameAppState::Running)?;
    if launch.start_paused {
        trace.transition(GameAppState::Paused)?;
        trace.transition(GameAppState::Running)?;
    }
    trace.transition(GameAppState::Shutdown)?;

    Ok(AppStartupSummary {
        schema: G01_APP_SHELL_SCHEMA,
        schema_version: G01_APP_SHELL_SCHEMA_VERSION,
        seed: config.deterministic_seed,
        brain_class: format!("{:?}", config.brain_class),
        requested_backend: config.backend.requested,
        gpu_feature_enabled: config.backend.gpu_feature_enabled,
        gpu_backend_enabled: config.features.gpu_backend_enabled,
        semantic_enabled: config.features.semantic_adapter_enabled,
        school_enabled: config.features.school_enabled,
        logging_enabled: config.logging.enabled,
        asset_count: manifest.entries.len(),
        state_trace: trace.states().to_vec(),
        bevy_feature_compiled: cfg!(feature = "bevy-app"),
        graphics_required_for_default_path: false,
    })
}

pub fn validate_app_shell_config(
    launch: &AppShellLaunchConfig,
) -> Result<AppStartupSummary, GameAppShellError> {
    run_headless_app_shell_smoke(launch)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveBrainRunMode {
    Paused,
    StepOnce,
    RunFixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveBrainTickControl {
    pub mode: LiveBrainRunMode,
    pub fixed_ticks: u32,
}

impl LiveBrainTickControl {
    pub const fn paused() -> Self {
        Self {
            mode: LiveBrainRunMode::Paused,
            fixed_ticks: 0,
        }
    }

    pub const fn step_once() -> Self {
        Self {
            mode: LiveBrainRunMode::StepOnce,
            fixed_ticks: 1,
        }
    }

    pub const fn run_fixed(fixed_ticks: u32) -> Self {
        Self {
            mode: LiveBrainRunMode::RunFixed,
            fixed_ticks,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveBrainCausalStage {
    GatherSensory,
    CpuBrainTick,
    ExecuteAction,
    MeasureOutcome,
    SealPatch,
    UpdateLogs,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveBrainTickSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub organism_id: OrganismId,
    pub tick_before: Tick,
    pub tick_after: Tick,
    pub world_tick_before: Tick,
    pub world_tick_after: Tick,
    pub status: BrainTickStatus,
    pub selected_action_kind: Option<ActionKind>,
    pub selected_action_id: Option<ActionId>,
    pub target_entity: Option<WorldEntityId>,
    pub patch_sealed: bool,
    pub patch_sequence_id: Option<u64>,
    pub patch_success: Option<bool>,
    pub physical_contact: Option<PhysicalContactKind>,
    pub action_failure: Option<ReferenceActionFailure>,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub memory_updates: u32,
    pub topology_updates: u32,
    pub learning_updates: u32,
    pub causal_stages: Vec<LiveBrainCausalStage>,
}

#[derive(Debug)]
pub struct LiveBrainLoop {
    organism_id: OrganismId,
    logging_enabled: bool,
    harness: HeadlessBrainHarness,
    mind: CreatureMind,
}

impl LiveBrainLoop {
    pub fn from_p34_launch(launch: &AppShellLaunchConfig) -> Result<Self, GameAppShellError> {
        let config = RuntimeConfig::from_json_file(&launch.config_path)?;
        config.validate()?;
        let manifest = AssetManifest::from_json_file(&launch.asset_manifest_path)?;
        manifest.validate_with_root(&launch.asset_root)?;
        let save = PortableSaveFile::from_json_file(&launch.save_path)?;
        save.validate_with_asset_root(&launch.asset_root)?;
        if save.deterministic_seed != config.deterministic_seed {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "runtime config seed must match portable save seed",
            });
        }
        let creature = save
            .creatures
            .first()
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "portable save must include at least one creature for G03",
            })?;
        let world = save.restore_headless_world()?;
        let mut mind = CreatureMind::scaffold(
            creature.organism_id,
            creature.brain_class,
            save.deterministic_seed,
            creature.mind.tick,
        )?;
        *mind.homeostasis_mut() = creature.mind.homeostasis;
        mind.homeostasis().validate_contract()?;
        Ok(Self::new(
            world,
            mind,
            creature.organism_id,
            config.logging.enabled,
        ))
    }

    pub fn new(
        world: HeadlessWorld,
        mind: CreatureMind,
        organism_id: OrganismId,
        logging_enabled: bool,
    ) -> Self {
        Self {
            organism_id,
            logging_enabled,
            harness: HeadlessBrainHarness::new(world),
            mind,
        }
    }

    pub const fn organism_id(&self) -> OrganismId {
        self.organism_id
    }

    pub const fn mind(&self) -> &CreatureMind {
        &self.mind
    }

    pub fn creature_visual_snapshot(
        &self,
        presentation: &VisibleWorldPresentation,
        last_tick: Option<&LiveBrainTickSummary>,
    ) -> Result<CreatureVisualSnapshot, GameAppShellError> {
        creature_visual_snapshot_from_presentation(
            presentation,
            self.organism_id,
            &self.mind,
            last_tick,
        )
    }

    pub fn world_signature(&self) -> Vec<String> {
        self.harness.world().stable_signature()
    }

    pub fn telemetry_counts(&self) -> (usize, usize) {
        (
            self.harness.telemetry().sealed_patches.len(),
            self.harness.telemetry().packed_records.len(),
        )
    }

    pub fn update(
        &mut self,
        control: LiveBrainTickControl,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        let ticks = match control.mode {
            LiveBrainRunMode::Paused => 0,
            LiveBrainRunMode::StepOnce => 1,
            LiveBrainRunMode::RunFixed => control.fixed_ticks.min(16),
        };
        let mut summaries = Vec::with_capacity(ticks as usize);
        for _ in 0..ticks {
            let proposals = self.proposals_from_current_sensory()?;
            summaries.push(self.tick_with_proposals(proposals));
        }
        Ok(summaries)
    }

    pub fn current_context_proposals(&self) -> Result<Vec<ActionProposal>, GameAppShellError> {
        self.proposals_from_current_sensory()
    }

    pub fn tick_with_proposals(&mut self, proposals: Vec<ActionProposal>) -> LiveBrainTickSummary {
        let tick_before = self.mind.current_tick();
        let world_tick_before = self.harness.world().tick();
        let input = BrainTickInput::new(tick_before, proposals)
            .with_pack_experience(self.logging_enabled)
            .with_action_duration(DurationTicks::new(1));
        let tick = self.harness.tick_mind(&mut self.mind, input);
        let world_tick_after = self.harness.world().tick();
        let action_failure = tick
            .action_result
            .as_ref()
            .and_then(|result| result.execution.failure);
        Self::summarize_tick(
            self.organism_id,
            tick_before,
            self.mind.current_tick(),
            world_tick_before,
            world_tick_after,
            &tick.brain,
            action_failure,
            self.harness.telemetry().sealed_patches.len(),
            self.harness.telemetry().packed_records.len(),
        )
    }

    fn proposals_from_current_sensory(&self) -> Result<Vec<ActionProposal>, GameAppShellError> {
        let report = self
            .harness
            .world()
            .sensory_report(self.organism_id, self.mind.current_tick())?;
        let mut proposals = Vec::new();
        for visible in report.visible_entities {
            match visible.kind {
                WorldObjectKind::Food => proposals.push(proposal(
                    HeadlessActionIds::EAT,
                    ActionKind::Interact,
                    Some(visible.id),
                    None,
                    0.72,
                    0.95,
                    visible.distance,
                )?),
                WorldObjectKind::Hazard => proposals.push(proposal(
                    HeadlessActionIds::FLEE,
                    ActionKind::Move,
                    Some(visible.id),
                    None,
                    0.66,
                    0.9,
                    visible.distance,
                )?),
                WorldObjectKind::Obstacle => proposals.push(proposal(
                    ActionKind::Inspect.canonical_id(),
                    ActionKind::Inspect,
                    Some(visible.id),
                    None,
                    0.38,
                    0.7,
                    visible.distance,
                )?),
                WorldObjectKind::Agent | WorldObjectKind::Token => proposals.push(proposal(
                    ActionKind::Inspect.canonical_id(),
                    ActionKind::Inspect,
                    Some(visible.id),
                    None,
                    0.42,
                    0.75,
                    visible.distance,
                )?),
            }
        }
        proposals.push(proposal(
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            None,
            None,
            0.28,
            0.55,
            0.0,
        )?);
        Ok(proposals)
    }

    #[allow(clippy::too_many_arguments)]
    fn summarize_tick(
        organism_id: OrganismId,
        tick_before: Tick,
        tick_after: Tick,
        world_tick_before: Tick,
        world_tick_after: Tick,
        brain: &alife_core::BrainTickOutput,
        action_failure: Option<ReferenceActionFailure>,
        sealed_patch_count: usize,
        packed_record_count: usize,
    ) -> LiveBrainTickSummary {
        let patch = brain.experience_patch.as_ref();
        let selected = brain.selected_action;
        LiveBrainTickSummary {
            schema: G03_LIVE_BRAIN_LOOP_SCHEMA,
            schema_version: G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
            organism_id,
            tick_before,
            tick_after,
            world_tick_before,
            world_tick_after,
            status: brain.status,
            selected_action_kind: selected.map(|command| command.kind),
            selected_action_id: selected.map(|command| command.action_id),
            target_entity: selected.and_then(|command| command.target_entity),
            patch_sealed: patch.is_some(),
            patch_sequence_id: patch.map(|patch| patch.pre_action().sequence_id.raw()),
            patch_success: patch.map(|patch| patch.outcome().success),
            physical_contact: patch.map(|patch| patch.outcome().physical.contact),
            action_failure,
            sealed_patch_count,
            packed_record_count,
            memory_updates: brain.diagnostics.memory_updates,
            topology_updates: brain.diagnostics.topology_updates,
            learning_updates: brain.diagnostics.learning_updates,
            causal_stages: vec![
                LiveBrainCausalStage::GatherSensory,
                LiveBrainCausalStage::CpuBrainTick,
                LiveBrainCausalStage::ExecuteAction,
                LiveBrainCausalStage::MeasureOutcome,
                LiveBrainCausalStage::SealPatch,
                LiveBrainCausalStage::UpdateLogs,
            ],
        }
    }
}

fn proposal(
    action_id: ActionId,
    kind: ActionKind,
    target_entity: Option<WorldEntityId>,
    target_position: Option<Vec3f>,
    score: f32,
    confidence: f32,
    distance: f32,
) -> Result<ActionProposal, ScaffoldContractError> {
    let salience = if distance <= 0.0 {
        0.5
    } else {
        (1.0 / (1.0 + distance)).clamp(0.1, 1.0)
    };
    let mut proposal = ActionProposal::new(
        action_id,
        kind,
        score,
        Confidence::new(confidence)?,
        None,
        0b11,
        ActionTarget::new(target_entity, target_position),
        NormalizedScalar::new(salience)?,
    )?;
    proposal.intensity = Intensity::new(1.0)?;
    Ok(proposal)
}

pub fn run_live_brain_loop_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<LiveBrainTickSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut summaries = live.update(LiveBrainTickControl::step_once())?;
    summaries
        .pop()
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "step once must produce one live brain tick",
        })
}

pub fn run_live_brain_loop_fixed_smoke(
    launch: &AppShellLaunchConfig,
    ticks: u32,
) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    live.update(LiveBrainTickControl::run_fixed(ticks))
}

pub fn run_live_brain_loop_paused_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<(Tick, Tick, usize), GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mind_tick = live.mind.current_tick();
    let world_tick = live.harness.world().tick();
    let summaries = live.update(LiveBrainTickControl::paused())?;
    Ok((mind_tick, world_tick, summaries.len()))
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

#[cfg(feature = "bevy-app")]
pub mod bevy_shell {
    use alife_bevy_adapter::{
        core_vec3_to_bevy, AffordanceTags, AlifeBevyAdapterPlugin, BevyEntityMap, CreatureBody,
        SensoryEmitter,
    };
    use alife_core::{AffordanceBits, WorldEntityId};
    use alife_world::WorldObjectKind;
    use bevy::prelude::{App, Component, Entity, MinimalPlugins, Resource, Transform};

    use crate::{
        load_visible_world_from_p34_save, run_creature_inspector_smoke, run_creature_visual_smoke,
        run_live_brain_loop_smoke, AppShellLaunchConfig, AppStartupSummary, CameraNavigationState,
        CreatureAnimationState, CreatureExpressionState, CreatureInspectorSnapshot,
        CreatureVisualSnapshot, EntitySelectionSnapshot, GameAppShellError, GameAppState,
        LiveBrainTickSummary, VisibleMaterialKind, VisiblePlaceholderShape,
        VisibleWorldPresentation,
    };

    #[derive(Debug, Clone, PartialEq, Resource)]
    pub struct BevyAppShellSummary {
        pub seed: u64,
        pub current_state: GameAppState,
        pub graphics_required_for_default_path: bool,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Component)]
    pub struct VisibleWorldObject {
        pub stable_id: WorldEntityId,
        pub kind: WorldObjectKind,
        pub shape: VisiblePlaceholderShape,
        pub material: VisibleMaterialKind,
        pub rgba: [f32; 4],
    }

    #[derive(Debug, Clone, PartialEq, Component)]
    pub struct VisibleWorldDebugLabel(pub String);

    #[derive(Debug, Clone, Copy, PartialEq, Component)]
    pub struct VisibleGroundPlane {
        pub shape: VisiblePlaceholderShape,
        pub material: VisibleMaterialKind,
        pub rgba: [f32; 4],
    }

    #[derive(Debug, Clone, PartialEq, Resource)]
    pub struct VisibleWorldSceneResource {
        pub schema: &'static str,
        pub schema_version: u16,
        pub seed: u64,
        pub save_id: String,
        pub visible_signature: Vec<String>,
        pub headless_signature: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct VisibleWorldSpawnSummary {
        pub ground_spawned: bool,
        pub object_count: usize,
        pub stable_map_count: usize,
        pub visible_signature: Vec<String>,
    }

    #[derive(Debug, Clone, PartialEq, Resource)]
    pub struct LiveBrainLoopResource {
        pub last_summary: LiveBrainTickSummary,
    }

    #[derive(Debug, Clone, PartialEq, Resource)]
    pub struct CreatureVisualStateResource {
        pub snapshot: CreatureVisualSnapshot,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Resource)]
    pub struct CameraNavigationResource {
        pub state: CameraNavigationState,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Resource)]
    pub struct SelectionResource {
        pub stable_id: WorldEntityId,
        pub local_entity: Option<Entity>,
    }

    #[derive(Debug, Clone, PartialEq, Resource)]
    pub struct CreatureInspectorResource {
        pub snapshot: CreatureInspectorSnapshot,
    }

    #[derive(Debug, Clone, PartialEq, Component)]
    pub struct SelectedVisibleEntity {
        pub selection: EntitySelectionSnapshot,
    }

    #[derive(Debug, Clone, PartialEq, Component)]
    pub struct VisibleCreatureState {
        pub animation: CreatureAnimationState,
        pub expression: CreatureExpressionState,
        pub base_rgba: [f32; 4],
        pub accent_rgba: [f32; 4],
        pub intent_rgba: [f32; 4],
        pub debug_summary: String,
    }

    pub fn build_minimal_bevy_app_shell(summary: AppStartupSummary) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(AlifeBevyAdapterPlugin)
            .insert_resource(BevyAppShellSummary {
                seed: summary.seed,
                current_state: GameAppState::Boot,
                graphics_required_for_default_path: summary.graphics_required_for_default_path,
            });
        app
    }

    pub fn build_visible_world_app_shell(
        launch: &AppShellLaunchConfig,
    ) -> Result<(App, VisibleWorldSpawnSummary), GameAppShellError> {
        let startup = crate::run_headless_app_shell_smoke(launch)?;
        let presentation = load_visible_world_from_p34_save(launch)?;
        let mut app = build_minimal_bevy_app_shell(startup);
        let summary = spawn_visible_world(&mut app, &presentation)?;
        Ok((app, summary))
    }

    pub fn build_live_brain_world_app_shell(
        launch: &AppShellLaunchConfig,
    ) -> Result<(App, VisibleWorldSpawnSummary, LiveBrainTickSummary), GameAppShellError> {
        let (mut app, summary) = build_visible_world_app_shell(launch)?;
        let tick_summary = run_live_brain_loop_smoke(launch)?;
        app.insert_resource(LiveBrainLoopResource {
            last_summary: tick_summary.clone(),
        });
        Ok((app, summary, tick_summary))
    }

    pub fn build_creature_visual_world_app_shell(
        launch: &AppShellLaunchConfig,
    ) -> Result<(App, VisibleWorldSpawnSummary, CreatureVisualSnapshot), GameAppShellError> {
        let (mut app, summary) = build_visible_world_app_shell(launch)?;
        let visual = run_creature_visual_smoke(launch)?;
        if let Some(entity) = app
            .world()
            .resource::<BevyEntityMap>()
            .bevy_entity(visual.stable_id)
        {
            app.world_mut()
                .entity_mut(entity)
                .insert(VisibleCreatureState {
                    animation: visual.animation,
                    expression: visual.expression,
                    base_rgba: visual.base_rgba,
                    accent_rgba: visual.accent_rgba,
                    intent_rgba: visual.intent_rgba,
                    debug_summary: visual.debug_summary.clone(),
                });
        }
        app.insert_resource(CreatureVisualStateResource {
            snapshot: visual.clone(),
        });
        Ok((app, summary, visual))
    }

    pub fn build_creature_inspector_world_app_shell(
        launch: &AppShellLaunchConfig,
    ) -> Result<(App, VisibleWorldSpawnSummary, CreatureInspectorSnapshot), GameAppShellError> {
        let (mut app, summary, _visual) = build_creature_visual_world_app_shell(launch)?;
        let inspector = run_creature_inspector_smoke(launch)?;
        let local_entity = app
            .world()
            .resource::<BevyEntityMap>()
            .bevy_entity(inspector.selection.stable_id);
        if let Some(entity) = local_entity {
            app.world_mut()
                .entity_mut(entity)
                .insert(SelectedVisibleEntity {
                    selection: inspector.selection.clone(),
                });
        }
        app.insert_resource(CameraNavigationResource {
            state: inspector.camera,
        });
        app.insert_resource(SelectionResource {
            stable_id: inspector.selection.stable_id,
            local_entity,
        });
        app.insert_resource(CreatureInspectorResource {
            snapshot: inspector.clone(),
        });
        Ok((app, summary, inspector))
    }

    pub fn spawn_visible_world(
        app: &mut App,
        presentation: &VisibleWorldPresentation,
    ) -> Result<VisibleWorldSpawnSummary, GameAppShellError> {
        crate::compare_visible_world_to_headless(presentation)?;
        let ground_material = presentation.ground_material;
        app.world_mut().spawn((
            Transform::from_xyz(0.0, 0.0, -0.05),
            VisibleGroundPlane {
                shape: presentation.ground_shape,
                material: ground_material,
                rgba: ground_material.rgba(),
            },
            VisibleWorldDebugLabel("ground:debug-plane".to_string()),
        ));

        for object in &presentation.objects {
            let material = object.material;
            let entity = app
                .world_mut()
                .spawn((
                    Transform::from_translation(core_vec3_to_bevy(object.position)?),
                    VisibleWorldObject {
                        stable_id: object.stable_id,
                        kind: object.kind,
                        shape: object.shape,
                        material,
                        rgba: material.rgba(),
                    },
                    VisibleWorldDebugLabel(object.debug_label.clone()),
                ))
                .id();
            {
                let mut entity_mut = app.world_mut().entity_mut(entity);
                match object.kind {
                    WorldObjectKind::Agent => {
                        if let Some(organism_id) = object.organism_id {
                            entity_mut.insert(CreatureBody::new(organism_id, object.stable_id)?);
                        }
                    }
                    WorldObjectKind::Food => {
                        entity_mut.insert(AffordanceTags::food(object.nutrition));
                    }
                    WorldObjectKind::Hazard => {
                        entity_mut.insert(AffordanceTags::hazard(object.hazard_pain));
                    }
                    WorldObjectKind::Obstacle => {
                        entity_mut.insert(AffordanceTags {
                            bits: AffordanceBits::RESOURCE,
                            nutrition: 0.0,
                            hazard_pain: 0.0,
                            blocks_movement: true,
                        });
                    }
                    WorldObjectKind::Token => {
                        entity_mut.insert(SensoryEmitter {
                            audible_token: object.token_id,
                            ..SensoryEmitter::default()
                        });
                    }
                }
            }
            app.world_mut()
                .resource_mut::<BevyEntityMap>()
                .bind(entity, object.stable_id)?;
        }

        app.insert_resource(VisibleWorldSceneResource {
            schema: presentation.schema,
            schema_version: presentation.schema_version,
            seed: presentation.seed,
            save_id: presentation.save_id.clone(),
            visible_signature: presentation.visible_signature.clone(),
            headless_signature: presentation.headless_signature.clone(),
        });
        app.update();

        let map_len = app.world().resource::<BevyEntityMap>().len();
        Ok(VisibleWorldSpawnSummary {
            ground_spawned: true,
            object_count: presentation.objects.len(),
            stable_map_count: map_len,
            visible_signature: presentation.visible_signature.clone(),
        })
    }
}

fn valid_transition(from: GameAppState, to: GameAppState) -> bool {
    matches!(
        (from, to),
        (GameAppState::Boot, GameAppState::LoadConfig)
            | (GameAppState::LoadConfig, GameAppState::DevMenu)
            | (GameAppState::DevMenu, GameAppState::Running)
            | (GameAppState::DevMenu, GameAppState::Shutdown)
            | (GameAppState::Running, GameAppState::Paused)
            | (GameAppState::Paused, GameAppState::Running)
            | (GameAppState::Running, GameAppState::Shutdown)
            | (GameAppState::Paused, GameAppState::Shutdown)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p34_fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/alife_world/tests/fixtures/p34")
    }

    #[test]
    fn headless_app_shell_loads_p34_config_and_manifest() {
        let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
        let summary = run_headless_app_shell_smoke(&launch).unwrap();
        assert_eq!(summary.schema, G01_APP_SHELL_SCHEMA);
        assert_eq!(summary.schema_version, G01_APP_SHELL_SCHEMA_VERSION);
        assert_eq!(summary.seed, 4242);
        assert_eq!(summary.brain_class, "Nano512");
        assert_eq!(summary.requested_backend, BackendSelection::CpuReference);
        assert_eq!(summary.asset_count, 2);
        assert!(!summary.graphics_required_for_default_path);
        assert_eq!(
            summary.state_labels(),
            vec!["Boot", "LoadConfig", "DevMenu", "Running", "Shutdown"]
        );
    }

    #[test]
    fn paused_state_path_is_explicit_and_deterministic() {
        let mut launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
        launch.start_paused = true;
        let summary = run_headless_app_shell_smoke(&launch).unwrap();
        assert_eq!(
            summary.state_labels(),
            vec![
                "Boot",
                "LoadConfig",
                "DevMenu",
                "Running",
                "Paused",
                "Running",
                "Shutdown"
            ]
        );
    }

    #[test]
    fn invalid_config_rejects_with_p34_diagnostics() {
        let mut launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
        launch.config_path = launch.fixture_root.join("missing_config.json");
        let err = validate_app_shell_config(&launch).unwrap_err().to_string();
        assert!(err.contains("persistence/config error") || err.contains("io error"));
    }

    #[test]
    fn invalid_state_transition_is_rejected() {
        let mut trace = AppShellStateTrace::default();
        let err = trace.transition(GameAppState::Running).unwrap_err();
        assert!(matches!(
            err,
            GameAppShellError::InvalidTransition {
                from: GameAppState::Boot,
                to: GameAppState::Running
            }
        ));
    }

    #[test]
    fn visible_world_signature_loads_from_p34_save_without_bevy() {
        let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
        let presentation = load_visible_world_from_p34_save(&launch).unwrap();
        compare_visible_world_to_headless(&presentation).unwrap();
        assert_eq!(presentation.schema, G02_VISIBLE_WORLD_SCHEMA);
        assert_eq!(
            presentation.schema_version,
            G02_VISIBLE_WORLD_SCHEMA_VERSION
        );
        assert_eq!(presentation.seed, 4242);
        assert_eq!(presentation.object_count, 2);
        assert_eq!(presentation.kind_count(WorldObjectKind::Agent), 1);
        assert_eq!(presentation.kind_count(WorldObjectKind::Food), 1);
        assert!(presentation
            .visible_signature
            .iter()
            .any(|line| line.contains("Food:berry")));
    }

    #[test]
    fn placeholder_mapping_covers_g02_required_visual_kinds() {
        assert_eq!(
            placeholder_for_kind(WorldObjectKind::Agent),
            (
                VisiblePlaceholderShape::CreatureCapsule,
                VisibleMaterialKind::Creature
            )
        );
        assert_eq!(
            placeholder_for_kind(WorldObjectKind::Food),
            (
                VisiblePlaceholderShape::FoodSphere,
                VisibleMaterialKind::Food
            )
        );
        assert_eq!(
            placeholder_for_kind(WorldObjectKind::Hazard),
            (
                VisiblePlaceholderShape::HazardCone,
                VisibleMaterialKind::Hazard
            )
        );
        assert_eq!(
            placeholder_for_kind(WorldObjectKind::Obstacle),
            (
                VisiblePlaceholderShape::ObstacleCube,
                VisibleMaterialKind::Obstacle
            )
        );
        assert_eq!(
            placeholder_for_kind(WorldObjectKind::Token),
            (
                VisiblePlaceholderShape::TokenBillboard,
                VisibleMaterialKind::Token
            )
        );
    }

    #[test]
    fn creature_visual_mapping_is_bounded_and_readable() {
        let mut homeostasis = HomeostaticSnapshot::baseline(Tick::new(9));
        homeostasis.drives.hunger = 0.82;
        homeostasis.drives.fear = 0.20;
        homeostasis.drives.pain = 0.10;
        homeostasis.drives.curiosity = 0.55;
        homeostasis.drives.brain_atp = 0.72;
        homeostasis.hormones.sleep_pressure = 0.25;
        let visual = creature_visual_snapshot_from_parts(
            OrganismId(1),
            WorldEntityId(1),
            Vec3f::new(0.0, 0.0, 0.0),
            Some(WorldEntityId(2)),
            Some(Vec3f::new(2.0, 0.0, 0.0)),
            &homeostasis,
            SleepPhase::Awake,
            Some(ActionKind::Interact),
        )
        .unwrap();

        assert_eq!(visual.schema, G04_CREATURE_VISUAL_SCHEMA);
        assert_eq!(visual.schema_version, G04_CREATURE_VISUAL_SCHEMA_VERSION);
        assert_eq!(visual.animation, CreatureAnimationState::Interacting);
        assert_eq!(visual.expression, CreatureExpressionState::Hungry);
        assert_eq!(visual.facing, Vec3f::new(1.0, 0.0, 0.0));
        assert_eq!(visual.cues.hunger.value, 0.82);
        assert!(visual
            .base_rgba
            .iter()
            .chain(visual.accent_rgba.iter())
            .chain(visual.intent_rgba.iter())
            .all(|channel| (0.0..=1.0).contains(channel)));
        visual.validate().unwrap();
    }

    #[test]
    fn sleep_and_pain_override_action_visual_states_without_cognitive_mutation() {
        let mut homeostasis = HomeostaticSnapshot::baseline(Tick::new(11));
        homeostasis.drives.pain = 0.80;
        let pain_visual = creature_visual_snapshot_from_parts(
            OrganismId(1),
            WorldEntityId(1),
            Vec3f::ZERO,
            None,
            None,
            &homeostasis,
            SleepPhase::Awake,
            Some(ActionKind::Move),
        )
        .unwrap();
        assert_eq!(pain_visual.animation, CreatureAnimationState::Hurt);
        assert_eq!(pain_visual.expression, CreatureExpressionState::Pained);

        let sleep_visual = creature_visual_snapshot_from_parts(
            OrganismId(1),
            WorldEntityId(1),
            Vec3f::ZERO,
            None,
            None,
            &homeostasis,
            SleepPhase::Consolidating,
            Some(ActionKind::Move),
        )
        .unwrap();
        assert_eq!(sleep_visual.animation, CreatureAnimationState::Sleeping);
        assert_eq!(sleep_visual.expression, CreatureExpressionState::Tired);
    }

    #[test]
    fn g04_creature_visual_smoke_derives_from_g03_tick_summary() {
        let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
        let visual = run_creature_visual_smoke(&launch).unwrap();
        assert_eq!(visual.organism_id, OrganismId(1));
        assert_eq!(visual.stable_id, WorldEntityId(1));
        assert_eq!(visual.selected_action_kind, Some(ActionKind::Interact));
        assert_eq!(visual.target_entity, Some(WorldEntityId(2)));
        assert_eq!(visual.animation, CreatureAnimationState::Interacting);
        assert!(visual.debug_summary.contains("organism=1"));
        visual.validate().unwrap();
    }

    #[test]
    fn g05_camera_controls_are_bounded_and_deterministic() {
        let camera = CameraNavigationState::top_down_default()
            .pan_by(2.0, -3.5)
            .unwrap()
            .zoom_by(20.0)
            .unwrap()
            .orbit_by(-45.0)
            .unwrap()
            .with_follow_target(WorldEntityId(1))
            .unwrap();

        assert_eq!(camera.zoom, 8.0);
        assert_eq!(camera.yaw_degrees, 315.0);
        assert_eq!(camera.follow_target, Some(WorldEntityId(1)));
        assert!(camera.signature_line().contains("315.00"));
        camera.validate().unwrap();
    }

    #[test]
    fn g05_selection_uses_stable_ids_from_visible_world() {
        let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
        let presentation = load_visible_world_from_p34_save(&launch).unwrap();
        let selection = select_visible_world_entity(&presentation, WorldEntityId(1)).unwrap();
        assert_eq!(selection.schema, G05_CAMERA_INSPECTOR_SCHEMA);
        assert_eq!(
            selection.schema_version,
            G05_CAMERA_INSPECTOR_SCHEMA_VERSION
        );
        assert_eq!(selection.stable_id, WorldEntityId(1));
        assert_eq!(selection.organism_id, Some(OrganismId(1)));
        assert_eq!(selection.kind, WorldObjectKind::Agent);
        assert!(selection.debug_label.contains("Agent"));
        selection.validate().unwrap();

        assert!(select_visible_world_entity(&presentation, WorldEntityId(99_999)).is_err());
    }

    #[test]
    fn g05_inspector_snapshot_is_read_only_and_covers_expected_fields() {
        let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
        let inspector = run_creature_inspector_smoke(&launch).unwrap();
        assert_eq!(inspector.schema, G05_CAMERA_INSPECTOR_SCHEMA);
        assert_eq!(
            inspector.schema_version,
            G05_CAMERA_INSPECTOR_SCHEMA_VERSION
        );
        assert!(inspector.read_only);
        assert_eq!(inspector.selection.stable_id, WorldEntityId(1));
        assert_eq!(inspector.camera.follow_target, Some(WorldEntityId(1)));
        assert_eq!(
            inspector.visual.selected_action_kind,
            Some(ActionKind::Interact)
        );
        assert!(inspector.action_summary.contains("Interact"));
        assert!(inspector.patch_summary.contains("sealed=true"));
        assert!(inspector
            .memory_topology_summary
            .contains("memory_updates=1"));
        assert!(inspector
            .drive_lines
            .iter()
            .any(|line| line.starts_with("hunger=")));
        assert!(inspector
            .hormone_lines
            .iter()
            .any(|line| line.starts_with("sleep_pressure=")));
        assert!(inspector
            .troubleshooting_messages
            .iter()
            .any(|line| line.contains("gpu_runtime=optional")));
        inspector.validate().unwrap();
    }

    #[test]
    fn g05_pause_step_run_controls_map_to_live_tick_controls() {
        let paused = InspectorControlPanel::paused();
        assert_eq!(
            paused.to_live_control().unwrap(),
            LiveBrainTickControl::paused()
        );
        let step = InspectorControlPanel::step_once();
        assert_eq!(
            step.to_live_control().unwrap(),
            LiveBrainTickControl::step_once()
        );
        let run = InspectorControlPanel::run_fixed(3, 150);
        assert_eq!(
            run.to_live_control().unwrap(),
            LiveBrainTickControl::run_fixed(3)
        );
        assert!(InspectorControlPanel::run_fixed(32, 100)
            .validate()
            .is_err());
    }

    #[cfg(feature = "bevy-app")]
    #[test]
    fn feature_gated_bevy_shell_builds_with_adapter_plugin() {
        let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
        let summary = run_headless_app_shell_smoke(&launch).unwrap();
        let mut app = crate::bevy_shell::build_minimal_bevy_app_shell(summary);
        app.update();
        assert!(app
            .world()
            .get_resource::<alife_bevy_adapter::AdapterScheduleTrace>()
            .is_some());
    }

    #[cfg(feature = "bevy-app")]
    #[test]
    fn feature_gated_visible_world_spawns_stable_mapped_entities() {
        let launch = AppShellLaunchConfig::from_p34_fixture_root(p34_fixture_root());
        let (mut app, summary) = crate::bevy_shell::build_visible_world_app_shell(&launch).unwrap();
        assert!(summary.ground_spawned);
        assert_eq!(summary.object_count, 2);
        assert_eq!(summary.stable_map_count, 2);
        let mut visible_query = app
            .world_mut()
            .query::<&crate::bevy_shell::VisibleWorldObject>();
        let visible = visible_query.iter(app.world()).collect::<Vec<_>>();
        assert_eq!(visible.len(), 2);
        let map = app.world().resource::<alife_bevy_adapter::BevyEntityMap>();
        for object in visible {
            assert!(map.bevy_entity(object.stable_id).is_some());
        }
        let mut ground_query = app
            .world_mut()
            .query::<&crate::bevy_shell::VisibleGroundPlane>();
        assert_eq!(ground_query.iter(app.world()).count(), 1);
    }
}
