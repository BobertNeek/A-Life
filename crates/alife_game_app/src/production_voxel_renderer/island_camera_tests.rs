//! CPU/ECS camera-clamp checks, without renderer, window, or GPU plugins.
use super::*;
use bevy::ecs::change_detection::Tick;

fn flat_terrain(height: f32) -> alife_world::WorldTerrain {
    alife_world::WorldTerrain::new(
        alife_world::TerrainData {
            width: 2,
            depth: 2,
            origin_x: 0.0,
            origin_z: 0.0,
            spacing: 1.0,
            heights: vec![height; 4],
            obstacles: vec![],
            water_level: None,
        },
        Default::default(),
    )
    .unwrap()
}

fn down_camera(position: Vec3) -> Transform {
    Transform::from_translation(position).looking_at(position - Vec3::Y, Vec3::Z)
}

fn orthographic(height: f32) -> Projection {
    Projection::Orthographic(OrthographicProjection {
        near: 0.1,
        far: 10000.0,
        scaling_mode: bevy::camera::ScalingMode::FixedVertical {
            viewport_height: height,
        },
        ..OrthographicProjection::default_3d()
    })
}

fn fixture(active: bool) -> App {
    let mut app = App::new();
    app.add_systems(Update, constrain_camera);
    if active {
        app.insert_resource(creature_grounding::SelectedTerrain(flat_terrain(0.0)));
    }
    app
}

fn spawn_camera(app: &mut App, transform: Transform, projection: Projection) -> Entity {
    app.world_mut()
        .spawn((
            Fvr03ProductionVoxelCamera {
                mode: Fvr03ProductionVoxelCameraMode::OrthographicIsometric,
            },
            transform,
            projection,
        ))
        .id()
}

fn changed_tick<T: Component>(app: &App, entity: Entity) -> Tick {
    app.world()
        .entity(entity)
        .get_ref::<T>()
        .unwrap()
        .last_changed()
}

fn transform_bits(transform: &Transform) -> ([u32; 3], [u32; 4], [u32; 3]) {
    (
        transform.translation.to_array().map(f32::to_bits),
        transform.rotation.to_array().map(f32::to_bits),
        transform.scale.to_array().map(f32::to_bits),
    )
}

fn expected_camera(app: &App, entity: Entity) -> Transform {
    let mut expected = *app.world().get::<Transform>(entity).unwrap();
    let height = view_height(app.world().get::<Projection>(entity).unwrap());
    let selected = app
        .world()
        .resource::<creature_grounding::SelectedTerrain>();
    // The production clamp helper is unchanged from the source baseline.
    constrain_to_surface(&mut expected, height, selected.0.surface());
    expected
}

// The key handler reads only the profile; other scene metadata is inert.
fn camera_scene(profile_id: ProductionFrontendProfileId) -> Fvr03ProductionVoxelSceneResource {
    Fvr03ProductionVoxelSceneResource {
        schema: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA,
        schema_version: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA_VERSION,
        snapshot_schema: "CPU camera fixture".into(),
        profile_id,
        population: 1,
        renderer_profile: String::new(),
        backend_id: FVR10_RENDERER_BACKEND_ID,
        uses_internal_voxel_terrain_mesh: false,
        visible_chunk_count: 0,
        resident_chunk_count: 0,
        tile_mesh_count: 0,
        creature_render_count: 0,
        creature_material_bucket_count: 0,
        creature_lod: Fvr04CreatureLod::CompactVoxel,
        creature_root_count: 0,
        creature_part_entity_count: 0,
        creature_join_cover_count: 0,
        creature_part_family_count: 0,
        creature_mixed_assembly_count: 0,
        creature_shared_mesh_handle_count: 0,
        selection_ref_count: 0,
        dirty_chunk_count: 0,
        estimated_resident_bytes: 0,
        draw_radius_chunks: 0,
        target_fps: 30,
        performance_artifact_path: None,
        material_counts: BTreeMap::new(),
        average_resource_bias: 0.,
        average_hazard_pressure: 0.,
        production_dressing_count: 0,
        production_vfx_marker_count: 0,
        production_gpu_vfx_emitter_count: 0,
        production_vfx_budget_state: "CPU fixture",
        production_visuals_display_only: true,
        production_vfx_uses_hanabi_gpu_particles: false,
        mesh_stats: Fvr09TerrainMeshStats {
            mode: Fvr09MesherMode::LayeredGridQuads,
            visible_voxels: 0,
            naive_visible_faces: 0,
            emitted_quads: 0,
            face_reduction_ratio: 0.,
            remesh_time_micros: 0,
            dirty_chunks: 0,
            cached_chunks: 0,
            skipped_chunks: 0,
            remesh_budget_chunks_per_frame: 0,
            material_palette_version: "fixture",
            vertex_color_face_variation: false,
            top_side_color_separation: false,
            variation_bucket_count: 0,
            cache_key: String::new(),
        },
        visible_tiles: BTreeSet::new(),
        visible_chunks: BTreeSet::new(),
        tile_summaries_by_tile: BTreeMap::new(),
        creature_refs_by_tile: BTreeMap::new(),
        selection_positions_by_raw_id: BTreeMap::new(),
    }
}

