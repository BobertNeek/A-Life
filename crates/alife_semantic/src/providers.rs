//! v0 scaffold: conversion orchestration for optional semantic/Gaussian adapters.

use alife_core::{GaussianContextRef, ScaffoldContractError, SemanticContextRef, Vec3f};

use crate::{
    build_gaussian_context, build_semantic_context, EgocentricBinGrid, EgocentricBinHasher,
    GaussianClusterObservation, SemanticCodeDescriptor, SemanticConceptBinding,
    SemanticProviderCapabilityManifest,
};

use crate::{
    MAX_GAUSSIAN_CONTEXT_CLUSTERS, MAX_SEMANTIC_CODE_COUNT, MAX_SEMANTIC_CONTEXT_BINDINGS,
};

/// A small, explicit request object for optional context synthesis.
#[derive(Debug, Clone)]
pub struct SemanticContextRequest {
    pub observer_offset: Vec3f,
    pub gaussian_observations: Vec<GaussianClusterObservation>,
    pub gaussian_confidence: f32,
    pub semantic_bindings: Vec<SemanticConceptBinding>,
    pub semantic_descriptors: Vec<SemanticCodeDescriptor>,
    pub semantic_confidence: f32,
}

impl SemanticContextRequest {
    pub const fn new(observer_offset: Vec3f) -> Self {
        Self {
            observer_offset,
            gaussian_observations: Vec::new(),
            gaussian_confidence: 1.0,
            semantic_bindings: Vec::new(),
            semantic_descriptors: Vec::new(),
            semantic_confidence: 1.0,
        }
    }

    pub fn with_gaussian_observation(
        mut self,
        cluster_id: alife_core::GaussianClusterId,
        salience: f32,
        distance_meters: f32,
        offset: Vec3f,
    ) -> Self {
        if self.gaussian_observations.len() < MAX_GAUSSIAN_CONTEXT_CLUSTERS {
            self.gaussian_observations.push(GaussianClusterObservation {
                cluster_id,
                salience,
                distance_meters,
                egocentric_offset: offset,
            });
        }
        self
    }

    pub fn with_semantic_binding(mut self, binding: SemanticConceptBinding) -> Self {
        if self.semantic_bindings.len() < MAX_SEMANTIC_CONTEXT_BINDINGS {
            self.semantic_bindings.push(binding);
        }
        self
    }

    pub fn with_semantic_descriptor(mut self, descriptor: SemanticCodeDescriptor) -> Self {
        if self.semantic_descriptors.len() < MAX_SEMANTIC_CODE_COUNT {
            self.semantic_descriptors.push(descriptor);
        }
        self
    }

    #[allow(dead_code)]
    fn gaussian_bin_hash(&self) -> u64 {
        EgocentricBinHasher::new().hash(self.observer_offset, EgocentricBinGrid::default())
    }
}

/// The optional semantic/Gaussian context pair returned by adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticContextBundle {
    pub gaussian_context: Option<GaussianContextRef>,
    pub semantic_context: Option<SemanticContextRef>,
}

/// Optional adapter contracts are explicit; missing sources must never fail.
pub trait SemanticContextProvider {
    fn capability_manifest(&self) -> SemanticProviderCapabilityManifest;

    fn build_context_bundle(
        &self,
        request: &SemanticContextRequest,
    ) -> Result<SemanticContextBundle, ScaffoldContractError>;
}

/// Keep adapters small and deterministic; this helper performs full conversion.
#[allow(dead_code)]
pub(crate) fn synthesize_context_bundle(
    request: &SemanticContextRequest,
) -> Result<SemanticContextBundle, ScaffoldContractError> {
    let gaussian_context = build_gaussian_context(
        &request.gaussian_observations,
        request.gaussian_confidence,
        request.gaussian_bin_hash(),
    )?;
    let semantic_context = build_semantic_context(
        &request.semantic_bindings,
        &request.semantic_descriptors,
        request.semantic_confidence,
    )?;

    Ok(SemanticContextBundle {
        gaussian_context,
        semantic_context,
    })
}
