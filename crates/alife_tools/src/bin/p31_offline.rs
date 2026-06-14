use std::{path::PathBuf, process};

use alife_tools::p31_offline_tools::{
    analyze_trace_file, default_etf_prototype_table, default_lobe_asset_path,
    generate_simplex_etf_prototypes, read_etf_prototype_table, write_default_lobe_asset,
    write_etf_prototype_table, write_nc_summary_json, EtfGeneratorConfig, NeuralCollapseSummary,
    TraceLoadSummary, P31_ETF_PROTOTYPE_SCHEMA_VERSION, P31_LOBE_ASSET_SCHEMA,
    P31_LOBE_ASSET_SCHEMA_VERSION, P31_NC_REPORT_SCHEMA, P31_NC_REPORT_SCHEMA_VERSION,
};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match run(args) {
        Ok(()) => {}
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut args = args.into_iter();
    let command = args.next().ok_or_else(usage)?;
    let command_args = args.collect::<Vec<_>>();
    match command.as_str() {
        "generate" => run_generate(&command_args),
        "analyze-trace" => run_analyze_trace(&command_args),
        "write-lobe-asset" => run_write_lobe_asset(&command_args),
        "help" | "--help" | "-h" => {
            println!("{}", usage_text());
            Ok(())
        }
        _ => Err(format!("unknown command `{command}`\n{}", usage_text())),
    }
}

#[derive(Debug, Default, PartialEq)]
struct GenerateOptions {
    classes: Option<usize>,
    dimension: Option<usize>,
    output: Option<PathBuf>,
    source: Option<String>,
}

#[derive(Debug, Default, PartialEq)]
struct AnalyzeTraceOptions {
    trace: Option<PathBuf>,
    prototypes: Option<PathBuf>,
    output: Option<PathBuf>,
}

#[derive(Debug, Default, PartialEq)]
struct WriteLobeAssetOptions {
    prototypes: Option<PathBuf>,
    classes: Option<usize>,
    dimension: Option<usize>,
    output: Option<PathBuf>,
}

fn run_generate(args: &[String]) -> Result<(), String> {
    let options = parse_generate_args(args)?;
    let source = options
        .source
        .unwrap_or_else(|| "alife_tools::p31_offline".to_string())
        .leak();
    let table = generate_simplex_etf_prototypes(EtfGeneratorConfig {
        class_count: options.classes.unwrap_or(10),
        embedding_dimension: options.dimension.unwrap_or(64),
        source,
    })
    .map_err(|error| error.to_string())?;
    let output = options.output.unwrap_or_else(|| {
        PathBuf::from("target").join("artifacts").join(format!(
            "p31_etf_prototypes_v{P31_ETF_PROTOTYPE_SCHEMA_VERSION}.json"
        ))
    });
    write_etf_prototype_table(&table, &output).map_err(|error| error.to_string())?;
    println!("generated prototype table: {}", output.display());
    println!(
        "schema: {} v{}",
        alife_tools::p31_offline_tools::P31_ETF_PROTOTYPE_SCHEMA,
        P31_ETF_PROTOTYPE_SCHEMA_VERSION
    );
    println!("classes: {}", table.classes.len());
    println!("embedding_dimension: {}", table.embedding_dimension);
    Ok(())
}

fn run_analyze_trace(args: &[String]) -> Result<(), String> {
    let options = parse_analyze_trace_args(args)?;
    let trace_path = options
        .trace
        .ok_or_else(|| "analyze-trace requires --trace <json path>".to_string())?;
    let prototypes = if let Some(path) = options.prototypes {
        read_etf_prototype_table(&path).map_err(|error| error.to_string())?
    } else {
        default_etf_prototype_table().map_err(|error| error.to_string())?
    };
    let (summary, load_summary) = analyze_trace_file(&trace_path, &prototypes)
        .map_err(|error| format!("trace analysis failed: {error}"))?;
    print_summary(&summary, &load_summary);
    if let Some(path) = options.output {
        write_nc_summary_json(&summary, &path).map_err(|error| error.to_string())?;
        println!("wrote summary json: {}", path.display());
    }
    Ok(())
}

