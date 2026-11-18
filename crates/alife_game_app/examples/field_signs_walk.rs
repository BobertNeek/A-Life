//! Observe a real route, then look back at one actual gift using the production Hand.
#[cfg(not(feature = "production-voxel-frontend"))]
fn main() {
    eprintln!("Enable production-voxel-frontend.");
}

#[cfg(feature = "production-voxel-frontend")]
mod playtest {
    use alife_game_app::{
        bevy_shell::{
            build_production_voxel_frontend_app_shell, LiveBrainPresentationFrameResource,
        },
        default_environment_manifest_path, Fvr03ProductionVoxelCamera, ProductionRunMode,
        ProductionVoxelLaunchConfig, ProductionWorldSource,
    };
    use alife_world::WorldObjectKind;
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
        time::SystemTime,
    };

    #[derive(Resource)]
    struct Walk {
        out: PathBuf,
        log: std::fs::File,
        passed: Arc<AtomicBool>,
        phase: u8,
        linger: bool,
        next: f32,
        detailed_since: Option<f32>,
        last_report: u64,
        release: Option<KeyCode>,
        before: BTreeSet<u64>,
        gift: Option<alife_core::WorldEntityId>,
    }
    impl Walk {
        fn press(&mut self, keys: &mut ButtonInput<KeyCode>, key: KeyCode) {
            keys.press(key);
            self.release = Some(key);
        }
        fn photo(&self, cmd: &mut Commands, name: &str) {
            cmd.spawn(Screenshot::primary_window())
                .observe(save_to_disk(self.out.join(format!("{name}.png"))));
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "Inspect public scene, input and presentation surfaces in an isolated playthrough."
    )]
    fn probe(
        time: Res<Time<Real>>,
        mut walk: ResMut<Walk>,
        frame: Res<LiveBrainPresentationFrameResource>,
        names: Query<&Name, With<Mesh3d>>,
        controls: Query<(Entity, &AccessibilityNode)>,
        cameras: Query<(&Camera, &GlobalTransform), With<Fvr03ProductionVoxelCamera>>,
        mut keys: ResMut<ButtonInput<KeyCode>>,
        mut actions: MessageWriter<ActionRequest>,
        mut commands: Commands,
        mut exit: MessageWriter<AppExit>,
    ) {
        if let Some(key) = walk.release.take() {
            keys.release(key);
        }
        let now = time.elapsed_secs();
        if now < walk.next {
            return;
        }
        walk.next = now + 0.4;
        let tick = frame.current.authoritative_world_tick.raw();
        let marks = names
            .iter()
            .filter(|n| n.as_str() == "Witnessed Hearthling footfall")
            .count();
        let detailed = names
            .iter()
            .any(|n| n.as_str().starts_with("Island chunk "));
        if detailed {
            walk.detailed_since.get_or_insert(now);
        }
        let controls: Vec<_> = controls
            .iter()
            .filter(|(_, n)| !n.is_hidden())
            .filter_map(|(e, n)| Some((e, n.label()?)))
            .collect();
        let lens = controls
            .iter()
            .find(|(_, label)| label.starts_with("Look "));
        let mut click = |entity: Entity| {
            actions.write(ActionRequest(accesskit::ActionRequest {
                action: accesskit::Action::Click,
                target: accesskit::NodeId(entity.to_bits()),
                data: None,
            }));
        };
        let food = || {
            frame
                .current
                .objects()
                .filter(|o| o.kind == WorldObjectKind::Food)
        };
        if tick >= walk.last_report + 45 {
            let ids: Vec<_> = food()
                .map(|o| (o.id.raw(), o.consumed, o.carried_by))
                .collect();
            let phase = walk.phase;
            writeln!(
                walk.log,
                "sample\t{now:.1}\t{tick}\t{phase}\t{marks}\t{ids:?}\t{:?}",
                lens.map(|(_, s)| s)
            )
            .unwrap();
            walk.last_report = tick;
        }
        match walk.phase {
            0 => {
                walk.press(&mut keys, KeyCode::Escape);
                walk.phase = 1;
                walk.next = now + 2.;
            }
            1 => {
                walk.press(&mut keys, KeyCode::Home);
                walk.phase = 2;
                walk.next = now + 1.;
            }
            2 if marks >= 4 && walk.detailed_since.is_some_and(|t| now - t >= 2.) => {
                walk.photo(&mut commands, "observed-route");
                writeln!(walk.log, "route\t{tick}\t{marks}").unwrap();
                walk.before = food().map(|o| o.id.raw()).collect();
                walk.press(&mut keys, KeyCode::KeyE);
                walk.phase = 3;
                walk.next = now + 1.5;
            }
            3 => {
                if let (Some(o), Some((entity, label))) =
                    (food().find(|o| !walk.before.contains(&o.id.raw())), lens)
                {
                    walk.gift = Some(o.id);
                    click(*entity);
                    writeln!(walk.log, "look\t{tick}\t{}\t{label}", o.id.raw()).unwrap();
                    walk.phase = 4;
                    walk.next = now + 2.;
                }
            }
            4 | 7 => {
                let o = frame
                    .current
                    .object(walk.gift.unwrap())
                    .expect("the actual placed object");
                let position =
                    o.carried_by
                        .and_then(|by| {
                            frame.current.objects().find(|p| {
                                p.kind == WorldObjectKind::Agent && p.organism_id == Some(by)
                            })
                        })
                        .map_or(o.position, |p| p.position);
                let (camera, transform) = cameras.single().unwrap();
                let point = camera
                    .world_to_viewport(
                        transform,
                        Vec3::new(position.x, position.y + 0.15, position.z),
                    )
                    .unwrap();
                let center = camera.logical_viewport_size().unwrap() * 0.5;
                writeln!(
                    walk.log,
                    "gift\t{tick}\t{}\t{point:?}\tcenter={center:?}\tconsumed={}\tcarried_by={:?}",
                    o.id.raw(),
                    o.consumed,
                    o.carried_by
                )
                .unwrap();
                assert!(
                    (point - center).length() < center.y * 0.48,
                    "gift not framed"
                );
                assert_eq!(
                    food()
                        .filter(|o| !walk.before.contains(&o.id.raw()))
                        .count(),
                    1,
                    "looking duplicated the gift"
                );
                let revisit = walk.phase == 7;
                walk.photo(
                    &mut commands,
                    if revisit {
                        "revisited-gift"
                    } else {
                        "real-gift"
                    },
                );
                if !walk.linger || revisit {
                    walk.passed.store(true, Ordering::Relaxed);
                    walk.phase = 8;
                } else {
                    walk.phase = 5;
                }
                walk.next = now + 2.;
            }
            5 => {
                walk.press(&mut keys, KeyCode::Home);
                walk.phase = 6;
            }
            6 if now >= 150.0 => {
                if let Some((entity, label)) = lens {
                    click(*entity);
                    let id = walk.gift.unwrap().raw();
                    let living = frame
                        .current
                        .objects()
                        .filter(|o| o.kind == WorldObjectKind::Agent && !o.consumed)
                        .count();
                    writeln!(
                        walk.log,
                        "revisit\t{now:.1}\t{tick}\t{id}\t{label}\tliving={living}"
                    )
                    .unwrap();
                    walk.phase = 7;
                    walk.next = now + 2.;
                }
            }
            8 => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
        walk.log.flush().unwrap();
    }

    pub fn run() {
        let ts = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let out = PathBuf::from("target/artifacts/field-signs").join(ts.to_string());
        std::fs::create_dir_all(&out).unwrap();
        let out = std::fs::canonicalize(out).unwrap();
        let log = std::fs::File::create(out.join("walk.tsv")).unwrap();
        let mut launch =
            ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
                .unwrap();
        launch.world_source = ProductionWorldSource::NewGame { seed: 1643 };
        launch.population = Some(1);
        launch.require_gpu = true;
        launch.run_mode = Some(ProductionRunMode::OneX);
        launch.resolution = (960, 540);
        let linger = std::env::args().skip(1).any(|s| s == "--linger");
        launch.smoke_seconds = Some(if linger { 200 } else { 110 });
        launch.ui_settings_path = Some(out.join("settings.json"));
        let (mut app, _) = build_production_voxel_frontend_app_shell(&launch).unwrap();
        let passed = Arc::new(AtomicBool::new(false));
        app.insert_resource(Walk {
            out,
            log,
            passed: passed.clone(),
            phase: 0,
            linger,
            next: 7.,
            detailed_since: None,
            last_report: 0,
            release: None,
            before: BTreeSet::new(),
            gift: None,
        });
        app.add_systems(PreUpdate, probe.after(InputSystems));
        assert!(
            app.run().is_success() && passed.load(Ordering::Relaxed),
            "field signs were not observed"
        );
    }
}
#[cfg(feature = "production-voxel-frontend")]
fn main() {
    playtest::run();
}
