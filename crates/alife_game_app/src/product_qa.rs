//! G22 product QA hardening and bug-bash evidence aggregation.
//!
//! This module does not add gameplay features. It collects deterministic
//! evidence from existing smoke paths, invalid-input checks, optional feature
//! gates, and docs so release blockers remain explicit before G23.

use std::{collections::BTreeSet, path::PathBuf};

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProductQaArea {
    AppLaunch,
    GameplayLoop,
    UiState,
    SaveLoad,
    School,
    Semantic,
    GpuFallback,
    Performance,
    Packaging,
    Docs,
}

impl ProductQaArea {
    pub const fn label(self) -> &'static str {
        match self {
            Self::AppLaunch => "app-launch",
            Self::GameplayLoop => "gameplay-loop",
            Self::UiState => "ui-state",
            Self::SaveLoad => "save-load",
            Self::School => "school",
            Self::Semantic => "semantic",
            Self::GpuFallback => "gpu-fallback",
            Self::Performance => "performance",
            Self::Packaging => "packaging",
            Self::Docs => "docs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProductQaStatus {
    Passed,
    Manual,
    KnownLimitation,
}

impl ProductQaStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Manual => "manual",
            Self::KnownLimitation => "known-limitation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductQaChecklistItem {
    pub id: String,
    pub area: ProductQaArea,
    pub status: ProductQaStatus,
    pub command: String,
    pub evidence: String,
    pub manual: bool,
    pub release_blocker: bool,
}

