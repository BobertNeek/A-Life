//! Shared required-GPU ownership and evidence contracts for the closed loop.
//!
//! The world supplies current perception and unscored candidates. This module
//! owns the one authoritative device, fixed class arenas, generation-checked
//! capabilities, bounded selection readback, and fail-stop transaction state.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use alife_core::{
    BrainActivityPolicyV1, BrainCapacityClass, BrainClassId, BrainDispatchIdentity, BrainPhenotype,
    BrainWorkCounters, BrainWorkReceipt, CanonicalDigestBuilder, Confidence, ExperiencePatch,
    FinalizedMemoryRecall, GpuPressureSample, GpuPressureSampleInput, LearningCommitToken,
    LearningSequenceGuard, NeuralActionSelection, NeuralThrottleDecision, NeuralThrottleLevel,
    OrganismId, OutcomeCreditPacket, PerceptionBaseDigest, PerceptionFrame, PerceptionFrameDigest,
    PhenotypeHash, ScaffoldContractError, SensorProfile, BRAIN_ATP_BASAL_DEBIT_Q16,
    BRAIN_ATP_Q16_MAX, BRAIN_ATP_SLEEP_RECOVERY_Q16, REQUIRED_GPU_FEATURE_MASK,
};
use serde::{Deserialize, Serialize};

use crate::{
    derive_executed_work, GpuActiveBatchUpload, GpuAdmissionReceipt, GpuAllocationEventKind,
    GpuAllocationEventReceipt, GpuBrainSlot, GpuClosedLoopError, GpuClosedLoopKernelSet,
    GpuClosedLoopPipelines, GpuCompactMapTicket, GpuFastPlasticityBatchEntry,
    GpuFixedActiveBatchEntry, GpuFixedClassArenaBuffers, GpuFixedClassArenaPlan,
    GpuFixedSlotRanges, GpuLearningReceipt, GpuMemoryContextDispatchReceipt,
    GpuMemoryContextUpload, GpuOutcomeCreditRecord, GpuPendingEligibilityRecord,
    GpuPerceptionUpload, GpuPreparedActiveBatch, GpuRuntimeBudget, GpuRuntimeProfile,
    GpuTimestampQueryResources, GpuValidatedClassBatch, PendingEligibilityDiscardReceipt,
    PendingEligibilityIdentity, PendingEligibilityReceipt, GPU_CLOSED_LOOP_LAYOUT_VERSION,
};

pub const GPU_HARDWARE_RECEIPT_SCHEMA_VERSION: u16 = 1;
pub const GPU_DRIVER_DIGEST_DOMAIN: &[u8] = b"alife.gpu.hardware.driver.v1";
pub const GPU_FEATURE_DIGEST_DOMAIN: &[u8] = b"alife.gpu.hardware.features.v1";
pub const GPU_LIMITS_DIGEST_DOMAIN: &[u8] = b"alife.gpu.hardware.limits.v1";

const BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");
static NEXT_BACKEND_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_HARDWARE_RECEIPT_GENERATION: AtomicU64 = AtomicU64::new(1);
const GPU_TIMESTAMP_QUERY_COUNT: u32 = 2;
const GPU_TIMESTAMP_READBACK_BYTES: u64 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExactGpuTimestampPeriod {
    significand: u32,
    binary_exponent: i16,
}

impl ExactGpuTimestampPeriod {
    fn try_from_f32_bits(bits: u32) -> Result<Self, ScaffoldContractError> {
        let sign = bits >> 31;
        let exponent_bits = (bits >> 23) & 0xff;
        let mantissa = bits & 0x7f_ffff;
        if sign != 0 || exponent_bits == 0xff || (exponent_bits == 0 && mantissa == 0) {
            return Err(ScaffoldContractError::GpuTimestampQueryUnavailable);
        }
        let (significand, binary_exponent) = if exponent_bits == 0 {
            (mantissa, -149)
        } else {
            (
                (1 << 23) | mantissa,
                i16::try_from(exponent_bits)
                    .map_err(|_| ScaffoldContractError::GpuTimestampQueryUnavailable)?
                    - 127
                    - 23,
            )
        };
        Ok(Self {
            significand,
            binary_exponent,
        })
    }

    fn elapsed_ns(self, begin: u64, end: u64) -> Result<u64, ScaffoldContractError> {
        let ticks = self.delta_ticks(begin, end)?;
        let scaled = u128::from(ticks)
            .checked_mul(u128::from(self.significand))
            .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?;
        let nanoseconds = if self.binary_exponent >= 0 {
            scaled
                .checked_shl(u32::from(self.binary_exponent.unsigned_abs()))
                .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?
        } else {
            let shift = u32::from(self.binary_exponent.unsigned_abs());
            if shift >= u128::BITS {
                1
            } else {
                let quotient = scaled >> shift;
                let remainder_mask = (1_u128 << shift) - 1;
                quotient
                    .checked_add(u128::from(scaled & remainder_mask != 0))
                    .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?
            }
        };
        u64::try_from(nanoseconds).map_err(|_| ScaffoldContractError::GpuTimestampQueryUnavailable)
    }

    fn delta_ticks(self, begin: u64, end: u64) -> Result<u64, ScaffoldContractError> {
        end.checked_sub(begin)
            .filter(|ticks| *ticks != 0)
            .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)
    }

    fn period_ns_q24(self) -> Result<u64, ScaffoldContractError> {
        let exponent = i32::from(self.binary_exponent) + 24;
        let scaled = u128::from(self.significand);
        let rounded = if exponent >= 0 {
            scaled
                .checked_shl(exponent as u32)
                .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?
        } else {
            let shift = exponent.unsigned_abs();
            if shift >= u128::BITS {
                0
            } else {
                scaled
                    .checked_add(1_u128 << (shift - 1))
                    .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?
                    >> shift
            }
        };
        u64::try_from(rounded)
            .ok()
            .filter(|value| *value != 0)
            .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)
    }
}

fn timestamp_mapping_completed(
    receiver: &std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
) -> bool {
    matches!(receiver.try_recv(), Ok(Ok(())))
}

struct GpuTimestampResources {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    period: ExactGpuTimestampPeriod,
}

impl GpuTimestampResources {
    fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self, ScaffoldContractError> {
        validate_required_device_features(device.features())?;
        let period =
            ExactGpuTimestampPeriod::try_from_f32_bits(queue.get_timestamp_period().to_bits())?;
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("closed-loop-runtime-timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: GPU_TIMESTAMP_QUERY_COUNT,
        });
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("closed-loop-runtime-timestamp-resolve"),
            size: GPU_TIMESTAMP_READBACK_BYTES,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("closed-loop-runtime-timestamp-readback"),
            size: GPU_TIMESTAMP_READBACK_BYTES,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Ok(Self {
            query_set,
            resolve_buffer,
            readback_buffer,
            period,
        })
    }

    fn read_delta_and_elapsed_ns(&self) -> Result<(u64, u64), ScaffoldContractError> {
        let mapped = self
            .readback_buffer
            .slice(..GPU_TIMESTAMP_READBACK_BYTES)
            .get_mapped_range();
        let bytes: &[u8] = &mapped;
        let begin = u64::from_le_bytes(
            bytes[0..8]
                .try_into()
                .map_err(|_| ScaffoldContractError::GpuTimestampQueryUnavailable)?,
        );
        let end = u64::from_le_bytes(
            bytes[8..16]
                .try_into()
                .map_err(|_| ScaffoldContractError::GpuTimestampQueryUnavailable)?,
        );
        drop(mapped);
        self.readback_buffer.unmap();
        Ok((
            self.period.delta_ticks(begin, end)?,
            self.period.elapsed_ns(begin, end)?,
        ))
    }

    fn period_ns_q24(&self) -> Result<u64, ScaffoldContractError> {
        self.period.period_ns_q24()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuHardwareReceipt {
    pub schema_version: u16,
    pub generation: u64,
    pub backend_api: String,
    pub adapter_name: String,
    pub vendor_id: u32,
    pub device_id: u32,
    pub driver_digest: [u64; 4],
    pub feature_digest: [u64; 4],
    pub limits_digest: [u64; 4],
    pub gpu_layout_version: u16,
    pub backend_version: String,
}

/// Ephemeral capture of Task 3 activity state. Runtime handle fields are
/// validated before the app canonicalizes this into portable world records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuActivityRuntimeSnapshot {
    pub next_sequence_cursor: u64,
    pub brain_atp_q16: u32,
    pub last_world_atp_tick: Option<u64>,
    pub next_completed_gpu_time_ns: u64,
    pub pressure: Option<GpuPressureSample>,
    pub throttle: Option<NeuralThrottleDecision>,
    pub work: Option<BrainWorkReceipt>,
}

/// Portable activity record accepted only as data for rebinding to a newly
/// allocated handle. It carries no backend instance, slot, or generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuPortableActivityRestoreRecord {
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
}

