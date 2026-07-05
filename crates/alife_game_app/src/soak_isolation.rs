//! CA36 manual multi-hour soak isolation protocol.
//!
//! This module does not create a new simulation path. It packages existing
//! headless, GPU, and graphical soak commands into a bounded manual evidence
//! protocol and keeps generated reports under ignored `target/` paths.

use crate::prelude::*;
use crate::{
    GameAppShellError, CA36_DEFAULT_REPORT_PATH, CA36_MIN_MANUAL_TICKS, CA36_MIN_MULTI_HOUR_HOURS,
    CA36_SOAK_ISOLATION_SCHEMA, CA36_SOAK_ISOLATION_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoakIsolationCommand {
    pub name: String,
    pub command: String,
    pub manual: bool,
    pub min_ticks: Option<u32>,
    pub expected_artifact: Option<String>,
    pub evidence_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoakIsolationCounter {
    pub name: String,
    pub source: String,
    pub pass_condition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoakIsolationSummary {
    pub schema: &'static str,
    pub schema_version: u16,
    pub artifact_root: String,
    pub default_report_path: String,
    pub ci_smoke_command: String,
    pub manual_10k_commands: Vec<SoakIsolationCommand>,
    pub multi_hour_commands: Vec<SoakIsolationCommand>,
    pub graphical_commands: Vec<SoakIsolationCommand>,
    pub monitoring_instructions: Vec<String>,
    pub precision_drift_counters: Vec<SoakIsolationCounter>,
    pub report_artifacts_untracked: bool,
    pub cpu_fallback_preserved: bool,
    pub cpu_shadow_parity_preserved: bool,
    pub no_active_bulk_readback: bool,
    pub full_action_authoritative_claim: bool,
    pub release_tag_created: bool,
    pub report_markdown: String,
}

impl SoakIsolationSummary {
    pub fn validate(&self) -> Result<(), GameAppShellError> {
        if self.schema != CA36_SOAK_ISOLATION_SCHEMA
            || self.schema_version != CA36_SOAK_ISOLATION_SCHEMA_VERSION
            || !self.artifact_root.starts_with("target/")
            || !self.default_report_path.starts_with("target/")
            || !self.report_artifacts_untracked
            || !self.cpu_fallback_preserved
            || !self.cpu_shadow_parity_preserved
            || !self.no_active_bulk_readback
            || self.full_action_authoritative_claim
            || self.release_tag_created
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA36 soak isolation summary violated boundary flags",
            });
        }
        if self.manual_10k_commands.len() < 3
            || !self
                .manual_10k_commands
                .iter()
                .all(|command| command.manual && command.min_ticks >= Some(CA36_MIN_MANUAL_TICKS))
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA36 requires manual 10k+ headless/GPU soak commands",
            });
        }
        if self.multi_hour_commands.is_empty()
            || !self
                .multi_hour_commands
                .iter()
                .all(|command| command.manual && command.command.contains("AddHours"))
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA36 requires optional multi-hour manual commands",
            });
        }
        for required in [
            "Get-Process",
            "WorkingSet64",
            "PrivateMemorySize64",
            "cpu_shadow_parity_checks",
            "parity_failures",
            "h_shadow",
            "target/ca36_soak_isolation",
            "not full action-authoritative",
        ] {
            if !self.report_markdown.contains(required) {
                return Err(GameAppShellError::VisibleWorldMismatch {
                    message: "CA36 report missing required soak protocol text",
                });
            }
        }
        if self
            .precision_drift_counters
            .iter()
            .any(|counter| counter.name.is_empty() || counter.pass_condition.is_empty())
        {
            return Err(GameAppShellError::VisibleWorldMismatch {
                message: "CA36 precision/drift counters must be named and bounded",
            });
        }
        Ok(())
    }
}

