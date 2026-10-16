//! Care objects follow canonical physical appearance, placement and carrying.
use super::*;
use bevy::prelude::{Cuboid, Meshable, Query, Sphere};

#[derive(Component)]
pub(super) struct LiveCareObject(pub(super) WorldEntityId);

/// Care-object geometry, picking and hand contact share this presentation center.
/// Terrain-bound world positions already include elevation; unbound positions
/// retain their canonical Y above the rendered surface (or the existing fallback).
pub(super) fn care_object_center(
    position: Vec3f,
    terrain_bound: bool,
    surface: &creature_grounding::RenderedTerrainSurface,
) -> Vec3 {
    let rendered = world_position_for_render(position, terrain_bound);
    let ground = Vec3::new(rendered.x, 0.0, rendered.z);
    let height = if terrain_bound {
        rendered.y
    } else {
        surface.height(ground).unwrap_or(0.44) + rendered.y
    };
    ground + Vec3::Y * (height + 0.30)
}

#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS system parameters."
)]
pub(super) fn sync_care_objects(
    mut commands: Commands,
    highlands: Option<Res<creature_grounding::SelectedTerrain>>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: Local<Option<(Handle<Mesh>, Handle<Mesh>)>>,
    mut object_entities: Query<(Entity, &LiveCareObject, &mut Transform)>,
) {
    let Some(frame) = frame else {
        return;
    };
    if !frame.is_changed() {
        return;
    }
    let mut objects: BTreeMap<_, _> = frame
        .current
        .objects()
        .filter(|object| {
            !object.consumed
                && matches!(
                    object.kind,
                    WorldObjectKind::Food | WorldObjectKind::Ball | WorldObjectKind::ActivityToy
                )
        })
        .map(|object| (object.id.raw(), object))
        .collect();
    let translation = |position| care_object_center(position, highlands.is_some(), &surface);
    for (entity, marker, mut transform) in &mut object_entities {
        if let Some(object) = objects.remove(&marker.0.raw()) {
            transform.translation = translation(object.position);
        } else {
            commands.entity(entity).despawn();
        }
    }
    if objects.is_empty() {
        return;
    }
    let (sphere, station) = assets.get_or_insert_with(|| {
        (
            meshes.add(Sphere::new(1.0).mesh().uv(16, 12)),
            meshes.add(Cuboid::new(2.0, 2.0, 2.0)),
        )
    });
    for (id, object) in objects {
        let physical = object.grounded_physical;
        let material = materials.add(StandardMaterial {
            base_color: Color::srgb(physical.color[0], physical.color[1], physical.color[2]),
            perceptual_roughness: physical.material[0].clamp(0.05, 1.0),
            ..default()
        });
        let shape = Vec3::new(physical.shape[0], physical.shape[1], physical.shape[2]) * 0.3
            + Vec3::splat(0.1);
        commands
            .spawn((
                Name::new(object.label.clone()),
                LiveCareObject(WorldEntityId(id)),
                Fvr04ProductionRuntimeSceneRoot,
                Transform::from_translation(translation(object.position)),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(if object.kind == WorldObjectKind::ActivityToy {
                        station.clone()
                    } else {
                        sphere.clone()
                    }),
                    MeshMaterial3d(material),
                    Transform::from_scale(shape),
                ));
            });
    }
}

#[cfg(test)]
#[derive(Resource, Default)]
pub(super) struct CarrierOffsetScanCounts {
    builds: usize,
    visits: usize,
}

