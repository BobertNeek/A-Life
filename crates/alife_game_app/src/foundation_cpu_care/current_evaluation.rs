//! Read-only Current admission into a new native CPU assessment life. A Current
//! is never relabeled as a CPU training receipt or a restored CPU individual.
use super::evaluation::{file_hash, hash_value, inventory, read_json, verify_pins};
use super::founder_evaluation::{
    collect_founder_assessment, collect_founder_assessment_with_policy, verify_founder_artifacts,
    write,
};
use super::*;
use alife_training::{CheckpointSeedsV1, CurrentTrainingCheckpointV1};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const MODE: &str = "current-to-native-cpu-fresh-founder-assessment";
const FULL_MODE: &str = "current-to-native-cpu-frozen-full-actor-assessment";
const FULL_PROOF: &str = "current-full-actor-preservation.json";

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
    /// Explicit source-only body portability proof. It never changes saved
    /// values or the new assessment body and is bound by the native receipt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saved_body_rounding_compatibility: Option<alife_world::SavedBodyRoundingCompatibilityV1>,
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
    handoff: alife_training::FoundationTrainingHandoffV1,
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
    let current = match &request.saved_body_rounding_compatibility {
        None => crate::read_current_training_checkpoint(&request.current)?,
        Some(proof) => crate::read_current_training_checkpoint_with_body_rounding(
            &request.current,
            &request.expected_current_sha256,
            proof,
        )?,
    };
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
        "exclusion_kinds_reconstructed":false,
        "saved_body_rounding_compatibility":request.saved_body_rounding_compatibility,
        "source_biological_bits_modified":false});
    if file_hash(&request.current)? != source_hash {
        return Err("Current changed during native admission".into());
    }
    Ok(AdmittedCurrent {
        asset_bytes,
        asset_sha256,
        policy_version: source.policy_version,
        proof,
        handoff: current.handoff,
    })
}

pub fn evaluate_cpu_current(request: CpuCurrentEvaluationRequest) -> Result<Value> {
    evaluate_current(request, false)
}

/// Fresh-life parity control with the actual saved actor and full Adam. Neither
/// source private life nor critic state is transplanted or modified.
pub fn evaluate_cpu_current_full(request: CpuCurrentEvaluationRequest) -> Result<Value> {
    evaluate_current(request, true)
}

fn full_actor_proof(
    admitted: &AdmittedCurrent,
    request: &CpuCurrentEvaluationRequest,
) -> Result<Value> {
    let mut source = admitted.handoff.clone().admit()?;
    let (game, next, _) =
        compile_cpu_founder_game(request.seed, request.founder_seed, &source.foundation)?;
    drop(game);
    validate_cpu_founder_rebinding(&source.phenotype, &source.actor, &next, &source.foundation)?;
    let source_actor_sha256 = hash_value(&source.actor)?;
    source.actor.phenotype_hash = next.phenotype_hash();
    source.actor.source_foundation_digest = source.foundation.digest();
    Ok(
        json!({"schema_version":1,"kind":"actual-current-frozen-full-actor-control",
        "source_actor_sha256":source_actor_sha256,
        "rebound_actor_before_sha256":hash_value(&source.actor)?,
        "rebound_actor_after_sha256":hash_value(&source.actor)?,
        "source_value_sha256":hash_value(&source.value)?,
        "source_private_life_sha256":admitted.proof["source_private_life_sha256"],
        "allowed_rebinding_metadata_fields":["phenotype_hash","source_foundation_digest"],
        "preserved_fields":["weights","first_moment","second_moment","update_ages","optimizer_step","config","stage_mask"],
        "actor_optimizer_updates":0,"value_optimizer_updates":0,
        "critic_checkpoint_preserved_and_unused":true,"same_individual_restore":false}),
    )
}

