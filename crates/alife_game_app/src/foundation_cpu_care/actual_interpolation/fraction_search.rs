//! Full-horizon assessment on a separately reserved, disclosed seed range.
//! An adverse individual outcome never rejects an entire candidate cohort.
use super::*;
const FIRST: u64 = 2026110418912001;
const LAST: u64 = 2026110418912020;
pub(super) fn validate_request(request: &Request) -> Result<()> {
    let first = request
        .seed_first
        .ok_or("search requires an explicit reserved first seed")?;
    let count = request
        .seed_count
        .ok_or("search requires an explicit seed count")?;
    let trace = request.phase == "injury_trace";
    if trace {
        if first != 202611185879 || count != 1 {
            return Err("injury replay must use its exact previously observed seed".into());
        }
    } else if count == 0
        || count > 20
        || first < FIRST
        || first
            .checked_add(count as u64 - 1)
            .is_none_or(|last| last > LAST)
    {
        return Err("search exceeds its separately reserved assessment range".into());
    }
    if !request.record_physiology {
        return Err("search requires passive physiology recording".into());
    }
    if request.candidates.len() != 4
        || request
            .candidates
            .iter()
            .map(|c| c.fraction)
            .collect::<Vec<_>>()
            != [0., 0.25, 0.5, 0.75]
    {
        return Err(
            "search requires source-bound fraction0,1/4,1/2,3/4 prepared experiments".into(),
        );
    }
    if !matches!(
        request.ablation_variant.as_deref(),
        Some(
            "interpolation_025_reset_somatic"
                | "interpolation_05_reset_somatic"
                | "interpolation_075_reset_somatic"
                | "reset_somatic"
                | "pc_native_fresh"
                | "cloud_native_fresh"
                | "fraction0_reset_somatic"
        )
    ) {
        return Err("search requires one explicitly partitioned arm".into());
    }
    Ok(())
}
pub(super) fn models(
    request: &Request,
    pc: &alife_training::CurrentTrainingCheckpointV1,
    cloud: &alife_training::FoundationTrainingHandoffV1,
    pc_bytes: &[u8],
    proof: &Option<SavedBodyRoundingCompatibilityV1>,
) -> Result<Vec<Model>> {
    let (role, index) = match request.ablation_variant.as_deref() {
        Some("fraction0_reset_somatic") => ("fraction0_reset_somatic", Some(0)),
        Some("interpolation_025_reset_somatic") => ("interpolation_025_reset_somatic", Some(1)),
        Some("interpolation_05_reset_somatic") => ("interpolation_05_reset_somatic", Some(2)),
        Some("interpolation_075_reset_somatic") => ("interpolation_075_reset_somatic", Some(3)),
        Some("reset_somatic") => ("reset_somatic", None),
        Some("pc_native_fresh") => ("pc_native_fresh", None),
        Some("cloud_native_fresh") => ("cloud_native_fresh", None),
        _ => return Err("unsupported partitioned search arm".into()),
    };
    let mut model = if let Some(index) = index {
        let mut model = model_candidate(
            &request.candidates[index],
            request,
            pc_bytes,
            proof,
            &pc.handoff.actor.stage_mask,
        )?;
        merge_reset::reset(&mut model, role)?;
        model
    } else if role == "reset_somatic" {
        let (projection, _) = scope(proof, || {
            Ok(derive_retained_projection_from_current(
                pc_bytes,
                &request.expected_pc_sha256,
            )?)
        })?;
        let mut model = model_parent(role, &pc.handoff, Some(projection))?;
        merge_reset::reset(&mut model, role)?;
        model
    } else if role == "pc_native_fresh" {
        model_parent(role, &pc.handoff, None)?
    } else {
        model_parent(role, cloud, None)?
    };
    model.identity["assessment_protocol"] = json!({"name":"full-horizon-fraction-search","new_reserved_seed_range":if request.phase=="injury_trace"{Value::Null}else{json!([FIRST,LAST])},"historical_freshness":"unknown; no disclosed prior use; native known-exclusion checks mandatory","bank_reset":"trial start only; ordinary waking and sleep afterwards","individual_boundary":"same128scored plus1bootstrap horizon; natural death or technical limit only; no injury/health censoring","cohort_rule":"continue all assigned cases; no zero-injury cohort rejection","original_histories":"both original parents and full Adam retained unchanged; no merged optimizer","cloud_parent":10468,"10343_reference_used":false});
    Ok(vec![model])
}
