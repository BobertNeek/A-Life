#![cfg(feature = "gpu-tests")]
//! Frozen pre-repair eligibility-finalizer differential; synthetic buffers only.
use sha2::{Digest, Sha256};
const ABI: &str = include_str!("../shaders/closed_loop_abi.wgsl");
const ACTIVITY: &str = include_str!("../shaders/closed_loop_activity_validation.wgsl");
const OLD: &str = include_str!("fixtures/pending_eligibility_original_6c188a60.wgsl");
const NEW: &str = include_str!("../shaders/closed_loop_eligibility.wgsl");
#[cfg(feature = "gpu-tests")]
#[path = "fixtures/pending_eligibility_cpu_original.rs"]
mod golden;
const STATE: usize = 512;
const FRAME: usize = 256;
const WORDS: usize = STATE + FRAME;
const ENTRY: &str = "finalize_pending_eligibility";

#[test]
fn frozen_prefix_and_publication_order_are_preserved() {
    let start = OLD
        .find("@compute @workgroup_size(1)\nfn finalize_pending_eligibility(")
        .unwrap();
    let end = OLD[start..].find("  storageBarrier();").unwrap() + start;
    let before = OLD[start..end].replace(
        "@compute @workgroup_size(1)\nfn finalize_pending_eligibility(@builtin(global_invocation_id) gid:vec3<u32>) {",
        "fn prepare_pending_eligibility(gid:vec3<u32>) -> vec3<u32> {",
    ).replace("{ return; }", "{ return vec3<u32>(0u); }");
    let begin = NEW.find("fn prepare_pending_eligibility(").unwrap();
    assert!(NEW[begin..].starts_with(&before));
    for body in [OLD, NEW] {
        let source = format!("{ABI}{ACTIVITY}{body}");
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
    let rest = &NEW[NEW
        .find("@compute @workgroup_size(1)\nfn finalize_pending_eligibility(")
        .unwrap()..];
    let entry = &rest[..rest.find("\nfn discard_contract_is_valid").unwrap()];
    assert_eq!(entry.matches("storageBarrier();").count(), 2);
    assert!(!entry.contains("return"));
}

#[derive(Clone)]
struct Case {
    name: String,
    brain: [u32; 72],
    identity: [u32; 16],
    dispatch: [u32; 664],
    frame: [u32; FRAME],
    state: [u32; STATE],
    jobs: u32,
}
impl Case {
    fn valid() -> Self {
        let mut c = Self {
            name: "valid".into(),
            brain: [0; 72],
            identity: std::array::from_fn(|i| i as u32 + 11),
            dispatch: [0; 664],
            frame: std::array::from_fn(|i| 0x12340000 + i as u32),
            state: [0xa5a54321; STATE],
            jobs: 1,
        };
        for row in 0..2 {
            let b = row * 36;
            let ext = row * 64;
            let learning = ext + 32;
            let selection = 128 + row * 32;
            let template = row * 64;
            let pending = 192 + row * 48;
            let head = 272 + row * 332;
            c.brain[b] = 4;
            c.brain[b + 1] = 4;
            c.brain[b + 2] = row as u32;
            c.brain[b + 3] = 7 + row as u32;
            c.brain[b + 29] = 300 + row as u32 * 4;
            c.brain[b + 32] = ext as u32;
            c.state[ext..ext + 20].fill(0);
            c.state[ext] = 4;
            c.state[ext + 15] = learning as u32;
            c.state[ext + 16] = pending as u32;
            c.state[learning..learning + 24].fill(0);
            c.state[learning] = 3;
            c.state[learning + 21] = pending as u32;
            c.state[learning + 6] = 5;
            c.state[learning + 7] = 2;
            c.state[learning + 22] = 17;
            c.state[learning + 23] = 9;
            c.state[302 + row * 4] = 0;
            c.state[selection..selection + 16].fill(0);
            c.state[selection + 5] = 2;
            c.state[selection + 11] = 1;
            c.dispatch[row * 8..row * 8 + 8].copy_from_slice(&[
                0x55550000 + row as u32,
                0,
                3,
                0,
                0,
                0,
                0,
                0,
            ]);
            c.dispatch[head..head + 20].copy_from_slice(&[
                3,
                4,
                row as u32,
                7 + row as u32,
                row as u32,
                1,
                23,
                11,
                1,
                row as u32 * 8,
                104 + row as u32 * 4,
                selection as u32,
                template as u32,
                2,
                2,
                24,
                pending as u32,
                0,
                0,
                0,
            ]);
            c.frame[template..template + 36].fill(0);
            c.frame[template] = 3;
            c.frame[template + 1] = row as u32;
            c.frame[template + 2] = 7 + row as u32;
            c.frame[template + 3] = 1;
            c.frame[template + 4..template + 12].copy_from_slice(&c.identity[row * 8..row * 8 + 8]);
            c.frame[template + 14] = 23;
            c.frame[template + 15] = 11;
            c.frame[template + 26] = u32::MAX;
            c.frame[template + 32..template + 36].copy_from_slice(&[5, 2, 6, 2]);
        }
        c
    }
}
fn field_mut<'a>(c: &'a mut Case, region: &str, field: usize) -> &'a mut u32 {
    match region {
        "header" => &mut c.dispatch[272 + field],
        "brain" => &mut c.brain[field],
        "extension" => &mut c.state[field],
        "learning" => &mut c.state[32 + field],
        "selection" => &mut c.state[128 + field],
        "candidate" => &mut c.dispatch[field],
        "template" => &mut c.frame[field],
        "identity" => &mut c.identity[field],
        "diagnostic" => &mut c.state[302],
        "digest" => &mut c.frame[128 + field],
        _ => unreachable!(),
    }
}

