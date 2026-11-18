//! Put the world's actual blocking and hazardous bodies on the island.
//! No decorative copies, guessed placement, or second collision system.
use super::*;
use bevy::{
    gltf::GltfAssetLabel,
    prelude::{AssetServer, Query},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Waykeeper,
    Emberstone,
}
impl Form {
    fn of(kind: WorldObjectKind) -> Option<Self> {
        match kind {
            WorldObjectKind::Obstacle => Some(Self::Waykeeper),
            WorldObjectKind::Hazard => Some(Self::Emberstone),
            _ => None,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Waykeeper => "Grounded waykeeper",
            Self::Emberstone => "Grounded emberstone",
        }
    }
}

#[derive(Resource)]
struct Models([Handle<Mesh>; 2]);
impl Models {
    fn load(server: &AssetServer) -> Self {
        Self(["waykeeper", "emberstone"].map(|stem| {
            server.load(
                GltfAssetLabel::Primitive {
                    mesh: 0,
                    primitive: 0,
                }
                .from_asset(format!("care/landmarks/{stem}.glb")),
            )
        }))
    }
    fn get(&self, form: Form) -> Handle<Mesh> {
        self.0[match form {
            Form::Waykeeper => 0,
            Form::Emberstone => 1,
        }]
        .clone()
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Finish {
    tint: [f32; 3],
    roughness: f32,
    opacity: f32,
}
impl Finish {
    fn apply(self, material: &mut StandardMaterial) {
        let [r, g, b] = self.tint;
        material.base_color = Color::srgba(r, g, b, self.opacity);
        material.alpha_mode = if self.opacity < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        };
        material.perceptual_roughness = self.roughness;
    }
}

#[derive(Component)]
struct Landmark {
    id: WorldEntityId,
    form: Form,
    finish: Finish,
}

struct Body {
    form: Form,
    pose: Transform,
    finish: Finish,
}
fn body(
    object: &alife_world::WorldObject,
    terrain_bound: bool,
    surface: &creature_grounding::RenderedTerrainSurface,
) -> Option<Body> {
    let form = Form::of(object.kind)?;
    if object.consumed
        || object.radius <= 0.0
        || !object.radius.is_finite()
        || object.optical_opacity <= 0.0
        || !object.optical_opacity.is_finite()
    {
        return None;
    }
    let physical = object.grounded_physical;
    let origin =
        live_food_projection::grounded_object_origin(object.position, terrain_bound, surface);
    let r = object.radius;
    Some(Body {
        form,
        // The authored footprint has unit radius. The physical shape changes
        // height, never the horizontal boundary that stops a rolling ball.
        pose: Transform::from_translation(origin)
            .with_rotation(Quat::from_rotation_y(object.body_yaw))
            .with_scale(Vec3::new(r, r * (0.80 + 0.36 * physical.shape[1]), r)),
        finish: Finish {
            tint: physical.color.map(|c| 0.62 + 0.38 * c.clamp(0.0, 1.0)),
            roughness: 0.50 + 0.42 * physical.material[0].clamp(0.0, 1.0),
            opacity: object.optical_opacity.min(1.0),
        },
    })
}

pub(super) fn install(app: &mut App) {
    let Some(server) = app.world().get_resource::<AssetServer>() else {
        return;
    };
    let models = Models::load(server);
    app.insert_resource(models).add_systems(
        Update,
        project.in_set(ProductionVoxelPresentationSet::AuthoritativeProjection),
    );
}

fn project(
    mut commands: Commands,
    frame: Option<Res<LiveBrainPresentationFrameResource>>,
    surface: Res<creature_grounding::RenderedTerrainSurface>,
    terrain: Option<Res<creature_grounding::SelectedTerrain>>,
    models: Res<Models>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut roots: Query<(
        Entity,
        &mut Landmark,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    let Some(frame) = frame else {
        return;
    };
    if !frame.is_changed() {
        return;
    }
    let mut bodies: BTreeMap<_, _> = frame
        .current
        .objects()
        .filter_map(|object| {
            body(object, terrain.is_some(), &surface).map(|b| (object.id.raw(), b))
        })
        .collect();
    if bodies.is_empty() && roots.is_empty() {
        return;
    }
    for (entity, mut mark, mut transform, material) in &mut roots {
        let std::collections::btree_map::Entry::Occupied(entry) = bodies.entry(mark.id.raw())
        else {
            commands.entity(entity).despawn();
            continue;
        };
        if entry.get().form != mark.form {
            commands.entity(entity).despawn();
            continue; // Its replacement is still in the map for creation below.
        }
        let b = entry.remove();
        if mark.finish != b.finish {
            if let Some(mat) = materials.get_mut(&material.0) {
                b.finish.apply(mat);
            }
            mark.finish = b.finish;
        }
        transform.set_if_neq(b.pose);
    }
    for (id, b) in bodies {
        let mut material = StandardMaterial::default();
        b.finish.apply(&mut material);
        commands.spawn((
            Name::new(b.form.label()),
            Landmark {
                id: WorldEntityId(id),
                form: b.form,
                finish: b.finish,
            },
            Mesh3d(models.get(b.form)),
            MeshMaterial3d(materials.add(material)),
            b.pose,
            Visibility::default(),
            bevy::picking::Pickable::IGNORE,
            Fvr04ProductionRuntimeSceneRoot,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn landmarks_follow_live_footprints_not_decorative_tile_coordinates() {
        let world = alife_world::HeadlessScenarioBuilder::new(87)
            .obstacle("rock", Vec3f::new(4.625, 2.75, 9.125), 1.3)
            .hazard("vent", Vec3f::new(7., 3., 8.), 0.2)
            .food("gift", Vec3f::ZERO, 0.2)
            .build()
            .unwrap();
        let floor =
            creature_grounding::RenderedTerrainSurface::from_meshes(std::iter::empty(), 1.0);
        let objects = world.object_snapshots();
        assert_eq!(
            objects.iter().filter_map(|o| body(o, true, &floor)).count(),
            2
        );
        for mut o in objects.into_iter().filter(|o| Form::of(o.kind).is_some()) {
            let b = body(&o, true, &floor).unwrap();
            assert_eq!(
                b.pose.translation,
                Vec3::new(o.position.x, o.position.y, o.position.z)
            );
            assert_eq!((b.pose.scale.x, b.pose.scale.z), (o.radius, o.radius));
            o.optical_opacity = 0.33;
            let mut material = StandardMaterial::default();
            body(&o, true, &floor).unwrap().finish.apply(&mut material);
            assert_eq!(material.alpha_mode, AlphaMode::Blend);
            o.optical_opacity = 0.0;
            assert!(body(&o, true, &floor).is_none());
            o.optical_opacity = 1.0;
            o.consumed = true;
            assert!(body(&o, true, &floor).is_none());
        }
    }
    #[test]
    fn moving_recoloring_consuming_and_replacing_world_rows_leave_no_ghosts() {
        let source = alife_world::HeadlessScenarioBuilder::new(89)
            .obstacle("rock", Vec3f::new(3., 1., 9.), 1.2)
            .hazard("vent", Vec3f::ZERO, 0.2)
            .build()
            .unwrap();
        let mut rows = source.object_snapshots();
        let rock = rows
            .iter()
            .position(|o| o.kind == WorldObjectKind::Obstacle)
            .unwrap();
        let vent = rows
            .iter()
            .position(|o| o.kind == WorldObjectKind::Hazard)
            .unwrap();
        let frame = |objects: Vec<_>| {
            LiveBrainPresentationFrameResource::from_current_frame(
                LiveBrainPresentationFrame::try_new(Vec::new(), alife_core::Tick(8), objects)
                    .unwrap(),
            )
        };
        let mut app = App::new();
        app.insert_resource(Models([Handle::default(), Handle::default()]))
            .insert_resource(Assets::<StandardMaterial>::default())
            .insert_resource(creature_grounding::RenderedTerrainSurface::from_meshes(
                std::iter::empty(),
                1.,
            ))
            .insert_resource(frame(rows.clone()))
            .add_systems(Update, project);
        let observed = |app: &mut App| {
            let w = app.world_mut();
            w.query::<(
                Entity,
                &Landmark,
                &Transform,
                &MeshMaterial3d<StandardMaterial>,
            )>()
            .iter(w)
            .map(|(e, b, t, m)| (b.id.raw(), (e, t.translation, m.0.clone())))
            .collect::<BTreeMap<_, _>>()
        };
        app.update();
        let initial = observed(&mut app);
        assert_eq!(initial.len(), 2);
        let id = rows[rock].id.raw();
        let first = initial[&id].clone();
        rows[rock].position.x += 3.;
        rows[rock].grounded_physical.color = [1., 0., 0.];
        rows[vent].consumed = true;
        app.insert_resource(frame(rows.clone()));
        app.update();
        let moved = observed(&mut app);
        assert_eq!(moved.len(), 1);
        assert_eq!(moved[&id].0, first.0);
        assert_eq!(moved[&id].2, first.2);
        assert_eq!(moved[&id].1.x, first.1.x + 3.);
        assert_eq!(
            app.world()
                .resource::<Assets<StandardMaterial>>()
                .get(&first.2)
                .unwrap()
                .base_color,
            Color::srgb(1., 0.62, 0.62)
        );
        rows[rock].kind = WorldObjectKind::Hazard;
        app.insert_resource(frame(rows));
        app.update();
        let changed = observed(&mut app);
        assert_ne!(changed[&id].0, first.0);
        assert_eq!(changed.len(), 1);
        app.insert_resource(frame(Vec::new()));
        app.update();
        assert!(observed(&mut app).is_empty());
    }
}
