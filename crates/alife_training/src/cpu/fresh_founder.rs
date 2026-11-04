//! Construction-only disposable founders. No host, rollout, or prior is created.
//! Optimizer state is initialized, never advanced. This is not a trained life.
use super::{continuation::compile_game, CpuFoundationTrainer};
use crate::{
    native_receipt::sha256, resolve_isolated_training_output, AdamWConfig,
    FoundationTrainingBackendV1, FoundationTrainingHandoffV1, PpoValueHeadCheckpoint,
    StageTrainableMask,
};
use alife_core::{
    BrainCapacityClass, FoundationWeightAsset, PhenotypeCompiler, SensorProfile, Tick,
    TrainingStageManifest,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path},
};

pub const CPU_UNTRAINED_FOUNDER_MODE: &str = "native_cpu_untrained_founder";
const INITIAL_POLICY_VERSION: u64 = 1;
const PHASE: &str = "complete-fresh-cohort-cpu";
type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn construct(seed: u64, asset: &FoundationWeightAsset) -> Result<FoundationTrainingHandoffV1> {
    if asset.manifest().sensor_profile() != SensorProfile::GroundedTerrainVisionV1
        || asset.manifest().training_stage() != TrainingStageManifest::bootstrap()
    {
        return Err("untrained care founder requires the native bootstrap terrain asset".into());
    }
    let (game, phenotype, inputs) = compile_game(seed, seed, asset)?;
    if game.world.tick() != Tick::ZERO {
        return Err("founder construction unexpectedly advanced the world".into());
    }
    // Do not allow learned weights to be relabeled as an untrained source, even
    // if a caller rewrites its training-stage metadata and optimizer counters.
    let baseline = PhenotypeCompiler::compile_testing_procedural_baseline(
        inputs.genome(),
        &BrainCapacityClass::n2048(),
        inputs.development(),
        inputs.sensor_profile(),
    )?;
    if FoundationWeightAsset::from_phenotype_for_genetic_birth(&baseline)?.encode_canonical()?
        != asset.encode_canonical()?
    {
        return Err("untrained founder differs from its procedural birth weights".into());
    }
    let mask = StageTrainableMask::from_synapse_indices(
        &phenotype,
        &(0..phenotype.synapses().len() as u32).collect::<Vec<_>>(),
    )?;
    let trainer =
        CpuFoundationTrainer::new(phenotype, asset.clone(), mask, AdamWConfig::default())?;
    let value = PpoValueHeadCheckpoint {
        feature_count: trainer.phenotype().neuron_count(),
        optimizer_step: 0,
        parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
        // The existing typed handoff requires a positive policy identity and a
        // critic bound to it. This marks initialization at version 1, not an
        // optimizer event. The explicit receipt preserves that distinction.
        last_updated_policy_version: Some(INITIAL_POLICY_VERSION),
    };
    Ok(FoundationTrainingHandoffV1::new(
        FoundationTrainingBackendV1::NativeCpu,
        INITIAL_POLICY_VERSION,
        inputs,
        trainer.checkpoint(),
        value,
    )?)
}

fn receipt(
    seed: u64,
    handoff: &FoundationTrainingHandoffV1,
    pins: &BTreeMap<String, String>,
) -> Value {
    let asset = handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .expect("constructed candidate");
    let digest = asset
        .digest()
        .bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    json!({
        "schema_version":1,"complete":true,"mode":CPU_UNTRAINED_FOUNDER_MODE,
        "backend":"native-cpu","receipt_filename":"cpu-cycle.json",
        "handoff_filename":"training-handoff.json","native_output_sha256":pins,
        "fixture":true,"continuation_kind":"construction-only-untrained-founder",
        "initialization":"construction_only_zero_updates_v1",
        "policy_version":INITIAL_POLICY_VERSION,"source_policy_version":null,
        "critic_policy_binding":"initialized-at-version-1; no optimizer update",
        "biological_objective_version":2,"critic_objective_version":1,"objective_state_reset":false,
        "seed":seed,"founder_seed_base":seed,"new_asset_digest":digest,
        "sensor_profile":SensorProfile::GroundedTerrainVisionV1,
        "world_steps":0,"canonical_valid_decisions":0,"training_rows":0,
        "actor_optimizer_updates":0,"value_optimizer_updates":0,
        "actor_optimizer_step":0,"value_optimizer_step":0,
        "teacher_present":false,"semantic_prior_enabled":false,"training_enabled":false,
        "private_lifetime_continuation":"none; no private life advanced",
        "claim_scope":"disposable untrained CPU founder; no training, behavioral qualification, or generalization claim",
        "seed_provenance":{"source_sha256":{},"seed_admission":{"allocation":"construction-only","historical_freshness":"unasserted"}}
    })
}

