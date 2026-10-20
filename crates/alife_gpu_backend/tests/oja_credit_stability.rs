//! Analytic oracles and executable production WGSL for credit-gated Oja.
//! The ignored device test permits a software Vulkan adapter for isolated shader
//! arithmetic. It is not a production backend or a gameplay learning claim.
use alife_core::{
    BrainCapacityClass, BrainGenome, CompiledSynapseKind, DevelopmentState, NormalizedScalar,
    PhenotypeCompiler, SensorProfile, Tick,
};
use alife_gpu_backend::CLOSED_LOOP_PLASTICITY_WGSL;
use std::collections::BTreeMap;

#[path = "oja_credit_stability/causal_kernels.rs"]
mod causal_kernels;

fn production_delta() -> &'static str {
    let source = CLOSED_LOOP_PLASTICITY_WGSL;
    let start = source.find("fn fast_plasticity_delta(").unwrap();
    let end = start + source[start..].find("\n}\n").unwrap() + 3;
    &source[start..end]
}

// Closed form for constant activity/eligibility/credit, independently derived
// in effective-weight space. Clipping is valid here because 0 <= h <= 1 makes
// each step monotone toward the fixed point (or an attainable endpoint).
fn expected(case: &Case, rule: usize) -> f64 {
    if case.words[11] == 1.0 {
        let mut first = case.clone();
        first.words[10] = (case.words[10] / 2.0).floor();
        first.words[11] = 0.0;
        let midpoint = expected(&first, rule);
        let mut second = first.clone();
        second.words[10] = case.words[10] - first.words[10];
        second.words[4] = -case.words[4] * 0.95_f32.powi(20);
        second.words[7] = ((midpoint - f64::from(case.words[6])) / f64::from(case.words[2])) as f32;
        return expected(&second, rule);
    }
    let [eta, lambda, alpha, m, eligibility, post, base, fast, low, high, steps, _] =
        case.words.map(f64::from);
    // 0 = synapse credit; 1 = original; 2 = receptor-only PR;
    // 3 = common clock; 4 = exact-nonzero eligibility gate.
    let credit_scale = match rule {
        0 | 3 => (m * eligibility).abs(),
        1 => 1.0,
        2 => m.abs(),
        4 => {
            if eligibility == 0.0 {
                0.0
            } else {
                m.abs()
            }
        }
        _ => unreachable!(),
    };
    let h = alpha * lambda * credit_scale * post * post;
    let drive =
        eta * alpha * alpha * m * eligibility * if rule == 3 { eligibility.abs() } else { 1.0 };
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

#[derive(Clone)]
struct Case {
    name: String,
    // eta, lambda, alpha, m, eligibility, post, genetic+lifetime, fast,
    // fast_min, fast_max, steps, reversal flag; exactly shared with WGSL.
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
    for m in [-1.0, 1.0] {
        cases.push(Case {
            name: format!("delayed-trace-reversal/m={m}"),
            words: [
                0.01, 0.001, 0.25, m, 0.4, 0.5, 0.0, 0.2, -2.0, 2.0, 20_000.0, 1.0,
            ],
        });
    }
    // Sustained learning toward interior fixed points under either sign and
    // small credit; plus clipping at both bounds and post=0 linear learning.
    for (name, m, e, post, alpha, steps) in [
        ("default-positive-drive", 1.0, 0.01, 0.5, 0.25, 100_000),
        ("default-negative-drive", -1.0, 0.01, 0.5, 0.25, 100_000),
        ("small-positive-learning", 0.01, 0.01, 0.5, 0.25, 100_000),
        ("small-negative-learning", -0.01, 0.01, 0.5, 0.25, 100_000),
        ("upper-clipping", 1.0, 1.0, 0.5, 0.25, 100_000),
        ("lower-clipping", -1.0, 1.0, 0.5, 0.25, 100_000),
        ("post-zero", 1.0, 0.01, 0.0, 0.25, 1_000),
        ("maximum-contraction", -1.0, 1.0, 1.0, 1.0, 100_000),
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
    for m in [-1.0, -0.01, 0.01, 1.0] {
        for e in [-0.4, 0.4] {
            cases.push(Case {
                name: format!("interior-fixed-point/m={m}/e={e}"),
                words: [
                    0.0001, 0.001, 1.0, m, e, 1.0, 0.0, 0.2, -2.0, 2.0, 100_000.0, 0.0,
                ],
            });
        }
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
    // Causal follow-up: outcome projection is present while per-synapse
    // eligibility can be absent, tiny, signed, canceled, or persisted on a
    // masked route. The production accumulator decays existing eligibility
    // on masked routes; current coactivation is added only on enabled ones.
    let genome = BrainGenome::scaffold(42, BrainCapacityClass::n512().id());
    let profile = genome.plasticity_parameters().receptor_profile();
    for (event, lanes) in [
        ("novelty", [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]),
        ("pain", [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
        ("frustration", [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0]),
        ("mixed", [0.0, 0.1, 0.0, 0.2, 0.5, 0.0, 0.0, 0.0]),
    ] {
        let m = profile
            .project(&alife_core::NeuromodulatoryFrame::try_new(lanes).unwrap())
            .unwrap();
        for (route, enabled, previous, pre) in [
            ("active-zero", true, 0.0_f32, 0.0_f32),
            ("masked-zero", false, 0.0, 0.8),
            ("active-tiny", true, 0.0, 0.000002),
            ("active-tiny-negative", true, 0.0, -0.000002),
            ("active-positive", true, 0.0, 0.02),
            ("active-negative", true, 0.0, -0.02),
            ("masked-persisted-positive", false, 0.4, 0.8),
            ("masked-persisted-negative", false, -0.4, 0.8),
            ("active-canceled", true, 0.4, -0.76),
        ] {
            let staged = (0.95 * previous + if enabled { pre * 0.5 } else { 0.0 }).clamp(-1.0, 1.0);
            let eligibility = staged * 0.95_f32.powi(3);
            for steps in [1, 100_000] {
                cases.push(Case {
                    name: format!("causal/{event}/{route}/m={m}/e={eligibility}/steps={steps}"),
                    words: [
                        0.01,
                        0.001,
                        0.25,
                        m,
                        eligibility,
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
    for (name, eta, lambda, alpha) in [
        ("decoder", 0.01, 0.001, 0.25),
        ("memory-decoder", 0.01, 0.001, 1.0),
        ("procedural-slow", 0.0005, 0.00005, 0.125),
    ] {
        for p in [0.1, 0.5, 1.0] {
            for e in [0.0, 0.0000001, 0.000001, 0.0001] {
                cases.push(Case {
                    name: format!("sustained-noise/{name}/p={p}/e={e}"),
                    words: [
                        eta,
                        lambda,
                        alpha,
                        0.2 / 3.9,
                        e,
                        p,
                        0.75,
                        0.2,
                        -2.0,
                        2.0,
                        1_000_000.0,
                        0.0,
                    ],
                });
            }
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
    assert!(delta.contains("letsigned_credit=local_third_factor*credited_eligibility;"));
    assert!(delta.contains("normalization_rate*abs(signed_credit)*post*post*effective"));
}

#[test]
fn retention_oracle_accounts_for_fast_clipping_and_alpha() {
    for case in cases() {
        let actual = expected(&case, 0);
        let [_, _, alpha, m, e, _, base, fast, low, high, _, _] = case.words;
        assert!(actual.is_finite());
        assert!(actual >= f64::from(base + alpha * low) - 1e-7);
        assert!(actual <= f64::from(base + alpha * high) + 1e-7);
        if m == 0.0 || e == 0.0 {
            assert!((actual - f64::from(base) - f64::from(alpha) * f64::from(fast)).abs() < 1e-7);
        }
        if e == 0.0 && m != 0.0 && alpha != 0.0 {
            let initial = f64::from(base) + f64::from(alpha) * f64::from(fast);
            assert!(actual.abs() <= initial.abs() + 1e-7);
        }
        if case.name.starts_with("clamp-floor") {
            assert!((expected(&case, 1).abs() - 0.25).abs() < 1e-7);
            assert!((actual.abs() - 0.75).abs() < 1e-7);
        }
    }
}

#[test]
fn tiny_noise_real_valued_clamp_times_are_distinct_from_f32_stagnation() {
    for case in cases().into_iter().filter(|case| {
        case.name.starts_with("sustained-noise/") && case.words[4] == 1e-6 && case.words[5] == 0.5
    }) {
        let [eta, lambda, alpha, m, e, p, base, fast, _, high, _, _] = case.words.map(f64::from);
        let target = eta * alpha / (lambda * p * p);
        let initial = base + alpha * fast;
        let upper = base + alpha * high;
        let contraction = alpha * lambda * (m * e).abs() * p * p;
        let steps = (((target - upper) / (target - initial)).ln() / (-contraction).ln_1p()).ceil();
        assert!(target > upper);
        assert!(steps > 1e9);
        println!(
            "{} E_star={target} upper={upper} real_steps_to_upper={steps} exact_gate_E_star={}",
            case.name,
            target * e
        );
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

// Exact production helper plus separately named original, receptor-only, and
// common-clock controls. No Rust update implementation supplies device results.
fn device_source() -> String {
    let delta = production_delta();
    format!(
        r#"
{delta}
fn original_delta(eta:f32, lambda:f32, alpha:f32, m:f32, e:f32, p:f32, w:f32) -> f32 {{
  return eta*alpha*m*e - lambda*p*p*w;
}}
fn receptor_only_delta(eta:f32, lambda:f32, alpha:f32, m:f32, e:f32, p:f32, w:f32) -> f32 {{
  return eta*alpha*m*e - lambda*abs(m)*p*p*w;
}}
fn common_clock_delta(eta:f32, lambda:f32, alpha:f32, m:f32, e:f32, p:f32, w:f32) -> f32 {{
  return abs(e)*(eta*alpha*m*e - lambda*abs(m)*p*p*w);
}}
fn exact_gate_delta(eta:f32, lambda:f32, alpha:f32, m:f32, e:f32, p:f32, w:f32) -> f32 {{
  if (e == 0.0) {{ return 0.0; }}
  return eta*alpha*m*e - lambda*abs(m)*p*p*w;
}}
struct Case {{ a:vec4<f32>, b:vec4<f32>, c:vec4<f32> }}
struct Result {{ effective:vec4<f32>, delta:vec4<f32>, exact_gate:vec4<f32> }}
@group(0) @binding(0) var<storage,read> cases:array<Case>;
@group(0) @binding(1) var<storage,read_write> results:array<Result>;
@compute @workgroup_size(1)
fn main(@builtin(global_invocation_id) gid:vec3<u32>) {{
  if (gid.x >= arrayLength(&cases)) {{ return; }}
  let v = cases[gid.x];
  var fast = vec4<f32>(v.b.w);
  var first_delta = vec4<f32>(0.0);
  var gate_fast = v.b.w;
  var first_gate_delta = 0.0;
  var midpoint = 0.0;
  var reversal_delta = 0.0;
  for (var step=0u; step<u32(v.c.z); step+=1u) {{
    let w = vec4<f32>(v.b.z) + v.a.z*fast;
    var e = v.b.x;
    if (v.c.w == 1.0 && step >= u32(v.c.z)/2u) {{ e = -e*pow(0.95,20.0); }}
    let updates = vec4<f32>(
      fast_plasticity_delta(v.a.x,v.a.y,v.a.z,v.a.w,e,v.b.y,w.x),
      original_delta(v.a.x,v.a.y,v.a.z,v.a.w,e,v.b.y,w.y),
      receptor_only_delta(v.a.x,v.a.y,v.a.z,v.a.w,e,v.b.y,w.z),
      common_clock_delta(v.a.x,v.a.y,v.a.z,v.a.w,e,v.b.y,w.w));
    if (step == 0u) {{ first_delta = updates; }}
    if (step == u32(v.c.z)/2u) {{ midpoint = w.x; reversal_delta = updates.x; }}
    let gate_delta = exact_gate_delta(v.a.x,v.a.y,v.a.z,v.a.w,e,v.b.y,v.b.z+v.a.z*gate_fast);
    if (step == 0u) {{ first_gate_delta = gate_delta; }}
    fast = clamp(fast+updates,vec4<f32>(v.c.x),vec4<f32>(v.c.y));
    gate_fast = clamp(gate_fast+gate_delta,v.c.x,v.c.y);
  }}
  results[gid.x] = Result(vec4<f32>(v.b.z)+v.a.z*fast,first_delta,vec4<f32>(v.b.z+v.a.z*gate_fast,first_gate_delta,midpoint,reversal_delta));
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
        let bytes = (cases.len() * 48) as u64;
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
        let (results, remainder) = values.as_chunks::<12>();
        assert!(remainder.is_empty());
        let mut max_abs_error = 0.0_f64;
        let mut max_standard_error = 0.0_f64;
        let mut max_noise_error = 0.0_f64;
        for (case, result) in cases.iter().zip(results) {
            for (rule, actual) in [result[0], result[1], result[2], result[3], result[8]]
                .into_iter()
                .enumerate()
            {
                let lower = case.words[6] + case.words[2] * case.words[8];
                let upper = case.words[6] + case.words[2] * case.words[9];
                assert!(
                    actual.is_finite() && actual >= lower - 2e-7 && actual <= upper + 2e-7,
                    "finite fast-clipped effective strength: {} rule={rule}",
                    case.name
                );
                let oracle = expected(case, rule);
                max_abs_error = max_abs_error.max((f64::from(actual) - oracle).abs());
                if case.name.starts_with("sustained-noise/") {
                    max_noise_error = max_noise_error.max((f64::from(actual) - oracle).abs());
                } else {
                    max_standard_error = max_standard_error.max((f64::from(actual) - oracle).abs());
                }
                // Long f32 recurrences stagnate below one ULP per fast update.
                // The million-step noise group deliberately exposes accumulated
                // rounding/stagnation against the real-valued oracle; zero
                // credit and eligibility still require exact device retention.
                let tolerance = if case.words[10] == 1.0 {
                    2e-7
                } else if case.name.starts_with("sustained-noise/") {
                    0.02
                } else {
                    0.0008
                };
                assert!(
                    (f64::from(actual) - oracle).abs() < tolerance,
                    "{} rule={rule}: actual={actual} oracle={oracle}",
                    case.name
                );
            }
            if case.words[3] == 0.0 || case.words[4] == 0.0 {
                assert_eq!(result[4], 0.0, "zero local credit delta: {}", case.name);
                assert_eq!(
                    result[0],
                    case.words[6] + case.words[2] * case.words[7],
                    "zero-credit retention: {}",
                    case.name
                );
            }
            if case.words[4] != 0.0 {
                assert_eq!(
                    result[8], result[2],
                    "exact-nonzero gate equals receptor-only rule for {}",
                    case.name
                );
            }
            if case.name.starts_with("sustained-noise/") {
                assert!(
                    result[0] >= case.words[6] + case.words[2] * case.words[7],
                    "tiny positive evidence must not cause unrelated cancellation: {}",
                    case.name
                );
            }
            if case.name.starts_with("delayed-trace-reversal/") {
                assert!(
                    result[10] * result[0] < 0.0,
                    "delayed reversal crosses sign: {}",
                    case.name
                );
                assert!(
                    result[10] * result[11] < 0.0,
                    "contraction does not reverse with eligibility sign: {}",
                    case.name
                );
                println!(
                    "{} midpoint={} reversal_delta={} final={}",
                    case.name, result[10], result[11], result[0]
                );
            }
            if case.name.contains("steps=100000")
                || !case.name.contains("/credit=")
                || case.name.starts_with("cpu-coordination-vector")
                || (case.words[3] == 0.0 && case.name.starts_with("decoder/"))
            {
                println!("{} original={} receptor_only={} synapse_credit={} common_clock={} exact_gate={} delta_synapse={} delta_receptor={} delta_clock={} delta_gate={}", case.name, result[1], result[2], result[0], result[3], result[8], result[4], result[6], result[7], result[9]);
            }
        }
        println!(
            "{} production WGSL/control vectors passed; max_abs_error={max_abs_error}; max_standard_error={max_standard_error}; max_noise_error={max_noise_error}",
            cases.len()
        );
    });
}
