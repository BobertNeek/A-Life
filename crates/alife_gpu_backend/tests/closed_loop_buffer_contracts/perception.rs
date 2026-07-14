use alife_core::{BrainCapacityClass, CandidateObservationRef, MAX_ACTION_CANDIDATES};
use alife_gpu_backend::{
    GpuBufferAccess, GpuCandidateRecord, GpuClassBucketBufferRole, GpuClassBucketBuffers,
    GpuClassBucketPlan, GpuClosedLoopError, GpuOffsetDomain, GpuPerceptionHeader,
    GpuPerceptionUpload,
};

use super::support::{compile, perception_fixture};

#[test]
fn candidate_upload_rejects_count_above_the_same_tick_limit() {
    assert!(GpuPerceptionUpload::validate_candidate_count(MAX_ACTION_CANDIDATES).is_ok());
    assert!(GpuPerceptionUpload::validate_candidate_count(MAX_ACTION_CANDIDATES + 1).is_err());
    assert!(GpuPerceptionUpload::validate_candidate_count(0).is_err());
}

#[test]
fn perception_upload_translates_the_validated_same_tick_frame_without_scores() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 7, &phenotype).unwrap();
    let frame = perception_fixture();
    let upload = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();

    assert_eq!(
        upload.header.schema_version,
        u32::from(capacity.execution().gpu_layout_version())
    );
    assert_eq!(upload.header.schema_version, slot.record().schema_version);
    assert_eq!(
        upload.header.class_id,
        phenotype.brain_class_id().raw() as u32
    );
    assert_eq!(upload.header.slot, 0);
    assert_eq!(upload.header.slot_generation, 7);
    assert_eq!(upload.header.neuron_count, 512);
    assert_eq!(upload.header.candidate_count, 2);
    assert_eq!(
        upload.header.microstep_count,
        u32::from(phenotype.microstep_count())
    );
    assert_eq!(upload.header.active_activation_side, 0);
    assert_eq!(upload.header.tick_lo, frame.tick().raw() as u32);
    assert_eq!(upload.header.tick_hi, (frame.tick().raw() >> 32) as u32);
    assert_eq!(upload.header.sensory_offset, 0);
    assert_eq!(upload.header.candidate_offset, 16);
    assert_eq!(upload.header.brain_slot_index, slot.brain_slot_index());
    assert_eq!(upload.header.dispatch_generation_lo, 0);
    assert_eq!(upload.header.dispatch_generation_hi, 0);
    assert_eq!(upload.header.reserved, 0);

    assert_eq!(upload.dispatch_header_words.len(), 16 + 2 * 8);
    assert_eq!(
        GpuPerceptionHeader::from_words(&upload.dispatch_header_words[..16]).unwrap(),
        upload.header
    );
    for (index, expected) in upload.candidates.iter().enumerate() {
        let start = 16 + index * 8;
        assert_eq!(
            GpuCandidateRecord::from_words(&upload.dispatch_header_words[start..start + 8])
                .unwrap(),
            *expected
        );
    }

    let mut expected_payload = Vec::new();
    expected_payload.extend(frame.sensory().channels.as_flat_array().map(f32::to_bits));
    let body = frame.body();
    expected_payload.extend(
        [
            body.pose.translation.x,
            body.pose.translation.y,
            body.pose.translation.z,
            body.pose.rotation.x,
            body.pose.rotation.y,
            body.pose.rotation.z,
            body.pose.rotation.w,
            body.velocity.linear.x,
            body.velocity.linear.y,
            body.velocity.linear.z,
            body.velocity.angular.x,
            body.velocity.angular.y,
            body.velocity.angular.z,
        ]
        .map(f32::to_bits),
    );
    expected_payload.extend(frame.homeostasis().drives.to_array().map(f32::to_bits));
    expected_payload.extend(frame.homeostasis().hormones.to_array().map(f32::to_bits));
    for candidate in frame.candidates() {
        expected_payload.extend(candidate.features.0.map(f32::to_bits));
    }
    assert_eq!(expected_payload.len(), 77 + 2 * 24);
    assert_eq!(upload.frame_payload_words, expected_payload);
    assert_eq!(upload.candidates.len(), 2);

    for (index, (gpu, candidate)) in upload.candidates.iter().zip(frame.candidates()).enumerate() {
        assert_eq!(gpu.action_id, candidate.action_id.raw());
        assert_eq!(gpu.kind, candidate.kind.raw() as u32);
        assert_eq!(gpu.family, candidate.family.raw() as u32);
        assert_eq!(gpu.candidate_index, index as u32);
        assert_eq!(gpu.feature_offset, 77 + index as u32 * 24);
        let expected_slot = match candidate.observation {
            CandidateObservationRef::None => u32::MAX,
            CandidateObservationRef::ObjectSlot(slot) => u32::from(slot),
        };
        assert_eq!(gpu.observation_slot_or_max, expected_slot);
        assert_eq!(
            gpu.confidence_q16,
            (candidate.sensor_confidence.raw() * 65535.0).round() as u32
        );
        assert_eq!(
            gpu.effort_q16,
            (candidate.required_effort.raw() * 65535.0).round() as u32
        );
    }
}

