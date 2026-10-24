//! Read-only import of explicitly CPU-produced founder/optimizer state.
//! Decisions and individual lifetimes keep their original execution domain.
use super::*;
#[cfg(not(feature = "cpu-training"))]
use alife_training::{FoundationTrainingBackendV1, FoundationTrainingHandoffV1};

#[cfg(not(feature = "cpu-training"))]
pub(super) fn source(previous: &Path) -> Result<SealedCycleContinuation> {
    let (_, receipt) = admission::read(&previous.join("cpu-cycle.json"))?;
    alife_training::native_receipt::Lineage::load(&receipt, previous)?;
    let (handoff_bytes, _) = admission::read(&previous.join("training-handoff.json"))?;
    let handoff: FoundationTrainingHandoffV1 = serde_json::from_slice(&handoff_bytes)?;
    handoff.validate()?;
    if std::fs::read_to_string(previous.join("phase.txt"))?.trim() != "complete-fresh-cohort-cpu"
        || receipt["schema_version"] != 1
        || receipt["complete"] != true
        || receipt["mode"] != "native_cpu_fresh_cohort"
        || receipt["backend"] != "native-cpu"
        || receipt["receipt_filename"] != "cpu-cycle.json"
        || receipt["handoff_filename"] != "training-handoff.json"
        || receipt["objective_state_reset"] != false
        || receipt["biological_objective_version"] != 2
        || receipt["critic_objective_version"] != 1
        || handoff.origin != FoundationTrainingBackendV1::NativeCpu
        || receipt["policy_version"].as_u64() != Some(handoff.policy_version)
        || receipt["native_output_sha256"]["training-handoff.json"]
            != admission::hash(&handoff_bytes)
    {
        return Err("CPU source is not a complete native typed handoff".into());
    }
    let admitted = handoff.admit()?;
    let (actor_bytes, _) = admission::read(&previous.join("actor-checkpoint.json"))?;
    let (value_bytes, _) = admission::read(&previous.join("value-checkpoint.json"))?;
    if serde_json::from_slice::<alife_training::FoundationTrainerCheckpoint>(&actor_bytes)?
        != admitted.actor
        || serde_json::from_slice::<alife_training::PpoValueHeadCheckpoint>(&value_bytes)?
            != admitted.value
        || receipt["native_output_sha256"]["actor-checkpoint.json"] != admission::hash(&actor_bytes)
        || receipt["native_output_sha256"]["value-checkpoint.json"] != admission::hash(&value_bytes)
    {
        return Err("CPU checkpoint differs from its exact typed handoff".into());
    }
    let asset_bytes = std::fs::read(previous.join("trained.alife-foundation"))?;
    if asset_bytes != admitted.foundation.encode_canonical()?
        || receipt["native_output_sha256"]["trained.alife-foundation"]
            != admission::hash(&asset_bytes)
        || receipt["new_asset_digest"] != digest(&admitted.foundation)
        || receipt["actor_optimizer_step"].as_u64()
            != Some(u64::from(admitted.actor.optimizer_step))
        || receipt["value_optimizer_step"].as_u64()
            != Some(u64::from(admitted.value.optimizer_step))
    {
        return Err("CPU native founder or optimizer receipt differs".into());
    }
    let seed = receipt["seed"]
        .as_u64()
        .filter(|s| *s > 0)
        .ok_or("missing CPU source seed")?;
    let founder_seed_base = receipt["founder_seed_base"]
        .as_u64()
        .filter(|s| *s > 0)
        .ok_or("missing CPU founder seed")?;
    // Validate coordinates against the ordinary next cohort, before reserving a seed.
    let canonical = crate::foundation_archived_success_replay::compile_source(
        admitted.foundation.clone(),
        seed,
        founder_seed_base,
    )?;
    admitted
        .actor
        .validate_for(&canonical, admitted.foundation.digest())?;
    if canonical.phenotype_hash() != admitted.phenotype.phenotype_hash() {
        return Err("CPU handoff compiler differs from the canonical group founder".into());
    }
    Ok(SealedCycleContinuation {
        asset: admitted.foundation,
        actor: admitted.actor,
        value: admitted.value,
        policy_version: admitted
            .policy_version
            .checked_add(1)
            .ok_or("policy overflow")?,
        founder_seed_base,
        objective_transition_from: None,
    })
}

#[cfg(feature = "cpu-training")]
pub(super) fn source(previous: &Path) -> Result<SealedCycleContinuation> {
    let source = alife_training::cpu::read_cpu_founder_source(previous)?;
    let seed = source.receipt["seed"]
        .as_u64()
        .ok_or("CPU source seed missing")?;
    let founder_seed_base = source.receipt["founder_seed_base"]
        .as_u64()
        .ok_or("CPU founder seed missing")?;
    let admitted = source.handoff.admit()?;
    let canonical = crate::foundation_archived_success_replay::compile_source(
        admitted.foundation.clone(),
        seed,
        founder_seed_base,
    )?;
    admitted
        .actor
        .validate_for(&canonical, admitted.foundation.digest())?;
    if canonical.phenotype_hash() != admitted.phenotype.phenotype_hash() {
        return Err("CPU handoff compiler differs from the canonical founder".into());
    }
    Ok(SealedCycleContinuation {
        asset: admitted.foundation,
        actor: admitted.actor,
        value: admitted.value,
        policy_version: admitted
            .policy_version
            .checked_add(1)
            .ok_or("policy overflow")?,
        founder_seed_base,
        objective_transition_from: None,
    })
}
