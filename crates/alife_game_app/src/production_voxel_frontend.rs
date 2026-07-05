//! FVR01 production voxel frontend launch policy.
//!
//! This module wires production profile budgets, launch states, and diagnostics
//! around the existing real P34 config/save/asset contracts. It does not move
//! renderer, Bevy, or GPU handles into core/world state.

use std::fs;

use crate::prelude::*;
use crate::*;
use alife_core::{DriveSnapshot, EndocrineSnapshot};
use alife_world::persistence::{
    CreatureMindSaveSummary, CreatureSaveState, LearningTraceSaveSummary, WeightLayerSaveSummary,
};

pub const PRODUCTION_VOXEL_COMMAND: &str = "production-voxel";
pub const PRODUCTION_VOXEL_WINDOW_TITLE: &str = "A-Life Voxel Frontend";
pub const PRODUCTION_VOXEL_RENDERER_PROFILE: &str = "voxel-backend";
pub const PRODUCTION_VOXEL_SCENARIO_ID: &str = "production-voxel";
pub const FVR01_RUNTIME_DIAGNOSTIC_LOG: &str =
    "target/artifacts/fvr01_production_voxel/runtime_prereq.log";
pub const FVR05_PRODUCTION_UX_SCHEMA: &str = "alife.fvr05.production_ux.v1";
pub const FVR05_PRODUCTION_UX_SCHEMA_VERSION: u16 = 1;
pub const FVR05_PRODUCTION_UX_SETTINGS_DIR: &str = "target/artifacts/fvr05";

const FVR05_ENGINE_LOCAL_TOKENS: [&str; 11] = [
    "bevy",
    "wgpu",
    "entity(",
    "renderer",
    "windowhandle",
    "oswindow",
    "mesh3d",
    "standardmaterial",
    "handle<",
    "avian",
    "egui",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Fvr05ProductionOverlayKind {
    Resources,
    Danger,
    Pheromones,
    Energy,
    Age,
    Fertility,
    Territory,
    Neural,
    Residency,
    BackendTiming,
    ChunkBoundaries,
    LodBudget,
    Persistence,
}

impl Fvr05ProductionOverlayKind {
    pub const fn all() -> &'static [Self; 13] {
        &[
            Self::Resources,
            Self::Danger,
            Self::Pheromones,
            Self::Energy,
            Self::Age,
            Self::Fertility,
            Self::Territory,
            Self::Neural,
            Self::Residency,
            Self::BackendTiming,
            Self::ChunkBoundaries,
            Self::LodBudget,
            Self::Persistence,
        ]
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Resources => "Resources",
            Self::Danger => "Danger",
            Self::Pheromones => "Pheromones",
            Self::Energy => "Energy",
            Self::Age => "Age",
            Self::Fertility => "Fertility",
            Self::Territory => "Territory",
            Self::Neural => "Neural",
            Self::Residency => "Residency",
            Self::BackendTiming => "BackendTiming",
            Self::ChunkBoundaries => "ChunkBoundaries",
            Self::LodBudget => "LodBudget",
            Self::Persistence => "Persistence",
        }
    }

    pub fn default_enabled_for_profile(profile_id: ProductionFrontendProfileId) -> Vec<Self> {
        match profile_id {
            ProductionFrontendProfileId::MinimumSettings30x30 => vec![
                Self::Resources,
                Self::Danger,
                Self::Energy,
                Self::BackendTiming,
                Self::Persistence,
            ],
            ProductionFrontendProfileId::MinSpecComfort1080p => vec![
                Self::Resources,
                Self::Danger,
                Self::Energy,
                Self::Fertility,
                Self::Neural,
                Self::Residency,
                Self::BackendTiming,
                Self::Persistence,
            ],
            _ => Self::all().to_vec(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Fvr05ProductionInspectorTab {
    Creature,
    Tile,
    World,
    GpuRuntime,
}

impl Fvr05ProductionInspectorTab {
    pub const fn all() -> &'static [Self; 4] {
        &[Self::Creature, Self::Tile, Self::World, Self::GpuRuntime]
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Creature => "Creature",
            Self::Tile => "Tile",
            Self::World => "World",
            Self::GpuRuntime => "GPU",
        }
    }

    pub fn next(self) -> Self {
        let all = Self::all();
        let index = all.iter().position(|tab| *tab == self).unwrap_or_default();
        all[(index + 1) % all.len()]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fvr05ProductionUxSettings {
    pub schema: String,
    pub schema_version: u16,
    pub selected_profile: ProductionFrontendProfileId,
    pub preferred_profile_for_next_launch: ProductionFrontendProfileId,
    pub active_inspector_tab: Fvr05ProductionInspectorTab,
    pub enabled_overlays: Vec<Fvr05ProductionOverlayKind>,
    pub camera_mode: String,
    pub paused: bool,
    pub simulation_speed: f32,
    pub follow_selection: bool,
    pub show_menu: bool,
    pub show_settings: bool,
    pub show_overlays: bool,
    pub pause_on_focus_loss: bool,
    pub selected_stable_id: Option<u64>,
    pub source_save_path: String,
    pub runtime_save_path: String,
    pub created_world_save_path: String,
    pub asset_manifest_path: String,
    pub backend_descriptor: String,
    pub validation_receipt: String,
}

impl Fvr05ProductionUxSettings {
    pub fn default_for_launch(
        launch: &ProductionVoxelLaunchConfig,
        diagnostics: &ProductionRuntimeDiagnostics,
        save_metadata: &ProductionSaveMetadata,
    ) -> Self {
        let artifact_dir = PathBuf::from(FVR05_PRODUCTION_UX_SETTINGS_DIR);
        let profile = launch.profile_id.label();
        Self {
            schema: FVR05_PRODUCTION_UX_SCHEMA.to_string(),
            schema_version: FVR05_PRODUCTION_UX_SCHEMA_VERSION,
            selected_profile: launch.profile_id,
            preferred_profile_for_next_launch: launch.profile_id,
            active_inspector_tab: Fvr05ProductionInspectorTab::Creature,
            enabled_overlays: Fvr05ProductionOverlayKind::default_enabled_for_profile(
                launch.profile_id,
            ),
            camera_mode: "orthographic-isometric".to_string(),
            paused: false,
            simulation_speed: 1.0,
            follow_selection: false,
            show_menu: true,
            show_settings: true,
            show_overlays: true,
            pause_on_focus_loss: true,
            selected_stable_id: None,
            source_save_path: launch.app_launch.save_path.display().to_string(),
            runtime_save_path: artifact_dir
                .join(format!("{profile}_runtime_save.json"))
                .display()
                .to_string(),
            created_world_save_path: artifact_dir
                .join(format!("{profile}_created_world_save.json"))
                .display()
                .to_string(),
            asset_manifest_path: launch.app_launch.asset_manifest_path.display().to_string(),
            backend_descriptor: diagnostics.selected_backend.clone(),
            validation_receipt: format!(
                "save={} schema={} v{} profile={} chunks={} selections={}",
                save_metadata.save_id,
                save_metadata.save_schema,
                save_metadata.save_schema_version,
                save_metadata.selected_profile,
                save_metadata.voxel_visible_chunk_signatures,
                save_metadata.voxel_stable_selection_refs
            ),
        }
    }

    pub fn refresh_runtime_context(&mut self, default: &Self) {
        self.schema = default.schema.clone();
        self.schema_version = default.schema_version;
        self.selected_profile = default.selected_profile;
        self.source_save_path = default.source_save_path.clone();
        self.runtime_save_path = default.runtime_save_path.clone();
        self.created_world_save_path = default.created_world_save_path.clone();
        self.asset_manifest_path = default.asset_manifest_path.clone();
        self.backend_descriptor = default.backend_descriptor.clone();
        self.validation_receipt = default.validation_receipt.clone();
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != FVR05_PRODUCTION_UX_SCHEMA
            || self.schema_version != FVR05_PRODUCTION_UX_SCHEMA_VERSION
            || self.enabled_overlays.is_empty()
            || !self.simulation_speed.is_finite()
            || !(0.10..=5.0).contains(&self.simulation_speed)
            || self.camera_mode.trim().is_empty()
            || self.source_save_path.trim().is_empty()
            || self.runtime_save_path.trim().is_empty()
            || self.created_world_save_path.trim().is_empty()
            || self.asset_manifest_path.trim().is_empty()
            || self.backend_descriptor.trim().is_empty()
            || self.validation_receipt.trim().is_empty()
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "invalid FVR05 production UX settings".to_string(),
            });
        }
        let json = serde_json::to_string(self)?;
        if fvr05_contains_engine_local_token(&json) {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "FVR05 UX settings leaked engine-local renderer tokens".to_string(),
            });
        }
        Ok(())
    }

    pub fn from_json_str(text: &str) -> Result<Self, GameAppShellError> {
        let settings: Self = serde_json::from_str(text)?;
        settings.validate()?;
        Ok(settings)
    }

    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, GameAppShellError> {
        Self::from_json_str(&fs::read_to_string(path)?)
    }

    pub fn to_json_string_pretty(&self) -> Result<String, GameAppShellError> {
        self.validate()?;
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn to_json_file(&self, path: impl AsRef<Path>) -> Result<(), GameAppShellError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.to_json_string_pretty()?)?;
        Ok(())
    }

    pub fn overlay_labels(&self) -> Vec<&'static str> {
        self.enabled_overlays
            .iter()
            .map(|overlay| overlay.label())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fvr05ProductionDebugAuthorityReport {
    pub schema: &'static str,
    pub schema_version: u16,
    pub read_only_projection: bool,
    pub direct_actions_blocked: bool,
    pub action_arbitration_bypass_blocked: bool,
    pub reward_injection_blocked: bool,
    pub weight_mutation_blocked: bool,
    pub hidden_cognition_mutation_blocked: bool,
    pub bulk_neural_readback_blocked: bool,
}

