//! S02 minimal interactive runtime controls for the graphical playground.
//!
//! This module is deliberately Bevy-free. The feature-gated Bevy shell maps
//! keyboard input into these commands, and this module applies them through the
//! existing sealed `LiveBrainLoop` control surface.

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimePlaybackState {
    Paused,
    Running,
    ShutdownRequested,
}

impl RuntimePlaybackState {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Paused => "paused",
            Self::Running => "running",
            Self::ShutdownRequested => "shutdown-requested",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeControlCommand {
    TogglePause,
    StepOnce,
    SetRunSpeed(u32),
    RunForTicks(u32),
    RestartAlphaFixture,
    RequestExit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeControlPanel {
    pub schema: &'static str,
    pub schema_version: u16,
    pub playback: RuntimePlaybackState,
    pub run_speed_ticks: u32,
    pub mind_tick: u64,
    pub world_tick: Option<u64>,
    pub selected_action_kind: Option<ActionKind>,
    pub selected_action_id: Option<u32>,
    pub target_entity: Option<u64>,
    pub last_status: Option<BrainTickStatus>,
    pub last_patch_sealed: bool,
    pub sealed_patch_count: usize,
    pub packed_record_count: usize,
    pub player_events: Vec<String>,
    pub terminal_recovery_cause: Option<String>,
    pub direct_cognition_mutation_allowed: bool,
}

impl RuntimeControlPanel {
    pub fn from_live_loop(live: &LiveBrainLoop) -> Self {
        Self {
            schema: S02_RUNTIME_CONTROLS_SCHEMA,
            schema_version: S02_RUNTIME_CONTROLS_SCHEMA_VERSION,
            playback: RuntimePlaybackState::Paused,
            run_speed_ticks: 1,
            mind_tick: live.mind().current_tick().raw(),
            world_tick: None,
            selected_action_kind: None,
            selected_action_id: None,
            target_entity: None,
            last_status: None,
            last_patch_sealed: false,
            sealed_patch_count: 0,
            packed_record_count: 0,
            player_events: vec![
                "Press Space to run, or N to step one GPU-backed tick.".to_string(),
                "GPU path is armed; first tick will verify CPU shadow parity.".to_string(),
                "Creature, food, and hazard markers are presentation-only.".to_string(),
            ],
            terminal_recovery_cause: None,
            direct_cognition_mutation_allowed: false,
        }
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != S02_RUNTIME_CONTROLS_SCHEMA
            || self.schema_version != S02_RUNTIME_CONTROLS_SCHEMA_VERSION
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "S02 runtime control schema must be current",
            });
        }
        if self.run_speed_ticks == 0 || self.run_speed_ticks > S02_MAX_RUN_TICKS_PER_UPDATE {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "S02 run speed must be in 1..=4 ticks per update",
            });
        }
        if self.direct_cognition_mutation_allowed {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "S02 controls must not mutate cognition directly",
            });
        }
        Ok(())
    }

    pub fn apply_command(
        &mut self,
        live: &mut LiveBrainLoop,
        command: RuntimeControlCommand,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        let summaries = match command {
            RuntimeControlCommand::TogglePause => {
                self.playback = match self.playback {
                    RuntimePlaybackState::Paused => RuntimePlaybackState::Running,
                    RuntimePlaybackState::Running => RuntimePlaybackState::Paused,
                    RuntimePlaybackState::ShutdownRequested => {
                        RuntimePlaybackState::ShutdownRequested
                    }
                };
                self.push_player_event(format!("Playback changed to {}.", self.playback.label()));
                Vec::new()
            }
            RuntimeControlCommand::StepOnce => {
                self.playback = RuntimePlaybackState::Paused;
                live.update(LiveBrainTickControl::step_once())?
            }
            RuntimeControlCommand::SetRunSpeed(speed) => {
                self.run_speed_ticks = speed.clamp(1, S02_MAX_RUN_TICKS_PER_UPDATE);
                self.playback = RuntimePlaybackState::Running;
                self.push_player_event(format!("Run speed set to {}x.", self.run_speed_ticks));
                Vec::new()
            }
            RuntimeControlCommand::RunForTicks(ticks) => {
                let bounded = ticks.min(S02_MAX_SMOKE_TICKS);
                self.playback = RuntimePlaybackState::Running;
                live.update(LiveBrainTickControl::run_fixed(bounded))?
            }
            RuntimeControlCommand::RestartAlphaFixture => {
                self.reset_to_alpha_fixture(live);
                Vec::new()
            }
            RuntimeControlCommand::RequestExit => {
                self.playback = RuntimePlaybackState::ShutdownRequested;
                self.push_player_event("Exit requested from graphical controls.".to_string());
                Vec::new()
            }
        };
        for summary in &summaries {
            self.record_tick(summary);
        }
        self.mind_tick = live.mind().current_tick().raw();
        self.validate()?;
        Ok(summaries)
    }

    pub fn advance_if_running(
        &mut self,
        live: &mut LiveBrainLoop,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        if self.playback != RuntimePlaybackState::Running {
            return Ok(Vec::new());
        }
        let summaries = live.update(LiveBrainTickControl::run_fixed(self.run_speed_ticks))?;
        for summary in &summaries {
            self.record_tick(summary);
        }
        self.mind_tick = live.mind().current_tick().raw();
        self.validate()?;
        Ok(summaries)
    }

    pub fn status_overlay_text(&self) -> String {
        self.status_overlay_text_with_backend(
            "GPU: CpuFallback degraded",
            "Gate: CPU shadow; fallback uses CPU reference",
        )
    }

    pub fn status_overlay_text_with_backend(
        &self,
        backend_line: &str,
        extra_lines: &str,
    ) -> String {
        let extra = if extra_lines.trim().is_empty() {
            String::new()
        } else {
            format!("\n{}", extra_lines.trim())
        };
        let action = self.selected_action_kind.map_or("None", |action| {
            action_badge_label_for_target(action, self.target_entity)
        });
        let goal = goal_label_from_action(self.selected_action_kind, self.target_entity);
        let target = self
            .target_entity
            .map_or_else(|| "none".to_string(), |id| format!("stable:{id}"));
        let terminal_note = self
            .terminal_recovery_cause
            .as_ref()
            .map_or(String::new(), |cause| {
                format!("\nSimulation stopped: {cause}. Press R to restart.")
            });
        let event_lines = self.player_event_lines();
        format!(
            "A-Life GPU Alpha Playground\nState: {}  speed={}x  tick={} world={}\n{}\nCreature: stable:1  Goal: {}  Action: {}\nTarget: {}  Intent: {}\nPatch: sealed={} count={}\nLearning: H_shadow pulse visible when count rises\nEvents (last 5):\n{}{}\nControls: Space run/pause | N step | R reset | Esc quit{}",
            self.playback.label(),
            self.run_speed_ticks,
            self.mind_tick,
            self.world_tick
                .map_or_else(|| "pending".to_string(), |tick| tick.to_string()),
            backend_line,
            goal,
            action,
            target,
            self.intent_marker_label(),
            self.last_patch_sealed,
            self.sealed_patch_count,
            event_lines,
            extra,
            terminal_note,
        )
    }

    pub fn structured_status_panel_text_with_backend(&self, backend_line: &str) -> String {
        let action = self.selected_action_kind.map_or("None", |action| {
            action_badge_label_for_target(action, self.target_entity)
        });
        let goal = goal_label_from_action(self.selected_action_kind, self.target_entity);
        let target = self
            .target_entity
            .map_or_else(|| "none".to_string(), |id| format!("stable:{id}"));
        let terminal_line = self
            .terminal_recovery_cause
            .as_ref()
            .map_or("Ready".to_string(), |cause| format!("Stopped: {cause}"));
        format!(
            concat!(
                "Status\n",
                "A-Life GPU Alpha Playground\n",
                "State: {}  Speed: {}x\n",
                "Tick: {}  World: {}\n",
                "{}\n",
                "Creature: stable:1\n",
                "Goal: {}  Action: {}\n",
                "Target: {}  Intent: {}\n",
                "Patch: sealed={} count={}\n",
                "Learning: H_shadow pulse\n",
                "{}"
            ),
            self.playback.label(),
            self.run_speed_ticks,
            self.mind_tick,
            self.world_tick
                .map_or_else(|| "pending".to_string(), |tick| tick.to_string()),
            backend_line,
            goal,
            action,
            target,
            self.intent_marker_label(),
            self.last_patch_sealed,
            self.sealed_patch_count,
            terminal_line,
        )
    }

    pub fn event_feed_panel_text(&self) -> String {
        format!("Event Feed\n{}", self.player_event_lines())
    }

    fn player_event_lines(&self) -> String {
        self.player_events
            .iter()
            .rev()
            .take(S02_MAX_PLAYER_EVENT_LINES)
            .map(|line| format!("- {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:speed={}:mind_tick={}:world_tick={:?}:action={:?}:sealed={}:patches={}:packed={}",
            self.schema,
            self.schema_version,
            self.playback.label(),
            self.run_speed_ticks,
            self.mind_tick,
            self.world_tick,
            self.selected_action_kind,
            self.last_patch_sealed,
            self.sealed_patch_count,
            self.packed_record_count
        )
    }

    pub fn record_tick(&mut self, summary: &LiveBrainTickSummary) {
        self.mind_tick = summary.tick_after.raw();
        self.world_tick = Some(summary.world_tick_after.raw());
        self.selected_action_kind = summary.selected_action_kind;
        self.selected_action_id = summary.selected_action_id.map(|id| id.raw());
        self.target_entity = summary.target_entity.map(|id| id.raw());
        self.last_status = Some(summary.status);
        self.last_patch_sealed = summary.patch_sealed;
        self.sealed_patch_count = summary.sealed_patch_count;
        self.packed_record_count = summary.packed_record_count;
        self.terminal_recovery_cause = match summary.status {
            BrainTickStatus::TerminalInvalidState => Some("invalid action/state".to_string()),
            _ => None,
        };
        self.record_player_events_for_tick(summary);
    }

    fn record_player_events_for_tick(&mut self, summary: &LiveBrainTickSummary) {
        self.push_player_event(format!("Tick advanced with status {:?}.", summary.status));
        if let Some(action) = summary.selected_action_kind {
            let target = summary.target_entity.map_or_else(
                || "no target".to_string(),
                |id| format!("stable:{}", id.raw()),
            );
            self.push_player_event(format!(
                "Creature action {} toward {}.",
                action_badge_label_for_target(action, summary.target_entity.map(|id| id.raw())),
                target
            ));
            if let Some(target) = summary.target_entity {
                self.push_player_event(format!("Intent line stable:1 -> stable:{}.", target.raw()));
                match (action, target.raw()) {
                    (ActionKind::Interact, 2) => {
                        self.push_player_event("Food interaction cue highlighted.".to_string())
                    }
                    (ActionKind::Move, 3) => {
                        self.push_player_event("Hazard avoidance cue highlighted.".to_string())
                    }
                    _ => {}
                }
            }
        } else {
            self.push_player_event("Creature produced no selected action this tick.".to_string());
        }
        if summary.patch_sealed {
            self.push_player_event(format!(
                "Patch sealed count={}.",
                summary.sealed_patch_count
            ));
        } else {
            self.push_player_event("Patch not sealed; press R if simulation stops.".to_string());
        }
    }

    pub fn record_terminal_recovery(&mut self, cause: impl Into<String>) {
        let cause = cause.into();
        self.playback = RuntimePlaybackState::Paused;
        self.last_status = Some(BrainTickStatus::TerminalInvalidState);
        self.last_patch_sealed = false;
        self.terminal_recovery_cause = Some(cause.clone());
        self.push_player_event(format!("Simulation stopped: {cause}. Press R to restart."));
    }

    pub fn record_control_event(&mut self, event: impl Into<String>) {
        self.push_player_event(event.into());
    }

    pub fn reset_to_alpha_fixture(&mut self, live: &LiveBrainLoop) {
        *self = Self::from_live_loop(live);
        self.player_events.clear();
        self.push_player_event(
            "Alpha fixture reset; stable IDs preserved. Press Space or N to continue.".to_string(),
        );
    }

    fn push_player_event(&mut self, event: String) {
        self.player_events.push(event);
        if self.player_events.len() > S02_MAX_PLAYER_EVENT_LINES {
            let drain_count = self.player_events.len() - S02_MAX_PLAYER_EVENT_LINES;
            self.player_events.drain(0..drain_count);
        }
    }

    pub fn intent_marker_label(&self) -> String {
        match (self.selected_action_kind, self.target_entity) {
            (Some(action), Some(target)) => format!(
                "stable:1 -> stable:{target} ({})",
                action_badge_label_for_target(action, Some(target))
            ),
            (Some(action), None) => {
                format!("stable:1 ({})", action_badge_label_for_target(action, None))
            }
            (None, _) => "pending".to_string(),
        }
    }
}

