//! Contract-only EI0 diploid genome, chromosome, and allele-expression records.

use serde::{Deserialize, Serialize};

use crate::{
    validate_finite, BrainCapacityClass, BrainClassId, GenomeId, LineageId, ScaffoldContractError,
    SchemaKind, Validate,
};

pub const CREATURE_GENOME_SCHEMA_VERSION: u16 = 1;
pub const MAX_CROSSOVER_SEGMENTS: u8 = 8;
pub const MAX_MUTATION_DELTA: f32 = 0.25;
pub const MAX_MUTATION_RECORDS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ContinuousLocus {
    pub maternal: f32,
    pub paternal: f32,
    pub lower: f32,
    pub upper: f32,
    pub maternal_weight: f32,
}

impl ContinuousLocus {
    pub fn mean(maternal: f32, paternal: f32) -> Result<Self, ScaffoldContractError> {
        Self::with_bounds(maternal, paternal, 0.0, 1.0, 0.5)
    }

    pub fn with_bounds(
        maternal: f32,
        paternal: f32,
        lower: f32,
        upper: f32,
        maternal_weight: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let locus = Self {
            maternal,
            paternal,
            lower,
            upper,
            maternal_weight,
        };
        locus.validate_contract()?;
        Ok(locus)
    }

    pub fn expressed(self) -> Result<f32, ScaffoldContractError> {
        self.validate_contract()?;
        let value = self.maternal.mul_add(
            self.maternal_weight,
            self.paternal * (1.0 - self.maternal_weight),
        );
        validate_finite(value)?;
        if (self.lower..=self.upper).contains(&value) {
            Ok(value)
        } else {
            Err(ScaffoldContractError::InvalidGeneticBounds)
        }
    }
}

