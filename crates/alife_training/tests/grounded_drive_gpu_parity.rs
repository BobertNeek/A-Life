//! Finite synthetic software-kernel qualification, not a world or training run.
//! Context: https://rikiyat.github.io/blog/weight-space-programming/
#![cfg(all(feature = "cpu-backend", feature = "gpu-tests"))]
use alife_core::*;
use alife_gpu_backend::training_rollout::GpuTrainingStateSnapshot;
use alife_gpu_backend::*;
use alife_runtime::canonical_preparation::*;
use alife_training::cpu::*;
use alife_training::*;
use std::ops::Range;

const ABS: f32 = 2e-5;
const REL: f32 = 3e-4;
const FRAMES: usize = 6;

fn fixture(
    seed: u64,
    profile: SensorProfile,
    maturation: f32,
) -> (BrainPhenotype, FoundationWeightAsset) {
    let g = BrainGenome::scaffold(seed, BrainCapacityClass::N2048_GROUNDED_DRIVE_RESEARCH_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(maturation).unwrap());
    let p = PhenotypeCompiler::compile_n2048_grounded_drive_basis(g.clone(), d.clone(), profile)
        .unwrap()
        .0;
    let mut weights: Vec<_> = p.synapses().iter().map(|s| s.genetic_weight()).collect();
    let mut count = 0;
    for (i, s) in p.synapses().iter().enumerate() {
        if let CompiledSynapseKind::Decoder(c) = s.kind() {
            if c.head() == DecoderHeadKind::ActionCandidate && (36..46).contains(&c.input_lane()) {
                // Hand-authored finite stress values, not a biological recipe or learned asset.
                let magnitude =
                    (1 + (usize::from(c.motor_index()) + seed as usize) % 5) as f32 / 32.;
                weights[i] = if c.input_lane() % 2 == 0 {
                    magnitude
                } else {
                    -magnitude
                };
                count += 1;
            }
        }
    }
    assert_eq!(count, 1280);
    let asset = FoundationWeightAsset::from_trained_weights(
        &p,
        weights,
        TrainingStageManifest::bootstrap(),
    )
    .unwrap();
    let p = PhenotypeCompiler::compile_n2048_grounded_drive_basis_candidate(g, d, asset.clone())
        .unwrap()
        .0;
    asset.validate_against(&p).unwrap();
    assert!(p.cognitive_decoder().is_none());
    assert_eq!(p.synapses().len(), 34048);
    (p, asset)
}

