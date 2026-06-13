//! v0 tooling: deterministic offline evolution, breeding, and genome-lab helpers.
//!
//! This module is intentionally outside active gameplay. It mutates and crosses
//! `alife_core` genome contracts for offline experiments, derives fitness from
//! packed logs, and records lineage metadata without inheriting lifetime state
//! into `W_genetic_fixed`.

use std::cmp::Ordering;

use alife_core::{
    AlphaStoragePolicy, BrainClassSpec, BrainGenome, CriticalPeriod, DevelopmentalMilestone,
    DriveThresholdGene, EndocrineConstantGene, GenomeId, GenomeSeedSet, LobeRatioOverride,
    LobeRatioPlan, MacroConnectomeMask, MotorAffordanceGene, MutationRates, NormalizedScalar,
    PackedExperienceRecord, PackedSideBufferKind, ProjectionAlphaOverride, SensorChannelGene,
    SparseDensityPrior, Tick, Validate,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const EVOLUTION_LAB_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Error)]
pub enum EvolutionLabError {
    #[error("contract violation: {0}")]
    Contract(#[from] alife_core::ScaffoldContractError),
    #[error("incompatible parent genomes")]
    IncompatibleParentGenomes,
    #[error("selection lab requires at least one candidate")]
    EmptyPopulation,
    #[error("selection lab configuration is invalid")]
    InvalidConfig,
    #[error("generated weight initializer must be birth-only")]
    WeightInitializerNotBirthOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationField {
    LobeRatios,
    MacroConnectomeMasks,
    SparseDensityPriors,
    AlphaMask,
    EndocrineConstants,
    DriveThresholds,
    SensorLayout,
    MotorAffordances,
    MutationRates,
    DevelopmentalSchedule,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MutationConfig {
    pub seed: u64,
    pub generation: u32,
    pub intensity: NormalizedScalar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutationOutcome {
    pub child: BrainGenome,
    pub generation: u32,
    pub random_seed: u64,
    pub touched_fields: Vec<MutationField>,
}

pub fn mutate_genome(
    parent: &BrainGenome,
    spec: &BrainClassSpec,
    config: MutationConfig,
) -> Result<MutationOutcome, EvolutionLabError> {
    parent.validate_contract()?;
    spec.validate()?;
    if parent.brain_class_id != spec.id || config.seed == 0 {
        return Err(EvolutionLabError::InvalidConfig);
    }

    let mut rng = LabRng::new(config.seed ^ u64::from(config.generation));
    let mut child = parent.clone();
    let species_seed = nonzero(rng.next_u64());
    child.species_seed = species_seed;
    child.seeds = GenomeSeedSet::from_species_seed(species_seed, spec.id);
    child.id = GenomeId(child.seeds.genome_id_seed);
    child.genetic_prior_seed = child.seeds.genetic_prior_seed;
    child.parent_genome_ids = vec![parent.id];
    child.brain_class_id = spec.id;
    child.lobe_ratios = mutate_lobe_ratios(spec, &mut rng, config.intensity)?;
    mutate_macro_masks(&mut child.macro_connectome_masks, &mut rng);
    mutate_density_priors(&mut child.sparse_density_priors, &mut rng, config.intensity)?;
    mutate_alpha_mask(&mut child, &mut rng, config.intensity)?;
    mutate_endocrine(&mut child.endocrine_constants, &mut rng, config.intensity)?;
    mutate_drive_thresholds(&mut child.drive_thresholds, &mut rng, config.intensity)?;
    mutate_sensor_layout(&mut child.sensor_layout.channels, &mut rng);
    mutate_motor_affordances(&mut child.motor_affordances, &mut rng);
    child.mutation_rates = mutate_mutation_rates(child.mutation_rates, &mut rng, config.intensity)?;
    mutate_developmental_schedule(&mut child, &mut rng, config.intensity)?;
    child.inheritance.inherit_lifetime_consolidation = false;
    child.inheritance.lamarckian_weights_enabled = false;

    child.validate_contract()?;
    Ok(MutationOutcome {
        child,
        generation: config.generation,
        random_seed: config.seed,
        touched_fields: all_mutation_fields(),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossoverConfig {
    pub seed: u64,
    pub generation: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BirthWeightInitializerRef {
    pub asset_id: String,
    pub asset_schema_version: u16,
    pub birth_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossoverLineageRecord {
    pub schema_version: u16,
    pub child_genome_id: GenomeId,
    pub parent_genome_ids: Vec<GenomeId>,
    pub generation: u32,
    pub random_seed: u64,
    pub compatible: bool,
    pub birth_weight_initializer: Option<BirthWeightInitializerRef>,
    pub lifetime_state_inherited: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OffspringRecord {
    pub child: BrainGenome,
    pub lineage: CrossoverLineageRecord,
}

pub fn crossover_genomes(
    parent_a: &BrainGenome,
    parent_b: &BrainGenome,
    spec: &BrainClassSpec,
    config: CrossoverConfig,
) -> Result<OffspringRecord, EvolutionLabError> {
    crossover_genomes_with_initializer(parent_a, parent_b, spec, config, None)
}

fn crossover_genomes_with_initializer(
    parent_a: &BrainGenome,
    parent_b: &BrainGenome,
    spec: &BrainClassSpec,
    config: CrossoverConfig,
    birth_weight_initializer: Option<BirthWeightInitializerRef>,
) -> Result<OffspringRecord, EvolutionLabError> {
    parent_a.validate_contract()?;
    parent_b.validate_contract()?;
    spec.validate()?;
    if parent_a.brain_class_id != parent_b.brain_class_id
        || parent_a.brain_class_id != spec.id
        || parent_a.schema_version != parent_b.schema_version
        || config.seed == 0
    {
        return Err(EvolutionLabError::IncompatibleParentGenomes);
    }
    if birth_weight_initializer
        .as_ref()
        .is_some_and(|initializer| !initializer.birth_only || initializer.asset_schema_version == 0)
    {
        return Err(EvolutionLabError::WeightInitializerNotBirthOnly);
    }

    let mut rng = LabRng::new(config.seed ^ 0xC0DE_3300);
    let mut child = if rng.next_bool() {
        parent_a.clone()
    } else {
        parent_b.clone()
    };
    let species_seed = nonzero(rng.next_u64());
    child.species_seed = species_seed;
    child.seeds = GenomeSeedSet::from_species_seed(species_seed, spec.id);
    child.id = GenomeId(child.seeds.genome_id_seed);
    child.genetic_prior_seed = child.seeds.genetic_prior_seed;
    child.parent_genome_ids = vec![parent_a.id, parent_b.id];
    child.lineage_id = parent_a.lineage_id.or(parent_b.lineage_id);
    child.brain_class_id = spec.id;

    if rng.next_bool() {
        child.lobe_ratios = parent_a.lobe_ratios.clone();
        child.alpha_mask = parent_b.alpha_mask.clone();
        child.sensor_layout = parent_a.sensor_layout.clone();
        child.motor_affordances = parent_b.motor_affordances.clone();
    } else {
        child.lobe_ratios = parent_b.lobe_ratios.clone();
        child.alpha_mask = parent_a.alpha_mask.clone();
        child.sensor_layout = parent_b.sensor_layout.clone();
        child.motor_affordances = parent_a.motor_affordances.clone();
    }
    child.macro_connectome_masks = choose_vec(
        &mut rng,
        &parent_a.macro_connectome_masks,
        &parent_b.macro_connectome_masks,
    );
    child.sparse_density_priors = choose_vec(
        &mut rng,
        &parent_a.sparse_density_priors,
        &parent_b.sparse_density_priors,
    );
    child.endocrine_constants = choose_vec(
        &mut rng,
        &parent_a.endocrine_constants,
        &parent_b.endocrine_constants,
    );
    child.drive_thresholds = choose_vec(
        &mut rng,
        &parent_a.drive_thresholds,
        &parent_b.drive_thresholds,
    );
    child.mutation_rates = if rng.next_bool() {
        parent_a.mutation_rates
    } else {
        parent_b.mutation_rates
    };
    child.crossover = if rng.next_bool() {
        parent_a.crossover
    } else {
        parent_b.crossover
    };
    child.developmental_schedule = if rng.next_bool() {
        parent_a.developmental_schedule.clone()
    } else {
        parent_b.developmental_schedule.clone()
    };
    child.inheritance.inherit_lifetime_consolidation = false;
    child.inheritance.lamarckian_weights_enabled = false;
    child.validate_contract()?;

    Ok(OffspringRecord {
        lineage: CrossoverLineageRecord {
            schema_version: EVOLUTION_LAB_SCHEMA_VERSION,
            child_genome_id: child.id,
            parent_genome_ids: vec![parent_a.id, parent_b.id],
            generation: config.generation,
            random_seed: config.seed,
            compatible: true,
            birth_weight_initializer,
            lifetime_state_inherited: false,
        },
        child,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FitnessSummary {
    pub survival_ticks: u64,
    pub energy_stability: f32,
    pub food_success: f32,
    pub pain_avoidance: f32,
    pub curiosity_resolution: f32,
    pub social_word_task_score: f32,
    pub teacher_verifier_score: Option<f32>,
    pub composite_score: f32,
}

impl FitnessSummary {
    pub fn synthetic(score: f32) -> Result<Self, EvolutionLabError> {
        let score = unit(score)?;
        Ok(Self {
            survival_ticks: (score * 1_000.0).round() as u64,
            energy_stability: score,
            food_success: score,
            pain_avoidance: score,
            curiosity_resolution: score,
            social_word_task_score: score,
            teacher_verifier_score: None,
            composite_score: score,
        })
    }

    pub fn from_packed_records(
        records: &[PackedExperienceRecord],
    ) -> Result<Self, EvolutionLabError> {
        if records.is_empty() {
            return Err(EvolutionLabError::EmptyPopulation);
        }
        for record in records {
            record.validate_contract()?;
        }

        let min_tick = records
            .iter()
            .map(|record| record.frame.pre_action_tick)
            .min()
            .unwrap_or(0);
        let max_tick = records
            .iter()
            .map(|record| record.frame.outcome_tick)
            .max()
            .unwrap_or(min_tick);
        let survival_ticks = max_tick.saturating_sub(min_tick);
        let count = records.len() as f32;
        let avg_abs_energy = records
            .iter()
            .map(|record| record.frame.energy_delta.abs().min(1.0))
            .sum::<f32>()
            / count;
        let energy_stability = unit(1.0 - avg_abs_energy)?;
        let success_count = records
            .iter()
            .filter(|record| record.frame.flags & alife_core::PACKED_FLAG_SUCCESS != 0)
            .count() as f32;
        let food_success = unit(success_count / count)?;
        let pain_avoidance = unit(
            1.0 - records
                .iter()
                .map(|record| record.frame.pain_delta)
                .sum::<f32>()
                / count,
        )?;
        let curiosity_resolution = unit(
            1.0 - records
                .iter()
                .map(|record| record.frame.prediction_error)
                .sum::<f32>()
                / count,
        )?;
        let social_word_task_score = unit(
            records
                .iter()
                .map(social_word_signal)
                .sum::<Result<f32, EvolutionLabError>>()?
                / count,
        )?;
        let teacher_scores = records.iter().flat_map(teacher_scores).collect::<Vec<_>>();
        let teacher_verifier_score = if teacher_scores.is_empty() {
            None
        } else {
            Some(unit(
                teacher_scores.iter().copied().sum::<f32>() / teacher_scores.len() as f32,
            )?)
        };
        let teacher_component = teacher_verifier_score.unwrap_or(social_word_task_score);
        let composite_score = unit(
            0.15 * energy_stability
                + 0.20 * food_success
                + 0.20 * pain_avoidance
                + 0.15 * curiosity_resolution
                + 0.15 * social_word_task_score
                + 0.15 * teacher_component,
        )?;

        Ok(Self {
            survival_ticks,
            energy_stability,
            food_success,
            pain_avoidance,
            curiosity_resolution,
            social_word_task_score,
            teacher_verifier_score,
            composite_score,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionCandidate {
    pub genome: BrainGenome,
    pub fitness: FitnessSummary,
}

impl SelectionCandidate {
    pub const fn new(genome: BrainGenome, fitness: FitnessSummary) -> Self {
        Self { genome, fitness }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FitnessInput {
    pub genome: BrainGenome,
    pub packed_records: Vec<PackedExperienceRecord>,
}

impl FitnessInput {
    pub fn summarize(self) -> Result<SelectionCandidate, EvolutionLabError> {
        Ok(SelectionCandidate {
            genome: self.genome,
            fitness: FitnessSummary::from_packed_records(&self.packed_records)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionLabConfig {
    pub seed: u64,
    pub generation: u32,
    pub survivor_count: usize,
    pub offspring_count: usize,
    pub mutation_intensity: NormalizedScalar,
    pub birth_weight_initializer: Option<BirthWeightInitializerRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionGenerationReport {
    pub schema_version: u16,
    pub generation: u32,
    pub seed: u64,
    pub survivors: Vec<SelectionCandidate>,
    pub offspring: Vec<OffspringRecord>,
}

pub fn run_selection_generation(
    candidates: &[SelectionCandidate],
    spec: &BrainClassSpec,
    config: EvolutionLabConfig,
) -> Result<SelectionGenerationReport, EvolutionLabError> {
    if candidates.is_empty()
        || config.seed == 0
        || config.survivor_count == 0
        || config.offspring_count == 0
    {
        return Err(EvolutionLabError::InvalidConfig);
    }
    if config
        .birth_weight_initializer
        .as_ref()
        .is_some_and(|initializer| !initializer.birth_only || initializer.asset_schema_version == 0)
    {
        return Err(EvolutionLabError::WeightInitializerNotBirthOnly);
    }
    spec.validate()?;
    for candidate in candidates {
        candidate.genome.validate_contract()?;
        if candidate.genome.brain_class_id != spec.id {
            return Err(EvolutionLabError::IncompatibleParentGenomes);
        }
    }

    let mut ranked = candidates.to_vec();
    ranked.sort_by(compare_candidates);
    let survivors = ranked
        .iter()
        .take(config.survivor_count.min(ranked.len()))
        .cloned()
        .collect::<Vec<_>>();
    if survivors.is_empty() {
        return Err(EvolutionLabError::EmptyPopulation);
    }

    let mut offspring = Vec::with_capacity(config.offspring_count);
    for index in 0..config.offspring_count {
        let parent_a = &survivors[index % survivors.len()].genome;
        let parent_b = &survivors[(index + 1) % survivors.len()].genome;
        let child_seed = mix_seed(config.seed, index as u64, 0xBEEF_3300);
        let crossed = crossover_genomes_with_initializer(
            parent_a,
            parent_b,
            spec,
            CrossoverConfig {
                seed: child_seed,
                generation: config.generation,
            },
            config.birth_weight_initializer.clone(),
        )?;
        let mutated = mutate_genome(
            &crossed.child,
            spec,
            MutationConfig {
                seed: mix_seed(config.seed, index as u64, 0xFA11_3300),
                generation: config.generation,
                intensity: config.mutation_intensity,
            },
        )?;
        let mut lineage = crossed.lineage;
        lineage.child_genome_id = mutated.child.id;
        offspring.push(OffspringRecord {
            child: mutated.child,
            lineage,
        });
    }

    Ok(SelectionGenerationReport {
        schema_version: EVOLUTION_LAB_SCHEMA_VERSION,
        generation: config.generation,
        seed: config.seed,
        survivors,
        offspring,
    })
}

pub fn tiny_generation_smoke(
    seed: u64,
    generations: u32,
) -> Result<SelectionGenerationReport, EvolutionLabError> {
    let spec = BrainClassSpec::for_tier(alife_core::BrainScaleTier::Nano512);
    let mut candidates = vec![
        SelectionCandidate::new(
            BrainGenome::scaffold(seed, spec.id),
            FitnessSummary::synthetic(0.7)?,
        ),
        SelectionCandidate::new(
            BrainGenome::scaffold(seed.wrapping_add(1), spec.id),
            FitnessSummary::synthetic(0.9)?,
        ),
    ];
    let mut report = None;
    for generation in 1..=generations.max(1) {
        let next = run_selection_generation(
            &candidates,
            &spec,
            EvolutionLabConfig {
                seed: mix_seed(seed, u64::from(generation), 0x5330_0001),
                generation,
                survivor_count: 2,
                offspring_count: 2,
                mutation_intensity: NormalizedScalar(0.35),
                birth_weight_initializer: None,
            },
        )?;
        candidates = next
            .offspring
            .iter()
            .enumerate()
            .map(|(index, offspring)| {
                SelectionCandidate::new(
                    offspring.child.clone(),
                    FitnessSummary::synthetic(0.6 + index as f32 * 0.1).unwrap(),
                )
            })
            .collect();
        report = Some(next);
    }
    report.ok_or(EvolutionLabError::InvalidConfig)
}

fn mutate_lobe_ratios(
    spec: &BrainClassSpec,
    rng: &mut LabRng,
    intensity: NormalizedScalar,
) -> Result<LobeRatioPlan, EvolutionLabError> {
    let min_ratio = 16.0 / spec.neuron_count as f32;
    let mut weights = spec
        .lobe_regions()
        .filter(|region| region.enabled)
        .map(|region| {
            let base = region.len as f32 / spec.neuron_count as f32;
            let delta = rng.signed_unit() * 0.08 * intensity.raw();
            (region.kind, (base + delta).clamp(min_ratio, 0.60))
        })
        .collect::<Vec<_>>();
    normalize_lobe_weights(&mut weights, min_ratio);
    Ok(LobeRatioPlan::InlineOverrides(
        weights
            .into_iter()
            .map(|(lobe, ratio)| {
                Ok(LobeRatioOverride {
                    lobe,
                    ratio: NormalizedScalar::new(ratio)?,
                })
            })
            .collect::<Result<Vec<_>, alife_core::ScaffoldContractError>>()?,
    ))
}

fn normalize_lobe_weights(weights: &mut [(alife_core::LobeKind, f32)], min_ratio: f32) {
    let mut sum = weights.iter().map(|(_, ratio)| *ratio).sum::<f32>();
    if sum <= 0.0 {
        return;
    }
    for (_, ratio) in weights.iter_mut() {
        *ratio = (*ratio / sum).clamp(min_ratio, 0.60);
    }
    sum = weights.iter().map(|(_, ratio)| *ratio).sum::<f32>();
    if let Some((_, last)) = weights.last_mut() {
        *last = (*last + (1.0 - sum)).clamp(min_ratio, 0.60);
    }
}

fn mutate_macro_masks(masks: &mut [MacroConnectomeMask], rng: &mut LabRng) {
    for mask in masks {
        if rng.next_fraction() < 0.5 {
            mask.structural_growth_allowed = !mask.structural_growth_allowed;
        } else if mask.structural_growth_allowed {
            mask.enabled = true;
        }
    }
}

fn mutate_density_priors(
    priors: &mut [SparseDensityPrior],
    rng: &mut LabRng,
    intensity: NormalizedScalar,
) -> Result<(), EvolutionLabError> {
    for prior in priors {
        prior.density = bounded_delta(prior.density, rng, 0.05, intensity)?;
        prior.max_active_synapse_share =
            bounded_delta(prior.max_active_synapse_share, rng, 0.10, intensity)?;
    }
    Ok(())
}

fn mutate_alpha_mask(
    genome: &mut BrainGenome,
    rng: &mut LabRng,
    intensity: NormalizedScalar,
) -> Result<(), EvolutionLabError> {
    genome.alpha_mask.storage_policy = AlphaStoragePolicy::HierarchicalSparse;
    genome.alpha_mask.dense_reference_opt_in = false;
    genome.alpha_mask.default_alpha =
        bounded_delta(genome.alpha_mask.default_alpha, rng, 0.10, intensity)?;
    if let Some(projection) = genome
        .macro_connectome_masks
        .first()
        .map(|mask| mask.projection)
    {
        genome
            .alpha_mask
            .projection_overrides
            .push(ProjectionAlphaOverride {
                projection,
                alpha: bounded_delta(genome.alpha_mask.default_alpha, rng, 0.05, intensity)?,
            });
    }
    Ok(())
}

fn mutate_endocrine(
    constants: &mut [EndocrineConstantGene],
    rng: &mut LabRng,
    intensity: NormalizedScalar,
) -> Result<(), EvolutionLabError> {
    for constant in constants {
        let delta = rng.signed_unit() * 0.15 * intensity.raw();
        constant.value = (constant.value + delta).clamp(0.0, 4.0);
        alife_core::validate_finite(constant.value)?;
    }
    Ok(())
}

fn mutate_drive_thresholds(
    thresholds: &mut [DriveThresholdGene],
    rng: &mut LabRng,
    intensity: NormalizedScalar,
) -> Result<(), EvolutionLabError> {
    for threshold in thresholds {
        threshold.threshold = bounded_delta(threshold.threshold, rng, 0.12, intensity)?;
    }
    Ok(())
}

fn mutate_sensor_layout(channels: &mut [SensorChannelGene], rng: &mut LabRng) {
    for channel in channels {
        let delta = if rng.next_bool() { 8_i32 } else { -8 };
        channel.receptor_count = bounded_u16(channel.receptor_count, delta, 1, 512);
        channel.enabled_at_maturation = bounded_u8(channel.enabled_at_maturation, rng, 100);
    }
}

fn mutate_motor_affordances(affordances: &mut [MotorAffordanceGene], rng: &mut LabRng) {
    for affordance in affordances {
        if affordance.enabled {
            let delta = if rng.next_bool() { 4_i32 } else { -4 };
            affordance.motor_lobe_units = bounded_u16(affordance.motor_lobe_units, delta, 1, 512);
        }
        affordance.enabled_at_maturation = bounded_u8(affordance.enabled_at_maturation, rng, 100);
    }
}

fn mutate_mutation_rates(
    rates: MutationRates,
    rng: &mut LabRng,
    intensity: NormalizedScalar,
) -> Result<MutationRates, EvolutionLabError> {
    Ok(MutationRates {
        point: bounded_delta(rates.point, rng, 0.02, intensity)?,
        structural: bounded_delta(rates.structural, rng, 0.01, intensity)?,
        lobe_ratio: bounded_delta(rates.lobe_ratio, rng, 0.02, intensity)?,
        density: bounded_delta(rates.density, rng, 0.02, intensity)?,
        alpha: bounded_delta(rates.alpha, rng, 0.02, intensity)?,
        endocrine: bounded_delta(rates.endocrine, rng, 0.02, intensity)?,
        developmental_schedule: bounded_delta(rates.developmental_schedule, rng, 0.02, intensity)?,
    })
}

fn mutate_developmental_schedule(
    genome: &mut BrainGenome,
    rng: &mut LabRng,
    intensity: NormalizedScalar,
) -> Result<(), EvolutionLabError> {
    let tick_delta = (rng.signed_unit() * 60.0 * intensity.raw()).round() as i64;
    for DevelopmentalMilestone { begins_at, .. } in
        genome.developmental_schedule.milestones.iter_mut().skip(1)
    {
        begins_at.0 = (begins_at.0 as i64 + tick_delta).max(1) as u64;
    }
    genome
        .developmental_schedule
        .milestones
        .sort_by_key(|milestone| milestone.begins_at.raw());
    for CriticalPeriod {
        opens_at,
        closes_at,
        plasticity_bias,
        ..
    } in &mut genome.developmental_schedule.critical_periods
    {
        let opens = (opens_at.raw() as i64 + tick_delta).max(0) as u64;
        let closes = (closes_at.raw() as i64 + tick_delta).max(opens as i64 + 1) as u64;
        *opens_at = Tick(opens);
        *closes_at = Tick(closes);
        *plasticity_bias = bounded_delta(*plasticity_bias, rng, 0.08, intensity)?;
    }
    genome.developmental_schedule.sleep_pressure_maturation_gate = bounded_delta(
        genome.developmental_schedule.sleep_pressure_maturation_gate,
        rng,
        0.05,
        intensity,
    )?;
    Ok(())
}

fn compare_candidates(a: &SelectionCandidate, b: &SelectionCandidate) -> Ordering {
    b.fitness
        .composite_score
        .partial_cmp(&a.fitness.composite_score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| a.genome.id.0.cmp(&b.genome.id.0))
}

fn social_word_signal(record: &PackedExperienceRecord) -> Result<f32, EvolutionLabError> {
    let heard = record
        .side_buffers
        .records()
        .iter()
        .any(|side| side.kind == PackedSideBufferKind::HeardToken);
    let teacher = record
        .side_buffers
        .records()
        .iter()
        .any(|side| side.kind == PackedSideBufferKind::TeacherSchoolRef);
    Ok(match (heard, teacher) {
        (true, true) => 1.0,
        (true, false) | (false, true) => 0.5,
        (false, false) => 0.0,
    })
}

fn teacher_scores(record: &PackedExperienceRecord) -> impl Iterator<Item = f32> + '_ {
    record
        .side_buffers
        .records()
        .iter()
        .filter(|side| side.kind == PackedSideBufferKind::TeacherSchoolRef)
        .map(|side| side.values[0].clamp(0.0, 1.0))
}

fn choose_vec<T: Clone>(rng: &mut LabRng, a: &[T], b: &[T]) -> Vec<T> {
    let max_len = a.len().max(b.len());
    let mut out = Vec::with_capacity(max_len);
    for index in 0..max_len {
        let chosen = if rng.next_bool() {
            a.get(index)
        } else {
            b.get(index)
        }
        .or_else(|| a.get(index))
        .or_else(|| b.get(index));
        if let Some(value) = chosen {
            out.push(value.clone());
        }
    }
    out
}

fn all_mutation_fields() -> Vec<MutationField> {
    vec![
        MutationField::LobeRatios,
        MutationField::MacroConnectomeMasks,
        MutationField::SparseDensityPriors,
        MutationField::AlphaMask,
        MutationField::EndocrineConstants,
        MutationField::DriveThresholds,
        MutationField::SensorLayout,
        MutationField::MotorAffordances,
        MutationField::MutationRates,
        MutationField::DevelopmentalSchedule,
    ]
}

fn bounded_delta(
    value: NormalizedScalar,
    rng: &mut LabRng,
    scale: f32,
    intensity: NormalizedScalar,
) -> Result<NormalizedScalar, EvolutionLabError> {
    Ok(NormalizedScalar::new(
        (value.raw() + rng.signed_unit() * scale * intensity.raw()).clamp(0.0, 1.0),
    )?)
}

fn bounded_u16(value: u16, delta: i32, min: u16, max: u16) -> u16 {
    (i32::from(value) + delta).clamp(i32::from(min), i32::from(max)) as u16
}

fn bounded_u8(value: u8, rng: &mut LabRng, max: u8) -> u8 {
    let delta = if rng.next_bool() { 5_i16 } else { -5 };
    (i16::from(value) + delta).clamp(0, i16::from(max)) as u8
}

fn unit(value: f32) -> Result<f32, EvolutionLabError> {
    alife_core::validate_finite(value)?;
    Ok(value.clamp(0.0, 1.0))
}

fn mix_seed(base: u64, index: u64, salt: u64) -> u64 {
    nonzero(base ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt)
}

fn nonzero(value: u64) -> u64 {
    if value == 0 {
        1
    } else {
        value
    }
}

#[derive(Debug, Clone, Copy)]
struct LabRng {
    state: u64,
}

impl LabRng {
    fn new(seed: u64) -> Self {
        Self {
            state: nonzero(seed),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn next_fraction(&mut self) -> f32 {
        let top = self.next_u64() >> 40;
        top as f32 / ((1_u64 << 24) - 1) as f32
    }

    fn signed_unit(&mut self) -> f32 {
        self.next_fraction() * 2.0 - 1.0
    }

    fn next_bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
}
