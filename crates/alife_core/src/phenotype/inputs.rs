//! Contract-only immutable compiler-input provenance and canonical identity.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    AlphaStoragePolicy, BrainCapacityClass, BrainClassId, BrainGenome, CanonicalDigestBuilder,
    DevelopmentState, FoundationAbiBinding, LobeRatioPlan, ScaffoldContractError, SensorProfile,
    Validate,
};

const INPUTS_SCHEMA_VERSION: u16 = 2;
const INPUTS_DOMAIN: &[u8] = b"alife.phenotype.compiler-inputs.v2";

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PhenotypeCompilerInputs {
    schema_version: u16,
    genome: BrainGenome,
    development: DevelopmentState,
    sensor_profile: SensorProfile,
    foundation_abi: FoundationAbiBinding,
    capacity_class_id: BrainClassId,
    capacity_digest: [u64; 4],
    canonical_digest: [u64; 4],
}

impl PhenotypeCompilerInputs {
    pub fn try_new(
        genome: BrainGenome,
        capacity: &BrainCapacityClass,
        development: DevelopmentState,
        sensor_profile: SensorProfile,
    ) -> Result<Self, ScaffoldContractError> {
        let foundation_abi = FoundationAbiBinding::canonical_for_capacity(capacity)?;
        Self::try_new_with_foundation_abi(
            genome,
            capacity,
            development,
            sensor_profile,
            foundation_abi,
        )
    }

    pub fn try_new_with_foundation_abi(
        genome: BrainGenome,
        capacity: &BrainCapacityClass,
        development: DevelopmentState,
        sensor_profile: SensorProfile,
        foundation_abi: FoundationAbiBinding,
    ) -> Result<Self, ScaffoldContractError> {
        capacity.validate_contract()?;
        genome.validate_contract()?;
        development.validate_contract()?;
        foundation_abi.validate_against(capacity)?;
        if genome.brain_class_id != capacity.id() || development.genome_id != genome.id {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        SensorProfile::try_from_raw(sensor_profile.raw())?;
        let mut value = Self {
            schema_version: INPUTS_SCHEMA_VERSION,
            genome,
            development,
            sensor_profile,
            foundation_abi,
            capacity_class_id: capacity.id(),
            capacity_digest: capacity.canonical_digest(),
            canonical_digest: [0; 4],
        };
        value.canonical_digest = value.recompute_digest()?;
        Ok(value)
    }

    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }
    pub const fn sensor_profile(&self) -> SensorProfile {
        self.sensor_profile
    }
    pub const fn capacity_class_id(&self) -> BrainClassId {
        self.capacity_class_id
    }
    pub const fn foundation_abi(&self) -> &FoundationAbiBinding {
        &self.foundation_abi
    }
    pub(super) const fn genome(&self) -> &BrainGenome {
        &self.genome
    }
    pub(super) const fn development(&self) -> &DevelopmentState {
        &self.development
    }

    pub fn validate_against(
        &self,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError> {
        capacity.validate_contract()?;
        self.genome.validate_contract()?;
        self.development.validate_contract()?;
        self.foundation_abi.validate_against(capacity)?;
        if self.schema_version != INPUTS_SCHEMA_VERSION
            || self.capacity_class_id != capacity.id()
            || self.capacity_digest != capacity.canonical_digest()
            || self.genome.brain_class_id != self.capacity_class_id
            || self.development.genome_id != self.genome.id
            || SensorProfile::try_from_raw(self.sensor_profile.raw()).is_err()
            || self.recompute_digest()? != self.canonical_digest
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut d = CanonicalDigestBuilder::new(INPUTS_DOMAIN);
        d.write_u16(self.schema_version);
        encode_genome(&mut d, &self.genome)?;
        encode_development(&mut d, &self.development)?;
        d.write_u16(self.sensor_profile.raw());
        d.write_u16(self.foundation_abi.capacity_class_id().raw());
        d.write_u64(self.foundation_abi.layout_id().0);
        for byte in self.foundation_abi.layout_digest().bytes() {
            d.write_u8(*byte);
        }
        d.write_u32(self.foundation_abi.language_codebook().id().0);
        for byte in self
            .foundation_abi
            .language_codebook()
            .canonical_digest()
            .bytes()
        {
            d.write_u8(*byte);
        }
        d.write_u16(self.capacity_class_id.raw());
        for word in self.capacity_digest {
            d.write_u64(word);
        }
        Ok(d.finish256())
    }
}

impl<'de> Deserialize<'de> for PhenotypeCompilerInputs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            genome: BrainGenome,
            development: DevelopmentState,
            sensor_profile: SensorProfile,
            foundation_abi: FoundationAbiBinding,
            capacity_class_id: BrainClassId,
            capacity_digest: [u64; 4],
            canonical_digest: [u64; 4],
        }
        let w = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: w.schema_version,
            genome: w.genome,
            development: w.development,
            sensor_profile: w.sensor_profile,
            foundation_abi: w.foundation_abi,
            capacity_class_id: w.capacity_class_id,
            capacity_digest: w.capacity_digest,
            canonical_digest: w.canonical_digest,
        };
        let capacity = BrainCapacityClass::production_for_id(value.capacity_class_id)
            .map_err(D::Error::custom)?;
        value
            .validate_against(&capacity)
            .map_err(D::Error::custom)?;
        Ok(value)
    }
}

