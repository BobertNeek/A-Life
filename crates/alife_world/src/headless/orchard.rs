use super::*;
use crate::orchard::{
    valid_coordinate, OrchardBed, OrchardGesture, OrchardReceipt, MAX_ORCHARD_BEDS,
};
use crate::VoxelTileCoord;

impl HeadlessWorld {
    fn soil_position(&self, tile: VoxelTileCoord) -> Option<Vec3f> {
        if !valid_coordinate(tile) {
            return None;
        }
        let (x, z) = (tile.x as f32 + 0.5, tile.z as f32 + 0.5);
        if self.terrain().is_some_and(|t| !t.walkable(x, z)) {
            return None;
        }
        if self.objects.values().any(|o| {
            let depth = if self.terrain().is_some() {
                o.position.z
            } else {
                o.position.y
            };
            !o.consumed
                && matches!(o.kind, WorldObjectKind::Obstacle | WorldObjectKind::Hazard)
                && (o.position.x - x).hypot(depth - z) <= o.radius + 0.42
        }) {
            return None;
        }
        if self.terrain().is_some() {
            Some(Vec3f::new(x, 0., z))
        } else {
            Some(Vec3f::new(x, z, 0.))
        }
    }

    fn harvest_space(&self, tile: VoxelTileCoord) -> Option<Vec3f> {
        [
            (0, 1),
            (1, 0),
            (0, -1),
            (-1, 0),
            (1, 1),
            (-1, 1),
            (1, -1),
            (-1, -1),
        ]
        .into_iter()
        .map(|(x, z)| VoxelTileCoord::new(tile.x + x, tile.z + z))
        .filter(|t| {
            self.ecology
                .orchard_beds
                .binary_search_by_key(t, |b| b.tile)
                .is_err()
        })
        .filter_map(|t| self.soil_position(t))
        .find(|p| {
            !self.objects.values().any(|o| {
                let depth = if self.terrain().is_some() {
                    o.position.z - p.z
                } else {
                    o.position.y - p.y
                };
                !o.consumed && (o.position.x - p.x).hypot(depth) < o.radius + 0.35
            })
        })
    }
    /// A nearby loose pear may be brought to a living resident, never taken from a carrier.
    pub fn orchard_offering(&self, recipient: WorldEntityId) -> Option<WorldEntityId> {
        let person = self.entity(recipient)?;
        let record = self.organism_registry().get(person.organism_id?)?;
        if person.kind != WorldObjectKind::Agent
            || person.consumed
            || !record.lifecycle().is_alive()
            || record.world_entity_id() != recipient
        {
            return None;
        }
        self.ecology
            .orchard_beds
            .iter()
            .filter(|b| !b.offered)
            .filter_map(|b| {
                let id = b.last_fruit?;
                let fruit = self.entity(id)?;
                let depth = if self.terrain().is_some() {
                    fruit.position.z - person.position.z
                } else {
                    fruit.position.y - person.position.y
                };
                let distance = (fruit.position.x - person.position.x).hypot(depth);
                (fruit.kind == WorldObjectKind::Food
                    && !fruit.consumed
                    && fruit.carried_by.is_none()
                    && self.player_held_object() != Some(id)
                    && distance <= 14.)
                    .then_some((distance, id))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.raw().cmp(&b.1.raw())))
            .map(|(_, id)| id)
    }
    pub fn offer_orchard_fruit(
        &mut self,
        recipient: WorldEntityId,
        tile: VoxelTileCoord,
    ) -> Result<WorldEntityId, ScaffoldContractError> {
        let id = self
            .orchard_offering(recipient)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let position = self
            .soil_position(tile)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let person = self
            .entity(recipient)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let depth = if self.terrain().is_some() {
            position.z - person.position.z
        } else {
            position.y - person.position.y
        };
        if (position.x - person.position.x).hypot(depth) > 3.6 {
            return Err(ScaffoldContractError::InvalidId);
        }
        let bin = self
            .ecology
            .orchard_beds
            .iter()
            .position(|b| b.last_fruit == Some(id))
            .ok_or(ScaffoldContractError::InvalidId)?;
        self.editor_move_object(id, position)?;
        self.ecology.orchard_beds[bin].offered = true;
        Ok(id)
    }

