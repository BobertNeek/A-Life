//! GPU-authoritative live cognition for the explicit neural policy.

use std::collections::{BTreeMap, BTreeSet};

use alife_core::{
    BrainCapacityClass, BrainGenome, BrainScaleTier, BrainTickStatus, DecisionSnapshot,
    DevelopmentState, ExperiencePatch, ExperiencePatchBuilder, ExperienceSequenceId,
    HomeostaticParameters, HomeostaticSnapshot, NeuralActionSelection, NormalizedScalar,
    OrganismId, PerceptionFrame, PhenotypeCompiler, PostActionOutcome, PreActionSnapshot,
    ScaffoldContractError, SensorProfile, Tick, Validate,
};
use alife_gpu_backend::{GpuBrainHandle, GpuClosedLoopBackend, GpuClosedLoopTick};
use alife_world::{
    persistence::{AssetManifest, PortableSaveFile, RuntimeConfig},
    HeadlessWorld,
};

use crate::{
    AppShellLaunchConfig, GameAppShellError, LiveBrainCausalStage, LiveBrainTickSummary,
    G03_LIVE_BRAIN_LOOP_SCHEMA, G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
};

#[derive(Debug, Clone)]
struct ResidentCognition {
    genome: BrainGenome,
    development: DevelopmentState,
    homeostasis: HomeostaticSnapshot,
    next_sequence: u64,
}

/// Owns all production neural authority for one headless world.
pub struct GpuLiveBrainRuntime {
    backend: GpuClosedLoopBackend,
    handles: BTreeMap<u64, GpuBrainHandle>,
    residents: BTreeMap<u64, ResidentCognition>,
    world: HeadlessWorld,
    deterministic_seed: u64,
    brain_class: BrainScaleTier,
    sealed_patches: Vec<ExperiencePatch>,
}

impl GpuLiveBrainRuntime {
    pub fn from_p34_launch(
        backend: GpuClosedLoopBackend,
        launch: &AppShellLaunchConfig,
    ) -> Result<Self, GameAppShellError> {
        let config = RuntimeConfig::from_json_file(&launch.config_path)?;
        config.validate()?;
        let manifest = AssetManifest::from_json_file(&launch.asset_manifest_path)?;
        manifest.validate_with_root(&launch.asset_root)?;
        let save = PortableSaveFile::from_json_file(&launch.save_path)?;
        save.validate_with_asset_root(&launch.asset_root)?;
        if launch.brain_policy != alife_core::PolicyBackend::NeuralClosedLoopGpu
            || config.brain_policy.policy != alife_core::PolicyBackend::NeuralClosedLoopGpu
            || save.config.brain_policy.policy != alife_core::PolicyBackend::NeuralClosedLoopGpu
            || config.deterministic_seed != save.deterministic_seed
        {
            return Err(GameAppShellError::InvalidGraphicalLaunch {
                message: "GPU neural runtime requires matching persisted neural policy and seed",
            });
        }
        let world = save.restore_headless_world()?;
        let world_tick = world.tick();
        let mut runtime = Self::new(
            backend,
            world,
            config.deterministic_seed,
            config.brain_class,
        )?;
        for creature in &save.creatures {
            let Some(resident) = runtime.residents.get_mut(&creature.organism_id.raw()) else {
                return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
            };
            if creature.brain_class != config.brain_class {
                return Err(ScaffoldContractError::PhenotypeCompile.into());
            }
            resident.homeostasis = HomeostaticSnapshot::new(
                world_tick,
                creature.mind.homeostasis.drives,
                creature.mind.homeostasis.hormones,
            )?;
        }
        Ok(runtime)
    }

    pub fn new(
        backend: GpuClosedLoopBackend,
        world: HeadlessWorld,
        deterministic_seed: u64,
        brain_class: BrainScaleTier,
    ) -> Result<Self, GameAppShellError> {
        if deterministic_seed == 0 || brain_class.neuron_count().is_none() {
            return Err(GameAppShellError::Core(
                ScaffoldContractError::PhenotypeCompile,
            ));
        }
        let mut runtime = Self {
            backend,
            handles: BTreeMap::new(),
            residents: BTreeMap::new(),
            world,
            deterministic_seed,
            brain_class,
            sealed_patches: Vec::new(),
        };
        runtime.reconcile_population()?;
        Ok(runtime)
    }

