use std::path::PathBuf;

use alife_game_app::{
    default_environment_manifest_path, default_production_asset_manifest_path,
    run_production_voxel_frontend_dry_run, validate_production_assets, ProductionFrontendProfileId,
    ProductionVoxelLaunchConfig, PRODUCTION_VOXEL_COMMAND,
};

const VALIDATE_PRODUCTION_ASSETS_COMMAND: &str = "validate-production-assets";
const GPU_CLOSED_LOOP_ACCEPTANCE_COMMAND: &str = "gpu-closed-loop-acceptance";
const GPU_LEARNING_SLEEP_ACCEPTANCE_COMMAND: &str = "gpu-learning-sleep-acceptance";
const GPU_MEMORY_GROUNDING_ACCEPTANCE_COMMAND: &str = "gpu-memory-grounding-acceptance";
const GPU_CLOSED_LOOP_SOAK_COMMAND: &str = "gpu-closed-loop-soak";
const GPU_EVIDENCE_VALIDATE_COMMAND: &str = "gpu-evidence-validate";

#[cfg(feature = "gpu-runtime")]
struct GpuAcceptanceCli {
    options: alife_game_app::GpuClosedLoopAcceptanceOptions,
    output: PathBuf,
}

#[cfg(feature = "gpu-runtime")]
struct GpuLearningSleepCli {
    options: alife_game_app::GpuLearningSleepAcceptanceOptions,
    output: PathBuf,
}

#[cfg(feature = "gpu-runtime")]
struct GpuMemoryGroundingCli {
    options: alife_game_app::GpuMemoryGroundingAcceptanceOptions,
}

