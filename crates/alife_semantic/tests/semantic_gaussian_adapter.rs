#[cfg(any(feature = "gaussian-adapter", feature = "fake-semantic-provider"))]
use alife_core::ScaffoldContractError;
#[cfg(feature = "gaussian-adapter")]
use alife_core::{ConceptCellId, GaussianClusterId};
use alife_semantic::SemanticBoundaryManifest;

#[cfg(feature = "gaussian-adapter")]
use alife_semantic::{
    build_gaussian_context, build_semantic_context, EgocentricBinGrid, EgocentricBinHasher,
    GaussianClusterObservation, SemanticCodeDescriptor, SemanticConceptBinding,
    SemanticContextRequest, MAX_GAUSSIAN_CONTEXT_CLUSTERS, MAX_SEMANTIC_CODE_COUNT,
    MAX_SEMANTIC_CONTEXT_BINDINGS,
};

#[cfg(feature = "fake-semantic-provider")]
use alife_semantic::{FakeSemanticProvider, SemanticContextProvider};

#[test]
fn missing_semantic_provider_stays_nonfatal() {
    let manifest = SemanticBoundaryManifest::INTERNAL_PRIOR;

    assert!(manifest.private_prior);
    assert!(!manifest.can_issue_actions);
    assert!(!manifest.can_rewrite_weights);
}

#[cfg(feature = "gaussian-adapter")]
#[test]
fn gaussian_context_conversion_sorts_and_caps() -> Result<(), ScaffoldContractError> {
    let observations = (0..(MAX_GAUSSIAN_CONTEXT_CLUSTERS + 2))
        .map(|idx| GaussianClusterObservation {
            cluster_id: GaussianClusterId(100 + idx as u64),
            salience: 1.0 - (idx as f32) * 0.03,
            distance_meters: 1.5 + idx as f32,
            egocentric_offset: alife_core::Vec3f::new(idx as f32, 0.0, 1.0),
        })
        .collect::<Vec<_>>();

    let context = build_gaussian_context(
        &observations,
        0.75,
        EgocentricBinHasher::new().hash(
            alife_core::Vec3f::new(1.0, 0.0, 0.0),
            EgocentricBinGrid::default(),
        ),
    )?
    .expect("context should be present when observations are nonzero");

    assert_eq!(context.clusters.len(), MAX_GAUSSIAN_CONTEXT_CLUSTERS);
    assert!(context
        .clusters
        .first()
        .map(|entry| entry.cluster_id == GaussianClusterId(100))
        .unwrap_or(false));
    assert!(context.confidence.raw() > 0.0);

    Ok(())
}

#[cfg(feature = "gaussian-adapter")]
#[test]
fn gaussian_context_builder_validates_bounded_confidence() {
    let result = build_gaussian_context(
        &[GaussianClusterObservation {
            cluster_id: GaussianClusterId(11),
            salience: 0.5,
            distance_meters: 0.5,
            egocentric_offset: alife_core::Vec3f::new(0.0, 0.0, 0.0),
        }],
        1.5,
        123,
    );
    assert!(result.is_err());
}

#[cfg(feature = "gaussian-adapter")]
#[test]
fn gaussian_context_absent_with_empty_or_zero_salience() -> Result<(), ScaffoldContractError> {
    let none_from_empty = build_gaussian_context(&[], 0.5, 0)?;
    assert!(none_from_empty.is_none());

    let none_from_zero = build_gaussian_context(
        &[GaussianClusterObservation {
            cluster_id: GaussianClusterId(11),
            salience: 0.0,
            distance_meters: 1.0,
            egocentric_offset: alife_core::Vec3f::new(0.0, 0.0, 1.0),
        }],
        0.5,
        0,
    )?;
    assert!(none_from_zero.is_none());
    Ok(())
}

#[cfg(feature = "gaussian-adapter")]
#[test]
fn semantic_context_sorts_and_caps() -> Result<(), ScaffoldContractError> {
    let mut bindings = vec![];
    let mut descriptors = vec![];

    for idx in 0..(MAX_SEMANTIC_CONTEXT_BINDINGS + 2) {
        bindings.push(SemanticConceptBinding {
            concept_id: ConceptCellId(10 + idx as u64),
            salience: 0.95 - idx as f32 * 0.03,
        });
    }
    for idx in 0..(MAX_SEMANTIC_CODE_COUNT + 3) {
        let mut descriptor = [0_i8; 32];
        descriptor[0] = idx as i8;
        descriptors.push(SemanticCodeDescriptor {
            codebook_id: 2,
            descriptor,
            salience: 0.7 - (idx as f32 * 0.01),
        });
    }

    let context = build_semantic_context(&bindings, &descriptors, 0.88)?
        .expect("semantic context should be generated from provided bindings");

    assert_eq!(context.salience.len(), MAX_SEMANTIC_CONTEXT_BINDINGS);
    assert_eq!(context.compressed_codes.len(), MAX_SEMANTIC_CODE_COUNT);
    assert!(context
        .salience
        .first()
        .map(|entry| entry.concept_id == ConceptCellId(10))
        .unwrap_or(false));
    assert!(context
        .compressed_codes
        .first()
        .map(|entry| entry.codebook_id == 2)
        .unwrap_or(false));
    Ok(())
}

#[cfg(feature = "gaussian-adapter")]
#[test]
fn semantic_context_rejects_invalid_codebook_id() -> Result<(), ScaffoldContractError> {
    let result = build_semantic_context(
        &[],
        &[SemanticCodeDescriptor {
            codebook_id: 0,
            descriptor: [1_i8; 32],
            salience: 0.8,
        }],
        0.9,
    );
    assert!(result.is_err());
    Ok(())
}

#[cfg(feature = "fake-semantic-provider")]
#[test]
fn fake_provider_generates_bundle_without_renderer() -> Result<(), ScaffoldContractError> {
    let request = SemanticContextRequest::new(alife_core::Vec3f::new(0.5, 0.2, 0.0))
        .with_gaussian_observation(
            GaussianClusterId(123),
            0.8,
            2.4,
            alife_core::Vec3f::new(0.5, 0.0, 1.0),
        )
        .with_semantic_binding(SemanticConceptBinding {
            concept_id: ConceptCellId(22),
            salience: 0.9,
        })
        .with_semantic_descriptor(SemanticCodeDescriptor {
            codebook_id: 3,
            descriptor: [7_i8; 32],
            salience: 0.4,
        });

    let provider = FakeSemanticProvider::new();
    let bundle = provider.build_context_bundle(&request)?;

    assert!(bundle.gaussian_context.is_some());
    assert!(bundle.semantic_context.is_some());
    Ok(())
}

#[cfg(feature = "fake-semantic-provider")]
#[test]
fn fake_provider_can_run_without_any_optional_inputs() -> Result<(), ScaffoldContractError> {
    let request = SemanticContextRequest::new(alife_core::Vec3f::new(0.0, 0.0, 0.0));
    let provider = FakeSemanticProvider::new();
    let bundle = provider.build_context_bundle(&request)?;

    assert!(bundle.gaussian_context.is_none());
    assert!(bundle.semantic_context.is_none());
    Ok(())
}
