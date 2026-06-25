param(
    [switch]$DryRun,
    [ValidateRange(0, 120)]
    [int]$SmokeSeconds = 0,
    [ValidateSet("cpu-reference", "static-plastic-cpu-shadow-guarded", "auto-with-cpu-fallback")]
    [string]$GpuMode = "static-plastic-cpu-shadow-guarded",
    [ValidateSet("auto", "dx12", "vulkan", "existing")]
    [string]$GraphicsBackend = "auto",
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
Write-Host "Graphics backend requested: $GraphicsBackend"
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
$IsWindowsHost = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::Windows
)

if ($IsWindowsHost) {
    $EffectiveGraphicsBackend = if ($GraphicsBackend -eq "auto") { "dx12" } else { $GraphicsBackend }

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
            Write-Host "Graphics backend: Vulkan diagnostics requested; injected overlay loader warnings may appear if ALIFE_SHOW_VULKAN_LOADER_LOGS=1."
        } else {
            Write-Host "Graphics backend: use -GraphicsBackend vulkan only for Vulkan diagnostics."
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
