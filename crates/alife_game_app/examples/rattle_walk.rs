//! Exercise the production pointer, world clock, chime mesh and accessible toy control.
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
        a11y::{AccessibilityNode, ActionRequest},
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
        time::SystemTime,
    };

    #[derive(Resource)]
    struct Walk {
        phase: u8,
        next: f32,
        release: Option<KeyCode>,
        start: Vec3,
        parked: Vec3,
        out: PathBuf,
        log: std::fs::File,
        passed: Arc<AtomicBool>,
    }
    impl Walk {
        fn key(&mut self, keys: &mut ButtonInput<KeyCode>, k: KeyCode) {
            keys.press(k);
            self.release = Some(k);
        }
        fn photo(&self, commands: &mut Commands, name: &str) {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(self.out.join(format!("{name}.png"))));
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "The probe observes real production picking and perceptible outcomes rather than writing a world or a selection."
    )]
    fn probe(
        time: Res<Time<Real>>,
        mut walk: ResMut<Walk>,
        frame: Res<LiveBrainPresentationFrameResource>,
        selection: Res<Fvr03ProductionVoxelSelectionResource>,
        mut windows: Query<&mut Window, With<PrimaryWindow>>,
        cam: Query<(&Camera, &GlobalTransform), With<Fvr03ProductionVoxelCamera>>,
        mut keys: ResMut<ButtonInput<KeyCode>>,
        mut mouse: ResMut<ButtonInput<MouseButton>>,
        names: Query<(&Name, &Visibility)>,
        controls: Query<(Entity, &AccessibilityNode)>,
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
        let (camera, ct) = cam.single().unwrap();
        let mut window = windows.single_mut().unwrap();
        let mut point =
            |p: Vec3| window.set_cursor_position(Some(camera.world_to_viewport(ct, p).unwrap()));
        let find = |label: &str| frame.current.objects().find(|o| o.label == label);
        let p = |o: &alife_world::WorldObject| Vec3::new(o.position.x, o.position.y, o.position.z);
        let tick = frame.current.authoritative_world_tick.raw();
        let signs = |name: &str| {
            names
                .iter()
                .filter(|(n, v)| n.as_str() == name && **v != Visibility::Hidden)
                .count()
        };
        let phase = walk.phase;
        writeln!(
            walk.log,
            "sample {now:.2} {tick} {phase} hover={:?} ball={:?} ripples={}",
            selection.hovered.and_then(|s| s.stable_id),
            find("player-plaything").map(p),
            signs("Struck chime ripple")
        )
        .unwrap();
        walk.log.flush().unwrap();
        walk.next = now + 0.08;
        match phase {
            0 => {
                walk.key(&mut keys, KeyCode::Escape);
                walk.next = now + 1.4;
            }
            1 => {
                walk.key(&mut keys, KeyCode::Home);
                walk.next = now + 1.;
            }
            2 => {
                walk.key(&mut keys, KeyCode::KeyJ);
                walk.next = now + 0.8;
            }
            3 => {
                let Some(ball) = find("player-plaything") else {
                    return;
                };
                walk.start = p(ball);
                walk.key(&mut keys, KeyCode::KeyZ);
                walk.next = now + 2.;
            }
            4 => {
                let ball = find("player-plaything").unwrap();
                point(p(ball) + Vec3::Y * 0.30);
                if selection.hovered.and_then(|s| s.stable_id) != Some(ball.id)
                    || signs("Rattle aiming pearl") < 5
                {
                    return;
                }
                walk.photo(&mut commands, "rattle-guide");
                mouse.press(MouseButton::Right);
            }
            5 => {
                mouse.release(MouseButton::Right);
            }
            6 => {
                let ball = find("player-plaything").unwrap();
                let station = find("player-activity-toy").unwrap();
                if signs("Struck chime ripple") == 0 {
                    return;
                }
                assert!(walk.start.distance(p(ball)) > 0.45);
                assert!(
                    (p(ball).distance(p(station)) - (ball.radius + station.radius)).abs() < 0.04
                );
                walk.parked = p(ball);
                walk.photo(&mut commands, "real-table-strike");
                walk.next = now + 1.8;
            }
            7 => {
                let ball = find("player-plaything").unwrap();
                assert!(walk.parked.distance(p(ball)) < 0.01);
                point(p(ball) + Vec3::Y * 0.30);
                if selection.hovered.and_then(|s| s.stable_id) != Some(ball.id) {
                    return;
                }
                mouse.press(MouseButton::Right);
            }
            8 => {
                let target = walk.parked + Vec3::new(-2.5, 0., 1.5);
                point(target);
                walk.next = now + 0.24;
            }
            9 => {
                let target = walk.parked + Vec3::new(-2.5, 0., 1.5);
                point(target);
                mouse.release(MouseButton::Right);
                walk.next = now + 2.5;
            }
            10 => {
                let ball = find("player-plaything").unwrap();
                assert!(
                    walk.parked.distance(p(ball)) > 1.4,
                    "drag must send the same ball away"
                );
                walk.photo(&mut commands, "returned-rattle");
                point(p(ball) + Vec3::Y * 0.30);
                if selection.hovered.and_then(|s| s.stable_id) != Some(ball.id) {
                    return;
                }
                mouse.press(MouseButton::Left);
                walk.next = now;
            }
            11 => {
                mouse.release(MouseButton::Left);
                walk.next = now + 0.18;
            }
            12 => {
                let ball = find("player-plaything").unwrap();
                assert_eq!(selection.selected.and_then(|s| s.stable_id), Some(ball.id));
                let (entity, _) = controls
                    .iter()
                    .find(|(_, n)| n.label().is_some_and(|s| s.starts_with("Roll this ball")))
                    .unwrap();
                actions.write(ActionRequest(accesskit::ActionRequest {
                    action: accesskit::Action::Click,
                    target: accesskit::NodeId(entity.to_bits()),
                    data: None,
                }));
            }
            13 => {
                if signs("Struck chime ripple") == 0 {
                    return;
                }
                walk.photo(&mut commands, "accessible-return");
                assert_eq!(
                    frame
                        .current
                        .objects()
                        .filter(|o| o.label == "player-plaything")
                        .count(),
                    1
                );
                walk.passed.store(true, Ordering::Relaxed);
                walk.next = now + 1.5;
            }
            _ => {
                exit.write(AppExit::Success);
                return;
            }
        }
        walk.phase += 1;
    }
    pub fn run() {
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let out = PathBuf::from("target/artifacts/rattle-walk").join(stamp.to_string());
        std::fs::create_dir_all(&out).unwrap();
        let out = std::fs::canonicalize(out).unwrap();
        let log = std::fs::File::create(out.join("walk.tsv")).unwrap();
        let mut config =
            ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
                .unwrap();
        config.world_source = ProductionWorldSource::NewGame { seed: 1643 };
        config.population = Some(1);
        config.run_mode = Some(ProductionRunMode::OneX);
        config.require_gpu = true;
        config.resolution = (960, 540);
        config.smoke_seconds = Some(70);
        config.ui_settings_path = Some(out.join("settings.json"));
        let (mut app, _) = build_production_voxel_frontend_app_shell(&config).unwrap();
        let passed = Arc::new(AtomicBool::new(false));
        app.insert_resource(Walk {
            phase: 0,
            next: 7.,
            release: None,
            start: Vec3::ZERO,
            parked: Vec3::ZERO,
            out,
            log,
            passed: passed.clone(),
        });
        app.add_systems(PreUpdate, probe.after(InputSystems));
        assert!(
            app.run().is_success() && passed.load(Ordering::Relaxed),
            "rattle playthrough did not finish"
        );
    }
}
#[cfg(feature = "production-voxel-frontend")]
fn main() {
    walk::run();
}
