//! Analytic oracles and executable production WGSL for credit-gated Oja.
//! The ignored device test permits a software Vulkan adapter for isolated shader
//! arithmetic. It is not a production backend or a gameplay learning claim.
use alife_core::{
    BrainCapacityClass, BrainGenome, CompiledSynapseKind, DevelopmentState, NormalizedScalar,
    PhenotypeCompiler, SensorProfile, Tick,
};
use alife_gpu_backend::CLOSED_LOOP_PLASTICITY_WGSL;
use std::collections::BTreeMap;

fn production_delta() -> &'static str {
    let source = CLOSED_LOOP_PLASTICITY_WGSL;
    let start = source.find("fn fast_plasticity_delta(").unwrap();
    let end = start + source[start..].find("\n}\n").unwrap() + 3;
    &source[start..end]
}

// Closed form for constant activity/eligibility/credit, independently derived
// in effective-weight space. Clipping is valid here because 0 <= h <= 1 makes
// each step monotone toward the fixed point (or an attainable endpoint).
fn expected(case: &Case, legacy: bool) -> f64 {
    let [eta, lambda, alpha, m, eligibility, post, base, fast, low, high, steps, _] =
        case.words.map(f64::from);
    let credit_scale = if legacy { 1.0 } else { m.abs() };
    let h = alpha * lambda * credit_scale * post * post;
    let drive = eta * alpha * alpha * m * eligibility;
    let initial = base + alpha * fast;
    let next = if h == 0.0 {
        initial + steps * drive
    } else {
        let fixed = drive / h;
        fixed + (initial - fixed) * (1.0 - h).powf(steps)
    };
    if alpha == 0.0 {
        base
    } else {
        next.clamp(base + alpha * low, base + alpha * high)
    }
}

struct Case {
    name: String,
    // eta, lambda, alpha, m, eligibility, post, genetic+lifetime, fast,
    // fast_min, fast_max, steps, padding; all exactly shared with WGSL.
    words: [f32; 12],
}

fn cases() -> Vec<Case> {
    let mut cases = Vec::new();
    for m in [0.0, 0.01, -0.01, 1.0, -1.0] {
        cases.push(Case {
            name: format!("cpu-coordination-vector/credit={m}"),
            words: [
                0.01, 0.001, 0.25, m, 0.4, 0.5, 0.75, 0.2, -2.0, 2.0, 1.0, 0.0,
            ],
        });
    }
    for (name, eta, lambda, alpha) in [
        ("decoder", 0.01, 0.001, 0.25),
        ("memory-decoder", 0.01, 0.001, 1.0),
        ("procedural-slow", 0.0005, 0.00005, 0.125),
        ("foundation-slow", 0.0025, 0.00025, 0.25),
        ("foundation-masked-slow", 0.00125, 0.000125, 0.125),
        ("foundation-masked-fast", 0.005, 0.0005, 0.125),
        ("fixed", 0.0, 0.0, 0.25),
        ("alpha-zero", 0.01, 0.001, 0.0),
    ] {
        for m in [0.0, 0.01, -0.01, 1.0, -1.0] {
            for steps in [1, 1_000, 10_000, 100_000] {
                cases.push(Case {
                    name: format!("{name}/credit={m}/steps={steps}"),
                    words: [
                        eta,
                        lambda,
                        alpha,
                        m,
                        0.0,
                        0.5,
                        0.75,
                        0.2,
                        -2.0,
                        2.0,
                        steps as f32,
                        0.0,
                    ],
                });
            }
        }
    }
    // Sustained learning toward interior fixed points under either sign and
    // small credit; plus clipping at both bounds and post=0 linear learning.
    for (name, m, e, post, alpha, steps) in [
        ("positive-fixed-point", 1.0, 0.01, 0.5, 0.25, 100_000),
        ("negative-fixed-point", -1.0, 0.01, 0.5, 0.25, 100_000),
        ("small-positive-learning", 0.01, 0.01, 0.5, 0.25, 100_000),
        ("small-negative-learning", -0.01, 0.01, 0.5, 0.25, 100_000),
        ("upper-clipping", 1.0, 1.0, 0.5, 0.25, 100_000),
        ("lower-clipping", -1.0, 1.0, 0.5, 0.25, 100_000),
        ("post-zero", 1.0, 0.01, 0.0, 0.25, 1_000),
        ("maximum-contraction", -1.0, 0.01, 1.0, 1.0, 100_000),
    ] {
        cases.push(Case {
            name: name.into(),
            words: [
                if name == "maximum-contraction" {
                    1.0
                } else {
                    0.01
                },
                if name == "maximum-contraction" {
                    1.0
                } else {
                    0.001
                },
                alpha,
                m,
                e,
                post,
                0.0,
                0.2,
                -2.0,
                2.0,
                steps as f32,
                0.0,
            ],
        });
    }
    // A large inherited/consolidated base cannot be canceled all the way to
    // zero by bounded fast weights; negative bases have the symmetric floor.
    for base in [-0.75, 0.75] {
        cases.push(Case {
            name: format!("clamp-floor/base={base}"),
            words: [
                0.01, 0.001, 0.25, 0.0, 0.4, 1.0, base, 0.0, -2.0, 2.0, 100_000.0, 0.0,
            ],
        });
    }
    for post in [0.1, 1.0] {
        for steps in [1_000, 10_000, 100_000] {
            cases.push(Case {
                name: format!("decoder-activity/post={post}/steps={steps}"),
                words: [
                    0.01,
                    0.001,
                    0.25,
                    0.0,
                    0.0,
                    post,
                    0.75,
                    0.2,
                    -2.0,
                    2.0,
                    steps as f32,
                    0.0,
                ],
            });
        }
    }
    cases
}

