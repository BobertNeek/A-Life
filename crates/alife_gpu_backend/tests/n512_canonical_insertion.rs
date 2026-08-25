#![cfg(feature = "gpu-tests")]

use alife_core::{
    BrainCapacityClass, BrainGenome, DevelopmentState, NormalizedScalar, OrganismId,
    PhenotypeCompiler, SensorProfile, Tick,
};
use alife_gpu_backend::{GpuClassBucketPlan, GpuClosedLoopBackend, GpuRuntimeProfile};

#[test]
fn canonical_v2_nano512_inserts_under_production_profile() {
    let capacity = BrainCapacityClass::n512();
    let genome = BrainGenome::scaffold(0x4E35_3132_5F00_0001, capacity.id());
    let development =
        DevelopmentState::new(genome.id, Tick::ZERO, NormalizedScalar::new(1.0).unwrap());
    let phenotype = PhenotypeCompiler::compile(
        &genome,
        &capacity,
        &development,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap();
    GpuClassBucketPlan::for_phenotype(&phenotype)
        .unwrap()
        .slot_allocation_receipt()
        .unwrap()
        .validate_contract()
        .unwrap();

    let mut backend =
        GpuClosedLoopBackend::new_required(GpuRuntimeProfile::production_v1()).unwrap();
    let hardware = backend.hardware_receipt();
    println!("BACKEND_API={:?}", hardware.backend_api);
    println!("ADAPTER={}", hardware.adapter_name);
    backend
        .insert_brain(OrganismId(51_200_002), phenotype)
        .unwrap();
}
