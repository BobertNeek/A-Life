//! Native full-life direct weight calibration. No optimizer reset, birth or fabricated PPO update.
use super::*;
use alife_core::{CompiledSynapseKind, DecoderHeadKind, N2048LivePolicyTransition, OrganismId};
use alife_training::{CurrentTrainingCheckpointV1, FoundationTrainingHandoffV1};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectWeightEditRequest {
    pub schema: u16,
    pub source_current_sha256: String,
    pub generator_id: String,
    pub edits: Vec<(usize, u32)>,
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn checked_direct_weights(
    baseline: &CurrentTrainingCheckpointV1,
    bytes: &[u8],
    request: &DirectWeightEditRequest,
) -> Result<Vec<f32>> {
    if request.schema != 1
        || request.source_current_sha256 != sha(bytes)
        || request.generator_id.is_empty()
        || request.generator_id.len() > 256
        || request.edits.len() > 8192
    {
        return Err("invalid direct-weight request/source/generator/bound".into());
    }
    baseline.validate()?;
    let phenotype = alife_training::compile_validated_n2048_cached(
        &baseline.handoff.compiler_inputs,
        &alife_core::BrainCapacityClass::n2048(),
    )?;
    let mut weights = baseline.handoff.actor.weights.clone();
    let mut seen = std::collections::BTreeSet::new();
    for &(index, bits) in &request.edits {
        let synapse = phenotype
            .synapses()
            .get(index)
            .ok_or("direct-weight coordinate out of bounds")?;
        if !seen.insert(index)
            || !baseline.handoff.actor.stage_mask.is_trainable(index)
            || !matches!(synapse.kind(), CompiledSynapseKind::Decoder(c) if c.head() == DecoderHeadKind::ActionCandidate)
            || !f32::from_bits(bits).is_finite()
        {
            return Err(
                "direct-weight edit is not a unique finite admitted ActionCandidate coordinate"
                    .into(),
            );
        }
        weights[index] = f32::from_bits(bits);
    }
    Ok(weights)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn install_and_seal_direct_weights(
    runtime: &mut GpuLiveBrainRuntime,
    baseline: &CurrentTrainingCheckpointV1,
    source_bytes: &[u8],
    life: &Path,
    save_path: &Path,
    assets: &Path,
    output: &Path,
    id: OrganismId,
    request: &DirectWeightEditRequest,
    deadline: u64,
) -> Result<(CurrentTrainingCheckpointV1, Vec<u8>, serde_json::Value)> {
    let weights = checked_direct_weights(baseline, source_bytes, request)?;
    let changed = weights
        .iter()
        .zip(&baseline.handoff.actor.weights)
        .filter(|(a, b)| a.to_bits() != b.to_bits())
        .count();
    if request.edits.is_empty() {
        return Ok((
            baseline.clone(),
            source_bytes.to_vec(),
            serde_json::json!({"kind":"direct-weight-control-v1","changed_weights":0,"optimizer_updates":0}),
        ));
    }
    if changed == 0 {
        return Err("nonempty direct-weight request changes no actual bits".into());
    }
    let tick = runtime.world().tick().raw();
    let sampling_before = runtime.foundation_training_sampling_state();
    runtime.quiesce_live_foundation_policy_boundary(id, deadline)?;
    let old_asset = baseline
        .handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or("direct-weight source lacks asset")?;
    let phenotype = alife_training::compile_validated_n2048_cached(
        &baseline.handoff.compiler_inputs,
        &alife_core::BrainCapacityClass::n2048(),
    )?;
    let asset = FoundationWeightAsset::from_trained_weights(
        &phenotype,
        weights.clone(),
        old_asset.manifest().training_stage(),
    )?;
    let transition = N2048LivePolicyTransition::calibrate_existing_policy(
        &baseline.handoff.compiler_inputs,
        baseline.handoff.policy_version,
        asset.clone(),
    )?;
    let mut actor = baseline.handoff.actor.clone();
    actor.weights = weights;
    actor.phenotype_hash = transition.next_phenotype().phenotype_hash();
    actor.source_foundation_digest = asset.digest();
    let handoff = FoundationTrainingHandoffV1::new_same_individual(
        baseline.handoff.origin,
        baseline.handoff.policy_version,
        transition.next_inputs().clone(),
        actor,
        baseline.handoff.value.clone(),
    )?;
    let install = runtime.install_live_foundation_policy(id, &transition)?;
    if install.mutable_words_written != 0
        || !install.submission_completed
        || runtime.world().tick().raw() != tick
        || runtime.foundation_training_sampling_state() != sampling_before
    {
        return Err("direct-weight installation modified mutable/time/sampling authority".into());
    }
    runtime.quiesce_live_foundation_checkpoint(deadline)?;
    let exact = runtime.capture_portable_checkpoint()?;
    let published = crate::GpuDurableSaveManifest::publish_snapshot(save_path, assets, &exact)?;
    if published.save != exact {
        return Err("direct-weight private publication differs".into());
    }
    runtime.rebind_durable_checkpoint_boundary(save_path, assets, &exact)?;
    let sampling = baseline
        .saved_training_sampling()?
        .ok_or("direct-weight parent sampler absent")?;
    let nonce = baseline
        .same_individual
        .as_ref()
        .ok_or("direct-weight parent is not an admitted individual")?
        .progress
        .session_nonce;
    let next = seal_same_individual_direct_weight_checkpoint(
        baseline,
        source_bytes,
        handoff,
        life,
        alife_training::SameIndividualProgressV1 {
            session_nonce: nonce,
            organism_id: id.raw(),
            session_start_world_tick: tick,
            world_tick: tick,
            trained_rows: 0,
            completed_updates: 0,
            sampling_temperature: sampling.temperature,
        },
        baseline.seeds.clone(),
    )?;
    let bytes = next.encode()?;
    let path = output.join("CALIBRATED-PARENT.alife-checkpoint");
    if path.exists() {
        return Err("direct-weight calibration parent output already exists".into());
    }
    alife_storage::write(&path, &bytes)?;
    if read_current_training_checkpoint(&path)? != next {
        return Err("direct-weight fullCurrent readback differs".into());
    }
    let receipt = serde_json::json!({"kind":"same-individual-direct-weights-v1","generator_id":request.generator_id,
        "source_current_sha256":sha(source_bytes),"calibrated_current_sha256":sha(&bytes),"request_sha256":sha(&serde_json::to_vec(request)?),
        "changed_weights":changed,"requested_coordinates":request.edits.len(),"optimizer_updates":0,"PPO_policy_counter_unchanged":true,
        "policy_version":next.handoff.policy_version,"actor_optimizer_step":next.handoff.actor.optimizer_step,"value_optimizer_step":next.handoff.value.optimizer_step,
        "Adam_moments_ages_retained":true,"critic_retained":true,"mutable_words_written":install.mutable_words_written,
        "world_tick":tick,"same_individual":true,"sampler_retained":true,"new_foundation_digest":asset.digest(),"new_phenotype_hash":transition.next_phenotype().phenotype_hash()});
    alife_storage::write(
        output.join("DIRECT-WEIGHT-CALIBRATION.json"),
        serde_json::to_vec_pretty(&receipt)?,
    )?;
    Ok((next, bytes, receipt))
}
