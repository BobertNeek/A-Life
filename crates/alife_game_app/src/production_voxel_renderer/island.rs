//! Island chunk and prop rendering for the playable world.
mod atmosphere;
pub(super) mod capture_pose;
mod groves;
mod marine_sky;
mod nursery;
mod strandline;
use super::*;
use bevy::{
    gltf::{Gltf, GltfMesh},
    light::NotShadowCaster,
    prelude::*,
};

#[derive(Resource)]
pub(super) struct IslandActive;
fn placements() -> Vec<Prop> {
    serde_json::from_str(include_str!("../../assets/landscape/island/props.json"))
        .expect("validated island prop placements")
}
pub(super) fn has_baked_art(terrain: Option<&alife_world::WorldTerrain>) -> bool {
    terrain.is_some_and(|terrain| terrain.binding() == alife_world::island_terrain().binding())
}

#[derive(Resource)]
pub(super) struct IslandAssets {
    terrain: Handle<Gltf>,
    habitats: Handle<Gltf>,
    habitats_attached: bool,
    groves: Handle<Gltf>,
    groves_attached: bool,
    props: BTreeMap<String, Handle<Gltf>>,
    surface: std::sync::Arc<alife_world::TerrainSurface>,
    spawned: bool,
    bootstrap_ground: Option<Entity>,
    next_update: f64,
}
#[derive(Component)]
pub(super) struct TerrainLod {
    coord: (usize, usize),
    materials: [Handle<StandardMaterial>; 3],
    meshes: [Handle<Mesh>; 3],
    center: Vec3,
    current: usize,
}
#[derive(Component)]
pub(super) struct PropRange {
    grove_form: Option<groves::GroveForm>,
    position: Vec3,
    distance: f32,
    meshes: [Handle<Mesh>; 3],
    current: usize,
    tree: bool,
}

// Non-tree prop visibility must remain below the L2 selection cutoffs.
const SOLID_PROP_DRAW_DISTANCE: f32 = 210.0;
#[derive(Component)]
pub(super) struct TerrainSun {
    enabled: bool,
    cascades: usize,
}

#[derive(Clone, Copy, PartialEq)]
struct ShadowCascadeInputs {
    count: usize,
    near: f32,
    far: f32,
}

struct CachedShadowConfig {
    inputs: ShadowCascadeInputs,
    config: bevy::light::CascadeShadowConfig,
}

// Cache only the desired configuration. Each actual component is still checked
// every eligible update, including newly spawned or externally edited lights.
#[derive(Default)]
pub(super) struct ShadowConfigCache {
    current: Option<CachedShadowConfig>,
}

impl ShadowConfigCache {
    fn desired(&mut self, inputs: ShadowCascadeInputs) -> &bevy::light::CascadeShadowConfig {
        if self.current.as_ref().is_none_or(|c| c.inputs != inputs) {
            self.current = Some(CachedShadowConfig {
                inputs,
                config: bevy::light::CascadeShadowConfigBuilder {
                    num_cascades: inputs.count,
                    minimum_distance: inputs.near,
                    maximum_distance: inputs.far,
                    first_cascade_far_bound: inputs.near + (inputs.far - inputs.near) * 0.55,
                    overlap_proportion: 0.18,
                }
                .build(),
            });
        }
        &self
            .current
            .as_ref()
            .expect("desired cascade config")
            .config
    }
}

fn cascade_configs_equal(
    actual: &bevy::light::CascadeShadowConfig,
    desired: &bevy::light::CascadeShadowConfig,
) -> bool {
    actual.minimum_distance == desired.minimum_distance
        && actual.overlap_proportion == desired.overlap_proportion
        && actual.bounds == desired.bounds
}

#[derive(serde::Deserialize)]
struct Prop {
    kind: String,
    position: [f32; 3],
    scale: f32,
    yaw: f32,
}

fn has_mineral_surface(kind: &str) -> bool {
    matches!(
        kind,
        "BoulderA" | "BoulderB" | "BoulderC" | "Scree" | "Cliff"
    )
}

fn grounded_prop_position(
    kind: &str,
    authored: [f32; 3],
    surface: &alife_world::TerrainSurface,
) -> Vec3 {
    let mut position = Vec3::from_array(authored);
    // Ground-centred plants follow the selected near-mesh triangles. Keep
    // authored placements for trees and solid props with baked collision proxies.
    if matches!(
        kind,
        "Grass" | "Fern" | "Flowers" | "BerryBush" | "CoastalShrub"
    ) {
        if let Some(height) = surface.height(position.x, position.z) {
            position.y = height;
        }
    }
    position
}

