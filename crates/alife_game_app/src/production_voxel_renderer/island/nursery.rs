//! Fixed, noninteractive art for the nursery on the approved island heightfield.
use super::*;

pub(super) fn plant(world: &mut World, surface: &alife_world::TerrainSurface) {
    let server = world.resource::<AssetServer>();
    let [silverbell, waystone] = ["care/silverbell.glb", "care/ceramic-waystone.glb"].map(|path| {
        server.load(
            bevy::gltf::GltfAssetLabel::Primitive {
                mesh: 0,
                primitive: 0,
            }
            .from_asset(path),
        )
    });
    let understory =
        server.load(bevy::gltf::GltfAssetLabel::Scene(0).from_asset("care/nursery/understory.glb"));
    plant_margins(world, surface, silverbell, waystone);
    if let Some(height) = surface.height(81.25, 250.) {
        world
            .spawn((
                Name::new("Grounded nursery understory"),
                SceneRoot(understory),
                Transform::from_xyz(81.25, height, 250.),
                Fvr04ProductionRuntimeSceneRoot,
                bevy::picking::Pickable::IGNORE,
            ))
            .observe(ignore_understory_geometry);
    }
}

// Decoration never receives a Hand interaction or darkens the physical ground below.
fn ignore_understory_geometry(
    ready: On<bevy::scene::SceneInstanceReady>,
    children: Query<&Children>,
    mut commands: Commands,
) {
    for entity in children.iter_descendants(ready.entity) {
        commands
            .entity(entity)
            .insert((bevy::picking::Pickable::IGNORE, NotShadowCaster));
    }
}

/// A fixed clearing at the island's nursery, never decoration that follows a creature.
/// Keep the middle open for the Hand. Low groundcover has no interaction or collision.
fn plant_margins(
    world: &mut World,
    surface: &alife_world::TerrainSurface,
    flowers: Handle<Mesh>,
    stones: Handle<Mesh>,
) {
    let material = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            perceptual_roughness: 0.86,
            ..default()
        });
    let mut plant = |mesh: &Handle<Mesh>, x: f32, z: f32, yaw: f32, size: f32, foot: f32| {
        let Some(y) = surface.height(x, z) else {
            return;
        };
        world.spawn((
            Name::new("Nursery groundcover"),
            groundlight::ContactArea::new(foot * size),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(x, y, z)
                .with_rotation(Quat::from_rotation_y(yaw))
                .with_scale(Vec3::splat(size)),
            bevy::picking::Pickable::IGNORE,
            Fvr04ProductionRuntimeSceneRoot,
        ));
    };
    // Three irregular drifts, not a fence: each opens onto the original island.
    for (cx, cz, length, turn) in [
        (-2.8, -1.8, 5, 0.22),
        (3.4, 1.8, 6, 2.31),
        (-0.9, 4.3, 4, 4.45),
    ] {
        for j in 0..length {
            let t = j as f32;
            let a = turn + t * 2.39996;
            let reach = 0.51 * t.sqrt();
            plant(
                &flowers,
                83.0 + cx + reach * a.cos(),
                250.0 + cz + reach * a.sin(),
                a,
                0.75 + 0.20 * (t + turn).cos(),
                0.38,
            );
        }
    }
    for (x, z, angle, size) in [
        (-3.3, 0.2, 0.3, 0.94),
        (-2.8, 0.8, 1.9, 0.66),
        (-2.4, 1.5, 2.3, 0.9),
        (-1.9, 2.3, 0.5, 1.07),
        (-1.1, 3.1, 2.0, 0.83),
        (1.6, -2.3, 1.7, 1.08),
        (2.4, -2.9, 0.1, 0.83),
    ] {
        plant(&stones, 83.0 + x, 250.0 + z, angle, size, 0.47);
    }
}
