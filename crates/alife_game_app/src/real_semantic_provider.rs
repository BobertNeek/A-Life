//! CA26 real local semantic provider smoke.

use std::path::Path;

use crate::prelude::*;
use crate::{
    GameAppShellError, CA26_REAL_SEMANTIC_PROVIDER_SCHEMA,
    CA26_REAL_SEMANTIC_PROVIDER_SCHEMA_VERSION,
};

pub const CA26_DEFAULT_MODEL_MANIFEST: &str =
    "../../examples/model_manifests/local_semantic_models.json";
pub const CA26_SMOKE_INPUT: &str = "teacher token food berry short lesson context";

#[derive(Debug, Clone, PartialEq)]
pub struct RealSemanticProviderSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub model_manifest_schema: &'static str,
    pub model_manifest_schema_version: u16,
    pub manifest_path: String,
    pub repo_id: String,
    pub model_role: String,
    pub license: String,
    pub runtime_backend: String,
    pub expected_local_path: String,
    pub ollama_model: String,
    pub sha256: String,
    pub downloaded_locally: bool,
    pub inference_smoke_passed: bool,
    pub local_runtime: String,
    pub input_chars: usize,
    pub raw_embedding_dims: usize,
    pub projected_embedding_dims: usize,
    pub semantic_code_count: usize,
    pub semantic_context_visible: bool,
    pub context_vectors_bounded: bool,
    pub timeout_ms: u64,
    pub unavailable_is_user_action_required: bool,
    pub fake_model_output_used: bool,
    pub can_issue_actions: bool,
    pub can_rewrite_weights: bool,
    pub hidden_vector_injection: bool,
    pub notes: Vec<String>,
}

impl RealSemanticProviderSmokeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA26_REAL_SEMANTIC_PROVIDER_SCHEMA
            || self.schema_version != CA26_REAL_SEMANTIC_PROVIDER_SCHEMA_VERSION
            || self.model_manifest_schema != CA26_LOCAL_MODEL_MANIFEST_SCHEMA
            || self.model_manifest_schema_version != CA26_LOCAL_MODEL_MANIFEST_SCHEMA_VERSION
            || self.manifest_path.is_empty()
            || self.repo_id != "Qwen/Qwen3-Embedding-0.6B-GGUF"
            || self.model_role != "semantic_embedding_provider"
            || self.license != "apache-2.0"
            || self.runtime_backend != "ollama-localhost-gguf"
            || self.expected_local_path.contains("Entity(")
            || self.ollama_model.trim().is_empty()
            || self.sha256.len() != 64
            || !self.downloaded_locally
            || !self.inference_smoke_passed
            || self.local_runtime != "ollama-localhost"
            || self.input_chars == 0
            || self.raw_embedding_dims == 0
            || self.raw_embedding_dims > 8_192
            || self.projected_embedding_dims != CA26_EMBEDDING_PROJECTION_DIMS
            || self.semantic_code_count == 0
            || !self.semantic_context_visible
            || !self.context_vectors_bounded
            || self.timeout_ms == 0
            || !self.unavailable_is_user_action_required
            || self.fake_model_output_used
            || self.can_issue_actions
            || self.can_rewrite_weights
            || self.hidden_vector_injection
            || self.notes.is_empty()
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.repo_id,
            self.runtime_backend,
            self.ollama_model,
            self.raw_embedding_dims,
            self.projected_embedding_dims,
            self.semantic_code_count,
            self.inference_smoke_passed,
            self.can_issue_actions,
            self.can_rewrite_weights
        )
    }
}

pub fn default_ca26_model_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(CA26_DEFAULT_MODEL_MANIFEST)
}

pub fn run_real_semantic_provider_smoke(
) -> Result<RealSemanticProviderSmokeSummary, GameAppShellError> {
    run_real_semantic_provider_smoke_with_manifest(default_ca26_model_manifest_path())
}

