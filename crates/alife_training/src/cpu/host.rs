//! Canonical CPU collector. Biology, legality, perception, sleep and episodic
//! evidence are organism-owned; only founder parameters/optimizer are shared.
use super::*;
use alife_core::cognitive_work::{CognitiveWorkCostPolicy, CognitiveWorkCounters};
use alife_core::predictive::GroundedSuccessorPredictor;
use alife_core::sleep::{
    ReplayEligibilitySample, ReplaySynapseSpan, SleepController, SleepReplayEvent,
    SleepReplayEvidence,
};
use alife_core::*;
use alife_gpu_backend::{GpuV11CausalState, GpuV11Checkpoint};
use alife_runtime::canonical_preparation::*;
use alife_world::{
    AssetManifest, BrainPolicyConfig, CanonicalNewGame, CreatureSaveState, HeadlessWorld,
    PortableSaveFile, RuntimeConfig,
};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum CpuHostError {
    #[error(transparent)]
    Organism(#[from] alife_world::OrganismRegistryError),
    #[error(transparent)]
    Training(#[from] crate::TrainingError),
    #[error(transparent)]
    Contract(#[from] ScaffoldContractError),
    #[error(transparent)]
    Persistence(#[from] alife_world::PersistenceError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("CPU canonical host: {0}")]
    State(&'static str),
    #[error("CPU episode preparation: {0}")]
    Preparation(String),
}
#[derive(Clone)]
struct Resident {
    neural: CpuOrganismState,
    activity: CpuActivityState,
    v11: GpuV11CausalState,
    memory: MemorySidecarState,
    topology: TopologySidecar,
    predictor: GroundedSuccessorPredictor,
    sleep: SleepController,
    hysteresis: HysteresisState,
    next_sequence: u64,
    replay: Vec<CpuReplayEvent>,
    language_grounding: LanguageGroundingLedger,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CpuReplayEvent {
    event: SleepReplayEvent,
    eligibility: Vec<i16>,
    target: PredictionTargetReceipt,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuResidentCheckpoint {
    pub neural: CpuOrganismCheckpoint,
    pub activity: CpuActivityState,
    pub neural_topology: GpuV11Checkpoint,
    pub memory: PortableMemoryBankAssetV2,
    pub memory_compaction: MemoryCompactionCheckpoint,
    pub staged_memory: Option<PortableMemoryBankAssetV2>,
    pub topology: PortableTopologySidecarAssetV1,
    pub predictor: GroundedSuccessorPredictor,
    pub sleep: SleepController,
    pub hysteresis: HysteresisState,
    pub next_sequence: u64,
    pub language_grounding: LanguageGroundingLedger,
    replay: Vec<CpuReplayEvent>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuCanonicalCheckpoint {
    pub schema_version: u16,
    pub phenotype_hash: PhenotypeHash,
    pub founder_weight_digest: [u64; 4],
    pub sampling_seed: u32,
    pub sampling_counter: u32,
    pub world: PortableSaveFile,
    /// CPU saves have no GPU brain sidecar to carry world perception identity.
    /// Version1 omitted this registry and cannot establish exact object continuity.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracked_objects: Vec<alife_world::TrackedObjectRegistrySaveState>,
    pub residents: Vec<CpuResidentCheckpoint>,
}
/// Complete in-memory pre-success branch state. Restoring it restores the world,
/// chemistry, memories, predictor, neural traces, sleep, sequence and RNG counter.
#[derive(Clone)]
pub struct CpuCanonicalHost {
    world: HeadlessWorld,
    creatures: Vec<CreatureSaveState>,
    phenotype_hash: PhenotypeHash,
    founder_weight_digest: [u64; 4],
    residents: BTreeMap<u64, Resident>,
    sampling_seed: u32,
    sampling_counter: u32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuCollectedDecision {
    pub organism_id: OrganismId,
    /// Exact native sampling counter. Older diagnostic rows lack this evidence;
    /// they remain readable but cannot be admitted as cooperative shards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sampling_counter: Option<u32>,
    pub initial: crate::TrainingInitialState,
    pub replay_tick: crate::TrainingReplayTick,
    pub action: crate::PpoJointAction,
    pub final_features: Vec<f32>,
    pub brain_work: BrainWorkCounters,
    pub biological_reward: f32,
    pub patch: ExperiencePatch,
}
pub struct CpuCanonicalStep {
    pub tick_before: Tick,
    pub tick_after: Tick,
    pub decisions: Vec<CpuCollectedDecision>,
    pub sleeping_organisms: usize,
    pub sleep_consolidations: usize,
}
fn weight_digest(founder: &CpuFoundationTrainer) -> Result<[u64; 4], ScaffoldContractError> {
    let mut d = CanonicalDigestBuilder::new(b"ALIFE-CPU-FOUNDER-WEIGHTS-V1");
    for &w in founder.weights() {
        d.write_f32(w)?;
    }
    Ok(d.finish256())
}
fn profile_identity(p: &BrainPhenotype) -> SensorProfileIdentity {
    SensorProfileIdentity {
        profile_id: p.sensor_profile().into(),
        profile_schema_version: 1,
        sensory_abi_version: SensoryAbiVersion::CURRENT.raw(),
    }
}
fn json_digest<T: serde::Serialize>(domain: &[u8], v: &T) -> Result<[u64; 4], CpuHostError> {
    let mut d = CanonicalDigestBuilder::new(domain);
    d.write_bytes(&serde_json::to_vec(v)?);
    Ok(d.finish256())
}
impl CpuCanonicalHost {
    /// Fresh cohort only. Resuming an existing individual requires from_checkpoint;
    /// existing somatic state is never inferred from a founder asset.
    pub fn new_cohort(
        game: CanonicalNewGame,
        founder: &CpuFoundationTrainer,
        sampling_seed: u32,
    ) -> Result<Self, CpuHostError> {
        let world = game.world;
        let creatures = game.creatures;
        let p = founder.phenotype();
        let profile = profile_identity(p);
        let mut residents = BTreeMap::new();
        for record in world
            .organism_registry()
            .iter()
            .filter(|r| r.lifecycle().is_alive())
        {
            if record.biochemistry().tick != world.tick()
                || record.cognitive_work() != &CognitiveWorkReceipt::zero()
                || record.sleep_cycle_id() != 0
                || record.birth_tick() != world.tick()
            {
                return Err(CpuHostError::State("fresh cohort requires newborn canonical organism records; use exact restore for existing individuals"));
            }
            validate_record_founder(record, founder)?;
            let id = record.organism_id();
            let n = p.neuron_count() as usize;
            let v11 = GpuV11CausalState::for_phenotype(p)?;
            let neural = CpuOrganismState::new(
                p,
                id,
                crate::TrainingInitialState {
                    activations: vec![0.; n],
                    activity_ema: vec![0.; n],
                    metabolic_load: vec![0.; n],
                    dendrites: v11.dendritic_branches().clone(),
                },
            )?;
            let policy = attention_selection_policy_for(p);
            residents.insert(
                id.raw(),
                Resident {
                    neural,
                    activity: CpuActivityState::default(),
                    v11,
                    memory: MemorySidecarState::new_profiled(
                        id,
                        profile,
                        MemoryBankConfig::new(256, 64, 4, 0.72, Confidence::new(0.)?)?,
                    )?,
                    topology: TopologySidecar::new_profiled(
                        id,
                        profile,
                        TopologicalMapConfig::default(),
                    )?,
                    predictor: predictor_for_phenotype(p)?,
                    sleep: SleepController::new(sleep_consolidation_config_for(p)?)?,
                    hysteresis: HysteresisState {
                        previous_identity: None,
                        retained_ticks: 0,
                        switch_margin: policy.hysteresis_margin,
                    },
                    next_sequence: 1,
                    replay: Vec::new(),
                    language_grounding: LanguageGroundingLedger::default(),
                },
            );
        }
        if residents.is_empty() {
            return Err(CpuHostError::State(
                "cohort has no living canonical organisms",
            ));
        }
        let mut host = Self {
            world,
            creatures,
            phenotype_hash: p.phenotype_hash(),
            founder_weight_digest: weight_digest(founder)?,
            residents,
            sampling_seed,
            sampling_counter: 0,
        };
        host.seal_all_residents()?;
        Ok(host)
    }
    /// A new episode, prepared by ordinary world operations before the first
    /// neural dispatch. This first enforces newborn admission; it cannot admit
    /// an aged retained individual or substitute a founder for its private state.
    pub fn new_prepared_cohort<T, E: std::fmt::Display>(
        game: CanonicalNewGame,
        founder: &CpuFoundationTrainer,
        sampling_seed: u32,
        prepare: impl FnOnce(&mut HeadlessWorld) -> Result<T, E>,
    ) -> Result<(Self, T), CpuHostError> {
        let mut host = Self::new_cohort(game, founder, sampling_seed)?;
        let initial = host
            .world
            .organism_registry()
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let entities = host.world.organism_entity_ids();
        let start = host.world.tick();
        let setup = prepare(&mut host.world)
            .map_err(|error| CpuHostError::Preparation(error.to_string()))?;
        if host.world.tick() < start
            || host.world.organism_entity_ids() != entities
            || host.world.organism_registry().iter().count() != initial.len()
        {
            return Err(CpuHostError::State(
                "episode setup changed the admitted roster or world clock",
            ));
        }
        for original in initial {
            let record = host
                .world
                .organism_registry()
                .get(original.organism_id())
                .ok_or(CpuHostError::State(
                    "episode setup lost its admitted learner",
                ))?;
            if record.genome() != original.genome()
                || record.phenotype() != original.phenotype()
                || record.birth_tick() != original.birth_tick()
                || !record.lifecycle().is_alive()
                || record.biochemistry().tick != host.world.tick()
                || record.sleep_cycle_id() != 0
                || record.biochemistry().body.sleeping
            {
                return Err(CpuHostError::State(
                    "episode setup changed learner identity or cannot begin an awake episode",
                ));
            }
            validate_record_founder(record, founder)?;
        }
        host.seal_all_residents()?;
        Ok((host, setup))
    }
    /// Read-only scene drafts for nonblocking semantic-prior priming. Priming
    /// advances no world tick, neural state, exposure or action sequence.
    pub fn semantic_prior_inputs(
        &mut self,
        founder: &CpuFoundationTrainer,
    ) -> Result<Vec<(PerceptionFrameDraft, ExperienceSequenceId)>, CpuHostError> {
        self.validate_founder(founder)?;
        let tick = self.world.tick();
        let mut perception_world = self.world.clone();
        let mut result = Vec::new();
        for (&raw, resident) in &self.residents {
            let id = OrganismId(raw);
            let record = self
                .world
                .organism_registry()
                .get(id)
                .ok_or(CpuHostError::State("semantic-prior learner missing"))?;
            if record.lifecycle().is_alive() && resident.sleep.state().phase == SleepPhase::Awake {
                let homeostasis = record.biochemistry().homeostasis;
                result.push((
                    perception_world.perception_frame_draft(
                        id,
                        tick,
                        founder.phenotype().sensor_profile(),
                        homeostasis,
                    )?,
                    ExperienceSequenceId(resident.next_sequence),
                ));
            }
        }
        Ok(result)
    }
    pub fn world(&self) -> &HeadlessWorld {
        &self.world
    }
    /// Scene variations use ordinary world APIs without reward or neural overrides.
    pub fn world_mut(&mut self) -> &mut HeadlessWorld {
        &mut self.world
    }
    pub fn snapshot(&self) -> Self {
        self.clone()
    }
    pub fn restore_snapshot(
        &mut self,
        snapshot: Self,
        founder: &CpuFoundationTrainer,
    ) -> Result<(), CpuHostError> {
        snapshot.validate_founder(founder)?;
        *self = snapshot;
        Ok(())
    }
    fn validate_founder(&self, founder: &CpuFoundationTrainer) -> Result<(), CpuHostError> {
        if self.phenotype_hash != founder.phenotype().phenotype_hash()
            || self.founder_weight_digest != weight_digest(founder)?
        {
            return Err(CpuHostError::State("living collector policy changed: export/recompile a fresh cohort or explicitly rebase a simulator branch"));
        }
        Ok(())
    }
    /// Explicit canonical recovery request, useful for recovery protocols and
    /// branch experiments. It changes sleep authority, never body chemistry.
    pub fn request_recovery_sleep(&mut self, id: OrganismId) -> Result<(), CpuHostError> {
        let mut next = self.clone();
        let tick = next.world.tick();
        let r = next
            .residents
            .get_mut(&id.raw())
            .ok_or(CpuHostError::State("unknown recovery organism"))?;
        r.sleep
            .force_sleep(tick, alife_core::SleepTrigger::RecoveryProtocol)?;
        let state = r.sleep.state();
        let mut record = next
            .world
            .organism_registry()
            .get(id)
            .cloned()
            .ok_or(CpuHostError::State("recovery organism missing"))?;
        record.seal_sleep_phase(state.phase, state.active_cycle_id, tick, 0)?;
        next.world.replace_organism_record_exact(record)?;
        next.seal_all_residents()?;
        *self = next;
        Ok(())
    }
    pub fn checkpoint(
        &self,
        founder: &CpuFoundationTrainer,
    ) -> Result<CpuCanonicalCheckpoint, CpuHostError> {
        self.validate_founder(founder)?;
        let mut config =
            RuntimeConfig::deterministic_default(self.world.seed(), BrainScaleTier::Standard2048);
        config.brain_policy = BrainPolicyConfig::offline_cpu_training();
        config.validate()?;
        let mut creatures = self.creatures.clone();
        for creature in &mut creatures {
            let record = self
                .world
                .organism_registry()
                .get(creature.organism_id)
                .ok_or(CpuHostError::State(
                    "creature summary missing canonical record",
                ))?;
            creature.mind.tick = record.biochemistry().tick;
            creature.mind.homeostasis = record.biochemistry().homeostasis;
            creature.development_tick = record.biochemistry().development.last_update_tick;
            creature.gpu_brain = None;
            creature.lifetime_state_asset = None;
            creature.composite_genetics = None;
        }
        let world = PortableSaveFile::from_headless_world(
            "cpu-canonical-collector",
            &self.world,
            config,
            AssetManifest::empty(),
            creatures,
        )?;
        let residents = self
            .residents
            .values()
            .map(|r| {
                Ok(CpuResidentCheckpoint {
                    neural: r.neural.checkpoint()?,
                    activity: r.activity.clone(),
                    neural_topology: r.v11.checkpoint(),
                    memory: r.memory.export_active_bank()?,
                    memory_compaction: *r.memory.compaction_checkpoint(),
                    staged_memory: r.memory.export_staged_bank()?,
                    topology: r.topology.export_portable()?,
                    predictor: r.predictor.clone(),
                    sleep: r.sleep,
                    hysteresis: r.hysteresis,
                    next_sequence: r.next_sequence,
                    replay: r.replay.clone(),
                    language_grounding: r.language_grounding.clone(),
                })
            })
            .collect::<Result<_, CpuHostError>>()?;
        Ok(CpuCanonicalCheckpoint {
            schema_version: 2,
            phenotype_hash: self.phenotype_hash,
            founder_weight_digest: self.founder_weight_digest,
            sampling_seed: self.sampling_seed,
            sampling_counter: self.sampling_counter,
            world,
            tracked_objects: self
                .residents
                .keys()
                .map(|&id| self.world.tracked_objects().save_state(OrganismId(id)))
                .collect::<Result<_, _>>()?,
            residents,
        })
    }
    pub fn from_checkpoint(
        checkpoint: CpuCanonicalCheckpoint,
        founder: &CpuFoundationTrainer,
    ) -> Result<Self, CpuHostError> {
        if checkpoint.schema_version == 1 {
            return Err(CpuHostError::State("version1 CPU checkpoint lacks tracked-object identity; exact private-life restore requires version2"));
        }
        if checkpoint.schema_version != 2
            || checkpoint.world.config.brain_policy.policy != PolicyBackend::NeuralClosedLoopCpu
        {
            return Err(CpuHostError::State(
                "not a versioned offline CPU checkpoint",
            ));
        }
        checkpoint.world.config.validate()?;
        let creatures = checkpoint.world.creatures.clone();
        let mut world = checkpoint.world.restore_headless_world()?;
        let owners = checkpoint
            .residents
            .iter()
            .map(|r| r.neural.organism_id.raw())
            .collect::<std::collections::BTreeSet<_>>();
        let tracked_owners = checkpoint
            .tracked_objects
            .iter()
            .map(|s| s.organism_id.raw())
            .collect::<std::collections::BTreeSet<_>>();
        if owners != tracked_owners
            || owners.len() != checkpoint.residents.len()
            || checkpoint
                .tracked_objects
                .iter()
                .any(|s| s.records.iter().any(|r| r.last_seen_tick > world.tick()))
        {
            return Err(CpuHostError::State(
                "checkpoint tracked-object owner or clock mismatch",
            ));
        }
        world.restore_tracked_object_states(checkpoint.tracked_objects)?;
        let mut residents = BTreeMap::new();
        let p = founder.phenotype();
        let profile = profile_identity(p);
        for r in checkpoint.residents {
            let id = r.neural.organism_id;
            let record = world
                .organism_registry()
                .get(id)
                .ok_or(CpuHostError::State(
                    "checkpoint resident has no canonical organism",
                ))?;
            if r.next_sequence == 0
                || r.sleep.state().phase != record.sleep_phase()
                || r.sleep
                    .state()
                    .active_cycle_id
                    .max(r.sleep.state().last_consolidated_cycle_id)
                    != record.sleep_cycle_id()
                || r.neural
                    .last_outcome_tick
                    .is_some_and(|t| t > world.tick().raw())
                || r.replay.len() > p.replay_capture_plan().event_capacity() as usize
            {
                return Err(CpuHostError::State("checkpoint causal boundary mismatch"));
            }
            validate_record_founder(record, founder)?;
            r.language_grounding.validate_contract()?;
            if r.memory.last_observed_sequence_id_raw != r.next_sequence - 1
                || r.language_grounding.last_sequence_id() != r.next_sequence - 1
            {
                return Err(CpuHostError::State(
                    "checkpoint memory/language sequence mismatch",
                ));
            }
            replay_evidence(p, &r.replay)?.validate_contract()?;
            if r.memory.organism_id_raw != id.raw()
                || r.memory_compaction.organism_id_raw != id.raw()
                || r.topology.organism_id_raw != id.raw()
                || r.topology.profile != profile
            {
                return Err(CpuHostError::State(
                    "checkpoint sidecar owner/profile mismatch",
                ));
            }
            let sleep =
                SleepController::restore(sleep_consolidation_config_for(p)?, r.sleep.state())?;
            if r.neural_topology.neuron_count != p.neuron_count()
                || r.neural_topology.dendritic_branches != r.neural.neural.dendrites
                || r.neural_topology.pending_lifetime_synapse.is_some()
                || r.neural_topology
                    .sparse_spans
                    .iter()
                    .flat_map(|s| &s.edges)
                    .any(|e| e.route as usize >= p.projections().len())
            {
                return Err(CpuHostError::State("neural topology checkpoint mismatch"));
            }
            let resident = Resident {
                v11: GpuV11CausalState::restore(r.neural_topology)?,
                neural: CpuOrganismState::from_checkpoint(p, r.neural)?,
                activity: r.activity,
                memory: MemorySidecarState::restore_portable(
                    profile,
                    r.memory_compaction,
                    r.memory,
                    r.staged_memory,
                )?,
                topology: TopologySidecar::restore_portable(r.topology)?,
                predictor: r.predictor,
                sleep,
                hysteresis: r.hysteresis,
                next_sequence: r.next_sequence,
                replay: r.replay,
                language_grounding: r.language_grounding,
            };
            validate_resident_authority(
                record,
                &resident,
                checkpoint.sampling_seed,
                checkpoint.sampling_counter,
            )?;
            if residents.insert(id.raw(), resident).is_some() {
                return Err(CpuHostError::State("duplicate checkpoint organism"));
            }
        }
        for r in world
            .organism_registry()
            .iter()
            .filter(|r| r.lifecycle().is_alive())
        {
            if !residents.contains_key(&r.organism_id().raw()) {
                return Err(CpuHostError::State("checkpoint missing living organism"));
            }
        }
        let host = Self {
            world,
            creatures,
            phenotype_hash: checkpoint.phenotype_hash,
            founder_weight_digest: checkpoint.founder_weight_digest,
            residents,
            sampling_seed: checkpoint.sampling_seed,
            sampling_counter: checkpoint.sampling_counter,
        };
        host.validate_founder(founder)?;
        Ok(host)
    }
    fn seal_all_residents(&mut self) -> Result<(), CpuHostError> {
        for (&raw, r) in &self.residents {
            let mut record = self
                .world
                .organism_registry()
                .get(OrganismId(raw))
                .cloned()
                .ok_or(CpuHostError::State(
                    "missing organism at CPU sealed boundary",
                ))?;
            record.seal_cognitive_subsystems(
                record.biochemistry().tick,
                resident_digest(r, self.sampling_seed, self.sampling_counter)?,
                r.memory.export_active_bank()?.active_bank_digest,
            )?;
            self.world.replace_organism_record_exact(record)?;
        }
        Ok(())
    }
    /// Atomic whole-world tick: a failed seal/learning/checkpoint invariant rolls
    /// back body, biology, all cognitive rows and the sampling counter together.
    pub fn step(
        &mut self,
        founder: &CpuFoundationTrainer,
        temperature: f32,
    ) -> Result<CpuCanonicalStep, CpuHostError> {
        self.step_with_preparation(founder, temperature, |draft, _| Ok(draft))
    }
    /// The shared semantic prior may enrich only its bounded private context.
    /// Ordinary senses, candidates, body and identity are checked unchanged.
    /// Host rollback does not roll back an external provider. Its owner archives
    /// that state and reconciles a failed episode instead of retrying its seed.
    pub fn step_with_preparation(
        &mut self,
        founder: &CpuFoundationTrainer,
        temperature: f32,
        mut prepare: impl FnMut(
            PerceptionFrameDraft,
            ExperienceSequenceId,
        ) -> Result<PerceptionFrameDraft, ScaffoldContractError>,
    ) -> Result<CpuCanonicalStep, CpuHostError> {
        self.validate_founder(founder)?;
        for (&raw, r) in &self.residents {
            let record = self
                .world
                .organism_registry()
                .get(OrganismId(raw))
                .ok_or(CpuHostError::State("missing canonical organism"))?;
            validate_resident_authority(record, r, self.sampling_seed, self.sampling_counter)?;
        }
        let mut next = self.clone();
        let result = next.step_inner(founder, temperature, &mut prepare)?;
        *self = next;
        Ok(result)
    }
    fn step_inner(
        &mut self,
        founder: &CpuFoundationTrainer,
        temperature: f32,
        prepare: &mut impl FnMut(
            PerceptionFrameDraft,
            ExperienceSequenceId,
        ) -> Result<PerceptionFrameDraft, ScaffoldContractError>,
    ) -> Result<CpuCanonicalStep, CpuHostError> {
        let p = founder.phenotype();
        let tick = self.world.tick();
        let mut decisions = Vec::new();
        let mut sleeping = 0;
        let mut consolidations = 0;
        let mut body_events = BTreeMap::new();
        let mut perception_world = self.world.clone();
        let ids = self
            .world
            .organism_registry()
            .iter()
            .filter(|r| r.lifecycle().is_alive())
            .map(|r| r.organism_id())
            .collect::<Vec<_>>();
        for id in ids {
            let mut record = self
                .world
                .organism_registry()
                .get(id)
                .cloned()
                .ok_or(CpuHostError::State("missing organism"))?;
            let r = self
                .residents
                .get_mut(&id.raw())
                .ok_or(CpuHostError::State("new organism requires brain admission"))?;
            let phase_before = r.sleep.state().phase;
            if r.activity
                .last_world_tick
                .is_some_and(|last| last.checked_add(1) != Some(tick.raw()))
            {
                return Err(CpuHostError::State("CPU activity world clock mismatch"));
            }
            r.activity.last_world_tick = Some(tick.raw());
            r.activity.remaining_atp_q16 =
                (f64::from(record.biochemistry().homeostasis.drives.brain_atp)
                    * f64::from(BRAIN_ATP_Q16_MAX))
                .floor() as u32;
            let neural_boundary = r.neural.checkpoint()?;
            let activity_anchor = CpuActivityAnchor {
                organism_id: id,
                tick,
                sequence_cursor: r.next_sequence,
                previous_dispatch_generation: neural_boundary.dispatch_generation,
                source_frame_digest: r.activity.source_frame_digest,
                frame_digest: p.phenotype_hash().0,
            };
            let capacity = BrainCapacityClass::production_for_id(p.brain_class_id())?;
            let lookahead = cpu_activity_budget(
                p,
                activity_anchor,
                record.biochemistry().homeostasis.drives.brain_atp,
                u32::from(capacity.execution().max_candidates()),
                u32::from(capacity.execution().max_memory_context_records()),
            )?;
            if phase_before == SleepPhase::Awake && !lookahead.affordable {
                r.sleep
                    .force_sleep(tick, alife_core::SleepTrigger::RecoveryProtocol)?;
            }

            let config = sleep_consolidation_config_for(p)?;
            if phase_before == SleepPhase::Awake {
                r.sleep.evaluate_homeostasis(
                    &record.biochemistry().homeostasis,
                    HomeostaticParameters::reference(),
                    tick,
                )?;
            } else if phase_before != SleepPhase::Waking
                || (record.biochemistry().homeostasis.drives.fatigue
                    < config.fatigue_threshold.raw()
                    && record.biochemistry().homeostasis.hormones.sleep_pressure
                        < config.sleep_pressure_threshold.raw())
            {
                r.sleep.advance(tick)?;
            }
            let mut sleep_work_units = 0;
            if r.sleep.state().phase == SleepPhase::Consolidating
                && matches!(r.sleep.state().consolidation, ConsolidationState::None)
            {
                let evidence = replay_evidence(p, &r.replay)?;
                if !evidence.batch.events.is_empty() {
                    // Same scheduled-work pressure projection used by the native
                    // scheduler; this is work admission, never reward/chemistry.
                    let mut due_homeostasis = record.biochemistry().homeostasis;
                    due_homeostasis.drives.fatigue = due_homeostasis
                        .drives
                        .fatigue
                        .max(config.fatigue_threshold.raw());
                    due_homeostasis.hormones.sleep_pressure = due_homeostasis
                        .hormones
                        .sleep_pressure
                        .max(config.sleep_pressure_threshold.raw());
                    let work = r.memory.run_bounded_sleep_transaction(
                        &SleepConsolidator::new(config)?,
                        &due_homeostasis,
                        tick,
                        &evidence,
                        &mut r.predictor,
                        &mut r.topology,
                    )?;
                    let structural = structural_sleep_evidence(p, &evidence.batch);
                    r.v11.apply_structural_phase(&structural)?;
                    let mut topology = r.v11.checkpoint();
                    topology.pending_lifetime_synapse = None;
                    r.v11 = GpuV11CausalState::restore(topology)?;
                    sleep_work_units = work.work_units;
                    let receipt = CognitiveWorkCounters::new(
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        0,
                        u64::from(work.predictor_update_count),
                        u64::from(work.replay_event_count)
                            + u64::from(work.replay_eligibility_sample_count),
                        0,
                        0,
                        work.work_units,
                    )?
                    .into_receipt()?;
                    record.account_cognitive_work(
                        receipt,
                        CognitiveWorkCostPolicy::enabled(1.0e-6)?,
                    )?;
                }
                r.neural.consolidate_sleep(
                    founder,
                    &evidence,
                    &record.phenotype().development_state_at(tick)?,
                    record.phenotype().brain_genome.plasticity_parameters(),
                )?;
                r.replay.clear();
                let checkpoint = r.neural.checkpoint()?;
                let digest = json_digest(b"ALIFE-CPU-SLEEP-OUTPUT-V1", &checkpoint)?;
                let mut state = r.sleep.state();
                state.consolidation = ConsolidationState::Committed {
                    cycle_id: state.active_cycle_id,
                    output_generation: checkpoint.weight_generation,
                    output_digest: digest,
                };
                r.sleep = SleepController::restore(config, state)?;
                consolidations += 1;
            }
            let state = r.sleep.state();
            record.seal_sleep_phase(
                state.phase,
                state.active_cycle_id.max(state.last_consolidated_cycle_id),
                tick,
                sleep_work_units,
            )?;
            self.world.replace_organism_record_exact(record.clone())?;
            if phase_before != SleepPhase::Awake {
                body_events.insert(
                    id.raw(),
                    BodyEventDelta {
                        sleep_recovery: 1.,
                        ..BodyEventDelta::zero()
                    },
                );
            }
            if phase_before != SleepPhase::Awake || state.phase != SleepPhase::Awake {
                sleeping += 1;
                continue;
            }
            let sequence = ExperienceSequenceId(r.next_sequence);
            let receptors = record
                .biochemistry()
                .neural_receptor_frame(record.phenotype())?;
            let effects = NeuralReceptorEffects::from_frame(
                &receptors,
                &NeuralReceptorPhenotype::compile(p)?,
            )?;
            let draft = perception_world.perception_frame_draft(
                id,
                tick,
                p.sensor_profile(),
                record.biochemistry().homeostasis,
            )?;
            let ordinary = draft.clone().with_semantic_context(None)?;
            let draft = prepare(draft, sequence)?;
            if draft.clone().with_semantic_context(None)? != ordinary {
                return Err(CpuHostError::State(
                    "semantic preparation changed ordinary perception",
                ));
            }
            let preparation = prepare_canonical_frame(
                &CanonicalPreparationInput {
                    draft,
                    sequence_id: sequence,
                    homeostasis: record.biochemistry().homeostasis,
                    hysteresis: r.hysteresis,
                    policy: attention_selection_policy_for(p),
                    neural_receptors: receptors.clone(),
                    receptor_effects: effects,
                },
                &r.memory,
                &r.topology,
                &r.predictor,
                false,
            );
            let prepared = preparation.result?;
            if let Some(h) = preparation.hysteresis {
                r.hysteresis = h;
            }
            let frame = prepared.frame;
            let recall = prepared.memory_recall;
            let activity = cpu_activity_budget(
                p,
                CpuActivityAnchor {
                    frame_digest: frame.frame_digest().0,
                    ..activity_anchor
                },
                record.biochemistry().homeostasis.drives.brain_atp,
                frame.candidates().len() as u32,
                recall.context().candidates.len() as u32,
            )?;
            if !activity.affordable {
                return Err(CpuHostError::State(
                    "CPU neural work exceeds canonical ATP availability",
                ));
            }
            let mut context = encode_cpu_frame(
                p,
                &frame,
                &recall,
                &effects,
                activity.enabled_routes.clone(),
                r.v11
                    .sparse_spans()
                    .iter()
                    .flat_map(|s| {
                        s.edges.iter().map(|e| crate::TrainingFrozenSynapse {
                            source: e.source,
                            target: e.target,
                            route: e.route,
                            cadence: u32::from(
                                p.projections()[e.route as usize].update_cadence().raw(),
                            ),
                            effective_weight: e.weight,
                        })
                    })
                    .collect(),
            )?;
            context.microstep_count = u32::from(activity.microsteps);
            let output = r.neural.advance(founder, context, &effects)?;
            r.activity.remaining_atp_q16 = activity.available_atp_q16 - activity.debit_atp_q16;
            r.activity.source_frame_digest = frame.frame_digest().0;
            let sampling_counter = self.sampling_counter;
            let action = sample_cpu_joint_action(
                p,
                &frame,
                &output.candidate_logits,
                self.sampling_seed,
                sampling_counter,
                temperature,
            )?;
            self.sampling_counter = self
                .sampling_counter
                .checked_add(1)
                .ok_or(CpuHostError::State("sampling counter overflow"))?;
            let speech = if action.motor_candidates[3].is_some() {
                decode_cpu_speech(
                    founder,
                    &r.neural.neural_state().activations,
                    &output.replay_tick.effective_weight_offsets,
                )?
            } else {
                None
            };
            let index = action.representative;
            let candidate = &frame.candidates()[index as usize];
            let confidence =
                Confidence::new((candidate.sensor_confidence.raw() * 65535.).round() / 65535.)?;
            let command = candidate.to_command(id, confidence)?;
            let channels = p.candidate_decoder().factorized_motor_channels(p)?;
            let slots = action.motor_candidates.map(|i| i.map_or(0, |i| i + 1));
            let prompted = frame
                .sensory()
                .language_context
                .heard_tokens
                .iter()
                .flatten()
                .any(|t| t.source_kind == UtteranceSourceKind::Player);
            let bundle = factorized_motor_bundle_for_candidates(
                id,
                sequence,
                tick,
                &frame,
                slots,
                &channels,
                &command,
                index,
                speech.as_ref(),
                prompted,
            )?;
            let joint = JointActionSelectionV1::new(slots, &bundle)?;
            let development = record.phenotype().development_state_at(tick)?;
            let pre = PreActionSnapshot::from_cpu_neural_frame(
                sequence,
                p.brain_class_id(),
                p.phenotype_hash(),
                record.phenotype().brain_genome.id,
                record.phenotype().brain_genome.schema_version,
                development.clone(),
                frame.clone(),
            )?;
            let work_neurons = activity.work.neuron_updates;
            let work_synapses = activity.work.synapse_ops;
            let decision = DecisionSnapshot::from_cpu_neural_selection(
                sequence,
                p.phenotype_hash(),
                output.dispatch_generation,
                output.active_activation_side,
                &frame,
                NeuralActionSelection {
                    candidate_index: index,
                    logit: output.candidate_logits[index as usize],
                    confidence,
                    active_tiles: u32::try_from(activity.work.tile_visits)
                        .map_err(|_| CpuHostError::State("CPU tile work overflow"))?,
                    active_synapses: u32::try_from(work_synapses)
                        .map_err(|_| CpuHostError::State("CPU synapse work overflow"))?,
                },
                command,
            )?
            .with_finalized_memory_recall(&frame, &recall, index)?;
            r.neural.stage_selection(
                p,
                &action,
                CpuDecisionBinding {
                    sequence_id: sequence,
                    frame_digest: frame.frame_digest(),
                    action_id: candidate.action_id,
                    candidate_feature_digest: candidate.feature_digest()?,
                    dispatch_generation: output.dispatch_generation,
                    active_activation_side: output.active_activation_side,
                    joint_selection: joint,
                },
                speech.is_some(),
            )?;
            let mut cognitive = recall
                .cognitive_context()
                .cloned()
                .ok_or(CpuHostError::State("missing canonical cognitive context"))?;
            let pre_context = cognitive.clone();
            let source = grounded_semantic_state_from_frame(&frame)?;
            let motor_condition = JointMotorCondition::from_bundle(&bundle)?;
            let mut forecast = r.predictor.predict(&source, &motor_condition)?;
            forecast.source_digest = frame.frame_digest().0;
            let mut neural_emissions = vec![
                NeuralEmission::new(
                    NeuralEmissionClass::RegionalArousal,
                    output.candidate_logits[index as usize].abs().tanh(),
                    confidence.raw(),
                )?,
                NeuralEmission::new(
                    NeuralEmissionClass::MotorCommitment,
                    confidence.raw(),
                    confidence.raw(),
                )?,
                NeuralEmission::new(NeuralEmissionClass::ExecutiveSustain, confidence.raw(), 1.)?,
            ];
            neural_emissions.extend(recall.recognized_social_emission());
            let emissions =
                NeuralEmissionFrame::new(tick, output.dispatch_generation, neural_emissions)?;
            let object_novelty = bundle
                .channels
                .iter()
                .filter_map(|c| c.target.and_then(|t| t.entity))
                .filter_map(|entity| {
                    let key = perception_world
                        .entity(entity)?
                        .tracking_provenance
                        .canonical_key();
                    let tracked = perception_world
                        .tracked_objects()
                        .records_for(id)?
                        .find(|r| r.tracking_key == key)?;
                    pre_context
                        .peripheral
                        .summaries
                        .iter()
                        .find(|s| {
                            s.identity
                                == StableFocusIdentity::TrackedObject(tracked.tracked_object_id)
                        })
                        .map(|s| (entity, s.salience.novelty.raw()))
                })
                .collect::<Vec<_>>();
            let receipt = self
                .world
                .apply_registered_motor_bundle_with_perceived_novelty_in_staged_tick(
                    &bundle,
                    record.world_entity_id(),
                    &emissions,
                    frame.sensory().channels.novelty_signal.raw(),
                    &object_novelty,
                )
                .map_err(|_| CpuHostError::State("canonical motor transaction rejected"))?;
            let successor = grounded_successor_state(
                &self.world,
                record.world_entity_id(),
                &receipt.biology_after,
                p.sensor_profile(),
            )?;
            let target = PredictionTargetReceipt::for_successor(
                id,
                sequence,
                candidate.action_id,
                tick,
                frame.frame_digest().0,
                source,
                motor_condition,
                successor,
            )?;
            let prediction = r.predictor.observe_frozen(&target, &forecast)?;
            let error = apply_prediction_evidence(
                &mut cognitive,
                &target,
                &prediction.error,
                prediction.prediction.category_coverage,
            )?;
            let memory = recall.receipt();
            let dendritic_work = r
                .v11
                .dendritic_branches()
                .branches()
                .iter()
                .map(|b| 1 + b.inputs.len() as u64)
                .sum::<u64>()
                * u64::from(activity.microsteps);
            let structural_work = r
                .v11
                .sparse_spans()
                .iter()
                .flat_map(|s| &s.edges)
                .map(|e| {
                    if !activity.enabled_routes[e.route as usize] {
                        return 0;
                    }
                    (0..activity.microsteps as usize)
                        .filter(|&step| {
                            super::replay::route_fires(
                                p.projections()[e.route as usize].update_cadence().raw(),
                                step,
                            )
                        })
                        .count() as u64
                })
                .sum::<u64>();
            let work = CognitiveWorkCounters::new(
                work_neurons,
                work_synapses,
                dendritic_work,
                cognitive.attention.budget_receipt.work_units,
                u64::from(memory.exact_bucket_reads)
                    + u64::from(memory.neighbor_bucket_reads)
                    + u64::from(memory.similarity_evaluations),
                cognitive.concept.active_concepts.len() as u64,
                cognitive.gap.active_gaps.len() as u64,
                prediction.error.len() as u64,
                0,
                structural_work,
                1,
                0,
            )?
            .into_receipt()?;
            let mut after = self
                .world
                .organism_registry()
                .get(id)
                .cloned()
                .ok_or(CpuHostError::State("missing post-action organism"))?;
            after.account_cognitive_work(work, CognitiveWorkCostPolicy::enabled(1.0e-6)?)?;
            after.stage_prediction_residual_pulse(
                sequence,
                output.dispatch_generation,
                error,
                1.,
            )?;
            self.world.replace_organism_record_exact(after.clone())?;
            let physiology =
                MeasuredPhysiologyTransition::new(receipt.biology_before, *after.biochemistry())?;
            let mut outcome = PostActionOutcome::new(
                id,
                sequence,
                receipt.outcome_tick,
                receipt.succeeded,
                receipt.joint.execution,
                physiology.homeostatic_delta,
                SignedValence::ZERO,
                NormalizedScalar::new(if receipt.succeeded { 0. } else { 1. })?,
                NormalizedScalar::new(physiology.aversive_harm())?,
                physiology.energy_delta,
                NormalizedScalar::new(error)?,
            )?
            .with_measured_physiology(physiology)?;
            outcome.contradiction_observed = !receipt.succeeded;
            outcome = outcome.with_v12_joint(receipt.joint, work)?;
            let patch = ExperiencePatch::new_v12_with_decision(
                pre,
                decision,
                bundle,
                outcome,
                target.clone(),
                work,
                pre_context,
            )?;
            let credit_receptors = receptors;
            r.neural.apply_credit(
                founder,
                &patch,
                &credit_receptors,
                &development,
                after.phenotype().brain_genome.plasticity_parameters(),
            )?;
            r.language_grounding.observe_sealed(&patch)?;
            r.memory.observe_sealed_patch(&patch)?;
            let topology_receipt = r.topology.observe_sealed_patch(&patch);
            if topology_receipt.rejected_invalid || topology_receipt.replay_rejected {
                return Err(CpuHostError::State("sealed topology observation rejected"));
            }
            r.topology.advance_lifecycle(receipt.outcome_tick)?;
            let checkpoint = r.neural.checkpoint()?;
            let credit = OutcomeCreditPacket::from_sealed_patch(&patch)?
                .with_biochemical_receptors(&credit_receptors)?;
            let selected = patch.decision().neural_evidence()?;
            r.replay.push(CpuReplayEvent {
                event: SleepReplayEvent {
                    sequence_id: sequence,
                    originating_tick: tick,
                    frame_digest: frame.frame_digest(),
                    candidate_feature_digest: candidate.feature_digest()?,
                    action_id: candidate.action_id,
                    family: selected.action_family,
                    modulator: credit.modulator(),
                },
                eligibility: p
                    .replay_capture_plan()
                    .global_synapse_ids()
                    .iter()
                    .map(|&id| {
                        let synapse = &p.synapses()[id as usize];
                        let decay = p.plasticity_receptors()[synapse.receptor_index() as usize]
                            .eligibility_decay();
                        let elapsed = (receipt.outcome_tick.raw() - tick.raw())
                            .min(u64::from(u32::MAX)) as f32;
                        (checkpoint.eligibility[id as usize] * decay.powf(elapsed))
                            .clamp(-1., 1.)
                            .mul_add(32767., 0.)
                            .round() as i16
                    })
                    .collect(),
                target,
            });
            if r.replay.len() > p.replay_capture_plan().event_capacity() as usize {
                r.replay.remove(0);
            }
            after.seal_cognitive_subsystems(
                receipt.outcome_tick,
                json_digest(b"ALIFE-CPU-NEURAL-STATE-V1", &checkpoint)?,
                r.memory.export_active_bank()?.active_bank_digest,
            )?;
            let parameters = after.phenotype().brain_genome.plasticity_parameters();
            let profile = parameters
                .action_candidate_credit_profile()
                .map_or_else(|| parameters.receptor_profile(), |p| p.receptor_profile());
            let biological_reward = profile.project(&credit.modulator().frame())?;
            self.world.replace_organism_record_exact(after)?;
            r.next_sequence = r
                .next_sequence
                .checked_add(1)
                .ok_or(CpuHostError::State("sequence overflow"))?;
            decisions.push(CpuCollectedDecision {
                organism_id: id,
                sampling_counter: Some(sampling_counter),
                initial: output.initial,
                replay_tick: output.replay_tick,
                action,
                final_features: checkpoint.neural.activations.clone(),
                brain_work: activity.work,
                biological_reward,
                patch,
            });
        }
        // The batch uses one pre-action scene. Commit the identities actually
        // observed there along with its decisions; priming stays read-only.
        let tracked_objects = self
            .residents
            .keys()
            .map(|&id| {
                perception_world
                    .tracked_objects()
                    .save_state(OrganismId(id))
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.world.restore_tracked_object_states(tracked_objects)?;
        let tick_after = self
            .world
            .try_advance_tick_with_body_events_in_staged_tick(&body_events)?;
        self.seal_all_residents()?;
        Ok(CpuCanonicalStep {
            tick_before: tick,
            tick_after,
            decisions,
            sleeping_organisms: sleeping,
            sleep_consolidations: consolidations,
        })
    }
}
fn replay_evidence(
    p: &BrainPhenotype,
    events: &[CpuReplayEvent],
) -> Result<SleepReplayEvidence, ScaffoldContractError> {
    let mut spans = Vec::new();
    let mut samples = Vec::new();
    for (lane, &id) in p
        .replay_capture_plan()
        .global_synapse_ids()
        .iter()
        .enumerate()
    {
        let start = samples.len() as u32;
        for (index, event) in events.iter().enumerate() {
            samples.push(ReplayEligibilitySample {
                event_index: index as u16,
                eligibility_q15: *event
                    .eligibility
                    .get(lane)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?,
            });
        }
        spans.push(ReplaySynapseSpan {
            local_synapse_id: id,
            sample_start: start,
            sample_count: events.len() as u32,
            reserved: 0,
        });
    }
    let mut batch = BoundedReplayBatch {
        schema_version: 1,
        events: events.iter().map(|e| e.event).collect(),
        synapse_spans: spans,
        eligibility_samples: samples,
        canonical_digest: [0; 4],
    };
    batch.canonical_digest = batch.recompute_canonical_digest()?;
    SleepReplayEvidence::new(batch, events.iter().map(|e| e.target.clone()).collect())
}

pub(super) fn validate_record_founder(
    record: &alife_world::WorldOrganismRecord,
    founder: &CpuFoundationTrainer,
) -> Result<(), CpuHostError> {
    let p = founder.phenotype();
    let genome = &record.phenotype().brain_genome;
    if genome.brain_class_id != p.brain_class_id() {
        return Err(CpuHostError::State(
            "organism brain class differs from CPU founder",
        ));
    }
    let capacity = BrainCapacityClass::production_for_id(genome.brain_class_id)?;
    let development = foundation_construction_development(
        genome,
        &capacity,
        &record
            .phenotype()
            .development_state_at(record.biochemistry().tick)?,
    )?;
    let (compiled, _) = PhenotypeCompiler::compile_n2048_foundation_candidate(
        genome.clone(),
        development,
        founder.source_foundation.clone(),
    )?;
    if compiled.phenotype_hash() != p.phenotype_hash() {
        return Err(CpuHostError::State("organism compiler identity differs from CPU founder; heterogeneous graphs require per-row compilation"));
    }
    let inherited =
        record
            .genome()
            .n2048_foundation_candidate
            .as_ref()
            .ok_or(CpuHostError::State(
                "canonical organism lacks inherited candidate asset",
            ))?;
    if inherited.encode_canonical()? != founder.source_foundation.encode_canonical()? {
        return Err(CpuHostError::State(
            "organism inherited foundation differs from CPU founder",
        ));
    }
    Ok(())
}

fn resident_digest(
    r: &Resident,
    sampling_seed: u32,
    sampling_counter: u32,
) -> Result<[u64; 4], CpuHostError> {
    json_digest(
        b"ALIFE-CPU-CANONICAL-PRIVATE-STATE-V1",
        &(
            r.neural.checkpoint()?,
            r.v11.checkpoint(),
            &r.predictor,
            &r.activity,
            &r.sleep,
            &r.hysteresis,
            r.next_sequence,
            &r.replay,
            &r.language_grounding,
            r.topology.export_portable()?,
            *r.memory.compaction_checkpoint(),
            r.memory.export_staged_bank()?,
            r.memory.export_active_bank()?,
            sampling_seed,
            sampling_counter,
        ),
    )
}
fn validate_resident_authority(
    record: &alife_world::WorldOrganismRecord,
    r: &Resident,
    sampling_seed: u32,
    sampling_counter: u32,
) -> Result<(), CpuHostError> {
    if record.state_graph().brain.content_digest
        != resident_digest(r, sampling_seed, sampling_counter)?
        || record.state_graph().memory.content_digest
            != r.memory.export_active_bank()?.active_bank_digest
    {
        return Err(CpuHostError::State(
            "canonical organism authority differs from private CPU state",
        ));
    }
    Ok(())
}

fn structural_sleep_evidence(
    p: &BrainPhenotype,
    replay: &BoundedReplayBatch,
) -> Vec<alife_core::CoactivationEvidence> {
    let base_pairs = p
        .synapses()
        .iter()
        .map(|synapse| (synapse.source(), synapse.target()))
        .collect::<std::collections::BTreeSet<_>>();
    let active = replay
        .synapse_spans
        .iter()
        .filter_map(|span| {
            let start = usize::try_from(span.sample_start).ok()?;
            let count = usize::try_from(span.sample_count).ok()?;
            let samples = replay
                .eligibility_samples
                .get(start..start.checked_add(count)?)?;
            let eligibility = samples
                .iter()
                .map(|sample| u32::from(sample.eligibility_q15.unsigned_abs()))
                .max()
                .unwrap_or(0);
            if eligibility == 0 {
                return None;
            }
            let synapse = p
                .synapses()
                .get(usize::try_from(span.local_synapse_id).ok()?)?;
            Some((synapse.source(), synapse.target(), eligibility))
        })
        .take(32)
        .collect::<Vec<_>>();
    // Local growth proposals are anchored in replay eligibility. Unit
    // coactivation is proposal support, not an observed firing count.
    const CANONICAL_SLEEP_STRUCTURAL_REGION: u16 = 0;
    let mut structural_evidence = Vec::new();
    for pair in active.windows(2).take(32) {
        let (source, target, eligibility) = (pair[0].0, pair[1].1, pair[0].2);
        if source == target || base_pairs.contains(&(source, target)) {
            continue;
        }
        structural_evidence.push(CoactivationEvidence {
            region: CANONICAL_SLEEP_STRUCTURAL_REGION,
            source,
            target,
            coactivation: 1,
            eligibility,
            concept_gap_support: 0,
        });
    }
    if structural_evidence.is_empty() {
        if let Some((source, target, eligibility)) = active.first().copied() {
            for offset in 1..=8_u32 {
                let candidate_target =
                    (target % p.neuron_count()).wrapping_add(offset) % p.neuron_count();
                if candidate_target == source || base_pairs.contains(&(source, candidate_target)) {
                    continue;
                }
                structural_evidence.push(CoactivationEvidence {
                    region: CANONICAL_SLEEP_STRUCTURAL_REGION,
                    source,
                    target: candidate_target,
                    coactivation: 1,
                    eligibility,
                    concept_gap_support: 0,
                });
                break;
            }
        }
    }
    structural_evidence
}
