param(
    [switch]$DryRun,
    [ValidateRange(0, 120)]
    [int]$SmokeSeconds = 0,
    [ValidateSet("gpu-required", "heuristic-baseline")]
    [string]$BrainPolicy = "gpu-required",
    [ValidateSet("player", "dev-overlay", "full-debug")]
    [string]$ViewMode = "player",
    [ValidateSet("gpu-alpha", "p34", "production-voxel")]
    [string]$Scenario = "production-voxel",
    [string]$EnvironmentManifest = "",
    [ValidateSet("auto", "dx12", "vulkan", "existing")]
    [string]$GraphicsBackend = "auto",
    [switch]$RequireGpu,
    [ValidateSet("MinimumSettings30x30", "MinSpecComfort1080p", "Balanced1080p", "HighSpecScaleUp", "ResearchScale")]
    [string]$Profile = "MinSpecComfort1080p"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$ProductionLauncher = Join-Path $Root "scripts/run_production_voxel_frontend.ps1"
$PreflightLog = Join-Path $Root "target/artifacts/ca42_runtime_prereq/runtime_prereq.log"
$FeedbackRoot = Join-Path $Root "target/artifacts/ca43_tester_feedback"
$CrashSummaryPath = Join-Path $FeedbackRoot "crash_summary.md"
$FeedbackTemplatePath = Join-Path $FeedbackRoot "tester_feedback_template.md"
$PreflightExitCode = 0

function Convert-ToCa43SafeText {
    param([AllowNull()][string]$Text)

    if ($null -eq $Text) {
        return ""
    }

    return $Text.Replace($Root, "<local-path>")
}

function Write-Ca43CrashSummary {
    param(
        [string]$Stage,
        [int]$ExitCode,
        [string[]]$CommandParts,
        [string]$LogPath
    )

    New-Item -ItemType Directory -Force -Path $FeedbackRoot | Out-Null
    $SafeCommand = Convert-ToCa43SafeText (($CommandParts | ForEach-Object { $_ }) -join " ")
    $SafeLog = Convert-ToCa43SafeText $LogPath
    @"
# A-Life Compatibility Launcher Crash Summary

Stage: $Stage
Exit code: $ExitCode
Command: $SafeCommand
Runtime preflight log: $SafeLog
Commit media/log artifacts: false
User action required: true
"@ | Set-Content -Encoding UTF8 -Path $CrashSummaryPath
}

# Runtime preflight log: $PreflightLog
# CA42 compatibility preflight command:
# cargo run -p alife_game_app --bin alife_game_app -- runtime-prereq-smoke --graphics-backend $GraphicsBackend --log $PreflightLog
# CA43 feedback files: $FeedbackRoot, $CrashSummaryPath, $FeedbackTemplatePath

Write-Host "FVR01 compatibility alias: scripts/run_graphical_playground.ps1 now routes to scripts/run_production_voxel_frontend.ps1."
Write-Host "Requested legacy view mode '$ViewMode' and scenario '$Scenario' are compatibility inputs; production launch uses profile '$Profile'."

$ForwardArgs = @(
    "-Profile", $Profile,
    "-BrainPolicy", $BrainPolicy,
    "-GraphicsBackend", $GraphicsBackend
)

if ($SmokeSeconds -gt 0) {
    $ForwardArgs += @("-SmokeSeconds", "$SmokeSeconds")
}

if ($DryRun) {
    $ForwardArgs += "-DryRun"
}

if ($RequireGpu) {
    $ForwardArgs += "-RequireGpu"
}

& powershell -NoProfile -ExecutionPolicy Bypass -File $ProductionLauncher @ForwardArgs
exit $LASTEXITCODE
