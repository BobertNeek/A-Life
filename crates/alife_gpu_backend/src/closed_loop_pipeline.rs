//! Production GPU-authoritative perception encoding and recurrent dispatch.
//!
//! Task 6 adds candidate decode/selection. This module deliberately exposes no
//! CPU neural execution and obtains neural results only from WGSL state.

use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU64,
    sync::{mpsc, Arc},
};

use alife_core::{PerceptionFrame, MAX_ACTION_CANDIDATES};

use crate::{
    GpuBrainSlot, GpuCandidateRecord, GpuClassBucketBuffers, GpuClosedLoopError,
    GpuFixedClassArenaBuffers, GpuPerceptionHeader, GpuPerceptionUpload, GpuSelectionRecord,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchLifecycleStage {
    Built,
    EncodeRecorded,
    RecurrentRecorded,
    SelectionRecorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingBatchAuthority {
    nonce: u64,
    stage: BatchLifecycleStage,
}

#[derive(Debug, Default)]
struct BatchAuthority {
    active_sides: BTreeMap<(u32, u32), u32>,
    pending: Option<PendingBatchAuthority>,
    poisoned_nonce: Option<u64>,
}

impl BatchAuthority {
    fn begin(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        self.ensure_healthy()?;
        if self.pending.is_some() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.pending = Some(PendingBatchAuthority {
            nonce,
            stage: BatchLifecycleStage::Built,
        });
        Ok(())
    }

    fn record_encode(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::Built {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        pending.stage = BatchLifecycleStage::EncodeRecorded;
        Ok(())
    }

    fn record_recurrent(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::EncodeRecorded {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        pending.stage = BatchLifecycleStage::RecurrentRecorded;
        Ok(())
    }

    fn recording_failed(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        pending.stage = BatchLifecycleStage::Built;
        Ok(())
    }

    fn submission_indeterminate(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if !matches!(
            pending.stage,
            BatchLifecycleStage::RecurrentRecorded | BatchLifecycleStage::SelectionRecorded
        ) {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.poisoned_nonce = Some(nonce);
        Ok(())
    }

    fn abandon_unsubmitted(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::Built {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.pending = None;
        Ok(())
    }

    fn submission_succeeded(
        &mut self,
        nonce: u64,
        final_sides: &[(u32, u32, u32)],
    ) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::SelectionRecorded {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        for &(slot, generation, side) in final_sides {
            self.active_sides.insert((slot, generation), side);
        }
        self.pending = None;
        Ok(())
    }

    fn prevalidate_submission_succeeded(&self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        self.require_stage(nonce, BatchLifecycleStage::SelectionRecorded)
    }

    fn record_selection(&mut self, nonce: u64) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::RecurrentRecorded {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        pending.stage = BatchLifecycleStage::SelectionRecorded;
        Ok(())
    }

    #[cfg(feature = "gpu-tests")]
    fn recurrent_diagnostic_succeeded(
        &mut self,
        nonce: u64,
        final_sides: &[(u32, u32, u32)],
    ) -> Result<(), GpuClosedLoopError> {
        let pending = self.pending_mut(nonce)?;
        if pending.stage != BatchLifecycleStage::RecurrentRecorded {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        for &(slot, generation, side) in final_sides {
            self.active_sides.insert((slot, generation), side);
        }
        self.pending = None;
        Ok(())
    }

    fn pending_mut(
        &mut self,
        nonce: u64,
    ) -> Result<&mut PendingBatchAuthority, GpuClosedLoopError> {
        self.ensure_healthy()?;
        self.pending
            .as_mut()
            .filter(|pending| pending.nonce == nonce)
            .ok_or(GpuClosedLoopError::StaleOrForeignHandle)
    }

    fn require_stage(
        &self,
        nonce: u64,
        stage: BatchLifecycleStage,
    ) -> Result<(), GpuClosedLoopError> {
        self.ensure_healthy()?;
        if self.pending == Some(PendingBatchAuthority { nonce, stage }) {
            Ok(())
        } else {
            Err(GpuClosedLoopError::MalformedUpload)
        }
    }

    fn ensure_healthy(&self) -> Result<(), GpuClosedLoopError> {
        if self.poisoned_nonce.is_some() {
            Err(GpuClosedLoopError::SubmissionFailed)
        } else {
            Ok(())
        }
    }

    fn retire_active_side(&mut self, slot: u32, generation: u32) -> Result<(), GpuClosedLoopError> {
        self.ensure_healthy()?;
        if self.pending.is_some() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.active_sides.remove(&(slot, generation));
        Ok(())
    }
}

pub const GPU_ACTIVE_DISPATCH_ROW_WORDS: usize = 272;
pub const GPU_ACTIVE_SIDE_DIAGNOSTIC_LANE: u32 = 3;
const GPU_PERCEPTION_HEADER_WORDS: usize = 16;
const GPU_CANDIDATE_RECORD_WORDS: usize = 8;
const WORKGROUP_SIZE: u32 = 64;
const GPU_REQUIRED_MAX_BUFFER_WORDS: usize = 268_435_456 / 4;

pub const CLOSED_LOOP_ENCODE_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_encode.wgsl")
);
pub const CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_clear_diagnostics.wgsl")
);
pub const CLOSED_LOOP_RECURRENT_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_recurrent.wgsl")
);
pub const CLOSED_LOOP_DECODE_WGSL: &str = concat!(
    include_str!("../shaders/closed_loop_abi.wgsl"),
    include_str!("../shaders/closed_loop_decode.wgsl")
);

#[derive(Debug, Clone, Copy)]
pub struct GpuActiveBatchEntry<'a> {
    frame: &'a PerceptionFrame,
    slot: &'a GpuBrainSlot,
}

impl<'a> GpuActiveBatchEntry<'a> {
    pub const fn new(frame: &'a PerceptionFrame, slot: &'a GpuBrainSlot) -> Self {
        Self { frame, slot }
    }
}

/// Borrowed fixed-arena row used by the live runtime. It carries no packed
/// append-plan state and binds the physical arena index explicitly.
pub(crate) struct GpuFixedActiveBatchEntry<'a> {
    frame: &'a PerceptionFrame,
    slot: &'a GpuBrainSlot,
}

impl<'a> GpuFixedActiveBatchEntry<'a> {
    pub(crate) const fn new(frame: &'a PerceptionFrame, slot: &'a GpuBrainSlot) -> Self {
        Self { frame, slot }
    }
}

#[derive(Clone, Copy)]
struct GpuBatchEntryView<'a> {
    frame: &'a PerceptionFrame,
    slot: &'a GpuBrainSlot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuActiveBatchUpload {
    headers: Vec<GpuPerceptionHeader>,
    dispatch_header_words: Vec<u32>,
    frame_payload_words: Vec<u32>,
    bucket_ownership_token: u64,
    authority_nonce: u64,
    selection_offsets: Vec<u32>,
}

impl GpuActiveBatchUpload {
    #[allow(clippy::too_many_arguments)]
    fn try_from_views(
        entries: &[GpuBatchEntryView<'_>],
        frame_base_words: u32,
        bucket_ownership_token: u64,
        active_sides: &BTreeMap<(u32, u32), u32>,
        dispatch_capacity_words: usize,
        frame_payload_capacity_words: usize,
        dispatch_generation: NonZeroU64,
        authority_nonce: u64,
    ) -> Result<Self, GpuClosedLoopError> {
        if entries.is_empty() {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let dispatch_words = entries
            .len()
            .checked_mul(GPU_ACTIVE_DISPATCH_ROW_WORDS)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if dispatch_words > GPU_REQUIRED_MAX_BUFFER_WORDS
            || dispatch_words > dispatch_capacity_words
            || frame_base_words as usize > GPU_REQUIRED_MAX_BUFFER_WORDS
            || frame_base_words as usize > frame_payload_capacity_words
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let mut dispatch_header_words = vec![0_u32; dispatch_words];
        let mut frame_payload_words = vec![0_u32; frame_base_words as usize];
        let mut headers = Vec::with_capacity(entries.len());
        let mut seen_slots = BTreeSet::new();
        let class_id = entries[0].slot.record().class_id;

        for (row, entry) in entries.iter().enumerate() {
            if entry.slot.record().class_id != class_id
                || entry.slot.record().slot_generation == 0
                || !seen_slots.insert(entry.slot.brain_slot_index())
            {
                return Err(GpuClosedLoopError::StaleOrForeignHandle);
            }
            if entry.frame.candidates().len() > MAX_ACTION_CANDIDATES {
                return Err(GpuClosedLoopError::CapacityExceeded);
            }
            let mut upload = GpuPerceptionUpload::try_from_frame(
                entry.frame,
                entry.slot,
                active_sides
                    .get(&(
                        entry.slot.brain_slot_index(),
                        entry.slot.record().slot_generation,
                    ))
                    .copied()
                    .unwrap_or(0),
            )?;
            let row_base = row
                .checked_mul(GPU_ACTIVE_DISPATCH_ROW_WORDS)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let row_base_u32 =
                u32::try_from(row_base).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
            let payload_base = u32::try_from(frame_payload_words.len())
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
            upload.rebase(row_base_u32, payload_base)?;
            upload.validate_against(entry.frame, entry.slot)?;

            let candidate_words = upload
                .candidates
                .len()
                .checked_mul(GPU_CANDIDATE_RECORD_WORDS)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            let used_words = GPU_PERCEPTION_HEADER_WORDS
                .checked_add(candidate_words)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            if used_words > GPU_ACTIVE_DISPATCH_ROW_WORDS {
                return Err(GpuClosedLoopError::CapacityExceeded);
            }
            dispatch_header_words[row_base..row_base + GPU_PERCEPTION_HEADER_WORDS]
                .copy_from_slice(upload.header.words());
            for (candidate_index, candidate) in upload.candidates.iter().enumerate() {
                let start = row_base
                    + GPU_PERCEPTION_HEADER_WORDS
                    + candidate_index * GPU_CANDIDATE_RECORD_WORDS;
                dispatch_header_words[start..start + GPU_CANDIDATE_RECORD_WORDS]
                    .copy_from_slice(candidate.words());
            }
            let payload_end = frame_payload_words
                .len()
                .checked_add(upload.frame_payload_words.len())
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
            if payload_end > GPU_REQUIRED_MAX_BUFFER_WORDS
                || payload_end > frame_payload_capacity_words
            {
                return Err(GpuClosedLoopError::CapacityExceeded);
            }
            frame_payload_words.extend_from_slice(&upload.frame_payload_words);
            upload.header.dispatch_generation_lo = dispatch_generation.get() as u32;
            upload.header.dispatch_generation_hi = (dispatch_generation.get() >> 32) as u32;
            dispatch_header_words[row_base..row_base + GPU_PERCEPTION_HEADER_WORDS]
                .copy_from_slice(upload.header.words());
            headers.push(upload.header);
        }

        Ok(Self {
            headers,
            dispatch_header_words,
            frame_payload_words,
            bucket_ownership_token,
            authority_nonce,
            selection_offsets: entries
                .iter()
                .map(|entry| entry.slot.record().selection_offset)
                .collect(),
        })
    }

    pub fn row_count(&self) -> usize {
        self.headers.len()
    }
    pub fn headers(&self) -> &[GpuPerceptionHeader] {
        &self.headers
    }
    pub fn dispatch_header_words(&self) -> &[u32] {
        &self.dispatch_header_words
    }
    pub fn frame_payload_words(&self) -> &[u32] {
        &self.frame_payload_words
    }
    #[allow(dead_code)]
    pub(crate) fn dispatch_generation(&self) -> u64 {
        self.headers.first().map_or(0, |header| {
            u64::from(header.dispatch_generation_lo)
                | (u64::from(header.dispatch_generation_hi) << 32)
        })
    }
    #[cfg(test)]
    fn authority_nonce_for_test(&self) -> u64 {
        self.authority_nonce
    }
    #[cfg(any(test, feature = "gpu-tests"))]
    pub fn zero_frame_payload_for_hardware_diagnostic(&mut self) {
        self.frame_payload_words.fill(0);
    }
}

pub(crate) struct GpuPreparedActiveBatch {
    batch: GpuActiveBatchUpload,
}

pub(crate) struct GpuCompactMapTicket {
    receiver: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
}

impl GpuCompactMapTicket {
    pub(crate) fn mapping_succeeded(self) -> bool {
        self.receiver.recv().ok().and_then(Result::ok).is_some()
    }
}

pub(crate) struct GpuValidatedClassBatch {
    bucket_ownership_token: u64,
    authority_nonce: u64,
    records: Vec<GpuSelectionRecord>,
    final_sides: Vec<(u32, u32, u32)>,
    readback_bytes: u64,
}

pub(crate) trait ClosedLoopBufferSet {
    fn neural_buffers(&self) -> [&wgpu::Buffer; 7];
    fn compact_readback(&self) -> &wgpu::Buffer;
    fn ownership_token(&self) -> u64;
    fn buffer_set_token(&self) -> u64;
    fn max_neurons(&self) -> u32;
    fn dispatch_capacity_words(&self) -> usize;
    fn frame_payload_capacity_words(&self) -> usize;
    fn compact_readback_capacity_bytes(&self) -> u64;
}

impl ClosedLoopBufferSet for GpuClassBucketBuffers {
    fn neural_buffers(&self) -> [&wgpu::Buffer; 7] {
        self.neural_buffers()
    }
    fn compact_readback(&self) -> &wgpu::Buffer {
        self.compact_readback()
    }
    fn ownership_token(&self) -> u64 {
        self.ownership_token()
    }
    fn buffer_set_token(&self) -> u64 {
        self.buffer_set_token()
    }
    fn max_neurons(&self) -> u32 {
        self.max_neurons()
    }
    fn dispatch_capacity_words(&self) -> usize {
        self.dispatch_capacity_words()
    }
    fn frame_payload_capacity_words(&self) -> usize {
        self.frame_payload_capacity_words()
    }
    fn compact_readback_capacity_bytes(&self) -> u64 {
        self.compact_readback_capacity_bytes()
    }
}

impl ClosedLoopBufferSet for GpuFixedClassArenaBuffers {
    fn neural_buffers(&self) -> [&wgpu::Buffer; 7] {
        self.neural_buffers()
    }
    fn compact_readback(&self) -> &wgpu::Buffer {
        self.compact_readback()
    }
    fn ownership_token(&self) -> u64 {
        self.ownership_token()
    }
    fn buffer_set_token(&self) -> u64 {
        self.buffer_set_token()
    }
    fn max_neurons(&self) -> u32 {
        self.max_neurons()
    }
    fn dispatch_capacity_words(&self) -> usize {
        self.dispatch_capacity_words()
    }
    fn frame_payload_capacity_words(&self) -> usize {
        self.frame_payload_capacity_words()
    }
    fn compact_readback_capacity_bytes(&self) -> u64 {
        self.compact_readback_capacity_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuRecurrentDispatchReceipt {
    pub max_microsteps_dispatched: u32,
    initial_activation_sides: Vec<u32>,
    row_microstep_counts: Vec<u32>,
}

impl GpuRecurrentDispatchReceipt {
    pub fn final_activation_side(&self, row: u32) -> Result<u32, GpuClosedLoopError> {
        let initial = *self
            .initial_activation_sides
            .get(row as usize)
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        let count = *self
            .row_microstep_counts
            .get(row as usize)
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        GpuClosedLoopPipelines::final_activation_side(initial, count)
    }
}

/// Device-owned immutable WGSL kernels shared by every class arena.
pub(crate) struct GpuClosedLoopKernelSet {
    bind_group_layout: wgpu::BindGroupLayout,
    encode_pipeline: wgpu::ComputePipeline,
    recurrent_pipelines: [wgpu::ComputePipeline; 4],
    clear_diagnostics_pipeline: wgpu::ComputePipeline,
    decode_pipeline: wgpu::ComputePipeline,
    select_pipeline: wgpu::ComputePipeline,
}

impl GpuClosedLoopKernelSet {
    pub(crate) fn new(device: &wgpu::Device) -> Result<Arc<Self>, GpuClosedLoopError> {
        let layout = create_neural_bind_group_layout(device);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("closed-loop-neural-pipeline-layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let encode_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-encode-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_ENCODE_WGSL.into()),
        });
        let recurrent_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-recurrent-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_RECURRENT_WGSL.into()),
        });
        let clear_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-clear-diagnostics-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL.into()),
        });
        let decode_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("closed-loop-decode-wgsl"),
            source: wgpu::ShaderSource::Wgsl(CLOSED_LOOP_DECODE_WGSL.into()),
        });
        let encode_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &encode_shader,
            "encode_perception",
            &[],
        );
        let recurrent_pipelines = [0_u32, 1, 2, 3].map(|step| {
            create_compute_pipeline(
                device,
                &pipeline_layout,
                &recurrent_shader,
                "recurrent_microstep",
                &[("microstep_index", step as f64)],
            )
        });
        let clear_diagnostics_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &clear_shader,
            "clear_diagnostics",
            &[],
        );
        let decode_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &decode_shader,
            "decode_candidates",
            &[],
        );
        let select_pipeline = create_compute_pipeline(
            device,
            &pipeline_layout,
            &decode_shader,
            "select_candidate",
            &[],
        );
        Ok(Arc::new(Self {
            bind_group_layout: layout,
            encode_pipeline,
            recurrent_pipelines,
            clear_diagnostics_pipeline,
            decode_pipeline,
            select_pipeline,
        }))
    }
}

