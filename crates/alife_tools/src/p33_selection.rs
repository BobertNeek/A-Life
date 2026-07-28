//! Deterministic offline selection over authoritative composite creature genomes.
//!
//! This module chooses managed pairings and records breeding evidence. It never
//! executes brains or changes runtime reproduction authority.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashSet};

use alife_core::{
    BrainCapacityClass, CreatureGenome, GeneticLineageProvenance, GenomeId, LineageId,
    PhenotypeCompiler, PhenotypeHash, SensorProfile, Tick, Validate,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::p33_evaluation::{ObjectiveVector, ScoreEstimate};

pub const MANAGED_SELECTION_SCHEMA_VERSION: u16 = 1;
const OBJECTIVE_COUNT: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PopulationLane {
    Wild,
    Managed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecialistRole {
    EcologicalSurvivor,
    Teacher,
    Coordinator,
    TransferSpecialist,
    StabilitySpecialist,
    EfficientSolver,
}

impl SpecialistRole {
    const fn objective_index(self) -> usize {
        match self {
            Self::EcologicalSurvivor => 0,
            Self::Teacher | Self::TransferSpecialist => 1,
            Self::Coordinator => 3,
            Self::StabilitySpecialist => 4,
            Self::EfficientSolver => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionCandidate {
    pub genome: CreatureGenome,
    pub objectives: ObjectiveVector,
    /// Evaluation-side ancestry evidence beyond the core genome's direct parents.
    pub known_ancestor_genome_ids: Vec<GenomeId>,
    pub population_share: f32,
    pub lane: PopulationLane,
    pub specialist_roles: Vec<SpecialistRole>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ManagedSelectionConfig {
    pub selection_seed: u64,
    pub max_pairings: usize,
    pub minority_lineage_share_max: f32,
    pub fragile_ecology_max: f32,
    pub high_cognition_min: f32,
    pub robust_ecology_min: f32,
    pub introgression_sibling_count: u8,
}

impl ManagedSelectionConfig {
    fn validate(self) -> Result<(), SelectionError> {
        let bounded = [
            self.minority_lineage_share_max,
            self.fragile_ecology_max,
            self.high_cognition_min,
            self.robust_ecology_min,
        ];
        if self.selection_seed == 0 {
            return Err(SelectionError::InvalidConfig {
                field: "selection_seed",
            });
        }
        if self.max_pairings == 0 {
            return Err(SelectionError::InvalidConfig {
                field: "max_pairings",
            });
        }
        if bounded
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(SelectionError::InvalidConfig {
                field: "selection thresholds",
            });
        }
        if self.robust_ecology_min <= self.fragile_ecology_max {
            return Err(SelectionError::InvalidConfig {
                field: "robust_ecology_min",
            });
        }
        if self.introgression_sibling_count < 2 {
            return Err(SelectionError::InvalidConfig {
                field: "introgression_sibling_count",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairingLane {
    Standard,
    CognitiveIntrogression,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedPairing {
    pub maternal_genome_id: GenomeId,
    pub paternal_genome_id: GenomeId,
    pub lane: PairingLane,
    pub offspring_genome_ids: Vec<GenomeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbationCheck {
    Cognition,
    Ecology,
    Transfer,
    StabilityHealth,
    Development,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbationCohort {
    pub scrutiny_multiplier: u8,
    pub required_checks: Vec<ProbationCheck>,
    pub sibling_controls: Vec<GenomeId>,
    pub population_controls: Vec<GenomeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledPhenotypeViability {
    pub phenotype_hash: PhenotypeHash,
    pub neuron_count: u32,
    pub synapse_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViableOffspring {
    pub genome: CreatureGenome,
    pub genetic_provenance: GeneticLineageProvenance,
    pub viability: CompiledPhenotypeViability,
    pub probation: Option<ProbationCohort>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateRejectionReason {
    MissingObjectiveEvidence,
    InvalidPopulationShare,
    InvalidGenome,
    NonViablePhenotype,
    NoEligibleBreedingLane,
    NoLegalMate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRejection {
    pub genome_id: GenomeId,
    pub reason: CandidateRejectionReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpecialistRetention {
    pub role: SpecialistRole,
    pub genome_id: GenomeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManagedBreedingPlan {
    pub schema_version: u16,
    pub selection_seed: u64,
    pub preserved_wild_genomes: Vec<CreatureGenome>,
    pub pareto_frontier: Vec<GenomeId>,
    pub retained_parent_genomes: Vec<GenomeId>,
    pub retained_minority_lineages: Vec<LineageId>,
    pub retained_specialists: Vec<SpecialistRetention>,
    pub pairings: Vec<ManagedPairing>,
    pub offspring: Vec<ViableOffspring>,
    pub rejected_candidates: Vec<CandidateRejection>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SelectionError {
    #[error("invalid managed-selection configuration field: {field}")]
    InvalidConfig { field: &'static str },
    #[error(
        "authoritative reproduction failed for parents {maternal:?}/{paternal:?} at seed {conception_seed}: {reason}"
    )]
    ReproductionFailed {
        maternal: GenomeId,
        paternal: GenomeId,
        conception_seed: u64,
        reason: String,
    },
    #[error("offspring {genome_id:?} failed authoritative phenotype viability: {reason}")]
    OffspringViabilityFailed { genome_id: GenomeId, reason: String },
}

struct PreparedCandidate<'a> {
    source: &'a SelectionCandidate,
    values: [f32; OBJECTIVE_COUNT],
}

pub fn run_managed_selection(
    candidates: &[SelectionCandidate],
    config: &ManagedSelectionConfig,
) -> Result<ManagedBreedingPlan, SelectionError> {
    config.validate()?;

    let mut preserved_wild_genomes = candidates
        .iter()
        .filter(|candidate| candidate.lane == PopulationLane::Wild)
        .map(|candidate| candidate.genome.clone())
        .collect::<Vec<_>>();
    preserved_wild_genomes.sort_by_key(|genome| genome.id.0);

    let mut prepared = Vec::new();
    let mut rejected_candidates = Vec::new();
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate.lane == PopulationLane::Managed)
    {
        let Some(values) = objective_values(&candidate.objectives) else {
            rejected_candidates.push(CandidateRejection {
                genome_id: candidate.genome.id,
                reason: CandidateRejectionReason::MissingObjectiveEvidence,
            });
            continue;
        };
        if !candidate.population_share.is_finite()
            || !(0.0..=1.0).contains(&candidate.population_share)
        {
            rejected_candidates.push(CandidateRejection {
                genome_id: candidate.genome.id,
                reason: CandidateRejectionReason::InvalidPopulationShare,
            });
            continue;
        }
        if candidate.genome.validate_contract().is_err()
            || candidate.genome.provenance.validate_contract().is_err()
        {
            rejected_candidates.push(CandidateRejection {
                genome_id: candidate.genome.id,
                reason: CandidateRejectionReason::InvalidGenome,
            });
            continue;
        }
        if compile_viability(&candidate.genome).is_err() {
            rejected_candidates.push(CandidateRejection {
                genome_id: candidate.genome.id,
                reason: CandidateRejectionReason::NonViablePhenotype,
            });
            continue;
        }
        prepared.push(PreparedCandidate {
            source: candidate,
            values,
        });
    }

    let pareto_indices = pareto_frontier(&prepared);
    let mut retained_indices = pareto_indices.iter().copied().collect::<BTreeSet<_>>();

    let mut minority_by_lineage = BTreeMap::<u64, Vec<usize>>::new();
    for (index, candidate) in prepared.iter().enumerate() {
        if candidate.source.population_share <= config.minority_lineage_share_max {
            minority_by_lineage
                .entry(candidate.source.genome.lineage_id.0)
                .or_default()
                .push(index);
        }
    }
    let mut retained_minority_lineages = Vec::new();
    for (lineage, indices) in minority_by_lineage {
        if let Some(index) =
            deterministic_lexicase_order(&prepared, &indices, config.selection_seed)
                .first()
                .copied()
        {
            retained_indices.insert(index);
            retained_minority_lineages.push(LineageId(lineage));
        }
    }

    let mut role_holders = BTreeMap::<SpecialistRole, Vec<usize>>::new();
    for (index, candidate) in prepared.iter().enumerate() {
        for role in &candidate.source.specialist_roles {
            role_holders.entry(*role).or_default().push(index);
        }
    }
    let mut retained_specialists = Vec::new();
    for (role, holders) in role_holders {
        let objective = role.objective_index();
        let selected = holders
            .into_iter()
            .max_by(|left, right| {
                compare_candidate_axis(&prepared[*left], &prepared[*right], objective)
            })
            .expect("role holder list is nonempty");
        retained_indices.insert(selected);
        retained_specialists.push(SpecialistRetention {
            role,
            genome_id: prepared[selected].source.genome.id,
        });
    }

    if retained_indices.len() < 2 {
        for index in deterministic_lexicase_order(
            &prepared,
            &(0..prepared.len()).collect::<Vec<_>>(),
            config.selection_seed,
        ) {
            retained_indices.insert(index);
            if retained_indices.len() >= 2 {
                break;
            }
        }
    }

    let retained_input = retained_indices.iter().copied().collect::<Vec<_>>();
    let retained_order =
        deterministic_lexicase_order(&prepared, &retained_input, config.selection_seed);
    let all_order = deterministic_lexicase_order(
        &prepared,
        &(0..prepared.len()).collect::<Vec<_>>(),
        config.selection_seed,
    );
    let mut retained_parent_genomes = retained_order
        .iter()
        .map(|index| prepared[*index].source.genome.id)
        .collect::<Vec<_>>();

    let mut used = HashSet::<u64>::new();
    let mut pairings = Vec::new();
    let mut offspring = Vec::new();
    for maternal_index in retained_order {
        if pairings.len() >= config.max_pairings {
            break;
        }
        let maternal = &prepared[maternal_index];
        if used.contains(&maternal.source.genome.id.0) {
            continue;
        }
        if is_fragile(maternal, config) && !is_cognitive_exception(maternal, config) {
            rejected_candidates.push(CandidateRejection {
                genome_id: maternal.source.genome.id,
                reason: CandidateRejectionReason::NoEligibleBreedingLane,
            });
            continue;
        }

        let paternal_index = all_order.iter().copied().find(|index| {
            *index != maternal_index
                && !used.contains(&prepared[*index].source.genome.id.0)
                && legal_pair(maternal, &prepared[*index], config)
        });
        let Some(paternal_index) = paternal_index else {
            rejected_candidates.push(CandidateRejection {
                genome_id: maternal.source.genome.id,
                reason: CandidateRejectionReason::NoLegalMate,
            });
            continue;
        };
        let paternal = &prepared[paternal_index];
        let lane = if is_fragile(maternal, config) || is_fragile(paternal, config) {
            PairingLane::CognitiveIntrogression
        } else {
            PairingLane::Standard
        };

        let mut population_controls = prepared
            .iter()
            .filter(|candidate| {
                candidate.source.genome.id != maternal.source.genome.id
                    && candidate.source.genome.id != paternal.source.genome.id
                    && !is_fragile(candidate, config)
            })
            .map(|candidate| candidate.source.genome.id)
            .collect::<Vec<_>>();
        population_controls.sort_by_key(|id| id.0);
        population_controls.dedup();
        if lane == PairingLane::CognitiveIntrogression && population_controls.is_empty() {
            rejected_candidates.push(CandidateRejection {
                genome_id: maternal.source.genome.id,
                reason: CandidateRejectionReason::NoLegalMate,
            });
            continue;
        }

        let sibling_count = match lane {
            PairingLane::Standard => 1,
            PairingLane::CognitiveIntrogression => config.introgression_sibling_count,
        };
        let mut pairing_offspring = Vec::new();
        for sibling_index in 0..sibling_count {
            let conception_seed = derive_conception_seed(
                config.selection_seed,
                pairings.len(),
                usize::from(sibling_index),
                maternal.source.genome.id,
                paternal.source.genome.id,
            );
            let child = CreatureGenome::reproduce(
                &maternal.source.genome,
                &paternal.source.genome,
                conception_seed,
            )
            .map_err(|error| SelectionError::ReproductionFailed {
                maternal: maternal.source.genome.id,
                paternal: paternal.source.genome.id,
                conception_seed,
                reason: error.to_string(),
            })?;
            child.validate_contract().map_err(|error| {
                SelectionError::OffspringViabilityFailed {
                    genome_id: child.id,
                    reason: error.to_string(),
                }
            })?;
            child.provenance.validate_contract().map_err(|error| {
                SelectionError::OffspringViabilityFailed {
                    genome_id: child.id,
                    reason: error.to_string(),
                }
            })?;
            let viability = compile_viability(&child).map_err(|reason| {
                SelectionError::OffspringViabilityFailed {
                    genome_id: child.id,
                    reason,
                }
            })?;
            pairing_offspring.push(ViableOffspring {
                genetic_provenance: child.provenance.clone(),
                genome: child,
                viability,
                probation: None,
            });
        }

        let offspring_ids = pairing_offspring
            .iter()
            .map(|child| child.genome.id)
            .collect::<Vec<_>>();
        if lane == PairingLane::CognitiveIntrogression {
            for child in &mut pairing_offspring {
                child.probation = Some(ProbationCohort {
                    scrutiny_multiplier: 2,
                    required_checks: vec![
                        ProbationCheck::Cognition,
                        ProbationCheck::Ecology,
                        ProbationCheck::Transfer,
                        ProbationCheck::StabilityHealth,
                        ProbationCheck::Development,
                    ],
                    sibling_controls: offspring_ids
                        .iter()
                        .copied()
                        .filter(|id| *id != child.genome.id)
                        .collect(),
                    population_controls: population_controls.clone(),
                });
            }
        }
        pairings.push(ManagedPairing {
            maternal_genome_id: maternal.source.genome.id,
            paternal_genome_id: paternal.source.genome.id,
            lane,
            offspring_genome_ids: offspring_ids,
        });
        offspring.extend(pairing_offspring);
        for parent in [maternal.source.genome.id, paternal.source.genome.id] {
            if !retained_parent_genomes.contains(&parent) {
                retained_parent_genomes.push(parent);
            }
        }
        used.insert(maternal.source.genome.id.0);
        used.insert(paternal.source.genome.id.0);
    }

    let mut pareto_frontier = pareto_indices
        .iter()
        .map(|index| prepared[*index].source.genome.id)
        .collect::<Vec<_>>();
    pareto_frontier.sort_by_key(|id| id.0);
    retained_minority_lineages.sort_by_key(|lineage| lineage.0);
    rejected_candidates.sort_by_key(|rejection| {
        (
            rejection.genome_id.0,
            rejection_reason_order(rejection.reason),
        )
    });
    rejected_candidates.dedup();

    Ok(ManagedBreedingPlan {
        schema_version: MANAGED_SELECTION_SCHEMA_VERSION,
        selection_seed: config.selection_seed,
        preserved_wild_genomes,
        pareto_frontier,
        retained_parent_genomes,
        retained_minority_lineages,
        retained_specialists,
        pairings,
        offspring,
        rejected_candidates,
    })
}

fn objective_values(objectives: &ObjectiveVector) -> Option<[f32; OBJECTIVE_COUNT]> {
    let estimates = [
        objectives.ecological,
        objectives.cognitive,
        objectives.social,
        objectives.group,
        objectives.stability,
        objectives.efficiency,
        objectives.diversity,
    ];
    let mut values = [0.0; OBJECTIVE_COUNT];
    for (index, estimate) in estimates.into_iter().enumerate() {
        values[index] = known_objective(estimate)?;
    }
    Some(values)
}

fn known_objective(estimate: ScoreEstimate) -> Option<f32> {
    let value = estimate.value?;
    (estimate.samples > 0 && value.is_finite() && (0.0..=1.0).contains(&value)).then_some(value)
}

fn compile_viability(genome: &CreatureGenome) -> Result<CompiledPhenotypeViability, String> {
    let expressed = genome.express().map_err(|error| error.to_string())?;
    let capacity = BrainCapacityClass::production_for_id(expressed.foundation.brain_class_id)
        .map_err(|error| error.to_string())?;
    let mature_age = Tick(u64::from(expressed.development.maturation_duration_ticks));
    let development = expressed
        .development_state_at(mature_age)
        .map_err(|error| error.to_string())?;
    let compiled = PhenotypeCompiler::compile(
        &expressed.brain_genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .map_err(|error| error.to_string())?;
    Ok(CompiledPhenotypeViability {
        phenotype_hash: compiled.phenotype_hash(),
        neuron_count: compiled.neuron_count(),
        synapse_count: compiled.synapses().len(),
    })
}

fn pareto_frontier(candidates: &[PreparedCandidate<'_>]) -> Vec<usize> {
    (0..candidates.len())
        .filter(|candidate| {
            !(0..candidates.len()).any(|challenger| {
                challenger != *candidate
                    && dominates(
                        &candidates[challenger].values,
                        &candidates[*candidate].values,
                    )
            })
        })
        .collect()
}

fn dominates(left: &[f32; OBJECTIVE_COUNT], right: &[f32; OBJECTIVE_COUNT]) -> bool {
    left.iter().zip(right).all(|(a, b)| a >= b) && left.iter().zip(right).any(|(a, b)| a > b)
}

fn deterministic_lexicase_order(
    candidates: &[PreparedCandidate<'_>],
    indices: &[usize],
    seed: u64,
) -> Vec<usize> {
    let mut remaining = indices.to_vec();
    let mut selected = Vec::with_capacity(remaining.len());
    let mut round = 0usize;
    while !remaining.is_empty() {
        let start = (seed as usize).wrapping_add(round) % OBJECTIVE_COUNT;
        let mut pool = remaining.clone();
        for offset in 0..OBJECTIVE_COUNT {
            let axis = (start + offset) % OBJECTIVE_COUNT;
            let best = pool
                .iter()
                .map(|index| candidates[*index].values[axis])
                .fold(f32::NEG_INFINITY, f32::max);
            pool.retain(|index| candidates[*index].values[axis] == best);
            if pool.len() == 1 {
                break;
            }
        }
        pool.sort_by_key(|index| candidates[*index].source.genome.id.0);
        let winner = pool[0];
        selected.push(winner);
        remaining.retain(|index| *index != winner);
        round = round.wrapping_add(1);
    }
    selected
}

fn compare_candidate_axis(
    left: &PreparedCandidate<'_>,
    right: &PreparedCandidate<'_>,
    axis: usize,
) -> Ordering {
    left.values[axis]
        .total_cmp(&right.values[axis])
        .then_with(|| right.source.genome.id.0.cmp(&left.source.genome.id.0))
}

fn is_fragile(candidate: &PreparedCandidate<'_>, config: &ManagedSelectionConfig) -> bool {
    candidate.values[0] <= config.fragile_ecology_max
}

fn is_cognitive_exception(
    candidate: &PreparedCandidate<'_>,
    config: &ManagedSelectionConfig,
) -> bool {
    is_fragile(candidate, config) && candidate.values[1] >= config.high_cognition_min
}

fn is_robust(candidate: &PreparedCandidate<'_>, config: &ManagedSelectionConfig) -> bool {
    candidate.values[0] >= config.robust_ecology_min
}

fn legal_pair(
    left: &PreparedCandidate<'_>,
    right: &PreparedCandidate<'_>,
    config: &ManagedSelectionConfig,
) -> bool {
    if !foundation_compatible(left.source, right.source) || !unrelated(left.source, right.source) {
        return false;
    }
    match (is_fragile(left, config), is_fragile(right, config)) {
        (true, true) => false,
        (true, false) => is_cognitive_exception(left, config) && is_robust(right, config),
        (false, true) => is_robust(left, config) && is_cognitive_exception(right, config),
        (false, false) => true,
    }
}

fn foundation_compatible(left: &SelectionCandidate, right: &SelectionCandidate) -> bool {
    left.genome.foundation.compatibility_family_id
        == right.genome.foundation.compatibility_family_id
        && left.genome.foundation.brain_class_id == right.genome.foundation.brain_class_id
}

fn unrelated(left: &SelectionCandidate, right: &SelectionCandidate) -> bool {
    if left.genome.id == right.genome.id || left.genome.lineage_id == right.genome.lineage_id {
        return false;
    }
    let left_ancestors = ancestry_ids(left);
    let right_ancestors = ancestry_ids(right);
    !left_ancestors.contains(&right.genome.id.0)
        && !right_ancestors.contains(&left.genome.id.0)
        && left_ancestors.is_disjoint(&right_ancestors)
}

fn ancestry_ids(candidate: &SelectionCandidate) -> HashSet<u64> {
    candidate
        .genome
        .parent_genome_ids
        .iter()
        .chain(&candidate.known_ancestor_genome_ids)
        .map(|id| id.0)
        .collect()
}

fn derive_conception_seed(
    selection_seed: u64,
    pairing_index: usize,
    sibling_index: usize,
    maternal: GenomeId,
    paternal: GenomeId,
) -> u64 {
    let mut value = selection_seed
        ^ maternal.0.rotate_left(13)
        ^ paternal.0.rotate_right(7)
        ^ (pairing_index as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
        ^ (sibling_index as u64).wrapping_mul(0xA076_1D64_78BD_642F);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let mixed = value ^ (value >> 31);
    if mixed == 0 {
        1
    } else {
        mixed
    }
}

const fn rejection_reason_order(reason: CandidateRejectionReason) -> u8 {
    match reason {
        CandidateRejectionReason::MissingObjectiveEvidence => 0,
        CandidateRejectionReason::InvalidPopulationShare => 1,
        CandidateRejectionReason::InvalidGenome => 2,
        CandidateRejectionReason::NonViablePhenotype => 3,
        CandidateRejectionReason::NoEligibleBreedingLane => 4,
        CandidateRejectionReason::NoLegalMate => 5,
    }
}
