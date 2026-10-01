use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::BrainScaleTier;
use alife_game_app::{
    default_environment_manifest_path, EnvironmentManifest, ProductionVoxelLaunchConfig,
};
use alife_world::RuntimeConfig;

#[path = "untrained_population_save.rs"]
mod untrained_population_save;

/// A current file-backed launch input, independent of the historical saved
/// source. Keep this owner alive until the launch and its output checks finish.
pub struct CurrentProductionLaunchFixture {
    _root: FixtureRoot,
    pub launch: ProductionVoxelLaunchConfig,
}

struct FixtureRoot(PathBuf);

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl CurrentProductionLaunchFixture {
    pub fn new() -> Self {
        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        // This guard also cleans up if fixture construction panics.
        let root = FixtureRoot(std::env::temp_dir().join(format!(
            "alife-current-production-launch-{}-{nonce}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        )));
        let manifest_dir = root.0.join("crates/alife_game_app");
        let scene_root = manifest_dir.join("current_scene");
        fs::create_dir_all(&scene_root).unwrap();
        let mut manifest: EnvironmentManifest =
            serde_json::from_slice(&fs::read(default_environment_manifest_path()).unwrap())
                .unwrap();
        let entry = manifest
            .scenarios
            .iter_mut()
            .find(|entry| entry.id == "production-voxel")
            .unwrap();
        entry.fixture_root = PathBuf::from("current_scene");
        let mut config = RuntimeConfig::deterministic_default(4242, BrainScaleTier::Nano512);
        config.features.gpu_backend_enabled = true;
        // Both profiles load an existing population of 30 fresh founders. This
        // does not alter the separate Phase 3 New Game cohort limit.
        let save = untrained_population_save::untrained_population_save(config, 30);
        save.validate_with_asset_root(&scene_root).unwrap();
        save.to_json_file(scene_root.join(&entry.save_file))
            .unwrap();
        fs::write(
            scene_root.join(&entry.config_file),
            serde_json::to_vec_pretty(&save.config).unwrap(),
        )
        .unwrap();
        fs::write(
            scene_root.join(&entry.asset_manifest_file),
            serde_json::to_vec_pretty(&save.assets).unwrap(),
        )
        .unwrap();
        let manifest_path = manifest_dir.join("environment_manifest.json");
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let launch = ProductionVoxelLaunchConfig::default_from_manifest(&manifest_path).unwrap();
        Self {
            _root: root,
            launch,
        }
    }
}
