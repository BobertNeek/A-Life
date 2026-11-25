//! Consolidated GPU backend integration test harness.

mod support;

#[path = "suite/closed_loop_activity.rs"]
mod closed_loop_activity;
#[path = "suite/closed_loop_admission.rs"]
mod closed_loop_admission;
#[path = "suite/closed_loop_buffer_contracts.rs"]
mod closed_loop_buffer_contracts;
#[cfg(feature = "gpu-tests")]
#[path = "suite/closed_loop_checkpoint.rs"]
mod closed_loop_checkpoint;
#[cfg(feature = "gpu-tests")]
#[path = "suite/closed_loop_eligibility.rs"]
mod closed_loop_eligibility;
#[path = "suite/closed_loop_exploration.rs"]
mod closed_loop_exploration;
#[cfg(feature = "gpu-tests")]
#[path = "suite/closed_loop_fast_plasticity.rs"]
mod closed_loop_fast_plasticity;
#[path = "suite/closed_loop_fixed_sleep.rs"]
mod closed_loop_fixed_sleep;
#[cfg(feature = "gpu-tests")]
#[path = "suite/closed_loop_gpu_behavior.rs"]
mod closed_loop_gpu_behavior;
#[path = "suite/closed_loop_learning_buffers.rs"]
mod closed_loop_learning_buffers;
#[path = "suite/closed_loop_memory_context.rs"]
mod closed_loop_memory_context;
#[path = "suite/closed_loop_runtime.rs"]
mod closed_loop_runtime;
#[cfg(feature = "gpu-tests")]
#[path = "suite/closed_loop_sleep.rs"]
mod closed_loop_sleep;
#[path = "suite/closed_loop_wgsl.rs"]
mod closed_loop_wgsl;
#[path = "suite/developmental_plasticity.rs"]
mod developmental_plasticity;
#[path = "suite/eligibility_simulation_time.rs"]
mod eligibility_simulation_time;
#[cfg(feature = "gpu-tests")]
#[path = "suite/joint_action_eligibility.rs"]
mod joint_action_eligibility;
#[cfg(feature = "gpu-tests")]
#[path = "suite/joint_action_outcome.rs"]
mod joint_action_outcome;
#[cfg(feature = "gpu-tests")]
#[path = "suite/n512_legacy_compatibility_gpu.rs"]
mod n512_legacy_compatibility_gpu;
#[path = "suite/oja_credit_stability.rs"]
mod oja_credit_stability;
#[cfg(all(feature = "training-rollout", feature = "gpu-tests"))]
#[path = "suite/training_capture_batch.rs"]
mod training_capture_batch;
#[cfg(feature = "gpu-tests")]
#[path = "suite/v11_dendritic_structural_dispatch.rs"]
mod v11_dendritic_structural_dispatch;
#[path = "suite/v2_neuromodulatory_wgsl.rs"]
mod v2_neuromodulatory_wgsl;
