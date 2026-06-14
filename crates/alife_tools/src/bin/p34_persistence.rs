use std::{env, path::PathBuf, process::ExitCode};

use alife_world::persistence::{AssetManifest, PortableSaveFile, RuntimeConfig};

fn main() -> ExitCode {
    match run() {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [command, root] if command == "validate-fixtures" => validate_fixtures(PathBuf::from(root)),
        [command, save, root] if command == "validate-save" => {
            let save = PortableSaveFile::from_json_file(save).map_err(|err| err.to_string())?;
            save.validate_with_asset_root(root)
                .map_err(|err| err.to_string())?;
            Ok(format!("validated save {}", save.save_id))
        }
        [command, config] if command == "validate-config" => {
            let config = RuntimeConfig::from_json_file(config).map_err(|err| err.to_string())?;
            config.validate().map_err(|err| err.to_string())?;
            Ok(format!(
                "validated config seed {} brain {:?}",
                config.deterministic_seed, config.brain_class
            ))
        }
        [command, manifest, root] if command == "validate-manifest" => {
            let manifest = AssetManifest::from_json_file(manifest).map_err(|err| err.to_string())?;
            manifest
                .validate_with_root(root)
                .map_err(|err| err.to_string())?;
            Ok(format!("validated manifest with {} entries", manifest.entries.len()))
        }
        _ => Err(
            "usage: p34_persistence validate-fixtures <root> | validate-save <save> <asset-root> | validate-config <config> | validate-manifest <manifest> <asset-root>"
                .to_string(),
        ),
    }
}

fn validate_fixtures(root: PathBuf) -> Result<String, String> {
    let save = PortableSaveFile::from_json_file(root.join("tiny_save.json"))
        .map_err(|err| err.to_string())?;
    save.validate_with_asset_root(&root)
        .map_err(|err| err.to_string())?;
    let config = RuntimeConfig::from_json_file(root.join("tiny_config.json"))
        .map_err(|err| err.to_string())?;
    config.validate().map_err(|err| err.to_string())?;
    let manifest = AssetManifest::from_json_file(root.join("tiny_asset_manifest.json"))
        .map_err(|err| err.to_string())?;
    manifest
        .validate_with_root(&root)
        .map_err(|err| err.to_string())?;
    Ok(format!(
        "validated P34 fixtures at {}: save={}, config_seed={}, manifest_entries={}",
        root.display(),
        save.save_id,
        config.deterministic_seed,
        manifest.entries.len()
    ))
}