pub fn run_multi_hour_soak_isolation_smoke() -> Result<SoakIsolationSummary, GameAppShellError> {
    let artifact_root = "target/ca36_soak_isolation".to_string();
    let default_report_path = CA36_DEFAULT_REPORT_PATH.to_string();
    let ci_smoke_command =
        "cargo run -p alife_game_app --bin alife_game_app -- multi-hour-soak-isolation-smoke"
            .to_string();
    let manual_10k_commands = vec![
        SoakIsolationCommand {
            name: "gpu_sustained_learning_10k".to_string(),
            command: "cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 10000 --report-every 1000 --json target/ca36_soak_isolation/gpu_sustained_learning_10k.json".to_string(),
            manual: true,
            min_ticks: Some(CA36_MIN_MANUAL_TICKS),
            expected_artifact: Some("target/ca36_soak_isolation/gpu_sustained_learning_10k.json".to_string()),
            evidence_boundary: "Manual local GPU evidence; CPU shadow parity remains the gate.".to_string(),
        },
        SoakIsolationCommand {
            name: "gpu_longrun_10k".to_string(),
            command: "cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-longrun-soak crates/alife_world/tests/fixtures/p34 --ticks 10000 --report-every 1000 --json target/ca36_soak_isolation/gpu_longrun_10k.json".to_string(),
            manual: true,
            min_ticks: Some(CA36_MIN_MANUAL_TICKS),
            expected_artifact: Some("target/ca36_soak_isolation/gpu_longrun_10k.json".to_string()),
            evidence_boundary: "Manual local GPU stability evidence; not release or cross-machine performance.".to_string(),
        },
        SoakIsolationCommand {
            name: "headless_ecology_10k".to_string(),
            command: "cargo test -p alife_game_app --test app_shell ca22_manual_10k_ecological_soak -- --ignored --nocapture".to_string(),
            manual: true,
            min_ticks: Some(CA36_MIN_MANUAL_TICKS),
            expected_artifact: None,
            evidence_boundary: "Manual headless ecology soak; no GPU performance claim.".to_string(),
        },
    ];
    let multi_hour_commands = vec![SoakIsolationCommand {
        name: "two_hour_repeated_gpu_sustained_learning".to_string(),
        command: "$end=(Get-Date).AddHours(2); $i=0; New-Item -ItemType Directory -Force target/ca36_soak_isolation | Out-Null; while((Get-Date) -lt $end){ $i++; cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-sustained-learning-soak crates/alife_world/tests/fixtures/p34 --ticks 10000 --report-every 1000 --json \"target/ca36_soak_isolation/gpu_sustained_learning_10k_$i.json\"; if($LASTEXITCODE -ne 0){ exit $LASTEXITCODE }; Get-Process -Id $PID | Select-Object Id,ProcessName,CPU,WorkingSet64,PrivateMemorySize64 | Out-File \"target/ca36_soak_isolation/process_$i.txt\" }".to_string(),
        manual: true,
        min_ticks: Some(CA36_MIN_MANUAL_TICKS),
        expected_artifact: Some("target/ca36_soak_isolation/gpu_sustained_learning_10k_<run>.json".to_string()),
        evidence_boundary: "Optional multi-hour local isolation loop; reports stay under target and must not be committed.".to_string(),
    }];
    let graphical_commands = vec![
        SoakIsolationCommand {
            name: "production_voxel_gpu_30s_smoke".to_string(),
            command: "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -SmokeSeconds 30 -GpuMode auto-with-cpu-fallback -RecordPerformance".to_string(),
            manual: false,
            min_ticks: None,
            expected_artifact: None,
            evidence_boundary: "Bounded graphical smoke; dry-run is not graphical evidence.".to_string(),
        },
        SoakIsolationCommand {
            name: "production_voxel_forced_cpu_fallback_10s_smoke".to_string(),
            command: "$env:ALIFE_GPU_RUNTIME_AVAILABLE=\"0\"; powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -SmokeSeconds 10 -GpuMode auto-with-cpu-fallback -RecordPerformance; Remove-Item Env:\\ALIFE_GPU_RUNTIME_AVAILABLE -ErrorAction SilentlyContinue".to_string(),
            manual: false,
            min_ticks: None,
            expected_artifact: None,
            evidence_boundary: "Fallback smoke must show degraded CPU mode and no GPU performance claim.".to_string(),
        },
    ];
    let monitoring_instructions = vec![
        "Sample process memory with Get-Process -Id <cargo-or-app-pid> | Select-Object Id,ProcessName,CPU,WorkingSet64,PrivateMemorySize64 every 10k run.".to_string(),
        "Keep command stdout/stderr and JSON outputs under target/ca36_soak_isolation/; verify `git ls-files target/ca36_soak_isolation` is empty before commit.".to_string(),
        "Record first failure tick, fallback reason, adapter/backend, wall time, ticks/sec, and whether the run was graphical, headless, or GPU manual.".to_string(),
    ];
    let precision_drift_counters = vec![
        SoakIsolationCounter {
            name: "cpu_shadow_parity_checks".to_string(),
            source: "gpu-longrun-soak and gpu-sustained-learning-soak summaries".to_string(),
            pass_condition: "checks equal completed GPU proposal ticks and parity_failures remains 0".to_string(),
        },
        SoakIsolationCounter {
            name: "first_parity_failure_tick".to_string(),
            source: "GPU soak summaries".to_string(),
            pass_condition: "None for PASS; exact tick recorded before fallback on failure".to_string(),
        },
        SoakIsolationCounter {
            name: "h_shadow_delta_max".to_string(),
            source: "post-seal H_shadow receipt".to_string(),
            pass_condition: "finite and bounded; W_genetic_fixed, lifetime-consolidated, and H_operational unchanged".to_string(),
        },
        SoakIsolationCounter {
            name: "compact_readback_bytes".to_string(),
            source: "GPU compact action summary".to_string(),
            pass_condition: "bounded compact readback only; no active bulk neural readback".to_string(),
        },
        SoakIsolationCounter {
            name: "sealed_patches_vs_packed_logs".to_string(),
            source: "live tick and packed logging summaries".to_string(),
            pass_condition: "monotonic and no hidden saturation in sustained-learning episode rotation".to_string(),
        },
        SoakIsolationCounter {
            name: "working_set_private_memory".to_string(),
            source: "Get-Process WorkingSet64 and PrivateMemorySize64 samples".to_string(),
            pass_condition: "no unbounded monotonic growth across repeated 10k runs without an explicit finding".to_string(),
        },
    ];
    let report_markdown = ca36_soak_isolation_report_markdown(
        &artifact_root,
        &ci_smoke_command,
        &manual_10k_commands,
        &multi_hour_commands,
        &graphical_commands,
        &monitoring_instructions,
        &precision_drift_counters,
    );
    let summary = SoakIsolationSummary {
        schema: CA36_SOAK_ISOLATION_SCHEMA,
        schema_version: CA36_SOAK_ISOLATION_SCHEMA_VERSION,
        artifact_root,
        default_report_path,
        ci_smoke_command,
        manual_10k_commands,
        multi_hour_commands,
        graphical_commands,
        monitoring_instructions,
        precision_drift_counters,
        report_artifacts_untracked: true,
        cpu_fallback_preserved: true,
        cpu_shadow_parity_preserved: true,
        no_active_bulk_readback: true,
        full_action_authoritative_claim: false,
        release_tag_created: false,
        report_markdown,
    };
    summary.validate()?;
    Ok(summary)
}

