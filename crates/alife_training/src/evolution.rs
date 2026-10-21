//! Genome mutation and multi-objective selection helpers.

use alife_core::{
    BrainCapacityClass, BrainGenome, GenomeId, GenomeSeedSet, NormalizedScalar,
    ScaffoldContractError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HardeningMutationKind {
    Baseline,
    SparseGeneticDelta,
    RouteDensity,
    PlasticityReceptor,
    AlphaPlasticity,
    BiasLeakResearch,
    BiochemicalSensitivity,
    DevelopmentalGate,
}

impl HardeningMutationKind {
    pub const ALL_MUTATIONS: [Self; 7] = [
        Self::SparseGeneticDelta,
        Self::RouteDensity,
        Self::PlasticityReceptor,
        Self::AlphaPlasticity,
        Self::BiasLeakResearch,
        Self::BiochemicalSensitivity,
        Self::DevelopmentalGate,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HardeningFitness {
    pub survival: f32,
    pub learning: f32,
    pub language_acquisition: f32,
    pub narration_fidelity: f32,
    pub mutation_robustness: f32,
    pub compute_efficiency: f32,
}

impl HardeningFitness {
    pub fn validate(self) -> Result<Self, ScaffoldContractError> {
        if [
            self.survival,
            self.learning,
            self.language_acquisition,
            self.narration_fidelity,
            self.mutation_robustness,
            self.compute_efficiency,
        ]
        .into_iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
        {
            Ok(self)
        } else {
            Err(ScaffoldContractError::ScalarOutOfRange)
        }
    }

    pub fn dominates(self, other: Self) -> bool {
        let left = self.objectives();
        let right = other.objectives();
        left.iter().zip(right).all(|(a, b)| *a >= b) && left.iter().zip(right).any(|(a, b)| *a > b)
    }

    fn objectives(self) -> [f32; 6] {
        [
            self.survival,
            self.learning,
            self.language_acquisition,
            self.narration_fidelity,
            self.mutation_robustness,
            self.compute_efficiency,
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardeningEvaluation {
    pub genome: BrainGenome,
    pub mutation: HardeningMutationKind,
    pub viable: bool,
    pub nonviable_reason: Option<String>,
    pub fitness: Option<HardeningFitness>,
}

pub fn pareto_front(evaluations: &[HardeningEvaluation]) -> Vec<usize> {
    evaluations
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.viable && candidate.fitness.is_some())
        .filter(|(index, candidate)| {
            let fitness = candidate.fitness.expect("filtered viable fitness");
            !evaluations.iter().enumerate().any(|(other_index, other)| {
                other_index != *index
                    && other.viable
                    && other
                        .fitness
                        .is_some_and(|other_fitness| other_fitness.dominates(fitness))
            })
        })
        .map(|(index, _)| index)
        .collect()
}

pub fn mutate_hardening_genome(
    parent: &BrainGenome,
    mutation: HardeningMutationKind,
    mutation_seed: u64,
) -> Result<BrainGenome, ScaffoldContractError> {
    if parent.brain_class_id != BrainCapacityClass::N2048_ID || mutation_seed == 0 {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    if mutation == HardeningMutationKind::Baseline {
        return Ok(parent.clone());
    }
    let species_seed = splitmix64(
        parent
            .species_seed
            .wrapping_add(splitmix64(mutation_seed))
            .wrapping_add(mutation_tag(mutation)),
    );
    let seeds = GenomeSeedSet::from_species_seed(species_seed, parent.brain_class_id);
    let mut child = parent.clone();
    child.species_seed = species_seed;
    child.id = GenomeId(seeds.genome_id_seed);
    child.genetic_prior_seed = seeds.genetic_prior_seed;
    child.seeds = seeds;
    child.parent_genome_ids = vec![parent.id];
    child.lineage_id = parent.lineage_id;
    match mutation {
        HardeningMutationKind::Baseline | HardeningMutationKind::SparseGeneticDelta => {}
        HardeningMutationKind::RouteDensity => {
            let row = child
                .sparse_density_priors
                .first_mut()
                .ok_or(ScaffoldContractError::PhenotypeCompile)?;
            row.density = NormalizedScalar::new((row.density.raw() * 1.05).min(1.0))?;
        }
        HardeningMutationKind::PlasticityReceptor => {
            let source = *child.plasticity_parameters();
            let mut receptor_weights = *source.receptor_profile().weights();
            receptor_weights[6] = (receptor_weights[6] + 0.05).min(2.0);
            let mut parameters = alife_core::PlasticityGenomeParameters::try_new(
                source.eligibility_decay(),
                (source.base_learning_rate() * 1.1).min(1.0),
                source.normalization_rate(),
                source.sleep_replay_rate(),
                alife_core::PlasticityReceptorProfile::try_new(receptor_weights)?,
                source.fast_bounds().0,
                source.fast_bounds().1,
                source.sleep_staging_rate(),
                source.sleep_weight_limit(),
                source.sleep_fast_decay_rate(),
            )?;
            if let Some(profile) = source.action_candidate_credit_profile() {
                parameters = parameters.with_action_candidate_credit_profile(profile)?;
            }
            child = child.with_plasticity_parameters(parameters)?;
        }
        HardeningMutationKind::AlphaPlasticity => {
            child.alpha_mask.default_alpha =
                NormalizedScalar::new((child.alpha_mask.default_alpha.raw() + 0.025).min(1.0))?;
        }
        HardeningMutationKind::BiasLeakResearch => {}
        HardeningMutationKind::BiochemicalSensitivity => {
            // The brain genome may evolve neural receptor expression, but it
            // does not own endocrine production or biochemical state.
            let source = *child.plasticity_parameters();
            let mut receptor_weights = *source.receptor_profile().weights();
            receptor_weights[7] = (receptor_weights[7] - 0.05).max(-2.0);
            let mut parameters = alife_core::PlasticityGenomeParameters::try_new(
                source.eligibility_decay(),
                source.base_learning_rate(),
                source.normalization_rate(),
                source.sleep_replay_rate(),
                alife_core::PlasticityReceptorProfile::try_new(receptor_weights)?,
                source.fast_bounds().0,
                source.fast_bounds().1,
                source.sleep_staging_rate(),
                source.sleep_weight_limit(),
                source.sleep_fast_decay_rate(),
            )?;
            if let Some(profile) = source.action_candidate_credit_profile() {
                parameters = parameters.with_action_candidate_credit_profile(profile)?;
            }
            child = child.with_plasticity_parameters(parameters)?;
        }
        HardeningMutationKind::DevelopmentalGate => {
            child.developmental_schedule.sleep_pressure_maturation_gate =
                NormalizedScalar::new(0.30)?;
        }
    }
    alife_core::Validate::validate_contract(&child)?;
    Ok(child)
}

const fn mutation_tag(mutation: HardeningMutationKind) -> u64 {
    mutation as u64 + 0x4841_5244_454E_0000
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let value = value ^ (value >> 31);
    if value == 0 {
        1
    } else {
        value
    }
}
