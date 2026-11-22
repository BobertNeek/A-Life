use super::*;
use crate::orchard::{
    valid_coordinate, OrchardBed, OrchardCourt, OrchardCourtCare, OrchardCourtFoot, OrchardGesture,
    OrchardReceipt, MAX_ORCHARD_BEDS, ORCHARD_COURT_REVIEW,
};
use crate::VoxelTileCoord;

const ORCHARD_SIDE_OFFSETS: [(i32, i32); 8] = [
    (0, 1), (1, 0), (0, -1), (-1, 0), (1, 1), (-1, 1), (1, -1), (-1, -1),
];
const ORCHARD_SIDE_MARGIN: f32 = 0.35;
const ORCHARD_PASSAGE_RADIUS: f32 = 0.55;

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

    /// The original settled chime can also mark one dry working court.
    /// Select the nearest real bed with an unplanted, signed two-cell apron.
    /// A moving chime does not reserve, receive or display a settled court.
    pub fn orchard_court(&self) -> Option<OrchardCourt> {
        if !ORCHARD_COURT_REVIEW {
            return None;
        }
        let id = self.entity_id("player-activity-toy")?;
        let table = self.entity(id)?;
        if !self.original_player_chime(table) || self.player_held_object() == Some(id) {
            return None;
        }
        self.orchard_court_at(id, table.position)
    }

    /// The existing Garden Return from a real, settled original Chime belongs
    /// to its own tended bed, not to whatever was planted most recently.
    pub fn orchard_court_home(&self, id: WorldEntityId) -> Option<VoxelTileCoord> {
        // A different selected prop should not drive another footprint survey.
        if !self.original_player_chime(self.entity(id)?) {
            return None;
        }
        self.orchard_court().filter(|court| court.chime == id).map(|court| court.bed)
    }

    /// Grounded next-use projection. The same native decision supplies ordinary
    /// tending, including side-harvest fallback when the actual tray is full.
    pub fn orchard_court_care(&self) -> Option<(OrchardCourt, OrchardCourtCare)> {
        let court = self.orchard_court()?;
        let care = match self.orchard_gesture_with_court(court.bed, Some(court)) {
            Some(OrchardGesture::Water) => OrchardCourtCare::WaterRoots,
            Some(OrchardGesture::Pick) if court.open => OrchardCourtCare::ReceivePear,
            Some(OrchardGesture::Pick) => OrchardCourtCare::PickAtSide,
            _ => OrchardCourtCare::Waiting,
        };
        Some((court, care))
    }

    /// The same settled layout, witnessed while aiming the real Chime Hand.
    /// A real unsettled hold still does not reserve ground or receive fruit.
    pub fn orchard_court_preview(
        &self,
        chime: WorldEntityId,
        aim: Vec3f,
    ) -> Option<(Vec3f, OrchardCourt)> {
        if !ORCHARD_COURT_REVIEW {
            return None;
        }
        let ground = self.surveyed_original_chime_ground(chime, aim)?;
        let court = self.orchard_court_at(chime, ground).filter(|c| c.open)?;
        Some((ground, court))
    }


    fn orchard_court_at(&self, id: WorldEntityId, at: Vec3f) -> Option<OrchardCourt> {
        let support = self.court_foot(at.x, at.z, self.entity(id)?.radius)?;
        if (support.ground.y - at.y).abs() > 0.012 {
            return None;
        }
        self.ecology.orchard_beds.iter()
            .filter_map(|bed| self.orchard_court_for_bed(id, at, bed.tile))
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.bed.cmp(&b.1.bed)))
            .map(|(_, court)| court)
    }

    fn court_foot(&self, x: f32, z: f32, r: f32) -> Option<OrchardCourtFoot> {
        let t = self.terrain()?;
        let surface = t.surface();
        let (y, rise) = surface.sample(x, z)?;
        let mut limits = t.limits();
        limits.body_radius = r;
        limits.max_wading_depth = 0.0;
        if !surface.walkable_with(x, z, limits) {
            return None;
        }
        let d = r * std::f32::consts::FRAC_1_SQRT_2;
        for (dx, dz) in [
            (0., 0.), (r, 0.), (-r, 0.), (0., r), (0., -r),
            (d, d), (d, -d), (-d, d), (-d, -d),
        ] {
            let h = surface.height(x + dx, z + dz)?;
            if (h - y).abs() > 0.18 || surface.water_level.is_some_and(|w| h <= w + 0.035) {
                return None;
            }
        }
        Some(OrchardCourtFoot { ground: Vec3f::new(x, y, z), rise })
    }

    fn orchard_court_for_bed(
        &self,
        chime: WorldEntityId,
        table: Vec3f,
        bed: VoxelTileCoord,
    ) -> Option<(f32, OrchardCourt)> {
        let (x, z) = (bed.x as f32 + 0.5, bed.z as f32 + 0.5);
        let (dx, dz) = (table.x - x, table.z - z);
        let (forward, across, axis) = if dx.abs() >= dz.abs() {
            (dx, dz, [if dx >= 0.0 { 1 } else { -1 }, 0])
        } else {
            (dz, dx, [0, if dz >= 0.0 { 1 } else { -1 }])
        };
        if !(3.55..=4.55).contains(&forward.abs()) || across.abs() > 0.65 {
            return None;
        }
        let tile = |n| VoxelTileCoord::new(bed.x + axis[0]*n, bed.z + axis[1]*n);
        let (aisle, landing) = (tile(1), tile(2));
        for target in [aisle, landing] {
            self.soil_position(target)?;
            if self.ecology.orchard_beds.binary_search_by_key(&target, |b| b.tile).is_ok() {
                return None;
            }
        }
        let sx = axis[0] as f32;
        let sz = axis[1] as f32;
        self.soil_position(bed)?;
        let root = self.court_foot(x, z, 0.43)?.ground;
        let middle = self.court_foot(x + sx, z + sz, ORCHARD_PASSAGE_RADIUS)?.ground;
        let end = self.court_foot(x + 2.0*sx, z + 2.0*sz, 0.56)?.ground;
        let t = self.terrain()?;
        let mut passage = t.limits();
        passage.body_radius = ORCHARD_PASSAGE_RADIUS;
        passage.max_wading_depth = 0.0;
        t.surface().resolve_move_with(root, middle, passage)?;
        t.surface().resolve_move_with(middle, end, passage)?;
        let rim = |p: Vec3f, side: f32| self.court_foot(p.x - sz*side, p.z + sx*side, 0.40);
        let rims = [rim(middle, -0.77)?, rim(middle, 0.77)?,
                    rim(end, -0.77)?, rim(end, 0.77)?];
        // Do not frame a route through scene props or a neighboring cultivated
        // bed. Shallow ceramic edging adds no new movement or collision rule.
        if self.ecology.orchard_beds.iter().filter(|other| other.tile != bed).any(|other| {
            rims.iter().any(|r| {
                (r.ground.x - (other.tile.x as f32 + 0.5))
                    .hypot(r.ground.z - (other.tile.z as f32 + 0.5)) < 0.82
            })
        }) {
            return None;
        }
        let flat = |p: Vec3f| Vec3f::new(p.x, 0.0, p.z);
        let crosses = |object: &WorldObject| {
            distance(flat(object.position), closest_point_on_segment(flat(object.position), flat(middle), flat(end)))
                < object.radius + ORCHARD_PASSAGE_RADIUS
        };
        let loose = |object: &&WorldObject| {
            object.id != chime && !object.consumed && object.carried_by.is_none()
                && self.player_held_object() != Some(object.id)
        };
        if self.objects.values().filter(loose).any(|object| {
            matches!(object.kind, WorldObjectKind::Obstacle | WorldObjectKind::Hazard | WorldObjectKind::ActivityToy)
                && (crosses(object) || rims.iter().any(|r| {
                    (object.position.x-r.ground.x).hypot(object.position.z-r.ground.z) < object.radius + 0.40
                }))
        }) {
            return None;
        }
        // Traffic can occupy the landing without unbuilding it. In that case
        // harvest goes to an ordinary legal non-court side, if there is one.
        let open = !self.objects.values().filter(loose).any(crosses);
        Some((dx.hypot(dz), OrchardCourt {
            chime, bed, aisle, landing, landing_ground:end, axis, rims, open,
        }))
    }

    fn harvest_space(&self, tile: VoxelTileCoord, court: Option<OrchardCourt>) -> Option<Vec3f> {
        if let Some(receiving) = court.filter(|s| s.bed == tile && s.open) {
            return Some(receiving.landing_ground);
        }
        ORCHARD_SIDE_OFFSETS
        .into_iter()
        .map(|(x, z)| VoxelTileCoord::new(tile.x + x, tile.z + z))
        .filter(|t| {
            court.is_none_or(|c| !c.reserves(*t)) && self.ecology
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
                !o.consumed && (o.position.x - p.x).hypot(depth) < o.radius + ORCHARD_SIDE_MARGIN
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
        self.orchard_gesture_with_court(tile, self.orchard_court())
    }

    fn orchard_gesture_with_court(
        &self,
        tile: VoxelTileCoord,
        court: Option<OrchardCourt>,
    ) -> Option<OrchardGesture> {
        self.soil_position(tile)?;
        let beds = &self.ecology.orchard_beds;
        match beds.binary_search_by_key(&tile, |bed| bed.tile) {
            Ok(i) => {
                let g = beds[i].gesture(self.tick);
                Some(
                    if g == OrchardGesture::Pick && self.harvest_space(tile, court).is_none() {
                        OrchardGesture::Watch
                    } else {
                        g
                    },
                )
            }
            Err(_) if beds.len() < MAX_ORCHARD_BEDS && court.is_none_or(|c| !c.reserves(tile)) => {
                Some(OrchardGesture::Sow)
            }
            _ => None,
        }
    }

    /// Once its own real crop is no longer in the serving area or is actually
    /// consumed, the existing unripe planter can move. Neither a previous
    /// harvest nor a past offering is rewritten. Look at current custody and
    /// geometry rather than recording a new permanent quest or pickup flag.
    fn orchard_crop_clears_planter(&self, bed: OrchardBed, at: VoxelTileCoord) -> bool {
        let Some(id) = bed.last_fruit else { return !bed.offered; };
        let Some(fruit) = self.entity(id) else { return false; };
        if fruit.kind != WorldObjectKind::Food {
            return false;
        }
        if fruit.consumed {
            return true;
        }

        let crop = Vec3f::new(
            fruit.position.x - (at.x as f32 + 0.5),
            0.0,
            fruit.position.z - (at.z as f32 + 0.5),
        );
        // Use the same side-landing disks as ordinary harvest. Include the root
        // at their center so bringing this pear back cannot invent portability.
        let near_side = std::iter::once((0, 0)).chain(ORCHARD_SIDE_OFFSETS).any(|(x, z)| {
            (crop.x - x as f32).hypot(crop.z - z as f32) < fruit.radius + ORCHARD_SIDE_MARGIN
        });
        // Any former court may already have been dismantled. The four
        // legal cardinal approaches use the same aisle-to-tray reach as
        // the actual court, without depending on a saved or current Chime.
        let near_apron = ORCHARD_SIDE_OFFSETS[..4].iter().any(|&(x, z)| {
            let start = Vec3f::new(x as f32, 0.0, z as f32);
            let end = Vec3f::new((2*x) as f32, 0.0, (2*z) as f32);
            distance(crop, closest_point_on_segment(crop, start, end))
                < fruit.radius + ORCHARD_PASSAGE_RADIUS
        });
        !near_side && !near_apron
    }

    /// A real unripe nursery bed may move after its last crop clears its home.
    /// Water, age, previous fruit and offering remain this exact bed's state.
    pub fn movable_orchard_planter(&self, tile: VoxelTileCoord) -> Option<OrchardBed> {
        if !ORCHARD_COURT_REVIEW {
            return None;
        }
        self.terrain()?;
        if self.player_held_object().is_some() || !valid_coordinate(tile) {
            return None;
        }
        let beds = &self.ecology.orchard_beds;
        let bed = beds[beds.binary_search_by_key(&tile, |bed| bed.tile).ok()?];
        (bed.stage(self.tick) != crate::orchard::OrchardStage::Ripe
            && self.orchard_crop_clears_planter(bed, tile))
            .then_some(bed)
    }

    /// A dry native seat for both ceramic fittings and the actual planted soil.
    /// This is a read-only check, never an auto-move to a neighbouring tile.
    pub fn survey_orchard_planter(
        &self,
        source: OrchardBed,
        target: VoxelTileCoord,
    ) -> Option<Vec3f> {
        const ROOT_PAD: f32 = 0.53;
        const MAX_REACH: f32 = 6.0;
        if self.movable_orchard_planter(source.tile) != Some(source)
            || source.tile == target
            || !valid_coordinate(target)
            || ((target.x - source.tile.x) as f32).hypot((target.z - source.tile.z) as f32) > MAX_REACH
            || !self.orchard_crop_clears_planter(source, target)
        {
            return None;
        }
        // A settled court reserves the same two tiles from moving a rooted
        // planter as from sowing. Leaving the court by moving its own root
        // to other ground still works. Commit goes through this same vote.
        if self.orchard_court().is_some_and(|court| court.reserves(target)) {
            return None;
        }
        let p = self.soil_position(target)?;
        let terrain = self.terrain()?;
        let surface = terrain.surface();
        let center = surface.height(p.x, p.z)?;
        let mut limits = terrain.limits();
        limits.body_radius = ROOT_PAD;
        limits.max_wading_depth = 0.0;
        if !surface.walkable_with(p.x, p.z, limits) {
            return None;
        }
        let edge = ROOT_PAD * std::f32::consts::FRAC_1_SQRT_2;
        for (dx, dz) in [(0., 0.), (ROOT_PAD, 0.), (-ROOT_PAD, 0.), (0., ROOT_PAD),
            (0., -ROOT_PAD), (edge, edge), (-edge, edge), (edge, -edge), (-edge, -edge)] {
            let y = surface.height(p.x + dx, p.z + dz)?;
            if (y - center).abs() > 0.16
                || surface.water_level.is_some_and(|water| y <= water + 0.035)
            {
                return None;
            }
        }
        let plot_gap = self.ecology.orchard_beds.iter().all(|bed| {
            bed.tile == source.tile || {
                let dx = bed.tile.x as f32 + 0.5 - p.x;
                let dz = bed.tile.z as f32 + 0.5 - p.z;
                dx.hypot(dz) >= ROOT_PAD + 0.42
            }
        });
        let body_gap = self.objects.values().all(|object| {
            object.consumed || object.carried_by.is_some() ||
                ((object.position.x - p.x).hypot(object.position.z - p.z) >= object.radius + ROOT_PAD)
        });
        (plot_gap && body_gap).then_some(Vec3f::new(p.x, center, p.z))
    }

    /// Commit the same bed, at the same existing sow count. The preview operand is
    /// revalidated against current World so a settling resident can refuse it.
    pub fn reseat_orchard_planter(
        &mut self,
        source: OrchardBed,
        target: VoxelTileCoord,
    ) -> Result<OrchardBed, ScaffoldContractError> {
        self.survey_orchard_planter(source, target).ok_or(ScaffoldContractError::InvalidId)?;
        let beds = &mut self.ecology.orchard_beds;
        let from = beds.binary_search_by_key(&source.tile, |bed| bed.tile)
            .map_err(|_| ScaffoldContractError::InvalidId)?;
        let mut bed = beds.remove(from);
        bed.tile = target;
        let dest = beds.partition_point(|other| other.tile < target);
        beds.insert(dest, bed);
        Ok(bed)
    }

    pub fn tend_orchard(
        &mut self,
        tile: VoxelTileCoord,
    ) -> Result<OrchardReceipt, ScaffoldContractError> {
        let court = self.orchard_court();
        let action = self
            .orchard_gesture_with_court(tile, court)
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
                    .harvest_space(tile, court)
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

    #[cfg(feature = "qualia-village-review")]
    #[test]
    fn wick_pot_reseats_the_same_watered_bed_and_reverses_without_losing_care() {
        let mut world = HeadlessWorld::new(1643);
        world.enable_island_for_new_game().unwrap();
        let old = VoxelTileCoord::new(79, 250);
        let next = VoxelTileCoord::new(82, 252);
        assert_eq!(world.tend_orchard(old).unwrap(), OrchardReceipt::Sown);
        advance(&mut world, 80);
        assert_eq!(world.tend_orchard(old).unwrap(), OrchardReceipt::Watered);
        let source = world.movable_orchard_planter(old).unwrap();
        assert!(source.watered);
        assert_eq!(source.cycle_started, Tick::ZERO);
        let inspect = world.canonical_signature_digest().unwrap();
        let preview = world.survey_orchard_planter(source, next).unwrap();
        assert_eq!((preview.x, preview.z), (82.5, 252.5));
        assert_eq!(world.canonical_signature_digest().unwrap(), inspect);
        let placed = world.reseat_orchard_planter(source, next).unwrap();
        assert_eq!(placed, OrchardBed { tile: next, ..source });
        assert_eq!(world.ecology().orchard_beds.as_slice(), &[placed]);
        assert_eq!(world.object_count(), 0);
        let portable = crate::PortableSaveFile::from_headless_world(
            "wick-pot-bed-reseat", &world,
            crate::RuntimeConfig::deterministic_default(world.seed(), alife_core::BrainScaleTier::Nano512),
            crate::AssetManifest::empty(), vec![],
        ).unwrap();
        let mut reopened = crate::PortableSaveFile::from_json_str(&portable.to_json_string_pretty().unwrap())
            .unwrap().restore_headless_world().unwrap();
        assert_eq!(reopened.ecology().orchard_beds.as_slice(), &[placed]);
        assert_eq!(reopened.reseat_orchard_planter(placed, old).unwrap(), source);
        let delayed = reopened.reseat_orchard_planter(source, next).unwrap();
        advance(&mut reopened, 230);
        assert_eq!(reopened.orchard_gesture(next), Some(OrchardGesture::Pick));
        assert!(reopened.movable_orchard_planter(next).is_none());
        let OrchardReceipt::Picked(pear) = reopened.tend_orchard(next).unwrap() else {
            panic!("the moved, cared-for bed must produce its ordinary pear");
        };
        let p = reopened.entity(pear).unwrap().position;
        assert_eq!((p.x, p.z), (82.5, 253.5));
        assert_eq!(reopened.ecology().orchard_beds[0].tile, delayed.tile);
        assert_eq!(reopened.ecology().orchard_beds[0].last_fruit, Some(pear));
    }

    #[cfg(feature = "qualia-village-review")]
    #[test]
    fn wick_pot_refuses_an_occupied_seat_without_moving_either_owner() {
        let mut world = HeadlessWorld::new(1643);
        world.enable_island_for_new_game().unwrap();
        let old = VoxelTileCoord::new(79, 250);
        let next = VoxelTileCoord::new(82, 252);
        world.tend_orchard(old).unwrap();
        let source = world.movable_orchard_planter(old).unwrap();
        let valid = world.survey_orchard_planter(source, next).unwrap();
        let original_ball = world.spawn_toy("wick-pot-late-original-rattle", valid, true).unwrap();
        let ball = world.entity(original_ball).unwrap().clone();
        let now = world.canonical_signature_digest().unwrap();
        assert_eq!(world.survey_orchard_planter(source, next), None);
        assert!(world.reseat_orchard_planter(source, next).is_err());
        assert_eq!(world.ecology().orchard_beds.as_slice(), &[source]);
        assert_eq!(world.entity(original_ball), Some(&ball));
        assert_eq!(world.canonical_signature_digest().unwrap(), now);
        assert!(world.reseat_orchard_planter(source, VoxelTileCoord::new(12, 435)).is_err());
        assert_eq!(world.canonical_signature_digest().unwrap(), now);
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
