param(
    [switch]$DryRun,
    [ValidateRange(0, 120)]
    [int]$SmokeSeconds = 0,
    [ValidateSet("static-plastic-cpu-shadow-guarded", "cpu-reference", "auto-with-cpu-fallback")]
    [string]$GpuMode = "static-plastic-cpu-shadow-guarded",
    [ValidateSet("gpu-alpha", "p34")]
    [string]$Scenario = "gpu-alpha",
    [ValidateSet("auto", "dx12", "vulkan", "existing")]
    [string]$GraphicsBackend = "auto",
    [switch]$RequireGpu
)

$ErrorActionPreference = "Stop"
$PackageRoot = Split-Path -Parent $PSCommandPath
$Exe = Join-Path $PackageRoot "alife_game_app.exe"
$Manifest = Join-Path $PackageRoot "crates/alife_game_app/environment_manifest.json"
$EffectiveGraphicsBackend = $GraphicsBackend
$PreflightLog = Join-Path $PackageRoot "diagnostics/runtime_prereq.log"
$FeedbackRoot = Join-Path $PackageRoot "diagnostics/ca43_tester_feedback"
$CrashSummaryPath = Join-Path $FeedbackRoot "crash_summary.md"
$FeedbackTemplatePath = Join-Path $FeedbackRoot "tester_feedback_template.md"

function Convert-ToCa43SafeText {
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

function Write-Ca43FeedbackTemplate {
    New-Item -ItemType Directory -Force -Path $FeedbackRoot | Out-Null
    @"
# A-Life Alpha Tester Feedback Template

Do not commit screenshots, videos, logs, captures, target artifacts, model files, or caches.
Record external media paths or links only.

- Tester alias:
- Date/time:
- OS / CPU / GPU / driver:
- Display resolution:
- Exact command:
- Window opened:
- GPU status visible: GPU / CPU fallback / unavailable
- Crash summary path, if any:
- Screenshot/video external reference, if manually captured:
- Creature/food/hazard visible:
- Pause/run/step/follow/reset worked:
- Inspector readable:
- App exited cleanly:
- Findings severity: BLOCKER / HIGH / MEDIUM / LOW / MANUAL_EVIDENCE_MISSING
- Release/tag recommendation: defer unless explicitly approved.
- Full action-authoritative GPU runtime claim: false.
"@ | Set-Content -Encoding UTF8 -LiteralPath $FeedbackTemplatePath
}

function Write-Ca43CrashSummary {
    param(
        [string]$Stage,
        [int]$ExitCode,
        [string[]]$CommandParts,
        [string]$LogPath
    )

    New-Item -ItemType Directory -Force -Path $FeedbackRoot | Out-Null
    $CommandText = $CommandParts -join " "
    $SafeCommand = Convert-ToCa43SafeText $CommandText
    $SafeLogPath = Convert-ToCa43SafeText $LogPath
    @"
# A-Life CA43 Crash Summary

Schema: alife.ca43.tester_feedback_capture.v1
Stage: $Stage
Exit code: $ExitCode
User action required: true
Commit media/log artifacts: false

## Command

```text
$SafeCommand
```

## Related Log

```text
$SafeLogPath
```

## Tester Note

Attach screenshots/video only as external references. Do not commit this file,
logs, captures, target artifacts, model files, or caches.
"@ | Set-Content -Encoding UTF8 -LiteralPath $CrashSummaryPath
    Write-Host "CA43 crash summary written: $CrashSummaryPath"
}

$Args = @(
    "graphical-playground",
    "--manifest",
    $Manifest,
    "--scenario",
    $Scenario,
    "--gpu-mode",
    $GpuMode
)

if ($SmokeSeconds -gt 0) {
    $Args += @("--smoke-seconds", "$SmokeSeconds")
}
if ($RequireGpu) {
    $Args += "--require-gpu"
}

Write-Host "Starting A-Life GPU Alpha Playground from package"
Write-Host "Executable: $Exe"
Write-Host "Manifest: $Manifest"
Write-Host "Scenario requested: $Scenario"
Write-Host "GPU mode requested: $GpuMode"
Write-Host "CPU fallback is safety fallback, not the target alpha path."
Write-Host "Product claim: CpuShadowGuardedStaticPlusLiveHShadow"
Write-Host "Full action-authoritative GPU runtime claim: false"
Write-Host "Release tag: not created"
Write-Host "Controls: left click select, Space pause/run, N step once, R reset, 1/2/3 speed, F follow, Esc quit."

$IsWindowsHost = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows
)
if ($IsWindowsHost) {
    $EffectiveGraphicsBackend = if ($GraphicsBackend -eq "auto") { "dx12" } else { $GraphicsBackend }
    if ($EffectiveGraphicsBackend -eq "existing") {
        Write-Host "Graphics backend: respecting existing WGPU_BACKEND=$env:WGPU_BACKEND"
    } else {
        $env:WGPU_BACKEND = $EffectiveGraphicsBackend
        Write-Host "Graphics backend: WGPU_BACKEND=$EffectiveGraphicsBackend"
    }

    if ([string]::IsNullOrWhiteSpace($env:ALIFE_SHOW_VULKAN_LOADER_LOGS)) {
        $VulkanLoaderFilter = "wgpu_hal::vulkan::instance=off"
        if ([string]::IsNullOrWhiteSpace($env:RUST_LOG)) {
            $env:RUST_LOG = "warn,$VulkanLoaderFilter"
        } elseif ($env:RUST_LOG -notmatch "wgpu_hal::vulkan::instance") {
            $env:RUST_LOG = "$env:RUST_LOG,$VulkanLoaderFilter"
        }
    }
}

$PreflightArgs = @(
    "runtime-prereq-smoke",
    "--gpu-mode",
    $GpuMode,
    "--graphics-backend",
    $EffectiveGraphicsBackend,
    "--log",
    $PreflightLog
)
if ($RequireGpu) {
    $PreflightArgs += "--require-gpu"
}
Write-Host "Runtime preflight log: $PreflightLog"
Write-Host "Tester feedback directory: $FeedbackRoot"
Write-Host "Crash summary path on failure: $CrashSummaryPath"
Write-Host "Feedback template path: $FeedbackTemplatePath"
Write-Host "Runtime preflight command:"
Write-Host ((@($Exe) + $PreflightArgs) -join " ")

$DisplayCommand = @($Exe) + $Args
Write-Host "Package command:"
Write-Host ($DisplayCommand -join " ")

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

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $PreflightLog) | Out-Null
Write-Ca43FeedbackTemplate
& $Exe @PreflightArgs
$PreflightExitCode = $LASTEXITCODE
if ($PreflightExitCode -ne 0) {
    Write-Ca43CrashSummary -Stage "runtime-preflight" -ExitCode $PreflightExitCode -CommandParts (@($Exe) + $PreflightArgs) -LogPath $PreflightLog
    Write-Error "A-Life runtime preflight failed. See $PreflightLog"
    exit $PreflightExitCode
}

& $Exe @Args
$AppExitCode = $LASTEXITCODE
if ($AppExitCode -ne 0) {
    Write-Ca43CrashSummary -Stage "graphical-playground" -ExitCode $AppExitCode -CommandParts (@($Exe) + $Args) -LogPath $PreflightLog
}
exit $AppExitCode
