use std::{env, path::PathBuf, process::ExitCode};

use alife_tools::g16_content_authoring::{
    validate_content_pack, validate_creature_preset_file, validate_lesson_pack_file,
    validate_world_preset_file,
};
use alife_world::persistence::AssetManifest;

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
        [command, manifest] if command == "validate-pack" => {
            let report = validate_content_pack(PathBuf::from(manifest))
                .map_err(|err| err.to_string())?;
            Ok(format!(
                "G16 content pack {} worlds={} lessons={} creatures={} files={} largest_bytes={}",
                report.pack_id,
                report.world_presets,
                report.lesson_packs,
                report.creature_presets,
                report.checked_files,
                report.largest_file_bytes
            ))
        }
        [command, world] if command == "validate-world" => {
            let world = validate_world_preset_file(PathBuf::from(world))
                .map_err(|err| err.to_string())?;
            Ok(format!(
                "validated G16 world preset {} objects={}",
                world.world_id,
                world.objects.len()
            ))
        }
        [command, lesson] if command == "validate-lesson" => {
            let lesson = validate_lesson_pack_file(PathBuf::from(lesson))
                .map_err(|err| err.to_string())?;
            Ok(format!(
                "validated G16 lesson pack {} steps={}",
                lesson.lesson_pack_id,
                lesson.steps.len()
            ))
        }
        [command, creature, manifest] if command == "validate-creature" => {
            let asset_manifest =
                AssetManifest::from_json_file(manifest).map_err(|err| err.to_string())?;
            let creature = validate_creature_preset_file(PathBuf::from(creature), &asset_manifest)
                .map_err(|err| err.to_string())?;
            Ok(format!(
                "validated G16 creature preset {} brain={:?}",
                creature.preset_id, creature.brain_class
            ))
        }
        _ => Err(
            "usage: g16_content_authoring validate-pack <manifest> | validate-world <world-preset> | validate-lesson <lesson-pack> | validate-creature <creature-preset> <p34-asset-manifest>"
                .to_string(),
        ),
    }
}
