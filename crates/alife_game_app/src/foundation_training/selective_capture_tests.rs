//! Explicit CPU-only read-set falsification; no GPU dispatch or capture change.
use super::*;
use alife_core::*;
use alife_gpu_backend::{GpuMemoryContextUpload, GpuPerceptionUpload};
use std::sync::Arc;

fn fill(snapshot: &mut GpuTrainingStateSnapshot, range: Range<u32>, value: u32) {
    let base = snapshot.mutable_word_base;
    snapshot.mutable_words[(range.start - base) as usize..(range.end - base) as usize].fill(value);
}

fn pattern(snapshot: &mut GpuTrainingStateSnapshot, range: Range<u32>, base: f32, scale: f32) {
    let offset = snapshot.mutable_word_base;
    for (index, word) in snapshot.mutable_words
        [(range.start - offset) as usize..(range.end - offset) as usize]
        .iter_mut()
        .enumerate()
    {
        *word = (base + index as f32 * scale).to_bits();
    }
}

fn fixture() -> (FoundationTrainingStep, FoundationWeightAsset) {
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(42, capacity.id());
    let development = DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar(1.0));
    let native = PhenotypeCompiler::compile_testing_procedural_baseline(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap();
    let asset = FoundationWeightAsset::from_phenotype_for_genetic_birth(&native).unwrap();
    let (phenotype, _) =
        PhenotypeCompiler::compile_n2048_foundation_candidate(genome, development, asset.clone())
            .unwrap();
    let mut world = super::held_food_lesson_tests::unconfigured(42);
    let scenario = configure_foundation_scenario(
        &mut world,
        42,
        Some(FoundationTeacherLesson::GrabFood),
        None,
        false,
    )
    .unwrap();
    let organism = world.organism_entity_ids()[0].0;
    let tick = world.tick();
    let homeostasis = world
        .organism_registry()
        .get(organism)
        .unwrap()
        .biochemistry()
        .homeostasis;
    let draft = world
        .perception_frame_draft(
            organism,
            tick,
            SensorProfile::GroundedTerrainVisionV1,
            homeostasis,
        )
        .unwrap();
    let memory = MemorySidecarState::new_profiled(
        organism,
        draft.profile_provenance().identity(),
        MemoryBankConfig::new(64, 64, 8, 0.0, Confidence(0.0)).unwrap(),
    )
    .unwrap();
    let (frame, recall) = memory
        .recall_frame(&draft)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let mut before = GpuClosedLoopBackend::synthetic_training_snapshot_for_cpu_test(
        Arc::new(phenotype),
        organism,
        tick,
    )
    .unwrap();
    let layout = before.brain_slot.word_ranges().clone();
    pattern(&mut before, layout.activation_a_words.clone(), 0.125, 1e-5);
    pattern(&mut before, layout.activation_b_words.clone(), 0.25, 1e-5);
    pattern(&mut before, layout.homeostasis_words.clone(), 0.25, 1e-5);
    pattern(
        &mut before,
        layout.lifetime_weight_words.clone(),
        0.015625,
        1e-7,
    );
    pattern(
        &mut before,
        layout.lifetime_weight_bank_1_words.clone(),
        0.03125,
        1e-7,
    );
    pattern(
        &mut before,
        layout.fast_weight_words.clone(),
        -0.0078125,
        1e-8,
    );
    pattern(
        &mut before,
        layout.fast_weight_bank_1_words.clone(),
        -0.00390625,
        1e-8,
    );
    let mut after = before.clone();
    after.logical_dispatch_generation = 2;
    pattern(&mut after, layout.activation_a_words.clone(), 0.375, 1e-5);
    pattern(&mut after, layout.activation_b_words.clone(), 0.5, 1e-5);
    pattern(&mut after, layout.encoded_input_words.clone(), 0.25, 1e-5);
    let upload = GpuPerceptionUpload::try_from_frame(&frame, &before.brain_slot, 0).unwrap();
    let organism_record = world.organism_registry().get(organism).unwrap();
    let receptors = organism_record
        .biochemistry()
        .neural_receptor_frame(organism_record.phenotype())
        .unwrap();
    let effects = NeuralReceptorEffects::from_frame(
        &receptors,
        &NeuralReceptorPhenotype::compile(&before.phenotype).unwrap(),
    )
    .unwrap();
    let memory_upload = GpuMemoryContextUpload::try_from_finalized(
        &frame,
        &recall,
        upload.frame_binding,
        &before.brain_slot,
    )
    .unwrap()
    .bind_neural_receptor_effects(effects)
    .unwrap();
    let (_, outcome) = super::held_food_lesson_tests::eat_outcome(&mut world, scenario.food);
    let patch = super::held_food_lesson_tests::eating_narration_patch(
        frame.clone(),
        scenario.food,
        outcome,
    );
    let p = before.phenotype.as_ref();
    let policy = BrainActivityPolicyV1::production_v1();
    let identity = BrainDispatchIdentity {
        organism_id_raw: organism.raw(),
        tick: tick.raw(),
        class_id_raw: p.brain_class_id().raw(),
        handle_slot: before.handle.slot(),
        handle_generation: before.handle.generation(),
        sequence_cursor: 1,
        dispatch_generation: 2,
        frame_digest: frame.frame_digest().0,
    };
    let pressure = GpuPressureSample::try_new(
        &policy,
        GpuPressureSampleInput {
            identity,
            source_dispatch_generation: 0,
            source_frame_digest: [0; 4],
            completed_gpu_time_ns: 0,
            queue_depth: 0,
            logical_heap_used: 0,
            logical_heap_capacity: 1,
            brain_atp_remaining_q16: BRAIN_ATP_Q16_MAX,
            brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
        },
    )
    .unwrap();
    let throttle =
        NeuralThrottleDecision::derive(&policy, p, capacity.execution(), identity, pressure)
            .unwrap();
    after.active_activation_side = before.active_activation_side ^ (throttle.microsteps & 1);
    let work = BrainWorkReceipt::try_new(
        &policy,
        &throttle,
        BrainWorkCounters {
            microsteps: u32::from(throttle.microsteps),
            ..Default::default()
        },
        BRAIN_ATP_Q16_MAX,
    )
    .unwrap();
    let stride = usize::from(p.candidate_decoder().flattened_input_lane_count());
    let inputs = frame
        .candidates()
        .iter()
        .flat_map(|c| {
            let mut a = vec![0.0; stride];
            a[..24].copy_from_slice(&c.features.0);
            a
        })
        .collect();
    let behavior = alife_gpu_backend::GpuTrainingRolloutReceipt {
        organism_id: organism.raw(),
        tick: tick.raw(),
        dispatch_generation: 2,
        frame_digest: frame.frame_digest(),
        active_weight_generation: before.active_weight_generation,
        sampling: GpuTrainingSamplingConfig {
            seed: 1,
            counter: 0,
            temperature: 1.0,
            demonstrator: None,
        },
        logits: vec![0.0; frame.candidates().len()],
        decoder_inputs: inputs,
        decoder_input_stride: stride,
        representative_mask: 1,
        motor_masks: [0; 6],
        forced_motor_slots: vec![u8::MAX; frame.candidates().len()],
        representative_index: 0,
        motor_indices: [u16::MAX; 6],
        factor_log_probabilities: [0.0; 7],
        behavior: alife_gpu_backend::GpuTrainingBehaviorKind::OnPolicy,
        joint_log_probability: Some(0.0),
        policy_joint_log_probability: 0.0,
    };
    (
        FoundationTrainingStep {
            outcome_credit: OutcomeCreditPacket::from_sealed_patch(&patch).unwrap(),
            activity_evidence: None,
            frame,
            memory_upload,
            before,
            after_inference: after,
            behavior,
            work,
            throttle,
            patch,
        },
        asset,
    )
}

