//! Qualifies the actual host batch builder without requesting a GPU device.

use super::*;
use alife_core::{
    ActionCandidate, ActionId, ActionTarget, BodySnapshot, BrainActivityPolicyV1,
    BrainDispatchIdentity, BrainGenome, CandidateActionFamily, CandidateFeatureVector,
    CandidateObservationRef, DevelopmentState, DurationTicks, GpuPressureSample,
    GpuPressureSampleInput, HomeostaticSnapshot, NormalizedScalar, OrganismId, PhenotypeCompiler,
    Pose, SensorProfile, SensorProfileProvenance, SensoryAbiVersion, SensoryChannels,
    SensorySnapshot, Tick, Vec3f, Velocity, WorldEntityId,
};

#[allow(dead_code)]
#[path = "exploration_reference.rs"]
mod reference;

fn phenotype() -> BrainPhenotype {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(13, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar(0.35));
    PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::PrivilegedAffordanceV1,
    )
    .unwrap()
}

fn frame(organism: u64, tick: u64) -> PerceptionFrame {
    let organism = OrganismId(organism);
    let tick = Tick::new(tick);
    let mut sensory = SensoryChannels::ZERO;
    sensory.novelty_signal = NormalizedScalar(0.7);
    let mut homeostasis = HomeostaticSnapshot::baseline(tick);
    homeostasis.drives.hunger = 0.27;
    homeostasis.drives.fatigue = 0.31;
    homeostasis.drives.fear = 0.17;
    homeostasis.drives.pain = 0.13;
    homeostasis.drives.curiosity = 0.63;
    homeostasis.drives.temperature_stress = 0.21;
    let candidates = (0..2)
        .map(|index| {
            ActionCandidate::new(
                index,
                ActionId(4 + u32::from(index)),
                ActionKind::Inspect,
                CandidateActionFamily::Inspect,
                CandidateObservationRef::None,
                ActionTarget::NONE,
                CandidateFeatureVector::zero(),
                Confidence(0.8),
                NormalizedScalar(0.1),
                DurationTicks::new(1),
                DurationTicks::new(1),
            )
            .unwrap()
        })
        .collect();
    PerceptionFrame::new(
        organism,
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        SensorySnapshot::new(organism, tick, Vec3f::ZERO, sensory, Default::default()).unwrap(),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        homeostasis,
        candidates,
        SensorProfileProvenance::new(
            SensorProfile::PrivilegedAffordanceV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        Vec::new(),
    )
    .unwrap()
}

fn activity(
    frame: &PerceptionFrame,
    slot: &GpuBrainSlot,
    phenotype: &BrainPhenotype,
    generation: u64,
) -> NeuralThrottleDecision {
    let policy = BrainActivityPolicyV1::production_v1();
    let identity = BrainDispatchIdentity {
        organism_id_raw: frame.organism_id().raw(),
        tick: frame.tick().raw(),
        class_id_raw: phenotype.brain_class_id().raw(),
        handle_slot: slot.record().slot,
        handle_generation: slot.record().slot_generation,
        sequence_cursor: generation,
        dispatch_generation: generation,
        frame_digest: frame.frame_digest().0,
    };
    let pressure = GpuPressureSample::try_new(
        &policy,
        GpuPressureSampleInput {
            identity,
            source_dispatch_generation: generation - 1,
            source_frame_digest: if generation == 1 {
                [0; 4]
            } else {
                frame.frame_digest().0
            },
            completed_gpu_time_ns: 0,
            queue_depth: 0,
            logical_heap_used: 0,
            logical_heap_capacity: 65_535,
            brain_atp_remaining_q16: 65_535,
            brain_atp_capacity_q16: 65_535,
        },
    )
    .unwrap();
    NeuralThrottleDecision::derive(
        &policy,
        phenotype,
        BrainCapacityClass::n512().execution(),
        identity,
        pressure,
    )
    .unwrap()
}

fn batch(
    frames: &[PerceptionFrame],
    slots: &[u32],
    generation: u64,
    base: u32,
) -> GpuActiveBatchUpload {
    let phenotype = phenotype();
    let mut plan = crate::GpuClassBucketPlan::new(BrainCapacityClass::n512(), 4).unwrap();
    let placed: Vec<_> = slots
        .iter()
        .map(|slot| plan.insert_phenotype(*slot, 7 + *slot, &phenotype).unwrap())
        .collect();
    let decisions: Vec<_> = frames
        .iter()
        .zip(&placed)
        .map(|(frame, slot)| activity(frame, slot, &phenotype, generation))
        .collect();
    let entries: Vec<_> = frames
        .iter()
        .zip(&placed)
        .zip(&decisions)
        .map(|((frame, slot), activity)| GpuActiveBatchEntry {
            frame,
            slot,
            phenotype: &phenotype,
            activity,
            memory_upload: None,
            active_eligibility_generation: 1,
        })
        .collect();
    GpuActiveBatchUpload::try_from_views(
        &entries,
        base,
        plan.ownership_token(),
        &BTreeMap::new(),
        4 * GPU_ACTIVE_DISPATCH_ROW_WORDS,
        65_536,
        NonZeroU64::new(generation).unwrap(),
        generation,
    )
    .unwrap()
}

fn uploaded_identity(batch: &GpuActiveBatchUpload, row: usize) -> reference::State {
    let header = batch.headers()[row];
    let pending = batch.learning_headers()[row].outcome_offset as usize;
    let payload = batch.frame_payload_words();
    let join = |offset| u64::from(payload[offset]) | (u64::from(payload[offset + 1]) << 32);
    reference::State {
        organism_id: join(pending + 12),
        tick: u64::from(header.tick_lo) | (u64::from(header.tick_hi) << 32),
        novelty: f32::from_bits(payload[header.sensory_offset as usize + 41]),
        hunger: f32::from_bits(payload[header.sensory_offset as usize + 55]),
        fatigue: f32::from_bits(payload[header.sensory_offset as usize + 56]),
        fear: f32::from_bits(payload[header.sensory_offset as usize + 57]),
        pain: f32::from_bits(payload[header.sensory_offset as usize + 58]),
        curiosity: f32::from_bits(payload[header.sensory_offset as usize + 60]),
        temperature_stress: f32::from_bits(payload[header.sensory_offset as usize + 62]),
    }
}

fn uploaded_candidate(
    batch: &GpuActiveBatchUpload,
    row: usize,
    index: usize,
) -> reference::Candidate {
    let learning = batch.learning_headers()[row];
    let start = learning.outcome_offset as usize + crate::GPU_PENDING_ELIGIBILITY_WORDS + index * 4;
    reference::Candidate {
        index: index as u32,
        identity: batch.frame_payload_words()[start..start + 4]
            .try_into()
            .unwrap(),
        logit: 0.0,
        gap_match: 0.0,
        motor_slot: Some(0),
    }
}

fn with_candidates(frame: &PerceptionFrame, candidates: Vec<ActionCandidate>) -> PerceptionFrame {
    PerceptionFrame::new(
        frame.organism_id(),
        frame.tick(),
        frame.sensor_profile(),
        frame.sensory().clone(),
        frame.body(),
        *frame.homeostasis(),
        candidates,
        frame.profile_provenance(),
        frame.grounded_object_slots().to_vec(),
    )
    .unwrap()
}

#[test]
fn real_batch_upload_binds_full_durable_identity_and_physiology_at_existing_offsets() {
    assert_eq!(
        std::mem::offset_of!(GpuPendingEligibilityRecord, organism_id),
        12 * 4
    );
    let frames = [
        frame(0x12_3456_789a, 0x23_4567_89ab),
        frame(0x34_5678_9abc, 0x45_6789_abcd),
    ];
    let batch = batch(&frames, &[0, 2], 1, 64);
    for (row, frame) in frames.iter().enumerate() {
        let state = uploaded_identity(&batch, row);
        assert_eq!(state.organism_id, frame.organism_id().raw());
        assert_eq!(state.tick, frame.tick().raw());
        assert_eq!(state.novelty, 0.7);
        assert_eq!(state.curiosity, 0.63);
        assert_eq!(state.hunger, 0.27);
        assert_eq!(state.fatigue, 0.31);
        assert_eq!(state.fear, 0.17);
        assert_eq!(state.pain, 0.13);
        assert_eq!(state.temperature_stress, 0.21);
        let header = batch.learning_headers()[row];
        let start = header.outcome_offset as usize;
        let restored = GpuPendingEligibilityRecord::from_words(
            &batch.frame_payload_words()[start..start + crate::GPU_PENDING_ELIGIBILITY_WORDS],
        )
        .unwrap();
        assert_eq!(restored, batch.pending_templates[row]);
        assert_eq!(
            restored.originating_tick,
            [frame.tick().raw() as u32, (frame.tick().raw() >> 32) as u32]
        );
        // Standard phenotypes have no gap lane. Its absence must not be
        // interpreted as a gap or trigger any phenotype/state migration.
        let stride = header.decoder_input_stride as usize;
        assert_eq!(stride, 36);
        for candidate in 0..header.candidate_count as usize {
            let start = header.decoder_learning_input_offset as usize + candidate * stride;
            assert!(
                batch.frame_payload_words()[start + CANDIDATE_FEATURE_COUNT..start + stride]
                    .iter()
                    .all(|word| *word == 0)
            );
            let uploaded = uploaded_candidate(&batch, row, candidate);
            assert_eq!(
                uploaded.identity,
                crate::game_candidate_exploration_identity(&frame.candidates()[candidate]).unwrap()
            );
            assert_eq!(
                uploaded.identity,
                batch.candidate_exploration_identities[row][candidate]
            );
        }
    }
}

#[test]
fn real_batch_row_order_slot_relocation_and_dispatch_generation_do_not_change_draws() {
    let frames = [
        frame(0x1_0000_0011, 0x2_0000_004d),
        frame(0x3_0000_0022, 0x4_0000_004d),
    ];
    let original = batch(&frames, &[0, 1], 1, 0);
    let relocated = batch(&[frames[1].clone(), frames[0].clone()], &[3, 2], 101, 257);
    for row in 0..2 {
        let before = uploaded_identity(&original, row);
        let after = uploaded_identity(&relocated, 1 - row);
        assert_ne!(
            original.headers()[row].slot,
            relocated.headers()[1 - row].slot
        );
        assert_ne!(
            original.headers()[row].sensory_offset,
            relocated.headers()[1 - row].sensory_offset
        );
        for index in 0..2 {
            let candidate = uploaded_candidate(&original, row, index);
            let relocated_candidate = uploaded_candidate(&relocated, 1 - row, index);
            assert_eq!(candidate.identity, relocated_candidate.identity);
            assert_eq!(
                reference::identity_hash(before, candidate),
                reference::identity_hash(after, relocated_candidate)
            );
            assert_eq!(
                reference::score(before, candidate).to_bits(),
                reference::score(after, relocated_candidate).to_bits()
            );
        }
    }
}

#[test]
fn real_pending_template_little_endian_byte_roundtrip_preserves_future_draw_inputs() {
    let frames = [frame(0xfedc_ba98_7654_3210, 0xdead_beef_8765_4321)];
    let batch = batch(&frames, &[1], 17, 0);
    let original = batch.pending_templates[0];
    let bytes: Vec<_> = original
        .words()
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect();
    let words: Vec<_> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| u32::from_le_bytes(*bytes))
        .collect();
    let restored = GpuPendingEligibilityRecord::from_words(&words).unwrap();
    assert_eq!(restored, original);
    let identity = |record: GpuPendingEligibilityRecord| reference::State {
        organism_id: u64::from(record.organism_id[0]) | (u64::from(record.organism_id[1]) << 32),
        tick: u64::from(record.originating_tick[0]) | (u64::from(record.originating_tick[1]) << 32),
        ..reference::State::default()
    };
    for candidate in reference::pair() {
        assert_eq!(
            reference::identity_hash(identity(original), candidate),
            reference::identity_hash(identity(restored), candidate)
        );
    }
}

