//! CA43 crash log and tester feedback capture policy.
//!
//! This is app/tooling policy only. It describes where local tester evidence is
//! written, sanitizes local paths from crash summaries, and renders a bounded
//! feedback template without changing simulation semantics.

use std::{
    env,
    path::{Component, Path, PathBuf},
};

use crate::prelude::*;
use crate::*;

pub const CA43_REPO_FEEDBACK_DIR: &str = "target/artifacts/ca43_tester_feedback";
pub const CA43_PACKAGE_FEEDBACK_DIR: &str = "diagnostics/ca43_tester_feedback";
pub const CA43_CRASH_SUMMARY_FILE: &str = "crash_summary.md";
pub const CA43_FEEDBACK_TEMPLATE_FILE: &str = "tester_feedback_template.md";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ca43LogDirectoryPolicy {
    pub repo_feedback_dir: PathBuf,
    pub package_feedback_dir: PathBuf,
    pub crash_summary_file: &'static str,
    pub feedback_template_file: &'static str,
    pub artifacts_must_remain_untracked: bool,
    pub screenshots_media_logs_untracked: bool,
    pub sanitize_paths_required: bool,
}

impl Default for Ca43LogDirectoryPolicy {
    fn default() -> Self {
        Self {
            repo_feedback_dir: PathBuf::from(CA43_REPO_FEEDBACK_DIR),
            package_feedback_dir: PathBuf::from(CA43_PACKAGE_FEEDBACK_DIR),
            crash_summary_file: CA43_CRASH_SUMMARY_FILE,
            feedback_template_file: CA43_FEEDBACK_TEMPLATE_FILE,
            artifacts_must_remain_untracked: true,
            screenshots_media_logs_untracked: true,
            sanitize_paths_required: true,
        }
    }
}

