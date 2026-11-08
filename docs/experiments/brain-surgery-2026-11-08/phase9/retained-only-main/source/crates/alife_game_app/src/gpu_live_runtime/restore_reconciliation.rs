//! Classify already validated checkpoint state after restore. Retained sealed
//! credit is preserved; unsealed pending work is discarded and must be saved.
#[derive(Clone, Copy)]
pub(super) struct CheckpointShape {
    pub pending_eligibility: bool,
    pub pending_transaction: bool,
    pub retained_credit: bool,
}

pub(super) fn population_requires_reconciliation(
    expected_population: usize,
    checkpoints: impl IntoIterator<Item = CheckpointShape>,
) -> bool {
    let mut count = 0;
    let mut discarded_pending = false;
    for state in checkpoints {
        count += 1;
        discarded_pending |=
            state.pending_transaction || (state.pending_eligibility && !state.retained_credit);
    }
    count != expected_population || discarded_pending
}

#[cfg(test)]
mod tests {
    use super::*;
    const QUIET: CheckpointShape = CheckpointShape {
        pending_eligibility: false,
        pending_transaction: false,
        retained_credit: false,
    };
    const RETAINED: CheckpointShape = CheckpointShape {
        pending_eligibility: true,
        retained_credit: true,
        ..QUIET
    };
    const UNSEALED: CheckpointShape = CheckpointShape {
        pending_eligibility: true,
        pending_transaction: true,
        ..QUIET
    };

    #[test]
    fn preserved_retained_credit_and_quiet_population_do_not_republish() {
        assert!(!population_requires_reconciliation(1, [RETAINED]));
        assert!(!population_requires_reconciliation(2, [RETAINED, QUIET]));
        assert!(!population_requires_reconciliation(1, [QUIET]));
    }

    #[test]
    fn discarded_unsealed_pending_and_missing_residents_keep_reconciliation() {
        assert!(population_requires_reconciliation(1, [UNSEALED]));
        assert!(population_requires_reconciliation(2, [RETAINED, UNSEALED]));
        assert!(population_requires_reconciliation(2, [RETAINED]));
        assert!(population_requires_reconciliation(1, []));
        assert!(population_requires_reconciliation(
            1,
            [CheckpointShape {
                pending_eligibility: true,
                ..QUIET
            }]
        ));
        // Classification is not admission: impossible shapes must still be
        // rejected by save/restore validation, never silently treated as quiet.
        assert!(population_requires_reconciliation(
            1,
            [CheckpointShape {
                retained_credit: true,
                ..UNSEALED
            }]
        ));
    }
}
