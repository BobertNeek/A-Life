use std::collections::BTreeMap;

use alife_core::{
    ActivationFunction, ActiveTilePolicy, BiologicalPriority, LobeKind, ProjectionType,
    UpdateCadence,
};
use alife_gpu_backend::{
    GpuBrainSlotRecord, GpuCandidateRecord, GpuDecoderFamilyRecord, GpuDecoderPlanRecord,
    GpuDecoderWeightIndexRecord, GpuEncoderAssignmentRecord, GpuEncoderPlanRecord,
    GpuNeuronDynamicsRecord, GpuPerceptionHeader, GpuPhenotypeIdentityRecord, GpuProjectionRecord,
    GpuRouteMetadataRecord, GpuSelectionRecord, CLOSED_LOOP_ABI_WGSL, GPU_BRAIN_SLOT_RECORD_BYTES,
    GPU_CANDIDATE_RECORD_BYTES, GPU_CLOSED_LOOP_LAYOUT_VERSION, GPU_PERCEPTION_HEADER_BYTES,
    GPU_SELECTION_RECORD_BYTES,
};
use bytemuck::Zeroable;

type ReflectedRecordLayout<'a> = (&'a [(&'a str, u32)], u32);

macro_rules! assert_record_offsets {
    ($record:ty, $expected:ident; $($field:ident),+ $(,)?) => {{
        let actual = [$(
            (stringify!($field), std::mem::offset_of!($record, $field) as u32),
        )+];
        assert_eq!(actual.as_slice(), &$expected, "{} Rust offsets", stringify!($record));
    }};
}

fn assert_pod_zeroable<T: bytemuck::Pod + bytemuck::Zeroable>() {}

#[test]
fn every_shader_visible_record_is_compile_time_pod_and_zeroable() {
    assert_pod_zeroable::<GpuPerceptionHeader>();
    assert_pod_zeroable::<GpuBrainSlotRecord>();
    assert_pod_zeroable::<GpuPhenotypeIdentityRecord>();
    assert_pod_zeroable::<GpuCandidateRecord>();
    assert_pod_zeroable::<GpuSelectionRecord>();
    assert_pod_zeroable::<GpuEncoderPlanRecord>();
    assert_pod_zeroable::<GpuEncoderAssignmentRecord>();
    assert_pod_zeroable::<GpuNeuronDynamicsRecord>();
    assert_pod_zeroable::<GpuProjectionRecord>();
    assert_pod_zeroable::<GpuRouteMetadataRecord>();
    assert_pod_zeroable::<GpuDecoderPlanRecord>();
    assert_pod_zeroable::<GpuDecoderFamilyRecord>();
    assert_pod_zeroable::<GpuDecoderWeightIndexRecord>();
}

#[test]
fn packed_records_decode_from_deliberately_unaligned_word_subslices() {
    let header = GpuPerceptionHeader {
        schema_version: 1,
        class_id: 512,
        slot: 3,
        slot_generation: 9,
        neuron_count: 512,
        candidate_count: 2,
        microstep_count: 3,
        active_activation_side: 1,
        tick_lo: 7,
        tick_hi: 8,
        sensory_offset: 11,
        candidate_offset: 16,
        brain_slot_index: 3,
        reserved: [0; 3],
    };
    let candidate = GpuCandidateRecord {
        action_id: 4,
        kind: 3,
        family: 1,
        candidate_index: 0,
        feature_offset: 77,
        observation_slot_or_max: u32::MAX,
        confidence_q16: 32_000,
        effort_q16: 1_000,
    };
    macro_rules! assert_unaligned_decode {
        ($ty:ty, $value:expr) => {{
            let value: $ty = $value;
            let mut packed = vec![0xA5A5_A5A5];
            packed.extend_from_slice(value.words());
            assert_eq!(<$ty>::from_words(&packed[1..]).unwrap(), value);
        }};
    }
    assert_unaligned_decode!(GpuPerceptionHeader, header);
    assert_unaligned_decode!(GpuCandidateRecord, candidate);
    assert_unaligned_decode!(GpuBrainSlotRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuPhenotypeIdentityRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuSelectionRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuEncoderPlanRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuEncoderAssignmentRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuNeuronDynamicsRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuProjectionRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuRouteMetadataRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuDecoderPlanRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuDecoderFamilyRecord, bytemuck::Zeroable::zeroed());
    assert_unaligned_decode!(GpuDecoderWeightIndexRecord, bytemuck::Zeroable::zeroed());
}

#[test]
fn closed_loop_records_have_stable_aligned_sizes_and_offsets() {
    assert_eq!(GPU_CLOSED_LOOP_LAYOUT_VERSION, 1);
    assert!(CLOSED_LOOP_ABI_WGSL.contains("const GPU_CLOSED_LOOP_LAYOUT_VERSION:u32 = 1u;"));
    assert_eq!(
        std::mem::size_of::<GpuPerceptionHeader>(),
        GPU_PERCEPTION_HEADER_BYTES
    );
    assert_eq!(
        std::mem::size_of::<GpuBrainSlotRecord>(),
        GPU_BRAIN_SLOT_RECORD_BYTES
    );
    assert_eq!(
        std::mem::size_of::<GpuCandidateRecord>(),
        GPU_CANDIDATE_RECORD_BYTES
    );
    assert_eq!(
        std::mem::size_of::<GpuSelectionRecord>(),
        GPU_SELECTION_RECORD_BYTES
    );
    assert_eq!(std::mem::size_of::<GpuPhenotypeIdentityRecord>(), 32);
    assert_eq!(GPU_PERCEPTION_HEADER_BYTES, 64);
    assert_eq!(GPU_BRAIN_SLOT_RECORD_BYTES, 144);
    assert_eq!(GPU_CANDIDATE_RECORD_BYTES, 32);
    assert_eq!(GPU_SELECTION_RECORD_BYTES, 48);

    assert_eq!(std::mem::align_of::<GpuPerceptionHeader>(), 16);
    assert_eq!(std::mem::align_of::<GpuBrainSlotRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuCandidateRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuSelectionRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuPhenotypeIdentityRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuEncoderPlanRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuEncoderAssignmentRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuNeuronDynamicsRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuProjectionRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuRouteMetadataRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuDecoderPlanRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuDecoderFamilyRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuDecoderWeightIndexRecord>(), 16);

    assert_eq!(std::mem::size_of::<GpuEncoderPlanRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuEncoderAssignmentRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuNeuronDynamicsRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuProjectionRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuRouteMetadataRecord>(), 48);
    assert_eq!(std::mem::size_of::<GpuDecoderPlanRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuDecoderFamilyRecord>(), 32);
    assert_eq!(std::mem::size_of::<GpuDecoderWeightIndexRecord>(), 16);

    assert_record_offsets!(GpuPerceptionHeader, PERCEPTION_FIELDS;
        schema_version, class_id, slot, slot_generation, neuron_count, candidate_count,
        microstep_count, active_activation_side, tick_lo, tick_hi, sensory_offset,
        candidate_offset, brain_slot_index, reserved);
    assert_record_offsets!(GpuBrainSlotRecord, BRAIN_SLOT_FIELDS;
        schema_version, class_id, slot, slot_generation, neuron_count, microstep_count,
        synapse_count, recurrent_synapse_count, encoder_plan_offset, neuron_dynamics_offset,
        projection_offset, route_metadata_offset, target_offsets_offset, source_indices_offset,
        route_indices_offset, decoder_plan_offset, decoder_family_offset,
        decoder_weight_indices_offset, genetic_weight_offset, alpha_offset, activation_a_offset,
        activation_b_offset, accumulator_offset, lifetime_weight_offset, fast_weight_offset,
        recurrent_eligibility_offset, decoder_eligibility_offset, encoded_input_offset,
        candidate_logit_offset, diagnostic_offset, selection_offset, neuron_homeostasis_offset,
        extension_record_offset, reserved);
    assert_record_offsets!(GpuPhenotypeIdentityRecord, IDENTITY_FIELDS; phenotype_hash);
    assert_record_offsets!(GpuCandidateRecord, CANDIDATE_FIELDS;
        action_id, kind, family, candidate_index, feature_offset, observation_slot_or_max,
        confidence_q16, effort_q16);
    assert_record_offsets!(GpuSelectionRecord, SELECTION_FIELDS;
        slot, slot_generation, candidate_index, logit_bits, confidence_q16, status, active_tiles,
        active_synapses, finite_rejections, dispatch_generation_lo, dispatch_generation_hi,
        active_activation_side);
    assert_record_offsets!(GpuEncoderPlanRecord, ENCODER_PLAN_FIELDS;
        schema_version, sensor_profile_raw, assignment_offset, assignment_count,
        target_offsets_offset, sensory_lane_count, body_lane_count, homeostasis_lane_count);
    assert_record_offsets!(GpuEncoderAssignmentRecord, ENCODER_ASSIGNMENT_FIELDS;
        source_group_raw, source_index, target_neuron, reserved0, scale_bits, bias_bits,
        clamp_min_bits, clamp_max_bits);
    assert_record_offsets!(GpuNeuronDynamicsRecord, DYNAMICS_FIELDS;
        bias_bits, leak_bits, activation_raw, homeostatic_gain_bits, activity_ema_decay_bits,
        metabolic_decay_bits, reserved0, reserved1);
    assert_record_offsets!(GpuProjectionRecord, PROJECTION_FIELDS;
        route_index, source_lobe_raw, target_lobe_raw, synapse_start, synapse_count,
        active_tile_count, reserved0, reserved1);
    assert_record_offsets!(GpuRouteMetadataRecord, ROUTE_FIELDS;
        route_index, projection_type_raw, active_tile_policy_raw, update_cadence_raw,
        biological_priority_raw, delay_microsteps, source_start, source_count, target_start,
        target_count, reserved0, reserved1);
    assert_record_offsets!(GpuDecoderPlanRecord, DECODER_PLAN_FIELDS;
        schema_version, motor_start, motor_width, feature_count, flattened_input_lane_count,
        family_offset, family_count, decoder_synapse_count);
    assert_record_offsets!(GpuDecoderFamilyRecord, DECODER_FAMILY_FIELDS;
        family_raw, bias_bits, decoder_synapse_start, decoder_synapse_count, weight_index_start,
        weight_index_count, reserved0, reserved1);
    assert_record_offsets!(GpuDecoderWeightIndexRecord, DECODER_WEIGHT_FIELDS;
        global_synapse_id, input_lane, motor_index, reserved0);
}

#[test]
fn slice_a_records_reject_matching_but_unsupported_layout_versions() {
    let mut slot = GpuBrainSlotRecord::zeroed();
    slot.schema_version = GPU_CLOSED_LOOP_LAYOUT_VERSION + 1;
    slot.slot_generation = 1;
    slot.microstep_count = 1;
    slot.extension_record_offset = u32::MAX;
    assert!(slot.validate_slice_a().is_err());
}

#[test]
fn closed_loop_enum_rows_round_trip_and_unknown_values_are_rejected() {
    for value in [
        ActivationFunction::Identity,
        ActivationFunction::Relu,
        ActivationFunction::Tanh,
        ActivationFunction::Logistic,
    ] {
        assert_eq!(
            ActivationFunction::try_from_raw(value.raw()).unwrap(),
            value
        );
    }
    assert!(ActivationFunction::try_from_raw(4).is_err());

    for raw in 1_u16..=17 {
        let value = LobeKind::try_from_raw(raw).unwrap();
        assert_eq!(value.raw(), raw);
    }
    assert!(LobeKind::try_from_raw(0).is_err());
    assert!(LobeKind::try_from_raw(18).is_err());

    for raw in 0_u8..=6 {
        let value = ProjectionType::try_from_raw(raw).unwrap();
        assert_eq!(value.raw(), raw);
    }
    assert!(ProjectionType::try_from_raw(7).is_err());

    for raw in 0_u8..=3 {
        let value = ActiveTilePolicy::try_from_raw(raw).unwrap();
        assert_eq!(value.raw(), raw);
        let value = BiologicalPriority::try_from_raw(raw).unwrap();
        assert_eq!(value.raw(), raw);
    }
    assert!(ActiveTilePolicy::try_from_raw(4).is_err());
    assert!(BiologicalPriority::try_from_raw(4).is_err());

    for raw in 0_u8..=6 {
        let value = UpdateCadence::try_from_raw(raw).unwrap();
        assert_eq!(value.raw(), raw);
    }
    assert!(UpdateCadence::try_from_raw(7).is_err());
}

#[test]
fn naga_reflection_matches_every_closed_loop_wgsl_record() {
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_ABI_WGSL).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).unwrap();

    let expected: BTreeMap<&str, ReflectedRecordLayout<'_>> = [
        ("GpuPerceptionHeader", (&PERCEPTION_FIELDS[..], 64)),
        ("GpuBrainSlotRecord", (&BRAIN_SLOT_FIELDS[..], 144)),
        ("GpuPhenotypeIdentityRecord", (&IDENTITY_FIELDS[..], 32)),
        ("GpuCandidateRecord", (&CANDIDATE_FIELDS[..], 32)),
        ("GpuSelectionRecord", (&SELECTION_FIELDS[..], 48)),
        ("GpuEncoderPlanRecord", (&ENCODER_PLAN_FIELDS[..], 32)),
        (
            "GpuEncoderAssignmentRecord",
            (&ENCODER_ASSIGNMENT_FIELDS[..], 32),
        ),
        ("GpuNeuronDynamicsRecord", (&DYNAMICS_FIELDS[..], 32)),
        ("GpuProjectionRecord", (&PROJECTION_FIELDS[..], 32)),
        ("GpuRouteMetadataRecord", (&ROUTE_FIELDS[..], 48)),
        ("GpuDecoderPlanRecord", (&DECODER_PLAN_FIELDS[..], 32)),
        ("GpuDecoderFamilyRecord", (&DECODER_FAMILY_FIELDS[..], 32)),
        (
            "GpuDecoderWeightIndexRecord",
            (&DECODER_WEIGHT_FIELDS[..], 16),
        ),
    ]
    .into_iter()
    .collect();

    for (handle, ty) in module.types.iter() {
        let Some(name) = ty.name.as_deref() else {
            continue;
        };
        let Some((fields, size)) = expected.get(name) else {
            continue;
        };
        let naga::TypeInner::Struct { members, span } = &ty.inner else {
            panic!("{name} is not a WGSL struct");
        };
        assert_eq!(*span, *size, "{name} declared span");
        assert_eq!(layouter[handle].size, *size, "{name} reflected size");
        assert_eq!(members.len(), fields.len(), "{name} member count");
        for (member, (field_name, offset)) in members.iter().zip(fields.iter()) {
            assert_eq!(
                member.name.as_deref(),
                Some(*field_name),
                "{name} field name"
            );
            assert_eq!(member.offset, *offset, "{name}.{field_name} offset");
        }
    }
    for name in expected.keys() {
        assert!(
            module
                .types
                .iter()
                .any(|(_, ty)| ty.name.as_deref() == Some(name)),
            "missing {name}"
        );
    }
}

