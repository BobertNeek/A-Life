//! CPU/ECS checks for island output changes, without renderer or GPU plugins.
use super::*;
use bevy::{ecs::change_detection::Tick, gltf::GltfPrimitive};
use std::time::Duration;

#[derive(Resource, Default, Debug, PartialEq, Eq)]
struct ChangedOutputs {
    visibility: usize,
    lights: usize,
    cascades: usize,
    meshes: usize,
    prop_materials: usize,
    terrain_lods: usize,
    prop_ranges: usize,
}

fn record_changes(
    visibility: Query<(), (With<PropRange>, Changed<Visibility>)>,
    lights: Query<(), (With<TerrainSun>, Changed<DirectionalLight>)>,
    cascades: Query<(), (With<TerrainSun>, Changed<bevy::light::CascadeShadowConfig>)>,
    meshes: Query<(), Changed<Mesh3d>>,
    prop_materials: Query<(), (With<PropRange>, Changed<MeshMaterial3d<StandardMaterial>>)>,
    terrain_lods: Query<(), Changed<TerrainLod>>,
    prop_ranges: Query<(), Changed<PropRange>>,
    mut changed: ResMut<ChangedOutputs>,
) {
    *changed = ChangedOutputs {
        visibility: visibility.iter().count(),
        lights: lights.iter().count(),
        cascades: cascades.iter().count(),
        meshes: meshes.iter().count(),
        prop_materials: prop_materials.iter().count(),
        terrain_lods: terrain_lods.iter().count(),
        prop_ranges: prop_ranges.iter().count(),
    };
}

fn flat_surface() -> alife_world::TerrainSurface {
    alife_world::TerrainSurface::from_data(alife_world::TerrainData {
        width: 2,
        depth: 2,
        origin_x: 0.0,
        origin_z: 0.0,
        spacing: 1.0,
        heights: vec![0.0; 4],
        obstacles: vec![],
        water_level: None,
    })
    .unwrap()
}

fn fixture() -> (App, Entity) {
    let mut app = App::new();
    app.insert_resource(Time::<()>::default())
        .init_resource::<Assets<Gltf>>()
        .init_resource::<Assets<GltfMesh>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<ChangedOutputs>()
        .insert_resource(IslandAssets {
            terrain: Handle::default(),
            habitats: Handle::default(),
            habitats_attached: false,
            groves: Handle::default(),
            groves_attached: false,
            props: BTreeMap::new(),
            surface: std::sync::Arc::new(flat_surface()),
            spawned: true,
            bootstrap_ground: None,
            next_update: 0.0,
        })
        .add_systems(Update, (update, record_changes).chain());
    let camera = app
        .world_mut()
        .spawn((
            Fvr03ProductionVoxelCamera {
                mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
            },
            Transform::from_translation(Vec3::new(0.5, 300.0, 0.5))
                .looking_at(Vec3::new(0.5, 0.0, 0.5), Vec3::Z),
            Projection::Orthographic(OrthographicProjection {
                scaling_mode: bevy::camera::ScalingMode::FixedVertical {
                    viewport_height: 30.0,
                },
                ..OrthographicProjection::default_3d()
            }),
        ))
        .id();
    (app, camera)
}

fn advance(app: &mut App, millis: u64) {
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(millis));
    app.update();
}

fn levels(app: &App) -> [Handle<Mesh>; 3] {
    std::array::from_fn(|_| app.world().resource::<Assets<Mesh>>().reserve_handle())
}

fn spawn_prop(app: &mut App, position: Vec3, tree: bool, distance: f32) -> Entity {
    let meshes = levels(app);
    app.world_mut()
        .spawn((
            Mesh3d(meshes[0].clone()),
            MeshMaterial3d::<StandardMaterial>(Handle::default()),
            Visibility::Hidden,
            PropRange {
                grove_form: None,
                position,
                distance,
                meshes,
                current: 0,
                tree,
            },
        ))
        .id()
}

