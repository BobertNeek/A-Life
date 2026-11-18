//! Sow two distinct plots, read their real stages, and return to pick only the
//! older crop. Drives the normal production Hand and menu; no fixture world edits.
#[cfg(not(feature = "production-voxel-frontend"))]
fn main() {
    eprintln!("Enable production-voxel-frontend.");
}
#[cfg(feature = "production-voxel-frontend")]
fn main() {
    use alife_game_app::{
        bevy_shell::{
            build_production_voxel_frontend_app_shell, LiveBrainPresentationFrameResource,
        },
        default_environment_manifest_path, Fvr03ProductionVoxelSelectionResource,
        Fvr05ProductionUxStateResource, ProductionRunMode, ProductionVoxelLaunchConfig,
        ProductionWorldSource,
    };
    use alife_world::{VoxelTileCoord, WorldObjectKind};
    use bevy::{
        a11y::{AccessibilityNode, ActionRequest},
        app::AppExit,
        input::InputSystems,
        prelude::*,
        render::view::screenshot::{save_to_disk, Screenshot},
    };
    use std::{
        collections::BTreeSet,
        io::Write,
        path::PathBuf,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::{SystemTime, UNIX_EPOCH},
    };
    let base = PathBuf::from("target/artifacts/garden-survey");
    let restore = std::env::args().nth(1).map(|s| {
        s.parse::<u64>()
            .expect("a numeric prior run, never a player-save path")
    });
    let stamp = restore.unwrap_or_else(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    });
    std::fs::create_dir_all(&base).unwrap();
    let base = std::fs::canonicalize(base).unwrap();
    let out = base.join(stamp.to_string());
    if restore.is_none() {
        std::fs::create_dir_all(&out).unwrap();
    }
    let out = std::fs::canonicalize(out).unwrap();
    assert!(
        out.starts_with(&base),
        "only isolated playtest saves may be loaded"
    );
    let mut launch =
        ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
            .unwrap();
    if restore.is_some() {
        launch.world_source = ProductionWorldSource::LoadExisting;
        let saved = std::fs::canonicalize(out.join("phase3-new-game-1643.json")).unwrap();
        assert!(saved.starts_with(&out));
        launch.app_launch.save_path = saved;
        launch.population = None;
    } else {
        launch.world_source = ProductionWorldSource::NewGame { seed: 1643 };
        launch.population = Some(1);
    }
    launch.run_mode = Some(ProductionRunMode::OneX);
    launch.require_gpu = true;
    launch.resolution = (960, 540);
    launch.smoke_seconds = Some(140);
    launch.ui_settings_path = Some(out.join("settings.json"));
    let (mut app, _) = build_production_voxel_frontend_app_shell(&launch).unwrap();
    let mut log = std::fs::File::create(out.join(if restore.is_some() {
        "reopen.tsv"
    } else {
        "walk.tsv"
    }))
    .unwrap();
    let (mut phase, mut next, mut release) = (0u8, 6., None);
    let mut first = None::<VoxelTileCoord>;
    let mut second = None;
    let mut paused_tick = 0;
    let mut save_tick = 0;
    let mut foods = BTreeSet::new();
    let passed = Arc::new(AtomicBool::new(false));
    let observed = passed.clone();
    app.add_systems(
        PreUpdate,
        (move |time: Res<Time<Real>>,
               frame: Res<LiveBrainPresentationFrameResource>,
               ux: Res<Fvr05ProductionUxStateResource>,
               selection: Res<Fvr03ProductionVoxelSelectionResource>,
               nodes: Query<(Entity, &AccessibilityNode)>,
               mut windows: Query<&mut Window, With<bevy::window::PrimaryWindow>>,
               charts: Query<(
            &Name,
            &ImageNode,
            &ComputedNode,
            &bevy::ui::UiGlobalTransform,
        )>,
               textures: Res<Assets<Image>>,
               mut mouse: ResMut<ButtonInput<MouseButton>>,
               mut keys: ResMut<ButtonInput<KeyCode>>,
               mut requests: MessageWriter<ActionRequest>,
               mut cmds: Commands,
               mut exit: MessageWriter<AppExit>| {
            if let Some(k) = release.take() {
                keys.release(k);
            }
            let now = time.elapsed_secs();
            if now < next {
                return;
            }
            next = now + 0.4;
            let tick = frame.current.authoritative_world_tick.raw();
            let current = selection.selected.and_then(|r| r.tile);
            let label = nodes
                .iter()
                .filter(|(_, n)| !n.is_hidden())
                .filter_map(|(_, n)| n.label())
                .find(|s| s.starts_with("Bed ") || *s == "No planted beds")
                .unwrap_or("");
            writeln!(log, "{phase}\t{now:.1}\t{tick}\t{current:?}\t{label}").unwrap();
            log.flush().unwrap();
            let mut click = |label: &str| {
                let Some((e, _)) = nodes
                    .iter()
                    .find(|(_, n)| !n.is_hidden() && !n.is_disabled() && n.label() == Some(label))
                else {
                    return false;
                };
                requests.write(ActionRequest(accesskit::ActionRequest {
                    action: accesskit::Action::Click,
                    target: accesskit::NodeId(e.to_bits()),
                    data: None,
                }));
                true
            };
            let mut shot = |name: &str| {
                cmds.spawn(Screenshot::primary_window())
                    .observe(save_to_disk(out.join(format!("{name}.png"))));
            };
            let mut key = |k| {
                keys.press(k);
                release = Some(k);
            };
            match phase {
                0 if click("Garden") => {
                    phase = if restore.is_some() { 30 } else { 1 };
                }
                1 if label == "No planted beds" => {
                    shot("empty");
                    assert!(ux.settings.paused);
                    phase = 16;
                    next = now + 0.8;
                }
                16 if click("Find soil for another planting") => {
                    phase = 2;
                }
                2 if click("Plant a lantern pear here") => {
                    first = current;
                    assert!(first.is_some());
                    phase = 3;
                }
                3 if nodes.iter().any(|(_, n)| {
                    !n.is_hidden() && n.label() == Some("Pick a ripe lantern pear")
                }) =>
                {
                    key(KeyCode::F6);
                    phase = 4;
                }
                4 if label.ends_with("Ripe pears") => {
                    assert!(click("Find soil for another planting"));
                    phase = 5;
                }
                5 if click("Plant a lantern pear here") => {
                    second = current;
                    assert!(second.is_some() && second != first);
                    phase = 6;
                    next = now + 1.;
                }
                6 => {
                    key(KeyCode::F6);
                    phase = 7;
                }
                7 if label.contains("of 2") && label.ends_with("Seed") => {
                    shot("seed-survey");
                    paused_tick = tick;
                    phase = 17;
                    next = now + 0.8;
                }
                17 if click("Previous planted bed") => {
                    phase = 8;
                }
                8 if label.contains("of 2") && label.ends_with("Ripe pears") => {
                    assert!(ux.settings.paused);
                    assert_eq!(tick, paused_tick);
                    shot("ripe-survey");
                    phase = 18;
                    next = now + 0.8;
                }
                18 if click("Next planted bed") => {
                    phase = 40;
                    next = now + 0.8;
                }
                40 if label.ends_with("Seed") => {
                    // Locate the pictured ripe fruit itself, not an out-of-band bed index.
                    let (_, chart, node, pose) = charts
                        .iter()
                        .find(|(name, _, _, _)| name.as_str() == "Orchard terrain survey")
                        .unwrap();
                    let map = textures
                        .get(&chart.image)
                        .unwrap()
                        .clone()
                        .try_into_dynamic()
                        .unwrap()
                        .to_rgba8();
                    let (x, y, _) = map
                        .enumerate_pixels()
                        .find(|(_, _, p)| p.0 == [255, 162, 77, 255])
                        .expect("ripe fruit mark");
                    let uv = Vec2::new(
                        x as f32 / map.width() as f32,
                        y as f32 / map.height() as f32,
                    );
                    let pixel = pose
                        .affine()
                        .transform_point2((uv - Vec2::splat(0.5)) * node.size());
                    let mut window = windows.single_mut().unwrap();
                    let logical = pixel / window.scale_factor();
                    writeln!(
                        log,
                        "pointer aim {logical:?}; map pixel {x},{y}; viewport {:?}",
                        node.size()
                    )
                    .unwrap();
                    window.set_cursor_position(Some(logical));
                    phase = 41;
                }
                41 => {
                    writeln!(
                        log,
                        "pointer pressed at {:?}",
                        windows.single().unwrap().cursor_position()
                    )
                    .unwrap();
                    mouse.press(MouseButton::Left);
                    phase = 42;
                }
                42 => {
                    mouse.release(MouseButton::Left);
                    phase = 43;
                }
                43 if label.ends_with("Ripe pears") => {
                    assert_eq!(tick, paused_tick);
                    phase = 9;
                }
                9 if click("Visit the marked bed") => {
                    phase = 10;
                }
                10 if current == first
                    && tick > paused_tick
                    && click("Pick a ripe lantern pear") =>
                {
                    foods = frame
                        .current
                        .objects()
                        .filter(|o| o.kind == WorldObjectKind::Food)
                        .map(|o| o.id.raw())
                        .collect();
                    phase = 11;
                    next = now + 1.;
                }
                11 => {
                    let new: Vec<_> = frame
                        .current
                        .objects()
                        .filter(|o| o.kind == WorldObjectKind::Food && !foods.contains(&o.id.raw()))
                        .map(|o| o.id.raw())
                        .collect();
                    assert_eq!(new.len(), 1);
                    writeln!(log, "actual harvest {:?} at {first:?}", new).unwrap();
                    shot("harvest");
                    phase = 19;
                    next = now + 0.8;
                }
                19 => {
                    key(KeyCode::F6);
                    phase = 12;
                }
                12 if click("Next planted bed") => {
                    phase = 13;
                }
                13 if click("Visit the marked bed") => {
                    phase = 14;
                    next = now + 1.;
                }
                14 if current == second => {
                    shot("returned");
                    save_tick = tick;
                    key(KeyCode::KeyS);
                    phase = 15;
                    next = now + 5.;
                }
                15 if ux.gpu_runtime_state.last_safe_checkpoint.world_tick.raw() >= save_tick => {
                    // A seed save already exists. The async publication, not the mere
                    // presence of that file or a staged generation, completes a save.
                    let save = alife_world::PortableSaveFile::from_json_file(
                        out.join("phase3-new-game-1643.json"),
                    )
                    .unwrap();
                    assert!(save.world.tick.raw() >= save_tick);
                    assert_eq!(save.world.ecology.orchard_beds.len(), 2);
                    assert!(save
                        .world
                        .ecology
                        .orchard_beds
                        .iter()
                        .any(|b| Some(b.tile) == first && b.last_fruit.is_some()));
                    writeln!(log, "published save at {}", save.world.tick.raw()).unwrap();
                    observed.store(true, Ordering::Relaxed);
                    exit.write(AppExit::Success);
                    phase = 99;
                }
                30 if label.contains("of 2") => {
                    shot("reopened");
                    phase = 33;
                    next = now + 0.8;
                }
                33 if click("Next planted bed") => {
                    phase = 31;
                    next = now + 0.5;
                }
                31 if label.contains("of 2") && click("Visit the marked bed") => {
                    phase = 32;
                    next = now + 2.;
                }
                32 if nodes.iter().any(|(_, n)| {
                    !n.is_hidden()
                        && matches!(
                            n.label(),
                            Some(
                                "Pick a ripe lantern pear"
                                    | "Water these roots"
                                    | "Watch this plant grow"
                            )
                        )
                }) =>
                {
                    observed.store(true, Ordering::Relaxed);
                    exit.write(AppExit::Success);
                    phase = 99;
                }
                _ => {}
            }
        })
        .after(InputSystems),
    );
    assert!(
        app.run().is_success() && passed.load(Ordering::Relaxed),
        "the garden survey route did not complete"
    );
}
