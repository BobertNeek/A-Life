use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::closed_loop_memory::GpuMemoryChannelPlan;
use alife_core::{
    BrainCapacityClass, BrainPhenotype, MAX_ACTION_CANDIDATES, MAX_REPLAY_CAPTURE_SYNAPSES,
};
use bytemuck::Zeroable;

use super::{
    GpuBrainSlotExtensionRecord, GpuBrainSlotRecord, GpuClosedLoopError,
    GpuDecoderEligibilityMetadata, GpuPhenotypeIdentityRecord, GpuPhenotypeUpload,
    GpuPlasticityReceptorRecord, GpuReplayCaptureIdentityRecord, GpuReplaySynapseSpanRecord,
    GpuSleepParameterRecord, GpuSlotLearningStateRecord, GpuSynapseLearningMetadata,
    GPU_CLOSED_LOOP_LAYOUT_VERSION, GPU_NO_EXTENSION_SENTINEL,
};

const GPU_PENDING_ELIGIBILITY_RECORD_WORDS: u32 = 36;

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
    pub memory_channel_plans: usize,
    pub memory_weight_indices: usize,
    pub plasticity_receptors: usize,
    pub synapse_learning_metadata: usize,
    pub decoder_eligibility_metadata: usize,
    pub replay_capture_synapses: usize,
    pub sleep_parameters: usize,
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
    pub memory_channel_plan_words: Range<u32>,
    pub memory_weight_index_words: Range<u32>,
    pub receptor_words: Range<u32>,
    pub synapse_learning_metadata_words: Range<u32>,
    pub decoder_eligibility_metadata_words: Range<u32>,
    pub replay_plan_identity_words: Range<u32>,
    pub sleep_parameter_words: Range<u32>,
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
    pub lifetime_weight_bank_1_words: Range<u32>,
    pub fast_weight_bank_1_words: Range<u32>,
    pub recurrent_eligibility_bank_1_words: Range<u32>,
    pub decoder_eligibility_bank_1_words: Range<u32>,
    pub encoded_input_words: Range<u32>,
    pub candidate_logit_words: Range<u32>,
    pub diagnostic_words: Range<u32>,
    pub selection_words: Range<u32>,
    pub extension_words: Range<u32>,
    pub learning_state_words: Range<u32>,
    pub pending_eligibility_words: Range<u32>,
    pub replay_event_words: Range<u32>,
    pub replay_sample_words: Range<u32>,
    pub replay_span_words: Range<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBrainSlot {
    record: GpuBrainSlotRecord,
    identity: GpuPhenotypeIdentityRecord,
    counts: GpuTypedCounts,
    ranges: GpuSlotWordRanges,
    decoder_input_stride: u32,
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
    pub const fn decoder_input_stride(&self) -> u32 {
        self.decoder_input_stride
    }
    pub const fn brain_slot_index(&self) -> u32 {
        self.brain_slot_index
    }

    pub(crate) fn rebind_bucket_ownership(&mut self, bucket_ownership_token: u64) {
        self.bucket_ownership_token = bucket_ownership_token;
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
    pub fn for_phenotype(phenotype: &BrainPhenotype) -> Result<Self, GpuClosedLoopError> {
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id())
            .map_err(|_| GpuClosedLoopError::LayoutMismatch)?;
        phenotype
            .validate_against(&capacity)
            .map_err(|_| GpuClosedLoopError::LayoutMismatch)?;
        let mut plan = Self::new(capacity, 1)?;
        plan.insert_phenotype(0, 1, phenotype)?;
        Ok(plan)
    }

    pub fn slot_allocation_receipt(
        &self,
    ) -> Result<crate::GpuSlotAllocationReceipt, GpuClosedLoopError> {
        GpuFixedClassArenaPlan::new(self.capacity, 1, u64::MAX)?.slot_allocation_receipt()
    }

    pub fn validate_adapter(
        phenotype: &BrainPhenotype,
        runtime: &crate::GpuRuntimeBudget,
    ) -> Result<(), GpuClosedLoopError> {
        let capacity = BrainCapacityClass::production_for_id(phenotype.brain_class_id())
            .map_err(|_| GpuClosedLoopError::LayoutMismatch)?;
        phenotype
            .validate_against(&capacity)
            .map_err(|_| GpuClosedLoopError::LayoutMismatch)?;
        runtime
            .validate_for(capacity.execution())
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)
    }

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
            memory_channel_plans: upload.memory_channel_plans.len(),
            memory_weight_indices: upload.memory_weight_indices.len(),
            plasticity_receptors: upload.plasticity_receptors.len(),
            synapse_learning_metadata: upload.synapse_learning_metadata.len(),
            decoder_eligibility_metadata: upload.decoder_eligibility_metadata.len(),
            replay_capture_synapses: upload.replay_capture_local_synapse_ids.len(),
            sleep_parameters: upload.sleep_parameters.len(),
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
        let memory_channel_plan_words = span(
            decoder_weight_index_words.end,
            counts.memory_channel_plans * 8,
        )?;
        let memory_weight_index_words =
            span(memory_channel_plan_words.end, counts.memory_weight_indices)?;
        let receptor_words = span(
            memory_weight_index_words.end,
            counts.plasticity_receptors * 8,
        )?;
        let synapse_learning_metadata_words =
            span(receptor_words.end, counts.synapse_learning_metadata * 8)?;
        let decoder_eligibility_metadata_words = span(
            synapse_learning_metadata_words.end,
            counts.decoder_eligibility_metadata * 8,
        )?;
        let replay_plan_identity_words = span(decoder_eligibility_metadata_words.end, 8)?;
        let sleep_parameter_words = span(
            replay_plan_identity_words.end,
            counts.sleep_parameters * (std::mem::size_of::<GpuSleepParameterRecord>() / 4),
        )?;

        let w0 = u32_len(&self.immutable_weight_words)?;
        let genetic_weight_words = span(w0, upload.genetic_weights.len())?;
        let alpha_words = span(genetic_weight_words.end, upload.alpha.len())?;

        let m0 = u32_len(&self.mutable_state_words)?;
        let n = phenotype.neuron_count() as usize;
        let total = upload.genetic_weights.len();
        let recurrent = upload.source_indices.len();
        let decoder = upload.decoder_eligibility_metadata.len();
        let activation_a_words = span(m0, n)?;
        let activation_b_words = span(activation_a_words.end, n)?;
        let accumulator_words = span(activation_b_words.end, n)?;
        let homeostasis_words = span(accumulator_words.end, n * 2)?;
        let lifetime_weight_words = span(homeostasis_words.end, total)?;
        let fast_weight_words = span(lifetime_weight_words.end, total)?;
        let recurrent_eligibility_words = span(fast_weight_words.end, recurrent)?;
        let decoder_eligibility_words = span(recurrent_eligibility_words.end, decoder)?;
        let lifetime_weight_bank_1_words = span(decoder_eligibility_words.end, total)?;
        let fast_weight_bank_1_words = span(lifetime_weight_bank_1_words.end, total)?;
        let recurrent_eligibility_bank_1_words = span(fast_weight_bank_1_words.end, recurrent)?;
        let decoder_eligibility_bank_1_words =
            span(recurrent_eligibility_bank_1_words.end, decoder)?;
        let encoded_input_words = span(decoder_eligibility_bank_1_words.end, n)?;
        let candidate_logit_words = span(encoded_input_words.end, MAX_ACTION_CANDIDATES)?;
        let diagnostic_words = span(candidate_logit_words.end, 4)?;
        let selection_words = span(diagnostic_words.end, 12)?;
        let extension_words = span(selection_words.end, 20)?;
        let learning_state_words = span(extension_words.end, 24)?;
        let pending_eligibility_words = span(
            learning_state_words.end,
            GPU_PENDING_ELIGIBILITY_RECORD_WORDS as usize,
        )?;
        let replay_event_words = span(
            pending_eligibility_words.end,
            phenotype.replay_capture_plan().event_capacity() as usize * 24,
        )?;
        let replay_sample_words = span(
            replay_event_words.end,
            phenotype.replay_capture_plan().sample_capacity() as usize,
        )?;
        let replay_span_words = span(
            replay_sample_words.end,
            phenotype.replay_capture_plan().global_synapse_ids().len() * 4,
        )?;
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
            memory_channel_plan_words,
            memory_weight_index_words,
            receptor_words,
            synapse_learning_metadata_words,
            decoder_eligibility_metadata_words,
            replay_plan_identity_words,
            sleep_parameter_words,
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
            lifetime_weight_bank_1_words,
            fast_weight_bank_1_words,
            recurrent_eligibility_bank_1_words,
            decoder_eligibility_bank_1_words,
            encoded_input_words,
            candidate_logit_words,
            diagnostic_words,
            selection_words,
            extension_words,
            learning_state_words,
            pending_eligibility_words,
            replay_event_words,
            replay_sample_words,
            replay_span_words,
        };

        validate_heap_word_end(&self.capacity, ranges.sleep_parameter_words.end)?;
        validate_heap_word_end(&self.capacity, ranges.alpha_words.end)?;
        validate_heap_word_end(&self.capacity, ranges.replay_span_words.end)?;
        self.immutable_plan_words
            .try_reserve_exact(
                (ranges.sleep_parameter_words.end - ranges.encoder_plan_words.start) as usize,
            )
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        self.immutable_weight_words
            .try_reserve_exact(
                (ranges.alpha_words.end - ranges.genetic_weight_words.start) as usize,
            )
            .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        self.mutable_state_words
            .try_reserve_exact(
                (ranges.replay_span_words.end - ranges.activation_a_words.start) as usize,
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
        for row in &upload.memory_channel_plans {
            push_record(&mut self.immutable_plan_words, row);
        }
        self.immutable_plan_words
            .extend_from_slice(&upload.memory_weight_indices);
        for row in &upload.plasticity_receptors {
            push_record(&mut self.immutable_plan_words, row);
        }
        for row in &upload.synapse_learning_metadata {
            push_record(&mut self.immutable_plan_words, row);
        }
        for row in &upload.decoder_eligibility_metadata {
            push_record(&mut self.immutable_plan_words, row);
        }
        push_record(
            &mut self.immutable_plan_words,
            &upload.replay_capture_identity,
        );
        for row in &upload.sleep_parameters {
            push_record(&mut self.immutable_plan_words, row);
        }
        self.immutable_weight_words
            .extend(upload.genetic_weights.iter().map(|v| v.to_bits()));
        self.immutable_weight_words
            .extend(upload.alpha.iter().map(|v| v.to_bits()));
        self.mutable_state_words
            .resize(ranges.replay_span_words.end as usize, 0);
        let extension = make_slot_extension(&ranges, &counts, recurrent as u32)?;
        store_pod_at(
            &mut self.mutable_state_words,
            0,
            ranges.extension_words.start,
            &extension,
        )?;
        let learning_state = make_learning_state(
            &ranges,
            phenotype.replay_capture_plan().event_capacity(),
            phenotype.replay_capture_plan().sample_capacity(),
            counts.replay_capture_synapses as u32,
        );
        store_pod_at(
            &mut self.mutable_state_words,
            0,
            ranges.learning_state_words.start,
            &learning_state,
        )?;
        let replay_spans = make_replay_spans(
            &upload.replay_capture_local_synapse_ids,
            phenotype.replay_capture_plan().event_capacity(),
        )?;
        store_pod_slice_at(
            &mut self.mutable_state_words,
            0,
            ranges.replay_span_words.start,
            &replay_spans,
        )?;

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
            extension_record_offset: ranges.extension_words.start,
            reserved: [0; 3],
        };
        record.validate_slice_a()?;
        let brain = GpuBrainSlot {
            record,
            identity: upload.identity,
            counts,
            ranges,
            decoder_input_stride: upload.decoder_plans[0].flattened_input_lane_count,
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
            validate_learning_slot_layout(
                &brain.record,
                &brain.counts,
                &brain.ranges,
                &self.immutable_plan_words,
                0,
                &self.immutable_weight_words,
                0,
                &self.mutable_state_words,
                0,
            )?;
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

fn make_slot_extension(
    ranges: &GpuSlotWordRanges,
    counts: &GpuTypedCounts,
    recurrent_synapse_count: u32,
) -> Result<GpuBrainSlotExtensionRecord, GpuClosedLoopError> {
    Ok(GpuBrainSlotExtensionRecord {
        schema_version: GPU_CLOSED_LOOP_LAYOUT_VERSION,
        projection_count: u32::try_from(counts.projections)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?,
        decoder_synapse_local_start: recurrent_synapse_count,
        decoder_synapse_count: u32::try_from(counts.decoder_eligibility_metadata)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?,
        receptor_offset: ranges.receptor_words.start,
        decoder_input_plan_offset: ranges.decoder_plan_words.start,
        decoder_metadata_offset: ranges.decoder_eligibility_metadata_words.start,
        synapse_metadata_offset: ranges.synapse_learning_metadata_words.start,
        recurrent_eligibility_bank_1_offset: ranges.recurrent_eligibility_bank_1_words.start,
        decoder_eligibility_bank_1_offset: ranges.decoder_eligibility_bank_1_words.start,
        fast_bank_1_offset: ranges.fast_weight_bank_1_words.start,
        lifetime_bank_1_offset: ranges.lifetime_weight_bank_1_words.start,
        sleep_parameter_offset: ranges.sleep_parameter_words.start,
        memory_plan_offset: if counts.memory_channel_plans == 1 {
            ranges.memory_channel_plan_words.start
        } else {
            GPU_NO_EXTENSION_SENTINEL
        },
        memory_weight_map_offset: if counts.memory_weight_indices > 0 {
            ranges.memory_weight_index_words.start
        } else {
            GPU_NO_EXTENSION_SENTINEL
        },
        learning_state_offset: ranges.learning_state_words.start,
        pending_eligibility_offset: ranges.pending_eligibility_words.start,
        replay_plan_identity_offset: ranges.replay_plan_identity_words.start,
        reserved0: 0,
        reserved1: 0,
    })
}

fn make_learning_state(
    ranges: &GpuSlotWordRanges,
    replay_event_capacity: u32,
    replay_sample_capacity: u32,
    replay_span_count: u32,
) -> GpuSlotLearningStateRecord {
    GpuSlotLearningStateRecord {
        schema_version: u32::from(alife_core::SchemaVersions::CURRENT.learning.raw()),
        active_weight_bank: 0,
        active_eligibility_bank: 0,
        pending_valid: 0,
        active_weight_generation_lo: 1,
        active_weight_generation_hi: 0,
        active_eligibility_generation_lo: 1,
        active_eligibility_generation_hi: 0,
        inactive_eligibility_generation_lo: 0,
        inactive_eligibility_generation_hi: 0,
        replay_generation_lo: 1,
        replay_generation_hi: 0,
        replay_cursor: 0,
        replay_event_count: 0,
        replay_event_capacity,
        replay_sample_capacity,
        replay_span_count,
        replay_event_rows_offset: ranges.replay_event_words.start,
        replay_sample_offset: ranges.replay_sample_words.start,
        replay_span_offset: ranges.replay_span_words.start,
        replay_plan_identity_offset: ranges.replay_plan_identity_words.start,
        pending_eligibility_offset: ranges.pending_eligibility_words.start,
        transaction_generation_lo: 1,
        transaction_generation_hi: 0,
    }
}

fn make_replay_spans(
    local_synapse_ids: &[u32],
    event_capacity: u32,
) -> Result<Vec<GpuReplaySynapseSpanRecord>, GpuClosedLoopError> {
    local_synapse_ids
        .iter()
        .enumerate()
        .map(|(index, local_synapse_id)| {
            let index = u32::try_from(index).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
            Ok(GpuReplaySynapseSpanRecord {
                local_synapse_id: *local_synapse_id,
                sample_start: index
                    .checked_mul(event_capacity)
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
                sample_count: 0,
                reserved: 0,
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn validate_learning_slot_layout(
    record: &GpuBrainSlotRecord,
    counts: &GpuTypedCounts,
    ranges: &GpuSlotWordRanges,
    immutable_plan_words: &[u32],
    immutable_plan_base: u32,
    immutable_weight_words: &[u32],
    immutable_weight_base: u32,
    mutable_state_words: &[u32],
    mutable_state_base: u32,
) -> Result<(), GpuClosedLoopError> {
    let synapse_count = usize::try_from(record.synapse_count)
        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
    let recurrent_count = usize::try_from(record.recurrent_synapse_count)
        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
    let decoder_count = synapse_count
        .checked_sub(recurrent_count)
        .ok_or(GpuClosedLoopError::MalformedUpload)?;
    if counts.synapse_learning_metadata != synapse_count
        || counts.decoder_eligibility_metadata != decoder_count
        || counts.plasticity_receptors == 0
        || counts.memory_channel_plans != 1
        || counts.memory_weight_indices == 0
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }

    validate_active_range(
        immutable_weight_words,
        immutable_weight_base,
        &ranges.genetic_weight_words,
        synapse_count,
    )?;
    validate_active_range(
        immutable_weight_words,
        immutable_weight_base,
        &ranges.alpha_words,
        synapse_count,
    )?;
    for (range, active_words) in [
        (
            &ranges.memory_channel_plan_words,
            counts.memory_channel_plans * 8,
        ),
        (
            &ranges.memory_weight_index_words,
            counts.memory_weight_indices,
        ),
        (&ranges.receptor_words, counts.plasticity_receptors * 8),
        (
            &ranges.synapse_learning_metadata_words,
            counts.synapse_learning_metadata * 8,
        ),
        (
            &ranges.decoder_eligibility_metadata_words,
            counts.decoder_eligibility_metadata * 8,
        ),
        (&ranges.replay_plan_identity_words, 8),
        (
            &ranges.sleep_parameter_words,
            counts.sleep_parameters * (std::mem::size_of::<GpuSleepParameterRecord>() / 4),
        ),
    ] {
        validate_active_range(
            immutable_plan_words,
            immutable_plan_base,
            range,
            active_words,
        )?;
    }
    for (range, active_words) in [
        (&ranges.lifetime_weight_words, synapse_count),
        (&ranges.fast_weight_words, synapse_count),
        (&ranges.recurrent_eligibility_words, recurrent_count),
        (&ranges.decoder_eligibility_words, decoder_count),
        (&ranges.lifetime_weight_bank_1_words, synapse_count),
        (&ranges.fast_weight_bank_1_words, synapse_count),
        (&ranges.recurrent_eligibility_bank_1_words, recurrent_count),
        (&ranges.decoder_eligibility_bank_1_words, decoder_count),
        (
            &ranges.extension_words,
            std::mem::size_of::<GpuBrainSlotExtensionRecord>() / 4,
        ),
        (
            &ranges.learning_state_words,
            std::mem::size_of::<GpuSlotLearningStateRecord>() / 4,
        ),
        (
            &ranges.pending_eligibility_words,
            GPU_PENDING_ELIGIBILITY_RECORD_WORDS as usize,
        ),
    ] {
        validate_active_range(mutable_state_words, mutable_state_base, range, active_words)?;
    }

    if record.genetic_weight_offset != ranges.genetic_weight_words.start
        || record.alpha_offset != ranges.alpha_words.start
        || record.lifetime_weight_offset != ranges.lifetime_weight_words.start
        || record.fast_weight_offset != ranges.fast_weight_words.start
        || record.recurrent_eligibility_offset != ranges.recurrent_eligibility_words.start
        || record.decoder_eligibility_offset != ranges.decoder_eligibility_words.start
        || record.extension_record_offset != ranges.extension_words.start
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }

    let extension: GpuBrainSlotExtensionRecord = read_pod_at(
        mutable_state_words,
        mutable_state_base,
        ranges.extension_words.start,
    )?;
    if extension.schema_version != GPU_CLOSED_LOOP_LAYOUT_VERSION
        || extension.projection_count
            != u32::try_from(counts.projections)
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
        || extension.decoder_synapse_local_start != record.recurrent_synapse_count
        || extension.decoder_synapse_count
            != u32::try_from(decoder_count).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
        || extension.receptor_offset != ranges.receptor_words.start
        || extension.decoder_input_plan_offset != ranges.decoder_plan_words.start
        || extension.decoder_metadata_offset != ranges.decoder_eligibility_metadata_words.start
        || extension.synapse_metadata_offset != ranges.synapse_learning_metadata_words.start
        || extension.recurrent_eligibility_bank_1_offset
            != ranges.recurrent_eligibility_bank_1_words.start
        || extension.decoder_eligibility_bank_1_offset
            != ranges.decoder_eligibility_bank_1_words.start
        || extension.fast_bank_1_offset != ranges.fast_weight_bank_1_words.start
        || extension.lifetime_bank_1_offset != ranges.lifetime_weight_bank_1_words.start
        || extension.sleep_parameter_offset != ranges.sleep_parameter_words.start
        || extension.memory_plan_offset != ranges.memory_channel_plan_words.start
        || extension.memory_weight_map_offset != ranges.memory_weight_index_words.start
        || extension.learning_state_offset != ranges.learning_state_words.start
        || extension.pending_eligibility_offset != ranges.pending_eligibility_words.start
        || extension.replay_plan_identity_offset != ranges.replay_plan_identity_words.start
        || extension.reserved0 != 0
        || extension.reserved1 != 0
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }

    let state: GpuSlotLearningStateRecord = read_pod_at(
        mutable_state_words,
        mutable_state_base,
        extension.learning_state_offset,
    )?;
    let learning_schema = u32::from(alife_core::SchemaVersions::CURRENT.learning.raw());
    let active_weight_generation = join_u64(
        state.active_weight_generation_lo,
        state.active_weight_generation_hi,
    );
    let active_eligibility_generation = join_u64(
        state.active_eligibility_generation_lo,
        state.active_eligibility_generation_hi,
    );
    let replay_generation = join_u64(state.replay_generation_lo, state.replay_generation_hi);
    let transaction_generation = join_u64(
        state.transaction_generation_lo,
        state.transaction_generation_hi,
    );
    if state.schema_version != learning_schema
        || state.active_weight_bank > 1
        || state.active_eligibility_bank > 1
        || state.pending_valid > 1
        || active_weight_generation == 0
        || active_eligibility_generation == 0
        || replay_generation == 0
        || transaction_generation == 0
        || state.replay_event_capacity == 0
        || state.replay_span_count
            != u32::try_from(counts.replay_capture_synapses)
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
        || state.replay_span_count == 0
        || state.replay_sample_capacity
            != state
                .replay_event_capacity
                .checked_mul(state.replay_span_count)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?
        || state.replay_cursor >= state.replay_event_capacity
        || state.replay_event_count > state.replay_event_capacity
        || state.replay_event_rows_offset != ranges.replay_event_words.start
        || state.replay_sample_offset != ranges.replay_sample_words.start
        || state.replay_span_offset != ranges.replay_span_words.start
        || state.replay_plan_identity_offset != ranges.replay_plan_identity_words.start
        || state.pending_eligibility_offset != ranges.pending_eligibility_words.start
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }

    let event_words = usize::try_from(state.replay_event_capacity)
        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
        .checked_mul(std::mem::size_of::<super::GpuReplayEventRecord>() / 4)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    validate_active_range(
        mutable_state_words,
        mutable_state_base,
        &ranges.replay_event_words,
        event_words,
    )?;
    validate_active_range(
        mutable_state_words,
        mutable_state_base,
        &ranges.replay_sample_words,
        usize::try_from(state.replay_sample_capacity)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?,
    )?;
    validate_active_range(
        mutable_state_words,
        mutable_state_base,
        &ranges.replay_span_words,
        usize::try_from(state.replay_span_count)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
            .checked_mul(std::mem::size_of::<GpuReplaySynapseSpanRecord>() / 4)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
    )?;

    let receptors = read_pod_prefix::<GpuPlasticityReceptorRecord>(
        immutable_plan_words,
        immutable_plan_base,
        &ranges.receptor_words,
        counts.plasticity_receptors,
    )?;
    for receptor in &receptors {
        let finite = [
            receptor.eligibility_decay,
            receptor.learning_rate,
            receptor.sleep_replay_rate,
            receptor.normalization_rate,
            receptor.modulator_sign,
            receptor.fast_min,
            receptor.fast_max,
        ]
        .into_iter()
        .all(f32::is_finite);
        if !finite
            || !(0.0..=1.0).contains(&receptor.eligibility_decay)
            || !(0.0..=1.0).contains(&receptor.learning_rate)
            || !(0.0..=1.0).contains(&receptor.sleep_replay_rate)
            || !(0.0..=1.0).contains(&receptor.normalization_rate)
            || !matches!(receptor.modulator_sign, -1.0 | 1.0)
            || !(-8.0..=8.0).contains(&receptor.fast_min)
            || !(-8.0..=8.0).contains(&receptor.fast_max)
            || receptor.fast_min >= receptor.fast_max
            || receptor.reserved.to_bits() != 0
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
    }
    let disabled_receptor = receptors
        .first()
        .ok_or(GpuClosedLoopError::MalformedUpload)?;
    if disabled_receptor.learning_rate.to_bits() != 0
        || disabled_receptor.sleep_replay_rate.to_bits() != 0
        || disabled_receptor.normalization_rate.to_bits() != 0
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }

    let synapse_metadata = read_pod_prefix::<GpuSynapseLearningMetadata>(
        immutable_plan_words,
        immutable_plan_base,
        &ranges.synapse_learning_metadata_words,
        synapse_count,
    )?;
    for (index, row) in synapse_metadata.iter().enumerate() {
        let index = u32::try_from(index).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        let recurrent = index < record.recurrent_synapse_count;
        let expected_eligibility = if recurrent {
            index
        } else {
            index - record.recurrent_synapse_count
        };
        let expected_decoder_metadata = if recurrent {
            u32::MAX
        } else {
            expected_eligibility
        };
        if row.global_synapse_id != index
            || row.kind != if recurrent { 1 } else { 2 }
            || row.source_neuron >= record.neuron_count
            || row.target_neuron >= record.neuron_count
            || usize::try_from(row.receptor_index)
                .ok()
                .is_none_or(|value| value >= receptors.len())
            || row.eligibility_local_index != expected_eligibility
            || row.decoder_metadata_local_or_max != expected_decoder_metadata
            || row.reserved != 0
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
    }

    let decoder_metadata = read_pod_prefix::<GpuDecoderEligibilityMetadata>(
        immutable_plan_words,
        immutable_plan_base,
        &ranges.decoder_eligibility_metadata_words,
        decoder_count,
    )?;
    for (index, row) in decoder_metadata.iter().enumerate() {
        let local = u32::try_from(index).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        let global = record
            .recurrent_synapse_count
            .checked_add(local)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let synapse = synapse_metadata
            .get(global as usize)
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        if row.global_synapse_id != global
            || !(1..=3).contains(&row.decoder_head)
            || row.family >= 8
            || row.receptor_index != synapse.receptor_index
            || row.eligibility_local_index != local
            || row.reserved != 0
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
    }

    let memory_plan: GpuMemoryChannelPlan = read_pod_at(
        immutable_plan_words,
        immutable_plan_base,
        ranges.memory_channel_plan_words.start,
    )?;
    if memory_plan.schema_version != 1
        || memory_plan.target_latent_lane_start != 24
        || memory_plan.family_value_lane_start != 32
        || memory_plan.decoder_input_stride != 36
        || memory_plan.max_candidate_gain.to_bits() != 0.5_f32.to_bits()
        || memory_plan.memory_decoder_synapse_count as usize != counts.memory_weight_indices
        || counts.memory_weight_indices < 96
        || !counts.memory_weight_indices.is_multiple_of(8)
        || memory_plan.reserved != [0; 2]
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    let memory_map = validate_active_range(
        immutable_plan_words,
        immutable_plan_base,
        &ranges.memory_weight_index_words,
        counts.memory_weight_indices,
    )?;
    let rows_per_family = counts.memory_weight_indices / 8;
    let mut seen_memory_rows = vec![false; decoder_count];
    for (map_index, global_synapse_id) in memory_map.iter().copied().enumerate() {
        let decoder_local = global_synapse_id
            .checked_sub(record.recurrent_synapse_count)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        let expected_family = map_index / rows_per_family;
        let metadata = decoder_metadata
            .get(decoder_local)
            .ok_or(GpuClosedLoopError::MalformedUpload)?;
        if seen_memory_rows[decoder_local]
            || metadata.global_synapse_id != global_synapse_id
            || metadata.decoder_head != 2
            || metadata.family as usize != expected_family
            || !(24..36).contains(&metadata.input_lane)
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        seen_memory_rows[decoder_local] = true;
    }
    if decoder_metadata
        .iter()
        .enumerate()
        .any(|(index, metadata)| (metadata.decoder_head == 2) != seen_memory_rows[index])
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }

    let replay_identity: GpuReplayCaptureIdentityRecord = read_pod_at(
        immutable_plan_words,
        immutable_plan_base,
        ranges.replay_plan_identity_words.start,
    )?;
    if replay_identity
        .replay_capture_plan_digest
        .iter()
        .all(|word| *word == 0)
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }

    if counts.sleep_parameters != 1 {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    let sleep: GpuSleepParameterRecord = read_pod_at(
        immutable_plan_words,
        immutable_plan_base,
        ranges.sleep_parameter_words.start,
    )?;
    if sleep.schema_version
        != u32::from(
            alife_core::SchemaVersions::CURRENT
                .sleep_consolidation
                .raw(),
        )
        || ![
            sleep.staging_rate,
            sleep.weight_limit,
            sleep.fast_decay_rate,
        ]
        .into_iter()
        .all(f32::is_finite)
        || !(0.0..=1.0).contains(&sleep.staging_rate)
        || sleep.staging_rate == 0.0
        || !(0.0..=8.0).contains(&sleep.weight_limit)
        || sleep.weight_limit == 0.0
        || !(0.0..=1.0).contains(&sleep.fast_decay_rate)
        || sleep.eligibility_reset_policy != 1
        || sleep.replay_consume_policy != 1
        || sleep.reserved != [0; 2]
    {
        return Err(GpuClosedLoopError::MalformedUpload);
    }

    let replay_spans = read_pod_prefix::<GpuReplaySynapseSpanRecord>(
        mutable_state_words,
        mutable_state_base,
        &ranges.replay_span_words,
        counts.replay_capture_synapses,
    )?;
    let mut seen = vec![false; synapse_count];
    for (index, span) in replay_spans.iter().enumerate() {
        let local = usize::try_from(span.local_synapse_id)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        let expected_start = u32::try_from(index)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?
            .checked_mul(state.replay_event_capacity)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let receptor = synapse_metadata
            .get(local)
            .and_then(|metadata| receptors.get(metadata.receptor_index as usize));
        if local >= seen.len()
            || seen[local]
            || span.sample_start != expected_start
            || span.sample_count > state.replay_event_count
            || span.sample_count > state.replay_event_capacity
            || span.reserved != 0
            || receptor.is_none_or(|row| {
                row.learning_rate == 0.0
                    && row.sleep_replay_rate == 0.0
                    && row.normalization_rate == 0.0
            })
        {
            return Err(GpuClosedLoopError::MalformedUpload);
        }
        seen[local] = true;
    }
    Ok(())
}

