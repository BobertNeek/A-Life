//! Bare-founder fresh-life assessment. No offline optimizer is constructed.
use super::evaluation::{file_hash, hash_value, input_pins, inventory, read_json, verify_pins};
use super::*;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const BARE_MODE: &str = "native-cpu-founder-inference-assessment";
const FULL_MODE: &str = "native-cpu-full-founder-actor-assessment";
const FULL_PROOF: &str = "native-full-actor-preservation.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuFounderEvaluationRequest {
    pub foundation: PathBuf,
    pub expected_foundation_sha256: String,
    /// Native CPU source authenticates the declared seed-exclusion history and
    /// fixes fresh-body construction. Its actor/critic are never transplanted.
    pub seed_anchor: PathBuf,
    pub seed_history: PathBuf,
    pub output: PathBuf,
    pub seed: u64,
    pub decisions: usize,
    pub temperature: f32,
    pub lesson: FoundationTeacherLesson,
    #[serde(default)]
    pub food_need_condition: FoundationFoodNeedCondition,
    #[serde(default)]
    pub known_exclusions: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FounderReceipt {
    schema_version: u16,
    mode: String,
    request: CpuFounderEvaluationRequest,
    input_sha256: BTreeMap<PathBuf, String>,
    artifacts: BTreeMap<String, String>,
    payload_sha256: String,
}
impl FounderReceipt {
    fn digest(&self) -> Result<String> {
        hash_value(&(
            "ALIFE-CPU-FOUNDER-INFERENCE-ASSESSMENT-V1",
            self.schema_version,
            &self.mode,
            &self.request,
            &self.input_sha256,
            &self.artifacts,
        ))
    }
}

pub(super) fn write(path: impl AsRef<Path>, value: &impl Serialize) -> Result<()> {
    use std::io::Write;
    let mut f = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    f.write_all(&serde_json::to_vec(value)?)?;
    f.sync_all()?;
    Ok(())
}
pub(super) fn reconstruct(root: &Path) -> Result<CpuInferenceFounder> {
    let inputs: alife_core::PhenotypeCompilerInputs =
        read_json(&root.join("collection-compiler-inputs.json"))?;
    let asset = alife_core::FoundationWeightAsset::decode_canonical(&fs::read(
        root.join("collection-policy.alife-foundation"),
    )?)?;
    let phenotype = alife_core::PhenotypeCompiler::compile_validated(
        &inputs,
        &alife_core::BrainCapacityClass::n2048(),
    )?;
    Ok(CpuInferenceFounder::new(phenotype, asset)?)
}

pub fn verify_cpu_founder_evaluation(root: &Path) -> Result<Value> {
    let receipt: FounderReceipt = read_json(&root.join(CPU_CARE_EVALUATION_FILE))?;
    if !matches!(
        (receipt.schema_version, receipt.mode.as_str()),
        (2, BARE_MODE) | (3, FULL_MODE)
    ) || receipt.payload_sha256 != receipt.digest()?
        || receipt.artifacts != inventory(root)?
    {
        return Err("founder assessment seal or retained artifacts changed".into());
    }
    if receipt.request.output != root.canonicalize()? {
        return Err(
            "founder assessment moved; preserve and explicitly rebind metadata before verification"
                .into(),
        );
    }
    verify_pins(&receipt.input_sha256)?;
    let mut verified = verify_founder_artifacts(root, &receipt.request.expected_foundation_sha256)?;
    if receipt.mode == FULL_MODE {
        let source = read_cpu_founder_source(&receipt.request.seed_anchor)?;
        let expected = native_full_proof(&receipt.request, &source.handoff)?;
        if fs::read(root.join(FULL_PROOF))? != serde_json::to_vec(&expected)?
            || verified["result"]["mode"] != FULL_MODE
            || verified["result"]["full_actor_control"] != true
        {
            return Err(
                "native full founder lost exact actor/value Adam preservation proof".into(),
            );
        }
        verified["offline_optimizer_constructed"] = json!(true);
    } else if root.join(FULL_PROOF).exists() || verified["result"]["mode"] != BARE_MODE {
        return Err("bare founder assessment claims full optimizer state".into());
    }
    Ok(verified)
}

