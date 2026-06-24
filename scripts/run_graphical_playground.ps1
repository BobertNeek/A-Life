param(
    [switch]$DryRun,
    [ValidateRange(0, 120)]
    [int]$SmokeSeconds = 0,
    [ValidateSet("cpu-reference", "static-plastic-cpu-shadow-guarded", "auto-with-cpu-fallback")]
    [string]$GpuMode = "static-plastic-cpu-shadow-guarded"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)

if ($SmokeSeconds -gt 0) {
    $ModeArgs = @("graphical-playground")
    $ModeArgs += "crates/alife_world/tests/fixtures/p34"
    $ModeArgs += @("--gpu-mode", $GpuMode, "--smoke-seconds", "$SmokeSeconds")
    $ModeLabel = "bounded graphical playground smoke"
} else {
    $ModeArgs = @("graphical-playground")
    $ModeArgs += "crates/alife_world/tests/fixtures/p34"
    $ModeArgs += @("--gpu-mode", $GpuMode)
    $ModeLabel = "persistent graphical playground"
}

$FeatureList = if ($GpuMode -eq "cpu-reference") { "bevy-app" } else { "bevy-app gpu-runtime" }

function Format-CommandArgument {
    param([string]$Value)

    if ($Value -match "[\s'`"]") {
        return "'" + ($Value -replace "'", "''") + "'"
    }

    return $Value
}

$Command = @(
    "cargo",
    "run",
    "-p",
    "alife_game_app",
    "--features",
    $FeatureList,
    "--bin",
    "alife_game_app",
    "--"
)
$Command += $ModeArgs

Write-Host "A-Life $ModeLabel command:"
$DisplayCommand = ($Command | ForEach-Object { Format-CommandArgument $_ }) -join " "
Write-Host $DisplayCommand
Write-Host "Alpha tester command: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded"
Write-Host "Manual graphics path: requires local windowing/graphics support. Requested GPU mode: $GpuMode."
Write-Host "GPU mode remains optional; CPU fallback is visible when hardware, feature, or validation gates are unavailable."
Write-Host "Title: A-Life Alpha Playground."
Write-Host "Controls: Space pause/run, N step once, 1/2/3 speed, F follow, Esc quit."
Write-Host "Camera/inspector: arrows/WASD pan, +/- zoom, Q/E orbit, F follow selected stable ID. Inspector is read-only."
Write-Host "Readability: color+shape markers, stable-ID badges, concise GPU/fallback status, read-only inspector."
Write-Host "Reset/restart: close and relaunch the P34 fixture if the current alpha run becomes confusing."

if ($DryRun) {
    exit 0
}

Push-Location $Root
try {
    $Args = $Command[1..($Command.Length - 1)]
    & $Command[0] @Args
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
