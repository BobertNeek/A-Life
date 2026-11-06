//! Explicit disposable public-founder experiment; never accepts a checkpoint.
use alife_training::connection_budget_experiment::DisposableBudgetFounder;
use std::{collections::BTreeMap, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let mut values = BTreeMap::new();
    let mut disposable = false;
    while let Some(key) = args.next() {
        match key.as_str() {
            "--help" | "-h" => {
                println!("experiment_connection_budget --disposable-public-founder --seed NONZERO --ticks 32\n\nCollects canonical CPU baseline world frames, then tests 0/16/32/48/64 average per-lobe connection budgets in detached replay. Prints a small JSON receipt. No native checkpoint is read, written, migrated or promoted. Infeasible budgets are reported and capped to the existing pool; no throughput or learning improvement is implied.");
                return Ok(());
            }
            "--disposable-public-founder" if !disposable => disposable = true,
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
    let receipt = DisposableBudgetFounder::new(seed)?.run(ticks)?;
    println!("{}", serde_json::to_string(&receipt)?);
    Ok(())
}
