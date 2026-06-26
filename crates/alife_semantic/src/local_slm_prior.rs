//! CA27: localhost-only internal SLM subconscious prior boundary.
//!
//! The local SLM prior produces bounded perception/context hints from a real
//! local llama.cpp model. It is private prior data: no action commands, motor
//! bypasses, hidden vectors, or weight updates are exposed.

use std::{
    collections::VecDeque,
    sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TrySendError},
    thread,
    time::Duration,
};

use alife_core::ScaffoldContractError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::local_llamacpp::{
    validate_local_llamacpp_host, LlamaCppServerClient, CA26_DEFAULT_LLAMA_CPP_HOST,
};

pub const CA27_SLM_PRIOR_OUTPUT_SCHEMA: &str = "alife.ca27.local_slm_prior_output.v1";
pub const CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION: u16 = 1;
pub const CA27_LOCAL_SLM_PRIOR_ID: &str = "qwen3-4b-local-slm-prior";
pub const CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS: &str = "alife-qwen3-4b-prior";
pub const CA27_DEFAULT_LLAMA_CPP_SLM_PORT: u16 = 18_081;
pub const CA27_MAX_PROMPT_CHARS: usize = 768;
pub const CA27_MAX_CONTEXT_SUMMARY_CHARS: usize = 160;
pub const CA27_MAX_SALIENCE_LABELS: usize = 4;
pub const CA27_MAX_LEXICON_ASSOCIATIONS: usize = 6;
pub const CA27_MAX_PERCEPTION_TAGS: usize = 6;
pub const CA27_MAX_QUEUE_DEPTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaCppSlmPriorConfig {
    pub host: String,
    pub port: u16,
    pub model: String,
    pub timeout_ms: u64,
    pub max_prompt_chars: usize,
    pub max_queue_depth: usize,
    pub num_predict: u16,
}

impl Default for LlamaCppSlmPriorConfig {
    fn default() -> Self {
        Self {
            host: CA26_DEFAULT_LLAMA_CPP_HOST.to_string(),
            port: CA27_DEFAULT_LLAMA_CPP_SLM_PORT,
            model: CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS.to_string(),
            timeout_ms: 180_000,
            max_prompt_chars: CA27_MAX_PROMPT_CHARS,
            max_queue_depth: CA27_MAX_QUEUE_DEPTH,
            num_predict: 192,
        }
    }
}

impl LlamaCppSlmPriorConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if validate_local_llamacpp_host(&self.host).is_err()
            || self.port == 0
            || self.model.trim().is_empty()
            || self.model.contains("http")
            || self.timeout_ms == 0
            || self.timeout_ms > 240_000
            || self.max_prompt_chars == 0
            || self.max_prompt_chars > CA27_MAX_PROMPT_CHARS
            || self.max_queue_depth == 0
            || self.max_queue_depth > CA27_MAX_QUEUE_DEPTH
            || self.num_predict == 0
            || self.num_predict > 512
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSlmPriorRequest {
    pub request_id: u64,
    pub prompt: String,
}