fn spawn_sun(app: &mut App, enabled: bool, count: usize) -> Entity {
    app.world_mut()
        .spawn((
            DirectionalLight {
                shadows_enabled: enabled,
                ..default()
            },
            bevy::light::CascadeShadowConfig::default(),
            TerrainSun {
                enabled,
                cascades: count,
            },
        ))
        .id()
}

fn set_height(app: &mut App, camera: Entity, height: f32) {
    let mut projection = app.world_mut().get_mut::<Projection>(camera).unwrap();
    let Projection::Orthographic(projection) = &mut *projection else {
        panic!("orthographic fixture");
    };
    projection.scaling_mode = bevy::camera::ScalingMode::FixedVertical {
        viewport_height: height,
    };
}

fn changed_tick<T: Component>(app: &App, entity: Entity) -> Tick {
    app.world()
        .entity(entity)
        .get_ref::<T>()
        .unwrap()
        .last_changed()
}

fn assert_quiet(app: &App) {
    assert_eq!(
        app.world().resource::<ChangedOutputs>(),
        &ChangedOutputs::default()
    );
}

fn expected_cascades(app: &App, camera: Entity, count: usize) -> bevy::light::CascadeShadowConfig {
    let camera_transform = app.world().get::<Transform>(camera).unwrap();
    let surface = &app.world().resource::<IslandAssets>().surface;
    let height = view_height(app.world().get::<Projection>(camera).unwrap());
    let distance = camera_transform
        .translation
        .distance(focus_on_surface(camera_transform, surface));
    let near = (distance - height * 0.5 - 90.0).max(0.1);
    let far = distance + height * 0.5 + 130.0;
    bevy::light::CascadeShadowConfigBuilder {
        num_cascades: count,
        minimum_distance: near,
        maximum_distance: far,
        first_cascade_far_bound: near + (far - near) * 0.55,
        overlap_proportion: 0.18,
    }
    .build()
}

#[test]
fn stationary_outputs_keep_change_ticks_and_existing_mesh_guards() {
    let (mut app, _) = fixture();
    let prop = spawn_prop(&mut app, Vec3::new(0.5, 0.0, 0.5), false, 45.0);
    spawn_prop(&mut app, Vec3::new(500.0, 0.0, 0.5), false, 45.0);
    let sun = spawn_sun(&mut app, true, 3);
    app.update();
    let ticks = (
        changed_tick::<Visibility>(&app, prop),
        changed_tick::<DirectionalLight>(&app, sun),
        changed_tick::<bevy::light::CascadeShadowConfig>(&app, sun),
    );
    for _ in 0..3 {
        advance(&mut app, 201);
        assert_quiet(&app);
        assert_eq!(changed_tick::<Visibility>(&app, prop), ticks.0);
        assert_eq!(changed_tick::<DirectionalLight>(&app, sun), ticks.1);
        assert_eq!(
            changed_tick::<bevy::light::CascadeShadowConfig>(&app, sun),
            ticks.2
        );
    }
}

#[test]
fn cascade_cache_reuses_bounds_and_matches_the_existing_builder() {
    let mut cache = ShadowConfigCache::default();
    for inputs in [
        ShadowCascadeInputs {
            count: 1,
            near: 0.1,
            far: 445.0,
        },
        ShadowCascadeInputs {
            count: 3,
            near: 0.1,
            far: 445.0,
        },
        ShadowCascadeInputs {
            count: 3,
            near: 195.0,
            far: 445.0,
        },
        ShadowCascadeInputs {
            count: 3,
            near: 195.0,
            far: 470.0,
        },
    ] {
        let expected = bevy::light::CascadeShadowConfigBuilder {
            num_cascades: inputs.count,
            minimum_distance: inputs.near,
            maximum_distance: inputs.far,
            first_cascade_far_bound: inputs.near + (inputs.far - inputs.near) * 0.55,
            overlap_proportion: 0.18,
        }
        .build();
        let desired = cache.desired(inputs);
        assert!(cascade_configs_equal(desired, &expected));
        let bounds = desired.bounds.as_ptr();
        assert_eq!(cache.desired(inputs).bounds.as_ptr(), bounds);
    }
}