fn selected_plan(
    snapshot: &GpuTrainingStateSnapshot,
    post: bool,
    both_banks: bool,
) -> Result<Vec<Range<u32>>> {
    let r = snapshot.brain_slot.word_ranges();
    // Invalid selector metadata must reach the unchanged consumer and retain
    // its original contextual error, rather than failing earlier in a new plan.
    let both_banks =
        both_banks || snapshot.active_activation_side > 1 || snapshot.active_weight_bank > 1;
    let mut ranges = if both_banks {
        vec![
            r.activation_a_words.clone(),
            r.activation_b_words.clone(),
            r.homeostasis_words.clone(),
            r.lifetime_weight_words.clone(),
            r.fast_weight_words.clone(),
            r.lifetime_weight_bank_1_words.clone(),
            r.fast_weight_bank_1_words.clone(),
        ]
    } else {
        let (l, f) = active_weight_ranges(snapshot)?;
        vec![
            activation_range(snapshot)?,
            r.homeostasis_words.clone(),
            l,
            f,
        ]
    };
    if post {
        ranges.push(
            snapshot
                .brain_slot
                .word_ranges()
                .encoded_input_words
                .clone(),
        );
    }
    Ok(ranges)
}

fn selected(snapshot: &GpuTrainingStateSnapshot, post: bool) -> Result<Vec<Range<u32>>> {
    selected_plan(snapshot, post, false)
}

