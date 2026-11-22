//! Grounded ceramic court, using the existing Hand's real pointer for fit only.
use super::*;
use alife_world::orchard::{OrchardCourt, OrchardCourtFoot, ORCHARD_COURT_REVIEW};

#[derive(bevy::ecs::system::SystemParam)]
pub(in crate::production_voxel_renderer) struct PreviewInput<'w, 's> {
    hand: Option<Res<'w, god_hand::HandInteraction>>,
    selection: Option<Res<'w, Fvr03ProductionVoxelSelectionResource>>,
    menu: Option<Res<'w, menu::MenuState>>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    mouse: Option<Res<'w, ButtonInput<MouseButton>>>,
    keyboard: Option<Res<'w, ButtonInput<KeyCode>>>,
}

/// A renderer receipt only: no draft record, selection write, or use result.
pub(super) struct Ghost {
    court: OrchardCourt,
    table_pose: Transform,
}

impl PreviewInput<'_, '_> {
    pub(super) fn ghost(
        &self,
        world: &alife_world::HeadlessWorld,
        dock: Option<Res<care_dock::State>>,
        surface: &creature_grounding::RenderedTerrainSurface,
        settled: Option<OrchardCourt>,
    ) -> Option<Ghost> {
        if !ORCHARD_COURT_REVIEW
            || !menu::world_input_allowed(self.menu.as_ref().map(Res::clone))
            || !care_dock::world_input_allowed(dock.as_ref().map(Res::clone))
            || !super::ordinary_hand(dock)
            || self.keyboard.as_ref()?.just_pressed(KeyCode::Escape)
        {
            return None;
        }
        let w = self.windows.single().ok()?;
        if !w.focused || !w.cursor_position().is_some_and(|p| p.is_finite()) {
            return None;
        }
        let hand = self.hand.as_ref()?;
        let selection = self.selection.as_ref()?;
        let ray = hand.ground.filter(|p| p.is_finite())?;
        let hovered = selection.hovered?;
        let held = match world.player_held_object() {
            Some(id) if hand.held == Some(id)
                && self.mouse.as_ref()?.pressed(MouseButton::Left) => Some(id),
            Some(_) => return None, // Restored or foreign World hold has no local pointer witness.
            None if hand.held.is_none() => None,
            None => return None,
        };
        let id = held.or_else(|| {
            selection.selected.filter(|s| s.kind == StableVoxelRefKind::Resource)?.stable_id
        })?;
        let tile = VoxelTileCoord::new(ray.x.floor() as i32, ray.z.floor() as i32);
        if !(super::soil_tile(Some(hovered)) == Some(tile)
            || (held == Some(id) && hovered.kind == StableVoxelRefKind::Resource
                && hovered.stable_id == Some(id)))
        {
            // Never preview through a different first body in the existing picker.
            return None;
        }
        let (at, court) = world.orchard_court_preview(id, Vec3f::new(ray.x, ray.y, ray.z))?;
        if settled.is_some_and(|existing| existing.bed == court.bed && existing.axis == court.axis) {
            // Standing inside the existing place does not stack a second translucent aisle.
            return None;
        }
        let [x, y, z] = world.entity(id)?.grounded_physical.shape;
        let body_size = Vec3::new(x, y, z) * 0.3 + Vec3::splat(0.1);
        let center = live_food_projection::care_object_center(at, true, surface);
        Some(Ghost { court, table_pose: Transform::from_translation(center).with_scale(body_size) })
    }
}

#[derive(Component)]
pub(in crate::production_voxel_renderer) struct Rim(usize);

struct Pieces {
    ceramic: Handle<Mesh>,
    chime: Handle<Mesh>,
    clay: Handle<StandardMaterial>,
    ghost: Handle<StandardMaterial>,
}

#[derive(Default)]
pub(in crate::production_voxel_renderer) struct Art(Option<Pieces>);

