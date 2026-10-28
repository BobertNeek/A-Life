//! Exact same-individual continuation. Counts describe retained training, never
//! inferred policy ages or a freshly generated replacement organism.
use super::*;
use crate::FoundationContinuationKindV1;
use alife_core::{BrainCapacityClass, BrainPhenotype, PhenotypeCompiler, PhenotypeCompilerInputs};
use alife_gpu_backend::GpuTrainingSamplingConfig;
mod progress;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SameIndividualProgressV1 {
    pub session_nonce: u64,
    pub organism_id: u64,
    pub session_start_world_tick: u64,
    pub world_tick: u64,
    /// Explicit session-to-date retained PPO rows and completed PPO updates.
    pub trained_rows: u64,
    pub completed_updates: u64,
    pub sampling_temperature: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SameIndividualCurrentV1 {
    pub schema: u16,
    pub kind: String,
    pub counting_scope: String,
    pub progress: SameIndividualProgressV1,
    pub baseline_current_sha256: String,
    pub baseline_policy_version: u64,
    pub baseline_actor_optimizer_step: u32,
    pub baseline_value_optimizer_step: u32,
    pub baseline_trained_rows: u64,
    pub baseline_completed_updates: u64,
    pub total_trained_rows: u64,
    pub total_completed_updates: u64,
    pub compiler_inputs_sha256: String,
    pub actor_sha256: String,
    pub value_sha256: String,
    pub private_life_sha256: String,
    pub sampling_state: GpuTrainingSamplingConfig,
}

fn sha(value: &impl Serialize) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn private_asset(
    life: &CurrentLifeCheckpointV1,
    save: &alife_world::PortableSaveFile,
    asset_id: &str,
) -> Result<Vec<u8>> {
    let mut entries = save
        .assets
        .entries
        .iter()
        .filter(|e| e.asset_id == asset_id);
    let entry = entries
        .next()
        .ok_or("same-individual private asset missing")?;
    if entries.next().is_some() {
        return Err("same-individual private asset identity is ambiguous".into());
    }
    life.asset_bytes(&entry.relative_path)
}

/// Validates actual saved compiler/immutable-head coordinates, without a device
/// or any fresh-cohort compiler. Native admission additionally validates every
/// private asset, journal and mutable-state codec through the ordinary loader.
fn private_binding(
    life: &CurrentLifeCheckpointV1,
    handoff: &FoundationTrainingHandoffV1,
    progress: &SameIndividualProgressV1,
) -> Result<GpuTrainingSamplingConfig> {
    let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
    if save.creatures.len() != 1
        || save.creatures[0].organism_id.raw() != progress.organism_id
        || save.world.tick.raw() != progress.world_tick
    {
        return Err("same-individual organism or world tick differs from private save".into());
    }
    let creature = &save.creatures[0];
    let gpu = creature
        .gpu_brain
        .as_ref()
        .ok_or("same-individual private GPU state missing")?;
    if gpu.checkpoint_tick.raw() != progress.world_tick || gpu.organism_id != creature.organism_id {
        return Err("same-individual private GPU checkpoint boundary differs".into());
    }
    let inputs: PhenotypeCompilerInputs = serde_json::from_slice(&private_asset(
        life,
        &save,
        &gpu.phenotype_compiler_inputs.asset_id,
    )?)?;
    let phenotype: BrainPhenotype = serde_json::from_slice(&private_asset(
        life,
        &save,
        &gpu.immutable_phenotype.asset_id,
    )?)?;
    let compiled = PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048())?;
    if sha(&inputs)? != sha(&handoff.compiler_inputs)?
        || sha(&phenotype)? != sha(&compiled)?
        || gpu.phenotype_hash != compiled.phenotype_hash()
    {
        return Err("same-individual private compiler/phenotype differs from global heads".into());
    }
    let birth = creature
        .composite_genetics
        .as_ref()
        .ok_or("private birth provenance missing")?;
    let birth_asset = alife_core::FoundationWeightAsset::decode_canonical(&private_asset(
        life,
        &save,
        &birth.foundation_asset_id,
    )?)?;
    if inputs
        .live_policy()
        .ok_or("same-individual live policy marker missing")?
        .birth_foundation_digest
        != birth_asset.digest()
    {
        return Err("same-individual overlay changed private birth founder".into());
    }
    let sampling =
        private_sampling(life, &save)?.ok_or("exact private training sampler missing")?;
    if sampling.temperature.to_bits() != progress.sampling_temperature.to_bits() {
        return Err("same-individual sampling configuration differs from private state".into());
    }
    Ok(sampling)
}