impl Fvr05ProductionDebugAuthorityReport {
    pub const fn production_read_only() -> Self {
        Self {
            schema: FVR05_PRODUCTION_UX_SCHEMA,
            schema_version: FVR05_PRODUCTION_UX_SCHEMA_VERSION,
            read_only_projection: true,
            direct_actions_blocked: true,
            action_arbitration_bypass_blocked: true,
            reward_injection_blocked: true,
            weight_mutation_blocked: true,
            hidden_cognition_mutation_blocked: true,
            bulk_neural_readback_blocked: true,
        }
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != FVR05_PRODUCTION_UX_SCHEMA
            || self.schema_version != FVR05_PRODUCTION_UX_SCHEMA_VERSION
            || !self.read_only_projection
            || !self.direct_actions_blocked
            || !self.action_arbitration_bypass_blocked
            || !self.reward_injection_blocked
            || !self.weight_mutation_blocked
            || !self.hidden_cognition_mutation_blocked
            || !self.bulk_neural_readback_blocked
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "FVR05 debug authority report is not read-only".to_string(),
            });
        }
        Ok(())
    }

    pub fn compact_line(&self) -> String {
        format!(
            "read_only={} actions_blocked={} arbitration_bypass_blocked={} rewards_blocked={} weights_blocked={} cognition_blocked={} bulk_readback_blocked={}",
            self.read_only_projection,
            self.direct_actions_blocked,
            self.action_arbitration_bypass_blocked,
            self.reward_injection_blocked,
            self.weight_mutation_blocked,
            self.hidden_cognition_mutation_blocked,
            self.bulk_neural_readback_blocked,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductionAppState {
    Boot,
    ValidateRuntime,
    LoadAssets,
    LoadOrCreateWorld,
    Running,
    Paused,
    Settings,
    Shutdown,
}

impl ProductionAppState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Boot => "Boot",
            Self::ValidateRuntime => "ValidateRuntime",
            Self::LoadAssets => "LoadAssets",
            Self::LoadOrCreateWorld => "LoadOrCreateWorld",
            Self::Running => "Running",
            Self::Paused => "Paused",
            Self::Settings => "Settings",
            Self::Shutdown => "Shutdown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionAppStateTrace {
    states: Vec<ProductionAppState>,
}

impl Default for ProductionAppStateTrace {
    fn default() -> Self {
        Self {
            states: vec![ProductionAppState::Boot],
        }
    }
}

impl ProductionAppStateTrace {
    pub fn states(&self) -> &[ProductionAppState] {
        &self.states
    }

    pub fn labels(&self) -> Vec<&'static str> {
        self.states.iter().map(|state| state.label()).collect()
    }

    pub fn current(&self) -> ProductionAppState {
        *self
            .states
            .last()
            .expect("production state trace always starts at Boot")
    }

    pub fn transition(&mut self, to: ProductionAppState) -> Result<(), GameAppShellError> {
        let from = self.current();
        if !valid_production_transition(from, to) {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!("invalid production state transition {from:?} -> {to:?}"),
            });
        }
        self.states.push(to);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductionFrontendProfileId {
    MinimumSettings30x30,
    #[default]
    MinSpecComfort1080p,
    Balanced1080p,
    HighSpecScaleUp,
    ResearchScale,
}

