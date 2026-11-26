//! A ground-specific Hand tool and the world-owned stages that it reveals.
use super::*;
#[cfg(feature = "gpu-runtime")]
mod court;
#[cfg(feature = "gpu-runtime")]
pub(super) mod fx;
#[cfg(feature = "gpu-runtime")]
mod wick_pot;
#[cfg(feature = "gpu-runtime")]
use alife_world::orchard::OrchardGesture;
use bevy::{a11y::AccessibilityNode, prelude::*};
#[cfg(feature = "gpu-runtime")]
pub(super) use wick_pot::{WickPot, WickReturn};

#[derive(Component)]
pub(super) struct GardenTool;
#[derive(Component)]
struct ToolIcon;
#[derive(Resource)]
struct Signs([Handle<Image>; 6]);

#[cfg(feature = "gpu-runtime")]
pub(super) fn soil_tile(s: Option<StableVoxelObjectRef>) -> Option<VoxelTileCoord> {
    s.filter(|r| r.kind == StableVoxelRefKind::Tile && r.is_stable())?
        .tile
}
#[cfg(feature = "gpu-runtime")]
pub(super) fn starting_soil(
    world: &alife_world::HeadlessWorld,
    selected: Option<StableVoxelObjectRef>,
) -> Option<VoxelTileCoord> {
    let center = soil_tile(selected).or_else(|| {
        let position = selected
            .and_then(|s| s.stable_id)
            .and_then(|id| world.entity(id))
            .or_else(|| {
                world
                    .organism_registry()
                    .iter()
                    .find_map(|r| world.entity(r.world_entity_id()))
            })?
            .position;
        let depth = if world.terrain().is_some() {
            position.z
        } else {
            position.y
        };
        Some(VoxelTileCoord::new(
            position.x.floor() as i32,
            depth.floor() as i32,
        ))
    })?;
    [
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
        (2, 0),
        (0, 2),
        (-2, 0),
        (0, -2),
    ]
    .into_iter()
    .map(|(x, z)| VoxelTileCoord::new(center.x + x, center.z + z))
    .find(|t| world.orchard_gesture(*t) == Some(OrchardGesture::Sow))
}
#[cfg(feature = "gpu-runtime")]
pub(super) fn selected_soil(tile: VoxelTileCoord) -> StableVoxelObjectRef {
    StableVoxelObjectRef {
        kind: StableVoxelRefKind::Tile,
        stable_id: None,
        chunk: VoxelChunkCoord::for_tile(16, tile),
        tile: Some(tile),
    }
}
#[cfg(feature = "gpu-runtime")]
#[derive(Clone, Copy)]
pub(super) enum Plan {
    Choose,
    Return(VoxelTileCoord),
    Soil(VoxelTileCoord, OrchardGesture),
    Offer(WorldEntityId),
}
#[cfg(feature = "gpu-runtime")]
pub(super) fn plan(
    world: &alife_world::HeadlessWorld,
    selected: Option<StableVoxelObjectRef>,
) -> Plan {
    if let Some(court) =
        soil_tile(selected).and_then(|tile| world.orchard_court().filter(|c| c.reserves(tile)))
    {
        // Use the existing return gesture at the actual entrance; do not
        // redirect to the most recent unrelated plot on the island.
        return Plan::Return(court.bed);
    }
    if let Some((t, g)) = soil_tile(selected).and_then(|t| world.orchard_gesture(t).map(|g| (t, g)))
    {
        return Plan::Soil(t, g);
    }
    if let Some(id) = selected
        .filter(|s| s.kind == StableVoxelRefKind::Creature)
        .and_then(|s| s.stable_id)
    {
        if world.orchard_offering(id).is_some() {
            return Plan::Offer(id);
        }
    }
    world
        .ecology()
        .orchard_beds
        .iter()
        .max_by_key(|bed| (bed.cycle_started.raw(), bed.tile))
        .map_or(Plan::Choose, |b| Plan::Return(b.tile))
}
#[cfg(feature = "gpu-runtime")]
fn gesture_art(g: OrchardGesture) -> (usize, &'static str) {
    match g {
        OrchardGesture::Sow => (0, "Plant a lantern pear here"),
        OrchardGesture::Water => (1, "Water these roots"),
        OrchardGesture::Watch => (2, "Watch this plant grow"),
        OrchardGesture::Pick => (3, "Pick a ripe lantern pear"),
    }
}
macro_rules! glyphs {
    ($($name:literal),+ $(,)?) => { [$((
        concat!("assets/ui/orchard/", $name, ".png"),
        include_bytes!(concat!("../../../../assets/ui/orchard/", $name, ".png")).as_slice()
    )),+] };
}
const ART: [(&str, &[u8]); 6] = glyphs!("sow", "water", "watch", "pick", "offer", "return");

