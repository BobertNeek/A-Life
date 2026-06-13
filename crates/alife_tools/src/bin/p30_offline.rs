use std::{fs, path::PathBuf, process};

use alife_core::PackedExperienceRecord;
use alife_tools::{
    p30_bundle::{read_packed_records_json_file, BundleConfig, PackedLogBundle},
    p30_markers::{BenchmarkMarker, ScenarioMarker},
    p30_summary::{ReplaySummary, SummaryConfig},
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
    match command.as_str() {
        "bundle" => run_bundle(args.collect()),
        "summary" => run_summary(args.collect()),
        "help" | "--help" | "-h" => {
            println!("{}", usage_text());
            Ok(())
        }
        _ => Err(format!("unknown command `{command}`\n{}", usage_text())),
    }
}

fn run_bundle(args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || args.first().is_some_and(|arg| arg == "help") {
        println!("{}", bundle_usage());
        return Ok(());
    }
    let mode = args
        .first()
        .ok_or_else(|| "bundle requires a mode".to_string())?;
    if mode != "import" {
        return Err(format!(
            "bundle supports `import` only; got `{mode}`\n{}",
            bundle_usage()
        ));
    }
    let options = parse_bundle_import_args(&args[1..])?;
    let records = read_bundle_records(&options.records)?;
    let scenario_markers =
        ScenarioMarker::read_many(&options.scenario_fixtures).map_err(|err| format!("{err}"))?;
    let benchmark_markers =
        BenchmarkMarker::read_many(&options.benchmark_markdown).map_err(|err| format!("{err}"))?;

    let bundle = PackedLogBundle::from_records(
        records,
        BundleConfig {
            source: options.source,
            notes: options.notes,
            scenario_markers,
            benchmark_markers,
        },
    );
    bundle
        .to_json_file(&options.output)
        .map_err(|err| format!("{err}"))?;
    println!("wrote bundle: {}", options.output.display());
    println!("records: {}", bundle.records.len());
    println!("scenario markers: {}", bundle.scenario_markers.len());
    println!("benchmark markers: {}", bundle.benchmark_markers.len());
    Ok(())
}

fn run_summary(args: Vec<String>) -> Result<(), String> {
    let options = parse_summary_args(&args)?;
    let mut config = SummaryConfig::default();
    if let Some(k) = options.cluster_k {
        config.cluster_k = Some(k);
    }
    if options.cluster_iterations > 0 {
        config.cluster_iterations = options.cluster_iterations;
    }

    let bundle = if let Some(bundle_path) = options.bundle {
        PackedLogBundle::from_json_file(bundle_path).map_err(|err| format!("{err}"))?
    } else {
        let records = read_bundle_records(&options.records)?;
        PackedLogBundle::from_records(records, BundleConfig::default())
    };

    let summary = ReplaySummary::from_bundle(&bundle, config).map_err(|err| err.to_string())?;
    let markdown = summary.to_markdown();
    println!("{markdown}");

    if let Some(path) = options.markdown {
        fs::write(path, markdown).map_err(|err| err.to_string())?;
    }
    if let Some(path) = options.trajectory_csv {
        summary
            .write_trajectory_csv(&path)
            .map_err(|err| format!("{err}"))?;
    }
    if let Some(path) = options.action_csv {
        summary
            .write_action_distribution_csv(&path)
            .map_err(|err| format!("{err}"))?;
    }
    if let Some(path) = options.summary_json {
        summary.write_json(&path).map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn read_bundle_records(paths: &[PathBuf]) -> Result<Vec<PackedExperienceRecord>, String> {
    let mut records = Vec::new();
    for path in paths {
        let packed_records = read_packed_records_json_file(path).map_err(|err| format!("{err}"))?;
        for record in packed_records {
            records.push(PackedExperienceRecord::try_from(record).map_err(|err| format!("{err}"))?);
        }
    }
    Ok(records)
}

#[derive(Debug, Default)]
struct BundleImportOptions {
    output: PathBuf,
    source: Option<String>,
    notes: Vec<String>,
    records: Vec<PathBuf>,
    scenario_fixtures: Vec<PathBuf>,
    benchmark_markdown: Vec<PathBuf>,
}

fn parse_bundle_import_args(args: &[String]) -> Result<BundleImportOptions, String> {
    let mut output = PathBuf::from("target")
        .join("artifacts")
        .join("p30_offline_bundle.json");
    let mut source = None;
    let mut notes = Vec::new();
    let mut records = Vec::new();
    let mut scenario_fixtures = Vec::new();
    let mut benchmark_markdown = Vec::new();

    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--output" | "--out" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--out requires a path".to_string())?;
                output = PathBuf::from(value);
            }
            "--source" => {
                index += 1;
                source = Some(
                    args.get(index)
                        .ok_or_else(|| "--source requires a value".to_string())?
                        .to_string(),
                );
            }
            "--note" => {
                index += 1;
                notes.push(
                    args.get(index)
                        .ok_or_else(|| "--note requires a value".to_string())?
                        .to_string(),
                );
            }
            "--record" => {
                index += 1;
                records.push(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--record requires a path".to_string())?,
                ));
            }
            "--scenario-fixture" | "--scenario" => {
                index += 1;
                scenario_fixtures
                    .push(PathBuf::from(args.get(index).ok_or_else(|| {
                        "--scenario-fixture requires a path".to_string()
                    })?));
            }
            "--benchmark-markdown" | "--benchmark" => {
                index += 1;
                benchmark_markdown
                    .push(PathBuf::from(args.get(index).ok_or_else(|| {
                        "--benchmark-markdown requires a path".to_string()
                    })?));
            }
            "--help" | "-h" => {
                println!("{}", bundle_usage());
                process::exit(0);
            }
            _ => {
                return Err(format!(
                    "unknown bundle argument `{}`\n{}",
                    args[index],
                    bundle_usage()
                ))
            }
        }
        index += 1;
    }

    if records.is_empty() && scenario_fixtures.is_empty() && benchmark_markdown.is_empty() {
        return Err("bundle import requires at least one source file".to_string());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    Ok(BundleImportOptions {
        output,
        source,
        notes,
        records,
        scenario_fixtures,
        benchmark_markdown,
    })
}

