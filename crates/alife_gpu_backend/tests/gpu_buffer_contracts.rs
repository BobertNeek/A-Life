use alife_core::{
    BrainClassSpec, BrainScaleTier, CooEntry, CooTile, DenseTile, NeuralProjectionSchema,
    ProjectionTile, ScaffoldContractError, SchemaKind, SchemaVersions, SparseTileCoord,
    SynapseWeightSplit, MICROTILE_CELLS,
};
use alife_gpu_backend::{
    GpuBufferContractHeader, GpuBufferView, GpuFixedPointPolicy, GpuReadbackClass,
    GpuReadbackPolicy, GpuShaderPass, GpuTileMetadataRecord, GpuUploadBuffers,
    GpuWeightBufferViews, WeightBufferFormat, GPU_ACTION_SUMMARY_RECORD_BYTES,
    GPU_BUFFER_CONTRACT_SCHEMA_VERSION, GPU_DIAGNOSTIC_COUNTER_BYTES, GPU_HEADER_BYTES,
    GPU_PACKED_SYNAPSE_INDEX_BYTES, GPU_ROUTING_DESCRIPTOR_BYTES, GPU_SUPERTILE_MASK_BYTES,
    GPU_TILE_METADATA_BYTES,
};

fn weights(
    genetic_fixed: f32,
    lifetime_consolidated: f32,
    alpha: f32,
    h_operational: f32,
    h_shadow: f32,
) -> SynapseWeightSplit {
    SynapseWeightSplit::new(
        genetic_fixed,
        lifetime_consolidated,
        alpha,
        h_operational,
        h_shadow,
    )
    .unwrap()
}

fn fixture_schema() -> NeuralProjectionSchema {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 0).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 1, weights(0.25, 0.125, 0.5, 0.25, -0.25)).unwrap(),
            CooEntry::new(2, 3, weights(-0.5, 0.25, 0.25, 0.5, 0.125)).unwrap(),
        ])
        .unwrap(),
    ));

    let mut dense = vec![SynapseWeightSplit::zero(); MICROTILE_CELLS];
    dense[5] = weights(0.125, 0.0, 1.0, 0.25, 0.0);
    schema.projections[0].tiles.push(ProjectionTile::new_dense(
        0,
        SparseTileCoord::new(1, 0).unwrap(),
        DenseTile::new(dense).unwrap(),
    ));
    schema.rebuild_supertile_masks();
    schema
}

#[test]
fn buffer_contract_records_have_explicit_byte_sizes_and_alignment() {
    assert_eq!(GPU_HEADER_BYTES, 48);
    assert_eq!(GPU_TILE_METADATA_BYTES, 32);
    assert_eq!(GPU_SUPERTILE_MASK_BYTES, 24);
    assert_eq!(GPU_PACKED_SYNAPSE_INDEX_BYTES, 16);
    assert_eq!(GPU_ROUTING_DESCRIPTOR_BYTES, 64);
    assert_eq!(GPU_DIAGNOSTIC_COUNTER_BYTES, 32);
    assert_eq!(GPU_ACTION_SUMMARY_RECORD_BYTES, 64);

    let header = GpuBufferContractHeader {
        gpu_schema_version: GPU_BUFFER_CONTRACT_SCHEMA_VERSION,
        neural_projection_schema_version: SchemaVersions::CURRENT.neural_projection.raw(),
        brain_class_id: 1,
        neuron_count: 512,
        microtile_edge: 16,
        supertile_edge: 128,
        projection_count: 1,
        tile_count: 2,
        synapse_count: 258,
        routing_descriptor_count: 1,
        flags: 0,
    };
    assert_eq!(header.to_le_bytes().len(), GPU_HEADER_BYTES);

    let view = GpuBufferView {
        offset_bytes: 64,
        len_bytes: 128,
        stride_bytes: 2,
        format: WeightBufferFormat::I16Fixed,
    };
    assert!(view.is_aligned());
}

#[test]
fn cpu_fixture_converts_to_deterministic_gpu_upload_buffers() {
    let schema = fixture_schema();
    let upload = GpuUploadBuffers::from_cpu_schema(&schema, GpuFixedPointPolicy::reference())
        .expect("fixture should encode");

    assert_eq!(
        upload.header.gpu_schema_version,
        GPU_BUFFER_CONTRACT_SCHEMA_VERSION
    );
    assert_eq!(
        upload.header.neural_projection_schema_version,
        schema.schema_version
    );
    assert_eq!(upload.header.neuron_count, 512);
    assert_eq!(upload.tile_metadata.len(), 2);
    assert_eq!(upload.packed_indices.len(), MICROTILE_CELLS + 2);
    assert_eq!(upload.genetic_fixed_q.len(), upload.packed_indices.len());
    assert_eq!(
        upload.lifetime_consolidated_q.len(),
        upload.packed_indices.len()
    );
    assert_eq!(upload.alpha_q16.len(), upload.packed_indices.len());
    assert_eq!(upload.h_operational_q.len(), upload.packed_indices.len());
    assert_eq!(upload.h_shadow_q.len(), upload.packed_indices.len());
    assert_eq!(upload.routing_descriptors.len(), 1);

    let encoded_again =
        GpuUploadBuffers::from_cpu_schema(&schema, GpuFixedPointPolicy::reference())
            .expect("fixture should encode twice");
    assert_eq!(upload.encoded_bytes(), encoded_again.encoded_bytes());
}

