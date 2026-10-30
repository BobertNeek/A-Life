//! Exact-target measured food evidence for care acceptance, never a reward bonus.
use alife_core::{
    ExperiencePatch, MotorChannel, OrganismId, PhysicalContactKind, Tick, WorldEntityId,
};

/// Observed events within this probe. Care completion uses nourished meals;
/// this record never supplies curriculum reward.
/// A successful explicit-Grab probe can stop before any meal is observed.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FoundationFoodOutcome {
    pub organism: OrganismId,
    pub target: WorldEntityId,
    pub explicit_grab_acquisition_events: usize,
    pub ordinary_acquisition_events: usize,
    pub consumption_events: usize,
    pub nourished_meal_events: usize,
}

/// Passive Feeding cycle evidence. Absence on historical cycles means unknown.
/// Row indices separate actual training meals from the captured bootstrap row.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationFeedingFoodReceipt {
    pub food_outcome: FoundationFoodOutcome,
    pub nourished_meal_rows: Vec<usize>,
    pub captured_rows: usize,
    pub trained_rows: usize,
}

impl FoundationFeedingFoodReceipt {
    pub(crate) fn new(organism: OrganismId, target: WorldEntityId) -> Self {
        Self {
            food_outcome: FoundationFoodOutcome::new(organism, target),
            nourished_meal_rows: Vec::new(),
            captured_rows: 0,
            trained_rows: 0,
        }
    }

    pub(crate) fn before_step(
        &self,
        world: &alife_world::HeadlessWorld,
    ) -> (Tick, Option<FoodOwnershipSnapshot>) {
        (world.tick(), ownership(world, self.food_outcome.target))
    }

    pub(crate) fn observe_step(
        &mut self,
        world: &alife_world::HeadlessWorld,
        before: (Tick, Option<FoodOwnershipSnapshot>),
        step: &crate::FoundationTrainingStep,
    ) -> super::Result<()> {
        let observation = FoodOutcomeObservation {
            organism: self.food_outcome.organism,
            target: self.food_outcome.target,
            decision_tick: before.0,
            outcome_tick: world.tick(),
            before: before.1,
            after: ownership(world, self.food_outcome.target),
        };
        self.observe_row(observation, &step.frame, &step.behavior, &step.patch)
    }

    fn observe_row(
        &mut self,
        observation: FoodOutcomeObservation,
        frame: &alife_core::PerceptionFrame,
        behavior: &alife_gpu_backend::GpuTrainingRolloutReceipt,
        patch: &ExperiencePatch,
    ) -> super::Result<()> {
        if !observation.matches_row(frame, behavior, patch) {
            return Err("feeding food capture identity mismatch".into());
        }
        let meals_before = self.food_outcome.nourished_meal_events;
        self.food_outcome.observe(observation, patch);
        if self.food_outcome.nourished_meal_events > meals_before {
            self.nourished_meal_rows.push(self.captured_rows);
        }
        self.captured_rows += 1;
        Ok(())
    }

    pub(crate) fn finish(mut self, trained_rows: usize) -> super::Result<Self> {
        crate::foundation_training_cycle::cycle_replay_dose(
            self.captured_rows,
            trained_rows,
            &self.nourished_meal_rows,
        )?;
        self.trained_rows = trained_rows;
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FoodOwnershipSnapshot {
    pub owner: Option<OrganismId>,
    pub consumed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FoodAcquisitionKind {
    Eat,
    Grab,
}

/// Derived from the same exact-target predicates as the public counters.
/// This is passive telemetry, not a new success or learning signal.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct FoodOutcomeEvents {
    pub acquisition: Option<FoodAcquisitionKind>,
    pub nourished: bool,
}

pub(crate) fn ownership(
    world: &alife_world::HeadlessWorld,
    target: WorldEntityId,
) -> Option<FoodOwnershipSnapshot> {
    world.entity(target).map(|food| FoodOwnershipSnapshot {
        owner: food.carried_by,
        consumed: food.consumed,
    })
}

/// Tiny CPU-map copies bracketing the actual tick, retained only for captured rows.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FoodOutcomeObservation {
    pub organism: OrganismId,
    pub target: WorldEntityId,
    pub decision_tick: Tick,
    pub outcome_tick: Tick,
    pub before: Option<FoodOwnershipSnapshot>,
    pub after: Option<FoodOwnershipSnapshot>,
}

impl FoodOutcomeObservation {
    pub fn held_before(self) -> Option<bool> {
        self.before
            .map(|food| food.owner == Some(self.organism) && !food.consumed)
    }

