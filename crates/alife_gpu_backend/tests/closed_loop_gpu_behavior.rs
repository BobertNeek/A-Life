#![cfg(feature = "gpu-tests")]

mod support;

use alife_core::{
    ActionCandidate, ActionId, ActionKind, ActionTarget, BodySnapshot, BrainPhenotype,
    CandidateActionFamily, CandidateFeatureVector, CandidateObservationRef, Confidence,
    DurationTicks, HomeostaticSnapshot, NormalizedScalar, OrganismId, PerceptionFrame, Pose,
    SensorProfile, SensorProfileProvenance, SensoryAbiVersion, SensoryChannels, SensorySnapshot,
    Tick, Vec3f, Velocity, WorldEntityId,
};
use alife_gpu_backend::{
    CLOSED_LOOP_DECODE_WGSL, GPU_CANDIDATE_RECORD_BYTES, GPU_CLOSED_LOOP_TICK_READBACK_BYTES,
    GPU_SELECTION_RECORD_BYTES,
};
use naga::ShaderStage;

use support::{CompactSelection, GpuFrameResult, GpuPipelineFixture};

#[derive(Clone)]
struct CausalGpuCheckpoint {
    phenotype: BrainPhenotype,
}

struct ReplayReceipt {
    adapter_identity: String,
    selected_candidates: Vec<u32>,
    selected_logits: Vec<f32>,
    dispatch_generations: Vec<u64>,
    tolerance: f32,
}

fn causal_n512_gpu_checkpoint() -> CausalGpuCheckpoint {
    CausalGpuCheckpoint {
        phenotype: support::controlled_sensory_n512_phenotype(),
    }
}

async fn restore_same_adapter_pair_from_checkpoint(
    checkpoint: &CausalGpuCheckpoint,
) -> GpuPipelineFixture {
    GpuPipelineFixture::new(&checkpoint.phenotype).await
}

fn two_candidate_frame(organism_raw: u64, tick_raw: u64, sensory: [f32; 2]) -> PerceptionFrame {
    two_candidate_frame_with_transport(organism_raw, tick_raw, sensory, false)
}

fn two_candidate_frame_with_transport(
    organism_raw: u64,
    tick_raw: u64,
    sensory: [f32; 2],
    alternate_transport: bool,
) -> PerceptionFrame {
    let organism_id = OrganismId(organism_raw);
    let tick = Tick::new(tick_raw);
    let mut channels = SensoryChannels::ZERO;
    channels.visual_affordance[0] = sensory[0].clamp(0.0, 1.0);
    channels.visual_affordance[1] = sensory[1].clamp(0.0, 1.0);
    let sensory_snapshot =
        SensorySnapshot::new(organism_id, tick, Vec3f::ZERO, channels, Default::default()).unwrap();
    let candidates = [
        (ActionKind::Idle, CandidateActionFamily::Idle),
        (ActionKind::Inspect, CandidateActionFamily::Inspect),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (kind, family))| {
        let mut features = CandidateFeatureVector::zero();
        features.0[0] = 1.0;
        features.0[1] = if index == 0 { -0.5 } else { 0.75 };
        let observation = CandidateObservationRef::None;
        let target = if alternate_transport {
            ActionTarget::new(
                Some(WorldEntityId(900 + index as u64)),
                Some(Vec3f::new(index as f32, 0.0, 1.0)),
            )
        } else {
            ActionTarget::NONE
        };
        ActionCandidate::new(
            index as u16,
            if alternate_transport {
                ActionId(700 + index as u32)
            } else {
                kind.canonical_id()
            },
            kind,
            family,
            observation,
            target,
            features,
            Confidence::new(if alternate_transport { 0.6 } else { 0.8 }).unwrap(),
            NormalizedScalar::new(if alternate_transport { 0.3 } else { 0.1 }).unwrap(),
            DurationTicks::new(1),
            DurationTicks::new(1),
        )
        .unwrap()
    })
    .collect();
    PerceptionFrame::new(
        organism_id,
        tick,
        SensorProfile::PrivilegedAffordanceV1,
        sensory_snapshot,
        BodySnapshot {
            pose: Pose::IDENTITY,
            velocity: Velocity::ZERO,
        },
        HomeostaticSnapshot::baseline(tick),
        candidates,
        SensorProfileProvenance::new(
            SensorProfile::PrivilegedAffordanceV1,
            SensoryAbiVersion::CURRENT,
            tick,
        )
        .unwrap(),
        Vec::new(),
    )
    .unwrap()
}

