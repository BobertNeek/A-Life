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
    pub disable_age_death: bool,
    pub save_path: PathBuf,
    pub asset_root: PathBuf,
    pub config: RuntimeConfig,
    pub assets: AssetManifest,
}

/// Explicit launch choice. Experimental candidates are never the default and
/// loaded individuals always use their saved genome, not this selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum NewGameFounderSelection {
    #[default]
    BuiltinNano512,
    ScaledChoiceNociceptiveV1,
    N2048Candidate {
        asset_path: PathBuf,
    },
}

impl NewGameFounderSelection {
    pub fn parse(value: &str) -> Result<Self, GameAppShellError> {
        match value {
            "builtin-nano512" => Ok(Self::BuiltinNano512),
            "scaled-choice-nociceptive-v1" => Ok(Self::ScaledChoiceNociceptiveV1),
            _ if value.starts_with("n2048:") && value.len() > "n2048:".len() => {
                Ok(Self::N2048Candidate {
                    asset_path: PathBuf::from(&value["n2048:".len()..]),
                })
            }
            _ => Err(invalid_launch("unknown New Game founder selection")),
        }
    }
}

fn scaled_choice_candidate() -> Result<alife_core::Nano512ActionCreditCandidateV2, GameAppShellError>
{
    let asset = FoundationWeightAsset::decode_canonical(include_bytes!(
        "../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
    ))?;
    if asset.digest().bytes()
        != &[
            162, 205, 184, 109, 162, 15, 136, 4, 200, 120, 232, 83, 105, 194, 153, 231, 150, 181,
            202, 200, 230, 97, 59, 164, 174, 244, 52, 65, 167, 166, 238, 30,
        ]
    {
        return Err(invalid_launch("bundled founder candidate digest mismatch"));
    }
    Ok(alife_core::Nano512ActionCreditCandidateV2::new(
        &asset,
        alife_core::ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )?)
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
    stage_phase3_new_game_with_founder(request, NewGameFounderSelection::default())
}

pub fn stage_phase3_new_game_with_founder(
    request: CanonicalNewGameLaunchRequest,
    founder: NewGameFounderSelection,
) -> Result<StagedCanonicalNewGame, GameAppShellError> {
    validate_stage_request(&request)?;

    let mut config = CanonicalNewGameConfig::phase3(request.world_seed, request.population)?;
    let mut game = match founder {
        NewGameFounderSelection::BuiltinNano512 => create_canonical_new_game(
            &config,
            &FoundationWeightAsset::builtin_nano512_v1(SensorProfile::GroundedObjectSlotsV1)?,
        )?,
        NewGameFounderSelection::ScaledChoiceNociceptiveV1 => {
            alife_world::create_canonical_new_game_with_nociceptive_candidate(
                &config,
                &scaled_choice_candidate()?,
            )?
        }
        NewGameFounderSelection::N2048Candidate { asset_path } => {
            config.brain_class = BrainScaleTier::Standard2048;
            let asset = FoundationWeightAsset::decode_canonical(&fs::read(asset_path)?)?;
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset)?
        }
    };
    game.world.enable_highlands_for_new_game()?;
    game.world
        .set_age_death_disabled_for_new_game(request.disable_age_death)?;
    if game.world.organism_registry().len() != usize::from(request.population)
        || game.creatures.len() != usize::from(request.population)
        || game.receipt.founders.len() != usize::from(request.population)
    {
        return Err(invalid_launch(
            "canonical New Game population does not match the requested population",
        ));
    }

    let mut runtime_config = request.config;
    runtime_config.brain_class = config.brain_class;
    let save = PortableSaveFile::from_headless_world(
        format!("phase3-new-game-{}", request.world_seed),
        &game.world,
        runtime_config,
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
    create_canonical_new_game_runtime_with_founder(request, NewGameFounderSelection::default())
}

#[cfg(feature = "gpu-runtime")]
pub fn create_canonical_new_game_runtime_with_founder(
    request: CanonicalNewGameLaunchRequest,
    founder: NewGameFounderSelection,
) -> Result<CanonicalNewGameLaunchResult, GameAppShellError> {
    create_canonical_new_game_runtime_inner(request, founder, false)
}

#[cfg(feature = "gpu-tests")]
pub fn create_canonical_new_game_runtime_with_forced_late_failure_for_test(
    request: CanonicalNewGameLaunchRequest,
) -> Result<CanonicalNewGameLaunchResult, GameAppShellError> {
    create_canonical_new_game_runtime_inner(request, NewGameFounderSelection::default(), true)
}

