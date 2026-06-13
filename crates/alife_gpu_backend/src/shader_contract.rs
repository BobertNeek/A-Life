//! v0 scaffold: WGSL pass contract descriptions without runtime compute kernels.
//!
//! A-Life source shaders are WGSL only. Native backend translation artifacts are
//! downstream driver concerns and are not authored as source here.

pub const P24_WGSL_CONTRACT_STUB: &str = include_str!("../shaders/p24_contract.wgsl");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuShaderPass {
    ClearAccumulators,
    SparseProjectionSpmv,
    ActivationFinalize,
    PlasticityUpdate,
}

impl GpuShaderPass {
    pub const fn contract_order() -> [Self; 4] {
        [
            Self::ClearAccumulators,
            Self::SparseProjectionSpmv,
            Self::ActivationFinalize,
            Self::PlasticityUpdate,
        ]
    }

    pub const fn culling_recompaction_hooks_are_deferred() -> bool {
        true
    }

    pub const fn contract_summary(self) -> &'static str {
        match self {
            Self::ClearAccumulators => {
                "pass 0 clears i32 atomic accumulators and resets diagnostic counters"
            }
            Self::SparseProjectionSpmv => {
                "pass 1 consumes tile metadata, masks, packed indices, fixed/lifetime/H/alpha buffers, and writes atomic accumulators"
            }
            Self::ActivationFinalize => {
                "pass 2 clamps fixed-point accumulators into activation ping-pong buffers and records overflow/range flags"
            }
            Self::PlasticityUpdate => {
                "pass 3 reads finalized pre/post activation buffers and writes H_shadow only; genetic, lifetime, and H_operational layers stay read-only"
            }
        }
    }
}
