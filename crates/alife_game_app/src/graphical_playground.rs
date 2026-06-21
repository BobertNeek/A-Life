//! S01 persistent graphical playground launch contract.
//!
//! This module is deliberately Bevy-free so the default headless CI path can
//! validate graphical launcher configuration without opening a window.

use crate::prelude::*;
use crate::*;

pub const S01_GRAPHICAL_WINDOW_TITLE: &str = "A-Life Graphical Playground";
pub const S01_DEFAULT_FIXTURE_ROOT: &str = "crates/alife_world/tests/fixtures/p34";

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphicalPlaygroundLaunchConfig {
    pub app_launch: AppShellLaunchConfig,
    pub mode: GraphicalPlaygroundMode,
    pub window_title: String,
}

impl GraphicalPlaygroundLaunchConfig {
    pub fn interactive(fixture_root: impl AsRef<Path>) -> Self {
        Self {
            app_launch: AppShellLaunchConfig::from_p34_fixture_root(fixture_root),
            mode: GraphicalPlaygroundMode::Interactive,
            window_title: S01_GRAPHICAL_WINDOW_TITLE.to_string(),
        }
    }

    pub fn smoke(fixture_root: impl AsRef<Path>, seconds: u32) -> Self {
        Self {
            app_launch: AppShellLaunchConfig::from_p34_fixture_root(fixture_root),
            mode: GraphicalPlaygroundMode::Smoke { seconds },
            window_title: format!("{S01_GRAPHICAL_WINDOW_TITLE} - smoke {seconds}s"),
        }
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
    pub cpu_fallback_visible: bool,
    pub stable_id_overlay_visible: bool,
    pub object_count: usize,
    pub creature_marker_count: usize,
    pub food_marker_count: usize,
    pub visible_signature: Vec<String>,
}

impl GraphicalPlaygroundLaunchSummary {
    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:objects={}:creatures={}:food={}:backend={:?}:persistent={}:timeout={:?}",
            self.schema,
            self.schema_version,
            self.mode_label,
            self.object_count,
            self.creature_marker_count,
            self.food_marker_count,
            self.selected_backend,
            self.persistent_window,
            self.smoke_seconds
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
        cpu_fallback_visible: true,
        stable_id_overlay_visible: true,
        object_count: presentation.object_count,
        creature_marker_count: presentation.kind_count(WorldObjectKind::Agent),
        food_marker_count: presentation.kind_count(WorldObjectKind::Food),
        visible_signature: presentation.visible_signature,
    })
}