#[cfg(feature = "gpu-runtime")]
fn create_canonical_new_game_runtime_inner(
    request: CanonicalNewGameLaunchRequest,
    founder: NewGameFounderSelection,
    force_late_failure_for_test: bool,
) -> Result<CanonicalNewGameLaunchResult, GameAppShellError> {
    let brain_class = match &founder {
        NewGameFounderSelection::N2048Candidate { .. } => BrainScaleTier::Standard2048,
        _ => BrainScaleTier::Nano512,
    };
    let staged = stage_phase3_new_game_with_founder(request, founder)?;
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
            brain_class,
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
    if !(alife_world::PHASE3_MIN_POPULATION..=alife_world::PHASE3_MAX_POPULATION)
        .contains(&request.population)
    {
        return Err(invalid_launch(&format!(
            "New Game requires {} to {} creatures; requested {}",
            alife_world::PHASE3_MIN_POPULATION,
            alife_world::PHASE3_MAX_POPULATION,
            request.population,
        )));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{
        BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, OrganismId,
        PhenotypeCompiler, Tick, TrainingStageManifest,
    };

    #[test]
    fn n2048_new_game_save_reload_preserves_exact_candidate_and_class() {
        let seed = 240_826;
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(seed, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let native = PhenotypeCompiler::compile_testing_procedural_baseline(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let source = FoundationWeightAsset::from_phenotype_for_genetic_birth(&native).unwrap();
        let (baseline, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome,
            development,
            source.clone(),
        )
        .unwrap();
        let mut weights = source.weights().to_vec();
        weights[0] = f32::from_bits(weights[0].to_bits() ^ 1);
        let candidate = FoundationWeightAsset::from_trained_weights(
            &baseline,
            weights,
            TrainingStageManifest::bootstrap(),
        )
        .unwrap();
        assert_ne!(candidate.digest(), source.digest());
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "alife-n2048-new-game-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&root).unwrap();
        let candidate_path = root.join("candidate.alife-foundation");
        fs::write(&candidate_path, candidate.encode_canonical().unwrap()).unwrap();
        let mut config = RuntimeConfig::deterministic_default(seed, BrainScaleTier::Nano512);
        config.features.gpu_backend_enabled = true;
        let staged = stage_phase3_new_game_with_founder(
            CanonicalNewGameLaunchRequest {
                world_seed: seed,
                population: 1,
                disable_age_death: true,
                save_path: root.join("world.json"),
                asset_root: root.clone(),
                config,
                assets: AssetManifest::empty(),
            },
            NewGameFounderSelection::parse(&format!("n2048:{}", candidate_path.display())).unwrap(),
        )
        .unwrap();
        assert_eq!(staged.save.config.brain_class, BrainScaleTier::Standard2048);
        assert_eq!(
            staged.save.creatures[0].brain_class,
            BrainScaleTier::Standard2048
        );
        staged.save.to_json_file(&staged.save_path).unwrap();
        // Loading must depend on the saved genome, not the original CLI asset path.
        fs::remove_file(&candidate_path).unwrap();
        let loaded = PortableSaveFile::from_json_file(&staged.save_path).unwrap();
        loaded.validate_with_asset_root(&root).unwrap();
        assert_eq!(loaded.config.brain_class, BrainScaleTier::Standard2048);
        let world = loaded.restore_headless_world().unwrap();
        assert!(world.age_death_disabled());
        assert_eq!(
            world.canonical_signature_digest().unwrap(),
            staged.world.canonical_signature_digest().unwrap()
        );
        let admission = world
            .organism_registry()
            .get(OrganismId(1))
            .unwrap()
            .authoritative_admission_at(world.tick())
            .unwrap();
        let restored_asset = admission.genome.n2048_foundation_candidate.unwrap();
        assert_eq!(
            restored_asset.encode_canonical().unwrap(),
            candidate.encode_canonical().unwrap()
        );
        let brain_genome = admission.phenotype.brain_genome;
        let development = DevelopmentState::new(
            brain_genome.id,
            Tick::ZERO,
            NormalizedScalar::new(1.0).unwrap(),
        );
        let (compiled, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            brain_genome,
            development,
            restored_asset,
        )
        .unwrap();
        assert_eq!(compiled.brain_class_id(), BrainCapacityClass::N2048_ID);
        assert_eq!(
            compiled
                .synapses()
                .iter()
                .map(|synapse| synapse.genetic_weight().to_bits())
                .collect::<Vec<_>>(),
            candidate
                .weights()
                .iter()
                .map(|weight| weight.to_bits())
                .collect::<Vec<_>>()
        );
        fs::remove_file(&staged.save_path).unwrap();
        fs::remove_dir(&root).unwrap();
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
            ));
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
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
