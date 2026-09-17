//! Chunk LOD and shared prop rendering for the authoritative highland surface.
use super::*;
use bevy::{
    gltf::{Gltf, GltfMesh},
    light::NotShadowCaster,
    prelude::*,
};

#[derive(Resource)]
pub(super) struct HighlandsActive;
#[derive(Resource)]
pub(super) struct HighlandsAssets {
    terrain: Handle<Gltf>,
    props: BTreeMap<String, Handle<Gltf>>,
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
    world.insert_resource(HighlandsActive);
    let Some(server) = world.get_resource::<AssetServer>() else {
        return;
    };
    let props: Vec<Prop> =
        serde_json::from_str(include_str!("../../assets/landscape/highlands/props.json"))
            .expect("validated prop export");
    let mut handles = BTreeMap::new();
    for p in &props {
        handles
            .entry(p.kind.clone())
            .or_insert_with(|| server.load(format!("landscape/{}.glb", p.kind)));
    }
    let terrain = server.load("landscape/highlands/terrain-chunks.glb");
    world.insert_resource(HighlandsAssets {
        terrain,
        props: handles,
        spawned: false,
        next_update: 0.0,
    });
    let mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(Plane3d::default().mesh().size(800.0, 1050.0));
    // Match the approved river. The legacy water material expects atlas-tile UVs,
    // which cannot be stretched across this entire valley plane.
    let water = world
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::linear_rgb(0.035, 0.22, 0.28),
            perceptual_roughness: 0.26,
            reflectance: 0.42,
            ..default()
        });
    world.spawn((
        Name::new("Highlands river water"),
        Mesh3d(mesh),
        MeshMaterial3d(water),
        Transform::from_xyz(0.0, alife_world::highlands::HIGHLANDS_WATER_HEIGHT, -175.0),
        NotShadowCaster,
        Fvr04ProductionRuntimeSceneRoot,
    ));
}

pub(super) fn update(
    mut commands: Commands,
    time: Res<Time>,
    assets: Option<ResMut<HighlandsAssets>>,
    gltfs: Option<Res<Assets<Gltf>>>,
    gltf_meshes: Option<Res<Assets<GltfMesh>>>,
    mut chunks: Query<(&mut Mesh3d, &mut TerrainLod)>,
    mut props: Query<(&mut Visibility, &PropRange)>,
    cameras: Query<&Transform, With<Fvr03ProductionVoxelCamera>>,
    creatures: Query<&Transform, With<ProductionCreatureAssemblyRoot>>,
) {
    let Some(mut assets) = assets else {
        return;
    };
    let (Some(gltfs), Some(gltf_meshes)) = (gltfs, gltf_meshes) else {
        return;
    };
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    if !assets.spawned {
        let Some(terrain) = gltfs.get(&assets.terrain) else {
            return;
        };
        if assets.props.values().any(|h| gltfs.get(h).is_none()) {
            return;
        }
        for z in 0..14 {
            for x in 0..10 {
                let levels = std::array::from_fn::<_, 3, _>(|level| {
                    let name = format!("Highlands_{x}_{z}_L{level}");
                    let mesh = gltf_meshes
                        .get(&terrain.named_meshes[name.as_str()])
                        .expect("loaded terrain mesh");
                    mesh.primitives[0].mesh.clone()
                });
                let center = Vec3::new(
                    -400.0 + x as f32 * 80.0 + 40.0,
                    35.0,
                    -700.0 + z as f32 * 80.0 + 40.0,
                );
                let material = gltf_meshes
                    .get(&terrain.named_meshes[format!("Highlands_{x}_{z}_L0").as_str()])
                    .unwrap()
                    .primitives[0]
                    .material
                    .clone()
                    .unwrap();
                commands.spawn((
                    Name::new(format!("Highlands chunk {x}/{z}")),
                    Mesh3d(levels[0].clone()),
                    MeshMaterial3d(material),
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
        let placements: Vec<Prop> =
            serde_json::from_str(include_str!("../../assets/landscape/highlands/props.json"))
                .unwrap();
        for p in &placements {
            let gltf = gltfs.get(&assets.props[&p.kind]).unwrap();
            let position = Vec3::from_array(p.position);
            let distance = if p.kind == "Grass_clump" {
                45.0
            } else if p.kind == "Fern_patch" || p.kind == "Wildflower_patch" {
                38.0
            } else if p.kind.starts_with("Oak") {
                360.0
            } else {
                210.0
            };
            for handle in &gltf.meshes {
                for primitive in &gltf_meshes.get(handle).unwrap().primitives {
                    let mut entity = commands.spawn((
                        Name::new(format!("Highlands {}", p.kind)),
                        Mesh3d(primitive.mesh.clone()),
                        MeshMaterial3d(primitive.material.clone().unwrap()),
                        Transform::from_translation(position)
                            .with_scale(Vec3::splat(p.scale))
                            .with_rotation(Quat::from_rotation_y(p.yaw)),
                        PropRange { position, distance },
                        Visibility::Hidden,
                        Fvr04ProductionRuntimeSceneRoot,
                    ));
                    if distance < 50.0 {
                        entity.insert(NotShadowCaster);
                    }
                }
            }
        }
        info!("Highlands loaded: 140 terrain chunks, 3 LODs, {} shared-mesh prop placements; authoritative digest {:016x}",placements.len(),alife_world::highlands().digest);
        assets.spawned = true;
    }
    if time.elapsed_secs_f64() < assets.next_update {
        return;
    }
    assets.next_update = time.elapsed_secs_f64() + 0.2;
    for (mut mesh, mut lod) in &mut chunks {
        let distance = camera.translation.distance(lod.center);
        let occupied = creatures.iter().any(|c| {
            (c.translation.x - lod.center.x).abs() < 42.0
                && (c.translation.z - lod.center.z).abs() < 42.0
        });
        let target = if occupied || distance < 150.0 {
            0
        } else if distance < 350.0 {
            1
        } else {
            2
        };
        if target != lod.current {
            mesh.0 = lod.meshes[target].clone();
            lod.current = target;
        }
    }
    for (mut visibility, prop) in &mut props {
        let target =
            if camera.translation.distance_squared(prop.position) < prop.distance * prop.distance {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        if *visibility != target {
            *visibility = target;
        }
    }
}

pub(super) fn constrain_camera(
    active: Option<Res<creature_grounding::SelectedTerrain>>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Fvr03ProductionVoxelCamera>>,
) {
    if active.is_none() {
        return;
    }
    for (mut camera, mut projection) in &mut cameras {
        if let Projection::Orthographic(p) = &mut *projection {
            if p.far < 1800.0 {
                p.far = 1800.0;
            }
        }
        let surface = active.as_ref().unwrap().0.surface();
        camera.translation.x = camera.translation.x.clamp(
            surface.origin_x,
            surface.origin_x + (surface.width - 1) as f32 * surface.spacing,
        );
        camera.translation.z = camera.translation.z.clamp(
            surface.origin_z,
            surface.origin_z + (surface.depth - 1) as f32 * surface.spacing,
        );
        if let Some(h) = active
            .as_ref()
            .unwrap()
            .0
            .surface()
            .height(camera.translation.x, camera.translation.z)
        {
            camera.translation.y = camera.translation.y.max(h + 2.0);
        }
    }
}
