//! Adapter-free experiment; source fixture matches the eligible 893e6395 assay.
use super::super::{build_step_headers, pack_metadata_and_layout};
use super::*;
use crate::{
    AdamWConfig, PpoBatch, PpoBoundary, PpoConfig, PpoJointAction, PpoTransition, TrainingError,
    TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick, TrainingSequence,
};
use alife_core::BrainPhenotype;
use std::{hint::black_box, sync::OnceLock, time::Instant};

#[path = "cohort_packing_bench.rs"]
mod experiment;

fn phenotype() -> &'static BrainPhenotype {
    static P: OnceLock<BrainPhenotype> = OnceLock::new();
    P.get_or_init(|| {
        use alife_core::{
            BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
            SensorProfile, Tick,
        };
        let capacity = BrainCapacityClass::n2048();
        let genome = BrainGenome::scaffold(99, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
        PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedTerrainVisionV1,
        )
        .unwrap()
    })
}

fn sequence(phenotype: &BrainPhenotype, ticks: usize, burn: usize) -> TrainingSequence {
    let neurons = phenotype.neuron_count() as usize;
    let dendrites = alife_gpu_backend::GpuV11Checkpoint::canonical_for_phenotype(phenotype)
        .unwrap()
        .dendritic_branches;
    let replay = TrainingSequence {
        phenotype_hash: phenotype.phenotype_hash(),
        initial: TrainingInitialState {
            activations: (0..neurons).map(|i| (i % 17) as f32 * 0.01).collect(),
            activity_ema: vec![0.2; neurons],
            metabolic_load: vec![0.03; neurons],
            dendrites,
        },
        ticks: (0..ticks)
            .map(|tick| TrainingReplayTick {
                encoded_inputs: (0..neurons)
                    .map(|i| ((i + tick) % 11) as f32 * 0.015)
                    .collect(),
                projection_gain: 0.85,
                local_threshold_shift: 0.017,
                microstep_count: u32::from(phenotype.microstep_count()),
                enabled_routes: vec![true; phenotype.projections().len()],
                effective_weight_offsets: (0..phenotype.synapses().len())
                    .map(|i| ((i + tick) % 7) as f32 * 0.002)
                    .collect(),
                structural_synapses: Vec::new(),
                candidates: (0..25)
                    .map(|i| TrainingReplayCandidate {
                        family: [
                            alife_core::CandidateActionFamily::Approach,
                            alife_core::CandidateActionFamily::Ingest,
                            alife_core::CandidateActionFamily::Idle,
                        ][i % 3],
                        decoder_inputs: std::array::from_fn(|lane| {
                            ((i + lane + tick) % 5) as f32 * 0.03
                        }),
                        innate_bias: 0.04,
                    })
                    .collect(),
            })
            .collect(),
        burn_in_ticks: burn,
        memory_candidate_gain: phenotype
            .candidate_decoder()
            .memory_channel()
            .map_or(0.0, |channel| channel.max_candidate_gain()),
    };
    replay.validate_for(phenotype).unwrap();
    replay
}

fn window(replay: TrainingSequence) -> Arc<PpoTrainingWindow> {
    let rows = replay.ticks.len() - replay.burn_in_ticks;
    let candidates = replay.ticks[replay.burn_in_ticks].candidates.len();
    let config = PpoConfig::default();
    let transitions = (0..rows)
        .map(|index| PpoTransition {
            policy_version: 7,
            trajectory_id: 1,
            step: index as u64,
            action: PpoJointAction {
                candidate_count: candidates as u16,
                representative_mask: u32::MAX >> (32 - candidates),
                motor_masks: [0; 6],
                representative: 0,
                forced_slots: vec![None; candidates],
                motor_candidates: [None; 6],
                old_joint_log_probability: -(candidates as f32).ln(),
                temperature: config.temperature,
            },
            reward: 0.1,
            old_value: 0.0,
            next_value: 0.0,
            elapsed_seconds: 0.05,
            boundary: if index + 1 == rows {
                PpoBoundary::Terminated
            } else {
                PpoBoundary::Continuing
            },
        })
        .collect();
    Arc::new(PpoTrainingWindow {
        speech_targets: vec![None; replay.ticks.len()],
        sequence: replay,
        batch: PpoBatch::from_rollout(7, transitions, config).unwrap(),
        auxiliary: None,
    })
}

fn checked<'a>(
    owner: &Arc<PpoTrainingWindow>,
    layout: PackedLayout,
    cache: &'a mut CohortPackedReplay,
    admit: &mut dyn FnMut(u64) -> bool,
) -> Result<Cow<'a, [u32]>, TrainingError> {
    // The experiment preserves fresh validation, before cache access/admission.
    owner.sequence.validate_for(phenotype())?;
    Ok(cache.words(owner, layout, phenotype().neuron_count(), admit))
}

fn stats(cache: &CohortPackedReplay) -> serde_json::Value {
    serde_json::json!({
        "packets":cache.packets.len(),
        "packet_length_bytes":cache.packets.iter().map(|p| p.words.len()*4).sum::<usize>(),
        "packet_capacity_bytes":cache.packets.iter().map(|p| p.words.capacity()*4).sum::<usize>(),
        "metadata_capacity_bytes":cache.packets.capacity()*size_of::<Packet>(),
        "conservative_charge_bytes":cache.charged_bytes,
        "byte_limit":cache.byte_limit,"disabled":cache.disabled,
    })
}