pub struct GpuClosedLoopPipelines {
    kernels: Arc<GpuClosedLoopKernelSet>,
    bind_group: wgpu::BindGroup,
    bucket_ownership_token: u64,
    buffer_set_token: u64,
    max_neurons: u32,
    max_compute_workgroups_per_dimension: u32,
    authority: BatchAuthority,
    dispatch_capacity_words: usize,
    frame_payload_capacity_words: usize,
    next_authority_nonce: u64,
    #[cfg(feature = "gpu-tests")]
    force_all_invalid_slot: Option<(u32, u32)>,
}

impl GpuClosedLoopPipelines {
    pub fn new(
        device: &wgpu::Device,
        buffers: &GpuClassBucketBuffers,
    ) -> Result<Self, GpuClosedLoopError> {
        let kernels = GpuClosedLoopKernelSet::new(device)?;
        Self::from_shared_kernel_set(device, buffers, kernels)
    }

    pub(crate) fn from_shared_kernel_set(
        device: &wgpu::Device,
        buffers: &GpuClassBucketBuffers,
        kernels: Arc<GpuClosedLoopKernelSet>,
    ) -> Result<Self, GpuClosedLoopError> {
        Self::from_buffer_set(device, buffers, kernels)
    }

    pub(crate) fn from_shared_kernel_set_for_fixed_arena(
        device: &wgpu::Device,
        buffers: &GpuFixedClassArenaBuffers,
        kernels: Arc<GpuClosedLoopKernelSet>,
    ) -> Result<Self, GpuClosedLoopError> {
        Self::from_buffer_set(device, buffers, kernels)
    }

