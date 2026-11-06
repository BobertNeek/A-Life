//! Explicit offline lineage continuation. Learned heads and, when explicitly
//! selected, exact neural structure cross the boundary. The old individual's
//! death and private experience remain source evidence.
use super::same_individual::{private_binding, sha};
use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewLifeNeuralInheritanceV1 {
    pub schema: u16,
    pub kind: String,
    pub canonical_actor_coordinates: usize,
    pub live_synapse_count: u32,
    pub live_recurrent_count: u32,
    pub live_decoder_count: u32,
    pub source_topology_sha256: String,
    pub source_lifetime_sha256: String,
    pub source_fast_sha256: String,
    pub source_predictor_sha256: String,
}
impl NewLifeNeuralInheritanceV1 {
    fn validate_transfer(
        &self,
        source: &CurrentLifeCheckpointV1,
        target: &CurrentLifeCheckpointV1,
    ) -> Result<()> {
        use super::same_individual::private_asset;
        let old = alife_world::PortableSaveFile::from_json_str(&source.world_json)?;
        let new = alife_world::PortableSaveFile::from_json_str(&target.world_json)?;
        if old.creatures.len() != 1 || new.creatures.len() != 1 {
            return Err("neural transfer requires exact source and target individuals".into());
        }
        let old_brain = old.creatures[0]
            .gpu_brain
            .as_ref()
            .ok_or("source neural brain missing")?;
        let new_brain = new.creatures[0]
            .gpu_brain
            .as_ref()
            .ok_or("target neural brain missing")?;
        let old_top = old_brain
            .live_structural_topology
            .as_ref()
            .ok_or("source neural topology missing")?;
        let new_top = new_brain
            .live_structural_topology
            .as_ref()
            .ok_or("target neural topology missing")?;
        let topology: alife_gpu_backend::GpuLiveTopologyCheckpointV1 =
            serde_json::from_slice(&private_asset(source, &old, &old_top.asset_id)?)?;
        if self.live_synapse_count != topology.total_synapse_count
            || self.live_recurrent_count != topology.recurrent_synapse_count
            || self.live_decoder_count != topology.decoder_synapse_count
        {
            return Err("neural inheritance evidence differs from source coordinates".into());
        }
        for (a, b, expected) in [
            (old_top, new_top, &self.source_topology_sha256),
            (
                &old_brain.lifetime_weights,
                &new_brain.lifetime_weights,
                &self.source_lifetime_sha256,
            ),
            (
                &old_brain.fast_weights,
                &new_brain.fast_weights,
                &self.source_fast_sha256,
            ),
        ] {
            let bytes = private_asset(source, &old, &a.asset_id)?;
            if &format!("{:x}", Sha256::digest(&bytes)) != expected
                || bytes != private_asset(target, &new, &b.asset_id)?
            {
                return Err(
                    "new-life inherited neural assets differ from exact terminal source".into(),
                );
            }
        }
        let source_exact = alife_world::ExactCognitiveCheckpointState::decode(&private_asset(
            source,
            &old,
            &old_brain
                .exact_cognitive_state
                .as_ref()
                .ok_or("source exact cognition missing")?
                .asset_id,
        )?)?;
        let target_exact = alife_world::ExactCognitiveCheckpointState::decode(&private_asset(
            target,
            &new,
            &new_brain
                .exact_cognitive_state
                .as_ref()
                .ok_or("target exact cognition missing")?
                .asset_id,
        )?)?;
        if sha(&source_exact.predictor)? != self.source_predictor_sha256
            || sha(&source_exact.predictor)? != sha(&target_exact.predictor)?
            || sha(&source_exact.dendritic_branches)? != sha(&target_exact.dendritic_branches)?
            || sha(&source_exact.structural_plasticity)?
                != sha(&target_exact.structural_plasticity)?
        {
            return Err("new-life inherited learned cognition differs from terminal source".into());
        }
        Ok(())
    }

    fn validate(&self, actor_coordinates: usize) -> Result<()> {
        if self.schema != 1
            || self.kind != "exact-live-order-global-adam-neural-inheritance-v1"
            || self.canonical_actor_coordinates != actor_coordinates
            || (self.live_synapse_count as usize) < actor_coordinates
            || self
                .live_recurrent_count
                .checked_add(self.live_decoder_count)
                != Some(self.live_synapse_count)
            || [
                &self.source_topology_sha256,
                &self.source_lifetime_sha256,
                &self.source_fast_sha256,
                &self.source_predictor_sha256,
            ]
            .iter()
            .any(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()))
        {
            return Err("invalid retained-neural inheritance identity or coordinates".into());
        }
        Ok(())
    }
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub neural_inheritance: Option<NewLifeNeuralInheritanceV1>,
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
            if let Some(inherited) = &life.neural_inheritance {
                inherited.validate(current.handoff.actor.weights.len())?;
            }
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
        self.seal_new_life_inner(source_bytes, world_seed, organism_id, life, None)
    }

    pub fn seal_new_life_with_retained_neural(
        &self,
        source_bytes: &[u8],
        world_seed: u64,
        organism_id: u64,
        life: CurrentLifeCheckpointV1,
        inheritance: NewLifeNeuralInheritanceV1,
    ) -> Result<Self> {
        inheritance.validate(self.handoff.actor.weights.len())?;
        inheritance.validate_transfer(
            self.life.as_ref().ok_or("source neural life missing")?,
            &life,
        )?;
        self.seal_new_life_inner(
            source_bytes,
            world_seed,
            organism_id,
            life,
            Some(inheritance),
        )
    }

    fn seal_new_life_inner(
        &self,
        source_bytes: &[u8],
        world_seed: u64,
        organism_id: u64,
        life: CurrentLifeCheckpointV1,
        neural_inheritance: Option<NewLifeNeuralInheritanceV1>,
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
            neural_inheritance,
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
