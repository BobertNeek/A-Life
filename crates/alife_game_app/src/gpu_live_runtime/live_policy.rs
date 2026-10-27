//! Same-individual learned policy binding. This does not edit the world record,
//! biological genes, memory, recurrent/lifetime banks or exploration counter.
use super::*;

#[cfg(feature = "foundation-training")]
fn decode_sampling(
    bytes: &[u8],
) -> Result<alife_gpu_backend::GpuTrainingSamplingConfig, GameAppShellError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Wire {
        seed: u32,
        counter: u32,
        temperature: f32,
        demonstrator: Option<alife_gpu_backend::GpuTrainingDemonstratorAction>,
    }
    if bytes.len() > 4096 {
        return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
    }
    let wire: Wire = serde_json::from_slice(bytes)?;
    let sampling = alife_gpu_backend::GpuTrainingSamplingConfig {
        seed: wire.seed,
        counter: wire.counter,
        temperature: wire.temperature,
        demonstrator: wire.demonstrator,
    };
    sampling.validate()?;
    if sampling.demonstrator.is_some() {
        return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
    }
    Ok(sampling)
}

pub(super) fn adopt_exact_live_policy_checkpoint(
    plan: &mut ResidentAuthorityPlan,
    checkpoint: ResidentCheckpointMetadata<'_>,
) -> Result<(), ScaffoldContractError> {
    let provenance = checkpoint
        .compiler_inputs
        .live_policy()
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    let birth = plan
        .compiler_inputs
        .n2048_candidate_asset()
        .ok_or(ScaffoldContractError::PhenotypeCompile)?;
    if checkpoint.organism_id != plan.organism_id
        || checkpoint.checkpoint_tick != plan.world_tick
        || checkpoint.capacity_class_id != BrainCapacityClass::N2048_ID
        || checkpoint.compiler_inputs.genome() != plan.compiler_inputs.genome()
        || checkpoint.compiler_inputs.development() != plan.compiler_inputs.development()
        || provenance.birth_foundation_digest != birth.digest()
        || checkpoint.legacy_nano512_compatibility_receipt.is_some()
    {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    let verified = PhenotypeCompiler::compile_validated(
        checkpoint.compiler_inputs,
        &BrainCapacityClass::n2048(),
    )?;
    if verified != *checkpoint.phenotype || verified.phenotype_hash() != checkpoint.phenotype_hash {
        return Err(ScaffoldContractError::PhenotypeCompile);
    }
    alife_core::validate_weight_only_policy_successor(&plan.phenotype, &verified)?;
    plan.phenotype = verified;
    plan.compiler_inputs = checkpoint.compiler_inputs.clone();
    Ok(())
}

impl GpuLiveBrainRuntime {
    pub(super) fn private_training_sampling_for_checkpoint(
        &self,
        organism_id: OrganismId,
    ) -> Result<Option<Vec<u8>>, GameAppShellError> {
        #[cfg(feature = "foundation-training")]
        if let Some(sampling) = self.training_sampling {
            sampling.validate()?;
            return Ok(Some(serde_json::to_vec(&sampling)?));
        }
        Ok(self
            .training_sampling_resume
            .get(&organism_id.raw())
            .cloned())
    }

    pub(super) fn restore_private_training_sampling(
        &mut self,
        organism_id: OrganismId,
        bytes: &Option<Vec<u8>>,
    ) -> Result<(), GameAppShellError> {
        let Some(bytes) = bytes else {
            return Ok(());
        };
        if bytes.len() > 4096
            || self
                .training_sampling_resume
                .get(&organism_id.raw())
                .is_some_and(|previous| previous != bytes)
        {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        #[cfg(feature = "foundation-training")]
        if self.training_sampling.is_some() {
            if self
                .training_sampling_resume
                .values()
                .any(|previous| previous != bytes)
            {
                return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
            }
            let sampling = decode_sampling(bytes)?;
            self.training_sampling = Some(sampling);
        }
        self.training_sampling_resume
            .insert(organism_id.raw(), bytes.clone());
        Ok(())
    }

    #[cfg(feature = "foundation-training")]
    pub fn foundation_training_sampling_state(
        &self,
    ) -> Option<alife_gpu_backend::GpuTrainingSamplingConfig> {
        self.training_sampling
    }

    #[cfg(feature = "foundation-training")]
    pub fn foundation_training_sampling_was_restored(&self, organism_id: OrganismId) -> bool {
        self.training_sampling_resume
            .contains_key(&organism_id.raw())
    }

    #[cfg(feature = "foundation-training")]
    pub fn live_foundation_compiler_inputs(
        &self,
        organism_id: OrganismId,
    ) -> Result<&PhenotypeCompilerInputs, ScaffoldContractError> {
        self.residents
            .get(&organism_id.raw())
            .map(|resident| &resident.compiler_inputs)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)
    }

    #[cfg(feature = "foundation-training")]
    pub fn ensure_live_foundation_policy_boundary(
        &self,
        organism_id: OrganismId,
    ) -> Result<(), ScaffoldContractError> {
        self.backend.ensure_neural_actions_available()?;
        let alive = self
            .world
            .organism_registry()
            .get(organism_id)
            .is_some_and(|record| record.lifecycle().is_alive());
        if !alive
            || self.training_sampling.is_none()
            || !self.last_foundation_training_steps.is_empty()
            || !self.retained_learning.is_empty()
            || self.sleep_journal_publication_worker.is_some()
            || !self.pending_sleep_journal_entries.is_empty()
            || !self.pending_exact_sleep_journal_entries.is_empty()
            || !matches!(
                self.exact_checkpoint_work,
                ExactPopulationCheckpointRuntimeWorkV1::Idle
            )
        {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let handle = *self
            .handles
            .get(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if self.backend.pending_eligibility(handle)?.is_some() {
            return Err(ScaffoldContractError::InvalidDecisionEvidence);
        }
        Ok(())
    }

    #[cfg(feature = "foundation-training")]
    pub fn install_live_foundation_policy(
        &mut self,
        organism_id: OrganismId,
        transition: &alife_core::N2048LivePolicyTransition,
    ) -> Result<alife_gpu_backend::GpuLivePolicyInstallReceipt, GameAppShellError> {
        self.ensure_live_foundation_policy_boundary(organism_id)?;
        let handle = *self
            .handles
            .get(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let resident = self
            .residents
            .get(&organism_id.raw())
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        if resident.phenotype != *transition.previous_phenotype()
            || resident.compiler_inputs.genome() != transition.next_inputs().genome()
            || resident.compiler_inputs.development() != transition.next_inputs().development()
        {
            return Err(ScaffoldContractError::PhenotypeCompile.into());
        }
        let receptors = ResidentNeuralReceptors::compile(transition.next_phenotype())?;
        let sampling = self.training_sampling;
        let result = self
            .backend
            .install_live_foundation_policy(handle, transition);
        if let Err(error) = &result {
            self.backend.record_contract_failure(error);
        }
        let receipt = result?;
        let resident = self
            .residents
            .get_mut(&organism_id.raw())
            .expect("prevalidated resident");
        resident.phenotype = transition.next_phenotype().clone();
        resident.compiler_inputs = transition.next_inputs().clone();
        resident.neural_receptors = receptors;
        self.handles.insert(organism_id.raw(), receipt.handle);
        if self
            .sleep_journal_neural_authorities
            .contains_key(&organism_id.raw())
        {
            let authority =
                match capture_sleep_journal_neural_authority(&mut self.backend, receipt.handle) {
                    Ok(authority) => authority,
                    Err(error) => {
                        self.backend
                            .fail_stop(GpuSessionFailStopCause::BackendUnavailable);
                        return Err(error.into());
                    }
                };
            self.sleep_journal_neural_authorities
                .insert(organism_id.raw(), authority);
        }
        // Sampling seed/counter/temperature and every living sidecar remain as-is.
        debug_assert_eq!(self.training_sampling, sampling);
        Ok(receipt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn transition_fixture() -> (ResidentAuthorityPlan, alife_core::N2048LivePolicyTransition) {
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(0x2048_0F03, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        let original = PhenotypeCompiler::compile_testing_procedural_baseline(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&original).unwrap();
        let mut config = alife_world::CanonicalNewGameConfig::phase3(0x2048_0F03, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
        let game =
            alife_world::create_canonical_new_game_with_n2048_candidate(&config, &asset).unwrap();
        let record = game.world.organism_registry().iter().next().unwrap();
        let plan = resident_authority_plan_from_record(
            record,
            record.organism_id(),
            record.world_entity_id(),
            game.world.tick(),
            BrainScaleTier::Standard2048,
            config.sensor_profile,
        )
        .unwrap();
        let mut weights = plan
            .phenotype
            .synapses()
            .iter()
            .map(|s| s.genetic_weight())
            .collect::<Vec<_>>();
        weights[0] += 0.001;
        let trained = FoundationWeightAsset::from_trained_weights(
            &plan.phenotype,
            weights,
            alife_core::TrainingStageManifest::new(1, 1, 1),
        )
        .unwrap();
        let transition =
            alife_core::N2048LivePolicyTransition::prepare(&plan.compiler_inputs, 7, trained)
                .unwrap();
        (plan, transition)
    }

    #[test]
    fn exact_live_overlay_restore_preserves_biological_authority_and_birth_genome() {
        let (mut plan, transition) = transition_fixture();
        let genome = plan.genome.clone();
        let development = plan.development.clone();
        let biology = plan.biochemistry;
        let checkpoint = ResidentCheckpointMetadata {
            organism_id: plan.organism_id,
            checkpoint_tick: plan.world_tick,
            capacity_class_id: BrainCapacityClass::N2048_ID,
            phenotype_hash: transition.next_phenotype().phenotype_hash(),
            phenotype: transition.next_phenotype(),
            compiler_inputs: transition.next_inputs(),
            legacy_nano512_compatibility_receipt: None,
        };
        adopt_exact_live_policy_checkpoint(&mut plan, checkpoint).unwrap();
        assert_eq!(plan.genome, genome);
        assert_eq!(plan.development, development);
        assert_eq!(plan.biochemistry, biology);
        assert_eq!(plan.phenotype, *transition.next_phenotype());
        assert_eq!(plan.compiler_inputs, *transition.next_inputs());
    }

    #[test]
    fn exact_overlay_restore_rejects_wrong_birth_binding_without_changing_plan() {
        let (mut plan, transition) = transition_fixture();
        let mut provenance = transition.next_inputs().live_policy().unwrap();
        provenance.birth_foundation_digest = Blake3Digest::from_bytes([9; 32]);
        let forged = PhenotypeCompilerInputs::try_new_with_live_n2048_policy(
            transition.next_inputs().genome().clone(),
            &BrainCapacityClass::n2048(),
            transition.next_inputs().development().clone(),
            transition.next_inputs().sensor_profile(),
            transition
                .next_inputs()
                .n2048_candidate_asset()
                .unwrap()
                .clone(),
            provenance,
        )
        .unwrap();
        let phenotype =
            PhenotypeCompiler::compile_validated(&forged, &BrainCapacityClass::n2048()).unwrap();
        let original = plan.compiler_inputs.clone();
        let checkpoint = ResidentCheckpointMetadata {
            organism_id: plan.organism_id,
            checkpoint_tick: plan.world_tick,
            capacity_class_id: BrainCapacityClass::N2048_ID,
            phenotype_hash: phenotype.phenotype_hash(),
            phenotype: &phenotype,
            compiler_inputs: &forged,
            legacy_nano512_compatibility_receipt: None,
        };
        assert!(adopt_exact_live_policy_checkpoint(&mut plan, checkpoint).is_err());
        assert_eq!(plan.compiler_inputs, original);
    }

    #[cfg(feature = "foundation-training")]
    #[test]
    fn sampling_codec_preserves_counter_seed_temperature_and_rejects_unknown_or_forced_state() {
        let sampling = alife_gpu_backend::GpuTrainingSamplingConfig {
            seed: 0xABCD,
            counter: 123456,
            temperature: 0.75,
            demonstrator: None,
        };
        let bytes = serde_json::to_vec(&sampling).unwrap();
        assert_eq!(decode_sampling(&bytes).unwrap(), sampling);
        let mut forged = serde_json::to_value(sampling).unwrap();
        forged["unknown"] = serde_json::json!(1);
        assert!(decode_sampling(&serde_json::to_vec(&forged).unwrap()).is_err());
        assert!(decode_sampling(&vec![b' '; 4097]).is_err());
        let forced = alife_gpu_backend::GpuTrainingSamplingConfig {
            demonstrator: Some(alife_gpu_backend::GpuTrainingDemonstratorAction {
                representative_index: 0,
                motor_indices: [u16::MAX; 6],
            }),
            ..sampling
        };
        assert!(decode_sampling(&serde_json::to_vec(&forced).unwrap()).is_err());
    }

    #[cfg(all(feature = "foundation-training", feature = "gpu-tests"))]
    #[test]
    #[ignore = "explicit hardware admission required; disposable one-body N2048 runtime only"]
    fn installed_policy_keeps_world_and_rng_then_applies_the_next_sealed_outcome() {
        let (plan, _) = transition_fixture();
        let seed = 0x2048_0F03;
        let mut config = alife_world::CanonicalNewGameConfig::phase3(seed, 1).unwrap();
        config.brain_class = BrainScaleTier::Standard2048;
        config.sensor_profile = SensorProfile::GroundedObjectSlotsV1;
        let mut game = alife_world::create_canonical_new_game_with_n2048_candidate(
            &config,
            plan.compiler_inputs.n2048_candidate_asset().unwrap(),
        )
        .unwrap();
        crate::foundation_training::ground_foundation_training_world(&mut game.world).unwrap();
        let root = std::env::temp_dir().join(format!(
            "alife-live-policy-runtime-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let asset_root = root.join("world-assets");
        std::fs::create_dir(&asset_root).unwrap();
        let backend = GpuClosedLoopBackend::new_required(
            alife_gpu_backend::GpuRuntimeProfile::production_v1(),
        )
        .unwrap();
        let sampling = alife_gpu_backend::GpuTrainingSamplingConfig {
            seed: 0xAABB,
            counter: 0,
            temperature: 0.75,
            demonstrator: None,
        };
        let mut runtime = GpuLiveBrainRuntime::new_profiled_foundation_training(
            backend,
            game.world,
            seed,
            BrainScaleTier::Standard2048,
            config.sensor_profile,
            LineageLibraryConfig::profile_default(root.join("lineage")),
            "live-policy-runtime-fixture".into(),
            ArchiveLearnedCapturePolicy::GeneticOnly,
            sampling,
        )
        .unwrap();
        let mut runtime_config =
            alife_world::RuntimeConfig::deterministic_default(seed, BrainScaleTier::Standard2048);
        runtime_config.features.gpu_backend_enabled = true;
        let base = PortableSaveFile::from_headless_world(
            "live-policy-runtime-fixture".into(),
            runtime.world(),
            runtime_config,
            AssetManifest::empty(),
            game.creatures,
        )
        .unwrap();
        let save_path = root.join("world.json");
        runtime
            .attach_durable_checkpoint_boundary(&save_path, &asset_root, base)
            .unwrap();
        let base = runtime.capture_portable_checkpoint().unwrap();
        GpuDurableSaveManifest::publish_snapshot(&save_path, &asset_root, &base).unwrap();
        runtime
            .rebind_durable_checkpoint_boundary(&save_path, &asset_root, &base)
            .unwrap();
        runtime.tick().unwrap();
        let before_steps = runtime.take_foundation_training_steps();
        assert_eq!(before_steps.len(), 1);
        assert!(!runtime.last_learning_receipts.is_empty());
        let before_sequence = before_steps[0].patch.pre_action().sequence_id;
        let organism = plan.organism_id;
        let inputs = runtime
            .live_foundation_compiler_inputs(organism)
            .unwrap()
            .clone();
        let phenotype =
            PhenotypeCompiler::compile_validated(&inputs, &BrainCapacityClass::n2048()).unwrap();
        let mut weights = phenotype
            .synapses()
            .iter()
            .map(|s| s.genetic_weight())
            .collect::<Vec<_>>();
        weights[0] += 0.001;
        let asset = FoundationWeightAsset::from_trained_weights(
            &phenotype,
            weights,
            alife_core::TrainingStageManifest::new(1, 1, 1),
        )
        .unwrap();
        let transition = alife_core::N2048LivePolicyTransition::prepare(&inputs, 7, asset).unwrap();
        let world = runtime.world().canonical_signature_digest().unwrap();
        let sampling = runtime.foundation_training_sampling_state().unwrap();
        let old_handle = runtime.handles[&organism.raw()];
        let authority =
            capture_sleep_journal_neural_authority(&mut runtime.backend, old_handle).unwrap();
        runtime
            .sleep_journal_neural_authorities
            .insert(organism.raw(), authority);
        let installed = runtime
            .install_live_foundation_policy(organism, &transition)
            .unwrap();
        assert!(installed.submission_completed);
        assert_eq!(runtime.world().canonical_signature_digest().unwrap(), world);
        assert_eq!(
            runtime.foundation_training_sampling_state().unwrap(),
            sampling
        );
        validate_sleep_journal_neural_authority(
            &mut runtime.backend,
            installed.handle,
            &runtime.sleep_journal_neural_authorities[&organism.raw()],
        )
        .unwrap();
        runtime.tick().unwrap();
        let next = runtime.take_foundation_training_steps();
        assert_eq!(next.len(), 1);
        assert!(next[0].patch.pre_action().sequence_id.raw() > before_sequence.raw());
        assert_eq!(
            next[0].before.handle.phenotype_hash(),
            transition.next_phenotype().phenotype_hash()
        );
        assert!(
            !runtime.last_learning_receipts.is_empty(),
            "next sealed outcome must apply under rebound guard"
        );
        assert!(
            runtime
                .foundation_training_sampling_state()
                .unwrap()
                .counter
                > sampling.counter
        );
        let exact = runtime.capture_portable_checkpoint().unwrap();
        GpuDurableSaveManifest::publish_snapshot(&save_path, &asset_root, &exact).unwrap();
        runtime
            .rebind_durable_checkpoint_boundary(&save_path, &asset_root, &exact)
            .unwrap();
        // Failure deliberately retains this disposable directory for diagnosis.
        std::fs::remove_dir_all(root).unwrap();
    }
}
