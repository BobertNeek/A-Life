//! A fieldglass looks back at the last actual gift. It cannot give food, command
//! a carrier, or award a creature credit for an event the world didn't identify.
use super::super::*;
use bevy::{a11y::AccessibilityNode, prelude::*};

#[derive(Component)]
struct Lens;
#[derive(Component)]
struct Glyph;
#[derive(Resource)]
struct Art([Handle<Image>; 4]);
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Condition {
    Food,
    Toy,
    Carried,
    Consumed,
}
impl Condition {
    fn art(self) -> (usize, &'static str) {
        match self {
            Self::Food => (0, "Look at the food you left"),
            Self::Toy => (1, "Look at the toy you left"),
            Self::Carried => (2, "Look at the resident carrying your gift"),
            Self::Consumed => (3, "Look where your food was consumed"),
        }
    }
}
struct Sighting {
    gift: WorldEntityId,
    point: Vec2,
    condition: Condition,
}

fn sighting(world: &alife_world::HeadlessWorld, last: Option<WorldEntityId>) -> Option<Sighting> {
    // Re-open the actual last harvested ID from a saved bed. If an explicitly
    // watched object disappeared, do not retarget the player to someone else's gift.
    let id = last.or_else(|| {
        world
            .ecology()
            .orchard_beds
            .iter()
            .filter(|b| b.last_fruit.is_some())
            .max_by_key(|b| (b.cycle_started.raw(), b.tile))?
            .last_fruit
    })?;
    let object = world.entity(id)?;
    let loose = match object.kind {
        WorldObjectKind::Food => Condition::Food,
        WorldObjectKind::Ball | WorldObjectKind::ActivityToy => Condition::Toy,
        _ => return None,
    };
    let (pos, condition) = if object.consumed {
        if loose != Condition::Food {
            return None;
        }
        (object.position, Condition::Consumed)
    } else if let Some(by) = object.carried_by {
        let record = world.organism_registry().get(by)?;
        let carrier = world.entity(record.world_entity_id())?;
        if carrier.organism_id != Some(by) || carrier.consumed || !record.lifecycle().is_alive() {
            return None;
        }
        (carrier.position, Condition::Carried)
    } else {
        // The god Hand can be transporting this object independently of a resident.
        if world.player_held_object() == Some(id) {
            return None;
        }
        (object.position, loose)
    };
    let point = Vec2::new(
        pos.x,
        if world.terrain().is_some() {
            pos.z
        } else {
            pos.y
        },
    );
    point.is_finite().then_some(Sighting {
        gift: id,
        point,
        condition,
    })
}

pub(in crate::production_voxel_renderer) fn current_point(
    world: &alife_world::HeadlessWorld,
    gift: WorldEntityId,
) -> Option<Vec2> {
    sighting(world, Some(gift)).map(|s| s.point)
}
pub(in crate::production_voxel_renderer) fn look(
    world: &alife_world::HeadlessWorld,
    selected: Option<StableVoxelObjectRef>,
    dock: &mut care_dock::State,
    follow: &mut Fvr04ProductionCreatureFollowResource,
) {
    if let Some(s) = sighting(world, dock.last_gift) {
        dock.garden_armed = false;
        dock.hand_focus = Some(care_dock::HandFocus {
            guard: selected,
            gift: Some(s.gift),
            point: s.point,
        });
        follow.enabled = false;
    }
}

