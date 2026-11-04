//! Read-only Current admission into a new native CPU assessment life. A Current
//! is never relabeled as a CPU training receipt or a restored CPU individual.
use super::evaluation::{file_hash, hash_value, inventory, read_json, verify_pins};
use super::founder_evaluation::{collect_founder_assessment, verify_founder_artifacts, write};
use super::*;
use alife_training::{CheckpointSeedsV1, CurrentTrainingCheckpointV1};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const MODE: &str = "current-to-native-cpu-fresh-founder-assessment";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCurrentEvaluationRequest {
    pub current: PathBuf,
    pub expected_current_sha256: String,
    pub seed_history: PathBuf,
    pub output: PathBuf,
    /// Explicit benchmark body, independent of the historical Current body.
    pub founder_seed: u64,
    pub seed: u64,
    pub decisions: usize,
    pub temperature: f32,
    pub lesson: FoundationTeacherLesson,
    #[serde(default)]
    pub food_need_condition: FoundationFoodNeedCondition,
    /// Required opt-in: this adapter never upgrades historical freshness.
    pub known_exclusions: bool,
}

/// An assessment matrix declaration, not a fabricated consumed-seed ledger.
/// Exclusions are the declared union from both parents; every Current exclusion
/// must remain included. Their original consumed/reserved/assessment kinds are
/// not recoverable from coalesced Current ranges and are never guessed here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCurrentAssessmentSeeds {
    pub schema_version: u16,
    pub kind: String,
    pub source_current_sha256: String,
    pub historical_freshness: String,
    pub excluded_ranges: Vec<[u64; 2]>,
    pub assessment_seeds: Vec<u64>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentReceipt {
    schema_version: u16,
    mode: String,
    request: CpuCurrentEvaluationRequest,
    asset_sha256: String,
    input_sha256: BTreeMap<PathBuf, String>,
    artifacts: BTreeMap<String, String>,
    payload_sha256: String,
}
impl CurrentReceipt {
    fn digest(&self) -> Result<String> {
        hash_value(&(
            "ALIFE-CURRENT-CPU-FRESH-ASSESSMENT-V1",
            self.schema_version,
            &self.mode,
            &self.request,
            &self.asset_sha256,
            &self.input_sha256,
            &self.artifacts,
        ))
    }
}

fn admit_seeds(
    request: &CpuCurrentEvaluationRequest,
    current: &CurrentTrainingCheckpointV1,
    history: &CpuCurrentAssessmentSeeds,
) -> Result<()> {
    if !request.known_exclusions
        || history.schema_version != 1
        || history.kind != "current-cpu-assessment-seeds"
        || history.source_current_sha256 != request.expected_current_sha256
        || history.historical_freshness != "unknown"
    {
        return Err("Current assessment requires explicit source-bound known exclusions and unknown historical freshness".into());
    }
    let union = CheckpointSeedsV1 {
        historical_freshness: "unknown".into(),
        excluded_ranges: history.excluded_ranges.clone(),
        pending_range: None,
    };
    union.validate()?;
    if current
        .seeds
        .excluded_ranges
        .iter()
        .any(|[a, b]| !union.excluded_ranges.iter().any(|[x, y]| x <= a && b <= y))
    {
        return Err(
            "assessment declaration drops a Current exclusion or pending allocation".into(),
        );
    }
    let mut declared = std::collections::BTreeSet::new();
    for seed in &history.assessment_seeds {
        if *seed == 0 || !declared.insert(*seed) || union.contains(*seed) {
            return Err(
                "assessment matrix repeats an invalid seed or collides with a known exclusion"
                    .into(),
            );
        }
    }
    if !declared.contains(&request.seed) {
        return Err("requested seed is not explicitly declared in the assessment matrix".into());
    }
    Ok(())
}

