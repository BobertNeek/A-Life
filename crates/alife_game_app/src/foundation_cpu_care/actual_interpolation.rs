//! Owner-run embodied checks for already prepared inference-only candidates.
//! Native parents stay immutable. Candidates export no optimizer/critic/handoff.
use super::retained::{run_view_trial, saved_source_phenotype, AssessmentCase, AssessmentView};
use super::*;
mod ablations;
use alife_core::*;
use alife_training::{cpu::retained_interpolation_experiment::*, AdamWConfig, StageTrainableMask};
use alife_world::{with_saved_body_rounding_compatibility, SavedBodyRoundingCompatibilityV1};
use serde::Deserialize;
use std::{fs, io::Read, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateSpec {
    directory: PathBuf,
    fraction: f64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    phase: String,
    pc_current: PathBuf,
    expected_pc_sha256: String,
    cloud_native_root: PathBuf,
    expected_cloud_tree_sha256: String,
    candidates: Vec<CandidateSpec>,
    qualified_controls: Option<PathBuf>,
    output: PathBuf,
    #[serde(default)]
    ablation_variant: Option<String>,
}
struct Model {
    role: &'static str,
    inputs: PhenotypeCompilerInputs,
    asset: FoundationWeightAsset,
    founder: CpuFoundationTrainer,
    value: Option<CpuValueHead>,
    retained: Option<CpuRetainedNeuralInheritance>,
    identity: Value,
    actor_before: Vec<u8>,
    value_before: Option<Vec<u8>>,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec(value)?)?;
    Ok(())
}
fn scope<T>(
    proof: &Option<SavedBodyRoundingCompatibilityV1>,
    f: impl FnOnce() -> Result<T>,
) -> Result<T> {
    if let Some(proof) = proof {
        let (result, _) = with_saved_body_rounding_compatibility(proof.clone(), f)?;
        result
    } else {
        f()
    }
}
fn model_parent(
    role: &'static str,
    handoff: &alife_training::FoundationTrainingHandoffV1,
    retained: Option<CpuRetainedNeuralInheritance>,
) -> Result<Model> {
    let admitted = handoff.clone().admit()?;
    let founder = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation.clone(),
        admitted.actor,
    )?;
    let value = CpuValueHead::from_checkpoint(admitted.value)?;
    Ok(Model {
        role,
        inputs: handoff.compiler_inputs.clone(),
        asset: admitted.foundation,
        actor_before: serde_json::to_vec(&founder.checkpoint())?,
        value_before: Some(serde_json::to_vec(&value.checkpoint())?),
        founder,
        value: Some(value),
        retained,
        identity: json!({"kind":"untouched-native-global-parent-control","policy":handoff.policy_version,"role":role,"historical_freshness":"unknown"}),
    })
}
fn model_candidate(
    spec: &CandidateSpec,
    request: &Request,
    pc_bytes: &[u8],
    proof: &Option<SavedBodyRoundingCompatibilityV1>,
    evaluator_mask: &StageTrainableMask,
) -> Result<Model> {
    let role = match spec.fraction {
        0. => "pc_fraction0",
        0.5 => "interpolation_05",
        1. => "cloud_genetics_pc_overlay",
        _ => return Err("bounded comparison supports only fractions 0, 0.5 and 1".into()),
    };
    let compressed = fs::read(spec.directory.join("experiment.json.zst"))?;
    let reader = zstd::stream::read::Decoder::new(std::io::Cursor::new(&compressed))?
        .take(1024 * 1024 * 1024);
    let experiment: RetainedInterpolationExperiment = serde_json::from_reader(reader)?;
    if experiment.inputs().pc_current != pc_bytes
        || experiment.inputs().expected_pc_current_sha256 != request.expected_pc_sha256
        || experiment.inputs().cloud_fraction != spec.fraction
    {
        return Err("candidate differs from selected PC parent or requested fraction".into());
    }
    match &experiment.inputs().cloud_parent {
        InterpolationCloudParent::NativeCpuArchive { archive }
            if archive.expected_tree_sha256 == request.expected_cloud_tree_sha256
                && archive.tree_sha256()? == request.expected_cloud_tree_sha256 => {}
        _ => return Err("candidate cloud archive differs from the inspected native tree".into()),
    }
    scope(proof, || {
        experiment.validate()?;
        Ok(())
    })?;
    for (name, expected) in [
        (
            "candidate-compiler-inputs.json",
            serde_json::to_vec(experiment.compiler_inputs())?,
        ),
        (
            "candidate-retained-neural.json",
            serde_json::to_vec(experiment.retained())?,
        ),
        (
            "inference-only-receipt.json",
            serde_json::to_vec(experiment.receipt())?,
        ),
    ] {
        if fs::read(spec.directory.join(name))? != expected {
            return Err("candidate export differs from cold-reconstructed experiment".into());
        }
    }
    let inputs = experiment.compiler_inputs().clone();
    let asset = inputs
        .n2048_candidate_asset()
        .ok_or("candidate asset missing")?
        .clone();
    if fs::read(spec.directory.join("candidate.alife-foundation"))? != asset.encode_canonical()? {
        return Err("candidate foundation export differs".into());
    }
    let p = PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048())?;
    // Read-only evaluator storage using the parent's validated coordinate mask.
    // No update API is called; the container is checked unchanged and never
    // exported as candidate optimizer/critic or resumable state.
    let founder = CpuFoundationTrainer::new(
        p.clone(),
        asset.clone(),
        evaluator_mask.clone(),
        AdamWConfig::default(),
    )?;
    Ok(Model {
        role,
        actor_before: serde_json::to_vec(&founder.checkpoint())?,
        value_before: None,
        founder,
        value: None,
        inputs,
        asset,
        retained: Some(experiment.retained().clone()),
        identity: json!({"kind":"inference-only-candidate","role":role,"cloud_fraction":spec.fraction,"receipt":experiment.receipt(),"experiment_sha256":hash(&compressed),"optimizer_and_critic":"absent; evaluator container is internal, frozen and not exported","historical_freshness":"unknown"}),
    })
}

