#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainCapacityClass, CanonicalDigestBuilder, DecoderHeadKind, OrganismId, ScaffoldContractError,
    SensorProfile,
};
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuClosedLoopError, GpuEligibilityDiscardRecord, GpuLearningHeader,
    GpuPendingEligibilityRecord, GpuPhenotypeUpload, CLOSED_LOOP_ELIGIBILITY_WGSL,
    GPU_CLOSED_LOOP_TICK_READBACK_BYTES,
};

mod support;

fn join_u64(words: [u32; 2]) -> u64 {
    u64::from(words[0]) | (u64::from(words[1]) << 32)
}

fn split_u64x2(words: [u64; 2]) -> [u32; 4] {
    [
        words[0] as u32,
        (words[0] >> 32) as u32,
        words[1] as u32,
        (words[1] >> 32) as u32,
    ]
}

fn eligibility_digest(domain: &[u8], values: &[u32]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(domain);
    digest.write_sequence_len(values.len());
    for value in values {
        digest.write_u32(*value);
    }
    digest.finish256()
}

fn quantized_eligibility_digest(domain: &[u8], values: &[f32]) -> [u64; 4] {
    let mut digest = CanonicalDigestBuilder::new(domain);
    digest.write_sequence_len(values.len());
    for value in values {
        let quantized = (*value * 1_000_000.0).round() as i32;
        digest.write_u32(quantized as u32);
    }
    digest.finish256()
}

fn range_words(words: &[u32], range: &std::ops::Range<u32>) -> Vec<u32> {
    words[range.start as usize..range.end as usize].to_vec()
}

#[test]
fn learning_header_and_pending_record_have_the_exact_shared_abi() {
    assert_eq!(std::mem::size_of::<GpuLearningHeader>(), 80);
    assert_eq!(std::mem::align_of::<GpuLearningHeader>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuLearningHeader, brain_slot_index),
        16
    );
    assert_eq!(std::mem::offset_of!(GpuLearningHeader, outcome_offset), 48);
    assert_eq!(
        std::mem::offset_of!(GpuLearningHeader, decoder_input_stride),
        60
    );
    assert_eq!(
        std::mem::offset_of!(GpuLearningHeader, pending_eligibility_offset),
        64
    );

    assert_eq!(std::mem::size_of::<GpuPendingEligibilityRecord>(), 144);
    assert_eq!(std::mem::align_of::<GpuPendingEligibilityRecord>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuPendingEligibilityRecord, frame_digest),
        72
    );
    assert_eq!(
        std::mem::offset_of!(GpuPendingEligibilityRecord, candidate_index_and_family),
        104
    );
    assert_eq!(std::mem::size_of::<GpuEligibilityDiscardRecord>(), 48);
    assert_eq!(std::mem::align_of::<GpuEligibilityDiscardRecord>(), 16);
    assert_eq!(
        std::mem::offset_of!(GpuEligibilityDiscardRecord, active_eligibility_generation),
        24
    );
    assert_eq!(
        std::mem::offset_of!(GpuEligibilityDiscardRecord, transaction_generation),
        40
    );
    assert_eq!(GPU_CLOSED_LOOP_TICK_READBACK_BYTES, 48);
}

