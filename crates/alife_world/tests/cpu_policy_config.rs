use alife_core::{BrainScaleTier, PolicyBackend};
use alife_world::{BrainPolicyConfig, RuntimeConfig};
#[test]
fn cpu_requires_explicit_offline_schema_and_gpu_default_remains_compatible() {
    let mut config = RuntimeConfig::deterministic_default(42, BrainScaleTier::Standard2048);
    assert_eq!(
        config.brain_policy.policy,
        PolicyBackend::NeuralClosedLoopGpu
    );
    assert_eq!(config.brain_policy.schema_version, 1);
    config.validate().unwrap();
    let old = serde_json::to_vec(&config).unwrap();
    let read: RuntimeConfig = serde_json::from_slice(&old).unwrap();
    assert_eq!(read, config);
    config.brain_policy.policy = PolicyBackend::NeuralClosedLoopCpu;
    assert!(config.validate().is_err());
    config.brain_policy = BrainPolicyConfig::offline_cpu_training();
    config.validate().unwrap();
    config.features.gpu_backend_enabled = true;
    assert!(config.validate().is_err());
}
