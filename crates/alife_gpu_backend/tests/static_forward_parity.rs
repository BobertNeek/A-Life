use alife_core::{
    cpu_spmv_projection, finalize_cpu_activations, ActivationFunction, BrainClassSpec,
    BrainScaleTier, CooEntry, CooTile, CpuNeuralState, DenseTile, NeuralActivationConfig,
    NeuralDiagnostics, NeuralProjectionSchema, ProjectionTile, SparseTileCoord, SynapseWeightSplit,
    MICROTILE_CELLS, MICROTILE_EDGE,
};
#[cfg(feature = "gpu-tests")]
use alife_gpu_backend::run_static_forward_gpu_diagnostic;
use alife_gpu_backend::{
    finalize_static_forward_accumulators_for_diagnostics, GpuFixedPointPolicy,
    GpuStaticForwardPlan, GpuUploadBuffers, P25_DIAGNOSTIC_COUNTER_WORDS,
    P25_STATIC_FORWARD_TOLERANCE_ABS, P25_STATIC_FORWARD_WORKGROUP_SIZE, P25_WGSL_STATIC_FORWARD,
};

fn weights(genetic: f32, lifetime: f32, alpha: f32, h: f32) -> SynapseWeightSplit {
    SynapseWeightSplit::new(genetic, lifetime, alpha, h, 0.0).unwrap()
}

fn parity_schema() -> NeuralProjectionSchema {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 0).unwrap(),
        CooTile::new(vec![
            CooEntry::new(0, 0, weights(0.25, 0.125, 0.5, 0.25)).unwrap(),
            CooEntry::new(1, 1, weights(-0.25, 0.5, 0.25, 1.0)).unwrap(),
        ])
        .unwrap(),
    ));

    let mut dense = vec![SynapseWeightSplit::zero(); MICROTILE_CELLS];
    dense[3] = weights(0.5, 0.0, 1.0, 0.5);
    dense[MICROTILE_EDGE as usize + 4] = weights(-0.25, 0.0, 1.0, -0.25);
    schema.projections[0].tiles.push(ProjectionTile::new_dense(
        0,
        SparseTileCoord::new(1, 0).unwrap(),
        DenseTile::new(dense).unwrap(),
    ));
    schema.rebuild_supertile_masks();
    schema
}

fn fixture_state() -> CpuNeuralState {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut state = CpuNeuralState::for_brain_class(&spec).unwrap();
    state.activations[0] = 0.5;
    state.activations[1] = -0.25;
    state.activations[3] = 0.75;
    state.activations[4] = -0.5;
    state
}

fn fixture_plan() -> (GpuStaticForwardPlan, CpuNeuralState, NeuralProjectionSchema) {
    let policy = GpuFixedPointPolicy::reference();
    let schema = parity_schema();
    let upload = GpuUploadBuffers::from_cpu_schema(&schema, policy).unwrap();
    let plan = GpuStaticForwardPlan::from_upload(&upload, policy).unwrap();
    (plan, fixture_state(), schema)
}

fn effective_weight_for(plan: &GpuStaticForwardPlan, target: u32, source: u32) -> i32 {
    let index = plan
        .packed_indices
        .iter()
        .position(|record| record.target_index == target && record.source_index == source)
        .unwrap();
    plan.effective_weight_q[index]
}

#[test]
fn static_forward_fixture_packs_dense_and_coo_effective_weights() {
    let (plan, _state, _schema) = fixture_plan();

    assert_eq!(P25_STATIC_FORWARD_WORKGROUP_SIZE, 64);
    assert_eq!(P25_DIAGNOSTIC_COUNTER_WORDS, 8);
    assert_eq!(plan.header.neuron_count, 512);
    assert_eq!(plan.tile_metadata.len(), 2);
    assert_eq!(plan.packed_indices.len(), MICROTILE_CELLS + 2);
    assert_eq!(plan.effective_weight_q.len(), MICROTILE_CELLS + 2);
    assert_eq!(plan.dispatch.pass0_workgroups, 9);
    assert_eq!(plan.dispatch.pass1_workgroups, 5);
    assert_eq!(plan.dispatch.pass2_workgroups, 8);

    assert_eq!(effective_weight_for(&plan, 0, 0), 2048);
    assert_eq!(effective_weight_for(&plan, 1, 1), 2048);
    assert_eq!(effective_weight_for(&plan, 16, 3), 4096);
    assert_eq!(effective_weight_for(&plan, 17, 4), -2048);
}

