//! Per-interval actuator arithmetic for the explicit awake candidate.
//! Distance is not elapsed time: a slower full-interval command still consumes
//! the whole actuator interval. Blocked effort can consume time without motion.
use alife_core::ScaffoldContractError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct TranslationIntervalBudget {
    full_speed_distance: f32,
    pub(super) spent_time: f32,
    pub(super) traveled_path: f32,
}

impl TranslationIntervalBudget {
    pub(super) fn new(full_speed_distance: f32) -> Result<Self, ScaffoldContractError> {
        if !full_speed_distance.is_finite() || full_speed_distance < 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self {
            full_speed_distance,
            spent_time: 0.0,
            traveled_path: 0.0,
        })
    }

    /// Record one staged, physically executed segment/attempt. `speed_distance`
    /// is that route's distance at its actual bounded speed for one full tick.
    /// `duration` is its allocated fraction of the same tick, not a second tick.
    pub(super) fn consume(
        &mut self,
        speed_distance: f32,
        duration: f32,
        actual_path: f32,
    ) -> Result<(), ScaffoldContractError> {
        if !speed_distance.is_finite()
            || !duration.is_finite()
            || !actual_path.is_finite()
            || speed_distance < 0.0
            || speed_distance > self.full_speed_distance
            || !(0.0..=1.0).contains(&duration)
            || actual_path < 0.0
            || duration > 1.0 - self.spent_time
            || actual_path > speed_distance * duration + 1e-6 * self.full_speed_distance.max(1.0)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.spent_time += duration;
        self.traveled_path += actual_path;
        Ok(())
    }
}

#[test]
fn slow_full_interval_cannot_be_followed_by_another_full_reflex() {
    let mut budget = TranslationIntervalBudget::new(0.1).unwrap();
    budget.consume(0.05, 1.0, 0.05).unwrap();
    let before = budget;
    assert!(budget.consume(0.05, 1.0, 0.05).is_err());
    assert_eq!(budget, before);
}

#[test]
fn early_contact_allocates_only_remaining_time_at_reflex_speed() {
    let mut budget = TranslationIntervalBudget::new(0.1).unwrap();
    budget.consume(0.05, 0.25, 0.0125).unwrap();
    budget.consume(0.1, 0.75, 0.075).unwrap();
    assert_eq!(budget.spent_time, 1.0);
    assert!((budget.traveled_path - 0.0875).abs() < 1e-7);
}

#[test]
fn initial_reflex_priority_consumes_interval_before_voluntary_translation() {
    let mut budget = TranslationIntervalBudget::new(0.1).unwrap();
    budget.consume(0.1, 1.0, 0.1).unwrap();
    assert!(budget.consume(0.1, 0.1, 0.01).is_err());
    budget.consume(0.1, 0.0, 0.0).unwrap();
}

#[test]
fn opposite_segments_count_path_not_zero_net_displacement() {
    let mut budget = TranslationIntervalBudget::new(0.1).unwrap();
    budget.consume(0.1, 0.5, 0.05).unwrap();
    budget.consume(0.1, 0.5, 0.05).unwrap();
    assert_eq!(budget.traveled_path, 0.1);
    assert_eq!(budget.spent_time, 1.0);
}

#[test]
fn blocked_attempt_still_uses_its_allocated_time() {
    let mut budget = TranslationIntervalBudget::new(0.1).unwrap();
    budget.consume(0.1, 0.5, 0.0).unwrap();
    assert!(budget.consume(0.1, 0.75, 0.075).is_err());
    budget.consume(0.1, 0.5, 0.05).unwrap();
    assert_eq!(budget.traveled_path, 0.05);
}

#[test]
fn malformed_and_over_budget_segments_are_atomic_rejections() {
    let mut budget = TranslationIntervalBudget::new(0.1).unwrap();
    for (speed, time, path) in [
        (f32::NAN, 1.0, 0.0),
        (0.1, f32::NAN, 0.0),
        (0.1, 1.0, f32::NAN),
        (0.2, 1.0, 0.1),
        (0.1, -0.1, 0.0),
        (0.1, 0.5, 0.06),
        (0.1, 1.1, 0.1),
        (0.1, 0.0, 0.01),
    ] {
        let before = budget;
        assert!(budget.consume(speed, time, path).is_err());
        assert_eq!(budget, before);
    }
    assert!(TranslationIntervalBudget::new(f32::INFINITY).is_err());
    assert!(TranslationIntervalBudget::new(-0.1).is_err());
}
