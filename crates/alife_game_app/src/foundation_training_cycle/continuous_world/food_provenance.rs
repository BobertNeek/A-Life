//! Passive, session-local acquisition/meal associations; never causal attribution.
use crate::foundation_training::food_outcome::{
    FoodAcquisitionKind, FoodOutcomeEvents, FoodOutcomeObservation,
};

#[derive(Default)]
pub(super) struct FoodMealProvenance {
    last_reposition: Option<u64>,
    last_cue_emission: Option<u64>,
    last_outcome: Option<u64>,
    acquisition: Option<(FoodAcquisitionKind, u64, bool)>,
}

#[derive(Clone, Copy)]
pub(super) struct MealLink {
    acquisition: Option<(FoodAcquisitionKind, u64, bool)>,
    outcome_tick: u64,
    cue_emission: Option<u64>,
    food_token_heard: Option<u64>,
    target_reposition: Option<u64>,
}

impl FoodMealProvenance {
    pub fn repositioned(&mut self, tick: u64) {
        self.last_reposition = Some(tick);
    }

    pub fn cue_emitted(&mut self, tick: u64) {
        self.last_cue_emission = Some(tick);
    }

    pub fn observe(
        &mut self,
        observation: FoodOutcomeObservation,
        events: FoodOutcomeEvents,
        last_food_token_heard: Option<u64>,
    ) -> Option<MealLink> {
        // Do not link through a release, missing ownership or unobserved gap.
        // PPO window boundaries themselves do not reset any history.
        if observation.held_before() != Some(true)
            || self.last_outcome != Some(observation.decision_tick.raw())
        {
            self.acquisition = None;
        }
        let assisted = self.last_reposition.is_some()
            || self.last_cue_emission.is_some()
            || last_food_token_heard.is_some();
        if let Some(kind) = events.acquisition {
            self.acquisition = Some((kind, observation.outcome_tick.raw(), assisted));
        }
        let meal = events.nourished.then_some(MealLink {
            acquisition: self.acquisition,
            outcome_tick: observation.outcome_tick.raw(),
            cue_emission: self.last_cue_emission,
            food_token_heard: last_food_token_heard,
            target_reposition: self.last_reposition,
        });
        if observation
            .after
            .is_none_or(|food| food.consumed || food.owner != Some(observation.organism))
        {
            self.acquisition = None;
        }
        self.last_outcome = Some(observation.outcome_tick.raw());
        // Placement and cue history deliberately survive consumption/regrowth:
        // the same entity can regrow at its earlier assisted position.
        meal
    }
}

#[derive(Default, serde::Serialize)]
struct TickRange {
    count: u64,
    min: Option<u64>,
    max: Option<u64>,
}

impl TickRange {
    fn observe(&mut self, ticks: u64) {
        self.count += 1;
        self.min = Some(self.min.map_or(ticks, |old| old.min(ticks)));
        self.max = Some(self.max.map_or(ticks, |old| old.max(ticks)));
    }
}

#[derive(Default, serde::Serialize)]
struct MealAssociations {
    meals: u64,
    with_prior_session_cue_emission: u64,
    with_prior_session_food_token_heard: u64,
    with_prior_session_target_reposition: u64,
    with_no_observed_session_assistance: u64,
    with_observed_assistance_before_acquisition: u64,
    acquisition_to_meal_ticks: TickRange,
    cue_emission_to_meal_ticks: TickRange,
    food_token_heard_to_meal_ticks: TickRange,
    target_reposition_to_meal_ticks: TickRange,
}

/// Three fixed categories, not a per-meal archive. Counts can overlap within a
/// category; no absence of observed session assistance proves independence.
#[derive(Default, serde::Serialize)]
pub(super) struct MealProvenanceSummary {
    eat_acquired: MealAssociations,
    grab_acquired: MealAssociations,
    acquisition_unobserved: MealAssociations,
}

impl MealProvenanceSummary {
    pub fn observe(&mut self, meal: MealLink) {
        let counts = match meal.acquisition.map(|(kind, _, _)| kind) {
            Some(FoodAcquisitionKind::Eat) => &mut self.eat_acquired,
            Some(FoodAcquisitionKind::Grab) => &mut self.grab_acquired,
            None => &mut self.acquisition_unobserved,
        };
        counts.meals += 1;
        if let Some((_, tick, assisted)) = meal.acquisition {
            counts
                .acquisition_to_meal_ticks
                .observe(meal.outcome_tick - tick);
            counts.with_observed_assistance_before_acquisition += u64::from(assisted);
        }
        for (tick, count, range) in [
            (
                meal.cue_emission,
                &mut counts.with_prior_session_cue_emission,
                &mut counts.cue_emission_to_meal_ticks,
            ),
            (
                meal.food_token_heard,
                &mut counts.with_prior_session_food_token_heard,
                &mut counts.food_token_heard_to_meal_ticks,
            ),
            (
                meal.target_reposition,
                &mut counts.with_prior_session_target_reposition,
                &mut counts.target_reposition_to_meal_ticks,
            ),
        ] {
            if let Some(tick) = tick {
                *count += 1;
                range.observe(meal.outcome_tick - tick);
            }
        }
        if meal.cue_emission.is_none()
            && meal.food_token_heard.is_none()
            && meal.target_reposition.is_none()
        {
            counts.with_no_observed_session_assistance += 1;
        }
    }

