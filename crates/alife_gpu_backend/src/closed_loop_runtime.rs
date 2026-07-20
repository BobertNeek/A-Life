//! Shared required-GPU ownership and evidence contracts for the closed loop.
//!
//! The world supplies current perception and unscored candidates. This module
//! owns the one authoritative device, fixed class arenas, generation-checked
//! capabilities, bounded selection readback, and fail-stop transaction state.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use alife_core::{
    BrainCapacityClass, BrainClassId, BrainPhenotype, CanonicalDigestBuilder, Confidence,
    ExperiencePatch, FinalizedMemoryRecall, LearningCommitToken, LearningSequenceGuard,
    NeuralActionSelection, OrganismId, OutcomeCreditPacket, PerceptionBaseDigest, PerceptionFrame,
    PerceptionFrameDigest, PhenotypeHash, ScaffoldContractError, SensorProfile,
};
use serde::{Deserialize, Serialize};

use crate::{
    GpuActiveBatchUpload, GpuBrainSlot, GpuClosedLoopError, GpuClosedLoopKernelSet,
    GpuClosedLoopPipelines, GpuCompactMapTicket, GpuFastPlasticityBatchEntry,
    GpuFixedActiveBatchEntry, GpuFixedClassArenaBuffers, GpuFixedClassArenaPlan,
    GpuFixedSlotRanges, GpuLearningReceipt, GpuMemoryContextDispatchReceipt,
    GpuMemoryContextUpload, GpuOutcomeCreditRecord, GpuPendingEligibilityRecord,
    GpuPerceptionUpload, GpuPreparedActiveBatch, GpuValidatedClassBatch,
    PendingEligibilityDiscardReceipt, PendingEligibilityIdentity, PendingEligibilityReceipt,
    GPU_CLOSED_LOOP_LAYOUT_VERSION,
};

pub const GPU_HARDWARE_RECEIPT_SCHEMA_VERSION: u16 = 1;
pub const GPU_DRIVER_DIGEST_DOMAIN: &[u8] = b"alife.gpu.hardware.driver.v1";
pub const GPU_FEATURE_DIGEST_DOMAIN: &[u8] = b"alife.gpu.hardware.features.v1";
pub const GPU_LIMITS_DIGEST_DOMAIN: &[u8] = b"alife.gpu.hardware.limits.v1";

const BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");
const DEFAULT_RESIDENT_CEILING_BYTES: u64 = 128 * 1024 * 1024;

static NEXT_BACKEND_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_HARDWARE_RECEIPT_GENERATION: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuClosedLoopRuntimeConfig {
    pub n512_slots: u32,
    pub n1024_slots: u32,
    pub n2048_slots: u32,
    pub aggregate_resident_ceiling_bytes: u64,
}

impl Default for GpuClosedLoopRuntimeConfig {
    fn default() -> Self {
        Self {
            n512_slots: 64,
            n1024_slots: 16,
            n2048_slots: 4,
            aggregate_resident_ceiling_bytes: DEFAULT_RESIDENT_CEILING_BYTES,
        }
    }
}

impl GpuClosedLoopRuntimeConfig {
    fn validate(self) -> Result<Self, ScaffoldContractError> {
        if self.n512_slots == 0
            || self.n1024_slots == 0
            || self.n2048_slots == 0
            || self.aggregate_resident_ceiling_bytes == 0
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        Ok(self)
    }

    fn slots_for_class(self, class_id: BrainClassId) -> Result<u32, ScaffoldContractError> {
        match class_id.raw() {
            1 => Ok(self.n512_slots),
            2 => Ok(self.n1024_slots),
            3 => Ok(self.n2048_slots),
            _ => Err(ScaffoldContractError::UnsupportedProductionBrainClass),
        }
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
    pub compact_readback_bytes: usize,
    pub hardware_receipt_generation: u64,
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
    pub(crate) brain_slot: GpuBrainSlot,
    pub(crate) ranges: GpuFixedSlotRanges,
    pub(crate) active_eligibility_bank: u8,
    pub(crate) active_eligibility_generation: u64,
    pub(crate) active_weight_bank: u8,
    pub(crate) active_weight_generation: u64,
    pub(crate) replay_journal_generation: u64,
    pub(crate) transaction_generation: u64,
    pub(crate) logical_dispatch_generation: u64,
    pub(crate) sleep_plan: alife_core::SleepConsolidationPlan,
    pub(crate) learning_sequence_guard: LearningSequenceGuard,
    pub(crate) pending_eligibility: Option<PendingEligibilityReceipt>,
    pub(crate) pending_eligibility_record: Option<GpuPendingEligibilityRecord>,
}

struct PreparedLearningApply {
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