#[cfg(test)]
pub(super) fn image_path(world: &World, image: &Handle<Image>) -> Option<String> {
    let i = world
        .get_resource::<Signs>()?
        .0
        .iter()
        .position(|h| h == image)?;
    Some(ART[i].0.to_owned())
}

pub(super) fn spawn_tool(world: &mut World, parent: Entity) {
    let handles = ART.map(|(_, bytes)| care_dock::decode_png(world, bytes));
    let (root, icon) = care_dock::spawn_accessory(
        world,
        parent,
        care_dock::Accessory::Garden,
        handles[0].clone(),
        "Tend the selected soil",
    );
    world.entity_mut(root).insert(GardenTool);
    world.entity_mut(icon).insert(ToolIcon);
    world.insert_resource(Signs(handles));
}
pub(super) fn sync_tool(world: &mut World, selected: Option<StableVoxelObjectRef>, debug: bool) {
    let armed = world.resource::<care_dock::State>().garden_armed;
    #[cfg(feature = "gpu-runtime")]
    let art = world
        .get_non_send_resource::<ProductionGpuBrainRuntimeResource>()
        .filter(|_| {
            !debug
                && !world
                    .get_resource::<menu::MenuState>()
                    .is_some_and(menu::captures_input_state)
                && !world
                    .get_resource::<ProductionConversationLineageUiState>()
                    .is_some_and(|s| s.blocks_world_shortcuts())
        })
        .map(|r| {
            if armed {
                (0, "Cancel soil selection")
            } else {
                match plan(r.runtime.world(), selected) {
                    Plan::Soil(_, g) => gesture_art(g),
                    Plan::Offer(_) => (4, "Bring a harvested pear to this individual"),
                    Plan::Return(_) => (5, "Return to your pear garden"),
                    Plan::Choose => (0, "Choose soil for a lantern pear"),
                }
            }
        });
    #[cfg(not(feature = "gpu-runtime"))]
    let art: Option<(usize, &str)> = {
        let _ = (selected, debug);
        None
    };
    if let Some((slot, _)) = art {
        let image = world.resource::<Signs>().0[slot].clone();
        for mut icon in world
            .query_filtered::<&mut ImageNode, With<ToolIcon>>()
            .iter_mut(world)
        {
            icon.image = image.clone();
        }
    }
    for (mut visibility, mut a, mut interaction, mut border) in world
        .query_filtered::<(
            &mut Visibility,
            &mut AccessibilityNode,
            &mut Interaction,
            &mut BorderColor,
        ), With<GardenTool>>()
        .iter_mut(world)
    {
        if let Some((_, label)) = art {
            *visibility = Visibility::Inherited;
            a.clear_hidden();
            a.set_label(label);
        } else {
            *visibility = Visibility::Hidden;
            a.set_hidden();
            *interaction = Interaction::None;
        }
        *border = if armed {
            BorderColor::all(sanctuary_ink::JADE)
        } else {
            sanctuary_ink::bevel(*interaction != Interaction::Pressed)
        };
    }
}

