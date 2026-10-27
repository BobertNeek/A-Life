//! Device-free invariants shared by typed Current admission and recovery.
pub(super) fn validate_session_counts(
    start: u64,
    tick: u64,
    rows: u64,
    updates: u64,
    limit: u64,
) -> Result<(), &'static str> {
    if tick.checked_sub(start).is_none_or(|elapsed| rows > elapsed)
        || rows > limit
        || updates > rows
        || (updates == 0) != (rows == 0)
    {
        return Err(
            "same-individual retained rows/updates exceed actual world progress or request",
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_rows_cannot_outnumber_actual_ticks() {
        assert!(validate_session_counts(100, 102, 3, 1, 10).is_err());
        assert!(validate_session_counts(100, 102, 2, 1, 10).is_ok());
    }
    #[test]
    fn unchanged_private_adoption_has_zero_training_not_invented_steps() {
        assert!(validate_session_counts(100, 100, 0, 0, 10).is_ok());
        assert!(validate_session_counts(100, 101, 0, 1, 10).is_err());
        assert!(validate_session_counts(100, 101, 1, 0, 10).is_err());
    }
    #[test]
    fn backwards_world_and_request_overflow_are_rejected() {
        assert!(validate_session_counts(100, 99, 0, 0, 10).is_err());
        assert!(validate_session_counts(0, 100, 11, 2, 10).is_err());
        assert!(validate_session_counts(0, 100, 1, 2, 10).is_err());
    }
}
