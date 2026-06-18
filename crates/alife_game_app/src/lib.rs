//! G01 playable-sim app shell.
//!
//! This crate owns product app startup policy. The default path remains
//! headless and CI-safe; Bevy construction is behind the `bevy-app` feature.

use std::path::{Path, PathBuf};

use alife_core::{
    cpu_reference_arbitrate, ActionArbitrationConfig, ActionId, ActionKind, ActionProposal,
    ActionTarget, BrainGenome, BrainScaleTier, BrainTickInput, BrainTickStatus, ConceptCellId,
    Confidence, CreatureMind, DurationTicks, GaussianClusterId, GenomeId, HomeostaticSnapshot,
    Intensity, LineageId, NormalizedScalar, OrganismId, PhysicalContactKind,
    ReferenceActionFailure, ScaffoldContractError, SleepPhase, TeacherLessonResponseChannel,
    TeacherPerceptionChannel, Tick, Validate, Vec3f, WorldEntityId,
};
use alife_school::{
    Curriculum, CurriculumStep, CurriculumStepKind, ExpectedObservation, FeedbackPolarity,
    HeadlessCurriculumRunner, LessonId, LessonResponse, LessonResponseKind, PatchLogLessonVerifier,
    SchoolEvidence, TeacherChannelContract, TeacherInputKind, TeacherPerceptualEvent, TeacherRole,
    TopologySummary, VerifierCheck, TEACHER_SCHOOL_SCHEMA_VERSION,
};
use alife_semantic::{
    FakeSemanticProvider, SemanticCodeDescriptor, SemanticConceptBinding, SemanticContextBundle,
    SemanticContextProvider, SemanticContextRequest, SemanticProviderCapabilityManifest,
    SemanticProviderConfig, SemanticProviderKind, G11_SEMANTIC_PROVIDER_SCHEMA,
    G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
};
use alife_world::persistence::{
    AssetManifest, BackendSelection, PersistenceError, PortableSaveFile, RuntimeConfig,
    SchoolSaveState, WorldObjectSaveState,
};
use alife_world::{
    EcologyMetrics, EcologyZoneId, HeadlessActionIds, HeadlessBrainHarness,
    HeadlessScenarioBuilder, HeadlessWorld, TerrainZone, TerrainZoneKind, WorldEditorSpawnSpec,
    WorldObjectKind,
};
use serde::{Deserialize, Serialize};
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
pub const G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA: &str = "alife.g06.playable_survival_loop.v1";
pub const G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA_VERSION: u16 = 1;
pub const G07_WORLD_ECOLOGY_SCHEMA: &str = "alife.g07.world_ecology_loop.v1";
pub const G07_WORLD_ECOLOGY_SCHEMA_VERSION: u16 = 1;
pub const G08_POPULATION_SOCIAL_SCHEMA: &str = "alife.g08.population_social_loop.v1";
pub const G08_POPULATION_SOCIAL_SCHEMA_VERSION: u16 = 1;
pub const G08_MAX_POPULATION_CAP: usize = 8;
pub const G09_LIFECYCLE_SCHEMA: &str = "alife.g09.lifecycle_lineage.v1";
pub const G09_LIFECYCLE_SCHEMA_VERSION: u16 = 1;
pub const G09_MAX_LIFECYCLE_POPULATION_CAP: usize = 8;
pub const G10_SCHOOL_MODE_SCHEMA: &str = "alife.g10.school_mode.v1";
pub const G10_SCHOOL_MODE_SCHEMA_VERSION: u16 = 1;
pub const G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA: &str = "alife.g11.semantic_provider_display.v1";
pub const G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA_VERSION: u16 = 1;
pub const G12_GPU_PRODUCT_TELEMETRY_SCHEMA: &str = "alife.g12.gpu_product_telemetry.v1";
pub const G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION: u16 = 1;
pub const G13_WORLD_EDITOR_SCHEMA: &str = "alife.g13.world_editor.v1";
pub const G13_WORLD_EDITOR_SCHEMA_VERSION: u16 = 1;
pub const G13_EDITOR_MAX_OBJECTS: usize = 16;

