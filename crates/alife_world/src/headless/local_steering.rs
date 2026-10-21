//! Bounded actuator clearance toward a currently observed destination.
//! No map, route, hidden destination pose, or semantic goal enters the selector.
use super::*;

const LOCAL_HORIZON: f32 = 1.5;
const MAX_NON_PROGRESS_STEPS: u8 = 16;
const STEERING_ANGLES: [f32; 9] = [0.0, -30.0, 30.0, -60.0, 60.0, -90.0, 90.0, -105.0, 105.0];

/// A physical-controller safety limit, not a route or cognitive destination.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LocalSteeringState {
    pub target: WorldEntityId,
    pub non_progress_steps: u8,
    pub best_observed_distance: f32,
}

impl LocalSteeringState {
    pub(crate) fn validate(self) -> Result<(), ScaffoldContractError> {
        self.target.validate()?;
        if self.non_progress_steps > MAX_NON_PROGRESS_STEPS
            || !self.best_observed_distance.is_finite()
            || self.best_observed_distance < 0.0
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

impl HeadlessWorld {
    /// Rebuild only the immutable canonical visual admission, without a second
    /// tracked-object observation or a remembered/world-pose fallback.
    pub(super) fn visible_motion_target(
        &self,
        organism: OrganismId,
        target: WorldEntityId,
    ) -> Result<Option<Vec3f>, ScaffoldContractError> {
        let observer = self.agent_for(organism)?;
        let record = self.organism_registry.get(organism);
        if observer.consumed || record.is_some_and(|record| !record.lifecycle().is_alive()) {
            return Ok(None);
        }
        let gain = record.map_or(1.0, |record| {
            record
                .embodiment()
                .sensor_gain(alife_core::SensorCapability::Vision)
        });
        if !gain.is_finite() || gain <= 0.0 {
            return Ok(None);
        }
        let snapshot = self.physical_observation_snapshot_from_objects(
            organism,
            self.tick,
            observer,
            self.objects.values(),
            true,
        )?;
        if snapshot.tick != self.tick {
            return Err(ScaffoldContractError::NonMonotonicTick);
        }
        Ok(snapshot
            .visible
            .iter()
            .find(|object| object.transport_entity == target)
            .filter(|object| (object.confidence.raw() * gain).clamp(0.0, 1.0) > 0.0)
            .map(|object| subtract(object.position, snapshot.observer_pose.translation)))
    }

    /// Only nearest physical clearance ranges leave this bounded sensor query.
    /// Optical transparency does not turn a solid surface into free space.
    fn local_clearance_ranges(&self, observer: &WorldObject, bearing: f32) -> [f32; 9] {
        let body_radius = self
            .terrain
            .as_ref()
            .map_or(crate::LocomotionLimits::default().body_radius, |terrain| {
                terrain.limits().body_radius
            });
        // Conservative swept envelope while carrying. Ordinary grip-arc and
        // terrain checks still validate the one selected step before mutation.
        let radius = self
            .objects
            .values()
            .filter(|object| !object.consumed && object.carried_by == observer.organism_id)
            .fold(body_radius, |radius, object| {
                radius.max(CARRY_GRIP_DISTANCE + object.radius)
            });
        let mut ranges = [0.0; 9];
        if !radius.is_finite() {
            return ranges;
        }
        for (index, degrees) in STEERING_ANGLES.iter().enumerate() {
            let yaw = bearing + degrees.to_radians();
            let gaze_delta = (yaw - observer.body_yaw - observer.head_yaw + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            if gaze_delta.abs() > VISION_HALF_ANGLE {
                continue; // Unknown/out-of-view directions are not free corridors.
            }
            let direction = Vec3f::new(yaw.cos(), 0.0, yaw.sin());
            let mut nearest = LOCAL_HORIZON;
            for object in self
                .objects
                .values()
                .filter(|object| object.kind == WorldObjectKind::Obstacle && !object.consumed)
            {
                if let Some(hit) = sight_sphere_hit(
                    observer.position,
                    direction,
                    object.position,
                    object.radius.max(HEADLESS_CONTACT_RADIUS) + radius,
                    nearest,
                ) {
                    nearest = nearest.min(hit);
                }
            }
            if let Some(terrain) = &self.terrain {
                for bounds in &terrain.surface().obstacles {
                    let expanded = [
                        bounds[0] - radius,
                        bounds[1] - radius,
                        bounds[2] + radius,
                        bounds[3] + radius,
                        bounds[4],
                        bounds[5],
                    ];
                    if let Some(hit) =
                        sight_box_hit(observer.position, direction, expanded, nearest)
                    {
                        nearest = nearest.min(hit);
                    }
                }
            }
            ranges[index] = nearest;
        }
        ranges
    }

    pub(super) fn steer_visible_step(
        &self,
        organism: OrganismId,
        target: WorldEntityId,
        relative: Vec3f,
        original: Vec3f,
    ) -> Result<Option<(Vec3f, bool)>, ScaffoldContractError> {
        let observer = self.agent_for(organism)?;
        let step = (original.x - observer.position.x).hypot(original.z - observer.position.z);
        if step <= f32::EPSILON {
            return Ok(Some((original, false)));
        }
        let bearing = relative.z.atan2(relative.x);
        let ranges = self.local_clearance_ranges(observer, bearing);
        let exhausted = observer.local_steering.is_some_and(|state| {
            state.target == target && state.non_progress_steps >= MAX_NON_PROGRESS_STEPS
        });
        Ok(select_clearance_step(ranges, step, exhausted).map(|index| {
            if index == 0 {
                // Keep step_toward's exact endpoint snap. Reconstructing an
                // unobstructed step through trigonometry can move it by an ULP.
                return (original, false);
            }
            let yaw = bearing + STEERING_ANGLES[index].to_radians();
            (
                Vec3f::new(
                    observer.position.x + yaw.cos() * step,
                    original.y,
                    observer.position.z + yaw.sin() * step,
                ),
                true,
            )
        }))
    }

    pub(super) fn commit_local_steering(
        &mut self,
        organism: OrganismId,
        observed_goal: Option<(WorldEntityId, Vec3f)>,
        displacement: Vec3f,
        detour: bool,
    ) -> Result<(), ScaffoldContractError> {
        let agent = self.agent_entity_id(organism)?;
        let object = self.objects.get_mut(&agent.raw()).expect("agent exists");
        if let Some((target, relative)) = observed_goal {
            let before = relative.x.hypot(relative.z);
            let after = (relative.x - displacement.x).hypot(relative.z - displacement.z);
            let previous = object.local_steering.filter(|state| state.target == target);
            let best = previous.map_or(before, |state| state.best_observed_distance.min(before));
            // Replenish only on net improvement beyond the best observed range.
            // An outward/inward cycle cannot refresh this finite allowance.
            let progress = after + 0.001 < best;
            let count = previous.map_or(0, |state| state.non_progress_steps);
            object.local_steering = Some(LocalSteeringState {
                target,
                non_progress_steps: if progress {
                    0
                } else if detour {
                    count.saturating_add(1).min(MAX_NON_PROGRESS_STEPS)
                } else {
                    count
                },
                best_observed_distance: if progress { after } else { best },
            });
        } else if displacement.x.hypot(displacement.z) > 0.000_01 {
            // A successful creature-chosen intermediate movement is a new
            // choice. Repeating a blocked command or losing sight is not.
            object.local_steering = None;
        }
        Ok(())
    }

    pub(super) fn blocked_local_move(
        &mut self,
        command: ActionCommand,
    ) -> Result<HeadlessActionResult, ScaffoldContractError> {
        self.finish_action(
            command,
            false,
            Some(ReferenceActionFailure::Blocked),
            physical(PhysicalContactKind::Blocked, None, Vec3f::ZERO, 0.08)?,
            OutcomeProfile::blocked(),
            Vec::new(),
        )
    }
}

/// Pure bounded actuator choice. It cannot query geometry, target objects,
/// history, rewards, or a route. A blocked selected step gets no second search.
fn select_clearance_step(ranges: [f32; 9], step: f32, exhausted: bool) -> Option<usize> {
    let clear = |index: usize| ranges[index].is_finite() && ranges[index] >= step + 0.005;
    if clear(0) {
        Some(0)
    } else if exhausted {
        None
    } else {
        (1..ranges.len()).find(|index| clear(*index))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{ActionTarget, DurationTicks};

    const ORGANISM: OrganismId = OrganismId(1);

    fn world(obstacles: Vec<[f32; 6]>, radius: f32) -> (HeadlessWorld, WorldEntityId) {
        let mut world = HeadlessScenarioBuilder::new(191_020)
            .agent("actor", ORGANISM, Vec3f::ZERO)
            .food("destination", Vec3f::new(2.0, 0.0, 0.0), 0.6)
            .build()
            .unwrap();
        world.terrain = Some(
            crate::WorldTerrain::new(
                crate::TerrainData {
                    width: 3,
                    depth: 3,
                    origin_x: -4.0,
                    origin_z: -4.0,
                    spacing: 4.0,
                    heights: vec![0.0; 9],
                    obstacles,
                    water_level: None,
                },
                crate::LocomotionLimits {
                    body_radius: radius,
                    ..Default::default()
                },
            )
            .unwrap(),
        );
        let target = world.entity_id("destination").unwrap();
        (world, target)
    }

    fn command(
        action: ActionId,
        target: Option<WorldEntityId>,
        point: Option<Vec3f>,
    ) -> ActionCommand {
        ActionCommand::structured(
            ORGANISM,
            action,
            ActionKind::Move,
            ActionTarget::new(target, point),
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

    fn approach(target: WorldEntityId) -> ActionCommand {
        command(HeadlessActionIds::APPROACH, Some(target), None)
    }

    fn low_blocker() -> [f32; 6] {
        [0.26, -0.15, 0.45, 0.15, 0.0, 0.2]
    }

    #[test]
    fn clear_visible_oblique_arrival_keeps_the_exact_chosen_endpoint() {
        for angle in [
            133.32324_f32.to_radians(),
            -133.32324_f32.to_radians(),
            std::f32::consts::PI,
            0.218827,
            -1.2,
            0.35,
        ] {
            let (mut world, target) = world(Vec::new(), 0.2);
            let endpoint = Vec3f::new(3.138_172, 0.0, -2.085_676_2);
            world.objects.get_mut(&target.raw()).unwrap().position = endpoint;
            let actor = world.agent_entity_id(ORGANISM).unwrap();
            let body = world.objects.get_mut(&actor.raw()).unwrap();
            body.position = Vec3f::new(
                endpoint.x - 0.05 * angle.cos(),
                endpoint.y,
                endpoint.z - 0.05 * angle.sin(),
            );
            body.body_yaw = angle;
            assert!(world
                .visible_motion_target(ORGANISM, target)
                .unwrap()
                .is_some());
            let moved = world.apply_command(&approach(target)).unwrap();
            assert!(moved.execution.succeeded);
            assert_eq!(world.entity(actor).unwrap().position, endpoint);
        }
    }

    fn roundtrip(world: &HeadlessWorld) -> HeadlessWorld {
        let save = crate::PortableSaveFile::from_headless_world(
            "visible-steering-regression",
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
    fn visible_destination_over_low_obstacle_gets_only_bounded_local_steps() {
        let (mut world, target) = world(vec![low_blocker()], 0.2);
        assert!(world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_some());
        let first = world.apply_command(&approach(target)).unwrap();
        assert!(first.execution.succeeded);
        assert!(first.execution.physical.displacement.z.abs() > 0.05);
        assert!(distance(first.execution.physical.displacement, Vec3f::ZERO) <= MOVE_STEP + 0.0001);
        assert_eq!(
            world
                .agent_for(ORGANISM)
                .unwrap()
                .local_steering
                .unwrap()
                .non_progress_steps,
            0
        );
        for _ in 0..60 {
            let before = world.agent_for(ORGANISM).unwrap().position;
            if distance(before, world.entity(target).unwrap().position) < 0.2 {
                break;
            }
            let step = world.apply_command(&approach(target)).unwrap();
            assert!(
                step.execution.succeeded,
                "position={before:?}, result={step:?}"
            );
            let after = world.agent_for(ORGANISM).unwrap().position;
            assert!(distance(before, after) <= MOVE_STEP + 0.0001);
            assert!(world.terrain.as_ref().unwrap().walkable(after.x, after.z));
        }
        assert!(
            distance(
                world.agent_for(ORGANISM).unwrap().position,
                world.entity(target).unwrap().position
            ) < 0.2
        );
        assert!(!world.entity(target).unwrap().consumed);
    }

    #[test]
    fn visible_gap_respects_body_size_and_does_not_squeeze_through() {
        let gap = vec![
            [0.26, 0.18, 0.45, 1.0, 0.0, 2.0],
            [0.26, -1.0, 0.45, -0.18, 0.0, 2.0],
        ];
        let (mut small, target) = world(gap.clone(), 0.1);
        let (mut large, large_target) = world(gap, 0.2);
        for (candidate, target) in [(&small, target), (&large, large_target)] {
            assert!(candidate
                .visible_motion_target(ORGANISM, target)
                .unwrap()
                .is_some());
        }
        let straight = small.apply_command(&approach(target)).unwrap();
        assert!(straight.execution.succeeded);
        assert!(straight.execution.physical.displacement.x > 0.09);
        assert!(straight.execution.physical.displacement.z.abs() < 0.0001);
        let around = large.apply_command(&approach(large_target)).unwrap();
        assert!(around.execution.succeeded);
        assert!(around.execution.physical.displacement.x < 0.09);
        assert!(around.execution.physical.displacement.z.abs() > 0.05);
    }

    #[test]
    fn transparent_solid_can_preserve_sight_without_becoming_walkable() {
        let (mut transparent, target) = world(Vec::new(), 0.2);
        let obstacle = transparent
            .insert_object(SpawnSpec {
                label: "glass",
                kind: WorldObjectKind::Obstacle,
                organism_id: None,
                position: Vec3f::new(1.0, 0.0, 0.0),
                nutrition: 0.0,
                hazard_pain: 0.0,
                token_id: None,
                social_affinity: 0.0,
                teacher_channel: None,
            })
            .unwrap();
        transparent
            .editor_set_optical_opacity(obstacle, 0.0)
            .unwrap();
        assert!(transparent
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_some());
        let result = transparent.apply_command(&approach(target)).unwrap();
        assert!(result.execution.succeeded);
        assert!(result.execution.physical.displacement.z.abs() > 0.01);
        let actor = transparent.agent_for(ORGANISM).unwrap();
        assert!(!transparent
            .entity(obstacle)
            .unwrap()
            .blocks_position(actor.position));
        let mut opaque = transparent.clone();
        opaque.editor_set_optical_opacity(obstacle, 1.0).unwrap();
        assert!(opaque
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_none());
        let before = opaque.agent_for(ORGANISM).unwrap().position;
        assert!(
            !opaque
                .apply_command(&approach(target))
                .unwrap()
                .execution
                .succeeded
        );
        assert_eq!(opaque.agent_for(ORGANISM).unwrap().position, before);
    }

    #[test]
    fn opaque_corner_and_remembered_bearing_do_not_enable_detours() {
        let wall = [0.26, -1.5, 0.45, 1.5, 0.0, 2.0];
        let (mut world, target) = world(vec![wall], 0.2);
        assert!(world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_none());
        let before = world.agent_for(ORGANISM).unwrap().position;
        let hidden = world.apply_command(&approach(target)).unwrap();
        assert_eq!(
            hidden.execution.failure,
            Some(ReferenceActionFailure::Blocked)
        );
        assert_eq!(hidden.execution.physical.displacement, Vec3f::ZERO);
        let remembered = command(
            HeadlessActionIds::APPROACH,
            Some(target),
            Some(Vec3f::new(2.0, 0.0, 0.0)),
        );
        let result = world.apply_command(&remembered).unwrap();
        assert!(!result.execution.succeeded);
        assert_eq!(world.agent_for(ORGANISM).unwrap().position, before);
        // Moving an unseen target changes no automatic bearing or detour.
        world.objects.get_mut(&target.raw()).unwrap().position.z = 0.3;
        assert_eq!(
            world.apply_command(&approach(target)).unwrap().execution,
            hidden.execution
        );
        // A learned intermediate movement remains possible through physics.
        let intermediate = command(
            ActionKind::Move.canonical_id(),
            None,
            Some(Vec3f::new(0.0, 0.0, -1.0)),
        );
        assert!(
            world
                .apply_command(&intermediate)
                .unwrap()
                .execution
                .succeeded
        );
        assert!(world.agent_for(ORGANISM).unwrap().position.z < 0.0);
    }

    #[test]
    fn losing_real_visibility_after_a_step_stops_assistance_immediately() {
        let post = [1.0, -0.09, 1.2, -0.01, 0.0, 2.0];
        let (mut world, target) = world(vec![low_blocker(), post], 0.2);
        assert!(world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_some());
        assert!(
            world
                .apply_command(&approach(target))
                .unwrap()
                .execution
                .succeeded
        );
        assert!(world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_none());
        let before = world.agent_for(ORGANISM).unwrap().position;
        let budget = world.agent_for(ORGANISM).unwrap().local_steering;
        let blocked = world.apply_command(&approach(target)).unwrap();
        assert!(!blocked.execution.succeeded);
        assert_eq!(blocked.execution.physical.displacement, Vec3f::ZERO);
        assert_eq!(world.agent_for(ORGANISM).unwrap().position, before);
        assert_eq!(world.agent_for(ORGANISM).unwrap().local_steering, budget);
    }

    #[test]
    fn exhausted_same_goal_stays_blocked_across_ticks_reload_and_repeated_commands() {
        let (mut world, target) = world(vec![low_blocker()], 0.2);
        let actor = world.agent_entity_id(ORGANISM).unwrap();
        world.objects.get_mut(&actor.raw()).unwrap().local_steering = Some(LocalSteeringState {
            target,
            non_progress_steps: MAX_NON_PROGRESS_STEPS,
            best_observed_distance: 2.0,
        });
        let initial = world.canonical_signature_digest().unwrap();
        let mut restored = roundtrip(&world);
        assert_eq!(
            restored.agent_for(ORGANISM).unwrap().local_steering,
            world.agent_for(ORGANISM).unwrap().local_steering
        );
        for candidate in [&mut world, &mut restored] {
            for _ in 0..3 {
                assert!(
                    !candidate
                        .apply_command(&approach(target))
                        .unwrap()
                        .execution
                        .succeeded
                );
                assert_eq!(
                    candidate
                        .agent_for(ORGANISM)
                        .unwrap()
                        .local_steering
                        .unwrap()
                        .non_progress_steps,
                    MAX_NON_PROGRESS_STEPS
                );
                candidate.advance_tick();
            }
            let intermediate = command(
                ActionKind::Move.canonical_id(),
                None,
                Some(Vec3f::new(0.0, 0.0, -1.0)),
            );
            assert!(
                candidate
                    .apply_command(&intermediate)
                    .unwrap()
                    .execution
                    .succeeded
            );
            assert!(candidate
                .agent_for(ORGANISM)
                .unwrap()
                .local_steering
                .is_none());
            assert!(
                candidate
                    .apply_command(&approach(target))
                    .unwrap()
                    .execution
                    .succeeded
            );
        }
        let (fresh, _) = self::world(vec![low_blocker()], 0.2);
        assert_ne!(initial, fresh.canonical_signature_digest().unwrap());
    }

    #[test]
    fn geometry_beyond_the_local_sensor_horizon_cannot_choose_a_route() {
        let (mut local, target) = world(vec![low_blocker()], 0.2);
        let (mut distant, distant_target) =
            world(vec![low_blocker(), [3.0, -3.0, 3.5, -2.0, 0.0, 3.0]], 0.2);
        assert_eq!(
            local.apply_command(&approach(target)).unwrap().execution,
            distant
                .apply_command(&approach(distant_target))
                .unwrap()
                .execution
        );
    }

    #[test]
    fn carry_clearance_changes_local_step_without_dropping_the_object() {
        let (mut bare, target) = world(vec![[1.06, -0.1, 1.26, 0.1, 0.0, 0.2]], 0.2);
        let actor = bare.agent_entity_id(ORGANISM).unwrap();
        bare.objects.get_mut(&actor.raw()).unwrap().position.x = 0.2;
        bare.objects.get_mut(&target.raw()).unwrap().position.x = 3.0;
        let ball = bare
            .spawn_toy("carried", Vec3f::new(0.7, 0.0, 0.0), true)
            .unwrap();
        let mut carrying = bare.clone();
        let grab = command(HeadlessActionIds::GRAB, Some(ball), None);
        assert!(carrying.apply_command(&grab).unwrap().execution.succeeded);
        let direct = bare.apply_command(&approach(target)).unwrap();
        assert!(direct.execution.succeeded);
        assert!(direct.execution.physical.displacement.x > 0.09);
        let protected = carrying.apply_command(&approach(target)).unwrap();
        assert!(protected.execution.succeeded);
        assert!(protected.execution.physical.displacement.z.abs() > 0.05);
        assert_eq!(carrying.entity(ball).unwrap().carried_by, Some(ORGANISM));
        assert!(!carrying.entity(ball).unwrap().activity_hold);
        assert!(protected.grip_transfers.is_empty());
    }

    #[test]
    fn absent_visual_gain_or_admission_cannot_authorize_steering() {
        let (mut world, target) = world(vec![low_blocker()], 0.2);
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
        let mut record =
            WorldOrganismRecord::new(ORGANISM, agent, genome, phenotype, biology, Tick::ZERO)
                .unwrap();
        // A valid body fixture without a Vision port has zero visual gain.
        let mut value = serde_json::to_value(record.embodiment()).unwrap();
        let ports = value["sensors"].as_array_mut().unwrap();
        let index = ports
            .iter()
            .position(|port| port["capability"] == "Vision")
            .unwrap();
        let start: usize = ports[..index]
            .iter()
            .map(|port| port["value_lanes"].as_u64().unwrap() as usize)
            .sum();
        let width = ports[index]["value_lanes"].as_u64().unwrap() as usize;
        ports.remove(index);
        value["sensor_calibration"]
            .as_array_mut()
            .unwrap()
            .drain(start..start + width);
        value["revision"] = serde_json::json!(record.embodiment().revision() + 1);
        record
            .replace_embodiment_state(serde_json::from_value(value).unwrap())
            .unwrap();
        world.register_organism_record(record).unwrap();
        assert!(world.object_in_sight(
            world.agent_for(ORGANISM).unwrap(),
            world.entity(target).unwrap()
        ));
        assert!(world
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_none());
        assert!(
            !world
                .apply_command(&approach(target))
                .unwrap()
                .execution
                .succeeded
        );

        let (mut cluttered, target) = self::world(Vec::new(), 0.2);
        for index in 0..MAX_VISIBLE_ENTITIES {
            let id = cluttered
                .spawn_toy(&format!("clutter-{index}"), Vec3f::new(0.4, 0.0, 0.4), true)
                .unwrap();
            cluttered.editor_set_optical_opacity(id, 0.0).unwrap();
        }
        assert!(cluttered.object_in_sight(
            cluttered.agent_for(ORGANISM).unwrap(),
            cluttered.entity(target).unwrap()
        ));
        assert!(cluttered
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_none());
        assert!(
            !cluttered
                .apply_command(&approach(target))
                .unwrap()
                .execution
                .succeeded
        );
    }

    #[test]
    fn equal_observations_hide_goal_and_distant_opening_changes_until_intermediate_reveals_goal() {
        let wall = [0.26, -0.3, 0.45, 0.3, 0.0, 2.0];
        let (mut left, target) = world(vec![wall], 0.2);
        let mut right = left.clone();
        right.objects.get_mut(&target.raw()).unwrap().position.z = 0.1;
        right.terrain = Some(
            crate::WorldTerrain::new(
                crate::TerrainData {
                    width: 3,
                    depth: 3,
                    origin_x: -4.0,
                    origin_z: -4.0,
                    spacing: 4.0,
                    heights: vec![0.0; 9],
                    obstacles: vec![wall, [3.0, -3.0, 3.5, -1.0, 0.0, 2.0]],
                    water_level: None,
                },
                crate::LocomotionLimits::default(),
            )
            .unwrap(),
        );
        let snapshot = |world: &HeadlessWorld| {
            world
                .physical_observation_snapshot_from_objects(
                    ORGANISM,
                    world.tick(),
                    world.agent_for(ORGANISM).unwrap(),
                    world.objects.values(),
                    true,
                )
                .unwrap()
        };
        assert_eq!(snapshot(&left), snapshot(&right));
        assert_eq!(
            left.apply_command(&approach(target)).unwrap().execution,
            right.apply_command(&approach(target)).unwrap().execution
        );
        let intermediate = command(
            ActionKind::Move.canonical_id(),
            None,
            Some(Vec3f::new(0.0, 0.0, -1.0)),
        );
        for _ in 0..7 {
            assert!(
                left.apply_command(&intermediate)
                    .unwrap()
                    .execution
                    .succeeded
            );
        }
        assert!(left
            .visible_motion_target(ORGANISM, target)
            .unwrap()
            .is_some());
        let restored_view = left.apply_command(&approach(target)).unwrap();
        assert!(restored_view.execution.succeeded);
        assert!(restored_view.execution.physical.displacement.x > 0.0);
    }

    #[test]
    fn invalid_controller_state_is_rejected_by_restore() {
        let (world, target) = world(Vec::new(), 0.2);
        for on_target in [false, true] {
            let mut parts = world.save_state();
            let id = if on_target {
                target
            } else {
                world.agent_entity_id(ORGANISM).unwrap()
            };
            parts
                .objects
                .iter_mut()
                .find(|object| object.id == id)
                .unwrap()
                .local_steering = Some(LocalSteeringState {
                target,
                non_progress_steps: if on_target {
                    1
                } else {
                    MAX_NON_PROGRESS_STEPS + 1
                },
                best_observed_distance: 2.0,
            });
            assert!(HeadlessWorld::from_save_state(&parts).is_err());
        }
    }

    #[test]
    fn alternating_outward_and_inward_detours_cannot_replenish_progress_budget() {
        let (mut world, target) = world(Vec::new(), 0.2);
        let agent = world.agent_entity_id(ORGANISM).unwrap();
        let best = 3.0_f32.sqrt() * MOVE_STEP;
        world.objects.get_mut(&agent.raw()).unwrap().local_steering = Some(LocalSteeringState {
            target,
            non_progress_steps: 0,
            best_observed_distance: best,
        });
        for _ in 0..MAX_NON_PROGRESS_STEPS {
            // Measured range alternates between these radii without ever
            // beating the best observed range; inward steps cannot reset it.
            world
                .commit_local_steering(
                    ORGANISM,
                    Some((target, Vec3f::new(best, 0.0, 0.0))),
                    Vec3f::new(best - 2.0 * MOVE_STEP, 0.0, 0.0),
                    true,
                )
                .unwrap();
            world
                .commit_local_steering(
                    ORGANISM,
                    Some((target, Vec3f::new(2.0 * MOVE_STEP, 0.0, 0.0))),
                    Vec3f::new(2.0 * MOVE_STEP - best, 0.0, 0.0),
                    true,
                )
                .unwrap();
        }
        let state = world.agent_for(ORGANISM).unwrap().local_steering.unwrap();
        assert_eq!(state.non_progress_steps, MAX_NON_PROGRESS_STEPS);
        assert_eq!(state.best_observed_distance, best);
        assert_eq!(
            roundtrip(&world)
                .agent_for(ORGANISM)
                .unwrap()
                .local_steering,
            Some(state)
        );
    }
}