#[derive(Debug, Default)]
struct SummaryOptions {
    bundle: Option<PathBuf>,
    records: Vec<PathBuf>,
    markdown: Option<PathBuf>,
    trajectory_csv: Option<PathBuf>,
    action_csv: Option<PathBuf>,
    summary_json: Option<PathBuf>,
    cluster_k: Option<usize>,
    cluster_iterations: usize,
}

fn parse_summary_args(args: &[String]) -> Result<SummaryOptions, String> {
    let mut bundle = None;
    let mut records = Vec::new();
    let mut markdown = None;
    let mut trajectory_csv = None;
    let mut action_csv = None;
    let mut summary_json = None;
    let mut cluster_k = None;
    let mut cluster_iterations = 8usize;

    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--bundle" => {
                index += 1;
                bundle = Some(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--bundle requires a path".to_string())?,
                ));
            }
            "--record" => {
                index += 1;
                records.push(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--record requires a path".to_string())?,
                ));
            }
            "--markdown" => {
                index += 1;
                markdown = Some(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--markdown requires a path".to_string())?,
                ));
            }
            "--trajectory-csv" => {
                index += 1;
                trajectory_csv =
                    Some(PathBuf::from(args.get(index).ok_or_else(|| {
                        "--trajectory-csv requires a path".to_string()
                    })?));
            }
            "--action-csv" => {
                index += 1;
                action_csv = Some(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--action-csv requires a path".to_string())?,
                ));
            }
            "--json" => {
                index += 1;
                summary_json = Some(PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--json requires a path".to_string())?,
                ));
            }
            "--cluster-k" => {
                index += 1;
                let raw = args
                    .get(index)
                    .ok_or_else(|| "--cluster-k requires a positive integer".to_string())?;
                let value = raw
                    .parse::<usize>()
                    .map_err(|_| "--cluster-k requires a positive integer".to_string())?;
                if value == 0 {
                    cluster_k = None;
                } else {
                    cluster_k = Some(value);
                }
            }
            "--cluster-iterations" => {
                index += 1;
                cluster_iterations = args
                    .get(index)
                    .ok_or_else(|| "--cluster-iterations requires a positive integer".to_string())?
                    .parse::<usize>()
                    .map_err(|_| "--cluster-iterations requires a positive integer".to_string())?;
            }
            "--help" | "-h" => {
                println!("{}", summary_usage());
                process::exit(0);
            }
            _ => {
                return Err(format!(
                    "unknown summary argument `{}`\n{}",
                    args[index],
                    summary_usage()
                ))
            }
        }
        index += 1;
    }

    if bundle.is_none() && records.is_empty() {
        return Err("summary requires --bundle or --record".to_string());
    }
    if bundle.is_some() && !records.is_empty() {
        return Err("summary cannot mix --bundle and --record".to_string());
    }

    Ok(SummaryOptions {
        bundle,
        records,
        markdown,
        trajectory_csv,
        action_csv,
        summary_json,
        cluster_k,
        cluster_iterations: cluster_iterations.max(1),
    })
}

