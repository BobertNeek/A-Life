//! Visible food follows canonical placement, carrying, consumption, and reloads.
use super::*;
use bevy::prelude::{Meshable, Query, Sphere};

#[derive(Component)]
pub(super) struct LiveFood(WorldEntityId);

pub(super) fn sync_food(
    mut commands: Commands,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut assets: Local<
        Option<(
            Handle<Mesh>,
            Handle<StandardMaterial>,
            Handle<StandardMaterial>,
        )>,
    >,
    mut food_entities: Query<(Entity, &LiveFood, &mut Transform)>,
) {
    let Some(frame) = frame else {
        return;
    };
    if !frame.is_changed() {
        return;
    }
    let mut food: BTreeMap<_, _> = frame
        .current
        .objects()
        .filter(|object| object.kind == WorldObjectKind::Food && !object.consumed)
        .map(|object| (object.id.raw(), object.position))
        .collect();
    let translation = |position: Vec3f| {
        let ground = Vec3::new(position.x, 0.0, position.y);
        ground + Vec3::Y * (surface.height(ground).unwrap_or(0.44) + position.z + 0.30)
    };
    for (entity, marker, mut transform) in &mut food_entities {
        if let Some(position) = food.remove(&marker.0.raw()) {
            transform.translation = translation(position);
        } else {
            commands.entity(entity).despawn();
        }
    }
    if food.is_empty() {
        return;
    }
    let (mesh, fruit, leaf) = assets.get_or_insert_with(|| {
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
        )
    });
    for (id, position) in food {
        commands
            .spawn((
                Name::new("Food"),
                LiveFood(WorldEntityId(id)),
                Transform::from_translation(translation(position)),
                Visibility::default(),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(fruit.clone()),
                    Transform::from_scale(Vec3::new(0.34, 0.30, 0.34)),
                ));
                parent.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(leaf.clone()),
                    Transform::from_xyz(0.10, 0.29, 0.0)
                        .with_rotation(Quat::from_rotation_z(0.4))
                        .with_scale(Vec3::new(0.19, 0.025, 0.07)),
                ));
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn food_projection_tracks_placement_and_consumption_without_advancing_time() {
        let world = alife_world::HeadlessScenarioBuilder::new(7)
            .food("apple", Vec3f::new(2.5, -3.5, 0.0), 0.25)
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
        .add_systems(Update, sync_food);
        app.update();
        let mut query = app
            .world_mut()
            .query_filtered::<&Transform, With<LiveFood>>();
        let fruit = query.single(app.world()).unwrap();
        assert_eq!(fruit.translation.x, 2.5);
        assert_eq!(fruit.translation.z, -3.5);
        let mut objects = world.object_snapshots();
        objects[0].consumed = true;
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