impl Validate for ContinuousLocus {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        for value in [
            self.maternal,
            self.paternal,
            self.lower,
            self.upper,
            self.maternal_weight,
        ] {
            validate_finite(value)?;
        }
        if self.lower >= self.upper
            || !(self.lower..=self.upper).contains(&self.maternal)
            || !(self.lower..=self.upper).contains(&self.paternal)
            || !(0.0..1.0).contains(&self.maternal_weight)
        {
            return Err(ScaffoldContractError::InvalidGeneticBounds);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlleleDominance {
    Recessive,
    Dominant,
    Codominant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscreteAllele<T> {
    pub value: T,
    pub dominance: AlleleDominance,
}

impl<T> DiscreteAllele<T> {
    pub const fn new(value: T, dominance: AlleleDominance) -> Self {
        Self { value, dominance }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscreteLocus<T> {
    pub maternal: DiscreteAllele<T>,
    pub paternal: DiscreteAllele<T>,
}

impl<T> DiscreteLocus<T> {
    pub const fn new(maternal: DiscreteAllele<T>, paternal: DiscreteAllele<T>) -> Self {
        Self { maternal, paternal }
    }
}

impl<T: Copy + PartialEq> DiscreteLocus<T> {
    pub fn expressed(self) -> DiscreteExpression<T> {
        if self.maternal.value == self.paternal.value {
            return DiscreteExpression::Single(self.maternal.value);
        }
        if self.maternal.dominance == AlleleDominance::Codominant
            || self.paternal.dominance == AlleleDominance::Codominant
        {
            return DiscreteExpression::Codominant(self.maternal.value, self.paternal.value);
        }
        match (self.maternal.dominance, self.paternal.dominance) {
            (AlleleDominance::Dominant, AlleleDominance::Recessive) => {
                DiscreteExpression::Single(self.maternal.value)
            }
            (AlleleDominance::Recessive, AlleleDominance::Dominant) => {
                DiscreteExpression::Single(self.paternal.value)
            }
            _ => DiscreteExpression::Codominant(self.maternal.value, self.paternal.value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscreteExpression<T> {
    Single(T),
    Codominant(T, T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BodyFrame {
    Light,
    Balanced,
    Sturdy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MatePreference {
    Novelty,
    Similarity,
    Health,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StarterVocabularyProfile {
    Minimal,
    Foraging,
    Social,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChromosomeKind {
    Body,
    Brain,
    Chemistry,
    Development,
    Reproduction,
    Predisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlleleSide {
    Maternal,
    Paternal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChromosomeRecombinationRecord {
    pub chromosome: ChromosomeKind,
    pub maternal_segments: u8,
    pub paternal_segments: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MutationRecord {
    Continuous {
        chromosome: ChromosomeKind,
        locus_index: u8,
        allele: AlleleSide,
        before: f32,
        after: f32,
        lower: f32,
        upper: f32,
    },
    Discrete {
        chromosome: ChromosomeKind,
        locus_index: u8,
        allele: AlleleSide,
        before: u16,
        after: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneticLineageProvenance {
    pub conception_seed: u64,
    pub ordinary_birth: bool,
    pub recombination: Vec<ChromosomeRecombinationRecord>,
    pub mutations: Vec<MutationRecord>,
}

impl GeneticLineageProvenance {
    fn founder(seed: u64) -> Self {
        Self {
            conception_seed: seed,
            ordinary_birth: false,
            recombination: Vec::new(),
            mutations: Vec::new(),
        }
    }
}

impl Validate for GeneticLineageProvenance {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.conception_seed == 0
            || self.recombination.len() > 6
            || self.mutations.len() > MAX_MUTATION_RECORDS
        {
            return Err(ScaffoldContractError::MutationOverflow);
        }
        for record in &self.recombination {
            if record.maternal_segments == 0
                || record.paternal_segments == 0
                || record.maternal_segments > MAX_CROSSOVER_SEGMENTS
                || record.paternal_segments > MAX_CROSSOVER_SEGMENTS
            {
                return Err(ScaffoldContractError::MutationOverflow);
            }
        }
        for record in &self.mutations {
            match *record {
                MutationRecord::Continuous {
                    before,
                    after,
                    lower,
                    upper,
                    ..
                } => {
                    for value in [before, after, lower, upper] {
                        validate_finite(value)?;
                    }
                    if lower >= upper
                        || !(lower..=upper).contains(&before)
                        || !(lower..=upper).contains(&after)
                        || before == after
                    {
                        return Err(ScaffoldContractError::MutationOverflow);
                    }
                }
                MutationRecord::Discrete { before, after, .. } if before == after => {
                    return Err(ScaffoldContractError::MutationOverflow);
                }
                MutationRecord::Discrete { .. } => {}
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyChromosome {
    pub size: ContinuousLocus,
    pub metabolic_efficiency: ContinuousLocus,
    pub sensory_acuity: ContinuousLocus,
    pub movement_efficiency: ContinuousLocus,
    pub lifespan: ContinuousLocus,
    pub injury_resistance: ContinuousLocus,
    pub temperature_tolerance: ContinuousLocus,
    pub appearance_hue: ContinuousLocus,
    pub frame: DiscreteLocus<BodyFrame>,
}

impl Validate for BodyChromosome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_loci(&[
            &self.size,
            &self.metabolic_efficiency,
            &self.sensory_acuity,
            &self.movement_efficiency,
            &self.lifespan,
            &self.injury_resistance,
            &self.temperature_tolerance,
            &self.appearance_hue,
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrainChromosome {
    pub brain_class: DiscreteLocus<BrainClassId>,
    pub sensory_lobe_ratio: ContinuousLocus,
    pub association_lobe_ratio: ContinuousLocus,
    pub working_memory_ratio: ContinuousLocus,
    pub connectivity_density: ContinuousLocus,
    pub plasticity: ContinuousLocus,
    pub receptor_sensitivity: ContinuousLocus,
    pub genetic_weight_bias: ContinuousLocus,
}

impl BrainChromosome {
    pub fn expressed_brain_class(&self) -> Result<BrainClassId, ScaffoldContractError> {
        match self.brain_class.expressed() {
            DiscreteExpression::Single(class_id) => {
                BrainCapacityClass::production_for_id(class_id)?;
                Ok(class_id)
            }
            DiscreteExpression::Codominant(left, right) if left == right => {
                BrainCapacityClass::production_for_id(left)?;
                Ok(left)
            }
            DiscreteExpression::Codominant(_, _) => {
                Err(ScaffoldContractError::IncompatibleGeneticClass)
            }
        }
    }
}

impl Validate for BrainChromosome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        BrainCapacityClass::production_for_id(self.brain_class.maternal.value)?;
        BrainCapacityClass::production_for_id(self.brain_class.paternal.value)?;
        self.expressed_brain_class()?;
        validate_loci(&[
            &self.sensory_lobe_ratio,
            &self.association_lobe_ratio,
            &self.working_memory_ratio,
            &self.connectivity_density,
            &self.plasticity,
            &self.receptor_sensitivity,
            &self.genetic_weight_bias,
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChemistryChromosome {
    pub stress_baseline: ContinuousLocus,
    pub reward_sensitivity: ContinuousLocus,
    pub bonding_sensitivity: ContinuousLocus,
    pub hormone_production: ContinuousLocus,
    pub hormone_decay: ContinuousLocus,
    pub hunger_threshold: ContinuousLocus,
    pub fatigue_threshold: ContinuousLocus,
    pub sleep_threshold: ContinuousLocus,
    pub reproductive_threshold: ContinuousLocus,
    pub brain_atp_efficiency: ContinuousLocus,
}

impl Validate for ChemistryChromosome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_loci(&[
            &self.stress_baseline,
            &self.reward_sensitivity,
            &self.bonding_sensitivity,
            &self.hormone_production,
            &self.hormone_decay,
            &self.hunger_threshold,
            &self.fatigue_threshold,
            &self.sleep_threshold,
            &self.reproductive_threshold,
            &self.brain_atp_efficiency,
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentChromosome {
    pub maturation_rate: ContinuousLocus,
    pub puberty_onset: ContinuousLocus,
    pub sensor_activation: ContinuousLocus,
    pub lobe_activation: ContinuousLocus,
    pub critical_period_open: ContinuousLocus,
    pub critical_period_close: ContinuousLocus,
    pub migration_checkpoint: ContinuousLocus,
}

impl Validate for DevelopmentChromosome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_loci(&[
            &self.maturation_rate,
            &self.puberty_onset,
            &self.sensor_activation,
            &self.lobe_activation,
            &self.critical_period_open,
            &self.critical_period_close,
            &self.migration_checkpoint,
        ])?;
        if self.critical_period_open.expressed()? >= self.critical_period_close.expressed()? {
            return Err(ScaffoldContractError::InvalidGeneticBounds);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReproductionChromosome {
    pub fertility: ContinuousLocus,
    pub crossover_probability: ContinuousLocus,
    pub max_crossover_segments: ContinuousLocus,
    pub mutation_rate: ContinuousLocus,
    pub discrete_mutation_rate: ContinuousLocus,
    pub max_mutation_delta: ContinuousLocus,
    pub parental_investment: ContinuousLocus,
    pub mate_preference: DiscreteLocus<MatePreference>,
}

impl Validate for ReproductionChromosome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_loci(&[
            &self.fertility,
            &self.crossover_probability,
            &self.max_crossover_segments,
            &self.mutation_rate,
            &self.discrete_mutation_rate,
            &self.max_mutation_delta,
            &self.parental_investment,
        ])?;
        if self.max_mutation_delta.expressed()? > MAX_MUTATION_DELTA {
            return Err(ScaffoldContractError::MutationOverflow);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PredispositionChromosome {
    pub starter_vocabulary: DiscreteLocus<StarterVocabularyProfile>,
    pub reflex_strength: ContinuousLocus,
    pub food_attraction: ContinuousLocus,
    pub hazard_aversion: ContinuousLocus,
    pub social_attention: ContinuousLocus,
    pub novelty_bias: ContinuousLocus,
}

impl Validate for PredispositionChromosome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_loci(&[
            &self.reflex_strength,
            &self.food_attraction,
            &self.hazard_aversion,
            &self.social_attention,
            &self.novelty_bias,
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundationGeneticIdentity {
    pub foundation_id: u64,
    pub version: u16,
    pub compatibility_family_id: u64,
    pub brain_class_id: BrainClassId,
}

impl FoundationGeneticIdentity {
    pub fn new(
        foundation_id: u64,
        version: u16,
        compatibility_family_id: u64,
        brain_class_id: BrainClassId,
    ) -> Result<Self, ScaffoldContractError> {
        let identity = Self {
            foundation_id,
            version,
            compatibility_family_id,
            brain_class_id,
        };
        identity.validate_contract()?;
        Ok(identity)
    }
}

impl Validate for FoundationGeneticIdentity {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.foundation_id == 0 || self.version == 0 || self.compatibility_family_id == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        BrainCapacityClass::production_for_id(self.brain_class_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreatureGenome {
    pub schema_version: u16,
    pub id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: LineageId,
    pub conception_seed: u64,
    pub foundation: FoundationGeneticIdentity,
    pub provenance: GeneticLineageProvenance,
    pub body: BodyChromosome,
    pub brain: BrainChromosome,
    pub chemistry: ChemistryChromosome,
    pub development: DevelopmentChromosome,
    pub reproduction: ReproductionChromosome,
    pub predisposition: PredispositionChromosome,
}

impl CreatureGenome {
    pub fn early_mammal_founder(
        species_seed: u64,
        foundation: FoundationGeneticIdentity,
    ) -> Result<Self, ScaffoldContractError> {
        if species_seed == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        foundation.validate_contract()?;
        let class = foundation.brain_class_id;
        let recessive = AlleleDominance::Recessive;
        let dominant = AlleleDominance::Dominant;
        let codominant = AlleleDominance::Codominant;
        let genome = Self {
            schema_version: CREATURE_GENOME_SCHEMA_VERSION,
            id: GenomeId(nonzero_mix(species_seed ^ 0xE10_0001)),
            parent_genome_ids: Vec::new(),
            lineage_id: LineageId(nonzero_mix(species_seed ^ 0xE10_0002)),
            conception_seed: species_seed,
            foundation,
            provenance: GeneticLineageProvenance::founder(species_seed),
            body: BodyChromosome {
                size: ContinuousLocus::mean(0.42, 0.48)?,
                metabolic_efficiency: ContinuousLocus::mean(0.56, 0.61)?,
                sensory_acuity: ContinuousLocus::mean(0.48, 0.55)?,
                movement_efficiency: ContinuousLocus::mean(0.50, 0.57)?,
                lifespan: ContinuousLocus::mean(0.44, 0.51)?,
                injury_resistance: ContinuousLocus::mean(0.38, 0.46)?,
                temperature_tolerance: ContinuousLocus::mean(0.47, 0.54)?,
                appearance_hue: ContinuousLocus::mean(0.28, 0.36)?,
                frame: DiscreteLocus::new(
                    DiscreteAllele::new(BodyFrame::Balanced, dominant),
                    DiscreteAllele::new(BodyFrame::Sturdy, recessive),
                ),
            },
            brain: BrainChromosome {
                brain_class: DiscreteLocus::new(
                    DiscreteAllele::new(class, dominant),
                    DiscreteAllele::new(class, recessive),
                ),
                sensory_lobe_ratio: ContinuousLocus::mean(0.20, 0.23)?,
                association_lobe_ratio: ContinuousLocus::mean(0.26, 0.30)?,
                working_memory_ratio: ContinuousLocus::mean(0.10, 0.13)?,
                connectivity_density: ContinuousLocus::mean(0.45, 0.52)?,
                plasticity: ContinuousLocus::mean(0.48, 0.56)?,
                receptor_sensitivity: ContinuousLocus::mean(0.50, 0.58)?,
                genetic_weight_bias: ContinuousLocus::mean(0.47, 0.53)?,
            },
            chemistry: ChemistryChromosome {
                stress_baseline: ContinuousLocus::mean(0.18, 0.24)?,
                reward_sensitivity: ContinuousLocus::mean(0.50, 0.58)?,
                bonding_sensitivity: ContinuousLocus::mean(0.46, 0.54)?,
                hormone_production: ContinuousLocus::mean(0.45, 0.52)?,
                hormone_decay: ContinuousLocus::mean(0.50, 0.57)?,
                hunger_threshold: ContinuousLocus::mean(0.38, 0.44)?,
                fatigue_threshold: ContinuousLocus::mean(0.66, 0.72)?,
                sleep_threshold: ContinuousLocus::mean(0.72, 0.80)?,
                reproductive_threshold: ContinuousLocus::mean(0.62, 0.69)?,
                brain_atp_efficiency: ContinuousLocus::mean(0.52, 0.60)?,
            },
            development: DevelopmentChromosome {
                maturation_rate: ContinuousLocus::mean(0.44, 0.52)?,
                puberty_onset: ContinuousLocus::mean(0.58, 0.64)?,
                sensor_activation: ContinuousLocus::mean(0.12, 0.18)?,
                lobe_activation: ContinuousLocus::mean(0.22, 0.28)?,
                critical_period_open: ContinuousLocus::mean(0.08, 0.12)?,
                critical_period_close: ContinuousLocus::mean(0.66, 0.74)?,
                migration_checkpoint: ContinuousLocus::mean(0.82, 0.90)?,
            },
            reproduction: ReproductionChromosome {
                fertility: ContinuousLocus::mean(0.50, 0.58)?,
                crossover_probability: ContinuousLocus::mean(0.18, 0.24)?,
                max_crossover_segments: ContinuousLocus::mean(0.35, 0.45)?,
                mutation_rate: ContinuousLocus::mean(0.015, 0.025)?,
                discrete_mutation_rate: ContinuousLocus::mean(0.002, 0.006)?,
                max_mutation_delta: ContinuousLocus::mean(0.08, 0.12)?,
                parental_investment: ContinuousLocus::mean(0.48, 0.56)?,
                mate_preference: DiscreteLocus::new(
                    DiscreteAllele::new(MatePreference::Health, codominant),
                    DiscreteAllele::new(MatePreference::Novelty, recessive),
                ),
            },
            predisposition: PredispositionChromosome {
                starter_vocabulary: DiscreteLocus::new(
                    DiscreteAllele::new(StarterVocabularyProfile::Foraging, codominant),
                    DiscreteAllele::new(StarterVocabularyProfile::Social, codominant),
                ),
                reflex_strength: ContinuousLocus::mean(0.44, 0.52)?,
                food_attraction: ContinuousLocus::mean(0.54, 0.62)?,
                hazard_aversion: ContinuousLocus::mean(0.58, 0.66)?,
                social_attention: ContinuousLocus::mean(0.42, 0.50)?,
                novelty_bias: ContinuousLocus::mean(0.45, 0.53)?,
            },
        };
        genome.validate_contract()?;
        Ok(genome)
    }

    pub fn reproduce(
        maternal: &Self,
        paternal: &Self,
        conception_seed: u64,
    ) -> Result<Self, ScaffoldContractError> {
        maternal.validate_contract()?;
        paternal.validate_contract()?;
        if conception_seed == 0
            || maternal.id == paternal.id
            || maternal.foundation.compatibility_family_id
                != paternal.foundation.compatibility_family_id
            || maternal.foundation.brain_class_id != paternal.foundation.brain_class_id
            || maternal.expressed_brain_class()? != paternal.expressed_brain_class()?
        {
            return Err(ScaffoldContractError::IncompatibleGeneticClass);
        }
        let class = maternal.foundation.brain_class_id;
        if [
            maternal.brain.brain_class.maternal.value,
            maternal.brain.brain_class.paternal.value,
            paternal.brain.brain_class.maternal.value,
            paternal.brain.brain_class.paternal.value,
        ]
        .iter()
        .any(|candidate| *candidate != class)
        {
            return Err(ScaffoldContractError::IncompatibleGeneticClass);
        }

        let settings = ReproductionSettings::from_parents(maternal, paternal)?;
        let mixed_seed = conception_seed
            ^ maternal.id.0.rotate_left(17)
            ^ paternal.id.0.rotate_right(11)
            ^ maternal.lineage_id.0.rotate_left(31)
            ^ paternal.lineage_id.0.rotate_right(23);
        let mut context = ReproductionContext::new(mixed_seed, settings);
        let foundation = choose_child_foundation(maternal, paternal, &mut context.rng);
        let body = recombine_body(&maternal.body, &paternal.body, &mut context)?;
        let brain = recombine_brain(&maternal.brain, &paternal.brain, &mut context)?;
        let chemistry =
            recombine_chemistry(&maternal.chemistry, &paternal.chemistry, &mut context)?;
        let development =
            recombine_development(&maternal.development, &paternal.development, &mut context)?;
        let reproduction =
            recombine_reproduction(&maternal.reproduction, &paternal.reproduction, &mut context)?;
        let predisposition = recombine_predisposition(
            &maternal.predisposition,
            &paternal.predisposition,
            &mut context,
        )?;
        if context.mutations.len() > MAX_MUTATION_RECORDS {
            return Err(ScaffoldContractError::MutationOverflow);
        }
        let id = GenomeId(nonzero_mix(
            mixed_seed ^ context.rng.next_u64() ^ 0xC41D_6E00_0000_0001,
        ));
        let lineage_id = if maternal.lineage_id == paternal.lineage_id {
            maternal.lineage_id
        } else {
            LineageId(nonzero_mix(
                maternal.lineage_id.0 ^ paternal.lineage_id.0.rotate_left(29) ^ conception_seed,
            ))
        };
        let genome = Self {
            schema_version: CREATURE_GENOME_SCHEMA_VERSION,
            id,
            parent_genome_ids: vec![maternal.id, paternal.id],
            lineage_id,
            conception_seed,
            foundation,
            provenance: GeneticLineageProvenance {
                conception_seed,
                ordinary_birth: true,
                recombination: context.recombination,
                mutations: context.mutations,
            },
            body,
            brain,
            chemistry,
            development,
            reproduction,
            predisposition,
        };
        genome.validate_contract()?;
        Ok(genome)
    }

    pub fn expressed_brain_class(&self) -> Result<BrainClassId, ScaffoldContractError> {
        self.brain.expressed_brain_class()
    }
}

#[derive(Debug, Clone, Copy)]
struct ReproductionSettings {
    crossover_probability: f32,
    max_segments: u8,
    mutation_rate: f32,
    discrete_mutation_rate: f32,
    max_mutation_delta: f32,
}

impl ReproductionSettings {
    fn from_parents(
        maternal: &CreatureGenome,
        paternal: &CreatureGenome,
    ) -> Result<Self, ScaffoldContractError> {
        let crossover_probability = average_expressed(
            maternal.reproduction.crossover_probability,
            paternal.reproduction.crossover_probability,
        )?;
        let segment_gene = average_expressed(
            maternal.reproduction.max_crossover_segments,
            paternal.reproduction.max_crossover_segments,
        )?;
        let max_segments = 1 + (segment_gene * f32::from(MAX_CROSSOVER_SEGMENTS - 1)).round() as u8;
        let settings = Self {
            crossover_probability,
            max_segments: max_segments.clamp(1, MAX_CROSSOVER_SEGMENTS),
            mutation_rate: average_expressed(
                maternal.reproduction.mutation_rate,
                paternal.reproduction.mutation_rate,
            )?,
            discrete_mutation_rate: average_expressed(
                maternal.reproduction.discrete_mutation_rate,
                paternal.reproduction.discrete_mutation_rate,
            )?,
            max_mutation_delta: average_expressed(
                maternal.reproduction.max_mutation_delta,
                paternal.reproduction.max_mutation_delta,
            )?,
        };
        if settings.max_mutation_delta > MAX_MUTATION_DELTA {
            return Err(ScaffoldContractError::MutationOverflow);
        }
        Ok(settings)
    }
}

struct ReproductionContext {
    rng: SeededRng,
    settings: ReproductionSettings,
    recombination: Vec<ChromosomeRecombinationRecord>,
    mutations: Vec<MutationRecord>,
}

impl ReproductionContext {
    fn new(seed: u64, settings: ReproductionSettings) -> Self {
        Self {
            rng: SeededRng::new(seed),
            settings,
            recombination: Vec::with_capacity(6),
            mutations: Vec::new(),
        }
    }

    fn selectors(&mut self) -> (GameteSelector, GameteSelector) {
        (
            GameteSelector::new(
                self.rng.next_bool(),
                self.settings.crossover_probability,
                self.settings.max_segments,
            ),
            GameteSelector::new(
                self.rng.next_bool(),
                self.settings.crossover_probability,
                self.settings.max_segments,
            ),
        )
    }

    fn finish_chromosome(
        &mut self,
        chromosome: ChromosomeKind,
        maternal: GameteSelector,
        paternal: GameteSelector,
    ) {
        self.recombination.push(ChromosomeRecombinationRecord {
            chromosome,
            maternal_segments: maternal.segments,
            paternal_segments: paternal.segments,
        });
    }
}

#[derive(Debug, Clone, Copy)]
struct GameteSelector {
    select_maternal_homolog: bool,
    crossover_probability: f32,
    max_segments: u8,
    segments: u8,
    seen_locus: bool,
}

impl GameteSelector {
    fn new(select_maternal_homolog: bool, crossover_probability: f32, max_segments: u8) -> Self {
        Self {
            select_maternal_homolog,
            crossover_probability,
            max_segments,
            segments: 1,
            seen_locus: false,
        }
    }

    fn select_continuous(&mut self, locus: &ContinuousLocus, rng: &mut SeededRng) -> f32 {
        self.maybe_cross(rng);
        if self.select_maternal_homolog {
            locus.maternal
        } else {
            locus.paternal
        }
    }

    fn select_discrete<T: Copy>(
        &mut self,
        locus: &DiscreteLocus<T>,
        rng: &mut SeededRng,
    ) -> DiscreteAllele<T> {
        self.maybe_cross(rng);
        if self.select_maternal_homolog {
            locus.maternal
        } else {
            locus.paternal
        }
    }

    fn maybe_cross(&mut self, rng: &mut SeededRng) {
        if self.seen_locus
            && self.segments < self.max_segments
            && rng.next_unit() < self.crossover_probability
        {
            self.select_maternal_homolog = !self.select_maternal_homolog;
            self.segments += 1;
        }
        self.seen_locus = true;
    }
}

struct SeededRng(u64);

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self(nonzero_mix(seed))
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        nonzero_mix(self.0)
    }

    fn next_unit(&mut self) -> f32 {
        let mantissa = (self.next_u64() >> 40) as u32;
        mantissa as f32 / (1_u32 << 24) as f32
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    fn choose_index(&mut self, len: usize) -> usize {
        (self.next_u64() % len as u64) as usize
    }
}

fn choose_child_foundation(
    maternal: &CreatureGenome,
    paternal: &CreatureGenome,
    rng: &mut SeededRng,
) -> FoundationGeneticIdentity {
    if maternal.foundation == paternal.foundation {
        maternal.foundation
    } else if maternal.foundation.version != paternal.foundation.version {
        if maternal.foundation.version > paternal.foundation.version {
            maternal.foundation
        } else {
            paternal.foundation
        }
    } else if rng.next_bool() {
        maternal.foundation
    } else {
        paternal.foundation
    }
}

fn average_expressed(
    maternal: ContinuousLocus,
    paternal: ContinuousLocus,
) -> Result<f32, ScaffoldContractError> {
    Ok((maternal.expressed()? + paternal.expressed()?) * 0.5)
}

// Explicit parental loci and selectors keep allele origin unambiguous in provenance.
#[allow(clippy::too_many_arguments)]
fn child_continuous(
    maternal_locus: &ContinuousLocus,
    paternal_locus: &ContinuousLocus,
    maternal_selector: &mut GameteSelector,
    paternal_selector: &mut GameteSelector,
    context: &mut ReproductionContext,
    chromosome: ChromosomeKind,
    locus_index: u8,
    mutation_upper_override: Option<f32>,
) -> Result<ContinuousLocus, ScaffoldContractError> {
    maternal_locus.validate_contract()?;
    paternal_locus.validate_contract()?;
    if maternal_locus.lower.to_bits() != paternal_locus.lower.to_bits()
        || maternal_locus.upper.to_bits() != paternal_locus.upper.to_bits()
    {
        return Err(ScaffoldContractError::InvalidGeneticBounds);
    }
    let mut maternal = maternal_selector.select_continuous(maternal_locus, &mut context.rng);
    let mut paternal = paternal_selector.select_continuous(paternal_locus, &mut context.rng);
    let mutation_upper = mutation_upper_override.unwrap_or(maternal_locus.upper);
    mutate_continuous_value(
        &mut maternal,
        maternal_locus.lower,
        mutation_upper,
        chromosome,
        locus_index,
        AlleleSide::Maternal,
        context,
    )?;
    mutate_continuous_value(
        &mut paternal,
        paternal_locus.lower,
        mutation_upper,
        chromosome,
        locus_index,
        AlleleSide::Paternal,
        context,
    )?;
    ContinuousLocus::with_bounds(
        maternal,
        paternal,
        maternal_locus.lower,
        maternal_locus.upper,
        (maternal_locus.maternal_weight + paternal_locus.maternal_weight) * 0.5,
    )
}

fn mutate_continuous_value(
    value: &mut f32,
    lower: f32,
    upper: f32,
    chromosome: ChromosomeKind,
    locus_index: u8,
    allele: AlleleSide,
    context: &mut ReproductionContext,
) -> Result<(), ScaffoldContractError> {
    if context.rng.next_unit() >= context.settings.mutation_rate {
        return Ok(());
    }
    let before = *value;
    let signed = context.rng.next_unit().mul_add(2.0, -1.0);
    let candidate = before + signed * context.settings.max_mutation_delta;
    validate_finite(candidate)?;
    let after = reflect_into_bounds(candidate, lower, upper)?;
    *value = after;
    if before.to_bits() != after.to_bits() {
        context.mutations.push(MutationRecord::Continuous {
            chromosome,
            locus_index,
            allele,
            before,
            after,
            lower,
            upper,
        });
    }
    Ok(())
}

fn reflect_into_bounds(value: f32, lower: f32, upper: f32) -> Result<f32, ScaffoldContractError> {
    for candidate in [value, lower, upper] {
        validate_finite(candidate)?;
    }
    let width = upper - lower;
    if width <= 0.0 {
        return Err(ScaffoldContractError::InvalidGeneticBounds);
    }
    let period = width * 2.0;
    let offset = (value - lower).rem_euclid(period);
    Ok(if offset <= width {
        lower + offset
    } else {
        upper - (offset - width)
    })
}

trait DiscreteDomain: Copy + PartialEq {
    fn values() -> &'static [Self];
    fn code(self) -> u16;
}

impl DiscreteDomain for BodyFrame {
    fn values() -> &'static [Self] {
        &[Self::Light, Self::Balanced, Self::Sturdy]
    }

    fn code(self) -> u16 {
        match self {
            Self::Light => 0,
            Self::Balanced => 1,
            Self::Sturdy => 2,
        }
    }
}

impl DiscreteDomain for MatePreference {
    fn values() -> &'static [Self] {
        &[Self::Novelty, Self::Similarity, Self::Health]
    }

    fn code(self) -> u16 {
        match self {
            Self::Novelty => 0,
            Self::Similarity => 1,
            Self::Health => 2,
        }
    }
}

impl DiscreteDomain for StarterVocabularyProfile {
    fn values() -> &'static [Self] {
        &[Self::Minimal, Self::Foraging, Self::Social]
    }

    fn code(self) -> u16 {
        match self {
            Self::Minimal => 0,
            Self::Foraging => 1,
            Self::Social => 2,
        }
    }
}

fn child_discrete<T: DiscreteDomain + 'static>(
    maternal_locus: &DiscreteLocus<T>,
    paternal_locus: &DiscreteLocus<T>,
    maternal_selector: &mut GameteSelector,
    paternal_selector: &mut GameteSelector,
    context: &mut ReproductionContext,
    chromosome: ChromosomeKind,
    locus_index: u8,
) -> DiscreteLocus<T> {
    let mut maternal = maternal_selector.select_discrete(maternal_locus, &mut context.rng);
    let mut paternal = paternal_selector.select_discrete(paternal_locus, &mut context.rng);
    mutate_discrete_allele(
        &mut maternal,
        chromosome,
        locus_index,
        AlleleSide::Maternal,
        context,
    );
    mutate_discrete_allele(
        &mut paternal,
        chromosome,
        locus_index,
        AlleleSide::Paternal,
        context,
    );
    DiscreteLocus::new(maternal, paternal)
}

fn child_discrete_unmutated<T: Copy>(
    maternal_locus: &DiscreteLocus<T>,
    paternal_locus: &DiscreteLocus<T>,
    maternal_selector: &mut GameteSelector,
    paternal_selector: &mut GameteSelector,
    rng: &mut SeededRng,
) -> DiscreteLocus<T> {
    DiscreteLocus::new(
        maternal_selector.select_discrete(maternal_locus, rng),
        paternal_selector.select_discrete(paternal_locus, rng),
    )
}

fn mutate_discrete_allele<T: DiscreteDomain + 'static>(
    allele: &mut DiscreteAllele<T>,
    chromosome: ChromosomeKind,
    locus_index: u8,
    allele_side: AlleleSide,
    context: &mut ReproductionContext,
) {
    let values = T::values();
    if values.len() < 2 || context.rng.next_unit() >= context.settings.discrete_mutation_rate {
        return;
    }
    let before = allele.value;
    let current = values
        .iter()
        .position(|value| *value == before)
        .expect("discrete allele must belong to its declared domain");
    let offset = 1 + context.rng.choose_index(values.len() - 1);
    allele.value = values[(current + offset) % values.len()];
    context.mutations.push(MutationRecord::Discrete {
        chromosome,
        locus_index,
        allele: allele_side,
        before: before.code(),
        after: allele.value.code(),
    });
}

fn recombine_body(
    maternal: &BodyChromosome,
    paternal: &BodyChromosome,
    context: &mut ReproductionContext,
) -> Result<BodyChromosome, ScaffoldContractError> {
    let (mut maternal_selector, mut paternal_selector) = context.selectors();
    let chromosome = ChromosomeKind::Body;
    let result = BodyChromosome {
        size: child_continuous(
            &maternal.size,
            &paternal.size,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            0,
            None,
        )?,
        metabolic_efficiency: child_continuous(
            &maternal.metabolic_efficiency,
            &paternal.metabolic_efficiency,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            1,
            None,
        )?,
        sensory_acuity: child_continuous(
            &maternal.sensory_acuity,
            &paternal.sensory_acuity,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            2,
            None,
        )?,
        movement_efficiency: child_continuous(
            &maternal.movement_efficiency,
            &paternal.movement_efficiency,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            3,
            None,
        )?,
        lifespan: child_continuous(
            &maternal.lifespan,
            &paternal.lifespan,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            4,
            None,
        )?,
        injury_resistance: child_continuous(
            &maternal.injury_resistance,
            &paternal.injury_resistance,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            5,
            None,
        )?,
        temperature_tolerance: child_continuous(
            &maternal.temperature_tolerance,
            &paternal.temperature_tolerance,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            6,
            None,
        )?,
        appearance_hue: child_continuous(
            &maternal.appearance_hue,
            &paternal.appearance_hue,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            7,
            None,
        )?,
        frame: child_discrete(
            &maternal.frame,
            &paternal.frame,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            8,
        ),
    };
    context.finish_chromosome(chromosome, maternal_selector, paternal_selector);
    Ok(result)
}

fn recombine_brain(
    maternal: &BrainChromosome,
    paternal: &BrainChromosome,
    context: &mut ReproductionContext,
) -> Result<BrainChromosome, ScaffoldContractError> {
    let (mut maternal_selector, mut paternal_selector) = context.selectors();
    let chromosome = ChromosomeKind::Brain;
    let brain_class = child_discrete_unmutated(
        &maternal.brain_class,
        &paternal.brain_class,
        &mut maternal_selector,
        &mut paternal_selector,
        &mut context.rng,
    );
    let result = BrainChromosome {
        brain_class,
        sensory_lobe_ratio: child_continuous(
            &maternal.sensory_lobe_ratio,
            &paternal.sensory_lobe_ratio,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            1,
            None,
        )?,
        association_lobe_ratio: child_continuous(
            &maternal.association_lobe_ratio,
            &paternal.association_lobe_ratio,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            2,
            None,
        )?,
        working_memory_ratio: child_continuous(
            &maternal.working_memory_ratio,
            &paternal.working_memory_ratio,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            3,
            None,
        )?,
        connectivity_density: child_continuous(
            &maternal.connectivity_density,
            &paternal.connectivity_density,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            4,
            None,
        )?,
        plasticity: child_continuous(
            &maternal.plasticity,
            &paternal.plasticity,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            5,
            None,
        )?,
        receptor_sensitivity: child_continuous(
            &maternal.receptor_sensitivity,
            &paternal.receptor_sensitivity,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            6,
            None,
        )?,
        genetic_weight_bias: child_continuous(
            &maternal.genetic_weight_bias,
            &paternal.genetic_weight_bias,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            7,
            None,
        )?,
    };
    context.finish_chromosome(chromosome, maternal_selector, paternal_selector);
    Ok(result)
}

fn recombine_chemistry(
    maternal: &ChemistryChromosome,
    paternal: &ChemistryChromosome,
    context: &mut ReproductionContext,
) -> Result<ChemistryChromosome, ScaffoldContractError> {
    let (mut maternal_selector, mut paternal_selector) = context.selectors();
    let chromosome = ChromosomeKind::Chemistry;
    macro_rules! locus {
        ($field:ident, $index:expr) => {
            child_continuous(
                &maternal.$field,
                &paternal.$field,
                &mut maternal_selector,
                &mut paternal_selector,
                context,
                chromosome,
                $index,
                None,
            )?
        };
    }
    let result = ChemistryChromosome {
        stress_baseline: locus!(stress_baseline, 0),
        reward_sensitivity: locus!(reward_sensitivity, 1),
        bonding_sensitivity: locus!(bonding_sensitivity, 2),
        hormone_production: locus!(hormone_production, 3),
        hormone_decay: locus!(hormone_decay, 4),
        hunger_threshold: locus!(hunger_threshold, 5),
        fatigue_threshold: locus!(fatigue_threshold, 6),
        sleep_threshold: locus!(sleep_threshold, 7),
        reproductive_threshold: locus!(reproductive_threshold, 8),
        brain_atp_efficiency: locus!(brain_atp_efficiency, 9),
    };
    context.finish_chromosome(chromosome, maternal_selector, paternal_selector);
    Ok(result)
}

fn recombine_development(
    maternal: &DevelopmentChromosome,
    paternal: &DevelopmentChromosome,
    context: &mut ReproductionContext,
) -> Result<DevelopmentChromosome, ScaffoldContractError> {
    let (mut maternal_selector, mut paternal_selector) = context.selectors();
    let chromosome = ChromosomeKind::Development;
    macro_rules! locus {
        ($field:ident, $index:expr) => {
            child_continuous(
                &maternal.$field,
                &paternal.$field,
                &mut maternal_selector,
                &mut paternal_selector,
                context,
                chromosome,
                $index,
                None,
            )?
        };
    }
    let result = DevelopmentChromosome {
        maturation_rate: locus!(maturation_rate, 0),
        puberty_onset: locus!(puberty_onset, 1),
        sensor_activation: locus!(sensor_activation, 2),
        lobe_activation: locus!(lobe_activation, 3),
        critical_period_open: locus!(critical_period_open, 4),
        critical_period_close: locus!(critical_period_close, 5),
        migration_checkpoint: locus!(migration_checkpoint, 6),
    };
    context.finish_chromosome(chromosome, maternal_selector, paternal_selector);
    Ok(result)
}

fn recombine_reproduction(
    maternal: &ReproductionChromosome,
    paternal: &ReproductionChromosome,
    context: &mut ReproductionContext,
) -> Result<ReproductionChromosome, ScaffoldContractError> {
    let (mut maternal_selector, mut paternal_selector) = context.selectors();
    let chromosome = ChromosomeKind::Reproduction;
    let result = ReproductionChromosome {
        fertility: child_continuous(
            &maternal.fertility,
            &paternal.fertility,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            0,
            None,
        )?,
        crossover_probability: child_continuous(
            &maternal.crossover_probability,
            &paternal.crossover_probability,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            1,
            None,
        )?,
        max_crossover_segments: child_continuous(
            &maternal.max_crossover_segments,
            &paternal.max_crossover_segments,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            2,
            None,
        )?,
        mutation_rate: child_continuous(
            &maternal.mutation_rate,
            &paternal.mutation_rate,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            3,
            None,
        )?,
        discrete_mutation_rate: child_continuous(
            &maternal.discrete_mutation_rate,
            &paternal.discrete_mutation_rate,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            4,
            None,
        )?,
        max_mutation_delta: child_continuous(
            &maternal.max_mutation_delta,
            &paternal.max_mutation_delta,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            5,
            Some(MAX_MUTATION_DELTA),
        )?,
        parental_investment: child_continuous(
            &maternal.parental_investment,
            &paternal.parental_investment,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            6,
            None,
        )?,
        mate_preference: child_discrete(
            &maternal.mate_preference,
            &paternal.mate_preference,
            &mut maternal_selector,
            &mut paternal_selector,
            context,
            chromosome,
            7,
        ),
    };
    context.finish_chromosome(chromosome, maternal_selector, paternal_selector);
    Ok(result)
}

fn recombine_predisposition(
    maternal: &PredispositionChromosome,
    paternal: &PredispositionChromosome,
    context: &mut ReproductionContext,
) -> Result<PredispositionChromosome, ScaffoldContractError> {
    let (mut maternal_selector, mut paternal_selector) = context.selectors();
    let chromosome = ChromosomeKind::Predisposition;
    let starter_vocabulary = child_discrete(
        &maternal.starter_vocabulary,
        &paternal.starter_vocabulary,
        &mut maternal_selector,
        &mut paternal_selector,
        context,
        chromosome,
        0,
    );
    macro_rules! locus {
        ($field:ident, $index:expr) => {
            child_continuous(
                &maternal.$field,
                &paternal.$field,
                &mut maternal_selector,
                &mut paternal_selector,
                context,
                chromosome,
                $index,
                None,
            )?
        };
    }
    let result = PredispositionChromosome {
        starter_vocabulary,
        reflex_strength: locus!(reflex_strength, 1),
        food_attraction: locus!(food_attraction, 2),
        hazard_aversion: locus!(hazard_aversion, 3),
        social_attention: locus!(social_attention, 4),
        novelty_bias: locus!(novelty_bias, 5),
    };
    context.finish_chromosome(chromosome, maternal_selector, paternal_selector);
    Ok(result)
}

impl Validate for CreatureGenome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        crate::require_version(
            SchemaKind::Genome,
            CREATURE_GENOME_SCHEMA_VERSION,
            self.schema_version,
        )?;
        self.id.validate()?;
        self.lineage_id.validate()?;
        if self.conception_seed == 0 || !matches!(self.parent_genome_ids.len(), 0 | 2) {
            return Err(ScaffoldContractError::InvalidId);
        }
        for parent in &self.parent_genome_ids {
            parent.validate()?;
        }
        if self.parent_genome_ids.len() == 2
            && self.parent_genome_ids[0] == self.parent_genome_ids[1]
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.foundation.validate_contract()?;
        self.provenance.validate_contract()?;
        if self.provenance.conception_seed != self.conception_seed
            || self.provenance.ordinary_birth != (self.parent_genome_ids.len() == 2)
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.body.validate_contract()?;
        self.brain.validate_contract()?;
        self.chemistry.validate_contract()?;
        self.development.validate_contract()?;
        self.reproduction.validate_contract()?;
        self.predisposition.validate_contract()?;
        if self.expressed_brain_class()? != self.foundation.brain_class_id {
            return Err(ScaffoldContractError::IncompatibleGeneticClass);
        }
        Ok(())
    }
}

fn validate_loci(loci: &[&ContinuousLocus]) -> Result<(), ScaffoldContractError> {
    for locus in loci {
        locus.validate_contract()?;
    }
    Ok(())
}

fn nonzero_mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let mixed = value ^ (value >> 31);
    if mixed == 0 {
        1
    } else {
        mixed
    }
}
