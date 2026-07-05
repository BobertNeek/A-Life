param(
    [switch]$DryRun,
    [switch]$SkipBuild,
    [switch]$NoZip,
    [ValidateSet("MinimumSettings30x30", "MinSpecComfort1080p", "Balanced1080p", "HighSpecScaleUp", "ResearchScale")]
    [string]$Profile = "MinSpecComfort1080p",
    [ValidateSet("MinimumSettings30x30", "MinSpecComfort1080p", "Balanced1080p", "HighSpecScaleUp", "ResearchScale")]
    [string]$FallbackProfile = "MinimumSettings30x30",
    [ValidateSet("cpu-reference", "static-plastic-cpu-shadow-guarded", "auto-with-cpu-fallback")]
    [string]$GpuMode = "auto-with-cpu-fallback",
    [string]$OutputRoot = "target/artifacts/fvr08_windows_production",
    [string]$PackageName = "alife-production-voxel-windows"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$ProductionFeatures = "bevy-app gpu-runtime voxel-backend production-assets vfx-hanabi"

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
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Copy-Item -Path (Join-Path $Source "*") -Destination $Destination -Recurse -Force
}

function Write-ProductionOnlyEnvironmentManifest {
    param([string]$DestinationRoot)

    $EnvironmentPath = Join-Path $DestinationRoot "crates/alife_game_app/environment_manifest.json"
    $Environment = Get-Content -Raw -LiteralPath $EnvironmentPath | ConvertFrom-Json
    $ProductionScenarios = @(
        $Environment.scenarios | Where-Object { $_.id -eq "production-voxel" }
    )
    if ($ProductionScenarios.Count -ne 1) {
        throw "Packaged environment manifest must contain exactly one production-voxel scenario."
    }
    $Environment.default_scenario_id = "production-voxel"
    $Environment.scenarios = $ProductionScenarios
    $Json = $Environment | ConvertTo-Json -Depth 8
    $Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($EnvironmentPath, $Json + [Environment]::NewLine, $Utf8NoBom)
}

$PackageRoot = Assert-TargetArtifactPath (Join-Path $OutputRoot $PackageName)
$ZipPath = Assert-TargetArtifactPath ((Join-Path $OutputRoot "$PackageName.zip"))
$ReleaseExe = Join-Path $Root "target/release/alife_game_app.exe"
$PackageExe = Join-Path $PackageRoot "alife_game_app.exe"
$PackageRunScript = Join-Path $PackageRoot "run_windows_production_voxel_package.ps1"
$CopiedSourceLauncher = Join-Path $PackageRoot "scripts/run_production_voxel_frontend.ps1"
$PackageManifest = Join-Path $PackageRoot "crates/alife_game_app/environment_manifest.json"
$PackageCrashSummary = Join-Path $PackageRoot "diagnostics/fvr08_acceptance/crash_summary.md"

$BuildCommand = @(
    "cargo",
    "build",
    "-p",
    "alife_game_app",
    "--bin",
    "alife_game_app",
    "--features",
    $ProductionFeatures,
    "--release"
)

$CopyFiles = @(
    "LICENSE",
    "scripts/run_production_voxel_frontend.ps1",
    "scripts/run_windows_production_voxel_package.ps1",
    "crates/alife_game_app/environment_manifest.json",
    "crates/alife_game_app/app_bundle_manifest.json",
    "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json",
    "crates/alife_world/tests/fixtures/production_voxel/tiny_config.json",
    "crates/alife_world/tests/fixtures/production_voxel/tiny_asset_manifest.json",
    "crates/alife_world/tests/fixtures/production_voxel/tiny_save.json",
    "crates/alife_world/tests/fixtures/production_voxel/assets/tiny_generated_weights_ref.json"
)

$CopyDirectories = @(
    "crates/alife_game_app/assets/production_voxel_v1",
    "crates/alife_gpu_backend/shaders"
)

Write-Host "A-Life FVR08 Windows production voxel package builder"
Write-Host "Package root: $PackageRoot"
Write-Host "Zip path: $ZipPath"
Write-Host "Default profile: $Profile"
Write-Host "Minimum fallback profile: $FallbackProfile"
Write-Host "GPU mode default: $GpuMode"
Write-Host "Features: $ProductionFeatures"
Write-Host "License bundle: LICENSE plus crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json"
Write-Host "GPU fallback diagnostics: gpu_fallback_diagnostics metadata and production-voxel preflight output"
Write-Host "Crash summary path: $PackageCrashSummary"
Write-Host "Cargo build command: $($BuildCommand -join ' ')"

