use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[cfg(feature = "gpu-runtime")]
use alife_archive::LineageLibraryConfig;
#[cfg(feature = "gpu-runtime")]
use alife_core::ArchiveLearnedCapturePolicy;
use alife_core::{BrainScaleTier, FoundationWeightAsset, PolicyBackend, SensorProfile};
#[cfg(feature = "gpu-runtime")]
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_world::{
    create_canonical_new_game, AssetManifest, CanonicalNewGameConfig, CanonicalNewGameReceipt,
    HeadlessWorld, PortableSaveFile, RuntimeConfig,
};

use crate::GameAppShellError;
#[cfg(feature = "gpu-runtime")]
use crate::{GpuDurableSaveManifest, GpuLiveBrainRuntime};

#[derive(Debug, Clone)]
pub struct CanonicalNewGameLaunchRequest {
    pub world_seed: u64,
    pub population: u16,
    pub save_path: PathBuf,
    pub asset_root: PathBuf,
    pub config: RuntimeConfig,
    pub assets: AssetManifest,
}

#[derive(Debug, Clone)]
pub struct StagedCanonicalNewGame {
    pub world: HeadlessWorld,
    pub save: PortableSaveFile,
    pub receipt: CanonicalNewGameReceipt,
    pub save_path: PathBuf,
    pub asset_root: PathBuf,
}

#[cfg(feature = "gpu-runtime")]
pub struct CanonicalNewGameLaunchResult {
    pub runtime: GpuLiveBrainRuntime,
    pub exact_save: PortableSaveFile,
    pub save_path: PathBuf,
    pub asset_root: PathBuf,
    pub receipt: CanonicalNewGameReceipt,
}

pub fn stage_phase3_new_game(
    request: CanonicalNewGameLaunchRequest,
) -> Result<StagedCanonicalNewGame, GameAppShellError> {
    validate_stage_request(&request)?;

    let foundation =
        FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let game = create_canonical_new_game(
        &CanonicalNewGameConfig::phase3(request.world_seed, request.population)?,
        &foundation,
    )?;
    if game.world.organism_registry().len() != usize::from(request.population)
        || game.creatures.len() != usize::from(request.population)
        || game.receipt.founders.len() != usize::from(request.population)
    {
        return Err(invalid_launch(
            "canonical New Game population does not match the requested population",
        ));
    }

    let save = PortableSaveFile::from_headless_world(
        format!("phase3-new-game-{}", request.world_seed),
        &game.world,
        request.config,
        request.assets,
        game.creatures,
    )?;
    save.validate_with_asset_root(&request.asset_root)?;

    Ok(StagedCanonicalNewGame {
        world: game.world,
        save,
        receipt: game.receipt,
        save_path: request.save_path,
        asset_root: request.asset_root,
    })
}

#[cfg(feature = "gpu-runtime")]
pub fn create_canonical_new_game_runtime(
    request: CanonicalNewGameLaunchRequest,
) -> Result<CanonicalNewGameLaunchResult, GameAppShellError> {
    create_canonical_new_game_runtime_inner(request, false)
}

#[cfg(feature = "gpu-tests")]
pub fn create_canonical_new_game_runtime_with_forced_late_failure_for_test(
    request: CanonicalNewGameLaunchRequest,
) -> Result<CanonicalNewGameLaunchResult, GameAppShellError> {
    create_canonical_new_game_runtime_inner(request, true)
}