fn assert_base_frames_differ_only_in_sensory_and_transport_identity(
    first: &PerceptionFrame,
    second: &PerceptionFrame,
) {
    assert_ne!(first.organism_id(), second.organism_id());
    assert_eq!(first.tick(), second.tick());
    assert_eq!(first.sensor_profile(), second.sensor_profile());
    assert_eq!(first.body(), second.body());
    assert_eq!(first.homeostasis(), second.homeostasis());
    assert_eq!(first.context(), second.context());
    assert_eq!(first.candidates(), second.candidates());
    assert_eq!(first.sensory().abi_version, second.sensory().abi_version);
    assert_eq!(first.sensory().tick, second.sensory().tick);
    assert_eq!(
        first.sensory().observer_position,
        second.sensory().observer_position
    );
    assert_eq!(
        first.sensory().context_streams,
        second.sensory().context_streams
    );
    assert_eq!(
        first.sensory().social_context,
        second.sensory().social_context
    );
    assert_eq!(
        first.sensory().language_context,
        second.sensory().language_context
    );
    assert_eq!(
        first.sensory().semantic_context,
        second.sensory().semantic_context
    );
    assert_eq!(
        first.sensory().gaussian_context,
        second.sensory().gaussian_context
    );
    assert_ne!(first.sensory().channels, second.sensory().channels);
}

async fn run_fresh_gpu_sequence(
    phenotype: BrainPhenotype,
    frames: &[PerceptionFrame],
) -> ReplayReceipt {
    let mut gpu = GpuPipelineFixture::new(&phenotype).await;
    let mut selected_candidates = Vec::with_capacity(frames.len());
    let mut selected_logits = Vec::with_capacity(frames.len());
    let mut dispatch_generations = Vec::with_capacity(frames.len());
    for frame in frames {
        let result = gpu.run_frame(frame).await;
        assert_eq!(
            result.compact_readback_bytes,
            GPU_CLOSED_LOOP_TICK_READBACK_BYTES as u64
        );
        selected_candidates.push(result.selection.candidate_index);
        selected_logits.push(result.selection.logit);
        dispatch_generations.push(result.selection.dispatch_generation);
    }
    ReplayReceipt {
        adapter_identity: gpu.adapter_identity().to_owned(),
        selected_candidates,
        selected_logits,
        dispatch_generations,
        tolerance: 1.0e-6,
    }
}

fn deterministic_frame_sequence(count: usize, seed: u64) -> Vec<PerceptionFrame> {
    (0..count)
        .map(|index| {
            let lane = ((seed.wrapping_add(index as u64 * 17) & 0xff) as f32) / 255.0;
            let tick = match index % 4 {
                0 | 1 => 900,
                2 => 100,
                _ => 800,
            };
            two_candidate_frame(7, tick, [lane, 1.0 - lane])
        })
        .collect()
}

fn expected_final_side(microsteps: u8) -> u32 {
    u32::from(microsteps & 1)
}

fn expected_candidate_for_side(microsteps: u8) -> u32 {
    expected_final_side(microsteps)
}

fn assert_compact_selection(result: &GpuFrameResult) -> CompactSelection {
    assert_eq!(
        result.compact_readback_bytes,
        GPU_CLOSED_LOOP_TICK_READBACK_BYTES as u64
    );
    assert!(result.selection.logit.is_finite());
    assert_eq!(result.selection.status, 1);
    result.selection
}