struct AdmittedCurrent {
    asset_bytes: Vec<u8>,
    asset_sha256: String,
    policy_version: u64,
    proof: Value,
}
fn admit_current(request: &CpuCurrentEvaluationRequest) -> Result<AdmittedCurrent> {
    if request.seed == 0
        || request.founder_seed == 0
        || !(1..=128).contains(&request.decisions)
        || !request.temperature.is_finite()
        || request.temperature <= 0.
    {
        return Err("invalid Current assessment sampling, body seed or decision dose".into());
    }
    request
        .food_need_condition
        .validate_lesson(Some(request.lesson))?;
    let source_hash = file_hash(&request.current)?;
    if source_hash != request.expected_current_sha256 {
        return Err("Current bytes differ from the explicitly selected source SHA256".into());
    }
    let current = crate::read_current_training_checkpoint(&request.current)?;
    let history: CpuCurrentAssessmentSeeds = read_json(&request.seed_history)?;
    admit_seeds(request, &current, &history)?;
    let source = current.handoff.clone().admit()?;
    let (game, next, _) =
        compile_cpu_founder_game(request.seed, request.founder_seed, &source.foundation)?;
    drop(game);
    // Reuse all native rebinding guards without constructing/resetting Adam.
    validate_cpu_founder_rebinding(&source.phenotype, &source.actor, &next, &source.foundation)?;
    let _ = CpuInferenceFounder::new(next, source.foundation.clone())?;
    let asset_bytes = source.foundation.encode_canonical()?;
    let asset_sha256 = format!("{:x}", Sha256::digest(&asset_bytes));
    let proof = json!({"schema_version":1,"source_kind":"native-current",
        "source_current_sha256":source_hash,"origin":source.origin,
        "source_policy_version":source.policy_version,"source_seed":current.seed,
        "historical_founder_seed":current.founder_seed_base,"assessment_founder_seed":request.founder_seed,
        "source_continuation":current.handoff.continuation,
        "source_same_individual":current.same_individual.is_some(),
        "source_terminal_disposition":current.terminal_disposition.is_some(),
        "source_training_attempt_origin":current.training_attempt_origin.is_some(),
        "source_actor_sha256":hash_value(&current.handoff.actor)?,
        "source_value_sha256":hash_value(&current.handoff.value)?,
        "source_private_life_sha256":hash_value(&current.life)?,
        "source_actor_optimizer_step":source.actor.optimizer_step,
        "source_value_optimizer_step":source.value.optimizer_step,
        "source_seeds":current.seeds,"assessment_seed_declaration":history,
        "historical_freshness":"unknown","candidate_sha256":asset_sha256,
        "native_rebinding_compatible":true,"source_changed":false,
        "same_individual_restore":false,"private_life_transferred":false,
        "sampler_transferred":false,"training_continuation":false,
        "exclusion_kinds_reconstructed":false});
    if file_hash(&request.current)? != source_hash {
        return Err("Current changed during native admission".into());
    }
    Ok(AdmittedCurrent {
        asset_bytes,
        asset_sha256,
        policy_version: source.policy_version,
        proof,
    })
}

