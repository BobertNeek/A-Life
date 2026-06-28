param(
    [switch]$DryRun,
    [switch]$SkipBuild,
    [switch]$NoZip,
    [ValidateSet("static-plastic-cpu-shadow-guarded", "cpu-reference", "auto-with-cpu-fallback")]
    [string]$GpuMode = "static-plastic-cpu-shadow-guarded",
    [ValidateSet("gpu-alpha", "p34")]
    [string]$Scenario = "gpu-alpha",
    [string]$OutputRoot = "target/artifacts/ca41_windows_alpha",
    [string]$PackageName = "alife-gpu-alpha-windows"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)

function Resolve-InWorkspacePath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $Root $Path))
}

function Assert-TargetArtifactPath {
    param([string]$Path)
    $FullPath = Resolve-InWorkspacePath $Path
    $AllowedRoot = [System.IO.Path]::GetFullPath((Join-Path $Root "target/artifacts"))
    $AllowedRootWithSeparator = $AllowedRoot.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    ) + [System.IO.Path]::DirectorySeparatorChar
    $IsAllowedRoot = [System.String]::Equals(
        $FullPath,
        $AllowedRoot,
        [System.StringComparison]::OrdinalIgnoreCase
    )
    $IsAllowedChild = $FullPath.StartsWith(
        $AllowedRootWithSeparator,
        [System.StringComparison]::OrdinalIgnoreCase
    )
    if (-not ($IsAllowedRoot -or $IsAllowedChild)) {
        throw "Refusing to write outside target/artifacts: $FullPath"
    }
    return $FullPath
}

function Copy-PackageFile {
    param(
        [string]$RelativePath,
        [string]$DestinationRoot
    )
    $Source = Join-Path $Root $RelativePath
    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        throw "Required package file is missing: $RelativePath"
    }
    $Destination = Join-Path $DestinationRoot $RelativePath
    $Parent = Split-Path -Parent $Destination
    New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

function Copy-PackageDirectory {
    param(
        [string]$RelativePath,
        [string]$DestinationRoot
    )
    $Source = Join-Path $Root $RelativePath
    if (-not (Test-Path -LiteralPath $Source -PathType Container)) {
        throw "Required package directory is missing: $RelativePath"
    }
    $Destination = Join-Path $DestinationRoot $RelativePath
    $Parent = Split-Path -Parent $Destination
    New-Item -ItemType Directory -Force -Path $Parent | Out-Null
    Copy-Item -LiteralPath $Source -Destination $Destination -Recurse -Force
}

$PackageRoot = Assert-TargetArtifactPath (Join-Path $OutputRoot $PackageName)
$ZipPath = Assert-TargetArtifactPath ((Join-Path $OutputRoot "$PackageName.zip"))
$ReleaseExe = Join-Path $Root "target/release/alife_game_app.exe"
$PackageExe = Join-Path $PackageRoot "alife_game_app.exe"
$PackageRunScript = Join-Path $PackageRoot "run_windows_alpha_package.ps1"
$PackageManifest = Join-Path $PackageRoot "crates/alife_game_app/environment_manifest.json"

$BuildCommand = @(
    "cargo",
    "build",
    "-p",
    "alife_game_app",
    "--bin",
    "alife_game_app",
    "--features",
    "bevy-app gpu-runtime",
    "--release"
)

$CopyFiles = @(
    "crates/alife_game_app/environment_manifest.json",
    "crates/alife_game_app/app_bundle_manifest.json",
    "crates/alife_game_app/placeholder_art_manifest.json",
    "crates/alife_world/tests/fixtures/gpu_alpha/tiny_config.json",
    "crates/alife_world/tests/fixtures/gpu_alpha/tiny_asset_manifest.json",
    "crates/alife_world/tests/fixtures/gpu_alpha/tiny_save.json",
    "crates/alife_world/tests/fixtures/gpu_alpha/assets/tiny_generated_weights_ref.json",
    "crates/alife_world/tests/fixtures/p34/tiny_config.json",
    "crates/alife_world/tests/fixtures/p34/tiny_asset_manifest.json",
    "crates/alife_world/tests/fixtures/p34/tiny_save.json",
    "crates/alife_world/tests/fixtures/p34/assets/tiny_generated_weights_ref.json",
    "docs/creatures_agi_roadmap_pack/templates/CA43_TESTER_FEEDBACK_TEMPLATE.md",
    "examples/model_manifests/local_semantic_models.json"
)

$CopyDirectories = @(
    "crates/alife_gpu_backend/shaders"
)

Write-Host "A-Life CA41 Windows alpha package builder"
Write-Host "Package root: $PackageRoot"
Write-Host "Zip path: $ZipPath"
Write-Host "GPU mode default: $GpuMode"
Write-Host "Scenario default: $Scenario"
Write-Host "Release tag: not created"
Write-Host "Cargo build command: $($BuildCommand -join ' ')"