fn validate_active_range<'a>(
    heap: &'a [u32],
    heap_base: u32,
    range: &Range<u32>,
    active_words: usize,
) -> Result<&'a [u32], GpuClosedLoopError> {
    if range.start > range.end || range.start < heap_base {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    let heap_words =
        u32::try_from(heap.len()).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
    let heap_end = heap_base
        .checked_add(heap_words)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    if range.end > heap_end {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    let local_start = usize::try_from(range.start - heap_base)
        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
    let allocated_words = usize::try_from(range.end - range.start)
        .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
    if allocated_words < active_words {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    let active_end = local_start
        .checked_add(active_words)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    let allocated_end = local_start
        .checked_add(allocated_words)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    let allocated = heap
        .get(local_start..allocated_end)
        .ok_or(GpuClosedLoopError::MalformedUpload)?;
    if allocated[active_words..].iter().any(|word| *word != 0) {
        return Err(GpuClosedLoopError::MalformedUpload);
    }
    heap.get(local_start..active_end)
        .ok_or(GpuClosedLoopError::MalformedUpload)
}

fn read_pod_at<T: bytemuck::Pod + Copy>(
    heap: &[u32],
    heap_base: u32,
    absolute_start: u32,
) -> Result<T, GpuClosedLoopError> {
    let words = std::mem::size_of::<T>() / 4;
    let range = absolute_start
        ..absolute_start
            .checked_add(u32::try_from(words).map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    let words = validate_active_range(heap, heap_base, &range, words)?;
    Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(words)))
}

fn read_pod_prefix<T: bytemuck::Pod + Copy>(
    heap: &[u32],
    heap_base: u32,
    range: &Range<u32>,
    count: usize,
) -> Result<Vec<T>, GpuClosedLoopError> {
    let words_per_record = std::mem::size_of::<T>() / 4;
    let active_words = count
        .checked_mul(words_per_record)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    let words = validate_active_range(heap, heap_base, range, active_words)?;
    words
        .chunks_exact(words_per_record)
        .map(|chunk| Ok(bytemuck::pod_read_unaligned(bytemuck::cast_slice(chunk))))
        .collect()
}

const fn join_u64(lo: u32, hi: u32) -> u64 {
    lo as u64 | ((hi as u64) << 32)
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
                access: ReadWrite,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GpuFixedArenaHeap {
    BrainSlots,
    PhenotypeIdentities,
    ImmutablePlanWords,
    ImmutableWeightWords,
    MutableStateWords,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GpuFixedSlotStrides {
    pub(crate) encoder_assignment_count: u32,
    pub(crate) projection_count: u32,
    pub(crate) route_count: u32,
    pub(crate) immutable_plan_words: u32,
    pub(crate) immutable_weight_words: u32,
    pub(crate) mutable_state_words: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GpuFixedSlotRanges {
    arena_ownership_token: u64,
    slot: u32,
    pub(crate) brain_slot_bytes: Range<u64>,
    pub(crate) identity_bytes: Range<u64>,
    pub(crate) immutable_plan_words: Range<u32>,
    pub(crate) immutable_weight_words: Range<u32>,
    pub(crate) mutable_state_words: Range<u32>,
    pub(crate) layout: GpuSlotWordRanges,
}

impl GpuFixedSlotRanges {
    #[allow(dead_code)]
    pub(crate) const fn slot(&self) -> u32 {
        self.slot
    }

    pub(crate) fn full_scrub_ranges(&self) -> [(GpuFixedArenaHeap, Range<u64>); 5] {
        [
            (GpuFixedArenaHeap::BrainSlots, self.brain_slot_bytes.clone()),
            (
                GpuFixedArenaHeap::PhenotypeIdentities,
                self.identity_bytes.clone(),
            ),
            (
                GpuFixedArenaHeap::ImmutablePlanWords,
                words_to_bytes(self.immutable_plan_words.clone())
                    .expect("validated fixed plan word range converts to bytes"),
            ),
            (
                GpuFixedArenaHeap::ImmutableWeightWords,
                words_to_bytes(self.immutable_weight_words.clone())
                    .expect("validated fixed weight word range converts to bytes"),
            ),
            (
                GpuFixedArenaHeap::MutableStateWords,
                words_to_bytes(self.mutable_state_words.clone())
                    .expect("validated fixed mutable word range converts to bytes"),
            ),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GpuFixedArenaBufferSizes {
    pub(crate) brain_slots: u64,
    pub(crate) phenotype_identities: u64,
    pub(crate) immutable_plan_words: u64,
    pub(crate) immutable_weight_words: u64,
    pub(crate) dispatch_header_words: u64,
    pub(crate) frame_payload_words: u64,
    pub(crate) mutable_state_words: u64,
    pub(crate) upload_staging: u64,
    pub(crate) compact_readback: u64,
    pub(crate) aggregate: u64,
}

#[derive(Debug)]
pub(crate) struct GpuFixedClassArenaPlan {
    capacity: BrainCapacityClass,
    slot_capacity: u32,
    strides: GpuFixedSlotStrides,
    relative_layout: GpuSlotWordRanges,
    sizes: GpuFixedArenaBufferSizes,
    arena_ownership_token: u64,
}

impl GpuFixedClassArenaPlan {
    pub(crate) fn new(
        capacity: BrainCapacityClass,
        slot_capacity: u32,
        aggregate_resident_ceiling_bytes: u64,
    ) -> Result<Self, GpuClosedLoopError> {
        capacity
            .validate_contract()
            .map_err(|_| GpuClosedLoopError::LayoutMismatch)?;
        if slot_capacity == 0 {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let execution = capacity.execution();
        if slot_capacity > execution.required_max_compute_workgroups_per_dimension() {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let n = execution.max_neurons();
        let total = execution.max_total_synapses();
        let recurrent = execution.max_recurrent_synapses();
        let candidate_decoder = execution.max_action_decoder_synapses();
        let decoder = candidate_decoder
            .checked_add(execution.max_memory_decoder_synapses())
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let tiles = execution.max_active_tiles();
        let candidates = u32::from(execution.max_candidates());
        let encoder_assignments = n
            .checked_mul(2)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;

        let mut cursor = 0_u32;
        let encoder_plan_words = span_u32(&mut cursor, 8)?;
        let encoder_assignment_words = span_u32(
            &mut cursor,
            encoder_assignments
                .checked_mul(8)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let encoder_target_offset_words = span_u32(
            &mut cursor,
            encoder_assignments
                .checked_add(1)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let neuron_dynamics_words = span_u32(
            &mut cursor,
            n.checked_mul(8)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let projection_words = span_u32(
            &mut cursor,
            tiles
                .checked_mul(8)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let route_metadata_words = span_u32(
            &mut cursor,
            tiles
                .checked_mul(12)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let target_offset_words = span_u32(
            &mut cursor,
            n.checked_add(1)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let source_index_words = span_u32(&mut cursor, recurrent)?;
        let route_index_words = span_u32(&mut cursor, recurrent)?;
        let decoder_plan_words = span_u32(&mut cursor, 8)?;
        let decoder_family_words = span_u32(&mut cursor, 8 * 8)?;
        let decoder_weight_index_words = span_u32(
            &mut cursor,
            candidate_decoder
                .checked_mul(4)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let memory_channel_plan_words = span_u32(&mut cursor, 8)?;
        let memory_weight_index_words =
            span_u32(&mut cursor, execution.max_memory_decoder_synapses())?;
        let receptor_words = span_u32(
            &mut cursor,
            tiles
                .checked_add(4)
                .and_then(|count| count.checked_mul(8))
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let synapse_learning_metadata_words = span_u32(
            &mut cursor,
            total
                .checked_mul(8)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let decoder_eligibility_metadata_words = span_u32(
            &mut cursor,
            decoder
                .checked_mul(8)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let replay_plan_identity_words = span_u32(&mut cursor, 8)?;
        let sleep_parameter_words = span_u32(
            &mut cursor,
            u32::try_from(std::mem::size_of::<GpuSleepParameterRecord>() / 4)
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let immutable_plan_stride =
            align_words(cursor, execution.storage_offset_alignment_bytes())?;

        cursor = 0;
        let genetic_weight_words = span_u32(&mut cursor, total)?;
        let alpha_words = span_u32(&mut cursor, total)?;
        let immutable_weight_stride =
            align_words(cursor, execution.storage_offset_alignment_bytes())?;

        cursor = 0;
        let activation_a_words = span_u32(&mut cursor, n)?;
        let activation_b_words = span_u32(&mut cursor, n)?;
        let accumulator_words = span_u32(&mut cursor, n)?;
        let homeostasis_words = span_u32(
            &mut cursor,
            n.checked_mul(2)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let lifetime_weight_words = span_u32(&mut cursor, total)?;
        let fast_weight_words = span_u32(&mut cursor, total)?;
        let recurrent_eligibility_words = span_u32(&mut cursor, recurrent)?;
        let decoder_eligibility_words = span_u32(&mut cursor, decoder)?;
        let lifetime_weight_bank_1_words = span_u32(&mut cursor, total)?;
        let fast_weight_bank_1_words = span_u32(&mut cursor, total)?;
        let recurrent_eligibility_bank_1_words = span_u32(&mut cursor, recurrent)?;
        let decoder_eligibility_bank_1_words = span_u32(&mut cursor, decoder)?;
        let encoded_input_words = span_u32(&mut cursor, n)?;
        let candidate_logit_words = span_u32(&mut cursor, candidates)?;
        let diagnostic_words = span_u32(&mut cursor, 4)?;
        let selection_words = span_u32(&mut cursor, 12)?;
        let extension_words = span_u32(&mut cursor, 20)?;
        let learning_state_words = span_u32(&mut cursor, 24)?;
        let pending_eligibility_words =
            span_u32(&mut cursor, GPU_PENDING_ELIGIBILITY_RECORD_WORDS)?;
        let replay_event_words = span_u32(
            &mut cursor,
            execution
                .max_replay_events()
                .checked_mul(24)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let replay_sample_words =
            span_u32(&mut cursor, execution.max_replay_eligibility_samples())?;
        let replay_span_words = span_u32(
            &mut cursor,
            MAX_REPLAY_CAPTURE_SYNAPSES
                .checked_mul(4)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let mutable_state_stride = align_words(cursor, execution.storage_offset_alignment_bytes())?;

        let relative_layout = GpuSlotWordRanges {
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
            memory_channel_plan_words,
            memory_weight_index_words,
            receptor_words,
            synapse_learning_metadata_words,
            decoder_eligibility_metadata_words,
            replay_plan_identity_words,
            sleep_parameter_words,
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
            lifetime_weight_bank_1_words,
            fast_weight_bank_1_words,
            recurrent_eligibility_bank_1_words,
            decoder_eligibility_bank_1_words,
            encoded_input_words,
            candidate_logit_words,
            diagnostic_words,
            selection_words,
            extension_words,
            learning_state_words,
            pending_eligibility_words,
            replay_event_words,
            replay_sample_words,
            replay_span_words,
        };
        let strides = GpuFixedSlotStrides {
            encoder_assignment_count: encoder_assignments,
            projection_count: tiles,
            route_count: tiles,
            immutable_plan_words: immutable_plan_stride,
            immutable_weight_words: immutable_weight_stride,
            mutable_state_words: mutable_state_stride,
        };
        let slot_count = u64::from(slot_capacity);
        let brain_slots =
            checked_mul_bytes(slot_count, std::mem::size_of::<GpuBrainSlotRecord>() as u64)?;
        let phenotype_identities = checked_mul_bytes(
            slot_count,
            std::mem::size_of::<GpuPhenotypeIdentityRecord>() as u64,
        )?;
        let immutable_plan_words = checked_word_buffer_bytes(immutable_plan_stride, slot_capacity)?;
        let immutable_weight_words =
            checked_word_buffer_bytes(immutable_weight_stride, slot_capacity)?;
        let mutable_state_words = checked_word_buffer_bytes(mutable_state_stride, slot_capacity)?;
        let dispatch_row_words = u64::try_from(crate::GPU_ACTIVE_DISPATCH_ROW_WORDS)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        let dispatch_header_words = checked_mul_bytes(
            slot_count,
            dispatch_row_words
                .checked_mul(4)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let frame_words_per_row = 77_u64
            .checked_add(
                u64::from(candidates)
                    .checked_mul(u64::from(execution.max_decoder_input_lanes()))
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
            )
            .and_then(|value| value.checked_add(u64::from(candidates) * 4))
            .and_then(|value| value.checked_add(crate::GPU_PENDING_ELIGIBILITY_WORDS as u64))
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let active_frame_payload_words = checked_mul_bytes(
            slot_count,
            frame_words_per_row
                .checked_mul(4)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        let sleep_frame_words = 44_u64
            .checked_add(
                u64::from(execution.max_replay_events())
                    .checked_mul(24)
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
            )
            .and_then(|value| value.checked_add(u64::from(MAX_REPLAY_CAPTURE_SYNAPSES) * 4))
            .and_then(|value| {
                value.checked_add(u64::from(execution.max_replay_eligibility_samples()))
            })
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let sleep_frame_payload_bytes = sleep_frame_words
            .checked_mul(4)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let frame_payload_words = active_frame_payload_words.max(sleep_frame_payload_bytes);
        let compact_readback = checked_mul_bytes(
            slot_count,
            crate::GPU_COMPACT_READBACK_CAPACITY_PER_ROW_BYTES as u64,
        )?;
        let upload_staging = immutable_plan_words
            .checked_div(slot_count)
            .and_then(|value| value.checked_add(immutable_weight_words / slot_count))
            .and_then(|value| {
                value.checked_add(
                    (std::mem::size_of::<GpuBrainSlotRecord>()
                        + std::mem::size_of::<GpuPhenotypeIdentityRecord>())
                        as u64,
                )
            })
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let mut aggregate = 0_u64;
        for bytes in [
            brain_slots,
            phenotype_identities,
            immutable_plan_words,
            immutable_weight_words,
            dispatch_header_words,
            frame_payload_words,
            mutable_state_words,
            upload_staging,
            compact_readback,
        ] {
            aggregate = aggregate
                .checked_add(bytes)
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        }
        let sizes = GpuFixedArenaBufferSizes {
            brain_slots,
            phenotype_identities,
            immutable_plan_words,
            immutable_weight_words,
            dispatch_header_words,
            frame_payload_words,
            mutable_state_words,
            upload_staging,
            compact_readback,
            aggregate,
        };
        let storage_limit = execution
            .required_max_buffer_size()
            .min(execution.required_max_storage_buffer_binding_size());
        if [
            sizes.brain_slots,
            sizes.phenotype_identities,
            sizes.immutable_plan_words,
            sizes.immutable_weight_words,
            sizes.dispatch_header_words,
            sizes.frame_payload_words,
            sizes.mutable_state_words,
        ]
        .into_iter()
        .any(|bytes| bytes == 0 || bytes > storage_limit)
            || sizes.aggregate > aggregate_resident_ceiling_bytes
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let arena_ownership_token = NEXT_BUCKET_OWNERSHIP_TOKEN
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        Ok(Self {
            capacity,
            slot_capacity,
            strides,
            relative_layout,
            sizes,
            arena_ownership_token,
        })
    }

    pub(crate) const fn capacity(&self) -> &BrainCapacityClass {
        &self.capacity
    }
    pub(crate) const fn slot_capacity(&self) -> u32 {
        self.slot_capacity
    }
    #[allow(dead_code)]
    pub(crate) const fn strides(&self) -> GpuFixedSlotStrides {
        self.strides
    }
    pub(crate) const fn buffer_sizes(&self) -> GpuFixedArenaBufferSizes {
        self.sizes
    }
    pub(crate) const fn aggregate_resident_bytes(&self) -> u64 {
        self.sizes.aggregate
    }
    pub(crate) fn slot_allocation_receipt(
        &self,
    ) -> Result<crate::GpuSlotAllocationReceipt, GpuClosedLoopError> {
        let layout = &self.relative_layout;
        let range_bytes = |range: &Range<u32>| -> Result<u64, GpuClosedLoopError> {
            u64::from(
                range
                    .end
                    .checked_sub(range.start)
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
            )
            .checked_mul(4)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)
        };
        let sum_ranges = |ranges: &[&Range<u32>]| -> Result<u64, GpuClosedLoopError> {
            ranges.iter().try_fold(0_u64, |sum, range| {
                sum.checked_add(range_bytes(range)?)
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)
            })
        };

        let immutable_topology_bytes = (std::mem::size_of::<GpuBrainSlotRecord>() as u64)
            .checked_add(std::mem::size_of::<GpuPhenotypeIdentityRecord>() as u64)
            .and_then(|value| value.checked_add(u64::from(layout.sleep_parameter_words.end) * 4))
            .and_then(|value| value.checked_add(u64::from(layout.alpha_words.end) * 4))
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let activation_bytes = sum_ranges(&[
            &layout.activation_a_words,
            &layout.activation_b_words,
            &layout.accumulator_words,
            &layout.encoded_input_words,
        ])?;
        let learning_bytes = sum_ranges(&[
            &layout.homeostasis_words,
            &layout.lifetime_weight_words,
            &layout.fast_weight_words,
            &layout.recurrent_eligibility_words,
            &layout.decoder_eligibility_words,
            &layout.lifetime_weight_bank_1_words,
            &layout.fast_weight_bank_1_words,
            &layout.recurrent_eligibility_bank_1_words,
            &layout.decoder_eligibility_bank_1_words,
            &layout.learning_state_words,
            &layout.pending_eligibility_words,
            &layout.replay_event_words,
            &layout.replay_sample_words,
            &layout.replay_span_words,
        ])?;
        let candidate_and_memory_bytes = range_bytes(&layout.candidate_logit_words)?;
        let dispatch_row_bytes = (crate::GPU_ACTIVE_DISPATCH_ROW_WORDS as u64)
            .checked_mul(4)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let compact_readback_bytes = crate::GPU_COMPACT_READBACK_CAPACITY_PER_ROW_BYTES as u64;
        let diagnostic_and_readback_bytes = sum_ranges(&[
            &layout.diagnostic_words,
            &layout.selection_words,
            &layout.extension_words,
        ])?
        .checked_add(dispatch_row_bytes)
        .and_then(|value| value.checked_add(compact_readback_bytes))
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let candidates = u64::from(self.capacity.execution().max_candidates());
        let staging_words = 77_u64
            .checked_add(
                candidates
                    .checked_mul(u64::from(
                        self.capacity.execution().max_decoder_input_lanes(),
                    ))
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
            )
            .and_then(|value| value.checked_add(candidates * 4))
            .and_then(|value| value.checked_add(crate::GPU_PENDING_ELIGIBILITY_WORDS as u64))
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let staging_bytes = staging_words
            .checked_mul(4)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let components = crate::GpuSlotComponentBytes {
            immutable_topology_bytes,
            activation_bytes,
            learning_bytes,
            candidate_and_memory_bytes,
            diagnostic_and_readback_bytes,
            staging_bytes,
        };
        let logical_slot_commit_bytes = components
            .checked_sum()
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let slot_count = u64::from(self.slot_capacity);
        let per_slot_physical_without_shared = (std::mem::size_of::<GpuBrainSlotRecord>() as u64)
            .checked_add(std::mem::size_of::<GpuPhenotypeIdentityRecord>() as u64)
            .and_then(|value| value.checked_add(u64::from(self.strides.immutable_plan_words) * 4))
            .and_then(|value| value.checked_add(u64::from(self.strides.immutable_weight_words) * 4))
            .and_then(|value| value.checked_add(u64::from(self.strides.mutable_state_words) * 4))
            .and_then(|value| value.checked_add(dispatch_row_bytes))
            .and_then(|value| value.checked_add(staging_bytes))
            .and_then(|value| value.checked_add(compact_readback_bytes))
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let alignment_padding_bytes = per_slot_physical_without_shared
            .checked_sub(logical_slot_commit_bytes)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let active_frame_bytes = staging_bytes
            .checked_mul(slot_count)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let shared_class_bytes = self
            .sizes
            .upload_staging
            .checked_add(
                self.sizes
                    .frame_payload_words
                    .saturating_sub(active_frame_bytes),
            )
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let receipt = crate::GpuSlotAllocationReceipt::new(
            self.capacity.id().raw(),
            components,
            alignment_padding_bytes,
            shared_class_bytes,
        )
        .map_err(|_| GpuClosedLoopError::CapacityExceeded)?;
        let reconciled = shared_class_bytes
            .checked_add(
                logical_slot_commit_bytes
                    .checked_add(alignment_padding_bytes)
                    .and_then(|value| value.checked_mul(slot_count))
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
            )
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        if reconciled != self.sizes.aggregate {
            return Err(GpuClosedLoopError::LayoutMismatch);
        }
        Ok(receipt)
    }
    #[allow(dead_code)]
    pub(crate) const fn ownership_token(&self) -> u64 {
        self.arena_ownership_token
    }
    pub(crate) fn slot_ranges(&self, slot: u32) -> Result<GpuFixedSlotRanges, GpuClosedLoopError> {
        if slot >= self.slot_capacity {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let brain_stride = std::mem::size_of::<GpuBrainSlotRecord>() as u64;
        let identity_stride = std::mem::size_of::<GpuPhenotypeIdentityRecord>() as u64;
        let brain_start = u64::from(slot)
            .checked_mul(brain_stride)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let identity_start = u64::from(slot)
            .checked_mul(identity_stride)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let plan_base = slot
            .checked_mul(self.strides.immutable_plan_words)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let weight_base = slot
            .checked_mul(self.strides.immutable_weight_words)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let mutable_base = slot
            .checked_mul(self.strides.mutable_state_words)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
        let mut layout = self.relative_layout.clone();
        for range in [
            &mut layout.encoder_plan_words,
            &mut layout.encoder_assignment_words,
            &mut layout.encoder_target_offset_words,
            &mut layout.neuron_dynamics_words,
            &mut layout.projection_words,
            &mut layout.route_metadata_words,
            &mut layout.target_offset_words,
            &mut layout.source_index_words,
            &mut layout.route_index_words,
            &mut layout.decoder_plan_words,
            &mut layout.decoder_family_words,
            &mut layout.decoder_weight_index_words,
            &mut layout.memory_channel_plan_words,
            &mut layout.memory_weight_index_words,
            &mut layout.receptor_words,
            &mut layout.synapse_learning_metadata_words,
            &mut layout.decoder_eligibility_metadata_words,
            &mut layout.replay_plan_identity_words,
            &mut layout.sleep_parameter_words,
        ] {
            shift_range(range, plan_base)?;
        }
        for range in [&mut layout.genetic_weight_words, &mut layout.alpha_words] {
            shift_range(range, weight_base)?;
        }
        for range in [
            &mut layout.activation_a_words,
            &mut layout.activation_b_words,
            &mut layout.accumulator_words,
            &mut layout.homeostasis_words,
            &mut layout.lifetime_weight_words,
            &mut layout.fast_weight_words,
            &mut layout.recurrent_eligibility_words,
            &mut layout.decoder_eligibility_words,
            &mut layout.lifetime_weight_bank_1_words,
            &mut layout.fast_weight_bank_1_words,
            &mut layout.recurrent_eligibility_bank_1_words,
            &mut layout.decoder_eligibility_bank_1_words,
            &mut layout.encoded_input_words,
            &mut layout.candidate_logit_words,
            &mut layout.diagnostic_words,
            &mut layout.selection_words,
            &mut layout.extension_words,
            &mut layout.learning_state_words,
            &mut layout.pending_eligibility_words,
            &mut layout.replay_event_words,
            &mut layout.replay_sample_words,
            &mut layout.replay_span_words,
        ] {
            shift_range(range, mutable_base)?;
        }
        Ok(GpuFixedSlotRanges {
            arena_ownership_token: self.arena_ownership_token,
            slot,
            brain_slot_bytes: brain_start..brain_start + brain_stride,
            identity_bytes: identity_start..identity_start + identity_stride,
            immutable_plan_words: plan_base..plan_base + self.strides.immutable_plan_words,
            immutable_weight_words: weight_base..weight_base + self.strides.immutable_weight_words,
            mutable_state_words: mutable_base..mutable_base + self.strides.mutable_state_words,
            layout,
        })
    }

    pub(crate) fn prepare_slot_upload(
        &self,
        slot: u32,
        generation: u32,
        phenotype: &BrainPhenotype,
    ) -> Result<GpuFixedSlotUpload, GpuClosedLoopError> {
        if generation == 0 || phenotype.brain_class_id() != self.capacity.id() {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        let upload = GpuPhenotypeUpload::try_from(phenotype)?;
        upload.validate_against(phenotype)?;
        let execution = self.capacity.execution();
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
            memory_channel_plans: upload.memory_channel_plans.len(),
            memory_weight_indices: upload.memory_weight_indices.len(),
            plasticity_receptors: upload.plasticity_receptors.len(),
            synapse_learning_metadata: upload.synapse_learning_metadata.len(),
            decoder_eligibility_metadata: upload.decoder_eligibility_metadata.len(),
            replay_capture_synapses: upload.replay_capture_local_synapse_ids.len(),
            sleep_parameters: upload.sleep_parameters.len(),
        };
        let encoder_ceiling = usize::try_from(self.strides.encoder_assignment_count)
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        if counts.encoder_plans != 1
            || counts.encoder_assignments > encoder_ceiling
            || counts.encoder_target_offsets > encoder_ceiling + 1
            || counts.neuron_dynamics > execution.max_neurons() as usize
            || counts.projections > execution.max_active_tiles() as usize
            || counts.route_metadata > execution.max_active_tiles() as usize
            || counts.target_offsets > execution.max_neurons() as usize + 1
            || counts.source_indices > execution.max_recurrent_synapses() as usize
            || counts.route_indices > execution.max_recurrent_synapses() as usize
            || counts.decoder_plans != 1
            || counts.decoder_families > 8
            || counts.decoder_weight_indices > execution.max_action_decoder_synapses() as usize
            || counts.memory_channel_plans != 1
            || counts.memory_weight_indices == 0
            || counts.memory_weight_indices > execution.max_memory_decoder_synapses() as usize
            || counts.plasticity_receptors > execution.max_active_tiles() as usize + 4
            || counts.synapse_learning_metadata > execution.max_total_synapses() as usize
            || counts.decoder_eligibility_metadata
                > execution
                    .max_action_decoder_synapses()
                    .saturating_add(execution.max_memory_decoder_synapses())
                    as usize
            || counts.replay_capture_synapses > MAX_REPLAY_CAPTURE_SYNAPSES as usize
            || counts.sleep_parameters != 1
            || upload.genetic_weights.len() > execution.max_total_synapses() as usize
            || upload.alpha.len() != upload.genetic_weights.len()
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let ranges = self.slot_ranges(slot)?;
        let mut immutable_plan_words = vec![0_u32; self.strides.immutable_plan_words as usize];
        let plan_base = ranges.immutable_plan_words.start;
        let mut encoder_plan = upload.encoder_plans[0];
        encoder_plan.assignment_offset = ranges.layout.encoder_assignment_words.start;
        encoder_plan.target_offsets_offset = ranges.layout.encoder_target_offset_words.start;
        store_pod_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.encoder_plan_words.start,
            &encoder_plan,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.encoder_assignment_words.start,
            &upload.encoder_assignments,
        )?;
        store_words_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.encoder_target_offset_words.start,
            &upload.encoder_target_offsets,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.neuron_dynamics_words.start,
            &upload.neuron_dynamics,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.projection_words.start,
            &upload.projections,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.route_metadata_words.start,
            &upload.route_metadata,
        )?;
        store_words_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.target_offset_words.start,
            &upload.target_offsets,
        )?;
        store_words_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.source_index_words.start,
            &upload.source_indices,
        )?;
        store_words_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.route_index_words.start,
            &upload.route_indices,
        )?;
        let mut decoder_plan = upload.decoder_plans[0];
        decoder_plan.family_offset = ranges.layout.decoder_family_words.start;
        store_pod_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.decoder_plan_words.start,
            &decoder_plan,
        )?;
        let relocated_families = upload
            .decoder_families
            .iter()
            .map(|row| {
                let mut relocated = *row;
                let local = row
                    .weight_index_start
                    .checked_sub(upload.decoder_weight_index_word_base)
                    .ok_or(GpuClosedLoopError::MalformedUpload)?
                    / 4;
                relocated.weight_index_start = ranges
                    .layout
                    .decoder_weight_index_words
                    .start
                    .checked_add(
                        local
                            .checked_mul(4)
                            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
                    )
                    .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
                Ok(relocated)
            })
            .collect::<Result<Vec<_>, GpuClosedLoopError>>()?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.decoder_family_words.start,
            &relocated_families,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.decoder_weight_index_words.start,
            &upload.decoder_weight_indices,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.memory_channel_plan_words.start,
            &upload.memory_channel_plans,
        )?;
        store_words_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.memory_weight_index_words.start,
            &upload.memory_weight_indices,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.receptor_words.start,
            &upload.plasticity_receptors,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.synapse_learning_metadata_words.start,
            &upload.synapse_learning_metadata,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.decoder_eligibility_metadata_words.start,
            &upload.decoder_eligibility_metadata,
        )?;
        store_pod_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.replay_plan_identity_words.start,
            &upload.replay_capture_identity,
        )?;
        store_pod_slice_at(
            &mut immutable_plan_words,
            plan_base,
            ranges.layout.sleep_parameter_words.start,
            &upload.sleep_parameters,
        )?;

        let mut immutable_weight_words = vec![0_u32; self.strides.immutable_weight_words as usize];
        let weight_base = ranges.immutable_weight_words.start;
        let genetic = upload
            .genetic_weights
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>();
        let alpha = upload
            .alpha
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>();
        store_words_at(
            &mut immutable_weight_words,
            weight_base,
            ranges.layout.genetic_weight_words.start,
            &genetic,
        )?;
        store_words_at(
            &mut immutable_weight_words,
            weight_base,
            ranges.layout.alpha_words.start,
            &alpha,
        )?;
        let mut mutable_state_words = vec![0_u32; self.strides.mutable_state_words as usize];
        let mutable_base = ranges.mutable_state_words.start;
        let extension = make_slot_extension(
            &ranges.layout,
            &counts,
            u32::try_from(upload.source_indices.len())
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?,
        )?;
        store_pod_at(
            &mut mutable_state_words,
            mutable_base,
            ranges.layout.extension_words.start,
            &extension,
        )?;
        let learning_state = make_learning_state(
            &ranges.layout,
            phenotype.replay_capture_plan().event_capacity(),
            phenotype.replay_capture_plan().sample_capacity(),
            u32::try_from(upload.replay_capture_local_synapse_ids.len())
                .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?,
        );
        store_pod_at(
            &mut mutable_state_words,
            mutable_base,
            ranges.layout.learning_state_words.start,
            &learning_state,
        )?;
        let replay_spans = make_replay_spans(
            &upload.replay_capture_local_synapse_ids,
            phenotype.replay_capture_plan().event_capacity(),
        )?;
        store_pod_slice_at(
            &mut mutable_state_words,
            mutable_base,
            ranges.layout.replay_span_words.start,
            &replay_spans,
        )?;
        let record = GpuBrainSlotRecord {
            schema_version: u32::from(upload.gpu_layout_version),
            class_id: upload.class_id,
            slot,
            slot_generation: generation,
            neuron_count: upload.neuron_count,
            microstep_count: upload.microstep_count,
            synapse_count: upload.genetic_weights.len() as u32,
            recurrent_synapse_count: upload.source_indices.len() as u32,
            encoder_plan_offset: ranges.layout.encoder_plan_words.start,
            neuron_dynamics_offset: ranges.layout.neuron_dynamics_words.start,
            projection_offset: ranges.layout.projection_words.start,
            route_metadata_offset: ranges.layout.route_metadata_words.start,
            target_offsets_offset: ranges.layout.target_offset_words.start,
            source_indices_offset: ranges.layout.source_index_words.start,
            route_indices_offset: ranges.layout.route_index_words.start,
            decoder_plan_offset: ranges.layout.decoder_plan_words.start,
            decoder_family_offset: ranges.layout.decoder_family_words.start,
            decoder_weight_indices_offset: ranges.layout.decoder_weight_index_words.start,
            genetic_weight_offset: ranges.layout.genetic_weight_words.start,
            alpha_offset: ranges.layout.alpha_words.start,
            activation_a_offset: ranges.layout.activation_a_words.start,
            activation_b_offset: ranges.layout.activation_b_words.start,
            accumulator_offset: ranges.layout.accumulator_words.start,
            lifetime_weight_offset: ranges.layout.lifetime_weight_words.start,
            fast_weight_offset: ranges.layout.fast_weight_words.start,
            recurrent_eligibility_offset: ranges.layout.recurrent_eligibility_words.start,
            decoder_eligibility_offset: ranges.layout.decoder_eligibility_words.start,
            encoded_input_offset: ranges.layout.encoded_input_words.start,
            candidate_logit_offset: ranges.layout.candidate_logit_words.start,
            diagnostic_offset: ranges.layout.diagnostic_words.start,
            selection_offset: ranges.layout.selection_words.start,
            neuron_homeostasis_offset: ranges.layout.homeostasis_words.start,
            extension_record_offset: ranges.layout.extension_words.start,
            reserved: [0; 3],
        };
        record.validate_slice_a()?;
        validate_learning_slot_layout(
            &record,
            &counts,
            &ranges.layout,
            &immutable_plan_words,
            plan_base,
            &immutable_weight_words,
            weight_base,
            &mutable_state_words,
            mutable_base,
        )?;
        let brain_slot = GpuBrainSlot {
            record,
            identity: upload.identity,
            counts,
            ranges: ranges.layout.clone(),
            decoder_input_stride: upload.decoder_plans[0].flattened_input_lane_count,
            brain_slot_index: slot,
            bucket_ownership_token: self.arena_ownership_token,
        };
        Ok(GpuFixedSlotUpload {
            arena_ownership_token: self.arena_ownership_token,
            brain_slot,
            ranges,
            immutable_plan_words,
            immutable_weight_words,
            mutable_state_words,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn validate_slot_handle(
        &self,
        slot: &GpuBrainSlot,
    ) -> Result<(), GpuClosedLoopError> {
        if slot.bucket_ownership_token != self.arena_ownership_token
            || slot.brain_slot_index >= self.slot_capacity
            || slot.record.class_id != u32::from(self.capacity.id().raw())
            || slot.record.slot != slot.brain_slot_index
            || slot.record.slot_generation == 0
            || self.slot_ranges(slot.brain_slot_index)?.layout != slot.ranges
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct GpuFixedSlotUpload {
    arena_ownership_token: u64,
    brain_slot: GpuBrainSlot,
    ranges: GpuFixedSlotRanges,
    immutable_plan_words: Vec<u32>,
    immutable_weight_words: Vec<u32>,
    mutable_state_words: Vec<u32>,
}

impl GpuFixedSlotUpload {
    pub(crate) const fn record(&self) -> &GpuBrainSlotRecord {
        self.brain_slot.record()
    }
    pub(crate) const fn identity(&self) -> &GpuPhenotypeIdentityRecord {
        self.brain_slot.identity()
    }
    #[allow(dead_code)]
    pub(crate) const fn counts(&self) -> &GpuTypedCounts {
        self.brain_slot.typed_counts()
    }
    pub(crate) const fn ranges(&self) -> &GpuFixedSlotRanges {
        &self.ranges
    }
    pub(crate) const fn brain_slot(&self) -> &GpuBrainSlot {
        &self.brain_slot
    }
}

fn store_pod_at<T: bytemuck::Pod>(
    destination: &mut [u32],
    destination_base: u32,
    absolute_start: u32,
    value: &T,
) -> Result<(), GpuClosedLoopError> {
    store_words_at(
        destination,
        destination_base,
        absolute_start,
        bytemuck::cast_slice(std::slice::from_ref(value)),
    )
}

fn store_pod_slice_at<T: bytemuck::Pod>(
    destination: &mut [u32],
    destination_base: u32,
    absolute_start: u32,
    values: &[T],
) -> Result<(), GpuClosedLoopError> {
    store_words_at(
        destination,
        destination_base,
        absolute_start,
        bytemuck::cast_slice(values),
    )
}

fn store_words_at(
    destination: &mut [u32],
    destination_base: u32,
    absolute_start: u32,
    words: &[u32],
) -> Result<(), GpuClosedLoopError> {
    let local_start = absolute_start
        .checked_sub(destination_base)
        .ok_or(GpuClosedLoopError::MalformedUpload)? as usize;
    let local_end = local_start
        .checked_add(words.len())
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    let target = destination
        .get_mut(local_start..local_end)
        .ok_or(GpuClosedLoopError::CapacityExceeded)?;
    target.copy_from_slice(words);
    Ok(())
}

pub(crate) struct GpuFixedClassArenaBuffers {
    brain_slots: wgpu::Buffer,
    phenotype_identities: wgpu::Buffer,
    immutable_plan_words: wgpu::Buffer,
    immutable_weight_words: wgpu::Buffer,
    dispatch_header_words: wgpu::Buffer,
    frame_payload_words: wgpu::Buffer,
    mutable_state_words: wgpu::Buffer,
    #[allow(dead_code)]
    upload_staging: wgpu::Buffer,
    compact_readback: wgpu::Buffer,
    arena_ownership_token: u64,
    buffer_set_token: u64,
    max_neurons: u32,
    slot_capacity: u32,
    sizes: GpuFixedArenaBufferSizes,
}

impl GpuFixedClassArenaBuffers {
    pub(crate) fn allocate(
        device: &wgpu::Device,
        plan: &GpuFixedClassArenaPlan,
    ) -> Result<Self, GpuClosedLoopError> {
        let sizes = plan.buffer_sizes();
        let limits = device.limits();
        if plan.slot_capacity() > limits.max_compute_workgroups_per_dimension
            || [
                sizes.brain_slots,
                sizes.phenotype_identities,
                sizes.immutable_plan_words,
                sizes.immutable_weight_words,
                sizes.dispatch_header_words,
                sizes.frame_payload_words,
                sizes.mutable_state_words,
            ]
            .into_iter()
            .any(|bytes| {
                bytes > limits.max_buffer_size || bytes > limits.max_storage_buffer_binding_size
            })
            || [sizes.upload_staging, sizes.compact_readback]
                .into_iter()
                .any(|bytes| bytes > limits.max_buffer_size)
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        let storage_read_only = wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_DST
            | wgpu::BufferUsages::COPY_SRC;
        let storage_mutable = storage_read_only;
        let buffer_set_token = NEXT_BUFFER_SET_TOKEN
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| GpuClosedLoopError::ArithmeticOverflow)?;
        Ok(Self {
            brain_slots: create_fixed_buffer(
                device,
                "closed-loop-runtime-brain-slots",
                sizes.brain_slots,
                storage_read_only,
            ),
            phenotype_identities: create_fixed_buffer(
                device,
                "closed-loop-runtime-identities",
                sizes.phenotype_identities,
                storage_read_only,
            ),
            immutable_plan_words: create_fixed_buffer(
                device,
                "closed-loop-runtime-immutable-plan",
                sizes.immutable_plan_words,
                storage_read_only,
            ),
            immutable_weight_words: create_fixed_buffer(
                device,
                "closed-loop-runtime-immutable-weights",
                sizes.immutable_weight_words,
                storage_read_only,
            ),
            dispatch_header_words: create_fixed_buffer(
                device,
                "closed-loop-runtime-dispatch",
                sizes.dispatch_header_words,
                storage_read_only,
            ),
            frame_payload_words: create_fixed_buffer(
                device,
                "closed-loop-runtime-frame-payload",
                sizes.frame_payload_words,
                storage_read_only,
            ),
            mutable_state_words: create_fixed_buffer(
                device,
                "closed-loop-runtime-mutable-state",
                sizes.mutable_state_words,
                storage_mutable,
            ),
            upload_staging: create_fixed_buffer(
                device,
                "closed-loop-runtime-upload-staging",
                sizes.upload_staging,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            ),
            compact_readback: create_fixed_buffer(
                device,
                "closed-loop-runtime-compact-readback",
                sizes.compact_readback,
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            ),
            arena_ownership_token: plan.arena_ownership_token,
            buffer_set_token,
            max_neurons: plan.capacity().execution().max_neurons(),
            slot_capacity: plan.slot_capacity(),
            sizes,
        })
    }

    pub(crate) fn write_slot_upload(
        &self,
        queue: &wgpu::Queue,
        upload: &GpuFixedSlotUpload,
    ) -> Result<(), GpuClosedLoopError> {
        if upload.arena_ownership_token != self.arena_ownership_token
            || upload.ranges.arena_ownership_token != self.arena_ownership_token
            || upload.ranges.slot >= self.slot_capacity
            || upload.immutable_plan_words.len() as u64 * 4
                != upload.ranges.immutable_plan_words.len() as u64 * 4
            || upload.immutable_weight_words.len() as u64 * 4
                != upload.ranges.immutable_weight_words.len() as u64 * 4
            || upload.mutable_state_words.len() as u64 * 4
                != upload.ranges.mutable_state_words.len() as u64 * 4
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        queue.write_buffer(
            &self.brain_slots,
            upload.ranges.brain_slot_bytes.start,
            bytemuck::bytes_of(upload.record()),
        );
        queue.write_buffer(
            &self.phenotype_identities,
            upload.ranges.identity_bytes.start,
            bytemuck::bytes_of(upload.identity()),
        );
        queue.write_buffer(
            &self.immutable_plan_words,
            u64::from(upload.ranges.immutable_plan_words.start) * 4,
            bytemuck::cast_slice(&upload.immutable_plan_words),
        );
        queue.write_buffer(
            &self.immutable_weight_words,
            u64::from(upload.ranges.immutable_weight_words.start) * 4,
            bytemuck::cast_slice(&upload.immutable_weight_words),
        );
        Ok(())
    }

    pub(crate) fn write_mutable_slot_upload(
        &self,
        queue: &wgpu::Queue,
        upload: &GpuFixedSlotUpload,
    ) -> Result<(), GpuClosedLoopError> {
        if upload.arena_ownership_token != self.arena_ownership_token
            || upload.ranges.arena_ownership_token != self.arena_ownership_token
            || upload.ranges.slot >= self.slot_capacity
            || upload.mutable_state_words.len() as u64 * 4
                != upload.ranges.mutable_state_words.len() as u64 * 4
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        queue.write_buffer(
            &self.mutable_state_words,
            u64::from(upload.ranges.mutable_state_words.start) * 4,
            bytemuck::cast_slice(&upload.mutable_state_words),
        );
        Ok(())
    }

    pub(crate) fn record_mutable_slot_reset(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        ranges: &GpuFixedSlotRanges,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_ranges(ranges)?;
        let bytes = words_to_bytes(ranges.mutable_state_words.clone())?;
        encoder.clear_buffer(
            &self.mutable_state_words,
            bytes.start,
            Some(bytes.end - bytes.start),
        );
        Ok(())
    }

    pub(crate) fn record_full_slot_scrub(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        ranges: &GpuFixedSlotRanges,
    ) -> Result<(), GpuClosedLoopError> {
        self.validate_ranges(ranges)?;
        for (heap, range) in ranges.full_scrub_ranges() {
            let buffer = match heap {
                GpuFixedArenaHeap::BrainSlots => &self.brain_slots,
                GpuFixedArenaHeap::PhenotypeIdentities => &self.phenotype_identities,
                GpuFixedArenaHeap::ImmutablePlanWords => &self.immutable_plan_words,
                GpuFixedArenaHeap::ImmutableWeightWords => &self.immutable_weight_words,
                GpuFixedArenaHeap::MutableStateWords => &self.mutable_state_words,
            };
            encoder.clear_buffer(buffer, range.start, Some(range.end - range.start));
        }
        Ok(())
    }

    pub(crate) fn record_persistent_prefix_copy_to(
        &self,
        target: &Self,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<(), GpuClosedLoopError> {
        if self.max_neurons != target.max_neurons
            || self.slot_capacity > target.slot_capacity
            || self.sizes.brain_slots > target.sizes.brain_slots
            || self.sizes.phenotype_identities > target.sizes.phenotype_identities
            || self.sizes.immutable_plan_words > target.sizes.immutable_plan_words
            || self.sizes.immutable_weight_words > target.sizes.immutable_weight_words
            || self.sizes.mutable_state_words > target.sizes.mutable_state_words
        {
            return Err(GpuClosedLoopError::CapacityExceeded);
        }
        for (source, destination, bytes) in [
            (
                &self.brain_slots,
                &target.brain_slots,
                self.sizes.brain_slots,
            ),
            (
                &self.phenotype_identities,
                &target.phenotype_identities,
                self.sizes.phenotype_identities,
            ),
            (
                &self.immutable_plan_words,
                &target.immutable_plan_words,
                self.sizes.immutable_plan_words,
            ),
            (
                &self.immutable_weight_words,
                &target.immutable_weight_words,
                self.sizes.immutable_weight_words,
            ),
            (
                &self.mutable_state_words,
                &target.mutable_state_words,
                self.sizes.mutable_state_words,
            ),
        ] {
            encoder.copy_buffer_to_buffer(source, 0, destination, 0, bytes);
        }
        Ok(())
    }

    fn validate_ranges(&self, ranges: &GpuFixedSlotRanges) -> Result<(), GpuClosedLoopError> {
        if ranges.arena_ownership_token != self.arena_ownership_token
            || ranges.slot >= self.slot_capacity
            || ranges.brain_slot_bytes.end > self.sizes.brain_slots
            || ranges.identity_bytes.end > self.sizes.phenotype_identities
            || u64::from(ranges.immutable_plan_words.end) * 4 > self.sizes.immutable_plan_words
            || u64::from(ranges.immutable_weight_words.end) * 4 > self.sizes.immutable_weight_words
            || u64::from(ranges.mutable_state_words.end) * 4 > self.sizes.mutable_state_words
        {
            return Err(GpuClosedLoopError::StaleOrForeignHandle);
        }
        Ok(())
    }

    pub(crate) fn neural_buffers(&self) -> [&wgpu::Buffer; 7] {
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
    #[allow(dead_code)]
    pub(crate) const fn upload_staging(&self) -> &wgpu::Buffer {
        &self.upload_staging
    }
    pub(crate) const fn compact_readback(&self) -> &wgpu::Buffer {
        &self.compact_readback
    }
    pub(crate) const fn ownership_token(&self) -> u64 {
        self.arena_ownership_token
    }
    pub(crate) const fn buffer_set_token(&self) -> u64 {
        self.buffer_set_token
    }
    pub(crate) const fn max_neurons(&self) -> u32 {
        self.max_neurons
    }
    #[allow(dead_code)]
    pub(crate) const fn slot_capacity(&self) -> u32 {
        self.slot_capacity
    }
    #[allow(dead_code)]
    pub(crate) const fn sizes(&self) -> GpuFixedArenaBufferSizes {
        self.sizes
    }
    pub(crate) fn dispatch_capacity_words(&self) -> usize {
        (self.sizes.dispatch_header_words / 4) as usize
    }
    pub(crate) fn frame_payload_capacity_words(&self) -> usize {
        (self.sizes.frame_payload_words / 4) as usize
    }
    pub(crate) const fn compact_readback_capacity_bytes(&self) -> u64 {
        self.sizes.compact_readback
    }
}

fn create_fixed_buffer(
    device: &wgpu::Device,
    label: &'static str,
    size: u64,
    usage: wgpu::BufferUsages,
) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size,
        usage,
        mapped_at_creation: false,
    })
}

fn span_u32(cursor: &mut u32, len: u32) -> Result<Range<u32>, GpuClosedLoopError> {
    let start = *cursor;
    *cursor = cursor
        .checked_add(len)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    Ok(start..*cursor)
}

fn align_words(words: u32, alignment_bytes: u32) -> Result<u32, GpuClosedLoopError> {
    let alignment_words = alignment_bytes
        .checked_div(4)
        .filter(|value| *value > 0 && alignment_bytes.is_multiple_of(4))
        .ok_or(GpuClosedLoopError::LayoutMismatch)?;
    let remainder = words % alignment_words;
    if remainder == 0 {
        Ok(words)
    } else {
        words
            .checked_add(alignment_words - remainder)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)
    }
}

fn checked_mul_bytes(left: u64, right: u64) -> Result<u64, GpuClosedLoopError> {
    left.checked_mul(right)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)
}

fn checked_word_buffer_bytes(
    stride_words: u32,
    slot_capacity: u32,
) -> Result<u64, GpuClosedLoopError> {
    checked_mul_bytes(
        u64::from(stride_words),
        u64::from(slot_capacity)
            .checked_mul(4)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?,
    )
}

fn words_to_bytes(words: Range<u32>) -> Result<Range<u64>, GpuClosedLoopError> {
    Ok(u64::from(words.start)
        .checked_mul(4)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?
        ..u64::from(words.end)
            .checked_mul(4)
            .ok_or(GpuClosedLoopError::ArithmeticOverflow)?)
}

fn shift_range(range: &mut Range<u32>, base: u32) -> Result<(), GpuClosedLoopError> {
    range.start = range
        .start
        .checked_add(base)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    range.end = range
        .end
        .checked_add(base)
        .ok_or(GpuClosedLoopError::ArithmeticOverflow)?;
    Ok(())
}

#[cfg(test)]
mod fixed_arena_tests {
    use super::*;
    use alife_core::{
        BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
        MAX_REPLAY_CAPTURE_SYNAPSES,
    };

    #[test]
    fn fixed_slot_ranges_are_disjoint_and_ceiling_derived_for_every_promoted_class() {
        for capacity in BrainCapacityClass::production_classes() {
            let plan = GpuFixedClassArenaPlan::new(capacity, 4, 512 * 1024 * 1024).unwrap();
            assert_eq!(
                plan.relative_layout.replay_span_words.end
                    - plan.relative_layout.replay_span_words.start,
                MAX_REPLAY_CAPTURE_SYNAPSES * 4
            );
            assert_eq!(
                plan.strides().encoder_assignment_count,
                capacity.execution().max_neurons() * 2
            );
            assert_eq!(
                plan.strides().projection_count,
                capacity.execution().max_active_tiles()
            );
            assert_eq!(
                plan.strides().route_count,
                capacity.execution().max_active_tiles()
            );
            for slot in 0..4 {
                let current = plan.slot_ranges(slot).unwrap();
                assert_eq!(current.slot(), slot);
                if slot > 0 {
                    let previous = plan.slot_ranges(slot - 1).unwrap();
                    assert!(previous.brain_slot_bytes.end <= current.brain_slot_bytes.start);
                    assert!(previous.identity_bytes.end <= current.identity_bytes.start);
                    assert!(
                        previous.immutable_plan_words.end <= current.immutable_plan_words.start
                    );
                    assert!(
                        previous.immutable_weight_words.end <= current.immutable_weight_words.start
                    );
                    assert!(previous.mutable_state_words.end <= current.mutable_state_words.start);
                }
            }
        }
    }

    #[test]
    fn fixed_arena_readback_covers_the_largest_gpu_transaction_per_slot() {
        let slots = 3_u32;
        let plan =
            GpuFixedClassArenaPlan::new(BrainCapacityClass::n512(), slots, 512 * 1024 * 1024)
                .unwrap();

        assert_eq!(
            plan.sizes.compact_readback,
            u64::from(slots) * crate::GPU_FAST_PLASTICITY_COMMIT_BYTES as u64
        );
        const {
            assert!(
                crate::GPU_FAST_PLASTICITY_COMMIT_BYTES
                    >= crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES
            );
        }
    }

    #[test]
    fn fixed_arena_rejects_zero_huge_and_aggregate_ceiling_overflow_without_allocation() {
        let capacity = BrainCapacityClass::n512();
        assert_eq!(
            GpuFixedClassArenaPlan::new(capacity, 0, u64::MAX).unwrap_err(),
            GpuClosedLoopError::CapacityExceeded
        );
        assert_eq!(
            GpuFixedClassArenaPlan::new(capacity, 1, 1).unwrap_err(),
            GpuClosedLoopError::CapacityExceeded
        );
        assert!(matches!(
            GpuFixedClassArenaPlan::new(capacity, u32::MAX, u64::MAX),
            Err(GpuClosedLoopError::CapacityExceeded | GpuClosedLoopError::ArithmeticOverflow)
        ));
        let default_total = [
            (BrainCapacityClass::n512(), 64),
            (BrainCapacityClass::n1024(), 16),
            (BrainCapacityClass::n2048(), 4),
        ]
        .into_iter()
        .try_fold(0_u64, |total, (capacity, slots)| {
            let plan = GpuFixedClassArenaPlan::new(capacity, slots, u64::MAX)?;
            total
                .checked_add(plan.aggregate_resident_bytes())
                .ok_or(GpuClosedLoopError::ArithmeticOverflow)
        })
        .unwrap();
        assert!(default_total <= 128 * 1024 * 1024);
    }

    #[test]
    fn full_slot_scrub_manifest_covers_every_reserved_byte_exactly_once() {
        let plan =
            GpuFixedClassArenaPlan::new(BrainCapacityClass::n512(), 2, 512 * 1024 * 1024).unwrap();
        let slot = plan.slot_ranges(1).unwrap();
        let scrub = slot.full_scrub_ranges();
        assert_eq!(scrub.len(), 5);
        assert_eq!(scrub[0].1, slot.brain_slot_bytes);
        assert_eq!(scrub[1].1, slot.identity_bytes);
        assert_eq!(
            scrub[2].1,
            words_to_bytes(slot.immutable_plan_words.clone()).unwrap()
        );
        assert_eq!(
            scrub[3].1,
            words_to_bytes(slot.immutable_weight_words.clone()).unwrap()
        );
        assert_eq!(
            scrub[4].1,
            words_to_bytes(slot.mutable_state_words.clone()).unwrap()
        );
        assert!(scrub.iter().all(|(_, range)| range.start < range.end));
    }

    #[test]
    fn fixed_slot_upload_relocates_immutable_and_initial_learning_state() {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(701, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::PrivilegedAffordanceV1,
        )
        .unwrap();
        let plan = GpuFixedClassArenaPlan::new(capacity, 2, 512 * 1024 * 1024).unwrap();
        let upload = plan.prepare_slot_upload(1, 9, &phenotype).unwrap();
        let expected_identity = GpuPhenotypeUpload::try_from(&phenotype).unwrap().identity;
        plan.validate_slot_handle(upload.brain_slot()).unwrap();
        let ranges = upload.ranges();
        assert_eq!(upload.record().slot, 1);
        assert_eq!(upload.record().slot_generation, 9);
        assert_eq!(*upload.identity(), expected_identity);
        assert_eq!(
            upload.record().encoder_plan_offset,
            ranges.layout.encoder_plan_words.start
        );
        assert_eq!(
            upload.record().activation_a_offset,
            ranges.layout.activation_a_words.start
        );
        assert_eq!(
            upload.immutable_plan_words.len(),
            plan.strides().immutable_plan_words as usize
        );
        assert_eq!(
            upload.immutable_weight_words.len(),
            plan.strides().immutable_weight_words as usize
        );
        assert_eq!(
            upload.mutable_state_words.len(),
            plan.strides().mutable_state_words as usize
        );
        let mutable_base = ranges.mutable_state_words.start;
        let extension_local = (upload.record().extension_record_offset - mutable_base) as usize;
        let extension = GpuBrainSlotExtensionRecord::from_words(
            &upload.mutable_state_words[extension_local
                ..extension_local + std::mem::size_of::<GpuBrainSlotExtensionRecord>() / 4],
        )
        .unwrap();
        assert_eq!(extension.schema_version, GPU_CLOSED_LOOP_LAYOUT_VERSION);
        assert_eq!(
            extension.fast_bank_1_offset,
            ranges.layout.fast_weight_bank_1_words.start
        );
        assert_eq!(
            extension.lifetime_bank_1_offset,
            ranges.layout.lifetime_weight_bank_1_words.start
        );
        let state_local = (extension.learning_state_offset - mutable_base) as usize;
        let state = GpuSlotLearningStateRecord::from_words(
            &upload.mutable_state_words
                [state_local..state_local + std::mem::size_of::<GpuSlotLearningStateRecord>() / 4],
        )
        .unwrap();
        assert_eq!(state.active_weight_bank, 0);
        assert_eq!(state.active_eligibility_bank, 0);
        assert_eq!(state.pending_valid, 0);
        assert_eq!(
            state.replay_event_capacity,
            phenotype.replay_capture_plan().event_capacity()
        );
        assert_eq!(
            state.replay_sample_capacity,
            phenotype.replay_capture_plan().sample_capacity()
        );
        for range in [
            &ranges.layout.lifetime_weight_words,
            &ranges.layout.fast_weight_words,
            &ranges.layout.recurrent_eligibility_words,
            &ranges.layout.decoder_eligibility_words,
            &ranges.layout.lifetime_weight_bank_1_words,
            &ranges.layout.fast_weight_bank_1_words,
            &ranges.layout.recurrent_eligibility_bank_1_words,
            &ranges.layout.decoder_eligibility_bank_1_words,
        ] {
            let local_start = (range.start - mutable_base) as usize;
            let local_end = (range.end - mutable_base) as usize;
            assert!(upload.mutable_state_words[local_start..local_end]
                .iter()
                .all(|word| *word == 0));
        }
        assert!(
            upload.counts().encoder_assignments
                <= (capacity.execution().max_neurons() * 2) as usize
        );
    }

    #[test]
    fn append_bucket_rejects_corrupted_learning_extension_offsets() {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(702, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let mut plan = GpuClassBucketPlan::new(capacity, 1).unwrap();
        let slot = plan.insert_phenotype(0, 1, &phenotype).unwrap();
        plan.validate().unwrap();

        let extension_start = slot.record().extension_record_offset as usize;
        let fast_bank_1_word = extension_start
            + std::mem::offset_of!(GpuBrainSlotExtensionRecord, fast_bank_1_offset) / 4;
        plan.mutable_state_words[fast_bank_1_word] = slot.word_ranges().fast_weight_words.start;

        assert_eq!(plan.validate(), Err(GpuClosedLoopError::MalformedUpload));
    }

    #[test]
    fn append_bucket_rejects_corrupted_learning_state_selectors() {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(703, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let mut plan = GpuClassBucketPlan::new(capacity, 1).unwrap();
        let slot = plan.insert_phenotype(0, 1, &phenotype).unwrap();
        let state_start = slot.word_ranges().learning_state_words.start as usize;
        let active_weight_bank_word =
            state_start + std::mem::offset_of!(GpuSlotLearningStateRecord, active_weight_bank) / 4;
        plan.mutable_state_words[active_weight_bank_word] = 2;

        assert_eq!(plan.validate(), Err(GpuClosedLoopError::MalformedUpload));
    }

    #[test]
    fn fixed_slot_validator_rejects_cross_heap_learning_offsets() {
        let capacity = BrainCapacityClass::n512();
        let genome = BrainGenome::scaffold(704, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let plan = GpuFixedClassArenaPlan::new(capacity, 2, 512 * 1024 * 1024).unwrap();
        let mut upload = plan.prepare_slot_upload(1, 11, &phenotype).unwrap();
        let mutable_base = upload.ranges.mutable_state_words.start;
        let extension_local = (upload.record().extension_record_offset - mutable_base) as usize;
        let receptor_offset_word = extension_local
            + std::mem::offset_of!(GpuBrainSlotExtensionRecord, receptor_offset) / 4;
        upload.mutable_state_words[receptor_offset_word] =
            upload.ranges.layout.fast_weight_words.start;

        assert_eq!(
            validate_learning_slot_layout(
                upload.record(),
                upload.counts(),
                &upload.ranges.layout,
                &upload.immutable_plan_words,
                upload.ranges.immutable_plan_words.start,
                &upload.immutable_weight_words,
                upload.ranges.immutable_weight_words.start,
                &upload.mutable_state_words,
                mutable_base,
            ),
            Err(GpuClosedLoopError::MalformedUpload)
        );
    }
}