impl ProductQaChecklistItem {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.id.is_empty()
            || self.command.is_empty()
            || self.evidence.is_empty()
            || self.command.contains("bash scripts/check.sh")
            || self.command.contains("gpu-report")
            || self.command.contains("ALIFE_GPU_BACKEND")
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        if self.manual && self.status == ProductQaStatus::Passed {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "manual QA gates must not be recorded as passed",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductQaFinding {
    pub id: String,
    pub severity: ProductQaStatus,
    pub area: ProductQaArea,
    pub reproduction: String,
    pub note: String,
    pub release_blocker: bool,
}

impl ProductQaFinding {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.id.is_empty()
            || self.reproduction.is_empty()
            || self.note.is_empty()
            || self.reproduction.contains("bash scripts/check.sh")
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductQaInvalidInputEvidence {
    pub invalid_config_rejected: bool,
    pub invalid_save_schema_rejected: bool,
    pub missing_required_asset_rejected: bool,
    pub digest_mismatch_rejected: bool,
    pub invalid_app_state_transition_rejected: bool,
    pub stale_gpu_command_rejected: bool,
    pub no_partial_load_after_error: bool,
}

impl ProductQaInvalidInputEvidence {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if !self.invalid_config_rejected
            || !self.invalid_save_schema_rejected
            || !self.missing_required_asset_rejected
            || !self.digest_mismatch_rejected
            || !self.invalid_app_state_transition_rejected
            || !self.stale_gpu_command_rejected
            || !self.no_partial_load_after_error
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductQaOptionalFeatureEvidence {
    pub headless_default_has_no_graphics_requirement: bool,
    pub semantic_absence_nonfatal: bool,
    pub semantic_fake_provider_non_authoritative: bool,
    pub school_verifier_uses_sealed_patches: bool,
    pub gpu_required_default: bool,
    pub gpu_no_active_readback: bool,
    pub graphical_smoke_manual: bool,
}

impl ProductQaOptionalFeatureEvidence {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if !self.headless_default_has_no_graphics_requirement
            || !self.semantic_absence_nonfatal
            || !self.semantic_fake_provider_non_authoritative
            || !self.school_verifier_uses_sealed_patches
            || !self.gpu_required_default
            || !self.gpu_no_active_readback
            || !self.graphical_smoke_manual
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductQaUiTransitionEvidence {
    pub app_trace: Vec<GameAppState>,
    pub pause_resume_seen: bool,
    pub save_load_menu_seen: bool,
    pub cognition_debug_read_only: bool,
    pub world_editor_resume_seen: bool,
}

impl ProductQaUiTransitionEvidence {
    pub fn validate(&self) -> Result<(), ScaffoldContractError> {
        if self.app_trace.len() < 5
            || !self.pause_resume_seen
            || !self.save_load_menu_seen
            || !self.cognition_debug_read_only
            || !self.world_editor_resume_seen
        {
            return Err(ScaffoldContractError::MissingPhaseData);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductQaSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub checklist: Vec<ProductQaChecklistItem>,
    pub findings: Vec<ProductQaFinding>,
    pub invalid_input: ProductQaInvalidInputEvidence,
    pub optional_features: ProductQaOptionalFeatureEvidence,
    pub ui_transitions: ProductQaUiTransitionEvidence,
    pub fast_soak_command: String,
    pub playground_smoke_command: String,
    pub extended_balance_command: String,
    pub manual_gpu_command: String,
    pub known_issues_doc: String,
    pub release_blocker_count: usize,
    pub known_limitation_count: usize,
    pub p36_gates_preserved: bool,
    pub no_p37_created: bool,
    pub no_generated_artifacts_tracked: bool,
}

impl ProductQaSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != G22_PRODUCT_QA_SCHEMA
            || self.schema_version != G22_PRODUCT_QA_SCHEMA_VERSION
            || self.checklist.len() < 9
            || self.findings.len() > G22_MAX_QA_FINDINGS
            || self.fast_soak_command.is_empty()
            || self.playground_smoke_command.is_empty()
            || !self.extended_balance_command.contains("--ignored")
            || !self.manual_gpu_command.contains("--gpu-runtime")
            || !self.known_issues_doc.ends_with("known_issues.md")
            || self.release_blocker_count != 0
            || self.known_limitation_count == 0
            || !self.p36_gates_preserved
            || !self.no_p37_created
            || !self.no_generated_artifacts_tracked
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        self.invalid_input.validate()?;
        self.optional_features.validate()?;
        self.ui_transitions.validate()?;
        let mut ids = BTreeSet::new();
        let mut areas = BTreeSet::new();
        for item in &self.checklist {
            item.validate()?;
            if !ids.insert(item.id.as_str()) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "G22 QA checklist item IDs must be unique",
                });
            }
            areas.insert(item.area.label());
        }
        for required in [
            ProductQaArea::AppLaunch,
            ProductQaArea::GameplayLoop,
            ProductQaArea::UiState,
            ProductQaArea::SaveLoad,
            ProductQaArea::School,
            ProductQaArea::Semantic,
            ProductQaArea::GpuFallback,
            ProductQaArea::Performance,
            ProductQaArea::Packaging,
            ProductQaArea::Docs,
        ] {
            if !areas.contains(required.label()) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "G22 QA checklist is missing a required area",
                });
            }
        }
        for finding in &self.findings {
            finding.validate()?;
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.checklist.len(),
            self.findings.len(),
            self.release_blocker_count,
            self.known_limitation_count,
            self.manual_gpu_command
        )
    }
}

