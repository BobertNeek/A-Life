//! Real capture producers and Bevy screenshot extraction; no renderer or GPU.
use super::*;
use bevy::app::SubApp;
use bevy::asset::AssetPlugin;
use bevy::render::view::screenshot::{Captured, Capturing, ScreenshotCaptured, ScreenshotPlugin};
use bevy::render::{ExtractSchedule, MainWorld, RenderApp};
use std::process::Command;

#[test]
fn recorded_and_player_captures_share_the_frame_without_losing_outputs() {
    if let Some(directory) = std::env::var_os("ALIFE_SCREENSHOT_REQUEST_TEST") {
        let directory = PathBuf::from(directory);
        run_fixture(&directory, false);
        run_fixture(&directory, true);
        fs::write(
            directory.join("completed"),
            b"CPU extraction and observer delivery passed",
        )
        .unwrap();
        return;
    }
    let directory = std::env::temp_dir().join(format!(
        "alife-screenshot-requests-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&directory).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "production_voxel_renderer::graphics_capture::screenshot_tests::recorded_and_player_captures_share_the_frame_without_losing_outputs", "--nocapture", "--test-threads=1"])
        .env("ALIFE_SCREENSHOT_REQUEST_TEST", &directory)
        .env("ALIFE_GRAPHICS_CAPTURE_DIR", directory.join("player"))
        .env("ALIFE_GRAPHICS_CAPTURE_MAX_FRAMES", "1")
        .env_remove("ALIFE_ACTION_TRACE_PATH")
        .output().unwrap();
    let completed = directory.join("completed").is_file();
    let _ = fs::remove_dir_all(&directory);
    assert!(
        output.status.success(),
        "CPU screenshot regression failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(completed, "child fixture did not run to completion");
    eprintln!("{}", String::from_utf8_lossy(&output.stdout));
}

fn run_fixture(directory: &Path, fail_recorded_write: bool) {
    let player = directory.join("player");
    if player.exists() {
        fs::remove_dir_all(&player).unwrap();
    }
    let recorded = directory.join(if fail_recorded_write {
        "blocked.png"
    } else {
        "product.png"
    });
    if fail_recorded_write {
        fs::create_dir(&recorded).unwrap();
    }
    let mut app = App::new();
    app.add_plugins((bevy::app::TaskPoolPlugin::default(), AssetPlugin::default()));
    app.init_resource::<Assets<Mesh>>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_resource(Time::<bevy::time::Real>::default())
        .insert_resource(super::tests::ux())
        .insert_resource(creature_grounding::RenderedTerrainSurface::from_meshes(
            std::iter::empty(),
            1.0,
        ))
        .add_message::<AppExit>()
        .insert_resource(scene())
        .insert_resource(Fvr03ProductionVoxelScreenshotResource {
            frame: 0,
            capture_after_frame: 0,
            measurement_sample_frames: u32::MAX,
            measurement_start_frame: 0,
            measurement_started_at: None,
            measurement_written: false,
            path: recorded.clone(),
            product_screenshot_requested: false,
            fvr05_capture_index: 0,
            fvr05_next_capture_frame: 0,
            fvr05_sequence_complete: false,
            developer_overlay: true,
            failure_started: None,
        });
    install(&mut app);
    // Exactly the production recorder system, in its production schedule.
    app.add_systems(Update, request_fvr03_recorded_screenshot);
    install_ready_presentation(&mut app);
    app.world_mut()
        .spawn(bevy::prelude::AnimationPlayer::default());
    app.world_mut().spawn((Window::default(), PrimaryWindow));

    // Install Bevy's actual extraction, but never run RenderStartup/Render or
    // create an adapter. Keep the sub-app detached while advancing main frames.
    let mut render = SubApp::new();
    render.init_schedule(ExtractSchedule);
    app.insert_sub_app(RenderApp, render);
    app.add_plugins(ScreenshotPlugin);
    let mut render = app.remove_sub_app(RenderApp).unwrap();
    render.world_mut().init_resource::<MainWorld>();

    app.world_mut()
        .resource_mut::<Time<bevy::time::Real>>()
        .advance_by(Duration::from_secs(3));
    app.update();
    let first = extract(&mut app, &mut render);
    assert!(
        app.world()
            .resource::<Fvr03ProductionVoxelScreenshotResource>()
            .product_screenshot_requested
    );
    assert!(player.join("frame-0000.json").is_file());
    // Leave the first image in flight across subsequent frames.
    app.update();
    assert_eq!(pending(&mut app), 0);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F12);
    app.update();
    let second = extract(&mut app, &mut render);
    assert_ne!(first, second, "later frames must have their own request");
    let manual = fs::read_dir(&player)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("view-")
                && p.extension().is_some_and(|e| e == "json")
        })
        .unwrap();
    let manual: serde_json::Value = serde_json::from_slice(&fs::read(manual).unwrap()).unwrap();
    assert!(manual["last_player_action"]
        .as_str()
        .unwrap()
        .contains("requested GPU tab"));
    assert_eq!(
        app.world()
            .resource::<Fvr03ProductionVoxelScreenshotResource>()
            .fvr05_capture_index,
        1
    );
    let manual_png = fs::read_dir(&player)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("view-")
        })
        .unwrap()
        .with_extension("png");
    let sequence_png = fvr05_screenshot_path(&recorded, "fvr05_gpu_panel");

    // Inject CPU images through the actual asynchronous delivery event, newest
    // first. This verifies fan-out and frame isolation, not GPU/image quality.
    deliver(&mut app, second, [17, 83, 201, 255]);
    assert_pixels(&manual_png, [17, 83, 201]);
    assert_pixels(&sequence_png, [17, 83, 201]);
    assert!(!player.join("frame-0000.png").exists());
    deliver(&mut app, first, [203, 71, 19, 255]);
    assert_pixels(&player.join("frame-0000.png"), [203, 71, 19]);
    if fail_recorded_write {
        assert!(recorded.is_dir());
        assert!(app
            .world()
            .resource::<Fvr03ProductionVoxelScreenshotResource>()
            .failure_started
            .is_some());
    } else {
        assert_pixels(&recorded, [203, 71, 19]);
        assert!(app
            .world()
            .resource::<Fvr03ProductionVoxelScreenshotResource>()
            .failure_started
            .is_none());
    }
    println!("write_failure={fail_recorded_write}: automatic+product and F12+sequence delivered to both observers; older frame completed last");
}

