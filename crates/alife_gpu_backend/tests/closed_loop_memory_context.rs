//! Candidate-local GPU episodic-context ABI, shader, and causal behavior contracts.

mod support;

use alife_core::{
    BrainCapacityClass, Confidence, DecoderHeadKind, MemoryBank, MemoryBankConfig,
    PerceptionFrameDraft, SensorProfile,
};
#[cfg(feature = "gpu-tests")]
use alife_core::{
    BrainGenome, DecisionSnapshot, DevelopmentState, EndocrineDelta, ExperiencePatch,
    ExperiencePatchBuilder, ExperienceSequenceId, HomeostaticDelta, NeuralActionSelection,
    NormalizedScalar, PhysicalActionOutcome, PhysicalContactKind, PostActionOutcome,
    PreActionSnapshot, SignedValence, Tick, Vec3f,
};
use alife_gpu_backend::{
    GpuBrainSlotExtensionRecord, GpuBufferAccess, GpuCandidateMemoryRecord,
    GpuClassBucketBufferRole, GpuClassBucketBuffers, GpuClassBucketPlan, GpuMemoryChannelPlan,
    GpuMemoryContextHeader, GpuMemoryContextUpload, GpuPerceptionUpload, GpuPhenotypeUpload,
    CLOSED_LOOP_ELIGIBILITY_WGSL, CLOSED_LOOP_MEMORY_CONTEXT_WGSL, GPU_ACTIVE_DISPATCH_ROW_WORDS,
    GPU_MEMORY_CONTEXT_HEADER_WORDS, GPU_NO_EXTENSION_SENTINEL, GPU_PERCEPTION_DISPATCH_ROW_WORDS,
};
#[cfg(feature = "gpu-tests")]
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput, GpuClosedLoopMemoryTickInput,
    GpuClosedLoopRuntimeConfig, GpuClosedLoopTick,
};

#[cfg(feature = "gpu-tests")]
fn memory_draft_from_frame(frame: &alife_core::PerceptionFrame) -> PerceptionFrameDraft {
    PerceptionFrameDraft::new(
        frame.organism_id(),
        frame.tick(),
        frame.sensor_profile(),
        frame.sensory().clone(),
        frame.body(),
        frame.homeostasis().clone(),
        frame.candidates().to_vec(),
        frame.profile_provenance(),
        frame.grounded_object_slots().to_vec(),
    )
    .unwrap()
}

#[cfg(feature = "gpu-tests")]
fn painful_memory_patch(
    frame: &alife_core::PerceptionFrame,
    recall: &alife_core::FinalizedMemoryRecall,
    class_id: alife_core::BrainClassId,
    phenotype_hash: alife_core::PhenotypeHash,
) -> ExperiencePatch {
    let sequence = ExperienceSequenceId(9_001);
    let genome = BrainGenome::scaffold(0xC600_1001, class_id);
    let development = DevelopmentState::new(
        genome.id,
        frame.tick(),
        NormalizedScalar::new(0.35).unwrap(),
    );
    let candidate = &frame.candidates()[0];
    let selection = NeuralActionSelection {
        candidate_index: candidate.candidate_index,
        logit: 0.5,
        confidence: candidate.sensor_confidence,
        active_tiles: 8,
        active_synapses: 64,
    };
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence,
        class_id,
        phenotype_hash,
        genome.id,
        genome.schema_version,
        development,
        frame.clone(),
    )
    .unwrap();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence,
        phenotype_hash,
        1,
        0,
        frame,
        selection,
        candidate
            .to_command(frame.organism_id(), candidate.sensor_confidence)
            .unwrap(),
    )
    .unwrap()
    .with_finalized_memory_recall(frame, recall, 0)
    .unwrap();
    let outcome = PostActionOutcome::new(
        frame.organism_id(),
        sequence,
        Tick::new(frame.tick().raw() + 1),
        false,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta {
            drives: alife_core::DriveDelta {
                fear: 0.7,
                pain: 0.9,
                brain_atp: -0.2,
                ..alife_core::DriveDelta::zero()
            },
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(-0.8).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.9).unwrap(),
        SignedValence::new(-0.2).unwrap(),
        NormalizedScalar::new(0.7).unwrap(),
    )
    .unwrap();
    ExperiencePatchBuilder::new(sequence)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}

