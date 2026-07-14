use std::path::PathBuf;

use alife_game_app::{run_platform_package_smoke, PackageSmokeKind};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn fvr08_production_launcher_uses_finished_feature_stack() {
    let root = workspace_root();
    let launcher =
        std::fs::read_to_string(root.join("scripts/run_production_voxel_frontend.ps1")).unwrap();
    let legacy_shell =
        std::fs::read_to_string(root.join("scripts/run_graphical_playground.sh")).unwrap();

    assert!(launcher.contains("A-Life Voxel Frontend"));
    assert!(launcher.contains("[string]$Profile = \"MinSpecComfort1080p\""));
    assert!(launcher.contains("[string]$BrainPolicy = \"gpu-required\""));
    assert!(launcher.contains(
        "$FeatureList = \"bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi\""
    ));
    assert!(launcher.contains("MinimumSettings30x30"));
    assert!(launcher.contains("--record-performance"));
    assert!(!launcher.contains("auto-with-cpu-fallback"));
    assert!(!launcher.contains("gpu-alpha"));

    assert!(legacy_shell.contains("FVR08 compatibility alias"));
    assert!(legacy_shell.contains("run_production_voxel_frontend.ps1"));
    assert!(!legacy_shell.contains("MODE_ARGS=(graphical-playground"));
}

#[test]
fn fvr08_windows_production_package_script_is_product_path() {
    let root = workspace_root();
    let package =
        std::fs::read_to_string(root.join("scripts/package_windows_production_voxel.ps1")).unwrap();
    let package_runner =
        std::fs::read_to_string(root.join("scripts/run_windows_production_voxel_package.ps1"))
            .unwrap();
    let legacy_package =
        std::fs::read_to_string(root.join("scripts/package_windows_alpha.ps1")).unwrap();
    let legacy_runner =
        std::fs::read_to_string(root.join("scripts/run_windows_alpha_package.ps1")).unwrap();

    assert!(package.contains("A-Life FVR08 Windows production voxel package builder"));
    assert!(package.contains("target/artifacts/fvr08_windows_production"));
    assert!(package.contains("alife-production-voxel-windows"));
    assert!(package.contains("alife.fvr08.windows_production_package.v1"));
    assert!(package.contains("bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi"));
    assert!(package.contains("scripts/run_production_voxel_frontend.ps1"));
    assert!(package.contains("crates/alife_game_app/assets/production_voxel_v1"));
    assert!(package.contains(
        "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json"
    ));
    assert!(package.contains("crates/alife_gpu_backend/shaders"));
    assert!(package.contains("LICENSE"));
    assert!(package.contains("README_PACKAGE.md"));
    assert!(package.contains("MinSpecComfort1080p"));
    assert!(package.contains("MinimumSettings30x30"));
    assert!(package.contains("gpu-required"));
    assert!(package.contains("gpu_authority_diagnostics"));
    assert!(!package.contains("auto-with-cpu-fallback"));
    assert!(package.contains("crash_summary.md"));
    assert!(!package.contains("alife-gpu-alpha-windows"));
    assert!(!package.contains("run_windows_alpha_package.ps1"));
    assert!(!package.contains("alpha_art_v1"));
    assert!(!package.contains("true_25d_alpha_v1"));
    assert!(package_runner.contains("Push-Location $PackageRoot"));
    assert!(package_runner.contains("Pop-Location"));
    assert!(package_runner.contains("Save directory policy: package-local"));

    assert!(legacy_package.contains("Legacy regression package"));
    assert!(legacy_runner.contains("Legacy GPU Alpha package runner"));
}

#[test]
fn fvr08_platform_package_smoke_exposes_production_package_commands() {
    let summary = run_platform_package_smoke().unwrap();

    assert!(summary.commands.iter().any(|command| command.id
        == "fvr08-windows-production-voxel-package-dry-run"
        && command.kind == PackageSmokeKind::Validation
        && command
            .windows_command
            .contains("scripts/package_windows_production_voxel.ps1 -DryRun")));
    assert!(summary.commands.iter().any(|command| command.id
        == "fvr08-windows-production-voxel-launcher-dry-run"
        && command.kind == PackageSmokeKind::GraphicalManual
        && command.manual
        && command.requires_graphics
        && command
            .windows_command
            .contains("scripts/run_production_voxel_frontend.ps1 -DryRun")));
    assert!(!summary.commands.iter().any(|command| command
        .windows_command
        .contains("package_windows_alpha.ps1")));
    assert!(!summary.commands.iter().any(|command| command
        .windows_command
        .contains("run_windows_alpha_package.ps1")));
}