impl LocalSlmPriorRequest {
    pub fn validate(&self, max_prompt_chars: usize) -> Result<(), ScaffoldContractError> {
        let chars = self.prompt.chars().count();
        if self.request_id == 0
            || chars == 0
            || chars > max_prompt_chars
            || contains_forbidden_runtime_text(&self.prompt)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlmLexiconAssociation {
    pub token: String,
    pub salience: f32,
}

impl SlmLexiconAssociation {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if !is_bounded_label(&self.token, 32)
            || !self.salience.is_finite()
            || !(0.0..=1.0).contains(&self.salience)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalSlmPriorOutput {
    pub schema: String,
    pub schema_version: u16,
    pub model: String,
    pub salience_labels: Vec<String>,
    pub context_summary: String,
    pub lexicon_associations: Vec<SlmLexiconAssociation>,
    pub perception_tags: Vec<String>,
    pub can_issue_actions: bool,
    pub can_rewrite_weights: bool,
    pub can_bypass_arbitration: bool,
    pub hidden_vector_injection: bool,
    pub bounded_context_only: bool,
}

impl LocalSlmPriorOutput {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA27_SLM_PRIOR_OUTPUT_SCHEMA
            || self.schema_version != CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION
            || self.model.trim().is_empty()
            || self.salience_labels.is_empty()
            || self.salience_labels.len() > CA27_MAX_SALIENCE_LABELS
            || !is_bounded_label(&self.context_summary, CA27_MAX_CONTEXT_SUMMARY_CHARS)
            || self.lexicon_associations.is_empty()
            || self.lexicon_associations.len() > CA27_MAX_LEXICON_ASSOCIATIONS
            || self.perception_tags.is_empty()
            || self.perception_tags.len() > CA27_MAX_PERCEPTION_TAGS
            || self.can_issue_actions
            || self.can_rewrite_weights
            || self.can_bypass_arbitration
            || self.hidden_vector_injection
            || !self.bounded_context_only
            || contains_forbidden_runtime_text(&self.context_summary)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self
            .salience_labels
            .iter()
            .chain(self.perception_tags.iter())
            .any(|value| !is_bounded_label(value, 32) || contains_forbidden_runtime_text(value))
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for association in &self.lexicon_associations {
            association.validate()?;
            if contains_forbidden_runtime_text(&association.token) {
                return Err(ScaffoldContractError::ScalarOutOfRange);
            }
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.model,
            self.salience_labels.len(),
            self.lexicon_associations.len(),
            self.perception_tags.len(),
            self.can_issue_actions,
            self.can_rewrite_weights,
            self.hidden_vector_injection
        )
    }
}

#[derive(Debug, Clone)]
pub struct LocalSlmPriorQueue {
    config: LlamaCppSlmPriorConfig,
    pending: VecDeque<LocalSlmPriorRequest>,
}

#[derive(Debug)]
enum LocalSlmPriorWork {
    Generate(
        LocalSlmPriorRequest,
        mpsc::Sender<Result<LocalSlmPriorOutput, String>>,
    ),
}

/// Bounded asynchronous worker queue for CA27 local SLM prior requests.
///
/// The queue is asynchronous in the app sense: enqueue is non-blocking, local
/// model inference runs on a dedicated worker thread, and callers receive a
/// bounded result handle. The model call itself remains a localhost llama.cpp HTTP
/// request with explicit timeouts.
#[derive(Debug)]
pub struct LocalSlmPriorAsyncQueue {
    config: LlamaCppSlmPriorConfig,
    sender: SyncSender<LocalSlmPriorWork>,
}

impl LocalSlmPriorAsyncQueue {
    pub fn new(config: LlamaCppSlmPriorConfig) -> Result<Self, ScaffoldContractError> {
        config.validate()?;
        let provider = LlamaCppSlmPriorProvider::new(config.clone())?;
        let (sender, receiver) = mpsc::sync_channel(config.max_queue_depth);
        thread::Builder::new()
            .name("alife-ca27-local-slm-prior".to_string())
            .spawn(move || run_slm_prior_worker(provider, receiver))
            .map_err(|_| ScaffoldContractError::MissingPhaseData)?;
        Ok(Self { config, sender })
    }

    pub fn capacity(&self) -> usize {
        self.config.max_queue_depth
    }

    pub fn timeout_ms(&self) -> u64 {
        self.config.timeout_ms
    }

    pub fn submit(
        &self,
        request: LocalSlmPriorRequest,
    ) -> Result<Receiver<Result<LocalSlmPriorOutput, String>>, ScaffoldContractError> {
        request.validate(self.config.max_prompt_chars)?;
        let (reply_tx, reply_rx) = mpsc::channel();
        match self
            .sender
            .try_send(LocalSlmPriorWork::Generate(request, reply_tx))
        {
            Ok(()) => Ok(reply_rx),
            Err(TrySendError::Full(_)) => Err(ScaffoldContractError::ScalarOutOfRange),
            Err(TrySendError::Disconnected(_)) => Err(ScaffoldContractError::MissingPhaseData),
        }
    }

    pub fn wait_for(
        &self,
        receiver: Receiver<Result<LocalSlmPriorOutput, String>>,
    ) -> Result<LocalSlmPriorOutput, String> {
        match receiver.recv_timeout(Duration::from_millis(self.config.timeout_ms)) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => Err(format!(
                "USER_ACTION_REQUIRED: local SLM prior timed out after {} ms",
                self.config.timeout_ms
            )),
            Err(RecvTimeoutError::Disconnected) => {
                Err("USER_ACTION_REQUIRED: local SLM prior worker disconnected".to_string())
            }
        }
    }
}

fn run_slm_prior_worker(
    provider: LlamaCppSlmPriorProvider,
    receiver: mpsc::Receiver<LocalSlmPriorWork>,
) {
    while let Ok(work) = receiver.recv() {
        match work {
            LocalSlmPriorWork::Generate(request, reply) => {
                let result = request
                    .validate(provider.config.max_prompt_chars)
                    .map_err(|err| format!("CA27 queued request invalid: {err:?}"))
                    .and_then(|_| provider.generate_prior(&request.prompt));
                let _ = reply.send(result);
            }
        }
    }
}

impl LocalSlmPriorQueue {
    pub fn new(config: LlamaCppSlmPriorConfig) -> Result<Self, ScaffoldContractError> {
        config.validate()?;
        Ok(Self {
            config,
            pending: VecDeque::new(),
        })
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn enqueue(&mut self, request: LocalSlmPriorRequest) -> Result<(), ScaffoldContractError> {
        request.validate(self.config.max_prompt_chars)?;
        if self.pending.len() >= self.config.max_queue_depth {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        self.pending.push_back(request);
        Ok(())
    }

    pub fn process_next(
        &mut self,
        provider: &LlamaCppSlmPriorProvider,
    ) -> Result<Option<LocalSlmPriorOutput>, String> {
        let Some(request) = self.pending.pop_front() else {
            return Ok(None);
        };
        request
            .validate(self.config.max_prompt_chars)
            .map_err(|err| format!("CA27 queued request invalid: {err:?}"))?;
        provider.generate_prior(&request.prompt).map(Some)
    }
}

#[derive(Debug, Clone)]
pub struct LlamaCppSlmPriorProvider {
    pub config: LlamaCppSlmPriorConfig,
}

impl LlamaCppSlmPriorProvider {
    pub fn new(config: LlamaCppSlmPriorConfig) -> Result<Self, ScaffoldContractError> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn generate_prior(&self, bounded_context: &str) -> Result<LocalSlmPriorOutput, String> {
        self.config
            .validate()
            .map_err(|err| format!("invalid CA27 local SLM prior config: {err:?}"))?;
        LocalSlmPriorRequest {
            request_id: 1,
            prompt: bounded_context.to_string(),
        }
        .validate(self.config.max_prompt_chars)
        .map_err(|err| format!("invalid CA27 SLM context: {err:?}"))?;
        let raw = self.request_generate(bounded_context)?;
        parse_slm_prior_json(&self.config.model, &raw)
    }

    fn request_generate(&self, bounded_context: &str) -> Result<String, String> {
        let system_prompt = concat!(
            "You are a private A-Life subconscious semantic prior. ",
            "Return exactly one compact JSON object and no prose. ",
            "Do not include thinking text, markdown fences, commands, motor plans, ",
            "weight changes, vectors, Bevy entities, or arbitration text. ",
            "Allowed keys only: salience_labels, context_summary, lexicon_associations, perception_tags."
        );
        let user_prompt = format!(
            concat!(
                "Required shape: {{\"salience_labels\":[\"food\",\"hazard\"],",
                "\"context_summary\":\"short sensory context\",",
                "\"lexicon_associations\":{{\"food\":0.8,\"hazard\":0.7}},",
                "\"perception_tags\":[\"near\",\"sees\"]}}. ",
                "Use only lowercase short labels. Context: {}"
            ),
            bounded_context
        );
        let request = serde_json::json!({
            "model": self.config.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "stream": false,
            "temperature": 0.0,
            "max_tokens": self.config.num_predict,
            "response_format": {"type": "json_object"}
        });
        let body = request.to_string();
        let client = LlamaCppServerClient::new(
            self.config.host.clone(),
            self.config.port,
            self.config.timeout_ms,
        )
        .map_err(|err| format!("invalid local llama.cpp client config: {err:?}"))?;
        let response = client.post_json("/v1/chat/completions", &body)?;
        parse_llamacpp_chat_response(&response)
    }
}

#[derive(Debug, Deserialize)]
struct LlamaCppChatResponse {
    choices: Vec<LlamaCppChatChoice>,
}

#[derive(Debug, Deserialize)]
struct LlamaCppChatChoice {
    message: LlamaCppChatMessage,
}

#[derive(Debug, Deserialize)]
struct LlamaCppChatMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct LlamaCppErrorResponse {
    error: LlamaCppErrorValue,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LlamaCppErrorValue {
    Message { message: String },
    Text(String),
    Other(Value),
}

impl LlamaCppErrorValue {
    fn into_message(self) -> String {
        match self {
            Self::Message { message } => message,
            Self::Text(text) => text,
            Self::Other(value) => value.to_string(),
        }
    }
}

fn parse_llamacpp_chat_response(response: &str) -> Result<String, String> {
    let (header, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "local llama.cpp generation response missing HTTP body".to_string())?;
    let body = if header
        .lines()
        .any(|line| line.eq_ignore_ascii_case("Transfer-Encoding: chunked"))
    {
        decode_chunked_http_body(body)?
    } else {
        body.to_string()
    };
    if !header.starts_with("HTTP/1.1 200") && !header.starts_with("HTTP/1.0 200") {
        let message = serde_json::from_str::<LlamaCppErrorResponse>(&body)
            .map(|err| err.error.into_message())
            .unwrap_or_else(|_| body.trim().to_string());
        return Err(format!(
            "USER_ACTION_REQUIRED: local llama.cpp SLM prior request failed: {message}"
        ));
    }
    let response =
        serde_json::from_str::<LlamaCppChatResponse>(&body).map_err(|err| err.to_string())?;
    let content = response
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| "local llama.cpp returned no chat choices".to_string())?;
    let content = extract_json_object_text(&content)?;
    if content.trim().is_empty() {
        return Err("local llama.cpp returned empty CA27 SLM output".to_string());
    }
    Ok(content)
}

fn extract_json_object_text(content: &str) -> Result<String, String> {
    let trimmed = content.trim();
    let start = trimmed
        .find('{')
        .ok_or_else(|| "local llama.cpp SLM output did not contain a JSON object".to_string())?;
    let end = trimmed
        .rfind('}')
        .ok_or_else(|| "local llama.cpp SLM output JSON object was incomplete".to_string())?;
    if end < start {
        return Err("local llama.cpp SLM output JSON bounds are invalid".to_string());
    }
    Ok(trimmed[start..=end].to_string())
}

pub fn parse_slm_prior_json(model: &str, json: &str) -> Result<LocalSlmPriorOutput, String> {
    if json.chars().count() > 2_048 || contains_forbidden_runtime_text(json) {
        return Err("CA27 SLM output contains forbidden runtime authority text".to_string());
    }
    let value = serde_json::from_str::<Value>(json).map_err(|err| err.to_string())?;
    let object = value
        .as_object()
        .ok_or_else(|| "CA27 SLM output must be a JSON object".to_string())?;
    for key in object.keys() {
        match key.as_str() {
            "salience_labels" | "context_summary" | "lexicon_associations" | "perception_tags" => {}
            _ => return Err(format!("CA27 SLM output contains forbidden key: {key}")),
        }
    }

    let salience_labels = bounded_string_array(
        object
            .get("salience_labels")
            .ok_or_else(|| "missing salience_labels".to_string())?,
        CA27_MAX_SALIENCE_LABELS,
        32,
    )?;
    let context_summary = object
        .get("context_summary")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing context_summary".to_string())?
        .trim()
        .to_string();
    let perception_tags = bounded_string_array(
        object
            .get("perception_tags")
            .ok_or_else(|| "missing perception_tags".to_string())?,
        CA27_MAX_PERCEPTION_TAGS,
        32,
    )?;
    let lexicon_associations = parse_lexicon_associations(
        object
            .get("lexicon_associations")
            .ok_or_else(|| "missing lexicon_associations".to_string())?,
    )?;

    let output = LocalSlmPriorOutput {
        schema: CA27_SLM_PRIOR_OUTPUT_SCHEMA.to_string(),
        schema_version: CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION,
        model: model.to_string(),
        salience_labels,
        context_summary,
        lexicon_associations,
        perception_tags,
        can_issue_actions: false,
        can_rewrite_weights: false,
        can_bypass_arbitration: false,
        hidden_vector_injection: false,
        bounded_context_only: true,
    };
    output
        .validate()
        .map_err(|err| format!("CA27 SLM prior output failed validation: {err:?}"))?;
    Ok(output)
}

fn parse_lexicon_associations(value: &Value) -> Result<Vec<SlmLexiconAssociation>, String> {
    let associations = if let Some(object) = value.as_object() {
        object
            .iter()
            .map(|(token, salience)| SlmLexiconAssociation {
                token: token.trim().to_string(),
                salience: salience.as_f64().unwrap_or(f64::NAN) as f32,
            })
            .collect::<Vec<_>>()
    } else if let Some(array) = value.as_array() {
        array
            .iter()
            .map(|item| {
                let item = item
                    .as_object()
                    .ok_or_else(|| "lexicon association must be an object".to_string())?;
                Ok(SlmLexiconAssociation {
                    token: item
                        .get("token")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                    salience: item
                        .get("salience")
                        .and_then(Value::as_f64)
                        .unwrap_or(f64::NAN) as f32,
                })
            })
            .collect::<Result<Vec<_>, String>>()?
    } else {
        return Err("lexicon_associations must be an object or array".to_string());
    };
    if associations.is_empty() || associations.len() > CA27_MAX_LEXICON_ASSOCIATIONS {
        return Err("lexicon association count is out of range".to_string());
    }
    for association in &associations {
        association
            .validate()
            .map_err(|err| format!("invalid lexicon association: {err:?}"))?;
    }
    Ok(associations)
}

fn bounded_string_array(
    value: &Value,
    max_len: usize,
    max_chars: usize,
) -> Result<Vec<String>, String> {
    let array = value
        .as_array()
        .ok_or_else(|| "expected bounded string array".to_string())?;
    if array.is_empty() || array.len() > max_len {
        return Err("bounded string array length is out of range".to_string());
    }
    array
        .iter()
        .map(|item| {
            let text = item
                .as_str()
                .ok_or_else(|| "bounded array item must be a string".to_string())?
                .trim()
                .to_string();
            if !is_bounded_label(&text, max_chars) || contains_forbidden_runtime_text(&text) {
                return Err("bounded string is empty, too long, or forbidden".to_string());
            }
            Ok(text)
        })
        .collect()
}

fn is_bounded_label(value: &str, max_chars: usize) -> bool {
    let count = value.chars().count();
    count > 0 && count <= max_chars && !value.contains("Entity(")
}

fn contains_forbidden_runtime_text(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "actioncommand",
        "action proposal",
        "motor command",
        "motor bypass",
        "rewrite weight",
        "write weights",
        "w_genetic_fixed",
        "h_operational",
        "entity(",
        "bevy entity",
        "arbitration instruction",
        "hidden vector",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn decode_chunked_http_body(body: &str) -> Result<String, String> {
    let mut remaining = body;
    let mut decoded = String::new();
    loop {
        let (size_line, rest) = remaining
            .split_once("\r\n")
            .ok_or_else(|| "chunked llama.cpp response missing chunk size".to_string())?;
        let size_text = size_line.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| "chunked llama.cpp response has invalid chunk size".to_string())?;
        if size == 0 {
            return Ok(decoded);
        }
        if rest.len() < size + 2 {
            return Err("chunked llama.cpp response ended inside a chunk".to_string());
        }
        decoded.push_str(&rest[..size]);
        let trailer = &rest[size..];
        if !trailer.starts_with("\r\n") {
            return Err("chunked llama.cpp response missing chunk terminator".to_string());
        }
        remaining = &trailer[2..];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> &'static str {
        r#"{
            "salience_labels":["food","hazard"],
            "context_summary":"Creature sees food near a hazard.",
            "lexicon_associations":{"food":0.95,"hazard":0.82},
            "perception_tags":["near","sees"]
        }"#
    }

    #[test]
    fn parser_accepts_bounded_structured_prior_without_authority() {
        let output = parse_slm_prior_json(CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS, valid_json()).unwrap();
        assert_eq!(output.salience_labels.len(), 2);
        assert_eq!(output.lexicon_associations.len(), 2);
        assert!(!output.can_issue_actions);
        assert!(!output.can_rewrite_weights);
        assert!(!output.can_bypass_arbitration);
        assert!(!output.hidden_vector_injection);
        output.validate().unwrap();
    }

    #[test]
    fn malformed_or_authoritative_output_rejects() {
        assert!(parse_slm_prior_json(CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS, "{}").is_err());
        assert!(parse_slm_prior_json(
            CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS,
            r#"{
                "salience_labels":["food"],
                "context_summary":"Creature sees food.",
                "lexicon_associations":{"food":0.9},
                "perception_tags":["near"],
                "action":"eat now"
            }"#
        )
        .is_err());
        assert!(parse_slm_prior_json(
            CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS,
            r#"{
                "salience_labels":["motor command"],
                "context_summary":"Creature sees food.",
                "lexicon_associations":{"food":0.9},
                "perception_tags":["near"]
            }"#
        )
        .is_err());
    }

