//! Compiler-owned plasticity receptor, replay-capture, and sleep plans.

use std::collections::BTreeMap;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    BrainGenome, CanonicalDigestBuilder, DevelopmentState, FoundationSectionPolicy,
    LifetimePlasticityBand, N2048FoundationLayoutV1, ScaffoldContractError, SchemaKind,
    SchemaVersions, Validate,
};

use super::{
    BrainCapacityClass, BrainPhenotype, CompiledProjection, CompiledSynapse, CompiledSynapseKind,
};

const REPLAY_CAPTURE_DOMAIN: &[u8] = b"alife.replay-capture-plan.v1";
const SLEEP_PLAN_DOMAIN: &[u8] = b"alife.sleep-consolidation-plan.v1";
const PLASTICITY_PLAN_DOMAIN: &[u8] = b"alife.plasticity-plan.v1";
const PROCEDURAL_RECURRENT_LIFETIME_SCALE: f32 = 0.1;
/// Maximum replay-capture identities admitted by the frozen learning ABI.
pub const MAX_REPLAY_CAPTURE_SYNAPSES: u32 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ReplayCaptureGroup {
    class_priority: u8,
    logical_group: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReplayCaptureCandidate {
    group: ReplayCaptureGroup,
    band_priority: u8,
    biological_priority: u8,
    global_synapse_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct PlasticityReceptorPlan {
    eligibility_decay: f32,
    learning_rate: f32,
    sleep_replay_rate: f32,
    normalization_rate: f32,
    modulator_sign: f32,
    fast_weight_min: f32,
    fast_weight_max: f32,
}

impl PlasticityReceptorPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        eligibility_decay: f32,
        learning_rate: f32,
        sleep_replay_rate: f32,
        normalization_rate: f32,
        modulator_sign: f32,
        fast_weight_min: f32,
        fast_weight_max: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let value = Self {
            eligibility_decay,
            learning_rate,
            sleep_replay_rate,
            normalization_rate,
            modulator_sign,
            fast_weight_min,
            fast_weight_max,
        };
        value.validate_contract()?;
        Ok(value)
    }

    pub const fn eligibility_decay(&self) -> f32 {
        self.eligibility_decay
    }
    pub const fn learning_rate(&self) -> f32 {
        self.learning_rate
    }
    pub const fn sleep_replay_rate(&self) -> f32 {
        self.sleep_replay_rate
    }
    pub const fn normalization_rate(&self) -> f32 {
        self.normalization_rate
    }
    pub const fn modulator_sign(&self) -> f32 {
        self.modulator_sign
    }
    pub const fn fast_weight_bounds(&self) -> (f32, f32) {
        (self.fast_weight_min, self.fast_weight_max)
    }
    pub const fn is_delta_enabled(&self) -> bool {
        self.learning_rate != 0.0 || self.normalization_rate != 0.0 || self.sleep_replay_rate != 0.0
    }

    pub(super) fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        let values = [
            self.eligibility_decay,
            self.learning_rate,
            self.sleep_replay_rate,
            self.normalization_rate,
            self.modulator_sign,
            self.fast_weight_min,
            self.fast_weight_max,
        ];
        if values.into_iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if !(0.0..=1.0).contains(&self.eligibility_decay)
            || !(0.0..=1.0).contains(&self.learning_rate)
            || !(0.0..=1.0).contains(&self.sleep_replay_rate)
            || !(0.0..=1.0).contains(&self.normalization_rate)
            || !matches!(self.modulator_sign, -1.0 | 1.0)
            || !(-8.0..=8.0).contains(&self.fast_weight_min)
            || !(-8.0..=8.0).contains(&self.fast_weight_max)
            || self.fast_weight_min >= self.fast_weight_max
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    fn write_canonical(
        &self,
        digest: &mut CanonicalDigestBuilder,
    ) -> Result<(), ScaffoldContractError> {
        digest.write_f32(self.eligibility_decay)?;
        digest.write_f32(self.learning_rate)?;
        digest.write_f32(self.sleep_replay_rate)?;
        digest.write_f32(self.normalization_rate)?;
        digest.write_f32(self.modulator_sign)?;
        digest.write_f32(self.fast_weight_min)?;
        digest.write_f32(self.fast_weight_max)
    }
}

