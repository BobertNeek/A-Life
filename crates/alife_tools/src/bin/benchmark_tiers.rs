use std::path::PathBuf;

use alife_gpu_backend::{GpuRuntimeBackendConfig, GpuRuntimeBackendKind, GpuRuntimeFallbackReason};
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
        let backend = gpu_backend_config_from_env().select_backend()?;
        let notes = match backend.fallback_reason {
            Some(GpuRuntimeFallbackReason::HardwareUnavailable) => {
                "P29 smoke: GPU hardware performance run unavailable; CPU fallback data only"
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
            None => "P29 smoke: backend selected; P20 CPU smoke metrics are not GPU timings",
        };
        let gpu_report = GpuRuntimeBenchmarkBridge::from_cpu_smoke(&report, backend, notes)?;
        std::fs::create_dir_all(&output_dir)?;
        let gpu_path = output_dir.join("gpu_runtime_performance.md");
        std::fs::write(&gpu_path, gpu_report.to_markdown())?;
        println!("{}", gpu_path.display());
    }
    Ok(())
}

fn gpu_backend_config_from_env() -> GpuRuntimeBackendConfig {
    let requested = match std::env::var("ALIFE_GPU_RUNTIME_BACKEND")
        .unwrap_or_else(|_| "static".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "cpu" => GpuRuntimeBackendKind::CpuReference,
        "plastic" => GpuRuntimeBackendKind::GpuPlastic,
        "full" => GpuRuntimeBackendKind::GpuFull,
        _ => GpuRuntimeBackendKind::GpuStatic,
    };
    GpuRuntimeBackendConfig::request(requested)
        .with_gpu_feature_enabled(env_flag(
            "ALIFE_GPU_RUNTIME_FEATURE",
            requested != GpuRuntimeBackendKind::CpuReference,
        ))
        .with_hardware_available(env_flag("ALIFE_GPU_RUNTIME_AVAILABLE", false))
        .with_validation_passed(env_flag("ALIFE_GPU_RUNTIME_VALIDATED", false))
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
