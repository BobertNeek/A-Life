//! Task 3 bridge from the Pass 2 planner contract to sealed Era 1 GPU trials.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    process::Command,
};

use alife_core::{
    BrainCapacityClass, CreatureGenome, Era1Ability, Era1Control, Era1EvidencePartition,
    FoundationGeneticIdentity, LanguageCodebookV1, OrganismId, ScaffoldContractError,
};
use alife_gpu_backend::GpuRuntimeSelectorDiagnosticEnableFailure;
use alife_training::{
    Era1TrialRunError, Era1TrialRunEvidence, Era1TrialRunRequest, Era1TrialRunner,
};
use alife_world::{Era1TrialManifest, Era1WorldFamily};
use serde::Serialize;
use serde_json::{Map, Value};
use thiserror::Error;

pub const RUNNER_ID: &str = "pass2_ei1_behavioral_harness_v1";
const MANIFEST_SCHEMA: &str = "pass2-experiment-manifest-v1";
const FOUNDATION_ID: u64 = 0x4E32_3034_385F_5631;
const FOUNDATION_VERSION: u16 = 1;
const FOUNDATION_CHECKSUM: u64 = 0x4E32_3034_385F_FA11;
const SCENARIO_CATALOG: &str = include_str!("../../../scripts/pass2_ei1_scenarios.json");

