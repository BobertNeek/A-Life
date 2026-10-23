//! Offline same-parent genetic delta merging. Never merges acquired life state.

use std::collections::BTreeMap;

use alife_core::{
    Blake3Digest, BrainCapacityClass, BrainPhenotype, FoundationWeightAsset, PhenotypeHash,
    ProjectionType, ScaffoldContractError,
};

use crate::StageTrainableMask;

/// How same-parent actor deltas with opposing signs are handled.
/// Neither policy merges acquired life state, critic weights, or optimizer history.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FounderMergeConflictPolicy {
    #[default]
    AncestorRevert,
    Midpoint,
}

impl FounderMergeConflictPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AncestorRevert => "ancestor_revert",
            Self::Midpoint => "midpoint",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FounderMergeReceipt {
    pub conflict_policy: FounderMergeConflictPolicy,
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
    merge_founder_branches_with_policy(
        phenotype,
        parent,
        left,
        right,
        mask,
        FounderMergeConflictPolicy::AncestorRevert,
    )
}

/// Explicit policy variant of [`merge_founder_branches`], with identical admission
/// and frozen-coordinate guards. Midpoint is an optional numerical comparison,
/// not evidence of skill retention or a resumable optimizer trajectory.
pub fn merge_founder_branches_with_policy(
    phenotype: &BrainPhenotype,
    parent: &FoundationWeightAsset,
    left: &FounderMergeBranch,
    right: &FounderMergeBranch,
    mask: &StageTrainableMask,
    conflict_policy: FounderMergeConflictPolicy,
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
        conflict_policy,
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
        let (merged, kind) = merge_coordinate_with_policy(base, a, b, threshold, conflict_policy);
        if !merged.is_finite() {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if synapse.kind() == alife_core::CompiledSynapseKind::Recurrent
            && merged.to_bits() != base.to_bits()
        {
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

fn midpoint(left: f32, right: f32) -> f32 {
    // For a common parent p, this is p + ((left-p) + (right-p))/2 in real
    // arithmetic. Averaging the finite f32 endpoints in f64 avoids f32 delta/sum
    // overflow and cancellation from subtracting and re-adding a large parent.
    ((f64::from(left) + f64::from(right)) * 0.5) as f32
}

fn merge_coordinate_with_policy(
    parent: f32,
    left: f32,
    right: f32,
    threshold: f64,
    conflict_policy: FounderMergeConflictPolicy,
) -> (f32, u8) {
    let a = f64::from(left) - f64::from(parent);
    let b = f64::from(right) - f64::from(parent);
    match (a.abs() >= threshold, b.abs() >= threshold) {
        (false, false) => (parent, 0),
        (true, false) => (left, 1),
        (false, true) => (right, 1),
        (true, true) if a.is_sign_positive() == b.is_sign_positive() => (midpoint(left, right), 2),
        (true, true) => (
            match conflict_policy {
                FounderMergeConflictPolicy::AncestorRevert => parent,
                FounderMergeConflictPolicy::Midpoint => midpoint(left, right),
            },
            3,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        merge_coordinate_with_policy, validate_recorded_parent, FounderMergeConflictPolicy,
    };
    use alife_core::{Blake3Digest, PhenotypeHash};

    #[test]
    fn opt_in_midpoint_changes_only_sign_conflicts() {
        let merge = |p, a, b, policy| merge_coordinate_with_policy(p, a, b, 1e-6, policy);
        assert_eq!(
            FounderMergeConflictPolicy::default(),
            FounderMergeConflictPolicy::AncestorRevert
        );
        for (parent, left, right) in [
            (0., 0., 0.),
            (0., 1e-7, -1e-7),
            (0., 1., -1e-7),
            (0., 1e-7, -1.),
            (2., 3., 4.),
            (2., 1., 0.),
        ] {
            let default = merge(
                parent,
                left,
                right,
                FounderMergeConflictPolicy::AncestorRevert,
            );
            let opt_in = merge(parent, left, right, FounderMergeConflictPolicy::Midpoint);
            assert_eq!(default.0.to_bits(), opt_in.0.to_bits());
            assert_eq!(default.1, opt_in.1);
            assert_ne!(opt_in.1, 3);
        }
        for (parent, left, right, expected) in [
            (0., 1., -1., 0.),
            (2., 3., 1., 2.),
            (2., 5., 1., 3.),
            (2., 3., -1., 1.),
        ] {
            assert_eq!(
                merge(
                    parent,
                    left,
                    right,
                    FounderMergeConflictPolicy::AncestorRevert
                ),
                (parent, 3)
            );
            assert_eq!(
                merge(parent, left, right, FounderMergeConflictPolicy::Midpoint),
                (expected, 3)
            );
        }
    }

    #[test]
    fn conflict_midpoint_rounds_once_to_f32_with_ties_to_even() {
        let base = 1.5_f32.to_bits();
        for (left, right, expected) in [(base + 4, base - 1, base + 2), (base + 3, base - 2, base)]
        {
            let (merged, kind) = merge_coordinate_with_policy(
                f32::from_bits(base),
                f32::from_bits(left),
                f32::from_bits(right),
                1e-10,
                FounderMergeConflictPolicy::Midpoint,
            );
            assert_eq!(kind, 3);
            assert_eq!(merged.to_bits(), expected);
        }
        let (merged, kind) = merge_coordinate_with_policy(
            0.,
            f32::from_bits(3),
            -f32::from_bits(1),
            1e-46,
            FounderMergeConflictPolicy::Midpoint,
        );
        assert_eq!(kind, 3);
        assert_eq!(merged.to_bits(), 1);
    }

    #[test]
    fn finite_endpoint_midpoint_avoids_f32_intermediate_overflow() {
        for (parent, left, right, expected, kind) in [
            (0., f32::MAX, f32::MAX, f32::MAX, 2),
            (f32::MAX / 2., f32::MAX, -f32::MAX, 0., 3),
        ] {
            let (merged, actual_kind) = merge_coordinate_with_policy(
                parent,
                left,
                right,
                1e-6,
                FounderMergeConflictPolicy::Midpoint,
            );
            assert!(merged.is_finite());
            assert_eq!(merged.to_bits(), expected.to_bits());
            assert_eq!(actual_kind, kind);
        }
    }

    #[test]
    fn selective_merge_preserves_unique_changes_averages_overlap_and_rejects_conflicts() {
        let merge_coordinate = |parent, left, right, threshold| {
            merge_coordinate_with_policy(
                parent,
                left,
                right,
                threshold,
                FounderMergeConflictPolicy::AncestorRevert,
            )
        };
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