    fn from_buffer_set(
        device: &wgpu::Device,
        buffers: &impl ClosedLoopBufferSet,
        kernels: Arc<GpuClosedLoopKernelSet>,
    ) -> Result<Self, GpuClosedLoopError> {
        let neural = buffers.neural_buffers();
        let entries = std::array::from_fn::<_, 7, _>(|index| wgpu::BindGroupEntry {
            binding: index as u32,
            resource: neural[index].as_entire_binding(),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("closed-loop-neural-bind-group"),
            layout: &kernels.bind_group_layout,
            entries: &entries,
        });
        Ok(Self {
            kernels,
            bind_group,
            bucket_ownership_token: buffers.ownership_token(),
            buffer_set_token: buffers.buffer_set_token(),
            max_neurons: buffers.max_neurons(),
            max_compute_workgroups_per_dimension: device
                .limits()
                .max_compute_workgroups_per_dimension,
            authority: BatchAuthority::default(),
            dispatch_capacity_words: buffers.dispatch_capacity_words(),
            frame_payload_capacity_words: buffers.frame_payload_capacity_words(),
            next_authority_nonce: 1,
            #[cfg(feature = "gpu-tests")]
            force_all_invalid_slot: None,
        })
    }

    pub fn build_active_batch(
        &mut self,
        plan: &crate::GpuClassBucketPlan,
        entries: &[GpuActiveBatchEntry<'_>],
        frame_base_words: u32,
    ) -> Result<GpuActiveBatchUpload, GpuClosedLoopError> {
        let dispatch_generation = NonZeroU64::new(self.next_authority_nonce)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let prepared =
            self.preflight_active_batch(plan, entries, frame_base_words, dispatch_generation)?;
        self.begin_prepared_batch(prepared)
    }

    /// Performs the complete class-local host preflight without reserving a
    /// private nonce or mutating persistent active-side authority.
    pub(crate) fn preflight_active_batch(
        &self,
        plan: &crate::GpuClassBucketPlan,
        entries: &[GpuActiveBatchEntry<'_>],
        frame_base_words: u32,
        dispatch_generation: NonZeroU64,
    ) -> Result<GpuPreparedActiveBatch, GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        if plan.ownership_token() != self.bucket_ownership_token
            || plan.capacity().execution().max_neurons() != self.max_neurons
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        for entry in entries {
            plan.validate_slot_handle(entry.slot)?;
            let record = entry.slot.record();
            if record.neuron_count != self.max_neurons
                || record.microstep_count < 2
                || record.microstep_count > 4
            {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
        }
        let views = entries
            .iter()
            .map(|entry| GpuBatchEntryView {
                frame: entry.frame,
                slot: entry.slot,
            })
            .collect::<Vec<_>>();
        let batch = GpuActiveBatchUpload::try_from_views(
            &views,
            frame_base_words,
            self.bucket_ownership_token,
            &self.authority.active_sides,
            self.dispatch_capacity_words,
            self.frame_payload_capacity_words,
            dispatch_generation,
            0,
        )?;
        Ok(GpuPreparedActiveBatch { batch })
    }

    pub(crate) fn preflight_fixed_active_batch(
        &self,
        entries: &[GpuFixedActiveBatchEntry<'_>],
        frame_base_words: u32,
        dispatch_generation: NonZeroU64,
    ) -> Result<GpuPreparedActiveBatch, GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some() || entries.is_empty() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let views = entries
            .iter()
            .map(|entry| GpuBatchEntryView {
                frame: entry.frame,
                slot: entry.slot,
            })
            .collect::<Vec<_>>();
        if views.iter().any(|entry| {
            entry.slot.record().slot != entry.slot.brain_slot_index()
                || entry.slot.record().neuron_count != self.max_neurons
                || !(2..=4).contains(&entry.slot.record().microstep_count)
        }) {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        let batch = GpuActiveBatchUpload::try_from_views(
            &views,
            frame_base_words,
            self.bucket_ownership_token,
            &self.authority.active_sides,
            self.dispatch_capacity_words,
            self.frame_payload_capacity_words,
            dispatch_generation,
            0,
        )?;
        Ok(GpuPreparedActiveBatch { batch })
    }

    /// Reserves the class-private authority nonce only after every class in a
    /// backend-global transaction has passed preflight.
    pub(crate) fn begin_prepared_batch(
        &mut self,
        mut prepared: GpuPreparedActiveBatch,
    ) -> Result<GpuActiveBatchUpload, GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some()
            || prepared.batch.bucket_ownership_token != self.bucket_ownership_token
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        for header in &prepared.batch.headers {
            let side = self
                .authority
                .active_sides
                .get(&(header.brain_slot_index, header.slot_generation))
                .copied()
                .unwrap_or(0);
            if side != header.active_activation_side {
                return Err(GpuClosedLoopError::StaleOrForeignHandle);
            }
        }
        let authority_nonce = self.next_authority_nonce;
        let next_authority_nonce = authority_nonce
            .checked_add(1)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        self.authority.begin(authority_nonce)?;
        for header in &prepared.batch.headers {
            self.authority
                .active_sides
                .entry((header.brain_slot_index, header.slot_generation))
                .or_insert(0);
        }
        prepared.batch.authority_nonce = authority_nonce;
        self.next_authority_nonce = next_authority_nonce;
        Ok(prepared.batch)
    }

    /// Explicitly releases a built batch that will not be submitted. The
    /// opaque batch is consumed, persistent activation sides are unchanged,
    /// and a later batch receives a fresh nonce.
    pub fn abandon_unsubmitted_batch(
        &mut self,
        batch: GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        if batch.bucket_ownership_token != self.bucket_ownership_token {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        self.authority.abandon_unsubmitted(batch.authority_nonce)
    }

    pub(crate) fn rollback_recorded_batch(
        &mut self,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_batch_identity(batch)?;
        self.authority.recording_failed(batch.authority_nonce)
    }

    pub(crate) fn mark_post_submit_poison(
        &mut self,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_batch_identity(batch)?;
        self.authority
            .submission_indeterminate(batch.authority_nonce)
    }

    pub(crate) fn retire_slot_active_side(
        &mut self,
        slot: u32,
        generation: u32,
    ) -> Result<(), GpuClosedLoopError> {
        self.authority.retire_active_side(slot, generation)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn reset_active_sides_for_hardware_diagnostic(&mut self) -> Result<(), GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if self.authority.pending.is_some() {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        self.authority.active_sides.clear();
        Ok(())
    }

    pub const fn recurrent_variant_count() -> usize {
        4
    }

    pub const fn recurrent_variant_microstep_indices() -> [u32; 4] {
        [0, 1, 2, 3]
    }

    pub fn validate_microstep_count(microsteps: u32) -> Result<(), GpuClosedLoopError> {
        if (2..=4).contains(&microsteps) {
            Ok(())
        } else {
            Err(GpuClosedLoopError::MalformedUpload)
        }
    }

    pub fn final_activation_side(
        initial_side: u32,
        microsteps: u32,
    ) -> Result<u32, GpuClosedLoopError> {
        Self::validate_microstep_count(microsteps)?;
        if initial_side > 1 {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        Ok(initial_side ^ (microsteps & 1))
    }

    pub(crate) fn write_staged_uploads(
        &self,
        queue: &wgpu::Queue,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_buffers_and_dispatch(buffers, batch)?;
        let neural = buffers.neural_buffers();
        queue.write_buffer(
            neural[4],
            0,
            bytemuck::cast_slice(batch.dispatch_header_words()),
        );
        queue.write_buffer(
            neural[5],
            0,
            bytemuck::cast_slice(batch.frame_payload_words()),
        );
        Ok(())
    }

    /// Records this class's complete closed-loop work and compact 48-byte row
    /// copies into a caller-owned encoder. It never submits or commits sides.
    pub(crate) fn record_staged_closed_loop(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<u64, GpuClosedLoopError> {
        self.validate_buffers_and_dispatch(buffers, batch)?;
        let readback_bytes = self.readback_bytes(buffers, batch)?;
        self.authority.record_encode(batch.authority_nonce)?;
        let result = (|| {
            self.record_encode(encoder, batch)?;
            self.record_microsteps(encoder, batch)?;
            self.authority.record_recurrent(batch.authority_nonce)?;
            self.record_decode_select(encoder, batch)?;
            self.authority.record_selection(batch.authority_nonce)?;
            let neural = buffers.neural_buffers();
            for (row, selection_offset) in batch.selection_offsets.iter().enumerate() {
                encoder.copy_buffer_to_buffer(
                    neural[6],
                    u64::from(*selection_offset) * 4,
                    buffers.compact_readback(),
                    row as u64 * 48,
                    48,
                );
            }
            Ok(readback_bytes)
        })();
        if result.is_err() {
            self.authority.recording_failed(batch.authority_nonce)?;
        }
        result
    }

    pub(crate) fn register_compact_mapping(
        &self,
        command_buffer: &wgpu::CommandBuffer,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<GpuCompactMapTicket, GpuClosedLoopError> {
        self.validate_buffers_and_dispatch(buffers, batch)?;
        self.authority.require_stage(
            batch.authority_nonce,
            BatchLifecycleStage::SelectionRecorded,
        )?;
        let readback_bytes = self.readback_bytes(buffers, batch)?;
        let (sender, receiver) = mpsc::channel();
        command_buffer.map_buffer_on_submit(
            buffers.compact_readback(),
            wgpu::MapMode::Read,
            0..readback_bytes,
            move |result| {
                let _ = sender.send(result);
            },
        );
        Ok(GpuCompactMapTicket { receiver })
    }

    /// Copies and validates compact GPU records but deliberately leaves the
    /// class active-side authority unchanged until `commit_validated_batch`.
    pub(crate) fn decode_validate_mapped_records(
        &mut self,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<GpuValidatedClassBatch, GpuClosedLoopError> {
        self.validate_buffers_and_dispatch(buffers, batch)?;
        self.authority.require_stage(
            batch.authority_nonce,
            BatchLifecycleStage::SelectionRecorded,
        )?;
        let readback_bytes = self.readback_bytes(buffers, batch)?;
        let mapped = buffers
            .compact_readback()
            .slice(..readback_bytes)
            .get_mapped_range();
        let words: Vec<u32> = bytemuck::cast_slice(&mapped).to_vec();
        drop(mapped);
        buffers.compact_readback().unmap();
        #[allow(unused_mut)]
        let mut records = words
            .chunks_exact(12)
            .map(GpuSelectionRecord::from_words)
            .collect::<Result<Vec<_>, _>>()?;
        #[cfg(feature = "gpu-tests")]
        if let Some((slot, generation)) = self.force_all_invalid_slot.take() {
            let record = records
                .iter_mut()
                .find(|record| record.slot == slot && record.slot_generation == generation)
                .ok_or(GpuClosedLoopError::StaleOrForeignHandle)?;
            record.candidate_index = u32::MAX;
            record.logit_bits = 0;
            record.confidence_q16 = 0;
            record.status = 2;
        }
        if !self.validate_selection_records(batch, &records) {
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let final_sides = batch
            .headers
            .iter()
            .zip(&records)
            .map(|(header, record)| {
                (
                    header.brain_slot_index,
                    header.slot_generation,
                    record.active_activation_side,
                )
            })
            .collect();
        Ok(GpuValidatedClassBatch {
            bucket_ownership_token: self.bucket_ownership_token,
            authority_nonce: batch.authority_nonce,
            records,
            final_sides,
            readback_bytes,
        })
    }

    #[cfg(feature = "gpu-tests")]
    pub(crate) fn force_all_invalid_record_for_test(&mut self, slot: u32, generation: u32) {
        self.force_all_invalid_slot = Some((slot, generation));
    }

    pub(crate) fn commit_validated_batch(
        &mut self,
        validated: GpuValidatedClassBatch,
    ) -> Result<(Vec<GpuSelectionRecord>, u64), GpuClosedLoopError> {
        if validated.bucket_ownership_token != self.bucket_ownership_token {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        self.authority
            .submission_succeeded(validated.authority_nonce, &validated.final_sides)?;
        Ok((validated.records, validated.readback_bytes))
    }

    pub(crate) fn prevalidate_commit_validated_batch(
        &self,
        validated: &GpuValidatedClassBatch,
    ) -> Result<(), GpuClosedLoopError> {
        if validated.bucket_ownership_token != self.bucket_ownership_token {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        self.authority
            .prevalidate_submission_succeeded(validated.authority_nonce)
    }

    #[cfg(feature = "gpu-tests")]
    pub fn submit_encode_and_microsteps(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        batch: &GpuActiveBatchUpload,
    ) -> Result<GpuRecurrentDispatchReceipt, GpuClosedLoopError> {
        self.validate_dispatch(batch)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("closed-loop-authoritative-batch"),
        });
        self.authority.record_encode(batch.authority_nonce)?;
        if let Err(error) = self.record_encode(&mut encoder, batch) {
            self.authority.recording_failed(batch.authority_nonce)?;
            return Err(error);
        }
        let receipt = match self.record_microsteps(&mut encoder, batch) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.authority.recording_failed(batch.authority_nonce)?;
                return Err(error);
            }
        };
        self.authority.record_recurrent(batch.authority_nonce)?;
        let submission = queue.submit(Some(encoder.finish()));
        if device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
        {
            self.authority
                .submission_indeterminate(batch.authority_nonce)?;
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let final_sides = batch
            .headers
            .iter()
            .map(|header| {
                Ok((
                    header.brain_slot_index,
                    header.slot_generation,
                    Self::final_activation_side(
                        header.active_activation_side,
                        header.microstep_count,
                    )?,
                ))
            })
            .collect::<Result<Vec<_>, GpuClosedLoopError>>()?;
        self.authority
            .recurrent_diagnostic_succeeded(batch.authority_nonce, &final_sides)?;
        Ok(receipt)
    }

    pub async fn submit_closed_loop_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &GpuClassBucketBuffers,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(Vec<GpuSelectionRecord>, u64), GpuClosedLoopError> {
        self.write_staged_uploads(queue, buffers, batch)?;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("closed-loop-authoritative-frame"),
        });
        self.record_staged_closed_loop(&mut encoder, buffers, batch)?;
        let command_buffer = encoder.finish();
        let map_ticket = self.register_compact_mapping(&command_buffer, buffers, batch)?;
        let submission = queue.submit(Some(command_buffer));
        let poll_result = device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: None,
        });
        if poll_result.is_err() || !map_ticket.mapping_succeeded() {
            self.mark_post_submit_poison(batch)?;
            buffers.compact_readback().unmap();
            return Err(GpuClosedLoopError::SubmissionFailed);
        }
        let validated = match self.decode_validate_mapped_records(buffers, batch) {
            Ok(validated) => validated,
            Err(_) => {
                self.mark_post_submit_poison(batch)?;
                return Err(GpuClosedLoopError::SubmissionFailed);
            }
        };
        self.commit_validated_batch(validated)
    }

    fn record_decode_select(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_dispatch(batch)?;
        let rows =
            u32::try_from(batch.row_count()).map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-decode-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.kernels.decode_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(1, rows, 1);
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("closed-loop-select-pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.kernels.select_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(1, rows, 1);
        Ok(())
    }

    fn validate_selection_records(
        &self,
        batch: &GpuActiveBatchUpload,
        records: &[GpuSelectionRecord],
    ) -> bool {
        records.len() == batch.headers.len()
            && records.iter().zip(&batch.headers).all(|(record, header)| {
                let generation = u64::from(record.dispatch_generation_lo)
                    | (u64::from(record.dispatch_generation_hi) << 32);
                let expected_generation = u64::from(header.dispatch_generation_lo)
                    | (u64::from(header.dispatch_generation_hi) << 32);
                let expected_side = Self::final_activation_side(
                    header.active_activation_side,
                    header.microstep_count,
                )
                .ok();
                if record.slot != header.slot
                    || record.slot_generation != header.slot_generation
                    || generation == 0
                    || generation != expected_generation
                    || Some(record.active_activation_side) != expected_side
                    || record.active_tiles == 0
                    || record.active_synapses == 0
                {
                    return false;
                }
                match record.status {
                    1 => {
                        if record.candidate_index >= header.candidate_count
                            || !f32::from_bits(record.logit_bits).is_finite()
                        {
                            return false;
                        }
                        let base = header.candidate_offset as usize
                            + record.candidate_index as usize * GPU_CANDIDATE_RECORD_WORDS;
                        GpuCandidateRecord::from_words(
                            &batch.dispatch_header_words[base..base + GPU_CANDIDATE_RECORD_WORDS],
                        )
                        .is_ok_and(|candidate| candidate.confidence_q16 == record.confidence_q16)
                    }
                    2 => {
                        record.candidate_index == u32::MAX
                            && record.logit_bits == 0
                            && record.confidence_q16 == 0
                    }
                    _ => false,
                }
            })
    }

    fn record_encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-clear-diagnostics-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.kernels.clear_diagnostics_pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(
                1,
                u32::try_from(batch.row_count())
                    .map_err(|_| GpuClosedLoopError::CapacityExceeded)?,
                1,
            );
        }
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("closed-loop-encode-pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.kernels.encode_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(
            self.max_neurons.div_ceil(WORKGROUP_SIZE),
            u32::try_from(batch.row_count()).map_err(|_| GpuClosedLoopError::CapacityExceeded)?,
            1,
        );
        Ok(())
    }

    /// Dispatches recurrent WGSL only. Task 6 consumes diagnostic lane 3 as
    /// the GPU-authored active-side receipt before decode/select.
    fn record_microsteps(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        batch: &GpuActiveBatchUpload,
    ) -> Result<GpuRecurrentDispatchReceipt, GpuClosedLoopError> {
        self.validate_dispatch(batch)?;
        let max_microsteps = batch
            .headers
            .iter()
            .map(|h| h.microstep_count)
            .max()
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        Self::validate_microstep_count(max_microsteps)?;
        for step in 0..max_microsteps as usize {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-recurrent-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.kernels.recurrent_pipelines[step]);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(
                self.max_neurons.div_ceil(WORKGROUP_SIZE),
                u32::try_from(batch.row_count())
                    .map_err(|_| GpuClosedLoopError::CapacityExceeded)?,
                1,
            );
        }
        let initial_activation_sides = batch
            .headers
            .iter()
            .map(|header| header.active_activation_side)
            .collect::<Vec<_>>();
        let row_microstep_counts = batch
            .headers
            .iter()
            .map(|header| header.microstep_count)
            .collect::<Vec<_>>();
        Ok(GpuRecurrentDispatchReceipt {
            max_microsteps_dispatched: max_microsteps,
            initial_activation_sides,
            row_microstep_counts,
        })
    }

    fn validate_batch_identity(
        &self,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if batch.bucket_ownership_token != self.bucket_ownership_token
            || self.authority.pending.map(|pending| pending.nonce) != Some(batch.authority_nonce)
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        Ok(())
    }

    fn validate_buffers_and_dispatch(
        &self,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<(), GpuClosedLoopError> {
        if buffers.ownership_token() != self.bucket_ownership_token
            || buffers.buffer_set_token() != self.buffer_set_token
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        self.validate_dispatch(batch)
    }

    fn readback_bytes(
        &self,
        buffers: &impl ClosedLoopBufferSet,
        batch: &GpuActiveBatchUpload,
    ) -> Result<u64, GpuClosedLoopError> {
        let bytes = batch
            .row_count()
            .checked_mul(48)
            .and_then(|value| u64::try_from(value).ok())
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if bytes == 0 || bytes > buffers.compact_readback_capacity_bytes() {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        Ok(bytes)
    }

    fn validate_dispatch(&self, batch: &GpuActiveBatchUpload) -> Result<(), GpuClosedLoopError> {
        self.validate_batch_identity(batch)?;
        if batch.headers.iter().any(|header| {
            self.authority
                .active_sides
                .get(&(header.brain_slot_index, header.slot_generation))
                .copied()
                != Some(header.active_activation_side)
        }) {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        validate_dispatch_dimensions(
            self.max_neurons,
            batch.row_count(),
            self.max_compute_workgroups_per_dimension,
        )?;
        validate_dispatch(self.max_neurons, batch)
    }
}

pub fn validate_dispatch_dimensions(
    max_neurons: u32,
    row_count: usize,
    limit: u32,
) -> Result<[u32; 3], GpuClosedLoopError> {
    if max_neurons == 0 || row_count == 0 {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    let y = u32::try_from(row_count).map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
    let dimensions = [max_neurons.div_ceil(WORKGROUP_SIZE), y, 1];
    if dimensions.into_iter().any(|value| value > limit) {
        return Err(GpuClosedLoopError::CapacityExceeded);
    }
    Ok(dimensions)
}

fn validate_dispatch(
    max_neurons: u32,
    batch: &GpuActiveBatchUpload,
) -> Result<(), GpuClosedLoopError> {
    if max_neurons == 0
        || batch.headers.is_empty()
        || batch.headers.iter().any(|header| {
            header.neuron_count == 0
                || header.neuron_count != max_neurons
                || header.active_activation_side > 1
        })
        || batch.dispatch_header_words.len() != batch.row_count() * GPU_ACTIVE_DISPATCH_ROW_WORDS
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    Ok(())
}

fn create_neural_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let entries = crate::GpuClassBucketBuffers::neural_binding_manifest().map(|manifest| {
        wgpu::BindGroupLayoutEntry {
            binding: manifest.binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage {
                    read_only: matches!(manifest.access, crate::GpuBufferAccess::ReadOnly),
                },
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(manifest.minimum_binding_size_bytes),
            },
            count: None,
        }
    });
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("closed-loop-neural-bind-group-layout"),
        entries: &entries,
    })
}

fn create_compute_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    entry_point: &'static str,
    constants: &[(&str, f64)],
) -> wgpu::ComputePipeline {
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(entry_point),
        layout: Some(layout),
        module: shader,
        entry_point: Some(entry_point),
        compilation_options: wgpu::PipelineCompilationOptions {
            constants,
            zero_initialize_workgroup_memory: true,
        },
        cache: None,
    })
}

