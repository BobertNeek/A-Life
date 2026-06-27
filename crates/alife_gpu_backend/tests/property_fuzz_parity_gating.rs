use alife_core::{
    cpu_spmv_projection, finalize_cpu_activations, update_oja_shadow_traces, ActivationFunction,
    BrainClassSpec, BrainScaleTier, CooEntry, CooTile, CpuNeuralState, NeuralActivationConfig,
    NeuralDiagnostics, NeuralProjectionSchema, OjaUpdateConfig, ProjectionTile, SparseTileCoord,
    SparseTilePayload, SynapseWeightSplit, MICROTILE_CELLS, MICROTILE_EDGE,
};
use alife_gpu_backend::{
    GpuActiveTileMaskConfig, GpuFixedPointPolicy, GpuOjaFixedPointConfig, GpuPlasticityPlan,
    GpuRoutingMaskPlan, GpuStaticForwardPlan, GpuUploadBuffers, P25_STATIC_FORWARD_TOLERANCE_ABS,
    P26_PLASTICITY_TOLERANCE_Q,
};

const CA35_STATIC_CASES: &[u64] = &[
    0xCA35_0001,
    0xCA35_0002,
    0xCA35_0003,
    0xCA35_0004,
    0xCA35_0005,
    0xCA35_0006,
    0xCA35_0007,
    0xCA35_0008,
];
const CA35_PLASTICITY_CASES: &[u64] = &[
    0xCA35_A001,
    0xCA35_A002,
    0xCA35_A003,
    0xCA35_A004,
    0xCA35_A005,
    0xCA35_A006,
];
const CA35_ROUTING_CASES: &[u64] = &[0xCA35_B001, 0xCA35_B002, 0xCA35_B003, 0xCA35_B004];

#[derive(Debug, Clone, Copy)]
struct Ca35FuzzCase {
    seed: u64,
    tile_count: u32,
    dense_every: u32,
    mask_mode: Ca35MaskMode,
}

