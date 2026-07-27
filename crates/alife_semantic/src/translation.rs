use std::collections::{BTreeMap, BTreeSet};

use alife_core::{
    Confidence, LanguageCodebookV1, LanguageTokenId, NovelLanguageToken, ScaffoldContractError,
    SpeechTranslationInput, SpeechTranslationReceipt, SpeechTranslationRequest, Validate,
};
use serde::{Deserialize, Serialize};

#[cfg(feature = "local-llamacpp")]
use crate::{
    local_llamacpp::{validate_local_llamacpp_host, LlamaCppServerClient},
    local_slm_prior::{
        parse_llamacpp_chat_response, CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS,
        CA27_DEFAULT_LLAMA_CPP_SLM_PORT,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslationAssistance {
    Disabled,
    SlmAssisted,
}

#[derive(Debug, Clone)]
pub struct BoundedSpeechTranslator {
    model_identity: String,
}

impl BoundedSpeechTranslator {
    pub fn new(
        model_identity: impl Into<String>,
        assistance: TranslationAssistance,
    ) -> Result<Self, ScaffoldContractError> {
        let model_identity = model_identity.into();
        if model_identity.trim().is_empty()
            || model_identity.chars().count() > 128
            || assistance == TranslationAssistance::SlmAssisted
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self { model_identity })
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
            assisted: false,
            uncertain,
        };
        receipt.validate_contract()?;
        Ok(receipt)
    }
}

#[cfg(feature = "local-llamacpp")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaCppSpeechTranslationConfig {
    pub host: String,
    pub port: u16,
    pub model: String,
    pub timeout_ms: u64,
    pub num_predict: u16,
}

#[cfg(feature = "local-llamacpp")]
impl Default for LlamaCppSpeechTranslationConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: CA27_DEFAULT_LLAMA_CPP_SLM_PORT,
            model: CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS.to_string(),
            timeout_ms: 5_000,
            num_predict: 96,
        }
    }
}

#[cfg(feature = "local-llamacpp")]
impl LlamaCppSpeechTranslationConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if validate_local_llamacpp_host(&self.host).is_err()
            || self.port == 0
            || self.model.trim().is_empty()
            || self.model.contains("http")
            || self.timeout_ms == 0
            || self.timeout_ms > 30_000
            || self.num_predict == 0
            || self.num_predict > 256
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[cfg(feature = "local-llamacpp")]
#[derive(Debug, Clone)]
pub struct LlamaCppSpeechTranslator {
    config: LlamaCppSpeechTranslationConfig,
}

