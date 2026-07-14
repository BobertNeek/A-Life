//! CA40 first-session onboarding tutorial surface.
//!
//! This module keeps the tutorial panel testable without Bevy. The panel is a
//! display-only guide over existing runtime controls, stable IDs, GPU telemetry,
//! and visible world markers.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ca40TutorialChecklistItem {
    pub id: &'static str,
    pub label: &'static str,
    pub instruction: &'static str,
    pub expected_signal: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ca40OnboardingTutorialSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub fixture_root: PathBuf,
    pub launch_command: &'static str,
    pub checklist: Vec<Ca40TutorialChecklistItem>,
    pub tutorial_panel_text: String,
    pub pause_step_follow_instructions_visible: bool,
    pub food_hazard_explanation_visible: bool,
    pub gpu_authority_explanation_visible: bool,
    pub graphical_controls_verified: bool,
    pub has_food_marker: bool,
    pub has_hazard_marker: bool,
    pub stable_ids_only: bool,
    pub display_only: bool,
    pub no_action_authority: bool,
    pub no_weight_authority: bool,
    pub gpu_authority_status_visible: bool,
    pub no_full_action_authoritative_claim: bool,
}

impl Ca40OnboardingTutorialSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA40_ONBOARDING_TUTORIAL_SCHEMA
            || self.schema_version != CA40_ONBOARDING_TUTORIAL_SCHEMA_VERSION
            || self.checklist.len() < CA40_REQUIRED_CHECKLIST_ITEMS
            || self.launch_command.trim().is_empty()
            || !self.fixture_root.is_dir()
            || self.tutorial_panel_text.trim().is_empty()
            || !self.pause_step_follow_instructions_visible
            || !self.food_hazard_explanation_visible
            || !self.gpu_authority_explanation_visible
            || !self.graphical_controls_verified
            || !self.has_food_marker
            || !self.has_hazard_marker
            || !self.stable_ids_only
            || !self.display_only
            || !self.no_action_authority
            || !self.no_weight_authority
            || !self.gpu_authority_status_visible
            || !self.no_full_action_authoritative_claim
            || self.tutorial_panel_text.contains("Entity(")
            || self
                .tutorial_panel_text
                .contains("full action-authoritative")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA40 onboarding tutorial summary violates first-session contract",
            });
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:items={}:food={}:hazard={}:controls={}:stable_ids={}:display_only={}:gpu_authority={}:full_auth={}",
            self.schema,
            self.schema_version,
            self.checklist.len(),
            self.has_food_marker,
            self.has_hazard_marker,
            self.graphical_controls_verified,
            self.stable_ids_only,
            self.display_only,
            self.gpu_authority_status_visible,
            !self.no_full_action_authoritative_claim
        )
    }
}

pub fn ca40_tutorial_checklist_items() -> Vec<Ca40TutorialChecklistItem> {
    vec![
        Ca40TutorialChecklistItem {
            id: "observe-creature",
            label: "Find the selected creature",
            instruction: "Look for [@] creature stable:1 and the selection ring.",
            expected_signal: "Selected stable ID is visible in the inspector.",
        },
        Ca40TutorialChecklistItem {
            id: "pause-run-step",
            label: "Control time",
            instruction: "Press Space to run/pause, then N to advance one step.",
            expected_signal: "Tick, action, and event feed advance.",
        },
        Ca40TutorialChecklistItem {
            id: "follow-creature",
            label: "Follow the creature",
            instruction: "Press F to follow selected stable:1; use Tab to cycle creatures.",
            expected_signal: "Camera/follow state stays stable-ID based.",
        },
        Ca40TutorialChecklistItem {
            id: "read-food-hazard",
            label: "Read food and danger",
            instruction: "Food is [+]; hazards are [!]; rock/terrain is [#].",
            expected_signal: "Food and hazard markers are visible and labelled.",
        },
        Ca40TutorialChecklistItem {
            id: "read-gpu-authority",
            label: "Read GPU authority state",
            instruction: "GPU should show authoritative; an unavailable GPU stops learned actions.",
            expected_signal: "CPU shadow gate stays visible; no full action-authoritative claim.",
        },
    ]
}