#[test]
fn eligibility_shader_parses_and_exposes_the_transaction_entries() {
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_ELIGIBILITY_WGSL)
        .expect("eligibility WGSL must parse");
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .expect("eligibility WGSL must validate");
    let entries = module
        .entry_points
        .iter()
        .map(|entry| entry.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        entries,
        [
            "accumulate_decoder_eligibility",
            "accumulate_recurrent_eligibility",
            "discard_pending_eligibility_arrays",
            "finalize_discard_pending_eligibility",
            "finalize_pending_eligibility",
        ]
        .into_iter()
        .collect()
    );

    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).unwrap();
    for (name, expected_span, expected_fields) in [
        (
            "GpuLearningHeader",
            80,
            vec![
                ("schema_version", 0),
                ("class_id", 4),
                ("slot", 8),
                ("slot_generation", 12),
                ("brain_slot_index", 16),
                ("active_activation_side", 20),
                ("dispatch_generation_lo", 24),
                ("dispatch_generation_hi", 28),
                ("candidate_count", 32),
                ("candidate_offset", 36),
                ("decoder_learning_input_offset", 40),
                ("selection_offset", 44),
                ("outcome_offset", 48),
                ("recurrent_synapse_count", 52),
                ("decoder_synapse_count", 56),
                ("decoder_input_stride", 60),
                ("pending_eligibility_offset", 64),
                ("reserved", 68),
            ],
        ),
        (
            "GpuPlasticityReceptorRecord",
            32,
            vec![
                ("eligibility_decay", 0),
                ("learning_rate", 4),
                ("sleep_replay_rate", 8),
                ("normalization_rate", 12),
                ("modulator_sign", 16),
                ("fast_min", 20),
                ("fast_max", 24),
                ("reserved", 28),
            ],
        ),
        (
            "GpuPendingEligibilityRecord",
            144,
            vec![
                ("schema_version", 0),
                ("slot", 4),
                ("slot_generation", 8),
                ("active_activation_side", 12),
                ("phenotype_hash", 16),
                ("organism_id", 48),
                ("dispatch_generation", 56),
                ("originating_tick", 64),
                ("frame_digest", 72),
                ("candidate_index_and_family", 104),
                ("action_id", 108),
                ("candidate_feature_digest", 112),
                ("active_eligibility_generation", 128),
                ("staging_eligibility_generation", 136),
            ],
        ),
    ] {
        let (handle, ty) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("missing reflected {name}"));
        let naga::TypeInner::Struct { members, span } = &ty.inner else {
            panic!("{name} is not a struct");
        };
        assert_eq!(*span, expected_span, "{name} declared span");
        assert_eq!(
            layouter[handle].size, expected_span,
            "{name} reflected size"
        );
        assert_eq!(members.len(), expected_fields.len(), "{name} member count");
        for (member, (field, offset)) in members.iter().zip(expected_fields) {
            assert_eq!(member.name.as_deref(), Some(field), "{name} field name");
            assert_eq!(member.offset, offset, "{name}.{field} offset");
        }
    }
}

#[test]
fn pending_transaction_blocks_the_next_frame_until_gpu_discard() {
    let mut backend = GpuClosedLoopBackend::new_required(support::scaling::bounded_profile(
        128 * 1024 * 1024,
        128 * 1024 * 1024,
        2,
        2,
    ))
    .expect("required Vulkan backend");
    let handle = backend
        .insert_brain(OrganismId(1), support::n512_phenotype(71))
        .unwrap();
    let first_frame = support::perception_frame(1, true, 2);
    let first = backend
        .tick_batch(&[(handle, first_frame)])
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(first.compact_readback_bytes, 48);
    assert_eq!(
        backend.pending_eligibility(handle).unwrap(),
        Some(first.pending_eligibility)
    );
    assert_eq!(
        backend
            .tick_batch(&[(
                handle,
                support::perception_frame_for_profile_at_tick(
                    1,
                    2,
                    alife_core::SensorProfile::PrivilegedAffordanceV1,
                    false,
                    2,
                )
            )])
            .unwrap_err(),
        ScaffoldContractError::LearningReplayRejected
    );

    let discard = backend
        .discard_pending_eligibility(handle, first.pending_eligibility.identity())
        .unwrap();
    assert_eq!(
        discard.discarded_staging_generation,
        first
            .pending_eligibility
            .identity()
            .staging_eligibility_generation()
    );
    assert_eq!(backend.pending_eligibility(handle).unwrap(), None);

    let second = backend
        .tick_batch(&[(
            handle,
            support::perception_frame_for_profile_at_tick(
                1,
                3,
                alife_core::SensorProfile::PrivilegedAffordanceV1,
                false,
                2,
            ),
        )])
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(
        second
            .pending_eligibility
            .identity()
            .active_eligibility_generation(),
        first
            .pending_eligibility
            .identity()
            .active_eligibility_generation()
    );
    backend
        .discard_pending_eligibility(handle, second.pending_eligibility.identity())
        .unwrap();
}

