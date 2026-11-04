//! Native artifact source binding, separate from full native recovery admission.
use super::*;
use alife_core::{BrainPhenotype, PhenotypeCompiler};
use alife_world::{persistence::*, PortableSaveFile};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedProjectionBinding {
    pub kind: String,
    pub source_current_sha256: String,
    pub organism_id: u64,
    pub checkpoint_tick: u64,
    /// SHA256 of the exact selected raw assets, not their JSON reserialization.
    pub selected_asset_sha256: BTreeMap<String, String>,
    pub complete_declared_asset_count: usize,
}

/// Read only from a caller-pinned complete Current. The returned projection has
/// no caller-supplied neural payload. This checks selected native asset identity
/// and closure bytes, not GPU execution, sleep recovery or every asset's semantics.
pub fn derive_retained_projection_from_current(
    source_bytes: &[u8],
    expected_sha256: &str,
) -> Result<(CpuRetainedNeuralInheritance, RetainedProjectionBinding)> {
    if sha256(source_bytes) != expected_sha256 {
        return Err(CpuHostError::State(
            "projection source differs from SHA256 pin",
        ));
    }
    let current = CurrentTrainingCheckpointV1::decode(source_bytes)
        .map_err(|_| CpuHostError::State("projection source Current codec admission failed"))?;
    extract(&current, expected_sha256)
}

pub(super) fn extract(
    current: &CurrentTrainingCheckpointV1,
    source_current_sha256: &str,
) -> Result<(CpuRetainedNeuralInheritance, RetainedProjectionBinding)> {
    let bad = |message| CpuHostError::State(message);
    let life = current
        .life
        .as_ref()
        .ok_or_else(|| bad("PC private life missing"))?;
    let save = PortableSaveFile::from_json_str(&life.world_json)?;
    if save.creatures.len() != 1 || save.deterministic_seed != current.seed {
        return Err(bad(
            "PC projection requires one source individual at its saved seed",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    // Authenticate the exact declared byte closure; no guessed paths or disk IO.
    for entry in &save.assets.entries {
        let path = entry.relative_path.as_str();
        if !ids.insert(entry.asset_id.as_str())
            || !paths.insert(path)
            || path.is_empty()
            || path.contains(['\\', ':'])
            || path.split('/').any(|p| matches!(p, "" | "." | ".."))
        {
            return Err(bad(
                "private asset identity/path is ambiguous or noncanonical",
            ));
        }
        let bytes = life
            .asset_bytes(path)
            .map_err(|_| bad("declared private asset missing"))?;
        if entry.size_bytes != Some(bytes.len() as u64)
            || entry.digest != PortableAssetDigest::for_bytes(&bytes)
        {
            return Err(bad(
                "private asset byte length or digest differs from manifest",
            ));
        }
    }
    if paths != life.assets_base64.keys().map(String::as_str).collect() {
        return Err(bad("private artifact closure has extra or missing assets"));
    }
    let brain = save.creatures[0]
        .gpu_brain
        .as_ref()
        .ok_or_else(|| bad("PC native GPU state missing"))?;
    let admitted = current.handoff.clone().admit()?;
    let p = &admitted.phenotype;
    if brain.schema_version != GPU_BRAIN_SAVE_STATE_SCHEMA_VERSION
        || brain.organism_id != save.creatures[0].organism_id
        || brain.checkpoint_tick != save.world.tick
        || brain.phenotype_hash != p.phenotype_hash()
        || brain.capacity_class_id != p.brain_class_id()
        || brain.sensor_profile.profile()? != p.sensor_profile()
        || brain.pending_eligibility.is_some()
        || brain.pending_experience_transaction.is_some()
        || brain.memory.retained_learning.is_some()
    {
        return Err(bad(
            "PC native identity or sealed inheritance boundary differs",
        ));
    }
    let mut selected_asset_sha256 = BTreeMap::new();
    let mut read = |role: &str, reference: &GpuBrainAssetRef| -> Result<Vec<u8>> {
        let entry = save
            .assets
            .entries
            .iter()
            .find(|e| e.asset_id == reference.asset_id)
            .ok_or_else(|| bad("selected private asset not declared"))?;
        if entry.digest != reference.digest || entry.presence != AssetPresence::Required {
            return Err(bad(
                "selected private reference digest or requiredness differs",
            ));
        }
        let bytes = life
            .asset_bytes(&entry.relative_path)
            .map_err(|_| bad("selected private asset missing"))?;
        selected_asset_sha256.insert(role.into(), sha256(&bytes));
        Ok(bytes)
    };
    let saved_p: BrainPhenotype =
        serde_json::from_slice(&read("phenotype", &brain.immutable_phenotype)?)?;
    let saved_inputs: PhenotypeCompilerInputs =
        serde_json::from_slice(&read("compiler_inputs", &brain.phenotype_compiler_inputs)?)?;
    let compiled =
        PhenotypeCompiler::compile_validated(&saved_inputs, &BrainCapacityClass::n2048())?;
    if serde_json::to_vec(&saved_inputs)? != serde_json::to_vec(&current.handoff.compiler_inputs)?
        || serde_json::to_vec(&saved_p)? != serde_json::to_vec(p)?
        || serde_json::to_vec(&compiled)? != serde_json::to_vec(p)?
    {
        return Err(bad(
            "source private phenotype/compiler inputs differ from global head",
        ));
    }
    let topology: GpuLiveTopologyCheckpointV1 = serde_json::from_slice(&read(
        "topology",
        brain
            .live_structural_topology
            .as_ref()
            .ok_or_else(|| bad("live topology reference missing"))?,
    )?)?;
    let exact = ExactCognitiveCheckpointState::decode(&read(
        "exact_cognition",
        brain
            .exact_cognitive_state
            .as_ref()
            .ok_or_else(|| bad("exact cognition reference missing"))?,
    )?)?;
    if exact.organism_id != brain.organism_id
        || exact.checkpoint_tick != brain.checkpoint_tick
        || exact.dendritic_branches != topology.v11_checkpoint.dendritic_branches
        || exact.structural_plasticity != topology.v11_checkpoint.structural
    {
        return Err(bad(
            "exact cognition identity or acquired structure differs from native topology",
        ));
    }
    let lifetime: PortableDualWeightBankV1 =
        serde_json::from_slice(&read("lifetime", &brain.lifetime_weights)?)?;
    let fast: PortableDualWeightBankV1 =
        serde_json::from_slice(&read("fast", &brain.fast_weights)?)?;
    for bank in [&lifetime, &fast] {
        if bank.active_bank != brain.active_weight_bank
            || bank.active_generation != brain.active_weight_generation
        {
            return Err(bad(
                "retained banks differ from source active selector/generation",
            ));
        }
    }
    let retained = CpuRetainedNeuralInheritance {
        topology,
        lifetime,
        fast,
        predictor: exact.predictor,
    };
    retained.canonical_to_live(p)?;
    Ok((
        retained,
        RetainedProjectionBinding {
            kind: "sha256-pinned-Current-manifest-native-retained-projection-v1".into(),
            source_current_sha256: source_current_sha256.into(),
            organism_id: brain.organism_id.raw(),
            checkpoint_tick: brain.checkpoint_tick.raw(),
            selected_asset_sha256,
            complete_declared_asset_count: save.assets.entries.len(),
        },
    ))
}