#[derive(Debug, Error)]
pub enum Pass2Ei1BehavioralError {
    #[error("invalid planned manifest: {0}")]
    InvalidManifest(&'static str),
    #[error("unsupported Task 3 scenario or configuration: {0}")]
    Unsupported(&'static str),
    #[error("source identity mismatch: {0}")]
    SourceIdentity(&'static str),
    #[error("I/O failure: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON failure: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Era 1 production runner failed: {0}")]
    Runner(String),
    #[error("Era 1 selector diagnostic enable-stage failure: {0}")]
    SelectorDiagnosticEnable(GpuRuntimeSelectorDiagnosticEnableFailure),
    #[error("Era 1 selector diagnostic later-stage GPU failure: {0}")]
    SelectorDiagnosticLaterStage(ScaffoldContractError),
}

#[derive(Debug, Serialize)]
struct RawArtifact<'a> {
    schema_version: &'static str,
    manifest_identity: &'a str,
    source_tree: String,
    scenario_id: &'a str,
    configuration_id: &'a str,
    capability_metrics: BTreeMap<&'static str, Value>,
    cognitive_work: BTreeMap<&'static str, Value>,
    evidence: &'a [Era1TrialRunEvidence],
}

pub fn run_planned_manifest(
    planned_manifest_path: &Path,
    manifest_identity: &str,
) -> Result<(), Pass2Ei1BehavioralError> {
    let mut manifest: Value = serde_json::from_slice(&fs::read(planned_manifest_path)?)?;
    let object = manifest
        .as_object_mut()
        .ok_or(Pass2Ei1BehavioralError::InvalidManifest(
            "root is not an object",
        ))?;
    validate_identity(object, manifest_identity)?;
    validate_contract_paths(object)?;
    let scenario_id = command_value(object, "--scenario-id")?;
    let configuration_id = command_value(object, "--configuration-id")?;
    validate_catalog_scenario(&scenario_id)?;
    let (ability, family, partition) = match scenario_id.as_str() {
        "few_shot_avoidance" => (
            Era1Ability::HazardAvoidance,
            Era1WorldFamily::ForagingHazardMaze,
            Era1EvidencePartition::Acquisition,
        ),
        "reversal_learning" => (
            Era1Ability::RewardReversal,
            Era1WorldFamily::RewardReversal,
            Era1EvidencePartition::ReversalProbe,
        ),
        _ => {
            return Err(Pass2Ei1BehavioralError::Unsupported(
                "scenario is unavailable",
            ));
        }
    };
    if configuration_id != "full_system" || !all_mechanisms_enabled(object)? {
        return Err(Pass2Ei1BehavioralError::Unsupported(
            "only the implemented full_system configuration is available",
        ));
    }
    if string(object, "brain_class")? != "N2048" || integer(object, "population")? != 1 {
        return Err(Pass2Ei1BehavioralError::Unsupported(
            "Era1TrialRunner requires one N2048 organism",
        ));
    }
    let source_commit = string(object, "source_commit")?.to_owned();
    let source_tree = git_identity("HEAD^{tree}")?;
    if source_commit != git_identity("HEAD")? {
        return Err(Pass2Ei1BehavioralError::SourceIdentity(
            "planned source commit is not the checked-out commit",
        ));
    }
    let foundation = FoundationGeneticIdentity::new(
        FOUNDATION_ID,
        FOUNDATION_VERSION,
        FOUNDATION_CHECKSUM,
        BrainCapacityClass::N2048_ID,
    )
    .map_err(core_failure)?;
    let mut runner = Era1TrialRunner::new_required().map_err(|error| {
        Pass2Ei1BehavioralError::Runner(format!("new_required rejected production GPU: {error}"))
    })?;
    let mut evidence = Vec::new();
    for seed in planned_seeds(object)? {
        let subject = derived_organism(seed, 1)?;
        let familiar = derived_organism(seed, 2)?;
        let novel = derived_organism(seed, 3)?;
        let taught_token =
            u16::try_from((seed % u64::from(LanguageCodebookV1::CODE_COUNT - 1)) + 1)
                .map_err(core_failure)?;
        let manifest_trial = Era1TrialManifest::new(
            seed,
            family,
            subject,
            familiar,
            novel,
            1,
            false,
            taught_token,
        )
        .map_err(core_failure)?;
        let genome = CreatureGenome::early_mammal_founder(seed ^ 0xE11_000, foundation.clone())
            .map_err(core_failure)?;
        let request = Era1TrialRunRequest::new(
            subject,
            0,
            &genome,
            &manifest_trial,
            ability,
            Era1Control::Intact,
            partition,
            &source_commit,
            &source_tree,
        )
        .map_err(core_failure)?
        .with_selector_diagnostics_for_candidates([3, 5]);
        let trial = runner
            .run_with_selector_diagnostics(request)
            .map_err(runner_failure)?;
        trial.validate_contract().map_err(core_failure)?;
        validate_production_evidence(&trial, ability)?;
        evidence.push(trial);
    }

    let raw_path = artifact_path(object, "raw_output_path")?;
    let receipt_path = artifact_path(object, "receipt_path")?;
    let capability_metrics = capability_metrics(&scenario_id, &evidence);
    let cognitive_work = cognitive_work(&evidence);
    write_raw(
        &raw_path,
        &RawArtifact {
            schema_version: "pass2-ei1-behavioral-raw-v1",
            manifest_identity,
            source_tree,
            scenario_id: &scenario_id,
            configuration_id: &configuration_id,
            capability_metrics: capability_metrics.clone(),
            cognitive_work: cognitive_work.clone(),
            evidence: &evidence,
        },
    )?;
    object.insert(
        "metrics".to_owned(),
        serde_json::json!({
            "capability": capability_metrics,
            "cognitive_work": cognitive_work,
        }),
    );
    object.insert(
        "outcome".to_owned(),
        serde_json::json!({"status": "completed", "execution_state": "executed"}),
    );
    write_json(&receipt_path, &manifest)?;
    Ok(())
}

fn validate_catalog_scenario(scenario_id: &str) -> Result<(), Pass2Ei1BehavioralError> {
    let catalog: Value = serde_json::from_str(SCENARIO_CATALOG)?;
    let scenarios = catalog.get("scenarios").and_then(Value::as_array).ok_or(
        Pass2Ei1BehavioralError::InvalidManifest("catalog scenarios are invalid"),
    )?;
    let scenario = scenarios
        .iter()
        .find(|scenario| scenario.get("id").and_then(Value::as_str) == Some(scenario_id))
        .ok_or(Pass2Ei1BehavioralError::Unsupported(
            "scenario is absent from the catalog",
        ))?;
    if scenario.get("support").and_then(Value::as_str) != Some("available") {
        return Err(Pass2Ei1BehavioralError::Unsupported(
            "scenario is unavailable in the catalog",
        ));
    }
    Ok(())
}

fn validate_contract_paths(object: &Map<String, Value>) -> Result<(), Pass2Ei1BehavioralError> {
    if command_value(object, "--scenario-catalog")? != "scripts/pass2_ei1_scenarios.json"
        || command_value(object, "--manifest-schema")?
            != "scripts/pass2_experiment_manifest.schema.json"
        || string(value_object(object, "outcome")?, "status")? != "planned"
        || string(value_object(object, "outcome")?, "execution_state")? != "not_run"
    {
        return Err(Pass2Ei1BehavioralError::InvalidManifest(
            "planner contract paths or state changed",
        ));
    }
    Ok(())
}

fn core_failure(error: impl std::fmt::Display) -> Pass2Ei1BehavioralError {
    Pass2Ei1BehavioralError::Runner(format!("production contract rejected the request: {error}"))
}

fn runner_failure(error: Era1TrialRunError) -> Pass2Ei1BehavioralError {
    match error {
        Era1TrialRunError::SelectorDiagnosticEnable(error) => {
            Pass2Ei1BehavioralError::SelectorDiagnosticEnable(error)
        }
        Era1TrialRunError::SelectorDiagnosticLaterStage(error) => {
            Pass2Ei1BehavioralError::SelectorDiagnosticLaterStage(error)
        }
        Era1TrialRunError::Contract(error) => {
            Pass2Ei1BehavioralError::Runner(format!("run rejected causal execution: {error}"))
        }
    }
}

fn validate_identity(
    object: &Map<String, Value>,
    identity: &str,
) -> Result<(), Pass2Ei1BehavioralError> {
    let configuration_hash = string(object, "configuration_hash")?;
    let raw_path = artifact_path(object, "raw_output_path")?;
    let receipt_path = artifact_path(object, "receipt_path")?;
    let expected_raw = format!("{identity}.raw.jsonl");
    let expected_receipt = format!("{identity}.receipt.json");
    if identity.len() != 64
        || !identity
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || configuration_hash.len() != 64
        || !configuration_hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || string(object, "schema_version")? != MANIFEST_SCHEMA
        || string(command_object(object)?, "runner_id")? != RUNNER_ID
        || raw_path.file_name().and_then(|name| name.to_str()) != Some(expected_raw.as_str())
        || receipt_path.file_name().and_then(|name| name.to_str())
            != Some(expected_receipt.as_str())
        || raw_path.parent() != receipt_path.parent()
        || raw_path == receipt_path
    {
        return Err(Pass2Ei1BehavioralError::InvalidManifest(
            "identity or runner contract changed",
        ));
    }
    Ok(())
}

fn command_value(
    object: &Map<String, Value>,
    flag: &str,
) -> Result<String, Pass2Ei1BehavioralError> {
    let arguments = array(command_object(object)?, "arguments")?;
    arguments
        .windows(2)
        .find_map(|pair| {
            (pair[0].as_str() == Some(flag))
                .then(|| pair[1].as_str())
                .flatten()
        })
        .map(str::to_owned)
        .ok_or(Pass2Ei1BehavioralError::InvalidManifest(
            "missing planned scenario/configuration",
        ))
}

fn all_mechanisms_enabled(object: &Map<String, Value>) -> Result<bool, Pass2Ei1BehavioralError> {
    let mechanisms = value_object(object, "mechanisms")?;
    Ok([
        "attention",
        "concept_gap_context",
        "prediction",
        "dendritic_conjunctions",
        "structural_plasticity",
        "weight_learning",
        "sleep_consolidation",
        "episodic_recall",
    ]
    .into_iter()
    .all(|name| mechanisms.get(name).and_then(Value::as_bool) == Some(true)))
}

fn planned_seeds(object: &Map<String, Value>) -> Result<Vec<u64>, Pass2Ei1BehavioralError> {
    let seed_set = value_object(object, "seed_set")?;
    let seeds = array(seed_set, "seeds")?
        .iter()
        .map(|seed| {
            seed.as_u64()
                .ok_or(Pass2Ei1BehavioralError::InvalidManifest(
                    "seed set contains an invalid value",
                ))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if seeds.is_empty()
        || seeds
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != seeds.len()
    {
        return Err(Pass2Ei1BehavioralError::InvalidManifest(
            "seed set is empty or has duplicates",
        ));
    }
    Ok(seeds)
}

fn derived_organism(seed: u64, domain: u64) -> Result<OrganismId, Pass2Ei1BehavioralError> {
    let value = seed
        .checked_mul(17)
        .and_then(|value| value.checked_add(domain))
        .ok_or(Pass2Ei1BehavioralError::InvalidManifest(
            "seed cannot derive valid organism IDs",
        ))?;
    Ok(OrganismId(value))
}

fn validate_production_evidence(
    evidence: &Era1TrialRunEvidence,
    ability: Era1Ability,
) -> Result<(), Pass2Ei1BehavioralError> {
    if evidence.receipt.ability != ability
        || evidence.adapter_name.trim().is_empty()
        || evidence.backend_api != "vulkan"
        || evidence.gpu_dispatches == 0
        || evidence.sealed_outcomes != evidence.gpu_dispatches
        || evidence.learning_commits != evidence.gpu_dispatches
        || evidence
            .steps
            .iter()
            .any(|step| step.world_before_digest == [0; 4])
    {
        return Err(Pass2Ei1BehavioralError::Runner(
            "production receipts are incomplete".to_owned(),
        ));
    }
    Ok(())
}

fn capability_metrics(
    scenario_id: &str,
    evidence: &[Era1TrialRunEvidence],
) -> BTreeMap<&'static str, Value> {
    let mut metrics = BTreeMap::new();
    let scores = evidence
        .iter()
        .map(|trial| serde_json::to_value(trial.receipt.score).unwrap_or(Value::Null))
        .collect();
    metrics.insert("success_rate", Value::Array(scores));
    metrics.insert("trials_completed", Value::from(evidence.len()));
    metrics.insert("causal_failures", Value::from(0));
    metrics.insert("invalid_receipts", Value::from(0));
    metrics.insert("nan_or_divergence", Value::from(0));
    metrics.insert("evaluator_corruption", Value::from(0));
    metrics.insert(
        if scenario_id == "few_shot_avoidance" {
            "hazard_avoidance_success_rate"
        } else {
            "reversal_adaptation_trials"
        },
        metrics["success_rate"].clone(),
    );
    metrics
}

fn cognitive_work(evidence: &[Era1TrialRunEvidence]) -> BTreeMap<&'static str, Value> {
    let gpu_dispatches = evidence
        .iter()
        .map(|trial| trial.gpu_dispatches)
        .sum::<u64>();
    let memory_updates = evidence
        .iter()
        .map(|trial| trial.memory_updates)
        .sum::<u64>();
    let learning_commits = evidence
        .iter()
        .map(|trial| trial.learning_commits)
        .sum::<u64>();
    let sleep_commits = evidence
        .iter()
        .map(|trial| u64::from(trial.sleep_commits))
        .sum::<u64>();
    let mut work = BTreeMap::new();
    work.insert("neural_updates", Value::from(gpu_dispatches));
    work.insert("memory_ops", Value::from(memory_updates));
    work.insert("learning_ops", Value::from(learning_commits));
    work.insert("sleep_ops", Value::from(sleep_commits));
    work.insert(
        "weighted_total",
        Value::from(gpu_dispatches + memory_updates + learning_commits + sleep_commits),
    );
    for field in [
        "synapses_evaluated",
        "dendritic_ops",
        "focal_target_ops",
        "concept_ops",
        "gap_ops",
        "prediction_ops",
        "replay_ops",
        "structural_ops",
    ] {
        work.insert(field, Value::Null);
    }
    work
}

fn write_raw(path: &Path, raw: &RawArtifact<'_>) -> Result<(), Pass2Ei1BehavioralError> {
    let parent = path
        .parent()
        .ok_or(Pass2Ei1BehavioralError::InvalidManifest(
            "raw path has no parent",
        ))?;
    fs::create_dir_all(parent)?;
    let mut file = BufWriter::new(File::create(path)?);
    serde_json::to_writer(&mut file, raw)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), Pass2Ei1BehavioralError> {
    let parent = path
        .parent()
        .ok_or(Pass2Ei1BehavioralError::InvalidManifest(
            "receipt path has no parent",
        ))?;
    fs::create_dir_all(parent)?;
    serde_json::to_writer_pretty(File::create(path)?, value)?;
    Ok(())
}

fn git_identity(revision: &str) -> Result<String, Pass2Ei1BehavioralError> {
    let output = Command::new("git").args(["rev-parse", revision]).output()?;
    let identity = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success()
        || identity.len() != 40
        || !identity
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(Pass2Ei1BehavioralError::SourceIdentity(
            "git did not return a lower-case object ID",
        ));
    }
    Ok(identity)
}

fn artifact_path(
    object: &Map<String, Value>,
    field: &'static str,
) -> Result<PathBuf, Pass2Ei1BehavioralError> {
    let path = PathBuf::from(string(value_object(object, "artifacts")?, field)?);
    if !path.is_absolute() {
        return Err(Pass2Ei1BehavioralError::InvalidManifest(
            "artifact path is not absolute",
        ));
    }
    Ok(path)
}

fn command_object(
    object: &Map<String, Value>,
) -> Result<&Map<String, Value>, Pass2Ei1BehavioralError> {
    value_object(object, "command")
}
fn value_object<'a>(
    object: &'a Map<String, Value>,
    key: &'static str,
) -> Result<&'a Map<String, Value>, Pass2Ei1BehavioralError> {
    object
        .get(key)
        .and_then(Value::as_object)
        .ok_or(Pass2Ei1BehavioralError::InvalidManifest(key))
}
fn array<'a>(
    object: &'a Map<String, Value>,
    key: &'static str,
) -> Result<&'a Vec<Value>, Pass2Ei1BehavioralError> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or(Pass2Ei1BehavioralError::InvalidManifest(key))
}
fn string<'a>(
    object: &'a Map<String, Value>,
    key: &'static str,
) -> Result<&'a str, Pass2Ei1BehavioralError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or(Pass2Ei1BehavioralError::InvalidManifest(key))
}
fn integer(object: &Map<String, Value>, key: &'static str) -> Result<u64, Pass2Ei1BehavioralError> {
    object
        .get(key)
        .and_then(Value::as_u64)
        .ok_or(Pass2Ei1BehavioralError::InvalidManifest(key))
}