#[cfg(feature = "gpu-tests")]
fn painful_patch_for_gpu_tick(
    handle: alife_gpu_backend::GpuBrainHandle,
    frame: &alife_core::PerceptionFrame,
    recall: &alife_core::FinalizedMemoryRecall,
    tick: &GpuClosedLoopTick,
    sequence: ExperienceSequenceId,
) -> ExperiencePatch {
    let genome = BrainGenome::scaffold(0xC600_1002, handle.class_id());
    let development = DevelopmentState::new(
        genome.id,
        frame.tick(),
        NormalizedScalar::new(0.35).unwrap(),
    );
    let candidate = &frame.candidates()[usize::from(tick.selection.candidate_index)];
    let pre_action = PreActionSnapshot::from_neural_frame(
        sequence,
        handle.class_id(),
        handle.phenotype_hash(),
        genome.id,
        genome.schema_version,
        development,
        frame.clone(),
    )
    .unwrap();
    let decision = DecisionSnapshot::from_neural_selection(
        sequence,
        handle.phenotype_hash(),
        tick.dispatch_generation,
        tick.active_activation_side,
        frame,
        tick.selection,
        candidate
            .to_command(frame.organism_id(), tick.selection.confidence)
            .unwrap(),
    )
    .unwrap()
    .with_finalized_memory_recall(frame, recall, tick.selection.candidate_index)
    .unwrap();
    let outcome = PostActionOutcome::new(
        frame.organism_id(),
        sequence,
        Tick::new(frame.tick().raw() + 1),
        false,
        PhysicalActionOutcome {
            contact: PhysicalContactKind::None,
            target_entity: None,
            displacement: Vec3f::ZERO,
            collision_normal: None,
            energy_cost: NormalizedScalar::new(0.1).unwrap(),
        },
        HomeostaticDelta {
            drives: alife_core::DriveDelta {
                fear: 0.7,
                pain: 0.9,
                brain_atp: -0.2,
                ..alife_core::DriveDelta::zero()
            },
            hormones: EndocrineDelta::zero(),
        },
        SignedValence::new(-0.8).unwrap(),
        NormalizedScalar::new(0.0).unwrap(),
        NormalizedScalar::new(0.9).unwrap(),
        SignedValence::new(-0.2).unwrap(),
        NormalizedScalar::new(0.7).unwrap(),
    )
    .unwrap();
    ExperiencePatchBuilder::new(sequence)
        .record_pre_action(pre_action)
        .unwrap()
        .record_decision(decision)
        .unwrap()
        .record_outcome(outcome)
        .unwrap()
        .seal()
        .unwrap()
}

#[cfg(feature = "gpu-tests")]
fn conditioned_memory_frame(
    organism_raw: u64,
    phenotype: &alife_core::BrainPhenotype,
) -> (
    alife_core::PerceptionFrame,
    alife_core::FinalizedMemoryRecall,
) {
    conditioned_memory_frame_with_candidate_count(organism_raw, phenotype, 2)
}

#[cfg(feature = "gpu-tests")]
fn conditioned_memory_frame_with_candidate_count(
    organism_raw: u64,
    phenotype: &alife_core::BrainPhenotype,
    candidate_count: usize,
) -> (
    alife_core::PerceptionFrame,
    alife_core::FinalizedMemoryRecall,
) {
    let mut bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let first_source = support::perception_frame_for_profile_at_tick(
        organism_raw,
        900,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        candidate_count,
    );
    let first_draft = memory_draft_from_frame(&first_source);
    let (first_frame, first_recall) = bank
        .recall_frame(&first_draft)
        .unwrap()
        .finalize(first_draft)
        .unwrap();
    bank.observe_sealed_patch(&painful_memory_patch(
        &first_frame,
        &first_recall,
        phenotype.brain_class_id(),
        phenotype.phenotype_hash(),
    ))
    .unwrap();

    let probe_source = support::perception_frame_for_profile_at_tick(
        organism_raw,
        902,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        candidate_count,
    );
    let probe_draft = memory_draft_from_frame(&probe_source);
    let (frame, recall) = bank
        .recall_frame(&probe_draft)
        .unwrap()
        .finalize(probe_draft)
        .unwrap();
    assert!(frame.context().values().iter().any(|value| *value != 0.0));
    (frame, recall)
}

#[cfg(feature = "gpu-tests")]
fn memory_lane_sample(frame: &alife_core::PerceptionFrame, candidate: usize, lane: u32) -> f32 {
    let row = &frame.context().values()[candidate * 16..candidate * 16 + 16];
    match lane {
        24..=31 => row[(lane - 24) as usize] * row[12],
        32..=35 => row[8 + (lane - 32) as usize] * row[13],
        _ => panic!("not a memory decoder lane: {lane}"),
    }
}

