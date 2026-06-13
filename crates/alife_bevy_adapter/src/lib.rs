//! v0 scaffold: Bevy/Avian adapter boundary, not cognitive runtime logic.
//!
//! This crate converts Bevy ECS state into stable `alife_core` contracts and
//! converts structured core actions back into engine-side commands/outcomes.
//! Bevy and Avian types stay here; they never flow into `alife_core`.

pub mod action;
#[cfg(feature = "avian3d")]
pub mod avian3d;
pub mod components;
pub mod entity_map;
pub mod math;
pub mod plugin;
pub mod sensory;

pub use action::{
    execute_action_command, ActionAdapterContext, ActionAdapterFeedback, BevyActionFailure,
    BevyActionKind, BevyActionPlan, BevyReferenceActionAdapter, TargetAdapterState,
    ACTION_APPROACH, ACTION_EAT, ACTION_FLEE, ACTION_GRAB,
};
pub use components::{
    ActionSink, AffordanceTags, BrainTickProposals, CoreBrainMind, CreatureBody,
    LatestSensorySnapshot, PatchTelemetry, SensoryEmitter, SleepDriveDebug,
};
pub use entity_map::BevyEntityMap;
pub use math::{
    bevy_quat_to_core, bevy_transform_to_core_pose, bevy_vec3_to_core, core_pose_to_bevy_transform,
    core_quat_to_bevy, core_vec3_to_bevy,
};
pub use plugin::{
    cpu_brain_tick_system, execute_action_system, gather_sensory_system, measure_outcome_system,
    seal_patch_system, AdapterScheduleTrace, AdapterWorldTick, AlifeBevyAdapterPlugin,
    AlifeBevyAdapterSet,
};
pub use sensory::{
    gather_sensory_from_observed, CachedSensoryAdapter, ObservedBevyEntity, DEFAULT_HEARING_RADIUS,
    DEFAULT_VISION_RADIUS,
};