#[test]
fn real_candidate_upload_permutation_preserves_primitive_target_draws_and_winner() {
    let original_frame = frame(0x1_0000_0042, 0x2_0000_004d);
    let mut candidates = original_frame.candidates().to_vec();
    candidates[1].action_id = candidates[0].action_id;
    candidates[0].target = ActionTarget::new(Some(WorldEntityId(0x3_0000_0007)), None);
    candidates[1].target = ActionTarget::new(Some(WorldEntityId(0x4_0000_0007)), None);
    let original_frame = with_candidates(&original_frame, candidates.clone());
    candidates.reverse();
    for (index, candidate) in candidates.iter_mut().enumerate() {
        candidate.candidate_index = index as u16;
    }
    let reordered_frame = with_candidates(&original_frame, candidates);
    let original = batch(&[original_frame], &[0], 1, 64);
    let reordered = batch(&[reordered_frame], &[3], 107, 257);
    let before = [
        uploaded_candidate(&original, 0, 0),
        uploaded_candidate(&original, 0, 1),
    ];
    let after = [
        uploaded_candidate(&reordered, 0, 0),
        uploaded_candidate(&reordered, 0, 1),
    ];
    assert_ne!(before[0].identity, before[1].identity);
    assert_eq!(before[0].identity, after[1].identity);
    assert_eq!(before[1].identity, after[0].identity);
    let mut state = uploaded_identity(&original, 0);
    for tick in 0..4096 {
        state.tick = tick;
        for index in 0..2 {
            assert_eq!(
                reference::noise(state, before[index]).to_bits(),
                reference::noise(state, after[1 - index]).to_bits()
            );
        }
        let old = reference::select(state, &before, None).unwrap() as usize;
        let new = reference::select(state, &after, None).unwrap() as usize;
        assert_eq!(before[old].identity, after[new].identity);
    }
}

