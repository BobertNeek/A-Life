use std::path::PathBuf;

use alife_game_app::{
    default_environment_manifest_path, run_production_voxel_frontend_dry_run,
    ProductionFrontendProfileId, ProductionVoxelLaunchConfig, PRODUCTION_VOXEL_COMMAND,
};

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
        "{PRODUCTION_VOXEL_COMMAND} [--profile PROFILE] [--population N] [--resolution WIDTHxHEIGHT] [--brain-policy gpu-required] [--graphics-backend vulkan] [--require-gpu] [--developer-overlay] [--record-performance] [--smoke-seconds N] [--dry-run]\nprofiles: MinimumSettings30x30, MinSpecComfort1080p, Balanced1080p, HighSpecScaleUp, ResearchScale"
    )
}