if ($DryRun) {
    Write-Host "Dry run: no build, copy, zip, or artifact writes will occur."
    Write-Host "Would copy runner: scripts/run_windows_alpha_package.ps1 -> $PackageRunScript"
    foreach ($File in $CopyFiles) {
        Write-Host "Would copy file: $File"
    }
    foreach ($Directory in $CopyDirectories) {
        Write-Host "Would copy directory: $Directory"
    }
    Write-Host "Would write metadata: package_metadata.json"
    exit 0
}

Push-Location $Root
try {
    if (-not $SkipBuild) {
        & $BuildCommand[0] @($BuildCommand[1..($BuildCommand.Length - 1)])
        if ($LASTEXITCODE -ne 0) {
            throw "Release build failed with exit code $LASTEXITCODE"
        }
    }

    if (-not (Test-Path -LiteralPath $ReleaseExe -PathType Leaf)) {
        throw "Release binary not found: $ReleaseExe. Run without -SkipBuild or build release first."
    }

    if (Test-Path -LiteralPath $PackageRoot) {
        Remove-Item -LiteralPath $PackageRoot -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $PackageRoot | Out-Null

    Copy-Item -LiteralPath $ReleaseExe -Destination $PackageExe -Force
    Copy-Item -LiteralPath (Join-Path $Root "scripts/run_windows_alpha_package.ps1") `
        -Destination $PackageRunScript -Force

    foreach ($File in $CopyFiles) {
        Copy-PackageFile -RelativePath $File -DestinationRoot $PackageRoot
    }
    foreach ($Directory in $CopyDirectories) {
        Copy-PackageDirectory -RelativePath $Directory -DestinationRoot $PackageRoot
    }

    $Commit = (git rev-parse --short HEAD 2>$null)
    if ([string]::IsNullOrWhiteSpace($Commit)) {
        $Commit = "unknown"
    }
    $Branch = (git branch --show-current 2>$null)
    if ([string]::IsNullOrWhiteSpace($Branch)) {
        $Branch = "unknown"
    }
    $GitStatus = (git status --short --untracked-files=no 2>$null)
    $WorkingTreeDirty = -not [string]::IsNullOrWhiteSpace($GitStatus)

    $Metadata = [ordered]@{
        schema = "alife.ca41.windows_alpha_package.v1"
        schema_version = 1
        package_id = $PackageName
        branch = $Branch.Trim()
        commit = $Commit.Trim()
        working_tree_dirty = $WorkingTreeDirty
        built_at_utc = [DateTime]::UtcNow.ToString("o")
        executable = "alife_game_app.exe"
        runner = "run_windows_alpha_package.ps1"
        environment_manifest = "crates/alife_game_app/environment_manifest.json"
        default_scenario = $Scenario
        default_gpu_mode = $GpuMode
        product_runtime_claim = "CpuShadowGuardedStaticPlusLiveHShadow"
        cpu_shadow_parity_required = $true
        cpu_fallback_available = $true
        full_action_authoritative_claim = $false
        release_tag_created = $false
        artifacts_must_remain_untracked = $true
        packaged_paths = @(
            "crates/alife_game_app",
            "crates/alife_world/tests/fixtures/gpu_alpha",
            "crates/alife_world/tests/fixtures/p34",
            "docs/creatures_agi_roadmap_pack/templates/CA43_TESTER_FEEDBACK_TEMPLATE.md",
            "crates/alife_gpu_backend/shaders",
            "examples/model_manifests/local_semantic_models.json"
        )
    }
    $Metadata | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 (Join-Path $PackageRoot "package_metadata.json")

    @"
# A-Life GPU Alpha Windows Package

Run from this directory:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\run_windows_alpha_package.ps1
```

Timed smoke:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\run_windows_alpha_package.ps1 -SmokeSeconds 30
```

This package is GPU-first and requests `static-plastic-cpu-shadow-guarded` by
default. CPU fallback remains available and is visibly degraded/safety mode.
This is not a full action-authoritative GPU runtime claim and no release tag is
created by the package builder.

If launch fails, the runner writes a sanitized CA43 crash summary and tester
feedback template under `diagnostics/ca43_tester_feedback/`. Keep those files
local or reference them externally; do not commit screenshots, logs, captures,
target artifacts, model files, or caches.
"@ | Set-Content -Encoding UTF8 (Join-Path $PackageRoot "README_PACKAGE.md")

    if (-not $NoZip) {
        if (Test-Path -LiteralPath $ZipPath) {
            Remove-Item -LiteralPath $ZipPath -Force
        }
        $PackageChildren = Get-ChildItem -LiteralPath $PackageRoot -Force |
            Select-Object -ExpandProperty FullName
        if ($PackageChildren.Count -eq 0) {
            throw "Package directory is empty; refusing to create ZIP."
        }
        Compress-Archive -LiteralPath $PackageChildren -DestinationPath $ZipPath -Force
    }

    Write-Host "Package complete."
    Write-Host "Package root: $PackageRoot"
    if (-not $NoZip) {
        Write-Host "Zip path: $ZipPath"
    }
    Write-Host "Run script: $PackageRunScript"
    Write-Host "Manifest: $PackageManifest"
    Write-Host "Release tag: not created"
    exit 0
} finally {
    Pop-Location
}
