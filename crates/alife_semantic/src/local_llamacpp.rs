//! CA26: localhost-only semantic embedding provider boundary.
//!
//! This module talks to a local llama.cpp process backed by a locally installed
//! open-weight embedding model. It never calls a hosted inference API and it
//! returns bounded context data, not actions or weight updates.

use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::Path,
    time::Duration,
};

use alife_core::ScaffoldContractError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{build_semantic_context, SemanticCodeDescriptor, SemanticProviderCapabilityManifest};

pub const CA26_LOCAL_MODEL_MANIFEST_SCHEMA: &str = "alife.ca26.local_semantic_models.v1";
pub const CA26_LOCAL_MODEL_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const CA26_LOCAL_SEMANTIC_PROVIDER_ID: &str = "qwen3-embedding-0.6b-local-llamacpp";
pub const CA26_DEFAULT_LLAMA_CPP_EMBEDDING_ALIAS: &str = "alife-qwen3-embedding-0.6b";
pub const CA26_DEFAULT_LLAMA_CPP_HOST: &str = "127.0.0.1";
pub const CA26_DEFAULT_LLAMA_CPP_EMBEDDING_PORT: u16 = 18_082;
pub const CA26_EMBEDDING_PROJECTION_DIMS: usize = 32;
pub const CA26_MAX_RAW_EMBEDDING_DIMS: usize = 8_192;
pub const CA26_MAX_CONTEXT_INPUT_CHARS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSemanticModelManifest {
    pub schema: String,
    pub schema_version: u16,
    pub models: Vec<LocalSemanticModelEntry>,
}

impl LocalSemanticModelManifest {
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, String> {
        let text = fs::read_to_string(path.as_ref()).map_err(|err| err.to_string())?;
        Self::from_json_str(&text)
    }

    pub fn from_json_str(text: &str) -> Result<Self, String> {
        let manifest = serde_json::from_str::<Self>(text).map_err(|err| err.to_string())?;
        manifest.validate().map_err(|err| format!("{err:?}"))?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA26_LOCAL_MODEL_MANIFEST_SCHEMA
            || self.schema_version != CA26_LOCAL_MODEL_MANIFEST_SCHEMA_VERSION
            || self.models.is_empty()
            || self.models.len() > 4
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        for model in &self.models {
            model.validate()?;
        }
        Ok(())
    }

    pub fn semantic_embedding_model(&self) -> Option<&LocalSemanticModelEntry> {
        self.models
            .iter()
            .find(|model| model.model_role == "semantic_embedding_provider")
    }

