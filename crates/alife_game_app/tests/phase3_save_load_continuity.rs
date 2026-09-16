#![cfg(all(feature = "gpu-tests", feature = "production-voxel-frontend"))]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use alife_core::{Blake3Digest, BrainScaleTier, OrganismId, PolicyBackend, Tick};
use alife_game_app::{
    bevy_shell::{
        build_production_voxel_frontend_app_shell_with_runtime, LiveBrainPresentationFrameResource,
    },
    create_canonical_new_game_runtime, default_environment_manifest_path,
    production_archive_birth_manifest_for_test, CanonicalNewGameLaunchRequest,
    Fvr03ProductionVoxelSelectionResource, Fvr05ProductionUxStateResource,
    ProductionFrontendProfileId, ProductionVoxelLaunchConfig,
};
use alife_world::{
    AssetManifest, PortableSaveFile, RuntimeConfig, StableVoxelObjectRef, StableVoxelRefKind,
    VoxelChunkCoord, VoxelTileCoord, WorldObjectKind,
};
use bevy::{
    input::{keyboard::KeyCode, ButtonInput},
    prelude::{App, Update},
    time::TimeUpdateStrategy,
};

static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

fn isolated_root(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time follows the Unix epoch")
        .as_nanos();
    let ordinal = NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "alife-phase3-save-load-{label}-{}-{nonce}-{ordinal}",
        std::process::id()
    ))
}

fn press_key(app: &mut App, key: KeyCode) {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(key);
    app.world_mut().run_schedule(Update);
    let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keyboard.release(key);
    keyboard.clear();
}

fn run_render_updates(app: &mut App, count: usize) {
    for _ in 0..count {
        app.world_mut().run_schedule(Update);
    }
}

fn set_runtime_save_path(app: &mut App, path: &Path) {
    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .runtime_save_path = path.display().to_string();
}

fn save_runtime(app: &mut App, path: &Path, asset_root: &Path) -> PortableSaveFile {
    set_runtime_save_path(app, path);
    let tick_before = app
        .world()
        .resource::<LiveBrainPresentationFrameResource>()
        .current
        .authoritative_world_tick;
    press_key(app, KeyCode::KeyS);
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        let ux = app.world().resource::<Fvr05ProductionUxStateResource>();
        if ux.last_action
            == format!(
                "Saved exact GPU checkpoint asynchronously: {}",
                path.display()
            )
        {
            break;
        }
        assert!(
            ux.last_error.is_none(),
            "manual save failed: {:?}",
            ux.last_error
        );
        assert!(
            std::time::Instant::now() < deadline,
            "paused Save did not finish publishing"
        );
        run_render_updates(app, 1);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        app.world()
            .resource::<LiveBrainPresentationFrameResource>()
            .current
            .authoritative_world_tick,
        tick_before,
        "publishing a paused save must not advance simulation time"
    );
    let save = PortableSaveFile::from_json_file(path)
        .expect("S writes an exact production GPU checkpoint");
    save.validate_with_asset_root(asset_root)
        .expect("saved runtime validates every required persisted subsystem");
    save
}

fn load_runtime(app: &mut App, path: &Path) {
    set_runtime_save_path(app, path);
    press_key(app, KeyCode::KeyL);
}

fn tamper_required_embodiment(source: &Path, target: &Path) {
    // Serialize a direct save so a retained authority pointer cannot redirect
    // this negative test to the original, undamaged checkpoint generation.
    let mut value: serde_json::Value = serde_json::to_value(
        PortableSaveFile::from_json_file(source).expect("read exact baseline save"),
    )
    .expect("baseline save is JSON");
    let first_record = value
        .pointer_mut("/world/organism_records/0")
        .and_then(serde_json::Value::as_object_mut)
        .expect("baseline save contains a canonical organism record");
    assert!(first_record.remove("embodiment").is_some());
    fs::write(
        target,
        serde_json::to_vec_pretty(&value).expect("serialize tampered save"),
    )
    .expect("write isolated tampered save");
}

