#![cfg(feature = "gpu-tests")]

use alife_core::{
    ActionId, ActionKind, CandidateActionFamily, DecoderHeadKind, PerceptionFrame, SensorProfile,
};
use alife_gpu_backend::GpuPhenotypeUpload;
mod support;

#[test]
fn joint_non_global_executed_locomotion_accumulates_eligibility() {
    pollster::block_on(async {
        let phenotype = support::controlled_n512_phenotype_at_maturation(0.5);
        let mut gpu = support::GpuPipelineFixture::new(&phenotype).await;
        let original = support::perception_frame_for_profile_at_tick(
            1,
            91,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let mut candidates = original.candidates().to_vec();
        candidates[1].kind = ActionKind::Move;
        candidates[1].family = CandidateActionFamily::Avoid;
        candidates[1].action_id = ActionId(102);
        let frame = PerceptionFrame::new(
            original.organism_id(),
            original.tick(),
            original.sensor_profile(),
            original.sensory().clone(),
            original.body(),
            original.homeostasis().clone(),
            candidates,
            original.profile_provenance(),
            original.grounded_object_slots().to_vec(),
        )
        .unwrap();
        gpu.set_decoder_genetic_weights_zeroed(true);
        let (frame, recall) = support::empty_recall(&frame);
        let (pending, _) = gpu.run_memory_frame_keep_pending(&frame, &recall).await;
        assert_eq!(
            pending.result.record.candidate_index, 0,
            "global Inspect must be distinct from executed locomotion"
        );
        let words = gpu.read_all_mutable_words().await;
        eprintln!("adapter={}", pending.result.adapter_identity);
        let slot = gpu.slot_for_test(0);
        assert_eq!(
            words[slot.word_ranges().speech_payload_words.start as usize + 2] & 0xff,
            2,
            "GPU must select candidate 1 for the enabled locomotion channel"
        );
        let activation = if pending.pending.active_activation_side == 0 {
            &slot.word_ranges().activation_a_words
        } else {
            &slot.word_ranges().activation_b_words
        };
        let range = &gpu
            .slot_for_test(0)
            .word_ranges()
            .decoder_eligibility_bank_1_words;
        let upload = GpuPhenotypeUpload::try_from(&phenotype).unwrap();
        assert!(
            upload
                .decoder_eligibility_metadata
                .iter()
                .any(
                    |row| row.decoder_head == DecoderHeadKind::ActionCandidate.raw()
                        && row.family == u32::from(CandidateActionFamily::Avoid.raw())
                        && f32::from_bits(
                            words[activation.start as usize + row.motor_index as usize]
                        )
                        .abs()
                            > 1e-7
                        && frame.candidates()[1].features.0[row.input_lane as usize].abs() > 1e-7
                ),
            "fixture needs a nonzero pre/post pair for the selected locomotion candidate"
        );
        let values: Vec<_> = upload
            .decoder_eligibility_metadata
            .iter()
            .filter(|row| {
                row.decoder_head == DecoderHeadKind::ActionCandidate.raw()
                    && row.family == u32::from(CandidateActionFamily::Avoid.raw())
            })
            .map(|row| {
                f32::from_bits(words[range.start as usize + row.eligibility_local_index as usize])
            })
            .collect();
        assert!(!values.is_empty());
        assert!(
            values.iter().any(|v| v.abs() > 1e-7),
            "executed non-global locomotion must receive current-action eligibility: {values:?}"
        );
        assert_eq!(gpu.guard_canary_violations(&words), 0);
        gpu.discard_pending_for_slot(0, &pending.pending);
    });
}
