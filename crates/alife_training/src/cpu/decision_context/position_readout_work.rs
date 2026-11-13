//! Logical detached score-kernel units, not FLOPs or an ATP/host admission policy.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionReadoutWork2018 {
    pub source_reads: u64,
    pub source_arithmetic: u64,
    pub sqrt_preparations: u64,
    pub candidate_reads: u64,
    pub candidate_arithmetic: u64,
    pub bound_and_merge_ops: u64,
}
impl PositionReadoutWork2018 {
    /// The batch kernel prepares the shared source once, even for an empty batch.
    /// Counts exclude validation, provenance/own-effect construction, allocation and hashing.
    pub fn derive(candidates: u64) -> Result<Self, TrainingError> {
        let mul = |n| candidates.checked_mul(n).ok_or_else(invalid);
        Ok(Self {
            source_reads: 4,
            source_arithmetic: 9,
            sqrt_preparations: 1,
            candidate_reads: mul(7)?,
            candidate_arithmetic: mul(10)?,
            bound_and_merge_ops: mul(2)?,
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositionOutputWork2029 {
    pub kind_reads: u64,
    pub kind_dispatches: u64,
    pub amplitude_reads: u64,
    pub output_multiplies: u64,
    pub final_merges: u64,
}
impl PositionOutputWork2029 {
    /// Additional operations of score_output2029 over the inner score with base0.
    /// Zero amplitudes still execute their read/multiply; Other routes neither.
    pub fn derive(candidates: u64, amplitude_routed: u64) -> Result<Self, TrainingError> {
        if amplitude_routed > candidates {
            return Err(invalid());
        }
        Ok(Self {
            kind_reads: candidates,
            kind_dispatches: candidates,
            amplitude_reads: amplitude_routed,
            output_multiplies: amplitude_routed,
            final_merges: candidates,
        })
    }
    pub fn for_kinds(kinds: &[PositionEffectKind2029]) -> Result<Self, TrainingError> {
        let n = u64::try_from(kinds.len()).map_err(|_| invalid())?;
        let routed = u64::try_from(
            kinds
                .iter()
                .filter(|k| **k != PositionEffectKind2029::Other)
                .count(),
        )
        .map_err(|_| invalid())?;
        Self::derive(n, routed)
    }
}
