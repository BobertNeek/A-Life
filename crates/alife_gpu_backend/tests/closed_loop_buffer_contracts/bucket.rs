use std::ops::Range;

use alife_core::{BrainCapacityClass, MAX_ACTION_CANDIDATES};
use alife_gpu_backend::{
    GpuClassBucketPlan, GpuClosedLoopError, GpuDecoderFamilyRecord, GpuDecoderPlanRecord,
    GpuEncoderPlanRecord, GPU_NO_EXTENSION_SENTINEL,
};

use super::support::{compile, ranges_are_disjoint};

#[test]
fn slot_receipts_separate_typed_counts_from_checked_word_heap_ranges() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 7, &phenotype).unwrap();
    let counts = slot.typed_counts();
    let ranges = slot.word_ranges();
    let recurrent = phenotype.budgets().global.recurrent_synapses as usize;
    let decoder = phenotype.budgets().global.action_decoder_synapses as usize;

    assert_eq!(counts.encoder_plans, 1);
    assert_eq!(
        counts.encoder_assignments,
        phenotype.sensor_encoder().assignments().len()
    );
    assert_eq!(
        counts.encoder_target_offsets,
        phenotype.neuron_count() as usize + 1
    );
    assert_eq!(counts.neuron_dynamics, phenotype.neuron_count() as usize);
    assert_eq!(counts.projections, phenotype.projections().len());
    assert_eq!(counts.route_metadata, phenotype.projections().len());
    assert_eq!(counts.target_offsets, phenotype.neuron_count() as usize + 1);
    assert_eq!(counts.source_indices, recurrent);
    assert_eq!(counts.route_indices, recurrent);
    assert_eq!(counts.decoder_plans, 1);
    assert_eq!(
        counts.decoder_families,
        phenotype.candidate_decoder().families().len()
    );
    assert_eq!(counts.decoder_weight_indices, decoder);

    assert_eq!(ranges.encoder_plan_words.len(), counts.encoder_plans * 8);
    assert_eq!(
        ranges.encoder_assignment_words.len(),
        counts.encoder_assignments * 8
    );
    assert_eq!(
        ranges.encoder_target_offset_words.len(),
        counts.encoder_target_offsets
    );
    assert_eq!(
        ranges.neuron_dynamics_words.len(),
        counts.neuron_dynamics * 8
    );
    assert_eq!(ranges.projection_words.len(), counts.projections * 8);
    assert_eq!(
        ranges.route_metadata_words.len(),
        counts.route_metadata * 12
    );
    assert_eq!(ranges.target_offset_words.len(), counts.target_offsets);
    assert_eq!(ranges.source_index_words.len(), counts.source_indices);
    assert_eq!(ranges.route_index_words.len(), counts.route_indices);
    assert_eq!(ranges.decoder_plan_words.len(), counts.decoder_plans * 8);
    assert_eq!(
        ranges.decoder_family_words.len(),
        counts.decoder_families * 8
    );
    assert_eq!(
        ranges.decoder_weight_index_words.len(),
        counts.decoder_weight_indices * 4
    );
    assert_pairwise_disjoint(&plan_ranges(ranges));

    assert_eq!(
        ranges.genetic_weight_words.len(),
        phenotype.synapses().len()
    );
    assert_eq!(ranges.alpha_words.len(), phenotype.synapses().len());
    assert_pairwise_disjoint(&[
        ranges.genetic_weight_words.clone(),
        ranges.alpha_words.clone(),
    ]);

    assert_eq!(
        slot.record().encoder_plan_offset,
        ranges.encoder_plan_words.start
    );
    assert_eq!(
        slot.record().neuron_dynamics_offset,
        ranges.neuron_dynamics_words.start
    );
    assert_eq!(
        slot.record().projection_offset,
        ranges.projection_words.start
    );
    assert_eq!(
        slot.record().route_metadata_offset,
        ranges.route_metadata_words.start
    );
    assert_eq!(
        slot.record().target_offsets_offset,
        ranges.target_offset_words.start
    );
    assert_eq!(
        slot.record().source_indices_offset,
        ranges.source_index_words.start
    );
    assert_eq!(
        slot.record().route_indices_offset,
        ranges.route_index_words.start
    );
    assert_eq!(
        slot.record().decoder_plan_offset,
        ranges.decoder_plan_words.start
    );
    assert_eq!(
        slot.record().decoder_family_offset,
        ranges.decoder_family_words.start
    );
    assert_eq!(
        slot.record().decoder_weight_indices_offset,
        ranges.decoder_weight_index_words.start
    );
    assert_eq!(
        slot.record().genetic_weight_offset,
        ranges.genetic_weight_words.start
    );
    assert_eq!(slot.record().alpha_offset, ranges.alpha_words.start);
}

