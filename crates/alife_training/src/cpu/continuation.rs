//! Native CPU continuation with shared admission, full optimizer and export semantics.
use super::*;
use crate::{
    resolve_isolated_training_output, AdamWConfig, FoundationSerialHandoffBindingV1,
    FoundationTrainerCheckpoint, FoundationTrainingBackendV1, FoundationTrainingHandoffV1,
    PpoConfig, PpoValueHeadCheckpoint, StageTrainableMask, SERIAL_HANDOFF_BINDING_FILE,
    SERIAL_HANDOFF_FILE, SERIAL_HANDOFF_SOURCE_FILES,
};
use alife_core::*;
use alife_runtime::canonical_preparation::foundation_construction_development;
use alife_world::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn asset_digest(asset: &FoundationWeightAsset) -> String {
    asset
        .digest()
        .bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn write_json(path: impl AsRef<Path>, value: &impl serde::Serialize) -> Result<()> {
    // Recovery includes full neural/Adam and collector state. Keep every value,
    // but do not multiply indentation across these large retained artifacts.
    fs::write(path, serde_json::to_vec(value)?)?;
    Ok(())
}
pub struct CpuContinuationConfig {
    pub previous: Option<PathBuf>,
    pub output: PathBuf,
    pub seed: u64,
    pub ticks: usize,
    pub temperature: f32,
    pub fixture: bool,
}
pub type CpuContinuationResult<T> = Result<T>;

/// Consumer orchestration only: this interface cannot choose neural actions or
/// alter founder learning. The host checks all ordinary perception unchanged.
pub trait CpuEpisodeHooks {
    fn decision_budget(&self) -> Option<usize> {
        None
    }
    fn fixture_asset(&self, _seed: u64) -> Result<Option<FoundationWeightAsset>> {
        Ok(None)
    }
    fn admit(
        &mut self,
        _config: &CpuContinuationConfig,
        _source: Option<&Value>,
        _source_hashes: &BTreeMap<String, String>,
    ) -> Result<Value> {
        Ok(json!({"allocation":"explicit-caller","historical_freshness":"unknown"}))
    }
    fn validate_policy(&mut self, _founder: &CpuFoundationTrainer) -> Result<()> {
        Ok(())
    }
    fn prepare_world(&mut self, _world: &mut HeadlessWorld) -> Result<()> {
        Ok(())
    }
    fn before_step(
        &mut self,
        _host: &mut CpuCanonicalHost,
        _founder: &CpuFoundationTrainer,
    ) -> Result<()> {
        Ok(())
    }
    fn prepare_draft(
        &mut self,
        draft: PerceptionFrameDraft,
        _sequence: ExperienceSequenceId,
    ) -> std::result::Result<PerceptionFrameDraft, ScaffoldContractError> {
        Ok(draft)
    }
    fn after_step(
        &mut self,
        _host: &CpuCanonicalHost,
        _founder: &CpuFoundationTrainer,
        _step: &CpuCanonicalStep,
    ) -> Result<()> {
        Ok(())
    }
    fn finish_collection(
        &mut self,
        _host: &CpuCanonicalHost,
        _founder: &CpuFoundationTrainer,
        _rows: &BTreeMap<u64, Vec<CpuCollectedDecision>>,
        _train_rows: usize,
        _output: &Path,
    ) -> Result<CpuEpisodeReceipt> {
        Ok(CpuEpisodeReceipt {
            metadata: Value::Null,
            artifacts: Vec::new(),
        })
    }
}
pub struct CpuEpisodeReceipt {
    pub metadata: Value,
    pub artifacts: Vec<String>,
}
pub struct DefaultCpuEpisode;
impl CpuEpisodeHooks for DefaultCpuEpisode {}
fn compile_game(
    seed: u64,
    founder_seed: u64,
    asset: &FoundationWeightAsset,
) -> Result<(CanonicalNewGame, BrainPhenotype, PhenotypeCompilerInputs)> {
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    config.founder_seed_base = founder_seed;
    config.sensor_profile = asset.manifest().sensor_profile();
    let game = create_canonical_new_game_with_n2048_candidate(&config, asset)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("no canonical founder")?;
    let genome = record.phenotype().brain_genome.clone();
    let capacity = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        &genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    let (phenotype, inputs) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())?;
    Ok((game, phenotype, inputs))
}
fn fixture_asset(seed: u64) -> Result<FoundationWeightAsset> {
    let builtin = FoundationWeightAsset::builtin_n2048_v1(SensorProfile::GroundedObjectSlotsV1)?;
    let mut config = CanonicalNewGameConfig::phase3(seed, 1)?;
    config.brain_class = BrainScaleTier::Standard2048;
    let game = create_canonical_new_game_with_n2048_candidate(&config, &builtin)?;
    let record = game
        .world
        .organism_registry()
        .iter()
        .next()
        .ok_or("no fixture founder")?;
    let genome = &record.phenotype().brain_genome;
    let capacity = BrainCapacityClass::n2048();
    let development = foundation_construction_development(
        genome,
        &capacity,
        &record.phenotype().development_state_at(Tick::ZERO)?,
    )?;
    let p = PhenotypeCompiler::compile(genome, &capacity, &development, config.sensor_profile)?;
    Ok(FoundationWeightAsset::from_trained_weights(
        &p,
        p.synapses().iter().map(|s| s.genetic_weight()).collect(),
        TrainingStageManifest::new(1, 1, 1),
    )?)
}
/// Completed native collection, detached from offline actor/critic learning.
/// Fields stay private: callers can inspect checkpoints or consume a durable
/// shard seal; serial learning alone takes ownership of the optimizer objects.
pub struct CpuCollectedEpisode {
    args: CpuContinuationConfig,
    asset: FoundationWeightAsset,
    founder_seed: u64,
    source_version: u64,
    source_bytes: BTreeMap<String, Vec<u8>>,
    source_hashes: BTreeMap<String, String>,
    fixture: bool,
    lineage: Option<crate::native_receipt::Lineage>,
    parent_ref: Option<String>,
    seed_admission: Value,
    inputs: PhenotypeCompilerInputs,
    trainer: CpuFoundationTrainer,
    value: CpuValueHead,
    host: CpuCanonicalHost,
    before_actor: FoundationTrainerCheckpoint,
    before_value: PpoValueHeadCheckpoint,
    collection_start_tick: u64,
    collection_seconds: f64,
    all_started: Instant,
    valid_decisions: usize,
    consolidations: usize,
    sleeping: usize,
    decision_budget: Option<usize>,
    version: u64,
    rows: BTreeMap<u64, Vec<CpuCollectedDecision>>,
    episode_receipt: Option<CpuEpisodeReceipt>,
    episode_hashes: BTreeMap<String, String>,
}
impl CpuCollectedEpisode {
    pub fn actor_checkpoint(&self) -> FoundationTrainerCheckpoint {
        self.trainer.checkpoint()
    }
    pub fn value_checkpoint(&self) -> PpoValueHeadCheckpoint {
        self.value.checkpoint()
    }
    pub fn host_checkpoint(&self) -> Result<CpuCanonicalCheckpoint> {
        Ok(self.host.checkpoint(&self.trainer)?)
    }
    pub fn compiler_inputs(&self) -> &PhenotypeCompilerInputs {
        &self.inputs
    }
    pub fn policy_version(&self) -> u64 {
        self.version
    }
    pub fn decisions(&self) -> &BTreeMap<u64, Vec<CpuCollectedDecision>> {
        &self.rows
    }
    pub fn output(&self) -> &Path {
        &self.args.output
    }
    pub fn config(&self) -> PpoConfig {
        PpoConfig {
            temperature: self.args.temperature,
            ..Default::default()
        }
    }
    pub fn episode_metadata(&self) -> Option<&Value> {
        self.episode_receipt.as_ref().map(|r| &r.metadata)
    }
    fn collection_metadata(&self) -> Value {
        json!({"schema_version":1,"mode":"native_cpu_collect_only","backend":"native-cpu",
            "source_policy_version":self.source_version,"collection_policy_version":self.version,
            "offline_actor_optimizer_updates":0,"offline_value_optimizer_updates":0,
            "fixture":self.fixture,"world_seed":self.args.seed,
            "seed_admission":self.seed_admission,"episode":self.episode_metadata(),
            "completion_authority":"collector-commit.json; phase.txt is informational",
            "candidate_acceptance":"none; collection is not a trained continuation",
            "concurrency":"no concurrent reservation or shared live prior cache adapter"})
    }
    fn complete_rows(&self) -> Result<usize> {
        self.rows.iter().try_fold(0, |total, (id, rows)| {
            let record = self
                .host
                .world()
                .organism_registry()
                .get(OrganismId(*id))
                .ok_or("collector lost final organism record")?;
            Ok(total
                + rows
                    .len()
                    .saturating_sub(usize::from(record.lifecycle().is_alive())))
        })
    }
    fn verify_frozen_and_source(&self) -> Result<()> {
        if self.trainer.checkpoint() != self.before_actor
            || self.value.checkpoint() != self.before_value
        {
            return Err("offline actor/critic state changed during collection".into());
        }
        if let Some(previous) = &self.args.previous {
            for (name, expected) in &self.source_hashes {
                if hash(&fs::read(previous.join(name))?) != *expected {
                    return Err("source changed during CPU collection".into());
                }
            }
            if let Some(lineage) = &self.lineage {
                lineage.verify_at(previous)?;
            }
        }
        Ok(())
    }
    /// Publish only collection evidence. This does not create a trained source,
    /// advance a policy/optimizer, or accept/promote a candidate.
    pub fn seal(self, assignment: CpuCollectorAssignmentV1) -> Result<CpuCollectorOutputCommit> {
        self.verify_frozen_and_source()?;
        let checkpoint = self.host_checkpoint()?;
        if assignment.world_seed != self.args.seed
            || assignment.backend != FoundationTrainingBackendV1::NativeCpu
            || assignment.sampling_seed != checkpoint.sampling_seed
            || assignment.sampling_counter_start != 0
            || assignment.sampling_counter_end != checkpoint.sampling_counter
            || self
                .episode_metadata()
                .and_then(|m| m["lesson"].as_str())
                .is_some_and(|lesson| lesson != assignment.lesson)
        {
            return Err("collector assignment differs from actual native collection".into());
        }
        for (name, expected) in [
            (
                "collection-policy.alife-foundation",
                self.asset.encode_canonical()?,
            ),
            (
                "collection-compiler-inputs.json",
                serde_json::to_vec(&self.inputs)?,
            ),
            (
                "collection-actor-checkpoint.json",
                serde_json::to_vec(&self.before_actor)?,
            ),
            (
                "initial-value-checkpoint.json",
                serde_json::to_vec(&self.before_value)?,
            ),
            (
                "collector-checkpoint.json",
                serde_json::to_vec(&checkpoint)?,
            ),
            (
                "collection-metadata.json",
                serde_json::to_vec(&self.collection_metadata())?,
            ),
        ] {
            if fs::read(self.args.output.join(name))? != expected {
                return Err("collector recovery artifact changed before seal".into());
            }
        }
        for (name, expected) in &self.episode_hashes {
            if hash(&fs::read(self.args.output.join(name))?) != *expected {
                return Err("episode artifact changed before seal".into());
            }
        }
        for (name, expected) in &self.source_bytes {
            if fs::read(self.args.output.join("source-native").join(name))? != *expected {
                return Err("retained source copy changed before seal".into());
            }
        }
        let snapshot = CpuCollectionSnapshotV1::capture(
            &self.trainer,
            &self.value,
            &self.inputs,
            self.config(),
            self.version,
        )?;
        let mut trajectories = Vec::new();
        for (id, decisions) in self.rows {
            let record = self
                .host
                .world()
                .organism_registry()
                .get(OrganismId(id))
                .ok_or("collector lost final organism record")?
                .clone();
            trajectories.push(CpuCollectorTrajectoryV1 {
                final_record: record,
                decisions,
            });
        }
        let shard = CpuCollectorShardV1::seal(snapshot, assignment, trajectories)?;
        let commit = write_cpu_collector_output(&self.args.output, &shard)?;
        // The durable commit is authoritative even if an informational update
        // cannot be written after publication.
        let _ = fs::write(self.args.output.join("phase.txt"), "sealed-cpu-collector");
        Ok(commit)
    }
    fn training_windows(&self) -> Result<Vec<CpuResidualTrainingWindow>> {
        let config = self.config();
        let mut windows = Vec::new();
        for (id, trajectory) in &self.rows {
            let record = self
                .host
                .world()
                .organism_registry()
                .get(OrganismId(*id))
                .ok_or("collector lost terminal organism record")?;
            let terminal = (!record.lifecycle().is_alive()).then_some(record);
            if terminal.is_some() || trajectory.len() >= 2 {
                windows.extend(cpu_training_windows(
                    trajectory,
                    terminal,
                    &self.trainer,
                    &self.value,
                    record.phenotype(),
                    self.version,
                    config,
                    32,
                    8,
                )?);
            }
        }
        if windows.is_empty() {
            return Err(
            "no trainable complete transitions; retained source and collected prefix remain intact"
                .into(),
        );
        }
        Ok(windows)
    }
    fn finish_episode(
        &mut self,
        episode: &mut impl CpuEpisodeHooks,
        training_rows: usize,
    ) -> Result<()> {
        let episode_receipt = episode.finish_collection(
            &self.host,
            &self.trainer,
            &self.rows,
            training_rows,
            &self.args.output,
        )?;
        let mut episode_hashes = BTreeMap::new();
        for name in &episode_receipt.artifacts {
            if Path::new(name).components().count() != 1
                || !matches!(
                    Path::new(name).components().next(),
                    Some(std::path::Component::Normal(_))
                )
            {
                return Err("episode artifact must be a direct output file".into());
            }
            let path = self.args.output.join(name);
            if !fs::symlink_metadata(&path)?.is_file() {
                return Err("episode artifact must be a regular file".into());
            }
            episode_hashes.insert(name.clone(), hash(&fs::read(path)?));
        }
        self.episode_receipt = Some(episode_receipt);
        self.episode_hashes = episode_hashes;
        self.verify_frozen_and_source()?;
        Ok(())
    }
}

