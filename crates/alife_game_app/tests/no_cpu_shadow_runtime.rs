#[test]
fn production_runtime_contains_no_cpu_shadow_or_neural_fallback_contract() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/gpu_live_runtime.rs",
        "src/live_brain_bridge.rs",
        "src/graphical_playground.rs",
    ] {
        let source = std::fs::read_to_string(root.join(relative)).unwrap();
        assert!(
            !source.to_ascii_lowercase().contains("cpu_shadow"),
            "{relative}"
        );
        assert!(!source.contains("AutoWithCpuFallback"), "{relative}");
        assert!(!source.contains("CpuReference"), "{relative}");
    }
}
