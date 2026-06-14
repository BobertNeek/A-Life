use std::{fs, path::PathBuf, process};

use alife_tools::p33_evolution::tiny_generation_smoke;

fn main() {
    if let Err(message) = run(std::env::args().skip(1).collect()) {
        eprintln!("{message}");
        process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut args = args.into_iter();
    let command = args.next().ok_or_else(usage)?;
    match command.as_str() {
        "smoke" => run_smoke(args.collect()),
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            Ok(())
        }
        _ => Err(format!("unknown command `{command}`\n{}", usage())),
    }
}

fn run_smoke(args: Vec<String>) -> Result<(), String> {
    let options = parse_smoke_args(&args)?;
    let report = tiny_generation_smoke(options.seed, options.generations)
        .map_err(|err| format!("P33 smoke failed: {err}"))?;
    let json = serde_json::to_string_pretty(&report).map_err(|err| err.to_string())?;
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    fs::write(&options.output, json).map_err(|err| err.to_string())?;
    println!("wrote P33 smoke report: {}", options.output.display());
    println!("offspring: {}", report.offspring.len());
    Ok(())
}

#[derive(Debug, Clone)]
struct SmokeOptions {
    seed: u64,
    generations: u32,
    output: PathBuf,
}

fn parse_smoke_args(args: &[String]) -> Result<SmokeOptions, String> {
    let mut seed = 0xA11F_0033;
    let mut generations = 1_u32;
    let mut output = PathBuf::from("target")
        .join("artifacts")
        .join("p33_generation_smoke.json");

    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--seed" => {
                index += 1;
                seed = args
                    .get(index)
                    .ok_or_else(|| "--seed requires a value".to_string())?
                    .parse::<u64>()
                    .map_err(|_| "--seed requires an unsigned integer".to_string())?;
                if seed == 0 {
                    return Err("--seed must be nonzero".to_string());
                }
            }
            "--generations" => {
                index += 1;
                generations = args
                    .get(index)
                    .ok_or_else(|| "--generations requires a value".to_string())?
                    .parse::<u32>()
                    .map_err(|_| "--generations requires an unsigned integer".to_string())?
                    .max(1);
            }
            "--out" | "--output" => {
                index += 1;
                output = PathBuf::from(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path".to_string())?,
                );
            }
            "--help" | "-h" => {
                println!("{}", smoke_usage());
                process::exit(0);
            }
            other => {
                return Err(format!(
                    "unknown smoke argument `{other}`\n{}",
                    smoke_usage()
                ))
            }
        }
        index += 1;
    }

    Ok(SmokeOptions {
        seed,
        generations,
        output,
    })
}

fn usage() -> String {
    format!(
        "{}\n\n{}",
        "Usage:\n  p33_genome_lab smoke [options]",
        smoke_usage()
    )
}

fn smoke_usage() -> &'static str {
    "\
Usage:
  p33_genome_lab smoke [--seed <u64>] [--generations <n>] [--out <path>]

Defaults:
  --seed 270467123
  --generations 1
  --out target/artifacts/p33_generation_smoke.json"
}
