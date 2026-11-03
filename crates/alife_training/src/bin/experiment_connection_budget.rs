//! Explicit disposable public-founder experiment; never accepts a checkpoint.
use alife_training::connection_budget_experiment::DisposableBudgetFounder;
use std::{collections::BTreeMap, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut values = BTreeMap::new();
    let mut disposable = false;
    let mut compact = false;
    while let Some(key) = args.next() {
        match key.as_str() {
            "--help" | "-h" => {
                println!("experiment_connection_budget --disposable-public-founder --seed NONZERO --ticks 32 [--compact-eval]\n\nCollects canonical CPU baseline world frames, then tests connection budgets in detached replay. --compact-eval compares full-state bits and replay preparation/execution for 0/16/full masks. Prints JSON including founder-construction-through-replay elapsed time, excluding argument parsing and final output. Measure external process wall separately. No native checkpoint is read, written, migrated or promoted. Infeasible budgets are reported and capped to the existing pool; no learning improvement is implied.");
                return Ok(());
            }
            "--disposable-public-founder" if !disposable => disposable = true,
            "--compact-eval" if !compact => compact = true,
            "--seed" | "--ticks" => {
                let value = args.next().ok_or("missing argument value")?;
                if values.insert(key, value).is_some() {
                    return Err("duplicate argument".into());
                }
            }
            _ => return Err(format!("unknown or duplicate argument {key}").into()),
        }
    }
    if !disposable {
        return Err("--disposable-public-founder is required".into());
    }
    let seed = values
        .remove("--seed")
        .ok_or("explicit --seed is required")?
        .parse()?;
    let ticks = values
        .remove("--ticks")
        .unwrap_or_else(|| "32".into())
        .parse()?;
    if !(4..=256).contains(&ticks) {
        return Err("ticks must be 4..256".into());
    }
    let started = std::time::Instant::now();
    let founder = DisposableBudgetFounder::new(seed)?;
    let mut receipt = if compact {
        founder.run_compact(ticks)?
    } else {
        founder.run(ticks)?
    };
    receipt["cli_work_seconds"] = serde_json::json!(started.elapsed().as_secs_f64());
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