fn project(
    snapshot: &GpuTrainingStateSnapshot,
    post: bool,
    poison: u32,
) -> Result<(GpuTrainingStateSnapshot, usize)> {
    project_plan(snapshot, post, poison, false)
}

fn project_plan(
    snapshot: &GpuTrainingStateSnapshot,
    post: bool,
    poison: u32,
    both_banks: bool,
) -> Result<(GpuTrainingStateSnapshot, usize)> {
    // A CPU packet of selected u32 spans, subsequently materialized with
    // poisoned holes only to exercise the unchanged full-snapshot consumers.
    // This is not installed as the public snapshot or GPU transport format.
    let parts = selected_plan(snapshot, post, both_banks)?;
    let mut packet = Vec::new();
    for range in &parts {
        packet.extend_from_slice(snapshot_words(snapshot, range.clone())?);
    }
    let mut result = snapshot.clone();
    result.mutable_words.fill(poison);
    let mut bytes = 0;
    let mut consumed = 0;
    for range in parts {
        let start = (range.start - snapshot.mutable_word_base) as usize;
        let end = (range.end - snapshot.mutable_word_base) as usize;
        let source = packet
            .get(consumed..consumed + end - start)
            .ok_or_else(invalid)?;
        result
            .mutable_words
            .get_mut(start..end)
            .ok_or_else(invalid)?
            .copy_from_slice(source);
        consumed += end - start;
        bytes += (end - start) * 4;
    }
    assert_eq!(consumed, packet.len());
    Ok((result, bytes))
}

fn activity(step: &mut FoundationTrainingStep, microsteps: u8) {
    let policy = BrainActivityPolicyV1::production_v1();
    let identity = BrainDispatchIdentity {
        organism_id_raw: step.frame.organism_id().raw(),
        tick: step.frame.tick().raw(),
        class_id_raw: step.before.phenotype.brain_class_id().raw(),
        handle_slot: step.before.handle.slot(),
        handle_generation: step.before.handle.generation(),
        sequence_cursor: 1,
        dispatch_generation: 2,
        frame_digest: step.frame.frame_digest().0,
    };
    let pressure = GpuPressureSample::try_new(
        &policy,
        GpuPressureSampleInput {
            identity,
            source_dispatch_generation: 0,
            source_frame_digest: [0; 4],
            completed_gpu_time_ns: match microsteps {
                2 => 10_000_000,
                3 => 2_000_000,
                _ => 0,
            },
            queue_depth: 0,
            logical_heap_used: 0,
            logical_heap_capacity: 1,
            brain_atp_remaining_q16: BRAIN_ATP_Q16_MAX,
            brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
        },
    )
    .unwrap();
    let capacity = BrainCapacityClass::n2048();
    step.throttle = NeuralThrottleDecision::derive(
        &policy,
        &step.before.phenotype,
        capacity.execution(),
        identity,
        pressure,
    )
    .unwrap();
    assert_eq!(step.throttle.microsteps, microsteps);
    step.throttle
        .validate_for(&step.before.phenotype, capacity.execution())
        .unwrap();
    step.work = BrainWorkReceipt::try_new(
        &policy,
        &step.throttle,
        BrainWorkCounters {
            microsteps: u32::from(microsteps),
            ..Default::default()
        },
        BRAIN_ATP_Q16_MAX,
    )
    .unwrap();
}

