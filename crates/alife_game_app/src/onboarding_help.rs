//! G20 onboarding help text and tutorial metadata.
//!
//! This module is intentionally descriptive: it exposes versioned first-run,
//! controls, troubleshooting, and tutorial references without changing runtime
//! behavior or making optional graphics/GPU/provider paths mandatory.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HelpControlReference {
    pub label: &'static str,
    pub action: &'static str,
    pub source_plan: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TroubleshootingReference {
    pub symptom: &'static str,
    pub diagnostic: &'static str,
    pub command: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TutorialStep {
    pub id: String,
    pub title: String,
    pub goal: String,
    pub command: String,
    pub expected_signal: String,
}

impl TutorialStep {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.id.trim().is_empty()
            || self.title.trim().is_empty()
            || self.goal.trim().is_empty()
            || self.command.trim().is_empty()
            || self.expected_signal.trim().is_empty()
            || self.command.contains("bash scripts/check.sh")
            || self.command.contains("gpu-report")
            || self.command.contains("ALIFE_GPU_BACKEND")
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct TutorialScript {
    pub schema: String,
    pub schema_version: u16,
    pub tutorial_id: String,
    pub title: String,
    pub steps: Vec<TutorialStep>,
    pub manual_graphics_note: String,
}

impl TutorialScript {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G20_TUTORIAL_SCRIPT_SCHEMA
            || self.schema_version != G20_TUTORIAL_SCRIPT_SCHEMA_VERSION
            || self.tutorial_id.trim().is_empty()
            || self.title.trim().is_empty()
            || self.steps.is_empty()
            || self.steps.len() > G20_MAX_TUTORIAL_STEPS
            || self.manual_graphics_note.trim().is_empty()
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        for step in &self.steps {
            step.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingHelpSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub first_run_command: &'static str,
    pub controls: Vec<HelpControlReference>,
    pub troubleshooting: Vec<TroubleshootingReference>,
    pub tutorial_script_path: PathBuf,
    pub tutorial_step_count: usize,
    pub docs_path: PathBuf,
    pub content_authoring_docs_path: PathBuf,
    pub optional_systems_remain_optional: bool,
    pub windows_wrappers_documented: bool,
}

impl OnboardingHelpSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G20_ONBOARDING_HELP_SCHEMA
            || self.schema_version != G20_ONBOARDING_HELP_SCHEMA_VERSION
            || self.first_run_command.trim().is_empty()
            || self.controls.is_empty()
            || self.troubleshooting.is_empty()
            || self.tutorial_step_count == 0
            || !self.tutorial_script_path.is_file()
            || !self.docs_path.is_file()
            || !self.content_authoring_docs_path.is_file()
            || !self.optional_systems_remain_optional
            || !self.windows_wrappers_documented
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if self
            .troubleshooting
            .iter()
            .any(|entry| entry.command.contains("bash scripts/check.sh"))
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}",
            self.schema_version,
            self.controls.len(),
            self.troubleshooting.len(),
            self.tutorial_step_count,
            self.optional_systems_remain_optional
        )
    }
}

pub fn g20_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_game_app should live under crates/")
        .to_path_buf()
}

pub fn g20_tutorial_script_path() -> PathBuf {
    g20_workspace_root().join("examples/g20/tutorial_food_hazard_sleep_inspection.json")
}

pub fn load_g20_tutorial_script() -> Result<TutorialScript, GameAppShellError> {
    let text = std::fs::read_to_string(g20_tutorial_script_path())?;
    let script: TutorialScript = serde_json::from_str(&text)?;
    script.validate()?;
    Ok(script)
}

pub fn run_onboarding_help_smoke() -> Result<OnboardingHelpSummary, GameAppShellError> {
    let root = g20_workspace_root();
    let script = load_g20_tutorial_script()?;
    let docs_path = root.join("docs/playable_sim_spec/onboarding_help.md");
    let docs = std::fs::read_to_string(&docs_path)?;
    let summary = OnboardingHelpSummary {
        schema: G20_ONBOARDING_HELP_SCHEMA,
        schema_version: G20_ONBOARDING_HELP_SCHEMA_VERSION,
        first_run_command:
            "cargo run -p alife_tools --bin p35_playground -- run-headless crates/alife_world/tests/fixtures/p34",
        controls: controls_reference(),
        troubleshooting: troubleshooting_reference(),
        tutorial_script_path: g20_tutorial_script_path(),
        tutorial_step_count: script.steps.len(),
        docs_path,
        content_authoring_docs_path: root.join("docs/playable_sim_spec/content_authoring.md"),
        optional_systems_remain_optional: docs.contains("optional")
            && docs.contains("typed GPU unavailability")
            && docs.contains("manual"),
        windows_wrappers_documented: docs.contains("scripts/check.ps1")
            && docs.contains("scripts/check_core_boundaries.ps1")
            && docs.contains("scripts/docs_check.ps1")
            && !docs.contains("bash scripts/check.sh"),
    };
    summary.validate()?;
    Ok(summary)
}

pub fn controls_reference() -> Vec<HelpControlReference> {
    vec![
        HelpControlReference {
            label: "Pause",
            action: "Stop automatic tick progression before inspecting a creature",
            source_plan: "G03/G05",
        },
        HelpControlReference {
            label: "Step",
            action: "Advance one deterministic headless brain/world tick",
            source_plan: "G03/G05",
        },
        HelpControlReference {
            label: "Run",
            action: "Resume deterministic ticking after pause",
            source_plan: "G05",
        },
        HelpControlReference {
            label: "Select",
            action: "Inspect a stable world entity ID instead of an engine-local entity",
            source_plan: "G02/G05",
        },
        HelpControlReference {
            label: "Inspect",
            action: "Read drives, hormones, current action, sleep state, and last sealed patch",
            source_plan: "G05/G14",
        },
        HelpControlReference {
            label: "Save/Load",
            action: "Use P34 stable IDs, schema validation, and asset manifest diagnostics",
            source_plan: "G15",
        },
    ]
}

pub fn troubleshooting_reference() -> Vec<TroubleshootingReference> {
    vec![
        TroubleshootingReference {
            symptom: "GPU unavailable or unvalidated",
            diagnostic: "The playable sim should stop learned actions on typed GPU unavailability and avoid GPU performance claims",
            command: "cargo run -p alife_tools --bin p35_playground -- gpu-fallback",
        },
        TroubleshootingReference {
            symptom: "Graphics or Bevy feature unavailable",
            diagnostic: "Use the default headless path; graphics demos are optional/manual",
            command:
                "cargo run -p alife_tools --bin p35_playground -- run-headless crates/alife_world/tests/fixtures/p34",
        },
        TroubleshootingReference {
            symptom: "Schema mismatch or missing asset",
            diagnostic: "Validate P34 fixtures and the P35 manifest before running demos",
            command:
                "cargo run -p alife_tools --bin p35_playground -- validate-manifest examples/p35/playground_manifest.json",
        },
        TroubleshootingReference {
            symptom: "Windows validation tries WSL",
            diagnostic: "Use the PowerShell Git Bash wrappers instead of plain bash",
            command: "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1",
        },
        TroubleshootingReference {
            symptom: "Balance smoke looks scripted",
            diagnostic: "G19 exposes degenerate behavior notes rather than hiding metrics",
            command: "cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke",
        },
    ]
}