#[test]
fn external_edits_are_reconciled_without_overwriting_other_light_fields() {
    let (mut app, _) = fixture();
    let prop = spawn_prop(&mut app, Vec3::new(0.5, 0.0, 0.5), false, 45.0);
    let sun = spawn_sun(&mut app, true, 3);
    app.update();
    let expected = app
        .world()
        .get::<bevy::light::CascadeShadowConfig>(sun)
        .unwrap()
        .clone();
    for field in 0..6 {
        *app.world_mut().get_mut::<Visibility>(prop).unwrap() = Visibility::Hidden;
        {
            let mut light = app.world_mut().get_mut::<DirectionalLight>(sun).unwrap();
            light.shadows_enabled = false;
            light.illuminance = 1234.0;
            light.color = Color::srgb(0.2, 0.3, 0.4);
        }
        {
            let mut config = app
                .world_mut()
                .get_mut::<bevy::light::CascadeShadowConfig>(sun)
                .unwrap();
            match field {
                0 => config.minimum_distance += 1.0,
                1 => config.overlap_proportion = 0.3,
                2 => config.bounds[1] += 1.0,
                3 => config.minimum_distance = f32::NAN,
                4 => config.overlap_proportion = f32::NAN,
                _ => config.bounds[1] = f32::NAN,
            }
        }
        advance(&mut app, 201);
        assert_eq!(
            *app.world().get::<Visibility>(prop).unwrap(),
            Visibility::Inherited
        );
        let light = app.world().get::<DirectionalLight>(sun).unwrap();
        assert!(light.shadows_enabled);
        assert_eq!(light.illuminance, 1234.0);
        assert_eq!(light.color, Color::srgb(0.2, 0.3, 0.4));
        assert!(cascade_configs_equal(
            app.world()
                .get::<bevy::light::CascadeShadowConfig>(sun)
                .unwrap(),
            &expected
        ));
        advance(&mut app, 201);
        assert_quiet(&app);
    }
}

#[test]
fn zoom_threshold_throttle_and_lod_transitions_are_preserved() {
    let (mut app, camera) = fixture();
    let small = spawn_prop(&mut app, Vec3::new(0.5, 0.0, 0.5), false, 45.0);
    let tree = spawn_prop(&mut app, Vec3::new(0.5, 0.0, 0.5), true, 360.0);
    let sun = spawn_sun(&mut app, true, 3);
    app.update();
    set_height(&mut app, camera, 240.0);
    advance(&mut app, 100);
    assert_quiet(&app);
    assert_eq!(
        *app.world().get::<Visibility>(small).unwrap(),
        Visibility::Inherited
    );
    advance(&mut app, 101);
    assert_eq!(
        *app.world().get::<Visibility>(small).unwrap(),
        Visibility::Hidden
    );
    assert_eq!(
        *app.world().get::<Visibility>(tree).unwrap(),
        Visibility::Inherited
    );
    assert_eq!(app.world().get::<PropRange>(small).unwrap().current, 1);
    assert!(
        !app.world()
            .get::<DirectionalLight>(sun)
            .unwrap()
            .shadows_enabled
    );
    let disabled_config_tick = changed_tick::<bevy::light::CascadeShadowConfig>(&app, sun);
    advance(&mut app, 201);
    assert_quiet(&app);
    assert_eq!(
        changed_tick::<bevy::light::CascadeShadowConfig>(&app, sun),
        disabled_config_tick
    );
    set_height(&mut app, camera, 351.0);
    advance(&mut app, 201);
    assert_eq!(app.world().get::<PropRange>(tree).unwrap().current, 2);
    set_height(&mut app, camera, 30.0);
    advance(&mut app, 201);
    assert_eq!(
        *app.world().get::<Visibility>(small).unwrap(),
        Visibility::Inherited
    );
    assert_eq!(app.world().get::<PropRange>(tree).unwrap().current, 0);
    assert!(
        app.world()
            .get::<DirectionalLight>(sun)
            .unwrap()
            .shadows_enabled
    );
    advance(&mut app, 201);
    assert_quiet(&app);
}

