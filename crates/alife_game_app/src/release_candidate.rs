//! G23 playable release-candidate evidence aggregation.
//!
//! This module does not add gameplay behavior. It collects existing smoke
//! evidence, exact validation commands, manual hardware/graphics gates, and
//! release-candidate report checks so G23 remains a bounded release gate before
//! R23.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use crate::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ReleaseCandidateArea {
    FullValidation,
    HeadlessPlayground,
    SaveLoad,
    Soak,
    Balance,
    ProductQa,
    Packaging,
    GpuManual,
    GraphicsManual,
    Docs,
}

impl ReleaseCandidateArea {
    pub const fn label(self) -> &'static str {
        match self {
            Self::FullValidation => "full-validation",
            Self::HeadlessPlayground => "headless-playground",
            Self::SaveLoad => "save-load",
            Self::Soak => "soak",
            Self::Balance => "balance",
            Self::ProductQa => "product-qa",
            Self::Packaging => "packaging",
            Self::GpuManual => "gpu-manual",
            Self::GraphicsManual => "graphics-manual",
            Self::Docs => "docs",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReleaseCandidateGateStatus {
    Passed,
    Manual,
    ExternalValidation,
}

impl ReleaseCandidateGateStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Manual => "manual",
            Self::ExternalValidation => "external-validation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseCandidateGate {
    pub id: String,
    pub area: ReleaseCandidateArea,
    pub status: ReleaseCandidateGateStatus,
    pub command: String,
    pub evidence: String,
    pub manual: bool,
    pub release_blocker: bool,
}

impl ReleaseCandidateGate {
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
        if self.manual && self.status == ReleaseCandidateGateStatus::Passed {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "manual G23 gates must not be recorded as passed without evidence",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseCandidateSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub candidate_id: String,
    pub playable_supported_path: String,
    pub gates: Vec<ReleaseCandidateGate>,
    pub release_blocker_count: usize,
    pub automated_gate_count: usize,
    pub manual_gate_count: usize,
    pub product_qa_release_blockers: usize,
    pub known_limitation_count: usize,
    pub gpu_performance_status: String,
    pub graphics_status: String,
    pub release_tag_created: bool,
    pub tag_proposal: String,
    pub report_path: String,
    pub p36_gates_preserved: bool,
    pub no_p37_created: bool,
    pub no_generated_artifacts_tracked: bool,
}

impl ReleaseCandidateSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != G23_RELEASE_CANDIDATE_SCHEMA
            || self.schema_version != G23_RELEASE_CANDIDATE_SCHEMA_VERSION
            || self.candidate_id.is_empty()
            || !self.playable_supported_path.contains("headless-cpu")
            || self.gates.len() < 10
            || self.gates.len() > G23_MAX_RELEASE_CANDIDATE_GATES
            || self.release_blocker_count != 0
            || self.product_qa_release_blockers != 0
            || self.known_limitation_count == 0
            || self.gpu_performance_status != "manual-unknown-unless-measured"
            || self.graphics_status != "manual-not-measured"
            || self.release_tag_created
            || !self.tag_proposal.contains("git tag -a playable-sim-rc1")
            || !self.report_path.ends_with("release_candidate.md")
            || !self.p36_gates_preserved
            || !self.no_p37_created
            || !self.no_generated_artifacts_tracked
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }

        let mut ids = BTreeSet::new();
        let mut areas = BTreeSet::new();
        for gate in &self.gates {
            gate.validate()?;
            if !ids.insert(gate.id.as_str()) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "G23 release candidate gate IDs must be unique",
                });
            }
            areas.insert(gate.area);
        }
        for required in [
            ReleaseCandidateArea::FullValidation,
            ReleaseCandidateArea::HeadlessPlayground,
            ReleaseCandidateArea::SaveLoad,
            ReleaseCandidateArea::Soak,
            ReleaseCandidateArea::Balance,
            ReleaseCandidateArea::ProductQa,
            ReleaseCandidateArea::Packaging,
            ReleaseCandidateArea::GpuManual,
            ReleaseCandidateArea::GraphicsManual,
            ReleaseCandidateArea::Docs,
        ] {
            if !areas.contains(&required) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "G23 release candidate is missing a required gate area",
                });
            }
        }
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.candidate_id,
            self.gates.len(),
            self.release_blocker_count,
            self.automated_gate_count,
            self.manual_gate_count,
            self.playable_supported_path
        )
    }
}