fn pending(app: &mut App) -> usize {
    app.world_mut()
        .query_filtered::<Entity, (With<Screenshot>, Without<Capturing>)>()
        .iter(app.world())
        .count()
}

fn extract(app: &mut App, render: &mut SubApp) -> Entity {
    let requested = app
        .world_mut()
        .query_filtered::<Entity, (With<Screenshot>, Without<Capturing>)>()
        .iter(app.world())
        .collect::<Vec<_>>();
    let before = requested.len();
    std::mem::swap(
        &mut **render.world_mut().resource_mut::<MainWorld>(),
        app.world_mut(),
    );
    render.world_mut().run_schedule(ExtractSchedule);
    std::mem::swap(
        &mut **render.world_mut().resource_mut::<MainWorld>(),
        app.world_mut(),
    );
    let capturing = app
        .world_mut()
        .query_filtered::<Entity, With<Capturing>>()
        .iter(app.world())
        .collect::<Vec<_>>();
    println!(
        "before Bevy extraction: {before} pending primary-window requests; after: {} capturing",
        capturing.len()
    );
    assert_eq!(
        before, 1,
        "competing same-target requests would be despawned by Bevy extraction"
    );
    assert!(capturing.contains(&requested[0]));
    requested[0]
}

fn deliver(app: &mut App, entity: Entity, pixel: [u8; 4]) {
    let image = Image::new_fill(
        bevy::render::render_resource::Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        &pixel,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD,
    );
    app.world_mut()
        .trigger(ScreenshotCaptured { entity, image });
    app.world_mut().entity_mut(entity).insert(Captured);
}

