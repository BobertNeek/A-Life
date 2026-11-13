//! Logical kernel work counts, not measured FLOPs or automatic biological debits.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionContextWorkV1 {
    pub retained_source_reads: u64,
    pub retained_parameter_terms: u64,
    pub retained_features: u64,
    pub retained_candidate_ops: u64,
    pub current_source_reads: u64,
    pub current_parameter_terms: u64,
    pub current_candidate_ops: u64,
}
impl DecisionContextWorkV1 {
    /// Count executed kernels regardless of zero coefficients or typed intention.
    /// Source absence skips retained products, not the endpoint source reads or merge.
    pub fn derive(candidates: u64, retained_source_present: bool) -> Result<Self, TrainingError> {
        let mul = |n| candidates.checked_mul(n).ok_or_else(invalid);
        Ok(Self {
            retained_source_reads: mul(8)?,
            retained_parameter_terms: mul(if retained_source_present { 48 } else { 0 })?,
            retained_features: mul(if retained_source_present { 6 } else { 0 })?,
            retained_candidate_ops: mul(2)?,
            current_source_reads: candidates,
            current_parameter_terms: candidates,
            current_candidate_ops: mul(2)?,
        })
    }
}
