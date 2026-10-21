//! Episodic encoding, cognitive-context replacement, and finalization boundaries.
use super::*;
use crate::{
    ActionCandidate, ActionTarget, BodySnapshot, CandidateFeatureVector, CandidateObservationRef,
    DriveSnapshot, DurationTicks, EndocrineSnapshot, HomeostaticSnapshot, Pose, SensorProfile,
    SensorProfileProvenance, SensoryAbiVersion, SensoryChannels, SensorySnapshot, Vec3f, Velocity,
};

fn fixture(count: usize) -> (MemoryBank, PerceptionFrameDraft) {
    let owner = OrganismId(811);
    let tick = Tick::new(40);
    let profile = SensorProfile::PrivilegedAffordanceV1;
    let candidates = (0..count)
        .map(|index| {
            ActionCandidate::new(
                index as u16,
                ActionId(200 + index as u32),
                ActionKind::Idle,
                CandidateActionFamily::Idle,
                CandidateObservationRef::None,
                ActionTarget::NONE,
                CandidateFeatureVector::zero(),
                Confidence::new(0.9).unwrap(),
                NormalizedScalar::new(0.2).unwrap(),
                DurationTicks::new(1),
                DurationTicks::new(2),
            )
            .unwrap()
        })
        .collect();
    let draft = PerceptionFrameDraft::new(
        owner,
        tick,
        profile,
        SensorySnapshot::new(
            owner,
            tick,
            Vec3f::ZERO,
            SensoryChannels::ZERO,
            Default::default(),
        )
        .unwrap(),
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::new(
            tick,
            DriveSnapshot::baseline(),
            EndocrineSnapshot::baseline(),
        )
        .unwrap(),
        candidates,
        SensorProfileProvenance::new(profile, SensoryAbiVersion::CURRENT, tick).unwrap(),
        Vec::new(),
    )
    .unwrap();
    let bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    (bank, draft)
}

fn prepared(
    bank: &MemoryBank,
    draft: &PerceptionFrameDraft,
    runtime: bool,
) -> PreparedMemoryRecall {
    if runtime {
        bank.recall_frame_runtime(draft, &Default::default(), true)
            .unwrap()
            .0
    } else {
        bank.recall_frame(draft).unwrap()
    }
}

fn cognitive(draft: &PerceptionFrameDraft, sequence: u64) -> CognitiveContextFrame {
    CognitiveContextFrame::empty(
        draft.organism_id(),
        ExperienceSequenceId(sequence),
        draft.tick(),
    )
    .unwrap()
}

#[test]
fn repeated_cognitive_context_keeps_episodic_encoding_and_final_keys() {
    for runtime in [false, true] {
        for count in [1, 3, MAX_ACTION_CANDIDATES] {
            let (bank, draft) = fixture(count);
            let mut recall = prepared(&bank, &draft, runtime);
            let expected_context = recall.context.to_perception_context_block().unwrap();
            for sequence in [101, 102, 103] {
                recall = recall
                    .with_cognitive_context(cognitive(&draft, sequence))
                    .unwrap();
                assert_eq!(recall.encoded_context, expected_context);
            }
            let (frame, finalized) = recall.finalize(draft).unwrap();
            assert_eq!(frame.context(), &expected_context);
            assert_eq!(finalized.candidate_keys.len(), count);
            finalized.validate_for_frame(&frame).unwrap();
        }
    }
}

#[test]
fn cloned_preparations_own_independent_payloads_and_finalize_independently() {
    for runtime in [false, true] {
        let (bank, draft) = fixture(3);
        let original = prepared(&bank, &draft, runtime);
        let cloned = original.clone();
        assert_eq!(original, cloned);
        let first = original
            .with_cognitive_context(cognitive(&draft, 101))
            .unwrap()
            .finalize(draft.clone())
            .unwrap();
        let second = cloned
            .with_cognitive_context(cognitive(&draft, 102))
            .unwrap()
            .finalize(draft)
            .unwrap();
        assert_eq!(first.0, second.0);
        assert_eq!(first.1.candidate_keys, second.1.candidate_keys);
        assert_ne!(first.1.cognitive_context, second.1.cognitive_context);
    }
}

#[test]
fn finalization_rejects_changed_drafts_receipts_and_retained_blocks() {
    for runtime in [false, true] {
        let (bank, draft) = fixture(3);
        let recall = prepared(&bank, &draft, runtime);
        let changed = draft.clone().with_remembered_novelty(0.5).unwrap();
        assert!(recall.clone().finalize(changed).is_err());
        for fault in 0..3 {
            let mut broken = recall.clone();
            match fault {
                0 => broken.receipt.context_digest.0[0] ^= 1,
                1 => broken.context.candidates[0].target_latent[0] = f32::NAN,
                2 => {
                    let mut values = broken.encoded_context.values().to_vec();
                    values[0] = 0.5;
                    broken.encoded_context = PerceptionContextBlock::try_new(
                        broken.encoded_context.schema_version(),
                        broken.encoded_context.context_kind(),
                        values,
                    )
                    .unwrap();
                }
                _ => unreachable!(),
            }
            assert_eq!(
                broken.finalize(draft.clone()).unwrap_err(),
                ScaffoldContractError::InvalidMemoryQuery
            );
        }
        // A failed consuming finalization leaves another owned clone usable.
        recall.finalize(draft).unwrap();
    }
}