pub fn run_release_candidate_smoke() -> Result<ReleaseCandidateSummary, GameAppShellError> {
    let root = g23_workspace_root();
    let launch = AppShellLaunchConfig::from_p34_fixture_root(g23_p34_fixture_root());

    let app = run_headless_app_shell_smoke(&launch)?;
    let save_load = run_save_load_ux_smoke(&launch)?;
    let balance = run_longrun_balance_smoke()?;
    let product_qa = run_product_qa_hardening_smoke()?;
    let packaging = run_platform_package_smoke()?;
    let onboarding = run_onboarding_help_smoke()?;
    let gpu = run_gpu_product_hardening_smoke()?;

    let gates = release_candidate_gates(ReleaseCandidateEvidenceRefs {
        app: &app,
        save_load: &save_load,
        balance: &balance,
        product_qa: &product_qa,
        packaging: &packaging,
        onboarding: &onboarding,
        gpu: &gpu,
    });
    let release_blocker_count = gates.iter().filter(|gate| gate.release_blocker).count();
    let automated_gate_count = gates.iter().filter(|gate| !gate.manual).count();
    let manual_gate_count = gates.iter().filter(|gate| gate.manual).count();

    validate_release_candidate_report(&root, &gpu.manual_hardware_command)?;
    let summary = ReleaseCandidateSummary {
        schema: G23_RELEASE_CANDIDATE_SCHEMA,
        schema_version: G23_RELEASE_CANDIDATE_SCHEMA_VERSION,
        candidate_id: "playable-sim-rc1".to_string(),
        playable_supported_path: "headless-cpu-playground".to_string(),
        gates,
        release_blocker_count,
        automated_gate_count,
        manual_gate_count,
        product_qa_release_blockers: product_qa.release_blocker_count,
        known_limitation_count: product_qa.known_limitation_count,
        gpu_performance_status: "manual-unknown-unless-measured".to_string(),
        graphics_status: "manual-not-measured".to_string(),
        release_tag_created: false,
        tag_proposal:
            "git tag -a playable-sim-rc1 <validated-main-sha> -m \"A-Life playable sim RC1\""
                .to_string(),
        report_path: "docs/release_candidate.md".to_string(),
        p36_gates_preserved: release_gate_docs_preserved(&root)?,
        no_p37_created: no_p37_plan_exists(&root)?,
        no_generated_artifacts_tracked: !tracked_generated_artifacts_present(&root)?,
    };
    summary.validate()?;
    Ok(summary)
}

struct ReleaseCandidateEvidenceRefs<'a> {
    app: &'a AppStartupSummary,
    save_load: &'a SaveLoadUxSmokeSummary,
    balance: &'a LongRunBalanceSummary,
    product_qa: &'a ProductQaSummary,
    packaging: &'a PlatformPackageSummary,
    onboarding: &'a OnboardingHelpSummary,
    gpu: &'a GpuProductHardeningSummary,
}

