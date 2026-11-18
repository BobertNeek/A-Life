//! A soft foot of shade at an actual displayed object's contact with the soil.
//! One light-weight mesh and material serve the entire habitat. The patch is a
//! child for lifetime only: a rolling rattle must never turn its shade upright.
use super::*;
use bevy::{
    light::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    transform::TransformSystems,
};

#[derive(Component)]
pub(super) struct ContactArea(f32);
impl ContactArea {
    /// World-space outer radius of this model's soft contact edge.
    pub(super) fn new(radius: f32) -> Self {
        Self(radius)
    }
}

#[derive(Component, Default)]
struct Pool {
    last: Option<(Transform, bool)>,
}

pub(super) fn install(app: &mut App) {
    if !app.world().contains_resource::<AssetServer>() {
        return;
    }
    app.add_systems(
        Update,
        attach.in_set(ProductionVoxelPresentationSet::RootReaders),
    )
    .add_systems(PostUpdate, seat.before(TransformSystems::Propagate));
}
fn attach(
    mut commands: Commands,
    areas: Query<Entity, Added<ContactArea>>,
    meshes: Option<ResMut<Assets<Mesh>>>,
    materials: Option<ResMut<Assets<StandardMaterial>>>,
    mut print: Local<Option<(Handle<Mesh>, Handle<StandardMaterial>)>>,
) {
    if areas.is_empty() {
        return;
    }
    let (Some(mut meshes), Some(mut materials)) = (meshes, materials) else {
        return;
    };
    let (mesh, material) = print.get_or_insert_with(|| {
        (
            meshes.add(penumbra()),
            materials.add(StandardMaterial {
                alpha_mode: AlphaMode::Blend,
                depth_bias: 2.0,
                unlit: true,
                ..default()
            }),
        )
    });
    for source in &areas {
        commands.spawn((
            Name::new("Ground contact"),
            Pool::default(),
            ChildOf(source),
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material.clone()),
            Visibility::Hidden,
            NotShadowCaster,
            NotShadowReceiver,
            bevy::picking::Pickable::IGNORE,
        ));
    }
}

/// A tangent to the rendered soil, never a horizontal sticker through a slope.
/// Missing central ground, an object well above it, or a real grip has no print.
fn foot_pose(
    source: Transform,
    radius: f32,
    surface: &creature_grounding::RenderedTerrainSurface,
    lifted: bool,
) -> Option<Transform> {
    if lifted || !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let center = source.translation;
    let h = surface.height(center)?;
    if !(-0.025..=radius * 1.7 + 0.10).contains(&(center.y - h)) {
        return None;
    }
    let slope = |axis: Vec3| {
        let step = radius * 0.68;
        let low = surface.height(center - axis * step).unwrap_or(h);
        let high = surface.height(center + axis * step).unwrap_or(h);
        (high - low) / (step * 2.0)
    };
    let normal = Vec3::new(-slope(Vec3::X), 1.0, -slope(Vec3::Z)).normalize();
    let world = Transform::from_xyz(center.x, h + 0.009, center.z)
        .with_rotation(Quat::from_rotation_arc(Vec3::Y, normal))
        .with_scale(Vec3::splat(radius));
    Some(Transform::from_matrix(
        source.to_matrix().inverse() * world.to_matrix(),
    ))
}

fn seat(
    surface: Option<Res<creature_grounding::RenderedTerrainSurface>>,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    sources: Query<
        (
            &ContactArea,
            &Transform,
            Option<&live_food_projection::LiveCareObject>,
        ),
        Without<Pool>,
    >,
    mut pools: Query<(&ChildOf, &mut Pool, &mut Transform, &mut Visibility)>,
) {
    let Some(surface) = surface else {
        return;
    };
    for (child, mut pool, mut local, mut visible) in &mut pools {
        let Ok((area, source, care)) = sources.get(child.parent()) else {
            continue;
        };
        let lifted = care
            .and_then(|id| frame.as_ref()?.current.object(id.0))
            .is_some_and(|body| body.carried_by.is_some());
        let state = (*source, lifted);
        if !surface.is_changed() && pool.last == Some(state) {
            continue;
        }
        pool.last = Some(state);
        if let Some(pose) = foot_pose(*source, area.0, &surface, lifted) {
            local.set_if_neq(pose);
            visible.set_if_neq(Visibility::Inherited);
        } else {
            visible.set_if_neq(Visibility::Hidden);
        }
    }
}

