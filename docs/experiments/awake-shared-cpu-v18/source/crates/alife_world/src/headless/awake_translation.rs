//! Explicit shared-time contact actuator factor. No ordinary caller activation.
use super::translation_budget::TranslationIntervalBudget;
use super::*;

/// Matches the existing actuator geometry without changing terrain walking.
/// Receipt path lengths remain spatial even when elapsed walking time is planar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TranslationTimeMetric {
    #[default]
    Spatial,
    GroundPlane,
}

impl TranslationTimeMetric {
    fn path_length(self, displacement: Vec3f) -> f32 {
        match self {
            Self::Spatial => distance(Vec3f::ZERO, displacement),
            Self::GroundPlane => displacement.x.hypot(displacement.z),
        }
    }
}

/// The voluntary receipt retains selected intent and actual per-channel motion.
/// Its body event/biology include the reflex once; `actual_displacement` also
/// includes the separately identified involuntary movement. This wrapper is not
/// an ordinary sealed learning receipt and must not be silently treated as one.
#[derive(Debug, Clone, PartialEq)]
pub struct AwakeTranslationIntervalReceipt {
    pub voluntary: HeadlessMotorTransactionReceipt,
    pub withdrawal: Option<ContactWithdrawalReceipt>,
    pub actual_displacement: Vec3f,
    pub allocated_translation_time: f32,
    pub translation_time_metric: TranslationTimeMetric,
    pub voluntary_time_allocated: f32,
    pub reflex_time_allocated: f32,
    pub reflex_budget_exhausted: bool,
    pub reflex_path: f32,
    pub voluntary_path: f32,
    pub voluntary_translation_energy: f32,
}

#[derive(Default)]
pub(super) struct AwakeTranslationState {
    pub(super) withdrawal: Option<ContactWithdrawalReceipt>,
    budget: Option<TranslationIntervalBudget>,
    voluntary_path: f32,
    voluntary_translation_energy: f32,
    capacity: Option<f32>,
    gain: f32,
    time_metric: TranslationTimeMetric,
    pub(super) thermal_peak: f32,
    voluntary_time: f32,
    reflex_time: f32,
}

pub(super) struct SweptContactPlan {
    pub(super) position: Vec3f,
    pub(super) sources: Vec<WorldEntityId>,
    duration: f32,
}

impl AwakeTranslationState {
    pub(super) fn begin(
        &mut self,
        world: &mut HeadlessWorld,
        organism: OrganismId,
        entity: WorldEntityId,
        capacity: Option<f32>,
        embodiment: &EmbodimentState,
    ) -> Result<(), ScaffoldContractError> {
        let physical_capacity = capacity.unwrap_or(1.0).clamp(0.0, 1.0);
        let gain = embodiment
            .effector_gain(EffectorCapability::Translation)
            .clamp(0.0, 1.0);
        self.time_metric = if world.terrain.is_some() {
            TranslationTimeMetric::GroundPlane
        } else {
            TranslationTimeMetric::Spatial
        };
        self.capacity = capacity;
        self.gain = gain;
        let mut budget = TranslationIntervalBudget::new(MOVE_STEP * physical_capacity)?;
        self.withdrawal = world.apply_contact_withdrawal(organism, entity, capacity, gain, 0.0)?;
        if let Some(reflex) = &self.withdrawal {
            self.reflex_time = 1.0;
            // Even a blocked protective attempt owns this interval. A voluntary
            // translation cannot undo the response or receive a second allowance.
            budget.consume(
                MOVE_STEP * physical_capacity * gain,
                1.0,
                self.time_metric.path_length(reflex.displacement),
            )?;
        }
        self.budget = Some(budget);
        Ok(())
    }

    pub(super) fn swept_contact_plan(
        &self,
        world: &HeadlessWorld,
        command: &ActionCommand,
        own: WorldEntityId,
        start: Vec3f,
        end: Vec3f,
    ) -> Option<SweptContactPlan> {
        let speed = MOVE_STEP * command.intensity.raw();
        let delta = subtract(end, start);
        if speed <= 0.0 || self.time_metric.path_length(delta) <= 0.0 {
            return None;
        }
        let contacts = world
            .objects
            .values()
            .filter(|o| {
                o.id != own
                    && !o.consumed
                    && o.carried_by != Some(command.organism_id)
                    && o.kind == WorldObjectKind::Hazard
                    && o.hazard_pain > 0.0
            })
            .filter_map(|o| {
                motion_sphere_entry(start, end, o.position, o.radius).map(|t| (o.id, t))
            })
            .collect::<Vec<_>>();
        let earliest = contacts.iter().map(|(_, t)| *t).min_by(f64::total_cmp)?;
        // Resolve the f64 sphere boundary onto f32 physical positions. The
        // contact kernel admits only actual surfaces within its tiny tolerance.
        let fraction = earliest as f32;
        let position = add(
            start,
            Vec3f::new(delta.x * fraction, delta.y * fraction, delta.z * fraction),
        );
        let sources = contacts
            .into_iter()
            .filter(|(id, _)| {
                let object = &world.objects[&id.raw()];
                distance(object.position, position) <= object.radius + 1e-5
            })
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        if sources.is_empty() {
            return None;
        }
        let duration = (self.time_metric.path_length(delta) * fraction / speed).clamp(0.0, 1.0);
        Some(SweptContactPlan {
            position,
            sources,
            duration,
        })
    }