pub fn run_real_semantic_provider_smoke_with_manifest(
    path: impl AsRef<Path>,
) -> Result<RealSemanticProviderSmokeSummary, GameAppShellError> {
    let path = path.as_ref();
    let manifest = LocalSemanticModelManifest::from_json_file(path).map_err(|_| {
        GameAppShellError::VisibleWorldMismatch {
            message: "CA26 local model manifest invalid",
        }
    })?;
    manifest.validate().map_err(GameAppShellError::Core)?;
    let model =
        manifest
            .semantic_embedding_model()
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "CA26 model manifest must contain semantic_embedding_provider",
            })?;

    let config = LocalOllamaEmbeddingConfig {
        model: model.ollama_model.clone(),
        ..LocalOllamaEmbeddingConfig::default()
    };
    let timeout_ms = config.timeout_ms;
    let provider = LocalOllamaEmbeddingProvider::new(config).map_err(GameAppShellError::Core)?;
    let capability = provider.capability_manifest(true);
    capability.validate().map_err(GameAppShellError::Core)?;
    if capability.can_issue_actions || capability.can_rewrite_weights {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "CA26 semantic provider boundary may not issue actions or rewrite weights",
        });
    }

    let (embedding, context) = provider
        .build_bounded_context(CA26_SMOKE_INPUT)
        .map_err(|_| GameAppShellError::VisibleWorldMismatch {
            message: "CA26 real local inference failed; USER_ACTION_REQUIRED",
        })?;
    context
        .validate_contract()
        .map_err(GameAppShellError::Core)?;
    validate_bounded_embedding(&embedding)?;

    let summary = RealSemanticProviderSmokeSummary {
        schema: CA26_REAL_SEMANTIC_PROVIDER_SCHEMA,
        schema_version: CA26_REAL_SEMANTIC_PROVIDER_SCHEMA_VERSION,
        model_manifest_schema: CA26_LOCAL_MODEL_MANIFEST_SCHEMA,
        model_manifest_schema_version: CA26_LOCAL_MODEL_MANIFEST_SCHEMA_VERSION,
        manifest_path: path.display().to_string(),
        repo_id: model.repo_id.clone(),
        model_role: model.model_role.clone(),
        license: model.license.clone(),
        runtime_backend: model.runtime_backend.clone(),
        expected_local_path: model.expected_local_path.clone(),
        ollama_model: model.ollama_model.clone(),
        sha256: model.sha256.clone(),
        downloaded_locally: model.downloaded_locally,
        inference_smoke_passed: true,
        local_runtime: "ollama-localhost".to_string(),
        input_chars: CA26_SMOKE_INPUT.chars().count(),
        raw_embedding_dims: embedding.raw_dims,
        projected_embedding_dims: CA26_EMBEDDING_PROJECTION_DIMS,
        semantic_code_count: context.compressed_codes.len(),
        semantic_context_visible: !context.compressed_codes.is_empty(),
        context_vectors_bounded: context.compressed_codes.len() <= 12,
        timeout_ms,
        unavailable_is_user_action_required: true,
        fake_model_output_used: false,
        can_issue_actions: capability.can_issue_actions,
        can_rewrite_weights: capability.can_rewrite_weights,
        hidden_vector_injection: false,
        notes: vec![
            "real local Qwen3 embedding via Ollama localhost".to_string(),
            "bounded projection enters only semantic context metadata".to_string(),
            "semantic provider cannot act, select motors, or rewrite weights".to_string(),
        ],
    };
    summary.validate()?;
    Ok(summary)
}

fn validate_bounded_embedding(
    embedding: &BoundedSemanticEmbedding,
) -> Result<(), GameAppShellError> {
    embedding.validate().map_err(GameAppShellError::Core)?;
    if embedding.can_issue_actions || embedding.can_rewrite_weights {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "CA26 embedding provider produced forbidden authority flags",
        });
    }
    Ok(())
}
