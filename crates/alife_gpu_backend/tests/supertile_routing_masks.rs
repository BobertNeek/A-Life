use alife_core::{
    BrainClassSpec, BrainScaleTier, CooEntry, CooTile, NeuralProjectionSchema, ProjectionTile,
    ScaffoldContractError, SparseTileCoord, SynapseWeightSplit, WeightSplitContract,
};
use alife_gpu_backend::{
    GpuActiveTileMaskConfig, GpuFixedPointPolicy, GpuReadbackClass, GpuReadbackPolicy,
    GpuRoutingMaskPlan, GpuStaticForwardPlan, GpuSupertileIndex, GpuSupertileMaskWords,
    GpuUploadBuffers, P27_SUPERTILE_MICROTILES, P27_WGSL_SUPERTILE_ROUTING,
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

fn two_tile_schema() -> NeuralProjectionSchema {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 0).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(0.25, 0.0, 1.0, 0.0, 0.1)).unwrap()
        ])
        .unwrap(),
    ));
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(1, 0).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(0.5, 0.0, 1.0, 0.0, 0.1)).unwrap()
        ])
        .unwrap(),
    ));
    schema.rebuild_supertile_masks();
    schema
}

fn activation_vec(first: f32, second_tile_source: f32) -> Vec<f32> {
    let mut values = vec![0.0; 512];
    values[0] = first;
    values[16] = second_tile_source;
    values
}

#[test]
fn supertile_index_math_handles_low_and_high_mask_boundaries() {
    assert_eq!(P27_SUPERTILE_MICROTILES, 8);

    let bit0 = GpuSupertileIndex::from_microtile(0, 0).unwrap();
    assert_eq!((bit0.supertile_row, bit0.supertile_col), (0, 0));
    assert_eq!((bit0.local_row, bit0.local_col, bit0.local_bit), (0, 0, 0));
    assert_eq!((bit0.mask_word_index, bit0.mask_word_bit), (0, 0));

    let bit31 = GpuSupertileIndex::from_microtile(3, 7).unwrap();
    assert_eq!(bit31.local_bit, 31);
    assert_eq!((bit31.mask_word_index, bit31.mask_word_bit), (0, 31));

    let bit32 = GpuSupertileIndex::from_microtile(4, 0).unwrap();
    assert_eq!(bit32.local_bit, 32);
    assert_eq!((bit32.mask_word_index, bit32.mask_word_bit), (1, 0));

    let bit63 = GpuSupertileIndex::from_microtile(7, 7).unwrap();
    assert_eq!(bit63.local_bit, 63);
    assert_eq!((bit63.mask_word_index, bit63.mask_word_bit), (1, 31));

    let next_supertile = GpuSupertileIndex::from_microtile(8, 8).unwrap();
    assert_eq!(
        (next_supertile.supertile_row, next_supertile.supertile_col),
        (1, 1)
    );
    assert_eq!(next_supertile.local_bit, 0);
}

#[test]
fn mask_packing_and_unpacking_splits_low_and_high_words() {
    let mut mask = GpuSupertileMaskWords::empty(7, 2, 3);
    mask.insert_local_bit(0).unwrap();
    mask.insert_local_bit(31).unwrap();
    mask.insert_local_bit(32).unwrap();
    mask.insert_local_bit(63).unwrap();

    assert_eq!(mask.low_word, 0x8000_0001);
    assert_eq!(mask.high_word, 0x8000_0001);
    assert!(mask.contains_local_bit(0).unwrap());
    assert!(mask.contains_local_bit(31).unwrap());
    assert!(mask.contains_local_bit(32).unwrap());
    assert!(mask.contains_local_bit(63).unwrap());
    assert!(!mask.contains_local_bit(30).unwrap());
    assert_eq!(
        mask.insert_local_bit(64),
        Err(ScaffoldContractError::InvalidSparseProjectionSchema)
    );
    assert_eq!(
        mask.contains_local_bit(64),
        Err(ScaffoldContractError::InvalidSparseProjectionSchema)
    );
}

