param(
    [switch]$DryRun,
    [ValidateRange(0, 120)]
    [int]$SmokeSeconds = 0,
    [ValidateSet("cpu-reference", "static-plastic-cpu-shadow-guarded", "auto-with-cpu-fallback")]
    [string]$GpuMode = "static-plastic-cpu-shadow-guarded",
    [switch]$RequireGpu
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)

if ($SmokeSeconds -gt 0) {
    $ModeArgs = @("graphical-playground")
    $ModeArgs += "crates/alife_world/tests/fixtures/gpu_alpha"
    $ModeArgs += @("--gpu-mode", $GpuMode, "--smoke-seconds", "$SmokeSeconds")
    $ModeLabel = "bounded graphical playground smoke"
} else {
    $ModeArgs = @("graphical-playground")
    $ModeArgs += "crates/alife_world/tests/fixtures/gpu_alpha"
    $ModeArgs += @("--gpu-mode", $GpuMode)
    $ModeLabel = "persistent graphical playground"
}

if ($RequireGpu) {
    $ModeArgs += "--require-gpu"
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

Write-Host "Starting A-Life GPU Alpha Playground"
Write-Host "A-Life $ModeLabel command:"
$DisplayCommand = ($Command | ForEach-Object { Format-CommandArgument $_ }) -join " "
Write-Host $DisplayCommand
Write-Host "Alpha tester command: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_graphical_playground.ps1 -SmokeSeconds 30 -GpuMode static-plastic-cpu-shadow-guarded"
Write-Host "GPU mode requested: $GpuMode"
Write-Host "CPU fallback is safety fallback, not the target alpha path."
if ($RequireGpu) {
    Write-Host "RequireGpu: enabled. A CPU fallback exits as a clear GPU-unavailable failure."
} else {
    Write-Host "RequireGpu: disabled. CPU fallback is allowed but shown as degraded mode."
}
Write-Host "Title: A-Life GPU Alpha Playground."
Write-Host "Controls: Space pause/run, N step once, R reset, 1/2/3 speed, F follow, Esc quit."
Write-Host "Camera/inspector: arrows/WASD pan, +/- zoom, Q/E orbit, F follow selected stable ID. Inspector is read-only."
Write-Host "Readability: color+shape markers, creature/food/hazard stable-ID badges, concise GPU/fallback status, read-only inspector."
Write-Host "Reset/restart: press R or close and relaunch the GPU alpha fixture if the current run becomes confusing."

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
