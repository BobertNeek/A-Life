//! CA13 double-buffered graphical/game tick scheduler.
//!
//! This module is deliberately Bevy-free. It aligns render frames to a fixed
//! simulation cadence and exposes compact scheduler state for the graphical
//! overlay without making rendering authoritative over cognition.

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ca13TickBuffer {
    A,
    B,
}

impl Ca13TickBuffer {
    pub const fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }

    pub const fn swapped(self) -> Self {
        match self {
            Self::A => Self::B,
            Self::B => Self::A,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoubleBufferedSchedulerConfig {
    pub fixed_tick_hz: u32,
    pub target_render_hz: u32,
    pub max_catch_up_ticks_per_frame: u32,
    pub max_accumulator_micros: u64,
}

impl Default for DoubleBufferedSchedulerConfig {
    fn default() -> Self {
        Self {
            fixed_tick_hz: CA13_FIXED_SIM_TICK_HZ,
            target_render_hz: CA13_TARGET_RENDER_FRAME_HZ,
            max_catch_up_ticks_per_frame: CA13_MAX_CATCH_UP_TICKS_PER_FRAME,
            max_accumulator_micros: CA13_MAX_ACCUMULATOR_MICROS,
        }
    }
}

impl DoubleBufferedSchedulerConfig {
    pub fn validate(self) -> Result<(), GameAppShellError> {
        if self.fixed_tick_hz == 0
            || self.target_render_hz == 0
            || self.max_catch_up_ticks_per_frame == 0
            || self.max_accumulator_micros == 0
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA13 scheduler config values must be nonzero",
            });
        }
        Ok(())
    }

    pub const fn fixed_tick_micros(self) -> u64 {
        1_000_000 / self.fixed_tick_hz as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoubleBufferedFramePlan {
    pub frame_index: u64,
    pub ticks_to_run: u32,
    pub render_alpha_milli: u16,
    pub front_buffer: Ca13TickBuffer,
    pub back_buffer: Ca13TickBuffer,
    pub catch_up_capped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleBufferedGraphicalScheduler {
    pub schema: &'static str,
    pub schema_version: u16,
    pub config: DoubleBufferedSchedulerConfig,
    pub fixed_tick_index: u64,
    pub render_frame_index: u64,
    pub accumulator_micros: u64,
    pub render_alpha_milli: u16,
    pub front_buffer: Ca13TickBuffer,
    pub back_buffer: Ca13TickBuffer,
    pub catch_up_ticks_dropped: u64,
    pub frames_observed: u64,
    pub ticks_executed: u64,
    pub paused_frames: u64,
}

impl Default for DoubleBufferedGraphicalScheduler {
    fn default() -> Self {
        Self::new(DoubleBufferedSchedulerConfig::default())
    }
}

impl DoubleBufferedGraphicalScheduler {
    pub fn new(config: DoubleBufferedSchedulerConfig) -> Self {
        Self {
            schema: CA13_DOUBLE_BUFFERED_SCHEDULER_SCHEMA,
            schema_version: CA13_DOUBLE_BUFFERED_SCHEDULER_SCHEMA_VERSION,
            config,
            fixed_tick_index: 0,
            render_frame_index: 0,
            accumulator_micros: 0,
            render_alpha_milli: 0,
            front_buffer: Ca13TickBuffer::A,
            back_buffer: Ca13TickBuffer::B,
            catch_up_ticks_dropped: 0,
            frames_observed: 0,
            ticks_executed: 0,
            paused_frames: 0,
        }
    }

    pub fn validate(&self) -> Result<(), GameAppShellError> {
        self.config.validate()?;
        if self.schema != CA13_DOUBLE_BUFFERED_SCHEDULER_SCHEMA
            || self.schema_version != CA13_DOUBLE_BUFFERED_SCHEDULER_SCHEMA_VERSION
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA13 scheduler schema must be current",
            });
        }
        if self.front_buffer == self.back_buffer {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA13 scheduler front/back buffers must be distinct",
            });
        }
        if self.accumulator_micros > self.config.max_accumulator_micros {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA13 scheduler accumulator must stay bounded",
            });
        }
        Ok(())
    }

    pub fn observe_render_frame(
        &mut self,
        delta_seconds: f32,
        playback: RuntimePlaybackState,
        speed_multiplier: u32,
    ) -> Result<DoubleBufferedFramePlan, GameAppShellError> {
        self.validate()?;
        if !delta_seconds.is_finite() || delta_seconds < 0.0 {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA13 scheduler delta must be finite and nonnegative",
            });
        }

        self.render_frame_index = self.render_frame_index.saturating_add(1);
        self.frames_observed = self.frames_observed.saturating_add(1);

        if playback != RuntimePlaybackState::Running {
            self.paused_frames = self.paused_frames.saturating_add(1);
            self.render_alpha_milli = 0;
            return Ok(self.frame_plan(0, false));
        }

        let speed = speed_multiplier.clamp(1, S02_MAX_RUN_TICKS_PER_UPDATE);
        let frame_micros = (delta_seconds * 1_000_000.0).round() as u64;
        let added = frame_micros.saturating_mul(speed as u64);
        self.accumulator_micros = self
            .accumulator_micros
            .saturating_add(added)
            .min(self.config.max_accumulator_micros);

        let tick_micros = self.config.fixed_tick_micros().max(1);
        let due = (self.accumulator_micros / tick_micros) as u32;
        let ticks_to_run = due.min(self.config.max_catch_up_ticks_per_frame);
        let catch_up_capped = due > ticks_to_run;
        self.accumulator_micros = self
            .accumulator_micros
            .saturating_sub(tick_micros.saturating_mul(ticks_to_run as u64));
        if catch_up_capped {
            let remaining_due = self.accumulator_micros / tick_micros;
            self.catch_up_ticks_dropped = self.catch_up_ticks_dropped.saturating_add(remaining_due);
            self.accumulator_micros = tick_micros.saturating_sub(1);
        }
        self.render_alpha_milli =
            ((self.accumulator_micros.saturating_mul(1000)) / tick_micros).min(999) as u16;
        Ok(self.frame_plan(ticks_to_run, catch_up_capped))
    }

    pub fn record_executed_ticks(&mut self, count: u32) -> Result<(), GameAppShellError> {
        if count == 0 {
            self.validate()?;
            return Ok(());
        }
        for _ in 0..count {
            self.fixed_tick_index = self.fixed_tick_index.saturating_add(1);
            self.ticks_executed = self.ticks_executed.saturating_add(1);
            self.front_buffer = self.front_buffer.swapped();
            self.back_buffer = self.back_buffer.swapped();
        }
        self.validate()
    }

    pub fn render_alpha(&self) -> f32 {
        self.render_alpha_milli as f32 / 1000.0
    }

    pub fn overlay_line(&self) -> String {
        format!(
            "Scheduler: fixed={}Hz render={}Hz alpha={:.3} buffer={}/{} drift={}us",
            self.config.fixed_tick_hz,
            self.config.target_render_hz,
            self.render_alpha(),
            self.front_buffer.label(),
            self.back_buffer.label(),
            self.accumulator_micros
        )
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:fixed={}Hz:render={}Hz:frame={}:tick={}:alpha={}:front={}:back={}:dropped={}",
            self.schema,
            self.schema_version,
            self.config.fixed_tick_hz,
            self.config.target_render_hz,
            self.render_frame_index,
            self.fixed_tick_index,
            self.render_alpha_milli,
            self.front_buffer.label(),
            self.back_buffer.label(),
            self.catch_up_ticks_dropped
        )
    }

    fn frame_plan(&self, ticks_to_run: u32, catch_up_capped: bool) -> DoubleBufferedFramePlan {
        DoubleBufferedFramePlan {
            frame_index: self.render_frame_index,
            ticks_to_run,
            render_alpha_milli: self.render_alpha_milli,
            front_buffer: self.front_buffer,
            back_buffer: self.back_buffer,
            catch_up_capped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoubleBufferedSchedulerSmokeSummary {
    pub scheduler: DoubleBufferedGraphicalScheduler,
    pub paused_ticks: u32,
    pub sub_tick_due: u32,
    pub fixed_tick_due: u32,
    pub step_ticks: u32,
    pub catch_up_ticks: u32,
    pub frame_driven_drift_prevented: bool,
}

impl DoubleBufferedSchedulerSmokeSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        self.scheduler.validate()?;
        if self.paused_ticks != 0
            || self.sub_tick_due != 0
            || self.fixed_tick_due != 1
            || self.step_ticks != 1
            || self.catch_up_ticks > CA13_MAX_CATCH_UP_TICKS_PER_FRAME
            || !self.frame_driven_drift_prevented
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA13 scheduler smoke must prove fixed cadence, pause, step, and catch-up bounds",
            });
        }
        Ok(())
    }
}