fn wgsl_struct_members_and_size(source: &str, name: &str) -> (Vec<String>, u32) {
    let module = naga::front::wgsl::parse_str(source).unwrap();
    let validation = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module);
    if let Err(error) = validation {
        let entry = module
            .entry_points
            .iter()
            .find(|entry| entry.name == "add_candidate_memory_context")
            .unwrap();
        for (handle, expression) in entry.function.expressions.iter() {
            if (138..=150).contains(&handle.index()) {
                eprintln!("expression {}: {expression:?}", handle.index());
            }
        }
        panic!("WGSL validation failed: {error:?}");
    }
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).unwrap();
    let (handle, ty) = module
        .types
        .iter()
        .find(|(_, ty)| ty.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("missing WGSL struct {name}"));
    let naga::TypeInner::Struct { members, span } = &ty.inner else {
        panic!("{name} is not a WGSL struct");
    };
    assert_eq!(layouter[handle].size, *span, "{name} reflected size");
    (
        members
            .iter()
            .map(|member| member.name.clone().unwrap())
            .collect(),
        *span,
    )
}

#[test]
fn candidate_memory_records_have_the_frozen_host_layout() {
    assert_eq!(std::mem::size_of::<GpuCandidateMemoryRecord>(), 64);
    assert_eq!(std::mem::size_of::<GpuMemoryContextHeader>(), 64);
    assert_eq!(std::mem::size_of::<GpuBrainSlotExtensionRecord>(), 80);
    assert_eq!(std::mem::size_of::<GpuMemoryChannelPlan>(), 32);
    assert_eq!(std::mem::align_of::<GpuCandidateMemoryRecord>(), 16);
    assert_eq!(std::mem::align_of::<GpuMemoryContextHeader>(), 16);
    assert_eq!(std::mem::align_of::<GpuMemoryChannelPlan>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuCandidateMemoryRecord, target_latent),
        16,
    );
    assert_eq!(
        std::mem::offset_of!(GpuCandidateMemoryRecord, family_value),
        48,
    );
    assert_eq!(
        std::mem::offset_of!(GpuMemoryContextHeader, brain_slot_index),
        48,
    );
    assert_eq!(
        std::mem::offset_of!(GpuMemoryContextHeader, decoder_learning_input_offset),
        52,
    );
    assert_eq!(
        std::mem::offset_of!(GpuMemoryContextHeader, perception_header_index),
        56,
    );
    assert_eq!(
        std::mem::offset_of!(GpuMemoryChannelPlan, max_candidate_gain),
        16,
    );
    assert_eq!(std::mem::offset_of!(GpuMemoryChannelPlan, reserved), 24);
}

#[test]
fn memory_wgsl_struct_lane_order_matches_the_host_abi() {
    let (members, size) =
        wgsl_struct_members_and_size(CLOSED_LOOP_MEMORY_CONTEXT_WGSL, "GpuCandidateMemoryRecord");
    assert_eq!(
        members,
        [
            "candidate_index",
            "target_confidence",
            "family_confidence",
            "source_counts_packed",
            "target_latent",
            "family_value",
        ]
    );
    assert_eq!(size, 64);

    let (members, size) =
        wgsl_struct_members_and_size(CLOSED_LOOP_MEMORY_CONTEXT_WGSL, "GpuMemoryContextHeader");
    assert_eq!(
        members,
        [
            "schema_version",
            "class_id",
            "slot",
            "slot_generation",
            "tick_lo",
            "tick_hi",
            "candidate_count",
            "memory_context_offset",
            "candidate_offset",
            "profile_id",
            "profile_schema_version",
            "sensory_abi_version",
            "brain_slot_index",
            "decoder_learning_input_offset",
            "perception_header_index",
            "reserved",
        ]
    );
    assert_eq!(size, 64);

    let (members, size) =
        wgsl_struct_members_and_size(CLOSED_LOOP_MEMORY_CONTEXT_WGSL, "GpuMemoryChannelPlan");
    assert_eq!(
        members,
        [
            "schema_version",
            "target_latent_lane_start",
            "family_value_lane_start",
            "decoder_input_stride",
            "max_candidate_gain",
            "memory_decoder_synapse_count",
            "reserved",
        ]
    );
    assert_eq!(size, 32);
}

#[test]
fn memory_shader_exposes_only_candidate_local_decoder_authority() {
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_MEMORY_CONTEXT_WGSL).unwrap();
    assert!(module
        .entry_points
        .iter()
        .any(|entry| entry.name == "add_candidate_memory_context"));
    assert!(!module
        .entry_points
        .iter()
        .any(|entry| entry.name == "encode_memory_state"));
    assert!(!CLOSED_LOOP_MEMORY_CONTEXT_WGSL.contains("encoded_inputs"));
    assert!(!CLOSED_LOOP_MEMORY_CONTEXT_WGSL.contains("activations["));
    assert_eq!(
        CLOSED_LOOP_MEMORY_CONTEXT_WGSL
            .matches("struct GpuCandidateMemoryRecord")
            .count(),
        1,
    );
    assert!(!CLOSED_LOOP_MEMORY_CONTEXT_WGSL.contains("GpuBrainMemoryExtensionRecord"));
}

