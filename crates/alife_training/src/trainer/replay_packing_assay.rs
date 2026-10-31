//! Bounded adapter-free baseline stages and one-call packing comparisons.
use super::*;
use crate::{TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick};
use std::{hint::black_box, sync::OnceLock, time::Instant};

#[path = "replay_packing_tests.rs"]
mod tests;

#[cfg(target_os = "linux")]
#[path = "replay_packing_memory.rs"]
mod memory;

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

// Frozen unmodified production baseline from 1d58e143.
fn reference_pack(sequence: &TrainingSequence, layout: PackedLayout, neurons: u32) -> Vec<u32> {
    let mut words = vec![0; layout.training_words as usize];
    for (tick_index, tick) in sequence.ticks.iter().enumerate() {
        let input = layout.inputs as usize + tick_index * neurons as usize;
        for (slot, value) in words[input..input + neurons as usize]
            .iter_mut()
            .zip(&tick.encoded_inputs)
        {
            *slot = value.to_bits();
        }
        for (candidate_index, candidate) in tick.candidates.iter().enumerate() {
            let base = layout.candidate_records as usize
                + (tick_index * layout.candidate_capacity as usize + candidate_index)
                    * CANDIDATE_RECORD_WORDS;
            words[base] = 1;
            words[base + 1] = u32::from(candidate.family.raw());
            words[base + 2] = candidate.innate_bias.to_bits();
            for (lane, value) in candidate.decoder_inputs.iter().enumerate() {
                let field = if lane < 24 { 4 + lane } else { 32 + lane - 24 };
                words[base + field] = value.to_bits();
            }
        }
        let base = layout.context as usize
            + tick_index
                * (4 + tick.enabled_routes.len()
                    + tick.effective_weight_offsets.len()
                    + alife_core::MAX_STRUCTURAL_EDGES * 5);
        words[base] = tick.projection_gain.to_bits();
        words[base + 1] = tick.local_threshold_shift.to_bits();
        words[base + 2] = tick.microstep_count;
        words[base + 3] = tick.structural_synapses.len() as u32;
        for (index, enabled) in tick.enabled_routes.iter().enumerate() {
            words[base + 4 + index] = u32::from(*enabled);
        }
        for (index, value) in tick.effective_weight_offsets.iter().enumerate() {
            words[base + 4 + tick.enabled_routes.len() + index] = value.to_bits();
        }
        let edge_base = base + 4 + tick.enabled_routes.len() + tick.effective_weight_offsets.len();
        for (index, edge) in tick.structural_synapses.iter().enumerate() {
            let at = edge_base + index * 5;
            words[at..at + 5].copy_from_slice(&[
                edge.source,
                edge.target,
                edge.route,
                edge.cadence,
                edge.effective_weight.to_bits(),
            ]);
        }
    }
    for (index, value) in sequence
        .initial
        .activations
        .iter()
        .chain(&sequence.initial.metabolic_load)
        .chain(&sequence.initial.activity_ema)
        .enumerate()
    {
        words[layout.initial as usize + index] = value.to_bits();
    }
    words
}

// Same baseline statements with exclusive, test-only clocks.
fn stage_pack(
    sequence: &TrainingSequence,
    layout: PackedLayout,
    neurons: u32,
) -> (Vec<u32>, [u64; 7], u64) {
    let enclosing = Instant::now();
    let mut spans = [0_u64; 7];
    let timer = Instant::now();
    let mut words = vec![0; layout.training_words as usize];
    spans[0] += timer.elapsed().as_nanos() as u64;
    for (tick_index, tick) in sequence.ticks.iter().enumerate() {
        let timer = Instant::now();
        let input = layout.inputs as usize + tick_index * neurons as usize;
        for (slot, value) in words[input..input + neurons as usize]
            .iter_mut()
            .zip(&tick.encoded_inputs)
        {
            *slot = value.to_bits();
        }
        spans[1] += timer.elapsed().as_nanos() as u64;
        let timer = Instant::now();
        for (candidate_index, candidate) in tick.candidates.iter().enumerate() {
            let base = layout.candidate_records as usize
                + (tick_index * layout.candidate_capacity as usize + candidate_index)
                    * CANDIDATE_RECORD_WORDS;
            words[base] = 1;
            words[base + 1] = u32::from(candidate.family.raw());
            words[base + 2] = candidate.innate_bias.to_bits();
            for (lane, value) in candidate.decoder_inputs.iter().enumerate() {
                let field = if lane < 24 { 4 + lane } else { 32 + lane - 24 };
                words[base + field] = value.to_bits();
            }
        }
        spans[2] += timer.elapsed().as_nanos() as u64;
        let timer = Instant::now();
        let base = layout.context as usize
            + tick_index
                * (4 + tick.enabled_routes.len()
                    + tick.effective_weight_offsets.len()
                    + alife_core::MAX_STRUCTURAL_EDGES * 5);
        words[base] = tick.projection_gain.to_bits();
        words[base + 1] = tick.local_threshold_shift.to_bits();
        words[base + 2] = tick.microstep_count;
        words[base + 3] = tick.structural_synapses.len() as u32;
        for (index, enabled) in tick.enabled_routes.iter().enumerate() {
            words[base + 4 + index] = u32::from(*enabled);
        }
        spans[3] += timer.elapsed().as_nanos() as u64;
        let timer = Instant::now();
        for (index, value) in tick.effective_weight_offsets.iter().enumerate() {
            words[base + 4 + tick.enabled_routes.len() + index] = value.to_bits();
        }
        spans[4] += timer.elapsed().as_nanos() as u64;
        let timer = Instant::now();
        let edge_base = base + 4 + tick.enabled_routes.len() + tick.effective_weight_offsets.len();
        for (index, edge) in tick.structural_synapses.iter().enumerate() {
            let at = edge_base + index * 5;
            words[at..at + 5].copy_from_slice(&[
                edge.source,
                edge.target,
                edge.route,
                edge.cadence,
                edge.effective_weight.to_bits(),
            ]);
        }
        spans[5] += timer.elapsed().as_nanos() as u64;
    }
    let timer = Instant::now();
    for (index, value) in sequence
        .initial
        .activations
        .iter()
        .chain(&sequence.initial.metabolic_load)
        .chain(&sequence.initial.activity_ema)
        .enumerate()
    {
        words[layout.initial as usize + index] = value.to_bits();
    }
    spans[6] += timer.elapsed().as_nanos() as u64;
    let elapsed = enclosing.elapsed().as_nanos() as u64;
    (words, spans, elapsed)
}