/// Follow the displayed carrier between world ticks without changing physical placement.
/// Always start from the snapshot, so offsets cannot accumulate or survive release.
pub(super) fn align_carried_care_objects(
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    roots: Query<(&ProductionCreatureAssemblyRoot, &Transform), Without<LiveCareObject>>,
    mut objects: Query<(&LiveCareObject, &mut Transform), Without<ProductionCreatureAssemblyRoot>>,
    #[cfg(test)] mut scan_counts: Option<ResMut<CarrierOffsetScanCounts>>,
) {
    let Some(frame) = frame else {
        return;
    };
    // Rebuild from displayed roots only if this invocation needs a carrier.
    let mut carrier_offsets: Option<BTreeMap<_, _>> = None;
    for (marker, mut transform) in &mut objects {
        let Some(object) = frame.current.object(marker.0) else {
            continue;
        };
        let offset = object
            .carried_by
            .and_then(|carrier| {
                let offsets = carrier_offsets.get_or_insert_with(|| {
                    #[cfg(test)]
                    if let Some(counts) = scan_counts.as_deref_mut() {
                        counts.builds += 1;
                    }
                    roots
                        .iter()
                        .filter_map(|(root, transform)| {
                            #[cfg(test)]
                            if let Some(counts) = scan_counts.as_deref_mut() {
                                counts.visits += 1;
                            }
                            let carrier = frame.current.object(root.stable_id)?;
                            (carrier.kind == WorldObjectKind::Agent
                                && carrier.organism_id == Some(root.organism_id))
                            .then_some((
                                root.organism_id.raw(),
                                transform.translation
                                    - world_position_for_render(carrier.position, false),
                            ))
                        })
                        .collect()
                });
                offsets.get(&carrier.raw()).copied()
            })
            .unwrap_or(Vec3::ZERO);
        // Hearthling heading follows world yaw directly; retain the world's grip
        // offset and existing object height, correcting only the smoothed X/Z.
        let position = world_position_for_render(object.position, false) + offset;
        transform.translation.x = position.x;
        transform.translation.z = position.z;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn care_feedback_projection_tracks_placement_and_consumption_without_advancing_time() {
        for highlands in [false, true] {
            let position = Vec3f::new(2.5, 4.0, -3.5);
            let world = alife_world::HeadlessScenarioBuilder::new(7)
                .food("apple", position, 0.25)
                .toy("player-plaything", position, true)
                .build()
                .unwrap();
            let mut app = App::new();
            app.insert_resource(
                LiveBrainPresentationFrameResource::from_authoritative_world(&world).unwrap(),
            )
            .insert_resource(creature_grounding::RenderedTerrainSurface::from_meshes(
                std::iter::empty(),
                1.0,
            ))
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<StandardMaterial>::default())
            .add_systems(Update, sync_care_objects);
            if highlands {
                app.insert_resource(creature_grounding::SelectedTerrain(
                    alife_world::island_terrain(),
                ));
            }
            app.update();
            let mut query = app
                .world_mut()
                .query_filtered::<&Transform, With<LiveCareObject>>();
            assert_eq!(query.iter(app.world()).count(), 2);
            let fruit = query.iter(app.world()).next().unwrap();
            assert_eq!(fruit.translation.x, 2.5);
            assert_eq!(fruit.translation.z, -3.5);
            assert!((fruit.translation.y - if highlands { 4.30 } else { 4.74 }).abs() < 1e-5);
            let mut objects = world.object_snapshots();
            let toy = objects
                .iter_mut()
                .find(|object| object.kind == WorldObjectKind::Ball)
                .unwrap();
            toy.position.x += 2.0;
            let moved =
                LiveBrainPresentationFrame::try_new(Vec::new(), world.tick(), objects.clone())
                    .unwrap();
            app.world_mut()
                .resource_mut::<LiveBrainPresentationFrameResource>()
                .current = moved;
            app.update();
            assert!(query
                .iter(app.world())
                .any(|transform| transform.translation.x == 4.5));
            objects.iter_mut().for_each(|object| object.consumed = true);
            let consumed =
                LiveBrainPresentationFrame::try_new(Vec::new(), world.tick(), objects).unwrap();
            app.world_mut()
                .resource_mut::<LiveBrainPresentationFrameResource>()
                .current = consumed;
            app.update();
            assert_eq!(query.iter(app.world()).count(), 0);
            assert_eq!(
                app.world()
                    .resource::<LiveBrainPresentationFrameResource>()
                    .current
                    .authoritative_world_tick,
                world.tick()
            );
        }
    }

    fn surface_with_height(height: Option<f32>) -> creature_grounding::RenderedTerrainSurface {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        if let Some(height) = height {
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![
                    [0.0, height, 0.0],
                    [0.0, height, 1.0],
                    [1.0, height, 1.0],
                    [1.0, height, 0.0],
                ],
            );
        }
        creature_grounding::RenderedTerrainSurface::from_meshes([&mesh], 1.0)
    }

    #[test]
    fn care_object_center_preserves_canonical_height_and_uses_only_unbound_surface_lift() {
        let position = Vec3f::new(0.5, 2.0, 0.5);
        for (terrain_bound, surface_height, expected_y) in [
            (false, Some(0.9), 3.2),
            (false, None, 2.74),
            (true, Some(0.9), 2.3),
            (true, None, 2.3),
        ] {
            let surface = surface_with_height(surface_height);
            let center = care_object_center(position, terrain_bound, &surface);
            assert_eq!(center.x, position.x);
            assert_eq!(center.z, position.z);
            assert!((center.y - expected_y).abs() < 1e-5);
        }
    }

    #[test]
    fn care_object_render_and_both_pick_paths_share_measured_and_fallback_centers() {
        for terrain_bound in [false, true] {
            for surface_height in [Some(0.9), None] {
                let world = alife_world::HeadlessScenarioBuilder::new(7)
                    .food("apple", Vec3f::new(0.5, 2.0, 0.5), 0.25)
                    .build()
                    .unwrap();
                let original = world.object_snapshots();
                let id = original[0].id;
                let mut app = App::new();
                app.insert_resource(
                    LiveBrainPresentationFrameResource::from_authoritative_world(&world).unwrap(),
                )
                .insert_resource(surface_with_height(surface_height))
                .insert_resource(Assets::<Mesh>::default())
                .insert_resource(Assets::<StandardMaterial>::default())
                .add_systems(Update, sync_care_objects);
                if terrain_bound {
                    app.insert_resource(creature_grounding::SelectedTerrain(
                        alife_world::island_terrain(),
                    ));
                }
                for y in [2.0, 2.0, 5.0, 5.0, 2.0] {
                    let mut objects = original.clone();
                    objects[0].position.y = y;
                    let frame =
                        LiveBrainPresentationFrame::try_new(Vec::new(), world.tick(), objects)
                            .unwrap();
                    app.world_mut()
                        .resource_mut::<LiveBrainPresentationFrameResource>()
                        .current = frame;
                    app.update();
                    let mut query = app.world_mut().query::<(&LiveCareObject, &Transform)>();
                    let (marker, transform) = query.single(app.world()).unwrap();
                    assert_eq!(marker.0, id);
                    let center = transform.translation;
                    let frame = &app
                        .world()
                        .resource::<LiveBrainPresentationFrameResource>()
                        .current;
                    let surface = app
                        .world()
                        .resource::<creature_grounding::RenderedTerrainSurface>();
                    assert_eq!(
                        center,
                        care_object_center(
                            frame.object(id).unwrap().position,
                            terrain_bound,
                            surface
                        )
                    );
                    let ray = bevy::math::Ray3d::new(center - Vec3::X * 2.0, bevy::math::Dir3::X);
                    let ground_hit = Vec3::new(4.0, 0.0, 0.5);
                    assert_eq!(
                        pick_live_object_body(frame, terrain_bound, surface, &ray, ground_hit)
                            .unwrap()
                            .1,
                        id
                    );
                    assert_eq!(
                        pick_live_food(frame, terrain_bound, surface, &ray, ground_hit)
                            .unwrap()
                            .1,
                        id
                    );
                    if !terrain_bound {
                        // The former primary/contact center was below the visible food.
                        let wrong = Vec3::new(0.5, y + 0.30, 0.5);
                        let wrong_ray =
                            bevy::math::Ray3d::new(wrong - Vec3::X * 2.0, bevy::math::Dir3::X);
                        assert!(pick_live_object_body(
                            frame,
                            terrain_bound,
                            surface,
                            &wrong_ray,
                            ground_hit
                        )
                        .is_none());
                        assert!(pick_live_food(
                            frame,
                            terrain_bound,
                            surface,
                            &wrong_ray,
                            ground_hit
                        )
                        .is_none());
                    }
                    assert_eq!(frame.authoritative_world_tick, world.tick());
                    assert_eq!(frame.object(id).unwrap().position.y, y);
                }
                assert_eq!(world.object_snapshots(), original);
            }
        }
    }

    #[test]
    fn both_food_pick_paths_reject_carried_and_consumed_objects_and_recover_after_drop() {
        let world = alife_world::HeadlessScenarioBuilder::new(7)
            .food("apple", Vec3f::new(0.5, 2.0, 0.5), 0.25)
            .build()
            .unwrap();
        let original = world.object_snapshots();
        let id = original[0].id;
        for terrain_bound in [false, true] {
            let surface = surface_with_height(Some(0.9));
            let center = care_object_center(original[0].position, terrain_bound, &surface);
            let ray = bevy::math::Ray3d::new(center - Vec3::X * 2.0, bevy::math::Dir3::X);
            let ground_hit = Vec3::new(4.0, 0.0, 0.5);
            for (carried_by, consumed) in [
                (None, false),
                (Some(OrganismId(11)), false),
                (Some(OrganismId(11)), false),
                (None, false),
                (None, true),
                (None, false),
            ] {
                let mut objects = original.clone();
                objects[0].carried_by = carried_by;
                objects[0].consumed = consumed;
                let frame =
                    LiveBrainPresentationFrame::try_new(Vec::new(), world.tick(), objects).unwrap();
                let expected = (carried_by.is_none() && !consumed).then_some(id);
                assert_eq!(
                    pick_live_object_body(&frame, terrain_bound, &surface, &ray, ground_hit)
                        .map(|hit| hit.1),
                    expected
                );
                assert_eq!(
                    pick_live_food(&frame, terrain_bound, &surface, &ray, ground_hit)
                        .map(|hit| hit.1),
                    expected
                );
            }
        }
        assert_eq!(world.object_snapshots(), original);
    }

    #[test]
    fn ball_primary_pick_uses_visual_height_and_preserves_eligibility() {
        let world = alife_world::HeadlessScenarioBuilder::new(7)
            .toy("ball", Vec3f::new(0.5, 2.0, 0.5), true)
            .build()
            .unwrap();
        let original = world.object_snapshots();
        assert_eq!(original[0].kind, WorldObjectKind::Ball);
        for terrain_bound in [false, true] {
            for height in [Some(0.9), None] {
                let surface = surface_with_height(height);
                let center = care_object_center(original[0].position, terrain_bound, &surface);
                let ray = bevy::math::Ray3d::new(center - Vec3::X * 2.0, bevy::math::Dir3::X);
                let ground_hit = Vec3::new(4.0, 0.0, 0.5);
                for (carried_by, consumed) in
                    [(None, false), (Some(OrganismId(11)), false), (None, true)]
                {
                    let mut objects = original.clone();
                    objects[0].carried_by = carried_by;
                    objects[0].consumed = consumed;
                    let frame =
                        LiveBrainPresentationFrame::try_new(Vec::new(), world.tick(), objects)
                            .unwrap();
                    let expected = (carried_by.is_none() && !consumed).then_some(original[0].id);
                    assert_eq!(
                        pick_live_object_body(&frame, terrain_bound, &surface, &ray, ground_hit)
                            .map(|hit| hit.1),
                        expected
                    );
                    assert!(
                        pick_live_food(&frame, terrain_bound, &surface, &ray, ground_hit).is_none()
                    );
                }
            }
        }
        assert_eq!(world.object_snapshots(), original);
    }

    #[test]
    fn primary_pick_preserves_agent_and_token_height_rules() {
        let world = alife_world::HeadlessScenarioBuilder::new(7)
            .agent("walker", OrganismId(11), Vec3f::new(0.5, 2.0, 0.5))
            .token("token", Vec3f::new(0.5, 2.0, 0.5), 1)
            .build()
            .unwrap();
        let surface = surface_with_height(Some(9.0));
        for object in world.object_snapshots() {
            let frame =
                LiveBrainPresentationFrame::try_new(Vec::new(), world.tick(), vec![object.clone()])
                    .unwrap();
            let lift = if object.kind == WorldObjectKind::Agent {
                0.90
            } else {
                0.30
            };
            let center = world_position_for_render(object.position, false) + Vec3::Y * lift;
            let ray = bevy::math::Ray3d::new(center - Vec3::X * 2.0, bevy::math::Dir3::X);
            assert_eq!(
                pick_live_object_body(&frame, false, &surface, &ray, Vec3::new(4.0, 0.0, 0.5))
                    .unwrap()
                    .1,
                object.id
            );
        }
    }

    // Frozen from main 73f8ecf03: keep independent of the lazy implementation.
    fn align_carried_care_objects_frozen_baseline(
        frame: Option<Res<LiveBrainPresentationFrameResource>>,
        roots: Query<(&ProductionCreatureAssemblyRoot, &Transform), Without<LiveCareObject>>,
        mut objects: Query<
            (&LiveCareObject, &mut Transform),
            Without<ProductionCreatureAssemblyRoot>,
        >,
    ) {
        let Some(frame) = frame else {
            return;
        };
        let carrier_offsets: BTreeMap<_, _> = roots
            .iter()
            .filter_map(|(root, transform)| {
                let carrier = frame.current.object(root.stable_id)?;
                (carrier.kind == WorldObjectKind::Agent
                    && carrier.organism_id == Some(root.organism_id))
                .then_some((
                    root.organism_id.raw(),
                    transform.translation - world_position_for_render(carrier.position, false),
                ))
            })
            .collect();
        for (marker, mut transform) in &mut objects {
            let Some(object) = frame.current.object(marker.0) else {
                continue;
            };
            let offset = object
                .carried_by
                .and_then(|carrier| carrier_offsets.get(&carrier.raw()))
                .copied()
                .unwrap_or(Vec3::ZERO);
            // Hearthling heading follows world yaw directly; retain the world's grip
            // offset and existing object height, correcting only the smoothed X/Z.
            let position = world_position_for_render(object.position, false) + offset;
            transform.translation.x = position.x;
            transform.translation.z = position.z;
        }
    }

    #[derive(Resource, Default)]
    struct DisplayedCarrierPositions(BTreeMap<u64, Vec3>);

    fn present_carrier_positions(
        positions: Res<DisplayedCarrierPositions>,
        mut roots: Query<(&ProductionCreatureAssemblyRoot, &mut Transform)>,
    ) {
        for (root, mut transform) in &mut roots {
            if let Some(position) = positions.0.get(&root.organism_id.raw()) {
                transform.translation = *position;
            }
        }
    }

    fn carried_fixture(highlands: bool) -> (App, Vec<alife_world::WorldObject>, [Entity; 2]) {
        let world = alife_world::HeadlessScenarioBuilder::new(7)
            .agent("carrier", OrganismId(11), Vec3f::new(4.0, 2.0, 6.0))
            .agent("other", OrganismId(22), Vec3f::new(-8.0, 2.0, -9.0))
            .food("held", Vec3f::new(4.5, 2.0, 6.0), 0.25)
            .food("loose", Vec3f::new(10.0, 2.0, 10.0), 0.25)
            .build()
            .unwrap();
        let mut objects = world.object_snapshots();
        objects[2].carried_by = Some(OrganismId(11));
        let frame =
            LiveBrainPresentationFrame::try_new(Vec::new(), world.tick(), objects.clone()).unwrap();
        let mut app = App::new();
        app.insert_resource(LiveBrainPresentationFrameResource {
            previous: frame.clone(),
            current: frame,
        })
        .insert_resource(creature_grounding::RenderedTerrainSurface::from_meshes(
            std::iter::empty(),
            1.0,
        ))
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<StandardMaterial>::default())
        .insert_resource(DisplayedCarrierPositions(BTreeMap::from([
            (11, Vec3::new(3.0, 99.0, 5.0)),
            (22, Vec3::new(90.0, 88.0, 90.0)),
        ])));
        if highlands {
            app.insert_resource(creature_grounding::SelectedTerrain(
                alife_world::island_terrain(),
            ));
        }
        let roots = std::array::from_fn(|index| {
            let object = &objects[index];
            app.world_mut()
                .spawn((
                    ProductionCreatureAssemblyRoot {
                        stable_id: object.id,
                        organism_id: object.organism_id.unwrap(),
                        display_only: true,
                    },
                    Transform::from_translation(Vec3::new(
                        object.position.x,
                        object.position.y,
                        object.position.z,
                    )),
                ))
                .id()
        });
        // Use the production registration and its real ECS systems, with CPU-only
        // displayed root poses supplied in the animation set instead of a GLB.
        configure_production_voxel_presentation_schedule(&mut app);
        app.add_systems(
            Update,
            sync_care_objects.in_set(ProductionVoxelPresentationSet::AuthoritativeProjection),
        )
        .add_systems(
            Update,
            present_carrier_positions.in_set(ProductionVoxelPresentationSet::ProceduralAnimation),
        );
        (app, objects, roots)
    }

    fn care_entity(app: &mut App, id: WorldEntityId) -> Option<Entity> {
        let mut query = app.world_mut().query::<(Entity, &LiveCareObject)>();
        query
            .iter(app.world())
            .find_map(|(entity, marker)| (marker.0 == id).then_some(entity))
    }

    fn care_position(app: &mut App, id: WorldEntityId) -> Vec3 {
        let entity = care_entity(app, id).unwrap();
        app.world().get::<Transform>(entity).unwrap().translation
    }

    fn replace_objects(app: &mut App, objects: &[alife_world::WorldObject]) {
        let tick = app
            .world()
            .resource::<LiveBrainPresentationFrameResource>()
            .current
            .authoritative_world_tick;
        app.world_mut()
            .resource_mut::<LiveBrainPresentationFrameResource>()
            .current =
            LiveBrainPresentationFrame::try_new(Vec::new(), tick, objects.to_vec()).unwrap();
    }

    fn alignment_objects() -> Vec<alife_world::WorldObject> {
        let world = alife_world::HeadlessScenarioBuilder::new(7)
            .agent("carrier", OrganismId(11), Vec3f::new(4.0, 2.0, 6.0))
            .agent("other", OrganismId(22), Vec3f::new(-8.0, 2.0, -9.0))
            .agent("duplicate", OrganismId(33), Vec3f::new(7.0, 2.0, 8.0))
            .food("held", Vec3f::new(4.5, 2.0, 6.0), 0.25)
            .food("other-held", Vec3f::new(-7.5, 2.0, -9.0), 0.25)
            .food("loose", Vec3f::new(10.0, 2.0, 10.0), 0.25)
            .food("missing-carrier", Vec3f::new(12.0, 2.0, 13.0), 0.25)
            .build()
            .unwrap();
        let mut objects = world.object_snapshots();
        objects[2].organism_id = Some(OrganismId(11));
        objects[3].carried_by = Some(OrganismId(11));
        objects[4].carried_by = Some(OrganismId(22));
        objects[6].carried_by = Some(OrganismId(99));
        objects
    }

    #[derive(Component)]
    struct AlternateCarrierRoot;

    fn alignment_only_fixture(
        frozen: bool,
        objects: &[alife_world::WorldObject],
        reverse_care_order: bool,
    ) -> (App, Vec<Entity>) {
        let frame =
            LiveBrainPresentationFrame::try_new(Vec::new(), Tick(17), objects.to_vec()).unwrap();
        let mut app = App::new();
        app.insert_resource(LiveBrainPresentationFrameResource {
            previous: frame.clone(),
            current: frame,
        })
        .init_resource::<CarrierOffsetScanCounts>()
        .init_resource::<BevyEntityMap>();
        if frozen {
            app.add_systems(Update, align_carried_care_objects_frozen_baseline);
        } else {
            app.add_systems(Update, align_carried_care_objects);
        }
        let specifications = [
            (objects[0].id, 11, Vec3::new(3.0, 99.0, 5.0)),
            (objects[1].id, 22, Vec3::new(90.0, 88.0, 90.0)),
            // Same stable ID, then a different stable ID for the same organism.
            (objects[0].id, 11, Vec3::new(30.0, 77.0, 31.0)),
            (objects[2].id, 11, Vec3::new(40.0, 66.0, 41.0)),
            (objects[1].id, 11, Vec3::new(50.0, 55.0, 51.0)),
            (WorldEntityId(999), 11, Vec3::new(60.0, 44.0, 61.0)),
            (objects[3].id, 11, Vec3::new(70.0, 33.0, 71.0)),
        ];
        let roots = specifications
            .into_iter()
            .enumerate()
            .map(|(index, (stable_id, organism, translation))| {
                let mut entity = app.world_mut().spawn((
                    ProductionCreatureAssemblyRoot {
                        stable_id,
                        organism_id: OrganismId(organism),
                        display_only: true,
                    },
                    Transform::from_translation(translation),
                ));
                // Duplicate winners must follow the frozen query across archetypes.
                if index == 2 {
                    entity.insert(Name::new("duplicate-stable-id"));
                } else if index == 3 {
                    entity.insert(AlternateCarrierRoot);
                }
                entity.id()
            })
            .collect();
        let mut care_objects: Vec<_> = objects
            .iter()
            .filter(|object| object.kind != WorldObjectKind::Agent)
            .collect();
        if reverse_care_order {
            care_objects.reverse();
        }
        for object in care_objects {
            app.world_mut().spawn((
                LiveCareObject(object.id),
                Transform {
                    translation: Vec3::new(-100.0, 2.30, -200.0),
                    rotation: Quat::from_rotation_y(0.7),
                    scale: Vec3::new(1.0, 2.0, 3.0),
                },
            ));
        }
        // An absent snapshot object must remain untouched and cannot trigger a scan.
        app.world_mut().spawn((
            LiveCareObject(WorldEntityId(998)),
            Transform::from_xyz(8.0, 9.0, 10.0),
        ));
        (app, roots)
    }

    fn assert_alignment_parity(apps: &mut [App; 2], builds: usize, visits: usize) {
        *apps[1]
            .world_mut()
            .resource_mut::<CarrierOffsetScanCounts>() = CarrierOffsetScanCounts::default();
        let mut projected = Vec::new();
        for app in &mut *apps {
            let frame = app
                .world()
                .get_resource::<LiveBrainPresentationFrameResource>()
                .cloned();
            app.update();
            let mut query = app.world_mut().query::<(&LiveCareObject, &Transform)>();
            let mut transforms: Vec<_> = query
                .iter(app.world())
                .map(|(marker, transform)| (marker.0.raw(), *transform))
                .collect();
            transforms.sort_by_key(|(id, _)| *id);
            projected.push(transforms);
            assert_eq!(
                app.world()
                    .get_resource::<LiveBrainPresentationFrameResource>(),
                frame.as_ref(),
            );
        }
        assert_eq!(projected[0], projected[1]);
        // The candidate is the second app; counters are invocation-local observations.
        let counts = apps[1].world().resource::<CarrierOffsetScanCounts>();
        assert_eq!((counts.builds, counts.visits), (builds, visits));
    }

    #[test]
    fn carrier_offsets_skip_root_scans_for_idle_absent_and_loose_care_objects() {
        for case in 0..4 {
            let mut objects = alignment_objects();
            for object in &mut objects {
                object.carried_by = None;
            }
            if case == 1 {
                for index in 0..128 {
                    let mut loose = objects[5].clone();
                    loose.id = WorldEntityId(1000 + index);
                    objects.push(loose);
                }
            }
            let (baseline, baseline_roots) = alignment_only_fixture(true, &objects, false);
            let (candidate, candidate_roots) = alignment_only_fixture(false, &objects, false);
            let mut apps = [baseline, candidate];
            if case == 0 || case == 2 {
                for app in &mut apps {
                    let mut query = app.world_mut().query::<(Entity, &LiveCareObject)>();
                    let entities: Vec<_> = query
                        .iter(app.world())
                        .filter_map(|(entity, marker)| {
                            (case == 0 || marker.0 != WorldEntityId(998)).then_some(entity)
                        })
                        .collect();
                    for entity in entities {
                        app.world_mut().despawn(entity);
                    }
                }
            }
            if case == 3 {
                for (app, roots) in apps.iter_mut().zip([baseline_roots, candidate_roots]) {
                    for root in roots {
                        app.world_mut().despawn(root);
                    }
                }
            }
            for _ in 0..2 {
                assert_alignment_parity(&mut apps, 0, 0);
            }
        }
    }

    #[test]
    fn carrier_offsets_match_frozen_duplicate_winners_with_one_scan_for_multiple_carriers() {
        let objects = alignment_objects();
        for reverse in [false, true] {
            let (baseline, baseline_roots) = alignment_only_fixture(true, &objects, reverse);
            let (candidate, candidate_roots) = alignment_only_fixture(false, &objects, reverse);
            let mut apps = [baseline, candidate];
            assert_alignment_parity(&mut apps, 1, 7);
            let winner = care_position(&mut apps[0], objects[3].id);
            // Bind a different valid root than the actual frozen winner. Other valid
            // roots remain deliberately unbound; binding order must not choose the pose.
            let bound_index = [0, 2, 3]
                .into_iter()
                .find(|&index| {
                    let root = apps[0]
                        .world()
                        .get::<ProductionCreatureAssemblyRoot>(baseline_roots[index])
                        .unwrap();
                    let position = apps[0]
                        .world()
                        .get::<Transform>(baseline_roots[index])
                        .unwrap()
                        .translation;
                    let carrier = objects
                        .iter()
                        .find(|object| object.id == root.stable_id)
                        .unwrap();
                    let projected = world_position_for_render(objects[3].position, false)
                        + position
                        - world_position_for_render(carrier.position, false);
                    (projected.x, projected.z) != (winner.x, winner.z)
                })
                .unwrap();
            for (app, roots) in apps.iter_mut().zip([baseline_roots, candidate_roots]) {
                let stable_id = app
                    .world()
                    .get::<ProductionCreatureAssemblyRoot>(roots[bound_index])
                    .unwrap()
                    .stable_id;
                let mut map = app.world_mut().resource_mut::<BevyEntityMap>();
                map.bind(roots[0], stable_id).unwrap();
                map.bind(roots[bound_index], stable_id).unwrap();
            }
            assert_alignment_parity(&mut apps, 1, 7);
            assert_eq!(care_position(&mut apps[1], objects[3].id), winner);
            assert_eq!(
                care_position(&mut apps[1], objects[4].id),
                Vec3::new(90.5, 2.30, 90.0)
            );
            assert_eq!(
                care_position(&mut apps[1], objects[6].id),
                Vec3::new(12.0, 2.30, 13.0)
            );
        }
    }

    #[test]
    fn carrier_offsets_rebuild_for_replaced_stale_and_restored_roots_without_frame_changes() {
        let mut objects = alignment_objects();
        let (baseline, baseline_roots) = alignment_only_fixture(true, &objects, false);
        let (candidate, candidate_roots) = alignment_only_fixture(false, &objects, false);
        let mut apps = [baseline, candidate];
        assert_alignment_parity(&mut apps, 1, 7);
        for (app, roots) in apps.iter_mut().zip([&baseline_roots, &candidate_roots]) {
            // Remove all valid roots for organism 11, leaving missing/mismatched roots.
            for index in [0, 2, 3] {
                app.world_mut().despawn(roots[index]);
            }
            app.world_mut()
                .resource_mut::<BevyEntityMap>()
                .bind(roots[0], objects[0].id)
                .unwrap();
        }
        assert_alignment_parity(&mut apps, 1, 4);
        assert_eq!(
            care_position(&mut apps[1], objects[3].id),
            Vec3::new(4.5, 2.30, 6.0)
        );
        for app in &mut apps {
            app.world_mut().spawn((
                ProductionCreatureAssemblyRoot {
                    stable_id: objects[0].id,
                    organism_id: OrganismId(11),
                    display_only: true,
                },
                Transform::from_xyz(123.0, 77.0, 125.0),
            ));
        }
        assert_alignment_parity(&mut apps, 1, 5);
        assert_eq!(
            care_position(&mut apps[1], objects[3].id),
            Vec3::new(123.5, 2.30, 125.0)
        );
        let frames: Vec<_> = apps
            .iter_mut()
            .map(|app| {
                app.world_mut()
                    .remove_resource::<LiveBrainPresentationFrameResource>()
                    .unwrap()
            })
            .collect();
        assert_alignment_parity(&mut apps, 0, 0);
        for (app, frame) in apps.iter_mut().zip(frames) {
            app.insert_resource(frame);
        }
        assert_alignment_parity(&mut apps, 1, 5);
        objects.remove(0);
        for app in &mut apps {
            replace_objects(app, &objects);
        }
        assert_alignment_parity(&mut apps, 1, 5);
        assert_eq!(
            care_position(&mut apps[1], objects[2].id),
            Vec3::new(4.5, 2.30, 6.0)
        );
        for app in &mut apps {
            let mut query = app
                .world_mut()
                .query_filtered::<Entity, With<ProductionCreatureAssemblyRoot>>();
            let roots: Vec<_> = query.iter(app.world()).collect();
            for root in roots {
                app.world_mut().despawn(root);
            }
        }
        assert_alignment_parity(&mut apps, 1, 0);
    }

    #[test]
    fn carried_care_objects_follow_exact_displayed_carrier_between_ticks_without_drift() {
        for highlands in [false, true] {
            let (mut app, objects, _) = carried_fixture(highlands);
            let authoritative = app
                .world()
                .resource::<LiveBrainPresentationFrameResource>()
                .clone();
            for x in [3.0, 3.25, 3.5, 3.75, 3.75, 4.0, 4.0] {
                app.world_mut()
                    .resource_mut::<DisplayedCarrierPositions>()
                    .0
                    .insert(11, Vec3::new(x, 99.0, x + 2.0));
                app.update();
                let held = care_position(&mut app, objects[2].id);
                assert!((held.x - (x + 0.5)).abs() < 1e-6);
                assert!((held.z - (x + 2.0)).abs() < 1e-6);
                assert!((held.y - if highlands { 2.30 } else { 2.74 }).abs() < 1e-5);
                let loose = care_position(&mut app, objects[3].id);
                assert_eq!((loose.x, loose.z), (10.0, 10.0));
                assert_eq!(
                    app.world().resource::<LiveBrainPresentationFrameResource>(),
                    &authoritative,
                );
            }
        }
    }

    #[test]
    fn carried_care_objects_preserve_world_grip_on_turn_release_and_regrab() {
        let (mut app, mut objects, roots) = carried_fixture(true);
        for yaw in [0.0, std::f32::consts::FRAC_PI_2, std::f32::consts::PI] {
            objects[0].body_yaw = yaw;
            objects[2].position = Vec3f::new(4.0 + 0.5 * yaw.cos(), 2.0, 6.0 + 0.5 * yaw.sin());
            app.world_mut()
                .get_mut::<Transform>(roots[0])
                .unwrap()
                .rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2 - yaw);
            replace_objects(&mut app, &objects);
            app.update();
            let held = care_position(&mut app, objects[2].id);
            assert!((held.x - (3.0 + 0.5 * yaw.cos())).abs() < 1e-6);
            assert!((held.z - (5.0 + 0.5 * yaw.sin())).abs() < 1e-6);
        }
        objects[2].carried_by = None;
        objects[2].position = Vec3f::new(7.0, 2.0, 8.0);
        replace_objects(&mut app, &objects);
        app.update();
        assert_eq!(
            care_position(&mut app, objects[2].id),
            Vec3::new(7.0, 2.30, 8.0)
        );
        objects[2].carried_by = Some(OrganismId(22));
        objects[2].position = Vec3f::new(-7.5, 2.0, -9.0);
        replace_objects(&mut app, &objects);
        app.update();
        assert_eq!(
            care_position(&mut app, objects[2].id),
            Vec3::new(90.5, 2.30, 90.0)
        );
    }

    #[test]
    fn carried_care_objects_fall_back_when_carrier_is_despawned_missing_or_mismatched() {
        let (mut app, mut objects, roots) = carried_fixture(true);
        app.update();
        assert_eq!(
            care_position(&mut app, objects[2].id),
            Vec3::new(3.5, 2.30, 5.0)
        );
        app.world_mut().despawn(roots[0]);
        app.update();
        assert_eq!(
            care_position(&mut app, objects[2].id),
            Vec3::new(4.5, 2.30, 6.0)
        );
        // A stale root with the owner's organism ID and another stable ID must
        // not attach the food to that unrelated entity.
        app.world_mut()
            .get_mut::<ProductionCreatureAssemblyRoot>(roots[1])
            .unwrap()
            .organism_id = OrganismId(11);
        app.update();
        assert_eq!(
            care_position(&mut app, objects[2].id),
            Vec3::new(4.5, 2.30, 6.0)
        );
        objects.remove(0);
        replace_objects(&mut app, &objects);
        app.update();
        assert_eq!(
            care_position(&mut app, objects[1].id),
            Vec3::new(4.5, 2.30, 6.0)
        );
    }

    #[test]
    fn carried_care_objects_keep_consumption_regrowth_and_removal_lifecycle() {
        let (mut app, mut objects, _) = carried_fixture(true);
        app.update();
        let old_entity = care_entity(&mut app, objects[2].id).unwrap();
        objects[2].consumed = true;
        replace_objects(&mut app, &objects);
        app.update();
        assert!(care_entity(&mut app, objects[2].id).is_none());
        assert!(app.world().get_entity(old_entity).is_err());
        objects[2].consumed = false;
        objects[2].carried_by = None;
        objects[2].position = Vec3f::new(12.0, 2.0, 13.0);
        replace_objects(&mut app, &objects);
        app.update();
        assert_ne!(care_entity(&mut app, objects[2].id).unwrap(), old_entity);
        assert_eq!(
            care_position(&mut app, objects[2].id),
            Vec3::new(12.0, 2.30, 13.0)
        );
        let removed_id = objects.remove(2).id;
        replace_objects(&mut app, &objects);
        app.update();
        assert!(care_entity(&mut app, removed_id).is_none());
    }
}
