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
        format!(
            "A-Life Graphical Playground\nStatus: {}  speed={} tick/update\nMind tick: {}  World tick: {}\nLast action: {}  target={}\nLast status: {}  sealed_patch={} sealed_patches={} packed_logs={}\nBackend: CPU Reference fallback\nControls: Space pause/run | N step | 1/2/3 speed | Esc quit",
            self.playback.label(),
            self.run_speed_ticks,
            self.mind_tick,
            self.world_tick
                .map_or_else(|| "pending".to_string(), |tick| tick.to_string()),
            self.selected_action_kind
                .map_or_else(|| "None".to_string(), |kind| format!("{kind:?}")),
            self.target_entity
                .map_or_else(|| "None".to_string(), |id| id.to_string()),
            self.last_status
                .map_or_else(|| "None".to_string(), |status| format!("{status:?}")),
            self.last_patch_sealed,
            self.sealed_patch_count,
            self.packed_record_count,
        )
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

    fn record_tick(&mut self, summary: &LiveBrainTickSummary) {
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
