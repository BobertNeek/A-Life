use std::{env, path::PathBuf, process::ExitCode};

use alife_tools::p35_playground::{
    run_gpu_authority_demo, run_headless_cpu_demo, run_save_load_demo, run_school_teacher_demo,
    run_semantic_fake_provider_demo, validate_playground_manifest, PlaygroundExampleConfig,
};

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
        [command, fixture_root] if command == "run-headless" => {
            let config = PlaygroundExampleConfig::from_p34_fixture_root(fixture_root)
                .map_err(|err| err.to_string())?;
            let report = run_headless_cpu_demo(config).map_err(|err| err.to_string())?;
            Ok(format!(
                "P35 headless playground seed={} backend={} sealed_patches={} packed_logs={} action={}",
                report.seed,
                report.backend_selected,
                report.sealed_patch_count,
                report.packed_log_count,
                report.action_debug
            ))
        }
        [command, fixture_root] if command == "save-load" => {
            let report = run_save_load_demo(fixture_root).map_err(|err| err.to_string())?;
            Ok(format!(
                "P35 save/load demo save={} seed={} world_entities={} stable_remap={} engine_ids={}",
                report.save_id,
                report.seed,
                report.world_entity_count,
                report.stable_id_remap_available,
                report.engine_local_ids_serialized
            ))
        }
        [command] if command == "school-demo" => {
            let report = run_school_teacher_demo().map_err(|err| err.to_string())?;
            Ok(format!(
                "P35 school demo perception_events={} verifier_passed={} direct_bypass={} hidden_vector={}",
                report.perception_event_count,
                report.verifier_passed,
                report.direct_motor_bypass,
                report.hidden_vector_injection
            ))
        }
        [command] if command == "semantic-demo" => {
            let report = run_semantic_fake_provider_demo().map_err(|err| err.to_string())?;
            Ok(format!(
                "P35 semantic demo missing_provider_tolerated={} fake_context={} provider_required={}",
                report.missing_provider_tolerated,
                report.fake_provider_context_available,
                report.provider_required_for_core_path
            ))
        }
        [command] if command == "gpu-authority" => {
            let report = run_gpu_authority_demo().map_err(|err| err.to_string())?;
            Ok(format!(
                "P35 GPU authority demo requested={} selected={} fail_closed={} active_bulk_readback={}",
                report.requested_backend,
                report.selected_backend,
                report.unavailable_is_fail_closed,
                report.active_bulk_readback_allowed
            ))
        }
        [command, manifest] if command == "validate-manifest" => {
            let report = validate_playground_manifest(manifest).map_err(|err| err.to_string())?;
            Ok(format!(
                "P35 manifest paths={} manual_optional_commands={} largest_sample_bytes={}",
                report.checked_paths,
                report.manual_optional_commands,
                report.largest_committed_sample_bytes
            ))
        }
        [command, fixture_root, manifest] if command == "run-all" => {
            let fixture_root = PathBuf::from(fixture_root);
            let config = PlaygroundExampleConfig::from_p34_fixture_root(&fixture_root)
                .map_err(|err| err.to_string())?;
            let headless = run_headless_cpu_demo(config).map_err(|err| err.to_string())?;
            let save = run_save_load_demo(&fixture_root).map_err(|err| err.to_string())?;
            let school = run_school_teacher_demo().map_err(|err| err.to_string())?;
            let semantic = run_semantic_fake_provider_demo().map_err(|err| err.to_string())?;
            let gpu = run_gpu_authority_demo().map_err(|err| err.to_string())?;
            let manifest = validate_playground_manifest(manifest).map_err(|err| err.to_string())?;
            Ok(format!(
                "P35 run-all seed={} save={} sealed_patches={} school={} semantic={} gpu_selected={} sample_paths={}",
                headless.seed,
                save.save_id,
                headless.sealed_patch_count,
                school.verifier_passed,
                semantic.fake_provider_context_available,
                gpu.selected_backend,
                manifest.checked_paths
            ))
        }
        _ => Err("usage: p35_playground run-headless <p34-fixture-root> | save-load <p34-fixture-root> | school-demo | semantic-demo | gpu-authority | validate-manifest <manifest> | run-all <p34-fixture-root> <manifest>".to_string()),
    }
}
