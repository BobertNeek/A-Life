//! v0 scaffold: internal private semantic-prior contracts.

use alife_core::{ScaffoldContractError, SemanticPriorProvider};
use serde::{Deserialize, Serialize};

#[cfg(feature = "fake-semantic-provider")]
mod fake;
#[cfg(feature = "gaussian-adapter")]
mod gaussian;
#[cfg(feature = "local-ollama")]
mod local_ollama;
#[cfg(feature = "local-ollama")]
mod local_slm_prior;
#[cfg(feature = "gaussian-adapter")]
mod providers;
#[cfg(feature = "gaussian-adapter")]
mod semantic;

#[derive(Debug, Default)]
pub struct NoopSemanticPriorProvider;

impl SemanticPriorProvider for NoopSemanticPriorProvider {
    fn provider_name(&self) -> &'static str {
        "noop-semantic-prior"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticBoundaryManifest {
    pub private_prior: bool,
    pub can_issue_actions: bool,
    pub can_rewrite_weights: bool,
}

impl SemanticBoundaryManifest {
    pub const INTERNAL_PRIOR: Self = Self {
        private_prior: true,
        can_issue_actions: false,
        can_rewrite_weights: false,
    };
}

pub const G11_SEMANTIC_PROVIDER_SCHEMA: &str = "alife.g11.semantic_provider.v1";
pub const G11_SEMANTIC_PROVIDER_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticProviderKind {
    Disabled,
    FakeLocalTable,
    ExternalExtension,
    LocalOllamaEmbedding,
}

impl SemanticProviderKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::FakeLocalTable => "fake-local-table",
            Self::ExternalExtension => "external-extension",
            Self::LocalOllamaEmbedding => "local-ollama-embedding",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticProviderConfig {
    pub schema: String,
    pub schema_version: u16,
    pub provider_id: String,
    pub provider_kind: SemanticProviderKind,
    pub required: bool,
    pub max_display_entries: usize,
}

impl Default for SemanticProviderConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

impl SemanticProviderConfig {
    pub fn disabled() -> Self {
        Self {
            schema: G11_SEMANTIC_PROVIDER_SCHEMA.to_string(),
            schema_version: G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
            provider_id: "disabled".to_string(),
            provider_kind: SemanticProviderKind::Disabled,
            required: false,
            max_display_entries: 0,
        }
    }

    pub fn fake_local_table() -> Self {
        Self {
            schema: G11_SEMANTIC_PROVIDER_SCHEMA.to_string(),
            schema_version: G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
            provider_id: "fake-local-table".to_string(),
            provider_kind: SemanticProviderKind::FakeLocalTable,
            required: false,
            max_display_entries: 8,
        }
    }

    pub fn external_extension(provider_id: impl Into<String>) -> Self {
        Self {
            schema: G11_SEMANTIC_PROVIDER_SCHEMA.to_string(),
            schema_version: G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
            provider_id: provider_id.into(),
            provider_kind: SemanticProviderKind::ExternalExtension,
            required: false,
            max_display_entries: 8,
        }
    }

    pub fn local_ollama_embedding(provider_id: impl Into<String>) -> Self {
        Self {
            schema: G11_SEMANTIC_PROVIDER_SCHEMA.to_string(),
            schema_version: G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
            provider_id: provider_id.into(),
            provider_kind: SemanticProviderKind::LocalOllamaEmbedding,
            required: false,
            max_display_entries: 8,
        }
    }

