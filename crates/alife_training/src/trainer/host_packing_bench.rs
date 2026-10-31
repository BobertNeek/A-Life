//! Opt-in, adapter-free measurements of unchanged production host helpers.
use super::*;
use crate::{TrainingInitialState, TrainingReplayCandidate, TrainingReplayTick};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
    time::Instant,
};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, serde::Serialize)]
struct AllocationCounts {
    alloc_calls: u64,
    alloc_zeroed_calls: u64,
    realloc_calls: u64,
    dealloc_calls: u64,
    requested_bytes: u64,
    released_bytes: u64,
}

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static COUNTS: Cell<AllocationCounts> = const { Cell::new(AllocationCounts {
        alloc_calls: 0, alloc_zeroed_calls: 0, realloc_calls: 0, dealloc_calls: 0,
        requested_bytes: 0, released_bytes: 0,
    }) };
}

fn record(update: impl FnOnce(&mut AllocationCounts)) {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNTS.try_with(|cell| {
            let mut counts = cell.get();
            update(&mut counts);
            cell.set(counts);
        });
    }
}

struct ObservedSystem;

// SAFETY: Every operation forwards the unchanged allocation contract to System.
// The optional thread-local counters neither allocate nor access allocation data.
unsafe impl GlobalAlloc for ObservedSystem {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller supplies GlobalAlloc's valid layout contract.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            record(|c| {
                c.alloc_calls += 1;
                c.requested_bytes += layout.size() as u64;
            });
        }
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller supplies GlobalAlloc's valid layout contract.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if !pointer.is_null() {
            record(|c| {
                c.alloc_zeroed_calls += 1;
                c.requested_bytes += layout.size() as u64;
            });
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        record(|c| {
            c.dealloc_calls += 1;
            c.released_bytes += layout.size() as u64;
        });
        // SAFETY: The caller supplies the original pointer and layout.
        unsafe { System.dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        // SAFETY: The caller supplies GlobalAlloc's pointer/layout/size contract.
        let next = unsafe { System.realloc(pointer, layout, size) };
        if !next.is_null() {
            record(|c| {
                c.realloc_calls += 1;
                c.requested_bytes += size as u64;
                c.released_bytes += layout.size() as u64;
            });
        }
        next
    }
}

// Only this crate's CPU-enabled libtest binary uses this pass-through allocator.
// Normal library builds and all downstream applications retain their allocator.
#[global_allocator]
static ALLOCATOR: ObservedSystem = ObservedSystem;

struct CountingGuard;
impl CountingGuard {
    fn start() -> Self {
        assert!(!COUNTING.with(Cell::get));
        COUNTS.with(|c| c.set(AllocationCounts::default()));
        COUNTING.with(|c| c.set(true));
        Self
    }
}
impl Drop for CountingGuard {
    fn drop(&mut self) {
        COUNTING.with(|c| c.set(false));
    }
}