#[test]
fn varied_shapes_topology_and_context_match_production_packing() {
    use alife_core::{DendriticBranch, DendriticBranchSet, DendriticInputRef};
    let p = phenotype();
    let mut cases = 0;
    for (ticks, burn) in [(1, 0), (3, 0), (4, 2)] {
        for topology in 0..3 {
            for candidates in [1, 25, 32] {
                for structural in [false, true] {
                    let mut q = sequence(p, ticks, burn);
                    if topology == 1 {
                        q.initial.dendrites = Default::default();
                    }
                    if topology == 2 {
                        q.initial.dendrites = DendriticBranchSet::new(vec![DendriticBranch::new(
                            35,
                            -0.0,
                            0.4,
                            vec![DendriticInputRef::new(6, -0.0).unwrap()],
                        )
                        .unwrap()])
                        .unwrap();
                    }
                    for (i, tick) in q.ticks.iter_mut().enumerate() {
                        let candidate = tick.candidates[0].clone();
                        tick.candidates.resize(candidates, candidate);
                        tick.microstep_count = if i % 2 == 0 {
                            0
                        } else {
                            u32::from(p.microstep_count())
                        };
                        tick.enabled_routes[0] = false;
                        tick.effective_weight_offsets[0] = -0.0;
                        if structural {
                            tick.structural_synapses.push(crate::TrainingFrozenSynapse {
                                source: 1,
                                target: 35,
                                route: 0,
                                cadence: u32::from(p.projections()[0].update_cadence().raw()),
                                effective_weight: -0.125,
                            });
                        }
                    }
                    q.validate_for(p).unwrap();
                    let owner = window(q);
                    let (_, layout) = pack_metadata_and_layout(p, Some(&owner.sequence)).unwrap();
                    let expected = pack_replay_sequence(&owner.sequence, layout, p.neuron_count());
                    let mut cache = CohortPackedReplay::new(1);
                    let first = checked(&owner, layout, &mut cache, &mut |_| true).unwrap();
                    assert!(matches!(first, Cow::Borrowed(_)));
                    assert_eq!(&*first, expected);
                    let pointer = first.as_ptr();
                    drop(first);
                    let second = checked(&owner, layout, &mut cache, &mut |_| true).unwrap();
                    assert_eq!(second.as_ptr(), pointer);
                    assert_eq!(&*second, expected);
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 54);
}

#[test]
fn headers_refresh_and_base_packet_clears_simulated_speech_upload() {
    let p = phenotype();
    let owner = window(sequence(p, 4, 0));
    let (_, mut layout) = pack_metadata_and_layout(p, Some(&owner.sequence)).unwrap();
    let mut cache = CohortPackedReplay::new(1);
    let words = checked(&owner, layout, &mut cache, &mut |_| true).unwrap();
    let mut simulated_upload = words.to_vec();
    let pointer = words.as_ptr();
    drop(words);
    let before = build_step_headers(p, layout, AdamWConfig::default(), 16_777_219).unwrap();
    simulated_upload[layout.replay_speech_targets as usize..].fill(0xdead_beef);
    layout.burn_in_steps = u32::from(p.microstep_count()) * 2;
    layout.memory_gain = 0.625;
    let after = build_step_headers(p, layout, AdamWConfig::default(), 16_777_220).unwrap();
    let words = checked(&owner, layout, &mut cache, &mut |_| true).unwrap();
    assert_eq!(words.as_ptr(), pointer);
    // CPU sink models a complete base upload; this is not a wgpu execution test.
    simulated_upload.copy_from_slice(&words);
    assert!(simulated_upload[layout.replay_speech_targets as usize..]
        .iter()
        .all(|w| *w == 0));
    assert_eq!(
        simulated_upload,
        pack_replay_sequence(&owner.sequence, layout, p.neuron_count())
    );
    assert_eq!(before[10], 16_777_219);
    assert_eq!(after[10], 16_777_220);
    assert_eq!(after[56], 0.625_f32.to_bits());
    assert_eq!(after[57], 8);
}

#[test]
fn arc_mutation_replacement_and_expiry_force_fresh_packets() {
    let p = phenotype();
    let mut owner = window(sequence(p, 3, 0));
    let (_, layout) = pack_metadata_and_layout(p, Some(&owner.sequence)).unwrap();
    let mut cache = CohortPackedReplay::new(2);
    let original = checked(&owner, layout, &mut cache, &mut |_| true)
        .unwrap()
        .to_vec();
    let old_address = Arc::as_ptr(&owner);
    assert_eq!(Arc::strong_count(&owner), 1); // Cache retains only a Weak.
    assert!(Arc::get_mut(&mut owner).is_none());
    Arc::make_mut(&mut owner).sequence.ticks[0].encoded_inputs[0] = -0.0;
    assert_ne!(Arc::as_ptr(&owner), old_address);
    let changed = checked(&owner, layout, &mut cache, &mut |_| true).unwrap();
    assert_ne!(&*changed, original);
    assert_eq!(
        &*changed,
        pack_replay_sequence(&owner.sequence, layout, p.neuron_count())
    );
    drop(changed);
    assert_eq!(cache.packets.len(), 1); // Dissociated old Weak was pruned.
    let replacement = window(sequence(p, 3, 0));
    checked(&replacement, layout, &mut cache, &mut |_| true).unwrap();
    assert_eq!(cache.packets.len(), 2);
    drop(owner);
    checked(&replacement, layout, &mut cache, &mut |_| true).unwrap();
    assert_eq!(cache.packets.len(), 1);
    let mut shifted = layout;
    shifted.training_words += 64;
    shifted.inputs += 64;
    shifted.candidate_records += 64;
    shifted.context += 64;
    shifted.initial += 64;
    let words = checked(&replacement, shifted, &mut cache, &mut |_| true).unwrap();
    assert_eq!(
        &*words,
        pack_replay_sequence(&replacement.sequence, shifted, p.neuron_count())
    );
    assert_eq!(words.len(), layout.training_words as usize + 64);
}

#[test]
fn invalid_inputs_fail_before_admission_or_lookup() {
    let p = phenotype();
    let original = window(sequence(p, 3, 0));
    let (_, layout) = pack_metadata_and_layout(p, Some(&original.sequence)).unwrap();
    let mut cache = CohortPackedReplay::new(1);
    checked(&original, layout, &mut cache, &mut |_| true).unwrap();
    let prior = stats(&cache);
    for case in 0..12 {
        let mut bad = (*original).clone();
        match case {
            0 => bad.sequence.ticks.clear(),
            1 => bad.sequence.burn_in_ticks = 3,
            2 => bad.sequence.initial.activations[0] = f32::NAN,
            3 => bad.sequence.initial.activity_ema[0] = 2.0,
            4 => {
                bad.sequence.ticks[0].encoded_inputs.pop();
            }
            5 => bad.sequence.ticks[0].effective_weight_offsets[0] = f32::INFINITY,
            6 => bad.sequence.ticks[0].candidates[0].innate_bias = f32::NEG_INFINITY,
            7 => bad.sequence.ticks[0].candidates.clear(),
            8 => bad.sequence.ticks[0].enabled_routes.clear(),
            9 => bad.sequence.ticks[0].microstep_count = 255,
            10 => bad.sequence.memory_candidate_gain = -1.0,
            11 => bad.sequence.ticks[0].projection_gain = f32::NAN,
            _ => unreachable!(),
        }
        let expected = TrainingError::from(bad.sequence.validate_for(p).unwrap_err()).to_string();
        let mut admission_calls = 0;
        let error = checked(&Arc::new(bad), layout, &mut cache, &mut |_| {
            admission_calls += 1;
            true
        })
        .unwrap_err();
        assert_eq!(error.to_string(), expected);
        assert_eq!(admission_calls, 0);
        assert_eq!(stats(&cache), prior);
    }
}

#[test]
fn admission_refusal_caps_and_eviction_keep_uncached_results() {
    let p = phenotype();
    let owner = window(sequence(p, 3, 0));
    let (_, layout) = pack_metadata_and_layout(p, Some(&owner.sequence)).unwrap();
    let expected = pack_replay_sequence(&owner.sequence, layout, p.neuron_count());
    for count in [0, 9] {
        let mut cache = CohortPackedReplay::new(count);
        let words = checked(&owner, layout, &mut cache, &mut |_| {
            panic!("disabled admission")
        })
        .unwrap();
        assert!(matches!(words, Cow::Owned(_)));
        assert_eq!(&*words, expected);
        assert_eq!(cache.packets.capacity(), 0);
    }
    let mut cache = CohortPackedReplay::new(2);
    assert!(matches!(
        checked(&owner, layout, &mut cache, &mut |_| false).unwrap(),
        Cow::Owned(_)
    ));
    assert_eq!(cache.charged_bytes, 0);
    let mut cache = CohortPackedReplay::new(2);
    let words = checked(&owner, layout, &mut cache, &mut |n| n < 8192).unwrap();
    assert!(matches!(words, Cow::Owned(_)));
    assert_eq!(&*words, expected);
    drop(words);
    assert!(cache.packets.is_empty());
    checked(&owner, layout, &mut cache, &mut |_| true).unwrap();
    let charged = cache.charged_bytes;
    cache.byte_limit = charged;
    let replacement = Arc::new((*owner).clone());
    let refused = checked(&replacement, layout, &mut cache, &mut |_| true).unwrap();
    assert!(matches!(refused, Cow::Owned(_)));
    assert_eq!(&*refused, expected);
    drop(refused);
    assert_eq!(cache.charged_bytes, charged);
    let evicted = checked(&owner, layout, &mut cache, &mut |_| false).unwrap();
    assert!(matches!(evicted, Cow::Owned(_)));
    assert_eq!(&*evicted, expected);
    drop(evicted);
    assert!(cache.packets.is_empty());
    assert_eq!(cache.charged_bytes, cache.metadata_charge);
}