#[test]
fn canonical_candidate_identity_uses_full_target_and_excludes_enumeration_and_sensory_features() {
    let original_frame = frame(9, 77);
    let original = original_frame.candidates()[0];
    let identity = crate::game_candidate_exploration_identity(&original).unwrap();
    let mut repacked = original;
    repacked.candidate_index = 17;
    repacked.observation = CandidateObservationRef::ObjectSlot(11);
    repacked.features.0[0] = 0.75;
    repacked.sensor_confidence = Confidence(0.2);
    assert_eq!(
        crate::game_candidate_exploration_identity(&repacked).unwrap(),
        identity
    );
    let mut target_a = original;
    target_a.target = ActionTarget::new(Some(WorldEntityId(0x1_0000_0007)), None);
    let mut target_b = target_a;
    target_b.target = ActionTarget::new(Some(WorldEntityId(0x2_0000_0007)), None);
    assert_ne!(
        crate::game_candidate_exploration_identity(&target_a).unwrap(),
        crate::game_candidate_exploration_identity(&target_b).unwrap()
    );
    target_b = target_a;
    target_b.target = ActionTarget::new(target_a.target.entity, Some(Vec3f::new(1.0, 2.0, 3.0)));
    assert_ne!(
        crate::game_candidate_exploration_identity(&target_a).unwrap(),
        crate::game_candidate_exploration_identity(&target_b).unwrap()
    );
    let mut changed = original;
    changed.action_id = ActionId(original.action_id.raw() + 1);
    assert_ne!(
        crate::game_candidate_exploration_identity(&changed).unwrap(),
        identity
    );
    changed = original;
    changed.max_duration = DurationTicks::new(2);
    assert_ne!(
        crate::game_candidate_exploration_identity(&changed).unwrap(),
        identity
    );
}

