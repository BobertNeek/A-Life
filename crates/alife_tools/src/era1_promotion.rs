//! Deterministic promotion and plateau derivation from sealed Era 1 receipts.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use alife_core::{
    BrainCapacityClass, CreatureGenome, Era1Ability, Era1Control, Era1EvidencePartition,
    Era1PlateauWindow, Era1TrialReceipt, FoundationGeneticIdentity, FoundationWeightAsset,
    MetricReading, OrganismId, PhenotypeHash, PolicyBackend, SensorProfile, Validate,
};
use alife_gpu_backend::closed_loop_shader_bundle_digest;
use alife_training::{Era1TrialRunEvidence, Era1TrialRunRequest, Era1TrialRunner};
use alife_world::{Era1TrialManifest, Era1WorldFamily};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ei0_exit_gate::{validate_committed_ei0_exit_gate_report, Ei0ExitGateReport},
    era1_evolution::{
        derive_era1_ecology_receipt, run_era1_evolution, Era1CandidateSelectionEvidence,
        Era1EvolutionConfig, Era1EvolutionError, Era1EvolutionReceipt,
        Era1SelectionCandidateIdentity, ERA1_SELECTION_EVIDENCE_SCHEMA_VERSION,
    },
};

pub const ERA1_PROMOTION_SCHEMA_VERSION: u16 = 1;
pub const ERA1_MINIMUM_MARGIN_Q16: i32 = 3_277;
pub const ERA1_MINIMUM_MATCHED_CELLS: u16 = 12;
pub const ERA1_PLATEAU_MAX_IMPROVEMENT_Q16: i32 = 655;
pub const ERA1_COMMITTED_PROMOTION_REPORT_SCHEMA_VERSION: u16 = 3;
pub const ERA1_CAUSAL_EVIDENCE_BUNDLE_SCHEMA_VERSION: u16 = 1;

const REQUIRED_GPU_ADAPTER: &str = "NVIDIA GeForce RTX 3050";
const REQUIRED_GPU_BACKEND_API: &str = "vulkan";
const ERA1_CAUSAL_EVIDENCE_BUNDLE_PATH: &str =
    "crates/alife_tools/reports/era1_trial_evidence.jsonl.zst";
