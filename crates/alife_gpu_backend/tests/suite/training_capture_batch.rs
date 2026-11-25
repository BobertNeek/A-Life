//! Real-GPU batch-versus-single state-capture checks.
#[allow(unused_imports)]
use crate::support;
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

fn assert_handles_match_modulo_instance(left: GpuBrainHandle, right: GpuBrainHandle) {
    assert_eq!(left.class_id(), right.class_id());
    assert_eq!(left.slot(), right.slot());
    assert_eq!(left.generation(), right.generation());
    assert_eq!(left.organism_id(), right.organism_id());
    assert_eq!(left.phenotype_hash(), right.phenotype_hash());
}

fn assert_captures_match_modulo_instance(
    left: &[GpuTrainingStateSnapshot],
    right: &[GpuTrainingStateSnapshot],
) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_handles_match_modulo_instance(a.handle, b.handle);
        assert_eq!(a.brain_slot.record(), b.brain_slot.record());
        assert_eq!(a.brain_slot.identity(), b.brain_slot.identity());
        assert_eq!(a.brain_slot.typed_counts(), b.brain_slot.typed_counts());
        assert_eq!(a.brain_slot.word_ranges(), b.brain_slot.word_ranges());
        assert_eq!(
            a.brain_slot.decoder_input_stride(),
            b.brain_slot.decoder_input_stride()
        );
        assert_eq!(
            a.brain_slot.brain_slot_index(),
            b.brain_slot.brain_slot_index()
        );
        let mut a_norm = a.clone();
        a_norm.handle = b.handle;
        a_norm.brain_slot = b.brain_slot.clone();
        assert_eq!(a_norm, *b);
    }
}

fn assert_ticks_match_modulo_instance_and_timing(
    left: &[alife_gpu_backend::GpuClosedLoopTick],
    right: &[alife_gpu_backend::GpuClosedLoopTick],
) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_handles_match_modulo_instance(a.handle, b.handle);
        assert_ne!(a.hardware_receipt_generation, 0);
        assert_ne!(b.hardware_receipt_generation, 0);
        // Compare all deterministic pressure/throttle fields while allowing wall-clock
        // GPU timestamp jitter across separate backend instances.
        let mut a_pressure = a.pressure;
        a_pressure.completed_gpu_time_ns = b.pressure.completed_gpu_time_ns;
        a_pressure.sample_digest = b.pressure.sample_digest;
        assert_eq!(a_pressure, b.pressure);
        let mut a_norm = a.clone();
        a_norm.handle = b.handle;
        a_norm.hardware_receipt_generation = b.hardware_receipt_generation;
        a_norm.pressure = b.pressure;
        a_norm.throttle.pressure = b.throttle.pressure;
        a_norm.throttle.pressure_digest = b.throttle.pressure_digest;
        a_norm.throttle.decision_digest = b.throttle.decision_digest;
        assert_eq!(a_norm, *b);
    }
}

