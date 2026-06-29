//! S01 persistent graphical playground launch contract.
//!
//! This module is deliberately Bevy-free so the default headless CI path can
//! validate graphical launcher configuration without opening a window.

use crate::prelude::*;
use crate::*;

pub const S01_GRAPHICAL_WINDOW_TITLE: &str = "A-Life GPU Alpha Playground";
pub const S01_DEFAULT_FIXTURE_ROOT: &str = "crates/alife_world/tests/fixtures/gpu_alpha";

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GraphicalGpuRuntimeMode {
    #[default]
    CpuReference,
    StaticPlasticCpuShadowGuarded,
    AutoWithCpuFallback,
}

impl GraphicalGpuRuntimeMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CpuReference => "cpu-reference",
            Self::StaticPlasticCpuShadowGuarded => "static-plastic-cpu-shadow-guarded",
            Self::AutoWithCpuFallback => "auto-with-cpu-fallback",
        }
    }

    pub const fn requests_gpu(self) -> bool {
        !matches!(self, Self::CpuReference)
    }

    pub fn parse(value: &str) -> Result<Self, GameAppShellError> {
        match value {
            "cpu-reference" => Ok(Self::CpuReference),
            "static-plastic-cpu-shadow-guarded" => Ok(Self::StaticPlasticCpuShadowGuarded),
            "auto-with-cpu-fallback" => Ok(Self::AutoWithCpuFallback),
            _ => Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "graphical GPU mode must be cpu-reference, static-plastic-cpu-shadow-guarded, or auto-with-cpu-fallback",
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicalPlaygroundMode {
    Interactive,
    Smoke { seconds: u32 },
}

impl GraphicalPlaygroundMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Smoke { .. } => "smoke-timeout",
        }
    }

    pub const fn smoke_seconds(self) -> Option<u32> {
        match self {
            Self::Interactive => None,
            Self::Smoke { seconds } => Some(seconds),
        }
    }

    pub const fn persistent_window(self) -> bool {
        matches!(self, Self::Interactive)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum GraphicalPlaygroundViewMode {
    #[default]
    Player,
    DevOverlay,
    FullDebug,
}

impl GraphicalPlaygroundViewMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::DevOverlay => "dev-overlay",
            Self::FullDebug => "full-debug",
        }
    }

    pub const fn dev_overlay_visible(self) -> bool {
        matches!(self, Self::DevOverlay | Self::FullDebug)
    }

    pub const fn full_debug_visible(self) -> bool {
        matches!(self, Self::FullDebug)
    }

    pub const fn world_labels_visible(self) -> bool {
        matches!(self, Self::DevOverlay | Self::FullDebug)
    }

    pub const fn topology_lines_visible(self) -> bool {
        matches!(self, Self::DevOverlay | Self::FullDebug)
    }

    pub const fn teacher_debug_labels_visible(self) -> bool {
        matches!(self, Self::FullDebug)
    }

    pub fn parse(value: &str) -> Result<Self, GameAppShellError> {
        match value {
            "player" => Ok(Self::Player),
            "dev-overlay" => Ok(Self::DevOverlay),
            "full-debug" => Ok(Self::FullDebug),
            _ => Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "graphical view mode must be player, dev-overlay, or full-debug",
            }),
        }
    }
}

pub const CA42A_MAX_PLAYER_TERRAIN_OVERLAY_ALPHA: f32 = 0.015;

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicalPlayerViewAcceptanceSummary {
    pub view_mode: GraphicalPlaygroundViewMode,
    pub dev_overlay_hidden: bool,
    pub full_debug_hidden: bool,
    pub event_feed_collapsed: bool,
    pub stable_id_labels_hidden_except_selected: bool,
    pub terrain_overlay_max_opacity: f32,
    pub internal_patch_gpu_claim_spam_hidden: bool,
    pub topology_lines_hidden: bool,
    pub teacher_debug_labels_hidden_unless_school: bool,
}

impl GraphicalPlayerViewAcceptanceSummary {
    pub const fn for_view_mode(view_mode: GraphicalPlaygroundViewMode) -> Self {
        Self {
            view_mode,
            dev_overlay_hidden: !view_mode.dev_overlay_visible(),
            full_debug_hidden: !view_mode.full_debug_visible(),
            event_feed_collapsed: !view_mode.full_debug_visible(),
            stable_id_labels_hidden_except_selected: !view_mode.world_labels_visible(),
            terrain_overlay_max_opacity: CA42A_MAX_PLAYER_TERRAIN_OVERLAY_ALPHA,
            internal_patch_gpu_claim_spam_hidden: !view_mode.full_debug_visible(),
            topology_lines_hidden: !view_mode.topology_lines_visible(),
            teacher_debug_labels_hidden_unless_school: !view_mode.teacher_debug_labels_visible(),
        }
    }