#[test]
fn camera_distance_and_sun_policy_inputs_refresh_cascades() {
    let (mut app, camera) = fixture();
    let sun = spawn_sun(&mut app, true, 3);
    app.update();
    let focus_before = focus_on_surface(
        app.world().get::<Transform>(camera).unwrap(),
        &app.world().resource::<IslandAssets>().surface,
    );
    let bounds_before = app
        .world()
        .get::<bevy::light::CascadeShadowConfig>(sun)
        .unwrap()
        .bounds
        .as_ptr();
    let old = app
        .world()
        .get::<bevy::light::CascadeShadowConfig>(sun)
        .unwrap()
        .clone();
    app.world_mut()
        .get_mut::<Transform>(camera)
        .unwrap()
        .translation
        .y += 50.0;
    advance(&mut app, 201);
    let focus_after = focus_on_surface(
        app.world().get::<Transform>(camera).unwrap(),
        &app.world().resource::<IslandAssets>().surface,
    );
    assert!(focus_after.distance(focus_before) < 0.001);
    let actual = app
        .world()
        .get::<bevy::light::CascadeShadowConfig>(sun)
        .unwrap();
    assert!(!cascade_configs_equal(actual, &old));
    assert_eq!(actual.bounds.as_ptr(), bounds_before);
    assert!(cascade_configs_equal(
        actual,
        &expected_cascades(&app, camera, 3)
    ));
    app.world_mut().get_mut::<TerrainSun>(sun).unwrap().cascades = 1;
    advance(&mut app, 201);
    assert!(cascade_configs_equal(
        app.world()
            .get::<bevy::light::CascadeShadowConfig>(sun)
            .unwrap(),
        &expected_cascades(&app, camera, 1)
    ));
    app.world_mut().get_mut::<TerrainSun>(sun).unwrap().enabled = false;
    advance(&mut app, 201);
    assert!(
        !app.world()
            .get::<DirectionalLight>(sun)
            .unwrap()
            .shadows_enabled
    );
    advance(&mut app, 201);
    assert_quiet(&app);
    app.world_mut().get_mut::<TerrainSun>(sun).unwrap().enabled = true;
    advance(&mut app, 201);
    assert!(
        app.world()
            .get::<DirectionalLight>(sun)
            .unwrap()
            .shadows_enabled
    );
    advance(&mut app, 201);
    assert_quiet(&app);
}

#[test]
fn stationary_camera_still_reconciles_new_props_and_suns() {
    let (mut app, camera) = fixture();
    spawn_sun(&mut app, true, 3);
    app.update();
    advance(&mut app, 201);
    assert_quiet(&app);
    let prop = spawn_prop(&mut app, Vec3::new(0.5, 0.0, 0.5), false, 45.0);
    let sun = spawn_sun(&mut app, true, 1);
    advance(&mut app, 201);
    assert_eq!(
        *app.world().get::<Visibility>(prop).unwrap(),
        Visibility::Inherited
    );
    assert!(cascade_configs_equal(
        app.world()
            .get::<bevy::light::CascadeShadowConfig>(sun)
            .unwrap(),
        &expected_cascades(&app, camera, 1)
    ));
    advance(&mut app, 201);
    assert_quiet(&app);
}

