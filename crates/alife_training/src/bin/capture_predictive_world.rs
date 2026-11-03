//! Public simulation capture/conversion only; no optimizer update is performed.
use alife_training::cpu::predictive::{artifact::identity, recording::*};
use std::{
    collections::BTreeMap,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = BTreeMap::new();
    let mut iter = std::env::args().skip(1);
    while let Some(key) = iter.next() {
        if key == "--help" || key == "-h" {
            println!("capture_predictive_world --output NEW_DIRECTORY [--seed 71] [--ticks 12] [--recording EXISTING_PUBLIC_RECORDING.json]\n\nCapture three fresh public canonical worlds (train/validation/test), sampled native actions, actual outcomes and adjacent sensor/body observations with a procedural untrained founder; or convert an existing capture. No accepted source, PC or GPU is used. Writes raw recordings, initial untrained native handoff, dataset and conversion metrics into a new local directory only. Never publishes trained artifacts.");
            return Ok(());
        }
        if !matches!(
            key.as_str(),
            "--output" | "--seed" | "--ticks" | "--recording"
        ) {
            return Err(format!("unknown argument {key}").into());
        }
        let value = iter.next().ok_or("missing argument value")?;
        if args.insert(key, value).is_some() {
            return Err("duplicate argument".into());
        }
    }
    let output = PathBuf::from(args.remove("--output").ok_or("--output is required")?);
    if output.exists() {
        return Err("capture output directory must be new".into());
    }
    let seed: u64 = args
        .remove("--seed")
        .unwrap_or_else(|| "71".into())
        .parse()?;
    let ticks = args
        .remove("--ticks")
        .unwrap_or_else(|| "12".into())
        .parse()?;
    let recording = if let Some(path) = args.remove("--recording") {
        if fs::metadata(&path)?.len() > 128 * 1024 * 1024 {
            return Err("recording exceeds 128 MiB".into());
        }
        serde_json::from_slice::<PublicSimulationRecording>(&fs::read(path)?)?
    } else {
        capture_public_simulation(PublicCaptureConfig {
            founder_seed: seed,
            world_seeds: [
                seed.checked_add(1).ok_or("seed overflow")?,
                seed.checked_add(2).ok_or("seed overflow")?,
                seed.checked_add(3).ok_or("seed overflow")?,
            ],
            sampling_seeds: [101, 202, 303],
            world_ticks: ticks,
        })?
    };
    let (dataset, metrics) = convert_recording(&recording)?;
    fs::create_dir(&output)?;
    for (name, bytes) in [
        ("recording.json", serde_json::to_vec(&recording)?),
        ("dataset.json", serde_json::to_vec(&dataset)?),
        (
            "initial-handoff.json",
            serde_json::to_vec(&recording.source)?,
        ),
        ("conversion.json", serde_json::to_vec(&metrics)?),
    ] {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(name))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    println!(
        "{}",
        serde_json::json!({"output":output,"config":recording.config,"recording_identity_sha256":identity(&recording)?,"source_identity_sha256":identity(&recording.source)?,"dataset_identity_sha256":identity(&dataset)?,"conversion":metrics,"offline_optimizer_updates":0,"accepted_checkpoint_written":false})
    );
    Ok(())
}
