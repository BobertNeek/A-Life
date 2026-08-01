//! Deterministic promotion and plateau derivation from sealed Era 1 receipts.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{
    BrainCapacityClass, Era1Ability, Era1Control, Era1EvidencePartition, Era1PlateauWindow,
    Era1TrialReceipt, PhenotypeHash, Validate,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::era1_evolution::Era1EvolutionReceipt;

pub const ERA1_PROMOTION_SCHEMA_VERSION: u16 = 1;
pub const ERA1_MINIMUM_MARGIN_Q16: i32 = 3_277;
pub const ERA1_MINIMUM_MATCHED_CELLS: u16 = 12;
pub const ERA1_PLATEAU_MAX_IMPROVEMENT_Q16: i32 = 655;

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
