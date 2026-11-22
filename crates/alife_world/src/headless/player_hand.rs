//! Player carrying is a world-owned physical constraint, never a neural action.
use super::*;
use serde::{Deserialize, Serialize};

// Isolated source review only; no runtime setting or additional tool.
const CHIME_PLACEMENT_REVIEW: bool = true;
const ORIGINAL_CHIME: &str = "player-activity-toy";
const CHIME_RADIUS: f32 = 0.50;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerHold {
    pub object_id: WorldEntityId,
    pub last_ground: Vec3f,
}

fn is_original_player_chime(
    object: &WorldObject,
    registered_id: Option<WorldEntityId>,
) -> bool {
    object.kind == WorldObjectKind::ActivityToy
        && object.label == ORIGINAL_CHIME
        && registered_id == Some(object.id)
        && object.radius == CHIME_RADIUS
        && object.organism_id.is_none()
        && !object.consumed
        && object.carried_by.is_none()
        && !object.activity_hold
}

/// Read the same exact identity before the live label index is reconstructed.
/// Only the opted-in named capability can enter the later terrain validator.
pub(super) fn saved_original_player_chime(
    object: &crate::persistence::WorldObjectSaveState,
    objects: &[crate::persistence::WorldObjectSaveState],
) -> bool {
    if !CHIME_PLACEMENT_REVIEW
        || object.kind != WorldObjectKind::ActivityToy
        || object.label != ORIGINAL_CHIME
    {
        return false;
    }
    let mut named = objects.iter().filter(|o| o.label == ORIGINAL_CHIME);
    let registered_id = named.next().map(|o| o.id);
    named.next().is_none()
        && is_original_player_chime(&WorldObject::from(object.clone()), registered_id)
}

impl HeadlessWorld {
    pub fn player_held_object(&self) -> Option<WorldEntityId> {
        self.player_hold.as_ref().map(|hold| hold.object_id)
    }

    fn original_player_chime(&self, object: &WorldObject) -> bool {
        is_original_player_chime(object, self.entity_id(ORIGINAL_CHIME))
    }

    /// Only the one deliberately movable chime refuses alternate App slide
    /// spots. Other ordinary held objects keep their existing cursor slide.
    pub fn player_holds_original_chime(&self) -> bool {
        CHIME_PLACEMENT_REVIEW
            && self.player_held_object().and_then(|id| self.entity(id))
                .is_some_and(|object| self.original_player_chime(object))
    }

    fn chime_terrain_foot(&self, at: Vec3f) -> Option<Vec3f> {
        let t = self.terrain()?;
        let surface = t.surface();
        let h = surface.height(at.x, at.z)?;
        let mut limits = t.limits();
        limits.body_radius = CHIME_RADIUS;
        limits.max_wading_depth = 0.;
        if !surface.walkable_with(at.x, at.z, limits) {
            return None;
        }
        let r = CHIME_RADIUS;
        let d = r * std::f32::consts::FRAC_1_SQRT_2;
        for (dx, dz) in [(0., 0.), (r, 0.), (-r, 0.), (0., r), (0., -r),
            (d, d), (d, -d), (-d, d), (-d, -d)] {
            let y = surface.height(at.x + dx, at.z + dz)?;
            if (y - h).abs() > 0.18 || surface.water_level.is_some_and(|level| y <= level + 0.035) {
                return None;
            }
        }
        Some(Vec3f::new(at.x, h, at.z))
    }

    fn chime_clear_foot(&self, id: WorldEntityId, at: Vec3f) -> Option<Vec3f> {
        let ground = self.chime_terrain_foot(at)?;
        // Give already planted ground the native orchard's .42m spacing,
        // in addition to the actual table radius. Never replace any plant.
        let overlaps_bed = self.ecology.orchard_beds.iter().any(|bed| {
            (bed.tile.x as f32 + 0.5 - ground.x).hypot(bed.tile.z as f32 + 0.5 - ground.z)
                < CHIME_RADIUS + 0.42
        });
        let occupied = self.objects.values().any(|o| {
            o.id != id && !o.consumed && o.carried_by.is_none()
                && (o.position.x - ground.x).hypot(o.position.z - ground.z) < CHIME_RADIUS + o.radius
        });
        (!overlaps_bed && !occupied).then_some(ground)
    }