#[test]
fn serialized_perception_roundtrip_rebuilds_identical_durable_draw_inputs_after_repacking() {
    let original = frame(0xfedc_ba98_7654_3210, 0xdead_beef_8765_4321);
    let bytes = serde_json::to_vec(&original).unwrap();
    let restored: PerceptionFrame = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, original);
    let original_batch = batch(&[original], &[0], 1, 64);
    let restored_batch = batch(&[restored], &[3], 101, 257);
    let before = uploaded_identity(&original_batch, 0);
    let after = uploaded_identity(&restored_batch, 0);
    for index in 0..2 {
        let original = uploaded_candidate(&original_batch, 0, index);
        let restored = uploaded_candidate(&restored_batch, 0, index);
        assert_eq!(original.identity, restored.identity);
        assert_eq!(
            reference::identity_hash(before, original),
            reference::identity_hash(after, restored)
        );
        assert_eq!(
            reference::score(before, original).to_bits(),
            reference::score(after, restored).to_bits()
        );
    }
}

#[test]
fn explicit_54_lane_extension_upload_zeros_absent_gap_without_mutating_brain_state() {
    let asset = alife_core::FoundationWeightAsset::decode_canonical(include_bytes!(
        "../../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
    ))
    .unwrap();
    let manifest = asset.manifest();
    let identity = alife_core::FoundationGeneticIdentity::new(
        manifest.foundation_id().raw(),
        manifest.foundation_version().raw() as u16,
        manifest.compatibility_family_id().raw(),
        manifest.capacity_class_id(),
    )
    .unwrap();
    let extension = alife_core::CognitiveChannelExtensionV1::try_new_v1(
        identity,
        alife_core::ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )
    .unwrap();
    let candidate =
        alife_core::Nano512ActionCreditCandidateV2::new_with_cognitive_extension(&asset, extension)
            .unwrap();
    let (phenotype, _) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&candidate).unwrap();
    let mut plan = crate::GpuClassBucketPlan::new(BrainCapacityClass::n512(), 1).unwrap();
    let slot = plan.insert_phenotype(0, 1, &phenotype).unwrap();
    assert_eq!(slot.decoder_input_stride(), 54);
    let original = frame(7, 77);
    let candidates = original
        .candidates()
        .iter()
        .cloned()
        .map(|mut candidate| {
            candidate.kind = ActionKind::Idle;
            candidate.family = CandidateActionFamily::Idle;
            candidate
        })
        .collect();
    let profile = SensorProfile::GroundedObjectSlotsV1;
    let frame = PerceptionFrame::new(
        original.organism_id(),
        original.tick(),
        profile,
        original.sensory().clone(),
        original.body(),
        *original.homeostasis(),
        candidates,
        SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, original.tick()).unwrap(),
        Vec::new(),
    )
    .unwrap();
    let before = plan.mutable_state_words().to_vec();
    let upload = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
    for candidate in &upload.candidates {
        let base = candidate.feature_offset as usize;
        assert_eq!(
            upload.frame_payload_words[base + 50],
            0,
            "absent gap evidence is zero"
        );
        assert_eq!(
            upload.frame_payload_words[base + 51],
            0,
            "prior outcome magnitude is separate"
        );
        assert!(upload.frame_payload_words[base + 24..base + 54]
            .iter()
            .all(|word| *word == 0));
    }
    assert_eq!(plan.mutable_state_words(), before);
}