pub fn evaluate_cpu_current(mut request: CpuCurrentEvaluationRequest) -> Result<Value> {
    // Check before canonicalization too; do not quietly dereference a source symlink.
    file_hash(&request.current)?;
    request.current = request.current.canonicalize()?;
    request.seed_history = request.seed_history.canonicalize()?;
    let executable = std::env::current_exe()?.canonicalize()?;
    let pins = [&request.current, &request.seed_history, &executable]
        .into_iter()
        .map(|p| Ok((p.clone(), file_hash(p)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    request.output = alife_training::resolve_isolated_training_output(
        &request.output,
        pins.keys().map(PathBuf::as_path),
    )?;
    if request.output.exists() {
        return Err(
            "Current assessment output exists; preserve its life and use verify, never restart"
                .into(),
        );
    }
    let admitted = admit_current(&request)?;
    verify_pins(&pins)?;
    // Internal runner controls only. No legacy CPU request/receipt is emitted,
    // and the founder bytes are from the admitted embedded asset, not this path.
    let controls = CpuFounderEvaluationRequest {
        foundation: request.current.clone(),
        expected_foundation_sha256: admitted.asset_sha256.clone(),
        seed_anchor: request.current.clone(),
        seed_history: request.seed_history.clone(),
        output: request.output.clone(),
        seed: request.seed,
        decisions: request.decisions,
        temperature: request.temperature,
        lesson: request.lesson,
        food_need_condition: request.food_need_condition,
        known_exclusions: true,
    };
    let context = json!({"mode":MODE,"source_kind":"native-current",
        "source_current_sha256":request.expected_current_sha256,"source_state_preserved":true,
        "private_life_transferred":false,"sampler_transferred":false,
        "seed_anchor_scope":"exact admitted Current policy head and declared exclusions; explicitly new prepared life, no personal continuation"});
    let result = collect_founder_assessment(
        &controls,
        admitted.asset_bytes,
        request.founder_seed,
        admitted.policy_version,
        &pins,
        Some(&context),
    )?;
    write(
        request.output.join("current-assessment-source.json"),
        &admitted.proof,
    )?;
    verify_pins(&pins)?;
    let mut receipt = CurrentReceipt {
        schema_version: 1,
        mode: MODE.into(),
        request: request.clone(),
        asset_sha256: admitted.asset_sha256,
        input_sha256: pins,
        artifacts: inventory(&request.output)?,
        payload_sha256: String::new(),
    };
    receipt.payload_sha256 = receipt.digest()?;
    write(request.output.join(CPU_CARE_EVALUATION_FILE), &receipt)?;
    verify_cpu_current_evaluation(&request.output)?;
    Ok(result)
}

pub fn verify_cpu_current_evaluation(root: &Path) -> Result<Value> {
    let receipt: CurrentReceipt = read_json(&root.join(CPU_CARE_EVALUATION_FILE))?;
    if receipt.schema_version != 1
        || receipt.mode != MODE
        || receipt.request.output != root.canonicalize()?
        || receipt.payload_sha256 != receipt.digest()?
        || receipt.artifacts != inventory(root)?
    {
        return Err("Current assessment seal, path or artifacts changed".into());
    }
    verify_pins(&receipt.input_sha256)?;
    let admitted = admit_current(&receipt.request)?;
    if admitted.asset_sha256 != receipt.asset_sha256
        || fs::read(root.join("current-assessment-source.json"))?
            != serde_json::to_vec(&admitted.proof)?
    {
        return Err("Current assessment differs from exact native source admission".into());
    }
    let verified = verify_founder_artifacts(root, &receipt.asset_sha256)?;
    let result = &verified["result"];
    if result["mode"] != MODE
        || result["source_current_sha256"] != receipt.request.expected_current_sha256
        || result["same_individual_restore"] != false
        || result["private_life_transferred"] != false
        || result["sampler_transferred"] != false
        || result["historical_freshness"] != "unknown"
        || result["evaluated_decisions"] != receipt.request.decisions
        || result["bootstrap_decisions"] != 1
    {
        return Err("Current assessment scope or dose changed".into());
    }
    let trace: Value = read_json(&root.join("care-trace.json"))?;
    let rows = trace["rows"]
        .as_array()
        .ok_or("Current assessment lacks trace rows")?;
    if rows.len() != receipt.request.decisions + 1
        || rows.iter().enumerate().any(|(i, row)| {
            row["loss_row"] != false
                || row["teacher_heard"] != false
                || row["evaluation_row"] != (i < receipt.request.decisions)
                || row["bootstrap_row"] != (i >= receipt.request.decisions)
        })
    {
        return Err("Current assessment trace dose or teacher/loss boundary changed".into());
    }
    verify_pins(&receipt.input_sha256)?;
    Ok(verified)
}

/// Explicit domain dispatch, never fallback after a failed verifier. Existing
/// production comparator remains unchanged; cross-domain equality is just as
/// strict, including executable SHA256 and all native initial controls.
pub fn verify_cpu_fresh_assessment(root: &Path) -> Result<Value> {
    let header: Value = read_json(&root.join(CPU_CARE_EVALUATION_FILE))?;
    match header["mode"].as_str() {
        Some(MODE) => verify_cpu_current_evaluation(root),
        Some("native-cpu-founder-inference-assessment") => verify_cpu_founder_evaluation(root),
        _ => Err("unsupported fresh CPU assessment receipt domain".into()),
    }
}
pub fn compare_cpu_fresh_assessments(left: &Path, right: &Path) -> Result<Value> {
    let left = verify_cpu_fresh_assessment(left)?;
    let right = verify_cpu_fresh_assessment(right)?;
    Ok(json!({"matched":left["comparison"] == right["comparison"],
        "scope":"fresh native CPU world/body and first actual neural inputs; later states are outcomes",
        "left":left,"right":right,"automatic_adoption":false}))
}

#[cfg(test)]
mod tests;
