//! One-off CPU preparation probe; never creates or dispatches a GPU device.
use alife_core::*;
use alife_gpu_backend::{
    GpuBrainSlot, GpuClassBucketPlan, GpuMemoryContextUpload, GpuPerceptionUpload,
};
use alife_world::grounded_peripheral_summaries;
use std::collections::{BTreeMap, BTreeSet};
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
            MemoryBankConfig::new(256, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
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

// Generated verbatim from the app's private CPU preparation functions; see the
// dated report for the exact extraction command and source hash.
include!(env!("ALIFE_ATTENTION_APP_HELPERS"));

type CpuResult = (
    Vec<FinalizedMemoryAttentionEvidence>,
    Vec<PeripheralSummary>,
    AttentionFrame,
    PerceptionFrame,
    FinalizedMemoryRecall,
    GpuPerceptionUpload,
    GpuMemoryContextUpload,
);

fn topology_fixture(draft: &PerceptionFrameDraft) -> TopologySidecar {
    let topology = TopologySidecar::new_profiled(
        draft.organism_id(),
        draft.profile_provenance().identity(),
        TopologicalMapConfig::default(),
    )
    .unwrap();
    let mut concept = ConceptCell::new(
        ConceptCellId(1),
        ConceptBindings {
            objects: vec![draft.grounded_object_slots()[1].tracked_object_id],
            ..ConceptBindings::default()
        },
    )
    .unwrap();
    concept.confidence = Confidence(1.0);
    concept.salience = NormalizedScalar(1.0);
    let gap = UnresolvedGap {
        id: UnresolvedGapId(1),
        source_concepts: vec![concept.id],
        contradiction_type: ContradictionType::PredictionError,
        prediction_error: NormalizedScalar(1.0),
        curiosity_voltage: NormalizedScalar(1.0),
        salience: NormalizedScalar(1.0),
        first_tick: draft.tick(),
        last_tick: draft.tick(),
        confidence: Confidence(1.0),
        status: GapResolutionStatus::Open,
    };
    let mut wire = serde_json::to_value(topology).unwrap();
    wire["map"]["concepts"] = serde_json::to_value(vec![concept]).unwrap();
    wire["map"]["unresolved_gaps"] = serde_json::to_value(vec![gap]).unwrap();
    wire["map"]["next_concept_id"] = serde_json::json!(2);
    wire["map"]["next_gap_id"] = serde_json::json!(2);
    let mut topology: TopologySidecar = serde_json::from_value(wire).unwrap();
    topology.decay_edges(0).unwrap();
    topology.validate_contract().unwrap();
    topology
}

fn full_cpu_preparation(
    draft: &PerceptionFrameDraft,
    bank: &MemoryBank,
    topology: &TopologySidecar,
    slot: &GpuBrainSlot,
    prepared_attention: bool,
) -> CpuResult {
    let sequence = ExperienceSequenceId(1_001);
    let recall = bank.recall_frame(draft).unwrap();
    let context =
        cognitive_context_for_recall(draft.organism_id(), sequence, &recall, topology).unwrap();
    let mut baseline_finalization = None;
    let evidence = if prepared_attention {
        recall
            .with_cognitive_context(context.clone())
            .unwrap()
            .attention_evidence_for_draft(draft)
            .unwrap()
    } else {
        let prepared = recall
            .clone()
            .with_cognitive_context(context.clone())
            .unwrap();
        let (frame, finalized) = prepared.finalize(draft.clone()).unwrap();
        finalized.validate_for_frame(&frame).unwrap();
        let evidence = finalized_memory_attention_evidence(&finalized).unwrap();
        baseline_finalization = Some((frame, finalized));
        evidence
    };
    let topology_evidence = topology_evidence_for_draft(draft, topology).unwrap();
    // Valid fixed receptor input. Physiology/receptor derivation is outside this probe.
    let receptors = NeuralReceptorEffects {
        source_tick: draft.tick(),
        source_chemistry_version: 1,
        interoceptive_gain: 1.0,
        regional_excitability: 1.5,
        projection_gain: 1.0,
        local_threshold_shift: 0.0,
        attention_gain: 1.5,
        plasticity_appetitive: 0.5,
        plasticity_aversive: 0.5,
        structural_growth_gate: 0.5,
        sleep_gate: 0.0,
        consolidation_gate: 0.5,
    };
    let mut summaries = grounded_peripheral_summaries(draft.grounded_object_slots()).unwrap();
    let body_need = draft
        .homeostasis()
        .drives
        .to_array()
        .iter()
        .copied()
        .fold(0.0, f32::max);
    apply_predecision_attention_evidence(
        &mut summaries,
        body_need,
        &evidence,
        &context,
        &topology_evidence,
        receptors,
    )
    .unwrap();
    for summary in &mut summaries {
        if let StableFocusIdentity::TrackedObject(id) = summary.identity {
            summary.salience.novelty = NormalizedScalar::new(
                1.0 - bank.object_familiarity(
                    draft.organism_id(),
                    id,
                    draft.profile_provenance().identity(),
                ),
            )
            .unwrap();
        }
    }
    let attention = select_focal_targets(
        draft.organism_id(),
        sequence,
        draft.tick(),
        &summaries,
        HysteresisState::default(),
        AttentionSelectionPolicy {
            focal_capacity: 1,
            protected_minimum: 1,
            requested_focal_count: 1,
            switch_cost: NormalizedScalar(0.01),
            hysteresis_margin: NormalizedScalar(0.01),
        },
    )
    .unwrap();
    let novelty = attention
        .focal_targets
        .first()
        .and_then(|id| match id {
            StableFocusIdentity::TrackedObject(id) => Some(
                1.0 - bank.object_familiarity(
                    draft.organism_id(),
                    *id,
                    draft.profile_provenance().identity(),
                ),
            ),
            _ => None,
        })
        .unwrap_or(0.0);
    let routed = route_focal_candidates(draft.clone(), &attention)
        .unwrap()
        .with_remembered_novelty(novelty)
        .unwrap();
    let recall = bank.recall_frame(&routed).unwrap();
    let context =
        cognitive_context_for_recall(draft.organism_id(), sequence, &recall, topology).unwrap();
    let context = cognitive_context_with_attention(context, attention.clone()).unwrap();
    let projection = cognitive_projection_for_draft(
        &routed,
        &recall,
        sequence,
        &GroundedSuccessorPredictor::default(),
        &topology_evidence,
    )
    .unwrap();
    let context = cognitive_context_with_projection(context, projection).unwrap();
    let (frame, finalized) = recall
        .with_cognitive_context(context)
        .unwrap()
        .finalize(routed)
        .unwrap();
    finalized.validate_for_frame(&frame).unwrap();
    let perception = GpuPerceptionUpload::try_from_frame(&frame, slot, 0).unwrap();
    let upload = GpuMemoryContextUpload::try_from_finalized(
        &frame,
        &finalized,
        perception.frame_binding,
        slot,
    )
    .unwrap()
    .bind_neural_receptor_effects(receptors)
    .unwrap();
    black_box(&baseline_finalization); // Keep legacy unused frame/keys alive through preparation.
    (
        evidence, summaries, attention, frame, finalized, perception, upload,
    )
}

fn paired_samples(before: &dyn Fn(), after: &dyn Fn(), after_first: bool) -> (Vec<u64>, Vec<u64>) {
    for _ in 0..3 {
        before();
        after();
    }
    let sample = |operation: &dyn Fn()| {
        let start = Instant::now();
        for _ in 0..3 {
            operation();
        }
        start.elapsed().as_nanos() as u64 / 3
    };
    let mut b = Vec::new();
    let mut a = Vec::new();
    for index in 0..5 {
        if (index % 2 == 0) == after_first {
            a.push(sample(after));
            b.push(sample(before));
        } else {
            b.push(sample(before));
            a.push(sample(after));
        }
    }
    (b, a)
}

fn main() {
    assert!(!cfg!(debug_assertions));
    let after_first = std::env::args().any(|arg| arg == "--after-first");
    let asset = FoundationWeightAsset::decode_canonical(include_bytes!(
        "../../../assets/founders/scaled-choice-nociceptive-v1/candidate.alife-foundation"
    ))
    .unwrap();
    let manifest = asset.manifest();
    let identity = FoundationGeneticIdentity::new(
        manifest.foundation_id().raw(),
        manifest.foundation_version().raw() as u16,
        manifest.compatibility_family_id().raw(),
        manifest.capacity_class_id(),
    )
    .unwrap();
    let extension = CognitiveChannelExtensionV1::try_new_v1(
        identity,
        ActionCandidateCreditProfileV1::SignedChoiceReadouts,
    )
    .unwrap();
    let candidate =
        Nano512ActionCreditCandidateV2::new_with_cognitive_extension(&asset, extension).unwrap();
    let (phenotype, _) =
        PhenotypeCompiler::compile_nano512_action_credit_candidate(&candidate).unwrap();
    let mut bucket = GpuClassBucketPlan::new(BrainCapacityClass::n512(), 1).unwrap();
    let slot = bucket.insert_phenotype(0, 1, &phenotype).unwrap();
    assert_eq!(slot.decoder_input_stride(), 54);
    for candidates in [2, 8, 26] {
        for records in [0, 64, 256] {
            for residents in [8, 16, 32, 50] {
                let cases: Vec<_> = (0..residents)
                    .map(|_| {
                        let (draft, bank) = fixture::input(candidates, records);
                        let topology = topology_fixture(&draft);
                        let recall = bank.recall_frame(&draft).unwrap();
                        let context = cognitive_context_for_recall(
                            draft.organism_id(),
                            ExperienceSequenceId(1_001),
                            &recall,
                            &topology,
                        )
                        .unwrap();
                        let prepared = recall.with_cognitive_context(context).unwrap();
                        (draft, prepared, bank, topology)
                    })
                    .collect();
                let phase_before = || {
                    for (draft, prepared, _, _) in &cases {
                        let (frame, recall) = prepared.clone().finalize(draft.clone()).unwrap();
                        recall.validate_for_frame(&frame).unwrap();
                        black_box(finalized_memory_attention_evidence(&recall).unwrap());
                    }
                };
                let phase_after = || {
                    for (draft, prepared, _, _) in &cases {
                        black_box(prepared.attention_evidence_for_draft(draft).unwrap());
                    }
                };
                let chain_before = || {
                    for (draft, _, bank, topology) in &cases {
                        black_box(full_cpu_preparation(draft, bank, topology, &slot, false));
                    }
                };
                let chain_after = || {
                    for (draft, _, bank, topology) in &cases {
                        black_box(full_cpu_preparation(draft, bank, topology, &slot, true));
                    }
                };
                for (draft, prepared, bank, topology) in &cases {
                    let (_, finalized) = prepared.clone().finalize(draft.clone()).unwrap();
                    assert_eq!(
                        prepared.attention_evidence_for_draft(draft).unwrap(),
                        finalized_memory_attention_evidence(&finalized).unwrap()
                    );
                    let before = full_cpu_preparation(draft, bank, topology, &slot, false);
                    let after = full_cpu_preparation(draft, bank, topology, &slot, true);
                    assert_eq!(before, after);
                    let payload_words = |upload: &GpuMemoryContextUpload| {
                        upload
                            .header
                            .words()
                            .iter()
                            .copied()
                            .chain(
                                upload
                                    .records
                                    .iter()
                                    .flat_map(|row| row.words().iter().copied()),
                            )
                            .chain(
                                upload
                                    .cognitive_records
                                    .iter()
                                    .flat_map(|row| row.words().iter().copied()),
                            )
                            .collect::<Vec<_>>()
                    };
                    assert_eq!(payload_words(&before.6), payload_words(&after.6));
                }
                let (phase_before_ns, phase_after_ns) =
                    paired_samples(&phase_before, &phase_after, after_first);
                let (chain_before_ns, chain_after_ns) =
                    paired_samples(&chain_before, &chain_after, after_first);
                let (draft, _, bank, topology) = &cases[0];
                let (evidence, summaries, attention, frame, finalized, perception, upload) =
                    full_cpu_preparation(draft, bank, topology, &slot, true);
                assert!(!upload.cognitive_records.is_empty());
                let words = upload
                    .header
                    .words()
                    .iter()
                    .copied()
                    .chain(
                        upload
                            .records
                            .iter()
                            .flat_map(|row| row.words().iter().copied()),
                    )
                    .chain(
                        upload
                            .cognitive_records
                            .iter()
                            .flat_map(|row| row.words().iter().copied()),
                    )
                    .collect::<Vec<_>>();
                let proof = serde_json::to_vec(&(
                    draft,
                    &evidence,
                    &summaries,
                    &attention,
                    &frame,
                    finalized.context(),
                    finalized.candidate_keys(),
                    finalized.receipt(),
                    finalized.cognitive_context(),
                    &perception.dispatch_header_words,
                    &perception.frame_payload_words,
                    &words,
                    upload.neural_receptor_effects,
                    bank,
                    topology,
                ))
                .unwrap();
                println!(
                    "{}",
                    serde_json::json!({
                        "candidates": candidates, "requested_records": records, "retained_records": bank.len(),
                        "synthetic_instances": residents, "after_first": after_first,
                        "phase_before_ns": phase_before_ns, "phase_after_ns": phase_after_ns,
                        "chain_before_ns": chain_before_ns, "chain_after_ns": chain_after_ns,
                        "full_result_equal_all_instances": true, "proof_scope": "representative_identical_instance",
                        "proof_bytes": proof.len(), "proof_blake3": blake3::hash(&proof).to_string(),
                        "perception_header_words": perception.dispatch_header_words,
                        "perception_payload_words": perception.frame_payload_words, "memory_payload_words": words,
                    })
                );
            }
        }
    }
}