#[cfg(feature = "local-llamacpp")]
impl LlamaCppSpeechTranslator {
    pub fn new(config: LlamaCppSpeechTranslationConfig) -> Result<Self, ScaffoldContractError> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn translate(
        &self,
        request: &SpeechTranslationRequest,
    ) -> Result<SpeechTranslationReceipt, String> {
        request
            .validate_contract()
            .map_err(|error| format!("invalid speech translation request: {error:?}"))?;
        let input = match &request.input {
            SpeechTranslationInput::PlayerText { text } => serde_json::json!({
                "kind": "player_text",
                "text": text,
                "known_surfaces": request.known_bindings.iter().map(|binding| binding.surface.as_str()).collect::<Vec<_>>(),
            }),
            SpeechTranslationInput::CreatureTokens { tokens } => {
                let codebook = LanguageCodebookV1::canonical();
                serde_json::json!({
                    "kind": "creature_tokens",
                    "tokens": tokens.iter().map(|token| token.raw()).collect::<Vec<_>>(),
                    "literal_words": tokens.iter().copied().map(|token| codebook.pronounceable_symbol(token)).collect::<Vec<_>>(),
                    "known_bindings": request.known_bindings.iter().map(|binding| serde_json::json!({"token": binding.token.raw(), "surface": binding.surface})).collect::<Vec<_>>(),
                })
            }
        };
        let system = concat!(
            "Translate only within the supplied bounded A-Life codebook. Return one JSON object and no prose. ",
            "For player_text return normalized_words, using only words present in text or known_surfaces. ",
            "For creature_tokens return rendered_words with exactly one supplied literal or bound word per token. ",
            "Never return actions, rewards, targets, scores, desirability, entities, or hidden concepts."
        );
        let body = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": input.to_string()}
            ],
            "stream": false,
            "temperature": 0.0,
            "max_tokens": self.config.num_predict,
            "response_format": {"type": "json_object"}
        })
        .to_string();
        let client = LlamaCppServerClient::new(
            self.config.host.clone(),
            self.config.port,
            self.config.timeout_ms,
        )
        .map_err(|error| format!("invalid local speech translator config: {error:?}"))?;
        let response = client.post_json("/v1/chat/completions", &body)?;
        let content = parse_llamacpp_chat_response(&response)?;
        let output = serde_json::from_str::<SlmSpeechTranslationOutput>(&content)
            .map_err(|error| format!("invalid local speech translation JSON: {error}"))?;
        self.receipt_from_output(request, output)
    }

    fn receipt_from_output(
        &self,
        request: &SpeechTranslationRequest,
        output: SlmSpeechTranslationOutput,
    ) -> Result<SpeechTranslationReceipt, String> {
        let unaided = BoundedSpeechTranslator::new(
            "alife-bounded-unaided-v1",
            TranslationAssistance::Disabled,
        )
        .map_err(|error| format!("invalid unaided translator: {error:?}"))?;
        let mut receipt = match &request.input {
            SpeechTranslationInput::PlayerText { text } => {
                let words = output
                    .normalized_words
                    .ok_or_else(|| "player translation omitted normalized_words".to_string())?;
                if output.rendered_words.is_some()
                    || words.is_empty()
                    || words.len() > usize::from(LanguageCodebookV1::MAX_HEARD_TOKENS)
                {
                    return Err("player translation returned the wrong bounded shape".to_string());
                }
                let source = normalized_words(text).into_iter().collect::<BTreeSet<_>>();
                let known = request
                    .known_bindings
                    .iter()
                    .map(|binding| binding.surface.to_lowercase())
                    .collect::<BTreeSet<_>>();
                let normalized = words
                    .into_iter()
                    .map(|word| normalize_model_word(&word))
                    .collect::<Result<Vec<_>, _>>()?;
                if normalized
                    .iter()
                    .any(|word| !source.contains(word) && !known.contains(word))
                {
                    return Err("local model invented an ungrounded speech concept".to_string());
                }
                let translated_request = SpeechTranslationRequest::try_new(
                    request.utterance_id,
                    request.addressee,
                    SpeechTranslationInput::PlayerText {
                        text: normalized.join(" "),
                    },
                    request.known_bindings.clone(),
                )
                .map_err(|error| format!("invalid normalized speech request: {error:?}"))?;
                unaided
                    .translate(&translated_request)
                    .map_err(|error| format!("bounded translation failed: {error:?}"))?
            }
            SpeechTranslationInput::CreatureTokens { tokens } => {
                let words = output
                    .rendered_words
                    .ok_or_else(|| "creature translation omitted rendered_words".to_string())?;
                if output.normalized_words.is_some() || words.len() != tokens.len() {
                    return Err("creature translation returned the wrong bounded shape".to_string());
                }
                let codebook = LanguageCodebookV1::canonical();
                let normalized = words
                    .into_iter()
                    .map(|word| normalize_model_word(&word))
                    .collect::<Result<Vec<_>, _>>()?;
                for (token, word) in tokens.iter().copied().zip(&normalized) {
                    let literal = codebook.pronounceable_symbol(token).to_lowercase();
                    let allowed = request
                        .known_bindings
                        .iter()
                        .filter(|binding| binding.token == token)
                        .any(|binding| binding.surface.eq_ignore_ascii_case(word));
                    if word != &literal && !allowed {
                        return Err("local model invented creature speech content".to_string());
                    }
                }
                let mut receipt = unaided
                    .translate(request)
                    .map_err(|error| format!("bounded translation failed: {error:?}"))?;
                receipt.rendered_text = normalized.join(" ");
                receipt
            }
        };
        receipt.model_identity = self.config.model.clone();
        receipt.assisted = true;
        receipt
            .validate_contract()
            .map_err(|error| format!("invalid assisted speech receipt: {error:?}"))?;
        Ok(receipt)
    }
}

#[cfg(feature = "local-llamacpp")]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SlmSpeechTranslationOutput {
    #[serde(default)]
    normalized_words: Option<Vec<String>>,
    #[serde(default)]
    rendered_words: Option<Vec<String>>,
}

#[cfg(feature = "local-llamacpp")]
fn normalize_model_word(word: &str) -> Result<String, String> {
    let normalized = normalized_words(word);
    if normalized.len() != 1 || normalized[0].chars().count() > 48 {
        return Err("local model returned an invalid bounded speech word".to_string());
    }
    Ok(normalized.into_iter().next().expect("one normalized word"))
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
