//! Disposable zero-update recovery and optimizer-initialization probes.
//! Never publishes a native training handoff or claims parameter continuation.
use super::*;
use alife_training::{
    cpu::retained_interpolation_experiment::*, AdamWConfig, FoundationTrainerCheckpoint,
    PpoValueHeadCheckpoint,
};
use serde::Deserialize;
use std::path::PathBuf;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    prepared: PathBuf,
    initial_trial: PathBuf,
    pc_current: PathBuf,
    expected_pc_sha256: String,
    cloud_native_root: PathBuf,
    expected_cloud_tree_sha256: String,
    output: PathBuf,
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn maybe_scope<T>(
    proof: &Option<SavedBodyRoundingCompatibilityV1>,
    f: impl FnOnce() -> Result<T>,
) -> Result<T> {
    if let Some(p) = proof {
        scope(p, f)
    } else {
        f()
    }
}
fn optimizer_hash(c: &FoundationTrainerCheckpoint) -> Result<String> {
    Ok(hash(&serde_json::to_vec(&(
        c.optimizer_step,
        c.config,
        &c.stage_mask,
        &c.first_moment,
        &c.second_moment,
        &c.update_ages,
    ))?))
}
pub(super) fn run(spec: &Path) -> Result<()> {
    let request_bytes = fs::read(spec)?;
    let req: Request = serde_json::from_slice(&request_bytes)?;
    let output = alife_training::resolve_isolated_training_output(
        &req.output,
        [
            req.prepared.as_path(),
            req.initial_trial.as_path(),
            req.pc_current.as_path(),
            req.cloud_native_root.as_path(),
            spec,
        ],
    )?;
    if output.exists() {
        return Err("existing recovery probe is preserved; choose a new output".into());
    }
    let pc_bytes = fs::read(&req.pc_current)?;
    if hash(&pc_bytes) != req.expected_pc_sha256 {
        return Err("PC source pin differs".into());
    }
    let (pc, source_proof) = match read_current_training_checkpoint(&req.pc_current) {
        Ok(s) => (s, None),
        Err(_) => {
            let p = propose_current_body_rounding_compatibility(
                &req.pc_current,
                &req.expected_pc_sha256,
            )?;
            (
                read_current_training_checkpoint_with_body_rounding(
                    &req.pc_current,
                    &req.expected_pc_sha256,
                    &p.proof,
                )?,
                Some(p.proof),
            )
        }
    };
    let binding = inspect_native_cpu_archive(&req.cloud_native_root)?;
    if binding.tree_sha256 != req.expected_cloud_tree_sha256 {
        return Err("cloud native pin differs".into());
    }
    let cloud = read_cpu_founder_source(&req.cloud_native_root)?;
    let compressed = fs::read(req.prepared.join("experiment.json.zst"))?;
    let experiment: RetainedInterpolationExperiment = serde_json::from_reader(
        zstd::stream::read::Decoder::new(std::io::Cursor::new(&compressed))?,
    )?;
    if experiment.inputs().cloud_fraction != 0.25
        || experiment.inputs().pc_current != pc_bytes
        || experiment.inputs().expected_pc_current_sha256 != req.expected_pc_sha256
    {
        return Err("quarter experiment source mismatch".into());
    }
    match &experiment.inputs().cloud_parent {
        InterpolationCloudParent::NativeCpuArchive { archive }
            if archive.expected_tree_sha256 == req.expected_cloud_tree_sha256
                && archive.tree_sha256()? == req.expected_cloud_tree_sha256 => {}
        _ => return Err("experiment cloud source mismatch".into()),
    }
    maybe_scope(&source_proof, || {
        experiment.validate()?;
        Ok(())
    })?;
    let inputs = experiment.compiler_inputs().clone();
    let asset = inputs
        .n2048_candidate_asset()
        .ok_or("candidate asset missing")?
        .clone();
    let phenotype = PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048())?;
    if fs::read(req.prepared.join("candidate.alife-foundation"))? != asset.encode_canonical()?
        || fs::read(req.initial_trial.join("collection-policy.alife-foundation"))?
            != asset.encode_canonical()?
    {
        return Err("measured genetic asset differs from cold-recovered experiment".into());
    }
    let initial_bytes = fs::read(req.initial_trial.join("initial-checkpoint.json"))?;
    let result: Value = serde_json::from_slice(&fs::read(
        req.initial_trial.join("retained-assessment.json"),
    )?)?;
    let proof: Option<SavedBodyRoundingCompatibilityV1> =
        serde_json::from_value(result["body_compatibility"]["explicit_proof"].clone())?;
    let initial: CpuCanonicalCheckpoint =
        maybe_scope(&proof, || Ok(serde_json::from_slice(&initial_bytes)?))?;
    let inherited = initial.residents[0]
        .inherited_neural_source
        .as_ref()
        .ok_or("measured initial inheritance missing")?;
    if serde_json::to_vec(&inherited.topology)?
        != serde_json::to_vec(&experiment.retained().topology)?
        || serde_json::to_vec(&inherited.predictor)?
            != serde_json::to_vec(&experiment.retained().predictor)?
        || [&inherited.fast, &inherited.lifetime]
            .iter()
            .any(|b| b.bank_0_bits.iter().chain(&b.bank_1_bits).any(|&v| v != 0))
    {
        return Err(
            "measured start does not retain topology/predictor with explicit zero banks".into(),
        );
    }
    inherited.canonical_to_live(&phenotype)?;
    let trace: Value =
        serde_json::from_slice(&fs::read(req.initial_trial.join("care-trace.json"))?)?;
    let expected: alife_training::PpoJointAction =
        serde_json::from_value(trace["rows"][0]["ppo_action"].clone())?;
    let original_pc_actor = serde_json::to_vec(&pc.handoff.actor)?;
    let original_pc_value = serde_json::to_vec(&pc.handoff.value)?;
    let original_cloud_actor = serde_json::to_vec(&cloud.handoff.actor)?;
    let original_cloud_value = serde_json::to_vec(&cloud.handoff.value)?;
    fs::create_dir(&output)?;
    fs::write(output.join("request.json"), &request_bytes)?;
    fs::write(output.join("measured-initial-copy.json"), &initial_bytes)?;
    fs::write(
        output.join("candidate.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    write(output.join("candidate-compiler-inputs.json"), &inputs)?;
    write(
        output.join("measured-reset-retained-neural.json"),
        inherited,
    )?;
    let mut rows = Vec::new();
    let mut reference_after = None;
    for mode in ["fresh", "pc_donor", "cloud_donor"] {
        let donor = match mode {
            "pc_donor" => Some(&pc.handoff),
            "cloud_donor" => Some(&cloud.handoff),
            _ => None,
        };
        let fresh = CpuFoundationTrainer::new(
            phenotype.clone(),
            asset.clone(),
            pc.handoff.actor.stage_mask.clone(),
            AdamWConfig::default(),
        )?;
        let fresh_checkpoint = fresh.checkpoint();
        let (mut founder, value, exact_route_rejected) = if let Some(d) = donor {
            let admitted = d.clone().admit()?;
            let mut original = CpuFoundationTrainer::from_checkpoint(
                admitted.phenotype,
                admitted.foundation,
                admitted.actor,
            )?;
            let before = serde_json::to_vec(&original.checkpoint())?;
            let rejected = original
                .rebind_for_next_cohort(phenotype.clone(), asset.clone())
                .is_err();
            if !rejected || before != serde_json::to_vec(&original.checkpoint())? {
                return Err(
                    "changed candidate unexpectedly accepted as exact donor continuation".into(),
                );
            }
            let mut fork = d.actor.clone();
            fork.phenotype_hash = phenotype.phenotype_hash();
            fork.source_foundation_digest = asset.digest();
            fork.weights = fresh_checkpoint.weights.clone();
            let f = CpuFoundationTrainer::from_checkpoint(phenotype.clone(), asset.clone(), fork)?;
            if optimizer_hash(&f.checkpoint())? != optimizer_hash(&d.actor)? {
                return Err("explicit donor initialization changed optimizer bits/ages".into());
            }
            (
                f,
                CpuValueHead::from_checkpoint(d.value.clone())?,
                Some(rejected),
            )
        } else {
            (
                fresh,
                CpuValueHead::from_checkpoint(PpoValueHeadCheckpoint {
                    feature_count: phenotype.neuron_count(),
                    optimizer_step: 0,
                    parameters: vec![0.; (phenotype.neuron_count() as usize + 1) * 3],
                    last_updated_policy_version: None,
                })?,
                None,
            )
        };
        let before_actor = serde_json::to_vec(&founder.checkpoint())?;
        let before_value = serde_json::to_vec(&value.checkpoint())?;
        let world = maybe_scope(&proof, || Ok(initial.world.restore_headless_world()?))?;
        let record = world
            .organism_registry()
            .iter()
            .next()
            .ok_or("no measured body")?;
        let genome = &record.phenotype().brain_genome;
        let development = foundation_construction_development(
            genome,
            &BrainCapacityClass::n2048(),
            &record.phenotype().development_state_at(Tick::ZERO)?,
        )?;
        let (body_p, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
            genome.clone(),
            development,
            asset.clone(),
        )?;
        founder.bind_equivalent_assessment_body(&body_p)?;
        let mut host = maybe_scope(&proof, || {
            Ok(CpuCanonicalHost::from_checkpoint(
                initial.clone(),
                &founder,
            )?)
        })?;
        let roundtrip = host.checkpoint(&founder)?;
        if serde_json::to_vec(&roundtrip)? != serde_json::to_vec(&initial)? {
            return Err("cold initial checkpoint roundtrip differs".into());
        }
        let mut decision = None;
        for _ in 0..256 {
            let step = host.step(&founder, 1.)?;
            if let Some(d) = step.decisions.into_iter().next() {
                decision = Some(d);
                break;
            }
        }
        let decision = decision.ok_or("one bounded recovered decision unavailable")?;
        if serde_json::to_vec(&decision.action)? != serde_json::to_vec(&expected)? {
            return Err("recovered initial action differs from measured candidate".into());
        }
        let after = host.checkpoint(&founder)?;
        let encoded_after = serde_json::to_vec(&after)?;
        if let Some(reference) = &reference_after {
            if reference != &encoded_after {
                return Err(
                    "optimizer initialization altered inference/world outcome without updates"
                        .into(),
                );
            }
        } else {
            reference_after = Some(encoded_after.clone());
        }
        if before_actor != serde_json::to_vec(&founder.checkpoint())?
            || before_value != serde_json::to_vec(&value.checkpoint())?
        {
            return Err("zero-update probe modified global optimizer/critic".into());
        }
        let mode_dir = output.join(mode);
        fs::create_dir(&mode_dir)?;
        write_cpu_private_checkpoint(&mode_dir, &after)?;
        rows.push(json!({"mode":mode,"native_constructor_accepted":true,"actor_optimizer_step":founder.optimizer_step(),"critic_optimizer_step":value.checkpoint().optimizer_step,"global_updates":0,"optimizer_sha256":optimizer_hash(&founder.checkpoint())?,"critic_sha256":hash(&before_value),"exact_donor_continuation_rejected":exact_route_rejected,"candidate_weights_and_original_adam_coordinates":phenotype.synapses().len(),"retained_live_slots":inherited.topology.total_synapse_count,"cold_roundtrip_exact":true,"one_recovered_action_matches_measured_case":true,"post_step_private_state_sha256":hash(&encoded_after),"training_status":"zero-update disposable initialization probe only; not an admitted native training frontier"}));
    }
    if fs::read(&req.pc_current)? != pc_bytes
        || inspect_native_cpu_archive(&req.cloud_native_root)?.tree_sha256
            != req.expected_cloud_tree_sha256
        || serde_json::to_vec(&pc.handoff.actor)? != original_pc_actor
        || serde_json::to_vec(&pc.handoff.value)? != original_pc_value
        || serde_json::to_vec(&cloud.handoff.actor)? != original_cloud_actor
        || serde_json::to_vec(&cloud.handoff.value)? != original_cloud_value
        || fs::read(req.initial_trial.join("initial-checkpoint.json"))? != initial_bytes
        || fs::read(req.prepared.join("experiment.json.zst"))? != compressed
    {
        return Err("recovery probe changed a preserved input".into());
    }
    let report = json!({"passed":true,"candidate_cloud_fraction":0.25,"experiment_sha256":hash(&compressed),"initial_snapshot_sha256":hash(&initial_bytes),"candidate_genetic_asset_sha256":hash(&asset.encode_canonical()?),"probes":rows,"parents_and_all_original_optimizer_histories_unchanged":true,"global_optimizer_updates":0,"no_candidate_training_handoff_or_merged_Adam_export":true,"source_assessment_seed":initial.world.deterministic_seed,"do_not_train_on_this_snapshot":"assessment seed; use only recovery/debugging. Future training must construct a fresh body on separately allocated non-assessment seeds.","remaining_training_integration":"low-level fresh/explicit-donor constructors available; shipped exact-continuation CLI rejects changed genetic weights. An explicit fork importer/provenance path and retained-layout rebind/export integration are required before production continuation."});
    write(output.join("recovery-probe.json"), &report)?;
    println!("{}", report);
    Ok(())
}