fn continuity_outcomes(step: &FoundationTrainingStep, projected: bool) -> Vec<bool> {
    let after = ReplayContinuity::capture(&step.after_inference).unwrap();
    let mut outcomes = Vec::new();
    for case in 0..8 {
        let mut next = step.after_inference.clone();
        next.tick += 1;
        match case {
            0 => {}
            1 => {
                let r = activation_range(&next).unwrap();
                fill(&mut next, r, 0.0_f32.to_bits());
            }
            2 => {
                let r = next.brain_slot.word_ranges().homeostasis_words.clone();
                fill(&mut next, r, 0.0_f32.to_bits());
            }
            3 => {
                let (r, _) = active_weight_ranges(&next).unwrap();
                let i = (r.start - next.mutable_word_base) as usize;
                next.mutable_words[i] ^= 1;
            }
            4 => next.active_weight_bank ^= 1,
            5 => next.logical_dispatch_generation += 1,
            6 => next.active_weight_generation -= 1,
            7 => {
                next.active_weight_generation += 1;
                next.active_weight_bank ^= 1;
                let (r, _) = active_weight_ranges(&next).unwrap();
                fill(&mut next, r, 0.125_f32.to_bits());
            }
            _ => unreachable!(),
        }
        if projected {
            next = project(&next, false, 0x7fc0_0001).unwrap().0;
        }
        let result = after
            .validate_next(&ReplayContinuity::capture(&next).unwrap())
            .is_ok();
        assert_eq!(result, matches!(case, 0 | 7));
        outcomes.push(result);
    }
    outcomes
}

fn bundle_bytes(record: &FoundationReplayRecord, phenotype: &BrainPhenotype) -> Result<Vec<u8>> {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "alife-selective-packet-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir(&root)?;
    let result = (|| -> Result<Vec<u8>> {
        let mut writer = CohortReplayWriter::new(&root, record.source.clone())?;
        let reference = writer.append(record)?;
        writer.seal()?;
        let mut reader = cohort_replay::CohortReplayReader::open(&root, &record.source)?;
        // Use the actual bounded/digest-checked storage decoder. Decline its
        // optional cache retention; do not stub the public Windows RAM gate.
        let decoded = load_foundation_replay_record_from_storage(
            &root,
            &reference,
            &record.source,
            phenotype,
            Some(&mut reader),
            |_| false,
        )?;
        assert_eq!(
            bincode::serde::encode_to_vec(&decoded, bincode::config::standard())?,
            bincode::serde::encode_to_vec(record, bincode::config::standard())?
        );
        drop(reader);
        let bytes = std::fs::read(root.join("cohort-replay.bundle"))?;
        let mut corrupt = bytes.clone();
        let location = reference.cohort.as_ref().expect("cohort reference");
        corrupt[(location.offset + reference.encoded_bytes - 1) as usize] ^= 1;
        std::fs::write(root.join("cohort-replay.bundle"), corrupt)?;
        let rejected = (|| -> Result<()> {
            let mut reader = cohort_replay::CohortReplayReader::open(&root, &record.source)?;
            load_foundation_replay_record_from_storage(
                &root,
                &reference,
                &record.source,
                phenotype,
                Some(&mut reader),
                |_| false,
            )?;
            Ok(())
        })()
        .unwrap_err()
        .to_string();
        Ok(bincode::serde::encode_to_vec(
            (bytes, rejected),
            bincode::config::standard(),
        )?)
    })();
    std::fs::remove_dir_all(&root)?;
    result
}