fn encode_genome(
    d: &mut CanonicalDigestBuilder,
    g: &BrainGenome,
) -> Result<(), ScaffoldContractError> {
    d.write_u16(g.schema_version);
    d.write_u64(g.id.0);
    d.write_sequence_len(g.parent_genome_ids.len());
    for id in &g.parent_genome_ids {
        d.write_u64(id.0);
    }
    match g.lineage_id {
        Some(id) => {
            d.write_some();
            d.write_u64(id.0);
        }
        None => d.write_none(),
    }
    d.write_u64(g.species_seed);
    d.write_u16(g.brain_class_id.raw());
    d.write_u64(g.genetic_prior_seed);
    for seed in [
        g.seeds.species_seed,
        g.seeds.genome_id_seed,
        g.seeds.genetic_prior_seed,
        g.seeds.mutation_seed,
        g.seeds.crossover_seed,
        g.seeds.development_seed,
        g.seeds.sensor_layout_seed,
    ] {
        d.write_u64(seed);
    }
    match &g.lobe_ratios {
        LobeRatioPlan::ClassDefault => d.write_u8(0),
        LobeRatioPlan::RegistryRef(r) => {
            d.write_u8(1);
            d.write_u64(r.registry_id);
            d.write_u16(r.version);
        }
        LobeRatioPlan::InlineOverrides(rows) => {
            d.write_u8(2);
            d.write_sequence_len(rows.len());
            for row in rows {
                d.write_u16(row.lobe.raw());
                d.write_f32(row.ratio.raw())?;
            }
        }
    }
    d.write_sequence_len(g.macro_connectome_masks.len());
    for row in &g.macro_connectome_masks {
        encode_projection_key(d, row.projection);
        d.write_bool(row.enabled);
        d.write_bool(row.structural_growth_allowed);
    }
    d.write_sequence_len(g.sparse_density_priors.len());
    for row in &g.sparse_density_priors {
        encode_projection_key(d, row.projection);
        d.write_f32(row.density.raw())?;
        d.write_f32(row.max_active_synapse_share.raw())?;
    }
    d.write_u8(match g.alpha_mask.storage_policy {
        AlphaStoragePolicy::HierarchicalSparse => 0,
        AlphaStoragePolicy::DenseDebugReference => 1,
    });
    d.write_f32(g.alpha_mask.default_alpha.raw())?;
    d.write_sequence_len(g.alpha_mask.projection_overrides.len());
    for row in &g.alpha_mask.projection_overrides {
        encode_projection_key(d, row.projection);
        d.write_f32(row.alpha.raw())?;
    }
    d.write_sequence_len(g.alpha_mask.lobe_overrides.len());
    for row in &g.alpha_mask.lobe_overrides {
        d.write_u16(row.lobe.raw());
        d.write_f32(row.alpha.raw())?;
    }
    d.write_sequence_len(g.alpha_mask.tile_overrides.len());
    for row in &g.alpha_mask.tile_overrides {
        d.write_u16(row.tile.lobe.raw());
        d.write_u32(row.tile.tile_index);
        d.write_f32(row.alpha.raw())?;
    }
    d.write_sequence_len(g.alpha_mask.per_synapse_overrides.len());
    for row in &g.alpha_mask.per_synapse_overrides {
        d.write_u32(row.synapse.source.0);
        d.write_u32(row.synapse.target.0);
        d.write_f32(row.alpha.raw())?;
        d.write_utf8(&row.exceptional_reason);
    }
    d.write_bool(g.alpha_mask.dense_reference_opt_in);
    d.write_bool(g.plasticity_mask.oja_enabled);
    d.write_bool(g.plasticity_mask.hebbian_enabled);
    d.write_sequence_len(g.plasticity_mask.projection_masks.len());
    for row in &g.plasticity_mask.projection_masks {
        encode_projection_key(d, row.projection);
        d.write_f32(row.learning_rate_scale.raw())?;
        d.write_bool(row.plasticity_enabled);
    }
    d.write_sequence_len(g.endocrine_constants.len());
    for row in &g.endocrine_constants {
        d.write_u8(row.kind.raw());
        d.write_f32(row.value)?;
    }
    d.write_sequence_len(g.drive_thresholds.len());
    for row in &g.drive_thresholds {
        d.write_u8(row.kind.raw());
        d.write_f32(row.threshold.raw())?;
    }
    d.write_sequence_len(g.sensor_layout.channels.len());
    for row in &g.sensor_layout.channels {
        d.write_u8(row.kind.raw());
        d.write_u16(row.receptor_count);
        d.write_u16(row.target_lobe.raw());
        d.write_u8(row.enabled_at_maturation);
    }
    d.write_sequence_len(g.motor_affordances.len());
    for row in &g.motor_affordances {
        d.write_u8(row.kind.raw());
        d.write_bool(row.enabled);
        d.write_u16(row.motor_lobe_units);
        d.write_u8(row.enabled_at_maturation);
    }
    for value in [
        g.mutation_rates.point,
        g.mutation_rates.structural,
        g.mutation_rates.lobe_ratio,
        g.mutation_rates.density,
        g.mutation_rates.alpha,
        g.mutation_rates.endocrine,
        g.mutation_rates.developmental_schedule,
    ] {
        d.write_f32(value.raw())?;
    }
    d.write_bool(g.crossover.enabled);
    d.write_u8(g.crossover.max_segments);
    d.write_f32(g.crossover.parent_mix_bias.raw())?;
    d.write_sequence_len(g.developmental_schedule.milestones.len());
    for row in &g.developmental_schedule.milestones {
        d.write_u8(row.stage.raw());
        d.write_u64(row.begins_at.0);
        d.write_f32(row.maturation.raw())?;
        match row.target_brain_class_id {
            Some(id) => {
                d.write_some();
                d.write_u16(id.raw());
            }
            None => d.write_none(),
        }
    }
    d.write_sequence_len(g.developmental_schedule.critical_periods.len());
    for row in &g.developmental_schedule.critical_periods {
        encode_critical_period(d, row)?;
    }
    d.write_u32(g.developmental_schedule.consolidation_cadence_ticks);
    d.write_f32(
        g.developmental_schedule
            .sleep_pressure_maturation_gate
            .raw(),
    )?;
    d.write_bool(g.inheritance.inherited_deja_vu_enabled);
    d.write_bool(g.inheritance.species_culture_priors_enabled);
    d.write_bool(g.inheritance.lamarckian_weights_enabled);
    d.write_bool(g.inheritance.inherit_lifetime_consolidation);
    Ok(())
}

