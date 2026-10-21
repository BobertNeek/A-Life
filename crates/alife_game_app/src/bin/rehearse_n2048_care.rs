//! Offline care rehearsal command.
use alife_game_app::simulator_rehearsal::SimulatorRehearsalConfig;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Request {
    source: PathBuf,
    output: PathBuf,
    config: SimulatorRehearsalConfig,
    compare: bool,
}

fn parse_request(args: &[String]) -> Result<Request> {
    if args.len() < 4
        || args.len() > 5
        || (args.len() == 5
            && !matches!(
                args[4].as_str(),
                "--control-fresh" | "--compare" | "--qualify"
            ))
    {
        return Err("usage: rehearse_n2048_care SOURCE_CYCLE NEW_OUTPUT EXPECTED_SOURCE_POLICY SEED [--control-fresh|--compare|--qualify]".into());
    }
    let qualify = args.get(4).is_some_and(|flag| flag == "--qualify");
    let config = SimulatorRehearsalConfig {
        expected_source_policy: args[2].parse()?,
        seed: args[3].parse()?,
        attempts: if qualify { 4 } else { 16 },
        decisions_per_attempt: if qualify { 8 } else { 32 },
        burn_in: 2,
        collection_wall_limit_seconds: if qualify { 240 } else { 600 },
        qualification_wall_limit_seconds: qualify.then_some(360),
        control_fresh: args.get(4).is_some_and(|flag| flag == "--control-fresh"),
    };
    config.validate()?;
    Ok(Request {
        source: PathBuf::from(&args[0]),
        output: PathBuf::from(&args[1]),
        config,
        compare: args.get(4).is_some_and(|flag| flag == "--compare"),
    })
}

fn main() -> Result<()> {
    let Request {
        source,
        output,
        config,
        compare,
    } = parse_request(&std::env::args().skip(1).collect::<Vec<_>>())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Request> {
        parse_request(&args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>())
    }
    #[test]
    fn qualification_is_small_and_cannot_start_a_comparison() {
        let request = parse(&["source", "output", "7400", "700001", "--qualify"]).unwrap();
        assert_eq!(request.config.attempts, 4);
        assert_eq!(request.config.decisions_per_attempt, 8);
        assert_eq!(request.config.burn_in, 2);
        assert_eq!(request.config.collection_wall_limit_seconds, 240);
        assert_eq!(request.config.qualification_wall_limit_seconds, Some(360));
        assert_eq!(
            (request.config.attempts + 2) * request.config.decisions_per_attempt,
            48
        );
        assert!(!request.config.control_fresh);
        assert!(!request.compare);
        assert!(parse(&[
            "source",
            "output",
            "7400",
            "700001",
            "--qualify",
            "--compare"
        ])
        .is_err());
    }
    #[test]
    fn ordinary_modes_keep_the_existing_limits_and_explicit_selection() {
        let request = parse(&["source", "output", "7400", "700001"]).unwrap();
        assert_eq!(request.config.attempts, 16);
        assert_eq!(request.config.decisions_per_attempt, 32);
        assert_eq!(request.config.collection_wall_limit_seconds, 600);
        assert_eq!(request.config.qualification_wall_limit_seconds, None);
        assert!(!request.compare);
        let control = parse(&["source", "output", "7400", "700001", "--control-fresh"]).unwrap();
        assert!(control.config.control_fresh);
        assert!(!control.compare);
        assert!(
            parse(&["source", "output", "7400", "700001", "--compare"])
                .unwrap()
                .compare
        );
    }
    #[test]
    fn invalid_options_and_overflow_fail_at_admission() {
        for args in [
            &[][..],
            &["source", "output", "7400"][..],
            &["source", "output", "7400", "0"][..],
            &["source", "output", "7400", "700001", "--unknown"][..],
            &[
                "source",
                "output",
                "18446744073709551615",
                "700001",
                "--qualify",
            ][..],
        ] {
            assert!(parse(args).is_err());
        }
    }
}
