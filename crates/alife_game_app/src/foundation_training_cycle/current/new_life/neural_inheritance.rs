//! Explicit acquired-neural inheritance. Live coordinates are copied exactly;
//! global Adam stays in its existing canonical coordinates. No world experience
//! or pending outcome is relabeled as experience of the newborn.
use super::*;
use alife_core::BrainPhenotype;
use alife_gpu_backend::GpuLiveTopologyCheckpointV1;
use alife_training::{CurrentLifeCheckpointV1, NewLifeNeuralInheritanceV1};
use alife_world::persistence::*;
use alife_world::PortableSaveFile;

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_asset(
    life: &CurrentLifeCheckpointV1,
    save: &PortableSaveFile,
    reference: &GpuBrainAssetRef,
) -> Result<Vec<u8>> {
    let mut entries = save
        .assets
        .entries
        .iter()
        .filter(|entry| entry.asset_id == reference.asset_id);
    let entry = entries.next().ok_or("retained-neural asset missing")?;
    if entries.next().is_some()
        || entry.digest != reference.digest
        || entry.presence != AssetPresence::Required
    {
        return Err("retained-neural asset identity is ambiguous or changed".into());
    }
    let bytes = life.asset_bytes(&entry.relative_path)?;
    if PortableAssetDigest::for_bytes(&bytes) != reference.digest
        || entry.size_bytes != Some(bytes.len() as u64)
    {
        return Err("retained-neural asset bytes differ from manifest".into());
    }
    Ok(bytes)
}

