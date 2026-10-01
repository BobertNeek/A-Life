//! One-off CPU upload-construction comparison; never opens a GPU device.
use alife_core::{
    BrainCapacityClass, CognitiveContextFrame, ExperienceSequenceId, NormalizedScalar,
    PhenotypeCompiler, Tick,
};
use alife_gpu_backend::{GpuClassBucketPlan, GpuMemoryContextUpload, GpuPerceptionUpload};
use std::hint::black_box;
use std::time::Instant;

#[allow(dead_code, unused_imports)]
mod fixture {
    include!("../../../crates/alife_core/tests/candidate_memory_retrieval.rs");

    pub fn input(count: usize, learned_records: usize) -> (PerceptionFrameDraft, MemoryBank) {
        let template = grounded_draft(0.4);
        let families = [
            CandidateActionFamily::Approach,
            CandidateActionFamily::Avoid,
            CandidateActionFamily::Contact,
            CandidateActionFamily::Ingest,
        ];
        let candidates = (0..count)
            .map(|index| {
                candidate_for_family(
                    &template.grounded_object_slots()[index % 2],
                    index as u16,
                    families[index % families.len()],
                )
            })
            .collect();
        let tick = Tick::new(1_000);
        let draft = PerceptionFrameDraft::new(
            ORGANISM,
            tick,
            SensorProfile::GroundedObjectSlotsV1,
            SensorySnapshot::new(
                ORGANISM,
                tick,
                Vec3f::ZERO,
                template.sensory().channels,
                Default::default(),
            )
            .unwrap(),
            template.body(),
            HomeostaticSnapshot::new(
                tick,
                template.homeostasis().drives,
                template.homeostasis().hormones,
            )
            .unwrap(),
            candidates,
            SensorProfileProvenance::new(
                SensorProfile::GroundedObjectSlotsV1,
                SensoryAbiVersion::CURRENT,
                tick,
            )
            .unwrap(),
            template.grounded_object_slots().to_vec(),
        )
        .unwrap();
        let mut bank = MemoryBank::new(
            MemoryBankConfig::new(64, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
        )
        .unwrap();
        for index in 0..learned_records {
            let object = if index == 0 {
                slot(0, 71, 0.4, [0.0, 0.8, 0.9])
            } else if index == 1 {
                slot(0, 72, 0.7, [0.9, 0.6, 0.1])
            } else {
                slot(0, 1_000 + index as u64, 0.4, [0.2, 0.5, 0.8])
            };
            bank.observe_sealed_patch(&sequenced_patch_for_object(
                index as u64 + 1,
                index as u64 * 2 + 2,
                object,
                -0.8,
                0.7,
            ))
            .unwrap();
        }
        (draft, bank)
    }
}

fn samples(mut operation: impl FnMut()) -> Vec<u64> {
    for _ in 0..20 {
        for _ in 0..8 {
            operation();
        }
    }
    (0..5)
        .map(|_| {
            let started = Instant::now();
            for _ in 0..20 {
                for _ in 0..8 {
                    operation();
                }
            }
            started.elapsed().as_nanos() as u64 / 20
        })
        .collect()
}

fn main() {
    assert!(
        !cfg!(debug_assertions),
        "release comparison requires debug assertions disabled"
    );
    let capacity = BrainCapacityClass::n512();
    let asset = alife_core::FoundationWeightAsset::decode_canonical(include_bytes!(
        "../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
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
    let mut bucket = GpuClassBucketPlan::new(capacity, 1).unwrap();
    let slot = bucket.insert_phenotype(0, 1, &phenotype).unwrap();
    assert_eq!(slot.decoder_input_stride(), 54);
    for count in [2, 8, 26] {
        for learned_records in [0, 64] {
            for projection_present in [false, true] {
                let (draft, bank) = fixture::input(count, learned_records);
                let mut cognitive = CognitiveContextFrame::empty(
                    draft.organism_id(),
                    ExperienceSequenceId(1_001),
                    Tick::new(1_000),
                )
                .unwrap();
                let recall = bank.recall_frame(&draft).unwrap();
                for candidate in &recall.context().candidates {
                    if let Some(id) = candidate.best_family_source {
                        cognitive
                            .memory
                            .expectancies
                            .push(alife_core::CognitiveMemoryExpectancy {
                                memory_id: id,
                                expected_valence: alife_core::SignedValence::new(
                                    candidate.family_value[0],
                                )
                                .unwrap(),
                                confidence: NormalizedScalar::new(
                                    candidate.family_confidence.raw(),
                                )
                                .unwrap(),
                            });
                        if cognitive.memory.expectancies.len()
                            == alife_core::MAX_CONTEXT_MEMORY_EXPECTANCIES
                        {
                            break;
                        }
                    }
                }
                if projection_present {
                    let state = alife_core::SemanticStateVector::new(vec![0.4; 13]).unwrap();
                    let motor = alife_core::JointMotorCondition::new(vec![
                        alife_core::MotorChannelFactor {
                            channel: alife_core::MotorChannel::Locomotion,
                            primitive: alife_core::ActionId(200),
                            intensity: 0.8,
                            duration_ticks: 2,
                            direction: alife_core::Vec3f::new(1.0, 0.0, 0.0),
                            stand_off_distance: 0.0,
                            confidence: 0.9,
                            target: None,
                            payload: Vec::new(),
                            coordination_group: 0,
                        },
                    ])
                    .unwrap();
                    let mut prediction = alife_core::GroundedSuccessorPredictor::default()
                        .predict(&state, &motor)
                        .unwrap();
                    prediction.source_digest = draft.base_digest().0;
                    cognitive.cognitive_projection = Some(alife_core::cognitive_context::CognitiveProjectionFrame {
                        schema_version: alife_core::cognitive_context::CognitiveProjectionFrame::SCHEMA_VERSION,
                        base_frame_digest: draft.base_digest(),
                        candidates: draft.candidates().iter().map(|candidate| alife_core::cognitive_context::CognitiveCandidateInput {
                            candidate_index: candidate.candidate_index,
                            candidate_feature_digest: candidate.feature_digest().unwrap(),
                            tracked_object_id: None,
                            prediction: prediction.clone(),
                            forecast_available: false,
                            concept_match: NormalizedScalar::new(0.2).unwrap(),
                            gap_match: NormalizedScalar::new(0.3).unwrap(),
                            prior_residual: NormalizedScalar::new(0.4).unwrap(),
                        }).collect(),
                        objects: Vec::new(),
                    });
                }
                let (frame, finalized) = recall
                    .with_cognitive_context(cognitive)
                    .unwrap()
                    .finalize(draft)
                    .unwrap();
                let perception = GpuPerceptionUpload::try_from_frame(&frame, &slot, 0).unwrap();
                let upload = GpuMemoryContextUpload::try_from_finalized(
                    &frame,
                    &finalized,
                    perception.frame_binding,
                    &slot,
                )
                .unwrap();
                upload.validate_against(&frame, &finalized, &slot).unwrap();
                let constructor_ns = samples(|| {
                    black_box(
                        GpuMemoryContextUpload::try_from_finalized(
                            black_box(&frame),
                            black_box(&finalized),
                            black_box(perception.frame_binding),
                            black_box(&slot),
                        )
                        .unwrap(),
                    );
                });
                let explicit_validation_ns = samples(|| {
                    black_box(
                        upload
                            .validate_against(
                                black_box(&frame),
                                black_box(&finalized),
                                black_box(&slot),
                            )
                            .unwrap(),
                    );
                });
                let recall_validation_ns = samples(|| {
                    black_box(finalized.validate_for_frame(black_box(&frame)).unwrap());
                });
                let mut payload = Vec::new();
                payload.extend_from_slice(upload.header.words());
                for row in &upload.records {
                    payload.extend_from_slice(row.words());
                }
                for row in &upload.cognitive_records {
                    payload.extend_from_slice(row.words());
                }
                let payload_bytes: Vec<u8> =
                    payload.iter().flat_map(|word| word.to_le_bytes()).collect();
                let host_proof = format!("{frame:?}\n{finalized:?}\n{bank:?}\n{upload:?}");
                println!("{{\"candidates\":{count},\"learned_records\":{learned_records},\"projection_present\":{projection_present},\"decoder_stride\":{},\"constructor_ns\":{constructor_ns:?},\"explicit_validation_ns\":{explicit_validation_ns:?},\"recall_validation_ns\":{recall_validation_ns:?},\"payload_bytes\":{},\"payload_words\":{payload:?},\"payload_blake3\":\"{}\",\"host_proof_bytes\":{},\"host_proof_blake3\":\"{}\"}}", slot.decoder_input_stride(), payload_bytes.len(), blake3::hash(&payload_bytes), host_proof.len(), blake3::hash(host_proof.as_bytes()));
            }
        }
    }
}