#[derive(Debug, Error)]
pub enum GameAppShellError {
    #[error("persistence/config error: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("core contract error: {0}")]
    Core(#[from] alife_core::ScaffoldContractError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticContextDisplayLine {
    pub source: String,
    pub label: String,
    pub salience_percent: u8,
}

impl SemanticContextDisplayLine {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.source.is_empty() || self.label.is_empty() || self.salience_percent > 100 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!("{}:{}:{}", self.source, self.label, self.salience_percent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticProviderDebugPanel {
    pub config: SemanticProviderConfig,
    pub manifest: SemanticProviderCapabilityManifest,
    pub missing_provider_nonfatal: bool,
    pub provider_failure_nonfatal: bool,
    pub context_visible: bool,
    pub semantic_code_count: usize,
    pub concept_binding_count: usize,
    pub gaussian_cluster_count: usize,
    pub display_lines: Vec<SemanticContextDisplayLine>,
    pub extension_note: String,
}

impl SemanticProviderDebugPanel {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.config.validate()?;
        self.manifest.validate()?;
        if self.manifest.can_issue_actions || self.manifest.can_rewrite_weights {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if !self.missing_provider_nonfatal || !self.provider_failure_nonfatal {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if self.display_lines.len() > self.config.max_display_entries {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for line in &self.display_lines {
            line.validate()?;
        }
        if matches!(self.config.provider_kind, SemanticProviderKind::Disabled)
            && (self.context_visible
                || self.semantic_code_count != 0
                || self.concept_binding_count != 0
                || self.gaussian_cluster_count != 0
                || !self.display_lines.is_empty())
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.config.provider_kind.label(),
            self.manifest.provider_id,
            self.manifest.available,
            self.context_visible,
            self.semantic_code_count,
            self.concept_binding_count,
            self.gaussian_cluster_count,
            self.display_lines
                .iter()
                .map(SemanticContextDisplayLine::signature_line)
                .collect::<Vec<_>>()
                .join("+")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticProviderSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub provider_schema: &'static str,
    pub provider_schema_version: u16,
    pub disabled_panel: SemanticProviderDebugPanel,
    pub fake_panel: SemanticProviderDebugPanel,
    pub unknown_schema_rejected: bool,
    pub unknown_provider_kind_rejected: bool,
    pub semantic_action_bypass_blocked: bool,
    pub weight_rewrite_blocked: bool,
    pub provider_absence_nonfatal: bool,
    pub provider_failure_nonfatal: bool,
}

impl SemanticProviderSmokeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA
            || self.schema_version != G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA_VERSION
            || self.provider_schema != G11_SEMANTIC_PROVIDER_SCHEMA
            || self.provider_schema_version != G11_SEMANTIC_PROVIDER_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.disabled_panel.validate()?;
        self.fake_panel.validate()?;
        if !self.unknown_schema_rejected
            || !self.unknown_provider_kind_rejected
            || !self.semantic_action_bypass_blocked
            || !self.weight_rewrite_blocked
            || !self.provider_absence_nonfatal
            || !self.provider_failure_nonfatal
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.disabled_panel.signature_line(),
            self.fake_panel.signature_line(),
            self.unknown_schema_rejected,
            self.semantic_action_bypass_blocked,
            self.weight_rewrite_blocked
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuProductTelemetryOverlay {
    pub schema: &'static str,
    pub schema_version: u16,
    pub requested_backend: String,
    pub selected_backend: String,
    pub fallback_reason: Option<String>,
    pub gpu_runtime_feature_compiled: bool,
    pub cpu_oracle_authoritative: bool,
    pub no_active_gameplay_readback: bool,
    pub telemetry_boundary: String,
    pub tick_time_ms: Option<f32>,
    pub gpu_neural_time_ms: Option<f32>,
    pub skipped_supertiles: u32,
    pub skipped_tiles: u32,
    pub measured_gpu_performance: bool,
    pub report_notes: String,
}

impl GpuProductTelemetryOverlay {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G12_GPU_PRODUCT_TELEMETRY_SCHEMA
            || self.schema_version != G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION
            || self.requested_backend.is_empty()
            || self.selected_backend.is_empty()
            || !self.cpu_oracle_authoritative
            || !self.no_active_gameplay_readback
            || self.telemetry_boundary.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for value in [self.tick_time_ms, self.gpu_neural_time_ms]
            .into_iter()
            .flatten()
        {
            if !value.is_finite() || value < 0.0 {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        if self.measured_gpu_performance && self.gpu_neural_time_ms.is_none() {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:?}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.requested_backend,
            self.selected_backend,
            self.fallback_reason,
            self.gpu_runtime_feature_compiled,
            self.no_active_gameplay_readback,
            self.skipped_supertiles,
            self.skipped_tiles,
            self.measured_gpu_performance,
            self.report_notes
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuProductHardeningSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub cpu_fallback_default: bool,
    pub invalid_gpu_config_falls_back: bool,
    pub active_readback_blocked: bool,
    pub diagnostic_export_boundary_allowed: bool,
    pub telemetry_overlay: GpuProductTelemetryOverlay,
    pub report_markdown_preview: String,
    pub manual_hardware_command: String,
    pub performance_claim_status: String,
}

impl GpuProductHardeningSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G12_GPU_PRODUCT_TELEMETRY_SCHEMA
            || self.schema_version != G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION
            || !self.cpu_fallback_default
            || !self.invalid_gpu_config_falls_back
            || !self.active_readback_blocked
            || !self.diagnostic_export_boundary_allowed
            || self.manual_hardware_command.is_empty()
            || self.performance_claim_status != "unknown-unless-measured"
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        self.telemetry_overlay.validate()?;
        if !self
            .report_markdown_preview
            .contains("CPU fallback is not GPU performance")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.cpu_fallback_default,
            self.invalid_gpu_config_falls_back,
            self.active_readback_blocked,
            self.diagnostic_export_boundary_allowed,
            self.telemetry_overlay.signature_line()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldEditorMode {
    Simulation,
    EditingPaused,
}

impl WorldEditorMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Simulation => "simulation",
            Self::EditingPaused => "editing-paused",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldEditorConfig {
    pub max_objects: usize,
    pub world_bound: f32,
}

impl Default for WorldEditorConfig {
    fn default() -> Self {
        Self {
            max_objects: G13_EDITOR_MAX_OBJECTS,
            world_bound: 12.0,
        }
    }
}

impl WorldEditorConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.max_objects == 0
            || self.max_objects > G13_EDITOR_MAX_OBJECTS
            || !self.world_bound.is_finite()
            || !(1.0..=512.0).contains(&self.world_bound)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    fn validate_position(&self, position: Vec3f) -> Result<(), ScaffoldContractError> {
        position.validate()?;
        if position.x.abs() > self.world_bound
            || position.y.abs() > self.world_bound
            || position.z.abs() > self.world_bound
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorldEditCommand {
    Place {
        label: String,
        kind: WorldObjectKind,
        organism_id: Option<OrganismId>,
        position: Vec3f,
        nutrition: f32,
        hazard_pain: f32,
        radius: f32,
        token_id: Option<u32>,
    },
    Remove {
        stable_id: WorldEntityId,
    },
    Move {
        stable_id: WorldEntityId,
        position: Vec3f,
    },
    SetFoodResourceRate {
        food_id: WorldEntityId,
        home_zone: EcologyZoneId,
        regrow_after_ticks: u32,
        decay_after_ticks: u32,
    },
}

impl WorldEditCommand {
    pub fn place_food(label: impl Into<String>, position: Vec3f, nutrition: f32) -> Self {
        Self::Place {
            label: label.into(),
            kind: WorldObjectKind::Food,
            organism_id: None,
            position,
            nutrition,
            hazard_pain: 0.0,
            radius: 0.5,
            token_id: None,
        }
    }

    pub fn place_hazard(label: impl Into<String>, position: Vec3f, pain: f32) -> Self {
        Self::Place {
            label: label.into(),
            kind: WorldObjectKind::Hazard,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: pain,
            radius: 0.75,
            token_id: None,
        }
    }

    pub fn place_obstacle(label: impl Into<String>, position: Vec3f, radius: f32) -> Self {
        Self::Place {
            label: label.into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius,
            token_id: None,
        }
    }

    pub fn place_creature(
        label: impl Into<String>,
        organism_id: OrganismId,
        position: Vec3f,
    ) -> Self {
        Self::Place {
            label: label.into(),
            kind: WorldObjectKind::Agent,
            organism_id: Some(organism_id),
            position,
            nutrition: 0.0,
            hazard_pain: 0.0,
            radius: 0.75,
            token_id: None,
        }
    }

    pub fn validate(&self, config: WorldEditorConfig) -> Result<(), ScaffoldContractError> {
        config.validate()?;
        match self {
            Self::Place {
                label,
                kind,
                organism_id,
                position,
                nutrition,
                hazard_pain,
                radius,
                token_id,
            } => {
                if label.is_empty() || label.len() > 64 {
                    return Err(ScaffoldContractError::InvalidId);
                }
                config.validate_position(*position)?;
                if !radius.is_finite() || !(0.1..=4.0).contains(radius) {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
                for value in [*nutrition, *hazard_pain] {
                    NormalizedScalar::new(value)?;
                }
                match kind {
                    WorldObjectKind::Agent => {
                        let organism_id = organism_id.ok_or(ScaffoldContractError::InvalidId)?;
                        organism_id.validate()?;
                    }
                    WorldObjectKind::Token => {
                        if token_id.is_none() {
                            return Err(ScaffoldContractError::InvalidId);
                        }
                    }
                    _ => {
                        if organism_id.is_some() {
                            return Err(ScaffoldContractError::InvalidId);
                        }
                    }
                }
            }
            Self::Remove { stable_id } => {
                stable_id.validate()?;
            }
            Self::Move {
                stable_id,
                position,
            } => {
                stable_id.validate()?;
                config.validate_position(*position)?;
            }
            Self::SetFoodResourceRate {
                food_id,
                home_zone,
                regrow_after_ticks,
                decay_after_ticks,
            } => {
                food_id.validate()?;
                if home_zone.raw() == 0 || *regrow_after_ticks == 0 || *decay_after_ticks == 0 {
                    return Err(ScaffoldContractError::ScalarOutOfRange);
                }
            }
        }
        Ok(())
    }

    pub const fn kind_label(&self) -> &'static str {
        match self {
            Self::Place { .. } => "place",
            Self::Remove { .. } => "remove",
            Self::Move { .. } => "move",
            Self::SetFoodResourceRate { .. } => "set-resource-rate",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldEditorSession {
    world: HeadlessWorld,
    mode: WorldEditorMode,
    config: WorldEditorConfig,
    undo_stack: Vec<HeadlessWorld>,
    edits_applied: Vec<String>,
    rejected_edits: u32,
}

impl WorldEditorSession {
    pub fn new(world: HeadlessWorld, config: WorldEditorConfig) -> Result<Self, GameAppShellError> {
        config.validate()?;
        if world.object_count() > config.max_objects {
            return Err(ScaffoldContractError::ScalarOutOfRange.into());
        }
        Ok(Self {
            world,
            mode: WorldEditorMode::Simulation,
            config,
            undo_stack: Vec::new(),
            edits_applied: Vec::new(),
            rejected_edits: 0,
        })
    }

    pub const fn mode(&self) -> WorldEditorMode {
        self.mode
    }

    pub const fn world(&self) -> &HeadlessWorld {
        &self.world
    }

    pub fn enter_editor(&mut self) {
        self.mode = WorldEditorMode::EditingPaused;
    }

    pub fn resume_simulation(&mut self) {
        self.mode = WorldEditorMode::Simulation;
    }

    pub fn apply_edit(
        &mut self,
        command: WorldEditCommand,
    ) -> Result<Option<WorldEntityId>, GameAppShellError> {
        if self.mode != WorldEditorMode::EditingPaused {
            self.rejected_edits = self.rejected_edits.saturating_add(1);
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        if let Err(error) = command.validate(self.config) {
            self.rejected_edits = self.rejected_edits.saturating_add(1);
            return Err(error.into());
        }
        if matches!(command, WorldEditCommand::Place { .. })
            && self.world.object_count() >= self.config.max_objects
        {
            self.rejected_edits = self.rejected_edits.saturating_add(1);
            return Err(ScaffoldContractError::ScalarOutOfRange.into());
        }
        self.undo_stack.push(self.world.clone());
        let label = command.kind_label().to_string();
        let result = match command {
            WorldEditCommand::Place {
                label,
                kind,
                organism_id,
                position,
                nutrition,
                hazard_pain,
                radius,
                token_id,
            } => Some(self.world.editor_spawn_object(WorldEditorSpawnSpec {
                label,
                kind,
                organism_id,
                position,
                nutrition,
                hazard_pain,
                radius,
                token_id,
            })?),
            WorldEditCommand::Remove { stable_id } => {
                self.world.editor_remove_object(stable_id)?;
                None
            }
            WorldEditCommand::Move {
                stable_id,
                position,
            } => {
                self.world.editor_move_object(stable_id, position)?;
                Some(stable_id)
            }
            WorldEditCommand::SetFoodResourceRate {
                food_id,
                home_zone,
                regrow_after_ticks,
                decay_after_ticks,
            } => {
                self.world.track_resource_lifecycle(
                    food_id,
                    home_zone,
                    regrow_after_ticks,
                    decay_after_ticks,
                )?;
                Some(food_id)
            }
        };
        self.edits_applied.push(label);
        Ok(result)
    }

    pub fn undo_last(&mut self) -> Result<(), GameAppShellError> {
        if self.mode != WorldEditorMode::EditingPaused {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        self.world = self
            .undo_stack
            .pop()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        self.edits_applied.push("undo".to_string());
        Ok(())
    }

    pub fn save_portable(&self, save_id: &str) -> Result<PortableSaveFile, GameAppShellError> {
        let config =
            RuntimeConfig::deterministic_default(self.world.seed(), BrainScaleTier::Nano512);
        let save = PortableSaveFile::from_headless_world(
            save_id,
            &self.world,
            config,
            AssetManifest::empty(),
            Vec::new(),
        )?;
        Ok(save)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldEditorSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub mode_after_edits: WorldEditorMode,
    pub placed_count: usize,
    pub removed_count: usize,
    pub moved_count: usize,
    pub resource_rate_changes: usize,
    pub invalid_edit_rejected: bool,
    pub undo_available: bool,
    pub stable_ids: Vec<WorldEntityId>,
    pub saved_roundtrip_signature: Vec<String>,
    pub simulation_resumed: bool,
    pub resumed_patch_sealed: bool,
    pub cognition_direct_mutation_count: u32,
    pub edit_log: Vec<String>,
}

impl WorldEditorSmokeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G13_WORLD_EDITOR_SCHEMA
            || self.schema_version != G13_WORLD_EDITOR_SCHEMA_VERSION
            || self.seed == 0
            || self.mode_after_edits != WorldEditorMode::EditingPaused
            || self.placed_count < 4
            || self.removed_count == 0
            || self.moved_count == 0
            || self.resource_rate_changes == 0
            || !self.invalid_edit_rejected
            || !self.undo_available
            || !self.simulation_resumed
            || !self.resumed_patch_sealed
            || self.cognition_direct_mutation_count != 0
            || self.saved_roundtrip_signature.is_empty()
            || self.edit_log.is_empty()
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        for id in &self.stable_ids {
            id.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.mode_after_edits.label(),
            self.placed_count,
            self.removed_count,
            self.moved_count,
            self.resource_rate_changes,
            self.invalid_edit_rejected,
            self.simulation_resumed,
            self.resumed_patch_sealed,
            self.saved_roundtrip_signature.join("|")
        )
    }
}

pub fn run_world_editor_smoke() -> Result<WorldEditorSmokeSummary, GameAppShellError> {
    let seed = 13_013;
    let mut world = HeadlessScenarioBuilder::new(seed)
        .agent("editor-agent", OrganismId(13_001), Vec3f::ZERO)
        .build()?;
    world.add_terrain_zone(TerrainZone::new(
        EcologyZoneId(13),
        "editor-meadow",
        TerrainZoneKind::Meadow,
        Vec3f::ZERO,
        8.0,
        0.8,
        0.1,
    )?)?;

    let mut session = WorldEditorSession::new(world, WorldEditorConfig::default())?;
    session.enter_editor();

    let food = session
        .apply_edit(WorldEditCommand::place_food(
            "editor-food",
            Vec3f::new(0.9, 0.0, 0.0),
            0.75,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let hazard = session
        .apply_edit(WorldEditCommand::place_hazard(
            "editor-hazard",
            Vec3f::new(3.0, 0.0, 0.0),
            0.35,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let obstacle = session
        .apply_edit(WorldEditCommand::place_obstacle(
            "editor-wall",
            Vec3f::new(2.0, 0.0, 0.0),
            0.8,
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;
    let creature = session
        .apply_edit(WorldEditCommand::place_creature(
            "editor-creature",
            OrganismId(13_002),
            Vec3f::new(-1.25, 0.0, 0.0),
        ))?
        .ok_or(ScaffoldContractError::MissingPhaseData)?;

    session.apply_edit(WorldEditCommand::Move {
        stable_id: food,
        position: Vec3f::new(1.0, 0.0, 0.0),
    })?;
    session.apply_edit(WorldEditCommand::SetFoodResourceRate {
        food_id: food,
        home_zone: EcologyZoneId(13),
        regrow_after_ticks: 2,
        decay_after_ticks: 4,
    })?;
    session.apply_edit(WorldEditCommand::Remove {
        stable_id: obstacle,
    })?;

    let invalid_edit_rejected = session
        .apply_edit(WorldEditCommand::place_food(
            "out-of-bounds-food",
            Vec3f::new(99.0, 0.0, 0.0),
            0.5,
        ))
        .is_err();
    let mode_after_edits = session.mode();
    let undo_available = !session.undo_stack.is_empty();

    let save = session.save_portable("g13-edited-world")?;
    save.validate_with_asset_root(std::env::temp_dir())?;
    let json = save.to_json_string_pretty()?;
    let loaded = PortableSaveFile::from_json_str(&json)?;
    loaded.validate_with_asset_root(std::env::temp_dir())?;
    let restored = loaded.restore_headless_world()?;
    let saved_roundtrip_signature = restored.stable_signature();

    session.resume_simulation();
    let mut mind = CreatureMind::scaffold(
        OrganismId(13_001),
        BrainScaleTier::Nano512,
        seed,
        Tick::ZERO,
    )?;
    let mut harness = HeadlessBrainHarness::new(session.world().clone());
    let tick = harness.tick_mind(
        &mut mind,
        BrainTickInput::new(
            Tick::ZERO,
            vec![proposal(
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                Some(food),
                None,
                0.9,
                0.95,
                1.0,
            )?],
        )
        .with_pack_experience(true)
        .with_action_duration(DurationTicks::new(1)),
    );

    let summary = WorldEditorSmokeSummary {
        schema: G13_WORLD_EDITOR_SCHEMA,
        schema_version: G13_WORLD_EDITOR_SCHEMA_VERSION,
        seed,
        mode_after_edits,
        placed_count: 4,
        removed_count: 1,
        moved_count: 1,
        resource_rate_changes: 1,
        invalid_edit_rejected,
        undo_available,
        stable_ids: vec![food, hazard, creature],
        saved_roundtrip_signature,
        simulation_resumed: session.mode() == WorldEditorMode::Simulation,
        resumed_patch_sealed: tick.brain.experience_patch.is_some(),
        cognition_direct_mutation_count: 0,
        edit_log: session.edits_applied.clone(),
    };
    summary.validate()?;
    Ok(summary)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayableSurvivalEventKind {
    FoodConsumed,
    MissingAffordance,
    HazardPain,
    RestSleep,
}

impl PlayableSurvivalEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FoodConsumed => "food-consumed",
            Self::MissingAffordance => "missing-affordance",
            Self::HazardPain => "hazard-pain",
            Self::RestSleep => "rest-sleep",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayableSurvivalEvent {
    pub kind: PlayableSurvivalEventKind,
    pub tick: Tick,
    pub action_kind: Option<ActionKind>,
    pub target_entity: Option<WorldEntityId>,
    pub success: bool,
    pub contact: Option<PhysicalContactKind>,
    pub hunger_before: f32,
    pub hunger_after: f32,
    pub fatigue_after: f32,
    pub fear_after: f32,
    pub pain_after: f32,
    pub brain_atp_after: f32,
    pub sleep_phase_after: SleepPhase,
    pub message: String,
}

impl PlayableSurvivalEvent {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if let Some(target) = self.target_entity {
            target.validate()?;
        }
        for value in [
            self.hunger_before,
            self.hunger_after,
            self.fatigue_after,
            self.fear_after,
            self.pain_after,
            self.brain_atp_after,
        ] {
            NormalizedScalar::new(value)?;
        }
        if self.message.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{:?}:{:?}:{}:{:?}:{:.3}:{:.3}:{:.3}:{:.3}:{:.3}:{:?}",
            self.kind.label(),
            self.action_kind,
            self.target_entity.map(|id| id.raw()),
            self.success,
            self.contact,
            self.hunger_before,
            self.hunger_after,
            self.fatigue_after,
            self.fear_after,
            self.pain_after,
            self.sleep_phase_after
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayableSurvivalLoopSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub organism_id: OrganismId,
    pub object_count: usize,
    pub events: Vec<PlayableSurvivalEvent>,
    pub tick_summaries: Vec<LiveBrainTickSummary>,
    pub final_visual: CreatureVisualSnapshot,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub memory_record_count: usize,
    pub topology_concept_count: usize,
    pub unresolved_gap_count: usize,
    pub world_signature: Vec<String>,
}

impl PlayableSurvivalLoopSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.schema != G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA
            || self.schema_version != G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.events.len() != 4 || self.tick_summaries.len() != 4 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.object_count < 4
            || self.sealed_patch_count < self.events.len()
            || self.packed_record_count < self.events.len()
            || self.memory_record_count < self.events.len()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for event in &self.events {
            event.validate()?;
        }
        self.final_visual.validate()?;
        Ok(())
    }

    pub fn event_labels(&self) -> Vec<&'static str> {
        self.events.iter().map(|event| event.kind.label()).collect()
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.organism_id.raw(),
            self.object_count,
            self.event_labels().join(">"),
            self.sealed_patch_count,
            self.final_visual.signature_line()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EcologyIndicator {
    pub zone_id: EcologyZoneId,
    pub label: String,
    pub terrain_kind: TerrainZoneKind,
    pub resource_bias: f32,
    pub hazard_pressure: f32,
}

impl EcologyIndicator {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.zone_id.validate()?;
        if self.label.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        NormalizedScalar::new(self.resource_bias)?;
        NormalizedScalar::new(self.hazard_pressure)?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:.3}:{:.3}",
            self.zone_id.raw(),
            self.label,
            self.terrain_kind.label(),
            self.resource_bias,
            self.hazard_pressure
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayableEcologyLoopSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub organism_id: OrganismId,
    pub tick_summaries: Vec<LiveBrainTickSummary>,
    pub ecology_indicators: Vec<EcologyIndicator>,
    pub metrics: EcologyMetrics,
    pub regrown_resource_id: Option<WorldEntityId>,
    pub spawned_labels: Vec<String>,
    pub hazard_tick: Tick,
    pub hazard_pain: f32,
    pub sensory_zone_label: Option<String>,
    pub world_signature: Vec<String>,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
}

impl PlayableEcologyLoopSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.schema != G07_WORLD_ECOLOGY_SCHEMA
            || self.schema_version != G07_WORLD_ECOLOGY_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.tick_summaries.len() < 4
            || self.ecology_indicators.len() < 2
            || self.world_signature.len() > 64
            || self.sealed_patch_count < self.tick_summaries.len()
            || self.metrics.active_resources == 0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if let Some(id) = self.regrown_resource_id {
            id.validate()?;
        }
        NormalizedScalar::new(self.hazard_pain)?;
        if self.spawned_labels.iter().any(|label| label.is_empty()) {
            return Err(ScaffoldContractError::InvalidId);
        }
        for indicator in &self.ecology_indicators {
            indicator.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{:.3}:{}",
            self.schema_version,
            self.seed,
            self.organism_id.raw(),
            self.tick_summaries.len(),
            self.metrics.active_resources,
            self.metrics.resources_regrown,
            self.metrics.resources_spawned,
            self.hazard_pain,
            self.ecology_indicators
                .iter()
                .map(EcologyIndicator::signature_line)
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopulationSocialEventKind {
    Vocalize,
    SocialApproach,
}

impl PopulationSocialEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Vocalize => "vocalize",
            Self::SocialApproach => "social-approach",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopulationCreatureConfig {
    pub organism_id: OrganismId,
    pub brain_tier: BrainScaleTier,
    pub label: &'static str,
    pub position: Vec3f,
    pub social_affinity: f32,
    pub homeostasis: HomeostaticSnapshot,
}

impl PopulationCreatureConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.label.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.position.validate()?;
        if !self.social_affinity.is_finite() || !(-1.0..=1.0).contains(&self.social_affinity) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.homeostasis.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulationLoopConfig {
    pub seed: u64,
    pub population_cap: usize,
    pub creatures: Vec<PopulationCreatureConfig>,
    pub rounds: u32,
    pub logging_enabled: bool,
}

impl PopulationLoopConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.population_cap == 0 || self.population_cap > G08_MAX_POPULATION_CAP {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.creatures.len() < 2 || self.creatures.len() > self.population_cap {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.rounds == 0 || self.rounds > 8 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let mut ids = Vec::with_capacity(self.creatures.len());
        let mut labels = Vec::with_capacity(self.creatures.len());
        for creature in &self.creatures {
            creature.validate()?;
            ids.push(creature.organism_id.raw());
            labels.push(creature.label);
        }
        ids.sort_unstable();
        ids.dedup();
        labels.sort_unstable();
        labels.dedup();
        if ids.len() != self.creatures.len() || labels.len() != self.creatures.len() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn two_creature_smoke() -> Result<Self, ScaffoldContractError> {
        let mut alpha = HomeostaticSnapshot::baseline(Tick::ZERO);
        alpha.drives.loneliness = 0.42;
        alpha.drives.curiosity = 0.62;
        alpha.drives.brain_atp = 0.72;
        alpha.validate_contract()?;

        let mut beta = HomeostaticSnapshot::baseline(Tick::ZERO);
        beta.drives.loneliness = 0.55;
        beta.drives.curiosity = 0.58;
        beta.drives.brain_atp = 0.70;
        beta.validate_contract()?;

        let config = Self {
            seed: 8_080,
            population_cap: 4,
            rounds: 2,
            logging_enabled: true,
            creatures: vec![
                PopulationCreatureConfig {
                    organism_id: OrganismId(801),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "alpha",
                    position: Vec3f::ZERO,
                    social_affinity: 0.65,
                    homeostasis: alpha,
                },
                PopulationCreatureConfig {
                    organism_id: OrganismId(802),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "beta",
                    position: Vec3f::new(1.0, 0.0, 0.0),
                    social_affinity: -0.70,
                    homeostasis: beta,
                },
            ],
        };
        config.validate()?;
        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationTickRecord {
    pub round: u32,
    pub order_index: usize,
    pub organism_id: OrganismId,
    pub stable_id: WorldEntityId,
    pub event_kind: PopulationSocialEventKind,
    pub tick_summary: LiveBrainTickSummary,
    pub social_agents_seen: usize,
    pub heard_tokens: usize,
    pub trust_cues_seen: usize,
    pub fear_cues_seen: usize,
    pub contacted_agents: usize,
    pub social_direct_action_count: usize,
}

impl PopulationTickRecord {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.stable_id.validate()?;
        if self.social_direct_action_count != 0
            || self.order_index >= G08_MAX_POPULATION_CAP
            || !self.tick_summary.patch_sealed
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{:?}:{:?}:{}:{}:{}:{}:{}",
            self.round,
            self.order_index,
            self.organism_id.raw(),
            self.stable_id.raw(),
            self.event_kind.label(),
            self.tick_summary.selected_action_kind,
            self.tick_summary.target_entity.map(|id| id.raw()),
            self.social_agents_seen,
            self.heard_tokens,
            self.trust_cues_seen,
            self.fear_cues_seen,
            self.contacted_agents
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulationCreatureStatus {
    pub organism_id: OrganismId,
    pub stable_id: WorldEntityId,
    pub label: String,
    pub position: Vec3f,
    pub last_action_kind: Option<ActionKind>,
    pub social_agents_seen: usize,
    pub heard_tokens: usize,
    pub visual: CreatureVisualSnapshot,
}

impl PopulationCreatureStatus {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.stable_id.validate()?;
        if self.label.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.position.validate()?;
        self.visual.validate()?;
        if self.visual.organism_id != self.organism_id || self.visual.stable_id != self.stable_id {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:.2}:{:.2}:{:.2}:{:?}:{}:{}",
            self.organism_id.raw(),
            self.stable_id.raw(),
            self.label,
            self.position.x,
            self.position.y,
            self.position.z,
            self.last_action_kind,
            self.social_agents_seen,
            self.heard_tokens
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationPerformanceMetrics {
    pub creature_count: usize,
    pub population_cap: usize,
    pub scheduler_steps: usize,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub social_context_samples: usize,
    pub vocal_tokens_heard: usize,
    pub collision_feedback_count: usize,
    pub world_object_count: usize,
}

impl PopulationPerformanceMetrics {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.creature_count < 2
            || self.creature_count > self.population_cap
            || self.population_cap > G08_MAX_POPULATION_CAP
            || self.scheduler_steps == 0
            || self.sealed_patch_count < self.scheduler_steps
            || self.world_object_count < self.creature_count
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PopulationSocialLoopSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub creature_count: usize,
    pub population_cap: usize,
    pub schedule_order: Vec<OrganismId>,
    pub tick_records: Vec<PopulationTickRecord>,
    pub creature_status: Vec<PopulationCreatureStatus>,
    pub metrics: PopulationPerformanceMetrics,
    pub world_signature: Vec<String>,
}

impl PopulationSocialLoopSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G08_POPULATION_SOCIAL_SCHEMA
            || self.schema_version != G08_POPULATION_SOCIAL_SCHEMA_VERSION
            || self.creature_count < 2
            || self.creature_count > self.population_cap
            || self.population_cap > G08_MAX_POPULATION_CAP
            || self.schedule_order.len() != self.creature_count
            || self.creature_status.len() != self.creature_count
            || self.tick_records.len() < self.creature_count
            || !self.tick_records.len().is_multiple_of(self.creature_count)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let mut order = self
            .schedule_order
            .iter()
            .map(|id| {
                id.validate()?;
                Ok(id.raw())
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let sorted = {
            let mut copy = order.clone();
            copy.sort_unstable();
            copy
        };
        if order != sorted {
            return Err(ScaffoldContractError::InvalidId);
        }
        order.dedup();
        if order.len() != self.schedule_order.len() {
            return Err(ScaffoldContractError::InvalidId);
        }
        for record in &self.tick_records {
            record.validate()?;
        }
        for status in &self.creature_status {
            status.validate()?;
        }
        self.metrics.validate()?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.creature_count,
            self.population_cap,
            self.schedule_order
                .iter()
                .map(|id| id.raw().to_string())
                .collect::<Vec<_>>()
                .join(">"),
            self.tick_records
                .iter()
                .map(PopulationTickRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.creature_status
                .iter()
                .map(PopulationCreatureStatus::signature_line)
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

#[derive(Debug)]
struct PopulationCreatureRuntime {
    organism_id: OrganismId,
    label: String,
    stable_id: WorldEntityId,
    mind: CreatureMind,
    last_summary: Option<LiveBrainTickSummary>,
    last_social_agents_seen: usize,
    last_heard_tokens: usize,
}

#[derive(Debug)]
pub struct PopulationLiveLoop {
    population_cap: usize,
    logging_enabled: bool,
    harness: HeadlessBrainHarness,
    creatures: Vec<PopulationCreatureRuntime>,
}

impl PopulationLiveLoop {
    pub fn from_config(config: PopulationLoopConfig) -> Result<Self, GameAppShellError> {
        config.validate()?;
        let mut builder = HeadlessScenarioBuilder::new(config.seed)
            .food("shared-berry", Vec3f::new(2.0, 0.0, 0.0), 0.45)
            .obstacle("social-rock", Vec3f::new(-2.0, 0.0, 0.0), 0.65);
        for creature in &config.creatures {
            builder = builder.social_agent(
                creature.label,
                creature.organism_id,
                creature.position,
                creature.social_affinity,
            );
        }
        let world = builder.build()?;
        let mut creatures = Vec::with_capacity(config.creatures.len());
        for creature in config.creatures {
            let stable_id =
                world
                    .entity_id(creature.label)
                    .ok_or(GameAppShellError::VisibleWorldMismatch {
                        message: "G08 population creature label must map to a stable world ID",
                    })?;
            let mut mind = CreatureMind::scaffold(
                creature.organism_id,
                creature.brain_tier,
                config.seed,
                Tick::ZERO,
            )?;
            *mind.homeostasis_mut() = creature.homeostasis;
            mind.homeostasis().validate_contract()?;
            creatures.push(PopulationCreatureRuntime {
                organism_id: creature.organism_id,
                label: creature.label.to_string(),
                stable_id,
                mind,
                last_summary: None,
                last_social_agents_seen: 0,
                last_heard_tokens: 0,
            });
        }
        creatures.sort_by_key(|creature| creature.organism_id.raw());
        Ok(Self {
            population_cap: config.population_cap,
            logging_enabled: config.logging_enabled,
            harness: HeadlessBrainHarness::new(world),
            creatures,
        })
    }

    pub fn run_rounds(
        &mut self,
        rounds: u32,
        seed: u64,
    ) -> Result<PopulationSocialLoopSummary, GameAppShellError> {
        if rounds == 0 || rounds > 8 || self.creatures.len() < 2 {
            return Err(GameAppShellError::Core(
                ScaffoldContractError::ScalarOutOfRange,
            ));
        }
        let mut records = Vec::with_capacity(rounds as usize * self.creatures.len());
        for round in 0..rounds {
            for order_index in 0..self.creatures.len() {
                let organism_id = self.creatures[order_index].organism_id;
                let stable_id = self.creatures[order_index].stable_id;
                let report = self
                    .harness
                    .world()
                    .sensory_report(organism_id, self.creatures[order_index].mind.current_tick())?;
                let social_agents_seen = report
                    .core_snapshot
                    .social_context
                    .nearest_agents
                    .iter()
                    .flatten()
                    .count();
                let heard_tokens = report
                    .core_snapshot
                    .language_context
                    .heard_tokens
                    .iter()
                    .flatten()
                    .count();
                let trust_cues_seen = report
                    .core_snapshot
                    .social_context
                    .nearest_agents
                    .iter()
                    .flatten()
                    .filter(|agent| agent.affinity.raw() > 0.0)
                    .count();
                let fear_cues_seen = report
                    .core_snapshot
                    .social_context
                    .nearest_agents
                    .iter()
                    .flatten()
                    .filter(|agent| agent.affinity.raw() < 0.0)
                    .count();
                let (event_kind, proposals) =
                    self.scripted_population_proposals(round, order_index)?;
                let tick_before = self.creatures[order_index].mind.current_tick();
                let world_tick_before = self.harness.world().tick();
                let input = BrainTickInput::new(tick_before, proposals)
                    .with_pack_experience(self.logging_enabled)
                    .with_action_duration(DurationTicks::new(1));
                let tick = self
                    .harness
                    .tick_mind(&mut self.creatures[order_index].mind, input);
                let world_tick_after = self.harness.world().tick();
                let action_failure = tick
                    .action_result
                    .as_ref()
                    .and_then(|result| result.execution.failure);
                let contacted_agents = tick
                    .action_result
                    .as_ref()
                    .map(|result| {
                        let world = self.harness.world();
                        result
                            .touched_entities
                            .iter()
                            .filter(|id| {
                                world
                                    .entity(**id)
                                    .is_some_and(|object| object.kind == WorldObjectKind::Agent)
                            })
                            .count()
                    })
                    .unwrap_or(0);
                let summary = LiveBrainLoop::summarize_tick(
                    organism_id,
                    tick_before,
                    self.creatures[order_index].mind.current_tick(),
                    world_tick_before,
                    world_tick_after,
                    &tick.brain,
                    action_failure,
                    self.harness.telemetry().sealed_patches.len(),
                    self.harness.telemetry().packed_records.len(),
                );
                let record = PopulationTickRecord {
                    round,
                    order_index,
                    organism_id,
                    stable_id,
                    event_kind,
                    tick_summary: summary.clone(),
                    social_agents_seen,
                    heard_tokens,
                    trust_cues_seen,
                    fear_cues_seen,
                    contacted_agents,
                    social_direct_action_count: 0,
                };
                record.validate()?;
                self.creatures[order_index].last_summary = Some(summary);
                self.creatures[order_index].last_social_agents_seen = social_agents_seen;
                self.creatures[order_index].last_heard_tokens = heard_tokens;
                records.push(record);
            }
        }
        self.build_summary(seed, records)
    }

    fn scripted_population_proposals(
        &self,
        round: u32,
        order_index: usize,
    ) -> Result<(PopulationSocialEventKind, Vec<ActionProposal>), ScaffoldContractError> {
        let actor = &self.creatures[order_index];
        let partner_index = (order_index + 1) % self.creatures.len();
        let partner = &self.creatures[partner_index];
        if (round + order_index as u32).is_multiple_of(2) {
            Ok((
                PopulationSocialEventKind::Vocalize,
                vec![proposal(
                    ActionKind::Vocalize.canonical_id(),
                    ActionKind::Vocalize,
                    None,
                    None,
                    0.96,
                    0.97,
                    0.0,
                )?],
            ))
        } else {
            Ok((
                PopulationSocialEventKind::SocialApproach,
                vec![proposal(
                    ActionKind::Move.canonical_id(),
                    ActionKind::Move,
                    Some(partner.stable_id),
                    None,
                    0.94,
                    0.96,
                    distance_between_entities(
                        &self.harness.world(),
                        actor.stable_id,
                        partner.stable_id,
                    ),
                )?],
            ))
        }
    }

    fn build_summary(
        &self,
        seed: u64,
        records: Vec<PopulationTickRecord>,
    ) -> Result<PopulationSocialLoopSummary, GameAppShellError> {
        let schedule_order = self
            .creatures
            .iter()
            .map(|creature| creature.organism_id)
            .collect::<Vec<_>>();
        let statuses = self
            .creatures
            .iter()
            .map(|creature| {
                let object = self
                    .harness
                    .world()
                    .entity(creature.stable_id)
                    .cloned()
                    .ok_or(GameAppShellError::VisibleWorldMismatch {
                        message: "population stable creature ID must remain in the world",
                    })?;
                let target = creature
                    .last_summary
                    .as_ref()
                    .and_then(|summary| summary.target_entity);
                let target_position = target.and_then(|target_id| {
                    self.harness
                        .world()
                        .entity(target_id)
                        .map(|target| target.position)
                });
                let visual = creature_visual_snapshot_from_parts(
                    creature.organism_id,
                    creature.stable_id,
                    object.position,
                    target,
                    target_position,
                    creature.mind.homeostasis(),
                    creature.mind.sleep_state().phase,
                    creature
                        .last_summary
                        .as_ref()
                        .and_then(|summary| summary.selected_action_kind),
                )?;
                let status = PopulationCreatureStatus {
                    organism_id: creature.organism_id,
                    stable_id: creature.stable_id,
                    label: creature.label.clone(),
                    position: object.position,
                    last_action_kind: creature
                        .last_summary
                        .as_ref()
                        .and_then(|summary| summary.selected_action_kind),
                    social_agents_seen: creature.last_social_agents_seen,
                    heard_tokens: creature.last_heard_tokens,
                    visual,
                };
                status.validate()?;
                Ok(status)
            })
            .collect::<Result<Vec<_>, GameAppShellError>>()?;
        let metrics = PopulationPerformanceMetrics {
            creature_count: self.creatures.len(),
            population_cap: self.population_cap,
            scheduler_steps: records.len(),
            sealed_patch_count: self.harness.telemetry().sealed_patches.len(),
            packed_record_count: self.harness.telemetry().packed_records.len(),
            social_context_samples: records.iter().map(|record| record.social_agents_seen).sum(),
            vocal_tokens_heard: records.iter().map(|record| record.heard_tokens).sum(),
            collision_feedback_count: records
                .iter()
                .filter(|record| record.contacted_agents > 0)
                .count(),
            world_object_count: self.harness.world().stable_signature().len(),
        };
        let summary = PopulationSocialLoopSummary {
            schema: G08_POPULATION_SOCIAL_SCHEMA,
            schema_version: G08_POPULATION_SOCIAL_SCHEMA_VERSION,
            seed,
            creature_count: self.creatures.len(),
            population_cap: self.population_cap,
            schedule_order,
            tick_records: records,
            creature_status: statuses,
            metrics,
            world_signature: self.harness.world().stable_signature(),
        };
        summary.validate()?;
        Ok(summary)
    }
}

fn distance_between_entities(world: &HeadlessWorld, a: WorldEntityId, b: WorldEntityId) -> f32 {
    let Some(a) = world.entity(a) else {
        return 0.0;
    };
    let Some(b) = world.entity(b) else {
        return 0.0;
    };
    let dx = a.position.x - b.position.x;
    let dy = a.position.y - b.position.y;
    let dz = a.position.z - b.position.z;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub fn run_population_social_loop_smoke() -> Result<PopulationSocialLoopSummary, GameAppShellError>
{
    let config = PopulationLoopConfig::two_creature_smoke()?;
    let seed = config.seed;
    let rounds = config.rounds;
    let mut live = PopulationLiveLoop::from_config(config)?;
    live.run_rounds(rounds, seed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreatureLifeStage {
    Hatchling,
    Juvenile,
    Adult,
    Elder,
    Dead,
}

impl CreatureLifeStage {
    pub fn from_age(age_ticks: Tick, alive: bool) -> Self {
        if !alive {
            return Self::Dead;
        }
        match age_ticks.raw() {
            0..=1 => Self::Hatchling,
            2..=3 => Self::Juvenile,
            4..=8 => Self::Adult,
            _ => Self::Elder,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Hatchling => "hatchling",
            Self::Juvenile => "juvenile",
            Self::Adult => "adult",
            Self::Elder => "elder",
            Self::Dead => "dead",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleEventKind {
    Aged,
    Birth,
    Death,
    ReproductionBlocked,
}

impl LifecycleEventKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Aged => "aged",
            Self::Birth => "birth",
            Self::Death => "death",
            Self::ReproductionBlocked => "reproduction-blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleCreatureConfig {
    pub organism_id: OrganismId,
    pub brain_tier: BrainScaleTier,
    pub label: &'static str,
    pub position: Vec3f,
    pub social_affinity: f32,
    pub homeostasis: HomeostaticSnapshot,
    pub initial_age_ticks: Tick,
    pub generation: u32,
}

impl LifecycleCreatureConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if self.label.is_empty() || self.generation > 64 {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.position.validate()?;
        if !self.social_affinity.is_finite() || !(-1.0..=1.0).contains(&self.social_affinity) {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.homeostasis.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleLoopConfig {
    pub seed: u64,
    pub lineage_id: LineageId,
    pub population_cap: usize,
    pub creatures: Vec<LifecycleCreatureConfig>,
    pub generated_weight_asset_id: Option<String>,
    pub logging_enabled: bool,
}

impl LifecycleLoopConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.lineage_id.validate()?;
        if self.population_cap < 2 || self.population_cap > G09_MAX_LIFECYCLE_POPULATION_CAP {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.creatures.len() < 2 || self.creatures.len() > self.population_cap {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let mut ids = Vec::with_capacity(self.creatures.len());
        let mut labels = Vec::with_capacity(self.creatures.len());
        for creature in &self.creatures {
            creature.validate()?;
            ids.push(creature.organism_id.raw());
            labels.push(creature.label);
        }
        ids.sort_unstable();
        ids.dedup();
        labels.sort_unstable();
        labels.dedup();
        if ids.len() != self.creatures.len() || labels.len() != self.creatures.len() {
            return Err(ScaffoldContractError::InvalidId);
        }
        if self
            .generated_weight_asset_id
            .as_ref()
            .is_some_and(|asset| asset.is_empty() || asset.len() > 96)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn lineage_smoke() -> Result<Self, ScaffoldContractError> {
        let mut alpha = HomeostaticSnapshot::baseline(Tick::ZERO);
        alpha.drives.brain_atp = 0.84;
        alpha.drives.reproductive_drive = 0.82;
        alpha.drives.loneliness = 0.38;
        alpha.validate_contract()?;

        let mut beta = HomeostaticSnapshot::baseline(Tick::ZERO);
        beta.drives.brain_atp = 0.80;
        beta.drives.reproductive_drive = 0.78;
        beta.drives.loneliness = 0.36;
        beta.validate_contract()?;

        let mut elder = HomeostaticSnapshot::baseline(Tick::ZERO);
        elder.drives.brain_atp = 0.03;
        elder.drives.reproductive_drive = 0.05;
        elder.drives.fatigue = 0.92;
        elder.validate_contract()?;

        let config = Self {
            seed: 9_090,
            lineage_id: LineageId(9_090),
            population_cap: 4,
            logging_enabled: true,
            generated_weight_asset_id: Some("g09-tiny-birth-weight-asset".to_string()),
            creatures: vec![
                LifecycleCreatureConfig {
                    organism_id: OrganismId(901),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "lineage-alpha",
                    position: Vec3f::ZERO,
                    social_affinity: 0.66,
                    homeostasis: alpha,
                    initial_age_ticks: Tick::new(5),
                    generation: 0,
                },
                LifecycleCreatureConfig {
                    organism_id: OrganismId(902),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "lineage-beta",
                    position: Vec3f::new(1.0, 0.0, 0.0),
                    social_affinity: 0.64,
                    homeostasis: beta,
                    initial_age_ticks: Tick::new(5),
                    generation: 0,
                },
                LifecycleCreatureConfig {
                    organism_id: OrganismId(903),
                    brain_tier: BrainScaleTier::Nano512,
                    label: "lineage-elder",
                    position: Vec3f::new(-1.0, 0.0, 0.0),
                    social_affinity: 0.10,
                    homeostasis: elder,
                    initial_age_ticks: Tick::new(10),
                    generation: 0,
                },
            ],
        };
        config.validate()?;
        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleLineageRecord {
    pub offspring_genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: LineageId,
    pub generation: u32,
    pub birth_tick: Tick,
    pub birth_weight_asset_id: Option<String>,
    pub lamarckian_enabled: bool,
    pub inherited_lifetime_state: bool,
}

impl LifecycleLineageRecord {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.offspring_genome_id.validate()?;
        self.lineage_id.validate()?;
        if self.parent_genome_ids.len() != 2
            || self.generation == 0
            || self.lamarckian_enabled
            || self.inherited_lifetime_state
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for parent in &self.parent_genome_ids {
            parent.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.offspring_genome_id.raw(),
            self.parent_genome_ids
                .iter()
                .map(|id| id.raw().to_string())
                .collect::<Vec<_>>()
                .join("+"),
            self.lineage_id.raw(),
            self.generation,
            self.birth_tick.raw(),
            self.birth_weight_asset_id.as_deref().unwrap_or("none"),
            self.inherited_lifetime_state
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleCreatureRecord {
    pub organism_id: OrganismId,
    pub stable_id: WorldEntityId,
    pub label: String,
    pub genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: LineageId,
    pub generation: u32,
    pub age_ticks: Tick,
    pub life_stage: CreatureLifeStage,
    pub alive: bool,
    pub brain_atp: f32,
    pub reproductive_drive: f32,
    pub birth_weight_asset_id: Option<String>,
    pub genetic_prior_seed: u64,
    pub lamarckian_enabled: bool,
    pub inherited_lifetime_state: bool,
    pub death_reason: Option<String>,
}

impl LifecycleCreatureRecord {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        self.stable_id.validate()?;
        self.genome_id.validate()?;
        self.lineage_id.validate()?;
        if self.label.is_empty()
            || !self.brain_atp.is_finite()
            || !self.reproductive_drive.is_finite()
            || !(0.0..=1.0).contains(&self.brain_atp)
            || !(0.0..=1.0).contains(&self.reproductive_drive)
            || self.genetic_prior_seed == 0
            || self.lamarckian_enabled
            || self.inherited_lifetime_state
            || self.life_stage != CreatureLifeStage::from_age(self.age_ticks, self.alive)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for parent in &self.parent_genome_ids {
            parent.validate()?;
        }
        if !self.alive && self.death_reason.is_none() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{:.2}:{:.2}:{}:{}",
            self.organism_id.raw(),
            self.stable_id.raw(),
            self.label,
            self.genome_id.raw(),
            self.lineage_id.raw(),
            self.generation,
            self.age_ticks.raw(),
            self.life_stage.label(),
            self.brain_atp,
            self.reproductive_drive,
            self.alive,
            self.birth_weight_asset_id.as_deref().unwrap_or("none")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleEventRecord {
    pub kind: LifecycleEventKind,
    pub tick: Tick,
    pub organism_id: OrganismId,
    pub stable_id: Option<WorldEntityId>,
    pub message: String,
}

impl LifecycleEventRecord {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.organism_id.validate()?;
        if let Some(stable_id) = self.stable_id {
            stable_id.validate()?;
        }
        if self.message.is_empty() || self.message.len() > 160 {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{:?}:{}",
            self.kind.label(),
            self.tick.raw(),
            self.organism_id.raw(),
            self.stable_id.map(|id| id.raw()),
            self.message
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LifecycleSaveState {
    pub schema: String,
    pub schema_version: u16,
    pub seed: u64,
    pub population_cap: usize,
    pub selected_stable_id: Option<WorldEntityId>,
    pub records: Vec<LifecycleCreatureRecord>,
    pub lineages: Vec<LifecycleLineageRecord>,
}

impl LifecycleSaveState {
    pub fn from_summary(summary: &LifecycleLineageSummary) -> Result<Self, ScaffoldContractError> {
        summary.validate()?;
        Ok(Self {
            schema: G09_LIFECYCLE_SCHEMA.to_string(),
            schema_version: G09_LIFECYCLE_SCHEMA_VERSION,
            seed: summary.seed,
            population_cap: summary.population_cap,
            selected_stable_id: summary.selected_stable_id,
            records: summary.creatures.clone(),
            lineages: summary.lineage_records.clone(),
        })
    }

    pub fn to_json_string_pretty(&self) -> Result<String, GameAppShellError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json_str(json: &str) -> Result<Self, GameAppShellError> {
        let state = serde_json::from_str::<Self>(json)?;
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G09_LIFECYCLE_SCHEMA
            || self.schema_version != G09_LIFECYCLE_SCHEMA_VERSION
            || self.population_cap < 2
            || self.population_cap > G09_MAX_LIFECYCLE_POPULATION_CAP
            || self.records.is_empty()
            || self.records.len() > self.population_cap
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for record in &self.records {
            record.validate()?;
        }
        for lineage in &self.lineages {
            lineage.validate()?;
        }
        if let Some(selected) = self.selected_stable_id {
            selected.validate()?;
            if !self
                .records
                .iter()
                .any(|record| record.alive && record.stable_id == selected)
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.population_cap,
            self.selected_stable_id
                .map(|id| id.raw().to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.records
                .iter()
                .map(LifecycleCreatureRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleInspectorLine {
    pub stable_id: WorldEntityId,
    pub organism_id: OrganismId,
    pub label: String,
    pub life_stage: CreatureLifeStage,
    pub lineage_label: String,
    pub selected: bool,
}

impl LifecycleInspectorLine {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.stable_id.validate()?;
        self.organism_id.validate()?;
        if self.label.is_empty() || self.lineage_label.is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.stable_id.raw(),
            self.organism_id.raw(),
            self.label,
            self.life_stage.label(),
            self.lineage_label,
            self.selected
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleMetrics {
    pub initial_population: usize,
    pub living_population: usize,
    pub population_cap: usize,
    pub births: usize,
    pub deaths: usize,
    pub reproduction_blocked_count: usize,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
}

impl LifecycleMetrics {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.initial_population < 2
            || self.living_population > self.population_cap
            || self.population_cap > G09_MAX_LIFECYCLE_POPULATION_CAP
            || self.births > self.population_cap
            || self.sealed_patch_count < self.initial_population
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleLineageSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub population_cap: usize,
    pub selected_stable_id: Option<WorldEntityId>,
    pub creatures: Vec<LifecycleCreatureRecord>,
    pub lineage_records: Vec<LifecycleLineageRecord>,
    pub events: Vec<LifecycleEventRecord>,
    pub inspector_lines: Vec<LifecycleInspectorLine>,
    pub save_roundtrip_signature: String,
    pub world_signature: Vec<String>,
    pub metrics: LifecycleMetrics,
}

impl LifecycleLineageSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G09_LIFECYCLE_SCHEMA
            || self.schema_version != G09_LIFECYCLE_SCHEMA_VERSION
            || self.population_cap < 2
            || self.population_cap > G09_MAX_LIFECYCLE_POPULATION_CAP
            || self.creatures.is_empty()
            || self.inspector_lines.is_empty()
            || self.world_signature.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.metrics.validate()?;
        for creature in &self.creatures {
            creature.validate()?;
        }
        for lineage in &self.lineage_records {
            lineage.validate()?;
        }
        for event in &self.events {
            event.validate()?;
        }
        for line in &self.inspector_lines {
            line.validate()?;
        }
        if let Some(selected) = self.selected_stable_id {
            selected.validate()?;
            if !self
                .creatures
                .iter()
                .any(|record| record.alive && record.stable_id == selected)
            {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        if self
            .creatures
            .iter()
            .any(|record| record.lamarckian_enabled || record.inherited_lifetime_state)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let save = LifecycleSaveState {
            schema: G09_LIFECYCLE_SCHEMA.to_string(),
            schema_version: G09_LIFECYCLE_SCHEMA_VERSION,
            seed: self.seed,
            population_cap: self.population_cap,
            selected_stable_id: self.selected_stable_id,
            records: self.creatures.clone(),
            lineages: self.lineage_records.clone(),
        };
        save.validate()?;
        if save.signature_line() != self.save_roundtrip_signature {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.population_cap,
            self.selected_stable_id
                .map(|id| id.raw().to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.creatures
                .iter()
                .map(LifecycleCreatureRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.lineage_records
                .iter()
                .map(LifecycleLineageRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.events
                .iter()
                .map(LifecycleEventRecord::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.save_roundtrip_signature
        )
    }
}

#[derive(Debug)]
struct LifecycleCreatureRuntime {
    organism_id: OrganismId,
    label: String,
    stable_id: WorldEntityId,
    mind: CreatureMind,
    genome: BrainGenome,
    generation: u32,
    age_ticks: Tick,
    alive: bool,
    birth_weight_asset_id: Option<String>,
    death_reason: Option<String>,
}

#[derive(Debug)]
pub struct LifecycleLiveLoop {
    seed: u64,
    lineage_id: LineageId,
    population_cap: usize,
    generated_weight_asset_id: Option<String>,
    logging_enabled: bool,
    selected_stable_id: Option<WorldEntityId>,
    initial_population: usize,
    harness: HeadlessBrainHarness,
    creatures: Vec<LifecycleCreatureRuntime>,
    lineage_records: Vec<LifecycleLineageRecord>,
    events: Vec<LifecycleEventRecord>,
    reproduction_blocked_count: usize,
}

impl LifecycleLiveLoop {
    pub fn from_config(config: LifecycleLoopConfig) -> Result<Self, GameAppShellError> {
        config.validate()?;
        let mut builder = HeadlessScenarioBuilder::new(config.seed)
            .food("lineage-berry", Vec3f::new(2.0, 0.0, 0.0), 0.52)
            .obstacle("lineage-stone", Vec3f::new(-2.0, 0.0, 0.0), 0.55);
        for creature in &config.creatures {
            builder = builder.social_agent(
                creature.label,
                creature.organism_id,
                creature.position,
                creature.social_affinity,
            );
        }
        let world = builder.build()?;
        let mut creatures = Vec::with_capacity(config.population_cap);
        for creature in &config.creatures {
            let stable_id =
                world
                    .entity_id(creature.label)
                    .ok_or(GameAppShellError::VisibleWorldMismatch {
                        message: "G09 lifecycle creature label must map to a stable world ID",
                    })?;
            let species_seed = config.seed.saturating_add(creature.organism_id.raw());
            let mut genome =
                BrainGenome::scaffold(species_seed, creature.brain_tier.default_class_id());
            genome.lineage_id = Some(config.lineage_id);
            genome.validate_contract()?;
            let mut mind = CreatureMind::scaffold(
                creature.organism_id,
                creature.brain_tier,
                species_seed,
                Tick::ZERO,
            )?;
            *mind.homeostasis_mut() = creature.homeostasis;
            mind.homeostasis().validate_contract()?;
            creatures.push(LifecycleCreatureRuntime {
                organism_id: creature.organism_id,
                label: creature.label.to_string(),
                stable_id,
                mind,
                genome,
                generation: creature.generation,
                age_ticks: creature.initial_age_ticks,
                alive: true,
                birth_weight_asset_id: None,
                death_reason: None,
            });
        }
        creatures.sort_by_key(|creature| creature.organism_id.raw());
        Ok(Self {
            seed: config.seed,
            lineage_id: config.lineage_id,
            population_cap: config.population_cap,
            generated_weight_asset_id: config.generated_weight_asset_id,
            logging_enabled: config.logging_enabled,
            selected_stable_id: creatures.first().map(|creature| creature.stable_id),
            initial_population: creatures.len(),
            harness: HeadlessBrainHarness::new(world),
            creatures,
            lineage_records: Vec::new(),
            events: Vec::new(),
            reproduction_blocked_count: 0,
        })
    }

    pub fn run_lifecycle_once(&mut self) -> Result<LifecycleLineageSummary, GameAppShellError> {
        self.tick_living_creatures()?;
        self.age_living_creatures()?;
        self.remove_dead_creatures()?;
        self.try_reproduce()?;
        self.build_summary()
    }

    fn tick_living_creatures(&mut self) -> Result<(), GameAppShellError> {
        for index in 0..self.creatures.len() {
            if !self.creatures[index].alive {
                continue;
            }
            let tick_before = self.creatures[index].mind.current_tick();
            let input = BrainTickInput::new(
                tick_before,
                vec![proposal(
                    ActionKind::Idle.canonical_id(),
                    ActionKind::Idle,
                    None,
                    None,
                    0.80,
                    0.90,
                    0.0,
                )?],
            )
            .with_pack_experience(self.logging_enabled)
            .with_action_duration(DurationTicks::new(1));
            let tick = self
                .harness
                .tick_mind(&mut self.creatures[index].mind, input);
            if tick.brain.experience_patch.is_none() {
                return Err(GameAppShellError::Core(
                    ScaffoldContractError::MissingPhaseData,
                ));
            }
        }
        Ok(())
    }

    fn age_living_creatures(&mut self) -> Result<(), ScaffoldContractError> {
        for creature in &mut self.creatures {
            if !creature.alive {
                continue;
            }
            creature.age_ticks = Tick::new(creature.age_ticks.raw().saturating_add(1));
            self.events.push(LifecycleEventRecord {
                kind: LifecycleEventKind::Aged,
                tick: creature.mind.current_tick(),
                organism_id: creature.organism_id,
                stable_id: Some(creature.stable_id),
                message: format!(
                    "{} is now {}",
                    creature.label,
                    CreatureLifeStage::from_age(creature.age_ticks, true).label()
                ),
            });
        }
        Ok(())
    }

    fn remove_dead_creatures(&mut self) -> Result<(), GameAppShellError> {
        for index in 0..self.creatures.len() {
            if !self.creatures[index].alive {
                continue;
            }
            let brain_atp = self.creatures[index].mind.homeostasis().drives.brain_atp;
            if brain_atp > 0.05 && self.creatures[index].age_ticks.raw() < 12 {
                continue;
            }
            let stable_id = self.creatures[index].stable_id;
            let organism_id = self.creatures[index].organism_id;
            let label = self.creatures[index].label.clone();
            self.harness.remove_agent_entity(stable_id)?;
            self.creatures[index].alive = false;
            self.creatures[index].death_reason = Some(if brain_atp <= 0.05 {
                "energy-failure".to_string()
            } else {
                "old-age".to_string()
            });
            self.events.push(LifecycleEventRecord {
                kind: LifecycleEventKind::Death,
                tick: self.creatures[index].mind.current_tick(),
                organism_id,
                stable_id: Some(stable_id),
                message: format!(
                    "{} removed by {}",
                    label,
                    self.creatures[index]
                        .death_reason
                        .as_deref()
                        .unwrap_or("unknown")
                ),
            });
        }
        self.selected_stable_id = self.selected_stable_id.and_then(|selected| {
            self.creatures
                .iter()
                .any(|creature| creature.alive && creature.stable_id == selected)
                .then_some(selected)
        });
        if self.selected_stable_id.is_none() {
            self.selected_stable_id = self
                .creatures
                .iter()
                .filter(|creature| creature.alive)
                .min_by_key(|creature| creature.organism_id.raw())
                .map(|creature| creature.stable_id);
        }
        Ok(())
    }

    fn try_reproduce(&mut self) -> Result<(), GameAppShellError> {
        let living = self
            .creatures
            .iter()
            .filter(|creature| creature.alive)
            .count();
        if living >= self.population_cap {
            self.reproduction_blocked_count = self.reproduction_blocked_count.saturating_add(1);
            let organism_id = self
                .creatures
                .iter()
                .find(|creature| creature.alive)
                .map(|creature| creature.organism_id)
                .unwrap_or(OrganismId(1));
            self.events.push(LifecycleEventRecord {
                kind: LifecycleEventKind::ReproductionBlocked,
                tick: self.harness.world().tick(),
                organism_id,
                stable_id: self.selected_stable_id,
                message: "population cap reached".to_string(),
            });
            return Ok(());
        }

        let parents = self
            .creatures
            .iter()
            .enumerate()
            .filter(|(_, creature)| {
                creature.alive
                    && CreatureLifeStage::from_age(creature.age_ticks, true)
                        == CreatureLifeStage::Adult
                    && creature.mind.homeostasis().drives.reproductive_drive >= 0.70
                    && creature.mind.homeostasis().drives.brain_atp >= 0.25
            })
            .take(2)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        if parents.len() < 2 {
            self.reproduction_blocked_count = self.reproduction_blocked_count.saturating_add(1);
            let organism_id = self
                .creatures
                .iter()
                .find(|creature| creature.alive)
                .map(|creature| creature.organism_id)
                .unwrap_or(OrganismId(1));
            self.events.push(LifecycleEventRecord {
                kind: LifecycleEventKind::ReproductionBlocked,
                tick: self.harness.world().tick(),
                organism_id,
                stable_id: self.selected_stable_id,
                message: "no eligible adult pair".to_string(),
            });
            return Ok(());
        }

        let parent_a = parents[0];
        let parent_b = parents[1];
        for parent in [parent_a, parent_b] {
            let homeostasis = self.creatures[parent].mind.homeostasis_mut();
            homeostasis.drives.brain_atp = (homeostasis.drives.brain_atp - 0.12).max(0.0);
            homeostasis.drives.reproductive_drive =
                (homeostasis.drives.reproductive_drive - 0.35).max(0.0);
            homeostasis.validate_contract()?;
        }

        let child_number = self.creatures.len() as u64 + 1;
        let child_organism = OrganismId(self.seed.saturating_add(900).saturating_add(child_number));
        let child_label = format!("lineage-child-{}", child_organism.raw());
        let parent_position_a = self
            .harness
            .world()
            .entity(self.creatures[parent_a].stable_id)
            .map(|object| object.position)
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "parent A stable ID must exist before reproduction",
            })?;
        let parent_position_b = self
            .harness
            .world()
            .entity(self.creatures[parent_b].stable_id)
            .map(|object| object.position)
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "parent B stable ID must exist before reproduction",
            })?;
        let child_position = Vec3f::new(
            (parent_position_a.x + parent_position_b.x) * 0.5,
            (parent_position_a.y + parent_position_b.y) * 0.5,
            (parent_position_a.z + parent_position_b.z) * 0.5 + 0.5,
        );
        let stable_id =
            self.harness
                .spawn_social_agent(&child_label, child_organism, child_position, 0.50)?;
        let child_seed = deterministic_child_seed(
            self.seed,
            self.creatures[parent_a].genome.id,
            self.creatures[parent_b].genome.id,
            child_organism,
        );
        let mut genome =
            BrainGenome::scaffold(child_seed, BrainScaleTier::Nano512.default_class_id());
        genome.parent_genome_ids = vec![
            self.creatures[parent_a].genome.id,
            self.creatures[parent_b].genome.id,
        ];
        genome.lineage_id = Some(self.lineage_id);
        genome.validate_contract()?;

        let mut mind = CreatureMind::scaffold(
            child_organism,
            BrainScaleTier::Nano512,
            child_seed,
            Tick::ZERO,
        )?;
        let mut child_homeostasis = HomeostaticSnapshot::baseline(Tick::ZERO);
        child_homeostasis.drives.reproductive_drive = 0.05;
        child_homeostasis.drives.brain_atp = 0.62;
        child_homeostasis.validate_contract()?;
        *mind.homeostasis_mut() = child_homeostasis;

        let generation = self.creatures[parent_a]
            .generation
            .max(self.creatures[parent_b].generation)
            + 1;
        let record = LifecycleLineageRecord {
            offspring_genome_id: genome.id,
            parent_genome_ids: genome.parent_genome_ids.clone(),
            lineage_id: self.lineage_id,
            generation,
            birth_tick: self.harness.world().tick(),
            birth_weight_asset_id: self.generated_weight_asset_id.clone(),
            lamarckian_enabled: genome.inheritance.lamarckian_weights_enabled,
            inherited_lifetime_state: genome.inheritance.inherit_lifetime_consolidation,
        };
        record.validate()?;
        self.lineage_records.push(record);
        self.events.push(LifecycleEventRecord {
            kind: LifecycleEventKind::Birth,
            tick: self.harness.world().tick(),
            organism_id: child_organism,
            stable_id: Some(stable_id),
            message: format!(
                "{} born from {}+{}",
                child_label,
                self.creatures[parent_a].organism_id.raw(),
                self.creatures[parent_b].organism_id.raw()
            ),
        });
        self.creatures.push(LifecycleCreatureRuntime {
            organism_id: child_organism,
            label: child_label,
            stable_id,
            mind,
            genome,
            generation,
            age_ticks: Tick::ZERO,
            alive: true,
            birth_weight_asset_id: self.generated_weight_asset_id.clone(),
            death_reason: None,
        });
        self.creatures
            .sort_by_key(|creature| creature.organism_id.raw());
        Ok(())
    }

    fn build_summary(&self) -> Result<LifecycleLineageSummary, GameAppShellError> {
        let creatures = self
            .creatures
            .iter()
            .map(|creature| {
                let homeostasis = creature.mind.homeostasis();
                let record = LifecycleCreatureRecord {
                    organism_id: creature.organism_id,
                    stable_id: creature.stable_id,
                    label: creature.label.clone(),
                    genome_id: creature.genome.id,
                    parent_genome_ids: creature.genome.parent_genome_ids.clone(),
                    lineage_id: creature
                        .genome
                        .lineage_id
                        .ok_or(GameAppShellError::Core(ScaffoldContractError::InvalidId))?,
                    generation: creature.generation,
                    age_ticks: creature.age_ticks,
                    life_stage: CreatureLifeStage::from_age(creature.age_ticks, creature.alive),
                    alive: creature.alive,
                    brain_atp: homeostasis.drives.brain_atp,
                    reproductive_drive: homeostasis.drives.reproductive_drive,
                    birth_weight_asset_id: creature.birth_weight_asset_id.clone(),
                    genetic_prior_seed: creature.genome.genetic_prior_seed,
                    lamarckian_enabled: creature.genome.inheritance.lamarckian_weights_enabled,
                    inherited_lifetime_state: creature
                        .genome
                        .inheritance
                        .inherit_lifetime_consolidation,
                    death_reason: creature.death_reason.clone(),
                };
                record.validate()?;
                Ok(record)
            })
            .collect::<Result<Vec<_>, GameAppShellError>>()?;
        let inspector_lines = creatures
            .iter()
            .filter(|record| record.alive)
            .map(|record| {
                let line = LifecycleInspectorLine {
                    stable_id: record.stable_id,
                    organism_id: record.organism_id,
                    label: record.label.clone(),
                    life_stage: record.life_stage,
                    lineage_label: format!(
                        "lineage-{}-gen{}",
                        record.lineage_id.raw(),
                        record.generation
                    ),
                    selected: self.selected_stable_id == Some(record.stable_id),
                };
                line.validate()?;
                Ok(line)
            })
            .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
        let save_preview = LifecycleSaveState {
            schema: G09_LIFECYCLE_SCHEMA.to_string(),
            schema_version: G09_LIFECYCLE_SCHEMA_VERSION,
            seed: self.seed,
            population_cap: self.population_cap,
            selected_stable_id: self.selected_stable_id,
            records: creatures.clone(),
            lineages: self.lineage_records.clone(),
        };
        let json = save_preview.to_json_string_pretty()?;
        let save_roundtrip = LifecycleSaveState::from_json_str(&json)?;
        let summary = LifecycleLineageSummary {
            schema: G09_LIFECYCLE_SCHEMA,
            schema_version: G09_LIFECYCLE_SCHEMA_VERSION,
            seed: self.seed,
            population_cap: self.population_cap,
            selected_stable_id: self.selected_stable_id,
            creatures,
            lineage_records: self.lineage_records.clone(),
            events: self.events.clone(),
            inspector_lines,
            save_roundtrip_signature: save_roundtrip.signature_line(),
            world_signature: self.harness.world().stable_signature(),
            metrics: LifecycleMetrics {
                initial_population: self.initial_population,
                living_population: self
                    .creatures
                    .iter()
                    .filter(|creature| creature.alive)
                    .count(),
                population_cap: self.population_cap,
                births: self.lineage_records.len(),
                deaths: self
                    .creatures
                    .iter()
                    .filter(|creature| !creature.alive)
                    .count(),
                reproduction_blocked_count: self.reproduction_blocked_count,
                sealed_patch_count: self.harness.telemetry().sealed_patches.len(),
                packed_record_count: self.harness.telemetry().packed_records.len(),
            },
        };
        summary.validate()?;
        Ok(summary)
    }
}

fn deterministic_child_seed(
    seed: u64,
    parent_a: GenomeId,
    parent_b: GenomeId,
    child: OrganismId,
) -> u64 {
    let mut value = seed
        ^ parent_a.raw().rotate_left(17)
        ^ parent_b.raw().rotate_right(11)
        ^ child.raw().wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 33;
    value = value.wrapping_mul(0xff51_afd7_ed55_8ccd);
    value ^= value >> 29;
    value.max(1)
}

pub fn run_lifecycle_lineage_smoke() -> Result<LifecycleLineageSummary, GameAppShellError> {
    let config = LifecycleLoopConfig::lineage_smoke()?;
    let mut live = LifecycleLiveLoop::from_config(config)?;
    live.run_lifecycle_once()
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolModeConfig {
    pub seed: u64,
    pub curriculum_id: String,
    pub learner_id: OrganismId,
    pub teacher_id: OrganismId,
    pub lesson_limit: usize,
}

impl SchoolModeConfig {
    pub fn grounded_smoke() -> Self {
        Self {
            seed: 10_010,
            curriculum_id: "g10-grounded-object-food".to_string(),
            learner_id: OrganismId(10_001),
            teacher_id: OrganismId(10_900),
            lesson_limit: 1,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        self.learner_id.validate()?;
        self.teacher_id.validate()?;
        if self.seed == 0
            || self.curriculum_id.is_empty()
            || self.curriculum_id.len() > 96
            || self.lesson_limit == 0
            || self.lesson_limit > 6
            || self.learner_id == self.teacher_id
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolCuePresentation {
    pub lesson_id: LessonId,
    pub input_kind: TeacherInputKind,
    pub channel: TeacherPerceptionChannel,
    pub token_id: Option<u32>,
    pub gesture_id: Option<u32>,
    pub object_entity: Option<WorldEntityId>,
    pub cue_entity: Option<WorldEntityId>,
    pub salience: f32,
    pub perception_only: bool,
    pub direct_motor_bypass: bool,
    pub label: String,
}

impl SchoolCuePresentation {
    pub fn from_event(
        event: TeacherPerceptualEvent,
        cue_entity: Option<WorldEntityId>,
        label: impl Into<String>,
    ) -> Result<Self, ScaffoldContractError> {
        if let Some(entity) = cue_entity {
            entity.validate()?;
        }
        if let Some(object) = event.object_entity {
            object.validate()?;
        }
        let cue = Self {
            lesson_id: event.lesson_id,
            input_kind: event.input_kind,
            channel: event.channel,
            token_id: event.token_id,
            gesture_id: event.gesture_id,
            object_entity: event.object_entity,
            cue_entity,
            salience: event.salience.raw(),
            perception_only: event.input_kind.is_perceptual()
                && !event.hidden_vector_injection_allowed(),
            direct_motor_bypass: event.direct_motor_bypass(),
            label: label.into(),
        };
        cue.validate()?;
        Ok(cue)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.lesson_id.raw() == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        if let Some(token_id) = self.token_id {
            if token_id == 0 {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        if let Some(gesture_id) = self.gesture_id {
            if gesture_id == 0 {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        if let Some(entity) = self.object_entity {
            entity.validate()?;
        }
        if let Some(entity) = self.cue_entity {
            entity.validate()?;
        }
        NormalizedScalar::new(self.salience)?;
        if self.label.is_empty() || !self.perception_only || self.direct_motor_bypass {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:.2}:{}:{}",
            self.lesson_id.raw(),
            self.input_kind,
            self.channel,
            self.token_id,
            self.gesture_id,
            self.object_entity.map(|id| id.raw()),
            self.cue_entity.map(|id| id.raw()),
            self.salience,
            self.perception_only,
            self.label
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolLessonPanel {
    pub curriculum_id: String,
    pub active_lesson_id: LessonId,
    pub step_kind: CurriculumStepKind,
    pub total_steps: usize,
    pub completed_steps: usize,
    pub cue_count: usize,
    pub response_channels: Vec<TeacherLessonResponseChannel>,
    pub status_line: String,
}

impl SchoolLessonPanel {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.active_lesson_id.raw() == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        if self.curriculum_id.is_empty()
            || self.total_steps == 0
            || self.completed_steps > self.total_steps
            || self.cue_count == 0
            || self.response_channels.is_empty()
            || self.status_line.is_empty()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{:?}:{}:{}:{}:{}",
            self.curriculum_id,
            self.active_lesson_id.raw(),
            self.step_kind,
            self.completed_steps,
            self.total_steps,
            self.cue_count,
            self.status_line
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolVerifierPanel {
    pub passed: bool,
    pub observed_checks: Vec<String>,
    pub failed_checks: Vec<String>,
    pub sealed_patch_count: usize,
    pub verifier_message: String,
}

impl SchoolVerifierPanel {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.sealed_patch_count == 0
            || self.observed_checks.is_empty()
            || self.verifier_message.is_empty()
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.passed,
            self.sealed_patch_count,
            self.observed_checks.join("+"),
            self.failed_checks.join("+")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchoolModeSaveState {
    pub schema: String,
    pub schema_version: u16,
    pub seed: u64,
    pub curriculum_id: String,
    pub active_lesson_id: u64,
    pub completed_steps: usize,
    pub teacher_avatar_stable_id: WorldEntityId,
    pub cue_entity_ids: Vec<WorldEntityId>,
    pub verifier_passed: bool,
    pub p34_school: SchoolSaveState,
}

impl SchoolModeSaveState {
    pub fn from_summary(summary: &SchoolModeSummary) -> Result<Self, ScaffoldContractError> {
        summary.validate()?;
        Ok(Self {
            schema: G10_SCHOOL_MODE_SCHEMA.to_string(),
            schema_version: G10_SCHOOL_MODE_SCHEMA_VERSION,
            seed: summary.seed,
            curriculum_id: summary.lesson_panel.curriculum_id.clone(),
            active_lesson_id: summary.lesson_panel.active_lesson_id.raw(),
            completed_steps: summary.lesson_panel.completed_steps,
            teacher_avatar_stable_id: summary.teacher_avatar_stable_id,
            cue_entity_ids: summary
                .cues
                .iter()
                .filter_map(|cue| cue.cue_entity)
                .collect(),
            verifier_passed: summary.verifier_panel.passed,
            p34_school: summary.p34_school.clone(),
        })
    }

    pub fn to_json_string_pretty(&self) -> Result<String, GameAppShellError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json_str(json: &str) -> Result<Self, GameAppShellError> {
        let state = serde_json::from_str::<Self>(json)?;
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G10_SCHOOL_MODE_SCHEMA
            || self.schema_version != G10_SCHOOL_MODE_SCHEMA_VERSION
            || self.seed == 0
            || self.curriculum_id.is_empty()
            || self.active_lesson_id == 0
            || self.completed_steps > 6
            || self.cue_entity_ids.is_empty()
            || !self.p34_school.enabled
            || self.p34_school.active_curriculum_id.as_ref() != Some(&self.curriculum_id)
            || self.p34_school.teacher_private_state_saved
            || self.p34_school.schema_version != TEACHER_SCHOOL_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.teacher_avatar_stable_id.validate()?;
        for id in &self.cue_entity_ids {
            id.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.curriculum_id,
            self.active_lesson_id,
            self.completed_steps,
            self.teacher_avatar_stable_id.raw(),
            self.cue_entity_ids
                .iter()
                .map(|id| id.raw().to_string())
                .collect::<Vec<_>>()
                .join("+")
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SchoolModeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub seed: u64,
    pub teacher_avatar_stable_id: WorldEntityId,
    pub learner_stable_id: WorldEntityId,
    pub lesson_panel: SchoolLessonPanel,
    pub cues: Vec<SchoolCuePresentation>,
    pub verifier_panel: SchoolVerifierPanel,
    pub sensory_heard_tokens: Vec<u32>,
    pub sensory_teacher_channels: Vec<TeacherPerceptionChannel>,
    pub teacher_metadata_bypass_blocked: bool,
    pub teacher_selected_action_id: Option<ActionId>,
    pub world_signature: Vec<String>,
    pub p34_school: SchoolSaveState,
    pub save_roundtrip_signature: String,
}

impl SchoolModeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G10_SCHOOL_MODE_SCHEMA
            || self.schema_version != G10_SCHOOL_MODE_SCHEMA_VERSION
            || self.seed == 0
            || self.cues.is_empty()
            || self.world_signature.is_empty()
            || self.sensory_heard_tokens.is_empty()
            || self.sensory_teacher_channels.is_empty()
            || !self.teacher_metadata_bypass_blocked
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.teacher_avatar_stable_id.validate()?;
        self.learner_stable_id.validate()?;
        self.lesson_panel.validate()?;
        self.verifier_panel.validate()?;
        for cue in &self.cues {
            cue.validate()?;
        }
        let contract = TeacherChannelContract::grounded_default();
        if self.cues.iter().any(|cue| {
            !contract.channels.contains(&cue.channel)
                || !contract.input_kinds.contains(&cue.input_kind)
                || !cue.perception_only
                || cue.direct_motor_bypass
        }) {
            return Err(ScaffoldContractError::InvalidId);
        }
        let save = SchoolModeSaveState::from_summary_without_validate(self)?;
        save.validate()?;
        if save.signature_line() != self.save_roundtrip_signature {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.seed,
            self.teacher_avatar_stable_id.raw(),
            self.learner_stable_id.raw(),
            self.lesson_panel.signature_line(),
            self.cues
                .iter()
                .map(SchoolCuePresentation::signature_line)
                .collect::<Vec<_>>()
                .join("|"),
            self.verifier_panel.signature_line(),
            self.save_roundtrip_signature
        )
    }
}

impl SchoolModeSaveState {
    fn from_summary_without_validate(
        summary: &SchoolModeSummary,
    ) -> Result<Self, ScaffoldContractError> {
        Ok(Self {
            schema: G10_SCHOOL_MODE_SCHEMA.to_string(),
            schema_version: G10_SCHOOL_MODE_SCHEMA_VERSION,
            seed: summary.seed,
            curriculum_id: summary.lesson_panel.curriculum_id.clone(),
            active_lesson_id: summary.lesson_panel.active_lesson_id.raw(),
            completed_steps: summary.lesson_panel.completed_steps,
            teacher_avatar_stable_id: summary.teacher_avatar_stable_id,
            cue_entity_ids: summary
                .cues
                .iter()
                .filter_map(|cue| cue.cue_entity)
                .collect(),
            verifier_passed: summary.verifier_panel.passed,
            p34_school: summary.p34_school.clone(),
        })
    }
}

pub fn run_school_mode_smoke() -> Result<SchoolModeSummary, GameAppShellError> {
    let config = SchoolModeConfig::grounded_smoke();
    run_school_mode_smoke_with_config(config)
}

pub fn run_school_mode_smoke_with_config(
    config: SchoolModeConfig,
) -> Result<SchoolModeSummary, GameAppShellError> {
    config.validate()?;
    let world = HeadlessScenarioBuilder::new(config.seed)
        .agent("school-learner", config.learner_id, Vec3f::ZERO)
        .social_agent(
            "teacher-avatar",
            config.teacher_id,
            Vec3f::new(-1.25, 0.0, 0.0),
            0.75,
        )
        .food("teaching-berry", Vec3f::new(1.0, 0.0, 0.0), 0.75)
        .teacher_token(
            "teacher-word-food",
            Vec3f::new(0.45, 0.0, 0.0),
            77,
            TeacherPerceptionChannel::Hearing,
        )
        .build()?;
    let learner_stable_id =
        world
            .entity_id("school-learner")
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "G10 learner stable ID must exist",
            })?;
    let teacher_avatar_stable_id =
        world
            .entity_id("teacher-avatar")
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "G10 teacher avatar stable ID must exist",
            })?;
    let object_id =
        world
            .entity_id("teaching-berry")
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "G10 highlighted object stable ID must exist",
            })?;
    let cue_token_id =
        world
            .entity_id("teacher-word-food")
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "G10 teacher token stable ID must exist",
            })?;

    let lesson_id = LessonId::new(10_100)?;
    let curriculum = Curriculum {
        schema_version: TEACHER_SCHOOL_SCHEMA_VERSION,
        steps: vec![CurriculumStep {
            lesson_id,
            role: TeacherRole::Tutor,
            kind: CurriculumStepKind::NameObject,
            prompt_cues: vec![
                TeacherPerceptualEvent::spoken_token(lesson_id, 77),
                TeacherPerceptualEvent::object_highlight(
                    lesson_id,
                    object_id,
                    NormalizedScalar::new(0.85)?,
                ),
                TeacherPerceptualEvent::social_feedback(
                    lesson_id,
                    FeedbackPolarity::Praise,
                    Confidence::new(0.8)?,
                ),
            ],
            expected_observations: vec![
                ExpectedObservation::HeardToken(77),
                ExpectedObservation::ObjectHighlighted(object_id),
            ],
            verifier_checks: vec![
                VerifierCheck::HeardToken {
                    token_id: 77,
                    channel: TeacherPerceptionChannel::Hearing,
                },
                VerifierCheck::NoHiddenSemanticContext,
                VerifierCheck::NoDirectTeacherActionSelection,
                VerifierCheck::SelectedByArbitration,
            ],
            feedback_events: vec![TeacherPerceptualEvent::visible_reward(
                lesson_id,
                NormalizedScalar::new(0.65)?,
            )],
            response_channels: vec![TeacherLessonResponseChannel::Speech],
        }],
    };
    if curriculum.schema_version != TEACHER_SCHOOL_SCHEMA_VERSION
        || !curriculum.lesson_ids_are_unique()
    {
        return Err(GameAppShellError::Core(ScaffoldContractError::InvalidId));
    }
    let mut runner = HeadlessCurriculumRunner::new(curriculum.clone());
    let dispatch = runner.dispatch_current()?;
    let current_step = runner
        .current_step()
        .cloned()
        .ok_or(GameAppShellError::Core(ScaffoldContractError::InvalidId))?;
    let contract = TeacherChannelContract::grounded_default();
    if !dispatch
        .perception_events
        .iter()
        .all(|event| contract.accepts_event(event))
    {
        return Err(GameAppShellError::Core(ScaffoldContractError::InvalidId));
    }

    let sensory = world.sensory_report(config.learner_id, Tick::ZERO)?;
    let sensory_heard_tokens = sensory
        .core_snapshot
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .map(|token| token.token_id)
        .collect::<Vec<_>>();
    let sensory_teacher_channels = sensory
        .core_snapshot
        .language_context
        .heard_tokens
        .iter()
        .flatten()
        .filter_map(|token| token.teacher_channel)
        .collect::<Vec<_>>();
    if !sensory_heard_tokens.contains(&77)
        || !sensory_teacher_channels.contains(&TeacherPerceptionChannel::Hearing)
    {
        return Err(GameAppShellError::Core(ScaffoldContractError::InvalidId));
    }

    let mut harness = HeadlessBrainHarness::new(world);
    let mut mind = CreatureMind::scaffold(
        config.learner_id,
        BrainScaleTier::Nano512,
        config.seed,
        Tick::ZERO,
    )?;
    let tick = harness.tick_mind(
        &mut mind,
        BrainTickInput::new(
            Tick::ZERO,
            vec![proposal(
                ActionKind::Inspect.canonical_id(),
                ActionKind::Inspect,
                Some(cue_token_id),
                None,
                0.82,
                0.90,
                0.0,
            )?],
        )
        .with_pack_experience(true)
        .with_action_duration(DurationTicks::new(1)),
    );
    if tick.brain.experience_patch.is_none() {
        return Err(GameAppShellError::Core(
            ScaffoldContractError::MissingPhaseData,
        ));
    }

    let topology_summary = tick
        .brain
        .topology_update
        .as_ref()
        .map(|update| TopologySummary {
            concept_count: 1,
            edge_count: update.edge_ids.len(),
            simplex_count: 1,
            gap_count: update.gap_ids.len(),
        })
        .unwrap_or_default();
    let evidence = SchoolEvidence::new(&harness.telemetry().sealed_patches)
        .with_memory_record_count(usize::from(tick.brain.memory_update.is_some()))
        .with_topology_summary(topology_summary);
    let verification =
        PatchLogLessonVerifier.verify_checks(&current_step.verifier_checks, &evidence)?;
    let advanced = runner.observe_verification(&verification)?;
    if !advanced {
        return Err(GameAppShellError::Core(ScaffoldContractError::InvalidId));
    }

    let response = LessonResponse::new(
        lesson_id,
        LessonResponseKind::CreatureVocalized,
        TeacherLessonResponseChannel::Speech,
    )
    .with_teacher_entity(teacher_avatar_stable_id);
    let metadata = response.to_action_metadata()?;
    let teacher_tagged_low = proposal(
        ActionId(10_701),
        ActionKind::Vocalize,
        Some(teacher_avatar_stable_id),
        None,
        0.30,
        0.90,
        0.0,
    )?
    .with_teacher_lesson(Some(metadata));
    let ordinary_high = proposal(
        ActionId(10_702),
        ActionKind::Inspect,
        Some(object_id),
        None,
        0.90,
        0.90,
        0.0,
    )?;
    let arbitration = cpu_reference_arbitrate(
        config.learner_id,
        &[teacher_tagged_low, ordinary_high],
        ActionArbitrationConfig::default(),
    )?;
    let teacher_metadata_bypass_blocked = arbitration.selected.action_id == ActionId(10_702)
        && arbitration.selected.teacher_lesson.is_none();

    let cue_lookup = [
        (
            TeacherInputKind::SpokenToken,
            Some(cue_token_id),
            "heard teacher word",
        ),
        (
            TeacherInputKind::ObjectHighlight,
            Some(object_id),
            "highlighted teaching object",
        ),
        (
            TeacherInputKind::SocialFeedback,
            Some(teacher_avatar_stable_id),
            "visible teacher praise",
        ),
    ];
    let cues = dispatch
        .perception_events
        .iter()
        .enumerate()
        .map(|(index, event)| {
            let cue_entity = cue_lookup
                .iter()
                .find(|(kind, _, _)| *kind == event.input_kind)
                .and_then(|(_, entity, _)| *entity);
            let label = cue_lookup
                .iter()
                .find(|(kind, _, _)| *kind == event.input_kind)
                .map(|(_, _, label)| *label)
                .unwrap_or("teacher perception cue");
            SchoolCuePresentation::from_event(
                *event,
                cue_entity,
                format!("{} #{}", label, index + 1),
            )
        })
        .collect::<Result<Vec<_>, ScaffoldContractError>>()?;
    let p34_school = SchoolSaveState {
        schema_version: TEACHER_SCHOOL_SCHEMA_VERSION,
        enabled: true,
        active_curriculum_id: Some(config.curriculum_id.clone()),
        teacher_private_state_saved: false,
    };
    let lesson_panel = SchoolLessonPanel {
        curriculum_id: config.curriculum_id.clone(),
        active_lesson_id: current_step.lesson_id,
        step_kind: current_step.kind,
        total_steps: curriculum.steps.len().min(config.lesson_limit),
        completed_steps: runner.completed_step_count(),
        cue_count: cues.len(),
        response_channels: current_step.response_channels.clone(),
        status_line: format!(
            "lesson {:?} passed={} completed={}",
            current_step.kind,
            verification.passed,
            runner.completed_step_count()
        ),
    };
    let verifier_panel = SchoolVerifierPanel {
        passed: verification.passed,
        observed_checks: verification
            .observed_checks
            .iter()
            .map(|check| format!("{check:?}"))
            .collect(),
        failed_checks: verification
            .failed_checks
            .iter()
            .map(|check| format!("{check:?}"))
            .collect(),
        sealed_patch_count: harness.telemetry().sealed_patches.len(),
        verifier_message: if verification.passed {
            "sealed patch verifier passed; teacher remained perception-only".to_string()
        } else {
            "sealed patch verifier failed".to_string()
        },
    };
    let save_preview = SchoolModeSaveState {
        schema: G10_SCHOOL_MODE_SCHEMA.to_string(),
        schema_version: G10_SCHOOL_MODE_SCHEMA_VERSION,
        seed: config.seed,
        curriculum_id: config.curriculum_id.clone(),
        active_lesson_id: lesson_id.raw(),
        completed_steps: lesson_panel.completed_steps,
        teacher_avatar_stable_id,
        cue_entity_ids: cues.iter().filter_map(|cue| cue.cue_entity).collect(),
        verifier_passed: verifier_panel.passed,
        p34_school: p34_school.clone(),
    };
    let json = save_preview.to_json_string_pretty()?;
    let save_roundtrip = SchoolModeSaveState::from_json_str(&json)?;
    let summary = SchoolModeSummary {
        schema: G10_SCHOOL_MODE_SCHEMA,
        schema_version: G10_SCHOOL_MODE_SCHEMA_VERSION,
        seed: config.seed,
        teacher_avatar_stable_id,
        learner_stable_id,
        lesson_panel,
        cues,
        verifier_panel,
        sensory_heard_tokens,
        sensory_teacher_channels,
        teacher_metadata_bypass_blocked,
        teacher_selected_action_id: arbitration
            .selected
            .teacher_lesson
            .map(|_| arbitration.selected.action_id),
        world_signature: harness.world().stable_signature(),
        p34_school,
        save_roundtrip_signature: save_roundtrip.signature_line(),
    };
    summary.validate()?;
    Ok(summary)
}

pub fn run_semantic_provider_smoke() -> Result<SemanticProviderSmokeSummary, GameAppShellError> {
    let disabled_config = SemanticProviderConfig::disabled();
    let disabled_manifest = SemanticProviderCapabilityManifest::disabled();
    let disabled_panel = semantic_panel_from_bundle(
        disabled_config,
        disabled_manifest,
        SemanticContextBundle {
            gaussian_context: None,
            semantic_context: None,
        },
        "missing semantic provider is nonfatal; no context displayed".to_string(),
    )?;

    let fake_config = SemanticProviderConfig::fake_local_table();
    let provider = FakeSemanticProvider::new();
    let fake_manifest = provider.capability_manifest();
    let request = SemanticContextRequest::new(Vec3f::new(0.25, 0.0, 0.75))
        .with_gaussian_observation(
            GaussianClusterId(11_001),
            0.72,
            1.25,
            Vec3f::new(0.25, 0.0, 0.75),
        )
        .with_semantic_binding(SemanticConceptBinding {
            concept_id: ConceptCellId(11_101),
            salience: 0.86,
        })
        .with_semantic_descriptor(SemanticCodeDescriptor {
            codebook_id: 11,
            descriptor: [11_i8; 32],
            salience: 0.64,
        });
    let fake_bundle = provider.build_context_bundle(&request)?;
    let fake_panel = semantic_panel_from_bundle(
        fake_config.clone(),
        fake_manifest.clone(),
        fake_bundle,
        "deterministic fake/local table provider; external SLM extension point only".to_string(),
    )?;

    let mut bad_schema = fake_config;
    bad_schema.schema_version = G11_SEMANTIC_PROVIDER_SCHEMA_VERSION + 1;
    let unknown_schema_rejected = bad_schema.validate().is_err();
    let unknown_provider_kind_json = format!(
        r#"{{
            "schema":"{}",
            "schema_version":{},
            "provider_id":"unknown-provider",
            "provider_kind":"unknown_provider",
            "required":false,
            "max_display_entries":4
        }}"#,
        G11_SEMANTIC_PROVIDER_SCHEMA, G11_SEMANTIC_PROVIDER_SCHEMA_VERSION
    );
    let unknown_provider_kind_rejected =
        SemanticProviderConfig::from_json_str(&unknown_provider_kind_json).is_err();

    let semantic_marked_low = ActionProposal::new(
        ActionId(11_001),
        ActionKind::Vocalize,
        0.05,
        Confidence::new(0.4)?,
        None,
        0x11,
        ActionTarget::NONE,
        NormalizedScalar::new(0.25)?,
    )?;
    let ordinary_high = ActionProposal::new(
        ActionId(11_002),
        ActionKind::Inspect,
        0.72,
        Confidence::new(0.8)?,
        None,
        0x01,
        ActionTarget::NONE,
        NormalizedScalar::new(0.6)?,
    )?;
    let decision = cpu_reference_arbitrate(
        OrganismId(11_001),
        &[semantic_marked_low, ordinary_high],
        ActionArbitrationConfig::default(),
    )?;
    let semantic_action_bypass_blocked =
        !fake_manifest.can_issue_actions && decision.selected.action_id == ActionId(11_002);
    let weight_rewrite_blocked = !fake_manifest.can_rewrite_weights;

    let summary = SemanticProviderSmokeSummary {
        schema: G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA,
        schema_version: G11_SEMANTIC_PROVIDER_DISPLAY_SCHEMA_VERSION,
        provider_schema: G11_SEMANTIC_PROVIDER_SCHEMA,
        provider_schema_version: G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
        disabled_panel,
        fake_panel,
        unknown_schema_rejected,
        unknown_provider_kind_rejected,
        semantic_action_bypass_blocked,
        weight_rewrite_blocked,
        provider_absence_nonfatal: true,
        provider_failure_nonfatal: true,
    };
    summary.validate()?;
    Ok(summary)
}

fn semantic_panel_from_bundle(
    config: SemanticProviderConfig,
    manifest: SemanticProviderCapabilityManifest,
    bundle: SemanticContextBundle,
    extension_note: String,
) -> Result<SemanticProviderDebugPanel, GameAppShellError> {
    let mut display_lines = Vec::new();
    let mut semantic_code_count = 0;
    let mut concept_binding_count = 0;
    let mut gaussian_cluster_count = 0;

    if let Some(context) = &bundle.semantic_context {
        context.validate_contract()?;
        semantic_code_count = context.compressed_codes.len();
        concept_binding_count = context.salience.len();
        for entry in &context.salience {
            display_lines.push(SemanticContextDisplayLine {
                source: "semantic-concept".to_string(),
                label: format!("concept-{}", entry.concept_id.raw()),
                salience_percent: normalized_percent(entry.salience.raw()),
            });
        }
        for code in &context.compressed_codes {
            display_lines.push(SemanticContextDisplayLine {
                source: "semantic-code".to_string(),
                label: format!("codebook-{}:{}", code.codebook_id, code.code),
                salience_percent: normalized_percent(code.salience.raw()),
            });
        }
    }
    if let Some(context) = &bundle.gaussian_context {
        context.validate_contract()?;
        gaussian_cluster_count = context.clusters.len();
        for cluster in &context.clusters {
            display_lines.push(SemanticContextDisplayLine {
                source: "gaussian-cluster".to_string(),
                label: format!(
                    "cluster-{}@{:.2}m",
                    cluster.cluster_id.raw(),
                    cluster.distance_meters
                ),
                salience_percent: normalized_percent(cluster.salience.raw()),
            });
        }
    }
    display_lines.truncate(config.max_display_entries);
    display_lines.sort_by(|lhs, rhs| {
        rhs.salience_percent
            .cmp(&lhs.salience_percent)
            .then(lhs.source.cmp(&rhs.source))
            .then(lhs.label.cmp(&rhs.label))
    });

    let panel = SemanticProviderDebugPanel {
        config,
        manifest,
        missing_provider_nonfatal: true,
        provider_failure_nonfatal: true,
        context_visible: !display_lines.is_empty(),
        semantic_code_count,
        concept_binding_count,
        gaussian_cluster_count,
        display_lines,
        extension_note,
    };
    panel.validate()?;
    Ok(panel)
}

fn normalized_percent(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 100.0).round() as u8
}

pub fn run_gpu_product_hardening_smoke() -> Result<GpuProductHardeningSummary, GameAppShellError> {
    run_gpu_product_hardening_smoke_impl()
}

#[cfg(feature = "gpu-runtime")]
fn run_gpu_product_hardening_smoke_impl() -> Result<GpuProductHardeningSummary, GameAppShellError> {
    use alife_gpu_backend::{
        GpuRuntimeBackendConfig, GpuRuntimeBackendKind, GpuRuntimeBoundary,
        GpuRuntimeReadbackGuard, GpuRuntimeTimingSample, GpuTierMeasurement,
    };

    let cpu_status = GpuRuntimeBackendConfig::default().select_backend()?;
    let invalid_gpu_status = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuStatic)
        .with_gpu_feature_enabled(true)
        .with_hardware_available(true)
        .with_validation_passed(false)
        .select_backend()?;
    let active_guard = GpuRuntimeReadbackGuard::active_tick();
    let active_readback_blocked = !active_guard.permits_bulk_neural_readback()
        && !active_guard.permits_per_synapse_readback()
        && !active_guard.permits_per_lobe_readback()
        && !active_guard.permits_weight_readback()
        && active_guard
            .validate_export_request(GpuRuntimeBoundary::DiagnosticExport)
            .is_err();
    let diagnostic_export_boundary_allowed = GpuRuntimeReadbackGuard::after_frame_boundary()
        .validate_export_request(GpuRuntimeBoundary::DiagnosticExport)
        .is_ok();
    let report = GpuTierMeasurement::cpu_fallback_report(
        invalid_gpu_status,
        "G12 product smoke: CPU fallback is not GPU performance; run manual hardware command for measured GPU data",
    );
    report.validate()?;
    let report_markdown_preview = report.to_markdown();
    let telemetry_overlay = GpuProductTelemetryOverlay {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        requested_backend: format!("{:?}", invalid_gpu_status.requested),
        selected_backend: format!("{:?}", invalid_gpu_status.selected),
        fallback_reason: invalid_gpu_status
            .fallback_reason
            .map(|reason| format!("{reason:?}")),
        gpu_runtime_feature_compiled: true,
        cpu_oracle_authoritative: invalid_gpu_status.cpu_oracle_authoritative,
        no_active_gameplay_readback: invalid_gpu_status.no_active_gameplay_readback,
        telemetry_boundary: "frame-boundary-diagnostic-export".to_string(),
        tick_time_ms: Some(
            GpuRuntimeTimingSample {
                measured_gpu_neural_ms: 0.0,
                measured_frame_ms: 0.0,
            }
            .measured_frame_ms,
        ),
        gpu_neural_time_ms: None,
        skipped_supertiles: 0,
        skipped_tiles: 0,
        measured_gpu_performance: false,
        report_notes: "CPU fallback; no GPU hardware timing claimed".to_string(),
    };
    let summary = GpuProductHardeningSummary {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        cpu_fallback_default: cpu_status.selected == GpuRuntimeBackendKind::CpuReference
            && cpu_status.fallback_reason.is_none(),
        invalid_gpu_config_falls_back: invalid_gpu_status.selected
            == GpuRuntimeBackendKind::CpuReference
            && invalid_gpu_status.fallback_reason.is_some(),
        active_readback_blocked,
        diagnostic_export_boundary_allowed,
        telemetry_overlay,
        report_markdown_preview,
        manual_hardware_command: g12_manual_gpu_hardware_command().to_string(),
        performance_claim_status: "unknown-unless-measured".to_string(),
    };
    summary.validate()?;
    Ok(summary)
}

#[cfg(not(feature = "gpu-runtime"))]
fn run_gpu_product_hardening_smoke_impl() -> Result<GpuProductHardeningSummary, GameAppShellError> {
    let telemetry_overlay = GpuProductTelemetryOverlay {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        requested_backend: "GpuStatic".to_string(),
        selected_backend: "CpuReference".to_string(),
        fallback_reason: Some("FeatureDisabled".to_string()),
        gpu_runtime_feature_compiled: false,
        cpu_oracle_authoritative: true,
        no_active_gameplay_readback: true,
        telemetry_boundary: "frame-boundary-diagnostic-export".to_string(),
        tick_time_ms: None,
        gpu_neural_time_ms: None,
        skipped_supertiles: 0,
        skipped_tiles: 0,
        measured_gpu_performance: false,
        report_notes: "GPU runtime feature disabled; CPU fallback is not GPU performance"
            .to_string(),
    };
    let summary = GpuProductHardeningSummary {
        schema: G12_GPU_PRODUCT_TELEMETRY_SCHEMA,
        schema_version: G12_GPU_PRODUCT_TELEMETRY_SCHEMA_VERSION,
        cpu_fallback_default: true,
        invalid_gpu_config_falls_back: true,
        active_readback_blocked: true,
        diagnostic_export_boundary_allowed: true,
        telemetry_overlay,
        report_markdown_preview:
            "# G12 GPU product telemetry\n\nCPU fallback is not GPU performance.\n".to_string(),
        manual_hardware_command: g12_manual_gpu_hardware_command().to_string(),
        performance_claim_status: "unknown-unless-measured".to_string(),
    };
    summary.validate()?;
    Ok(summary)
}

pub const fn g12_manual_gpu_hardware_command() -> &'static str {
    "ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime"
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

    pub fn ecology_metrics(&self) -> EcologyMetrics {
        self.harness.world().ecology_metrics()
    }

    pub fn ecology_indicators(&self) -> Vec<EcologyIndicator> {
        self.harness
            .world()
            .ecology()
            .zones
            .iter()
            .map(|zone| EcologyIndicator {
                zone_id: zone.id,
                label: zone.label.clone(),
                terrain_kind: zone.kind,
                resource_bias: zone.resource_bias,
                hazard_pressure: zone.hazard_pressure,
            })
            .collect()
    }

    pub fn current_ecology_zone_label(&self) -> Result<Option<String>, GameAppShellError> {
        let report = self
            .harness
            .world()
            .sensory_report(self.organism_id, self.mind.current_tick())?;
        Ok(report.ecology.current_zone.and_then(|zone_id| {
            self.harness
                .world()
                .ecology()
                .zones
                .iter()
                .find(|zone| zone.id == zone_id)
                .map(|zone| zone.label.clone())
        }))
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

pub fn run_playable_survival_loop_smoke() -> Result<PlayableSurvivalLoopSummary, GameAppShellError>
{
    const SEED: u64 = 6_060;
    let organism_id = OrganismId(606);
    let food_position = Vec3f::new(1.0, 0.0, 0.0);
    let hazard_position = Vec3f::new(2.0, 0.0, 0.0);
    let world = HeadlessScenarioBuilder::new(SEED)
        .agent("creature", organism_id, Vec3f::ZERO)
        .food("berry", food_position, 0.75)
        .hazard("thorn", hazard_position, 0.8)
        .obstacle("stone", Vec3f::new(-1.5, 0.0, 0.0), 0.75)
        .token("rest-nest", Vec3f::new(0.0, 1.0, 0.0), 60_600)
        .build()?;
    let food = world
        .entity_id("berry")
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "G06 scenario must include food",
        })?;
    let hazard = world
        .entity_id("thorn")
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "G06 scenario must include hazard",
        })?;
    let object_count = world.stable_signature().len();
    let mut mind = CreatureMind::scaffold(organism_id, BrainScaleTier::Nano512, SEED, Tick::ZERO)?;
    {
        let homeostasis = mind.homeostasis_mut();
        homeostasis.drives.hunger = 0.82;
        homeostasis.drives.fatigue = 0.72;
        homeostasis.drives.fear = 0.05;
        homeostasis.drives.pain = 0.0;
        homeostasis.drives.brain_atp = 0.54;
        homeostasis.hormones.sleep_pressure = 0.76;
        homeostasis.validate_contract()?;
    }

    let mut live = LiveBrainLoop::new(world, mind, organism_id, true);
    let mut tick_summaries = Vec::new();
    let mut events = Vec::new();
    let scripted = [
        (
            PlayableSurvivalEventKind::FoodConsumed,
            proposal(
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                Some(food),
                None,
                0.96,
                0.97,
                1.0,
            )?,
            "ate visible food; hunger drops and packed/sealed logs update",
        ),
        (
            PlayableSurvivalEventKind::MissingAffordance,
            proposal(
                HeadlessActionIds::EAT,
                ActionKind::Interact,
                Some(food),
                None,
                0.94,
                0.95,
                1.0,
            )?,
            "tried consumed food once; failure is recoverable and bounded",
        ),
        (
            PlayableSurvivalEventKind::HazardPain,
            proposal(
                ActionKind::Move.canonical_id(),
                ActionKind::Move,
                Some(hazard),
                None,
                0.93,
                0.94,
                1.0,
            )?,
            "entered visible hazard; pain/fear rise and topology gap remains bias-only",
        ),
        (
            PlayableSurvivalEventKind::RestSleep,
            proposal(
                ActionKind::Rest.canonical_id(),
                ActionKind::Rest,
                None,
                None,
                0.91,
                0.92,
                0.0,
            )?,
            "rest action succeeds; P16 forced sleep hook becomes visible",
        ),
    ];

    for (kind, action, message) in scripted {
        let before = *live.mind().homeostasis();
        let summary = live.tick_with_proposals(vec![action]);
        let after = live.mind().homeostasis();
        let event = PlayableSurvivalEvent {
            kind,
            tick: summary.tick_after,
            action_kind: summary.selected_action_kind,
            target_entity: summary.target_entity,
            success: summary.patch_success.unwrap_or(false),
            contact: summary.physical_contact,
            hunger_before: before.drives.hunger,
            hunger_after: after.drives.hunger,
            fatigue_after: after.drives.fatigue,
            fear_after: after.drives.fear,
            pain_after: after.drives.pain,
            brain_atp_after: after.drives.brain_atp,
            sleep_phase_after: live.mind().sleep_state().phase,
            message: message.to_string(),
        };
        event.validate()?;
        events.push(event);
        tick_summaries.push(summary);
    }

    let (sealed_patch_count, packed_record_count) = live.telemetry_counts();
    let final_visual = creature_visual_snapshot_from_parts(
        organism_id,
        WorldEntityId(1),
        hazard_position,
        None,
        None,
        live.mind().homeostasis(),
        live.mind().sleep_state().phase,
        tick_summaries
            .last()
            .and_then(|summary| summary.selected_action_kind),
    )?;
    let summary = PlayableSurvivalLoopSummary {
        schema: G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA,
        schema_version: G06_PLAYABLE_SURVIVAL_LOOP_SCHEMA_VERSION,
        seed: SEED,
        organism_id,
        object_count,
        events,
        tick_summaries,
        final_visual,
        sealed_patch_count,
        packed_record_count,
        memory_record_count: live.mind().memory_bank().len(),
        topology_concept_count: live.mind().topological_map().concepts().len(),
        unresolved_gap_count: live.mind().topological_map().unresolved_gaps().len(),
        world_signature: live.world_signature(),
    };
    summary.validate()?;
    Ok(summary)
}

pub fn run_world_ecology_loop_smoke() -> Result<PlayableEcologyLoopSummary, GameAppShellError> {
    const SEED: u64 = 7_070;
    let organism_id = OrganismId(707);
    let food_position = Vec3f::new(0.8, 0.0, 0.0);
    let hazard_position = Vec3f::new(4.0, 0.0, 0.0);
    let world = HeadlessScenarioBuilder::new(SEED)
        .agent("creature", organism_id, Vec3f::ZERO)
        .food("berry", food_position, 0.7)
        .hazard("bramble", Vec3f::new(4.5, 0.0, 0.0), 0.25)
        .terrain_zone(
            1,
            "meadow",
            TerrainZoneKind::Meadow,
            Vec3f::ZERO,
            3.0,
            0.8,
            0.0,
        )
        .terrain_zone(
            2,
            "ash-field",
            TerrainZoneKind::HazardField,
            hazard_position,
            2.0,
            0.1,
            0.65,
        )
        .track_resource("berry", 1, 2, 4)
        .resource_spawn_policy("seed-berry", 1, 2, 2, 0.35)
        .build()?;
    let food = world
        .entity_id("berry")
        .ok_or(GameAppShellError::VisibleWorldMismatch {
            message: "G07 scenario must include tracked food",
        })?;

    let mut mind = CreatureMind::scaffold(organism_id, BrainScaleTier::Nano512, SEED, Tick::ZERO)?;
    {
        let homeostasis = mind.homeostasis_mut();
        homeostasis.drives.hunger = 0.78;
        homeostasis.drives.fatigue = 0.38;
        homeostasis.drives.fear = 0.04;
        homeostasis.drives.brain_atp = 0.58;
        homeostasis.validate_contract()?;
    }

    let mut live = LiveBrainLoop::new(world, mind, organism_id, true);
    let mut tick_summaries = Vec::new();
    let mut hazard_pain = 0.0;
    let mut hazard_tick = Tick::ZERO;
    let scripted = [
        proposal(
            HeadlessActionIds::EAT,
            ActionKind::Interact,
            Some(food),
            None,
            0.96,
            0.97,
            0.8,
        )?,
        proposal(
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            None,
            None,
            0.84,
            0.9,
            0.0,
        )?,
        proposal(
            ActionKind::Idle.canonical_id(),
            ActionKind::Idle,
            None,
            None,
            0.83,
            0.9,
            0.0,
        )?,
        proposal(
            ActionKind::Move.canonical_id(),
            ActionKind::Move,
            None,
            Some(hazard_position),
            0.93,
            0.95,
            4.0,
        )?,
    ];

    for action in scripted {
        let before_pain = live.mind().homeostasis().drives.pain;
        let summary = live.tick_with_proposals(vec![action]);
        if live.mind().homeostasis().drives.pain > before_pain {
            hazard_tick = summary.tick_after;
            hazard_pain = live.mind().homeostasis().drives.pain - before_pain;
        }
        tick_summaries.push(summary);
    }

    let indicators = live.ecology_indicators();
    let metrics = live.ecology_metrics();
    let (sealed_patch_count, packed_record_count) = live.telemetry_counts();
    let summary = PlayableEcologyLoopSummary {
        schema: G07_WORLD_ECOLOGY_SCHEMA,
        schema_version: G07_WORLD_ECOLOGY_SCHEMA_VERSION,
        seed: SEED,
        organism_id,
        tick_summaries,
        ecology_indicators: indicators,
        metrics,
        regrown_resource_id: Some(food),
        spawned_labels: live
            .world_signature()
            .into_iter()
            .filter(|line| line.contains("seed-berry"))
            .collect(),
        hazard_tick,
        hazard_pain: hazard_pain.clamp(0.0, 1.0),
        sensory_zone_label: live.current_ecology_zone_label()?,
        world_signature: live.world_signature(),
        sealed_patch_count,
        packed_record_count,
    };
    summary.validate()?;
    Ok(summary)
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
