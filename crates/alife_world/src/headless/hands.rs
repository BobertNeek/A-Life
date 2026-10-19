//! Physical hand preparation. It never chooses an activity or destination.
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
        // Corrupt/multiple possession never authorizes another acquisition.
        held.next().is_none()
            && (object.id == target
                || (object.activity_hold && self.activity_placement(organism, object.id).is_some()))
    }

    /// One acquisition or placement transition per selected activity. The
    /// requested target is validated before any existing possession changes.
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
            && (!mobile
                || (self.grasp_target_is_reachable(command.organism_id, agent, target)
                    && (already_held || object.hazard_pain == 0.0)));
        if !valid || !self.activity_grip_available(command.organism_id, target) {
            return self.hand_preparation_failure(command, target).map(Some);
        }
        let held = self
            .objects
            .values()
            .find(|object| !object.consumed && object.carried_by == Some(command.organism_id))
            .map(|object| object.id);
        if let Some(held) = held.filter(|held| *held != target) {
            let position = self
                .activity_placement(command.organism_id, held)
                .expect("validated activity placement");
            let previous = self.objects[&held.raw()].position;
            let object = self
                .objects
                .get_mut(&held.raw())
                .expect("held object exists");
            object.position = position;
            object.carried_by = None;
            object.activity_hold = false;
            object.grounded_physical.velocity = subtract(position, previous);
            let mut result = self.finish_action(
                command,
                true,
                None,
                physical(PhysicalContactKind::Touch, Some(held), Vec3f::ZERO, 0.06)?,
                OutcomeProfile::grab(),
                vec![held],
            )?;
            result.phase = ActionPhase::Preparation;
            result.grip_transfers.push(GripTransfer {
                object: held,
                previous_owner: Some(command.organism_id),
                new_owner: None,
                previous_position: previous,
                new_position: position,
                kind: GripTransferKind::Placed,
            });
            return Ok(Some(result));
        }
        if mobile && !already_held {
            // Keep the sampled Eat/Play identity. This is not selected Grab and
            // cannot provide explicit-Grab curriculum credit.
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

    fn activity_placement(&self, organism: OrganismId, held: WorldEntityId) -> Option<Vec3f> {
        let agent = self.agent_for(organism).ok()?;
        let object = self.objects.get(&held.raw())?;
        if !object.activity_hold || object.carried_by != Some(organism) {
            return None;
        }
        let body_radius = self
            .terrain
            .as_ref()
            .map_or(crate::LocomotionLimits::default().body_radius, |terrain| {
                terrain.limits().body_radius
            });
        let reach = (body_radius + object.radius + 0.05).clamp(CARRY_GRIP_DISTANCE, EAT_RADIUS);
        // Bounded local physical placement, with no value or task preference.
        for degrees in [0.0_f32, 45.0, -45.0, 90.0, -90.0, 135.0, -135.0, 180.0] {
            let yaw = agent.body_yaw + degrees.to_radians();
            let mut position = add(
                agent.position,
                Vec3f::new(yaw.cos() * reach, 0.0, yaw.sin() * reach),
            );
            if let Some(terrain) = &self.terrain {
                let limits = crate::LocomotionLimits {
                    body_radius: object.radius,
                    max_wading_depth: 0.0,
                    ..terrain.limits()
                };
                let Some(grounded) =
                    terrain
                        .surface()
                        .resolve_move_with(object.position, position, limits)
                else {
                    continue;
                };
                position = grounded;
            }
            if distance(agent.position, position) > EAT_RADIUS
                || (position.x - agent.position.x).hypot(position.z - agent.position.z)
                    < body_radius + object.radius
                || !self.solid_segment_clear(agent.position, position)
                || !self.solid_segment_clear(object.position, position)
            {
                continue;
            }
            let travel = subtract(position, object.position);
            let length = distance(position, object.position);
            if length > f32::EPSILON
                && self.terrain.as_ref().is_some_and(|terrain| {
                    terrain.surface().obstacles.iter().any(|bounds| {
                        let expanded = [
                            bounds[0] - object.radius,
                            bounds[1] - object.radius,
                            bounds[2] + object.radius,
                            bounds[3] + object.radius,
                            bounds[4],
                            bounds[5],
                        ];
                        sight_box_hit(
                            object.position,
                            scale(travel, length.recip()),
                            expanded,
                            length,
                        )
                        .is_some()
                    })
                })
            {
                continue;
            }
            let clear = self.objects.values().all(|other| {
                if other.id == held || other.id == agent.id || other.consumed {
                    return true;
                }
                if other.kind == WorldObjectKind::Obstacle {
                    distance(
                        other.position,
                        closest_point_on_segment(other.position, object.position, position),
                    ) > other.radius.max(HEADLESS_CONTACT_RADIUS) + object.radius
                } else {
                    distance(other.position, position) > other.radius + object.radius
                }
            });
            if clear {
                return Some(position);
            }
        }
        None
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
    fn switching_activity_places_old_object_then_acquires_and_really_eats() {
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
        assert_eq!(placement.execution.physical.target_entity, Some(ball));
        assert_eq!(world.entity(ball).unwrap().carried_by, None);
        assert!(!world.entity(ball).unwrap().activity_hold);
        assert_eq!(world.entity(food).unwrap().carried_by, None);
        assert!(!world.entity(food).unwrap().consumed);
        assert!(
            distance(
                world.entity(ball).unwrap().position,
                world.entity(food).unwrap().position
            ) > world.entity(ball).unwrap().radius + world.entity(food).unwrap().radius
        );
        let pickup = world.apply_command(&eat).unwrap();
        assert_eq!(pickup.phase, ActionPhase::Preparation);
        assert_no_activity(&pickup);
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
    fn deliberate_carry_is_protected_and_capacity_is_one_even_after_restore() {
        let (mut world, ball, food) = world();
        world
            .apply_command(&command(HeadlessActionIds::GRAB, ball))
            .unwrap();
        assert!(!world.entity(ball).unwrap().activity_hold);
        for mut candidate in [world.clone(), roundtrip(&world)] {
            let before = material_objects(&candidate);
            for action in [HeadlessActionIds::EAT, HeadlessActionIds::GRAB] {
                let result = candidate.apply_command(&command(action, food)).unwrap();
                assert!(!result.execution.succeeded);
                assert_no_activity(&result);
                assert!(result.grip_transfers.is_empty());
                assert_eq!(material_objects(&candidate), before);
            }
        }
        let release = world
            .apply_command(&command(HeadlessActionIds::GRAB, ball))
            .unwrap();
        assert_eq!(release.grip_transfers[0].kind, GripTransferKind::Released);
        assert_eq!(world.entity(ball).unwrap().carried_by, None);
    }

    #[test]
    fn activity_provenance_survives_save_and_missing_legacy_provenance_protects_carry() {
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
        let mut parts = world.persistence_parts();
        *parts
            .objects
            .iter_mut()
            .find(|object| object.id == ball)
            .unwrap() = legacy.into();
        let mut legacy = HeadlessWorld::from_persistence_parts(parts).unwrap();
        assert!(
            !legacy
                .apply_command(&command(HeadlessActionIds::EAT, food))
                .unwrap()
                .execution
                .succeeded
        );
        assert_eq!(legacy.entity(ball).unwrap().carried_by, Some(ORGANISM));
    }

    #[test]
    fn invalid_or_unsafe_requested_target_does_not_release_activity_possession() {
        for case in ["reach", "other_owner", "player", "unsafe", "consumed"] {
            let (mut world, ball, food) = world();
            world
                .apply_command(&command(HeadlessActionIds::PLAY, ball))
                .unwrap();
            match case {
                "reach" => world.objects.get_mut(&food.raw()).unwrap().position.x = 4.0,
                "other_owner" => {
                    world.objects.get_mut(&food.raw()).unwrap().carried_by = Some(OrganismId(2))
                }
                "player" => {
                    world.begin_player_hold(food).unwrap();
                }
                "unsafe" => world.objects.get_mut(&food.raw()).unwrap().hazard_pain = 0.5,
                "consumed" => world.objects.get_mut(&food.raw()).unwrap().consumed = true,
                _ => unreachable!(),
            }
            let before = material_objects(&world);
            let result = world
                .apply_command(&command(HeadlessActionIds::EAT, food))
                .unwrap();
            assert!(!result.execution.succeeded, "{case}");
            assert_no_activity(&result);
            assert_eq!(material_objects(&world), before, "{case}");
        }
    }

    #[test]
    fn no_safe_supported_placement_keeps_old_object_without_new_activity() {
        let (mut world, ball, food) = world();
        world
            .apply_command(&command(HeadlessActionIds::PLAY, ball))
            .unwrap();
        world.terrain = Some(
            crate::WorldTerrain::new(
                crate::TerrainData {
                    width: 3,
                    depth: 3,
                    origin_x: -2.0,
                    origin_z: -2.0,
                    spacing: 2.0,
                    heights: vec![0.0; 9],
                    obstacles: Vec::new(),
                    water_level: Some(1.0),
                },
                crate::LocomotionLimits::default(),
            )
            .unwrap(),
        );
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
        let mut parts = world.persistence_parts();
        let object = parts
            .objects
            .iter_mut()
            .find(|object| object.id == ball)
            .unwrap();
        object.carried_by = None;
        assert!(HeadlessWorld::from_persistence_parts(parts).is_err());
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
        assert_eq!(placed.body_event.nutrition, 0.0);
        assert_eq!(placed.body_event.investigation, 0.0);
    }
}