fn release_candidate_gates(
    evidence: ReleaseCandidateEvidenceRefs<'_>,
) -> Vec<ReleaseCandidateGate> {
    vec![
        ReleaseCandidateGate {
            id: "g23-default-validation".to_string(),
            area: ReleaseCandidateArea::FullValidation,
            status: ReleaseCandidateGateStatus::ExternalValidation,
            command: "cargo fmt --all -- --check && cargo check --workspace --all-targets && cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings".to_string(),
            evidence: "run by the orchestrator before accepting G23; not embedded as a runtime claim".to_string(),
            manual: false,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "g23-windows-wrapper-validation".to_string(),
            area: ReleaseCandidateArea::FullValidation,
            status: ReleaseCandidateGateStatus::ExternalValidation,
            command: "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/check.ps1".to_string(),
            evidence: "Windows wrapper command is required; plain bash is not used on Windows".to_string(),
            manual: false,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "g23-headless-playground".to_string(),
            area: ReleaseCandidateArea::HeadlessPlayground,
            status: ReleaseCandidateGateStatus::Passed,
            command: "cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json".to_string(),
            evidence: format!(
                "default_path={} graphics_required={}",
                !evidence.app.graphics_required_for_default_path,
                evidence.app.graphics_required_for_default_path
            ),
            manual: false,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "g23-save-load-ux".to_string(),
            area: ReleaseCandidateArea::SaveLoad,
            status: ReleaseCandidateGateStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- save-load-ux-smoke crates/alife_world/tests/fixtures/p34".to_string(),
            evidence: format!(
                "loaded={} stable_ids={} no_partial_load={}",
                evidence.save_load.loaded_save_id,
                evidence.save_load.stable_world_ids.len(),
                evidence.save_load.no_partial_load_after_error
            ),
            manual: false,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "g23-fast-soak".to_string(),
            area: ReleaseCandidateArea::Soak,
            status: ReleaseCandidateGateStatus::ExternalValidation,
            command: "cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants".to_string(),
            evidence: "fast headless soak is part of the G23 validation evidence set".to_string(),
            manual: false,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "g23-balance-smoke".to_string(),
            area: ReleaseCandidateArea::Balance,
            status: ReleaseCandidateGateStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke".to_string(),
            evidence: format!(
                "sealed={} population_bound={} resource_bound={}",
                evidence.balance.metrics.sealed_patch_count,
                evidence.balance.metrics.population_bounds_enforced,
                evidence.balance.metrics.resource_bounds_enforced
            ),
            manual: false,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "g23-product-qa".to_string(),
            area: ReleaseCandidateArea::ProductQa,
            status: ReleaseCandidateGateStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke".to_string(),
            evidence: format!(
                "checklist={} blockers={} limitations={}",
                evidence.product_qa.checklist.len(),
                evidence.product_qa.release_blocker_count,
                evidence.product_qa.known_limitation_count
            ),
            manual: false,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "g23-platform-package".to_string(),
            area: ReleaseCandidateArea::Packaging,
            status: ReleaseCandidateGateStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- platform-package-smoke".to_string(),
            evidence: format!(
                "commands={} artifacts_tracked={}",
                evidence.packaging.commands.len(),
                evidence.packaging.generated_artifacts_tracked
            ),
            manual: false,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "g23-onboarding-docs".to_string(),
            area: ReleaseCandidateArea::Docs,
            status: ReleaseCandidateGateStatus::Passed,
            command: "cargo run -p alife_game_app --bin alife_game_app -- onboarding-help-smoke".to_string(),
            evidence: format!(
                "tutorial_steps={} wrappers={}",
                evidence.onboarding.tutorial_step_count,
                evidence.onboarding.windows_wrappers_documented
            ),
            manual: false,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "g23-gpu-hardware-manual".to_string(),
            area: ReleaseCandidateArea::GpuManual,
            status: ReleaseCandidateGateStatus::Manual,
            command: evidence.gpu.manual_hardware_command.clone(),
            evidence: "manual hardware flags and validation are required before claiming GPU performance".to_string(),
            manual: true,
            release_blocker: false,
        },
        ReleaseCandidateGate {
            id: "fvr08-production-voxel-playtest-manual".to_string(),
            area: ReleaseCandidateArea::GraphicsManual,
            status: ReleaseCandidateGateStatus::Manual,
            command: "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1".to_string(),
            evidence: "manual production voxel graphics support is required for final playtest evidence; dry-run is CI-safe only".to_string(),
            manual: true,
            release_blocker: false,
        },
    ]
}

fn validate_release_candidate_report(
    root: &Path,
    manual_gpu_command: &str,
) -> Result<(), GameAppShellError> {
    let report = std::fs::read_to_string(root.join("docs/release_candidate.md"))?;
    for required in [
        "G23 Playable Release Candidate",
        "cargo run -p alife_game_app --bin alife_game_app -- release-candidate-smoke",
        "cargo run -p alife_tools --bin p35_playground -- run-all crates/alife_world/tests/fixtures/p34 examples/p35/playground_manifest.json",
        "cargo run -p alife_game_app --bin alife_game_app -- save-load-ux-smoke crates/alife_world/tests/fixtures/p34",
        "cargo test -p alife_world --test headless_soak fast_headless_soak_preserves_release_gate_invariants",
        "cargo run -p alife_game_app --bin alife_game_app -- longrun-balance-smoke",
        "cargo run -p alife_game_app --bin alife_game_app -- product-qa-smoke",
        "No release tag was created",
        "CPU fallback is not GPU performance",
    ] {
        if !report.contains(required) {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "G23 release candidate report is missing a required command or limitation",
            });
        }
    }
    if !report.contains(manual_gpu_command)
        || report.contains("bash scripts/check.sh")
        || report.contains("gpu-report")
        || report.contains("ALIFE_GPU_BACKEND")
    {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "G23 release candidate report contains stale or unsafe command text",
        });
    }
    Ok(())
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

fn g23_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn g23_p34_fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../alife_world/tests/fixtures/p34")
}
