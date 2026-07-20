//! Populated phenotype, memory, and runtime-profile fixtures for Slice D scaling tests.

use alife_core::{
    BrainCapacityClass, BrainPhenotype, Confidence, FinalizedMemoryRecall, MemoryBank,
    MemoryBankConfig, PerceptionFrame, PerceptionFrameDraft, SensorProfile,
};
use alife_gpu_backend::GpuRuntimeProfile;

use super::{perception_frame_for_profile_at_tick, phenotype_for_capacity_at_maturation};

pub fn populated(
    capacity: BrainCapacityClass,
    seed: u64,
    profile: SensorProfile,
) -> BrainPhenotype {
    phenotype_for_capacity_at_maturation(capacity, seed, 0.35, profile)
}

pub fn bounded_profile(
    logical_bytes: u64,
    physical_bytes: u64,
    max_hot_brains: u32,
    growth_chunk_slots: u16,
) -> GpuRuntimeProfile {
    GpuRuntimeProfile {
        schema_version: 1,
        profile_id: 65_000,
        logical_neural_heap_budget_bytes: logical_bytes,
        physical_allocation_ceiling_bytes: physical_bytes,
        max_hot_brains,
        max_in_flight_batches: 4,
        growth_chunk_slots,
        retain_empty_chunks: 1,
        reserved: [0; 7],
    }
}

pub fn finalized_memory_frame(
    organism_raw: u64,
    tick: u64,
) -> (PerceptionFrame, FinalizedMemoryRecall) {
    let source = perception_frame_for_profile_at_tick(
        organism_raw,
        tick,
        SensorProfile::GroundedObjectSlotsV1,
        true,
        2,
    );
    let draft = PerceptionFrameDraft::new(
        source.organism_id(),
        source.tick(),
        source.sensor_profile(),
        source.sensory().clone(),
        source.body(),
        *source.homeostasis(),
        source.candidates().to_vec(),
        source.profile_provenance(),
        source.grounded_object_slots().to_vec(),
    )
    .unwrap();
    let bank = MemoryBank::new(
        MemoryBankConfig::new(8, 64, 4, 0.72, Confidence::new(0.0).unwrap()).unwrap(),
    )
    .unwrap();
    bank.recall_frame(&draft).unwrap().finalize(draft).unwrap()
}
