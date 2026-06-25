//! CA10 versioned environment manifest launcher.
//!
//! This module stays Bevy-free. It resolves player-facing scenario IDs into
//! existing P34-compatible fixture roots and validates those roots through the
//! normal persistence/config contracts.

use std::collections::BTreeSet;

use crate::prelude::*;
use crate::*;

pub const CA10_DEFAULT_ENVIRONMENT_MANIFEST: &str = "environment_manifest.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentManifest {
    pub schema: String,
    pub schema_version: u16,
    pub default_scenario_id: String,
    pub scenarios: Vec<EnvironmentScenarioEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvironmentScenarioEntry {
    pub id: String,
    pub title: String,
    pub description: String,
    pub fixture_root: PathBuf,
    pub config_file: String,
    pub asset_manifest_file: String,
    pub save_file: String,
    pub player_visible: bool,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentScenarioSelection {
    pub manifest_path: PathBuf,
    pub entry: EnvironmentScenarioEntry,
    pub launch: AppShellLaunchConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentLauncherSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub manifest_path: PathBuf,
    pub default_scenario_id: String,
    pub selected_scenario_id: String,
    pub title: String,
    pub scenario_count: usize,
    pub fixture_root: PathBuf,
    pub seed: u64,
    pub asset_count: usize,
    pub object_count: usize,
    pub creature_count: usize,
    pub food_count: usize,
    pub hazard_count: usize,
    pub obstacle_count: usize,
    pub player_visible_error_sample: String,
}

impl EnvironmentLauncherSummary {
    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:scenarios={}:seed={}:objects={}:creatures={}:food={}:hazards={}:obstacles={}",
            self.schema,
            self.schema_version,
            self.selected_scenario_id,
            self.scenario_count,
            self.seed,
            self.object_count,
            self.creature_count,
            self.food_count,
            self.hazard_count,
            self.obstacle_count
        )
    }
}

impl EnvironmentManifest {
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, GameAppShellError> {
        let text = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }

    pub fn validate(&self, manifest_path: &Path) -> Result<(), GameAppShellError> {
        if self.schema != CA10_ENVIRONMENT_MANIFEST_SCHEMA {
            return Err(GameAppShellError::EnvironmentManifest {
                message: format!(
                    "environment manifest schema must be {}, got {}",
                    CA10_ENVIRONMENT_MANIFEST_SCHEMA, self.schema
                ),
            });
        }
        if self.schema_version != CA10_ENVIRONMENT_MANIFEST_SCHEMA_VERSION {
            return Err(GameAppShellError::EnvironmentManifest {
                message: format!(
                    "environment manifest version must be {}, got {}",
                    CA10_ENVIRONMENT_MANIFEST_SCHEMA_VERSION, self.schema_version
                ),
            });
        }
        if self.scenarios.is_empty() {
            return Err(GameAppShellError::EnvironmentManifest {
                message: "environment manifest must list at least one scenario".to_string(),
            });
        }
        if self.scenarios.len() > CA10_MAX_ENVIRONMENT_SCENARIOS {
            return Err(GameAppShellError::EnvironmentManifest {
                message: format!(
                    "environment manifest has {} scenarios; maximum is {}",
                    self.scenarios.len(),
                    CA10_MAX_ENVIRONMENT_SCENARIOS
                ),
            });
        }

        let mut ids = BTreeSet::new();
        for scenario in &self.scenarios {
            scenario.validate(manifest_path)?;
            if !ids.insert(scenario.id.as_str()) {
                return Err(GameAppShellError::EnvironmentManifest {
                    message: format!("duplicate environment scenario id '{}'", scenario.id),
                });
            }
        }
        if !ids.contains(self.default_scenario_id.as_str()) {
            return Err(GameAppShellError::EnvironmentManifest {
                message: format!(
                    "default environment scenario '{}' is not listed",
                    self.default_scenario_id
                ),
            });
        }
        Ok(())
    }

    pub fn select(
        &self,
        manifest_path: impl AsRef<Path>,
        scenario_id: Option<&str>,
    ) -> Result<EnvironmentScenarioSelection, GameAppShellError> {
        let manifest_path = manifest_path.as_ref();
        self.validate(manifest_path)?;
        let requested = scenario_id.unwrap_or(&self.default_scenario_id);
        let entry = self
            .scenarios
            .iter()
            .find(|scenario| scenario.id == requested)
            .ok_or_else(|| GameAppShellError::EnvironmentManifest {
                message: format!(
                    "unknown environment scenario '{requested}'. Known scenarios: {}",
                    self.scenario_ids().join(", ")
                ),
            })?
            .clone();
        let fixture_root = entry.absolute_fixture_root(manifest_path);
        let launch = AppShellLaunchConfig {
            config_path: fixture_root.join(&entry.config_file),
            asset_manifest_path: fixture_root.join(&entry.asset_manifest_file),
            save_path: fixture_root.join(&entry.save_file),
            asset_root: fixture_root.clone(),
            fixture_root,
            start_paused: false,
        };
        validate_app_shell_config(&launch)?;
        Ok(EnvironmentScenarioSelection {
            manifest_path: manifest_path.to_path_buf(),
            entry,
            launch,
        })
    }