const PERCEPTION_FIELDS: [(&str, u32); 14] = sequential(&[
    "schema_version",
    "class_id",
    "slot",
    "slot_generation",
    "neuron_count",
    "candidate_count",
    "microstep_count",
    "active_activation_side",
    "tick_lo",
    "tick_hi",
    "sensory_offset",
    "candidate_offset",
    "brain_slot_index",
    "reserved",
]);
const BRAIN_SLOT_FIELDS: [(&str, u32); 34] = sequential(&[
    "schema_version",
    "class_id",
    "slot",
    "slot_generation",
    "neuron_count",
    "microstep_count",
    "synapse_count",
    "recurrent_synapse_count",
    "encoder_plan_offset",
    "neuron_dynamics_offset",
    "projection_offset",
    "route_metadata_offset",
    "target_offsets_offset",
    "source_indices_offset",
    "route_indices_offset",
    "decoder_plan_offset",
    "decoder_family_offset",
    "decoder_weight_indices_offset",
    "genetic_weight_offset",
    "alpha_offset",
    "activation_a_offset",
    "activation_b_offset",
    "accumulator_offset",
    "lifetime_weight_offset",
    "fast_weight_offset",
    "recurrent_eligibility_offset",
    "decoder_eligibility_offset",
    "encoded_input_offset",
    "candidate_logit_offset",
    "diagnostic_offset",
    "selection_offset",
    "neuron_homeostasis_offset",
    "extension_record_offset",
    "reserved",
]);
const IDENTITY_FIELDS: [(&str, u32); 1] = [("phenotype_hash", 0)];
const CANDIDATE_FIELDS: [(&str, u32); 8] = sequential(&[
    "action_id",
    "kind",
    "family",
    "candidate_index",
    "feature_offset",
    "observation_slot_or_max",
    "confidence_q16",
    "effort_q16",
]);
const SELECTION_FIELDS: [(&str, u32); 12] = sequential(&[
    "slot",
    "slot_generation",
    "candidate_index",
    "logit_bits",
    "confidence_q16",
    "status",
    "active_tiles",
    "active_synapses",
    "finite_rejections",
    "dispatch_generation_lo",
    "dispatch_generation_hi",
    "active_activation_side",
]);
const ENCODER_PLAN_FIELDS: [(&str, u32); 8] = sequential(&[
    "schema_version",
    "sensor_profile_raw",
    "assignment_offset",
    "assignment_count",
    "target_offsets_offset",
    "sensory_lane_count",
    "body_lane_count",
    "homeostasis_lane_count",
]);
const ENCODER_ASSIGNMENT_FIELDS: [(&str, u32); 8] = sequential(&[
    "source_group_raw",
    "source_index",
    "target_neuron",
    "reserved0",
    "scale_bits",
    "bias_bits",
    "clamp_min_bits",
    "clamp_max_bits",
]);
const DYNAMICS_FIELDS: [(&str, u32); 8] = sequential(&[
    "bias_bits",
    "leak_bits",
    "activation_raw",
    "homeostatic_gain_bits",
    "activity_ema_decay_bits",
    "metabolic_decay_bits",
    "reserved0",
    "reserved1",
]);
const PROJECTION_FIELDS: [(&str, u32); 8] = sequential(&[
    "route_index",
    "source_lobe_raw",
    "target_lobe_raw",
    "synapse_start",
    "synapse_count",
    "active_tile_count",
    "reserved0",
    "reserved1",
]);
const ROUTE_FIELDS: [(&str, u32); 12] = sequential(&[
    "route_index",
    "projection_type_raw",
    "active_tile_policy_raw",
    "update_cadence_raw",
    "biological_priority_raw",
    "delay_microsteps",
    "source_start",
    "source_count",
    "target_start",
    "target_count",
    "reserved0",
    "reserved1",
]);
const DECODER_PLAN_FIELDS: [(&str, u32); 8] = sequential(&[
    "schema_version",
    "motor_start",
    "motor_width",
    "feature_count",
    "flattened_input_lane_count",
    "family_offset",
    "family_count",
    "decoder_synapse_count",
]);
const DECODER_FAMILY_FIELDS: [(&str, u32); 8] = sequential(&[
    "family_raw",
    "bias_bits",
    "decoder_synapse_start",
    "decoder_synapse_count",
    "weight_index_start",
    "weight_index_count",
    "reserved0",
    "reserved1",
]);
const DECODER_WEIGHT_FIELDS: [(&str, u32); 4] = sequential(&[
    "global_synapse_id",
    "input_lane",
    "motor_index",
    "reserved0",
]);

const fn sequential<const N: usize>(names: &[&'static str; N]) -> [(&'static str, u32); N] {
    let mut out = [("", 0); N];
    let mut index = 0;
    while index < N {
        out[index] = (names[index], index as u32 * 4);
        index += 1;
    }
    out
}
