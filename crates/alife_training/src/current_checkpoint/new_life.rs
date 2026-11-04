//! Explicit offline lineage continuation. Only the learned global heads cross
//! this boundary; the old individual's death and private life remain evidence.
use super::same_individual::{private_binding, sha};
use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewLifeEvidenceV1 {
    pub source_current_sha256: String,
    pub source_terminal: CurrentTerminalDispositionV1,
    pub source_training_attempt_origin: Option<TrainingAttemptOriginV1>,
    pub source_seeds: CheckpointSeedsV1,
    pub source_handoff_sha256: String,
    pub compiler_inputs_sha256: String,
    pub actor_sha256: String,
    pub value_sha256: String,
    pub policy_version: u64,
    pub actor_optimizer_step: u32,
    pub value_optimizer_step: u32,
    pub world_seed: u64,
    pub initialization_nonce: u64,
    pub organism_id: u64,
    pub initial_private_life_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewLifeOriginV1 {
    pub schema: u16,
    pub kind: String,
    /// Oldest first. Constant-depth provenance; original source files remain
    /// separately archived, never substituted with these hash records.
    pub lives: Vec<NewLifeEvidenceV1>,
    pub record_sha256: String,
}

impl NewLifeOriginV1 {
    fn digest(&self) -> Result<String> {
        let mut unsigned = self.clone();
        unsigned.record_sha256.clear();
        sha(&unsigned)
    }
    pub(super) fn validate(&self, current: &CurrentTrainingCheckpointV1) -> Result<()> {
        if self.schema != 1
            || self.kind != "explicit-new-individual-learned-heads-v1"
            || self.lives.is_empty()
            || self.lives.len() > 1024
            || self.record_sha256 != self.digest()?
        {
            return Err("new-life provenance is incomplete or changed".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut seeds = std::collections::BTreeSet::new();
        for life in &self.lives {
            life.source_seeds.validate()?;
            if life.organism_id == 0
                || life.organism_id <= life.source_terminal.organism_id
                || !ids.insert(life.organism_id)
                || !seeds.insert(life.world_seed)
                || !seeds.insert(life.initialization_nonce)
                || life.world_seed.checked_add(1) != Some(life.initialization_nonce)
                || !current.seeds.contains(life.world_seed)
                || !current.seeds.contains(life.initialization_nonce)
                || current
                    .seeds
                    .pending_range
                    .is_some_and(|[a, b]| a <= life.initialization_nonce && life.world_seed <= b)
                || life.source_seeds.historical_freshness != current.seeds.historical_freshness
                || life.source_seeds.excluded_ranges.iter().any(|[a, b]| {
                    !current
                        .seeds
                        .excluded_ranges
                        .iter()
                        .any(|[x, y]| x <= a && b <= y)
                })
            {
                return Err("new-life identities or conservative seed history changed".into());
            }
        }
        let initial = self.lives.last().expect("nonempty");
        let save = alife_world::PortableSaveFile::from_json_str(
            &current
                .life
                .as_ref()
                .ok_or("new-life private state missing")?
                .world_json,
        )?;
        let tag = current
            .same_individual
            .as_ref()
            .ok_or("new-life continuous identity missing")?;
        if current.seed != initial.world_seed
            || save.creatures.len() != 1
            || save.creatures[0].organism_id.raw() != initial.organism_id
            || current.handoff.policy_version < initial.policy_version
            || current.handoff.actor.optimizer_step < initial.actor_optimizer_step
            || current.handoff.value.optimizer_step < initial.value_optimizer_step
            || tag
                .total_completed_updates
                .checked_add(initial.policy_version)
                != Some(current.handoff.policy_version)
        {
            return Err("new-life body identity or retained optimizer history changed".into());
        }
        if tag.total_completed_updates == 0
            && (sha(&current.handoff)? != initial.source_handoff_sha256
                || sha(&current.handoff.actor)? != initial.actor_sha256
                || sha(&current.handoff.value)? != initial.value_sha256
                || sha(&current.handoff.compiler_inputs)? != initial.compiler_inputs_sha256)
        {
            return Err("new-life imported heads differ from exact terminal source".into());
        }
        if save.world.tick.raw() == 0
            && sha(current.life.as_ref().expect("checked"))? != initial.initial_private_life_sha256
        {
            return Err("new-life initial private state changed".into());
        }
        Ok(())
    }
}

impl CurrentTrainingCheckpointV1 {
    /// Device-free preflight. A fresh world and initialization boundary consume
    /// two entries from the next reservation, preserving its entire unused tail.
    pub fn new_life_seeds(&self, world_seed: u64, organism_id: u64) -> Result<CheckpointSeedsV1> {
        self.validate()?;
        let terminal = self
            .terminal_disposition
            .as_ref()
            .ok_or("new life requires an observed terminal checkpoint")?;
        if self.same_individual.is_none() || self.handoff.compiler_inputs.live_policy().is_none() {
            return Err("new life requires exact same-individual compiler and heads".into());
        }
        let previous_ids = self
            .new_life_origin
            .as_ref()
            .into_iter()
            .flat_map(|o| &o.lives);
        if organism_id <= terminal.organism_id
            || previous_ids
                .into_iter()
                .any(|p| organism_id <= p.organism_id)
        {
            return Err("new-life organism identity collides with retained lineage".into());
        }
        let nonce = world_seed.checked_add(1).ok_or("new-life seed overflow")?;
        let mut seeds = self.seeds.clone();
        match seeds.pending_range {
            None => seeds.reserve(world_seed, nonce)?,
            Some([a, b]) if a == world_seed && nonce <= b => (),
            _ => return Err("new life must consume the next two unused owned seeds".into()),
        }
        seeds.consume(world_seed)?;
        seeds.consume(nonce)?;
        Ok(seeds)
    }

    /// Seal a newly constructed, never-dispatched private body. The native host
    /// additionally admits all private codecs and verifies canonical birth.
    pub fn seal_new_life(
        &self,
        source_bytes: &[u8],
        world_seed: u64,
        organism_id: u64,
        life: CurrentLifeCheckpointV1,
    ) -> Result<Self> {
        let seeds = self.new_life_seeds(world_seed, organism_id)?;
        let source_current_sha256 = self.admitted_source_sha256(source_bytes)?;
        let save = alife_world::PortableSaveFile::from_json_str(&life.world_json)?;
        let world = save.restore_headless_world()?;
        let records = world.organism_registry().iter().collect::<Vec<_>>();
        if world.seed() != world_seed
            || world.tick().raw() != 0
            || records.len() != 1
            || records[0].organism_id().raw() != organism_id
            || !records[0].lifecycle().is_alive()
            || records[0].birth_tick().raw() != 0
            || records[0].biochemistry().tick.raw() != 0
            || records[0].sleep_cycle_id() != 0
            || records[0].cognitive_work() != &alife_core::CognitiveWorkReceipt::zero()
        {
            return Err("new life must be an actual newborn at an undispatched boundary".into());
        }
        let progress = SameIndividualProgressV1 {
            session_nonce: world_seed + 1,
            organism_id,
            session_start_world_tick: 0,
            world_tick: 0,
            trained_rows: 0,
            completed_updates: 0,
            sampling_temperature: self
                .saved_training_sampling()?
                .ok_or("terminal sampler missing")?
                .temperature,
        };
        let adopted = if save.creatures[0].composite_genetics.is_none() {
            Some(
                self.handoff
                    .compiler_inputs
                    .live_policy()
                    .ok_or("live compiler missing")?
                    .birth_foundation_digest,
            )
        } else {
            None
        };
        let sampling = private_binding(&life, &self.handoff, &progress, adopted)?;
        if sampling.counter != 0
            || sampling.demonstrator.is_some()
            || sampling.seed
                == self
                    .saved_training_sampling()?
                    .ok_or("terminal sampler missing")?
                    .seed
        {
            return Err("new-life sampling stream is not fresh".into());
        }
        let mut origin = self.new_life_origin.clone().unwrap_or(NewLifeOriginV1 {
            schema: 1,
            kind: "explicit-new-individual-learned-heads-v1".into(),
            lives: Vec::new(),
            record_sha256: String::new(),
        });
        origin.lives.push(NewLifeEvidenceV1 {
            source_current_sha256: source_current_sha256.clone(),
            source_terminal: self
                .terminal_disposition
                .clone()
                .ok_or("terminal missing")?,
            source_training_attempt_origin: self.training_attempt_origin.clone(),
            source_seeds: self.seeds.clone(),
            source_handoff_sha256: sha(&self.handoff)?,
            compiler_inputs_sha256: sha(&self.handoff.compiler_inputs)?,
            actor_sha256: sha(&self.handoff.actor)?,
            value_sha256: sha(&self.handoff.value)?,
            policy_version: self.handoff.policy_version,
            actor_optimizer_step: self.handoff.actor.optimizer_step,
            value_optimizer_step: self.handoff.value.optimizer_step,
            world_seed,
            initialization_nonce: world_seed + 1,
            organism_id,
            initial_private_life_sha256: sha(&life)?,
        });
        origin.record_sha256 = origin.digest()?;
        let tag = SameIndividualCurrentV1 {
            schema: 1,
            kind: "same-individual-ppo-v1".into(),
            counting_scope: "retained-same-individual-ppo-since-first-tag".into(),
            progress,
            baseline_current_sha256: source_current_sha256,
            baseline_policy_version: self.handoff.policy_version,
            baseline_actor_optimizer_step: self.handoff.actor.optimizer_step,
            baseline_value_optimizer_step: self.handoff.value.optimizer_step,
            baseline_trained_rows: 0,
            baseline_completed_updates: 0,
            total_trained_rows: 0,
            total_completed_updates: 0,
            compiler_inputs_sha256: sha(&self.handoff.compiler_inputs)?,
            actor_sha256: sha(&self.handoff.actor)?,
            value_sha256: sha(&self.handoff.value)?,
            private_life_sha256: sha(&life)?,
            adopted_foundation_digest: adopted,
            sampling_state: sampling,
        };
        let next = Self {
            schema_version: 1,
            seed: world_seed,
            founder_seed_base: self.founder_seed_base,
            seeds,
            handoff: self.handoff.clone(),
            life: Some(life),
            same_individual: Some(tag),
            terminal_disposition: None,
            training_attempt_origin: None,
            new_life_origin: Some(origin),
        };
        next.validate()?;
        Ok(next)
    }
}
