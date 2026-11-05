//! Matched, full-horizon assessment of the saved fork before/after a segment
//! and both actual global parents. This never updates shared Adam state.
use super::super::retained::{run_view_trial, AssessmentCase, AssessmentView};
use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RetentionRequest {
    baseline: PreviousFork,
    candidate: PreviousFork,
    expected_experiment_sha256: String,
    pc_current: PathBuf,
    expected_pc_sha256: String,
    cloud_native_root: PathBuf,
    expected_cloud_tree_sha256: String,
    assessment_reservation: PathBuf,
    expected_reservation_sha256: String,
    seed_first: u64,
    seed_count: usize,
    output: PathBuf,
}
struct Model {
    role: &'static str,
    trainer: CpuFoundationTrainer,
    value: CpuValueHead,
    retained: Option<CpuRetainedNeuralInheritance>,
    inputs: PhenotypeCompilerInputs,
    asset: FoundationWeightAsset,
    before_actor: Vec<u8>,
    before_value: Vec<u8>,
    version: u64,
}
fn fork_model(role: &'static str, p: &PreviousFork) -> Result<Model> {
    let (trainer, value, retained, inputs, version, _) = previous_state(p)?;
    let asset = inputs
        .n2048_candidate_asset()
        .ok_or("fork assessment asset missing")?
        .clone();
    Ok(Model {
        role,
        before_actor: serde_json::to_vec(&trainer.checkpoint())?,
        before_value: serde_json::to_vec(&value.checkpoint())?,
        trainer,
        value,
        retained: Some(retained),
        inputs,
        asset,
        version,
    })
}
fn parent_model(role: &'static str, h: &FoundationTrainingHandoffV1) -> Result<Model> {
    let a = h.clone().admit()?;
    let trainer =
        CpuFoundationTrainer::from_checkpoint(a.phenotype, a.foundation.clone(), a.actor)?;
    let value = CpuValueHead::from_checkpoint(a.value)?;
    Ok(Model {
        role,
        before_actor: serde_json::to_vec(&trainer.checkpoint())?,
        before_value: serde_json::to_vec(&value.checkpoint())?,
        trainer,
        value,
        retained: None,
        inputs: h.compiler_inputs.clone(),
        asset: a.foundation,
        version: h.policy_version,
    })
}
fn exclude_dispatched_fork_seeds(p: &PreviousFork, seed: u64, depth: usize) -> Result<()> {
    if depth > 32 {
        return Err("assessment ancestry depth exceeded".into());
    }
    let m: ForkManifest = serde_json::from_slice(&pinned(
        p.directory.join("retained-fork.json"),
        &p.manifest_sha256,
    )?)?;
    if seed == m.training_seed || seed == m.recovery_seed {
        return Err("held-out seed was used by this fork".into());
    }
    if let Some(previous) = m.previous_fork {
        exclude_dispatched_fork_seeds(&previous, seed, depth + 1)?;
    }
    Ok(())
}
pub fn run_retained_fork_retention(spec: &Path) -> Result<Value> {
    let req: RetentionRequest = read(spec)?;
    if req.seed_count == 0 || req.seed_count > 20 {
        return Err("retention requires 1..=20 predeclared matched seeds".into());
    }
    let last = req
        .seed_first
        .checked_add(req.seed_count as u64 - 1)
        .ok_or("assessment seed overflow")?;
    let reservation: Value = serde_json::from_slice(&pinned(
        &req.assessment_reservation,
        &req.expected_reservation_sha256,
    )?)?;
    if reservation["purpose"] != "assessment_only"
        || reservation["training_forbidden"] != true
        || reservation["seed_range"] != json!([req.seed_first, last])
    {
        return Err("assessment reservation differs from planned holdouts".into());
    }
    let output = resolve_isolated_training_output(
        &req.output,
        [
            spec,
            req.baseline.directory.as_path(),
            req.candidate.directory.as_path(),
            req.pc_current.as_path(),
            req.cloud_native_root.as_path(),
            req.assessment_reservation.as_path(),
        ],
    )?;
    if output.exists() {
        return Err("existing assessment preserved; no implicit restart".into());
    }
    require_space(
        output.parent().ok_or("assessment output parent missing")?,
        1024u64.pow(3),
    )?;
    for p in [&req.baseline, &req.candidate] {
        verify_pc_donor_retained_training_fork(
            &p.directory,
            &p.manifest_sha256,
            &req.expected_experiment_sha256,
        )?;
    }
    let pc_bytes = pinned(&req.pc_current, &req.expected_pc_sha256)?;
    let pc = match crate::read_current_training_checkpoint(&req.pc_current) {
        Ok(p) => p,
        Err(_) => {
            let proof = crate::propose_current_body_rounding_compatibility(
                &req.pc_current,
                &req.expected_pc_sha256,
            )?;
            crate::read_current_training_checkpoint_with_body_rounding(
                &req.pc_current,
                &req.expected_pc_sha256,
                &proof.proof,
            )?
        }
    };
    if inspect_native_cpu_archive(&req.cloud_native_root)?.tree_sha256
        != req.expected_cloud_tree_sha256
    {
        return Err("assessment cloud source differs".into());
    }
    let cloud = read_cpu_founder_source(&req.cloud_native_root)?;
    let lineage = native_receipt::Lineage::load(&cloud.receipt, &req.cloud_native_root)?;
    let mut evidence = vec![
        cloud.receipt.clone(),
        native_receipt::read(&req.cloud_native_root.join("care-seed-admission.json"))?.1,
    ];
    for frame in lineage.frames() {
        evidence.push(frame?);
    }
    for seed in req.seed_first..=last {
        if pc.seeds.contains(seed) {
            return Err("assessment overlaps PC exclusions".into());
        }
        for e in &evidence {
            crate::foundation_training_cycle::verify_native_cpu_care_training_exclusions(e, seed)?;
        }
        for p in [&req.baseline, &req.candidate] {
            exclude_dispatched_fork_seeds(p, seed, 0)?;
        }
    }
    let saved = retained::saved_source_phenotype(&pc)?;
    let mut models = vec![
        fork_model("pre_segment_one_update_fork", &req.baseline)?,
        fork_model("post_segment_fork", &req.candidate)?,
        parent_model("PC11891_native_fresh", &pc.handoff)?,
        parent_model("cloud10468_native_fresh", &cloud.handoff)?,
    ];
    fs::create_dir(&output)?;
    fs::write(output.join("request.json"), fs::read(spec)?)?;
    write(
        output.join("protocol.json"),
        &json!({"schema":1,"n":req.seed_count,"seed_range":[req.seed_first,last],"held_out":"reserved before segment, no training or selection on these seeds; ancestral freshness unknown","pre_update_definition":"preserved verified one-update fork before this segment, not the earlier zero-update inference candidate","parents":"actual PC11891 and cloud10468 global heads with native fresh neural initialization; full original archives preserved","fork_initialization":"each trial clones that fork's retained topology/predictor/current somatic projection; no repeated bank reset","dose":128,"bootstrap":1,"temperature":1,"teacher":false,"semantic_prior":false,"global_updates":0,"waking_and_sleep":"ordinary","health_injury_censoring":false,"storage_floor_bytes":8*1024u64.pow(3)}),
    )?;
    let mut results = Vec::new();
    let mut blocked = None;
    'seeds: for seed in req.seed_first..=last {
        for (condition, lesson, need) in [
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
        ] {
            for m in &mut models {
                if let Err(e) = require_space(&output, 256 * 1024 * 1024) {
                    blocked = Some(e.to_string());
                    break 'seeds;
                }
                let case_output = output.join(format!("{condition}-{seed}-{}", m.role));
                let mut result = match run_view_trial(
                    &AssessmentView {
                        asset: &m.asset,
                        compiler_inputs: &m.inputs,
                        body_seed: pc.founder_seed_base,
                        saved_body: &saved,
                        identity: json!({"role":m.role,"policy":m.version,"kind":"frozen-native-head-retention-control"}),
                        archive_heads: true,
                    },
                    m.retained.as_ref(),
                    &mut m.trainer,
                    Some(&m.value),
                    &case_output,
                    &AssessmentCase {
                        seed,
                        lesson,
                        condition: need,
                        scored_limit: 128,
                        record_physiology: true,
                    },
                ) {
                    Ok(r) => r,
                    Err(e) => {
                        json!({"error":e.to_string(),"complete":false,"lesson_completed":false,"alive":null,"dose_complete":false})
                    }
                };
                result["role"] = json!(m.role);
                result["condition"] = json!(condition);
                result["seed"] = json!(seed);
                results.push(result);
                write(output.join("results.json"), &results)?;
            }
        }
    }
    for m in &models {
        if serde_json::to_vec(&m.trainer.checkpoint())? != m.before_actor
            || serde_json::to_vec(&m.value.checkpoint())? != m.before_value
        {
            return Err("retention assessment changed a full shared optimizer".into());
        }
    }
    for p in [&req.baseline, &req.candidate] {
        let manifest: ForkManifest = serde_json::from_slice(&pinned(
            p.directory.join("retained-fork.json"),
            &p.manifest_sha256,
        )?)?;
        if inventory(&p.directory)? != manifest.artifacts {
            return Err("assessment source fork changed during trials".into());
        }
    }
    if fs::read(&req.pc_current)? != pc_bytes
        || inspect_native_cpu_archive(&req.cloud_native_root)?.tree_sha256
            != req.expected_cloud_tree_sha256
    {
        return Err("assessment changed original parents".into());
    }
    let report = json!({"completed_cases":results.len(),"requested_cases":req.seed_count*6*4,"complete":results.len()==req.seed_count*6*4,"blocker":blocked,"full_shared_optimizers_unchanged":true,"original_parents_unchanged":true,"source_forks_unchanged":true,"hardware_GPU_parity":false,"promoted":false});
    write(output.join("completion.json"), &report)?;
    Ok(report)
}
