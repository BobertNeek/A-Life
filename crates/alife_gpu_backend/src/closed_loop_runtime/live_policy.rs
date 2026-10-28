//! Checked live learned-head installation. Recurrent/fast/lifetime/eligibility,
//! homeostasis, structural and sleep state are never reinitialized or uploaded.
use super::*;
use alife_core::N2048LivePolicyTransition;

#[derive(Debug, Clone, PartialEq)]
pub struct GpuLivePolicyInstallReceipt {
    pub previous_handle: GpuBrainHandle,
    pub handle: GpuBrainHandle,
    pub previous_policy: Option<u64>,
    pub policy: u64,
    pub mutable_words_written: u64,
    pub submission_completed: bool,
}

impl GpuClosedLoopBackend {
    pub fn install_live_foundation_policy(
        &mut self,
        handle: GpuBrainHandle,
        transition: &N2048LivePolicyTransition,
    ) -> Result<GpuLivePolicyInstallReceipt, ScaffoldContractError> {
        self.ensure_ready()?;
        self.validate_handle_backend(handle)?;
        if self.sleep_jobs.values().any(|job| job.handle == handle) {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        // Validate the current portable activity authority (including retained
        // sequence/ATP history) before changing any immutable GPU identity.
        self.snapshot_activity_state(handle)?;
        let class = handle.class_id.raw();
        let pool = self
            .class_buckets
            .get(&class)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
        let bucket = pool.bucket_for_handle(handle)?;
        let slot = pool.resident(handle)?;
        if slot.phenotype != *transition.previous_phenotype()
            || slot.pending_eligibility.is_some()
            || slot.pending_eligibility_record.is_some()
            || slot.v11.pending_lifetime_synapse().is_some()
        {
            return Err(ScaffoldContractError::BrainOwnershipMismatch);
        }
        let mut rebound_guard = slot.learning_sequence_guard.clone();
        rebound_guard.rebind_phenotype(
            handle.phenotype_hash,
            transition.next_phenotype().phenotype_hash(),
        )?;
        let rebound_activity = slot
            .last_activity
            .as_ref()
            .map(|activity| {
                let capacity = capacity_for_gpu_class(handle.class_id)?;
                let (throttle, work) = activity.throttle.rebind_weight_only_policy_authority(
                    &activity.work,
                    &self.activity_policy,
                    &slot.phenotype,
                    transition.next_phenotype(),
                    capacity.execution(),
                )?;
                throttle.validate_runtime_binding(handle.slot, handle.generation)?;
                work.validate_runtime_binding(handle.slot, handle.generation)?;
                Ok::<_, ScaffoldContractError>(ResidentActivity { throttle, work })
            })
            .transpose()?;
        // Reconstruct the SAME live structural additions on both immutable
        // plans. Target-major insertions can shift baseline weight coordinates.
        let before =
            compile_v11_slot_upload(&bucket.plan, &slot.brain_slot, &slot.phenotype, &slot.v11)
                .map_err(map_gpu_contract_error)?;
        let after = compile_v11_slot_upload(
            &bucket.plan,
            &slot.brain_slot,
            transition.next_phenotype(),
            &slot.v11,
        )
        .map_err(map_gpu_contract_error)?;
        if before.brain_slot() != &slot.brain_slot {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch);
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("same-individual-learned-policy-install"),
            });
        let mut staging = Vec::new();
        bucket
            .buffers
            .record_weight_only_policy_upload(
                &self.device,
                &mut encoder,
                &before,
                &after,
                &mut staging,
            )
            .map_err(map_gpu_contract_error)?;
        let submission = self.queue.submit(Some(encoder.finish()));
        if self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err()
            || self.device_lost.load(Ordering::Acquire)
        {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        // All fallible preparation precedes submission; cut over the exact
        // identity only after confirmed completion. Old handles become stale.
        let new_handle = GpuBrainHandle {
            phenotype_hash: transition.next_phenotype().phenotype_hash(),
            ..handle
        };
        let pool = self
            .class_buckets
            .get_mut(&class)
            .expect("prevalidated class pool");
        let slot = pool.resident_mut(handle).expect("prevalidated resident");
        slot.ownership.phenotype_hash = new_handle.phenotype_hash;
        slot.phenotype = transition.next_phenotype().clone();
        slot.brain_slot = after.brain_slot().clone();
        slot.training_phenotype = None;
        slot.learning_sequence_guard = rebound_guard;
        slot.last_activity = rebound_activity;
        self.organisms.insert(handle.organism_id.raw(), new_handle);
        self.training_structural_cache.retain(|key, _| {
            !(key.0 == class && key.1 == handle.slot && key.2 == handle.generation)
        });
        Ok(GpuLivePolicyInstallReceipt {
            previous_handle: handle,
            handle: new_handle,
            previous_policy: transition.previous_policy(),
            policy: transition.next_policy(),
            mutable_words_written: 0,
            submission_completed: true,
        })
    }
}

#[cfg(all(test, feature = "gpu-tests"))]
mod tests {
    use super::*;
    use alife_core::{
        BrainGenome, DevelopmentState, FoundationWeightAsset, NormalizedScalar, PhenotypeCompiler,
        SensorProfile, Tick, TrainingStageManifest,
    };