#[test]
fn all_zero_cull_skips_all_tiles_and_keeps_output_zero() {
    let policy = GpuFixedPointPolicy::reference();
    let mut schema = two_tile_schema();
    schema.projections[0].supertile_masks[0].active_microtile_mask = 0;
    let upload = GpuUploadBuffers::from_cpu_schema(&schema, policy).unwrap();
    let plan = GpuStaticForwardPlan::from_upload(&upload, policy).unwrap();
    let activation_q = plan
        .quantize_activations(&activation_vec(0.75, 0.5))
        .unwrap();
    let result = plan.execute_cpu_diagnostic(&activation_q).unwrap();
    let counters = plan.routing_counters();

    assert_eq!(result.diagnostics.active_tiles, 0);
    assert_eq!(result.diagnostics.mask_skipped_tiles, 2);
    assert_eq!(result.diagnostics.active_synapses, 0);
    assert!(result.activations_q.iter().all(|value| *value == 0));
    assert_eq!(counters.skipped_supertiles, 1);
    assert_eq!(counters.skipped_microtiles, 2);
}

#[test]
fn active_tile_mask_passes_through_to_unculled_reference() {
    let policy = GpuFixedPointPolicy::reference();
    let schema = two_tile_schema();
    let mut unmasked = schema.clone();
    unmasked.projections[0].supertile_masks.clear();
    let masked_upload = GpuUploadBuffers::from_cpu_schema(&schema, policy).unwrap();
    let unmasked_upload = GpuUploadBuffers::from_cpu_schema(&unmasked, policy).unwrap();
    let masked_plan = GpuStaticForwardPlan::from_upload(&masked_upload, policy).unwrap();
    let unmasked_plan = GpuStaticForwardPlan::from_upload(&unmasked_upload, policy).unwrap();
    let activation_q = masked_plan
        .quantize_activations(&activation_vec(0.75, 0.5))
        .unwrap();

    let masked_result = masked_plan.execute_cpu_diagnostic(&activation_q).unwrap();
    let unmasked_result = unmasked_plan.execute_cpu_diagnostic(&activation_q).unwrap();

    assert_eq!(masked_result.activations_q, unmasked_result.activations_q);
    assert_eq!(masked_result.diagnostics.active_tiles, 2);
    assert_eq!(masked_result.diagnostics.mask_skipped_tiles, 0);
}

#[test]
fn masked_and_unmasked_outputs_match_when_skipped_region_is_inactive_zero() {
    let policy = GpuFixedPointPolicy::reference();
    let mut schema = two_tile_schema();
    schema.projections[0].tiles[1] = ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(1, 1).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(0.5, 0.0, 1.0, 0.0, 0.1)).unwrap()
        ])
        .unwrap(),
    );
    schema.rebuild_supertile_masks();
    schema.projections[0].supertile_masks[0].active_microtile_mask = 1;
    let mut unmasked = schema.clone();
    unmasked.projections[0].supertile_masks.clear();
    let masked_plan = GpuStaticForwardPlan::from_upload(
        &GpuUploadBuffers::from_cpu_schema(&schema, policy).unwrap(),
        policy,
    )
    .unwrap();
    let unmasked_plan = GpuStaticForwardPlan::from_upload(
        &GpuUploadBuffers::from_cpu_schema(&unmasked, policy).unwrap(),
        policy,
    )
    .unwrap();
    let activation_q = masked_plan
        .quantize_activations(&activation_vec(0.75, 0.0))
        .unwrap();

    let masked_result = masked_plan.execute_cpu_diagnostic(&activation_q).unwrap();
    let unmasked_result = unmasked_plan.execute_cpu_diagnostic(&activation_q).unwrap();

    assert_eq!(masked_result.activations_q, unmasked_result.activations_q);
    assert_eq!(masked_result.diagnostics.mask_skipped_tiles, 1);
}

#[test]
fn lobe_routing_descriptors_derive_from_core_data_and_reject_invalid_refs() {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Small1024);
    let weight_split = WeightSplitContract::for_brain_class(
        spec.id,
        spec.max_active_synapses,
        spec.max_active_microtiles,
        1,
    )
    .unwrap();
    let schema = NeuralProjectionSchema::from_routing_for_fixture(&spec, &weight_split).unwrap();
    let mut upload =
        GpuUploadBuffers::from_cpu_schema(&schema, GpuFixedPointPolicy::reference()).unwrap();
    let plan = GpuRoutingMaskPlan::from_upload_and_brain_class(
        &upload,
        &spec,
        GpuActiveTileMaskConfig::for_deterministic_fixture(0, true),
    )
    .unwrap();

    assert_eq!(
        plan.routing_descriptors_evaluated,
        upload.routing_descriptors.len() as u32
    );
    assert!(plan
        .routing_descriptors
        .iter()
        .any(|descriptor| descriptor.source_lobe_id != 0 && descriptor.target_lobe_id != 0));
    assert_eq!(plan.brain_class_id, u32::from(spec.id.raw()));

    upload.routing_descriptors[0].source_lobe_id = 99_999;
    assert_eq!(
        GpuRoutingMaskPlan::from_upload_and_brain_class(
            &upload,
            &spec,
            GpuActiveTileMaskConfig::for_deterministic_fixture(0, true),
        ),
        Err(ScaffoldContractError::InvalidSparseProjectionSchema)
    );
}