pub(super) fn validate_source(
    receipt_value: &Value,
    handoff: &FoundationTrainingHandoffV1,
) -> Result<()> {
    let seed = receipt_value["seed"]
        .as_u64()
        .ok_or("untrained founder seed missing")?;
    let asset = handoff
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or("untrained candidate missing")?;
    let expected = construct(seed, asset)?;
    if serde_json::to_vec(handoff)? != serde_json::to_vec(&expected)? {
        return Err("untrained founder has changed weights, compiler inputs, optimizer state, or policy identity".into());
    }
    let pins: BTreeMap<String, String> =
        serde_json::from_value(receipt_value["native_output_sha256"].clone())?;
    if pins.len() != 4
        || [
            "training-handoff.json",
            "trained.alife-foundation",
            "actor-checkpoint.json",
            "value-checkpoint.json",
        ]
        .iter()
        .any(|key| !pins.contains_key(*key))
        || *receipt_value != receipt(seed, &expected, &pins)
    {
        return Err("untrained founder construction receipt is inconsistent".into());
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Seal a fresh, procedural, construction-only care founder in the existing CPU
/// source transport. An existing or interrupted directory is always preserved
/// and refused. This never collects a decision, initializes a semantic provider,
/// creates a canonical host, or calls training. The application supplies its
/// ordinary canonical birth asset; it is independently rechecked here.
pub fn initialize_cpu_untrained_founder(
    output: &Path,
    seed: u64,
    asset: FoundationWeightAsset,
) -> Result<Value> {
    // Never let lexical normalization hide an existing symlink behind a
    // nonexistent ancestor such as missing/../link/new-source.
    if output
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err("untrained founder output must not contain parent traversal".into());
    }
    for ancestor in output.ancestors() {
        if fs::symlink_metadata(ancestor).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return Err("untrained founder output must not traverse a symlink".into());
        }
    }
    let output = resolve_isolated_training_output(output, std::iter::empty::<&Path>())?;
    if fs::symlink_metadata(&output).is_ok() {
        return Err(
            "untrained founder requires a fresh output; existing evidence is never replaced".into(),
        );
    }
    let handoff = construct(seed, &asset)?;
    let mut files = BTreeMap::from([
        (
            "training-handoff.json".to_owned(),
            serde_json::to_vec(&handoff)?,
        ),
        (
            "trained.alife-foundation".to_owned(),
            asset.encode_canonical()?,
        ),
        (
            "actor-checkpoint.json".to_owned(),
            serde_json::to_vec(&handoff.actor)?,
        ),
        (
            "value-checkpoint.json".to_owned(),
            serde_json::to_vec(&handoff.value)?,
        ),
    ]);
    let pins = files
        .iter()
        .map(|(name, bytes)| (name.clone(), sha256(bytes)))
        .collect();
    let sealed = receipt(seed, &handoff, &pins);
    validate_source(&sealed, &handoff)?;
    files.insert("phase.txt".into(), PHASE.as_bytes().to_vec());
    // Exclusive ownership precedes every write; the receipt is the final seal.
    fs::create_dir_all(output.parent().ok_or("founder output has no parent")?)?;
    fs::create_dir(&output)?;
    for (name, bytes) in &files {
        write_new(&output.join(name), bytes)?;
    }
    write_new(
        &output.join("cpu-cycle.json"),
        &serde_json::to_vec(&sealed)?,
    )?;
    super::collector_output::sync_directory_ancestry(&output)?;
    let source = super::read_cpu_founder_source(&output)?;
    let source_sha256 = source
        .files
        .iter()
        .map(|(name, bytes)| (name.clone(), sha256(bytes)))
        .collect::<BTreeMap<_, _>>();
    Ok(
        json!({"mode":"construction_only_zero_updates_v1","source_sha256":source_sha256,
        "founder_seed":seed,"world_steps":0,"training_rows":0,
        "actor_optimizer_updates":0,"value_optimizer_updates":0,"actor_optimizer_step":0,"value_optimizer_step":0,
        "teacher_present":false,"semantic_prior_enabled":false}),
    )
}