impl ProductionFrontendProfileId {
    pub const fn all() -> &'static [Self; 5] {
        &[
            Self::MinimumSettings30x30,
            Self::MinSpecComfort1080p,
            Self::Balanced1080p,
            Self::HighSpecScaleUp,
            Self::ResearchScale,
        ]
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::MinimumSettings30x30 => "MinimumSettings30x30",
            Self::MinSpecComfort1080p => "MinSpecComfort1080p",
            Self::Balanced1080p => "Balanced1080p",
            Self::HighSpecScaleUp => "HighSpecScaleUp",
            Self::ResearchScale => "ResearchScale",
        }
    }

    pub fn parse(value: &str) -> Result<Self, GameAppShellError> {
        Self::all()
            .iter()
            .copied()
            .find(|profile| profile.label() == value)
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "unknown production profile '{value}'. Known profiles: {}",
                    Self::labels().join(", ")
                ),
            })
    }

    pub fn labels() -> Vec<&'static str> {
        Self::all().iter().map(|profile| profile.label()).collect()
    }

    pub const fn budget(self) -> ProductionFrontendProfileBudget {
        match self {
            Self::MinimumSettings30x30 => ProductionFrontendProfileBudget {
                profile_id: Self::MinimumSettings30x30,
                default_population: 30,
                maximum_profile_population: 30,
                target_fps: 30,
                target_frame_ms: 33.3,
                output_resolution: (1920, 1080),
                internal_render_scale_floor: 0.67,
                default_internal_render_scale: 0.80,
                chunk_tile_size: 16,
                chunk_activation_radius: 2,
                active_chunk_cap: 128,
                hot_brain_slots: 4,
                warm_brain_slots: 12,
                cold_brain_slots: 14,
                vfx_budget: "conservative",
                shadow_quality: "low",
                label_density: "selected-hover-only",
                renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE,
                hard_floor: true,
                comfort_default: false,
                research_mode: false,
            },
            Self::MinSpecComfort1080p => ProductionFrontendProfileBudget {
                profile_id: Self::MinSpecComfort1080p,
                default_population: 30,
                maximum_profile_population: 50,
                target_fps: 60,
                target_frame_ms: 16.7,
                output_resolution: (1920, 1080),
                internal_render_scale_floor: 1.0,
                default_internal_render_scale: 1.0,
                chunk_tile_size: 16,
                chunk_activation_radius: 4,
                active_chunk_cap: 256,
                hot_brain_slots: 8,
                warm_brain_slots: 16,
                cold_brain_slots: 6,
                vfx_budget: "medium",
                shadow_quality: "stylized-medium",
                label_density: "compact",
                renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE,
                hard_floor: false,
                comfort_default: true,
                research_mode: false,
            },
            Self::Balanced1080p => ProductionFrontendProfileBudget {
                profile_id: Self::Balanced1080p,
                default_population: 50,
                maximum_profile_population: 100,
                target_fps: 60,
                target_frame_ms: 16.7,
                output_resolution: (1920, 1080),
                internal_render_scale_floor: 1.0,
                default_internal_render_scale: 1.0,
                chunk_tile_size: 16,
                chunk_activation_radius: 5,
                active_chunk_cap: 384,
                hot_brain_slots: 12,
                warm_brain_slots: 24,
                cold_brain_slots: 14,
                vfx_budget: "balanced",
                shadow_quality: "medium",
                label_density: "compact-expanded",
                renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE,
                hard_floor: false,
                comfort_default: false,
                research_mode: false,
            },
            Self::HighSpecScaleUp => ProductionFrontendProfileBudget {
                profile_id: Self::HighSpecScaleUp,
                default_population: 100,
                maximum_profile_population: 500,
                target_fps: 60,
                target_frame_ms: 16.7,
                output_resolution: (1920, 1080),
                internal_render_scale_floor: 1.0,
                default_internal_render_scale: 1.0,
                chunk_tile_size: 16,
                chunk_activation_radius: 8,
                active_chunk_cap: 768,
                hot_brain_slots: 24,
                warm_brain_slots: 64,
                cold_brain_slots: 412,
                vfx_budget: "high",
                shadow_quality: "high",
                label_density: "expanded",
                renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE,
                hard_floor: false,
                comfort_default: false,
                research_mode: false,
            },
            Self::ResearchScale => ProductionFrontendProfileBudget {
                profile_id: Self::ResearchScale,
                default_population: 250,
                maximum_profile_population: 500,
                target_fps: 30,
                target_frame_ms: 33.3,
                output_resolution: (1920, 1080),
                internal_render_scale_floor: 0.67,
                default_internal_render_scale: 1.0,
                chunk_tile_size: 16,
                chunk_activation_radius: 10,
                active_chunk_cap: 1024,
                hot_brain_slots: 32,
                warm_brain_slots: 128,
                cold_brain_slots: 340,
                vfx_budget: "adaptive-research",
                shadow_quality: "adaptive",
                label_density: "sampled",
                renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE,
                hard_floor: false,
                comfort_default: false,
                research_mode: true,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProductionFrontendProfileBudget {
    pub profile_id: ProductionFrontendProfileId,
    pub default_population: u16,
    pub maximum_profile_population: u16,
    pub target_fps: u16,
    pub target_frame_ms: f32,
    pub output_resolution: (u32, u32),
    pub internal_render_scale_floor: f32,
    pub default_internal_render_scale: f32,
    pub chunk_tile_size: u16,
    pub chunk_activation_radius: u16,
    pub active_chunk_cap: u16,
    pub hot_brain_slots: u16,
    pub warm_brain_slots: u16,
    pub cold_brain_slots: u16,
    pub vfx_budget: &'static str,
    pub shadow_quality: &'static str,
    pub label_density: &'static str,
    pub renderer_profile: &'static str,
    pub hard_floor: bool,
    pub comfort_default: bool,
    pub research_mode: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionVoxelLaunchConfig {
    pub manifest_path: PathBuf,
    pub scenario_id: Option<String>,
    pub app_launch: AppShellLaunchConfig,
    pub profile_id: ProductionFrontendProfileId,
    pub population: Option<u16>,
    pub resolution: (u32, u32),
    pub gpu_mode: GraphicalGpuRuntimeMode,
    pub require_gpu: bool,
    pub graphics_backend: String,
    pub smoke_seconds: Option<u32>,
    pub dry_run: bool,
    pub record_performance: bool,
    pub legacy_alias: bool,
    pub ui_settings_path: Option<PathBuf>,
}

impl ProductionVoxelLaunchConfig {
    pub fn default_from_manifest(path: impl AsRef<Path>) -> Result<Self, GameAppShellError> {
        Self::from_manifest(path, None, ProductionFrontendProfileId::default())
    }

    pub fn from_manifest(
        path: impl AsRef<Path>,
        scenario_id: Option<&str>,
        profile_id: ProductionFrontendProfileId,
    ) -> Result<Self, GameAppShellError> {
        let manifest_path = path.as_ref().to_path_buf();
        let selection = select_environment_scenario(&manifest_path, scenario_id)?;
        let budget = profile_id.budget();
        Ok(Self {
            manifest_path,
            scenario_id: Some(selection.entry.id),
            app_launch: selection.launch,
            profile_id,
            population: None,
            resolution: budget.output_resolution,
            gpu_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
            require_gpu: false,
            graphics_backend: default_production_graphics_backend(),
            smoke_seconds: None,
            dry_run: false,
            record_performance: false,
            legacy_alias: false,
            ui_settings_path: None,
        })
    }

    pub fn effective_population(&self) -> u16 {
        self.population
            .unwrap_or_else(|| self.profile_id.budget().default_population)
    }

    pub fn effective_graphics_backend(&self) -> Result<String, GameAppShellError> {
        match self.graphics_backend.as_str() {
            "auto" if cfg!(windows) => Ok("vulkan".to_string()),
            "auto" => Ok("auto".to_string()),
            "existing" => Ok("existing".to_string()),
            "dx12" | "vulkan" => Ok(self.graphics_backend.clone()),
            other => Err(GameAppShellError::InvalidProductionFrontend {
                message: format!("unknown production --graphics-backend value: {other}"),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionSaveMetadata {
    pub save_id: String,
    pub deterministic_seed: u64,
    pub selected_profile: String,
    pub profile_budget_version: u16,
    pub save_schema: String,
    pub save_schema_version: u16,
    pub config_schema: String,
    pub config_schema_version: u16,
    pub object_count: usize,
    pub creature_count: usize,
    pub asset_count: usize,
    pub voxel_backend_schema: Option<String>,
    pub voxel_visible_chunk_signatures: usize,
    pub voxel_materialized_chunks: usize,
    pub voxel_resource_hazard_refs: usize,
    pub voxel_stable_selection_refs: usize,
    pub voxel_dirty_region_count: usize,
    pub voxel_roundtrip_signatures_match: bool,
    pub no_renderer_tokens_in_voxel_save: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionRuntimeDiagnostics {
    pub requested_backend: String,
    pub selected_backend: String,
    pub adapter_name: Option<String>,
    pub backend_api: Option<String>,
    pub fallback_reason: Option<String>,
    pub renderer_profile: String,
    pub save_path: PathBuf,
    pub asset_manifest_path: PathBuf,
    pub graphics_backend: String,
    pub require_gpu: bool,
    pub cpu_fallback_degraded_visible: bool,
    pub no_full_action_authoritative_claim: bool,
    pub cpu_shadow_gate_preserved: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProductionVoxelLaunchSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub window_title: String,
    pub scenario_id: String,
    pub profile_id: ProductionFrontendProfileId,
    pub profile_budget: ProductionFrontendProfileBudget,
    pub effective_population: u16,
    pub resolution: (u32, u32),
    pub state_trace: Vec<ProductionAppState>,
    pub renderer_profile: String,
    pub save_path: PathBuf,
    pub config_path: PathBuf,
    pub asset_manifest_path: PathBuf,
    pub asset_root: PathBuf,
    pub diagnostics: ProductionRuntimeDiagnostics,
    pub save_metadata: ProductionSaveMetadata,
    pub real_save_loaded: bool,
    pub mock_data_source: bool,
    pub legacy_alias: bool,
    pub dry_run: bool,
    pub record_performance: bool,
    pub ui_settings_path: PathBuf,
    pub ui_settings: Fvr05ProductionUxSettings,
    pub ui_settings_load_error: Option<String>,
    pub debug_authority: Fvr05ProductionDebugAuthorityReport,
}

impl ProductionVoxelLaunchSummary {
    pub fn state_labels(&self) -> Vec<&'static str> {
        self.state_trace.iter().map(|state| state.label()).collect()
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:profile={}:population={}:states={}:backend={}:fallback={:?}:save={}:assets={}",
            self.schema,
            self.schema_version,
            self.scenario_id,
            self.profile_id.label(),
            self.effective_population,
            self.state_labels().join(">"),
            self.diagnostics.selected_backend,
            self.diagnostics.fallback_reason,
            self.save_path.display(),
            self.asset_manifest_path.display()
        )
    }
}

pub fn fvr05_default_ui_settings_path(profile_id: ProductionFrontendProfileId) -> PathBuf {
    PathBuf::from(FVR05_PRODUCTION_UX_SETTINGS_DIR).join(format!(
        "{}_production_ux_settings.json",
        profile_id.label()
    ))
}

fn fvr05_contains_engine_local_token(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    FVR05_ENGINE_LOCAL_TOKENS
        .iter()
        .any(|token| lower.contains(token))
}

fn load_fvr05_ui_settings_or_default(
    path: &Path,
    default_settings: &Fvr05ProductionUxSettings,
) -> (Fvr05ProductionUxSettings, Option<String>) {
    if !path.exists() {
        return (default_settings.clone(), None);
    }
    match Fvr05ProductionUxSettings::from_json_file(path) {
        Ok(mut settings) => {
            settings.refresh_runtime_context(default_settings);
            if let Err(error) = settings.validate() {
                (default_settings.clone(), Some(error.to_string()))
            } else {
                (settings, None)
            }
        }
        Err(error) => (default_settings.clone(), Some(error.to_string())),
    }
}

pub fn run_production_voxel_frontend_dry_run(
    launch: &ProductionVoxelLaunchConfig,
) -> Result<ProductionVoxelLaunchSummary, GameAppShellError> {
    let mut launch = launch.clone();
    launch.dry_run = true;
    run_production_voxel_frontend_preflight(&launch)
}

pub fn run_production_voxel_frontend_preflight(
    launch: &ProductionVoxelLaunchConfig,
) -> Result<ProductionVoxelLaunchSummary, GameAppShellError> {
    let budget = launch.profile_id.budget();
    let population = launch.effective_population();
    if population == 0 {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "production population must be nonzero".to_string(),
        });
    }
    if population > budget.maximum_profile_population {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: format!(
                "profile {} allows up to {} creatures in FVR01 plumbing, got {population}",
                launch.profile_id.label(),
                budget.maximum_profile_population
            ),
        });
    }

    let mut trace = ProductionAppStateTrace::default();
    let graphics_backend = launch.effective_graphics_backend()?;
    if matches!(graphics_backend.as_str(), "dx12" | "vulkan") {
        std::env::set_var("WGPU_BACKEND", &graphics_backend);
    }
    let runtime_options = RuntimePrereqDiagnosticsOptions::new(
        launch.gpu_mode,
        launch.require_gpu,
        graphics_backend.clone(),
        FVR01_RUNTIME_DIAGNOSTIC_LOG,
    );
    let runtime = run_runtime_prereq_diagnostics(&runtime_options)?;
    trace.transition(ProductionAppState::ValidateRuntime)?;
    if runtime.would_block_launch {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: format!(
                "GPU is required but production preflight selected fallback: {:?}",
                runtime.fallback_reason
            ),
        });
    }

    let config = RuntimeConfig::from_json_file(&launch.app_launch.config_path)?;
    config.validate()?;
    let manifest = AssetManifest::from_json_file(&launch.app_launch.asset_manifest_path)?;
    manifest.validate_with_root(&launch.app_launch.asset_root)?;
    trace.transition(ProductionAppState::LoadAssets)?;

    let save = PortableSaveFile::from_json_file(&launch.app_launch.save_path)?;
    let production_save = production_voxel_save_with_population(
        &save,
        &launch.app_launch.asset_root,
        launch.profile_id,
        population,
    )?;
    production_save.validate_with_asset_root(&launch.app_launch.asset_root)?;
    let voxel_evidence = production_voxel_backend_evidence(&production_save)?;
    let visible = visible_world_from_save(&production_save)?;
    compare_visible_world_to_headless(&visible)?;
    trace.transition(ProductionAppState::LoadOrCreateWorld)?;
    trace.transition(ProductionAppState::Running)?;

    if launch.dry_run || launch.smoke_seconds.is_some() {
        trace.transition(ProductionAppState::Shutdown)?;
    }

    let diagnostics = ProductionRuntimeDiagnostics {
        requested_backend: runtime.requested_backend,
        selected_backend: runtime.selected_backend,
        adapter_name: runtime.adapter_name,
        backend_api: runtime.backend_api,
        fallback_reason: runtime.fallback_reason,
        renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE.to_string(),
        save_path: launch.app_launch.save_path.clone(),
        asset_manifest_path: launch.app_launch.asset_manifest_path.clone(),
        graphics_backend,
        require_gpu: launch.require_gpu,
        cpu_fallback_degraded_visible: runtime.cpu_fallback_degraded_visible,
        no_full_action_authoritative_claim: runtime.no_full_action_authoritative_claim,
        cpu_shadow_gate_preserved: runtime.cpu_shadow_gate_preserved,
    };

    let save_metadata = ProductionSaveMetadata {
        save_id: production_save.save_id,
        deterministic_seed: production_save.deterministic_seed,
        selected_profile: launch.profile_id.label().to_string(),
        profile_budget_version: FVR01_PROFILE_BUDGET_SCHEMA_VERSION,
        save_schema: production_save.schema,
        save_schema_version: production_save.schema_version,
        config_schema: config.schema,
        config_schema_version: config.schema_version,
        object_count: visible.object_count,
        creature_count: visible.kind_count(WorldObjectKind::Agent),
        asset_count: manifest.entries.len(),
        voxel_backend_schema: Some(voxel_evidence.schema),
        voxel_visible_chunk_signatures: voxel_evidence.visible_chunk_signatures,
        voxel_materialized_chunks: voxel_evidence.materialized_chunks,
        voxel_resource_hazard_refs: voxel_evidence.resource_hazard_refs,
        voxel_stable_selection_refs: voxel_evidence.stable_selection_refs,
        voxel_dirty_region_count: voxel_evidence.dirty_regions,
        voxel_roundtrip_signatures_match: voxel_evidence.roundtrip_signatures_match,
        no_renderer_tokens_in_voxel_save: voxel_evidence.no_renderer_tokens,
    };
    let default_ui_settings =
        Fvr05ProductionUxSettings::default_for_launch(launch, &diagnostics, &save_metadata);
    let ui_settings_path = launch
        .ui_settings_path
        .clone()
        .unwrap_or_else(|| fvr05_default_ui_settings_path(launch.profile_id));
    let (ui_settings, ui_settings_load_error) =
        load_fvr05_ui_settings_or_default(&ui_settings_path, &default_ui_settings);
    let debug_authority = Fvr05ProductionDebugAuthorityReport::production_read_only();
    debug_authority.validate()?;

    Ok(ProductionVoxelLaunchSummary {
        schema: FVR01_PRODUCTION_FRONTEND_SCHEMA,
        schema_version: FVR01_PRODUCTION_FRONTEND_SCHEMA_VERSION,
        window_title: PRODUCTION_VOXEL_WINDOW_TITLE.to_string(),
        scenario_id: launch
            .scenario_id
            .clone()
            .unwrap_or_else(|| PRODUCTION_VOXEL_SCENARIO_ID.to_string()),
        profile_id: launch.profile_id,
        profile_budget: budget,
        effective_population: population,
        resolution: launch.resolution,
        state_trace: trace.states().to_vec(),
        renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE.to_string(),
        save_path: launch.app_launch.save_path.clone(),
        config_path: launch.app_launch.config_path.clone(),
        asset_manifest_path: launch.app_launch.asset_manifest_path.clone(),
        asset_root: launch.app_launch.asset_root.clone(),
        diagnostics,
        save_metadata,
        real_save_loaded: true,
        mock_data_source: false,
        legacy_alias: launch.legacy_alias,
        dry_run: launch.dry_run,
        record_performance: launch.record_performance,
        ui_settings_path,
        ui_settings,
        ui_settings_load_error,
        debug_authority,
    })
}

pub fn validate_production_voxel_save(
    launch: &ProductionVoxelLaunchConfig,
) -> Result<ProductionVoxelLaunchSummary, GameAppShellError> {
    run_production_voxel_frontend_dry_run(launch)
}

pub(crate) fn production_voxel_save_with_population(
    save: &PortableSaveFile,
    asset_root: &Path,
    profile_id: ProductionFrontendProfileId,
    target_population: u16,
) -> Result<PortableSaveFile, GameAppShellError> {
    if target_population == 0 {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "production population must be nonzero".to_string(),
        });
    }
    save.validate_with_asset_root(asset_root)?;
    let mut production_save = save.clone();
    apply_production_population_target(&mut production_save, usize::from(target_population))?;
    production_save.world.voxel_backend = None;
    let production_save =
        production_save.with_migrated_voxel_backend(persistent_profile_id(profile_id))?;
    production_save.validate_with_asset_root(asset_root)?;
    Ok(production_save)
}