fn frame(
    p: &BrainPhenotype,
    id: OrganismId,
    variant: usize,
    step: usize,
) -> (
    PerceptionFrame,
    FinalizedMemoryRecall,
    NeuralReceptorEffects,
) {
    let tick = Tick::new(step as u64 + 1);
    let mut channels = SensoryChannels::ZERO;
    channels.auditory_acoustic[0] = (step + 1) as f32 / 10.;
    channels.novelty_signal = NormalizedScalar::new(0.3).unwrap();
    let mut sensory =
        SensorySnapshot::new(id, tick, Vec3f::ZERO, channels, Default::default()).unwrap();
    if variant == 2 {
        sensory.social_context.nearest_agents[0] = Some(SocialAgentSnapshot {
            agent_id: OrganismId(1200),
            body_entity: Some(WorldEntityId(200)),
            relative_position: Vec3f::new(0.25, 0., 0.),
            gaze_direction: Vec3f::new(1., 0., 0.),
            orientation_forward: Vec3f::new(1., 0., 0.),
            affinity: SignedValence::new(0.).unwrap(),
            proximity: NormalizedScalar::new(0.75).unwrap(),
        });
    }
    let sign = if (step + variant).is_multiple_of(2) {
        1.
    } else {
        -1.
    };
    let slots: Vec<_> = (0..3)
        .map(|i| GroundedObjectSlotV1 {
            slot_index: i,
            tracked_object_id: TrackedObjectId(100 + u64::from(i)),
            bearing: [0.2 + f32::from(i) * 0.1, -0.1],
            distance: 0.25,
            relative_velocity: [0.; 3],
            color: [0.2, 0.4, 0.6],
            material: [0.1; 3],
            shape: [0.3; 3],
            chemical: [
                if i == 0 {
                    sign * 0.7
                } else if i == 1 {
                    -sign * 0.7
                } else {
                    0.
                },
                0.,
                0.,
            ],
            contact: if i == 0 {
                if step == 5 {
                    0.
                } else {
                    1.
                }
            } else {
                0.
            },
            proprioception: [0.; 2],
            temperature: if i == 0 {
                sign * 0.8
            } else if i == 1 {
                -sign * 0.8
            } else {
                0.1
            },
            terrain: [0.; 2],
            confidence: Confidence::new(0.8).unwrap(),
        })
        .collect();
    let candidates = (0..6)
        .map(|i| {
            let (kind, family) = match (variant, i) {
                (1, 0) => (ActionKind::Interact, CandidateActionFamily::Ingest),
                (1, 1) => (ActionKind::Move, CandidateActionFamily::Approach),
                (1, 2) => (ActionKind::Move, CandidateActionFamily::Avoid),
                (2, 0) => (ActionKind::Interact, CandidateActionFamily::Contact),
                (_, 0..=2) => (ActionKind::Inspect, CandidateActionFamily::Inspect),
                (_, 3) => (ActionKind::Idle, CandidateActionFamily::Idle),
                (_, 4) => (ActionKind::Vocalize, CandidateActionFamily::Other),
                _ => (ActionKind::Rest, CandidateActionFamily::Rest),
            };
            ActionCandidate::new(
                i as u16,
                kind.canonical_id(),
                kind,
                family,
                if i < 3 {
                    CandidateObservationRef::ObjectSlot(i as u16)
                } else {
                    CandidateObservationRef::None
                },
                if i < 3 {
                    ActionTarget::new(
                        Some(WorldEntityId(200 + i as u64)),
                        Some(Vec3f::new(i as f32, 0., 0.)),
                    )
                } else {
                    ActionTarget::NONE
                },
                if i < 3 {
                    slots[i].candidate_features().unwrap()
                } else {
                    CandidateFeatureVector::zero()
                },
                Confidence::new(0.8).unwrap(),
                NormalizedScalar::new(0.1).unwrap(),
                DurationTicks::new(1),
                DurationTicks::new(1),
            )
            .unwrap()
        })
        .collect();
    let draft = PerceptionFrameDraft::new(
        id,
        tick,
        p.sensor_profile(),
        sensory,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        candidates,
        SensorProfileProvenance::new(p.sensor_profile(), SensoryAbiVersion::CURRENT, tick).unwrap(),
        slots,
    )
    .unwrap();
    // Fresh empty bank only. No retained bank/checkpoint or world is read.
    let bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.).unwrap()).unwrap(),
    )
    .unwrap();
    let recall = bank.recall_frame(&draft).unwrap();
    let topology = TopologySidecar::new_profiled(
        id,
        draft.profile_provenance().identity(),
        TopologicalMapConfig::default(),
    )
    .unwrap();
    let evidence = topology_evidence_for_draft(&draft, &topology).unwrap();
    let mut projection = cognitive_projection_for_draft(
        &draft,
        &recall,
        ExperienceSequenceId(step as u64 + 1),
        &predictor_for_phenotype(p).unwrap(),
        &evidence,
    )
    .unwrap();
    for c in &mut projection.candidates {
        c.prediction.predicted_successor.fill(0.42);
    }
    let context =
        cognitive_memory_context_for_recall(id, ExperienceSequenceId(step as u64 + 1), &recall)
            .unwrap();
    let context = cognitive_context_with_projection(context, projection).unwrap();
    let (frame, recall) = recall
        .with_cognitive_context(context)
        .unwrap()
        .finalize(draft)
        .unwrap();
    let effects = NeuralReceptorEffects {
        source_tick: tick,
        source_chemistry_version: 1,
        interoceptive_gain: 0.9,
        regional_excitability: 0.95,
        projection_gain: if variant == 0 { 0.5 } else { 0.85 },
        local_threshold_shift: if variant == 0 {
            0.25
        } else {
            step as f32 / 1000.
        },
        attention_gain: 0.8,
        plasticity_appetitive: 0.,
        plasticity_aversive: 0.,
        structural_growth_gate: 0.,
        sleep_gate: 0.,
        consolidation_gate: 0.,
    };
    (frame, recall, effects)
}