impl Ca43LogDirectoryPolicy {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if !self.artifacts_must_remain_untracked
            || !self.screenshots_media_logs_untracked
            || !self.sanitize_paths_required
            || self.crash_summary_file != CA43_CRASH_SUMMARY_FILE
            || self.feedback_template_file != CA43_FEEDBACK_TEMPLATE_FILE
            || !is_safe_relative_path(&self.repo_feedback_dir)
            || !is_safe_relative_path(&self.package_feedback_dir)
            || !self
                .repo_feedback_dir
                .starts_with(Path::new("target").join("artifacts"))
            || self.package_feedback_dir != Path::new(CA43_PACKAGE_FEEDBACK_DIR)
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashSummaryInput {
    pub stage: String,
    pub command: String,
    pub exit_code: i32,
    pub log_path: PathBuf,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

impl CrashSummaryInput {
    pub fn sample_for_workspace(workspace_root: &Path) -> Self {
        let command = format!(
            "powershell -NoProfile -ExecutionPolicy Bypass -File {} -SmokeSeconds 30",
            workspace_root
                .join("scripts/run_graphical_playground.ps1")
                .display()
        );
        Self {
            stage: "graphical-playground".to_string(),
            command,
            exit_code: 101,
            log_path: workspace_root
                .join(CA43_REPO_FEEDBACK_DIR)
                .join(CA43_CRASH_SUMMARY_FILE),
            stdout_tail: "Starting A-Life GPU Alpha Playground".to_string(),
            stderr_tail: format!(
                "error: window failed near {}",
                workspace_root
                    .join("target/artifacts/ca42_runtime_prereq/runtime_prereq.log")
                    .display()
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub stage: String,
    pub exit_code: i32,
    pub sanitized_command: String,
    pub sanitized_log_path: String,
    pub sanitized_stdout_tail: String,
    pub sanitized_stderr_tail: String,
    pub user_action_required: bool,
    pub commit_media_forbidden: bool,
}

impl CrashSummary {
    pub fn validate(&self, workspace_root: &Path) -> Result<(), GameAppShellError> {
        if self.schema != CA43_TESTER_FEEDBACK_SCHEMA
            || self.schema_version != CA43_TESTER_FEEDBACK_SCHEMA_VERSION
            || self.stage.trim().is_empty()
            || self.sanitized_command.trim().is_empty()
            || self.sanitized_log_path.trim().is_empty()
            || self.sanitized_command.len() > CA43_MAX_SANITIZED_TEXT_BYTES
            || self.sanitized_stdout_tail.len() > CA43_MAX_SANITIZED_TEXT_BYTES
            || self.sanitized_stderr_tail.len() > CA43_MAX_SANITIZED_TEXT_BYTES
            || !self.user_action_required
            || !self.commit_media_forbidden
            || contains_unsanitized_local_path(&self.sanitized_command, workspace_root)
            || contains_unsanitized_local_path(&self.sanitized_log_path, workspace_root)
            || contains_unsanitized_local_path(&self.sanitized_stdout_tail, workspace_root)
            || contains_unsanitized_local_path(&self.sanitized_stderr_tail, workspace_root)
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        Ok(())
    }

    pub fn to_markdown(&self) -> String {
        format!(
            "# A-Life CA43 Crash Summary\n\n\
Schema: `{}` v{}\n\
Stage: `{}`\n\
Exit code: `{}`\n\
User action required: `{}`\n\
Commit media/log artifacts: `false`\n\n\
## Command\n\n```text\n{}\n```\n\n\
## Log Path\n\n```text\n{}\n```\n\n\
## Stdout Tail\n\n```text\n{}\n```\n\n\
## Stderr Tail\n\n```text\n{}\n```\n",
            self.schema,
            self.schema_version,
            self.stage,
            self.exit_code,
            self.user_action_required,
            self.sanitized_command,
            self.sanitized_log_path,
            self.sanitized_stdout_tail,
            self.sanitized_stderr_tail
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TesterFeedbackTemplate {
    pub schema: &'static str,
    pub schema_version: u16,
    pub required_fields: Vec<&'static str>,
    pub severity_labels: Vec<&'static str>,
    pub media_external_only: bool,
    pub no_release_claim: bool,
    pub no_full_action_authoritative_claim: bool,
}

impl TesterFeedbackTemplate {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        for required in [
            "tester alias",
            "machine",
            "exact command",
            "window opened",
            "gpu or fallback visible",
            "crash summary path",
            "external media references",
            "severity",
        ] {
            if !self.required_fields.contains(&required) {
                return Err(ScaffoldContractError::MissingPhaseData.into());
            }
        }
        for severity in [
            "BLOCKER",
            "HIGH",
            "MEDIUM",
            "LOW",
            "MANUAL_EVIDENCE_MISSING",
        ] {
            if !self.severity_labels.contains(&severity) {
                return Err(ScaffoldContractError::MissingPhaseData.into());
            }
        }
        if self.schema != CA43_TESTER_FEEDBACK_SCHEMA
            || self.schema_version != CA43_TESTER_FEEDBACK_SCHEMA_VERSION
            || !self.media_external_only
            || !self.no_release_claim
            || !self.no_full_action_authoritative_claim
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        Ok(())
    }

    pub fn to_markdown(&self) -> String {
        format!(
            "# A-Life Alpha Tester Feedback Template\n\n\
Schema: `{}` v{}\n\n\
Do not attach or commit screenshots, videos, logs, captures, target artifacts,\n\
model files, or caches to git. Record external media paths or links only.\n\n\
## Tester\n\n\
- Tester alias:\n\
- Date/time:\n\
- OS:\n\
- CPU:\n\
- GPU / driver:\n\
- RAM:\n\
- Display resolution:\n\
- Repo/package SHA:\n\n\
## Launch\n\n\
- Exact command:\n\
- Window opened: yes/no\n\
- GPU status visible: authoritative / unavailable\n\
- Crash summary path, if any:\n\
- App exited cleanly without leaving a process: yes/no\n\n\
## Playability\n\n\
- Creature visible:\n\
- Food visible:\n\
- Hazard visible:\n\
- Pause/run worked:\n\
- Step worked:\n\
- Follow/reset worked:\n\
- Inspector readable:\n\
- Most confusing text or visual:\n\n\
## Evidence References\n\n\
- Screenshot/video path or external link, if manually captured:\n\
- Local log/crash-summary path, if generated:\n\n\
## Findings\n\n\
Use one of: `{}`.\n\n\
| Severity | Finding | Repro command | Notes |\n\
| --- | --- | --- | --- |\n\
|  |  |  |  |\n\n\
Release/tag recommendation: defer unless explicitly approved.\n\
Full action-authoritative GPU runtime claim: false.\n",
            self.schema,
            self.schema_version,
            self.severity_labels.join("`, `")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TesterFeedbackCaptureSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub policy: Ca43LogDirectoryPolicy,
    pub crash_summary: CrashSummary,
    pub feedback_template: TesterFeedbackTemplate,
    pub tracked_artifacts_present: bool,
    pub launcher_script_wired: bool,
    pub package_script_wired: bool,
    pub docs_template_present: bool,
    pub no_release_tag_claim: bool,
    pub no_core_dependency_change_required: bool,
}

impl TesterFeedbackCaptureSummary {
    pub fn validate(&self, workspace_root: &Path) -> Result<(), GameAppShellError> {
        if self.schema != CA43_TESTER_FEEDBACK_SCHEMA
            || self.schema_version != CA43_TESTER_FEEDBACK_SCHEMA_VERSION
            || self.tracked_artifacts_present
            || !self.launcher_script_wired
            || !self.package_script_wired
            || !self.docs_template_present
            || !self.no_release_tag_claim
            || !self.no_core_dependency_change_required
        {
            return Err(ScaffoldContractError::MissingPhaseData.into());
        }
        self.policy.validate()?;
        self.crash_summary.validate(workspace_root)?;
        self.feedback_template.validate()?;
        Ok(())
    }

    pub fn signature_line(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}",
            self.schema_version,
            self.policy.repo_feedback_dir.display(),
            self.policy.package_feedback_dir.display(),
            self.launcher_script_wired,
            self.package_script_wired,
            self.tracked_artifacts_present
        )
    }
}

pub fn ca43_workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn ca43_feedback_template() -> TesterFeedbackTemplate {
    TesterFeedbackTemplate {
        schema: CA43_TESTER_FEEDBACK_SCHEMA,
        schema_version: CA43_TESTER_FEEDBACK_SCHEMA_VERSION,
        required_fields: vec![
            "tester alias",
            "machine",
            "exact command",
            "window opened",
            "gpu or fallback visible",
            "crash summary path",
            "external media references",
            "severity",
        ],
        severity_labels: vec![
            "BLOCKER",
            "HIGH",
            "MEDIUM",
            "LOW",
            "MANUAL_EVIDENCE_MISSING",
        ],
        media_external_only: true,
        no_release_claim: true,
        no_full_action_authoritative_claim: true,
    }
}

pub fn render_ca43_crash_summary(input: &CrashSummaryInput, workspace_root: &Path) -> CrashSummary {
    CrashSummary {
        schema: CA43_TESTER_FEEDBACK_SCHEMA,
        schema_version: CA43_TESTER_FEEDBACK_SCHEMA_VERSION,
        stage: sanitize_tester_text(&input.stage, workspace_root),
        exit_code: input.exit_code,
        sanitized_command: sanitize_tester_text(&input.command, workspace_root),
        sanitized_log_path: sanitize_tester_text(
            &input.log_path.display().to_string(),
            workspace_root,
        ),
        sanitized_stdout_tail: sanitize_tester_text(
            &truncate_for_summary(&input.stdout_tail),
            workspace_root,
        ),
        sanitized_stderr_tail: sanitize_tester_text(
            &truncate_for_summary(&input.stderr_tail),
            workspace_root,
        ),
        user_action_required: true,
        commit_media_forbidden: true,
    }
}

pub fn run_tester_feedback_capture_smoke() -> Result<TesterFeedbackCaptureSummary, GameAppShellError>
{
    let root = ca43_workspace_root();
    let policy = Ca43LogDirectoryPolicy::default();
    let crash_summary =
        render_ca43_crash_summary(&CrashSummaryInput::sample_for_workspace(&root), &root);
    let feedback_template = ca43_feedback_template();
    let launcher_script =
        std::fs::read_to_string(root.join("scripts/run_graphical_playground.ps1"))?;
    let package_script =
        std::fs::read_to_string(root.join("scripts/run_windows_alpha_package.ps1"))?;
    let docs_template = std::fs::read_to_string(
        root.join("docs/creatures_agi_roadmap_pack/templates/CA43_TESTER_FEEDBACK_TEMPLATE.md"),
    )?;
    let summary = TesterFeedbackCaptureSummary {
        schema: CA43_TESTER_FEEDBACK_SCHEMA,
        schema_version: CA43_TESTER_FEEDBACK_SCHEMA_VERSION,
        policy,
        crash_summary,
        feedback_template,
        tracked_artifacts_present: tracked_ca43_artifacts_present(&root)?,
        launcher_script_wired: script_has_ca43_feedback_wiring(&launcher_script),
        package_script_wired: script_has_ca43_feedback_wiring(&package_script),
        docs_template_present: docs_template.contains("A-Life Alpha Tester Feedback Template")
            && docs_template.contains("Full action-authoritative GPU runtime claim: false")
            && docs_template.contains("Do not attach or commit screenshots"),
        no_release_tag_claim: !launcher_script.contains("git tag")
            && !package_script.contains("git tag")
            && docs_template.contains("Release/tag recommendation: defer"),
        no_core_dependency_change_required: true,
    };
    summary.validate(&root)?;
    Ok(summary)
}

pub fn sanitize_tester_text(text: &str, workspace_root: &Path) -> String {
    let mut sanitized = text.replace('\0', "");
    for needle in local_path_needles(workspace_root) {
        if !needle.is_empty() {
            sanitized = sanitized.replace(&needle, "<local-path>");
        }
    }
    truncate_for_summary(&sanitized)
}

fn local_path_needles(workspace_root: &Path) -> Vec<String> {
    let mut needles = vec![
        workspace_root.display().to_string(),
        workspace_root.to_string_lossy().replace('\\', "/"),
    ];
    if let Ok(home) = env::var("USERPROFILE").or_else(|_| env::var("HOME")) {
        if !home.trim().is_empty() {
            needles.push(home.clone());
            needles.push(home.replace('\\', "/"));
        }
    }
    needles.sort();
    needles.dedup();
    needles
}

fn contains_unsanitized_local_path(text: &str, workspace_root: &Path) -> bool {
    local_path_needles(workspace_root)
        .into_iter()
        .any(|needle| !needle.is_empty() && text.contains(&needle))
}

fn truncate_for_summary(text: &str) -> String {
    if text.len() <= CA43_MAX_SANITIZED_TEXT_BYTES {
        return text.to_string();
    }
    let mut end = CA43_MAX_SANITIZED_TEXT_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...[truncated]", &text[..end])
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path.components().all(|component| {
            !matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn script_has_ca43_feedback_wiring(script: &str) -> bool {
    script.contains("ca43_tester_feedback")
        && script.contains("crash_summary.md")
        && script.contains("tester_feedback_template.md")
        && script.contains("Write-Ca43CrashSummary")
        && script.contains("Convert-ToCa43SafeText")
        && !script.contains("git tag")
}

fn tracked_ca43_artifacts_present(root: &Path) -> Result<bool, GameAppShellError> {
    let output = std::process::Command::new("git")
        .args([
            "ls-files",
            "target",
            "target/artifacts",
            "target/playtest_evidence",
            "models",
            ".cache",
            "graphify-out",
        ])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        return Err(GameAppShellError::VisibleWorldMismatch {
            message: "git ls-files failed while checking CA43 artifact policy",
        });
    }
    Ok(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}
