use crate::{
    GameAppShellError, GpuBrainAuthorityTelemetry, GraphicalBrainPolicyMode,
    GraphicalPlaygroundLaunchConfig, LiveBrainLoop, LiveBrainTickControl, LiveBrainTickSummary,
    MotorRingPresentation,
};

#[cfg(feature = "gpu-runtime")]
use crate::GpuLiveBrainRuntime;

pub struct GraphicalGpuRuntimeController {
    mode: GraphicalBrainPolicyMode,
    #[cfg(feature = "gpu-runtime")]
    neural: Option<GpuLiveBrainRuntime>,
    telemetry: GpuBrainAuthorityTelemetry,
}

impl GraphicalGpuRuntimeController {
    pub fn new(launch: &GraphicalPlaygroundLaunchConfig) -> Result<Self, GameAppShellError> {
        let mode = if launch.brain_policy == alife_core::PolicyBackend::NeuralClosedLoopGpu {
            GraphicalBrainPolicyMode::GpuRequired
        } else {
            GraphicalBrainPolicyMode::HeuristicBaseline
        };
        #[cfg(feature = "gpu-runtime")]
        let neural = if mode == GraphicalBrainPolicyMode::GpuRequired {
            let backend =
                alife_gpu_backend::GpuClosedLoopBackend::new_required().map_err(|error| {
                    GameAppShellError::NeuralBackendUnavailable {
                        message: error.to_string(),
                    }
                })?;
            Some(GpuLiveBrainRuntime::from_p34_launch(
                backend,
                &launch.app_launch,
            )?)
        } else {
            None
        };
        #[cfg(not(feature = "gpu-runtime"))]
        if mode == GraphicalBrainPolicyMode::GpuRequired {
            return Err(GameAppShellError::NeuralBackendUnavailable {
                message: "GPU neural policy requires the gpu-runtime feature".to_string(),
            });
        }

        #[cfg(feature = "gpu-runtime")]
        let telemetry = neural.as_ref().map_or_else(
            || heuristic_baseline_telemetry(mode),
            GpuLiveBrainRuntime::authority_telemetry,
        );
        #[cfg(not(feature = "gpu-runtime"))]
        let telemetry = heuristic_baseline_telemetry(mode);

        Ok(Self {
            mode,
            #[cfg(feature = "gpu-runtime")]
            neural,
            telemetry,
        })
    }

    pub const fn mode(&self) -> GraphicalBrainPolicyMode {
        self.mode
    }

    pub const fn telemetry(&self) -> &GpuBrainAuthorityTelemetry {
        &self.telemetry
    }

    pub fn tick_with_motor_ring(
        &mut self,
        live: &mut LiveBrainLoop,
    ) -> Result<(LiveBrainTickSummary, MotorRingPresentation), GameAppShellError> {
        #[cfg(feature = "gpu-runtime")]
        let neural_summary = match &mut self.neural {
            Some(runtime) => {
                let summary = runtime.tick()?.into_iter().next().ok_or(
                    GameAppShellError::VisibleWorldMismatch {
                        message: "GPU neural policy produced no organism tick",
                    },
                )?;
                self.telemetry = runtime.authority_telemetry();
                Some(summary)
            }
            None => None,
        };
        #[cfg(not(feature = "gpu-runtime"))]
        let neural_summary = None;

        let summary = if let Some(summary) = neural_summary {
            summary
        } else {
            live.update(LiveBrainTickControl::step_once())?
                .into_iter()
                .next()
                .ok_or(GameAppShellError::VisibleWorldMismatch {
                    message: "heuristic baseline produced no organism tick",
                })?
        };
        let mut motor_ring = MotorRingPresentation::pending();
        motor_ring.selected_action_id = summary.selected_action_id;
        motor_ring.selected_label = summary
            .selected_action_kind
            .map_or_else(|| "Pending".to_string(), |kind| format!("{kind:?}"));
        motor_ring.source = if self.mode == GraphicalBrainPolicyMode::GpuRequired {
            "GPU candidate decoder"
        } else {
            "explicit heuristic baseline"
        };
        motor_ring.validate()?;
        Ok((summary, motor_ring))
    }
}

fn heuristic_baseline_telemetry(mode: GraphicalBrainPolicyMode) -> GpuBrainAuthorityTelemetry {
    let mut telemetry = GpuBrainAuthorityTelemetry::pending("heuristic-baseline");
    telemetry.requested_mode = mode;
    telemetry.selected_backend = "HeuristicBaseline".to_string();
    telemetry
}

#[cfg(all(test, not(feature = "gpu-runtime")))]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn gpu_required_launch_rejects_a_build_without_gpu_runtime() {
        let root =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34");
        let launch = GraphicalPlaygroundLaunchConfig::interactive(root);
        assert!(GraphicalGpuRuntimeController::new(&launch).is_err());
    }
}
