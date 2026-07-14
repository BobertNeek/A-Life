//! GPU-authoritative, phenotype-owned closed-loop storage contracts.

mod abi;
mod bucket;
mod perception;
mod upload;

pub use abi::*;
pub use bucket::*;
pub(crate) use bucket::{GpuFixedClassArenaBuffers, GpuFixedClassArenaPlan, GpuFixedSlotRanges};
pub use perception::*;
pub use upload::*;

/// Categorized validation failure for closed-loop upload and class-bucket planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuClosedLoopError {
    StaleOrForeignHandle,
    LayoutMismatch,
    CapacityExceeded,
    ArithmeticOverflow,
    MalformedUpload,
    NonFinitePayload,
    InvalidOffsetDomain,
    SubmissionFailed,
}

impl std::fmt::Display for GpuClosedLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GPU closed-loop contract failure: {self:?}")
    }
}

impl std::error::Error for GpuClosedLoopError {}
