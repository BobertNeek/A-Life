//! Shared chunk/prop rendering for approved terrain datasets, including old saves.
use super::*;
use bevy::{
    gltf::{Gltf, GltfMesh},
    light::NotShadowCaster,
    prelude::*,
};

#[derive(Resource)]
pub(super) struct HighlandsActive;
#[derive(Clone, Copy)]
enum Pack {
    Highlands,
    Island,
}
impl Pack {
    fn folder(self) -> &'static str {
        match self {
            Self::Highlands => "landscape/highlands",
            Self::Island => "landscape/island",
        }
    }
    fn prefix(self) -> &'static str {
        match self {
            Self::Highlands => "Highlands",
            Self::Island => "Island",
        }
    }
    fn placements(self) -> Vec<Prop> {
        serde_json::from_str(match self {
            Self::Highlands => include_str!("../../assets/landscape/highlands/props.json"),
            Self::Island => include_str!("../../assets/landscape/island/props.json"),
        })
        .expect("validated approved prop placements")
    }
}
fn pack(terrain: &alife_world::WorldTerrain) -> Option<Pack> {
    if terrain.binding() == alife_world::TerrainBinding::highlands() {
        Some(Pack::Highlands)
    } else if terrain.binding() == alife_world::island_terrain().binding() {
        Some(Pack::Island)
    } else {
        None
    }
}
pub(super) fn has_baked_art(terrain: Option<&alife_world::WorldTerrain>) -> bool {
    terrain.and_then(pack).is_some()
}

#[derive(Resource)]
pub(super) struct HighlandsAssets {
    terrain: Handle<Gltf>,
    props: BTreeMap<String, Handle<Gltf>>,
    pack: Pack,
    surface: std::sync::Arc<alife_world::TerrainSurface>,
    spawned: bool,
    next_update: f64,
}
#[derive(Component)]
pub(super) struct TerrainLod {
    meshes: [Handle<Mesh>; 3],
    center: Vec3,
    current: usize,
}
#[derive(Component)]
pub(super) struct PropRange {
    position: Vec3,
    distance: f32,
    meshes: [Handle<Mesh>; 3],
    current: usize,
    tree: bool,
}
#[derive(Component)]
pub(super) struct TerrainSun {
    enabled: bool,
    cascades: usize,
}
#[derive(serde::Deserialize)]
struct Prop {
    kind: String,
    position: [f32; 3],
    scale: f32,
    yaw: f32,
}