#[test]
fn complete_production_shader_validates_and_evaluator_uses_credit_gated_delta() {
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_PLASTICITY_WGSL).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    let evaluator = CLOSED_LOOP_PLASTICITY_WGSL
        .split("fn evaluate_fast_plasticity_synapse(")
        .nth(1)
        .unwrap()
        .split("fn evaluate_fast_plasticity_replay_span(")
        .next()
        .unwrap();
    assert!(evaluator.contains("let delta = fast_plasticity_delta("));
    assert!(evaluator.contains("clamp(fast+delta,fast_min,fast_max)"));
    let delta: String = production_delta().split_whitespace().collect();
    assert!(delta.contains("normalization_rate*abs(local_third_factor)*post*post*effective"));
}

#[test]
fn retention_oracle_accounts_for_fast_clipping_and_alpha() {
    for case in cases() {
        let actual = expected(&case, false);
        let [_, _, alpha, m, e, _, base, fast, low, high, _, _] = case.words;
        assert!(actual.is_finite());
        assert!(actual >= f64::from(base + alpha * low) - 1e-7);
        assert!(actual <= f64::from(base + alpha * high) + 1e-7);
        if m == 0.0 {
            assert!((actual - f64::from(base) - f64::from(alpha) * f64::from(fast)).abs() < 1e-7);
        }
        if e == 0.0 && m != 0.0 && alpha != 0.0 {
            let initial = f64::from(base) + f64::from(alpha) * f64::from(fast);
            assert!(actual.abs() <= initial.abs() + 1e-7);
        }
        if case.name.starts_with("clamp-floor") {
            assert!((expected(&case, true).abs() - 0.25).abs() < 1e-7);
            assert!((actual.abs() - 0.75).abs() < 1e-7);
        }
    }
}

#[test]
fn founder_profiles_have_zero_positive_negative_and_small_local_credit() {
    use alife_core::{ActionCandidateCreditProfileV1, NeuromodulatoryFrame};
    let genome = BrainGenome::scaffold(42, BrainCapacityClass::n512().id());
    for (name, profile, scale) in [
        (
            "general",
            genome.plasticity_parameters().receptor_profile(),
            3.9,
        ),
        (
            "signed-choice",
            ActionCandidateCreditProfileV1::SignedChoiceReadouts.receptor_profile(),
            3.7,
        ),
    ] {
        for (lanes, expected) in [
            ([0.0; 8], 0.0),
            ([0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], 0.0),
            ([0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0], 1.0 / scale),
            ([0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0], -1.0 / scale),
            ([0.0, 0.0, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0], 0.01 / scale),
        ] {
            let m = profile
                .project(&NeuromodulatoryFrame::try_new(lanes).unwrap())
                .unwrap();
            assert!((m - expected).abs() < 1e-7);
            println!("{name} lanes={lanes:?} local_credit={m}");
        }
    }
}

#[test]
fn default_founder_route_inventory_uses_actual_compiled_receptors() {
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        let genome = BrainGenome::scaffold(42, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let mut groups = BTreeMap::<_, usize>::new();
        for synapse in phenotype.synapses() {
            let receptor = phenotype.plasticity_receptors()[usize::from(synapse.receptor_index())];
            let kind = match synapse.kind() {
                CompiledSynapseKind::Recurrent => "recurrent",
                CompiledSynapseKind::Decoder(_) => "decoder",
            };
            *groups
                .entry((
                    kind,
                    synapse.alpha().to_bits(),
                    receptor.learning_rate().to_bits(),
                    receptor.normalization_rate().to_bits(),
                ))
                .or_default() += 1;
            assert_eq!(receptor.fast_weight_bounds(), (-2.0, 2.0));
        }
        assert!(groups
            .keys()
            .any(|(_, _, _, norm)| f32::from_bits(*norm) > 0.0));
        for ((kind, alpha, eta, lambda), count) in groups {
            println!(
                "N{} {kind} count={count} alpha={} eta={} lambda={}",
                phenotype.neuron_count(),
                f32::from_bits(alpha),
                f32::from_bits(eta),
                f32::from_bits(lambda)
            );
        }
    }
}

