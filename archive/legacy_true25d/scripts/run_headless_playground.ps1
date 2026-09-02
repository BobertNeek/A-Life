param(
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$Command = @(
    "cargo",
    "run",
    "-p",
    "alife_tools",
    "--bin",
    "p35_playground",
    "--",
    "run-all",
    "crates/alife_world/tests/fixtures/p34",
    "examples/p35/playground_manifest.json"
)

Write-Host "A-Life headless playground command:"
Write-Host ($Command -join " ")

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
