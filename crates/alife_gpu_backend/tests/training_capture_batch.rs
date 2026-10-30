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
    let phenotypes = (1..=50)
        .map(|seed| {
            support::n512_phenotype_at_maturation(seed, if seed % 2 == 0 { 1.0 } else { 0.35 })
        })
        .collect::<Vec<_>>();
    assert!(phenotypes.iter().any(|p| p.microstep_count() % 2 == 0));
    assert!(phenotypes.iter().any(|p| p.microstep_count() % 2 == 1));
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
        assert!(backend.prepare_training_tick_capture(&invalid).is_err());
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
        let requests = handles
            .iter()
            .map(|handle| (*handle, tick))
            .collect::<Vec<_>>();
        if round == 1 {
            let stale = backend.prepare_training_tick_capture(&requests).unwrap();
            backend
                .force_activity_sequence_cursor_for_test(handles[0], 99)
                .unwrap();
            assert!(backend
                .tick_memory_batch_training_captured(&batch, &sampling, stale)
                .is_err());
            assert_eq!(backend.completed_dispatch_count(), completed_before);
            backend
                .force_activity_sequence_cursor_for_test(handles[0], round + 1)
                .unwrap();
        }
        let boundary = if round == 0 {
            // Force ONLY the combined arena below budget. Standalone captures
            // retain their real ceiling and old commit-before-post ordering.
            backend
                .prepare_training_tick_capture_with_limit_for_test(&requests, 4)
                .unwrap()
        } else {
            backend.prepare_training_tick_capture(&requests).unwrap()
        };
        let captured = backend
            .tick_memory_batch_training_captured(&batch, &sampling, boundary)
            .unwrap();
        assert_eq!(captured.before, before);
        let outcomes = captured.ticks;
        let (after, after_activity) = capture_boundary(&mut backend, &handles, tick);
        assert_eq!(captured.after, after);
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
    assert_eq!(backend.training_capture_metrics().batch_captures, 10);
    assert_eq!(backend.training_capture_metrics().fused_tick_packets, 1);
    assert_eq!(
        backend.training_capture_metrics().staging_fallback_packets,
        1
    );
    assert_eq!(backend.training_capture_metrics().captured_rows, 500);
}