/// Ground the signed ceramic's actual bottom on the existing surveyed normal.
fn rim_pose(contact: OrchardCourtFoot, axis: [i32; 2]) -> Transform {
    let [gx, gz] = contact.rise;
    let up = Vec3::new(-gx, 1.0, -gz).normalize();
    let full = Vec3::new(contact.ground.x, contact.ground.y, contact.ground.z)
        + up * (0.0454 * 1.08 + 0.008);
    let turn = (axis[0] as f32).atan2(axis[1] as f32);
    Transform::from_translation(full)
        .with_rotation(Quat::from_rotation_arc(Vec3::Y, up) * Quat::from_rotation_y(turn))
        .with_scale(Vec3::new(0.49, 1.08, 0.74))
}

#[allow(
    clippy::type_complexity,
    reason = "Distinct transform queries keep the two Orchard dressings independent."
)]
pub(super) fn sync(
    court: Option<OrchardCourt>,
    preview: Option<Ghost>,
    server: Option<&AssetServer>,
    materials: Option<&mut Assets<StandardMaterial>>,
    state: &mut Art,
    commands: &mut Commands,
    found: &mut Query<
        (Entity, &Rim, &mut Transform),
        (Without<super::plants::Plant>, Without<super::wick_pot::WickPiece>),
    >,
) {
    if court.is_none() && preview.is_none() {
        for (entity, ..) in found.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }
    let (Some(server), Some(materials)) = (server, materials) else {
        return;
    };
    let form = state.0.get_or_insert_with(|| {
        let source = |path| server.load(
            bevy::gltf::GltfAssetLabel::Primitive { mesh: 0, primitive: 0 }.from_asset(path),
        );
        Pieces {
            ceramic: source("care/ceramic-waystone.glb"),
            chime: source("care/chime-table.glb"),
            clay: materials.add(StandardMaterial {
                base_color: Color::srgb(0.76, 0.64, 0.51),
                perceptual_roughness: 0.78,
                ..default()
            }),
            ghost: materials.add(StandardMaterial {
                base_color: Color::srgba(0.92, 0.85, 0.71, 0.48),
                perceptual_roughness: 0.78,
                alpha_mode: AlphaMode::Blend,
                ..default()
            }),
        }
    });
    let mut wanted = [None; 9];
    if let Some(c) = court {
        for (i, f) in c.rims.into_iter().enumerate() {
            wanted[i] = Some(rim_pose(f, c.axis));
        }
    }
    if let Some(g) = preview {
        for (i, f) in g.court.rims.into_iter().enumerate() {
            wanted[i + 4] = Some(rim_pose(f, g.court.axis));
        }
        wanted[8] = Some(g.table_pose);
    }
    for (entity, rim, mut transform) in found.iter_mut() {
        if let Some(pose) = wanted.get_mut(rim.0).and_then(Option::take) {
            transform.set_if_neq(pose);
        } else {
            commands.entity(entity).despawn();
        }
    }
    for (i, pose) in wanted.into_iter().enumerate() {
        let Some(pose) = pose else { continue; };
        let preview_part = i >= 4;
        let mesh = if i == 8 { &form.chime } else { &form.ceramic };
        let skin = if preview_part { &form.ghost } else { &form.clay };
        let mut body = commands.spawn((
            Rim(i),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(skin.clone()),
            pose,
            Fvr04ProductionRuntimeSceneRoot,
            bevy::picking::Pickable::IGNORE,
        ));
        if preview_part {
            body.insert(bevy::light::NotShadowCaster);
        }
    }
}

#[cfg(all(test, feature = "qualia-village-review"))]
mod tests {
    use super::*;
    use bevy::{
        camera::{CameraProjection, OrthographicProjection, RenderTargetInfo, ScalingMode},
        ecs::system::SystemState,
        math::UVec2,
        window::WindowResolution,
    };

    const ROOT: VoxelTileCoord = VoxelTileCoord::new(79, 250);
    const OTHER: VoxelTileCoord = VoxelTileCoord::new(79, 252);
    const AISLE: VoxelTileCoord = VoxelTileCoord::new(80, 250);

    fn ground(world: &alife_world::HeadlessWorld, x: f32, z: f32) -> Vec3f {
        Vec3f::new(x, world.terrain().unwrap().surface().height(x, z).unwrap(), z)
    }

    fn xyz(v: Vec3f) -> Vec3 {
        Vec3::new(v.x, v.y, v.z)
    }

