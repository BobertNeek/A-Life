//! Controlled-channel capture evidence for real TerrainVision/N2048 CPU inputs.
//! These channels model reply readiness at the owned-hint capture boundary.
//! They do not instantiate or certify the LocalSLM provider/cache pipeline.
use super::*;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};

struct ControlledHintInbox {
    sender: Option<Sender<alife_core::SemanticContextRef>>,
    receiver: Receiver<alife_core::SemanticContextRef>,
    polls: usize,
    disconnected_at_capture: bool,
}

fn hint(code: u32) -> alife_core::SemanticContextRef {
    alife_core::SemanticContextRef {
        feature_flags: alife_core::ContextFeatureFlags(0),
        confidence: Confidence(0.1),
        compressed_codes: vec![alife_core::CompressedSemanticCode {
            codebook_id: 1,
            code,
            salience: NormalizedScalar(1.0),
        }],
        salience: Vec::new(),
    }
}

fn poll_owned_hint_once(fixture: &mut OwnerFixture, inbox: &mut ControlledHintInbox) {
    assert_eq!(
        inbox.polls, 0,
        "each owner is polled only at serial capture"
    );
    inbox.polls += 1;
    let captured = match inbox.receiver.try_recv() {
        Ok(reply) => Some(reply),
        Err(TryRecvError::Empty) => None,
        Err(TryRecvError::Disconnected) => {
            // Capture absence and record the controlled channel status. This
            // is not evidence about actual prior failure metrics or retries.
            inbox.disconnected_at_capture = true;
            None
        }
    };
    fixture.input.draft = fixture
        .input
        .draft
        .clone()
        .with_semantic_context(captured)
        .unwrap();
}

fn capture_controlled_cohort() -> (Vec<OwnerFixture>, Vec<ControlledHintInbox>) {
    let owners = [3, 11, 28, 37, 52, 86, 111, 205];
    let mut fixtures: Vec<_> = owners
        .iter()
        .enumerate()
        .map(|(index, owner)| OwnerFixture::terrain(*owner, index % 2 == 0))
        .collect();
    let mut inboxes: Vec<_> = owners
        .iter()
        .map(|_| {
            let (sender, receiver) = mpsc::channel();
            ControlledHintInbox {
                sender: Some(sender),
                receiver,
                polls: 0,
                disconnected_at_capture: false,
            }
        })
        .collect();
    // One owner has a closed, empty inbox before its first capture.
    drop(inboxes[7].sender.take());
    // Replies already ready before any owner is captured.
    for (index, code) in [(0, 11), (2, 31), (6, 71)] {
        inboxes[index]
            .sender
            .as_ref()
            .unwrap()
            .send(hint(code))
            .unwrap();
    }
    let mut capture_order = Vec::new();
    for (index, fixture) in fixtures.iter_mut().enumerate() {
        assert_eq!(
            fixture.input.draft.sensor_profile(),
            SensorProfile::GroundedTerrainVisionV1
        );
        assert_eq!(
            fixture.memory.profile(),
            fixture.input.draft.profile_provenance().identity()
        );
        poll_owned_hint_once(fixture, &mut inboxes[index]);
        capture_order.push(fixture.input.draft.organism_id().raw());
        match index {
            0 => {
                // A replacement for this owner arrives after its capture and
                // stays queued. A later owner can receive a ready reply now.
                inboxes[0].sender.as_ref().unwrap().send(hint(12)).unwrap();
                inboxes[1].sender.as_ref().unwrap().send(hint(21)).unwrap();
            }
            1 => inboxes[1].sender.as_ref().unwrap().send(hint(22)).unwrap(),
            2 => inboxes[5].sender.as_ref().unwrap().send(hint(61)).unwrap(),
            3 => {
                // This owner was captured with an empty inbox. The late reply
                // cannot fill in its current frame, even on spawn fallback.
                assert!(fixture.input.draft.sensory().semantic_context.is_none());
                inboxes[3].sender.as_ref().unwrap().send(hint(42)).unwrap();
            }
            _ => {}
        }
    }
    assert_eq!(capture_order, owners);
    let expected = [
        Some(11),
        Some(21),
        Some(31),
        None,
        None,
        Some(61),
        Some(71),
        None,
    ];
    for (fixture, code) in fixtures.iter().zip(expected) {
        assert_eq!(
            fixture.input.draft.sensory().semantic_context,
            code.map(hint)
        );
    }
    assert_polled_once(&inboxes);
    for (index, inbox) in inboxes.iter().enumerate() {
        assert_eq!(inbox.disconnected_at_capture, index == 7);
    }
    (fixtures, inboxes)
}

