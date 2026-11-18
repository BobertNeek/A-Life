//! Long real-renderer playthrough in a new, disposable world. Only sends normal tool
//! activations and keyboard soil selection, without moving a desktop pointer.
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
        ProductionRunMode, ProductionVoxelLaunchConfig, ProductionWorldSource,
    };
    use alife_world::WorldObjectKind;
    use bevy::{
        a11y::{AccessibilityNode, ActionRequest},
        app::AppExit,
        input::InputSystems,
        prelude::*,
        render::view::screenshot::{save_to_disk, Screenshot},
    };
    use std::{io::Write, path::PathBuf, time::SystemTime};

    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let out = PathBuf::from("target/artifacts/orchard-watch").join(ts.to_string());
    std::fs::create_dir_all(&out).unwrap();
    let out = std::fs::canonicalize(&out).unwrap();
    let mut log = std::fs::File::create(out.join("watch.tsv")).unwrap();
    let mut launch =
        ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
            .unwrap();
    launch.world_source = ProductionWorldSource::NewGame { seed: 1643 };
    launch.population = Some(1);
    launch.run_mode = Some(ProductionRunMode::OneX);
    launch.require_gpu = true;
    launch.smoke_seconds = Some(420);
    launch.resolution = (960, 540);
    launch.ui_settings_path = Some(out.join("settings.json"));
    let (mut app, _) = build_production_voxel_frontend_app_shell(&launch).unwrap();
    println!("Orchard playthrough: {}", out.display());
    let mut phase = 0u8;
    let mut next = 7.;

    let mut last_tick = 0;
    let mut known = std::collections::BTreeSet::new();
    let mut issued = Vec::new();
    let mut release: Option<KeyCode> = None;
    let mut completion = false;
    let passed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = passed.clone();
    app.add_systems(
        PreUpdate,
        (move |time: Res<Time<Real>>,
               frame: Res<LiveBrainPresentationFrameResource>,
               selection: Res<Fvr03ProductionVoxelSelectionResource>,
               labels: Query<(Entity, &AccessibilityNode)>,
               mut keys: ResMut<ButtonInput<KeyCode>>,
               mut actions: MessageWriter<ActionRequest>,
               mut commands: Commands,
               mut exit: MessageWriter<AppExit>| {
            if let Some(k) = release.take() {
                keys.release(k);
            }
            let seconds = time.elapsed_secs();
            let tick = frame.current.authoritative_world_tick.raw();
            if seconds < next {
                return;
            }
            next = seconds + 0.7;
            let mut photo = |name: &str| {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(out.join(format!("{name}.png"))));
            };
            let mut key = |k| {
                keys.press(k);
                release = Some(k);
            };
            let mut click = |label: &str| -> bool {
                let Some((e, _)) = labels
                    .iter()
                    .find(|(_, n)| n.label() == Some(label) && !n.is_hidden())
                else {
                    return false;
                };
                actions.write(ActionRequest(accesskit::ActionRequest {
                    action: accesskit::Action::Click,
                    target: accesskit::NodeId(e.to_bits()),
                    data: None,
                }));
                true
            };
            if tick >= last_tick + 40 {
                let residents: Vec<_> = frame
                    .current
                    .objects()
                    .filter(|o| o.kind == WorldObjectKind::Agent)
                    .map(|o| (o.id.raw(), o.position.x, o.position.z))
                    .collect();
                let fruit: Vec<_> = frame
                    .current
                    .objects()
                    .filter(|o| issued.contains(&o.id.raw()))
                    .map(|o| {
                        (
                            o.id.raw(),
                            o.consumed,
                            o.carried_by.map(|id| id.raw()),
                            o.position.x,
                            o.position.z,
                        )
                    })
                    .collect();
                writeln!(
                    log,
                    "sample\t{seconds:.1}\t{tick}\t{residents:?}\t{fruit:?}"
                )
                .unwrap();
                last_tick = tick;
            }
            match phase {
                0 => {
                    key(KeyCode::Escape);
                    phase = 1;
                    next = seconds + 4.;
                }
                1 => {
                    key(KeyCode::Home);
                    phase = 2;
                    next = seconds + 3.;
                }
                2 => {
                    if click("Choose soil for a lantern pear") {
                        phase = 3;
                    }
                }
                3 => {
                    if labels
                        .iter()
                        .any(|(_, n)| !n.is_hidden() && n.label() == Some("Cancel soil selection"))
                    {
                        photo("first-soil");
                        key(KeyCode::KeyU);
                        phase = 4;
                        next = seconds + 2.;
                    }
                }
                4 => {
                    let tile = selection.selected.and_then(|s| s.tile);
                    assert!(tile.is_some());
                    writeln!(log, "sow\t{seconds:.1}\t{tick}\t{tile:?}").unwrap();
                    photo("seed");
                    phase = 5;
                }
                5 => {
                    if click("Water these roots") {
                        photo("bud");
                        writeln!(log, "water\t{seconds:.1}\t{tick}").unwrap();
                        phase = 6;
                        next = seconds + 2.;
                    }
                }
                6 => {
                    photo("blossom");
                    phase = 7;
                }
                7 => {
                    if labels.iter().any(|(_, n)| {
                        !n.is_hidden() && n.label() == Some("Pick a ripe lantern pear")
                    }) {
                        photo("ripe");
                        known = frame
                            .current
                            .objects()
                            .filter(|o| o.kind == WorldObjectKind::Food)
                            .map(|o| o.id.raw())
                            .collect();
                        phase = 8;
                        next = seconds + 2.;
                    }
                }
                8 => {
                    assert!(click("Pick a ripe lantern pear"));
                    phase = 9;
                    next = seconds + 2.;
                }
                9 => {
                    let fruit = frame
                        .current
                        .objects()
                        .find(|o| o.kind == WorldObjectKind::Food && !known.contains(&o.id.raw()))
                        .expect("harvested physical fruit");
                    issued.push(fruit.id.raw());
                    photo("first-harvest");
                    writeln!(log, "picked\t{seconds:.1}\t{tick}\t{}", fruit.id.raw()).unwrap();
                    key(KeyCode::Home);
                    phase = 20;
                    next = seconds + 2.;
                }
                20 => {
                    // The resident is selected through Home, not repositioned.
                    if click("Bring a harvested pear to this individual") {
                        phase = 21;
                        next = seconds + 2.;
                    } else if seconds > 150. {
                        panic!("the known first harvest was not offerable");
                    }
                }
                21 => {
                    let fruit = frame
                        .current
                        .objects()
                        .find(|o| o.id.raw() == issued[0])
                        .expect("same offered fruit");
                    let agent = frame
                        .current
                        .objects()
                        .find(|o| o.kind == WorldObjectKind::Agent)
                        .unwrap();
                    let distance = (fruit.position.x - agent.position.x)
                        .hypot(fruit.position.z - agent.position.z);
                    writeln!(
                        log,
                        "offered\t{seconds:.1}\t{tick}\t{}\t{distance}\t{}",
                        fruit.id.raw(),
                        fruit.consumed
                    )
                    .unwrap();
                    photo("offered");
                    key(KeyCode::KeyS);
                    phase = 22;
                    next = seconds + 2.;
                }
                22 => {
                    if click("Return to your pear garden") {
                        phase = 10;
                    }
                }
                10 => {
                    if labels.iter().any(|(_, n)| {
                        !n.is_hidden() && n.label() == Some("Pick a ripe lantern pear")
                    }) {
                        photo("unwatered-return");
                        known = frame
                            .current
                            .objects()
                            .filter(|o| o.kind == WorldObjectKind::Food)
                            .map(|o| o.id.raw())
                            .collect();
                        phase = 11;
                        next = seconds + 2.;
                    }
                }
                11 => {
                    assert!(click("Pick a ripe lantern pear"));
                    phase = 12;
                    next = seconds + 2.;
                }
                12 => {
                    let fruit = frame
                        .current
                        .objects()
                        .find(|o| o.kind == WorldObjectKind::Food && !known.contains(&o.id.raw()))
                        .expect("second physical fruit");
                    issued.push(fruit.id.raw());
                    photo("second-harvest");
                    writeln!(log, "second\t{seconds:.1}\t{tick}\t{}", fruit.id.raw()).unwrap();
                    key(KeyCode::KeyS);
                    completion = true;
                    phase = 13;
                }
                13 if seconds >= 300. => {
                    photo("five-minutes");
                    writeln!(log, "completed\t{seconds:.1}\t{tick}\t{issued:?}").unwrap();
                    observed.store(completion, std::sync::atomic::Ordering::Relaxed);
                    phase = 14;
                    next = seconds + 2.;
                }
                14 => {
                    exit.write(AppExit::Success);
                }
                _ => {}
            }
            log.flush().unwrap();
        })
        .after(InputSystems),
    );
    assert!(
        app.run().is_success() && passed.load(std::sync::atomic::Ordering::Relaxed),
        "orchard loop did not complete"
    );
}