#[cfg(test)]
mod lifecycle_tests {
    use bytemuck::Zeroable;

    use super::*;

    #[test]
    fn backend_dispatch_generation_is_distinct_from_private_class_nonce() {
        let mut header = GpuPerceptionHeader::zeroed();
        header.dispatch_generation_lo = 0x5566_7788;
        header.dispatch_generation_hi = 0x1122_3344;
        let batch = GpuActiveBatchUpload {
            headers: vec![header],
            dispatch_header_words: header.words().to_vec(),
            frame_payload_words: Vec::new(),
            bucket_ownership_token: 1,
            authority_nonce: 7,
            selection_offsets: vec![0],
        };

        assert_eq!(batch.authority_nonce_for_test(), 7);
        assert_eq!(batch.dispatch_generation(), 0x1122_3344_5566_7788);
    }

    #[test]
    fn validated_sides_remain_staged_until_explicit_commit() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((3, 7), 0);
        authority.begin(17).unwrap();
        authority.record_encode(17).unwrap();
        authority.record_recurrent(17).unwrap();
        authority.record_selection(17).unwrap();
        let validated_sides = [(3, 7, 1)];

        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&0));
        authority
            .submission_succeeded(17, &validated_sides)
            .unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
    }

    #[test]
    fn retiring_slot_side_is_exact_and_forbidden_while_a_batch_is_pending() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((5, 11), 1);
        authority.active_sides.insert((5, 12), 0);
        authority.begin(23).unwrap();
        assert_eq!(
            authority.retire_active_side(5, 11),
            Err(GpuClosedLoopError::MalformedUpload)
        );
        authority.abandon_unsubmitted(23).unwrap();
        authority.retire_active_side(5, 11).unwrap();
        assert!(!authority.active_sides.contains_key(&(5, 11)));
        assert_eq!(authority.active_sides.get(&(5, 12)), Some(&0));
    }

    #[test]
    fn recurrent_recording_before_same_batch_encode_is_rejected() {
        let mut authority = BatchAuthority::default();
        authority.begin(41).unwrap();
        assert_eq!(
            authority.record_recurrent(41),
            Err(GpuClosedLoopError::MalformedUpload)
        );
        assert_eq!(authority.pending.unwrap().stage, BatchLifecycleStage::Built);
    }

    #[test]
    fn unsubmitted_or_pre_submit_failure_preserves_side_and_same_nonce_retry() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((3, 7), 1);
        authority.begin(52).unwrap();

        // A built batch that is dropped before recording has no authority to
        // mutate the persistent side and retains its exact retry nonce.
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
        assert_eq!(authority.pending.unwrap().nonce, 52);

        authority.abandon_unsubmitted(52).unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
        assert_eq!(authority.pending, None);
        authority.begin(53).unwrap();

        authority.record_encode(53).unwrap();
        authority.record_recurrent(53).unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
        assert_eq!(authority.pending.unwrap().nonce, 53);
        authority.recording_failed(53).unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&1));
        assert_eq!(
            authority.pending,
            Some(PendingBatchAuthority {
                nonce: 53,
                stage: BatchLifecycleStage::Built
            })
        );

        authority.record_encode(53).unwrap();
        authority.record_recurrent(53).unwrap();
        authority.record_selection(53).unwrap();
        authority.submission_succeeded(53, &[(3, 7, 0)]).unwrap();
        assert_eq!(authority.active_sides.get(&(3, 7)), Some(&0));
        assert_eq!(authority.pending, None);
    }

    #[test]
    fn recording_failure_rolls_the_exact_nonce_back_to_built() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((5, 9), 0);
        authority.begin(77).unwrap();
        authority.record_encode(77).unwrap();

        // This is the rollback used if either private command-recording stage
        // returns an error before Queue::submit.
        authority.recording_failed(77).unwrap();
        assert_eq!(authority.active_sides.get(&(5, 9)), Some(&0));
        assert_eq!(
            authority.pending,
            Some(PendingBatchAuthority {
                nonce: 77,
                stage: BatchLifecycleStage::Built
            })
        );
        authority.record_encode(77).unwrap();
        authority.record_recurrent(77).unwrap();
    }

    #[test]
    fn post_submit_failure_poison_rejects_retry_abandon_and_new_batches() {
        let mut authority = BatchAuthority::default();
        authority.active_sides.insert((8, 11), 1);
        authority.begin(91).unwrap();
        authority.record_encode(91).unwrap();
        authority.record_recurrent(91).unwrap();
        authority.submission_indeterminate(91).unwrap();

        assert_eq!(authority.active_sides.get(&(8, 11)), Some(&1));
        assert_eq!(authority.poisoned_nonce, Some(91));
        assert_eq!(authority.pending.unwrap().nonce, 91);
        assert_eq!(
            authority.record_encode(91),
            Err(GpuClosedLoopError::SubmissionFailed)
        );
        assert_eq!(
            authority.abandon_unsubmitted(91),
            Err(GpuClosedLoopError::SubmissionFailed)
        );
        assert_eq!(
            authority.submission_succeeded(91, &[(8, 11, 0)]),
            Err(GpuClosedLoopError::SubmissionFailed)
        );
        assert_eq!(
            authority.begin(92),
            Err(GpuClosedLoopError::SubmissionFailed)
        );
        assert_eq!(authority.active_sides.get(&(8, 11)), Some(&1));
    }
}