pub fn run_actual_interpolation_assessment(spec_path: &Path) -> Result<Value> {
    let request_bytes = fs::read(spec_path)?;
    let request: Request = serde_json::from_slice(&request_bytes)?;
    if !matches!(
        request.phase.as_str(),
        "controls" | "candidates" | "ablations" | "fresh_battery"
    ) {
        return Err(
            "explicit controls, candidates, ablations or fresh_battery phase required".into(),
        );
    }
    if request.phase == "controls"
        && (request.candidates.len() != 1 || request.candidates[0].fraction != 0.)
        || request.phase == "candidates"
            && (request.candidates.len() != 2
                || request.candidates[0].fraction != 0.5
                || request.candidates[1].fraction != 1.)
        || matches!(request.phase.as_str(), "ablations" | "fresh_battery")
            && !request.candidates.is_empty()
    {
        return Err("unexpected bounded candidate set".into());
    }
    let mut protected = vec![
        request.pc_current.as_path(),
        request.cloud_native_root.as_path(),
        spec_path,
    ];
    protected.extend(request.candidates.iter().map(|c| c.directory.as_path()));
    if let Some(controls) = &request.qualified_controls {
        protected.push(controls.as_path());
    }
    let output = alife_training::resolve_isolated_training_output(&request.output, protected)?;
    if output.exists() {
        return Err("existing assessment output is preserved; do not restart".into());
    }
    let pc_bytes = fs::read(&request.pc_current)?;
    if hash(&pc_bytes) != request.expected_pc_sha256 {
        return Err("PC original differs from explicit SHA256 pin".into());
    }
    let (pc, proof) = match crate::read_current_training_checkpoint(&request.pc_current) {
        Ok(source) => (source, None),
        Err(_) => {
            let p = crate::propose_current_body_rounding_compatibility(
                &request.pc_current,
                &request.expected_pc_sha256,
            )?;
            (
                crate::read_current_training_checkpoint_with_body_rounding(
                    &request.pc_current,
                    &request.expected_pc_sha256,
                    &p.proof,
                )?,
                Some(p.proof),
            )
        }
    };
    let cloud_binding = inspect_native_cpu_archive(&request.cloud_native_root)?;
    if cloud_binding.tree_sha256 != request.expected_cloud_tree_sha256 {
        return Err("cloud native tree differs from inspected pin".into());
    }
    crate::validate_cpu_care_output(&request.cloud_native_root)?;
    let cloud = read_cpu_founder_source(&request.cloud_native_root)?;
    let cloud_lineage =
        alife_training::native_receipt::Lineage::load(&cloud.receipt, &request.cloud_native_root)?;
    let cloud_seed_evidence: Value = serde_json::from_slice(&fs::read(
        request.cloud_native_root.join("care-seed-admission.json"),
    )?)?;
    let seeds = if request.phase == "fresh_battery" {
        202611185878..=202611185897
    } else {
        202611185878..=202611185879
    };
    for seed in seeds.clone() {
        if pc.seeds.contains(seed) {
            return Err("assessment seed overlaps known PC training history".into());
        }
        for evidence in [&cloud.receipt, &cloud_seed_evidence] {
            crate::foundation_training_cycle::verify_native_cpu_care_assessment_exclusions(
                evidence, seed,
            )?;
        }
        for frame in cloud_lineage.frames() {
            crate::foundation_training_cycle::verify_native_cpu_care_assessment_exclusions(
                &frame?, seed,
            )?;
        }
    }
    if cloud.receipt["founder_seed_base"] != pc.founder_seed_base {
        return Err("parent body seeds differ".into());
    }
    let executable_sha = hash(&fs::read(std::env::current_exe()?)?);
    if request.phase == "fresh_battery" {
        ablations::qualify_battery_request(&request, &executable_sha)?;
    }
    if request.phase == "candidates" {
        let controls = request
            .qualified_controls
            .as_ref()
            .ok_or("candidate dispatch requires qualified fraction-zero controls")?;
        let gate: Value =
            serde_json::from_slice(&fs::read(controls.join("fraction-zero-parity.json"))?)?;
        if gate["passed"] != true
            || gate["pc_sha256"] != request.expected_pc_sha256
            || gate["cloud_tree_sha256"] != request.expected_cloud_tree_sha256
            || gate["executable_sha256"] != executable_sha
        {
            return Err(
                "fraction-zero control gate differs from actual sources or this executable".into(),
            );
        }
        for (name, expected) in gate["trace_sha256"]
            .as_object()
            .ok_or("control trace hashes missing")?
        {
            if name.contains("..")
                || name.starts_with('/')
                || hash(&fs::read(controls.join(name))?)
                    != expected.as_str().ok_or("invalid control trace hash")?
            {
                return Err("qualified control trace changed".into());
            }
        }
    }
    let saved_body = saved_source_phenotype(&pc)?;
    let mut models = Vec::new();
    if request.phase == "controls" {
        let (retained, _) = scope(&proof, || {
            Ok(derive_retained_projection_from_current(
                &pc_bytes,
                &request.expected_pc_sha256,
            )?)
        })?;
        models.push(model_parent("pc_retained", &pc.handoff, Some(retained))?);
        models.push(model_candidate(
            &request.candidates[0],
            &request,
            &pc_bytes,
            &proof,
            &pc.handoff.actor.stage_mask,
        )?);
        models.push(model_parent("pc_native_fresh", &pc.handoff, None)?);
        models.push(model_parent("cloud_native_fresh", &cloud.handoff, None)?);
    } else if matches!(request.phase.as_str(), "ablations" | "fresh_battery") {
        let (retained, _) = scope(&proof, || {
            Ok(derive_retained_projection_from_current(
                &pc_bytes,
                &request.expected_pc_sha256,
            )?)
        })?;
        models = ablations::models(&request, &pc.handoff, &cloud.handoff, &retained)?;
    } else {
        for candidate in &request.candidates {
            models.push(model_candidate(
                candidate,
                &request,
                &pc_bytes,
                &proof,
                &pc.handoff.actor.stage_mask,
            )?);
        }
    }
    fs::create_dir_all(&output)?;
    fs::write(output.join("request.json"), &request_bytes)?;
    write(
        output.join("native-admission.json"),
        &json!({"pc_policy":pc.handoff.policy_version,"pc_actor_adam":pc.handoff.actor.optimizer_step,"pc_value_adam":pc.handoff.value.optimizer_step,"pc_sha256":request.expected_pc_sha256,"cloud_binding":cloud_binding,"body_seed":pc.founder_seed_base,"body_rounding_proof":proof,"executable_sha256":executable_sha,"teacher_and_prior":false,"temperature":1,"scored_decisions":128,"bootstrap":1,"waking_plasticity":"ordinary; global heads frozen","freshness":"unknown","candidate_training":"not resumable; no candidate actor/critic/Current export"}),
    )?;
    let mut results = Vec::new();
    let mut parity = true;
    let mut trace_pins = BTreeMap::new();
    let conditions = if request.phase == "fresh_battery" {
        vec![
            (
                "held",
                FoundationTeacherLesson::EatHeldFood,
                FoundationFoodNeedCondition::NewbornV1,
            ),
            (
                "reachable",
                FoundationTeacherLesson::EatReachableFood,
                FoundationFoodNeedCondition::NewbornV1,
            ),
            (
                "feeding_newborn",
                FoundationTeacherLesson::Feeding,
                FoundationFoodNeedCondition::NewbornV1,
            ),
            (
                "feeding_aged",
                FoundationTeacherLesson::Feeding,
                FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
            ),
            (
                "navigation",
                FoundationTeacherLesson::ObstacleNavigation,
                FoundationFoodNeedCondition::NewbornV1,
            ),
            (
                "relocated_aged",
                FoundationTeacherLesson::RelocatedFood,
                FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
            ),
        ]
    } else {
        vec![
            (
                "held",
                FoundationTeacherLesson::EatHeldFood,
                FoundationFoodNeedCondition::NewbornV1,
            ),
            (
                "feeding_aged",
                FoundationTeacherLesson::Feeding,
                FoundationFoodNeedCondition::OrdinaryHealthyAgingV1,
            ),
        ]
    };
    for (condition_label, lesson, condition) in conditions {
        for seed in seeds.clone() {
            for model in &mut models {
                let trial = output.join(format!("{condition_label}-{seed}-{}", model.role));
                let mut result = match run_view_trial(
                    &AssessmentView {
                        asset: &model.asset,
                        compiler_inputs: &model.inputs,
                        body_seed: pc.founder_seed_base,
                        saved_body: &saved_body,
                        identity: model.identity.clone(),
                        archive_heads: model.value.is_some(),
                    },
                    model.retained.as_ref(),
                    &mut model.founder,
                    model.value.as_ref(),
                    &trial,
                    &AssessmentCase {
                        seed,
                        lesson,
                        condition,
                        scored_limit: 128,
                    },
                ) {
                    Ok(result) => result,
                    Err(error) => {
                        json!({"complete":false,"lesson_completed":false,"error":error.to_string(),"preserve_prefix":true})
                    }
                };
                result["role"] = json!(model.role);
                result["condition"] = json!(condition_label);
                result["seed"] = json!(seed);
                println!(
                    "{}",
                    json!({"role":model.role,"condition":condition_label,"seed":seed,"complete":result["complete"],"success":result["lesson_completed"],"error":result["error"]})
                );
                results.push(result);
                write(output.join("results.json"), &results)?;
            }
            if request.phase == "controls" {
                let mut traces = Vec::new();
                for role in ["pc_retained", "pc_fraction0"] {
                    let relative = format!("{condition_label}-{seed}-{role}/care-trace.json");
                    match fs::read(output.join(&relative)) {
                        Ok(bytes) => {
                            trace_pins.insert(relative, hash(&bytes));
                            traces.push(serde_json::from_slice::<Value>(&bytes)?);
                        }
                        Err(_) => parity = false,
                    }
                }
                parity &= traces.len() == 2 && traces[0]["rows"] == traces[1]["rows"];
            }
        }
    }
    for model in &models {
        if serde_json::to_vec(&model.founder.checkpoint())? != model.actor_before
            || model
                .value
                .as_ref()
                .map(|v| serde_json::to_vec(&v.checkpoint()))
                .transpose()?
                != model.value_before
        {
            return Err(
                "assessment modified a frozen global evaluator or actual parent head".into(),
            );
        }
    }
    if fs::read(spec_path)? != request_bytes
        || fs::read(&request.pc_current)? != pc_bytes
        || inspect_native_cpu_archive(&request.cloud_native_root)?.tree_sha256
            != request.expected_cloud_tree_sha256
    {
        return Err("actual parent bytes changed during assessment".into());
    }
    let result = json!({"pc_sha256":request.expected_pc_sha256,"cloud_tree_sha256":request.expected_cloud_tree_sha256,"executable_sha256":executable_sha,"phase":request.phase,"trials":results,"parents_unchanged":true,"global_optimizer_updates":0,"candidate_adoption":"unresolved; no promotion","hardware_gpu_parity":false,"historical_freshness":"unknown"});
    write(output.join("comparison.json"), &result)?;
    if request.phase == "controls" {
        parity &= results
            .iter()
            .filter(|r| matches!(r["role"].as_str(), Some("pc_retained" | "pc_fraction0")))
            .all(|r| r["complete"] == true);
        write(
            output.join("fraction-zero-parity.json"),
            &json!({"passed":parity,"pc_sha256":request.expected_pc_sha256,"cloud_tree_sha256":request.expected_cloud_tree_sha256,"executable_sha256":executable_sha,"trace_sha256":trace_pins,"comparison":"exact complete normalized JSON trace rows; all commands, bundles, input frame identities, sampling and measured outcomes"}),
        )?;
        if !parity {
            return Err(
                "fraction-zero embodied trace differs; mixed-candidate dispatch remains blocked"
                    .into(),
            );
        }
    }
    Ok(result)
}