pub fn run_double_buffered_scheduler_smoke(
    launch: &AppShellLaunchConfig,
) -> Result<DoubleBufferedSchedulerSmokeSummary, GameAppShellError> {
    let mut live = LiveBrainLoop::from_p34_launch(launch)?;
    let mut panel = RuntimeControlPanel::from_live_loop(&live);

    let paused = panel.scheduler.observe_render_frame(
        1.0,
        RuntimePlaybackState::Paused,
        panel.run_speed_ticks,
    )?;
    let sub_tick = panel.scheduler.observe_render_frame(
        0.016,
        RuntimePlaybackState::Running,
        panel.run_speed_ticks,
    )?;
    let fixed_tick = panel.scheduler.observe_render_frame(
        0.034,
        RuntimePlaybackState::Running,
        panel.run_speed_ticks,
    )?;
    let tick_summaries = panel.apply_command(
        &mut live,
        RuntimeControlCommand::RunForTicks(fixed_tick.ticks_to_run),
    )?;
    let before_step = panel.mind_tick;
    let step_summaries = panel.apply_command(&mut live, RuntimeControlCommand::StepOnce)?;
    let catch_up = panel.scheduler.observe_render_frame(
        1.0,
        RuntimePlaybackState::Running,
        panel.run_speed_ticks,
    )?;
    let summary = DoubleBufferedSchedulerSmokeSummary {
        scheduler: panel.scheduler.clone(),
        paused_ticks: paused.ticks_to_run,
        sub_tick_due: sub_tick.ticks_to_run,
        fixed_tick_due: fixed_tick.ticks_to_run,
        step_ticks: step_summaries.len() as u32,
        catch_up_ticks: catch_up.ticks_to_run,
        frame_driven_drift_prevented: tick_summaries.len() == 1
            && panel.mind_tick == before_step + 1,
    };
    summary.validate()?;
    Ok(summary)
}
