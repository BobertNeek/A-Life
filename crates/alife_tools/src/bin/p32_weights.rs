//! P32 smoke CLI for optional generated initial-weight assets.

use std::env;
use std::path::PathBuf;

use alife_core::{BrainClassSpec, BrainScaleTier};
use alife_tools::p32_weights::{GeneratedInitialWeightAsset, GeneratedWeightTemplate};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_default();
    match command.as_str() {
        "generate-tiny" => {
            let output = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("target/artifacts/p32_tiny_initial_weights.json"));
            if let Some(parent) = output.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let spec = BrainClassSpec::for_tier(BrainScaleTier::Nano512);
            let asset = GeneratedInitialWeightAsset::procedural_fallback(
                &spec,
                GeneratedWeightTemplate::NeutralControl,
                32,
            )?;
            asset.to_json_file(&output)?;
            println!(
                "wrote {} with {} inherited synapses",
                output.display(),
                asset.w_genetic_fixed.entries.len()
            );
        }
        "validate" => {
            let path = args
                .next()
                .ok_or("usage: p32_weights validate <asset.json>")?;
            let asset = GeneratedInitialWeightAsset::from_json_file(path)?;
            println!(
                "valid {} v{} digest {}",
                asset.schema, asset.schema_version, asset.validation_digest
            );
        }
        _ => {
            eprintln!("usage: p32_weights generate-tiny [output.json] | validate <asset.json>");
            std::process::exit(2);
        }
    }
    Ok(())
}
