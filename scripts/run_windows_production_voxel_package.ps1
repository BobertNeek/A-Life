param(
    [switch]$DryRun,
    [ValidateRange(0, 120)]
    [int]$SmokeSeconds = 0,
    [ValidateSet("MinimumSettings30x30", "MinSpecComfort1080p", "Balanced1080p", "HighSpecScaleUp", "ResearchScale")]
    [string]$Profile = "MinSpecComfort1080p",
    [ValidateRange(0, 500)]
    [int]$Population = 0,
    [string]$Resolution = "1920x1080",
    [ValidateSet("gpu-required")]
    [string]$BrainPolicy = "gpu-required",
    [ValidateSet("auto", "dx12", "vulkan", "existing")]
    [string]$GraphicsBackend = "auto",
    [switch]$RequireGpu,
    [switch]$RecordPerformance
)

$ErrorActionPreference = "Stop"
$PackageRoot = Split-Path -Parent $PSCommandPath
$Exe = Join-Path $PackageRoot "alife_game_app.exe"
$Manifest = Join-Path $PackageRoot "crates/alife_game_app/environment_manifest.json"
$CrashRoot = Join-Path $PackageRoot "diagnostics/fvr08_acceptance"
$CrashSummaryPath = Join-Path $CrashRoot "crash_summary.md"
$EffectiveGraphicsBackend = $GraphicsBackend

function Convert-ToFvr08SafeText {
    param([AllowNull()][string]$Text)

    if ($null -eq $Text) {
        return ""
    }

    $Safe = $Text.Replace($PackageRoot, "<package-root>")
    if (-not [string]::IsNullOrWhiteSpace($env:USERPROFILE)) {
        $Safe = $Safe.Replace($env:USERPROFILE, "<local-path>")
    }
    return $Safe
}

function Write-Fvr08CrashSummary {
    param(
        [int]$ExitCode,
        [string[]]$CommandParts
    )

    New-Item -ItemType Directory -Force -Path $CrashRoot | Out-Null
    $SafeCommand = Convert-ToFvr08SafeText ($CommandParts -join " ")
    @"
# A-Life FVR08 Crash Summary

Schema: alife.fvr08.windows_production_crash_summary.v1
Stage: production-voxel
Exit code: $ExitCode
Commit media/log artifacts: false
Default profile: MinSpecComfort1080p
Minimum fallback profile: MinimumSettings30x30
GPU authority diagnostics: failure stops learned actions

## Command

Command:

$SafeCommand

## Operator Note

Keep screenshots, logs, captures, target artifacts, model files, and caches out
of git. Reference local or external evidence paths in acceptance receipts.
"@ | Set-Content -Encoding UTF8 -LiteralPath $CrashSummaryPath
    Write-Host "FVR08 crash summary written: $CrashSummaryPath"
}

$Args = @(
    "production-voxel",
    "--manifest",
    $Manifest,
    "--scenario",
    "production-voxel",
    "--profile",
    $Profile,
    "--resolution",
    $Resolution,
    "--brain-policy",
    $BrainPolicy,
    "--graphics-backend",
    $EffectiveGraphicsBackend
)

if ($Population -gt 0) {
    $Args += @("--population", "$Population")
}
if ($SmokeSeconds -gt 0) {
    $Args += @("--smoke-seconds", "$SmokeSeconds")
}
if ($RequireGpu) {
    $Args += "--require-gpu"
}
if ($RecordPerformance) {
    $Args += "--record-performance"
}

Write-Host "Starting A-Life Voxel Frontend production package"
Write-Host "Executable: $Exe"
Write-Host "Manifest: $Manifest"
Write-Host "Profile: $Profile"
Write-Host "Minimum fallback profile: MinimumSettings30x30"
Write-Host "Brain policy requested: $BrainPolicy"
Write-Host "GPU authority diagnostics: failure stops learned actions"
Write-Host "Save directory policy: package-local fixture saves plus runtime/user settings under package diagnostics or app-managed artifacts."
Write-Host "Crash summary path on failure: $CrashSummaryPath"
Write-Host "Package command:"
Write-Host ((@($Exe) + $Args) -join " ")

if ($DryRun) {
    if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
        Write-Host "Dry run note: package executable is not present yet: $Exe"
    }
    if (-not (Test-Path -LiteralPath $Manifest -PathType Leaf)) {
        Write-Host "Dry run note: package environment manifest is not present yet: $Manifest"
    }
    exit 0
}

if (-not (Test-Path -LiteralPath $Exe -PathType Leaf)) {
    throw "A-Life package executable is missing: $Exe"
}
if (-not (Test-Path -LiteralPath $Manifest -PathType Leaf)) {
    throw "A-Life package environment manifest is missing: $Manifest"
}

if ([System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
        [System.Runtime.InteropServices.OSPlatform]::Windows
    )) {
    if ($EffectiveGraphicsBackend -eq "existing") {
        Write-Host "Graphics backend: respecting existing WGPU_BACKEND=$env:WGPU_BACKEND"
    } elseif ($EffectiveGraphicsBackend -ne "auto") {
        $env:WGPU_BACKEND = $EffectiveGraphicsBackend
        Write-Host "Graphics backend: WGPU_BACKEND=$EffectiveGraphicsBackend"
    }
}

Push-Location $PackageRoot
try {
    & $Exe @Args
    $AppExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
if ($AppExitCode -ne 0) {
    Write-Fvr08CrashSummary -ExitCode $AppExitCode -CommandParts (@($Exe) + $Args)
}
exit $AppExitCode
