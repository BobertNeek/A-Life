//! Portable fixed-point neural-work, GPU-pressure, and throttle receipts.
//!
//! These contracts contain no wgpu objects and perform no neural computation.
//! They make the production GPU scheduler's inputs and committed work replayable
//! without introducing a host neural oracle.

use serde::{Deserialize, Serialize};

use crate::{
    BiologicalPriority, BrainExecutionBudget, BrainPhenotype, CanonicalDigestBuilder,
    CompiledProjection, LobeKind, ProjectionType, ScaffoldContractError,
};

pub const BRAIN_ACTIVITY_SCHEMA_VERSION: u16 = 1;
pub const BRAIN_ACTIVITY_POLICY_VERSION: u16 = 1;
pub const BRAIN_ATP_Q16_MAX: u32 = 65_535;
/// Exact basal organism ATP debit charged once before each world-tick neural
/// opportunity. This is the Q16 policy value corresponding to one percent.
pub const BRAIN_ATP_BASAL_DEBIT_Q16: u32 = 655;
/// Exact restorative credit charged once for a world tick that begins asleep.
pub const BRAIN_ATP_SLEEP_RECOVERY_Q16: u32 = 2_621;

const POLICY_DOMAIN: &[u8] = b"alife.brain.activity.policy.v1";
const PRESSURE_DOMAIN: &[u8] = b"alife.brain.activity.pressure.v1";
const DECISION_DOMAIN: &[u8] = b"alife.brain.activity.decision.v1";
const WORK_RECEIPT_DOMAIN: &[u8] = b"alife.brain.activity.work_receipt.v1";

