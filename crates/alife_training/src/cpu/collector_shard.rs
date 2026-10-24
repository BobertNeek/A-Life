//! Sealed CPU collector evidence for one frozen central actor/critic snapshot.
//!
//! These are detached rollout records, not checkpoint continuation or creature
//! saves. Each collector keeps its private host/archive. The seal detects changed
//! evidence; it is not a signature or proof of an untrusted producer's history.
use super::*;
use crate::{FoundationTrainingBackendV1, PpoConfig, PpoUpdateReceipt};
use alife_core::{OrganismId, PhenotypeCompiler, PhenotypeCompilerInputs, Validate};
use alife_world::WorldOrganismRecord;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

fn digest(domain: &[u8], value: &impl Serialize) -> Result<[u8; 32], TrainingError> {
    let mut hash = Sha256::new();
    hash.update(domain);
    // Stream large replay records rather than make another complete byte copy.
    serde_json::to_writer(&mut hash, value).map_err(|_| invalid())?;
    Ok(hash.finalize().into())
}

/// Exact collection boundary, including both Adam histories and integer ages.
/// Objective versions are the existing biological-v2 / residual-critic-v1 pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCollectionSnapshotV1 {
    pub policy_version: u64,
    pub biological_objective_version: u16,
    pub critic_objective_version: u16,
    pub actor_sha256: [u8; 32],
    pub critic_sha256: [u8; 32],
    pub compiler_sha256: [u8; 32],
    pub ppo_config_sha256: [u8; 32],
}
impl CpuCollectionSnapshotV1 {
    pub fn capture(
        trainer: &CpuFoundationTrainer,
        value: &CpuValueHead,
        inputs: &PhenotypeCompilerInputs,
        config: PpoConfig,
        policy_version: u64,
    ) -> Result<Self, TrainingError> {
        config.validate()?;
        let actor = trainer.checkpoint();
        let critic = value.checkpoint();
        let compiled =
            PhenotypeCompiler::compile_validated(inputs, &alife_core::BrainCapacityClass::n2048())?;
        let asset = inputs.n2048_candidate_asset().ok_or_else(invalid)?;
        actor.validate_for(&compiled, asset.digest())?;
        if policy_version == 0
            || critic
                .last_updated_policy_version
                .is_some_and(|last| policy_version <= last)
            || critic.feature_count != compiled.neuron_count()
            || compiled.phenotype_hash() != trainer.phenotype().phenotype_hash()
            || actor
                .weights
                .iter()
                .zip(compiled.synapses())
                .any(|(w, s)| w.to_bits() != s.genetic_weight().to_bits())
        {
            return Err(invalid());
        }
        Ok(Self {
            policy_version,
            biological_objective_version: 2,
            critic_objective_version: 1,
            actor_sha256: digest(b"ALIFE-COLLECTOR-ACTOR-V1", &actor)?,
            critic_sha256: digest(b"ALIFE-COLLECTOR-CRITIC-V1", &critic)?,
            compiler_sha256: digest(b"ALIFE-COLLECTOR-COMPILER-V1", inputs)?,
            ppo_config_sha256: digest(b"ALIFE-COLLECTOR-PPO-CONFIG-V1", &config)?,
        })
    }
}

/// Learner-approved assignment. A run ID names one collector lifetime namespace;
/// it must never be reused for another shard, even when organism IDs restart.
/// The episode digest binds caller-owned lesson/environment/sampling settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCollectorAssignmentV1 {
    pub collector_run_id: String,
    pub backend: FoundationTrainingBackendV1,
    pub producer_revision: String,
    pub world_seed: u64,
    pub sampling_seed: u32,
    pub sampling_counter_start: u32,
    /// Exclusive end, covering every decision including bootstrap observations.
    pub sampling_counter_end: u32,
    pub lesson: String,
    pub episode_config_sha256: [u8; 32],
}
impl CpuCollectorAssignmentV1 {
    fn validate(&self) -> Result<(), TrainingError> {
        let text = |s: &str| !s.trim().is_empty() && s.len() <= 256;
        if !text(&self.collector_run_id)
            || !text(&self.producer_revision)
            || !text(&self.lesson)
            || self.world_seed == 0
            || self.sampling_counter_start >= self.sampling_counter_end
            || self.backend != FoundationTrainingBackendV1::NativeCpu
            || self.episode_config_sha256 == [0; 32]
        {
            return Err(invalid());
        }
        Ok(())
    }
}

/// Full per-life decisions plus its actual final biology. Living records require
/// the last actual decision as bootstrap; dead records end without bootstrapping.
/// Existing replay/context/patch types retain private acquired weights per tick.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCollectorTrajectoryV1 {
    pub final_record: WorldOrganismRecord,
    pub decisions: Vec<CpuCollectedDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuCollectorTrajectoryId {
    pub collector_run_id: String,
    pub organism_id: OrganismId,
}

