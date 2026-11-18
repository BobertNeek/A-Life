//! Re-open only a disposable orchard_watch save and verify its physical crop identity.
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
    use alife_world::orchard::OrchardBed;
    use bevy::{
        a11y::AccessibilityNode,
        app::AppExit,
        input::InputSystems,
        prelude::*,
        render::view::screenshot::{save_to_disk, Screenshot},
    };
    let root = std::fs::canonicalize("target/artifacts/orchard-watch").unwrap();
    let name = std::env::args()
        .nth(1)
        .expect("pass the numeric disposable run directory");
    assert!(name.len() <= 20 && !name.is_empty() && name.bytes().all(|b| b.is_ascii_digit()));
    let run = std::fs::canonicalize(root.join(name)).unwrap();
    let save = std::fs::canonicalize(run.join("phase3-new-game-1643.json")).unwrap();
    assert!(
        run.starts_with(&root) && save.starts_with(&run),
        "only isolated playtest saves may be loaded"
    );
    let data: serde_json::Value =
        serde_json::from_reader(std::fs::File::open(&save).unwrap()).unwrap();
    let beds: Vec<OrchardBed> =
        serde_json::from_value(data["world"]["ecology"]["orchard_beds"].clone()).unwrap();
    let bed = beds
        .iter()
        .find(|b| b.last_fruit.is_some())
        .expect("saved harvested crop");
    let tile = bed.tile;
    let fruit = bed.last_fruit.unwrap();
    let population =
        u16::try_from(data["creatures"].as_array().expect("saved roster").len()).unwrap();
    drop(data);
    let mut launch =
        ProductionVoxelLaunchConfig::default_from_manifest(default_environment_manifest_path())
            .unwrap();
    launch.world_source = ProductionWorldSource::LoadExisting;
    // Load the real saved roster, including an honestly empty sanctuary.
    launch.population = None;
    launch.app_launch.save_path = save;
    launch.ui_settings_path = Some(run.join("resume-settings.json"));
    launch.require_gpu = true;
    launch.resolution = (960, 540);
    launch.run_mode = Some(ProductionRunMode::OneX);
    launch.smoke_seconds = Some(45);
    let (mut app, _) = build_production_voxel_frontend_app_shell(&launch).unwrap();
    let mut phase = 0u8;
    let mut next = 8.;
    let passed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let observed = passed.clone();
    app.add_systems(
        PreUpdate,
        (move |time: Res<Time<Real>>,
               mut keys: ResMut<ButtonInput<KeyCode>>,
               frame: Res<LiveBrainPresentationFrameResource>,
               selection: Res<Fvr03ProductionVoxelSelectionResource>,
               labels: Query<&AccessibilityNode>,
               mut commands: Commands,
               mut exit: MessageWriter<AppExit>| {
            let now = time.elapsed_secs();
            if now < next {
                return;
            }
            next = now + 1.;
            match phase {
                0 => {
                    keys.press(KeyCode::Escape);
                    phase = 1;
                }
                1 => {
                    keys.release(KeyCode::Escape);
                    if labels.iter().any(|node| {
                        !node.is_hidden() && node.label() == Some("Return to your pear garden")
                    }) {
                        keys.press(KeyCode::KeyU);
                        phase = 2;
                        next = now + 1.5;
                    } else if selection.selected.and_then(|s| s.tile) == Some(tile) {
                        phase = 2;
                    }
                }
                2 => {
                    keys.release(KeyCode::KeyU);
                    assert_eq!(selection.selected.and_then(|s| s.tile), Some(tile));
                    let stage = labels
                        .iter()
                        .filter(|n| !n.is_hidden())
                        .filter_map(|n| n.label())
                        .find(|s| {
                            matches!(
                                *s,
                                "Watch this plant grow"
                                    | "Water these roots"
                                    | "Pick a ripe lantern pear"
                            )
                        });
                    if let Some(stage) = stage {
                        assert_eq!(
                            frame
                                .current
                                .objects()
                                .filter(|o| o.kind == alife_world::WorldObjectKind::Agent)
                                .count(),
                            usize::from(population)
                        );
                        let obj = frame
                            .current
                            .object(fruit)
                            .expect("saved fruit identity in reloaded world");
                        assert_eq!(obj.kind, alife_world::WorldObjectKind::Food);
                        std::fs::write(
                            run.join("resume.tsv"),
                            format!(
                                "restored\t{}\t{}\t{}\t{}\t{}\tresidents={population}\n",
                                frame.current.authoritative_world_tick.raw(),
                                tile.x,
                                tile.z,
                                fruit.raw(),
                                stage
                            ),
                        )
                        .unwrap();
                        println!(
                            "Restored: crop {},{}; physical fruit {}; {}",
                            tile.x,
                            tile.z,
                            fruit.raw(),
                            stage
                        );
                        commands
                            .spawn(Screenshot::primary_window())
                            .observe(save_to_disk(run.join("reloaded.png")));
                        observed.store(true, std::sync::atomic::Ordering::Relaxed);
                        phase = 3;
                        next = now + 2.;
                    }
                }
                _ => {
                    exit.write(AppExit::Success);
                }
            }
        })
        .after(InputSystems),
    );
    assert!(app.run().is_success() && passed.load(std::sync::atomic::Ordering::Relaxed));
}
