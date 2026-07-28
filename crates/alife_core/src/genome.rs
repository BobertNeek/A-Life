//! Genome, development, and weight-split contracts.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    ensure_current_version, validate_finite, BrainClassId, BrainScaleTier, GenomeId, LineageId,
    LobeKind, NeuronIndex, NormalizedScalar, ScaffoldContractError, SchemaKind, SchemaVersions,
    Tick, Validate,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BrainGenome {
    pub schema_version: u16,
    pub id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub lineage_id: Option<LineageId>,
    pub species_seed: u64,
    pub brain_class_id: BrainClassId,
    pub genetic_prior_seed: u64,
    pub seeds: GenomeSeedSet,
    pub lobe_ratios: LobeRatioPlan,
    pub macro_connectome_masks: Vec<MacroConnectomeMask>,
    pub sparse_density_priors: Vec<SparseDensityPrior>,
    pub alpha_mask: AlphaMask,
    pub plasticity_mask: PlasticityMask,
    plasticity_parameters: PlasticityGenomeParameters,
    pub endocrine_constants: Vec<EndocrineConstantGene>,
    pub drive_thresholds: Vec<DriveThresholdGene>,
    pub sensor_layout: SensorLayoutGene,
    pub motor_affordances: Vec<MotorAffordanceGene>,
    pub mutation_rates: MutationRates,
    pub crossover: CrossoverPolicy,
    pub developmental_schedule: DevelopmentalSchedule,
    pub inheritance: InheritancePolicy,
}

impl BrainGenome {
    pub const SCHEMA_VERSION: u16 = SchemaVersions::CURRENT.genome.0;

    pub fn scaffold(species_seed: u64, brain_class_id: BrainClassId) -> Self {
        let seeds = GenomeSeedSet::from_species_seed(species_seed, brain_class_id);
        Self {
            schema_version: Self::SCHEMA_VERSION,
            id: GenomeId(seeds.genome_id_seed),
            parent_genome_ids: Vec::new(),
            lineage_id: None,
            species_seed,
            brain_class_id,
            genetic_prior_seed: seeds.genetic_prior_seed,
            seeds,
            lobe_ratios: LobeRatioPlan::ClassDefault,
            macro_connectome_masks: if brain_class_id == crate::BrainCapacityClass::N2048_ID {
                MacroConnectomeMask::n2048_foundation_defaults()
            } else {
                MacroConnectomeMask::scaffold_defaults()
            },
            sparse_density_priors: if brain_class_id == crate::BrainCapacityClass::N2048_ID {
                SparseDensityPrior::n2048_foundation_defaults()
            } else {
                SparseDensityPrior::scaffold_defaults()
            },
            alpha_mask: AlphaMask::default_for_projection(NormalizedScalar(0.25)),
            plasticity_mask: PlasticityMask::scaffold_default(),
            plasticity_parameters: PlasticityGenomeParameters::canonical_default(),
            endocrine_constants: EndocrineConstantGene::baseline_defaults(),
            drive_thresholds: DriveThresholdGene::baseline_defaults(),
            sensor_layout: SensorLayoutGene::minimal_grounded(),
            motor_affordances: MotorAffordanceGene::minimal_grounded(),
            mutation_rates: MutationRates::conservative_defaults(),
            crossover: CrossoverPolicy::conservative_defaults(),
            developmental_schedule: DevelopmentalSchedule::standard(brain_class_id)
                .expect("canonical scaffold developmental schedule must validate"),
            inheritance: InheritancePolicy::default(),
        }
    }

    pub const fn plasticity_parameters(&self) -> &PlasticityGenomeParameters {
        &self.plasticity_parameters
    }

    /// Replace the heritable plasticity parameters through their validated
    /// constructor boundary. This is the causal mutation/training entrypoint;
    /// callers cannot write unchecked parameter lanes.
    pub fn with_plasticity_parameters(
        mut self,
        parameters: PlasticityGenomeParameters,
    ) -> Result<Self, ScaffoldContractError> {
        parameters.validate_contract()?;
        self.plasticity_parameters = parameters;
        self.validate_contract()?;
        Ok(self)
    }
}

impl Validate for BrainGenome {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::Genome, self.schema_version)?;
        self.id.validate()?;
        self.brain_class_id.validate()?;
        validate_known_brain_class(self.brain_class_id)?;
        for parent in &self.parent_genome_ids {
            parent.validate()?;
        }
        if let Some(lineage_id) = self.lineage_id {
            lineage_id.validate()?;
        }
        self.seeds.validate_contract()?;
        if self.genetic_prior_seed == 0 || self.genetic_prior_seed != self.seeds.genetic_prior_seed
        {
            return Err(ScaffoldContractError::InvalidId);
        }
        self.lobe_ratios.validate_contract()?;
        validate_all(&self.macro_connectome_masks)?;
        validate_all(&self.sparse_density_priors)?;
        validate_connectome_density_contract(
            &self.macro_connectome_masks,
            &self.sparse_density_priors,
        )?;
        self.alpha_mask.validate_contract()?;
        self.plasticity_mask.validate_contract()?;
        self.plasticity_parameters.validate_contract()?;
        validate_all(&self.endocrine_constants)?;
        validate_all(&self.drive_thresholds)?;
        self.sensor_layout.validate_contract()?;
        validate_all(&self.motor_affordances)?;
        self.mutation_rates.validate_contract()?;
        self.crossover.validate_contract()?;
        self.developmental_schedule.validate_contract()?;
        self.inheritance.validate_contract()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenomeSeedSet {
    pub species_seed: u64,
    pub genome_id_seed: u64,
    pub genetic_prior_seed: u64,
    pub mutation_seed: u64,
    pub crossover_seed: u64,
    pub development_seed: u64,
    pub sensor_layout_seed: u64,
}

