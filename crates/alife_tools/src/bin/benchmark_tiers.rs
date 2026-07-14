use std::path::PathBuf;

use alife_gpu_backend::{
    probe_local_wgpu_runtime, run_local_gpu_diagnostic_timing, GpuRuntimeBackendConfig,
    GpuRuntimeBackendKind,
};
use alife_tools::benchmark::{BenchmarkHarness, BenchmarkHarnessConfig, GpuRuntimeBenchmarkBridge};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let output_dir = args
        .windows(2)
        .find(|window| window[0] == "--out")
        .map(|window| PathBuf::from(&window[1]))
        .unwrap_or_else(|| PathBuf::from("target/artifacts"));
    let config = if args.iter().any(|arg| arg == "--all") {
        BenchmarkHarnessConfig::manual_full()
    } else {
        BenchmarkHarnessConfig::smoke()
    };
    let report = BenchmarkHarness::run(config)?;
    println!("{}", report.write_markdown(&output_dir)?.display());

    if args.iter().any(|arg| arg == "--gpu-runtime") {
        let requested = GpuRuntimeBackendKind::GpuAuthoritative;
        let probe = probe_local_wgpu_runtime(requested);
        std::fs::create_dir_all(&output_dir)?;
        let gpu_path = output_dir.join("gpu_runtime_performance.md");
        if probe.hardware_available() && probe.error.is_none() {
            let backend = GpuRuntimeBackendConfig::request(requested)
                .with_hardware_available(true)
                .with_validation_passed(true)
                .select_backend()?;
            let mut gpu_report = GpuRuntimeBenchmarkBridge::from_cpu_smoke(
                &report,
                backend,
                "CPU smoke metrics are context only; GPU measurements remain unknown",
            )?;
            gpu_report.hardware_identifier = probe.hardware_identifier();
            std::fs::write(&gpu_path, gpu_report.to_markdown())?;
        } else {
            std::fs::write(
                &gpu_path,
                format!(
                    "# GPU runtime performance\n\nStatus: unavailable\n\nFailure policy: stop learned actions.\n\nProbe: {}\n",
                    probe.error.as_deref().unwrap_or("adapter unavailable")
                ),
            )?;
        }
        println!("{}", gpu_path.display());

        if args.iter().any(|arg| arg == "--measure-gpu") {
            let timing = run_local_gpu_diagnostic_timing(3, 10)?;
            let path = output_dir.join("local_gpu_timing_evidence.md");
            std::fs::write(&path, timing.to_markdown())?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