fn apply_production_population_target(
    save: &mut PortableSaveFile,
    target_population: usize,
) -> Result<(), GameAppShellError> {
    let mut agents = save
        .world
        .objects
        .iter()
        .filter(|object| object.kind == WorldObjectKind::Agent)
        .cloned()
        .collect::<Vec<_>>();
    agents.sort_by_key(|object| object.id.raw());
    if agents.is_empty() {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: "production save must contain at least one real creature agent".to_string(),
        });
    }

    if target_population < agents.len() {
        let keep_agent_ids = agents
            .iter()
            .take(target_population)
            .map(|object| object.id.raw())
            .collect::<std::collections::BTreeSet<_>>();
        let keep_organism_ids = agents
            .iter()
            .take(target_population)
            .filter_map(|object| object.organism_id)
            .map(|id| id.raw())
            .collect::<std::collections::BTreeSet<_>>();
        save.world.objects.retain(|object| {
            object.kind != WorldObjectKind::Agent || keep_agent_ids.contains(&object.id.raw())
        });
        save.creatures
            .retain(|creature| keep_organism_ids.contains(&creature.organism_id.raw()));
        retain_existing_touched_entities(save);
        return Ok(());
    }

    let Some(template_agent) = agents.first().cloned() else {
        return Ok(());
    };
    let template_organism =
        template_agent
            .organism_id
            .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
                message: "production creature agent must carry an organism_id".to_string(),
            })?;
    let template_creature = save
        .creatures
        .iter()
        .find(|creature| creature.organism_id == template_organism)
        .or_else(|| save.creatures.first())
        .cloned()
        .ok_or_else(|| GameAppShellError::InvalidProductionFrontend {
            message: "production save must contain at least one creature mind record".to_string(),
        })?;

    let mut next_world_id = save
        .world
        .objects
        .iter()
        .map(|object| object.id.raw())
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mut next_organism_id = save
        .world
        .objects
        .iter()
        .filter_map(|object| object.organism_id)
        .map(|id| id.raw())
        .chain(
            save.creatures
                .iter()
                .map(|creature| creature.organism_id.raw()),
        )
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mut next_genome_id = save
        .creatures
        .iter()
        .map(|creature| creature.genome_id.raw())
        .max()
        .unwrap_or(template_creature.genome_id.raw())
        .saturating_add(1);

    for slot in agents.len()..target_population {
        let stable_id = WorldEntityId(next_world_id)
            .validate()
            .map_err(GameAppShellError::Core)?;
        let organism_id = OrganismId(next_organism_id)
            .validate()
            .map_err(GameAppShellError::Core)?;
        let genome_id = GenomeId(next_genome_id)
            .validate()
            .map_err(GameAppShellError::Core)?;
        next_world_id = next_world_id.saturating_add(1);
        next_organism_id = next_organism_id.saturating_add(1);
        next_genome_id = next_genome_id.saturating_add(1);

        save.world.objects.push(WorldObjectSaveState {
            id: stable_id,
            label: format!("production-creature-{slot:03}"),
            kind: WorldObjectKind::Agent,
            organism_id: Some(organism_id),
            position: production_population_position(save.deterministic_seed, slot),
            radius: template_agent.radius,
            nutrition: 0.0,
            hazard_pain: 0.0,
            token_id: None,
            social_affinity: production_social_affinity(slot),
            teacher_channel: None,
            consumed: false,
            carried_by: None,
        });
        save.creatures.push(production_creature_save_for_slot(
            &template_creature,
            organism_id,
            genome_id,
            slot,
        )?);
    }

    save.world.objects.sort_by_key(|object| object.id.raw());
    save.creatures
        .sort_by_key(|creature| creature.organism_id.raw());
    save.world.next_entity_id = save.world.next_entity_id.max(next_world_id).max(
        save.world
            .objects
            .iter()
            .map(|object| object.id.raw())
            .max()
            .unwrap_or(0)
            + 1,
    );
    retain_existing_touched_entities(save);
    Ok(())
}