pub(super) fn verify_founder_artifacts(root: &Path, expected_asset_sha256: &str) -> Result<Value> {
    let result: Value = read_json(&root.join("founder-result.json"))?;
    let comparison: Value = read_json(&root.join("founder-comparison.json"))?;
    if file_hash(&root.join("collection-policy.alife-foundation"))? != expected_asset_sha256 {
        return Err(
            "retained collection founder differs from explicitly selected candidate".into(),
        );
    }
    let founder = reconstruct(root)?;
    let checkpoint = read_cpu_private_checkpoint(root)?;
    let before = serde_json::to_vec(&checkpoint)?;
    let host = CpuCanonicalHost::from_checkpoint(checkpoint, &founder)?;
    evaluation::ensure_no_teacher(host.world())?;
    if serde_json::to_vec(&host.checkpoint(&founder)?)? != before {
        return Err("founder assessment private checkpoint failed exact roundtrip".into());
    }
    for forbidden in [
        "actor-checkpoint.json",
        "value-checkpoint.json",
        "collection-actor-checkpoint.json",
        "initial-value-checkpoint.json",
        "training-handoff.json",
        "cpu-cycle.json",
    ] {
        if root.join(forbidden).exists() {
            return Err("inference-only output contains offline learning/source state".into());
        }
    }
    let episode: Value = read_json(&root.join("care-episode.json"))?;
    let prior: Value = read_json(&root.join("care-prior-lives.json"))?;
    if episode["semantic_prior_enabled"] != false
        || episode["teacher_present"] != false
        || episode["offline_actor_optimizer_updates"] != 0
        || episode["offline_value_optimizer_updates"] != 0
        || !prior["provider_identity"].is_null()
        || prior["lives"].as_object().is_none_or(|v| !v.is_empty())
        || prior["world_tick"] != json!(host.world().tick())
    {
        return Err("inference assessment teacher/prior/offline learning boundary changed".into());
    }
    Ok(
        json!({"verified":true,"result":result,"comparison":comparison,
        "private_checkpoint_roundtrip_exact":true,"verification_neural_dispatches":0,"offline_optimizer_constructed":false}),
    )
}

pub fn compare_cpu_founder_evaluations(left: &Path, right: &Path) -> Result<Value> {
    let left = verify_cpu_founder_evaluation(left)?;
    let right = verify_cpu_founder_evaluation(right)?;
    Ok(json!({"matched":left["comparison"] == right["comparison"],
        "scope":"fresh native CPU world/body and first actual neural inputs; later states are outcomes",
        "left":left,"right":right,"automatic_adoption":false}))
}

