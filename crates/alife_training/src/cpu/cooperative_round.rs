//! One explicit offline CPU round. Collectors never own a learner, and a result
//! is not an accepted native continuation or a request to replace any lineage.
use super::{collector_output, round_output, *};
use crate::{
    resolve_isolated_training_output, FoundationTrainingBackendV1, FoundationTrainingHandoffV1,
    PpoConfig, PpoUpdateReceipt,
};
use alife_core::PhenotypeCompiler;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const MAX_COLLECTORS: usize = 64;
const MAX_DECISIONS: u64 = 65_536;
const MAX_OPTIMIZER_UPDATES: usize = 65_536;
const MAX_SHARD_BYTES: u64 = 256 * 1024 * 1024;
const MAX_SOURCE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_COMMIT_BYTES: u64 = 16 * 1024 * 1024;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn digest(domain: &[u8], value: &impl Serialize) -> Result<[u8; 32]> {
    let mut hash = Sha256::new();
    hash.update(domain);
    serde_json::to_writer(&mut hash, value)?;
    Ok(hash.finalize().into())
}
fn exact(a: &impl Serialize, b: &impl Serialize) -> Result<bool> {
    Ok(digest(b"ALIFE-CPU-ROUND-EXACT", a)? == digest(b"ALIFE-CPU-ROUND-EXACT", b)?)
}

/// The caller approves the exact assignment and raw collector-commit.json bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCooperativeRoundMemberV1 {
    pub assignment: CpuCollectorAssignmentV1,
    pub collector_commit_sha256: [u8; 32],
}

