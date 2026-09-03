//! Durable checkpoint attachment, capture, journal publication, and shutdown coordination.

use super::*;

impl GpuLiveBrainRuntime {
    pub fn attach_durable_checkpoint_boundary(
        &mut self,
        save_path: impl AsRef<Path>,
        asset_root: impl AsRef<Path>,
        mut base: PortableSaveFile,
    ) -> Result<(), GameAppShellError> {
        if self.checkpoint_durability.is_some() {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU runtime already has a durable save boundary".to_string(),
            });
        }
        validate_replacement_policy(
            base.config.brain_policy.policy,
            base.deterministic_seed,
            base.config.brain_class,
            self.deterministic_seed,
            self.brain_class,
        )?;
        let base_world = base.restore_headless_world()?;
        if base.deterministic_seed != self.deterministic_seed
            || base.config.deterministic_seed != self.deterministic_seed
            || base_world.seed() != self.world.seed()
            || base_world.tick() != self.world.tick()
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message:
                    "durable checkpoint base seed or tick does not match the canonical live world"
                        .to_string(),
            });
        }
        let live_ids = self.handles.keys().copied().collect::<BTreeSet<_>>();
        let saved_ids = base
            .creatures
            .iter()
            .map(|creature| creature.organism_id.raw())
            .collect::<BTreeSet<_>>();
        if saved_ids != live_ids || saved_ids.len() != base.creatures.len() {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "durable checkpoint base does not cover the live GPU residents"
                    .to_string(),
            });
        }
        if base
            .creatures
            .iter()
            .any(|creature| creature.brain_class != self.brain_class)
        {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "durable checkpoint base contains an incompatible brain class".to_string(),
            });
        }
        // The full canonical signature also binds runtime-only tracked-object
        // state. PortableSaveFile normalizes that state through WorldSaveState,
        // so compare the supplied durable representation with the exact
        // normalized representation expected from the live world. This keeps
        // persisted organisms, archive identity, objects, ecology, habitats,
        // and counters strict without rejecting a valid save for transient
        // state that the save authority does not persist.
        let mut normalized_base = base.clone();
        normalized_base.replace_headless_world_snapshot(&self.world)?;
        if normalized_base.world != base.world {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "durable checkpoint base does not match the canonical live world"
                    .to_string(),
            });
        }
        base.replace_headless_world_snapshot(&self.world)?;

        let save_path = save_path.as_ref();
        let asset_root = asset_root.as_ref();
        GpuDurableSaveManifest::publish_snapshot(save_path, asset_root, &base)?;
        let (durable_manifest, published) =
            GpuDurableSaveManifest::open_loaded(save_path, asset_root)?;
        let store = GpuCheckpointAssetStore::new(durable_manifest.asset_root().to_path_buf())?;
        let canonical_save_id = published.save.save_id.clone();
        let durability = GpuLiveCheckpointDurability {
            store,
            durable_manifest,
            published,
        };
        let durable_reference = durability.durable_reference()?;
        self.backend.note_durable_checkpoint(durable_reference)?;
        self.canonical_save_id = Some(canonical_save_id);
        self.checkpoint_durability = Some(durability);
        Ok(())
    }

    /// Captures one exact, sealed-boundary portable save without publishing it.
    /// The caller may atomically publish the returned manifest as a manual save;
    /// all bulk neural state remains behind content-addressed asset references.
    pub fn capture_portable_checkpoint(&mut self) -> Result<PortableSaveFile, GameAppShellError> {
        self.flush_sleep_journal_publication_blocking()?;
        let started = Instant::now();
        let readback_before = self.backend.mutable_slot_readback_metrics();
        let Some(durability) = self.checkpoint_durability.take() else {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "GPU runtime has no durable save boundary".to_string(),
            });
        };
        let base = durability.published.save.clone();
        let store = durability.store.clone();
        let result = self
            .capture_checkpointed_save(base, &store)
            .map(|(save, _)| save);
        self.checkpoint_durability = Some(durability);
        let readback_after = self.backend.mutable_slot_readback_metrics();
        self.performance_metrics.checkpoint_capture_calls = self
            .performance_metrics
            .checkpoint_capture_calls
            .saturating_add(1);
        self.performance_metrics.checkpoint_capture_wall_ns = self
            .performance_metrics
            .checkpoint_capture_wall_ns
            .saturating_add(u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        self.performance_metrics.checkpoint_snapshot_calls = self
            .performance_metrics
            .checkpoint_snapshot_calls
            .saturating_add(readback_after.calls.saturating_sub(readback_before.calls));
        self.performance_metrics.checkpoint_snapshot_bytes = self
            .performance_metrics
            .checkpoint_snapshot_bytes
            .saturating_add(readback_after.bytes.saturating_sub(readback_before.bytes));
        self.performance_metrics.checkpoint_snapshot_poll_wait_ns = self
            .performance_metrics
            .checkpoint_snapshot_poll_wait_ns
            .saturating_add(
                readback_after
                    .poll_wait_ns
                    .saturating_sub(readback_before.poll_wait_ns),
            );
        self.performance_metrics
            .checkpoint_snapshot_map_receive_wait_ns = self
            .performance_metrics
            .checkpoint_snapshot_map_receive_wait_ns
            .saturating_add(
                readback_after
                    .map_receive_wait_ns
                    .saturating_sub(readback_before.map_receive_wait_ns),
            );
        result
    }

    pub(crate) fn rebind_durable_checkpoint_boundary(
        &mut self,
        save_path: impl AsRef<Path>,
        asset_root: impl AsRef<Path>,
        expected: &PortableSaveFile,
    ) -> Result<(), GameAppShellError> {
        let (durable_manifest, published) =
            GpuDurableSaveManifest::open_loaded(save_path.as_ref(), asset_root.as_ref())?;
        if published.save != *expected {
            return Err(GameAppShellError::InvalidProductionFrontend {
                message: "rebound GPU checkpoint boundary differs from the exact save".to_string(),
            });
        }
        let store = GpuCheckpointAssetStore::new(durable_manifest.asset_root().to_path_buf())?;
        let candidate = GpuLiveCheckpointDurability {
            store,
            durable_manifest,
            published,
        };
        let durable_reference = candidate.durable_reference()?;
        self.backend.note_durable_checkpoint(durable_reference)?;
        self.canonical_save_id = Some(candidate.published.save.save_id.clone());
        self.checkpoint_durability = Some(candidate);
        Ok(())
    }

    pub(super) fn capture_checkpointed_save(
        &mut self,
        mut replacement: PortableSaveFile,
        store: &GpuCheckpointAssetStore,
    ) -> Result<(PortableSaveFile, u64), GameAppShellError> {
        let checkpoint_tick = self.world.tick();
        self.add_missing_checkpoint_creature_summaries(&mut replacement)?;
        replacement.replace_headless_world_snapshot(&self.world)?;
        let mut manifest_entries = Vec::new();
        let mut exact_neural_captures = 0_u64;
        for (&raw, &handle) in &self.handles {
            let organism_id = OrganismId(raw);
            let record = self
                .world
                .organism_registry()
                .get(organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let authoritative_age = record.age_at(checkpoint_tick)?;
            let resident = self
                .residents
                .get(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if resident.homeostasis != record.biochemistry().homeostasis
                || resident.homeostasis.tick != checkpoint_tick
                || resident.development.age_ticks != authoritative_age
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            let exact = self.exact_cognitive_state_for_checkpoint(
                organism_id,
                handle,
                resident,
                checkpoint_tick,
            )?;
            let replay_patches = replay_patches_for_checkpoint(
                &mut self.backend,
                handle,
                organism_id,
                &self.restored_replay_patches,
                &self.sealed_patches,
                &self.last_sealed_patches,
            )?;
            let mut write = store.capture_brain_with_runtime_replay_state(
                &mut self.backend,
                handle,
                &resident.phenotype,
                &resident.compiler_inputs,
                resident.sleep_scheduler.state(),
                checkpoint_tick,
                None,
                &replay_patches,
                GpuBrainSidecarCapture {
                    sensor_profile: self
                        .memories
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                        .profile(),
                    memory: self
                        .memories
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?,
                    topology: self
                        .topologies
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?,
                    tracked_objects: self.world.tracked_objects().save_state(organism_id)?,
                    language_grounding: &resident.language_grounding,
                    life_statistics: &resident.life_statistics,
                    legacy_nano512_compatibility_receipt: resident
                        .legacy_nano512_compatibility_receipt
                        .as_ref(),
                    retained_learning: self.retained_learning.get(&raw).map(|recovery| {
                        RetainedLearningCapture {
                            sealed_patch: &recovery.sealed_patch,
                            neural_receptors: &recovery.neural_receptors,
                            attempts: recovery.attempts,
                            last_error_code: recovery.last_error.slug(),
                        }
                    }),
                },
            )?;
            write.attach_exact_cognitive_state(store, &exact)?;
            exact_neural_captures = exact_neural_captures.saturating_add(1);
            manifest_entries.extend(write.manifest_entries);
            let canonical_biochemistry = self
                .world
                .organism_registry()
                .get(organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                .biochemistry()
                .clone();
            let creature = replacement
                .creatures
                .iter_mut()
                .find(|creature| creature.organism_id.raw() == raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if creature.brain_class != self.brain_class {
                return Err(ScaffoldContractError::PhenotypeCompile.into());
            }
            creature.development_tick = canonical_biochemistry.development.last_update_tick;
            creature.mind.tick = canonical_biochemistry.tick;
            creature.mind.homeostasis = canonical_biochemistry.homeostasis;
            creature.mind.sleep_state_label =
                gpu_sleep_state_label(resident.sleep_scheduler.state());
            creature.gpu_brain = Some(write.save_state);
        }
        if replacement.creatures.len() != self.handles.len() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        merge_gpu_checkpoint_manifest_entries(&mut replacement.assets, manifest_entries)?;
        replacement.validate_with_asset_root(store.root())?;
        Ok((replacement, exact_neural_captures))
    }

    pub(super) fn freeze_exact_population_host_snapshot(
        &self,
        mut replacement: PortableSaveFile,
    ) -> Result<ExactPopulationHostSnapshotV1, GameAppShellError> {
        let checkpoint_tick = self.world.tick();
        self.add_missing_checkpoint_creature_summaries(&mut replacement)?;
        replacement.replace_headless_world_snapshot(&self.world)?;
        let mut brains = Vec::with_capacity(self.handles.len());
        for (&raw, &handle) in &self.handles {
            let organism_id = OrganismId(raw);
            let record = self
                .world
                .organism_registry()
                .get(organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = self
                .residents
                .get(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if resident.homeostasis != record.biochemistry().homeostasis
                || resident.homeostasis.tick != checkpoint_tick
                || resident.development.age_ticks != record.age_at(checkpoint_tick)?
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            let canonical_biochemistry = record.biochemistry().clone();
            let creature = replacement
                .creatures
                .iter_mut()
                .find(|creature| creature.organism_id == organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if creature.brain_class != self.brain_class {
                return Err(ScaffoldContractError::PhenotypeCompile.into());
            }
            creature.development_tick = canonical_biochemistry.development.last_update_tick;
            creature.mind.tick = canonical_biochemistry.tick;
            creature.mind.homeostasis = canonical_biochemistry.homeostasis;
            creature.mind.sleep_state_label =
                gpu_sleep_state_label(resident.sleep_scheduler.state());
            brains.push(ExactBrainHostSnapshotV1 {
                handle,
                phenotype: resident.phenotype.clone(),
                compiler_inputs: resident.compiler_inputs.clone(),
                sleep: resident.sleep_scheduler.state(),
                memory: self
                    .memories
                    .get(&raw)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                    .clone(),
                topology: self
                    .topologies
                    .get(&raw)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                    .clone(),
                tracked_objects: self.world.tracked_objects().save_state(organism_id)?,
                language_grounding: resident.language_grounding.clone(),
                life_statistics: resident.life_statistics.clone(),
                legacy_nano512_compatibility_receipt: resident
                    .legacy_nano512_compatibility_receipt
                    .clone(),
                retained_learning: self.retained_learning.get(&raw).map(|recovery| {
                    ExactRetainedLearningHostSnapshotV1 {
                        sealed_patch: recovery.sealed_patch.clone(),
                        neural_receptors: recovery.neural_receptors.clone(),
                        attempts: recovery.attempts,
                        last_error_code: recovery.last_error.slug(),
                    }
                }),
                exact_cognitive_state: Self::exact_cognitive_host_snapshot(
                    organism_id,
                    resident,
                    checkpoint_tick,
                )?,
            });
        }
        if replacement.creatures.len() != brains.len() {
            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
        }
        Ok(ExactPopulationHostSnapshotV1 {
            checkpoint_tick,
            replacement,
            brains,
            restored_replay_patches: self.restored_replay_patches.clone(),
            sealed_patches: self.sealed_patches.clone(),
            last_sealed_patches: self.last_sealed_patches.clone(),
        })
    }

    pub(super) fn add_missing_checkpoint_creature_summaries(
        &self,
        replacement: &mut PortableSaveFile,
    ) -> Result<(), GameAppShellError> {
        let live_ids = self.handles.keys().copied().collect::<BTreeSet<_>>();
        for raw in live_ids {
            if replacement
                .creatures
                .iter()
                .any(|creature| creature.organism_id.raw() == raw)
            {
                continue;
            }
            let organism_id = OrganismId(raw);
            let record = self
                .world
                .organism_registry()
                .get(organism_id)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = self
                .residents
                .get(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let summary =
                checkpoint_creature_save_state(replacement, record, resident, self.brain_class)?;
            replacement.creatures.push(summary);
        }
        replacement
            .creatures
            .sort_by_key(|creature| creature.organism_id.raw());
        Ok(())
    }

    pub(super) fn start_sleep_journal_publication(
        &mut self,
        entries: Vec<GpuSleepTransactionJournalEntryV2>,
    ) -> Result<(), GameAppShellError> {
        if entries.is_empty() {
            return Ok(());
        }
        if self.sleep_journal_publication_worker.is_some() || self.checkpoint_durability.is_none() {
            append_bounded_sleep_journal_entries(&mut self.pending_sleep_journal_entries, entries)?;
            self.performance_metrics.sleep_journal_pending_entries_peak = self
                .performance_metrics
                .sleep_journal_pending_entries_peak
                .max(u64::try_from(self.pending_sleep_journal_entries.len()).unwrap_or(u64::MAX));
            return Ok(());
        }
        let durability = self
            .checkpoint_durability
            .as_ref()
            .cloned()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        self.sleep_journal_publication_worker = Some(spawn_sleep_journal_publication_worker(
            durability,
            entries,
            self.performance_measurement_enabled,
        ));
        self.performance_metrics.sleep_journal_worker_starts = self
            .performance_metrics
            .sleep_journal_worker_starts
            .saturating_add(1);
        Ok(())
    }

    pub(super) fn exact_checkpoint_accepts_journal_entries(&self) -> bool {
        matches!(
            self.exact_checkpoint_work,
            ExactPopulationCheckpointRuntimeWorkV1::Capture { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::Worker { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::CommitWorker { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::JournalWorker { .. }
                | ExactPopulationCheckpointRuntimeWorkV1::Finalizing { .. }
        )
    }

    pub(super) fn resume_exact_checkpoint_after_sleep_journal_drain(
        &mut self,
    ) -> Result<(), GameAppShellError> {
        if self.sleep_journal_publication_worker.is_some()
            || !self.pending_sleep_journal_entries.is_empty()
            || !self.exact_checkpoint_waiting_for_sleep_journal
        {
            return Ok(());
        }
        self.exact_checkpoint_waiting_for_sleep_journal = false;
        self.request_exact_population_checkpoint()?;
        if let Some(destination) = self.manual_checkpoint_waiting_for_sleep_journal.take() {
            let _ = self.request_manual_checkpoint(destination)?;
        }
        Ok(())
    }

    pub(super) fn poll_sleep_journal_publication(&mut self) -> Result<(), GameAppShellError> {
        let started = self.performance_measurement_enabled.then(Instant::now);
        self.performance_metrics.sleep_journal_worker_poll_calls = self
            .performance_metrics
            .sleep_journal_worker_poll_calls
            .saturating_add(1);
        let Some(mut worker) = self.sleep_journal_publication_worker.take() else {
            self.resume_exact_checkpoint_after_sleep_journal_drain()?;
            self.performance_metrics.sleep_journal_worker_poll_wall_ns = self
                .performance_metrics
                .sleep_journal_worker_poll_wall_ns
                .saturating_add(started.map_or(0, elapsed_ns));
            return Ok(());
        };
        match worker.poll() {
            SleepJournalPublicationWorkerPollV1::Pending => {
                self.sleep_journal_publication_worker = Some(worker);
            }
            SleepJournalPublicationWorkerPollV1::Panicked => {
                self.performance_metrics.sleep_journal_worker_failures = self
                    .performance_metrics
                    .sleep_journal_worker_failures
                    .saturating_add(1);
                self.backend
                    .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
            }
            SleepJournalPublicationWorkerPollV1::Ready(final_result) => {
                self.admit_sleep_journal_publication(final_result)?;
                if !self.pending_sleep_journal_entries.is_empty() {
                    let pending = std::mem::take(&mut self.pending_sleep_journal_entries);
                    self.start_sleep_journal_publication(pending)?;
                } else {
                    self.resume_exact_checkpoint_after_sleep_journal_drain()?;
                }
            }
        }
        self.performance_metrics.sleep_journal_worker_poll_wall_ns = self
            .performance_metrics
            .sleep_journal_worker_poll_wall_ns
            .saturating_add(started.map_or(0, elapsed_ns));
        Ok(())
    }

    pub(super) fn admit_sleep_journal_publication(
        &mut self,
        final_result: SleepJournalPublicationWorkerFinalV1,
    ) -> Result<(), GameAppShellError> {
        self.performance_metrics.sleep_journal_worker_wall_ns = self
            .performance_metrics
            .sleep_journal_worker_wall_ns
            .saturating_add(final_result.worker_wall_ns);
        let (published, timing) = match final_result.result {
            Ok(result) => result,
            Err(error) => {
                self.performance_metrics.sleep_journal_worker_failures = self
                    .performance_metrics
                    .sleep_journal_worker_failures
                    .saturating_add(1);
                self.backend
                    .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                return Err(error);
            }
        };
        let durability = self
            .checkpoint_durability
            .as_mut()
            .ok_or(ScaffoldContractError::MissingPhaseData)?;
        let expected_published_generation = match final_result.expected_base_generation {
            Some(generation) => generation.checked_add(1),
            None => Some(1),
        };
        if durability.published.digest.as_str() != final_result.expected_base_digest
            || durability.published.authority_generation() != final_result.expected_base_generation
            || published.authority_generation() != expected_published_generation
            || durability.published.save != published.save
            || durability.published.exact_save_anchor_digest()?
                != published.exact_save_anchor_digest()?
        {
            self.backend
                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        durability.published = published;
        self.record_sleep_journal_publication_timing(timing);
        self.performance_metrics.sleep_journal_worker_completions = self
            .performance_metrics
            .sleep_journal_worker_completions
            .saturating_add(1);
        self.performance_metrics.sleep_compact_journal_organisms = self
            .performance_metrics
            .sleep_compact_journal_organisms
            .saturating_add(final_result.entry_count);
        Ok(())
    }

    pub(crate) fn persistence_idle_for_shutdown(&self) -> bool {
        self.sleep_journal_publication_worker.is_none()
            && self.pending_sleep_journal_entries.is_empty()
            && !self.exact_checkpoint_waiting_for_sleep_journal
            && !self.exact_checkpoint_coordinator.is_active()
            && matches!(
                self.exact_checkpoint_work,
                ExactPopulationCheckpointRuntimeWorkV1::Idle
            )
    }

    pub(crate) fn persistence_failed_for_shutdown(&self) -> bool {
        self.exact_checkpoint_coordinator.stage() == ExactPopulationCheckpointStageV1::Failed
            || matches!(
                self.exact_checkpoint_work,
                ExactPopulationCheckpointRuntimeWorkV1::Failed
            )
    }

    pub(crate) fn persistence_terminal_for_shutdown(&self) -> bool {
        self.persistence_idle_for_shutdown() || self.persistence_failed_for_shutdown()
    }

    pub(crate) fn persistence_shutdown_diagnostics(&self) -> String {
        format!(
            "checkpoint={:?}; sleep_worker_active={}; pending_sleep_entries={}; exact_waiting_for_sleep_journal={}; manual_waiting={}",
            self.exact_checkpoint_performance_state(),
            self.sleep_journal_publication_worker.is_some(),
            self.pending_sleep_journal_entries.len(),
            self.exact_checkpoint_waiting_for_sleep_journal,
            self.manual_checkpoint_waiting_for_sleep_journal.is_some()
        )
    }

    pub(crate) fn poll_persistence_for_shutdown(&mut self) -> Result<(), GameAppShellError> {
        self.poll_sleep_journal_publication()?;
        self.poll_exact_population_checkpoint()?;
        if matches!(
            self.exact_checkpoint_work,
            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { .. }
        ) {
            // A normal tick promotes durable Completed sleep states before it
            // tells the exact worker to finalize. Shutdown has quiesced ticks,
            // so finalize the already-durable boundary without inventing that
            // later Completed -> Committed transition.
            self.finalize_awaiting_exact_checkpoint(&[])?;
        }
        Ok(())
    }

    pub(super) fn flush_sleep_journal_publication_blocking(
        &mut self,
    ) -> Result<(), GameAppShellError> {
        loop {
            if let Some(worker) = self.sleep_journal_publication_worker.take() {
                let final_result = worker.finish().map_err(|_| {
                    GameAppShellError::from(ScaffoldContractError::NeuralBackendUnavailable)
                })?;
                self.admit_sleep_journal_publication(final_result)?;
            }
            if self.pending_sleep_journal_entries.is_empty() {
                return Ok(());
            }
            let pending = std::mem::take(&mut self.pending_sleep_journal_entries);
            self.start_sleep_journal_publication(pending)?;
        }
    }

    pub(super) fn request_exact_population_checkpoint(&mut self) -> Result<(), GameAppShellError> {
        if let Some(active) = self.exact_checkpoint_coordinator.active_identity() {
            let expected_base_digest = active.expected_base_digest.clone();
            let _ = self
                .exact_checkpoint_coordinator
                .request_exact(self.world.tick(), expected_base_digest)?;
            return Ok(());
        }
        if self.sleep_journal_publication_worker.is_some()
            || !self.pending_sleep_journal_entries.is_empty()
        {
            self.exact_checkpoint_waiting_for_sleep_journal = true;
            return Ok(());
        }
        let Some(durability) = self.checkpoint_durability.as_ref() else {
            return Ok(());
        };
        let checkpoint_tick = self.world.tick();
        let expected_base_digest = durability.published.digest.as_str().to_string();
        let disposition = self
            .exact_checkpoint_coordinator
            .request_exact(checkpoint_tick, expected_base_digest.clone())?;
        let ExactCheckpointRequestDispositionV1::Started { transaction_id } = disposition else {
            return Ok(());
        };
        self.exact_checkpoint_transaction_started_at =
            self.performance_measurement_enabled.then(Instant::now);
        self.performance_metrics
            .exact_checkpoint_transactions_started = self
            .performance_metrics
            .exact_checkpoint_transactions_started
            .saturating_add(1);
        let result = (|| {
            let base = self
                .checkpoint_durability
                .as_ref()
                .ok_or(ScaffoldContractError::MissingPhaseData)?
                .published
                .save
                .clone();
            let host = self.freeze_exact_population_host_snapshot(base)?;
            let capacity =
                BrainCapacityClass::production_for_id(self.brain_class.default_class_id())?;
            let context =
                GpuExactCheckpointTransactionContextV1::capture(self.backend.backend(), &capacity)?;
            let handles = self.handles.values().copied().collect::<Vec<_>>();
            let ticket = self.backend.submit_exact_population_capture(
                checkpoint_tick,
                transaction_id,
                &handles,
            )?;
            self.performance_metrics.sleep_checkpoint_capture_calls = self
                .performance_metrics
                .sleep_checkpoint_capture_calls
                .saturating_add(1);
            // The submitted exact capture starts a new neural-authority epoch.
            // Ordinary journal transitions remain queued until that epoch is
            // durably installed, so the prior compact cache is no longer a
            // valid source for later edges.
            self.sleep_journal_neural_authorities.clear();
            self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Capture {
                transaction_id,
                expected_base_digest,
                host,
                context,
                ticket,
            };
            Ok::<_, GameAppShellError>(())
        })();
        if result.is_err() {
            self.exact_checkpoint_coordinator.fail_stop();
            self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
        }
        result
    }

    pub(super) fn queue_exact_checkpoint_journal_entries(
        &mut self,
        entries: Vec<GpuSleepTransactionJournalEntryV2>,
    ) -> Result<(), GameAppShellError> {
        if !self.exact_checkpoint_coordinator.is_active() {
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        }
        append_bounded_sleep_journal_entries(
            &mut self.pending_exact_sleep_journal_entries,
            entries,
        )?;
        Ok(())
    }

    pub(super) fn take_exact_checkpoint_journal_writes(
        &mut self,
        permit: &DurableCompletedCheckpointPermitV1,
    ) -> Result<
        (
            Vec<ExactPopulationCheckpointJournalPromotionV1>,
            Vec<(u64, SleepJournalNeuralAuthority)>,
        ),
        GameAppShellError,
    > {
        let entries = self.pending_exact_sleep_journal_entries.clone();
        let mut captured_targets = BTreeMap::new();
        let mut follow_up_required = false;
        for entry in &entries {
            if entry.transition_tick <= permit.checkpoint_tick() {
                captured_targets.insert(entry.organism_id.raw(), entry.target);
            } else {
                follow_up_required = true;
            }
        }
        for (raw, expected_sleep) in captured_targets {
            let captured_sleep = permit
                .published()
                .save
                .creatures
                .iter()
                .find(|creature| creature.organism_id.raw() == raw)
                .and_then(|creature| creature.gpu_brain.as_ref())
                .map(|brain| brain.sleep)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            if !captured_sleep_covers_queued_target(expected_sleep, captured_sleep) {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
        }
        if follow_up_required {
            let _ = self.exact_checkpoint_coordinator.request_exact(
                self.world.tick(),
                permit.published().digest.as_str().to_string(),
            )?;
        }
        self.pending_exact_sleep_journal_entries.clear();
        Ok((Vec::new(), Vec::new()))
    }

    pub(super) fn retain_failed_exact_checkpoint_worker(
        &mut self,
        transaction_id: u64,
        worker: ExactPopulationCheckpointWorkerOwnerV1,
        error: GameAppShellError,
    ) {
        self.exact_checkpoint_coordinator.fail_stop();
        self.backend
            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::FailedJoining {
            transaction_id,
            failed: worker.abort_and_retain(error),
        };
    }

    pub(super) fn retain_failed_exact_checkpoint_capture(
        &mut self,
        transaction_id: u64,
        ticket: GpuExactPopulationCaptureTicketV1,
        error: GameAppShellError,
    ) {
        self.exact_checkpoint_coordinator.fail_stop();
        self.backend
            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::CaptureFailed {
            transaction_id,
            ticket,
            error: Some(error),
        };
    }

    pub(super) fn persist_sleep_checkpoint_boundary(&mut self) -> Result<(), GameAppShellError> {
        self.flush_sleep_journal_publication_blocking()?;
        let Some(mut durability) = self.checkpoint_durability.take() else {
            return Ok(());
        };
        self.performance_metrics.sleep_persistence_calls = self
            .performance_metrics
            .sleep_persistence_calls
            .saturating_add(1);
        let store = durability.store.clone();
        let readback_before = self.backend.mutable_slot_readback_metrics();
        let capture_started = Instant::now();
        let replacement = self.capture_checkpointed_save(durability.published.save.clone(), &store);
        let readback_after = self.backend.mutable_slot_readback_metrics();
        self.performance_metrics.sleep_checkpoint_capture_calls = self
            .performance_metrics
            .sleep_checkpoint_capture_calls
            .saturating_add(1);
        self.performance_metrics.sleep_checkpoint_capture_wall_ns = self
            .performance_metrics
            .sleep_checkpoint_capture_wall_ns
            .saturating_add(elapsed_ns(capture_started));
        self.performance_metrics.sleep_checkpoint_readback_calls = self
            .performance_metrics
            .sleep_checkpoint_readback_calls
            .saturating_add(readback_after.calls.saturating_sub(readback_before.calls));
        self.performance_metrics.sleep_checkpoint_readback_bytes = self
            .performance_metrics
            .sleep_checkpoint_readback_bytes
            .saturating_add(readback_after.bytes.saturating_sub(readback_before.bytes));
        self.performance_metrics
            .sleep_checkpoint_readback_poll_wait_ns = self
            .performance_metrics
            .sleep_checkpoint_readback_poll_wait_ns
            .saturating_add(
                readback_after
                    .poll_wait_ns
                    .saturating_sub(readback_before.poll_wait_ns),
            );
        self.performance_metrics
            .sleep_checkpoint_readback_map_receive_wait_ns = self
            .performance_metrics
            .sleep_checkpoint_readback_map_receive_wait_ns
            .saturating_add(
                readback_after
                    .map_receive_wait_ns
                    .saturating_sub(readback_before.map_receive_wait_ns),
            );
        let result = match replacement {
            Ok((replacement, exact_neural_captures)) => {
                self.performance_metrics
                    .sleep_exact_neural_capture_organisms = self
                    .performance_metrics
                    .sleep_exact_neural_capture_organisms
                    .saturating_add(exact_neural_captures);
                let prospective = durability.prospective_durable_reference(&replacement);
                match prospective.and_then(|reference| {
                    self.backend
                        .prevalidate_durable_checkpoint(reference)
                        .map_err(Into::into)
                }) {
                    Ok(permit) => {
                        let publish_started = Instant::now();
                        let result = durability.publish(replacement).map(|_| permit);
                        self.performance_metrics.sleep_checkpoint_publish_calls = self
                            .performance_metrics
                            .sleep_checkpoint_publish_calls
                            .saturating_add(1);
                        self.performance_metrics.sleep_checkpoint_publish_wall_ns = self
                            .performance_metrics
                            .sleep_checkpoint_publish_wall_ns
                            .saturating_add(elapsed_ns(publish_started));
                        result
                    }
                    Err(error) => Err(error),
                }
            }
            Err(error) => Err(error),
        };
        self.checkpoint_durability = Some(durability);
        let permit = result?;
        self.backend.install_prevalidated_durable_checkpoint(permit);
        Ok(())
    }

    pub(super) fn promote_durable_completed_sleep_batch(
        &mut self,
        promotions: &[(OrganismId, SleepState)],
    ) -> Result<(), GameAppShellError> {
        if promotions.is_empty() {
            return Ok(());
        }
        self.finalize_awaiting_exact_checkpoint(promotions)
    }

    pub(super) fn finalize_awaiting_exact_checkpoint(
        &mut self,
        promotions: &[(OrganismId, SleepState)],
    ) -> Result<(), GameAppShellError> {
        let work = std::mem::take(&mut self.exact_checkpoint_work);
        let ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, worker } = work
        else {
            self.exact_checkpoint_work = work;
            return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
        };
        permit.validate_restored_provenance()?;
        let transaction_id = permit.transaction_id();
        let (mut worker_promotions, queued_authorities) =
            match self.take_exact_checkpoint_journal_writes(&permit) {
                Ok(writes) => writes,
                Err(error) => {
                    self.retain_failed_exact_checkpoint_worker(transaction_id, worker, error);
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
                }
            };
        if !promotions.is_empty() {
            self.performance_metrics.sleep_promotion_calls = self
                .performance_metrics
                .sleep_promotion_calls
                .saturating_add(1);
        }
        let prepared = (|| {
            let mut ordered = promotions.to_vec();
            ordered.sort_unstable_by_key(|(organism_id, _)| organism_id.raw());
            if ordered.windows(2).any(|pair| pair[0].0 == pair[1].0) {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            let base = &permit.published().save;
            let mut authorities = queued_authorities
                .iter()
                .cloned()
                .collect::<BTreeMap<_, _>>();
            for (organism_id, committed_sleep) in ordered {
                let creature = base
                    .creatures
                    .iter()
                    .find(|creature| creature.organism_id == organism_id)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let completed = creature
                    .gpu_brain
                    .as_ref()
                    .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                let promoted = completed.promoted_completed_sleep_state()?;
                if promoted.sleep != committed_sleep || promoted.checkpoint_tick != base.world.tick
                {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
                }
                let resident = self
                    .residents
                    .get(&organism_id.raw())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let authority = permit
                    .captured_journal_authorities()
                    .get(&organism_id.raw())
                    .cloned()
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let entry = GpuSleepTransactionJournalEntryV2::try_new(
                    organism_id,
                    Tick::new(completed.checkpoint_tick.raw().saturating_add(1)),
                    completed.sleep,
                    committed_sleep,
                )?;
                worker_promotions.push(ExactPopulationCheckpointJournalPromotionV1 {
                    entry,
                    authority: authority.clone(),
                    phenotype: resident.phenotype.clone(),
                });
                authorities.insert(organism_id.raw(), authority);
            }
            worker_promotions.sort_unstable_by_key(|write| {
                (
                    write.entry.organism_id.raw(),
                    write.entry.transition_tick.raw(),
                    write.entry.transition_ordinal,
                )
            });
            Ok::<_, GameAppShellError>((
                worker_promotions,
                authorities.into_iter().collect::<Vec<_>>(),
            ))
        })();
        let (worker_promotions, authorities) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                self.retain_failed_exact_checkpoint_worker(
                    transaction_id,
                    worker,
                    ScaffoldContractError::NeuralBackendUnavailable.into(),
                );
                return Err(error);
            }
        };
        let entry_count = u64::try_from(worker_promotions.len()).unwrap_or(u64::MAX);
        let manual = match self
            .exact_checkpoint_coordinator
            .take_pending_manual_after_durable_permit()
        {
            Ok(manual) => manual,
            Err(error) => {
                self.retain_failed_exact_checkpoint_worker(
                    transaction_id,
                    worker,
                    ScaffoldContractError::NeuralBackendUnavailable.into(),
                );
                return Err(error.into());
            }
        };
        if let Err(error) = self
            .exact_checkpoint_coordinator
            .transition(ExactPopulationCheckpointStageV1::DeferredJournalPublishing)
        {
            self.retain_failed_exact_checkpoint_worker(
                transaction_id,
                worker,
                ScaffoldContractError::NeuralBackendUnavailable.into(),
            );
            return Err(error.into());
        }
        if worker
            .try_send_command(ExactPopulationCheckpointWorkerCommandV1::Finalize {
                promotions: worker_promotions,
                manual,
            })
            .is_err()
        {
            let error = GameAppShellError::Core(ScaffoldContractError::NeuralBackendUnavailable);
            self.retain_failed_exact_checkpoint_worker(transaction_id, worker, error);
            return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
        }
        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::JournalWorker {
            transaction_id,
            worker,
            journal_commit: Some(ExactPopulationCheckpointJournalCommitV1 {
                authorities,
                entry_count,
                contains_completed_promotion: !promotions.is_empty(),
            }),
        };
        Ok(())
    }
}
