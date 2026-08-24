use alife_gpu_backend::{
    GpuNeuralReceptorEffectsRecord, GpuOutcomeCreditRecord, GpuPlasticityReceptorRecord,
    GpuReplayEventRecord, CLOSED_LOOP_ENCODE_WGSL, CLOSED_LOOP_PLASTICITY_WGSL,
    CLOSED_LOOP_RECURRENT_WGSL, CLOSED_LOOP_REPLAY_LEARNING_WGSL,
};

#[test]
fn v2_learning_shaders_validate_and_consume_lane_and_receptor_vectors() {
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
        assert!(!source.contains("modulator_value"));
        assert!(!source.contains("modulator_sign"));
        assert!(source.contains("receptor_weights"));
    }
    assert!(CLOSED_LOOP_PLASTICITY_WGSL.contains("project_third_factor"));
    assert_eq!(std::mem::size_of::<GpuPlasticityReceptorRecord>(), 64);
    assert_eq!(std::mem::size_of::<GpuOutcomeCreditRecord>(), 176);
    assert_eq!(std::mem::size_of::<GpuReplayEventRecord>(), 112);
}

#[test]
fn production_inference_wgsl_directly_consumes_targeted_receptor_effects() {
    for source in [CLOSED_LOOP_ENCODE_WGSL, CLOSED_LOOP_RECURRENT_WGSL] {
        let module = naga::front::wgsl::parse_str(source).expect("receptor-aware WGSL must parse");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .expect("receptor-aware WGSL must validate");
        assert!(source.contains("load_neural_receptor_effects"));
        assert!(source.contains("neural_receptor_effects_offset"));
    }
    assert!(CLOSED_LOOP_ENCODE_WGSL.contains("interoceptive_gain"));
    assert!(CLOSED_LOOP_ENCODE_WGSL.contains("structural_growth_gate"));
    assert!(CLOSED_LOOP_ENCODE_WGSL.contains("sleep_gate"));
    assert!(CLOSED_LOOP_ENCODE_WGSL.contains("consolidation_gate"));
    assert!(CLOSED_LOOP_RECURRENT_WGSL.contains("projection_gain"));
    assert!(CLOSED_LOOP_RECURRENT_WGSL.contains("local_threshold_shift"));
    assert_eq!(std::mem::size_of::<GpuNeuralReceptorEffectsRecord>(), 64);
}
