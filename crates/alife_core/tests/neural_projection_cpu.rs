use alife_core::{
    cpu_spmv_projection, finalize_cpu_activations, update_oja_shadow_traces, ActivationFunction,
    BrainClassSpec, BrainScaleTier, CooEntry, CooTile, CpuNeuralState, DenseTile,
    NeuralActivationConfig, NeuralDiagnostics, NeuralProjectionSchema, OjaUpdateConfig,
    ProjectionTile, ScaffoldContractError, SparseTileCoord, SparseTilePayload, SparseTileType,
    SynapseWeightSplit, WeightSplitContract, MICROTILE_CELLS, MICROTILE_EDGE, SUPERTILE_EDGE,
    SUPERTILE_MICROTILES,
};

fn weights(genetic: f32, lifetime: f32, alpha: f32, h: f32) -> SynapseWeightSplit {
    SynapseWeightSplit::new(genetic, lifetime, alpha, h, 0.0).unwrap()
}

#[test]
fn microtile_and_supertile_indexing_are_16_and_128_aligned() {
    assert_eq!(MICROTILE_EDGE, 16);
    assert_eq!(SUPERTILE_MICROTILES, 8);
    assert_eq!(SUPERTILE_EDGE, 128);

    let coord = SparseTileCoord::from_neuron_indices(143, 260).unwrap();
    assert_eq!(coord.microtile_row, 8);
    assert_eq!(coord.microtile_col, 16);
    assert_eq!(coord.target_start(), 128);
    assert_eq!(coord.source_start(), 256);
    assert_eq!(coord.supertile_row(), 1);
    assert_eq!(coord.supertile_col(), 2);
    assert_eq!(coord.supertile_local_bit(), 0);

    let last_local = SparseTileCoord::new(15, 23).unwrap();
    assert_eq!(last_local.supertile_row(), 1);
    assert_eq!(last_local.supertile_col(), 2);
    assert_eq!(last_local.supertile_local_bit(), 63);

    assert_eq!(
        SparseTileCoord::new(u32::MAX, 0),
        Err(ScaffoldContractError::InvalidSparseProjectionSchema)
    );
}

#[test]
fn dense_and_coo_tiles_decode_effective_weights() {
    let coord = SparseTileCoord::new(0, 1).unwrap();
    let mut dense_weights = vec![SynapseWeightSplit::zero(); MICROTILE_CELLS];
    dense_weights[2 * MICROTILE_EDGE as usize + 3] = weights(0.25, 0.5, 0.5, 0.25);
    let dense = ProjectionTile::new_dense(0, coord, DenseTile::new(dense_weights).unwrap());
    let decoded = dense.decode_synapses().unwrap();
    assert_eq!(decoded.len(), MICROTILE_CELLS);
    let selected = decoded
        .iter()
        .find(|synapse| synapse.target == 2 && synapse.source == 19)
        .unwrap();
    assert!((selected.effective_weight - 0.875).abs() < f32::EPSILON);

    let coo = ProjectionTile::new_coo(
        0,
        coord,
        CooTile::new(vec![
            CooEntry::new(4, 5, weights(-0.5, 0.25, 0.25, 1.0)).unwrap()
        ])
        .unwrap(),
    );
    let decoded = coo.decode_synapses().unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].target, 4);
    assert_eq!(decoded[0].source, 21);
    assert_eq!(decoded[0].effective_weight, 0.0);
}

#[test]
fn unsupported_tile_variants_are_defined_but_explicit_errors() {
    let tile = ProjectionTile::new_unsupported(
        0,
        SparseTileCoord::new(0, 0).unwrap(),
        SparseTileType::RowRun,
    );

    assert_eq!(
        tile.decode_synapses(),
        Err(ScaffoldContractError::UnsupportedSparseTileFormat)
    );
}

#[test]
fn cpu_spmv_uses_masks_and_effective_weight_formula() {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut state = CpuNeuralState::for_brain_class(&spec).unwrap();
    state.activations[0] = 2.0;
    state.activations[1] = 3.0;

    let mut active_schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    let tile = ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 0).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(0.5, 0.25, 0.5, 0.5)).unwrap(),
            CooEntry::new(1, 1, weights(-0.25, 0.5, 0.25, 1.0)).unwrap(),
        ])
        .unwrap(),
    );
    active_schema.projections[0].tiles.push(tile.clone());
    active_schema.rebuild_supertile_masks();
    let report =
        cpu_spmv_projection(&active_schema, &mut state, NeuralDiagnostics::reference()).unwrap();

    assert_eq!(report.active_tiles, 1);
    assert_eq!(report.active_synapses, 2);
    assert_eq!(state.accumulators[0], 2.0 * 1.0);
    assert_eq!(state.accumulators[1], 3.0 * 0.5);

    let mut culled_schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    culled_schema.projections[0].tiles.push(tile);
    culled_schema.rebuild_supertile_masks();
    culled_schema.projections[0].supertile_masks[0].active_microtile_mask = 0;
    let mut culled_state = CpuNeuralState::for_brain_class(&spec).unwrap();
    culled_state.activations[0] = 2.0;
    let report = cpu_spmv_projection(
        &culled_schema,
        &mut culled_state,
        NeuralDiagnostics::reference(),
    )
    .unwrap();
    assert_eq!(report.mask_skipped_tiles, 1);
    assert_eq!(culled_state.accumulators[0], 0.0);
}

