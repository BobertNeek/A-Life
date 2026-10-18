//! Production composer state-helper tests only; no GPU completion handler fixture.
use super::*;

pub(super) fn state() -> ProductionConversationLineageUiState {
    ProductionConversationLineageUiState {
        input_open: false,
        input_capture_held: false,
        input_help_open: false,
        input_cue: SpeechInputCue::None,
        input: String::new(),
        address_selected: true,
        muted: false,
        narration: NarrationDisplayFrequency::Normal,
        rendered_translation_visible: true,
        raw_tokens_visible: false,
        slm_off: false,
        developer_overlay: false,
        bindings: alife_core::basic_vocabulary_bindings(),
        last_player_receipt: None,
        last_creature_receipt: None,
        last_creature_utterance_id: None,
        last_creature_speaker: None,
        creature_utterance_active: false,
        status: String::new(),
        lineage_root: PathBuf::new(),
        lineage_open: false,
        lineage_source_filter: LineageSourceFilter::All,
        lineage_data_filter: LineageDataFilter::All,
        lineage_sort: LineageSort::Overall,
        lineage_loaded: false,
        lineage_rows: Vec::new(),
        lineage_cursor: 0,
        pending_founder_mode: FounderMode::GeneticFounder,
        cohort: Vec::new(),
        habitat_focus_id: None,
        habitat_partner_id: None,
    }
}

fn accepted_state() -> ProductionConversationLineageUiState {
    let mut state = state();
    state.prepare_typed_speech_fixture("the submitted draft");
    state.accept_player_speech_translation();
    assert!(state.input.is_empty());
    assert!(!state.input_open);
    assert_eq!(state.input_cue, SpeechInputCue::Pending);
    state
}

#[test]
fn inline_completion_clears_and_closes_its_submitted_draft() {
    let mut state = state();
    state.prepare_typed_speech_fixture("the submitted draft");
    state.input_cue = SpeechInputCue::Failed;
    let bindings = state.bindings.clone();
    let status = state.status.clone();
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Inline);
    assert!(state.input.is_empty());
    assert!(!state.input_open);
    assert_eq!(state.input_cue, SpeechInputCue::None);
    assert!(state.address_selected);
    assert_eq!(state.bindings, bindings);
    assert_eq!(state.status, status);
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Inline);
    assert!(state.input.is_empty());
    assert!(!state.input_open);
}

#[test]
fn worker_completion_preserves_a_reopened_newer_unicode_draft() {
    let mut state = accepted_state();
    state.open_player_speech_input();
    let newer_draft = "é猫".repeat(MAX_TYPED_CHARS / 2);
    state.input = newer_draft.clone();
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    assert_eq!(state.input, newer_draft);
    assert!(state.input_open);
    assert_eq!(state.input_cue, SpeechInputCue::None);
    assert!(state.address_selected);
}

#[test]
fn worker_completion_keeps_an_untouched_accepted_composer_closed() {
    let mut state = accepted_state();
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    assert!(state.input.is_empty());
    assert!(!state.input_open);
    assert_eq!(state.input_cue, SpeechInputCue::None);
}

#[test]
fn worker_completion_clears_a_retained_pending_cue_without_closing_open_input() {
    let mut state = accepted_state();
    // Recorded capture opens current input without resetting the worker's cue.
    state.prepare_recorded_speech_capture();
    assert!(state.input_open);
    assert_eq!(state.input_cue, SpeechInputCue::Pending);
    let draft = state.input.clone();
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    assert_eq!(state.input, draft);
    assert!(state.input_open);
    assert_eq!(state.input_cue, SpeechInputCue::None);
}

#[test]
fn worker_completion_preserves_a_busy_rejection_cue_on_a_newer_open_draft() {
    let mut state = accepted_state();
    state.prepare_typed_speech_fixture("a newer draft rejected while the worker was busy");
    // The production busy branch keeps this draft open and marks it Failed.
    state.input_cue = SpeechInputCue::Failed;
    let draft = state.input.clone();
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    assert_eq!(state.input, draft);
    assert!(state.input_open);
    assert_eq!(state.input_cue, SpeechInputCue::Failed);
}

#[test]
fn worker_completion_preserves_current_text_even_when_the_composer_is_closed() {
    let mut state = accepted_state();
    state.prepare_typed_speech_fixture("a closed newer draft");
    state.clear_recorded_capture();
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    assert_eq!(state.input, "a closed newer draft");
    assert!(!state.input_open);
}

#[test]
fn repeated_worker_state_completion_never_consumes_later_composer_input() {
    let mut state = accepted_state();
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    state.prepare_typed_speech_fixture("first newer draft");
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    state.input.push_str(" and more");
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    assert_eq!(state.input, "first newer draft and more");
    assert!(state.input_open);
    state.input.clear();
    state.complete_player_speech_input(PlayerSpeechCompletionOrigin::Worker);
    assert!(state.input.is_empty());
    assert!(state.input_open, "an empty reopened composer stays open");
}
