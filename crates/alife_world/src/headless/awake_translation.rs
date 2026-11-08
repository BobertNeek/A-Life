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

/// Ephemeral provenance for actual voluntary actuator time, never inferred
/// from rounded final world-coordinate subtraction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VoluntaryTimeProvenance {
    #[default]
    NoTranslation,
    InitialPreemption,
    FullInterval,
    GoalArrival,
    ZeroSpeed,
    BlockedAttempt,
    SweptContact,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PlannedTranslationTime {
    pub(super) duration: f32,
    provenance: VoluntaryTimeProvenance,
}
impl PlannedTranslationTime {
    pub(super) fn full(speed: f32) -> Self {
        if speed > 0.0 {
            Self {
                duration: 1.0,
                provenance: VoluntaryTimeProvenance::FullInterval,
            }
        } else {
            Self {
                duration: 0.0,
                provenance: VoluntaryTimeProvenance::ZeroSpeed,
            }
        }
    }
    pub(super) fn toward(relative: Vec3f, speed: f32) -> Self {
        if speed <= 0.0 {
            return Self::full(speed);
        }
        let length = distance(Vec3f::ZERO, relative);
        if length >= speed {
            Self::full(speed)
        } else {
            Self {
                duration: length / speed,
                provenance: VoluntaryTimeProvenance::GoalArrival,
            }
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
    pub planned_voluntary_time: f32,
    pub voluntary_time_provenance: VoluntaryTimeProvenance,
    pub reflex_time_allocated: f32,
    pub reflex_budget_exhausted: bool,
    /// Only a late zero-time/effort/motion response with the same normalized
    /// body event as ordinary execution qualifies. Initial/swept responses do not.
    pub exhausted_late_no_effect: bool,
    /// Monotonic: candidate-only preemption, clipping or rejection occurred.
    /// This remains true even if no withdrawal could be executed.
    pub voluntary_execution_altered: bool,
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
    planned_time: Option<PlannedTranslationTime>,
    voluntary_time_provenance: VoluntaryTimeProvenance,
    reflex_time: f32,
    late_response: bool,
    exhausted_late_no_effect: bool,
    voluntary_execution_altered: bool,
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
            self.voluntary_execution_altered = true;
            self.voluntary_time_provenance = VoluntaryTimeProvenance::InitialPreemption;
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
    ) -> Result<Option<SweptContactPlan>, ScaffoldContractError> {
        let Some(terrain) = &world.terrain else {
            return Ok(self.linear_swept_contact_plan(world, command, own, start, end));
        };
        // Existing old/editor poses can start above or below sampled ground.
        // Preserve their exact legacy route; do not invent zero-time grounding.
        if terrain.surface().height(start.x, start.z) != Some(start.y) {
            return Ok(self.linear_swept_contact_plan(world, command, own, start, end));
        }
        let segments = terrain
            .grounded_move_segments(start, end)
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        // Preserve the existing flat route's exact scalar arithmetic.
        if segments.iter().all(|segment| {
            segment.start[1] == f64::from(start.y) && segment.end[1] == f64::from(start.y)
        }) {
            return Ok(self.linear_swept_contact_plan(world, command, own, start, end));
        }
        let hazards = world
            .objects
            .values()
            .filter(|o| {
                o.id != own
                    && !o.consumed
                    && o.carried_by != Some(command.organism_id)
                    && o.kind == WorldObjectKind::Hazard
                    && o.hazard_pain > 0.0
            })
            .collect::<Vec<_>>();
        let mut contacts = Vec::new();
        for segment in &segments {
            for hazard in &hazards {
                if let Some(local) = super::grounded_path::grounded_segment_sphere_entry(
                    segment.start,
                    segment.end,
                    hazard.position,
                    hazard.radius,
                ) {
                    contacts.push((
                        segment.from + (segment.to - segment.from) * local,
                        hazard.id,
                    ));
                }
            }
        }
        contacts.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.raw().cmp(&b.1.raw())));
        let Some((fraction, _)) = contacts.first() else {
            return Ok(None);
        };
        let geometric_fraction = (*fraction).clamp(0.0, 1.0);
        let planned = self
            .planned_time
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?
            .duration;
        let speed = MOVE_STEP * command.intensity.raw();
        let point_at = |fraction: f64| {
            Vec3f::new(
                (f64::from(start.x) + (f64::from(end.x) - f64::from(start.x)) * fraction) as f32,
                start.y,
                (f64::from(start.z) + (f64::from(end.z) - f64::from(start.z)) * fraction) as f32,
            )
        };
        let first = point_at(geometric_fraction);
        let mut fractions = vec![geometric_fraction];
        // A finite stencil of forward f32-coordinate centers on this SAME plan.
        // Each axis contributes its next eight representable values. This avoids
        // starving a coarse translated axis behind thousands of fine-axis ULPs.
        // No spatial nudge toward a source and no endpoint/time allowance added.
        for (from, to, coordinate) in [(start.x, end.x, first.x), (start.z, end.z, first.z)] {
            let delta = f64::from(to) - f64::from(from);
            if delta == 0.0 {
                continue;
            }
            let mut next = coordinate;
            for _ in 0..8 {
                next = if delta > 0.0 {
                    next.next_up()
                } else {
                    next.next_down()
                };
                let progress = (f64::from(next) - f64::from(from)) / delta;
                if progress.is_finite() && progress > geometric_fraction && progress <= 1.0 {
                    fractions.push(progress);
                }
            }
        }
        fractions.sort_by(f64::total_cmp);
        fractions.dedup();
        for fraction in fractions {
            let proposed = point_at(fraction);
            let position = terrain
                .resolve_move(start, proposed)
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            // The represented full endpoint always consumes the full planned
            // interval, never a residue inferred from rounded displacement.
            let accepted_fraction = if position == end { 1.0 } else { fraction };
            let duration = if position == end {
                planned
            } else {
                planned * accepted_fraction as f32
            };
            let path = self.time_metric.path_length(subtract(position, start));
            if path > speed * duration + 1e-6 {
                continue;
            } // Exact existing budget guard.
            let sources = hazards
                .iter()
                .filter(|object| {
                    contacts
                        .iter()
                        .any(|(entry, id)| *id == object.id && *entry <= accepted_fraction)
                        && distance(object.position, position) <= object.radius + 1e-5
                })
                .map(|object| object.id)
                .collect::<Vec<_>>();
            if sources.is_empty() {
                continue;
            }
            #[cfg(test)]
            if accepted_fraction > geometric_fraction {
                eprintln!("GROUNDED_REPRESENTED_CONTACT geometric={geometric_fraction} accepted={accepted_fraction} planar={path} time={duration}");
            }
            return Ok(Some(SweptContactPlan {
                position,
                sources,
                duration,
            }));
        }
        // No admitted point in the explicit bounded representable stencil.
        // Do not invent source membership, route motion or extra actuator time.
        Err(ScaffoldContractError::InvalidDecisionEvidence)
    }

    fn linear_swept_contact_plan(
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
        let duration = self.planned_time?.duration * fraction.clamp(0.0, 1.0);
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
        self.voluntary_time_provenance = VoluntaryTimeProvenance::SweptContact;
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
            self.late_response = true;
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

    pub(super) fn set_planned_time(&mut self, time: PlannedTranslationTime) {
        self.planned_time = Some(time);
    }

    pub(super) fn mark_voluntary_execution_altered(&mut self) {
        self.voluntary_execution_altered = true;
    }

    pub(super) fn bind_ordinary_body_equivalence(
        &mut self,
        ordinary: BodyEventDelta,
        actual: BodyEventDelta,
    ) {
        self.exhausted_late_no_effect = self.late_response
            && !self.voluntary_execution_altered
            && self.reflex_time == 0.0
            && self.withdrawal.as_ref().is_some_and(|r| {
                r.blocked && r.displacement == Vec3f::ZERO && r.energy_cost == 0.0
            })
            && ordinary == actual
            // Only these fields can change above the normalization boundary.
            // Do not call signed-zero changes byte-exact equivalence.
            && ordinary.energy.to_bits() == actual.energy.to_bits()
            && ordinary.damage.to_bits() == actual.damage.to_bits()
            && ordinary.sleep_recovery.to_bits() == actual.sleep_recovery.to_bits();
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
            self.voluntary_time_provenance = VoluntaryTimeProvenance::BlockedAttempt;
            1.0
        } else {
            let planned = self
                .planned_time
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            self.voluntary_time_provenance = planned.provenance;
            planned.duration
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
                    exhausted_late_no_effect: state.exhausted_late_no_effect,
                    voluntary_execution_altered: state.voluntary_execution_altered,
                    withdrawal: state.withdrawal,
                    voluntary_time_allocated: state.voluntary_time,
                    planned_voluntary_time: state.planned_time.map_or(0.0, |p| p.duration),
                    voluntary_time_provenance: state.voluntary_time_provenance,
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