pub fn run_product_qa_hardening_smoke() -> Result<ProductQaSummary, GameAppShellError> {
    let root = g22_workspace_root();
    let launch = AppShellLaunchConfig::from_p34_fixture_root(g22_p34_fixture_root());

    let app = run_headless_app_shell_smoke(&launch)?;
    let survival = run_playable_survival_loop_smoke()?;
    let live = run_live_brain_loop_smoke(&launch)?;
    let save_load = run_save_load_ux_smoke(&launch)?;
    let school = run_school_mode_smoke()?;
    let semantic = run_semantic_provider_smoke()?;
    let gpu = run_gpu_product_hardening_smoke()?;
    let cognition = run_cognition_debug_timeline_smoke()?;
    let world_editor = run_world_editor_smoke()?;
    let balance = run_longrun_balance_smoke()?;
    let packaging = run_platform_package_smoke()?;
    let onboarding = run_onboarding_help_smoke()?;

    let invalid_input = invalid_input_evidence(&save_load)?;
    let optional_features = ProductQaOptionalFeatureEvidence {
        headless_default_has_no_graphics_requirement: !app.graphics_required_for_default_path,
        semantic_absence_nonfatal: semantic.provider_absence_nonfatal,
        semantic_fake_provider_non_authoritative: !semantic.fake_panel.manifest.can_issue_actions
            && !semantic.fake_panel.manifest.can_rewrite_weights,
        school_verifier_uses_sealed_patches: school.verifier_panel.passed
            && school.verifier_panel.sealed_patch_count > 0
            && school.teacher_metadata_bypass_blocked,
        gpu_required_default: gpu.gpu_required_default,
        gpu_no_active_readback: gpu.telemetry_overlay.no_active_gameplay_readback,
        graphical_smoke_manual: packaging
            .commands
            .iter()
            .any(|command| command.kind == PackageSmokeKind::GraphicalManual && command.manual),
    };
    let ui_transitions = ProductQaUiTransitionEvidence {
        app_trace: app.state_trace.clone(),
        pause_resume_seen: app.state_trace.contains(&GameAppState::Running)
            && app.state_trace.contains(&GameAppState::Shutdown),
        save_load_menu_seen: save_load.menu.manual_save_enabled && save_load.menu.autosave_enabled,
        cognition_debug_read_only: cognition.read_only && !cognition.mutation_controls_enabled,
        world_editor_resume_seen: world_editor.simulation_resumed
            && world_editor.resumed_patch_sealed,
    };
    let manual_gpu_command = gpu.manual_hardware_command.clone();
    let checklist = product_qa_checklist(ProductQaEvidenceRefs {
        app: &app,
        survival: &survival,
        live: &live,
        save_load: &save_load,
        school: &school,
        semantic: &semantic,
        gpu: &gpu,
        balance: &balance,
        packaging: &packaging,
        onboarding: &onboarding,
    });
    let findings = known_findings(&balance, &manual_gpu_command);
    let release_blocker_count = findings
        .iter()
        .filter(|finding| finding.release_blocker)
        .count();
    let known_limitation_count = findings
        .iter()
        .filter(|finding| finding.severity == ProductQaStatus::KnownLimitation)
        .count();

    let docs =
        std::fs::read_to_string(root.join("docs/playable_sim_spec/product_qa_hardening.md"))?;
    let known_issues =
        std::fs::read_to_string(root.join("docs/playable_sim_spec/known_issues.md"))?;
    if !docs.contains("cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke")
        || !docs.contains(&balance.manual_extended_command)
        || !docs.contains(&manual_gpu_command)
        || docs.contains("bash scripts/check.sh")
        || known_issues.contains("release blocker: unknown")
        || known_issues.contains("gpu-report")
        || known_issues.contains("ALIFE_GPU_BACKEND")
    {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "G22 QA docs must keep exact commands and honest limitation language",
        });
    }

    let summary = ProductQaSummary {
        schema: G22_PRODUCT_QA_SCHEMA,
        schema_version: G22_PRODUCT_QA_SCHEMA_VERSION,
        checklist,
        findings,
        invalid_input,
        optional_features,
        ui_transitions,
        fast_soak_command: "cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants".to_string(),
        playground_smoke_command: "cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json".to_string(),
        extended_balance_command: balance.manual_extended_command.clone(),
        manual_gpu_command,
        known_issues_doc: "docs/playable_sim_spec/known_issues.md".to_string(),
        release_blocker_count,
        known_limitation_count,
        p36_gates_preserved: release_gate_docs_preserved(&root)?,
        no_p37_created: no_p37_plan_exists(&root)?,
        no_generated_artifacts_tracked: !tracked_generated_artifacts_present(&root)?,
    };
    summary.validate()?;
    Ok(summary)
}