fn corpus() -> Vec<Case> {
    let base = Case::valid();
    let mut cases = vec![base.clone()];
    for (region, len) in [
        ("header", 20),
        ("brain", 36),
        ("extension", 20),
        ("learning", 24),
        ("selection", 16),
        ("candidate", 8),
        ("template", 36),
        ("identity", 8),
        ("diagnostic", 1),
        ("digest", 4),
    ] {
        for field in 0..len {
            let original = *field_mut(&mut base.clone(), region, field);
            let mut values = std::collections::BTreeSet::from([
                0,
                1,
                2,
                3,
                7,
                8,
                15,
                16,
                31,
                32,
                63,
                64,
                127,
                128,
                255,
                256,
                511,
                512,
                513,
                u32::MAX - 1,
                u32::MAX,
            ]);
            for bit in 0..32 {
                values.insert(original ^ (1 << bit));
            }
            values.remove(&original);
            for value in values {
                let mut c = base.clone();
                *field_mut(&mut c, region, field) = value;
                c.name = format!("{region}[{field}]={value:08x}");
                cases.push(c);
            }
        }
    }
    for (name, active, transaction) in [
        ("eligibility-carry", [u32::MAX, 2], [17, 9]),
        ("eligibility-wrap", [u32::MAX, u32::MAX], [17, 9]),
        ("transaction-carry", [5, 2], [u32::MAX, 9]),
        ("transaction-wrap", [5, 2], [u32::MAX, u32::MAX]),
        ("both-carry", [u32::MAX, 7], [u32::MAX, 8]),
    ] {
        let mut c = base.clone();
        c.name = name.into();
        c.state[38..40].copy_from_slice(&active);
        c.state[54..56].copy_from_slice(&transaction);
        let next = (u64::from(active[0]) | (u64::from(active[1]) << 32)).wrapping_add(1);
        c.frame[32..36].copy_from_slice(&[active[0], active[1], next as u32, (next >> 32) as u32]);
        cases.push(c);
    }
    for pending in [0, 12, 32, 40, 128, 133, 140, 160, 300, 476, 477, u32::MAX] {
        let mut c = base.clone();
        c.name = format!("pending-alias-{pending}");
        c.dispatch[288] = pending;
        cases.push(c);
    }
    for selection in [0, 32, 40, 128, 192, 200, 302, 496, 497] {
        let mut c = base.clone();
        c.name = format!("selection-relocated-{selection}");
        let copied = 16.min(STATE - selection);
        c.state[selection..selection + copied].copy_from_slice(&base.state[128..128 + copied]);
        c.dispatch[283] = selection as u32;
        cases.push(c);
    }
    let mut c = base.clone();
    c.jobs = 2;
    c.name = "two-disjoint-rows".into();
    cases.push(c.clone());
    c.name = "mixed-second-reject".into();
    c.state[162] = 99;
    cases.push(c);
    let mut c = base.clone();
    c.name = "only-second-row-valid".into();
    c.jobs = 2;
    c.state[133] = 0;
    cases.push(c);
    let mut c = base.clone();
    c.name = "nonzero-brain-index-relocated-records".into();
    c.dispatch[276] = 1;
    c.dispatch[280] = 1;
    c.dispatch[281] = 8;
    c.dispatch[282] = 108;
    c.dispatch[283] = 160;
    c.dispatch[284] = 64;
    c.dispatch[288] = 240;
    cases.push(c);
    cases
}
fn corpus_sha(cases: &[Case]) -> String {
    let mut hash = Sha256::new();
    for c in cases {
        hash.update((c.name.len() as u64).to_le_bytes());
        hash.update(c.name.as_bytes());
        hash.update(c.jobs.to_le_bytes());
        for words in [
            &c.brain[..],
            &c.identity[..],
            &c.dispatch[..],
            &c.frame[..],
            &c.state[..],
        ] {
            for w in words {
                hash.update(w.to_le_bytes());
            }
        }
    }
    format!("{:x}", hash.finalize())
}

