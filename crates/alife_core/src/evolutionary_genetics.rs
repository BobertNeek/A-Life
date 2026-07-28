//! Contract-only EI0 diploid genome, chromosome, and allele-expression records.

use serde::{Deserialize, Serialize};

use crate::{
    validate_finite, BrainCapacityClass, BrainClassId, GenomeId, LineageId, ScaffoldContractError,
    SchemaKind, Validate,
};

pub const CREATURE_GENOME_SCHEMA_VERSION: u16 = 1;
pub const MAX_CROSSOVER_SEGMENTS: u8 = 8;
pub const MAX_MUTATION_DELTA: f32 = 0.25;

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

    pub fn expressed_brain_class(&self) -> Result<BrainClassId, ScaffoldContractError> {
        self.brain.expressed_brain_class()
    }
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
