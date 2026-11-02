//! Falsification oracle for the initializer-only compiler-pressure experiment.
//! The old shader is frozen from b9f78ed0, not rebuilt from candidate predicates.
//! Explicit CPU Vulkan and NVIDIA DX12 modes never fall back to each other.
//! The CPU golden digest pins every frozen-original output across all cases.
use std::collections::BTreeSet;

const ABI: &str = include_str!("../shaders/closed_loop_abi.wgsl");
const OLD: &str = include_str!("fixtures/sleep_initializer_legacy_b9f78ed0.wgsl");
const NEW: &str = include_str!("../shaders/closed_loop_consolidate.wgsl");
const STATE_WORDS: usize = 512;
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
fn both_initializer_variants_validate_and_candidate_avoids_all_record_copies() {
    let old = module(OLD);
    let new = module(NEW);
    assert!(reachable(&old)
        .iter()
        .any(|name| name == "load_consolidation_request"));
    assert!(!reachable(&new)
        .iter()
        .any(|name| name == "load_consolidation_request"));
    for forbidden in [
        "load_sleep_header",
        "load_slot_extension",
        "load_slot_learning_state",
        "load_sleep_parameter",
        "sleep_transaction_valid",
    ] {
        assert!(
            !reachable(&new).iter().any(|name| name == forbidden),
            "initializer still reaches {forbidden}"
        );
    }
    // Other entrypoints continue to use the original full validator and ABI.
    assert!(new
        .functions
        .iter()
        .any(|(_, f)| f.name.as_deref() == Some("sleep_transaction_valid")));
}