    fn chime_clear_transfer(&self, id: WorldEntityId, from: Vec3f, ground: Vec3f) -> bool {
        let Some(t) = self.terrain() else { return false; };
        let mut limits = t.limits();
        limits.body_radius = CHIME_RADIUS;
        limits.max_wading_depth = 0.;
        let surface = t.surface();
        let Some(end) = surface.resolve_move_with(from, ground, limits) else { return false; };
        let v = |p: [f64; 3]| Vec3f::new(p[0] as f32, p[1] as f32, p[2] as f32);
        surface.grounded_sweep_segments(from, end).is_some_and(|pieces| pieces.into_iter().all(|p| {
            self.objects.values().all(|o| {
                o.id == id || o.consumed || o.carried_by.is_some()
                    || motion_sphere_entry(v(p.start), v(p.end), o.position, CHIME_RADIUS + o.radius).is_none()
            })
        }))
    }

    fn can_lift_original_chime(&self, o: &WorldObject) -> bool {
        CHIME_PLACEMENT_REVIEW && self.original_player_chime(o) && self.toy_rolls.is_empty()
            && self.chime_clear_foot(o.id, o.position).is_some_and(|p| {
                (p.y - o.position.y).abs() <= 0.012
            })
    }

    /// Whether this object is available for the player's pending pickup gesture.
    /// Beginning the hold still validates the terrain support before moving it.
    pub fn can_begin_player_hold(&self, id: WorldEntityId) -> bool {
        self.player_hold_target(id).is_ok()
    }

