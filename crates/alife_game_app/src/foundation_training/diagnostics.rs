//! Offline measurements; none of these values is fed back into the policy.
use alife_core::{OrganismId, Tick, Validate};
use alife_training::ReplaySpeechTarget;
use alife_world::AudibleUtterance;

use super::FoundationTeacherLesson;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct FoundationSpeechBehaviorDiagnostics {
    pub vocalize_available_rows: usize,
    pub vocalize_selected_rows: usize,
    /// Positive payload label AND Vocalize in the sampled legal support.
    pub vocalize_opportunities: usize,
    pub selected_vocalize_opportunities: usize,
    pub valid_utterance_rows: usize,
    pub valid_utterance_opportunities: usize,
    pub correct_utterance_opportunities: usize,
    pub selected_vocalize_rate: Option<f64>,
    pub valid_utterance_rate: Option<f64>,
    pub correct_utterance_rate: Option<f64>,
}

pub(crate) fn vocalize_action_evidence(
    forced_slots: &[u8],
    representative: u16,
    motors: &[u16; 6],
    support: u32,
) -> (bool, bool) {
    let is_vocal = |index: u16| forced_slots.get(usize::from(index)) == Some(&3);
    let selected = is_vocal(representative) || is_vocal(motors[3]);
    let available = forced_slots.iter().enumerate().any(|(index, slot)| {
        *slot == 3 && support & 1_u32.checked_shl(index as u32).unwrap_or(0) != 0
    });
    (available, selected)
}

pub(crate) fn speech_output_evidence(
    utterances: &[AudibleUtterance],
    speaker: OrganismId,
    tick: Tick,
    target: Option<ReplaySpeechTarget>,
) -> (bool, bool) {
    let expected = target.and_then(|label| {
        label.token.map(|first| {
            let mut tokens = vec![first];
            tokens.extend(
                label
                    .continuation
                    .into_iter()
                    .take_while(|token| *token != 0),
            );
            tokens
        })
    });
    let mut valid = false;
    let mut correct = false;
    for utterance in utterances.iter().filter(|utterance| {
        utterance.source_kind == alife_core::UtteranceSourceKind::Creature
            && utterance.speaker_id == Some(speaker)
            && utterance.emitted_tick == tick
    }) {
        if utterance.validate_contract().is_ok() && !utterance.tokens.is_empty() {
            valid = true;
            correct |= expected.as_ref().is_some_and(|tokens| {
                utterance
                    .tokens
                    .iter()
                    .map(|token| token.raw())
                    .eq(tokens.iter().copied())
            });
        }
    }
    (valid, correct)
}

