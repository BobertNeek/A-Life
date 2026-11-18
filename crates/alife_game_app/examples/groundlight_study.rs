//! Photograph the same paused native frame with and without the display treatment.
//! This comparison changes only camera edge smoothing and visibility of shade meshes.
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
        default_environment_manifest_path, Fvr03ProductionVoxelCamera,
        Fvr05ProductionUxStateResource, ProductionRunMode, ProductionVoxelLaunchConfig,
        ProductionWorldSource,
    };
    use bevy::{
        app::AppExit,
        camera::visibility::RenderLayers,
        input::{
            keyboard::{Key, KeyboardInput, NativeKey},
            ButtonState, InputSystems,
        },
        prelude::*,
        render::view::screenshot::{save_to_disk, Screenshot},
        window::PrimaryWindow,
    };
    use bevy_anti_alias::fxaa::Fxaa;
    use std::{io::Write, path::PathBuf, time::SystemTime};
    let stamp = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs();
    let out = PathBuf::from("target/artifacts/groundlight").join(stamp.to_string());
    std::fs::create_dir_all(&out).unwrap();
    let out = std::fs::canonicalize(out).unwrap();
    let mut log = std::fs::File::create(out.join("study.tsv")).unwrap();
    let mut config =
        ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
            .unwrap();
    config.world_source = ProductionWorldSource::NewGame { seed: 1643 };
    config.population = Some(1);
    config.run_mode = Some(ProductionRunMode::OneX);
    config.require_gpu = true;
    config.resolution = (960, 540);
    config.smoke_seconds = Some(55);
    config.ui_settings_path = Some(out.join("settings.json"));
    let (mut app, _) = build_production_voxel_frontend_app_shell(&config).unwrap();
    let result = out.join("with-groundlight.png");
    let (mut phase, mut next, mut release, mut stopped) = (0, 7., None, 0);
    app.add_systems(
        PreUpdate,
        (move |time: Res<Time<Real>>,
               frame: Res<LiveBrainPresentationFrameResource>,
               ux: Res<Fvr05ProductionUxStateResource>,
               window: Query<Entity, With<PrimaryWindow>>,
               mut aa: Query<&mut Fxaa, With<Fvr03ProductionVoxelCamera>>,
               prints: Query<(Entity, &Name, &Visibility)>,
               mut keys: ResMut<ButtonInput<KeyCode>>,
               mut events: MessageWriter<KeyboardInput>,
               mut commands: Commands,
               mut exit: MessageWriter<AppExit>| {
            let window = window.single().unwrap();
            let key_event = |code, state| KeyboardInput {
                key_code: code,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state,
                text: None,
                repeat: false,
                window,
            };
            if let Some(key) = release.take() {
                keys.release(key);
                events.write(key_event(key, ButtonState::Released));
            }
            let now = time.elapsed_secs();
            if now < next {
                return;
            }
            let tick = frame.current.authoritative_world_tick.raw();
            let pools: Vec<_> = prints
                .iter()
                .filter(|(_, n, v)| n.as_str() == "Ground contact" && **v != Visibility::Hidden)
                .map(|(e, _, _)| e)
                .collect();
            writeln!(
                log,
                "{phase}\t{tick}\t{}\t{}",
                ux.settings.paused,
                pools.len()
            )
            .unwrap();
            log.flush().unwrap();
            let mut key = |code| {
                keys.press(code);
                events.write(key_event(code, ButtonState::Pressed));
                release = Some(code);
            };
            let photo = |cmds: &mut Commands, stem: &str| {
                cmds.spawn(Screenshot::primary_window())
                    .observe(save_to_disk(out.join(format!("{stem}.png"))));
            };
            next = now + 1.0;
            match phase {
                0 => key(KeyCode::Escape), // return to habitat
                1 => key(KeyCode::Home),   // find resident
                2 => key(KeyCode::KeyJ),   // place one real rattle
                3 => key(KeyCode::KeyZ),   // encounter lens
                4 => {
                    if pools.len() < 20 || tick == 0 {
                        return;
                    }
                    key(KeyCode::Space);
                }
                5 => {
                    assert!(ux.settings.paused);
                    stopped = tick;
                    let mut aa = aa.single_mut().unwrap();
                    assert!(aa.enabled);
                    aa.enabled = false;
                    for &e in &pools {
                        commands.entity(e).insert(RenderLayers::layer(31));
                    }
                }
                6 => {
                    assert_eq!(tick, stopped);
                    photo(&mut commands, "without-groundlight");
                }
                7 => {
                    assert_eq!(tick, stopped);
                    aa.single_mut().unwrap().enabled = true;
                    for &e in &pools {
                        commands.entity(e).remove::<RenderLayers>();
                    }
                }
                8 => {
                    assert_eq!(tick, stopped);
                    photo(&mut commands, "with-groundlight");
                }
                _ => {
                    if out.join("with-groundlight.png").is_file() {
                        exit.write(AppExit::Success);
                    }
                    return;
                }
            }
            phase += 1;
        })
        .after(InputSystems),
    );
    assert!(
        app.run().is_success() && result.is_file(),
        "both native exposures must finish"
    );
}