#[test]
fn production_save_load_restores_every_authority_and_rejects_missing_embodiment_atomically() {
    let root = isolated_root("exact-continuity");
    let asset_root = root.join("assets");
    fs::create_dir_all(&asset_root).expect("create isolated asset root");
    let seed = 250_827;
    let mut config = RuntimeConfig::deterministic_default(seed, BrainScaleTier::Nano512);
    config.features.gpu_backend_enabled = true;
    let created = create_canonical_new_game_runtime(CanonicalNewGameLaunchRequest {
        world_seed: seed,
        population: 4,
        save_path: root.join("canonical-new-game.json"),
        asset_root: asset_root.clone(),
        config,
        assets: AssetManifest::empty(),
    })
    .expect("canonical New Game runtime");

    let config_path = root.join("runtime-config.json");
    let asset_manifest_path = root.join("asset-manifest.json");
    fs::write(
        &config_path,
        serde_json::to_vec_pretty(&created.exact_save.config).expect("serialize runtime config"),
    )
    .expect("write runtime config");
    fs::write(
        &asset_manifest_path,
        serde_json::to_vec_pretty(&created.exact_save.assets).expect("serialize asset manifest"),
    )
    .expect("write asset manifest");

    let mut launch = ProductionVoxelLaunchConfig::from_manifest(
        default_environment_manifest_path(),
        Some("production-voxel"),
        ProductionFrontendProfileId::MinimumSettings30x30,
    )
    .expect("production launch config");
    launch.app_launch.fixture_root = root.clone();
    launch.app_launch.config_path = config_path;
    launch.app_launch.asset_manifest_path = asset_manifest_path;
    launch.app_launch.save_path = created.save_path.clone();
    launch.app_launch.asset_root = created.asset_root.clone();
    launch.app_launch.brain_policy = PolicyBackend::NeuralClosedLoopGpu;
    launch.population = Some(4);
    launch.require_gpu = true;
    launch.graphics_backend = "existing".to_string();
    launch.dry_run = true;
    launch.ui_settings_path = Some(root.join("player-ui.json"));

    let (mut app, _) =
        build_production_voxel_frontend_app_shell_with_runtime(&launch, created.runtime)
            .expect("production graphical/runtime app");
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        34,
    )));

    let reached_meaningful_state = (0..384).any(|_| {
        app.update();
        // Give asynchronous GPU readback and checkpoint workers wall-clock time.
        std::thread::sleep(Duration::from_millis(10));
        let frame = &app
            .world()
            .resource::<LiveBrainPresentationFrameResource>()
            .current;
        frame.authoritative_world_tick > Tick::ZERO
            && frame
                .tick_summaries
                .iter()
                .any(|summary| summary.patch_sealed)
    });
    assert!(
        reached_meaningful_state,
        "canonical New Game must reach an ordinary sealed production GPU tick"
    );

    press_key(&mut app, KeyCode::KeyP);
    assert!(
        app.world()
            .resource::<Fvr05ProductionUxStateResource>()
            .settings
            .paused,
        "manual save starts from a bounded paused state"
    );

    assert!(
        alife_game_app::Fvr04ProductionCreatureRendererSettings::for_profile(
            ProductionFrontendProfileId::MinSpecComfort1080p,
            4,
        )
        .max_visible_creatures
            > 4,
        "the display must have room for births"
    );
    let before_action_path = root.join("player-before-resource-action.json");
    // Exercise the actual production input/schedule without a native window.
    let window_entity = app
        .world_mut()
        .spawn((bevy::window::Window::default(), bevy::window::PrimaryWindow))
        .id();
    press_key(&mut app, KeyCode::Home);
    assert!(
        app.world()
            .resource::<alife_game_app::Fvr04ProductionCreatureFollowResource>()
            .enabled
    );
    let camera_entity = app.world_mut().query_filtered::<bevy::prelude::Entity, bevy::prelude::With<alife_game_app::Fvr03ProductionVoxelCamera>>()
        .single(app.world()).unwrap();
    let snapped = *app
        .world()
        .get::<bevy::prelude::Transform>(camera_entity)
        .unwrap();
    press_key(&mut app, KeyCode::ArrowRight);
    assert!(
        !app.world()
            .resource::<alife_game_app::Fvr04ProductionCreatureFollowResource>()
            .enabled
    );
    assert_ne!(
        app.world()
            .get::<bevy::prelude::Transform>(camera_entity)
            .unwrap()
            .translation,
        snapped.translation
    );
    press_key(&mut app, KeyCode::Home);
    assert_eq!(
        *app.world()
            .get::<bevy::prelude::Transform>(camera_entity)
            .unwrap(),
        snapped
    );
    {
        let mut window = app
            .world_mut()
            .get_mut::<bevy::window::Window>(window_entity)
            .unwrap();
        let edge = bevy::math::Vec2::new(window.width() - 1.0, window.height() / 2.0);
        window.set_cursor_position(Some(edge));
    }
    run_render_updates(&mut app, 1);
    assert!(
        !app.world()
            .resource::<alife_game_app::Fvr04ProductionCreatureFollowResource>()
            .enabled
    );
    assert_ne!(
        app.world()
            .get::<bevy::prelude::Transform>(camera_entity)
            .unwrap()
            .translation,
        snapped.translation
    );
    app.world_mut()
        .get_mut::<bevy::window::Window>(window_entity)
        .unwrap()
        .set_cursor_position(None);
    let mut mesh_counts = Vec::new();
    for distance in [200.0, 400.0, 600.0, 0.0] {
        let mut moved = snapped;
        moved.translation.x += distance;
        *app.world_mut()
            .get_mut::<bevy::prelude::Transform>(camera_entity)
            .unwrap() = moved;
        run_render_updates(&mut app, 1);
        let forward = moved.rotation * bevy::math::Vec3::NEG_Z;
        let focus = moved.translation + forward * (-moved.translation.y / forward.y);
        let scene = app
            .world()
            .resource::<alife_game_app::Fvr03ProductionVoxelSceneResource>();
        assert!(scene.contains_tile(VoxelTileCoord::new(
            focus.x.floor() as i32,
            focus.z.floor() as i32
        )));
        assert_eq!(scene.creature_render_count, 4);
        mesh_counts.push(
            app.world()
                .resource::<bevy::prelude::Assets<bevy::prelude::Mesh>>()
                .len(),
        );
    }
    assert!(
        mesh_counts.iter().max().unwrap() - mesh_counts.iter().min().unwrap() < 100,
        "terrain mesh assets must remain bounded: {mesh_counts:?}"
    );
    press_key(&mut app, KeyCode::Home);
    app.world_mut().despawn(window_entity);
    println!("PASS arrows, window-edge input, Home snap, follow release, terrain streaming: {mesh_counts:?}");
    let before_action = save_runtime(&mut app, &before_action_path, &asset_root);
    let selected_tile = VoxelTileCoord::new(2, 2);
    let existing_food_id = before_action
        .world
        .objects
        .iter()
        .find(|object| object.kind == WorldObjectKind::Food)
        .expect("canonical New Game contains one world-owned food resource")
        .id;
    app.world_mut()
        .resource_mut::<Fvr03ProductionVoxelSelectionResource>()
        .selected = Some(StableVoxelObjectRef {
        kind: StableVoxelRefKind::Resource,
        stable_id: Some(existing_food_id),
        chunk: VoxelChunkCoord::for_tile(16, selected_tile),
        tile: Some(selected_tile),
    });
    press_key(&mut app, KeyCode::KeyE);
    {
        let ux = app.world().resource::<Fvr05ProductionUxStateResource>();
        assert_eq!(
            ux.last_action,
            "Food placement rejected; world left unchanged"
        );
        assert!(ux.last_error.is_some());
    }
    let after_non_tile_path = root.join("player-after-non-tile-selection.json");
    let after_non_tile = save_runtime(&mut app, &after_non_tile_path, &asset_root);
    assert_eq!(
        after_non_tile, before_action,
        "a non-tile selection must not mutate canonical world authority"
    );

    app.world_mut()
        .resource_mut::<Fvr03ProductionVoxelSelectionResource>()
        .selected = Some(StableVoxelObjectRef {
        kind: StableVoxelRefKind::Tile,
        stable_id: None,
        chunk: VoxelChunkCoord::for_tile(16, selected_tile),
        tile: Some(selected_tile),
    });
    press_key(&mut app, KeyCode::KeyE);
    {
        let ux = app.world().resource::<Fvr05ProductionUxStateResource>();
        assert!(ux.last_error.is_none());
        assert!(ux.last_action.starts_with("Placed canonical food "));
    }

    let baseline_path = root.join("player-baseline.json");
    let baseline = save_runtime(&mut app, &baseline_path, &asset_root);
    assert_eq!(
        baseline.world.objects.len(),
        before_action.world.objects.len() + 1,
        "the real production E handler must add one canonical world object"
    );
    let placed_food = baseline
        .world
        .objects
        .iter()
        .find(|object| object.label.starts_with("player-food-"))
        .expect("the player action persists a canonical food object");
    assert_eq!(
        placed_food.label,
        format!(
            "player-food-t{}-x{:08x}-z{:08x}",
            before_action.world.tick.raw(),
            2.5_f32.to_bits(),
            2.5_f32.to_bits()
        ),
        "the same canonical tick and terrain position derive the same stable label"
    );
    assert_eq!(placed_food.kind, WorldObjectKind::Food);
    assert_eq!(placed_food.position, alife_core::Vec3f::new(2.5, 0.0, 2.5));
    assert_eq!(placed_food.radius, 0.5);
    assert!(placed_food.nutrition > 0.0);
    assert_eq!(placed_food.hazard_pain, 0.0);

    press_key(&mut app, KeyCode::KeyE);
    {
        let ux = app.world().resource::<Fvr05ProductionUxStateResource>();
        assert_eq!(
            ux.last_action,
            "Food placement rejected; world left unchanged"
        );
        assert!(ux.last_error.is_some());
    }
    let after_rejected_duplicate_path = root.join("player-after-rejected-duplicate.json");
    let after_rejected_duplicate =
        save_runtime(&mut app, &after_rejected_duplicate_path, &asset_root);
    assert_eq!(
        after_rejected_duplicate, baseline,
        "a repeated same-tick placement must fail atomically"
    );
    assert!(baseline.world.tick > Tick::ZERO);
    assert!(baseline.creatures.iter().all(|creature| {
        creature.gpu_brain.as_ref().is_some_and(|brain| {
            brain.checkpoint_tick == baseline.world.tick
                && brain.exact_cognitive_state.is_some()
                && brain.learning_transaction_generation > 0
        })
    }));
    let baseline_world = baseline
        .restore_headless_world()
        .expect("baseline restores every canonical world authority");
    let expected_birth_manifests = baseline_world
        .organism_registry()
        .iter()
        .map(|record| {
            (
                record.organism_id().raw(),
                record
                    .archive()
                    .birth_manifest_digest()
                    .expect("canonical founder retains birth-manifest identity"),
            )
        })
        .collect::<BTreeMap<_, Blake3Digest>>();

    press_key(&mut app, KeyCode::KeyN);
    press_key(&mut app, KeyCode::KeyN);
    let mutated_path = root.join("player-mutated.json");
    let mutated = save_runtime(&mut app, &mutated_path, &asset_root);
    assert!(mutated.world.tick > baseline.world.tick);
    assert_ne!(mutated.world, baseline.world);

    load_runtime(&mut app, &baseline_path);
    let restored_birth_manifests = expected_birth_manifests
        .keys()
        .copied()
        .map(|organism_id_raw| {
            (
                organism_id_raw,
                production_archive_birth_manifest_for_test(&mut app, OrganismId(organism_id_raw))
                    .expect("real L retains live birth-manifest authority"),
            )
        })
        .collect::<BTreeMap<_, Blake3Digest>>();
    assert_eq!(
        restored_birth_manifests, expected_birth_manifests,
        "real L must restore live lineage identity, not only persisted world records"
    );
    let restored_path = root.join("player-restored.json");
    let restored = save_runtime(&mut app, &restored_path, &asset_root);
    assert_eq!(
        restored, baseline,
        "L must restore the complete authoritative record, not selected presentation fields"
    );

    press_key(&mut app, KeyCode::KeyN);
    run_render_updates(&mut app, 13);
    let before_rejected_path = root.join("player-before-rejected-load.json");
    let before_rejected = save_runtime(&mut app, &before_rejected_path, &asset_root);
    assert_eq!(
        before_rejected.world.tick,
        Tick::new(baseline.world.tick.raw() + 1),
        "the ordinary production GPU runtime must continue after load"
    );

    let tampered_path = root.join("player-tampered-missing-embodiment.json");
    tamper_required_embodiment(&baseline_path, &tampered_path);
    load_runtime(&mut app, &tampered_path);
    {
        let ux = app.world().resource::<Fvr05ProductionUxStateResource>();
        assert_eq!(ux.last_action, "Load failed; current world left unchanged");
        assert!(ux.last_error.is_some());
    }
    let after_rejected_path = root.join("player-after-rejected-load.json");
    let after_rejected = save_runtime(&mut app, &after_rejected_path, &asset_root);
    assert_eq!(
        after_rejected, before_rejected,
        "rejected load must leave every live runtime authority unchanged"
    );

    drop(app);
    fs::remove_dir_all(root).expect("remove isolated save/load artifacts");
}
