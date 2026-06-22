param(
    [switch]$DryRun,
    [ValidateRange(0, 120)]
    [int]$SmokeSeconds = 0
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)

if ($SmokeSeconds -gt 0) {
    $ModeArgs = @("graphical-playground-smoke", "--seconds", "$SmokeSeconds")
    $ModeLabel = "bounded graphical playground smoke"
} else {
    $ModeArgs = @("graphical-playground")
    $ModeLabel = "persistent graphical playground"
}

$Command = @(
    "cargo",
    "run",
    "-p",
    "alife_game_app",
    "--features",
    "bevy-app",
    "--bin",
    "alife_game_app",
    "--"
)
$Command += $ModeArgs
$Command += "crates/alife_world/tests/fixtures/p34"

Write-Host "A-Life $ModeLabel command:"
Write-Host ($Command -join " ")
Write-Host "Manual graphics path: requires local windowing/graphics support. CPU fallback is used for cognition/backend status."
Write-Host "Controls: Space pause/run, N step once, 1/2/3 speed, Esc quit."
Write-Host "Camera/inspector: arrows/WASD pan, +/- zoom, Q/E orbit, F follow selected stable ID. Inspector is read-only."

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
