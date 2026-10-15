//! CPU integration at the existing worker/translation/world-perception boundary.
//!
//! The translation callback and runtime identity are explicitly synthetic. Tests
//! use the real worker start/poll/join lifecycle, bounded unaided translator and
//! world speech APIs. They do not instantiate the GPU-only Bevy handler or its
//! completion system, render UI, call an LLM, or demonstrate comprehension.
use super::*;
use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

use alife_world::{HeadlessScenarioBuilder, StableVoxelObjectRef, VoxelChunkCoord, VoxelTileCoord};
use bevy::{
    ecs::message::{MessageCursor, Messages},
    input::keyboard::Key,
    prelude::Entity,
};

const WAIT_LIMIT: Duration = Duration::from_secs(5);

/// Releases on assertion unwinding too, so the real worker Drop cannot hang.
struct SyntheticCompletionGate(Option<Sender<()>>);

impl SyntheticCompletionGate {
    fn release(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

impl Drop for SyntheticCompletionGate {
    fn drop(&mut self) {
        self.release();
    }
}

enum SyntheticResult {
    Unaided,
    Failure,
    Panic,
}

fn install_synthetic_translation(
    worker: &mut ProductionSpeechTranslationWorker,
    result: SyntheticResult,
) -> (SyntheticCompletionGate, Receiver<SpeechTranslationRequest>) {
    let (release, wait) = mpsc::channel();
    let (observed, snapshot) = mpsc::channel();
    worker.synthetic_translation = Some(Box::new(move |request| {
        observed.send(request.clone()).unwrap();
        wait.recv_timeout(WAIT_LIMIT)
            .map_err(|error| format!("synthetic completion gate failed: {error}"))?;
        match result {
            SyntheticResult::Unaided => translate_speech_unaided(request)
                .map(|receipt| (receipt, None))
                .map_err(|error| format!("synthetic unaided fixture failed: {error}")),
            SyntheticResult::Failure => Err("synthetic translation failure".to_string()),
            SyntheticResult::Panic => panic!("synthetic translation panic"),
        }
    }));
    (SyntheticCompletionGate(Some(release)), snapshot)
}

fn synthetic_context(
    save_id: &str,
    addressee: Option<OrganismId>,
    source_position: Vec3f,
) -> SpeechTranslationJobContext {
    // This is a context-ownership fixture, not proof of real runtime authority.
    SpeechTranslationJobContext::Player {
        addressee,
        source_position,
        runtime_identity: SpeechRuntimeIdentity {
            authority: crate::gpu_live_runtime::LiveRuntimeSaveAuthorityView {
                save_id: save_id.to_string(),
                deterministic_seed: 20_261_013,
                sensor_profile: SensorProfile::PrivilegedAffordanceV1,
                organism_ids: vec![OrganismId(1), OrganismId(2), OrganismId(3)],
            },
            started_at_tick: Tick::new(0),
        },
    }
}

fn request(text: &str, addressee: Option<OrganismId>) -> SpeechTranslationRequest {
    SpeechTranslationRequest::try_new(
        UtteranceId::new(1).unwrap(),
        addressee,
        SpeechTranslationInput::PlayerText {
            text: text.to_string(),
        },
        alife_core::basic_vocabulary_bindings(),
    )
    .unwrap()
}

fn wait_for_completion(
    worker: &mut ProductionSpeechTranslationWorker,
) -> SpeechTranslationJobCompletion {
    let deadline = Instant::now() + WAIT_LIMIT;
    loop {
        if let Some(completion) = worker.poll() {
            return completion;
        }
        assert!(Instant::now() < deadline, "synthetic worker did not finish");
        thread::sleep(Duration::from_millis(1));
    }
}

fn key(key_code: KeyCode, text: Option<&str>) -> KeyboardInput {
    KeyboardInput {
        key_code,
        logical_key: if key_code == KeyCode::Enter {
            Key::Enter
        } else {
            Key::Character(text.unwrap_or_default().into())
        },
        state: ButtonState::Pressed,
        text: text.map(Into::into),
        repeat: false,
        window: Entity::PLACEHOLDER,
    }
}

fn submitted_text(text: &str) -> String {
    // Actual raw message ordering boundary, followed by a distinct submit press.
    let mut messages = Messages::default();
    let mut cursor = MessageCursor::default();
    messages.write_batch([
        key(KeyCode::KeyC, Some("preopening shortcut")),
        key(KeyCode::Enter, Some("\r")),
        key(KeyCode::KeyC, Some(text)),
    ]);
    let mut input = String::new();
    assert!(!append_player_speech_key_text(
        &mut input,
        cursor.read(&messages),
        true,
    ));
    messages.update();
    messages.write(key(KeyCode::Enter, Some("\r")));
    assert!(append_player_speech_key_text(
        &mut input,
        cursor.read(&messages),
        false,
    ));
    assert_eq!(input, text);
    assert!(cursor.is_empty(&messages));
    input
}

fn world_fixture() -> HeadlessWorld {
    HeadlessScenarioBuilder::new(20_261_013)
        .agent("near", OrganismId(1), Vec3f::new(1.5, 0.5, 0.0))
        .agent("bystander", OrganismId(2), Vec3f::new(2.5, 0.5, 0.0))
        .agent("far", OrganismId(3), Vec3f::new(20.5, 0.5, 0.0))
        .build()
        .unwrap()
}

fn heard(world: &HeadlessWorld, organism: OrganismId) -> Vec<alife_core::HeardToken> {
    world
        .sensory_report(organism, world.tick())
        .unwrap()
        .core_snapshot
        .language_context
        .heard_tokens
        .into_iter()
        .flatten()
        .collect()
}

fn assert_spatial_player_hearing(text: &str, addressed: Option<OrganismId>) {
    let mut world = world_fixture();
    let selection = Fvr03ProductionVoxelSelectionResource {
        hovered: None,
        selected: Some(StableVoxelObjectRef {
            kind: StableVoxelRefKind::Tile,
            stable_id: None,
            chunk: VoxelChunkCoord::new(0, 0),
            tile: Some(VoxelTileCoord::new(0, 0)),
        }),
    };
    let source = player_speech_position(&selection, &world).unwrap();
    assert_eq!(source, Vec3f::new(0.5, 0.5, 0.0));
    let input = submitted_text(text);
    let addressee = named_addressee(&input, &world);
    assert_eq!(addressee, addressed);
    let request = request(&input, addressee);
    let mut worker = ProductionSpeechTranslationWorker::default();
    let (mut gate, observed) = install_synthetic_translation(&mut worker, SyntheticResult::Unaided);
    assert!(worker
        .start(
            &request,
            synthetic_context("synthetic-hearing", addressee, source)
        )
        .unwrap());
    assert_eq!(observed.recv_timeout(WAIT_LIMIT).unwrap(), request);
    assert!(worker.poll().is_none());
    assert!(world.audible_utterances().is_empty());
    assert!(heard(&world, OrganismId(1)).is_empty());
    gate.release();
    let completion = wait_for_completion(&mut worker);
    let (receipt, warning) = completion.result.unwrap();
    assert_eq!(
        receipt.literal_text,
        translate_speech_unaided(&request).unwrap().literal_text
    );
    assert!(!receipt.assisted);
    assert_eq!(receipt.model_identity, "alife-bounded-unaided-v1");
    assert!(warning.is_none());
    receipt.validate_contract().unwrap();
    let SpeechTranslationJobContext::Player {
        addressee,
        source_position,
        ..
    } = completion.context
    else {
        panic!("synthetic player context changed")
    };
    // Existing world emission API; this is not the GPU Bevy completion system.
    let audible = world
        .emit_player_tokens(addressee, source_position, receipt.literal_tokens.clone())
        .unwrap();
    let expected = receipt
        .literal_tokens
        .iter()
        .map(|token| u32::from(token.raw()))
        .collect::<Vec<_>>();
    for organism in [OrganismId(1), OrganismId(2), OrganismId(3)] {
        let hearing = heard(&world, organism);
        let should_hear =
            organism != OrganismId(3) && addressee.is_none_or(|target| target == organism);
        if should_hear {
            assert_eq!(
                hearing
                    .iter()
                    .map(|token| token.token_id)
                    .collect::<Vec<_>>(),
                expected
            );
            assert!(hearing.iter().all(|token| {
                token.utterance_id == audible.utterance_id
                    && token.source_kind == UtteranceSourceKind::Player
                    && token.speaker_id.is_none()
                    && token.addressee == addressee
                    && token.source_position == source
                    && token.teacher_channel.is_none()
            }));
        } else {
            assert!(hearing.is_empty());
        }
    }
    assert!(worker.active_key().is_none());
    assert!(
        worker.poll().is_none(),
        "completion is consumed exactly once"
    );
}

#[test]
fn submitted_text_crosses_real_worker_and_bounded_translation_into_spatial_broadcast() {
    assert_spatial_player_hearing("hello food", None);
}

#[test]
fn submitted_named_text_respects_addressee_and_cannot_bypass_hearing_range() {
    assert_spatial_player_hearing("near, hello food", Some(OrganismId(1)));
    assert_spatial_player_hearing("far, hello food", Some(OrganismId(3)));
}

#[test]
fn pending_worker_rejects_newer_submission_without_replacing_the_owned_request() {
    let mut worker = ProductionSpeechTranslationWorker::default();
    let mut old = request("hello food", Some(OrganismId(1)));
    let snapshot = old.clone();
    let old_context = synthetic_context("synthetic-old-save", old.addressee, Vec3f::ZERO);
    let (mut gate, observed) = install_synthetic_translation(&mut worker, SyntheticResult::Unaided);
    assert!(worker.start(&old, old_context.clone()).unwrap());
    assert_eq!(observed.recv_timeout(WAIT_LIMIT).unwrap(), snapshot);
    old.input = SpeechTranslationInput::PlayerText {
        text: "changed after submit".to_string(),
    };
    old.known_bindings.clear();
    let newer = request("drink water", Some(OrganismId(2)));
    assert!(!worker
        .start(
            &newer,
            synthetic_context(
                "synthetic-new-save",
                newer.addressee,
                Vec3f::new(9.0, 0.0, 0.0)
            ),
        )
        .unwrap());
    assert_eq!(worker.active_key(), Some(SpeechTranslationJobKey::Player));
    assert!(worker.poll().is_none(), "pending is not completion");
    gate.release();
    let completion = wait_for_completion(&mut worker);
    assert_eq!(
        completion.context.runtime_identity(),
        old_context.runtime_identity()
    );
    let (receipt, _) = completion.result.unwrap();
    assert_eq!(
        receipt.literal_text,
        translate_speech_unaided(&snapshot).unwrap().literal_text
    );
    assert_eq!(receipt.addressee, Some(OrganismId(1)));
    assert_eq!(
        receipt.literal_tokens,
        translate_speech_unaided(&snapshot).unwrap().literal_tokens
    );
    assert!(worker.active_key().is_none());
    assert!(worker.poll().is_none());
}

#[test]
fn explicit_translation_failure_is_an_error_completion_and_releases_the_worker() {
    let mut worker = ProductionSpeechTranslationWorker::default();
    let request = request("hello", None);
    let context = synthetic_context("synthetic-failure", None, Vec3f::ZERO);
    let (mut gate, observed) = install_synthetic_translation(&mut worker, SyntheticResult::Failure);
    assert!(worker.start(&request, context.clone()).unwrap());
    assert_eq!(observed.recv_timeout(WAIT_LIMIT).unwrap(), request);
    assert!(worker.poll().is_none());
    gate.release();
    let completion = wait_for_completion(&mut worker);
    assert_eq!(
        completion.context.runtime_identity(),
        context.runtime_identity()
    );
    assert_eq!(
        completion.result.unwrap_err(),
        "synthetic translation failure"
    );
    assert!(worker.active_key().is_none());
    assert!(worker.poll().is_none());
}

#[test]
fn worker_panic_is_reported_as_failure_without_losing_its_context() {
    let mut worker = ProductionSpeechTranslationWorker::default();
    let request = request("hello", None);
    let context = synthetic_context("synthetic-panic", None, Vec3f::ZERO);
    let (mut gate, observed) = install_synthetic_translation(&mut worker, SyntheticResult::Panic);
    assert!(worker.start(&request, context.clone()).unwrap());
    observed.recv_timeout(WAIT_LIMIT).unwrap();
    gate.release();
    let completion = wait_for_completion(&mut worker);
    assert_eq!(
        completion.context.runtime_identity(),
        context.runtime_identity()
    );
    assert_eq!(
        completion.result.unwrap_err(),
        "local SLM translation worker panicked"
    );
    assert!(worker.active_key().is_none());
    assert!(worker.poll().is_none());
}

#[test]
fn a_failed_job_cannot_reappear_or_corrupt_a_later_successful_job() {
    let mut worker = ProductionSpeechTranslationWorker::default();
    let old = request("food", Some(OrganismId(1)));
    let (mut old_gate, old_observed) =
        install_synthetic_translation(&mut worker, SyntheticResult::Failure);
    assert!(worker
        .start(
            &old,
            synthetic_context("synthetic-old", old.addressee, Vec3f::ZERO)
        )
        .unwrap());
    old_observed.recv_timeout(WAIT_LIMIT).unwrap();
    old_gate.release();
    assert!(wait_for_completion(&mut worker).result.is_err());
    let newer = request("water", Some(OrganismId(2)));
    let newer_context =
        synthetic_context("synthetic-new", newer.addressee, Vec3f::new(2.0, 0.0, 0.0));
    let (mut newer_gate, newer_observed) =
        install_synthetic_translation(&mut worker, SyntheticResult::Unaided);
    assert!(worker.start(&newer, newer_context.clone()).unwrap());
    assert_eq!(newer_observed.recv_timeout(WAIT_LIMIT).unwrap(), newer);
    assert!(worker.poll().is_none());
    newer_gate.release();
    let completion = wait_for_completion(&mut worker);
    assert_eq!(
        completion.context.runtime_identity(),
        newer_context.runtime_identity()
    );
    let (receipt, warning) = completion.result.unwrap();
    assert_eq!(
        receipt.literal_text,
        translate_speech_unaided(&newer).unwrap().literal_text
    );
    assert_eq!(receipt.addressee, Some(OrganismId(2)));
    assert!(warning.is_none());
    assert!(worker.poll().is_none());
}
