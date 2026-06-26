//! CA26: localhost-only semantic embedding provider boundary.
//!
//! This module talks to a local Ollama process backed by a locally installed
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

use crate::{build_semantic_context, SemanticCodeDescriptor, SemanticProviderCapabilityManifest};

pub const CA26_LOCAL_MODEL_MANIFEST_SCHEMA: &str = "alife.ca26.local_semantic_models.v1";
pub const CA26_LOCAL_MODEL_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const CA26_LOCAL_SEMANTIC_PROVIDER_ID: &str = "qwen3-embedding-0.6b-local-ollama";
pub const CA26_DEFAULT_OLLAMA_MODEL: &str = "alife-qwen3-embedding-0.6b";
pub const CA26_DEFAULT_OLLAMA_HOST: &str = "127.0.0.1";
pub const CA26_DEFAULT_OLLAMA_PORT: u16 = 11_434;
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
    pub ollama_model: String,
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
            || self.runtime_backend != "ollama-localhost-gguf"
            || self.expected_local_path.trim().is_empty()
            || self.ollama_model.trim().is_empty()
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
            || self.ollama_model.contains("http")
            || self.expected_local_path.contains("Entity(")
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalOllamaEmbeddingConfig {
    pub host: String,
    pub port: u16,
    pub model: String,
    pub timeout_ms: u64,
    pub max_input_chars: usize,
    pub projected_dims: usize,
}

impl Default for LocalOllamaEmbeddingConfig {
    fn default() -> Self {
        Self {
            host: CA26_DEFAULT_OLLAMA_HOST.to_string(),
            port: CA26_DEFAULT_OLLAMA_PORT,
            model: CA26_DEFAULT_OLLAMA_MODEL.to_string(),
            timeout_ms: 120_000,
            max_input_chars: CA26_MAX_CONTEXT_INPUT_CHARS,
            projected_dims: CA26_EMBEDDING_PROJECTION_DIMS,
        }
    }
}

impl LocalOllamaEmbeddingConfig {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.host != CA26_DEFAULT_OLLAMA_HOST
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
pub struct LocalOllamaEmbeddingProvider {
    pub config: LocalOllamaEmbeddingConfig,
}

impl LocalOllamaEmbeddingProvider {
    pub fn new(config: LocalOllamaEmbeddingConfig) -> Result<Self, ScaffoldContractError> {
        config.validate()?;
        Ok(Self { config })
    }

    pub fn capability_manifest(&self, available: bool) -> SemanticProviderCapabilityManifest {
        SemanticProviderCapabilityManifest::local_ollama_embedding(
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
            "input": [input],
        });
        let body = request.to_string();
        let address = format!("{}:{}", self.config.host, self.config.port);
        let timeout = Duration::from_millis(self.config.timeout_ms);
        let mut stream = TcpStream::connect(&address).map_err(|err| {
            format!("USER_ACTION_REQUIRED: local Ollama unavailable at {address}: {err}")
        })?;
        stream
            .set_read_timeout(Some(timeout))
            .map_err(|err| err.to_string())?;
        stream
            .set_write_timeout(Some(timeout))
            .map_err(|err| err.to_string())?;

        let http = format!(
            "POST /api/embed HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(http.as_bytes())
            .map_err(|err| format!("local Ollama request failed: {err}"))?;
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .map_err(|err| format!("local Ollama response failed: {err}"))?;
        parse_ollama_embedding_response(&response)
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
struct OllamaEmbedResponse {
    model: Option<String>,
    embeddings: Vec<Vec<f32>>,
}

#[derive(Debug, Deserialize)]
struct OllamaErrorResponse {
    error: String,
}

fn parse_ollama_embedding_response(response: &str) -> Result<Vec<f32>, String> {
    let (header, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "local Ollama response missing HTTP body".to_string())?;
    let body = if header
        .lines()
        .any(|line| line.eq_ignore_ascii_case("Transfer-Encoding: chunked"))
    {
        decode_chunked_http_body(body)?
    } else {
        body.to_string()
    };
    if !header.starts_with("HTTP/1.1 200") && !header.starts_with("HTTP/1.0 200") {
        let message = serde_json::from_str::<OllamaErrorResponse>(&body)
            .map(|err| err.error)
            .unwrap_or_else(|_| body.trim().to_string());
        return Err(format!(
            "USER_ACTION_REQUIRED: local Ollama embedding request failed: {message}"
        ));
    }
    let response =
        serde_json::from_str::<OllamaEmbedResponse>(&body).map_err(|err| err.to_string())?;
    if response
        .model
        .as_deref()
        .is_some_and(|model| model.trim().is_empty())
    {
        return Err("local Ollama returned an empty model label".to_string());
    }
    let embedding = response
        .embeddings
        .into_iter()
        .next()
        .ok_or_else(|| "local Ollama returned no embedding vectors".to_string())?;
    if embedding.is_empty()
        || embedding.len() > CA26_MAX_RAW_EMBEDDING_DIMS
        || embedding.iter().any(|value| !value.is_finite())
    {
        return Err("local Ollama returned invalid embedding values".to_string());
    }
    Ok(embedding)
}

fn decode_chunked_http_body(body: &str) -> Result<String, String> {
    let mut remaining = body;
    let mut decoded = String::new();
    loop {
        let (size_line, rest) = remaining
            .split_once("\r\n")
            .ok_or_else(|| "chunked Ollama response missing chunk size".to_string())?;
        let size_text = size_line.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| "chunked Ollama response has invalid chunk size".to_string())?;
        if size == 0 {
            return Ok(decoded);
        }
        if rest.len() < size + 2 {
            return Err("chunked Ollama response ended inside a chunk".to_string());
        }
        decoded.push_str(&rest[..size]);
        let trailer = &rest[size..];
        if !trailer.starts_with("\r\n") {
            return Err("chunked Ollama response missing chunk terminator".to_string());
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
                    "runtime_backend":"ollama-localhost-gguf",
                    "expected_local_path":"models/local/qwen3-embedding-0.6b-gguf/Qwen3-Embedding-0.6B-Q8_0.gguf",
                    "ollama_model":"alife-qwen3-embedding-0.6b",
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
    fn unavailable_ollama_model_is_user_action_required_not_fake_output() {
        let provider = LocalOllamaEmbeddingProvider::new(LocalOllamaEmbeddingConfig {
            port: 9,
            timeout_ms: 1_000,
            ..LocalOllamaEmbeddingConfig::default()
        })
        .unwrap();
        let err = provider.embed_text("teacher token food").unwrap_err();
        assert!(err.contains("USER_ACTION_REQUIRED"));
    }

    #[test]
    fn chunked_ollama_response_decodes_before_json_parse() {
        let response = concat!(
            "HTTP/1.1 200 OK\r\n",
            "Transfer-Encoding: chunked\r\n",
            "\r\n",
            "26\r\n",
            "{\"model\":\"m\",\"embeddings\":[[1.0,2.0]]}\r\n",
            "0\r\n",
            "\r\n"
        );
        let embedding = parse_ollama_embedding_response(response).unwrap();
        assert_eq!(embedding, vec![1.0, 2.0]);
    }
}
