//! Versioned playable-sim schema constants split during R13 remediation.

pub const G01_APP_SHELL_SCHEMA: &str = "alife.g01.app_shell.v1";
pub const G01_APP_SHELL_SCHEMA_VERSION: u16 = 1;
pub const G02_VISIBLE_WORLD_SCHEMA: &str = "alife.g02.visible_world.v1";
pub const G02_VISIBLE_WORLD_SCHEMA_VERSION: u16 = 1;
pub const G03_LIVE_BRAIN_LOOP_SCHEMA: &str = "alife.g03.live_brain_loop.v1";
pub const G03_LIVE_BRAIN_LOOP_SCHEMA_VERSION: u16 = 1;
pub const G04_CREATURE_VISUAL_SCHEMA: &str = "alife.g04.creature_visual_state.v1";
pub const G04_CREATURE_VISUAL_SCHEMA_VERSION: u16 = 1;
pub const G05_CAMERA_INSPECTOR_SCHEMA: &str = "alife.g05.camera_selection_inspector.v1";
pub const G05_CAMERA_INSPECTOR_SCHEMA_VERSION: u16 = 1;
pub const G13_WORLD_EDITOR_SCHEMA: &str = "alife.g13.world_editor.v1";
pub const G13_WORLD_EDITOR_SCHEMA_VERSION: u16 = 1;
pub const G13_EDITOR_MAX_OBJECTS: usize = 16;
pub const CA10_ENVIRONMENT_MANIFEST_SCHEMA: &str = "alife.ca10.environment_manifest.v1";
pub const CA10_ENVIRONMENT_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const CA10_MAX_ENVIRONMENT_SCENARIOS: usize = 8;
pub const CA11_PLAYER_SANDBOX_EDITOR_SCHEMA: &str = "alife.ca11.player_sandbox_editor.v1";
pub const CA11_PLAYER_SANDBOX_EDITOR_SCHEMA_VERSION: u16 = 1;
pub const CA12_APP_BUNDLE_MANIFEST_SCHEMA: &str = "alife.ca12.app_bundle_manifest.v1";
pub const CA12_APP_BUNDLE_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const CA12_MAX_BUNDLE_FILE_BYTES: u64 = 128 * 1024;
/// Metadata packs referenced by the CA12 bundle may describe many bounded assets
/// while runtime configs and WGSL remain subject to the smaller metadata cap.
pub const CA12_MAX_REFERENCED_MANIFEST_BYTES: u64 = 256 * 1024;
pub const CA12_MAX_BUNDLE_ENTRIES: usize = 32;
pub const CA13_DOUBLE_BUFFERED_SCHEDULER_SCHEMA: &str =
    "alife.ca13.double_buffered_graphical_scheduler.v1";
pub const CA13_DOUBLE_BUFFERED_SCHEDULER_SCHEMA_VERSION: u16 = 1;
pub const CA13_FIXED_SIM_TICK_HZ: u32 = alife_world::WORLD_TICKS_PER_SECOND;
pub const CA13_TARGET_RENDER_FRAME_HZ: u32 = 60;
pub const CA13_MAX_CATCH_UP_TICKS_PER_FRAME: u32 = 4;
pub const CA13_MAX_ACCUMULATOR_MICROS: u64 = 250_000;
pub const CA14_MOTOR_RING_PRESENTATION_SCHEMA: &str =
    "alife.ca14.motor_ring_arbitration_presentation.v1";
pub const CA14_MOTOR_RING_PRESENTATION_SCHEMA_VERSION: u16 = 1;
pub const CA14_MAX_MOTOR_RING_CHANNELS: usize = 6;
pub const CA16_EAT_REACH_DISTANCE: f32 = 1.25;

pub const CA32_REALTIME_WGSL_TELEMETRY_SCHEMA: &str = "alife.ca32.realtime_wgsl_telemetry.v1";
pub const CA32_REALTIME_WGSL_TELEMETRY_SCHEMA_VERSION: u16 = 1;
pub const CA33_BATCHED_GPU_RUNTIME_SCHEMA: &str = "alife.ca33.batched_gpu_runtime.v1";
pub const CA33_BATCHED_GPU_RUNTIME_SCHEMA_VERSION: u16 = 1;
pub const CA33_MAX_BATCH_CREATURES: usize = 30;
pub const CA33_MAX_BATCH_TICKS: u32 = 8;
pub const CA34_SAMPLED_GPU_RUNTIME_SCHEMA: &str = "alife.ca34.sampled_gpu_runtime.v1";
pub const CA34_SAMPLED_GPU_RUNTIME_SCHEMA_VERSION: u16 = 1;
pub const CA34_MAX_SAMPLED_TICKS: u32 = 16;
pub const CA42_RUNTIME_PREREQ_SCHEMA: &str = "alife.ca42.runtime_prereq_diagnostics.v1";
pub const CA42_RUNTIME_PREREQ_SCHEMA_VERSION: u16 = 1;
pub const FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA: &str = "alife.fvr07.production_asset_manifest.v1";
pub const FVR07_PRODUCTION_ASSET_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const S02_MAX_RUN_TICKS_PER_UPDATE: u32 = 4;
pub const FULL_GPU_NEURAL_RUNTIME_SCHEMA: &str = "alife.product.full_gpu_neural_runtime.v1";
pub const FULL_GPU_NEURAL_RUNTIME_SCHEMA_VERSION: u16 = 1;
pub const FVR01_PRODUCTION_FRONTEND_SCHEMA: &str = "alife.fvr01.production_voxel_frontend.v1";
pub const FVR01_PRODUCTION_FRONTEND_SCHEMA_VERSION: u16 = 1;
pub const FVR01_PROFILE_BUDGET_SCHEMA_VERSION: u16 = 1;
pub const FULL_GPU_NEURAL_RUNTIME_MAX_TICKS: u32 = 16;
pub const GPU_LONGRUN_SOAK_SCHEMA: &str = "alife.product.gpu_longrun_soak.v1";
pub const GPU_LONGRUN_SOAK_SCHEMA_VERSION: u16 = 1;
pub const GPU_LONGRUN_SOAK_DEFAULT_TICKS: u32 = 1000;
pub const GPU_LONGRUN_SOAK_DEFAULT_REPORT_EVERY: u32 = 100;
pub const GPU_LONGRUN_SOAK_MAX_TICKS_MANUAL: u32 = 10_000;
pub const GPU_SUSTAINED_LEARNING_SOAK_SCHEMA: &str = "alife.product.gpu_sustained_learning_soak.v1";
pub const GPU_SUSTAINED_LEARNING_SOAK_SCHEMA_VERSION: u16 = 1;
pub const GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_TICKS: u32 = 1000;
pub const GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_REPORT_EVERY: u32 = 100;
pub const GPU_SUSTAINED_LEARNING_SOAK_DEFAULT_EPISODE_TICKS: u32 = 32;
pub const GPU_SUSTAINED_LEARNING_SOAK_MAX_TICKS_MANUAL: u32 = 10_000;
