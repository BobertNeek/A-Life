//! Physical hand preparation for the exact selected target; no activity preference.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionPhase {
    Attempt,
    Preparation,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GripTransferKind {
    Acquired,
    Released,
    Placed,
    Consumed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GripTransfer {
    pub object: WorldEntityId,
    pub previous_owner: Option<OrganismId>,
    pub new_owner: Option<OrganismId>,
    pub previous_position: Vec3f,
    pub new_position: Vec3f,
    pub kind: GripTransferKind,
}

impl HeadlessWorld {
    pub(super) fn activity_grip_available(
        &self,
        organism: OrganismId,
        target: WorldEntityId,
    ) -> bool {
        let mut held = self
            .objects
            .values()
            .filter(|object| !object.consumed && object.carried_by == Some(organism));
        let Some(object) = held.next() else {
            return true;
        };
        // Provenance is historical evidence, not a veto on physical release.
        held.next().is_none()
            && (object.id == target
                || self
                    .hand_placement(organism, object.id, Some(target))
                    .is_some())
    }

    /// Acquisition may place a previous hold in the same atomic preparation.
    /// Actual ingestion/activation still requires a later selected activity.
    pub(super) fn prepare_hands_on(
        &mut self,
        command: ActionCommand,
        agent: WorldEntityId,
        target: WorldEntityId,
        eating: bool,
    ) -> Result<Option<HeadlessActionResult>, ScaffoldContractError> {
        let object = &self.objects[&target.raw()];
        let mobile = object.kind != WorldObjectKind::ActivityToy;
        let already_held = object.carried_by == Some(command.organism_id);
        let valid_kind = if eating {
            object.kind == WorldObjectKind::Food
        } else {
            matches!(
                object.kind,
                WorldObjectKind::Ball | WorldObjectKind::ActivityToy
            )
        };
        let effector = self
            .organism_registry
            .get(command.organism_id)
            .is_none_or(|record| {
                record
                    .embodiment()
                    .effector_gain(EffectorCapability::Manipulation)
                    > 0.0
            });
        let valid = valid_kind
            && effector
            && !self.objects[&agent.raw()].consumed
            && self
                .organism_registry
                .get(command.organism_id)
                .is_none_or(|record| record.lifecycle().is_alive())
            && !object.consumed
            && self.player_held_object() != Some(target)
            && object
                .carried_by
                .is_none_or(|owner| owner == command.organism_id)
            && self
                .physical_contact_reachable(self.objects[&agent.raw()].position, object.position)
            // Validate physical capability, not hidden desirability. Ingesting
            // harmful food retains its ordinary measured consequences.
            && (!mobile || self.grasp_target_is_reachable(command.organism_id, agent, target));
        if !valid {
            return self.hand_preparation_failure(command, target).map(Some);
        }
        let (held, multiple_held) = {
            let mut held = self.objects.values().filter(|object| {
                !object.consumed && object.carried_by == Some(command.organism_id)
            });
            (held.next().map(|object| object.id), held.next().is_some())
        };
        if multiple_held {
            return self.hand_preparation_failure(command, target).map(Some);
        }
        if mobile && !already_held {
            // The selected Eat/Play identity survives the compound transition.
            let mut result = self.execute_grab(command, agent)?;
            if result.execution.succeeded {
                self.objects
                    .get_mut(&target.raw())
                    .expect("acquired target exists")
                    .activity_hold = true;
                result.phase = ActionPhase::Preparation;
            }
            return Ok(Some(result));
        }
        if let Some(held) = held.filter(|held| *held != target) {
            let Some(position) = self.hand_placement(command.organism_id, held, None) else {
                return self.hand_preparation_failure(command, target).map(Some);
            };
            let mut result =
                self.commit_hand_transition(command, agent, target, Some((held, position)), false)?;
            result.phase = ActionPhase::Preparation;
            return Ok(Some(result));
        }
        Ok(None)
    }

    fn hand_preparation_failure(
        &mut self,
        command: ActionCommand,
        target: WorldEntityId,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        self.finish_action(
            command,
            false,
            Some(ReferenceActionFailure::MissingAffordance),
            physical(PhysicalContactKind::None, Some(target), Vec3f::ZERO, 0.06)?,
            OutcomeProfile::missing_affordance(),
            Vec::new(),
        )
    }

    /// Stage at most two item records. All fallible receipt work completes
    /// before publishing either item; the registered transaction additionally
    /// covers biology, clocks and final joint-receipt construction.
    pub(super) fn commit_hand_transition(
        &mut self,
        command: ActionCommand,
        agent: WorldEntityId,
        target: WorldEntityId,
        placement: Option<(WorldEntityId, Vec3f)>,
        acquire: bool,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        let actor = self
            .objects
            .get(&agent.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        if actor.organism_id != Some(command.organism_id) {
            return Err(ScaffoldContractError::InvalidId);
        }
        let grip = Self::carry_grip(actor.position, actor.body_yaw);
        grip.validate()?;
        let mut staged = Vec::with_capacity(2);
        let mut transfers = Vec::with_capacity(2);
        if let Some((held, position)) = placement {
            position.validate()?;
            let mut object = self
                .objects
                .get(&held.raw())
                .ok_or(ScaffoldContractError::InvalidId)?
                .clone();
            transfers.push(GripTransfer {
                object: held,
                previous_owner: object.carried_by,
                new_owner: None,
                previous_position: object.position,
                new_position: position,
                kind: GripTransferKind::Placed,
            });
            object.grounded_physical.velocity = subtract(position, object.position);
            object.position = position;
            object.carried_by = None;
            object.activity_hold = false;
            staged.push(object);
        }
        if acquire {
            let mut object = self
                .objects
                .get(&target.raw())
                .ok_or(ScaffoldContractError::InvalidId)?
                .clone();
            transfers.push(GripTransfer {
                object: target,
                previous_owner: object.carried_by,
                new_owner: Some(command.organism_id),
                previous_position: object.position,
                new_position: grip,
                kind: GripTransferKind::Acquired,
            });
            object.grounded_physical.velocity = subtract(grip, object.position);
            object.position = grip;
            object.carried_by = Some(command.organism_id);
            object.activity_hold = false;
            staged.push(object);
        }
        let touched = staged.iter().map(|object| object.id).collect();
        let contact_target = if acquire {
            target
        } else {
            placement
                .map(|(held, _)| held)
                .ok_or(ScaffoldContractError::InvalidId)?
        };
        let mut result = self.finish_action(
            command,
            true,
            None,
            physical(
                PhysicalContactKind::Touch,
                Some(contact_target),
                Vec3f::ZERO,
                0.06,
            )?,
            OutcomeProfile::grab(),
            touched,
        )?;
        // No fallible work after finish_action's final velocity commit.
        for object in staged {
            self.toy_rolls.remove(&object.id.raw());
            self.objects.insert(object.id.raw(), object);
        }
        result.grip_transfers = transfers;
        Ok(result)
    }

    pub(super) fn hand_placement(
        &self,
        organism: OrganismId,
        held: WorldEntityId,
        acquiring: Option<WorldEntityId>,
    ) -> Option<Vec3f> {
        let agent = self.agent_for(organism).ok()?;
        let object = self.objects.get(&held.raw())?;
        if object.carried_by != Some(organism)
            || object.consumed
            || self.player_held_object() == Some(held)
            || self.player_held_object() == Some(agent.id)
        {
            return None;
        }
        let body_radius = self.locomotion_body_radius();
        let reach = (body_radius + object.radius + 0.05).clamp(CARRY_GRIP_DISTANCE, EAT_RADIUS);
        let ring = [0.0_f32, 45.0, -45.0, 90.0, -90.0, 135.0, -135.0, 180.0]
            .into_iter()
            .map(|degrees| {
                let yaw = agent.body_yaw + degrees.to_radians();
                add(
                    agent.position,
                    Vec3f::new(yaw.cos() * reach, 0.0, yaw.sin() * reach),
                )
            });
        // The ground anchor beneath the actor is a valid last local candidate.
        // Contact radii are not mesh volumes: ordinary carry already overlaps
        // the carrier, and its newly carried target vacates its old ground slot.
        ring.chain(std::iter::once(agent.position))
            .find_map(|position| {
                self.supported_hand_placement(agent.id, object, acquiring, position)
            })
    }

    fn supported_hand_placement(
        &self,
        agent: WorldEntityId,
        object: &WorldObject,
        acquiring: Option<WorldEntityId>,
        mut position: Vec3f,
    ) -> Option<Vec3f> {
        position.validate().ok()?;
        if let Some(terrain) = &self.terrain {
            let surface = terrain.surface();
            // Items use grounded anchors, including underwater terrain, as in
            // editor_move_object. They do not walk: wading/slope limits must not
            // silently encode a safe-food preference. No buoyancy is invented.
            let limits = crate::LocomotionLimits {
                body_radius: object.radius,
                max_wading_depth: f32::MAX,
                max_slope: f32::MAX,
            };
            // The held anchor may legitimately extend outside the terrain.
            // Only the destination needs ground support; the airborne path is
            // checked separately against actual solids with the item's radius.
            position = surface.resolve_move_with(position, position, limits)?;
            // Center support alone does not prove an arbitrary item's footprint
            // fits the map. The radius also governs terrain obstacle clearance.
            for (x, z) in [
                (position.x - object.radius, position.z),
                (position.x + object.radius, position.z),
                (position.x, position.z - object.radius),
                (position.x, position.z + object.radius),
            ] {
                surface.height(x, z)?;
            }
        }
        if distance(self.objects[&agent.raw()].position, position) > EAT_RADIUS
            || !self.solid_segment_clear(self.objects[&agent.raw()].position, position)
            || !self.hand_solid_sweep_clear(object, position)
        {
            return None;
        }
        // Keep the existing conservative spacing for unrelated items/peers.
        // This is placement policy, not a new universal hard-sphere model:
        // the carrier and atomically acquired target are the only exceptions.
        if self.objects.values().any(|other| {
            other.id != object.id
                && other.id != agent
                && Some(other.id) != acquiring
                && !other.consumed
                && other.kind != WorldObjectKind::Obstacle
                && distance(other.position, position) <= other.radius + object.radius
        }) {
            return None;
        }
        Some(position)
    }

    fn hand_solid_sweep_clear(&self, object: &WorldObject, position: Vec3f) -> bool {
        let travel = subtract(position, object.position);
        let length = distance(position, object.position);
        if !length.is_finite() || !object.radius.is_finite() || object.radius <= 0.0 {
            return false;
        }
        let direction = if length > f32::EPSILON {
            scale(travel, length.recip())
        } else {
            Vec3f::ZERO
        };
        if self.terrain.as_ref().is_some_and(|terrain| {
            terrain.surface().obstacles.iter().any(|bounds| {
                let expanded = [
                    bounds[0] - object.radius,
                    bounds[1] - object.radius,
                    bounds[2] + object.radius,
                    bounds[3] + object.radius,
                    bounds[4] - object.radius,
                    bounds[5] + object.radius,
                ];
                sight_box_hit(object.position, direction, expanded, length).is_some()
            })
        }) {
            return false;
        }
        self.objects.values().all(|other| {
            other.kind != WorldObjectKind::Obstacle
                || other.consumed
                || (distance(other.position, position)
                    > other.radius.max(HEADLESS_CONTACT_RADIUS) + object.radius
                    && motion_sphere_entry(
                        object.position,
                        position,
                        other.position,
                        other.radius.max(HEADLESS_CONTACT_RADIUS) + object.radius,
                    )
                    .is_none())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActionLegalityChecker;
    use alife_core::{ActionTarget, DurationTicks};

    const ORGANISM: OrganismId = OrganismId(1);

    fn world() -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
        let mut world = HeadlessScenarioBuilder::new(191_019)
            .agent("actor", ORGANISM, Vec3f::ZERO)
            .food("meal", Vec3f::new(1.1, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        let food = world.entity_id("meal").unwrap();
        let ball = world
            .spawn_toy("ball", Vec3f::new(0.7, 0.0, 0.0), true)
            .unwrap();
        (world, ball, food)
    }

    fn command(action: ActionId, target: WorldEntityId) -> ActionCommand {
        ActionCommand::structured(
            ORGANISM,
            action,
            if action == ActionKind::Inspect.canonical_id() {
                ActionKind::Inspect
            } else {
                ActionKind::Interact
            },
            ActionTarget::new(Some(target), None),
            Intensity::new(1.0).unwrap(),
            DurationTicks::new(1),
            Confidence::new(1.0).unwrap(),
            0,
            None,
            None,
            None,
        )
        .unwrap()
    }

    fn assert_no_activity(result: &HeadlessActionResult) {
        assert_eq!(result.body_event.nutrition, 0.0);
        assert_eq!(result.body_event.play_stimulation, 0.0);
        assert_eq!(result.body_event.investigation, 0.0);
    }

    fn material_objects(world: &HeadlessWorld) -> Vec<WorldObject> {
        let mut objects = world.object_snapshots();
        // A new stationary interval clears previous velocity. Failure must
        // preserve every material/ownership field, not stale interval motion.
        for object in &mut objects {
            object.grounded_physical.velocity = Vec3f::ZERO;
        }
        objects
    }

    fn roundtrip(world: &HeadlessWorld) -> HeadlessWorld {
        let save = crate::PortableSaveFile::from_headless_world(
            "local-hands-regression",
            world,
            crate::RuntimeConfig::deterministic_default(
                world.seed(),
                alife_core::BrainScaleTier::Nano512,
            ),
            crate::AssetManifest::empty(),
            Vec::new(),
        )
        .unwrap();
        let json = serde_json::to_string(&save).unwrap();
        let save: crate::PortableSaveFile = serde_json::from_str(&json).unwrap();
        save.restore_headless_world().unwrap()
    }

    #[test]
    fn play_acquires_mobile_ball_before_actual_activation() {
        let (mut world, ball, _) = world();
        let selected = command(HeadlessActionIds::PLAY, ball);
        let pickup = world.apply_command(&selected).unwrap();
        assert_eq!(pickup.command, selected);
        assert_eq!(pickup.phase, ActionPhase::Preparation);
        assert_no_activity(&pickup);
        assert!(pickup.execution.succeeded);
        assert_eq!(world.entity(ball).unwrap().carried_by, Some(ORGANISM));
        assert!(world.entity(ball).unwrap().activity_hold);
        assert_eq!(pickup.grip_transfers.len(), 1);
        assert_eq!(pickup.grip_transfers[0].kind, GripTransferKind::Acquired);
        let play = world.apply_command(&selected).unwrap();
        assert_eq!(play.phase, ActionPhase::Completed);
        assert_eq!(play.body_event.play_stimulation, 1.0);
        assert!(play.grip_transfers.is_empty());
    }

    #[test]
    fn switching_activity_places_and_acquires_atomically_before_really_eating() {
        let (mut world, ball, food) = world();
        world
            .apply_command(&command(HeadlessActionIds::PLAY, ball))
            .unwrap();
        let eat = HeadlessWorldCommand::eat(ORGANISM, food).unwrap();
        assert_eq!(world.check_action(&eat), crate::ActionLegality::Legal);
        let placement = world.apply_command(&eat).unwrap();
        assert_eq!(placement.command, eat);
        assert_eq!(placement.phase, ActionPhase::Preparation);
        assert_no_activity(&placement);
        assert_eq!(placement.grip_transfers[0].kind, GripTransferKind::Placed);
        assert_eq!(placement.grip_transfers[0].object, ball);
        assert_eq!(placement.execution.physical.target_entity, Some(food));
        assert_eq!(placement.grip_transfers.len(), 2);
        assert_eq!(placement.grip_transfers[1].kind, GripTransferKind::Acquired);
        assert_eq!(placement.grip_transfers[1].object, food);
        assert_eq!(world.entity(ball).unwrap().carried_by, None);
        assert!(!world.entity(ball).unwrap().activity_hold);
        assert_eq!(world.entity(food).unwrap().carried_by, Some(ORGANISM));
        assert!(!world.entity(food).unwrap().consumed);
        let meal = world.apply_command(&eat).unwrap();
        assert_eq!(meal.phase, ActionPhase::Completed);
        assert_eq!(
            meal.execution.physical.contact,
            PhysicalContactKind::Consumed
        );
        assert_eq!(meal.body_event.nutrition, 0.6);
        assert_eq!(meal.grip_transfers[0].kind, GripTransferKind::Consumed);
        assert_eq!(world.entity(food).unwrap().carried_by, None);
        assert!(!world.entity(food).unwrap().activity_hold);
        assert!(!world.apply_command(&eat).unwrap().execution.succeeded);
    }

    #[test]
    fn deliberate_carry_swaps_and_capacity_stays_one_after_restore() {
        let (mut world, ball, food) = world();
        world
            .apply_command(&command(HeadlessActionIds::GRAB, ball))
            .unwrap();
        assert!(!world.entity(ball).unwrap().activity_hold);
        for saved in [world.clone(), roundtrip(&world)] {
            for action in [HeadlessActionIds::EAT, HeadlessActionIds::GRAB] {
                let mut candidate = saved.clone();
                let result = candidate.apply_command(&command(action, food)).unwrap();
                assert!(result.execution.succeeded);
                assert_no_activity(&result);
                assert_eq!(result.grip_transfers.len(), 2);
                assert_eq!(candidate.entity(ball).unwrap().carried_by, None);
                assert_eq!(candidate.entity(food).unwrap().carried_by, Some(ORGANISM));
                assert_eq!(
                    candidate
                        .objects
                        .values()
                        .filter(|object| object.carried_by == Some(ORGANISM))
                        .count(),
                    1
                );
            }
        }
        let release = world
            .apply_command(&command(HeadlessActionIds::GRAB, ball))
            .unwrap();
        assert_eq!(release.grip_transfers[0].kind, GripTransferKind::Released);
        assert_eq!(release.grip_transfers.len(), 1);
        assert_eq!(world.entity(ball).unwrap().carried_by, None);
    }

    #[test]
    fn activity_provenance_survives_save_without_vetoing_legacy_swaps() {
        let (mut world, ball, food) = world();
        world
            .apply_command(&command(HeadlessActionIds::PLAY, ball))
            .unwrap();
        let mut restored = roundtrip(&world);
        assert!(restored.entity(ball).unwrap().activity_hold);
        let transition = restored
            .apply_command(&command(HeadlessActionIds::EAT, food))
            .unwrap();
        assert_eq!(transition.grip_transfers[0].kind, GripTransferKind::Placed);
        let object = crate::WorldObjectSaveState::from(world.entity(ball).unwrap().clone());
        let mut json = serde_json::to_value(object).unwrap();
        json.as_object_mut().unwrap().remove("activity_hold");
        let legacy: crate::WorldObjectSaveState = serde_json::from_value(json).unwrap();
        let mut parts = world.save_state();
        *parts
            .objects
            .iter_mut()
            .find(|object| object.id == ball)
            .unwrap() = legacy;
        let mut legacy = HeadlessWorld::from_save_state(&parts).unwrap();
        assert!(
            legacy
                .apply_command(&command(HeadlessActionIds::EAT, food))
                .unwrap()
                .execution
                .succeeded
        );
        assert_eq!(legacy.entity(ball).unwrap().carried_by, None);
        assert_eq!(legacy.entity(food).unwrap().carried_by, Some(ORGANISM));
    }

    #[test]
    fn invalid_requested_target_preserves_deliberate_and_activity_holds() {
        for initial in [HeadlessActionIds::GRAB, HeadlessActionIds::PLAY] {
            for action in [HeadlessActionIds::EAT, HeadlessActionIds::GRAB] {
                for case in ["reach", "other_owner", "player", "consumed", "missing"] {
                    let (mut world, ball, food) = world();
                    world.apply_command(&command(initial, ball)).unwrap();
                    let target = match case {
                        "reach" => {
                            world.objects.get_mut(&food.raw()).unwrap().position.x = 4.0;
                            food
                        }
                        "other_owner" => {
                            world.objects.get_mut(&food.raw()).unwrap().carried_by =
                                Some(OrganismId(2));
                            food
                        }
                        "player" => {
                            world.begin_player_hold(food).unwrap();
                            food
                        }
                        "consumed" => {
                            world.objects.get_mut(&food.raw()).unwrap().consumed = true;
                            food
                        }
                        "missing" => WorldEntityId(u64::MAX),
                        _ => unreachable!(),
                    };
                    let before = material_objects(&world);
                    let result = world.apply_command(&command(action, target)).unwrap();
                    assert!(!result.execution.succeeded, "{case}");
                    assert_no_activity(&result);
                    assert!(result.grip_transfers.is_empty());
                    assert_eq!(material_objects(&world), before, "{case}");
                }
            }
        }
    }

    #[test]
    fn harmful_food_switch_preserves_preparation_and_portable_continuation() {
        let (mut world, ball, food) = world();
        world.objects.get_mut(&food.raw()).unwrap().hazard_pain = 0.3;
        world
            .objects
            .get_mut(&food.raw())
            .unwrap()
            .grounded_physical
            .chemical[2] = -0.65;
        world
            .apply_command(&command(HeadlessActionIds::PLAY, ball))
            .unwrap();
        let selected = command(HeadlessActionIds::EAT, food);
        // A legal target permits an atomic placement/pickup, not a meal.
        let placed = world.apply_command(&selected).unwrap();
        assert!(placed.execution.succeeded);
        assert_eq!(placed.phase, ActionPhase::Preparation);
        assert_eq!(placed.grip_transfers[0].kind, GripTransferKind::Placed);
        assert_eq!(world.entity(ball).unwrap().carried_by, None);
        assert_eq!(world.entity(food).unwrap().carried_by, Some(ORGANISM));
        assert_eq!(placed.grip_transfers.len(), 2);
        assert_eq!(placed.grip_transfers[1].kind, GripTransferKind::Acquired);
        assert_no_activity(&placed);
        world.advance_tick();
        let mut restored = roundtrip(&world);
        assert_eq!(restored.object_snapshots(), world.object_snapshots());
        assert!(restored.entity(food).unwrap().activity_hold);
        let meal = world.apply_command(&selected).unwrap();
        assert_eq!(restored.apply_command(&selected).unwrap(), meal);
        assert!(meal.execution.succeeded);
        assert_eq!(meal.phase, ActionPhase::Completed);
        assert_eq!(meal.body_event.nutrition, 0.6);
        assert_eq!(meal.body_event.damage, 0.3);
        assert_eq!(world.ingestion_observations[&ORGANISM.raw()].taste, -0.65);
        assert_eq!(restored.object_snapshots(), world.object_snapshots());
        assert!(world.entity(food).unwrap().consumed);
        assert_eq!(world.entity(food).unwrap().carried_by, None);
    }

    #[test]
    fn mobile_play_acquisition_uses_the_same_physical_checks_as_explicit_grab() {
        let (mut world, ball, _) = world();
        world.objects.get_mut(&ball.raw()).unwrap().hazard_pain = 0.3;
        let mut explicit = world.clone();
        assert!(
            explicit
                .apply_command(&command(HeadlessActionIds::GRAB, ball))
                .unwrap()
                .execution
                .succeeded
        );
        let pickup = world
            .apply_command(&command(HeadlessActionIds::PLAY, ball))
            .unwrap();
        assert!(pickup.execution.succeeded);
        assert_eq!(pickup.phase, ActionPhase::Preparation);
        assert_no_activity(&pickup);
        assert_eq!(
            world.entity(ball).unwrap().position,
            explicit.entity(ball).unwrap().position
        );
        assert_eq!(world.entity(ball).unwrap().carried_by, Some(ORGANISM));
        world.advance_tick();
        explicit.advance_tick();
        let selected = command(HeadlessActionIds::PLAY, ball);
        let play = world.apply_command(&selected).unwrap();
        assert!(play.execution.succeeded);
        assert_eq!(play.phase, ActionPhase::Completed);
        assert_eq!(
            play.body_event,
            explicit.apply_command(&selected).unwrap().body_event
        );
        assert!(play.body_event.play_stimulation > 0.0);
    }

    #[test]
    fn no_safe_supported_placement_keeps_old_object_without_new_activity() {
        let (mut world, ball, food) = world();
        world
            .apply_command(&command(HeadlessActionIds::PLAY, ball))
            .unwrap();
        let body_radius = world.locomotion_body_radius();
        let item_radius = world.entity(ball).unwrap().radius;
        assert!(item_radius > body_radius);
        // A map exactly as wide as the item radius admits the feet anchor.
        // Instead keep the actor supported while the item footprint cannot fit.
        let half_width = (body_radius + item_radius) * 0.5;
        assert!(half_width > body_radius && half_width < item_radius);
        world.terrain = Some(flat_terrain(half_width, None));
        world.objects.get_mut(&food.raw()).unwrap().position = Vec3f::new(0.0, 0.0, 0.1);
        let actor = world.agent_entity_id(ORGANISM).unwrap();
        assert!(world.terrain.as_ref().unwrap().walkable(0.0, 0.0));
        assert!(world.grasp_target_is_reachable(ORGANISM, actor, food));
        assert_eq!(world.hand_placement(ORGANISM, ball, Some(food)), None);
        let before = material_objects(&world);
        let result = world
            .apply_command(&command(HeadlessActionIds::EAT, food))
            .unwrap();
        assert!(!result.execution.succeeded);
        assert_no_activity(&result);
        assert_eq!(material_objects(&world), before);
    }

    #[test]
    fn fixed_activity_toy_is_used_in_place_after_safe_grip_preparation() {
        let (mut world, ball, _) = world();
        let station = world
            .spawn_toy("station", Vec3f::new(0.0, 0.0, 1.0), false)
            .unwrap();
        let position = world.entity(station).unwrap().position;
        world
            .apply_command(&command(HeadlessActionIds::PLAY, ball))
            .unwrap();
        let selected = command(HeadlessActionIds::PLAY, station);
        let placement = world.apply_command(&selected).unwrap();
        assert_eq!(placement.phase, ActionPhase::Preparation);
        assert_eq!(placement.command.target_entity, Some(station));
        assert_eq!(placement.execution.physical.target_entity, Some(ball));
        assert_eq!(placement.touched_entities, vec![ball]);
        assert_eq!(placement.grip_transfers.len(), 1);
        assert_eq!(placement.grip_transfers[0].object, ball);
        assert_eq!(placement.grip_transfers[0].kind, GripTransferKind::Placed);
        assert_no_activity(&placement);
        let play = world.apply_command(&selected).unwrap();
        assert_eq!(play.body_event.play_stimulation, 1.0);
        assert_eq!(world.entity(station).unwrap().position, position);
        assert_eq!(world.entity(station).unwrap().carried_by, None);
    }

    #[test]
    fn looking_inspecting_and_approaching_never_acquire_or_release() {
        let (mut world, ball, _) = world();
        for action in [
            HeadlessActionIds::LOOK_CENTER,
            ActionKind::Inspect.canonical_id(),
            HeadlessActionIds::APPROACH,
        ] {
            let result = world.apply_command(&command(action, ball)).unwrap();
            assert!(result.grip_transfers.is_empty());
            assert_eq!(world.entity(ball).unwrap().carried_by, None);
        }
    }

    #[test]
    fn explicit_grab_cannot_steal_player_hold() {
        let (mut world, ball, _) = world();
        world.begin_player_hold(ball).unwrap();
        let before = world.object_snapshots();
        assert!(
            !world
                .apply_command(&command(HeadlessActionIds::GRAB, ball))
                .unwrap()
                .execution
                .succeeded
        );
        assert_eq!(world.object_snapshots(), before);
        assert_eq!(world.player_held_object(), Some(ball));
    }

    fn register(world: &mut HeadlessWorld) -> WorldEntityId {
        let agent = world.agent_entity_id(ORGANISM).unwrap();
        let genome = alife_core::CreatureGenome::early_mammal_founder(
            0xE10_3601,
            alife_core::FoundationGeneticIdentity::new(
                10,
                1,
                7,
                alife_core::BrainCapacityClass::N512_ID,
            )
            .unwrap(),
        )
        .unwrap();
        let phenotype = genome.express().unwrap();
        let biology = BiochemistryState::new(&phenotype, Tick::ZERO).unwrap();
        world
            .register_organism_record(
                WorldOrganismRecord::new(ORGANISM, agent, genome, phenotype, biology, Tick::ZERO)
                    .unwrap(),
            )
            .unwrap();
        agent
    }

    #[test]
    fn grip_provenance_binds_world_identity_and_invalid_possession_is_rejected() {
        let (mut world, ball, _) = world();
        register(&mut world);
        world
            .apply_command(&command(HeadlessActionIds::PLAY, ball))
            .unwrap();
        let mut deliberate = world.clone();
        deliberate
            .objects
            .get_mut(&ball.raw())
            .unwrap()
            .activity_hold = false;
        assert_ne!(
            world.canonical_signature_digest().unwrap(),
            deliberate.canonical_signature_digest().unwrap()
        );
        let mut parts = world.save_state();
        let object = parts
            .objects
            .iter_mut()
            .find(|object| object.id == ball)
            .unwrap();
        object.carried_by = None;
        assert!(HeadlessWorld::from_save_state(&parts).is_err());
    }

    #[test]
    fn registered_preparation_receipts_and_rollback_preserve_exact_possession() {
        let (mut world, ball, food) = world();
        let agent = register(&mut world);
        let bundle = |world: &HeadlessWorld, action, target, sequence| {
            MotorCommandBundle::new(
                ORGANISM,
                alife_core::ExperienceSequenceId(sequence),
                world.tick(),
                vec![alife_core::channel_command_for_action(
                    MotorChannel::Manipulation,
                    &command(action, target),
                )
                .unwrap()],
            )
            .unwrap()
        };
        let play = bundle(&world, HeadlessActionIds::PLAY, ball, 1);
        let before = world.canonical_signature_digest().unwrap();
        let mut failed = world.clone();
        failed.inject_post_action_failure();
        assert!(failed.apply_registered_motor_bundle(&play, agent).is_err());
        failed.injected_post_action_failure = false;
        assert_eq!(failed.canonical_signature_digest().unwrap(), before);
        let pickup = world.apply_registered_motor_bundle(&play, agent).unwrap();
        assert_eq!(pickup.channel_receipts[0].phase, ActionPhase::Preparation);
        assert_eq!(pickup.channel_receipts[0].play_stimulation, 0.0);
        assert_eq!(pickup.grip_transfers[0].kind, GripTransferKind::Acquired);
        assert_eq!(
            pickup.grip_transfers,
            pickup.channel_receipts[0].grip_transfers
        );
        assert_eq!(pickup.body_event.investigation, 0.0);
        assert!(world.entity(ball).unwrap().activity_hold);
        world.try_advance_tick().unwrap();
        let play = bundle(&world, HeadlessActionIds::PLAY, ball, 2);
        let played = world.apply_registered_motor_bundle(&play, agent).unwrap();
        assert_eq!(played.channel_receipts[0].phase, ActionPhase::Completed);
        assert!(played.channel_receipts[0].play_stimulation > 0.0);
        world.try_advance_tick().unwrap();
        let eat = bundle(&world, HeadlessActionIds::EAT, food, 3);
        let before = world.canonical_signature_digest().unwrap();
        let mut failed = world.clone();
        failed.inject_post_action_failure();
        assert!(failed.apply_registered_motor_bundle(&eat, agent).is_err());
        failed.injected_post_action_failure = false;
        assert_eq!(failed.canonical_signature_digest().unwrap(), before);
        let placed = world.apply_registered_motor_bundle(&eat, agent).unwrap();
        assert_eq!(
            placed.channel_receipts[0].command.primitive,
            HeadlessActionIds::EAT
        );
        assert_eq!(placed.channel_receipts[0].phase, ActionPhase::Preparation);
        assert_eq!(placed.grip_transfers[0].object, ball);
        assert_eq!(placed.grip_transfers[0].kind, GripTransferKind::Placed);
        assert_eq!(placed.grip_transfers.len(), 2);
        assert_eq!(placed.grip_transfers[1].kind, GripTransferKind::Acquired);
        assert_eq!(placed.grip_transfers[1].object, food);
        assert_eq!(placed.joint.execution.target_entity, Some(food));
        assert_eq!(placed.body_event.nutrition, 0.0);
        assert_eq!(placed.body_event.investigation, 0.0);
    }

    fn flat_terrain(half_width: f32, water: Option<f32>) -> crate::WorldTerrain {
        crate::WorldTerrain::new(
            crate::TerrainData {
                width: 3,
                depth: 3,
                origin_x: -half_width,
                origin_z: -half_width,
                spacing: half_width,
                heights: vec![0.0; 9],
                obstacles: Vec::new(),
                water_level: water,
            },
            crate::LocomotionLimits::default(),
        )
        .unwrap()
    }

    fn feet_world(radius: f32) -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
        let (mut world, held, food) = world();
        world.objects.get_mut(&held.raw()).unwrap().kind = WorldObjectKind::Food;
        world.objects.get_mut(&held.raw()).unwrap().radius = radius;
        world.objects.get_mut(&food.raw()).unwrap().radius = radius;
        world.objects.get_mut(&food.raw()).unwrap().position.x = 0.4;
        world
            .apply_command(&command(HeadlessActionIds::GRAB, held))
            .unwrap();
        // All ring centers overhang the map, but the item at the feet fits.
        world.terrain = Some(flat_terrain(radius + 0.02, None));
        (world, held, food)
    }

    #[test]
    fn food_at_feet_uses_existing_carry_abstraction_at_authored_and_default_radii() {
        for radius in [0.5, HEADLESS_CONTACT_RADIUS] {
            for action in [HeadlessActionIds::EAT, HeadlessActionIds::GRAB] {
                let (mut world, held, food) = feet_world(radius);
                assert_eq!(
                    world.hand_placement(ORGANISM, held, Some(food)),
                    Some(Vec3f::ZERO)
                );
                let before_tick = world.tick();
                let selected = command(action, food);
                assert_eq!(world.check_action(&selected), crate::ActionLegality::Legal);
                let result = world.apply_command(&selected).unwrap();
                assert!(result.execution.succeeded);
                assert_eq!(result.execution.physical.target_entity, Some(food));
                assert_eq!(result.grip_transfers.len(), 2);
                assert_eq!(result.grip_transfers[0].object, held);
                assert_eq!(result.grip_transfers[1].object, food);
                assert_eq!(world.entity(held).unwrap().position, Vec3f::ZERO);
                assert_eq!(world.entity(held).unwrap().carried_by, None);
                assert_eq!(world.entity(food).unwrap().carried_by, Some(ORGANISM));
                assert_eq!(world.tick(), before_tick);
                // One bounded manipulation interval, not two hidden actions.
                assert_eq!(result.execution.physical.energy_cost.raw(), 0.06);
                assert_eq!(result.body_event, OutcomeProfile::grab().body_event);
                assert_no_activity(&result);
                assert_eq!(
                    roundtrip(&world).object_snapshots(),
                    world.object_snapshots()
                );
            }
        }
    }

    #[test]
    fn item_grounding_is_not_actor_wading_or_slope_policy() {
        for water in [Some(0.3), Some(1.0)] {
            let (mut world, held, food) = feet_world(0.5);
            world.terrain = Some(flat_terrain(0.52, water));
            let result = world
                .apply_command(&command(HeadlessActionIds::EAT, food))
                .unwrap();
            assert!(result.execution.succeeded);
            assert_eq!(world.entity(held).unwrap().position, Vec3f::ZERO);
            assert_no_activity(&result);
        }
        let (mut world, held, food) = feet_world(0.5);
        let mut data = flat_terrain(0.52, None).state().data;
        // A supported item anchor can exist on a slope an actor cannot walk.
        data.heights = (0..9).map(|i| ((i % 3) as f32 - 1.0) * 1.04).collect();
        world.terrain =
            Some(crate::WorldTerrain::new(data, crate::LocomotionLimits::default()).unwrap());
        assert!(!world.terrain.as_ref().unwrap().walkable(0.0, 0.0));
        assert_eq!(
            world.hand_placement(ORGANISM, held, Some(food)),
            Some(Vec3f::ZERO)
        );
    }

    #[test]
    fn feet_do_not_waive_item_bounds_unrelated_items_or_peers() {
        for case in ["large", "item", "peer", "player_lift"] {
            let (mut world, held, food) = feet_world(0.5);
            match case {
                "large" => world.objects.get_mut(&held.raw()).unwrap().radius = 0.8,
                "item" | "peer" => {
                    let blocker = world.spawn_toy("unrelated", Vec3f::ZERO, true).unwrap();
                    if case == "peer" {
                        let object = world.objects.get_mut(&blocker.raw()).unwrap();
                        object.kind = WorldObjectKind::Agent;
                        object.organism_id = Some(OrganismId(2));
                    }
                }
                "player_lift" => {
                    let actor = world.agent_entity_id(ORGANISM).unwrap();
                    world.begin_player_hold(actor).unwrap();
                }
                _ => unreachable!(),
            }
            let before = material_objects(&world);
            let result = world
                .apply_command(&command(HeadlessActionIds::EAT, food))
                .unwrap();
            assert!(!result.execution.succeeded, "{case}");
            assert!(result.grip_transfers.is_empty());
            assert_eq!(material_objects(&world), before, "{case}");
        }
    }

    #[test]
    fn feet_reject_thin_solid_obstacles_across_the_item_sweep() {
        let (mut world, held, food) = feet_world(0.5);
        let mut data = flat_terrain(0.52, None).state().data;
        // The contact ray is clear, but the item's horizontal footprint is not.
        data.obstacles = vec![[0.2, 0.3, 0.21, 0.31, -0.1, 2.0]];
        world.terrain =
            Some(crate::WorldTerrain::new(data, crate::LocomotionLimits::default()).unwrap());
        assert!(world.solid_segment_clear(world.entity(held).unwrap().position, Vec3f::ZERO));
        assert_eq!(world.hand_placement(ORGANISM, held, Some(food)), None);
        let before = material_objects(&world);
        let result = world
            .apply_command(&command(HeadlessActionIds::GRAB, food))
            .unwrap();
        assert!(!result.execution.succeeded);
        assert_eq!(material_objects(&world), before);
    }

    #[test]
    fn multiple_held_corruption_blocks_acquisition_but_preserves_same_target_release() {
        let (mut world, held, food) = world();
        world
            .apply_command(&command(HeadlessActionIds::GRAB, held))
            .unwrap();
        let extra = world.spawn_toy("extra", Vec3f::ZERO, true).unwrap();
        world.objects.get_mut(&extra.raw()).unwrap().carried_by = Some(ORGANISM);
        for target in [held, food] {
            for action in [HeadlessActionIds::EAT, HeadlessActionIds::GRAB] {
                if target == held && action == HeadlessActionIds::GRAB {
                    continue;
                }
                let before = material_objects(&world);
                let result = world.apply_command(&command(action, target)).unwrap();
                assert!(!result.execution.succeeded);
                assert!(result.grip_transfers.is_empty());
                assert_eq!(material_objects(&world), before);
            }
        }
        let release = world
            .apply_command(&command(HeadlessActionIds::GRAB, held))
            .unwrap();
        assert!(release.execution.succeeded);
        assert_eq!(release.grip_transfers.len(), 1);
        assert_eq!(release.grip_transfers[0].kind, GripTransferKind::Released);
        assert_eq!(world.entity(held).unwrap().carried_by, None);
        assert_eq!(world.entity(extra).unwrap().carried_by, Some(ORGANISM));
        assert_eq!(world.entity(food).unwrap().carried_by, None);
    }

    #[test]
    fn staged_swap_contract_failure_preserves_poses_velocities_and_diagnostics() {
        let (mut world, held, food) = world();
        world
            .apply_command(&command(HeadlessActionIds::GRAB, held))
            .unwrap();
        let actor = world.agent_entity_id(ORGANISM).unwrap();
        let placement = world.hand_placement(ORGANISM, held, Some(food)).unwrap();
        let before = world.object_snapshots();
        let diagnostic = world.last_action_result.clone();
        let touched = world.last_touched_entities.clone();
        // Target disappearance is encountered after staging the old item.
        let missing = WorldEntityId(u64::MAX);
        assert!(world
            .commit_hand_transition(
                command(HeadlessActionIds::GRAB, missing),
                actor,
                missing,
                Some((held, placement)),
                true
            )
            .is_err());
        assert_eq!(world.object_snapshots(), before);
        assert_eq!(world.last_action_result, diagnostic);
        assert_eq!(world.last_touched_entities, touched);
        // A fallible outcome constructor must run before interval mutation.
        let mut profile = OutcomeProfile::grab();
        profile.frustration = f32::NAN;
        assert!(world
            .finish_action(
                command(HeadlessActionIds::GRAB, food),
                true,
                None,
                physical(PhysicalContactKind::Touch, Some(food), Vec3f::ZERO, 0.06).unwrap(),
                profile,
                vec![held, food]
            )
            .is_err());
        assert_eq!(world.object_snapshots(), before);
        assert_eq!(world.last_action_result, diagnostic);
    }

    #[test]
    fn dead_or_missing_effector_actor_cannot_swap_a_deliberate_hold() {
        for case in ["dead", "effector"] {
            let (mut world, held, food) = world();
            register(&mut world);
            world
                .apply_command(&command(HeadlessActionIds::GRAB, held))
                .unwrap();
            if case == "dead" {
                let tick = world.tick();
                world.organism_registry.mark_dead(ORGANISM, tick).unwrap();
            } else {
                let mut record = world.organism_registry.get(ORGANISM).unwrap().clone();
                let mut value = serde_json::to_value(record.embodiment()).unwrap();
                let ports = value["effectors"].as_array_mut().unwrap();
                let index = ports
                    .iter()
                    .position(|port| port["capability"] == "Manipulation")
                    .unwrap();
                let start: usize = ports[..index]
                    .iter()
                    .map(|port| port["command_lanes"].as_u64().unwrap() as usize)
                    .sum();
                let width = ports[index]["command_lanes"].as_u64().unwrap() as usize;
                ports.remove(index);
                value["effector_controllability"]
                    .as_array_mut()
                    .unwrap()
                    .drain(start..start + width);
                value["revision"] = serde_json::json!(record.embodiment().revision() + 1);
                record
                    .replace_embodiment_state(serde_json::from_value(value).unwrap())
                    .unwrap();
                world.replace_organism_record_exact(record).unwrap();
            }
            for action in [HeadlessActionIds::EAT, HeadlessActionIds::GRAB] {
                let selected = command(action, food);
                let before = material_objects(&world);
                if action == HeadlessActionIds::EAT {
                    assert_eq!(
                        world.check_action(&selected),
                        crate::ActionLegality::BlockedByWorldState
                    );
                }
                let result = world.apply_command(&selected).unwrap();
                assert!(!result.execution.succeeded, "{case}");
                assert_eq!(material_objects(&world), before, "{case}");
            }
        }
    }

    #[test]
    fn joint_locomotion_precedes_exact_target_swap_and_biology_advances_once() {
        let (mut world, held, food) = world();
        let actor = register(&mut world);
        world
            .apply_command(&command(HeadlessActionIds::GRAB, held))
            .unwrap();
        // Require a real move, but stay within one embodiment-adapted interval.
        // The previous 1.4-unit fixture remained unreachable after that move.
        let target_before = Vec3f::new(EAT_RADIUS + MOVE_STEP * 0.1, 0.0, 0.0);
        world.objects.get_mut(&food.raw()).unwrap().position = target_before;
        let actor_before = world.entity(actor).unwrap().position;
        assert!(distance(actor_before, target_before) > EAT_RADIUS);
        assert!(!world.physical_contact_reachable(actor_before, target_before));
        let eat = command(HeadlessActionIds::EAT, food);
        assert_eq!(
            world.check_action(&eat),
            crate::ActionLegality::BlockedByWorldState
        );
        let approach = HeadlessWorldCommand::approach(ORGANISM, food).unwrap();
        let before_tick = world.tick();
        let bundle = MotorCommandBundle::new(
            ORGANISM,
            alife_core::ExperienceSequenceId(1),
            before_tick,
            vec![
                alife_core::channel_command_for_action(MotorChannel::Manipulation, &eat).unwrap(),
                alife_core::channel_command_for_action(MotorChannel::Locomotion, &approach)
                    .unwrap(),
            ],
        )
        .unwrap();
        let before = world.canonical_signature_digest().unwrap();
        let mut failed = world.clone();
        failed.inject_post_action_failure();
        assert!(failed
            .apply_registered_motor_bundle(&bundle, actor)
            .is_err());
        failed.injected_post_action_failure = false;
        assert_eq!(failed.canonical_signature_digest().unwrap(), before);
        let receipt = world.apply_registered_motor_bundle(&bundle, actor).unwrap();
        assert!(receipt.succeeded);
        assert_eq!(world.tick(), before_tick);
        assert_eq!(receipt.outcome_tick.raw(), before_tick.raw() + 1);
        let actor_after = world.entity(actor).unwrap().position;
        assert!(actor_after.x > actor_before.x);
        // Compare against the original loose-target pose, not its new grip pose.
        assert!(distance(actor_after, target_before) < EAT_RADIUS);
        assert!(world.physical_contact_reachable(actor_after, target_before));
        assert_eq!(
            receipt.channel_receipts[0].command.channel,
            MotorChannel::Locomotion
        );
        assert_eq!(
            receipt.channel_receipts[1].command.channel,
            MotorChannel::Manipulation
        );
        let locomotion = &receipt.channel_receipts[0];
        assert_eq!(locomotion.physical.contact, PhysicalContactKind::Moved);
        assert_eq!(
            locomotion.physical.displacement,
            subtract(actor_after, actor_before)
        );
        let manipulation = receipt
            .channel_receipts
            .iter()
            .find(|entry| entry.command.channel == MotorChannel::Manipulation)
            .unwrap();
        assert_eq!(manipulation.phase, ActionPhase::Preparation);
        assert_eq!(manipulation.physical.target_entity, Some(food));
        assert_eq!(manipulation.grip_transfers.len(), 2);
        assert_eq!(manipulation.grip_transfers[1].object, food);
        assert_eq!(
            manipulation.grip_transfers[1].previous_position,
            target_before
        );
        assert_eq!(receipt.grip_transfers, manipulation.grip_transfers);
        assert_eq!(world.entity(held).unwrap().carried_by, None);
        assert_eq!(world.entity(food).unwrap().carried_by, Some(ORGANISM));
        assert_eq!(receipt.body_event.nutrition, 0.0);
        assert_eq!(receipt.body_event.damage, 0.0);
        assert_eq!(
            receipt.biology_after,
            receipt
                .biology_before
                .advance(
                    receipt.outcome_tick,
                    receipt.body_event,
                    world.organism_registry.get(ORGANISM).unwrap().phenotype()
                )
                .unwrap()
        );
    }

    #[test]
    fn supported_feet_accept_an_airborne_grip_outside_the_terrain() {
        let (mut world, held, food) = world();
        for (id, x) in [(held, 0.02), (food, -0.02)] {
            let object = world.objects.get_mut(&id.raw()).unwrap();
            object.kind = WorldObjectKind::Food;
            object.radius = 0.05;
            object.position.x = x;
        }
        world.terrain = Some(flat_terrain(0.1, None));
        // This uses ordinary allowed acquisition, not a manufactured held pose.
        let pickup = world
            .apply_command(&command(HeadlessActionIds::GRAB, held))
            .unwrap();
        assert!(pickup.execution.succeeded);
        let start = world.entity(held).unwrap().position;
        assert!(world
            .terrain
            .as_ref()
            .unwrap()
            .surface()
            .height(start.x, start.z)
            .is_none());
        assert_eq!(
            world.hand_placement(ORGANISM, held, Some(food)),
            Some(Vec3f::ZERO)
        );
        let swap = world
            .apply_command(&command(HeadlessActionIds::EAT, food))
            .unwrap();
        assert!(swap.execution.succeeded);
        assert_eq!(swap.grip_transfers.len(), 2);
        assert_eq!(world.entity(held).unwrap().position, Vec3f::ZERO);
        assert_eq!(world.entity(held).unwrap().carried_by, None);
        assert_eq!(world.entity(food).unwrap().carried_by, Some(ORGANISM));
    }

    #[test]
    fn selected_impossible_grab_reports_failure_instead_of_stopping_the_host() {
        for case in ["reach", "solid"] {
            for route in ["direct", "registered", "bundle", "staged_runtime"] {
                let (mut world, held, food) = world();
                let actor = register(&mut world);
                world
                    .apply_command(&command(HeadlessActionIds::GRAB, held))
                    .unwrap();
                if case == "reach" {
                    world.objects.get_mut(&food.raw()).unwrap().position.x = 4.0;
                } else {
                    let mut data = flat_terrain(2.0, None).state().data;
                    data.obstacles = vec![[0.8, -0.1, 0.85, 0.1, -0.1, 2.0]];
                    world.terrain = Some(
                        crate::WorldTerrain::new(data, crate::LocomotionLimits::default()).unwrap(),
                    );
                }
                let selected = command(HeadlessActionIds::GRAB, food);
                // Keep the legacy advisory classifier. Execution owns physical
                // rejection and must return its ordinary learnable receipt.
                assert_eq!(world.check_action(&selected), crate::ActionLegality::Legal);
                let before = material_objects(&world);
                let bundle = MotorCommandBundle::new(
                    ORGANISM,
                    alife_core::ExperienceSequenceId(1),
                    world.tick(),
                    vec![alife_core::channel_command_for_action(
                        MotorChannel::Manipulation,
                        &selected,
                    )
                    .unwrap()],
                )
                .unwrap();
                let succeeded = match route {
                    "direct" => world.apply_command(&selected).unwrap().execution.succeeded,
                    "registered" => {
                        let outcome_tick = Tick::new(world.tick().raw() + 1);
                        world
                            .apply_registered_command(&selected, actor, outcome_tick)
                            .unwrap()
                            .action_result
                            .execution
                            .succeeded
                    }
                    "bundle" => {
                        world
                            .apply_registered_motor_bundle(&bundle, actor)
                            .unwrap()
                            .succeeded
                    }
                    "staged_runtime" => {
                        let neural = NeuralEmissionFrame::new(world.tick(), 1, Vec::new()).unwrap();
                        // This is the exact public world path used by CpuCanonicalHost.
                        world
                            .apply_registered_motor_bundle_with_perceived_novelty_in_staged_tick(
                                &bundle,
                                actor,
                                &neural,
                                0.0,
                                &[],
                            )
                            .unwrap()
                            .succeeded
                    }
                    _ => unreachable!(),
                };
                assert!(!succeeded, "{case}/{route}");
                let result = world.last_action_result.as_ref().unwrap();
                assert_eq!(result.phase, ActionPhase::Attempt);
                assert_eq!(result.command.target_entity, Some(food));
                assert_eq!(result.observation.frustration_delta.raw(), 1.0);
                assert_eq!(result.body_event.nutrition, 0.0);
                assert!(result.grip_transfers.is_empty());
                assert_eq!(material_objects(&world), before, "{case}/{route}");
            }
        }
    }

    fn raised_midpath_world(
        min_y: f32,
        max_y: f32,
    ) -> (HeadlessWorld, WorldEntityId, WorldEntityId) {
        let (mut world, held, food) = world();
        for (id, x) in [(held, 0.02), (food, -0.02)] {
            let object = world.objects.get_mut(&id.raw()).unwrap();
            object.kind = WorldObjectKind::Food;
            object.radius = 0.05;
            object.position.x = x;
        }
        let mut data = flat_terrain(0.1, None).state().data;
        // Obstacle metadata may extend outside the supporting terrain. The
        // small map permits feet placement only; the held anchor is airborne.
        data.obstacles = vec![[0.24, -0.01, 0.26, 0.01, min_y, max_y]];
        world.terrain =
            Some(crate::WorldTerrain::new(data, crate::LocomotionLimits::default()).unwrap());
        // Preserve existing pickup rules: this is an ordinary allowed pickup,
        // not an injected pose or a change to general carry/locomotion physics.
        let pickup = world
            .apply_command(&command(HeadlessActionIds::GRAB, held))
            .unwrap();
        assert!(pickup.execution.succeeded);
        world
            .apply_command(&HeadlessWorldCommand::rest(ORGANISM).unwrap())
            .unwrap();
        (world, held, food)
    }

    #[test]
    fn raised_midpath_solid_rejects_swap_without_losing_the_previous_hold() {
        for action in [HeadlessActionIds::EAT, HeadlessActionIds::GRAB] {
            for min_y in [0.02, 0.08] {
                let (mut world, held, food) = raised_midpath_world(min_y, min_y + 0.02);
                let start = world.entity(held).unwrap().position;
                assert!(world.solid_segment_clear(start, Vec3f::ZERO));
                assert!(!world
                    .terrain
                    .as_ref()
                    .unwrap()
                    .surface()
                    .obstacle_at(0.0, 0.0, 0.05));
                let clear = min_y > 0.05;
                assert_eq!(
                    world.hand_placement(ORGANISM, held, Some(food)),
                    clear.then_some(Vec3f::ZERO)
                );
                let before = world.object_snapshots();
                let tick = world.tick();
                let result = world.apply_command(&command(action, food)).unwrap();
                assert_eq!(result.execution.succeeded, clear);
                assert_eq!(world.tick(), tick);
                assert_no_activity(&result);
                if clear {
                    assert_eq!(result.grip_transfers.len(), 2);
                    assert_eq!(result.grip_transfers[0].kind, GripTransferKind::Placed);
                    assert_eq!(result.grip_transfers[1].kind, GripTransferKind::Acquired);
                    assert_eq!(world.entity(held).unwrap().position, Vec3f::ZERO);
                    assert_eq!(world.entity(held).unwrap().carried_by, None);
                    assert_eq!(world.entity(food).unwrap().carried_by, Some(ORGANISM));
                } else {
                    assert_eq!(result.phase, ActionPhase::Attempt);
                    assert!(result.grip_transfers.is_empty());
                    assert_eq!(world.object_snapshots(), before);
                    assert_eq!(world.entity(held).unwrap().carried_by, Some(ORGANISM));
                    assert_eq!(world.entity(food).unwrap().carried_by, None);
                }
            }
        }
    }

    #[test]
    fn item_sweep_vertical_radius_boundaries_are_closed_above_and_below() {
        let radius = 0.05_f32;
        for above in [true, false] {
            for clearance in [radius.next_down(), radius, radius.next_up()] {
                let (min_y, max_y) = if above {
                    (clearance, clearance + 0.02)
                } else {
                    (-clearance - 0.02, -clearance)
                };
                let (world, held, _) = raised_midpath_world(min_y, max_y);
                let object = world.entity(held).unwrap();
                assert!(world.solid_segment_clear(object.position, Vec3f::ZERO));
                assert_eq!(
                    world.hand_solid_sweep_clear(object, Vec3f::ZERO),
                    clearance > radius,
                    "above={above}, clearance={clearance:?}"
                );
            }
        }
    }
}