#[test]
fn static_forward_cpu_diagnostic_matches_p14_reference_outputs() {
    let (plan, mut cpu_state, schema) = fixture_plan();
    let activation_q = plan.quantize_activations(&cpu_state.activations).unwrap();
    let gpu_oracle = plan.execute_cpu_diagnostic(&activation_q).unwrap();

    let spmv =
        cpu_spmv_projection(&schema, &mut cpu_state, NeuralDiagnostics::reference()).unwrap();
    assert_eq!(spmv.active_tiles, 2);
    assert_eq!(spmv.active_synapses, (MICROTILE_CELLS + 2) as u32);
    finalize_cpu_activations(
        &mut cpu_state,
        NeuralActivationConfig {
            function: ActivationFunction::Identity,
            clamp_min: -1.0,
            clamp_max: 1.0,
            clear_accumulators: false,
        },
    )
    .unwrap();

    assert_eq!(gpu_oracle.diagnostics.active_tiles, 2);
    assert_eq!(
        gpu_oracle.diagnostics.active_synapses,
        (MICROTILE_CELLS + 2) as u32
    );
    assert_eq!(gpu_oracle.activations_q[0], 8192);
    assert_eq!(gpu_oracle.activations_q[1], -4096);
    assert_eq!(gpu_oracle.activations_q[16], 24575);
    assert_eq!(gpu_oracle.activations_q[17], 8192);

    let gpu_f32 = plan
        .dequantize_activations(&gpu_oracle.activations_q)
        .unwrap();
    for index in [0_usize, 1, 16, 17] {
        assert!(
            (gpu_f32[index] - cpu_state.activations[index]).abs()
                <= P25_STATIC_FORWARD_TOLERANCE_ABS,
            "index {index}: gpu={} cpu={}",
            gpu_f32[index],
            cpu_state.activations[index]
        );
    }
}

#[test]
fn activation_finalize_cpu_diagnostic_clamps_q_outputs() {
    let policy = GpuFixedPointPolicy::reference();
    let result = finalize_static_forward_accumulators_for_diagnostics(
        &[
            policy.activation_clamp_max_q + 64,
            policy.activation_clamp_min_q - 64,
            0,
        ],
        policy,
    )
    .unwrap();

    assert_eq!(result.activations_q[0], policy.activation_clamp_max_q);
    assert_eq!(result.activations_q[1], policy.activation_clamp_min_q);
    assert_eq!(result.activations_q[2], 0);
    assert_eq!(result.diagnostics.range_rejections, 2);
}

#[test]
fn static_forward_masks_skip_inactive_tiles() {
    let policy = GpuFixedPointPolicy::reference();
    let mut schema = parity_schema();
    schema.projections[0].supertile_masks[0].active_microtile_mask = 1;
    let upload = GpuUploadBuffers::from_cpu_schema(&schema, policy).unwrap();
    let plan = GpuStaticForwardPlan::from_upload(&upload, policy).unwrap();
    let state = fixture_state();
    let activation_q = plan.quantize_activations(&state.activations).unwrap();
    let result = plan.execute_cpu_diagnostic(&activation_q).unwrap();

    assert_eq!(result.diagnostics.active_tiles, 1);
    assert_eq!(result.diagnostics.mask_skipped_tiles, 1);
    assert_eq!(result.diagnostics.active_synapses, 2);
    assert_eq!(result.activations_q[0], 8192);
    assert_eq!(result.activations_q[16], 0);
    assert_eq!(result.activations_q[17], 0);
}

#[test]
fn p25_wgsl_static_forward_passes_parse_and_expose_expected_entries() {
    let module = naga::front::wgsl::parse_str(P25_WGSL_STATIC_FORWARD).unwrap();
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator.validate(&module).unwrap();

    assert!(P25_WGSL_STATIC_FORWARD.contains("fn clear_accumulators"));
    assert!(P25_WGSL_STATIC_FORWARD.contains("fn sparse_projection_spmv"));
    assert!(P25_WGSL_STATIC_FORWARD.contains("fn activation_finalize"));
    assert!(!P25_WGSL_STATIC_FORWARD.contains("oja"));
}

#[cfg(feature = "gpu-tests")]
#[test]
#[ignore = "requires a local wgpu adapter; run with `cargo test -p alife_gpu_backend --features gpu-tests --test static_forward_parity -- --ignored`"]
fn gpu_static_forward_passes_match_cpu_diagnostic_fixture() {
    pollster::block_on(async {
        let (plan, state, _schema) = fixture_plan();
        let activation_q = plan.quantize_activations(&state.activations).unwrap();
        let cpu = plan.execute_cpu_diagnostic(&activation_q).unwrap();
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("manual GPU parity test requires an adapter");
        let mut required_limits = wgpu::Limits::downlevel_defaults();
        required_limits.max_storage_buffers_per_shader_stage =
            required_limits.max_storage_buffers_per_shader_stage.max(9);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("p25-static-forward-parity-device"),
                required_features: wgpu::Features::empty(),
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("manual GPU parity test requires a device");

        let gpu = run_static_forward_gpu_diagnostic(&device, &queue, &plan, &activation_q)
            .await
            .unwrap();
        assert_eq!(gpu.activations_q, cpu.activations_q);
        assert_eq!(gpu.diagnostics, cpu.diagnostics);
    });
}