    #[test]
    #[ignore = "requires an explicitly admitted hardware GPU; disposable N2048 fixture only"]
    fn in_place_policy_install_preserves_all_nonzero_live_mutable_words_and_cursors() {
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(0x2048_0F02, capacity.id());
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
        let (original, inputs) =
            PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset)
                .unwrap();
        let mut weights = original
            .synapses()
            .iter()
            .map(|s| s.genetic_weight())
            .collect::<Vec<_>>();
        weights[0] += 0.001;
        let trained = FoundationWeightAsset::from_trained_weights(
            &original,
            weights,
            TrainingStageManifest::new(1, 1, 1),
        )
        .unwrap();
        let transition = N2048LivePolicyTransition::prepare(&inputs, 5, trained).unwrap();
        let mut backend =
            GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
        let handle = backend
            .insert_brain(OrganismId(7101), original.clone())
            .unwrap();
        let other = backend.insert_brain(OrganismId(7102), original).unwrap();
        let other_before = backend
            .capture_training_state(other, Tick::new(11))
            .unwrap();
        let genetic_before = backend
            .read_immutable_genetic_weights_for_test(handle)
            .unwrap();
        let other_genetic_before = backend
            .read_immutable_genetic_weights_for_test(other)
            .unwrap();
        let pool = &backend.class_buckets[&handle.class_id.raw()];
        let slot = pool.resident(handle).unwrap();
        let ranges = slot.ranges.clone();
        let bucket = pool.bucket_for_handle(handle).unwrap();
        let mutable = bucket.buffers.neural_buffers()[6];
        for offset in [
            ranges.layout.activation_a_words.start,
            ranges.layout.activation_b_words.start,
            ranges.layout.fast_weight_words.start,
            ranges.layout.fast_weight_bank_1_words.start,
            ranges.layout.lifetime_weight_words.start,
            ranges.layout.lifetime_weight_bank_1_words.start,
            ranges.layout.recurrent_eligibility_words.start,
            ranges.layout.recurrent_eligibility_bank_1_words.start,
            ranges.layout.homeostasis_words.start,
        ] {
            backend.queue.write_buffer(
                mutable,
                u64::from(offset) * 4,
                bytemuck::cast_slice(&[0.125_f32]),
            );
        }
        let replay = backend.build_sleep_replay_batch(handle).unwrap();
        let request = backend
            .prepare_sleep_consolidation(
                handle,
                alife_core::ConsolidationIntent { cycle_id: 41 },
                &replay,
            )
            .unwrap();
        let job = backend
            .submit_sleep_consolidation(handle, &request, &replay)
            .unwrap();
        let staged = backend
            .poll_sleep_consolidation(handle, job)
            .unwrap()
            .unwrap();
        let committed = backend
            .commit_sleep_consolidation(handle, &request, &staged.staged)
            .unwrap();
        assert_eq!(
            backend
                .commit_sleep_consolidation(handle, &request, &staged.staged)
                .unwrap(),
            committed
        );
        let before = backend
            .capture_training_state(handle, Tick::new(11))
            .unwrap();
        let installed = backend
            .install_live_foundation_policy(handle, &transition)
            .unwrap();
        let after = backend
            .capture_training_state(installed.handle, Tick::new(11))
            .unwrap();
        assert!(installed.submission_completed);
        assert_eq!(installed.mutable_words_written, 0);
        assert_eq!(before.mutable_words, after.mutable_words);
        assert_eq!(before.v11, after.v11);
        assert_eq!(before.active_activation_side, after.active_activation_side);
        assert_eq!(before.active_weight_bank, after.active_weight_bank);
        assert_eq!(
            before.active_weight_generation,
            after.active_weight_generation
        );
        assert_eq!(
            before.logical_dispatch_generation,
            after.logical_dispatch_generation
        );
        assert!(backend.pending_eligibility(handle).is_err());
        assert!(backend
            .pending_eligibility(installed.handle)
            .unwrap()
            .is_none());
        let genetic_after = backend
            .read_immutable_genetic_weights_for_test(installed.handle)
            .unwrap();
        assert_ne!(genetic_before, genetic_after);
        let upload = crate::GpuPhenotypeUpload::try_from(transition.next_phenotype()).unwrap();
        assert_eq!(
            genetic_after[upload.canonical_to_local_synapse[0] as usize].to_bits(),
            transition.next_phenotype().synapses()[0]
                .genetic_weight()
                .to_bits()
        );
        let other_after = backend
            .capture_training_state(other, Tick::new(11))
            .unwrap();
        assert_eq!(other_before.mutable_words, other_after.mutable_words);
        assert_eq!(
            other_genetic_before,
            backend
                .read_immutable_genetic_weights_for_test(other)
                .unwrap()
        );
        assert!(backend
            .commit_sleep_consolidation(handle, &request, &staged.staged)
            .is_err());
        assert!(backend
            .commit_sleep_consolidation(installed.handle, &request, &staged.staged)
            .is_err());
        assert!(backend
            .committed_sleep
            .contains_key(&crate::closed_loop_sleep::sleep_commit_key(
                installed.handle,
                41
            )));
    }
}
