//! One staged authoritative world and GPU cognition tick.

use super::*;

impl GpuLiveBrainRuntime {
    pub(super) fn tick_with_sleep_progress_staged<F>(
        &mut self,
        progress: &mut F,
    ) -> Result<Vec<LiveBrainTickSummary>, GameAppShellError>
    where
        F: FnMut(
            &mut GpuClosedLoopBackend,
            GpuBrainHandle,
            OrganismId,
            SleepState,
            Option<ConsolidationIntent>,
        ) -> SleepProgressResult,
    {
        let preamble_started = Instant::now();
        let curated_first_tick_resident = match self.curated_first_tick_residency_gate() {
            Ok(receipt) => receipt.and_then(|receipt| receipt.ordered_residents.first().cloned()),
            Err(error) => {
                self.backend
                    .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
                return Err(error);
            }
        };
        let curated_first_tick = curated_first_tick_resident.is_some();
        self.retire_dead_organisms()?;
        self.reconcile_population()?;
        self.last_sealed_patches
            .retain(|patch| self.handles.contains_key(&patch.header().organism_id.raw()));
        self.restored_replay_patches
            .retain(|patch| self.handles.contains_key(&patch.header().organism_id.raw()));
        self.last_learning_receipts.clear();
        self.last_gpu_authority_receipts.clear();
        self.last_activity_work_receipts.clear();
        self.last_cognitive_work_receipts.clear();
        self.last_memory_recall_receipts.clear();
        self.last_memory_update_receipts.clear();
        self.last_cognitive_context_digests.clear();
        self.last_memory_compaction_receipts.clear();
        self.last_memory_preparation_errors.clear();
        self.last_memory_observation_errors.clear();
        self.last_topology_observations.clear();
        self.last_eligibility_discard_receipts.clear();
        self.last_pre_seal_discard_failures.clear();
        self.last_post_seal_learning_failures.clear();
        #[cfg(feature = "gpu-tests")]
        {
            self.last_sleep_memory_compaction_preparation_count = 0;
        }
        if self.handles.is_empty() {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "GPU neural policy requires at least one live organism",
            });
        }

        let tick_before = self.world.tick();
        let tick_after = Tick::new(tick_before.raw().saturating_add(1));
        let checkpoint_active = self.exact_checkpoint_coordinator.is_active();
        // A durable checkpoint permit is organism-scoped. Only the exact
        // Completed states captured in the active immutable save may advance
        // to Committed under this worker. Founders that complete later remain
        // Completed until the one bounded follow-up capture makes their state
        // durable; they must never be folded into the first worker's finalize.
        let durable_completed_sleep_permits = match &self.exact_checkpoint_work {
            ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { permit, .. } => {
                permit.validate_restored_provenance()?;
                permit
                    .published()
                    .save
                    .creatures
                    .iter()
                    .filter_map(|creature| {
                        let brain = creature.gpu_brain.as_ref()?;
                        matches!(
                            brain.sleep.consolidation,
                            ConsolidationState::Completed { .. }
                        )
                        .then_some((creature.organism_id.raw(), brain.sleep))
                    })
                    .collect::<BTreeMap<_, _>>()
            }
            _ => BTreeMap::new(),
        };
        let completed_sleep_states = self
            .residents
            .iter()
            .filter_map(|(&raw, resident)| {
                let sleep = resident.sleep_scheduler.state();
                durable_completed_sleep_permits
                    .get(&raw)
                    .is_some_and(|durable| *durable == sleep)
                    .then_some((OrganismId(raw), sleep))
            })
            .collect::<Vec<_>>();
        let mut prepared_memory_commits = BTreeMap::new();
        for (organism_id, completed_sleep) in completed_sleep_states {
            let prepared =
                self.prepare_memory_compaction_at_sleep_commit(organism_id, completed_sleep)?;
            #[cfg(feature = "gpu-tests")]
            {
                self.last_sleep_memory_compaction_preparation_count = self
                    .last_sleep_memory_compaction_preparation_count
                    .saturating_add(1);
            }
            if prepared_memory_commits
                .insert(organism_id.raw(), prepared)
                .is_some()
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
        }
        let homeostatic_parameters = self.homeostatic_parameters;
        let mut batch = Vec::with_capacity(self.handles.len());
        let mut summaries_by_organism = BTreeMap::new();
        let mut scheduled_body_events = BTreeMap::new();
        let mut persist_exact_sleep_boundary = false;
        let mut sleep_journal_entries = Vec::new();
        let mut sleep_journal_neural_authority_updates = BTreeMap::new();
        let mut completed_promotions = Vec::new();
        let scheduled_handles = if let Some(first) = curated_first_tick_resident {
            vec![(
                first.organism_id.raw(),
                first.handle,
                WorldEntityId(first.opaque_target_identity.raw()),
            )]
        } else {
            self.handles
                .iter()
                .map(|(&raw, &handle)| {
                    let organism_id = OrganismId(raw);
                    let record = self
                        .world
                        .organism_registry()
                        .get(organism_id)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    let world_entity_id = record.world_entity_id();
                    let object = self
                        .world
                        .entity(world_entity_id)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    if object.kind != WorldObjectKind::Agent
                        || object.organism_id != Some(organism_id)
                    {
                        return Err(ScaffoldContractError::BrainOwnershipMismatch);
                    }
                    Ok::<_, ScaffoldContractError>((raw, handle, world_entity_id))
                })
                .collect::<Result<Vec<_>, _>>()?
        };
        let perception_index = self.world.build_perception_batch_index()?;
        self.performance_metrics.tick_preamble_wall_ns = self
            .performance_metrics
            .tick_preamble_wall_ns
            .saturating_add(elapsed_ns(preamble_started));
        let preparation_started = Instant::now();
        let measure_preparation = self.performance_measurement_enabled;
        let mut sleep_eligibility_replay_wall_ns = 0_u64;
        let mut sleep_timing = SleepPreparationTiming::default();
        let mut grounded_perception_wall_ns = 0_u64;
        let mut episodic_retrieval_wall_ns = 0_u64;
        let mut attention_context_wall_ns = 0_u64;
        let mut topology_concept_wall_ns = 0_u64;
        let mut gpu_upload_wall_ns = 0_u64;
        let mut checkpoint_publication_wall_ns = 0_u64;
        for (raw, handle, world_entity_id) in scheduled_handles {
            let sleep_preparation_started = measure_preparation.then(Instant::now);
            let retained_learning_pending =
                self.retry_retained_learning(OrganismId(raw), tick_before)?;
            let mut record = self
                .world
                .organism_registry()
                .get(OrganismId(raw))
                .cloned()
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = self
                .residents
                .get_mut(&raw)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            synchronize_resident_from_record(resident, &record, tick_before)?;
            let sleep_before = resident.sleep_scheduler.state();
            let phase_before = sleep_before.phase;
            let completed_waiting_for_durable_permit = matches!(
                sleep_before.consolidation,
                ConsolidationState::Completed { .. }
            ) && !durable_completed_sleep_permits
                .get(&raw)
                .is_some_and(|durable| *durable == sleep_before);
            let allow_sleep_progress = !completed_waiting_for_durable_permit;
            // Fixed continuous-wake lab protocols suppress sleep phases but
            // keep the production work-cost ledger. Applying the existing
            // sleep-rate recovery prevents ecology energy exhaustion from
            // truncating their bounded neural measurement windows.
            match brain_atp_world_tick_mode(
                phase_before,
                self.schedule_sleep,
                completed_waiting_for_durable_permit,
            ) {
                BrainAtpWorldTickMode::Charge { recover } => {
                    self.backend
                        .charge_world_brain_atp_tick(handle, tick_before.raw(), recover)?;
                }
                BrainAtpWorldTickMode::DurabilityHold => {
                    self.backend
                        .hold_world_brain_atp_tick(handle, tick_before.raw())?;
                }
            }
            if self.schedule_sleep
                && phase_before == SleepPhase::Awake
                && !retained_learning_pending
                && !self.backend.next_bounded_activity_is_affordable(handle)?
            {
                resident.sleep_scheduler.force_recovery_sleep(tick_before)?;
            }
            let sleep_event = if self.schedule_sleep && allow_sleep_progress {
                let sleep_config = sleep_consolidation_config_for(&resident.phenotype)?;
                let mut routed_driver = RoutedGpuSleepDriver {
                    authoritative: AuthoritativeGpuSleepDriver {
                        backend: &mut self.backend,
                        handle,
                        sleep_config: Some(sleep_config),
                        context: Some(AuthoritativeSleepContext {
                            memory: self
                                .memories
                                .get_mut(&raw)
                                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?,
                            predictor: &mut resident.predictor,
                            topology: self
                                .topologies
                                .get_mut(&raw)
                                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?,
                            restored_replay_patches: &self.restored_replay_patches,
                            sealed_patches: &self.sealed_patches,
                            last_sealed_patches: &self.last_sealed_patches,
                        }),
                        replay_evidence_before_commit: None,
                        last_sleep_work: Some(&mut resident.last_sleep_work),
                    },
                    progress,
                    timing: &mut sleep_timing,
                    measure: measure_preparation,
                };
                let event = resident.sleep_scheduler.scheduled_tick_with_organism(
                    &mut record,
                    homeostatic_parameters,
                    tick_before,
                    &mut routed_driver,
                    false,
                )?;
                replace_canonical_organism_record(&mut self.world, record)?;
                if event.sleep_work_units > 0 {
                    let sleep_work = resident
                        .last_sleep_work
                        .as_ref()
                        .ok_or(ScaffoldContractError::MissingPhaseData)?;
                    let cognitive_work = sleep_cognitive_work_receipt(sleep_work)?;
                    resident.last_cognitive_work = cognitive_work;
                    self.last_cognitive_work_receipts.push(cognitive_work);
                    apply_cognitive_work_cost(
                        &mut self.world,
                        OrganismId(raw),
                        cognitive_work,
                        self.cognitive_work_cost_policy,
                    )?;
                }
                event
            } else if !self.schedule_sleep {
                if phase_before != SleepPhase::Awake {
                    return Err(ScaffoldContractError::MissingPhaseData.into());
                }
                GpuSleepScheduleEvent {
                    tick: tick_before,
                    phase: SleepPhase::Awake,
                    cycle_id: sleep_before.last_consolidated_cycle_id,
                    transition: None,
                    consolidation_kind_raw: sleep_before.consolidation.kind_raw(),
                    selected_action: None,
                    motor_eligible: motor_eligible(SleepPhase::Awake),
                    sleep_work_units: 0,
                    phase_receipt: SleepPhaseReceipt {
                        phase: SleepPhase::Awake,
                        cycle_id: sleep_before.last_consolidated_cycle_id,
                        tick: tick_before,
                        due_work: SleepWorkDue::empty(),
                        work_units: 0,
                        cumulative_work_units: 0,
                        sealed: false,
                    },
                }
            } else {
                GpuSleepScheduleEvent {
                    tick: tick_before,
                    phase: phase_before,
                    cycle_id: if sleep_before.active_cycle_id != 0 {
                        sleep_before.active_cycle_id
                    } else {
                        sleep_before.last_consolidated_cycle_id
                    },
                    transition: None,
                    consolidation_kind_raw: sleep_before.consolidation.kind_raw(),
                    selected_action: None,
                    motor_eligible: motor_eligible(phase_before),
                    sleep_work_units: 0,
                    phase_receipt: SleepPhaseReceipt {
                        phase: phase_before,
                        cycle_id: if sleep_before.active_cycle_id != 0 {
                            sleep_before.active_cycle_id
                        } else {
                            sleep_before.last_consolidated_cycle_id
                        },
                        tick: tick_before,
                        due_work: SleepWorkDue::empty(),
                        work_units: 0,
                        cumulative_work_units: 0,
                        sealed: false,
                    },
                }
            };
            let sleep_after = resident.sleep_scheduler.state();
            sleep_eligibility_replay_wall_ns = sleep_eligibility_replay_wall_ns
                .saturating_add(sleep_preparation_started.map_or(0, elapsed_ns));
            if sleep_recovery_body_event_due(phase_before, completed_waiting_for_durable_permit) {
                scheduled_body_events.insert(
                    raw,
                    BodyEventDelta {
                        sleep_recovery: 1.0,
                        ..BodyEventDelta::zero()
                    },
                );
            }
            let checkpoint_preparation_started = measure_preparation.then(Instant::now);
            if sleep_after != sleep_before {
                match (sleep_before.consolidation, sleep_after.consolidation) {
                    (
                        ConsolidationState::Completed { .. },
                        ConsolidationState::Committed { .. },
                    ) => completed_promotions.push((OrganismId(raw), sleep_after)),
                    (
                        ConsolidationState::Submitted { .. },
                        ConsolidationState::Completed { .. },
                    ) => persist_exact_sleep_boundary = true,
                    (ConsolidationState::None, ConsolidationState::Pending { .. })
                    | (ConsolidationState::Pending { .. }, ConsolidationState::Prepared { .. })
                    | (ConsolidationState::Prepared { .. }, ConsolidationState::Submitted { .. })
                    | (
                        ConsolidationState::Committed { .. },
                        ConsolidationState::Committed { .. },
                    )
                    | (ConsolidationState::Committed { .. }, ConsolidationState::None) => {
                        let refresh_authority = matches!(
                            (sleep_before.consolidation, sleep_after.consolidation),
                            (ConsolidationState::None, ConsolidationState::Pending { .. })
                        );
                        if refresh_authority && !checkpoint_active {
                            sleep_journal_neural_authority_updates.insert(
                                raw,
                                capture_sleep_journal_neural_authority(&mut self.backend, handle)?,
                            );
                        } else if !checkpoint_active {
                            if let Some(expected) = sleep_journal_neural_authority_updates
                                .get(&raw)
                                .or_else(|| self.sleep_journal_neural_authorities.get(&raw))
                            {
                                validate_sleep_journal_neural_authority(
                                    &mut self.backend,
                                    handle,
                                    expected,
                                )?;
                            }
                        }
                        if matches!(
                            (sleep_before.consolidation, sleep_after.consolidation),
                            (ConsolidationState::None, ConsolidationState::Pending { .. })
                        ) && sleep_before.phase != SleepPhase::Consolidating
                            && sleep_after.phase == SleepPhase::Consolidating
                        {
                            let intermediate = SleepState {
                                consolidation: ConsolidationState::None,
                                ..sleep_after
                            };
                            sleep_journal_entries.push(
                                GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
                                    OrganismId(raw),
                                    tick_after,
                                    0,
                                    sleep_before,
                                    intermediate,
                                )?,
                            );
                            sleep_journal_entries.push(
                                GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
                                    OrganismId(raw),
                                    tick_after,
                                    1,
                                    intermediate,
                                    sleep_after,
                                )?,
                            );
                        } else {
                            sleep_journal_entries.push(GpuSleepTransactionJournalEntryV2::try_new(
                                OrganismId(raw),
                                tick_after,
                                sleep_before,
                                sleep_after,
                            )?);
                        }
                    }
                    (ConsolidationState::None, ConsolidationState::None) => {
                        if sleep_before.phase == SleepPhase::Awake && !checkpoint_active {
                            sleep_journal_neural_authority_updates.insert(
                                raw,
                                capture_sleep_journal_neural_authority(&mut self.backend, handle)?,
                            );
                        } else if !checkpoint_active {
                            if let Some(expected) = sleep_journal_neural_authority_updates
                                .get(&raw)
                                .or_else(|| self.sleep_journal_neural_authorities.get(&raw))
                            {
                                validate_sleep_journal_neural_authority(
                                    &mut self.backend,
                                    handle,
                                    expected,
                                )?;
                            }
                        }
                        sleep_journal_entries.push(GpuSleepTransactionJournalEntryV2::try_new(
                            OrganismId(raw),
                            tick_after,
                            sleep_before,
                            sleep_after,
                        )?);
                    }
                    _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into()),
                }
            }
            checkpoint_publication_wall_ns = checkpoint_publication_wall_ns
                .saturating_add(checkpoint_preparation_started.map_or(0, elapsed_ns));
            let remains_dispatchable = phase_before == SleepPhase::Awake
                && sleep_event.phase == SleepPhase::Awake
                && sleep_event.transition.is_none();
            if !remains_dispatchable || retained_learning_pending {
                summaries_by_organism.insert(
                    raw,
                    if retained_learning_pending && sleep_event.phase == SleepPhase::Awake {
                        Self::retained_learning_summary(
                            OrganismId(raw),
                            tick_before,
                            tick_after,
                            self.sealed_patch_count,
                        )
                    } else {
                        Self::sleeping_tick_summary(
                            OrganismId(raw),
                            tick_before,
                            tick_after,
                            self.sealed_patch_count,
                        )
                    },
                );
                continue;
            }
            #[cfg(feature = "gpu-tests")]
            let force_preparation_failure = self.forced_memory_preparation_failures.remove(&raw);
            #[cfg(not(feature = "gpu-tests"))]
            let force_preparation_failure = false;
            let preparation = (|| -> Result<PreparedGpuBrainFrame, ScaffoldContractError> {
                if force_preparation_failure {
                    return Err(ScaffoldContractError::InvalidMemoryQuery);
                }
                let grounded_perception_started = measure_preparation.then(Instant::now);
                let organism = self
                    .world
                    .organism_registry()
                    .get(OrganismId(raw))
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let neural_receptors = organism
                    .biochemistry()
                    .neural_receptor_frame(organism.phenotype())?;
                if neural_receptors.source_tick != tick_before {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence);
                }
                let receptor_phenotype = NeuralReceptorPhenotype::compile(&resident.phenotype)?;
                let receptor_effects =
                    NeuralReceptorEffects::from_frame(&neural_receptors, &receptor_phenotype)?;
                let draft = self.world.perception_frame_draft_indexed(
                    OrganismId(raw),
                    tick_before,
                    self.sensor_profile,
                    resident.homeostasis,
                    &perception_index,
                )?;
                grounded_perception_wall_ns = grounded_perception_wall_ns
                    .saturating_add(grounded_perception_started.map_or(0, elapsed_ns));
                let episodic_retrieval_started = measure_preparation.then(Instant::now);
                let memory = self
                    .memories
                    .get(&raw)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let topology = self
                    .topologies
                    .get(&raw)
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let sequence_id = ExperienceSequenceId(resident.next_sequence);
                sequence_id.validate()?;
                let prepared_recall = memory.recall_frame(&draft)?;
                let baseline_context = cognitive_context_for_recall(
                    OrganismId(raw),
                    sequence_id,
                    &prepared_recall,
                    topology,
                )?;
                let baseline_prepared = prepared_recall
                    .clone()
                    .with_cognitive_context(baseline_context.clone())?;
                let (baseline_frame, baseline_recall) =
                    baseline_prepared.finalize(draft.clone())?;
                baseline_recall.validate_for_frame(&baseline_frame)?;
                let memory_evidence = finalized_memory_attention_evidence(&baseline_recall)?;
                episodic_retrieval_wall_ns = episodic_retrieval_wall_ns
                    .saturating_add(episodic_retrieval_started.map_or(0, elapsed_ns));
                let attention_context_started = measure_preparation.then(Instant::now);
                let mut peripheral_summaries =
                    grounded_peripheral_summaries(draft.grounded_object_slots())?;
                let body_need = resident
                    .homeostasis
                    .drives
                    .to_array()
                    .iter()
                    .copied()
                    .fold(0.0, f32::max);
                apply_predecision_attention_evidence(
                    &mut peripheral_summaries,
                    body_need,
                    &memory_evidence,
                    &baseline_context,
                    receptor_effects,
                )?;
                let attention = select_focal_targets(
                    OrganismId(raw),
                    sequence_id,
                    tick_before,
                    &peripheral_summaries,
                    resident.attention_hysteresis,
                    attention_selection_policy_for(&resident.phenotype),
                )?;
                resident.attention_hysteresis = attention.hysteresis;
                let routed_draft = route_focal_candidates(draft, &attention)?;
                attention_context_wall_ns = attention_context_wall_ns
                    .saturating_add(attention_context_started.map_or(0, elapsed_ns));
                let topology_concept_started = measure_preparation.then(Instant::now);
                let routed_recall = memory.recall_frame(&routed_draft)?;
                let cognitive_context = cognitive_context_for_recall(
                    OrganismId(raw),
                    sequence_id,
                    &routed_recall,
                    topology,
                )?;
                let cognitive_context =
                    cognitive_context_with_attention(cognitive_context, attention)?;
                let prepared_recall = routed_recall.with_cognitive_context(cognitive_context)?;
                let (frame, memory_recall) = prepared_recall.finalize(routed_draft)?;
                memory_recall.validate_for_frame(&frame)?;
                topology_concept_wall_ns = topology_concept_wall_ns
                    .saturating_add(topology_concept_started.map_or(0, elapsed_ns));
                let gpu_upload_started = measure_preparation.then(Instant::now);
                let memory_upload = self
                    .backend
                    .prepare_memory_context_upload(handle, &frame, &memory_recall)?
                    .bind_neural_receptor_effects(receptor_effects)
                    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                gpu_upload_wall_ns =
                    gpu_upload_wall_ns.saturating_add(gpu_upload_started.map_or(0, elapsed_ns));
                Ok(PreparedGpuBrainFrame {
                    handle,
                    world_entity_id,
                    frame,
                    memory_recall,
                    memory_upload,
                    neural_receptors,
                    receptor_effects,
                })
            })();
            match preparation {
                Ok(prepared) => batch.push(prepared),
                Err(error) => {
                    self.last_memory_preparation_errors
                        .push((OrganismId(raw), error));
                    summaries_by_organism.insert(
                        raw,
                        Self::preparation_failure_summary(
                            OrganismId(raw),
                            tick_before,
                            tick_after,
                            self.sealed_patch_count,
                        ),
                    );
                }
            }
        }
        self.performance_metrics
            .perception_sleep_preparation_wall_ns = self
            .performance_metrics
            .perception_sleep_preparation_wall_ns
            .saturating_add(elapsed_ns(preparation_started));
        self.performance_metrics
            .preparation_sleep_eligibility_replay_wall_ns = self
            .performance_metrics
            .preparation_sleep_eligibility_replay_wall_ns
            .saturating_add(sleep_eligibility_replay_wall_ns);
        self.performance_metrics
            .preparation_sleep_phase_data_wall_ns = self
            .performance_metrics
            .preparation_sleep_phase_data_wall_ns
            .saturating_add(sleep_timing.phase_data_wall_ns);
        self.performance_metrics
            .preparation_sleep_replay_progress_wall_ns = self
            .performance_metrics
            .preparation_sleep_replay_progress_wall_ns
            .saturating_add(sleep_timing.replay_progress_wall_ns);
        self.performance_metrics
            .preparation_sleep_consolidation_wall_ns = self
            .performance_metrics
            .preparation_sleep_consolidation_wall_ns
            .saturating_add(sleep_timing.consolidation_wall_ns);
        self.performance_metrics
            .preparation_grounded_perception_wall_ns = self
            .performance_metrics
            .preparation_grounded_perception_wall_ns
            .saturating_add(grounded_perception_wall_ns);
        self.performance_metrics
            .preparation_episodic_retrieval_wall_ns = self
            .performance_metrics
            .preparation_episodic_retrieval_wall_ns
            .saturating_add(episodic_retrieval_wall_ns);
        self.performance_metrics
            .preparation_attention_context_wall_ns = self
            .performance_metrics
            .preparation_attention_context_wall_ns
            .saturating_add(attention_context_wall_ns);
        self.performance_metrics
            .preparation_topology_concept_wall_ns = self
            .performance_metrics
            .preparation_topology_concept_wall_ns
            .saturating_add(topology_concept_wall_ns);
        self.performance_metrics.preparation_gpu_upload_wall_ns = self
            .performance_metrics
            .preparation_gpu_upload_wall_ns
            .saturating_add(gpu_upload_wall_ns);
        self.performance_metrics
            .preparation_checkpoint_publication_wall_ns = self
            .performance_metrics
            .preparation_checkpoint_publication_wall_ns
            .saturating_add(checkpoint_publication_wall_ns);

        // The exact worker must receive every journal consequence from this
        // canonical tick before Completed promotion grants it permission to
        // finalize. Entries newer than the captured tick are consumed by the
        // coordinator's single bounded follow-up checkpoint request.
        if !completed_promotions.is_empty()
            && self.exact_checkpoint_accepts_journal_entries()
            && !sleep_journal_entries.is_empty()
        {
            self.queue_exact_checkpoint_journal_entries(sleep_journal_entries.clone())?;
            sleep_journal_entries.clear();
        }

        // The GPU selector has already committed, while the world is still at
        // the exact tick named by the durable Completed checkpoint. Publish
        // the manifest-side selector/ref promotion before any world action or
        // subsequent poll can occur.
        let sleep_promotion_started = Instant::now();
        let mut memory_commits = Vec::with_capacity(completed_promotions.len());
        for (organism_id, committed_sleep) in &completed_promotions {
            let committed_cycle_id = match committed_sleep.consolidation {
                ConsolidationState::Committed { cycle_id, .. } if cycle_id != 0 => cycle_id,
                _ => return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into()),
            };
            let (memory, receipt) = prepared_memory_commits
                .remove(&organism_id.raw())
                .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
            if receipt.identity.organism_id_raw != organism_id.raw()
                || receipt.identity.cycle_id != committed_cycle_id
            {
                return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
            }
            memory_commits.push((*organism_id, memory, receipt));
        }
        if let Err(error) = self.promote_durable_completed_sleep_batch(&completed_promotions) {
            // The backend's Completed -> Committed transaction precedes the
            // manifest CAS. The staged tick wrapper restores world and host
            // sleep authority to Completed; GPU authority must fail-stop
            // because that committed device state cannot be rolled back.
            self.backend
                .fail_stop(GpuSessionFailStopCause::CheckpointRestoreFailed);
            return Err(error);
        }
        self.post_irreversible_gpu_commit_fail_stop_armed |= !completed_promotions.is_empty();
        for (organism_id, memory, receipt) in memory_commits {
            let previous = self.memories.insert(organism_id.raw(), memory);
            debug_assert!(previous.is_some());
            self.last_memory_compaction_receipts.push(receipt);
            self.restored_replay_patches
                .retain(|patch| patch.header().organism_id != organism_id);
        }
        self.performance_metrics.sleep_promotion_wall_ns = self
            .performance_metrics
            .sleep_promotion_wall_ns
            .saturating_add(elapsed_ns(sleep_promotion_started));

        let awake_summaries = if batch.is_empty() {
            self.record_gpu_tick_metrics(&[])?;
            Vec::new()
        } else {
            let memory_inputs = batch
                .iter()
                .map(|prepared| {
                    GpuClosedLoopMemoryTickInput::try_new(
                        prepared.handle,
                        &prepared.frame,
                        &prepared.memory_upload,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let memory_batch = GpuClosedLoopMemoryBatchInput::try_new(memory_inputs)?;
            let inference_rows = u64::try_from(batch.len()).unwrap_or(u64::MAX);
            let inference_started = Instant::now();
            let gpu_ticks = self.backend.tick_memory_batch(&memory_batch)?;
            self.performance_metrics.inference_batches =
                self.performance_metrics.inference_batches.saturating_add(1);
            self.performance_metrics.inference_rows = self
                .performance_metrics
                .inference_rows
                .saturating_add(inference_rows);
            self.performance_metrics.inference_transaction_wall_ns = self
                .performance_metrics
                .inference_transaction_wall_ns
                .saturating_add(
                    u64::try_from(inference_started.elapsed().as_nanos()).unwrap_or(u64::MAX),
                );
            if gpu_ticks.len() != batch.len() {
                return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
            }
            self.record_gpu_tick_metrics(&gpu_ticks)?;
            let rows = batch.into_iter().zip(gpu_ticks).collect();
            self.process_selection_batch_in_staged_tick(rows)?
        };
        for summary in awake_summaries {
            summaries_by_organism.insert(summary.organism_id.raw(), summary);
        }
        let expected_summary_count = if curated_first_tick {
            1
        } else {
            self.handles.len()
        };
        if summaries_by_organism.len() != expected_summary_count {
            return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
        }
        #[cfg(any(test, feature = "gpu-tests"))]
        if std::mem::take(&mut self.forced_late_advance_failure) {
            return Err(ScaffoldContractError::NonMonotonicTick.into());
        }
        let authority_timing = advance_and_synchronize_authority(
            &mut self.world,
            &mut self.residents,
            tick_after,
            &scheduled_body_events,
        )?;
        self.performance_metrics.world_authority_advance_wall_ns = self
            .performance_metrics
            .world_authority_advance_wall_ns
            .saturating_add(authority_timing.world_advance_ns);
        self.performance_metrics.resident_synchronize_wall_ns = self
            .performance_metrics
            .resident_synchronize_wall_ns
            .saturating_add(authority_timing.resident_synchronize_ns);
        let passive_observation_started = Instant::now();
        self.observe_passive_tick(tick_before, tick_after)?;
        self.performance_metrics.passive_observation_wall_ns = self
            .performance_metrics
            .passive_observation_wall_ns
            .saturating_add(elapsed_ns(passive_observation_started));
        let population_reconcile_started = Instant::now();
        self.reconcile_population()?;
        self.performance_metrics.population_reconcile_wall_ns = self
            .performance_metrics
            .population_reconcile_wall_ns
            .saturating_add(elapsed_ns(population_reconcile_started));
        if persist_exact_sleep_boundary {
            let sleep_persistence_started = Instant::now();
            self.request_exact_population_checkpoint()?;
            if self.exact_checkpoint_accepts_journal_entries() && !sleep_journal_entries.is_empty()
            {
                self.queue_exact_checkpoint_journal_entries(sleep_journal_entries)?;
            } else if !sleep_journal_entries.is_empty() {
                self.sleep_journal_neural_authorities
                    .extend(sleep_journal_neural_authority_updates);
                let enqueue_started = self.performance_measurement_enabled.then(Instant::now);
                self.start_sleep_journal_publication(sleep_journal_entries)?;
                self.performance_metrics
                    .sleep_journal_update_thread_enqueue_wall_ns = self
                    .performance_metrics
                    .sleep_journal_update_thread_enqueue_wall_ns
                    .saturating_add(enqueue_started.map_or(0, elapsed_ns));
            }
            self.performance_metrics.sleep_persistence_wall_ns = self
                .performance_metrics
                .sleep_persistence_wall_ns
                .saturating_add(elapsed_ns(sleep_persistence_started));
        } else if !sleep_journal_entries.is_empty() {
            if self.exact_checkpoint_accepts_journal_entries() {
                self.queue_exact_checkpoint_journal_entries(sleep_journal_entries)?;
                return Ok(summaries_by_organism.into_values().collect());
            }
            let sleep_persistence_started = Instant::now();
            let durability = self.checkpoint_durability.take();
            let validation_result = (|| -> Result<(), GameAppShellError> {
                for entry in &sleep_journal_entries {
                    let raw = entry.organism_id.raw();
                    if sleep_journal_neural_authority_updates.contains_key(&raw)
                        || self.sleep_journal_neural_authorities.contains_key(&raw)
                    {
                        continue;
                    }
                    let durability = durability
                        .as_ref()
                        .ok_or(ScaffoldContractError::MissingPhaseData)?;
                    let handle = *self
                        .handles
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    let resident = self
                        .residents
                        .get(&raw)
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    let exact_base = durability
                        .published
                        .save
                        .creatures
                        .iter()
                        .find(|creature| creature.organism_id == entry.organism_id)
                        .and_then(|creature| creature.gpu_brain.as_ref())
                        .ok_or(ScaffoldContractError::ConsolidationGenerationMismatch)?;
                    durability.store.validate_compact_neural_reuse(
                        &mut self.backend,
                        &durability.published.save.assets,
                        exact_base,
                        handle,
                        &resident.phenotype,
                    )?;
                    sleep_journal_neural_authority_updates.insert(
                        raw,
                        capture_sleep_journal_neural_authority(&mut self.backend, handle)?,
                    );
                }
                Ok(())
            })();
            if let Err(error) = validation_result {
                self.checkpoint_durability = durability;
                return Err(error);
            }
            self.performance_metrics.sleep_persistence_calls = self
                .performance_metrics
                .sleep_persistence_calls
                .saturating_add(1);
            self.checkpoint_durability = durability;
            self.sleep_journal_neural_authorities
                .extend(sleep_journal_neural_authority_updates);
            let enqueue_started = self.performance_measurement_enabled.then(Instant::now);
            self.start_sleep_journal_publication(sleep_journal_entries)?;
            self.performance_metrics
                .sleep_journal_update_thread_enqueue_wall_ns = self
                .performance_metrics
                .sleep_journal_update_thread_enqueue_wall_ns
                .saturating_add(enqueue_started.map_or(0, elapsed_ns));
            self.performance_metrics.sleep_persistence_wall_ns = self
                .performance_metrics
                .sleep_persistence_wall_ns
                .saturating_add(elapsed_ns(sleep_persistence_started));
        }
        Ok(summaries_by_organism.into_values().collect())
    }
}