#[cfg(feature = "gpu-runtime")]
fn focus_existing_garden_return(
    tool: bool,
    plan: Plan,
    selection: &mut Fvr03ProductionVoxelSelectionResource,
    follow: &mut Fvr04ProductionCreatureFollowResource,
    dock: &mut care_dock::State,
) -> bool {
    let (true, false, Plan::Return(tile)) = (tool, dock.garden_armed, plan) else {
        return false;
    };
    selection.selected = Some(selected_soil(tile));
    follow.enabled = false;
    dock.hand_focus = Some(care_dock::HandFocus::bed(tile));
    true
}

#[cfg(feature = "gpu-runtime")]
pub(super) struct DriveInput<'a> {
    pub(super) keyboard: &'a ButtonInput<KeyCode>,
    pub(super) hand_ground: Option<Vec3>,
}

#[cfg(feature = "gpu-runtime")]
pub(super) fn drive(
    DriveInput {
        keyboard,
        hand_ground,
    }: DriveInput<'_>,
    selection: &mut Fvr03ProductionVoxelSelectionResource,
    follow: &mut Fvr04ProductionCreatureFollowResource,
    dock: &mut care_dock::State,
    mut runtime: Option<&mut crate::GpuLiveBrainRuntime>,
    frame: &mut LiveBrainPresentationFrameResource,
    ux: &mut Fvr05ProductionUxStateResource,
) {
    let shortcut = keyboard.just_pressed(KeyCode::KeyU);
    let pointer_tool = dock.take_garden();
    if wick_pot::drive(
        keyboard,
        hand_ground,
        shortcut,
        pointer_tool,
        selection,
        follow,
        dock,
        runtime.as_deref_mut(),
    ) {
        return;
    }
    let tool = pointer_tool || shortcut;
    let mut ground = dock.garden_ground_click;
    let mut commit_marker = false;
    if dock.garden_armed {
        let dx = i32::from(keyboard.just_pressed(KeyCode::ArrowRight))
            - i32::from(keyboard.just_pressed(KeyCode::ArrowLeft));
        let dz = i32::from(keyboard.just_pressed(KeyCode::ArrowDown))
            - i32::from(keyboard.just_pressed(KeyCode::ArrowUp));
        if let Some(t) = dock.garden_cursor.filter(|_| dx != 0 || dz != 0) {
            dock.garden_pointer = false;
            let next = VoxelTileCoord::new(t.x + dx, t.z + dz);
            if runtime
                .as_ref()
                .is_some_and(|r| r.world().orchard_gesture(next).is_some())
            {
                dock.garden_cursor = Some(next);
            }
        }
        let cursor = if dock.garden_pointer {
            soil_tile(selection.hovered)
        } else {
            dock.garden_cursor
        };
        if let Some(tile) = cursor.filter(|_| shortcut && !pointer_tool) {
            selection.selected = Some(selected_soil(tile));
            ground = true;
            commit_marker = true;
        }
    }
    if let Some(runtime) = runtime.filter(|_| tool || ground) {
        let was_armed = dock.garden_armed;
        let plan = plan(runtime.world(), selection.selected);
        let soil = if let Plan::Soil(t, _) = plan {
            Some(t)
        } else {
            None
        };
        if let (true, false, Plan::Offer(id)) = (tool, dock.garden_armed, plan) {
            let Some(tile) = care_dock::contextual_target(
                care_dock::Action::Food,
                selection.selected,
                None,
                None,
                runtime.world(),
            )
            .and_then(|target| target.tile) else {
                ux.last_error = Some("No clear place for this offering".into());
                return;
            };
            match runtime.offer_orchard_fruit(id, tile) {
                Ok(id) => {
                    dock.last_gift = Some(id);
                    frame.refresh_world_objects(runtime.world());
                    ux.last_error = None;
                }
                Err(e) => ux.last_error = Some(e.to_string()),
            }
        } else if focus_existing_garden_return(tool, plan, selection, follow, dock) {
            // Return focuses the existing bed. Normal tending needs another gesture.
        } else if let Some(tile) = aim(&mut dock.garden_armed, tool && !commit_marker, ground, soil)
        {
            match runtime.tend_player_orchard(tile) {
                Ok(receipt) => {
                    if let alife_world::orchard::OrchardReceipt::Picked(id) = receipt {
                        dock.last_gift = Some(id);
                    }
                    dock.garden_effect = Some((tile, receipt));
                    if receipt == alife_world::orchard::OrchardReceipt::Watching {
                        wick_pot::observe_watch(dock, runtime.world(), tile);
                    }
                    if matches!(
                        receipt,
                        alife_world::orchard::OrchardReceipt::Sown
                            | alife_world::orchard::OrchardReceipt::Watching
                    ) {
                        follow.enabled = false;
                        dock.hand_focus = Some(care_dock::HandFocus::bed(tile));
                    }
                    frame.refresh_world_objects(runtime.world());
                    ux.last_error = None;
                }
                Err(error) => ux.last_error = Some(error.to_string()),
            }
        }
        if !was_armed && dock.garden_armed {
            dock.garden_pointer = false;
            dock.garden_cursor = starting_soil(runtime.world(), selection.selected);
        }
        if !dock.garden_armed {
            dock.garden_cursor = None;
        }
    }
}
pub(super) fn ordinary_hand(dock: Option<Res<care_dock::State>>) -> bool {
    dock.is_none_or(|d| !d.garden_armed && !d.garden_ground_click)
}