#[cfg(feature = "gpu-tests")]
struct GpuClosedLoopSoakCli {
    options: alife_game_app::GpuClosedLoopSoakOptions,
    output: PathBuf,
}

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let Some((command, rest)) = args.split_first() else {
        return Err(help());
    };
    if command == "--help" || command == "-h" {
        println!("{}", help());
        return Ok(());
    }
    if command == GPU_CLOSED_LOOP_ACCEPTANCE_COMMAND {
        if rest
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
        {
            println!("{}", help());
            return Ok(());
        }
        #[cfg(feature = "gpu-runtime")]
        {
            let parsed = parse_gpu_acceptance(rest)?;
            let receipt = alife_game_app::run_and_write_gpu_closed_loop_acceptance(
                parsed.options,
                &parsed.output,
            )
            .map_err(|error| error.to_string())?;
            println!(
                "Slice A GPU evidence class={} backend={} adapter={} ticks={} active_synapses={} readback_bytes={} replay_error={} artifact={} commit={} tree={}",
                receipt.capacity_class,
                receipt.backend_api,
                receipt.adapter_name,
                receipt.requested_ticks,
                receipt.active_synapses,
                receipt.compact_readback_bytes,
                receipt.replay.max_abs_error,
                parsed.output.display(),
                receipt.header.git_commit,
                receipt.header.source_tree_digest,
            );
            return Ok(());
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            return Err(format!(
                "{GPU_CLOSED_LOOP_ACCEPTANCE_COMMAND} requires --features gpu-runtime"
            ));
        }
    }
    if command == GPU_LEARNING_SLEEP_ACCEPTANCE_COMMAND {
        if rest
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
        {
            println!("{}", help());
            return Ok(());
        }
        #[cfg(feature = "gpu-runtime")]
        {
            let parsed = parse_gpu_learning_sleep(rest)?;
            let receipt = alife_game_app::run_and_write_gpu_learning_sleep_acceptance(
                parsed.options,
                &parsed.output,
            )
            .map_err(|error| error.to_string())?;
            println!(
                "Slice B GPU evidence class={} backend={} adapter={} reward_delta={} pain_delta={} replay_delta={} swaps={} artifact={} commit={} tree={}",
                receipt.capacity_class,
                receipt.backend_api,
                receipt.adapter_name,
                receipt.reward_target_delta,
                receipt.pain_avoidance_delta,
                receipt.replay_vs_zero_sample_post_wake_delta,
                receipt.restore.actual_remaining_swaps,
                parsed.output.display(),
                receipt.header.git_commit,
                receipt.header.source_tree_digest,
            );
            return Ok(());
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            return Err(format!(
                "{GPU_LEARNING_SLEEP_ACCEPTANCE_COMMAND} requires --features gpu-runtime"
            ));
        }
    }
    if command == GPU_MEMORY_GROUNDING_ACCEPTANCE_COMMAND {
        if rest
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
        {
            println!("{}", help());
            return Ok(());
        }
        #[cfg(feature = "gpu-runtime")]
        {
            let parsed = parse_gpu_memory_grounding(rest)?;
            let artifact_path = parsed
                .options
                .artifact_path()
                .map_err(|error| error.to_string())?;
            let receipt =
                alife_game_app::run_and_write_gpu_memory_grounding_acceptance(parsed.options)
                    .map_err(|error| error.to_string())?;
            println!(
                "Slice C GPU evidence class={} profile={} backend={} adapter={} ticks={} selections={} readback_bytes={} artifact={} commit={} tree={}",
                receipt.capacity_class_slug,
                receipt.sensor_profile.profile_id.raw(),
                receipt.hardware.backend_api,
                receipt.hardware.adapter_name,
                receipt.completed_ticks,
                receipt.gpu_selection_count,
                receipt.compact_readback_bytes,
                artifact_path.display(),
                receipt.header.common.git_commit,
                receipt.header.common.source_tree_digest,
            );
            return Ok(());
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            return Err(format!(
                "{GPU_MEMORY_GROUNDING_ACCEPTANCE_COMMAND} requires --features gpu-runtime"
            ));
        }
    }
    if command == GPU_CLOSED_LOOP_SOAK_COMMAND {
        if rest
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
        {
            println!("{}", help());
            return Ok(());
        }
        #[cfg(feature = "gpu-tests")]
        {
            let parsed = parse_gpu_closed_loop_soak(rest)?;
            let receipt =
                alife_game_app::run_and_write_gpu_closed_loop_soak(parsed.options, &parsed.output)
                    .map_err(|error| error.to_string())?;
            println!(
                "Slice D GPU evidence class={} profile={} backend={} adapter={} ticks={} dispatches={} learning_commits={} sleep_cycles={} artifact={} commit={} tree={}",
                receipt.capacity_class_slug,
                receipt.sensor_profile.profile_id.raw(),
                receipt.header.adapter_backend,
                receipt.header.adapter_name,
                receipt.completed_ticks,
                receipt.authoritative_gpu_dispatches,
                receipt.activity.learning_commits,
                receipt.save_restore.sleep_cycles,
                parsed.output.display(),
                receipt.header.common.git_commit,
                receipt.header.common.source_tree_digest,
            );
            return Ok(());
        }
        #[cfg(not(feature = "gpu-tests"))]
        {
            return Err(format!(
                "{GPU_CLOSED_LOOP_SOAK_COMMAND} requires --features gpu-tests"
            ));
        }
    }
    if command == GPU_EVIDENCE_VALIDATE_COMMAND {
        if rest
            .iter()
            .any(|argument| argument == "--help" || argument == "-h")
        {
            println!("{}", help());
            return Ok(());
        }
        #[cfg(feature = "gpu-runtime")]
        {
            let (slice_raw, input) = parse_gpu_evidence_validation(rest)?;
            let evidence = alife_game_app::validate_gpu_evidence_file(slice_raw, &input)
                .map_err(|error| error.to_string())?;
            let header = evidence.header();
            println!(
                "GPU evidence valid slice={} class={} backend={} adapter={} activity={} artifact_digest={:016x}{:016x}{:016x}{:016x}",
                header.slice_raw,
                evidence.capacity_class(),
                evidence.backend_api(),
                evidence.adapter_name(),
                evidence.activity_count(),
                header.artifact_digest[0],
                header.artifact_digest[1],
                header.artifact_digest[2],
                header.artifact_digest[3],
            );
            return Ok(());
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            return Err(format!(
                "{GPU_EVIDENCE_VALIDATE_COMMAND} requires --features gpu-runtime"
            ));
        }
    }
    if command == VALIDATE_PRODUCTION_ASSETS_COMMAND {
        if !rest.is_empty() {
            return Err(format!(
                "{VALIDATE_PRODUCTION_ASSETS_COMMAND} does not accept options\n{}",
                help()
            ));
        }
        println!("{}", production_asset_validation_receipt()?);
        return Ok(());
    }
    if command != PRODUCTION_VOXEL_COMMAND && command != "graphical-playground" {
        return Err(format!("unknown command: {command}\n{}", help()));
    }
    if rest
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!("{}", help());
        return Ok(());
    }
    let legacy_alias = command == "graphical-playground";
    let launch = parse_launch(rest, legacy_alias)?;
    let summary = if launch.dry_run {
        run_production_voxel_frontend_dry_run(&launch).map_err(|error| error.to_string())?
    } else {
        run_graphical(&launch)?
    };
    println!(
        "A-Life production voxel profile={} population={} backend={} adapter={} authoritative={} signature={}",
        summary.profile_id.label(),
        summary.effective_population,
        summary.diagnostics.selected_backend,
        summary.diagnostics.adapter_name.as_deref().unwrap_or("unavailable"),
        summary.diagnostics.authoritative,
        summary.signature_line(),
    );
    if legacy_alias {
        println!("legacy_alias=true routed_to={PRODUCTION_VOXEL_COMMAND}");
    }
    Ok(())
}