#[derive(Clone)]
struct Case {
    name: String,
    brain: [u32; 72],
    identity: [u32; 16],
    plan: [u32; 64],
    header: [u32; 40],
    frame: [u32; 128],
    state: [u32; STATE_WORDS],
    prepared: Option<bool>,
    jobs: u32,
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
            brain: [0; 72],
            identity: std::array::from_fn(|i| i as u32 + 11),
            plan: [0; 64],
            header: [0; 40],
            frame: [0x5a5a_4321; 128],
            state: [0xa5a5_1234; STATE_WORDS],
            prepared: Some(true),
            jobs: 1,
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
        case.header[..20].copy_from_slice(&[
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
        case.frame[4..12].copy_from_slice(&case.identity[..8]);
        case.frame[12..16].copy_from_slice(&[5, 2, 6, 2]);
        case.frame[32..34].copy_from_slice(&[1, 1]);
        case.frame[42..44].fill(0);
        case
    }

    fn region(&mut self, region: Region) -> &mut [u32] {
        match region {
            Region::Header => &mut self.header[..20],
            Region::Brain => &mut self.brain[..36],
            Region::Extension => &mut self.state[..20],
            Region::Learning => &mut self.state[32..56],
            Region::Request => &mut self.frame[..44],
            Region::Parameters => &mut self.plan[..8],
            Region::Identity => &mut self.identity[..8],
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
                    511,
                    512,
                    513,
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
    // All completion locations, including overlapping mutable inputs. These
    // detect changes to the legacy clear/write-before-snapshot ordering.
    for completion in 0..=STATE_WORDS - 16 {
        let mut changed = base.clone();
        changed.name = format!("completion-overlap-{completion}");
        changed.header[13] = completion as u32;
        changed.prepared = None;
        cases.push(changed);
    }
    for region in [
        "request",
        "parameters",
        "extension",
        "learning",
        "brain-index",
    ] {
        let mut changed = base.clone();
        changed.name = format!("valid-relocated-{region}");
        match region {
            "request" => {
                changed.frame.copy_within(0..44, 84);
                changed.frame[..44].fill(0);
                changed.header[5] = 84;
            }
            "parameters" => {
                changed.plan.copy_within(0..8, 32);
                changed.plan[..8].fill(0);
                changed.state[12] = 32;
            }
            "extension" => {
                changed.state.copy_within(0..20, 256);
                changed.state[..20].fill(0);
                changed.brain[32] = 256;
            }
            "learning" => {
                changed.state.copy_within(32..56, 288);
                changed.state[32..56].fill(0);
                changed.state[15] = 288;
            }
            _ => {
                changed.brain.copy_within(0..36, 36);
                changed.brain[38] = 1;
                changed.brain[..36].fill(0);
                for lane in 0..8 {
                    changed.identity[8 + lane] = 101 + lane as u32;
                }
                changed.frame[4..12].copy_from_slice(&changed.identity[8..16]);
                changed.header[2] = 1;
                changed.header[4] = 1;
            }
        }
        cases.push(changed);
    }
    let mut pair = base.clone();
    pair.name = "two-concurrent-disjoint-jobs".into();
    pair.jobs = 2;
    pair.brain.copy_within(0..36, 36);
    pair.brain[38] = 1;
    pair.brain[68] = 256;
    for lane in 0..8 {
        pair.identity[8 + lane] = 101 + lane as u32;
    }
    pair.header.copy_within(0..20, 20);
    pair.header[22] = 1;
    pair.header[24] = 1;
    pair.header[25] = 84;
    pair.header[33] = 448;
    pair.header[34] = 29;
    pair.frame.copy_within(0..44, 84);
    pair.frame[88..96].copy_from_slice(&pair.identity[8..16]);
    pair.frame[96..100].copy_from_slice(&[9, 13, 10, 13]);
    pair.state.copy_within(0..20, 256);
    pair.state[271] = 288;
    pair.state.copy_within(32..56, 288);
    pair.state[289..294].copy_from_slice(&[1, 1, 0, 9, 13]);
    for (field, value) in [(23, 336), (24, 340), (25, 344), (26, 348)] {
        pair.brain[36 + field] = value;
    }
    for (field, value) in [(8, 352), (9, 354), (10, 356), (11, 360), (16, 364)] {
        pair.state[256 + field] = value;
    }
    for (field, value) in [(17, 400), (18, 428), (19, 432)] {
        pair.state[288 + field] = value;
    }
    cases.push(pair);
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
mod device_oracle {
    use super::*;
    use wgpu::util::DeviceExt;

    #[test]
    #[ignore = "explicit CPU Vulkan old/new initializer equivalence; not RTX qualification"]
    fn software_vulkan_initializer_matches_frozen_legacy_for_all_corruptions() {
        run(wgpu::Backends::VULKAN, true);
    }

    #[test]
    #[ignore = "explicit NVIDIA DX12 full-state legacy/candidate equivalence; synthetic state only"]
    fn nvidia_dx12_initializer_matches_frozen_legacy_for_all_corruptions() {
        run(wgpu::Backends::DX12, false);
    }

    fn run(backends: wgpu::Backends, software_only: bool) {
        pollster::block_on(async {
            let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
            descriptor.backends = backends;
            if !software_only {
                alife_gpu_backend::configure_gpu_test_dx12_compiler(&mut descriptor).unwrap();
            }
            let instance = wgpu::Instance::new(descriptor);
            let adapter = instance
                .enumerate_adapters(backends)
                .await
                .into_iter()
                .find(|adapter| {
                    let info = adapter.get_info();
                    if software_only {
                        info.device_type == wgpu::DeviceType::Cpu
                    } else {
                        info.vendor == 0x10de && info.device_type == wgpu::DeviceType::DiscreteGpu
                    }
                })
                .expect("requested CPU Vulkan or NVIDIA DX12 adapter absent; no fallback");
            let info = adapter.get_info();
            assert_eq!(
                info.backend,
                if software_only {
                    wgpu::Backend::Vulkan
                } else {
                    wgpu::Backend::Dx12
                }
            );
            eprintln!("sleep_initializer_equivalence_adapter={info:?}");
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("sleep-initializer-explicit-device-oracle"),
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
            let new = pipeline("converged-barrier-initializer", NEW);
            eprintln!("initializer_pipeline_create_seconds old={old_seconds:.6} candidate={:.6} software_only={software_only}",started.elapsed().as_secs_f64());
            // A deliberately weakened bank guard must be falsified by this
            // corpus; otherwise old/new equality could be a vacuous harness pass.
            let weakened = NEW.replacen(
                "if (!(l_active_weight_bank <= 1u))",
                "if (!(l_active_weight_bank <= 2u))",
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
                        pass.dispatch_workgroups(1, case.jobs, 1);
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
            let mut golden = alife_core::CanonicalDigestBuilder::new(
                b"alife.sleep-initializer.synthetic-corpus-output.v1",
            );
            golden.write_sequence_len(cases.len());
            golden.write_sequence_len(STATE_WORDS);
            let mut negative_control_differences = 0;
            for (index, case) in cases.iter().enumerate() {
                let old = &words[(index * 3) * STATE_WORDS..(index * 3 + 1) * STATE_WORDS];
                let new = &words[(index * 3 + 1) * STATE_WORDS..(index * 3 + 2) * STATE_WORDS];
                let weakened = &words[(index * 3 + 2) * STATE_WORDS..(index * 3 + 3) * STATE_WORDS];
                for word in old {
                    golden.write_u32(*word);
                }
                negative_control_differences += usize::from(old != weakened);
                assert_eq!(
                    old, new,
                    "case {} changed initializer state/completion writes",
                    case.name
                );
                if let Some(prepared) = case.prepared {
                    for job in 0..case.jobs as usize {
                        let header = job * 20;
                        let brain = case.header[header + 4] as usize * 36;
                        let completion = case.header[header + 13] as usize;
                        assert_eq!(
                            old[completion + 3],
                            if prepared { 256 } else { 512 },
                            "case {} job{job} expected status",
                            case.name
                        );
                        if prepared {
                            let extension = case.brain[brain + 32] as usize;
                            let learning = case.state[extension + 15] as usize;
                            let request = case.header[header + 5] as usize;
                            assert_eq!(
                                &old[completion..completion + 16],
                                &[
                                    1,
                                    case.brain[brain + 2],
                                    case.brain[brain + 3],
                                    256,
                                    case.state[learning + 4],
                                    case.state[learning + 5],
                                    case.frame[request + 14],
                                    case.frame[request + 15],
                                    case.state[learning + 1] ^ 1,
                                    case.header[header + 9],
                                    0,
                                    0,
                                    case.header[header + 14],
                                    case.header[header + 15],
                                    0,
                                    0,
                                ],
                                "case {} job{job} completion",
                                case.name
                            );
                        }
                    }
                }
            }
            let golden = golden.finish256();
            eprintln!("sleep_initializer_frozen_original_output_digest={golden:016x?}");
            // Frozen original on explicit llvmpipe Vulkan. This pins all output
            // bits across backends; old/new agreement alone could hide a shared
            // backend miscompile. A mismatch must not refresh this reference.
            assert_eq!(
                golden,
                [
                    0xdb70_c676_c019_08bb,
                    0xcbe9_1726_a043_5cd3,
                    0x02dc_bc58_f3e6_9598,
                    0xad2b_2bc0_771f_a666,
                ],
                "frozen original differs from the CPU Vulkan output reference"
            );
            assert!(
                negative_control_differences > 0,
                "corpus must detect a weakened bank guard in actual shader execution"
            );
            eprintln!(
                "sleep_initializer_negative_control_detected_cases={negative_control_differences}"
            );
            drop(mapped);
            output.unmap();
            eprintln!("sleep_initializer_equivalence_cases={} mutable_words_compared_per_case={} old_new_identical=true software_only={software_only} backend={:?}",cases.len(),STATE_WORDS,info.backend);
        });
    }
}
