//! Autonomous edible critters advance only with native world ticks. Their controller
//! perceives nearby visible agents; it never knows goals, learner decisions or weights.
use super::*;
use crate::ecology::{AutonomousCritterState, CritterMode};

fn random_unit(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    ((*state >> 40) as u32) as f32 / 16_777_216.0
}
impl HeadlessWorld {
    pub fn enable_autonomous_critter(
        &mut self,
        object_id: WorldEntityId,
        seed: u64,
        speed_per_tick: f32,
        sight_radius: f32,
    ) -> Result<(), ScaffoldContractError> {
        let o = self
            .entity(object_id)
            .ok_or(ScaffoldContractError::InvalidId)?;
        if o.kind != WorldObjectKind::Food
            || o.organism_id.is_some()
            || o.teacher_channel.is_some()
            || o.consumed
            || self
                .ecology
                .autonomous_critters
                .iter()
                .any(|c| c.object_id == object_id)
            || self.ecology.autonomous_critters.len() >= self.ecology.config.max_resource_records
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        let mut c = AutonomousCritterState {
            object_id,
            rng_state: seed,
            speed_per_tick,
            sight_radius,
            heading: 0.0,
            turn_in_ticks: 0,
            last_tick: self.tick,
            last_threat: None,
            last_mode: CritterMode::Wander,
            last_before: o.position,
            last_after: o.position,
            moved_ticks: 0,
            fleeing_ticks: 0,
            blocked_ticks: 0,
        };
        c.validate()?;
        c.heading = random_unit(&mut c.rng_state) * std::f32::consts::TAU;
        self.ecology.autonomous_critters.push(c);
        Ok(())
    }
    pub fn autonomous_critter(&self, id: WorldEntityId) -> Option<&AutonomousCritterState> {
        self.ecology
            .autonomous_critters
            .iter()
            .find(|c| c.object_id == id)
    }
    fn critter_destination(&self, o: &WorldObject, heading: f32, step: f32) -> Option<Vec3f> {
        let requested = add(
            o.position,
            Vec3f::new(heading.cos() * step, 0.0, heading.sin() * step),
        );
        let (end, segments) = if let Some(terrain) = &self.terrain {
            let mut limits = terrain.limits();
            limits.body_radius = o.radius;
            let end = terrain
                .surface()
                .resolve_move_with(o.position, requested, limits)?;
            let segments = terrain.surface().grounded_sweep_segments(o.position, end)?;
            (
                end,
                segments
                    .into_iter()
                    .map(|s| {
                        (
                            Vec3f::new(s.start[0] as f32, s.start[1] as f32, s.start[2] as f32),
                            Vec3f::new(s.end[0] as f32, s.end[1] as f32, s.end[2] as f32),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            (requested, vec![(o.position, requested)])
        };
        for (start, end) in segments {
            if self.objects.values().any(|b| {
                b.id != o.id
                    && !b.consumed
                    && b.kind == WorldObjectKind::Obstacle
                    && motion_sphere_entry(start, end, b.position, b.radius + o.radius).is_some()
            }) {
                return None;
            }
        }
        Some(end)
    }
    /// Omnidirectional sight uses the same optical/heightfield occlusion query
    /// as native vision, not the opacity-independent physical reach query.
    fn critter_can_see(&self, observer: &WorldObject, target: &WorldObject) -> bool {
        let eye = add(
            observer.position,
            Vec3f::new(0.0, observer.radius * 0.5, 0.0),
        );
        let target_point = add(target.position, Vec3f::new(0.0, target.radius * 0.5, 0.0));
        let range = distance(eye, target_point);
        if range <= f32::EPSILON {
            return true;
        }
        let direction = scale(subtract(target_point, eye), 1.0 / range);
        self.first_sight_hit(eye, direction, range, observer.id, Some(target.id))
            .is_none_or(|hit| hit >= range - target.radius * 0.25)
    }
    pub(super) fn advance_autonomous_critters(&mut self) {
        // Decisions see one shared pre-movement snapshot; stable vector order is persisted.
        let mut states = self.ecology.autonomous_critters.clone();
        let mut moves = Vec::new();
        for c in &mut states {
            if c.last_tick == self.tick {
                continue;
            }
            let Some(o) = self.entity(c.object_id) else {
                continue;
            };
            c.last_tick = self.tick;
            c.last_before = o.position;
            c.last_after = o.position;
            c.last_threat = None;
            if o.consumed {
                c.last_mode = CritterMode::Consumed;
                continue;
            }
            if o.carried_by.is_some() || o.activity_hold || self.player_held_object() == Some(o.id)
            {
                c.last_mode = CritterMode::Held;
                continue;
            }
            // Omnidirectional, bounded sight; opaque walls/terrain occlude threats.
            let threat = self
                .objects
                .values()
                .filter(|a| a.kind == WorldObjectKind::Agent && !a.consumed)
                .filter_map(|a| {
                    let d = distance(o.position, a.position);
                    (d <= c.sight_radius && self.critter_can_see(o, a))
                        .then_some((d, a.id, a.position))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.raw().cmp(&b.1.raw())));
            let step = if let Some((d, id, p)) = threat {
                c.last_threat = Some(id);
                c.last_mode = CritterMode::Flee;
                c.fleeing_ticks = c.fleeing_ticks.saturating_add(1);
                if d > 1e-5 {
                    c.heading = (o.position.z - p.z).atan2(o.position.x - p.x);
                }
                c.speed_per_tick
            } else {
                c.last_mode = CritterMode::Wander;
                if c.turn_in_ticks == 0 {
                    c.heading += (random_unit(&mut c.rng_state) - 0.5) * std::f32::consts::PI;
                    c.turn_in_ticks = 16;
                }
                c.turn_in_ticks -= 1;
                c.speed_per_tick * 0.35
            };
            let side = if random_unit(&mut c.rng_state) < 0.5 {
                -1.0
            } else {
                1.0
            };
            let choices = [0.0, side * 0.65, -side * 0.65, side * 1.3, -side * 1.3];
            let movement = choices.iter().find_map(|angle| {
                let heading = c.heading + angle;
                self.critter_destination(o, heading, step)
                    .map(|p| (p, heading))
            });
            if let Some((p, heading)) = movement {
                c.heading = heading;
                c.last_after = p;
                c.moved_ticks = c
                    .moved_ticks
                    .saturating_add(u64::from(distance(p, o.position) > 1e-6));
                moves.push((o.id, p, subtract(p, o.position)));
            } else {
                c.last_mode = CritterMode::Blocked;
                c.blocked_ticks = c.blocked_ticks.saturating_add(1);
                c.heading += side * std::f32::consts::FRAC_PI_2;
                moves.push((o.id, o.position, Vec3f::ZERO));
            }
        }
        for (id, p, velocity) in moves {
            if let Some(o) = self.objects.get_mut(&id.raw()) {
                o.position = p;
                o.grounded_physical.velocity = velocity;
            }
        }
        self.ecology.autonomous_critters = states;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (HeadlessWorld, WorldEntityId) {
        let w = HeadlessScenarioBuilder::new(991)
            .food("critter", Vec3f::ZERO, 0.6)
            .build()
            .unwrap();
        let id = w
            .object_snapshots()
            .into_iter()
            .find(|o| o.label == "critter")
            .unwrap()
            .id;
        (w, id)
    }
    #[test]
    fn autonomous_critter_wanders_and_roundtrips_its_own_rng() {
        let (mut w, id) = fixture();
        w.enable_autonomous_critter(id, 12345, 0.04, 4.0).unwrap();
        for _ in 0..12 {
            w.advance_tick();
        }
        assert!(w.autonomous_critter(id).unwrap().moved_ticks > 0);
        let wire = serde_json::to_vec(&w.save_state()).unwrap();
        let save: crate::WorldSaveState = serde_json::from_slice(&wire).unwrap();
        let mut restored = HeadlessWorld::from_save_state(&save).unwrap();
        for _ in 0..40 {
            w.advance_tick();
            restored.advance_tick();
        }
        assert_eq!(w.entity(id), restored.entity(id));
        assert_eq!(w.ecology(), restored.ecology());
    }
    #[test]
    fn autonomous_critter_reacts_to_visible_threat_but_not_through_wall() {
        let (mut w, id) = fixture();
        w.spawn_social_agent("threat", OrganismId(99), Vec3f::new(-2.0, 0.0, 0.0), 0.0)
            .unwrap();
        w.enable_autonomous_critter(id, 7, 0.04, 4.0).unwrap();
        w.advance_tick();
        let c = w.autonomous_critter(id).unwrap();
        assert_eq!(c.last_mode, CritterMode::Flee);
        assert!(w.entity(id).unwrap().position.x > 0.039);
        let mut hidden = HeadlessScenarioBuilder::new(991)
            .food("critter", Vec3f::ZERO, 0.6)
            .agent("threat", OrganismId(99), Vec3f::new(-2.0, 0.0, 0.0))
            .obstacle("wall", Vec3f::new(-1.0, 0.0, 0.0), 0.5)
            .build()
            .unwrap();
        let id = hidden
            .object_snapshots()
            .into_iter()
            .find(|o| o.label == "critter")
            .unwrap()
            .id;
        hidden.enable_autonomous_critter(id, 7, 0.04, 4.0).unwrap();
        hidden.advance_tick();
        assert_eq!(hidden.autonomous_critter(id).unwrap().last_threat, None);
        assert_eq!(
            hidden.autonomous_critter(id).unwrap().last_mode,
            CritterMode::Wander
        );
    }
    #[test]
    fn autonomous_critter_sight_respects_opacity_and_heightfield_ridges() {
        let (mut w, id) = fixture();
        let threat = w
            .spawn_social_agent("threat", OrganismId(99), Vec3f::new(2.0, 0.0, 0.0), 0.0)
            .unwrap();
        assert!(w.critter_can_see(w.entity(id).unwrap(), w.entity(threat).unwrap()));
        let wall = w
            .editor_spawn_object(crate::WorldEditorSpawnSpec {
                label: "optical-wall".into(),
                kind: WorldObjectKind::Obstacle,
                organism_id: None,
                position: Vec3f::new(1.0, 0.0, 0.0),
                radius: 0.5,
                nutrition: 0.0,
                hazard_pain: 0.0,
                token_id: None,
            })
            .unwrap();
        assert!(!w.critter_can_see(w.entity(id).unwrap(), w.entity(threat).unwrap()));
        w.objects.get_mut(&wall.raw()).unwrap().optical_opacity = 0.0;
        assert!(w.critter_can_see(w.entity(id).unwrap(), w.entity(threat).unwrap()));
        // The transparent wall is still a physical obstacle, but sight must not
        // reuse that reach test. An opaque heightfield ridge does hide the agent.
        assert!(!w.solid_segment_clear(
            w.entity(id).unwrap().position,
            w.entity(threat).unwrap().position
        ));
        w.terrain = Some(
            crate::WorldTerrain::new(
                crate::TerrainData {
                    width: 5,
                    depth: 2,
                    origin_x: -2.0,
                    origin_z: -0.5,
                    spacing: 1.0,
                    heights: vec![0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0],
                    water_level: None,
                    obstacles: vec![],
                },
                crate::LocomotionLimits::default(),
            )
            .unwrap(),
        );
        assert!(!w.critter_can_see(w.entity(id).unwrap(), w.entity(threat).unwrap()));
        w.enable_autonomous_critter(id, 7, 0.04, 4.0).unwrap();
        w.advance_tick();
        assert_eq!(w.autonomous_critter(id).unwrap().last_threat, None);
    }
    #[test]
    fn autonomous_critter_freezes_held_consumed_and_duplicate_tick() {
        let (mut w, id) = fixture();
        w.enable_autonomous_critter(id, 7, 0.04, 4.0).unwrap();
        let rng = w.autonomous_critter(id).unwrap().rng_state;
        w.objects.get_mut(&id.raw()).unwrap().activity_hold = true;
        w.advance_tick();
        assert_eq!(w.entity(id).unwrap().position, Vec3f::ZERO);
        assert_eq!(w.autonomous_critter(id).unwrap().rng_state, rng);
        assert_eq!(
            w.autonomous_critter(id).unwrap().last_mode,
            CritterMode::Held
        );
        w.objects.get_mut(&id.raw()).unwrap().activity_hold = false;
        w.objects.get_mut(&id.raw()).unwrap().consumed = true;
        w.advance_tick();
        assert_eq!(
            w.autonomous_critter(id).unwrap().last_mode,
            CritterMode::Consumed
        );
        let c = w.autonomous_critter(id).unwrap().clone();
        w.advance_autonomous_critters();
        assert_eq!(*w.autonomous_critter(id).unwrap(), c);
    }
    #[test]
    fn autonomous_critter_sweeps_obstacles_not_just_endpoints() {
        let mut w = HeadlessScenarioBuilder::new(991)
            .food("critter", Vec3f::ZERO, 0.6)
            .obstacle("wall", Vec3f::new(0.5, 0.0, 0.0), 0.2)
            .build()
            .unwrap();
        let id = w
            .object_snapshots()
            .into_iter()
            .find(|o| o.label == "critter")
            .unwrap()
            .id;
        w.objects.get_mut(&id.raw()).unwrap().radius = 0.1;
        assert!(w
            .critter_destination(w.entity(id).unwrap(), 0.0, 1.0)
            .is_none());
        assert!(w
            .critter_destination(w.entity(id).unwrap(), std::f32::consts::FRAC_PI_2, 1.0)
            .is_some());
    }
    #[test]
    fn autonomous_critter_legacy_ecology_wire_remains_unchanged() {
        let ecology = EcologyState::default();
        let bytes = serde_json::to_vec(&ecology).unwrap();
        assert!(!String::from_utf8(bytes.clone())
            .unwrap()
            .contains("autonomous_critters"));
        assert_eq!(
            serde_json::from_slice::<EcologyState>(&bytes).unwrap(),
            ecology
        );
        let (mut w, id) = fixture();
        assert!(w.enable_autonomous_critter(id, 0, 0.04, 4.0).is_err());
        w.enable_autonomous_critter(id, 7, 0.04, 4.0).unwrap();
        assert!(w.enable_autonomous_critter(id, 8, 0.04, 4.0).is_err());
    }
    #[test]
    fn autonomous_critter_respects_terrain_boxes_and_edges() {
        let (mut w, id) = fixture();
        // The generic builder food radius is larger than this narrow fixture.
        w.objects.get_mut(&id.raw()).unwrap().radius = 0.1;
        w.terrain = Some(
            crate::WorldTerrain::new(
                crate::TerrainData {
                    width: 2,
                    depth: 2,
                    origin_x: -4.0,
                    origin_z: -4.0,
                    spacing: 8.0,
                    heights: vec![0.0; 4],
                    water_level: None,
                    obstacles: vec![[0.4, -1.0, 0.6, 1.0, -1.0, 1.0]],
                },
                crate::LocomotionLimits::default(),
            )
            .unwrap(),
        );
        assert!(w
            .critter_destination(w.entity(id).unwrap(), 0.0, 1.0)
            .is_none());
        assert!(w
            .critter_destination(w.entity(id).unwrap(), std::f32::consts::PI, 1.0)
            .is_some());
        assert!(w
            .critter_destination(w.entity(id).unwrap(), std::f32::consts::PI, 5.0)
            .is_none());
    }
    #[test]
    fn autonomous_critter_rejects_dangling_and_future_saved_state() {
        let (mut w, id) = fixture();
        w.enable_autonomous_critter(id, 7, 0.04, 4.0).unwrap();
        let mut state = w.save_state();
        state.ecology.autonomous_critters[0].object_id = WorldEntityId(999999);
        assert!(HeadlessWorld::from_save_state(&state).is_err());
        state = w.save_state();
        state.ecology.autonomous_critters[0].last_tick = Tick::new(1);
        assert!(HeadlessWorld::from_save_state(&state).is_err());
    }
}