#[test]
fn packed_plan_records_relocate_every_nested_offset_to_shared_word_bases() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 7, &phenotype).unwrap();
    let ranges = slot.word_ranges();
    let words = bucket.immutable_plan_words();

    let encoder_start = ranges.encoder_plan_words.start as usize;
    let encoder =
        GpuEncoderPlanRecord::from_words(&words[encoder_start..encoder_start + 8]).unwrap();
    assert_eq!(
        encoder.assignment_offset,
        ranges.encoder_assignment_words.start
    );
    assert_eq!(
        encoder.target_offsets_offset,
        ranges.encoder_target_offset_words.start
    );

    let decoder_start = ranges.decoder_plan_words.start as usize;
    let decoder =
        GpuDecoderPlanRecord::from_words(&words[decoder_start..decoder_start + 8]).unwrap();
    assert_eq!(decoder.family_offset, ranges.decoder_family_words.start);
    assert_eq!(slot.record().decoder_family_offset, decoder.family_offset);

    let mut local_weight_cursor = 0_u32;
    for family_index in 0..slot.typed_counts().decoder_families {
        let start = ranges.decoder_family_words.start as usize + family_index * 8;
        let family = GpuDecoderFamilyRecord::from_words(&words[start..start + 8]).unwrap();
        assert_eq!(
            family.weight_index_start,
            ranges.decoder_weight_index_words.start + local_weight_cursor * 4
        );
        assert_eq!(family.weight_index_count, family.decoder_synapse_count);
        local_weight_cursor += family.decoder_synapse_count;
    }
    assert_eq!(
        local_weight_cursor as usize,
        slot.typed_counts().decoder_weight_indices
    );
}

#[test]
fn every_slice_a_mutable_pool_has_the_exact_word_length_and_offset() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 7, &phenotype).unwrap();
    let ranges = slot.word_ranges();
    let neuron_count = phenotype.neuron_count() as usize;
    let total = phenotype.synapses().len();
    let recurrent = phenotype.budgets().global.recurrent_synapses as usize;
    let decoder = total - recurrent;

    assert_eq!(ranges.activation_a_words.len(), neuron_count);
    assert_eq!(ranges.activation_b_words.len(), neuron_count);
    assert_eq!(ranges.accumulator_words.len(), neuron_count);
    assert_eq!(ranges.homeostasis_words.len(), neuron_count * 2);
    assert_eq!(ranges.lifetime_weight_words.len(), total);
    assert_eq!(ranges.fast_weight_words.len(), total);
    assert_eq!(ranges.recurrent_eligibility_words.len(), recurrent);
    assert_eq!(ranges.decoder_eligibility_words.len(), decoder);
    assert_eq!(ranges.encoded_input_words.len(), neuron_count);
    assert_eq!(ranges.candidate_logit_words.len(), MAX_ACTION_CANDIDATES);
    assert_eq!(ranges.diagnostic_words.len(), 4);
    assert_eq!(ranges.selection_words.len(), 12);
    assert_eq!(
        (ranges.diagnostic_words.len() + ranges.selection_words.len()) * 4,
        64
    );
    assert_pairwise_disjoint(&mutable_ranges(ranges));

    let record = slot.record();
    assert_eq!(record.activation_a_offset, ranges.activation_a_words.start);
    assert_eq!(record.activation_b_offset, ranges.activation_b_words.start);
    assert_eq!(record.accumulator_offset, ranges.accumulator_words.start);
    assert_eq!(
        record.lifetime_weight_offset,
        ranges.lifetime_weight_words.start
    );
    assert_eq!(record.fast_weight_offset, ranges.fast_weight_words.start);
    assert_eq!(
        record.recurrent_eligibility_offset,
        ranges.recurrent_eligibility_words.start
    );
    assert_eq!(
        record.decoder_eligibility_offset,
        ranges.decoder_eligibility_words.start
    );
    assert_eq!(
        record.encoded_input_offset,
        ranges.encoded_input_words.start
    );
    assert_eq!(
        record.candidate_logit_offset,
        ranges.candidate_logit_words.start
    );
    assert_eq!(record.diagnostic_offset, ranges.diagnostic_words.start);
    assert_eq!(record.selection_offset, ranges.selection_words.start);
    assert_eq!(
        record.neuron_homeostasis_offset,
        ranges.homeostasis_words.start
    );
}