#[test]
fn maximum_candidate_batch_adds_exactly_512_bytes_of_unscored_identity() {
    let original = frame(1, 77);
    let candidates = (0..alife_core::MAX_ACTION_CANDIDATES)
        .map(|index| {
            let mut candidate = original.candidates()[0];
            candidate.candidate_index = index as u16;
            candidate.action_id = ActionId(4 + index as u32);
            candidate
        })
        .collect();
    let frame = with_candidates(&original, candidates);
    let batch = batch(&[frame], &[0], 1, 64);
    let pending_end =
        batch.learning_headers()[0].outcome_offset as usize + crate::GPU_PENDING_ELIGIBILITY_WORDS;
    assert_eq!((batch.frame_payload_words().len() - pending_end) * 4, 512);
    assert_eq!(
        batch.candidate_exploration_identities[0].len(),
        alife_core::MAX_ACTION_CANDIDATES
    );
}

#[cfg(feature = "training-rollout")]
#[test]
fn actual_training_composer_bypasses_game_perturbations_before_exact_training_sampler() {
    let source = crate::training_rollout::shader_source(CLOSED_LOOP_DECODE_WGSL);
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    assert!(source.contains("fn game_exploration_enabled(header:GpuPerceptionHeader) -> bool {\n  return (header.reserved & 0x80000000u) == 0u;\n}"));
    assert!(source.contains("selected_candidate = training_sample(header, brain, 0u);"));
    assert!(source.contains("motor_candidates[slot] = training_sample(header, brain, slot + 1u);"));
}

