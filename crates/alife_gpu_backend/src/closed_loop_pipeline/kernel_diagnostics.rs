//! Bounded compilation/first-dispatch isolation, not cognition qualification.
use super::*;
use std::time::Duration;
use wgpu::util::DeviceExt;

pub(super) fn scopes(device: &wgpu::Device) -> [wgpu::ErrorScopeGuard; 3] {
    [
        wgpu::ErrorFilter::OutOfMemory,
        wgpu::ErrorFilter::Validation,
        wgpu::ErrorFilter::Internal,
    ]
    .map(|filter| device.push_error_scope(filter))
}

pub(super) fn check(
    scopes: [wgpu::ErrorScopeGuard; 3],
    stage: &str,
    entry: &str,
    microstep: Option<u32>,
) {
    let errors: Vec<_> = scopes
        .into_iter()
        .rev()
        .filter_map(|scope| pollster::block_on(scope.pop()))
        .collect();
    assert!(
        errors.is_empty(),
        "kernel_diagnostic stage={stage} entry={entry} microstep={microstep:?} errors={errors:?}"
    );
}

fn diagnose(nvidia: bool, backends: wgpu::Backends) {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backends;
    let instance = wgpu::Instance::new(descriptor);
    let adapter = pollster::block_on(instance.enumerate_adapters(backends))
        .into_iter()
        .find(|adapter| {
            let info = adapter.get_info();
            if nvidia {
                info.vendor == 0x10de && info.device_type == wgpu::DeviceType::DiscreteGpu
            } else {
                info.device_type == wgpu::DeviceType::Cpu
            }
        })
        .expect("requested NVIDIA hardware or explicit CPU Vulkan adapter is absent; no fallback");
    let info = adapter.get_info();
    eprintln!("kernel_diagnostic_adapter={info:?} synthetic_inactive_records=true");
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("bounded-production-kernel-isolation"),
        required_features: crate::closed_loop_runtime::required_device_features(),
        required_limits: crate::closed_loop_runtime::required_device_limits(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
    }))
    .unwrap();
    let layout = create_neural_bind_group_layout(&device);
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("unchanged-production-seven-buffer-layout"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    // Deliberately invalid/inactive schema records exercise compilation and
    // first dispatch only. Runtime robustness stays enabled. No real checkpoint,
    // valid neural fixture, learner or policy is loaded or advanced here.
    let zero = [0_u32; 1024];
    let buffers: Vec<_> = (0..7)
        .map(|_| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("synthetic-inactive-kernel-records"),
                contents: bytemuck::cast_slice(&zero),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            })
        })
        .collect();
    let entries: Vec<_> = buffers
        .iter()
        .enumerate()
        .map(|(binding, buffer)| wgpu::BindGroupEntry {
            binding: binding as u32,
            resource: buffer.as_entire_binding(),
        })
        .collect();
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic-inactive-seven-buffer-group"),
        layout: &layout,
        entries: &entries,
    });
    let mut tested = 0;
    for (source, names) in [
        (CLOSED_LOOP_ENCODE_WGSL, &["encode_perception"][..]),
        (
            CLOSED_LOOP_RECURRENT_WGSL,
            &["clear_v11_work", "recurrent_microstep"][..],
        ),
        (
            CLOSED_LOOP_CLEAR_DIAGNOSTICS_WGSL,
            &["clear_diagnostics"][..],
        ),
        (
            CLOSED_LOOP_DECODE_WGSL,
            &[
                "decode_candidates",
                "select_candidate",
                "decode_speech_payload",
            ][..],
        ),
        (
            CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
            &["add_candidate_memory_context"][..],
        ),
        (
            CLOSED_LOOP_ELIGIBILITY_WGSL,
            &[
                "prevalidate_eligibility",
                "accumulate_recurrent_eligibility",
                "accumulate_decoder_eligibility",
                "finalize_pending_eligibility",
                "discard_pending_eligibility_arrays",
                "finalize_discard_pending_eligibility",
            ][..],
        ),
        (
            CLOSED_LOOP_PLASTICITY_WGSL,
            &[
                "initialize_fast_plasticity",
                "apply_fast_plasticity",
                "capture_fast_plasticity_replay",
                "finalize_fast_plasticity",
            ][..],
        ),
        (
            CLOSED_LOOP_CONSOLIDATE_WGSL,
            &[
                "initialize_sleep_transaction",
                "copy_sleep_weight_banks",
                "consolidate_fast_weights",
                "finalize_sleep_staging",
                "reset_sleep_mutable_state",
                "finalize_sleep_commit",
            ][..],
        ),
        (
            CLOSED_LOOP_REPLAY_LEARNING_WGSL,
            &["replay_sleep_learning"][..],
        ),
    ] {
        for entry in names {
            for step in 0..if *entry == "recurrent_microstep" {
                4
            } else {
                1
            } {
                let microstep = (*entry == "recurrent_microstep").then_some(step);
                let constants: Vec<_> = microstep
                    .into_iter()
                    .map(|i| ("microstep_index", i as f64))
                    .collect();
                eprintln!("kernel_diagnostic_create_begin entry={entry} microstep={microstep:?}");
                let scope = scopes(&device);
                let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(entry),
                    source: wgpu::ShaderSource::Wgsl(runtime_shader_source(source)),
                });
                let pipeline =
                    create_compute_pipeline(&device, &pipeline_layout, &shader, entry, &constants);
                check(scope, "create", entry, microstep);
                eprintln!("kernel_diagnostic_create_ok entry={entry} microstep={microstep:?}");
                for buffer in &buffers {
                    queue.write_buffer(buffer, 0, bytemuck::cast_slice(&zero));
                }
                let scope = scopes(&device);
                let mut encoder = device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(entry) });
                {
                    let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some(entry),
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&pipeline);
                    pass.set_bind_group(0, &group, &[]);
                    pass.dispatch_workgroups(1, 1, 1);
                }
                eprintln!("kernel_diagnostic_submit_begin entry={entry} microstep={microstep:?}");
                let submission = queue.submit(Some(encoder.finish()));
                let polled = device.poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: Some(Duration::from_secs(30)),
                });
                check(scope, "submit_poll", entry, microstep);
                assert!(polled.is_ok(), "kernel_diagnostic_poll_failed entry={entry} microstep={microstep:?} error={polled:?}");
                eprintln!("kernel_diagnostic_submit_ok entry={entry} microstep={microstep:?}");
                tested += 1;
            }
        }
    }
    assert_eq!(tested, 28);
    eprintln!("kernel_diagnostic_complete pipelines={tested} synthetic_inactive_only=true hardware={nvidia}");
}

#[test]
#[ignore = "explicit NVIDIA Vulkan compilation/first-dispatch diagnostic; no private state"]
fn nvidia_vulkan_each_production_kernel_compiles_and_dispatches() {
    diagnose(true, wgpu::Backends::VULKAN);
}

#[test]
#[ignore = "explicit CPU Vulkan diagnostic harness check; not RTX qualification"]
fn software_vulkan_each_production_kernel_compiles_and_dispatches() {
    diagnose(false, wgpu::Backends::VULKAN);
}

#[test]
#[ignore = "explicit NVIDIA DX12 production-kernel diagnostic; no private state"]
fn nvidia_dx12_each_production_kernel_compiles_and_dispatches() {
    diagnose(true, wgpu::Backends::DX12);
}
