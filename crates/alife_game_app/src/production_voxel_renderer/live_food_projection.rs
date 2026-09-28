//! Visible care objects follow canonical placement, carrying, consumption, and reloads.
use super::*;
use bevy::prelude::{Meshable, Query, Sphere};

#[derive(Component)]
pub(super) struct LiveCareObject(WorldEntityId);

pub(super) fn sync_care_objects(
    mut commands: Commands,
    highlands: Option<Res<creature_grounding::SelectedTerrain>>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: Local<
        Option<(
            Handle<Mesh>,
            Handle<StandardMaterial>,
            Handle<StandardMaterial>,
            Handle<StandardMaterial>,
        )>,
    >,
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
                && (object.kind == WorldObjectKind::Food
                    || (object.kind == WorldObjectKind::Token
                        && object.label == "player-plaything"))
        })
        .map(|object| (object.id.raw(), (object.position, object.kind)))
        .collect();
    let translation = |position: Vec3f| {
        let rendered = world_position_for_render(position, highlands.is_some());
        let ground = Vec3::new(rendered.x, 0.0, rendered.z);
        let height = if highlands.is_some() {
            surface.height(ground).unwrap_or(rendered.y)
        } else {
            surface.height(ground).unwrap_or(0.44) + rendered.y
        };
        ground + Vec3::Y * (height + 0.30)
    };
    for (entity, marker, mut transform) in &mut object_entities {
        if let Some((position, _)) = objects.remove(&marker.0.raw()) {
            transform.translation = translation(position);
        } else {
            commands.entity(entity).despawn();
        }
    }
    if objects.is_empty() {
        return;
    }
    let (mesh, fruit, leaf, toy) = assets.get_or_insert_with(|| {
        (
            meshes.add(Sphere::new(1.0).mesh().uv(16, 12)),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.90, 0.16, 0.035),
                perceptual_roughness: 0.5,
                ..default()
            }),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.18, 0.42, 0.035),
                ..default()
            }),
            materials.add(StandardMaterial {
                base_color: Color::srgb(0.08, 0.38, 0.95),
                perceptual_roughness: 0.6,
                ..default()
            }),
        )
    });
    for (id, (position, kind)) in objects {
        let is_food = kind == WorldObjectKind::Food;
        commands
            .spawn((
                Name::new(if is_food { "Food" } else { "Plaything" }),
                LiveCareObject(WorldEntityId(id)),
                Fvr04ProductionRuntimeSceneRoot,
                Transform::from_translation(translation(position)),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(if is_food { fruit.clone() } else { toy.clone() }),
                    Transform::from_scale(Vec3::new(0.34, 0.30, 0.34)),
                ));
                if is_food {
                    parent.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(leaf.clone()),
                        Transform::from_xyz(0.10, 0.29, 0.0)
                            .with_rotation(Quat::from_rotation_z(0.4))
                            .with_scale(Vec3::new(0.19, 0.025, 0.07)),
                    ));
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn care_feedback_projection_tracks_placement_and_consumption_without_advancing_time() {
        for highlands in [false, true] {
            let position = if highlands {
                Vec3f::new(2.5, 4.0, -3.5)
            } else {
                Vec3f::new(2.5, -3.5, 0.0)
            };
            let world = alife_world::HeadlessScenarioBuilder::new(7)
                .food("apple", position, 0.25)
                .token("player-plaything", position, 1)
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
                    alife_world::WorldTerrain::highlands(),
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
            assert!((fruit.translation.y - if highlands { 4.30 } else { 0.74 }).abs() < 1e-5);
            let mut objects = world.object_snapshots();
            let toy = objects
                .iter_mut()
                .find(|object| object.kind == WorldObjectKind::Token)
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
}