fn private_sampling(
    life: &CurrentLifeCheckpointV1,
    save: &alife_world::PortableSaveFile,
) -> Result<Option<GpuTrainingSamplingConfig>> {
    if save.creatures.len() != 1 {
        return Err("same-individual sampler requires exactly one retained organism".into());
    }
    let Some(exact) = save.creatures[0]
        .gpu_brain
        .as_ref()
        .and_then(|gpu| gpu.exact_cognitive_state.as_ref())
    else {
        return Ok(None);
    };
    let packet: serde_json::Value =
        serde_json::from_slice(&private_asset(life, &save, &exact.asset_id)?)?;
    // The runtime owns this optional private byte packet. Reading the explicit
    // field here also allows this admission code to preserve older codecs.
    let Some(value) = packet
        .get("private_training_sampling")
        .filter(|v| !v.is_null())
    else {
        return Ok(None);
    };
    let sampling_bytes: Vec<u8> = serde_json::from_value(value.clone())?;
    if sampling_bytes.len() > 4096 {
        return Err("private training sampler exceeds its bound".into());
    }
    let sampling: GpuTrainingSamplingConfig = serde_json::from_slice(&sampling_bytes)?;
    sampling.validate()?;
    if sampling.demonstrator.is_some() {
        return Err("same-individual saved sampler contains a demonstrator".into());
    }
    Ok(Some(sampling))
}

impl SameIndividualCurrentV1 {
    pub fn validate(&self, current: &CurrentTrainingCheckpointV1) -> Result<()> {
        let p = &self.progress;
        progress::validate_session_counts(
            p.session_start_world_tick,
            p.world_tick,
            p.trained_rows,
            p.completed_updates,
            10_000_000,
        )?;
        progress::validate_actor_updates(
            self.baseline_actor_optimizer_step,
            current.handoff.actor.optimizer_step,
            p.completed_updates,
        )?;
        if self.counting_scope != "retained-same-individual-ppo-since-first-tag"
            || self.schema != 1
            || self.kind != "same-individual-ppo-v1"
            || current.handoff.continuation != FoundationContinuationKindV1::SameIndividual
            || p.session_nonce == 0
            || p.session_nonce == current.seed
            || p.organism_id == 0
            || p.world_tick < p.session_start_world_tick
            || !p.sampling_temperature.is_finite()
            || !(0.0001..=128.).contains(&p.sampling_temperature)
            || (p.completed_updates == 0) != (p.trained_rows == 0)
            || p.completed_updates > p.trained_rows
            || !current.seeds.contains(p.session_nonce)
            || current
                .seeds
                .pending_range
                .is_some_and(|[a, b]| a <= p.session_nonce && p.session_nonce <= b)
            || self.baseline_current_sha256.len() != 64
            || !self
                .baseline_current_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            || self
                .baseline_policy_version
                .checked_add(p.completed_updates)
                != Some(current.handoff.policy_version)
            || self.baseline_trained_rows.checked_add(p.trained_rows)
                != Some(self.total_trained_rows)
            || self
                .baseline_completed_updates
                .checked_add(p.completed_updates)
                != Some(self.total_completed_updates)
            || current.handoff.actor.optimizer_step < self.baseline_actor_optimizer_step
            || current.handoff.value.optimizer_step < self.baseline_value_optimizer_step
            || self.compiler_inputs_sha256 != sha(&current.handoff.compiler_inputs)?
            || self.actor_sha256 != sha(&current.handoff.actor)?
            || self.value_sha256 != sha(&current.handoff.value)?
        {
            return Err(
                "same-individual identity, exact heads, counters or session nonce mismatch".into(),
            );
        }
        let life = current
            .life
            .as_ref()
            .ok_or("same-individual private life missing")?;
        if self.private_life_sha256 != sha(life)?
            || sha(&self.sampling_state)? != sha(&private_binding(life, &current.handoff, p)?)?
        {
            return Err("same-individual private closure or exact sampler mismatch".into());
        }
        Ok(())
    }
}

impl CurrentTrainingCheckpointV1 {
    /// A retained private stream overrides a new request's sampling defaults.
    /// Missing legacy packets may be initialized by the producer, never reset.
    pub fn saved_training_sampling(&self) -> Result<Option<GpuTrainingSamplingConfig>> {
        let life = self.life.as_ref().ok_or("baseline private life missing")?;
        let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
        private_sampling(life, &save)
    }