fn production_asset_validation_receipt() -> Result<String, String> {
    let summary = validate_production_assets(default_production_asset_manifest_path())
        .map_err(|error| error.to_string())?;
    Ok(format!(
        "FVR07 production assets schema={} version={} pack={} manifest={} assets={} generated={} external={} required_categories={} final_art={} placeholder_final={} unknown_license={} rejected={} committed_bytes={} largest={} generated_target={} loader={}:{} missing_policy={} vfx_profiles={} vfx_effects={} min_vfx={} comfort_vfx={} display_only_vfx={} adaptive_vfx={} no_large_artifacts={} renderer_authority_blocked={} scale_up_profiles={} signature={}",
        summary.schema,
        summary.schema_version,
        summary.pack_id,
        summary.manifest_path.display(),
        summary.asset_count,
        summary.generated_assets,
        summary.external_assets,
        summary.required_usage_categories_present,
        summary.final_art_entries,
        summary.placeholder_final_entries,
        summary.unknown_license_entries,
        summary.missing_or_rejected_assets,
        summary.committed_asset_bytes,
        summary.largest_asset_bytes,
        summary.generated_art_target,
        summary.loader_crate,
        summary.loader_version,
        summary.missing_asset_policy,
        summary.vfx_profile_count,
        summary.vfx_effects_present,
        summary.minimum_vfx_budget_state,
        summary.comfort_vfx_budget_state,
        summary.display_only_vfx,
        summary.adaptive_vfx,
        summary.no_large_artifacts_committed,
        summary.no_renderer_authority,
        summary.scale_up_profiles_present,
        summary.signature_line(),
    ))
}

fn parse_launch(
    args: &[String],
    legacy_alias: bool,
) -> Result<ProductionVoxelLaunchConfig, String> {
    let mut manifest = default_environment_manifest_path();
    let mut scenario = None::<String>;
    let mut profile = ProductionFrontendProfileId::default();
    let mut population = None;
    let mut resolution = None;
    let mut graphics_backend = if cfg!(windows) { "vulkan" } else { "auto" }.to_string();
    let mut smoke_seconds = None;
    let mut dry_run = false;
    let mut record_performance = false;
    let mut require_gpu = false;
    let mut developer_overlay = false;
    let mut ui_settings_path = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--manifest" => {
                manifest = PathBuf::from(value(args, index, "--manifest")?);
                index += 2;
            }
            "--scenario" => {
                scenario = Some(value(args, index, "--scenario")?.to_string());
                index += 2;
            }
            "--profile" => {
                profile = ProductionFrontendProfileId::parse(value(args, index, "--profile")?)
                    .map_err(|error| error.to_string())?;
                index += 2;
            }
            "--population" => {
                population = Some(
                    value(args, index, "--population")?
                        .parse()
                        .map_err(|_| "--population must be an unsigned integer".to_string())?,
                );
                index += 2;
            }
            "--resolution" => {
                resolution = Some(parse_resolution(value(args, index, "--resolution")?)?);
                index += 2;
            }
            "--brain-policy" => {
                if value(args, index, "--brain-policy")? != "gpu-required" {
                    return Err("product brain policy must be gpu-required".to_string());
                }
                index += 2;
            }
            "--graphics-backend" => {
                graphics_backend = value(args, index, "--graphics-backend")?.to_string();
                index += 2;
            }
            "--smoke-seconds" => {
                smoke_seconds = Some(
                    value(args, index, "--smoke-seconds")?
                        .parse()
                        .map_err(|_| "--smoke-seconds must be an unsigned integer".to_string())?,
                );
                index += 2;
            }
            "--ui-settings" => {
                ui_settings_path = Some(PathBuf::from(value(args, index, "--ui-settings")?));
                index += 2;
            }
            "--dry-run" => {
                dry_run = true;
                index += 1;
            }
            "--record-performance" => {
                record_performance = true;
                index += 1;
            }
            "--require-gpu" => {
                require_gpu = true;
                index += 1;
            }
            "--developer-overlay" => {
                developer_overlay = true;
                index += 1;
            }
            "--view-mode" if legacy_alias => {
                index += 2;
            }
            unknown if legacy_alias && !unknown.starts_with("--") => {
                index += 1;
            }
            unknown => return Err(format!("unknown production option: {unknown}")),
        }
    }
    let mut launch =
        ProductionVoxelLaunchConfig::from_manifest(&manifest, scenario.as_deref(), profile)
            .map_err(|error| error.to_string())?;
    launch.population = population;
    if let Some(resolution) = resolution {
        launch.resolution = resolution;
    }
    launch.graphics_backend = graphics_backend;
    launch.smoke_seconds = smoke_seconds;
    launch.dry_run = dry_run;
    launch.record_performance = record_performance;
    launch.require_gpu = require_gpu;
    launch.developer_overlay = developer_overlay;
    launch.legacy_alias = legacy_alias;
    launch.ui_settings_path = ui_settings_path;
    Ok(launch)
}

