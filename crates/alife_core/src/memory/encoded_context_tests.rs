//! Frozen reconstruction references exercise the retained encoding's lifecycle.
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

// These two helpers preserve the pre-change reconstruction path independently.
fn legacy_with_cognitive_context(
    mut recall: PreparedMemoryRecall,
    cognitive: CognitiveContextFrame,
) -> Result<PreparedMemoryRecall, ScaffoldContractError> {
    cognitive.validate_contract()?;
    recall.cognitive_context = Some(cognitive);
    recall.receipt.context_digest = recall
        .context
        .to_perception_context_block()?
        .canonical_digest();
    recall.receipt.validate_contract()?;
    Ok(recall)
}

fn legacy_finalize(
    recall: PreparedMemoryRecall,
    draft: PerceptionFrameDraft,
) -> Result<(PerceptionFrame, FinalizedMemoryRecall), ScaffoldContractError> {
    recall.validate_for_draft(&draft)?;
    let context_block = recall.context.to_perception_context_block()?;
    if context_block.canonical_digest() != recall.receipt.context_digest {
        return Err(ScaffoldContractError::InvalidMemoryQuery);
    }
    let context_digest = context_block.canonical_digest();
    let frame = draft.finalize(context_block)?;
    let final_frame_digest = frame.frame_digest();
    let candidate_keys = recall
        .candidate_queries
        .into_iter()
        .map(|query| EpisodicDecisionKeyV2::try_new(query, context_digest, final_frame_digest))
        .collect::<Result<Vec<_>, _>>()?;
    let finalized = FinalizedMemoryRecall {
        context: recall.context,
        base_frame_digest: recall.base_frame_digest,
        context_digest,
        final_frame_digest,
        candidate_keys,
        receipt: recall.receipt,
        cognitive_context: recall.cognitive_context,
        social_recognition: recall.social_recognition,
    };
    #[cfg(debug_assertions)]
    finalized.validate_for_frame(&frame)?;
    Ok((frame, finalized))
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
fn retained_encoding_matches_reconstruction_for_both_constructors_and_repeated_contexts() {
    for runtime in [false, true] {
        for count in [1, 3, MAX_ACTION_CANDIDATES] {
            let (bank, draft) = fixture(count);
            let mut actual = prepared(&bank, &draft, runtime);
            let mut reference = actual.clone();
            let ptr = actual.encoded_context.values().as_ptr();
            for sequence in [101, 102, 103] {
                actual = actual
                    .with_cognitive_context(cognitive(&draft, sequence))
                    .unwrap();
                reference =
                    legacy_with_cognitive_context(reference, cognitive(&draft, sequence)).unwrap();
                assert_eq!(actual, reference);
                assert_eq!(actual.encoded_context.values().as_ptr(), ptr);
            }
            let (frame, finalized) = actual.finalize(draft.clone()).unwrap();
            assert_eq!(
                frame.context().values().as_ptr(),
                ptr,
                "finalization moves the retained Vec"
            );
            let expected = legacy_finalize(reference, draft).unwrap();
            assert_eq!((&frame, &finalized), (&expected.0, &expected.1));
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
        assert_ne!(
            original.encoded_context.values().as_ptr(),
            cloned.encoded_context.values().as_ptr()
        );
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
fn retained_encoding_preserves_cognitive_episodic_and_receipt_error_order() {
    for runtime in [false, true] {
        let (bank, draft) = fixture(3);
        for fault in 0..4 {
            let mut recall = prepared(&bank, &draft, runtime);
            let mut context = cognitive(&draft, 101);
            match fault {
                0 => {
                    context.schema_version = 0;
                    recall.context.schema_version = 0;
                }
                1 => recall.context.schema_version = 0,
                2 => recall.context.candidates[0].target_latent[0] = f32::NAN,
                3 => recall.receipt.organism_id_raw = 0,
                _ => unreachable!(),
            }
            assert_eq!(
                recall
                    .clone()
                    .with_cognitive_context(context.clone())
                    .unwrap_err(),
                legacy_with_cognitive_context(recall, context).unwrap_err(),
            );
        }
    }
}

#[test]
fn finalization_rejects_changed_drafts_receipts_and_retained_blocks() {
    for runtime in [false, true] {
        let (bank, draft) = fixture(3);
        let recall = prepared(&bank, &draft, runtime);
        let changed = draft.clone().with_remembered_novelty(0.5).unwrap();
        assert_eq!(
            recall.clone().finalize(changed.clone()),
            legacy_finalize(recall.clone(), changed)
        );
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
            if fault < 2 {
                assert_eq!(
                    broken.clone().finalize(draft.clone()),
                    legacy_finalize(broken, draft.clone())
                );
            } else {
                assert_eq!(
                    broken.finalize(draft.clone()).unwrap_err(),
                    ScaffoldContractError::InvalidMemoryQuery
                );
            }
        }
        // A failed consuming finalization leaves another owned clone usable.
        recall.finalize(draft).unwrap();
    }
}