impl GenomeSeedSet {
    pub fn from_species_seed(species_seed: u64, brain_class_id: BrainClassId) -> Self {
        Self {
            species_seed,
            genome_id_seed: derive_seed(species_seed, brain_class_id, 0xA11F_E001),
            genetic_prior_seed: derive_seed(species_seed, brain_class_id, 0xA11F_E002),
            mutation_seed: derive_seed(species_seed, brain_class_id, 0xA11F_E003),
            crossover_seed: derive_seed(species_seed, brain_class_id, 0xA11F_E004),
            development_seed: derive_seed(species_seed, brain_class_id, 0xA11F_E005),
            sensor_layout_seed: derive_seed(species_seed, brain_class_id, 0xA11F_E006),
        }
    }
}

impl Validate for GenomeSeedSet {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        for seed in [
            self.genome_id_seed,
            self.genetic_prior_seed,
            self.mutation_seed,
            self.crossover_seed,
            self.development_seed,
            self.sensor_layout_seed,
        ] {
            if seed == 0 {
                return Err(ScaffoldContractError::InvalidId);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LobeRatioPlan {
    ClassDefault,
    RegistryRef(LobeRatioRegistryRef),
    InlineOverrides(Vec<LobeRatioOverride>),
}

impl Validate for LobeRatioPlan {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        match self {
            Self::ClassDefault => Ok(()),
            Self::RegistryRef(registry_ref) => registry_ref.validate_contract(),
            Self::InlineOverrides(overrides) => validate_all(overrides),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LobeRatioRegistryRef {
    pub registry_id: u64,
    pub version: u16,
}

impl Validate for LobeRatioRegistryRef {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.registry_id == 0 || self.version == 0 {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LobeRatioOverride {
    pub lobe: LobeKind,
    pub ratio: NormalizedScalar,
}

impl Validate for LobeRatioOverride {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.ratio)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectionKey {
    pub source_lobe: LobeKind,
    pub target_lobe: LobeKind,
}

impl ProjectionKey {
    pub const fn new(source_lobe: LobeKind, target_lobe: LobeKind) -> Self {
        Self {
            source_lobe,
            target_lobe,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroConnectomeMask {
    pub projection: ProjectionKey,
    pub enabled: bool,
    pub structural_growth_allowed: bool,
}

impl MacroConnectomeMask {
    pub fn scaffold_defaults() -> Vec<Self> {
        [
            ProjectionKey::new(LobeKind::SensoryGrounding, LobeKind::CoreAssociation),
            ProjectionKey::new(LobeKind::MetabolicDrive, LobeKind::HomeostaticRegulation),
            ProjectionKey::new(LobeKind::CoreAssociation, LobeKind::MotorArbitration),
            ProjectionKey::new(LobeKind::MotorArbitration, LobeKind::MotorArbitration),
        ]
        .into_iter()
        .map(|projection| Self {
            projection,
            enabled: true,
            structural_growth_allowed: false,
        })
        .collect()
    }

    pub fn n2048_foundation_defaults() -> Vec<Self> {
        crate::N2048FoundationLayoutV1::route_specs()
            .iter()
            .map(|route| Self {
                projection: ProjectionKey::new(route.source_lobe(), route.target_lobe()),
                enabled: true,
                structural_growth_allowed: false,
            })
            .collect()
    }
}

impl Validate for MacroConnectomeMask {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.structural_growth_allowed || !is_canonical_slice_a_projection(self.projection) {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SparseDensityPrior {
    pub projection: ProjectionKey,
    pub density: NormalizedScalar,
    pub max_active_synapse_share: NormalizedScalar,
}

impl SparseDensityPrior {
    pub fn scaffold_defaults() -> Vec<Self> {
        vec![
            Self {
                projection: ProjectionKey::new(
                    LobeKind::SensoryGrounding,
                    LobeKind::CoreAssociation,
                ),
                density: NormalizedScalar(0.04),
                max_active_synapse_share: NormalizedScalar(0.35),
            },
            Self {
                projection: ProjectionKey::new(
                    LobeKind::CoreAssociation,
                    LobeKind::MotorArbitration,
                ),
                density: NormalizedScalar(0.03),
                max_active_synapse_share: NormalizedScalar(0.25),
            },
            Self {
                projection: ProjectionKey::new(
                    LobeKind::MetabolicDrive,
                    LobeKind::HomeostaticRegulation,
                ),
                density: NormalizedScalar(0.02),
                max_active_synapse_share: NormalizedScalar(0.15),
            },
            Self {
                projection: ProjectionKey::new(
                    LobeKind::MotorArbitration,
                    LobeKind::MotorArbitration,
                ),
                density: NormalizedScalar(0.05),
                max_active_synapse_share: NormalizedScalar(0.10),
            },
        ]
    }

    pub fn n2048_foundation_defaults() -> Vec<Self> {
        let layout = crate::N2048FoundationLayoutV1::lobe_layout();
        crate::N2048FoundationLayoutV1::route_specs()
            .iter()
            .map(|route| {
                let source = layout
                    .region(route.source_lobe())
                    .expect("frozen source lobe");
                let target = layout
                    .region(route.target_lobe())
                    .expect("frozen target lobe");
                let possible = (u64::from(source.len) * u64::from(target.len)) as f32;
                Self {
                    projection: ProjectionKey::new(route.source_lobe(), route.target_lobe()),
                    density: NormalizedScalar(route.synapse_count() as f32 / possible),
                    max_active_synapse_share: NormalizedScalar(
                        route.synapse_count() as f32
                            / crate::N2048FoundationLayoutV1::RECURRENT_SYNAPSE_COUNT as f32,
                    ),
                }
            })
            .collect()
    }
}

impl Validate for SparseDensityPrior {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.density)?;
        validate_normalized(self.max_active_synapse_share)
    }
}

fn validate_connectome_density_contract(
    masks: &[MacroConnectomeMask],
    densities: &[SparseDensityPrior],
) -> Result<(), ScaffoldContractError> {
    for (index, mask) in masks.iter().enumerate() {
        if masks[index + 1..]
            .iter()
            .any(|other| other.projection == mask.projection)
        {
            return Err(ScaffoldContractError::RoutingDuplicateMask);
        }
        let density_count = densities
            .iter()
            .filter(|density| density.projection == mask.projection)
            .count();
        if (mask.enabled && density_count != 1) || (!mask.enabled && density_count != 0) {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
    }
    for (index, density) in densities.iter().enumerate() {
        if densities[index + 1..]
            .iter()
            .any(|other| other.projection == density.projection)
            || !masks
                .iter()
                .any(|mask| mask.enabled && mask.projection == density.projection)
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
    }
    Ok(())
}

const fn is_canonical_slice_a_projection(key: ProjectionKey) -> bool {
    matches!(
        (key.source_lobe, key.target_lobe),
        (LobeKind::SensoryGrounding, LobeKind::CoreAssociation)
            | (LobeKind::CoreAssociation, LobeKind::MotorArbitration)
            | (LobeKind::MetabolicDrive, LobeKind::HomeostaticRegulation)
            | (LobeKind::MotorArbitration, LobeKind::MotorArbitration)
            | (LobeKind::AuditorySpeech, LobeKind::CoreAssociation)
            | (LobeKind::GlyphVision, LobeKind::CoreAssociation)
            | (LobeKind::HomeostaticRegulation, LobeKind::CoreAssociation)
            | (LobeKind::HomeostaticRegulation, LobeKind::MotorArbitration)
            | (LobeKind::CoreAssociation, LobeKind::WorkingMemory)
            | (LobeKind::WorkingMemory, LobeKind::CoreAssociation)
            | (LobeKind::CoreAssociation, LobeKind::EpisodicMemory)
            | (LobeKind::EpisodicMemory, LobeKind::CoreAssociation)
            | (LobeKind::CoreAssociation, LobeKind::LexiconConcept)
            | (LobeKind::LexiconConcept, LobeKind::CoreAssociation)
            | (LobeKind::LexiconConcept, LobeKind::WorkingMemory)
            | (LobeKind::WorkingMemory, LobeKind::LexiconConcept)
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlphaStoragePolicy {
    HierarchicalSparse,
    DenseDebugReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlphaMask {
    pub storage_policy: AlphaStoragePolicy,
    pub default_alpha: NormalizedScalar,
    pub projection_overrides: Vec<ProjectionAlphaOverride>,
    pub lobe_overrides: Vec<LobeAlphaOverride>,
    pub tile_overrides: Vec<TileAlphaOverride>,
    pub per_synapse_overrides: Vec<SynapseAlphaOverride>,
    pub dense_reference_opt_in: bool,
}

impl AlphaMask {
    pub fn default_for_projection(default_alpha: NormalizedScalar) -> Self {
        Self {
            storage_policy: AlphaStoragePolicy::HierarchicalSparse,
            default_alpha,
            projection_overrides: Vec::new(),
            lobe_overrides: Vec::new(),
            tile_overrides: Vec::new(),
            per_synapse_overrides: Vec::new(),
            dense_reference_opt_in: false,
        }
    }
}

impl Validate for AlphaMask {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.storage_policy == AlphaStoragePolicy::DenseDebugReference
            && !self.dense_reference_opt_in
        {
            return Err(ScaffoldContractError::DenseAlphaRequiresOptIn);
        }
        validate_normalized(self.default_alpha)?;
        validate_all(&self.projection_overrides)?;
        validate_all(&self.lobe_overrides)?;
        validate_all(&self.tile_overrides)?;
        validate_all(&self.per_synapse_overrides)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectionAlphaOverride {
    pub projection: ProjectionKey,
    pub alpha: NormalizedScalar,
}

impl Validate for ProjectionAlphaOverride {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.alpha)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LobeAlphaOverride {
    pub lobe: LobeKind,
    pub alpha: NormalizedScalar,
}

impl Validate for LobeAlphaOverride {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.alpha)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TileAddress {
    pub lobe: LobeKind,
    pub tile_index: u32,
}

impl TileAddress {
    pub const fn new(lobe: LobeKind, tile_index: u32) -> Self {
        Self { lobe, tile_index }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TileAlphaOverride {
    pub tile: TileAddress,
    pub alpha: NormalizedScalar,
}

impl Validate for TileAlphaOverride {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.alpha)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SynapseAddress {
    pub source: NeuronIndex,
    pub target: NeuronIndex,
}

impl SynapseAddress {
    pub const fn new(source: u32, target: u32) -> Self {
        Self {
            source: NeuronIndex(source),
            target: NeuronIndex(target),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SynapseAlphaOverride {
    pub synapse: SynapseAddress,
    pub alpha: NormalizedScalar,
    pub exceptional_reason: String,
}

impl Validate for SynapseAlphaOverride {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.alpha)?;
        if self.exceptional_reason.trim().is_empty() {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlasticityMask {
    pub oja_enabled: bool,
    pub hebbian_enabled: bool,
    pub projection_masks: Vec<ProjectionPlasticityMask>,
}

/// Versioned heritable parameters from which phenotype receptor and sleep
/// plans are compiled. Fields are private so invalid learning lanes cannot be
/// introduced through struct literals or unchecked deserialization.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PlasticityGenomeParameters {
    schema_version: u16,
    eligibility_decay: f32,
    base_learning_rate: f32,
    normalization_rate: f32,
    sleep_replay_rate: f32,
    modulator_sign: f32,
    fast_min: f32,
    fast_max: f32,
    sleep_staging_rate: f32,
    sleep_weight_limit: f32,
    sleep_fast_decay_rate: f32,
}

impl PlasticityGenomeParameters {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new_v1(
        eligibility_decay: f32,
        base_learning_rate: f32,
        normalization_rate: f32,
        sleep_replay_rate: f32,
        modulator_sign: f32,
        fast_min: f32,
        fast_max: f32,
        sleep_staging_rate: f32,
        sleep_weight_limit: f32,
        sleep_fast_decay_rate: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            schema_version: SchemaVersions::CURRENT.learning.raw(),
            eligibility_decay,
            base_learning_rate,
            normalization_rate,
            sleep_replay_rate,
            modulator_sign,
            fast_min,
            fast_max,
            sleep_staging_rate,
            sleep_weight_limit,
            sleep_fast_decay_rate,
        };
        value.validate_contract()?;
        Ok(value)
    }

    const fn canonical_default() -> Self {
        Self {
            schema_version: SchemaVersions::CURRENT.learning.raw(),
            eligibility_decay: 0.95,
            base_learning_rate: 0.01,
            normalization_rate: 0.001,
            sleep_replay_rate: 0.25,
            modulator_sign: 1.0,
            fast_min: -2.0,
            fast_max: 2.0,
            sleep_staging_rate: 0.5,
            sleep_weight_limit: 4.0,
            sleep_fast_decay_rate: 0.5,
        }
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn eligibility_decay(&self) -> f32 {
        self.eligibility_decay
    }
    pub const fn base_learning_rate(&self) -> f32 {
        self.base_learning_rate
    }
    pub const fn normalization_rate(&self) -> f32 {
        self.normalization_rate
    }
    pub const fn sleep_replay_rate(&self) -> f32 {
        self.sleep_replay_rate
    }
    pub const fn modulator_sign(&self) -> f32 {
        self.modulator_sign
    }
    pub const fn fast_bounds(&self) -> (f32, f32) {
        (self.fast_min, self.fast_max)
    }
    pub const fn sleep_staging_rate(&self) -> f32 {
        self.sleep_staging_rate
    }
    pub const fn sleep_weight_limit(&self) -> f32 {
        self.sleep_weight_limit
    }
    pub const fn sleep_fast_decay_rate(&self) -> f32 {
        self.sleep_fast_decay_rate
    }
}

impl Validate for PlasticityGenomeParameters {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        ensure_current_version(SchemaKind::Learning, self.schema_version)?;
        let values = [
            self.eligibility_decay,
            self.base_learning_rate,
            self.normalization_rate,
            self.sleep_replay_rate,
            self.modulator_sign,
            self.fast_min,
            self.fast_max,
            self.sleep_staging_rate,
            self.sleep_weight_limit,
            self.sleep_fast_decay_rate,
        ];
        if values.into_iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if !(0.0..=1.0).contains(&self.eligibility_decay)
            || !(0.0..=1.0).contains(&self.base_learning_rate)
            || self.base_learning_rate == 0.0
            || !(0.0..=1.0).contains(&self.normalization_rate)
            || !(0.0..=1.0).contains(&self.sleep_replay_rate)
            || !matches!(self.modulator_sign, -1.0 | 1.0)
            || !(-8.0..=8.0).contains(&self.fast_min)
            || !(-8.0..=8.0).contains(&self.fast_max)
            || self.fast_min >= self.fast_max
            || !(0.0..=1.0).contains(&self.sleep_staging_rate)
            || self.sleep_staging_rate == 0.0
            || !(0.0..=8.0).contains(&self.sleep_weight_limit)
            || self.sleep_weight_limit == 0.0
            || !(0.0..=1.0).contains(&self.sleep_fast_decay_rate)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for PlasticityGenomeParameters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            eligibility_decay: f32,
            base_learning_rate: f32,
            normalization_rate: f32,
            sleep_replay_rate: f32,
            modulator_sign: f32,
            fast_min: f32,
            fast_max: f32,
            sleep_staging_rate: f32,
            sleep_weight_limit: f32,
            sleep_fast_decay_rate: f32,
        }
        let wire = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: wire.schema_version,
            eligibility_decay: wire.eligibility_decay,
            base_learning_rate: wire.base_learning_rate,
            normalization_rate: wire.normalization_rate,
            sleep_replay_rate: wire.sleep_replay_rate,
            modulator_sign: wire.modulator_sign,
            fast_min: wire.fast_min,
            fast_max: wire.fast_max,
            sleep_staging_rate: wire.sleep_staging_rate,
            sleep_weight_limit: wire.sleep_weight_limit,
            sleep_fast_decay_rate: wire.sleep_fast_decay_rate,
        };
        value.validate_contract().map_err(D::Error::custom)?;
        Ok(value)
    }
}

impl PlasticityMask {
    pub fn scaffold_default() -> Self {
        Self {
            oja_enabled: true,
            hebbian_enabled: true,
            projection_masks: vec![ProjectionPlasticityMask {
                projection: ProjectionKey::new(
                    LobeKind::CoreAssociation,
                    LobeKind::MotorArbitration,
                ),
                learning_rate_scale: NormalizedScalar(0.5),
                plasticity_enabled: true,
            }],
        }
    }
}

impl Validate for PlasticityMask {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_all(&self.projection_masks)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectionPlasticityMask {
    pub projection: ProjectionKey,
    pub learning_rate_scale: NormalizedScalar,
    pub plasticity_enabled: bool,
}

impl Validate for ProjectionPlasticityMask {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.learning_rate_scale)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EndocrineConstantKind {
    DopamineBaseline = 0,
    SerotoninBaseline = 1,
    CortisolBaseline = 2,
    OxytocinBaseline = 3,
    AdrenalineBaseline = 4,
    AcetylcholineBaseline = 5,
    BrainAtpBaseline = 6,
    DevelopmentHormoneBaseline = 7,
}

impl EndocrineConstantKind {
    pub const fn raw(self) -> u8 {
        match self {
            Self::DopamineBaseline => 0,
            Self::SerotoninBaseline => 1,
            Self::CortisolBaseline => 2,
            Self::OxytocinBaseline => 3,
            Self::AdrenalineBaseline => 4,
            Self::AcetylcholineBaseline => 5,
            Self::BrainAtpBaseline => 6,
            Self::DevelopmentHormoneBaseline => 7,
        }
    }
    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::DopamineBaseline),
            1 => Ok(Self::SerotoninBaseline),
            2 => Ok(Self::CortisolBaseline),
            3 => Ok(Self::OxytocinBaseline),
            4 => Ok(Self::AdrenalineBaseline),
            5 => Ok(Self::AcetylcholineBaseline),
            6 => Ok(Self::BrainAtpBaseline),
            7 => Ok(Self::DevelopmentHormoneBaseline),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EndocrineConstantGene {
    pub kind: EndocrineConstantKind,
    pub value: f32,
}

impl EndocrineConstantGene {
    pub fn baseline_defaults() -> Vec<Self> {
        vec![
            Self {
                kind: EndocrineConstantKind::DopamineBaseline,
                value: 1.0,
            },
            Self {
                kind: EndocrineConstantKind::SerotoninBaseline,
                value: 1.0,
            },
            Self {
                kind: EndocrineConstantKind::CortisolBaseline,
                value: 0.2,
            },
            Self {
                kind: EndocrineConstantKind::BrainAtpBaseline,
                value: 1.0,
            },
        ]
    }
}

impl Validate for EndocrineConstantGene {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_finite(self.value)?;
        Ok(())
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DriveThresholdKind {
    Hunger = 0,
    Fatigue = 1,
    Fear = 2,
    Pain = 3,
    Loneliness = 4,
    Curiosity = 5,
    Reproduction = 6,
}

impl DriveThresholdKind {
    pub const fn raw(self) -> u8 {
        match self {
            Self::Hunger => 0,
            Self::Fatigue => 1,
            Self::Fear => 2,
            Self::Pain => 3,
            Self::Loneliness => 4,
            Self::Curiosity => 5,
            Self::Reproduction => 6,
        }
    }
    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::Hunger),
            1 => Ok(Self::Fatigue),
            2 => Ok(Self::Fear),
            3 => Ok(Self::Pain),
            4 => Ok(Self::Loneliness),
            5 => Ok(Self::Curiosity),
            6 => Ok(Self::Reproduction),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DriveThresholdGene {
    pub kind: DriveThresholdKind,
    pub threshold: NormalizedScalar,
}

impl DriveThresholdGene {
    pub fn baseline_defaults() -> Vec<Self> {
        vec![
            Self {
                kind: DriveThresholdKind::Hunger,
                threshold: NormalizedScalar(0.4),
            },
            Self {
                kind: DriveThresholdKind::Fatigue,
                threshold: NormalizedScalar(0.7),
            },
            Self {
                kind: DriveThresholdKind::Curiosity,
                threshold: NormalizedScalar(0.25),
            },
        ]
    }
}

impl Validate for DriveThresholdGene {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.threshold)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SensorChannelKind {
    Vision = 0,
    Hearing = 1,
    Touch = 2,
    Smell = 3,
    Taste = 4,
    Proprioception = 5,
    Interoception = 6,
    GlyphVision = 7,
}

impl SensorChannelKind {
    pub const fn raw(self) -> u8 {
        match self {
            Self::Vision => 0,
            Self::Hearing => 1,
            Self::Touch => 2,
            Self::Smell => 3,
            Self::Taste => 4,
            Self::Proprioception => 5,
            Self::Interoception => 6,
            Self::GlyphVision => 7,
        }
    }
    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::Vision),
            1 => Ok(Self::Hearing),
            2 => Ok(Self::Touch),
            3 => Ok(Self::Smell),
            4 => Ok(Self::Taste),
            5 => Ok(Self::Proprioception),
            6 => Ok(Self::Interoception),
            7 => Ok(Self::GlyphVision),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorLayoutGene {
    pub channels: Vec<SensorChannelGene>,
}

impl SensorLayoutGene {
    pub fn minimal_grounded() -> Self {
        Self {
            channels: vec![
                SensorChannelGene {
                    kind: SensorChannelKind::Interoception,
                    receptor_count: 16,
                    target_lobe: LobeKind::MetabolicDrive,
                    enabled_at_maturation: 0,
                },
                SensorChannelGene {
                    kind: SensorChannelKind::Vision,
                    receptor_count: 64,
                    target_lobe: LobeKind::SensoryGrounding,
                    enabled_at_maturation: 0,
                },
                SensorChannelGene {
                    kind: SensorChannelKind::Touch,
                    receptor_count: 24,
                    target_lobe: LobeKind::SensoryGrounding,
                    enabled_at_maturation: 0,
                },
            ],
        }
    }
}

impl Validate for SensorLayoutGene {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_all(&self.channels)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SensorChannelGene {
    pub kind: SensorChannelKind,
    pub receptor_count: u16,
    pub target_lobe: LobeKind,
    pub enabled_at_maturation: u8,
}

impl Validate for SensorChannelGene {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.receptor_count == 0 || self.enabled_at_maturation > 100 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MotorAffordanceKind {
    Move = 0,
    Turn = 1,
    Eat = 2,
    Rest = 3,
    Interact = 4,
    Vocalize = 5,
    Write = 6,
    Gesture = 7,
    Reproduce = 8,
}

impl MotorAffordanceKind {
    pub const fn raw(self) -> u8 {
        match self {
            Self::Move => 0,
            Self::Turn => 1,
            Self::Eat => 2,
            Self::Rest => 3,
            Self::Interact => 4,
            Self::Vocalize => 5,
            Self::Write => 6,
            Self::Gesture => 7,
            Self::Reproduce => 8,
        }
    }
    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::Move),
            1 => Ok(Self::Turn),
            2 => Ok(Self::Eat),
            3 => Ok(Self::Rest),
            4 => Ok(Self::Interact),
            5 => Ok(Self::Vocalize),
            6 => Ok(Self::Write),
            7 => Ok(Self::Gesture),
            8 => Ok(Self::Reproduce),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotorAffordanceGene {
    pub kind: MotorAffordanceKind,
    pub enabled: bool,
    pub motor_lobe_units: u16,
    pub enabled_at_maturation: u8,
}

impl MotorAffordanceGene {
    pub fn minimal_grounded() -> Vec<Self> {
        vec![
            Self {
                kind: MotorAffordanceKind::Move,
                enabled: true,
                motor_lobe_units: 16,
                enabled_at_maturation: 0,
            },
            Self {
                kind: MotorAffordanceKind::Eat,
                enabled: true,
                motor_lobe_units: 8,
                enabled_at_maturation: 0,
            },
            Self {
                kind: MotorAffordanceKind::Rest,
                enabled: true,
                motor_lobe_units: 8,
                enabled_at_maturation: 0,
            },
            Self {
                kind: MotorAffordanceKind::Interact,
                enabled: true,
                motor_lobe_units: 8,
                enabled_at_maturation: 20,
            },
        ]
    }
}

impl Validate for MotorAffordanceGene {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.enabled && self.motor_lobe_units == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.enabled_at_maturation > 100 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MutationRates {
    pub point: NormalizedScalar,
    pub structural: NormalizedScalar,
    pub lobe_ratio: NormalizedScalar,
    pub density: NormalizedScalar,
    pub alpha: NormalizedScalar,
    pub endocrine: NormalizedScalar,
    pub developmental_schedule: NormalizedScalar,
}

impl MutationRates {
    pub const fn conservative_defaults() -> Self {
        Self {
            point: NormalizedScalar(0.01),
            structural: NormalizedScalar(0.002),
            lobe_ratio: NormalizedScalar(0.01),
            density: NormalizedScalar(0.01),
            alpha: NormalizedScalar(0.01),
            endocrine: NormalizedScalar(0.005),
            developmental_schedule: NormalizedScalar(0.002),
        }
    }
}

impl Validate for MutationRates {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        for value in [
            self.point,
            self.structural,
            self.lobe_ratio,
            self.density,
            self.alpha,
            self.endocrine,
            self.developmental_schedule,
        ] {
            validate_normalized(value)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CrossoverPolicy {
    pub enabled: bool,
    pub max_segments: u8,
    pub parent_mix_bias: NormalizedScalar,
}

impl CrossoverPolicy {
    pub const fn conservative_defaults() -> Self {
        Self {
            enabled: true,
            max_segments: 4,
            parent_mix_bias: NormalizedScalar(0.5),
        }
    }
}

impl Validate for CrossoverPolicy {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.parent_mix_bias)?;
        if self.enabled && self.max_segments == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DevelopmentStage {
    Hatchling = 0,
    Juvenile = 1,
    Adolescent = 2,
    Adult = 3,
    Elder = 4,
}

impl DevelopmentStage {
    pub const fn raw(self) -> u8 {
        match self {
            Self::Hatchling => 0,
            Self::Juvenile => 1,
            Self::Adolescent => 2,
            Self::Adult => 3,
            Self::Elder => 4,
        }
    }
    pub fn try_from_raw(raw: u8) -> Result<Self, ScaffoldContractError> {
        match raw {
            0 => Ok(Self::Hatchling),
            1 => Ok(Self::Juvenile),
            2 => Ok(Self::Adolescent),
            3 => Ok(Self::Adult),
            4 => Ok(Self::Elder),
            _ => Err(ScaffoldContractError::PhenotypeCompile),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentalSchedule {
    pub milestones: Vec<DevelopmentalMilestone>,
    pub critical_periods: Vec<CriticalPeriod>,
    pub consolidation_cadence_ticks: u32,
    pub sleep_pressure_maturation_gate: NormalizedScalar,
}

impl DevelopmentalSchedule {
    pub fn standard(brain_class_id: BrainClassId) -> Result<Self, ScaffoldContractError> {
        validate_known_brain_class(brain_class_id)?;
        let schedule = Self {
            milestones: vec![
                DevelopmentalMilestone {
                    stage: DevelopmentStage::Hatchling,
                    begins_at: Tick(0),
                    maturation: NormalizedScalar(0.0),
                    target_brain_class_id: Some(brain_class_id),
                },
                DevelopmentalMilestone {
                    stage: DevelopmentStage::Juvenile,
                    begins_at: Tick(600),
                    maturation: NormalizedScalar(0.35),
                    target_brain_class_id: None,
                },
                DevelopmentalMilestone {
                    stage: DevelopmentStage::Adult,
                    begins_at: Tick(1_800),
                    maturation: NormalizedScalar(1.0),
                    target_brain_class_id: None,
                },
            ],
            critical_periods: vec![CriticalPeriod {
                lobe: LobeKind::CoreAssociation,
                opens_at: Tick(100),
                closes_at: Tick(1_200),
                plasticity_bias: NormalizedScalar(0.8),
            }],
            consolidation_cadence_ticks: 900,
            sleep_pressure_maturation_gate: NormalizedScalar(0.25),
        };
        schedule.validate_contract()?;
        Ok(schedule)
    }
}

impl Validate for DevelopmentalSchedule {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let mut previous_tick = Tick(0);
        let mut previous_maturation = 0.0;
        for (index, milestone) in self.milestones.iter().enumerate() {
            if index > 0 {
                Tick::validate_monotonic(previous_tick, milestone.begins_at)?;
            }
            milestone.validate_contract()?;
            let maturation = milestone.maturation.raw();
            if maturation < previous_maturation {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
            previous_tick = milestone.begins_at;
            previous_maturation = maturation;
        }
        validate_all(&self.critical_periods)?;
        if self.consolidation_cadence_ticks == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        validate_normalized(self.sleep_pressure_maturation_gate)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentalMilestone {
    pub stage: DevelopmentStage,
    pub begins_at: Tick,
    pub maturation: NormalizedScalar,
    pub target_brain_class_id: Option<BrainClassId>,
}

impl Validate for DevelopmentalMilestone {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_normalized(self.maturation)?;
        if let Some(brain_class_id) = self.target_brain_class_id {
            brain_class_id.validate()?;
            validate_known_brain_class(brain_class_id)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CriticalPeriod {
    pub lobe: LobeKind,
    pub opens_at: Tick,
    pub closes_at: Tick,
    pub plasticity_bias: NormalizedScalar,
}

impl Validate for CriticalPeriod {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        Tick::validate_monotonic(self.opens_at, self.closes_at)?;
        validate_normalized(self.plasticity_bias)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DevelopmentState {
    pub genome_id: GenomeId,
    pub age_ticks: Tick,
    pub maturation: NormalizedScalar,
    pub enabled_lobes: Vec<LobeKind>,
    pub active_sensor_channels: Vec<SensorChannelKind>,
    pub active_motor_affordances: Vec<MotorAffordanceKind>,
    pub open_critical_periods: Vec<CriticalPeriod>,
    pub sleep_cycle_count: u32,
    pub consolidation_cycle_count: u32,
    pub last_sleep_tick: Option<Tick>,
}

impl DevelopmentState {
    pub fn new(genome_id: GenomeId, age_ticks: Tick, maturation: NormalizedScalar) -> Self {
        Self {
            genome_id,
            age_ticks,
            maturation,
            enabled_lobes: Vec::new(),
            active_sensor_channels: Vec::new(),
            active_motor_affordances: Vec::new(),
            open_critical_periods: Vec::new(),
            sleep_cycle_count: 0,
            consolidation_cycle_count: 0,
            last_sleep_tick: None,
        }
    }

    pub fn with_enabled_lobes<I>(mut self, lobes: I) -> Self
    where
        I: IntoIterator<Item = LobeKind>,
    {
        self.enabled_lobes = lobes.into_iter().collect();
        self
    }
}

impl Validate for DevelopmentState {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.genome_id.validate()?;
        validate_normalized(self.maturation)?;
        validate_all(&self.open_critical_periods)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InheritancePolicy {
    pub inherited_deja_vu_enabled: bool,
    pub species_culture_priors_enabled: bool,
    pub lamarckian_weights_enabled: bool,
    pub inherit_lifetime_consolidation: bool,
}

impl Validate for InheritancePolicy {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.inherit_lifetime_consolidation && !self.lamarckian_weights_enabled {
            return Err(ScaffoldContractError::LamarckianInheritanceRequiresOptIn);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightLayerKind {
    WGeneticFixed,
    WLifetimeConsolidated,
    HOperational,
    HShadow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightStorageSemantics {
    SharedSparseSpeciesTemplate,
    PerCreatureSparseLifetime,
    PerCreatureOperationalTrace,
    PerCreatureShadowTrace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeightLayerDescriptor {
    pub layer: WeightLayerKind,
    pub brain_class_id: BrainClassId,
    pub max_active_synapses: u32,
    pub storage: WeightStorageSemantics,
    pub mutable_during_lifetime: bool,
    pub shared_species_template: bool,
}

impl WeightLayerDescriptor {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.brain_class_id.validate()?;
        validate_known_brain_class(self.brain_class_id)?;
        if self.max_active_synapses == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WGeneticFixed {
    pub descriptor: WeightLayerDescriptor,
    pub template_seed: u64,
}

impl WGeneticFixed {
    pub fn new(
        brain_class_id: BrainClassId,
        max_active_synapses: u32,
        template_seed: u64,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            descriptor: WeightLayerDescriptor {
                layer: WeightLayerKind::WGeneticFixed,
                brain_class_id,
                max_active_synapses,
                storage: WeightStorageSemantics::SharedSparseSpeciesTemplate,
                mutable_during_lifetime: false,
                shared_species_template: true,
            },
            template_seed,
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub const fn mutable_during_lifetime(&self) -> bool {
        self.descriptor.mutable_during_lifetime
    }
}

impl Validate for WGeneticFixed {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.descriptor.validate_contract()?;
        if self.template_seed == 0 || self.descriptor.mutable_during_lifetime {
            return Err(ScaffoldContractError::InvalidId);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WLifetimeConsolidated {
    pub descriptor: WeightLayerDescriptor,
    pub consolidation_events: u64,
    pub last_l2_norm_delta: f32,
}

impl WLifetimeConsolidated {
    pub fn new(
        brain_class_id: BrainClassId,
        max_active_synapses: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            descriptor: WeightLayerDescriptor {
                layer: WeightLayerKind::WLifetimeConsolidated,
                brain_class_id,
                max_active_synapses,
                storage: WeightStorageSemantics::PerCreatureSparseLifetime,
                mutable_during_lifetime: true,
                shared_species_template: false,
            },
            consolidation_events: 0,
            last_l2_norm_delta: 0.0,
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub fn record_consolidation(
        &mut self,
        delta: LifetimeConsolidationDelta,
    ) -> Result<(), ScaffoldContractError> {
        delta.validate_contract()?;
        self.consolidation_events = self.consolidation_events.saturating_add(1);
        self.last_l2_norm_delta = delta.l2_norm_delta;
        Ok(())
    }
}

impl Validate for WLifetimeConsolidated {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.descriptor.validate_contract()?;
        validate_finite(self.last_l2_norm_delta)?;
        if self.last_l2_norm_delta < 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HOperational {
    pub descriptor: WeightLayerDescriptor,
}

impl HOperational {
    pub fn new(
        brain_class_id: BrainClassId,
        max_active_synapses: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            descriptor: WeightLayerDescriptor {
                layer: WeightLayerKind::HOperational,
                brain_class_id,
                max_active_synapses,
                storage: WeightStorageSemantics::PerCreatureOperationalTrace,
                mutable_during_lifetime: true,
                shared_species_template: false,
            },
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for HOperational {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.descriptor.validate_contract()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HShadow {
    pub descriptor: WeightLayerDescriptor,
}

impl HShadow {
    pub fn new(
        brain_class_id: BrainClassId,
        max_active_synapses: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            descriptor: WeightLayerDescriptor {
                layer: WeightLayerKind::HShadow,
                brain_class_id,
                max_active_synapses,
                storage: WeightStorageSemantics::PerCreatureShadowTrace,
                mutable_during_lifetime: true,
                shared_species_template: false,
            },
        };
        value.validate_contract()?;
        Ok(value)
    }
}

impl Validate for HShadow {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.descriptor.validate_contract()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightSplitContract {
    pub genetic_fixed: WGeneticFixed,
    pub lifetime_consolidated: WLifetimeConsolidated,
    pub alpha_mask: AlphaMask,
    pub h_operational: HOperational,
    pub h_shadow: HShadow,
    pub max_active_tiles: u32,
}

impl WeightSplitContract {
    pub fn for_brain_class(
        brain_class_id: BrainClassId,
        max_active_synapses: u32,
        max_active_tiles: u32,
        template_seed: u64,
    ) -> Result<Self, ScaffoldContractError> {
        if max_active_tiles == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        let contract = Self {
            genetic_fixed: WGeneticFixed::new(brain_class_id, max_active_synapses, template_seed)?,
            lifetime_consolidated: WLifetimeConsolidated::new(brain_class_id, max_active_synapses)?,
            alpha_mask: AlphaMask::default_for_projection(NormalizedScalar(0.25)),
            h_operational: HOperational::new(brain_class_id, max_active_synapses)?,
            h_shadow: HShadow::new(brain_class_id, max_active_synapses)?,
            max_active_tiles,
        };
        contract.validate_contract()?;
        Ok(contract)
    }

    pub fn consolidate_lifetime(
        &mut self,
        delta: LifetimeConsolidationDelta,
    ) -> Result<(), ScaffoldContractError> {
        self.lifetime_consolidated.record_consolidation(delta)
    }
}

impl Validate for WeightSplitContract {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.genetic_fixed.validate_contract()?;
        self.lifetime_consolidated.validate_contract()?;
        self.alpha_mask.validate_contract()?;
        self.h_operational.validate_contract()?;
        self.h_shadow.validate_contract()?;
        if self.max_active_tiles == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LifetimeConsolidationDelta {
    pub consolidated_synapses: u32,
    pub l2_norm_delta: f32,
}

impl Validate for LifetimeConsolidationDelta {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        validate_finite(self.l2_norm_delta)?;
        if self.l2_norm_delta < 0.0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EffectiveWeightSample {
    pub genetic_fixed: f32,
    pub lifetime_consolidated: f32,
    pub alpha: NormalizedScalar,
    pub h_operational: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WEffective {
    pub value: f32,
}

impl WEffective {
    pub fn from_components(sample: EffectiveWeightSample) -> Result<Self, ScaffoldContractError> {
        validate_finite(sample.genetic_fixed)?;
        validate_finite(sample.lifetime_consolidated)?;
        validate_normalized(sample.alpha)?;
        validate_finite(sample.h_operational)?;
        let value = sample.genetic_fixed
            + sample.lifetime_consolidated
            + sample.alpha.raw() * sample.h_operational;
        validate_finite(value)?;
        Ok(Self { value })
    }
}

fn validate_all<T: Validate>(values: &[T]) -> Result<(), ScaffoldContractError> {
    for value in values {
        value.validate_contract()?;
    }
    Ok(())
}

fn validate_normalized(value: NormalizedScalar) -> Result<(), ScaffoldContractError> {
    NormalizedScalar::new(value.raw()).map(|_| ())
}

fn validate_known_brain_class(id: BrainClassId) -> Result<(), ScaffoldContractError> {
    if is_known_brain_class_id(id) {
        Ok(())
    } else {
        Err(ScaffoldContractError::UnknownBrainClass)
    }
}

fn is_known_brain_class_id(id: BrainClassId) -> bool {
    [
        BrainScaleTier::Nano512.default_class_id(),
        BrainScaleTier::Small1024.default_class_id(),
        BrainScaleTier::Standard2048.default_class_id(),
        BrainScaleTier::Large4096.default_class_id(),
        BrainScaleTier::Cognitive32768.default_class_id(),
        BrainScaleTier::Student131k.default_class_id(),
        BrainScaleTier::Ascended1M.default_class_id(),
        BrainScaleTier::Ascended5M.default_class_id(),
    ]
    .contains(&id)
}

fn derive_seed(species_seed: u64, brain_class_id: BrainClassId, salt: u64) -> u64 {
    let mut value = species_seed ^ ((u64::from(brain_class_id.0)) << 32) ^ salt;
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    if value == 0 {
        1
    } else {
        value
    }
}