fn value<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_resolution(value: &str) -> Result<(u32, u32), String> {
    let (width, height) = value
        .split_once('x')
        .ok_or_else(|| "--resolution must use WIDTHxHEIGHT".to_string())?;
    let width = width
        .parse()
        .map_err(|_| "invalid resolution width".to_string())?;
    let height = height
        .parse()
        .map_err(|_| "invalid resolution height".to_string())?;
    if width == 0 || height == 0 {
        return Err("resolution dimensions must be nonzero".to_string());
    }
    Ok((width, height))
}

#[cfg(feature = "gpu-runtime")]
fn parse_gpu_acceptance(args: &[String]) -> Result<GpuAcceptanceCli, String> {
    let mut capacity = None;
    let mut ticks = None;
    let mut seed = None;
    let mut sensor_profile = None;
    let mut output = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--class" => {
                set_once(
                    &mut capacity,
                    match value(args, index, flag)? {
                        "n512" => alife_core::BrainCapacityClass::n512(),
                        "n1024" => alife_core::BrainCapacityClass::n1024(),
                        "n2048" => alife_core::BrainCapacityClass::n2048(),
                        _ => return Err("--class must be n512, n1024, or n2048".to_string()),
                    },
                    flag,
                )?;
                index += 2;
            }
            "--ticks" => {
                let parsed = value(args, index, flag)?
                    .parse::<u32>()
                    .map_err(|_| "--ticks must be an unsigned integer".to_string())?;
                set_once(&mut ticks, parsed, flag)?;
                index += 2;
            }
            "--seed" => {
                let parsed = value(args, index, flag)?
                    .parse::<u64>()
                    .map_err(|_| "--seed must be an unsigned integer".to_string())?;
                set_once(&mut seed, parsed, flag)?;
                index += 2;
            }
            "--sensor-profile" => {
                let parsed = match value(args, index, flag)? {
                    "privileged-affordance-v1" => alife_core::SensorProfile::PrivilegedAffordanceV1,
                    _ => {
                        return Err("--sensor-profile must be privileged-affordance-v1".to_string());
                    }
                };
                set_once(&mut sensor_profile, parsed, flag)?;
                index += 2;
            }
            "--output" => {
                let parsed = PathBuf::from(value(args, index, flag)?);
                set_once(&mut output, parsed, flag)?;
                index += 2;
            }
            unknown => return Err(format!("unknown GPU acceptance option: {unknown}")),
        }
    }
    Ok(GpuAcceptanceCli {
        options: alife_game_app::GpuClosedLoopAcceptanceOptions {
            capacity: capacity.ok_or_else(|| "--class is required".to_string())?,
            requested_ticks: ticks.ok_or_else(|| "--ticks is required".to_string())?,
            deterministic_seed: seed.ok_or_else(|| "--seed is required".to_string())?,
            sensor_profile: sensor_profile
                .ok_or_else(|| "--sensor-profile is required".to_string())?,
        },
        output: output.ok_or_else(|| "--output is required".to_string())?,
    })
}

#[cfg(feature = "gpu-runtime")]
fn parse_gpu_learning_sleep(args: &[String]) -> Result<GpuLearningSleepCli, String> {
    let mut capacity = None;
    let mut seed = None;
    let mut output = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--class" => {
                set_once(
                    &mut capacity,
                    match value(args, index, flag)? {
                        "n512" => alife_core::BrainCapacityClass::n512(),
                        "n1024" => alife_core::BrainCapacityClass::n1024(),
                        "n2048" => alife_core::BrainCapacityClass::n2048(),
                        _ => return Err("--class must be n512, n1024, or n2048".to_string()),
                    },
                    flag,
                )?;
                index += 2;
            }
            "--seed" => {
                let parsed = value(args, index, flag)?
                    .parse::<u64>()
                    .map_err(|_| "--seed must be an unsigned integer".to_string())?;
                set_once(&mut seed, parsed, flag)?;
                index += 2;
            }
            "--output" => {
                set_once(&mut output, PathBuf::from(value(args, index, flag)?), flag)?;
                index += 2;
            }
            unknown => return Err(format!("unknown GPU learning/sleep option: {unknown}")),
        }
    }
    Ok(GpuLearningSleepCli {
        options: alife_game_app::GpuLearningSleepAcceptanceOptions {
            capacity: capacity.ok_or_else(|| "--class is required".to_string())?,
            deterministic_seed: seed.ok_or_else(|| "--seed is required".to_string())?,
        },
        output: output.ok_or_else(|| "--output is required".to_string())?,
    })
}