    pub fn reconcile_population(&mut self) -> Result<(), GameAppShellError> {
        let live_ids = self
            .world
            .organism_entity_ids()
            .into_iter()
            .map(|(organism_id, _)| organism_id.raw())
            .collect::<BTreeSet<_>>();

        let retired = self
            .handles
            .keys()
            .copied()
            .filter(|raw| !live_ids.contains(raw))
            .collect::<Vec<_>>();
        for raw in retired {
            let handle = *self
                .handles
                .get(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            self.backend.remove_brain(handle)?;
            self.handles.remove(&raw);
            self.residents.remove(&raw);
        }

        for raw in live_ids {
            if self.handles.contains_key(&raw) {
                if !self.residents.contains_key(&raw) {
                    return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
                }
                continue;
            }
            let organism_id = OrganismId(raw);
            let (phenotype, resident) = self.compile_birth(organism_id)?;
            let handle = self.backend.insert_brain(organism_id, phenotype)?;
            if handle.organism_id().raw() != raw {
                self.backend.remove_brain(handle)?;
                return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
            }
            self.handles.insert(raw, handle);
            self.residents.insert(raw, resident);
        }
        Ok(())
    }

    pub fn tick(&mut self) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError> {
        self.reconcile_population()?;
        if self.handles.is_empty() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU neural policy requires at least one live organism",
            });
        }

        let tick_before = self.world.tick();
        let mut batch = Vec::with_capacity(self.handles.len());
        for (&raw, &handle) in &self.handles {
            let resident = self
                .residents
                .get_mut(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if resident.homeostasis.tick != tick_before {
                resident.homeostasis.tick = tick_before;
                resident.homeostasis.validate_contract()?;
            }
            let frame = self.world.perception_frame(
                OrganismId(raw),
                tick_before,
                SensorProfile::PrivilegedAffordanceV1,
                resident.homeostasis,
            )?;
            batch.push((handle, frame));
        }

        let gpu_ticks = self.backend.tick_batch(&batch)?;
        if gpu_ticks.len() != batch.len() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }

        let mut summaries = Vec::with_capacity(gpu_ticks.len());
        for ((handle, frame), gpu_tick) in batch.into_iter().zip(gpu_ticks) {
            summaries.push(self.apply_selection(handle, frame, gpu_tick)?);
        }
        self.world.advance_tick();
        Ok(summaries)
    }

    pub fn sealed_patches(&self) -> &[ExperiencePatch] {
        &self.sealed_patches
    }

    fn compile_birth(
        &self,
        organism_id: OrganismId,
    ) -> Result<(alife_core::BrainPhenotype, ResidentCognition), GameAppShellError> {
        let capacity = BrainCapacityClass::production_for_id(self.brain_class.default_class_id())?;
        let birth_seed = self.deterministic_seed ^ organism_id.raw().rotate_left(17);
        let genome = BrainGenome::scaffold(birth_seed, capacity.id());
        let development =
            DevelopmentState::new(genome.id, self.world.tick(), NormalizedScalar::new(0.35)?);
        let phenotype = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::PrivilegedAffordanceV1,
        )?;
        let resident = ResidentCognition {
            genome,
            development,
            homeostasis: HomeostaticSnapshot::baseline(self.world.tick()),
            next_sequence: 1,
        };
        Ok((phenotype, resident))
    }

    fn apply_selection(
        &mut self,
        handle: GpuBrainHandle,
        frame: PerceptionFrame,
        gpu_tick: GpuClosedLoopTick,
    ) -> Result<LiveBrainTickSummary, GameAppShellError> {
        if gpu_tick.handle != handle
            || gpu_tick.base_digest != frame.base_digest()
            || gpu_tick.frame_digest != frame.frame_digest()
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        let organism_id = handle.organism_id();
        let resident = self
            .residents
            .get_mut(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let sequence_id = ExperienceSequenceId(resident.next_sequence);
        sequence_id.validate()?;
        let candidate = *frame
            .candidates()
            .get(usize::from(gpu_tick.selection.candidate_index))
            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
        let command = candidate.to_command(organism_id, gpu_tick.selection.confidence)?;
        let pre_action = PreActionSnapshot::from_neural_frame(
            sequence_id,
            handle.class_id(),
            handle.phenotype_hash(),
            resident.genome.id,
            resident.genome.schema_version,
            resident.development.clone(),
            frame.clone(),
        )?;
        let decision = DecisionSnapshot::from_neural_selection(
            sequence_id,
            handle.phenotype_hash(),
            gpu_tick.dispatch_generation,
            gpu_tick.active_activation_side,
            &frame,
            NeuralActionSelection {
                candidate_index: gpu_tick.selection.candidate_index,
                logit: gpu_tick.selection.logit,
                confidence: gpu_tick.selection.confidence,
                active_tiles: gpu_tick.selection.active_tiles,
                active_synapses: gpu_tick.selection.active_synapses,
            },
            command,
        )?;
        let action_result = self.world.apply_command(&decision.selected_action)?;
        let outcome_tick = Tick::new(frame.tick().raw().saturating_add(1));
        let mut outcome = PostActionOutcome::new(
            organism_id,
            sequence_id,
            outcome_tick,
            action_result.observation.success && action_result.execution.succeeded,
            action_result.execution.physical,
            action_result.observation.homeostatic_delta,
            action_result.observation.reward_valence,
            action_result.observation.frustration_delta,
            action_result.observation.pain_delta,
            action_result.observation.energy_delta,
            action_result.observation.prediction_error,
        )?;
        outcome.contradiction_observed =
            action_result.observation.contradiction_observed || !action_result.execution.succeeded;
        outcome.validate_contract()?;
        let patch = ExperiencePatchBuilder::new(sequence_id)
            .record_pre_action(pre_action)?
            .record_decision(decision.clone())?
            .record_outcome(outcome)?
            .seal()?;
        resident.homeostasis = resident.homeostasis.advance(
            outcome_tick,
            patch.outcome().homeostatic_delta,
            HomeostaticParameters::reference(),
        )?;
        resident.development.age_ticks = outcome_tick;
        resident.next_sequence = resident
            .next_sequence
            .checked_add(1)
            .ok_or(ScaffoldContractError::InvalidId)?;
        self.sealed_patches.push(patch.clone());

        Ok(LiveBrainTickSummary {
            schema: G03_LIVE_BRAIN_LOOP_SCHEMA,
            schema_version: G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION,
            organism_id,
            tick_before: frame.tick(),
            tick_after: outcome_tick,
            world_tick_before: frame.tick(),
            world_tick_after: outcome_tick,
            status: BrainTickStatus::Normal,
            selected_action_kind: Some(decision.selected_action.kind),
            selected_action_id: Some(decision.selected_action.action_id),
            target_entity: decision.selected_action.target_entity,
            patch_sealed: true,
            patch_sequence_id: Some(sequence_id.raw()),
            patch_success: Some(patch.outcome().success),
            physical_contact: Some(patch.outcome().physical.contact),
            action_failure: action_result.execution.failure,
            sealed_patch_count: self.sealed_patches.len(),
            packed_record_count: 0,
            memory_updates: 0,
            topology_updates: 0,
            learning_updates: 0,
            invalid_or_rejected_action_count: u32::from(!action_result.execution.succeeded),
            last_diagnostic: None,
            causal_stages: vec![
                LiveBrainCausalStage::GatherSensory,
                LiveBrainCausalStage::GpuBrainTick,
                LiveBrainCausalStage::ExecuteAction,
                LiveBrainCausalStage::MeasureOutcome,
                LiveBrainCausalStage::SealPatch,
                LiveBrainCausalStage::UpdateLogs,
            ],
        })
    }

    #[cfg(test)]
    pub(crate) fn handle_for(&self, organism_id: OrganismId) -> Option<GpuBrainHandle> {
        self.handles.get(&organism_id.raw()).copied()
    }

    #[cfg(test)]
    pub(crate) fn world_mut(&mut self) -> &mut HeadlessWorld {
        &mut self.world
    }

    #[cfg(test)]
    pub(crate) fn test_tick_retired_handle(
        &mut self,
        handle: GpuBrainHandle,
        frame: PerceptionFrame,
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        self.backend.tick_batch(&[(handle, frame)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alife_core::{PreActionBrainEvidence, Vec3f};
    use alife_world::HeadlessScenarioBuilder;

    #[test]
    fn organism_despawn_retires_its_gpu_handle_before_slot_reuse() {
        let backend = GpuClosedLoopBackend::new_required().expect("required GPU");
        let world = HeadlessScenarioBuilder::new(91)
            .agent("one", OrganismId(1), Vec3f::ZERO)
            .agent("two", OrganismId(2), Vec3f::new(2.0, 0.0, 0.0))
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 91, BrainScaleTier::Nano512).unwrap();
        let retired = runtime.handle_for(OrganismId(1)).unwrap();
        let retired_frame = runtime
            .world
            .perception_frame(
                OrganismId(1),
                Tick::ZERO,
                SensorProfile::PrivilegedAffordanceV1,
                HomeostaticSnapshot::baseline(Tick::ZERO),
            )
            .unwrap();
        runtime.world_mut().remove_organism(OrganismId(1)).unwrap();
        runtime.reconcile_population().unwrap();

        assert!(runtime.handle_for(OrganismId(1)).is_none());
        assert!(runtime
            .test_tick_retired_handle(retired, retired_frame)
            .is_err());
    }

    #[test]
    fn gpu_tick_executes_and_seals_neural_evidence_before_world_advance() {
        let backend = GpuClosedLoopBackend::new_required().expect("required GPU");
        let world = HeadlessScenarioBuilder::new(92)
            .agent("agent", OrganismId(1), Vec3f::ZERO)
            .food("food", Vec3f::new(1.0, 0.0, 0.0), 0.8)
            .build()
            .unwrap();
        let mut runtime =
            GpuLiveBrainRuntime::new(backend, world, 92, BrainScaleTier::Nano512).unwrap();

        let summaries = runtime.tick().unwrap();

        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].patch_sealed);
        assert_eq!(runtime.backend.completed_dispatch_count(), 1);
        assert_eq!(runtime.world.tick(), Tick::new(1));
        assert_eq!(runtime.sealed_patches().len(), 1);
        assert!(matches!(
            runtime.sealed_patches()[0].pre_action().brain_evidence,
            PreActionBrainEvidence::NeuralClosedLoopGpu { .. }
        ));
    }
}