macro_rules! signs {
    ($($name:literal),+ $(,)?)=>{[$((concat!("assets/ui/fieldglass/",$name,".png"),
        include_bytes!(concat!("../../../../../assets/ui/fieldglass/",$name,".png")).as_slice())),+]};
}
const IMAGES: [(&str, &[u8]); 4] = signs!("food", "toy", "carried", "consumed");
pub(in crate::production_voxel_renderer) fn spawn(world: &mut World, parent: Entity) {
    let art = IMAGES.map(|(_, b)| care_dock::decode_png(world, b));
    let (root, icon) = care_dock::spawn_accessory(
        world,
        parent,
        care_dock::Accessory::Fieldglass,
        art[0].clone(),
        "Fieldglass",
    );
    world.entity_mut(root).insert(Lens);
    world.entity_mut(icon).insert(Glyph);
    world.insert_resource(Art(art));
}
pub(in crate::production_voxel_renderer) fn sync(world: &mut World, debug: bool) {
    let last = world.resource::<care_dock::State>().last_gift;
    let blocked = debug
        || world
            .get_resource::<menu::MenuState>()
            .is_some_and(menu::captures_input_state)
        || world
            .get_resource::<ProductionConversationLineageUiState>()
            .is_some_and(|s| s.blocks_world_shortcuts());
    let seen = (!blocked)
        .then(|| {
            world
                .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
                .and_then(|r| sighting(r.runtime.world(), last))
        })
        .flatten();
    let info = seen.map(|s| s.condition.art());
    if let Some((slot, _)) = info {
        let handle = world.resource::<Art>().0[slot].clone();
        for mut glyph in world
            .query_filtered::<&mut ImageNode, With<Glyph>>()
            .iter_mut(world)
        {
            if glyph.image != handle {
                glyph.image = handle.clone();
            }
        }
    }
    for (mut v, mut a, mut i) in world
        .query_filtered::<(&mut Visibility, &mut AccessibilityNode, &mut Interaction), With<Lens>>()
        .iter_mut(world)
    {
        if let Some((_, label)) = info {
            v.set_if_neq(Visibility::Inherited);
            a.clear_hidden();
            a.set_label(label);
        } else {
            v.set_if_neq(Visibility::Hidden);
            a.set_hidden();
            *i = Interaction::None;
        }
    }
}
#[cfg(test)]
pub(in crate::production_voxel_renderer) fn image_path(
    world: &World,
    image: &Handle<Image>,
) -> Option<String> {
    let i = world
        .get_resource::<Art>()?
        .0
        .iter()
        .position(|h| h == image)?;
    Some(IMAGES[i].0.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_actual_selected_or_saved_object_can_be_revisited() {
        let mut world = alife_world::HeadlessWorld::new(7);
        assert!(sighting(&world, None).is_none());
        let pear = world
            .spawn_player_food(Vec3f::new(4.0, 2.0, 0.0), alife_world::FoodVariety::Fruit)
            .unwrap();
        let toy = world
            .spawn_toy("test toy", Vec3f::new(-3.0, 1.0, 0.0), true)
            .unwrap();
        let s = sighting(&world, Some(pear)).unwrap();
        assert_eq!(s.point, Vec2::new(4.0, 2.0));
        assert_eq!(s.condition, Condition::Food);
        assert_eq!(
            sighting(&world, Some(toy)).unwrap().condition,
            Condition::Toy
        );
        assert!(sighting(&world, Some(WorldEntityId(999))).is_none());
        assert!(sighting(&world, None).is_none());
        world.begin_player_hold(pear).unwrap();
        assert!(
            sighting(&world, Some(pear)).is_none(),
            "the Hand is not a creature carrying the gift"
        );
        world.release_player_hold().unwrap();
        assert_eq!(
            sighting(&world, Some(pear)).unwrap().condition,
            Condition::Food
        );
    }

    #[test]
    fn carried_and_consumed_are_distinct_and_never_assign_an_unobserved_meal() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../alife_world/tests/fixtures/production_voxel");
        let mut saved = crate::tests::fixtures::current_scene_save(&root, 1);
        let world = saved.restore_headless_world().unwrap();
        let (organism, id) = world.organism_entity_ids()[0];
        let resident = world.entity(id).unwrap().position;
        let pear = saved
            .world
            .objects
            .iter()
            .position(|o| o.kind == WorldObjectKind::Food)
            .unwrap();
        let gift = saved.world.objects[pear].id;
        saved.world.objects[pear].carried_by = Some(organism);
        let carried = saved.restore_headless_world().unwrap();
        let seen = sighting(&carried, Some(gift)).unwrap();
        assert_eq!(seen.condition, Condition::Carried);
        assert_eq!(
            seen.point,
            Vec2::new(
                resident.x,
                if world.terrain().is_some() {
                    resident.z
                } else {
                    resident.y
                }
            )
        );
        saved.world.objects[pear].carried_by = None;
        saved.world.objects[pear].consumed = true;
        let consumed = saved.restore_headless_world().unwrap();
        assert_eq!(
            sighting(&consumed, Some(gift)).unwrap().condition,
            Condition::Consumed
        );
    }
}