#[test]
fn active_mask_derivation_respects_fixture_budget_and_is_deterministic() {
    let policy = GpuFixedPointPolicy::reference();
    let upload = GpuUploadBuffers::from_cpu_schema(&two_tile_schema(), policy).unwrap();
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let first = GpuRoutingMaskPlan::from_upload_and_brain_class(
        &upload,
        &spec,
        GpuActiveTileMaskConfig {
            tick_index: 0,
            sensory_activity_present: true,
            biological_tile_budget: 1,
            force_static_fixture_tiles: false,
        },
    )
    .unwrap();
    let second = GpuRoutingMaskPlan::from_upload_and_brain_class(
        &upload,
        &spec,
        GpuActiveTileMaskConfig {
            tick_index: 0,
            sensory_activity_present: true,
            biological_tile_budget: 1,
            force_static_fixture_tiles: false,
        },
    )
    .unwrap();

    assert_eq!(first.active_masks, second.active_masks);
    assert_eq!(first.active_tiles, 1);
    assert_eq!(first.skipped_microtiles, 1);
}

#[test]
fn static_forward_can_consume_p27_masks_without_output_drift() {
    let policy = GpuFixedPointPolicy::reference();
    let mut upload = GpuUploadBuffers::from_cpu_schema(&two_tile_schema(), policy).unwrap();
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let routing = GpuRoutingMaskPlan::from_upload_and_brain_class(
        &upload,
        &spec,
        GpuActiveTileMaskConfig::for_deterministic_fixture(0, true),
    )
    .unwrap();
    upload.supertile_masks = routing.active_masks.clone();
    let plan = GpuStaticForwardPlan::from_upload(&upload, policy).unwrap();
    let activation_q = plan
        .quantize_activations(&activation_vec(0.75, 0.5))
        .unwrap();
    let result = plan.execute_cpu_diagnostic(&activation_q).unwrap();

    assert_eq!(result.diagnostics.active_tiles, 2);
    assert_eq!(result.diagnostics.mask_skipped_tiles, 0);
    assert_ne!(result.activations_q[0], 0);
    assert_ne!(result.activations_q[16], 0);
}

#[test]
fn p27_policy_exposes_diagnostics_only_readback_and_wgsl_contract() {
    let policy = GpuReadbackPolicy::active_gameplay();
    assert!(policy.allows(GpuReadbackClass::DiagnosticExportStaging));
    assert!(policy.allows(GpuReadbackClass::ActionSummaryStaging));
    assert!(!policy.allows(GpuReadbackClass::BulkActivation));
    assert!(!policy.allows(GpuReadbackClass::PerSynapse));
    assert!(!policy.allows(GpuReadbackClass::PerLobeSlice));
    assert!(!policy.allows(GpuReadbackClass::WeightBuffer));

    let module = naga::front::wgsl::parse_str(P27_WGSL_SUPERTILE_ROUTING).unwrap();
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator.validate(&module).unwrap();
    assert!(P27_WGSL_SUPERTILE_ROUTING.contains("fn p27_microtile_is_active"));
    assert!(!P27_WGSL_SUPERTILE_ROUTING.contains("recompact"));
    assert!(!P27_WGSL_SUPERTILE_ROUTING.contains("autophagy"));
}

#[test]
fn unsupported_tile_formats_remain_explicit_errors_under_masks() {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    schema.projections[0]
        .tiles
        .push(ProjectionTile::new_unsupported(
            0,
            SparseTileCoord::new(0, 0).unwrap(),
            alife_core::SparseTileType::RowRun,
        ));
    schema.rebuild_supertile_masks();

    assert_eq!(
        GpuUploadBuffers::from_cpu_schema(&schema, GpuFixedPointPolicy::reference()),
        Err(ScaffoldContractError::UnsupportedSparseTileFormat)
    );
}
