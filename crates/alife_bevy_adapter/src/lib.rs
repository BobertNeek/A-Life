//! Bevy/Avian adapter boundary, not cognitive runtime logic.
//!
//! This crate converts Bevy ECS state into stable `alife_core` contracts and
//! converts structured core actions into engine-side plans. Authoritative world
//! and biology systems execute those plans and measure their outcomes.
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
    execute_action_command, plan_action_command, ActionAdapterContext, ActionAdapterFeedback,
    BevyActionFailure, BevyActionKind, BevyActionPlan, TargetAdapterState, ACTION_APPROACH,
    ACTION_EAT, ACTION_FLEE, ACTION_GRAB,
};
pub use components::{
    ActionSink, AdapterContractFailure, AdapterStage, AffordanceTags, CreatureBody,
    LatestSensorySnapshot, SensoryEmitter,
};
pub use entity_map::BevyEntityMap;
pub use math::{
    bevy_quat_to_core, bevy_transform_to_core_pose, bevy_vec3_to_core, core_pose_to_bevy_transform,
    core_quat_to_bevy, core_vec3_to_bevy,
};
pub use plugin::{
    gather_sensory_system, plan_action_system, AdapterScheduleTrace, AdapterWorldTick,
    AlifeBevyAdapterPlugin, AlifeBevyAdapterSet, AlifeReferenceAdapterPlugin,
};
pub use sensory::{
    gather_sensory_from_observed, gather_sensory_from_observed_with_profile, CachedSensoryAdapter,
    ObservedBevyEntity, ObserverSensoryProfile, DEFAULT_HEARING_RADIUS, DEFAULT_VISION_RADIUS,
};