    /// Admit the decoded full state while retaining the original file identity.
    /// JSON whitespace/order is not silently substituted for source provenance.
    pub fn admitted_source_sha256(&self, bytes: &[u8]) -> Result<String> {
        if Self::decode(bytes)?.encode()? != self.encode()? {
            return Err("same-individual baseline bytes differ from admitted source state".into());
        }
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }

    pub fn same_individual_successor(
        &self,
        baseline_bytes: &[u8],
        handoff: FoundationTrainingHandoffV1,
        life: CurrentLifeCheckpointV1,
        progress: SameIndividualProgressV1,
        seeds: CheckpointSeedsV1,
    ) -> Result<Self> {
        self.validate()?;
        let baseline_current_sha256 = self.admitted_source_sha256(baseline_bytes)?;
        handoff.validate()?;
        let previous_actor = &self.handoff.actor;
        let next_actor = &handoff.actor;
        if previous_actor.stage_mask != next_actor.stage_mask
            || sha(&previous_actor.config)? != sha(&next_actor.config)?
            || previous_actor.weights.len() != next_actor.weights.len()
            || previous_actor.update_ages.len() != next_actor.update_ages.len()
            || previous_actor.update_ages.len() != previous_actor.weights.len()
        {
            return Err(
                "same-individual optimizer configuration, mask or age coordinates changed".into(),
            );
        }
        for i in 0..previous_actor.weights.len() {
            if next_actor.update_ages[i] < previous_actor.update_ages[i]
                || (!previous_actor.stage_mask.is_trainable(i)
                    && (next_actor.weights[i].to_bits() != previous_actor.weights[i].to_bits()
                        || next_actor.first_moment[i].to_bits()
                            != previous_actor.first_moment[i].to_bits()
                        || next_actor.second_moment[i].to_bits()
                            != previous_actor.second_moment[i].to_bits()
                        || next_actor.update_ages[i] != previous_actor.update_ages[i]))
            {
                return Err(
                    "same-individual per-coordinate Adam age rolled back or frozen state changed"
                        .into(),
                );
            }
        }
        let old_life = self
            .life
            .as_ref()
            .ok_or("same-individual baseline private life missing")?;
        let old = alife_world::PortableSaveFile::from_json_str(&old_life.world_json)?;
        let saved = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
        if old.creatures.len() != 1
            || saved.creatures.len() != 1
            || old.creatures[0].organism_id != saved.creatures[0].organism_id
            || old.creatures[0].genome_id != saved.creatures[0].genome_id
            || old.creatures[0].composite_genetics != saved.creatures[0].composite_genetics
            || progress.session_start_world_tick != old.world.tick.raw()
            || saved.deterministic_seed != self.seed
        {
            return Err(
                "same-individual successor replaced baseline organism, birth or world".into(),
            );
        }
        let (rows, updates) = self.same_individual.as_ref().map_or((0, 0), |tag| {
            (tag.total_trained_rows, tag.total_completed_updates)
        });
        let tag = SameIndividualCurrentV1 {
            schema: 1,
            kind: "same-individual-ppo-v1".into(),
            counting_scope: "retained-same-individual-ppo-since-first-tag".into(),
            baseline_current_sha256,
            baseline_policy_version: self.handoff.policy_version,
            baseline_actor_optimizer_step: self.handoff.actor.optimizer_step,
            baseline_value_optimizer_step: self.handoff.value.optimizer_step,
            baseline_trained_rows: rows,
            baseline_completed_updates: updates,
            total_trained_rows: rows
                .checked_add(progress.trained_rows)
                .ok_or("trained rows overflow")?,
            total_completed_updates: updates
                .checked_add(progress.completed_updates)
                .ok_or("update count overflow")?,
            compiler_inputs_sha256: sha(&handoff.compiler_inputs)?,
            actor_sha256: sha(&handoff.actor)?,
            value_sha256: sha(&handoff.value)?,
            private_life_sha256: sha(&life)?,
            sampling_state: private_binding(&life, &handoff, &progress)?,
            progress,
        };
        let next = Self {
            schema_version: 1,
            seed: self.seed,
            founder_seed_base: self.founder_seed_base,
            seeds,
            handoff,
            life: Some(life),
            same_individual: Some(tag),
        };
        next.validate()?;
        Ok(next)
    }
}
