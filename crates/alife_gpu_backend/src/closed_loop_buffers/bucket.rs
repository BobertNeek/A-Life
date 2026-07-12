use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use alife_core::{BrainCapacityClass, BrainPhenotype, MAX_ACTION_CANDIDATES};
use bytemuck::Zeroable;

use super::{
    GpuBrainSlotRecord, GpuClosedLoopError, GpuPhenotypeIdentityRecord, GpuPhenotypeUpload,
    GPU_NO_EXTENSION_SENTINEL,
};

static NEXT_BUCKET_OWNERSHIP_TOKEN: AtomicU64 = AtomicU64::new(1);
static NEXT_BUFFER_SET_TOKEN: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuTypedCounts {
    pub encoder_plans: usize,
    pub encoder_assignments: usize,
    pub encoder_target_offsets: usize,
    pub neuron_dynamics: usize,
    pub projections: usize,
    pub route_metadata: usize,
    pub target_offsets: usize,
    pub source_indices: usize,
    pub route_indices: usize,
    pub decoder_plans: usize,
    pub decoder_families: usize,
    pub decoder_weight_indices: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuSlotWordRanges {
    pub encoder_plan_words: Range<u32>,
    pub encoder_assignment_words: Range<u32>,
    pub encoder_target_offset_words: Range<u32>,
    pub neuron_dynamics_words: Range<u32>,
    pub projection_words: Range<u32>,
    pub route_metadata_words: Range<u32>,
    pub target_offset_words: Range<u32>,
    pub source_index_words: Range<u32>,
    pub route_index_words: Range<u32>,
    pub decoder_plan_words: Range<u32>,
    pub decoder_family_words: Range<u32>,
    pub decoder_weight_index_words: Range<u32>,
    pub genetic_weight_words: Range<u32>,
    pub alpha_words: Range<u32>,
    pub activation_a_words: Range<u32>,
    pub activation_b_words: Range<u32>,
    pub accumulator_words: Range<u32>,
    pub homeostasis_words: Range<u32>,
    pub lifetime_weight_words: Range<u32>,
    pub fast_weight_words: Range<u32>,
    pub recurrent_eligibility_words: Range<u32>,
    pub decoder_eligibility_words: Range<u32>,
    pub encoded_input_words: Range<u32>,
    pub candidate_logit_words: Range<u32>,
    pub diagnostic_words: Range<u32>,
    pub selection_words: Range<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBrainSlot {
    record: GpuBrainSlotRecord,
    identity: GpuPhenotypeIdentityRecord,
    counts: GpuTypedCounts,
    ranges: GpuSlotWordRanges,
    brain_slot_index: u32,
    bucket_ownership_token: u64,
}
impl GpuBrainSlot {
    pub const fn record(&self) -> &GpuBrainSlotRecord {
        &self.record
    }
    pub const fn identity(&self) -> &GpuPhenotypeIdentityRecord {
        &self.identity
    }
    pub const fn typed_counts(&self) -> &GpuTypedCounts {
        &self.counts
    }
    pub const fn word_ranges(&self) -> &GpuSlotWordRanges {
        &self.ranges
    }
    pub const fn brain_slot_index(&self) -> u32 {
        self.brain_slot_index
    }
}

#[derive(Debug)]
pub struct GpuClassBucketPlan {
    capacity: BrainCapacityClass,
    slot_capacity: u32,
    slots: Vec<Option<GpuBrainSlot>>,
    brain_slot_records: Vec<GpuBrainSlotRecord>,
    phenotype_identities: Vec<GpuPhenotypeIdentityRecord>,
    immutable_plan_words: Vec<u32>,
    immutable_weight_words: Vec<u32>,
    mutable_state_words: Vec<u32>,
    bucket_ownership_token: u64,
}

impl GpuClassBucketPlan {
    pub fn new(
        capacity: BrainCapacityClass,
        slot_capacity: u32,
    ) -> Result<Self, GpuClosedLoopError> {
        capacity
            .validate_contract()
            .map_err(|_| GpuClosedLoopError::LayoutMismatch)?;
        if slot_capacity == 0 {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let slot_count =
            usize::try_from(slot_capacity).map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        validate_fixed_slot_heaps(&capacity, slot_count)?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(slot_count)
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        slots.resize(slot_count, None);
        let mut brain_slot_records = Vec::new();
        brain_slot_records
            .try_reserve_exact(slot_count)
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        brain_slot_records.resize(slot_count, GpuBrainSlotRecord::zeroed());
        let mut phenotype_identities = Vec::new();
        phenotype_identities
            .try_reserve_exact(slot_count)
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        phenotype_identities.resize(slot_count, GpuPhenotypeIdentityRecord::zeroed());
        let bucket_ownership_token = NEXT_BUCKET_OWNERSHIP_TOKEN
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        Ok(Self {
            capacity,
            slot_capacity,
            slots,
            brain_slot_records,
            phenotype_identities,
            immutable_plan_words: Vec::new(),
            immutable_weight_words: Vec::new(),
            mutable_state_words: Vec::new(),
            bucket_ownership_token,
        })
    }

    pub fn insert_phenotype(
        &mut self,
        slot: u32,
        generation: u32,
        phenotype: &BrainPhenotype,
    ) -> Result<GpuBrainSlot, GpuClosedLoopError> {
        if generation == 0
            || slot >= self.slot_capacity
            || self.slots[slot as usize].is_some()
            || phenotype.brain_class_id() != self.capacity.id()
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        let upload = GpuPhenotypeUpload::try_from(phenotype)?;
        let counts = GpuTypedCounts {
            encoder_plans: upload.encoder_plans.len(),
            encoder_assignments: upload.encoder_assignments.len(),
            encoder_target_offsets: upload.encoder_target_offsets.len(),
            neuron_dynamics: upload.neuron_dynamics.len(),
            projections: upload.projections.len(),
            route_metadata: upload.route_metadata.len(),
            target_offsets: upload.target_offsets.len(),
            source_indices: upload.source_indices.len(),
            route_indices: upload.route_indices.len(),
            decoder_plans: upload.decoder_plans.len(),
            decoder_families: upload.decoder_families.len(),
            decoder_weight_indices: upload.decoder_weight_indices.len(),
        };

        let p0 = u32_len(&self.immutable_plan_words)?;
        let encoder_plan_words = span(p0, counts.encoder_plans * 8)?;
        let encoder_assignment_words =
            span(encoder_plan_words.end, counts.encoder_assignments * 8)?;
        let encoder_target_offset_words =
            span(encoder_assignment_words.end, counts.encoder_target_offsets)?;
        let neuron_dynamics_words =
            span(encoder_target_offset_words.end, counts.neuron_dynamics * 8)?;
        let projection_words = span(neuron_dynamics_words.end, counts.projections * 8)?;
        let route_metadata_words = span(projection_words.end, counts.route_metadata * 12)?;
        let target_offset_words = span(route_metadata_words.end, counts.target_offsets)?;
        let source_index_words = span(target_offset_words.end, counts.source_indices)?;
        let route_index_words = span(source_index_words.end, counts.route_indices)?;
        let decoder_plan_words = span(route_index_words.end, counts.decoder_plans * 8)?;
        let decoder_family_words = span(decoder_plan_words.end, counts.decoder_families * 8)?;
        let decoder_weight_index_words =
            span(decoder_family_words.end, counts.decoder_weight_indices * 4)?;

        let w0 = u32_len(&self.immutable_weight_words)?;
        let genetic_weight_words = span(w0, upload.genetic_weights.len())?;
        let alpha_words = span(genetic_weight_words.end, upload.alpha.len())?;

        let m0 = u32_len(&self.mutable_state_words)?;
        let n = phenotype.neuron_count() as usize;
        let total = upload.genetic_weights.len();
        let recurrent = upload.source_indices.len();
        let decoder = upload.decoder_weight_indices.len();
        let activation_a_words = span(m0, n)?;
        let activation_b_words = span(activation_a_words.end, n)?;
        let accumulator_words = span(activation_b_words.end, n)?;
        let homeostasis_words = span(accumulator_words.end, n * 2)?;
        let lifetime_weight_words = span(homeostasis_words.end, total)?;
        let fast_weight_words = span(lifetime_weight_words.end, total)?;
        let recurrent_eligibility_words = span(fast_weight_words.end, recurrent)?;
        let decoder_eligibility_words = span(recurrent_eligibility_words.end, decoder)?;
        let encoded_input_words = span(decoder_eligibility_words.end, n)?;
        let candidate_logit_words = span(encoded_input_words.end, MAX_ACTION_CANDIDATES)?;
        let diagnostic_words = span(candidate_logit_words.end, 4)?;
        let selection_words = span(diagnostic_words.end, 12)?;
        let ranges = GpuSlotWordRanges {
            encoder_plan_words,
            encoder_assignment_words,
            encoder_target_offset_words,
            neuron_dynamics_words,
            projection_words,
            route_metadata_words,
            target_offset_words,
            source_index_words,
            route_index_words,
            decoder_plan_words,
            decoder_family_words,
            decoder_weight_index_words,
            genetic_weight_words,
            alpha_words,
            activation_a_words,
            activation_b_words,
            accumulator_words,
            homeostasis_words,
            lifetime_weight_words,
            fast_weight_words,
            recurrent_eligibility_words,
            decoder_eligibility_words,
            encoded_input_words,
            candidate_logit_words,
            diagnostic_words,
            selection_words,
        };

        validate_heap_word_end(&self.capacity, ranges.decoder_weight_index_words.end)?;
        validate_heap_word_end(&self.capacity, ranges.alpha_words.end)?;
        validate_heap_word_end(&self.capacity, ranges.selection_words.end)?;
        self.immutable_plan_words
            .try_reserve_exact(
                (ranges.decoder_weight_index_words.end - ranges.encoder_plan_words.start) as usize,
            )
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        self.immutable_weight_words
            .try_reserve_exact(
                (ranges.alpha_words.end - ranges.genetic_weight_words.start) as usize,
            )
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        self.mutable_state_words
            .try_reserve_exact(
                (ranges.selection_words.end - ranges.activation_a_words.start) as usize,
            )
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;

        let mut encoder_plan = upload.encoder_plans[0];
        encoder_plan.assignment_offset = ranges.encoder_assignment_words.start;
        encoder_plan.target_offsets_offset = ranges.encoder_target_offset_words.start;
        push_record(&mut self.immutable_plan_words, &encoder_plan);
        for row in &upload.encoder_assignments {
            push_record(&mut self.immutable_plan_words, row);
        }
        self.immutable_plan_words
            .extend_from_slice(&upload.encoder_target_offsets);
        for row in &upload.neuron_dynamics {
            push_record(&mut self.immutable_plan_words, row);
        }
        for row in &upload.projections {
            push_record(&mut self.immutable_plan_words, row);
        }
        for row in &upload.route_metadata {
            push_record(&mut self.immutable_plan_words, row);
        }
        self.immutable_plan_words
            .extend_from_slice(&upload.target_offsets);
        self.immutable_plan_words
            .extend_from_slice(&upload.source_indices);
        self.immutable_plan_words
            .extend_from_slice(&upload.route_indices);
        let mut decoder_plan = upload.decoder_plans[0];
        decoder_plan.family_offset = ranges.decoder_family_words.start;
        push_record(&mut self.immutable_plan_words, &decoder_plan);
        for row in &upload.decoder_families {
            let mut relocated = *row;
            let local = (row.weight_index_start - upload.decoder_weight_index_word_base) / 4;
            relocated.weight_index_start = ranges.decoder_weight_index_words.start + local * 4;
            push_record(&mut self.immutable_plan_words, &relocated);
        }
        for row in &upload.decoder_weight_indices {
            push_record(&mut self.immutable_plan_words, row);
        }
        self.immutable_weight_words
            .extend(upload.genetic_weights.iter().map(|v| v.to_bits()));
        self.immutable_weight_words
            .extend(upload.alpha.iter().map(|v| v.to_bits()));
        self.mutable_state_words
            .resize(ranges.selection_words.end as usize, 0);

        let record = GpuBrainSlotRecord {
            // This field binds the executable heap ordering to the capacity's GPU layout ABI.
            schema_version: u32::from(upload.gpu_layout_version),
            class_id: upload.class_id,
            slot,
            slot_generation: generation,
            neuron_count: upload.neuron_count,
            microstep_count: upload.microstep_count,
            synapse_count: total as u32,
            recurrent_synapse_count: recurrent as u32,
            encoder_plan_offset: ranges.encoder_plan_words.start,
            neuron_dynamics_offset: ranges.neuron_dynamics_words.start,
            projection_offset: ranges.projection_words.start,
            route_metadata_offset: ranges.route_metadata_words.start,
            target_offsets_offset: ranges.target_offset_words.start,
            source_indices_offset: ranges.source_index_words.start,
            route_indices_offset: ranges.route_index_words.start,
            decoder_plan_offset: ranges.decoder_plan_words.start,
            decoder_family_offset: ranges.decoder_family_words.start,
            decoder_weight_indices_offset: ranges.decoder_weight_index_words.start,
            genetic_weight_offset: ranges.genetic_weight_words.start,
            alpha_offset: ranges.alpha_words.start,
            activation_a_offset: ranges.activation_a_words.start,
            activation_b_offset: ranges.activation_b_words.start,
            accumulator_offset: ranges.accumulator_words.start,
            lifetime_weight_offset: ranges.lifetime_weight_words.start,
            fast_weight_offset: ranges.fast_weight_words.start,
            recurrent_eligibility_offset: ranges.recurrent_eligibility_words.start,
            decoder_eligibility_offset: ranges.decoder_eligibility_words.start,
            encoded_input_offset: ranges.encoded_input_words.start,
            candidate_logit_offset: ranges.candidate_logit_words.start,
            diagnostic_offset: ranges.diagnostic_words.start,
            selection_offset: ranges.selection_words.start,
            neuron_homeostasis_offset: ranges.homeostasis_words.start,
            extension_record_offset: GPU_NO_EXTENSION_SENTINEL,
            reserved: [0; 3],
        };
        record.validate_slice_a()?;
        let brain = GpuBrainSlot {
            record,
            identity: upload.identity,
            counts,
            ranges,
            brain_slot_index: slot,
            bucket_ownership_token: self.bucket_ownership_token,
        };
        self.brain_slot_records[slot as usize] = record;
        self.phenotype_identities[slot as usize] = upload.identity;
        self.slots[slot as usize] = Some(brain.clone());
        Ok(brain)
    }

    pub fn immutable_plan_words(&self) -> &[u32] {
        &self.immutable_plan_words
    }
    pub fn brain_slot_records(&self) -> &[GpuBrainSlotRecord] {
        &self.brain_slot_records
    }
    pub fn phenotype_identities(&self) -> &[GpuPhenotypeIdentityRecord] {
        &self.phenotype_identities
    }
    pub fn immutable_weight_words(&self) -> &[u32] {
        &self.immutable_weight_words
    }
    pub fn mutable_state_words(&self) -> &[u32] {
        &self.mutable_state_words
    }
    pub fn fast_weights<'a>(
        &'a self,
        slot: &GpuBrainSlot,
    ) -> Result<&'a [f32], GpuClosedLoopError> {
        self.f32_slice(slot, &slot.ranges.fast_weight_words)
    }
    pub fn fast_weights_mut<'a>(
        &'a mut self,
        slot: &GpuBrainSlot,
    ) -> Result<&'a mut [f32], GpuClosedLoopError> {
        self.f32_slice_mut(slot, &slot.ranges.fast_weight_words)
    }
    pub fn activation_a<'a>(
        &'a self,
        slot: &GpuBrainSlot,
    ) -> Result<&'a [f32], GpuClosedLoopError> {
        self.f32_slice(slot, &slot.ranges.activation_a_words)
    }
    pub fn activation_a_mut<'a>(
        &'a mut self,
        slot: &GpuBrainSlot,
    ) -> Result<&'a mut [f32], GpuClosedLoopError> {
        self.f32_slice_mut(slot, &slot.ranges.activation_a_words)
    }
    pub fn validate(&self) -> Result<(), GpuClosedLoopError> {
        for brain in self.slots.iter().flatten() {
            self.check_slot(brain)?;
            brain.record.validate_slice_a()?;
            if brain.record.schema_version
                != u32::from(self.capacity.execution().gpu_layout_version())
            {
                return Err(GpuClosedLoopError::LayoutMismatch);
            }
            if self.brain_slot_records[brain.brain_slot_index as usize] != brain.record
                || self.phenotype_identities[brain.brain_slot_index as usize] != brain.identity
            {
                return Err(GpuClosedLoopError::MalformedUpload);
            }
        }
        Ok(())
    }
    pub(crate) const fn ownership_token(&self) -> u64 {
        self.bucket_ownership_token
    }
    pub(crate) const fn capacity(&self) -> &BrainCapacityClass {
        &self.capacity
    }
    pub(crate) fn validate_slot_handle(
        &self,
        slot: &GpuBrainSlot,
    ) -> Result<(), GpuClosedLoopError> {
        self.check_slot(slot)
    }
    fn check_slot(&self, slot: &GpuBrainSlot) -> Result<(), GpuClosedLoopError> {
        let stored = self
            .slots
            .get(slot.brain_slot_index as usize)
            .and_then(Option::as_ref)
            .ok_or(GpuClosedLoopError::StaleOrForeignHandle)?;
        if slot.bucket_ownership_token != self.bucket_ownership_token
            || stored != slot
            || stored.record.class_id != self.capacity.id().raw() as u32
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        Ok(())
    }
    fn f32_slice<'a>(
        &'a self,
        slot: &GpuBrainSlot,
        range: &Range<u32>,
    ) -> Result<&'a [f32], GpuClosedLoopError> {
        self.check_slot(slot)?;
        Ok(bytemuck::cast_slice(
            &self.mutable_state_words[range.start as usize..range.end as usize],
        ))
    }
    fn f32_slice_mut<'a>(
        &'a mut self,
        slot: &GpuBrainSlot,
        range: &Range<u32>,
    ) -> Result<&'a mut [f32], GpuClosedLoopError> {
        self.check_slot(slot)?;
        Ok(bytemuck::cast_slice_mut(
            &mut self.mutable_state_words[range.start as usize..range.end as usize],
        ))
    }
}

