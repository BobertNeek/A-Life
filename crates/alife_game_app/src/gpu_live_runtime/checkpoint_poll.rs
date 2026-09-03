//! Exact-population checkpoint transaction polling and terminal admission.

use super::*;

impl GpuLiveBrainRuntime {
    pub(super) fn poll_exact_population_checkpoint(&mut self) -> Result<(), GameAppShellError> {
        let work = std::mem::take(&mut self.exact_checkpoint_work);
        match work {
            ExactPopulationCheckpointRuntimeWorkV1::Idle => Ok(()),
            ExactPopulationCheckpointRuntimeWorkV1::Failed => {
                self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                Err(ScaffoldContractError::NeuralBackendUnavailable.into())
            }
            ExactPopulationCheckpointRuntimeWorkV1::Capture {
                transaction_id,
                expected_base_digest,
                host,
                context,
                mut ticket,
            } => {
                if self.exact_checkpoint_coordinator.stage()
                    == ExactPopulationCheckpointStageV1::CaptureSubmitted
                {
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::MappingPending)
                    {
                        self.retain_failed_exact_checkpoint_capture(
                            transaction_id,
                            ticket,
                            error.into(),
                        );
                        return Ok(());
                    }
                }
                let poll = match self.backend.poll_exact_population_capture(&mut ticket) {
                    Ok(poll) => poll,
                    Err(error) => {
                        self.retain_failed_exact_checkpoint_capture(
                            transaction_id,
                            ticket,
                            error.into(),
                        );
                        return Ok(());
                    }
                };
                match poll {
                    GpuExactPopulationCapturePollV1::Pending => {
                        self.exact_checkpoint_work =
                            ExactPopulationCheckpointRuntimeWorkV1::Capture {
                                transaction_id,
                                expected_base_digest,
                                host,
                                context,
                                ticket,
                            };
                        Ok(())
                    }
                    GpuExactPopulationCapturePollV1::Failed(_) => {
                        self.exact_checkpoint_coordinator.fail_stop();
                        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                        self.backend
                            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                        Err(ScaffoldContractError::NeuralBackendUnavailable.into())
                    }
                    GpuExactPopulationCapturePollV1::Ready(capture) => {
                        if let Err(error) = self
                            .exact_checkpoint_coordinator
                            .transition(ExactPopulationCheckpointStageV1::CpuBytesReady)
                        {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(error.into());
                        }
                        let Some(active) =
                            self.exact_checkpoint_coordinator.active_identity().cloned()
                        else {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(
                                ScaffoldContractError::ConsolidationGenerationMismatch.into()
                            );
                        };
                        if active.transaction_id != transaction_id
                            || active.checkpoint_tick != host.checkpoint_tick
                            || active.expected_base_digest != expected_base_digest
                            || capture.capture_transaction_generation() != transaction_id
                        {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(
                                ScaffoldContractError::ConsolidationGenerationMismatch.into()
                            );
                        }
                        let Some(durability) = self.checkpoint_durability.take() else {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(ScaffoldContractError::MissingPhaseData.into());
                        };
                        if let Err(error) = self
                            .exact_checkpoint_coordinator
                            .transition(ExactPopulationCheckpointStageV1::Encoding)
                        {
                            self.checkpoint_durability = Some(durability);
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(error.into());
                        }
                        let capture_transaction_generation =
                            capture.capture_transaction_generation();
                        let population_set_digest = capture.population_set_digest();
                        let worker = spawn_exact_population_checkpoint_worker(
                            transaction_id,
                            expected_base_digest.clone(),
                            host,
                            capture,
                            context,
                            durability,
                        );
                        self.exact_checkpoint_work =
                            ExactPopulationCheckpointRuntimeWorkV1::Worker {
                                transaction_id,
                                checkpoint_tick: active.checkpoint_tick,
                                expected_base_digest,
                                capture_transaction_generation,
                                population_set_digest,
                                worker,
                            };
                        Ok(())
                    }
                }
            }
            ExactPopulationCheckpointRuntimeWorkV1::CaptureFailed {
                transaction_id,
                mut ticket,
                mut error,
            } => match self.backend.poll_exact_population_capture(&mut ticket) {
                Ok(GpuExactPopulationCapturePollV1::Pending) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::CaptureFailed {
                            transaction_id,
                            ticket,
                            error,
                        };
                    Ok(())
                }
                Ok(GpuExactPopulationCapturePollV1::Ready(_))
                | Ok(GpuExactPopulationCapturePollV1::Failed(_)) => {
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    Err(error
                        .take()
                        .unwrap_or_else(|| ScaffoldContractError::NeuralBackendUnavailable.into()))
                }
                Err(poll_error) => {
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    Err(error.take().unwrap_or_else(|| poll_error.into()))
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::Worker {
                transaction_id,
                checkpoint_tick,
                expected_base_digest,
                capture_transaction_generation,
                population_set_digest,
                worker,
            } => match worker.try_recv_event() {
                Ok(None) => {
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Worker {
                        transaction_id,
                        checkpoint_tick,
                        expected_base_digest,
                        capture_transaction_generation,
                        population_set_digest,
                        worker,
                    };
                    Ok(())
                }
                Err(_) => {
                    self.retain_failed_exact_checkpoint_worker(
                        transaction_id,
                        worker,
                        ScaffoldContractError::NeuralBackendUnavailable.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::Final(report))) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                            transaction_id,
                            report,
                            join_handle: worker.into_join_handle(),
                            journal_commit: None,
                        };
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::ExactPublished(_))) => {
                    self.retain_failed_exact_checkpoint_worker(
                        transaction_id,
                        worker,
                        ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::ManifestPrepared(prepared))) => {
                    if prepared.transaction_id != transaction_id
                        || prepared.checkpoint_tick != checkpoint_tick
                        || prepared.expected_base_digest != expected_base_digest
                        || prepared.capture_transaction_generation != capture_transaction_generation
                        || prepared.population_set_digest != population_set_digest
                        || prepared.prospective_durable_reference.checkpoint_tick != checkpoint_tick
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                        );
                        return Ok(());
                    }
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::ManifestPrepared)
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            error.into(),
                        );
                        return Ok(());
                    }
                    let permit = match self.backend.prevalidate_durable_checkpoint(
                        prepared.prospective_durable_reference.clone(),
                    ) {
                        Ok(permit) => permit,
                        Err(error) => {
                            let surfaced_error = GameAppShellError::from(error);
                            self.retain_failed_exact_checkpoint_worker(
                                transaction_id,
                                worker,
                                ScaffoldContractError::NeuralBackendUnavailable.into(),
                            );
                            return Err(surfaced_error);
                        }
                    };
                    if worker
                        .try_send_command(ExactPopulationCheckpointWorkerCommandV1::CommitExact)
                        .is_err()
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            ScaffoldContractError::NeuralBackendUnavailable.into(),
                        );
                        return Ok(());
                    }
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::CommitWorker {
                            prepared,
                            permit,
                            worker,
                        };
                    Ok(())
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::CommitWorker {
                prepared,
                permit,
                worker,
            } => match worker.try_recv_event() {
                Ok(None) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::CommitWorker {
                            prepared,
                            permit,
                            worker,
                        };
                    Ok(())
                }
                Err(_) => {
                    self.retain_failed_exact_checkpoint_worker(
                        prepared.transaction_id,
                        worker,
                        ScaffoldContractError::NeuralBackendUnavailable.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::ManifestPrepared(_))) => {
                    self.retain_failed_exact_checkpoint_worker(
                        prepared.transaction_id,
                        worker,
                        ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::Final(report))) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                            transaction_id: prepared.transaction_id,
                            report,
                            join_handle: worker.into_join_handle(),
                            journal_commit: None,
                        };
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::ExactPublished(success))) => {
                    let transaction_id = prepared.transaction_id;
                    if success.transaction_id != transaction_id
                        || success.checkpoint_tick != prepared.checkpoint_tick
                        || success.expected_base_digest != prepared.expected_base_digest
                        || success.capture_transaction_generation
                            != prepared.capture_transaction_generation
                        || success.population_set_digest != prepared.population_set_digest
                        || success.durable_reference != prepared.prospective_durable_reference
                        || success.exact_neural_captures != prepared.exact_neural_captures
                        || success.captured_journal_authorities
                            != prepared.captured_journal_authorities
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                        );
                        return Ok(());
                    }
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::CasCommitted)
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            error.into(),
                        );
                        return Ok(());
                    }
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::ReloadValidated)
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            error.into(),
                        );
                        return Ok(());
                    }
                    self.backend.install_prevalidated_durable_checkpoint(permit);
                    self.performance_metrics
                        .sleep_exact_neural_capture_organisms = self
                        .performance_metrics
                        .sleep_exact_neural_capture_organisms
                        .saturating_add(success.exact_neural_captures);
                    if let Err(error) = self
                        .exact_checkpoint_coordinator
                        .transition(ExactPopulationCheckpointStageV1::DurablePermitInstalled)
                    {
                        self.retain_failed_exact_checkpoint_worker(
                            transaction_id,
                            worker,
                            error.into(),
                        );
                        return Ok(());
                    }
                    let permit = DurableCompletedCheckpointPermitV1::Captured(success);
                    let has_completed = permit.published().save.creatures.iter().any(|creature| {
                        creature.gpu_brain.as_ref().is_some_and(|brain| {
                            matches!(
                                brain.sleep.consolidation,
                                ConsolidationState::Completed { .. }
                            )
                        })
                    });
                    if has_completed {
                        self.exact_checkpoint_work =
                            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal {
                                permit,
                                worker,
                            };
                    } else {
                        let (journal_writes, authorities) =
                            match self.take_exact_checkpoint_journal_writes(&permit) {
                                Ok(writes) => writes,
                                Err(error) => {
                                    self.retain_failed_exact_checkpoint_worker(
                                        transaction_id,
                                        worker,
                                        error,
                                    );
                                    return Ok(());
                                }
                            };
                        let journal_entry_count =
                            u64::try_from(journal_writes.len()).unwrap_or(u64::MAX);
                        let manual = match self
                            .exact_checkpoint_coordinator
                            .take_pending_manual_after_durable_permit()
                        {
                            Ok(manual) => manual,
                            Err(error) => {
                                self.retain_failed_exact_checkpoint_worker(
                                    transaction_id,
                                    worker,
                                    error.into(),
                                );
                                return Ok(());
                            }
                        };
                        if let Err(error) = self
                            .exact_checkpoint_coordinator
                            .transition(ExactPopulationCheckpointStageV1::DeferredJournalPublishing)
                        {
                            self.retain_failed_exact_checkpoint_worker(
                                transaction_id,
                                worker,
                                error.into(),
                            );
                            return Ok(());
                        }
                        if worker
                            .try_send_command(ExactPopulationCheckpointWorkerCommandV1::Finalize {
                                promotions: journal_writes,
                                manual,
                            })
                            .is_err()
                        {
                            self.retain_failed_exact_checkpoint_worker(
                                transaction_id,
                                worker,
                                ScaffoldContractError::NeuralBackendUnavailable.into(),
                            );
                            return Ok(());
                        }
                        self.exact_checkpoint_work =
                            ExactPopulationCheckpointRuntimeWorkV1::JournalWorker {
                                transaction_id,
                                worker,
                                journal_commit: Some(ExactPopulationCheckpointJournalCommitV1 {
                                    entry_count: journal_entry_count,
                                    authorities,
                                    contains_completed_promotion: false,
                                }),
                            };
                    }
                    Ok(())
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, worker } => {
                self.exact_checkpoint_work =
                    ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, worker };
                Ok(())
            }
            ExactPopulationCheckpointRuntimeWorkV1::JournalWorker {
                transaction_id,
                worker,
                journal_commit,
            } => match worker.try_recv_event() {
                Ok(None) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::JournalWorker {
                            transaction_id,
                            worker,
                            journal_commit,
                        };
                    Ok(())
                }
                Err(_) => {
                    self.retain_failed_exact_checkpoint_worker(
                        transaction_id,
                        worker,
                        ScaffoldContractError::NeuralBackendUnavailable.into(),
                    );
                    Ok(())
                }
                Ok(Some(
                    ExactPopulationCheckpointWorkerEventV1::ManifestPrepared(_)
                    | ExactPopulationCheckpointWorkerEventV1::ExactPublished(_),
                )) => {
                    self.retain_failed_exact_checkpoint_worker(
                        transaction_id,
                        worker,
                        ScaffoldContractError::ConsolidationGenerationMismatch.into(),
                    );
                    Ok(())
                }
                Ok(Some(ExactPopulationCheckpointWorkerEventV1::Final(report))) => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                            transaction_id,
                            report,
                            join_handle: worker.into_join_handle(),
                            journal_commit,
                        };
                    Ok(())
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::FailedJoining {
                transaction_id,
                mut failed,
            } => match failed.poll() {
                FailedExactPopulationCheckpointWorkerJoinPollV1::Pending => {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::FailedJoining {
                            transaction_id,
                            failed,
                        };
                    Ok(())
                }
                FailedExactPopulationCheckpointWorkerJoinPollV1::Ready {
                    error,
                    worker_panicked,
                } => {
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    if worker_panicked {
                        Err(ScaffoldContractError::NeuralBackendUnavailable.into())
                    } else {
                        Err(error)
                    }
                }
            },
            ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                transaction_id,
                report,
                join_handle,
                journal_commit,
            } => {
                if !join_handle.is_finished() {
                    self.exact_checkpoint_work =
                        ExactPopulationCheckpointRuntimeWorkV1::Finalizing {
                            transaction_id,
                            report,
                            join_handle,
                            journal_commit,
                        };
                    return Ok(());
                }
                if join_handle.join().is_err() {
                    self.exact_checkpoint_coordinator.fail_stop();
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    self.backend
                        .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                    return Err(ScaffoldContractError::NeuralBackendUnavailable.into());
                }
                self.checkpoint_durability = Some(report.durability);
                if let Err(error) = report.result {
                    if let GpuManualCheckpointStatus::Queued { destination, .. } =
                        &self.manual_checkpoint_status
                    {
                        self.manual_checkpoint_status = GpuManualCheckpointStatus::Failed {
                            destination: destination.clone(),
                            message: error.to_string(),
                        };
                    }
                    self.exact_checkpoint_coordinator.fail_stop();
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    self.backend
                        .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                    return Err(error);
                }
                if let Some(manual) = report.manual_completion {
                    self.manual_checkpoint_status = GpuManualCheckpointStatus::Complete {
                        destination: manual.destination,
                        checkpoint_tick: manual.checkpoint_tick,
                    };
                }
                if let Some(journal_commit) = journal_commit {
                    if journal_commit.contains_completed_promotion {
                        self.performance_metrics.sleep_promotion_publish_calls = self
                            .performance_metrics
                            .sleep_promotion_publish_calls
                            .saturating_add(1);
                    }
                    let mut current_authorities =
                        Vec::with_capacity(journal_commit.authorities.len());
                    for (organism_id_raw, _) in &journal_commit.authorities {
                        let Some(handle) = self.handles.get(organism_id_raw).copied() else {
                            self.exact_checkpoint_coordinator.fail_stop();
                            self.exact_checkpoint_work =
                                ExactPopulationCheckpointRuntimeWorkV1::Failed;
                            self.backend
                                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                            return Err(ScaffoldContractError::BrainOwnershipMismatch.into());
                        };
                        let authority =
                            match capture_sleep_journal_neural_authority(&mut self.backend, handle)
                            {
                                Ok(authority) => authority,
                                Err(error) => {
                                    self.exact_checkpoint_coordinator.fail_stop();
                                    self.exact_checkpoint_work =
                                        ExactPopulationCheckpointRuntimeWorkV1::Failed;
                                    self.backend.fail_stop(
                                        GpuSessionFailStopCause::CheckpointRestoreFailed,
                                    );
                                    return Err(error.into());
                                }
                            };
                        current_authorities.push((*organism_id_raw, authority));
                    }
                    // Worker validation and publication use the immutable
                    // tick-T authority. Only after that durable success may
                    // later compact edges bind the now-promoted resident host
                    // metadata. This performs no mutable-buffer readback.
                    for (organism_id_raw, authority) in current_authorities {
                        self.sleep_journal_neural_authorities
                            .insert(organism_id_raw, authority);
                    }
                    self.performance_metrics.sleep_compact_journal_organisms = self
                        .performance_metrics
                        .sleep_compact_journal_organisms
                        .saturating_add(journal_commit.entry_count);
                }
                if !self.pending_exact_sleep_journal_entries.is_empty() {
                    let durability = self
                        .checkpoint_durability
                        .as_ref()
                        .ok_or(ScaffoldContractError::MissingPhaseData)?;
                    let _ = self.exact_checkpoint_coordinator.request_exact(
                        self.world.tick(),
                        durability.published.digest.as_str().to_string(),
                    )?;
                }
                if let Err(error) = self
                    .exact_checkpoint_coordinator
                    .transition(ExactPopulationCheckpointStageV1::Complete)
                {
                    self.exact_checkpoint_coordinator.fail_stop();
                    self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                    self.backend
                        .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                    return Err(error.into());
                }
                let follow_up = match self.exact_checkpoint_coordinator.finish() {
                    Ok(follow_up) => follow_up,
                    Err(error) => {
                        self.exact_checkpoint_coordinator.fail_stop();
                        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                        self.backend
                            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                        return Err(error.into());
                    }
                };
                self.performance_metrics
                    .exact_checkpoint_transactions_completed = self
                    .performance_metrics
                    .exact_checkpoint_transactions_completed
                    .saturating_add(1);
                self.performance_metrics
                    .exact_checkpoint_transaction_wall_ns = self
                    .performance_metrics
                    .exact_checkpoint_transaction_wall_ns
                    .saturating_add(
                        self.exact_checkpoint_transaction_started_at
                            .take()
                            .map_or(0, elapsed_ns),
                    );
                self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Idle;
                if !self.pending_sleep_journal_entries.is_empty() {
                    let pending = std::mem::take(&mut self.pending_sleep_journal_entries);
                    self.start_sleep_journal_publication(pending)?;
                }
                if follow_up {
                    if let Err(error) = self.request_exact_population_checkpoint() {
                        self.exact_checkpoint_coordinator.fail_stop();
                        self.exact_checkpoint_work = ExactPopulationCheckpointRuntimeWorkV1::Failed;
                        self.backend
                            .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                        return Err(error);
                    }
                }
                Ok(())
            }
        }
    }
}