#[test]
fn decode_wgsl_parses_and_abi_remains_compact_candidate_conditioned_and_entity_blind() {
    assert_eq!(GPU_CANDIDATE_RECORD_BYTES, 32);
    assert_eq!(GPU_SELECTION_RECORD_BYTES, 48);
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_DECODE_WGSL).unwrap();
    let entries = module
        .entry_points
        .iter()
        .map(|entry| (entry.name.as_str(), entry.stage, entry.workgroup_size))
        .collect::<Vec<_>>();
    assert_eq!(
        entries,
        vec![
            ("decode_candidates", ShaderStage::Compute, [32, 1, 1]),
            ("select_candidate", ShaderStage::Compute, [1, 1, 1]),
        ]
    );
    let decode_body = &CLOSED_LOOP_DECODE_WGSL[CLOSED_LOOP_DECODE_WGSL
        .find("fn decode_candidates")
        .unwrap()..];
    for required in [
        "decoder_plan_offset",
        "candidate_offset",
        "feature_offset",
        "decoder_weight_indices_offset",
        "candidate_logit_offset",
        "selection_offset",
    ] {
        assert!(decode_body.contains(required), "missing {required}");
    }
    assert!(!decode_body.contains("observation_slot_or_max"));
    assert!(!decode_body.contains("entity_id"));
    assert!(!decode_body.contains("inhibition_coefficient"));
    assert!(!decode_body.contains("lateral_inhibition_gain"));
    let header_start = CLOSED_LOOP_DECODE_WGSL
        .find("struct GpuPerceptionHeader")
        .unwrap();
    let header_end = CLOSED_LOOP_DECODE_WGSL[header_start..]
        .find("struct GpuBrainSlotRecord")
        .unwrap()
        + header_start;
    let header = &CLOSED_LOOP_DECODE_WGSL[header_start..header_end];
    assert!(CLOSED_LOOP_DECODE_WGSL.contains("GPU_CLOSED_LOOP_LAYOUT_VERSION:u32 = 3u"));
    assert!(header.contains("dispatch_generation_lo"));
    assert!(header.contains("dispatch_generation_hi"));
    assert!(!header.contains("reserved:array<u32,3>"));
    let compact_decode = decode_body.split_whitespace().collect::<String>();
    assert!(compact_decode.contains("candidate_offset+candidate*8u"));
    assert!(compact_decode.contains("diagnostic_offset+3u"));
    assert!(compact_decode.contains(
        "span_within(brain.recurrent_synapse_count,decoder.decoder_synapse_count,brain.synapse_count)"
    ));
    assert!(!compact_decode.contains(
        "decoder.decoder_synapse_count==brain.synapse_count-brain.recurrent_synapse_count"
    ));
    let support_source = include_str!("support/mod.rs").to_ascii_lowercase();
    for forbidden in ["max_by(", "total_cmp(", "argmax", "cpu_winner"] {
        assert!(!support_source.contains(forbidden));
    }
}

#[test]
fn same_candidates_different_sensory_frames_change_gpu_logits() {
    pollster::block_on(async {
        let checkpoint = causal_n512_gpu_checkpoint();
        let mut gpu = restore_same_adapter_pair_from_checkpoint(&checkpoint).await;
        gpu.configure_controlled_sensory_path_and_decoder(&checkpoint.phenotype);
        let first_frame = two_candidate_frame(7, 77, [0.9, 0.0]);
        let second_frame = two_candidate_frame(9, 77, [0.0, 0.9]);
        assert_base_frames_differ_only_in_sensory_and_transport_identity(
            &first_frame,
            &second_frame,
        );
        assert_eq!(first_frame.candidates(), second_frame.candidates());
        let [first, second] = gpu.run_frame_pair([&first_frame, &second_frame]).await;
        assert_eq!(first.adapter_identity, second.adapter_identity);
        assert_eq!(
            first.selection.dispatch_generation,
            second.selection.dispatch_generation
        );
        assert_ne!(
            assert_compact_selection(&first).logit.to_bits(),
            assert_compact_selection(&second).logit.to_bits()
        );
    });
}

#[test]
fn lesioning_motor_weights_changes_gpu_selection() {
    pollster::block_on(async {
        let phenotype = support::controlled_n512_phenotype_at_maturation(0.2);
        let frame = two_candidate_frame(7, 77, [0.8, -0.3]);
        let mut gpu = GpuPipelineFixture::new(&phenotype).await;
        let adapter = gpu.adapter_identity().to_owned();
        gpu.configure_controlled_motor_loop_and_decoder(&phenotype, true);
        let before = gpu.run_frame(&frame).await.selection;
        assert_eq!(before.candidate_index, 1);
        gpu.restore_mutable_checkpoint();
        gpu.configure_controlled_motor_loop_and_decoder(&phenotype, true);
        let lesion = gpu.lesion_decoder_genetic_weights();
        assert_eq!(lesion.adapter_identity, adapter);
        assert!(lesion.recurrent_prefixes_unchanged);
        assert_eq!(lesion.changed_ranges.len(), 3);
        assert!(lesion
            .changed_ranges
            .iter()
            .all(|range| range.start < range.end));
        assert!(lesion
            .changed_ranges
            .windows(2)
            .all(|pair| pair[0].end <= pair[1].start));
        let after = gpu.run_frame(&frame).await.selection;
        assert_eq!(after.candidate_index, 0);
        assert_ne!(before.candidate_index, after.candidate_index);
    });
}