const SOURCE_CONTRACT_PATHS: &[&str] = &[
    "Cargo.lock",
    "Cargo.toml",
    "assets/brain_foundations/n2048-v1-grounded.alife-foundation",
    "crates/alife_core/src/era1_evaluation.rs",
    "crates/alife_core/src/foundation.rs",
    "crates/alife_core/src/genome.rs",
    "crates/alife_gpu_backend/src/lib.rs",
    "crates/alife_gpu_backend/shaders/closed_loop_abi.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_activity_validation.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_clear_diagnostics.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_consolidate.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_decode.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_eligibility.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_encode.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_memory_context.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_plasticity.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_recurrent.wgsl",
    "crates/alife_gpu_backend/shaders/closed_loop_replay_learning.wgsl",
    "crates/alife_world/src/era1_trials.rs",
    "crates/alife_world/src/persistence.rs",
    "crates/alife_archive/src/lib.rs",
    "crates/alife_archive/src/bundle.rs",
    "crates/alife_training/src/era1_trials.rs",
    "crates/alife_tools/src/era1_evolution.rs",
    "crates/alife_tools/src/era1_promotion.rs",
    "crates/alife_tools/src/bin/era1_promotion.rs",
    "crates/alife_tools/tests/era1_promotion.rs",
    "docs/architecture/era1_norn_plus.md",
    "docs/architecture/evolution_genome_lab.md",
    "docs/creatures_agi_roadmap_pack/ROADMAP_OVERVIEW.md",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era1PromotionVerdict {
    Pass,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era1ComparisonStatus {
    Measured,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era1PlateauStatus {
    Measured,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Era1PromotionBlocker {
    MissingOrUnknownIntact,
    NonPositiveIntact,
    IncompleteControlComparison,
    ControlNotWorseForEveryCell,
    ControlMarginBelowMinimum,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1SubgroupScore {
    pub ability: Era1Ability,
    pub seed: u64,
    pub lineage_slot: usize,
    pub world_variant_id: u64,
    pub generation: u32,
    pub intact_q16: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1ControlComparison {
    pub ability: Era1Ability,
    pub control: Era1Control,
    pub status: Era1ComparisonStatus,
    pub matched_cells: u16,
    pub required_cells: u16,
    pub intact_mean_q16: Option<u32>,
    pub control_mean_q16: Option<u32>,
    pub margin_q16: Option<i32>,
    pub passes_minimum_margin: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1HardwareCost {
    pub adapter_name: String,
    pub backend_api: String,
    pub trial_receipts: u32,
    pub gpu_dispatches: u64,
    pub elapsed_ns: u64,
    pub peak_vram_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1PlateauAssessment {
    pub status: Era1PlateauStatus,
    pub review_eligible: bool,
    pub brain_class_change_authorized: bool,
    pub windows: Vec<Era1PlateauWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1PromotionReport {
    pub schema_version: u16,
    pub verdict: Era1PromotionVerdict,
    pub evolution_seed: u64,
    pub brain_class_id: alife_core::BrainClassId,
    pub source_commit: String,
    pub source_tree: String,
    pub subgroup_scores: Vec<Era1SubgroupScore>,
    pub control_comparisons: Vec<Era1ControlComparison>,
    pub blockers: Vec<Era1PromotionBlocker>,
    pub hardware: Era1HardwareCost,
    pub plateau: Era1PlateauAssessment,
}

#[derive(Debug, Error)]
pub enum Era1PromotionError {
    #[error("Task 5 evolution receipt is invalid: {0}")]
    InvalidEvolution(String),
    #[error("Era 1 trial receipt {index} is invalid: {reason}")]
    InvalidTrial { index: usize, reason: String },
    #[error("Era 1 promotion evidence is invalid: {0}")]
    InvalidEvidence(&'static str),
    #[error("Era 1 plateau evidence is invalid: {0}")]
    InvalidPlateau(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CellKey {
    ability: u8,
    control: u8,
    seed: u64,
    lineage_slot: usize,
    world_variant_id: u64,
    generation: u32,
}

pub fn derive_era1_promotion(
    evolution: &Era1EvolutionReceipt,
    trials: &[Era1TrialReceipt],
    hardware: Era1HardwareCost,
    plateau_windows: &[Era1PlateauWindow],
) -> Result<Era1PromotionReport, Era1PromotionError> {
    evolution
        .validate_contract()
        .map_err(|error| Era1PromotionError::InvalidEvolution(error.to_string()))?;
    validate_hardware(&hardware, trials.len())?;
    let first = trials
        .first()
        .ok_or(Era1PromotionError::InvalidEvidence("trial matrix is empty"))?;
    if hardware.adapter_name != first.adapter_name || hardware.backend_api != first.backend_api {
        return Err(Era1PromotionError::InvalidEvidence(
            "hardware identity does not match trials",
        ));
    }

    let descendants = evolution
        .generations
        .iter()
        .skip(1)
        .flat_map(|generation| {
            generation.births.iter().map(move |birth| {
                (
                    birth.genome.id.0,
                    (generation.generation, birth.lineage_slot, birth),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let mut phenotype_by_genome = BTreeMap::<u64, PhenotypeHash>::new();
    let mut family_by_ability = BTreeMap::<u8, u64>::new();
    let mut cells = BTreeMap::<CellKey, &Era1TrialReceipt>::new();

    for (index, trial) in trials.iter().enumerate() {
        trial
            .validate_contract()
            .map_err(|error| Era1PromotionError::InvalidTrial {
                index,
                reason: error.to_string(),
            })?;
        if trial.partition != Era1EvidencePartition::ReproducedOffspring
            || !trial.assistance.is_empty()
            || trial.source_commit != first.source_commit
            || trial.source_tree != first.source_tree
            || trial.adapter_name != first.adapter_name
            || trial.backend_api != first.backend_api
        {
            return Err(Era1PromotionError::InvalidEvidence(
                "trial assistance, partition, source, or hardware identity changed",
            ));
        }
        if trial.identity.world_family_id != canonical_world_family_id(trial.ability) {
            return Err(Era1PromotionError::InvalidEvidence(
                "ability is paired with the wrong canonical world family",
            ));
        }
        let Some((generation, lineage_slot, birth)) =
            descendants.get(&trial.identity.genome_id.0).copied()
        else {
            return Err(Era1PromotionError::InvalidEvidence(
                "trial genome is not a Task 5 descendant",
            ));
        };
        if trial.identity.parent_genome_ids != birth.genome.parent_genome_ids
            || trial.identity.lineage_id != birth.genome.lineage_id
            || trial.identity.generation != generation
            || trial.identity.brain_class_id != BrainCapacityClass::N2048_ID
            || trial.foundation_id != birth.genome.foundation.foundation_id
            || trial.foundation_version != u32::from(birth.genome.foundation.version)
            || !evolution
                .config
                .evaluation_seeds
                .contains(&trial.identity.seed)
            || !evolution
                .config
                .held_out_world_transforms
                .contains(&trial.identity.world_variant_id)
        {
            return Err(Era1PromotionError::InvalidEvidence(
                "trial identity does not match Task 5 provenance",
            ));
        }
        match phenotype_by_genome.entry(trial.identity.genome_id.0) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(trial.phenotype_hash);
            }
            std::collections::btree_map::Entry::Occupied(entry)
                if *entry.get() != trial.phenotype_hash =>
            {
                return Err(Era1PromotionError::InvalidEvidence(
                    "phenotype identity changed across matched trials",
                ));
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
        match family_by_ability.entry(trial.ability as u8) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(trial.identity.world_family_id);
            }
            std::collections::btree_map::Entry::Occupied(entry)
                if *entry.get() != trial.identity.world_family_id =>
            {
                return Err(Era1PromotionError::InvalidEvidence(
                    "world family changed within an ability",
                ));
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
        let key = CellKey {
            ability: trial.ability as u8,
            control: trial.control as u8,
            seed: trial.identity.seed,
            lineage_slot,
            world_variant_id: trial.identity.world_variant_id,
            generation,
        };
        if cells.insert(key, trial).is_some() {
            return Err(Era1PromotionError::InvalidEvidence(
                "duplicate or contaminated trial cell",
            ));
        }
    }

    let expected_subgroups = expected_subgroup_count(evolution)?;
    let expected_cells = expected_subgroups
        .checked_mul(Era1Control::ALL.len())
        .ok_or(Era1PromotionError::InvalidEvidence("matrix size overflow"))?;
    if trials.len() > expected_cells {
        return Err(Era1PromotionError::InvalidEvidence(
            "trial matrix contains unexpected cells",
        ));
    }

    let mut blockers = BTreeSet::new();
    let mut subgroup_scores = Vec::with_capacity(expected_subgroups);
    for ability in Era1Ability::ALL {
        for generation in evolution.generations.iter().skip(1) {
            for birth in &generation.births {
                for &seed in &evolution.config.evaluation_seeds {
                    for &world_variant_id in &evolution.config.held_out_world_transforms {
                        let key = CellKey {
                            ability: ability as u8,
                            control: Era1Control::Intact as u8,
                            seed,
                            lineage_slot: birth.lineage_slot,
                            world_variant_id,
                            generation: generation.generation,
                        };
                        let intact_q16 = cells.get(&key).and_then(|trial| trial.score.value_q16());
                        match intact_q16 {
                            None => {
                                blockers.insert(Era1PromotionBlocker::MissingOrUnknownIntact);
                            }
                            Some(0) => {
                                blockers.insert(Era1PromotionBlocker::NonPositiveIntact);
                            }
                            Some(_) => {}
                        }
                        subgroup_scores.push(Era1SubgroupScore {
                            ability,
                            seed,
                            lineage_slot: birth.lineage_slot,
                            world_variant_id,
                            generation: generation.generation,
                            intact_q16,
                        });
                    }
                }
            }
        }
    }

    let required_cells = u16::try_from(expected_subgroups / Era1Ability::ALL.len())
        .map_err(|_| Era1PromotionError::InvalidEvidence("comparison size overflow"))?;
    let mut control_comparisons =
        Vec::with_capacity(Era1Ability::ALL.len() * (Era1Control::ALL.len() - 1));
    for ability in Era1Ability::ALL {
        for control in Era1Control::ALL.into_iter().skip(1) {
            let mut matched_cells = 0u16;
            let mut intact_sum = 0u64;
            let mut control_sum = 0u64;
            let mut every_cell_worse = true;
            for generation in evolution.generations.iter().skip(1) {
                for birth in &generation.births {
                    for &seed in &evolution.config.evaluation_seeds {
                        for &world_variant_id in &evolution.config.held_out_world_transforms {
                            let base = CellKey {
                                ability: ability as u8,
                                control: Era1Control::Intact as u8,
                                seed,
                                lineage_slot: birth.lineage_slot,
                                world_variant_id,
                                generation: generation.generation,
                            };
                            let disabled = CellKey {
                                control: control as u8,
                                ..base
                            };
                            let intact = cells.get(&base).and_then(|trial| trial.score.value_q16());
                            let disabled = cells
                                .get(&disabled)
                                .and_then(|trial| trial.score.value_q16());
                            if let (Some(intact), Some(disabled)) = (intact, disabled) {
                                matched_cells = matched_cells.checked_add(1).ok_or(
                                    Era1PromotionError::InvalidEvidence("matched cell overflow"),
                                )?;
                                intact_sum += u64::from(intact);
                                control_sum += u64::from(disabled);
                                every_cell_worse &= intact > disabled;
                            }
                        }
                    }
                }
            }

            let status =
                if matched_cells == required_cells && matched_cells >= ERA1_MINIMUM_MATCHED_CELLS {
                    Era1ComparisonStatus::Measured
                } else {
                    blockers.insert(Era1PromotionBlocker::IncompleteControlComparison);
                    Era1ComparisonStatus::Unknown
                };
            let (intact_mean_q16, control_mean_q16, margin_q16) = if status
                == Era1ComparisonStatus::Measured
            {
                let count = u64::from(matched_cells);
                let intact_mean = u32::try_from(intact_sum / count)
                    .map_err(|_| Era1PromotionError::InvalidEvidence("intact mean overflow"))?;
                let control_mean = u32::try_from(control_sum / count)
                    .map_err(|_| Era1PromotionError::InvalidEvidence("control mean overflow"))?;
                let margin = i32::try_from(i64::from(intact_mean) - i64::from(control_mean))
                    .map_err(|_| Era1PromotionError::InvalidEvidence("margin overflow"))?;
                (Some(intact_mean), Some(control_mean), Some(margin))
            } else {
                (None, None, None)
            };
            let passes_minimum_margin = status == Era1ComparisonStatus::Measured
                && every_cell_worse
                && margin_q16.is_some_and(|margin| margin >= ERA1_MINIMUM_MARGIN_Q16);
            if status == Era1ComparisonStatus::Measured {
                if !every_cell_worse {
                    blockers.insert(Era1PromotionBlocker::ControlNotWorseForEveryCell);
                }
                if margin_q16.is_none_or(|margin| margin < ERA1_MINIMUM_MARGIN_Q16) {
                    blockers.insert(Era1PromotionBlocker::ControlMarginBelowMinimum);
                }
            }
            control_comparisons.push(Era1ControlComparison {
                ability,
                control,
                status,
                matched_cells,
                required_cells,
                intact_mean_q16,
                control_mean_q16,
                margin_q16,
                passes_minimum_margin,
            });
        }
    }

    let plateau = assess_era1_plateau(plateau_windows)?;
    let blockers = blockers.into_iter().collect::<Vec<_>>();
    let verdict = if blockers.is_empty()
        && trials.len() == expected_cells
        && control_comparisons
            .iter()
            .all(|comparison| comparison.passes_minimum_margin)
    {
        Era1PromotionVerdict::Pass
    } else {
        Era1PromotionVerdict::Blocked
    };
    Ok(Era1PromotionReport {
        schema_version: ERA1_PROMOTION_SCHEMA_VERSION,
        verdict,
        evolution_seed: evolution.config.evolution_seed,
        brain_class_id: BrainCapacityClass::N2048_ID,
        source_commit: first.source_commit.clone(),
        source_tree: first.source_tree.clone(),
        subgroup_scores,
        control_comparisons,
        blockers,
        hardware,
        plateau,
    })
}

pub const fn canonical_world_family_id(ability: Era1Ability) -> u64 {
    match ability {
        Era1Ability::FlexibleForaging | Era1Ability::HazardAvoidance => {
            Era1WorldFamily::ForagingHazardMaze as u64
        }
        Era1Ability::SpatialMemory
        | Era1Ability::DelayedChoice
        | Era1Ability::PostSleepRetention => Era1WorldFamily::DelayedLocation as u64,
        Era1Ability::RewardReversal => Era1WorldFamily::RewardReversal as u64,
        Era1Ability::ObjectTransfer => Era1WorldFamily::TransformedObjectsLayout as u64,
        Era1Ability::MultiStepProblem => Era1WorldFamily::TwoStepAccessProblem as u64,
        Era1Ability::IndividualRecognition => Era1WorldFamily::FamiliarNovelIndividual as u64,
        Era1Ability::Imitation => Era1WorldFamily::PeerDemonstration as u64,
        Era1Ability::GroundedLanguage => Era1WorldFamily::GroundedVocabulary as u64,
    }
}

pub fn assess_era1_plateau(
    windows: &[Era1PlateauWindow],
) -> Result<Era1PlateauAssessment, Era1PromotionError> {
    for window in windows {
        window
            .validate_contract()
            .map_err(|error| Era1PromotionError::InvalidPlateau(error.to_string()))?;
    }
    if windows.len() < 3 {
        return Ok(Era1PlateauAssessment {
            status: Era1PlateauStatus::Unknown,
            review_eligible: false,
            brain_class_change_authorized: false,
            windows: windows.to_vec(),
        });
    }
    let review_eligible = windows.windows(3).any(|group| {
        group[1].first_generation == group[0].first_generation + 1
            && group[2].first_generation == group[1].first_generation + 1
            && group[1].last_generation == group[0].last_generation + 1
            && group[2].last_generation == group[1].last_generation + 1
            && group.iter().all(|window| {
                (0..=ERA1_PLATEAU_MAX_IMPROVEMENT_Q16).contains(&window.improvement_q16)
                    && !window.ecological_regression
                    && !window.diversity_regression
            })
    });
    Ok(Era1PlateauAssessment {
        status: Era1PlateauStatus::Measured,
        review_eligible,
        brain_class_change_authorized: false,
        windows: windows.to_vec(),
    })
}

fn validate_hardware(
    hardware: &Era1HardwareCost,
    trial_count: usize,
) -> Result<(), Era1PromotionError> {
    if hardware.adapter_name.trim().is_empty()
        || hardware.backend_api != "vulkan"
        || usize::try_from(hardware.trial_receipts).ok() != Some(trial_count)
        || hardware.gpu_dispatches == 0
        || hardware.elapsed_ns == 0
        || hardware.peak_vram_bytes == 0
    {
        return Err(Era1PromotionError::InvalidEvidence(
            "hardware costs are missing or do not match the trial matrix",
        ));
    }
    Ok(())
}

fn expected_subgroup_count(evolution: &Era1EvolutionReceipt) -> Result<usize, Era1PromotionError> {
    Era1Ability::ALL
        .len()
        .checked_mul(evolution.config.evaluation_seeds.len())
        .and_then(|count| count.checked_mul(evolution.config.lineage_count))
        .and_then(|count| count.checked_mul(evolution.config.held_out_world_transforms.len()))
        .and_then(|count| count.checked_mul(evolution.config.ordinary_birth_generations as usize))
        .ok_or(Era1PromotionError::InvalidEvidence("matrix size overflow"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Era1EvidenceStatus {
    Measured,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1MatrixCoverage {
    pub ability: Era1Ability,
    pub control: Era1Control,
    pub required_receipts: u16,
    pub observed_receipts: u16,
    pub status: Era1EvidenceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1ArtifactBinding {
    pub producing_source_commit: String,
    pub producing_source_tree: String,
    pub source_contract_paths: Vec<String>,
    pub source_contract_digest: String,
    pub adapter_name: String,
    pub backend_api: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1EvidenceDigests {
    pub genome_receipts: String,
    pub foundation_weights: String,
    pub wgsl_bundle: String,
    pub world_receipts: String,
    pub portable_save: String,
    pub archive_receipts: String,
    pub trial_receipts: String,
    pub causal_trial_evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1ProgramBoundaries {
    pub assistance_present: bool,
    pub hidden_policy_present: bool,
    pub brain_class_scaling_performed: bool,
    pub era2_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Era1CausalEvidenceBundle {
    pub schema_version: u16,
    pub relative_path: String,
    pub evidence_count: u32,
    pub uncompressed_digest: String,
    pub compressed_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Era1CommittedPromotionReport {
    pub schema_version: u16,
    pub artifact_binding: Era1ArtifactBinding,
    pub baseline_save_archive_receipt: Ei0ExitGateReport,
    pub evolution: Era1EvolutionReceipt,
    pub causal_evidence_bundle: Era1CausalEvidenceBundle,
    pub trial_receipts: Vec<Era1TrialReceipt>,
    pub matrix_coverage: Vec<Era1MatrixCoverage>,
    pub promotion: Era1PromotionReport,
    pub evidence_digests: Era1EvidenceDigests,
    pub boundaries: Era1ProgramBoundaries,
}

#[derive(Debug, Error)]
pub enum Era1CommittedReportError {
    #[error("Era 1 evidence generation failed: {0}")]
    Generation(String),
    #[error("Era 1 committed evidence is inconsistent: {0}")]
    Evidence(&'static str),
    #[error("Era 1 report I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("Era 1 report JSON failed: {0}")]
    Json(#[from] serde_json::Error),
}

struct CausalEvidenceBundleWriter {
    temp_path: PathBuf,
    final_path: PathBuf,
    relative_path: String,
    encoder: zstd::stream::write::Encoder<'static, File>,
    uncompressed_hasher: blake3::Hasher,
    receipts: Vec<Era1TrialReceipt>,
    gpu_dispatches: u64,
    elapsed_ns: u64,
    peak_vram_bytes: u64,
}

impl CausalEvidenceBundleWriter {
    fn new(final_path: PathBuf, relative_path: String) -> Result<Self, Era1CommittedReportError> {
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp_path = final_path.with_extension(format!("zst.{}.tmp", std::process::id()));
        let encoder = zstd::stream::write::Encoder::new(File::create(&temp_path)?, 9)?;
        Ok(Self {
            temp_path,
            final_path,
            relative_path,
            encoder,
            uncompressed_hasher: blake3::Hasher::new(),
            receipts: Vec::with_capacity(2_640),
            gpu_dispatches: 0,
            elapsed_ns: 0,
            peak_vram_bytes: observed_rtx_vram_bytes()?,
        })
    }

    fn record_batch(
        &mut self,
        evidence: &[Era1TrialRunEvidence],
        elapsed_ns: u64,
    ) -> Result<(), Era1CommittedReportError> {
        for item in evidence {
            item.validate_contract()
                .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
            let encoded = serde_json::to_vec(item)?;
            self.encoder.write_all(&encoded)?;
            self.encoder.write_all(b"\n")?;
            self.uncompressed_hasher.update(&encoded);
            self.uncompressed_hasher.update(b"\n");
            self.gpu_dispatches = self.gpu_dispatches.saturating_add(item.gpu_dispatches);
            self.receipts.push(item.receipt.clone());
        }
        self.elapsed_ns = self.elapsed_ns.saturating_add(elapsed_ns);
        self.peak_vram_bytes = self.peak_vram_bytes.max(observed_rtx_vram_bytes()?);
        Ok(())
    }

    fn finish(
        mut self,
    ) -> Result<
        (
            Era1CausalEvidenceBundle,
            Vec<Era1TrialReceipt>,
            Era1HardwareCost,
        ),
        Era1CommittedReportError,
    > {
        self.encoder.flush()?;
        let file = self.encoder.finish()?;
        file.sync_all()?;
        if self.final_path.is_file() {
            fs::remove_file(&self.final_path)?;
        }
        fs::rename(&self.temp_path, &self.final_path)?;
        let evidence_count = u32::try_from(self.receipts.len())
            .map_err(|_| Era1CommittedReportError::Evidence("causal evidence count overflow"))?;
        let bundle = Era1CausalEvidenceBundle {
            schema_version: ERA1_CAUSAL_EVIDENCE_BUNDLE_SCHEMA_VERSION,
            relative_path: self.relative_path,
            evidence_count,
            uncompressed_digest: format!(
                "blake3-256:{}",
                self.uncompressed_hasher.finalize().to_hex()
            ),
            compressed_digest: digest_file(&self.final_path)?,
        };
        let hardware = Era1HardwareCost {
            adapter_name: REQUIRED_GPU_ADAPTER.to_string(),
            backend_api: REQUIRED_GPU_BACKEND_API.to_string(),
            trial_receipts: evidence_count,
            gpu_dispatches: self.gpu_dispatches,
            elapsed_ns: self.elapsed_ns,
            peak_vram_bytes: self.peak_vram_bytes,
        };
        Ok((bundle, self.receipts, hardware))
    }
}

pub fn run_era1_promotion_report() -> Result<Era1CommittedPromotionReport, Era1CommittedReportError>
{
    let root = workspace_root();
    generate_era1_promotion_report(
        root.join(ERA1_CAUSAL_EVIDENCE_BUNDLE_PATH),
        ERA1_CAUSAL_EVIDENCE_BUNDLE_PATH.to_string(),
    )
}

fn generate_era1_promotion_report(
    bundle_path: PathBuf,
    bundle_relative_path: String,
) -> Result<Era1CommittedPromotionReport, Era1CommittedReportError> {
    let root = workspace_root();
    let producing_source_commit = git_output(&root, &["rev-parse", "HEAD"])?;
    let producing_source_tree = git_output(&root, &["rev-parse", "HEAD^{tree}"])?;
    require_clean_source_contract(&root, &producing_source_commit)?;

    let temp = std::env::temp_dir().join(format!(
        "alife-era1-evidence-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?
            .as_nanos()
    ));
    fs::create_dir_all(&temp)?;
    let baseline: Ei0ExitGateReport = serde_json::from_slice(&fs::read(
        root.join("crates/alife_tools/reports/ei0_exit_gate_report.json"),
    )?)?;
    validate_committed_ei0_exit_gate_report(&baseline)
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;

    let mut evidence_bundle = CausalEvidenceBundleWriter::new(bundle_path, bundle_relative_path)?;
    let evolution = bounded_evolution(
        &temp.join("era1"),
        &baseline,
        &producing_source_commit,
        &producing_source_tree,
        &mut evidence_bundle,
    )?;
    record_final_descendant_generation(
        &evolution,
        &producing_source_commit,
        &producing_source_tree,
        &mut evidence_bundle,
    )?;
    let (causal_evidence_bundle, trial_receipts, hardware) = evidence_bundle.finish()?;
    let promotion = derive_era1_promotion(&evolution, &trial_receipts, hardware, &[])
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    let matrix_coverage = derive_matrix_coverage(&evolution, &trial_receipts)?;
    let source_contract_digest =
        source_contract_digest_at_revision(&root, &producing_source_commit, SOURCE_CONTRACT_PATHS)?;
    let artifact_binding = Era1ArtifactBinding {
        producing_source_commit,
        producing_source_tree,
        source_contract_paths: SOURCE_CONTRACT_PATHS
            .iter()
            .map(|path| (*path).to_string())
            .collect(),
        source_contract_digest,
        adapter_name: REQUIRED_GPU_ADAPTER.to_string(),
        backend_api: REQUIRED_GPU_BACKEND_API.to_string(),
    };
    let evidence_digests = derive_evidence_digests(
        &baseline,
        &evolution,
        &trial_receipts,
        &causal_evidence_bundle.uncompressed_digest,
    )?;
    let report = Era1CommittedPromotionReport {
        schema_version: ERA1_COMMITTED_PROMOTION_REPORT_SCHEMA_VERSION,
        artifact_binding,
        baseline_save_archive_receipt: baseline,
        evolution,
        causal_evidence_bundle,
        trial_receipts,
        matrix_coverage,
        promotion,
        evidence_digests,
        boundaries: Era1ProgramBoundaries {
            assistance_present: false,
            hidden_policy_present: false,
            brain_class_scaling_performed: false,
            era2_status: "OUT_OF_SCOPE".to_string(),
        },
    };
    validate_committed_era1_promotion_report(&report)?;
    let _ = fs::remove_dir_all(&temp);
    Ok(report)
}

pub fn run_era1_promotion_and_write(
    output: impl AsRef<Path>,
) -> Result<Era1CommittedPromotionReport, Era1CommittedReportError> {
    let output = output.as_ref();
    let root = workspace_root();
    let absolute_output = if output.is_absolute() {
        output.to_path_buf()
    } else {
        root.join(output)
    };
    if let Some(parent) = absolute_output.parent() {
        fs::create_dir_all(parent)?;
    }
    let bundle_path = absolute_output.with_file_name("era1_trial_evidence.jsonl.zst");
    let bundle_relative_path = bundle_path
        .strip_prefix(&root)
        .map_err(|_| Era1CommittedReportError::Evidence("evidence output is outside workspace"))?
        .to_string_lossy()
        .replace('\\', "/");
    let report = generate_era1_promotion_report(bundle_path, bundle_relative_path)?;
    fs::write(absolute_output, serde_json::to_vec_pretty(&report)?)?;
    Ok(report)
}

pub fn validate_committed_era1_promotion_report(
    report: &Era1CommittedPromotionReport,
) -> Result<(), Era1CommittedReportError> {
    if report.schema_version != ERA1_COMMITTED_PROMOTION_REPORT_SCHEMA_VERSION
        || report.boundaries.assistance_present
        || report.boundaries.hidden_policy_present
        || report.boundaries.brain_class_scaling_performed
        || report.boundaries.era2_status != "OUT_OF_SCOPE"
    {
        return Err(Era1CommittedReportError::Evidence(
            "schema or program boundary changed",
        ));
    }
    validate_committed_ei0_exit_gate_report(&report.baseline_save_archive_receipt)
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    report
        .evolution
        .validate_contract()
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    let causal_evidence_digest = validate_causal_evidence_bundle(report)?;
    if report.evolution.baseline_ei0_exit_gate != report.baseline_save_archive_receipt
        || report.trial_receipts.is_empty()
        || report.trial_receipts.iter().any(|trial| {
            trial.validate_contract().is_err()
                || !trial.assistance.is_empty()
                || trial.policy_backend != PolicyBackend::NeuralClosedLoopGpu
                || trial.identity.brain_class_id != BrainCapacityClass::N2048_ID
                || trial.source_commit != report.artifact_binding.producing_source_commit
                || trial.source_tree != report.artifact_binding.producing_source_tree
        })
    {
        return Err(Era1CommittedReportError::Evidence(
            "trial receipts are missing, assisted, non-GPU, scaled, or source-mismatched",
        ));
    }
    let expected_promotion = derive_era1_promotion(
        &report.evolution,
        &report.trial_receipts,
        report.promotion.hardware.clone(),
        &report.promotion.plateau.windows,
    )
    .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    if expected_promotion != report.promotion
        || report.promotion.verdict != Era1PromotionVerdict::Blocked
        || report.promotion.plateau.status != Era1PlateauStatus::Unknown
        || report.promotion.plateau.brain_class_change_authorized
        || report.matrix_coverage
            != derive_matrix_coverage(&report.evolution, &report.trial_receipts)?
        || report.matrix_coverage.len() != Era1Ability::ALL.len() * Era1Control::ALL.len()
    {
        return Err(Era1CommittedReportError::Evidence(
            "promotion, plateau, or matrix coverage does not recompute",
        ));
    }
    let binding = &report.artifact_binding;
    let expected_paths = SOURCE_CONTRACT_PATHS
        .iter()
        .map(|path| (*path).to_string())
        .collect::<Vec<_>>();
    let root = workspace_root();
    if binding.source_contract_paths != expected_paths
        || binding.adapter_name != REQUIRED_GPU_ADAPTER
        || binding.backend_api != REQUIRED_GPU_BACKEND_API
        || report.promotion.hardware.adapter_name != REQUIRED_GPU_ADAPTER
        || report.promotion.hardware.backend_api != REQUIRED_GPU_BACKEND_API
        || report
            .evolution
            .selection_rounds
            .iter()
            .flat_map(|round| &round.derived_profiles)
            .any(|profile| {
                profile.source_commit != binding.producing_source_commit
                    || profile.source_tree != binding.producing_source_tree
            })
        || source_contract_digest_at_revision(
            &root,
            &binding.producing_source_commit,
            SOURCE_CONTRACT_PATHS,
        )? != binding.source_contract_digest
        || git_output(
            &root,
            &[
                "rev-parse",
                &format!("{}^{{tree}}", binding.producing_source_commit),
            ],
        )? != binding.producing_source_tree
    {
        return Err(Era1CommittedReportError::Evidence(
            "source or hardware binding does not recompute",
        ));
    }
    require_clean_source_contract(&root, &binding.producing_source_commit)?;
    if derive_evidence_digests(
        &report.baseline_save_archive_receipt,
        &report.evolution,
        &report.trial_receipts,
        &causal_evidence_digest,
    )? != report.evidence_digests
    {
        return Err(Era1CommittedReportError::Evidence(
            "genome, foundation, WGSL, world, save, archive, or trial digest changed",
        ));
    }
    Ok(())
}

fn validate_causal_evidence_bundle(
    report: &Era1CommittedPromotionReport,
) -> Result<String, Era1CommittedReportError> {
    let bundle = &report.causal_evidence_bundle;
    if bundle.schema_version != ERA1_CAUSAL_EVIDENCE_BUNDLE_SCHEMA_VERSION
        || bundle.relative_path != ERA1_CAUSAL_EVIDENCE_BUNDLE_PATH
        || usize::try_from(bundle.evidence_count).ok() != Some(report.trial_receipts.len())
    {
        return Err(Era1CommittedReportError::Evidence(
            "causal evidence bundle metadata changed",
        ));
    }
    let path = workspace_root().join(&bundle.relative_path);
    if digest_file(&path)? != bundle.compressed_digest {
        return Err(Era1CommittedReportError::Evidence(
            "compressed causal evidence digest changed",
        ));
    }
    let decoder = zstd::stream::read::Decoder::new(File::open(path)?)?;
    let mut reader = BufReader::new(decoder);
    let mut encoded = String::new();
    let mut hasher = blake3::Hasher::new();
    let mut count = 0usize;
    loop {
        encoded.clear();
        if reader.read_line(&mut encoded)? == 0 {
            break;
        }
        let receipt =
            report
                .trial_receipts
                .get(count)
                .ok_or(Era1CommittedReportError::Evidence(
                    "causal evidence bundle has extra records",
                ))?;
        let evidence: Era1TrialRunEvidence =
            serde_json::from_str(encoded.trim_end_matches(['\r', '\n']))?;
        evidence
            .validate_contract()
            .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
        if evidence.receipt != *receipt {
            return Err(Era1CommittedReportError::Evidence(
                "causal evidence does not match its trial receipt",
            ));
        }
        hasher.update(encoded.as_bytes());
        count += 1;
    }
    let digest = format!("blake3-256:{}", hasher.finalize().to_hex());
    if count != report.trial_receipts.len() || digest != bundle.uncompressed_digest {
        return Err(Era1CommittedReportError::Evidence(
            "causal evidence bundle is incomplete or changed",
        ));
    }
    Ok(digest)
}

fn bounded_evolution(
    artifact_root: &Path,
    baseline: &Ei0ExitGateReport,
    source_commit: &str,
    source_tree: &str,
    promotion_evidence: &mut CausalEvidenceBundleWriter,
) -> Result<Era1EvolutionReceipt, Era1CommittedReportError> {
    let foundation = FoundationGeneticIdentity::new(
        0x4E32_3034_385F_5631,
        1,
        0x4E32_3034_385F_FA11,
        BrainCapacityClass::N2048_ID,
    )
    .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    let founders = [61_001, 61_002, 61_003, 61_004]
        .into_iter()
        .map(|seed| CreatureGenome::early_mammal_founder(seed, foundation))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    let config = Era1EvolutionConfig::bounded_default(0xE1_6001)
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    let mut runner = Era1TrialRunner::new_required()
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    let cache_root = workspace_root()
        .join("target")
        .join("era1-trial-cache")
        .join(source_commit);
    run_era1_evolution(
        &config,
        Some(baseline),
        &founders,
        artifact_root,
        |generation, candidate_index, births| {
            let started = Instant::now();
            let evidence = bounded_selection_evidence(
                &config,
                generation,
                candidate_index,
                births,
                source_commit,
                source_tree,
                &cache_root,
                &mut runner,
            )?;
            if generation > 0 {
                let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
                promotion_evidence
                    .record_batch(&evidence.trial_evidence, elapsed_ns)
                    .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
            }
            Ok(evidence)
        },
    )
    .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))
}

fn record_final_descendant_generation(
    evolution: &Era1EvolutionReceipt,
    source_commit: &str,
    source_tree: &str,
    promotion_evidence: &mut CausalEvidenceBundleWriter,
) -> Result<(), Era1CommittedReportError> {
    let births = evolution
        .generations
        .last()
        .filter(|generation| generation.generation == evolution.config.ordinary_birth_generations)
        .map(|generation| generation.births.as_slice())
        .ok_or(Era1CommittedReportError::Evidence(
            "bounded evolution omitted its final descendant generation",
        ))?;
    let cache_root = workspace_root()
        .join("target")
        .join("era1-trial-cache")
        .join(source_commit);
    let mut runner = Era1TrialRunner::new_required()
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    for candidate_index in 0..births.len() {
        let started = Instant::now();
        let evidence = bounded_selection_evidence(
            &evolution.config,
            evolution.config.ordinary_birth_generations,
            candidate_index,
            births,
            source_commit,
            source_tree,
            &cache_root,
            &mut runner,
        )
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
        let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        promotion_evidence.record_batch(&evidence.trial_evidence, elapsed_ns)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn bounded_selection_evidence(
    config: &Era1EvolutionConfig,
    generation: u32,
    candidate_index: usize,
    births: &[crate::era1_evolution::Era1BirthReceipt],
    source_commit: &str,
    source_tree: &str,
    cache_root: &Path,
    runner: &mut Era1TrialRunner,
) -> Result<Era1CandidateSelectionEvidence, Era1EvolutionError> {
    let birth = births
        .get(candidate_index)
        .ok_or(Era1EvolutionError::InvalidEvidence(
            "selection candidate index is out of range",
        ))?;
    let partner = births.get((candidate_index + 1) % births.len()).ok_or(
        Era1EvolutionError::InvalidEvidence("ecology reproduction partner is missing"),
    )?;
    let identity = Era1SelectionCandidateIdentity {
        organism_id: birth.organism_id,
        genome_id: birth.genome.id,
        parent_genome_ids: birth.genome.parent_genome_ids.clone(),
        lineage_id: birth.genome.lineage_id,
        generation,
    };
    let partition = if generation == 0 {
        Era1EvidencePartition::HeldOutTransfer
    } else {
        Era1EvidencePartition::ReproducedOffspring
    };
    let mut trial_evidence = Vec::with_capacity(
        config.evaluation_seeds.len()
            * config.held_out_world_transforms.len()
            * Era1Ability::ALL.len()
            * config.controls.len(),
    );
    for &seed in &config.evaluation_seeds {
        for &world in &config.held_out_world_transforms {
            for ability in Era1Ability::ALL {
                for &control in &config.controls {
                    let subject = birth.organism_id;
                    let manifest = Era1TrialManifest::new(
                        seed,
                        world_family(ability),
                        subject,
                        OrganismId(subject.raw().wrapping_add(10_000_001)),
                        OrganismId(subject.raw().wrapping_add(10_000_002)),
                        world,
                        true,
                        birth.inherited_starter_tokens[0].raw(),
                    )
                    .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
                    trial_evidence.push(load_or_run_trial_evidence(
                        cache_root,
                        runner,
                        birth,
                        &manifest,
                        ability,
                        control,
                        partition,
                        source_commit,
                        source_tree,
                    )?);
                }
            }
        }
    }

    let mut ecology_receipts =
        Vec::with_capacity(config.evaluation_seeds.len() * config.held_out_world_transforms.len());
    for (seed_index, &seed) in config.evaluation_seeds.iter().enumerate() {
        for (world_index, &world) in config.held_out_world_transforms.iter().enumerate() {
            let reproduction_seed = ecology_reproduction_seed(
                config.evolution_seed,
                generation,
                candidate_index,
                seed_index,
                world_index,
            );
            ecology_receipts.push(derive_era1_ecology_receipt(
                &identity,
                &birth.genome,
                &partner.genome,
                reproduction_seed,
                seed,
                world,
                &trial_evidence,
            )?);
        }
    }
    Ok(Era1CandidateSelectionEvidence {
        schema_version: ERA1_SELECTION_EVIDENCE_SCHEMA_VERSION,
        identity,
        trial_evidence,
        ecology_receipts,
    })
}

#[allow(clippy::too_many_arguments)]
fn load_or_run_trial_evidence(
    cache_root: &Path,
    runner: &mut Era1TrialRunner,
    birth: &crate::era1_evolution::Era1BirthReceipt,
    manifest: &Era1TrialManifest,
    ability: Era1Ability,
    control: Era1Control,
    partition: Era1EvidencePartition,
    source_commit: &str,
    source_tree: &str,
) -> Result<Era1TrialRunEvidence, Era1EvolutionError> {
    let cache_input = serde_json::json!({
        "schema": 1,
        "organism_id": birth.organism_id,
        "generation": birth.generation,
        "genome": birth.genome,
        "manifest": manifest,
        "ability": ability,
        "control": control,
        "partition": partition,
        "source_commit": source_commit,
        "source_tree": source_tree,
    });
    let cache_bytes = serde_json::to_vec(&cache_input)
        .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    let cache_key = blake3::hash(&cache_bytes).to_hex().to_string();
    let cache_path = cache_root.join(format!("{cache_key}.json.gz"));
    if cache_path.is_file() {
        let file = File::open(&cache_path)
            .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
        let evidence: Era1TrialRunEvidence = serde_json::from_reader(GzDecoder::new(file))
            .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
        validate_cached_trial(
            &evidence,
            birth,
            manifest,
            ability,
            control,
            partition,
            source_commit,
            source_tree,
        )?;
        return Ok(evidence);
    }

    fs::create_dir_all(cache_root)
        .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    let request = Era1TrialRunRequest::new(
        birth.organism_id,
        birth.generation,
        &birth.genome,
        manifest,
        ability,
        control,
        partition,
        source_commit,
        source_tree,
    )
    .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    let evidence = runner
        .run(request)
        .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    validate_cached_trial(
        &evidence,
        birth,
        manifest,
        ability,
        control,
        partition,
        source_commit,
        source_tree,
    )?;

    let temp_path = cache_root.join(format!(".{cache_key}-{}.tmp", std::process::id()));
    let file = File::create(&temp_path)
        .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    let mut encoder = GzEncoder::new(file, Compression::fast());
    serde_json::to_writer(&mut encoder, &evidence)
        .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    encoder
        .flush()
        .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    encoder
        .finish()
        .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    fs::rename(&temp_path, &cache_path)
        .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    Ok(evidence)
}

#[allow(clippy::too_many_arguments)]
fn validate_cached_trial(
    evidence: &Era1TrialRunEvidence,
    birth: &crate::era1_evolution::Era1BirthReceipt,
    manifest: &Era1TrialManifest,
    ability: Era1Ability,
    control: Era1Control,
    partition: Era1EvidencePartition,
    source_commit: &str,
    source_tree: &str,
) -> Result<(), Era1EvolutionError> {
    evidence
        .validate_contract()
        .map_err(|error| Era1EvolutionError::TrialEvidence(error.to_string()))?;
    let receipt = &evidence.receipt;
    if evidence.manifest != *manifest
        || receipt.identity.organism_id != birth.organism_id
        || receipt.identity.genome_id != birth.genome.id
        || receipt.identity.parent_genome_ids != birth.genome.parent_genome_ids
        || receipt.identity.lineage_id != birth.genome.lineage_id
        || receipt.identity.generation != birth.generation
        || receipt.ability != ability
        || receipt.control != control
        || receipt.partition != partition
        || receipt.source_commit != source_commit
        || receipt.source_tree != source_tree
    {
        return Err(Era1EvolutionError::TrialEvidence(
            "cached trial does not match its immutable input digest".to_string(),
        ));
    }
    Ok(())
}

fn ecology_reproduction_seed(
    evolution_seed: u64,
    generation: u32,
    candidate_index: usize,
    seed_index: usize,
    world_index: usize,
) -> u64 {
    let mut value = evolution_seed
        ^ u64::from(generation).rotate_left(11)
        ^ (candidate_index as u64).rotate_left(23)
        ^ (seed_index as u64).rotate_left(37)
        ^ (world_index as u64).rotate_left(47)
        ^ 0xE1C0_10C0_11EC_7001;
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    (value ^ (value >> 31)).max(1)
}

const fn world_family(ability: Era1Ability) -> Era1WorldFamily {
    match ability {
        Era1Ability::FlexibleForaging | Era1Ability::HazardAvoidance => {
            Era1WorldFamily::ForagingHazardMaze
        }
        Era1Ability::SpatialMemory
        | Era1Ability::DelayedChoice
        | Era1Ability::PostSleepRetention => Era1WorldFamily::DelayedLocation,
        Era1Ability::RewardReversal => Era1WorldFamily::RewardReversal,
        Era1Ability::ObjectTransfer => Era1WorldFamily::TransformedObjectsLayout,
        Era1Ability::MultiStepProblem => Era1WorldFamily::TwoStepAccessProblem,
        Era1Ability::IndividualRecognition => Era1WorldFamily::FamiliarNovelIndividual,
        Era1Ability::Imitation => Era1WorldFamily::PeerDemonstration,
        Era1Ability::GroundedLanguage => Era1WorldFamily::GroundedVocabulary,
    }
}

fn derive_matrix_coverage(
    evolution: &Era1EvolutionReceipt,
    trials: &[Era1TrialReceipt],
) -> Result<Vec<Era1MatrixCoverage>, Era1CommittedReportError> {
    let required_receipts = u16::try_from(
        evolution.config.evaluation_seeds.len()
            * evolution.config.lineage_count
            * evolution.config.held_out_world_transforms.len()
            * evolution.config.ordinary_birth_generations as usize,
    )
    .map_err(|_| Era1CommittedReportError::Evidence("matrix size overflow"))?;
    let mut coverage = Vec::with_capacity(Era1Ability::ALL.len() * Era1Control::ALL.len());
    for ability in Era1Ability::ALL {
        for control in Era1Control::ALL {
            let matching = trials
                .iter()
                .filter(|trial| trial.ability == ability && trial.control == control)
                .collect::<Vec<_>>();
            let observed_receipts = u16::try_from(matching.len())
                .map_err(|_| Era1CommittedReportError::Evidence("matrix count overflow"))?;
            let status = if observed_receipts == required_receipts
                && matching
                    .iter()
                    .all(|trial| matches!(trial.score, MetricReading::Measured { .. }))
            {
                Era1EvidenceStatus::Measured
            } else {
                Era1EvidenceStatus::Unknown
            };
            coverage.push(Era1MatrixCoverage {
                ability,
                control,
                required_receipts,
                observed_receipts,
                status,
            });
        }
    }
    Ok(coverage)
}

fn derive_evidence_digests(
    baseline: &Ei0ExitGateReport,
    evolution: &Era1EvolutionReceipt,
    trials: &[Era1TrialReceipt],
    causal_trial_evidence_digest: &str,
) -> Result<Era1EvidenceDigests, Era1CommittedReportError> {
    let foundation = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    let portable_save = baseline.evidence_digests.portable_save.clone().ok_or(
        Era1CommittedReportError::Evidence("baseline portable save digest is UNKNOWN"),
    )?;
    if baseline.evidence_digests.archive_manifests.is_empty() {
        return Err(Era1CommittedReportError::Evidence(
            "baseline archive digest set is empty",
        ));
    }
    let worlds = trials
        .iter()
        .map(|trial| trial.world_digest)
        .collect::<Vec<_>>();
    Ok(Era1EvidenceDigests {
        genome_receipts: digest_json(evolution)?,
        foundation_weights: format_blake3(foundation.digest()),
        wgsl_bundle: format_blake3(closed_loop_shader_bundle_digest()),
        world_receipts: digest_json(&worlds)?,
        portable_save,
        archive_receipts: digest_json(&(
            &baseline.evidence_digests.archive_manifests,
            &baseline.evidence_digests.archive_composite_assets,
        ))?,
        trial_receipts: digest_json(trials)?,
        causal_trial_evidence: causal_trial_evidence_digest.to_string(),
    })
}

fn observed_rtx_vram_bytes() -> Result<u64, Era1CommittedReportError> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.used",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    if !output.status.success() {
        return Err(Era1CommittedReportError::Evidence(
            "RTX VRAM observation is unavailable",
        ));
    }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let Some((name, used_mib)) = line.split_once(',') else {
            continue;
        };
        if name.trim() == REQUIRED_GPU_ADAPTER {
            let used_mib = used_mib
                .trim()
                .parse::<u64>()
                .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
            return used_mib
                .checked_mul(1024 * 1024)
                .ok_or(Era1CommittedReportError::Evidence(
                    "VRAM byte count overflow",
                ));
        }
    }
    Err(Era1CommittedReportError::Evidence(
        "required RTX adapter is not visible to nvidia-smi",
    ))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_tools lives under <workspace>/crates")
        .to_path_buf()
}

fn git_output(root: &Path, args: &[&str]) -> Result<String, Era1CommittedReportError> {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
    if !output.status.success() {
        return Err(Era1CommittedReportError::Generation(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn require_clean_source_contract(
    root: &Path,
    revision: &str,
) -> Result<(), Era1CommittedReportError> {
    let mut diff = Command::new("git");
    diff.current_dir(root)
        .args(["diff", "--quiet", revision, "--"])
        .args(SOURCE_CONTRACT_PATHS);
    if !diff
        .status()
        .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?
        .success()
    {
        return Err(Era1CommittedReportError::Evidence(
            "relevant source differs from the producing commit",
        ));
    }
    Ok(())
}

fn source_contract_digest_at_revision(
    root: &Path,
    revision: &str,
    paths: &[&str],
) -> Result<String, Era1CommittedReportError> {
    let mut hasher = blake3::Hasher::new();
    for path in paths {
        let object = format!("{revision}:{path}");
        let output = Command::new("git")
            .current_dir(root)
            .args(["show", "--no-ext-diff", "--no-textconv", &object])
            .output()
            .map_err(|error| Era1CommittedReportError::Generation(error.to_string()))?;
        if !output.status.success() {
            return Err(Era1CommittedReportError::Generation(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        hasher.update(&(path.len() as u64).to_le_bytes());
        hasher.update(path.as_bytes());
        hasher.update(&(output.stdout.len() as u64).to_le_bytes());
        hasher.update(&output.stdout);
    }
    Ok(format!("blake3-256:{}", hasher.finalize().to_hex()))
}

fn digest_json(value: &(impl Serialize + ?Sized)) -> Result<String, Era1CommittedReportError> {
    Ok(format!(
        "blake3-256:{}",
        blake3::hash(&serde_json::to_vec(value)?).to_hex()
    ))
}

fn digest_file(path: &Path) -> Result<String, Era1CommittedReportError> {
    let mut file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("blake3-256:{}", hasher.finalize().to_hex()))
}

fn format_blake3(digest: alife_core::Blake3Digest) -> String {
    let mut hex = String::with_capacity(64);
    for byte in digest.bytes() {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    format!("blake3-256:{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::era1_evolution::{Era1AcquiredStateEvidence, Era1BirthReceipt};

    #[test]
    fn trial_cache_miss_then_digest_identical_hit_reuses_validated_evidence() {
        let foundation = FoundationGeneticIdentity::new(
            0x4E32_3034_385F_5631,
            1,
            0x4E32_3034_385F_FA11,
            BrainCapacityClass::N2048_ID,
        )
        .unwrap();
        let genome = CreatureGenome::early_mammal_founder(0xE1CA_C4E0, foundation).unwrap();
        let inherited_starter_tokens = genome.express().unwrap().predisposition.starter_tokens;
        let birth = Era1BirthReceipt {
            generation: 0,
            lineage_slot: 0,
            organism_id: OrganismId(20_001),
            genome,
            inherited_starter_tokens,
            acquired_state: Era1AcquiredStateEvidence::default(),
        };
        let manifest = Era1TrialManifest::new(
            0xE1CA_5001,
            Era1WorldFamily::ForagingHazardMaze,
            birth.organism_id,
            OrganismId(30_001),
            OrganismId(30_002),
            0xE1CA_6001,
            true,
            birth.inherited_starter_tokens[0].raw(),
        )
        .unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mut runner = Era1TrialRunner::new_required().unwrap();
        let first = load_or_run_trial_evidence(
            cache.path(),
            &mut runner,
            &birth,
            &manifest,
            Era1Ability::FlexibleForaging,
            Era1Control::Intact,
            Era1EvidencePartition::HeldOutTransfer,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .unwrap();
        let cached_files = fs::read_dir(cache.path()).unwrap().count();
        let second = load_or_run_trial_evidence(
            cache.path(),
            &mut runner,
            &birth,
            &manifest,
            Era1Ability::FlexibleForaging,
            Era1Control::Intact,
            Era1EvidencePartition::HeldOutTransfer,
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(cached_files, 1);
        assert_eq!(fs::read_dir(cache.path()).unwrap().count(), 1);
    }
}