if ($DryRun) {
    Write-Host "Dry run: no build, copy, zip, or artifact writes will occur."
    Write-Host "Would copy package runner: scripts/run_windows_production_voxel_package.ps1 -> $PackageRunScript"
    Write-Host "Would copy source launcher: scripts/run_production_voxel_frontend.ps1 -> $CopiedSourceLauncher"
    foreach ($File in $CopyFiles) {
        Write-Host "Would copy file: $File"
    }
    foreach ($Directory in $CopyDirectories) {
        Write-Host "Would copy directory: $Directory"
    }
    Write-Host "Would write metadata: package_metadata.json"
    Write-Host "Would write package README: README_PACKAGE.md"
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
    Copy-Item -LiteralPath (Join-Path $Root "scripts/run_windows_production_voxel_package.ps1") `
        -Destination $PackageRunScript -Force

    foreach ($File in $CopyFiles) {
        Copy-PackageFile -RelativePath $File -DestinationRoot $PackageRoot
    }
    foreach ($Directory in $CopyDirectories) {
        Copy-PackageDirectory -RelativePath $Directory -DestinationRoot $PackageRoot
    }
    Write-ProductionOnlyEnvironmentManifest -DestinationRoot $PackageRoot

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
        schema = "alife.fvr08.windows_production_package.v1"
        schema_version = 1
        package_id = $PackageName
        branch = $Branch.Trim()
        commit = $Commit.Trim()
        working_tree_dirty = $WorkingTreeDirty
        built_at_utc = [DateTime]::UtcNow.ToString("o")
        executable = "alife_game_app.exe"
        runner = "run_windows_production_voxel_package.ps1"
        source_launcher = "scripts/run_production_voxel_frontend.ps1"
        environment_manifest = "crates/alife_game_app/environment_manifest.json"
        production_asset_manifest = "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json"
        production_asset_root = "crates/alife_game_app/assets/production_voxel_v1"
        license_bundle = @("LICENSE", "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json")
        default_profile = $Profile
        minimum_fallback_profile = $FallbackProfile
        default_gpu_mode = $GpuMode
        default_resolution = "1920x1080"
        save_directory_policy = "Package carries clean fixture saves; user/runtime settings and diagnostics are app-managed and kept out of git."
        gpu_fallback_diagnostics = [ordered]@{
            mode = $GpuMode
            visible = $true
            require_gpu_supported = $true
            cpu_fallback_available = $true
        }
        crash_summary = "diagnostics/fvr08_acceptance/crash_summary.md"
        release_tag_created = $false
        artifacts_must_remain_untracked = $true
        packaged_paths = @(
            "alife_game_app.exe",
            "run_windows_production_voxel_package.ps1",
            "scripts/run_production_voxel_frontend.ps1",
            "crates/alife_game_app/environment_manifest.json",
            "crates/alife_game_app/assets/production_voxel_v1",
            "crates/alife_world/tests/fixtures/production_voxel",
            "crates/alife_gpu_backend/shaders",
            "LICENSE"
        )
    }
    $Metadata | ConvertTo-Json -Depth 8 | Set-Content -Encoding UTF8 (Join-Path $PackageRoot "package_metadata.json")

    @"
# A-Life Production Voxel Windows Package

Run from this directory:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\run_windows_production_voxel_package.ps1
```

Minimum fallback run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\run_windows_production_voxel_package.ps1 -Profile MinimumSettings30x30 -Population 30 -RecordPerformance
```

Default profile: `MinSpecComfort1080p`.
Minimum fallback profile: `MinimumSettings30x30`.
GPU fallback diagnostics: `auto-with-cpu-fallback` with visible production
preflight output and crash summary at `diagnostics/fvr08_acceptance/crash_summary.md`.
Save directory policy: clean package fixture saves ship with the package;
runtime diagnostics and generated receipts stay package-local or under
`target/artifacts/` and must not be committed.

The package includes the production voxel asset pack, production asset manifest,
MIT project license, GPU shader sources, production environment manifest, and
production-named fixture data. It creates no release tag.
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
    Write-Host "Source launcher copy: $CopiedSourceLauncher"
    Write-Host "Manifest: $PackageManifest"
    Write-Host "Release tag: not created"
    exit 0
} finally {
    Pop-Location
}