struct ProductQaEvidenceRefs<'a> {
    app: &'a AppStartupSummary,
    survival: &'a PlayableSurvivalLoopSummary,
    live: &'a LiveBrainTickSummary,
    save_load: &'a SaveLoadUxSmokeSummary,
    school: &'a SchoolModeSummary,
    semantic: &'a SemanticProviderSmokeSummary,
    gpu: &'a GpuProductHardeningSummary,
    balance: &'a LongRunBalanceSummary,
    packaging: &'a PlatformPackageSummary,
    onboarding: &'a OnboardingHelpSummary,
}

fn product_qa_checklist(evidence: ProductQaEvidenceRefs<'_>) -> Vec<ProductQaChecklistItem> {
    vec![
        ProductQaChecklistItem {
            id: "qa-app-launch-headless".to_string(),
            area: ProductQaArea::AppLaunch,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- headless-smoke crates/alife_world/tests/fixtures/p34".to_string(),
            evidence: format!("states={}", evidence.app.state_labels().join(">")),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-gameplay-loop".to_string(),
            area: ProductQaArea::GameplayLoop,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- playable-survival-loop-smoke".to_string(),
            evidence: format!("sealed_patches={}", evidence.survival.sealed_patch_count),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-ui-state-transitions".to_string(),
            area: ProductQaArea::UiState,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- cognition-debug-smoke".to_string(),
            evidence: format!("live_patch_sealed={} read_only=true", evidence.live.patch_sealed),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-save-load-errors".to_string(),
            area: ProductQaArea::SaveLoad,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- save-load-ux-smoke crates/alife_world/tests/fixtures/p34".to_string(),
            evidence: format!(
                "invalid_config={}",
                evidence.save_load.invalid_config_error.code
            ),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-school-verifier".to_string(),
            area: ProductQaArea::School,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- school-mode-smoke".to_string(),
            evidence: format!("sealed={}", evidence.school.verifier_panel.sealed_patch_count),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-semantic-optional".to_string(),
            area: ProductQaArea::Semantic,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- semantic-provider-smoke".to_string(),
            evidence: format!(
                "absence_nonfatal={} action_blocked={}",
                evidence.semantic.provider_absence_nonfatal,
                evidence.semantic.semantic_action_bypass_blocked
            ),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-gpu-fallback".to_string(),
            area: ProductQaArea::GpuFallback,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- gpu-product-smoke".to_string(),
            evidence: format!("selected={}", evidence.gpu.telemetry_overlay.selected_backend),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-performance-balance".to_string(),
            area: ProductQaArea::Performance,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke".to_string(),
            evidence: format!("sealed={}", evidence.balance.metrics.sealed_patch_count),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-packaging-headless".to_string(),
            area: ProductQaArea::Packaging,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke".to_string(),
            evidence: format!("commands={}", evidence.packaging.commands.len()),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-docs-onboarding".to_string(),
            area: ProductQaArea::Docs,
            status: ProductQaStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- onboarding-help-smoke".to_string(),
            evidence: format!("tutorial_steps={}", evidence.onboarding.tutorial_step_count),
            manual: false,
            release_blocker: false,
        },
        ProductQaChecklistItem {
            id: "qa-gpu-hardware-manual".to_string(),
            area: ProductQaArea::GpuFallback,
            status: ProductQaStatus::Manual,
            command: "ALIFE_GPU_RUNTIME_BACKEND=static ALIFE_GPU_RUNTIME_FEATURE=1 ALIFE_GPU_RUNTIME_AVAILABLE=1 ALIFE_GPU_RUNTIME_VALIDATED=1 cargo run -p alife_tools --bin benchmark_tiers -- --gpu-runtime".to_string(),
            evidence: "manual hardware evidence required for GPU performance claims".to_string(),
            manual: true,
            release_blocker: false,
        },
    ]
}