#[test]
fn reset_view_restores_default_zoom_but_mode_switches_preserve_it() {
    for profile in ProductionFrontendProfileId::all() {
        for terrain_bound in [false, true] {
            let extent = production_camera_extent(*profile);
            let mut app = fixture(terrain_bound);
            app.init_resource::<ButtonInput<KeyCode>>()
                .insert_resource(camera_scene(*profile))
                .add_systems(
                    Update,
                    handle_fvr03_camera_mode_input.before(constrain_camera),
                );
            let camera = spawn_camera(
                &mut app,
                down_camera(Vec3::new(0.5, 300.0, 0.5)),
                orthographic(extent),
            );
            let (near, far) = if terrain_bound {
                (9.8 / extent, 1600.0 / extent)
            } else {
                (0.35, 3.5)
            };
            for (key, zoom, expected) in [
                (KeyCode::KeyO, near, near),
                (KeyCode::KeyI, far, far),
                (KeyCode::KeyR, far, 1.0),
                (KeyCode::KeyR, near, 1.0),
                (KeyCode::KeyR, 1.0, 1.0),
            ] {
                let mut projection = app.world_mut().get_mut::<Projection>(camera).unwrap();
                let Projection::Orthographic(projection) = &mut *projection else {
                    unreachable!()
                };
                projection.scale = zoom;
                let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                keys.reset_all();
                keys.press(key);
                app.update();
                let Projection::Orthographic(projection) =
                    app.world().get::<Projection>(camera).unwrap()
                else {
                    unreachable!()
                };
                assert_eq!(
                    projection.scale, expected,
                    "{profile:?} terrain={terrain_bound} {key:?} after zoom {zoom}"
                );
                assert_eq!(
                    app.world()
                        .get::<Fvr03ProductionVoxelCamera>(camera)
                        .unwrap()
                        .mode,
                    if key == KeyCode::KeyO {
                        Fvr03ProductionVoxelCameraMode::Orbit
                    } else {
                        Fvr03ProductionVoxelCameraMode::OrthographicIsometric
                    }
                );
            }
        }
    }
}