fn run_write_lobe_asset(args: &[String]) -> Result<(), String> {
    let options = parse_write_lobe_asset_args(args)?;
    let table = if let Some(path) = options.prototypes {
        read_etf_prototype_table(&path).map_err(|error| error.to_string())?
    } else {
        generate_simplex_etf_prototypes(EtfGeneratorConfig {
            class_count: options.classes.unwrap_or(10),
            embedding_dimension: options.dimension.unwrap_or(64),
            source: "alife_tools::p31_offline",
        })
        .map_err(|error| error.to_string())?
    };
    let output = options.output.unwrap_or_else(|| {
        default_lobe_asset_path(PathBuf::from("target").join("artifacts").as_path())
    });
    write_default_lobe_asset(&output, &table).map_err(|error| error.to_string())?;
    println!("wrote sensory lobe prototype asset: {}", output.display());
    println!(
        "schema: {} v{}, generated_for: p08+p14 sensory projection prototypes",
        P31_LOBE_ASSET_SCHEMA, P31_LOBE_ASSET_SCHEMA_VERSION
    );
    Ok(())
}

fn parse_generate_args(args: &[String]) -> Result<GenerateOptions, String> {
    let mut output = GenerateOptions::default();

    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--classes" => {
                index += 1;
                output.classes = Some(next_usize_arg(args, &mut index, "--classes")?);
            }
            "--dimension" => {
                index += 1;
                output.dimension = Some(next_usize_arg(args, &mut index, "--dimension")?);
            }
            "--out" | "--output" => {
                index += 1;
                output.output = Some(next_path_arg(args, &mut index, "--out/--output")?);
            }
            "--source" => {
                index += 1;
                output.source = Some(
                    args.get(index)
                        .ok_or_else(|| "--source requires a value".to_string())?
                        .to_string(),
                );
            }
            "--help" | "-h" => {
                println!("{}", generate_usage());
                process::exit(0);
            }
            _ => {
                return Err(format!(
                    "unknown `generate` argument `{}`\n{}",
                    args[index],
                    generate_usage()
                ));
            }
        }
        index += 1;
    }

    Ok(output)
}

fn parse_analyze_trace_args(args: &[String]) -> Result<AnalyzeTraceOptions, String> {
    let mut output = AnalyzeTraceOptions::default();

    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--trace" => {
                index += 1;
                output.trace = Some(next_path_arg(args, &mut index, "--trace")?);
            }
            "--prototypes" => {
                index += 1;
                output.prototypes = Some(next_path_arg(args, &mut index, "--prototypes")?);
            }
            "--out" | "--output" => {
                index += 1;
                output.output = Some(next_path_arg(args, &mut index, "--out/--output")?);
            }
            "--help" | "-h" => {
                println!("{}", analyze_trace_usage());
                process::exit(0);
            }
            _ => {
                return Err(format!(
                    "unknown `analyze-trace` argument `{}`\n{}",
                    args[index],
                    analyze_trace_usage()
                ));
            }
        }
        index += 1;
    }

    Ok(output)
}

fn parse_write_lobe_asset_args(args: &[String]) -> Result<WriteLobeAssetOptions, String> {
    let mut output = WriteLobeAssetOptions::default();

    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--prototypes" => {
                index += 1;
                output.prototypes = Some(next_path_arg(args, &mut index, "--prototypes")?);
            }
            "--classes" => {
                index += 1;
                output.classes = Some(next_usize_arg(args, &mut index, "--classes")?);
            }
            "--dimension" => {
                index += 1;
                output.dimension = Some(next_usize_arg(args, &mut index, "--dimension")?);
            }
            "--out" | "--output" => {
                index += 1;
                output.output = Some(next_path_arg(args, &mut index, "--out/--output")?);
            }
            "--help" | "-h" => {
                println!("{}", write_lobe_asset_usage());
                process::exit(0);
            }
            _ => {
                return Err(format!(
                    "unknown `write-lobe-asset` argument `{}`\n{}",
                    args[index],
                    write_lobe_asset_usage()
                ));
            }
        }
        index += 1;
    }

    Ok(output)
}

fn next_usize_arg(args: &[String], index: &mut usize, flag: &str) -> Result<usize, String> {
    let raw = args
        .get(*index)
        .ok_or_else(|| format!("`{flag}` expects a positive integer value"))?;
    raw.parse::<usize>()
        .map_err(|_| format!("`{flag}` expects a positive integer, got `{raw}`"))
}

fn next_path_arg(args: &[String], index: &mut usize, flag: &str) -> Result<PathBuf, String> {
    let raw = args
        .get(*index)
        .ok_or_else(|| format!("`{flag}` requires a value"))?;
    Ok(PathBuf::from(raw))
}