#[test]
#[ignore = "requires GPU hardware"]
fn tick_memory_batch_training_with_captures_matches_capture_training_states() {
    let mut fused = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let mut reference =
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let phenotypes = (1..=6).map(support::n512_phenotype).collect::<Vec<_>>();
    let fused_handles = phenotypes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            fused
                .insert_brain(OrganismId(i as u64 + 1), p.clone())
                .unwrap()
        })
        .collect::<Vec<_>>();
    let ref_handles = phenotypes
        .iter()
        .enumerate()
        .map(|(i, p)| {
            reference
                .insert_brain(OrganismId(i as u64 + 1), p.clone())
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(fused_handles.len(), ref_handles.len());
    for (fused_handle, ref_handle) in fused_handles.iter().zip(&ref_handles) {
        assert_handles_match_modulo_instance(*fused_handle, *ref_handle);
    }

    for round in 0..2 {
        let tick = Tick::new(42 + round);
        let fused_requests = fused_handles.iter().map(|h| (*h, tick)).collect::<Vec<_>>();
        let ref_requests = ref_handles.iter().map(|h| (*h, tick)).collect::<Vec<_>>();
        let pre_explicit = fused.capture_training_states(&fused_requests).unwrap();
        let ref_pre = reference.capture_training_states(&ref_requests).unwrap();
        assert_captures_match_modulo_instance(&pre_explicit, &ref_pre);

        let mut frames = Vec::new();
        let mut fused_uploads = Vec::new();
        let mut ref_uploads = Vec::new();
        for (i, phenotype) in phenotypes.iter().enumerate() {
            let frame = support::perception_frame_for_profile_at_tick(
                i as u64 + 1,
                tick.raw(),
                SensorProfile::PrivilegedAffordanceV1,
                i % 2 == 0,
                2,
            );
            let (frame, recall) = support::try_empty_recall(&frame).unwrap();
            let f_up = fused
                .prepare_memory_context_upload(fused_handles[i], &frame, &recall)
                .unwrap();
            let r_up = reference
                .prepare_memory_context_upload(ref_handles[i], &frame, &recall)
                .unwrap();
            let physiology = support::test_physiology(i as u64 + 1, phenotype).unwrap();
            fused_uploads.push(
                support::bind_chemistry_receptor_effects(f_up, phenotype, &physiology, tick)
                    .unwrap(),
            );
            ref_uploads.push(
                support::bind_chemistry_receptor_effects(r_up, phenotype, &physiology, tick)
                    .unwrap(),
            );
            frames.push(frame);
        }
        let fused_inputs = fused_handles
            .iter()
            .zip(&frames)
            .zip(&fused_uploads)
            .map(|((h, f), u)| GpuClosedLoopMemoryTickInput::try_new(*h, f, u).unwrap())
            .collect();
        let ref_inputs = ref_handles
            .iter()
            .zip(&frames)
            .zip(&ref_uploads)
            .map(|((h, f), u)| GpuClosedLoopMemoryTickInput::try_new(*h, f, u).unwrap())
            .collect();
        let fused_batch = GpuClosedLoopMemoryBatchInput::try_new(fused_inputs).unwrap();
        let ref_batch = GpuClosedLoopMemoryBatchInput::try_new(ref_inputs).unwrap();
        let sampling = (0..6)
            .map(|i| GpuTrainingSamplingConfig {
                seed: 29,
                counter: round as u32 * 6 + i,
                temperature: 1.0,
                demonstrator: None,
            })
            .collect::<Vec<_>>();

        let (fused_ticks, fused_pre, fused_post) = fused
            .tick_memory_batch_training_with_captures(&fused_batch, &sampling)
            .unwrap();
        let ref_ticks = reference
            .tick_memory_batch_training(&ref_batch, &sampling)
            .unwrap();
        let post_explicit = fused.capture_training_states(&fused_requests).unwrap();
        let ref_post = reference.capture_training_states(&ref_requests).unwrap();

        assert_ticks_match_modulo_instance_and_timing(&fused_ticks, &ref_ticks);
        assert_eq!(fused_pre, pre_explicit);
        assert_eq!(fused_post, post_explicit);
        assert_captures_match_modulo_instance(&fused_post, &ref_post);
        assert!(fused_pre
            .iter()
            .zip(&fused_post)
            .any(|(a, b)| a.mutable_words != b.mutable_words));

        let fused_discards = fused_handles
            .iter()
            .zip(&fused_ticks)
            .map(|(h, t)| (*h, t.pending_eligibility.identity()))
            .collect::<Vec<_>>();
        let ref_discards = ref_handles
            .iter()
            .zip(&ref_ticks)
            .map(|(h, t)| (*h, t.pending_eligibility.identity()))
            .collect::<Vec<_>>();
        let f_rec = fused
            .discard_pending_eligibility_batch(&fused_discards)
            .unwrap();
        let r_rec = reference
            .discard_pending_eligibility_batch(&ref_discards)
            .unwrap();
        assert_eq!(f_rec.len(), r_rec.len());
        for (a, b) in f_rec.iter().zip(&r_rec) {
            assert_eq!(a.identity, b.identity);
            assert_eq!(
                a.discarded_staging_generation,
                b.discarded_staging_generation
            );
            assert_ne!(a.hardware_receipt_generation, 0);
            assert_ne!(b.hardware_receipt_generation, 0);
            assert_ne!(a.receipt_digest, [0; 4]);
            assert_ne!(b.receipt_digest, [0; 4]);
        }
    }
}