#[test]
fn stationary_camera_still_tracks_creature_occupancy_and_removal() {
    let (mut app, _) = fixture();
    let meshes = levels(&app);
    app.init_resource::<Assets<StandardMaterial>>();
    let materials: [Handle<StandardMaterial>; 3] = std::array::from_fn(|_| {
        app.world()
            .resource::<Assets<StandardMaterial>>()
            .reserve_handle()
    });
    let near_material = materials[0].clone();
    let far_material = materials[2].clone();
    let center = Vec3::new(500.0, 0.0, 0.5);
    let chunk = app
        .world_mut()
        .spawn((
            Mesh3d(meshes[0].clone()),
            MeshMaterial3d(near_material.clone()),
            TerrainLod {
                coord: (0, 0),
                meshes,
                materials,
                center,
                current: 0,
            },
        ))
        .id();
    app.update();
    assert_eq!(app.world().get::<TerrainLod>(chunk).unwrap().current, 2);
    assert_eq!(
        app.world()
            .get::<MeshMaterial3d<StandardMaterial>>(chunk)
            .unwrap()
            .0,
        far_material
    );
    let creature = app
        .world_mut()
        .spawn((
            Transform::from_translation(center),
            ProductionCreatureAssemblyRoot {
                stable_id: WorldEntityId(1),
                organism_id: OrganismId(1),
                display_only: true,
            },
        ))
        .id();
    advance(&mut app, 201);
    assert_eq!(app.world().get::<TerrainLod>(chunk).unwrap().current, 0);
    assert_eq!(
        app.world()
            .get::<MeshMaterial3d<StandardMaterial>>(chunk)
            .unwrap()
            .0,
        near_material
    );
    app.world_mut()
        .get_mut::<Transform>(creature)
        .unwrap()
        .translation
        .x += 100.0;
    advance(&mut app, 201);
    assert_eq!(app.world().get::<TerrainLod>(chunk).unwrap().current, 2);
    app.world_mut()
        .get_mut::<Transform>(creature)
        .unwrap()
        .translation = center;
    advance(&mut app, 201);
    assert_eq!(app.world().get::<TerrainLod>(chunk).unwrap().current, 0);
    app.world_mut().despawn(creature);
    advance(&mut app, 201);
    assert_eq!(app.world().get::<TerrainLod>(chunk).unwrap().current, 2);
    advance(&mut app, 201);
    assert_quiet(&app);
}

fn primitive_fixture(app: &mut App, mesh: Handle<Mesh>) -> Handle<GltfMesh> {
    app.world_mut()
        .resource_mut::<Assets<GltfMesh>>()
        .add(GltfMesh {
            index: 0,
            name: "fixture".into(),
            primitives: vec![GltfPrimitive {
                index: 0,
                parent_mesh_index: 0,
                name: "fixture".into(),
                mesh,
                material: Some(Handle::default()),
                extras: None,
                material_extras: None,
            }],
            extras: None,
        })
}

fn pack_fixture() -> Gltf {
    Gltf {
        scenes: vec![],
        named_scenes: default(),
        meshes: vec![],
        named_meshes: default(),
        materials: vec![],
        named_materials: default(),
        nodes: vec![],
        named_nodes: default(),
        skins: vec![],
        named_skins: default(),
        default_scene: None,
        animations: vec![],
        named_animations: default(),
        source: None,
    }
}

fn install_spawn_assets(app: &mut App) -> Handle<StandardMaterial> {
    let mesh = primitive_fixture(app, Handle::default());
    let rock = primitive_fixture(app, Handle::default());
    app.init_resource::<Assets<StandardMaterial>>();
    let mineral = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .reserve_handle();
    app.world_mut()
        .resource_mut::<Assets<GltfMesh>>()
        .get_mut(&rock)
        .unwrap()
        .primitives[0]
        .material = Some(mineral.clone());
    let mut named_meshes = bevy::platform::collections::HashMap::default();
    for level in 0..3 {
        named_meshes.insert(
            format!("Island_0_0_L{level}").into_boxed_str(),
            mesh.clone(),
        );
        for prop in placements() {
            let source = if has_mineral_surface(&prop.kind) {
                &rock
            } else {
                &mesh
            };
            named_meshes.insert(
                format!("{}_L{level}", prop.kind).into_boxed_str(),
                source.clone(),
            );
        }
    }
    let gltf = app.world_mut().resource_mut::<Assets<Gltf>>().add(Gltf {
        named_meshes,
        ..pack_fixture()
    });
    let mut assets = app.world_mut().resource_mut::<IslandAssets>();
    assets.terrain = gltf.clone();
    assets.props = placements()
        .into_iter()
        .map(|p| (p.kind, gltf.clone()))
        .collect();
    assets.spawned = false;
    mineral
}

