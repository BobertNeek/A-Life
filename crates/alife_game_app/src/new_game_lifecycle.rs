use std::path::PathBuf;

use alife_core::{BrainScaleTier, FoundationWeightAsset, PolicyBackend, SensorProfile};
use alife_world::{
    create_canonical_new_game, AssetManifest, CanonicalNewGameConfig, CanonicalNewGameReceipt,
    HeadlessWorld, PortableSaveFile, RuntimeConfig,
};

use crate::GameAppShellError;

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
