//! The Hand supplies one bounded shove. A loose ball then travels on world ticks;
//! neither the gesture nor hitting a chime table is a creature action.
use super::*;

const MAX_TRAVEL: f32 = 6.0;
const SETTLE: f32 = 0.045;

impl HeadlessWorld {
    pub fn can_roll_toy(&self, id: WorldEntityId) -> bool {
        self.player_held_object().is_none() && self.loose_ball(id)
    }

    fn loose_ball(&self, id: WorldEntityId) -> bool {
        self.entity(id).is_some_and(|o| {
            o.kind == WorldObjectKind::Ball
                && !o.consumed
                && o.carried_by.is_none()
                && !o.activity_hold
                && self.player_held_object() != Some(id)
        })
    }

    /// X/Z offset in ground metres, not an absolute destination and not a command to an animal.
    pub fn roll_player_toy(
        &mut self,
        id: WorldEntityId,
        offset: [f32; 2],
    ) -> Result<(), ScaffoldContractError> {
        let distance = offset[0].hypot(offset[1]);
        if !self.can_roll_toy(id) || !distance.is_finite() || distance < SETTLE {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        let scale = (MAX_TRAVEL / distance).min(1.0);
        self.toy_rolls.insert(id.raw(), offset.map(|v| v * scale));
        Ok(())
    }

    /// Presentation may witness the latest table struck by a rolling object. This ephemeral
    /// receipt is not an organism reward, not a sound delivered to an ear, and not replayed on load.
    pub fn last_chime_strike(&self) -> Option<(Tick, WorldEntityId)> {
        self.last_chime_strike
    }

    // Collision is the first sphere entry on the actual ground triangles, not the
    // straight chord joining their endpoints. Moving out of an existing overlap is legal.
    fn roll_step(
        &self,
        ball: &WorldObject,
        offset: [f32; 2],
    ) -> Option<(Vec3f, Option<WorldEntityId>)> {
        let start = ball.position;
        let requested = add(start, Vec3f::new(offset[0], 0., offset[1]));
        let sweep = |a: Vec3f, b: Vec3f| {
            self.objects
                .values()
                .filter(|other| {
                    other.id != ball.id
                        && !other.consumed
                        && matches!(
                            other.kind,
                            WorldObjectKind::Obstacle
                                | WorldObjectKind::ActivityToy
                                | WorldObjectKind::Agent
                                | WorldObjectKind::Ball
                        )
                })
                .filter_map(|other| {
                    motion_sphere_entry(a, b, other.position, ball.radius + other.radius)
                        .map(|t| (t, other.id))
                })
                .min_by(|(at, ai), (bt, bi)| at.total_cmp(bt).then_with(|| ai.raw().cmp(&bi.raw())))
                .map(|(t, id)| (add(a, scale(subtract(b, a), t as f32)), Some(id)))
        };
        if let Some(terrain) = &self.terrain {
            let mut limits = terrain.limits();
            limits.body_radius = ball.radius;
            let surface = terrain.surface();
            let end = surface.resolve_move_with(start, requested, limits)?;
            let point = |p: [f64; 3]| Vec3f::new(p[0] as f32, p[1] as f32, p[2] as f32);
            for segment in surface.grounded_sweep_segments(start, end)? {
                if let Some(hit) = sweep(point(segment.start), point(segment.end)) {
                    return Some(hit);
                }
            }
            Some((end, None))
        } else {
            Some(sweep(start, requested).unwrap_or((requested, None)))
        }
    }

    pub(super) fn advance_toy_rolls(&mut self) {
        if self.toy_rolls.is_empty() {
            return;
        }
        let mut rolls = std::mem::take(&mut self.toy_rolls);
        rolls.retain(|&raw, remaining| {
            let id = WorldEntityId(raw);
            if !self.loose_ball(id) {
                return false;
            }
            let ball = &self.objects[&raw];
            let start = ball.position;
            let finishing = remaining[0].hypot(remaining[1]) <= SETTLE;
            let step = remaining.map(|v| if finishing { v } else { v * 0.11 });
            let motion = self.roll_step(ball, step);
            if let Some(hit) = motion.and_then(|(_, hit)| hit).filter(|&id| {
                self.entity(id)
                    .is_some_and(|o| o.kind == WorldObjectKind::ActivityToy)
            }) {
                self.last_chime_strike = Some((self.tick, hit));
            }
            let ball = self
                .objects
                .get_mut(&raw)
                .expect("loose ball was validated");
            let (end, hit) = motion.unwrap_or((start, None));
            ball.position = end;
            let rolling = motion.is_some() && hit.is_none() && !finishing;
            ball.grounded_physical.velocity = if rolling {
                subtract(end, start)
            } else {
                Vec3f::ZERO
            };
            remaining[0] -= step[0];
            remaining[1] -= step[1];
            rolling
        });
        self.toy_rolls = rolls;
    }
}

impl crate::WorldSaveState {
    pub(crate) fn validate_toy_rolls(&self) -> Result<(), ScaffoldContractError> {
        if self.toy_rolls.is_empty() {
            return Ok(());
        }
        let eligible = self
            .objects
            .iter()
            .filter(|o| {
                self.toy_rolls.contains_key(&o.id.raw())
                    && o.kind == WorldObjectKind::Ball
                    && !o.consumed
                    && o.carried_by.is_none()
                    && !o.activity_hold
                    && !self
                        .player_hold
                        .as_ref()
                        .is_some_and(|held| held.object_id == o.id)
            })
            .count();
        let speeds_valid = self
            .toy_rolls
            .values()
            .all(|&[x, z]| (f32::MIN_POSITIVE..=MAX_TRAVEL + 0.00001).contains(&x.hypot(z)));
        if eligible == self.toy_rolls.len() && speeds_valid {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidActionDecision)
        }
    }
}

#[cfg(test)]
mod tests;