    pub fn signature_line(&self) -> String {
        format!(
            "view_mode={} dev_overlay_hidden={} full_debug_hidden={} event_feed_collapsed={} stable_labels_hidden={} terrain_alpha_max={:.3} internal_spam_hidden={} topology_lines_hidden={} teacher_debug_hidden={}",
            self.view_mode.label(),
            self.dev_overlay_hidden,
            self.full_debug_hidden,
            self.event_feed_collapsed,
            self.stable_id_labels_hidden_except_selected,
            self.terrain_overlay_max_opacity,
            self.internal_patch_gpu_claim_spam_hidden,
            self.topology_lines_hidden,
            self.teacher_debug_labels_hidden_unless_school,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicalPlaygroundLaunchConfig {
    pub app_launch: AppShellLaunchConfig,
    pub mode: GraphicalPlaygroundMode,
    pub gpu_mode: GraphicalGpuRuntimeMode,
    pub view_mode: GraphicalPlaygroundViewMode,
    pub window_title: String,
    pub require_gpu: bool,
}

impl GraphicalPlaygroundLaunchConfig {
    pub fn interactive(fixture_root: impl AsRef<Path>) -> Self {
        Self {
            app_launch: AppShellLaunchConfig::from_p34_fixture_root(fixture_root),
            mode: GraphicalPlaygroundMode::Interactive,
            gpu_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
            view_mode: GraphicalPlaygroundViewMode::Player,
            window_title: S01_GRAPHICAL_WINDOW_TITLE.to_string(),
            require_gpu: false,
        }
    }

    pub fn smoke(fixture_root: impl AsRef<Path>, seconds: u32) -> Self {
        Self {
            app_launch: AppShellLaunchConfig::from_p34_fixture_root(fixture_root),
            mode: GraphicalPlaygroundMode::Smoke { seconds },
            gpu_mode: GraphicalGpuRuntimeMode::StaticPlasticCpuShadowGuarded,
            view_mode: GraphicalPlaygroundViewMode::Player,
            window_title: format!("{S01_GRAPHICAL_WINDOW_TITLE} - smoke {seconds}s"),
            require_gpu: false,
        }
    }

    pub const fn with_gpu_mode(mut self, gpu_mode: GraphicalGpuRuntimeMode) -> Self {
        self.gpu_mode = gpu_mode;
        self
    }

    pub const fn with_view_mode(mut self, view_mode: GraphicalPlaygroundViewMode) -> Self {
        self.view_mode = view_mode;
        self
    }

    pub const fn require_gpu(mut self, require_gpu: bool) -> Self {
        self.require_gpu = require_gpu;
        self
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if !self.window_title.contains("A-Life") {
            return Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "graphical playground window title must contain A-Life",
            });
        }
        if let Some(seconds) = self.mode.smoke_seconds() {
            if seconds == 0 || seconds > S01_MAX_GRAPHICAL_SMOKE_SECONDS {
                return Err(GameAppShellError::InvalidGraphicalLaunch {
                    message: "graphical smoke seconds must be in 1..=120",
                });
            }
        }
        if self.require_gpu && !self.gpu_mode.requests_gpu() {
            return Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "RequireGpu needs a GPU runtime mode, not cpu-reference",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicalPlaygroundLaunchSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub window_title: String,
    pub mode_label: &'static str,
    pub smoke_seconds: Option<u32>,
    pub persistent_window: bool,
    pub fixture_root: PathBuf,
    pub seed: u64,
    pub selected_backend: BackendSelection,
    pub requested_gpu_mode: GraphicalGpuRuntimeMode,
    pub view_mode: GraphicalPlaygroundViewMode,
    pub require_gpu: bool,
    pub gpu_mode_visible: bool,
    pub cpu_fallback_visible: bool,
    pub stable_id_overlay_visible: bool,
    pub player_view_acceptance: GraphicalPlayerViewAcceptanceSummary,
    pub object_count: usize,
    pub creature_marker_count: usize,
    pub food_marker_count: usize,
    pub hazard_marker_count: usize,
    pub visible_signature: Vec<String>,
}

impl GraphicalPlaygroundLaunchSummary {
    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:objects={}:creatures={}:food={}:hazards={}:backend={:?}:gpu_mode={}:view_mode={}:require_gpu={}:persistent={}:timeout={:?}:{}",
            self.schema,
            self.schema_version,
            self.mode_label,
            self.object_count,
            self.creature_marker_count,
            self.food_marker_count,
            self.hazard_marker_count,
            self.selected_backend,
            self.requested_gpu_mode.label(),
            self.view_mode.label(),
            self.require_gpu,
            self.persistent_window,
            self.smoke_seconds,
            self.player_view_acceptance.signature_line(),
        )
    }
}

pub fn validate_graphical_playground_launch(
    launch: &GraphicalPlaygroundLaunchConfig,
) -> Result<GraphicalPlaygroundLaunchSummary, GameAppShellError> {
    launch.validate()?;
    let startup = run_headless_app_shell_smoke(&launch.app_launch)?;
    let presentation = load_visible_world_from_p34_save(&launch.app_launch)?;
    compare_visible_world_to_headless(&presentation)?;

    Ok(GraphicalPlaygroundLaunchSummary {
        schema: S01_GRAPHICAL_PLAYGROUND_SCHEMA,
        schema_version: S01_GRAPHICAL_PLAYGROUND_SCHEMA_VERSION,
        window_title: launch.window_title.clone(),
        mode_label: launch.mode.label(),
        smoke_seconds: launch.mode.smoke_seconds(),
        persistent_window: launch.mode.persistent_window(),
        fixture_root: launch.app_launch.fixture_root.clone(),
        seed: startup.seed,
        selected_backend: startup.requested_backend,
        requested_gpu_mode: launch.gpu_mode,
        view_mode: launch.view_mode,
        require_gpu: launch.require_gpu,
        gpu_mode_visible: true,
        cpu_fallback_visible: true,
        stable_id_overlay_visible: launch.view_mode.world_labels_visible(),
        player_view_acceptance: GraphicalPlayerViewAcceptanceSummary::for_view_mode(
            launch.view_mode,
        ),
        object_count: presentation.object_count,
        creature_marker_count: presentation.kind_count(WorldObjectKind::Agent),
        food_marker_count: presentation.kind_count(WorldObjectKind::Food),
        hazard_marker_count: presentation.kind_count(WorldObjectKind::Hazard),
        visible_signature: presentation.visible_signature,
    })
}