fn assert_polled_once(inboxes: &[ControlledHintInbox]) {
    assert!(inboxes.iter().all(|inbox| inbox.polls == 1));
}

fn assert_late_replies_still_queued(inboxes: &[ControlledHintInbox]) {
    for (index, inbox) in inboxes.iter().enumerate() {
        let expected = match index {
            0 => Some(12),
            1 => Some(22),
            3 => Some(42),
            _ => None,
        };
        match expected {
            Some(code) => assert_eq!(inbox.receiver.try_recv().unwrap(), hint(code)),
            None => assert_eq!(
                inbox.receiver.try_recv(),
                Err(if inbox.disconnected_at_capture {
                    TryRecvError::Disconnected
                } else {
                    TryRecvError::Empty
                })
            ),
        }
        assert_eq!(
            inbox.receiver.try_recv(),
            Err(if inbox.disconnected_at_capture {
                TryRecvError::Disconnected
            } else {
                TryRecvError::Empty
            })
        );
    }
    assert_polled_once(inboxes);
}

fn assert_captured_hint_in_output(job: &CpuPreparationJob<'_>, row: &CpuPreparationOutcome) {
    let prepared = row.result.as_ref().unwrap();
    assert_eq!(
        prepared.frame.sensor_profile(),
        SensorProfile::GroundedTerrainVisionV1
    );
    assert_eq!(prepared.frame.organism_id(), job.input.draft.organism_id());
    assert_eq!(prepared.frame.tick(), job.input.draft.tick());
    assert_eq!(
        prepared.frame.sensory().semantic_context,
        job.input.draft.sensory().semantic_context
    );
    assert_eq!(
        prepared.frame.sensory().language_prior_neural_lanes(),
        job.input.draft.sensory().language_prior_neural_lanes()
    );
    let projection = prepared
        .memory_recall
        .cognitive_context()
        .unwrap()
        .cognitive_projection
        .as_ref()
        .unwrap();
    let learned = job.predictor.unwrap().has_acquired_state();
    assert!(!projection.candidates.is_empty());
    assert!(projection
        .candidates
        .iter()
        .all(|candidate| candidate.forecast_available == learned));
    if learned {
        assert!(projection.candidates.iter().any(|candidate| {
            candidate
                .prediction
                .predicted_successor
                .iter()
                .any(|value| *value > 0.0)
        }));
    }
}