fn span(start: u32, len: usize) -> Result<Range<u32>, GpuClosedLoopError> {
    let len = u32::try_from(len).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
    Ok(start
        ..start
            .checked_add(len)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?)
}
fn u32_len<T>(v: &[T]) -> Result<u32, GpuClosedLoopError> {
    u32::try_from(v.len()).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)
}

fn validate_fixed_slot_heaps(
    capacity: &BrainCapacityClass,
    slot_count: usize,
) -> Result<(), GpuClosedLoopError> {
    let limit = capacity.execution().required_max_buffer_size().min(
        capacity
            .execution()
            .required_max_storage_buffer_binding_size(),
    );
    for record_size in [
        std::mem::size_of::<GpuBrainSlotRecord>(),
        std::mem::size_of::<GpuPhenotypeIdentityRecord>(),
    ] {
        let bytes = (slot_count as u64)
            .checked_mul(record_size as u64)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if bytes > limit {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
    }
    Ok(())
}

fn validate_heap_word_end(
    capacity: &BrainCapacityClass,
    word_end: u32,
) -> Result<(), GpuClosedLoopError> {
    let bytes = u64::from(word_end)
        .checked_mul(4)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    let limit = capacity.execution().required_max_buffer_size().min(
        capacity
            .execution()
            .required_max_storage_buffer_binding_size(),
    );
    if bytes > limit {
        return Err(GpuClosedLoopError::CapacityExceeded);
    }
    Ok(())
}
fn push_record<T: bytemuck::Pod>(words: &mut Vec<u32>, record: &T) {
    words.extend_from_slice(bytemuck::cast_slice(std::slice::from_ref(record)));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuBufferAccess {
    ReadOnly,
    ReadWrite,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuClassBucketBufferRole {
    BrainSlots,
    PhenotypeIdentities,
    ImmutablePlanWords,
    ImmutableWeightWords,
    DispatchHeaderWords,
    FramePayloadWords,
    MutableStateWords,
    UploadStaging,
    CompactReadback,
}
impl GpuClassBucketBufferRole {
    pub const fn is_staging_or_readback(self) -> bool {
        matches!(self, Self::UploadStaging | Self::CompactReadback)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpuClassBucketBufferManifestEntry {
    pub group: u32,
    pub binding: u32,
    pub role: GpuClassBucketBufferRole,
    pub access: GpuBufferAccess,
    pub neural_pipeline_bindable: bool,
    pub minimum_binding_size_bytes: u64,
}

/// Owns the seven fixed neural heap buffers. Staging/readback resources are auxiliary only.
pub struct GpuClassBucketBuffers {
    brain_slots: wgpu::Buffer,
    phenotype_identities: wgpu::Buffer,
    immutable_plan_words: wgpu::Buffer,
    immutable_weight_words: wgpu::Buffer,
    dispatch_header_words: wgpu::Buffer,
    frame_payload_words: wgpu::Buffer,
    mutable_state_words: wgpu::Buffer,
    upload_staging: wgpu::Buffer,
    compact_readback: wgpu::Buffer,
    bucket_ownership_token: u64,
    max_neurons: u32,
    dispatch_capacity_words: usize,
    frame_payload_capacity_words: usize,
    buffer_set_token: u64,
}
impl GpuClassBucketBuffers {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plan: &GpuClassBucketPlan,
        brain_slots: wgpu::Buffer,
        phenotype_identities: wgpu::Buffer,
        immutable_plan_words: wgpu::Buffer,
        immutable_weight_words: wgpu::Buffer,
        dispatch_header_words: wgpu::Buffer,
        frame_payload_words: wgpu::Buffer,
        mutable_state_words: wgpu::Buffer,
        upload_staging: wgpu::Buffer,
        compact_readback: wgpu::Buffer,
    ) -> Result<Self, GpuClosedLoopError> {
        plan.validate()?;
        let dispatch_capacity_words = usize::try_from(dispatch_header_words.size() / 4)
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        let frame_payload_capacity_words = usize::try_from(frame_payload_words.size() / 4)
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        let buffer_set_token = NEXT_BUFFER_SET_TOKEN
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        let buffers = Self {
            brain_slots,
            phenotype_identities,
            immutable_plan_words,
            immutable_weight_words,
            dispatch_header_words,
            frame_payload_words,
            mutable_state_words,
            upload_staging,
            compact_readback,
            bucket_ownership_token: plan.ownership_token(),
            max_neurons: plan.capacity().execution().max_neurons(),
            dispatch_capacity_words,
            frame_payload_capacity_words,
            buffer_set_token,
        };
        for (buffer, manifest) in buffers
            .neural_buffers()
            .into_iter()
            .zip(Self::neural_binding_manifest())
        {
            if buffer.size() < manifest.minimum_binding_size_bytes {
                return Err(GpuClosedLoopError::CapacityExceeded);
            }
        }
        Ok(buffers)
    }
    pub const fn neural_binding_manifest() -> [GpuClassBucketBufferManifestEntry; 7] {
        use GpuBufferAccess::*;
        use GpuClassBucketBufferRole::*;
        [
            GpuClassBucketBufferManifestEntry {
                group: 0,
                binding: 0,
                role: BrainSlots,
                access: ReadOnly,
                neural_pipeline_bindable: true,
                minimum_binding_size_bytes: 144,
            },
            GpuClassBucketBufferManifestEntry {
                group: 0,
                binding: 1,
                role: PhenotypeIdentities,
                access: ReadOnly,
                neural_pipeline_bindable: true,
                minimum_binding_size_bytes: 32,
            },
            GpuClassBucketBufferManifestEntry {
                group: 0,
                binding: 2,
                role: ImmutablePlanWords,
                access: ReadOnly,
                neural_pipeline_bindable: true,
                minimum_binding_size_bytes: 4,
            },
            GpuClassBucketBufferManifestEntry {
                group: 0,
                binding: 3,
                role: ImmutableWeightWords,
                access: ReadOnly,
                neural_pipeline_bindable: true,
                minimum_binding_size_bytes: 4,
            },
            GpuClassBucketBufferManifestEntry {
                group: 0,
                binding: 4,
                role: DispatchHeaderWords,
                access: ReadOnly,
                neural_pipeline_bindable: true,
                minimum_binding_size_bytes: 4,
            },
            GpuClassBucketBufferManifestEntry {
                group: 0,
                binding: 5,
                role: FramePayloadWords,
                access: ReadOnly,
                neural_pipeline_bindable: true,
                minimum_binding_size_bytes: 4,
            },
            GpuClassBucketBufferManifestEntry {
                group: 0,
                binding: 6,
                role: MutableStateWords,
                access: ReadWrite,
                neural_pipeline_bindable: true,
                minimum_binding_size_bytes: 4,
            },
        ]
    }
    pub const fn auxiliary_buffer_manifest() -> [GpuClassBucketBufferManifestEntry; 2] {
        use GpuBufferAccess::*;
        use GpuClassBucketBufferRole::*;
        [
            GpuClassBucketBufferManifestEntry {
                group: u32::MAX,
                binding: u32::MAX,
                role: UploadStaging,
                access: ReadWrite,
                neural_pipeline_bindable: false,
                minimum_binding_size_bytes: 0,
            },
            GpuClassBucketBufferManifestEntry {
                group: u32::MAX,
                binding: u32::MAX,
                role: CompactReadback,
                access: ReadWrite,
                neural_pipeline_bindable: false,
                minimum_binding_size_bytes: 0,
            },
        ]
    }
    pub fn neural_buffers(&self) -> [&wgpu::Buffer; 7] {
        [
            &self.brain_slots,
            &self.phenotype_identities,
            &self.immutable_plan_words,
            &self.immutable_weight_words,
            &self.dispatch_header_words,
            &self.frame_payload_words,
            &self.mutable_state_words,
        ]
    }
    pub const fn upload_staging(&self) -> &wgpu::Buffer {
        &self.upload_staging
    }
    pub const fn compact_readback(&self) -> &wgpu::Buffer {
        &self.compact_readback
    }
    pub(crate) const fn ownership_token(&self) -> u64 {
        self.bucket_ownership_token
    }
    pub(crate) const fn max_neurons(&self) -> u32 {
        self.max_neurons
    }
    pub(crate) const fn dispatch_capacity_words(&self) -> usize {
        self.dispatch_capacity_words
    }
    pub(crate) const fn frame_payload_capacity_words(&self) -> usize {
        self.frame_payload_capacity_words
    }
    pub(crate) fn compact_readback_capacity_bytes(&self) -> u64 {
        self.compact_readback.size()
    }
    pub(crate) const fn buffer_set_token(&self) -> u64 {
        self.buffer_set_token
    }
}
