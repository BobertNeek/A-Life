use alife_core::*;
use alife_gpu_backend::{
    GpuClassBucketPlan, GpuMemoryChannelPlan, GpuPhenotypeUpload, CLOSED_LOOP_MEMORY_CONTEXT_WGSL,
};
#[test]
fn explicit_gain_upload_uses_the_same_versioned_plan_as_cpu() {
    let g = BrainGenome::scaffold(0x2048_2026, BrainCapacityClass::N2048_ID);
    let d = DevelopmentState::new(g.id, Tick::ZERO, NormalizedScalar::new(1.).unwrap());
    let p = PhenotypeCompiler::compile_n2048_memory_gain_graph(
        g,
        d,
        SensorProfile::GroundedObjectSlotsV1,
    )
    .unwrap()
    .0;
    let upload = GpuPhenotypeUpload::try_from(&p).unwrap();
    let plan = upload.memory_channel_plans[0];
    assert_eq!(plan.schema_version, 2);
    assert_eq!(plan.max_candidate_gain, 12.);
    assert_eq!(plan.memory_decoder_synapse_count, 4096);
    assert_eq!(
        plan.max_candidate_gain,
        p.candidate_decoder()
            .memory_channel()
            .unwrap()
            .max_candidate_gain()
    );
    let mut bucket = GpuClassBucketPlan::new(BrainCapacityClass::n2048(), 1).unwrap();
    bucket.insert_phenotype(0, 1, &p).unwrap();
    let copy =
        GpuMemoryChannelPlan::try_from(p.candidate_decoder().memory_channel().unwrap()).unwrap();
    assert_eq!(copy.max_candidate_gain, 12.);
    let module = naga::front::wgsl::parse_str(CLOSED_LOOP_MEMORY_CONTEXT_WGSL).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
}