/// A tool press either acts on selected soil or arms exactly one ground click.
/// Never reinterpret an old crop selection after the user has explicitly armed the Hand.
#[cfg(feature = "gpu-runtime")]
pub(super) fn aim(
    armed: &mut bool,
    tool: bool,
    ground_click: bool,
    tile: Option<VoxelTileCoord>,
) -> Option<VoxelTileCoord> {
    if tool {
        if *armed {
            *armed = false;
            return None;
        }
        if tile.is_some() {
            return tile;
        }
        *armed = true;
    } else if *armed && ground_click {
        *armed = false;
        return tile;
    }
    None
}
#[cfg(feature = "gpu-runtime")]
mod plants {
    use super::*;
    use alife_world::orchard::OrchardStage;
    #[derive(Component)]
    pub(in crate::production_voxel_renderer) struct Plant(VoxelTileCoord, OrchardStage);
    #[derive(Default)]
    pub(in crate::production_voxel_renderer) struct AssetsCache(
        Option<([Handle<Mesh>; 4], Handle<StandardMaterial>)>,
        court::Art,
    );

    #[allow(
        clippy::too_many_arguments,
        reason = "Independent authoritative, surface and renderer resources."
    )]
    pub(in crate::production_voxel_renderer) fn sync(
        mut commands: Commands,
        runtime: Option<NonSend<ProductionGpuBrainRuntimeResource>>,
        server: Option<Res<AssetServer>>,
        mut materials: Option<ResMut<Assets<StandardMaterial>>>,
        surface: Option<Res<creature_grounding::RenderedTerrainSurface>>,
        dock: Option<Res<care_dock::State>>,
        mut cache: Local<AssetsCache>,
        mut pot_cache: Local<wick_pot::PotArt>,
        live_seat: court::PreviewInput,
        mut pots: Query<(Entity, &wick_pot::WickPiece, &mut Transform), Without<Plant>>,
        mut rims: court::Rims<'_, '_>,
        mut query: Query<
            (Entity, &mut Plant, &mut Mesh3d, &mut Transform),
            Without<wick_pot::WickPiece>,
        >,
    ) {
        let live = runtime.as_ref().map(|r| r.runtime.world());
        wick_pot::sync(
            live,
            dock.as_deref(),
            server.as_deref(),
            materials.as_deref_mut(),
            &mut pot_cache,
            &mut commands,
            &mut pots,
        );
        let settled = live.and_then(alife_world::HeadlessWorld::orchard_court);
        let ghost = live.zip(surface.as_deref()).and_then(|(world, t)| {
            live_seat.ghost(world, dock.as_ref().map(Res::clone), t, settled)
        });
        court::sync(
            settled,
            ghost,
            server.as_deref(),
            materials.as_deref_mut(),
            &mut cache.1,
            &mut commands,
            &mut rims,
        );
        let beds = live.map_or(&[][..], |w| w.ecology().orchard_beds.as_slice());
        if beds.is_empty() {
            for (id, ..) in &query {
                commands.entity(id).despawn();
            }
            return;
        }
        let (Some(w), Some(server), Some(mut materials), Some(surface)) =
            (live, server, materials, surface)
        else {
            return;
        };
        let (shapes, material) = cache.0.get_or_insert_with(|| {
            let shapes = ["seed", "bud", "bloom", "ripe"].map(|name| {
                server.load(
                    bevy::gltf::GltfAssetLabel::Primitive {
                        mesh: 0,
                        primitive: 0,
                    }
                    .from_asset(format!("care/orchard/{name}.glb")),
                )
            });
            (
                shapes,
                materials.add(StandardMaterial {
                    perceptual_roughness: 0.64,
                    metallic: 0.06,
                    ..default()
                }),
            )
        });
        let model = |stage| {
            shapes[match stage {
                OrchardStage::Seed => 0,
                OrchardStage::Bud => 1,
                OrchardStage::Bloom => 2,
                OrchardStage::Ripe => 3,
            }]
            .clone()
        };
        let pose = |tile: VoxelTileCoord| {
            let x = tile.x as f32 + 0.5;
            let z = tile.z as f32 + 0.5;
            let grounded = surface.height(Vec3::new(x, 0., z)).unwrap_or(0.);
            Transform::from_xyz(x, grounded + 0.016, z).with_rotation(Quat::from_rotation_y(
                (tile.x as f32 * 1.7 + tile.z as f32 * 0.39).rem_euclid(std::f32::consts::TAU),
            ))
        };
        let mut present = std::collections::BTreeSet::new();
        for (id, mut plant, mut mesh, mut transform) in &mut query {
            let Ok(i) = beds.binary_search_by_key(&plant.0, |bed| bed.tile) else {
                commands.entity(id).despawn();
                continue;
            };
            present.insert(plant.0);
            let stage = beds[i].stage(w.tick());
            if plant.1 != stage {
                plant.1 = stage;
                mesh.0 = model(stage);
            }
            transform.set_if_neq(pose(plant.0));
        }
        for b in beds.iter().filter(|b| !present.contains(&b.tile)) {
            let stage = b.stage(w.tick());
            commands.spawn((
                Name::new("Lantern pear"),
                Plant(b.tile, stage),
                groundlight::ContactArea::new(0.46),
                Mesh3d(model(stage)),
                MeshMaterial3d(material.clone()),
                pose(b.tile),
                bevy::picking::Pickable::IGNORE,
            ));
        }
    }
}
#[cfg(feature = "gpu-runtime")]
pub(super) use plants::sync as sync_plants;

#[cfg(all(test, feature = "gpu-runtime"))]
mod tests {
    use super::*;
    #[test]
    fn tool_activation_only_uses_deliberate_soil_and_is_cancelable() {
        let tile = VoxelTileCoord::new(1, 2);
        let mut armed = false;
        assert_eq!(aim(&mut armed, true, false, None), None);
        assert!(armed);
        assert_eq!(aim(&mut armed, false, false, Some(tile)), None);
        assert_eq!(aim(&mut armed, true, false, Some(tile)), None);
        assert!(!armed);
        assert_eq!(aim(&mut armed, false, true, Some(tile)), None);
        assert_eq!(aim(&mut armed, true, false, None), None);
        assert_eq!(aim(&mut armed, false, true, Some(tile)), Some(tile));
        assert!(!armed);
        assert_eq!(aim(&mut armed, true, false, Some(tile)), Some(tile));
        assert!(!armed);
    }
}
