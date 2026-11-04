//! Current and native-CPU fresh-life assessments under one executable identity.
use std::{fs, path::Path};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const USAGE: &str = "evaluate_cpu_current --spec CURRENT_REQUEST | --full-spec CURRENT_REQUEST | --founder-spec NATIVE_CPU_REQUEST | --full-founder-spec NATIVE_CPU_REQUEST | --verify OUTPUT | --compare LEFT RIGHT | --parity BARE FULL | --founder-parity BARE FULL";

fn execute(args: &[String]) -> Result<serde_json::Value> {
    match args {
        [flag, path] if flag == "--spec" => {
            alife_game_app::evaluate_cpu_current(serde_json::from_slice(&fs::read(path)?)?)
        }
        [flag, path] if flag == "--full-spec" => {
            alife_game_app::evaluate_cpu_current_full(serde_json::from_slice(&fs::read(path)?)?)
        }
        [flag, path] if flag == "--founder-spec" => {
            alife_game_app::evaluate_cpu_founder(serde_json::from_slice(&fs::read(path)?)?)
        }
        [flag, path] if flag == "--full-founder-spec" => {
            alife_game_app::evaluate_cpu_full_founder(serde_json::from_slice(&fs::read(path)?)?)
        }
        [flag, path] if flag == "--verify" => {
            alife_game_app::verify_cpu_fresh_assessment(Path::new(path))
        }
        [flag, left, right] if flag == "--compare" => {
            alife_game_app::compare_cpu_fresh_assessments(Path::new(left), Path::new(right))
        }
        [flag, left, right] if flag == "--parity" => {
            alife_game_app::verify_cpu_current_parity(Path::new(left), Path::new(right))
        }
        [flag, left, right] if flag == "--founder-parity" => {
            alife_game_app::verify_cpu_native_founder_parity(Path::new(left), Path::new(right))
        }
        _ => Err(USAGE.into()),
    }
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.as_slice() == ["--help"] {
        println!("{USAGE}");
        return Ok(());
    }
    let result = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || execute(&args).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Current evaluator panicked; preserve partial output")??;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