#[test]
fn discard_rejects_a_foreign_pending_identity_without_mutating_either_transaction() {
    let mut backend = GpuClosedLoopBackend::new_required(support::scaling::bounded_profile(
        128 * 1024 * 1024,
        128 * 1024 * 1024,
        2,
        2,
    ))
    .expect("required Vulkan backend");
    let first_handle = backend
        .insert_brain(OrganismId(1), support::n512_phenotype(72))
        .unwrap();
    let second_handle = backend
        .insert_brain(OrganismId(2), support::n512_phenotype(73))
        .unwrap();
    let ticks = backend
        .tick_batch(&[
            (first_handle, support::perception_frame(1, true, 2)),
            (second_handle, support::perception_frame(2, false, 2)),
        ])
        .unwrap();
    let first_pending = ticks[0].pending_eligibility;
    let second_pending = ticks[1].pending_eligibility;

    assert_eq!(
        backend
            .discard_pending_eligibility(first_handle, second_pending.identity())
            .unwrap_err(),
        ScaffoldContractError::LearningEvidenceMismatch
    );
    assert_eq!(
        backend.pending_eligibility(first_handle).unwrap(),
        Some(first_pending)
    );
    assert_eq!(
        backend.pending_eligibility(second_handle).unwrap(),
        Some(second_pending)
    );

    backend
        .discard_pending_eligibility(first_handle, first_pending.identity())
        .unwrap();
    backend
        .discard_pending_eligibility(second_handle, second_pending.identity())
        .unwrap();
}

#[test]
fn retirement_requires_the_pending_eligibility_transaction_to_be_resolved() {
    let mut backend = GpuClosedLoopBackend::new_required(support::scaling::bounded_profile(
        128 * 1024 * 1024,
        128 * 1024 * 1024,
        2,
        2,
    ))
    .expect("required Vulkan backend");
    let handle = backend
        .insert_brain(OrganismId(1), support::n512_phenotype(74))
        .unwrap();
    let tick = backend
        .tick_batch(&[(handle, support::perception_frame(1, true, 2))])
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    assert_eq!(
        backend.remove_brain(handle).unwrap_err(),
        ScaffoldContractError::LearningReplayRejected
    );
    assert_eq!(
        backend.pending_eligibility(handle).unwrap(),
        Some(tick.pending_eligibility)
    );
    backend
        .discard_pending_eligibility(handle, tick.pending_eligibility.identity())
        .unwrap();
    backend.remove_brain(handle).unwrap();
}

#[test]
fn stale_nonzero_pending_row_with_valid_zero_fails_typed_and_preserves_learning_banks() {
    pollster::block_on(async {
        let phenotype = support::controlled_n512_phenotype_at_maturation(0.5);
        let mut gpu = support::GpuPipelineFixture::new(&phenotype).await;
        gpu.write_stale_pending_word_with_valid_zero(0, 0xdead_beef);
        let before = gpu.read_all_mutable_words().await;
        let before_banks = gpu.slot_learning_banks_snapshot(&before, 0);
        let frame = support::perception_frame_for_profile_at_tick(
            1,
            19,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );

        let error = gpu.run_slot_expect_failure(0, &frame).await;
        assert_eq!(error, GpuClosedLoopError::SubmissionFailed);
        let after = gpu.read_all_mutable_words().await;
        assert_eq!(before_banks, gpu.slot_learning_banks_snapshot(&after, 0));
        assert_eq!(gpu.guard_canary_violations(&after), 0);
    });
}

