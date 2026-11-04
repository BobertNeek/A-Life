//! Disposable native CPU pilot. Never imports an existing creature/checkpoint.
use alife_game_app::{
    reload_language_pilot_child, run_retained_language_pilot, LanguagePilotConfig,
};
use std::{collections::BTreeMap, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if let [flag, directory] = arguments.as_slice() {
        if flag == "--experimental-reload-child" {
            println!("{}", reload_language_pilot_child(Path::new(directory))?);
            return Ok(());
        }
    }
    let mut args = arguments.into_iter();
    let mut values = BTreeMap::new();
    let mut disposable = false;
    let mut slm = false;
    while let Some(key) = args.next() {
        match key.as_str() {
            "--help" | "-h" => {
                println!("experiment_retained_language --disposable-public-founder --output NEW_DIRECTORY [--seed 847501] [--epochs 16] [--rounds 2] [--decisions 24] [--llamacpp-translation]\n\nExperimental native CPU food/eat versus toy/get teaching with grounded, shuffled-word, and untreated arms. Reuses ordinary spatial speech and the existing bounded translator. Optional llama.cpp translation uses its existing localhost defaults and must return confirmed translations; it never falls back silently. No semantic prior is injected. Evaluates player requests and silent lives with the teacher absent, reversed positions and visible distractors. Writes local experimental recovery data and a source-safe results JSON; does not promote a creature or change live training.");
                return Ok(());
            }
            "--disposable-public-founder" if !disposable => disposable = true,
            "--llamacpp-translation" if !slm => slm = true,
            "--output" | "--seed" | "--epochs" | "--rounds" | "--decisions" => {
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
    let output = values.remove("--output").ok_or("--output is required")?;
    let config = LanguagePilotConfig {
        seed: values
            .remove("--seed")
            .unwrap_or_else(|| "847501".into())
            .parse()?,
        epochs: values
            .remove("--epochs")
            .unwrap_or_else(|| "16".into())
            .parse()?,
        rounds: values
            .remove("--rounds")
            .unwrap_or_else(|| "2".into())
            .parse()?,
        decisions: values
            .remove("--decisions")
            .unwrap_or_else(|| "24".into())
            .parse()?,
        llamacpp_translation: slm,
    };
    println!(
        "{}",
        run_retained_language_pilot(Path::new(&output), config)?
    );
    Ok(())
}
