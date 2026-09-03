use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use std::ops::Range;

use crate::{validate_finite, ScaffoldContractError};

pub const MAX_DENDRITIC_INPUTS: usize = 32;
pub const MAX_DENDRITIC_BRANCHES: usize = 4096;
pub const MAX_DENDRITIC_BRANCHES_PER_NEURON: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
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

impl<'de> Deserialize<'de> for DendriticInputRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            source: u32,
            weight: f32,
        }

        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.source, wire.weight).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DendriticBranch {
    pub target: u32,
    pub threshold: f32,
    pub output_gain: f32,
    pub inputs: Vec<DendriticInputRef>,
}

impl<'de> Deserialize<'de> for DendriticBranch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            target: u32,
            threshold: f32,
            output_gain: f32,
            inputs: Vec<DendriticInputRef>,
        }

        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.target, wire.threshold, wire.output_gain, wire.inputs)
            .map_err(D::Error::custom)
    }
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
        };
        branch.validate_metadata()?;
        Ok(branch)
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

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct DendriticBranchSet {
    branches: Vec<DendriticBranch>,
}

impl<'de> Deserialize<'de> for DendriticBranchSet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            branches: Vec<DendriticBranch>,
        }

        let wire = Wire::deserialize(deserializer)?;
        let canonical = Self::new(wire.branches.clone()).map_err(D::Error::custom)?;
        if canonical.branches != wire.branches {
            return Err(D::Error::custom("dendritic branches are not canonical"));
        }
        Ok(canonical)
    }
}

impl DendriticBranchSet {
    pub fn new(mut branches: Vec<DendriticBranch>) -> Result<Self, ScaffoldContractError> {
        if branches.len() > MAX_DENDRITIC_BRANCHES {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        branches.sort_by_key(|branch| branch.target);

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

    pub fn validate_for_neuron_count(
        &self,
        neuron_count: u32,
    ) -> Result<(), ScaffoldContractError> {
        let neuron_count = usize::try_from(neuron_count)
            .map_err(|_| ScaffoldContractError::InvalidSparseProjectionSchema)?;
        if neuron_count == 0 {
            return Err(ScaffoldContractError::InvalidSparseProjectionSchema);
        }
        for branch in &self.branches {
            branch.validate_for_shape(neuron_count, neuron_count)?;
        }
        Ok(())
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
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DendriticWorkReceipt {
    pub branches_evaluated: u32,
    pub inputs_evaluated: u32,
    pub gated_branches: u32,
    pub work_units: u32,
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

    #[test]
    fn deserialization_rejects_branch_sets_that_bypass_constructor_bounds() {
        let malformed = serde_json::json!({
            "branches": [{
                "target": 0,
                "threshold": 0.0,
                "output_gain": 1.0,
                "inputs": []
            }]
        });

        assert!(serde_json::from_value::<DendriticBranchSet>(malformed).is_err());
    }

    #[test]
    fn neuron_count_validation_rejects_out_of_range_sources_and_targets() {
        let branches = DendriticBranchSet::new(vec![DendriticBranch::new(
            4,
            0.0,
            1.0,
            vec![DendriticInputRef::new(5, 1.0).unwrap()],
        )
        .unwrap()])
        .unwrap();

        assert_eq!(
            branches.validate_for_neuron_count(5),
            Err(ScaffoldContractError::InvalidSparseProjectionSchema)
        );
    }
}