#[test]
fn distinct_target_draw_collision_preserves_exact_index_ties_after_restore_and_reordering() {
    // Distinct canonical identities can share the sampler's 23-bit uniform.
    // Existing exact lowest-index tie semantics still apply in this case.
    let original_frame = frame(9, 77);
    let mut candidates = original_frame.candidates().to_vec();
    candidates[1].action_id = candidates[0].action_id;
    candidates[0].target = ActionTarget::new(Some(WorldEntityId(2988)), None);
    candidates[1].target = ActionTarget::new(Some(WorldEntityId(4659)), None);
    let original_frame = with_candidates(&original_frame, candidates);
    let bytes = serde_json::to_vec(&original_frame).unwrap();
    let restored: PerceptionFrame = serde_json::from_slice(&bytes).unwrap();
    restored.validate().unwrap();
    let mut reordered = restored.candidates().to_vec();
    reordered.reverse();
    for (index, candidate) in reordered.iter_mut().enumerate() {
        candidate.candidate_index = index as u16;
    }
    let reordered_frame = with_candidates(&restored, reordered);
    let original = batch(&[original_frame], &[0], 1, 64);
    let restored = batch(&[restored], &[2], 57, 193);
    let reordered = batch(&[reordered_frame], &[3], 101, 257);
    let before = [
        uploaded_candidate(&original, 0, 0),
        uploaded_candidate(&original, 0, 1),
    ];
    let after = [
        uploaded_candidate(&reordered, 0, 0),
        uploaded_candidate(&reordered, 0, 1),
    ];
    let state = uploaded_identity(&original, 0);
    let restored_state = uploaded_identity(&restored, 0);
    let reordered_state = uploaded_identity(&reordered, 0);
    let restored_candidates = [
        uploaded_candidate(&restored, 0, 0),
        uploaded_candidate(&restored, 0, 1),
    ];
    assert_ne!(before[0].identity, before[1].identity);
    assert_ne!(
        reference::identity_hash(state, before[0]),
        reference::identity_hash(state, before[1])
    );
    assert_eq!(
        reference::identity_hash(state, before[0]) >> 9,
        reference::identity_hash(state, before[1]) >> 9
    );
    assert_eq!(
        reference::score(state, before[0]).to_bits(),
        reference::score(state, before[1]).to_bits()
    );
    assert_eq!(before[0].identity, after[1].identity);
    assert_eq!(before[1].identity, after[0].identity);
    for motor_slot in [None, Some(0)] {
        assert_eq!(reference::select(state, &before, motor_slot), Some(0));
        assert_eq!(
            reference::select(restored_state, &restored_candidates, motor_slot),
            Some(0)
        );
        assert_eq!(
            reference::select(reordered_state, &after, motor_slot),
            Some(0)
        );
    }
    assert_ne!(before[0].identity, after[0].identity);
    for (index, candidate) in before.iter().enumerate() {
        let restored_candidate = uploaded_candidate(&restored, 0, index);
        assert_eq!(candidate.identity, restored_candidate.identity);
        assert_eq!(
            reference::score(state, *candidate).to_bits(),
            reference::score(restored_state, restored_candidate).to_bits()
        );
        assert_eq!(
            reference::score(state, *candidate).to_bits(),
            reference::score(reordered_state, after[1 - index]).to_bits()
        );
    }
}