#[cfg(feature = "gpu-runtime")]
fn parse_gpu_memory_grounding(args: &[String]) -> Result<GpuMemoryGroundingCli, String> {
    let mut capacity = None;
    let mut ticks = None;
    let mut seed = None;
    let mut sensor_profile = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--class" => {
                set_once(
                    &mut capacity,
                    match value(args, index, flag)? {
                        "n512" => alife_core::BrainCapacityClass::n512(),
                        "n1024" => alife_core::BrainCapacityClass::n1024(),
                        "n2048" => alife_core::BrainCapacityClass::n2048(),
                        _ => return Err("--class must be n512, n1024, or n2048".to_string()),
                    },
                    flag,
                )?;
                index += 2;
            }
            "--ticks" => {
                let parsed = value(args, index, flag)?
                    .parse::<u64>()
                    .map_err(|_| "--ticks must be an unsigned integer".to_string())?;
                set_once(&mut ticks, parsed, flag)?;
                index += 2;
            }
            "--seed" => {
                let parsed = value(args, index, flag)?
                    .parse::<u64>()
                    .map_err(|_| "--seed must be an unsigned integer".to_string())?;
                set_once(&mut seed, parsed, flag)?;
                index += 2;
            }
            "--sensor-profile" => {
                let parsed = match value(args, index, flag)? {
                    "grounded-object-slots-v1" => alife_core::SensorProfile::GroundedObjectSlotsV1,
                    "privileged-affordance-v1" => alife_core::SensorProfile::PrivilegedAffordanceV1,
                    _ => {
                        return Err(
                            "--sensor-profile must be grounded-object-slots-v1 or privileged-affordance-v1"
                                .to_string(),
                        );
                    }
                };
                set_once(&mut sensor_profile, parsed, flag)?;
                index += 2;
            }
            unknown => return Err(format!("unknown GPU memory/grounding option: {unknown}")),
        }
    }
    let parsed = GpuMemoryGroundingCli {
        options: alife_game_app::GpuMemoryGroundingAcceptanceOptions {
            capacity: capacity.ok_or_else(|| "--class is required".to_string())?,
            requested_ticks: ticks.ok_or_else(|| "--ticks is required".to_string())?,
            deterministic_seed: seed.ok_or_else(|| "--seed is required".to_string())?,
            sensor_profile: sensor_profile
                .ok_or_else(|| "--sensor-profile is required".to_string())?,
        },
    };
    parsed
        .options
        .artifact_path()
        .map_err(|error| error.to_string())?;
    Ok(parsed)
}