/// Collect under an explicitly supplied native source (or development fixture).
/// Waking acquired learning remains native and private; no offline update runs.
/// Hooks keep their existing reservation/prior behavior. This API does not supply
/// a concurrent reservation adapter or isolate the shared live prior cache.
pub fn collect_cpu_episode(
    args: CpuContinuationConfig,
    episode: &mut impl CpuEpisodeHooks,
) -> Result<CpuCollectedEpisode> {
    let mut collected = collect_cpu_episode_inner(args, episode)?;
    let rows = collected.complete_rows()?;
    if rows == 0 {
        return Err("no complete collection transitions; retained recovery remains intact".into());
    }
    collected.finish_episode(episode, rows)?;
    write_json(
        collected.args.output.join("collection-metadata.json"),
        &collected.collection_metadata(),
    )?;
    fs::write(
        collected.args.output.join("phase.txt"),
        "collected-cpu-awaiting-seal",
    )?;
    Ok(collected)
}

fn collect_cpu_episode_inner(
    mut args: CpuContinuationConfig,
    episode: &mut impl CpuEpisodeHooks,
) -> Result<CpuCollectedEpisode> {
    args.previous = args.previous.map(|p| p.canonicalize()).transpose()?;
    args.output =
        resolve_isolated_training_output(&args.output, args.previous.iter().map(PathBuf::as_path))?;
    if args.fixture == args.previous.is_some()
        || args.seed == 0
        || !(2..=2048).contains(&args.ticks)
        || !args.temperature.is_finite()
        || args.temperature <= 0.
        || args.output.exists()
    {
        return Err("invalid source, seed, dose, temperature or nonfresh output".into());
    }
    let decision_budget = episode.decision_budget();
    if decision_budget.is_some_and(|n| !(1..=128).contains(&n)) {
        return Err("care decision budget must be in 1..=128".into());
    }
    let all_started = Instant::now();
    let mut source_bytes = BTreeMap::new();
    let mut source_hashes = BTreeMap::new();
    let (asset, founder_seed, source_version, actor, value, source_receipt) = if let Some(
        previous,
    ) = &args.previous
    {
        let previous = previous.canonicalize()?;
        let receipt_names = ["cycle.json", "cpu-cycle.json", "group50.json"]
            .into_iter()
            .filter(|name| previous.join(name).is_file())
            .collect::<Vec<_>>();
        let receipt_name = match receipt_names.as_slice() {
            [name] => *name,
            _ => return Err("source requires exactly one native training receipt domain".into()),
        };
        let expected_phase = if receipt_name == "cpu-cycle.json" {
            "complete-fresh-cohort-cpu"
        } else {
            "next-cohort-admitted"
        };
        if receipt_name != "group50.json"
            && fs::read_to_string(previous.join("phase.txt"))?.trim() != expected_phase
        {
            return Err("previous source has no completed sealed-cohort marker".into());
        }
        for name in [
            "phase.txt",
            receipt_name,
            "trained.alife-foundation",
            "actor-checkpoint.json",
            "value-checkpoint.json",
        ] {
            if name == "phase.txt" && receipt_name == "group50.json" {
                continue;
            }
            let bytes = fs::read(previous.join(name))?;
            source_hashes.insert(name.to_string(), hash(&bytes));
            source_bytes.insert(name.to_string(), bytes);
        }
        let serial_handoff = if receipt_name == "cycle.json" {
            let binding_bytes = fs::read(previous.join(SERIAL_HANDOFF_BINDING_FILE)).map_err(|_| "serial source requires a validated portable handoff; run export_native_training_handoff SOURCE NEW_OUTPUT with the native verifier first")?;
            let handoff_bytes = fs::read(previous.join(SERIAL_HANDOFF_FILE))?;
            let binding: FoundationSerialHandoffBindingV1 = serde_json::from_slice(&binding_bytes)?;
            let pinned = SERIAL_HANDOFF_SOURCE_FILES
                .into_iter()
                .map(|name| (name.to_owned(), hash(&source_bytes[name])))
                .collect();
            binding.validate(&pinned, &hash(&handoff_bytes))?;
            let handoff: FoundationTrainingHandoffV1 = serde_json::from_slice(&handoff_bytes)?;
            handoff.validate()?;
            if handoff.origin != FoundationTrainingBackendV1::NativeGpu
                || serde_json::from_slice::<FoundationTrainerCheckpoint>(
                    &source_bytes["actor-checkpoint.json"],
                )? != handoff.actor
                || serde_json::from_slice::<PpoValueHeadCheckpoint>(
                    &source_bytes["value-checkpoint.json"],
                )? != handoff.value
                || handoff
                    .compiler_inputs
                    .n2048_candidate_asset()
                    .ok_or("missing serial handoff asset")?
                    .encode_canonical()?
                    != source_bytes["trained.alife-foundation"]
            {
                return Err("serial source differs from its validated typed handoff".into());
            }
            for (name, bytes) in [
                (SERIAL_HANDOFF_BINDING_FILE, binding_bytes),
                (SERIAL_HANDOFF_FILE, handoff_bytes),
            ] {
                source_hashes.insert(name.into(), hash(&bytes));
                source_bytes.insert(name.into(), bytes);
            }
            Some(handoff)
        } else {
            None
        };
        let receipt = crate::native_receipt::parse(&source_bytes[receipt_name])?;
        if serial_handoff.is_none()
            && (receipt["biological_objective_version"].as_u64() != Some(2)
                || receipt["critic_objective_version"].as_u64() != Some(1)
                || receipt["objective_state_reset"].as_bool() != Some(false))
        {
            return Err("source must retain biological objective2, full residual critic objective1, and objective_state_reset=false".into());
        }
        if receipt_name == "cpu-cycle.json" {
            if receipt["complete"] != true
                || receipt["mode"] != "native_cpu_fresh_cohort"
                || receipt["backend"] != "native-cpu"
                || receipt["receipt_filename"] != "cpu-cycle.json"
                || receipt["handoff_filename"] != "training-handoff.json"
            {
                return Err("CPU source lacks a sealed typed handoff receipt".into());
            }
            let bytes = fs::read(previous.join("training-handoff.json"))?;
            let handoff: FoundationTrainingHandoffV1 = serde_json::from_slice(&bytes)?;
            handoff.validate()?;
            for name in [
                "trained.alife-foundation",
                "actor-checkpoint.json",
                "value-checkpoint.json",
            ] {
                if receipt["native_output_sha256"][name].as_str()
                    != Some(hash(&source_bytes[name]).as_str())
                {
                    return Err("CPU source native file differs from its sealed receipt".into());
                }
            }
            if receipt["native_output_sha256"]["training-handoff.json"].as_str()
                != Some(hash(&bytes).as_str())
                || handoff.origin != FoundationTrainingBackendV1::NativeCpu
                || serde_json::from_slice::<FoundationTrainerCheckpoint>(
                    &source_bytes["actor-checkpoint.json"],
                )? != handoff.actor
                || serde_json::from_slice::<PpoValueHeadCheckpoint>(
                    &source_bytes["value-checkpoint.json"],
                )? != handoff.value
                || receipt["policy_version"].as_u64() != Some(handoff.policy_version)
                || handoff
                    .compiler_inputs
                    .n2048_candidate_asset()
                    .ok_or("missing typed handoff asset")?
                    .encode_canonical()?
                    != source_bytes["trained.alife-foundation"]
            {
                return Err(
                    "CPU source typed handoff differs from its native founder/optimizer files"
                        .into(),
                );
            }
            source_hashes.insert("training-handoff.json".into(), hash(&bytes));
            source_bytes.insert("training-handoff.json".into(), bytes);
        }
        if receipt_name == "group50.json"
            && (receipt["schema"] != 1
                || receipt["complete"] != true
                || receipt["mode"] != "group50_shared_habitat"
                || receipt["population"] != 50
                || receipt["next_cohort_optimizer_rebound"] != true
                || receipt["actor_checkpoint_sha256"]
                    != hash(&source_bytes["actor-checkpoint.json"])
                || receipt["value_checkpoint_sha256"]
                    != hash(&source_bytes["value-checkpoint.json"]))
        {
            return Err("group source is not a sealed exact native handoff".into());
        }
        let asset =
            FoundationWeightAsset::decode_canonical(&source_bytes["trained.alife-foundation"])?;
        if serial_handoff.is_none()
            && receipt["new_asset_digest"].as_str() != Some(asset_digest(&asset).as_str())
        {
            return Err("source asset differs from its native receipt".into());
        }
        let seed = receipt["seed"].as_u64().ok_or("source seed is missing")?;
        if args.seed == seed {
            return Err(
                "new cohort seed is the retained source seed; allocate an explicit new seed".into(),
            );
        }
        let founder = receipt["founder_seed_base"]
            .as_u64()
            .filter(|s| *s != 0)
            .unwrap_or(seed);
        if receipt_name == "cycle.json"
            && (receipt["next_cohort_tick_captured"].as_bool() != Some(true)
                || receipt["next_cohort_optimizer_rebound"].as_bool() != Some(true))
        {
            return Err("native source is not a sealed next-cohort handoff".into());
        }
        let version = receipt["policy_version"]
            .as_u64()
            .ok_or("source policy version is missing")?;
        if serial_handoff
            .as_ref()
            .is_some_and(|h| h.policy_version != version)
        {
            return Err("serial policy version differs from validated handoff".into());
        }
        let actor: FoundationTrainerCheckpoint =
            serde_json::from_slice(&source_bytes["actor-checkpoint.json"])?;
        let value: PpoValueHeadCheckpoint =
            serde_json::from_slice(&source_bytes["value-checkpoint.json"])?;
        if value.last_updated_policy_version != Some(version) {
            return Err("source critic belongs to another policy version".into());
        }
        if serial_handoff.is_none()
            && (receipt["actor_optimizer_step"].as_u64() != Some(u64::from(actor.optimizer_step))
                || receipt["value_optimizer_step"].as_u64()
                    != Some(u64::from(value.optimizer_step)))
        {
            return Err("source optimizer ages differ from native receipt".into());
        }
        (
            asset,
            founder,
            version,
            Some(actor),
            Some(value),
            Some(receipt),
        )
    } else {
        (
            match episode.fixture_asset(args.seed)? {
                Some(asset) => asset,
                None => fixture_asset(args.seed)?,
            },
            args.seed,
            0,
            None,
            None,
            None,
        )
    };
    let mut lineage = source_receipt
        .as_ref()
        .map(|receipt| {
            crate::native_receipt::Lineage::load(
                receipt,
                args.previous.as_ref().expect("source receipt has path"),
            )
        })
        .transpose()?;
    let fixture = args.fixture
        || lineage
            .as_ref()
            .map(|lineage| lineage.contains_fixture())
            .transpose()?
            .unwrap_or(false);
    let parent_ref = if let Some(lineage) = lineage.as_ref() {
        let name = ["cycle.json", "cpu-cycle.json", "group50.json"]
            .into_iter()
            .find(|name| source_bytes.contains_key(*name))
            .ok_or("missing source receipt")?;
        Some(lineage.check_parent(&source_bytes[name])?)
    } else {
        None
    };
    let seed_admission = episode.admit(&args, source_receipt.as_ref(), &source_hashes)?;
    let (game, p, inputs) = compile_game(args.seed, founder_seed, &asset)?;
    let trainer = if let Some(actor) = actor {
        if actor
            .weights
            .iter()
            .zip(p.synapses())
            .any(|(w, s)| w.to_bits() != s.genetic_weight().to_bits())
        {
            return Err("native actor is not rebound to its retained founder; no weight repair/reset is permitted".into());
        }
        CpuFoundationTrainer::from_checkpoint(p, asset.clone(), actor)?
    } else {
        let mask = StageTrainableMask::from_synapse_indices(
            &p,
            &(0..p.synapses().len() as u32).collect::<Vec<_>>(),
        )?;
        CpuFoundationTrainer::new(p, asset.clone(), mask, AdamWConfig::default())?
    };
    let value = CpuValueHead::from_checkpoint(value.unwrap_or_else(|| PpoValueHeadCheckpoint {
        feature_count: trainer.phenotype().neuron_count(),
        optimizer_step: 0,
        parameters: vec![0.; (trainer.phenotype().neuron_count() as usize + 1) * 3],
        last_updated_policy_version: None,
    }))?;
    if value.checkpoint().feature_count != trainer.phenotype().neuron_count() {
        return Err("native critic width differs; resizing is not a continuation".into());
    }
    let version = source_version
        .checked_add(1)
        .ok_or("policy version overflow")?;
    episode.validate_policy(&trainer)?;
    let (mut host, ()) = CpuCanonicalHost::new_prepared_cohort(
        game,
        &trainer,
        (args.seed as u32) ^ ((args.seed >> 32) as u32),
        |world| episode.prepare_world(world),
    )?;
    let collection_start_tick = host.world().tick().raw();
    let before_actor = trainer.checkpoint();
    let before_value = value.checkpoint();
    // Admission, byte/checkpoint validation and world compilation all precede output ownership.
    fs::create_dir_all(args.output.parent().unwrap_or(Path::new(".")))?;
    fs::create_dir(&args.output)?;
    fs::write(args.output.join("phase.txt"), "collecting-canonical-cpu")?;
    if let Some(lineage) = &lineage {
        fs::create_dir(args.output.join("source-native"))?;
        lineage.write_to(&args.output.join("source-native"))?;
    }
    if let Some(lineage) = &mut lineage {
        let name = ["cycle.json", "cpu-cycle.json", "group50.json"]
            .into_iter()
            .find(|name| source_bytes.contains_key(*name))
            .ok_or("missing source receipt")?;
        lineage.retain_parent(source_bytes[name].clone())?;
        lineage.write_to(&args.output)?;
    }
    for (name, bytes) in &source_bytes {
        let dir = args.output.join("source-native");
        fs::create_dir_all(&dir)?;
        fs::write(dir.join(name), bytes)?;
    }
    fs::write(
        args.output.join("collection-policy.alife-foundation"),
        asset.encode_canonical()?,
    )?;
    write_json(args.output.join("collection-compiler-inputs.json"), &inputs)?;
    write_json(
        args.output.join("collection-actor-checkpoint.json"),
        &before_actor,
    )?;
    write_json(
        args.output.join("initial-value-checkpoint.json"),
        &before_value,
    )?;
    let collection_started = Instant::now();
    let mut rows = BTreeMap::<u64, Vec<CpuCollectedDecision>>::new();
    let mut consolidations = 0;
    let mut sleeping = 0;
    let world_tick_budget = decision_budget.map_or(args.ticks, |n| n * 8 + 4096);
    let mut captured = 0;
    let collection_result = (|| -> Result<()> {
        for _ in 0..world_tick_budget {
            episode.before_step(&mut host, &trainer)?;
            let step =
                host.step_with_preparation(&trainer, args.temperature, |draft, sequence| {
                    episode.prepare_draft(draft, sequence)
                })?;
            episode.after_step(&host, &trainer, &step)?;
            captured += step.decisions.len();
            consolidations += step.sleep_consolidations;
            sleeping += step.sleeping_organisms;
            for row in step.decisions {
                rows.entry(row.organism_id.raw()).or_default().push(row);
            }
            if decision_budget.is_some_and(|n| captured > n) {
                break;
            }
            if host
                .world()
                .organism_registry()
                .iter()
                .all(|r| !r.lifecycle().is_alive())
            {
                break;
            }
        }
        Ok(())
    })();
    let collection_seconds = collection_started.elapsed().as_secs_f64();
    let valid_decisions = rows.values().map(Vec::len).sum::<usize>();
    // A hook failure must not discard the last sealed private native state.
    // An unsealed output is retained for explicit inspection/reconciliation;
    // it is never a reusable shard or a trained continuation.
    let recovery_result = host
        .checkpoint(&trainer)
        .map_err(Box::<dyn Error>::from)
        .and_then(|checkpoint| {
            write_json(args.output.join("collector-checkpoint.json"), &checkpoint)
        });
    collection_result?;
    recovery_result?;
    if let Some(n) = decision_budget {
        if rows.len() != 1
            || valid_decisions != n + 1
            || host
                .world()
                .organism_registry()
                .iter()
                .any(|r| !r.lifecycle().is_alive())
        {
            return Err("care episode did not capture its complete decision dose and real bootstrap; retained source and collected prefix remain intact".into());
        }
    }
    Ok(CpuCollectedEpisode {
        args,
        asset,
        founder_seed,
        source_version,
        source_bytes,
        source_hashes,
        fixture,
        lineage,
        parent_ref,
        seed_admission,
        inputs,
        trainer,
        value,
        host,
        before_actor,
        before_value,
        collection_start_tick,
        collection_seconds,
        all_started,
        valid_decisions,
        consolidations,
        sleeping,
        decision_budget,
        version,
        rows,
        episode_receipt: None,
        episode_hashes: BTreeMap::new(),
    })
}

