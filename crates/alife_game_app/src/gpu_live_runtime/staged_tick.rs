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
        #[cfg(feature = "foundation-training")]
        self.last_foundation_training_steps.clear();
        #[cfg(feature = "foundation-training")]
        self.last_foundation_terminal_biology.clear();
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
        if let Some(prior) = self.semantic_prior.as_mut() {
            prior.retain_lives(&self.handles);
        }
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
            if matches!(
                self.exact_checkpoint_work,
                ExactPopulationCheckpointRuntimeWorkV1::AwaitingJournal { .. }
            ) {
                self.finalize_awaiting_exact_checkpoint(&[])?;
            }
            // Retirement has already archived and removed the final resident.
            // An empty habitat still advances ecology without neural dispatch.
            self.world.try_advance_tick()?;
            return Ok(Vec::new());
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
        let mut journaled_recovery_edges = Vec::new();
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
        let workers = preparation_worker_count(
            scheduled_handles.len(),
            thread::available_parallelism().map_or(1, usize::from),
        );
        let mut captured_rows = Vec::with_capacity(if workers == 2 {
            scheduled_handles.len()
        } else {
            0
        });
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
        let mut cpu_timing = cpu_preparation::CpuPreparationTiming::default();
        let mut checkpoint_publication_wall_ns = 0_u64;
        for (raw, handle, world_entity_id) in scheduled_handles {
            let sleep_preparation_started = measure_preparation.then(Instant::now);
            let retained_learning_pending =
                self.retry_retained_learning(OrganismId(raw), tick_before)?;
            let recovery_edge = self.pending_recovery_sleep_edges.get(&raw).copied();
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
            match brain_atp_world_tick_mode(
                phase_before,
                self.schedule_sleep,
                completed_waiting_for_durable_permit,
            ) {
                BrainAtpWorldTickMode::Charge { recover } => {
                    if self.schedule_sleep {
                        // Chemistry owns availability. The backend projects it
                        // into a tick-bound budget; measured cognitive work is
                        // charged against canonical body reserve separately.
                        self.backend.bind_world_brain_atp_tick(
                            handle,
                            tick_before.raw(),
                            record.biochemistry().homeostasis.drives.brain_atp,
                        )?;
                    } else {
                        // Explicit continuous-wake lab protocols retain the
                        // old budget policy and sleep-rate recovery so bounded
                        // neural measurements are not truncated by exhaustion.
                        self.backend.charge_world_brain_atp_tick(
                            handle,
                            tick_before.raw(),
                            recover,
                        )?;
                    }
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
                        fail_stop_armed: Some(
                            &mut self.post_irreversible_gpu_commit_fail_stop_armed,
                        ),
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
            let journal_start = sleep_journal_entries.len();
            if let Some(edge) = recovery_edge {
                if edge.target != sleep_before {
                    return Err(ScaffoldContractError::ConsolidationGenerationMismatch.into());
                }
                sleep_journal_entries.push(GpuSleepTransactionJournalEntryV2::try_new(
                    OrganismId(raw),
                    tick_after,
                    edge.source,
                    edge.target,
                )?);
                if !checkpoint_active {
                    sleep_journal_neural_authority_updates.insert(
                        raw,
                        capture_sleep_journal_neural_authority(&mut self.backend, handle)?,
                    );
                }
                journaled_recovery_edges.push(raw);
            }
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
            if recovery_edge.is_some() {
                for (ordinal, entry) in sleep_journal_entries[journal_start..]
                    .iter_mut()
                    .enumerate()
                {
                    *entry = GpuSleepTransactionJournalEntryV2::try_new_with_ordinal(
                        entry.organism_id,
                        entry.transition_tick,
                        u8::try_from(ordinal)
                            .map_err(|_| ScaffoldContractError::ConsolidationGenerationMismatch)?,
                        entry.source,
                        entry.target,
                    )?;
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
            let mut preparation_stage = "receptors";
            let preparation = (|| -> Result<CapturedLivePreparation, ScaffoldContractError> {
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
                preparation_stage = "grounded draft";
                let draft = self.world.perception_frame_draft_indexed(
                    OrganismId(raw),
                    tick_before,
                    self.sensor_profile,
                    resident.homeostasis,
                    &perception_index,
                )?;
                // This is the owner's hint snapshot boundary for this attempt.
                // Poll once in roster order, freeze the returned context in the
                // owned draft, and leave replies arriving after this point for
                // a later preparation. Workers never receive the prior actor.
                let draft = if let Some(prior) = self.semantic_prior.as_mut() {
                    prior.prepare(draft, ExperienceSequenceId(resident.next_sequence))?
                } else {
                    draft
                };
                grounded_perception_wall_ns = grounded_perception_wall_ns
                    .saturating_add(grounded_perception_started.map_or(0, elapsed_ns));
                Ok(CapturedLivePreparation {
                    handle,
                    world_entity_id,
                    input: CapturedCpuPreparation {
                        draft,
                        sequence_id: ExperienceSequenceId(resident.next_sequence),
                        homeostasis: resident.homeostasis,
                        hysteresis: resident.attention_hysteresis,
                        policy: attention_selection_policy_for(&resident.phenotype),
                        neural_receptors,
                        receptor_effects,
                    },
                })
            })();
            match preparation {
                Ok(captured) => {
                    if workers == 1 {
                        let outcomes = self.prepare_captured_cpu_rows(
                            std::slice::from_ref(&captured),
                            1,
                            measure_preparation,
                        )?;
                        let outcome = outcomes
                            .into_iter()
                            .next()
                            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                        collect_preparation(
                            self,
                            captured,
                            outcome,
                            &mut CpuPreparationCollection {
                                batch: &mut batch,
                                summaries: &mut summaries_by_organism,
                                tick_before,
                                tick_after,
                                timing: &mut cpu_timing,
                            },
                        )?;
                    } else {
                        captured_rows.push(captured);
                    }
                }
                Err(error) => {
                    if std::env::var_os("ALIFE_FOUNDATION_PROFILE").is_some() {
                        eprintln!("foundation perception preparation failed at {tick_before:?} in {preparation_stage}: {error:?}");
                    }
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
        if !captured_rows.is_empty() {
            let outcomes = self.prepare_captured_cpu_rows(
                &captured_rows,
                preparation_worker_count(captured_rows.len(), workers),
                measure_preparation,
            )?;
            if outcomes.len() != captured_rows.len() {
                return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
            }
            for (captured, outcome) in captured_rows.into_iter().zip(outcomes) {
                collect_preparation(
                    self,
                    captured,
                    outcome,
                    &mut CpuPreparationCollection {
                        batch: &mut batch,
                        summaries: &mut summaries_by_organism,
                        tick_before,
                        tick_after,
                        timing: &mut cpu_timing,
                    },
                )?;
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
            .saturating_add(cpu_timing.episodic_retrieval_wall_ns);
        self.performance_metrics
            .preparation_attention_context_wall_ns = self
            .performance_metrics
            .preparation_attention_context_wall_ns
            .saturating_add(cpu_timing.attention_context_wall_ns);
        self.performance_metrics
            .preparation_topology_concept_wall_ns = self
            .performance_metrics
            .preparation_topology_concept_wall_ns
            .saturating_add(cpu_timing.topology_concept_wall_ns);
        self.performance_metrics.preparation_gpu_upload_wall_ns = self
            .performance_metrics
            .preparation_gpu_upload_wall_ns
            .saturating_add(cpu_timing.gpu_upload_wall_ns);
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
            journaled_recovery_edges.clear();
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
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| GameAppShellError::InvalidProductionFrontend {
                    message: format!("neural input preparation failed: {error}"),
                })?;
            let memory_batch = GpuClosedLoopMemoryBatchInput::try_new(memory_inputs)?;
            let inference_rows = u64::try_from(batch.len()).unwrap_or(u64::MAX);
            #[cfg(feature = "foundation-training")]
            let before_training = if self.training_sampling.is_some() {
                batch
                    .iter()
                    .map(|prepared| {
                        self.backend
                            .capture_training_state(prepared.handle, prepared.frame.tick())
                    })
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            };
            let inference_started = Instant::now();
            #[cfg(all(test, feature = "gpu-tests", not(feature = "foundation-training")))]
            let gpu_ticks = super::action_credit_food_tests::tick_with_selector_capture(
                &mut self.backend,
                &memory_batch,
                &batch,
            )?;
            #[cfg(all(
                not(all(test, feature = "gpu-tests")),
                not(feature = "foundation-training")
            ))]
            let gpu_ticks = self
                .backend
                .tick_memory_batch(&memory_batch)
                .map_err(|error| GameAppShellError::InvalidProductionFrontend {
                    message: format!("neural execution failed: {error}"),
                })?;
            #[cfg(feature = "foundation-training")]
            let gpu_ticks = if let Some(sampling) = self.training_sampling {
                let count = u32::try_from(batch.len())
                    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                let next_counter = sampling
                    .counter
                    .checked_add(count)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let mut configs = Vec::with_capacity(batch.len());
                for (index, prepared) in batch.iter().enumerate() {
                    let demonstrator = match self.training_demonstrator.as_mut() {
                        Some(teacher) => Some(teacher(
                            &prepared.frame,
                            self.backend
                                .training_enabled_motor_channels(prepared.handle)?,
                        )?),
                        None => sampling.demonstrator,
                    };
                    configs.push(alife_gpu_backend::GpuTrainingSamplingConfig {
                        counter: sampling.counter + index as u32,
                        demonstrator,
                        ..sampling
                    });
                }
                self.training_sampling
                    .as_mut()
                    .expect("training enabled")
                    .counter = next_counter;
                self.backend
                    .tick_memory_batch_training(&memory_batch, &configs)?
            } else {
                self.backend.tick_memory_batch(&memory_batch)?
            };
            #[cfg(feature = "foundation-training")]
            let mut training_rows = Vec::new();
            #[cfg(feature = "foundation-training")]
            for ((prepared, gpu_tick), before) in batch.iter().zip(&gpu_ticks).zip(before_training)
            {
                let behavior = gpu_tick
                    .training_rollout
                    .clone()
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let after = self
                    .backend
                    .capture_training_state(prepared.handle, prepared.frame.tick())?;
                training_rows.push((
                    prepared.frame.clone(),
                    prepared.memory_upload.clone(),
                    before,
                    after,
                    behavior,
                    gpu_tick.work.clone(),
                    gpu_tick.throttle.clone(),
                ));
            }
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
            let summaries = self
                .process_selection_batch_in_staged_tick(rows)
                .map_err(|error| GameAppShellError::InvalidProductionFrontend {
                    message: format!("selected action processing failed: {error}"),
                })?;
            #[cfg(feature = "foundation-training")]
            for (frame, memory_upload, before, after_inference, behavior, work, throttle) in
                training_rows
            {
                let patch = self
                    .last_sealed_patches
                    .iter()
                    .find(|patch| {
                        patch.header().organism_id == frame.organism_id()
                            && patch.outcome().outcome_tick == tick_after
                    })
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?
                    .clone();
                if !self.last_learning_receipts.iter().any(|receipt| {
                    receipt.handle.organism_id() == frame.organism_id()
                        && receipt.dispatch_generation == behavior.dispatch_generation
                        && receipt.sequence_id == patch.header().sequence_id
                }) || before.active_weight_generation != behavior.active_weight_generation
                    || after_inference.active_weight_generation != behavior.active_weight_generation
                    || after_inference.logical_dispatch_generation != behavior.dispatch_generation
                {
                    return Err(ScaffoldContractError::InvalidDecisionEvidence.into());
                }
                self.last_foundation_training_steps
                    .push(FoundationTrainingStep {
                        outcome_credit: alife_core::OutcomeCreditPacket::from_sealed_patch(&patch)?,
                        frame,
                        memory_upload,
                        before,
                        after_inference,
                        behavior,
                        work,
                        throttle,
                        patch,
                    });
            }
            summaries
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
        // Reconciliation archives deaths and removes their GPU handles. Their
        // final summaries and sleep transitions must not enter the live frame
        // or attempt a later checkpoint lookup through a retired handle.
        summaries_by_organism.retain(|raw, _| self.handles.contains_key(raw));
        sleep_journal_entries.retain(|entry| self.handles.contains_key(&entry.organism_id.raw()));
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
                journaled_recovery_edges.clear();
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
        // A failed tick or enqueue must retain the earliest unjournaled source.
        for raw in journaled_recovery_edges {
            self.pending_recovery_sleep_edges.remove(&raw);
        }
        Ok(summaries_by_organism.into_values().collect())
    }
}

impl GpuLiveBrainRuntime {
    fn prepare_captured_cpu_rows(
        &mut self,
        captured: &[CapturedLivePreparation],
        workers: usize,
        measure: bool,
    ) -> Result<Vec<CpuPreparationOutcome>, ScaffoldContractError> {
        let slots = captured
            .iter()
            .map(|row| {
                self.backend.memory_context_slot(
                    row.handle,
                    row.input.draft.organism_id(),
                    row.input.draft.sensor_profile(),
                )
            })
            .collect::<Vec<_>>();
        let jobs = captured
            .iter()
            .zip(&slots)
            .map(|(row, slot)| {
                let raw = row.input.draft.organism_id().raw();
                CpuPreparationJob {
                    input: &row.input,
                    slot: slot.as_ref().ok(),
                    memory: self.memories.get(&raw),
                    topology: self.topologies.get(&raw),
                    predictor: self.residents.get(&raw).map(|resident| &resident.predictor),
                }
            })
            .collect::<Vec<_>>();
        // Existing receipt leaves are additive serial intervals. Parallel row
        // intervals overlap, so leave that suffix in the enclosing wall residual
        // rather than subtracting a worker-duration sum from elapsed wall time.
        let mut outcomes = prepare_cpu_rows(&jobs, workers, measure && workers == 1)?;
        drop(jobs);
        for ((row, slot), outcome) in captured.iter().zip(slots).zip(&mut outcomes) {
            match slot {
                Err(error) if outcome.stage == "GPU memory upload" => outcome.result = Err(error),
                Ok(slot) => {
                    if let Ok(prepared) = &outcome.result {
                        let live = self.backend.memory_context_slot(
                            row.handle,
                            prepared.frame.organism_id(),
                            prepared.frame.sensor_profile(),
                        );
                        if let Err(error) = readmit_cpu_slot(&slot, live) {
                            outcome.stage = "GPU memory re-admission";
                            outcome.result = Err(error);
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(outcomes)
    }
}

fn readmit_cpu_slot(
    captured: &alife_gpu_backend::GpuBrainSlot,
    live: Result<alife_gpu_backend::GpuBrainSlot, ScaffoldContractError>,
) -> Result<(), ScaffoldContractError> {
    if *captured == live? {
        Ok(())
    } else {
        Err(ScaffoldContractError::BrainOwnershipMismatch)
    }
}

struct CpuPreparationCollection<'a> {
    batch: &'a mut Vec<PreparedGpuBrainFrame>,
    summaries: &'a mut BTreeMap<u64, LiveBrainTickSummary>,
    tick_before: Tick,
    tick_after: Tick,
    timing: &'a mut cpu_preparation::CpuPreparationTiming,
}

fn collect_preparation(
    runtime: &mut GpuLiveBrainRuntime,
    captured: CapturedLivePreparation,
    outcome: CpuPreparationOutcome,
    collection: &mut CpuPreparationCollection<'_>,
) -> Result<(), ScaffoldContractError> {
    let raw = captured.input.draft.organism_id().raw();
    if let Some(hysteresis) = outcome.hysteresis {
        runtime
            .residents
            .get_mut(&raw)
            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
            .attention_hysteresis = hysteresis;
    }
    collection.timing.episodic_retrieval_wall_ns = collection
        .timing
        .episodic_retrieval_wall_ns
        .saturating_add(outcome.timing.episodic_retrieval_wall_ns);
    collection.timing.attention_context_wall_ns = collection
        .timing
        .attention_context_wall_ns
        .saturating_add(outcome.timing.attention_context_wall_ns);
    collection.timing.topology_concept_wall_ns = collection
        .timing
        .topology_concept_wall_ns
        .saturating_add(outcome.timing.topology_concept_wall_ns);
    collection.timing.gpu_upload_wall_ns = collection
        .timing
        .gpu_upload_wall_ns
        .saturating_add(outcome.timing.gpu_upload_wall_ns);
    match outcome.result {
        Ok(prepared) => collection.batch.push(PreparedGpuBrainFrame {
            handle: captured.handle,
            world_entity_id: captured.world_entity_id,
            frame: prepared.frame,
            memory_recall: prepared.memory_recall,
            memory_upload: prepared.memory_upload,
            neural_receptors: captured.input.neural_receptors,
            receptor_effects: captured.input.receptor_effects,
        }),
        Err(error) => {
            if std::env::var_os("ALIFE_FOUNDATION_PROFILE").is_some() {
                eprintln!(
                    "foundation perception preparation failed at {:?} in {}: {error:?}",
                    collection.tick_before, outcome.stage
                );
            }
            runtime
                .last_memory_preparation_errors
                .push((OrganismId(raw), error));
            collection.summaries.insert(
                raw,
                GpuLiveBrainRuntime::preparation_failure_summary(
                    OrganismId(raw),
                    collection.tick_before,
                    collection.tick_after,
                    runtime.sealed_patch_count,
                ),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod cpu_admission_tests {
    use super::*;

    #[test]
    fn cpu_re_admission_rejects_generation_offset_owner_token_and_device_loss() {
        let organism = OrganismId(1);
        let mut world = alife_world::HeadlessScenarioBuilder::new(77112)
            .agent("agent", organism, Vec3f::ZERO)
            .build()
            .unwrap();
        super::super::tests::register_sealing_test_organism(&mut world, organism);
        let (phenotype, _) = GpuLiveBrainRuntime::compile_birth(
            &world,
            BrainScaleTier::Nano512,
            SensorProfile::GroundedObjectSlotsV1,
            organism,
        )
        .unwrap();
        let mut bucket =
            alife_gpu_backend::GpuClassBucketPlan::new(BrainCapacityClass::n512(), 2).unwrap();
        let captured = bucket.insert_phenotype(0, 1, &phenotype).unwrap();
        let different_offset = bucket.insert_phenotype(1, 1, &phenotype).unwrap();
        assert_eq!(readmit_cpu_slot(&captured, Ok(captured.clone())), Ok(()));
        assert_eq!(
            readmit_cpu_slot(&captured, Ok(different_offset)),
            Err(ScaffoldContractError::BrainOwnershipMismatch)
        );
        let mut other =
            alife_gpu_backend::GpuClassBucketPlan::new(BrainCapacityClass::n512(), 2).unwrap();
        let different_generation = other.insert_phenotype(0, 2, &phenotype).unwrap();
        assert_eq!(
            readmit_cpu_slot(&captured, Ok(different_generation)),
            Err(ScaffoldContractError::BrainOwnershipMismatch)
        );
        let mut other =
            alife_gpu_backend::GpuClassBucketPlan::new(BrainCapacityClass::n512(), 2).unwrap();
        let foreign_bucket = other.insert_phenotype(0, 1, &phenotype).unwrap();
        assert_eq!(captured.record(), foreign_bucket.record());
        assert_eq!(captured.word_ranges(), foreign_bucket.word_ranges());
        assert_ne!(captured, foreign_bucket);
        assert_eq!(
            readmit_cpu_slot(&captured, Ok(foreign_bucket)),
            Err(ScaffoldContractError::BrainOwnershipMismatch)
        );
        assert_eq!(
            readmit_cpu_slot(
                &captured,
                Err(ScaffoldContractError::NeuralBackendUnavailable)
            ),
            Err(ScaffoldContractError::NeuralBackendUnavailable)
        );
    }
}