    pub(super) fn matches_patch(self, patch: &ExperiencePatch) -> bool {
        let frame = patch.pre_action().perception();
        let decision = patch.decision();
        let outcome = patch.outcome();
        patch.header().phase == alife_core::ExperiencePatchPhase::Sealed
            && patch.header().organism_id == self.organism
            && frame.organism_id() == self.organism
            && decision.organism_id == self.organism
            && decision.selected_action.organism_id == self.organism
            && outcome.organism_id == self.organism
            && frame.tick() == self.decision_tick
            && patch.header().world_tick == self.decision_tick
            && decision.decision_tick == self.decision_tick
            && outcome.outcome_tick == self.outcome_tick
            && self.outcome_tick.raw() == self.decision_tick.raw().saturating_add(1)
            && patch.header().sequence_id == decision.sequence_id
            && decision.sequence_id == outcome.sequence_id
            && patch.selected_bundle().is_none_or(|bundle| {
                bundle.organism_id == self.organism
                    && bundle.sequence_id == decision.sequence_id
                    && bundle.tick == self.decision_tick
            })
    }

    pub fn matches_step(self, step: &crate::FoundationTrainingStep) -> bool {
        self.matches_row(&step.frame, &step.behavior, &step.patch)
    }

    pub(crate) fn matches_row(
        self,
        frame: &alife_core::PerceptionFrame,
        behavior: &alife_gpu_backend::GpuTrainingRolloutReceipt,
        patch: &ExperiencePatch,
    ) -> bool {
        self.matches_patch(patch)
            && frame.organism_id() == self.organism
            && frame.tick() == self.decision_tick
            && behavior.organism_id == self.organism.raw()
            && behavior.tick == self.decision_tick.raw()
            && behavior.frame_digest == frame.frame_digest()
            && patch.pre_action().perception().frame_digest() == frame.frame_digest()
            && patch.decision().neural_evidence().is_ok_and(|evidence| {
                evidence.frame_digest == frame.frame_digest()
                    && evidence.dispatch_generation == behavior.dispatch_generation
            })
    }

    pub fn decorate_trace(self, trace: &mut serde_json::Value) {
        trace["lesson_target"] = serde_json::json!(self.target);
        trace["lesson_target_ownership_before_decision"] = serde_json::json!(self.before);
        trace["lesson_target_ownership_after_outcome"] = serde_json::json!(self.after);
    }
}

impl FoundationFoodOutcome {
    pub(crate) fn new(organism: OrganismId, target: WorldEntityId) -> Self {
        Self {
            organism,
            target,
            explicit_grab_acquisition_events: 0,
            ordinary_acquisition_events: 0,
            consumption_events: 0,
            nourished_meal_events: 0,
        }
    }

    // The collector additionally checks the captured GPU row identity. CPU
    // regressions call this with honestly sealed CPU patches, never fake GPU steps.
    pub(crate) fn observe(
        &mut self,
        observation: FoodOutcomeObservation,
        patch: &ExperiencePatch,
    ) -> FoodOutcomeEvents {
        if self.organism != observation.organism
            || self.target != observation.target
            || !observation.matches_patch(patch)
        {
            return FoodOutcomeEvents::default();
        }
        self.observe_selected(observation, patch.decision(), patch.outcome())
    }

