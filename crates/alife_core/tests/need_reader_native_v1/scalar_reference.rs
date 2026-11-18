//! Frozen scalar oracle from need-specific-reader design V1; test-only here.
//!
//! This source is deliberately incapable of admitting real memory evidence. It
//! has no store adapter, constructor for validated source binding, serialization,
//! action selector, optimizer, checkpoint access, or GPU interface. Its synthetic
//! inputs must never be described as sealed measured observations. This copy is
//! compiled only by the detached native integration test, with visibility changes.
//!
//! Design: four paired, shared monotone tanh(ReLU) branches per biological lane.
//! Current readings and historical deltas remain separate. The local successor
//! is arithmetic, not a forecast. Every returned result is diagnostic only.

const N: usize = 11;
const K: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Role {
    Burden,
    Availability { target: f32 },
    Informational,
    // Fixture meaning only. Production requires a validated chemistry binding.
    TypedBoredomFixture,
}

impl Role {
    fn pressure(self, raw: f32) -> f32 {
        match self {
            Self::Burden | Self::TypedBoredomFixture => raw,
            Self::Availability { target } => (target - raw).max(0.0),
            Self::Informational => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Recipe {
    pub(super) roles: [Role; N],
    pub(super) knots: [f32; K],
    pub(super) basis: [f32; K],
    pub(super) magnitudes: [f32; N],
    pub(super) gain: f32,
    pub(super) ceiling: f32,
}

impl Recipe {
    pub(super) fn reference() -> Self {
        use Role::*;
        Self {
            roles: [
                Burden,
                Burden,
                Burden,
                Burden,
                Burden,
                Informational,
                Availability { target: 0.75 },
                Burden,
                Informational,
                Informational,
                Informational,
            ],
            knots: [0.0, 0.25, 0.5, 0.75],
            basis: [0.125, 0.25, 0.25, 0.25],
            magnitudes: [
                1.0 / 7.0,
                1.0 / 7.0,
                1.0 / 7.0,
                1.0 / 7.0,
                1.0 / 7.0,
                0.0,
                1.0 / 7.0,
                1.0 / 7.0,
                0.0,
                0.0,
                0.0,
            ],
            gain: 0.35,
            ceiling: 0.35,
        }
    }

    pub(super) fn validate(self) -> Result<(), ModelError> {
        if !unit(self.gain)
            || self.gain > 0.35
            || !unit(self.ceiling)
            || self.ceiling > 0.35
            || self.knots.iter().any(|&v| !unit(v))
            || self.knots.windows(2).any(|p| p[0] >= p[1])
            || self.basis.iter().any(|&v| !unit(v))
            || self.basis.iter().sum::<f32>() > 1.0
            || self.magnitudes.iter().any(|&v| !unit(v))
            || self.magnitudes.iter().sum::<f32>() > 2.0
        {
            return Err(ModelError::InvalidRecipe);
        }
        for (role, magnitude) in self.roles.into_iter().zip(self.magnitudes) {
            match role {
                Role::Availability { target } if !unit(target) || target <= 0.0 => {
                    return Err(ModelError::InvalidRecipe);
                }
                Role::Informational if magnitude != 0.0 => {
                    return Err(ModelError::InvalidRecipe);
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn burden(self, pressure: f32) -> f32 {
        self.knots
            .into_iter()
            .zip(self.basis)
            .map(|(threshold, weight)| weight * (pressure - threshold).max(0.0).tanh())
            .sum()
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum SyntheticEvidence {
    Unknown,
    MeasuredFixture {
        delta: [f32; N],
        confidence: f32,
        age_ticks: u64,
        similarity: Option<f32>,
        // No authority implied. This only tests arithmetic suppression.
        arithmetic_supported: bool,
    },
}

#[derive(Debug, PartialEq)]
pub(super) enum ModelError {
    InvalidRecipe,
    InvalidInput,
}

#[derive(Debug)]
pub(super) struct Features {
    pub(super) known: bool,
    pub(super) arithmetic_supported: bool,
    pub(super) current: [f32; N],
    pub(super) raw_delta: [f32; N],
    pub(super) local_successor: [f32; N],
    pub(super) signed_association: [f32; N],
    pub(super) confidence: f32,
    pub(super) age_ticks: Option<u64>,
    pub(super) similarity: Option<f32>,
    pub(super) readout_before_confidence: f32,
    pub(super) readout_after_confidence: f32,
    pub(super) readout_clipped: bool,
    // Always false: no result from this arithmetic model is runtime evidence.
    pub(super) admissible_for_exposure: bool,
}

fn unit(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

pub(super) fn evaluate(
    recipe: Recipe,
    current: [f32; N],
    evidence: SyntheticEvidence,
) -> Result<Features, ModelError> {
    recipe.validate()?;
    if current.iter().any(|&x| !unit(x)) {
        return Err(ModelError::InvalidInput);
    }
    let (known, raw_delta, confidence, age_ticks, similarity, supported) = match evidence {
        SyntheticEvidence::Unknown => (false, [0.0; N], 0.0, None, None, false),
        SyntheticEvidence::MeasuredFixture {
            delta,
            confidence,
            age_ticks,
            similarity,
            arithmetic_supported,
        } => {
            if delta
                .iter()
                .any(|x| !x.is_finite() || !(-1.0..=1.0).contains(x))
                || !unit(confidence)
                || similarity.is_some_and(|v| !unit(v))
            {
                return Err(ModelError::InvalidInput);
            }
            (
                true,
                delta,
                confidence,
                Some(age_ticks),
                similarity,
                arithmetic_supported,
            )
        }
    };
    let mut local_successor = [0.0; N];
    let mut association = [0.0; N];
    // Work is identical for all gain and status factors. Clamping here is a
    // hypothetical biological bound, not a way to admit invalid source values.
    for lane in 0..N {
        local_successor[lane] = (current[lane] + raw_delta[lane]).clamp(0.0, 1.0);
        let role = recipe.roles[lane];
        association[lane] = recipe.burden(role.pressure(current[lane]))
            - recipe.burden(role.pressure(local_successor[lane]));
    }
    let raw = recipe.gain
        * association
            .iter()
            .zip(recipe.magnitudes)
            .map(|(value, magnitude)| value * magnitude)
            .sum::<f32>();
    let arithmetic_mask = f32::from(u8::from(known && supported));
    let bounded = raw.clamp(-recipe.ceiling, recipe.ceiling);
    Ok(Features {
        known,
        arithmetic_supported: supported,
        current,
        raw_delta,
        local_successor,
        signed_association: association,
        confidence,
        age_ticks,
        similarity,
        readout_before_confidence: bounded * arithmetic_mask,
        readout_after_confidence: bounded * confidence * arithmetic_mask,
        readout_clipped: bounded != raw,
        admissible_for_exposure: false,
    })
}

#[cfg(test)]
mod synthetic_tests {
    use super::*;

    fn fixture(delta: [f32; N], confidence: f32) -> SyntheticEvidence {
        SyntheticEvidence::MeasuredFixture {
            delta,
            confidence,
            age_ticks: 1,
            similarity: Some(0.99988),
            arithmetic_supported: true,
        }
    }

    fn state() -> [f32; N] {
        [0.25, 0.2, 0.1, 0.0, 0.15, 0.45, 0.75, 0.0, 0.1, 0.0, 0.0]
    }

    #[test]
    fn zero_pressure_cannot_receive_historical_relief() {
        let recipe = Recipe::reference();
        let mut delta = [0.0; N];
        delta[4] = -0.007499993;
        let at_baseline = evaluate(recipe, state(), fixture(delta, 1.0)).unwrap();
        let mut none = state();
        none[4] = 0.0;
        let at_zero = evaluate(recipe, none, fixture(delta, 1.0)).unwrap();
        assert!(at_baseline.readout_after_confidence > 0.0);
        assert_eq!(at_zero.signed_association[4], 0.0);
        assert_eq!(at_zero.readout_after_confidence, 0.0);
        assert_eq!(at_zero.raw_delta[4], delta[4]);
        assert!(!at_zero.admissible_for_exposure);
        none[4] = 0.001;
        let at_low = evaluate(recipe, none, fixture(delta, 1.0)).unwrap();
        assert!(at_low.signed_association[4] <= recipe.burden(0.001));
        assert!(at_low.readout_after_confidence < at_baseline.readout_after_confidence);
    }

    #[test]
    fn positive_burden_change_is_adverse_even_at_zero() {
        for lane in [0, 1, 2, 3, 4, 7] {
            let mut delta = [0.0; N];
            delta[lane] = 0.05;
            let result = evaluate(Recipe::reference(), [0.0; N], fixture(delta, 1.0)).unwrap();
            assert!(result.signed_association[lane] < 0.0);
            assert!(result.readout_after_confidence < 0.0);
        }
    }

    #[test]
    fn every_burden_relief_has_a_positive_sign_and_zero_has_none() {
        for lane in [0, 1, 2, 3, 4, 7] {
            let mut current = state();
            current[lane] = 0.5;
            let mut delta = [0.0; N];
            delta[lane] = -0.05;
            let result = evaluate(Recipe::reference(), current, fixture(delta, 1.0)).unwrap();
            assert!(result.signed_association[lane] > 0.0);
            assert!(result.readout_after_confidence > 0.0);
        }
        let zero = evaluate(Recipe::reference(), state(), fixture([0.0; N], 1.0)).unwrap();
        assert!(zero.known);
        assert_eq!(zero.signed_association, [0.0; N]);
        assert_eq!(zero.readout_after_confidence, 0.0);
    }

    #[test]
    fn atp_is_availability_with_an_explicit_target() {
        let mut delta = [0.0; N];
        delta[6] = 0.05;
        let mut current = state();
        current[6] = 0.4;
        let low = evaluate(Recipe::reference(), current, fixture(delta, 1.0)).unwrap();
        assert!(low.signed_association[6] > 0.0);
        for available in [0.75, 0.9, 1.0] {
            current[6] = available;
            let enough = evaluate(Recipe::reference(), current, fixture(delta, 1.0)).unwrap();
            assert_eq!(enough.signed_association[6], 0.0);
        }
        current[6] = 0.76;
        delta[6] = -0.05;
        let loss = evaluate(Recipe::reference(), current, fixture(delta, 1.0)).unwrap();
        assert!(loss.signed_association[6] < 0.0);
    }

    #[test]
    fn informational_lanes_are_retained_without_invented_valence() {
        for lane in [5, 8, 9, 10] {
            let mut delta = [0.0; N];
            delta[lane] = 0.05;
            let result = evaluate(Recipe::reference(), state(), fixture(delta, 1.0)).unwrap();
            assert_eq!(result.raw_delta[lane], 0.05);
            assert_eq!(result.signed_association[lane], 0.0);
            assert_eq!(result.readout_after_confidence, 0.0);
        }
        let mut recipe = Recipe::reference();
        recipe.roles[9] = Role::TypedBoredomFixture;
        recipe.magnitudes[9] = 0.08;
        let mut current = state();
        current[9] = 0.2;
        let mut delta = [0.0; N];
        delta[9] = -0.05;
        assert!(
            evaluate(recipe, current, fixture(delta, 1.0))
                .unwrap()
                .readout_after_confidence
                > 0.0
        );
    }

    #[test]
    fn unknown_zero_zero_confidence_and_unsupported_stay_distinct() {
        let recipe = Recipe::reference();
        let unknown = evaluate(recipe, state(), SyntheticEvidence::Unknown).unwrap();
        let zero = evaluate(recipe, state(), fixture([0.0; N], 1.0)).unwrap();
        assert!(!unknown.known);
        assert!(zero.known);
        assert_eq!(
            unknown.readout_after_confidence,
            zero.readout_after_confidence
        );
        let mut delta = [0.0; N];
        delta[4] = -0.05;
        let no_confidence = evaluate(recipe, state(), fixture(delta, 0.0)).unwrap();
        assert!(no_confidence.known);
        assert!(no_confidence.readout_before_confidence > 0.0);
        assert_eq!(no_confidence.readout_after_confidence, 0.0);
        let unsupported = evaluate(
            recipe,
            state(),
            SyntheticEvidence::MeasuredFixture {
                delta,
                confidence: 1.0,
                age_ticks: 1,
                similarity: Some(0.99),
                arithmetic_supported: false,
            },
        )
        .unwrap();
        assert!(unsupported.known);
        assert!(!unsupported.arithmetic_supported);
        assert_eq!(unsupported.raw_delta, delta);
        assert_eq!(unsupported.readout_after_confidence, 0.0);
    }

    #[test]
    fn confidence_is_applied_once_and_similarity_is_not_a_weight() {
        let mut delta = [0.0; N];
        delta[4] = -0.05;
        let full = evaluate(Recipe::reference(), state(), fixture(delta, 1.0)).unwrap();
        let half = evaluate(Recipe::reference(), state(), fixture(delta, 0.5)).unwrap();
        assert_eq!(
            half.readout_after_confidence,
            full.readout_after_confidence * 0.5
        );
        for similarity in [Some(0.0), Some(0.975), Some(1.0), None] {
            let result = evaluate(
                Recipe::reference(),
                state(),
                SyntheticEvidence::MeasuredFixture {
                    delta,
                    confidence: 0.5,
                    age_ticks: 1,
                    similarity,
                    arithmetic_supported: true,
                },
            )
            .unwrap();
            assert_eq!(
                result.readout_after_confidence,
                half.readout_after_confidence
            );
            assert_eq!(result.similarity, similarity);
            assert_eq!(result.confidence, 0.5);
            assert_eq!(result.age_ticks, Some(1));
        }
    }

    #[test]
    fn conflict_can_favor_work_or_synthetic_recovery_without_selecting_actions() {
        let mut work = [0.0; N];
        work[4] = -0.05;
        work[1] = 0.03;
        work[6] = -0.03;
        // Synthetic arithmetic only. V1 store does not admit targetless Rest.
        let mut recovery = [0.0; N];
        recovery[1] = -0.04;
        recovery[6] = 0.04;
        let mut rested_lonely = state();
        rested_lonely[4] = 0.8;
        rested_lonely[1] = 0.01;
        rested_lonely[6] = 0.8;
        let mut exhausted_content = state();
        exhausted_content[4] = 0.01;
        exhausted_content[1] = 0.8;
        exhausted_content[6] = 0.2;
        let score = |current, delta| {
            evaluate(Recipe::reference(), current, fixture(delta, 1.0))
                .unwrap()
                .readout_after_confidence
        };
        assert!(score(rested_lonely, work) > score(rested_lonely, recovery));
        assert!(score(exhausted_content, recovery) > score(exhausted_content, work));
        assert_eq!(score(rested_lonely, [0.0; N]), 0.0);
    }

    #[test]
    fn zero_gain_preserves_features_and_work_shape() {
        let mut delta = [0.0; N];
        delta[4] = -0.05;
        let recipe = Recipe::reference();
        let active = evaluate(recipe, state(), fixture(delta, 1.0)).unwrap();
        let ablated = evaluate(
            Recipe {
                gain: 0.0,
                ..recipe
            },
            state(),
            fixture(delta, 1.0),
        )
        .unwrap();
        assert_eq!(active.current, ablated.current);
        assert_eq!(active.raw_delta, ablated.raw_delta);
        assert_eq!(active.local_successor, ablated.local_successor);
        assert_eq!(active.signed_association, ablated.signed_association);
        assert_eq!(ablated.readout_after_confidence, 0.0);
        assert!(!active.readout_clipped);
    }

    #[test]
    fn malformed_inputs_and_unpaired_recipe_bounds_fail() {
        let mut current = state();
        current[4] = f32::NAN;
        assert!(evaluate(Recipe::reference(), current, SyntheticEvidence::Unknown).is_err());
        let mut delta = [0.0; N];
        delta[4] = f32::INFINITY;
        assert!(evaluate(Recipe::reference(), state(), fixture(delta, 1.0)).is_err());
        let mut recipe = Recipe::reference();
        recipe.basis[0] = -0.1;
        assert!(recipe.validate().is_err());
        recipe = Recipe::reference();
        recipe.magnitudes[5] = 0.1;
        assert!(recipe.validate().is_err());
    }
}
