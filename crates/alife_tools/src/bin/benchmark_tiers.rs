use std::path::PathBuf;

use alife_gpu_backend::{
    probe_local_wgpu_runtime, GpuRuntimeBackendConfig, GpuRuntimeBackendKind,
    GpuRuntimeFallbackReason, GpuRuntimeHardwareProbe,
};
use alife_tools::benchmark::{BenchmarkHarness, BenchmarkHarnessConfig, GpuRuntimeBenchmarkBridge};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let full = args.iter().any(|arg| arg == "--all");
    let gpu_runtime_report = args.iter().any(|arg| arg == "--gpu-runtime");
    let output_dir = args
        .windows(2)
        .find(|window| window[0] == "--out")
        .map(|window| PathBuf::from(&window[1]))
        .unwrap_or_else(|| PathBuf::from("target").join("artifacts"));
    let config = if full {
        BenchmarkHarnessConfig::manual_full()
    } else {
        BenchmarkHarnessConfig::smoke()
    };
    let report = BenchmarkHarness::run(config)?;
    let path = report.write_markdown(&output_dir)?;
    println!("{}", path.display());

    if gpu_runtime_report {
        let requested = requested_gpu_backend_from_env();
        let gpu_feature_enabled = env_flag(
            "ALIFE_GPU_RUNTIME_FEATURE",
            requested != GpuRuntimeBackendKind::CpuReference,
        );
        let probe = if gpu_feature_enabled || requested == GpuRuntimeBackendKind::CpuReference {
            probe_local_wgpu_runtime(requested)
        } else {
            GpuRuntimeHardwareProbe::unavailable(
                requested,
                "GPU runtime feature disabled; wgpu probe skipped",
            )
        };
        let env_hardware_available = env_flag_optional("ALIFE_GPU_RUNTIME_AVAILABLE");
        let env_validation_passed = env_flag_optional("ALIFE_GPU_RUNTIME_VALIDATED");
        let backend = gpu_backend_config_from_env(
            requested,
            &probe,
            gpu_feature_enabled,
            env_hardware_available,
            env_validation_passed,
        )
        .select_backend()?;
        let notes = match backend.fallback_reason {
            Some(GpuRuntimeFallbackReason::HardwareUnavailable)
                if env_hardware_available == Some(false) =>
            {
                "P29 smoke: ALIFE_GPU_RUNTIME_AVAILABLE=0 forced CPU fallback; GPU hardware timing not measured"
            }
            Some(GpuRuntimeFallbackReason::HardwareUnavailable) => {
                "P29 smoke: real wgpu adapter/device probe unavailable; CPU fallback data only"
            }
            Some(GpuRuntimeFallbackReason::ValidationFailed)
                if env_validation_passed == Some(false) =>
            {
                "P29 smoke: ALIFE_GPU_RUNTIME_VALIDATED=0 forced CPU fallback; GPU hardware timing not measured"
            }
            Some(GpuRuntimeFallbackReason::ValidationFailed) => {
                "P29 smoke: GPU validation failed; CPU fallback data only"
            }
            Some(GpuRuntimeFallbackReason::FeatureDisabled) => {
                "P29 smoke: GPU runtime feature disabled; CPU fallback data only"
            }
            Some(GpuRuntimeFallbackReason::UnsupportedBackend) => {
                "P29 smoke: requested GPU backend unsupported; CPU fallback data only"
            }
            None => {
                "P29 smoke: local wgpu adapter/device probe passed; P20 CPU smoke metrics are not GPU timings"
            }
        };
        let mut gpu_report = GpuRuntimeBenchmarkBridge::from_cpu_smoke(&report, backend, notes)?;
        gpu_report.hardware_identifier = probe.hardware_identifier();
        gpu_report.feature_flags.push(format!(
            "wgpu-probe={}",
            if probe.hardware_available() {
                "adapter-device-ok"
            } else {
                "unavailable"
            }
        ));
        if let Some(backend_api) = &probe.backend_api {
            gpu_report
                .feature_flags
                .push(format!("wgpu-backend={backend_api}"));
        }
        if let Some(limit) = probe.adapter_storage_buffers_per_shader_stage {
            gpu_report
                .feature_flags
                .push(format!("storage-buffers-per-stage={limit}"));
        }
        if let Some(error) = &probe.error {
            gpu_report
                .feature_flags
                .push(format!("probe-error={}", compact_note(error)));
        }
        std::fs::create_dir_all(&output_dir)?;
        let gpu_path = output_dir.join("gpu_runtime_performance.md");
        std::fs::write(&gpu_path, gpu_report.to_markdown())?;
        println!("{}", gpu_path.display());
    }
    Ok(())
}

fn requested_gpu_backend_from_env() -> GpuRuntimeBackendKind {
    match std::env::var("ALIFE_GPU_RUNTIME_BACKEND")
        .unwrap_or_else(|_| "static".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "cpu" => GpuRuntimeBackendKind::CpuReference,
        "plastic" => GpuRuntimeBackendKind::GpuPlastic,
        "full" => GpuRuntimeBackendKind::GpuFull,
        _ => GpuRuntimeBackendKind::GpuStatic,
    }
}

fn gpu_backend_config_from_env(
    requested: GpuRuntimeBackendKind,
    probe: &GpuRuntimeHardwareProbe,
    gpu_feature_enabled: bool,
    env_hardware_available: Option<bool>,
    env_validation_passed: Option<bool>,
) -> GpuRuntimeBackendConfig {
    let hardware_available = env_hardware_available.unwrap_or(probe.hardware_available());
    let validation_passed = env_validation_passed.unwrap_or(probe.device_request_succeeded);
    GpuRuntimeBackendConfig::request(requested)
        .with_gpu_feature_enabled(gpu_feature_enabled)
        .with_hardware_available(probe.hardware_available() && hardware_available)
        .with_validation_passed(validation_passed)
        .with_full_runtime_available(env_flag("ALIFE_GPU_FULL_RUNTIME_AVAILABLE", false))
}

fn env_flag(name: &str, default: bool) -> bool {
    std::env::var(name).map_or(default, |value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn env_flag_optional(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|value| {
        matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn compact_note(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch == '\n' || ch == '\r' { ' ' } else { ch })
        .collect::<String>()
        .chars()
        .take(96)
        .collect()
}