#[test]
fn decoder_eligibility_uses_the_exact_derivative_for_each_known_head() {
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL.contains("const DECODER_HEAD_MEMORY_CONTEXT:u32 = 2u;"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL.contains("const DECODER_HEAD_SPEECH_PAYLOAD:u32 = 3u;"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL
        .contains("metadata.decoder_head == DECODER_HEAD_ACTION_CANDIDATE"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL.contains("metadata.input_lane < CANDIDATE_FEATURE_COUNT"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL
        .contains("metadata.decoder_head == DECODER_HEAD_MEMORY_CONTEXT"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL.contains("metadata.input_lane >= CANDIDATE_FEATURE_COUNT"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL
        .contains("local = bitcast<f32>(frame_payload_words[feature_index]);"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL
        .contains("metadata.decoder_head == DECODER_HEAD_SPEECH_PAYLOAD"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL.contains("ELIGIBILITY_DIAGNOSTIC_UNKNOWN_DECODER_HEAD"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL.contains("atomicOr("));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL
        .contains("&mutable_state_words[brain.diagnostic_offset + ELIGIBILITY_DIAGNOSTIC_LANE]"));
    assert!(CLOSED_LOOP_ELIGIBILITY_WGSL
        .contains("& ELIGIBILITY_DIAGNOSTIC_UNKNOWN_DECODER_HEAD) != 0u"));
}

#[test]
fn dispatch_validation_rechecks_the_rebased_memory_header_and_record_span() {
    let source = include_str!("../src/closed_loop_pipeline.rs");
    for required in [
        "batch.memory_context_bindings.len() != batch.headers.len()",
        "let memory_header_start = learning_start + GPU_LEARNING_HEADER_WORDS;",
        "GpuMemoryContextHeader::from_words",
        "memory_header.perception_header_index",
        "memory_header.memory_context_offset",
        "GpuCandidateMemoryRecord::from_words",
        "memory_binding.base_frame_digest",
        "memory_binding.context_digest",
        "memory_binding.final_frame_digest",
    ] {
        assert!(
            source.contains(required),
            "missing dispatch memory check: {required}"
        );
    }
}

#[test]
fn shared_frame_payload_is_writable_only_for_candidate_local_context_materialization() {
    let binding = GpuClassBucketBuffers::neural_binding_manifest()
        .into_iter()
        .find(|entry| entry.role == GpuClassBucketBufferRole::FramePayloadWords)
        .unwrap();
    assert_eq!(binding.group, 0);
    assert_eq!(binding.binding, 5);
    assert_eq!(binding.access, GpuBufferAccess::ReadWrite);
}

#[test]
fn active_dispatch_rows_reserve_one_exact_memory_context_header() {
    assert_eq!(GPU_PERCEPTION_DISPATCH_ROW_WORDS, 272);
    assert_eq!(GPU_MEMORY_CONTEXT_HEADER_WORDS, 16);
    assert_eq!(GPU_ACTIVE_DISPATCH_ROW_WORDS, 308);
    assert_eq!(
        GPU_ACTIVE_DISPATCH_ROW_WORDS - GPU_PERCEPTION_DISPATCH_ROW_WORDS,
        20 + GPU_MEMORY_CONTEXT_HEADER_WORDS,
    );
}

#[test]
fn perception_payload_uses_the_compiled_memory_decoder_stride() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = support::phenotype_for_capacity_at_maturation(
        capacity.clone(),
        0xC600_0004,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 1, &phenotype).unwrap();
    let frame =
        support::perception_frame_for_profile(812, SensorProfile::GroundedObjectSlotsV1, true, 2);
    let upload = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();

    assert_eq!(slot.decoder_input_stride(), 36);
    assert_eq!(upload.candidates[0].feature_offset, 77);
    assert_eq!(upload.candidates[1].feature_offset, 113);
    assert_eq!(upload.frame_payload_words.len(), 77 + 2 * 36);
    for candidate in 0..2 {
        let start = 77 + candidate * 36;
        assert!(upload.frame_payload_words[start + 24..start + 36]
            .iter()
            .all(|word| *word == 0));
    }
}

#[test]
fn phenotype_upload_owns_a_complete_family_major_memory_weight_map() {
    for (capacity, expected_memory_rows) in [
        (BrainCapacityClass::n512(), 96_usize),
        (BrainCapacityClass::n1024(), 96),
        (BrainCapacityClass::n2048(), 4_096),
    ] {
        let phenotype = support::phenotype_for_capacity_at_maturation(
            capacity,
            0xC600_0001,
            0.35,
            SensorProfile::GroundedObjectSlotsV1,
        );
        let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
        assert_eq!(upload.memory_channel_plans.len(), 1);
        let plan = upload.memory_channel_plans[0];
        assert_eq!(plan.target_latent_lane_start, 24);
        assert_eq!(plan.family_value_lane_start, 32);
        assert_eq!(plan.decoder_input_stride, 36);
        assert_eq!(
            plan.memory_decoder_synapse_count as usize,
            expected_memory_rows,
        );
        assert_eq!(upload.memory_weight_indices.len(), expected_memory_rows);
        assert_eq!(
            upload
                .decoder_eligibility_metadata
                .iter()
                .filter(|row| row.decoder_head == DecoderHeadKind::MemoryContext.raw())
                .count(),
            expected_memory_rows,
        );

        let rows_per_family = expected_memory_rows / 8;
        let recurrent = upload.source_indices.len() as u32;
        let mut unique = std::collections::BTreeSet::new();
        for family in 0_u32..8 {
            for &local_synapse in &upload.memory_weight_indices
                [family as usize * rows_per_family..(family as usize + 1) * rows_per_family]
            {
                assert!(unique.insert(local_synapse));
                assert!(local_synapse >= recurrent);
                let metadata =
                    upload.decoder_eligibility_metadata[(local_synapse - recurrent) as usize];
                assert_eq!(metadata.global_synapse_id, local_synapse);
                assert_eq!(metadata.decoder_head, DecoderHeadKind::MemoryContext.raw());
                assert_eq!(metadata.family, family);
                assert!((24..36).contains(&metadata.input_lane));
            }
        }
    }
}

#[test]
fn slot_extension_points_at_the_uploaded_memory_plan_and_map() {
    let capacity = BrainCapacityClass::n512();
    let phenotype = support::phenotype_for_capacity_at_maturation(
        capacity.clone(),
        0xC600_0002,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 1, &phenotype).unwrap();
    let extension_range = &slot.word_ranges().extension_words;
    let extension = GpuBrainSlotExtensionRecord::from_words(
        &bucket.mutable_state_words()[extension_range.start as usize..extension_range.end as usize],
    )
    .unwrap();
    assert_ne!(extension.memory_plan_offset, GPU_NO_EXTENSION_SENTINEL);
    assert_ne!(
        extension.memory_weight_map_offset,
        GPU_NO_EXTENSION_SENTINEL
    );
    let plan_start = extension.memory_plan_offset as usize;
    let plan = GpuMemoryChannelPlan::from_words(
        &bucket.immutable_plan_words()[plan_start..plan_start + 8],
    )
    .unwrap();
    assert_eq!(plan.memory_decoder_synapse_count, 96);
    let map_start = extension.memory_weight_map_offset as usize;
    let map_end = map_start + plan.memory_decoder_synapse_count as usize;
    assert!(map_end <= bucket.immutable_plan_words().len());
    assert!(bucket.immutable_plan_words()[map_start..map_end]
        .iter()
        .all(|local| *local >= slot.record().recurrent_synapse_count
            && *local < slot.record().synapse_count));
}

#[test]
fn finalized_memory_upload_binds_base_context_final_and_perception_header_identity() {
    let source =
        support::perception_frame_for_profile(811, SensorProfile::GroundedObjectSlotsV1, true, 2);
    let draft = PerceptionFrameDraft::new(
        source.organism_id(),
        source.tick(),
        source.sensor_profile(),
        source.sensory().clone(),
        source.body(),
        source.homeostasis().clone(),
        source.candidates().to_vec(),
        source.profile_provenance(),
        source.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let (frame, recall) = bank.recall_frame(&draft).unwrap().finalize(draft).unwrap();

    let capacity = BrainCapacityClass::n512();
    let phenotype = support::phenotype_for_capacity_at_maturation(
        capacity.clone(),
        0xC600_0003,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 1, &phenotype).unwrap();
    let perception = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
    let upload = GpuMemoryContextUpload::try_from_finalized(
        &frame,
        &recall,
        perception.frame_binding,
        &slot,
    )
    .unwrap();
    assert_eq!(upload.base_frame_digest, frame.base_digest());
    assert_eq!(upload.context_digest, frame.context().canonical_digest());
    assert_eq!(upload.final_frame_digest, frame.frame_digest());
    assert_eq!(upload.header.perception_header_index, 0);
    assert_eq!(upload.header.candidate_count, 2);
    assert_eq!(upload.records.len(), 2);
    assert!(upload.records.iter().all(|row| {
        row.target_latent == [0.0; 8]
            && row.family_value == [0.0; 4]
            && row.source_counts_packed == 0
    }));

    let mut foreign_binding = perception.frame_binding;
    foreign_binding.final_frame_digest = alife_core::PerceptionFrameDigest([9; 4]);
    assert!(
        GpuMemoryContextUpload::try_from_finalized(&frame, &recall, foreign_binding, &slot,)
            .is_err()
    );
}

#[cfg(feature = "gpu-tests")]
#[test]
fn finalized_memory_context_runs_inside_the_authoritative_gpu_batch() {
    pollster::block_on(async {
        let frame = support::perception_frame_for_profile(
            813,
            SensorProfile::GroundedObjectSlotsV1,
            true,
            2,
        );
        let draft = PerceptionFrameDraft::new(
            frame.organism_id(),
            frame.tick(),
            frame.sensor_profile(),
            frame.sensory().clone(),
            frame.body(),
            frame.homeostasis().clone(),
            frame.candidates().to_vec(),
            frame.profile_provenance(),
            frame.grounded_object_slots().to_vec(),
        )
        .unwrap();
        let bank = MemoryBank::new(
            MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
        )
        .unwrap();
        let (frame, recall) = bank.recall_frame(&draft).unwrap().finalize(draft).unwrap();
        let phenotype = support::phenotype_for_capacity_at_maturation(
            BrainCapacityClass::n512(),
            0xC600_0005,
            0.35,
            SensorProfile::GroundedObjectSlotsV1,
        );
        let mut gpu = support::GpuPipelineFixture::new(&phenotype).await;
        let (pending, receipt) = gpu.run_memory_frame_keep_pending(&frame, &recall).await;

        assert_eq!(pending.result.record.status, 1);
        assert_eq!(receipt.slot, pending.result.record.slot);
        assert_eq!(
            receipt.slot_generation,
            pending.result.record.slot_generation
        );
        assert_eq!(receipt.candidate_count, frame.candidates().len() as u16);
        assert_eq!(receipt.base_frame_digest, frame.base_digest());
        assert_eq!(receipt.context_digest, frame.context().canonical_digest());
        assert_eq!(receipt.final_frame_digest, frame.frame_digest());
        assert_eq!(receipt.perception_header_index, 0);
    });
}

#[cfg(feature = "gpu-tests")]
#[test]
fn required_runtime_dispatches_finalized_memory_and_returns_its_exact_binding() {
    let source =
        support::perception_frame_for_profile(814, SensorProfile::GroundedObjectSlotsV1, true, 2);
    let draft = PerceptionFrameDraft::new(
        source.organism_id(),
        source.tick(),
        source.sensor_profile(),
        source.sensory().clone(),
        source.body(),
        source.homeostasis().clone(),
        source.candidates().to_vec(),
        source.profile_provenance(),
        source.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    let (frame, recall) = bank.recall_frame(&draft).unwrap().finalize(draft).unwrap();
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n512(),
        0xC600_0006,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let mut backend = GpuClosedLoopBackend::new_required_with_config(GpuClosedLoopRuntimeConfig {
        n512_slots: 1,
        n1024_slots: 1,
        n2048_slots: 1,
        aggregate_resident_ceiling_bytes: 128 * 1024 * 1024,
    })
    .unwrap();
    let handle = backend
        .insert_brain(frame.organism_id(), phenotype)
        .unwrap();
    let upload = backend
        .prepare_memory_context_upload(handle, &frame, &recall)
        .unwrap();
    let input = GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &upload).unwrap();
    let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![input]).unwrap();
    let ticks = backend.tick_memory_batch(&batch).unwrap();

    assert_eq!(ticks.len(), 1);
    let tick = &ticks[0];
    let receipt = tick
        .memory_context_binding
        .expect("memory-aware runtime tick must bind finalized recall");
    assert_eq!(receipt.slot, handle.slot());
    assert_eq!(receipt.slot_generation, handle.generation());
    assert_eq!(receipt.base_frame_digest, frame.base_digest());
    assert_eq!(receipt.context_digest, frame.context().canonical_digest());
    assert_eq!(receipt.final_frame_digest, frame.frame_digest());
    assert_eq!(receipt.candidate_count, frame.candidates().len() as u16);
    backend
        .discard_pending_eligibility(handle, tick.pending_eligibility.identity())
        .unwrap();
}

#[cfg(feature = "gpu-tests")]
#[test]
fn evidence_logit_snapshot_is_bound_to_the_pending_frame() {
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n512(),
        0xC600_0010,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let (frame, recall) = conditioned_memory_frame(817, &phenotype);
    let mut backend = GpuClosedLoopBackend::new_required_with_config(GpuClosedLoopRuntimeConfig {
        n512_slots: 1,
        n1024_slots: 1,
        n2048_slots: 1,
        aggregate_resident_ceiling_bytes: 128 * 1024 * 1024,
    })
    .unwrap();
    let handle = backend
        .insert_brain(frame.organism_id(), phenotype)
        .unwrap();
    let upload = backend
        .prepare_memory_context_upload(handle, &frame, &recall)
        .unwrap();
    let input = GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &upload).unwrap();
    let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![input]).unwrap();
    let tick = backend.tick_memory_batch(&batch).unwrap().remove(0);

    let snapshot = backend
        .candidate_logits_for_evidence(handle, &frame, tick.pending_eligibility.identity())
        .unwrap();
    assert_eq!(snapshot.handle, handle);
    assert_eq!(snapshot.dispatch_generation, tick.dispatch_generation);
    assert_eq!(snapshot.originating_tick, frame.tick());
    assert_eq!(snapshot.frame_digest, frame.frame_digest());
    assert_eq!(snapshot.logits.len(), frame.candidates().len());
    assert_eq!(
        snapshot.logits[usize::from(tick.selection.candidate_index)],
        tick.selection.logit
    );

    let mut changed_frame = frame.clone();
    changed_frame = support::perception_frame_for_profile_at_tick(
        changed_frame.organism_id().raw(),
        changed_frame.tick().raw() + 1,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        2,
    );
    assert!(backend
        .candidate_logits_for_evidence(handle, &changed_frame, tick.pending_eligibility.identity(),)
        .is_err());
    backend
        .discard_pending_eligibility(handle, tick.pending_eligibility.identity())
        .unwrap();
}

#[cfg(feature = "gpu-tests")]
#[test]
fn memory_decoder_eligibility_matches_a_real_gpu_finite_difference() {
    pollster::block_on(async {
        let phenotype = support::phenotype_for_capacity_at_maturation(
            BrainCapacityClass::n512(),
            0xC600_0007,
            0.35,
            SensorProfile::GroundedObjectSlotsV1,
        );
        let (frame, recall) = conditioned_memory_frame(815, &phenotype);
        let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
        let family = u32::from(frame.candidates()[0].family.raw());
        let metadata = upload
            .decoder_eligibility_metadata
            .iter()
            .find(|metadata| {
                metadata.decoder_head == DecoderHeadKind::MemoryContext.raw()
                    && metadata.family == family
                    && memory_lane_sample(&frame, 0, metadata.input_lane).abs() > 1.0e-4
                    && (memory_lane_sample(&frame, 0, metadata.input_lane)
                        - memory_lane_sample(&frame, 1, metadata.input_lane))
                    .abs()
                        > 1.0e-4
            })
            .copied()
            .expect("conditioned target must expose a differentiating memory lane");
        let sample_0 = memory_lane_sample(&frame, 0, metadata.input_lane);
        let sample_1 = memory_lane_sample(&frame, 1, metadata.input_lane);
        let epsilon = 1.0e-3 * (sample_0 - sample_1).signum();

        let mut baseline = support::GpuPipelineFixture::new(&phenotype).await;
        baseline.set_decoder_genetic_weights_zeroed(true);
        baseline.set_genetic_weight_for_slot(0, metadata.global_synapse_id, 0.0);
        let (base_pending, _) = baseline
            .run_memory_frame_keep_pending(&frame, &recall)
            .await;

        let mut perturbed = support::GpuPipelineFixture::new(&phenotype).await;
        perturbed.set_decoder_genetic_weights_zeroed(true);
        perturbed.set_genetic_weight_for_slot(0, metadata.global_synapse_id, epsilon);
        let (perturbed_pending, _) = perturbed
            .run_memory_frame_keep_pending(&frame, &recall)
            .await;
        assert_eq!(base_pending.result.record.status, 1);
        assert_eq!(perturbed_pending.result.record.status, 1);
        assert_eq!(base_pending.result.record.candidate_index, 0);
        assert_eq!(perturbed_pending.result.record.candidate_index, 0);

        let finite_difference = (f32::from_bits(perturbed_pending.result.record.logit_bits)
            - f32::from_bits(base_pending.result.record.logit_bits))
            / epsilon;
        let words = perturbed.read_all_mutable_words().await;
        let range = &perturbed
            .slot_for_test(0)
            .word_ranges()
            .decoder_eligibility_bank_1_words;
        let actual_eligibility =
            f32::from_bits(words[range.start as usize + metadata.eligibility_local_index as usize]);
        let expected_derivative = sample_0.clamp(-1.0, 1.0);
        assert!((finite_difference - expected_derivative).abs() <= 2.0e-3);
        assert!((actual_eligibility - expected_derivative).abs() <= 1.0e-6);
        assert_ne!(actual_eligibility, 0.0);

        baseline.discard_pending_for_slot(0, &base_pending.pending);
        perturbed.discard_pending_for_slot(0, &perturbed_pending.pending);
    });
}

#[cfg(feature = "gpu-tests")]
#[test]
fn sealed_outcome_changes_the_selected_memory_decoder_fast_weights_immediately() {
    let phenotype = support::phenotype_for_capacity_at_maturation(
        BrainCapacityClass::n512(),
        0xC600_0008,
        0.35,
        SensorProfile::GroundedObjectSlotsV1,
    );
    let (frame, recall) = conditioned_memory_frame_with_candidate_count(816, &phenotype, 1);
    let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
    let mut backend = GpuClosedLoopBackend::new_required_with_config(GpuClosedLoopRuntimeConfig {
        n512_slots: 1,
        n1024_slots: 1,
        n2048_slots: 1,
        aggregate_resident_ceiling_bytes: 128 * 1024 * 1024,
    })
    .unwrap();
    let handle = backend
        .insert_brain(frame.organism_id(), phenotype.clone())
        .unwrap();
    let before = backend
        .snapshot_brain(handle, Tick::new(frame.tick().raw() - 1))
        .unwrap()
        .into_parts();
    let memory_upload = backend
        .prepare_memory_context_upload(handle, &frame, &recall)
        .unwrap();
    let input = GpuClosedLoopMemoryTickInput::try_new(handle, &frame, &memory_upload).unwrap();
    let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![input]).unwrap();
    let tick = backend.tick_memory_batch(&batch).unwrap().remove(0);
    let pending = backend
        .snapshot_brain(handle, frame.tick())
        .unwrap()
        .into_parts();
    let staged_decoder_eligibility = if pending.active_eligibility_bank == 0 {
        &pending.decoder_eligibility_bank_1_bits
    } else {
        &pending.decoder_eligibility_bank_0_bits
    };
    let selected_family = u32::from(
        frame.candidates()[usize::from(tick.selection.candidate_index)]
            .family
            .raw(),
    );
    let eligible_memory_rows = upload
        .decoder_eligibility_metadata
        .iter()
        .filter(|metadata| {
            metadata.decoder_head == DecoderHeadKind::MemoryContext.raw()
                && metadata.family == selected_family
                && memory_lane_sample(
                    &frame,
                    usize::from(tick.selection.candidate_index),
                    metadata.input_lane,
                )
                .abs()
                    > 1.0e-6
        })
        .collect::<Vec<_>>();
    assert!(!eligible_memory_rows.is_empty());
    let staged_memory_rows = eligible_memory_rows
        .into_iter()
        .filter(|metadata| {
            f32::from_bits(staged_decoder_eligibility[metadata.eligibility_local_index as usize])
                .abs()
                > 1.0e-6
        })
        .count();
    assert!(staged_memory_rows > 0);
    let patch =
        painful_patch_for_gpu_tick(handle, &frame, &recall, &tick, ExperienceSequenceId(9_002));
    let learning = backend.apply_sealed_outcome(handle, &patch).unwrap();
    assert!(learning.fast_weights_changed > 0);
    assert!(backend.pending_eligibility(handle).unwrap().is_none());
    let after = backend
        .snapshot_brain(handle, Tick::new(frame.tick().raw() + 1))
        .unwrap()
        .into_parts();
    let before_fast = if before.active_weight_bank == 0 {
        &before.fast_bank_0_bits
    } else {
        &before.fast_bank_1_bits
    };
    let after_fast = if after.active_weight_bank == 0 {
        &after.fast_bank_0_bits
    } else {
        &after.fast_bank_1_bits
    };
    let changed_memory_rows = upload
        .decoder_eligibility_metadata
        .iter()
        .filter(|metadata| {
            metadata.decoder_head == DecoderHeadKind::MemoryContext.raw()
                && metadata.family == selected_family
                && memory_lane_sample(
                    &frame,
                    usize::from(tick.selection.candidate_index),
                    metadata.input_lane,
                )
                .abs()
                    > 1.0e-6
        })
        .filter(|metadata| {
            before_fast[metadata.global_synapse_id as usize]
                != after_fast[metadata.global_synapse_id as usize]
        })
        .count();
    assert!(changed_memory_rows > 0);
}