#[test]
fn zero_neural_weights_remove_non_idle_behavior() {
    pollster::block_on(async {
        let phenotype = support::n512_phenotype(4101);
        let mut gpu = GpuPipelineFixture::new(&phenotype).await;
        gpu.zero_all_mutable_layers_and_assert_biases(&phenotype);
        gpu.set_all_genetic_weights_zeroed(true);
        let result = gpu
            .run_frame(&two_candidate_frame(7, 77, [0.8, -0.3]))
            .await;
        assert_eq!(result.selection.candidate_index, 0);
        assert_eq!(result.selection.logit, 0.0);
        assert_eq!(result.selection.status, 1);
        assert_ne!(result.selection.dispatch_generation, 0);
        assert_ne!(result.selection.dispatch_generation, 77);
        assert_eq!(
            result.compact_readback_bytes,
            GPU_CLOSED_LOOP_TICK_READBACK_BYTES as u64
        );
        assert_eq!(result.record.slot, 0);
        assert_eq!(result.record.slot_generation, 7);
        assert_eq!(result.record.candidate_index, 0);
        assert_eq!(result.record.logit_bits, 0.0_f32.to_bits());
        assert_eq!(
            result.record.confidence_q16,
            (0.8_f32 * 65535.0).round() as u32
        );
        assert_eq!(result.record.status, 1);
        assert_eq!(result.record.finite_rejections, 0);
        assert_eq!(result.record.active_activation_side, 1);
        let expected =
            support::expected_cadence_counts(&phenotype, u32::from(phenotype.microstep_count()));
        assert_eq!(
            (result.record.active_tiles, result.record.active_synapses),
            expected
        );
    });
}

#[test]
fn transport_identity_only_does_not_change_gpu_selection_or_logit() {
    pollster::block_on(async {
        let phenotype = support::n512_phenotype(4101);
        let mut gpu = GpuPipelineFixture::new(&phenotype).await;
        let first = two_candidate_frame(7, 88, [0.6, 0.2]);
        let second = two_candidate_frame_with_transport(9, 88, [0.6, 0.2], true);
        assert_ne!(first.organism_id(), second.organism_id());
        assert_eq!(first.sensory().channels, second.sensory().channels);
        for (a, b) in first.candidates().iter().zip(second.candidates()) {
            assert_eq!(a.family, b.family);
            assert_eq!(a.features, b.features);
            assert_ne!(a.action_id, b.action_id);
            assert_eq!(a.observation, CandidateObservationRef::None);
            assert_eq!(b.observation, CandidateObservationRef::None);
            assert_ne!(a.target, b.target);
            assert_ne!(a.sensor_confidence, b.sensor_confidence);
            assert_ne!(a.required_effort, b.required_effort);
        }
        let [a, b] = gpu.run_frame_pair([&first, &second]).await;
        assert_eq!(a.selection.candidate_index, b.selection.candidate_index);
        assert_eq!(a.selection.logit.to_bits(), b.selection.logit.to_bits());
        assert_eq!(
            a.selection.confidence_q16,
            (0.8_f32 * 65535.0).round() as u32
        );
        assert_eq!(
            b.selection.confidence_q16,
            (0.6_f32 * 65535.0).round() as u32
        );
        assert_eq!(
            a.selection.dispatch_generation,
            b.selection.dispatch_generation
        );
        assert_ne!(a.selection.dispatch_generation, 0);
        assert_ne!(a.selection.dispatch_generation, 88);
    });
}

#[test]
fn all_non_finite_logits_fail_closed_with_explicit_status() {
    pollster::block_on(async {
        let phenotype = support::n512_phenotype(4101);
        let mut gpu = GpuPipelineFixture::new(&phenotype).await;
        gpu.set_decoder_genetic_weights_non_finite();
        let result = gpu
            .run_frame(&two_candidate_frame(7, 99, [0.8, -0.3]))
            .await;
        assert_eq!(result.selection.status, 2);
        assert_eq!(result.selection.candidate_index, u32::MAX);
        assert_eq!(result.selection.logit.to_bits(), 0.0_f32.to_bits());
        assert_eq!(result.selection.confidence_q16, 0);
        assert_ne!(result.selection.dispatch_generation, 0);
        assert_ne!(result.selection.dispatch_generation, 99);
        assert_eq!(
            result.compact_readback_bytes,
            GPU_CLOSED_LOOP_TICK_READBACK_BYTES as u64
        );
        assert_eq!(result.record.slot, 0);
        assert_eq!(result.record.slot_generation, 7);
        assert_eq!(result.record.finite_rejections, 2);
        assert_eq!(result.record.active_activation_side, 1);
        let expected =
            support::expected_cadence_counts(&phenotype, u32::from(phenotype.microstep_count()));
        assert_eq!(
            (result.record.active_tiles, result.record.active_synapses),
            expected
        );
    });
}