    fn observe_selected(
        &mut self,
        observation: FoodOutcomeObservation,
        decision: &alife_core::DecisionSnapshot,
        outcome: &alife_core::PostActionOutcome,
    ) -> FoodOutcomeEvents {
        let mut events = FoodOutcomeEvents::default();
        let touch = |physical: &alife_core::PhysicalActionOutcome| {
            physical.target_entity == Some(self.target)
                && physical.contact == PhysicalContactKind::Touch
        };
        let selected_acquisition = if let Some(bundle) = decision.selected_bundle.as_ref() {
            bundle
                .channels
                .iter()
                .find(|command| {
                    command.channel == MotorChannel::Manipulation
                        && matches!(
                            command.primitive,
                            alife_world::HeadlessActionIds::EAT
                                | alife_world::HeadlessActionIds::GRAB
                        )
                        && command.target.and_then(|target| target.entity) == Some(self.target)
                        && outcome.joint.as_ref().is_some_and(|joint| {
                            joint.channel_outcomes.iter().any(|channel| {
                                channel.channel == command.channel && touch(&channel.physical)
                            })
                        })
                })
                .map(|command| command.primitive)
        } else {
            (matches!(
                decision.selected_action.action_id,
                alife_world::HeadlessActionIds::EAT | alife_world::HeadlessActionIds::GRAB
            ) && decision.selected_action.target_entity == Some(self.target)
                && touch(&outcome.physical))
            .then_some(decision.selected_action.action_id)
        };
        if observation.before
            == Some(FoodOwnershipSnapshot {
                owner: None,
                consumed: false,
            })
            && observation.after
                == Some(FoodOwnershipSnapshot {
                    owner: Some(self.organism),
                    consumed: false,
                })
            && selected_acquisition.is_some()
        {
            self.ordinary_acquisition_events += 1;
            events.acquisition = Some(
                if selected_acquisition == Some(alife_world::HeadlessActionIds::EAT) {
                    FoodAcquisitionKind::Eat
                } else {
                    FoodAcquisitionKind::Grab
                },
            );
        }
        if observation.after.is_some_and(|food| food.consumed)
            && super::selected_eat_consumed(
                &decision.selected_action,
                decision.selected_bundle.as_ref(),
                outcome,
                self.target,
            )
        {
            self.consumption_events += 1;
            if super::held_food_action_consumed(
                &decision.selected_action,
                decision.selected_bundle.as_ref(),
                outcome,
                self.target,
                observation.held_before() == Some(true),
            ) {
                self.nourished_meal_events += 1;
                events.nourished = true;
            }
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{
        ActionKind, ActionProposal, ActionTarget, BrainScaleTier, BrainTickInput, Confidence,
        CreatureMind, MotorCommandBundle, NormalizedScalar, Validate, Vec3f,
    };
    use alife_world::{HeadlessActionIds, HeadlessBrainHarness, HeadlessScenarioBuilder};

    const LEARNER: OrganismId = OrganismId(17);

    fn fixture() -> (HeadlessBrainHarness, CreatureMind, WorldEntityId) {
        let world = HeadlessScenarioBuilder::new(20_261_011)
            .agent("learner", LEARNER, Vec3f::ZERO)
            .food("berry", Vec3f::new(0.0, 0.0, 0.6), 0.6)
            .food("other", Vec3f::new(0.6, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        let target = world.entity_id("berry").unwrap();
        let mind =
            CreatureMind::scaffold(LEARNER, BrainScaleTier::Nano512, 99, Tick::ZERO).unwrap();
        (HeadlessBrainHarness::new(world), mind, target)
    }

    fn act(
        harness: &mut HeadlessBrainHarness,
        mind: &mut CreatureMind,
        target: WorldEntityId,
        primitive: alife_core::ActionId,
    ) -> (FoodOutcomeObservation, ExperiencePatch) {
        let decision_tick = harness.world().tick();
        let before = ownership(&harness.world(), target);
        let proposal = ActionProposal::new(
            primitive,
            if primitive == HeadlessActionIds::GRAB {
                ActionKind::Hold
            } else {
                ActionKind::Interact
            },
            0.9,
            Confidence::new(0.9).unwrap(),
            None,
            0b11,
            ActionTarget::new(Some(target), None),
            NormalizedScalar::new(0.8).unwrap(),
        )
        .unwrap();
        let result = harness.tick_mind(mind, BrainTickInput::new(decision_tick, vec![proposal]));
        let patch = result.brain.experience_patch.unwrap();
        patch.validate_contract().unwrap();
        // These are real CPU-scaffold sealed world receipts, not sampled GPU evidence.
        assert_eq!(
            patch.decision().evidence_kind(),
            alife_core::EvidenceKind::HeuristicBaseline
        );
        assert!(
            patch.outcome().measured_physiology.is_none(),
            "the legacy scaffold fixture has no measured physiology"
        );
        let observation = FoodOutcomeObservation {
            organism: LEARNER,
            target,
            decision_tick,
            outcome_tick: harness.world().tick(),
            before,
            after: ownership(&harness.world(), target),
        };
        (observation, patch)
    }

    fn measured_world_fixture() -> (alife_world::HeadlessWorld, WorldEntityId) {
        let mut world = super::super::held_food_lesson_tests::unconfigured(42);
        let scenario = super::super::configure_foundation_scenario(
            &mut world,
            42,
            Some(super::super::FoundationTeacherLesson::GrabFood),
            None,
            false,
        )
        .unwrap();
        assert!(scenario.grab_food_setup.is_some());
        assert!(scenario.held_food_setup.is_none());
        (world, scenario.food)
    }

    fn measured_eat(
        world: &mut alife_world::HeadlessWorld,
        target: WorldEntityId,
    ) -> (FoodOutcomeObservation, ExperiencePatch) {
        measured_action(world, target, HeadlessActionIds::EAT)
    }

    fn measured_action(
        world: &mut alife_world::HeadlessWorld,
        target: WorldEntityId,
        action: alife_core::ActionId,
    ) -> (FoodOutcomeObservation, ExperiencePatch) {
        let frame = super::super::held_food_lesson_tests::frame(world);
        let organism = frame.organism_id();
        let decision_tick = world.tick();
        let before = ownership(world, target);
        let command = frame
            .candidates()
            .iter()
            .find(|candidate| {
                candidate.action_id == action && candidate.target.entity == Some(target)
            })
            .unwrap()
            .to_command(organism, Confidence::new(1.0).unwrap())
            .unwrap();
        let body = world.organism_entity_ids()[0].1;
        let execution = world
            .apply_registered_command(&command, body, Tick(decision_tick.raw() + 1))
            .unwrap();
        world.try_advance_tick().unwrap();
        let outcome = alife_core::PostActionOutcome::new(
            organism,
            alife_core::ExperienceSequenceId(1),
            world.tick(),
            execution.action_result.execution.succeeded,
            execution.action_result.execution.physical,
            alife_core::HomeostaticDelta::zero(),
            alife_core::SignedValence::ZERO,
            NormalizedScalar::ZERO,
            NormalizedScalar::ZERO,
            alife_core::SignedValence::ZERO,
            NormalizedScalar::ZERO,
        )
        .unwrap()
        .with_measured_physiology(
            alife_core::MeasuredPhysiologyTransition::new(
                execution.biology_before,
                execution.biology_after,
            )
            .unwrap(),
        )
        .unwrap();
        // Reuse the established synthetic selection scaffolding over actual
        // canonical-world action/biology receipts. No GPU choice is claimed.
        let patch = super::super::held_food_lesson_tests::action_patch(
            frame,
            Some(target),
            outcome,
            action,
            None,
        );
        (
            FoodOutcomeObservation {
                organism,
                target,
                decision_tick,
                outcome_tick: world.tick(),
                before,
                after: ownership(world, target),
            },
            patch,
        )
    }

    fn bundle(patch: &ExperiencePatch) -> MotorCommandBundle {
        MotorCommandBundle::new(
            patch.header().organism_id,
            patch.header().sequence_id,
            patch.header().world_tick,
            vec![alife_core::channel_command_for_action(
                MotorChannel::Manipulation,
                &patch.decision().selected_action,
            )
            .unwrap()],
        )
        .unwrap()
    }

    fn captured_behavior(patch: &ExperiencePatch) -> alife_gpu_backend::GpuTrainingRolloutReceipt {
        // Synthetic GPU binding data over canonical CPU world outcomes tests
        // receipt accounting only, not inference, training or learned behavior.
        let frame = patch.pre_action().perception();
        alife_gpu_backend::GpuTrainingRolloutReceipt {
            organism_id: frame.organism_id().raw(),
            tick: frame.tick().raw(),
            dispatch_generation: patch
                .decision()
                .neural_evidence()
                .unwrap()
                .dispatch_generation,
            frame_digest: frame.frame_digest(),
            active_weight_generation: 0,
            sampling: alife_gpu_backend::GpuTrainingSamplingConfig {
                seed: 1,
                counter: 0,
                temperature: 1.0,
                demonstrator: None,
            },
            logits: Vec::new(),
            decoder_inputs: Vec::new(),
            decoder_input_stride: 24,
            representative_mask: 0,
            motor_masks: [0; 6],
            forced_motor_slots: Vec::new(),
            representative_index: 0,
            motor_indices: [u16::MAX; 6],
            factor_log_probabilities: [0.0; 7],
            behavior: alife_gpu_backend::GpuTrainingBehaviorKind::OnPolicy,
            joint_log_probability: Some(0.0),
            policy_joint_log_probability: 0.0,
        }
    }

    #[test]
    fn feeding_food_cycle_counts_real_nourishment_and_excludes_bootstrap_meal() {
        let (mut world, target) = measured_world_fixture();
        let position = world.entity(target).unwrap().position;
        // A disposable reachable Feeding scene, retaining natural hunger and
        // canonical action/biochemistry. No training or accepted brain is used.
        let scenario = super::super::configure_foundation_scenario(
            &mut world,
            42,
            Some(super::super::FoundationTeacherLesson::Feeding),
            Some([position.x, position.z]),
            false,
        )
        .unwrap();
        assert_eq!(scenario.food, target);
        assert!(scenario.care_food_subject().is_none());
        let organism = world.organism_entity_ids()[0].0;
        let mut receipt = FoundationFeedingFoodReceipt::new(organism, target);
        for row in 0..2 {
            let (observation, patch) = measured_eat(&mut world, target);
            let before_observe = world.canonical_signature_digest().unwrap();
            receipt
                .observe_row(
                    observation,
                    patch.pre_action().perception(),
                    &captured_behavior(&patch),
                    &patch,
                )
                .unwrap();
            assert_eq!(world.canonical_signature_digest().unwrap(), before_observe);
            assert_eq!(receipt.food_outcome.ordinary_acquisition_events, 1);
            assert_eq!(receipt.food_outcome.consumption_events, row);
            assert_eq!(receipt.food_outcome.nourished_meal_events, row);
        }
        assert_eq!(receipt.nourished_meal_rows, vec![1]);
        let bootstrap = receipt.clone().finish(1).unwrap();
        assert_eq!(bootstrap.trained_rows, 1);
        assert_eq!(
            crate::foundation_training_cycle::cycle_replay_dose(
                bootstrap.captured_rows,
                bootstrap.trained_rows,
                &bootstrap.nourished_meal_rows
            )
            .unwrap(),
            (1, 0)
        );
        let trained = receipt.finish(2).unwrap();
        assert_eq!(
            crate::foundation_training_cycle::cycle_replay_dose(
                trained.captured_rows,
                trained.trained_rows,
                &trained.nourished_meal_rows
            )
            .unwrap(),
            (0, 1)
        );
        let restored: FoundationFeedingFoodReceipt =
            serde_json::from_slice(&serde_json::to_vec(&trained).unwrap()).unwrap();
        assert_eq!(restored.food_outcome, trained.food_outcome);
        assert_eq!(restored.nourished_meal_rows, vec![1]);
    }

    #[test]
    fn feeding_food_cycle_rejects_unbound_capture_without_partial_credit() {
        let (mut world, target) = measured_world_fixture();
        measured_eat(&mut world, target);
        let (observation, patch) = measured_eat(&mut world, target);
        for variant in 0..5 {
            let mut receipt = FoundationFeedingFoodReceipt::new(observation.organism, target);
            let mut behavior = captured_behavior(&patch);
            let mut observation = observation;
            match variant {
                0 => behavior.tick += 1,
                1 => behavior.organism_id += 1,
                2 => behavior.dispatch_generation += 1,
                3 => behavior.frame_digest = alife_core::PerceptionFrameDigest([0; 4]),
                _ => observation.outcome_tick = observation.decision_tick,
            }
            assert!(
                receipt
                    .observe_row(
                        observation,
                        patch.pre_action().perception(),
                        &behavior,
                        &patch
                    )
                    .is_err(),
                "variant {variant}"
            );
            assert_eq!(receipt.captured_rows, 0);
            assert_eq!(receipt.food_outcome.nourished_meal_events, 0);
            assert!(receipt.nourished_meal_rows.is_empty());
        }
    }

    #[test]
    fn feeding_food_drop_regrab_and_repeat_eat_cannot_double_credit_nourishment() {
        let (mut world, target) = measured_world_fixture();
        let organism = world.organism_entity_ids()[0].0;
        let mut receipt = FoundationFeedingFoodReceipt::new(organism, target);
        let mut last_decision = None;
        for action in [
            HeadlessActionIds::EAT,
            HeadlessActionIds::GRAB,
            HeadlessActionIds::EAT,
            HeadlessActionIds::EAT,
        ] {
            let (observation, patch) = measured_action(&mut world, target, action);
            receipt
                .observe_row(
                    observation,
                    patch.pre_action().perception(),
                    &captured_behavior(&patch),
                    &patch,
                )
                .unwrap();
            last_decision = Some(patch.decision().clone());
        }
        assert_eq!(receipt.food_outcome.ordinary_acquisition_events, 2);
        assert_eq!(receipt.food_outcome.consumption_events, 1);
        assert_eq!(receipt.food_outcome.nourished_meal_events, 1);
        assert_eq!(receipt.nourished_meal_rows, vec![3]);
        // Once consumed, the world offers no second meal. An explicit retry
        // returns the actual non-consumption outcome and cannot add credit.
        let decision_tick = world.tick();
        let ownership_before = ownership(&world, target);
        let (_, outcome) = super::super::held_food_lesson_tests::eat_outcome(&mut world, target);
        assert!(!super::super::outcome_consumed(&outcome));
        let before = receipt.food_outcome.clone();
        // Exercise the food predicate with the retry's real world consequence;
        // this is not a new GPU-selected capture.
        receipt.food_outcome.observe_selected(
            FoodOutcomeObservation {
                organism,
                target,
                decision_tick,
                outcome_tick: world.tick(),
                before: ownership_before,
                after: ownership(&world, target),
            },
            &last_decision.unwrap(),
            &outcome,
        );
        assert_eq!(receipt.finish(4).unwrap().food_outcome, before);
    }

    #[test]
    fn food_outcome_eat_pickup_then_meal_has_zero_explicit_grab() {
        let (mut world, target) = measured_world_fixture();
        let learner = world.organism_entity_ids()[0].0;
        let start_tick = world.tick().raw();
        let mut summary = FoundationFoodOutcome::new(learner, target);
        for row in 0..2 {
            let (observation, patch) = measured_eat(&mut world, target);
            assert_eq!(observation.held_before(), Some(row == 1));
            let selected = bundle(&patch);
            assert!(
                crate::foundation_grab_food::selected_grab_command(&selected, target).is_none()
            );
            let events = summary.observe(observation, &patch);
            assert_eq!(
                events.acquisition,
                (row == 0).then_some(FoodAcquisitionKind::Eat)
            );
            assert_eq!(events.nourished, row == 1);
            assert_eq!(summary.explicit_grab_acquisition_events, 0);
            assert_eq!(summary.ordinary_acquisition_events, 1);
            assert_eq!(summary.consumption_events, row);
            assert_eq!(summary.nourished_meal_events, row);
            assert_eq!(
                patch.decision().selected_action.action_id,
                HeadlessActionIds::EAT
            );
            assert_eq!(patch.header().world_tick, Tick(start_tick + row as u64));
            assert_eq!(
                patch.outcome().outcome_tick,
                Tick(start_tick + row as u64 + 1)
            );
        }
        let historical = serde_json::json!({
            "seed": 53, "founder_seed_base": 53, "ticks": 2,
            "collection_seconds": 0, "replay_seconds": 0,
            "maximum_logit_error": 0, "maximum_activation_error": 0,
            "maximum_activity_ema_error": 0, "maximum_metabolic_error": 0,
            "source_asset_digest": "CPU reporting fixture", "teacher_mode": false,
            "consumed_events": 1, "demonstration_replay_records": 0,
            "food_position": null, "initial_food_distance": null, "final_food_distance": null,
            "lesson": "grab_food", "lesson_completed": false,
        });
        let mut receipt: crate::FoundationPilotReceipt =
            serde_json::from_value(historical).unwrap();
        assert!(receipt.food_outcome.is_none());
        receipt.food_outcome = Some(summary.clone());
        let pilot = serde_json::to_value(&receipt).unwrap();
        let diagnostic = serde_json::json!({"lesson_completed": false, "food_outcome": summary});
        for report in [pilot, diagnostic] {
            assert_eq!(report["lesson_completed"], false);
            assert_eq!(
                report["food_outcome"]["explicit_grab_acquisition_events"],
                0
            );
            assert_eq!(report["food_outcome"]["ordinary_acquisition_events"], 1);
            assert_eq!(report["food_outcome"]["nourished_meal_events"], 1);
        }
    }

    #[test]
    fn food_outcome_real_grab_transfer_and_release_preserve_boundary() {
        let (mut harness, mut mind, target) = fixture();
        let (acquisition, patch) = act(&mut harness, &mut mind, target, HeadlessActionIds::GRAB);
        assert!(
            crate::foundation_grab_food::selected_grab_command(&bundle(&patch), target).is_some()
        );
        let mut summary = FoundationFoodOutcome::new(LEARNER, target);
        let events = summary.observe(acquisition, &patch);
        assert_eq!(events.acquisition, Some(FoodAcquisitionKind::Grab));
        assert!(!events.nourished);
        // Production copies this count only from its unchanged strict GPU observer.
        summary.explicit_grab_acquisition_events = 1;
        assert_eq!(summary.ordinary_acquisition_events, 1);
        assert_eq!(summary.consumption_events, 0);
        assert_eq!(summary.nourished_meal_events, 0);
        let (release, patch) = act(&mut harness, &mut mind, target, HeadlessActionIds::GRAB);
        summary.observe(release, &patch);
        assert_eq!(release.after.unwrap().owner, None);
        assert_eq!(summary.ordinary_acquisition_events, 1);
        assert_eq!(summary.nourished_meal_events, 0);
    }

    #[test]
    fn food_outcome_rejects_wrong_owner_target_ticks_and_missing_snapshots() {
        let (mut harness, mut mind, target) = fixture();
        let (observation, patch) = act(&mut harness, &mut mind, target, HeadlessActionIds::EAT);
        let unheld = FoodOwnershipSnapshot {
            owner: None,
            consumed: false,
        };
        let other = FoodOwnershipSnapshot {
            owner: Some(OrganismId(99)),
            consumed: false,
        };
        let held = observation.after.unwrap();
        for bad in [
            FoodOutcomeObservation {
                target: WorldEntityId(target.raw() + 1),
                ..observation
            },
            FoodOutcomeObservation {
                organism: OrganismId(99),
                ..observation
            },
            FoodOutcomeObservation {
                decision_tick: Tick(99),
                ..observation
            },
            FoodOutcomeObservation {
                outcome_tick: Tick(99),
                ..observation
            },
            FoodOutcomeObservation {
                before: Some(other),
                ..observation
            },
            FoodOutcomeObservation {
                after: Some(other),
                ..observation
            },
            FoodOutcomeObservation {
                after: Some(unheld),
                ..observation
            },
            FoodOutcomeObservation {
                before: Some(held),
                ..observation
            },
            FoodOutcomeObservation {
                before: Some(FoodOwnershipSnapshot {
                    consumed: true,
                    ..unheld
                }),
                ..observation
            },
            FoodOutcomeObservation {
                after: Some(FoodOwnershipSnapshot {
                    consumed: true,
                    ..held
                }),
                ..observation
            },
            FoodOutcomeObservation {
                before: None,
                ..observation
            },
            FoodOutcomeObservation {
                after: None,
                ..observation
            },
        ] {
            let mut summary = FoundationFoodOutcome::new(LEARNER, target);
            let events = summary.observe(bad, &patch);
            assert!(events.acquisition.is_none());
            assert!(!events.nourished);
            assert_eq!(summary, FoundationFoodOutcome::new(LEARNER, target));
        }
        let missing = FoodOutcomeObservation {
            before: None,
            after: None,
            ..observation
        };
        let mut trace = serde_json::json!({});
        missing.decorate_trace(&mut trace);
        assert!(trace["lesson_target_ownership_before_decision"].is_null());
        assert!(trace["lesson_target_ownership_after_outcome"].is_null());
        assert_eq!(missing.held_before(), None);
        assert_eq!(observation.held_before(), Some(false));
    }

    #[test]
    fn food_outcome_unrelated_held_food_and_global_grip_do_not_prove_target_ownership() {
        let (mut harness, mut mind, target) = fixture();
        let other = harness.world().entity_id("other").unwrap();
        act(&mut harness, &mut mind, other, HeadlessActionIds::GRAB);
        assert_eq!(
            ownership(&harness.world(), other).unwrap().owner,
            Some(LEARNER)
        );
        // Ask the real rich perception path for grip. The legacy scaffold
        // pre-action snapshot above omits this channel, as distinct from zero grip.
        let mut rich_world = (*harness.world()).clone();
        let tick = rich_world.tick();
        let rich_frame = rich_world
            .perception_frame(
                LEARNER,
                tick,
                alife_core::SensorProfile::GroundedTerrainVisionV1,
                alife_core::HomeostaticSnapshot::baseline(tick),
            )
            .unwrap();
        assert_eq!(rich_frame.sensory().channels.tactile_contact[2], 1.0);
        let target_before = ownership(&harness.world(), target);
        let (mut observation, patch) = act(&mut harness, &mut mind, other, HeadlessActionIds::GRAB);
        observation.target = target;
        observation.before = target_before;
        observation.after = ownership(&harness.world(), target);
        assert_eq!(observation.held_before(), Some(false));
        let mut summary = FoundationFoodOutcome::new(LEARNER, target);
        summary.observe(observation, &patch);
        assert_eq!(summary, FoundationFoodOutcome::new(LEARNER, target));
    }

    #[test]
    fn food_outcome_bundle_uses_exact_manipulation_with_no_empty_bundle_fallback() {
        let (mut harness, mut mind, target) = fixture();
        let (observation, patch) = act(&mut harness, &mut mind, target, HeadlessActionIds::EAT);
        // Synthetic channel decoration of measured CPU outcomes exercises the
        // bundle predicate only; it does not claim a GPU produced this bundle.
        let mut decision = patch.decision().clone();
        decision.selected_bundle = Some(bundle(&patch));
        let mut outcome = patch.outcome().clone();
        outcome.joint = Some(
            alife_core::JointPhysicalOutcome::new(outcome.physical, vec![])
                .unwrap()
                .with_channel_outcomes(vec![alife_core::experience::ChannelPhysicalOutcome::new(
                    MotorChannel::Manipulation,
                    outcome.physical,
                )
                .unwrap()])
                .unwrap(),
        );
        let mut summary = FoundationFoodOutcome::new(LEARNER, target);
        let events = summary.observe_selected(observation, &decision, &outcome);
        assert_eq!(events.acquisition, Some(FoodAcquisitionKind::Eat));
        assert!(!events.nourished);
        assert_eq!(summary.ordinary_acquisition_events, 1);
        for variant in 0..4 {
            let mut wrong_decision = decision.clone();
            let mut wrong_outcome = outcome.clone();
            match variant {
                0 => wrong_decision
                    .selected_bundle
                    .as_mut()
                    .unwrap()
                    .channels
                    .clear(),
                1 => {
                    wrong_decision.selected_bundle.as_mut().unwrap().channels[0].target =
                        Some(ActionTarget::new(Some(WorldEntityId(999)), None))
                }
                2 => {
                    wrong_outcome.joint.as_mut().unwrap().channel_outcomes[0]
                        .physical
                        .target_entity = Some(WorldEntityId(999))
                }
                _ => {
                    wrong_outcome.joint.as_mut().unwrap().channel_outcomes[0].channel =
                        MotorChannel::Locomotion
                }
            }
            let mut summary = FoundationFoodOutcome::new(LEARNER, target);
            summary.observe_selected(observation, &wrong_decision, &wrong_outcome);
            assert_eq!(summary.ordinary_acquisition_events, 0);
        }
    }

    #[test]
    fn food_outcome_navigation_sees_movement_beneath_other_channel_contacts() {
        let (mut world, target) = measured_world_fixture();
        measured_eat(&mut world, target);
        let (_, patch) = measured_eat(&mut world, target);
        let mut outcome = patch.outcome().clone();
        use crate::foundation_training::teacher_outcome_moved;
        use alife_core::experience::ChannelPhysicalOutcome;
        for contact in [PhysicalContactKind::Touch, PhysicalContactKind::Consumed] {
            outcome.physical.contact = contact;
            outcome.joint = None;
            assert!(!teacher_outcome_moved(&outcome));
            let mut movement = outcome.physical;
            movement.contact = PhysicalContactKind::Moved;
            movement.target_entity = None;
            movement.displacement = alife_core::Vec3f::new(1.0, 0.0, 0.0);
            for channel in [MotorChannel::Manipulation, MotorChannel::Locomotion] {
                outcome.joint = Some(
                    alife_core::JointPhysicalOutcome::new(outcome.physical, Vec::new())
                        .unwrap()
                        .with_channel_outcomes(vec![
                            ChannelPhysicalOutcome::new(channel, movement).unwrap()
                        ])
                        .unwrap(),
                );
                assert_eq!(
                    teacher_outcome_moved(&outcome),
                    channel == MotorChannel::Locomotion
                );
            }
        }
    }

    #[test]
    fn food_outcome_blocked_move_keeps_exact_meal_and_requires_benefit() {
        let (mut world, target) = measured_world_fixture();
        let learner = world.organism_entity_ids()[0].0;
        measured_eat(&mut world, target);
        let (observation, patch) = measured_eat(&mut world, target);
        let mut decision = patch.decision().clone();
        let mut selected = bundle(&patch);
        let mut move_command = selected.channels[0].clone();
        move_command.channel = MotorChannel::Locomotion;
        move_command.primitive = HeadlessActionIds::STEP_FORWARD;
        move_command.target = None;
        selected.channels.push(move_command);
        decision.selected_bundle = Some(selected);
        let mut outcome = patch.outcome().clone();
        let consumption = outcome.physical;
        outcome.physical.contact = PhysicalContactKind::Blocked;
        outcome.physical.target_entity = None;
        outcome.success = false;
        outcome.joint = Some(
            alife_core::JointPhysicalOutcome::new(outcome.physical, vec![])
                .unwrap()
                .with_channel_outcomes(vec![
                    alife_core::experience::ChannelPhysicalOutcome::new(
                        MotorChannel::Manipulation,
                        consumption,
                    )
                    .unwrap(),
                    alife_core::experience::ChannelPhysicalOutcome::new(
                        MotorChannel::Locomotion,
                        outcome.physical,
                    )
                    .unwrap(),
                ])
                .unwrap(),
        );
        let mut summary = FoundationFoodOutcome::new(learner, target);
        let events = summary.observe_selected(observation, &decision, &outcome);
        assert!(events.acquisition.is_none());
        assert!(events.nourished);
        assert_eq!(summary.consumption_events, 1);
        assert_eq!(summary.nourished_meal_events, 1);
        #[cfg(feature = "cpu-training")]
        {
            use crate::foundation_cpu_care::trained_care_meal_completed;
            // This is an actual world Eat and measured biological benefit. A
            // bootstrap-only meal cannot stand in for a trained care decision.
            assert!(trained_care_meal_completed(&summary, 2, &[1]));
            use crate::foundation_training::grounded_navigation_completed;
            let meal = trained_care_meal_completed(&summary, 2, &[1]);
            assert!(!grounded_navigation_completed(meal, []));
            // A real meal alone does not prove navigation. Every route condition
            // matters, including a blocked joint-channel action.
            for route in [
                vec![(true, false, true)],
                vec![(false, false, true), (false, false, true)],
                vec![(true, false, false), (false, false, false)],
                vec![(true, false, true), (false, true, true)],
            ] {
                assert!(!grounded_navigation_completed(meal, route));
            }
            let route = [(true, false, true), (false, false, true)];
            assert!(grounded_navigation_completed(meal, route));
            assert!(!grounded_navigation_completed(false, route));
            // A visibility transition confined to the bootstrap row cannot pass.
            assert!(!grounded_navigation_completed(
                meal,
                route.into_iter().take(1)
            ));

            assert!(!trained_care_meal_completed(&summary, 1, &[1]));
            let mut generic_consumption = summary.clone();
            generic_consumption.nourished_meal_events = 0;
            assert!(!trained_care_meal_completed(&generic_consumption, 2, &[1]));
        }
        assert!(super::super::care_meal_completed(
            Some(&summary),
            learner,
            target
        ));
        assert!(!super::super::care_meal_completed(
            Some(&summary),
            OrganismId(999),
            target
        ));
        assert!(!super::super::care_meal_completed(
            Some(&summary),
            learner,
            WorldEntityId(999)
        ));
        assert!(!super::super::care_meal_completed(None, learner, target));
        for variant in 0..10 {
            let mut wrong_decision = decision.clone();
            let mut wrong_outcome = outcome.clone();
            let mut wrong_observation = observation;
            match variant {
                0 => {
                    wrong_decision.selected_bundle.as_mut().unwrap().channels[0].target =
                        Some(ActionTarget::new(Some(WorldEntityId(999)), None))
                }
                1 => wrong_outcome.measured_physiology = None,
                2 => {
                    let biology = wrong_outcome.measured_physiology.as_mut().unwrap();
                    biology.after = biology.before;
                }
                3 => {
                    wrong_outcome
                        .measured_physiology
                        .as_mut()
                        .unwrap()
                        .after
                        .body
                        .health -= 0.01
                }
                4 => {
                    wrong_outcome
                        .measured_physiology
                        .as_mut()
                        .unwrap()
                        .after
                        .body
                        .injury += 0.01
                }
                5 => {
                    wrong_outcome
                        .measured_physiology
                        .as_mut()
                        .unwrap()
                        .after
                        .homeostasis
                        .drives
                        .pain += 0.01
                }
                6 => {
                    wrong_observation.before = Some(FoodOwnershipSnapshot {
                        owner: Some(OrganismId(99)),
                        consumed: false,
                    })
                }
                7 => wrong_observation.after = None,
                8 => wrong_observation.before = None,
                _ => wrong_decision
                    .selected_bundle
                    .as_mut()
                    .unwrap()
                    .channels
                    .clear(),
            }
            let mut summary = FoundationFoodOutcome::new(learner, target);
            assert!(super::super::outcome_consumed(&wrong_outcome));
            let events =
                summary.observe_selected(wrong_observation, &wrong_decision, &wrong_outcome);
            assert!(!events.nourished, "variant {variant}");
            assert_eq!(summary.nourished_meal_events, 0, "variant {variant}");
        }
    }
}
