//! FVR01 production voxel frontend launch policy.
//!
//! This module wires production profile budgets, launch states, and diagnostics
//! around the existing real P34 config/save/asset contracts. It does not move
//! renderer, Bevy, or GPU handles into core/world state.

use crate::prelude::*;
use crate::*;

pub const PRODUCTION_VOXEL_COMMAND: &str = "production-voxel";
pub const PRODUCTION_VOXEL_WINDOW_TITLE: &str = "A-Life Voxel Frontend";
pub const PRODUCTION_VOXEL_RENDERER_PROFILE: &str = "voxel-backend";
pub const PRODUCTION_VOXEL_SCENARIO_ID: &str = "production-voxel";
pub const FVR01_RUNTIME_DIAGNOSTIC_LOG: &str =
    "target/artifacts/fvr01_production_voxel/runtime_prereq.log";

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
        })
    }

    pub fn effective_population(&self) -> u16 {
        self.population
            .unwrap_or_else(|| self.profile_id.budget().default_population)
    }

    pub fn effective_graphics_backend(&self) -> Result<String, GameAppShellError> {
        match self.graphics_backend.as_str() {
            "auto" if cfg!(windows) => Ok("dx12".to_string()),
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
    save.validate_with_asset_root(&launch.app_launch.asset_root)?;
    let production_save =
        save.with_migrated_voxel_backend(persistent_profile_id(launch.profile_id))?;
    production_save.validate_with_asset_root(&launch.app_launch.asset_root)?;
    let voxel_evidence = production_voxel_backend_evidence(&production_save)?;
    let visible = load_visible_world_from_p34_save(&launch.app_launch)?;
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
    })
}

pub fn validate_production_voxel_save(
    launch: &ProductionVoxelLaunchConfig,
) -> Result<ProductionVoxelLaunchSummary, GameAppShellError> {
    run_production_voxel_frontend_dry_run(launch)
}

fn default_production_graphics_backend() -> String {
    if cfg!(windows) {
        "dx12".to_string()
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