    pub fn rows(rows: &[(usize, MealLink)], trained_rows: usize) -> Self {
        let mut summary = Self::default();
        for (_, meal) in rows.iter().filter(|(row, _)| *row < trained_rows) {
            summary.observe(*meal);
        }
        summary
    }
}

pub(super) const SCOPE: &str = "Exact-target nourishing meals only; last observed acquisition linked through contiguous captured ownership, including Eat pickup. Cue emission is not hearing; addressed teacher food/eat tokens are not target-grounded. Prior session assistance is temporal association, not causal attribution. History survives PPO windows and same-entity regrowth, but starts unknown before this collector session/restart; no-observed assistance is not independence. Tick ranges end at meal outcome; categories include captured bootstrap unless explicitly trained.";

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation_training::food_outcome::FoodOwnershipSnapshot;
    use alife_core::{OrganismId, Tick, WorldEntityId};

    fn observation(
        tick: u64,
        held: bool,
        held_after: bool,
        consumed: bool,
    ) -> FoodOutcomeObservation {
        FoodOutcomeObservation {
            organism: OrganismId(1),
            target: WorldEntityId(2),
            decision_tick: Tick(tick),
            outcome_tick: Tick(tick + 1),
            before: Some(FoodOwnershipSnapshot {
                owner: held.then_some(OrganismId(1)),
                consumed: false,
            }),
            after: Some(FoodOwnershipSnapshot {
                owner: held_after.then_some(OrganismId(1)),
                consumed,
            }),
        }
    }

    fn acquire(
        tracker: &mut FoodMealProvenance,
        tick: u64,
        kind: FoodAcquisitionKind,
        heard: Option<u64>,
    ) {
        assert!(tracker
            .observe(
                observation(tick, false, true, false),
                FoodOutcomeEvents {
                    acquisition: Some(kind),
                    nourished: false
                },
                heard
            )
            .is_none());
    }

    fn meal(tracker: &mut FoodMealProvenance, tick: u64, heard: Option<u64>) -> MealLink {
        tracker
            .observe(
                observation(tick, true, false, true),
                FoodOutcomeEvents {
                    acquisition: None,
                    nourished: true,
                },
                heard,
            )
            .unwrap()
    }

    #[test]
    fn prior_assistance_and_eat_pickup_survive_window_boundary_and_bootstrap_filter() {
        let mut tracker = FoodMealProvenance::default();
        tracker.repositioned(10); // includes initial placement
        tracker.cue_emitted(11);
        acquire(&mut tracker, 12, FoodAcquisitionKind::Eat, Some(12));
        // End one PPO window here; only row-summary state is recreated.
        let rows = [(0, meal(&mut tracker, 13, Some(12)))];
        let trained = MealProvenanceSummary::rows(&rows, 1);
        assert_eq!(trained.eat_acquired.meals, 1);
        assert_eq!(
            trained
                .eat_acquired
                .with_observed_assistance_before_acquisition,
            1
        );
        assert_eq!(trained.eat_acquired.with_prior_session_cue_emission, 1);
        assert_eq!(trained.eat_acquired.with_prior_session_food_token_heard, 1);
        assert_eq!(trained.eat_acquired.with_prior_session_target_reposition, 1);
        assert_eq!(trained.eat_acquired.with_no_observed_session_assistance, 0);
        assert_eq!(trained.eat_acquired.acquisition_to_meal_ticks.min, Some(1));
        assert_eq!(trained.eat_acquired.cue_emission_to_meal_ticks.max, Some(3));
        assert_eq!(
            trained.eat_acquired.target_reposition_to_meal_ticks.max,
            Some(4)
        );
        assert_eq!(MealProvenanceSummary::rows(&rows, 0).eat_acquired.meals, 0);
        // No NEW cue/reposition in a later window is not no prior assistance.
        // Ordinary ecology resets consumed state but retains entity position.
        acquire(&mut tracker, 30, FoodAcquisitionKind::Eat, Some(12));
        let rows = [(0, meal(&mut tracker, 31, Some(12)))];
        let later = MealProvenanceSummary::rows(&rows, 1);
        assert_eq!(later.eat_acquired.with_prior_session_target_reposition, 1);
        assert_eq!(
            later.eat_acquired.target_reposition_to_meal_ticks.max,
            Some(22)
        );
    }

    #[test]
    fn release_unobserved_gap_and_session_restart_do_not_invent_acquisition_links() {
        for gap in [false, true] {
            let mut tracker = FoodMealProvenance::default();
            tracker.repositioned(0);
            acquire(&mut tracker, 0, FoodAcquisitionKind::Grab, None);
            let tick = if gap {
                4
            } else {
                tracker.observe(
                    observation(1, true, false, false),
                    FoodOutcomeEvents::default(),
                    None,
                );
                2
            };
            let rows = [(0, meal(&mut tracker, tick, None))];
            let summary = MealProvenanceSummary::rows(&rows, 1);
            assert_eq!(summary.acquisition_unobserved.meals, 1);
            assert_eq!(
                summary
                    .acquisition_unobserved
                    .with_prior_session_target_reposition,
                1
            );
            assert_eq!(summary.grab_acquired.meals, 0);
        }
        let mut restarted = FoodMealProvenance::default();
        let rows = [(0, meal(&mut restarted, 100, None))];
        let summary = MealProvenanceSummary::rows(&rows, 1);
        assert_eq!(summary.acquisition_unobserved.meals, 1);
        assert_eq!(
            summary
                .acquisition_unobserved
                .with_no_observed_session_assistance,
            1
        );
        assert!(SCOPE.contains("unknown before this collector session/restart"));
    }

    #[test]
    fn cue_emission_hearing_and_acquisition_timing_remain_distinct() {
        let mut tracker = FoodMealProvenance::default();
        acquire(&mut tracker, 0, FoodAcquisitionKind::Grab, None);
        tracker.cue_emitted(1); // emitted after acquisition, never observed heard
        let rows = [(0, meal(&mut tracker, 1, None))];
        let summary = MealProvenanceSummary::rows(&rows, 1);
        assert_eq!(summary.grab_acquired.with_prior_session_cue_emission, 1);
        assert_eq!(summary.grab_acquired.with_prior_session_food_token_heard, 0);
        assert_eq!(
            summary
                .grab_acquired
                .with_observed_assistance_before_acquisition,
            0
        );
        assert_eq!(summary.grab_acquired.with_no_observed_session_assistance, 0);
        assert_eq!(summary.eat_acquired.meals, 0);
        // Another food does not inherit this target's placement/cue history.
        let mut other = FoodMealProvenance::default();
        acquire(&mut other, 0, FoodAcquisitionKind::Eat, None);
        let rows = [(0, meal(&mut other, 1, Some(1)))];
        let summary = MealProvenanceSummary::rows(&rows, 1);
        assert_eq!(summary.eat_acquired.with_prior_session_cue_emission, 0);
        assert_eq!(summary.eat_acquired.with_prior_session_food_token_heard, 1);
        assert_eq!(summary.eat_acquired.with_prior_session_target_reposition, 0);
        assert_eq!(
            summary
                .eat_acquired
                .with_observed_assistance_before_acquisition,
            0
        );
    }

    #[test]
    fn generic_consumption_without_exact_nourishment_cannot_create_meal_link() {
        let mut tracker = FoodMealProvenance::default();
        acquire(&mut tracker, 0, FoodAcquisitionKind::Eat, None);
        assert!(tracker
            .observe(
                observation(1, true, false, true),
                FoodOutcomeEvents::default(),
                None
            )
            .is_none());
        // The consumed possession cannot be reused as a later acquisition.
        let rows = [(0, meal(&mut tracker, 2, None))];
        let summary = MealProvenanceSummary::rows(&rows, 1);
        assert_eq!(summary.acquisition_unobserved.meals, 1);
        assert_eq!(summary.eat_acquired.meals, 0);
    }

    #[test]
    fn trained_meal_prefix_excludes_bootstrap_without_claiming_acquisition_training() {
        let mut eat = FoodMealProvenance::default();
        let mut grab = FoodMealProvenance::default();
        acquire(&mut eat, 0, FoodAcquisitionKind::Eat, None);
        acquire(&mut grab, 0, FoodAcquisitionKind::Grab, None);
        let rows = [(0, meal(&mut eat, 1, None)), (1, meal(&mut grab, 1, None))];
        let captured = MealProvenanceSummary::rows(&rows, 2);
        let trained = MealProvenanceSummary::rows(&rows, 1);
        assert_eq!(captured.eat_acquired.meals, 1);
        assert_eq!(captured.grab_acquired.meals, 1);
        assert_eq!(trained.eat_acquired.meals, 1);
        assert_eq!(trained.grab_acquired.meals, 0);
        assert_eq!(trained.eat_acquired.with_no_observed_session_assistance, 1);
        assert_eq!(
            trained
                .eat_acquired
                .with_observed_assistance_before_acquisition,
            0
        );
        assert_eq!(trained.eat_acquired.acquisition_to_meal_ticks.max, Some(1));
    }
}