fn words(s: &GpuTrainingStateSnapshot, range: &Range<u32>) -> Vec<f32> {
    s.mutable_words
        [(range.start - s.mutable_word_base) as usize..(range.end - s.mutable_word_base) as usize]
        .iter()
        .copied()
        .map(f32::from_bits)
        .collect()
}
fn neural(s: &GpuTrainingStateSnapshot) -> TrainingInitialState {
    let r = s.brain_slot.word_ranges();
    let home = words(s, &r.homeostasis_words);
    TrainingInitialState {
        activations: words(
            s,
            if s.active_activation_side == 0 {
                &r.activation_a_words
            } else {
                &r.activation_b_words
            },
        ),
        activity_ema: home.iter().step_by(2).copied().collect(),
        metabolic_load: home.iter().skip(1).step_by(2).copied().collect(),
        dendrites: s.v11.dendritic_branches.clone(),
    }
}
fn compare(label: &str, cpu: &[f32], gpu: &[f32], max_error: &mut f32) {
    assert_eq!(cpu.len(), gpu.len(), "{label} length");
    for (i, (&a, &b)) in cpu.iter().zip(gpu).enumerate() {
        let error = (a - b).abs();
        *max_error = max_error.max(error);
        assert!(
            a.is_finite() && b.is_finite() && error <= ABS + REL * a.abs().max(b.abs()),
            "{label}[{i}] cpu={a} software_gpu={b} error={error}"
        );
    }
}

