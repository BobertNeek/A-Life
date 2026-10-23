//! One physical scene relocation and passive, exact-target GPU capture evidence.
//! This observer never chooses an action or contributes curriculum reward.
use super::food_outcome::{ownership, FoodOutcomeObservation, FoodOwnershipSnapshot};
use super::{FoundationFoodOutcome, FoundationScenarioSetup, Result};
use alife_core::{OrganismId, Tick, Validate, Vec3f, WorldEntityId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationFoodRelocationEvent {
    /// First possible captured row after the move, including a later waking row.
    pub row: usize,
    pub decision_tick: u64,
    pub from_position: Vec3f,
    pub to_position: Vec3f,
    pub ownership_before: FoodOwnershipSnapshot,
    pub ownership_after: FoodOwnershipSnapshot,
}

/// Outcome observations in an assessment window, with no optimizer/loss dose.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationFoodRelocationAssessment {
    pub rows: usize,
    pub pre_relocation_visible_rows: usize,
    pub post_relocation_visible_rows: usize,
    pub post_relocation_acquisitions: usize,
    pub post_relocation_nourished_meals: usize,
    pub completed: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationFoodRelocationReceipt {
    pub schema: u16,
    pub organism: OrganismId,
    pub target: WorldEntityId,
    pub setup_tick: u64,
    pub initial_position: Vec3f,
    pub relocation_target: Vec3f,
    pub relocation: Option<FoundationFoodRelocationEvent>,
    pub captured_rows: usize,
    /// Actual optimizer/loss rows only; cycle bootstrap and assessment are excluded.
    pub trained_rows: usize,
    pub acquisition_rows: Vec<usize>,
    pub nourished_meal_rows: Vec<usize>,
    pub trained_post_relocation_acquisitions: usize,
    pub trained_post_relocation_nourished_meals: usize,
    pub trained_pre_relocation_visible_rows: usize,
    pub trained_post_relocation_visible_rows: usize,
    pub food_outcome: FoundationFoodOutcome,
    /// Completion within actual training rows; assessments report separately.
    pub lesson_completed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assessment: Option<FoundationFoodRelocationAssessment>,
    pub rows: Vec<serde_json::Value>,
}

fn target_visible_in_captured_slots(
    world: &alife_world::HeadlessWorld,
    frame: &alife_core::PerceptionFrame,
    target: WorldEntityId,
) -> bool {
    let Some(target) = world.entity(target) else {
        return false;
    };
    // Raw transports are intentionally absent from the neural frame. Resolve
    // its actual captured tracked slots through the world-owned physical key;
    // the candidate shortlist may omit visible objects beyond its first three.
    frame.grounded_object_slots().iter().any(|slot| {
        world
            .tracked_objects()
            .record(frame.organism_id(), slot.tracked_object_id)
            .is_some_and(|record| {
                record.tracking_key == target.tracking_key
                    && record.tracking_provenance == target.tracking_provenance
                    && record.last_seen_tick == frame.tick()
            })
    })
}

pub(crate) struct RelocatedFoodCollector {
    receipt: FoundationFoodRelocationReceipt,
}

#[derive(Clone, Copy)]
pub(crate) struct RelocatedFoodStepStart {
    tick: Tick,
    position: Vec3f,
    ownership: FoodOwnershipSnapshot,
}

impl RelocatedFoodCollector {
    pub(crate) fn new(
        world: &alife_world::HeadlessWorld,
        scenario: &FoundationScenarioSetup,
        seed: u64,
    ) -> Result<Self> {
        let (organism, body) = *world
            .organism_entity_ids()
            .first()
            .ok_or("relocated-food learner missing")?;
        let origin = world.entity(body).ok_or("learner body missing")?.position;
        let initial_position = world.entity(scenario.food).ok_or("food missing")?.position;
        let side = if seed & 1 == 0 { 1.0 } else { -1.0 };
        // Identical sideways geometry to the reviewed CPU care lesson.
        let relocation_target = Vec3f::new(
            origin.x - side * (initial_position.z - origin.z),
            initial_position.y,
            origin.z + side * (initial_position.x - origin.x),
        );
        if relocation_target == initial_position {
            return Err("relocated-food scene requires an actual sideways move".into());
        }
        Ok(Self {
            receipt: FoundationFoodRelocationReceipt {
                schema: 1,
                organism,
                target: scenario.food,
                setup_tick: scenario.start_tick,
                initial_position,
                relocation_target,
                relocation: None,
                captured_rows: 0,
                trained_rows: 0,
                acquisition_rows: Vec::new(),
                nourished_meal_rows: Vec::new(),
                trained_post_relocation_acquisitions: 0,
                trained_post_relocation_nourished_meals: 0,
                trained_pre_relocation_visible_rows: 0,
                trained_post_relocation_visible_rows: 0,
                food_outcome: FoundationFoodOutcome::new(organism, scenario.food),
                lesson_completed: false,
                assessment: None,
                rows: Vec::new(),
            },
        })
    }

    pub(crate) fn before_step(
        &mut self,
        world: &mut alife_world::HeadlessWorld,
    ) -> Result<RelocatedFoodStepStart> {
        let target = self.receipt.target;
        if self.receipt.relocation.is_none()
            && world.tick().raw().saturating_sub(self.receipt.setup_tick) >= 2
        {
            let from_position = world.entity(target).ok_or("food missing")?.position;
            let ownership_before = ownership(world, target).ok_or("food missing")?;
            if from_position != self.receipt.relocation_target
                && super::relocate_unheld_foundation_food(
                    world,
                    target,
                    self.receipt.relocation_target,
                )?
            {
                let food = world.entity(target).ok_or("relocated food missing")?;
                if food.position != self.receipt.relocation_target
                    || food.position == from_position
                    || food.grounded_physical.velocity != Vec3f::ZERO
                {
                    return Err("relocated-food world move was not realized".into());
                }
                self.receipt.relocation = Some(FoundationFoodRelocationEvent {
                    row: self.receipt.rows.len(),
                    decision_tick: world.tick().raw(),
                    from_position,
                    to_position: food.position,
                    ownership_before,
                    ownership_after: ownership(world, target).ok_or("food missing")?,
                });
            }
        }
        Ok(RelocatedFoodStepStart {
            tick: world.tick(),
            position: world.entity(target).ok_or("food missing")?.position,
            ownership: ownership(world, target).ok_or("food missing")?,
        })
    }

    pub(crate) fn observe_step(
        &mut self,
        world: &alife_world::HeadlessWorld,
        before: RelocatedFoodStepStart,
        step: &crate::FoundationTrainingStep,
    ) -> Result<()> {
        self.observe_row(world, before, &step.frame, &step.behavior, &step.patch)
    }

    fn observe_row(
        &mut self,
        world: &alife_world::HeadlessWorld,
        before: RelocatedFoodStepStart,
        frame: &alife_core::PerceptionFrame,
        behavior: &alife_gpu_backend::GpuTrainingRolloutReceipt,
        patch: &alife_core::ExperiencePatch,
    ) -> Result<()> {
        patch.validate_contract()?;
        let observation = FoodOutcomeObservation {
            organism: self.receipt.organism,
            target: self.receipt.target,
            decision_tick: before.tick,
            outcome_tick: world.tick(),
            before: Some(before.ownership),
            after: ownership(world, self.receipt.target),
        };
        if !observation.matches_row(frame, behavior, patch)
            || patch.decision().policy_backend() != alife_core::PolicyBackend::NeuralClosedLoopGpu
            || behavior.behavior != alife_gpu_backend::GpuTrainingBehaviorKind::OnPolicy
            || behavior.sampling.demonstrator.is_some()
        {
            return Err("relocated-food capture identity or on-policy evidence mismatch".into());
        }
        behavior.on_policy_log_probability()?;
        let representative = frame
            .candidates()
            .get(behavior.representative_index as usize)
            .ok_or("relocated-food representative is outside the captured candidates")?;
        let selected = &patch.decision().selected_action;
        if representative.action_id != selected.action_id
            || representative.target.entity != selected.target_entity
        {
            return Err("relocated-food selected action differs from sampled GPU candidate".into());
        }
        if let Some(command) = patch.selected_bundle().and_then(|bundle| {
            bundle
                .channels
                .iter()
                .find(|command| command.channel == alife_core::MotorChannel::Manipulation)
        }) {
            let candidate = frame
                .candidates()
                .get(behavior.motor_indices[2] as usize)
                .ok_or("relocated-food manipulation is outside sampled GPU candidates")?;
            if candidate.action_id != command.primitive
                || candidate.target.entity != command.target.and_then(|target| target.entity)
            {
                return Err(
                    "relocated-food manipulation differs from sampled GPU candidate".into(),
                );
            }
        }
        let row = self.receipt.rows.len();
        if self.receipt.rows.last().is_some_and(|previous| {
            previous["outcome_tick"]
                .as_u64()
                .is_none_or(|tick| tick > before.tick.raw())
        }) {
            return Err("relocated-food capture ticks overlap or regress".into());
        }
        let acquisitions_before = self.receipt.food_outcome.ordinary_acquisition_events;
        let meals_before = self.receipt.food_outcome.nourished_meal_events;
        self.receipt.food_outcome.observe(observation, patch);
        let acquisition =
            self.receipt.food_outcome.ordinary_acquisition_events > acquisitions_before;
        let nourished_meal = self.receipt.food_outcome.nourished_meal_events > meals_before;
        if acquisition {
            self.receipt.acquisition_rows.push(row);
        }
        if nourished_meal {
            self.receipt.nourished_meal_rows.push(row);
        }
        let evidence = patch.decision().neural_evidence()?;
        let outcome = patch.outcome();
        let mut trace = serde_json::json!({
            "organism_id": self.receipt.organism,
            "sequence_id": patch.header().sequence_id,
            "decision_tick": before.tick,
            "outcome_tick": world.tick(),
            "frame_digest": evidence.frame_digest,
            "dispatch_generation": evidence.dispatch_generation,
            "behavior": behavior.behavior,
            "food_visible": target_visible_in_captured_slots(world, frame, self.receipt.target),
            "selected_action": patch.decision().selected_action,
            "selected_commands": patch.selected_bundle().map(|bundle| bundle.channels.iter()
                .filter(|command| command.channel == alife_core::MotorChannel::Manipulation)
                .map(|command| serde_json::json!({"channel":command.channel,
                    "primitive":command.primitive,"target":command.target}))
                .collect::<Vec<_>>()),
            "physical": outcome.physical,
            "channel_outcomes": outcome.joint.as_ref().map(|joint| joint.channel_outcomes.iter()
                .filter(|channel| channel.channel == alife_core::MotorChannel::Manipulation)
                .collect::<Vec<_>>()),
            "physiology": outcome.measured_physiology.map(|transition| serde_json::json!({
                "before_energy":transition.before.body.energy,"after_energy":transition.after.body.energy,
                "before_hunger":transition.before.homeostasis.drives.hunger,"after_hunger":transition.after.homeostasis.drives.hunger,
                "before_health":transition.before.body.health,"after_health":transition.after.body.health,
                "before_injury":transition.before.body.injury,"after_injury":transition.after.body.injury,
                "before_pain":transition.before.homeostasis.drives.pain,"after_pain":transition.after.homeostasis.drives.pain,
            })),
        });
        observation.decorate_trace(&mut trace);
        trace["row"] = serde_json::json!(row);
        trace["food_relocated"] = serde_json::json!(self.receipt.relocation.is_some());
        trace["food_position_before_decision"] = serde_json::json!(before.position);
        trace["food_position_after_outcome"] =
            serde_json::json!(world.entity(self.receipt.target).map(|food| food.position));
        trace["ordinary_acquisition"] = serde_json::json!(acquisition);
        trace["nourished_meal"] = serde_json::json!(nourished_meal);
        self.receipt.rows.push(trace);
        Ok(())
    }

    fn outcome_window(&self, rows: usize) -> FoundationFoodRelocationAssessment {
        let mut window = FoundationFoodRelocationAssessment {
            rows,
            pre_relocation_visible_rows: 0,
            post_relocation_visible_rows: 0,
            post_relocation_acquisitions: 0,
            post_relocation_nourished_meals: 0,
            completed: false,
        };
        for trace in self.receipt.rows.iter().take(rows) {
            if trace["food_visible"] == true {
                if trace["food_relocated"] == true {
                    window.post_relocation_visible_rows += 1;
                } else {
                    window.pre_relocation_visible_rows += 1;
                }
            }
        }
        if let Some(event) = &self.receipt.relocation {
            let eligible = |row: &&usize| **row >= event.row && **row < rows;
            window.post_relocation_acquisitions = self
                .receipt
                .acquisition_rows
                .iter()
                .filter(eligible)
                .count();
            window.post_relocation_nourished_meals = self
                .receipt
                .nourished_meal_rows
                .iter()
                .filter(eligible)
                .filter(|meal| {
                    self.receipt
                        .acquisition_rows
                        .iter()
                        .any(|acquisition| *acquisition >= event.row && acquisition < *meal)
                })
                .count();
            window.completed = window.post_relocation_nourished_meals > 0
                && window.pre_relocation_visible_rows > 0
                && window.post_relocation_visible_rows > 0;
        }
        window
    }

    pub(crate) fn finish(mut self, trained_rows: usize) -> Result<FoundationFoodRelocationReceipt> {
        let captured_rows = self.receipt.rows.len();
        if trained_rows > captured_rows || captured_rows - trained_rows > 1 {
            return Err("relocated-food training window is inconsistent".into());
        }
        self.receipt.captured_rows = captured_rows;
        self.receipt.trained_rows = trained_rows;
        let window = self.outcome_window(trained_rows);
        self.receipt.trained_pre_relocation_visible_rows = window.pre_relocation_visible_rows;
        self.receipt.trained_post_relocation_visible_rows = window.post_relocation_visible_rows;
        self.receipt.trained_post_relocation_acquisitions = window.post_relocation_acquisitions;
        self.receipt.trained_post_relocation_nourished_meals =
            window.post_relocation_nourished_meals;
        self.receipt.lesson_completed = window.completed;
        self.receipt.assessment = None;
        for (row, trace) in self.receipt.rows.iter_mut().enumerate() {
            trace["loss_row"] = serde_json::json!(row < trained_rows);
        }
        Ok(self.receipt)
    }

    pub(crate) fn finish_assessment(
        mut self,
        assessment_rows: usize,
    ) -> Result<FoundationFoodRelocationReceipt> {
        if assessment_rows != self.receipt.rows.len() {
            return Err(
                "relocated-food assessment window differs from captured observations".into(),
            );
        }
        self.receipt.captured_rows = assessment_rows;
        self.receipt.trained_rows = 0;
        self.receipt.trained_pre_relocation_visible_rows = 0;
        self.receipt.trained_post_relocation_visible_rows = 0;
        self.receipt.trained_post_relocation_acquisitions = 0;
        self.receipt.trained_post_relocation_nourished_meals = 0;
        self.receipt.lesson_completed = false;
        self.receipt.assessment = Some(self.outcome_window(assessment_rows));
        for trace in &mut self.receipt.rows {
            trace["loss_row"] = serde_json::json!(false);
        }
        Ok(self.receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{
        configure_foundation_scenario, held_food_lesson_tests as measured, FoundationTeacherLesson,
    };
    use super::*;
    use alife_core::{
        ExperiencePatch, ExperienceSequenceId, HomeostaticDelta, NormalizedScalar, PerceptionFrame,
        PostActionOutcome, SignedValence,
    };
    use alife_gpu_backend::{
        GpuTrainingBehaviorKind, GpuTrainingRolloutReceipt, GpuTrainingSamplingConfig,
    };

    fn fixture(seed: u64) -> (alife_world::HeadlessWorld, RelocatedFoodCollector) {
        let mut world = measured::unconfigured(seed);
        let scenario = configure_foundation_scenario(
            &mut world,
            seed,
            Some(FoundationTeacherLesson::GrabFood),
            None,
            false,
        )
        .unwrap();
        let body = world.entity(world.organism_entity_ids()[0].1).unwrap();
        let origin = body.position;
        let side = if seed & 1 == 0 { 1.0 } else { -1.0 };
        let angle = body.body_yaw + body.head_yaw - side * std::f32::consts::FRAC_PI_4;
        // A small physical arena isolates acquisition/biology from locomotion.
        // Production relocated-food scenes retain the reviewed 2.2..3.6 distance.
        super::super::move_scenario_object(
            &mut world,
            scenario.food,
            Vec3f::new(
                origin.x + 0.2 * angle.cos(),
                origin.y,
                origin.z + 0.2 * angle.sin(),
            ),
        )
        .unwrap();
        let collector = RelocatedFoodCollector::new(&world, &scenario, seed).unwrap();
        (world, collector)
    }

    fn act(
        world: &mut alife_world::HeadlessWorld,
        collector: &mut RelocatedFoodCollector,
        eat: bool,
        target: Option<WorldEntityId>,
    ) -> (
        RelocatedFoodStepStart,
        PerceptionFrame,
        GpuTrainingRolloutReceipt,
        ExperiencePatch,
    ) {
        let before = collector.before_step(world).unwrap();
        let frame = measured::frame(world);
        let (organism, entity) = world.organism_entity_ids()[0];
        let command = if eat {
            alife_world::HeadlessWorldCommand::eat(organism, target.unwrap()).unwrap()
        } else {
            alife_world::HeadlessWorldCommand::idle(organism).unwrap()
        };
        let representative_index = frame
            .candidates()
            .iter()
            .position(|candidate| {
                candidate.action_id == command.action_id && candidate.target.entity == target
            })
            .expect("physical fixture action must be a legal perceived candidate")
            as u16;
        let execution = world
            .apply_registered_command(&command, entity, Tick(world.tick().raw() + 1))
            .unwrap();
        world.try_advance_tick().unwrap();
        let physiology = alife_core::MeasuredPhysiologyTransition::new(
            execution.biology_before,
            execution.biology_after,
        )
        .unwrap();
        let outcome = PostActionOutcome::new(
            organism,
            ExperienceSequenceId(before.tick.raw() + 1),
            world.tick(),
            execution.action_result.execution.succeeded,
            execution.action_result.execution.physical,
            HomeostaticDelta::zero(),
            SignedValence::ZERO,
            NormalizedScalar::ZERO,
            NormalizedScalar::ZERO,
            SignedValence::ZERO,
            NormalizedScalar::ZERO,
        )
        .unwrap()
        .with_measured_physiology(physiology)
        .unwrap();
        let patch = measured::action_patch(frame.clone(), target, outcome, command.action_id, None);
        // Synthetic GPU identity/mask data tests host receipt binding over real
        // canonical physical and biological outcomes. This does not claim GPU
        // inference or an actual sampled on-policy success.
        let behavior = GpuTrainingRolloutReceipt {
            organism_id: organism.raw(),
            tick: before.tick.raw(),
            dispatch_generation: patch
                .decision()
                .neural_evidence()
                .unwrap()
                .dispatch_generation,
            frame_digest: frame.frame_digest(),
            active_weight_generation: 0,
            sampling: GpuTrainingSamplingConfig {
                seed: 1,
                counter: 0,
                temperature: 1.0,
                demonstrator: None,
            },
            logits: vec![0.0; frame.candidates().len()],
            decoder_inputs: Vec::new(),
            decoder_input_stride: 24,
            representative_mask: 0,
            motor_masks: [0; 6],
            forced_motor_slots: Vec::new(),
            representative_index,
            motor_indices: [u16::MAX; 6],
            factor_log_probabilities: [0.0; 7],
            behavior: GpuTrainingBehaviorKind::OnPolicy,
            joint_log_probability: Some(0.0),
            policy_joint_log_probability: 0.0,
        };
        (before, frame, behavior, patch)
    }

    fn collect(
        world: &mut alife_world::HeadlessWorld,
        collector: &mut RelocatedFoodCollector,
        eat: bool,
    ) {
        let target = eat.then_some(collector.receipt.target);
        let (before, frame, behavior, patch) = act(world, collector, eat, target);
        collector
            .observe_row(world, before, &frame, &behavior, &patch)
            .unwrap();
    }

    #[test]
    fn relocated_food_moves_once_sideways_at_elapsed_two_and_stays_static() {
        for seed in [42, 43] {
            let (mut world, mut collector) = fixture(seed);
            let target = collector.receipt.target;
            let initial = world.entity(target).unwrap().position;
            let body = world
                .entity(world.organism_entity_ids()[0].1)
                .unwrap()
                .position;
            collector.before_step(&mut world).unwrap();
            world.try_advance_tick().unwrap();
            collector.before_step(&mut world).unwrap();
            assert!(collector.receipt.relocation.is_none());
            assert_eq!(world.entity(target).unwrap().position, initial);
            world.try_advance_tick().unwrap();
            collector.before_step(&mut world).unwrap();
            let event = collector.receipt.relocation.as_ref().unwrap().clone();
            assert_eq!(event.decision_tick, collector.receipt.setup_tick + 2);
            assert_eq!(event.from_position, initial);
            let side = if seed & 1 == 0 { 1.0 } else { -1.0 };
            assert_eq!(
                event.to_position,
                Vec3f::new(
                    body.x - side * (initial.z - body.z),
                    initial.y,
                    body.z + side * (initial.x - body.x)
                )
            );
            assert_eq!(event.ownership_before, event.ownership_after);
            for _ in 0..3 {
                world.try_advance_tick().unwrap();
                collector.before_step(&mut world).unwrap();
                assert_eq!(world.entity(target).unwrap().position, event.to_position);
                assert_eq!(
                    world.entity(target).unwrap().grounded_physical.velocity,
                    Vec3f::ZERO
                );
            }
            assert_eq!(
                collector.receipt.relocation.as_ref().unwrap().decision_tick,
                event.decision_tick
            );
        }
    }

    #[test]
    fn relocated_food_owned_pickup_then_nourishing_meal_excludes_bootstrap() {
        let (mut world, mut collector) = fixture(42);
        collect(&mut world, &mut collector, false);
        collect(&mut world, &mut collector, false);
        collect(&mut world, &mut collector, true);
        assert_eq!(collector.receipt.acquisition_rows, vec![2]);
        assert!(collector.receipt.nourished_meal_rows.is_empty());
        collect(&mut world, &mut collector, true);
        let receipt = collector.finish(4).unwrap();
        assert_eq!(receipt.relocation.as_ref().unwrap().row, 2);
        assert_eq!(receipt.nourished_meal_rows, vec![3]);
        assert_eq!(receipt.trained_post_relocation_acquisitions, 1);
        assert_eq!(receipt.trained_post_relocation_nourished_meals, 1);
        assert_eq!(receipt.trained_pre_relocation_visible_rows, 2);
        assert_eq!(receipt.trained_post_relocation_visible_rows, 2);
        assert!(receipt.lesson_completed);
        assert_eq!(
            receipt.rows[2]["lesson_target_ownership_before_decision"]["owner"],
            serde_json::Value::Null
        );
        assert_eq!(
            receipt.rows[2]["lesson_target_ownership_after_outcome"]["owner"],
            receipt.organism.raw()
        );
        assert_eq!(
            receipt.rows[3]["lesson_target_ownership_after_outcome"]["consumed"],
            true
        );
        let encoded = serde_json::to_vec(&receipt).unwrap();
        assert!(
            encoded.len() < 20_000,
            "compact four-row receipt unexpectedly grew"
        );
        let bootstrap = RelocatedFoodCollector {
            receipt: receipt.clone(),
        };
        let bootstrap = bootstrap.finish(3).unwrap();
        assert_eq!(bootstrap.food_outcome.nourished_meal_events, 1);
        assert_eq!(bootstrap.trained_post_relocation_nourished_meals, 0);
        assert_eq!(bootstrap.rows[3]["loss_row"], false);
        assert!(!bootstrap.lesson_completed);
    }

    #[test]
    fn relocated_food_assessment_can_complete_without_any_training_or_loss_rows() {
        let (mut world, mut collector) = fixture(42);
        collect(&mut world, &mut collector, false);
        collect(&mut world, &mut collector, false);
        collect(&mut world, &mut collector, true);
        collect(&mut world, &mut collector, true);
        let incomplete = RelocatedFoodCollector {
            receipt: collector.receipt.clone(),
        };
        assert!(incomplete.finish_assessment(3).is_err());
        let receipt = collector.finish_assessment(4).unwrap();
        assert_eq!(receipt.captured_rows, 4);
        assert_eq!(receipt.trained_rows, 0);
        assert_eq!(receipt.trained_pre_relocation_visible_rows, 0);
        assert_eq!(receipt.trained_post_relocation_visible_rows, 0);
        assert_eq!(receipt.trained_post_relocation_acquisitions, 0);
        assert_eq!(receipt.trained_post_relocation_nourished_meals, 0);
        assert!(!receipt.lesson_completed);
        assert!(receipt.rows.iter().all(|row| row["loss_row"] == false));
        let assessment = receipt.assessment.as_ref().unwrap();
        assert_eq!(assessment.rows, 4);
        assert_eq!(assessment.pre_relocation_visible_rows, 2);
        assert_eq!(assessment.post_relocation_visible_rows, 2);
        assert_eq!(assessment.post_relocation_acquisitions, 1);
        assert_eq!(assessment.post_relocation_nourished_meals, 1);
        assert!(assessment.completed);
        let decoded: FoundationFoodRelocationReceipt =
            serde_json::from_slice(&serde_json::to_vec(&receipt).unwrap()).unwrap();
        assert!(decoded.assessment.unwrap().completed);
        assert_eq!(decoded.trained_rows, 0);
    }

    #[test]
    fn relocated_food_visible_sensory_slot_survives_candidate_top_three_exclusion() {
        let (mut world, mut collector) = fixture(42);
        let body = world.entity(world.organism_entity_ids()[0].1).unwrap();
        let origin = body.position;
        let gaze = body.body_yaw + body.head_yaw;
        // Remove optical occlusion between these tightly packed test objects;
        // all remain ordinary world objects in the actual sensory snapshot.
        world
            .editor_set_optical_opacity(collector.receipt.target, 0.0)
            .unwrap();
        for index in 0..3 {
            let radius = 0.02 + index as f32 * 0.02;
            let position = Vec3f::new(
                origin.x + radius * gaze.cos(),
                origin.y,
                origin.z + radius * gaze.sin(),
            );
            let decoy = world
                .spawn_toy(&format!("visible-decoy-{index}"), position, false)
                .unwrap();
            world.editor_set_optical_opacity(decoy, 0.0).unwrap();
        }
        let (before, frame, behavior, patch) = act(&mut world, &mut collector, false, None);
        assert!(frame.grounded_object_slots().len() > 3);
        assert!(!frame
            .candidates()
            .iter()
            .any(|candidate| candidate.target.entity == Some(collector.receipt.target)));
        assert!(target_visible_in_captured_slots(
            &world,
            &frame,
            collector.receipt.target
        ));
        collector
            .observe_row(&world, before, &frame, &behavior, &patch)
            .unwrap();
        assert_eq!(collector.receipt.rows[0]["food_visible"], true);
        let receipt = collector.finish(1).unwrap();
        assert_eq!(receipt.trained_pre_relocation_visible_rows, 1);
        assert!(receipt.assessment.is_none());
    }

    #[test]
    fn relocated_food_player_hold_and_pre_relocation_meal_do_not_complete() {
        let (mut world, mut collector) = fixture(42);
        world.begin_player_hold(collector.receipt.target).unwrap();
        for _ in 0..3 {
            world.try_advance_tick().unwrap();
        }
        let initial = world.entity(collector.receipt.target).unwrap().position;
        collector.before_step(&mut world).unwrap();
        assert!(collector.receipt.relocation.is_none());
        assert_eq!(
            world.entity(collector.receipt.target).unwrap().position,
            initial
        );
        assert!(!collector.finish(0).unwrap().lesson_completed);

        let (mut world, mut collector) = fixture(42);
        collect(&mut world, &mut collector, true);
        collect(&mut world, &mut collector, true);
        collector.before_step(&mut world).unwrap();
        let receipt = collector.finish(2).unwrap();
        assert!(receipt.relocation.is_none());
        assert_eq!(receipt.food_outcome.nourished_meal_events, 1);
        assert_eq!(receipt.trained_post_relocation_nourished_meals, 0);
        assert!(!receipt.lesson_completed);
    }

    #[test]
    fn relocated_food_rejects_mismatched_capture_and_demonstrator_receipts() {
        let (mut world, mut collector) = fixture(42);
        let (before, frame, behavior, patch) = act(&mut world, &mut collector, false, None);
        for mutation in 0..5 {
            let mut wrong = behavior.clone();
            match mutation {
                0 => wrong.organism_id += 1,
                1 => wrong.tick += 1,
                2 => wrong.dispatch_generation += 1,
                3 => wrong.behavior = GpuTrainingBehaviorKind::Demonstrator,
                _ => wrong.joint_log_probability = None,
            }
            assert!(collector
                .observe_row(&world, before, &frame, &wrong, &patch)
                .is_err());
        }
        assert!(collector.receipt.rows.is_empty());
        collector
            .observe_row(&world, before, &frame, &behavior, &patch)
            .unwrap();
        assert!(collector
            .observe_row(&world, before, &frame, &behavior, &patch)
            .is_err());
    }

    #[test]
    fn relocated_food_real_alternate_target_meal_never_counts_for_intended_food() {
        let (mut world, mut collector) = fixture(42);
        collect(&mut world, &mut collector, false);
        collect(&mut world, &mut collector, false);
        collector.before_step(&mut world).unwrap();
        let other = world.entity_id("food-02").unwrap();
        let body = world.entity(world.organism_entity_ids()[0].1).unwrap();
        let origin = body.position;
        let angle = body.body_yaw + body.head_yaw;
        super::super::move_scenario_object(
            &mut world,
            other,
            Vec3f::new(
                origin.x + 0.05 * angle.cos(),
                origin.y,
                origin.z + 0.05 * angle.sin(),
            ),
        )
        .unwrap();
        for _ in 0..2 {
            let (before, frame, behavior, patch) =
                act(&mut world, &mut collector, true, Some(other));
            collector
                .observe_row(&world, before, &frame, &behavior, &patch)
                .unwrap();
        }
        assert!(world.entity(other).unwrap().consumed);
        let receipt = collector.finish(4).unwrap();
        assert!(receipt.acquisition_rows.is_empty());
        assert!(receipt.nourished_meal_rows.is_empty());
        assert!(!receipt.lesson_completed);
    }

    #[test]
    fn relocated_food_consumption_without_measured_nourishment_cannot_complete() {
        let (mut world, mut collector) = fixture(42);
        collect(&mut world, &mut collector, false);
        collect(&mut world, &mut collector, false);
        collect(&mut world, &mut collector, true);
        let target = collector.receipt.target;
        let (before, frame, behavior, patch) = act(&mut world, &mut collector, true, Some(target));
        assert_eq!(
            patch.outcome().physical.contact,
            alife_core::PhysicalContactKind::Consumed
        );
        let mut partial = patch.outcome().clone();
        partial.measured_physiology = None;
        // Drop biology only in this synthetic reporting regression; the real
        // canonical world has already carried out and measured the meal.
        let partial = measured::eating_narration_patch(frame.clone(), target, partial);
        collector
            .observe_row(&world, before, &frame, &behavior, &partial)
            .unwrap();
        let receipt = collector.finish(4).unwrap();
        assert_eq!(receipt.food_outcome.consumption_events, 1);
        assert_eq!(receipt.food_outcome.nourished_meal_events, 0);
        assert!(receipt.rows[3]["physiology"].is_null());
        assert!(!receipt.lesson_completed);
    }
}