#[cfg(feature = "gpu-tests")]
fn parse_gpu_closed_loop_soak(args: &[String]) -> Result<GpuClosedLoopSoakCli, String> {
    let mut capacity = None;
    let mut ticks = None;
    let mut seed = None;
    let mut sensor_profile = None;
    let mut output = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--class" => {
                set_once(
                    &mut capacity,
                    match value(args, index, flag)? {
                        "n512" => alife_core::BrainCapacityClass::n512(),
                        "n1024" => alife_core::BrainCapacityClass::n1024(),
                        "n2048" => alife_core::BrainCapacityClass::n2048(),
                        _ => return Err("--class must be n512, n1024, or n2048".to_string()),
                    },
                    flag,
                )?;
                index += 2;
            }
            "--ticks" => {
                let parsed = value(args, index, flag)?
                    .parse::<u64>()
                    .map_err(|_| "--ticks must be an unsigned integer".to_string())?;
                set_once(&mut ticks, parsed, flag)?;
                index += 2;
            }
            "--seed" => {
                let parsed = value(args, index, flag)?
                    .parse::<u64>()
                    .map_err(|_| "--seed must be an unsigned integer".to_string())?;
                set_once(&mut seed, parsed, flag)?;
                index += 2;
            }
            "--sensor-profile" => {
                let parsed = match value(args, index, flag)? {
                    "grounded-object-slots-v1" => alife_core::SensorProfile::GroundedObjectSlotsV1,
                    "privileged-affordance-v1" => alife_core::SensorProfile::PrivilegedAffordanceV1,
                    _ => {
                        return Err(
                            "--sensor-profile must be grounded-object-slots-v1 or privileged-affordance-v1"
                                .to_string(),
                        );
                    }
                };
                set_once(&mut sensor_profile, parsed, flag)?;
                index += 2;
            }
            "--output" => {
                set_once(&mut output, PathBuf::from(value(args, index, flag)?), flag)?;
                index += 2;
            }
            unknown => return Err(format!("unknown GPU soak option: {unknown}")),
        }
    }
    let parsed = GpuClosedLoopSoakCli {
        options: alife_game_app::GpuClosedLoopSoakOptions {
            capacity: capacity.ok_or_else(|| "--class is required".to_string())?,
            sensor_profile: sensor_profile
                .ok_or_else(|| "--sensor-profile is required".to_string())?,
            completed_ticks: ticks.ok_or_else(|| "--ticks is required".to_string())?,
            deterministic_seed: seed.ok_or_else(|| "--seed is required".to_string())?,
        },
        output: output.ok_or_else(|| "--output is required".to_string())?,
    };
    let expected = parsed
        .options
        .artifact_path()
        .map_err(|error| error.to_string())?;
    if parsed.output.file_name() != expected.file_name() {
        return Err("--output filename must match the Slice D profile/class slug".to_string());
    }
    Ok(parsed)
}

#[cfg(feature = "gpu-runtime")]
fn parse_gpu_evidence_validation(args: &[String]) -> Result<(u16, PathBuf), String> {
    let mut slice = None;
    let mut input = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        match flag {
            "--slice" => {
                let parsed = match value(args, index, flag)? {
                    "a" => alife_game_app::GPU_SLICE_A_RAW,
                    "b" => alife_game_app::GPU_SLICE_B_RAW,
                    "c" => alife_game_app::GPU_SLICE_C_RAW,
                    "d" => alife_game_app::GPU_SLICE_D_RAW,
                    _ => return Err("--slice must be a, b, c, or d".to_string()),
                };
                set_once(&mut slice, parsed, flag)?;
                index += 2;
            }
            "--input" => {
                set_once(&mut input, PathBuf::from(value(args, index, flag)?), flag)?;
                index += 2;
            }
            unknown => return Err(format!("unknown GPU evidence option: {unknown}")),
        }
    }
    Ok((
        slice.ok_or_else(|| "--slice is required".to_string())?,
        input.ok_or_else(|| "--input is required".to_string())?,
    ))
}

#[cfg(feature = "gpu-runtime")]
fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("{flag} may be provided only once"))
    } else {
        Ok(())
    }
}

#[cfg(feature = "bevy-app")]
fn run_graphical(
    launch: &ProductionVoxelLaunchConfig,
) -> Result<alife_game_app::ProductionVoxelLaunchSummary, String> {
    alife_game_app::bevy_shell::run_production_voxel_frontend_window(launch)
        .map_err(|error| error.to_string())
}

#[cfg(not(feature = "bevy-app"))]
fn run_graphical(
    _: &ProductionVoxelLaunchConfig,
) -> Result<alife_game_app::ProductionVoxelLaunchSummary, String> {
    Err("production window requires the bevy-app feature".to_string())
}

