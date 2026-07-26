use std::collections::{BTreeMap, BTreeSet};

use alife_core::{
    Confidence, LanguageCodebookV1, LanguageTokenId, NovelLanguageToken, ScaffoldContractError,
    SpeechTranslationInput, SpeechTranslationReceipt, SpeechTranslationRequest, Validate,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationAssistance {
    Disabled,
    SlmAssisted,
}

#[derive(Debug, Clone)]
pub struct BoundedSpeechTranslator {
    model_identity: String,
    assistance: TranslationAssistance,
}

impl BoundedSpeechTranslator {
    pub fn new(
        model_identity: impl Into<String>,
        assistance: TranslationAssistance,
    ) -> Result<Self, ScaffoldContractError> {
        let model_identity = model_identity.into();
        if model_identity.trim().is_empty() || model_identity.chars().count() > 128 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self {
            model_identity,
            assistance,
        })
    }

    pub fn translate(
        &self,
        request: &SpeechTranslationRequest,
    ) -> Result<SpeechTranslationReceipt, ScaffoldContractError> {
        request.validate_contract()?;
        let bindings = request
            .known_bindings
            .iter()
            .map(|binding| (binding.surface.as_str(), binding.token))
            .collect::<BTreeMap<_, _>>();
        let codebook = LanguageCodebookV1::canonical();

        let (literal_tokens, novel_tokens, rendered_words, uncertain) = match &request.input {
            SpeechTranslationInput::PlayerText { text } => {
                let words = normalized_words(text);
                let mut used = request
                    .known_bindings
                    .iter()
                    .map(|binding| binding.token.raw())
                    .collect::<BTreeSet<_>>();
                let mut novel_assignments = BTreeMap::<String, LanguageTokenId>::new();
                let mut literal_tokens = Vec::with_capacity(words.len());
                let mut novel_tokens = Vec::new();
                for word in words
                    .into_iter()
                    .take(usize::from(LanguageCodebookV1::MAX_HEARD_TOKENS))
                {
                    if let Some(token) = bindings.get(word.as_str()).copied() {
                        literal_tokens.push(token);
                    } else {
                        let token = if let Some(token) = novel_assignments.get(&word).copied() {
                            token
                        } else {
                            let token = allocate_novel_token(&word, &mut used)?;
                            novel_assignments.insert(word.clone(), token);
                            novel_tokens.push(NovelLanguageToken {
                                token,
                                surface: word.clone(),
                            });
                            token
                        };
                        literal_tokens.push(token);
                    }
                }
                if literal_tokens.is_empty() {
                    return Err(ScaffoldContractError::InvalidPerceptionFrame);
                }
                let rendered_words = literal_tokens
                    .iter()
                    .copied()
                    .map(|token| codebook.pronounceable_symbol(token))
                    .collect::<Vec<_>>();
                let uncertain = !novel_tokens.is_empty();
                (literal_tokens, novel_tokens, rendered_words, uncertain)
            }
            SpeechTranslationInput::CreatureTokens { tokens } => {
                let by_token = request
                    .known_bindings
                    .iter()
                    .map(|binding| (binding.token, binding.surface.as_str()))
                    .collect::<BTreeMap<_, _>>();
                let mut uncertain = false;
                let rendered_words = tokens
                    .iter()
                    .copied()
                    .map(|token| {
                        by_token.get(&token).map_or_else(
                            || {
                                uncertain = true;
                                codebook.pronounceable_symbol(token)
                            },
                            |surface| (*surface).to_string(),
                        )
                    })
                    .collect::<Vec<_>>();
                (tokens.clone(), Vec::new(), rendered_words, uncertain)
            }
        };

        let known_tokens = request
            .known_bindings
            .iter()
            .map(|binding| binding.token)
            .collect::<BTreeSet<_>>();
        let known_count = literal_tokens
            .iter()
            .filter(|token| known_tokens.contains(token))
            .count();
        let confidence_value = if uncertain {
            (known_count as f32 / literal_tokens.len() as f32).max(0.25)
        } else {
            1.0
        };
        let rendered = rendered_words.join(" ");
        let receipt = SpeechTranslationReceipt {
            utterance_id: request.utterance_id,
            addressee: request.addressee,
            literal_text: literal_tokens
                .iter()
                .copied()
                .map(|token| codebook.pronounceable_symbol(token))
                .collect::<Vec<_>>()
                .join(" "),
            literal_tokens,
            novel_tokens,
            rendered_text: if uncertain {
                format!("[uncertain] {rendered}")
            } else {
                rendered
            },
            confidence: Confidence::new(confidence_value)?,
            model_identity: self.model_identity.clone(),
            assisted: self.assistance == TranslationAssistance::SlmAssisted,
            uncertain,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }
}

fn normalized_words(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric() && ch != '\'' && ch != '-')
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn allocate_novel_token(
    surface: &str,
    used: &mut BTreeSet<u16>,
) -> Result<LanguageTokenId, ScaffoldContractError> {
    let mut hash = 0x811c_9dc5_u32;
    for byte in surface.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    let start = 129 + (hash % 64) as u16;
    for offset in 0..64_u16 {
        let raw = 129 + ((start - 129 + offset) % 64);
        if used.insert(raw) {
            return LanguageTokenId::new(raw);
        }
    }
    Err(ScaffoldContractError::InvalidPerceptionFrame)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageEvaluationScores {
    pub unaided_trials: u64,
    pub unaided_successes: u64,
    pub assisted_trials: u64,
    pub assisted_successes: u64,
}

impl LanguageEvaluationScores {
    pub fn record(&mut self, assisted: bool, success: bool) {
        if assisted {
            self.assisted_trials = self.assisted_trials.saturating_add(1);
            self.assisted_successes = self.assisted_successes.saturating_add(u64::from(success));
        } else {
            self.unaided_trials = self.unaided_trials.saturating_add(1);
            self.unaided_successes = self.unaided_successes.saturating_add(u64::from(success));
        }
    }
}