#[cfg(feature = "gpu-tests")]
mod device_oracle {
    use super::*;
    use std::io::Write;
    use wgpu::util::DeviceExt;
    fn run(software: bool) {
        let export =
            std::env::var_os("ALIFE_PENDING_GOLDEN_EXPORT_DIR").map(std::path::PathBuf::from);
        assert!(
            software || export.is_none(),
            "only frozen original CPU may export"
        );
        let cases = corpus();
        let corpus_hash = corpus_sha(&cases);
        let old_hash = format!("{:x}", Sha256::digest(OLD.as_bytes()));
        assert_eq!(
            old_hash, "e83ff58e472c6037c094a3a564215571bc5e9501a6fe20672adb1992c11a8420",
            "frozen original changed"
        );
        let mut streams = export.as_ref().map(|p| {
            std::fs::create_dir(p).unwrap();
            (
                std::io::BufWriter::new(
                    std::fs::File::create_new(p.join("initial-u32le.bin")).unwrap(),
                ),
                std::io::BufWriter::new(
                    std::fs::File::create_new(p.join("original-u32le.bin")).unwrap(),
                ),
            )
        });
        if export.is_none() {
            assert_eq!(corpus_hash, golden::CORPUS_SHA256);
            assert_eq!(old_hash, golden::OLD_SHA256);
            assert_eq!(cases.len(), golden::IDS.len());
        }
        let artifact =
            std::env::var_os("ALIFE_PENDING_ORACLE_ARTIFACT_DIR").map(std::path::PathBuf::from);
        if let Some(p) = &artifact {
            std::fs::create_dir(p).unwrap();
        }
        let backends = wgpu::Backends::VULKAN;
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = backends;
        if !software {
            assert_eq!(std::env::var("ALIFE_GPU_BACKEND").as_deref(), Ok("vulkan"));
            assert_eq!(
                std::env::var("ALIFE_GPU_DX12_COMPILER").as_deref(),
                Ok("fxc")
            );
            // Explicit Vulkan harness: DX12 compiler configuration is inapplicable.
        }
        let instance = wgpu::Instance::new(descriptor);
        let adapter = pollster::block_on(instance.enumerate_adapters(backends))
            .into_iter()
            .find(|a| {
                let i = a.get_info();
                i.backend == wgpu::Backend::Vulkan
                    && if software {
                        i.device_type == wgpu::DeviceType::Cpu
                    } else {
                        i.vendor == 0x10de && i.device_type == wgpu::DeviceType::DiscreteGpu
                    }
            })
            .expect("strict CPU Vulkan or NVIDIA DX12; no fallback");
        let info = adapter.get_info();
        eprintln!("pending_oracle_adapter={info:?}");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("synthetic-pending-oracle"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        }))
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
            label: None,
            entries: &entries,
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = |label: &str, body: &str| {
            let source = format!("{ABI}{ACTIVITY}{body}");
            eprintln!(
                "pending_oracle_source label={label} sha256={:x}",
                Sha256::digest(source.as_bytes())
            );
            if let Some(p) = &artifact {
                std::fs::write(p.join(format!("{label}.wgsl")), &source).unwrap();
            }
            let m = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&pl),
                module: &m,
                entry_point: Some(ENTRY),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let old = software.then(|| pipeline("frozen-original", OLD));
        let new = pipeline("candidate", NEW);
        let weak = NEW.replacen("selected.family >= 8u", "selected.family >= 9u", 1);
        assert_ne!(weak, NEW);
        let bad_write = NEW.replacen(
            "store_state_u32(publication.y + 3u, 1u)",
            "store_state_u32(publication.y + 3u, 2u)",
            1,
        );
        assert_ne!(bad_write, NEW);
        let negative_guard = pipeline("negative-family-guard", &weak);
        let negative_write = pipeline("negative-publication", &bad_write);
        let mut variants = Vec::new();
        if let Some(p) = &old {
            variants.push(p);
        }
        variants.extend([&new, &negative_guard, &negative_write]);
        let count = variants.len();
        let initial = Case::valid();
        let arrays: [&[u32]; 7] = [
            &initial.brain,
            &initial.identity,
            &[0; 8],
            &[0; 8],
            &initial.dispatch,
            &initial.frame,
            &initial.state,
        ];
        let buffers: Vec<_> = arrays
            .iter()
            .map(|a| {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(a),
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                })
            })
            .collect();
        let entries: Vec<_> = buffers
            .iter()
            .enumerate()
            .map(|(i, b)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: b.as_entire_binding(),
            })
            .collect();
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &entries,
        });
        let bytes = (WORDS * 4) as u64;
        let reset = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let output = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytes * count as u64 * cases.len() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        for (i, c) in cases.iter().enumerate() {
            let data: [&[u32]; 7] = [
                &c.brain,
                &c.identity,
                &[0; 8],
                &[0; 8],
                &c.dispatch,
                &c.frame,
                &c.state,
            ];
            for (b, w) in buffers.iter().zip(data) {
                queue.write_buffer(b, 0, bytemuck::cast_slice(w));
            }
            queue.write_buffer(&reset, 0, bytemuck::cast_slice(&c.state));
            queue.write_buffer(&reset, (STATE * 4) as u64, bytemuck::cast_slice(&c.frame));
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            for (v, p) in variants.iter().enumerate() {
                encoder.copy_buffer_to_buffer(&reset, 0, &buffers[6], 0, (STATE * 4) as u64);
                encoder.copy_buffer_to_buffer(
                    &reset,
                    (STATE * 4) as u64,
                    &buffers[5],
                    0,
                    (FRAME * 4) as u64,
                );
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: None,
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(p);
                    pass.set_bind_group(0, &group, &[]);
                    pass.dispatch_workgroups(1, c.jobs, 1);
                }
                let dest = (i * count + v) as u64 * bytes;
                encoder.copy_buffer_to_buffer(&buffers[6], 0, &output, dest, (STATE * 4) as u64);
                encoder.copy_buffer_to_buffer(
                    &buffers[5],
                    0,
                    &output,
                    dest + (STATE * 4) as u64,
                    (FRAME * 4) as u64,
                );
            }
            queue.submit(Some(encoder.finish()));
            if i % 128 == 127 {
                device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(std::time::Duration::from_secs(30)),
                    })
                    .unwrap();
            }
        }
        let (tx, rx) = std::sync::mpsc::channel();
        output
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(30)),
            })
            .unwrap();
        rx.recv().unwrap().unwrap();
        let map = output.slice(..).get_mapped_range();
        let words: &[u32] = bytemuck::cast_slice(&map);
        let mut raw = Sha256::new();
        let mut negatives = [0usize; 2];
        let mut patches = Vec::<Vec<(usize, u32)>>::new();
        let mut ids = Vec::<usize>::new();
        let mut changes = 0;
        for (i, c) in cases.iter().enumerate() {
            let variant = |v| &words[(i * count + v) * WORDS..(i * count + v + 1) * WORDS];
            let candidate = variant(usize::from(software));
            let initial: Vec<_> = c.state.iter().chain(&c.frame).copied().collect();
            let expected: Vec<u32> = if export.is_some() {
                variant(0).to_vec()
            } else {
                let mut e = initial.clone();
                for &(w, value) in golden::PATCHES[golden::IDS[i]] {
                    e[w] = value;
                }
                e
            };
            assert_eq!(candidate, expected, "candidate differs case {}", c.name);
            if software {
                assert_eq!(
                    variant(0),
                    expected,
                    "frozen original differs case {}",
                    c.name
                );
            }
            for w in &expected {
                raw.update(w.to_le_bytes());
            }
            if let Some((initial_stream, original_stream)) = &mut streams {
                for w in &initial {
                    initial_stream.write_all(&w.to_le_bytes()).unwrap();
                }
                for w in &expected {
                    original_stream.write_all(&w.to_le_bytes()).unwrap();
                }
            }
            for (n, differences) in negatives.iter_mut().enumerate() {
                *differences += usize::from(candidate != variant(count - 2 + n));
            }
            changes += usize::from(candidate != initial);
            if c.name.starts_with("pending-alias-") {
                assert_eq!(candidate[35], 1, "pending alias must publish valid");
                assert_eq!(
                    candidate[133], 3,
                    "pending alias must use captured selection"
                );
            }
            if c.name == "eligibility-wrap" || c.name == "transaction-wrap" {
                assert_eq!(candidate, initial, "full generation wrap must reject");
            }
            if c.name == "two-disjoint-rows" {
                assert_eq!(candidate[133], 3);
                assert_eq!(candidate[165], 3);
            }
            if c.name == "mixed-second-reject" {
                assert_eq!(candidate[133], 3);
                assert_eq!(candidate[165], 2);
            }
            if c.name == "only-second-row-valid" {
                assert_eq!(candidate[133], 0);
                assert_eq!(candidate[165], 3);
            }
            if c.name == "valid" {
                assert_eq!(candidate[35], 1);
                assert_eq!(candidate[133], 3);
                assert_eq!(candidate[200], 15);
                assert_eq!(candidate[218], 3 << 16);
                assert_eq!(candidate[219], 0x55550000);
            }
            if export.is_some() {
                let delta: Vec<_> = expected
                    .iter()
                    .zip(&initial)
                    .enumerate()
                    .filter_map(|(w, (&a, &b))| (a != b).then_some((w, a)))
                    .collect();
                let id = patches.iter().position(|p| p == &delta).unwrap_or_else(|| {
                    let id = patches.len();
                    patches.push(delta);
                    id
                });
                ids.push(id);
            }
        }
        let output_hash = format!("{:x}", raw.finalize());
        assert!(negatives.iter().all(|&n| n > 0));
        assert!(changes > 0 && changes < cases.len());
        if let Some(p) = export {
            if let Some((a, b)) = &mut streams {
                a.flush().unwrap();
                b.flush().unwrap();
            }
            let mut source=format!("// Synthetic full frozen-original CPU Vulkan outputs, all {WORDS} words per case.\npub const OLD_SHA256: &str = \"{old_hash}\";\npub const CORPUS_SHA256: &str = \"{corpus_hash}\";\npub const OUTPUT_SHA256: &str = \"{output_hash}\";\n#[rustfmt::skip]\npub const PATCHES: &[&[(usize,u32)]] = &[\n");
            for patch in &patches {
                source += " &[";
                for (w, v) in patch {
                    source += &format!("({w},0x{v:08x}),");
                }
                source += "],\n";
            }
            source += "];\n#[rustfmt::skip]\npub const IDS: &[usize] = &[\n";
            for chunk in ids.chunks(32) {
                source += " ";
                for id in chunk {
                    source += &format!("{id},");
                }
                source += "\n";
            }
            source += "];\n";
            std::fs::write(p.join("pending_eligibility_cpu_original.rs"), source).unwrap();
        } else {
            assert_eq!(output_hash, golden::OUTPUT_SHA256);
        }
        eprintln!("pending_oracle_complete cases={} words_each={WORDS} changed_cases={changes} negative_guard_cases={} negative_publication_cases={} original_compiled={software} corpus_sha256={corpus_hash} output_sha256={output_hash}",cases.len(),negatives[0],negatives[1]);
        drop(map);
        output.unmap();
    }
    #[test]
    #[ignore = "explicit CPU-only Vulkan frozen-original differential and golden validation"]
    fn software_vulkan_matches_frozen_original_all_corruptions() {
        run(true);
    }
    #[test]
    #[ignore = "strict NVIDIA DX12/FXC candidate versus pinned frozen-original CPU output"]
    fn nvidia_vulkan_fxc_matches_frozen_cpu_original_all_corruptions() {
        run(false);
    }
}
