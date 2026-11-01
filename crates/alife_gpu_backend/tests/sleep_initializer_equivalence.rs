//! Falsification oracle for the initializer-only compiler-pressure experiment.
//! The old shader is frozen from b9f78ed0, not rebuilt from candidate predicates.
//! Software execution is explicit and never hardware/NVIDIA qualification.
use std::collections::BTreeSet;

const ABI: &str = include_str!("../shaders/closed_loop_abi.wgsl");
const OLD: &str = include_str!("fixtures/sleep_initializer_legacy_b9f78ed0.wgsl");
const NEW: &str = include_str!("../shaders/closed_loop_consolidate.wgsl");
const STATE_WORDS: usize = 256;
const COMPLETION: usize = 192;

fn module(body: &str) -> naga::Module {
    let source = format!("{ABI}{body}");
    let module = naga::front::wgsl::parse_str(&source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    module
}

fn calls(block: &naga::Block, found: &mut Vec<naga::Handle<naga::Function>>) {
    for statement in block.iter() {
        match statement {
            naga::Statement::Call { function, .. } => {
                if !found.contains(function) {
                    found.push(*function);
                }
            }
            naga::Statement::Block(block) => calls(block, found),
            naga::Statement::If { accept, reject, .. } => {
                calls(accept, found);
                calls(reject, found);
            }
            naga::Statement::Switch { cases, .. } => {
                for case in cases {
                    calls(&case.body, found);
                }
            }
            naga::Statement::Loop {
                body, continuing, ..
            } => {
                calls(body, found);
                calls(continuing, found);
            }
            _ => {}
        }
    }
}

fn reachable(module: &naga::Module) -> Vec<String> {
    let entry = module
        .entry_points
        .iter()
        .find(|entry| entry.name == "initialize_sleep_transaction")
        .unwrap();
    let mut found = Vec::new();
    calls(&entry.function.body, &mut found);
    let mut index = 0;
    while index < found.len() {
        calls(&module.functions[found[index]].body, &mut found);
        index += 1;
    }
    found
        .into_iter()
        .map(|handle| module.functions[handle].name.clone().unwrap())
        .collect()
}

#[test]
fn both_initializer_variants_validate_and_candidate_avoids_full_request_copy() {
    let old = module(OLD);
    let new = module(NEW);
    assert!(reachable(&old)
        .iter()
        .any(|name| name == "load_consolidation_request"));
    assert!(!reachable(&new)
        .iter()
        .any(|name| name == "load_consolidation_request"));
    // Other entrypoints continue to use the original full validator and ABI.
    assert!(new
        .functions
        .iter()
        .any(|(_, f)| f.name.as_deref() == Some("sleep_transaction_valid")));
}

#[derive(Clone)]
struct Case {
    name: String,
    brain: [u32; 36],
    identity: [u32; 8],
    plan: [u32; 64],
    header: [u32; 20],
    frame: [u32; 128],
    state: [u32; STATE_WORDS],
    prepared: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
enum Region {
    Header,
    Brain,
    Extension,
    Learning,
    Request,
    Parameters,
    Identity,
}
const REGIONS: [(Region, usize); 7] = [
    (Region::Header, 20),
    (Region::Brain, 36),
    (Region::Extension, 20),
    (Region::Learning, 24),
    (Region::Request, 44),
    (Region::Parameters, 8),
    (Region::Identity, 8),
];

impl Case {
    fn valid(weight_bank: u32, eligibility_bank: u32) -> Self {
        let mut case = Self {
            name: format!("valid-bank-{weight_bank}-{eligibility_bank}"),
            brain: [0; 36],
            identity: std::array::from_fn(|i| i as u32 + 11),
            plan: [0; 64],
            header: [0; 20],
            frame: [0x5a5a_4321; 128],
            state: [0xa5a5_1234; STATE_WORDS],
            prepared: Some(true),
        };
        case.brain[0] = 4;
        case.brain[1] = 1;
        case.brain[3] = 7;
        case.brain[4] = 512;
        case.brain[5] = 2;
        case.brain[6] = 4;
        case.brain[7] = 2;
        for (field, value) in [(23, 104), (24, 108), (25, 120), (26, 124), (32, 0)] {
            case.brain[field] = value;
        }
        case.state[..20].fill(0);
        for (field, value) in [
            (0, 4),
            (8, 122),
            (9, 126),
            (10, 116),
            (11, 112),
            (12, 0),
            (15, 32),
            (16, 64),
        ] {
            case.state[field] = value;
        }
        case.state[32..56].fill(0);
        for (field, value) in [
            (0, 3),
            (1, weight_bank),
            (2, eligibility_bank),
            (4, 5),
            (5, 2),
            (13, 1),
            (14, 1),
            (15, 1),
            (16, 1),
            (17, 128),
            (18, 156),
            (19, 160),
        ] {
            case.state[32 + field] = value;
        }
        case.plan[..8].copy_from_slice(&[
            1,
            0.25_f32.to_bits(),
            2.0_f32.to_bits(),
            0.5_f32.to_bits(),
            1,
            1,
            0,
            0,
        ]);
        case.header.copy_from_slice(&[
            1,
            1,
            0,
            7,
            0,
            0,
            44,
            1,
            72,
            1,
            76,
            1,
            4,
            COMPLETION as u32,
            17,
            9,
            23,
            11,
            0,
            0,
        ]);
        case.frame[0..4].copy_from_slice(&[1, 0, 23, 11]);
        case.frame[4..12].copy_from_slice(&case.identity);
        case.frame[12..16].copy_from_slice(&[5, 2, 6, 2]);
        case.frame[32..34].copy_from_slice(&[1, 1]);
        case.frame[42..44].fill(0);
        case
    }

    fn region(&mut self, region: Region) -> &mut [u32] {
        match region {
            Region::Header => &mut self.header,
            Region::Brain => &mut self.brain,
            Region::Extension => &mut self.state[..20],
            Region::Learning => &mut self.state[32..56],
            Region::Request => &mut self.frame[..44],
            Region::Parameters => &mut self.plan[..8],
            Region::Identity => &mut self.identity,
        }
    }
}

fn corpus() -> Vec<Case> {
    let base = Case::valid(0, 0);
    let mut cases = Vec::new();
    for weight in 0..2 {
        for eligibility in 0..2 {
            cases.push(Case::valid(weight, eligibility));
        }
    }
    for (region, count) in REGIONS {
        for field in 0..count {
            let original = base.clone().region(region)[field];
            // Every bit of every field, including unused digest/reserved fields.
            let values: BTreeSet<u32> = (0..32)
                .map(|bit| original ^ (1 << bit))
                .chain([
                    0,
                    1,
                    2,
                    127,
                    128,
                    129,
                    255,
                    256,
                    257,
                    u32::MAX,
                    u32::MAX / 28,
                    u32::MAX / 28 + 1,
                    u32::MAX / 4,
                    u32::MAX / 4 + 1,
                    0x8000_0000,
                    0x7f80_0000,
                    0xff80_0000,
                    0x7fc0_0001,
                    0x3f80_0001,
                ])
                .filter(|value| *value != original)
                .collect();
            for value in values {
                let mut changed = base.clone();
                changed.name = format!("{region:?}[{field}]={value:08x}");
                changed.region(region)[field] = value;
                changed.prepared = None;
                cases.push(changed);
            }
        }
    }
    let mut zero_output = base.clone();
    zero_output.name = "both-output-generation-halves-zero".into();
    zero_output.frame[14..16].fill(0);
    zero_output.prepared = Some(false);
    cases.push(zero_output);
    // Matched multi-field changes retain nonzero high generations and test both
    // possible bank selections without assuming output == input + 1 in WGSL.
    for (low, high) in [(0, u32::MAX), (u32::MAX, 0), (u32::MAX, u32::MAX)] {
        let mut changed = base.clone();
        changed.name = format!("matched-generations-{low}-{high}");
        changed.state[36..38].copy_from_slice(&[low, high]);
        changed.frame[12..14].copy_from_slice(&[low, high]);
        changed.frame[14..16].copy_from_slice(&[high, low]);
        cases.push(changed);
    }
    cases
}

#[test]
fn corpus_covers_all_record_words_bits_and_both_bank_rules() {
    assert_eq!(REGIONS.iter().map(|(_, count)| count).sum::<usize>(), 160);
    let cases = corpus();
    for (region, count) in REGIONS {
        for field in 0..count {
            let base = Case::valid(0, 0).region(region)[field];
            for bit in 0..32 {
                let expected = format!("{region:?}[{field}]={:08x}", base ^ (1 << bit));
                assert!(
                    cases.iter().any(|case| case.name == expected),
                    "missing {expected}"
                );
            }
        }
    }
    eprintln!(
        "sleep_initializer_corpus_cases={} fields=160 bit_flips_per_field=32",
        cases.len()
    );
}

#[cfg(feature = "gpu-tests")]
mod software {
    use super::*;
    use wgpu::util::DeviceExt;

    #[test]
    #[ignore = "explicit CPU Vulkan old/new initializer equivalence; not RTX qualification"]
    fn software_vulkan_initializer_matches_frozen_legacy_for_all_corruptions() {
        pollster::block_on(async {
            let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
            descriptor.backends = wgpu::Backends::VULKAN;
            let instance = wgpu::Instance::new(descriptor);
            let adapter = instance
                .enumerate_adapters(wgpu::Backends::VULKAN)
                .await
                .into_iter()
                .find(|adapter| adapter.get_info().device_type == wgpu::DeviceType::Cpu)
                .expect("explicit software fixture requires an actual CPU Vulkan adapter");
            let info = adapter.get_info();
            assert_eq!(info.backend, wgpu::Backend::Vulkan);
            eprintln!("sleep_initializer_equivalence_adapter={info:?}");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("sleep-initializer-explicit-software-oracle"),
                    required_features: wgpu::Features::empty(),
                    required_limits: adapter.limits(),
                    experimental_features: wgpu::ExperimentalFeatures::disabled(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                    trace: wgpu::Trace::Off,
                })
                .await
                .unwrap();
            let entries: Vec<_> = (0..7)
                .map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage {
                            read_only: binding < 5,
                        },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                })
                .collect();
            let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sleep-oracle-unchanged-seven-buffer-ABI"),
                entries: &entries,
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("sleep-oracle-layout"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let pipeline = |label, body: &str| {
                let source = format!("{ABI}{body}");
                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(label),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });
                device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some(label),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: Some("initialize_sleep_transaction"),
                    compilation_options: Default::default(),
                    cache: None,
                })
            };
            let started = std::time::Instant::now();
            let old = pipeline("frozen-main-initializer", OLD);
            let old_seconds = started.elapsed().as_secs_f64();
            let started = std::time::Instant::now();
            let new = pipeline("staged-direct-offset-initializer", NEW);
            eprintln!("software_pipeline_create_seconds old={old_seconds:.6} candidate={:.6} not_NVIDIA_timing",started.elapsed().as_secs_f64());
            // A deliberately weakened bank guard must be falsified by this
            // corpus; otherwise old/new equality could be a vacuous harness pass.
            let weakened = NEW.replacen(
                "if (!(learning.active_weight_bank <= 1u))",
                "if (!(learning.active_weight_bank <= 2u))",
                1,
            );
            assert_ne!(weakened, NEW, "negative-control mutation must be applied");
            let negative = pipeline("negative-control-weakened-bank-guard", &weakened);
            let cases = corpus();
            let initial = Case::valid(0, 0);
            let arrays: [&[u32]; 7] = [
                &initial.brain,
                &initial.identity,
                &initial.plan,
                &[0],
                &initial.header,
                &initial.frame,
                &initial.state,
            ];
            let buffers: Vec<_> = arrays
                .iter()
                .map(|words| {
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("sleep-oracle-fixture"),
                        contents: bytemuck::cast_slice(words),
                        usage: wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::COPY_DST
                            | wgpu::BufferUsages::COPY_SRC,
                    })
                })
                .collect();
            let group_entries: Vec<_> = buffers
                .iter()
                .enumerate()
                .map(|(binding, buffer)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: buffer.as_entire_binding(),
                })
                .collect();
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sleep-oracle-bindings"),
                layout: &layout,
                entries: &group_entries,
            });
            let state_bytes = (STATE_WORDS * 4) as u64;
            let reset = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sleep-oracle-reset"),
                size: state_bytes,
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let output = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sleep-oracle-all-results"),
                size: state_bytes * 3 * cases.len() as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            for (index, case) in cases.iter().enumerate() {
                let data: [&[u32]; 7] = [
                    &case.brain,
                    &case.identity,
                    &case.plan,
                    &[0],
                    &case.header,
                    &case.frame,
                    &case.state,
                ];
                for (buffer, words) in buffers.iter().zip(data) {
                    queue.write_buffer(buffer, 0, bytemuck::cast_slice(words));
                }
                queue.write_buffer(&reset, 0, bytemuck::cast_slice(&case.state));
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("sleep-oracle-case"),
                });
                for (variant, pipeline) in [&old, &new, &negative].into_iter().enumerate() {
                    encoder.copy_buffer_to_buffer(&reset, 0, &buffers[6], 0, state_bytes);
                    {
                        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("sleep-oracle-initialize"),
                            timestamp_writes: None,
                        });
                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, &group, &[]);
                        pass.dispatch_workgroups(1, 1, 1);
                    }
                    encoder.copy_buffer_to_buffer(
                        &buffers[6],
                        0,
                        &output,
                        (index as u64 * 3 + variant as u64) * state_bytes,
                        state_bytes,
                    );
                }
                queue.submit(Some(encoder.finish()));
                if index % 128 == 127 {
                    device
                        .poll(wgpu::PollType::Wait {
                            submission_index: None,
                            timeout: None,
                        })
                        .unwrap();
                }
            }
            let (sender, receiver) = std::sync::mpsc::channel();
            output
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    sender.send(result).unwrap();
                });
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .unwrap();
            receiver.recv().unwrap().unwrap();
            let mapped = output.slice(..).get_mapped_range();
            let words: &[u32] = bytemuck::cast_slice(&mapped);
            let mut negative_control_differences = 0;
            for (index, case) in cases.iter().enumerate() {
                let old = &words[(index * 3) * STATE_WORDS..(index * 3 + 1) * STATE_WORDS];
                let new = &words[(index * 3 + 1) * STATE_WORDS..(index * 3 + 2) * STATE_WORDS];
                let weakened = &words[(index * 3 + 2) * STATE_WORDS..(index * 3 + 3) * STATE_WORDS];
                negative_control_differences += usize::from(old != weakened);
                assert_eq!(
                    old, new,
                    "case {} changed initializer state/completion writes",
                    case.name
                );
                if let Some(prepared) = case.prepared {
                    assert_eq!(
                        old[COMPLETION + 3],
                        if prepared { 256 } else { 512 },
                        "case {} did not reach expected validation outcome",
                        case.name
                    );
                    if prepared {
                        assert_eq!(
                            &old[COMPLETION..COMPLETION + 16],
                            &[
                                1,
                                case.brain[2],
                                case.brain[3],
                                256,
                                case.state[36],
                                case.state[37],
                                case.frame[14],
                                case.frame[15],
                                case.state[33] ^ 1,
                                case.header[9],
                                0,
                                0,
                                case.header[14],
                                case.header[15],
                                0,
                                0,
                            ],
                            "case {} changed completion ABI/banks/generations",
                            case.name
                        );
                    }
                }
            }
            assert!(
                negative_control_differences > 0,
                "corpus must detect a weakened bank guard in actual shader execution"
            );
            eprintln!(
                "sleep_initializer_negative_control_detected_cases={negative_control_differences}"
            );
            drop(mapped);
            output.unmap();
            eprintln!("sleep_initializer_equivalence_cases={} mutable_words_compared_per_case={} old_new_identical=true software_only=true",cases.len(),STATE_WORDS);
        });
    }
}