impl Ca35FuzzCase {
    fn repro(self) -> String {
        format!(
            "CA35 repro seed=0x{seed:016X} tile_count={tile_count} dense_every={dense_every} mask_mode={mask_mode:?}",
            seed = self.seed,
            tile_count = self.tile_count,
            dense_every = self.dense_every,
            mask_mode = self.mask_mode,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ca35MaskMode {
    ExplicitAll,
    SkipOddTiles,
    EmptyMeansAll,
}

#[derive(Debug, Clone)]
struct Ca35Fixture {
    case: Ca35FuzzCase,
    schema: NeuralProjectionSchema,
    activations: Vec<f32>,
    previous_activations: Vec<f32>,
    oja: OjaUpdateConfig,
}

#[derive(Debug, Clone, Copy)]
struct Ca35Lcg {
    state: u64,
}

impl Ca35Lcg {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    fn range_u32(&mut self, min: u32, max_inclusive: u32) -> u32 {
        let span = max_inclusive.saturating_sub(min).saturating_add(1);
        min + (self.next_u32() % span)
    }

    fn signed_unit(&mut self) -> f32 {
        let raw = (self.next_u32() % 2001) as i32 - 1000;
        raw as f32 / 1000.0
    }

    fn bounded_weight(&mut self) -> f32 {
        self.signed_unit() * 0.5
    }

    fn nonnegative(&mut self) -> f32 {
        (self.next_u32() % 1001) as f32 / 1000.0
    }
}

#[test]
fn ca35_static_forward_property_fuzz_matches_cpu_reference() {
    for seed in CA35_STATIC_CASES {
        let fixture = ca35_fixture(*seed);
        let repro = fixture.case.repro();
        let policy = GpuFixedPointPolicy::reference();
        let upload = GpuUploadBuffers::from_cpu_schema(&fixture.schema, policy)
            .unwrap_or_else(|err| panic!("{repro}: upload failed: {err:?}"));
        let plan = GpuStaticForwardPlan::from_upload(&upload, policy)
            .unwrap_or_else(|err| panic!("{repro}: static plan failed: {err:?}"));
        let activation_q = plan
            .quantize_activations(&fixture.activations)
            .unwrap_or_else(|err| panic!("{repro}: activation quantization failed: {err:?}"));
        let gpu_oracle = plan
            .execute_cpu_diagnostic(&activation_q)
            .unwrap_or_else(|err| panic!("{repro}: fixed-point GPU oracle failed: {err:?}"));

        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let mut cpu_state = CpuNeuralState::for_brain_class(&spec).unwrap();
        cpu_state.activations.clone_from(&fixture.activations);
        let cpu_spmv = cpu_spmv_projection(
            &fixture.schema,
            &mut cpu_state,
            NeuralDiagnostics {
                accumulator_abs_limit: 1.0e6,
                effective_weight_abs_limit: 8.0,
            },
        )
        .unwrap_or_else(|err| panic!("{repro}: CPU reference SpMV failed: {err:?}"));
        finalize_cpu_activations(
            &mut cpu_state,
            NeuralActivationConfig {
                function: ActivationFunction::Identity,
                clamp_min: -1.0,
                clamp_max: 1.0,
                clear_accumulators: false,
            },
        )
        .unwrap_or_else(|err| panic!("{repro}: CPU reference finalize failed: {err:?}"));

        assert_eq!(
            gpu_oracle.diagnostics.active_tiles, cpu_spmv.active_tiles,
            "{repro}: active tile parity failed"
        );
        assert_eq!(
            gpu_oracle.diagnostics.mask_skipped_tiles, cpu_spmv.mask_skipped_tiles,
            "{repro}: mask skipped parity failed"
        );
        assert_eq!(
            gpu_oracle.diagnostics.active_synapses, cpu_spmv.active_synapses,
            "{repro}: active synapse parity failed"
        );

        let gpu_f32 = plan
            .dequantize_activations(&gpu_oracle.activations_q)
            .unwrap_or_else(|err| panic!("{repro}: activation dequantization failed: {err:?}"));
        assert_activation_close(&repro, &gpu_f32, &cpu_state.activations);
    }
}

#[test]
fn ca35_routing_property_fuzz_preserves_masked_static_outputs() {
    let policy = GpuFixedPointPolicy::reference();
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    for seed in CA35_ROUTING_CASES {
        let mut fixture = ca35_fixture(*seed);
        fixture.case.mask_mode = Ca35MaskMode::SkipOddTiles;
        apply_mask_mode(&mut fixture.schema, fixture.case.mask_mode);
        let repro = fixture.case.repro();

        let mut upload = GpuUploadBuffers::from_cpu_schema(&fixture.schema, policy)
            .unwrap_or_else(|err| panic!("{repro}: upload failed: {err:?}"));
        let routing = GpuRoutingMaskPlan::from_upload_and_brain_class(
            &upload,
            &spec,
            GpuActiveTileMaskConfig {
                tick_index: *seed,
                sensory_activity_present: true,
                biological_tile_budget: fixture.case.tile_count.saturating_sub(1).max(1),
                force_static_fixture_tiles: false,
            },
        )
        .unwrap_or_else(|err| panic!("{repro}: routing mask derivation failed: {err:?}"));
        assert!(
            routing.active_tiles < upload.tile_metadata.len() as u32,
            "{repro}: routing case must skip at least one tile"
        );
        upload.supertile_masks = routing.active_masks.clone();
        let masked_plan = GpuStaticForwardPlan::from_upload(&upload, policy)
            .unwrap_or_else(|err| panic!("{repro}: masked plan failed: {err:?}"));
        let activation_q = masked_plan
            .quantize_activations(&fixture.activations)
            .unwrap_or_else(|err| panic!("{repro}: activation quantization failed: {err:?}"));
        let masked = masked_plan
            .execute_cpu_diagnostic(&activation_q)
            .unwrap_or_else(|err| panic!("{repro}: masked static diagnostic failed: {err:?}"));
        let counters = masked_plan.routing_counters();

        assert_eq!(
            counters.active_tiles, masked.diagnostics.active_tiles,
            "{repro}: routing counter active tiles must match diagnostic"
        );
        assert_eq!(
            counters.skipped_microtiles, masked.diagnostics.mask_skipped_tiles,
            "{repro}: routing counter skipped tiles must match diagnostic"
        );
        assert!(
            masked.diagnostics.mask_skipped_tiles > 0,
            "{repro}: masked fuzz case did not exercise culling"
        );
    }
}

#[test]
fn ca35_plasticity_property_fuzz_matches_cpu_oja_reference() {
    for seed in CA35_PLASTICITY_CASES {
        let fixture = ca35_fixture(*seed);
        let repro = fixture.case.repro();
        let policy = GpuFixedPointPolicy::reference();
        let upload = GpuUploadBuffers::from_cpu_schema(&fixture.schema, policy)
            .unwrap_or_else(|err| panic!("{repro}: upload failed: {err:?}"));
        let plan = GpuPlasticityPlan::from_upload(
            &upload,
            policy,
            GpuOjaFixedPointConfig::from_oja_config(fixture.oja, policy, *seed as u32)
                .unwrap_or_else(|err| panic!("{repro}: Oja config failed: {err:?}")),
        )
        .unwrap_or_else(|err| panic!("{repro}: plasticity plan failed: {err:?}"));
        let previous_q = plan
            .quantize_activations(&fixture.previous_activations)
            .unwrap_or_else(|err| {
                panic!("{repro}: previous activation quantization failed: {err:?}")
            });
        let finalized_q = plan
            .quantize_activations(&fixture.activations)
            .unwrap_or_else(|err| {
                panic!("{repro}: finalized activation quantization failed: {err:?}")
            });
        let gpu_oracle = plan
            .execute_cpu_diagnostic(&previous_q, &finalized_q)
            .unwrap_or_else(|err| panic!("{repro}: fixed-point plasticity oracle failed: {err:?}"));

        let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
        let mut cpu_state = CpuNeuralState::for_brain_class(&spec).unwrap();
        cpu_state
            .previous_activations
            .clone_from(&fixture.previous_activations);
        cpu_state.activations.clone_from(&fixture.activations);
        let mut cpu_schema = fixture.schema.clone();
        let cpu_report = update_oja_shadow_traces(&mut cpu_schema, &cpu_state, fixture.oja)
            .unwrap_or_else(|err| panic!("{repro}: CPU Oja reference failed: {err:?}"));

        assert_eq!(
            gpu_oracle.diagnostics.active_tiles, cpu_report.active_tiles,
            "{repro}: plasticity active tile parity failed"
        );
        assert_eq!(
            gpu_oracle.diagnostics.mask_skipped_tiles, cpu_report.mask_skipped_tiles,
            "{repro}: plasticity mask skipped parity failed"
        );
        assert_eq!(
            gpu_oracle.diagnostics.active_synapses, cpu_report.active_synapses,
            "{repro}: plasticity active synapse parity failed"
        );
        assert_eq!(
            gpu_oracle.genetic_fixed_q, plan.genetic_fixed_q,
            "{repro}: genetic fixed layer changed"
        );
        assert_eq!(
            gpu_oracle.lifetime_consolidated_q, plan.lifetime_consolidated_q,
            "{repro}: lifetime-consolidated layer changed"
        );
        assert_eq!(
            gpu_oracle.h_operational_q, plan.h_operational_q,
            "{repro}: H_operational layer changed"
        );

        let cpu_reference_h_shadow_q = quantized_h_shadow(&cpu_schema, policy);
        let expected_h_shadow_q = plan
            .alpha_q16
            .iter()
            .zip(plan.h_shadow_initial_q.iter())
            .zip(cpu_reference_h_shadow_q.iter())
            .map(|((alpha, initial), cpu_reference)| {
                if *alpha == 0 {
                    *initial
                } else {
                    *cpu_reference
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(
            gpu_oracle.h_shadow_q.len(),
            expected_h_shadow_q.len(),
            "{repro}: H_shadow length drift"
        );
        for (index, (actual, expected)) in gpu_oracle
            .h_shadow_q
            .iter()
            .zip(expected_h_shadow_q.iter())
            .enumerate()
        {
            assert!(
                (i32::from(*actual) - i32::from(*expected)).abs() <= P26_PLASTICITY_TOLERANCE_Q,
                "{repro}: H_shadow parity failed at weight {index}: gpu-fixed={} cpu={}",
                actual,
                expected
            );
        }
    }
}

#[test]
fn ca35_fuzz_failure_repro_strings_are_stable_and_shrinkable() {
    let fixture = ca35_fixture(0xCA35_D00D);
    let repro = fixture.case.repro();
    assert!(repro.contains("seed=0x00000000CA35D00D"));
    assert!(repro.contains("tile_count="));
    assert!(repro.contains("mask_mode="));
}

fn ca35_fixture(seed: u64) -> Ca35Fixture {
    let mut rng = Ca35Lcg::new(seed);
    let tile_count = rng.range_u32(2, 4);
    let dense_every = rng.range_u32(2, 3);
    let mask_mode = match rng.range_u32(0, 2) {
        0 => Ca35MaskMode::ExplicitAll,
        1 => Ca35MaskMode::SkipOddTiles,
        _ => Ca35MaskMode::EmptyMeansAll,
    };
    let case = Ca35FuzzCase {
        seed,
        tile_count,
        dense_every,
        mask_mode,
    };
    let mut schema = random_schema(&mut rng, tile_count, dense_every);
    apply_mask_mode(&mut schema, mask_mode);
    let activations = random_activations(&mut rng);
    let previous_activations = random_activations(&mut rng);
    let oja = OjaUpdateConfig {
        learning_rate: 0.02 + rng.nonnegative() * 0.18,
        learning_rate_scale: 1.0,
        decay: 0.85 + rng.nonnegative() * 0.15,
        shadow_min: -1.0,
        shadow_max: 1.0,
    };
    Ca35Fixture {
        case,
        schema,
        activations,
        previous_activations,
        oja,
    }
}

fn random_schema(rng: &mut Ca35Lcg, tile_count: u32, dense_every: u32) -> NeuralProjectionSchema {
    let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
    let mut schema = NeuralProjectionSchema::empty_for_brain_class(&spec).unwrap();
    for tile_index in 0..tile_count {
        let coord = SparseTileCoord::new(tile_index, 0).unwrap();
        if tile_index.is_multiple_of(dense_every) {
            let mut dense = vec![SynapseWeightSplit::zero(); MICROTILE_CELLS];
            for entry in 0..4 {
                let local_target = rng.range_u32(0, MICROTILE_EDGE - 1) as usize;
                let local_source = rng.range_u32(0, MICROTILE_EDGE - 1) as usize;
                dense[local_target * MICROTILE_EDGE as usize + local_source] =
                    random_weights(rng, entry % 2 == 0);
            }
            schema.projections[0].tiles.push(ProjectionTile::new_dense(
                0,
                coord,
                alife_core::DenseTile::new(dense).unwrap(),
            ));
        } else {
            let mut entries = Vec::new();
            for entry_index in 0..4 {
                entries.push(
                    CooEntry::new(
                        rng.range_u32(0, MICROTILE_EDGE - 1) as u8,
                        rng.range_u32(0, MICROTILE_EDGE - 1) as u8,
                        random_weights(rng, entry_index % 2 == 0),
                    )
                    .unwrap(),
                );
            }
            schema.projections[0].tiles.push(ProjectionTile::new_coo(
                0,
                coord,
                CooTile::new(entries).unwrap(),
            ));
        }
    }
    schema.rebuild_supertile_masks();
    schema
}

fn random_weights(rng: &mut Ca35Lcg, plastic: bool) -> SynapseWeightSplit {
    SynapseWeightSplit::new(
        rng.bounded_weight(),
        rng.bounded_weight() * 0.5,
        if plastic {
            0.25 + rng.nonnegative() * 0.75
        } else {
            0.0
        },
        rng.bounded_weight() * 0.5,
        rng.bounded_weight() * 0.25,
    )
    .unwrap()
}

fn random_activations(rng: &mut Ca35Lcg) -> Vec<f32> {
    let mut activations = vec![0.0; 512];
    for value in activations.iter_mut().take(96) {
        *value = rng.signed_unit() * 0.75;
    }
    activations
}

fn apply_mask_mode(schema: &mut NeuralProjectionSchema, mask_mode: Ca35MaskMode) {
    match mask_mode {
        Ca35MaskMode::ExplicitAll => schema.rebuild_supertile_masks(),
        Ca35MaskMode::SkipOddTiles => {
            schema.rebuild_supertile_masks();
            for projection in &mut schema.projections {
                let active_bits = projection
                    .tiles
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| index % 2 == 0)
                    .fold(0_u64, |mask, (_, tile)| {
                        mask | (1_u64 << tile.metadata.coord.supertile_local_bit())
                    });
                if let Some(mask) = projection.supertile_masks.first_mut() {
                    mask.active_microtile_mask = active_bits;
                }
            }
        }
        Ca35MaskMode::EmptyMeansAll => {
            for projection in &mut schema.projections {
                projection.supertile_masks.clear();
            }
        }
    }
}

fn assert_activation_close(repro: &str, gpu: &[f32], cpu: &[f32]) {
    let tolerance = P25_STATIC_FORWARD_TOLERANCE_ABS * 4.0;
    for (index, (actual, expected)) in gpu.iter().zip(cpu.iter()).enumerate() {
        assert!(
            (*actual - *expected).abs() <= tolerance,
            "{repro}: activation parity failed at neuron {index}: gpu-fixed={actual} cpu={expected}"
        );
    }
}

fn quantized_h_shadow(schema: &NeuralProjectionSchema, policy: GpuFixedPointPolicy) -> Vec<i16> {
    let mut values = Vec::new();
    for projection in &schema.projections {
        for tile in &projection.tiles {
            match &tile.payload {
                SparseTilePayload::Dense(dense) => {
                    values.extend(
                        dense
                            .weights
                            .iter()
                            .map(|weights| policy.quantize_weight(weights.h_shadow).unwrap()),
                    );
                }
                SparseTilePayload::Coo(coo) => {
                    values.extend(
                        coo.entries
                            .iter()
                            .map(|entry| policy.quantize_weight(entry.weights.h_shadow).unwrap()),
                    );
                }
                SparseTilePayload::RowRunUnsupported | SparseTilePayload::ColumnRunUnsupported => {
                    unreachable!("CA35 fuzz fixture only emits supported tile formats")
                }
            }
        }
    }
    values
}