#[test]
#[ignore = "requires GPU hardware"]
fn fused_capture_failures_preserve_host_activity_and_refuse_selection() {
    for fault in 0..5 {
        let mut backend =
            GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
        let phenotype = support::n512_phenotype(17);
        let handle = backend
            .insert_brain(OrganismId(1), phenotype.clone())
            .unwrap();
        let source = support::perception_frame_for_profile_at_tick(
            1,
            42,
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let (frame, recall) = support::try_empty_recall(&source).unwrap();
        let upload = backend
            .prepare_memory_context_upload(handle, &frame, &recall)
            .unwrap();
        let physiology = support::test_physiology(1, &phenotype).unwrap();
        let upload = support::bind_chemistry_receptor_effects(
            upload,
            &phenotype,
            &physiology,
            Tick::new(42),
        )
        .unwrap();
        let batch =
            GpuClosedLoopMemoryBatchInput::try_new(vec![GpuClosedLoopMemoryTickInput::try_new(
                handle, &frame, &upload,
            )
            .unwrap()])
            .unwrap();
        let boundary = backend
            .prepare_training_tick_capture(&[(handle, Tick::new(42))])
            .unwrap();
        let before_activity = backend.snapshot_activity_state(handle).unwrap();
        let before_v11 = backend.checkpoint_v11(handle).unwrap();
        match fault {
            0 => backend.force_training_capture_failure_for_test(true),
            1 => backend.force_training_capture_failure_for_test(false),
            2 => backend.force_all_invalid_after_next_decode_for_test(handle),
            3 => backend.force_pending_identity_mismatch_after_next_decode_for_test(handle),
            _ => backend.force_device_lost_after_next_submit_for_test(),
        }
        let sampling = [GpuTrainingSamplingConfig {
            seed: 19,
            counter: 0,
            temperature: 1.0,
            demonstrator: None,
        }];
        let result = backend.tick_memory_batch_training_captured(&batch, &sampling, boundary);
        assert!(result.is_err(), "fault {fault}");
        if fault < 2 {
            assert_eq!(
                result.err(),
                Some(alife_core::ScaffoldContractError::NeuralBackendUnavailable)
            );
        }
        assert!(matches!(
            backend.state(),
            alife_gpu_backend::GpuBackendState::DeviceLost { .. }
        ));
        assert_eq!(backend.completed_dispatch_count(), 0);
        assert_eq!(
            backend.snapshot_activity_state(handle).unwrap(),
            before_activity
        );
        assert_eq!(backend.checkpoint_v11(handle).unwrap(), before_v11);
        // The public selection/eligibility accessor must refuse a poisoned
        // backend; it cannot inspect private host pending state after failure.
        assert_eq!(
            backend.pending_eligibility(handle),
            Err(alife_core::ScaffoldContractError::NeuralBackendUnavailable)
        );
        assert_eq!(backend.training_capture_metrics().fused_tick_packets, 0);
    }
}

#[test]
#[ignore = "requires GPU hardware"]
fn structural_growth_admits_synchronously_then_exact_cache_hits_fuse() {
    use alife_core::{CoactivationEvidence, StructuralPlasticityConfig, StructuralPlasticityState};
    let mut backend =
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let phenotype = support::controlled_sensory_n512_phenotype();
    let handle = backend
        .insert_brain(OrganismId(1), phenotype.clone())
        .unwrap();
    let route = phenotype
        .projections()
        .iter()
        .map(|p| p.route_index())
        .find(|route| *route > 0 && *route < 16)
        .unwrap();
    let target = phenotype.candidate_decoder().motor_start();
    let mut checkpoint = backend.checkpoint_v11(handle).unwrap();
    checkpoint.structural = StructuralPlasticityState::new(
        phenotype.neuron_count(),
        StructuralPlasticityConfig {
            max_candidates_per_region: 8,
            max_regions: route + 1,
            max_accepted_per_phase: 1,
            max_structural_edges: 2,
            min_candidate_score: 2,
            prune_score_below: 0,
        },
    )
    .unwrap();
    backend.restore_v11(handle, checkpoint).unwrap();
    for round in 0..3 {
        if round != 1 {
            let work = backend
                .apply_v11_structural_phase(
                    handle,
                    &[CoactivationEvidence {
                        region: route,
                        source: target + if round == 0 { 1 } else { 2 },
                        target,
                        coactivation: 100,
                        eligibility: 0,
                        concept_gap_support: 0,
                    }],
                )
                .unwrap();
            assert_eq!(work.structural.accepted_edges, 1);
        }
        let tick = Tick::new(42 + round);
        let source = support::perception_frame_for_profile_at_tick(
            1,
            tick.raw(),
            SensorProfile::PrivilegedAffordanceV1,
            true,
            2,
        );
        let (frame, recall) = support::try_empty_recall(&source).unwrap();
        let upload = backend
            .prepare_memory_context_upload(handle, &frame, &recall)
            .unwrap();
        let physiology = support::test_physiology(1, &phenotype).unwrap();
        let upload =
            support::bind_chemistry_receptor_effects(upload, &phenotype, &physiology, tick)
                .unwrap();
        let batch =
            GpuClosedLoopMemoryBatchInput::try_new(vec![GpuClosedLoopMemoryTickInput::try_new(
                handle, &frame, &upload,
            )
            .unwrap()])
            .unwrap();
        let before_modes = backend.training_capture_metrics();
        let boundary = backend
            .prepare_training_tick_capture(&[(handle, tick)])
            .unwrap();
        let sampling = [GpuTrainingSamplingConfig {
            seed: 19,
            counter: round as u32,
            temperature: 1.0,
            demonstrator: None,
        }];
        let captured = backend
            .tick_memory_batch_training_captured(&batch, &sampling, boundary)
            .unwrap();
        let modes = backend.training_capture_metrics();
        assert_eq!(
            modes.fused_tick_packets - before_modes.fused_tick_packets,
            u64::from(round == 1)
        );
        assert_eq!(
            modes.structural_fallback_packets - before_modes.structural_fallback_packets,
            u64::from(round != 1)
        );
        assert_eq!(
            captured.before[0].structural_synapses.len(),
            if round == 2 { 2 } else { 1 }
        );
        assert_eq!(
            captured.after,
            backend.capture_training_states(&[(handle, tick)]).unwrap()
        );
        let outcome = &captured.ticks[0];
        backend
            .discard_pending_eligibility(handle, outcome.pending_eligibility.identity())
            .unwrap();
    }
}
