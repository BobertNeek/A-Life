//! Inference-only 50/50 genetics after an explicit trial-start somatic reset.
//! The prepared experiment and both complete original parents stay unchanged.
use super::*;
const ROLES: [&str; 4] = [
    "interpolation_05_reset_somatic",
    "reset_somatic",
    "pc_native_fresh",
    "cloud_native_fresh",
];
pub(super) fn reset(model: &mut Model, role: &'static str) -> Result<()> {
    let source = model
        .retained
        .as_ref()
        .ok_or("reset lacks a retained projection")?;
    let projection = ablations::modify("reset_somatic", source, model.founder.phenotype())?;
    model.identity["derived_initial_state"] = json!({"operation":"zero both lifetime and fast arrays on a disposable clone at trial start; ordinary subsequent plasticity enabled","source_projection_sha256":hash(&serde_json::to_vec(source)?),"reset_projection_sha256":hash(&serde_json::to_vec(&projection)?),"bank_selectors_and_generations":"preserved","topology_dendrites_predictor":"unchanged","global_optimizer_updates":0,"parent_Adam_histories":"unchanged original archives; no candidate optimizer/critic export or merged-Adam claim"});
    model.identity["role"] = json!(role);
    model.role = role;
    model.retained = Some(projection);
    Ok(())
}
pub(super) fn models(
    request: &Request,
    pc: &alife_training::CurrentTrainingCheckpointV1,
    cloud: &alife_training::FoundationTrainingHandoffV1,
    pc_bytes: &[u8],
    proof: &Option<SavedBodyRoundingCompatibilityV1>,
) -> Result<Vec<Model>> {
    let pilot = request.phase == "merge_reset_pilot";
    if pilot && request.ablation_variant.is_some()
        || !pilot
            && !ROLES
                .iter()
                .any(|role| request.ablation_variant.as_deref() == Some(role))
    {
        return Err(
            "merge reset requires an explicit supported battery arm or an unpartitioned pilot"
                .into(),
        );
    }
    let mut models = Vec::new();
    if pilot {
        let mut m = model_candidate(
            &request.candidates[0],
            request,
            pc_bytes,
            proof,
            &pc.handoff.actor.stage_mask,
        )?;
        reset(&mut m, "fraction0_reset_somatic")?;
        models.push(m);
    }
    for role in ROLES {
        if !pilot && request.ablation_variant.as_deref() != Some(role) {
            continue;
        }
        let model = match role {
            "interpolation_05_reset_somatic" => {
                let mut m = model_candidate(
                    &request.candidates[1],
                    request,
                    pc_bytes,
                    proof,
                    &pc.handoff.actor.stage_mask,
                )?;
                reset(&mut m, role)?;
                m
            }
            "reset_somatic" => {
                let (projection, _) = scope(proof, || {
                    Ok(derive_retained_projection_from_current(
                        pc_bytes,
                        &request.expected_pc_sha256,
                    )?)
                })?;
                let mut m = model_parent(role, &pc.handoff, Some(projection))?;
                reset(&mut m, role)?;
                m
            }
            "pc_native_fresh" => model_parent(role, &pc.handoff, None)?,
            "cloud_native_fresh" => model_parent(role, cloud, None)?,
            _ => return Err("unsupported merge control".into()),
        };
        models.push(model);
    }
    Ok(models)
}
fn experiment_pins(request: &Request) -> Result<Value> {
    Ok(json!(request
        .candidates
        .iter()
        .map(|c| Ok((
            c.fraction,
            hash(&fs::read(c.directory.join("experiment.json.zst"))?)
        )))
        .collect::<Result<Vec<_>>>()?))
}
pub(super) fn finish_pilot(
    request: &Request,
    output: &Path,
    exe: &str,
    results: &[Value],
) -> Result<()> {
    let previous = request
        .previous_assessment
        .as_ref()
        .ok_or("pilot requires the three completed controls")?;
    let native: Value = serde_json::from_slice(&fs::read(previous.join("native-admission.json"))?)?;
    let audit: Value =
        serde_json::from_slice(&fs::read(previous.join("independent-final-audit.json"))?)?;
    if native["pc_sha256"] != request.expected_pc_sha256
        || native["cloud_binding"]["tree_sha256"] != request.expected_cloud_tree_sha256
        || audit["passed"] != true
    {
        return Err("prior assessment does not bind the selected actual parents".into());
    }
    let mut parity = true;
    let mut comparisons = Vec::new();
    for condition in ["held", "feeding_newborn", "feeding_aged"] {
        for seed in 202611185878u64..=202611185879 {
            let base =
                fs::read(output.join(format!("{condition}-{seed}-reset_somatic/care-trace.json")))?;
            let fraction0 = fs::read(output.join(format!(
                "{condition}-{seed}-fraction0_reset_somatic/care-trace.json"
            )))?;
            parity &= base == fraction0;
            comparisons.push(json!({"kind":"fraction-zero-reset-vs-PC-reset","condition":condition,"seed":seed,"byte_exact":base==fraction0,"trace_sha256":hash(&base)}));
            for role in &ROLES[1..] {
                let name = format!("{condition}-{seed}-{role}/care-trace.json");
                let old = fs::read(previous.join(&name))?;
                let old_pin = audit["trace_pins"]
                    .as_array()
                    .ok_or("prior trace pins missing")?
                    .iter()
                    .find(|p| {
                        p["role"] == *role && p["condition"] == condition && p["seed"] == seed
                    })
                    .ok_or("prior case not audited")?;
                if old_pin["trace_sha256"] != hash(&old) {
                    return Err("prior audited trace changed".into());
                }
                let new = fs::read(output.join(&name))?;
                parity &= old == new;
                comparisons.push(json!({"kind":"prior-completed-control","role":role,"condition":condition,"seed":seed,"byte_exact":old==new,"trace_sha256":hash(&new)}));
            }
        }
    }
    let candidate = results
        .iter()
        .filter(|r| r["role"] == ROLES[0])
        .collect::<Vec<_>>();
    let viable = candidate.len() == 6
        && candidate.iter().all(|r| {
            r["complete"] == true
                && r["alive"] == true
                && r["organisms"][0]["biology"]["body"]["injury"].as_f64() == Some(0.)
        })
        && candidate
            .iter()
            .filter(|r| r["lesson_completed"] == true)
            .count()
            >= 4
        && candidate
            .iter()
            .filter(|r| r["condition"] == "held" && r["lesson_completed"] == true)
            .count()
            == 2;
    let gate = json!({"passed":parity && viable,"exact_control_parity":parity,"candidate_viable_for_battery":viable,"viability_rule":"all6 complete/alive, no injury, both held-food meals, at least4/6 total meals; pilot only, not adoption","pc_sha256":request.expected_pc_sha256,"cloud_tree_sha256":request.expected_cloud_tree_sha256,"cloud_policy":10468,"earlier_10343_reference_used":false,"executable_sha256":exe,"experiment_pins":experiment_pins(request)?,"comparisons":comparisons,"seed_overlap":"all20 assessment seeds were previously evaluated on controls; first2 reused in this pilot; no independent-holdout claim"});
    write(output.join("merge-reset-gate.json"), &gate)?;
    if !parity || !viable {
        return Err(
            "merge reset pilot did not qualify a larger battery; all outcomes retained".into(),
        );
    }
    Ok(())
}
pub(super) fn qualify_battery(request: &Request, exe: &str) -> Result<()> {
    let pilot = request
        .qualified_controls
        .as_ref()
        .ok_or("merge battery requires its completed pilot")?;
    let gate: Value = serde_json::from_slice(&fs::read(pilot.join("merge-reset-gate.json"))?)?;
    if gate["passed"] != true
        || gate["pc_sha256"] != request.expected_pc_sha256
        || gate["cloud_tree_sha256"] != request.expected_cloud_tree_sha256
        || gate["executable_sha256"] != exe
        || gate["experiment_pins"] != experiment_pins(request)?
    {
        return Err("merge battery gate differs from sources, candidates or executable".into());
    }
    Ok(())
}