fn measure<T>(
    operation: &str,
    operations_per_trial: u64,
    mut run: impl FnMut() -> T,
    verify: impl Fn(&T),
) -> serde_json::Value {
    for _ in 0..2 {
        let output = black_box(run());
        verify(&output);
        drop(output);
    }
    let mut elapsed_ns = Vec::new();
    for _ in 0..11 {
        let start = Instant::now();
        let output = black_box(run());
        elapsed_ns.push(start.elapsed().as_nanos() as u64);
        verify(&output);
        drop(output);
    }
    let mut allocation_trials = Vec::new();
    for _ in 0..3 {
        let guard = CountingGuard::start();
        let output = black_box(run());
        drop(guard);
        allocation_trials.push(COUNTS.with(Cell::get));
        verify(&output);
        drop(output);
    }
    assert!(allocation_trials.windows(2).all(|pair| pair[0] == pair[1]));
    serde_json::json!({
        "operation": operation, "operations_per_trial": operations_per_trial,
        "elapsed_ns": elapsed_ns, "allocation_trials": allocation_trials,
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

#[test]
#[ignore = "manual bounded CPU packing assay; no adapter or device is requested"]
fn production_gpu_ppo_host_packing() {
    use alife_core::{
        BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, PhenotypeCompiler,
        SensorProfile, Tick,
    };
    let capacity = BrainCapacityClass::n2048();
    let genome = BrainGenome::scaffold(99, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let phenotype = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedTerrainVisionV1,
    )
    .unwrap();
    assert_eq!(phenotype.neuron_count(), 2048);
    assert_eq!(phenotype.synapses().len(), 32768);
    let mut shapes = Vec::new();
    for (ticks, burn) in [(512, 0), (1, 0), (256, 0), (384, 128)] {
        let replay = sequence(&phenotype, ticks, burn);
        let (metadata, layout) = pack_metadata_and_layout(&phenotype, Some(&replay)).unwrap();
        let expected_words = pack_replay_sequence(&replay, layout, phenotype.neuron_count());
        let expected_headers =
            build_step_headers(&phenotype, layout, AdamWConfig::default(), 19801).unwrap();
        let shape = (ticks, replay.initial.dendrites.clone());
        let unequal = (ticks + 1, replay.initial.dendrites.clone());
        let mut rows = vec![measure(
            "validate_for",
            1,
            || {
                black_box(&replay)
                    .validate_for(black_box(&phenotype))
                    .unwrap()
            },
            |_| {},
        )];
        rows.push(measure(
            "shape_key_clone",
            1,
            || {
                (
                    black_box(replay.ticks.len()),
                    black_box(&replay.initial.dendrites).clone(),
                )
            },
            |next| assert!(replay_shapes_equal(&shape, next)),
        ));
        for (name, other, expected) in [
            ("shape_equal_1024", &shape, true),
            ("shape_different_tick_count_1024", &unequal, false),
        ] {
            rows.push(measure(
                name,
                1024,
                || {
                    let mut matches = 0;
                    for _ in 0..1024 {
                        matches += usize::from(black_box(replay_shapes_equal(
                            black_box(&shape),
                            black_box(other),
                        )));
                    }
                    matches
                },
                |matches| assert_eq!(*matches, if expected { 1024 } else { 0 }),
            ));
        }
        rows.push(measure(
            "pack_metadata_and_layout_warm_cache",
            1,
            || pack_metadata_and_layout(black_box(&phenotype), Some(black_box(&replay))).unwrap(),
            |(words, next)| {
                assert_eq!(words, &metadata);
                assert_eq!(next.training_words, layout.training_words);
                assert_eq!(next.burn_in_steps, layout.burn_in_steps);
            },
        ));
        rows.push(measure(
            "pack_replay_sequence",
            1,
            || {
                pack_replay_sequence(
                    black_box(&replay),
                    black_box(layout),
                    phenotype.neuron_count(),
                )
            },
            |words| assert_eq!(words, &expected_words),
        ));
        rows.push(measure(
            "build_step_headers",
            1,
            || {
                build_step_headers(
                    black_box(&phenotype),
                    black_box(layout),
                    AdamWConfig::default(),
                    19801,
                )
                .unwrap()
            },
            |words| assert_eq!(words, &expected_headers),
        ));
        rows.push(measure(
            "warm_shape_host_prefix_overlaps_components",
            1,
            || {
                let sequence = black_box(&replay);
                sequence.validate_for(black_box(&phenotype)).unwrap();
                let next = (sequence.ticks.len(), sequence.initial.dendrites.clone());
                assert!(replay_shapes_equal(black_box(&shape), &next));
                let mut refreshed = black_box(layout);
                refreshed.refresh_replay_parameters(sequence, phenotype.microstep_count());
                let words = pack_replay_sequence(sequence, refreshed, phenotype.neuron_count());
                let headers =
                    build_step_headers(&phenotype, refreshed, AdamWConfig::default(), 19801)
                        .unwrap();
                (words, headers)
            },
            |(words, headers)| {
                assert_eq!(words, &expected_words);
                assert_eq!(headers, &expected_headers);
            },
        ));
        // A separate timed control quantifies enabling the counters, not the
        // disabled pass-through allocator's overhead versus an unwrapped System.
        let mut counted_pack_elapsed_ns = Vec::new();
        for _ in 0..11 {
            let guard = CountingGuard::start();
            let start = Instant::now();
            let words = black_box(pack_replay_sequence(
                &replay,
                layout,
                phenotype.neuron_count(),
            ));
            counted_pack_elapsed_ns.push(start.elapsed().as_nanos() as u64);
            drop(guard);
            assert_eq!(words, expected_words);
            drop(words);
        }
        shapes.push(serde_json::json!({
            "ticks": ticks, "burn_in_ticks": burn,
            "candidate_count_per_tick": 25,
            "dendritic_branch_count": replay.initial.dendrites.branches().len(),
            "dendritic_input_count": replay.initial.dendrites.branches().iter().map(|b| b.inputs.len()).sum::<usize>(),
            "structural_edges_per_tick": 0,
            "training_words": layout.training_words,
            "packed_bytes": expected_words.len() * 4,
            "metadata_bytes": metadata.len() * 4,
            "header_bytes": expected_headers.len() * 4,
            "rows": rows, "counted_pack_elapsed_ns": counted_pack_elapsed_ns,
        }));
    }
    println!(
        "HOST_PACKING_JSON={}",
        serde_json::json!({
            "assay": "production_gpu_ppo_host_packing", "schema": 1,
            "neuron_count": phenotype.neuron_count(), "synapse_count": phenotype.synapses().len(),
            "route_count": phenotype.projections().len(), "microsteps": phenotype.microstep_count(),
            "warmups_per_component": 2, "timing_trials_per_component": 11,
            "allocation_trials_per_component": 3, "timing_output_drop_excluded": true,
            "allocation_output_drop_excluded": true, "device_requested": false,
            "shapes": shapes,
        })
    );
}
