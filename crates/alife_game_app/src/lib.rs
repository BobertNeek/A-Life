//! Application launch, runtime scheduling, controls, and presentation.
//! The default build is headless; Bevy and GPU support are feature-gated.

#![recursion_limit = "256"]

mod prelude {
    pub(crate) use std::path::{Path, PathBuf};

    pub(crate) use alife_core::{
        heuristic_baseline_arbitrate, ActionArbitrationConfig, ActionDecision, ActionId,
        ActionKind, ActionProposal, ActionTarget, BrainTickInput, BrainTickStatus, Confidence,
        ContractDiagnostic, CreatureMind, DurationTicks, ExperiencePatch, HomeostaticSnapshot,
        Intensity, NeuralProjectionSchema, NormalizedScalar, OrganismId, PhysicalContactKind,
        PolicyBackend, ReferenceActionFailure, ScaffoldContractError, SleepPhase, Tick, Validate,
        Vec3f, WorldEntityId,
    };
    pub(crate) use alife_world::persistence::{
        AssetManifest, GpuRuntimeActiveProfileCaps, GpuRuntimeAdapterIdentity,
        GpuRuntimeAuthorityState, GpuRuntimeClassBucketAllocation, GpuRuntimeResidencySlots,
        GpuRuntimeSafeCheckpoint, GpuRuntimeSaveState, GpuRuntimeShaderAbiVersions,
        PersistenceError, PortableSaveFile, RuntimeConfig, WorldObjectSaveState,
        FVR06_GPU_RUNTIME_STATE_SCHEMA, FVR06_GPU_RUNTIME_STATE_SCHEMA_VERSION,
    };
    pub(crate) use alife_world::{
        CreatureAppearanceGenome, CreatureWorldAnchor, EcologyMetrics, EcologyZoneId,
        HeadlessActionIds, HeadlessBrainHarness, HeadlessSensoryReport, HeadlessWorld,
        PersistentVoxelProfileId, PersistentVoxelWorldBackend, TerrainZoneKind, WorldObjectKind,
    };
    pub(crate) use serde::{Deserialize, Serialize};
    pub(crate) use thiserror::Error;
}

mod schema;
pub use schema::*;

mod production_run_mode;
pub use production_run_mode::*;

mod app_shell;
pub use app_shell::*;

mod composite_population_runtime;
pub use composite_population_runtime::*;

mod habitat_lab_commands;
pub use habitat_lab_commands::*;

mod brain_policy;
pub use brain_policy::*;

mod app_bundle_ingestion;
pub use app_bundle_ingestion::*;

mod visible_world;
pub use visible_world::*;

mod creature_visuals;
pub use creature_visuals::*;

mod creature_part_catalog;
pub use creature_part_catalog::*;

mod creature_part_genetics;
pub use creature_part_genetics::*;

mod creature_part_assets;
pub use creature_part_assets::*;

mod creature_coat;
pub use creature_coat::*;

mod creature_assembly;
pub use creature_assembly::*;

mod creature_surface_details;
pub use creature_surface_details::*;
mod creature_visual_geometry;
pub use creature_visual_geometry::*;

mod creature_part_pose;
pub use creature_part_pose::*;

mod live_brain_bridge;
pub use live_brain_bridge::*;

#[cfg(feature = "gpu-runtime")]
pub use alife_runtime::{
    merge_gpu_checkpoint_manifest_entries, GpuBrainCheckpointWrite, GpuBrainSidecarCapture,
    GpuCheckpointAssetStore, GpuDurableSaveManifest, GpuLoadedSaveManifest, GpuRuntimeError,
    GpuSaveManifestCasOutcome, GpuSaveManifestDigest, GpuSleepConsolidationDriver,
    GpuSleepScheduleEvent, GpuSleepScheduler, RestoredGpuBrainCheckpoint, RestoredRetainedLearning,
    RetainedLearningCapture,
};

#[cfg(feature = "gpu-runtime")]
mod founder_import;
#[cfg(feature = "gpu-runtime")]
pub use founder_import::*;

mod ecology_loop;
pub use ecology_loop::*;

mod runtime_prereq_diagnostics;
pub use runtime_prereq_diagnostics::*;

mod environment_launcher;
pub use environment_launcher::*;

mod production_voxel_frontend;
pub use production_voxel_frontend::*;

