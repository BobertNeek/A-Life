//! G01-G13 playable-sim app surfaces.
//!
//! R13 keeps this crate headless and CI-safe by default while splitting the
//! previous monolithic `lib.rs` into focused modules. Bevy construction remains
//! behind the `bevy-app` feature and GPU runtime support remains optional.

mod prelude {
    pub(crate) use std::path::{Path, PathBuf};

    pub(crate) use alife_core::{
        cpu_reference_arbitrate, ActionArbitrationConfig, ActionDecision, ActionId, ActionKind,
        ActionProposal, ActionTarget, BrainGenome, BrainScaleTier, BrainTickInput, BrainTickStatus,
        ChemistryModulation, CognitiveEdgeId, ConceptCellId, Confidence, ContractDiagnostic,
        CreatureMind, DurationTicks, EdgeRelationKind, ExperiencePatch, GapResolutionStatus,
        GaussianClusterId, GenomeId, HomeostaticParameters, HomeostaticSnapshot, Intensity,
        LineageId, NeuralProjectionSchema, NormalizedScalar, OrganismId, PhysicalContactKind,
        PostSealLifetimeDeltaBatch, PostSealLifetimeDeltaReceipt, ReferenceActionFailure,
        ScaffoldContractError, SleepPhase, TeacherLessonResponseChannel, TeacherPerceptionChannel,
        Tick, UnresolvedGapId, Validate, Vec3f, WorldEntityId,
    };
    pub(crate) use alife_school::{
        Curriculum, CurriculumStep, CurriculumStepKind, ExpectedObservation, FeedbackPolarity,
        HeadlessCurriculumRunner, LessonId, LessonResponse, LessonResponseKind,
        PatchLogLessonVerifier, SchoolEvidence, TeacherChannelContract, TeacherInputKind,
        TeacherPerceptualEvent, TeacherRole, TopologySummary, VerifierCheck,
        TEACHER_SCHOOL_SCHEMA_VERSION,
    };
    pub(crate) use alife_semantic::{
        BoundedSemanticEmbedding, FakeSemanticProvider, LlamaCppEmbeddingConfig,
        LlamaCppEmbeddingProvider, LlamaCppSlmPriorConfig, LlamaCppSlmPriorProvider,
        LocalSemanticModelManifest, LocalSlmPriorAsyncQueue, LocalSlmPriorOutput,
        LocalSlmPriorRequest, SemanticCodeDescriptor, SemanticConceptBinding,
        SemanticContextBundle, SemanticContextProvider, SemanticContextRequest,
        SemanticProviderCapabilityManifest, SemanticProviderConfig, SemanticProviderKind,
        CA26_EMBEDDING_PROJECTION_DIMS, CA26_LOCAL_MODEL_MANIFEST_SCHEMA,
        CA26_LOCAL_MODEL_MANIFEST_SCHEMA_VERSION, CA27_SLM_PRIOR_OUTPUT_SCHEMA,
        CA27_SLM_PRIOR_OUTPUT_SCHEMA_VERSION, G11_SEMANTIC_PROVIDER_SCHEMA,
        G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
    };
    pub(crate) use alife_world::persistence::{
        AssetManifest, BackendSelection, PersistenceError, PortableSaveFile, RuntimeConfig,
        SchoolSaveState, WorldObjectSaveState,
    };
    pub(crate) use alife_world::{
        EcologyMetrics, EcologyZoneId, HeadlessActionIds, HeadlessBrainHarness,
        HeadlessScenarioBuilder, HeadlessSensoryReport, HeadlessWorld, TerrainZone,
        TerrainZoneKind, WorldEditorSpawnSpec, WorldObjectKind,
    };
    pub(crate) use serde::{Deserialize, Serialize};
    pub(crate) use thiserror::Error;
}

mod schema;
pub use schema::*;

mod app_shell;
pub use app_shell::*;

mod app_bundle_ingestion;
pub use app_bundle_ingestion::*;

mod visible_world;
pub use visible_world::*;