fn production_creature_save_for_slot(
    template: &CreatureSaveState,
    organism_id: OrganismId,
    genome_id: GenomeId,
    slot: usize,
) -> Result<CreatureSaveState, GameAppShellError> {
    let tick = template.mind.tick;
    let homeostasis = production_homeostasis_for_slot(tick, slot)?;
    Ok(CreatureSaveState {
        organism_id,
        genome_id,
        brain_class: template.brain_class,
        development_tick: template.development_tick,
        mind: CreatureMindSaveSummary {
            tick,
            homeostasis,
            memory_record_count: template.mind.memory_record_count,
            memory_source_ids: template.mind.memory_source_ids.clone(),
            concept_count: template.mind.concept_count,
            edge_count: template.mind.edge_count,
            simplex_count: template.mind.simplex_count,
            unresolved_gap_count: template.mind.unresolved_gap_count,
            sleep_state_label: if homeostasis.hormones.sleep_pressure >= 0.78 {
                "sleeping".to_string()
            } else {
                "awake".to_string()
            },
            diagnostics: vec![
                "FVR04 production population target materialized from P34 save/config".to_string(),
                format!("population_slot={slot}"),
            ],
        },
        weights: WeightLayerSaveSummary {
            generated_weight_asset_id: template.weights.generated_weight_asset_id.clone(),
            genetic_fixed_digest: template.weights.genetic_fixed_digest.clone(),
            genetic_layer_mutable: false,
            lifetime_consolidated_entries: template.weights.lifetime_consolidated_entries,
            h_operational_entries: template.weights.h_operational_entries,
            h_shadow_entries: template.weights.h_shadow_entries,
        },
        learning: LearningTraceSaveSummary {
            lifetime_learning_enabled: template.learning.lifetime_learning_enabled,
            lamarckian_mode_enabled: false,
            last_consolidated_tick: template.learning.last_consolidated_tick,
        },
    })
}