    pub(super) fn finish_swept_contact(
        &mut self,
        world: &mut HeadlessWorld,
        command: &ActionCommand,
        own: WorldEntityId,
        result: &mut HeadlessActionResult,
        plan: SweptContactPlan,
    ) -> Result<(), ScaffoldContractError> {
        let displacement = result.execution.physical.displacement;
        let budget = self
            .budget
            .as_mut()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        budget.consume(
            MOVE_STEP * command.intensity.raw(),
            plan.duration,
            self.time_metric.path_length(displacement),
        )?;
        // This candidate allocates the existing full-interval effort profile by
        // actual elapsed time. Injury dose remains physical and is never scaled.
        result.body_event.energy *= plan.duration;
        result.execution.physical.energy_cost =
            NormalizedScalar::new(result.execution.physical.energy_cost.raw() * plan.duration)?;
        result.observation.energy_delta = SignedValence::new(result.body_event.energy)?;
        result.observation.homeostatic_delta.drives.brain_atp *= plan.duration;
        self.voluntary_time = plan.duration;
        self.voluntary_path += distance(Vec3f::ZERO, displacement);
        self.voluntary_translation_energy += result.body_event.energy;
        self.thermal_peak = world.thermal_contact_severity_at(own, plan.position);
        let remaining = 1.0 - budget.spent_time;
        self.reflex_time = remaining;
        self.withdrawal = world.apply_contact_withdrawal_for_interval(
            command.organism_id,
            own,
            self.capacity,
            self.gain,
            0.0,
            remaining,
            Some(&plan.sources),
        )?;
        if let Some(reflex) = &self.withdrawal {
            budget.consume(
                MOVE_STEP * self.capacity.unwrap_or(1.0).clamp(0.0, 1.0) * self.gain,
                remaining,
                self.time_metric.path_length(reflex.displacement),
            )?;
        }
        Ok(())
    }

    pub(super) fn finish_late_response(
        &mut self,
        world: &mut HeadlessWorld,
        organism: OrganismId,
        own: WorldEntityId,
        acute_damage: f32,
        voluntary_displacement: Vec3f,
    ) -> Result<(), ScaffoldContractError> {
        if self.withdrawal.is_some() {
            return Ok(());
        }
        let budget = self
            .budget
            .as_mut()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let remaining = 1.0 - budget.spent_time;
        let carried = world.carried_interval_positions(organism, None);
        self.withdrawal = world.apply_contact_withdrawal_for_interval(
            organism,
            own,
            self.capacity,
            self.gain,
            acute_damage,
            remaining,
            None,
        )?;
        if let Some(reflex) = &self.withdrawal {
            self.reflex_time = remaining;
            budget.consume(
                MOVE_STEP * self.capacity.unwrap_or(1.0).clamp(0.0, 1.0) * self.gain,
                remaining,
                self.time_metric.path_length(reflex.displacement),
            )?;
            world.commit_current_interval_velocity(
                organism,
                add(voluntary_displacement, reflex.displacement),
            )?;
            world.commit_carried_interval_velocity(organism, &carried);
        }
        Ok(())
    }