#[test]
fn two_slots_cover_pairwise_disjoint_ranges_in_each_shared_heap() {
    let capacity = BrainCapacityClass::n512();
    let first = compile(capacity.id(), 41);
    let second = compile(capacity.id(), 42);
    let mut bucket = GpuClassBucketPlan::new(capacity, 2).unwrap();
    let left = bucket.insert_phenotype(0, 7, &first).unwrap();
    let right = bucket.insert_phenotype(1, 9, &second).unwrap();

    assert_eq!(left.record().reserved, [0; 3]);
    assert_eq!(right.record().reserved, [0; 3]);
    assert_eq!(
        left.record().extension_record_offset,
        left.word_ranges().extension_words.start
    );
    assert_eq!(
        right.record().extension_record_offset,
        right.word_ranges().extension_words.start
    );
    assert_ne!(
        left.record().extension_record_offset,
        right.record().extension_record_offset
    );
    assert_eq!(left.brain_slot_index(), 0);
    assert_eq!(right.brain_slot_index(), 1);
    assert_ne!(
        left.identity().phenotype_hash,
        right.identity().phenotype_hash
    );

    let mut plan = plan_ranges(left.word_ranges());
    plan.extend(plan_ranges(right.word_ranges()));
    assert_pairwise_disjoint(&plan);
    assert_exact_heap_coverage(&plan, bucket.immutable_plan_words().len());
    let weights = [
        left.word_ranges().genetic_weight_words.clone(),
        left.word_ranges().alpha_words.clone(),
        right.word_ranges().genetic_weight_words.clone(),
        right.word_ranges().alpha_words.clone(),
    ];
    assert_pairwise_disjoint(&weights);
    assert_exact_heap_coverage(&weights, bucket.immutable_weight_words().len());
    let mut mutable = mutable_ranges(left.word_ranges());
    mutable.extend(mutable_ranges(right.word_ranges()));
    assert_pairwise_disjoint(&mutable);
    assert_exact_heap_coverage(&mutable, bucket.mutable_state_words().len());
    bucket.validate().unwrap();
}

#[test]
fn mutating_one_slot_cannot_change_another_slot_or_immutable_weights() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 2).unwrap();
    let left = bucket.insert_phenotype(0, 7, &phenotype).unwrap();
    let right = bucket.insert_phenotype(1, 8, &phenotype).unwrap();
    let immutable_before = bucket.immutable_weight_words().to_vec();
    let right_before = bucket.fast_weights(&right).unwrap().to_vec();

    bucket.fast_weights_mut(&left).unwrap()[0] = 0.75;
    bucket.activation_a_mut(&left).unwrap()[0] = -0.5;

    assert_eq!(bucket.fast_weights(&left).unwrap()[0], 0.75);
    assert_eq!(bucket.activation_a(&left).unwrap()[0], -0.5);
    assert_eq!(bucket.fast_weights(&right).unwrap(), right_before);
    assert_eq!(bucket.activation_a(&right).unwrap()[0], 0.0);
    assert_eq!(bucket.immutable_weight_words(), immutable_before);
    bucket.validate().unwrap();
}

#[test]
fn same_class_slot_generation_and_phenotype_handle_from_another_bucket_is_rejected() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let filler = compile(capacity.id(), 42);
    let mut left = GpuClassBucketPlan::new(capacity, 2).unwrap();
    let mut right = GpuClassBucketPlan::new(capacity, 2).unwrap();
    left.insert_phenotype(1, 99, &filler).unwrap();
    let left_handle = left.insert_phenotype(0, 7, &phenotype).unwrap();
    let foreign_handle = right.insert_phenotype(0, 7, &phenotype).unwrap();
    let left_before = left.fast_weights(&left_handle).unwrap().to_vec();
    let right_before = right.fast_weights(&foreign_handle).unwrap().to_vec();

    assert_eq!(
        left.fast_weights_mut(&foreign_handle).unwrap_err(),
        GpuClosedLoopError::StaleOrForeignHandle
    );
    assert_eq!(left.fast_weights(&left_handle).unwrap(), left_before);
    assert_eq!(right.fast_weights(&foreign_handle).unwrap(), right_before);
}

#[test]
fn absurd_slot_capacity_is_rejected_before_allocation() {
    assert_eq!(
        GpuClassBucketPlan::new(BrainCapacityClass::n512(), u32::MAX).unwrap_err(),
        GpuClosedLoopError::CapacityExceeded
    );
}