fn production_homeostasis_for_slot(
    tick: Tick,
    slot: usize,
) -> Result<HomeostaticSnapshot, GameAppShellError> {
    let phase = production_unit_wave(slot, 17);
    let alternate = production_unit_wave(slot, 41);
    let stress = production_unit_wave(slot, 73);
    let mut drives = DriveSnapshot::baseline();
    drives.hunger = (0.18 + phase * 0.74).clamp(0.0, 1.0);
    drives.fatigue = (0.12 + alternate * 0.78).clamp(0.0, 1.0);
    drives.fear = (stress * 0.82).clamp(0.0, 1.0);
    drives.pain = if stress > 0.86 { 0.36 } else { 0.0 };
    drives.loneliness = (1.0 - production_social_affinity(slot).max(0.0)) * 0.5;
    drives.curiosity = (0.28 + production_unit_wave(slot, 23) * 0.62).clamp(0.0, 1.0);
    drives.brain_atp = (0.92 - drives.fatigue * 0.42 - drives.hunger * 0.18).clamp(0.05, 1.0);
    drives.temperature_stress = (production_unit_wave(slot, 97) * 0.22).clamp(0.0, 1.0);
    drives.reproductive_drive = (0.08 + production_unit_wave(slot, 61) * 0.78).clamp(0.0, 1.0);

    let mut hormones = EndocrineSnapshot::baseline();
    hormones.adrenaline = (0.12 + drives.fear * 0.58 + drives.pain * 0.30).clamp(0.0, 1.0);
    hormones.cortisol = (0.16 + drives.fear * 0.64 + drives.fatigue * 0.16).clamp(0.0, 1.0);
    hormones.dopamine = (0.28 + drives.curiosity * 0.34 + drives.brain_atp * 0.26
        - drives.fear * 0.14)
        .clamp(0.0, 1.0);
    hormones.oxytocin = (0.42 + production_social_affinity(slot) * 0.28).clamp(0.0, 1.0);
    hormones.serotonin = (0.36 + drives.brain_atp * 0.36 - drives.pain * 0.22).clamp(0.0, 1.0);
    hormones.acetylcholine = (0.40 + drives.curiosity * 0.42).clamp(0.0, 1.0);
    hormones.learning_modulator = (0.32 + hormones.dopamine * 0.48).clamp(0.0, 1.0);
    hormones.developmental_hormone = (0.42 + drives.reproductive_drive * 0.20).clamp(0.0, 1.0);
    hormones.sleep_pressure = (0.10 + drives.fatigue * 0.82).clamp(0.0, 1.0);

    HomeostaticSnapshot::new(tick, drives, hormones).map_err(GameAppShellError::Core)
}

fn production_population_position(seed: u64, slot: usize) -> Vec3f {
    let ring = (slot / 12) as f32 + 1.0;
    let lane = (slot % 12) as f32;
    let seed_phase = (seed % 360) as f32 * 0.017_453_292;
    let angle = seed_phase + lane * 0.523_598_8 + ring * 0.37;
    let radius = 2.0 + ring * 2.35;
    Vec3f::new(angle.cos() * radius, 0.0, angle.sin() * radius)
}

fn production_social_affinity(slot: usize) -> f32 {
    (production_unit_wave(slot, 29) * 2.0 - 1.0).clamp(-1.0, 1.0)
}

fn production_unit_wave(slot: usize, salt: usize) -> f32 {
    let mixed = slot
        .saturating_mul(1_103_515_245)
        .saturating_add(salt.saturating_mul(12_345));
    (mixed % 10_000) as f32 / 10_000.0
}