    fn player_hold_target(&self, id: WorldEntityId) -> Result<&WorldObject, ScaffoldContractError> {
        if self.player_hold.is_some() {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        let object = self.entity(id).ok_or(ScaffoldContractError::InvalidId)?;
        if object.consumed
            || object.carried_by.is_some()
            || !(matches!(
                object.kind,
                WorldObjectKind::Agent
                    | WorldObjectKind::Food
                    | WorldObjectKind::Ball
                    | WorldObjectKind::Token
            ) || self.can_lift_original_chime(object))
            || object.organism_id.is_some_and(|organism| {
                self.organism_registry
                    .get(organism)
                    .is_some_and(|r| !r.lifecycle().is_alive())
            })
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        Ok(object)
    }

    pub fn begin_player_hold(&mut self, id: WorldEntityId) -> Result<Vec3f, ScaffoldContractError> {
        let ground = self.player_hold_target(id)?.position;
        self.player_hold = Some(PlayerHold {
            object_id: id,
            last_ground: ground,
        });
        match self.move_player_hold_to(ground, true) {
            Ok(position) => {
                self.toy_rolls.remove(&id.raw());
                if self.player_holds_original_chime()
                    && self.last_chime_strike.is_some_and(|(_, struck)| struck == id)
                {
                    // The real old strike belongs to its old support.
                    self.last_chime_strike = None;
                }
                Ok(position)
            }
            Err(error) => {
                self.player_hold = None;
                Err(error)
            }
        }
    }

    pub fn move_player_hold(&mut self, ground: Vec3f) -> Result<Vec3f, ScaffoldContractError> {
        self.move_player_hold_to(ground, false)
    }

    fn move_player_hold_to(
        &mut self,
        mut ground: Vec3f,
        initial_pickup: bool,
    ) -> Result<Vec3f, ScaffoldContractError> {
        ground.validate()?;
        let hold = self
            .player_hold
            .as_ref()
            .ok_or(ScaffoldContractError::InvalidActionDecision)?;
        let id = hold.object_id;
        if let Some(terrain) = &self.terrain {
            // A release location must support the object. Moving through the air
            // may cross rocks/rivers, but dropping into them is not silently allowed.
            if !terrain.walkable(ground.x, ground.z) {
                return Err(ScaffoldContractError::InvalidId);
            }
            ground.y = terrain
                .surface()
                .height(ground.x, ground.z)
                .ok_or(ScaffoldContractError::InvalidId)?;
        } else {
            ground.z = 0.0;
        }
        let chime = self.player_holds_original_chime();
        if chime && self.chime_clear_foot(id, ground) != Some(ground) {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        // Keep initial pickup and unchanged projected ground available for
        // recovery. Check only the destination, allowing carrying across rocks.
        if !initial_pickup && ground != hold.last_ground {
            let object = self.entity(id).ok_or(ScaffoldContractError::InvalidId)?;
            let blocked = if chime {
                !self.chime_clear_transfer(id, hold.last_ground, ground)
            } else if object.kind == WorldObjectKind::Agent {
                // Air carrying is endpoint-only. An old save/editor overlap may
                // be rescued without deepening it or creating another overlap.
                // Do not use the escape-aware motion query for static placement.
                let body_radius = self.locomotion_body_radius();
                self.objects.values().any(|obstacle| {
                    if obstacle.kind != WorldObjectKind::Obstacle || obstacle.consumed {
                        return false;
                    }
                    let radius_squared = f64::from(obstacle.radius + body_radius).powi(2);
                    let previous = sphere_distance_squared(hold.last_ground, obstacle.position);
                    let next = sphere_distance_squared(ground, obstacle.position);
                    next <= radius_squared && (previous > radius_squared || next < previous)
                })
            } else {
                self.objects
                    .values()
                    .any(|obstacle| obstacle.blocks_position(ground))
            };
            if blocked {
                return Err(ScaffoldContractError::InvalidId);
            }
            if let Some(owner) = object.organism_id {
                let carries = self
                    .objects
                    .values()
                    .any(|item| item.carried_by == Some(owner));
                // Match the static reach used by carried_pose_reachable without
                // sweeping from the old grip, which belongs to the air carry.
                if carries
                    && !self.solid_segment_clear(ground, Self::carry_grip(ground, object.body_yaw))
                {
                    return Err(ScaffoldContractError::InvalidId);
                }
            }
        }
        let mut raised = ground;
        if self.terrain.is_some() {
            raised.y += 1.3;
        } else {
            raised.z += 1.3;
        }
        self.set_player_object_position(id, raised)?;
        self.player_hold.as_mut().unwrap().last_ground = ground;
        Ok(raised)
    }

    pub fn release_player_hold(&mut self) -> Result<Option<WorldEntityId>, ScaffoldContractError> {
        let Some(hold) = self.player_hold.clone() else {
            return Ok(None);
        };
        if self.player_holds_original_chime()
            && self.chime_clear_foot(hold.object_id, hold.last_ground) != Some(hold.last_ground)
        {
            // A living body may enter after the last approved position.
            // Preserve the same hold: no set-down into it or onto new ground.
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        if self.entity(hold.object_id).is_some() {
            self.set_player_object_position(hold.object_id, hold.last_ground)?;
        }
        self.player_hold = None;
        Ok(Some(hold.object_id))
    }

    pub(super) fn validate_player_hold(&self) -> Result<(), ScaffoldContractError> {
        let Some(hold) = &self.player_hold else {
            return Ok(());
        };
        let object = self
            .entity(hold.object_id)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let ground = hold.last_ground;
        if CHIME_PLACEMENT_REVIEW && object.kind == WorldObjectKind::ActivityToy
            && (!self.original_player_chime(object) || self.chime_terrain_foot(ground) != Some(ground))
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        // An independent occupant can refuse set-down after reopen, but
        // does not invalidate the recorded, geometrically supported hold.
        let mut expected = ground;
        if let Some(terrain) = &self.terrain {
            if !terrain.walkable(ground.x, ground.z)
                || terrain
                    .surface()
                    .height(ground.x, ground.z)
                    .is_none_or(|h| (h - ground.y).abs() > 0.001)
            {
                return Err(ScaffoldContractError::InvalidId);
            }
            expected.y += 1.3;
        } else {
            if ground.z != 0.0 {
                return Err(ScaffoldContractError::InvalidId);
            }
            expected.z += 1.3;
        }
        let error = subtract(expected, object.position);
        if error.x * error.x + error.y * error.y + error.z * error.z > 0.000001 {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }

    fn set_player_object_position(
        &mut self,
        id: WorldEntityId,
        position: Vec3f,
    ) -> Result<(), ScaffoldContractError> {
        let object = self
            .objects
            .get_mut(&id.raw())
            .ok_or(ScaffoldContractError::InvalidId)?;
        let displacement = subtract(position, object.position);
        if displacement.x * displacement.x
            + displacement.y * displacement.y
            + displacement.z * displacement.z
            < 0.000001
        {
            return Ok(());
        }
        let organism = object.organism_id;
        object.position = position;
        object.grounded_physical.velocity = Vec3f::ZERO;
        if let Some(organism) = organism {
            self.sync_carried_pose(organism)?;
        }
        self.rebuild_ecology_metrics();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_fixture(terrain: bool) -> HeadlessWorld {
        let mut world = HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(2.0, 0.0, 0.0), 0.5)
            .toy("ball", Vec3f::new(4.0, 0.0, 0.0), true)
            .token("token", Vec3f::new(6.0, 0.0, 0.0), 1)
            .food("unrelated", Vec3f::new(8.0, 0.0, 0.0), 0.5)
            .build()
            .unwrap();
        if terrain {
            world
                .enable_terrain_for_new_game(crate::island_terrain(), Vec3f::new(80.0, 0.0, 250.0))
                .unwrap();
        }
        world
    }

    fn portable_roundtrip(world: &HeadlessWorld) -> HeadlessWorld {
        use crate::persistence::{AssetManifest, PortableSaveFile, RuntimeConfig};
        let save = PortableSaveFile::from_headless_world(
            "player-hold-editor",
            world,
            RuntimeConfig::deterministic_default(world.seed(), alife_core::BrainScaleTier::Nano512),
            AssetManifest::empty(),
            vec![],
        )
        .unwrap();
        PortableSaveFile::from_json_str(&save.to_json_string_pretty().unwrap())
            .unwrap()
            .restore_headless_world()
            .expect("editor operations must leave a restorable world")
    }

    #[test]
    fn editor_move_cannot_displace_a_player_held_object_or_break_its_save() {
        for terrain in [false, true] {
            for held_label in ["walker", "food", "ball", "token"] {
                let mut world = editor_fixture(terrain);
                let held = world.entity_id(held_label).unwrap();
                world.begin_player_hold(held).unwrap();
                let unrelated = world.entity_id("unrelated").unwrap();
                let position = world.entity(unrelated).unwrap().position;
                world
                    .editor_move_object(
                        unrelated,
                        Vec3f::new(position.x + 1.0, position.y, position.z),
                    )
                    .unwrap();
                assert_eq!(world.player_held_object(), Some(held));
                let before = world.save_state();
                let position = world.entity(held).unwrap().position;
                let destination = if terrain {
                    Vec3f::new(position.x + 2.0, 0.0, position.z)
                } else {
                    Vec3f::new(position.x + 2.0, position.y, 0.0)
                };
                let result = world.editor_move_object(held, destination);
                let mut restored = portable_roundtrip(&world);
                assert!(result.is_err());
                assert_eq!(world.save_state(), before);
                assert_eq!(restored.save_state(), before);
                assert_eq!(restored.release_player_hold().unwrap(), Some(held));
                restored.editor_move_object(held, destination).unwrap();
                let restored = portable_roundtrip(&restored);
                assert_eq!(restored.entity(held).unwrap().position.x, destination.x);
            }
        }
    }

    #[test]
    fn removing_an_agent_clears_only_its_own_player_hold_before_save() {
        for registered in [false, true] {
            for held_label in ["walker", "food"] {
                let organism = OrganismId(1);
                let mut world = HeadlessScenarioBuilder::new(17)
                    .agent("walker", organism, Vec3f::ZERO)
                    .food("food", Vec3f::new(2.0, 0.0, 0.0), 0.5)
                    .build()
                    .unwrap();
                let agent = world.entity_id("walker").unwrap();
                let held = world.entity_id(held_label).unwrap();
                world.begin_player_hold(held).unwrap();
                if registered {
                    let genome = alife_core::CreatureGenome::early_mammal_founder(
                        17,
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
                    let mut record = WorldOrganismRecord::newborn(
                        organism,
                        agent,
                        genome,
                        phenotype,
                        Tick::ZERO,
                    )
                    .unwrap();
                    record.mark_dead(Tick::ZERO).unwrap();
                    record
                        .link_birth_manifest(alife_core::Blake3Digest::from_bytes([1; 32]))
                        .unwrap();
                    record
                        .link_life_manifest(alife_core::Blake3Digest::from_bytes([2; 32]))
                        .unwrap();
                    world.register_organism_record(record).unwrap();
                    world.retire_dead_organism(organism).unwrap();
                } else {
                    world.remove_agent_entity(agent).unwrap();
                }
                let expected = (held != agent).then_some(held);
                let mut restored = HeadlessWorld::from_save_state(&world.save_state()).unwrap();
                assert_eq!(world.player_held_object(), expected);
                assert_eq!(restored.player_held_object(), expected);
                assert_eq!(restored.release_player_hold().unwrap(), expected);
                assert!(restored.can_begin_player_hold(world.entity_id("food").unwrap()));
            }
        }
    }

    #[test]
    fn player_can_pick_move_persist_and_release_creatures_and_movable_objects() {
        let mut world = HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(2.0, 0.0, 0.0), 0.5)
            .toy("ball", Vec3f::new(4.0, 0.0, 0.0), true)
            .token("token", Vec3f::new(6.0, 0.0, 0.0), 1)
            .build()
            .unwrap();
        for label in ["walker", "food", "ball", "token"] {
            let id = world.entity_id(label).unwrap();
            assert!(world.can_begin_player_hold(id), "{label}");
            let original = world.entity(id).unwrap().position;
            let raised = world.begin_player_hold(id).unwrap();
            assert_eq!(raised, Vec3f::new(original.x, original.y, 1.3));
            assert!(!world.can_begin_player_hold(id));
            let destination = Vec3f::new(original.x + 1.0, original.y + 2.0, 0.0);
            world.move_player_hold(destination).unwrap();
            assert_eq!(world.entity(id).unwrap().position.z, 1.3);
            assert_eq!(
                world.entity(id).unwrap().grounded_physical.velocity,
                Vec3f::ZERO
            );
            let mut restored = HeadlessWorld::from_save_state(&world.save_state()).unwrap();
            assert_eq!(restored.player_held_object(), Some(id));
            assert_eq!(
                restored.entity(id).unwrap().position,
                world.entity(id).unwrap().position
            );
            assert_eq!(restored.release_player_hold().unwrap(), Some(id));
            assert_eq!(restored.entity(id).unwrap().position, destination);
            assert_eq!(world.release_player_hold().unwrap(), Some(id));
            assert_eq!(world.entity(id).unwrap().position, destination);
        }
    }

    #[test]
    fn fixed_scenery_missing_and_unavailable_objects_are_not_pickup_targets() {
        let mut world = HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .toy("fixed toy", Vec3f::new(2.0, 0.0, 0.0), false)
            .obstacle("rock", Vec3f::new(4.0, 0.0, 0.0), 0.5)
            .hazard("hazard", Vec3f::new(6.0, 0.0, 0.0), 0.5)
            .food("food", Vec3f::new(8.0, 0.0, 0.0), 0.5)
            .build()
            .unwrap();
        for label in ["fixed toy", "rock", "hazard"] {
            let id = world.entity_id(label).unwrap();
            assert!(!world.can_begin_player_hold(id), "{label}");
            assert!(world.begin_player_hold(id).is_err(), "{label}");
        }
        assert!(!world.can_begin_player_hold(WorldEntityId(u64::MAX)));
        let food = world.entity_id("food").unwrap();
        world.objects.get_mut(&food.raw()).unwrap().consumed = true;
        assert!(!world.can_begin_player_hold(food));
        let object = world.objects.get_mut(&food.raw()).unwrap();
        object.consumed = false;
        object.carried_by = Some(OrganismId(1));
        assert!(!world.can_begin_player_hold(food));
        assert_eq!(world.player_held_object(), None);
    }

    #[test]
    fn player_carry_blocks_self_movement_and_releases_on_selected_terrain() {
        let mut world = HeadlessScenarioBuilder::new(17)
            .agent("walker", OrganismId(1), Vec3f::ZERO)
            .build()
            .unwrap();
        world
            .enable_terrain_for_new_game(crate::island_terrain(), Vec3f::new(80.0, 0.0, 250.0))
            .unwrap();
        let id = world.entity_id("walker").unwrap();
        let original = world.entity(id).unwrap().position;
        let before = world.canonical_signature_digest().unwrap();
        world.begin_player_hold(id).unwrap();
        assert_eq!(world.player_held_object(), Some(id));
        assert_eq!(world.entity(id).unwrap().position.y, original.y + 1.3);
        assert_ne!(before, world.canonical_signature_digest().unwrap());
        let command = HeadlessWorldCommand::approach(OrganismId(1), id).unwrap();
        assert!(!world.apply_command(&command).unwrap().execution.succeeded);
        assert!(world
            .move_player_hold(Vec3f::new(-500.0, 0.0, -600.0))
            .is_err());
        let restored = HeadlessWorld::from_save_state(&world.save_state()).unwrap();
        assert_eq!(restored.player_held_object(), Some(id));
        world.release_player_hold().unwrap();
        assert_eq!(world.entity(id).unwrap().position, original);
    }
}