    #[test]
    fn openai_compatible_chat_response_extracts_bounded_json() {
        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: application/json\r\n",
            "\r\n",
            "{\"choices\":[{\"message\":{\"content\":\"```json\\n{\\\"salience_labels\\\":[\\\"food\\\"],\\\"context_summary\\\":\\\"food nearby\\\",\\\"lexicon_associations\\\":{\\\"food\\\":0.8},\\\"perception_tags\\\":[\\\"near\\\"]}\\n```\"}}]}"
        );
        let json = parse_llamacpp_chat_response(response).unwrap();
        let output = parse_slm_prior_json(CA27_DEFAULT_LLAMA_CPP_SLM_ALIAS, &json).unwrap();
        assert_eq!(output.salience_labels, vec!["food"]);
        assert!(!output.can_issue_actions);
    }

    #[test]
    fn remote_llamacpp_slm_host_rejects() {
        assert!(LlamaCppSlmPriorConfig {
            host: "localhost".to_string(),
            ..LlamaCppSlmPriorConfig::default()
        }
        .validate()
        .is_ok());
        assert!(LlamaCppSlmPriorConfig {
            host: "https://api.example.com".to_string(),
            ..LlamaCppSlmPriorConfig::default()
        }
        .validate()
        .is_err());
    }

    #[test]
    fn queue_is_bounded_and_preserves_request_validation() {
        let config = LlamaCppSlmPriorConfig {
            max_queue_depth: 2,
            ..LlamaCppSlmPriorConfig::default()
        };
        let mut queue = LocalSlmPriorQueue::new(config).unwrap();
        queue
            .enqueue(LocalSlmPriorRequest {
                request_id: 1,
                prompt: "teacher token food".to_string(),
            })
            .unwrap();
        queue
            .enqueue(LocalSlmPriorRequest {
                request_id: 2,
                prompt: "teacher token hazard".to_string(),
            })
            .unwrap();
        assert_eq!(queue.pending_len(), 2);
        assert!(queue
            .enqueue(LocalSlmPriorRequest {
                request_id: 3,
                prompt: "teacher token peer".to_string(),
            })
            .is_err());
    }

    #[test]
    fn unavailable_local_model_is_user_action_required_not_fake_output() {
        let provider = LlamaCppSlmPriorProvider::new(LlamaCppSlmPriorConfig {
            port: 9,
            timeout_ms: 1_000,
            ..LlamaCppSlmPriorConfig::default()
        })
        .unwrap();
        let err = provider.generate_prior("teacher token food").unwrap_err();
        assert!(err.contains("USER_ACTION_REQUIRED"));
    }
}
