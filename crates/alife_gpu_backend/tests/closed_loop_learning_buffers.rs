use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
    SensorProfile, Tick,
};
use alife_gpu_backend::{
    pack_replay_eligibility_sample, unpack_replay_eligibility_sample, GpuBrainSlotExtensionRecord,
    GpuClassBucketPlan, GpuDecoderEligibilityMetadata, GpuPlasticityReceptorRecord,
    GpuReplayCaptureIdentityRecord, GpuReplayEventRecord, GpuSleepParameterRecord,
    GpuSlotLearningStateRecord, GpuSynapseLearningMetadata, CLOSED_LOOP_DECODE_WGSL,
    CLOSED_LOOP_RECURRENT_WGSL, GPU_NO_EXTENSION_SENTINEL,
};

fn phenotype() -> alife_core::BrainPhenotype {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(0xB002, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
}

fn reads_bank_zero_mutable_weights_directly(source: &str) -> bool {
    source.contains("load_state_f32(brain.lifetime_weight_offset")
        || source.contains("load_state_f32(brain.fast_weight_offset")
}

#[test]
fn every_active_synapse_has_separate_double_banked_mutable_learning_layers() {
    let phenotype = phenotype();
    let capacity = BrainCapacityClass::n512();
    let mut plan = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = plan.insert_phenotype(0, 1, &phenotype).unwrap();
    let ranges = slot.word_ranges();
    let synapses = phenotype.synapses().len();
    let recurrent = phenotype.budgets().global.recurrent_synapses as usize;
    let decoder = synapses - recurrent;

    assert_eq!(ranges.genetic_weight_words.len(), synapses);
    assert_eq!(ranges.lifetime_weight_words.len(), synapses);
    assert_eq!(ranges.lifetime_weight_bank_1_words.len(), synapses);
    assert_eq!(ranges.fast_weight_words.len(), synapses);
    assert_eq!(ranges.fast_weight_bank_1_words.len(), synapses);
    assert_eq!(ranges.recurrent_eligibility_words.len(), recurrent);
    assert_eq!(ranges.recurrent_eligibility_bank_1_words.len(), recurrent);
    assert_eq!(ranges.decoder_eligibility_words.len(), decoder);
    assert_eq!(ranges.decoder_eligibility_bank_1_words.len(), decoder);
    assert_ne!(
        ranges.genetic_weight_words.start,
        ranges.fast_weight_words.start
    );
    assert_ne!(
        ranges.fast_weight_words.start,
        ranges.fast_weight_bank_1_words.start
    );
    assert!(
        plan.mutable_state_words()[ranges.lifetime_weight_words.start as usize
            ..ranges.fast_weight_bank_1_words.end as usize]
            .iter()
            .all(|word| *word == 0)
    );
    assert_ne!(
        slot.record().extension_record_offset,
        GPU_NO_EXTENSION_SENTINEL
    );
    plan.validate().unwrap();
}

#[test]
fn slot_extension_has_stable_offsets_for_learning_sleep_and_memory() {
    assert_eq!(std::mem::size_of::<GpuBrainSlotExtensionRecord>(), 80);
    assert_eq!(std::mem::align_of::<GpuBrainSlotExtensionRecord>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuBrainSlotExtensionRecord, decoder_synapse_local_start),
        8
    );
    assert_eq!(
        std::mem::offset_of!(GpuBrainSlotExtensionRecord, receptor_offset),
        16
    );
    assert_eq!(
        std::mem::offset_of!(
            GpuBrainSlotExtensionRecord,
            recurrent_eligibility_bank_1_offset
        ),
        32
    );
    assert_eq!(
        std::mem::offset_of!(GpuBrainSlotExtensionRecord, fast_bank_1_offset),
        40
    );
    assert_eq!(
        std::mem::offset_of!(GpuBrainSlotExtensionRecord, memory_plan_offset),
        52
    );
    assert_eq!(
        std::mem::offset_of!(GpuBrainSlotExtensionRecord, learning_state_offset),
        60
    );
    assert_eq!(
        std::mem::offset_of!(GpuBrainSlotExtensionRecord, pending_eligibility_offset),
        64
    );
    assert_eq!(
        std::mem::offset_of!(GpuBrainSlotExtensionRecord, replay_plan_identity_offset),
        68
    );
    assert_eq!(std::mem::size_of::<GpuSynapseLearningMetadata>(), 32);
    assert_eq!(std::mem::align_of::<GpuSynapseLearningMetadata>(), 16);
    assert_eq!(std::mem::size_of::<GpuDecoderEligibilityMetadata>(), 32);
    assert_eq!(std::mem::align_of::<GpuDecoderEligibilityMetadata>(), 16);
    assert_eq!(std::mem::size_of::<GpuPlasticityReceptorRecord>(), 32);
    assert_eq!(std::mem::align_of::<GpuPlasticityReceptorRecord>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuPlasticityReceptorRecord, fast_min),
        20
    );
    assert_eq!(std::mem::size_of::<GpuSleepParameterRecord>(), 32);
    assert_eq!(std::mem::align_of::<GpuSleepParameterRecord>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuSleepParameterRecord, eligibility_reset_policy),
        16
    );
    assert_eq!(std::mem::size_of::<GpuReplayEventRecord>(), 96);
    assert_eq!(std::mem::align_of::<GpuReplayEventRecord>(), 16);
    assert_eq!(std::mem::size_of::<GpuSlotLearningStateRecord>(), 96);
    assert_eq!(std::mem::align_of::<GpuSlotLearningStateRecord>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuSlotLearningStateRecord, replay_generation_lo),
        40
    );
    assert_eq!(
        std::mem::offset_of!(GpuSlotLearningStateRecord, pending_eligibility_offset),
        84
    );
    assert_eq!(std::mem::size_of::<GpuReplayCaptureIdentityRecord>(), 32);
    assert_eq!(std::mem::align_of::<GpuReplayCaptureIdentityRecord>(), 16);
    assert_eq!(
        unpack_replay_eligibility_sample(pack_replay_eligibility_sample(7, -12_345)),
        (7, -12_345)
    );
}

