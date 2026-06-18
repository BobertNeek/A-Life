//! G01 playable-sim app shell.
//!
//! This crate owns product app startup policy. The default path remains
//! headless and CI-safe; Bevy construction is behind the `bevy-app` feature.

use std::path::{Path, PathBuf};

use alife_core::{
    ActionId, ActionKind, ActionProposal, ActionTarget, BrainTickInput, BrainTickStatus,
    Confidence, CreatureMind, DurationTicks, Intensity, NormalizedScalar, OrganismId,
    PhysicalContactKind, ReferenceActionFailure, ScaffoldContractError, Tick, Validate, Vec3f,
    WorldEntityId,
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

#[cfg(feature = "bevy-app")]
pub mod bevy_shell {
    use alife_bevy_adapter::{
        core_vec3_to_bevy, AffordanceTags, AlifeBevyAdapterPlugin, BevyEntityMap, CreatureBody,
        SensoryEmitter,
    };
    use alife_core::{AffordanceBits, WorldEntityId};
    use alife_world::WorldObjectKind;
    use bevy::prelude::{App, Component, MinimalPlugins, Resource, Transform};

    use crate::{
        load_visible_world_from_p34_save, run_live_brain_loop_smoke, AppShellLaunchConfig,
        AppStartupSummary, GameAppShellError, GameAppState, LiveBrainTickSummary,
        VisibleMaterialKind, VisiblePlaceholderShape, VisibleWorldPresentation,
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
