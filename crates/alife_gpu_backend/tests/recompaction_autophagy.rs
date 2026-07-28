use alife_core::{
    BrainClassSpec, BrainScaleTier, Confidence, CooEntry, CooTile, NeuralProjectionSchema,
    NormalizedScalar, ProjectionTile, ScaffoldContractError, SparseTileCoord, StructuralEditBatch,
    StructuralEditCandidate, StructuralEditKind, StructuralEditReason, SynapseWeightSplit, Tick,
};
use alife_gpu_backend::{
    GpuAutophagyPolicy, GpuBufferReplacement, GpuFixedPointPolicy, GpuReadbackClass,
    GpuReadbackPolicy, GpuRecompactionPlan, GpuRecompactionRemapTable, GpuRecompactionSwapState,
    GpuStaticForwardPlan, GpuUploadBuffers, P28_WGSL_RECOMPACTION_AUTOPHAGY,
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

fn recompaction_schema() -> NeuralProjectionSchema {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 0).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(0.25, 0.0, 1.0, 0.0, 0.1)).unwrap(),
            CooEntry::new(1, 1, SynapseWeightSplit::zero()).unwrap(),
        ])
        .unwrap(),
    ));
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(1, 0).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(0.5, 0.0, 1.0, 0.0, 0.2)).unwrap()
        ])
        .unwrap(),
    ));
    schema.rebuild_supertile_masks();
    schema
}

fn activation_vec(first: f32, second: f32) -> Vec<f32> {
    let mut values = vec![0.0; 512];
    values[0] = first;
    values[16] = second;
    values
}

fn structural_batch(kind: StructuralEditKind) -> StructuralEditBatch {
    let schema = recompaction_schema();
    let projection = schema.projections[0].routing_ref;
    StructuralEditBatch::new(
        Tick(42),
        vec![StructuralEditCandidate::new(
            7,
            projection,
            kind,
            StructuralEditReason::LowSalience,
            NormalizedScalar::new(0.1).unwrap(),
            Confidence::new(0.9).unwrap(),
            1,
        )
        .unwrap()],
        4,
    )
    .unwrap()
}

#[test]
fn structural_edit_import_rejects_invalid_and_unsupported_edits() {
    let schema = recompaction_schema();
    let unsupported = structural_batch(StructuralEditKind::SynaptogenesisCandidate);
    let plan = GpuRecompactionPlan::from_sleep_batch(
        &schema,
        &unsupported,
        GpuAutophagyPolicy::reference(),
    )
    .unwrap();

    assert_eq!(plan.diagnostics.edit_candidates_received, 1);
    assert_eq!(plan.diagnostics.unsupported_edit_kinds, 1);
    assert_eq!(plan.diagnostics.edits_accepted, 0);

    let capacity_policy = GpuAutophagyPolicy {
        max_edit_candidates: 0,
        ..GpuAutophagyPolicy::reference()
    };
    assert_eq!(
        GpuRecompactionPlan::from_sleep_batch(&schema, &unsupported, capacity_policy),
        Err(ScaffoldContractError::InvalidSparseProjectionSchema)
    );
    let projection = schema.projections[0].routing_ref;
    let overflow_batch = StructuralEditBatch::new(
        Tick(44),
        vec![
            StructuralEditCandidate::new(
                11,
                projection,
                StructuralEditKind::PruneMarker,
                StructuralEditReason::LowSalience,
                NormalizedScalar::new(0.1).unwrap(),
                Confidence::new(0.9).unwrap(),
                1,
            )
            .unwrap(),
            StructuralEditCandidate::new(
                12,
                projection,
                StructuralEditKind::RecompactionHint,
                StructuralEditReason::Fatigue,
                NormalizedScalar::new(0.1).unwrap(),
                Confidence::new(0.9).unwrap(),
                1,
            )
            .unwrap(),
        ],
        4,
    )
    .unwrap();
    let bounded_policy = GpuAutophagyPolicy {
        max_edit_candidates: 1,
        ..GpuAutophagyPolicy::reference()
    };
    assert_eq!(
        GpuRecompactionPlan::from_sleep_batch(&schema, &overflow_batch, bounded_policy),
        Err(ScaffoldContractError::ScalarOutOfRange)
    );

    let mut invalid = unsupported.clone();
    invalid.schema_version = invalid.schema_version.saturating_add(1);
    assert_eq!(
        GpuRecompactionPlan::from_sleep_batch(&schema, &invalid, GpuAutophagyPolicy::reference()),
        Err(ScaffoldContractError::IncompatibleAbi {
            kind: alife_core::SchemaKind::SleepConsolidation,
            expected: alife_core::SchemaVersions::CURRENT
                .sleep_consolidation
                .raw(),
            actual: invalid.schema_version,
        })
    );
}

