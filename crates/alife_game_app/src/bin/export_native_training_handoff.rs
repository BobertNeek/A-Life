//! Device-free legacy export and single-current checkpoint admission.
use alife_training::{CheckpointSeedsV1, CurrentTrainingCheckpointV1};
use std::{fs::OpenOptions, io::Write, path::Path};

fn write_new(
    path: &str,
    checkpoint: &CurrentTrainingCheckpointV1,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let bytes = checkpoint.encode()?;
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    // No replacement here: the run-scoped publisher owns atomic publication.
    alife_game_app::inspect_current_training_checkpoint(Path::new(path))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage =
        "export_native_training_handoff SOURCE NEW_OUTPUT\n  --inspect-current FILE\n  --inspect-native-frontier SOURCE --expected-policy POLICY\n  --import-current LEGACY_SOURCE SEEDS_JSON NEW_FILE\n  --reserve-current CURRENT FIRST LAST NEW_FILE\n  --consume-current CURRENT SEED NEW_FILE\n  --advance-current CURRENT EPISODE NEW_FILE\n  --extract-current-life CURRENT NEW_DIRECTORY\n  --begin-training-attempt TERMINAL_CURRENT ATTEMPT_NONCE NEW_FILE\nDevice-free admission; no simulation or GPU execution. New files must not exist.";
    if args.as_slice() == ["--help"] {
        println!("{usage}");
        return Ok(());
    }
    let report = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["--inspect-current", path] => Some(alife_game_app::inspect_current_training_checkpoint(
            Path::new(path),
        )?),
        ["--begin-training-attempt", path, nonce, output] => {
            alife_game_app::resolve_current_training_destination(
                Path::new(path),
                Path::new(output),
            )?;
            let source = std::fs::read(path)?;
            let checkpoint = alife_game_app::read_current_training_checkpoint(Path::new(path))?;
            let branch =
                checkpoint.begin_training_attempt_from_terminal(&source, nonce.parse()?)?;
            // The source is immutable, and a branch operation never reserves or
            // consumes a seed. The ordinary Native WAL owns real dispatch.
            if std::fs::read(path)? != source {
                return Err("terminal source changed during branch admission".into());
            }
            write_new(output, &branch)?;
            if std::fs::read(path)? != source {
                return Err("terminal source changed after create-new branch admission".into());
            }
            let mut report =
                alife_game_app::inspect_current_training_checkpoint(Path::new(output))?;
            report["kind"] = "explicit-preterminal-training-branch-created-v1".into();
            report["source_unchanged"] = true.into();
            report["seeds_modified"] = false.into();
            report["private_state_modified"] = false.into();
            Some(report)
        }
        ["--inspect-native-frontier", source, "--expected-policy", policy] => Some(
            alife_game_app::inspect_native_training_frontier(Path::new(source), policy.parse()?)?,
        ),
        ["--import-current", source, seeds, output] => {
            alife_game_app::resolve_current_training_destination(
                Path::new(source),
                Path::new(output),
            )?;
            let metadata: CheckpointSeedsV1 = serde_json::from_slice(&std::fs::read(seeds)?)?;
            let checkpoint =
                alife_game_app::migrate_current_training_checkpoint(Path::new(source), metadata)?;
            Some(write_new(output, &checkpoint)?)
        }
        ["--check-current-destination", source, output] => {
            let output = alife_game_app::resolve_current_training_destination(
                Path::new(source),
                Path::new(output),
            )?;
            Some(
                serde_json::json!({"output":output,"neural_execution":false,"weights_modified":false}),
            )
        }
        ["--reserve-current", path, first, last, output] => {
            let mut checkpoint = alife_game_app::read_current_training_checkpoint(Path::new(path))?;
            checkpoint.seeds.reserve(first.parse()?, last.parse()?)?;
            Some(write_new(output, &checkpoint)?)
        }
        ["--check-current-allocation", path, first, last] => {
            let mut checkpoint = alife_game_app::read_current_training_checkpoint(Path::new(path))?;
            checkpoint.seeds.reserve(first.parse()?, last.parse()?)?;
            Some(
                serde_json::json!({"valid":true,"neural_execution":false,"weights_modified":false}),
            )
        }
        ["--consume-current", path, seed, output] => {
            let mut checkpoint = alife_game_app::read_current_training_checkpoint(Path::new(path))?;
            checkpoint.seeds.consume(seed.parse()?)?;
            Some(write_new(output, &checkpoint)?)
        }
        ["--extract-current-life", path, output] => {
            alife_game_app::extract_current_training_life(Path::new(path), Path::new(output))?;
            Some(serde_json::json!({"extracted":true,"neural_execution":false}))
        }
        ["--advance-current", path, source, output] => {
            let previous = alife_game_app::read_current_training_checkpoint(Path::new(path))?;
            let checkpoint =
                alife_game_app::advance_current_training_checkpoint(&previous, Path::new(source))?;
            Some(write_new(output, &checkpoint)?)
        }
        [source, output] if !source.starts_with('-') => {
            let binding = alife_game_app::export_native_training_handoff(
                Path::new(source),
                Path::new(output),
            )?;
            println!("{}", serde_json::to_string_pretty(&binding)?);
            None
        }
        _ => return Err(usage.into()),
    };
    if let Some(report) = report {
        println!("{}", serde_json::to_string(&report)?);
    }
    Ok(())
}
