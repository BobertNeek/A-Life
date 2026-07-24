//! Strict portable GPU provenance, throttle replay, and inspection-only records.
//!
//! These records intentionally contain no live GPU handles, packed offsets,
//! wgpu objects, execution availability cache, or hidden policy-substitution state.

use alife_core::{
    BrainActivityPolicyV1, BrainCapacityClass, BrainClassId, BrainWorkCounters,
    CanonicalDigestBuilder, NeuralThrottleLevel, ScaffoldContractError, BRAIN_ATP_Q16_MAX,
};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

use super::{GpuBrainAssetRef, PersistenceError};

pub const GPU_BACKEND_PROVENANCE_SAVE_SCHEMA_VERSION: u16 = 1;
pub const PORTABLE_THROTTLE_CHECKPOINT_SCHEMA_VERSION: u16 = 1;
pub const THROTTLE_REPLAY_SAVE_SCHEMA_VERSION: u16 = 1;
pub const INSPECTION_ONLY_BRAIN_SCHEMA_VERSION: u16 = 1;

pub const NEURAL_AVAILABILITY_REASON_UNSUPPORTED_CLASS: u16 = 1;
pub const NEURAL_AVAILABILITY_REASON_REQUIRED_FEATURE_MISSING: u16 = 2;
pub const NEURAL_AVAILABILITY_REASON_REQUIRED_LIMIT_MISSING: u16 = 3;
pub const NEURAL_AVAILABILITY_REASON_ADMISSION_EXCEEDED: u16 = 4;
pub const NEURAL_AVAILABILITY_REASON_DEVICE_UNAVAILABLE: u16 = 5;

const PORTABLE_PROVENANCE_DOMAIN: &[u8] = b"alife.gpu.backend.portable-compatibility.v1";
const SAME_ADAPTER_DOMAIN: &[u8] = b"alife.gpu.backend.same-adapter.v1";
const THROTTLE_CHECKPOINT_DOMAIN: &[u8] = b"alife.gpu.throttle-checkpoint.v1";
const THROTTLE_SEQUENCE_DOMAIN: &[u8] = b"alife.gpu.throttle-sequence.v1";
const INSPECTION_ONLY_DOMAIN: &[u8] = b"alife.gpu.inspection-only.v1";

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NeuralGpuBackendApi {
    Vulkan = 1,
}

impl NeuralGpuBackendApi {
    pub const fn raw(self) -> u16 {
        self as u16
    }

    pub fn try_from_raw(raw: u16) -> Result<Self, ScaffoldContractError> {
        match raw {
            1 => Ok(Self::Vulkan),
            _ => Err(ScaffoldContractError::NeuralBackendUnavailable),
        }
    }

    pub fn try_from_slug(slug: &str) -> Result<Self, ScaffoldContractError> {
        match slug {
            "vulkan" => Ok(Self::Vulkan),
            _ => Err(ScaffoldContractError::NeuralBackendUnavailable),
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Vulkan => "vulkan",
        }
    }
}

impl Serialize for NeuralGpuBackendApi {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(self.raw())
    }
}

