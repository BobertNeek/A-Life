use alife_gpu_backend::{
    GpuNeuralReceptorEffectsRecord, GpuOutcomeCreditRecord, GpuPlasticityReceptorRecord,
    GpuReplayEventRecord, CLOSED_LOOP_ENCODE_WGSL, CLOSED_LOOP_PLASTICITY_WGSL,
    CLOSED_LOOP_RECURRENT_WGSL, CLOSED_LOOP_REPLAY_LEARNING_WGSL,
};

#[test]
fn v2_learning_shaders_validate_and_match_host_layouts() {
    for source in [
        CLOSED_LOOP_PLASTICITY_WGSL,
        CLOSED_LOOP_REPLAY_LEARNING_WGSL,
    ] {
        let module = naga::front::wgsl::parse_str(source).expect("v2 learning WGSL must parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("v2 learning WGSL must validate");
    }
    assert_eq!(std::mem::size_of::<GpuPlasticityReceptorRecord>(), 64);
    assert_eq!(std::mem::size_of::<GpuOutcomeCreditRecord>(), 176);
    assert_eq!(std::mem::size_of::<GpuReplayEventRecord>(), 112);
}

#[test]
fn production_inference_wgsl_validates_and_matches_receptor_layout() {
    for source in [CLOSED_LOOP_ENCODE_WGSL, CLOSED_LOOP_RECURRENT_WGSL] {
        let module = naga::front::wgsl::parse_str(source).expect("receptor-aware WGSL must parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("receptor-aware WGSL must validate");
    }
    assert_eq!(std::mem::size_of::<GpuNeuralReceptorEffectsRecord>(), 64);
}
