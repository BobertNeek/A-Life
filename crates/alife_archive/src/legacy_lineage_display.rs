use std::collections::HashSet;

use alife_core::{
    AlphaStoragePolicy, DevelopmentStage, DriveThresholdKind, EndocrineConstantKind,
    LegacyLobeKindV1, MotorAffordanceKind, SensorChannelKind,
};
use serde::Deserialize;

const MAX_DISPLAY_BYTES: usize = 64 * 1024;
const MAX_PARENTS: usize = 16;
const MAX_LOBES: usize = 17;
const MAX_PROJECTIONS: usize = 64;
const MAX_CHANNELS: usize = 32;
const MAX_GENES: usize = 32;
const MAX_MILESTONES: usize = 16;
const MAX_SYNAPSE_OVERRIDES: usize = 4_096;
const NANO512_NEURON_COUNT: u32 = 512;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyBrainGenomeV1DisplayWire {
    schema_version: u16,
    id: u64,
    parent_genome_ids: Vec<u64>,
    lineage_id: Option<u64>,
    species_seed: u64,
    brain_class_id: u16,
    genetic_prior_seed: u64,
    seeds: LegacyGenomeSeedSetV1,
    lobe_ratios: LegacyLobeRatioPlanV1,
    macro_connectome_masks: Vec<LegacyMacroConnectomeMaskV1>,
    sparse_density_priors: Vec<LegacySparseDensityPriorV1>,
    alpha_mask: LegacyAlphaMaskV1,
    plasticity_mask: LegacyPlasticityMaskV1,
    plasticity_parameters: LegacyPlasticityParametersV1,
    cognitive_architecture: LegacyCognitiveArchitectureV1,
    endocrine_constants: Vec<LegacyEndocrineConstantGeneV1>,
    drive_thresholds: Vec<LegacyDriveThresholdGeneV1>,
    sensor_layout: LegacySensorLayoutV1,
    motor_affordances: Vec<LegacyMotorAffordanceGeneV1>,
    mutation_rates: LegacyMutationRatesV1,
    crossover: LegacyCrossoverPolicyV1,
    developmental_schedule: LegacyDevelopmentalScheduleV1,
    inheritance: LegacyInheritancePolicyV1,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyGenomeSeedSetV1 {
    species_seed: u64,
    genome_id_seed: u64,
    genetic_prior_seed: u64,
    mutation_seed: u64,
    crossover_seed: u64,
    development_seed: u64,
    sensor_layout_seed: u64,
}

#[derive(Debug, Deserialize)]
enum LegacyLobeRatioPlanV1 {
    ClassDefault,
    RegistryRef(LegacyLobeRatioRegistryRefV1),
    InlineOverrides(Vec<LegacyLobeRatioOverrideV1>),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyLobeRatioRegistryRefV1 {
    registry_id: u64,
    version: u16,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyLobeRatioOverrideV1 {
    lobe: LegacyLobeKindV1,
    ratio: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyProjectionKeyV1 {
    source_lobe: LegacyLobeKindV1,
    target_lobe: LegacyLobeKindV1,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyMacroConnectomeMaskV1 {
    projection: LegacyProjectionKeyV1,
    enabled: bool,
    structural_growth_allowed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySparseDensityPriorV1 {
    projection: LegacyProjectionKeyV1,
    density: f32,
    max_active_synapse_share: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyAlphaMaskV1 {
    storage_policy: AlphaStoragePolicy,
    default_alpha: f32,
    projection_overrides: Vec<LegacyProjectionAlphaOverrideV1>,
    lobe_overrides: Vec<LegacyLobeAlphaOverrideV1>,
    tile_overrides: Vec<LegacyTileAlphaOverrideV1>,
    per_synapse_overrides: Vec<LegacySynapseAlphaOverrideV1>,
    dense_reference_opt_in: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyProjectionAlphaOverrideV1 {
    projection: LegacyProjectionKeyV1,
    alpha: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyLobeAlphaOverrideV1 {
    lobe: LegacyLobeKindV1,
    alpha: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTileAlphaOverrideV1 {
    tile: LegacyTileAddressV1,
    alpha: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyTileAddressV1 {
    lobe: LegacyLobeKindV1,
    tile_index: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySynapseAlphaOverrideV1 {
    synapse: LegacySynapseAddressV1,
    alpha: f32,
    exceptional_reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySynapseAddressV1 {
    source: u32,
    target: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPlasticityMaskV1 {
    oja_enabled: bool,
    hebbian_enabled: bool,
    projection_masks: Vec<LegacyProjectionPlasticityMaskV1>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyProjectionPlasticityMaskV1 {
    projection: LegacyProjectionKeyV1,
    learning_rate_scale: f32,
    plasticity_enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyPlasticityParametersV1 {
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCognitiveArchitectureV1 {
    schema_version: u16,
    attention_capacity: u8,
    active_concept_limit: u16,
    active_gap_limit: u8,
    predictor_capacity: u16,
    predictor_learning_rate: f32,
    motor_head_count: u8,
    motor_head_width: u16,
    dendritic_branch_capacity: u16,
    structural_candidate_budget: u16,
    structural_edit_budget: u8,
    sleep_trigger_threshold: f32,
    sleep_replay_rate: f32,
    sleep_consolidation_rate: f32,
    attention_learning_rate: f32,
    concept_learning_rate: f32,
    motor_learning_rate: f32,
    structural_learning_rate: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyEndocrineConstantGeneV1 {
    kind: EndocrineConstantKind,
    value: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyDriveThresholdGeneV1 {
    kind: DriveThresholdKind,
    threshold: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySensorLayoutV1 {
    channels: Vec<LegacySensorChannelGeneV1>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySensorChannelGeneV1 {
    kind: SensorChannelKind,
    receptor_count: u16,
    target_lobe: LegacyLobeKindV1,
    enabled_at_maturation: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyMotorAffordanceGeneV1 {
    kind: MotorAffordanceKind,
    enabled: bool,
    motor_lobe_units: u16,
    enabled_at_maturation: u8,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyMutationRatesV1 {
    point: f32,
    structural: f32,
    lobe_ratio: f32,
    density: f32,
    alpha: f32,
    endocrine: f32,
    developmental_schedule: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCrossoverPolicyV1 {
    enabled: bool,
    max_segments: u8,
    parent_mix_bias: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyDevelopmentalScheduleV1 {
    milestones: Vec<LegacyDevelopmentalMilestoneV1>,
    critical_periods: Vec<LegacyCriticalPeriodV1>,
    consolidation_cadence_ticks: u32,
    sleep_pressure_maturation_gate: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyDevelopmentalMilestoneV1 {
    stage: DevelopmentStage,
    begins_at: u64,
    maturation: f32,
    target_brain_class_id: Option<u16>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCriticalPeriodV1 {
    lobe: LegacyLobeKindV1,
    opens_at: u64,
    closes_at: u64,
    plasticity_bias: f32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyInheritancePolicyV1 {
    inherited_deja_vu_enabled: bool,
    species_culture_priors_enabled: bool,
    lamarckian_weights_enabled: bool,
    inherit_lifetime_consolidation: bool,
}

pub(crate) fn validate_legacy_nano512_genome_v1(
    bytes: &[u8],
    expected_genome_id: u64,
    expected_brain_class_id: u16,
    expected_lineage_id: Option<u64>,
) -> Result<(), String> {
    if bytes.len() > MAX_DISPLAY_BYTES {
        return Err("legacy lineage genome exceeds the display-only byte bound".to_string());
    }
    let genome: LegacyBrainGenomeV1DisplayWire =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if genome.schema_version != 1
        || genome.id == 0
        || genome.id != expected_genome_id
        || genome.brain_class_id != 1
        || genome.brain_class_id != expected_brain_class_id
        || genome.lineage_id != expected_lineage_id
        || genome.lineage_id == Some(0)
        || genome.species_seed == 0
        || genome.genetic_prior_seed == 0
        || genome.parent_genome_ids.len() > MAX_PARENTS
        || genome.parent_genome_ids.contains(&0)
    {
        return Err("legacy lineage genome identity or class is invalid".to_string());
    }
    genome.seeds.validate(&genome)?;
    genome.lobe_ratios.validate()?;
    validate_connectome(
        &genome.macro_connectome_masks,
        &genome.sparse_density_priors,
    )?;
    genome.alpha_mask.validate()?;
    genome.plasticity_mask.validate()?;
    genome.plasticity_parameters.validate()?;
    genome.cognitive_architecture.validate()?;
    validate_bounded_unique_genes(
        &genome.endocrine_constants,
        MAX_GENES,
        |gene| gene.kind,
        |gene| finite(gene.value),
    )?;
    validate_bounded_unique_genes(
        &genome.drive_thresholds,
        MAX_GENES,
        |gene| gene.kind,
        |gene| normalized(gene.threshold),
    )?;
    genome.sensor_layout.validate()?;
    validate_bounded_unique_genes(
        &genome.motor_affordances,
        MAX_GENES,
        |gene| gene.kind,
        |gene| {
            (!gene.enabled || gene.motor_lobe_units > 0)
                && gene.motor_lobe_units <= NANO512_NEURON_COUNT as u16
                && gene.enabled_at_maturation <= 100
        },
    )?;
    genome.mutation_rates.validate()?;
    genome.crossover.validate()?;
    genome.developmental_schedule.validate()?;
    genome.inheritance.validate()?;
    Ok(())
}

impl LegacyGenomeSeedSetV1 {
    fn validate(&self, genome: &LegacyBrainGenomeV1DisplayWire) -> Result<(), String> {
        let seeds = [
            self.species_seed,
            self.genome_id_seed,
            self.genetic_prior_seed,
            self.mutation_seed,
            self.crossover_seed,
            self.development_seed,
            self.sensor_layout_seed,
        ];
        if seeds.contains(&0)
            || self.species_seed != genome.species_seed
            || self.genetic_prior_seed != genome.genetic_prior_seed
        {
            return Err("legacy lineage genome seed contract is invalid".to_string());
        }
        Ok(())
    }
}

impl LegacyLobeRatioPlanV1 {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::ClassDefault => Ok(()),
            Self::RegistryRef(reference) => {
                if reference.registry_id == 0 || reference.version == 0 {
                    Err("legacy lobe ratio registry reference is invalid".to_string())
                } else {
                    Ok(())
                }
            }
            Self::InlineOverrides(overrides) => {
                if overrides.len() > MAX_LOBES
                    || !all_unique(overrides.iter().map(|entry| entry.lobe.raw()))
                    || overrides.iter().any(|entry| !normalized(entry.ratio))
                {
                    Err("legacy lobe ratio overrides are invalid".to_string())
                } else {
                    Ok(())
                }
            }
        }
    }
}

fn validate_connectome(
    masks: &[LegacyMacroConnectomeMaskV1],
    densities: &[LegacySparseDensityPriorV1],
) -> Result<(), String> {
    if masks.is_empty()
        || masks.len() > MAX_PROJECTIONS
        || densities.len() > MAX_PROJECTIONS
        || !all_unique(masks.iter().map(|mask| projection_raw(mask.projection)))
        || !all_unique(
            densities
                .iter()
                .map(|density| projection_raw(density.projection)),
        )
        || masks.iter().any(|mask| {
            mask.structural_growth_allowed || !is_legacy_slice_a_projection(mask.projection)
        })
        || densities.iter().any(|density| {
            !normalized(density.density) || !normalized(density.max_active_synapse_share)
        })
    {
        return Err("legacy connectome metadata is invalid".to_string());
    }
    for mask in masks {
        let count = densities
            .iter()
            .filter(|density| density.projection == mask.projection)
            .count();
        if (mask.enabled && count != 1) || (!mask.enabled && count != 0) {
            return Err("legacy connectome density contract is invalid".to_string());
        }
    }
    if densities.iter().any(|density| {
        !masks
            .iter()
            .any(|mask| mask.enabled && mask.projection == density.projection)
    }) {
        return Err("legacy connectome density has no enabled projection".to_string());
    }
    Ok(())
}

fn is_legacy_slice_a_projection(key: LegacyProjectionKeyV1) -> bool {
    use LegacyLobeKindV1::{
        AuditorySpeech, CoreAssociation, EpisodicMemory, GlyphVision, HomeostaticRegulation,
        LexiconConcept, MetabolicDrive, MotorArbitration, SensoryGrounding, WorkingMemory,
    };
    matches!(
        (key.source_lobe, key.target_lobe),
        (SensoryGrounding, CoreAssociation)
            | (CoreAssociation, MotorArbitration)
            | (MetabolicDrive, HomeostaticRegulation)
            | (MotorArbitration, MotorArbitration)
            | (AuditorySpeech, CoreAssociation)
            | (GlyphVision, CoreAssociation)
            | (HomeostaticRegulation, CoreAssociation)
            | (HomeostaticRegulation, MotorArbitration)
            | (CoreAssociation, WorkingMemory)
            | (WorkingMemory, CoreAssociation)
            | (CoreAssociation, EpisodicMemory)
            | (EpisodicMemory, CoreAssociation)
            | (CoreAssociation, LexiconConcept)
            | (LexiconConcept, CoreAssociation)
            | (LexiconConcept, WorkingMemory)
            | (WorkingMemory, LexiconConcept)
    )
}

impl LegacyAlphaMaskV1 {
    fn validate(&self) -> Result<(), String> {
        if (self.storage_policy == AlphaStoragePolicy::DenseDebugReference
            && !self.dense_reference_opt_in)
            || !normalized(self.default_alpha)
            || self.projection_overrides.len() > MAX_PROJECTIONS
            || self.lobe_overrides.len() > MAX_LOBES
            || self.tile_overrides.len() > MAX_PROJECTIONS
            || self.per_synapse_overrides.len() > MAX_SYNAPSE_OVERRIDES
            || self.projection_overrides.iter().any(|entry| {
                !normalized(entry.alpha) || !is_legacy_slice_a_projection(entry.projection)
            })
            || self.lobe_overrides.iter().any(|entry| {
                let _ = entry.lobe.raw();
                !normalized(entry.alpha)
            })
            || self.tile_overrides.iter().any(|entry| {
                let _ = entry.tile.lobe.raw();
                !normalized(entry.alpha) || entry.tile.tile_index >= NANO512_NEURON_COUNT
            })
            || self.per_synapse_overrides.iter().any(|entry| {
                !normalized(entry.alpha)
                    || entry.synapse.source >= NANO512_NEURON_COUNT
                    || entry.synapse.target >= NANO512_NEURON_COUNT
                    || entry.exceptional_reason.trim().is_empty()
                    || entry.exceptional_reason.len() > 256
            })
        {
            return Err("legacy alpha mask is invalid".to_string());
        }
        Ok(())
    }
}

impl LegacyPlasticityMaskV1 {
    fn validate(&self) -> Result<(), String> {
        let _ = (self.oja_enabled, self.hebbian_enabled);
        if self.projection_masks.len() > MAX_PROJECTIONS
            || !all_unique(
                self.projection_masks
                    .iter()
                    .map(|entry| projection_raw(entry.projection)),
            )
            || self.projection_masks.iter().any(|entry| {
                let _ = entry.plasticity_enabled;
                !is_legacy_slice_a_projection(entry.projection)
                    || !normalized(entry.learning_rate_scale)
            })
        {
            return Err("legacy plasticity mask is invalid".to_string());
        }
        Ok(())
    }
}

impl LegacyPlasticityParametersV1 {
    fn validate(&self) -> Result<(), String> {
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
        if self.schema_version != 1
            || values.iter().any(|value| !value.is_finite())
            || !normalized(self.eligibility_decay)
            || !normalized(self.base_learning_rate)
            || self.base_learning_rate == 0.0
            || !normalized(self.normalization_rate)
            || !normalized(self.sleep_replay_rate)
            || !matches!(self.modulator_sign, -1.0 | 1.0)
            || !(-8.0..=8.0).contains(&self.fast_min)
            || !(-8.0..=8.0).contains(&self.fast_max)
            || self.fast_min >= self.fast_max
            || !normalized(self.sleep_staging_rate)
            || self.sleep_staging_rate == 0.0
            || !(0.0..=8.0).contains(&self.sleep_weight_limit)
            || self.sleep_weight_limit == 0.0
            || !normalized(self.sleep_fast_decay_rate)
        {
            return Err("legacy plasticity parameters are invalid".to_string());
        }
        Ok(())
    }
}

impl LegacyCognitiveArchitectureV1 {
    fn validate(&self) -> Result<(), String> {
        let rates = [
            self.predictor_learning_rate,
            self.sleep_trigger_threshold,
            self.sleep_replay_rate,
            self.sleep_consolidation_rate,
            self.attention_learning_rate,
            self.concept_learning_rate,
            self.motor_learning_rate,
            self.structural_learning_rate,
        ];
        if self.schema_version != 1
            || self.attention_capacity == 0
            || self.attention_capacity > 16
            || self.active_concept_limit == 0
            || self.active_concept_limit > 512
            || self.active_gap_limit == 0
            || self.active_gap_limit > 64
            || self.predictor_capacity < 8
            || self.predictor_capacity > 512
            || self.motor_head_count == 0
            || self.motor_head_count > 16
            || self.motor_head_width == 0
            || self.motor_head_width > 256
            || self.dendritic_branch_capacity == 0
            || self.dendritic_branch_capacity > 1_024
            || self.structural_candidate_budget == 0
            || self.structural_candidate_budget > 512
            || self.structural_edit_budget == 0
            || self.structural_edit_budget > 64
            || rates.iter().any(|rate| !normalized(*rate))
        {
            return Err("legacy cognitive architecture is invalid".to_string());
        }
        Ok(())
    }
}

impl LegacySensorLayoutV1 {
    fn validate(&self) -> Result<(), String> {
        if self.channels.is_empty()
            || self.channels.len() > MAX_CHANNELS
            || !all_unique(self.channels.iter().map(|channel| channel.kind))
            || self.channels.iter().any(|channel| {
                let _ = channel.target_lobe.raw();
                channel.receptor_count == 0
                    || channel.receptor_count > NANO512_NEURON_COUNT as u16
                    || channel.enabled_at_maturation > 100
            })
        {
            return Err("legacy sensor layout is invalid".to_string());
        }
        Ok(())
    }
}

impl LegacyMutationRatesV1 {
    fn validate(&self) -> Result<(), String> {
        if [
            self.point,
            self.structural,
            self.lobe_ratio,
            self.density,
            self.alpha,
            self.endocrine,
            self.developmental_schedule,
        ]
        .iter()
        .any(|rate| !normalized(*rate))
        {
            return Err("legacy mutation rates are invalid".to_string());
        }
        Ok(())
    }
}

impl LegacyCrossoverPolicyV1 {
    fn validate(&self) -> Result<(), String> {
        if !normalized(self.parent_mix_bias) || (self.enabled && self.max_segments == 0) {
            return Err("legacy crossover policy is invalid".to_string());
        }
        Ok(())
    }
}

impl LegacyDevelopmentalScheduleV1 {
    fn validate(&self) -> Result<(), String> {
        if self.milestones.is_empty()
            || self.milestones.len() > MAX_MILESTONES
            || self.critical_periods.len() > MAX_LOBES
            || self.consolidation_cadence_ticks == 0
            || !normalized(self.sleep_pressure_maturation_gate)
        {
            return Err("legacy developmental schedule bounds are invalid".to_string());
        }
        let mut previous_tick = 0;
        let mut previous_maturation = 0.0;
        for (index, milestone) in self.milestones.iter().enumerate() {
            let _ = milestone.stage;
            if (index > 0 && milestone.begins_at <= previous_tick)
                || !normalized(milestone.maturation)
                || milestone.maturation < previous_maturation
                || milestone
                    .target_brain_class_id
                    .is_some_and(|class_id| class_id != 1)
            {
                return Err("legacy developmental milestone is invalid".to_string());
            }
            previous_tick = milestone.begins_at;
            previous_maturation = milestone.maturation;
        }
        if self.critical_periods.iter().any(|period| {
            let _ = period.lobe;
            period.opens_at >= period.closes_at || !normalized(period.plasticity_bias)
        }) {
            return Err("legacy critical period is invalid".to_string());
        }
        Ok(())
    }
}

impl LegacyInheritancePolicyV1 {
    fn validate(&self) -> Result<(), String> {
        let _ = (
            self.inherited_deja_vu_enabled,
            self.species_culture_priors_enabled,
        );
        if self.inherit_lifetime_consolidation && !self.lamarckian_weights_enabled {
            return Err("legacy inheritance policy is invalid".to_string());
        }
        Ok(())
    }
}

fn validate_bounded_unique_genes<T, K, Key, Check>(
    values: &[T],
    max: usize,
    key: Key,
    check: Check,
) -> Result<(), String>
where
    K: Eq + std::hash::Hash,
    Key: Fn(&T) -> K,
    Check: Fn(&T) -> bool,
{
    if values.len() > max || !all_unique(values.iter().map(key)) || values.iter().any(|v| !check(v))
    {
        return Err("legacy genome gene collection is invalid".to_string());
    }
    Ok(())
}

fn all_unique<T>(values: impl IntoIterator<Item = T>) -> bool
where
    T: Eq + std::hash::Hash,
{
    let mut seen = HashSet::new();
    values.into_iter().all(|value| seen.insert(value))
}

fn projection_raw(projection: LegacyProjectionKeyV1) -> (u16, u16) {
    (projection.source_lobe.raw(), projection.target_lobe.raw())
}

fn finite(value: f32) -> bool {
    value.is_finite()
}

fn normalized(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}
