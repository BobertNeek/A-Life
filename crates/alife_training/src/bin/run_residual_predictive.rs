//! Standalone empirical trial on local disposable simulation recordings.
use alife_training::cpu::predictive::{
    artifact::identity, recording::PublicSimulationRecording, residual::*,
};
use std::{
    collections::BTreeMap,
    error::Error,
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    serde_json::to_writer(&file, value)?;
    file.sync_all()?;
    Ok(())
}
fn write_zstd(path: &Path, value: &impl serde::Serialize) -> Result<(), Box<dyn Error>> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let mut encoder = zstd::stream::write::Encoder::new(file, 3)?;
    serde_json::to_writer(&mut encoder, value)?;
    encoder.finish()?.sync_all()?;
    Ok(())
}
fn main() -> Result<(), Box<dyn Error>> {
    let started = std::time::Instant::now();
    let mut config = ResidualConfig::default();
    let mut args = BTreeMap::new();
    let mut iter = std::env::args().skip(1);
    while let Some(key) = iter.next() {
        if key == "--help" || key == "-h" {
            println!("run_residual_predictive --output NEW_DIRECTORY [--seed 71] [--ticks 32] [--temperature 4] [--horizons 1,4] [--epochs 16] [--head-lr 0.03] [--native-lr 0.0001] [--scale-floor 0.001] [--recording CAPTURES.json.zst]\n\nThree prepared public scenes, one complete train/validation/test world per scene. Actual sampled native actions and world outcomes; no accepted brain/PC/GPU. Zero-output residual heads begin at persistence, with scales fitted on training deltas only. Matched full, within-split shuffled-action and recurrent ablations, fixed learning curves and heldout tests. Writes recoverable LOCAL experimental states, compressed recordings/datasets and numerical results; never publishes trained artifacts.");
            return Ok(());
        }
        if !matches!(
            key.as_str(),
            "--output"
                | "--seed"
                | "--ticks"
                | "--temperature"
                | "--horizons"
                | "--epochs"
                | "--head-lr"
                | "--native-lr"
                | "--scale-floor"
                | "--recording"
        ) {
            return Err(format!("unknown argument {key}").into());
        }
        let value = iter.next().ok_or("missing argument value")?;
        if args.insert(key, value).is_some() {
            return Err("duplicate argument".into());
        }
    }
    let output = PathBuf::from(args.remove("--output").ok_or("--output required")?);
    if output.exists() {
        return Err("output must be a new directory".into());
    }
    if let Some(v) = args.remove("--seed") {
        config.seed = v.parse()?;
    }
    if let Some(v) = args.remove("--ticks") {
        config.world_ticks = v.parse()?;
    }
    if let Some(v) = args.remove("--temperature") {
        config.temperature = v.parse()?;
    }
    if let Some(v) = args.remove("--epochs") {
        config.epochs = v.parse()?;
    }
    if let Some(v) = args.remove("--head-lr") {
        config.head_learning_rate = v.parse()?;
    }
    if let Some(v) = args.remove("--native-lr") {
        config.native_learning_rate = v.parse()?;
    }
    if let Some(v) = args.remove("--scale-floor") {
        config.scale_floor = v.parse()?;
    }
    if let Some(v) = args.remove("--horizons") {
        config.horizons = v
            .split(',')
            .map(str::parse)
            .collect::<Result<Vec<_>, _>>()?;
    }
    if !(1..=64).contains(&config.epochs)
        || config.horizons.is_empty()
        || config.horizons.iter().any(|&h| h == 0 || h > 32)
        || !config.head_learning_rate.is_finite()
        || config.head_learning_rate <= 0.
        || !config.native_learning_rate.is_finite()
        || config.native_learning_rate <= 0.
    {
        return Err("invalid experiment configuration".into());
    }
    let captures = if let Some(path) = args.remove("--recording") {
        let file = fs::File::open(path)?;
        let decoder = zstd::stream::read::Decoder::new(file)?;
        serde_json::from_reader::<_, Vec<PublicSimulationRecording>>(decoder)?
    } else {
        capture_diverse(&config)?
    };
    fs::create_dir(&output)?;
    write_json(&output.join("config.json"), &config)?;
    write_zstd(&output.join("recordings.json.zst"), &captures)?;
    write_json(&output.join("initial-handoff.json"), &captures[0].source)?;
    let diagnostics = action_diagnostics(&captures);
    println!(
        "{}",
        serde_json::json!({"stage":"capture_complete","actions":diagnostics,"source_identity_sha256":identity(&captures[0].source)?})
    );
    let mut horizons = Vec::new();
    for &horizon in &config.horizons {
        let data = horizon_dataset(&captures, horizon)?;
        write_zstd(&output.join(format!("dataset-h{horizon}.json.zst")), &data)?;
        let result = run_horizon(&config, &captures, &data, horizon, &output)?;
        write_json(&output.join(format!("result-h{horizon}.json")), &result)?;
        println!(
            "{}",
            serde_json::json!({"stage":"horizon_complete","horizon":horizon,"persistence_validation":result.persistence_validation,"persistence_test":result.persistence_test,"arms":result.arms.iter().map(|a|serde_json::json!({"inputs":a.inputs,"final_validation":a.curve.last().unwrap().validation,"test":a.test,"optimizer_steps":a.optimizer_steps,"training_seconds":a.training_seconds})).collect::<Vec<_>>()})
        );
        horizons.push(result);
    }
    let source_identity = identity(&captures[0].source)?;
    let recordings_identity = identity(&captures)?;
    let elapsed_seconds = started.elapsed().as_secs_f64();
    let result = serde_json::json!({"experiment":"public-residual-predictive-v1","config":config,"source_identity_sha256":source_identity,"recordings_identity_sha256":recordings_identity,"action_diagnostics":diagnostics,"horizons":horizons,"elapsed_seconds":elapsed_seconds,"elapsed_scope":"capture-or-load, conversion, training, evaluation and local experimental publication; excludes Cargo build","accepted_state_written":false,"trained_artifacts_published":false,"behavior_gain_measured":false,"throughput_gain_measured":false});
    write_json(&output.join("result.json"), &result)?;
    println!("result {}", output.join("result.json").display());
    Ok(())
}
