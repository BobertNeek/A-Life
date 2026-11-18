//! GPU smoke playthrough on a newly generated, disposable world. No live save is loaded.
//! Run: cargo run -p alife_game_app --features production-voxel-frontend --example lantern_garden_playtest
#[cfg(not(feature = "production-voxel-frontend"))]
fn main() {
    eprintln!("Enable production-voxel-frontend to run this graphical playtest.");
}

#[cfg(feature = "production-voxel-frontend")]
fn main() {
    use alife_game_app::{
        bevy_shell::build_production_voxel_frontend_app_shell, default_environment_manifest_path,
        ProductionRunMode, ProductionVoxelLaunchConfig, ProductionWorldSource,
    };
    use bevy::{input::InputSystems, prelude::*};
    use std::{path::PathBuf, time::SystemTime};

    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let path = PathBuf::from("target/artifacts/lantern-playtest").join(unique.to_string());
    std::fs::create_dir_all(&path).expect("create isolated playtest output");
    let path = std::fs::canonicalize(path).unwrap();
    let mut launch =
        ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
            .unwrap();
    launch.world_source = ProductionWorldSource::NewGame { seed: 1643 };
    launch.population = Some(1);
    launch.resolution = (960, 540);
    launch.ui_settings_path = Some(path.join("settings.json"));
    launch.run_mode = Some(ProductionRunMode::OneX);
    launch.require_gpu = true;
    launch.smoke_seconds = Some(90);

    // Exercise normal controls, not world mutation APIs or the user's device.
    let (mut app, _) =
        build_production_voxel_frontend_app_shell(&launch).expect("production frontend");
    println!("Playtest output: {}", path.display());
    app.add_systems(
        PreUpdate,
        (move |time: Res<Time<Real>>,
               mut keys: ResMut<ButtonInput<KeyCode>>,
               mut commands: Commands,
               mut step: Local<usize>,
               mut release: Local<Option<KeyCode>>| {
            use bevy::render::view::screenshot::{save_to_disk, Screenshot};
            if let Some(key) = release.take() {
                keys.release(key);
            }
            let sequence = [
                (8., Some(KeyCode::Escape), "menu"),
                (24., None, "garden"),
                (27., Some(KeyCode::KeyE), "offer"),
                (30., Some(KeyCode::KeyC), "pet"),
                (33., None, "response"),
                (36., Some(KeyCode::KeyJ), "toy"),
                (40., None, "play"),
                (43., Some(KeyCode::Enter), "conversation"),
                (48., None, "composer"),
            ];
            if let Some(&(at, key, name)) = sequence.get(*step) {
                if time.elapsed_secs() >= at {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(path.join(format!("{name}.png"))));
                    if let Some(key) = key {
                        keys.press(key);
                        *release = Some(key);
                    }
                    *step += 1;
                }
            }
        })
        .after(InputSystems),
    );
    app.run();
}