    fn camera() -> (Camera, GlobalTransform) {
        let (width, height) = (1280, 720);
        let extent = 9.8;
        let mut projection = OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height: extent },
            scale: 1.0,
            near: -200.0,
            far: 500.0,
            ..OrthographicProjection::default_3d()
        };
        projection.update(width as f32, height as f32);
        let mut camera = Camera::default();
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(width, height),
            scale_factor: 1.0,
        });
        camera.computed.clip_from_view = projection.get_clip_from_view();
        let transform = production_follow_camera_transform(
            Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
            extent,
            Vec3::new(81.5, 6.9, 251.5),
        );
        (camera, GlobalTransform::from(transform))
    }

    fn aim_screen(
        ecs: &mut World,
        window_id: Entity,
        camera: &Camera,
        eye: &GlobalTransform,
        world: &alife_world::HeadlessWorld,
        surface: &creature_grounding::RenderedTerrainSurface,
        target: Vec3,
    ) -> (Vec3, StableVoxelObjectRef) {
        let pixels = camera.world_to_viewport(eye, target).unwrap();
        {
            let mut window = ecs.get_mut::<Window>(window_id).unwrap();
            window.set_cursor_position(Some(pixels));
        }
        let frame = LiveBrainPresentationFrame::try_new(Vec::new(), world.tick(), world.object_snapshots()).unwrap();
        let sample = sample_fvr03_world_pointer(
            ecs.get::<Window>(window_id).unwrap(), camera, eye, None, world.terrain(),
            Some(&frame), surface, || god_hand::chime_body_pick_id(world),
        ).unwrap();
        let selected = ecs.resource::<ButtonInput<MouseButton>>().just_pressed(MouseButton::Left);
        // Apply the production sample transition; the two resources are temporarily
        // separated only to let a bare ECS test borrow them without a schedule.
        let mut hand = ecs.remove_resource::<god_hand::HandInteraction>().unwrap();
        {
            let mut selection = ecs.resource_mut::<Fvr03ProductionVoxelSelectionResource>();
            publish_fvr03_world_pointer(&mut hand, &mut selection, Some(sample), selected);
        }
        ecs.insert_resource(hand);
        sample
    }

    fn visible_preview(
        ecs: &mut World,
        world: &alife_world::HeadlessWorld,
        surface: &creature_grounding::RenderedTerrainSurface,
    ) -> Option<Ghost> {
        let mut param = SystemState::<(PreviewInput, Option<Res<care_dock::State>>)>::new(ecs);
        let (input, dock) = param.get_mut(ecs);
        input.ghost(world, dock, surface, world.orchard_court())
    }

    #[test]
    fn signed_viewport_chime_preview_garden_return_and_late_seat_refusal() {
        use alife_world::orchard::{OrchardGesture, OrchardReceipt};

        assert!(ORCHARD_COURT_REVIEW);
        let mut world = alife_world::HeadlessWorld::new(1643);
        world.enable_island_for_new_game().unwrap();
        assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Sown);
        world.try_advance_tick().unwrap();
        assert_eq!(world.tend_orchard(OTHER).unwrap(), OrchardReceipt::Sown);
        let initial = ground(&world, 80.0, 251.5);
        let chime = world.spawn_toy("player-activity-toy", initial, false).unwrap();
        let mut surface = creature_grounding::RenderedTerrainSurface::from_meshes(
            std::iter::empty::<&Mesh>(), 1.0,
        );
        surface.set_surface(Some(world.terrain().unwrap().surface().clone()));

        // No App runner, window plugin, device or operating-system click:
        // Window is an in-memory input record, Camera uses exact viewport math.
        let mut ecs = World::new();
        let window = ecs.spawn((
            Window { focused: true, resolution: WindowResolution::new(1280, 720), ..default() },
            PrimaryWindow,
        )).id();
        ecs.insert_resource(Fvr03ProductionVoxelSelectionResource { hovered: None, selected: None });
        ecs.init_resource::<ButtonInput<MouseButton>>();
        ecs.init_resource::<ButtonInput<KeyCode>>();
        ecs.init_resource::<god_hand::HandInteraction>();
        ecs.init_resource::<care_dock::State>();
        let (camera, eye) = camera();
        let set = xyz(ground(&world, 83.5, 250.5));

        ecs.resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
        let contact = live_food_projection::care_object_center(initial, true, &surface);
        let (_, hit) = aim_screen(&mut ecs, window, &camera, &eye, &world, &surface, contact);
        assert_eq!((hit.kind, hit.stable_id), (StableVoxelRefKind::Resource, Some(chime)));
        assert_eq!(ecs.resource::<Fvr03ProductionVoxelSelectionResource>().selected, Some(hit));
        ecs.resource_mut::<ButtonInput<MouseButton>>().clear_just_pressed(MouseButton::Left);

        let (seat, target) = aim_screen(&mut ecs, window, &camera, &eye, &world, &surface, set);
        assert_eq!(target.kind, StableVoxelRefKind::Tile);
        assert!(seat.distance(set) < 0.01, "actual signed ground ray, not test-adjusted aim");
        let at = Vec3f::new(seat.x, seat.y, seat.z);
        let stable = world.canonical_signature_digest().unwrap();
        let before = visible_preview(&mut ecs, &world, &surface).unwrap();
        assert_eq!((before.court.chime, before.court.bed, before.court.aisle), (chime, ROOT, AISLE));
        assert!(before.court.open && world.orchard_court().is_none());
        assert_eq!(world.canonical_signature_digest().unwrap(), stable);

        // Real World authority advances through its public Hand, not a
        // fabricated renderer position or a simulated full GPU App input loop.
        world.begin_player_hold(chime).unwrap();
        ecs.resource_mut::<god_hand::HandInteraction>().held = Some(chime);
        assert!(visible_preview(&mut ecs, &world, &surface).is_none(), "no accepted held aim yet");
        world.move_player_hold(at).unwrap();
        assert_eq!(visible_preview(&mut ecs, &world, &surface).unwrap().court, before.court);

        // Genuine late native body arrives on the accepted foot. Both the
        // new real top-ranked ray and the direct World release must now refuse.
        let blocker = world.spawn_toy("late-court-proof-ball", at, true).unwrap();
        let (_, blocked_hit) = aim_screen(&mut ecs, window, &camera, &eye, &world, &surface, set);
        assert_eq!((blocked_hit.kind, blocked_hit.stable_id), (StableVoxelRefKind::Resource, Some(blocker)));
        let blocked = world.canonical_signature_digest().unwrap();
        assert!(visible_preview(&mut ecs, &world, &surface).is_none());
        assert_eq!(world.orchard_court_preview(chime, at), None);
        assert!(world.release_player_hold().is_err());
        assert_eq!(world.player_held_object(), Some(chime));
        assert_eq!(world.orchard_court(), None);
        assert_eq!(world.canonical_signature_digest().unwrap(), blocked);

        let away = ground(&world, 86.0, 253.5);
        world.editor_move_object(blocker, away).unwrap();
        let (reaim, _) = aim_screen(&mut ecs, window, &camera, &eye, &world, &surface, set);
        let again = Vec3f::new(reaim.x, reaim.y, reaim.z);
        world.move_player_hold(again).unwrap();
        assert_eq!(visible_preview(&mut ecs, &world, &surface).unwrap().court, before.court);
        assert_eq!(world.release_player_hold().unwrap(), Some(chime));
        assert_eq!(world.orchard_court(), Some(before.court));
        ecs.resource_mut::<god_hand::HandInteraction>().held = None;
        ecs.resource_mut::<ButtonInput<MouseButton>>().release(MouseButton::Left);

        // The real ground ray enters the physical apron, rather than constructing
        // any StableVoxelObjectRef. Run the same Garden Return branch as drive.
        ecs.resource_mut::<ButtonInput<MouseButton>>().press(MouseButton::Left);
        let aisle_position = xyz(ground(&world, 80.5, 250.5));
        let (_, aisle) = aim_screen(&mut ecs, window, &camera, &eye, &world, &surface, aisle_position);
        assert_eq!(super::super::soil_tile(Some(aisle)), Some(AISLE));
        let plan = super::super::plan(&world, Some(aisle));
        assert!(matches!(plan, super::super::Plan::Return(t) if t == ROOT));
        ecs.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyU);
        let shortcut = ecs.resource::<ButtonInput<KeyCode>>().just_pressed(KeyCode::KeyU);
        let mut follow = Fvr04ProductionCreatureFollowResource { enabled: true, target_stable_id: None };
        let mut selection = ecs.remove_resource::<Fvr03ProductionVoxelSelectionResource>().unwrap();
        let before_return = world.canonical_signature_digest().unwrap();
        let mut dock = ecs.resource_mut::<care_dock::State>();
        assert!(super::super::focus_existing_garden_return(shortcut, plan, &mut selection, &mut follow, &mut dock));
        assert!(!follow.enabled && !dock.garden_armed && dock.garden_effect.is_none());
        let expected = super::super::selected_soil(ROOT);
        assert_eq!(selection.selected, Some(expected));
        let focus = dock.hand_focus.unwrap();
        assert_eq!(focus.guard, Some(expected));
        assert_eq!(focus.point, bevy::math::Vec2::new(79.5, 250.5));
        assert_eq!(world.canonical_signature_digest().unwrap(), before_return);
        // Return focuses the original root, never the later OTHER planting,
        // and it doesn't silently repeat the next Garden action.
        assert!(matches!(super::super::plan(&world, selection.selected),
                         super::super::Plan::Soil(t, OrchardGesture::Watch) if t==ROOT));

        // Continue the same selected court and garden. A true harvested pear
        // must suppress the renderer's Wick grip even when ordinary Watch
        // still works. The same existing grip returns after that pear is moved.
        let observe_watch = |world: &mut alife_world::HeadlessWorld, dock: &mut care_dock::State| {
            let stable = world.canonical_signature_digest().unwrap();
            assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Watching);
            super::super::wick_pot::observe_watch(dock, world, ROOT);
            assert_eq!(world.canonical_signature_digest().unwrap(), stable);
        };
        observe_watch(&mut world, &mut dock);
        let first_grip = dock.wick_watch.expect("ordinary Watch presents the original young Wick");
        assert_eq!((first_grip.tile, first_grip.last_fruit), (ROOT, None));
        let other_bed = *world.ecology().orchard_beds.iter().find(|b| b.tile == OTHER).unwrap();
        assert_eq!(world.tick().raw(), 1);
        for _ in 1..80 {
            world.try_advance_tick().unwrap();
        }
        assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Watered);
        for _ in 0..220 {
            world.try_advance_tick().unwrap();
        }
        let OrchardReceipt::Picked(pear) = world.tend_orchard(ROOT).unwrap() else {
            panic!("the same returned bed must supply a native ripe pear");
        };
        let tray = world.orchard_court().expect("settled court persists with its real crop");
        assert_eq!((tray.chime, tray.bed, tray.open), (chime, ROOT, false));
        let fruit = world.entity(pear).unwrap();
        assert_eq!((fruit.kind, fruit.position), (WorldObjectKind::Food, tray.landing_ground));
        // Keep the same root growing after harvest; the difference below is
        // only actual pear clearance, not a change of age, root or held state.
        for _ in 0..80 {
            world.try_advance_tick().unwrap();
        }
        assert_eq!(world.tend_orchard(ROOT).unwrap(), OrchardReceipt::Watered);
        let root = *world.ecology().orchard_beds.iter().find(|b| b.tile == ROOT).unwrap();
        assert!(root.watered && root.last_fruit == Some(pear) && !root.offered);
        assert!(world.player_held_object().is_none());
        observe_watch(&mut world, &mut dock);
        assert!(dock.wick_watch.is_none(), "the native tray pear must clear the old offered grip");

        // Native World Hand only, just as for Chime setup above: this does not
        // simulate a successful GPU Garden drag/commit or a customer click.
        let parking = ground(&world, 77.5, 246.5);
        world.begin_player_hold(pear).unwrap();
        world.move_player_hold(parking).unwrap();
        assert_eq!(world.release_player_hold().unwrap(), Some(pear));
        assert_eq!(world.entity(pear).unwrap().position, parking);
        assert!(world.orchard_court().unwrap().open);
        assert!(world.player_held_object().is_none());
        observe_watch(&mut world, &mut dock);
        assert_eq!(dock.wick_watch, Some(root), "exact rewatered Wick presents the grip after real clearance");
        assert_eq!(world.ecology().orchard_beds.as_slice(), &[root, other_bed]);
        assert_eq!(selection.selected, Some(expected));
    }
}