pub(super) fn stop(world: &mut World) {
    world.remove_resource::<HighlandsAssets>();
}
pub(super) fn start(world: &mut World) {
    let Some(selected) = world.get_resource::<creature_grounding::SelectedTerrain>() else {
        return;
    };
    let Some(pack) = pack(&selected.0) else {
        return;
    };
    let surface = selected.0.surface().clone();
    world.insert_resource(HighlandsActive);
    let mut lights = world.query_filtered::<(
        Entity,
        &DirectionalLight,
        Option<&bevy::light::CascadeShadowConfig>,
    ), With<Fvr04ProductionRuntimeSceneRoot>>();
    let suns: Vec<_> = lights
        .iter(world)
        .map(|(e, l, c)| {
            (
                e,
                TerrainSun {
                    enabled: l.shadows_enabled,
                    cascades: c.map_or(1, |c| c.bounds.len()),
                },
            )
        })
        .collect();
    for (entity, sun) in suns {
        world.entity_mut(entity).insert(sun);
    }
    let Some(server) = world.get_resource::<AssetServer>() else {
        return;
    };
    let mut handles = BTreeMap::new();
    for p in pack.placements() {
        handles.entry(p.kind.clone()).or_insert_with(|| {
            server.load(match pack {
                Pack::Highlands => format!("landscape/{}.glb", p.kind),
                Pack::Island => format!("{}/{}.glb", pack.folder(), p.kind),
            })
        });
    }
    let terrain = server.load(format!("{}/terrain-chunks.glb", pack.folder()));
    let water_height = match pack {
        Pack::Highlands => 0.25,
        Pack::Island => 0.0,
    };
    let size = match pack {
        Pack::Highlands => Vec2::new(800.0, 1050.0),
        Pack::Island => Vec2::splat(10000.0),
    };
    let center = Vec3::new(
        surface.origin_x + (surface.width - 1) as f32 * surface.spacing * 0.5,
        water_height,
        surface.origin_z + (surface.depth - 1) as f32 * surface.spacing * 0.5,
    );
    world.insert_resource(HighlandsAssets {
        terrain,
        props: handles,
        pack,
        surface: surface.clone(),
        spawned: false,
        next_update: 0.0,
    });
    let (mesh, material) = match pack {
        Pack::Island => {
            let (mesh, mut material, images) = terrain_water::island_water(&surface, size, center);
            let [shore, ripple] =
                images.map(|image| world.resource_mut::<Assets<Image>>().add(image));
            material.base_color_texture = Some(shore);
            material.normal_map_texture = Some(ripple);
            (mesh, material)
        }
        Pack::Highlands => (
            Plane3d::default().mesh().size(size.x, size.y).build(),
            StandardMaterial {
                base_color: Color::linear_rgb(0.025, 0.24, 0.33),
                perceptual_roughness: 0.28,
                reflectance: 0.42,
                ..default()
            },
        ),
    };
    let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
    let water = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(material);
    world.spawn((
        Name::new("Approved terrain water"),
        Mesh3d(mesh),
        MeshMaterial3d(water),
        Transform::from_translation(center),
        NotShadowCaster,
        Fvr04ProductionRuntimeSceneRoot,
    ));
}
pub(super) fn view_height(projection: &Projection) -> f32 {
    match projection {
        Projection::Orthographic(p) => match p.scaling_mode {
            bevy::camera::ScalingMode::FixedVertical { viewport_height } => {
                viewport_height * p.scale
            }
            _ => p.area.height(),
        },
        _ => 30.0,
    }
}
pub(super) fn focus_on_surface(camera: &Transform, surface: &alife_world::TerrainSurface) -> Vec3 {
    let d = camera.rotation * Vec3::NEG_Z;
    surface
        .ray_hit(
            Vec3f::new(
                camera.translation.x,
                camera.translation.y,
                camera.translation.z,
            ),
            Vec3f::new(d.x, d.y, d.z),
            10000.0,
        )
        .map(|p| Vec3::new(p.x, p.y, p.z))
        .unwrap_or_else(|| camera.translation + d * (-camera.translation.y / d.y.min(-0.01)))
}
#[allow(
    clippy::too_many_arguments,
    reason = "Bevy injects independent ECS system parameters."
)]
pub(super) fn update(
    mut commands: Commands,
    time: Res<Time>,
    assets: Option<ResMut<HighlandsAssets>>,
    gltfs: Option<Res<Assets<Gltf>>>,
    gltf_meshes: Option<Res<Assets<GltfMesh>>>,
    mut chunks: Query<(&mut Mesh3d, &mut TerrainLod), Without<PropRange>>,
    mut props: Query<(&mut Mesh3d, &mut Visibility, &mut PropRange), Without<TerrainLod>>,
    cameras: Query<(&Transform, &Projection), With<Fvr03ProductionVoxelCamera>>,
    creatures: Query<&Transform, With<ProductionCreatureAssemblyRoot>>,
    mut suns: Query<(
        &mut DirectionalLight,
        &mut bevy::light::CascadeShadowConfig,
        &TerrainSun,
    )>,
) {
    let Some(mut assets) = assets else {
        return;
    };
    let (Some(gltfs), Some(gltf_meshes)) = (gltfs, gltf_meshes) else {
        return;
    };
    let Some((camera, projection)) = cameras.iter().next() else {
        return;
    };
    if !assets.spawned {
        let Some(terrain) = gltfs.get(&assets.terrain) else {
            return;
        };
        if assets.props.values().any(|h| gltfs.get(h).is_none()) {
            return;
        }
        let surface = &assets.surface;
        let nx = (surface.width - 1).div_ceil(32);
        let nz = (surface.depth - 1).div_ceil(32);
        for z in 0..nz {
            for x in 0..nx {
                let mesh_for = |level| {
                    let name = format!("{}_{x}_{z}_L{level}", assets.pack.prefix());
                    gltf_meshes
                        .get(&terrain.named_meshes[name.as_str()])
                        .expect("approved chunk")
                };
                let levels =
                    std::array::from_fn(|level| mesh_for(level).primitives[0].mesh.clone());
                let cx = surface.origin_x
                    + (x * 32 + 16).min(surface.width - 1) as f32 * surface.spacing;
                let cz = surface.origin_z
                    + (z * 32 + 16).min(surface.depth - 1) as f32 * surface.spacing;
                let center = Vec3::new(cx, surface.height(cx, cz).unwrap_or(0.0), cz);
                commands.spawn((
                    Name::new(format!("{} chunk {x}/{z}", assets.pack.prefix())),
                    Mesh3d(levels[0].clone()),
                    MeshMaterial3d(mesh_for(0).primitives[0].material.clone().unwrap()),
                    Transform::default(),
                    TerrainLod {
                        meshes: levels,
                        center,
                        current: 0,
                    },
                    Fvr04ProductionRuntimeSceneRoot,
                ));
            }
        }
        let mut shared_prop_material = None;
        for p in assets.pack.placements() {
            let gltf = gltfs.get(&assets.props[&p.kind]).unwrap();
            let position = Vec3::from_array(p.position);
            let tree = p.kind.starts_with("Oak")
                || matches!(p.kind.as_str(), "Broadleaf" | "Conifer" | "Sapling");
            let small = matches!(
                p.kind.as_str(),
                "Grass_clump" | "Fern_patch" | "Wildflower_patch" | "Grass" | "Fern" | "Flowers"
            );
            let distance = if small {
                45.0
            } else if tree {
                360.0
            } else {
                210.0
            };
            let meshes: Vec<_> = match assets.pack {
                Pack::Highlands => gltf
                    .meshes
                    .iter()
                    .map(|h| gltf_meshes.get(h).unwrap())
                    .collect(),
                Pack::Island => vec![gltf_meshes
                    .get(&gltf.named_meshes[format!("{}_L0", p.kind).as_str()])
                    .unwrap()],
            };
            for source in meshes {
                for primitive in &source.primitives {
                    let levels = match assets.pack {
                        Pack::Highlands => std::array::from_fn(|_| primitive.mesh.clone()),
                        Pack::Island => std::array::from_fn(|level| {
                            gltf_meshes
                                .get(&gltf.named_meshes[format!("{}_L{level}", p.kind).as_str()])
                                .unwrap()
                                .primitives[0]
                                .mesh
                                .clone()
                        }),
                    };
                    let material = match assets.pack {
                        Pack::Highlands => primitive.material.clone().unwrap(),
                        Pack::Island => shared_prop_material
                            .get_or_insert_with(|| primitive.material.clone().unwrap())
                            .clone(),
                    };
                    let mut entity = commands.spawn((
                        Name::new(format!("{} {}", assets.pack.prefix(), p.kind)),
                        Mesh3d(levels[0].clone()),
                        MeshMaterial3d(material),
                        Transform::from_translation(position)
                            .with_scale(Vec3::splat(p.scale))
                            .with_rotation(Quat::from_rotation_y(p.yaw)),
                        PropRange {
                            position,
                            distance,
                            meshes: levels,
                            current: 0,
                            tree,
                        },
                        Visibility::Hidden,
                        Fvr04ProductionRuntimeSceneRoot,
                    ));
                    if small {
                        entity.insert(NotShadowCaster);
                    }
                }
            }
        }
        info!(
            "{} loaded: {nx}x{nz} terrain chunks, shared prop meshes, 3 LODs",
            assets.pack.prefix()
        );
        assets.spawned = true;
    }
    if time.elapsed_secs_f64() < assets.next_update {
        return;
    }
    assets.next_update = time.elapsed_secs_f64() + 0.2;
    let focus = focus_on_surface(camera, &assets.surface);
    let height = view_height(projection);
    for (mut light, mut cascades, sun) in &mut suns {
        light.shadows_enabled = sun.enabled && height < 240.0;
        if light.shadows_enabled {
            let distance = camera.translation.distance(focus);
            let near = (distance - height * 0.5 - 90.0).max(0.1);
            let far = distance + height * 0.5 + 130.0;
            *cascades = bevy::light::CascadeShadowConfigBuilder {
                num_cascades: sun.cascades,
                minimum_distance: near,
                maximum_distance: far,
                first_cascade_far_bound: near + (far - near) * 0.55,
                overlap_proportion: 0.18,
            }
            .build();
        }
    }
    for (mut mesh, mut lod) in &mut chunks {
        let distance = Vec2::new(lod.center.x - focus.x, lod.center.z - focus.z).length();
        let occupied = creatures.iter().any(|c| {
            (c.translation.x - lod.center.x).abs() < assets.surface.spacing * 16.5
                && (c.translation.z - lod.center.z).abs() < assets.surface.spacing * 16.5
        });
        let target = if occupied || (height < 100.0 && distance < 150.0) {
            0
        } else if height < 500.0 && distance < 350.0 {
            1
        } else {
            2
        };
        if target != lod.current {
            mesh.0 = lod.meshes[target].clone();
            lod.current = target;
        }
    }
    for (mut mesh, mut visibility, mut prop) in &mut props {
        let distance = Vec2::new(prop.position.x - focus.x, prop.position.z - focus.z).length();
        let range = if prop.tree {
            prop.distance.max(height * 0.9)
        } else {
            prop.distance
        };
        *visibility = if distance < range && (prop.tree || height < 240.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let target = if height > 350.0 || distance > 240.0 {
            2
        } else if height > 80.0 || distance > 100.0 {
            1
        } else {
            0
        };
        if target != prop.current {
            mesh.0 = prop.meshes[target].clone();
            prop.current = target;
        }
    }
}
pub(super) fn constrain_camera(
    active: Option<Res<creature_grounding::SelectedTerrain>>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Fvr03ProductionVoxelCamera>>,
) {
    let Some(active) = active else {
        return;
    };
    for (mut camera, mut projection) in &mut cameras {
        let height = view_height(&projection);
        if let Projection::Orthographic(p) = &mut *projection {
            p.near = 0.1;
            p.far = 10000.0;
        }
        let surface = active.0.surface();
        constrain_to_surface(&mut camera, height, surface);
    }
}

fn constrain_to_surface(
    camera: &mut Transform,
    height: f32,
    surface: &alife_world::TerrainSurface,
) {
    let focus = focus_on_surface(camera, surface);
    let x = focus.x.clamp(
        surface.origin_x,
        surface.origin_x + (surface.width - 1) as f32 * surface.spacing,
    );
    let z = focus.z.clamp(
        surface.origin_z,
        surface.origin_z + (surface.depth - 1) as f32 * surface.spacing,
    );
    camera.translation.x += x - focus.x;
    camera.translation.z += z - focus.z;
    let forward = camera.rotation * Vec3::NEG_Z;
    let up = camera.rotation * Vec3::Y;
    let safe = surface.height(x, z).unwrap_or(0.0) + 245.0 + up.y.abs() * height * 0.5;
    if camera.translation.y < safe {
        let backoff = (safe - camera.translation.y) / (-forward.y).max(0.05);
        camera.translation -= forward * backoff;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn camera_zoom_keeps_the_ground_focus_and_clearance() {
        let terrain = alife_world::island_terrain();
        let surface = terrain.surface();
        for (x, z) in [(80.0, 250.0), (-200.0, -100.0), (160.0, -170.0)] {
            let focus = Vec3::new(x, surface.height(x, z).unwrap(), z);
            let mut camera = Transform::from_translation(focus + Vec3::new(35.0, 55.0, 45.0))
                .looking_at(focus, Vec3::Y);
            for height in [10.5, 30.0, 400.0, 1600.0, 400.0, 30.0, 10.5] {
                constrain_to_surface(&mut camera, height, surface);
                assert!(focus_on_surface(&camera, surface).distance(focus) < 0.03);
                assert!(camera.translation.y >= focus.y + 244.9);
                assert!(camera.translation.distance(focus) < 10000.0);
            }
        }
    }
}
