//! Five remaining single-invocation publication barriers; frozen d139 sources.
use sha2::{Digest, Sha256};
const ABI: &str = include_str!("../shaders/closed_loop_abi.wgsl");
const ACTIVITY: &str = include_str!("../shaders/closed_loop_activity_validation.wgsl");
const STATE: usize = 512;
const FRAME: usize = 256;
const WORDS: usize = STATE + FRAME;
struct Spec {
    entry: &'static str,
    old: &'static str,
    new: &'static str,
    old_sha: &'static str,
    status_offset: usize,
    status: u32,
    guard_from: &'static str,
    guard_to: &'static str,
    publication_from: &'static str,
    publication_to: &'static str,
}
#[cfg(feature = "gpu-tests")]
#[path = "fixtures/barrier_cpu_finalize_discard_pending_eligibility.rs"]
mod golden_0;
#[cfg(feature = "gpu-tests")]
#[path = "fixtures/barrier_cpu_initialize_fast_plasticity.rs"]
mod golden_1;
#[cfg(feature = "gpu-tests")]
#[path = "fixtures/barrier_cpu_finalize_fast_plasticity.rs"]
mod golden_2;
#[cfg(feature = "gpu-tests")]
#[path = "fixtures/barrier_cpu_finalize_sleep_staging.rs"]
mod golden_3;
#[cfg(feature = "gpu-tests")]
#[path = "fixtures/barrier_cpu_finalize_sleep_commit.rs"]
mod golden_4;
const SPECS:&[Spec]=&[
Spec{entry:"finalize_discard_pending_eligibility",old:include_str!("fixtures/barrier_original_d139e44c_eligibility.wgsl"),new:include_str!("../shaders/closed_loop_eligibility.wgsl"),old_sha:"c671ebacfdef15e0fef9bd92f663e02648f3a62c658d25439306ec8429b9d39c",status_offset:163,status:1,guard_from:"learning.pending_valid != 1u",guard_to:"learning.pending_valid > 1u",publication_from:"store_state_u32(publication.z + 3u, 1u)",publication_to:"store_state_u32(publication.z + 3u, 2u)"},
Spec{entry:"initialize_fast_plasticity",old:include_str!("fixtures/barrier_original_d139e44c_plasticity.wgsl"),new:include_str!("../shaders/closed_loop_plasticity.wgsl"),old_sha:"582bb5960646514065855f17b4e5df07c0afe0a01d440de927251327780b297d",status_offset:163,status:256,guard_from:"header.schema_version != GPU_LEARNING_SCHEMA_VERSION",guard_to:"header.schema_version > GPU_LEARNING_SCHEMA_VERSION",publication_from:"store_state_u32(publication.y+3u,PLASTICITY_STATUS_PREPARED)",publication_to:"store_state_u32(publication.y+3u,PLASTICITY_STATUS_SUCCESS)"},
Spec{entry:"finalize_fast_plasticity",old:include_str!("fixtures/barrier_original_d139e44c_plasticity.wgsl"),new:include_str!("../shaders/closed_loop_plasticity.wgsl"),old_sha:"582bb5960646514065855f17b4e5df07c0afe0a01d440de927251327780b297d",status_offset:163,status:1,guard_from:"if (status != PLASTICITY_STATUS_PREPARED)",guard_to:"if (status != PLASTICITY_STATUS_PREPARED && status != 0u)",publication_from:"store_state_u32(publication.y+3u,PLASTICITY_STATUS_SUCCESS)",publication_to:"store_state_u32(publication.y+3u,PLASTICITY_STATUS_PREPARED)"},
Spec{entry:"finalize_sleep_staging",old:include_str!("fixtures/barrier_original_d139e44c_consolidate.wgsl"),new:include_str!("../shaders/closed_loop_consolidate.wgsl"),old_sha:"d5f04df3f50994ae25ee522585785746b0f1de571ccace5835debcc5cfae67ee",status_offset:195,status:1,guard_from:"if (status != SLEEP_STATUS_PREPARED)",guard_to:"if (status != SLEEP_STATUS_PREPARED && status != 0u)",publication_from:"store_state_u32(publication.y+3u,SLEEP_STATUS_SUCCESS)",publication_to:"store_state_u32(publication.y+3u,SLEEP_STATUS_COMMITTED)"},
Spec{entry:"finalize_sleep_commit",old:include_str!("fixtures/barrier_original_d139e44c_consolidate.wgsl"),new:include_str!("../shaders/closed_loop_consolidate.wgsl"),old_sha:"d5f04df3f50994ae25ee522585785746b0f1de571ccace5835debcc5cfae67ee",status_offset:195,status:2,guard_from:"|| load_state_u32(header.completion_offset+3u) != SLEEP_STATUS_SUCCESS)",guard_to:"|| (load_state_u32(header.completion_offset+3u) != SLEEP_STATUS_SUCCESS && load_state_u32(header.completion_offset+3u) != 0u))",publication_from:"store_state_u32(publication.y+3u,SLEEP_STATUS_COMMITTED)",publication_to:"store_state_u32(publication.y+3u,SLEEP_STATUS_SUCCESS)"},
];
#[cfg(feature = "gpu-tests")]
struct Reference {
    old: &'static str,
    corpus: &'static str,
    output: &'static str,
    patches: &'static [&'static [(usize, u32)]],
    ids: &'static [usize],
}
#[cfg(feature = "gpu-tests")]
fn reference(index: usize) -> Reference {
    match index {
        0 => Reference {
            old: golden_0::OLD_SHA256,
            corpus: golden_0::CORPUS_SHA256,
            output: golden_0::OUTPUT_SHA256,
            patches: golden_0::PATCHES,
            ids: golden_0::IDS,
        },
        1 => Reference {
            old: golden_1::OLD_SHA256,
            corpus: golden_1::CORPUS_SHA256,
            output: golden_1::OUTPUT_SHA256,
            patches: golden_1::PATCHES,
            ids: golden_1::IDS,
        },
        2 => Reference {
            old: golden_2::OLD_SHA256,
            corpus: golden_2::CORPUS_SHA256,
            output: golden_2::OUTPUT_SHA256,
            patches: golden_2::PATCHES,
            ids: golden_2::IDS,
        },
        3 => Reference {
            old: golden_3::OLD_SHA256,
            corpus: golden_3::CORPUS_SHA256,
            output: golden_3::OUTPUT_SHA256,
            patches: golden_3::PATCHES,
            ids: golden_3::IDS,
        },
        4 => Reference {
            old: golden_4::OLD_SHA256,
            corpus: golden_4::CORPUS_SHA256,
            output: golden_4::OUTPUT_SHA256,
            patches: golden_4::PATCHES,
            ids: golden_4::IDS,
        },
        _ => unreachable!(),
    }
}

