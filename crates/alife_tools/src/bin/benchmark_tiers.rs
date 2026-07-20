use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use alife_gpu_backend::{
    probe_local_wgpu_runtime, run_local_gpu_diagnostic_timing, GpuRuntimeBackendConfig,
    GpuRuntimeBackendKind,
};
use alife_tools::benchmark::{
    gpu_closed_loop::{
        adapter_identity_digest, canonical_performance_targets_v1, load_benchmark_manifest,
        load_performance_targets, run_single_benchmark_row, GpuBenchmarkError,
        GpuClosedLoopBenchmarkManifest, GpuPerformanceTargetRowV1,
    },
    BenchmarkHarness, BenchmarkHarnessConfig, GpuRuntimeBenchmarkBridge,
};

struct BenchmarkStagingDirectory(PathBuf);

impl Drop for BenchmarkStagingDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(path) = value_after(&args, "--write-canonical-targets") {
        atomic_write_json(Path::new(path), &canonical_performance_targets_v1())?;
        return Ok(());
    }
    if let Some(path) = value_after(&args, "--validate") {
        let targets_path = required_value(&args, "--targets")?;
        let targets = load_performance_targets(targets_path)?;
        load_benchmark_manifest(path, &targets)?;
        println!("validated {}", path);
        return Ok(());
    }
    if args.iter().any(|arg| arg == "--single-row") {
        run_private_single_row(&args)?;
        return Ok(());
    }
    if value_after(&args, "--backend") == Some("gpu-closed-loop") {
        run_gpu_matrix(&args)?;
        return Ok(());
    }
    run_legacy_benchmark(&args)
}

fn run_private_single_row(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.iter().filter(|arg| *arg == "--single-row").count() != 1
        || args.iter().any(|arg| {
            matches!(
                arg.as_str(),
                "--output" | "--classes" | "--sensor-profiles" | "--populations" | "--base-seed"
            )
        })
    {
        return Err(GpuBenchmarkError::Contract(
            "private benchmark mode accepts exactly one row key and no matrix output",
        )
        .into());
    }
    let targets = load_performance_targets(required_single_value(args, "--targets")?)?;
    let class_id_raw = parse_class(required_single_value(args, "--class")?)?;
    let sensor_profile_id_raw = parse_profile(required_single_value(args, "--sensor-profile")?)?;
    let population = required_single_value(args, "--population")?.parse::<u32>()?;
    let target = targets
        .rows
        .iter()
        .copied()
        .find(|target| target.key() == (class_id_raw, sensor_profile_id_raw, population))
        .ok_or(GpuBenchmarkError::Contract(
            "private benchmark row is not in the target matrix",
        ))?;
    let row = run_single_benchmark_row(target)?;
    atomic_write_json(
        Path::new(required_single_value(args, "--row-output")?),
        &row,
    )?;
    Ok(())
}

fn run_gpu_matrix(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let targets_path = PathBuf::from(required_value(args, "--targets")?);
    let targets = load_performance_targets(&targets_path)?;
    let requested = requested_targets(args, &targets.rows)?;
    if requested != targets.rows {
        return Err(GpuBenchmarkError::Contract(
            "promotion benchmark requires the exact 36-row target matrix",
        )
        .into());
    }
    if required_value(args, "--base-seed")?.parse::<u64>()? != 4_404 {
        return Err(GpuBenchmarkError::Contract("benchmark base seed must be 4404").into());
    }
    let output = PathBuf::from(required_value(args, "--output")?);
    let provenance = clean_git_provenance()?;
    let staging = output.with_extension(format!("staging-{}", std::process::id()));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    let _staging_cleanup = BenchmarkStagingDirectory(staging.clone());
    let executable = std::env::current_exe()?;
    let result = (|| -> Result<Vec<_>, Box<dyn std::error::Error>> {
        let mut rows = Vec::with_capacity(requested.len());
        for (index, target) in requested.iter().enumerate() {
            let row_path = staging.join(format!("row-{index:02}.json"));
            let status = Command::new(&executable)
                .arg("--single-row")
                .arg("--targets")
                .arg(&targets_path)
                .arg("--class")
                .arg(class_slug(target.class_id_raw)?)
                .arg("--sensor-profile")
                .arg(profile_slug(target.sensor_profile_id_raw)?)
                .arg("--population")
                .arg(target.population.to_string())
                .arg("--row-output")
                .arg(&row_path)
                .stdin(Stdio::null())
                .status()?;
            if !status.success() {
                return Err(GpuBenchmarkError::ContractDetail(format!(
                    "private benchmark child failed for {:?}",
                    target.key()
                ))
                .into());
            }
            let row: alife_tools::benchmark::gpu_closed_loop::GpuClosedLoopBenchmarkRow =
                serde_json::from_slice(&fs::read(&row_path)?)?;
            row.validate(
                &alife_tools::benchmark::gpu_closed_loop::GpuClosedLoopBenchmarkProtocolV1::canonical(),
                target,
            )?;
            rows.push(row);
        }
        Ok(rows)
    })();
    let rows = match result {
        Ok(rows) => rows,
        Err(error) => {
            fs::remove_dir_all(&staging)?;
            return Err(error);
        }
    };
    let mut observed = None;
    let mut adapter = None;
    for row in &rows {
        if let Some(row_adapter) = &row.environment.adapter {
            let identity = adapter_identity_digest(row_adapter)?;
            match observed {
                None => {
                    observed = Some(identity);
                    adapter = Some(row_adapter.clone());
                }
                Some(expected) if expected == identity => {}
                Some(_) => {
                    fs::remove_dir_all(&staging)?;
                    return Err(GpuBenchmarkError::Contract(
                        "benchmark children used different adapters",
                    )
                    .into());
                }
            }
        }
    }
    let mut manifest = GpuClosedLoopBenchmarkManifest {
        schema_version: 1,
        git_commit: provenance.0,
        source_tree_digest: provenance.1,
        adapter,
        adapter_identity_digest_or_zero: observed.unwrap_or([0; 4]),
        protocol:
            alife_tools::benchmark::gpu_closed_loop::GpuClosedLoopBenchmarkProtocolV1::canonical(),
        rows,
        manifest_digest: [0; 4],
    };
    manifest.seal_digest()?;
    manifest.validate(&targets)?;
    atomic_write_json(&output, &manifest)?;
    fs::remove_dir_all(&staging)?;
    println!("{}", output.display());
    Ok(())
}

