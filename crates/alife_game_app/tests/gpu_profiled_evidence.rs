#![cfg(feature = "gpu-runtime")]

use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
    PhenotypeCompilerInputs, PolicyBackend, SensorProfile, SensorProfileIdentity,
    SensoryAbiVersion, Tick, TopologyCounts,
};
use alife_game_app::{
    load_gpu_slice_c_evidence, validate_gpu_evidence_file, CapacitySaturationEvidence,
    GpuMemoryGroundingEvidenceReceipt, GpuSliceEvidenceHeader, MemoryContextProbeEvidence,
    PhenotypeEvidenceManifest, ProfiledBehaviorReceiptHeader, TopologyCapacityReceipt,
    ValidatedGpuEvidence, GPU_EVIDENCE_BACKEND_API_SLUG, GPU_EVIDENCE_BACKEND_API_VERSION,
    GPU_EVIDENCE_PASSING_STATUS_RAW, GPU_SLICE_C_RAW, GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
};
use alife_gpu_backend::{GpuHardwareReceipt, GPU_HARDWARE_RECEIPT_SCHEMA_VERSION};

fn profile_identity(profile: SensorProfile) -> SensorProfileIdentity {
    SensorProfileIdentity {
        profile_id: profile.into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    }
}

fn fixture(profile: SensorProfile) -> GpuMemoryGroundingEvidenceReceipt {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(9_901, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
    let inputs = PhenotypeCompilerInputs::try_new(genome, &capacity, development, profile).unwrap();
    let phenotype = PhenotypeCompiler::compile_validated(&inputs, &capacity).unwrap();
    let manifest =
        PhenotypeEvidenceManifest::from_learning_phenotype(&phenotype, &capacity).unwrap();
    let sensor_profile = profile_identity(profile);
    let artifact_slug = format!(
        "gpu-memory-grounding-slice-c-{}-n512",
        match profile {
            SensorProfile::PrivilegedAffordanceV1 => "privileged-affordance-v1",
            SensorProfile::GroundedObjectSlotsV1 => "grounded-object-slots-v1",
        }
    );
    let hardware = GpuHardwareReceipt {
        schema_version: GPU_HARDWARE_RECEIPT_SCHEMA_VERSION,
        generation: 1,
        backend_api: "vulkan".to_string(),
        adapter_name: "test-vulkan-adapter".to_string(),
        vendor_id: 1,
        device_id: 2,
        driver_digest: [11, 12, 13, 14],
        feature_digest: [21, 22, 23, 24],
        limits_digest: [31, 32, 33, 34],
        gpu_layout_version: 3,
        backend_version: "gpu-closed-loop-test".to_string(),
    };
    let probe = MemoryContextProbeEvidence {
        phenotype_hash: phenotype.phenotype_hash(),
        phenotype_manifest_digest: manifest.manifest_digest,
        activation_digest: [41, 42, 43, 44],
        recurrent_activation_digest: [51, 52, 53, 54],
        lifetime_weight_digest: [61, 62, 63, 64],
        fast_weight_digest: [71, 72, 73, 74],
        eligibility_digest: [81, 82, 83, 84],
        poisoned_ingest_delta: -0.5,
        safe_ingest_delta: -0.01,
        selected_candidate: 3,
    };
    let capacity_saturation =
        (profile == SensorProfile::GroundedObjectSlotsV1).then_some(CapacitySaturationEvidence {
            grounded_semantic_label_channels_nonzero: 0,
            memory_records: 16,
            memory_capacity: 16,
            memory_merges: 2,
            memory_evictions: 1,
            tracked_object_records: 8,
            tracked_object_capacity: 8,
            tracked_object_evictions: 1,
            tracked_object_id_reuse_count: 0,
            topology_counts: TopologyCounts {
                concepts: 4,
                edges: 4,
                simplexes: 4,
                unresolved_gaps: 1,
            },
            topology_capacity: TopologyCapacityReceipt {
                max_concepts: 4,
                max_edges: 4,
                max_simplexes: 4,
                max_unresolved_gaps: 1,
                max_bindings_per_kind: 4,
            },
            max_observed_bindings_per_kind: 4,
            topology_degradations: 1,
            terminal_capacity_errors: 0,
        });
    let mut receipt = GpuMemoryGroundingEvidenceReceipt {
        header: ProfiledBehaviorReceiptHeader {
            common: GpuSliceEvidenceHeader {
                artifact_schema: GPU_SLICE_EVIDENCE_ARTIFACT_SCHEMA,
                slice_raw: GPU_SLICE_C_RAW,
                class_id_raw: capacity.id().raw(),
                profile_id_raw: sensor_profile.profile_id.raw(),
                profile_schema: sensor_profile.profile_schema_version,
                status_raw: GPU_EVIDENCE_PASSING_STATUS_RAW,
                git_commit: "1".repeat(40),
                source_tree_digest: "2".repeat(40),
                artifact_digest: [0; 4],
                phenotype_hash: phenotype.phenotype_hash(),
                phenotype_manifest_digest: manifest.manifest_digest,
                capacity_digest: capacity.canonical_digest(),
            },
            artifact_slug,
            backend_api_version: GPU_EVIDENCE_BACKEND_API_VERSION,
            backend_api_slug: GPU_EVIDENCE_BACKEND_API_SLUG.to_string(),
            adapter_name: hardware.adapter_name.clone(),
            adapter_backend: hardware.backend_api.clone(),
            run_seed: 4_303,
        },
        phenotype_manifest: manifest,
        sensor_profile,
        capacity_class_slug: "n512".to_string(),
        policy_backend: PolicyBackend::NeuralClosedLoopGpu,
        hardware,
        completed_ticks: if profile == SensorProfile::GroundedObjectSlotsV1 {
            10_240
        } else {
            64
        },
        completed_waking_ticks: 64,
        gpu_selection_count: 64,
        poisoned_ingest_candidate: 2,
        post_learning_selection: 3,
        poisoned_ingest_logit_before: 1.0,
        poisoned_ingest_logit_after: 0.5,
        poisoned_avoid_logit_before: 0.0,
        poisoned_avoid_logit_after: if profile == SensorProfile::GroundedObjectSlotsV1 {
            0.5
        } else {
            0.0
        },
        poisoned_ingest_delta: -0.5,
        safe_ingest_delta: -0.01,
        cyan_ingest_target_latent: if profile == SensorProfile::GroundedObjectSlotsV1 {
            [0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0]
        } else {
            [0.0; 8]
        },
        cyan_avoid_target_latent: if profile == SensorProfile::GroundedObjectSlotsV1 {
            [0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0]
        } else {
            [0.0; 8]
        },
        cyan_ingest_family_value: [0.0, 0.0, 0.5, 0.0],
        cyan_avoid_family_value: [0.0; 4],
        amber_target_latent: [0.0; 8],
        memory_enabled: probe.clone(),
        memory_ablated: MemoryContextProbeEvidence {
            poisoned_ingest_delta: 0.0,
            selected_candidate: 2,
            ..probe
        },
        capacity_saturation,
        compact_readback_bytes: 16,
        tolerance: 1.0e-6,
    };
    receipt.header.common.artifact_digest = receipt.recompute_artifact_digest().unwrap();
    receipt
}

fn unique_artifact_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("alife-slice-c-evidence-{nonce}.json"))
}