#[derive(Clone)]
struct Case {
    name: String,
    brain: [u32; 72],
    identity: [u32; 16],
    plan: [u32; 128],
    weight: [u32; 64],
    dispatch: [u32; 664],
    frame: [u32; FRAME],
    state: [u32; STATE],
    jobs: u32,
}
impl Case {
    fn valid(index: usize) -> Self {
        let mut c = Self {
            name: "valid".into(),
            brain: [0; 72],
            identity: std::array::from_fn(|i| i as u32 + 11),
            plan: [0; 128],
            weight: [0; 64],
            dispatch: [0; 664],
            frame: [0; FRAME],
            state: [0; STATE],
            jobs: 1,
        };
        c.brain[..8].copy_from_slice(&[4, 4, 0, 7, 2, 2, 4, 2]);
        for (f, v) in [
            (18, 0),
            (19, 8),
            (20, 320),
            (21, 324),
            (23, 300),
            (24, 304),
            (25, 280),
            (26, 284),
            (29, 160),
            (30, 164),
            (32, 0),
        ] {
            c.brain[f] = v;
        }
        for (f, v) in [
            (0, 4),
            (2, 2),
            (3, 2),
            (4, 64),
            (7, 16),
            (8, 288),
            (9, 292),
            (10, 312),
            (11, 308),
            (12, 0),
            (15, 32),
            (16, 96),
            (17, 80),
            (18, 176),
        ] {
            c.state[f] = v;
        }
        c.state[32..56].copy_from_slice(&[
            3, 0, 0, 1, 5, 2, 5, 2, 6, 2, 7, 2, 0, 0, 2, 2, 1, 208, 264, 268, 80, 96, 17, 9,
        ]);
        c.dispatch[272..292].copy_from_slice(&[
            3, 4, 0, 7, 0, 1, 23, 11, 0, 0, 0, 160, 0, 2, 2, 0, 96, 0, 0, 0,
        ]);
        c.frame[0..4].copy_from_slice(&[3, 3 << 16, 17, 9]);
        c.frame[4..12].copy_from_slice(&c.identity[..8]);
        c.frame[12..20].copy_from_slice(&[1, 0, 7, 0, 8, 0, 85, 1]);
        c.frame[20..24].copy_from_slice(&[111, 112, 113, 114]);
        c.frame[24..32].copy_from_slice(&[21, 22, 23, 24, 25, 26, 27, 28]);
        c.frame[32..34].copy_from_slice(&[23, 11]);
        c.frame[34..42].copy_from_slice(&[0.5f32.to_bits(), 0.25f32.to_bits(), 0, 0, 0, 0, 0, 0]);
        c.state[96..100].copy_from_slice(&[3, 0, 7, 1]);
        c.state[100..108].copy_from_slice(&c.identity[..8]);
        c.state[108..114].copy_from_slice(&[17, 9, 23, 11, 7, 0]);
        c.state[114..122].copy_from_slice(&[21, 22, 23, 24, 25, 26, 27, 28]);
        c.state[122..128].copy_from_slice(&[3 << 16, 85, 111, 112, 113, 114]);
        c.state[128..132].copy_from_slice(&[5, 2, 6, 2]);
        c.state[163] = 256;
        for word in 170..173 {
            c.state[word] = u32::MAX;
        }
        c.state[268..272].copy_from_slice(&[0, 0, 0, 0]);
        for syn in 0..4 {
            c.plan[16 + syn * 8..24 + syn * 8].copy_from_slice(&[
                syn as u32,
                if syn < 2 { 1 } else { 2 },
                0,
                0,
                0,
                syn as u32 % 2,
                0,
                0,
            ]);
        }
        c.plan[64..80].copy_from_slice(&[
            0.5f32.to_bits(),
            0.1f32.to_bits(),
            0,
            0,
            1f32.to_bits(),
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            (-1f32).to_bits(),
            1f32.to_bits(),
            0,
            0,
        ]);
        if index == 0 {
            c.frame[..36].copy_from_slice(&c.state[96..132]);
        } else if index >= 3 {
            c.brain[1] = 1;
            c.brain[4] = 512;
            c.brain[23] = 104;
            c.brain[24] = 108;
            c.brain[25] = 120;
            c.brain[26] = 124;
            c.state.fill(0);
            for (f, v) in [
                (0, 4),
                (4, 64),
                (7, 16),
                (8, 122),
                (9, 126),
                (10, 116),
                (11, 112),
                (12, 0),
                (15, 32),
                (16, 64),
            ] {
                c.state[f] = v;
            }
            c.state[32..56].copy_from_slice(&[
                3, 0, 0, 0, 5, 2, 5, 2, 0, 0, 7, 2, 0, 1, 1, 1, 1, 128, 156, 160, 0, 64, 17, 9,
            ]);
            c.plan[..8].copy_from_slice(&[
                1,
                0.25f32.to_bits(),
                2f32.to_bits(),
                0.5f32.to_bits(),
                1,
                1,
                0,
                0,
            ]);
            c.dispatch[..20].copy_from_slice(&[
                1, 1, 0, 7, 0, 0, 44, 1, 72, 1, 76, 1, 4, 192, 17, 9, 23, 11, 0, 0,
            ]);
            c.frame.fill(0);
            c.frame[..4].copy_from_slice(&[1, 0, 23, 11]);
            c.frame[4..12].copy_from_slice(&c.identity[..8]);
            c.frame[12..16].copy_from_slice(&[5, 2, 6, 2]);
            c.frame[32..34].copy_from_slice(&[1, 1]);
            for (base, v) in [(104, 1f32), (108, 0.5), (112, 1.125), (116, 0.25)] {
                for i in 0..4 {
                    c.state[base + i] = v.to_bits();
                }
            }
            c.state[195] = if index == 3 { 256 } else { 1 };
        }
        c
    }
    fn region(&mut self, index: usize, region: usize) -> &mut [u32] {
        match region {
            0 => {
                if index >= 3 {
                    &mut self.dispatch[..20]
                } else {
                    &mut self.dispatch[272..292]
                }
            }
            1 => &mut self.brain[..36],
            2 => &mut self.state[..20],
            3 => &mut self.state[32..56],
            4 => &mut self.identity[..8],
            5 => {
                &mut self.frame[..if index >= 3 {
                    44
                } else if index == 0 {
                    36
                } else {
                    44
                }]
            }
            6 => {
                if index >= 3 {
                    &mut self.plan[..8]
                } else {
                    &mut self.state[96..132]
                }
            }
            7 => {
                if index >= 3 {
                    &mut self.state[192..208]
                } else {
                    &mut self.state[160..176]
                }
            }
            _ => unreachable!(),
        }
    }
}
fn corpus(index: usize) -> Vec<Case> {
    let base = Case::valid(index);
    let mut cases = vec![base.clone()];
    for region in 0..8 {
        let len = base.clone().region(index, region).len();
        for field in 0..len {
            let original = base.clone().region(index, region)[field];
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
                c.name = format!("r{region}[{field}]={value:08x}");
                c.region(index, region)[field] = value;
                cases.push(c);
            }
        }
    }
    for (name, lo, hi) in [("carry", u32::MAX, 2), ("wrap", u32::MAX, u32::MAX)] {
        for pair in [4, 6, 10, 22] {
            let mut c = base.clone();
            c.name = format!("{name}-generation-{pair}");
            c.state[32 + pair] = lo;
            c.state[33 + pair] = hi;
            if index == 0 && pair == 6 {
                c.frame[32] = lo;
                c.frame[33] = hi;
                c.state[128] = lo;
                c.state[129] = hi;
            }
            if index >= 3 && pair == 4 {
                c.frame[12] = lo;
                c.frame[13] = hi;
                let n = (u64::from(lo) | (u64::from(hi) << 32)).wrapping_add(1);
                c.frame[14] = n as u32;
                c.frame[15] = (n >> 32) as u32;
            }
            cases.push(c);
        }
    }
    if index == 0 {
        for word in [288, 289, 292, 293] {
            let mut c = base.clone();
            c.name = format!("uncleared-staging-{word}");
            c.state[word] = 1;
            cases.push(c);
        }
        for receipt in [0, 32, 96, 100, 160, 176, 500] {
            let mut c = base.clone();
            c.name = format!("receipt-alias-{receipt}");
            c.dispatch[283] = receipt;
            cases.push(c);
        }
    }
    if index == 2 {
        for context in [10usize, 11, 12] {
            let mut c = base.clone();
            c.name = format!("rejection-context-{context}");
            c.state[163] = 512;
            c.state[context + 160] = 0;
            cases.push(c);
        }
    }
    if index == 2 {
        let mut c = base.clone();
        c.name = "rejection-nonfinite-synapse".into();
        c.state[163] = 512;
        c.state[170] = 0;
        c.state[300] = 0x7fc00000;
        cases.push(c);
        let mut c = base.clone();
        c.name = "rejection-malformed-span".into();
        c.state[163] = 512;
        c.state[171] = 0;
        c.state[268] = 4;
        cases.push(c);
    }
    cases
}
fn corpus_sha(cases: &[Case]) -> String {
    let mut h = Sha256::new();
    for c in cases {
        h.update((c.name.len() as u64).to_le_bytes());
        h.update(c.name.as_bytes());
        h.update(c.jobs.to_le_bytes());
        for a in [
            &c.brain[..],
            &c.identity[..],
            &c.plan[..],
            &c.weight[..],
            &c.dispatch[..],
            &c.frame[..],
            &c.state[..],
        ] {
            for w in a {
                h.update(w.to_le_bytes());
            }
        }
    }
    format!("{:x}", h.finalize())
}