fn requested_targets(
    args: &[String],
    targets: &[GpuPerformanceTargetRowV1],
) -> Result<Vec<GpuPerformanceTargetRowV1>, Box<dyn std::error::Error>> {
    let classes = required_value(args, "--classes")?
        .split(',')
        .map(parse_class)
        .collect::<Result<Vec<_>, _>>()?;
    let profiles = required_value(args, "--sensor-profiles")?
        .split(',')
        .map(parse_profile)
        .collect::<Result<Vec<_>, _>>()?;
    let populations = required_value(args, "--populations")?
        .split(',')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()?;
    let mut requested = targets
        .iter()
        .copied()
        .filter(|target| {
            classes.contains(&target.class_id_raw)
                && profiles.contains(&target.sensor_profile_id_raw)
                && populations.contains(&target.population)
        })
        .collect::<Vec<_>>();
    requested.sort_unstable();
    Ok(requested)
}

fn clean_git_provenance() -> Result<(String, String), Box<dyn std::error::Error>> {
    let status = Command::new("git").args(["status", "--short"]).output()?;
    if !status.status.success() || !status.stdout.is_empty() {
        return Err(GpuBenchmarkError::Contract(
            "benchmark evidence requires a clean committed source tree",
        )
        .into());
    }
    let commit = git_text(&["rev-parse", "HEAD"])?;
    let tree = git_text(&["rev-parse", "HEAD^{tree}"])?;
    if !alife_tools::benchmark::gpu_closed_loop::is_lower_hex_oid(&commit)
        || !alife_tools::benchmark::gpu_closed_loop::is_lower_hex_oid(&tree)
    {
        return Err(GpuBenchmarkError::Contract("Git provenance is not a strict object ID").into());
    }
    Ok((commit, tree))
}

fn git_text(args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).output()?;
    if !output.status.success() {
        return Err(GpuBenchmarkError::Contract("Git provenance command failed").into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

fn atomic_write_json<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = fs::File::create(&temp)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    drop(file);
    if let Err(error) = atomic_replace(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(error.into());
    }
    Ok(())
}

#[cfg(not(windows))]
fn atomic_replace(temp: &Path, path: &Path) -> std::io::Result<()> {
    fs::rename(temp, path)
}

#[cfg(windows)]
fn atomic_replace(temp: &Path, path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let temp_wide = temp
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let path_wide = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: both paths are owned, NUL-terminated UTF-16 buffers that remain
    // alive for the duration of the Win32 call. No raw handle is retained.
    let replaced = unsafe {
        MoveFileExW(
            temp_wide.as_ptr(),
            path_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn parse_class(value: &str) -> Result<u16, GpuBenchmarkError> {
    match value {
        "n512" => Ok(1),
        "n1024" => Ok(2),
        "n2048" => Ok(3),
        _ => Err(GpuBenchmarkError::Contract("unknown benchmark class")),
    }
}

fn class_slug(value: u16) -> Result<&'static str, GpuBenchmarkError> {
    match value {
        1 => Ok("n512"),
        2 => Ok("n1024"),
        3 => Ok("n2048"),
        _ => Err(GpuBenchmarkError::Contract("unknown benchmark class")),
    }
}

fn parse_profile(value: &str) -> Result<u16, GpuBenchmarkError> {
    match value {
        "privileged-affordance-v1" => Ok(1),
        "grounded-object-slots-v1" => Ok(2),
        _ => Err(GpuBenchmarkError::Contract(
            "unknown benchmark sensor profile",
        )),
    }
}

fn profile_slug(value: u16) -> Result<&'static str, GpuBenchmarkError> {
    match value {
        1 => Ok("privileged-affordance-v1"),
        2 => Ok("grounded-object-slots-v1"),
        _ => Err(GpuBenchmarkError::Contract(
            "unknown benchmark sensor profile",
        )),
    }
}

fn value_after<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|window| window[0] == flag)
        .map(|window| window[1].as_str())
}

fn required_value<'a>(
    args: &'a [String],
    flag: &'static str,
) -> Result<&'a str, GpuBenchmarkError> {
    value_after(args, flag).ok_or(GpuBenchmarkError::Contract(flag))
}

fn required_single_value<'a>(
    args: &'a [String],
    flag: &'static str,
) -> Result<&'a str, GpuBenchmarkError> {
    let mut values = args
        .windows(2)
        .filter(|window| window[0] == flag)
        .map(|window| window[1].as_str());
    let value = values.next().ok_or(GpuBenchmarkError::Contract(flag))?;
    if values.next().is_some() {
        return Err(GpuBenchmarkError::Contract(
            "private benchmark row flag is duplicated",
        ));
    }
    Ok(value)
}

fn run_legacy_benchmark(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = value_after(args, "--out")
        .map(PathBuf::from)
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
        fs::create_dir_all(&output_dir)?;
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
            fs::write(&gpu_path, gpu_report.to_markdown())?;
        } else {
            fs::write(
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
            fs::write(&path, timing.to_markdown())?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
