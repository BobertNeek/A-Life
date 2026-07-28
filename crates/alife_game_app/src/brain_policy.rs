//! Explicit product brain-policy intent and required-GPU construction boundary.

use alife_core::PolicyBackend;

use crate::{GameAppShellError, LiveBrainLoop, LiveBrainTickControl, LiveBrainTickSummary};

#[cfg(feature = "gpu-runtime")]
use crate::GraphicalPlaygroundLaunchConfig;

pub enum BrainPolicyRuntime {
    #[cfg(feature = "gpu-runtime")]
    Neural(Box<crate::GpuLiveBrainRuntime>),
    Heuristic(Box<LiveBrainLoop>),
}

impl BrainPolicyRuntime {
    pub const fn policy(&self) -> PolicyBackend {
        match self {
            #[cfg(feature = "gpu-runtime")]
            Self::Neural(_) => PolicyBackend::NeuralClosedLoopGpu,
            Self::Heuristic(_) => PolicyBackend::HeuristicBaseline,
        }
    }

    pub fn tick(&mut self) -> Result<LiveBrainTickSummary, GameAppShellError> {
        match self {
            #[cfg(feature = "gpu-runtime")]
            Self::Neural(runtime) => {
                runtime
                    .tick()?
                    .into_iter()
                    .next()
                    .ok_or(GameAppShellError::VisibleWorldMismatch {
                        message: "GPU neural policy produced no organism tick",
                    })
            }
            Self::Heuristic(runtime) => runtime
                .update(LiveBrainTickControl::step_once())?
                .into_iter()
                .next()
                .ok_or(GameAppShellError::VisibleWorldMismatch {
                    message: "heuristic baseline produced no organism tick",
                }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicalBrainPolicyMode {
    GpuRequired,
    HeuristicBaseline,
}

impl GraphicalBrainPolicyMode {
    pub const fn label(self) -> &'static str {
        match self {
            Self::GpuRequired => "gpu-required",
            Self::HeuristicBaseline => "heuristic-baseline",
        }
    }

    pub fn parse(value: &str) -> Result<Self, GameAppShellError> {
        match value {
            "gpu-required" => Ok(Self::GpuRequired),
            "heuristic-baseline" => Ok(Self::HeuristicBaseline),
            _ => Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "brain policy must be gpu-required or heuristic-baseline",
            }),
        }
    }

    pub const fn policy(self) -> PolicyBackend {
        match self {
            Self::GpuRequired => PolicyBackend::NeuralClosedLoopGpu,
            Self::HeuristicBaseline => PolicyBackend::HeuristicBaseline,
        }
    }
}

impl From<GraphicalBrainPolicyMode> for PolicyBackend {
    fn from(value: GraphicalBrainPolicyMode) -> Self {
        value.policy()
    }
}

#[cfg(feature = "gpu-runtime")]
#[doc(hidden)]
pub trait RequiredGpuFactory {
    fn new_required(
        &self,
    ) -> Result<alife_gpu_backend::GpuClosedLoopBackend, alife_core::ScaffoldContractError>;
}

#[cfg(feature = "gpu-runtime")]
#[derive(Debug, Clone, Copy, Default)]
#[doc(hidden)]
pub struct ProductionRequiredGpuFactory;

#[cfg(feature = "gpu-runtime")]
impl RequiredGpuFactory for ProductionRequiredGpuFactory {
    fn new_required(
        &self,
    ) -> Result<alife_gpu_backend::GpuClosedLoopBackend, alife_core::ScaffoldContractError> {
        alife_gpu_backend::GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
    }
}

#[cfg(feature = "gpu-runtime")]
#[doc(hidden)]
pub fn run_gpu_closed_loop_smoke_with_factory(
    launch: GraphicalPlaygroundLaunchConfig,
    factory: &impl RequiredGpuFactory,
) -> Result<BrainPolicyRuntime, GameAppShellError> {
    launch.validate()?;
    if launch.brain_policy != PolicyBackend::NeuralClosedLoopGpu {
        return Err(GameAppShellError::InvalidGraphicalLaunch {
            message: "GPU closed-loop launch requires gpu-required policy",
        });
    }
    let backend =
        factory
            .new_required()
            .map_err(|error| GameAppShellError::NeuralBackendUnavailable {
                message: error.to_string(),
            })?;
    Ok(BrainPolicyRuntime::Neural(Box::new(
        crate::GpuLiveBrainRuntime::from_p34_launch(backend, &launch.app_launch)?,
    )))
}