#[test]
fn selected_candidate_builds_target_specific_eligibility() {
    pollster::block_on(async {
        let phenotype = support::controlled_n512_phenotype_at_maturation(0.5);
        let mut gpu = support::GpuPipelineFixture::new(&phenotype).await;
        let frame = support::perception_frame_for_profile_at_tick(
            1,
            91,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let before = gpu.read_all_mutable_words().await;
        let pending = gpu.run_slot_keep_pending(0, &frame).await;
        assert_eq!(pending.result.record.status, 1);
        let after = gpu.read_all_mutable_words().await;
        let slot = gpu.slot_for_test(0);
        let ranges = slot.word_ranges();
        let recurrent = range_words(&after, &ranges.recurrent_eligibility_bank_1_words);
        let decoder = range_words(&after, &ranges.decoder_eligibility_bank_1_words);
        assert!(recurrent.iter().any(|word| *word != 0));
        assert!(decoder.iter().any(|word| *word != 0));
        assert_eq!(
            range_words(&before, &ranges.fast_weight_words),
            range_words(&after, &ranges.fast_weight_words)
        );
        assert_eq!(
            range_words(&before, &ranges.lifetime_weight_words),
            range_words(&after, &ranges.lifetime_weight_words)
        );

        let selected_index = pending.result.record.candidate_index as usize;
        let selected = frame.candidates()[selected_index];
        assert_eq!(
            pending.pending.candidate_index_and_family & 0xffff,
            pending.result.record.candidate_index
        );
        assert_eq!(
            (pending.pending.candidate_index_and_family >> 16) & 0xff,
            u32::from(selected.family.raw())
        );
        assert_eq!(pending.pending.candidate_index_and_family >> 24, 0);
        assert_eq!(pending.pending.action_id, selected.action_id.raw());
        assert_eq!(
            pending.pending.candidate_feature_digest,
            split_u64x2(selected.feature_digest().unwrap().0)
        );
        assert_eq!(join_u64(pending.pending.active_eligibility_generation), 1);
        assert_eq!(join_u64(pending.pending.staging_eligibility_generation), 2);
        assert_eq!(gpu.guard_canary_violations(&after), 0);
        gpu.discard_pending_for_slot(0, &pending.pending);
    });
}

#[test]
fn action_decoder_eligibility_matches_a_real_gpu_finite_difference() {
    pollster::block_on(async {
        let phenotype = support::phenotype_for_capacity_at_maturation(
            BrainCapacityClass::n512(),
            0xB300_1001,
            0.35,
            SensorProfile::GroundedObjectSlotsV1,
        );
        let frame = support::perception_frame_for_profile_at_tick(
            1,
            916,
            SensorProfile::GroundedObjectSlotsV1,
            true,
            2,
        );
        let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
        let family = u32::from(frame.candidates()[0].family.raw());

        let mut baseline = support::GpuPipelineFixture::new(&phenotype).await;
        baseline.set_decoder_genetic_weights_zeroed(true);
        let base_pending = baseline.run_slot_keep_pending(0, &frame).await;
        assert_eq!(base_pending.result.record.status, 1);
        assert_eq!(base_pending.result.record.candidate_index, 0);
        let base_words = baseline.read_all_mutable_words().await;
        let base_ranges = baseline.slot_for_test(0).word_ranges();
        let activation_range = if base_pending.pending.active_activation_side == 0 {
            &base_ranges.activation_a_words
        } else {
            &base_ranges.activation_b_words
        };
        let activations = range_words(&base_words, activation_range)
            .into_iter()
            .map(f32::from_bits)
            .collect::<Vec<_>>();
        let metadata = upload
            .decoder_eligibility_metadata
            .iter()
            .find(|metadata| {
                metadata.decoder_head == DecoderHeadKind::ActionCandidate.raw()
                    && metadata.family == family
                    && metadata.input_lane < 24
                    && activations[metadata.motor_index as usize].abs() > 1.0e-4
                    && (frame.candidates()[0].features.0[metadata.input_lane as usize]
                        - frame.candidates()[1].features.0[metadata.input_lane as usize])
                        .abs()
                        > 1.0e-4
            })
            .copied()
            .expect("grounded candidates must expose a differentiating action lane");
        let motor = activations[metadata.motor_index as usize];
        let feature_0 = frame.candidates()[0].features.0[metadata.input_lane as usize];
        let feature_1 = frame.candidates()[1].features.0[metadata.input_lane as usize];
        let epsilon = 1.0e-3 * (motor * (feature_0 - feature_1)).signum();

        let mut perturbed = support::GpuPipelineFixture::new(&phenotype).await;
        perturbed.set_decoder_genetic_weights_zeroed(true);
        perturbed.set_genetic_weight_for_slot(0, metadata.global_synapse_id, epsilon);
        let perturbed_pending = perturbed.run_slot_keep_pending(0, &frame).await;
        assert_eq!(perturbed_pending.result.record.status, 1);
        assert_eq!(perturbed_pending.result.record.candidate_index, 0);

        let finite_difference = (f32::from_bits(perturbed_pending.result.record.logit_bits)
            - f32::from_bits(base_pending.result.record.logit_bits))
            / epsilon;
        let perturbed_words = perturbed.read_all_mutable_words().await;
        let eligibility_range = &perturbed
            .slot_for_test(0)
            .word_ranges()
            .decoder_eligibility_bank_1_words;
        let actual_eligibility = f32::from_bits(
            perturbed_words
                [eligibility_range.start as usize + metadata.eligibility_local_index as usize],
        );
        let expected_derivative = (motor * feature_0).clamp(-1.0, 1.0);
        assert!((finite_difference - expected_derivative).abs() <= 2.0e-3);
        assert!((actual_eligibility - expected_derivative).abs() <= 1.0e-6);

        baseline.discard_pending_for_slot(0, &base_pending.pending);
        perturbed.discard_pending_for_slot(0, &perturbed_pending.pending);
    });
}

#[test]
fn eligibility_decay_is_seeded_deterministic_and_bounded() {
    pollster::block_on(async {
        let phenotype = support::controlled_n512_phenotype_at_maturation(0.5);
        let frame = support::perception_frame_for_profile_at_tick(
            1,
            101,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let mut first = support::GpuPipelineFixture::new(&phenotype).await;
        let mut second = support::GpuPipelineFixture::new(&phenotype).await;
        first.seed_active_eligibility(0, 0.25, -0.4);
        second.seed_active_eligibility(0, 0.25, -0.4);
        let first_pending = first.run_slot_keep_pending(0, &frame).await;
        let second_pending = second.run_slot_keep_pending(0, &frame).await;
        let first_words = first.read_all_mutable_words().await;
        let second_words = second.read_all_mutable_words().await;
        let first_ranges = first.slot_for_test(0).word_ranges();
        let second_ranges = second.slot_for_test(0).word_ranges();
        let first_recurrent = range_words(
            &first_words,
            &first_ranges.recurrent_eligibility_bank_1_words,
        );
        let second_recurrent = range_words(
            &second_words,
            &second_ranges.recurrent_eligibility_bank_1_words,
        );
        let first_decoder =
            range_words(&first_words, &first_ranges.decoder_eligibility_bank_1_words);
        let second_decoder = range_words(
            &second_words,
            &second_ranges.decoder_eligibility_bank_1_words,
        );
        assert_eq!(first_recurrent, second_recurrent);
        assert_eq!(first_decoder, second_decoder);
        assert_eq!(first_pending.pending, second_pending.pending);
        assert_eq!(
            eligibility_digest(b"eligibility.recurrent.v1", &first_recurrent),
            eligibility_digest(b"eligibility.recurrent.v1", &second_recurrent)
        );
        let max_abs = first_recurrent
            .iter()
            .chain(&first_decoder)
            .map(|word| f32::from_bits(*word).abs())
            .fold(0.0_f32, f32::max);
        assert!(max_abs <= 1.0);
        assert_eq!(first.guard_canary_violations(&first_words), 0);
        assert_eq!(second.guard_canary_violations(&second_words), 0);
        first.discard_pending_for_slot(0, &first_pending.pending);
        second.discard_pending_for_slot(0, &second_pending.pending);
    });
}

#[test]
fn recurrent_eligibility_uses_explicit_non_aliasing_learning_metadata() {
    pollster::block_on(async {
        let phenotype = support::controlled_n512_phenotype_at_maturation(0.5);
        let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
        let mut gpu = support::GpuPipelineFixture::new(&phenotype).await;
        let frame = support::perception_frame_for_profile_at_tick(
            1,
            111,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let pending = gpu.run_slot_keep_pending(0, &frame).await;
        let words = gpu.read_all_mutable_words().await;
        let ranges = gpu.slot_for_test(0).word_ranges();
        let staged = range_words(&words, &ranges.recurrent_eligibility_bank_1_words);
        let (metadata_index, metadata) = upload
            .synapse_learning_metadata
            .iter()
            .enumerate()
            .take(phenotype.budgets().global.recurrent_synapses as usize)
            .find(|(_, metadata)| {
                metadata.source_neuron != metadata.target_neuron
                    && staged[metadata.eligibility_local_index as usize] != 0
            })
            .expect("fixture must produce a nonzero non-aliasing recurrent trace");
        assert_eq!(metadata.kind, 1);
        assert_eq!(metadata.global_synapse_id as usize, metadata_index);
        assert_eq!(metadata.eligibility_local_index as usize, metadata_index);
        assert_ne!(metadata.source_neuron, metadata.target_neuron);
        assert_ne!(
            ranges.source_index_words.start,
            ranges.synapse_learning_metadata_words.start
        );
        assert_eq!(gpu.guard_canary_violations(&words), 0);
        gpu.discard_pending_for_slot(0, &pending.pending);
    });
}

#[test]
fn eligibility_uses_final_and_prior_banks_for_two_three_and_four_microsteps() {
    pollster::block_on(async {
        for (maturation, microsteps) in [(0.2, 2_u8), (0.5, 3), (0.8, 4)] {
            let phenotype = support::controlled_n512_phenotype_at_maturation(maturation);
            assert_eq!(phenotype.microstep_count(), microsteps);
            let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
            let mut gpu = support::GpuPipelineFixture::new(&phenotype).await;
            let frame = support::perception_frame_for_profile_at_tick(
                1,
                120 + u64::from(microsteps),
                SensorProfile::PrivilegedAffordanceV1,
                true,
                2,
            );
            let pending = gpu.run_slot_keep_pending(0, &frame).await;
            let words = gpu.read_all_mutable_words().await;
            let ranges = gpu.slot_for_test(0).word_ranges();
            let final_side = u32::from(microsteps & 1);
            assert_eq!(pending.pending.active_activation_side, final_side);
            let (post_range, pre_range) = if final_side == 0 {
                (&ranges.activation_a_words, &ranges.activation_b_words)
            } else {
                (&ranges.activation_b_words, &ranges.activation_a_words)
            };
            let post = range_words(&words, post_range)
                .into_iter()
                .map(f32::from_bits)
                .collect::<Vec<_>>();
            let pre = range_words(&words, pre_range)
                .into_iter()
                .map(f32::from_bits)
                .collect::<Vec<_>>();
            let recurrent_actual = range_words(&words, &ranges.recurrent_eligibility_bank_1_words)
                .into_iter()
                .map(f32::from_bits)
                .collect::<Vec<_>>();
            let recurrent_expected = upload
                .synapse_learning_metadata
                .iter()
                .take(phenotype.budgets().global.recurrent_synapses as usize)
                .map(|metadata| {
                    (pre[metadata.source_neuron as usize] * post[metadata.target_neuron as usize])
                        .clamp(-1.0, 1.0)
                })
                .collect::<Vec<_>>();
            assert!(recurrent_actual
                .iter()
                .zip(&recurrent_expected)
                .all(|(actual, expected)| (*actual - *expected).abs() <= 1.0e-6));

            let selected = frame.candidates()[pending.result.record.candidate_index as usize];
            let decoder_actual = range_words(&words, &ranges.decoder_eligibility_bank_1_words)
                .into_iter()
                .map(f32::from_bits)
                .collect::<Vec<_>>();
            let decoder_expected = upload
                .decoder_eligibility_metadata
                .iter()
                .map(|metadata| {
                    if metadata.decoder_head == 1
                        && metadata.family == u32::from(selected.family.raw())
                    {
                        (post[metadata.motor_index as usize]
                            * selected.features.0[metadata.input_lane as usize])
                            .clamp(-1.0, 1.0)
                    } else {
                        0.0
                    }
                })
                .collect::<Vec<_>>();
            assert!(decoder_actual
                .iter()
                .zip(&decoder_expected)
                .all(|(actual, expected)| (*actual - *expected).abs() <= 1.0e-6));
            assert_eq!(
                quantized_eligibility_digest(b"eligibility.recurrent.side.v1", &recurrent_actual),
                quantized_eligibility_digest(b"eligibility.recurrent.side.v1", &recurrent_expected)
            );
            assert_eq!(
                quantized_eligibility_digest(b"eligibility.decoder.side.v1", &decoder_actual),
                quantized_eligibility_digest(b"eligibility.decoder.side.v1", &decoder_expected)
            );
            assert_eq!(gpu.guard_canary_violations(&words), 0);
            gpu.discard_pending_for_slot(0, &pending.pending);
        }
    });
}

#[test]
fn two_same_class_slots_keep_every_mutable_learning_region_isolated() {
    pollster::block_on(async {
        let phenotype = support::controlled_n512_phenotype_at_maturation(0.5);
        let frame_a = support::perception_frame_for_profile_at_tick(
            1,
            141,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let frame_b = support::perception_frame_for_profile_at_tick(
            2,
            141,
            SensorProfile::PrivilegedAffordanceV1,
            false,
            2,
        );
        assert_ne!(frame_a.organism_id(), frame_b.organism_id());

        let mut isolation = support::GpuPipelineFixture::new(&phenotype).await;
        let before_a_only = isolation.read_all_mutable_words().await;
        let before_b = isolation.slot_mutable_snapshot(&before_a_only, 1);
        let only_a = isolation.run_slot_keep_pending(0, &frame_a).await;
        let after_a_only = isolation.read_all_mutable_words().await;
        assert_ne!(
            isolation.slot_mutable_snapshot(&before_a_only, 0),
            isolation.slot_mutable_snapshot(&after_a_only, 0)
        );
        assert_eq!(before_b, isolation.slot_mutable_snapshot(&after_a_only, 1));
        assert_eq!(isolation.guard_canary_violations(&after_a_only), 0);
        isolation.discard_pending_for_slot(0, &only_a.pending);

        // Full-state comparisons must begin from one common authority checkpoint.
        // Restoring only GPU words cannot rewind the pipeline's monotonic authority
        // nonce, so use fresh fixtures for the batched and independent runs.
        let mut batched = support::GpuPipelineFixture::new(&phenotype).await;
        let mut independent_a = support::GpuPipelineFixture::new(&phenotype).await;
        let mut independent_b = support::GpuPipelineFixture::new(&phenotype).await;
        let batched_pending = batched
            .run_frame_pair_keep_pending([&frame_a, &frame_b])
            .await;
        let independent_a_pending = independent_a.run_slot_keep_pending(0, &frame_a).await;
        let independent_b_pending = independent_b.run_slot_keep_pending(1, &frame_b).await;
        let batched_words = batched.read_all_mutable_words().await;
        let independent_a_words = independent_a.read_all_mutable_words().await;
        let independent_b_words = independent_b.read_all_mutable_words().await;
        assert_eq!(
            eligibility_digest(
                b"eligibility.full-slot-a.v1",
                &batched.slot_mutable_snapshot(&batched_words, 0),
            ),
            eligibility_digest(
                b"eligibility.full-slot-a.v1",
                &independent_a.slot_mutable_snapshot(&independent_a_words, 0),
            )
        );
        assert_eq!(
            eligibility_digest(
                b"eligibility.full-slot-b.v1",
                &batched.slot_mutable_snapshot(&batched_words, 1),
            ),
            eligibility_digest(
                b"eligibility.full-slot-b.v1",
                &independent_b.slot_mutable_snapshot(&independent_b_words, 1),
            )
        );
        assert_eq!(batched.guard_canary_violations(&batched_words), 0);
        assert_eq!(
            independent_a.guard_canary_violations(&independent_a_words),
            0
        );
        assert_eq!(
            independent_b.guard_canary_violations(&independent_b_words),
            0
        );
        assert_eq!(
            (
                batched_pending[0].pending.organism_id,
                batched_pending[1].pending.organism_id
            ),
            ([1, 0], [2, 0])
        );

        batched.discard_pending_for_slot(0, &batched_pending[0].pending);
        batched.discard_pending_for_slot(1, &batched_pending[1].pending);
        independent_a.discard_pending_for_slot(0, &independent_a_pending.pending);
        independent_b.discard_pending_for_slot(1, &independent_b_pending.pending);
    });
}

#[test]
fn decoder_learning_metadata_uses_absolute_activation_indices() {
    let phenotype = support::n512_phenotype(41);
    let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
    for metadata in &upload.decoder_eligibility_metadata {
        let synapse = &upload.synapse_learning_metadata[metadata.global_synapse_id as usize];
        assert_eq!(metadata.motor_index, synapse.source_neuron);
        assert!(metadata.motor_index < phenotype.neuron_count());
    }
}