/// One complete, sealed assignment. JSON serialization reuses native decisions;
/// derived GAE, advantages and critic targets are rebuilt by the central learner.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCollectorShardV1 {
    pub schema_version: u16,
    pub snapshot: CpuCollectionSnapshotV1,
    pub assignment: CpuCollectorAssignmentV1,
    pub trajectories: Vec<CpuCollectorTrajectoryV1>,
    pub payload_sha256: [u8; 32],
}
impl CpuCollectorShardV1 {
    pub fn seal(
        snapshot: CpuCollectionSnapshotV1,
        assignment: CpuCollectorAssignmentV1,
        trajectories: Vec<CpuCollectorTrajectoryV1>,
    ) -> Result<Self, TrainingError> {
        assignment.validate()?;
        if trajectories.is_empty() {
            return Err(invalid());
        }
        let mut shard = Self {
            schema_version: 1,
            snapshot,
            assignment,
            trajectories,
            payload_sha256: [0; 32],
        };
        shard.payload_sha256 = shard.digest()?;
        Ok(shard)
    }
    fn digest(&self) -> Result<[u8; 32], TrainingError> {
        digest(
            b"ALIFE-CPU-COLLECTOR-SHARD-V1",
            &(
                self.schema_version,
                &self.snapshot,
                &self.assignment,
                &self.trajectories,
            ),
        )
    }
}

/// Immutable admitted windows. The key accompanies every window; local organism
/// IDs can coincide across collectors because GAE stays per complete private life.
pub struct AdmittedCpuCollectorCohort {
    snapshot: CpuCollectionSnapshotV1,
    windows: Vec<CpuResidualTrainingWindow>,
    window_trajectories: Vec<CpuCollectorTrajectoryId>,
}
impl AdmittedCpuCollectorCohort {
    pub fn windows(&self) -> &[CpuResidualTrainingWindow] {
        &self.windows
    }
    pub fn window_trajectories(&self) -> &[CpuCollectorTrajectoryId] {
        &self.window_trajectories
    }
    /// Recheck the exact central state immediately before using the existing
    /// learner. An earlier admission cannot authorize a later changed snapshot.
    pub fn train(
        self,
        trainer: &mut CpuFoundationTrainer,
        value: &mut CpuValueHead,
        inputs: &PhenotypeCompilerInputs,
        config: PpoConfig,
        effective_batch_size: usize,
    ) -> Result<PpoUpdateReceipt, TrainingError> {
        let version = self.snapshot.policy_version;
        if CpuCollectionSnapshotV1::capture(trainer, value, inputs, config, version)?
            != self.snapshot
        {
            return Err(invalid());
        }
        train_cpu_residual_cohort(
            trainer,
            value,
            self.windows.len(),
            effective_batch_size,
            |i| Ok(self.windows[i].clone()),
            config,
            version,
        )
    }
}