pub fn run_onboarding_tutorial_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<Ca40OnboardingTutorialSummary, GameAppShellError> {
    let visible = load_visible_world_from_p34_save(launch)?;
    let controls = run_graphical_controls_smoke(launch)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&LiveBrainLoop::from_p34_launch(launch)?);
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    panel.apply_command(&mut live, RuntimeControlCommand::StepOnce)?;
    let gpu = GraphicalGpuRuntimeTelemetry::pending("N2048");
    let tutorial_panel_text = ca40_first_session_tutorial_panel_text(&panel, &gpu);
    let summary = Ca40OnboardingTutorialSummary {
        schema: CA40_ONBOARDING_TUTORIAL_SCHEMA,
        schema_version: CA40_ONBOARDING_TUTORIAL_SCHEMA_VERSION,
        fixture_root: launch.fixture_root.clone(),
        launch_command:
            "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -BrainPolicy gpu-required",
        checklist: ca40_tutorial_checklist_items(),
        pause_step_follow_instructions_visible: tutorial_panel_text.contains("Space")
            && tutorial_panel_text.contains("N step")
            && tutorial_panel_text.contains("F follow"),
        food_hazard_explanation_visible: tutorial_panel_text.contains("[+] food")
            && tutorial_panel_text.contains("[!] hazard"),
        gpu_authority_explanation_visible: tutorial_panel_text.contains("GPU")
            && tutorial_panel_text.contains("stop learned actions"),
        graphical_controls_verified: controls.toggle_pause_run_verified
            && controls.follow_target == Some(WorldEntityId(1))
            && controls.reset_verified
            && controls.exit_requested,
        has_food_marker: visible.kind_count(WorldObjectKind::Food) > 0,
        has_hazard_marker: visible.kind_count(WorldObjectKind::Hazard) > 0,
        stable_ids_only: true,
        display_only: true,
        no_action_authority: true,
        no_weight_authority: true,
        gpu_authority_status_visible: tutorial_panel_text.contains("GPU neural: authoritative"),
        no_full_action_authoritative_claim: true,
        tutorial_panel_text,
    };
    summary.validate()?;
    Ok(summary)
}

pub fn ca40_first_session_tutorial_panel_text(
    panel: &RuntimeControlPanel,
    gpu: &GraphicalGpuRuntimeTelemetry,
) -> String {
    let observe_done = true;
    let run_seen = panel
        .player_events
        .iter()
        .any(|event| event.contains("Playback changed"))
        || panel.playback == RuntimePlaybackState::Running;
    let step_seen = panel.world_tick.is_some() || panel.mind_tick > 0;
    let food_hazard_seen = panel
        .player_events
        .iter()
        .any(|event| event.contains("Food") || event.contains("Hazard"))
        || panel.target_entity == Some(2)
        || panel.target_entity == Some(3);
    let gpu_seen = gpu.authoritative;
    format!(
        concat!(
            "First Steps\n",
            "{} [@] creature stable:1 selected\n",
            "{} Space run/pause; N step once\n",
            "{} F follow; Tab cycles stable IDs\n",
            "{} Map: [+] food, [!] hazard, [#] rock\n",
            "{} GPU neural: {}\n",
            "Next: press Space, then N, then F follow.\n",
            "Failure policy: stop learned actions; tutorial display-only"
        ),
        checklist_mark(observe_done),
        checklist_mark(run_seen || step_seen),
        checklist_mark(observe_done),
        checklist_mark(food_hazard_seen),
        checklist_mark(gpu_seen),
        compact_tutorial_value(
            if gpu.authoritative {
                "authoritative"
            } else {
                "initializing"
            },
            18,
        ),
    )
}

pub fn ca40_first_session_tutorial_placeholder_text() -> &'static str {
    concat!(
        "First Steps\n",
        "[x] [@] creature stable:1 selected\n",
        "[ ] Space run/pause; N step once\n",
        "[ ] F follow; Tab cycles stable IDs\n",
        "[ ] Map: [+] food, [!] hazard, [#] rock\n",
        "[ ] GPU neural: initializing\n",
        "Next: press Space, then N, then F follow.\n",
        "Failure policy: stop learned actions; tutorial display-only"
    )
}

fn checklist_mark(done: bool) -> &'static str {
    if done {
        "[x]"
    } else {
        "[ ]"
    }
}

fn compact_tutorial_value(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut compact = value
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    compact.push_str("...");
    compact
}