#[cfg(feature = "gpu-runtime")]
fn create_canonical_new_game_runtime_inner(
    request: CanonicalNewGameLaunchRequest,
    force_late_failure_for_test: bool,
) -> Result<CanonicalNewGameLaunchResult, GameAppShellError> {
    let staged = stage_phase3_new_game(request)?;
    let staging_path = staging_save_path(&staged.save_path)?;
    let archive_root = lineage_archive_root(&staged.save_path)?;
    if staging_path.exists() || archive_root.exists() {
        return Err(invalid_launch(
            "canonical New Game staging or lineage target already exists",
        ));
    }

    let population = usize::from(staged.receipt.requested_population);
    let final_save_path = staged.save_path.clone();
    let asset_root = staged.asset_root.clone();
    let gpu_assets_before = NewGameGpuAssetSnapshot::capture(&asset_root)?;
    let mut final_created = false;
    let result = (|| {
        let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())
            .map_err(|error| GameAppShellError::NeuralBackendUnavailable {
                message: error.to_string(),
            })?;
        let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
            backend,
            staged.world,
            staged.save.deterministic_seed,
            BrainScaleTier::Nano512,
            SensorProfile::GroundedObjectSlotsV1,
            LineageLibraryConfig::profile_default(&archive_root),
            format!("phase3-new-game-{}", staged.save.deterministic_seed),
            ArchiveLearnedCapturePolicy::GeneticOnly,
        )?;
        let residency = runtime.residency_summary();
        if residency.handle_count != population
            || residency.resident_count != population
            || residency.memory_sidecar_count != population
            || residency.topology_sidecar_count != population
            || runtime.lineage_archive_manifest_count()? != Some(population as u64)
        {
            return Err(invalid_launch(
                "canonical New Game did not admit every founder to complete GPU residency",
            ));
        }

        let mut base_save = staged.save;
        base_save.replace_headless_world_snapshot(&runtime.world_snapshot())?;
        base_save.validate_with_asset_root(&asset_root)?;
        runtime.attach_durable_checkpoint_boundary(&staging_path, &asset_root, base_save)?;
        let exact_save = runtime.capture_portable_checkpoint()?;
        if exact_save.creatures.len() != population
            || exact_save
                .creatures
                .iter()
                .any(|creature| creature.gpu_brain.is_none())
        {
            return Err(invalid_launch(
                "canonical New Game exact checkpoint is missing a resident GPU brain",
            ));
        }
        exact_save.validate_with_asset_root(&asset_root)?;
        let staged_exact =
            GpuDurableSaveManifest::publish_snapshot(&staging_path, &asset_root, &exact_save)?;
        if staged_exact.save != exact_save {
            return Err(invalid_launch(
                "canonical New Game staging reload differs from the exact checkpoint",
            ));
        }

        std::fs::rename(&staging_path, &final_save_path)?;
        final_created = true;
        runtime.rebind_durable_checkpoint_boundary(&final_save_path, &asset_root, &exact_save)?;
        let (_, final_loaded) = GpuDurableSaveManifest::open_loaded(&final_save_path, &asset_root)?;
        if final_loaded.save != exact_save {
            return Err(invalid_launch(
                "canonical New Game final reload differs from the exact checkpoint",
            ));
        }
        if force_late_failure_for_test {
            return Err(invalid_launch(
                "test-forced late canonical New Game failure",
            ));
        }

        Ok(CanonicalNewGameLaunchResult {
            runtime,
            exact_save: final_loaded.save,
            save_path: final_save_path.clone(),
            asset_root: asset_root.clone(),
            receipt: staged.receipt,
        })
    })();

    match result {
        Ok(result) => Ok(result),
        Err(error) => {
            if let Err(rollback_error) = rollback_new_game_artifacts(
                &staging_path,
                final_created.then_some(final_save_path.as_path()),
                &archive_root,
                &gpu_assets_before,
            ) {
                return Err(invalid_launch(&format!(
                    "canonical New Game failed: {error}; rollback failed: {rollback_error}"
                )));
            }
            Err(error)
        }
    }
}

fn validate_stage_request(
    request: &CanonicalNewGameLaunchRequest,
) -> Result<(), GameAppShellError> {
    request.config.validate()?;
    if request.save_path.as_os_str().is_empty() || request.asset_root.as_os_str().is_empty() {
        return Err(invalid_launch("save path and asset root are required"));
    }
    if request.save_path.exists() {
        return Err(invalid_launch(
            "canonical New Game refuses to replace an existing save",
        ));
    }
    if request.config.deterministic_seed != request.world_seed
        || request.config.brain_class != BrainScaleTier::Nano512
        || request.config.brain_policy.policy != PolicyBackend::NeuralClosedLoopGpu
        || !request.config.features.gpu_backend_enabled
        || request.config.features.school_enabled
        || request.config.school.teacher_enabled
    {
        return Err(invalid_launch(
            "canonical New Game requires matching Nano512 GPU-only runtime configuration",
        ));
    }
    Ok(())
}