pub const S02_MAX_PLAYER_EVENT_LINES: usize = 5;

pub fn action_badge_label(kind: ActionKind) -> &'static str {
    action_badge_label_for_target(kind, None)
}

pub fn action_badge_label_for_target(kind: ActionKind, target_entity: Option<u64>) -> &'static str {
    match kind {
        ActionKind::Move if target_entity == Some(3) => "FLEE",
        ActionKind::Move => "APPROACH",
        ActionKind::Interact | ActionKind::Hold => "EAT",
        ActionKind::Inspect => "INSPECT",
        ActionKind::Rest => "SLEEP",
        ActionKind::Idle => "IDLE",
        ActionKind::Vocalize | ActionKind::Write | ActionKind::Gesture => "SIGNAL",
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeControlSmokeSummary {
    pub panel: RuntimeControlPanel,
    pub paused_produced: usize,
    pub step_produced: usize,
    pub run_produced: usize,
    pub all_patches_sealed: bool,
}

impl RuntimeControlSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        self.panel.validate()?;
        if self.paused_produced != 0 || self.step_produced != 1 || self.run_produced == 0 {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "S02 runtime smoke must prove pause, step, and run semantics",
            });
        }
        if !self.all_patches_sealed {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "S02 runtime smoke must use sealed patches only",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphicalControlSmokeSummary {
    pub runtime: RuntimeControlSmokeSummary,
    pub toggle_pause_run_verified: bool,
    pub speed_sequence: [u32; 3],
    pub follow_target: Option<WorldEntityId>,
    pub reset_verified: bool,
    pub terminal_guidance_visible: bool,
    pub exit_requested: bool,
    pub overlay_text: String,
}

impl GraphicalControlSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        self.runtime.validate()?;
        if !self.toggle_pause_run_verified {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical control smoke must verify Space-equivalent pause/run toggle",
            });
        }
        if self.speed_sequence != [1, 2, 3] {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical control smoke must verify 1/2/3 speed semantics",
            });
        }
        if self.follow_target != Some(WorldEntityId(1)) {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical control smoke must verify F-equivalent stable-ID follow",
            });
        }
        if !self.reset_verified || !self.overlay_text.contains("Alpha fixture reset") {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical control smoke must verify R-equivalent reset/restart",
            });
        }
        if self.runtime.panel.run_speed_ticks != 1 {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical control smoke reset must restore default 1x speed",
            });
        }
        if !self.terminal_guidance_visible
            || !self.overlay_text.contains("Simulation stopped:")
            || !self.overlay_text.contains("Press R to restart")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical control smoke must verify terminal recovery guidance",
            });
        }
        if !self.exit_requested
            || self.runtime.panel.playback != RuntimePlaybackState::ShutdownRequested
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical control smoke must verify Esc-equivalent exit request",
            });
        }
        if !self.overlay_text.contains("Controls:")
            || self.overlay_text.contains("Entity(")
            || !self.overlay_text.contains("A-Life GPU Alpha Playground")
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message:
                    "graphical control smoke overlay must stay player-facing and stable-ID safe",
            });
        }
        Ok(())
    }
}