#[test]
fn admitted_shared_heaps_stay_within_the_capacity_buffer_ceiling() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 2).unwrap();
    bucket.insert_phenotype(0, 1, &phenotype).unwrap();
    bucket.insert_phenotype(1, 2, &phenotype).unwrap();
    let limit = capacity.execution().required_max_buffer_size().min(
        capacity
            .execution()
            .required_max_storage_buffer_binding_size(),
    );
    for words in [
        bucket.immutable_plan_words().len(),
        bucket.immutable_weight_words().len(),
        bucket.mutable_state_words().len(),
    ] {
        assert!((words as u64) * 4 <= limit);
    }
}

#[test]
fn bucket_rejects_wrong_class_duplicate_slot_and_out_of_range_slot() {
    let capacity = BrainCapacityClass::n512();
    let n512 = compile(capacity.id(), 41);
    let n1024 = compile(BrainCapacityClass::N1024_ID, 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 2).unwrap();
    bucket.insert_phenotype(0, 1, &n512).unwrap();
    assert!(bucket.insert_phenotype(0, 2, &n512).is_err());
    assert!(bucket.insert_phenotype(2, 1, &n512).is_err());
    assert!(bucket.insert_phenotype(1, 1, &n1024).is_err());
}

#[test]
fn slot_record_rejects_reserved_words_and_missing_extension() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 1, &phenotype).unwrap();
    slot.record().validate_slice_a().unwrap();

    let mut reserved = *slot.record();
    reserved.reserved[0] = 1;
    assert!(reserved.validate_slice_a().is_err());

    let mut extension = *slot.record();
    extension.extension_record_offset = GPU_NO_EXTENSION_SENTINEL;
    assert!(extension.validate_slice_a().is_err());
}

fn plan_ranges(ranges: &alife_gpu_backend::GpuSlotWordRanges) -> Vec<Range<u32>> {
    vec![
        ranges.encoder_plan_words.clone(),
        ranges.encoder_assignment_words.clone(),
        ranges.encoder_target_offset_words.clone(),
        ranges.neuron_dynamics_words.clone(),
        ranges.projection_words.clone(),
        ranges.route_metadata_words.clone(),
        ranges.target_offset_words.clone(),
        ranges.source_index_words.clone(),
        ranges.route_index_words.clone(),
        ranges.decoder_plan_words.clone(),
        ranges.decoder_family_words.clone(),
        ranges.decoder_weight_index_words.clone(),
        ranges.memory_channel_plan_words.clone(),
        ranges.memory_weight_index_words.clone(),
        ranges.receptor_words.clone(),
        ranges.synapse_learning_metadata_words.clone(),
        ranges.decoder_eligibility_metadata_words.clone(),
        ranges.replay_plan_identity_words.clone(),
        ranges.sleep_parameter_words.clone(),
    ]
}

fn mutable_ranges(ranges: &alife_gpu_backend::GpuSlotWordRanges) -> Vec<Range<u32>> {
    vec![
        ranges.activation_a_words.clone(),
        ranges.activation_b_words.clone(),
        ranges.accumulator_words.clone(),
        ranges.homeostasis_words.clone(),
        ranges.lifetime_weight_words.clone(),
        ranges.fast_weight_words.clone(),
        ranges.recurrent_eligibility_words.clone(),
        ranges.decoder_eligibility_words.clone(),
        ranges.lifetime_weight_bank_1_words.clone(),
        ranges.fast_weight_bank_1_words.clone(),
        ranges.recurrent_eligibility_bank_1_words.clone(),
        ranges.decoder_eligibility_bank_1_words.clone(),
        ranges.encoded_input_words.clone(),
        ranges.candidate_logit_words.clone(),
        ranges.diagnostic_words.clone(),
        ranges.selection_words.clone(),
        ranges.extension_words.clone(),
        ranges.learning_state_words.clone(),
        ranges.pending_eligibility_words.clone(),
        ranges.replay_event_words.clone(),
        ranges.replay_sample_words.clone(),
        ranges.replay_span_words.clone(),
    ]
}

fn assert_pairwise_disjoint(ranges: &[Range<u32>]) {
    for (index, left) in ranges.iter().enumerate() {
        assert!(left.start <= left.end);
        for right in &ranges[index + 1..] {
            assert!(
                ranges_are_disjoint(left.clone(), right.clone()),
                "{left:?} overlaps {right:?}"
            );
        }
    }
}

fn assert_exact_heap_coverage(ranges: &[Range<u32>], heap_words: usize) {
    let mut sorted = ranges.to_vec();
    sorted.sort_by_key(|range| range.start);
    let mut cursor = 0_u32;
    for range in sorted {
        assert_eq!(range.start, cursor, "gap or overlap before {range:?}");
        cursor = range.end;
    }
    assert_eq!(cursor as usize, heap_words);
}