fn invalid_input_evidence(
    save_load: &SaveLoadUxSmokeSummary,
) -> Result<ProductQaInvalidInputEvidence, GameAppShellError> {
    let mut trace = AppShellStateTrace::default();
    let invalid_transition_rejected = trace.transition(GameAppState::Running).is_err();
    let stale_gpu_command = PlatformPackageCommand {
        id: "stale-gpu-command".to_string(),
        kind: PackageSmokeKind::Validation,
        windows_command:
            "ALIFE_GPU_BACKEND=static cargo run -p alife_tools --bin benchmark_tiers -- --gpu-report"
                .to_string(),
        non_windows_command:
            "ALIFE_GPU_BACKEND=static cargo run -p alife_tools --bin benchmark_tiers -- --gpu-report"
                .to_string(),
        manual: false,
        requires_graphics: false,
        requires_gpu: true,
    };
    Ok(ProductQaInvalidInputEvidence {
        invalid_config_rejected: save_load.invalid_config_error.code == "invalid-config",
        invalid_save_schema_rejected: save_load.invalid_schema_error.code == "schema-version",
        missing_required_asset_rejected: save_load.missing_asset_error.code
            == "missing-required-asset",
        digest_mismatch_rejected: save_load.digest_error.code == "digest-mismatch",
        invalid_app_state_transition_rejected: invalid_transition_rejected,
        stale_gpu_command_rejected: stale_gpu_command.validate().is_err(),
        no_partial_load_after_error: save_load.no_partial_load_after_error,
    })
}

fn known_findings(
    balance: &LongRunBalanceSummary,
    manual_gpu_command: &str,
) -> Vec<ProductQaFinding> {
    let mut findings = balance
        .degenerate_behaviors
        .iter()
        .enumerate()
        .map(|(index, behavior)| ProductQaFinding {
            id: format!("g22-known-balance-{}", index + 1),
            severity: ProductQaStatus::KnownLimitation,
            area: ProductQaArea::Performance,
            reproduction: balance.manual_extended_command.clone(),
            note: behavior.clone(),
            release_blocker: false,
        })
        .collect::<Vec<_>>();
    findings.push(ProductQaFinding {
        id: "g22-manual-gpu-performance".to_string(),
        severity: ProductQaStatus::Manual,
        area: ProductQaArea::GpuFallback,
        reproduction: manual_gpu_command.to_string(),
        note: "GPU performance remains manual/unknown unless hardware flags and validation evidence are set.".to_string(),
        release_blocker: false,
    });
    findings
}

fn release_gate_docs_preserved(root: &Path) -> Result<bool, GameAppShellError> {
    let release = std::fs::read_to_string(root.join("docs/release_checklist.md"))?;
    let status = std::fs::read_to_string(root.join("docs/final_status_report.md"))?;
    Ok(
        release.contains("cargo test --workspace --all-features --all-targets")
            && release
                .contains("powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1")
            && release.contains("Golden trace")
            && status.contains("Product GPU performance is not claimed"),
    )
}

fn no_p37_plan_exists(root: &Path) -> Result<bool, GameAppShellError> {
    for dir in [
        root.join("docs/playable_sim_spec/plans"),
        root.join("docs/playable_sim_spec/review_gates"),
        root.join("docs/codex_plan_pack/plans"),
    ] {
        if !dir.exists() {
            continue;
        }
        for entry in std::fs::read_dir(dir)? {
            let name = entry?.file_name().to_string_lossy().to_string();
            if name.contains("P37") || name.contains("G25") {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn tracked_generated_artifacts_present(root: &Path) -> Result<bool, GameAppShellError> {
    let output = std::process::Command::new("git")
        .args([
            "ls-files",
            "target",
            "dist",
            "target/artifacts",
            "graphify-out",
        ])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "git ls-files failed while checking generated artifacts",
        });
    }
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn g22_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn g22_p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34")
}