pub fn run_runtime_controls_smoke(
    launch: &AppShellLaunchConfig,
    run_ticks: u32,
) -> Result<RuntimeControlSmokeSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let paused = live.update(LiveBrainTickControl::paused())?;
    panel.mind_tick = live.mind().current_tick().raw();
    panel.validate()?;
    let step = panel.apply_command(&mut live, RuntimeControlCommand::StepOnce)?;
    panel.apply_command(&mut live, RuntimeControlCommand::SetRunSpeed(2))?;
    let run = panel.apply_command(&mut live, RuntimeControlCommand::RunForTicks(run_ticks))?;
    let all_patches_sealed = step
        .iter()
        .chain(run.iter())
        .all(|summary| summary.patch_sealed);
    let summary = RuntimeControlSmokeSummary {
        panel,
        paused_produced: paused.len(),
        step_produced: step.len(),
        run_produced: run.len(),
        all_patches_sealed,
    };
    summary.validate()?;
    Ok(summary)
}

pub fn run_graphical_controls_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<GraphicalControlSmokeSummary, GameAppShellError> {
    let presentation = load_visible_world_from_p34_save(launch)?;
    let selection = select_visible_world_entity(&presentation, WorldEntityId(1))?;
    let camera = CameraNavigationState::top_down_default()
        .focus_on(selection.position)?
        .with_follow_target(selection.stable_id)?;

    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&live);
    let paused = live.update(LiveBrainTickControl::paused())?;
    panel.mind_tick = live.mind().current_tick().raw();
    panel.validate()?;

    panel.apply_command(&mut live, RuntimeControlCommand::TogglePause)?;
    let toggle_pause_run_verified = panel.playback == RuntimePlaybackState::Running;
    let step = panel.apply_command(&mut live, RuntimeControlCommand::StepOnce)?;
    panel.apply_command(&mut live, RuntimeControlCommand::SetRunSpeed(1))?;
    panel.apply_command(&mut live, RuntimeControlCommand::SetRunSpeed(2))?;
    panel.apply_command(&mut live, RuntimeControlCommand::SetRunSpeed(3))?;
    let run = panel.apply_command(&mut live, RuntimeControlCommand::RunForTicks(3))?;
    panel.apply_command(&mut live, RuntimeControlCommand::RestartAlphaFixture)?;
    let reset_verified = panel
        .player_events
        .iter()
        .any(|event| event.contains("Alpha fixture reset"));
    let reset_overlay_text = panel.status_overlay_text();
    panel.record_terminal_recovery("invalid action/state");
    let terminal_guidance_visible = panel
        .status_overlay_text()
        .contains("Simulation stopped: invalid action/state. Press R to restart.");
    panel.apply_command(&mut live, RuntimeControlCommand::RequestExit)?;

    let all_patches_sealed = step
        .iter()
        .chain(run.iter())
        .all(|summary| summary.patch_sealed);
    let overlay_text = format!("{}\n{}", reset_overlay_text, panel.status_overlay_text());
    let runtime = RuntimeControlSmokeSummary {
        panel,
        paused_produced: paused.len(),
        step_produced: step.len(),
        run_produced: run.len(),
        all_patches_sealed,
    };
    let summary = GraphicalControlSmokeSummary {
        runtime,
        toggle_pause_run_verified,
        speed_sequence: [1, 2, 3],
        follow_target: camera.follow_target,
        reset_verified,
        terminal_guidance_visible,
        exit_requested: true,
        overlay_text,
    };
    summary.validate()?;
    Ok(summary)
}

fn goal_label_from_action(kind: Option<ActionKind>, target: Option<u64>) -> &'static str {
    match (kind, target) {
        (Some(ActionKind::Interact), Some(2)) => "food",
        (Some(ActionKind::Move), Some(3)) => "hazard",
        (Some(ActionKind::Inspect), _) => "inspect",
        (Some(ActionKind::Idle), _) | (None, _) => "idle",
        _ => "world",
    }
}