fn assert_pixels(path: &Path, expected: [u8; 3]) {
    let image = image::open(path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()))
        .to_rgb8();
    assert_eq!(image.dimensions(), (2, 2));
    assert!(
        image.pixels().all(|pixel| pixel.0 == expected),
        "{} has wrong frame pixels",
        path.display()
    );
}

fn install_ready_presentation(app: &mut App) {
    use alife_core::{BrainCapacityClass, CreatureGenome, FoundationGeneticIdentity};
    use alife_world::{HeadlessScenarioBuilder, WorldOrganismRecord};
    let id = OrganismId(1);
    let mut world = HeadlessScenarioBuilder::new(21)
        .agent("agent", id, Vec3f::ZERO)
        .build()
        .unwrap();
    let stable_id = world.entity_id("agent").unwrap();
    let genome = CreatureGenome::early_mammal_founder(
        21,
        FoundationGeneticIdentity::new(10, 1, 7, BrainCapacityClass::N512_ID).unwrap(),
    )
    .unwrap();
    let phenotype = genome.express().unwrap();
    world
        .register_organism_record(
            WorldOrganismRecord::newborn(id, stable_id, genome, phenotype, Tick::ZERO).unwrap(),
        )
        .unwrap();
    world.advance_tick();
    let mut frames = LiveBrainPresentationFrameResource::from_authoritative_world(&world).unwrap();
    frames
        .try_publish_successful_tick_with_cognitive(
            vec![LiveBrainTickSummary {
                schema: crate::G03_LIVE_BRAIN_LOOP_SCHEMA,
                schema_version: crate::G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
                organism_id: id,
                tick_before: Tick::ZERO,
                tick_after: world.tick(),
                world_tick_before: Tick::ZERO,
                world_tick_after: world.tick(),
                status: alife_core::BrainTickStatus::Normal,
                selected_action_kind: None,
                selected_action_id: None,
                target_entity: None,
                patch_sealed: true,
                patch_sequence_id: Some(1),
                patch_success: Some(true),
                physical_contact: None,
                action_failure: None,
                motor_execution: None,
                sealed_patch_count: 1,
                packed_record_count: 1,
                memory_updates: 0,
                topology_updates: 0,
                learning_updates: 0,
                invalid_or_rejected_action_count: 0,
                last_diagnostic: None,
                causal_stages: vec![],
            }],
            vec![crate::LiveCognitivePresentationSnapshot {
                organism_id: id,
                brain_class_id: None,
                brain_neuron_count: None,
                fast_memory_count: None,
                lifetime_memory_count: None,
                concept_count: None,
                unresolved_gap_count: None,
                learning_active: None,
                sleep_phase_raw: None,
                consolidation_state_raw: None,
                last_consolidated_tick: None,
                topology_update_count: None,
            }],
            &world,
        )
        .unwrap();
    app.insert_resource(frames);
    app.insert_resource(Fvr03ProductionVoxelSelectionResource {
        hovered: None,
        selected: Some(StableVoxelObjectRef {
            kind: StableVoxelRefKind::Creature,
            stable_id: Some(stable_id),
            tile: Some(VoxelTileCoord::new(0, 0)),
            chunk: VoxelChunkCoord::for_tile(16, VoxelTileCoord::new(0, 0)),
        }),
    });
}

fn scene() -> Fvr03ProductionVoxelSceneResource {
    Fvr03ProductionVoxelSceneResource {
        schema: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA,
        schema_version: FVR03_PRODUCTION_VOXEL_RENDERER_SCHEMA_VERSION,
        snapshot_schema: "CPU screenshot fixture".into(),
        profile_id: ProductionFrontendProfileId::MinimumSettings30x30,
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
