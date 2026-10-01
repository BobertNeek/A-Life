use std::hint::black_box;
use std::time::Instant;

use alife_core::{CognitiveContextFrame, ExperienceSequenceId, Tick};

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
    for count in [2, 8, 26] {
        for learned_records in [0, 64] {
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
                            confidence: alife_core::NormalizedScalar::new(
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
            let prepared = recall.with_cognitive_context(cognitive).unwrap();
            let (frame, finalized) = prepared.clone().finalize(draft.clone()).unwrap();
            let recall_ns = samples(|| {
                black_box(bank.recall_frame(black_box(&draft)).unwrap());
            });
            let finalize_ns = samples(|| {
                black_box(prepared.clone().finalize(draft.clone()).unwrap());
            });
            let validate_ns = samples(|| {
                black_box(finalized.validate_for_frame(black_box(&frame)).unwrap());
            });
            let proof = serde_json::to_vec(&(
                &frame,
                finalized.context(),
                finalized.candidate_keys(),
                finalized.receipt(),
                finalized.cognitive_context(),
                &bank,
            ))
            .unwrap();
            println!(
                "candidates={count} learned_records={learned_records} recall_ns={recall_ns:?} finalize_ns={finalize_ns:?} validate_ns={validate_ns:?} proof_bytes={} proof_blake3={}",
                proof.len(),
                blake3::hash(&proof),
            );
        }
    }
}