pub(super) struct RetainedNeural {
    topology_bytes: Vec<u8>,
    lifetime_bytes: Vec<u8>,
    fast_bytes: Vec<u8>,
    pub(super) topology: GpuLiveTopologyCheckpointV1,
    pub(super) exact: ExactCognitiveCheckpointState,
    lifetime: PortableDualWeightBankV1,
    fast: PortableDualWeightBankV1,
}
impl RetainedNeural {
    #[cfg(feature = "cpu-training")]
    pub(super) fn cpu_projection(&self) -> alife_training::cpu::CpuRetainedNeuralInheritance {
        alife_training::cpu::CpuRetainedNeuralInheritance {
            topology: self.topology.clone(),
            lifetime: self.lifetime.clone(),
            fast: self.fast.clone(),
            predictor: self.exact.predictor.clone(),
        }
    }
    pub(super) fn read(source: &CurrentTrainingCheckpointV1) -> Result<Self> {
        let life = source
            .life
            .as_ref()
            .ok_or("retained-neural source private state missing")?;
        let save = PortableSaveFile::from_json_str(&life.world_json)?;
        if save.creatures.len() != 1 {
            return Err("retained-neural source must have one individual".into());
        }
        let brain = save.creatures[0]
            .gpu_brain
            .as_ref()
            .ok_or("retained-neural GPU checkpoint missing")?;
        if brain.schema_version != GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION
            || brain.pending_eligibility.is_some()
            || brain.pending_experience_transaction.is_some()
            || brain.memory.retained_learning.is_some()
        {
            return Err("retained-neural transfer requires a current sealed checkpoint with no pending outcome credit; preserve the source".into());
        }
        let topology_bytes = read_asset(
            life,
            &save,
            brain
                .live_structural_topology
                .as_ref()
                .ok_or("retained live topology missing")?,
        )?;
        let topology: GpuLiveTopologyCheckpointV1 = serde_json::from_slice(&topology_bytes)?;
        let phenotype = source.handoff.clone().admit()?.phenotype;
        validate_topology(&topology, &phenotype)?;
        let exact = ExactCognitiveCheckpointState::decode(&read_asset(
            life,
            &save,
            brain
                .exact_cognitive_state
                .as_ref()
                .ok_or("exact source cognition missing")?,
        )?)?;
        if exact.organism_id != brain.organism_id
            || exact.checkpoint_tick != brain.checkpoint_tick
            || exact.dendritic_branches != topology.v11_checkpoint.dendritic_branches
            || exact.structural_plasticity != topology.v11_checkpoint.structural
        {
            return Err("source cognitive structure differs from live topology".into());
        }
        let lifetime_bytes = read_asset(life, &save, &brain.lifetime_weights)?;
        let fast_bytes = read_asset(life, &save, &brain.fast_weights)?;
        let lifetime: PortableDualWeightBankV1 = serde_json::from_slice(&lifetime_bytes)?;
        let fast: PortableDualWeightBankV1 = serde_json::from_slice(&fast_bytes)?;
        lifetime.validate()?;
        fast.validate()?;
        if lifetime.layer_raw != GPU_BRAIN_WEIGHT_LAYER_LIFETIME
            || fast.layer_raw != GPU_BRAIN_WEIGHT_LAYER_FAST
            || lifetime.phenotype_hash != topology.phenotype_hash
            || fast.phenotype_hash != topology.phenotype_hash
            || lifetime.synapse_count != topology.total_synapse_count
            || fast.synapse_count != topology.total_synapse_count
            || lifetime.active_generation != brain.active_weight_generation
            || fast.active_generation != brain.active_weight_generation
            || lifetime.active_bank != brain.active_weight_bank
            || fast.active_bank != brain.active_weight_bank
        {
            return Err("source neural weight banks do not match exact live coordinates".into());
        }
        Ok(Self {
            topology_bytes,
            lifetime_bytes,
            fast_bytes,
            topology,
            exact,
            lifetime,
            fast,
        })
    }
    fn proof(&self, source: &CurrentTrainingCheckpointV1) -> Result<NewLifeNeuralInheritanceV1> {
        Ok(NewLifeNeuralInheritanceV1 {
            schema: 1,
            kind: "exact-live-order-global-adam-neural-inheritance-v1".into(),
            canonical_actor_coordinates: source.handoff.actor.weights.len(),
            live_synapse_count: self.topology.total_synapse_count,
            live_recurrent_count: self.topology.recurrent_synapse_count,
            live_decoder_count: self.topology.decoder_synapse_count,
            source_topology_sha256: sha(&self.topology_bytes),
            source_lifetime_sha256: sha(&self.lifetime_bytes),
            source_fast_sha256: sha(&self.fast_bytes),
            source_predictor_sha256: sha(&serde_json::to_vec(&self.exact.predictor)?),
        })
    }
}
fn validate_topology(
    topology: &GpuLiveTopologyCheckpointV1,
    phenotype: &BrainPhenotype,
) -> Result<()> {
    topology.validate_for_capacity(&BrainCapacityClass::n2048())?;
    let compiled = GpuLiveTopologyCheckpointV1::recompile_for_phenotype(
        phenotype,
        topology.v11_checkpoint.clone(),
    )?;
    if serde_json::to_vec(topology)? != serde_json::to_vec(&compiled)? {
        return Err(
            "retained-neural topology differs from exact native compiler coordinates".into(),
        );
    }
    Ok(())
}

fn put(
    life: &mut CurrentLifeCheckpointV1,
    save: &mut PortableSaveFile,
    kind: &str,
    bytes: &[u8],
) -> Result<GpuBrainAssetRef> {
    let digest = PortableAssetDigest::for_bytes(bytes);
    let suffix = digest
        .0
        .strip_prefix("fnv1a64:")
        .ok_or("invalid neural content digest")?;
    let id = format!("gpu-brain.{kind}.{suffix}");
    let relative = format!("gpu-brain/{kind}-{suffix}.json");
    if let Some(old) = save.assets.entries.iter().find(|e| e.asset_id == id) {
        if old.relative_path != relative || life.asset_bytes(&relative)? != bytes {
            return Err("neural inheritance asset collision".into());
        }
    } else {
        save.assets.entries.push(AssetManifestEntry {
            asset_id: id.clone(),
            kind: AssetKind::Other,
            relative_path: relative.clone(),
            digest: digest.clone(),
            presence: AssetPresence::Required,
            schema_version: GPU_BRAIN_PORTABLE_ASSET_SCHEMA_VERSION,
            size_bytes: Some(bytes.len() as u64),
            provenance: Some(format!("gpu-checkpoint:{kind}")),
        });
        life.insert_asset(relative, bytes);
    }
    Ok(GpuBrainAssetRef {
        asset_id: id,
        digest,
    })
}

