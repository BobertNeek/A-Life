//! v0 scaffold: optional deterministic fake semantic provider for tests/headless use.

use alife_core::ScaffoldContractError;

use crate::providers::{
    synthesize_context_bundle, SemanticContextProvider, SemanticContextRequest,
};
use crate::SemanticProviderCapabilityManifest;

/// Fake provider that returns deterministic semantic/Gaussian contexts for
/// tests and headless scenarios.
#[derive(Debug, Clone)]
pub struct FakeSemanticProvider;

impl SemanticContextProvider for FakeSemanticProvider {
    fn capability_manifest(&self) -> SemanticProviderCapabilityManifest {
        SemanticProviderCapabilityManifest::fake_local_table()
    }

    fn build_context_bundle(
        &self,
        request: &SemanticContextRequest,
    ) -> Result<crate::providers::SemanticContextBundle, ScaffoldContractError> {
        synthesize_context_bundle(request)
    }
}

impl FakeSemanticProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FakeSemanticProvider {
    fn default() -> Self {
        Self::new()
    }
}
