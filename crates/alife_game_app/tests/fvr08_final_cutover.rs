use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn fvr08_production_launcher_uses_finished_feature_stack() {
    let root = workspace_root();
    let launcher =
        std::fs::read_to_string(root.join("scripts/run_production_voxel_frontend.ps1")).unwrap();

    assert!(launcher.contains("A-Life Voxel Frontend"));
    assert!(launcher.contains("[string]$Profile = \"MinSpecComfort1080p\""));
    assert!(launcher.contains("[string]$BrainPolicy = \"gpu-required\""));
    assert!(
        launcher.contains("$FeatureList = \"bevy-app gpu-runtime production-assets vfx-hanabi\"")
    );
    assert!(launcher.contains("MinimumSettings30x30"));
    assert!(launcher.contains("--record-performance"));
    assert!(!launcher.contains("auto-with-cpu-fallback"));
    assert!(!launcher.contains("gpu-alpha"));

    assert!(!root.join("scripts/run_graphical_playground.sh").exists());
}

#[test]
fn fvr08_windows_production_package_script_is_product_path() {
    let root = workspace_root();
    let package =
        std::fs::read_to_string(root.join("scripts/package_windows_production_voxel.ps1")).unwrap();
    let package_runner =
        std::fs::read_to_string(root.join("scripts/run_windows_production_voxel_package.ps1"))
            .unwrap();

    assert!(package.contains("A-Life FVR08 Windows production voxel package builder"));
    assert!(package.contains("target/artifacts/fvr08_windows_production"));
    assert!(package.contains("alife-production-voxel-windows"));
    assert!(package.contains("alife.fvr08.windows_production_package.v1"));
    assert!(package.contains("bevy-app gpu-runtime production-assets vfx-hanabi"));
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

    assert!(!root.join("scripts/package_windows_alpha.ps1").exists());
    assert!(!root.join("scripts/run_windows_alpha_package.ps1").exists());
}