/// Portable activity continuation rebound only after a new opaque handle exists.
/// Runtime slots and generations are deliberately absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuActivityRestoreInput {
    pub next_sequence_cursor: u64,
    pub checkpoint_tick: u64,
    pub next_completed_gpu_time_ns: u64,
    pub brain_atp_q16: u32,
    pub last_world_atp_tick: Option<u64>,
    pub record: Option<GpuPortableActivityRestoreRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuBackendState {
    Ready,
    DeviceLost {
        last_checkpoint_digest: Option<[u64; 4]>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GpuBrainHandle {
    backend_instance_id: NonZeroU64,
    class_id: BrainClassId,
    slot: u32,
    generation: u32,
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
}

impl GpuBrainHandle {
    pub const fn class_id(self) -> BrainClassId {
        self.class_id
    }

    pub const fn slot(self) -> u32 {
        self.slot
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }

    pub const fn organism_id(self) -> OrganismId {
        self.organism_id
    }

    pub const fn phenotype_hash(self) -> PhenotypeHash {
        self.phenotype_hash
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GpuClosedLoopTick {
    pub handle: GpuBrainHandle,
    pub dispatch_generation: u64,
    pub base_digest: PerceptionBaseDigest,
    pub frame_digest: PerceptionFrameDigest,
    pub memory_context_binding: Option<GpuMemoryContextDispatchReceipt>,
    pub active_activation_side: u8,
    pub selection: NeuralActionSelection,
    pub pending_eligibility: PendingEligibilityReceipt,
    pub pressure: GpuPressureSample,
    pub throttle: NeuralThrottleDecision,
    pub work: BrainWorkReceipt,
    pub compact_readback_bytes: usize,
    pub hardware_receipt_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuNeuralTimingSample {
    pub dispatch_generation: u64,
    pub class_id_raw: u16,
    pub population: u32,
    pub inference_timestamp_ticks: u64,
    pub plasticity_timestamp_ticks: u64,
    pub timestamp_period_ns_q24: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingInferenceTiming {
    dispatch_generation: u64,
    class_id_raw: Option<u16>,
    population: u32,
    inference_timestamp_ticks: u64,
}

/// One finalized candidate-memory context bound to an exact live brain handle
/// and immutable perception frame.
#[derive(Debug, Clone, Copy)]
pub struct GpuClosedLoopMemoryTickInput<'a> {
    handle: GpuBrainHandle,
    frame: &'a PerceptionFrame,
    memory_upload: &'a GpuMemoryContextUpload,
}

impl<'a> GpuClosedLoopMemoryTickInput<'a> {
    pub fn try_new(
        handle: GpuBrainHandle,
        frame: &'a PerceptionFrame,
        memory_upload: &'a GpuMemoryContextUpload,
    ) -> Result<Self, ScaffoldContractError> {
        frame.validate()?;
        if handle.organism_id != frame.organism_id()
            || memory_upload.header.class_id != u32::from(handle.class_id.raw())
            || memory_upload.header.slot != handle.slot
            || memory_upload.header.slot_generation != handle.generation
            || memory_upload.header.tick() != frame.tick().raw()
            || memory_upload.base_frame_digest != frame.base_digest()
            || memory_upload.context_digest != frame.context().canonical_digest()
            || memory_upload.final_frame_digest != frame.frame_digest()
        {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Ok(Self {
            handle,
            frame,
            memory_upload,
        })
    }
}

/// Mixed-class memory-aware runtime tick input. The backend still groups rows
/// by class internally and submits all class pipelines in one command buffer.
#[derive(Debug)]
pub struct GpuClosedLoopMemoryBatchInput<'a> {
    members: Vec<GpuClosedLoopMemoryTickInput<'a>>,
}

impl<'a> GpuClosedLoopMemoryBatchInput<'a> {
    pub fn try_new(
        members: Vec<GpuClosedLoopMemoryTickInput<'a>>,
    ) -> Result<Self, ScaffoldContractError> {
        if members.is_empty() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        Ok(Self { members })
    }
}

#[derive(Clone, Copy)]
struct GpuRuntimeTickInput<'a> {
    handle: GpuBrainHandle,
    frame: &'a PerceptionFrame,
    memory_upload: Option<&'a GpuMemoryContextUpload>,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CheckedNonZeroAllocator {
    next: Option<NonZeroU64>,
}

#[cfg(test)]
impl CheckedNonZeroAllocator {
    const fn new(next: u64) -> Self {
        Self {
            next: NonZeroU64::new(next),
        }
    }

    fn take(&mut self) -> Result<NonZeroU64, GpuClosedLoopError> {
        let value = self.next.ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        self.next = value.get().checked_add(1).and_then(NonZeroU64::new);
        Ok(value)
    }
}

#[cfg(test)]
thread_local! {
    static TEST_ALLOCATION_STATE: std::cell::RefCell<Option<(CheckedNonZeroAllocator, CheckedNonZeroAllocator)>> = const { std::cell::RefCell::new(None) };
}

fn take_atomic_nonzero(allocator: &AtomicU64) -> Result<NonZeroU64, GpuClosedLoopError> {
    let value = allocator
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
    NonZeroU64::new(value).ok_or(GpuClosedLoopError::ArithmeticOverflow)
}

fn next_backend_instance_id() -> Result<NonZeroU64, GpuClosedLoopError> {
    #[cfg(test)]
    if let Some(value) = TEST_ALLOCATION_STATE.with(|state| {
        state
            .borrow_mut()
            .as_mut()
            .map(|(backend, _)| backend.take())
    }) {
        return value;
    }
    take_atomic_nonzero(&NEXT_BACKEND_INSTANCE_ID)
}

fn next_hardware_receipt_generation() -> Result<NonZeroU64, GpuClosedLoopError> {
    #[cfg(test)]
    if let Some(value) = TEST_ALLOCATION_STATE.with(|state| {
        state
            .borrow_mut()
            .as_mut()
            .map(|(_, receipt)| receipt.take())
    }) {
        return value;
    }
    take_atomic_nonzero(&NEXT_HARDWARE_RECEIPT_GENERATION)
}

#[cfg(test)]
fn with_runtime_allocation_state_for_test<R>(
    backend_next: u64,
    receipt_next: u64,
    operation: impl FnOnce() -> R,
) -> R {
    TEST_ALLOCATION_STATE.with(|state| {
        assert!(state.borrow().is_none(), "nested allocation test state");
        *state.borrow_mut() = Some((
            CheckedNonZeroAllocator::new(backend_next),
            CheckedNonZeroAllocator::new(receipt_next),
        ));
    });
    let result = operation();
    TEST_ALLOCATION_STATE.with(|state| *state.borrow_mut() = None);
    result
}

fn canonical_driver_digest(driver: &str, driver_info: &str) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(GPU_DRIVER_DIGEST_DOMAIN);
    digest.write_sequence_len(2);
    digest.write_utf8(driver);
    digest.write_utf8(driver_info);
    digest.finish256()
}

fn canonical_feature_digest(requested: wgpu::Features, enabled: wgpu::Features) -> [u64; 4] {
    let requested = requested.bits().0;
    let enabled = enabled.bits().0;
    let mut digest = CanonicalDigestBuilder::new(GPU_FEATURE_DIGEST_DOMAIN);
    digest.write_sequence_len(4);
    for word in [requested[0], requested[1], enabled[0], enabled[1]] {
        digest.write_u64(word);
    }
    digest.finish256()
}

fn canonical_limit_words_for_test(limits: &wgpu::Limits) -> [u64; 51] {
    [
        u64::from(limits.max_texture_dimension_1d),
        u64::from(limits.max_texture_dimension_2d),
        u64::from(limits.max_texture_dimension_3d),
        u64::from(limits.max_texture_array_layers),
        u64::from(limits.max_bind_groups),
        u64::from(limits.max_bindings_per_bind_group),
        u64::from(limits.max_dynamic_uniform_buffers_per_pipeline_layout),
        u64::from(limits.max_dynamic_storage_buffers_per_pipeline_layout),
        u64::from(limits.max_sampled_textures_per_shader_stage),
        u64::from(limits.max_samplers_per_shader_stage),
        u64::from(limits.max_storage_buffers_per_shader_stage),
        u64::from(limits.max_storage_textures_per_shader_stage),
        u64::from(limits.max_uniform_buffers_per_shader_stage),
        u64::from(limits.max_binding_array_elements_per_shader_stage),
        u64::from(limits.max_binding_array_acceleration_structure_elements_per_shader_stage),
        u64::from(limits.max_binding_array_sampler_elements_per_shader_stage),
        limits.max_uniform_buffer_binding_size,
        limits.max_storage_buffer_binding_size,
        u64::from(limits.max_vertex_buffers),
        limits.max_buffer_size,
        u64::from(limits.max_vertex_attributes),
        u64::from(limits.max_vertex_buffer_array_stride),
        u64::from(limits.max_inter_stage_shader_variables),
        u64::from(limits.min_uniform_buffer_offset_alignment),
        u64::from(limits.min_storage_buffer_offset_alignment),
        u64::from(limits.max_color_attachments),
        u64::from(limits.max_color_attachment_bytes_per_sample),
        u64::from(limits.max_compute_workgroup_storage_size),
        u64::from(limits.max_compute_invocations_per_workgroup),
        u64::from(limits.max_compute_workgroup_size_x),
        u64::from(limits.max_compute_workgroup_size_y),
        u64::from(limits.max_compute_workgroup_size_z),
        u64::from(limits.max_compute_workgroups_per_dimension),
        u64::from(limits.max_immediate_size),
        u64::from(limits.max_non_sampler_bindings),
        u64::from(limits.max_task_mesh_workgroup_total_count),
        u64::from(limits.max_task_mesh_workgroups_per_dimension),
        u64::from(limits.max_task_invocations_per_workgroup),
        u64::from(limits.max_task_invocations_per_dimension),
        u64::from(limits.max_mesh_invocations_per_workgroup),
        u64::from(limits.max_mesh_invocations_per_dimension),
        u64::from(limits.max_task_payload_size),
        u64::from(limits.max_mesh_output_vertices),
        u64::from(limits.max_mesh_output_primitives),
        u64::from(limits.max_mesh_output_layers),
        u64::from(limits.max_mesh_multiview_view_count),
        u64::from(limits.max_blas_primitive_count),
        u64::from(limits.max_blas_geometry_count),
        u64::from(limits.max_tlas_instance_count),
        u64::from(limits.max_acceleration_structures_per_shader_stage),
        u64::from(limits.max_multiview_view_count),
    ]
}

fn canonical_limits_digest(limits: &wgpu::Limits) -> [u64; 4] {
    let words = canonical_limit_words_for_test(limits);
    let mut digest = CanonicalDigestBuilder::new(GPU_LIMITS_DIGEST_DOMAIN);
    digest.write_sequence_len(words.len());
    for word in words {
        digest.write_u64(word);
    }
    digest.finish256()
}

fn validate_required_gpu_layout_version(version: u32) -> Result<(), ScaffoldContractError> {
    if version == GPU_CLOSED_LOOP_LAYOUT_VERSION {
        Ok(())
    } else {
        Err(ScaffoldContractError::GpuLayoutMismatch)
    }
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_owned();
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn backend_slug(backend: wgpu::Backend) -> Result<&'static str, ScaffoldContractError> {
    match backend {
        wgpu::Backend::Vulkan => Ok("vulkan"),
        wgpu::Backend::Metal => Ok("metal"),
        wgpu::Backend::Dx12 => Ok("dx12"),
        wgpu::Backend::Gl => Ok("gl"),
        wgpu::Backend::BrowserWebGpu => Ok("webgpu"),
        wgpu::Backend::Noop => Err(ScaffoldContractError::NeuralBackendUnavailable),
    }
}

fn build_hardware_receipt(
    info: &wgpu::AdapterInfo,
    requested_features: wgpu::Features,
    enabled_features: wgpu::Features,
    enabled_limits: &wgpu::Limits,
) -> Result<GpuHardwareReceipt, ScaffoldContractError> {
    // Cargo validates CARGO_PKG_VERSION as SemVer; the receipt additionally
    // enforces its transport bound before consuming a process-local ID.
    if !BACKEND_VERSION.is_ascii() || BACKEND_VERSION.len() > 64 {
        return Err(ScaffoldContractError::NeuralBackendUnavailable);
    }
    let generation = next_hardware_receipt_generation()
        .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
    Ok(GpuHardwareReceipt {
        schema_version: GPU_HARDWARE_RECEIPT_SCHEMA_VERSION,
        generation: generation.get(),
        backend_api: backend_slug(info.backend)?.to_owned(),
        adapter_name: truncate_utf8(&info.name, 256),
        vendor_id: info.vendor,
        device_id: info.device,
        driver_digest: canonical_driver_digest(
            &truncate_utf8(&info.driver, 256),
            &truncate_utf8(&info.driver_info, 256),
        ),
        feature_digest: canonical_feature_digest(requested_features, enabled_features),
        limits_digest: canonical_limits_digest(enabled_limits),
        gpu_layout_version: GPU_CLOSED_LOOP_LAYOUT_VERSION as u16,
        backend_version: BACKEND_VERSION.to_owned(),
    })
}

enum GpuAdapterCandidate {
    Hardware(wgpu::Adapter),
    #[cfg(test)]
    Software,
}

trait GpuDeviceFactory {
    fn request_adapters(&self) -> Result<Vec<GpuAdapterCandidate>, ScaffoldContractError>;

    fn request_device(
        &self,
        adapter: &wgpu::Adapter,
        descriptor: &wgpu::DeviceDescriptor<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue), ScaffoldContractError>;
}

struct WgpuDeviceFactory;

impl GpuDeviceFactory for WgpuDeviceFactory {
    fn request_adapters(&self) -> Result<Vec<GpuAdapterCandidate>, ScaffoldContractError> {
        pollster::block_on(async {
            let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
            descriptor.backends = wgpu::Backends::PRIMARY | wgpu::Backends::SECONDARY;
            let instance = wgpu::Instance::new(descriptor);
            let required_features = wgpu::Features::empty();
            let required_limits = required_device_limits();
            let mut adapters = instance
                .enumerate_adapters(wgpu::Backends::PRIMARY | wgpu::Backends::SECONDARY)
                .await
                .into_iter()
                .filter(|adapter| {
                    let info = adapter.get_info();
                    info.device_type != wgpu::DeviceType::Cpu
                        && info.backend != wgpu::Backend::Noop
                        && backend_slug(info.backend).is_ok()
                        && adapter.features().contains(required_features)
                        && required_limits.check_limits(&adapter.limits())
                })
                .collect::<Vec<_>>();
            adapters.sort_by_key(|adapter| {
                let info = adapter.get_info();
                let backend_rank = match info.backend {
                    wgpu::Backend::Vulkan => 0,
                    wgpu::Backend::Metal => 1,
                    wgpu::Backend::Dx12 => 2,
                    wgpu::Backend::BrowserWebGpu => 3,
                    wgpu::Backend::Gl => 4,
                    wgpu::Backend::Noop => 5,
                };
                let device_rank = match info.device_type {
                    wgpu::DeviceType::DiscreteGpu => 0,
                    wgpu::DeviceType::IntegratedGpu => 1,
                    _ => 2,
                };
                (
                    backend_rank,
                    device_rank,
                    info.vendor,
                    info.device,
                    info.device_pci_bus_id.clone(),
                    info.name.clone(),
                    info.driver.clone(),
                    info.driver_info.clone(),
                )
            });
            if adapters.is_empty() {
                Err(ScaffoldContractError::NeuralBackendUnavailable)
            } else {
                Ok(adapters
                    .into_iter()
                    .map(GpuAdapterCandidate::Hardware)
                    .collect())
            }
        })
    }

    fn request_device(
        &self,
        adapter: &wgpu::Adapter,
        descriptor: &wgpu::DeviceDescriptor<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue), ScaffoldContractError> {
        pollster::block_on(adapter.request_device(descriptor))
            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)
    }
}

struct RequiredGpuDevice {
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    hardware: GpuHardwareReceipt,
    lost: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GpuBrainSlotOwnership {
    organism_id: OrganismId,
    phenotype_hash: PhenotypeHash,
    sensor_profile: SensorProfile,
}

pub(crate) struct ResidentBrainSlot {
    ownership: GpuBrainSlotOwnership,
    pub(crate) phenotype: BrainPhenotype,
    pub(crate) brain_slot: GpuBrainSlot,
    pub(crate) ranges: GpuFixedSlotRanges,
    pub(crate) active_eligibility_bank: u8,
    pub(crate) active_eligibility_generation: u64,
    pub(crate) active_weight_bank: u8,
    pub(crate) active_weight_generation: u64,
    pub(crate) replay_journal_generation: u64,
    pub(crate) transaction_generation: u64,
    pub(crate) logical_dispatch_generation: u64,
    pub(crate) activity_sequence_cursor: u64,
    pub(crate) brain_atp_q16: u32,
    pub(crate) last_world_atp_tick: Option<u64>,
    pub(crate) last_activity_dispatch_generation: u64,
    pub(crate) last_activity_frame_digest: [u64; 4],
    pub(crate) last_completed_gpu_time_ns: u64,
    pub(crate) last_pressure: Option<GpuPressureSample>,
    pub(crate) last_throttle: Option<NeuralThrottleDecision>,
    pub(crate) last_work: Option<BrainWorkReceipt>,
    pub(crate) sleep_plan: alife_core::SleepConsolidationPlan,
    pub(crate) learning_sequence_guard: LearningSequenceGuard,
    pub(crate) pending_eligibility: Option<PendingEligibilityReceipt>,
    pub(crate) pending_eligibility_record: Option<GpuPendingEligibilityRecord>,
}

struct PreparedLearningApply {
    chunk_index: usize,
    handle: GpuBrainHandle,
    packet: OutcomeCreditPacket,
    outcome: GpuOutcomeCreditRecord,
    brain_slot: GpuBrainSlot,
    pending_receipt: PendingEligibilityReceipt,
    pending_record: GpuPendingEligibilityRecord,
    active_weight_generation: u64,
    active_eligibility_generation: u64,
    replay_journal_generation: u64,
    transaction_generation: u64,
    expected_last_committed: Option<alife_core::OutcomeCreditReplayKey>,
    commit_token: LearningCommitToken,
}

pub(crate) struct ClassBucketRuntime {
    pub(crate) plan: GpuFixedClassArenaPlan,
    pub(crate) buffers: GpuFixedClassArenaBuffers,
    pub(crate) pipelines: GpuClosedLoopPipelines,
    pub(crate) slots: Vec<Option<ResidentBrainSlot>>,
    pub(crate) generations: Vec<u32>,
    pub(crate) retired: BTreeSet<u32>,
    pub(crate) free_slots: Vec<u32>,
}

impl ClassBucketRuntime {
    fn from_plan(
        device: &wgpu::Device,
        kernels: Arc<GpuClosedLoopKernelSet>,
        plan: GpuFixedClassArenaPlan,
    ) -> Result<Self, GpuClosedLoopError> {
        let slot_capacity = plan.slot_capacity();
        let buffers = GpuFixedClassArenaBuffers::allocate(device, &plan)?;
        let pipelines = GpuClosedLoopPipelines::from_shared_kernel_set_for_fixed_arena(
            device, &buffers, kernels,
        )?;
        let slot_count =
            usize::try_from(slot_capacity).map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        let mut free_slots = (0..slot_capacity).collect::<Vec<_>>();
        free_slots.reverse();
        Ok(Self {
            plan,
            buffers,
            pipelines,
            slots: (0..slot_count).map(|_| None).collect(),
            generations: vec![0; slot_count],
            retired: BTreeSet::new(),
            free_slots,
        })
    }

    pub(crate) fn contains(&self, handle: GpuBrainHandle) -> bool {
        self.slots
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .is_some_and(|resident| {
                resident.brain_slot.record().slot_generation == handle.generation
                    && resident.ownership.organism_id == handle.organism_id
                    && resident.ownership.phenotype_hash == handle.phenotype_hash
            })
    }
}

#[derive(Default)]
pub(crate) struct ClassBucketPool {
    pub(crate) chunks: Vec<ClassBucketRuntime>,
}

impl ClassBucketPool {
    pub(crate) fn bucket_index_for_handle(&self, handle: GpuBrainHandle) -> Option<usize> {
        self.chunks
            .iter()
            .position(|bucket| bucket.contains(handle))
    }

    pub(crate) fn bucket_for_handle(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<&ClassBucketRuntime, ScaffoldContractError> {
        self.bucket_index_for_handle(handle)
            .and_then(|index| self.chunks.get(index))
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    pub(crate) fn bucket_for_handle_mut(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<&mut ClassBucketRuntime, ScaffoldContractError> {
        let index = self
            .bucket_index_for_handle(handle)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        self.chunks
            .get_mut(index)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    pub(crate) fn resident(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<&ResidentBrainSlot, ScaffoldContractError> {
        self.bucket_for_handle(handle)?
            .slots
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    pub(crate) fn resident_mut(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<&mut ResidentBrainSlot, ScaffoldContractError> {
        self.bucket_for_handle_mut(handle)?
            .slots
            .get_mut(handle.slot as usize)
            .and_then(Option::as_mut)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    fn reusable_slot(
        &self,
        class_raw: u16,
        watermarks: &BTreeMap<(u16, u32), u32>,
    ) -> Option<(usize, u32, u32)> {
        self.chunks
            .iter()
            .enumerate()
            .find_map(|(chunk_index, bucket)| {
                let slot = *bucket.free_slots.last()?;
                let generation = watermarks
                    .get(&(class_raw, slot))
                    .copied()
                    .unwrap_or(0)
                    .checked_add(1)?;
                Some((chunk_index, slot, generation))
            })
    }
}

struct PreparedClassDispatch {
    class_id: u16,
    chunk_index: usize,
    original_indices: Vec<usize>,
    prepared: Option<GpuPreparedActiveBatch>,
    batch: Option<GpuActiveBatchUpload>,
    recorded: bool,
    map_ticket: Option<GpuCompactMapTicket>,
    validated: Option<GpuValidatedClassBatch>,
}

fn capacity_for_promoted_class(
    class_id: BrainClassId,
) -> Result<BrainCapacityClass, ScaffoldContractError> {
    BrainCapacityClass::production_for_id(class_id)
}

fn live_pressure_sample(
    policy: &BrainActivityPolicyV1,
    identity: BrainDispatchIdentity,
    resident: &ResidentBrainSlot,
    admission: &GpuAdmissionReceipt,
    runtime_budget: &GpuRuntimeBudget,
) -> Result<GpuPressureSample, ScaffoldContractError> {
    GpuPressureSample::try_new(
        policy,
        GpuPressureSampleInput {
            identity,
            source_dispatch_generation: resident.last_activity_dispatch_generation,
            source_frame_digest: resident.last_activity_frame_digest,
            completed_gpu_time_ns: resident.last_completed_gpu_time_ns,
            // The runtime waits for the submitted mixed-class batch before it
            // accepts another neural dispatch, so no older neural work is queued.
            queue_depth: 0,
            logical_heap_used: admission.logical_committed_bytes,
            logical_heap_capacity: runtime_budget.logical_neural_heap_budget_bytes,
            brain_atp_remaining_q16: resident.brain_atp_q16,
            brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
        },
    )
}

pub(crate) fn map_gpu_contract_error(error: GpuClosedLoopError) -> ScaffoldContractError {
    match error {
        GpuClosedLoopError::LayoutMismatch => ScaffoldContractError::GpuLayoutMismatch,
        GpuClosedLoopError::StaleOrForeignHandle => ScaffoldContractError::BrainOwnershipMismatch,
        GpuClosedLoopError::MalformedUpload
        | GpuClosedLoopError::NonFinitePayload
        | GpuClosedLoopError::InvalidOffsetDomain => ScaffoldContractError::InvalidPerceptionFrame,
        GpuClosedLoopError::CapacityExceeded
        | GpuClosedLoopError::ArithmeticOverflow
        | GpuClosedLoopError::SubmissionFailed => ScaffoldContractError::NeuralBackendUnavailable,
    }
}

pub struct GpuClosedLoopBackend {
    backend_instance_id: NonZeroU64,
    pub(crate) hardware: GpuHardwareReceipt,
    #[allow(dead_code)]
    adapter: wgpu::Adapter,
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,
    timestamp_resources: GpuTimestampResources,
    plasticity_timestamp_resources: GpuTimestampResources,
    pub(crate) device_lost: Arc<AtomicBool>,
    kernels: Arc<GpuClosedLoopKernelSet>,
    pub(crate) state: GpuBackendState,
    runtime_profile: GpuRuntimeProfile,
    runtime_budget: GpuRuntimeBudget,
    activity_policy: BrainActivityPolicyV1,
    admission: GpuAdmissionReceipt,
    pub(crate) class_buckets: BTreeMap<u16, ClassBucketPool>,
    slot_generation_watermarks: BTreeMap<(u16, u32), u32>,
    organisms: BTreeMap<u64, GpuBrainHandle>,
    pub(crate) next_dispatch_generation: u64,
    force_device_lost_after_submit: bool,
    #[cfg(feature = "gpu-tests")]
    forced_learning_rejections_remaining: u8,
    #[cfg(feature = "gpu-tests")]
    forced_discard_rejections_remaining: u8,
    recorded_pressure_replay: VecDeque<GpuPressureSample>,
    completed_dispatch_count: u64,
    perception_upload_count: u64,
    completed_selection_count: u64,
    last_compact_readback_bytes: usize,
    pending_inference_timing: Option<PendingInferenceTiming>,
    completed_neural_timing: Option<GpuNeuralTimingSample>,
    pub(crate) next_sleep_job_id: u64,
    pub(crate) sleep_jobs: BTreeMap<u64, crate::GpuSleepJobState>,
    pub(crate) committed_sleep: BTreeMap<(u16, u32, u32, u64), crate::GpuSleepConsolidationReceipt>,
}

impl GpuClosedLoopBackend {
    pub fn new_required(profile: GpuRuntimeProfile) -> Result<Self, ScaffoldContractError> {
        Self::new_with_factory_and_profile(&WgpuDeviceFactory, profile)
    }

    #[cfg(test)]
    fn new_with_factory(factory: &impl GpuDeviceFactory) -> Result<Self, ScaffoldContractError> {
        Self::new_with_factory_and_profile(factory, GpuRuntimeProfile::production_v1())
    }

    fn new_with_factory_and_profile(
        factory: &impl GpuDeviceFactory,
        profile: GpuRuntimeProfile,
    ) -> Result<Self, ScaffoldContractError> {
        profile.validate_contract()?;
        let required = acquire_required_gpu(factory)?;
        let backend_instance_id = next_backend_instance_id()
            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
        let runtime_budget = GpuRuntimeBudget::from_device(
            profile,
            required.device.features(),
            &required.device.limits(),
            required.hardware.limits_digest,
        )?;
        let admission = GpuAdmissionReceipt::empty(runtime_budget);
        let activity_policy = BrainActivityPolicyV1::production_v1();
        activity_policy.validate_contract()?;
        let timestamp_resources = GpuTimestampResources::new(&required.device, &required.queue)?;
        let plasticity_timestamp_resources =
            GpuTimestampResources::new(&required.device, &required.queue)?;
        let kernels =
            GpuClosedLoopKernelSet::new(&required.device).map_err(map_gpu_contract_error)?;
        Ok(Self {
            backend_instance_id,
            hardware: required.hardware,
            adapter: required.adapter,
            device: required.device,
            queue: required.queue,
            timestamp_resources,
            plasticity_timestamp_resources,
            device_lost: required.lost,
            kernels,
            state: GpuBackendState::Ready,
            runtime_profile: profile,
            runtime_budget,
            activity_policy,
            admission,
            class_buckets: BTreeMap::new(),
            slot_generation_watermarks: BTreeMap::new(),
            organisms: BTreeMap::new(),
            next_dispatch_generation: 1,
            force_device_lost_after_submit: false,
            #[cfg(feature = "gpu-tests")]
            forced_learning_rejections_remaining: 0,
            #[cfg(feature = "gpu-tests")]
            forced_discard_rejections_remaining: 0,
            recorded_pressure_replay: VecDeque::new(),
            completed_dispatch_count: 0,
            perception_upload_count: 0,
            completed_selection_count: 0,
            last_compact_readback_bytes: 0,
            pending_inference_timing: None,
            completed_neural_timing: None,
            next_sleep_job_id: 1,
            sleep_jobs: BTreeMap::new(),
            committed_sleep: BTreeMap::new(),
        })
    }

    pub const fn hardware_receipt(&self) -> &GpuHardwareReceipt {
        &self.hardware
    }

    /// Installs an exact pressure sequence for same-adapter evidence replay.
    ///
    /// This exact-replay boundary replaces only the host pressure sample;
    /// perception, recurrent execution, logits, selection, world outcomes, and
    /// learning remain GPU-authoritative and run through the production path.
    pub fn install_recorded_pressure_replay(
        &mut self,
        samples: Vec<GpuPressureSample>,
    ) -> Result<(), ScaffoldContractError> {
        if samples.is_empty() || !self.recorded_pressure_replay.is_empty() {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        for sample in &samples {
            sample.validate_for(&self.activity_policy)?;
        }
        self.recorded_pressure_replay = samples.into();
        Ok(())
    }

    pub fn recorded_pressure_replay_remaining(&self) -> usize {
        self.recorded_pressure_replay.len()
    }

    pub const fn runtime_profile(&self) -> &GpuRuntimeProfile {
        &self.runtime_profile
    }

    pub const fn runtime_budget(&self) -> &GpuRuntimeBudget {
        &self.runtime_budget
    }

    pub const fn activity_policy(&self) -> &BrainActivityPolicyV1 {
        &self.activity_policy
    }

    pub const fn admission_receipt(&self) -> &GpuAdmissionReceipt {
        &self.admission
    }

    pub const fn state(&self) -> &GpuBackendState {
        &self.state
    }

    pub const fn completed_dispatch_count(&self) -> u64 {
        self.completed_dispatch_count
    }

    pub const fn perception_upload_count(&self) -> u64 {
        self.perception_upload_count
    }

    pub const fn completed_selection_count(&self) -> u64 {
        self.completed_selection_count
    }

    pub fn take_completed_neural_timing_sample(&mut self) -> Option<GpuNeuralTimingSample> {
        self.completed_neural_timing.take()
    }

    pub fn brain_atp_q16(&self, handle: GpuBrainHandle) -> Result<u32, ScaffoldContractError> {
        self.validate_handle_backend(handle)?;
        self.class_buckets
            .get(&handle.class_id.raw())
            .and_then(|pool| pool.resident(handle).ok())
            .map(|resident| resident.brain_atp_q16)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    pub fn snapshot_activity_state(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<GpuActivityRuntimeSnapshot, ScaffoldContractError> {
        self.validate_handle_backend(handle)?;
        let resident = self
            .class_buckets
            .get(&handle.class_id.raw())
            .and_then(|pool| pool.resident(handle).ok())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let all_absent = resident.last_pressure.is_none()
            && resident.last_throttle.is_none()
            && resident.last_work.is_none();
        let all_present = resident.last_pressure.is_some()
            && resident.last_throttle.is_some()
            && resident.last_work.is_some();
        if !(all_absent || all_present)
            || resident.activity_sequence_cursor == 0
            || resident.brain_atp_q16 > BRAIN_ATP_Q16_MAX
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        if let (Some(pressure), Some(throttle), Some(work)) = (
            resident.last_pressure,
            resident.last_throttle.as_ref(),
            resident.last_work.as_ref(),
        ) {
            pressure.validate_for(&self.activity_policy)?;
            let capacity = BrainCapacityClass::production_for_id(handle.class_id)?;
            throttle.validate_for(&resident.phenotype, capacity.execution())?;
            work.validate_for(&self.activity_policy, throttle)?;
            if pressure.handle_slot != handle.slot
                || pressure.handle_generation != handle.generation
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            throttle.validate_runtime_binding(handle.slot, handle.generation)?;
            work.validate_runtime_binding(handle.slot, handle.generation)?;
            if resident.activity_sequence_cursor
                != pressure.sequence_cursor.checked_add(1).unwrap_or(0)
                || resident.last_activity_dispatch_generation != pressure.dispatch_generation
                || resident.last_activity_frame_digest != pressure.frame_digest
            {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
        } else if resident.activity_sequence_cursor != 1
            || resident.last_activity_dispatch_generation != 0
            || resident.last_activity_frame_digest != [0; 4]
            || resident.last_completed_gpu_time_ns != 0
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        Ok(GpuActivityRuntimeSnapshot {
            next_sequence_cursor: resident.activity_sequence_cursor,
            brain_atp_q16: resident.brain_atp_q16,
            last_world_atp_tick: resident.last_world_atp_tick,
            next_completed_gpu_time_ns: resident.last_completed_gpu_time_ns,
            pressure: resident.last_pressure,
            throttle: resident.last_throttle.clone(),
            work: resident.last_work.clone(),
        })
    }

    pub fn restore_activity_state(
        &mut self,
        handle: GpuBrainHandle,
        input: GpuActivityRestoreInput,
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        if input.brain_atp_q16 > BRAIN_ATP_Q16_MAX
            || input
                .last_world_atp_tick
                .is_some_and(|tick| tick > input.checkpoint_tick)
        {
            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
        }
        let phenotype = self
            .class_buckets
            .get(&handle.class_id.raw())
            .and_then(|pool| pool.resident(handle).ok())
            .map(|resident| resident.phenotype.clone())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;

        let rebound = match input.record {
            Some(record) => {
                if record.policy_version != self.activity_policy.policy_version
                    || record.policy_digest != self.activity_policy.policy_digest
                    || record.organism_id_raw != handle.organism_id.raw()
                    || record.class_id_raw != handle.class_id.raw()
                    || record.tick > input.checkpoint_tick
                    || input.next_sequence_cursor
                        != record.sequence_cursor.checked_add(1).unwrap_or(0)
                    || record.brain_atp_fraction_q16 > BRAIN_ATP_Q16_MAX
                {
                    return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                }
                let identity = BrainDispatchIdentity {
                    organism_id_raw: record.organism_id_raw,
                    tick: record.tick,
                    class_id_raw: record.class_id_raw,
                    handle_slot: handle.slot,
                    handle_generation: handle.generation,
                    sequence_cursor: record.sequence_cursor,
                    dispatch_generation: record.dispatch_generation,
                    frame_digest: record.frame_digest,
                };
                let pressure = GpuPressureSample::try_new(
                    &self.activity_policy,
                    GpuPressureSampleInput {
                        identity,
                        source_dispatch_generation: record.source_dispatch_generation,
                        source_frame_digest: record.source_frame_digest,
                        completed_gpu_time_ns: record.completed_gpu_time_ns,
                        queue_depth: record.queue_depth,
                        logical_heap_used: u64::from(record.logical_heap_pressure_q16),
                        logical_heap_capacity: u64::from(BRAIN_ATP_Q16_MAX),
                        brain_atp_remaining_q16: record.brain_atp_fraction_q16,
                        brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
                    },
                )?;
                let capacity = BrainCapacityClass::production_for_id(handle.class_id)?;
                let throttle = NeuralThrottleDecision::derive(
                    &self.activity_policy,
                    &phenotype,
                    capacity.execution(),
                    identity,
                    pressure,
                )?;
                if throttle.level != record.level
                    || throttle.microsteps != record.microsteps
                    || throttle.enabled_route_ids != record.enabled_route_ids
                    || throttle.route_schedule_digest != record.route_schedule_digest
                {
                    return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                }
                let work = BrainWorkReceipt::try_new(
                    &self.activity_policy,
                    &throttle,
                    record.work,
                    record.atp_before_q16,
                )?;
                if work.neural_cost_q24 != record.neural_cost_q24
                    || work.atp_debit_q16 != record.atp_debit_q16
                    || work.atp_after_q16 != record.atp_after_q16
                {
                    return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                }
                Some((pressure, throttle, work))
            }
            None if input.next_sequence_cursor == 1 && input.next_completed_gpu_time_ns == 0 => {
                None
            }
            None => return Err(ScaffoldContractError::BrainActivitySequenceMismatch),
        };

        let resident = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .and_then(|pool| pool.resident_mut(handle).ok())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        resident.activity_sequence_cursor = input.next_sequence_cursor;
        match rebound {
            Some((pressure, throttle, work)) => {
                resident.brain_atp_q16 = input.brain_atp_q16;
                resident.last_world_atp_tick = input.last_world_atp_tick;
                resident.last_activity_dispatch_generation = pressure.dispatch_generation;
                resident.last_activity_frame_digest = pressure.frame_digest;
                resident.last_completed_gpu_time_ns = input.next_completed_gpu_time_ns;
                resident.last_pressure = Some(pressure);
                resident.last_throttle = Some(throttle);
                resident.last_work = Some(work);
            }
            None => {
                resident.brain_atp_q16 = input.brain_atp_q16;
                resident.last_world_atp_tick = input.last_world_atp_tick;
                resident.last_activity_dispatch_generation = 0;
                resident.last_activity_frame_digest = [0; 4];
                resident.last_completed_gpu_time_ns = 0;
                resident.last_pressure = None;
                resident.last_throttle = None;
                resident.last_work = None;
            }
        }
        Ok(())
    }

    /// Charges the exact world-owned ATP term before neural dispatch.
    ///
    /// The monotonic tick guard makes basal cost replay-safe. Sleep recovery is
    /// a distinct credit in the same fixed-point transaction and never alters
    /// the neural work receipt's independently computed debit.
    pub fn charge_world_brain_atp_tick(
        &mut self,
        handle: GpuBrainHandle,
        world_tick: u64,
        began_tick_asleep: bool,
    ) -> Result<u32, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = pool.resident_mut(handle)?;
        if let Some(last) = resident.last_world_atp_tick {
            if last == world_tick {
                return Ok(resident.brain_atp_q16);
            }
            if last.checked_add(1) != Some(world_tick) {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
        }
        let after_basal = resident
            .brain_atp_q16
            .saturating_sub(BRAIN_ATP_BASAL_DEBIT_Q16);
        resident.brain_atp_q16 = if began_tick_asleep {
            after_basal
                .saturating_add(BRAIN_ATP_SLEEP_RECOVERY_Q16)
                .min(BRAIN_ATP_Q16_MAX)
        } else {
            after_basal
        };
        resident.last_world_atp_tick = Some(world_tick);
        Ok(resident.brain_atp_q16)
    }

    pub fn pending_eligibility(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<Option<PendingEligibilityReceipt>, ScaffoldContractError> {
        if self.device_lost.load(Ordering::Acquire) || !matches!(self.state, GpuBackendState::Ready)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = pool.resident(handle)?;
        Ok(resident.pending_eligibility)
    }

    /// Apply one sealed measured outcome to the pending waking eligibility.
    pub fn apply_sealed_outcome(
        &mut self,
        handle: GpuBrainHandle,
        patch: &ExperiencePatch,
    ) -> Result<GpuLearningReceipt, ScaffoldContractError> {
        let mut receipts = self.apply_sealed_outcome_batch(&[(handle, patch)])?;
        receipts
            .pop()
            .ok_or(ScaffoldContractError::LearningEvidenceMismatch)
    }

    /// Apply a same-class batch. Rows may span fixed arenas, but every row is
    /// bound to its arena-local slot, durable pending eligibility, and a
    /// core-owned sequence token before any command is submitted.
    pub fn apply_sealed_outcome_batch(
        &mut self,
        batch: &[(GpuBrainHandle, &ExperiencePatch)],
    ) -> Result<Vec<GpuLearningReceipt>, ScaffoldContractError> {
        self.ensure_ready()?;
        if batch.is_empty() {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        #[cfg(feature = "gpu-tests")]
        if self.forced_learning_rejections_remaining > 0 {
            self.forced_learning_rejections_remaining -= 1;
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        let class_id = batch[0].0.class_id.raw();
        let mut seen = BTreeSet::new();
        let mut prepared = Vec::with_capacity(batch.len());
        for (handle, patch) in batch {
            self.validate_handle_backend(*handle)?;
            if handle.class_id.raw() != class_id
                || !seen.insert((handle.slot, handle.generation, handle.organism_id.raw()))
            {
                return Err(ScaffoldContractError::LearningEvidenceMismatch);
            }
            let packet = OutcomeCreditPacket::from_sealed_patch(patch)?;
            let outcome = GpuOutcomeCreditRecord::try_from(&packet)?;
            let pool = self
                .class_buckets
                .get(&class_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let chunk_index = pool
                .bucket_index_for_handle(*handle)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = pool.resident(*handle)?;
            let expected_last_committed = resident.learning_sequence_guard.last_committed();
            let commit_token = resident
                .learning_sequence_guard
                .validate_next(packet.replay_key())?;
            let pending_receipt = resident
                .pending_eligibility
                .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
            let pending_record = resident
                .pending_eligibility_record
                .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
            let identity = pending_receipt.identity();
            if packet.organism_id() != handle.organism_id
                || packet.phenotype_hash() != handle.phenotype_hash
                || identity.handle_generation() != handle.generation
                || identity.phenotype_hash() != packet.phenotype_hash()
                || identity.dispatch_generation() != packet.dispatch_generation()
                || identity.originating_tick() != packet.originating_tick()
                || identity.frame_digest() != packet.frame_digest()
                || identity.active_activation_side() != packet.active_activation_side()
                || identity.candidate_index() != packet.selected_candidate()
                || identity.action_id() != packet.selected_action()
                || identity.action_family() != packet.selected_family()
                || identity.candidate_feature_digest() != packet.candidate_feature_digest()
                || identity.active_eligibility_generation()
                    != resident.active_eligibility_generation
                || identity.staging_eligibility_generation()
                    != resident
                        .active_eligibility_generation
                        .checked_add(1)
                        .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?
                || resident.active_weight_generation == 0
                || resident.replay_journal_generation == 0
                || resident.transaction_generation == 0
            {
                return Err(ScaffoldContractError::LearningEvidenceMismatch);
            }
            prepared.push(PreparedLearningApply {
                chunk_index,
                handle: *handle,
                packet,
                outcome,
                brain_slot: resident.brain_slot.clone(),
                pending_receipt,
                pending_record,
                active_weight_generation: resident.active_weight_generation,
                active_eligibility_generation: resident.active_eligibility_generation,
                replay_journal_generation: resident.replay_journal_generation,
                transaction_generation: resident.transaction_generation,
                expected_last_committed,
                commit_token,
            });
        }
        let mut grouped_indices = BTreeMap::<usize, Vec<usize>>::new();
        for (index, entry) in prepared.iter().enumerate() {
            grouped_indices
                .entry(entry.chunk_index)
                .or_default()
                .push(index);
        }
        let mut ordered_gpu_records = vec![None; prepared.len()];
        let mut plasticity_timestamp_ticks = 0_u64;
        for (chunk_index, indices) in grouped_indices {
            let gpu_entries = indices
                .iter()
                .map(|index| {
                    let entry = &prepared[*index];
                    GpuFastPlasticityBatchEntry {
                        slot: &entry.brain_slot,
                        pending: &entry.pending_record,
                        outcome: entry.outcome,
                        active_weight_generation: entry.active_weight_generation,
                        replay_generation: entry.replay_journal_generation,
                        transaction_generation: entry.transaction_generation,
                    }
                })
                .collect::<Vec<_>>();
            let gpu_result = {
                let bucket = self
                    .class_buckets
                    .get_mut(&class_id)
                    .and_then(|pool| pool.chunks.get_mut(chunk_index))
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                bucket.pipelines.apply_fast_plasticity(
                    &self.device,
                    &self.queue,
                    &bucket.buffers,
                    &gpu_entries,
                    GpuTimestampQueryResources::new(
                        &self.plasticity_timestamp_resources.query_set,
                        &self.plasticity_timestamp_resources.resolve_buffer,
                        &self.plasticity_timestamp_resources.readback_buffer,
                    ),
                )
            };
            let gpu_timed_result = match gpu_result {
                Ok(result) => result,
                Err(
                    GpuClosedLoopError::MalformedUpload | GpuClosedLoopError::StaleOrForeignHandle,
                ) => return Err(ScaffoldContractError::LearningEvidenceMismatch),
                Err(_) => {
                    self.mark_device_lost();
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
            };
            if gpu_timed_result.records.len() != indices.len() {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            plasticity_timestamp_ticks = plasticity_timestamp_ticks
                .checked_add(gpu_timed_result.timestamp_delta_ticks)
                .ok_or(ScaffoldContractError::GpuTimestampQueryUnavailable)?;
            for (index, record) in indices.into_iter().zip(gpu_timed_result.records) {
                ordered_gpu_records[index] = Some(record);
            }
        }
        let gpu_records = ordered_gpu_records
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        if gpu_records.len() != prepared.len() {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let host_precommit_valid = prepared.iter().zip(&gpu_records).all(|(entry, record)| {
            self.class_buckets
                .get(&class_id)
                .and_then(|pool| pool.resident(entry.handle).ok())
                .is_some_and(|resident| {
                    resident.pending_eligibility == Some(entry.pending_receipt)
                        && resident.pending_eligibility_record == Some(entry.pending_record)
                        && resident.active_weight_generation == entry.active_weight_generation
                        && resident.active_eligibility_generation
                            == entry.active_eligibility_generation
                        && resident.replay_journal_generation == entry.replay_journal_generation
                        && resident.transaction_generation == entry.transaction_generation
                        && resident.learning_sequence_guard.last_committed()
                            == entry.expected_last_committed
                        && record.input_fast_generation() == entry.active_weight_generation
                        && record.output_eligibility_generation()
                            == entry
                                .pending_receipt
                                .identity()
                                .staging_eligibility_generation()
                })
        });
        if !host_precommit_valid {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let hardware_receipt_generation = self.hardware.generation;
        let readback_bytes = prepared
            .len()
            .checked_mul(crate::GPU_FAST_PLASTICITY_COMMIT_BYTES)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let mut receipts = Vec::with_capacity(prepared.len());
        for (entry, record) in prepared.into_iter().zip(gpu_records) {
            let guard_commit = self
                .class_buckets
                .get_mut(&class_id)
                .and_then(|pool| pool.resident_mut(entry.handle).ok())
                .expect("learning host commit was prevalidated")
                .learning_sequence_guard
                .commit_validated(entry.commit_token);
            if guard_commit.is_err() {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            let resident = self
                .class_buckets
                .get_mut(&class_id)
                .and_then(|pool| pool.resident_mut(entry.handle).ok())
                .expect("learning guard commit retained the resident slot");
            resident.active_weight_bank ^= 1;
            resident.active_eligibility_bank ^= 1;
            resident.active_weight_generation = record.output_fast_generation();
            resident.active_eligibility_generation = record.output_eligibility_generation();
            resident.replay_journal_generation = record.replay_generation();
            resident.transaction_generation = record.transaction_generation();
            resident.pending_eligibility = None;
            resident.pending_eligibility_record = None;
            receipts.push(GpuLearningReceipt {
                handle: entry.handle,
                sequence_id: entry.packet.sequence_id(),
                dispatch_generation: entry.packet.dispatch_generation(),
                active_activation_side: entry.packet.active_activation_side(),
                input_fast_generation: record.input_fast_generation(),
                output_fast_generation: record.output_fast_generation(),
                output_eligibility_generation: record.output_eligibility_generation(),
                replay_journal_generation: record.replay_generation(),
                fast_weights_changed: record.fast_weights_changed,
                max_abs_delta: record.max_abs_delta(),
                hardware_receipt_generation,
            });
        }
        self.last_compact_readback_bytes = readback_bytes;
        if let Some(pending) = self.pending_inference_timing {
            let dispatch_generation = receipts
                .first()
                .map(|receipt| receipt.dispatch_generation)
                .unwrap_or(0);
            if pending.dispatch_generation == dispatch_generation
                && pending.class_id_raw == Some(class_id)
                && usize::try_from(pending.population).ok() == Some(receipts.len())
            {
                let inference_period_ns_q24 = self.timestamp_resources.period_ns_q24()?;
                let plasticity_period_ns_q24 =
                    self.plasticity_timestamp_resources.period_ns_q24()?;
                if inference_period_ns_q24 != plasticity_period_ns_q24 {
                    return Err(ScaffoldContractError::GpuTimestampQueryUnavailable);
                }
                self.completed_neural_timing = Some(GpuNeuralTimingSample {
                    dispatch_generation,
                    class_id_raw: class_id,
                    population: pending.population,
                    inference_timestamp_ticks: pending.inference_timestamp_ticks,
                    plasticity_timestamp_ticks,
                    timestamp_period_ns_q24: inference_period_ns_q24,
                });
                self.pending_inference_timing = None;
            }
        }
        Ok(receipts)
    }

    pub fn discard_pending_eligibility(
        &mut self,
        handle: GpuBrainHandle,
        identity: &PendingEligibilityIdentity,
    ) -> Result<PendingEligibilityDiscardReceipt, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let (brain_slot, pending_receipt, pending_record, transaction_generation) = {
            let pool = self
                .class_buckets
                .get(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = pool.resident(handle)?;
            (
                resident.brain_slot.clone(),
                resident
                    .pending_eligibility
                    .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?,
                resident
                    .pending_eligibility_record
                    .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?,
                resident.transaction_generation,
            )
        };
        if pending_receipt.identity() != identity
            || identity.handle_generation() != handle.generation
            || identity.phenotype_hash() != handle.phenotype_hash
        {
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        #[cfg(feature = "gpu-tests")]
        if self.forced_discard_rejections_remaining > 0 {
            self.forced_discard_rejections_remaining -= 1;
            return Err(ScaffoldContractError::LearningEvidenceMismatch);
        }
        let discard_result = {
            let bucket = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .bucket_for_handle_mut(handle)?;
            bucket.pipelines.discard_pending_eligibility(
                &self.device,
                &self.queue,
                &bucket.buffers,
                &brain_slot,
                &pending_record,
                transaction_generation,
            )
        };
        let discard_record = match discard_result {
            Ok(record) => record,
            Err(_) => {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        let next_transaction_generation = transaction_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::LearningEvidenceMismatch)?;
        if discard_record.active_eligibility_generation()
            != identity.active_eligibility_generation()
            || discard_record.discarded_staging_generation()
                != identity.staging_eligibility_generation()
            || discard_record.transaction_generation() != next_transaction_generation
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let resident = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .and_then(|pool| pool.resident_mut(handle).ok())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        resident.transaction_generation = next_transaction_generation;
        resident.pending_eligibility = None;
        resident.pending_eligibility_record = None;
        if self
            .pending_inference_timing
            .is_some_and(|pending| pending.dispatch_generation == identity.dispatch_generation())
        {
            self.pending_inference_timing = None;
        }
        Ok(PendingEligibilityDiscardReceipt::new(
            *identity,
            self.hardware.generation,
        ))
    }

    pub fn prepare_memory_context_upload(
        &mut self,
        handle: GpuBrainHandle,
        frame: &PerceptionFrame,
        recall: &FinalizedMemoryRecall,
    ) -> Result<GpuMemoryContextUpload, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        frame.validate()?;
        let pool = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = pool.resident(handle)?;
        if resident.ownership.organism_id != frame.organism_id()
            || handle.organism_id != frame.organism_id()
            || resident.ownership.sensor_profile != frame.sensor_profile()
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let perception = GpuPerceptionUpload::try_from_frame(frame, &resident.brain_slot, 0)
            .map_err(map_gpu_contract_error)?;
        GpuMemoryContextUpload::try_from_finalized(
            frame,
            recall,
            perception.frame_binding,
            &resident.brain_slot,
        )
        .map_err(map_gpu_contract_error)
    }

    pub fn tick_batch(
        &mut self,
        batch: &[(GpuBrainHandle, PerceptionFrame)],
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        let inputs = batch
            .iter()
            .map(|(handle, frame)| GpuRuntimeTickInput {
                handle: *handle,
                frame,
                memory_upload: None,
            })
            .collect::<Vec<_>>();
        self.tick_inputs(&inputs)
    }

    pub fn tick_memory_batch(
        &mut self,
        batch: &GpuClosedLoopMemoryBatchInput<'_>,
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        let inputs = batch
            .members
            .iter()
            .map(|member| GpuRuntimeTickInput {
                handle: member.handle,
                frame: member.frame,
                memory_upload: Some(member.memory_upload),
            })
            .collect::<Vec<_>>();
        self.tick_inputs(&inputs)
    }

    fn tick_inputs(
        &mut self,
        batch: &[GpuRuntimeTickInput<'_>],
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        self.ensure_ready()?;
        if batch.is_empty() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let mut seen_handles = BTreeSet::new();
        let mut seen_organisms = BTreeSet::new();
        let mut grouped = BTreeMap::<(u16, usize), Vec<usize>>::new();
        for (index, input) in batch.iter().enumerate() {
            let handle = input.handle;
            let frame = input.frame;
            self.validate_handle_backend(handle)?;
            if !seen_handles.insert((handle.class_id.raw(), handle.slot, handle.generation))
                || !seen_organisms.insert(handle.organism_id.0)
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            frame.validate()?;
            let pool = self
                .class_buckets
                .get(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let chunk_index = pool
                .bucket_index_for_handle(handle)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = pool.resident(handle)?;
            if resident.ownership.organism_id != frame.organism_id()
                || handle.organism_id != frame.organism_id()
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            if resident.ownership.sensor_profile != frame.sensor_profile() {
                return Err(ScaffoldContractError::SensorProfileMismatch);
            }
            if resident.pending_eligibility.is_some() {
                return Err(ScaffoldContractError::LearningReplayRejected);
            }
            if resident.activity_sequence_cursor.checked_add(1).is_none() {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
            grouped
                .entry((handle.class_id.raw(), chunk_index))
                .or_default()
                .push(index);
        }

        let dispatch_generation = NonZeroU64::new(self.next_dispatch_generation)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_dispatch_generation = self
            .next_dispatch_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_upload_count = self
            .perception_upload_count
            .checked_add(batch.len() as u64)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_completed_dispatch_count = self
            .completed_dispatch_count
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_completed_selection_count = self
            .completed_selection_count
            .checked_add(batch.len() as u64)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let replayed_pressure = if self.recorded_pressure_replay.is_empty() {
            None
        } else {
            if self.recorded_pressure_replay.len() < batch.len() {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
            Some(
                self.recorded_pressure_replay
                    .drain(..batch.len())
                    .collect::<Vec<_>>(),
            )
        };
        let mut replayed_pressure_iter = replayed_pressure.as_deref().map(<[_]>::iter);
        let activity_decisions = batch
            .iter()
            .map(|input| {
                let handle = input.handle;
                let resident = self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let identity = BrainDispatchIdentity {
                    organism_id_raw: handle.organism_id.raw(),
                    tick: input.frame.tick().raw(),
                    class_id_raw: handle.class_id.raw(),
                    handle_slot: handle.slot,
                    handle_generation: handle.generation,
                    sequence_cursor: resident.activity_sequence_cursor,
                    dispatch_generation: dispatch_generation.get(),
                    frame_digest: input.frame.frame_digest().0,
                };
                let pressure = match replayed_pressure_iter
                    .as_mut()
                    .and_then(|samples| samples.next())
                    .copied()
                {
                    Some(sample) => {
                        sample.validate_for(&self.activity_policy)?;
                        if sample.dispatch_identity() != identity
                            || sample.source_dispatch_generation
                                != resident.last_activity_dispatch_generation
                            || sample.source_frame_digest != resident.last_activity_frame_digest
                        {
                            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                        }
                        sample
                    }
                    None => live_pressure_sample(
                        &self.activity_policy,
                        identity,
                        resident,
                        &self.admission,
                        &self.runtime_budget,
                    )?,
                };
                let capacity = capacity_for_promoted_class(handle.class_id)?;
                NeuralThrottleDecision::derive(
                    &self.activity_policy,
                    &resident.phenotype,
                    capacity.execution(),
                    identity,
                    pressure,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let first_class_id = batch[0].handle.class_id.raw();
        let timing_class_id = batch
            .iter()
            .all(|input| input.handle.class_id.raw() == first_class_id)
            .then_some(first_class_id);
        let mut dispatches = Vec::with_capacity(grouped.len());
        for ((class_id, chunk_index), original_indices) in grouped {
            let bucket = self
                .class_buckets
                .get(&class_id)
                .and_then(|pool| pool.chunks.get(chunk_index))
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let entries = original_indices
                .iter()
                .map(|index| {
                    let input = batch[*index];
                    let resident = bucket.slots[input.handle.slot as usize]
                        .as_ref()
                        .expect("complete preflight retained occupied slot");
                    match input.memory_upload {
                        Some(memory_upload) => GpuFixedActiveBatchEntry::with_memory(
                            input.frame,
                            &resident.brain_slot,
                            &resident.phenotype,
                            &activity_decisions[*index],
                            memory_upload,
                            resident.active_eligibility_generation,
                        ),
                        None => GpuFixedActiveBatchEntry::new(
                            input.frame,
                            &resident.brain_slot,
                            &resident.phenotype,
                            &activity_decisions[*index],
                            resident.active_eligibility_generation,
                        ),
                    }
                })
                .collect::<Vec<_>>();
            let prepared = bucket
                .pipelines
                .preflight_fixed_active_batch(&entries, 0, dispatch_generation)
                .map_err(map_gpu_contract_error)?;
            dispatches.push(PreparedClassDispatch {
                class_id,
                chunk_index,
                original_indices,
                prepared: Some(prepared),
                batch: None,
                recorded: false,
                map_ticket: None,
                validated: None,
            });
        }

        for index in 0..dispatches.len() {
            let class_id = dispatches[index].class_id;
            let prepared = dispatches[index]
                .prepared
                .take()
                .expect("prepared exactly once");
            let result = self
                .class_buckets
                .get_mut(&class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatches[index].chunk_index))
                .expect("preflight bucket exists")
                .pipelines
                .begin_prepared_batch(prepared);
            match result {
                Ok(active) => dispatches[index].batch = Some(active),
                Err(error) => {
                    for prior in &mut dispatches[..index] {
                        if let Some(active) = prior.batch.take() {
                            let _ = self
                                .class_buckets
                                .get_mut(&prior.class_id)
                                .and_then(|pool| pool.chunks.get_mut(prior.chunk_index))
                                .expect("prior bucket exists")
                                .pipelines
                                .abandon_unsubmitted_batch(active);
                        }
                    }
                    return Err(map_gpu_contract_error(error));
                }
            }
        }

        for index in 0..dispatches.len() {
            let dispatch = &dispatches[index];
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                .expect("prepared bucket exists");
            if let Err(error) = bucket.pipelines.write_staged_uploads(
                &self.queue,
                &bucket.buffers,
                dispatch.batch.as_ref().expect("begun batch"),
            ) {
                self.cleanup_unsubmitted_dispatches(&mut dispatches);
                return Err(map_gpu_contract_error(error));
            }
        }
        self.perception_upload_count = next_upload_count;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("closed-loop-runtime-mixed-class-tick"),
            });
        {
            let _timestamp_start = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-runtime-timestamp-start"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &self.timestamp_resources.query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: None,
                }),
            });
        }
        for index in 0..dispatches.len() {
            let dispatch = &mut dispatches[index];
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                .expect("begun bucket exists");
            if let Err(error) = bucket.pipelines.record_staged_closed_loop(
                &mut encoder,
                &bucket.buffers,
                dispatch.batch.as_ref().expect("begun batch"),
            ) {
                self.cleanup_unsubmitted_dispatches(&mut dispatches);
                return Err(map_gpu_contract_error(error));
            }
            dispatch.recorded = true;
        }
        {
            let _timestamp_end = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-runtime-timestamp-end"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &self.timestamp_resources.query_set,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: Some(1),
                }),
            });
        }
        encoder.resolve_query_set(
            &self.timestamp_resources.query_set,
            0..GPU_TIMESTAMP_QUERY_COUNT,
            &self.timestamp_resources.resolve_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.timestamp_resources.resolve_buffer,
            0,
            &self.timestamp_resources.readback_buffer,
            0,
            GPU_TIMESTAMP_READBACK_BYTES,
        );
        let command_buffer = encoder.finish();
        for index in 0..dispatches.len() {
            let dispatch = &mut dispatches[index];
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                .expect("recorded bucket exists");
            match bucket.pipelines.register_compact_mapping(
                &command_buffer,
                &bucket.buffers,
                dispatch.batch.as_ref().expect("recorded batch"),
            ) {
                Ok(ticket) => dispatch.map_ticket = Some(ticket),
                Err(error) => {
                    self.cleanup_unsubmitted_dispatches(&mut dispatches);
                    return Err(map_gpu_contract_error(error));
                }
            }
        }
        let (timestamp_sender, timestamp_receiver) = std::sync::mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &self.timestamp_resources.readback_buffer,
            wgpu::MapMode::Read,
            0..GPU_TIMESTAMP_READBACK_BYTES,
            move |result| {
                let _ = timestamp_sender.send(result);
            },
        );
        let submission = self.queue.submit(Some(command_buffer));
        let forced_loss = std::mem::take(&mut self.force_device_lost_after_submit);
        let poll_failed = self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err();
        let mappings_succeeded = dispatches.iter_mut().all(|dispatch| {
            dispatch
                .map_ticket
                .take()
                .is_some_and(GpuCompactMapTicket::mapping_succeeded)
        });
        let timestamp_mapping_succeeded = timestamp_mapping_completed(&timestamp_receiver);
        if forced_loss
            || poll_failed
            || !mappings_succeeded
            || !timestamp_mapping_succeeded
            || self.device_lost.load(Ordering::Acquire)
        {
            for dispatch in &dispatches {
                let bucket = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                    .expect("submitted bucket exists");
                bucket.buffers.compact_readback().unmap();
                let _ = bucket
                    .pipelines
                    .mark_post_submit_poison(dispatch.batch.as_ref().expect("submitted batch"));
            }
            self.timestamp_resources.readback_buffer.unmap();
            self.mark_device_lost();
            return Err(if !timestamp_mapping_succeeded {
                ScaffoldContractError::GpuTimestampQueryUnavailable
            } else {
                ScaffoldContractError::NeuralBackendUnavailable
            });
        }

        let (inference_timestamp_ticks, completed_gpu_time_ns) =
            match self.timestamp_resources.read_delta_and_elapsed_ns() {
                Ok(timing) => timing,
                Err(error) => {
                    for dispatch in &dispatches {
                        self.class_buckets
                            .get(&dispatch.class_id)
                            .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                            .expect("submitted bucket exists")
                            .buffers
                            .compact_readback()
                            .unmap();
                    }
                    self.poison_submitted_dispatches(&dispatches);
                    return Err(error);
                }
            };

        for index in 0..dispatches.len() {
            let dispatch = &dispatches[index];
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                .expect("mapped bucket exists");
            match bucket.pipelines.decode_validate_mapped_records(
                &bucket.buffers,
                dispatch.batch.as_ref().expect("mapped batch"),
            ) {
                Ok(validated) => dispatches[index].validated = Some(validated),
                Err(_) => {
                    for still_mapped in &dispatches[index + 1..] {
                        self.class_buckets
                            .get(&still_mapped.class_id)
                            .and_then(|pool| pool.chunks.get(still_mapped.chunk_index))
                            .expect("submitted bucket exists")
                            .buffers
                            .compact_readback()
                            .unmap();
                    }
                    for submitted in &dispatches {
                        let bucket = self
                            .class_buckets
                            .get_mut(&submitted.class_id)
                            .and_then(|pool| pool.chunks.get_mut(submitted.chunk_index))
                            .expect("submitted bucket exists");
                        let _ = bucket.pipelines.mark_post_submit_poison(
                            submitted.batch.as_ref().expect("submitted batch"),
                        );
                    }
                    self.mark_device_lost();
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
            }
        }

        if dispatches.iter().any(|dispatch| {
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                .expect("validated bucket exists");
            bucket
                .pipelines
                .prevalidate_commit_validated_batch(
                    dispatch.validated.as_ref().expect("validated batch"),
                )
                .is_err()
        }) {
            for dispatch in &dispatches {
                let bucket = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                    .expect("validated bucket exists");
                let _ = bucket
                    .pipelines
                    .mark_post_submit_poison(dispatch.batch.as_ref().expect("submitted batch"));
            }
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        let mut ordered_records = vec![None; batch.len()];
        let mut ordered_pending_receipts = vec![None; batch.len()];
        let mut ordered_pending_records = vec![None; batch.len()];
        let mut ordered_next_transaction_generations = vec![None; batch.len()];
        let mut ordered_memory_receipts = vec![None; batch.len()];
        let receipt_validation = (|| -> Result<(), ScaffoldContractError> {
            for dispatch in &dispatches {
                let validated = dispatch.validated.as_ref().expect("validated batch");
                let memory_bindings = dispatch
                    .batch
                    .as_ref()
                    .expect("validated batch retains its upload")
                    .memory_context_bindings();
                for (((original_index, selection), pending_record), memory_binding) in dispatch
                    .original_indices
                    .iter()
                    .zip(validated.records())
                    .zip(validated.pending_records())
                    .zip(memory_bindings)
                {
                    if selection.status != 1 {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    let input = batch[*original_index];
                    let handle = input.handle;
                    let frame = input.frame;
                    let candidate_index = u16::try_from(selection.candidate_index)
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                    let candidate = frame
                        .candidates()
                        .get(candidate_index as usize)
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    let receipt = PendingEligibilityReceipt::from_gpu_record(
                        *pending_record,
                        handle.slot,
                        handle.organism_id,
                        handle.phenotype_hash,
                    )?;
                    let identity = receipt.identity();
                    let resident = self
                        .class_buckets
                        .get(&handle.class_id.raw())
                        .and_then(|pool| pool.resident(handle).ok())
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    if identity.handle_generation() != handle.generation
                        || identity.dispatch_generation() != dispatch_generation.get()
                        || identity.originating_tick() != frame.tick()
                        || identity.frame_digest() != frame.frame_digest()
                        || u32::from(identity.active_activation_side())
                            != selection.active_activation_side
                        || identity.candidate_index() != candidate_index
                        || identity.action_id() != candidate.action_id
                        || identity.action_family() != candidate.family
                        || identity.candidate_feature_digest() != candidate.feature_digest()?
                        || identity.active_eligibility_generation()
                            != resident.active_eligibility_generation
                        || identity.staging_eligibility_generation()
                            != resident
                                .active_eligibility_generation
                                .checked_add(1)
                                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?
                    {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    match (input.memory_upload, *memory_binding) {
                        (None, None) => {}
                        (Some(_), Some(memory_receipt))
                            if memory_receipt.slot == handle.slot
                                && memory_receipt.slot_generation == handle.generation
                                && memory_receipt.base_frame_digest == frame.base_digest()
                                && memory_receipt.context_digest
                                    == frame.context().canonical_digest()
                                && memory_receipt.final_frame_digest == frame.frame_digest()
                                && usize::from(memory_receipt.candidate_count)
                                    == frame.candidates().len() =>
                        {
                            ordered_memory_receipts[*original_index] = Some(memory_receipt);
                        }
                        _ => return Err(ScaffoldContractError::InvalidDecisionEvidence),
                    }
                    let next_transaction_generation = resident
                        .transaction_generation
                        .checked_add(1)
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    ordered_records[*original_index] = Some(*selection);
                    ordered_pending_receipts[*original_index] = Some(receipt);
                    ordered_pending_records[*original_index] = Some(*pending_record);
                    ordered_next_transaction_generations[*original_index] =
                        Some(next_transaction_generation);
                }
            }
            Ok(())
        })();
        if receipt_validation.is_err() {
            self.poison_submitted_dispatches(&dispatches);
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        let activity_work_receipts: Result<Vec<BrainWorkReceipt>, ScaffoldContractError> = batch
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let handle = input.handle;
                let resident = self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let decision = &activity_decisions[index];
                let candidate_count = u32::try_from(input.frame.candidates().len())
                    .map_err(|_| ScaffoldContractError::BrainActivityPolicyMismatch)?;
                let memory_context_count = input
                    .memory_upload
                    .map_or(0, |upload| upload.header.candidate_count);
                let work = derive_executed_work(
                    &resident.phenotype,
                    decision.microsteps,
                    &decision.enabled_route_ids,
                    candidate_count,
                    memory_context_count,
                )?;
                let record =
                    ordered_records[index].ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                if work.tile_visits != u64::from(record.active_tiles)
                    || work.synapse_ops != u64::from(record.active_synapses)
                {
                    return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                }
                BrainWorkReceipt::try_new(
                    &self.activity_policy,
                    decision,
                    work,
                    resident.brain_atp_q16,
                )
            })
            .collect();
        let activity_work_receipts = match activity_work_receipts {
            Ok(receipts) => receipts,
            Err(error) => {
                self.poison_submitted_dispatches(&dispatches);
                return Err(error);
            }
        };

        let prepared_ticks = (|| -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
            let mut ticks = Vec::with_capacity(batch.len());
            for (index, input) in batch.iter().enumerate() {
                let handle = input.handle;
                let frame = input.frame;
                let record =
                    ordered_records[index].ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let pending_eligibility = ordered_pending_receipts[index]
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let candidate_index = u16::try_from(record.candidate_index)
                    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                let candidate = frame
                    .candidates()
                    .get(candidate_index as usize)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                ticks.push(GpuClosedLoopTick {
                    handle,
                    dispatch_generation: dispatch_generation.get(),
                    base_digest: frame.base_digest(),
                    frame_digest: frame.frame_digest(),
                    memory_context_binding: ordered_memory_receipts[index],
                    active_activation_side: u8::try_from(record.active_activation_side)
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?,
                    selection: NeuralActionSelection {
                        candidate_index,
                        logit: f32::from_bits(record.logit_bits),
                        confidence: Confidence::new(candidate.sensor_confidence.raw())?,
                        active_tiles: record.active_tiles,
                        active_synapses: record.active_synapses,
                    },
                    pending_eligibility,
                    pressure: activity_decisions[index].pressure,
                    throttle: activity_decisions[index].clone(),
                    work: activity_work_receipts[index].clone(),
                    compact_readback_bytes: crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES,
                    hardware_receipt_generation: self.hardware.generation,
                });
            }
            Ok(ticks)
        })();
        let prepared_ticks = match prepared_ticks {
            Ok(ticks) => ticks,
            Err(_) => {
                self.poison_submitted_dispatches(&dispatches);
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        let total_readback_bytes = match batch
            .len()
            .checked_mul(crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES)
        {
            Some(bytes) => bytes,
            None => {
                self.poison_submitted_dispatches(&dispatches);
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        let mut commit_mismatch = false;
        for dispatch in &mut dispatches {
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                .expect("validated bucket exists");
            let commit = bucket
                .pipelines
                .commit_validated_batch(dispatch.validated.take().expect("validated batch"));
            let committed = match commit {
                Ok(committed) => committed,
                Err(_) => {
                    self.mark_device_lost();
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
            };
            if committed.readback_bytes as usize
                != dispatch.original_indices.len() * crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES
                || committed.records.len() != dispatch.original_indices.len()
                || committed.pending_records.len() != dispatch.original_indices.len()
            {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            for ((original_index, record), pending_record) in dispatch
                .original_indices
                .iter()
                .zip(committed.records)
                .zip(committed.pending_records)
            {
                commit_mismatch |= ordered_records[*original_index] != Some(record)
                    || ordered_pending_records[*original_index] != Some(pending_record);
            }
        }
        if commit_mismatch {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        let host_precommit_valid = batch.iter().enumerate().all(|(index, input)| {
            let handle = input.handle;
            let Some(expected_generation) = ordered_next_transaction_generations[index] else {
                return false;
            };
            ordered_pending_receipts[index].is_some()
                && ordered_pending_records[index].is_some()
                && self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .is_some_and(|resident| {
                        resident.pending_eligibility.is_none()
                            && resident.pending_eligibility_record.is_none()
                            && resident.activity_sequence_cursor
                                == activity_decisions[index].sequence_cursor
                            && resident.brain_atp_q16
                                == activity_work_receipts[index].atp_before_q16
                            && activity_work_receipts[index]
                                .validate_for(&self.activity_policy, &activity_decisions[index])
                                .is_ok()
                            && resident.transaction_generation.checked_add(1)
                                == Some(expected_generation)
                    })
        });
        if !host_precommit_valid {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        for (index, input) in batch.iter().enumerate() {
            let handle = input.handle;
            let resident = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .and_then(|pool| pool.resident_mut(handle).ok())
                .expect("host pending commit was prevalidated");
            resident.transaction_generation = ordered_next_transaction_generations[index]
                .expect("host transaction generation was prevalidated");
            resident.logical_dispatch_generation = dispatch_generation.get();
            resident.activity_sequence_cursor = resident
                .activity_sequence_cursor
                .checked_add(1)
                .expect("activity cursor was prevalidated");
            resident.brain_atp_q16 = activity_work_receipts[index].atp_after_q16;
            resident.last_activity_dispatch_generation = dispatch_generation.get();
            resident.last_activity_frame_digest = input.frame.frame_digest().0;
            resident.last_completed_gpu_time_ns = completed_gpu_time_ns;
            resident.last_pressure = Some(activity_decisions[index].pressure);
            resident.last_throttle = Some(activity_decisions[index].clone());
            resident.last_work = Some(activity_work_receipts[index].clone());
            resident.pending_eligibility = ordered_pending_receipts[index];
            resident.pending_eligibility_record = ordered_pending_records[index];
        }

        self.completed_dispatch_count = next_completed_dispatch_count;
        self.last_compact_readback_bytes = total_readback_bytes;
        self.next_dispatch_generation = next_dispatch_generation;
        self.completed_selection_count = next_completed_selection_count;
        self.completed_neural_timing = None;
        self.pending_inference_timing = Some(PendingInferenceTiming {
            dispatch_generation: dispatch_generation.get(),
            class_id_raw: timing_class_id,
            population: u32::try_from(batch.len())
                .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?,
            inference_timestamp_ticks,
        });
        Ok(prepared_ticks)
    }

    fn current_admission_snapshot(&self) -> Result<GpuAdmissionReceipt, ScaffoldContractError> {
        let mut logical_committed_bytes = 0_u64;
        let mut physical_allocated_bytes = 0_u64;
        let mut physical_unused_retained_bytes = 0_u64;
        let mut physical_shared_bytes = 0_u64;
        let mut physical_alignment_slack_bytes = 0_u64;
        let mut live_brains = 0_u32;
        for pool in self.class_buckets.values() {
            for bucket in &pool.chunks {
                let receipt = bucket
                    .plan
                    .slot_allocation_receipt()
                    .map_err(map_gpu_contract_error)?;
                let live = u64::try_from(bucket.slots.iter().filter(|slot| slot.is_some()).count())
                    .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
                let slots = u64::from(bucket.plan.slot_capacity());
                let unused = slots
                    .checked_sub(live)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                logical_committed_bytes = logical_committed_bytes
                    .checked_add(
                        receipt
                            .logical_slot_commit_bytes
                            .checked_mul(live)
                            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?,
                    )
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                physical_unused_retained_bytes = physical_unused_retained_bytes
                    .checked_add(
                        receipt
                            .logical_slot_commit_bytes
                            .checked_mul(unused)
                            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?,
                    )
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                physical_shared_bytes = physical_shared_bytes
                    .checked_add(receipt.shared_class_bytes)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                physical_alignment_slack_bytes = physical_alignment_slack_bytes
                    .checked_add(
                        receipt
                            .alignment_padding_bytes
                            .checked_mul(slots)
                            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?,
                    )
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                physical_allocated_bytes = physical_allocated_bytes
                    .checked_add(bucket.plan.aggregate_resident_bytes())
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                live_brains = live_brains
                    .checked_add(
                        u32::try_from(live)
                            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?,
                    )
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
            }
        }
        let logical_available_bytes = self
            .runtime_budget
            .logical_neural_heap_budget_bytes
            .checked_sub(logical_committed_bytes)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let receipt = GpuAdmissionReceipt {
            schema_version: 1,
            runtime: self.runtime_budget,
            logical_committed_bytes,
            logical_available_bytes,
            physical_allocated_bytes,
            physical_unused_retained_bytes,
            physical_shared_bytes,
            physical_alignment_slack_bytes,
            peak_logical_committed_bytes: self
                .admission
                .peak_logical_committed_bytes
                .max(logical_committed_bytes),
            peak_physical_allocated_bytes: self
                .admission
                .peak_physical_allocated_bytes
                .max(physical_allocated_bytes),
            live_brains,
            max_hot_brains: self.runtime_budget.max_hot_brains,
            allocation_generation: 0,
            last_event: None,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }

    fn commit_admission_event(
        &mut self,
        kind: GpuAllocationEventKind,
        handle: GpuBrainHandle,
        transient_peak_physical_bytes: u64,
    ) -> Result<(), ScaffoldContractError> {
        let before = self.admission.clone();
        let mut after = self.current_admission_snapshot()?;
        after.allocation_generation = before
            .allocation_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        after.peak_logical_committed_bytes = before
            .peak_logical_committed_bytes
            .max(after.logical_committed_bytes);
        after.peak_physical_allocated_bytes = before
            .peak_physical_allocated_bytes
            .max(after.physical_allocated_bytes)
            .max(transient_peak_physical_bytes);
        after.last_event = Some(GpuAllocationEventReceipt::new(
            kind,
            handle.class_id.raw(),
            handle.slot,
            handle.generation,
            &before,
            &after,
        )?);
        after.validate_contract()?;
        self.admission = after;
        Ok(())
    }

    fn validate_logical_admission(
        &self,
        slot_receipt: &crate::GpuSlotAllocationReceipt,
    ) -> Result<(), ScaffoldContractError> {
        if self.admission.live_brains >= self.runtime_budget.max_hot_brains
            || self
                .admission
                .logical_committed_bytes
                .checked_add(slot_receipt.logical_slot_commit_bytes)
                .is_none_or(|bytes| bytes > self.runtime_budget.logical_neural_heap_budget_bytes)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(())
    }

    pub fn insert_brain(
        &mut self,
        organism_id: OrganismId,
        phenotype: BrainPhenotype,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        self.ensure_ready()?;
        organism_id
            .validate()
            .map_err(|_| ScaffoldContractError::BrainOwnershipMismatch)?;
        if self.organisms.contains_key(&organism_id.0) {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let class_id = phenotype.brain_class_id();
        let capacity = capacity_for_promoted_class(class_id)?;
        validate_required_gpu_layout_version(u32::from(capacity.execution().gpu_layout_version()))?;
        phenotype
            .validate_against(&capacity)
            .map_err(|_| ScaffoldContractError::GpuLayoutMismatch)?;
        self.runtime_budget.validate_for(capacity.execution())?;
        crate::GpuClassBucketPlan::validate_adapter(&phenotype, &self.runtime_budget)
            .map_err(map_gpu_contract_error)?;
        let slot_receipt = GpuFixedClassArenaPlan::new(
            capacity,
            1,
            self.runtime_budget.physical_allocation_ceiling_bytes,
        )
        .map_err(map_gpu_contract_error)?
        .slot_allocation_receipt()
        .map_err(map_gpu_contract_error)?;
        self.validate_logical_admission(&slot_receipt)?;
        let class_raw = class_id.raw();
        let current_physical = self.admission.physical_allocated_bytes;
        let reusable = self
            .class_buckets
            .get(&class_raw)
            .and_then(|pool| pool.reusable_slot(class_raw, &self.slot_generation_watermarks));
        let (chunk_index, slot, generation, upload, event_kind, transient_peak_physical_bytes) =
            if let Some((chunk_index, slot, generation)) = reusable {
                let bucket = self
                    .class_buckets
                    .get(&class_raw)
                    .and_then(|pool| pool.chunks.get(chunk_index))
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let upload = bucket
                    .plan
                    .prepare_slot_upload(slot, generation, &phenotype)
                    .map_err(map_gpu_contract_error)?;
                (
                    chunk_index,
                    slot,
                    generation,
                    upload,
                    GpuAllocationEventKind::AdmitFromRetainedSlot,
                    current_physical,
                )
            } else {
                let remaining_hot = self
                    .runtime_profile
                    .max_hot_brains
                    .checked_sub(self.admission.live_brains)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                let slot_capacity =
                    u32::from(self.runtime_profile.growth_chunk_slots).min(remaining_hot);
                if slot_capacity == 0 {
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
                let plan = GpuFixedClassArenaPlan::new(
                    capacity,
                    slot_capacity,
                    self.runtime_budget.physical_allocation_ceiling_bytes,
                )
                .map_err(map_gpu_contract_error)?;
                let transient_peak = current_physical
                    .checked_add(plan.aggregate_resident_bytes())
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                if transient_peak > self.runtime_budget.physical_allocation_ceiling_bytes {
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
                let mut bucket =
                    ClassBucketRuntime::from_plan(&self.device, Arc::clone(&self.kernels), plan)
                        .map_err(map_gpu_contract_error)?;
                for candidate_slot in 0..slot_capacity {
                    if let Some(previous) = self
                        .slot_generation_watermarks
                        .get(&(class_raw, candidate_slot))
                        .copied()
                    {
                        bucket.generations[candidate_slot as usize] = previous;
                        if previous == u32::MAX {
                            bucket.retired.insert(candidate_slot);
                            bucket
                                .free_slots
                                .retain(|free_slot| *free_slot != candidate_slot);
                        }
                    }
                }
                let slot = *bucket
                    .free_slots
                    .last()
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                let generation = self
                    .slot_generation_watermarks
                    .get(&(class_raw, slot))
                    .copied()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
                let upload = bucket
                    .plan
                    .prepare_slot_upload(slot, generation, &phenotype)
                    .map_err(map_gpu_contract_error)?;
                let pool = self.class_buckets.entry(class_raw).or_default();
                let chunk_index = pool.chunks.len();
                pool.chunks.push(bucket);
                (
                    chunk_index,
                    slot,
                    generation,
                    upload,
                    GpuAllocationEventKind::AdmitFromNewChunk,
                    transient_peak,
                )
            };
        let bucket = self
            .class_buckets
            .get_mut(&class_raw)
            .and_then(|pool| pool.chunks.get_mut(chunk_index))
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        bucket
            .buffers
            .write_slot_upload(&self.queue, &upload)
            .map_err(map_gpu_contract_error)?;
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("closed-loop-runtime-slot-initialize"),
            });
        bucket
            .buffers
            .record_mutable_slot_reset(&mut encoder, upload.ranges())
            .map_err(map_gpu_contract_error)?;
        let submission = self.queue.submit(Some(encoder.finish()));
        if self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
            || self.device_lost.load(Ordering::Acquire)
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        bucket
            .buffers
            .write_mutable_slot_upload(&self.queue, &upload)
            .map_err(map_gpu_contract_error)?;
        let initialization_submission = self.queue.submit(std::iter::empty());
        if self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(initialization_submission),
                timeout: None,
            })
            .is_err()
            || self.device_lost.load(Ordering::Acquire)
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let handle = GpuBrainHandle {
            backend_instance_id: self.backend_instance_id,
            class_id,
            slot,
            generation,
            organism_id,
            phenotype_hash: phenotype.phenotype_hash(),
        };
        let popped = bucket.free_slots.pop();
        debug_assert_eq!(popped, Some(slot));
        bucket.generations[slot as usize] = generation;
        bucket.slots[slot as usize] = Some(ResidentBrainSlot {
            ownership: GpuBrainSlotOwnership {
                organism_id,
                phenotype_hash: phenotype.phenotype_hash(),
                sensor_profile: phenotype.sensor_profile(),
            },
            phenotype: phenotype.clone(),
            brain_slot: upload.brain_slot().clone(),
            ranges: upload.ranges().clone(),
            active_eligibility_generation: 1,
            active_eligibility_bank: 0,
            active_weight_bank: 0,
            active_weight_generation: 1,
            replay_journal_generation: 1,
            transaction_generation: 1,
            logical_dispatch_generation: self.next_dispatch_generation,
            activity_sequence_cursor: 1,
            brain_atp_q16: BRAIN_ATP_Q16_MAX,
            last_world_atp_tick: None,
            last_activity_dispatch_generation: 0,
            last_activity_frame_digest: [0; 4],
            last_completed_gpu_time_ns: 0,
            last_pressure: None,
            last_throttle: None,
            last_work: None,
            sleep_plan: *phenotype.sleep_consolidation_plan(),
            learning_sequence_guard: LearningSequenceGuard::new(
                organism_id,
                phenotype.phenotype_hash(),
            ),
            pending_eligibility: None,
            pending_eligibility_record: None,
        });
        self.slot_generation_watermarks
            .insert((class_raw, slot), generation);
        self.organisms.insert(organism_id.0, handle);
        if self
            .commit_admission_event(event_kind, handle, transient_peak_physical_bytes)
            .is_err()
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(handle)
    }

    pub fn rebind_brain_for_restore(
        &mut self,
        organism_id: OrganismId,
        phenotype: BrainPhenotype,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        self.insert_brain(organism_id, phenotype)
    }

    pub fn remove_brain(&mut self, handle: GpuBrainHandle) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let class_raw = handle.class_id.raw();
        let transient_peak_physical_bytes = self.admission.physical_allocated_bytes;
        let chunk_index = self
            .class_buckets
            .get(&class_raw)
            .and_then(|pool| pool.bucket_index_for_handle(handle))
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        {
            let bucket = self
                .class_buckets
                .get_mut(&class_raw)
                .and_then(|pool| pool.chunks.get_mut(chunk_index))
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = bucket.slots[handle.slot as usize]
                .as_ref()
                .expect("validated occupied slot");
            if resident.pending_eligibility.is_some()
                || resident.pending_eligibility_record.is_some()
            {
                return Err(ScaffoldContractError::LearningReplayRejected);
            }
            let ranges = resident.ranges.clone();
            let mut encoder = self
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("closed-loop-runtime-slot-scrub"),
                });
            bucket
                .buffers
                .record_full_slot_scrub(&mut encoder, &ranges)
                .map_err(map_gpu_contract_error)?;
            let submission = self.queue.submit(Some(encoder.finish()));
            if self
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: None,
                })
                .is_err()
                || self.device_lost.load(Ordering::Acquire)
            {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            if bucket
                .pipelines
                .retire_slot_active_side(handle.slot, handle.generation)
                .is_err()
            {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            bucket.slots[handle.slot as usize] = None;
            if handle.generation == u32::MAX {
                bucket.retired.insert(handle.slot);
            } else {
                bucket.free_slots.push(handle.slot);
            }
        }
        self.organisms.remove(&handle.organism_id.0);
        let drop_empty_chunk = self.runtime_profile.retain_empty_chunks == 0
            && self
                .class_buckets
                .get(&class_raw)
                .and_then(|pool| pool.chunks.get(chunk_index))
                .is_some_and(|bucket| bucket.slots.iter().all(Option::is_none));
        let event_kind = if drop_empty_chunk {
            let pool = self
                .class_buckets
                .get_mut(&class_raw)
                .expect("validated class pool exists");
            pool.chunks.remove(chunk_index);
            if pool.chunks.is_empty() {
                self.class_buckets.remove(&class_raw);
            }
            GpuAllocationEventKind::ReleaseAndDropEmptyChunk
        } else {
            GpuAllocationEventKind::ReleaseToRetainedSlot
        };
        if self
            .commit_admission_event(event_kind, handle, transient_peak_physical_bytes)
            .is_err()
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(())
    }

    pub(crate) fn ensure_ready(&mut self) -> Result<(), ScaffoldContractError> {
        if self.device_lost.load(Ordering::Acquire) {
            self.mark_device_lost();
        }
        if matches!(self.state, GpuBackendState::Ready) {
            Ok(())
        } else {
            Err(ScaffoldContractError::NeuralBackendUnavailable)
        }
    }

    pub(crate) fn validate_handle_backend(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<(), ScaffoldContractError> {
        if handle.backend_instance_id == self.backend_instance_id {
            Ok(())
        } else {
            Err(ScaffoldContractError::BrainOwnershipMismatch)
        }
    }

    pub(crate) fn mark_device_lost(&mut self) {
        self.state = GpuBackendState::DeviceLost {
            last_checkpoint_digest: None,
        };
    }

    fn poison_submitted_dispatches(&mut self, dispatches: &[PreparedClassDispatch]) {
        for dispatch in dispatches {
            if let Some(batch) = dispatch.batch.as_ref() {
                if let Some(bucket) = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                {
                    let _ = bucket.pipelines.mark_post_submit_poison(batch);
                }
            }
        }
        self.mark_device_lost();
    }

    fn cleanup_unsubmitted_dispatches(&mut self, dispatches: &mut [PreparedClassDispatch]) {
        for dispatch in dispatches.iter_mut() {
            if dispatch.recorded {
                if let Some(batch) = dispatch.batch.as_ref() {
                    let _ = self
                        .class_buckets
                        .get_mut(&dispatch.class_id)
                        .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                        .expect("transaction bucket exists")
                        .pipelines
                        .rollback_recorded_batch(batch);
                }
                dispatch.recorded = false;
            }
        }
        for dispatch in dispatches.iter_mut() {
            if let Some(batch) = dispatch.batch.take() {
                let _ = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                    .expect("transaction bucket exists")
                    .pipelines
                    .abandon_unsubmitted_batch(batch);
            }
        }
    }

    #[cfg(feature = "gpu-tests")]
    pub fn shared_resource_counts_for_test(&self) -> (usize, usize, usize) {
        let _ = (&self.adapter, &self.device, &self.queue);
        (1, 1, 1)
    }

    #[cfg(feature = "gpu-tests")]
    pub const fn shared_kernel_set_count_for_test(&self) -> usize {
        1
    }

    #[cfg(feature = "gpu-tests")]
    pub fn allocated_class_arena_count_for_test(&self) -> usize {
        self.class_buckets
            .values()
            .map(|pool| pool.chunks.len())
            .sum()
    }

    #[cfg(feature = "gpu-tests")]
    pub const fn runtime_counters_for_test(&self) -> (u64, u64, u64) {
        (
            self.completed_dispatch_count,
            self.perception_upload_count,
            self.completed_selection_count,
        )
    }

    #[cfg(feature = "gpu-tests")]
    pub fn contains_organism_for_test(&self, organism_id: OrganismId) -> bool {
        self.organisms.contains_key(&organism_id.0)
    }

    #[cfg(feature = "gpu-tests")]
    pub const fn last_compact_readback_bytes_for_test(&self) -> usize {
        self.last_compact_readback_bytes
    }

    #[cfg(feature = "gpu-tests")]
    pub fn read_active_fast_weights_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) -> Result<Vec<f32>, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let bucket = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .bucket_for_handle(handle)?;
        let resident = bucket
            .slots
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let words = if resident.active_weight_bank == 0 {
            resident.ranges.layout.fast_weight_words.clone()
        } else {
            resident.ranges.layout.fast_weight_bank_1_words.clone()
        };
        let range_word_count = words
            .end
            .checked_sub(words.start)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let word_count = resident.brain_slot.record().synapse_count;
        if word_count == 0 || range_word_count < word_count {
            return Err(ScaffoldContractError::GpuLayoutMismatch);
        }
        let size = u64::from(word_count) * 4;
        let readback = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("closed-loop-test-active-fast-readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("closed-loop-test-active-fast-copy"),
            });
        encoder.copy_buffer_to_buffer(
            bucket.buffers.neural_buffers()[6],
            u64::from(words.start) * 4,
            &readback,
            0,
            size,
        );
        let command_buffer = encoder.finish();
        let (sender, receiver) = std::sync::mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &readback,
            wgpu::MapMode::Read,
            0..size,
            move |result| {
                let _ = sender.send(result);
            },
        );
        let submission = self.queue.submit(Some(command_buffer));
        if self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
            || receiver.recv().ok().and_then(Result::ok).is_none()
        {
            readback.unmap();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let mapped = readback.slice(..size).get_mapped_range();
        let values = bytemuck::cast_slice::<u8, f32>(&mapped).to_vec();
        drop(mapped);
        readback.unmap();
        if values.iter().any(|value| !value.is_finite()) {
            return Err(ScaffoldContractError::NonFiniteFloat);
        }
        Ok(values)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_all_invalid_after_next_decode_for_test(&mut self, handle: GpuBrainHandle) {
        if handle.backend_instance_id == self.backend_instance_id {
            if let Some(bucket) = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .and_then(|pool| pool.bucket_for_handle_mut(handle).ok())
            {
                bucket
                    .pipelines
                    .force_all_invalid_record_for_test(handle.slot, handle.generation);
            }
        }
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_pending_identity_mismatch_after_next_decode_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) {
        if handle.backend_instance_id == self.backend_instance_id {
            if let Some(bucket) = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .and_then(|pool| pool.bucket_for_handle_mut(handle).ok())
            {
                bucket
                    .pipelines
                    .force_pending_identity_mismatch_for_test(handle.slot, handle.generation);
            }
        }
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_device_lost_after_next_submit_for_test(&mut self) {
        self.force_device_lost_after_submit = true;
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_activity_sequence_cursor_for_test(
        &mut self,
        handle: GpuBrainHandle,
        cursor: u64,
    ) -> Result<(), ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        let pool = self
            .class_buckets
            .get_mut(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        pool.resident_mut(handle)?.activity_sequence_cursor = cursor;
        Ok(())
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_learning_rejections_for_test(&mut self, rejection_count: u8) {
        self.forced_learning_rejections_remaining = rejection_count;
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_discard_rejections_for_test(&mut self, rejection_count: u8) {
        self.forced_discard_rejections_remaining = rejection_count;
    }
}

#[allow(clippy::infallible_destructuring_match)]
fn acquire_required_gpu(
    factory: &impl GpuDeviceFactory,
) -> Result<RequiredGpuDevice, ScaffoldContractError> {
    let required_features = required_device_features();
    let required_limits = required_device_limits();
    let mut found_base_compatible_without_timestamps = false;
    for candidate in factory.request_adapters()? {
        let adapter = match candidate {
            GpuAdapterCandidate::Hardware(adapter) => adapter,
            #[cfg(test)]
            GpuAdapterCandidate::Software => continue,
        };
        let info = adapter.get_info();
        if info.device_type == wgpu::DeviceType::Cpu
            || info.backend == wgpu::Backend::Noop
            || backend_slug(info.backend).is_err()
            || !required_limits.check_limits(&adapter.limits())
        {
            continue;
        }
        if validate_required_device_features(adapter.features()).is_err() {
            found_base_compatible_without_timestamps = true;
            continue;
        }
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("alife-required-closed-loop-device"),
            required_features,
            required_limits: required_limits.clone(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        };
        let Ok((device, queue)) = factory.request_device(&adapter, &descriptor) else {
            continue;
        };
        let lost = Arc::new(AtomicBool::new(false));
        let callback_lost = Arc::clone(&lost);
        device.set_device_lost_callback(move |_reason, _message| {
            callback_lost.store(true, Ordering::Release);
        });
        let hardware = build_hardware_receipt(
            &info,
            required_features,
            device.features(),
            &device.limits(),
        )?;
        return Ok(RequiredGpuDevice {
            adapter,
            device,
            queue,
            hardware,
            lost,
        });
    }
    Err(if found_base_compatible_without_timestamps {
        ScaffoldContractError::GpuTimestampQueryUnavailable
    } else {
        ScaffoldContractError::NeuralBackendUnavailable
    })
}

fn required_device_features() -> wgpu::Features {
    wgpu::Features::TIMESTAMP_QUERY
}

fn validate_required_device_features(
    available: wgpu::Features,
) -> Result<(), ScaffoldContractError> {
    let required = required_device_features();
    if REQUIRED_GPU_FEATURE_MASK != 1 {
        return Err(ScaffoldContractError::GpuLayoutMismatch);
    }
    if available.contains(required) {
        Ok(())
    } else {
        Err(ScaffoldContractError::GpuTimestampQueryUnavailable)
    }
}

fn required_device_limits() -> wgpu::Limits {
    let mut required = wgpu::Limits::downlevel_defaults();
    required.max_storage_buffers_per_shader_stage =
        required.max_storage_buffers_per_shader_stage.max(10);
    required.max_buffer_size = required.max_buffer_size.max(268_435_456);
    required.max_storage_buffer_binding_size =
        required.max_storage_buffer_binding_size.max(134_217_728);
    required
}

#[cfg(test)]
struct UnavailableGpuFactory;

#[cfg(test)]
impl GpuDeviceFactory for UnavailableGpuFactory {
    fn request_adapters(&self) -> Result<Vec<GpuAdapterCandidate>, ScaffoldContractError> {
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    }

    fn request_device(
        &self,
        _adapter: &wgpu::Adapter,
        _descriptor: &wgpu::DeviceDescriptor<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue), ScaffoldContractError> {
        unreachable!("unavailable adapter must stop before device request")
    }
}

#[cfg(test)]
#[derive(Default)]
struct SoftwareAdapterGpuFactory {
    device_requests: std::cell::Cell<u32>,
}

#[cfg(test)]
impl SoftwareAdapterGpuFactory {
    fn device_request_count(&self) -> u32 {
        self.device_requests.get()
    }
}

#[cfg(test)]
impl GpuDeviceFactory for SoftwareAdapterGpuFactory {
    fn request_adapters(&self) -> Result<Vec<GpuAdapterCandidate>, ScaffoldContractError> {
        Ok(vec![GpuAdapterCandidate::Software])
    }

    fn request_device(
        &self,
        _adapter: &wgpu::Adapter,
        _descriptor: &wgpu::DeviceDescriptor<'_>,
    ) -> Result<(wgpu::Device, wgpu::Queue), ScaffoldContractError> {
        self.device_requests.set(self.device_requests.get() + 1);
        Err(ScaffoldContractError::NeuralBackendUnavailable)
    }
}

#[cfg(test)]
#[derive(Clone)]
struct RuntimeArenaFixtureSlot {
    generation: u32,
    owner: Option<(OrganismId, PhenotypeHash)>,
    retired: bool,
    ranges: Vec<Vec<u32>>,
}

#[cfg(test)]
struct RuntimeArenaTestHarness {
    backend_instance_id: NonZeroU64,
    class_id: BrainClassId,
    slots: Vec<RuntimeArenaFixtureSlot>,
    state: GpuBackendState,
    fail_next_scrub: bool,
}

#[cfg(test)]
impl RuntimeArenaTestHarness {
    fn n512(slot_count: usize) -> Self {
        Self {
            backend_instance_id: NonZeroU64::new(1).unwrap(),
            class_id: BrainClassId(1),
            slots: vec![
                RuntimeArenaFixtureSlot {
                    generation: 0,
                    owner: None,
                    retired: false,
                    ranges: (0..9).map(|_| vec![0; 8]).collect(),
                };
                slot_count
            ],
            state: GpuBackendState::Ready,
            fail_next_scrub: false,
        }
    }

    fn insert_fixture(
        &mut self,
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        let index = self
            .slots
            .iter()
            .position(|slot| slot.owner.is_none() && !slot.retired)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next = self.slots[index]
            .generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        self.insert_at(index, organism_id, phenotype_hash, next)
    }

    fn insert_fixture_with_generation(
        &mut self,
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
        generation: u32,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        let index = self
            .slots
            .iter()
            .position(|slot| slot.owner.is_none() && !slot.retired)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        self.insert_at(index, organism_id, phenotype_hash, generation)
    }

    fn rebind_fixture_for_restore(
        &mut self,
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        if self
            .slots
            .iter()
            .any(|slot| slot.owner.is_some_and(|owner| owner.0 == organism_id))
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        self.insert_fixture(organism_id, phenotype_hash)
    }

    fn insert_at(
        &mut self,
        index: usize,
        organism_id: OrganismId,
        phenotype_hash: PhenotypeHash,
        generation: u32,
    ) -> Result<GpuBrainHandle, ScaffoldContractError> {
        if !matches!(self.state, GpuBackendState::Ready) || generation == 0 {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let slot = &mut self.slots[index];
        if slot.owner.is_some() || slot.retired {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        slot.generation = generation;
        slot.owner = Some((organism_id, phenotype_hash));
        Ok(GpuBrainHandle {
            backend_instance_id: self.backend_instance_id,
            class_id: self.class_id,
            slot: index as u32,
            generation,
            organism_id,
            phenotype_hash,
        })
    }

    fn fill_every_reserved_range(&mut self, handle: GpuBrainHandle, value: u32) {
        assert!(self.owns(handle));
        for range in &mut self.slots[handle.slot as usize].ranges {
            range.fill(value);
        }
    }

    fn remove_fixture(&mut self, handle: GpuBrainHandle) -> Result<(), ScaffoldContractError> {
        if !matches!(self.state, GpuBackendState::Ready) || !self.owns(handle) {
            return Err(if matches!(self.state, GpuBackendState::Ready) {
                ScaffoldContractError::BrainOwnershipMismatch
            } else {
                ScaffoldContractError::NeuralBackendUnavailable
            });
        }
        if self.fail_next_scrub {
            self.fail_next_scrub = false;
            self.state = GpuBackendState::DeviceLost {
                last_checkpoint_digest: None,
            };
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let slot = &mut self.slots[handle.slot as usize];
        for range in &mut slot.ranges {
            range.fill(0);
        }
        slot.owner = None;
        if slot.generation == u32::MAX {
            slot.retired = true;
        }
        Ok(())
    }

    fn every_reserved_range_is_zero(&self, slot: u32) -> bool {
        self.slots[slot as usize]
            .ranges
            .iter()
            .flatten()
            .all(|word| *word == 0)
    }

    fn slot_is_permanently_retired(&self, slot: u32) -> bool {
        self.slots[slot as usize].retired
    }

    fn free_slot_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| slot.owner.is_none() && !slot.retired)
            .count()
    }

    fn fail_next_scrub_after_submit(&mut self) {
        self.fail_next_scrub = true;
    }

    fn state(&self) -> &GpuBackendState {
        &self.state
    }

    fn owns(&self, handle: GpuBrainHandle) -> bool {
        handle.backend_instance_id == self.backend_instance_id
            && handle.class_id == self.class_id
            && self.slots.get(handle.slot as usize).is_some_and(|slot| {
                slot.generation == handle.generation
                    && slot.owner == Some((handle.organism_id, handle.phenotype_hash))
            })
    }

    fn validate_frame_organism(
        &self,
        handle: GpuBrainHandle,
        organism_id: OrganismId,
    ) -> Result<(), ScaffoldContractError> {
        if self.owns(handle) && handle.organism_id == organism_id {
            Ok(())
        } else {
            Err(ScaffoldContractError::BrainOwnershipMismatch)
        }
    }
}

#[cfg(test)]
#[derive(Default)]
struct RuntimePreflightTestHarness {
    allocated_arenas: usize,
    counters: (u64, u64, u64),
}

#[cfg(test)]
impl RuntimePreflightTestHarness {
    fn validate_class(&mut self, class_id: BrainClassId) -> Result<(), ScaffoldContractError> {
        match class_id.raw() {
            1..=3 => Ok(()),
            _ => Err(ScaffoldContractError::UnsupportedProductionBrainClass),
        }
    }

    fn allocated_arena_count(&self) -> usize {
        self.allocated_arenas
    }

    fn runtime_counters(&self) -> (u64, u64, u64) {
        self.counters
    }

    fn perception_upload_count(&self) -> u64 {
        self.counters.1
    }

    fn validate_frame_digest(
        &mut self,
        expected: PerceptionFrameDigest,
        actual: PerceptionFrameDigest,
    ) -> Result<(), ScaffoldContractError> {
        if expected == actual {
            Ok(())
        } else {
            Err(ScaffoldContractError::InvalidPerceptionFrame)
        }
    }
}

#[cfg(test)]
#[path = "../tests/support/closed_loop_runtime_private.rs"]
mod task7_private_tests;