#[test]
fn terrain_ready_absent_and_later_owner_hints_match_frozen_serial_and_two_workers() {
    let (fixtures, inboxes) = capture_controlled_cohort();
    let before: Vec<_> = fixtures.iter().map(OwnerFixture::snapshot).collect();
    let slots = terrain_slots(fixtures.len());
    let jobs: Vec<_> = fixtures
        .iter()
        .zip(&slots)
        .map(|(fixture, slot)| fixture.job(slot))
        .collect();
    let serial = prepare_cpu_rows(&jobs, 1, false).unwrap();
    let faults = visits(jobs.len());
    let parallel = prepare_cpu_rows_inner(&jobs, 2, false, &faults).unwrap();
    assert_once(&faults);
    for ((job, expected), actual) in jobs.iter().zip(&serial).zip(&parallel) {
        assert_same_trace(expected, &legacy_serial_reference(job));
        assert_same_trace(actual, expected);
        assert_captured_hint_in_output(job, actual);
    }
    assert_polled_once(&inboxes);
    assert_eq!(
        fixtures
            .iter()
            .map(OwnerFixture::snapshot)
            .collect::<Vec<_>>(),
        before
    );
    // Reorder noncontiguous owners across both worker halves after capture.
    // Each hint, candidate binding, predictor and slot stays with its owner.
    let reversed: Vec<_> = jobs
        .iter()
        .rev()
        .map(|job| CpuPreparationJob {
            input: job.input,
            slot: job.slot,
            memory: job.memory,
            topology: job.topology,
            predictor: job.predictor,
        })
        .collect();
    let reversed_rows = prepare_cpu_rows(&reversed, 2, false).unwrap();
    for ((job, actual), expected) in reversed.iter().zip(&reversed_rows).zip(serial.iter().rev()) {
        assert_same_trace(actual, expected);
        assert_captured_hint_in_output(job, actual);
    }
    assert_eq!(
        fixtures
            .iter()
            .map(OwnerFixture::snapshot)
            .collect::<Vec<_>>(),
        before
    );
    assert_late_replies_still_queued(&inboxes);
}

#[test]
fn terrain_spawn_failure_reuses_each_owned_hint_once_without_repolling() {
    let (fixtures, inboxes) = capture_controlled_cohort();
    let before: Vec<_> = fixtures.iter().map(OwnerFixture::snapshot).collect();
    let slots = terrain_slots(fixtures.len());
    let jobs: Vec<_> = fixtures
        .iter()
        .zip(&slots)
        .map(|(fixture, slot)| fixture.job(slot))
        .collect();
    let mut faults = visits(jobs.len());
    faults.spawn_failure = true;
    let fallback = prepare_cpu_rows_inner(&jobs, 2, false, &faults).unwrap();
    assert_once(&faults);
    for (job, row) in jobs.iter().zip(&fallback) {
        assert_same_trace(row, &legacy_serial_reference(job));
        assert_captured_hint_in_output(job, row);
    }
    assert_eq!(
        fixtures
            .iter()
            .map(OwnerFixture::snapshot)
            .collect::<Vec<_>>(),
        before
    );
    assert_late_replies_still_queued(&inboxes);
}

#[test]
fn terrain_caller_and_child_panics_do_not_mutate_inputs_repoll_or_continue() {
    for panic_row in [0, 4] {
        let (fixtures, inboxes) = capture_controlled_cohort();
        let before: Vec<_> = fixtures.iter().map(OwnerFixture::snapshot).collect();
        let slots = terrain_slots(fixtures.len());
        let jobs: Vec<_> = fixtures
            .iter()
            .zip(&slots)
            .map(|(fixture, slot)| fixture.job(slot))
            .collect();
        let mut faults = visits(jobs.len());
        faults.panic_row = Some(panic_row);
        let continuation_visits = AtomicUsize::new(0);
        let result = prepare_cpu_rows_inner(&jobs, 2, false, &faults).map(|rows| {
            // An explicit CPU test continuation boundary, not a device or
            // learning-commit counter. Real staged world rollback is covered
            // by preparation_panics_roll_back_the_real_staged_tick_wrapper.
            continuation_visits.fetch_add(1, Ordering::Relaxed);
            rows.len()
        });
        assert_eq!(result, Err(ScaffoldContractError::InvalidDecisionEvidence));
        assert_eq!(continuation_visits.load(Ordering::Relaxed), 0);
        assert_eq!(faults.visits[panic_row].load(Ordering::Relaxed), 1);
        let healthy_half = if panic_row == 0 { 4..8 } else { 0..4 };
        for index in healthy_half {
            assert_eq!(faults.visits[index].load(Ordering::Relaxed), 1);
        }
        assert!(faults
            .visits
            .iter()
            .all(|count| count.load(Ordering::Relaxed) <= 1));
        assert_eq!(
            fixtures
                .iter()
                .map(OwnerFixture::snapshot)
                .collect::<Vec<_>>(),
            before
        );
        assert_late_replies_still_queued(&inboxes);
    }
}
