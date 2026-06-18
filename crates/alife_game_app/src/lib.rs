//! G01-G13 playable-sim app surfaces.
//!
//! R13 keeps this crate headless and CI-safe by default while splitting the
//! previous monolithic `lib.rs` into focused modules. Bevy construction remains
//! behind the `bevy-app` feature and GPU runtime support remains optional.

mod prelude {
    pub(crate) use std::path::{Path, PathBuf};

    pub(crate) use alife_core::{
        cpu_reference_arbitrate, ActionArbitrationConfig, ActionId, ActionKind, ActionProposal,
        ActionTarget, BrainGenome, BrainScaleTier, BrainTickInput, BrainTickStatus, ConceptCellId,
        Confidence, CreatureMind, DurationTicks, GaussianClusterId, GenomeId, HomeostaticSnapshot,
        Intensity, LineageId, NormalizedScalar, OrganismId, PhysicalContactKind,
        ReferenceActionFailure, ScaffoldContractError, SleepPhase, TeacherLessonResponseChannel,
        TeacherPerceptionChannel, Tick, Validate, Vec3f, WorldEntityId,
    };
    pub(crate) use alife_school::{
        Curriculum, CurriculumStep, CurriculumStepKind, ExpectedObservation, FeedbackPolarity,
        HeadlessCurriculumRunner, LessonId, LessonResponse, LessonResponseKind,
        PatchLogLessonVerifier, SchoolEvidence, TeacherChannelContract, TeacherInputKind,
        TeacherPerceptualEvent, TeacherRole, TopologySummary, VerifierCheck,
        TEACHER_SCHOOL_SCHEMA_VERSION,
    };
    pub(crate) use alife_semantic::{
        FakeSemanticProvider, SemanticCodeDescriptor, SemanticConceptBinding,
        SemanticContextBundle, SemanticContextProvider, SemanticContextRequest,
        SemanticProviderCapabilityManifest, SemanticProviderConfig, SemanticProviderKind,
        G11_SEMANTIC_PROVIDER_SCHEMA, G11_SEMANTIC_PROVIDER_SCHEMA_VERSION,
    };
    pub(crate) use alife_world::persistence::{
        AssetManifest, BackendSelection, PersistenceError, PortableSaveFile, RuntimeConfig,
        SchoolSaveState, WorldObjectSaveState,
    };
    pub(crate) use alife_world::{
        EcologyMetrics, EcologyZoneId, HeadlessActionIds, HeadlessBrainHarness,
        HeadlessScenarioBuilder, HeadlessWorld, TerrainZone, TerrainZoneKind, WorldEditorSpawnSpec,
        WorldObjectKind,
    };
    pub(crate) use serde::{Deserialize, Serialize};
    pub(crate) use thiserror::Error;
}

mod schema;
pub use schema::*;

mod app_shell;
pub use app_shell::*;

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

mod population_social;
pub use population_social::*;

mod lifecycle_lineage;
pub use lifecycle_lineage::*;

mod school_mode;
pub use school_mode::*;

mod semantic_provider_display;
pub use semantic_provider_display::*;

mod gpu_product_telemetry;
pub use gpu_product_telemetry::*;

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