#[test]
fn all_rock_kinds_share_the_mineral_material_but_plants_keep_their_own() {
    let (mut app, _) = fixture();
    let mineral = install_spawn_assets(&mut app);
    app.update();
    let world = app.world_mut();
    let mut stones = 0;
    let mut plants = 0;
    for (name, material) in world
        .query_filtered::<(&Name, &MeshMaterial3d<StandardMaterial>), With<PropRange>>()
        .iter(world)
    {
        let kind = name.as_str().strip_prefix("Island ").unwrap();
        if matches!(
            kind,
            "Cliff" | "Scree" | "BoulderA" | "BoulderB" | "BoulderC"
        ) {
            assert_eq!(material.0, mineral);
            stones += 1;
        } else {
            assert_ne!(material.0, mineral);
            plants += 1;
        }
    }
    assert_eq!((stones, plants), (574, 3142));
}

#[test]
fn deferred_initial_population_is_reconciled_then_stops_visibility_writes() {
    let (mut app, camera) = fixture();
    install_spawn_assets(&mut app);
    app.update();
    assert!(app.world().resource::<IslandAssets>().spawned);
    let world = app.world_mut();
    let mut props = world.query::<(&PropRange, &Visibility)>();
    assert_eq!(props.iter(world).count(), 3716);
    assert!(props
        .iter(world)
        .all(|(_, visibility)| *visibility == Visibility::Hidden));
    advance(&mut app, 201);
    let world = app.world_mut();
    let camera = world.get::<Transform>(camera).unwrap();
    let focus = focus_on_surface(camera, &world.resource::<IslandAssets>().surface);
    let mut props = world.query::<(&PropRange, &Visibility)>();
    let mut visible = 0;
    for (prop, visibility) in props.iter(world) {
        let distance = Vec2::new(prop.position.x - focus.x, prop.position.z - focus.z).length();
        let (expected_visibility, expected_lod) =
            prop_view_state(prop.tree, prop.distance, distance, 30.0);
        assert_eq!(*visibility, expected_visibility);
        assert_eq!(prop.current, expected_lod);
        visible += usize::from(*visibility == Visibility::Inherited);
    }
    assert!(visible > 0);
    advance(&mut app, 201);
    assert_quiet(&app);
}

#[test]
fn late_habitat_material_replaces_only_near_and_never_blocks_original_population() {
    let (mut app, _) = fixture();
    install_spawn_assets(&mut app);
    app.update();
    // No loaded optional artwork, but the original terrain and props already exist.
    assert!(app.world().resource::<IslandAssets>().spawned);
    assert!(!app.world().resource::<IslandAssets>().habitats_attached);
    let unique = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .reserve_handle();
    let handle = primitive_fixture(&mut app, unique.clone());
    let mut pack = pack_fixture();
    pack.named_meshes.insert("Habitat_0_0".into(), handle);
    let pack = app.world_mut().resource_mut::<Assets<Gltf>>().add(pack);
    app.world_mut().resource_mut::<IslandAssets>().habitats = pack;
    advance(&mut app, 201);
    assert!(app.world().resource::<IslandAssets>().habitats_attached);
    let world = app.world_mut();
    let lod = world.query::<&TerrainLod>().single(world).unwrap();
    assert_eq!(lod.meshes[0], unique);
    assert_ne!(lod.meshes[1], unique);
    assert_ne!(lod.meshes[2], unique);
    advance(&mut app, 201);
    assert_quiet(&app);
}