pub(super) fn inherit(
    source: &CurrentTrainingCheckpointV1,
    mut life: CurrentLifeCheckpointV1,
) -> Result<(CurrentLifeCheckpointV1, NewLifeNeuralInheritanceV1)> {
    let retained = RetainedNeural::read(source)?;
    let proof = retained.proof(source)?;
    let mut save = PortableSaveFile::from_json_str(&life.world_json)?;
    if save.world.tick.raw() != 0 || save.creatures.len() != 1 {
        return Err("retained-neural target must be an undispatched newborn".into());
    }
    let mut brain = save.creatures[0]
        .gpu_brain
        .clone()
        .ok_or("newborn GPU state missing")?;
    if brain.organism_id == retained.exact.organism_id
        || brain.phenotype_hash != retained.topology.phenotype_hash
        || brain.pending_eligibility.is_some()
        || brain.replay_journal_event_count != 0
    {
        return Err("retained-neural target identity or boundary mismatch".into());
    }
    let mut exact = ExactCognitiveCheckpointState::decode(&read_asset(
        &life,
        &save,
        brain
            .exact_cognitive_state
            .as_ref()
            .ok_or("newborn exact cognition missing")?,
    )?)?;
    if exact.checkpoint_tick.raw() != 0
        || exact.organism_id != brain.organism_id
        || exact.selected_motor_bundle.is_some()
        || exact.cognitive_work != alife_core::CognitiveWorkReceipt::zero()
    {
        return Err("retained-neural target contains prior world activity".into());
    }
    // Preserve learned parameters and all structural candidate/connection ages.
    // Current attention, motor/work/sleep receipts, replay and episodic memory
    // remain those of the actual newborn; they are never relabeled source events.
    exact.dendritic_branches = retained.exact.dendritic_branches.clone();
    exact.structural_plasticity = retained.exact.structural_plasticity.clone();
    exact.predictor = retained.exact.predictor.clone();
    exact.validate()?;
    let mut eligibility: PortableEligibilityBanksV1 =
        serde_json::from_slice(&read_asset(&life, &save, &brain.eligibility)?)?;
    eligibility.validate()?;
    if eligibility
        .recurrent_bank_0_bits
        .iter()
        .chain(&eligibility.recurrent_bank_1_bits)
        .chain(&eligibility.decoder_bank_0_bits)
        .chain(&eligibility.decoder_bank_1_bits)
        .any(|word| *word != 0)
    {
        return Err("newborn eligibility is not fresh".into());
    }
    eligibility.recurrent_count = retained.topology.recurrent_synapse_count;
    eligibility.decoder_count = retained.topology.decoder_synapse_count;
    eligibility.recurrent_bank_0_bits = vec![0; eligibility.recurrent_count as usize];
    eligibility.recurrent_bank_1_bits = vec![0; eligibility.recurrent_count as usize];
    eligibility.decoder_bank_0_bits = vec![0; eligibility.decoder_count as usize];
    eligibility.decoder_bank_1_bits = vec![0; eligibility.decoder_count as usize];
    eligibility.canonical_digest = eligibility.recompute_canonical_digest()?;
    eligibility.validate()?;
    brain.live_structural_topology = Some(put(
        &mut life,
        &mut save,
        "live-structural-topology",
        &retained.topology_bytes,
    )?);
    brain.lifetime_weights = put(&mut life, &mut save, "lifetime", &retained.lifetime_bytes)?;
    brain.fast_weights = put(&mut life, &mut save, "fast", &retained.fast_bytes)?;
    brain.active_weight_generation = retained.lifetime.active_generation;
    brain.active_weight_bank = retained.fast.active_bank;
    brain.eligibility = put(
        &mut life,
        &mut save,
        "eligibility",
        &serde_json::to_vec(&eligibility)?,
    )?;
    brain.exact_cognitive_state = Some(put(
        &mut life,
        &mut save,
        "exact-cognitive-state",
        &exact.encode()?,
    )?);
    brain.validate()?;
    save.creatures[0].gpu_brain = Some(brain);
    // Reuse the normal closure filter and reset only the newborn's empty sleep
    // journal binding to its new manifest. The source journal is never modified.
    alife_runtime::retain_current_gpu_checkpoint_manifest_entries(&mut save);
    let closure = save
        .assets
        .entries
        .iter()
        .map(|e| e.relative_path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    life.assets_base64.retain(|path, _| closure.contains(path));
    life.world_json = serde_json::to_string(&save)?;
    reanchor_newborn_journal(&mut life, &save)?;
    verify(&retained, &life)?;
    Ok((life, proof))
}
fn reanchor_newborn_journal(
    life: &mut CurrentLifeCheckpointV1,
    save: &PortableSaveFile,
) -> Result<()> {
    if save.world.tick != alife_core::Tick::ZERO {
        return Err("only an undispatched newborn can receive an empty journal".into());
    }
    // Same canonical anchor as GpuLoadedSaveManifest::exact_save_anchor_digest.
    let anchor = PortableAssetDigest::for_bytes(&serde_json::to_vec_pretty(save)?);
    life.sleep_journal_json =
        serde_json::to_string(&alife_runtime::GpuSleepTransactionJournalV2::try_new(
            anchor.0,
            save.world.tick,
            Vec::new(),
        )?)?;
    Ok(())
}
pub(super) fn verify(retained: &RetainedNeural, life: &CurrentLifeCheckpointV1) -> Result<()> {
    let save = PortableSaveFile::from_json_str(&life.world_json)?;
    let brain = save.creatures[0]
        .gpu_brain
        .as_ref()
        .ok_or("inherited GPU state missing")?;
    if read_asset(
        life,
        &save,
        brain
            .live_structural_topology
            .as_ref()
            .ok_or("inherited topology missing")?,
    )? != retained.topology_bytes
        || read_asset(life, &save, &brain.lifetime_weights)? != retained.lifetime_bytes
        || read_asset(life, &save, &brain.fast_weights)? != retained.fast_bytes
    {
        return Err("inherited neural coordinates or weight bits changed".into());
    }
    let exact = ExactCognitiveCheckpointState::decode(&read_asset(
        life,
        &save,
        brain
            .exact_cognitive_state
            .as_ref()
            .ok_or("inherited cognition missing")?,
    )?)?;
    if serde_json::to_vec(&exact.predictor)? != serde_json::to_vec(&retained.exact.predictor)?
        || serde_json::to_vec(&exact.dendritic_branches)?
            != serde_json::to_vec(&retained.exact.dendritic_branches)?
        || serde_json::to_vec(&exact.structural_plasticity)?
            != serde_json::to_vec(&retained.exact.structural_plasticity)?
    {
        return Err("inherited learned cognition changed".into());
    }
    Ok(())
}

pub(super) fn inspect(source: &CurrentTrainingCheckpointV1) -> Result<serde_json::Value> {
    let retained = RetainedNeural::read(source)?;
    Ok(
        serde_json::json!({"kind":"retained-neural-preflight-v1","schema":retained.topology.schema_version,
        "neuron_count":retained.topology.neuron_count,"canonical_actor_coordinates":source.handoff.actor.weights.len(),
        "live_synapses":retained.topology.total_synapse_count,"recurrent_synapses":retained.topology.recurrent_synapse_count,
        "decoder_synapses":retained.topology.decoder_synapse_count,"sparse_spans":retained.topology.v11_checkpoint.sparse_spans.len(),
        "acquired_edges":retained.topology.v11_checkpoint.sparse_spans.iter().map(|s|s.edges.len()).sum::<usize>(),
        "structural_connections":retained.topology.v11_checkpoint.structural.connection_count(),
        "structural_candidates":retained.topology.v11_checkpoint.structural.candidate_count(),
        "dendritic_branches":retained.topology.v11_checkpoint.dendritic_branches.branches().len(),
        "pending_lifetime_synapse":false,"source_unchanged":true,"gpu_initialization":false,"seeds_consumed":0}),
    )
}

#[cfg(test)]
pub(super) mod tests;