#[test]
fn all_five_frozen_prefixes_validate_and_keep_order() {
    for spec in SPECS {
        let signature = format!(
            "@compute @workgroup_size(1)\nfn {}(@builtin(global_invocation_id) gid:vec3<u32>) {{",
            spec.entry
        );
        let start = spec.old.find(&signature).unwrap();
        let end = start + spec.old[start..].find("  storageBarrier();").unwrap();
        let ty = if spec.entry == "finalize_discard_pending_eligibility" {
            "vec3<u32>"
        } else {
            "vec2<u32>"
        };
        let transformed = spec.old[start..end]
            .replace(
                &signature,
                &format!("fn prepare_{}(gid:vec3<u32>) -> {ty} {{", spec.entry),
            )
            .replace("return;", &format!("return {ty}(0u);"));
        let begin = spec
            .new
            .find(&format!("fn prepare_{}(", spec.entry))
            .unwrap();
        assert!(spec.new[begin..].starts_with(&transformed));
        for body in [spec.old, spec.new] {
            let source = format!("{ABI}{ACTIVITY}{body}");
            let m = naga::front::wgsl::parse_str(&source).unwrap();
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&m)
            .unwrap();
        }
    }
}
#[cfg(feature = "gpu-tests")]
mod device_oracle {
    fn mutate_function(source: &str, function: &str, from: &str, to: &str) -> String {
        let start = source.find(&format!("fn {function}(")).unwrap();
        let end = start + source[start..].find("\n}").unwrap() + 2;
        let changed = source[start..end].replacen(from, to, 1);
        assert_ne!(
            changed,
            &source[start..end],
            "negative mutation must affect target function"
        );
        format!("{}{}{}", &source[..start], changed, &source[end..])
    }
    use super::*;
    use std::io::Write;
    use wgpu::util::DeviceExt;
    fn run_one(software: bool, index: usize) {
        let spec = &SPECS[index];
        let old_source = spec.old;
        let new_source = spec.new;
        let reference = reference(index);
        let export = std::env::var_os("ALIFE_BARRIER_GOLDEN_EXPORT_DIR")
            .map(|p| std::path::PathBuf::from(p).join(spec.entry));
        assert!(
            software || export.is_none(),
            "only frozen original CPU may export"
        );
        let cases = corpus(index);
        let corpus_hash = corpus_sha(&cases);
        let old_hash = format!("{:x}", Sha256::digest(old_source.as_bytes()));
        assert_eq!(old_hash, spec.old_sha, "frozen original changed");
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
            assert_eq!(corpus_hash, reference.corpus);
            assert_eq!(old_hash, reference.old);
            assert_eq!(cases.len(), reference.ids.len());
        }
        let artifact = std::env::var_os("ALIFE_BARRIER_ORACLE_ARTIFACT_DIR")
            .map(|p| std::path::PathBuf::from(p).join(spec.entry));
        if let Some(p) = &artifact {
            std::fs::create_dir(p).unwrap();
        }
        let backends = if software {
            wgpu::Backends::VULKAN
        } else {
            wgpu::Backends::DX12
        };
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = backends;
        if !software {
            assert_eq!(std::env::var("ALIFE_GPU_BACKEND").as_deref(), Ok("dx12"));
            assert_eq!(
                std::env::var("ALIFE_GPU_DX12_COMPILER").as_deref(),
                Ok("fxc")
            );
            alife_gpu_backend::configure_gpu_test_dx12_compiler(&mut descriptor).unwrap();
        }
        let instance = wgpu::Instance::new(descriptor);
        let adapter = pollster::block_on(instance.enumerate_adapters(backends))
            .into_iter()
            .find(|a| {
                let i = a.get_info();
                i.backend
                    == if software {
                        wgpu::Backend::Vulkan
                    } else {
                        wgpu::Backend::Dx12
                    }
                    && if software {
                        i.device_type == wgpu::DeviceType::Cpu
                    } else {
                        i.vendor == 0x10de && i.device_type == wgpu::DeviceType::DiscreteGpu
                    }
            })
            .expect("strict CPU Vulkan or NVIDIA DX12; no fallback");
        let info = adapter.get_info();
        eprintln!("barrier_oracle_adapter={info:?}");
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
                "barrier_oracle_source label={label} sha256={:x}",
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
                entry_point: Some(spec.entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let old = software.then(|| pipeline("frozen-original", old_source));
        let new = pipeline("candidate", new_source);
        let guard_function = if index == 0 {
            "discard_contract_is_valid".to_string()
        } else {
            format!("prepare_{}", spec.entry)
        };
        let weak = mutate_function(new_source, &guard_function, spec.guard_from, spec.guard_to);
        assert_ne!(weak, new_source);
        let bad_write = if index == 0 {
            mutate_function(new_source,spec.entry,
                "store_state_u32(publication.y + 3u, 0u); }\n  storageBarrier();\n  if (publication.x != 0u) { store_state_u32(publication.z + 3u, 1u);",
                "store_state_u32(publication.z + 3u, 1u); }\n  storageBarrier();\n  if (publication.x != 0u) { store_state_u32(publication.y + 3u, 0u);")
        } else {
            mutate_function(
                new_source,
                spec.entry,
                spec.publication_from,
                spec.publication_to,
            )
        };
        assert_ne!(bad_write, new_source);
        let negative_guard = pipeline("negative-admission-guard", &weak);
        let negative_write = pipeline("negative-publication", &bad_write);
        let mut variants = Vec::new();
        if let Some(p) = &old {
            variants.push(p);
        }
        variants.extend([&new, &negative_guard, &negative_write]);
        let count = variants.len();
        let initial = Case::valid(index);
        let arrays: [&[u32]; 7] = [
            &initial.brain,
            &initial.identity,
            &initial.plan,
            &initial.weight,
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
                &c.plan,
                &c.weight,
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
                for &(w, value) in reference.patches[reference.ids[i]] {
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
            if c.name == "valid" {
                assert_eq!(
                    candidate[spec.status_offset], spec.status,
                    "valid fixture must succeed: {}",
                    spec.entry
                );
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
            std::fs::write(p.join("cpu-original.rs"), source).unwrap();
        } else {
            assert_eq!(output_hash, reference.output);
        }
        eprintln!("barrier_oracle_complete entry={} cases={} words_each={WORDS} changed_cases={changes} negative_guard_cases={} negative_publication_cases={} original_compiled={software} corpus_sha256={corpus_hash} output_sha256={output_hash}",spec.entry,cases.len(),negatives[0],negatives[1]);
        drop(map);
        output.unmap();
    }
    fn run(software: bool) {
        for variable in [
            "ALIFE_BARRIER_GOLDEN_EXPORT_DIR",
            "ALIFE_BARRIER_ORACLE_ARTIFACT_DIR",
        ] {
            if let Some(p) = std::env::var_os(variable) {
                std::fs::create_dir(p).unwrap();
            }
        }
        for index in 0..SPECS.len() {
            run_one(software, index);
        }
    }
    #[test]
    #[ignore = "CPU-only frozen differential for all five publication kernels"]
    fn software_vulkan_all_five_match_frozen_original() {
        run(true);
    }
    #[test]
    #[ignore = "Strict NVIDIA DX12 FXC all five against pinned CPU-original outputs"]
    fn nvidia_dx12_fxc_all_five_match_frozen_cpu_original() {
        run(false);
    }
}

fn inspect_flow(block: &naga::Block, depth: usize) -> (usize, bool) {
    let (mut barriers, mut returns) = (0, false);
    for stmt in block.iter() {
        match stmt {
            naga::Statement::ControlBarrier(_) | naga::Statement::MemoryBarrier(_) => {
                assert_eq!(depth, 0, "barrier is conditional");
                barriers += 1;
            }
            naga::Statement::Return { .. } => returns = true,
            naga::Statement::Block(b) => {
                let (n, r) = inspect_flow(b, depth + 1);
                barriers += n;
                returns |= r;
            }
            naga::Statement::If { accept, reject, .. } => {
                for b in [accept, reject] {
                    let (n, r) = inspect_flow(b, depth + 1);
                    barriers += n;
                    returns |= r;
                }
            }
            naga::Statement::Loop {
                body, continuing, ..
            } => {
                for b in [body, continuing] {
                    let (n, r) = inspect_flow(b, depth + 1);
                    barriers += n;
                    returns |= r;
                }
            }
            naga::Statement::Switch { cases, .. } => {
                for c in cases {
                    let (n, r) = inspect_flow(&c.body, depth + 1);
                    barriers += n;
                    returns |= r;
                }
            }
            _ => {}
        }
    }
    (barriers, returns)
}
#[test]
fn all_twenty_eight_variants_have_only_unconditional_single_lane_barriers() {
    use alife_gpu_backend::*;
    let (mut entries, mut variants, mut synchronized, mut barriers) = (0, 0, 0, 0);
    for source in [
        CLOSED_LOOP_ENCODE_WGSL,
        CLOSED_LOOP_RECURRENT_WGSL,
        CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
        CLOSED_LOOP_DECODE_WGSL,
        CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
        CLOSED_LOOP_ELIGIBILITY_WGSL,
        CLOSED_LOOP_PLASTICITY_WGSL,
        CLOSED_LOOP_CONSOLIDATE_WGSL,
        CLOSED_LOOP_REPLAY_LEARNING_WGSL,
    ] {
        let module = naga::front::wgsl::parse_str(source).unwrap();
        for (_, f) in module.functions.iter() {
            assert_eq!(inspect_flow(&f.body, 0).0, 0, "helper contains a barrier");
        }
        for entry in module.entry_points {
            entries += 1;
            variants += if entry.name == "recurrent_microstep" {
                4
            } else {
                1
            };
            let (n, _) = inspect_flow(&entry.function.body, 0);
            if n != 0 {
                assert_eq!(entry.workgroup_size, [1, 1, 1]);
                // Naga can insert implicit returns into the final publication
                // branches. Returns AFTER the final barrier cannot skip it.
                let last = entry
                    .function
                    .body
                    .iter()
                    .rposition(|s| {
                        matches!(
                            s,
                            naga::Statement::ControlBarrier(_) | naga::Statement::MemoryBarrier(_)
                        )
                    })
                    .unwrap();
                let mut prefix = entry.function.body.clone();
                prefix.cull(last + 1..);
                assert!(
                    !inspect_flow(&prefix, 0).1,
                    "entrypoint {} returns before barrier",
                    entry.name
                );
                synchronized += 1;
                barriers += n;
            }
        }
    }
    assert_eq!((entries, variants, synchronized, barriers), (25, 28, 7, 9));
}
