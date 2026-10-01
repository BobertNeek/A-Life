//! Playback policy only: all modes execute the same authoritative world ticks.

#[cfg(all(feature = "gpu-runtime", feature = "bevy-app"))]
use crate::{
    GameAppShellError, GpuLiveBrainRuntime, GpuLiveNoProgressReason, GpuLiveTickOutcome,
    GpuManualCheckpointStatus, ProductionVoxelLaunchSummary,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProductionRunMode {
    #[default]
    OneX,
    MaxSpeed,
    HeadlessMaxSpeed,
}

impl ProductionRunMode {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "1x" => Ok(Self::OneX),
            "max" => Ok(Self::MaxSpeed),
            "headless-max" => Ok(Self::HeadlessMaxSpeed),
            _ => Err("run mode must be 1x, max, or headless-max".to_string()),
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::OneX => "1x",
            Self::MaxSpeed => "Max speed",
            Self::HeadlessMaxSpeed => "Headless max speed",
        }
    }
}

#[cfg(all(feature = "gpu-runtime", feature = "bevy-app"))]
pub(crate) fn run_headless_max_runtime(
    mut runtime: GpuLiveBrainRuntime,
    summary: &ProductionVoxelLaunchSummary,
    duration: Option<std::time::Duration>,
) -> Result<(), GameAppShellError> {
    use std::{path::PathBuf, time::Instant};
    let save_path = PathBuf::from(&summary.ui_settings.runtime_save_path);
    let stop_path = save_path.with_extension("stop");
    if stop_path.exists() {
        return Err(GameAppShellError::InvalidProductionFrontend {
            message: format!("headless stop file already exists: {}", stop_path.display()),
        });
    }
    println!(
        "Headless max speed: create {} to stop and checkpoint.",
        stop_path.display()
    );
    let started = Instant::now();
    let initial_tick = runtime.world().tick();
    while !duration.is_some_and(|limit| started.elapsed() >= limit) && !stop_path.exists() {
        match runtime.tick_outcome()? {
            GpuLiveTickOutcome::Progressed(_) => {}
            GpuLiveTickOutcome::NoProgress(
                GpuLiveNoProgressReason::CheckpointPublicationPending,
            ) => {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            GpuLiveTickOutcome::NoProgress(GpuLiveNoProgressReason::CheckpointFailed) => {
                return Err(GameAppShellError::InvalidProductionFrontend {
                    message: "headless checkpoint transaction failed".to_string(),
                });
            }
        }
    }
    let final_tick = runtime.world().tick();
    let run_elapsed_seconds = started.elapsed().as_secs_f64();
    runtime.request_manual_checkpoint(save_path.clone())?;
    let deadline = Instant::now() + std::time::Duration::from_secs(60);
    loop {
        runtime.poll_persistence_for_shutdown()?;
        if let GpuManualCheckpointStatus::Failed { message, .. } =
            runtime.manual_checkpoint_status()
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: message.clone(),
            });
        }
        if runtime.persistence_idle_for_shutdown()
            && matches!(runtime.manual_checkpoint_status(), GpuManualCheckpointStatus::Complete { checkpoint_tick, .. } if *checkpoint_tick >= final_tick)
        {
            break;
        }
        if Instant::now() >= deadline {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: format!(
                    "headless final checkpoint timed out: {}",
                    runtime.persistence_shutdown_diagnostics()
                ),
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let receipt = serde_json::json!({
        "mode": "headless-max", "initial_tick": initial_tick.raw(),
        "final_tick": final_tick.raw(), "elapsed_seconds": started.elapsed().as_secs_f64(),
        "run_elapsed_seconds": run_elapsed_seconds,
        "achieved_tps": (final_tick.raw() - initial_tick.raw()) as f64 / run_elapsed_seconds.max(f64::EPSILON),
        "checkpoint_verified": true, "checkpoint": save_path,
    });
    std::fs::write(
        save_path.with_extension("headless.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    println!(
        "Headless completed at tick {}; checkpoint verified at {}.",
        final_tick.raw(),
        save_path.display()
    );
    Ok(())
}