#[test]
fn no_op_edit_batch_preserves_upload_and_static_forward_output() {
    let schema = recompaction_schema();
    let upload =
        GpuUploadBuffers::from_cpu_schema(&schema, GpuFixedPointPolicy::reference()).unwrap();
    let empty = StructuralEditBatch::new(Tick(43), Vec::new(), 4).unwrap();
    let plan =
        GpuRecompactionPlan::from_sleep_batch(&schema, &empty, GpuAutophagyPolicy::reference())
            .unwrap();
    let output = plan.rebuild_scratch_upload(&upload).unwrap();

    assert_eq!(output.diagnostics.edits_accepted, 0);
    assert_eq!(
        output.diagnostics.preserved_entries,
        upload.packed_indices.len() as u32
    );
    assert_eq!(
        output.compacted_upload.encoded_bytes(),
        upload.encoded_bytes()
    );
    assert_eq!(output.remap.old_to_new, vec![Some(0), Some(1), Some(2)]);

    let old_plan =
        GpuStaticForwardPlan::from_upload(&upload, GpuFixedPointPolicy::reference()).unwrap();
    let new_plan = GpuStaticForwardPlan::from_upload(
        &output.compacted_upload,
        GpuFixedPointPolicy::reference(),
    )
    .unwrap();
    let activation_q = old_plan
        .quantize_activations(&activation_vec(0.75, 0.5))
        .unwrap();
    let old_result = old_plan.execute_cpu_diagnostic(&activation_q).unwrap();
    let new_result = new_plan.execute_cpu_diagnostic(&activation_q).unwrap();
    assert_eq!(old_result.activations_q, new_result.activations_q);
}

#[test]
fn pruning_autophagy_remaps_zero_effective_synapse_without_output_drift() {
    let schema = recompaction_schema();
    let upload =
        GpuUploadBuffers::from_cpu_schema(&schema, GpuFixedPointPolicy::reference()).unwrap();
    let plan = GpuRecompactionPlan::from_sleep_batch(
        &schema,
        &structural_batch(StructuralEditKind::PruneMarker),
        GpuAutophagyPolicy::reference(),
    )
    .unwrap();
    let output = plan.rebuild_scratch_upload(&upload).unwrap();

    assert_eq!(output.diagnostics.edits_accepted, 1);
    assert_eq!(output.diagnostics.pruned_entries, 1);
    assert_eq!(output.diagnostics.remapped_entries, 2);
    assert_eq!(output.diagnostics.preserved_entries, 2);
    assert_eq!(output.diagnostics.byproduct_decay_events, 1);
    assert_eq!(output.diagnostics.brain_atp_recovery_signal_q16, 256);
    assert_eq!(output.remap.old_to_new, vec![Some(0), None, Some(1)]);
    assert_eq!(output.compacted_upload.packed_indices.len(), 2);
    assert_eq!(output.autophagy_markers.len(), 1);

    let old_plan =
        GpuStaticForwardPlan::from_upload(&upload, GpuFixedPointPolicy::reference()).unwrap();
    let new_plan = GpuStaticForwardPlan::from_upload(
        &output.compacted_upload,
        GpuFixedPointPolicy::reference(),
    )
    .unwrap();
    let activation_q = old_plan
        .quantize_activations(&activation_vec(0.75, 0.5))
        .unwrap();
    let old_result = old_plan.execute_cpu_diagnostic(&activation_q).unwrap();
    let new_result = new_plan.execute_cpu_diagnostic(&activation_q).unwrap();
    assert_eq!(old_result.activations_q, new_result.activations_q);
}

