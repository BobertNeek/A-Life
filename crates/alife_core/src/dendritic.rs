use serde::{Deserialize, Serialize};
use std::ops::Range;

use crate::{validate_finite, ScaffoldContractError};

pub const MAX_DENDRITIC_INPUTS: usize = 32;
pub const MAX_DENDRITIC_BRANCHES: usize = 4096;
pub const MAX_DENDRITIC_BRANCHES_PER_NEURON: usize = 4;
pub const MAX_DENDRITIC_ALLOCATION_EVIDENCE: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DendriticInputRef {
    pub source: u32,
    pub weight: f32,
}

impl DendriticInputRef {
    pub fn new(source: u32, weight: f32) -> Result<Self, ScaffoldContractError> {
        validate_finite(weight)?;
        Ok(Self { source, weight })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DendriticBranch {
    pub target: u32,
    pub threshold: f32,
    pub output_gain: f32,
    pub inputs: Vec<DendriticInputRef>,
    #[serde(default)]
    pub branch_id: u64,
    #[serde(default)]
    pub structural_utility: u32,
    #[serde(default)]
    pub observations: u16,
    #[serde(default)]
    pub last_tick: u64,
    #[serde(default)]
    pub source_identity: u64,
}

impl DendriticBranch {
    pub fn new(
        target: u32,
        threshold: f32,
        output_gain: f32,
        inputs: Vec<DendriticInputRef>,
    ) -> Result<Self, ScaffoldContractError> {
        let branch = Self {
            target,
            threshold,
            output_gain,
            inputs,
            branch_id: 0,
            structural_utility: 0,
            observations: 0,
            last_tick: 0,
            source_identity: 0,
        };
        branch.with_derived_identity()
    }

    fn with_derived_identity(mut self) -> Result<Self, ScaffoldContractError> {
        self.validate_metadata()?;
        if self.branch_id == 0 {
            self.branch_id = branch_identity(self.target, &self.inputs, self.source_identity);
        }
        Ok(self)
    }

    fn validate_metadata(&self) -> Result<(), ScaffoldContractError> {
        if self.inputs.is_empty() || self.inputs.len() > MAX_DENDRITIC_INPUTS {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        validate_finite(self.threshold)?;
        validate_finite(self.output_gain)?;
        for input in &self.inputs {
            validate_finite(input.weight)?;
        }
        Ok(())
    }

    fn validate_for_shape(
        &self,
        activation_count: usize,
        accumulator_count: usize,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_metadata()?;
        if self.target as usize >= accumulator_count {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        if self
            .inputs
            .iter()
            .any(|input| input.source as usize >= activation_count)
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DendriticBranchSet {
    branches: Vec<DendriticBranch>,
}

impl DendriticBranchSet {
    pub fn new(mut branches: Vec<DendriticBranch>) -> Result<Self, ScaffoldContractError> {
        if branches.len() > MAX_DENDRITIC_BRANCHES {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        for branch in &mut branches {
            *branch = branch.clone().with_derived_identity()?;
        }
        branches.sort_by_key(|branch| (branch.target, branch.branch_id));

        let mut previous_target = None;
        let mut branches_for_target = 0;
        for branch in &branches {
            branch.validate_metadata()?;
            if previous_target == Some(branch.target) {
                branches_for_target += 1;
            } else {
                previous_target = Some(branch.target);
                branches_for_target = 1;
            }
            if branches_for_target > MAX_DENDRITIC_BRANCHES_PER_NEURON {
                return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
            }
        }

        Ok(Self { branches })
    }

    pub fn branches(&self) -> &[DendriticBranch] {
        &self.branches
    }

    /// Returns the stable branch span for one target without adding derived
    /// offsets to the serialized logical checkpoint state.
    pub fn target_span(&self, target: u32) -> Range<usize> {
        let start = self
            .branches
            .partition_point(|branch| branch.target < target);
        let end = self.branches[start..].partition_point(|branch| branch.target == target) + start;
        start..end
    }

    pub fn is_empty(&self) -> bool {
        self.branches.is_empty()
    }

    /// Allocates bounded conjunction branches from replay-stable experience.
    /// Inputs are supplied by events, never by a neuron-wide pair scan.
    pub fn allocate_from_evidence(
        &mut self,
        activation_count: u32,
        branch_capacity: usize,
        evidence: &[DendriticAllocationEvidence],
    ) -> Result<DendriticAllocationReceipt, ScaffoldContractError> {
        if branch_capacity == 0
            || branch_capacity > MAX_DENDRITIC_BRANCHES
            || evidence.len() > MAX_DENDRITIC_ALLOCATION_EVIDENCE
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        let mut ordered = evidence.to_vec();
        ordered.sort_unstable_by(|left, right| {
            right
                .score()
                .cmp(&left.score())
                .then_with(|| left.target.cmp(&right.target))
                .then_with(|| left.source_identity.cmp(&right.source_identity))
                .then_with(|| left.tick.cmp(&right.tick))
        });
        let mut receipt = DendriticAllocationReceipt::default();
        for item in ordered {
            receipt.evidence_examined = receipt.evidence_examined.saturating_add(1);
            item.validate(activation_count)?;
            let score = item.score();
            if item.inputs.len() < 2 || score == 0 {
                continue;
            }
            let branch_id = branch_identity(item.target, &item.inputs, item.source_identity);
            let existing = self.branches.iter_mut().find(|branch| branch.branch_id == branch_id);
            if let Some(branch) = existing {
                branch.structural_utility = branch.structural_utility.saturating_add(score);
                branch.observations = branch.observations.saturating_add(item.observations.max(1));
                branch.last_tick = branch.last_tick.max(item.tick);
                receipt.maintained = receipt.maintained.saturating_add(1);
                continue;
            }
            receipt.branch_candidates = receipt.branch_candidates.saturating_add(1);
            let target_count = self
                .branches
                .iter()
                .filter(|branch| branch.target == item.target)
                .count();
            let replacement = if target_count >= MAX_DENDRITIC_BRANCHES_PER_NEURON
                || self.branches.len() >= branch_capacity
            {
                self.branches
                    .iter()
                    .enumerate()
                    .filter(|(_, branch)| branch.target == item.target)
                    .min_by_key(|(_, branch)| {
                        (branch.structural_utility, branch.observations, branch.branch_id)
                    })
                    .and_then(|(index, branch)| {
                        (score > branch.structural_utility.saturating_add(1)).then_some(index)
                    })
            } else {
                None
            };
            if let Some(index) = replacement {
                self.branches[index] = item.to_branch(branch_id, score)?;
                receipt.replaced = receipt.replaced.saturating_add(1);
                receipt.allocated = receipt.allocated.saturating_add(1);
            } else if self.branches.len() < branch_capacity
                && target_count < MAX_DENDRITIC_BRANCHES_PER_NEURON
            {
                self.branches.push(item.to_branch(branch_id, score)?);
                receipt.allocated = receipt.allocated.saturating_add(1);
            }
        }
        self.branches.sort_by_key(|branch| (branch.target, branch.branch_id));
        receipt.active_branches = self.branches.len() as u32;
        receipt.work_units = receipt
            .evidence_examined
            .saturating_add(receipt.branch_candidates)
            .saturating_add(receipt.allocated)
            .saturating_add(receipt.replaced)
            .saturating_add(receipt.maintained);
        receipt.deterministic_digest = branch_set_digest(&self.branches);
        Ok(receipt)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DendriticAllocationEvidence {
    pub target: u32,
    pub inputs: Vec<DendriticInputRef>,
    pub conjunction_score: u32,
    pub working_memory: u32,
    pub prediction_residual: u32,
    pub concept_gap_support: u32,
    pub route_locality: u32,
    pub novelty: u32,
    pub observations: u16,
    pub tick: u64,
    pub source_identity: u64,
}

impl DendriticAllocationEvidence {
    fn score(&self) -> u32 {
        self.conjunction_score
            .saturating_add(self.working_memory)
            .saturating_add(self.prediction_residual)
            .saturating_add(self.concept_gap_support)
            .saturating_add(self.route_locality)
            .saturating_add(self.novelty)
    }

    fn validate(&self, activation_count: u32) -> Result<(), ScaffoldContractError> {
        if self.target >= activation_count
            || self.source_identity == 0
            || self.inputs.len() < 2
            || self.inputs.len() > MAX_DENDRITIC_INPUTS
            || self.observations == 0
            || self.inputs.iter().any(|input| input.source >= activation_count)
        {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        for input in &self.inputs {
            validate_finite(input.weight)?;
        }
        Ok(())
    }

    fn to_branch(&self, branch_id: u64, score: u32) -> Result<DendriticBranch, ScaffoldContractError> {
        let branch = DendriticBranch {
            target: self.target,
            threshold: 0.5,
            output_gain: 1.0 + (self.working_memory.min(1_000) as f32 / 1_000.0),
            inputs: self.inputs.clone(),
            branch_id,
            structural_utility: score,
            observations: self.observations,
            last_tick: self.tick,
            source_identity: self.source_identity,
        };
        branch.with_derived_identity()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DendriticAllocationReceipt {
    pub evidence_examined: u32,
    pub branch_candidates: u32,
    pub allocated: u32,
    pub replaced: u32,
    pub maintained: u32,
    pub active_branches: u32,
    pub work_units: u32,
    pub deterministic_digest: u64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DendriticWorkReceipt {
    pub branches_evaluated: u32,
    pub inputs_evaluated: u32,
    pub gated_branches: u32,
    pub work_units: u32,
    #[serde(default)]
    pub allocation_candidates: u32,
    #[serde(default)]
    pub allocated_branches: u32,
    #[serde(default)]
    pub replaced_branches: u32,
    #[serde(default)]
    pub allocation_work_units: u32,
}

pub fn apply_dendritic_conjunctions(
    activations: &[f32],
    accumulators: &mut [f32],
    branches: &DendriticBranchSet,
) -> Result<DendriticWorkReceipt, ScaffoldContractError> {
    let mut receipt = DendriticWorkReceipt::default();
    for branch in branches.branches() {
        branch.validate_for_shape(activations.len(), accumulators.len())?;
        let mut weighted_sum = 0.0;
        receipt.branches_evaluated = receipt.branches_evaluated.saturating_add(1);

        for input in &branch.inputs {
            let activation = validate_finite(activations[input.source as usize])?;
            let weighted_input = validate_finite(activation * input.weight)?;
            weighted_sum = validate_finite(weighted_sum + weighted_input)?;
            receipt.inputs_evaluated = receipt.inputs_evaluated.saturating_add(1);
        }

        let excess = (weighted_sum - branch.threshold).max(0.0);
        let branch_output = validate_finite(excess.tanh() * branch.output_gain)?;
        if excess > 0.0 {
            receipt.gated_branches = receipt.gated_branches.saturating_add(1);
        }
        let target = branch.target as usize;
        accumulators[target] = validate_finite(accumulators[target] + branch_output)?;
    }
    receipt.work_units = receipt
        .branches_evaluated
        .saturating_add(receipt.inputs_evaluated);
    Ok(receipt)
}

fn branch_identity(target: u32, inputs: &[DendriticInputRef], source_identity: u64) -> u64 {
    let mut digest = 14_695_981_039_346_656_037_u64;
    digest = mix_digest(digest, u64::from(target));
    digest = mix_digest(digest, source_identity);
    let mut sources = inputs.iter().map(|input| input.source).collect::<Vec<_>>();
    sources.sort_unstable();
    for source in sources {
        digest = mix_digest(digest, u64::from(source));
    }
    if digest == 0 { 1 } else { digest }
}

fn branch_set_digest(branches: &[DendriticBranch]) -> u64 {
    let mut digest = 14_695_981_039_346_656_037_u64;
    for branch in branches {
        digest = mix_digest(digest, branch.branch_id);
        digest = mix_digest(digest, u64::from(branch.target));
        digest = mix_digest(digest, u64::from(branch.structural_utility));
        digest = mix_digest(digest, u64::from(branch.observations));
        digest = mix_digest(digest, branch.last_tick);
        for input in &branch.inputs {
            digest = mix_digest(digest, u64::from(input.source));
            digest = mix_digest(digest, u64::from(input.weight.to_bits()));
        }
    }
    digest
}

fn mix_digest(mut digest: u64, value: u64) -> u64 {
    for byte in value.to_le_bytes() {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(1_099_511_628_211);
    }
    digest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain_class::{BrainClassSpec, BrainScaleTier};
    use crate::neural::{finalize_cpu_activations, CpuNeuralState, NeuralActivationConfig};

    #[test]
    fn joint_inputs_gate_a_branch_and_change_final_neuron_output() {
        let branches = DendriticBranchSet::new(vec![DendriticBranch::new(
            0,
            1.0,
            1.0,
            vec![
                DendriticInputRef::new(1, 1.0).unwrap(),
                DendriticInputRef::new(2, 1.0).unwrap(),
            ],
        )
        .unwrap()])
        .unwrap();

        let mut one_input_accumulators = vec![0.0];
        let one_input_receipt =
            apply_dendritic_conjunctions(&[0.0, 0.6, 0.0], &mut one_input_accumulators, &branches)
                .unwrap();
        assert_eq!(one_input_receipt.gated_branches, 0);
        assert_eq!(one_input_accumulators[0], 0.0);

        let mut joint_accumulators = vec![0.0];
        let joint_receipt =
            apply_dendritic_conjunctions(&[0.0, 0.6, 0.6], &mut joint_accumulators, &branches)
                .unwrap();
        assert_eq!(joint_receipt.gated_branches, 1);
        assert!(joint_accumulators[0] > 0.0);

        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let mut state = CpuNeuralState::for_brain_class(&spec).unwrap();
        state.activations[1] = 0.6;
        state.activations[2] = 0.6;
        state.dendritic_branches = branches;

        let report =
            finalize_cpu_activations(&mut state, NeuralActivationConfig::reference()).unwrap();

        assert_eq!(report.dendritic_work.gated_branches, 1);
        assert!(state.activations[0] > 0.0);
    }

    #[test]
    fn target_spans_are_sorted_bounded_and_skip_empty_targets() {
        let branches = DendriticBranchSet::new(vec![
            DendriticBranch::new(5, 0.0, 1.0, vec![DendriticInputRef::new(0, 1.0).unwrap()])
                .unwrap(),
            DendriticBranch::new(2, 0.0, 1.0, vec![DendriticInputRef::new(1, 1.0).unwrap()])
                .unwrap(),
            DendriticBranch::new(5, 0.0, 1.0, vec![DendriticInputRef::new(2, 1.0).unwrap()])
                .unwrap(),
        ])
        .unwrap();

        assert_eq!(branches.target_span(0), 0..0);
        assert_eq!(branches.target_span(2), 0..1);
        assert_eq!(branches.target_span(3), 1..1);
        assert_eq!(branches.target_span(5), 1..3);
        assert_eq!(branches.branches()[branches.target_span(5)].len(), 2);
    }
}