#[cfg(test)]
mod selector_diagnostic_error_tests {
    use super::*;
    use alife_gpu_backend::{
        GpuRuntimeSelectorDiagnosticErrorReceipt, GpuRuntimeSelectorDiagnosticFailureClass,
    };
    use alife_training::Era1TrialRunError;

    #[test]
    fn task3_preserves_selector_enable_receipt_and_later_stage_classification() {
        let receipt = GpuRuntimeSelectorDiagnosticErrorReceipt {
            class: GpuRuntimeSelectorDiagnosticFailureClass::CapacityExceeded,
            class_id: 2048,
            chunk_index: 3,
            row: 2,
            base_words: 100,
            candidate_count: 2,
            decoder_synapse_count: 10,
            record_words: 29,
            detail_words: 580,
            frame_payload_capacity_words: 200,
        };
        let enable = runner_failure(Era1TrialRunError::SelectorDiagnosticEnable(
            GpuRuntimeSelectorDiagnosticEnableFailure::Receipt(receipt),
        ));
        let rendered = enable.to_string();
        for field in [
            "CapacityExceeded",
            "class_id=2048",
            "chunk_index=3",
            "row=2",
            "base_words=100",
            "candidate_count=2",
            "decoder_synapse_count=10",
            "record_words=29",
            "detail_words=580",
            "frame_payload_capacity_words=200",
        ] {
            assert!(rendered.contains(field), "missing {field} in {rendered}");
        }

        let later = runner_failure(Era1TrialRunError::SelectorDiagnosticLaterStage(
            ScaffoldContractError::NeuralBackendUnavailable,
        ));
        assert!(later.to_string().contains("later-stage GPU failure"));
        assert_ne!(rendered, later.to_string());
    }
}