mod creature_visuals;
pub use creature_visuals::*;

mod live_brain_bridge;
pub(crate) use live_brain_bridge::proposal;
pub use live_brain_bridge::*;

mod camera_inspector;
pub use camera_inspector::*;

mod survival_loop;
pub use survival_loop::*;

mod ecology_loop;
pub use ecology_loop::*;

mod feedback_polish;
pub use feedback_polish::*;

mod population_performance;
pub use population_performance::*;

mod longrun_balance;
pub use longrun_balance::*;

mod onboarding_help;
pub use onboarding_help::*;

mod onboarding_tutorial;
pub use onboarding_tutorial::*;

mod packaging_platform;
pub use packaging_platform::*;

mod runtime_prereq_diagnostics;
pub use runtime_prereq_diagnostics::*;

mod tester_feedback_capture;
pub use tester_feedback_capture::*;

mod alpha_art_assets;
pub use alpha_art_assets::*;

mod alpha_tick_stability;
pub use alpha_tick_stability::*;

mod product_qa;
pub use product_qa::*;

mod release_candidate;
pub use release_candidate::*;

mod graphical_playground;
pub use graphical_playground::*;

mod environment_launcher;
pub use environment_launcher::*;

mod production_voxel_frontend;
pub use production_voxel_frontend::*;

mod interactive_runtime;
pub use interactive_runtime::*;

mod double_buffered_scheduler;
pub use double_buffered_scheduler::*;

mod motor_ring;
pub use motor_ring::*;

mod homeostasis_runtime;
pub use homeostasis_runtime::*;

mod affordance_loop;
pub use affordance_loop::*;

mod hazard_recovery_loop;
pub use hazard_recovery_loop::*;

mod graphical_population;
pub use graphical_population::*;

mod graphical_ecology;
pub use graphical_ecology::*;

mod world_art_style;
pub use world_art_style::*;

mod procedural_world_streaming;
pub use procedural_world_streaming::*;

mod production_asset_pipeline;
pub use production_asset_pipeline::*;

mod true_25d_assets;
pub use true_25d_assets::*;

mod creature_animation_style;
pub use creature_animation_style::*;

mod drive_coupled_audio_vfx;
pub use drive_coupled_audio_vfx::*;

mod graphical_lifecycle;
pub use graphical_lifecycle::*;

mod graphical_school;
pub use graphical_school::*;

mod curriculum_authoring;
pub use curriculum_authoring::*;

mod behavior_tuning;
pub use behavior_tuning::*;

mod ecological_soak;
pub use ecological_soak::*;

mod population_social;
pub use population_social::*;

mod lifecycle_lineage;
pub use lifecycle_lineage::*;

mod school_mode;
pub use school_mode::*;

mod semantic_provider_display;
pub use semantic_provider_display::*;

mod real_semantic_provider;
pub use real_semantic_provider::*;

mod internal_slm_prior;
pub use internal_slm_prior::*;

mod topological_concept_overlay;
pub use topological_concept_overlay::*;

mod memory_history_journal;
pub use memory_history_journal::*;

mod neural_activity_profiler;
pub use neural_activity_profiler::*;

mod behavior_comparison_lab;
pub use behavior_comparison_lab::*;

mod advanced_gameplay_ux;
pub use advanced_gameplay_ux::*;

mod gpu_graphics_performance;
pub use gpu_graphics_performance::*;

mod content_tutorial_authoring;
pub use content_tutorial_authoring::*;

mod gpu_product_telemetry;
pub use gpu_product_telemetry::*;

mod gpu_live_runtime;
pub use gpu_live_runtime::*;

mod soak_isolation;
pub use soak_isolation::*;

mod world_editor;
pub use world_editor::*;

mod cognition_debug_timeline;
pub use cognition_debug_timeline::*;

mod save_load_ux;
pub use save_load_ux::*;

#[cfg(feature = "bevy-app")]
pub mod bevy_shell;

#[cfg(test)]
mod tests;
