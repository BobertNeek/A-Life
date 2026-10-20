//! Explicit real-GPU differential qualification. Compile-only in CPU clouds.
#![cfg(all(feature = "training-rollout", feature = "gpu-tests"))]
mod support;
use alife_core::{OrganismId, SensorProfile, Tick};
use alife_gpu_backend::{
    GpuClosedLoopBackend, GpuClosedLoopMemoryBatchInput, GpuClosedLoopMemoryTickInput,
    GpuRuntimeProfile, GpuTrainingSamplingConfig,
};

#[test]
#[ignore = "requires an explicitly coordinated GPU; no fallback or cloud emulation"]
fn fifty_row_pre_post_capture_matches_single_and_preserves_private_order() {
    let mut backend =
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let tick = Tick::new(42);
    let phenotypes = (1..=50).map(support::n512_phenotype).collect::<Vec<_>>();
    let handles = phenotypes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            backend
                .insert_brain(OrganismId(i as u64 + 1), p.clone())
                .unwrap()
        })
        .collect::<Vec<_>>();
    let requests = handles.iter().map(|h| (*h, tick)).collect::<Vec<_>>();
    let singles = requests
        .iter()
        .map(|(h, t)| backend.capture_training_state(*h, *t).unwrap())
        .collect::<Vec<_>>();
    let before_maps = backend.mutable_slot_readback_metrics();
    let before = backend.capture_training_states(&requests).unwrap();
    let batch_maps = backend.mutable_slot_readback_metrics().calls - before_maps.calls;
    assert!(batch_maps > 0 && batch_maps < 50);
    assert_eq!(before, singles);
    let retained_before = before
        .iter()
        .map(|r| r.mutable_words.clone())
        .collect::<Vec<_>>();
    assert_eq!(backend.training_capture_metrics().captured_rows, 50);
    assert_eq!(
        backend.training_capture_metrics().readback_submissions,
        batch_maps
    );

    // The complete capability inventory is checked before even a valid prefix
    // can submit, including duplicate owners, foreign handles and mixed labels.
    let mut foreign = backend.new_staging_like_live().unwrap();
    let foreign_handle = foreign
        .insert_brain(OrganismId(999), phenotypes[0].clone())
        .unwrap();
    let stale = backend
        .insert_brain(OrganismId(1000), phenotypes[0].clone())
        .unwrap();
    backend.remove_brain(stale).unwrap();
    for invalid in [
        vec![requests[0], requests[0]],
        vec![requests[0], (foreign_handle, tick)],
        vec![requests[0], (stale, tick)],
        vec![requests[0], (handles[1], Tick::new(43))],
    ] {
        let maps = backend.mutable_slot_readback_metrics().calls;
        assert!(backend.capture_training_states(&invalid).is_err());
        assert_eq!(backend.mutable_slot_readback_metrics().calls, maps);
    }
    let mut frames = Vec::new();
    let mut uploads = Vec::new();
    for (i, (handle, phenotype)) in handles.iter().zip(&phenotypes).enumerate() {
        let frame = support::perception_frame_for_profile_at_tick(
            i as u64 + 1,
            42,
            SensorProfile::PrivilegedAffordanceV1,
            i % 2 == 0,
            2,
        );
        let (frame, recall) = support::try_empty_recall(&frame).unwrap();
        let upload = backend
            .prepare_memory_context_upload(*handle, &frame, &recall)
            .unwrap();
        let physiology = support::test_physiology(i as u64 + 1, phenotype).unwrap();
        uploads.push(
            support::bind_chemistry_receptor_effects(upload, phenotype, &physiology, tick).unwrap(),
        );
        frames.push(frame);
    }
    let inputs = handles
        .iter()
        .zip(&frames)
        .zip(&uploads)
        .map(|((h, f), u)| GpuClosedLoopMemoryTickInput::try_new(*h, f, u).unwrap())
        .collect();
    let batch = GpuClosedLoopMemoryBatchInput::try_new(inputs).unwrap();
    let sampling = (0..50)
        .map(|i| GpuTrainingSamplingConfig {
            seed: 19,
            counter: i,
            temperature: 1.0,
            demonstrator: None,
        })
        .collect::<Vec<_>>();
    let outcomes = backend
        .tick_memory_batch_training(&batch, &sampling)
        .unwrap();
    let after = backend.capture_training_states(&requests).unwrap();
    let singles = requests
        .iter()
        .map(|(h, t)| backend.capture_training_state(*h, *t).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(after, singles);
    assert_eq!(outcomes.len(), 50);
    for (i, ((pre, post), outcome)) in before.iter().zip(&after).zip(&outcomes).enumerate() {
        assert_eq!(post.handle, handles[i]);
        assert!(post.logical_dispatch_generation > pre.logical_dispatch_generation);
        assert_eq!(pre.active_weight_generation, post.active_weight_generation);
        assert_eq!(pre.mutable_words, retained_before[i]);
        let behavior = outcome.training_rollout.as_ref().unwrap();
        assert_eq!(behavior.organism_id, i as u64 + 1);
        assert_eq!(behavior.tick, 42);
        assert!(behavior.on_policy_log_probability().unwrap().is_finite());
    }
    assert!(before
        .iter()
        .zip(&after)
        .any(|(a, b)| a.mutable_words != b.mutable_words));
    assert_eq!(backend.training_capture_metrics().batch_captures, 2);
    assert_eq!(backend.training_capture_metrics().captured_rows, 100);
}