    pub fn slm_subconscious_prior_model(&self) -> Option<&LocalSemanticModelEntry> {
        self.models
            .iter()
            .find(|model| model.model_role == "slm_subconscious_prior")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSemanticModelEntry {
    pub repo_id: String,
    #[serde(default)]
    pub target_repo_id: Option<String>,
    pub model_role: String,
    pub license: String,
    pub selected_file: String,
    pub runtime_backend: String,
    pub expected_local_path: String,
    pub llamacpp_alias: String,
    pub llamacpp_host: String,
    pub llamacpp_port: u16,
    pub sha256: String,
    pub downloaded_locally: bool,
    pub inference_smoke_passed: bool,
    pub limitations: Vec<String>,
}

impl LocalSemanticModelEntry {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.repo_id.trim().is_empty()
            || self
                .target_repo_id
                .as_deref()
                .is_some_and(|target| target.trim().is_empty())
            || self.model_role.trim().is_empty()
            || self.license.trim().is_empty()
            || self.selected_file.trim().is_empty()
            || self.runtime_backend != "llamacpp-server-gguf"
            || self.expected_local_path.trim().is_empty()
            || self.llamacpp_alias.trim().is_empty()
            || validate_local_llamacpp_host(&self.llamacpp_host).is_err()
            || self.llamacpp_port == 0
            || self.sha256.len() != 64
            || self.limitations.is_empty()
            || self.limitations.len() > 8
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.repo_id.contains("api")
            || self
                .target_repo_id
                .as_deref()
                .is_some_and(|target| target.contains("api") || target.contains("http"))
            || self.runtime_backend.contains("cloud")
            || self.llamacpp_alias.contains("http")
            || self.llamacpp_host.contains("://")
            || self.expected_local_path.contains("Entity(")
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaCppEmbeddingConfig {
    pub host: String,
    pub port: u16,
    pub model: String,
    pub timeout_ms: u64,
    pub max_input_chars: usize,
    pub projected_dims: usize,
}

impl Default for LlamaCppEmbeddingConfig {
    fn default() -> Self {
        Self {
            host: CA26_DEFAULT_LLAMA_CPP_HOST.to_string(),
            port: CA26_DEFAULT_LLAMA_CPP_EMBEDDING_PORT,
            model: CA26_DEFAULT_LLAMA_CPP_EMBEDDING_ALIAS.to_string(),
            timeout_ms: 120_000,
            max_input_chars: CA26_MAX_CONTEXT_INPUT_CHARS,
            projected_dims: CA26_EMBEDDING_PROJECTION_DIMS,
        }
    }
}

impl LlamaCppEmbeddingConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if validate_local_llamacpp_host(&self.host).is_err()
            || self.port == 0
            || self.model.trim().is_empty()
            || self.model.contains("http")
            || self.timeout_ms == 0
            || self.timeout_ms > 180_000
            || self.max_input_chars == 0
            || self.max_input_chars > CA26_MAX_CONTEXT_INPUT_CHARS
            || self.projected_dims != CA26_EMBEDDING_PROJECTION_DIMS
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

pub fn validate_local_llamacpp_host(host: &str) -> Result<(), ScaffoldContractError> {
    match host.trim() {
        "127.0.0.1" | "localhost" => Ok(()),
        _ => Err(ScaffoldContractError::ScalarOutOfRange),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlamaCppServerClient {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
}

impl LlamaCppServerClient {
    pub fn new(host: String, port: u16, timeout_ms: u64) -> Result<Self, ScaffoldContractError> {
        if validate_local_llamacpp_host(&host).is_err() || port == 0 || timeout_ms == 0 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(Self {
            host,
            port,
            timeout_ms,
        })
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn post_json(&self, path: &str, body: &str) -> Result<String, String> {
        if !path.starts_with("/v1/") || path.contains("://") {
            return Err("local llama.cpp path must be a relative /v1 endpoint".to_string());
        }
        let address = self.address();
        let timeout = Duration::from_millis(self.timeout_ms);
        let mut stream = TcpStream::connect(&address).map_err(|err| {
            format!("USER_ACTION_REQUIRED: local llama.cpp unavailable at {address}: {err}")
        })?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|err| err.to_string())?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|err| err.to_string())?;

        let http = format!(
            "POST {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(http.as_bytes())
            .map_err(|err| format!("local llama.cpp request failed: {err}"))?;
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|err| format!("local llama.cpp response failed: {err}"))?;
        Ok(response)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundedSemanticEmbedding {
    pub model: String,
    pub raw_dims: usize,
    pub projected: [i8; CA26_EMBEDDING_PROJECTION_DIMS],
    pub projected_norm: f32,
    pub semantic_code_count: usize,
    pub can_issue_actions: bool,
    pub can_rewrite_weights: bool,
}

impl BoundedSemanticEmbedding {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.model.trim().is_empty()
            || self.raw_dims == 0
            || self.raw_dims > CA26_MAX_RAW_EMBEDDING_DIMS
            || self.semantic_code_count == 0
            || self.projected_norm.is_nan()
            || !(0.0..=1.0).contains(&self.projected_norm)
            || self.can_issue_actions
            || self.can_rewrite_weights
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LlamaCppEmbeddingProvider {
    pub config: LlamaCppEmbeddingConfig,
}

impl LlamaCppEmbeddingProvider {
    pub fn new(config: LlamaCppEmbeddingConfig) -> Result<Self, ScaffoldContractError> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn capability_manifest(&self, available: bool) -> SemanticProviderCapabilityManifest {
        SemanticProviderCapabilityManifest::local_llamacpp_embedding(
            CA26_LOCAL_SEMANTIC_PROVIDER_ID,
            available,
        )
    }

    pub fn embed_text(&self, input: &str) -> Result<BoundedSemanticEmbedding, String> {
        self.config
            .validate()
            .map_err(|err| format!("invalid CA26 local provider config: {err:?}"))?;
        validate_input(input, self.config.max_input_chars)?;
        let raw = self.request_embedding(input)?;
        let projected = project_embedding_to_i8(&raw)?;
        let projected_norm = projected
            .iter()
            .map(|value| f32::from(*value).abs() / 127.0)
            .fold(0.0_f32, f32::max);
        let embedding = BoundedSemanticEmbedding {
            model: self.config.model.clone(),
            raw_dims: raw.len(),
            projected,
            projected_norm,
            semantic_code_count: 1,
            can_issue_actions: false,
            can_rewrite_weights: false,
        };
        embedding
            .validate()
            .map_err(|err| format!("invalid CA26 embedding result: {err:?}"))?;
        Ok(embedding)
    }

    pub fn build_bounded_context(
        &self,
        input: &str,
    ) -> Result<(BoundedSemanticEmbedding, alife_core::SemanticContextRef), String> {
        let embedding = self.embed_text(input)?;
        let context = build_semantic_context(
            &[],
            &[SemanticCodeDescriptor {
                codebook_id: 26,
                descriptor: embedding.projected,
                salience: 0.75,
            }],
            0.75,
        )
        .map_err(|err| format!("semantic context projection failed: {err:?}"))?
        .ok_or_else(|| "local semantic embedding produced no bounded context".to_string())?;
        Ok((embedding, context))
    }

    fn request_embedding(&self, input: &str) -> Result<Vec<f32>, String> {
        let request = serde_json::json!({
            "model": self.config.model,
            "input": input,
        });
        let body = request.to_string();
        let client = LlamaCppServerClient::new(
            self.config.host.clone(),
            self.config.port,
            self.config.timeout_ms,
        )
        .map_err(|err| format!("invalid local llama.cpp client config: {err:?}"))?;
        let response = client.post_json("/v1/embeddings", &body)?;
        parse_llamacpp_embedding_response(&response)
    }
}

fn validate_input(input: &str, max_chars: usize) -> Result<(), String> {
    let count = input.chars().count();
    if count == 0 || count > max_chars || input.contains("Entity(") {
        return Err("CA26 semantic input must be bounded player-facing text".to_string());
    }
    Ok(())
}

pub fn project_embedding_to_i8(
    raw: &[f32],
) -> Result<[i8; CA26_EMBEDDING_PROJECTION_DIMS], String> {
    if raw.is_empty()
        || raw.len() > CA26_MAX_RAW_EMBEDDING_DIMS
        || raw.iter().any(|v| !v.is_finite())
    {
        return Err("CA26 embedding vector is empty, too large, NaN, or Inf".to_string());
    }
    let max_abs = raw.iter().map(|value| value.abs()).fold(0.0_f32, f32::max);
    if max_abs == 0.0 {
        return Ok([0; CA26_EMBEDDING_PROJECTION_DIMS]);
    }
    let mut projected = [0_i8; CA26_EMBEDDING_PROJECTION_DIMS];
    for (slot, value) in projected.iter_mut().enumerate() {
        let start = slot * raw.len() / CA26_EMBEDDING_PROJECTION_DIMS;
        let end = ((slot + 1) * raw.len() / CA26_EMBEDDING_PROJECTION_DIMS).max(start + 1);
        let end = end.min(raw.len());
        let avg = raw[start..end].iter().copied().sum::<f32>() / (end - start) as f32;
        *value = (avg / max_abs * 127.0).clamp(-127.0, 127.0).round() as i8;
    }
    Ok(projected)
}

#[derive(Debug, Deserialize)]
struct LlamaCppEmbeddingResponse {
    model: Option<String>,
    data: Vec<LlamaCppEmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct LlamaCppEmbeddingData {
    embedding: Vec<f32>,
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

fn parse_llamacpp_embedding_response(response: &str) -> Result<Vec<f32>, String> {
    let (header, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "local llama.cpp response missing HTTP body".to_string())?;
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
            "USER_ACTION_REQUIRED: local llama.cpp embedding request failed: {message}"
        ));
    }
    let response =
        serde_json::from_str::<LlamaCppEmbeddingResponse>(&body).map_err(|err| err.to_string())?;
    if response
        .model
        .as_deref()
        .is_some_and(|model| model.trim().is_empty())
    {
        return Err("local llama.cpp returned an empty model label".to_string());
    }
    let embedding = response
        .data
        .into_iter()
        .next()
        .map(|item| item.embedding)
        .ok_or_else(|| "local llama.cpp returned no embedding vectors".to_string())?;
    if embedding.is_empty()
        || embedding.len() > CA26_MAX_RAW_EMBEDDING_DIMS
        || embedding.iter().any(|value| !value.is_finite())
    {
        return Err("local llama.cpp returned invalid embedding values".to_string());
    }
    Ok(embedding)
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

    #[test]
    fn projection_rejects_invalid_values_and_bounds_output() {
        assert!(project_embedding_to_i8(&[]).is_err());
        assert!(project_embedding_to_i8(&[f32::NAN]).is_err());

        let raw = (0..1024)
            .map(|index| (index as f32 / 1024.0) - 0.5)
            .collect::<Vec<_>>();
        let projected = project_embedding_to_i8(&raw).unwrap();
        assert_eq!(projected.len(), CA26_EMBEDDING_PROJECTION_DIMS);
        assert!(projected.iter().all(|value| *value >= -127));
    }

    #[test]
    fn local_model_manifest_validates_real_provider_boundary() {
        let manifest = LocalSemanticModelManifest::from_json_str(
            r#"{
                "schema":"alife.ca26.local_semantic_models.v1",
                "schema_version":1,
                "models":[{
                    "repo_id":"Qwen/Qwen3-Embedding-0.6B-GGUF",
                    "model_role":"semantic_embedding_provider",
                    "license":"apache-2.0",
                    "selected_file":"Qwen3-Embedding-0.6B-Q8_0.gguf",
                    "runtime_backend":"llamacpp-server-gguf",
                    "expected_local_path":"models/local/qwen3-embedding-0.6b-gguf/Qwen3-Embedding-0.6B-Q8_0.gguf",
                    "llamacpp_alias":"alife-qwen3-embedding-0.6b",
                    "llamacpp_host":"127.0.0.1",
                    "llamacpp_port":18082,
                    "sha256":"06507c7b42688469c4e7298b0a1e16deff06caf291cf0a5b278c308249c3e439",
                    "downloaded_locally":true,
                    "inference_smoke_passed":true,
                    "limitations":["localhost-only","perception-context-only"]
                }]
            }"#,
        )
        .unwrap();
        let model = manifest.semantic_embedding_model().unwrap();
        assert_eq!(model.model_role, "semantic_embedding_provider");
    }

    #[test]
    fn unavailable_llamacpp_alias_is_user_action_required_not_fake_output() {
        let provider = LlamaCppEmbeddingProvider::new(LlamaCppEmbeddingConfig {
            port: 9,
            timeout_ms: 1_000,
            ..LlamaCppEmbeddingConfig::default()
        })
        .unwrap();
        let err = provider.embed_text("teacher token food").unwrap_err();
        assert!(err.contains("USER_ACTION_REQUIRED"));
    }

    #[test]
    fn chunked_llamacpp_response_decodes_before_json_parse() {
        let decoded = decode_chunked_http_body("5\r\nhello\r\n0\r\n\r\n").unwrap();
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn openai_compatible_embedding_response_parses() {
        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Content-Type: application/json\r\n",
            "\r\n",
            "{\"model\":\"m\",\"data\":[{\"embedding\":[1.0,2.0]}]}"
        );
        let embedding = parse_llamacpp_embedding_response(response).unwrap();
        assert_eq!(embedding, vec![1.0, 2.0]);
    }

    #[test]
    fn remote_llamacpp_hosts_are_rejected() {
        assert!(validate_local_llamacpp_host("127.0.0.1").is_ok());
        assert!(validate_local_llamacpp_host("localhost").is_ok());
        assert!(validate_local_llamacpp_host("https://api.example.com").is_err());
        assert!(validate_local_llamacpp_host("192.168.1.5").is_err());
    }
}