#[test]
fn double_buffer_swap_is_all_or_nothing_at_sleep_boundary() {
    let schema = recompaction_schema();
    let active =
        GpuUploadBuffers::from_cpu_schema(&schema, GpuFixedPointPolicy::reference()).unwrap();
    let plan = GpuRecompactionPlan::from_sleep_batch(
        &schema,
        &structural_batch(StructuralEditKind::PruneMarker),
        GpuAutophagyPolicy::reference(),
    )
    .unwrap();
    let output = plan.rebuild_scratch_upload(&active).unwrap();
    let replacement = GpuBufferReplacement::stage(active.clone(), output.clone()).unwrap();

    assert_eq!(replacement.state(), GpuRecompactionSwapState::ReadyToSwap);
    let failed = replacement.reject_for_failed_validation();
    assert_eq!(failed.state(), GpuRecompactionSwapState::Failed);
    assert_eq!(
        failed.active_upload().encoded_bytes(),
        active.encoded_bytes()
    );

    let committed = GpuBufferReplacement::stage(active.clone(), output)
        .unwrap()
        .swap_at_sleep_boundary()
        .unwrap();
    assert_eq!(committed.state(), GpuRecompactionSwapState::Active);
    assert_ne!(
        committed.active_upload().encoded_bytes(),
        active.encoded_bytes()
    );
}

#[test]
fn persisted_weight_layers_and_routing_masks_survive_recompaction() {
    let schema = recompaction_schema();
    let upload =
        GpuUploadBuffers::from_cpu_schema(&schema, GpuFixedPointPolicy::reference()).unwrap();
    let output = GpuRecompactionPlan::from_sleep_batch(
        &schema,
        &structural_batch(StructuralEditKind::PruneMarker),
        GpuAutophagyPolicy::reference(),
    )
    .unwrap()
    .rebuild_scratch_upload(&upload)
    .unwrap();

    assert_eq!(output.routing_mask_preservation.failures, 0);
    assert_eq!(
        output.compacted_upload.header.routing_descriptor_count,
        upload.header.routing_descriptor_count
    );
    assert_eq!(
        output.compacted_upload.supertile_masks,
        upload.supertile_masks
    );

    let compacted = &output.compacted_upload;
    assert_eq!(
        compacted.genetic_fixed_q.len(),
        compacted.packed_indices.len()
    );
    assert_eq!(
        compacted.lifetime_consolidated_q.len(),
        compacted.packed_indices.len()
    );
    assert_eq!(
        compacted.h_operational_q.len(),
        compacted.packed_indices.len()
    );
    assert_eq!(compacted.h_shadow_q.len(), compacted.packed_indices.len());
    assert!(compacted.h_shadow_q.iter().any(|value| *value != 0));
}

#[test]
fn active_gameplay_policy_still_rejects_bulk_structural_readback() {
    let policy = GpuReadbackPolicy::active_gameplay();
    assert!(policy.allows(GpuReadbackClass::DiagnosticExportStaging));
    assert!(!policy.allows(GpuReadbackClass::PerSynapse));
    assert!(!policy.allows(GpuReadbackClass::WeightBuffer));
}

#[test]
fn invalid_remap_tables_and_p28_wgsl_contract_are_rejected_or_sleep_only() {
    assert_eq!(
        GpuRecompactionRemapTable {
            old_to_new: vec![Some(0), Some(0)]
        }
        .validate(2),
        Err(ScaffoldContractError::InvalidSparseProjectionSchema)
    );

    let module = naga::front::wgsl::parse_str(P28_WGSL_RECOMPACTION_AUTOPHAGY).unwrap();
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator.validate(&module).unwrap();
    assert!(P28_WGSL_RECOMPACTION_AUTOPHAGY.contains("p28_recompaction_contract_stub"));
    assert!(P28_WGSL_RECOMPACTION_AUTOPHAGY.contains("sleep/offline"));
    assert!(P28_WGSL_RECOMPACTION_AUTOPHAGY.contains("active gameplay must not depend"));
}