#[test]
fn dynamic_upload_validation_rejects_activation_side_offsets_counts_and_reserved_words() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 7, &phenotype).unwrap();
    let frame = perception_fixture();
    let valid = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
    valid.validate_against(&frame, &slot).unwrap();

    let mut activation = valid.clone();
    activation.header.active_activation_side = 2;
    assert!(activation.validate_against(&frame, &slot).is_err());

    let mut layout = valid.clone();
    layout.header.schema_version += 1;
    assert_eq!(
        layout.validate_against(&frame, &slot).unwrap_err(),
        GpuClosedLoopError::LayoutMismatch
    );

    let mut candidate_offset = valid.clone();
    candidate_offset.header.candidate_offset = 15;
    assert!(candidate_offset.validate_against(&frame, &slot).is_err());

    let mut feature_offset = valid.clone();
    feature_offset.candidates[0].feature_offset = 76;
    assert!(feature_offset.validate_against(&frame, &slot).is_err());

    let mut count = valid.clone();
    count.header.candidate_count += 1;
    assert!(count.validate_against(&frame, &slot).is_err());

    let mut reserved = valid;
    reserved.header.reserved = 1;
    assert!(reserved.validate_against(&frame, &slot).is_err());

    let mut packed_candidate = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
    packed_candidate.dispatch_header_words[16] = 0;
    assert!(packed_candidate.validate_against(&frame, &slot).is_err());

    let mut nonfinite_payload = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
    nonfinite_payload.frame_payload_words[77] = f32::NAN.to_bits();
    assert!(nonfinite_payload.validate_against(&frame, &slot).is_err());
}

#[test]
fn dynamic_upload_rebases_once_and_validates_in_its_explicit_offset_domain() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 7, &phenotype).unwrap();
    let frame = perception_fixture();
    let mut upload = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
    assert_eq!(upload.offset_domain(), GpuOffsetDomain::Local);

    upload.rebase(128, 256).unwrap();
    assert_eq!(
        upload.offset_domain(),
        GpuOffsetDomain::Rebased {
            dispatch_base: 128,
            frame_base: 256
        }
    );
    assert_eq!(upload.header.candidate_offset, 144);
    assert_eq!(upload.header.sensory_offset, 256);
    assert_eq!(upload.candidates[0].feature_offset, 333);
    upload.validate_against(&frame, &slot).unwrap();
    assert_eq!(
        upload.rebase(1, 1).unwrap_err(),
        GpuClosedLoopError::InvalidOffsetDomain
    );
}

#[test]
fn matching_slot_and_header_with_stale_v1_layout_are_rejected() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = compile(capacity.id(), 41);
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 7, &phenotype).unwrap();
    let frame = perception_fixture();
    let upload = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
    let mut slot_record = *slot.record();
    let mut header = upload.header;
    slot_record.schema_version = 1;
    header.schema_version = 1;
    assert_eq!(
        header.validate_layout_for_slot(&slot_record).unwrap_err(),
        GpuClosedLoopError::LayoutMismatch
    );
}

#[test]
fn class_bucket_buffers_expose_exactly_seven_neural_bindings() {
    let manifest = GpuClassBucketBuffers::neural_binding_manifest();
    let expected = [
        (
            GpuClassBucketBufferRole::BrainSlots,
            GpuBufferAccess::ReadOnly,
        ),
        (
            GpuClassBucketBufferRole::PhenotypeIdentities,
            GpuBufferAccess::ReadOnly,
        ),
        (
            GpuClassBucketBufferRole::ImmutablePlanWords,
            GpuBufferAccess::ReadOnly,
        ),
        (
            GpuClassBucketBufferRole::ImmutableWeightWords,
            GpuBufferAccess::ReadOnly,
        ),
        (
            GpuClassBucketBufferRole::DispatchHeaderWords,
            GpuBufferAccess::ReadOnly,
        ),
        (
            GpuClassBucketBufferRole::FramePayloadWords,
            GpuBufferAccess::ReadOnly,
        ),
        (
            GpuClassBucketBufferRole::MutableStateWords,
            GpuBufferAccess::ReadWrite,
        ),
    ];
    assert_eq!(manifest.len(), 7);
    for (binding, (actual, (role, access))) in manifest.iter().zip(expected).enumerate() {
        assert_eq!(actual.group, 0);
        assert_eq!(actual.binding, binding as u32);
        assert_eq!(actual.role, role);
        assert_eq!(actual.access, access);
        assert!(actual.neural_pipeline_bindable);
    }
    let auxiliary = GpuClassBucketBuffers::auxiliary_buffer_manifest();
    assert!(!auxiliary.is_empty());
    assert!(auxiliary.iter().all(|row| !row.neural_pipeline_bindable));
    assert!(auxiliary
        .iter()
        .all(|row| row.role.is_staging_or_readback()));
}
