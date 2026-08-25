#![cfg(all(feature = "bevy-app", feature = "gpu-runtime"))]

use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use alife_game_app::bevy_shell::build_production_voxel_frontend_app_shell;
use alife_game_app::{
    default_environment_manifest_path, Fvr05ProductionRightInspectorPanel,
    Fvr05ProductionUxStateResource, ProductionFrontendProfileId, ProductionVoxelLaunchConfig,
    ProductionWorldSource, V0PlayerCreaturePanel,
};
use bevy::prelude::Visibility;

fn production_launch() -> ProductionVoxelLaunchConfig {
    static NEXT_TEST_PATH: AtomicU64 = AtomicU64::new(0);

    let profile = ProductionFrontendProfileId::MinimumSettings30x30;
    let mut launch = ProductionVoxelLaunchConfig::from_manifest(
        default_environment_manifest_path(),
        None,
        profile,
    )
    .expect("production launch fixture must resolve");
    launch.population = Some(4);
    launch.world_source = ProductionWorldSource::NewGame { seed: 240_825 };
    launch.smoke_seconds = None;
    launch.dry_run = true;
    launch.record_performance = false;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time must follow the Unix epoch")
        .as_nanos();
    loop {
        let ordinal = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/artifacts/phase3-capture-tests")
            .join(format!("run-{}-{nonce}-{ordinal}", std::process::id(),));
        if !directory.exists() {
            launch.ui_settings_path = Some(directory.join("ui-settings.json"));
            break;
        }
    }
    launch
}

#[test]
fn fvr05_inspector_chrome_exclusively_owns_the_right_panel_region() {
    let launch = production_launch();
    let (mut app, _summary) =
        build_production_voxel_frontend_app_shell(&launch).expect("production app must build");

    {
        let mut ux = app
            .world_mut()
            .resource_mut::<Fvr05ProductionUxStateResource>();
        ux.settings.show_menu = true;
        ux.settings.show_settings = false;
    }
    app.update();

    let right_visibility = {
        let mut query = app
            .world_mut()
            .query::<(&Fvr05ProductionRightInspectorPanel, &Visibility)>();
        *query
            .iter(app.world())
            .next()
            .map(|(_, visibility)| visibility)
            .expect("FVR05 right inspector must exist")
    };
    let v0_visibility = {
        let mut query = app
            .world_mut()
            .query::<(&V0PlayerCreaturePanel, &Visibility)>();
        *query
            .iter(app.world())
            .next()
            .map(|(_, visibility)| visibility)
            .expect("V0 creature panel must exist")
    };
    assert_eq!(right_visibility, Visibility::Visible);
    assert_eq!(v0_visibility, Visibility::Hidden);

    app.world_mut()
        .resource_mut::<Fvr05ProductionUxStateResource>()
        .settings
        .show_menu = false;
    app.update();

    let right_visibility = {
        let mut query = app
            .world_mut()
            .query::<(&Fvr05ProductionRightInspectorPanel, &Visibility)>();
        *query
            .iter(app.world())
            .next()
            .map(|(_, visibility)| visibility)
            .expect("FVR05 right inspector must exist")
    };
    let v0_visibility = {
        let mut query = app
            .world_mut()
            .query::<(&V0PlayerCreaturePanel, &Visibility)>();
        *query
            .iter(app.world())
            .next()
            .map(|(_, visibility)| visibility)
            .expect("V0 creature panel must exist")
    };
    assert_eq!(right_visibility, Visibility::Hidden);
    assert_eq!(v0_visibility, Visibility::Visible);
}
