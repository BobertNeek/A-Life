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
                Vec::new()
            }
            RuntimeControlCommand::StepOnce => {
                self.playback = RuntimePlaybackState::Paused;
                live.update(LiveBrainTickControl::step_once())?
            }
            RuntimeControlCommand::SetRunSpeed(speed) => {
                self.run_speed_ticks = speed.clamp(1, S02_MAX_RUN_TICKS_PER_UPDATE);
                self.playback = RuntimePlaybackState::Running;
                Vec::new()
            }
            RuntimeControlCommand::RunForTicks(ticks) => {
                let bounded = ticks.min(S02_MAX_SMOKE_TICKS);
                self.playback = RuntimePlaybackState::Running;
                live.update(LiveBrainTickControl::run_fixed(bounded))?
            }
            RuntimeControlCommand::RestartAlphaFixture => {
                self.playback = RuntimePlaybackState::Paused;
                self.world_tick = None;
                self.selected_action_kind = None;
                self.selected_action_id = None;
                self.target_entity = None;
                self.last_status = None;
                self.last_patch_sealed = false;
                self.sealed_patch_count = 0;
                self.packed_record_count = 0;
                Vec::new()
            }
            RuntimeControlCommand::RequestExit => {
                self.playback = RuntimePlaybackState::ShutdownRequested;
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
        let action = self
            .selected_action_kind
            .map_or_else(|| "None".to_string(), |kind| format!("{kind:?}"));
        let goal = goal_label_from_action(self.selected_action_kind, self.target_entity);
        let target = self
            .target_entity
            .map_or_else(|| "none".to_string(), |id| format!("stable:{id}"));
        let terminal_note = match self.last_status {
            Some(BrainTickStatus::TerminalInvalidState) => {
                "\nSimulation stopped: invalid action/state. Press R to restart."
            }
            _ => "",
        };
        let event_lines = self.player_event_lines();
        format!(
            "A-Life GPU Alpha Playground\nState: {}  speed={}x  tick={} world={}\n{}\nCreature: stable:1  Goal: {}  Action: {}\nTarget: {}  Patch: sealed={} count={}\nLearning: H_shadow pulse visible when count rises\nEvents:\n{}{}\nControls: Space run/pause | N step | R reset | Esc quit{}",
            self.playback.label(),
            self.run_speed_ticks,
            self.mind_tick,
            self.world_tick
                .map_or_else(|| "pending".to_string(), |tick| tick.to_string()),
            backend_line,
            goal,
            action,
            target,
            self.last_patch_sealed,
            self.sealed_patch_count,
            event_lines,
            extra,
            terminal_note,
        )
    }

    fn player_event_lines(&self) -> String {
        if self.last_status.is_none() {
            return [
                "- Press Space to run, or N to step one GPU-backed tick.",
                "- GPU path is armed; first tick will verify CPU shadow parity.",
                "- Creature, food, and hazard markers are presentation-only.",
            ]
            .join("\n");
        }

        let mut lines = Vec::new();
        if let Some(status) = self.last_status {
            lines.push(format!("- Tick advanced with status {status:?}."));
        }
        if let Some(action) = self.selected_action_kind {
            let target = self
                .target_entity
                .map_or_else(|| "no target".to_string(), |id| format!("stable:{id}"));
            lines.push(format!("- Creature chose {action:?} toward {target}."));
        } else {
            lines.push("- Creature produced no selected action this tick.".to_string());
        }
        if self.last_patch_sealed {
            lines.push(format!("- Patch sealed count={}.", self.sealed_patch_count));
        } else {
            lines.push("- Patch not sealed yet; press R if simulation stops.".to_string());
        }
        lines.push("- H_shadow learning pulse appears when count rises.".to_string());
        lines.join("\n")
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
        if self.speed_sequence != [1, 2, 3] || self.runtime.panel.run_speed_ticks != 3 {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical control smoke must verify 1/2/3 speed semantics",
            });
        }
        if self.follow_target != Some(WorldEntityId(1)) {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "graphical control smoke must verify F-equivalent stable-ID follow",
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
    panel.apply_command(&mut live, RuntimeControlCommand::RequestExit)?;

    let all_patches_sealed = step
        .iter()
        .chain(run.iter())
        .all(|summary| summary.patch_sealed);
    let overlay_text = panel.status_overlay_text();
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
