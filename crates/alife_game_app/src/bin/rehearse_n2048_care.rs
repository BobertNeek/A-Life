//! Explicit offline tool, never linked into the ordinary game executable.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 4
        || args.len() > 5
        || (args.len() == 5 && !matches!(args[4].as_str(), "--control-fresh" | "--compare"))
    {
        return Err("usage: rehearse_n2048_care SOURCE_CYCLE NEW_OUTPUT EXPECTED_SOURCE_POLICY SEED [--control-fresh|--compare]".into());
    }
    let source = std::path::PathBuf::from(&args[0]);
    let output = std::path::PathBuf::from(&args[1]);
    let config = alife_game_app::simulator_rehearsal::SimulatorRehearsalConfig {
        expected_source_policy: args[2].parse()?,
        seed: args[3].parse()?,
        attempts: 16,
        decisions_per_attempt: 32,
        burn_in: 2,
        collection_wall_limit_seconds: 600,
        control_fresh: args.get(4).is_some_and(|flag| flag == "--control-fresh"),
    };
    let compare = args.get(4).is_some_and(|flag| flag == "--compare");
    let receipt = std::thread::Builder::new()
        .name("simulator-rehearsal".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            if compare {
                alife_game_app::simulator_rehearsal::compare_simulator_rehearsal(
                    &source, &output, config,
                )
                .map_err(|error| error.to_string())
            } else {
                alife_game_app::simulator_rehearsal::run_simulator_rehearsal(
                    &source, &output, config,
                )
                .and_then(|receipt| Ok(serde_json::to_value(receipt)?))
                .map_err(|error| error.to_string())
            }
        })?
        .join()
        .map_err(|_| "rehearsal worker panicked")??;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