const ROUTE_DIGEST_SEEDS: [u32; 8] = [
    0x811c_9dc5,
    0x9e37_79b9,
    0x243f_6a88,
    0xb7e1_5163,
    0xa409_3822,
    0x299f_31d0,
    0x082e_fa98,
    0xec4e_6c89,
];
const ROUTE_DIGEST_SALTS: [u32; 8] = [
    0x0000_0000,
    0x85eb_ca6b,
    0xc2b2_ae35,
    0x27d4_eb2f,
    0x1656_67b1,
    0xd3a2_646c,
    0xfd70_46c5,
    0xb55a_4f09,
];
const ROUTE_DIGEST_PRIME: u32 = 16_777_619;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainWorkCounters {
    pub microsteps: u32,
    pub neuron_updates: u64,
    pub tile_visits: u64,
    pub synapse_ops: u64,
    pub decoder_candidate_ops: u64,
    pub memory_context_ops: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainAtpCostModel {
    pub schema_version: u16,
    pub q_fraction_bits: u8,
    pub rounding_mode_raw: u8,
    pub neuron_update_q24: u64,
    pub tile_visit_q24: u64,
    pub synapse_op_q24: u64,
    pub decoder_candidate_op_q24: u64,
    pub memory_context_op_q24: u64,
}

impl BrainAtpCostModel {
    pub const fn production_v1() -> Self {
        Self {
            schema_version: BRAIN_ACTIVITY_SCHEMA_VERSION,
            q_fraction_bits: 24,
            rounding_mode_raw: 1,
            neuron_update_q24: 32,
            tile_visit_q24: 256,
            synapse_op_q24: 4,
            decoder_candidate_op_q24: 128,
            memory_context_op_q24: 64,
        }
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if *self != Self::production_v1() {
            return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
        }
        Ok(())
    }

    pub fn neural_cost_q24(
        &self,
        counters: &BrainWorkCounters,
    ) -> Result<u64, ScaffoldContractError> {
        self.validate_contract()?;
        let terms = [
            (counters.neuron_updates, self.neuron_update_q24),
            (counters.tile_visits, self.tile_visit_q24),
            (counters.synapse_ops, self.synapse_op_q24),
            (
                counters.decoder_candidate_ops,
                self.decoder_candidate_op_q24,
            ),
            (counters.memory_context_ops, self.memory_context_op_q24),
        ];
        let total = terms.iter().try_fold(0_u128, |sum, (count, rate)| {
            sum.checked_add(u128::from(*count).checked_mul(u128::from(*rate))?)
        });
        total
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(ScaffoldContractError::BrainActivityPolicyMismatch)
    }

    pub fn q24_to_atp_q16_round_half_up(
        &self,
        cost_q24: u64,
    ) -> Result<u32, ScaffoldContractError> {
        self.validate_contract()?;
        let rounded = cost_q24
            .checked_add(0x80)
            .ok_or(ScaffoldContractError::BrainActivityPolicyMismatch)?
            >> 8;
        u32::try_from(rounded).map_err(|_| ScaffoldContractError::BrainActivityPolicyMismatch)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainActivityPolicyV1 {
    pub schema_version: u16,
    pub policy_version: u16,
    pub cost: BrainAtpCostModel,
    pub gpu_time_threshold_ns: [u64; 3],
    pub queue_depth_thresholds: [u32; 3],
    pub logical_heap_pressure_thresholds_q16: [u32; 3],
    pub atp_remaining_thresholds_q16_desc: [u32; 3],
    pub policy_digest: [u64; 4],
}

impl BrainActivityPolicyV1 {
    pub fn production_v1() -> Self {
        let mut policy = Self {
            schema_version: BRAIN_ACTIVITY_SCHEMA_VERSION,
            policy_version: BRAIN_ACTIVITY_POLICY_VERSION,
            cost: BrainAtpCostModel::production_v1(),
            gpu_time_threshold_ns: [2_000_000, 4_000_000, 8_000_000],
            queue_depth_thresholds: [1, 2, 4],
            logical_heap_pressure_thresholds_q16: [32_768, 49_152, 58_982],
            atp_remaining_thresholds_q16_desc: [49_152, 32_768, 16_384],
            policy_digest: [0; 4],
        };
        policy.policy_digest = policy.recompute_digest();
        policy
    }

    pub fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        self.cost.validate_contract()?;
        let expected = Self::production_v1();
        if self.schema_version != BRAIN_ACTIVITY_SCHEMA_VERSION
            || self.policy_version != BRAIN_ACTIVITY_POLICY_VERSION
            || self.gpu_time_threshold_ns != expected.gpu_time_threshold_ns
            || self.queue_depth_thresholds != expected.queue_depth_thresholds
            || self.logical_heap_pressure_thresholds_q16
                != expected.logical_heap_pressure_thresholds_q16
            || self.atp_remaining_thresholds_q16_desc != expected.atp_remaining_thresholds_q16_desc
            || self.policy_digest == [0; 4]
            || self.policy_digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(POLICY_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.policy_version);
        digest.write_u16(self.cost.schema_version);
        digest.write_u8(self.cost.q_fraction_bits);
        digest.write_u8(self.cost.rounding_mode_raw);
        digest.write_u64(self.cost.neuron_update_q24);
        digest.write_u64(self.cost.tile_visit_q24);
        digest.write_u64(self.cost.synapse_op_q24);
        digest.write_u64(self.cost.decoder_candidate_op_q24);
        digest.write_u64(self.cost.memory_context_op_q24);
        for value in self.gpu_time_threshold_ns {
            digest.write_u64(value);
        }
        for value in self.queue_depth_thresholds {
            digest.write_u32(value);
        }
        for value in self.logical_heap_pressure_thresholds_q16 {
            digest.write_u32(value);
        }
        for value in self.atp_remaining_thresholds_q16_desc {
            digest.write_u32(value);
        }
        digest.finish256()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrainDispatchIdentity {
    pub organism_id_raw: u64,
    pub tick: u64,
    pub class_id_raw: u16,
    pub handle_slot: u32,
    pub handle_generation: u32,
    pub sequence_cursor: u64,
    pub dispatch_generation: u64,
    pub frame_digest: [u64; 4],
}

impl BrainDispatchIdentity {
    fn validate_contract(&self) -> Result<(), ScaffoldContractError> {
        if self.organism_id_raw == 0
            || self.class_id_raw == 0
            || self.handle_generation == 0
            || self.sequence_cursor == 0
            || self.dispatch_generation == 0
            || self.frame_digest == [0; 4]
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuPressureSampleInput {
    pub identity: BrainDispatchIdentity,
    pub source_dispatch_generation: u64,
    pub source_frame_digest: [u64; 4],
    pub completed_gpu_time_ns: u64,
    pub queue_depth: u32,
    pub logical_heap_used: u64,
    pub logical_heap_capacity: u64,
    pub brain_atp_remaining_q16: u32,
    pub brain_atp_capacity_q16: u32,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NeuralThrottleLevel {
    Full = 0,
    Reduced = 1,
    EssentialOnly = 2,
}

impl NeuralThrottleLevel {
    pub const fn raw(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuPressureSample {
    pub schema_version: u16,
    pub policy_version: u16,
    pub organism_id_raw: u64,
    pub tick: u64,
    pub class_id_raw: u16,
    pub handle_slot: u32,
    pub handle_generation: u32,
    pub sequence_cursor: u64,
    pub source_dispatch_generation: u64,
    pub source_frame_digest: [u64; 4],
    pub completed_gpu_time_ns: u64,
    pub queue_depth: u32,
    pub logical_heap_pressure_q16: u32,
    pub brain_atp_fraction_q16: u32,
    pub completed_gpu_time_bucket: u16,
    pub queue_depth_bucket: u8,
    pub neural_heap_pressure_bucket: u8,
    pub brain_atp_bucket: u8,
    pub sample_digest: [u64; 4],
    pub dispatch_generation: u64,
    pub frame_digest: [u64; 4],
}

impl GpuPressureSample {
    pub fn try_new(
        policy: &BrainActivityPolicyV1,
        input: GpuPressureSampleInput,
    ) -> Result<Self, ScaffoldContractError> {
        policy.validate_contract()?;
        input.identity.validate_contract()?;
        if input.logical_heap_capacity == 0
            || input.brain_atp_capacity_q16 == 0
            || input.logical_heap_used > input.logical_heap_capacity
            || input.brain_atp_remaining_q16 > input.brain_atp_capacity_q16
            || input.source_dispatch_generation >= input.identity.dispatch_generation
            || (input.source_dispatch_generation == 0) != (input.source_frame_digest == [0; 4])
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        let logical_heap_pressure_q16 =
            q16_fraction(input.logical_heap_used, input.logical_heap_capacity)?;
        let brain_atp_fraction_q16 = q16_fraction(
            u64::from(input.brain_atp_remaining_q16),
            u64::from(input.brain_atp_capacity_q16),
        )?;
        let completed_gpu_time_bucket =
            threshold_bucket_u64(input.completed_gpu_time_ns, policy.gpu_time_threshold_ns);
        let queue_depth_bucket =
            threshold_bucket_u32(input.queue_depth, policy.queue_depth_thresholds);
        let neural_heap_pressure_bucket = threshold_bucket_u32(
            logical_heap_pressure_q16,
            policy.logical_heap_pressure_thresholds_q16,
        );
        let brain_atp_bucket = descending_atp_bucket(
            brain_atp_fraction_q16,
            policy.atp_remaining_thresholds_q16_desc,
        );
        let mut sample = Self {
            schema_version: policy.schema_version,
            policy_version: policy.policy_version,
            organism_id_raw: input.identity.organism_id_raw,
            tick: input.identity.tick,
            class_id_raw: input.identity.class_id_raw,
            handle_slot: input.identity.handle_slot,
            handle_generation: input.identity.handle_generation,
            sequence_cursor: input.identity.sequence_cursor,
            source_dispatch_generation: input.source_dispatch_generation,
            source_frame_digest: input.source_frame_digest,
            completed_gpu_time_ns: input.completed_gpu_time_ns,
            queue_depth: input.queue_depth,
            logical_heap_pressure_q16,
            brain_atp_fraction_q16,
            completed_gpu_time_bucket: u16::from(completed_gpu_time_bucket),
            queue_depth_bucket,
            neural_heap_pressure_bucket,
            brain_atp_bucket,
            sample_digest: [0; 4],
            dispatch_generation: input.identity.dispatch_generation,
            frame_digest: input.identity.frame_digest,
        };
        sample.sample_digest = sample.recompute_digest();
        sample.validate_for(policy)?;
        Ok(sample)
    }

    pub fn dispatch_identity(self) -> BrainDispatchIdentity {
        BrainDispatchIdentity {
            organism_id_raw: self.organism_id_raw,
            tick: self.tick,
            class_id_raw: self.class_id_raw,
            handle_slot: self.handle_slot,
            handle_generation: self.handle_generation,
            sequence_cursor: self.sequence_cursor,
            dispatch_generation: self.dispatch_generation,
            frame_digest: self.frame_digest,
        }
    }

    pub fn throttle_level(self) -> NeuralThrottleLevel {
        match self
            .completed_gpu_time_bucket
            .max(u16::from(self.queue_depth_bucket))
            .max(u16::from(self.neural_heap_pressure_bucket))
            .max(u16::from(self.brain_atp_bucket))
        {
            0 => NeuralThrottleLevel::Full,
            1 => NeuralThrottleLevel::Reduced,
            _ => NeuralThrottleLevel::EssentialOnly,
        }
    }

    pub fn validate_for(
        &self,
        policy: &BrainActivityPolicyV1,
    ) -> Result<(), ScaffoldContractError> {
        policy.validate_contract()?;
        self.dispatch_identity().validate_contract()?;
        let source_binding_valid = self.source_dispatch_generation < self.dispatch_generation
            && ((self.source_dispatch_generation == 0) == (self.source_frame_digest == [0; 4]));
        if self.schema_version != policy.schema_version
            || self.policy_version != policy.policy_version
            || self.logical_heap_pressure_q16 > BRAIN_ATP_Q16_MAX
            || self.brain_atp_fraction_q16 > BRAIN_ATP_Q16_MAX
            || self.completed_gpu_time_bucket
                != u16::from(threshold_bucket_u64(
                    self.completed_gpu_time_ns,
                    policy.gpu_time_threshold_ns,
                ))
            || self.queue_depth_bucket
                != threshold_bucket_u32(self.queue_depth, policy.queue_depth_thresholds)
            || self.neural_heap_pressure_bucket
                != threshold_bucket_u32(
                    self.logical_heap_pressure_q16,
                    policy.logical_heap_pressure_thresholds_q16,
                )
            || self.brain_atp_bucket
                != descending_atp_bucket(
                    self.brain_atp_fraction_q16,
                    policy.atp_remaining_thresholds_q16_desc,
                )
            || !source_binding_valid
            || self.sample_digest == [0; 4]
            || self.sample_digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(PRESSURE_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.policy_version);
        digest.write_u64(self.organism_id_raw);
        digest.write_u64(self.tick);
        digest.write_u16(self.class_id_raw);
        digest.write_u32(self.handle_slot);
        digest.write_u32(self.handle_generation);
        digest.write_u64(self.sequence_cursor);
        digest.write_u64(self.dispatch_generation);
        write_digest4(&mut digest, self.frame_digest);
        digest.write_u64(self.source_dispatch_generation);
        write_digest4(&mut digest, self.source_frame_digest);
        digest.write_u64(self.completed_gpu_time_ns);
        digest.write_u32(self.queue_depth);
        digest.write_u32(self.logical_heap_pressure_q16);
        digest.write_u32(self.brain_atp_fraction_q16);
        digest.write_u16(self.completed_gpu_time_bucket);
        digest.write_u8(self.queue_depth_bucket);
        digest.write_u8(self.neural_heap_pressure_bucket);
        digest.write_u8(self.brain_atp_bucket);
        digest.finish256()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeuralThrottleDecision {
    pub schema_version: u16,
    pub policy_version: u16,
    pub organism_id_raw: u64,
    pub tick: u64,
    pub class_id_raw: u16,
    pub handle_slot: u32,
    pub handle_generation: u32,
    pub sequence_cursor: u64,
    pub dispatch_generation: u64,
    pub frame_digest: [u64; 4],
    pub level: NeuralThrottleLevel,
    pub pressure: GpuPressureSample,
    pub microsteps: u8,
    pub enabled_route_ids: Vec<u16>,
    pub pressure_digest: [u64; 4],
    pub route_schedule_digest: [u64; 4],
    pub policy_digest: [u64; 4],
    pub decision_digest: [u64; 4],
}

impl NeuralThrottleDecision {
    pub fn derive(
        policy: &BrainActivityPolicyV1,
        phenotype: &BrainPhenotype,
        execution: &BrainExecutionBudget,
        identity: BrainDispatchIdentity,
        pressure: GpuPressureSample,
    ) -> Result<Self, ScaffoldContractError> {
        policy.validate_contract()?;
        pressure.validate_for(policy)?;
        if identity != pressure.dispatch_identity()
            || identity.class_id_raw != phenotype.brain_class_id().raw()
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        let level = pressure.throttle_level();
        let (min_microsteps, max_microsteps) = execution.microstep_range();
        let configured = phenotype.microstep_count();
        if !(min_microsteps..=max_microsteps).contains(&configured) {
            return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
        }
        let microsteps = match level {
            NeuralThrottleLevel::Full => configured,
            NeuralThrottleLevel::Reduced => configured.saturating_sub(1).max(min_microsteps),
            NeuralThrottleLevel::EssentialOnly => min_microsteps,
        };
        let mut enabled_route_ids = Vec::with_capacity(phenotype.projections().len());
        for route in phenotype.projections() {
            let mandatory = Self::route_is_mandatory(route);
            if mandatory && route.priority() == BiologicalPriority::NonEssential {
                return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
            }
            let enabled = match level {
                NeuralThrottleLevel::Full => true,
                NeuralThrottleLevel::Reduced => {
                    route.priority() != BiologicalPriority::NonEssential
                }
                NeuralThrottleLevel::EssentialOnly => {
                    route.priority() == BiologicalPriority::Essential || mandatory
                }
            };
            if enabled {
                enabled_route_ids.push(route.route_index());
            }
        }
        enabled_route_ids.sort_unstable();
        enabled_route_ids.dedup();
        if enabled_route_ids.is_empty() {
            return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
        }
        let route_schedule_digest =
            route_schedule_digest(phenotype, microsteps, &enabled_route_ids);
        let mut decision = Self {
            schema_version: policy.schema_version,
            policy_version: policy.policy_version,
            organism_id_raw: identity.organism_id_raw,
            tick: identity.tick,
            class_id_raw: identity.class_id_raw,
            handle_slot: identity.handle_slot,
            handle_generation: identity.handle_generation,
            sequence_cursor: identity.sequence_cursor,
            dispatch_generation: identity.dispatch_generation,
            frame_digest: identity.frame_digest,
            level,
            pressure,
            microsteps,
            enabled_route_ids,
            pressure_digest: pressure.sample_digest,
            route_schedule_digest,
            policy_digest: policy.policy_digest,
            decision_digest: [0; 4],
        };
        decision.decision_digest = decision.recompute_digest();
        decision.validate_for(phenotype, execution)?;
        Ok(decision)
    }

    pub const fn route_is_mandatory(route: &CompiledProjection) -> bool {
        matches!(
            route.source_lobe(),
            LobeKind::SensoryGrounding | LobeKind::AuditorySpeech | LobeKind::GlyphVision
        ) || matches!(route.source_lobe(), LobeKind::HomeostaticRegulation)
            || matches!(
                route.target_lobe(),
                LobeKind::HomeostaticRegulation | LobeKind::MotorArbitration
            )
            || matches!(
                route.projection_type(),
                ProjectionType::MotorProposal
                    | ProjectionType::Homeostatic
                    | ProjectionType::LateralInhibition
            )
    }

    /// Returns the canonical route-schedule digest in the exact eight-word
    /// representation carried by GPU dispatch headers.
    pub const fn route_schedule_digest_words(&self) -> [u32; 8] {
        let digest = self.route_schedule_digest;
        [
            digest[0] as u32,
            (digest[0] >> 32) as u32,
            digest[1] as u32,
            (digest[1] >> 32) as u32,
            digest[2] as u32,
            (digest[2] >> 32) as u32,
            digest[3] as u32,
            (digest[3] >> 32) as u32,
        ]
    }

    pub fn validate_for(
        &self,
        phenotype: &BrainPhenotype,
        execution: &BrainExecutionBudget,
    ) -> Result<(), ScaffoldContractError> {
        let policy = BrainActivityPolicyV1::production_v1();
        self.pressure.validate_for(&policy)?;
        let identity = BrainDispatchIdentity {
            organism_id_raw: self.organism_id_raw,
            tick: self.tick,
            class_id_raw: self.class_id_raw,
            handle_slot: self.handle_slot,
            handle_generation: self.handle_generation,
            sequence_cursor: self.sequence_cursor,
            dispatch_generation: self.dispatch_generation,
            frame_digest: self.frame_digest,
        };
        identity.validate_contract()?;
        let expected =
            Self::derive_unchecked(&policy, phenotype, execution, identity, self.pressure)?;
        if self.schema_version != policy.schema_version
            || self.policy_version != policy.policy_version
            || self.pressure.dispatch_identity() != identity
            || self.pressure_digest != self.pressure.sample_digest
            || self.policy_digest != policy.policy_digest
            || self.level != expected.level
            || self.microsteps != expected.microsteps
            || self.enabled_route_ids != expected.enabled_route_ids
            || self.route_schedule_digest != expected.route_schedule_digest
            || self.decision_digest == [0; 4]
            || self.decision_digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        Ok(())
    }

    fn derive_unchecked(
        policy: &BrainActivityPolicyV1,
        phenotype: &BrainPhenotype,
        execution: &BrainExecutionBudget,
        identity: BrainDispatchIdentity,
        pressure: GpuPressureSample,
    ) -> Result<Self, ScaffoldContractError> {
        let level = pressure.throttle_level();
        let (min_microsteps, max_microsteps) = execution.microstep_range();
        let configured = phenotype.microstep_count();
        if !(min_microsteps..=max_microsteps).contains(&configured) {
            return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
        }
        let microsteps = match level {
            NeuralThrottleLevel::Full => configured,
            NeuralThrottleLevel::Reduced => configured.saturating_sub(1).max(min_microsteps),
            NeuralThrottleLevel::EssentialOnly => min_microsteps,
        };
        let mut enabled_route_ids = phenotype
            .projections()
            .iter()
            .filter_map(|route| {
                let mandatory = Self::route_is_mandatory(route);
                let enabled = match level {
                    NeuralThrottleLevel::Full => true,
                    NeuralThrottleLevel::Reduced => {
                        route.priority() != BiologicalPriority::NonEssential
                    }
                    NeuralThrottleLevel::EssentialOnly => {
                        route.priority() == BiologicalPriority::Essential || mandatory
                    }
                };
                enabled.then_some(route.route_index())
            })
            .collect::<Vec<_>>();
        enabled_route_ids.sort_unstable();
        enabled_route_ids.dedup();
        let route_schedule_digest =
            route_schedule_digest(phenotype, microsteps, &enabled_route_ids);
        let mut value = Self {
            schema_version: policy.schema_version,
            policy_version: policy.policy_version,
            organism_id_raw: identity.organism_id_raw,
            tick: identity.tick,
            class_id_raw: identity.class_id_raw,
            handle_slot: identity.handle_slot,
            handle_generation: identity.handle_generation,
            sequence_cursor: identity.sequence_cursor,
            dispatch_generation: identity.dispatch_generation,
            frame_digest: identity.frame_digest,
            level,
            pressure,
            microsteps,
            enabled_route_ids,
            pressure_digest: pressure.sample_digest,
            route_schedule_digest,
            policy_digest: policy.policy_digest,
            decision_digest: [0; 4],
        };
        value.decision_digest = value.recompute_digest();
        Ok(value)
    }

    pub fn validate_runtime_binding(
        &self,
        handle_slot: u32,
        handle_generation: u32,
    ) -> Result<(), ScaffoldContractError> {
        if self.handle_slot != handle_slot || self.handle_generation != handle_generation {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(DECISION_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.policy_version);
        digest.write_u64(self.organism_id_raw);
        digest.write_u64(self.tick);
        digest.write_u16(self.class_id_raw);
        digest.write_u32(self.handle_slot);
        digest.write_u32(self.handle_generation);
        digest.write_u64(self.sequence_cursor);
        digest.write_u64(self.dispatch_generation);
        write_digest4(&mut digest, self.frame_digest);
        digest.write_u8(self.level.raw());
        digest.write_u8(self.microsteps);
        digest.write_sequence_len(self.enabled_route_ids.len());
        for route in &self.enabled_route_ids {
            digest.write_u16(*route);
        }
        write_digest4(&mut digest, self.pressure_digest);
        write_digest4(&mut digest, self.route_schedule_digest);
        write_digest4(&mut digest, self.policy_digest);
        digest.finish256()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrainWorkReceipt {
    pub schema_version: u16,
    pub class_id_raw: u16,
    pub organism_id_raw: u64,
    pub tick: u64,
    pub handle_slot: u32,
    pub handle_generation: u32,
    pub dispatch_generation: u64,
    pub frame_digest: [u64; 4],
    pub sequence_cursor: u64,
    pub counters: BrainWorkCounters,
    pub route_schedule_digest: [u64; 4],
    pub neural_cost_q24: u64,
    pub atp_before_q16: u32,
    pub atp_debit_q16: u32,
    pub atp_after_q16: u32,
    pub receipt_digest: [u64; 4],
}

impl BrainWorkReceipt {
    pub fn try_new(
        policy: &BrainActivityPolicyV1,
        decision: &NeuralThrottleDecision,
        counters: BrainWorkCounters,
        atp_before_q16: u32,
    ) -> Result<Self, ScaffoldContractError> {
        policy.validate_contract()?;
        if counters.microsteps != u32::from(decision.microsteps)
            || atp_before_q16 > BRAIN_ATP_Q16_MAX
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        let neural_cost_q24 = policy.cost.neural_cost_q24(&counters)?;
        let atp_debit_q16 = policy.cost.q24_to_atp_q16_round_half_up(neural_cost_q24)?;
        let atp_after_q16 = atp_before_q16
            .checked_sub(atp_debit_q16)
            .ok_or(ScaffoldContractError::BrainAtpExhausted)?;
        let mut receipt = Self {
            schema_version: policy.schema_version,
            class_id_raw: decision.class_id_raw,
            organism_id_raw: decision.organism_id_raw,
            tick: decision.tick,
            handle_slot: decision.handle_slot,
            handle_generation: decision.handle_generation,
            dispatch_generation: decision.dispatch_generation,
            frame_digest: decision.frame_digest,
            sequence_cursor: decision.sequence_cursor,
            counters,
            route_schedule_digest: decision.route_schedule_digest,
            neural_cost_q24,
            atp_before_q16,
            atp_debit_q16,
            atp_after_q16,
            receipt_digest: [0; 4],
        };
        receipt.receipt_digest = receipt.recompute_digest();
        receipt.validate_for(policy, decision)?;
        Ok(receipt)
    }

    pub fn validate_for(
        &self,
        policy: &BrainActivityPolicyV1,
        decision: &NeuralThrottleDecision,
    ) -> Result<(), ScaffoldContractError> {
        policy.validate_contract()?;
        let expected_cost = policy.cost.neural_cost_q24(&self.counters)?;
        let expected_debit = policy.cost.q24_to_atp_q16_round_half_up(expected_cost)?;
        let expected_after = self.atp_before_q16.checked_sub(expected_debit);
        if self.schema_version != policy.schema_version
            || self.class_id_raw != decision.class_id_raw
            || self.organism_id_raw != decision.organism_id_raw
            || self.tick != decision.tick
            || self.handle_slot != decision.handle_slot
            || self.handle_generation != decision.handle_generation
            || self.dispatch_generation != decision.dispatch_generation
            || self.frame_digest != decision.frame_digest
            || self.sequence_cursor != decision.sequence_cursor
            || self.counters.microsteps != u32::from(decision.microsteps)
            || self.route_schedule_digest != decision.route_schedule_digest
            || self.neural_cost_q24 != expected_cost
            || self.atp_debit_q16 != expected_debit
            || expected_after != Some(self.atp_after_q16)
            || self.receipt_digest == [0; 4]
            || self.receipt_digest != self.recompute_digest()
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        Ok(())
    }

    pub fn validate_runtime_binding(
        &self,
        handle_slot: u32,
        handle_generation: u32,
    ) -> Result<(), ScaffoldContractError> {
        if self.handle_slot != handle_slot || self.handle_generation != handle_generation {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        Ok(())
    }

    fn recompute_digest(&self) -> [u64; 4] {
        let mut digest = CanonicalDigestBuilder::new(WORK_RECEIPT_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.class_id_raw);
        digest.write_u64(self.organism_id_raw);
        digest.write_u64(self.tick);
        digest.write_u32(self.handle_slot);
        digest.write_u32(self.handle_generation);
        digest.write_u64(self.dispatch_generation);
        write_digest4(&mut digest, self.frame_digest);
        digest.write_u64(self.sequence_cursor);
        digest.write_u32(self.counters.microsteps);
        digest.write_u64(self.counters.neuron_updates);
        digest.write_u64(self.counters.tile_visits);
        digest.write_u64(self.counters.synapse_ops);
        digest.write_u64(self.counters.decoder_candidate_ops);
        digest.write_u64(self.counters.memory_context_ops);
        write_digest4(&mut digest, self.route_schedule_digest);
        digest.write_u64(self.neural_cost_q24);
        digest.write_u32(self.atp_before_q16);
        digest.write_u32(self.atp_debit_q16);
        digest.write_u32(self.atp_after_q16);
        digest.finish256()
    }
}

fn route_schedule_digest(
    phenotype: &BrainPhenotype,
    microsteps: u8,
    enabled_route_ids: &[u16],
) -> [u64; 4] {
    let mut digest = ROUTE_DIGEST_SEEDS;
    for word in phenotype
        .phenotype_hash()
        .0
        .into_iter()
        .flat_map(|word| [word as u32, (word >> 32) as u32])
    {
        mix_route_digest_word(&mut digest, word);
    }
    mix_route_digest_word(&mut digest, u32::from(microsteps));
    mix_route_digest_word(
        &mut digest,
        u32::try_from(enabled_route_ids.len()).unwrap_or(u32::MAX),
    );
    for route in enabled_route_ids {
        mix_route_digest_word(&mut digest, u32::from(*route));
    }
    [
        u64::from(digest[0]) | (u64::from(digest[1]) << 32),
        u64::from(digest[2]) | (u64::from(digest[3]) << 32),
        u64::from(digest[4]) | (u64::from(digest[5]) << 32),
        u64::from(digest[6]) | (u64::from(digest[7]) << 32),
    ]
}

fn mix_route_digest_word(digest: &mut [u32; 8], word: u32) {
    for (lane, value) in digest.iter_mut().enumerate() {
        *value =
            (value.wrapping_add(ROUTE_DIGEST_SALTS[lane]) ^ word).wrapping_mul(ROUTE_DIGEST_PRIME);
    }
}

fn q16_fraction(numerator: u64, denominator: u64) -> Result<u32, ScaffoldContractError> {
    if denominator == 0 || numerator > denominator {
        return Err(ScaffoldContractError::BrainActivityPolicyMismatch);
    }
    let scaled = u128::from(numerator)
        .checked_shl(16)
        .ok_or(ScaffoldContractError::BrainActivityPolicyMismatch)?
        / u128::from(denominator);
    u32::try_from(scaled.min(u128::from(BRAIN_ATP_Q16_MAX)))
        .map_err(|_| ScaffoldContractError::BrainActivityPolicyMismatch)
}

fn threshold_bucket_u64(value: u64, thresholds: [u64; 3]) -> u8 {
    if value < thresholds[0] {
        0
    } else if value < thresholds[1] {
        1
    } else if value < thresholds[2] {
        2
    } else {
        3
    }
}

fn threshold_bucket_u32(value: u32, thresholds: [u32; 3]) -> u8 {
    if value < thresholds[0] {
        0
    } else if value < thresholds[1] {
        1
    } else if value < thresholds[2] {
        2
    } else {
        3
    }
}

fn descending_atp_bucket(value: u32, thresholds: [u32; 3]) -> u8 {
    if value >= thresholds[0] {
        0
    } else if value >= thresholds[1] {
        1
    } else if value >= thresholds[2] {
        2
    } else {
        3
    }
}

fn write_digest4(digest: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    for word in words {
        digest.write_u64(word);
    }
}
