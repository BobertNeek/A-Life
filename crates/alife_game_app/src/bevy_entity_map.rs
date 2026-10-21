//! Stable correspondence between authoritative world IDs and rendered entities.

use std::collections::HashMap;

use alife_core::{ScaffoldContractError, WorldEntityId};
use bevy::prelude::{Entity, Resource};

#[derive(Debug, Default, Resource)]
pub struct BevyEntityMap {
    by_entity: HashMap<Entity, WorldEntityId>,
    by_world_id: HashMap<WorldEntityId, Entity>,
}

impl BevyEntityMap {
    pub fn bevy_entity(&self, world_id: WorldEntityId) -> Option<Entity> {
        self.by_world_id.get(&world_id).copied()
    }

    pub fn bind(
        &mut self,
        entity: Entity,
        world_id: WorldEntityId,
    ) -> Result<(), ScaffoldContractError> {
        world_id.validate()?;
        if self.by_entity.get(&entity) == Some(&world_id)
            && self.by_world_id.get(&world_id) == Some(&entity)
        {
            return Ok(());
        }
        if let Some(previous_id) = self.by_entity.insert(entity, world_id) {
            self.by_world_id.remove(&previous_id);
        }
        if let Some(previous_entity) = self.by_world_id.insert(world_id, entity) {
            if previous_entity != entity {
                self.by_entity.remove(&previous_entity);
            }
        }
        Ok(())
    }

    pub fn remove_by_entity(&mut self, entity: Entity) -> Option<WorldEntityId> {
        let id = self.by_entity.remove(&entity)?;
        self.by_world_id.remove(&id);
        Some(id)
    }

    pub fn remove_by_world_id(&mut self, world_id: WorldEntityId) -> Option<Entity> {
        let entity = self.by_world_id.remove(&world_id)?;
        self.by_entity.remove(&entity);
        Some(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::World;

    #[test]
    fn identity_bindings_are_bidirectional_across_rebind_and_removal() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let mut map = BevyEntityMap::default();
        map.bind(first, WorldEntityId(40)).unwrap();
        map.bind(second, WorldEntityId(41)).unwrap();
        map.bind(first, WorldEntityId(40)).unwrap();
        assert_eq!(map.bevy_entity(WorldEntityId(40)), Some(first));
        assert_eq!(map.bevy_entity(WorldEntityId(41)), Some(second));

        map.bind(first, WorldEntityId(42)).unwrap();
        assert_eq!(map.bevy_entity(WorldEntityId(40)), None);
        map.bind(second, WorldEntityId(42)).unwrap();
        assert_eq!(map.remove_by_entity(first), None);
        assert_eq!(map.bevy_entity(WorldEntityId(41)), None);
        assert_eq!(map.bevy_entity(WorldEntityId(42)), Some(second));
        assert!(map.bind(first, WorldEntityId::INVALID).is_err());
        assert_eq!(map.bevy_entity(WorldEntityId(42)), Some(second));

        assert_eq!(map.remove_by_world_id(WorldEntityId(42)), Some(second));
        assert_eq!(map.remove_by_entity(second), None);
        map.bind(first, WorldEntityId(40)).unwrap();
        assert_eq!(map.remove_by_entity(first), Some(WorldEntityId(40)));
        assert_eq!(map.bevy_entity(WorldEntityId(40)), None);
    }
}
