//! Explicit product brain-policy intent and required-GPU construction boundary.

use alife_core::PolicyBackend;

use crate::GameAppShellError;

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