fn help() -> String {
    format!(
        "{PRODUCTION_VOXEL_COMMAND} [--profile PROFILE] [--population N] [--resolution WIDTHxHEIGHT] [--brain-policy gpu-required] [--graphics-backend vulkan] [--require-gpu] [--developer-overlay] [--record-performance] [--smoke-seconds N] [--dry-run]\n{VALIDATE_PRODUCTION_ASSETS_COMMAND}\n{GPU_CLOSED_LOOP_ACCEPTANCE_COMMAND} --class n512|n1024|n2048 --ticks N --seed N --sensor-profile privileged-affordance-v1 --output PATH\n{GPU_LEARNING_SLEEP_ACCEPTANCE_COMMAND} --class n512|n1024|n2048 --seed N --output PATH\n{GPU_MEMORY_GROUNDING_ACCEPTANCE_COMMAND} --class n512|n1024|n2048 --ticks 64|10240 --seed N --sensor-profile privileged-affordance-v1|grounded-object-slots-v1\n{GPU_CLOSED_LOOP_SOAK_COMMAND} --class n512|n1024|n2048 --ticks 10240 --seed N --sensor-profile privileged-affordance-v1|grounded-object-slots-v1 --output PATH\n{GPU_EVIDENCE_VALIDATE_COMMAND} --slice a|b|c|d --input PATH\nprofiles: MinimumSettings30x30, MinSpecComfort1080p, Balanced1080p, HighSpecScaleUp, ResearchScale"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_asset_validation_command_remains_available() {
        let receipt = production_asset_validation_receipt().unwrap();
        assert!(receipt.starts_with("FVR07 production assets"));
        assert!(receipt.contains("unknown_license=0"));
        assert!(receipt.contains("renderer_authority_blocked=true"));
        assert!(help().contains(VALIDATE_PRODUCTION_ASSETS_COMMAND));
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn parse_gpu_evidence_cli_requires_the_complete_class_bound_contract() {
        let args = [
            "--class",
            "n2048",
            "--ticks",
            "64",
            "--seed",
            "4101",
            "--sensor-profile",
            "privileged-affordance-v1",
            "--output",
            "target/artifacts/gpu-closed-loop-slice-a-n2048.json",
        ]
        .map(str::to_string);
        let parsed = parse_gpu_acceptance(&args).unwrap();

        assert_eq!(
            parsed.options.capacity,
            alife_core::BrainCapacityClass::n2048()
        );
        assert_eq!(parsed.options.requested_ticks, 64);
        assert_eq!(parsed.options.deterministic_seed, 4_101);
        assert!(parsed
            .output
            .ends_with("gpu-closed-loop-slice-a-n2048.json"));
        assert!(help().contains(GPU_CLOSED_LOOP_ACCEPTANCE_COMMAND));
        assert!(help().contains(GPU_EVIDENCE_VALIDATE_COMMAND));
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn parse_gpu_learning_sleep_cli_requires_exact_class_seed_and_output() {
        let args = [
            "--class",
            "n2048",
            "--seed",
            "4202",
            "--output",
            "target/artifacts/gpu-learning-sleep-slice-b-n2048.json",
        ]
        .map(str::to_string);
        let parsed = parse_gpu_learning_sleep(&args).unwrap();

        assert_eq!(
            parsed.options.capacity,
            alife_core::BrainCapacityClass::n2048()
        );
        assert_eq!(parsed.options.deterministic_seed, 4_202);
        assert!(parsed
            .output
            .ends_with("gpu-learning-sleep-slice-b-n2048.json"));
        assert!(parse_gpu_learning_sleep(&args[..4]).is_err());
        assert!(parse_gpu_learning_sleep(
            &[
                "--class",
                "n4096",
                "--seed",
                "4202",
                "--output",
                "receipt.json",
            ]
            .map(str::to_string),
        )
        .is_err());
        assert!(help().contains(GPU_LEARNING_SLEEP_ACCEPTANCE_COMMAND));
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn parse_gpu_memory_grounding_cli_derives_profile_qualified_artifacts() {
        let grounded_args = [
            "--class",
            "n512",
            "--ticks",
            "10240",
            "--seed",
            "4303",
            "--sensor-profile",
            "grounded-object-slots-v1",
        ]
        .map(str::to_string);
        let grounded = parse_gpu_memory_grounding(&grounded_args).unwrap();

        assert_eq!(
            grounded.options.capacity,
            alife_core::BrainCapacityClass::n512()
        );
        assert_eq!(grounded.options.requested_ticks, 10_240);
        assert_eq!(grounded.options.deterministic_seed, 4_303);
        assert_eq!(
            grounded.options.sensor_profile,
            alife_core::SensorProfile::GroundedObjectSlotsV1
        );
        assert!(grounded
            .options
            .artifact_path()
            .unwrap()
            .ends_with("gpu-memory-grounding-slice-c-grounded-object-slots-v1-n512.json"));

        let privileged_args = [
            "--class",
            "n2048",
            "--ticks",
            "64",
            "--seed",
            "4303",
            "--sensor-profile",
            "privileged-affordance-v1",
        ]
        .map(str::to_string);
        let privileged = parse_gpu_memory_grounding(&privileged_args).unwrap();
        assert_eq!(privileged.options.requested_ticks, 64);
        assert_eq!(
            privileged.options.sensor_profile,
            alife_core::SensorProfile::PrivilegedAffordanceV1
        );
        assert!(privileged
            .options
            .artifact_path()
            .unwrap()
            .ends_with("gpu-memory-grounding-slice-c-privileged-affordance-v1-n2048.json"));
        assert!(help().contains(GPU_MEMORY_GROUNDING_ACCEPTANCE_COMMAND));
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn parse_gpu_memory_grounding_cli_rejects_incomplete_or_invalid_contracts() {
        let valid = [
            "--class",
            "n1024",
            "--ticks",
            "10240",
            "--seed",
            "4303",
            "--sensor-profile",
            "grounded-object-slots-v1",
        ]
        .map(str::to_string);

        assert!(parse_gpu_memory_grounding(&valid[..6]).is_err());
        assert!(parse_gpu_memory_grounding(
            &[
                "--class",
                "n4096",
                "--ticks",
                "10240",
                "--seed",
                "4303",
                "--sensor-profile",
                "grounded-object-slots-v1",
            ]
            .map(str::to_string),
        )
        .is_err());
        assert!(parse_gpu_memory_grounding(
            &[
                "--class",
                "n512",
                "--ticks",
                "64",
                "--seed",
                "4303",
                "--sensor-profile",
                "grounded-object-slots-v1",
            ]
            .map(str::to_string),
        )
        .is_err());
        assert!(parse_gpu_memory_grounding(
            &[
                "--class",
                "n512",
                "--ticks",
                "10240",
                "--seed",
                "4303",
                "--sensor-profile",
                "unknown-profile",
            ]
            .map(str::to_string),
        )
        .is_err());
        let mut duplicate = valid.to_vec();
        duplicate.extend(["--seed".to_string(), "4304".to_string()]);
        assert!(parse_gpu_memory_grounding(&duplicate).is_err());
        let mut output_override = valid.to_vec();
        output_override.extend(["--output".to_string(), "receipt.json".to_string()]);
        assert!(parse_gpu_memory_grounding(&output_override).is_err());
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn parse_gpu_closed_loop_soak_cli_requires_exact_slice_d_contract() {
        let args = [
            "--class",
            "n2048",
            "--ticks",
            "10240",
            "--seed",
            "4505",
            "--sensor-profile",
            "grounded-object-slots-v1",
            "--output",
            "target/artifacts/gpu-closed-loop-slice-d-grounded-object-slots-v1-n2048.json",
        ]
        .map(str::to_string);
        let parsed = parse_gpu_closed_loop_soak(&args).unwrap();

        assert_eq!(
            parsed.options.capacity,
            alife_core::BrainCapacityClass::n2048()
        );
        assert_eq!(parsed.options.completed_ticks, 10_240);
        assert_eq!(parsed.options.deterministic_seed, 4_505);
        assert_eq!(
            parsed.options.sensor_profile,
            alife_core::SensorProfile::GroundedObjectSlotsV1
        );
        assert!(parsed
            .output
            .ends_with("gpu-closed-loop-slice-d-grounded-object-slots-v1-n2048.json"));
        assert!(parse_gpu_closed_loop_soak(&args[..8]).is_err());

        let mut wrong_ticks = args.to_vec();
        wrong_ticks[3] = "64".to_string();
        assert!(parse_gpu_closed_loop_soak(&wrong_ticks).is_err());

        let mut wrong_output = args.to_vec();
        wrong_output[9] = "target/artifacts/receipt.json".to_string();
        assert!(parse_gpu_closed_loop_soak(&wrong_output).is_err());
        assert!(help().contains(GPU_CLOSED_LOOP_SOAK_COMMAND));
    }

    #[cfg(feature = "gpu-runtime")]
    #[test]
    fn parse_gpu_evidence_validation_accepts_all_reviewed_slices_only_with_input() {
        let args = [
            "--slice",
            "b",
            "--input",
            "target/artifacts/gpu-learning-sleep-slice-b-n512.json",
        ]
        .map(str::to_string);
        let (slice, input) = parse_gpu_evidence_validation(&args).unwrap();

        assert_eq!(slice, alife_game_app::GPU_SLICE_B_RAW);
        assert!(input.ends_with("gpu-learning-sleep-slice-b-n512.json"));
        assert!(parse_gpu_evidence_validation(&args[..2]).is_err());
        let (slice_c, input_c) = parse_gpu_evidence_validation(
            &["--slice", "c", "--input", "receipt.json"].map(str::to_string),
        )
        .unwrap();
        assert_eq!(slice_c, alife_game_app::GPU_SLICE_C_RAW);
        assert!(input_c.ends_with("receipt.json"));
        let (slice_d, input_d) = parse_gpu_evidence_validation(
            &["--slice", "d", "--input", "soak.json"].map(str::to_string),
        )
        .unwrap();
        assert_eq!(slice_d, alife_game_app::GPU_SLICE_D_RAW);
        assert!(input_d.ends_with("soak.json"));
        assert!(help().contains("--slice a|b|c|d"));
    }
}
