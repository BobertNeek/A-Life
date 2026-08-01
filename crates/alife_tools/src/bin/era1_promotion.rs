use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;

use alife_core::{Era1PlateauWindow, Era1TrialReceipt};
use alife_tools::era1_evolution::Era1EvolutionReceipt;
use alife_tools::era1_promotion::{derive_era1_promotion, Era1HardwareCost};

fn main() -> Result<(), Box<dyn Error>> {
    let evolution_path = required_path("--evolution")?;
    let trials_path = required_path("--trials")?;
    let hardware_path = required_path("--hardware")?;
    let plateau_path = required_path("--plateau")?;
    let output_path = required_path("--out")?;

    let evolution: Era1EvolutionReceipt = read_json(&evolution_path)?;
    let trials: Vec<Era1TrialReceipt> = read_json(&trials_path)?;
    let hardware: Era1HardwareCost = read_json(&hardware_path)?;
    let plateau: Vec<Era1PlateauWindow> = read_json(&plateau_path)?;
    let report = derive_era1_promotion(&evolution, &trials, hardware, &plateau)?;
    fs::write(output_path, serde_json::to_vec_pretty(&report)?)?;
    Ok(())
}

fn required_path(flag: &str) -> Result<PathBuf, Box<dyn Error>> {
    let mut arguments = env::args_os();
    let value = arguments
        .position(|argument| argument == flag)
        .and_then(|_| arguments.next())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("missing required {flag} path"),
            )
        })?;
    Ok(PathBuf::from(value))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, Box<dyn Error>> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
