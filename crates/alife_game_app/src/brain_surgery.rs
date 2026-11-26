//! Fail-closed weight programming and V11 branch surgery bundles, patches,
//! matched scorecards, and separate-process reopen verification.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use alife_archive::LineageLibraryConfig;
use alife_core::{
    ArchiveLearnedCapturePolicy, BrainCapacityClass, BrainPhenotype, BrainScaleTier,
    CompiledSynapseKind, DendriticBranch, DendriticBranchSet, DendriticInputRef, OrganismId,
    PhenotypeCompiler, PolicyBackend, ProjectionType, ScaffoldContractError, SensorProfile, Tick,
};
use alife_gpu_backend::{GpuClosedLoopBackend, GpuRuntimeProfile};
use alife_runtime::{GpuCheckpointAssetStore, GpuDurableSaveManifest};
use alife_world::{
    create_canonical_new_game_with_n2048_candidate, AssetManifest, CanonicalNewGameConfig,
    PortableSaveFile, RuntimeConfig,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    initial_n2048_care_asset, run_foundation_evaluation_pilot_with_v11_branches,
    run_foundation_request_evaluation_with_v11_branches, AppShellLaunchConfig,
    FoundationEvaluationConfig, FoundationEvaluationRequest, FoundationFoodNeedCondition,
    FoundationPilotReceipt, FoundationTeacherLesson, GpuLiveBrainRuntime,
};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub const BRAIN_SURGERY_BUNDLE_SCHEMA: &str = "alife.brain_surgery.base_bundle.v1";
pub const BRAIN_SURGERY_PATCH_SCHEMA: &str = "alife.brain_surgery.patch.v1";
pub const BRAIN_SURGERY_SCORECARD_SCHEMA: &str = "alife.brain_surgery.scorecard.v1";
pub const BRAIN_SURGERY_REPORT_SCHEMA: &str = "alife.brain_surgery.comparison_report.v1";
pub const BRAIN_SURGERY_REOPEN_SCHEMA: &str = "alife.brain_surgery.reopen_receipt.v1";
pub const BRAIN_SURGERY_SCHEMA_VERSION: u16 = 1;
pub const N2048_NEURON_COUNT: u32 = 2048;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrainSurgeryBaseBundleManifestV1 {
    pub schema: String,
    pub schema_version: u16,
    pub base_id: String,
    pub parent_base_id: Option<String>,
    pub foundation_file: String,
    pub foundation_sha256: String,
    pub foundation_digest: String,
    pub sensor_profile: SensorProfile,
    pub weight_count: usize,
    pub dendritic_branches_file: String,
    pub dendritic_branches_sha256: String,
    pub dendritic_branch_count: usize,
    pub semantic_prior_mode: String,
    pub semantic_prior_context_contract: String,
    pub scorecard_file: String,
    pub scorecard_sha256: String,
    pub reopen_verified: bool,
    pub reopen_receipt_file: Option<String>,
    pub reopen_receipt_sha256: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BrainSurgeryBaseBundle {
    pub root: PathBuf,
    pub manifest: BrainSurgeryBaseBundleManifestV1,
    pub asset: alife_core::FoundationWeightAsset,
    pub dendritic_branches: DendriticBranchSet,
    pub scorecard: BrainSurgeryScorecardV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightEditMode {
    Set,
    Add,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeightEdit {
    pub index: usize,
    pub mode: WeightEditMode,
    pub value: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
#[derive(Default)]
pub enum DendriticPatchOp {
    #[default]
    Keep,
    ReplaceAll {
        branches: DendriticBranchSet,
    },
    UpsertTargets {
        #[serde(default)]
        remove_targets: Vec<u32>,
        #[serde(default)]
        add_branches: Vec<DendriticBranch>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrainSurgeryPatchV1 {
    pub schema: String,
    pub schema_version: u16,
    pub patch_id: String,
    pub target_base_id: String,
    pub target_foundation_digest: String,
    pub target_dendritic_branches_sha256: String,
    #[serde(default)]
    pub weight_edits: Vec<WeightEdit>,
    #[serde(default)]
    pub dendritic_edits: DendriticPatchOp,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchApplicationReceipt {
    pub patch_id: String,
    pub source_base_id: String,
    pub candidate_base_id: String,
    pub source_foundation_digest: String,
    pub candidate_foundation_digest: String,
    pub source_dendritic_branches_sha256: String,
    pub candidate_dendritic_branches_sha256: String,
    pub weight_edits_applied: usize,
    pub dendritic_branch_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScorecardCase {
    pub id: String,
    pub lesson: FoundationTeacherLesson,
    pub world_seed: u64,
    pub founder_seed_base: u64,
    pub ticks: usize,
    #[serde(default = "default_case_weight")]
    pub weight: f64,
    #[serde(default)]
    pub food_position: Option<[f32; 2]>,
    #[serde(default = "default_temperature")]
    pub sampling_temperature: f32,
    #[serde(default)]
    pub silent_request: bool,
    #[serde(default)]
    pub request_token: Option<u16>,
    #[serde(default)]
    pub food_need_condition: FoundationFoodNeedCondition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrainSurgeryScorecardV1 {
    pub schema: String,
    pub schema_version: u16,
    pub scorecard_id: String,
    pub cases: Vec<ScorecardCase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaseComparisonResult {
    pub case_id: String,
    pub lesson: FoundationTeacherLesson,
    pub weight: f64,
    pub base_score: f64,
    pub candidate_score: f64,
    pub delta: f64,
    pub base_completed: Option<bool>,
    pub candidate_completed: Option<bool>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrainSurgeryComparisonReportV1 {
    pub schema: String,
    pub schema_version: u16,
    pub scorecard_id: String,
    pub base_id: String,
    pub candidate_id: String,
    pub candidate_patch_id: Option<String>,
    pub base_foundation_digest: String,
    pub candidate_foundation_digest: String,
    pub base_dendritic_sha256: String,
    pub candidate_dendritic_sha256: String,
    pub base_average: f64,
    pub candidate_average: f64,
    pub delta: f64,
    pub min_improvement: f64,
    pub improved_cases: usize,
    pub regressed_cases: Vec<String>,
    pub unchanged_cases: usize,
    pub failed_cases: usize,
    pub promoted: bool,
    pub results: Vec<CaseComparisonResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReopenVerificationReceiptV1 {
    pub schema: String,
    pub schema_version: u16,
    pub base_id: String,
    pub foundation_digest: String,
    pub dendritic_branches_sha256: String,
    pub dendritic_branch_count: usize,
    pub tick: u64,
    pub parent_pid: u32,
    pub child_pid: u32,
    pub verified: bool,
    pub semantic_prior_mode: String,
    pub verified_at_unix: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExpectedReopenState {
    base_id: String,
    foundation_digest: String,
    dendritic_branches: DendriticBranchSet,
    dendritic_branches_sha256: String,
    tick: u64,
    seed: u64,
}

fn default_case_weight() -> f64 {
    1.0
}

fn default_temperature() -> f32 {
    1.0
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn blake3_hex(digest: &alife_core::Blake3Digest) -> String {
    digest.bytes().iter().map(|b| format!("{b:02x}")).collect()
}

fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn validate_identifier(label: &str, value: &str) -> Result<()> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed != value
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(format!("invalid {label} identifier: {value:?}").into());
    }
    Ok(())
}

/// Returns the default starter scorecard spanning feeding, hazard avoidance,
/// maze routing, reachable foraging, and grounded vocabulary reception.
pub fn default_starter_scorecard() -> BrainSurgeryScorecardV1 {
    BrainSurgeryScorecardV1 {
        schema: BRAIN_SURGERY_SCORECARD_SCHEMA.to_string(),
        schema_version: BRAIN_SURGERY_SCHEMA_VERSION,
        scorecard_id: "starter-care-maze-vocab-v1".to_string(),
        cases: vec![
            ScorecardCase {
                id: "feeding-core".into(),
                lesson: FoundationTeacherLesson::Feeding,
                world_seed: 700_001,
                founder_seed_base: 700_001,
                ticks: 64,
                weight: 1.0,
                food_position: None,
                sampling_temperature: 1.0,
                silent_request: false,
                request_token: None,
                food_need_condition: FoundationFoodNeedCondition::default(),
            },
            ScorecardCase {
                id: "hazard-avoidance-core".into(),
                lesson: FoundationTeacherLesson::HazardAvoidance,
                world_seed: 700_002,
                founder_seed_base: 700_001,
                ticks: 64,
                weight: 1.0,
                food_position: None,
                sampling_temperature: 1.0,
                silent_request: false,
                request_token: None,
                food_need_condition: FoundationFoodNeedCondition::default(),
            },
            ScorecardCase {
                id: "obstacle-nav-core".into(),
                lesson: FoundationTeacherLesson::ObstacleNavigation,
                world_seed: 700_003,
                founder_seed_base: 700_001,
                ticks: 96,
                weight: 1.0,
                food_position: None,
                sampling_temperature: 1.0,
                silent_request: false,
                request_token: None,
                food_need_condition: FoundationFoodNeedCondition::default(),
            },
            ScorecardCase {
                id: "maze-u-turn-core".into(),
                lesson: FoundationTeacherLesson::MazeNavigation,
                world_seed: 700_004,
                founder_seed_base: 700_001,
                ticks: 128,
                weight: 1.0,
                food_position: None,
                sampling_temperature: 1.0,
                silent_request: false,
                request_token: None,
                food_need_condition: FoundationFoodNeedCondition::default(),
            },
            ScorecardCase {
                id: "eat-reachable-core".into(),
                lesson: FoundationTeacherLesson::EatReachableFood,
                world_seed: 700_005,
                founder_seed_base: 700_001,
                ticks: 64,
                weight: 1.0,
                food_position: None,
                sampling_temperature: 1.0,
                silent_request: false,
                request_token: None,
                food_need_condition: FoundationFoodNeedCondition::default(),
            },
            ScorecardCase {
                id: "vocab-food-core".into(),
                lesson: FoundationTeacherLesson::VocabularyReception,
                world_seed: 700_006,
                founder_seed_base: 700_001,
                ticks: 32,
                weight: 1.0,
                food_position: None,
                sampling_temperature: 1.0,
                silent_request: false,
                request_token: Some(1),
                food_need_condition: FoundationFoodNeedCondition::default(),
            },
        ],
    }
}

pub fn validate_scorecard(scorecard: &BrainSurgeryScorecardV1) -> Result<()> {
    if scorecard.schema != BRAIN_SURGERY_SCORECARD_SCHEMA
        || scorecard.schema_version != BRAIN_SURGERY_SCHEMA_VERSION
    {
        return Err("unsupported scorecard schema or version".into());
    }
    validate_identifier("scorecard_id", &scorecard.scorecard_id)?;
    if scorecard.cases.is_empty() {
        return Err("scorecard must contain at least one case".into());
    }
    let mut seen_ids = BTreeSet::new();
    for case in &scorecard.cases {
        validate_identifier("case.id", &case.id)?;
        if !seen_ids.insert(case.id.clone()) {
            return Err(format!("duplicate scorecard case id: {}", case.id).into());
        }
        if case.world_seed == 0
            || case.founder_seed_base == 0
            || !(1..=2048).contains(&case.ticks)
            || !case.weight.is_finite()
            || case.weight <= 0.0
        {
            return Err(format!("invalid scorecard case parameters: {}", case.id).into());
        }
        if let Some(token) = case.request_token {
            if case.lesson != FoundationTeacherLesson::VocabularyReception
                || !matches!(token, 1 | 2 | 9 | 13 | 15 | 16)
            {
                return Err(format!(
                    "case {} sets invalid request_token {token} for lesson {:?}",
                    case.id, case.lesson
                )
                .into());
            }
        }
        FoundationEvaluationConfig {
            food_need_condition: case.food_need_condition,
            food_position: case.food_position,
            sampling_temperature: case.sampling_temperature,
            silent_request: case.silent_request,
        }
        .validate()?;
    }
    Ok(())
}

fn compile_n2048_phenotype(asset: &alife_core::FoundationWeightAsset) -> Result<BrainPhenotype> {
    if asset.manifest().capacity_class_id() != BrainCapacityClass::N2048_ID {
        return Err("brain surgery bundles require N2048 foundation assets".into());
    }
    let mut config = CanonicalNewGameConfig::phase3(1, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = 1;
    config.sensor_profile = asset.manifest().sensor_profile();
    let game = create_canonical_new_game_with_n2048_candidate(&config, asset)?;
    let record = game
        .world
        .organism_registry()
        .get(OrganismId(1))
        .ok_or("compiled N2048 world missing founder OrganismId(1)")?;
    let brain_genome = record.phenotype().brain_genome.clone();
    let capacity = BrainCapacityClass::n2048();
    let development = crate::gpu_live_runtime::foundation_construction_development(
        &brain_genome,
        &capacity,
        &record
            .phenotype()
            .development_state_at(record.age_at(game.world.tick())?)?,
    )?;
    let (phenotype, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        brain_genome,
        development,
        asset.clone(),
    )?;
    Ok(phenotype)
}

fn normalize_target_path(path: &Path) -> Result<PathBuf> {
    let abs = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let parent = abs
        .parent()
        .ok_or_else(|| format!("path must have a parent directory: {}", path.display()))?;
    std::fs::create_dir_all(parent)?;
    let canon_parent = parent.canonicalize()?;
    let name = abs
        .file_name()
        .ok_or_else(|| format!("path must name a leaf directory: {}", path.display()))?;
    Ok(canon_parent.join(name))
}

fn ensure_absent_output_dir(output_dir: &Path, forbidden_source: Option<&Path>) -> Result<PathBuf> {
    if output_dir.exists() {
        return Err(format!(
            "refusing to overwrite existing bundle directory: {}",
            output_dir.display()
        )
        .into());
    }
    let normalized_out = normalize_target_path(output_dir)?;
    if normalized_out.exists() {
        return Err(format!(
            "refusing to overwrite existing bundle directory: {}",
            output_dir.display()
        )
        .into());
    }
    if let Some(source) = forbidden_source {
        let canon_src = source.canonicalize()?;
        if normalized_out == canon_src
            || normalized_out.starts_with(&canon_src)
            || canon_src.starts_with(&normalized_out)
        {
            return Err(format!(
                "refusing bundle output {} overlapping source bundle {}",
                output_dir.display(),
                source.display()
            )
            .into());
        }
    }
    Ok(normalized_out)
}

#[allow(clippy::too_many_arguments)]
fn write_base_bundle(
    output_dir: &Path,
    base_id: &str,
    parent_base_id: Option<String>,
    asset: &alife_core::FoundationWeightAsset,
    branches: &DendriticBranchSet,
    scorecard: &BrainSurgeryScorecardV1,
    reopen_receipt: Option<&ReopenVerificationReceiptV1>,
) -> Result<BrainSurgeryBaseBundle> {
    validate_identifier("base_id", base_id)?;
    if let Some(parent) = &parent_base_id {
        validate_identifier("parent_base_id", parent)?;
    }
    if asset.manifest().capacity_class_id() != BrainCapacityClass::N2048_ID {
        return Err("brain surgery bundles require N2048 foundation assets".into());
    }
    asset.encode_canonical()?;
    branches.validate_for_neuron_count(N2048_NEURON_COUNT)?;
    validate_scorecard(scorecard)?;

    std::fs::create_dir_all(output_dir)?;

    let foundation_file = "foundation.alife-foundation".to_string();
    let foundation_bytes = asset.encode_canonical()?;
    let foundation_sha256 = sha256_hex(&foundation_bytes);
    alife_storage::write(output_dir.join(&foundation_file), &foundation_bytes)?;

    let dendritic_branches_file = "dendritic_branches.json".to_string();
    let branch_bytes = serde_json::to_vec_pretty(branches)?;
    let dendritic_branches_sha256 = sha256_hex(&branch_bytes);
    alife_storage::write(output_dir.join(&dendritic_branches_file), &branch_bytes)?;

    let scorecard_file = "scorecard.json".to_string();
    let scorecard_bytes = serde_json::to_vec_pretty(scorecard)?;
    let scorecard_sha256 = sha256_hex(&scorecard_bytes);
    alife_storage::write(output_dir.join(&scorecard_file), &scorecard_bytes)?;

    let (reopen_verified, reopen_receipt_file, reopen_receipt_sha256) = match reopen_receipt {
        Some(receipt) => {
            if !receipt.verified {
                return Err("cannot embed an unverified reopen receipt".into());
            }
            let file = "reopen_receipt.json".to_string();
            let bytes = serde_json::to_vec_pretty(receipt)?;
            let digest = sha256_hex(&bytes);
            alife_storage::write(output_dir.join(&file), &bytes)?;
            (true, Some(file), Some(digest))
        }
        None => (false, None, None),
    };

    let manifest = BrainSurgeryBaseBundleManifestV1 {
        schema: BRAIN_SURGERY_BUNDLE_SCHEMA.to_string(),
        schema_version: BRAIN_SURGERY_SCHEMA_VERSION,
        base_id: base_id.to_string(),
        parent_base_id,
        foundation_file,
        foundation_sha256,
        foundation_digest: blake3_hex(&asset.digest()),
        sensor_profile: asset.manifest().sensor_profile(),
        weight_count: asset.weights().len(),
        dendritic_branches_file,
        dendritic_branches_sha256,
        dendritic_branch_count: branches.branches().len(),
        semantic_prior_mode: "deterministic_for_training".to_string(),
        semantic_prior_context_contract: GpuLiveBrainRuntime::semantic_prior_context_contract(),
        scorecard_file,
        scorecard_sha256,
        reopen_verified,
        reopen_receipt_file,
        reopen_receipt_sha256,
    };
    alife_storage::write(
        output_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;

    Ok(BrainSurgeryBaseBundle {
        root: output_dir.to_path_buf(),
        manifest,
        asset: asset.clone(),
        dendritic_branches: branches.clone(),
        scorecard: scorecard.clone(),
    })
}

#[allow(clippy::too_many_arguments)]
fn write_base_bundle_new(
    output_dir: &Path,
    forbidden_source: Option<&Path>,
    base_id: &str,
    parent_base_id: Option<String>,
    asset: &alife_core::FoundationWeightAsset,
    branches: &DendriticBranchSet,
    scorecard: &BrainSurgeryScorecardV1,
) -> Result<BrainSurgeryBaseBundle> {
    let target = ensure_absent_output_dir(output_dir, forbidden_source)?;
    let parent = target
        .parent()
        .ok_or("bundle target must have a parent directory")?;
    let stage_dir = parent.join(format!(
        ".brain-surgery-stage-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    let _ = std::fs::remove_dir_all(&stage_dir);
    let write_res = write_base_bundle(
        &stage_dir,
        base_id,
        parent_base_id,
        asset,
        branches,
        scorecard,
        None,
    );
    let mut bundle = match write_res {
        Ok(b) => b,
        Err(err) => {
            let _ = std::fs::remove_dir_all(&stage_dir);
            return Err(err);
        }
    };
    if let Err(err) = std::fs::rename(&stage_dir, &target) {
        let _ = std::fs::remove_dir_all(&stage_dir);
        return Err(err.into());
    }
    bundle.root = output_dir.to_path_buf();
    Ok(bundle)
}

/// Initializes a self-contained surgery base bundle from an explicit
/// `FoundationWeightAsset` (or procedurally compiled N2048 grounded founder)
/// plus optional V11 dendritic branches and scorecard.
pub fn init_base_bundle(
    output_dir: &Path,
    base_id: &str,
    foundation_path: Option<&Path>,
    branches_path: Option<&Path>,
    scorecard_path: Option<&Path>,
    founder_seed: u64,
) -> Result<BrainSurgeryBaseBundle> {
    let asset = match foundation_path {
        Some(path) => alife_core::FoundationWeightAsset::decode_canonical(&std::fs::read(path)?)?,
        None => initial_n2048_care_asset(founder_seed)?,
    };
    let branches = match branches_path {
        Some(path) => {
            let decoded: DendriticBranchSet = serde_json::from_slice(&std::fs::read(path)?)?;
            decoded.validate_for_neuron_count(N2048_NEURON_COUNT)?;
            decoded
        }
        None => DendriticBranchSet::default(),
    };
    let scorecard = match scorecard_path {
        Some(path) => {
            let decoded: BrainSurgeryScorecardV1 = serde_json::from_slice(&std::fs::read(path)?)?;
            validate_scorecard(&decoded)?;
            decoded
        }
        None => default_starter_scorecard(),
    };
    write_base_bundle_new(
        output_dir, None, base_id, None, &asset, &branches, &scorecard,
    )
}

/// Initializes a base bundle from a published GPU save, failing closed if the
/// saved N2048 foundation candidate OR exact V11 dendritic branch bytes are
/// missing or corrupted.
pub fn init_base_bundle_from_save(
    output_dir: &Path,
    base_id: &str,
    save_path: &Path,
    asset_root: &Path,
    organism_id: Option<u64>,
    scorecard_path: Option<&Path>,
) -> Result<BrainSurgeryBaseBundle> {
    let (durable_manifest, loaded) = GpuDurableSaveManifest::open_loaded(save_path, asset_root)?;
    let save = &loaded.save;
    let creature = match organism_id {
        Some(raw) => save
            .creatures
            .iter()
            .find(|c| c.organism_id.raw() == raw)
            .ok_or_else(|| format!("save does not contain organism {raw}"))?,
        None => save
            .creatures
            .first()
            .ok_or("save does not contain any creatures")?,
    };
    let brain = creature
        .gpu_brain
        .as_ref()
        .ok_or("creature save does not include GPU brain state")?;
    let exact_ref = brain.exact_cognitive_state.as_ref().ok_or(
        "creature GPU brain is missing v11 exact_cognitive_state (refusing incomplete export)",
    )?;
    let store = GpuCheckpointAssetStore::new(durable_manifest.asset_root().to_path_buf())?;
    let exact = store.read_exact_cognitive_state(&save.assets, exact_ref)?;
    let world = save.restore_headless_world()?;
    let admission = world
        .organism_registry()
        .get(creature.organism_id)
        .ok_or("saved organism record missing from world registry")?
        .authoritative_admission_at(world.tick())?;
    let asset = admission
        .genome
        .n2048_foundation_candidate
        .ok_or("saved organism does not carry an N2048 foundation candidate")?;
    let scorecard = match scorecard_path {
        Some(path) => {
            let decoded: BrainSurgeryScorecardV1 = serde_json::from_slice(&std::fs::read(path)?)?;
            validate_scorecard(&decoded)?;
            decoded
        }
        None => default_starter_scorecard(),
    };
    write_base_bundle_new(
        output_dir,
        None,
        base_id,
        None,
        &asset,
        &exact.dendritic_branches,
        &scorecard,
    )
}

/// Loads and verifies every SHA-256 and structural contract inside a base bundle.
pub fn load_and_verify_base_bundle(bundle_dir: &Path) -> Result<BrainSurgeryBaseBundle> {
    let manifest_bytes = std::fs::read(bundle_dir.join("manifest.json"))?;
    let manifest: BrainSurgeryBaseBundleManifestV1 = serde_json::from_slice(&manifest_bytes)?;
    if manifest.schema != BRAIN_SURGERY_BUNDLE_SCHEMA
        || manifest.schema_version != BRAIN_SURGERY_SCHEMA_VERSION
    {
        return Err("unsupported brain surgery bundle manifest schema or version".into());
    }
    validate_identifier("manifest.base_id", &manifest.base_id)?;

    let foundation_bytes = std::fs::read(bundle_dir.join(&manifest.foundation_file))?;
    if sha256_hex(&foundation_bytes) != manifest.foundation_sha256 {
        return Err("bundle foundation.alife-foundation SHA-256 mismatch".into());
    }
    let asset = alife_core::FoundationWeightAsset::decode_canonical(&foundation_bytes)?;
    if blake3_hex(&asset.digest()) != manifest.foundation_digest {
        return Err("bundle foundation BLAKE3 digest mismatch".into());
    }
    if asset.weights().len() != manifest.weight_count
        || asset.manifest().sensor_profile() != manifest.sensor_profile
        || asset.manifest().capacity_class_id() != BrainCapacityClass::N2048_ID
    {
        return Err("bundle foundation metadata does not match manifest".into());
    }

    let branch_bytes = std::fs::read(bundle_dir.join(&manifest.dendritic_branches_file))?;
    if sha256_hex(&branch_bytes) != manifest.dendritic_branches_sha256 {
        return Err("bundle dendritic_branches.json SHA-256 mismatch".into());
    }
    let dendritic_branches: DendriticBranchSet = serde_json::from_slice(&branch_bytes)?;
    dendritic_branches.validate_for_neuron_count(N2048_NEURON_COUNT)?;
    if dendritic_branches.branches().len() != manifest.dendritic_branch_count {
        return Err("bundle dendritic branch count mismatch".into());
    }

    let scorecard_bytes = std::fs::read(bundle_dir.join(&manifest.scorecard_file))?;
    if sha256_hex(&scorecard_bytes) != manifest.scorecard_sha256 {
        return Err("bundle scorecard.json SHA-256 mismatch".into());
    }
    let scorecard: BrainSurgeryScorecardV1 = serde_json::from_slice(&scorecard_bytes)?;
    validate_scorecard(&scorecard)?;

    if manifest.reopen_verified {
        let receipt_file = manifest
            .reopen_receipt_file
            .as_ref()
            .ok_or("reopen_verified=true requires reopen_receipt_file")?;
        let receipt_sha = manifest
            .reopen_receipt_sha256
            .as_ref()
            .ok_or("reopen_verified=true requires reopen_receipt_sha256")?;
        let receipt_bytes = std::fs::read(bundle_dir.join(receipt_file))?;
        if sha256_hex(&receipt_bytes) != *receipt_sha {
            return Err("bundle reopen_receipt.json SHA-256 mismatch".into());
        }
        let receipt: ReopenVerificationReceiptV1 = serde_json::from_slice(&receipt_bytes)?;
        if receipt.schema != BRAIN_SURGERY_REOPEN_SCHEMA
            || receipt.schema_version != BRAIN_SURGERY_SCHEMA_VERSION
            || !receipt.verified
            || receipt.base_id != manifest.base_id
            || receipt.foundation_digest != manifest.foundation_digest
            || receipt.dendritic_branches_sha256 != manifest.dendritic_branches_sha256
        {
            return Err("bundle reopen receipt does not match manifest".into());
        }
    }

    Ok(BrainSurgeryBaseBundle {
        root: bundle_dir.to_path_buf(),
        manifest,
        asset,
        dendritic_branches,
        scorecard,
    })
}

fn apply_dendritic_op(
    current: &DendriticBranchSet,
    op: &DendriticPatchOp,
) -> Result<DendriticBranchSet> {
    let next = match op {
        DendriticPatchOp::Keep => current.clone(),
        DendriticPatchOp::ReplaceAll { branches } => branches.clone(),
        DendriticPatchOp::UpsertTargets {
            remove_targets,
            add_branches,
        } => {
            let remove_set: BTreeSet<u32> = remove_targets.iter().copied().collect();
            let mut merged: Vec<DendriticBranch> = current
                .branches()
                .iter()
                .filter(|branch| !remove_set.contains(&branch.target))
                .cloned()
                .collect();
            merged.extend(add_branches.iter().cloned());
            DendriticBranchSet::new(merged)?
        }
    };
    next.validate_for_neuron_count(N2048_NEURON_COUNT)?;
    Ok(next)
}

fn validate_edited_weights(
    phenotype: &BrainPhenotype,
    base_weights: &[f32],
    next_weights: &[f32],
) -> Result<()> {
    for (index, synapse) in phenotype.synapses().iter().enumerate() {
        let base = base_weights[index];
        let edited = next_weights[index];
        if !edited.is_finite() {
            return Err(format!("non-finite weight at index {index}").into());
        }
        if synapse.kind() == CompiledSynapseKind::Recurrent && edited.to_bits() != base.to_bits() {
            let projection = phenotype
                .projections()
                .get(usize::from(synapse.route_index()))
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            let invalid_sign = match projection.projection_type() {
                ProjectionType::LateralInhibition => edited >= 0.0,
                ProjectionType::Homeostatic | ProjectionType::MotorProposal => edited < 0.0,
                _ => false,
            };
            if invalid_sign {
                return Err(format!(
                    "weight edit at index {index} violates {:?} sign invariant (value={edited})",
                    projection.projection_type()
                )
                .into());
            }
        }
    }
    Ok(())
}

/// Validates and applies a declarative data-only patch onto an existing base
/// bundle in memory without touching disk.
pub fn validate_and_apply_patch(
    bundle: &BrainSurgeryBaseBundle,
    patch: &BrainSurgeryPatchV1,
    candidate_id: &str,
) -> Result<(
    alife_core::FoundationWeightAsset,
    DendriticBranchSet,
    PatchApplicationReceipt,
)> {
    if patch.schema != BRAIN_SURGERY_PATCH_SCHEMA
        || patch.schema_version != BRAIN_SURGERY_SCHEMA_VERSION
    {
        return Err("unsupported brain surgery patch schema or version".into());
    }
    validate_identifier("patch_id", &patch.patch_id)?;
    validate_identifier("candidate_id", candidate_id)?;

    if patch.target_base_id != bundle.manifest.base_id {
        return Err(format!(
            "patch targets base {:?}, but active base is {:?}",
            patch.target_base_id, bundle.manifest.base_id
        )
        .into());
    }
    if patch.target_foundation_digest != bundle.manifest.foundation_digest {
        return Err("patch target_foundation_digest does not match base bundle".into());
    }
    if patch.target_dendritic_branches_sha256 != bundle.manifest.dendritic_branches_sha256 {
        return Err("patch target_dendritic_branches_sha256 does not match base bundle".into());
    }

    let phenotype = compile_n2048_phenotype(&bundle.asset)?;
    let base_weights = bundle.asset.weights();
    let mut weights = base_weights.to_vec();
    let mut seen_indices = BTreeSet::new();
    for edit in &patch.weight_edits {
        if !seen_indices.insert(edit.index) {
            return Err(format!("duplicate weight edit index {}", edit.index).into());
        }
        if edit.index >= weights.len() {
            return Err(format!(
                "weight edit index {} out of range (len={})",
                edit.index,
                weights.len()
            )
            .into());
        }
        if !edit.value.is_finite() {
            return Err(format!("non-finite weight edit value at index {}", edit.index).into());
        }
        let next = match edit.mode {
            WeightEditMode::Set => edit.value,
            WeightEditMode::Add => weights[edit.index] + edit.value,
        };
        if !next.is_finite() {
            return Err(format!("weight edit overflow/NaN at index {}", edit.index).into());
        }
        weights[edit.index] = next;
    }

    validate_edited_weights(&phenotype, base_weights, &weights)?;

    let next_asset = if patch.weight_edits.is_empty() {
        bundle.asset.clone()
    } else {
        alife_core::FoundationWeightAsset::from_trained_weights(
            &phenotype,
            weights,
            bundle.asset.manifest().training_stage(),
        )?
    };
    let next_branches = apply_dendritic_op(&bundle.dendritic_branches, &patch.dendritic_edits)?;
    let next_branch_bytes = serde_json::to_vec_pretty(&next_branches)?;
    let next_branch_sha = sha256_hex(&next_branch_bytes);

    let receipt = PatchApplicationReceipt {
        patch_id: patch.patch_id.clone(),
        source_base_id: bundle.manifest.base_id.clone(),
        candidate_base_id: candidate_id.to_string(),
        source_foundation_digest: bundle.manifest.foundation_digest.clone(),
        candidate_foundation_digest: blake3_hex(&next_asset.digest()),
        source_dendritic_branches_sha256: bundle.manifest.dendritic_branches_sha256.clone(),
        candidate_dendritic_branches_sha256: next_branch_sha,
        weight_edits_applied: patch.weight_edits.len(),
        dendritic_branch_count: next_branches.branches().len(),
    };
    Ok((next_asset, next_branches, receipt))
}

/// Validates and applies `patch_path` onto `base_dir`, writing an immutable
/// self-contained candidate bundle into `output_dir`.
pub fn apply_patch_to_bundle(
    base_dir: &Path,
    patch_path: &Path,
    output_dir: &Path,
    candidate_id: Option<&str>,
) -> Result<PatchApplicationReceipt> {
    let base = load_and_verify_base_bundle(base_dir)?;
    let patch: BrainSurgeryPatchV1 = serde_json::from_slice(&std::fs::read(patch_path)?)?;
    let resolved_id = candidate_id.unwrap_or(&patch.patch_id);
    let (next_asset, next_branches, receipt) =
        validate_and_apply_patch(&base, &patch, resolved_id)?;
    write_base_bundle_new(
        output_dir,
        Some(base_dir),
        resolved_id,
        Some(base.manifest.base_id.clone()),
        &next_asset,
        &next_branches,
        &base.scorecard,
    )?;
    Ok(receipt)
}

fn distance_closure_ratio(initial: Option<f32>, final_dist: Option<f32>) -> f64 {
    let (Some(i), Some(f)) = (initial, final_dist) else {
        return 0.0;
    };
    if !i.is_finite() || !f.is_finite() || i <= 1e-6 {
        return 0.0;
    }
    ((f64::from(i) - f64::from(f)) / f64::from(i)).clamp(0.0, 1.0)
}

/// Deterministic `0.0..=100.0` per-case scorer operating solely on the measured
/// `FoundationPilotReceipt` produced by paired base/candidate runs.
pub fn score_pilot_receipt(
    lesson: FoundationTeacherLesson,
    receipt: &FoundationPilotReceipt,
) -> f64 {
    let completed = if receipt.lesson_completed == Some(true) {
        1.0
    } else {
        0.0
    };
    let meals = receipt
        .food_outcome
        .as_ref()
        .map_or(0, |o| o.nourished_meal_events);
    let consumed = receipt
        .food_outcome
        .as_ref()
        .map_or(0, |o| o.consumption_events)
        .max(receipt.held_food_consumption_events);
    let grabbed = receipt
        .food_outcome
        .as_ref()
        .map_or(0, |o| o.explicit_grab_acquisition_events);
    let closure =
        distance_closure_ratio(receipt.initial_food_distance, receipt.final_food_distance);

    let score = match lesson {
        FoundationTeacherLesson::MazeNavigation => {
            let route = receipt.maze_route_nodes.max(1) as f64;
            let progress = (receipt.maze_nodes_reached as f64 / route).clamp(0.0, 1.0);
            50.0 * completed + 40.0 * progress + 10.0 * closure
        }
        FoundationTeacherLesson::VocabularyReception => {
            let acted = if receipt.vocabulary_target_actions > 0 {
                1.0
            } else {
                0.0
            };
            let matches = (receipt.literal_word_matches.min(4) as f64) / 4.0;
            40.0 * completed + 35.0 * acted + 25.0 * matches
        }
        FoundationTeacherLesson::VocabularyProduction => {
            let opps = receipt.speech_opportunities.max(1) as f64;
            let ratio = (receipt.correct_utterances as f64 / opps).clamp(0.0, 1.0);
            let unprompted = (receipt.unprompted_correct_utterances.min(2) as f64) / 2.0;
            (40.0 * completed + 50.0 * ratio + 10.0 * unprompted).min(100.0)
        }
        FoundationTeacherLesson::Feeding
        | FoundationTeacherLesson::EatHeldFood
        | FoundationTeacherLesson::EatReachableFood
        | FoundationTeacherLesson::GrabFood
        | FoundationTeacherLesson::RelocatedFood => {
            let meal_pts = (meals.min(1) as f64) * 25.0;
            let action_pts = if consumed > 0 || grabbed > 0 {
                15.0
            } else {
                0.0
            };
            50.0 * completed + meal_pts + action_pts + 10.0 * closure
        }
        FoundationTeacherLesson::HazardAvoidance
        | FoundationTeacherLesson::ObstacleNavigation
        | FoundationTeacherLesson::Recovery
        | FoundationTeacherLesson::VisionSearch => {
            let food_pts = if meals > 0 || consumed > 0 { 20.0 } else { 0.0 };
            60.0 * completed + food_pts + 20.0 * closure
        }
    };
    score.clamp(0.0, 100.0)
}

/// Aggregates case-by-case base vs candidate comparisons into a promotion decision
/// using the weighted average across the entire matched scorecard.
pub fn aggregate_comparison_report(
    base: &BrainSurgeryBaseBundle,
    candidate: &BrainSurgeryBaseBundle,
    candidate_patch_id: Option<String>,
    min_improvement: f64,
    results: Vec<CaseComparisonResult>,
) -> Result<BrainSurgeryComparisonReportV1> {
    if !min_improvement.is_finite() || min_improvement < 0.0 {
        return Err("min_improvement must be finite and >= 0".into());
    }
    let mut total_weight = 0.0_f64;
    let mut base_weighted_sum = 0.0_f64;
    let mut cand_weighted_sum = 0.0_f64;
    let mut improved_cases = 0_usize;
    let mut unchanged_cases = 0_usize;
    let mut failed_cases = 0_usize;
    let mut regressed_cases = Vec::new();

    for row in &results {
        if row.error.is_some() {
            failed_cases += 1;
            continue;
        }
        total_weight += row.weight;
        base_weighted_sum += row.weight * row.base_score;
        cand_weighted_sum += row.weight * row.candidate_score;
        if row.delta > 1e-6 {
            improved_cases += 1;
        } else if row.delta < -1e-6 {
            regressed_cases.push(row.case_id.clone());
        } else {
            unchanged_cases += 1;
        }
    }

    let (base_average, candidate_average) = if total_weight > 0.0 {
        (
            base_weighted_sum / total_weight,
            cand_weighted_sum / total_weight,
        )
    } else {
        (0.0, 0.0)
    };
    let delta = candidate_average - base_average;
    let promoted = failed_cases == 0 && total_weight > 0.0 && delta > min_improvement + 1e-9;

    Ok(BrainSurgeryComparisonReportV1 {
        schema: BRAIN_SURGERY_REPORT_SCHEMA.to_string(),
        schema_version: BRAIN_SURGERY_SCHEMA_VERSION,
        scorecard_id: base.scorecard.scorecard_id.clone(),
        base_id: base.manifest.base_id.clone(),
        candidate_id: candidate.manifest.base_id.clone(),
        candidate_patch_id,
        base_foundation_digest: base.manifest.foundation_digest.clone(),
        candidate_foundation_digest: candidate.manifest.foundation_digest.clone(),
        base_dendritic_sha256: base.manifest.dendritic_branches_sha256.clone(),
        candidate_dendritic_sha256: candidate.manifest.dendritic_branches_sha256.clone(),
        base_average,
        candidate_average,
        delta,
        min_improvement,
        improved_cases,
        regressed_cases,
        unchanged_cases,
        failed_cases,
        promoted,
        results,
    })
}

fn evaluate_single_case(
    bundle: &BrainSurgeryBaseBundle,
    case: &ScorecardCase,
    run_dir: &Path,
) -> Result<FoundationPilotReceipt> {
    let branches = (!bundle.dendritic_branches.branches().is_empty())
        .then(|| bundle.dendritic_branches.clone());
    if let Some(token) = case.request_token {
        return run_foundation_request_evaluation_with_v11_branches(
            run_dir,
            case.world_seed,
            case.founder_seed_base,
            case.ticks,
            bundle.asset.clone(),
            branches,
            FoundationEvaluationRequest {
                token,
                silent: case.silent_request,
            },
        );
    }
    let config = FoundationEvaluationConfig {
        food_need_condition: case.food_need_condition,
        food_position: case.food_position,
        sampling_temperature: case.sampling_temperature,
        silent_request: case.silent_request,
    };
    run_foundation_evaluation_pilot_with_v11_branches(
        run_dir,
        case.world_seed,
        case.founder_seed_base,
        case.ticks,
        case.lesson,
        config,
        bundle.asset.clone(),
        branches,
    )
}

/// Evaluates both `base_dir` and `candidate_dir` on identical matched scorecard
/// cases (`scorecard_override` or `base`'s bundled scorecard).
pub fn compare_bundles(
    base_dir: &Path,
    candidate_dir: &Path,
    scorecard_override: Option<&Path>,
    candidate_patch_id: Option<String>,
    min_improvement: f64,
    work_dir: &Path,
) -> Result<BrainSurgeryComparisonReportV1> {
    let base = load_and_verify_base_bundle(base_dir)?;
    let candidate = load_and_verify_base_bundle(candidate_dir)?;
    let scorecard = match scorecard_override {
        Some(path) => {
            let decoded: BrainSurgeryScorecardV1 = serde_json::from_slice(&std::fs::read(path)?)?;
            validate_scorecard(&decoded)?;
            decoded
        }
        None => base.scorecard.clone(),
    };
    if !base.scorecard.cases.is_empty()
        && scorecard_override.is_none()
        && candidate.scorecard != base.scorecard
    {
        return Err("candidate bundle scorecard differs from base bundle scorecard".into());
    }

    std::fs::create_dir_all(work_dir)?;
    let mut results = Vec::with_capacity(scorecard.cases.len());
    for case in &scorecard.cases {
        let base_case_dir = work_dir.join(format!("base-{}", case.id));
        let cand_case_dir = work_dir.join(format!("cand-{}", case.id));
        let _ = std::fs::remove_dir_all(&base_case_dir);
        let _ = std::fs::remove_dir_all(&cand_case_dir);

        let base_res = evaluate_single_case(&base, case, &base_case_dir);
        let cand_res = evaluate_single_case(&candidate, case, &cand_case_dir);
        let row = match (base_res, cand_res) {
            (Ok(b), Ok(c)) => {
                let bs = score_pilot_receipt(case.lesson, &b);
                let cs = score_pilot_receipt(case.lesson, &c);
                CaseComparisonResult {
                    case_id: case.id.clone(),
                    lesson: case.lesson,
                    weight: case.weight,
                    base_score: bs,
                    candidate_score: cs,
                    delta: cs - bs,
                    base_completed: b.lesson_completed,
                    candidate_completed: c.lesson_completed,
                    error: None,
                }
            }
            (b, c) => {
                let msg = format!(
                    "base={:?}; candidate={:?}",
                    b.as_ref().err().map(|e| e.to_string()),
                    c.as_ref().err().map(|e| e.to_string())
                );
                CaseComparisonResult {
                    case_id: case.id.clone(),
                    lesson: case.lesson,
                    weight: case.weight,
                    base_score: 0.0,
                    candidate_score: 0.0,
                    delta: 0.0,
                    base_completed: b.ok().and_then(|r| r.lesson_completed),
                    candidate_completed: c.ok().and_then(|r| r.lesson_completed),
                    error: Some(msg),
                }
            }
        };
        results.push(row);
    }

    aggregate_comparison_report(
        &base,
        &candidate,
        candidate_patch_id,
        min_improvement,
        results,
    )
}

fn branches_bit_exact_eq(left: &DendriticBranchSet, right: &DendriticBranchSet) -> bool {
    if left.branches().len() != right.branches().len() {
        return false;
    }
    left.branches().iter().zip(right.branches()).all(|(a, b)| {
        a.target == b.target
            && a.threshold.to_bits() == b.threshold.to_bits()
            && a.output_gain.to_bits() == b.output_gain.to_bits()
            && a.inputs.len() == b.inputs.len()
            && a.inputs.iter().zip(&b.inputs).all(|(ia, ib)| {
                ia.source == ib.source && ia.weight.to_bits() == ib.weight.to_bits()
            })
    })
}

/// Verifies in a separate OS child process that this bundle's N2048 weights
/// and V11 dendritic branches survive a real 1-tick GPU run, save capture,
/// removal of scratch inputs, and clean reload via `GpuLiveBrainRuntime::from_p34_launch`.
pub fn verify_bundle_reopen(
    bundle_dir: &Path,
    seed: u64,
    stamp_bundle: bool,
) -> Result<ReopenVerificationReceiptV1> {
    let exe = std::env::current_exe()?;
    verify_bundle_reopen_with_child_bin(bundle_dir, seed, &exe, stamp_bundle)
}

pub fn verify_bundle_reopen_with_child_bin(
    bundle_dir: &Path,
    seed: u64,
    child_bin: &Path,
    stamp_bundle: bool,
) -> Result<ReopenVerificationReceiptV1> {
    if seed == 0 {
        return Err("verify-reopen requires a non-zero seed".into());
    }
    let bundle = load_and_verify_base_bundle(bundle_dir)?;
    let run_root = std::env::temp_dir().join(format!(
        "alife-brain-surgery-reopen-{}-{}",
        std::process::id(),
        now_unix_secs()
    ));
    let _ = std::fs::remove_dir_all(&run_root);
    std::fs::create_dir_all(&run_root)?;

    let run_result = (|| -> Result<ReopenVerificationReceiptV1> {
        let asset_root = run_root.join("assets");
        std::fs::create_dir_all(&asset_root)?;
        let staging_save = run_root.join("staging-save.json");
        let final_save = run_root.join("final-save.json");
        let config_path = run_root.join("config.json");
        let asset_manifest_path = run_root.join("assets.json");
        let expected_path = run_root.join("expected.json");
        let archive_root = run_root.join("lineage");

        let mut cfg = CanonicalNewGameConfig::phase3(seed, 1)?;
        cfg.brain_class = BrainScaleTier::Standard2048;
        cfg.founder_seed_base = seed;
        cfg.sensor_profile = bundle.asset.manifest().sensor_profile();
        let mut game = create_canonical_new_game_with_n2048_candidate(&cfg, &bundle.asset)?;
        game.world.enable_island_for_new_game()?;
        game.world.set_age_death_disabled_for_new_game(true)?;

        let mut runtime_cfg =
            RuntimeConfig::deterministic_default(seed, BrainScaleTier::Standard2048);
        runtime_cfg.features.gpu_backend_enabled = true;
        runtime_cfg.brain_policy.policy = PolicyBackend::NeuralClosedLoopGpu;

        let base_save = PortableSaveFile::from_headless_world(
            format!("brain-surgery-reopen-{seed}"),
            &game.world,
            runtime_cfg.clone(),
            AssetManifest::empty(),
            game.creatures,
        )?;
        base_save.validate_with_asset_root(&asset_root)?;

        let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
        let mut runtime = GpuLiveBrainRuntime::new_profiled_archived(
            backend,
            game.world,
            seed,
            BrainScaleTier::Standard2048,
            cfg.sensor_profile,
            LineageLibraryConfig::profile_default(&archive_root),
            format!("brain-surgery-reopen-{seed}"),
            ArchiveLearnedCapturePolicy::GeneticOnly,
        )?;
        runtime.set_deterministic_semantic_prior_for_surgery(seed);
        let learner = OrganismId(1);
        let mut normalized_base = base_save;
        normalized_base.replace_headless_world_snapshot(&runtime.world_snapshot())?;
        runtime.attach_durable_checkpoint_boundary(&staging_save, &asset_root, normalized_base)?;
        runtime.set_v11_dendritic_branches(learner, bundle.dendritic_branches.clone())?;
        let _summaries = runtime.tick()?;
        if runtime.world_snapshot().tick() == Tick::ZERO {
            return Err("verify-reopen expected runtime.tick() to advance world tick > 0".into());
        }

        let live_branches = runtime.checkpoint_v11_dendritic_branches(learner)?;
        if !branches_bit_exact_eq(&live_branches, &bundle.dendritic_branches) {
            return Err("live GPU V11 branches diverged from edited bundle after 1 tick".into());
        }
        let exact_save = runtime.capture_portable_checkpoint()?;
        GpuDurableSaveManifest::publish_snapshot(&final_save, &asset_root, &exact_save)?;
        std::fs::write(&config_path, serde_json::to_vec_pretty(&exact_save.config)?)?;
        std::fs::write(
            &asset_manifest_path,
            serde_json::to_vec_pretty(&exact_save.assets)?,
        )?;
        let expected = ExpectedReopenState {
            base_id: bundle.manifest.base_id.clone(),
            foundation_digest: bundle.manifest.foundation_digest.clone(),
            dendritic_branches: bundle.dendritic_branches.clone(),
            dendritic_branches_sha256: bundle.manifest.dendritic_branches_sha256.clone(),
            tick: exact_save.world.tick.raw(),
            seed,
        };
        std::fs::write(&expected_path, serde_json::to_vec_pretty(&expected)?)?;
        drop(runtime);
        let _ = std::fs::remove_file(&staging_save);

        let child_out = Command::new(child_bin)
            .arg("verify-reopen-child")
            .arg(&final_save)
            .arg(&asset_root)
            .arg(&config_path)
            .arg(&asset_manifest_path)
            .arg(&expected_path)
            .env_remove("ALIFE_SLM_PRIOR")
            .env("ALIFE_SLM_PRIOR_BACKEND", "deterministic")
            .output()?;
        if !child_out.status.success() {
            return Err(format!(
                "verify-reopen child process failed (status={}): stdout={} stderr={}",
                child_out.status,
                String::from_utf8_lossy(&child_out.stdout),
                String::from_utf8_lossy(&child_out.stderr)
            )
            .into());
        }
        let mut receipt: ReopenVerificationReceiptV1 = serde_json::from_slice(&child_out.stdout)?;
        if !receipt.verified {
            return Err("child reopen verification returned verified=false".into());
        }
        receipt.parent_pid = std::process::id();
        if stamp_bundle {
            write_base_bundle(
                bundle_dir,
                &bundle.manifest.base_id,
                bundle.manifest.parent_base_id.clone(),
                &bundle.asset,
                &bundle.dendritic_branches,
                &bundle.scorecard,
                Some(&receipt),
            )?;
        }
        Ok(receipt)
    })();

    let _ = std::fs::remove_dir_all(&run_root);
    run_result
}

/// Separate-process verification entrypoint invoked by `verify_bundle_reopen`.
/// Opens `save_path` through `GpuLiveBrainRuntime::from_p34_launch` and verifies
/// tick preservation, foundation digest, and bit-exact V11 dendritic branches.
pub fn verify_bundle_reopen_child(
    save_path: &Path,
    asset_root: &Path,
    config_path: &Path,
    asset_manifest_path: &Path,
    expected_path: &Path,
) -> Result<ReopenVerificationReceiptV1> {
    let expected: ExpectedReopenState = serde_json::from_slice(&std::fs::read(expected_path)?)?;
    let fixture_root = save_path
        .parent()
        .ok_or("save_path must have parent directory")?
        .to_path_buf();
    let launch = AppShellLaunchConfig {
        fixture_root,
        config_path: config_path.to_path_buf(),
        asset_manifest_path: asset_manifest_path.to_path_buf(),
        save_path: save_path.to_path_buf(),
        asset_root: asset_root.to_path_buf(),
        start_paused: false,
        brain_policy: PolicyBackend::NeuralClosedLoopGpu,
    };
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    let restored = GpuLiveBrainRuntime::from_p34_launch(backend, &launch)?;
    let learner = OrganismId(1);
    if restored.world_snapshot().tick().raw() != expected.tick {
        return Err(format!(
            "child reopen tick mismatch: expected {}, got {}",
            expected.tick,
            restored.world_snapshot().tick().raw()
        )
        .into());
    }
    let reopened_branches = restored.checkpoint_v11_dendritic_branches(learner)?;
    if !branches_bit_exact_eq(&reopened_branches, &expected.dendritic_branches) {
        return Err("reopened GPU V11 dendritic branches are not bit-exact".into());
    }
    let record = restored
        .world_snapshot()
        .organism_registry()
        .get(learner)
        .ok_or("reopened world missing OrganismId(1)")?
        .clone();
    let admission = record.authoritative_admission_at(restored.world_snapshot().tick())?;
    let restored_asset = admission
        .genome
        .n2048_foundation_candidate
        .ok_or("reopened creature missing N2048 foundation candidate")?;
    if blake3_hex(&restored_asset.digest()) != expected.foundation_digest {
        return Err("reopened foundation BLAKE3 digest mismatch".into());
    }

    let (durable_manifest, loaded) = GpuDurableSaveManifest::open_loaded(save_path, asset_root)?;
    let creature = loaded
        .save
        .creatures
        .iter()
        .find(|c| c.organism_id == learner)
        .ok_or("loaded save missing OrganismId(1)")?;
    let exact_ref = creature
        .gpu_brain
        .as_ref()
        .and_then(|b| b.exact_cognitive_state.as_ref())
        .ok_or("loaded save missing exact_cognitive_state")?;
    let store = GpuCheckpointAssetStore::new(durable_manifest.asset_root().to_path_buf())?;
    let exact = store.read_exact_cognitive_state(&loaded.save.assets, exact_ref)?;
    if !branches_bit_exact_eq(&exact.dendritic_branches, &expected.dendritic_branches) {
        return Err("loaded CAS exact_cognitive_state branches are not bit-exact".into());
    }

    let prior_backend = restored
        .semantic_prior_metrics()
        .map_or("none", |m| m.backend.as_str());
    if prior_backend != "deterministic" {
        return Err(format!(
            "reopen verification expected deterministic semantic prior, got {prior_backend}"
        )
        .into());
    }

    Ok(ReopenVerificationReceiptV1 {
        schema: BRAIN_SURGERY_REOPEN_SCHEMA.to_string(),
        schema_version: BRAIN_SURGERY_SCHEMA_VERSION,
        base_id: expected.base_id,
        foundation_digest: expected.foundation_digest,
        dendritic_branches_sha256: expected.dendritic_branches_sha256,
        dendritic_branch_count: expected.dendritic_branches.branches().len(),
        tick: expected.tick,
        parent_pid: 0,
        child_pid: std::process::id(),
        verified: true,
        semantic_prior_mode: format!(
            "deterministic_seed_{}_backend_{}",
            expected.seed, prior_backend
        ),
        verified_at_unix: now_unix_secs(),
    })
}

/// Constructs a sample valid dendritic branch edit for tests and smoke scripts.
pub fn sample_v11_branch(target: u32, source: u32, weight: f32) -> Result<DendriticBranch> {
    let input = DendriticInputRef::new(source, weight)?;
    Ok(DendriticBranch::new(target, 0.25, 1.15, vec![input])?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "alife-brain-surgery-test-{label}-{}-{}",
            std::process::id(),
            now_unix_secs()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn dummy_receipt(completed: bool, meals: usize) -> FoundationPilotReceipt {
        serde_json::from_value(serde_json::json!({
            "seed": 1,
            "founder_seed_base": 1,
            "ticks": 64,
            "collection_seconds": 0.1,
            "replay_seconds": 0.0,
            "maximum_logit_error": 0.0,
            "maximum_activation_error": 0.0,
            "maximum_activity_ema_error": 0.0,
            "maximum_metabolic_error": 0.0,
            "source_asset_digest": "00",
            "teacher_mode": false,
            "consumed_events": 0,
            "demonstration_replay_records": 0,
            "food_position": null,
            "initial_food_distance": 5.0,
            "final_food_distance": if completed { 0.0 } else { 2.5 },
            "lesson": "feeding",
            "lesson_completed": completed,
            "food_outcome": {
                "organism": 1,
                "target": 2,
                "explicit_grab_acquisition_events": meals,
                "ordinary_acquisition_events": 0,
                "consumption_events": meals,
                "nourished_meal_events": meals
            }
        }))
        .unwrap()
    }

    #[test]
    fn base_bundle_roundtrips_and_rejects_tamper() {
        let dir = temp_dir("tamper");
        let bundle = init_base_bundle(&dir, "r41-test", None, None, None, 700_001).unwrap();
        let loaded = load_and_verify_base_bundle(&dir).unwrap();
        assert_eq!(loaded.manifest, bundle.manifest);
        assert_eq!(loaded.asset.weights().len(), bundle.asset.weights().len());

        // Tamper dendritic_branches.json and verify fail-closed hash check catches it.
        let corrupted =
            DendriticBranchSet::new(vec![sample_v11_branch(4, 1, 0.5).unwrap()]).unwrap();
        std::fs::write(
            dir.join("dendritic_branches.json"),
            serde_json::to_vec_pretty(&corrupted).unwrap(),
        )
        .unwrap();
        assert!(load_and_verify_base_bundle(&dir).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn patch_validation_applies_weights_and_v11_and_enforces_invariants() {
        let base_dir = temp_dir("patch-base");
        let cand_dir = temp_dir("patch-cand");
        let bundle = init_base_bundle(&base_dir, "base-1", None, None, None, 700_001).unwrap();
        let phenotype = compile_n2048_phenotype(&bundle.asset).unwrap();
        let mut decoder_idx = None;
        let mut lat_inh_idx = None;
        for (i, syn) in phenotype.synapses().iter().enumerate() {
            if decoder_idx.is_none() && matches!(syn.kind(), CompiledSynapseKind::Decoder(_)) {
                decoder_idx = Some(i);
            }
            if lat_inh_idx.is_none() && syn.kind() == CompiledSynapseKind::Recurrent {
                if let Some(proj) = phenotype.projections().get(usize::from(syn.route_index())) {
                    if proj.projection_type() == ProjectionType::LateralInhibition {
                        lat_inh_idx = Some(i);
                    }
                }
            }
        }
        let decoder_idx = decoder_idx.expect("decoder synapse exists");
        let valid_patch = BrainSurgeryPatchV1 {
            schema: BRAIN_SURGERY_PATCH_SCHEMA.into(),
            schema_version: 1,
            patch_id: "cand-v11".into(),
            target_base_id: bundle.manifest.base_id.clone(),
            target_foundation_digest: bundle.manifest.foundation_digest.clone(),
            target_dendritic_branches_sha256: bundle.manifest.dendritic_branches_sha256.clone(),
            weight_edits: vec![WeightEdit {
                index: decoder_idx,
                mode: WeightEditMode::Add,
                value: 0.05,
            }],
            dendritic_edits: DendriticPatchOp::UpsertTargets {
                remove_targets: vec![],
                add_branches: vec![sample_v11_branch(42, 7, 0.75).unwrap()],
            },
            notes: Some("test gain-of-function edit".into()),
        };
        let patch_path = base_dir.join("patch.json");
        std::fs::write(
            &patch_path,
            serde_json::to_vec_pretty(&valid_patch).unwrap(),
        )
        .unwrap();
        let receipt = apply_patch_to_bundle(&base_dir, &patch_path, &cand_dir, None).unwrap();
        assert_eq!(receipt.weight_edits_applied, 1);
        assert_eq!(receipt.dendritic_branch_count, 1);
        assert_ne!(
            receipt.source_foundation_digest,
            receipt.candidate_foundation_digest
        );
        let cand_loaded = load_and_verify_base_bundle(&cand_dir).unwrap();
        assert_eq!(
            cand_loaded.manifest.parent_base_id.as_deref(),
            Some("base-1")
        );
        assert!(!cand_loaded.manifest.reopen_verified);

        // Stale base rejection.
        let mut stale = valid_patch.clone();
        stale.target_base_id = "wrong-base".into();
        assert!(validate_and_apply_patch(&bundle, &stale, "bad").is_err());

        // Duplicate index rejection.
        let mut dup = valid_patch.clone();
        dup.weight_edits.push(WeightEdit {
            index: decoder_idx,
            mode: WeightEditMode::Set,
            value: 0.1,
        });
        assert!(validate_and_apply_patch(&bundle, &dup, "dup").is_err());

        // Non-finite rejection.
        let mut nan_patch = valid_patch.clone();
        nan_patch.weight_edits[0].value = f32::NAN;
        assert!(validate_and_apply_patch(&bundle, &nan_patch, "nan").is_err());

        // Out-of-bounds neuron branch rejection.
        let mut oob_branch = valid_patch.clone();
        oob_branch.dendritic_edits = DendriticPatchOp::ReplaceAll {
            branches: DendriticBranchSet::new(vec![sample_v11_branch(3000, 1, 0.5).unwrap()])
                .unwrap(),
        };
        assert!(validate_and_apply_patch(&bundle, &oob_branch, "oob").is_err());

        // Recurrent sign constraint rejection.
        if let Some(inh_idx) = lat_inh_idx {
            let mut bad_sign = valid_patch;
            bad_sign.weight_edits = vec![WeightEdit {
                index: inh_idx,
                mode: WeightEditMode::Set,
                value: 0.25,
            }];
            assert!(validate_and_apply_patch(&bundle, &bad_sign, "bad-sign").is_err());
        }

        let _ = std::fs::remove_dir_all(&base_dir);
        let _ = std::fs::remove_dir_all(&cand_dir);
    }

    #[test]
    fn scorecard_aggregation_ranks_by_overall_average_and_rejects_failures() {
        let dir = temp_dir("scorecard");
        let base = init_base_bundle(&dir, "r41", None, None, None, 700_001).unwrap();
        let cand_dir = temp_dir("scorecard-cand");
        let cand = init_base_bundle(&cand_dir, "r42", None, None, None, 700_002).unwrap();

        let high = score_pilot_receipt(FoundationTeacherLesson::Feeding, &dummy_receipt(true, 1));
        let mid = score_pilot_receipt(FoundationTeacherLesson::Feeding, &dummy_receipt(false, 0));
        assert!(high > mid);

        // One small regression (-2) outweighed by big gain (+25) must promote.
        let rows = vec![
            CaseComparisonResult {
                case_id: "case-a".into(),
                lesson: FoundationTeacherLesson::Feeding,
                weight: 1.0,
                base_score: 90.0,
                candidate_score: 88.0,
                delta: -2.0,
                base_completed: Some(true),
                candidate_completed: Some(true),
                error: None,
            },
            CaseComparisonResult {
                case_id: "case-b".into(),
                lesson: FoundationTeacherLesson::MazeNavigation,
                weight: 1.0,
                base_score: 40.0,
                candidate_score: 65.0,
                delta: 25.0,
                base_completed: Some(false),
                candidate_completed: Some(true),
                error: None,
            },
        ];
        let report =
            aggregate_comparison_report(&base, &cand, Some("p1".into()), 0.0, rows).unwrap();
        assert!(report.promoted);
        assert_eq!(report.regressed_cases, vec!["case-a".to_string()]);
        assert_eq!(report.improved_cases, 1);
        assert!(report.delta > 11.0);

        // Failed case blocks promotion even with high average on other cases.
        let failed_rows = vec![
            CaseComparisonResult {
                case_id: "case-a".into(),
                lesson: FoundationTeacherLesson::Feeding,
                weight: 1.0,
                base_score: 40.0,
                candidate_score: 95.0,
                delta: 55.0,
                base_completed: Some(false),
                candidate_completed: Some(true),
                error: None,
            },
            CaseComparisonResult {
                case_id: "case-b".into(),
                lesson: FoundationTeacherLesson::MazeNavigation,
                weight: 1.0,
                base_score: 0.0,
                candidate_score: 0.0,
                delta: 0.0,
                base_completed: None,
                candidate_completed: None,
                error: Some("candidate crashed".into()),
            },
        ];
        let failed_report =
            aggregate_comparison_report(&base, &cand, Some("p2".into()), 0.0, failed_rows).unwrap();
        assert!(!failed_report.promoted);
        assert_eq!(failed_report.failed_cases, 1);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&cand_dir);
    }

    #[test]
    fn apply_patch_refuses_to_overwrite_base_or_existing_directory() {
        let dir = temp_dir("apply-no-overwrite");
        let base_dir = dir.join("base");
        let existing_dir = dir.join("existing");
        let base = init_base_bundle(&base_dir, "r41-base", None, None, None, 33).unwrap();
        std::fs::create_dir_all(&existing_dir).unwrap();
        std::fs::write(existing_dir.join("sentinel.txt"), b"keep-me").unwrap();

        let manifest_before = std::fs::read(base_dir.join("manifest.json")).unwrap();
        let foundation_before =
            std::fs::read(base_dir.join(&base.manifest.foundation_file)).unwrap();
        let branches_before =
            std::fs::read(base_dir.join(&base.manifest.dendritic_branches_file)).unwrap();

        let patch = BrainSurgeryPatchV1 {
            schema: BRAIN_SURGERY_PATCH_SCHEMA.into(),
            schema_version: BRAIN_SURGERY_SCHEMA_VERSION,
            patch_id: "p-no-overwrite".into(),
            target_base_id: base.manifest.base_id.clone(),
            target_foundation_digest: base.manifest.foundation_digest.clone(),
            target_dendritic_branches_sha256: base.manifest.dendritic_branches_sha256.clone(),
            weight_edits: vec![WeightEdit {
                index: 0,
                mode: WeightEditMode::Add,
                value: 0.01,
            }],
            dendritic_edits: DendriticPatchOp::Keep,
            notes: None,
        };
        let patch_path = dir.join("patch.json");
        std::fs::write(&patch_path, serde_json::to_vec_pretty(&patch).unwrap()).unwrap();

        let err_same = apply_patch_to_bundle(&base_dir, &patch_path, &base_dir, None)
            .expect_err("overwriting base_dir in place must fail");
        assert!(
            err_same.to_string().contains("refusing to overwrite")
                || err_same.to_string().contains("already exists")
        );

        let err_existing = apply_patch_to_bundle(&base_dir, &patch_path, &existing_dir, None)
            .expect_err("writing into existing non-empty output directory must fail");
        assert!(err_existing
            .to_string()
            .contains("refusing to overwrite existing bundle directory"));
        assert_eq!(
            std::fs::read(existing_dir.join("sentinel.txt")).unwrap(),
            b"keep-me"
        );

        assert_eq!(
            std::fs::read(base_dir.join("manifest.json")).unwrap(),
            manifest_before
        );
        assert_eq!(
            std::fs::read(base_dir.join(&base.manifest.foundation_file)).unwrap(),
            foundation_before
        );
        assert_eq!(
            std::fs::read(base_dir.join(&base.manifest.dendritic_branches_file)).unwrap(),
            branches_before
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
