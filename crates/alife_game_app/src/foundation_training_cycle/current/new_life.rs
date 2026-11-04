//! An explicitly selected training birth. No source/private state is mutated.
use super::*;
use alife_core::{BrainCapacityClass, N2048LivePolicyTransition, OrganismId, PhenotypeCompiler};
use std::{fs, io::Write};
mod neural_inheritance;

/// Preflight through the ordinary newborn constructor and the exact existing
/// learned-policy overlay. No device, filesystem writes or simulated tick.
pub(super) fn prepare(
    source: &CurrentTrainingCheckpointV1,
    seed: u64,
    id: u64,
) -> Result<(alife_world::CanonicalNewGame, N2048LivePolicyTransition)> {
    prepare_mode(source, seed, id, false)
}
fn prepare_mode(
    source: &CurrentTrainingCheckpointV1,
    seed: u64,
    id: u64,
    retained_neural: bool,
) -> Result<(alife_world::CanonicalNewGame, N2048LivePolicyTransition)> {
    source.new_life_seeds(seed, id)?;
    let saved = alife_world::PortableSaveFile::from_json_str(
        &source
            .life
            .as_ref()
            .ok_or("terminal private life missing")?
            .world_json,
    )?;
    let old_world = saved.restore_headless_world()?;
    let old = old_world
        .organism_registry()
        .iter()
        .next()
        .ok_or("terminal organism missing")?;
    let birth_asset = old
        .genome()
        .n2048_foundation_candidate
        .as_ref()
        .ok_or("terminal birth foundation missing")?;
    let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1)?;
    config.founder_seed_base = source.founder_seed_base;
    config.brain_class = BrainScaleTier::Standard2048;
    config.sensor_profile = source.handoff.compiler_inputs.sensor_profile();
    let game =
        alife_world::create_canonical_training_new_life(&config, birth_asset, OrganismId(id))?;
    let newborn = game
        .world
        .organism_registry()
        .get(OrganismId(id))
        .ok_or("newborn missing")?;
    if newborn.genome() != old.genome() || newborn.phenotype() != old.phenotype() {
        return Err("canonical new body is incompatible with terminal inherited identity".into());
    }
    let capacity = BrainCapacityClass::n2048();
    let genome = &newborn.phenotype().brain_genome;
    let development = alife_runtime::canonical_preparation::foundation_construction_development(
        genome,
        &capacity,
        &newborn
            .phenotype()
            .development_state_at(alife_core::Tick::ZERO)?,
    )?;
    let (_, inputs) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development,
        birth_asset.clone(),
    )?;
    let transition = N2048LivePolicyTransition::adopt_existing_policy(
        &inputs,
        source.handoff.policy_version,
        source
            .handoff
            .compiler_inputs
            .n2048_candidate_asset()
            .ok_or("learned asset missing")?
            .clone(),
    )?;
    if serde_json::to_vec(transition.next_inputs())?
        != serde_json::to_vec(&source.handoff.compiler_inputs)?
        || transition.next_phenotype().phenotype_hash() != source.handoff.actor.phenotype_hash
    {
        return Err("new-life compiler/topology differs from terminal learned heads".into());
    }
    if retained_neural {
        neural_inheritance::RetainedNeural::read(source)?;
    } else if let Some(reference) = saved.creatures[0]
        .gpu_brain
        .as_ref()
        .and_then(|brain| brain.live_structural_topology.as_ref())
    {
        let entry = saved
            .assets
            .entries
            .iter()
            .find(|entry| entry.asset_id == reference.asset_id)
            .ok_or("source live topology asset missing")?;
        let bytes = source
            .life
            .as_ref()
            .ok_or("source private life missing")?
            .asset_bytes(&entry.relative_path)?;
        let topology: alife_gpu_backend::GpuLiveTopologyCheckpointV1 =
            serde_json::from_slice(&bytes)?;
        topology.validate_for_capacity(&capacity)?;
        require_canonical_private_topology(&topology.v11_checkpoint, transition.next_phenotype())?;
    }
    Ok((game, transition))
}