    pub fn from_json_str(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G11_SEMANTIC_PROVIDER_SCHEMA
            || self.schema_version != G11_SEMANTIC_PROVIDER_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.provider_id.trim().is_empty() || self.max_display_entries > 32 {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.required && matches!(self.provider_kind, SemanticProviderKind::Disabled) {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        if matches!(self.provider_kind, SemanticProviderKind::Disabled)
            && (self.provider_id != "disabled" || self.max_display_entries != 0)
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticProviderCapabilityManifest {
    pub schema: String,
    pub schema_version: u16,
    pub provider_id: String,
    pub provider_kind: SemanticProviderKind,
    pub private_prior: bool,
    pub optional_runtime_dependency: bool,
    pub available: bool,
    pub bounded_context: bool,
    pub max_gaussian_clusters: usize,
    pub max_semantic_codes: usize,
    pub max_semantic_bindings: usize,
    pub can_issue_actions: bool,
    pub can_rewrite_weights: bool,
    pub requires_external_model: bool,
    pub failure_is_nonfatal: bool,
}

impl SemanticProviderCapabilityManifest {
    pub fn disabled() -> Self {
        Self::new(
            "disabled",
            SemanticProviderKind::Disabled,
            false,
            false,
            false,
        )
    }

    pub fn fake_local_table() -> Self {
        Self::new(
            "fake-local-table",
            SemanticProviderKind::FakeLocalTable,
            true,
            false,
            false,
        )
    }

    pub fn external_extension(provider_id: impl Into<String>, available: bool) -> Self {
        Self::new(
            provider_id,
            SemanticProviderKind::ExternalExtension,
            available,
            true,
            true,
        )
    }

    pub fn local_ollama_embedding(provider_id: impl Into<String>, available: bool) -> Self {
        Self::new(
            provider_id,
            SemanticProviderKind::LocalOllamaEmbedding,
            available,
            true,
            true,
        )
    }

    fn new(
        provider_id: impl Into<String>,
        provider_kind: SemanticProviderKind,
        available: bool,
        optional_runtime_dependency: bool,
        requires_external_model: bool,
    ) -> Self {
        Self {
            schema: G11_SEMANTIC_PROVIDER_SCHEMA.to_string(),
            schema_version: G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
            provider_id: provider_id.into(),
            provider_kind,
            private_prior: true,
            optional_runtime_dependency,
            available,
            bounded_context: true,
            max_gaussian_clusters: max_gaussian_context_clusters(),
            max_semantic_codes: max_semantic_code_count(),
            max_semantic_bindings: max_semantic_context_bindings(),
            can_issue_actions: false,
            can_rewrite_weights: false,
            requires_external_model,
            failure_is_nonfatal: true,
        }
    }

    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.schema != G11_SEMANTIC_PROVIDER_SCHEMA
            || self.schema_version != G11_SEMANTIC_PROVIDER_SCHEMA_VERSION
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.provider_id.trim().is_empty()
            || !self.private_prior
            || !self.bounded_context
            || self.can_issue_actions
            || self.can_rewrite_weights
            || !self.failure_is_nonfatal
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        if self.max_gaussian_clusters == 0
            || self.max_semantic_codes == 0
            || self.max_semantic_bindings == 0
        {
            return Err(ScaffoldContractError::ScalarOutOfRange);
        }
        Ok(())
    }
}

const fn max_gaussian_context_clusters() -> usize {
    #[cfg(feature = "gaussian-adapter")]
    {
        crate::MAX_GAUSSIAN_CONTEXT_CLUSTERS
    }
    #[cfg(not(feature = "gaussian-adapter"))]
    {
        8
    }
}

const fn max_semantic_code_count() -> usize {
    #[cfg(feature = "gaussian-adapter")]
    {
        crate::MAX_SEMANTIC_CODE_COUNT
    }
    #[cfg(not(feature = "gaussian-adapter"))]
    {
        12
    }
}

const fn max_semantic_context_bindings() -> usize {
    #[cfg(feature = "gaussian-adapter")]
    {
        crate::MAX_SEMANTIC_CONTEXT_BINDINGS
    }
    #[cfg(not(feature = "gaussian-adapter"))]
    {
        12
    }
}

#[cfg(feature = "gaussian-adapter")]
pub use gaussian::{
    build_gaussian_context, EgocentricBinGrid, EgocentricBinHasher, GaussianClusterObservation,
    MAX_GAUSSIAN_CONTEXT_CLUSTERS,
};

#[cfg(feature = "gaussian-adapter")]
pub use providers::{SemanticContextBundle, SemanticContextProvider, SemanticContextRequest};

#[cfg(feature = "gaussian-adapter")]
pub use semantic::{
    build_semantic_context, SemanticCodeDescriptor, SemanticConceptBinding,
    MAX_SEMANTIC_CODE_COUNT, MAX_SEMANTIC_CONTEXT_BINDINGS,
};

#[cfg(feature = "fake-semantic-provider")]
pub use fake::FakeSemanticProvider;

#[cfg(feature = "local-ollama")]
pub use local_ollama::{
    project_embedding_to_i8, BoundedSemanticEmbedding, LocalOllamaEmbeddingConfig,
    LocalOllamaEmbeddingProvider, LocalSemanticModelEntry, LocalSemanticModelManifest,
    CA26_DEFAULT_OLLAMA_MODEL, CA26_EMBEDDING_PROJECTION_DIMS, CA26_LOCAL_MODEL_MANIFEST_SCHEMA,
    CA26_LOCAL_MODEL_MANIFEST_SCHEMA_VERSION, CA26_LOCAL_SEMANTIC_PROVIDER_ID,
};

#[cfg(feature = "local-ollama")]
pub use local_slm_prior::{
    parse_slm_prior_json, LocalOllamaSlmPriorConfig, LocalOllamaSlmPriorProvider,
    LocalSlmPriorAsyncQueue, LocalSlmPriorOutput, LocalSlmPriorQueue, LocalSlmPriorRequest,
    SlmLexiconAssociation, CA27_DEFAULT_OLLAMA_MODEL, CA27_LOCAL_SLM_PRIOR_ID,
    CA27_MAX_PERCEPTION_TAGS, CA27_MAX_SALIENCE_LABELS, CA27_SLM_PRIOR_OUTPUT_SCHEMA,
    CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION,
};