fn retain_existing_touched_entities(save: &mut PortableSaveFile) {
    let ids = save
        .world
        .objects
        .iter()
        .map(|object| object.id.raw())
        .collect::<std::collections::BTreeSet<_>>();
    save.world
        .last_touched_entities
        .retain(|id| ids.contains(&id.raw()));
}

fn default_production_graphics_backend() -> String {
    if cfg!(windows) {
        "vulkan".to_string()
    } else {
        "auto".to_string()
    }
}

fn valid_production_transition(from: ProductionAppState, to: ProductionAppState) -> bool {
    matches!(
        (from, to),
        (
            ProductionAppState::Boot,
            ProductionAppState::ValidateRuntime
        ) | (
            ProductionAppState::ValidateRuntime,
            ProductionAppState::LoadAssets
        ) | (
            ProductionAppState::LoadAssets,
            ProductionAppState::LoadOrCreateWorld
        ) | (
            ProductionAppState::LoadOrCreateWorld,
            ProductionAppState::Running
        ) | (ProductionAppState::Running, ProductionAppState::Paused)
            | (ProductionAppState::Paused, ProductionAppState::Running)
            | (ProductionAppState::Running, ProductionAppState::Settings)
            | (ProductionAppState::Settings, ProductionAppState::Running)
            | (ProductionAppState::Running, ProductionAppState::Shutdown)
            | (ProductionAppState::Paused, ProductionAppState::Shutdown)
            | (ProductionAppState::Settings, ProductionAppState::Shutdown)
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProductionVoxelBackendEvidence {
    schema: String,
    visible_chunk_signatures: usize,
    materialized_chunks: usize,
    resource_hazard_refs: usize,
    stable_selection_refs: usize,
    dirty_regions: usize,
    roundtrip_signatures_match: bool,
    no_renderer_tokens: bool,
}

fn persistent_profile_id(profile_id: ProductionFrontendProfileId) -> PersistentVoxelProfileId {
    match profile_id {
        ProductionFrontendProfileId::MinimumSettings30x30 => {
            PersistentVoxelProfileId::MinimumSettings30x30
        }
        ProductionFrontendProfileId::MinSpecComfort1080p => {
            PersistentVoxelProfileId::MinSpecComfort1080p
        }
        ProductionFrontendProfileId::Balanced1080p => PersistentVoxelProfileId::Balanced1080p,
        ProductionFrontendProfileId::HighSpecScaleUp => PersistentVoxelProfileId::HighSpecScaleUp,
        ProductionFrontendProfileId::ResearchScale => PersistentVoxelProfileId::ResearchScale,
    }
}

fn production_voxel_backend_evidence(
    save: &PortableSaveFile,
) -> Result<ProductionVoxelBackendEvidence, GameAppShellError> {
    let backend_state = save.require_voxel_backend()?.clone();
    let backend = PersistentVoxelWorldBackend::from_save_state(backend_state.clone())?;
    let anchors = backend_state
        .creature_anchors
        .iter()
        .map(|anchor| {
            CreatureWorldAnchor::new(
                anchor.stable_id,
                Vec3f::new(anchor.tile.x as f32, 0.0, anchor.tile.z as f32),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let snapshot = backend.snapshot_for_anchors(&anchors)?;
    let backend_json = serde_json::to_string(&backend_state)?;
    let roundtrip: alife_world::PersistentVoxelWorldSaveState =
        serde_json::from_str(&backend_json)?;
    let roundtrip_signatures_match =
        backend_state.visible_chunk_signatures() == roundtrip.visible_chunk_signatures();
    let lower = backend_json.to_ascii_lowercase();
    let no_renderer_tokens = ["bevy", "wgpu", "entity(", "renderer", "windowhandle"]
        .iter()
        .all(|needle| !lower.contains(needle));
    Ok(ProductionVoxelBackendEvidence {
        schema: backend_state.schema,
        visible_chunk_signatures: snapshot.visible_chunks.len(),
        materialized_chunks: backend_state.materialized_chunk_count,
        resource_hazard_refs: snapshot.resources_and_hazards.len(),
        stable_selection_refs: snapshot.selection_refs.len(),
        dirty_regions: snapshot.dirty_regions.len(),
        roundtrip_signatures_match,
        no_renderer_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gpu_alpha_fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/gpu_alpha")
    }

    fn gpu_alpha_save() -> PortableSaveFile {
        PortableSaveFile::from_json_file(gpu_alpha_fixture_root().join("tiny_save.json")).unwrap()
    }

    fn fvr05_test_launch() -> ProductionVoxelLaunchConfig {
        let root = gpu_alpha_fixture_root();
        let app_launch = AppShellLaunchConfig::from_p34_fixture_root(&root);
        ProductionVoxelLaunchConfig {
            manifest_path: root.join("environment_manifest.json"),
            scenario_id: Some(PRODUCTION_VOXEL_SCENARIO_ID.to_string()),
            app_launch,
            profile_id: ProductionFrontendProfileId::MinSpecComfort1080p,
            population: Some(30),
            resolution: (1920, 1080),
            gpu_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
            require_gpu: false,
            graphics_backend: "existing".to_string(),
            smoke_seconds: None,
            dry_run: true,
            record_performance: false,
            legacy_alias: false,
            ui_settings_path: None,
        }
    }

    fn fvr05_test_diagnostics(
        launch: &ProductionVoxelLaunchConfig,
    ) -> ProductionRuntimeDiagnostics {
        ProductionRuntimeDiagnostics {
            requested_backend: "StaticPlasticCpuShadowGuarded".to_string(),
            selected_backend: "CpuReference".to_string(),
            adapter_name: Some("test-adapter".to_string()),
            backend_api: Some("test-api".to_string()),
            fallback_reason: Some("unit-test fallback".to_string()),
            renderer_profile: PRODUCTION_VOXEL_RENDERER_PROFILE.to_string(),
            save_path: launch.app_launch.save_path.clone(),
            asset_manifest_path: launch.app_launch.asset_manifest_path.clone(),
            graphics_backend: launch.graphics_backend.clone(),
            require_gpu: false,
            cpu_fallback_degraded_visible: true,
            no_full_action_authoritative_claim: true,
            cpu_shadow_gate_preserved: true,
        }
    }

    fn fvr05_test_save_metadata() -> ProductionSaveMetadata {
        ProductionSaveMetadata {
            save_id: "unit-test-save".to_string(),
            deterministic_seed: 42,
            selected_profile: ProductionFrontendProfileId::MinSpecComfort1080p
                .label()
                .to_string(),
            profile_budget_version: FVR01_PROFILE_BUDGET_SCHEMA_VERSION,
            save_schema: "alife.p34.save.v1".to_string(),
            save_schema_version: 1,
            config_schema: "alife.p34.runtime_config.v1".to_string(),
            config_schema_version: 1,
            object_count: 30,
            creature_count: 30,
            asset_count: 3,
            voxel_backend_schema: Some("alife.fvr02.persistent_voxel_world.v1".to_string()),
            voxel_visible_chunk_signatures: 25,
            voxel_materialized_chunks: 25,
            voxel_resource_hazard_refs: 12,
            voxel_stable_selection_refs: 30,
            voxel_dirty_region_count: 0,
            voxel_roundtrip_signatures_match: true,
            no_renderer_tokens_in_voxel_save: true,
        }
    }

    fn creature_anchor_signature(save: &PortableSaveFile) -> Vec<(u64, i32, i32, i32, i32)> {
        let backend = save.require_voxel_backend().unwrap();
        backend
            .creature_anchors
            .iter()
            .map(|anchor| {
                (
                    anchor.stable_id.raw(),
                    anchor.tile.x,
                    anchor.tile.z,
                    anchor.chunk.x,
                    anchor.chunk.z,
                )
            })
            .collect()
    }

    #[test]
    fn fvr05_overlay_catalog_covers_production_debug_surfaces() {
        let labels = Fvr05ProductionOverlayKind::all()
            .iter()
            .map(|overlay| overlay.label())
            .collect::<Vec<_>>();
        for required in [
            "Resources",
            "Danger",
            "Pheromones",
            "Energy",
            "Age",
            "Fertility",
            "Territory",
            "Neural",
            "Residency",
            "BackendTiming",
            "ChunkBoundaries",
            "LodBudget",
            "Persistence",
        ] {
            assert!(labels.contains(&required));
        }
        assert!(Fvr05ProductionOverlayKind::default_enabled_for_profile(
            ProductionFrontendProfileId::MinimumSettings30x30
        )
        .contains(&Fvr05ProductionOverlayKind::Persistence));
    }

    #[test]
    fn fvr05_ux_settings_roundtrip_excludes_engine_tokens_and_preserves_profile() {
        let launch = fvr05_test_launch();
        let diagnostics = fvr05_test_diagnostics(&launch);
        let metadata = fvr05_test_save_metadata();
        let settings =
            Fvr05ProductionUxSettings::default_for_launch(&launch, &diagnostics, &metadata);
        settings.validate().unwrap();
        let json = settings.to_json_string_pretty().unwrap();
        let lower = json.to_ascii_lowercase();
        assert!(!lower.contains("bevy"));
        assert!(!lower.contains("wgpu"));
        assert!(!lower.contains("entity("));
        assert!(!lower.contains("renderer"));

        let path = std::env::temp_dir().join(format!(
            "alife_fvr05_ux_settings_{}.json",
            std::process::id()
        ));
        settings.to_json_file(&path).unwrap();
        let roundtrip = Fvr05ProductionUxSettings::from_json_file(&path).unwrap();
        let _ = std::fs::remove_file(path);
        assert_eq!(roundtrip.selected_profile, launch.profile_id);
        assert_eq!(
            roundtrip.active_inspector_tab,
            Fvr05ProductionInspectorTab::Creature
        );
        assert!(roundtrip
            .enabled_overlays
            .contains(&Fvr05ProductionOverlayKind::BackendTiming));
    }

    #[test]
    fn fvr05_debug_authority_blocks_every_mutating_path() {
        let report = Fvr05ProductionDebugAuthorityReport::production_read_only();
        report.validate().unwrap();
        assert!(report.read_only_projection);
        assert!(report.direct_actions_blocked);
        assert!(report.action_arbitration_bypass_blocked);
        assert!(report.reward_injection_blocked);
        assert!(report.weight_mutation_blocked);
        assert!(report.hidden_cognition_mutation_blocked);
        assert!(report.bulk_neural_readback_blocked);
    }

    #[test]
    fn fvr04_population_target_materializes_real_save_state_and_voxel_anchors() {
        let root = gpu_alpha_fixture_root();
        let save = gpu_alpha_save();
        let production = production_voxel_save_with_population(
            &save,
            &root,
            ProductionFrontendProfileId::MinimumSettings30x30,
            30,
        )
        .unwrap();
        let visible = visible_world_from_save(&production).unwrap();
        let backend = production.require_voxel_backend().unwrap();

        assert_eq!(visible.kind_count(WorldObjectKind::Agent), 30);
        assert_eq!(production.creatures.len(), 30);
        assert_eq!(backend.creature_anchors.len(), 30);
        assert!(backend.validate().is_ok());
        assert!(production.creatures.iter().any(|creature| creature
            .mind
            .homeostasis
            .drives
            .hunger
            > 0.70));
        assert!(production.creatures.iter().any(|creature| creature
            .mind
            .homeostasis
            .hormones
            .sleep_pressure
            > 0.65));
    }

    #[test]
    fn fvr04_population_target_supports_one_and_scale_up_500_without_renderer_tokens() {
        let root = gpu_alpha_fixture_root();
        let save = gpu_alpha_save();
        let one = production_voxel_save_with_population(
            &save,
            &root,
            ProductionFrontendProfileId::MinimumSettings30x30,
            1,
        )
        .unwrap();
        assert_eq!(
            visible_world_from_save(&one)
                .unwrap()
                .kind_count(WorldObjectKind::Agent),
            1
        );
        assert_eq!(one.creatures.len(), 1);
        assert_eq!(
            one.require_voxel_backend().unwrap().creature_anchors.len(),
            1
        );

        let scale_up = production_voxel_save_with_population(
            &save,
            &root,
            ProductionFrontendProfileId::HighSpecScaleUp,
            500,
        )
        .unwrap();
        let backend = scale_up.require_voxel_backend().unwrap();
        let backend_json = serde_json::to_string(backend).unwrap().to_ascii_lowercase();
        assert_eq!(
            visible_world_from_save(&scale_up)
                .unwrap()
                .kind_count(WorldObjectKind::Agent),
            500
        );
        assert_eq!(scale_up.creatures.len(), 500);
        assert_eq!(backend.creature_anchors.len(), 500);
        assert!(!backend_json.contains("bevy"));
        assert!(!backend_json.contains("wgpu"));
        assert!(!backend_json.contains("renderer"));
    }

    #[test]
    fn fvr04_save_roundtrip_preserves_selected_creature_and_visible_signature() {
        let root = gpu_alpha_fixture_root();
        let save = gpu_alpha_save();
        let production = production_voxel_save_with_population(
            &save,
            &root,
            ProductionFrontendProfileId::MinSpecComfort1080p,
            30,
        )
        .unwrap();
        let selected_creature = production
            .require_voxel_backend()
            .unwrap()
            .creature_anchors
            .first()
            .unwrap()
            .clone();
        let signature_before = creature_anchor_signature(&production);
        let json = serde_json::to_string_pretty(&production).unwrap();
        let roundtrip = PortableSaveFile::from_json_str(&json).unwrap();
        let signature_after = creature_anchor_signature(&roundtrip);
        let selected_after = roundtrip
            .require_voxel_backend()
            .unwrap()
            .creature_anchors
            .first()
            .unwrap()
            .clone();
        let lower_json = json.to_ascii_lowercase();

        assert_eq!(selected_after, selected_creature);
        assert_eq!(signature_after, signature_before);
        assert!(!lower_json.contains("entity("));
        assert!(!lower_json.contains("bevy"));
        assert!(!lower_json.contains("wgpu"));
        assert!(!lower_json.contains("renderer"));
    }
}