fn require_canonical_private_topology(
    state: &alife_gpu_backend::GpuV11Checkpoint,
    phenotype: &alife_core::BrainPhenotype,
) -> Result<()> {
    let canonical = alife_gpu_backend::GpuV11Checkpoint::canonical_for_phenotype(phenotype)?;
    if !state.sparse_spans.is_empty()
        || state.pending_lifetime_synapse.is_some()
        || state.structural.connection_count() != 0
        || state.dendritic_branches != canonical.dendritic_branches
    {
        return Err("new life rejects acquired structural topology outside the exact global optimizer coordinates; preserve the terminal source".into());
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Full native source admission and canonical compatibility check, without a
/// device, seed consumption, output creation or training.
pub fn check_new_individual_training_life(
    source_path: &Path,
    seed: u64,
    organism_id: u64,
) -> Result<serde_json::Value> {
    let source = read_current_training_checkpoint(source_path)?;
    let bytes = fs::read(source_path)?;
    let source_sha = source.admitted_source_sha256(&bytes)?;
    let (game, _) = prepare(&source, seed, organism_id)?;
    let seeds = source.new_life_seeds(seed, organism_id)?;
    if fs::read(source_path)? != bytes {
        return Err("terminal source changed during new-life preflight".into());
    }
    Ok(
        serde_json::json!({"kind":"explicit-new-life-preflight-v1", "source_current_sha256":source_sha,
        "compatible":true,"policy_version":source.handoff.policy_version,
        "actor_optimizer_step":source.handoff.actor.optimizer_step,
        "value_optimizer_step":source.handoff.value.optimizer_step,
        "world_seed":game.world.seed(),"organism_id":organism_id,"initialization_nonce":seed+1,
        "planned_pending_range":seeds.pending_range,"historical_freshness":seeds.historical_freshness,
        "source_unchanged":true,"seeds_consumed":0,"world_ticks":0,"gpu_initialization":false}),
    )
}

/// Creates a separate store containing the exact terminal source, a new private
/// birth and its immutable baseline. No training or source publication occurs.
/// A partial output is intentionally never treated as an admitted store.
pub fn begin_new_individual_training_life(
    source_path: &Path,
    seed: u64,
    organism_id: u64,
    output: &Path,
) -> Result<serde_json::Value> {
    begin_mode(source_path, seed, organism_id, output, false)
}
pub fn begin_new_individual_training_life_with_retained_neural(
    source_path: &Path,
    seed: u64,
    organism_id: u64,
    output: &Path,
) -> Result<serde_json::Value> {
    begin_mode(source_path, seed, organism_id, output, true)
}
pub fn check_new_individual_retained_neural(
    source_path: &Path,
    seed: u64,
    organism_id: u64,
) -> Result<serde_json::Value> {
    let original = fs::read(source_path)?;
    let source = read_current_training_checkpoint(source_path)?;
    let source_sha = source.admitted_source_sha256(&original)?;
    prepare_mode(&source, seed, organism_id, true)?;
    let mut report = neural_inheritance::inspect(&source)?;
    if fs::read(source_path)? != original {
        return Err("terminal source changed during retained-neural preflight".into());
    }
    report["source_current_sha256"] = source_sha.into();
    report["organism_id"] = organism_id.into();
    report["world_seed"] = seed.into();
    report["policy_version"] = source.handoff.policy_version.into();
    report["actor_optimizer_step"] = source.handoff.actor.optimizer_step.into();
    report["value_optimizer_step"] = source.handoff.value.optimizer_step.into();
    Ok(report)
}
fn begin_mode(
    source_path: &Path,
    seed: u64,
    organism_id: u64,
    output: &Path,
    retained_neural: bool,
) -> Result<serde_json::Value> {
    let source = read_current_training_checkpoint(source_path)?;
    let source_path = source_path.canonicalize()?;
    let original = fs::read(&source_path)?;
    source.admitted_source_sha256(&original)?;
    let output = resolve_current_training_destination(&source_path, output)?;
    if fs::symlink_metadata(&output).is_ok() {
        return Err("new-life output must be a new isolated directory".into());
    }
    let (game, transition) = prepare_mode(&source, seed, organism_id, retained_neural)?;
    let source_sampling = source
        .saved_training_sampling()?
        .ok_or("terminal sampler missing")?;
    let sampling = super::super::cycle_sampling_config(
        seed + 1,
        source.handoff.policy_version,
        source_sampling.temperature,
    )?;
    if sampling.seed == source_sampling.seed {
        return Err("new-life sampling seed collides with the terminal stream".into());
    }
    // Device admission happens before any output. No CPU substitution is made.
    let backend = GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1())?;
    if fs::read(&source_path)? != original {
        return Err("terminal source changed before new-life construction".into());
    }
    fs::create_dir_all(output.parent().ok_or("new-life output has no parent")?)?;
    fs::create_dir(&output)?;
    write_new(&output.join("terminal-source.alife-checkpoint"), &original)?;
    let life = initialize(&source, game, &transition, sampling, backend, &output)?;
    let next = if retained_neural {
        let (life, proof) = neural_inheritance::inherit(&source, life)?;
        let root = output.join("inherited-life");
        fs::create_dir(&root)?;
        super::life::materialize(&life, &root)?;
        source.seal_new_life_with_retained_neural(&original, seed, organism_id, life, proof)?
    } else {
        source.seal_new_life(&original, seed, organism_id, life)?
    };
    let bytes = next.encode()?;
    // Full ordinary native/private admission precedes either published Current.
    admit_current_training_checkpoint(next)?;
    if fs::read(&source_path)? != original {
        return Err("terminal source changed during new-life admission".into());
    }
    write_new(&output.join("new-life-baseline.alife-checkpoint"), &bytes)?;
    // Ordinary resident stores intentionally admit only their own files. Keep
    // the immutable source/birth archives and construction assets outside it.
    let store = output.join("store");
    fs::create_dir(&store)?;
    write_new(&store.join("current.alife-checkpoint"), &bytes)?;
    let mut report = inspect_current_training_checkpoint(&store.join("current.alife-checkpoint"))?;
    report["kind"] = "explicit-new-individual-training-life-created-v1".into();
    report["source_unchanged"] = true.into();
    report["trained_rows"] = 0.into();
    report["new_private_life"] = true.into();
    report["neural_execution"] = false.into();
    report["gpu_initialization"] = true.into();
    report["output"] = serde_json::to_value(&output)?;
    report["store"] = serde_json::to_value(&store)?;
    if fs::read(&source_path)? != original {
        return Err("terminal source changed after new-life publication; preserve both outputs for inspection".into());
    }
    Ok(report)
}

fn initialize(
    source: &CurrentTrainingCheckpointV1,
    game: alife_world::CanonicalNewGame,
    transition: &N2048LivePolicyTransition,
    sampling: alife_gpu_backend::GpuTrainingSamplingConfig,
    backend: GpuClosedLoopBackend,
    output: &Path,
) -> Result<alife_training::CurrentLifeCheckpointV1> {
    let seed = game.world.seed();
    let organism_id = game.creatures[0].organism_id.raw();
    let assets = output.join("world-assets");
    fs::create_dir(&assets)?;
    let mut runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
        backend,
        game.world,
        seed,
        BrainScaleTier::Standard2048,
        source.handoff.compiler_inputs.sensor_profile(),
        alife_archive::LineageLibraryConfig::profile_default(output.join("lineage")),
        format!("explicit-training-new-life-{seed}-{organism_id}"),
        alife_core::ArchiveLearnedCapturePolicy::GeneticOnly,
        sampling,
    )?;
    runtime.install_live_foundation_policy(OrganismId(organism_id), transition)?;
    if serde_json::to_vec(runtime.live_foundation_compiler_inputs(OrganismId(organism_id))?)?
        != serde_json::to_vec(&source.handoff.compiler_inputs)?
    {
        return Err("installed new-life compiler differs from source".into());
    }
    let mut config =
        alife_world::RuntimeConfig::deterministic_default(seed, BrainScaleTier::Standard2048);
    config.features.gpu_backend_enabled = true;
    let base = alife_world::PortableSaveFile::from_headless_world(
        "explicit-training-new-life-v1",
        runtime.world(),
        config,
        alife_world::AssetManifest::empty(),
        game.creatures,
    )?;
    let save_path = output.join("world.json");
    runtime.attach_durable_checkpoint_boundary(&save_path, &assets, base)?;
    let exact = runtime.capture_portable_checkpoint()?;
    GpuDurableSaveManifest::publish_snapshot(&save_path, &assets, &exact)?;
    super::life::capture(output)
}

#[cfg(test)]
mod tests;
