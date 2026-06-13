//! v0 scaffold: internal private semantic-prior contracts.

use alife_core::SemanticPriorProvider;

#[cfg(feature = "fake-semantic-provider")]
mod fake;
#[cfg(feature = "gaussian-adapter")]
mod gaussian;
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