#[test]
#[ignore = "explicit finite SwiftShader qualification; no physical GPU or gameplay claim"]
fn priced_grounded_drive_software_kernels_match_cpu() {
    let (control, _) = fixture(0x2048_2026, SensorProfile::GroundedObjectSlotsV1, 0.5);
    let mut ordinary =
        GpuClosedLoopBackend::new_software_vulkan_runtime_test(GpuRuntimeProfile::production_v1())
            .unwrap();
    let before = ordinary.admission_receipt().clone();
    assert!(ordinary
        .insert_research_brain(OrganismId(100), control)
        .is_err());
    assert_eq!(&before, ordinary.admission_receipt());
    assert_eq!(ordinary.allocated_class_arena_count_for_test(), 0);
    assert!(!ordinary.contains_organism_for_test(OrganismId(100)));
    drop(ordinary);
    let mut gpu = GpuClosedLoopBackend::new_software_vulkan_grounded_drive_research_test(
        GpuRuntimeProfile::production_v1(),
    )
    .unwrap();
    let info = gpu.software_runtime_test_adapter_info().unwrap();
    assert_eq!(info.device_type, wgpu::DeviceType::Cpu);
    assert_eq!(info.backend, wgpu::Backend::Vulkan);
    eprintln!(
        "software_adapter={info:?} hardware_receipt={:?}",
        gpu.hardware_receipt()
    );
    let g = BrainGenome::scaffold(1, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let old = PhenotypeCompiler::compile_n2048_bounded_founder(
        g,
        d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
    .0;
    let before = gpu.admission_receipt().clone();
    assert!(gpu.insert_brain(OrganismId(101), old).is_err());
    assert_eq!(&before, gpu.admission_receipt());
    assert_eq!(gpu.allocated_class_arena_count_for_test(), 0);
    assert!(!gpu.contains_organism_for_test(OrganismId(101)));
    let mut next_dispatch_generation = 1;
    for variant in 0..3 {
        let profile = if variant == 2 {
            SensorProfile::GroundedTerrainVisionV1
        } else {
            SensorProfile::GroundedObjectSlotsV1
        };
        let (p, asset) = fixture(
            0x2048_2026 + variant as u64,
            profile,
            if variant == 0 { 0.5 } else { 1. },
        );
        let mask = StageTrainableMask::from_synapse_indices(&p, &[0]).unwrap();
        let cpu = CpuFoundationTrainer::new(p.clone(), asset.clone(), mask, AdamWConfig::default())
            .unwrap();
        let checkpoint = cpu.checkpoint();
        let id = OrganismId(900 + variant as u64);
        let handle = gpu.insert_research_brain(id, p.clone()).unwrap();
        let initial = gpu.capture_training_state(handle, Tick::ZERO).unwrap();
        let mut cpu_initial = neural(&initial);
        let mut max_logit = 0_f32;
        let mut max_neural = 0_f32;
        let mut max_eligibility = 0_f32;
        let mut nonzero_eligibility = 0;
        let mut derived_effect = 0_f32;
        for step in 0..FRAMES {
            let (f, r, e) = frame(&p, id, variant, step);
            let upload = gpu
                .prepare_memory_context_upload(handle, &f, &r)
                .unwrap()
                .bind_neural_receptor_effects(e)
                .unwrap();
            let bound_effects = upload.neural_receptor_effects().unwrap();
            assert_eq!(bound_effects, e);
            // Neutral timing/queue pressure makes this a repeatable numerical
            // fixture, not a performance assay. Actual ATP and causal source
            // identity are retained, and the native kernels still execute.
            let activity = gpu.snapshot_activity_state(handle).unwrap();
            let prior = activity.pressure;
            let pressure = GpuPressureSample::try_new(
                &BrainActivityPolicyV1::grounded_drive_research_v1(),
                GpuPressureSampleInput {
                    identity: BrainDispatchIdentity {
                        organism_id_raw: id.raw(),
                        tick: f.tick().raw(),
                        class_id_raw: handle.class_id().raw(),
                        handle_slot: handle.slot(),
                        handle_generation: handle.generation(),
                        sequence_cursor: activity.next_sequence_cursor,
                        dispatch_generation: next_dispatch_generation,
                        frame_digest: f.frame_digest().0,
                    },
                    source_dispatch_generation: prior.map_or(0, |p| p.dispatch_generation),
                    source_frame_digest: prior.map_or([0; 4], |p| p.frame_digest),
                    completed_gpu_time_ns: 0,
                    queue_depth: 0,
                    logical_heap_used: 0,
                    logical_heap_capacity: 1,
                    brain_atp_remaining_q16: activity.brain_atp_q16,
                    brain_atp_capacity_q16: BRAIN_ATP_Q16_MAX,
                },
            )
            .unwrap();
            gpu.install_recorded_pressure_replay(vec![pressure])
                .unwrap();
            let batch = GpuClosedLoopMemoryBatchInput::try_new(vec![
                GpuClosedLoopMemoryTickInput::try_new(handle, &f, &upload).unwrap(),
            ])
            .unwrap();
            let tick = gpu
                .tick_memory_batch_training(
                    &batch,
                    &[GpuTrainingSamplingConfig {
                        seed: 9000 + variant as u32,
                        counter: step as u32,
                        temperature: 1.,
                        demonstrator: None,
                    }],
                )
                .unwrap()
                .remove(0);
            next_dispatch_generation = tick.dispatch_generation + 1;
            assert_eq!(tick.throttle.microsteps, if variant == 0 { 3 } else { 4 });
            assert_eq!(gpu.recorded_pressure_replay_remaining(), 0);
            eprintln!("bound_receptor_effects variant={variant} step={step} effects={bound_effects:?} actual_microsteps={}", tick.throttle.microsteps);
            assert_eq!(
                tick.throttle.policy_digest,
                BrainActivityPolicyV1::grounded_drive_research_v1().policy_digest
            );
            let rollout = tick.training_rollout.as_ref().unwrap();
            let mut encoded = encode_cpu_frame(
                &p,
                &f,
                &r,
                &e,
                p.projections()
                    .iter()
                    .map(|route| {
                        tick.throttle
                            .enabled_route_ids
                            .contains(&route.route_index())
                    })
                    .collect(),
                vec![],
            )
            .unwrap();
            encoded.microstep_count = u32::from(tick.throttle.microsteps);
            assert!(encoded.microstep_count > 0);
            assert_eq!(rollout.decoder_input_stride, 54);
            let derived = prepare_grounded_drive_features(&f).unwrap();
            for (i, c) in encoded.candidates.iter().enumerate() {
                let actual = &rollout.decoder_inputs[i * 54..(i + 1) * 54];
                assert_eq!(
                    actual.iter().map(|v| v.to_bits()).collect::<Vec<_>>(),
                    c.decoder_inputs.map(f32::to_bits),
                    "exact final decoder lanes variant={variant} step={step} candidate={i}"
                );
                assert_eq!(&actual[..24], &f.candidates()[i].features.0);
                assert_eq!(&actual[36..46], &derived[i]);
                assert_eq!(&actual[46..49], &[0.42; 3]);
                assert_eq!(actual[53], 0.);
            }
            assert_eq!(derived[0][1], derived[1][2]);
            assert_eq!(derived[0][3], derived[1][4]);
            assert_eq!(&derived[2][1..7], &[0.; 6]);
            assert_eq!(derived[3][0], 0.);
            assert_eq!(derived[4][6], 1.);
            assert_eq!(derived[0][5], f32::from(variant == 2));
            assert_eq!(derived[1][5], 0.);
            assert!(derived.iter().all(|row| row[8..] == [0.; 2]));
            let q = TrainingSequence {
                phenotype_hash: p.phenotype_hash(),
                initial: cpu_initial.clone(),
                ticks: vec![encoded.clone()],
                burn_in_ticks: 0,
                memory_candidate_gain: p
                    .candidate_decoder()
                    .memory_channel()
                    .map_or(0., |m| m.max_candidate_gain()),
            };
            let expected = cpu.evaluate_replay(&q).unwrap();
            compare(
                "candidate_logits",
                &expected.candidate_logits[0],
                &rollout.logits,
                &mut max_logit,
            );
            let mut ablated = q.clone();
            for c in &mut ablated.ticks[0].candidates {
                c.decoder_inputs[36..46].fill(0.);
            }
            let ablated = cpu.evaluate_replay(&ablated).unwrap();
            for (a, b) in expected.candidate_logits[0]
                .iter()
                .zip(&ablated.candidate_logits[0])
            {
                derived_effect = derived_effect.max((a - b).abs());
            }
            let state = gpu.capture_training_state(handle, f.tick()).unwrap();
            let actual = neural(&state);
            compare(
                "activations",
                &expected.final_activations[0],
                &actual.activations,
                &mut max_neural,
            );
            compare(
                "activity_ema",
                &expected.final_activity_ema[0],
                &actual.activity_ema,
                &mut max_neural,
            );
            compare(
                "metabolic_load",
                &expected.final_metabolic_load[0],
                &actual.metabolic_load,
                &mut max_neural,
            );
            compare(
                "encoded_inputs",
                &encoded.encoded_inputs,
                &words(&state, &state.brain_slot.word_ranges().encoded_input_words),
                &mut max_neural,
            );
            // Every pending trace is discarded; committed traces remain zero. Check
            // actual inactive GPU eligibility against native feature × motor algebra.
            let ranges = state.brain_slot.word_ranges();
            assert!(words(&state, &ranges.decoder_eligibility_words)
                .iter()
                .all(|v| *v == 0.));
            let staged = words(&state, &ranges.decoder_eligibility_bank_1_words);
            let mut selected: Vec<usize> = rollout
                .motor_indices
                .iter()
                .filter(|i| **i != u16::MAX)
                .map(|i| usize::from(*i))
                .collect();
            selected.sort_unstable();
            selected.dedup();
            assert!(selected.contains(&usize::from(rollout.representative_index)));
            let mut decoder_index = 0;
            for s in p.synapses() {
                if let CompiledSynapseKind::Decoder(c) = s.kind() {
                    if c.head() == DecoderHeadKind::ActionCandidate
                        && (36..46).contains(&c.input_lane())
                    {
                        let value: f32 = selected
                            .iter()
                            .filter(|&&i| encoded.candidates[i].family == c.family())
                            .map(|&i| {
                                encoded.candidates[i].decoder_inputs[c.input_lane() as usize]
                                    * expected.final_activations[0][s.source() as usize]
                            })
                            .sum();
                        let value = value.clamp(-1., 1.);
                        if value.abs() > 1e-6 {
                            nonzero_eligibility += 1;
                        }
                        compare(
                            "derived_eligibility",
                            &[value],
                            &[staged[decoder_index]],
                            &mut max_eligibility,
                        );
                    }
                    decoder_index += 1;
                }
            }
            cpu_initial = TrainingInitialState {
                activations: expected.final_activations[0].clone(),
                activity_ema: expected.final_activity_ema[0].clone(),
                metabolic_load: expected.final_metabolic_load[0].clone(),
                dendrites: cpu_initial.dendrites,
            };
            gpu.discard_pending_eligibility(handle, tick.pending_eligibility.identity())
                .unwrap();
            eprintln!("frame variant={variant} step={step} microsteps={} max_logit={max_logit:e} max_neural={max_neural:e} max_eligibility={max_eligibility:e}",tick.throttle.microsteps);
        }
        assert!(
            derived_effect > 1e-5,
            "nonzero added weights must affect actual candidate logits"
        );
        assert!(
            nonzero_eligibility > 0,
            "derived eligibility must actually execute"
        );
        assert_eq!(checkpoint, cpu.checkpoint());
        gpu.remove_brain(handle).unwrap();
        eprintln!("PARITY_RESULT variant={variant} frames={FRAMES} profile={profile:?} phenotype={:?} asset={:?} tolerance_abs={ABS} tolerance_rel={REL} max_logit={max_logit:e} max_neural={max_neural:e} max_eligibility={max_eligibility:e} nonzero_derived_eligibility={nonzero_eligibility} derived_ablation_effect={derived_effect:e} optimizer_unchanged=true credit_applied=false",p.phenotype_hash(),asset.digest());
    }
    assert_eq!(gpu.admission_receipt().live_brains, 0);
}