pub fn run_cpu_continuation(
    args: CpuContinuationConfig,
    episode: &mut impl CpuEpisodeHooks,
) -> Result<Value> {
    let mut collected = collect_cpu_episode_inner(args, episode)?;
    let config = collected.config();
    let windows = collected.training_windows()?;
    let training_rows = windows.iter().map(|w| w.actor.batch.len()).sum::<usize>();
    if collected
        .decision_budget
        .is_some_and(|n| training_rows != n)
    {
        return Err("care replay loss-row dose differs from requested decisions".into());
    }
    collected.finish_episode(episode, training_rows)?;
    let CpuCollectedEpisode {
        args,
        asset,
        founder_seed,
        source_version,
        source_hashes,
        fixture,
        lineage,
        parent_ref,
        seed_admission,
        mut trainer,
        mut value,
        host,
        before_actor,
        before_value,
        collection_start_tick,
        collection_seconds,
        all_started,
        valid_decisions,
        consolidations,
        sleeping,
        decision_budget,
        version,
        episode_receipt,
        episode_hashes,
        ..
    } = collected;
    let episode_receipt = episode_receipt.ok_or("collection finalization missing")?;
    let profile = trainer.phenotype().sensor_profile();
    let update_started = Instant::now();
    let update = train_cpu_residual_cohort(
        &mut trainer,
        &mut value,
        windows.len(),
        windows.len(),
        |i| Ok(windows[i].clone()),
        config,
        version,
    )?;
    let update_seconds = update_started.elapsed().as_secs_f64();
    let trained = trainer.export_candidate(asset.manifest().training_stage())?;
    let (_, next, next_inputs) = compile_game(args.seed, founder_seed, &trained)?;
    trainer.rebind_for_next_cohort(next, trained.clone())?;
    fs::write(
        args.output.join("trained.alife-foundation"),
        trained.encode_canonical()?,
    )?;
    write_json(args.output.join("compiler-inputs.json"), &next_inputs)?;
    let handoff = FoundationTrainingHandoffV1::new(
        FoundationTrainingBackendV1::NativeCpu,
        version,
        next_inputs.clone(),
        trainer.checkpoint(),
        value.checkpoint(),
    )?;
    write_json(args.output.join("training-handoff.json"), &handoff)?;
    write_json(
        args.output.join("actor-checkpoint.json"),
        &trainer.checkpoint(),
    )?;
    write_json(
        args.output.join("value-checkpoint.json"),
        &value.checkpoint(),
    )?;
    let output_hashes = [
        "training-handoff.json",
        "trained.alife-foundation",
        "actor-checkpoint.json",
        "value-checkpoint.json",
    ]
    .into_iter()
    .map(|name| Ok((name.to_owned(), hash(&fs::read(args.output.join(name))?))))
    .collect::<Result<BTreeMap<_, _>>>()?;
    let ancestry_hashes = source_hashes
        .iter()
        .filter(|(name, _)| {
            !matches!(
                name.as_str(),
                "phase.txt" | SERIAL_HANDOFF_FILE | SERIAL_HANDOFF_BINDING_FILE
            )
        })
        .map(|(name, hash)| (name.clone(), hash.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut receipt = json!({"schema_version":1,"complete":true,"mode":"native_cpu_fresh_cohort","receipt_filename":"cpu-cycle.json","handoff_filename":"training-handoff.json","native_output_sha256":output_hashes,"backend":"native-cpu","continuation_kind":"fresh-cohort-foundation-continuation","fixture":fixture,
  "biological_objective_version":2,"critic_objective_version":1,"objective_state_reset":false,"source_policy_version":source_version,"policy_version":version,
  "seed":args.seed,"founder_seed_base":founder_seed,"sensor_profile":profile,"sampling_temperature":args.temperature,
  "seed_admission":seed_admission,"source_native_sha256":source_hashes,
  "seed_provenance":{"source_sha256":ancestry_hashes,"native_source_receipt_ref":parent_ref,"seed_admission":seed_admission},
  "old_asset_digest":asset_digest(&asset),"new_asset_digest":asset_digest(&trained),"initial_actor_optimizer_step":before_actor.optimizer_step,"initial_value_optimizer_step":before_value.optimizer_step,
  "actor_optimizer_step":trainer.optimizer_step(),"value_optimizer_step":value.checkpoint().optimizer_step,"completed_epochs":update.completed_epochs,"stopped_for_kl":update.stopped_for_kl,
  "requested_world_ticks":args.ticks,"world_ticks_elapsed":host.world().tick().raw()-collection_start_tick,"collection_start_tick":collection_start_tick,"canonical_valid_decisions":valid_decisions,"training_rows":training_rows,
  "sleeping_organism_ticks":sleeping,"sleep_consolidations":consolidations,"collection_seconds":collection_seconds,"update_seconds":update_seconds,
  "elapsed_seconds":all_started.elapsed().as_secs_f64(),"private_individual_archive":"collector-checkpoint.json","private_archive_policy":"collection-policy.alife-foundation",
  "terminal_gpu_individual_restore_proven":false,"hardware":trainer.hardware_receipt()});
    if parent_ref.is_none() {
        receipt["seed_provenance"]
            .as_object_mut()
            .unwrap()
            .remove("native_source_receipt_ref");
    }
    receipt["consumer_build"] = json!({"git_head":option_env!("ALIFE_CONSUMER_GIT_HEAD"),"git_tree":option_env!("ALIFE_CONSUMER_GIT_TREE"),"executable_sha256":std::env::current_exe().ok().and_then(|p| fs::read(p).ok()).map(|bytes| hash(&bytes))});
    receipt["episode"] = episode_receipt.metadata;
    receipt["episode_artifact_sha256"] = json!(episode_hashes);
    receipt["actor_optimizer_updates"] =
        json!(trainer.optimizer_step() - before_actor.optimizer_step);
    receipt["value_optimizer_updates"] =
        json!(value.checkpoint().optimizer_step - before_value.optimizer_step);
    if let Some(n) = decision_budget {
        receipt["collection_limit"] = json!("waking-decisions-with-real-bootstrap");
        receipt["requested_world_ticks"] = Value::Null;
        receipt["requested_training_ticks"] = json!(n);
        receipt["training_ticks"] = json!(training_rows);
        receipt["captured_replay_rows"] = json!(valid_decisions);
        receipt["trained_replay_rows"] = json!(training_rows);
        receipt["bootstrap_replay_rows"] = json!(1);
    }
    // Verify the caller's immutable source after all collection/update/export work.
    if let Some(previous) = &args.previous {
        for (name, expected) in &source_hashes {
            if hash(&fs::read(previous.join(name))?) != *expected {
                return Err("source changed during CPU run".into());
            }
        }
        if let Some(lineage) = &lineage {
            lineage.verify_at(previous)?;
        }
    }
    fs::write(args.output.join("phase.txt"), "complete-fresh-cohort-cpu")?;
    write_json(args.output.join("cpu-cycle.json"), &receipt)?;
    // Return the sealed representation. serde_json's fast float parser can
    // round an in-memory f64 differently on its first text roundtrip; acceptance
    // compares two reads of the same authoritative bytes.
    Ok(crate::native_receipt::read(&args.output.join("cpu-cycle.json"))?.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_recovery_json_preserves_float_bits_integer_ages_and_strings() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Recovery {
            moments: Vec<Vec<f32>>,
            update_ages: Vec<u32>,
            description: String,
        }
        let state = Recovery {
            moments: vec![vec![-0.0, f32::MIN_POSITIVE, f32::MAX, -0.125]; 32],
            update_ages: vec![0, (1 << 24) + 17, u32::MAX],
            description: "spaces, \"quotes\", backslash \\ and newline\n stay intact".into(),
        };
        let path = std::env::temp_dir().join(format!(
            "alife-compact-recovery-{}.json",
            std::process::id()
        ));
        write_json(&path, &state).unwrap();
        let bytes = fs::read(&path).unwrap();
        fs::remove_file(path).unwrap();
        assert!(bytes.len() < serde_json::to_vec_pretty(&state).unwrap().len());
        let restored: Recovery = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored.update_ages, state.update_ages);
        assert_eq!(restored.description, state.description);
        for (before, after) in state
            .moments
            .iter()
            .flatten()
            .zip(restored.moments.iter().flatten())
        {
            assert_eq!(before.to_bits(), after.to_bits());
        }
    }
}