    /// Action implied by this exact patch, never a fallback to an unrelated animal.
    pub fn orchard_gesture(&self, tile: VoxelTileCoord) -> Option<OrchardGesture> {
        self.soil_position(tile)?;
        let beds = &self.ecology.orchard_beds;
        match beds.binary_search_by_key(&tile, |bed| bed.tile) {
            Ok(i) => {
                let g = beds[i].gesture(self.tick);
                Some(
                    if g == OrchardGesture::Pick && self.harvest_space(tile).is_none() {
                        OrchardGesture::Watch
                    } else {
                        g
                    },
                )
            }
            Err(_) if beds.len() < MAX_ORCHARD_BEDS => Some(OrchardGesture::Sow),
            _ => None,
        }
    }

    pub fn tend_orchard(
        &mut self,
        tile: VoxelTileCoord,
    ) -> Result<OrchardReceipt, ScaffoldContractError> {
        let action = self
            .orchard_gesture(tile)
            .ok_or(ScaffoldContractError::InvalidId)?;
        let slot = self
            .ecology
            .orchard_beds
            .binary_search_by_key(&tile, |bed| bed.tile);
        match (action, slot) {
            (OrchardGesture::Sow, Err(i)) => {
                self.ecology.orchard_beds.insert(
                    i,
                    OrchardBed {
                        tile,
                        cycle_started: self.tick,
                        watered: false,
                        last_fruit: None,
                        offered: false,
                    },
                );
                Ok(OrchardReceipt::Sown)
            }
            (OrchardGesture::Water, Ok(i)) => {
                self.ecology.orchard_beds[i].watered = true;
                Ok(OrchardReceipt::Watered)
            }
            (OrchardGesture::Watch, Ok(_)) => Ok(OrchardReceipt::Watching),
            (OrchardGesture::Pick, Ok(i)) => {
                // A loose fruit sits alongside the bush rather than hiding its selectable soil.
                let position = self
                    .harvest_space(tile)
                    .ok_or(ScaffoldContractError::InvalidId)?;
                let fruit = self.spawn_player_food(position, crate::FoodVariety::Fruit)?;
                self.ecology.orchard_beds[i] = OrchardBed {
                    tile,
                    cycle_started: self.tick,
                    watered: false,
                    last_fruit: Some(fruit),
                    offered: false,
                };
                Ok(OrchardReceipt::Picked(fruit))
            }
            _ => Err(ScaffoldContractError::InvalidId),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchard::OrchardStage;
    fn advance(w: &mut HeadlessWorld, n: u64) {
        for _ in 0..n {
            w.try_advance_tick().unwrap();
        }
    }
    #[test]
    fn same_tick_and_save_reload_cannot_double_water_or_harvest() {
        let tile = VoxelTileCoord::new(3, -2);
        let mut world = HeadlessWorld::new(99);
        assert_eq!(world.orchard_gesture(tile), Some(OrchardGesture::Sow));
        assert_eq!(world.tend_orchard(tile).unwrap(), OrchardReceipt::Sown);
        assert_eq!(world.tend_orchard(tile).unwrap(), OrchardReceipt::Watching);
        advance(&mut world, 80);
        assert_eq!(world.tend_orchard(tile).unwrap(), OrchardReceipt::Watered);
        assert_eq!(
            world.ecology.orchard_beds[0].stage(world.tick),
            OrchardStage::Bloom
        );
        assert_eq!(world.tend_orchard(tile).unwrap(), OrchardReceipt::Watching);
        advance(&mut world, 219);
        let bytes = serde_json::to_vec(&world.save_state()).unwrap();
        let mut restored =
            HeadlessWorld::from_save_state(&serde_json::from_slice(&bytes).unwrap()).unwrap();
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            restored.canonical_signature_digest().unwrap()
        );
        for w in [&mut world, &mut restored] {
            assert_eq!(w.orchard_gesture(tile), Some(OrchardGesture::Watch));
            advance(w, 1);
            let OrchardReceipt::Picked(id) = w.tend_orchard(tile).unwrap() else {
                panic!("expected real produce")
            };
            let fruit = w.entity(id).unwrap();
            assert_eq!(fruit.kind, WorldObjectKind::Food);
            assert!(!fruit.consumed && fruit.carried_by.is_none());
            assert_eq!(fruit.nutrition, crate::FoodVariety::Fruit.nutrition());
            assert_eq!(w.ecology.orchard_beds[0].last_fruit, Some(id));
            assert_eq!(w.object_count(), 1);
            assert_eq!(w.tend_orchard(tile).unwrap(), OrchardReceipt::Watching);
            assert_eq!(w.object_count(), 1);
            advance(w, 440); // A later crop also ripens without watering.
            assert_eq!(w.orchard_gesture(tile), Some(OrchardGesture::Pick));
        }
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            restored.canonical_signature_digest().unwrap()
        );
    }
    #[test]
    fn garden_is_bounded_and_does_not_pollute_old_saves_or_unsafe_ground() {
        let mut w = HeadlessWorld::new(72);
        let mut clean = serde_json::to_value(w.save_state()).unwrap();
        assert!(clean["ecology"].get("orchard_beds").is_none());
        assert!(w.tend_orchard(VoxelTileCoord::new(i32::MAX, 0)).is_err());
        let blocked = VoxelTileCoord::new(5, 3);
        w.editor_spawn_object(WorldEditorSpawnSpec {
            label: "stone".into(),
            kind: WorldObjectKind::Obstacle,
            organism_id: None,
            position: Vec3f::new(5.5, 3.5, 0.),
            nutrition: 0.,
            hazard_pain: 0.,
            radius: 0.5,
            token_id: None,
        })
        .unwrap();
        assert!(w.tend_orchard(blocked).is_err());
        for x in (0..MAX_ORCHARD_BEDS as i32).rev() {
            w.tend_orchard(VoxelTileCoord::new(x, -1)).unwrap();
        }
        assert!(w.tend_orchard(VoxelTileCoord::new(42, 42)).is_err());
        let mut saved = w.save_state();
        saved.ecology.orchard_beds[0].cycle_started = Tick::new(23);
        assert!(HeadlessWorld::from_save_state(&saved).is_err());
        saved.ecology.orchard_beds[0] = saved.ecology.orchard_beds[1];
        assert!(HeadlessWorld::from_save_state(&saved).is_err());
        clean["ecology"]["orchard_beds"] = serde_json::json!([]);
        let legacy =
            HeadlessWorld::from_save_state(&serde_json::from_value(clean).unwrap()).unwrap();
        assert!(legacy.ecology.orchard_beds.is_empty());
    }
    #[test]
    fn a_second_harvest_drops_beside_the_existing_fruit() {
        let mut w = HeadlessWorld::new(23);
        let tile = VoxelTileCoord::new(1, 3);
        w.tend_orchard(tile).unwrap();
        let mut crops = Vec::new();
        for _ in 0..2 {
            advance(&mut w, 440);
            let OrchardReceipt::Picked(id) = w.tend_orchard(tile).unwrap() else {
                panic!("missing fruit")
            };
            crops.push(w.entity(id).unwrap().position);
        }
        assert_ne!(crops[0], crops[1]);
        assert_eq!(w.object_count(), 2);
    }
    #[test]
    fn a_harvest_is_offered_once_by_its_real_id_without_commanding_the_resident() {
        use alife_core::{FoundationWeightAsset, SensorProfile};
        let foundation =
            FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1)
                .unwrap();
        let mut w = crate::create_canonical_new_game(
            &crate::CanonicalNewGameConfig::phase3(81, 1).unwrap(),
            &foundation,
        )
        .unwrap()
        .world;
        let (_, recipient) = w.organism_entity_ids()[0];
        let p = w.entity(recipient).unwrap().position;
        let base = VoxelTileCoord::new(p.x.floor() as i32, p.y.floor() as i32);
        let tile = (0..12)
            .map(|i| VoxelTileCoord::new(base.x + i % 4 - 2, base.z + i / 4 - 1))
            .find(|&t| w.orchard_gesture(t).is_some())
            .unwrap();
        w.tend_orchard(tile).unwrap();
        // Isolate the physical-world command: this fixture advances no organism.
        w.tick = Tick::new(w.tick.raw() + 440);
        let OrchardReceipt::Picked(fruit) = w.tend_orchard(tile).unwrap() else {
            panic!("expected harvested fruit")
        };
        assert_eq!(w.orchard_offering(recipient), Some(fruit));
        let count = w.object_count();
        assert!(w
            .offer_orchard_fruit(recipient, VoxelTileCoord::new(400, 400))
            .is_err());
        assert_eq!(w.orchard_offering(recipient), Some(fruit));
        assert_eq!(w.offer_orchard_fruit(recipient, tile).unwrap(), fruit);
        assert_eq!(w.object_count(), count);
        assert!(!w.entity(fruit).unwrap().consumed);
        assert!(w.offer_orchard_fruit(recipient, tile).is_err());
        let restored = HeadlessWorld::from_save_state(&w.save_state()).unwrap();
        assert_eq!(restored.orchard_offering(recipient), None);
    }
}