impl<'de> Deserialize<'de> for NeuralGpuBackendApi {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_from_raw(u16::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuBackendProvenanceSave {
    pub schema_version: u16,
    pub backend_api_raw: u16,
    pub vendor_id: u32,
    pub device_id: u32,
    pub backend_version_major: u16,
    pub backend_version_minor: u16,
    pub backend_version_patch: u16,
    pub adapter_name_len: u16,
    #[serde(with = "fixed_bytes_128")]
    pub adapter_name_utf8: [u8; 128],
    pub driver_digest: [u64; 4],
    pub required_features_digest: [u64; 4],
    pub required_limits_digest: [u64; 4],
    pub available_features_digest: [u64; 4],
    pub adapter_limits_digest: [u64; 4],
}

impl GpuBackendProvenanceSave {
    pub fn set_adapter_name(&mut self, adapter_name: &str) -> Result<(), PersistenceError> {
        let bytes = adapter_name.as_bytes();
        if bytes.len() > self.adapter_name_utf8.len() {
            return Err(invalid_provenance());
        }
        self.adapter_name_utf8.fill(0);
        self.adapter_name_utf8[..bytes.len()].copy_from_slice(bytes);
        self.adapter_name_len = u16::try_from(bytes.len()).map_err(|_| invalid_provenance())?;
        self.validate()
    }

    pub fn adapter_name(&self) -> Result<&str, PersistenceError> {
        self.validate()?;
        let len = usize::from(self.adapter_name_len);
        std::str::from_utf8(&self.adapter_name_utf8[..len]).map_err(|_| invalid_provenance())
    }

    pub fn validate(&self) -> Result<(), PersistenceError> {
        let len = usize::from(self.adapter_name_len);
        if self.schema_version != GPU_BACKEND_PROVENANCE_SAVE_SCHEMA_VERSION
            || NeuralGpuBackendApi::try_from_raw(self.backend_api_raw).is_err()
            || len > self.adapter_name_utf8.len()
            || std::str::from_utf8(&self.adapter_name_utf8[..len]).is_err()
            || self.adapter_name_utf8[len..].iter().any(|byte| *byte != 0)
            || self.driver_digest == [0; 4]
            || self.required_features_digest == [0; 4]
            || self.required_limits_digest == [0; 4]
            || self.available_features_digest == [0; 4]
            || self.adapter_limits_digest == [0; 4]
        {
            return Err(invalid_provenance());
        }
        Ok(())
    }

    /// Compatibility required for an ordinary restore. Human display name,
    /// adapter identity, and currently available capabilities are provenance,
    /// not portable phenotype identity.
    pub fn portable_compatibility_digest(&self) -> Result<[u64; 4], PersistenceError> {
        self.validate()?;
        let mut digest = CanonicalDigestBuilder::new(PORTABLE_PROVENANCE_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.backend_api_raw);
        digest.write_u16(self.backend_version_major);
        digest.write_u16(self.backend_version_minor);
        digest.write_u16(self.backend_version_patch);
        write_digest4(&mut digest, self.required_features_digest);
        write_digest4(&mut digest, self.required_limits_digest);
        Ok(digest.finish256())
    }

    /// Strong adapter identity used only for same-adapter replay claims. The
    /// display name remains excluded because drivers may rename the adapter.
    pub fn same_adapter_digest(&self) -> Result<[u64; 4], PersistenceError> {
        self.validate()?;
        let mut digest = CanonicalDigestBuilder::new(SAME_ADAPTER_DOMAIN);
        write_digest4(&mut digest, self.portable_compatibility_digest()?);
        digest.write_u32(self.vendor_id);
        digest.write_u32(self.device_id);
        write_digest4(&mut digest, self.driver_digest);
        write_digest4(&mut digest, self.available_features_digest);
        write_digest4(&mut digest, self.adapter_limits_digest);
        Ok(digest.finish256())
    }

    pub fn validate_portable_restore_against(
        &self,
        current: &Self,
    ) -> Result<(), PersistenceError> {
        if self.portable_compatibility_digest()? != current.portable_compatibility_digest()? {
            return Err(invalid_provenance());
        }
        Ok(())
    }

    pub fn validate_same_adapter_replay_against(
        &self,
        current: &Self,
    ) -> Result<(), PersistenceError> {
        if self.same_adapter_digest()? != current.same_adapter_digest()? {
            return Err(invalid_provenance());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PortableThrottleCheckpoint {
    pub schema_version: u16,
    pub policy_version: u16,
    pub organism_id_raw: u64,
    pub tick: u64,
    pub class_id_raw: u16,
    pub sequence_cursor: u64,
    pub dispatch_generation: u64,
    pub frame_digest: [u64; 4],
    pub source_dispatch_generation: u64,
    pub source_frame_digest: [u64; 4],
    pub completed_gpu_time_ns: u64,
    pub queue_depth: u32,
    pub logical_heap_pressure_q16: u32,
    pub brain_atp_fraction_q16: u32,
    pub level: NeuralThrottleLevel,
    pub microsteps: u8,
    pub enabled_route_ids: Vec<u16>,
    pub route_schedule_digest: [u64; 4],
    pub work: BrainWorkCounters,
    pub neural_cost_q24: u64,
    pub atp_before_q16: u32,
    pub atp_debit_q16: u32,
    pub atp_after_q16: u32,
    pub policy_digest: [u64; 4],
    pub portable_digest: [u64; 4],
}

impl PortableThrottleCheckpoint {
    pub fn seal(mut self) -> Result<Self, PersistenceError> {
        self.portable_digest = self.recompute_digest()?;
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), PersistenceError> {
        let policy = BrainActivityPolicyV1::production_v1();
        policy.validate_contract()?;
        let expected_cost = policy.cost.neural_cost_q24(&self.work)?;
        let expected_debit = policy.cost.q24_to_atp_q16_round_half_up(expected_cost)?;
        let expected_level = throttle_level_for_pressure(
            &policy,
            self.completed_gpu_time_ns,
            self.queue_depth,
            self.logical_heap_pressure_q16,
            self.brain_atp_fraction_q16,
        );
        if self.schema_version != PORTABLE_THROTTLE_CHECKPOINT_SCHEMA_VERSION
            || self.policy_version != policy.policy_version
            || self.policy_digest != policy.policy_digest
            || self.organism_id_raw == 0
            || BrainCapacityClass::production_for_id(BrainClassId(self.class_id_raw)).is_err()
            || self.sequence_cursor == 0
            || self.dispatch_generation == 0
            || self.frame_digest == [0; 4]
            || self.source_dispatch_generation >= self.dispatch_generation
            || ((self.source_dispatch_generation == 0) != (self.source_frame_digest == [0; 4]))
            || self.logical_heap_pressure_q16 > BRAIN_ATP_Q16_MAX
            || self.brain_atp_fraction_q16 > BRAIN_ATP_Q16_MAX
            || self.level != expected_level
            || self.microsteps == 0
            || self.work.microsteps != u32::from(self.microsteps)
            || self.enabled_route_ids.is_empty()
            || !self
                .enabled_route_ids
                .windows(2)
                .all(|pair| pair[0] < pair[1])
            || self.route_schedule_digest == [0; 4]
            || self.neural_cost_q24 != expected_cost
            || self.atp_debit_q16 != expected_debit
            || self.atp_before_q16.checked_sub(expected_debit) != Some(self.atp_after_q16)
            || self.portable_digest == [0; 4]
            || self.portable_digest != self.recompute_digest()?
        {
            return Err(activity_sequence_error());
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], PersistenceError> {
        let mut digest = CanonicalDigestBuilder::new(THROTTLE_CHECKPOINT_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.policy_version);
        digest.write_u64(self.organism_id_raw);
        digest.write_u64(self.tick);
        digest.write_u16(self.class_id_raw);
        digest.write_u64(self.sequence_cursor);
        digest.write_u64(self.dispatch_generation);
        write_digest4(&mut digest, self.frame_digest);
        digest.write_u64(self.source_dispatch_generation);
        write_digest4(&mut digest, self.source_frame_digest);
        digest.write_u64(self.completed_gpu_time_ns);
        digest.write_u32(self.queue_depth);
        digest.write_u32(self.logical_heap_pressure_q16);
        digest.write_u32(self.brain_atp_fraction_q16);
        digest.write_u8(self.level.raw());
        digest.write_u8(self.microsteps);
        digest.write_sequence_len(self.enabled_route_ids.len());
        for route in &self.enabled_route_ids {
            digest.write_u16(*route);
        }
        write_digest4(&mut digest, self.route_schedule_digest);
        digest.write_u32(self.work.microsteps);
        digest.write_u64(self.work.neuron_updates);
        digest.write_u64(self.work.tile_visits);
        digest.write_u64(self.work.synapse_ops);
        digest.write_u64(self.work.decoder_candidate_ops);
        digest.write_u64(self.work.memory_context_ops);
        digest.write_u64(self.neural_cost_q24);
        digest.write_u32(self.atp_before_q16);
        digest.write_u32(self.atp_debit_q16);
        digest.write_u32(self.atp_after_q16);
        write_digest4(&mut digest, self.policy_digest);
        Ok(digest.finish256())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThrottleReplaySaveState {
    pub schema_version: u16,
    pub policy_version: u16,
    pub next_sequence_cursor: u64,
    pub last_committed_sequence_cursor: Option<u64>,
    pub policy_digest: [u64; 4],
    pub sequence_digest: [u64; 4],
    pub sequence_asset: GpuBrainAssetRef,
    pub last_checkpoint: Option<PortableThrottleCheckpoint>,
    pub next_completed_gpu_time_ns: u64,
    pub brain_atp_q16: u32,
    pub last_world_atp_tick: Option<u64>,
}

/// Exact host-owned activity continuation fields captured at a sealed boundary.
///
/// The previous checkpoint records the pressure sample that selected the last
/// committed schedule. These fields separately retain the completed duration
/// that will drive the *next* schedule, current BrainATP after any no-dispatch
/// world ticks, and the basal-charge cursor. None is a live GPU identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThrottleReplaySaveInput {
    pub schema_version: u16,
    pub policy_version: u16,
    pub next_sequence_cursor: u64,
    pub last_committed_sequence_cursor: Option<u64>,
    pub policy_digest: [u64; 4],
    pub next_completed_gpu_time_ns: u64,
    pub brain_atp_q16: u32,
    pub last_world_atp_tick: Option<u64>,
}

impl ThrottleReplaySaveState {
    pub fn try_new(
        input: ThrottleReplaySaveInput,
        sequence_asset: GpuBrainAssetRef,
        last_checkpoint: Option<PortableThrottleCheckpoint>,
    ) -> Result<Self, PersistenceError> {
        let mut value = Self {
            schema_version: input.schema_version,
            policy_version: input.policy_version,
            next_sequence_cursor: input.next_sequence_cursor,
            last_committed_sequence_cursor: input.last_committed_sequence_cursor,
            policy_digest: input.policy_digest,
            sequence_digest: [0; 4],
            sequence_asset,
            last_checkpoint,
            next_completed_gpu_time_ns: input.next_completed_gpu_time_ns,
            brain_atp_q16: input.brain_atp_q16,
            last_world_atp_tick: input.last_world_atp_tick,
        };
        value.sequence_digest = value.recompute_digest()?;
        value.validate()?;
        Ok(value)
    }

    pub fn bootstrap(sequence_asset: GpuBrainAssetRef) -> Result<Self, PersistenceError> {
        let policy = BrainActivityPolicyV1::production_v1();
        Self::try_new(
            ThrottleReplaySaveInput {
                schema_version: THROTTLE_REPLAY_SAVE_SCHEMA_VERSION,
                policy_version: policy.policy_version,
                next_sequence_cursor: 1,
                last_committed_sequence_cursor: None,
                policy_digest: policy.policy_digest,
                next_completed_gpu_time_ns: 0,
                brain_atp_q16: BRAIN_ATP_Q16_MAX,
                last_world_atp_tick: None,
            },
            sequence_asset,
            None,
        )
    }

    pub fn validate(&self) -> Result<(), PersistenceError> {
        let policy = BrainActivityPolicyV1::production_v1();
        self.sequence_asset.validate()?;
        let checkpoint_valid = match (&self.last_checkpoint, self.last_committed_sequence_cursor) {
            (None, None) => self.next_sequence_cursor == 1,
            (Some(checkpoint), Some(last)) => {
                checkpoint.validate().is_ok()
                    && last == checkpoint.sequence_cursor
                    && self.next_sequence_cursor == last.checked_add(1).unwrap_or(0)
                    && checkpoint.policy_version == self.policy_version
                    && checkpoint.policy_digest == self.policy_digest
            }
            _ => false,
        };
        if self.schema_version != THROTTLE_REPLAY_SAVE_SCHEMA_VERSION
            || self.policy_version != policy.policy_version
            || self.policy_digest != policy.policy_digest
            || self.next_sequence_cursor == 0
            || self.brain_atp_q16 > BRAIN_ATP_Q16_MAX
            || (self.last_checkpoint.is_none() && self.next_completed_gpu_time_ns != 0)
            || !checkpoint_valid
            || self.sequence_digest == [0; 4]
            || self.sequence_digest != self.recompute_digest()?
        {
            return Err(activity_sequence_error());
        }
        Ok(())
    }

    pub fn validate_next_dispatch(
        &self,
        organism_id_raw: u64,
        class_id_raw: u16,
        sequence_cursor: u64,
        policy_version: u16,
        policy_digest: [u64; 4],
    ) -> Result<(), PersistenceError> {
        self.validate()?;
        let checkpoint_binding_matches = self.last_checkpoint.as_ref().is_none_or(|checkpoint| {
            checkpoint.organism_id_raw == organism_id_raw && checkpoint.class_id_raw == class_id_raw
        });
        if organism_id_raw == 0
            || BrainCapacityClass::production_for_id(BrainClassId(class_id_raw)).is_err()
            || sequence_cursor != self.next_sequence_cursor
            || policy_version != self.policy_version
            || policy_digest != self.policy_digest
            || !checkpoint_binding_matches
        {
            return Err(activity_sequence_error());
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], PersistenceError> {
        self.sequence_asset.validate()?;
        let mut digest = CanonicalDigestBuilder::new(THROTTLE_SEQUENCE_DOMAIN);
        digest.write_u16(self.schema_version);
        digest.write_u16(self.policy_version);
        digest.write_u64(self.next_sequence_cursor);
        digest.write_u8(u8::from(self.last_committed_sequence_cursor.is_some()));
        if let Some(cursor) = self.last_committed_sequence_cursor {
            digest.write_u64(cursor);
        }
        write_digest4(&mut digest, self.policy_digest);
        digest.write_utf8(&self.sequence_asset.asset_id);
        digest.write_utf8(&self.sequence_asset.digest.0);
        digest.write_u8(u8::from(self.last_checkpoint.is_some()));
        if let Some(checkpoint) = &self.last_checkpoint {
            write_digest4(&mut digest, checkpoint.portable_digest);
        }
        digest.write_u64(self.next_completed_gpu_time_ns);
        digest.write_u32(self.brain_atp_q16);
        digest.write_u8(u8::from(self.last_world_atp_tick.is_some()));
        if let Some(tick) = self.last_world_atp_tick {
            digest.write_u64(tick);
        }
        Ok(digest.finish256())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionNeuralAvailability {
    ReadyGpu {
        class_id_raw: u16,
        runtime_profile_digest: [u64; 4],
        adapter_limits_digest: [u64; 4],
    },
    InspectionOnly {
        legacy_class_id_raw: u16,
        reason_code: u16,
    },
    Unavailable {
        class_id_raw: u16,
        reason_code: u16,
    },
}

impl ProductionNeuralAvailability {
    pub fn for_saved_class(
        class_id: BrainClassId,
        runtime_profile_digest: [u64; 4],
        adapter_limits_digest: [u64; 4],
    ) -> Result<Self, ScaffoldContractError> {
        match class_id.raw() {
            1..=3 => Ok(if runtime_profile_digest == [0; 4] {
                Self::Unavailable {
                    class_id_raw: class_id.raw(),
                    reason_code: NEURAL_AVAILABILITY_REASON_ADMISSION_EXCEEDED,
                }
            } else if adapter_limits_digest == [0; 4] {
                Self::Unavailable {
                    class_id_raw: class_id.raw(),
                    reason_code: NEURAL_AVAILABILITY_REASON_REQUIRED_LIMIT_MISSING,
                }
            } else {
                Self::ReadyGpu {
                    class_id_raw: class_id.raw(),
                    runtime_profile_digest,
                    adapter_limits_digest,
                }
            }),
            4..=8 => Ok(Self::InspectionOnly {
                legacy_class_id_raw: class_id.raw(),
                reason_code: NEURAL_AVAILABILITY_REASON_UNSUPPORTED_CLASS,
            }),
            _ => Err(ScaffoldContractError::UnsupportedProductionBrainClass),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionOnlyLegacyBrainState {
    pub source_schema: u16,
    pub legacy_class_id_raw: u16,
    pub raw_brain_asset: GpuBrainAssetRef,
    pub inspection_reason_code: u16,
    pub canonical_digest: [u64; 4],
}

impl InspectionOnlyLegacyBrainState {
    pub fn try_new(
        source_schema: u16,
        legacy_class_id_raw: u16,
        raw_brain_asset: GpuBrainAssetRef,
        inspection_reason_code: u16,
    ) -> Result<Self, PersistenceError> {
        let mut value = Self {
            source_schema,
            legacy_class_id_raw,
            raw_brain_asset,
            inspection_reason_code,
            canonical_digest: [0; 4],
        };
        value.canonical_digest = value.recompute_digest()?;
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), PersistenceError> {
        self.raw_brain_asset.validate()?;
        if self.source_schema != INSPECTION_ONLY_BRAIN_SCHEMA_VERSION
            || !(4..=8).contains(&self.legacy_class_id_raw)
            || self.inspection_reason_code != NEURAL_AVAILABILITY_REASON_UNSUPPORTED_CLASS
            || self.canonical_digest == [0; 4]
            || self.canonical_digest != self.recompute_digest()?
        {
            return Err(PersistenceError::Contract(
                ScaffoldContractError::UnsupportedProductionBrainClass,
            ));
        }
        Ok(())
    }

    fn recompute_digest(&self) -> Result<[u64; 4], PersistenceError> {
        self.raw_brain_asset.validate()?;
        let mut digest = CanonicalDigestBuilder::new(INSPECTION_ONLY_DOMAIN);
        digest.write_u16(self.source_schema);
        digest.write_u16(self.legacy_class_id_raw);
        digest.write_utf8(&self.raw_brain_asset.asset_id);
        digest.write_utf8(&self.raw_brain_asset.digest.0);
        digest.write_u16(self.inspection_reason_code);
        Ok(digest.finish256())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionOnlyLegacyBrainLoad {
    pub state: InspectionOnlyLegacyBrainState,
    pub availability: ProductionNeuralAvailability,
    pub phenotype_compile_count: u64,
    pub gpu_admission_count: u64,
}

pub fn load_legacy_large_tier_for_inspection(
    state: InspectionOnlyLegacyBrainState,
) -> Result<InspectionOnlyLegacyBrainLoad, PersistenceError> {
    state.validate()?;
    let availability = ProductionNeuralAvailability::for_saved_class(
        BrainClassId(state.legacy_class_id_raw),
        [1; 4],
        [1; 4],
    )?;
    if !matches!(
        availability,
        ProductionNeuralAvailability::InspectionOnly { .. }
    ) {
        return Err(PersistenceError::Contract(
            ScaffoldContractError::UnsupportedProductionBrainClass,
        ));
    }
    Ok(InspectionOnlyLegacyBrainLoad {
        state,
        availability,
        phenotype_compile_count: 0,
        gpu_admission_count: 0,
    })
}

fn throttle_level_for_pressure(
    policy: &BrainActivityPolicyV1,
    completed_gpu_time_ns: u64,
    queue_depth: u32,
    logical_heap_pressure_q16: u32,
    brain_atp_fraction_q16: u32,
) -> NeuralThrottleLevel {
    let time = bucket_u64(completed_gpu_time_ns, policy.gpu_time_threshold_ns);
    let queue = bucket_u32(queue_depth, policy.queue_depth_thresholds);
    let heap = bucket_u32(
        logical_heap_pressure_q16,
        policy.logical_heap_pressure_thresholds_q16,
    );
    let atp = if brain_atp_fraction_q16 >= policy.atp_remaining_thresholds_q16_desc[0] {
        0
    } else if brain_atp_fraction_q16 >= policy.atp_remaining_thresholds_q16_desc[1] {
        1
    } else if brain_atp_fraction_q16 >= policy.atp_remaining_thresholds_q16_desc[2] {
        2
    } else {
        3
    };
    match time.max(queue).max(heap).max(atp) {
        0 => NeuralThrottleLevel::Full,
        1 => NeuralThrottleLevel::Reduced,
        _ => NeuralThrottleLevel::EssentialOnly,
    }
}

fn bucket_u64(value: u64, thresholds: [u64; 3]) -> u8 {
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

fn bucket_u32(value: u32, thresholds: [u32; 3]) -> u8 {
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

fn invalid_provenance() -> PersistenceError {
    PersistenceError::Contract(ScaffoldContractError::NeuralBackendUnavailable)
}

fn activity_sequence_error() -> PersistenceError {
    PersistenceError::Contract(ScaffoldContractError::BrainActivitySequenceMismatch)
}

fn write_digest4(digest: &mut CanonicalDigestBuilder, words: [u64; 4]) {
    for word in words {
        digest.write_u64(word);
    }
}

mod fixed_bytes_128 {
    use serde::{de::Error as _, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 128], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(bytes)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 128], D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        bytes
            .try_into()
            .map_err(|bytes: Vec<u8>| D::Error::invalid_length(bytes.len(), &"128 bytes"))
    }
}