type Packer = fn(&TrainingSequence, PackedLayout, u32) -> Vec<u32>;

#[test]
#[ignore = "bounded CPU stage assay; no adapter or device"]
fn single_call_packing_stages() {
    let p = phenotype();
    let mut shapes = Vec::new();
    for (ticks, burn) in [(512, 0), (1, 0), (256, 0), (384, 128)] {
        let replay = sequence(p, ticks, burn);
        let (_, layout) = pack_metadata_and_layout(p, Some(&replay)).unwrap();
        let expected = reference_pack(&replay, layout, p.neuron_count());
        let mut rows = Vec::new();
        for warmup in 0..2 {
            for packer in [pack_replay_sequence as Packer, reference_pack as Packer] {
                let words = black_box(packer)(&replay, layout, p.neuron_count());
                assert_eq!(words, expected, "warmup {warmup}");
            }
            assert_eq!(stage_pack(&replay, layout, p.neuron_count()).0, expected);
        }
        for trial in 0..11 {
            let modes = if trial % 2 == 0 { [0, 1, 2] } else { [2, 1, 0] };
            for mode in modes {
                let start = Instant::now();
                let (words, spans, internal_ns) = match mode {
                    0 => (
                        black_box(pack_replay_sequence as Packer)(
                            black_box(&replay),
                            layout,
                            p.neuron_count(),
                        ),
                        None,
                        None,
                    ),
                    1 => (
                        black_box(reference_pack as Packer)(
                            black_box(&replay),
                            layout,
                            p.neuron_count(),
                        ),
                        None,
                        None,
                    ),
                    _ => {
                        let (words, spans, elapsed) =
                            stage_pack(black_box(&replay), layout, p.neuron_count());
                        (words, Some(spans), Some(elapsed))
                    }
                };
                let elapsed_ns = start.elapsed().as_nanos() as u64;
                assert_eq!(words, expected);
                rows.push(serde_json::json!({"trial":trial,"mode":(["production","frozen_reference","instrumented_reference"][mode]),
                    "elapsed_ns":elapsed_ns,"exclusive_stage_ns":spans,"instrumented_internal_ns":internal_ns}));
                drop(words); // Verification/destruction are outside the timed span.
            }
        }
        shapes.push(serde_json::json!({"ticks":ticks,"burn_in_ticks":burn,
            "packed_bytes":expected.len()*4,"weight_offset_bytes":ticks*p.synapses().len()*4,"trials":rows}));
    }
    println!(
        "PACKING_STAGE_JSON={}",
        serde_json::json!({
            "assay":"single_call_packing_stages","build_tag":std::env::var("ALIFE_SINGLE_PACK_PROFILE").unwrap_or_default(),
            "warmups_per_mode":2,"trials_per_mode":11,"device_requested":false,
            "stage_names":["zeroed_allocation_call","inputs","candidates","context_routes","dense_weight_offsets","structural_edges","initial_state"],
            "zeroing_note":"lazy zero-page/first-touch work can occur in later write spans; allocation-call time is not total zeroing time",
            "full_equality_every_trial":true,"output_drop_excluded":true,"shapes":shapes,
        })
    );
}
