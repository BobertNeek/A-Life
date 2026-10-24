//! One explicit care round. No accepted-source search, concurrency or promotion.
use alife_game_app::{
    cpu_care_round_summary, prepare_cpu_care_round, run_cpu_care_round, CpuCareRoundPlanV1,
    CpuCareRoundSpecV1,
};
use alife_training::cpu::{materialize_cpu_cooperative_source, read_cpu_cooperative_round};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, Box<dyn std::error::Error>> {
    if fs::symlink_metadata(path)?.len() > 16 * 1024 * 1024 {
        return Err("care round JSON exceeds16 MiB".into());
    }
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Native checkpoint reconstruction and recurrent PPO need more stack than
    // the Windows executable's main thread. Bound it explicitly as other
    // offline care tools do; RUST_MIN_STACK does not resize the main thread.
    std::thread::Builder::new()
        .name("cpu-care-round".into())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| run().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "CPU care round worker panicked; retained output requires inspection")??;
    Ok(())
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "--help".into());
    if matches!(command.as_str(), "--help" | "-h") {
        println!("train_cpu_care_round plan --spec RECIPE.json --output SEALED_PLAN.json\ntrain_cpu_care_round run --plan SEALED_PLAN.json\ntrain_cpu_care_round inspect --result ROUND_DIR/learner\ntrain_cpu_care_round export-source --source EXACT_NATIVE_SOURCE --result ROUND_DIR/learner --output NEW_NATIVE_SOURCE\ntrain_cpu_care_round evaluate --spec ASSESSMENT.json\n\nV1 serially collects explicit care lessons under one frozen native CPU source and isolated deterministic_v1 priors, then invokes one central learner. Export carries full founder/Adam state into fresh lives only; private archives are never transplanted. Assessment uses declared held-out seeds with teachers/prior removed and no offline updates. Reuse the same binary and plan for collection retry. Incomplete private lives remain preserved and are never automatically restarted. No accepted brain is selected or promoted. Serial collection is a parity control; the separate comparison command executes serial learning too. These mechanisms alone do not establish synergy.");
        return Ok(());
    }
    let mut values = BTreeMap::new();
    while let Some(key) = args.next() {
        if !matches!(
            key.as_str(),
            "--spec" | "--output" | "--plan" | "--result" | "--source"
        ) {
            return Err(format!("unknown argument {key}").into());
        }
        let value = args.next().ok_or("argument value missing")?;
        if values.insert(key, PathBuf::from(value)).is_some() {
            return Err("duplicate argument".into());
        }
    }
    let summary = match command.as_str() {
        "plan"
            if values.len() == 2
                && values.contains_key("--spec")
                && values.contains_key("--output") =>
        {
            let spec: CpuCareRoundSpecV1 = read(&values["--spec"])?;
            let plan = prepare_cpu_care_round(spec)?;
            let mut preserved = plan
                .seed_evidence_sha256
                .keys()
                .map(PathBuf::as_path)
                .collect::<Vec<_>>();
            preserved.push(&plan.spec.previous);
            let output =
                alife_training::resolve_isolated_training_output(&values["--output"], preserved)?;
            if output.starts_with(&plan.spec.output) || plan.spec.output.starts_with(&output) {
                return Err(
                    "sealed plan file must live outside its new round output directory".into(),
                );
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&output)?;
            file.write_all(&serde_json::to_vec_pretty(&serde_json::to_value(&plan)?)?)?;
            file.sync_all()?;
            serde_json::json!({"plan":output,"plan_sha256":plan.payload_sha256,
                "output":plan.spec.output,"collectors":plan.assignments.len(),
                "prior":"deterministic_v1","prior_provider_identity":plan.prior_provider_identity})
        }
        "run" if values.len() == 1 && values.contains_key("--plan") => {
            let plan: CpuCareRoundPlanV1 = read(&values["--plan"])?;
            let result = run_cpu_care_round(&plan)?;
            cpu_care_round_summary(&result, &plan.spec.output.join("learner"))
        }
        "inspect" if values.len() == 1 && values.contains_key("--result") => {
            let result = read_cpu_cooperative_round(&values["--result"])?;
            cpu_care_round_summary(&result, &values["--result"])
        }
        "export-source"
            if values.len() == 3
                && values.contains_key("--source")
                && values.contains_key("--result")
                && values.contains_key("--output") =>
        {
            let receipt = materialize_cpu_cooperative_source(
                &values["--source"],
                &values["--result"],
                &values["--output"],
            )?;
            serde_json::json!({"output":values["--output"],"mode":receipt["mode"],
                "policy_version":receipt["policy_version"],"actor_optimizer_step":receipt["actor_optimizer_step"],
                "value_optimizer_step":receipt["value_optimizer_step"],"consumed_seed_ranges":receipt["consumed_seed_ranges"],
                "private_lifetime_continuation":"none; fresh lives only","promotion":"none"})
        }
        "evaluate" if values.len() == 1 && values.contains_key("--spec") => {
            let request: alife_game_app::CpuCareEvaluationRequest = read(&values["--spec"])?;
            alife_game_app::evaluate_cpu_care(request)?
        }
        _ => return Err("expected plan --spec --output, run --plan or inspect --result".into()),
    };
    println!("{}", serde_json::to_string(&summary)?);
    Ok(())
}
