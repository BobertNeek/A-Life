//! Batched GPU tick preparation, submission, compact readback, and authoritative decode.

use super::*;

impl GpuClosedLoopBackend {
    pub(super) fn tick_inputs_with_selector_diagnostic_capture(
        &mut self,
        batch: &[GpuRuntimeTickInput<'_>],
        selector_diagnostic_candidate_indices: Option<&[u16]>,
        mut selector_diagnostic_error_capture: Option<&mut SelectorDiagnosticErrorCapture>,
    ) -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
        let capture_selector_diagnostics = selector_diagnostic_candidate_indices.is_some();
        self.ensure_ready()?;
        if batch.is_empty() {
            return Err(ScaffoldContractError::InvalidPerceptionFrame);
        }
        let mut seen_handles = BTreeSet::new();
        let mut seen_organisms = BTreeSet::new();
        let mut grouped = BTreeMap::<(u16, usize), Vec<usize>>::new();
        for (index, input) in batch.iter().enumerate() {
            let handle = input.handle;
            let frame = input.frame;
            self.validate_handle_backend(handle)?;
            if !seen_handles.insert((handle.class_id.raw(), handle.slot, handle.generation))
                || !seen_organisms.insert(handle.organism_id.0)
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            frame.validate()?;
            let pool = self
                .class_buckets
                .get(&handle.class_id.raw())
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let chunk_index = pool
                .bucket_index_for_handle(handle)
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let resident = pool.resident(handle)?;
            if resident.ownership.organism_id != frame.organism_id()
                || handle.organism_id != frame.organism_id()
            {
                return Err(ScaffoldContractError::BrainOwnershipMismatch);
            }
            if resident.ownership.sensor_profile != frame.sensor_profile() {
                return Err(ScaffoldContractError::SensorProfileMismatch);
            }
            if resident.pending_eligibility.is_some() {
                return Err(ScaffoldContractError::LearningReplayRejected);
            }
            if resident.activity_sequence_cursor.checked_add(1).is_none() {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
            grouped
                .entry((handle.class_id.raw(), chunk_index))
                .or_default()
                .push(index);
        }

        let dispatch_generation = NonZeroU64::new(self.next_dispatch_generation)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_dispatch_generation = self
            .next_dispatch_generation
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_upload_count = self
            .perception_upload_count
            .checked_add(batch.len() as u64)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_completed_dispatch_count = self
            .completed_dispatch_count
            .checked_add(1)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let next_completed_selection_count = self
            .completed_selection_count
            .checked_add(batch.len() as u64)
            .ok_or(ScaffoldContractError::NeuralBackendUnavailable)?;
        let replayed_pressure = if self.recorded_pressure_replay.is_empty() {
            None
        } else {
            if self.recorded_pressure_replay.len() < batch.len() {
                return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
            }
            Some(
                self.recorded_pressure_replay
                    .drain(..batch.len())
                    .collect::<Vec<_>>(),
            )
        };
        let mut replayed_pressure_iter = replayed_pressure.as_deref().map(<[_]>::iter);
        let activity_decisions = batch
            .iter()
            .map(|input| {
                let handle = input.handle;
                let resident = self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let identity = BrainDispatchIdentity {
                    organism_id_raw: handle.organism_id.raw(),
                    tick: input.frame.tick().raw(),
                    class_id_raw: handle.class_id.raw(),
                    handle_slot: handle.slot,
                    handle_generation: handle.generation,
                    sequence_cursor: resident.activity_sequence_cursor,
                    dispatch_generation: dispatch_generation.get(),
                    frame_digest: input.frame.frame_digest().0,
                };
                let pressure = match replayed_pressure_iter
                    .as_mut()
                    .and_then(|samples| samples.next())
                    .copied()
                {
                    Some(sample) => {
                        sample.validate_for(&self.activity_policy)?;
                        if sample.dispatch_identity() != identity
                            || sample.source_dispatch_generation
                                != resident.last_activity_dispatch_generation
                            || sample.source_frame_digest != resident.last_activity_frame_digest
                        {
                            return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                        }
                        sample
                    }
                    None => live_pressure_sample(
                        &self.activity_policy,
                        identity,
                        resident,
                        &self.admission,
                        &self.runtime_budget,
                    )?,
                };
                let capacity = capacity_for_gpu_class(handle.class_id)?;
                NeuralThrottleDecision::derive(
                    &self.activity_policy,
                    &resident.phenotype,
                    capacity.execution(),
                    identity,
                    pressure,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let first_class_id = batch[0].handle.class_id.raw();
        let timing_class_id = batch
            .iter()
            .all(|input| input.handle.class_id.raw() == first_class_id)
            .then_some(first_class_id);
        let mut dispatches = Vec::with_capacity(grouped.len());
        for ((class_id, chunk_index), original_indices) in grouped {
            let bucket = self
                .class_buckets
                .get(&class_id)
                .and_then(|pool| pool.chunks.get(chunk_index))
                .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
            let entries = original_indices
                .iter()
                .map(|index| {
                    let input = batch[*index];
                    let resident = bucket.slots[input.handle.slot as usize]
                        .as_ref()
                        .expect("complete preflight retained occupied slot");
                    match input.memory_upload {
                        Some(memory_upload) => GpuFixedActiveBatchEntry::with_memory(
                            input.frame,
                            &resident.brain_slot,
                            &resident.phenotype,
                            &activity_decisions[*index],
                            memory_upload,
                            resident.active_eligibility_generation,
                        ),
                        None => GpuFixedActiveBatchEntry::new(
                            input.frame,
                            &resident.brain_slot,
                            &resident.phenotype,
                            &activity_decisions[*index],
                            resident.active_eligibility_generation,
                        ),
                    }
                })
                .collect::<Vec<_>>();
            let prepared = bucket
                .pipelines
                .preflight_fixed_active_batch(&entries, 0, dispatch_generation)
                .map_err(map_gpu_contract_error)?;
            dispatches.push(PreparedClassDispatch {
                class_id,
                chunk_index,
                original_indices,
                prepared: Some(prepared),
                batch: None,
                recorded: false,
                map_ticket: None,
                selector_readback: None,
                selector_map_ticket: None,
                selector_captures: None,
                validated: None,
            });
        }

        for index in 0..dispatches.len() {
            let class_id = dispatches[index].class_id;
            let prepared = dispatches[index]
                .prepared
                .take()
                .expect("prepared exactly once");
            let result = self
                .class_buckets
                .get_mut(&class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatches[index].chunk_index))
                .expect("preflight bucket exists")
                .pipelines
                .begin_prepared_batch(prepared);
            match result {
                Ok(mut active) => {
                    if capture_selector_diagnostics {
                        let capacity = self
                            .class_buckets
                            .get(&class_id)
                            .and_then(|pool| pool.chunks.get(dispatches[index].chunk_index))
                            .expect("preflight bucket exists")
                            .buffers
                            .frame_payload_capacity_words();
                        let enable_result = active.enable_selector_diagnostics(
                            class_id,
                            dispatches[index].chunk_index,
                            selector_diagnostic_candidate_indices
                                .expect("capture flag follows requested candidates"),
                            capacity,
                        );
                        if let Err(error) = enable_result {
                            let translated_error =
                                translate_selector_diagnostic_enable_error(error);
                            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut()
                            {
                                capture.enable_error = Some(translated_error.clone());
                            }
                            if let Some(receipt) = error.receipt() {
                                eprintln!("gpu_selector_diagnostic_error_receipt: {receipt}");
                            }
                            let _ = self
                                .class_buckets
                                .get_mut(&class_id)
                                .and_then(|pool| pool.chunks.get_mut(dispatches[index].chunk_index))
                                .expect("preflight bucket exists")
                                .pipelines
                                .abandon_unsubmitted_batch(active);
                            for prior in &mut dispatches[..index] {
                                if let Some(active) = prior.batch.take() {
                                    let _ = self
                                        .class_buckets
                                        .get_mut(&prior.class_id)
                                        .and_then(|pool| pool.chunks.get_mut(prior.chunk_index))
                                        .expect("prior bucket exists")
                                        .pipelines
                                        .abandon_unsubmitted_batch(active);
                                }
                            }
                            return Err(map_gpu_contract_error(error.gpu_error()));
                        }
                    }
                    dispatches[index].batch = Some(active)
                }
                Err(error) => {
                    for prior in &mut dispatches[..index] {
                        if let Some(active) = prior.batch.take() {
                            let _ = self
                                .class_buckets
                                .get_mut(&prior.class_id)
                                .and_then(|pool| pool.chunks.get_mut(prior.chunk_index))
                                .expect("prior bucket exists")
                                .pipelines
                                .abandon_unsubmitted_batch(active);
                        }
                    }
                    return Err(map_gpu_contract_error(error));
                }
            }
        }

        if capture_selector_diagnostics {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.enable_completed = true;
            }
        }

        if capture_selector_diagnostics {
            for dispatch in &mut dispatches {
                let bytes = dispatch
                    .batch
                    .as_ref()
                    .expect("begun batch")
                    .selector_diagnostic_bytes();
                let bytes = match bytes {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                            capture.later_stage_receipt =
                                GpuRuntimeSelectorDiagnosticFailureReceipt::from_gpu_error(
                                    error,
                                    dispatch.class_id,
                                    dispatch.chunk_index,
                                );
                        }
                        return Err(map_gpu_contract_error(error));
                    }
                };
                dispatch.selector_readback =
                    Some(self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("closed-loop-selector-diagnostic-readback"),
                        size: bytes,
                        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                        mapped_at_creation: false,
                    }));
            }
        }

        for index in 0..dispatches.len() {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::WriteStagedUploads);
            }
            let dispatch = &dispatches[index];
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                .expect("prepared bucket exists");
            if let Err(error) = bucket.pipelines.write_staged_uploads(
                &self.queue,
                &bucket.buffers,
                dispatch.batch.as_ref().expect("begun batch"),
            ) {
                self.cleanup_unsubmitted_dispatches(&mut dispatches);
                return Err(map_gpu_contract_error(error));
            }
        }
        self.perception_upload_count = next_upload_count;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("closed-loop-runtime-mixed-class-tick"),
            });
        {
            let _timestamp_start = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-runtime-timestamp-start"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &self.timestamp_resources.query_set,
                    beginning_of_pass_write_index: Some(0),
                    end_of_pass_write_index: None,
                }),
            });
        }
        for index in 0..dispatches.len() {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::RecordDispatch);
            }
            let dispatch = &mut dispatches[index];
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                .expect("begun bucket exists");
            let result = match dispatch.selector_readback.as_ref() {
                Some(readback) => bucket
                    .pipelines
                    .record_staged_closed_loop_with_selector_diagnostics(
                        &mut encoder,
                        &bucket.buffers,
                        dispatch.batch.as_ref().expect("begun batch"),
                        readback,
                    ),
                None => bucket.pipelines.record_staged_closed_loop(
                    &mut encoder,
                    &bucket.buffers,
                    dispatch.batch.as_ref().expect("begun batch"),
                ),
            };
            if let Err(error) = result {
                self.cleanup_unsubmitted_dispatches(&mut dispatches);
                return Err(map_gpu_contract_error(error));
            }
            dispatch.recorded = true;
        }
        {
            let _timestamp_end = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("closed-loop-runtime-timestamp-end"),
                timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                    query_set: &self.timestamp_resources.query_set,
                    beginning_of_pass_write_index: None,
                    end_of_pass_write_index: Some(1),
                }),
            });
        }
        encoder.resolve_query_set(
            &self.timestamp_resources.query_set,
            0..GPU_TIMESTAMP_QUERY_COUNT,
            &self.timestamp_resources.resolve_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.timestamp_resources.resolve_buffer,
            0,
            &self.timestamp_resources.readback_buffer,
            0,
            GPU_TIMESTAMP_READBACK_BYTES,
        );
        let command_buffer = encoder.finish();
        for index in 0..dispatches.len() {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage =
                    Some(GpuRuntimeSelectorDiagnosticStage::RegisterCompactMapping);
            }
            let dispatch = &mut dispatches[index];
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                .expect("recorded bucket exists");
            match bucket.pipelines.register_compact_mapping(
                &command_buffer,
                &bucket.buffers,
                dispatch.batch.as_ref().expect("recorded batch"),
            ) {
                Ok(ticket) => dispatch.map_ticket = Some(ticket),
                Err(error) => {
                    self.cleanup_unsubmitted_dispatches(&mut dispatches);
                    return Err(map_gpu_contract_error(error));
                }
            }
            if let Some(readback) = dispatch.selector_readback.as_ref() {
                if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                    capture.later_stage =
                        Some(GpuRuntimeSelectorDiagnosticStage::RegisterSelectorDiagnosticMapping);
                }
                match bucket.pipelines.register_selector_diagnostic_mapping(
                    &command_buffer,
                    readback,
                    dispatch.batch.as_ref().expect("recorded batch"),
                ) {
                    Ok(ticket) => dispatch.selector_map_ticket = Some(ticket),
                    Err(error) => {
                        self.cleanup_unsubmitted_dispatches(&mut dispatches);
                        return Err(map_gpu_contract_error(error));
                    }
                }
            }
        }
        let (timestamp_sender, timestamp_receiver) = std::sync::mpsc::channel();
        command_buffer.map_buffer_on_submit(
            &self.timestamp_resources.readback_buffer,
            wgpu::MapMode::Read,
            0..GPU_TIMESTAMP_READBACK_BYTES,
            move |result| {
                let _ = timestamp_sender.send(result);
            },
        );
        let submission = self.queue.submit(Some(command_buffer));
        let forced_loss = std::mem::take(&mut self.force_device_lost_after_submit);
        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::DevicePoll);
        }
        let poll_failed = self
            .device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: None,
            })
            .is_err();
        let mappings_succeeded = dispatches.iter_mut().all(|dispatch| {
            dispatch
                .map_ticket
                .take()
                .is_some_and(GpuCompactMapTicket::mapping_succeeded)
        });
        let selector_mappings_succeeded = dispatches.iter_mut().all(|dispatch| {
            dispatch.selector_readback.is_none()
                || dispatch
                    .selector_map_ticket
                    .take()
                    .is_some_and(GpuCompactMapTicket::mapping_succeeded)
        });
        let timestamp_mapping_succeeded = timestamp_mapping_completed(&timestamp_receiver);
        let post_submit_failure_stage = if forced_loss {
            Some(GpuRuntimeSelectorDiagnosticStage::DeviceLostAfterSubmit)
        } else if poll_failed {
            Some(GpuRuntimeSelectorDiagnosticStage::DevicePoll)
        } else if !mappings_succeeded {
            Some(GpuRuntimeSelectorDiagnosticStage::CompactMappingCompletion)
        } else if !selector_mappings_succeeded {
            Some(GpuRuntimeSelectorDiagnosticStage::SelectorMappingCompletion)
        } else if !timestamp_mapping_succeeded {
            Some(GpuRuntimeSelectorDiagnosticStage::TimestampMappingCompletion)
        } else if self.device_lost.load(Ordering::Acquire) {
            Some(GpuRuntimeSelectorDiagnosticStage::DeviceLostAfterSubmit)
        } else {
            None
        };
        if let Some(stage) = post_submit_failure_stage {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(stage);
            }
            for dispatch in &dispatches {
                let bucket = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                    .expect("submitted bucket exists");
                bucket.buffers.compact_readback().unmap();
                if let Some(readback) = dispatch.selector_readback.as_ref() {
                    readback.unmap();
                }
                let _ = bucket
                    .pipelines
                    .mark_post_submit_poison(dispatch.batch.as_ref().expect("submitted batch"));
            }
            self.timestamp_resources.readback_buffer.unmap();
            self.mark_device_lost();
            return Err(
                if stage == GpuRuntimeSelectorDiagnosticStage::TimestampMappingCompletion {
                    ScaffoldContractError::GpuTimestampQueryUnavailable
                } else {
                    ScaffoldContractError::NeuralBackendUnavailable
                },
            );
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::TimestampReadback);
        }
        let (inference_timestamp_ticks, completed_gpu_time_ns) =
            match self.timestamp_resources.read_delta_and_elapsed_ns() {
                Ok(timing) => timing,
                Err(error) => {
                    for dispatch in &dispatches {
                        self.class_buckets
                            .get(&dispatch.class_id)
                            .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                            .expect("submitted bucket exists")
                            .buffers
                            .compact_readback()
                            .unmap();
                        if let Some(readback) = dispatch.selector_readback.as_ref() {
                            readback.unmap();
                        }
                    }
                    self.poison_submitted_dispatches(&dispatches);
                    return Err(error);
                }
            };

        if capture_selector_diagnostics {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage =
                    Some(GpuRuntimeSelectorDiagnosticStage::DecodeSelectorDiagnostics);
            }
            for index in 0..dispatches.len() {
                let result = {
                    let dispatch = &dispatches[index];
                    let bucket = self
                        .class_buckets
                        .get(&dispatch.class_id)
                        .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                        .expect("mapped bucket exists");
                    bucket.pipelines.decode_mapped_selector_diagnostics(
                        dispatch
                            .selector_readback
                            .as_ref()
                            .expect("diagnostic capture was requested"),
                        dispatch.batch.as_ref().expect("mapped batch"),
                    )
                };
                match result {
                    Ok(captures) => dispatches[index].selector_captures = Some(captures),
                    Err(_) => {
                        for still_mapped in &dispatches[index + 1..] {
                            still_mapped
                                .selector_readback
                                .as_ref()
                                .expect("diagnostic capture was requested")
                                .unmap();
                        }
                        for submitted in &dispatches {
                            let bucket = self
                                .class_buckets
                                .get_mut(&submitted.class_id)
                                .and_then(|pool| pool.chunks.get_mut(submitted.chunk_index))
                                .expect("submitted bucket exists");
                            bucket.buffers.compact_readback().unmap();
                            let _ = bucket.pipelines.mark_post_submit_poison(
                                submitted.batch.as_ref().expect("submitted batch"),
                            );
                        }
                        self.mark_device_lost();
                        return Err(ScaffoldContractError::NeuralBackendUnavailable);
                    }
                }
            }
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::DecodeMappedRecords);
        }
        for index in 0..dispatches.len() {
            let dispatch = &dispatches[index];
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                .expect("mapped bucket exists");
            let mut decode_diagnostic = PipelineDecodeMappedRecordsDiagnostic::default();
            let decoded = if selector_diagnostic_error_capture.is_some() {
                bucket
                    .pipelines
                    .decode_validate_mapped_records_with_diagnostic(
                        &bucket.buffers,
                        dispatch.batch.as_ref().expect("mapped batch"),
                        &mut decode_diagnostic,
                    )
            } else {
                bucket.pipelines.decode_validate_mapped_records(
                    &bucket.buffers,
                    dispatch.batch.as_ref().expect("mapped batch"),
                )
            };
            match decoded {
                Ok(validated) => dispatches[index].validated = Some(validated),
                Err(error) => {
                    if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                        capture.decode_mapped_records_receipt =
                            GpuRuntimeSelectorDiagnosticDecodeMappedRecordsFailureReceipt::from_gpu_error(
                                error,
                                dispatch.class_id,
                                dispatch.chunk_index,
                                decode_diagnostic,
                            );
                    }
                    for still_mapped in &dispatches[index + 1..] {
                        self.class_buckets
                            .get(&still_mapped.class_id)
                            .and_then(|pool| pool.chunks.get(still_mapped.chunk_index))
                            .expect("submitted bucket exists")
                            .buffers
                            .compact_readback()
                            .unmap();
                    }
                    for submitted in &dispatches {
                        let bucket = self
                            .class_buckets
                            .get_mut(&submitted.class_id)
                            .and_then(|pool| pool.chunks.get_mut(submitted.chunk_index))
                            .expect("submitted bucket exists");
                        let _ = bucket.pipelines.mark_post_submit_poison(
                            submitted.batch.as_ref().expect("submitted batch"),
                        );
                    }
                    self.mark_device_lost();
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
            }
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::PrevalidateCommit);
        }
        if dispatches.iter().any(|dispatch| {
            let bucket = self
                .class_buckets
                .get(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get(dispatch.chunk_index))
                .expect("validated bucket exists");
            bucket
                .pipelines
                .prevalidate_commit_validated_batch(
                    dispatch.validated.as_ref().expect("validated batch"),
                )
                .is_err()
        }) {
            for dispatch in &dispatches {
                let bucket = self
                    .class_buckets
                    .get_mut(&dispatch.class_id)
                    .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                    .expect("validated bucket exists");
                let _ = bucket
                    .pipelines
                    .mark_post_submit_poison(dispatch.batch.as_ref().expect("submitted batch"));
            }
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        let mut ordered_records = vec![None; batch.len()];
        let mut ordered_speech_payloads = vec![None; batch.len()];
        let mut ordered_factorized_motor_candidates =
            vec![[0_u16; crate::GPU_MOTOR_CHANNEL_SLOT_COUNT]; batch.len()];
        let mut ordered_pending_receipts = vec![None; batch.len()];
        let mut ordered_pending_records = vec![None; batch.len()];
        let mut ordered_next_transaction_generations = vec![None; batch.len()];
        let mut ordered_memory_receipts = vec![None; batch.len()];
        let mut ordered_selector_diagnostics = vec![None; batch.len()];
        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ValidateReceiptIdentity);
        }
        let receipt_validation = (|| -> Result<(), ScaffoldContractError> {
            for dispatch in &dispatches {
                let validated = dispatch.validated.as_ref().expect("validated batch");
                let memory_bindings = dispatch
                    .batch
                    .as_ref()
                    .expect("validated batch retains its upload")
                    .memory_context_bindings();
                for (
                    (
                        (((original_index, selection), speech_payload), motor_candidates),
                        pending_record,
                    ),
                    memory_binding,
                ) in dispatch
                    .original_indices
                    .iter()
                    .zip(validated.records())
                    .zip(validated.speech_payloads())
                    .zip(validated.factorized_motor_candidates())
                    .zip(validated.pending_records())
                    .zip(memory_bindings)
                {
                    if selection.status != 1 {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    let input = batch[*original_index];
                    let handle = input.handle;
                    let frame = input.frame;
                    let candidate_index = u16::try_from(selection.candidate_index)
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                    let candidate = frame
                        .candidates()
                        .get(candidate_index as usize)
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    let receipt = PendingEligibilityReceipt::from_gpu_record(
                        *pending_record,
                        handle.slot,
                        handle.organism_id,
                        handle.phenotype_hash,
                    )?;
                    let identity = receipt.identity();
                    let resident = self
                        .class_buckets
                        .get(&handle.class_id.raw())
                        .and_then(|pool| pool.resident(handle).ok())
                        .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                    if identity.handle_generation() != handle.generation
                        || identity.dispatch_generation() != dispatch_generation.get()
                        || identity.originating_tick() != frame.tick()
                        || identity.frame_digest() != frame.frame_digest()
                        || u32::from(identity.active_activation_side())
                            != selection.active_activation_side
                        || identity.candidate_index() != candidate_index
                        || identity.action_id() != candidate.action_id
                        || identity.action_family() != candidate.family
                        || identity.candidate_feature_digest() != candidate.feature_digest()?
                        || identity.active_eligibility_generation()
                            != resident.active_eligibility_generation
                        || identity.staging_eligibility_generation()
                            != resident
                                .active_eligibility_generation
                                .checked_add(1)
                                .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?
                    {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    match (input.memory_upload, *memory_binding) {
                        (None, None) => {}
                        (Some(_), Some(memory_receipt))
                            if memory_receipt.slot == handle.slot
                                && memory_receipt.slot_generation == handle.generation
                                && memory_receipt.base_frame_digest == frame.base_digest()
                                && memory_receipt.context_digest
                                    == frame.context().canonical_digest()
                                && memory_receipt.final_frame_digest == frame.frame_digest()
                                && usize::from(memory_receipt.candidate_count)
                                    == frame.candidates().len() =>
                        {
                            ordered_memory_receipts[*original_index] = Some(memory_receipt);
                        }
                        _ => return Err(ScaffoldContractError::InvalidDecisionEvidence),
                    }
                    let next_transaction_generation = resident
                        .transaction_generation
                        .checked_add(1)
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    ordered_records[*original_index] = Some(*selection);
                    ordered_speech_payloads[*original_index] = speech_payload.clone();
                    ordered_factorized_motor_candidates[*original_index] = *motor_candidates;
                    ordered_pending_receipts[*original_index] = Some(receipt);
                    ordered_pending_records[*original_index] = Some(*pending_record);
                    ordered_next_transaction_generations[*original_index] =
                        Some(next_transaction_generation);
                }
            }
            Ok(())
        })();
        if receipt_validation.is_err() {
            self.poison_submitted_dispatches(&dispatches);
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        if capture_selector_diagnostics {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage =
                    Some(GpuRuntimeSelectorDiagnosticStage::BuildSelectorDiagnostic);
            }
            let mut failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::MissingCapture;
            let mut binding_identity_failure = None;
            let selector_validation = (|| -> Result<(), ScaffoldContractError> {
                for dispatch in &dispatches {
                    failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::MissingCapture;
                    let captures = dispatch
                        .selector_captures
                        .as_ref()
                        .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                    failure_field = GpuRuntimeSelectorDiagnosticBuildFailureField::CaptureCount;
                    if captures.len() != dispatch.original_indices.len() {
                        return Err(ScaffoldContractError::InvalidDecisionEvidence);
                    }
                    for (original_index, capture) in dispatch.original_indices.iter().zip(captures)
                    {
                        let input = batch[*original_index];
                        failure_field =
                            GpuRuntimeSelectorDiagnosticBuildFailureField::MissingSelectionRecord;
                        let record = ordered_records[*original_index]
                            .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                        failure_field =
                            GpuRuntimeSelectorDiagnosticBuildFailureField::ChosenCandidateIndex;
                        let chosen = u16::try_from(record.candidate_index)
                            .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                        failure_field =
                            GpuRuntimeSelectorDiagnosticBuildFailureField::ResidentBrainOwnership;
                        let resident = self
                            .class_buckets
                            .get(&input.handle.class_id.raw())
                            .and_then(|pool| pool.resident(input.handle).ok())
                            .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                        ordered_selector_diagnostics[*original_index] =
                            Some(build_selector_diagnostic(
                                input.frame,
                                &resident.phenotype,
                                &resident.brain_slot,
                                resident.active_weight_bank,
                                dispatch_generation.get(),
                                chosen,
                                capture,
                                &mut failure_field,
                                &mut binding_identity_failure,
                            )?);
                    }
                }
                Ok(())
            })();
            if let Err(error) = selector_validation {
                let class = match error {
                    ScaffoldContractError::InvalidDecisionEvidence => {
                        Some(GpuRuntimeSelectorDiagnosticBuildFailureClass::InvalidDecisionEvidence)
                    }
                    ScaffoldContractError::BrainOwnershipMismatch => {
                        Some(GpuRuntimeSelectorDiagnosticBuildFailureClass::BrainOwnershipMismatch)
                    }
                    _ => None,
                };
                if let (Some(capture), Some(class)) =
                    (selector_diagnostic_error_capture.as_deref_mut(), class)
                {
                    capture.build_selector_diagnostic_receipt =
                        Some(GpuRuntimeSelectorDiagnosticBuildFailureReceipt {
                            class,
                            field: failure_field,
                            expected_binding_identity: binding_identity_failure
                                .map(|(expected, _)| expected),
                            actual_binding_identity: binding_identity_failure
                                .map(|(_, actual)| actual),
                        });
                }
                self.poison_submitted_dispatches(&dispatches);
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::AccountActivityWork);
        }
        let activity_work_receipts: Result<Vec<BrainWorkReceipt>, ScaffoldContractError> = batch
            .iter()
            .enumerate()
            .map(|(index, input)| {
                let handle = input.handle;
                let resident = self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?;
                let decision = &activity_decisions[index];
                let candidate_count = u32::try_from(input.frame.candidates().len())
                    .map_err(|_| ScaffoldContractError::BrainActivityPolicyMismatch)?;
                let memory_context_count = input
                    .memory_upload
                    .map_or(0, |upload| upload.header.candidate_count);
                let work = derive_executed_work(
                    &resident.phenotype,
                    decision.microsteps,
                    &decision.enabled_route_ids,
                    candidate_count,
                    memory_context_count,
                )?;
                let record =
                    ordered_records[index].ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                if work.tile_visits != u64::from(record.active_tiles)
                    || work.synapse_ops != u64::from(record.active_synapses)
                {
                    return Err(ScaffoldContractError::BrainActivitySequenceMismatch);
                }
                BrainWorkReceipt::try_new(
                    &self.activity_policy,
                    decision,
                    work,
                    resident.brain_atp_q16,
                )
            })
            .collect();
        let activity_work_receipts = match activity_work_receipts {
            Ok(receipts) => receipts,
            Err(error) => {
                self.poison_submitted_dispatches(&dispatches);
                return Err(error);
            }
        };

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::PrepareTicks);
        }
        let prepared_ticks = (|| -> Result<Vec<GpuClosedLoopTick>, ScaffoldContractError> {
            let mut ticks = Vec::with_capacity(batch.len());
            for (index, input) in batch.iter().enumerate() {
                let handle = input.handle;
                let frame = input.frame;
                let record =
                    ordered_records[index].ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let pending_eligibility = ordered_pending_receipts[index]
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let candidate_index = u16::try_from(record.candidate_index)
                    .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?;
                let candidate = frame
                    .candidates()
                    .get(candidate_index as usize)
                    .ok_or(ScaffoldContractError::InvalidDecisionEvidence)?;
                let v11_work = self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .ok_or(ScaffoldContractError::BrainOwnershipMismatch)?
                    .v11
                    .gpu_recurrent_work_receipt(
                        record.dendritic_branches_evaluated,
                        record.dendritic_inputs_evaluated,
                        record.dendritic_gated_branches,
                        record.structural_edges_evaluated,
                    )?;
                ticks.push(GpuClosedLoopTick {
                    handle,
                    dispatch_generation: dispatch_generation.get(),
                    base_digest: frame.base_digest(),
                    frame_digest: frame.frame_digest(),
                    memory_context_binding: ordered_memory_receipts[index],
                    active_activation_side: u8::try_from(record.active_activation_side)
                        .map_err(|_| ScaffoldContractError::InvalidDecisionEvidence)?,
                    selection: NeuralActionSelection {
                        candidate_index,
                        logit: f32::from_bits(record.logit_bits),
                        confidence: Confidence::new(candidate.sensor_confidence.raw())?,
                        active_tiles: record.active_tiles,
                        active_synapses: record.active_synapses,
                    },
                    speech_payload: ordered_speech_payloads[index].clone(),
                    factorized_motor_candidates: ordered_factorized_motor_candidates[index],
                    pending_eligibility,
                    pressure: activity_decisions[index].pressure,
                    throttle: activity_decisions[index].clone(),
                    work: activity_work_receipts[index].clone(),
                    v11_work,
                    compact_readback_bytes: crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES,
                    hardware_receipt_generation: self.hardware.generation,
                    selector_diagnostic: ordered_selector_diagnostics[index].clone(),
                });
            }
            Ok(ticks)
        })();
        let prepared_ticks = match prepared_ticks {
            Ok(ticks) => ticks,
            Err(_) => {
                self.poison_submitted_dispatches(&dispatches);
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ComputeReadbackBytes);
        }
        let total_readback_bytes = match batch
            .len()
            .checked_mul(crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES)
        {
            Some(bytes) => bytes,
            None => {
                self.poison_submitted_dispatches(&dispatches);
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
        };
        let mut commit_mismatch = false;
        for dispatch in &mut dispatches {
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::CommitValidatedBatch);
            }
            let bucket = self
                .class_buckets
                .get_mut(&dispatch.class_id)
                .and_then(|pool| pool.chunks.get_mut(dispatch.chunk_index))
                .expect("validated bucket exists");
            let commit = bucket
                .pipelines
                .commit_validated_batch(dispatch.validated.take().expect("validated batch"));
            let committed = match commit {
                Ok(committed) => committed,
                Err(_) => {
                    self.mark_device_lost();
                    return Err(ScaffoldContractError::NeuralBackendUnavailable);
                }
            };
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ValidateCommitShape);
            }
            if committed.readback_bytes as usize
                != dispatch.original_indices.len() * crate::GPU_CLOSED_LOOP_TICK_READBACK_BYTES
                || committed.records.len() != dispatch.original_indices.len()
                || committed.speech_payloads.len() != dispatch.original_indices.len()
                || committed.factorized_motor_candidates.len() != dispatch.original_indices.len()
                || committed.pending_records.len() != dispatch.original_indices.len()
            {
                self.mark_device_lost();
                return Err(ScaffoldContractError::NeuralBackendUnavailable);
            }
            if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
                capture.later_stage =
                    Some(GpuRuntimeSelectorDiagnosticStage::ValidateCommitContents);
            }
            for ((((original_index, record), speech_payload), motor_candidates), pending_record) in
                dispatch
                    .original_indices
                    .iter()
                    .zip(committed.records)
                    .zip(committed.speech_payloads)
                    .zip(committed.factorized_motor_candidates)
                    .zip(committed.pending_records)
            {
                commit_mismatch |= ordered_records[*original_index] != Some(record)
                    || ordered_speech_payloads[*original_index] != speech_payload
                    || ordered_factorized_motor_candidates[*original_index] != motor_candidates
                    || ordered_pending_records[*original_index] != Some(pending_record);
            }
        }
        if commit_mismatch {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }

        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ValidateHostPrecommit);
        }
        let host_precommit_valid = batch.iter().enumerate().all(|(index, input)| {
            let handle = input.handle;
            let Some(expected_generation) = ordered_next_transaction_generations[index] else {
                return false;
            };
            ordered_pending_receipts[index].is_some()
                && ordered_pending_records[index].is_some()
                && self
                    .class_buckets
                    .get(&handle.class_id.raw())
                    .and_then(|pool| pool.resident(handle).ok())
                    .is_some_and(|resident| {
                        resident.pending_eligibility.is_none()
                            && resident.pending_eligibility_record.is_none()
                            && resident.activity_sequence_cursor
                                == activity_decisions[index].sequence_cursor
                            && resident.brain_atp_q16
                                == activity_work_receipts[index].atp_before_q16
                            && activity_work_receipts[index]
                                .validate_for(&self.activity_policy, &activity_decisions[index])
                                .is_ok()
                            && resident.transaction_generation.checked_add(1)
                                == Some(expected_generation)
                    })
        });
        if !host_precommit_valid {
            self.mark_device_lost();
            return Err(ScaffoldContractError::NeuralBackendUnavailable);
        }
        for (index, input) in batch.iter().enumerate() {
            let handle = input.handle;
            let resident = self
                .class_buckets
                .get_mut(&handle.class_id.raw())
                .and_then(|pool| pool.resident_mut(handle).ok())
                .expect("host pending commit was prevalidated");
            resident.transaction_generation = ordered_next_transaction_generations[index]
                .expect("host transaction generation was prevalidated");
            resident.inactive_eligibility_generation = resident
                .active_eligibility_generation
                .checked_add(1)
                .expect("eligibility generation was prevalidated");
            resident.logical_dispatch_generation = dispatch_generation.get();
            resident.activity_sequence_cursor = resident
                .activity_sequence_cursor
                .checked_add(1)
                .expect("activity cursor was prevalidated");
            resident.brain_atp_q16 = activity_work_receipts[index].atp_after_q16;
            resident.last_activity_dispatch_generation = dispatch_generation.get();
            resident.last_activity_frame_digest = input.frame.frame_digest().0;
            resident.last_completed_gpu_time_ns = completed_gpu_time_ns;
            resident.last_pressure = Some(activity_decisions[index].pressure);
            resident.last_throttle = Some(activity_decisions[index].clone());
            resident.last_work = Some(activity_work_receipts[index].clone());
            resident
                .v11
                .record_gpu_recurrent_work(prepared_ticks[index].v11_work);
            resident.pending_eligibility = ordered_pending_receipts[index];
            resident.pending_eligibility_record = ordered_pending_records[index];
        }

        self.completed_dispatch_count = next_completed_dispatch_count;
        self.last_compact_readback_bytes = total_readback_bytes;
        self.next_dispatch_generation = next_dispatch_generation;
        self.completed_selection_count = next_completed_selection_count;
        self.completed_neural_timing = None;
        if let Some(capture) = selector_diagnostic_error_capture.as_deref_mut() {
            capture.later_stage = Some(GpuRuntimeSelectorDiagnosticStage::ConvertPopulation);
        }
        self.pending_inference_timing = Some(PendingInferenceTiming {
            dispatch_generation: dispatch_generation.get(),
            class_id_raw: timing_class_id,
            population: u32::try_from(batch.len())
                .map_err(|_| ScaffoldContractError::NeuralBackendUnavailable)?,
            inference_timestamp_ticks,
        });
        Ok(prepared_ticks)
    }
}
