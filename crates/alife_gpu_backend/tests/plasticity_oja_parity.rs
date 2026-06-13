use alife_core::{
    update_oja_shadow_traces, BrainClassSpec, BrainScaleTier, CooEntry, CooTile, CpuNeuralState,
    NeuralProjectionSchema, OjaUpdateConfig, ProjectionTile, SparseTileCoord, SparseTilePayload,
    SynapseWeightSplit,
};
#[cfg(feature = "gpu-tests")]
use alife_gpu_backend::run_plasticity_gpu_diagnostic;
use alife_gpu_backend::{
    GpuFixedPointPolicy, GpuOjaFixedPointConfig, GpuPlasticityPlan, GpuUploadBuffers,
    P26_PLASTICITY_TOLERANCE_Q, P26_PLASTICITY_WORKGROUP_SIZE, P26_WGSL_PLASTICITY,
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

fn plasticity_schema(alpha: f32, h_shadow: f32) -> NeuralProjectionSchema {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    schema.projections[0].tiles.push(ProjectionTile::new_coo(
        0,
        SparseTileCoord::new(0, 0).unwrap(),
        CooTile::new(vec![CooEntry::new(
            0,
            0,
            weights(0.25, 0.125, alpha, 0.5, h_shadow),
        )
        .unwrap()])
        .unwrap(),
    ));
    schema.rebuild_supertile_masks();
    schema
}

fn fixture_plan(alpha: f32, h_shadow: f32) -> GpuPlasticityPlan {
    let policy = GpuFixedPointPolicy::reference();
    let upload = GpuUploadBuffers::from_cpu_schema(&plasticity_schema(alpha, h_shadow), policy)
        .expect("fixture should encode");
    GpuPlasticityPlan::from_upload(
        &upload,
        policy,
        GpuOjaFixedPointConfig::from_oja_config(
            OjaUpdateConfig {
                learning_rate: 0.5,
                learning_rate_scale: 1.0,
                decay: 1.0,
                shadow_min: -1.0,
                shadow_max: 1.0,
            },
            policy,
            0xACE1,
        )
        .unwrap(),
    )
    .unwrap()
}

#[test]
fn plasticity_plan_preserves_weight_split_and_dispatch_shape() {
    let plan = fixture_plan(0.5, 0.1);

    assert_eq!(P26_PLASTICITY_WORKGROUP_SIZE, 64);
    assert_eq!(plan.header.neuron_count, 512);
    assert_eq!(plan.packed_indices.len(), 1);
    assert_eq!(plan.dispatch.pass3_workgroups, 1);
    assert_eq!(plan.genetic_fixed_q, vec![1024]);
    assert_eq!(plan.lifetime_consolidated_q, vec![512]);
    assert_eq!(plan.alpha_q16.len(), 1);
    assert_eq!(plan.h_operational_q, vec![2048]);
    assert_eq!(plan.h_shadow_initial_q, vec![410]);
}

#[test]
fn plasticity_cpu_diagnostic_updates_h_shadow_only_when_alpha_is_positive() {
    let active = fixture_plan(0.5, 0.1);
    let pre_q = active
        .quantize_activations(&activation_vec(0.8, 0.0))
        .unwrap();
    let post_q = active
        .quantize_activations(&activation_vec(0.6, 0.0))
        .unwrap();

    let result = active.execute_cpu_diagnostic(&pre_q, &post_q).unwrap();
    assert_eq!(result.diagnostics.active_synapses, 1);
    assert_eq!(result.diagnostics.alpha_zero_skips, 0);
    assert_ne!(result.h_shadow_q, active.h_shadow_initial_q);
    assert_eq!(result.genetic_fixed_q, active.genetic_fixed_q);
    assert_eq!(
        result.lifetime_consolidated_q,
        active.lifetime_consolidated_q
    );
    assert_eq!(result.h_operational_q, active.h_operational_q);

    let inactive = fixture_plan(0.0, 0.1);
    let result = inactive.execute_cpu_diagnostic(&pre_q, &post_q).unwrap();
    assert_eq!(result.h_shadow_q, inactive.h_shadow_initial_q);
    assert_eq!(result.diagnostics.alpha_zero_skips, 1);
}

#[test]
fn plasticity_fixed_point_matches_cpu_oja_reference_with_tolerance() {
    let policy = GpuFixedPointPolicy::reference();
    let oja = OjaUpdateConfig {
        learning_rate: 0.5,
        learning_rate_scale: 1.0,
        decay: 1.0,
        shadow_min: -1.0,
        shadow_max: 1.0,
    };
    let mut schema = plasticity_schema(0.5, 0.1);
    let upload = GpuUploadBuffers::from_cpu_schema(&schema, policy).unwrap();
    let plan = GpuPlasticityPlan::from_upload(
        &upload,
        policy,
        GpuOjaFixedPointConfig::from_oja_config(oja, policy, 0xACE1).unwrap(),
    )
    .unwrap();

    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut state = CpuNeuralState::for_brain_class(&spec).unwrap();
    state.previous_activations[0] = 0.8;
    state.activations[0] = 0.6;
    update_oja_shadow_traces(&mut schema, &state, oja).unwrap();

    let pre_q = plan
        .quantize_activations(&activation_vec(0.8, 0.0))
        .unwrap();
    let post_q = plan
        .quantize_activations(&activation_vec(0.6, 0.0))
        .unwrap();
    let result = plan.execute_cpu_diagnostic(&pre_q, &post_q).unwrap();
    let expected = match &schema.projections[0].tiles[0].payload {
        SparseTilePayload::Coo(tile) => policy.quantize_weight(tile.entries[0].weights.h_shadow),
        _ => unreachable!("fixture is COO"),
    }
    .unwrap();

    assert!(
        (i32::from(result.h_shadow_q[0]) - i32::from(expected)).abs() <= P26_PLASTICITY_TOLERANCE_Q,
        "gpu-fixed={} cpu={}",
        result.h_shadow_q[0],
        expected
    );
}

#[test]
fn plasticity_clamps_saturation_and_rejects_invalid_scales() {
    let policy = GpuFixedPointPolicy::reference();
    let upload = GpuUploadBuffers::from_cpu_schema(&plasticity_schema(1.0, 0.95), policy).unwrap();
    let config = GpuOjaFixedPointConfig::from_oja_config(
        OjaUpdateConfig {
            learning_rate: 8.0,
            learning_rate_scale: 1.0,
            decay: 0.0,
            shadow_min: -1.0,
            shadow_max: 1.0,
        },
        policy,
        7,
    )
    .unwrap();
    let plan = GpuPlasticityPlan::from_upload(&upload, policy, config).unwrap();
    let pre_q = plan
        .quantize_activations(&activation_vec(1.0, 0.0))
        .unwrap();
    let post_q = plan
        .quantize_activations(&activation_vec(1.0, 0.0))
        .unwrap();
    let result = plan.execute_cpu_diagnostic(&pre_q, &post_q).unwrap();

    assert_eq!(result.h_shadow_q[0], config.shadow_max_q);
    assert_eq!(result.diagnostics.saturation_count, 1);

    let invalid_policy = GpuFixedPointPolicy {
        activation_scale: 0,
        ..policy
    };
    assert!(GpuOjaFixedPointConfig::from_oja_config(
        config.to_oja_config(policy),
        invalid_policy,
        1
    )
    .is_err());
}

#[test]
fn stochastic_rounding_is_seeded_and_deterministic() {
    let a = GpuOjaFixedPointConfig::stochastic_round_div_signed(7, 10, 0xA5A5).unwrap();
    let b = GpuOjaFixedPointConfig::stochastic_round_div_signed(7, 10, 0xA5A5).unwrap();
    let c = GpuOjaFixedPointConfig::stochastic_round_div_signed(7, 10, 0xA5A6).unwrap();

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn p26_wgsl_plasticity_pass_parses_and_exposes_expected_entry() {
    let module = naga::front::wgsl::parse_str(P26_WGSL_PLASTICITY).unwrap();
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator.validate(&module).unwrap();

    assert!(P26_WGSL_PLASTICITY.contains("fn plasticity_update"));
    assert!(P26_WGSL_PLASTICITY.contains("h_shadow_write_q"));
    assert!(!P26_WGSL_PLASTICITY.contains("atomicStore(&diagnostics"));
    assert!(!P26_WGSL_PLASTICITY.contains("genetic_fixed_q: array<atomic"));
}

#[ignore = "manual extended saturation/bias smoke: run with `cargo test -p alife_gpu_backend --test plasticity_oja_parity -- --ignored`"]
#[test]
fn plasticity_long_run_saturation_bias_smoke_stays_bounded() {
    let mut plan = fixture_plan(1.0, 0.0);
    let pre_q = plan
        .quantize_activations(&activation_vec(0.125, 0.0))
        .unwrap();
    let post_q = plan
        .quantize_activations(&activation_vec(0.125, 0.0))
        .unwrap();

    for _ in 0..4096 {
        let result = plan.execute_cpu_diagnostic(&pre_q, &post_q).unwrap();
        plan.h_shadow_initial_q = result.h_shadow_q;
    }

    assert!(plan.h_shadow_initial_q[0] >= plan.oja.shadow_min_q);
    assert!(plan.h_shadow_initial_q[0] <= plan.oja.shadow_max_q);
}

#[cfg(feature = "gpu-tests")]
#[test]
#[ignore = "requires a local wgpu adapter; run with `cargo test -p alife_gpu_backend --features gpu-tests --test plasticity_oja_parity -- --ignored`"]
fn gpu_plasticity_pass_matches_cpu_diagnostic_fixture() {
    pollster::block_on(async {
        let plan = fixture_plan(0.5, 0.1);
        let pre_q = plan
            .quantize_activations(&activation_vec(0.8, 0.0))
            .unwrap();
        let post_q = plan
            .quantize_activations(&activation_vec(0.6, 0.0))
            .unwrap();
        let cpu = plan.execute_cpu_diagnostic(&pre_q, &post_q).unwrap();
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("manual GPU parity test requires an adapter");
        let mut required_limits = wgpu::Limits::downlevel_defaults();
        required_limits.max_storage_buffers_per_shader_stage =
            required_limits.max_storage_buffers_per_shader_stage.max(10);
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("p26-plasticity-parity-device"),
                required_features: wgpu::Features::empty(),
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("manual GPU parity test requires a device");

        let gpu = run_plasticity_gpu_diagnostic(&device, &queue, &plan, &pre_q, &post_q)
            .await
            .unwrap();
        assert_eq!(gpu.h_shadow_q, cpu.h_shadow_q);
        assert_eq!(gpu.diagnostics, cpu.diagnostics);
    });
}

fn activation_vec(index0: f32, index1: f32) -> Vec<f32> {
    let mut values = vec![0.0; 512];
    values[0] = index0;
    values[1] = index1;
    values
}