pub fn evaluate_cpu_founder(request: CpuFounderEvaluationRequest) -> Result<Value> {
    evaluate_native_founder(request, false)
}
pub fn evaluate_cpu_full_founder(request: CpuFounderEvaluationRequest) -> Result<Value> {
    evaluate_native_founder(request, true)
}
fn native_full_proof(
    request: &CpuFounderEvaluationRequest,
    handoff: &alife_training::FoundationTrainingHandoffV1,
) -> Result<Value> {
    let source = handoff.clone().admit()?;
    let native = read_cpu_founder_source(&request.seed_anchor)?;
    let founder_seed = native.receipt["founder_seed_base"]
        .as_u64()
        .ok_or("native full source lacks body seed")?;
    if source.foundation.encode_canonical()? != fs::read(&request.foundation)?
        || file_hash(&request.foundation)? != request.expected_foundation_sha256
    {
        return Err("full founder control must use its actual native source policy".into());
    }
    let (_, next, _) = compile_cpu_founder_game(request.seed, founder_seed, &source.foundation)?;
    validate_cpu_founder_rebinding(&source.phenotype, &source.actor, &next, &source.foundation)?;
    let mut actor = source.actor.clone();
    actor.phenotype_hash = next.phenotype_hash();
    actor.source_foundation_digest = source.foundation.digest();
    Ok(
        json!({"kind":"actual-native-cpu-frozen-full-actor-control","schema_version":1,
        "source_actor_sha256":hash_value(&source.actor)?,"source_value_sha256":hash_value(&source.value)?,
        "rebound_actor_before_sha256":hash_value(&actor)?,"rebound_actor_after_sha256":hash_value(&actor)?,
        "allowed_rebinding_metadata_fields":["phenotype_hash","source_foundation_digest"],
        "preserved_fields":["weights","first_moment","second_moment","update_ages","optimizer_step","config","stage_mask"],
        "actor_optimizer_updates":0,"value_optimizer_updates":0,"critic_checkpoint_preserved_and_unused":true,
        "private_life_transferred":false,"same_individual_restore":false}),
    )
}
fn evaluate_native_founder(mut request: CpuFounderEvaluationRequest, full: bool) -> Result<Value> {
    request.foundation = request.foundation.canonicalize()?;
    request.seed_anchor = request.seed_anchor.canonicalize()?;
    request.seed_history = request.seed_history.canonicalize()?;
    let source = read_cpu_founder_source(&request.seed_anchor)?;
    let expected = file_hash(&request.foundation)?;
    if expected != request.expected_foundation_sha256 {
        return Err("selected founder asset differs from expected SHA256".into());
    }
    let asset_bytes = fs::read(&request.foundation)?;
    alife_core::FoundationWeightAsset::decode_canonical(&asset_bytes)?;
    request.output = alife_training::resolve_isolated_training_output(
        &request.output,
        [
            &request.foundation,
            &request.seed_anchor,
            &request.seed_history,
        ]
        .map(PathBuf::as_path),
    )?;
    if request.output.exists() {
        return Err(
            "assessment output already exists; preserve its life and use verify, never restart"
                .into(),
        );
    }
    let anchor_request = CpuCareEvaluationRequest {
        previous: request.seed_anchor.clone(),
        output: request.output.clone(),
        seed_history: request.seed_history.clone(),
        seed: request.seed,
        decisions: request.decisions,
        temperature: request.temperature,
        lesson: request.lesson,
        food_need_condition: request.food_need_condition,
        known_exclusions: request.known_exclusions,
    };
    if !request.temperature.is_finite() || request.temperature <= 0. || request.seed == 0 {
        return Err("invalid founder assessment sampling".into());
    }
    let provenance = crate::foundation_training_cycle::admit_native_cpu_care_assessment_seed(
        &request.seed_anchor,
        &request.seed_history,
        &request.output,
        request.seed,
        request.known_exclusions,
    )?;
    let mut pins = input_pins(&anchor_request, &source.files, &provenance)?;
    pins.insert(request.foundation.clone(), expected);
    pins.insert(
        std::env::current_exe()?.canonicalize()?,
        file_hash(&std::env::current_exe()?)?,
    );
    request.output = alife_training::resolve_isolated_training_output(
        &request.output,
        pins.keys().map(PathBuf::as_path),
    )?;
    let founder_seed = source.receipt["founder_seed_base"]
        .as_u64()
        .filter(|v| *v != 0)
        .or(source.receipt["seed"].as_u64())
        .ok_or("seed anchor lacks founder construction seed")?;
    let result = if full {
        let expected = native_full_proof(&request, &source.handoff)?;
        let admitted = source.handoff.clone().admit()?;
        let value = CpuValueHead::from_checkpoint(admitted.value.clone())?;
        let mut before = None;
        let context = json!({"mode":FULL_MODE,"full_actor_control":true,"offline_optimizer_constructed":true,"critic_constructed":true});
        let (result, trainer) = collect_founder_assessment_with_policy(
            &request,
            asset_bytes,
            founder_seed,
            source.handoff.policy_version,
            &pins,
            Some(&context),
            |next, asset| {
                let mut trainer = CpuFoundationTrainer::from_checkpoint(
                    admitted.phenotype,
                    admitted.foundation,
                    admitted.actor,
                )?;
                trainer.rebind_for_next_cohort(next, asset)?;
                before = Some(hash_value(&trainer.checkpoint())?);
                Ok(trainer)
            },
        )?;
        if before.as_deref() != expected["rebound_actor_before_sha256"].as_str()
            || hash_value(&trainer.checkpoint())? != expected["rebound_actor_after_sha256"]
            || hash_value(&value.checkpoint())? != expected["source_value_sha256"]
        {
            return Err(
                "native full founder actor or value changed during assessment; preserve prefix"
                    .into(),
            );
        }
        write(request.output.join(FULL_PROOF), &expected)?;
        result
    } else {
        collect_founder_assessment(
            &request,
            asset_bytes,
            founder_seed,
            source.handoff.policy_version,
            &pins,
            None,
        )?
    };
    let mut receipt = FounderReceipt {
        schema_version: if full { 3 } else { 2 },
        mode: if full { FULL_MODE } else { BARE_MODE }.into(),
        request: request.clone(),
        input_sha256: pins,
        artifacts: inventory(&request.output)?,
        payload_sha256: String::new(),
    };
    receipt.payload_sha256 = receipt.digest()?;
    write(request.output.join(CPU_CARE_EVALUATION_FILE), &receipt)?;
    verify_cpu_founder_evaluation(&request.output)?;
    Ok(result)
}