pub fn write_ca36_soak_isolation_report(
    summary: &SoakIsolationSummary,
    output_path: impl AsRef<Path>,
) -> Result<(), GameAppShellError> {
    let output_path = output_path.as_ref();
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, &summary.report_markdown)?;
    Ok(())
}

fn ca36_soak_isolation_report_markdown(
    artifact_root: &str,
    ci_smoke_command: &str,
    manual_10k_commands: &[SoakIsolationCommand],
    multi_hour_commands: &[SoakIsolationCommand],
    graphical_commands: &[SoakIsolationCommand],
    monitoring_instructions: &[String],
    precision_drift_counters: &[SoakIsolationCounter],
) -> String {
    let command_list = |commands: &[SoakIsolationCommand]| {
        commands
            .iter()
            .map(|command| {
                format!(
                    "- `{}`\n  - artifact: `{}`\n  - boundary: {}\n",
                    command.command,
                    command.expected_artifact.as_deref().unwrap_or("none"),
                    command.evidence_boundary
                )
            })
            .collect::<String>()
    };
    let counter_list = precision_drift_counters
        .iter()
        .map(|counter| {
            format!(
                "- `{}` from {}: {}\n",
                counter.name, counter.source, counter.pass_condition
            )
        })
        .collect::<String>();
    let monitoring_list = monitoring_instructions
        .iter()
        .map(|line| format!("- {line}\n"))
        .collect::<String>();
    format!(
        "# CA36 Multi-Hour Soak Isolation Protocol\n\n\
Status: CI-safe protocol smoke only. Manual long-run reports belong under `{artifact_root}` and must stay untracked.\n\n\
## CI Smoke\n\n`{ci_smoke_command}`\n\n\
## Manual 10k+ Commands\n\n{}\
## Optional Multi-Hour Commands\n\nMinimum optional duration: `{}` hours.\n\n{}\
## Graphical/Fallback Checks\n\n{}\
## Memory And Process Monitoring\n\n{}\
## Precision/Drift Counters\n\n{}\
## Boundaries\n\n\
- CPU fallback remains available and visibly degraded where applicable.\n\
- CPU shadow parity remains the gate; `cpu_shadow_parity_checks` and `parity_failures` must be recorded.\n\
- H_shadow deltas apply only through the post-seal core contract; `h_shadow_delta_max` must be finite.\n\
- This is not full action-authoritative GPU runtime evidence.\n\
- No active bulk neural readback is allowed.\n\
- No release tag, S12, G25, or P37 is created.\n\
- Verify report artifacts with `git ls-files {artifact_root}` before commit.\n",
        command_list(manual_10k_commands),
        CA36_MIN_MULTI_HOUR_HOURS,
        command_list(multi_hour_commands),
        command_list(graphical_commands),
        monitoring_list,
        counter_list
    )
}