/// Admit the complete learner-approved collector inventory before any mutation.
/// Both shards and runs are unique; repeated local organism IDs in different
/// runs remain separate. Assignment metadata is compared to the caller's plan.
/// This in-memory first slice supports CPU-only homogeneous n2048 snapshots.
#[allow(clippy::too_many_arguments)]
pub fn admit_cpu_collector_shards(
    trainer: &CpuFoundationTrainer,
    value: &CpuValueHead,
    inputs: &PhenotypeCompilerInputs,
    config: PpoConfig,
    version: u64,
    assignments: &[CpuCollectorAssignmentV1],
    shards: Vec<CpuCollectorShardV1>,
    window_ticks: usize,
    burn_in: usize,
) -> Result<AdmittedCpuCollectorCohort, TrainingError> {
    let snapshot = CpuCollectionSnapshotV1::capture(trainer, value, inputs, config, version)?;
    if assignments.is_empty()
        || assignments.len() != shards.len()
        || window_ticks == 0
        || window_ticks
            .checked_add(burn_in)
            .is_none_or(|n| n > crate::MAX_TRAINING_SEQUENCE_TICKS)
    {
        return Err(invalid());
    }
    let mut runs = BTreeSet::new();
    for assignment in assignments {
        assignment.validate()?;
        if !runs.insert(&assignment.collector_run_id) {
            return Err(invalid());
        }
    }
    let mut seen = BTreeSet::new();
    let mut sampling_evidence = BTreeSet::new();
    let mut cohort = AdmittedCpuCollectorCohort {
        snapshot: snapshot.clone(),
        windows: Vec::new(),
        window_trajectories: Vec::new(),
    };
    for shard in shards {
        if shard.schema_version != 1
            || shard.snapshot != snapshot
            || shard.payload_sha256 != shard.digest()?
            || !assignments.contains(&shard.assignment)
            || !seen.insert(shard.assignment.collector_run_id.clone())
            || shard.trajectories.is_empty()
        {
            return Err(invalid());
        }
        let mut organisms = BTreeSet::new();
        let mut counters = BTreeSet::new();
        for trajectory in shard.trajectories {
            let record = trajectory.final_record;
            record.validate_contract()?;
            // Use the same native founder/compiler check as a live CPU host.
            super::host::validate_record_founder(&record, trainer).map_err(|_| invalid())?;
            let id = record.organism_id();
            if !organisms.insert(id.raw()) || trajectory.decisions.is_empty() {
                return Err(invalid());
            }
            if record
                .lifecycle()
                .death_tick()
                .is_some_and(|death| death != record.biochemistry().tick)
            {
                return Err(invalid());
            }
            let mut previous_counter = None;
            for decision in &trajectory.decisions {
                let counter = decision.sampling_counter.ok_or_else(invalid)?;
                decision.patch.validate_contract()?;
                let neural = decision.patch.decision().neural_evidence()?;
                if decision.organism_id != id
                    || decision.patch.header().organism_id != id
                    || neural.phenotype_hash != trainer.phenotype().phenotype_hash()
                    || neural.candidate_index != decision.action.representative
                    || decision.patch.decision().policy_backend()
                        != alife_core::PolicyBackend::NeuralClosedLoopCpu
                    || decision.patch.outcome().outcome_tick > record.biochemistry().tick
                    || previous_counter.is_some_and(|c| counter <= c)
                    || !(shard.assignment.sampling_counter_start
                        ..shard.assignment.sampling_counter_end)
                        .contains(&counter)
                    || !counters.insert(counter)
                    || !sampling_evidence.insert((
                        shard.assignment.world_seed,
                        shard.assignment.sampling_seed,
                        shard.assignment.episode_config_sha256,
                        counter,
                    ))
                {
                    return Err(invalid());
                }
                previous_counter = Some(counter);
                // Include the living bootstrap decision, absent from actor loss.
                validate_decision(trainer, decision, &record, &shard.assignment, config)?;
            }
            let windows = cpu_training_windows(
                &trajectory.decisions,
                (!record.lifecycle().is_alive()).then_some(&record),
                trainer,
                value,
                record.phenotype(),
                version,
                config,
                window_ticks,
                burn_in,
            )?;
            for window in windows {
                super::cohort::validate_frozen_residual_window(
                    trainer, value, &window, config, version,
                )?;
                cohort.window_trajectories.push(CpuCollectorTrajectoryId {
                    collector_run_id: shard.assignment.collector_run_id.clone(),
                    organism_id: id,
                });
                cohort.windows.push(window);
            }
        }
        if counters.len() as u64
            != u64::from(shard.assignment.sampling_counter_end)
                - u64::from(shard.assignment.sampling_counter_start)
        {
            return Err(invalid());
        }
    }
    Ok(cohort)
}

fn validate_decision(
    trainer: &CpuFoundationTrainer,
    decision: &CpuCollectedDecision,
    record: &WorldOrganismRecord,
    assignment: &CpuCollectorAssignmentV1,
    config: PpoConfig,
) -> Result<(), TrainingError> {
    let sequence = TrainingSequence {
        phenotype_hash: trainer.phenotype().phenotype_hash(),
        initial: decision.initial.clone(),
        ticks: vec![decision.replay_tick.clone()],
        burn_in_ticks: 0,
        memory_candidate_gain: trainer
            .phenotype()
            .candidate_decoder()
            .memory_channel()
            .map_or(0., |m| m.max_candidate_gain()),
    };
    let replay = trainer.evaluate_replay(&sequence)?;
    let features = &replay.final_activations[0];
    let objective = cpu_ppo_objective(&replay.candidate_logits[0], &decision.action, 0., config)?;
    let mut sampled = sample_cpu_joint_action(
        trainer.phenotype(),
        decision.patch.pre_action().perception(),
        &replay.candidate_logits[0],
        assignment.sampling_seed,
        decision.sampling_counter.ok_or_else(invalid)?,
        config.temperature,
    )?;
    // Production replay permits the same small numerical tolerance as PPO's
    // existing frozen pass. Every mask/index/temperature must still match.
    sampled.old_joint_log_probability = decision.action.old_joint_log_probability;
    let physiology = decision
        .patch
        .outcome()
        .measured_physiology
        .as_ref()
        .ok_or_else(invalid)?;
    let receptors = physiology
        .before
        .neural_receptor_frame(record.phenotype())?;
    let credit = alife_core::OutcomeCreditPacket::from_sealed_patch(&decision.patch)?
        .with_biochemical_receptors(&receptors)?;
    let parameters = record.phenotype().brain_genome.plasticity_parameters();
    let profile = parameters
        .action_candidate_credit_profile()
        .map_or_else(|| parameters.receptor_profile(), |p| p.receptor_profile());
    let reward = profile.project(&credit.modulator().frame())?;
    if features.len() != decision.final_features.len()
        || features
            .iter()
            .zip(&decision.final_features)
            .any(|(a, b)| !b.is_finite() || (*a - *b).abs() > 2e-5 + 2e-4 * a.abs().max(b.abs()))
        || (objective.joint_log_probability - decision.action.old_joint_log_probability).abs()
            > 1e-3
        || sampled != decision.action
        || reward.to_bits() != decision.biological_reward.to_bits()
    {
        return Err(invalid());
    }
    Ok(())
}
