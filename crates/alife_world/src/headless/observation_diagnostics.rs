//! Opt-in developer evidence. Authoritative geometry must never become learner input.
use super::*;
use serde_json::{json, Value};

impl HeadlessWorld {
    /// Read-only explanation of current optical admission, not route reachability.
    /// Cloned tracking is used only to label the same ordinary sensed identities;
    /// this query never observes into the live registry or advances the world.
    pub fn observation_target_diagnostics(
        &self,
        organism: OrganismId,
        target_id: WorldEntityId,
    ) -> Result<Value, ScaffoldContractError> {
        let observer = self.agent_for(organism)?;
        let Some(target) = self.objects.get(&target_id.raw()) else {
            return Ok(json!({"schema":1,"tick":self.tick,"target":target_id,
                "target_exists":false,"visibility_reason":"missing_target"}));
        };
        let relative = subtract(target.position, observer.position);
        let distance = distance(observer.position, target.position);
        let gaze = observer.body_yaw + observer.head_yaw;
        let angular_delta = if relative.x == 0.0 && relative.z == 0.0 {
            0.0
        } else {
            (relative.z.atan2(relative.x) - gaze + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI
        };
        let in_range = distance <= HEADLESS_VISION_RADIUS;
        let in_view = angular_delta.abs() <= VISION_HALF_ANGLE;
        let canonical_sight = self.object_in_sight(observer, target);
        let snapshot = self.physical_observation_snapshot_from_objects(
            organism,
            self.tick,
            observer,
            self.objects.values(),
            true,
        )?;
        let physical_snapshot_has_target = snapshot
            .visible
            .iter()
            .any(|object| object.transport_entity == target_id);
        let mut tracker = self.tracked_objects.clone();
        let mut sensed = GroundedSensorExtractor::extract(&snapshot, &mut tracker)?;
        sensed.calibrate(
            self.organism_registry
                .get(organism)
                .map(WorldOrganismRecord::embodiment),
        )?;
        let observed_target_tracking_id = sensed
            .slots()
            .iter()
            .zip(sensed.transports())
            .find(|(slot, transport)| {
                transport.transport_entity == target_id && slot.confidence.raw() > 0.0
            })
            .map(|(slot, _)| slot.tracked_object_id);
        let reason = if target_id == observer.id {
            "self_target"
        } else if target.consumed {
            "consumed"
        } else if !in_range {
            "out_of_range"
        } else if !in_view {
            "outside_view_angle"
        } else if !canonical_sight {
            "occluded"
        } else if !physical_snapshot_has_target {
            "physical_snapshot_budget"
        } else if observed_target_tracking_id.is_none() {
            "sensor_calibration"
        } else {
            "observed"
        };
        Ok(
            json!({"schema":1,"tick":self.tick,"organism":organism,"target":target_id,
            "target_exists":true,"target_consumed":target.consumed,
            "body_position":observer.position,"body_yaw":observer.body_yaw,
            "head_yaw":observer.head_yaw,"gaze_yaw":gaze,
            "target_position":target.position,"target_radius":target.radius,
            "target_distance":distance,"vision_radius":HEADLESS_VISION_RADIUS,
            "angular_delta":angular_delta,"vision_half_angle":VISION_HALF_ANGLE,
            "within_vision_range":in_range,"within_view_angle":in_view,
            "canonical_in_sight":canonical_sight,
            "physical_snapshot_contains_target":physical_snapshot_has_target,
            "observed_target_tracking_id":observed_target_tracking_id,
            "visibility_reason":reason,
            "visible_world_entities":snapshot.visible.iter()
                .map(|object| object.transport_entity).collect::<Vec<_>>(),
            "contact_reachable":self.physical_contact_reachable(observer.position,target.position),
            "straight_solid_segment_clear":self.solid_segment_clear(observer.position,target.position),
            "terrain_present":self.terrain.is_some(),
            "target_ground_height":self.terrain.as_ref()
                .and_then(|terrain| terrain.surface().height(target.position.x,target.position.z)),
            "terrain_route_reachability":"not_assessed",
            "claim_scope":"optical admission and direct contact/segment facts; no path planner or hidden-goal assistance"}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SUBJECT: OrganismId = OrganismId(1);
    fn scene(position: Vec3f, wall: bool) -> (HeadlessWorld, WorldEntityId) {
        let mut builder = HeadlessScenarioBuilder::new(624_010)
            .agent("observer", SUBJECT, Vec3f::ZERO)
            .food("food", position, 0.6);
        if wall {
            builder = builder.obstacle("gate", Vec3f::new(1.5, 0.0, 0.0), 1.0);
        }
        let world = builder.build().unwrap();
        let target = world.entity_id("food").unwrap();
        (world, target)
    }
    #[test]
    fn separates_range_view_and_occlusion_without_mutation() {
        for (position, wall, expected) in [
            (Vec3f::new(3.0, 0.0, 0.0), false, "observed"),
            (Vec3f::new(9.0, 0.0, 0.0), false, "out_of_range"),
            (Vec3f::new(-3.0, 0.0, 0.0), false, "outside_view_angle"),
            (Vec3f::new(3.0, 0.0, 0.0), true, "occluded"),
        ] {
            let (world, target) = scene(position, wall);
            let before = world.canonical_signature_digest().unwrap();
            let tracking =
                serde_json::to_value(world.tracked_objects.save_state(SUBJECT).unwrap()).unwrap();
            let report = world
                .observation_target_diagnostics(SUBJECT, target)
                .unwrap();
            assert_eq!(report["visibility_reason"], expected);
            assert_eq!(world.canonical_signature_digest().unwrap(), before);
            assert_eq!(
                serde_json::to_value(world.tracked_objects.save_state(SUBJECT).unwrap()).unwrap(),
                tracking
            );
            assert_eq!(report["terrain_route_reachability"], "not_assessed");
        }
    }
    #[test]
    fn observed_is_not_contact_reachable_and_tracking_is_reproducible() {
        let (mut world, target) = scene(Vec3f::new(3.0, 0.0, 0.0), false);
        let before = world
            .observation_target_diagnostics(SUBJECT, target)
            .unwrap();
        assert_eq!(before["visibility_reason"], "observed");
        assert_eq!(before["contact_reachable"], false);
        let snapshot = world
            .physical_observation_snapshot_from_objects(
                SUBJECT,
                world.tick,
                world.agent_for(SUBJECT).unwrap(),
                world.objects.values(),
                true,
            )
            .unwrap();
        let _ = GroundedSensorExtractor::extract(&snapshot, &mut world.tracked_objects).unwrap();
        let after = world
            .observation_target_diagnostics(SUBJECT, target)
            .unwrap();
        assert_eq!(
            before["observed_target_tracking_id"],
            after["observed_target_tracking_id"]
        );
    }

    #[test]
    fn closing_a_real_gate_explains_occlusion_while_food_remains_in_range() {
        let mut world = HeadlessScenarioBuilder::new(624_011)
            .agent("observer", SUBJECT, Vec3f::ZERO)
            .food("food", Vec3f::new(6.5, 0.0, 0.0), 0.6)
            .obstacle("gate", Vec3f::new(4.0, 0.0, 7.0), 1.0)
            .build()
            .unwrap();
        let food = world.entity_id("food").unwrap();
        let gate = world.entity_id("gate").unwrap();
        let open = world.observation_target_diagnostics(SUBJECT, food).unwrap();
        assert_eq!(open["visibility_reason"], "observed");
        world
            .editor_move_object(gate, Vec3f::new(4.0, 0.0, 0.0))
            .unwrap();
        let signature = world.canonical_signature_digest().unwrap();
        let closed = world.observation_target_diagnostics(SUBJECT, food).unwrap();
        assert_eq!(closed["within_vision_range"], true);
        assert_eq!(closed["within_view_angle"], true);
        assert_eq!(closed["visibility_reason"], "occluded");
        assert_eq!(closed["physical_snapshot_contains_target"], false);
        assert_eq!(closed["terrain_route_reachability"], "not_assessed");
        assert_eq!(world.canonical_signature_digest().unwrap(), signature);
    }
}