// This wrapper executes the exact production helper text, with the old formula
// as a separately named control. No Rust update implementation supplies results.
fn device_source() -> String {
    let delta = production_delta();
    let legacy = delta
        .replace("fast_plasticity_delta", "legacy_delta")
        .replace("abs(local_third_factor)", "1.0");
    format!(
        r#"
{delta}
{legacy}
struct Case {{ a:vec4<f32>, b:vec4<f32>, c:vec4<f32> }}
@group(0) @binding(0) var<storage,read> cases:array<Case>;
@group(0) @binding(1) var<storage,read_write> results:array<vec2<f32>>;
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) gid:vec3<u32>) {{
  if (gid.x >= arrayLength(&cases)) {{ return; }}
  let v = cases[gid.x];
  var fast = v.b.w;
  var old_fast = v.b.w;
  for (var step=0u; step<u32(v.c.z); step+=1u) {{
    let effective = v.b.z + v.a.z*fast;
    let old_effective = v.b.z + v.a.z*old_fast;
    fast = clamp(fast + fast_plasticity_delta(v.a.x,v.a.y,v.a.z,v.a.w,v.b.x,v.b.y,effective),v.c.x,v.c.y);
    old_fast = clamp(old_fast + legacy_delta(v.a.x,v.a.y,v.a.z,v.a.w,v.b.x,v.b.y,old_effective),v.c.x,v.c.y);
  }}
  results[gid.x] = vec2<f32>(v.b.z+v.a.z*fast,v.b.z+v.a.z*old_fast);
}}
"#
    )
}

#[test]
fn isolated_device_wrapper_validates() {
    let module = naga::front::wgsl::parse_str(&device_source()).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}

#[test]
#[ignore = "explicit isolated WGSL arithmetic run; permits software adapter, requires Vulkan"]
fn production_wgsl_retention_and_sustained_learning_match_closed_form() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = instance
            .enumerate_adapters(wgpu::Backends::VULKAN)
            .await
            .into_iter()
            .next()
            .expect("Vulkan adapter required for isolated shader test");
        println!("isolated WGSL adapter: {:?}", adapter.get_info());
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("isolated-oja-test"),
                required_limits: adapter.limits(),
                ..Default::default()
            })
            .await
            .unwrap();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("production-oja-arithmetic"),
            source: wgpu::ShaderSource::Wgsl(device_source().into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("production-oja-arithmetic"),
            layout: None,
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let cases = cases();
        let words = cases.iter().flat_map(|c| c.words).collect::<Vec<_>>();
        use wgpu::util::DeviceExt;
        let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("oja-cases"),
            contents: bytemuck::cast_slice(&words),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bytes = (cases.len() * 8) as u64;
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oja-results"),
            size: bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("oja-readback"),
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("oja-bindings"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(cases.len() as u32, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, bytes);
        queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap()
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();
        let mapped = readback.slice(..).get_mapped_range();
        let values: &[f32] = bytemuck::cast_slice(&mapped);
        let (results, remainder) = values.as_chunks::<2>();
        assert!(remainder.is_empty());
        let mut max_abs_error = 0.0_f64;
        for (case, result) in cases.iter().zip(results) {
            for (actual, legacy) in [(result[0], false), (result[1], true)] {
                let oracle = expected(case, legacy);
                max_abs_error = max_abs_error.max((f64::from(actual) - oracle).abs());
                // Long f32 recurrences stagnate below one ULP per fast update.
                // The tolerance covers accumulation over 100k steps, not a
                // change in the fixed point or clipping endpoints.
                let tolerance = if case.words[10] == 1.0 { 2e-7 } else { 0.0008 };
                assert!(
                    (f64::from(actual) - oracle).abs() < tolerance,
                    "{} legacy={legacy}: actual={actual} oracle={oracle}",
                    case.name
                );
            }
            if case.words[3] == 0.0 {
                assert_eq!(
                    result[0],
                    case.words[6] + case.words[2] * case.words[7],
                    "zero-credit retention: {}",
                    case.name
                );
            }
            if case.name.contains("steps=100000")
                || !case.name.contains("/credit=")
                || case.name.starts_with("cpu-coordination-vector")
                || (case.words[3] == 0.0 && case.name.starts_with("decoder/"))
            {
                println!("{} before={} after={}", case.name, result[1], result[0]);
            }
        }
        println!(
            "{} production WGSL/control vectors passed; max_abs_error={max_abs_error}",
            cases.len()
        );
    });
}