/// Fixed membership, sampling provenance and optimizer budget. Paths are supplied
/// separately; neither a directory scan nor an accepted-source selector fills it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCooperativeRoundPlanV1 {
    pub schema_version: u16,
    pub round_id: String,
    pub source_handoff_sha256: [u8; 32],
    pub snapshot: CpuCollectionSnapshotV1,
    pub config: PpoConfig,
    pub window_ticks: usize,
    pub burn_in: usize,
    /// Number of complete windows in each effective batch, as in the learner.
    pub effective_batch_size: usize,
    pub members: Vec<CpuCooperativeRoundMemberV1>,
    pub payload_sha256: [u8; 32],
}
impl CpuCooperativeRoundPlanV1 {
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        round_id: String,
        source_handoff_sha256: [u8; 32],
        snapshot: CpuCollectionSnapshotV1,
        config: PpoConfig,
        window_ticks: usize,
        burn_in: usize,
        effective_batch_size: usize,
        mut members: Vec<CpuCooperativeRoundMemberV1>,
    ) -> Result<Self> {
        members.sort_by(|a, b| {
            a.assignment
                .collector_run_id
                .cmp(&b.assignment.collector_run_id)
        });
        let mut plan = Self {
            schema_version: 1,
            round_id,
            source_handoff_sha256,
            snapshot,
            config,
            window_ticks,
            burn_in,
            effective_batch_size,
            members,
            payload_sha256: [0; 32],
        };
        plan.payload_sha256 = plan.digest()?;
        plan.validate()?;
        Ok(plan)
    }
    fn digest(&self) -> Result<[u8; 32]> {
        digest(
            b"ALIFE-CPU-COOPERATIVE-ROUND-PLAN-V1",
            &(
                self.schema_version,
                &self.round_id,
                self.source_handoff_sha256,
                &self.snapshot,
                self.config,
                self.window_ticks,
                self.burn_in,
                self.effective_batch_size,
                &self.members,
            ),
        )
    }
    pub(super) fn validate(&self) -> Result<()> {
        self.config.validate()?;
        if self.schema_version != 1
            || self.round_id.trim().is_empty()
            || self.round_id.len() > 256
            || self.source_handoff_sha256 == [0; 32]
            || self.payload_sha256 != self.digest()?
            || self.window_ticks == 0
            || self
                .window_ticks
                .checked_add(self.burn_in)
                .is_none_or(|n| n > crate::MAX_TRAINING_SEQUENCE_TICKS)
            || self.effective_batch_size == 0
            || self.config.value_coefficient <= 0.
            || self.config.epochs as usize > MAX_OPTIMIZER_UPDATES
            || self.members.is_empty()
            || self.members.len() > MAX_COLLECTORS
        {
            return Err(invalid("invalid or unbounded cooperative round plan").into());
        }
        let mut previous: Option<&str> = None;
        let mut decisions = 0u64;
        for member in &self.members {
            member.assignment.validate()?;
            let run = member.assignment.collector_run_id.as_str();
            if previous.is_some_and(|p| p >= run) || member.collector_commit_sha256 == [0; 32] {
                return Err(invalid("round members must be unique and canonically ordered").into());
            }
            previous = Some(run);
            decisions = decisions
                .checked_add(u64::from(
                    member.assignment.sampling_counter_end
                        - member.assignment.sampling_counter_start,
                ))
                .ok_or_else(|| invalid("round decision count overflow"))?;
        }
        if decisions > MAX_DECISIONS {
            return Err(invalid("round exceeds its decision bound").into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CpuCooperativeRoundInput {
    pub collector_run_id: String,
    pub directory: PathBuf,
}

/// A self-contained, recoverable optimizer result. Both checkpoints use the
/// existing handoff codec; private collector hosts remain in their own outputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCooperativeRoundResultV1 {
    pub schema_version: u16,
    pub mode: String,
    pub plan: CpuCooperativeRoundPlanV1,
    pub source_handoff: FoundationTrainingHandoffV1,
    pub output_handoff: FoundationTrainingHandoffV1,
    pub update: PpoUpdateReceipt,
    pub windows: usize,
    pub training_rows: usize,
    pub window_trajectories: Vec<CpuCollectorTrajectoryId>,
    pub payload_sha256: [u8; 32],
}
impl CpuCooperativeRoundResultV1 {
    fn digest(&self) -> Result<[u8; 32]> {
        digest(
            b"ALIFE-CPU-COOPERATIVE-ROUND-RESULT-V1",
            &(
                self.schema_version,
                &self.mode,
                &self.plan,
                &self.source_handoff,
                &self.output_handoff,
                &self.update,
                self.windows,
                self.training_rows,
                &self.window_trajectories,
            ),
        )
    }
    pub(super) fn validate(&self) -> Result<()> {
        self.plan.validate()?;
        let (mut trainer, _) = restore_source(&self.source_handoff, &self.plan)?;
        self.output_handoff.validate()?;
        let before = &self.source_handoff;
        let after = &self.output_handoff;
        let delta = after
            .actor
            .optimizer_step
            .checked_sub(before.actor.optimizer_step)
            .ok_or_else(|| invalid("round actor optimizer moved backwards"))?;
        let critic_delta = after
            .value
            .optimizer_step
            .checked_sub(before.value.optimizer_step);
        let batches = self.windows.div_ceil(self.plan.effective_batch_size);
        let max_updates = batches
            .checked_mul(self.plan.config.epochs as usize)
            .ok_or_else(|| invalid("round optimizer budget overflow"))?;
        if self.schema_version != 1
            || self.mode != "native_cpu_cooperative_round"
            || self.payload_sha256 != self.digest()?
            || self.windows == 0
            || self.windows > MAX_DECISIONS as usize
            || self.training_rows < self.windows
            || self.training_rows > MAX_DECISIONS as usize
            || max_updates > MAX_OPTIMIZER_UPDATES
            || self.window_trajectories.len() != self.windows
            || critic_delta != Some(delta)
            || self.update.collection_policy_version != self.plan.snapshot.policy_version
            || self.update.actor_optimizer_step != after.actor.optimizer_step
            || self.update.value_optimizer_step != after.value.optimizer_step
            || self.update.completed_epochs as usize != delta as usize / batches.max(1)
            || self.update.mean_kl_before_updates.len()
                != delta as usize + usize::from(self.update.stopped_for_kl)
            || (self.update.stopped_for_kl && delta as usize >= max_updates)
            || (!self.update.stopped_for_kl && delta as usize != max_updates)
            || self
                .update
                .mean_kl_before_updates
                .iter()
                .enumerate()
                .any(|(i, kl)| {
                    !kl.is_finite()
                        || ((*kl > self.plan.config.target_kl)
                            != (self.update.stopped_for_kl
                                && i + 1 == self.update.mean_kl_before_updates.len()))
                })
        {
            return Err(invalid("invalid cooperative optimizer result or budget").into());
        }
        let mut last_run: Option<&str> = None;
        for trajectory in &self.window_trajectories {
            let run = trajectory.collector_run_id.as_str();
            if last_run.is_some_and(|last| last > run)
                || !self
                    .plan
                    .members
                    .iter()
                    .any(|m| m.assignment.collector_run_id == run)
            {
                return Err(
                    invalid("result trajectory order or membership differs from plan").into(),
                );
            }
            last_run = Some(run);
        }
        if delta == 0 {
            if !self.update.stopped_for_kl || !exact(before, after)? {
                return Err(invalid("zero-update round must retain exact source handoff").into());
            }
            return Ok(());
        }
        if after.origin != FoundationTrainingBackendV1::NativeCpu
            || after.policy_version != self.plan.snapshot.policy_version
            || !exact(&before.actor.config, &after.actor.config)?
            || before.actor.stage_mask != after.actor.stage_mask
            || before.actor.weights.len() != after.actor.weights.len()
        {
            return Err(invalid("round changed optimizer configuration or policy identity").into());
        }
        for i in 0..before.actor.weights.len() {
            if before.actor.stage_mask.is_trainable(i) {
                if before.actor.update_ages[i].checked_add(delta)
                    != Some(after.actor.update_ages[i])
                {
                    return Err(invalid("round reset a trainable coordinate age").into());
                }
            } else if before.actor.weights[i].to_bits() != after.actor.weights[i].to_bits()
                || before.actor.first_moment[i].to_bits() != after.actor.first_moment[i].to_bits()
                || before.actor.second_moment[i].to_bits() != after.actor.second_moment[i].to_bits()
                || before.actor.update_ages[i] != after.actor.update_ages[i]
            {
                return Err(invalid("round changed a frozen coordinate").into());
            }
        }
        let asset = after
            .compiler_inputs
            .n2048_candidate_asset()
            .ok_or_else(|| invalid("round output lacks candidate asset"))?
            .clone();
        let (phenotype, expected_inputs) = compile_round_output(before, asset.clone())?;
        if !exact(&expected_inputs, &after.compiler_inputs)? {
            return Err(invalid("round changed inherited compiler inputs").into());
        }
        let mut checkpoint = after.actor.clone();
        checkpoint.phenotype_hash = before.actor.phenotype_hash;
        checkpoint.source_foundation_digest = before.actor.source_foundation_digest;
        trainer.restore_checkpoint(checkpoint)?;
        trainer.rebind_for_next_cohort(phenotype, asset)?;
        let before_stage = before
            .compiler_inputs
            .n2048_candidate_asset()
            .ok_or_else(|| invalid("round source lacks candidate asset"))?
            .manifest()
            .training_stage();
        let after_stage = after
            .compiler_inputs
            .n2048_candidate_asset()
            .ok_or_else(|| invalid("round output lacks candidate asset"))?
            .manifest()
            .training_stage();
        if !exact(&trainer.checkpoint(), &after.actor)? || before_stage != after_stage {
            return Err(invalid("round changed compiler semantics or training stage").into());
        }
        Ok(())
    }
}

fn compile_round_output(
    source: &FoundationTrainingHandoffV1,
    asset: alife_core::FoundationWeightAsset,
) -> Result<(
    alife_core::BrainPhenotype,
    alife_core::PhenotypeCompilerInputs,
)> {
    Ok(PhenotypeCompiler::compile_n2048_foundation_candidate(
        source.compiler_inputs.genome().clone(),
        source.compiler_inputs.development().clone(),
        asset,
    )?)
}

fn restore_source(
    source: &FoundationTrainingHandoffV1,
    plan: &CpuCooperativeRoundPlanV1,
) -> Result<(CpuFoundationTrainer, CpuValueHead)> {
    if source.origin != FoundationTrainingBackendV1::NativeCpu
        || source.policy_version.checked_add(1) != Some(plan.snapshot.policy_version)
    {
        return Err(invalid("round needs the exact prior CPU policy version").into());
    }
    let admitted = source.clone().admit()?;
    let trainer = CpuFoundationTrainer::from_checkpoint(
        admitted.phenotype,
        admitted.foundation,
        admitted.actor,
    )?;
    let value = CpuValueHead::from_checkpoint(admitted.value)?;
    if CpuCollectionSnapshotV1::capture(
        &trainer,
        &value,
        &source.compiler_inputs,
        plan.config,
        plan.snapshot.policy_version,
    )? != plan.snapshot
    {
        return Err(invalid("round source does not match frozen collection snapshot").into());
    }
    Ok((trainer, value))
}

struct RoundInputs {
    source_path: PathBuf,
    directories: Vec<PathBuf>,
}
impl RoundInputs {
    fn load_shards(&self, plan: &CpuCooperativeRoundPlanV1) -> Result<Vec<CpuCollectorShardV1>> {
        if collector_output::fingerprint(&self.source_path, false)?.sha256
            != plan.source_handoff_sha256
        {
            return Err(invalid("round source handoff bytes changed").into());
        }
        let mut shards = Vec::with_capacity(plan.members.len());
        for (member, directory) in plan.members.iter().zip(&self.directories) {
            let commit = directory.join(CPU_COLLECTOR_COMMIT_FILE);
            if collector_output::fingerprint(&commit, false)?.sha256
                != member.collector_commit_sha256
            {
                return Err(invalid("round collector commit bytes changed").into());
            }
            let shard = read_cpu_collector_output(directory)?;
            if collector_output::fingerprint(&commit, false)?.sha256
                != member.collector_commit_sha256
                || shard.assignment != member.assignment
                || shard.snapshot != plan.snapshot
            {
                return Err(
                    invalid("round shard assignment or frozen snapshot differs from plan").into(),
                );
            }
            shards.push(shard);
        }
        Ok(shards)
    }
    fn verify(&self, plan: &CpuCooperativeRoundPlanV1) -> Result<()> {
        // Revalidate all private/source artifacts too, not just the shard seals.
        for shard in self.load_shards(plan)? {
            drop(shard);
        }
        Ok(())
    }
}

/// Execute at most the fixed plan's PPO budget using locally owned optimizers.
/// Bad evidence cannot mutate caller state. Retrying a committed/completed staged
/// result never learns again. Caller exclusively owns `output`; care scheduling,
/// seed reservation, prior-cache isolation and promotion are outside this API.
pub fn run_cpu_cooperative_round(
    source_handoff: &Path,
    plan: &CpuCooperativeRoundPlanV1,
    collector_directories: &[CpuCooperativeRoundInput],
    output: &Path,
) -> Result<CpuCooperativeRoundResultV1> {
    plan.validate()?;
    if collector_directories.len() != plan.members.len() {
        return Err(invalid("round collector inventory is incomplete").into());
    }
    let mut supplied = BTreeMap::new();
    let mut roots = BTreeSet::new();
    for input in collector_directories {
        let directory = collector_output::owned_root(&input.directory)?;
        if supplied
            .insert(input.collector_run_id.as_str(), directory.clone())
            .is_some()
            || !roots.insert(directory)
        {
            return Err(invalid("round collector inventory contains duplicates").into());
        }
    }
    let directories = plan
        .members
        .iter()
        .map(|member| {
            supplied
                .get(member.assignment.collector_run_id.as_str())
                .cloned()
                .ok_or_else(|| invalid("round is missing a planned collector"))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let source_path = source_handoff.canonicalize()?;
    let mut preserved = directories.iter().map(PathBuf::as_path).collect::<Vec<_>>();
    preserved.push(&source_path);
    let output = resolve_isolated_training_output(output, preserved)?;
    if fs::symlink_metadata(&source_path)?.len() > MAX_SOURCE_BYTES {
        return Err(invalid("round source exceeds byte limit").into());
    }
    // Bound the aggregate before materializing any replay arrays.
    let mut shard_bytes = 0u64;
    for (member, directory) in plan.members.iter().zip(&directories) {
        let commit_path = directory.join(CPU_COLLECTOR_COMMIT_FILE);
        if collector_output::fingerprint(&commit_path, false)?.sha256
            != member.collector_commit_sha256
        {
            return Err(invalid("round collector commit differs from plan").into());
        }
        let commit: CpuCollectorOutputCommit =
            collector_output::read_json(&commit_path, MAX_COMMIT_BYTES)?;
        let bytes = commit
            .artifacts
            .get(CPU_COLLECTOR_SHARD_FILE)
            .ok_or_else(|| invalid("round collector lacks shard artifact"))?
            .encoded_bytes;
        if fs::symlink_metadata(directory.join(CPU_COLLECTOR_SHARD_FILE))?.len() != bytes {
            return Err(invalid("round shard byte length changed").into());
        }
        shard_bytes = shard_bytes
            .checked_add(bytes)
            .filter(|n| *n <= MAX_SHARD_BYTES)
            .ok_or_else(|| invalid("round exceeds aggregate shard byte limit"))?;
    }
    let inputs = RoundInputs {
        source_path,
        directories,
    };
    let source: FoundationTrainingHandoffV1 =
        collector_output::read_json(&inputs.source_path, MAX_SOURCE_BYTES)?;
    let (mut trainer, mut value) = restore_source(&source, plan)?;
    let shards = inputs.load_shards(plan)?;
    let assignments = plan
        .members
        .iter()
        .map(|m| m.assignment.clone())
        .collect::<Vec<_>>();
    let cohort = admit_cpu_collector_shards(
        &trainer,
        &value,
        &source.compiler_inputs,
        plan.config,
        plan.snapshot.policy_version,
        &assignments,
        shards,
        plan.window_ticks,
        plan.burn_in,
    )?;
    let windows = cohort.windows().len();
    let training_rows = cohort
        .windows()
        .iter()
        .try_fold(0usize, |n, w| n.checked_add(w.actor.batch.len()))
        .ok_or_else(|| invalid("round training row count overflow"))?;
    if windows == 0 || windows > MAX_DECISIONS as usize || training_rows > MAX_DECISIONS as usize {
        return Err(invalid("round has no loss rows or exceeds its window bound").into());
    }
    let max_updates = windows
        .div_ceil(plan.effective_batch_size)
        .checked_mul(plan.config.epochs as usize)
        .filter(|n| *n <= MAX_OPTIMIZER_UPDATES)
        .ok_or_else(|| invalid("round exceeds its optimizer update bound"))?;
    let max_updates = u32::try_from(max_updates)?;
    if source
        .actor
        .optimizer_step
        .checked_add(max_updates)
        .is_none()
        || source
            .value
            .optimizer_step
            .checked_add(max_updates)
            .is_none()
    {
        return Err(invalid("round optimizer budget would overflow retained integer steps").into());
    }
    let window_trajectories = cohort.window_trajectories().to_vec();
    inputs.verify(plan)?;
    fs::create_dir_all(&output)?;
    if let Some(result) = round_output::recover_round_result(&output, plan)? {
        inputs.verify(plan)?;
        return Ok(result);
    }
    let update = cohort.train(
        &mut trainer,
        &mut value,
        &source.compiler_inputs,
        plan.config,
        plan.effective_batch_size,
    )?;
    let output_handoff = if trainer.optimizer_step() == source.actor.optimizer_step {
        source.clone()
    } else {
        let stage = source
            .compiler_inputs
            .n2048_candidate_asset()
            .ok_or_else(|| invalid("round source has no candidate asset"))?
            .manifest()
            .training_stage();
        let asset = trainer.export_candidate(stage)?;
        let (phenotype, compiler_inputs) = compile_round_output(&source, asset.clone())?;
        trainer.rebind_for_next_cohort(phenotype, asset)?;
        FoundationTrainingHandoffV1::new(
            FoundationTrainingBackendV1::NativeCpu,
            plan.snapshot.policy_version,
            compiler_inputs,
            trainer.checkpoint(),
            value.checkpoint(),
        )?
    };
    let mut result = CpuCooperativeRoundResultV1 {
        schema_version: 1,
        mode: "native_cpu_cooperative_round".into(),
        plan: plan.clone(),
        source_handoff: source,
        output_handoff,
        update,
        windows,
        training_rows,
        window_trajectories,
        payload_sha256: [0; 32],
    };
    result.payload_sha256 = result.digest()?;
    result.validate()?;
    inputs.verify(plan)?;
    let result = round_output::publish_round_result(&output, &result)?;
    inputs.verify(plan)?;
    Ok(result)
}