fn encode_development(
    d: &mut CanonicalDigestBuilder,
    state: &DevelopmentState,
) -> Result<(), ScaffoldContractError> {
    d.write_u64(state.genome_id.0);
    d.write_u64(state.age_ticks.0);
    d.write_f32(state.maturation.raw())?;
    d.write_sequence_len(state.enabled_lobes.len());
    for value in &state.enabled_lobes {
        d.write_u16(value.raw());
    }
    d.write_sequence_len(state.active_sensor_channels.len());
    for value in &state.active_sensor_channels {
        d.write_u8(value.raw());
    }
    d.write_sequence_len(state.active_motor_affordances.len());
    for value in &state.active_motor_affordances {
        d.write_u8(value.raw());
    }
    d.write_sequence_len(state.open_critical_periods.len());
    for row in &state.open_critical_periods {
        encode_critical_period(d, row)?;
    }
    d.write_u32(state.sleep_cycle_count);
    d.write_u32(state.consolidation_cycle_count);
    match state.last_sleep_tick {
        Some(tick) => {
            d.write_some();
            d.write_u64(tick.0);
        }
        None => d.write_none(),
    }
    Ok(())
}

fn encode_projection_key(d: &mut CanonicalDigestBuilder, key: crate::ProjectionKey) {
    d.write_u16(key.source_lobe.raw());
    d.write_u16(key.target_lobe.raw());
}

fn encode_critical_period(
    d: &mut CanonicalDigestBuilder,
    row: &crate::CriticalPeriod,
) -> Result<(), ScaffoldContractError> {
    d.write_u16(row.lobe.raw());
    d.write_u64(row.opens_at.0);
    d.write_u64(row.closes_at.0);
    d.write_f32(row.plasticity_bias.raw())
}