#[test]
fn installed_extension_binds_learning_memory_plans_and_zeroed_selectors() {
    let phenotype = phenotype();
    let mut plan = GpuClassBucketPlan::new(BrainCapacityClass::n512(), 1).unwrap();
    let slot = plan.insert_phenotype(0, 1, &phenotype).unwrap();
    let words = plan.mutable_state_words();
    let extension_start = slot.record().extension_record_offset as usize;
    let extension = GpuBrainSlotExtensionRecord::from_words(
        &words[extension_start
            ..extension_start + std::mem::size_of::<GpuBrainSlotExtensionRecord>() / 4],
    )
    .unwrap();
    let state_start = extension.learning_state_offset as usize;
    let state = GpuSlotLearningStateRecord::from_words(
        &words[state_start..state_start + std::mem::size_of::<GpuSlotLearningStateRecord>() / 4],
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
    assert_eq!(
        state.replay_span_count,
        phenotype.replay_capture_plan().global_synapse_ids().len() as u32
    );
    assert_eq!(
        extension.memory_plan_offset,
        slot.word_ranges().memory_channel_plan_words.start
    );
    assert_eq!(
        extension.memory_weight_map_offset,
        slot.word_ranges().memory_weight_index_words.start
    );
    assert_eq!(
        extension.sleep_parameter_offset,
        slot.word_ranges().sleep_parameter_words.start
    );
}

#[test]
fn every_effective_weight_reader_uses_the_active_weight_bank_selector() {
    for source in [CLOSED_LOOP_RECURRENT_WGSL, CLOSED_LOOP_DECODE_WGSL] {
        assert!(source.contains("active_weight_bank"));
        assert!(source.contains("fast_bank_1_offset"));
        assert!(source.contains("lifetime_bank_1_offset"));
        assert!(!reads_bank_zero_mutable_weights_directly(source));
    }
}