pub fn verify_cpu_native_founder_parity(left: &Path, right: &Path) -> Result<Value> {
    let a = verify_cpu_founder_evaluation(left)?;
    let b = verify_cpu_founder_evaluation(right)?;
    let mut ra: FounderReceipt = read_json(&left.join(CPU_CARE_EVALUATION_FILE))?;
    let rb: FounderReceipt = read_json(&right.join(CPU_CARE_EVALUATION_FILE))?;
    if !matches!(
        (ra.mode.as_str(), rb.mode.as_str()),
        (BARE_MODE, FULL_MODE) | (FULL_MODE, BARE_MODE)
    ) {
        return Err("native parity requires one bare and one full native parent".into());
    }
    ra.request.output = rb.request.output.clone();
    if hash_value(&ra.request)? != hash_value(&rb.request)? || a["comparison"] != b["comparison"] {
        return Err("native full/bare source, case, initial state or executable differs".into());
    }
    for name in ["first-input.json", "care-trace.json"] {
        if fs::read(left.join(name))? != fs::read(right.join(name))? {
            return Err("native full/bare inputs or actions differ".into());
        }
    }
    if read_cpu_private_checkpoint_bytes(left, 64 * 1024 * 1024)?
        != read_cpu_private_checkpoint_bytes(right, 64 * 1024 * 1024)?
    {
        return Err("native full/bare complete private state differs".into());
    }
    Ok(
        json!({"parity_verified":true,"full_actor_and_adam_preserved":true,"actual_inputs_and_actions_exact":true,
        "new_private_checkpoint_exact":true,"native_initial_controls_and_executable_exact":true,"automatic_adoption":false}),
    )
}

/// Shared native life runner only; each entry point owns its source admission
/// and independent receipt contract. No source is synthesized here.
pub(super) fn collect_founder_assessment(
    request: &CpuFounderEvaluationRequest,
    asset_bytes: Vec<u8>,
    founder_seed: u64,
    anchor_policy_version: u64,
    pins: &BTreeMap<PathBuf, String>,
    result_context: Option<&Value>,
) -> Result<Value> {
    Ok(collect_founder_assessment_with_policy(
        request,
        asset_bytes,
        founder_seed,
        anchor_policy_version,
        pins,
        result_context,
        |phenotype, asset| Ok(CpuInferenceFounder::new(phenotype, asset)?),
    )?
    .0)
}

