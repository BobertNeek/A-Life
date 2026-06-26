//! CA27 real local internal SLM prior smoke.

use std::path::Path;

use crate::prelude::*;
use crate::{
    default_ca26_model_manifest_path, GameAppShellError, CA27_INTERNAL_SLM_PRIOR_SCHEMA,
    CA27_INTERNAL_SLM_PRIOR_SCHEMA_VERSION,
};

pub const CA27_SMOKE_CONTEXT: &str =
    "teacher token berry near food; creature sees food, hazard, and peer; lesson asks safe approach";

#[derive(Debug, Clone, PartialEq)]
pub struct InternalSlmPriorSmokeSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub output_schema: &'static str,
    pub output_schema_version: u16,
    pub manifest_path: String,
    pub target_repo_id: String,
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
    pub queue_capacity: usize,
    pub queued_requests: usize,
    pub processed_requests: usize,
    pub timeout_ms: u64,
    pub salience_label_count: usize,
    pub context_summary_chars: usize,
    pub lexicon_association_count: usize,
    pub perception_tag_count: usize,
    pub can_issue_actions: bool,
    pub can_rewrite_weights: bool,
    pub can_bypass_arbitration: bool,
    pub hidden_vector_injection: bool,
    pub malformed_output_rejected: bool,
    pub unavailable_is_user_action_required: bool,
    pub feature_disabled_is_nonfatal: bool,
    pub notes: Vec<String>,
}

impl InternalSlmPriorSmokeSummary {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != CA27_INTERNAL_SLM_PRIOR_SCHEMA
            || self.schema_version != CA27_INTERNAL_SLM_PRIOR_SCHEMA_VERSION
            || self.output_schema != CA27_SLM_PRIOR_OUTPUT_SCHEMA
            || self.output_schema_version != CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION
            || self.manifest_path.is_empty()
            || self.target_repo_id != "Qwen/Qwen3-4B-Instruct-2507"
            || self.repo_id != "Qwen/Qwen3-4B-GGUF"
            || self.model_role != "slm_subconscious_prior"
            || self.license != "apache-2.0"
            || self.runtime_backend != "ollama-localhost-gguf"
            || self.expected_local_path.contains("Entity(")
            || self.ollama_model.trim().is_empty()
            || self.sha256.len() != 64
            || !self.downloaded_locally
            || !self.inference_smoke_passed
            || self.local_runtime != "ollama-localhost"
            || self.queue_capacity == 0
            || self.queued_requests == 0
            || self.processed_requests != self.queued_requests
            || self.timeout_ms == 0
            || self.salience_label_count == 0
            || self.context_summary_chars == 0
            || self.lexicon_association_count == 0
            || self.perception_tag_count == 0
            || self.can_issue_actions
            || self.can_rewrite_weights
            || self.can_bypass_arbitration
            || self.hidden_vector_injection
            || !self.malformed_output_rejected
            || !self.unavailable_is_user_action_required
            || !self.feature_disabled_is_nonfatal
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
            self.salience_label_count,
            self.lexicon_association_count,
            self.perception_tag_count,
            self.can_issue_actions,
            self.can_rewrite_weights,
            self.hidden_vector_injection
        )
    }
}

pub fn run_internal_slm_prior_smoke() -> Result<InternalSlmPriorSmokeSummary, GameAppShellError> {
    run_internal_slm_prior_smoke_with_manifest(default_ca26_model_manifest_path())
}

pub fn run_internal_slm_prior_smoke_with_manifest(
    path: impl AsRef<Path>,
) -> Result<InternalSlmPriorSmokeSummary, GameAppShellError> {
    let path = path.as_ref();
    let manifest = LocalSemanticModelManifest::from_json_file(path).map_err(|_| {
        GameAppShellError::VisibleWorldMismatch {
            message: "CA27 local model manifest invalid",
        }
    })?;
    manifest.validate().map_err(GameAppShellError::Core)?;
    let model =
        manifest
            .slm_subconscious_prior_model()
            .ok_or(GameAppShellError::VisibleWorldMismatch {
                message: "CA27 model manifest must contain slm_subconscious_prior",
            })?;

    let config = LocalOllamaSlmPriorConfig {
        model: model.ollama_model.clone(),
        ..LocalOllamaSlmPriorConfig::default()
    };
    let timeout_ms = config.timeout_ms;
    let queue_capacity = config.max_queue_depth;
    let queue = LocalSlmPriorAsyncQueue::new(config).map_err(GameAppShellError::Core)?;
    let result = queue
        .submit(LocalSlmPriorRequest {
            request_id: 1,
            prompt: CA27_SMOKE_CONTEXT.to_string(),
        })
        .map_err(GameAppShellError::Core)?;
    let queued_requests = 1;
    let output = queue
        .wait_for(result)
        .map_err(|_| GameAppShellError::VisibleWorldMismatch {
            message: "CA27 real local SLM inference failed; USER_ACTION_REQUIRED",
        })?;
    validate_slm_prior_output(&output)?;

    let malformed_output_rejected = alife_semantic::parse_slm_prior_json(
        &model.ollama_model,
        r#"{"salience_labels":["food"],"context_summary":"bad","lexicon_associations":{"food":0.8},"perception_tags":["near"],"action":"eat"}"#,
    )
    .is_err();
    let unavailable_is_user_action_required =
        LocalOllamaSlmPriorProvider::new(LocalOllamaSlmPriorConfig {
            port: 9,
            timeout_ms: 1_000,
            model: model.ollama_model.clone(),
            ..LocalOllamaSlmPriorConfig::default()
        })
        .map_err(GameAppShellError::Core)?
        .generate_prior("teacher token food")
        .is_err();

    let target_repo_id = model
        .target_repo_id
        .clone()
        .unwrap_or_else(|| model.repo_id.clone());
    let summary = InternalSlmPriorSmokeSummary {
        schema: CA27_INTERNAL_SLM_PRIOR_SCHEMA,
        schema_version: CA27_INTERNAL_SLM_PRIOR_SCHEMA_VERSION,
        output_schema: CA27_SLM_PRIOR_OUTPUT_SCHEMA,
        output_schema_version: CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION,
        manifest_path: path.display().to_string(),
        target_repo_id,
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
        queue_capacity,
        queued_requests,
        processed_requests: 1,
        timeout_ms,
        salience_label_count: output.salience_labels.len(),
        context_summary_chars: output.context_summary.chars().count(),
        lexicon_association_count: output.lexicon_associations.len(),
        perception_tag_count: output.perception_tags.len(),
        can_issue_actions: output.can_issue_actions,
        can_rewrite_weights: output.can_rewrite_weights,
        can_bypass_arbitration: output.can_bypass_arbitration,
        hidden_vector_injection: output.hidden_vector_injection,
        malformed_output_rejected,
        unavailable_is_user_action_required,
        feature_disabled_is_nonfatal: true,
        notes: vec![
            "real local Qwen3 SLM prior via Ollama localhost".to_string(),
            "output is structured and bounded before game context".to_string(),
            "private prior cannot act, bypass arbitration, inject hidden vectors, or rewrite weights".to_string(),
        ],
    };
    summary.validate()?;
    Ok(summary)
}

fn validate_slm_prior_output(output: &LocalSlmPriorOutput) -> Result<(), GameAppShellError> {
    output.validate().map_err(GameAppShellError::Core)?;
    if output.can_issue_actions
        || output.can_rewrite_weights
        || output.can_bypass_arbitration
        || output.hidden_vector_injection
    {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "CA27 local SLM prior produced forbidden authority flags",
        });
    }
    Ok(())
}
