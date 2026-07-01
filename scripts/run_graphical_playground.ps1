param(
    [switch]$DryRun,
    [ValidateRange(0, 120)]
    [int]$SmokeSeconds = 0,
    [ValidateSet("cpu-reference", "static-plastic-cpu-shadow-guarded", "auto-with-cpu-fallback")]
    [string]$GpuMode = "static-plastic-cpu-shadow-guarded",
    [ValidateSet("player", "dev-overlay", "full-debug")]
    [string]$ViewMode = "player",
    [ValidateSet("gpu-alpha", "p34")]
    [string]$Scenario = "gpu-alpha",
    [string]$EnvironmentManifest = "",
    [ValidateSet("auto", "dx12", "vulkan", "existing")]
    [string]$GraphicsBackend = "auto",
    [switch]$RequireGpu
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$EffectiveGraphicsBackend = $GraphicsBackend

if ($SmokeSeconds -gt 0) {
    $ModeArgs = @("graphical-playground")
    $ModeArgs += @("--scenario", $Scenario, "--gpu-mode", $GpuMode, "--view-mode", $ViewMode, "--smoke-seconds", "$SmokeSeconds")
    $ModeLabel = "bounded graphical playground smoke"
} else {
    $ModeArgs = @("graphical-playground")
    $ModeArgs += @("--scenario", $Scenario, "--gpu-mode", $GpuMode, "--view-mode", $ViewMode)
    $ModeLabel = "persistent graphical playground"
}

if (-not [string]::IsNullOrWhiteSpace($EnvironmentManifest)) {
    $ModeArgs += @("--manifest", $EnvironmentManifest)
}

if ($RequireGpu) {
    $ModeArgs += "--require-gpu"
}

$FeatureList = if ($GpuMode -eq "cpu-reference") { "bevy-app" } else { "bevy-app gpu-runtime" }
$PreflightLog = Join-Path $Root "target/artifacts/ca42_runtime_prereq/runtime_prereq.log"
$FeedbackRoot = Join-Path $Root "target/artifacts/ca43_tester_feedback"
$CrashSummaryPath = Join-Path $FeedbackRoot "crash_summary.md"
$FeedbackTemplatePath = Join-Path $FeedbackRoot "tester_feedback_template.md"
$SmokeWatchdogJob = $null

function Format-CommandArgument {
    param([string]$Value)

    if ($Value -match "[\s'`"]") {
        return "'" + ($Value -replace "'", "''") + "'"
    }

    return $Value
}

