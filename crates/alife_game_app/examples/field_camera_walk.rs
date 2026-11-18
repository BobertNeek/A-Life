//! Production-only player route: expose, inspect, browse and reopen a local plate.
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
        default_environment_manifest_path, Fvr05ProductionUxStateResource, ProductionRunMode,
        ProductionVoxelLaunchConfig, ProductionWorldSource,
    };
    use bevy::{
        a11y::{AccessibilityNode, ActionRequest},
        app::AppExit,
        input::InputSystems,
        prelude::*,
        render::view::screenshot::{save_to_disk, Screenshot},
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

    #[derive(Resource)]
    struct Walk {
        out: PathBuf,
        folder: PathBuf,
        log: std::fs::File,
        phase: u8,
        next: f32,
        key: Option<KeyCode>,
        first_tick: u64,
        paused_tick: u64,
        first: Option<Handle<Image>>,
        reopen: bool,
        passed: Arc<AtomicBool>,
    }
    impl Walk {
        fn press(&mut self, keys: &mut ButtonInput<KeyCode>, key: KeyCode) {
            keys.press(key);
            self.key = Some(key);
        }
        fn snapshot(&self, cmd: &mut Commands, name: &str) {
            cmd.spawn(Screenshot::primary_window())
                .observe(save_to_disk(self.out.join(format!("{name}.png"))));
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Read the same exposed menus, images, accessibility actions and simulation clock as a player."
    )]
    fn probe(
        time: Res<Time<Real>>,
        mut w: ResMut<Walk>,
        frame: Res<LiveBrainPresentationFrameResource>,
        ux: Res<Fvr05ProductionUxStateResource>,
        nodes: Query<(Entity, &AccessibilityNode)>,
        images: Query<(&Name, &ImageNode, &Visibility)>,
        names: Query<&Name>,
        mut keys: ResMut<ButtonInput<KeyCode>>,
        mut requests: MessageWriter<ActionRequest>,
        mut cmd: Commands,
        mut exit: MessageWriter<AppExit>,
    ) {
        if let Some(key) = w.key.take() {
            keys.release(key);
        }
        let now = time.elapsed_secs();
        if now < w.next {
            return;
        }
        w.next = now + 0.22;
        let tick = frame.current.authoritative_world_tick.raw();
        let mut click = |label: &str| {
            let Some((e, _)) = nodes
                .iter()
                .find(|(_, n)| !n.is_hidden() && n.label() == Some(label))
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
        let paper = images
            .iter()
            .find(|(n, _, v)| {
                n.as_str() == "Developed field photograph" && **v != Visibility::Hidden
            })
            .map(|(_, n, _)| n.image.clone());
        let photos = std::fs::read_dir(&w.folder).map_or(0, |d| {
            d.flatten()
                .filter(|e| e.file_name().to_string_lossy().ends_with(".png"))
                .count()
        });
        let phase = w.phase;
        writeln!(
            w.log,
            "{phase} {now:.2} tick={tick} paused={} files={photos} paper={paper:?}",
            ux.settings.paused
        )
        .unwrap();
        w.log.flush().unwrap();
        match phase {
            0 if w.reopen => {
                if !click("Photos") {
                    return;
                }
                w.phase = 12;
                return;
            }
            0 => {
                w.press(&mut keys, KeyCode::Escape);
                w.next = now + 1.3;
            }
            1 => {
                w.press(&mut keys, KeyCode::Home);
                w.next = now + 2.2;
            }
            2 => {
                if tick < 150
                    || !names
                        .iter()
                        .any(|name| name.as_str().starts_with("Island chunk "))
                {
                    return;
                }
                assert!(!ux.settings.paused);
                w.first_tick = tick;
                w.snapshot(&mut cmd, "source-view");
                w.press(&mut keys, KeyCode::F5);
            }
            3 => {
                if photos != 1 || tick <= w.first_tick {
                    return;
                }
                assert!(!ux.settings.paused);
                keys.press(KeyCode::ArrowRight);
                w.phase = 14;
                w.next = now + 0.42;
                return;
            }
            14 => {
                keys.release(KeyCode::ArrowRight);
                w.press(&mut keys, KeyCode::Escape);
                w.phase = 4;
                w.next = now + 0.55;
                return;
            }
            4 => {
                if !click("Photos") {
                    return;
                }
            }
            5 => {
                if photos != 1 || paper.is_none() {
                    return;
                }
                w.paused_tick = tick;
                assert!(ux.settings.paused);
                w.snapshot(&mut cmd, "first-print");
                if !click("Take a field photograph") {
                    return;
                }
            }
            6 => {
                if photos != 2
                    || !nodes
                        .iter()
                        .any(|(_, n)| !n.is_hidden() && n.label() == Some("Photograph 1 of 2"))
                {
                    return;
                }
                assert_eq!(tick, w.paused_tick);
                w.first = paper.clone();
                if !click("Previous photograph") {
                    return;
                }
            }
            7 => {
                if !nodes
                    .iter()
                    .any(|(_, n)| !n.is_hidden() && n.label() == Some("Photograph 2 of 2"))
                {
                    return;
                }
                assert_ne!(w.first, paper);
                assert_eq!(tick, w.paused_tick);
                w.snapshot(&mut cmd, "older-print");
                if !click("Next photograph") {
                    return;
                }
            }
            8 => {
                if !nodes
                    .iter()
                    .any(|(_, n)| !n.is_hidden() && n.label() == Some("Photograph 1 of 2"))
                {
                    return;
                }
                if !click("Back") {
                    return;
                }
            }
            9 => {
                if !click("Play") {
                    return;
                }
                w.next = now + 1.1;
            }
            10 => {
                assert!(!ux.settings.paused && tick > w.paused_tick);
                verify_prints(&w.folder);
                w.passed.store(true, Ordering::Relaxed);
                w.phase = 13;
                return;
            }
            12 => {
                if photos != 2 || paper.is_none() {
                    return;
                }
                verify_prints(&w.folder);
                w.snapshot(&mut cmd, "reopened-shelf");
                w.passed.store(true, Ordering::Relaxed);
                w.next = now + 1.2;
            }
            13 => {
                exit.write(AppExit::Success);
                return;
            }
            _ => return,
        };
        w.phase += 1;
    }
    fn verify_prints(folder: &std::path::Path) {
        let mut prints = Vec::new();
        for entry in std::fs::read_dir(folder).unwrap().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "png") {
                continue;
            }
            let frame = image::open(path).unwrap().to_rgb8();
            assert_eq!(frame.dimensions(), (640, 360));
            let tones: std::collections::BTreeSet<_> = frame
                .pixels()
                .step_by(13)
                .map(|p| p.0.map(|value| value / 12))
                .collect();
            assert!(
                tones.len() > 28,
                "The plate contains no recognizable light or terrain detail."
            );
            prints.push(frame);
        }
        assert_eq!(prints.len(), 2);
        let changed = prints[0]
            .pixels()
            .zip(prints[1].pixels())
            .filter(|(a, b)| a.0.iter().zip(b.0).any(|(x, y)| x.abs_diff(y) > 18))
            .count();
        assert!(
            changed > prints[0].len() / 300,
            "Panning did not change the exposure."
        );
    }
    pub fn run() {
        let args: Vec<_> = std::env::args().skip(1).collect();
        let reopen = args.first().is_some_and(|v| v == "--reopen");
        let out = if reopen {
            PathBuf::from(
                args.get(1)
                    .expect("--reopen requires previous evidence directory"),
            )
        } else {
            PathBuf::from("target/artifacts/field-camera").join(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    .to_string(),
            )
        };
        std::fs::create_dir_all(&out).unwrap();
        let out = std::fs::canonicalize(out).unwrap();
        let mut launch =
            ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
                .unwrap();
        if reopen {
            launch.world_source = ProductionWorldSource::LoadExisting;
            launch.app_launch.save_path = out.join("phase3-new-game-1643.json");
        } else {
            launch.world_source = ProductionWorldSource::NewGame { seed: 1643 };
            launch.population = Some(1);
        }
        launch.run_mode = Some(ProductionRunMode::OneX);
        launch.require_gpu = true;
        launch.resolution = (960, 540);
        launch.smoke_seconds = Some(68);
        launch.ui_settings_path = Some(out.join("settings.json"));
        let (mut app, _) = build_production_voxel_frontend_app_shell(&launch).unwrap();
        let passed = Arc::new(AtomicBool::new(false));
        let log = std::fs::File::create(out.join(if reopen { "reopen.tsv" } else { "walk.tsv" }))
            .unwrap();
        app.insert_resource(Walk {
            folder: out.join("field-photographs"),
            out,
            log,
            phase: 0,
            next: 7.,
            key: None,
            first_tick: 0,
            paused_tick: 0,
            first: None,
            reopen,
            passed: passed.clone(),
        });
        app.add_systems(PreUpdate, probe.after(InputSystems));
        assert!(
            app.run().is_success() && passed.load(Ordering::Relaxed),
            "field camera walk unfinished"
        );
    }
}
#[cfg(feature = "production-voxel-frontend")]
fn main() {
    walk::run();
}
