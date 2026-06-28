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

& $Exe @Args
exit $LASTEXITCODE