impl<'de> Deserialize<'de> for PlasticityReceptorPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            eligibility_decay: f32,
            learning_rate: f32,
            sleep_replay_rate: f32,
            normalization_rate: f32,
            modulator_sign: f32,
            fast_weight_min: f32,
            fast_weight_max: f32,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::try_new(
            wire.eligibility_decay,
            wire.learning_rate,
            wire.sleep_replay_rate,
            wire.normalization_rate,
            wire.modulator_sign,
            wire.fast_weight_min,
            wire.fast_weight_max,
        )
        .map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplayCapturePlan {
    schema_version: u16,
    global_synapse_ids: Vec<u32>,
    samples_per_event: u16,
    event_capacity: u32,
    sample_capacity: u32,
    canonical_digest: [u64; 4],
}

impl ReplayCapturePlan {
    pub fn try_new(
        global_synapse_ids: Vec<u32>,
        samples_per_event: u16,
        event_capacity: u32,
        sample_capacity: u32,
    ) -> Result<Self, ScaffoldContractError> {
        let mut value = Self {
            schema_version: SchemaVersions::CURRENT.learning.raw(),
            global_synapse_ids,
            samples_per_event,
            event_capacity,
            sample_capacity,
            canonical_digest: [0; 4],
        };
        value.validate_local()?;
        value.canonical_digest = value.recompute_digest();
        Ok(value)
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub fn global_synapse_ids(&self) -> &[u32] {
        &self.global_synapse_ids
    }
    pub const fn samples_per_event(&self) -> u16 {
        self.samples_per_event
    }
    pub const fn event_capacity(&self) -> u32 {
        self.event_capacity
    }
    pub const fn sample_capacity(&self) -> u32 {
        self.sample_capacity
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }

    pub fn validate_against(
        &self,
        phenotype: &BrainPhenotype,
        capacity: &BrainCapacityClass,
    ) -> Result<(), ScaffoldContractError> {
        self.validate_local()?;
        capacity.validate_contract()?;
        if phenotype.brain_class_id() != capacity.id()
            || self.event_capacity > capacity.execution().max_replay_events()
            || self.sample_capacity > capacity.execution().max_replay_eligibility_samples()
            || self.global_synapse_ids.len() as u32
                != phenotype.budgets().global.replay_capture_synapse_count
            || self.global_synapse_ids.iter().any(|id| {
                phenotype
                    .synapses()
                    .get(*id as usize)
                    .and_then(|synapse| {
                        phenotype
                            .plasticity_receptors()
                            .get(usize::from(synapse.receptor_index()))
                    })
                    .is_none_or(|receptor| !receptor.is_delta_enabled())
            })
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.validate_local()
    }

    fn validate_local(&self) -> Result<(), ScaffoldContractError> {
        if self.schema_version != SchemaVersions::CURRENT.learning.raw()
            || self.global_synapse_ids.is_empty()
            || self.global_synapse_ids.len() > MAX_REPLAY_CAPTURE_SYNAPSES as usize
            || self
                .global_synapse_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            || usize::from(self.samples_per_event) != self.global_synapse_ids.len()
            || self.event_capacity == 0
            || self.event_capacity > 65_536
            || self.sample_capacity
                != self
                    .event_capacity
                    .checked_mul(u32::from(self.samples_per_event))
                    .ok_or(ScaffoldContractError::PhenotypeCompile)?
        {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        if self.canonical_digest != [0; 4] && self.canonical_digest != self.recompute_digest() {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(REPLAY_CAPTURE_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_sequence_len(self.global_synapse_ids.len());
        for id in &self.global_synapse_ids {
            digest.write_u32(*id);
        }
        digest.write_u16(self.samples_per_event);
        digest.write_u32(self.event_capacity);
        digest.write_u32(self.sample_capacity);
        digest.finish256()
    }
}

impl<'de> Deserialize<'de> for ReplayCapturePlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            global_synapse_ids: Vec<u32>,
            samples_per_event: u16,
            event_capacity: u32,
            sample_capacity: u32,
            canonical_digest: [u64; 4],
        }
        let wire = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: wire.schema_version,
            global_synapse_ids: wire.global_synapse_ids,
            samples_per_event: wire.samples_per_event,
            event_capacity: wire.event_capacity,
            sample_capacity: wire.sample_capacity,
            canonical_digest: wire.canonical_digest,
        };
        value.validate_local().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SleepConsolidationPlan {
    schema_version: u16,
    staging_rate: f32,
    weight_limit: f32,
    fast_decay_rate: f32,
    eligibility_reset_policy_raw: u16,
    replay_consume_policy_raw: u16,
    canonical_digest: [u64; 4],
}

impl SleepConsolidationPlan {
    pub fn try_new_v1(
        staging_rate: f32,
        weight_limit: f32,
        fast_decay_rate: f32,
    ) -> Result<Self, ScaffoldContractError> {
        let mut value = Self {
            schema_version: SchemaVersions::CURRENT.sleep_consolidation.raw(),
            staging_rate,
            weight_limit,
            fast_decay_rate,
            eligibility_reset_policy_raw: 1,
            replay_consume_policy_raw: 1,
            canonical_digest: [0; 4],
        };
        value.validate_fields()?;
        value.canonical_digest = value.recompute_digest()?;
        Ok(value)
    }

    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }
    pub const fn staging_rate(&self) -> f32 {
        self.staging_rate
    }
    pub const fn weight_limit(&self) -> f32 {
        self.weight_limit
    }
    pub const fn fast_decay_rate(&self) -> f32 {
        self.fast_decay_rate
    }
    pub const fn eligibility_reset_policy_raw(&self) -> u16 {
        self.eligibility_reset_policy_raw
    }
    pub const fn replay_consume_policy_raw(&self) -> u16 {
        self.replay_consume_policy_raw
    }
    pub const fn canonical_digest(&self) -> [u64; 4] {
        self.canonical_digest
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.validate_fields()?;
        if self.canonical_digest != self.recompute_digest()? {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        Ok(())
    }

    fn validate_fields(&self) -> Result<(), ScaffoldContractError> {
        crate::require_current_version(SchemaKind::SleepConsolidation, self.schema_version)?;
        if ![self.staging_rate, self.weight_limit, self.fast_decay_rate]
            .into_iter()
            .all(f32::is_finite)
        {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        if !(0.0..=1.0).contains(&self.staging_rate)
            || self.staging_rate == 0.0
            || !(0.0..=8.0).contains(&self.weight_limit)
            || self.weight_limit == 0.0
            || !(0.0..=1.0).contains(&self.fast_decay_rate)
            || self.eligibility_reset_policy_raw != 1
            || self.replay_consume_policy_raw != 1
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], ScaffoldContractError> {
        let mut digest = CanonicalDigestBuilder::new(SLEEP_PLAN_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_f32(self.staging_rate)?;
        digest.write_f32(self.weight_limit)?;
        digest.write_f32(self.fast_decay_rate)?;
        digest.write_u16(self.eligibility_reset_policy_raw);
        digest.write_u16(self.replay_consume_policy_raw);
        Ok(digest.finish256())
    }
}

impl<'de> Deserialize<'de> for SleepConsolidationPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Wire {
            schema_version: u16,
            staging_rate: f32,
            weight_limit: f32,
            fast_decay_rate: f32,
            eligibility_reset_policy_raw: u16,
            replay_consume_policy_raw: u16,
            canonical_digest: [u64; 4],
        }
        let wire = Wire::deserialize(deserializer)?;
        let value = Self {
            schema_version: wire.schema_version,
            staging_rate: wire.staging_rate,
            weight_limit: wire.weight_limit,
            fast_decay_rate: wire.fast_decay_rate,
            eligibility_reset_policy_raw: wire.eligibility_reset_policy_raw,
            replay_consume_policy_raw: wire.replay_consume_policy_raw,
            canonical_digest: wire.canonical_digest,
        };
        value.validate_contract().map_err(D::Error::custom)?;
        Ok(value)
    }
}

pub(super) struct CompiledLearningPlans {
    pub receptors: Vec<PlasticityReceptorPlan>,
    pub replay: ReplayCapturePlan,
    pub sleep: SleepConsolidationPlan,
    pub digest: [u64; 4],
}

pub(super) fn compile_learning_plans(
    genome: &BrainGenome,
    development: &DevelopmentState,
    capacity: &BrainCapacityClass,
    projections: &[CompiledProjection],
    synapses: &mut [CompiledSynapse],
) -> Result<CompiledLearningPlans, ScaffoldContractError> {
    genome.plasticity_parameters().validate_contract()?;
    let parameters = genome.plasticity_parameters();
    let (fast_min, fast_max) = parameters.fast_bounds();
    let disabled = PlasticityReceptorPlan::try_new(
        parameters.eligibility_decay(),
        0.0,
        0.0,
        0.0,
        parameters.modulator_sign(),
        fast_min,
        fast_max,
    )?;
    let mut receptors = vec![disabled];
    let mut replay_candidates = Vec::new();

    for projection in projections {
        let (start, len) = projection.synapse_range();
        let end = start
            .checked_add(len)
            .ok_or(ScaffoldContractError::PhenotypeCompile)?;
        let section_policy = n2048_section_policy(capacity, projection);
        for (local, synapse) in synapses[start as usize..end as usize]
            .iter_mut()
            .enumerate()
        {
            let (scale, band_priority) = receptor_scale(
                genome,
                projection,
                synapse.kind(),
                section_policy,
                local as u32,
            );
            let developmental_multiplier = critical_period_multiplier(development, projection);
            let effective_scale = (scale * developmental_multiplier).clamp(0.0, 1.0);
            let receptor = if effective_scale == 0.0 {
                disabled
            } else {
                PlasticityReceptorPlan::try_new(
                    parameters.eligibility_decay(),
                    (parameters.base_learning_rate() * effective_scale).clamp(0.0, 1.0),
                    (parameters.sleep_replay_rate() * effective_scale).clamp(0.0, 1.0),
                    (parameters.normalization_rate() * effective_scale).clamp(0.0, 1.0),
                    parameters.modulator_sign(),
                    fast_min,
                    fast_max,
                )?
            };
            let receptor_index =
                if let Some(index) = receptors.iter().position(|row| *row == receptor) {
                    u16::try_from(index).map_err(|_| ScaffoldContractError::PhenotypeCompile)?
                } else {
                    receptors.push(receptor);
                    u16::try_from(receptors.len() - 1)
                        .map_err(|_| ScaffoldContractError::PhenotypeCompile)?
                };
            synapse.set_receptor_index(receptor_index);
            if receptor.is_delta_enabled() {
                let global_id = start + local as u32;
                replay_candidates.push(ReplayCaptureCandidate {
                    group: replay_capture_group(synapse.kind(), projection.route_index()),
                    band_priority,
                    biological_priority: projection.priority().raw(),
                    global_synapse_id: global_id,
                });
            }
        }
    }

    let event_capacity = capacity.execution().max_replay_events();
    let max_per_event = capacity
        .execution()
        .max_replay_eligibility_samples()
        .checked_div(event_capacity)
        .ok_or(ScaffoldContractError::PhenotypeCompile)? as usize;
    let capture_count = replay_candidates
        .len()
        .min(MAX_REPLAY_CAPTURE_SYNAPSES as usize)
        .min(max_per_event);
    if capture_count == 0 {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    let mut capture_buckets = BTreeMap::<ReplayCaptureGroup, Vec<_>>::new();
    for candidate in replay_candidates {
        capture_buckets
            .entry(candidate.group)
            .or_default()
            .push(candidate);
    }
    for bucket in capture_buckets.values_mut() {
        bucket.sort_unstable_by_key(|candidate| {
            (
                candidate.band_priority,
                candidate.biological_priority,
                candidate.global_synapse_id,
            )
        });
    }
    let mut capture_ids = Vec::with_capacity(capture_count);
    let mut depth = 0_usize;
    while capture_ids.len() < capture_count {
        let mut added = false;
        for bucket in capture_buckets.values() {
            if let Some(candidate) = bucket.get(depth) {
                capture_ids.push(candidate.global_synapse_id);
                added = true;
                if capture_ids.len() == capture_count {
                    break;
                }
            }
        }
        if !added {
            return Err(ScaffoldContractError::PhenotypeCompile);
        }
        depth += 1;
    }
    capture_ids.sort_unstable();
    let samples_per_event =
        u16::try_from(capture_ids.len()).map_err(|_| ScaffoldContractError::PhenotypeCompile)?;
    let sample_capacity = event_capacity
        .checked_mul(u32::from(samples_per_event))
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let replay = ReplayCapturePlan::try_new(
        capture_ids,
        samples_per_event,
        event_capacity,
        sample_capacity,
    )?;
    let sleep = SleepConsolidationPlan::try_new_v1(
        parameters.sleep_staging_rate(),
        parameters.sleep_weight_limit(),
        parameters.sleep_fast_decay_rate(),
    )?;
    let digest = compute_plasticity_plan_digest(&receptors, &replay, &sleep)?;
    Ok(CompiledLearningPlans {
        receptors,
        replay,
        sleep,
        digest,
    })
}

fn replay_capture_group(kind: CompiledSynapseKind, route_index: u16) -> ReplayCaptureGroup {
    match kind {
        CompiledSynapseKind::Decoder(coordinate)
            if coordinate.head() == super::DecoderHeadKind::ActionCandidate =>
        {
            ReplayCaptureGroup {
                class_priority: 0,
                logical_group: u16::from(coordinate.family().raw()),
            }
        }
        CompiledSynapseKind::Decoder(coordinate) => ReplayCaptureGroup {
            class_priority: 1,
            logical_group: (coordinate.head().raw() as u16)
                .saturating_mul(8)
                .saturating_add(u16::from(coordinate.family().raw())),
        },
        CompiledSynapseKind::Recurrent => ReplayCaptureGroup {
            class_priority: 2,
            logical_group: route_index,
        },
    }
}

fn n2048_section_policy(
    capacity: &BrainCapacityClass,
    projection: &CompiledProjection,
) -> Option<FoundationSectionPolicy> {
    (capacity.id() == BrainCapacityClass::N2048_ID)
        .then(|| {
            N2048FoundationLayoutV1::route_specs()
                .iter()
                .copied()
                .find(|spec| {
                    spec.source_lobe() == projection.source_lobe()
                        && spec.target_lobe() == projection.target_lobe()
                })
                .map(|spec| spec.section_policy())
        })
        .flatten()
}

fn receptor_scale(
    genome: &BrainGenome,
    projection: &CompiledProjection,
    kind: CompiledSynapseKind,
    section_policy: Option<FoundationSectionPolicy>,
    local_index: u32,
) -> (f32, u8) {
    if matches!(kind, CompiledSynapseKind::Decoder(_)) {
        return (1.0, 0);
    }
    if let Some(policy) = section_policy {
        let fixed_end = policy.count(LifetimePlasticityBand::Fixed);
        let slow_end = fixed_end + policy.count(LifetimePlasticityBand::Slow);
        return if local_index < fixed_end {
            (0.0, 2)
        } else if local_index < slow_end {
            (
                0.25 * projection_mask_scale(genome, projection).unwrap_or(1.0),
                1,
            )
        } else {
            (projection_mask_scale(genome, projection).unwrap_or(1.0), 0)
        };
    }
    projection_mask_scale(genome, projection).map_or((0.0, 2), |scale| {
        (scale * PROCEDURAL_RECURRENT_LIFETIME_SCALE, 1)
    })
}

fn projection_mask_scale(genome: &BrainGenome, projection: &CompiledProjection) -> Option<f32> {
    genome
        .plasticity_mask
        .projection_masks
        .iter()
        .find(|row| {
            row.projection.source_lobe == projection.source_lobe()
                && row.projection.target_lobe == projection.target_lobe()
        })
        .filter(|row| row.plasticity_enabled)
        .map(|row| row.learning_rate_scale.raw())
}

fn critical_period_multiplier(
    development: &DevelopmentState,
    projection: &CompiledProjection,
) -> f32 {
    1.0 + development
        .open_critical_periods
        .iter()
        .filter(|period| {
            period.lobe == projection.source_lobe() || period.lobe == projection.target_lobe()
        })
        .map(|period| period.plasticity_bias.raw())
        .fold(0.0_f32, f32::max)
}

pub(super) fn compute_plasticity_plan_digest(
    receptors: &[PlasticityReceptorPlan],
    replay: &ReplayCapturePlan,
    sleep: &SleepConsolidationPlan,
) -> Result<[u64; 4], ScaffoldContractError> {
    let mut digest = CanonicalDigestBuilder::new(PLASTICITY_PLAN_DOMAIN);
    digest.write_sequence_len(receptors.len());
    for receptor in receptors {
        receptor.validate_contract()?;
        receptor.write_canonical(&mut digest)?;
    }
    for word in replay.canonical_digest() {
        digest.write_u64(word);
    }
    for word in sleep.canonical_digest() {
        digest.write_u64(word);
    }
    Ok(digest.finish256())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_receptor_has_exactly_zero_delta_rates() {
        let receptor =
            PlasticityReceptorPlan::try_new(0.95, 0.0, 0.0, 0.0, 1.0, -2.0, 2.0).unwrap();
        assert!(!receptor.is_delta_enabled());
    }

    #[test]
    fn sleep_plan_rejects_unknown_wire_policies() {
        let plan = SleepConsolidationPlan::try_new_v1(0.5, 4.0, 0.5).unwrap();
        let mut json = serde_json::to_value(plan).unwrap();
        json["eligibility_reset_policy_raw"] = serde_json::json!(2);
        assert!(serde_json::from_value::<SleepConsolidationPlan>(json).is_err());
    }

    #[test]
    fn sleep_plan_reports_the_sleep_schema_domain() {
        let mut plan = SleepConsolidationPlan::try_new_v1(0.5, 4.0, 0.5).unwrap();
        plan.schema_version = u16::MAX;
        assert!(matches!(
            plan.validate_contract(),
            Err(ScaffoldContractError::IncompatibleAbi {
                kind: SchemaKind::SleepConsolidation,
                ..
            })
        ));
    }
}