fn print_summary(summary: &NeuralCollapseSummary, load_summary: &TraceLoadSummary) {
    println!(
        "summary schema: {} v{}",
        P31_NC_REPORT_SCHEMA, P31_NC_REPORT_SCHEMA_VERSION
    );
    println!(
        "sample_count={}, class_count={}",
        summary.sample_count, summary.class_count
    );
    if let Some(simplex) = &summary.between_class_simplex {
        println!(
            "between-class: pairs={}, target_offdiag_dot={:?}, observed_dot={:.5}, mean_angle={:.3}",
            simplex.class_pairs,
            simplex.target_offdiag_dot,
            simplex.observed_mean_dot,
            simplex.mean_angle_deg
        );
    }
    if let Some(drift) = &summary.drift {
        println!(
            "drift: points={}, mean_l2={:.5}, min_l2={:.5}, max_l2={:.5}",
            drift.point_count, drift.mean_drift_l2, drift.min_drift_l2, drift.max_drift_l2
        );
    }
    if load_summary.synthetic {
        println!("note: synthetic activations were used");
    }
    if let Some(notice) = &summary.synthetic_notice {
        println!("note: {notice}");
    }
    for warning in &summary.warnings {
        println!("warning: {warning}");
    }
}

fn usage() -> String {
    usage_text().to_string()
}

fn usage_text() -> &'static str {
    "\
Usage:
  p31_offline generate [--classes <n>] [--dimension <n>] [--out <file>] [--source <label>]
  p31_offline analyze-trace --trace <trace_json> [--prototypes <table_json>] [--out <summary_json>]
  p31_offline write-lobe-asset [--prototypes <table_json>] [--classes <n>] [--dimension <n>] [--out <file>]
  p31_offline help"
}

fn generate_usage() -> &'static str {
    usage_text()
}

fn analyze_trace_usage() -> &'static str {
    "\
Usage:
  p31_offline analyze-trace --trace <trace_json> [--prototypes <table_json>] [--out <summary_json>]

Supports packed log arrays and P18/P19-like trace envelopes."
}

fn write_lobe_asset_usage() -> &'static str {
    "\
Usage:
  p31_offline write-lobe-asset [--prototypes <table_json>] [--classes <n>] [--dimension <n>] [--out <file>]

If --prototypes is omitted, a simplex prototype table is generated."
}

#[cfg(test)]
mod tests {
    use super::{
        next_path_arg, next_usize_arg, parse_analyze_trace_args, parse_generate_args,
        parse_write_lobe_asset_args, GenerateOptions,
    };
    use std::path::PathBuf;

    #[test]
    fn next_usize_arg_rejects_non_integer_value() {
        let args = vec!["--classes".to_string(), "bad".to_string()];
        let mut index = 1usize;
        let err = next_usize_arg(&args, &mut index, "--classes").unwrap_err();
        assert!(err.contains("expects a positive integer"));
    }

    #[test]
    fn next_path_arg_rejects_missing_value() {
        let args: Vec<String> = Vec::new();
        let mut index = 0usize;
        let err = next_path_arg(&args, &mut index, "--out").unwrap_err();
        assert!(err.contains("requires a value"));
    }

    #[test]
    fn parse_generate_options() {
        let parsed = parse_generate_args(&[
            "--classes".to_string(),
            "8".to_string(),
            "--dimension".to_string(),
            "12".to_string(),
            "--source".to_string(),
            "smoke".to_string(),
            "--out".to_string(),
            "tmp/p31_etf.json".to_string(),
        ])
        .expect("parse generate args");
        assert_eq!(
            parsed,
            GenerateOptions {
                classes: Some(8),
                dimension: Some(12),
                output: Some(PathBuf::from("tmp/p31_etf.json")),
                source: Some("smoke".to_string()),
            }
        );
    }

    #[test]
    fn parse_analyze_trace_requires_trace_argument_to_be_provided() {
        let parsed = parse_analyze_trace_args(&[
            "--prototypes".to_string(),
            "table.json".to_string(),
            "--out".to_string(),
            "summary.json".to_string(),
        ])
        .expect("parse analyze args");
        assert!(parsed.trace.is_none());
        assert_eq!(parsed.prototypes, Some(PathBuf::from("table.json")));
        assert_eq!(parsed.output, Some(PathBuf::from("summary.json")));
    }

    #[test]
    fn parse_write_lobe_asset_collects_options() {
        let parsed = parse_write_lobe_asset_args(&[
            "--classes".to_string(),
            "9".to_string(),
            "--dimension".to_string(),
            "11".to_string(),
            "--out".to_string(),
            "target/lobe.json".to_string(),
        ])
        .expect("parse write args");
        assert_eq!(parsed.classes, Some(9));
        assert_eq!(parsed.dimension, Some(11));
        assert_eq!(parsed.output, Some(PathBuf::from("target/lobe.json")));
        assert_eq!(parsed.prototypes, None);
    }
}
