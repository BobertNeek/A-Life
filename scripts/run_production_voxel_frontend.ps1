param(
    [switch]$DryRun,
    [ValidateRange(0, 120)]
    [int]$SmokeSeconds = 0,
    [ValidateSet("MinimumSettings30x30", "MinSpecComfort1080p", "Balanced1080p", "HighSpecScaleUp", "ResearchScale")]
    [string]$Profile = "MinSpecComfort1080p",
    [ValidateRange(0, 500)]
    [int]$Population = 0,
    [string]$Resolution = "1920x1080",
    [ValidateSet("cpu-reference", "static-plastic-cpu-shadow-guarded", "auto-with-cpu-fallback")]
    [string]$GpuMode = "auto-with-cpu-fallback",
    [ValidateSet("auto", "dx12", "vulkan", "existing")]
    [string]$GraphicsBackend = "auto",
    [switch]$RequireGpu,
    [switch]$RecordPerformance
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$FeatureList = "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi"
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_production_voxel_frontend.ps1 -DryRun

function Format-CommandArgument {
    param([string]$Value)

    if ($Value -match "[\s'`"]") {
        return "'" + ($Value -replace "'", "''") + "'"
    }

    return $Value
}

$AppArgs = @(
    "production-voxel",
    "--profile", $Profile,
    "--resolution", $Resolution,
    "--gpu-mode", $GpuMode,
    "--graphics-backend", $GraphicsBackend
)

if ($Population -gt 0) {
    $AppArgs += @("--population", "$Population")
}

if ($SmokeSeconds -gt 0) {
    $AppArgs += @("--smoke-seconds", "$SmokeSeconds")
}

if ($DryRun) {
    $AppArgs += "--dry-run"
}

if ($RequireGpu) {
    $AppArgs += "--require-gpu"
}

if ($RecordPerformance) {
    $AppArgs += "--record-performance"
}

$CargoArgs = @(
    "run",
    "-p", "alife_game_app",
    "--features", $FeatureList,
    "--bin", "alife_game_app",
    "--"
) + $AppArgs

$CommandPreview = "cargo " + (($CargoArgs | ForEach-Object { Format-CommandArgument $_ }) -join " ")

Write-Host "A-Life Voxel Frontend"
Write-Host "Profile: $Profile"
Write-Host "Minimum fallback profile: MinimumSettings30x30"
Write-Host "Features: $FeatureList"
Write-Host "Save directory policy: fixture saves stay under the selected environment; UI/profile settings are written under target/artifacts/fvr05 unless --ui-settings overrides them."
Write-Host "GPU fallback diagnostics: $GpuMode with explicit production-voxel preflight output."
Write-Host "Command: $CommandPreview"

if ($DryRun) {
    Write-Host "Dry run only; production-voxel command was not executed."
    exit 0
}

Push-Location $Root
try {
    & cargo @CargoArgs
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