fn prop_view_state(
    tree: bool,
    draw_distance: f32,
    distance: f32,
    height: f32,
) -> (Visibility, usize) {
    let range = if tree {
        draw_distance.max(height * 0.9)
    } else {
        draw_distance
    };
    let visibility = if distance < range && (tree || height < 240.0) {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    let lod = if height > 350.0 || distance > 240.0 {
        2
    } else if height > 80.0 || distance > 100.0 {
        1
    } else {
        0
    };
    (visibility, lod)
}

pub(super) fn stop(world: &mut World) {
    atmosphere::leave(world);
    world.remove_resource::<IslandAssets>();
}
pub(super) fn start(world: &mut World) {
    let Some(selected) = world.get_resource::<creature_grounding::SelectedTerrain>() else {
        return;
    };
    if !has_baked_art(Some(&selected.0)) {
        return;
    }
    let surface = selected.0.surface().clone();
    // A runtime scene can be rebuilt without leaving the island. Release the
    // prior light snapshot before capturing this scene's replacement sun.
    atmosphere::leave(world);
    world.insert_resource(IslandActive);
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
    atmosphere::enter(world);
    let Some(server) = world.get_resource::<AssetServer>() else {
        return;
    };
    let mut handles = BTreeMap::new();
    for p in placements() {
        handles
            .entry(p.kind.clone())
            .or_insert_with(|| server.load(format!("landscape/island/{}.glb", p.kind)));
    }
    let terrain = server.load("landscape/island/terrain-chunks.glb");
    let habitats = server.load("landscape/island/habitat-ground.glb");
    let groves = server.load(groves::PACK);
    let size = Vec2::splat(10000.0);
    let center = Vec3::new(
        surface.origin_x + (surface.width - 1) as f32 * surface.spacing * 0.5,
        0.0,
        surface.origin_z + (surface.depth - 1) as f32 * surface.spacing * 0.5,
    );
    let bootstrap_mesh = world
        .resource_mut::<Assets<Mesh>>()
        .add(bootstrap_surface(&surface));
    let bootstrap_material =
        world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial {
                perceptual_roughness: 1.0,
                ..default()
            });
    let bootstrap_ground = world
        .spawn((
            Name::new("Island while detailed terrain streams"),
            Mesh3d(bootstrap_mesh),
            MeshMaterial3d(bootstrap_material),
            NotShadowCaster,
            bevy::picking::Pickable::IGNORE,
            Fvr04ProductionRuntimeSceneRoot,
        ))
        .id();
    world.insert_resource(IslandAssets {
        terrain,
        habitats,
        habitats_attached: false,
        groves,
        groves_attached: false,
        props: handles,
        surface: surface.clone(),
        spawned: false,
        bootstrap_ground: Some(bootstrap_ground),
        next_update: 0.0,
    });
    nursery::plant(world, &surface);
    strandline::spawn(world);
    marine_sky::spawn(world);
    let (mesh, mut material, images) = terrain_water::island_water(&surface, size, center);
    let [shore, ripple] = images.map(|image| world.resource_mut::<Assets<Image>>().add(image));
    material.base_color_texture = Some(shore);
    material.normal_map_texture = Some(ripple);
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
/// The real island is immediately visible, even while the authored glTF is
/// streaming. The coarse mesh samples the same immutable heightfield and never
/// participates in picking, collision, or the simulation.
fn bootstrap_surface(surface: &alife_world::TerrainSurface) -> Mesh {
    let samples = |count| {
        (0..count - 1)
            .step_by(4)
            .chain(std::iter::once(count - 1))
            .collect::<Vec<usize>>()
    };
    let xs = samples(surface.width);
    let zs = samples(surface.depth);
    let mut positions = Vec::with_capacity(xs.len() * zs.len());
    let mut colors = Vec::with_capacity(positions.capacity());
    let mut indices = Vec::with_capacity((xs.len() - 1) * (zs.len() - 1) * 6);
    for (j, &z) in zs.iter().enumerate() {
        for (i, &x) in xs.iter().enumerate() {
            let wx = surface.origin_x + x as f32 * surface.spacing;
            let wz = surface.origin_z + z as f32 * surface.spacing;
            let h = surface.height(wx, wz).unwrap_or(0.0);
            positions.push([wx, h, wz]);
            let grass = ((h - 1.0) / 2.5).clamp(0., 1.);
            let grain = (wx * 0.23 + wz * 0.41).sin() * 0.014;
            colors.push([
                0.46 - 0.23 * grass + grain,
                0.40 - 0.085 * grass + grain,
                0.26 - 0.12 * grass + grain,
                1.0,
            ]);
            if i + 1 < xs.len() && j + 1 < zs.len() {
                let a = positions.len() as u32 - 1;
                let b = a + xs.len() as u32;
                indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_smooth_normals();
    mesh
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
    clippy::type_complexity,
    reason = "Bevy injects independent ECS system parameters and disjoint component queries."
)]
pub(super) fn update(
    mut commands: Commands,
    time: Res<Time>,
    assets: Option<ResMut<IslandAssets>>,
    gltfs: Option<Res<Assets<Gltf>>>,
    gltf_meshes: Option<Res<Assets<GltfMesh>>>,
    asset_server: Option<Res<AssetServer>>,
    mut chunks: Query<
        (
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut TerrainLod,
        ),
        Without<PropRange>,
    >,
    mut props: Query<
        (
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
            &mut PropRange,
        ),
        Without<TerrainLod>,
    >,
    mut strand_patches: strandline::PatchViews,
    mut cameras: Query<
        (
            &Transform,
            &Projection,
            Option<(&mut DistanceFog, &atmosphere::CoastalAir)>,
        ),
        With<Fvr03ProductionVoxelCamera>,
    >,
    creatures: Query<&Transform, With<ProductionCreatureAssemblyRoot>>,
    mut suns: Query<(
        &mut DirectionalLight,
        &mut bevy::light::CascadeShadowConfig,
        &TerrainSun,
    )>,
    mut shadow_configs: Local<ShadowConfigCache>,
) {
    let Some(mut assets) = assets else {
        return;
    };
    let (Some(gltfs), Some(gltf_meshes)) = (gltfs, gltf_meshes) else {
        return;
    };
    let Some((camera, projection, air)) = cameras.iter_mut().next() else {
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
                    let name = format!("Island_{x}_{z}_L{level}");
                    gltf_meshes
                        .get(&terrain.named_meshes[name.as_str()])
                        .expect("approved chunk")
                };
                let levels =
                    std::array::from_fn(|level| mesh_for(level).primitives[0].mesh.clone());
                let materials = std::array::from_fn(|level| {
                    mesh_for(level).primitives[0]
                        .material
                        .clone()
                        .expect("approved terrain material")
                });
                let cx = surface.origin_x
                    + (x * 32 + 16).min(surface.width - 1) as f32 * surface.spacing;
                let cz = surface.origin_z
                    + (z * 32 + 16).min(surface.depth - 1) as f32 * surface.spacing;
                let center = Vec3::new(cx, surface.height(cx, cz).unwrap_or(0.0), cz);
                commands.spawn((
                    Name::new(format!("Island chunk {x}/{z}")),
                    Mesh3d(levels[0].clone()),
                    MeshMaterial3d(materials[0].clone()),
                    Transform::default(),
                    TerrainLod {
                        coord: (x, z),
                        meshes: levels,
                        materials,
                        center,
                        current: 0,
                    },
                    Fvr04ProductionRuntimeSceneRoot,
                ));
            }
        }
        // Reuse exactly two source materials rather than the material of the
        // first prop for the entire island. The five stone meshes share one PBR
        // atlas; its normals must never be sent to foliage without those UVs.
        let mut shared_prop_materials = [None, None];
        for (index, p) in placements().into_iter().enumerate() {
            let gltf = gltfs.get(&assets.props[&p.kind]).unwrap();
            let position = grounded_prop_position(&p.kind, p.position, surface);
            let tree = matches!(p.kind.as_str(), "Broadleaf" | "Conifer" | "Sapling");
            let small = matches!(p.kind.as_str(), "Grass" | "Fern" | "Flowers");
            let distance = if small {
                45.0
            } else if tree {
                360.0
            } else {
                SOLID_PROP_DRAW_DISTANCE
            };
            let primitive = &gltf_meshes
                .get(&gltf.named_meshes[format!("{}_L0", p.kind).as_str()])
                .unwrap()
                .primitives[0];
            let levels = std::array::from_fn(|level| {
                gltf_meshes
                    .get(&gltf.named_meshes[format!("{}_L{level}", p.kind).as_str()])
                    .unwrap()
                    .primitives[0]
                    .mesh
                    .clone()
            });
            let family = usize::from(has_mineral_surface(&p.kind));
            let material = shared_prop_materials[family]
                .get_or_insert_with(|| primitive.material.clone().unwrap())
                .clone();
            let mut entity = commands.spawn((
                Name::new(format!("Island {}", p.kind)),
                Mesh3d(levels[0].clone()),
                MeshMaterial3d(material),
                Transform::from_translation(position)
                    .with_scale(Vec3::splat(p.scale))
                    .with_rotation(Quat::from_rotation_y(p.yaw)),
                PropRange {
                    grove_form: groves::at(index, &p),
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
        info!("Island loaded: {nx}x{nz} terrain chunks, shared prop meshes, 3 LODs");
        if let Some(ground) = assets.bootstrap_ground.take() {
            commands.entity(ground).despawn();
        }
        assets.spawned = true;
    }
    if time.elapsed_secs_f64() < assets.next_update {
        return;
    }
    assets.next_update = time.elapsed_secs_f64() + 0.2;
    let focus = focus_on_surface(camera, &assets.surface);
    let height = view_height(projection);
    strandline::update(&mut strand_patches, focus, height);
    if let Some((mut fog, _)) = air {
        atmosphere::refocus(&mut fog, camera.translation.distance(focus), height);
    }
    for (mut light, mut cascades, sun) in &mut suns {
        let shadows_enabled = sun.enabled && height < 240.0;
        if light.shadows_enabled != shadows_enabled {
            light.shadows_enabled = shadows_enabled;
        }
        if shadows_enabled {
            let distance = camera.translation.distance(focus);
            let near = (distance - height * 0.5 - 90.0).max(0.1);
            let far = distance + height * 0.5 + 130.0;
            let desired = shadow_configs.desired(ShadowCascadeInputs {
                count: sun.cascades,
                near,
                far,
            });
            if !cascade_configs_equal(&cascades, desired) {
                let actual = &mut *cascades;
                actual.bounds.clone_from(&desired.bounds);
                actual.minimum_distance = desired.minimum_distance;
                actual.overlap_proportion = desired.overlap_proportion;
            }
        }
    }
    // Optional presentation art must never hold the island or its inhabitants
    // behind a slow/missing texture fetch. Bind once after the original chunks
    // are live; L1/L2 and the underwater chunks stay on their original asset.
    let ready_habitats = (!assets.habitats_attached)
        .then(|| gltfs.get(&assets.habitats))
        .flatten()
        .filter(|pack| {
            pack.named_meshes
                .values()
                .all(|h| gltf_meshes.get(h).is_some())
        });
    let mut saw_chunk = false;
    for (mut mesh, mut material, mut lod) in &mut chunks {
        saw_chunk = true;
        let mut near_replaced = false;
        if let Some(pack) = ready_habitats {
            let (x, z) = lod.coord;
            if let Some(handle) = pack.named_meshes.get(format!("Habitat_{x}_{z}").as_str()) {
                let prim = &gltf_meshes.get(handle).expect("ready habitat").primitives[0];
                lod.meshes[0] = prim.mesh.clone();
                lod.materials[0] = prim.material.clone().expect("approved habitat material");
                near_replaced = true;
            }
        }
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
        if target != lod.current || (target == 0 && near_replaced) {
            mesh.0 = lod.meshes[target].clone();
            if material.0 != lod.materials[target] {
                material.0 = lod.materials[target].clone();
            }
            lod.current = target;
        }
    }
    if saw_chunk && ready_habitats.is_some() {
        assets.habitats_attached = true;
    }
    let grove_models = (!assets.groves_attached
        && asset_server
            .as_ref()
            .is_none_or(|server| server.is_loaded_with_dependencies(&assets.groves)))
    .then(|| gltfs.get(&assets.groves))
    .flatten()
    .and_then(|pack| groves::models(pack, &gltf_meshes));
    let mut saw_prop = false;
    for (mut mesh, mut material, mut visibility, mut prop) in &mut props {
        saw_prop = true;
        let mut crown_replaced = false;
        if let (Some(models), Some(form)) = (grove_models.as_ref(), prop.grove_form) {
            prop.meshes = groves::levels(&form, models).clone();
            material.0 = models.material.clone();
            crown_replaced = true;
        }
        let distance = Vec2::new(prop.position.x - focus.x, prop.position.z - focus.z).length();
        let (next_visibility, target) = prop_view_state(prop.tree, prop.distance, distance, height);
        visibility.set_if_neq(next_visibility);
        if target != prop.current || crown_replaced {
            mesh.0 = prop.meshes[target].clone();
            prop.current = target;
        }
    }
    if saw_prop && grove_models.is_some() {
        assets.groves_attached = true;
    }
}

#[cfg(test)]
#[path = "island_update_tests.rs"]
mod update_tests;
pub(super) fn constrain_camera(
    active: Option<Res<creature_grounding::SelectedTerrain>>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Fvr03ProductionVoxelCamera>>,
) {
    let Some(active) = active else {
        return;
    };
    for (mut camera, mut projection) in &mut cameras {
        let height = view_height(&projection);
        if matches!(
            &*projection,
            Projection::Orthographic(p) if p.near != 0.1 || p.far != 10000.0
        ) {
            if let Projection::Orthographic(p) = &mut *projection {
                p.near = 0.1;
                p.far = 10000.0;
            }
        }
        let surface = active.0.surface();
        // Keep the original clamp math on a copy, so equal results do not dirty
        // the ECS transform. Bit equality also preserves signed-zero and NaN outputs.
        let mut constrained = *camera;
        constrain_to_surface(&mut constrained, height, surface);
        if constrained.translation.to_array().map(f32::to_bits)
            != camera.translation.to_array().map(f32::to_bits)
        {
            camera.translation = constrained.translation;
        }
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
    fn bootstrap_ground_spans_the_real_island_and_uses_its_elevation() {
        let terrain = alife_world::island_terrain();
        let surface = terrain.surface();
        let mesh = bootstrap_surface(surface);
        let bevy::mesh::VertexAttributeValues::Float32x3(points) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap()
        else {
            panic!("positions");
        };
        assert!(points.len() < surface.width * surface.depth / 8);
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert_eq!(points[0][0], surface.origin_x);
        assert_eq!(
            points.last().unwrap()[2],
            surface.origin_z + (surface.depth - 1) as f32 * surface.spacing
        );
        assert!(points
            .iter()
            .all(|&[x, y, z]| (surface.height(x, z).unwrap() - y).abs() < 1e-5));
        assert_eq!(mesh.indices().unwrap().len() % 3, 0);
    }

    #[test]
    fn corrected_boulder_placements_load_without_regrounding() {
        let props = placements();
        let terrain = alife_world::island_terrain();
        for (index, kind, position, proxy) in [
            (
                1319,
                "BoulderC",
                [35.826_015, 154.784_97, -212.133_06],
                1310,
            ),
            (1325, "BoulderA", [20.672_583, 64.743_13, -79.558_28], 1316),
            (1364, "BoulderA", [95.546_88, 5.163_428, 368.988_74], 1348),
            (1541, "BoulderC", [-159.474_44, 85.708_21, -63.083_46], 1489),
            (
                1730,
                "BoulderC",
                [-57.308_784, 137.131_4, -286.232_33],
                1631,
            ),
        ] {
            let prop = &props[index];
            assert_eq!(prop.kind, kind);
            assert_eq!(prop.position, position);
            assert_eq!(
                grounded_prop_position(&prop.kind, prop.position, terrain.surface()),
                Vec3::from_array(prop.position)
            );
            assert_eq!(terrain.state().data.obstacles[proxy][4], prop.position[1]);
        }
    }

    #[test]
    fn solid_prop_l2_is_always_culled_by_shipping_policy() {
        assert_eq!(placements()[1325].kind, "BoulderA");
        assert_eq!(SOLID_PROP_DRAW_DISTANCE, 210.0);
        for height in [
            9.8, 30.0, 80.0, 80.001, 239.999, 240.0, 240.001, 350.0, 350.001, 1600.0,
        ] {
            for distance in [
                0.0, 99.999, 100.0, 100.001, 209.999, 210.0, 210.001, 240.0, 240.001, 1000.0,
            ] {
                let (visibility, lod) =
                    prop_view_state(false, SOLID_PROP_DRAW_DISTANCE, distance, height);
                if visibility == Visibility::Inherited {
                    assert!(distance < 210.0 && height < 240.0);
                    assert!(lod < 2);
                }
            }
        }
        assert_eq!(
            prop_view_state(false, SOLID_PROP_DRAW_DISTANCE, 209.999, 239.999),
            (Visibility::Inherited, 1)
        );
        assert_eq!(
            prop_view_state(false, SOLID_PROP_DRAW_DISTANCE, 210.0, 30.0),
            (Visibility::Hidden, 1)
        );
        assert_eq!(
            prop_view_state(false, SOLID_PROP_DRAW_DISTANCE, 0.0, 240.0),
            (Visibility::Hidden, 1)
        );
        assert_eq!(
            prop_view_state(false, SOLID_PROP_DRAW_DISTANCE, 240.001, 30.0),
            (Visibility::Hidden, 2)
        );
        assert_eq!(
            prop_view_state(false, SOLID_PROP_DRAW_DISTANCE, 0.0, 350.001),
            (Visibility::Hidden, 2)
        );
        // Positive control: the unchanged tree policy can display L2.
        assert_eq!(
            prop_view_state(true, 360.0, 241.0, 351.0),
            (Visibility::Inherited, 2)
        );
    }

    #[test]
    fn corrected_island_binding_preserves_old_save_state_with_generic_art_fallback() {
        let current = alife_world::island_terrain();
        let current_state = current.state();
        let restored =
            alife_world::WorldTerrain::restore(Some(current.binding()), Some(&current_state))
                .unwrap()
                .unwrap();
        assert_eq!(restored.state(), current_state);
        assert!(has_baked_art(Some(&restored)));

        for original_main in [false, true] {
            let mut previous_state = current_state.clone();
            for (proxy, lo, hi) in [
                (1316, 0x42826693, 0x4286f3aa),
                (1348, 0x40a93bb5, 0x40ec0376),
                (1489, 0x42abb407, 0x42acf04c),
                (1631, 0x4309422f, 0x430a306d),
            ] {
                previous_state.data.obstacles[proxy][4] = f32::from_bits(lo);
                previous_state.data.obstacles[proxy][5] = f32::from_bits(hi);
            }
            if original_main {
                previous_state.data.obstacles[1310][4] = 155.707_53;
                previous_state.data.obstacles[1310][5] = 156.379_7;
            }
            let previous = alife_world::WorldTerrain::new(
                previous_state.data.clone(),
                previous_state.locomotion,
            )
            .unwrap();
            let restored =
                alife_world::WorldTerrain::restore(Some(previous.binding()), Some(&previous_state))
                    .unwrap()
                    .unwrap();
            assert_eq!(restored.state(), previous_state);
            assert_ne!(restored.binding(), current.binding());
            assert!(!has_baked_art(Some(&restored)));
        }
    }

    fn plant_grounding_surface() -> alife_world::TerrainSurface {
        alife_world::TerrainSurface::from_data(alife_world::TerrainData {
            width: 2,
            depth: 2,
            origin_x: 0.0,
            origin_z: 0.0,
            spacing: 1.0,
            heights: vec![0.0, 2.0, 4.0, 10.0],
            obstacles: vec![],
            water_level: None,
        })
        .unwrap()
    }

    #[test]
    fn plant_grounding_uses_both_near_mesh_triangles() {
        let surface = plant_grounding_surface();
        for kind in ["Grass", "Fern", "Flowers", "BerryBush", "CoastalShrub"] {
            for (x, z, height) in [(0.75, 0.25, 3.5), (0.25, 0.75, 4.5)] {
                let position = grounded_prop_position(kind, [x, 12.0, z], &surface);
                assert_eq!(position, Vec3::new(x, height, z));
                assert_eq!(Some(position.y), surface.height(x, z));
            }
        }
    }

    #[test]
    fn plant_grounding_preserves_other_authored_positions() {
        let surface = plant_grounding_surface();
        let authored = [0.75, 12.0, 0.25];
        for kind in [
            "Broadleaf",
            "Conifer",
            "Sapling",
            "BoulderA",
            "BoulderB",
            "BoulderC",
            "Scree",
            "Cliff",
            "Unknown",
            "grass",
        ] {
            assert_eq!(
                grounded_prop_position(kind, authored, &surface),
                Vec3::from_array(authored)
            );
        }
    }

    #[test]
    fn plant_grounding_preserves_authored_position_when_sample_is_invalid() {
        let surface = plant_grounding_surface();
        for authored in [
            [-0.25, 12.0, 0.5],
            [1.25, 12.0, 0.5],
            [0.5, 12.0, -0.25],
            [0.5, 12.0, 1.25],
            [f32::NAN, 12.0, 0.5],
            [0.5, 12.0, f32::INFINITY],
        ] {
            for kind in ["Grass", "Fern", "Flowers", "BerryBush", "CoastalShrub"] {
                let position = grounded_prop_position(kind, authored, &surface);
                assert_eq!(
                    position.to_array().map(f32::to_bits),
                    authored.map(f32::to_bits)
                );
            }
        }
    }

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

#[cfg(test)]
#[path = "island_camera_tests.rs"]
mod camera_tests;