fn evaluate_current(mut request: CpuCurrentEvaluationRequest, full: bool) -> Result<Value> {
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
    let mode = if full { FULL_MODE } else { MODE };
    let context = json!({"mode":mode,"source_kind":"native-current",
        "source_current_sha256":request.expected_current_sha256,"source_state_preserved":true,
        "private_life_transferred":false,"sampler_transferred":false,
        "full_actor_control":full,"offline_optimizer_constructed":full,"critic_constructed":full,
        "seed_anchor_scope":"exact admitted Current policy head and declared exclusions; explicitly new prepared life, no personal continuation"});
    let result = if full {
        let expected = full_actor_proof(&admitted, &request)?;
        let source = admitted.handoff.clone().admit()?;
        let value = CpuValueHead::from_checkpoint(source.value.clone())?;
        if hash_value(&value.checkpoint())? != expected["source_value_sha256"] {
            return Err("full Current critic failed exact saved-state restoration".into());
        }
        let mut actual_before = None;
        let (result, trainer) = collect_founder_assessment_with_policy(
            &controls,
            admitted.asset_bytes.clone(),
            request.founder_seed,
            admitted.policy_version,
            &pins,
            Some(&context),
            |next, asset| {
                let mut trainer = CpuFoundationTrainer::from_checkpoint(
                    source.phenotype,
                    source.foundation,
                    source.actor,
                )?;
                trainer.rebind_for_next_cohort(next, asset)?;
                actual_before = Some(hash_value(&trainer.checkpoint())?);
                Ok(trainer)
            },
        )?;
        if actual_before.as_deref() != expected["rebound_actor_before_sha256"].as_str()
            || hash_value(&trainer.checkpoint())? != expected["rebound_actor_after_sha256"]
            || hash_value(&value.checkpoint())? != expected["source_value_sha256"]
        {
            return Err(
                "full Current actor or Adam changed during fresh assessment; retain private prefix"
                    .into(),
            );
        }
        write(request.output.join(FULL_PROOF), &expected)?;
        result
    } else {
        collect_founder_assessment(
            &controls,
            admitted.asset_bytes.clone(),
            request.founder_seed,
            admitted.policy_version,
            &pins,
            Some(&context),
        )?
    };
    write(
        request.output.join("current-assessment-source.json"),
        &admitted.proof,
    )?;
    verify_pins(&pins)?;
    let mut receipt = CurrentReceipt {
        schema_version: 1,
        mode: mode.into(),
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
        || !matches!(receipt.mode.as_str(), MODE | FULL_MODE)
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
    let mut verified = verify_founder_artifacts(root, &receipt.asset_sha256)?;
    let result = &verified["result"];
    if result["mode"] != receipt.mode
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
    if receipt.mode == FULL_MODE {
        if result["full_actor_control"] != true
            || result["offline_optimizer_constructed"] != true
            || result["critic_constructed"] != true
            || fs::read(root.join(FULL_PROOF))?
                != serde_json::to_vec(&full_actor_proof(&admitted, &receipt.request)?)?
        {
            return Err("full Current control lost its exact actor/Adam preservation proof".into());
        }
        verified["offline_optimizer_constructed"] = json!(true);
        verified["verification_offline_optimizer_constructed"] = json!(false);
    } else if root.join(FULL_PROOF).exists() || result["offline_optimizer_constructed"] != false {
        return Err("bare Current assessment claims a full actor or optimizer".into());
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
        Some(MODE | FULL_MODE) => verify_cpu_current_evaluation(root),
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

/// Parent full/bare parity is stricter than between-policy comparison: actions
/// and the entire new private life must also match exactly. No source is moved.
pub fn verify_cpu_current_parity(left: &Path, right: &Path) -> Result<Value> {
    let left_verified = verify_cpu_current_evaluation(left)?;
    let right_verified = verify_cpu_current_evaluation(right)?;
    let mut a: CurrentReceipt = read_json(&left.join(CPU_CARE_EVALUATION_FILE))?;
    let b: CurrentReceipt = read_json(&right.join(CPU_CARE_EVALUATION_FILE))?;
    if !matches!(
        (a.mode.as_str(), b.mode.as_str()),
        (MODE, FULL_MODE) | (FULL_MODE, MODE)
    ) {
        return Err(
            "Current parity requires one actual full-actor control and one bare assessment".into(),
        );
    }
    a.request.output = b.request.output.clone();
    if hash_value(&a.request)? != hash_value(&b.request)?
        || left_verified["comparison"] != right_verified["comparison"]
    {
        return Err(
            "Current parity source, request, binary or native initial controls differ".into(),
        );
    }
    for name in ["first-input.json", "care-trace.json"] {
        if fs::read(left.join(name))? != fs::read(right.join(name))? {
            return Err("Current full/bare actual inputs or actions differ".into());
        }
    }
    if read_cpu_private_checkpoint_bytes(left, 64 * 1024 * 1024)?
        != read_cpu_private_checkpoint_bytes(right, 64 * 1024 * 1024)?
    {
        return Err("Current full/bare complete private checkpoints differ".into());
    }
    Ok(
        json!({"parity_verified":true,"source_current_sha256":a.request.expected_current_sha256,
        "full_actor_and_adam_preserved":true,"source_private_life_preserved":true,
        "actual_inputs_and_actions_exact":true,"new_private_checkpoint_exact":true,
        "native_initial_controls_and_executable_exact":true,"same_individual_restore":false,
        "scope":"actual saved policy head in matched new CPU lives; not saved GPU private-life or cross-platform restoration",
        "automatic_adoption":false}),
    )
}

#[cfg(test)]
mod tests;
