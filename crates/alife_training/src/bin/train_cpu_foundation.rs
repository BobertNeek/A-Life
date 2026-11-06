//! Explicit offline CPU fresh-cohort continuation. No graphics device is created.
use alife_training::{
    cpu::{
        run_cpu_continuation_with_replay_execution, CpuContinuationConfig as Args,
        CpuReplayExecution, DefaultCpuEpisode,
    },
    resolve_isolated_training_output,
};
use std::{
    collections::BTreeMap,
    error::Error,
    path::{Path, PathBuf},
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn args() -> Result<(Args, CpuReplayExecution)> {
    let mut values = BTreeMap::new();
    let mut fixture = false;
    let mut iter = std::env::args().skip(1);
    while let Some(key) = iter.next() {
        match key.as_str() {
            "--help" | "-h" => {
                println!("train_cpu_foundation (--previous NATIVE_TRAINING_DIRECTORY | --fixture) --output NEW_DIRECTORY --seed EXPLICIT_SEED [--ticks 64] [--temperature 1] [--replay-execution reference|cadence-csr]\n\nRuns offline CPU fresh-cohort foundation continuation with canonical world biology, sealed CPU evidence, bounded sparse BPTT/PPO and full residual critic Adam. Reference is default; cadence-csr explicitly selects CPU training replay scheduling only. Collection remains reference. This flag does not select or accelerate a GPU backend. Source files are read only. Seed freshness/allocation is caller-owned and recorded as unknown. Individual state is archived separately under its collection policy; this command does not claim a GPU terminal-individual restore.");
                std::process::exit(0);
            }
            "--fixture" => fixture = true,
            "--previous" | "--output" | "--seed" | "--ticks" | "--temperature"
            | "--replay-execution" => {
                if values
                    .insert(key.clone(), iter.next().ok_or("missing argument value")?)
                    .is_some()
                {
                    return Err(format!("duplicate argument {key}").into());
                }
            }
            _ => return Err(format!("unknown argument {key}").into()),
        }
    }
    let execution = match values.remove("--replay-execution").as_deref() {
        None | Some("reference") => CpuReplayExecution::Reference,
        Some("cadence-csr") => CpuReplayExecution::CadenceCsr,
        _ => return Err("--replay-execution must be reference or cadence-csr".into()),
    };
    let previous = values
        .remove("--previous")
        .map(|path| PathBuf::from(path).canonicalize())
        .transpose()?;
    let output = resolve_isolated_training_output(
        Path::new(&values.remove("--output").ok_or("--output is required")?),
        previous.iter().map(PathBuf::as_path),
    )?;
    let seed = values
        .remove("--seed")
        .ok_or("--seed is required; allocation is caller-owned")?
        .parse()?;
    let ticks = values
        .remove("--ticks")
        .unwrap_or_else(|| "64".into())
        .parse()?;
    let temperature = values
        .remove("--temperature")
        .unwrap_or_else(|| "1".into())
        .parse()?;
    if fixture == previous.is_some()
        || seed == 0
        || !(2..=2048).contains(&ticks)
        || !f32::is_finite(temperature)
        || temperature <= 0.
    {
        return Err("choose exactly one source mode, a nonzero seed, 2..2048 ticks and positive temperature".into());
    }
    if output.exists() {
        return Err("output must be a new experiment directory".into());
    }
    Ok((
        Args {
            previous,
            output,
            seed,
            ticks,
            temperature,
            fixture,
        },
        execution,
    ))
}
fn main() -> Result<()> {
    let (args, execution) = args()?;
    let receipt =
        run_cpu_continuation_with_replay_execution(args, &mut DefaultCpuEpisode, execution)?;
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