impl FoundationSpeechBehaviorDiagnostics {
    pub(crate) fn observe(
        &mut self,
        available: bool,
        selected: bool,
        positive_label: bool,
        valid: bool,
        correct: bool,
    ) {
        self.vocalize_available_rows += usize::from(available);
        self.vocalize_selected_rows += usize::from(selected);
        self.valid_utterance_rows += usize::from(valid);
        if positive_label && available {
            self.vocalize_opportunities += 1;
            self.selected_vocalize_opportunities += usize::from(selected);
            self.valid_utterance_opportunities += usize::from(valid);
            self.correct_utterance_opportunities += usize::from(correct);
        }
        let rate = |count| {
            (self.vocalize_opportunities != 0)
                .then(|| count as f64 / self.vocalize_opportunities as f64)
        };
        self.selected_vocalize_rate = rate(self.selected_vocalize_opportunities);
        self.valid_utterance_rate = rate(self.valid_utterance_opportunities);
        self.correct_utterance_rate = rate(self.correct_utterance_opportunities);
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FoundationLessonBudget {
    pub lesson: FoundationTeacherLesson,
    pub demonstrations: usize,
    pub records: usize,
    pub loss_windows: usize,
    pub burn_in_replay_rows: usize,
    /// Sum of the unchanged per-window episode weights, not a new weight.
    pub episode_loss_budget: f64,
    pub positive_payload_label_rows: usize,
    pub silence_payload_label_rows: usize,
    pub unlabeled_payload_rows: usize,
    pub positive_payload_token_steps: usize,
    pub selected_vocalize_rows: usize,
}

impl FoundationLessonBudget {
    pub(crate) fn new(lesson: FoundationTeacherLesson) -> Self {
        Self {
            lesson,
            demonstrations: 0,
            records: 0,
            loss_windows: 0,
            burn_in_replay_rows: 0,
            episode_loss_budget: 0.0,
            positive_payload_label_rows: 0,
            silence_payload_label_rows: 0,
            unlabeled_payload_rows: 0,
            positive_payload_token_steps: 0,
            selected_vocalize_rows: 0,
        }
    }

    pub(crate) fn observe_labels(&mut self, targets: &[Option<ReplaySpeechTarget>]) {
        self.demonstrations += 1;
        self.records += targets.len();
        for target in targets {
            match target {
                Some(label) if label.token.is_some() => {
                    self.positive_payload_label_rows += 1;
                    self.positive_payload_token_steps += label.len();
                }
                Some(_) => self.silence_payload_label_rows += 1,
                None => self.unlabeled_payload_rows += 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_pilot_receipts_preserve_legacy_counts_without_new_diagnostics() {
        let receipt: super::super::FoundationPilotReceipt =
            serde_json::from_value(serde_json::json!({
                "seed": 17, "founder_seed_base": 7, "ticks": 20,
                "collection_seconds": 1.0, "replay_seconds": 1.0,
                "maximum_logit_error": 0.0, "maximum_activation_error": 0.0,
                "maximum_activity_ema_error": 0.0, "maximum_metabolic_error": 0.0,
                "source_asset_digest": "source", "teacher_mode": false,
                "consumed_events": 0, "demonstration_replay_records": 20,
                "speech_opportunities": 5, "correct_utterances": 2
            }))
            .unwrap();
        assert_eq!(receipt.speech_opportunities, 5);
        assert_eq!(receipt.correct_utterances, 2);
        assert!(receipt.speech_behavior.is_none());
    }

    #[test]
    fn cooldown_labels_do_not_inflate_legal_speech_opportunities() {
        let mut metrics = FoundationSpeechBehaviorDiagnostics::default();
        for _ in 0..15 {
            metrics.observe(false, false, true, false, false);
        }
        assert_eq!(metrics.correct_utterance_rate, None);
        metrics.observe(true, true, true, true, true);
        metrics.observe(true, false, true, false, false);
        metrics.observe(true, true, false, true, false);
        assert_eq!(metrics.vocalize_opportunities, 2);
        assert_eq!(metrics.vocalize_selected_rows, 2);
        assert_eq!(metrics.valid_utterance_rows, 2);
        assert_eq!(metrics.valid_utterance_rate, Some(0.5));
        assert_eq!(metrics.correct_utterance_rate, Some(0.5));
    }

    #[test]
    fn vocalize_is_counted_once_for_representative_and_motor_selection() {
        let mut motors = [u16::MAX; 6];
        motors[3] = 1;
        assert_eq!(
            vocalize_action_evidence(&[4, 3, 0], 1, &motors, 2),
            (true, true)
        );
        assert_eq!(
            vocalize_action_evidence(&[4, 3, 0], 0, &[u16::MAX; 6], 1),
            (false, false)
        );
    }

    #[test]
    fn utterance_metrics_require_valid_fresh_learner_output_and_exact_tokens() {
        use alife_core::{
            Confidence, LanguageTokenId, SpeechActKind, SpeechMotorPayload, UtteranceId, Vec3f,
        };
        let speaker = OrganismId(1);
        let tick = Tick::new(20);
        let utterance = AudibleUtterance::from_creature(
            UtteranceId::new(1).unwrap(),
            speaker,
            None,
            Vec3f::ZERO,
            SpeechMotorPayload::try_new(
                SpeechActKind::Declare,
                vec![LanguageTokenId::new(11).unwrap()],
                Confidence::new(1.0).unwrap(),
            )
            .unwrap(),
            tick,
        )
        .unwrap();
        let label = ReplaySpeechTarget {
            token: Some(11),
            continuation: [0; 5],
            act: SpeechActKind::Declare,
            weight: 1.0,
        };
        assert_eq!(
            speech_output_evidence(std::slice::from_ref(&utterance), speaker, tick, Some(label)),
            (true, true)
        );
        assert_eq!(
            speech_output_evidence(
                std::slice::from_ref(&utterance),
                OrganismId(2),
                tick,
                Some(label)
            ),
            (false, false)
        );
        assert_eq!(
            speech_output_evidence(
                std::slice::from_ref(&utterance),
                speaker,
                Tick::new(21),
                Some(label)
            ),
            (false, false)
        );
        let mut teacher = utterance.clone();
        teacher.source_kind = alife_core::UtteranceSourceKind::Teacher;
        assert_eq!(
            speech_output_evidence(&[teacher], speaker, tick, Some(label)),
            (false, false)
        );
        let wrong = ReplaySpeechTarget {
            token: Some(12),
            ..label
        };
        assert_eq!(
            speech_output_evidence(std::slice::from_ref(&utterance), speaker, tick, Some(wrong)),
            (true, false)
        );
        let mut invalid = utterance;
        invalid.tokens.clear();
        assert_eq!(
            speech_output_evidence(&[invalid], speaker, tick, Some(label)),
            (false, false)
        );
    }

    #[test]
    fn corpus_budget_separates_labels_tokens_and_record_counts() {
        let label = ReplaySpeechTarget {
            token: Some(11),
            continuation: [1, 0, 0, 0, 0],
            act: alife_core::SpeechActKind::Declare,
            weight: 1.0,
        };
        let mut budget = FoundationLessonBudget::new(FoundationTeacherLesson::VocabularyProduction);
        budget.observe_labels(&[
            Some(label),
            Some(ReplaySpeechTarget {
                token: None,
                continuation: [0; 5],
                ..label
            }),
            None,
        ]);
        assert_eq!(budget.records, 3);
        assert_eq!(budget.positive_payload_label_rows, 1);
        assert_eq!(budget.positive_payload_token_steps, 2);
        assert_eq!(budget.silence_payload_label_rows, 1);
        assert_eq!(budget.unlabeled_payload_rows, 1);
    }
}
