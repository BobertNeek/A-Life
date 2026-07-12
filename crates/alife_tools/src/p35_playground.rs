//! P35 playground/example orchestration helpers.
//!
//! This module is tooling-owned. It demonstrates the existing runtime contracts
//! through headless smoke paths and optional/manual integration notes without
//! making Bevy, GPU hardware, semantic providers, or school tooling mandatory
//! for `alife_core`.

use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(feature = "semantic-demo")]
use alife_core::{ConceptCellId, GaussianClusterId, Vec3f};
use alife_core::{PolicyBackend, ScaffoldContractError, TeacherPerceptionChannel};
use alife_gpu_backend::{
    GpuRuntimeBackendConfig, GpuRuntimeBackendKind, GpuRuntimeBoundary, GpuRuntimeReadbackGuard,
};
use alife_school::{
    Curriculum, HeadlessCurriculumRunner, PatchLogLessonVerifier, SchoolEvidence,
    TeacherChannelContract, TopologySummary, VerifierCheck,
};
#[cfg(feature = "semantic-demo")]
use alife_semantic::{
    FakeSemanticProvider, SemanticBoundaryManifest, SemanticCodeDescriptor, SemanticConceptBinding,
    SemanticContextProvider, SemanticContextRequest,
};
use alife_world::{
    persistence::{AssetManifest, PersistenceError, PortableSaveFile, RuntimeConfig},
    ScenarioFixture, ScenarioName,
};
use serde::Deserialize;
use thiserror::Error;