    pub fn scenario_ids(&self) -> Vec<String> {
        self.scenarios
            .iter()
            .map(|scenario| scenario.id.clone())
            .collect()
    }
}

impl EnvironmentScenarioEntry {
    fn validate(&self, manifest_path: &Path) -> Result<(), GameAppShellError> {
        validate_scenario_id(&self.id)?;
        if self.title.trim().is_empty() {
            return Err(GameAppShellError::EnvironmentManifest {
                message: format!("environment scenario '{}' needs a title", self.id),
            });
        }
        if self.description.trim().is_empty() {
            return Err(GameAppShellError::EnvironmentManifest {
                message: format!("environment scenario '{}' needs a description", self.id),
            });
        }
        for (label, value) in [
            ("config_file", &self.config_file),
            ("asset_manifest_file", &self.asset_manifest_file),
            ("save_file", &self.save_file),
        ] {
            if value.trim().is_empty() || value.contains('/') || value.contains('\\') {
                return Err(GameAppShellError::EnvironmentManifest {
                    message: format!(
                        "environment scenario '{}' has invalid {label}; use a fixture-local file name",
                        self.id
                    ),
                });
            }
        }
        let root = self.absolute_fixture_root(manifest_path);
        for required in [
            root.join(&self.config_file),
            root.join(&self.asset_manifest_file),
            root.join(&self.save_file),
        ] {
            if !required.exists() {
                return Err(GameAppShellError::EnvironmentManifest {
                    message: format!(
                        "environment scenario '{}' references missing file {}",
                        self.id,
                        required.display()
                    ),
                });
            }
        }
        Ok(())
    }

    fn absolute_fixture_root(&self, manifest_path: &Path) -> PathBuf {
        let root = if self.fixture_root.is_absolute() {
            self.fixture_root.clone()
        } else {
            manifest_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(&self.fixture_root)
        };
        normalize_path(root)
    }
}

pub fn default_environment_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CA10_DEFAULT_ENVIRONMENT_MANIFEST)
}

pub fn load_default_environment_manifest() -> Result<EnvironmentManifest, GameAppShellError> {
    EnvironmentManifest::from_json_file(default_environment_manifest_path())
}

pub fn select_environment_scenario(
    manifest_path: impl AsRef<Path>,
    scenario_id: Option<&str>,
) -> Result<EnvironmentScenarioSelection, GameAppShellError> {
    let manifest = EnvironmentManifest::from_json_file(manifest_path.as_ref())?;
    manifest.select(manifest_path, scenario_id)
}

pub fn run_environment_launcher_smoke(
    manifest_path: impl AsRef<Path>,
    scenario_id: Option<&str>,
) -> Result<EnvironmentLauncherSummary, GameAppShellError> {
    let manifest_path = manifest_path.as_ref();
    let manifest = EnvironmentManifest::from_json_file(manifest_path)?;
    let selection = manifest.select(manifest_path, scenario_id)?;
    let startup = run_headless_app_shell_smoke(&selection.launch)?;
    let visible = load_visible_world_from_p34_save(&selection.launch)?;
    compare_visible_world_to_headless(&visible)?;

    Ok(EnvironmentLauncherSummary {
        schema: CA10_ENVIRONMENT_MANIFEST_SCHEMA,
        schema_version: CA10_ENVIRONMENT_MANIFEST_SCHEMA_VERSION,
        manifest_path: manifest_path.to_path_buf(),
        default_scenario_id: manifest.default_scenario_id,
        selected_scenario_id: selection.entry.id,
        title: selection.entry.title,
        scenario_count: manifest.scenarios.len(),
        fixture_root: selection.launch.fixture_root,
        seed: startup.seed,
        asset_count: startup.asset_count,
        object_count: visible.object_count,
        creature_count: visible.kind_count(WorldObjectKind::Agent),
        food_count: visible.kind_count(WorldObjectKind::Food),
        hazard_count: visible.kind_count(WorldObjectKind::Hazard),
        obstacle_count: visible.kind_count(WorldObjectKind::Obstacle),
        player_visible_error_sample: "Unknown scenario. Pick one of the listed alpha scenarios."
            .to_string(),
    })
}

fn validate_scenario_id(id: &str) -> Result<(), GameAppShellError> {
    let valid = !id.is_empty()
        && id.len() <= 48
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if valid {
        Ok(())
    } else {
        Err(GameAppShellError::EnvironmentManifest {
            message: format!(
                "invalid environment scenario id '{id}'; use lowercase letters, digits, and '-'"
            ),
        })
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}
