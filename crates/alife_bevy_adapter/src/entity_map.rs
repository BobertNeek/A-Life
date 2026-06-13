//! v0 scaffold: stable Bevy `Entity` to core `WorldEntityId` mapping.

use std::collections::HashMap;

use alife_core::{ScaffoldContractError, WorldEntityId, WorldEntityIdMapper};
use bevy::prelude::{Entity, Resource};

#[derive(Debug, Clone, Resource)]
pub struct BevyEntityMap {
    next_id: u64,
    by_entity: HashMap<Entity, WorldEntityId>,
    by_world_id: HashMap<WorldEntityId, Entity>,
}

impl Default for BevyEntityMap {
    fn default() -> Self {
        Self::with_next_id(1)
    }
}

impl BevyEntityMap {
    pub fn with_next_id(next_id: u64) -> Self {
        Self {
            next_id: next_id.max(1),
            by_entity: HashMap::new(),
            by_world_id: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.by_entity.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_entity.is_empty()
    }

    pub fn world_id(&self, entity: Entity) -> Option<WorldEntityId> {
        self.by_entity.get(&entity).copied()
    }

    pub fn bevy_entity(&self, world_id: WorldEntityId) -> Option<Entity> {
        self.by_world_id.get(&world_id).copied()
    }

    pub fn bind(
        &mut self,
        entity: Entity,
        world_id: WorldEntityId,
    ) -> Result<(), ScaffoldContractError> {
        world_id.validate()?;
        if let Some(previous_id) = self.by_entity.insert(entity, world_id) {
            self.by_world_id.remove(&previous_id);
        }
        if let Some(previous_entity) = self.by_world_id.insert(world_id, entity) {
            if previous_entity != entity {
                self.by_entity.remove(&previous_entity);
            }
        }
        self.next_id = self.next_id.max(world_id.raw().saturating_add(1));
        Ok(())
    }

    pub fn get_or_allocate(
        &mut self,
        entity: Entity,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        if let Some(id) = self.world_id(entity) {
            return Ok(id);
        }

        let id = loop {
            let id = WorldEntityId::new(self.next_id).ok_or(ScaffoldContractError::InvalidId)?;
            if !self.by_world_id.contains_key(&id) {
                break id;
            }
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or(ScaffoldContractError::InvalidId)?;
        };
        self.next_id = self.next_id.saturating_add(1);
        self.bind(entity, id)?;
        Ok(id)
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

impl WorldEntityIdMapper<Entity> for BevyEntityMap {
    fn to_world_entity_id(&self, entity: Entity) -> WorldEntityId {
        self.world_id(entity).unwrap_or(WorldEntityId::INVALID)
    }
}
