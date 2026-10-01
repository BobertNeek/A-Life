//! Offline same-parent genetic delta merging. Never merges acquired life state.

use std::collections::BTreeMap;

use alife_core::{
    Blake3Digest, BrainCapacityClass, BrainPhenotype, FoundationWeightAsset, PhenotypeHash,
    ProjectionType, ScaffoldContractError,
};

use crate::StageTrainableMask;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FounderMergeReceipt {
    pub unique_changes: usize,
    pub averaged_changes: usize,
    pub sign_conflicts: usize,
    pub unchanged: usize,
    /// Threshold by compiled route, derived exclusively from the common parent.
    pub route_thresholds: BTreeMap<u16, f64>,
}

/// Recorded checkpoint ancestry. These fields are checked for consistency, not
/// authenticated: callers must obtain them from the actual branch checkpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct FounderMergeBranch {
    pub asset: FoundationWeightAsset,
    pub parent_asset_digest: Blake3Digest,
    pub parent_phenotype_hash: PhenotypeHash,
    pub parent_compiler_inputs_digest: [u64; 4],
}

fn validate_recorded_parent(
    recorded: (Blake3Digest, PhenotypeHash, [u64; 4]),
    expected: (Blake3Digest, PhenotypeHash, [u64; 4]),
) -> Result<(), ScaffoldContractError> {
    if recorded != expected {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    Ok(())
}

/// Both branches must have been exported from this exact compiled graph and
/// common parent. The explicit branch descriptors bind recorded ancestry to
/// the supplied parent and compiler inputs; this is not ancestry authentication.
pub fn merge_founder_branches(
    phenotype: &BrainPhenotype,
    parent: &FoundationWeightAsset,
    left: &FounderMergeBranch,
    right: &FounderMergeBranch,
    mask: &StageTrainableMask,
) -> Result<(FoundationWeightAsset, FounderMergeReceipt), ScaffoldContractError> {
    if phenotype.brain_class_id() != BrainCapacityClass::N2048_ID {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    parent.validate_against(phenotype)?;
    mask.validate_for(phenotype)?;
    for branch in [left, right] {
        validate_recorded_parent(
            (
                branch.parent_asset_digest,
                branch.parent_phenotype_hash,
                branch.parent_compiler_inputs_digest,
            ),
            (
                parent.digest(),
                phenotype.phenotype_hash(),
                phenotype.compiler_inputs_digest(),
            ),
        )?;
        // Reconstructing metadata rejects mismatched graph/decoder identities,
        // invalid numbers, and promoted assets. Do not compare weight digests
        // against the parent's ABI: those necessarily change during training.
        let expected = FoundationWeightAsset::from_trained_weights(
            phenotype,
            branch.asset.weights().to_vec(),
            parent.manifest().training_stage(),
        )?;
        if expected != branch.asset {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
    }
    let mut squared_by_route = BTreeMap::<u16, (f64, usize)>::new();
    for (synapse, weight) in phenotype.synapses().iter().zip(parent.weights()) {
        let entry = squared_by_route.entry(synapse.route_index()).or_default();
        entry.0 += f64::from(*weight).powi(2);
        entry.1 += 1;
    }
    let mut receipt = FounderMergeReceipt {
        route_thresholds: squared_by_route
            .into_iter()
            .map(|(route, (sum, count))| (route, 1e-6_f64.max(1e-4 * (sum / count as f64).sqrt())))
            .collect(),
        ..FounderMergeReceipt::default()
    };
    let mut weights = parent.weights().to_vec();
    for (index, synapse) in phenotype.synapses().iter().enumerate() {
        let base = parent.weights()[index];
        let a = left.asset.weights()[index];
        let b = right.asset.weights()[index];
        if !mask.is_trainable(index) {
            if a.to_bits() != base.to_bits() || b.to_bits() != base.to_bits() {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
            receipt.unchanged += 1;
            continue;
        }
        let threshold = receipt.route_thresholds[&synapse.route_index()];
        let (merged, kind) = merge_coordinate(base, a, b, threshold);
        if !merged.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if merged.to_bits() != base.to_bits() {
            let projection = phenotype
                .projections()
                .get(usize::from(synapse.route_index()))
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            if match projection.projection_type() {
                ProjectionType::LateralInhibition => merged >= 0.0,
                ProjectionType::Homeostatic | ProjectionType::MotorProposal => merged < 0.0,
                _ => false,
            } {
                return Err(ScaffoldContractError::PhenotypeCompile);
            }
        }
        weights[index] = merged;
        match kind {
            1 => receipt.unique_changes += 1,
            2 => receipt.averaged_changes += 1,
            3 => receipt.sign_conflicts += 1,
            _ => receipt.unchanged += 1,
        }
    }
    Ok((
        FoundationWeightAsset::from_trained_weights(
            phenotype,
            weights,
            parent.manifest().training_stage(),
        )?,
        receipt,
    ))
}

fn merge_coordinate(parent: f32, left: f32, right: f32, threshold: f64) -> (f32, u8) {
    let a = f64::from(left) - f64::from(parent);
    let b = f64::from(right) - f64::from(parent);
    match (a.abs() >= threshold, b.abs() >= threshold) {
        (false, false) => (parent, 0),
        (true, false) => (left, 1),
        (false, true) => (right, 1),
        (true, true) if a.is_sign_positive() == b.is_sign_positive() => {
            (((f64::from(left) + f64::from(right)) * 0.5) as f32, 2)
        }
        (true, true) => (parent, 3),
    }
}

#[cfg(test)]
mod tests {
    use super::{merge_coordinate, validate_recorded_parent};
    use alife_core::{Blake3Digest, PhenotypeHash};

    #[test]
    fn selective_merge_preserves_unique_changes_averages_overlap_and_rejects_conflicts() {
        let a = [0.0, 1.0, 0.0, 1.0, 0.0];
        let b = [1.0, 1.0, 0.0, 0.0, 0.0];
        let merge = |left: [f32; 5], right: [f32; 5]| {
            std::array::from_fn::<_, 5, _>(|i| merge_coordinate(0.0, left[i], right[i], 1e-6).0)
        };
        assert_eq!(merge(a, a), a);
        assert_eq!(merge(a, b), [1.0, 1.0, 0.0, 1.0, 0.0]);
        assert_eq!(merge_coordinate(2.0, 3.0, 1.0, 1e-6), (2.0, 3));
        assert_eq!(merge_coordinate(2.0, 3.0, 2.0, 1e-6), (3.0, 1));
        assert_eq!(merge_coordinate(2.0, 3.0, 4.0, 1e-6), (3.5, 2));
        assert_eq!(merge_coordinate(0.0, 1e-7, 2e-7, 1e-6), (0.0, 0));
        let parent = (
            Blake3Digest::from_bytes([1; 32]),
            PhenotypeHash([2; 4]),
            [3; 4],
        );
        assert!(validate_recorded_parent(parent, parent).is_ok());
        for malformed in [
            (Blake3Digest::from_bytes([9; 32]), parent.1, parent.2),
            (parent.0, PhenotypeHash([9; 4]), parent.2),
            (parent.0, parent.1, [9; 4]),
        ] {
            assert!(validate_recorded_parent(malformed, parent).is_err());
        }
    }
}