fn invalid_launch(message: &str) -> GameAppShellError {
    GameAppShellError::InvalidProductionFrontend {
        message: message.to_string(),
    }
}

#[cfg(feature = "gpu-runtime")]
fn staging_save_path(save_path: &std::path::Path) -> Result<PathBuf, GameAppShellError> {
    let file_name = save_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid_launch("canonical New Game save path requires a UTF-8 file name"))?;
    Ok(save_path.with_file_name(format!(".{file_name}.phase3-staging")))
}

#[cfg(feature = "gpu-runtime")]
fn lineage_archive_root(save_path: &std::path::Path) -> Result<PathBuf, GameAppShellError> {
    let parent = save_path
        .parent()
        .ok_or_else(|| invalid_launch("canonical New Game save path requires a parent"))?;
    let stem = save_path
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| invalid_launch("canonical New Game save path requires a UTF-8 stem"))?;
    Ok(parent.join(format!(".{stem}.lineage")))
}

#[cfg(feature = "gpu-runtime")]
struct NewGameGpuAssetSnapshot {
    gpu_root: PathBuf,
    root_existed: bool,
    existing_paths: BTreeSet<PathBuf>,
}

#[cfg(feature = "gpu-runtime")]
impl NewGameGpuAssetSnapshot {
    fn capture(asset_root: &Path) -> Result<Self, GameAppShellError> {
        let asset_root = fs::canonicalize(asset_root)?;
        let gpu_root = asset_root.join("gpu-brain");
        let root_existed = gpu_root.exists();
        let existing_paths = collect_tree_paths(&gpu_root)?;
        Ok(Self {
            gpu_root,
            root_existed,
            existing_paths,
        })
    }

    fn rollback(&self) -> Result<(), GameAppShellError> {
        if !self.gpu_root.exists() {
            return Ok(());
        }
        if !self.root_existed {
            fs::remove_dir_all(&self.gpu_root)?;
            return Ok(());
        }
        let mut created = collect_tree_paths(&self.gpu_root)?
            .difference(&self.existing_paths)
            .cloned()
            .collect::<Vec<_>>();
        created.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
        for path in created {
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                fs::remove_dir(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        Ok(())
    }
}

#[cfg(feature = "gpu-runtime")]
fn collect_tree_paths(root: &Path) -> Result<BTreeSet<PathBuf>, GameAppShellError> {
    let mut paths = BTreeSet::new();
    if !root.exists() {
        return Ok(paths);
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            paths.insert(path.clone());
            if file_type.is_dir() && !file_type.is_symlink() {
                pending.push(path);
            }
        }
    }
    Ok(paths)
}

#[cfg(feature = "gpu-runtime")]
fn rollback_new_game_artifacts(
    staging_path: &Path,
    final_save_path: Option<&Path>,
    archive_root: &Path,
    gpu_assets: &NewGameGpuAssetSnapshot,
) -> Result<(), GameAppShellError> {
    let mut failures = Vec::new();
    for path in [Some(staging_path), final_save_path].into_iter().flatten() {
        if let Err(error) = remove_transaction_file(path) {
            failures.push(error.to_string());
        }
    }
    if let Err(error) = remove_transaction_tree(archive_root) {
        failures.push(error.to_string());
    }
    if let Err(error) = gpu_assets.rollback() {
        failures.push(error.to_string());
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(invalid_launch(&failures.join("; ")))
    }
}

#[cfg(feature = "gpu-runtime")]
fn remove_transaction_file(path: &Path) -> Result<(), GameAppShellError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() || metadata.file_type().is_symlink() => {
            fs::remove_file(path)?;
        }
        Ok(_) => {
            return Err(invalid_launch(
                "New Game manifest rollback target is not a file",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[cfg(feature = "gpu-runtime")]
fn remove_transaction_tree(path: &Path) -> Result<(), GameAppShellError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            fs::remove_dir_all(path)?;
        }
        Ok(metadata) if metadata.file_type().is_symlink() => fs::remove_file(path)?,
        Ok(_) => {
            return Err(invalid_launch(
                "New Game lineage rollback target is not a directory",
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
