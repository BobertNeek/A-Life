//! A normal nursery: aim the actual meadow ball at an existing obstacle, then inspect the hazard.
#[cfg(not(feature = "production-voxel-frontend"))]
fn main() {
    eprintln!("Enable production-voxel-frontend.");
}
#[cfg(feature = "production-voxel-frontend")]
mod walk {
    use alife_game_app::{
        bevy_shell::{
            build_production_voxel_frontend_app_shell, LiveBrainPresentationFrameResource,
        },
        default_environment_manifest_path, Fvr03ProductionVoxelCamera,
        Fvr03ProductionVoxelSelectionResource, ProductionRunMode, ProductionVoxelLaunchConfig,
        ProductionWorldSource,
    };
    use bevy::{
        app::AppExit,
        input::InputSystems,
        prelude::*,
        render::view::screenshot::{save_to_disk, Screenshot},
        window::PrimaryWindow,
    };
    use std::{
        io::Write,
        path::PathBuf,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        time::{SystemTime, UNIX_EPOCH},
    };
    const PAN: [KeyCode; 4] = [
        KeyCode::ArrowLeft,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
    ];

    #[derive(Resource)]
    struct Walk {
        step: u8,
        next: f32,
        rolls: u8,
        pulse: Option<KeyCode>,
        start: Vec3,
        out: PathBuf,
        log: std::fs::File,
        passed: Arc<AtomicBool>,
    }
    impl Walk {
        fn key(&mut self, keys: &mut ButtonInput<KeyCode>, k: KeyCode) {
            keys.press(k);
            self.pulse = Some(k);
        }
        fn photo(&self, cmd: &mut Commands, name: &str) {
            cmd.spawn(Screenshot::primary_window())
                .observe(save_to_disk(self.out.join(format!("{name}.png"))));
        }
    }
    fn frame_target(
        keys: &mut ButtonInput<KeyCode>,
        camera: &Camera,
        pose: &GlobalTransform,
        size: Vec2,
        target: Vec3,
    ) -> bool {
        for k in PAN {
            keys.release(k);
        }
        let Some(delta) = camera
            .world_to_viewport(pose, target)
            .ok()
            .map(|p| p - size * Vec2::new(0.60, 0.56))
        else {
            return false;
        };
        if delta.x.abs() > 18. {
            keys.press(if delta.x < 0. { PAN[0] } else { PAN[1] });
        }
        if delta.y.abs() > 18. {
            keys.press(if delta.y < 0. { PAN[2] } else { PAN[3] });
        }
        delta.abs().max_element() <= 18.
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "The player-route probe only reads the authoritative world, camera, meshes and selection; it sends ordinary keyboard and pointer input."
    )]
    fn probe(
        time: Res<Time<Real>>,
        mut w: ResMut<Walk>,
        frame: Res<LiveBrainPresentationFrameResource>,
        selection: Res<Fvr03ProductionVoxelSelectionResource>,
        cam: Query<(&Camera, &GlobalTransform), With<Fvr03ProductionVoxelCamera>>,
        mut window: Query<&mut Window, With<PrimaryWindow>>,
        visuals: Query<(&Name, &Mesh3d, &Transform)>,
        meshes: Res<Assets<Mesh>>,
        mut keys: ResMut<ButtonInput<KeyCode>>,
        mut mouse: ResMut<ButtonInput<MouseButton>>,
        mut cmd: Commands,
        mut exit: MessageWriter<AppExit>,
    ) {
        if let Some(k) = w.pulse.take() {
            keys.release(k);
        }
        let now = time.elapsed_secs();
        if now < w.next {
            return;
        }
        w.next = now + 0.09;
        let (camera, ct) = cam.single().unwrap();
        let mut window = window.single_mut().unwrap();
        let size = Vec2::new(window.width(), window.height());
        let find = |label: &str| frame.current.objects().find(|o| o.label == label);
        let (Some(ball), Some(wall), Some(vent)) =
            (find("meadow-ball"), find("obstacle-02"), find("hazard-01"))
        else {
            return;
        };
        let xyz =
            |o: &alife_world::WorldObject| Vec3::new(o.position.x, o.position.y, o.position.z);
        let (b, t, h) = (xyz(ball), xyz(wall), xyz(vent));
        let gap = b.distance(t) - ball.radius - wall.radius;
        let tick = frame.current.authoritative_world_tick.raw();
        let step = w.step;
        let rolls = w.rolls;
        writeln!(
            w.log,
            "{step} {now:0.2} t={tick} roll={rolls} ball={b:?} wall={t:?} clearance={gap:0.4}"
        )
        .unwrap();
        w.log.flush().unwrap();
        let mut point =
            |pos: Vec3| window.set_cursor_position(camera.world_to_viewport(ct, pos).ok());
        let visible = |label: &str, p: Vec3| {
            visuals.iter().any(|(n, m, tr)| {
                n.as_str() == label && tr.translation.distance(p) < 0.02 && meshes.contains(m.id())
            })
        };
        match step {
            0 => {
                w.key(&mut keys, KeyCode::Escape);
                w.next = now + 1.2;
            }
            1 => {
                window.set_cursor_position(Some(size * 0.5));
                w.key(&mut keys, KeyCode::Home);
                w.next = now + 1.;
            }
            2 => {
                if tick < 145
                    || !visible("Grounded waykeeper", t)
                    || !visible("Grounded emberstone", h)
                {
                    return;
                }
                if !frame_target(&mut keys, camera, ct, size, (b + t) * 0.5) {
                    return;
                }
                writeln!(w.log,"observed world fixtures: obstacle {} radius {} at {:?}; hazard {} radius {} at {:?}",wall.id.raw(),wall.radius,t,vent.id.raw(),vent.radius,h).unwrap();
                w.photo(&mut cmd, "nursery-route");
            }
            3 => {
                assert!(!ball.consumed && ball.carried_by.is_none());
                point(b + Vec3::Y * 0.30);
                if selection.hovered.and_then(|s| s.stable_id) != Some(ball.id) {
                    return;
                }
                w.start = b;
                mouse.press(MouseButton::Right);
            }
            4 => {
                point(t);
                w.next = now + 0.24;
            }
            5 => {
                point(t);
                mouse.release(MouseButton::Right);
                w.rolls += 1;
                w.next = now + 3.6;
            }
            6 => {
                assert!(b.distance(w.start) > 0.04, "rattle did not travel");
                if gap > 0.065 {
                    assert!(w.rolls < 4, "rolls never reached the physical obstacle");
                    w.step = 3;
                    return;
                }
                assert!(gap >= -0.045, "ball entered obstacle");
                w.start = b;
                w.next = now + 0.7;
            }
            7 => {
                assert!(b.distance(w.start) < 0.02, "parked ball drifted");
                if !frame_target(&mut keys, camera, ct, size, (b + t) * 0.5) {
                    return;
                }
                w.photo(&mut cmd, "waykeeper-contact");
                w.key(&mut keys, KeyCode::F5);
                w.next = now + 1.;
            }
            8 => {
                if !frame_target(&mut keys, camera, ct, size, h) {
                    return;
                }
                w.photo(&mut cmd, "emberstone");
                w.key(&mut keys, KeyCode::F5);
                w.next = now + 1.4;
            }
            9 => {
                let count =
                    std::fs::read_dir(w.out.join("field-photographs")).map_or(0, |entries| {
                        entries
                            .flatten()
                            .filter(|e| e.path().extension().is_some_and(|x| x == "png"))
                            .count()
                    });
                if count != 2 {
                    return;
                }
                w.passed.store(true, Ordering::Relaxed);
                exit.write(AppExit::Success);
                return;
            }
            _ => return,
        }
        w.step += 1;
    }
    pub fn run() {
        let out = PathBuf::from("target/artifacts/landmark-walk").join(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
        );
        std::fs::create_dir_all(&out).unwrap();
        let mut launch =
            ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
                .unwrap();
        launch.world_source = ProductionWorldSource::NewGame { seed: 1643 };
        launch.population = Some(1);
        launch.run_mode = Some(ProductionRunMode::OneX);
        launch.require_gpu = true;
        launch.resolution = (960, 540);
        launch.ui_settings_path = Some(out.join("settings.json"));
        launch.smoke_seconds = Some(85);
        let (mut app, _) = build_production_voxel_frontend_app_shell(&launch).unwrap();
        let passed = Arc::new(AtomicBool::new(false));
        let log = std::fs::File::create(out.join("walk.tsv")).unwrap();
        app.insert_resource(Walk {
            step: 0,
            next: 7.,
            rolls: 0,
            pulse: None,
            start: Vec3::ZERO,
            out,
            log,
            passed: passed.clone(),
        });
        app.add_systems(PreUpdate, probe.after(InputSystems));
        assert!(
            app.run().is_success() && passed.load(Ordering::Relaxed),
            "landmark player route unfinished"
        );
    }
}
#[cfg(feature = "production-voxel-frontend")]
fn main() {
    walk::run();
}