#[test]
fn tile_metadata_and_supertile_masks_encode_cpu_schema_positions() {
    let upload =
        GpuUploadBuffers::from_cpu_schema(&fixture_schema(), GpuFixedPointPolicy::reference())
            .unwrap();

    let first = upload.tile_metadata[0];
    assert_eq!(
        first,
        GpuTileMetadataRecord {
            projection_index: 0,
            microtile_row: 0,
            microtile_col: 0,
            tile_type: 2,
            nonzero_count: 2,
            synapse_offset: 0,
            synapse_count: 2,
            flags: 0,
        }
    );
    assert_eq!(first.to_le_bytes().len(), GPU_TILE_METADATA_BYTES);

    let mask = upload.supertile_masks[0];
    assert_eq!(mask.projection_index, 0);
    assert_eq!(mask.supertile_row, 0);
    assert_eq!(mask.supertile_col, 0);
    assert_eq!(mask.active_microtile_mask_lo, 0b1 | 0b1_0000_0000);
    assert_eq!(mask.active_microtile_mask_hi, 0);
    assert_eq!(mask.to_le_bytes().len(), GPU_SUPERTILE_MASK_BYTES);
}

#[test]
fn routing_descriptors_and_weight_views_are_page_relative() {
    let upload =
        GpuUploadBuffers::from_cpu_schema(&fixture_schema(), GpuFixedPointPolicy::reference())
            .unwrap();
    let routing = upload.routing_descriptors[0];
    assert_eq!(routing.projection_index, 0);
    assert_eq!(routing.source_start, 0);
    assert_eq!(routing.target_start, 0);
    assert_eq!(routing.tile_metadata_offset, 0);
    assert_eq!(routing.tile_count, 2);
    assert_eq!(routing.supertile_mask_offset, 0);
    assert_eq!(routing.supertile_mask_count, 1);
    assert_eq!(routing.to_le_bytes().len(), GPU_ROUTING_DESCRIPTOR_BYTES);

    let GpuWeightBufferViews {
        genetic_fixed,
        lifetime_consolidated,
        alpha,
        h_operational,
        h_shadow,
    } = upload.weight_views();
    assert_eq!(genetic_fixed.offset_bytes, 0);
    assert_eq!(genetic_fixed.stride_bytes, 2);
    assert_eq!(lifetime_consolidated.offset_bytes, genetic_fixed.len_bytes);
    assert_eq!(alpha.stride_bytes, 2);
    assert_eq!(h_operational.stride_bytes, 2);
    assert_eq!(h_shadow.stride_bytes, 2);

    let activation = upload.activation_ping_pong_views();
    assert_eq!(activation.activation_read.offset_bytes, 0);
    assert_eq!(
        activation.activation_write.offset_bytes,
        activation.activation_read.len_bytes
    );
    assert_eq!(activation.activation_read.stride_bytes, 4);

    let accumulators = upload.accumulator_layout();
    assert_eq!(accumulators.accumulators.offset_bytes, 0);
    assert_eq!(accumulators.accumulators.stride_bytes, 4);
    assert_eq!(
        accumulators.diagnostics.offset_bytes,
        accumulators.accumulators.len_bytes
    );
}

#[test]
fn fixed_point_policy_clamps_flags_overflow_and_rejects_invalid_scales() {
    let policy = GpuFixedPointPolicy::reference();
    assert_eq!(policy.quantize_weight(0.5).unwrap(), 2048);
    assert_eq!(policy.quantize_alpha(0.5).unwrap(), 32768);
    assert_eq!(
        policy.clamp_activation_q(99_999),
        policy.activation_clamp_max_q
    );
    assert!(policy.accumulator_overflows(policy.accumulator_abs_limit_q + 1));
    assert!(!policy.accumulator_overflows(policy.accumulator_abs_limit_q));

    let invalid = GpuFixedPointPolicy {
        weight_scale: 0,
        ..policy
    };
    assert_eq!(
        invalid.validate(),
        Err(ScaffoldContractError::ScalarOutOfRange)
    );
}

#[test]
fn incompatible_cpu_schema_version_is_rejected() {
    let mut schema = fixture_schema();
    schema.schema_version = schema.schema_version.saturating_add(1);
    let err = GpuUploadBuffers::from_cpu_schema(&schema, GpuFixedPointPolicy::reference())
        .expect_err("future schema must be rejected");
    assert_eq!(
        err,
        ScaffoldContractError::IncompatibleAbi {
            kind: SchemaKind::NeuralProjection,
            expected: SchemaVersions::CURRENT.neural_projection.raw(),
            actual: schema.schema_version,
        }
    );
}

#[test]
fn no_readback_policy_allows_only_summary_and_diagnostic_staging() {
    let policy = GpuReadbackPolicy::active_gameplay();
    assert!(policy.allows(GpuReadbackClass::ActionSummaryStaging));
    assert!(policy.allows(GpuReadbackClass::DiagnosticExportStaging));
    assert!(!policy.allows(GpuReadbackClass::BulkActivation));
    assert!(!policy.allows(GpuReadbackClass::PerSynapse));
    assert!(!policy.allows(GpuReadbackClass::PerLobeSlice));
    assert!(!policy.allows(GpuReadbackClass::WeightBuffer));
}

#[test]
fn wgsl_contract_passes_keep_p27_and_p28_deferred() {
    let passes = GpuShaderPass::contract_order();
    assert_eq!(passes.len(), 4);
    assert!(matches!(passes[0], GpuShaderPass::ClearAccumulators));
    assert!(matches!(passes[1], GpuShaderPass::SparseProjectionSpmv));
    assert!(matches!(passes[2], GpuShaderPass::ActivationFinalize));
    assert!(matches!(passes[3], GpuShaderPass::PlasticityUpdate));
    assert!(GpuShaderPass::culling_recompaction_hooks_are_deferred());
}