pub const P35_PLAYGROUND_MANIFEST_SCHEMA: &str = "alife.p35.playground_manifest.v1";
pub const P35_PLAYGROUND_MANIFEST_SCHEMA_VERSION: u16 = 1;
pub const P35_MAX_COMMITTED_SAMPLE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Error)]
pub enum PlaygroundError {
    #[error("core contract error: {0}")]
    Core(#[from] ScaffoldContractError),
    #[error("persistence/config error: {0}")]
    Persistence(#[from] PersistenceError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid P35 manifest field {field}: {message}")]
    InvalidManifest {
        field: &'static str,
        message: &'static str,
    },
}

#[derive(Debug, Clone)]
pub struct PlaygroundExampleConfig {
    pub p34_fixture_root: PathBuf,
    pub runtime_config: RuntimeConfig,
    pub asset_manifest: AssetManifest,
}

impl PlaygroundExampleConfig {
    pub fn from_p34_fixture_root(root: impl AsRef<Path>) -> Result<Self, PlaygroundError> {
        let root = root.as_ref().to_path_buf();
        let runtime_config = RuntimeConfig::from_json_file(root.join("tiny_config.json"))?;
        runtime_config.validate()?;
        let asset_manifest = AssetManifest::from_json_file(root.join("tiny_asset_manifest.json"))?;
        asset_manifest.validate_with_root(&root)?;
        Ok(Self {
            p34_fixture_root: root,
            runtime_config,
            asset_manifest,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeadlessPlaygroundReport {
    pub seed: u64,
    pub backend_selected: String,
    pub sealed_patch_count: usize,
    pub packed_log_count: usize,
    pub world_signature: Vec<String>,
    pub drive_hormone_debug: String,
    pub action_debug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveLoadDemoReport {
    pub save_id: String,
    pub seed: u64,
    pub world_entity_count: usize,
    pub stable_id_remap_available: bool,
    pub engine_local_ids_serialized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolDemoReport {
    pub perception_event_count: usize,
    pub verifier_passed: bool,
    pub direct_motor_bypass: bool,
    pub hidden_vector_injection: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticDemoReport {
    pub missing_provider_tolerated: bool,
    pub fake_provider_context_available: bool,
    pub provider_required_for_core_path: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuFallbackDemoReport {
    pub requested_backend: String,
    pub selected_backend: String,
    pub cpu_fallback: bool,
    pub active_bulk_readback_allowed: bool,
    pub diagnostic_export_boundary_allowed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaygroundManifestValidationReport {
    pub checked_paths: usize,
    pub manual_optional_commands: usize,
    pub largest_committed_sample_bytes: u64,
    pub documented_commands: Vec<String>,
}

pub fn run_headless_cpu_demo(
    config: PlaygroundExampleConfig,
) -> Result<HeadlessPlaygroundReport, PlaygroundError> {
    config.runtime_config.validate()?;
    config
        .asset_manifest
        .validate_with_root(&config.p34_fixture_root)?;

    let run = ScenarioFixture::with_seed(
        ScenarioName::FoodSeeking,
        config.runtime_config.deterministic_seed,
    )?
    .run()?;
    let first_patch = run.first_patch();
    let packed_log_count = run
        .ticks
        .iter()
        .filter(|tick| tick.brain.packed_record.is_some())
        .count();
    let action_debug = format!(
        "{:?}:{:?}:success={}",
        first_patch.decision().selected_action.kind,
        first_patch.decision().selected_action.target_entity,
        first_patch.outcome().success
    );
    Ok(HeadlessPlaygroundReport {
        seed: config.runtime_config.deterministic_seed,
        backend_selected: format!("{:?}", PolicyBackend::HeuristicBaseline),
        sealed_patch_count: run.patches.len(),
        packed_log_count,
        world_signature: run.world_signature,
        drive_hormone_debug: format!(
            "hunger={:.3} fatigue={:.3} dopamine={:.3} cortisol={:.3}",
            run.final_homeostasis.drives.hunger,
            run.final_homeostasis.drives.fatigue,
            run.final_homeostasis.hormones.dopamine,
            run.final_homeostasis.hormones.cortisol
        ),
        action_debug,
    })
}

pub fn run_save_load_demo(root: impl AsRef<Path>) -> Result<SaveLoadDemoReport, PlaygroundError> {
    let root = root.as_ref();
    let save_path = root.join("tiny_save.json");
    let save_text = fs::read_to_string(&save_path)?;
    let save = PortableSaveFile::from_json_str(&save_text)?;
    save.validate_with_asset_root(root)?;
    let restored = save.restore_headless_world()?;
    Ok(SaveLoadDemoReport {
        save_id: save.save_id,
        seed: save.deterministic_seed,
        world_entity_count: restored.stable_signature().len(),
        stable_id_remap_available: true,
        engine_local_ids_serialized: contains_engine_local_token(&save_text),
    })
}

pub fn run_school_teacher_demo() -> Result<SchoolDemoReport, PlaygroundError> {
    let contract = TeacherChannelContract::grounded_default();
    let mut runner = HeadlessCurriculumRunner::new(Curriculum::grounded_object_food_poison());
    let dispatch = runner.dispatch_current()?;
    let direct_motor_bypass = contract.direct_motor_bypass_allowed
        || dispatch
            .perception_events
            .iter()
            .any(|event| event.direct_motor_bypass());
    let hidden_vector_injection = contract.hidden_vector_injection_allowed
        || dispatch
            .perception_events
            .iter()
            .any(|event| event.hidden_vector_injection_allowed());

    let run = ScenarioFixture::named(ScenarioName::TeacherPerceptionEvent)?.run()?;
    let evidence = SchoolEvidence::new(&run.patches)
        .with_memory_record_count(run.memory_record_count)
        .with_topology_summary(TopologySummary {
            concept_count: run.topology_concept_count,
            edge_count: run.topology_edge_count,
            simplex_count: run.topology_simplex_count,
            gap_count: run.topology_gap_ids.len(),
        });
    let verification = PatchLogLessonVerifier.verify_checks(
        &[
            VerifierCheck::HeardToken {
                token_id: 77,
                channel: TeacherPerceptionChannel::Hearing,
            },
            VerifierCheck::NoDirectTeacherActionSelection,
            VerifierCheck::SelectedByArbitration,
            VerifierCheck::MinimumMemoryRecords(1),
            VerifierCheck::MinimumTopologyConcepts(2),
        ],
        &evidence,
    )?;
    let _advanced = runner.observe_verification(&verification)?;

    Ok(SchoolDemoReport {
        perception_event_count: dispatch.perception_events.len(),
        verifier_passed: verification.passed,
        direct_motor_bypass,
        hidden_vector_injection,
    })
}

pub fn run_semantic_fake_provider_demo() -> Result<SemanticDemoReport, PlaygroundError> {
    #[cfg(feature = "semantic-demo")]
    {
        let boundary = SemanticBoundaryManifest::INTERNAL_PRIOR;
        let request = SemanticContextRequest::new(Vec3f::new(0.5, 0.0, 1.0))
            .with_gaussian_observation(GaussianClusterId(35), 0.8, 2.0, Vec3f::new(0.5, 0.0, 1.0))
            .with_semantic_binding(SemanticConceptBinding {
                concept_id: ConceptCellId(35),
                salience: 0.9,
            })
            .with_semantic_descriptor(SemanticCodeDescriptor {
                codebook_id: 3,
                descriptor: [5_i8; 32],
                salience: 0.7,
            });
        let provider = FakeSemanticProvider::new();
        let bundle = provider.build_context_bundle(&request)?;

        Ok(SemanticDemoReport {
            missing_provider_tolerated: boundary.private_prior
                && !boundary.can_issue_actions
                && !boundary.can_rewrite_weights,
            fake_provider_context_available: bundle.gaussian_context.is_some()
                && bundle.semantic_context.is_some(),
            provider_required_for_core_path: false,
        })
    }
    #[cfg(not(feature = "semantic-demo"))]
    {
        Ok(SemanticDemoReport {
            missing_provider_tolerated: true,
            fake_provider_context_available: false,
            provider_required_for_core_path: false,
        })
    }
}

pub fn run_gpu_fallback_demo() -> Result<GpuFallbackDemoReport, PlaygroundError> {
    let status = GpuRuntimeBackendConfig::request(GpuRuntimeBackendKind::GpuStatic)
        .with_hardware_available(false)
        .select_backend()?;
    let active_guard = GpuRuntimeReadbackGuard::active_tick();
    let export_guard = GpuRuntimeReadbackGuard::after_frame_boundary();
    Ok(GpuFallbackDemoReport {
        requested_backend: format!("{:?}", status.requested),
        selected_backend: format!("{:?}", status.selected),
        cpu_fallback: status.selected == GpuRuntimeBackendKind::CpuReference,
        active_bulk_readback_allowed: active_guard.permits_bulk_neural_readback(),
        diagnostic_export_boundary_allowed: export_guard
            .validate_export_request(GpuRuntimeBoundary::DiagnosticExport)
            .is_ok(),
    })
}

pub fn validate_playground_manifest(
    manifest_path: impl AsRef<Path>,
) -> Result<PlaygroundManifestValidationReport, PlaygroundError> {
    let manifest_path = manifest_path.as_ref();
    let workspace = find_workspace_root(manifest_path)?;
    let text = fs::read_to_string(manifest_path)?;
    let manifest: PlaygroundExampleManifest = serde_json::from_str(&text)?;
    manifest.validate(&workspace)
}

#[derive(Debug, Deserialize)]
struct PlaygroundExampleManifest {
    schema: String,
    schema_version: u16,
    documentation_path: String,
    paths: Vec<PlaygroundManifestPath>,
    commands: Vec<PlaygroundManifestCommand>,
}

#[derive(Debug, Deserialize)]
struct PlaygroundManifestPath {
    label: String,
    relative_path: String,
    required: bool,
}

#[derive(Debug, Deserialize)]
struct PlaygroundManifestCommand {
    name: String,
    command: String,
    manual: bool,
    optional_feature: Option<String>,
}

impl PlaygroundExampleManifest {
    fn validate(
        &self,
        workspace: &Path,
    ) -> Result<PlaygroundManifestValidationReport, PlaygroundError> {
        if self.schema != P35_PLAYGROUND_MANIFEST_SCHEMA {
            return Err(PlaygroundError::InvalidManifest {
                field: "schema",
                message: "unexpected P35 playground manifest schema",
            });
        }
        if self.schema_version != P35_PLAYGROUND_MANIFEST_SCHEMA_VERSION {
            return Err(PlaygroundError::InvalidManifest {
                field: "schema_version",
                message: "unsupported P35 playground manifest schema version",
            });
        }
        let docs_path = workspace.join(&self.documentation_path);
        let docs = fs::read_to_string(&docs_path)?;
        let mut checked_paths = 0_usize;
        let mut largest = fs::metadata(&docs_path)?.len();
        for path in &self.paths {
            if path.label.is_empty() || path.relative_path.is_empty() {
                return Err(PlaygroundError::InvalidManifest {
                    field: "paths",
                    message: "path labels and relative paths are required",
                });
            }
            let resolved = workspace.join(&path.relative_path);
            if path.required && !resolved.is_file() {
                return Err(PlaygroundError::InvalidManifest {
                    field: "paths",
                    message: "required sample path does not exist",
                });
            }
            if resolved.is_file() {
                largest = largest.max(fs::metadata(&resolved)?.len());
                checked_paths += 1;
            }
        }
        if largest >= P35_MAX_COMMITTED_SAMPLE_BYTES {
            return Err(PlaygroundError::InvalidManifest {
                field: "paths",
                message: "committed P35 samples must remain small",
            });
        }
        let mut manual_optional_commands = 0_usize;
        let mut documented_commands = Vec::with_capacity(self.commands.len());
        for command in &self.commands {
            if command.name.is_empty() || command.command.is_empty() {
                return Err(PlaygroundError::InvalidManifest {
                    field: "commands",
                    message: "command name and body are required",
                });
            }
            if command.manual || command.optional_feature.is_some() {
                manual_optional_commands += 1;
            }
            if !docs.contains(&command.name) || !docs.contains(&command.command) {
                return Err(PlaygroundError::InvalidManifest {
                    field: "commands",
                    message: "documented command missing from playground docs",
                });
            }
            documented_commands.push(command.command.clone());
        }
        Ok(PlaygroundManifestValidationReport {
            checked_paths,
            manual_optional_commands,
            largest_committed_sample_bytes: largest,
            documented_commands,
        })
    }
}

fn contains_engine_local_token(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "entity(",
        "bevy::",
        "avian::",
        "wgpu::",
        "rendererhandle",
        "windowhandle",
        "oswindow",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn find_workspace_root(path: &Path) -> Result<PathBuf, PlaygroundError> {
    let mut cursor = if path.is_file() {
        path.parent().map(Path::to_path_buf)
    } else {
        Some(path.to_path_buf())
    };
    while let Some(dir) = cursor {
        if dir.join("Cargo.toml").is_file() && dir.join("crates").is_dir() {
            return Ok(dir);
        }
        cursor = dir.parent().map(Path::to_path_buf);
    }
    Err(PlaygroundError::InvalidManifest {
        field: "manifest_path",
        message: "could not locate workspace root",
    })
}