function Convert-ToCa43SafeText {
    param([AllowNull()][string]$Text)

    if ($null -eq $Text) {
        return ""
    }

    $Safe = $Text.Replace($Root, "<local-path>")
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
    $CommandText = ($CommandParts | ForEach-Object { Format-CommandArgument $_ }) -join " "
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

function Start-SmokeWindowWatchdog {
    param(
        [int]$Seconds,
        [string]$Title
    )

    if ($Seconds -le 0) {
        return $null
    }

    return Start-Job -ScriptBlock {
        param([int]$Seconds, [string]$Title)

        $WindowSeenAt = $null
        $SearchDeadline = (Get-Date).AddMinutes(10)
        $CloseSlackSeconds = 4

        while ((Get-Date) -lt $SearchDeadline) {
            $Matches = @(Get-Process -Name "alife_game_app" -ErrorAction SilentlyContinue | Where-Object {
                $_.MainWindowTitle -eq $Title
            })

            if ($Matches.Count -gt 0) {
                if ($null -eq $WindowSeenAt) {
                    $WindowSeenAt = Get-Date
                }

                $Elapsed = ((Get-Date) - $WindowSeenAt).TotalSeconds
                if ($Elapsed -ge ($Seconds + $CloseSlackSeconds)) {
                    foreach ($Process in $Matches) {
                        [void]$Process.CloseMainWindow()
                    }
                    return
                }
            } else {
                $WindowSeenAt = $null
            }

            Start-Sleep -Milliseconds 500
        }
    } -ArgumentList $Seconds, $Title
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
Write-Host "Scenario requested: $Scenario"
if (-not [string]::IsNullOrWhiteSpace($EnvironmentManifest)) {
    Write-Host "Environment manifest: $EnvironmentManifest"
} else {
    Write-Host "Environment manifest: crates/alife_game_app/environment_manifest.json"
}
Write-Host "GPU mode requested: $GpuMode"
Write-Host "View mode requested: $ViewMode"
Write-Host "CPU fallback is safety fallback, not the target alpha path."
Write-Host "Graphics backend requested: $GraphicsBackend"
if ($RequireGpu) {
    Write-Host "RequireGpu: enabled. A CPU fallback exits as a clear GPU-unavailable failure."
} else {
    Write-Host "RequireGpu: disabled. CPU fallback is allowed but shown as degraded mode."
}
Write-Host "Title: A-Life GPU Alpha Playground."
Write-Host "Controls: left click select, Space pause/run, N step once, R reset, 1/2/3 speed, F follow, Esc quit."
Write-Host "Camera/inspector: arrows/WASD pan, +/- zoom, Q/E orbit, F follow selected stable ID. Inspector is read-only."
Write-Host "Readability: default Player View hides debug labels; use -ViewMode dev-overlay or -ViewMode full-debug for diagnostics."
Write-Host "Reset/restart: press R or close and relaunch the GPU alpha fixture if the current run becomes confusing."
$IsWindowsHost = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows
)

if ($IsWindowsHost) {
    $EffectiveGraphicsBackend = if ($GraphicsBackend -eq "auto") {
        if ($ViewMode -eq "player") { "vulkan" } else { "dx12" }
    } else {
        $GraphicsBackend
    }

    if ($EffectiveGraphicsBackend -eq "existing") {
        if ([string]::IsNullOrWhiteSpace($env:WGPU_BACKEND)) {
            Write-Host "Graphics backend: existing WGPU_BACKEND is empty; wgpu will choose its default."
        } else {
            Write-Host "Graphics backend: respecting existing WGPU_BACKEND=$env:WGPU_BACKEND"
        }
    } else {
        $PreviousWgpuBackend = $env:WGPU_BACKEND
        $env:WGPU_BACKEND = $EffectiveGraphicsBackend

        if ([string]::IsNullOrWhiteSpace($PreviousWgpuBackend) -or $PreviousWgpuBackend -eq $EffectiveGraphicsBackend) {
            Write-Host "Graphics backend: WGPU_BACKEND=$EffectiveGraphicsBackend for clean Windows alpha launch."
        } else {
            Write-Host "Graphics backend: overriding inherited WGPU_BACKEND=$PreviousWgpuBackend with $EffectiveGraphicsBackend for clean Windows alpha launch."
        }

        if ($EffectiveGraphicsBackend -eq "vulkan") {
            if ($GraphicsBackend -eq "auto" -and $ViewMode -eq "player") {
                Write-Host "Graphics backend: auto selected Vulkan for True 2.5D Player View on this Windows host."
            } else {
                Write-Host "Graphics backend: Vulkan diagnostics requested; injected overlay loader warnings may appear if ALIFE_SHOW_VULKAN_LOADER_LOGS=1."
            }
        } else {
            Write-Host "Graphics backend: DX12 selected. Use -GraphicsBackend vulkan for True 2.5D Player View diagnostics."
        }
    }
} elseif (-not [string]::IsNullOrWhiteSpace($env:WGPU_BACKEND)) {
    Write-Host "Graphics backend: WGPU_BACKEND=$env:WGPU_BACKEND"
}

$VulkanLoaderFilter = "wgpu_hal::vulkan::instance=off"
if ($IsWindowsHost -and [string]::IsNullOrWhiteSpace($env:ALIFE_SHOW_VULKAN_LOADER_LOGS)) {
    if ([string]::IsNullOrWhiteSpace($env:RUST_LOG)) {
        $env:RUST_LOG = "warn,$VulkanLoaderFilter"
    } elseif ($env:RUST_LOG -notmatch "wgpu_hal::vulkan::instance") {
        $env:RUST_LOG = "$env:RUST_LOG,$VulkanLoaderFilter"
    }

    Write-Host "Log filter: hiding non-fatal Vulkan loader layer noise from injected overlays. Set ALIFE_SHOW_VULKAN_LOADER_LOGS=1 for diagnostics."
}

$PreflightCommand = @(
    "cargo",
    "run",
    "-p",
    "alife_game_app",
    "--features",
    $FeatureList,
    "--bin",
    "alife_game_app",
    "--",
    "runtime-prereq-smoke",
    "--gpu-mode",
    $GpuMode,
    "--graphics-backend",
    $EffectiveGraphicsBackend,
    "--log",
    $PreflightLog
)
if ($RequireGpu) {
    $PreflightCommand += "--require-gpu"
}

Write-Host "Runtime preflight log: $PreflightLog"
Write-Host "Tester feedback directory: $FeedbackRoot"
Write-Host "Crash summary path on failure: $CrashSummaryPath"
Write-Host "Feedback template path: $FeedbackTemplatePath"
Write-Host "Runtime preflight command:"
Write-Host (($PreflightCommand | ForEach-Object { Format-CommandArgument $_ }) -join " ")

if ($DryRun) {
    exit 0
}

Push-Location $Root
try {
    Write-Ca43FeedbackTemplate
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $PreflightLog) | Out-Null
    $PreflightArgs = $PreflightCommand[1..($PreflightCommand.Length - 1)]
    & $PreflightCommand[0] @PreflightArgs
    $PreflightExitCode = $LASTEXITCODE
    if ($PreflightExitCode -ne 0) {
        Write-Ca43CrashSummary -Stage "runtime-preflight" -ExitCode $PreflightExitCode -CommandParts $PreflightCommand -LogPath $PreflightLog
        Write-Error "A-Life runtime preflight failed. See $PreflightLog"
        exit $PreflightExitCode
    }

    if ($SmokeSeconds -gt 0) {
        $SmokeWindowTitle = "A-Life GPU Alpha Playground - smoke ${SmokeSeconds}s"
        $SmokeWatchdogJob = Start-SmokeWindowWatchdog -Seconds $SmokeSeconds -Title $SmokeWindowTitle
    }

    $Args = $Command[1..($Command.Length - 1)]
    & $Command[0] @Args
    $AppExitCode = $LASTEXITCODE
    if ($AppExitCode -ne 0) {
        Write-Ca43CrashSummary -Stage "graphical-playground" -ExitCode $AppExitCode -CommandParts $Command -LogPath $PreflightLog
    }
    exit $AppExitCode
} finally {
    if ($null -ne $SmokeWatchdogJob) {
        if ($SmokeWatchdogJob.State -eq "Running") {
            Stop-Job -Job $SmokeWatchdogJob -ErrorAction SilentlyContinue
        }
        Remove-Job -Job $SmokeWatchdogJob -Force -ErrorAction SilentlyContinue
    }
    Pop-Location
}