    fn next_free_slot(&self) -> Result<(u32, u32), ScaffoldContractError> {
        let slot = *self
            .free_slots
            .last()
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let generation = self.generations[slot as usize]
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        Ok((slot, generation))
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

struct PreparedClassDispatch {
    class_id: u16,
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
    match class_id.raw() {
        1 => Ok(BrainCapacityClass::n512()),
        2 => Ok(BrainCapacityClass::n1024()),
        3 => Ok(BrainCapacityClass::n2048()),
        _ => Err(ScaffoldContractError::UnsupportedProductionBrainClass),
    }
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
    pub(crate) device_lost: Arc<AtomicBool>,
    kernels: Arc<GpuClosedLoopKernelSet>,
    pub(crate) state: GpuBackendState,
    config: GpuClosedLoopRuntimeConfig,
    pub(crate) class_buckets: BTreeMap<u16, ClassBucketRuntime>,
    organisms: BTreeMap<u64, GpuBrainHandle>,
    resident_bytes: u64,
    pub(crate) next_dispatch_generation: u64,
    force_device_lost_after_submit: bool,
    #[cfg(feature = "gpu-tests")]
    forced_learning_rejections_remaining: u8,
    #[cfg(feature = "gpu-tests")]
    forced_discard_rejections_remaining: u8,
    completed_dispatch_count: u64,
    perception_upload_count: u64,
    completed_selection_count: u64,
    last_compact_readback_bytes: usize,
    pub(crate) next_sleep_job_id: u64,
    pub(crate) sleep_jobs: BTreeMap<u64, crate::GpuSleepJobState>,
    pub(crate) committed_sleep: BTreeMap<(u16, u32, u32, u64), crate::GpuSleepConsolidationReceipt>,
}

impl GpuClosedLoopBackend {
    pub fn new_required() -> Result<Self, ScaffoldContractError> {
        Self::new_required_with_config(GpuClosedLoopRuntimeConfig::default())
    }

    pub fn new_required_with_config(
        config: GpuClosedLoopRuntimeConfig,
    ) -> Result<Self, ScaffoldContractError> {
        Self::new_with_factory_and_config(&WgpuDeviceFactory, config)
    }

    #[cfg(test)]
    fn new_with_factory(factory: &impl GpuDeviceFactory) -> Result<Self, ScaffoldContractError> {
        Self::new_with_factory_and_config(factory, GpuClosedLoopRuntimeConfig::default())
    }

    fn new_with_factory_and_config(
        factory: &impl GpuDeviceFactory,
        config: GpuClosedLoopRuntimeConfig,
    ) -> Result<Self, ScaffoldContractError> {
        let config = config.validate()?;
        let required = acquire_required_gpu(factory)?;
        let backend_instance_id = next_backend_instance_id()
            .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?;
        let kernels =
            GpuClosedLoopKernelSet::new(&required.device).map_err(map_gpu_contract_error)?;
        Ok(Self {
            backend_instance_id,
            hardware: required.hardware,
            adapter: required.adapter,
            device: required.device,
            queue: required.queue,
            device_lost: required.lost,
            kernels,
            state: GpuBackendState::Ready,
            config,
            class_buckets: BTreeMap::new(),
            organisms: BTreeMap::new(),
            resident_bytes: 0,
            next_dispatch_generation: 1,
            force_device_lost_after_submit: false,
            #[cfg(feature = "gpu-tests")]
            forced_learning_rejections_remaining: 0,
            #[cfg(feature = "gpu-tests")]
            forced_discard_rejections_remaining: 0,
            completed_dispatch_count: 0,
            perception_upload_count: 0,
            completed_selection_count: 0,
            last_compact_readback_bytes: 0,
            next_sleep_job_id: 1,
            sleep_jobs: BTreeMap::new(),
            committed_sleep: BTreeMap::new(),
        })
    }

    pub const fn hardware_receipt(&self) -> &GpuHardwareReceipt {
        &self.hardware
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

    pub fn pending_eligibility(
        &self,
        handle: GpuBrainHandle,
    ) -> Result<Option<PendingEligibilityReceipt>, ScaffoldContractError> {
        if self.device_lost.load(Ordering::Acquire) || !matches!(self.state, GpuBackendState::Ready)
        {
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        self.validate_handle_backend(handle)?;
        let bucket = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = bucket
            .slots
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if !bucket.contains(handle) {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
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

    /// Apply a same-class batch in one GPU transaction. Every row is bound to
    /// the slot's durable pending eligibility and a core-owned sequence token
    /// before any command is submitted.
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
            let bucket = self
                .class_buckets
                .get(&class_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = bucket
                .slots
                .get(handle.slot as usize)
                .and_then(Option::as_ref)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if !bucket.contains(*handle) {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
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
        let gpu_entries = prepared
            .iter()
            .map(|entry| GpuFastPlasticityBatchEntry {
                slot: &entry.brain_slot,
                pending: &entry.pending_record,
                outcome: entry.outcome,
                active_weight_generation: entry.active_weight_generation,
                replay_generation: entry.replay_journal_generation,
                transaction_generation: entry.transaction_generation,
            })
            .collect::<Vec<_>>();
        let gpu_result = {
            let bucket = self
                .class_buckets
                .get_mut(&class_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            bucket.pipelines.apply_fast_plasticity(
                &self.device,
                &self.queue,
                &bucket.buffers,
                &gpu_entries,
            )
        };
        let gpu_records = match gpu_result {
            Ok(records) => records,
            Err(GpuClosedLoopError::MalformedUpload | GpuClosedLoopError::StaleOrForeignHandle) => {
                return Err(ScaffoldContractError::LearningEvidenceMismatch);
            }
            Err(_) => {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        if gpu_records.len() != prepared.len() {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        let host_precommit_valid = prepared.iter().zip(&gpu_records).all(|(entry, record)| {
            self.class_buckets
                .get(&class_id)
                .and_then(|bucket| bucket.slots.get(entry.handle.slot as usize))
                .and_then(Option::as_ref)
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
                .and_then(|bucket| bucket.slots.get_mut(entry.handle.slot as usize))
                .and_then(Option::as_mut)
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
                .and_then(|bucket| bucket.slots.get_mut(entry.handle.slot as usize))
                .and_then(Option::as_mut)
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
            let bucket = self
                .class_buckets
                .get(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = bucket
                .slots
                .get(handle.slot as usize)
                .and_then(Option::as_ref)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if !bucket.contains(handle) {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
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
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
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
            .and_then(|bucket| bucket.slots.get_mut(handle.slot as usize))
            .and_then(Option::as_mut)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        resident.transaction_generation = next_transaction_generation;
        resident.pending_eligibility = None;
        resident.pending_eligibility_record = None;
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
        let bucket = self
            .class_buckets
            .get(&handle.class_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = bucket
            .slots
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if !bucket.contains(handle)
            || resident.ownership.organism_id != frame.organism_id()
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
        let mut grouped = BTreeMap::<u16, Vec<usize>>::new();
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
            let bucket = self
                .class_buckets
                .get(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = bucket
                .slots
                .get(handle.slot as usize)
                .and_then(Option::as_ref)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if !bucket.contains(handle)
                || resident.ownership.organism_id != frame.organism_id()
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
            grouped
                .entry(handle.class_id.raw())
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
        let mut dispatches = Vec::with_capacity(grouped.len());
        for (class_id, original_indices) in grouped {
            let bucket = self
                .class_buckets
                .get(&class_id)
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
                            memory_upload,
                            resident.active_eligibility_generation,
                        ),
                        None => GpuFixedActiveBatchEntry::new(
                            input.frame,
                            &resident.brain_slot,
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
        for index in 0..dispatches.len() {
            let dispatch = &mut dispatches[index];
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
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
        let command_buffer = encoder.finish();
        for index in 0..dispatches.len() {
            let dispatch = &mut dispatches[index];
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
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
        if forced_loss
            || poll_failed
            || !mappings_succeeded
            || self.device_lost.load(Ordering::Acquire)
        {
            for dispatch in &dispatches {
                let bucket = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .expect("submitted bucket exists");
                bucket.buffers.compact_readback().unmap();
                let _ = bucket
                    .pipelines
                    .mark_post_submit_poison(dispatch.batch.as_ref().expect("submitted batch"));
            }
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        for index in 0..dispatches.len() {
            let dispatch = &dispatches[index];
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
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
                            .expect("submitted bucket exists")
                            .buffers
                            .compact_readback()
                            .unmap();
                    }
                    for submitted in &dispatches {
                        let bucket = self
                            .class_buckets
                            .get_mut(&submitted.class_id)
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
                        .and_then(|bucket| bucket.slots.get(handle.slot as usize))
                        .and_then(Option::as_ref)
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
                    .and_then(|bucket| bucket.slots.get(handle.slot as usize))
                    .and_then(Option::as_ref)
                    .is_some_and(|resident| {
                        resident.pending_eligibility.is_none()
                            && resident.pending_eligibility_record.is_none()
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
                .and_then(|bucket| bucket.slots.get_mut(handle.slot as usize))
                .and_then(Option::as_mut)
                .expect("host pending commit was prevalidated");
            resident.transaction_generation = ordered_next_transaction_generations[index]
                .expect("host transaction generation was prevalidated");
            resident.logical_dispatch_generation = dispatch_generation.get();
            resident.pending_eligibility = ordered_pending_receipts[index];
            resident.pending_eligibility_record = ordered_pending_records[index];
        }

        self.completed_dispatch_count = next_completed_dispatch_count;
        self.last_compact_readback_bytes = total_readback_bytes;
        self.next_dispatch_generation = next_dispatch_generation;
        self.completed_selection_count = next_completed_selection_count;
        Ok(prepared_ticks)
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
        let class_raw = class_id.raw();
        let (slot, generation, upload, new_bucket) = if self.class_buckets.contains_key(&class_raw)
        {
            let bucket = self
                .class_buckets
                .get(&class_raw)
                .expect("existing class key resolves");
            let (slot, generation) = bucket.next_free_slot()?;
            let upload = bucket
                .plan
                .prepare_slot_upload(slot, generation, &phenotype)
                .map_err(map_gpu_contract_error)?;
            (slot, generation, upload, None)
        } else {
            let slot_capacity = self.config.slots_for_class(class_id)?;
            let remaining = self
                .config
                .aggregate_resident_ceiling_bytes
                .checked_sub(self.resident_bytes)
                .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
            let plan = GpuFixedClassArenaPlan::new(capacity, slot_capacity, remaining)
                .map_err(map_gpu_contract_error)?;
            let next_resident_bytes = self
                .resident_bytes
                .checked_add(plan.aggregate_resident_bytes())
                .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
            if next_resident_bytes > self.config.aggregate_resident_ceiling_bytes {
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            let slot = 0;
            let generation = 1;
            let upload = plan
                .prepare_slot_upload(slot, generation, &phenotype)
                .map_err(map_gpu_contract_error)?;
            let bucket =
                ClassBucketRuntime::from_plan(&self.device, Arc::clone(&self.kernels), plan)
                    .map_err(map_gpu_contract_error)?;
            debug_assert_eq!(bucket.next_free_slot()?, (slot, generation));
            (
                slot,
                generation,
                upload,
                Some((bucket, next_resident_bytes)),
            )
        };
        if let Some((bucket, next_resident_bytes)) = new_bucket {
            self.class_buckets.insert(class_raw, bucket);
            self.resident_bytes = next_resident_bytes;
        }
        let bucket = self
            .class_buckets
            .get_mut(&class_raw)
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
            brain_slot: upload.brain_slot().clone(),
            ranges: upload.ranges().clone(),
            active_eligibility_generation: 1,
            active_eligibility_bank: 0,
            active_weight_bank: 0,
            active_weight_generation: 1,
            replay_journal_generation: 1,
            transaction_generation: 1,
            logical_dispatch_generation: self.next_dispatch_generation,
            sleep_plan: *phenotype.sleep_consolidation_plan(),
            learning_sequence_guard: LearningSequenceGuard::new(
                organism_id,
                phenotype.phenotype_hash(),
            ),
            pending_eligibility: None,
            pending_eligibility_record: None,
        });
        self.organisms.insert(organism_id.0, handle);
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
        let bucket = self
            .class_buckets
            .get_mut(&class_raw)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if !bucket.contains(handle) {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let resident = bucket.slots[handle.slot as usize]
            .as_ref()
            .expect("validated occupied slot");
        if resident.pending_eligibility.is_some() || resident.pending_eligibility_record.is_some() {
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
        self.organisms.remove(&handle.organism_id.0);
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
                if let Some(bucket) = self.class_buckets.get_mut(&dispatch.class_id) {
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
        self.class_buckets.len()
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
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = bucket
            .slots
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if !bucket.contains(handle) {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
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
            if let Some(bucket) = self.class_buckets.get_mut(&handle.class_id.raw()) {
                if bucket.contains(handle) {
                    bucket
                        .pipelines
                        .force_all_invalid_record_for_test(handle.slot, handle.generation);
                }
            }
        }
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_pending_identity_mismatch_after_next_decode_for_test(
        &mut self,
        handle: GpuBrainHandle,
    ) {
        if handle.backend_instance_id == self.backend_instance_id {
            if let Some(bucket) = self.class_buckets.get_mut(&handle.class_id.raw()) {
                if bucket.contains(handle) {
                    bucket
                        .pipelines
                        .force_pending_identity_mismatch_for_test(handle.slot, handle.generation);
                }
            }
        }
    }

    #[cfg(feature = "gpu-tests")]
    pub fn force_device_lost_after_next_submit_for_test(&mut self) {
        self.force_device_lost_after_submit = true;
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
    let required_features = wgpu::Features::empty();
    let required_limits = required_device_limits();
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
            || !adapter.features().contains(required_features)
            || !required_limits.check_limits(&adapter.limits())
        {
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
    Err(ScaffoldContractError::NeuralBackendUnavailable)
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