fn usage() -> String {
    format!(
        "{}\n{}\n{}",
        bundle_usage(),
        summary_usage(),
        "Run `p30_offline bundle --help` or `p30_offline summary --help` for per-command options."
    )
}

fn usage_text() -> &'static str {
    "\
Usage:
  p30_offline bundle import [options]
  p30_offline summary [options]"
}

fn bundle_usage() -> String {
    "\
Usage:
  p30_offline bundle import [--out <path>] [--source <string>] [--note <text>]
                           [--record <path>]...
                           [--scenario-fixture <path>]...
                           [--benchmark-markdown <path>]...

Defaults:
  --out target/artifacts/p30_offline_bundle.json

At least one source input is required."
        .to_string()
}

fn summary_usage() -> String {
    "\
Usage:
  p30_offline summary --bundle <path> [--markdown <path>]
                           [--trajectory-csv <path>] [--action-csv <path>]
                           [--json <path>] [--cluster-k <k>] [--cluster-iterations <n>]
  p30_offline summary --record <path> [--cluster-k <k>] [--cluster-iterations <n>]

Examples:
  p30_offline summary --bundle target/artifacts/p30_offline_bundle.json --markdown summary.md
  p30_offline summary --record packed_records.json --cluster-k 3 --cluster-iterations 10 --json summary.json"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{parse_bundle_import_args, parse_summary_args};
    use std::path::PathBuf;

    #[test]
    fn bundle_parser_collects_inputs_and_output_dir() {
        let args = vec![
            "import".to_string(),
            "--out".to_string(),
            "target/artifacts/manual.json".to_string(),
            "--record".to_string(),
            "run.json".to_string(),
            "--scenario-fixture".to_string(),
            "food-seeking.json".to_string(),
            "--benchmark-markdown".to_string(),
            "bench.md".to_string(),
            "--note".to_string(),
            "integration".to_string(),
            "--source".to_string(),
            "smoke".to_string(),
        ];
        let parsed = parse_bundle_import_args(&args[1..]).expect("bundle args parse");
        assert_eq!(parsed.output, PathBuf::from("target/artifacts/manual.json"));
        assert_eq!(parsed.records, vec![PathBuf::from("run.json")]);
        assert_eq!(
            parsed.scenario_fixtures,
            vec![PathBuf::from("food-seeking.json")]
        );
        assert_eq!(parsed.benchmark_markdown, vec![PathBuf::from("bench.md")]);
        assert_eq!(parsed.source, Some("smoke".to_string()));
        assert_eq!(parsed.notes, vec!["integration".to_string()]);
    }

    #[test]
    fn summary_parser_supports_bundle_and_cluster_args() {
        let args = vec![
            "--bundle".to_string(),
            "bundle.json".to_string(),
            "--cluster-k".to_string(),
            "4".to_string(),
            "--cluster-iterations".to_string(),
            "10".to_string(),
            "--markdown".to_string(),
            "summary.md".to_string(),
        ];
        let parsed = parse_summary_args(&args).expect("summary args parse");
        assert_eq!(parsed.bundle, Some(PathBuf::from("bundle.json")));
        assert_eq!(parsed.cluster_k, Some(4));
        assert_eq!(parsed.cluster_iterations, 10);
        assert_eq!(parsed.markdown, Some(PathBuf::from("summary.md")));
    }

    #[test]
    fn summary_parser_rejects_missing_input() {
        let args: Vec<String> = vec!["--cluster-k".to_string(), "2".to_string()];
        assert!(parse_summary_args(&args).is_err());
    }
}
