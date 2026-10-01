//! Player carrying is a world-owned physical constraint, never a neural action.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerHold {
    pub object_id: WorldEntityId,
    pub last_ground: Vec3f,
}

impl HeadlessWorld {
    pub fn player_held_object(&self) -> Option<WorldEntityId> {
        self.player_hold.as_ref().map(|hold| hold.object_id)
    }

    pub fn begin_player_hold(&mut self, id: WorldEntityId) -> Result<Vec3f, ScaffoldContractError> {
        if self.player_hold.is_some() {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        let object = self.entity(id).ok_or(ScaffoldContractError::InvalidId)?;
        if object.consumed
            || object.carried_by.is_some()
            || !matches!(
                object.kind,
                WorldObjectKind::Agent
                    | WorldObjectKind::Food
                    | WorldObjectKind::Ball
                    | WorldObjectKind::Token
            )
            || object.organism_id.is_some_and(|organism| {
                self.organism_registry
                    .get(organism)
                    .is_some_and(|r| !r.lifecycle().is_alive())
            })
        {
            return Err(ScaffoldContractError::InvalidActionDecision);
        }
        let ground = object.position;
        self.player_hold = Some(PlayerHold {
            object_id: id,
            last_ground: ground,
        });
        match self.move_player_hold(ground) {
            Ok(position) => Ok(position),
            Err(error) => {
                self.player_hold = None;
                Err(error)
            }
        }
    }

    pub fn move_player_hold(&mut self, mut ground: Vec3f) -> Result<Vec3f, ScaffoldContractError> {
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
            self.move_carried_objects(organism, displacement);
        }
        self.rebuild_ecology_metrics();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let restored = HeadlessWorld::from_persistence_parts(world.persistence_parts()).unwrap();
        assert_eq!(restored.player_held_object(), Some(id));
        world.release_player_hold().unwrap();
        assert_eq!(world.entity(id).unwrap().position, original);
    }
}