#[test]
fn production_camera_input_reset_wins_over_wheel_without_deferring_it() {
    use bevy::{
        ecs::schedule::{LogLevel, ScheduleBuildSettings},
        input::mouse::{MouseScrollUnit, MouseWheel},
    };
    for profile in ProductionFrontendProfileId::all() {
        for terrain_bound in [false, true] {
            for wheel in [-1.0, 1.0] {
                let extent = production_camera_extent(*profile);
                let mut app = fixture(terrain_bound);
                app.init_resource::<ButtonInput<KeyCode>>()
                    .add_message::<MouseWheel>()
                    .insert_resource(camera_scene(*profile))
                    // Use production's actual camera registration, not a test-only chain.
                    .add_systems(
                        Update,
                        camera_view_input_systems()
                            .in_set(ProductionVoxelPresentationSet::Input)
                            .before(constrain_camera),
                    )
                    .edit_schedule(Update, |schedule| {
                        schedule.set_build_settings(ScheduleBuildSettings {
                            ambiguity_detection: LogLevel::Error,
                            ..Default::default()
                        });
                    });
                let window = app
                    .world_mut()
                    .spawn((
                        bevy::window::Window {
                            focused: true,
                            ..Default::default()
                        },
                        bevy::window::PrimaryWindow,
                    ))
                    .id();
                let camera = spawn_camera(
                    &mut app,
                    down_camera(Vec3::new(0.5, 300.0, 0.5)),
                    orthographic(extent),
                );
                let scale = |app: &App| {
                    let Projection::Orthographic(p) =
                        app.world().get::<Projection>(camera).unwrap()
                    else {
                        unreachable!()
                    };
                    p.scale
                };
                let initial = if terrain_bound {
                    if wheel < 0.0 {
                        1600.0 / extent
                    } else {
                        9.8 / extent
                    }
                } else if wheel < 0.0 {
                    3.5
                } else {
                    0.35
                };
                let mut projection = app.world_mut().get_mut::<Projection>(camera).unwrap();
                let Projection::Orthographic(projection) = &mut *projection else {
                    unreachable!()
                };
                projection.scale = initial;
                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .press(KeyCode::KeyR);
                app.world_mut().write_message(MouseWheel {
                    unit: MouseScrollUnit::Line,
                    x: 0.0,
                    y: wheel,
                    window,
                });
                app.update();
                assert_eq!(
                    scale(&app),
                    1.0,
                    "{profile:?} terrain={terrain_bound} wheel={wheel}"
                );

                app.world_mut()
                    .resource_mut::<ButtonInput<KeyCode>>()
                    .reset_all();
                app.update();
                assert_eq!(
                    scale(&app),
                    1.0,
                    "same-frame scrolling must not be deferred"
                );

                app.world_mut().write_message(MouseWheel {
                    unit: MouseScrollUnit::Line,
                    x: 0.0,
                    y: -1.0,
                    window,
                });
                app.update();
                assert!(
                    scale(&app) > 1.0 && scale(&app) < 1.2,
                    "ordinary wheel input still zooms"
                );
                for key in [KeyCode::KeyI, KeyCode::KeyO] {
                    let mut projection = app.world_mut().get_mut::<Projection>(camera).unwrap();
                    let Projection::Orthographic(projection) = &mut *projection else {
                        unreachable!()
                    };
                    projection.scale = 1.0;
                    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
                    keys.reset_all();
                    keys.press(key);
                    app.world_mut().write_message(MouseWheel {
                        unit: MouseScrollUnit::Line,
                        x: 0.0,
                        y: -1.0,
                        window,
                    });
                    app.update();
                    assert!(
                        scale(&app) > 1.0 && scale(&app) < 1.2,
                        "view-mode changes retain wheel zoom"
                    );
                    assert_eq!(
                        app.world()
                            .get::<Fvr03ProductionVoxelCamera>(camera)
                            .unwrap()
                            .mode,
                        if key == KeyCode::KeyO {
                            Fvr03ProductionVoxelCameraMode::Orbit
                        } else {
                            Fvr03ProductionVoxelCameraMode::OrthographicIsometric
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn settled_cameras_preserve_transform_and_projection_change_ticks() {
    let mut app = fixture(true);
    let cameras = [
        orthographic(30.0),
        Projection::Perspective(PerspectiveProjection::default()),
    ]
    .map(|projection| {
        spawn_camera(
            &mut app,
            down_camera(Vec3::new(0.5, 300.0, 0.5)),
            projection,
        )
    });
    app.update();
    let ticks = cameras.map(|entity| {
        (
            changed_tick::<Transform>(&app, entity),
            changed_tick::<Projection>(&app, entity),
        )
    });
    for _ in 0..4 {
        app.update();
        for (entity, ticks) in cameras.into_iter().zip(ticks) {
            assert_eq!(changed_tick::<Transform>(&app, entity), ticks.0);
            assert_eq!(changed_tick::<Projection>(&app, entity), ticks.1);
        }
    }
}

#[test]
fn new_cameras_and_projection_variant_switches_are_reconciled() {
    let mut app = fixture(true);
    let first = spawn_camera(
        &mut app,
        down_camera(Vec3::new(0.5, 300.0, 0.5)),
        orthographic(30.0),
    );
    app.update();
    let first_tick = changed_tick::<Transform>(&app, first);
    let second = spawn_camera(
        &mut app,
        down_camera(Vec3::new(-5.0, 10.0, 3.0)),
        Projection::Orthographic(OrthographicProjection::default_3d()),
    );
    let expected = expected_camera(&app, second);
    app.update();
    assert_eq!(
        transform_bits(app.world().get::<Transform>(second).unwrap()),
        transform_bits(&expected)
    );
    assert_eq!(changed_tick::<Transform>(&app, first), first_tick);

    *app.world_mut().get_mut::<Projection>(first).unwrap() =
        Projection::Perspective(PerspectiveProjection {
            near: 0.6,
            far: 1234.0,
            ..default()
        });
    let edited_projection_tick = changed_tick::<Projection>(&app, first);
    app.update();
    assert_eq!(
        changed_tick::<Projection>(&app, first),
        edited_projection_tick
    );
    let Projection::Perspective(p) = app.world().get::<Projection>(first).unwrap() else {
        panic!("perspective camera");
    };
    assert_eq!((p.near, p.far), (0.6, 1234.0));

    *app.world_mut().get_mut::<Projection>(first).unwrap() =
        Projection::Orthographic(OrthographicProjection::default_3d());
    app.update();
    let Projection::Orthographic(p) = app.world().get::<Projection>(first).unwrap() else {
        panic!("orthographic camera");
    };
    assert_eq!((p.near, p.far), (0.1, 10000.0));
}

#[test]
fn first_pass_clamps_all_cameras_and_only_repairs_orthographic_clips() {
    let mut app = fixture(true);
    let original = down_camera(Vec3::new(-2.0, 10.0, 2.0)).with_scale(Vec3::new(2.0, 3.0, 4.0));
    let mut ortho = orthographic(30.0);
    let Projection::Orthographic(p) = &mut ortho else {
        unreachable!();
    };
    p.near = -7.0;
    p.far = 12.0;
    p.scale = 2.0;
    p.viewport_origin = Vec2::new(0.25, 0.75);
    let perspective = PerspectiveProjection {
        near: 0.7,
        far: 1234.0,
        ..default()
    };
    let orthographic = spawn_camera(&mut app, original, ortho);
    let perspective_entity = spawn_camera(
        &mut app,
        original,
        Projection::Perspective(perspective.clone()),
    );
    let expected = [orthographic, perspective_entity].map(|entity| expected_camera(&app, entity));
    let perspective_tick = changed_tick::<Projection>(&app, perspective_entity);
    app.update();
    for (entity, expected) in [orthographic, perspective_entity].into_iter().zip(expected) {
        let actual = app.world().get::<Transform>(entity).unwrap();
        assert_eq!(transform_bits(actual), transform_bits(&expected));
        assert_eq!(actual.rotation, original.rotation);
        assert_eq!(actual.scale, original.scale);
        assert!(actual.translation.y >= 245.0);
    }
    let Projection::Orthographic(p) = app.world().get::<Projection>(orthographic).unwrap() else {
        panic!("orthographic camera");
    };
    assert_eq!((p.near, p.far), (0.1, 10000.0));
    assert_eq!(p.scale, 2.0);
    assert_eq!(p.viewport_origin, Vec2::new(0.25, 0.75));
    let Projection::Perspective(p) = app.world().get::<Projection>(perspective_entity).unwrap()
    else {
        panic!("perspective camera");
    };
    assert_eq!((p.near, p.far), (perspective.near, perspective.far));
    assert_eq!(
        changed_tick::<Projection>(&app, perspective_entity),
        perspective_tick
    );
}

#[test]
fn external_edits_zoom_and_terrain_replacement_reach_original_clamp() {
    let mut app = fixture(true);
    let camera = spawn_camera(
        &mut app,
        down_camera(Vec3::new(0.5, 300.0, 0.5)),
        orthographic(30.0),
    );
    app.update();
    for height in [10.5, 30.0, 400.0, 1600.0, 400.0, 30.0, 10.5] {
        *app.world_mut().get_mut::<Projection>(camera).unwrap() = orthographic(height);
        *app.world_mut().get_mut::<Transform>(camera).unwrap() =
            Transform::from_xyz(-5.0, 10.0, 3.0).looking_at(Vec3::new(0.5, 0.0, 0.5), Vec3::Y);
        let expected = expected_camera(&app, camera);
        app.update();
        assert_eq!(
            transform_bits(app.world().get::<Transform>(camera).unwrap()),
            transform_bits(&expected)
        );
    }
    // Change projection alone after the tilted camera has settled. A future
    // Changed<Transform>-only shortcut must not skip this required backoff.
    let previous = transform_bits(app.world().get::<Transform>(camera).unwrap());
    let previous_tick = changed_tick::<Transform>(&app, camera);
    *app.world_mut().get_mut::<Projection>(camera).unwrap() = orthographic(1600.0);
    let expected = expected_camera(&app, camera);
    assert_ne!(previous.0, transform_bits(&expected).0);
    app.update();
    assert_eq!(
        transform_bits(app.world().get::<Transform>(camera).unwrap()),
        transform_bits(&expected)
    );
    assert_ne!(changed_tick::<Transform>(&app, camera), previous_tick);

    app.insert_resource(creature_grounding::SelectedTerrain(flat_terrain(500.0)));
    let expected = expected_camera(&app, camera);
    app.update();
    assert_eq!(
        transform_bits(app.world().get::<Transform>(camera).unwrap()),
        transform_bits(&expected)
    );
}

#[test]
fn external_clip_edits_are_repaired_without_dirtying_an_equal_transform() {
    let mut app = fixture(true);
    let camera = spawn_camera(
        &mut app,
        down_camera(Vec3::new(0.5, 300.0, 0.5)),
        orthographic(30.0),
    );
    app.update();
    let transform_tick = changed_tick::<Transform>(&app, camera);
    for (near, far) in [
        (0.2, 10000.0),
        (0.1, 8000.0),
        (f32::NAN, f32::INFINITY),
        (f32::NEG_INFINITY, f32::NAN),
    ] {
        let mut projection = app.world_mut().get_mut::<Projection>(camera).unwrap();
        let Projection::Orthographic(p) = &mut *projection else {
            unreachable!();
        };
        p.near = near;
        p.far = far;
        let edited_projection_tick = changed_tick::<Projection>(&app, camera);
        app.update();
        assert_eq!(changed_tick::<Transform>(&app, camera), transform_tick);
        assert_ne!(
            changed_tick::<Projection>(&app, camera),
            edited_projection_tick
        );
        let Projection::Orthographic(p) = app.world().get::<Projection>(camera).unwrap() else {
            panic!("orthographic camera");
        };
        assert_eq!((p.near, p.far), (0.1, 10000.0));
        let projection_tick = changed_tick::<Projection>(&app, camera);
        app.update();
        assert_eq!(changed_tick::<Projection>(&app, camera), projection_tick);
    }
}

#[test]
fn missing_terrain_and_non_camera_entities_remain_untouched() {
    let mut app = fixture(false);
    let transform = down_camera(Vec3::new(-2.0, 10.0, 2.0));
    let camera = spawn_camera(&mut app, transform, orthographic(30.0));
    let other = app.world_mut().spawn((transform, orthographic(30.0))).id();
    let ticks = [camera, other].map(|entity| {
        (
            changed_tick::<Transform>(&app, entity),
            changed_tick::<Projection>(&app, entity),
        )
    });
    app.update();
    for (entity, ticks) in [camera, other].into_iter().zip(ticks) {
        assert_eq!(changed_tick::<Transform>(&app, entity), ticks.0);
        assert_eq!(changed_tick::<Projection>(&app, entity), ticks.1);
        assert_eq!(
            transform_bits(app.world().get::<Transform>(entity).unwrap()),
            transform_bits(&transform)
        );
    }
    app.insert_resource(creature_grounding::SelectedTerrain(flat_terrain(0.0)));
    app.update();
    assert_ne!(
        transform_bits(app.world().get::<Transform>(camera).unwrap()),
        transform_bits(&transform)
    );
    assert_eq!(
        transform_bits(app.world().get::<Transform>(other).unwrap()),
        transform_bits(&transform)
    );
    assert_eq!(changed_tick::<Transform>(&app, other), ticks[1].0);
    assert_eq!(changed_tick::<Projection>(&app, other), ticks[1].1);
}

#[test]
fn nonfinite_and_signed_zero_inputs_preserve_original_output_bits() {
    let mut app = fixture(true);
    let camera = spawn_camera(
        &mut app,
        down_camera(Vec3::new(0.5, 300.0, 0.5)),
        orthographic(30.0),
    );
    for position in [
        Vec3::new(-0.0, 300.0, -0.0),
        Vec3::new(f32::NAN, 300.0, 0.5),
        Vec3::new(0.5, f32::NAN, 0.5),
        Vec3::new(0.5, 300.0, f32::from_bits(0x7fc0_1234)),
        Vec3::new(f32::INFINITY, 300.0, f32::NEG_INFINITY),
        Vec3::new(0.5, f32::NEG_INFINITY, 0.5),
        Vec3::new(0.5, f32::INFINITY, 0.5),
    ] {
        let mut transform = down_camera(Vec3::new(0.5, 300.0, 0.5));
        transform.translation = position;
        *app.world_mut().get_mut::<Transform>(camera).unwrap() = transform;
        let expected = expected_camera(&app, camera);
        app.update();
        assert_eq!(
            transform_bits(app.world().get::<Transform>(camera).unwrap()),
            transform_bits(&expected)
        );
    }
    for height in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        *app.world_mut().get_mut::<Projection>(camera).unwrap() = orthographic(height);
        *app.world_mut().get_mut::<Transform>(camera).unwrap() =
            down_camera(Vec3::new(0.5, 10.0, 0.5));
        let expected = expected_camera(&app, camera);
        app.update();
        assert_eq!(
            transform_bits(app.world().get::<Transform>(camera).unwrap()),
            transform_bits(&expected)
        );
    }
}

#[test]
fn stable_nan_rotation_does_not_force_unrelated_translation_writes() {
    let mut app = fixture(true);
    let camera = spawn_camera(
        &mut app,
        Transform {
            translation: Vec3::splat(f32::from_bits(0x7fc0_1234)),
            rotation: Quat::from_xyzw(f32::NAN, 0.0, 0.0, 1.0),
            scale: Vec3::splat(f32::NAN),
        },
        orthographic(30.0),
    );
    app.update();
    // Some operations may normalize a NaN payload on the first pass. Once its
    // translation bits settle, unrelated nonfinite fields must not dirty it.
    let mut quiet_passes = 0;
    for _ in 0..3 {
        let before = transform_bits(app.world().get::<Transform>(camera).unwrap());
        let tick = changed_tick::<Transform>(&app, camera);
        let expected = expected_camera(&app, camera);
        app.update();
        assert_eq!(
            transform_bits(app.world().get::<Transform>(camera).unwrap()),
            transform_bits(&expected)
        );
        if before.0 == transform_bits(&expected).0 {
            assert_eq!(changed_tick::<Transform>(&app, camera), tick);
            quiet_passes += 1;
        }
    }
    assert!(quiet_passes > 0);
}