#[test]
fn profiled_receipts_use_exact_distinct_slugs_and_aggregate_keys() {
    let privileged = fixture(SensorProfile::PrivilegedAffordanceV1);
    let grounded = fixture(SensorProfile::GroundedObjectSlotsV1);

    privileged.validate_in_memory().unwrap();
    grounded.validate_in_memory().unwrap();
    assert_eq!(
        grounded.header.artifact_slug,
        "gpu-memory-grounding-slice-c-grounded-object-slots-v1-n512"
    );
    assert_eq!(
        privileged.header.artifact_slug,
        "gpu-memory-grounding-slice-c-privileged-affordance-v1-n512"
    );
    assert_ne!(privileged.aggregate_key(), grounded.aggregate_key());
}

#[test]
fn privileged_receipt_requires_family_conditioning_without_grounded_target_latents() {
    let mut privileged = fixture(SensorProfile::PrivilegedAffordanceV1);
    privileged.cyan_ingest_target_latent = [0.0; 8];
    privileged.cyan_avoid_target_latent = [0.0; 8];
    privileged.header.common.artifact_digest = privileged.recompute_artifact_digest().unwrap();

    privileged.validate_in_memory().unwrap();
}

#[test]
fn slice_c_loader_roundtrips_through_the_shared_validating_loader() {
    let receipt = fixture(SensorProfile::GroundedObjectSlotsV1);
    let path = unique_artifact_path();
    fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();

    assert_eq!(load_gpu_slice_c_evidence(&path).unwrap(), receipt);
    let shared = validate_gpu_evidence_file(GPU_SLICE_C_RAW, &path).unwrap();
    assert!(matches!(shared, ValidatedGpuEvidence::SliceC(value) if value == receipt));

    fs::remove_file(path).unwrap();
}

#[test]
fn slice_c_rejects_manifest_capacity_and_profile_tampering() {
    let receipt = fixture(SensorProfile::GroundedObjectSlotsV1);

    let mut manifest_tamper = receipt.clone();
    manifest_tamper.header.common.phenotype_manifest_digest[0] ^= 1;
    manifest_tamper.header.common.artifact_digest =
        manifest_tamper.recompute_artifact_digest().unwrap();
    assert!(manifest_tamper.validate_in_memory().is_err());

    let mut capacity_tamper = receipt.clone();
    capacity_tamper.header.common.capacity_digest[0] ^= 1;
    capacity_tamper.header.common.artifact_digest =
        capacity_tamper.recompute_artifact_digest().unwrap();
    assert!(capacity_tamper.validate_in_memory().is_err());

    let mut profile_tamper = receipt;
    profile_tamper.header.common.profile_id_raw = SensorProfile::PrivilegedAffordanceV1.raw();
    profile_tamper.header.common.artifact_digest =
        profile_tamper.recompute_artifact_digest().unwrap();
    assert!(profile_tamper.validate_in_memory().is_err());
}

#[test]
fn topology_capacity_receipt_bounds_counts_and_bindings() {
    let capacity = TopologyCapacityReceipt {
        max_concepts: 4,
        max_edges: 5,
        max_simplexes: 6,
        max_unresolved_gaps: 2,
        max_bindings_per_kind: 3,
    };
    assert!(capacity.contains(
        TopologyCounts {
            concepts: 4,
            edges: 5,
            simplexes: 6,
            unresolved_gaps: 2,
        },
        3,
    ));
    assert!(!capacity.contains(
        TopologyCounts {
            concepts: 5,
            edges: 5,
            simplexes: 6,
            unresolved_gaps: 2,
        },
        3,
    ));
}
