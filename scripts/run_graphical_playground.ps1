param(
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$Command = @(
    "cargo",
    "run",
    "-p",
    "alife_game_app",
    "--features",
    "bevy-app",
    "--bin",
    "alife_game_app",
    "--",
    "visible-world-smoke",
    "crates/alife_world/tests/fixtures/p34"
)

Write-Host "A-Life graphical playground smoke command:"
Write-Host ($Command -join " ")
Write-Host "Manual graphics smoke only: requires local graphics support."

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