#[test]
fn same_adapter_replay_matches_the_declared_tolerance() {
    pollster::block_on(async {
        let phenotype = support::n512_phenotype(4101);
        let frames = deterministic_frame_sequence(64, 4101);
        let first = run_fresh_gpu_sequence(phenotype.clone(), &frames).await;
        let second = run_fresh_gpu_sequence(phenotype, &frames).await;
        assert_eq!(first.adapter_identity, second.adapter_identity);
        assert_eq!(first.selected_candidates, second.selected_candidates);
        assert_eq!(first.dispatch_generations, second.dispatch_generations);
        let ticks = frames.iter().map(PerceptionFrame::tick).collect::<Vec<_>>();
        assert!(ticks.windows(2).any(|pair| pair[0] == pair[1]));
        assert!(ticks.windows(2).any(|pair| pair[1].raw() < pair[0].raw()));
        assert!(first
            .dispatch_generations
            .windows(2)
            .all(|pair| pair[1] == pair[0] + 1));
        assert_ne!(first.dispatch_generations[0], frames[0].tick().raw());
        for (a, b) in first.selected_logits.iter().zip(&second.selected_logits) {
            assert!((a - b).abs() <= first.tolerance);
        }
    });
}

#[test]
fn decoder_reads_the_final_ping_pong_side_for_two_three_and_four_microsteps() {
    pollster::block_on(async {
        for (maturation, microsteps) in [(0.2, 2_u8), (0.5, 3), (0.8, 4)] {
            let phenotype = support::controlled_n512_phenotype_at_maturation(maturation);
            assert_eq!(phenotype.microstep_count(), microsteps);
            let mut gpu = GpuPipelineFixture::new(&phenotype).await;
            gpu.configure_controlled_motor_loop_and_decoder(&phenotype, false);
            let result = gpu
                .run_frame(&two_candidate_frame(7, 77, [0.65, -0.25]))
                .await;
            assert_eq!(
                result.active_activation_side,
                expected_final_side(microsteps)
            );
            assert_eq!(
                result.selection.candidate_index,
                expected_candidate_for_side(microsteps)
            );
        }
    });
}

#[test]
fn active_weight_bank_selector_changes_gpu_behavior_without_mutating_either_fast_bank() {
    pollster::block_on(async {
        let phenotype = support::controlled_n512_phenotype_at_maturation(0.5);
        let frame = two_candidate_frame(7, 77, [0.65, -0.25]);
        let mut gpu = GpuPipelineFixture::new(&phenotype).await;

        let synapse = gpu.configure_controlled_fast_bank_decoder(&phenotype, 1.0, -1.0);
        gpu.set_active_weight_bank(0, 0);
        let bank_zero_before = gpu.read_fast_bank_pair(0, synapse).await;
        assert_eq!(bank_zero_before, [1.0_f32.to_bits(), (-1.0_f32).to_bits()]);
        let bank_zero_result = gpu.run_frame(&frame).await.selection;
        let bank_zero_after = gpu.read_fast_bank_pair(0, synapse).await;
        assert_eq!(bank_zero_before, bank_zero_after);

        gpu.restore_mutable_checkpoint();
        let restored_synapse = gpu.configure_controlled_fast_bank_decoder(&phenotype, 1.0, -1.0);
        assert_eq!(restored_synapse, synapse);
        gpu.set_active_weight_bank(0, 1);
        let bank_one_before = gpu.read_fast_bank_pair(0, synapse).await;
        assert_eq!(bank_one_before, [1.0_f32.to_bits(), (-1.0_f32).to_bits()]);
        let bank_one_result = gpu.run_frame(&frame).await.selection;
        let bank_one_after = gpu.read_fast_bank_pair(0, synapse).await;
        assert_eq!(bank_one_before, bank_one_after);

        assert_ne!(
            (
                bank_zero_result.candidate_index,
                bank_zero_result.logit.to_bits()
            ),
            (
                bank_one_result.candidate_index,
                bank_one_result.logit.to_bits()
            )
        );
    });
}
