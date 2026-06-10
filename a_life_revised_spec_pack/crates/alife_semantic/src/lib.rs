//! v0 scaffold: internal private semantic-prior contracts.

use alife_core::SemanticPriorProvider;

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