/// The same native life may use a real saved full actor as a read-only parity
/// control. Returning the policy lets the caller prove full Adam stayed exact.
pub(super) fn collect_founder_assessment_with_policy<F: CpuFounderPolicy>(
    request: &CpuFounderEvaluationRequest,
    asset_bytes: Vec<u8>,
    founder_seed: u64,
    anchor_policy_version: u64,
    pins: &BTreeMap<PathBuf, String>,
    result_context: Option<&Value>,
    make_policy: impl FnOnce(alife_core::BrainPhenotype, alife_core::FoundationWeightAsset) -> Result<F>,
) -> Result<(Value, F)> {
    let asset = alife_core::FoundationWeightAsset::decode_canonical(&asset_bytes)?;
    let (_, mut episode) = prepare_care_episode(CpuCareRequest {
        previous: Some(request.seed_anchor.clone()),
        output: request.output.clone(),
        seed: request.seed,
        decisions: request.decisions,
        temperature: request.temperature,
        lesson: request.lesson,
        food_need_condition: request.food_need_condition,
        seed_history: Some(request.seed_history.clone()),
        known_exclusions: request.known_exclusions,
        record_diagnostic_replay: false,
        fixture: false,
    })?;
    let (game, phenotype, inputs) = compile_cpu_founder_game(request.seed, founder_seed, &asset)?;
    let founder = make_policy(phenotype, asset)?;
    episode.assessment = true;
    episode.founder_only = true;
    episode.frozen_actor_control = result_context.is_some_and(|v| v["full_actor_control"] == true);
    episode.validate_founder_policy(&founder)?;
    // Same shared setup, captured before native private-state admission. The
    // comparison projection is the existing fresh-founder world contract.
    let mut comparison_world = game.world.clone();
    episode.prepare_world(&mut comparison_world)?;
    let subject = comparison_world.organism_entity_ids()[0].0;
    let initial_world =
        comparison_world.fresh_foundation_checkpoint_comparison_signature_v1(subject)?;
    let (mut host, ()) = CpuCanonicalHost::new_prepared_cohort(
        game,
        &founder,
        (request.seed as u32) ^ ((request.seed >> 32) as u32),
        |world| episode.prepare_world(world),
    )?;
    let prepared_physiology = *host
        .world()
        .organism_registry()
        .get(subject)
        .ok_or("assessment learner missing")?
        .biochemistry();
    fs::create_dir_all(
        request
            .output
            .parent()
            .ok_or("assessment output has no parent")?,
    )?;
    fs::create_dir(&request.output)?;
    fs::write(
        request.output.join("collection-policy.alife-foundation"),
        asset_bytes,
    )?;
    write(
        request.output.join("collection-compiler-inputs.json"),
        &inputs,
    )?;
    write(
        request.output.join("initial-private-checkpoint.json"),
        &host.checkpoint(&founder)?,
    )?;
    let mut rows = BTreeMap::<u64, Vec<CpuCollectedDecision>>::new();
    let mut captured = 0;
    let collection = (|| -> Result<()> {
        for _ in 0..request.decisions * 8 + 4096 {
            episode.before_care_step(&mut host, &founder)?;
            let step =
                host.step_with_preparation(&founder, request.temperature, |draft, seq| {
                    episode.prepare_draft(draft, seq)
                })?;
            episode.after_care_step(&host, &founder, &step)?;
            captured += step.decisions.len();
            for row in step.decisions {
                rows.entry(row.organism_id.raw()).or_default().push(row);
            }
            if captured > request.decisions
                || host
                    .world()
                    .organism_registry()
                    .iter()
                    .all(|r| !r.lifecycle().is_alive())
            {
                break;
            }
        }
        Ok(())
    })();
    // Retain native recovery at the last sealed tick even on a failed outcome.
    write_cpu_private_checkpoint(&request.output, &host.checkpoint(&founder)?)?;
    let (_, prior) = episode.private_life_archive(&host, &request.output)?;
    write(request.output.join("care-prior-lives-prefix.json"), &prior)?;
    write(
        request.output.join("care-trace-prefix.json"),
        &episode.traces,
    )?;
    collection?;
    if rows.len() != 1
        || captured != request.decisions + 1
        || host
            .world()
            .organism_registry()
            .iter()
            .any(|r| !r.lifecycle().is_alive())
    {
        return Err(
            "incomplete/censored founder assessment; retain private prefix without qualification"
                .into(),
        );
    }
    let first = &rows.values().next().ok_or("missing assessment rows")?[0];
    let first_input = json!({"initial":first.initial,"context":first.replay_tick,
        "sensory":first.patch.pre_action().perception().sensory()});
    write(request.output.join("first-input.json"), &first_input)?;
    let metadata = episode
        .finish_care_collection(&host, &rows, request.decisions, &request.output)?
        .metadata;
    let comparison = json!({"scope":"fresh-cpu-founder-assessment-v1","pre_runtime_world":initial_world,
        "prepared_physiology_sha256":hash_value(&prepared_physiology)?,"first_input_sha256":hash_value(&first_input)?,
        "seed":request.seed,"founder_seed":founder_seed,"decisions":request.decisions,"temperature":request.temperature,
        "lesson":request.lesson,"food_need_condition":request.food_need_condition,
        "executable_sha256":pins[&std::env::current_exe()?.canonicalize()?]});
    let final_record = host
        .world()
        .organism_registry()
        .get(subject)
        .ok_or("assessment lost learner")?;
    let mut result = json!({"mode":"native-cpu-founder-inference-assessment","backend":"native-cpu",
        "candidate_sha256":request.expected_foundation_sha256,"anchor_policy_version":anchor_policy_version,
        "seed_anchor_scope":"source-bound exclusions and fresh-body construction only; candidate is not a native training continuation",
        "historical_freshness":if request.known_exclusions {"unknown"} else {"operator-attested-complete"},
        "same_individual_restore":false,"body":"new-prepared-life","offline_optimizer_constructed":false,
        "critic_constructed":false,"offline_optimizer_updates":0,"non_resumable_offline_training":true,
        "teacher_present":false,"semantic_prior_enabled":false,"ordinary_private_learning":true,
        "evaluated_decisions":request.decisions,"bootstrap_decisions":1,
        "lesson_completed":metadata["lesson_completed"],"evaluated_nourished_meals":metadata["evaluated_nourished_meals"],
        "final_physiology":final_record.biochemistry(),"world_tick":host.world().tick(),"automatic_adoption":false});
    if let Some(context) = result_context {
        for (key, value) in context
            .as_object()
            .ok_or("invalid assessment result context")?
        {
            result[key] = value.clone();
        }
    }
    verify_pins(pins)?;
    // Hash exact retained bytes. Re-parsing arbitrary f64 JSON into Value and
    // reserializing it is not a canonical raw-evidence digest contract.
    write(request.output.join("founder-result.json"), &result)?;
    write(request.output.join("founder-comparison.json"), &comparison)?;
    Ok((result, founder))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn founder_only_world_assessment_matches_full_source_and_rejects_tampering_or_restart() {
        struct Fixture(PathBuf);
        impl Drop for Fixture {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let root = Fixture(std::env::temp_dir().join(
            format!("alife-founder-inference-{}-{}",std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
        ));
        fs::create_dir(&root.0).unwrap();
        let source = root.0.join("synthetic-native-source");
        let (config, mut episode) = prepare_care_episode(CpuCareRequest {
            previous: None,
            output: source.clone(),
            seed: 0x94_10,
            decisions: 2,
            temperature: 1.,
            lesson: FoundationTeacherLesson::EatHeldFood,
            food_need_condition: FoundationFoodNeedCondition::default(),
            seed_history: None,
            known_exclusions: false,
            record_diagnostic_replay: false,
            fixture: true,
        })
        .unwrap();
        episode.explicit_deterministic_prior = true;
        run_cpu_continuation(config, &mut episode).unwrap();
        let original = inventory(&source).unwrap();
        let history = root.0.join("history.json");
        let source_pins = [
            "cpu-cycle.json",
            "trained.alife-foundation",
            "actor-checkpoint.json",
            "value-checkpoint.json",
        ]
        .into_iter()
        .map(|name| (name, file_hash(&source.join(name)).unwrap()))
        .collect::<BTreeMap<_, _>>();
        write(
            &history,
            &json!({"schema":1,"kind":"mixed-care-seed-history","complete":true,
            "evidence":"Disposable synthetic fixture only; no real seed history qualified",
            "source_sha256":source_pins,"consumed_seed_ranges":[[0x94_10,0x94_10]],
            "reserved_seed_ranges":[],"assessment_seeds":[0x94_11],"imports":[]}),
        )
        .unwrap();
        let control = root.0.join("full-source-assessment");
        evaluate_cpu_care(CpuCareEvaluationRequest {
            previous: source.clone(),
            output: control.clone(),
            seed_history: history.clone(),
            seed: 0x94_11,
            decisions: 2,
            temperature: 1.,
            lesson: FoundationTeacherLesson::EatHeldFood,
            food_need_condition: FoundationFoodNeedCondition::default(),
            known_exclusions: false,
        })
        .unwrap();
        let request = CpuFounderEvaluationRequest {
            foundation: source.join("trained.alife-foundation"),
            expected_foundation_sha256: file_hash(&source.join("trained.alife-foundation"))
                .unwrap(),
            seed_anchor: source.clone(),
            seed_history: history,
            output: root.0.join("bare-assessment"),
            seed: 0x94_11,
            decisions: 2,
            temperature: 1.,
            lesson: FoundationTeacherLesson::EatHeldFood,
            food_need_condition: FoundationFoodNeedCondition::default(),
            known_exclusions: false,
        };
        let mut wrong = request.clone();
        wrong.expected_foundation_sha256 = "0".repeat(64);
        assert!(evaluate_cpu_founder(wrong).is_err());
        assert!(!request.output.exists());
        let result = evaluate_cpu_founder(request.clone()).unwrap();
        assert_eq!(result["offline_optimizer_constructed"], false);
        assert_eq!(result["critic_constructed"], false);
        assert_eq!(
            fs::read(control.join("care-trace.json")).unwrap(),
            fs::read(request.output.join("care-trace.json")).unwrap()
        );
        assert_eq!(
            read_cpu_private_checkpoint_bytes(&control, u64::MAX).unwrap(),
            read_cpu_private_checkpoint_bytes(&request.output, u64::MAX).unwrap()
        );
        assert_eq!(inventory(&source).unwrap(), original);
        assert_eq!(
            compare_cpu_founder_evaluations(&request.output, &request.output).unwrap()["matched"],
            true
        );
        let before = inventory(&request.output).unwrap();
        assert!(evaluate_cpu_founder(request.clone()).is_err());
        assert_eq!(inventory(&request.output).unwrap(), before);
        let mut full_request = request.clone();
        full_request.output = root.0.join("actual-native-full-control");
        let full = evaluate_cpu_full_founder(full_request.clone()).unwrap();
        assert_eq!(full["full_actor_control"], true);
        assert_eq!(
            verify_cpu_native_founder_parity(&request.output, &full_request.output).unwrap()
                ["parity_verified"],
            true
        );
        assert!(verify_cpu_native_founder_parity(&request.output, &request.output).is_err());
        let proof = full_request.output.join(FULL_PROOF);
        fs::write(proof, b"{}").unwrap();
        assert!(verify_cpu_founder_evaluation(&full_request.output).is_err());
        fs::write(
            request.output.join("care-trace.json"),
            b"tampered disposable trace",
        )
        .unwrap();
        assert!(verify_cpu_founder_evaluation(&request.output).is_err());
        assert_eq!(inventory(&source).unwrap(), original);
    }
}