fn penumbra() -> Mesh {
    let mut points = vec![[0., 0., 0.]];
    let pigment = |alpha| [0.025, 0.032, 0.027, alpha];
    let mut colors = vec![pigment(0.43)];
    let mut triangles = Vec::new();
    const STEPS: u32 = 20;
    for (band, (radius, alpha)) in [(0.32, 0.39), (0.57, 0.27), (0.79, 0.10), (1., 0.)]
        .into_iter()
        .enumerate()
    {
        let first = 1 + band as u32 * STEPS;
        for i in 0..STEPS {
            let (z, x) = (std::f32::consts::TAU * i as f32 / STEPS as f32).sin_cos();
            points.push([radius * x, 0., radius * z]);
            colors.push(pigment(alpha));
            let a = first + i;
            let b = first + (i + 1) % STEPS;
            if band == 0 {
                triangles.extend([0, b, a]);
            } else {
                triangles.extend([a - STEPS, b, a, a - STEPS, b - STEPS, b]);
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 1., 0.]; points.len()]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, points);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(triangles));
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hill() -> creature_grounding::RenderedTerrainSurface {
        let terrain = alife_world::WorldTerrain::new(
            alife_world::TerrainData {
                width: 2,
                depth: 2,
                origin_x: 0.0,
                origin_z: 0.0,
                spacing: 8.0,
                heights: vec![1., 5., 3., 7.],
                obstacles: vec![],
                water_level: None,
            },
            Default::default(),
        )
        .unwrap();
        let mut surface = creature_grounding::RenderedTerrainSurface::from_meshes([], 1.);
        surface.set_surface(Some(terrain.surface().clone()));
        surface
    }

    #[test]
    fn a_rolling_or_scaled_object_cannot_tilt_its_print_off_the_receiver() {
        let surface = hill();
        for roll in [0.0, 1.2, 2.7] {
            let root = Transform::from_xyz(4., 4.2, 4.)
                .with_rotation(Quat::from_rotation_x(roll))
                .with_scale(Vec3::splat(0.8));
            let foot = foot_pose(root, 0.45, &surface, false).unwrap();
            let world = root.to_matrix() * foot.to_matrix();
            for (x, z) in [(0., 0.), (0.95, 0.), (0., -0.95)] {
                let point = world.transform_point3(Vec3::new(x, 0., z));
                let separation = point.y - surface.height(point).unwrap();
                assert!((separation - 0.009).abs() < 1e-5);
            }
        }
    }
    #[test]
    fn prints_share_assets_follow_moving_sources_and_leave_with_them() {
        let mut app = App::new();
        app.insert_resource(hill())
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .add_systems(Update, attach)
            .add_systems(PostUpdate, seat);
        let roots = [3.9, 4.1].map(|x| {
            app.world_mut()
                .spawn((ContactArea::new(0.35), Transform::from_xyz(x, 4.20, 4.)))
                .id()
        });
        app.update();
        let prints = roots.map(|id| app.world().get::<Children>(id).unwrap()[0]);
        assert_eq!(
            app.world().get::<Mesh3d>(prints[0]).unwrap().0,
            app.world().get::<Mesh3d>(prints[1]).unwrap().0
        );
        assert_eq!(
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(prints[0])
                .unwrap()
                .0,
            app.world()
                .get::<MeshMaterial3d<StandardMaterial>>(prints[1])
                .unwrap()
                .0
        );
        assert_eq!(
            app.world().get::<Visibility>(prints[0]),
            Some(&Visibility::Inherited)
        );
        app.world_mut()
            .get_mut::<Transform>(roots[0])
            .unwrap()
            .translation
            .y += 2.;
        app.update();
        assert_eq!(
            app.world().get::<Visibility>(prints[0]),
            Some(&Visibility::Hidden)
        );
        assert_eq!(
            app.world().get::<Visibility>(prints[1]),
            Some(&Visibility::Inherited)
        );
        app.world_mut().entity_mut(roots[0]).despawn();
        assert!(app.world().get_entity(prints[0]).is_err());
        assert!(app.world().get_entity(prints[1]).is_ok());
    }

    #[test]
    fn a_grip_a_gap_or_missing_soil_never_makes_a_contact_mark() {
        let surface = hill();
        let at = |x, y| Transform::from_xyz(x, y, 4.);
        assert!(foot_pose(at(4., 4.2), 0.35, &surface, true).is_none());
        assert!(foot_pose(at(4., 6.), 0.35, &surface, false).is_none());
        assert!(foot_pose(at(4., 2.), 0.35, &surface, false).is_none());
        assert!(foot_pose(at(-2., 1.), 0.35, &surface, false).is_none());
    }
}