fn observable(step: &FoundationTrainingStep, asset: &FoundationWeightAsset) -> Result<Vec<u8>> {
    let p = step.before.phenotype.as_ref();
    let sequence = foundation_replay_sequence(std::slice::from_ref(step), 0)?;
    let (before, after) = ReplayContinuity::capture_step_pair(&step.before, &step.after_inference)?;
    let features = snapshot_activation(&step.after_inference)?;
    let width = p.neuron_count() as usize + 1;
    let mut parameters = vec![0.0; width * 3];
    for i in 0..width {
        parameters[i] = (i % 7) as f32 * 0.0001;
        parameters[width + i] = 1e-6;
        parameters[2 * width + i] = 1e-8;
    }
    let value = alife_training::cpu::CpuValueHead::from_checkpoint(
        alife_training::PpoValueHeadCheckpoint {
            feature_count: p.neuron_count(),
            optimizer_step: 19_800,
            parameters,
            last_updated_policy_version: Some(7),
        },
    )?;
    let checkpoint = value.checkpoint();
    let prediction = value.predict(&features)?;
    assert_eq!(value.checkpoint(), checkpoint);
    let source = FoundationReplaySource {
        source_revision: format!("synthetic-selective-range-fixture-{}", std::process::id()),
        policy_version: 7,
        actor_checkpoint_digest: Blake3Digest::from_bytes([1; 32]),
        foundation_asset_digest: asset.digest(),
        phenotype_hash: p.phenotype_hash(),
        compiler_inputs_digest: p.compiler_inputs_digest(),
    };
    let record = FoundationReplayRecord {
        schema: FOUNDATION_REPLAY_SCHEMA,
        source,
        record_index: 0,
        segment: 0,
        tick: step.frame.tick().raw(),
        organism_id: step.frame.organism_id().raw(),
        sequence,
        behavior: step.behavior.clone(),
        patch: step.patch.clone(),
        before,
        after,
    };
    record.validate(&record.source, p)?;
    let bundle = bundle_bytes(&record, p)?;
    Ok(bincode::serde::encode_to_vec(
        (&record, features, prediction.to_bits(), checkpoint, bundle),
        bincode::config::standard(),
    )?)
}

fn identical_rejection(step: &FoundationTrainingStep, asset: &FoundationWeightAsset) -> String {
    let expected = observable(step, asset).unwrap_err().to_string();
    let reduced = (|| -> Result<Vec<u8>> {
        let mut reduced = step.clone();
        reduced.before = project(&step.before, false, 0x7fc0_0001)?.0;
        reduced.after_inference = project(&step.after_inference, true, 0x7fc0_0001)?.0;
        observable(&reduced, asset)
    })();
    assert_eq!(reduced.unwrap_err().to_string(), expected);
    expected
}

fn packet_plans(step: &FoundationTrainingStep) -> Vec<serde_json::Value> {
    let mut result = Vec::new();
    let limit = 64_u64 * 1024 * 1024;
    for residents in [1, 8, 50] {
        for post in [false, true] {
            for plan in [0, 1, 2] {
                let snapshot = if post {
                    &step.after_inference
                } else {
                    &step.before
                };
                let ranges = if plan == 0 {
                    std::iter::once(
                        snapshot.mutable_word_base
                            ..snapshot.mutable_word_base + snapshot.mutable_words.len() as u32,
                    )
                    .collect()
                } else {
                    selected_plan(snapshot, post, plan == 2).unwrap()
                };
                let sizes = (0..residents)
                    .flat_map(|_| ranges.iter().map(|r| u64::from(r.end - r.start) * 4))
                    .collect::<Vec<_>>();
                let packets =
                    GpuClosedLoopBackend::training_capture_packet_plan_for_cpu_test(&sizes, limit)
                        .unwrap();
                assert_eq!(packets, vec![0..sizes.len()]);
                result.push(serde_json::json!({"residents":residents,"post":post,"plan":(["full","selected","both_banks"][plan]),
            "bytes":sizes.iter().sum::<u64>(),"regions":sizes.len(),"packet_count":packets.len(),"packet_limit_bytes":limit}));
            }
        }
    }
    result
}

