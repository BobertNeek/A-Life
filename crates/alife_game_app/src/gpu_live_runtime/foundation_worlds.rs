//! Baseline collector: independent ordinary worlds sharing one GPU device.
//!
//! Sessions and dispatches remain separate, and rounds are serial. This is not
//! fused cross-world inference or a claimed throughput improvement.
use super::*;

pub struct FoundationTrainingWorldSpec {
    pub world: HeadlessWorld,
    pub deterministic_seed: u64,
    pub brain_class: BrainScaleTier,
    pub sensor_profile: SensorProfile,
    pub archive_config: LineageLibraryConfig,
    pub source_run_id: String,
    pub learned_capture_policy: ArchiveLearnedCapturePolicy,
    pub sampling: alife_gpu_backend::GpuTrainingSamplingConfig,
}

pub struct FoundationTrainingWorldRound {
    pub world_index: usize,
    pub tick_before: Tick,
    pub tick_after: Tick,
    pub outcome: GpuLiveTickOutcome,
    pub steps: Vec<FoundationTrainingStep>,
}

pub struct FoundationTrainingDeviceRound {
    /// Includes ordinary world simulation, GPU work, and requested readback.
    pub elapsed_seconds: f64,
    pub worlds: Vec<FoundationTrainingWorldRound>,
}

/// Bounded comparison baseline for one through eight independent worlds.
/// Each world keeps its own world transaction, archive, session and eligibility.
/// Organism IDs must already be globally unique; identities are never rewritten.
pub struct FoundationTrainingSharedDevice {
    runtimes: Vec<GpuLiveBrainRuntime>,
    organism_owners: BTreeMap<u64, usize>,
    failed: bool,
}

impl FoundationTrainingSharedDevice {
    pub fn new(
        backend: GpuClosedLoopBackend,
        worlds: Vec<FoundationTrainingWorldSpec>,
    ) -> Result<Self, GameAppShellError> {
        if !(1..=8).contains(&worlds.len()) {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let mut owners = BTreeMap::new();
        for (index, spec) in worlds.iter().enumerate() {
            spec.sampling.validate()?;
            spec.world.validate_organism_bindings()?;
            for record in spec.world.organism_registry().iter() {
                if owners.insert(record.organism_id().raw(), index).is_some() {
                    return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
                }
            }
        }
        // Bound the whole baseline by the existing profile's resident ceiling.
        if owners.len() > backend.runtime_profile().max_hot_brains as usize {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let mut backends = Vec::with_capacity(worlds.len());
        for _ in 1..worlds.len() {
            backends.push(backend.new_staging_like_live()?);
        }
        backends.insert(0, backend);
        let mut runtimes = Vec::with_capacity(worlds.len());
        for (backend, spec) in backends.into_iter().zip(worlds) {
            runtimes.push(GpuLiveBrainRuntime::new_profiled_foundation_training(
                backend,
                spec.world,
                spec.deterministic_seed,
                spec.brain_class,
                spec.sensor_profile,
                spec.archive_config,
                spec.source_run_id,
                spec.learned_capture_policy,
                spec.sampling,
            )?);
        }
        let cohort = Self {
            runtimes,
            organism_owners: owners,
            failed: false,
        };
        cohort.validate_device_budget()?;
        Ok(cohort)
    }

    pub fn world_count(&self) -> usize {
        self.runtimes.len()
    }

    /// Configure ordinary teacher/environment interactions between sealed rounds.
    /// The next round revalidates organism ownership before any neural work.
    pub fn runtime_mut(&mut self, world_index: usize) -> Option<&mut GpuLiveBrainRuntime> {
        if self.failed {
            return None;
        }
        self.runtimes.get_mut(world_index)
    }

    fn validate_ownership(&mut self) -> Result<(), GameAppShellError> {
        self.validate_device_budget()?;
        for (index, runtime) in self.runtimes.iter().enumerate() {
            for record in runtime.world.organism_registry().iter() {
                let raw = record.organism_id().raw();
                match self.organism_owners.get(&raw) {
                    Some(owner) if *owner != index => {
                        return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
                    }
                    Some(_) => {}
                    None => {
                        self.organism_owners.insert(raw, index);
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_device_budget(&self) -> Result<(), GameAppShellError> {
        let profile = self.runtimes[0].backend.runtime_profile();
        let (mut logical, mut physical, mut brains) = (0u64, 0u64, 0u64);
        for runtime in &self.runtimes {
            let receipt = runtime.backend.admission_receipt();
            logical = logical
                .checked_add(receipt.logical_committed_bytes)
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            physical = physical
                .checked_add(receipt.physical_allocated_bytes)
                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
            brains += u64::from(receipt.live_brains);
        }
        if logical > profile.logical_neural_heap_budget_bytes
            || physical > profile.physical_allocation_ceiling_bytes
            || brains > u64::from(profile.max_hot_brains)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        Ok(())
    }

    /// Calls each runtime's existing transactional tick, then drains sealed rows.
    /// A failed partial round is terminal; callers must not replay earlier worlds.
    /// No cohort-wide rollback is claimed across independently committed worlds.
    pub fn tick_round(&mut self) -> Result<FoundationTrainingDeviceRound, GameAppShellError> {
        if self.failed {
            return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
        }
        let started = Instant::now();
        let result = (|| {
            self.validate_ownership()?;
            let mut worlds = Vec::with_capacity(self.runtimes.len());
            for (world_index, runtime) in self.runtimes.iter_mut().enumerate() {
                let tick_before = runtime.world.tick();
                let outcome = runtime.tick_outcome()?;
                let tick_after = runtime.world.tick();
                let steps = runtime.take_foundation_training_steps();
                for step in &steps {
                    if self.organism_owners.get(&step.frame.organism_id().raw())
                        != Some(&world_index)
                        || step.behavior.organism_id != step.frame.organism_id().raw()
                        || step.behavior.tick != step.frame.tick().raw()
                        || step.patch.header().organism_id != step.frame.organism_id()
                    {
                        return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
                    }
                }
                worlds.push(FoundationTrainingWorldRound {
                    world_index,
                    tick_before,
                    tick_after,
                    outcome,
                    steps,
                });
            }
            self.validate_ownership()?;
            Ok(FoundationTrainingDeviceRound {
                elapsed_seconds: started.elapsed().as_secs_f64(),
                worlds,
            })
        })();
        if result.is_err() {
            self.failed = true;
            for runtime in &mut self.runtimes {
                runtime
                    .backend
                    .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                runtime.last_foundation_training_steps.clear();
            }
        }
        result
    }
}