    pub(super) fn observe_voluntary(
        &mut self,
        command: &ActionCommand,
        result: &HeadlessActionResult,
    ) -> Result<(), ScaffoldContractError> {
        if self.withdrawal.is_some()
            || !matches!(
                classify_action(command),
                HeadlessAction::StepForward
                    | HeadlessAction::Move
                    | HeadlessAction::Approach
                    | HeadlessAction::Flee
            )
        {
            return Ok(());
        }
        let displacement = result.execution.physical.displacement;
        let path = self.time_metric.path_length(displacement);
        let speed = MOVE_STEP * command.intensity.raw();
        let time = if !result.execution.succeeded {
            1.0
        } else if speed > 0.0 {
            (path / speed).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.budget
            .as_mut()
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?
            .consume(speed, time, path)?;
        self.voluntary_time = time;
        self.voluntary_path += distance(Vec3f::ZERO, displacement);
        self.voluntary_translation_energy += result.body_event.energy;
        Ok(())
    }

    pub(super) fn preempts(&self, command: &ActionCommand) -> bool {
        self.withdrawal.is_some()
            && matches!(
                classify_action(command),
                HeadlessAction::StepForward
                    | HeadlessAction::Move
                    | HeadlessAction::Approach
                    | HeadlessAction::Flee
            )
    }
}

impl HeadlessWorld {
    pub(super) fn execute_awake_translation(
        &mut self,
        command: &ActionCommand,
        state: &mut AwakeTranslationState,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        let own = self.agent_entity_id(command.organism_id)?;
        let positions = self.carried_interval_positions(command.organism_id, None);
        let intent = match classify_action(command) {
            HeadlessAction::StepForward => MoveIntent::Forward,
            HeadlessAction::Move => MoveIntent::Absolute,
            HeadlessAction::Approach => MoveIntent::Approach,
            HeadlessAction::Flee => MoveIntent::Flee,
            _ => return Err(ScaffoldContractError::InvalidActionDecision),
        };
        let result = self.execute_move_with_interval(*command, own, intent, Some(state))?;
        let actual = state
            .withdrawal
            .as_ref()
            .map_or(result.execution.physical.displacement, |r| {
                add(result.execution.physical.displacement, r.displacement)
            });
        self.commit_current_interval_velocity(command.organism_id, actual)?;
        self.commit_carried_interval_velocity(command.organism_id, &positions);
        Ok(result)
    }

    /// Candidate initial/swept contact and fresh late-action damage priority.
    /// Ordinary sealed CPU/GPU credit is not integrated by this world API.
    /// Existing ordinary APIs and physical defaults are unchanged.
    pub fn apply_registered_motor_bundle_with_shared_contact_priority(
        &mut self,
        bundle: &MotorCommandBundle,
        entity: WorldEntityId,
    ) -> Result<AwakeTranslationIntervalReceipt, HeadlessMotorTransactionError> {
        self.apply_shared_contact_bundle(bundle, entity, None, None)
    }

    /// Explicit staged-host variant preserving ordinary neural emissions and
    /// perceived novelty. The enclosing host owns its whole-state commit.
    pub fn apply_registered_motor_bundle_with_shared_contact_priority_and_perceived_novelty(
        &mut self,
        bundle: &MotorCommandBundle,
        entity: WorldEntityId,
        neural: &NeuralEmissionFrame,
        novelty: f32,
        object_novelty: &[(WorldEntityId, f32)],
    ) -> Result<AwakeTranslationIntervalReceipt, HeadlessMotorTransactionError> {
        if !novelty.is_finite()
            || !(0.0..=1.0).contains(&novelty)
            || object_novelty.len() > bundle.channels.len()
            || object_novelty.iter().any(|(id, value)| {
                id.raw() == 0 || !value.is_finite() || !(0.0..=1.0).contains(value)
            })
        {
            return Err(ScaffoldContractError::ScalarOutOfRange.into());
        }
        self.apply_shared_contact_bundle(
            bundle,
            entity,
            Some(neural),
            Some((novelty, object_novelty)),
        )
    }

    fn apply_shared_contact_bundle(
        &mut self,
        bundle: &MotorCommandBundle,
        entity: WorldEntityId,
        neural: Option<&NeuralEmissionFrame>,
        novelty: Option<(f32, &[(WorldEntityId, f32)])>,
    ) -> Result<AwakeTranslationIntervalReceipt, HeadlessMotorTransactionError> {
        let before = self.clone();
        let mut state = AwakeTranslationState::default();
        let result = self.apply_registered_motor_bundle_inner(
            bundle,
            entity,
            neural,
            novelty,
            Some(&mut state),
        );
        match result {
            Err(error) => {
                *self = before;
                Err(error)
            }
            Ok(voluntary) => {
                let reflex_displacement = state
                    .withdrawal
                    .as_ref()
                    .map_or(Vec3f::ZERO, |r| r.displacement);
                Ok(AwakeTranslationIntervalReceipt {
                    actual_displacement: add(
                        voluntary.joint.execution.displacement,
                        reflex_displacement,
                    ),
                    voluntary,
                    reflex_budget_exhausted: state.withdrawal.is_some() && state.reflex_time == 0.0,
                    withdrawal: state.withdrawal,
                    voluntary_time_allocated: state.voluntary_time,
                    reflex_time_allocated: state.reflex_time,
                    allocated_translation_time: state.budget.map_or(0.0, |b| b.spent_time),
                    translation_time_metric: state.time_metric,
                    reflex_path: distance(Vec3f::ZERO, reflex_displacement),
                    voluntary_path: state.voluntary_path,
                    voluntary_translation_energy: state.voluntary_translation_energy,
                })
            }
        }
    }
}
