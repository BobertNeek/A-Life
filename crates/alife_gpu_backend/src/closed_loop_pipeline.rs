//! Production GPU-authoritative perception encoding and recurrent dispatch.
//!
//! Task 6 adds candidate decode/selection. This module deliberately exposes no
//! CPU neural execution and obtains neural results only from WGSL state.

use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU64,
};

use alife_core::{PerceptionFrame, MAX_ACTION_CANDIDATES};

use crate::{
    GpuBrainSlot, GpuClassBucketBuffers, GpuClosedLoopError, GpuPerceptionHeader,
    GpuPerceptionUpload,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BatchLifecycleStage {
    Built,
    EncodeRecorded,
    RecurrentRecorded,
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
        if pending.stage != BatchLifecycleStage::RecurrentRecorded {
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

    fn ensure_healthy(&self) -> Result<(), GpuClosedLoopError> {
        if self.poisoned_nonce.is_some() {
            Err(GpuClosedLoopError::SubmissionFailed)
        } else {
            Ok(())
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuActiveBatchUpload {
    headers: Vec<GpuPerceptionHeader>,
    dispatch_header_words: Vec<u32>,
    frame_payload_words: Vec<u32>,
    bucket_ownership_token: u64,
    authority_nonce: u64,
}

impl GpuActiveBatchUpload {
    fn try_from_entries(
        entries: &[GpuActiveBatchEntry<'_>],
        frame_base_words: u32,
        bucket_ownership_token: u64,
        active_sides: &BTreeMap<(u32, u32), u32>,
        dispatch_capacity_words: usize,
        frame_payload_capacity_words: usize,
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
                *active_sides
                    .get(&(
                        entry.slot.brain_slot_index(),
                        entry.slot.record().slot_generation,
                    ))
                    .ok_or(GpuClosedLoopError::StaleOrForeignHandle)?,
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
            headers.push(upload.header);
        }

        Ok(Self {
            headers,
            dispatch_header_words,
            frame_payload_words,
            bucket_ownership_token,
            authority_nonce,
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
    #[cfg(any(test, feature = "gpu-tests"))]
    pub fn zero_frame_payload_for_hardware_diagnostic(&mut self) {
        self.frame_payload_words.fill(0);
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

pub struct GpuClosedLoopPipelines {
    bind_group: wgpu::BindGroup,
    encode_pipeline: wgpu::ComputePipeline,
    recurrent_pipelines: [wgpu::ComputePipeline; 4],
    clear_diagnostics_pipeline: wgpu::ComputePipeline,
    bucket_ownership_token: u64,
    max_neurons: u32,
    max_compute_workgroups_per_dimension: u32,
    authority: BatchAuthority,
    dispatch_capacity_words: usize,
    frame_payload_capacity_words: usize,
    next_authority_nonce: u64,
}

impl GpuClosedLoopPipelines {
    pub fn new(
        device: &wgpu::Device,
        buffers: &GpuClassBucketBuffers,
    ) -> Result<Self, GpuClosedLoopError> {
        let layout = create_neural_bind_group_layout(device);
        let neural = buffers.neural_buffers();
        let entries = std::array::from_fn::<_, 7, _>(|index| wgpu::BindGroupEntry {
            binding: index as u32,
            resource: neural[index].as_entire_binding(),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("closed-loop-neural-bind-group"),
            layout: &layout,
            entries: &entries,
        });
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
        Ok(Self {
            bind_group,
            encode_pipeline,
            recurrent_pipelines,
            clear_diagnostics_pipeline,
            bucket_ownership_token: buffers.ownership_token(),
            max_neurons: buffers.max_neurons(),
            max_compute_workgroups_per_dimension: device
                .limits()
                .max_compute_workgroups_per_dimension,
            authority: BatchAuthority::default(),
            dispatch_capacity_words: buffers.dispatch_capacity_words(),
            frame_payload_capacity_words: buffers.frame_payload_capacity_words(),
            next_authority_nonce: 1,
        })
    }

    pub fn build_active_batch(
        &mut self,
        plan: &crate::GpuClassBucketPlan,
        entries: &[GpuActiveBatchEntry<'_>],
        frame_base_words: u32,
    ) -> Result<GpuActiveBatchUpload, GpuClosedLoopError> {
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
            self.authority
                .active_sides
                .entry((entry.slot.brain_slot_index(), record.slot_generation))
                .or_insert(0);
        }
        let authority_nonce = self.next_authority_nonce;
        self.next_authority_nonce = self
            .next_authority_nonce
            .checked_add(1)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let batch = GpuActiveBatchUpload::try_from_entries(
            entries,
            frame_base_words,
            self.bucket_ownership_token,
            &self.authority.active_sides,
            self.dispatch_capacity_words,
            self.frame_payload_capacity_words,
            authority_nonce,
        )?;
        self.authority.begin(authority_nonce)?;
        Ok(batch)
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
            .submission_succeeded(batch.authority_nonce, &final_sides)?;
        Ok(receipt)
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
            pass.set_pipeline(&self.clear_diagnostics_pipeline);
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
        pass.set_pipeline(&self.encode_pipeline);
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
            pass.set_pipeline(&self.recurrent_pipelines[step]);
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

    fn validate_dispatch(&self, batch: &GpuActiveBatchUpload) -> Result<(), GpuClosedLoopError> {
        self.authority.ensure_healthy()?;
        if batch.bucket_ownership_token != self.bucket_ownership_token {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        if self.authority.pending.map(|pending| pending.nonce) != Some(batch.authority_nonce)
            || batch.headers.iter().any(|header| {
                self.authority
                    .active_sides
                    .get(&(header.brain_slot_index, header.slot_generation))
                    .copied()
                    != Some(header.active_activation_side)
            })
        {
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
    use super::*;

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