#[test]
fn activation_finalization_clamps_and_tracks_previous_buffer() {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut state = CpuNeuralState::for_brain_class(&spec).unwrap();
    state.activations[0] = 0.25;
    state.accumulators[0] = 10.0;
    state.accumulators[1] = -10.0;

    finalize_cpu_activations(
        &mut state,
        NeuralActivationConfig {
            function: ActivationFunction::Identity,
            clamp_min: -0.75,
            clamp_max: 0.75,
            clear_accumulators: true,
        },
    )
    .unwrap();

    assert_eq!(state.previous_activations[0], 0.25);
    assert_eq!(state.activations[0], 0.75);
    assert_eq!(state.activations[1], -0.75);
    assert_eq!(state.accumulators[0], 0.0);
}

#[test]
fn oja_shadow_update_is_bounded_and_uses_safe_floats() {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut state = CpuNeuralState::for_brain_class(&spec).unwrap();
    state.previous_activations[0] = 0.8;
    state.activations[0] = 0.6;

    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 0).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(0.0, 0.0, 0.5, 0.9)).unwrap()
        ])
        .unwrap(),
    ));
    schema.rebuild_supertile_masks();

    let report = update_oja_shadow_traces(
        &mut schema,
        &state,
        OjaUpdateConfig {
            learning_rate: 4.0,
            learning_rate_scale: 1.0,
            decay: 1.0,
            shadow_min: -1.0,
            shadow_max: 1.0,
        },
    )
    .unwrap();

    assert_eq!(report.active_synapses, 1);
    let SparseTilePayload::Coo(tile) = &schema.projections[0].tiles[0].payload else {
        panic!("expected COO tile");
    };
    assert!((-1.0..=1.0).contains(&tile.entries[0].weights.h_shadow));
}

#[test]
fn routing_conversion_validates_alignment_and_is_deterministically_ordered() {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let split = WeightSplitContract::for_brain_class(
        spec.id,
        spec.max_active_synapses,
        spec.max_active_microtiles,
        99,
    )
    .unwrap();

    let schema = NeuralProjectionSchema::from_routing_for_fixture(&spec, &split).unwrap();
    schema.validate().unwrap();

    assert!(schema.projections.len() > 1);
    let ordered: Vec<_> = schema
        .projections
        .iter()
        .map(|projection| {
            (
                projection.routing_ref.source_lobe.stable_id().raw(),
                projection.routing_ref.target_lobe.stable_id().raw(),
                projection.routing_ref.projection_type as u8,
            )
        })
        .collect();
    let mut sorted = ordered.clone();
    sorted.sort();
    assert_eq!(ordered, sorted);

    let first = &schema.projections[0];
    assert_eq!(first.source_range.start % 16, 0);
    assert_eq!(first.target_range.start % 16, 0);
    assert_eq!(first.source_range.len % 16, 0);
    assert_eq!(first.target_range.len % 16, 0);
}

#[test]
fn invalid_projection_schema_is_rejected() {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    schema.projections[0].target_range.len = 15;

    assert_eq!(
        schema.validate(),
        Err(ScaffoldContractError::InvalidSparseProjectionSchema)
    );
}

#[test]
fn cpu_state_uses_linear_buffers_not_dense_weight_matrices() {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let state = CpuNeuralState::for_brain_class(&spec).unwrap();

    assert_eq!(state.activations.len(), spec.neuron_count as usize);
    assert_eq!(state.previous_activations.len(), spec.neuron_count as usize);
    assert_eq!(state.accumulators.len(), spec.neuron_count as usize);
    assert_eq!(
        state.weight_split.max_active_tiles,
        spec.max_active_microtiles
    );
    assert_eq!(
        state.update_metadata.max_active_synapses,
        spec.max_active_synapses
    );
    assert!(state.projections.is_empty());
}

#[test]
fn alife_core_neural_projection_api_stays_engine_independent() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<CpuNeuralState>();
    assert_send_sync::<NeuralProjectionSchema>();
}
