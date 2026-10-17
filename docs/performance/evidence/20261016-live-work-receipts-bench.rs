//! CPU structural-accounting component benchmark, never a neural executor.
//! Compile this file with rustc -O and the workspace dependency rlibs.
#![allow(dead_code)]

#[path = "../../../crates/alife_gpu_backend/src/closed_loop_activity.rs"]
mod production;
#[path = "../../../crates/alife_gpu_backend/tests/closed_loop_activity/work_scan_reference.rs"]
mod scan_reference;

use alife_core::{
    BrainActivityPolicyV1, BrainCapacityClass, BrainDispatchIdentity, BrainGenome,
    BrainWorkCounters, DevelopmentState, GpuPressureSample, GpuPressureSampleInput,
    NeuralThrottleDecision, NormalizedScalar, PhenotypeCompiler, SensorProfile, Tick,
};
use alife_gpu_backend::GpuBrainSlot;
use std::{hint::black_box, time::Instant};

fn main() {
    for capacity in [
        BrainCapacityClass::n512(),
        BrainCapacityClass::n1024(),
        BrainCapacityClass::n2048(),
    ] {
        let genome = BrainGenome::scaffold(47_601, capacity.id());
        let development =
            DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(0.35).unwrap());
        let source = PhenotypeCompiler::compile(
            &genome,
            &capacity,
            &development,
            SensorProfile::GroundedObjectSlotsV1,
        )
        .unwrap();
        let policy = BrainActivityPolicyV1::production_v1();
        let schedules = (0..20)
            .map(|tick| {
                let identity = BrainDispatchIdentity {
                    organism_id_raw: 1,
                    tick,
                    class_id_raw: capacity.id().raw(),
                    handle_slot: 0,
                    handle_generation: 1,
                    sequence_cursor: tick + 1,
                    dispatch_generation: tick + 1,
                    frame_digest: [tick + 1; 4],
                };
                let pressure = GpuPressureSample::try_new(
                    &policy,
                    GpuPressureSampleInput {
                        identity,
                        source_dispatch_generation: 0,
                        source_frame_digest: [0; 4],
                        completed_gpu_time_ns: (tick % 4) * 2_000_000,
                        queue_depth: (tick % 4) as u32,
                        logical_heap_used: tick % 4,
                        logical_heap_capacity: 4,
                        brain_atp_remaining_q16: 65_535,
                        brain_atp_capacity_q16: 65_535,
                    },
                )
                .unwrap();
                NeuralThrottleDecision::derive(
                    &policy,
                    &source,
                    capacity.execution(),
                    identity,
                    pressure,
                )
                .unwrap()
            })
            .collect::<Vec<_>>();
        let mut span_visits = 0_u64;
        let mut receipt_reads = 0_u64;
        for decision in &schedules {
            for step in 0..u32::from(decision.microsteps) {
                for projection in source.projections() {
                    let fires = match projection.update_cadence() {
                        alife_core::UpdateCadence::Hot60Hz => true,
                        alife_core::UpdateCadence::Hot15To60Hz
                        | alife_core::UpdateCadence::Hot10To30Hz => step.is_multiple_of(2),
                        alife_core::UpdateCadence::Hot5To15Hz
                        | alife_core::UpdateCadence::Hot1To5Hz => step == 0,
                        alife_core::UpdateCadence::SleepOrOffline
                        | alife_core::UpdateCadence::Disabled => false,
                    };
                    if fires
                        && decision
                            .enabled_route_ids
                            .contains(&projection.route_index())
                    {
                        span_visits += u64::from(projection.synapse_range().1) * 3;
                        receipt_reads += 3;
                    }
                }
            }
        }
        for (index, decision) in schedules.iter().take(4).enumerate() {
            println!(
                "SCHEDULE class={} index={} microsteps={} enabled={:?}",
                capacity.id().raw(),
                index,
                decision.microsteps,
                decision.enabled_route_ids
            );
        }
        println!(
            "META class={} neurons={} synapses={} projections={} ticks={} debug_assertions={} span_visits_per_organism_sample={} receipt_reads_per_organism_sample={}",
            capacity.id().raw(),
            source.neuron_count(),
            source.synapses().len(),
            source.projections().len(),
            schedules.len(),
            cfg!(debug_assertions),
            span_visits,
            receipt_reads,
        );
        for population in [1, 10, 50] {
            let phenotypes = (0..population).map(|_| source.clone()).collect::<Vec<_>>();
            let run = |receipts: bool| {
                let mut outputs = Vec::<BrainWorkCounters>::with_capacity(population * 20 * 3);
                let started = Instant::now();
                for decision in &schedules {
                    for phenotype in &phenotypes {
                        // Model affordability maxima, packing and final receipt.
                        for (candidates, memory) in [(32, 8), (3, 2), (3, 2)] {
                            let derive = if receipts {
                                production::derive_executed_work
                            } else {
                                scan_reference::derive_executed_work
                            };
                            outputs.push(
                                derive(
                                    black_box(phenotype),
                                    black_box(decision.microsteps),
                                    black_box(&decision.enabled_route_ids),
                                    black_box(candidates),
                                    black_box(memory),
                                )
                                .unwrap(),
                            );
                        }
                    }
                }
                (started.elapsed().as_nanos(), outputs)
            };
            for _ in 0..2 {
                let a = run(false);
                let b = run(true);
                assert_eq!(a.1, b.1);
                black_box((a, b));
            }
            for sample in 0..9 {
                let (baseline, receipts) = if sample % 2 == 0 {
                    (run(false), run(true))
                } else {
                    let receipts = run(true);
                    (run(false), receipts)
                };
                assert_eq!(baseline.1, receipts.1);
                println!(
                    "SAMPLE class={} population={} sample={} baseline_first={} baseline_ns={} receipts_ns={} exact_work_rows={}",
                    capacity.id().raw(),
                    population,
                    sample,
                    sample % 2 == 0,
                    baseline.0,
                    receipts.0,
                    baseline.1.len(),
                );
                black_box((baseline, receipts));
            }
        }
    }
}
