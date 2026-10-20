//! Explicit group-50 native GPU continuation. No hardware or seed safety fallback.
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let usage="usage: train_n2048_group50 SOURCE OUTPUT --seed N --seed-history FILE [--decisions 256] [--seed-admission known-exclusions] [--replay-device-mib 512] [--replay-host-mib 8192] [--temperature 1]\n       train_n2048_group50 --validate-source SOURCE";
    let first = args.next().ok_or(usage)?;
    if first == "--help" {
        println!("{usage}");
        return Ok(());
    }
    if first == "--validate-source" {
        let source = PathBuf::from(args.next().ok_or(usage)?);
        if args.next().is_some() {
            return Err(usage.into());
        }
        println!(
            "{}",
            serde_json::to_string_pretty(
                &alife_game_app::validate_foundation_training_group_source(&source)?
            )?
        );
        return Ok(());
    }
    let previous = PathBuf::from(first);
    let output = PathBuf::from(args.next().ok_or(usage)?);
    let mut seed = None;
    let mut history = None;
    let mut decisions = 256;
    let mut known = false;
    let mut device = 512u64;
    let mut host = 8192u64;
    let mut temperature = 1.0;
    while let Some(flag) = args.next() {
        let value = args.next().ok_or("missing option value")?;
        match flag.as_str() {
            "--seed" => seed = Some(value.parse()?),
            "--seed-history" => history = Some(PathBuf::from(value)),
            "--decisions" => decisions = value.parse()?,
            "--seed-admission" if value == "known-exclusions" => known = true,
            "--replay-device-mib" => device = value.parse()?,
            "--replay-host-mib" => host = value.parse()?,
            "--temperature" => temperature = value.parse()?,
            _ => return Err(format!("unknown option {flag}: {usage}").into()),
        }
    }
    let receipt =
        alife_game_app::run_foundation_training_group(alife_game_app::FoundationGroupRequest {
            previous: &previous,
            output: &output,
            seed_history: &history.ok_or("--seed-history required")?,
            seed: seed.ok_or("--seed required")?,
            decisions_per_organism: decisions,
            known_exclusions: known,
            replay_device_limit_bytes: device
                .checked_mul(1024 * 1024)
                .ok_or("device budget overflow")?,
            replay_host_limit_bytes: host
                .checked_mul(1024 * 1024)
                .ok_or("host budget overflow")?,
            sampling_temperature: temperature,
        })?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