#[cfg(feature = "foundation-training")]
mod foundation_acquisition_rehearsal;
#[cfg(feature = "foundation-training")]
pub use foundation_acquisition_rehearsal::FoundationAcquisitionRehearsalReceipt;
#[cfg(feature = "foundation-training")]
mod foundation_archived_success_replay;
#[cfg(feature = "foundation-training")]
pub use foundation_archived_success_replay::{
    inspect_archived_successes, inspect_foundation_checkpoint, ArchivedSuccessEntry,
    ArchivedSuccessInspection, ArchivedSuccessManifest, ArchivedSuccessReplayReceipt,
    FoundationCheckpointFileDigest, FoundationCheckpointInspection,
};
#[cfg(feature = "foundation-training")]
mod foundation_critic_correction;
#[cfg(feature = "foundation-training")]
mod foundation_grab_food;
#[cfg(feature = "foundation-training")]
mod foundation_training;
#[cfg(feature = "foundation-training")]
mod foundation_training_cycle;
#[cfg(feature = "foundation-training")]
pub use foundation_grab_food::{
    FoundationCurriculumRewardRow, FoundationGrabAcquisition, FoundationGrabFoodSetup,
};
#[cfg(feature = "foundation-training")]
mod foundation_training_output;
#[cfg(feature = "foundation-training")]
mod foundation_training_warmup;
mod new_game_lifecycle;
#[cfg(feature = "foundation-training")]
pub use foundation_training::*;
#[cfg(feature = "foundation-training")]
pub use foundation_training_cycle::*;
#[cfg(feature = "foundation-training")]
mod critic_qualification;
#[cfg(feature = "foundation-training")]
pub use critic_qualification::{
    evaluate_critic_capture, run_critic_gradient_regression, run_critic_qualification,
};
#[cfg(feature = "foundation-training")]
pub use foundation_training_warmup::*;
pub use new_game_lifecycle::*;

mod curated_founder_reset;
pub use curated_founder_reset::*;

mod curated_founder_materializer;

#[cfg(feature = "gpu-runtime")]
mod curated_founder_staging;

#[cfg(feature = "bevy-app")]
mod production_terrain;
#[cfg(feature = "bevy-app")]
pub use production_terrain::{
    Fvr11ProductionTerrainLayer, Fvr11ProductionTerrainSceneResource, Fvr11TerrainSurfaceRole,
    FVR11_PRODUCTION_TERRAIN_VISUAL_VERSION,
};

#[cfg(feature = "bevy-app")]
mod terrain_mesh;

#[cfg(feature = "bevy-app")]
mod terrain_materials;
#[cfg(feature = "bevy-app")]
pub use terrain_materials::Fvr11ProductionTerrainMaterialContract;

#[cfg(feature = "bevy-app")]
mod terrain_water;

#[cfg(feature = "bevy-app")]
mod terrain_dressing;

#[cfg(feature = "bevy-app")]
mod terrain_lighting;
#[cfg(feature = "bevy-app")]
pub use terrain_lighting::{Fvr11ProductionContactShadow, Fvr11ProductionTerrainLightingMarker};

#[cfg(feature = "bevy-app")]
mod production_voxel_renderer;
#[cfg(feature = "bevy-app")]
pub use production_voxel_renderer::*;

#[cfg(all(feature = "bevy-app", feature = "gpu-runtime"))]
mod production_conversation_lineage_ui;
#[cfg(all(feature = "bevy-app", feature = "gpu-runtime"))]
pub use production_conversation_lineage_ui::*;

mod runtime_playback;
pub use runtime_playback::RuntimePlaybackState;

mod double_buffered_scheduler;
pub use double_buffered_scheduler::*;

mod motor_ring;
pub use motor_ring::*;

mod production_assets;
pub use production_assets::*;

mod gpu_authority_telemetry;
pub use gpu_authority_telemetry::*;

#[cfg(feature = "gpu-runtime")]
mod gpu_live_runtime;
#[cfg(feature = "gpu-runtime")]
pub use gpu_live_runtime::*;

#[cfg(feature = "gpu-runtime")]
mod gpu_evidence;
#[cfg(feature = "gpu-runtime")]
pub use gpu_evidence::*;

#[cfg(feature = "bevy-app")]
mod bevy_entity_map;
#[cfg(feature = "bevy-app")]
pub use bevy_entity_map::BevyEntityMap;

#[cfg(feature = "bevy-app")]
pub mod bevy_shell;

#[cfg(test)]
mod tests;

#[cfg(feature = "foundation-training")]
pub mod simulator_rehearsal;