#[test]
fn shipping_substrates_are_locked_to_near_chunks_and_real_pbr_maps() {
    let raw = include_bytes!("../../assets/landscape/island/habitat-ground.glb");
    assert_eq!(&raw[..4], b"glTF");
    let len = u32::from_le_bytes(raw[12..16].try_into().unwrap()) as usize;
    let doc: serde_json::Value = serde_json::from_slice(&raw[20..20 + len]).unwrap();
    let nodes = doc["nodes"].as_array().unwrap();
    let meshes = doc["meshes"].as_array().unwrap();
    assert_eq!(nodes.len(), 76);
    for node in nodes {
        let m = &meshes[node["mesh"].as_u64().unwrap() as usize];
        assert_eq!(node["name"], m["name"]);
        let parts: Vec<_> = m["name"].as_str().unwrap().split('_').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "Habitat");
        assert!(parts[1].parse::<usize>().unwrap() < 10);
        assert!(parts[2].parse::<usize>().unwrap() < 12);
        assert!(node["translation"].is_null() && node["scale"].is_null());
        assert_eq!(m["primitives"].as_array().unwrap().len(), 1);
        assert_eq!(m["primitives"][0]["material"], 0);
    }
    let m = &doc["materials"][0];
    assert_eq!(m["normalTexture"]["index"], 1);
    assert_eq!(m["pbrMetallicRoughness"]["baseColorTexture"]["index"], 0);
    assert_eq!(
        m["pbrMetallicRoughness"]["metallicRoughnessTexture"]["index"],
        2
    );
    for (entry, uri) in doc["images"].as_array().unwrap().iter().zip([
        "habitats/substrates.png",
        "habitats/substrates-normal.png",
        "habitats/substrates-orm.png",
    ]) {
        assert_eq!(entry["uri"].as_str(), Some(uri));
    }
}

#[test]
fn arriving_crown_pack_replaces_only_mapped_visuals_once_without_moving_a_trunk() {
    let (mut app, _) = fixture();
    install_spawn_assets(&mut app);
    app.update();
    let unique = app.world().resource::<Assets<Mesh>>().reserve_handle();
    let prim = primitive_fixture(&mut app, unique.clone());
    let pine = app
        .world()
        .resource::<Assets<StandardMaterial>>()
        .reserve_handle();
    app.world_mut()
        .resource_mut::<Assets<GltfMesh>>()
        .get_mut(&prim)
        .unwrap()
        .primitives[0]
        .material = Some(pine.clone());
    let mut pack = pack_fixture();
    pack.materials.push(pine.clone());
    for form in 1..=groves::FORMS {
        for level in 0..3 {
            pack.named_meshes.insert(
                format!("PineForm{form}_L{level}").into_boxed_str(),
                prim.clone(),
            );
        }
    }
    let world = app.world_mut();
    let before: std::collections::HashMap<_, _> = world
        .query_filtered::<(Entity, &Transform, &MeshMaterial3d<StandardMaterial>), With<PropRange>>(
        )
        .iter(world)
        .map(|(id, t, m)| (id, (*t, m.0.clone())))
        .collect();
    // A top-level glTF can appear before all named subassets. In that case
    // absolutely nothing in the trunk/material population should be replaced.
    let last = format!("PineForm{}_L2", groves::FORMS);
    let last_mesh = pack.named_meshes.remove(last.as_str()).unwrap();
    let pack = world.resource_mut::<Assets<Gltf>>().add(pack);
    world.resource_mut::<IslandAssets>().groves = pack.clone();
    advance(&mut app, 201);
    let world = app.world_mut();
    assert!(!world.resource::<IslandAssets>().groves_attached);
    for (entity, t, m) in world
        .query_filtered::<(Entity, &Transform, &MeshMaterial3d<StandardMaterial>), With<PropRange>>(
        )
        .iter(world)
    {
        assert_eq!(*t, before[&entity].0);
        assert_eq!(m.0, before[&entity].1);
    }
    world
        .resource_mut::<Assets<Gltf>>()
        .get_mut(&pack)
        .unwrap()
        .named_meshes
        .insert(last.into_boxed_str(), last_mesh);
    advance(&mut app, 201);
    let world = app.world_mut();
    assert!(world.resource::<IslandAssets>().groves_attached);
    let mut changed = 0;
    for (id, prop, t, mesh, mat) in world
        .query::<(
            Entity,
            &PropRange,
            &Transform,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
        )>()
        .iter(world)
    {
        assert_eq!(*t, before[&id].0);
        if prop.grove_form.is_some() {
            assert_eq!(mesh.0, unique);
            assert_eq!(mat.0, pine);
            changed += 1;
        } else {
            assert_ne!(mesh.0, unique);
            assert_eq!(mat.0, before[&id].1);
        }
    }
    assert_eq!(changed, 421);
    advance(&mut app, 201);
    assert_quiet(&app);
}