#[test]
#[ignore = "Explicit CPU-only selective-range falsification; no GPU capture implementation"]
fn selective_capture_projection_falsification() {
    let (base, asset) = fixture();
    let packets = packet_plans(&base);
    let mut accepted = 0;
    let mut corruptions = 0;
    let mut metadata_rejections = 0;
    let mut cases = Vec::new();
    for activation_side in [0, 1] {
        for weight_bank in [0, 1] {
            for microsteps in [2, 3, base.throttle.microsteps] {
                let mut step = base.clone();
                step.before.active_activation_side = activation_side;
                step.before.active_weight_bank = weight_bank;
                step.after_inference.active_weight_bank = weight_bank;
                activity(&mut step, microsteps);
                step.after_inference.active_activation_side = activation_side ^ (microsteps & 1);
                let expected = observable(&step, &asset).unwrap();
                assert_eq!(
                    continuity_outcomes(&step, false),
                    continuity_outcomes(&step, true)
                );
                let mut pair_bytes = 0;
                let mut independent_bytes = 0;
                for (pre_poison, post_poison) in [
                    (0x7fc0_0001, 0x7fc0_0002),
                    (0x7f80_0000, 0xff80_0000),
                    (0xff80_0000, 0x7f80_0000),
                    (0xffff_ffff, 0x7fc1_2345),
                ] {
                    for both_banks in [false, true] {
                        let mut projected = step.clone();
                        let (before, b) =
                            project_plan(&step.before, false, pre_poison, both_banks).unwrap();
                        let (after, a) =
                            project_plan(&step.after_inference, true, post_poison, both_banks)
                                .unwrap();
                        projected.before = before;
                        projected.after_inference = after;
                        if both_banks {
                            independent_bytes = b + a;
                        } else {
                            pair_bytes = b + a;
                        }
                        assert_eq!(observable(&projected, &asset).unwrap(), expected);
                        accepted += 1;
                    }
                }
                // Every retained region is poisoned at both boundaries: retain the
                // original finite-state, exact weight and continuity admission behavior.
                for post in [false, true] {
                    for range in selected(
                        if post {
                            &step.after_inference
                        } else {
                            &step.before
                        },
                        post,
                    )
                    .unwrap()
                    {
                        for at_end in [false, true] {
                            let mut bad = step.clone();
                            let s = if post {
                                &mut bad.after_inference
                            } else {
                                &mut bad.before
                            };
                            let index = (if at_end { range.end - 1 } else { range.start }
                                - s.mutable_word_base)
                                as usize;
                            s.mutable_words[index] = f32::NAN.to_bits();
                            let error = observable(&bad, &asset).unwrap_err().to_string();
                            let mut reduced = bad.clone();
                            reduced.before = project(&bad.before, false, 0x7fc0_0001).unwrap().0;
                            reduced.after_inference =
                                project(&bad.after_inference, true, 0x7fc0_0001).unwrap().0;
                            assert_eq!(
                                observable(&reduced, &asset).unwrap_err().to_string(),
                                error
                            );
                            corruptions += 1;
                        }
                    }
                }
                let mut changed = step.clone();
                let (active, _) = active_weight_ranges(&changed.after_inference).unwrap();
                let index = (active.start - changed.after_inference.mutable_word_base) as usize;
                changed.after_inference.mutable_words[index] ^= 1;
                assert!(identical_rejection(&changed, &asset).contains("weights changed inside"));
                corruptions += 1;
                for case in 0..8 {
                    let mut changed = step.clone();
                    match case {
                        0 => changed.before.tick += 1,
                        1 => changed.after_inference.logical_dispatch_generation += 1,
                        2 => changed.after_inference.active_weight_generation += 1,
                        3 => changed.after_inference.active_weight_bank ^= 1,
                        4 => changed.after_inference.active_activation_side ^= 1,
                        5 => changed.before.active_activation_side = 2,
                        6 => changed.before.active_weight_bank = 2,
                        7 => changed.after_inference.structural_synapses.push(
                            alife_gpu_backend::training_rollout::GpuTrainingStructuralSynapse {
                                slot_index: 0,
                                source: 0,
                                target: 0,
                                route: 0,
                                genetic_weight: 0.0,
                                alpha: -0.0,
                            },
                        ),
                        _ => unreachable!(),
                    }
                    identical_rejection(&changed, &asset);
                    metadata_rejections += 1;
                }
                let mut changed = step.clone();
                let (r, _) = active_weight_ranges(&changed.before).unwrap();
                let i = (r.start - changed.before.mutable_word_base) as usize;
                changed.before.mutable_words[i] = 0.0_f32.to_bits();
                changed.after_inference.mutable_words[i] = (-0.0_f32).to_bits();
                let error = observable(&changed, &asset).unwrap_err().to_string();
                assert!(error.contains("weights changed inside"));
                let mut reduced = changed.clone();
                reduced.before = project(&changed.before, false, 0x7fc0_0001).unwrap().0;
                reduced.after_inference = project(&changed.after_inference, true, 0x7fc0_0001)
                    .unwrap()
                    .0;
                assert_eq!(observable(&reduced, &asset).unwrap_err().to_string(), error);
                corruptions += 1;
                cases.push(
                    serde_json::json!({"activation_side":activation_side,"weight_bank":weight_bank,
            "microsteps":microsteps,"selected_pair_bytes":pair_bytes,"selector_independent_pair_bytes":independent_bytes}),
                );
            }
        }
    }
    assert_eq!(base.before.mutable_words.len() * 4, 886016);
    #[cfg(not(windows))]
    {
        let p = base.before.phenotype.as_ref();
        let source = FoundationReplaySource {
            source_revision: "synthetic-platform-admission-probe".into(),
            policy_version: 7,
            actor_checkpoint_digest: Blake3Digest::from_bytes([1; 32]),
            foundation_asset_digest: asset.digest(),
            phenotype_hash: p.phenotype_hash(),
            compiler_inputs_digest: p.compiler_inputs_digest(),
        };
        let root =
            std::env::temp_dir().join(format!("alife-selective-platform-{}", std::process::id()));
        assert!(!root.exists());
        let error = FoundationReplayWriter::new(
            &root,
            source,
            p.clone(),
            &asset,
            FoundationReplayBudget::default(),
        )
        .err()
        .expect("Linux native writer must retain its Windows RAM requirement");
        assert!(error.to_string().contains("Windows process-memory"));
        assert!(!root.exists());
    }
    println!(
        "{}",
        serde_json::json!({"assay":"selective_capture_projection_falsification","device_created":false,
        "synthetic_snapshots":true,"banks_and_indices_have_distinct_values":true,"omitted_pre_post_poisons_differ":true,"full_pair_bytes":base.before.mutable_words.len()*8,
        "accepted_poison_cases":accepted,"retained_corruption_rejections":corruptions,"metadata_rejections":metadata_rejections,"cases":cases,"packet_plans":packets,
        "continuity_cases":cases.len()*8,"gpu_copy_or_mapper_tested":false,"windows_ram_admission_tested":false,
        "cohort_writer_encoding_tested":true,"production_storage_decoder_tested":true,"late_storage_corruption_rejected":true,
        "memory_guard_stubbed":false,"native_writer_platform_rejection_tested":!cfg!(windows),
        "valid_structural_growth_tested":false,"gpu_fail_stop_executed":false})
    );
}
