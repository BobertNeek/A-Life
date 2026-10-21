//! Real-GPU batch-versus-single state-capture checks.
#![cfg(all(feature = "training-rollout", feature = "gpu-tests"))]
mod support;
use alife_core::{OrganismId, SensorProfile, Tick};
use alife_gpu_backend::training_rollout::GpuTrainingStateSnapshot;
use alife_gpu_backend::{
    GpuActivityRuntimeSnapshot, GpuBrainHandle, GpuClosedLoopBackend,
    GpuClosedLoopMemoryBatchInput, GpuClosedLoopMemoryTickInput, GpuRuntimeProfile,
    GpuTrainingSamplingConfig,
};

fn capture_boundary(
    backend: &mut GpuClosedLoopBackend,
    handles: &[GpuBrainHandle],
    tick: Tick,
) -> (
    Vec<GpuTrainingStateSnapshot>,
    Vec<GpuActivityRuntimeSnapshot>,
) {
    let requests = handles.iter().map(|h| (*h, tick)).collect::<Vec<_>>();
    let before_maps = backend.mutable_slot_readback_metrics();
    let before_capture = backend.training_capture_metrics();
    let batch = backend.capture_training_states(&requests).unwrap();
    let batch_maps = backend.mutable_slot_readback_metrics().calls - before_maps.calls;
    assert!(batch_maps > 0 && batch_maps < handles.len() as u64);
    assert_eq!(
        backend.training_capture_metrics().readback_submissions
            - before_capture.readback_submissions,
        batch_maps
    );
    let singles = requests
        .iter()
        .map(|(h, t)| backend.capture_training_state(*h, *t).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(batch, singles);
    let activity = handles
        .iter()
        .map(|h| backend.snapshot_activity_state(*h).unwrap())
        .collect();
    (batch, activity)
}

#[test]
#[ignore = "requires GPU hardware"]
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
    let (mut before, mut before_activity) = capture_boundary(&mut backend, &handles, tick);
    assert_eq!(backend.completed_dispatch_count(), 0);
    for ((state, activity), handle) in before.iter().zip(&before_activity).zip(&handles) {
        assert_eq!(state.handle, *handle);
        assert_eq!(state.logical_dispatch_generation, 1);
        assert_eq!(activity.next_sequence_cursor, 1);
        assert!(activity.pressure.is_none());
        assert!(activity.throttle.is_none());
        assert!(activity.work.is_none());
        assert!(backend.pending_eligibility(*handle).unwrap().is_none());
    }
    assert_eq!(backend.training_capture_metrics().captured_rows, 50);
    assert!(backend.training_capture_metrics().readback_submissions > 0);

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
    for round in 0..2 {
        let tick = Tick::new(42 + round);
        if round > 0 {
            // Discarding pending eligibility changes mutable state. The next
            // replay boundary must be captured after that explicit operation.
            (before, before_activity) = capture_boundary(&mut backend, &handles, tick);
        }
        let retained_before = before
            .iter()
            .map(|r| r.mutable_words.clone())
            .collect::<Vec<_>>();
        let mut frames = Vec::new();
        let mut uploads = Vec::new();
        for (i, (handle, phenotype)) in handles.iter().zip(&phenotypes).enumerate() {
            let frame = support::perception_frame_for_profile_at_tick(
                i as u64 + 1,
                tick.raw(),
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
                support::bind_chemistry_receptor_effects(upload, phenotype, &physiology, tick)
                    .unwrap(),
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
                counter: round as u32 * 50 + i,
                temperature: 1.0,
                demonstrator: None,
            })
            .collect::<Vec<_>>();
        let completed_before = backend.completed_dispatch_count();
        assert_eq!(completed_before, round);
        let outcomes = backend
            .tick_memory_batch_training(&batch, &sampling)
            .unwrap();
        let (after, after_activity) = capture_boundary(&mut backend, &handles, tick);
        let completed_after = backend.completed_dispatch_count();
        assert_eq!(completed_after, completed_before + 1);
        assert_eq!(outcomes.len(), 50);
        for (i, ((pre, post), outcome)) in before.iter().zip(&after).zip(&outcomes).enumerate() {
            let old_activity = &before_activity[i];
            let activity = &after_activity[i];
            let behavior = outcome.training_rollout.as_ref().unwrap();
            let pending = outcome.pending_eligibility.identity();
            let diagnostic = format!(
                "round={round} organism={} tick={} logical={}->{} receipt={} cursor={}->{} completed={completed_before}->{completed_after}",
                i + 1, tick.raw(), pre.logical_dispatch_generation,
                post.logical_dispatch_generation, behavior.dispatch_generation,
                old_activity.next_sequence_cursor, activity.next_sequence_cursor,
            );
            eprintln!("{diagnostic}");
            assert_eq!(pre.handle, handles[i], "{diagnostic}");
            assert_eq!(post.handle, handles[i], "{diagnostic}");
            assert_eq!(pre.tick, tick.raw(), "{diagnostic}");
            assert_eq!(post.tick, tick.raw(), "{diagnostic}");
            assert_eq!(pre.logical_dispatch_generation, 1, "{diagnostic}");
            // Every newly admitted resident has the next nonzero dispatch
            // label already. Its first commit retains that label; subsequent
            // commits advance it. Cursors and receipts prove actual work.
            assert_eq!(post.logical_dispatch_generation, round + 1, "{diagnostic}");
            if round == 0 {
                assert_eq!(
                    post.logical_dispatch_generation, pre.logical_dispatch_generation,
                    "{diagnostic}"
                );
            } else {
                assert_eq!(
                    post.logical_dispatch_generation,
                    pre.logical_dispatch_generation + 1,
                    "{diagnostic}"
                );
            }
            assert_eq!(old_activity.next_sequence_cursor, round + 1, "{diagnostic}");
            assert_eq!(activity.next_sequence_cursor, round + 2, "{diagnostic}");
            assert_eq!(
                pre.active_weight_generation, post.active_weight_generation,
                "{diagnostic}"
            );
            assert_eq!(
                pre.active_weight_bank, post.active_weight_bank,
                "{diagnostic}"
            );
            assert_eq!(pre.mutable_words, retained_before[i], "{diagnostic}");
            assert_eq!(outcome.handle, handles[i], "{diagnostic}");
            assert_eq!(
                outcome.dispatch_generation, post.logical_dispatch_generation,
                "{diagnostic}"
            );
            assert_eq!(
                outcome.frame_digest,
                frames[i].frame_digest(),
                "{diagnostic}"
            );
            assert_eq!(
                outcome.active_activation_side, post.active_activation_side,
                "{diagnostic}"
            );
            assert_eq!(behavior.organism_id, i as u64 + 1, "{diagnostic}");
            assert_eq!(behavior.tick, tick.raw(), "{diagnostic}");
            assert_eq!(
                behavior.dispatch_generation, outcome.dispatch_generation,
                "{diagnostic}"
            );
            assert_eq!(behavior.frame_digest, outcome.frame_digest, "{diagnostic}");
            assert_eq!(
                behavior.active_weight_generation, post.active_weight_generation,
                "{diagnostic}"
            );
            assert_eq!(behavior.sampling, sampling[i], "{diagnostic}");
            assert!(
                behavior.on_policy_log_probability().unwrap().is_finite(),
                "{diagnostic}"
            );
            assert_eq!(
                pending.handle_generation(),
                handles[i].generation(),
                "{diagnostic}"
            );
            assert_eq!(
                pending.phenotype_hash(),
                handles[i].phenotype_hash(),
                "{diagnostic}"
            );
            assert_eq!(
                pending.dispatch_generation(),
                outcome.dispatch_generation,
                "{diagnostic}"
            );
            assert_eq!(pending.originating_tick(), tick, "{diagnostic}");
            assert_eq!(pending.frame_digest(), outcome.frame_digest, "{diagnostic}");
            assert_eq!(
                pending.active_activation_side(),
                post.active_activation_side,
                "{diagnostic}"
            );
            assert_eq!(
                backend.pending_eligibility(handles[i]).unwrap(),
                Some(outcome.pending_eligibility),
                "{diagnostic}"
            );
            assert_eq!(activity.pressure, Some(outcome.pressure), "{diagnostic}");
            assert_eq!(
                activity.throttle.as_ref(),
                Some(&outcome.throttle),
                "{diagnostic}"
            );
            assert_eq!(activity.work.as_ref(), Some(&outcome.work), "{diagnostic}");
            for (owner, slot, generation, receipt_tick, dispatch, digest, cursor) in [
                (
                    outcome.pressure.organism_id_raw,
                    outcome.pressure.handle_slot,
                    outcome.pressure.handle_generation,
                    outcome.pressure.tick,
                    outcome.pressure.dispatch_generation,
                    outcome.pressure.frame_digest,
                    outcome.pressure.sequence_cursor,
                ),
                (
                    outcome.throttle.organism_id_raw,
                    outcome.throttle.handle_slot,
                    outcome.throttle.handle_generation,
                    outcome.throttle.tick,
                    outcome.throttle.dispatch_generation,
                    outcome.throttle.frame_digest,
                    outcome.throttle.sequence_cursor,
                ),
                (
                    outcome.work.organism_id_raw,
                    outcome.work.handle_slot,
                    outcome.work.handle_generation,
                    outcome.work.tick,
                    outcome.work.dispatch_generation,
                    outcome.work.frame_digest,
                    outcome.work.sequence_cursor,
                ),
            ] {
                assert_eq!(owner, i as u64 + 1, "{diagnostic}");
                assert_eq!(slot, handles[i].slot(), "{diagnostic}");
                assert_eq!(generation, handles[i].generation(), "{diagnostic}");
                assert_eq!(receipt_tick, tick.raw(), "{diagnostic}");
                assert_eq!(dispatch, post.logical_dispatch_generation, "{diagnostic}");
                assert_eq!(digest, outcome.frame_digest.0, "{diagnostic}");
                assert_eq!(cursor, old_activity.next_sequence_cursor, "{diagnostic}");
            }
            backend
                .discard_pending_eligibility(handles[i], pending)
                .unwrap();
            assert!(
                backend.pending_eligibility(handles[i]).unwrap().is_none(),
                "{diagnostic}"
            );
            assert_eq!(
                backend.snapshot_activity_state(handles[i]).unwrap(),
                *activity,
                "{diagnostic}"
            );
            assert_eq!(
                backend.completed_dispatch_count(),
                completed_after,
                "{diagnostic}"
            );
        }
        // Verify scalar generation neutrality with one batch readback, rather
        // than mapping fifty private buffers again just for their host labels.
        let requests = handles.iter().map(|h| (*h, tick)).collect::<Vec<_>>();
        let discarded = backend.capture_training_states(&requests).unwrap();
        for (post, discarded) in after.iter().zip(&discarded) {
            assert_eq!(post.handle, discarded.handle);
            assert_eq!(
                post.logical_dispatch_generation,
                discarded.logical_dispatch_generation,
                "round={round} organism={:?} discard changed logical label {}->{}",
                post.handle.organism_id(),
                post.logical_dispatch_generation,
                discarded.logical_dispatch_generation
            );
        }
        assert!(before
            .iter()
            .zip(&after)
            .any(|(a, b)| a.mutable_words != b.mutable_words));
    }
    assert_eq!(backend.training_capture_metrics().batch_captures, 6);
    assert_eq!(backend.training_capture_metrics().captured_rows, 300);
}
