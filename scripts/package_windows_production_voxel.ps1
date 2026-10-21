param(
    [switch]$DryRun,
    [switch]$SkipBuild,
    [switch]$NoZip,
    [ValidateSet("MinimumSettings30x30", "MinSpecComfort1080p", "Balanced1080p", "HighSpecScaleUp", "ResearchScale")]
    [string]$Profile = "MinSpecComfort1080p",
    [ValidateSet("MinimumSettings30x30", "MinSpecComfort1080p", "Balanced1080p", "HighSpecScaleUp", "ResearchScale")]
    [string]$FallbackProfile = "MinimumSettings30x30",
    [ValidateSet("gpu-required")]
    [string]$BrainPolicy = "gpu-required",
    [string]$OutputRoot = "target/artifacts/fvr08_windows_production",
    [string]$PackageName = "alife-production-voxel-windows"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
$ProductionFeatures = "production-voxel-frontend"

function Resolve-InWorkspacePath {
    param([string]$Path)
    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $Root $Path))
}

function Assert-NoReparseAncestor {
    param(
        [string]$FullPath,
        [string]$WorkspaceRoot
    )

    $Current = $FullPath
    while (-not [string]::IsNullOrWhiteSpace($Current)) {
        if (Test-Path -LiteralPath $Current) {
            $Item = Get-Item -LiteralPath $Current -Force
            if (($Item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) {
                throw "Refusing to use a reparse-point package path: $Current"
            }
        }
        if ([System.String]::Equals(
                $Current,
                $WorkspaceRoot,
                [System.StringComparison]::OrdinalIgnoreCase
            )) {
            return
        }
        $Parent = Split-Path -Parent $Current
        if ([string]::IsNullOrWhiteSpace($Parent) -or $Parent -eq $Current) {
            break
        }
        $Current = $Parent
    }
    throw "Package path is not rooted in the workspace: $FullPath"
}

function Assert-TargetArtifactPath {
    param([string]$Path)
    $FullPath = Resolve-InWorkspacePath $Path
    $AllowedRoot = [System.IO.Path]::GetFullPath((Join-Path $Root "target/artifacts"))
    $AllowedRootWithSeparator = $AllowedRoot.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    ) + [System.IO.Path]::DirectorySeparatorChar
    $IsAllowedChild = $FullPath.StartsWith(
        $AllowedRootWithSeparator,
        [System.StringComparison]::OrdinalIgnoreCase
    )
    if (-not $IsAllowedChild) {
        throw "Refusing to write outside target/artifacts: $FullPath"
    }
    Assert-NoReparseAncestor -FullPath $FullPath -WorkspaceRoot ([System.IO.Path]::GetFullPath($Root))
    return $FullPath
}

function Assert-PackageName {
    param([string]$Name)

    if ([string]::IsNullOrWhiteSpace($Name) -or $Name -in @('.', '..')) {
        throw "PackageName must be a non-empty directory name."
    }
    if ($Name.IndexOfAny([System.IO.Path]::GetInvalidFileNameChars()) -ge 0 `
        -or $Name.Contains([System.IO.Path]::DirectorySeparatorChar) `
        -or $Name.Contains([System.IO.Path]::AltDirectorySeparatorChar)) {
        throw "PackageName must be one directory name, not a path: $Name"
    }
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

Assert-PackageName $PackageName
$PackageRoot = Assert-TargetArtifactPath (Join-Path $OutputRoot $PackageName)
$ZipPath = Assert-TargetArtifactPath ((Join-Path $OutputRoot "$PackageName.zip"))
$ReleaseExe = Join-Path $Root "target/release/alife_game_app.exe"
$PackageExe = Join-Path $PackageRoot "alife_game_app.exe"
$PackageRunScript = Join-Path $PackageRoot "run_windows_production_voxel_package.ps1"
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

$ProductionManifestPath = "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json"
$ProductionManifest = Get-Content -Raw -LiteralPath (Join-Path $Root $ProductionManifestPath) | ConvertFrom-Json
$BundleManifestPath = "crates/alife_game_app/app_bundle_manifest.json"
$BundleManifest = Get-Content -Raw -LiteralPath (Join-Path $Root $BundleManifestPath) | ConvertFrom-Json

# Copy the files the runtime uses, including registered assets and license notices.
# Editable sources and old map exports stay in the repository, outside the game package.
$CopyFiles = @(
    "LICENSE"
    "crates/alife_game_app/environment_manifest.json"
    $BundleManifestPath
    $ProductionManifestPath
    "crates/alife_game_app/assets/production_voxel_v1/models/ATTRIBUTION.md"
    "crates/alife_game_app/assets/production_voxel_v1/models/GENEFORGE_LICENSE_RECEIPT.md"
    foreach ($Entry in $ProductionManifest.entries) {
        $Entry.local_path
        $Entry.license_ref
    }
    foreach ($Entry in $BundleManifest.entries) {
        if (-not $Entry.required) { continue }
        $Entry.relative_path
        if ($Entry.kind -eq "asset-manifest") {
            $Assets = Get-Content -Raw -LiteralPath (Join-Path $Root $Entry.relative_path) | ConvertFrom-Json
            foreach ($Asset in $Assets.entries) {
                if ($Asset.presence -eq "Required") {
                    Join-Path (Split-Path -Parent $Entry.relative_path) $Asset.relative_path
                }
            }
        }
    }
    foreach ($Shader in $BundleManifest.shader_assets) {
        if ($Shader.required) { $Shader.relative_path }
    }
) | Sort-Object -Unique

Write-Host "A-Life FVR08 Windows production voxel package builder"
Write-Host "Package root: $PackageRoot"
Write-Host "Zip path: $ZipPath"
Write-Host "Default profile: $Profile"
Write-Host "Minimum fallback profile: $FallbackProfile"
Write-Host "Brain policy default: $BrainPolicy"
Write-Host "Features: $ProductionFeatures"
Write-Host "License bundle: LICENSE plus crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json"
Write-Host "GPU authority diagnostics: failure stops learned actions"
Write-Host "Crash summary path: $PackageCrashSummary"
Write-Host "Cargo build command: $($BuildCommand -join ' ')"

if ($DryRun) {
    Write-Host "Dry run: no build, copy, zip, or artifact writes will occur."
    Write-Host "Would copy package runner: scripts/run_windows_production_voxel_package.ps1 -> $PackageRunScript"
    foreach ($File in $CopyFiles) {
        Write-Host "Would copy file: $File"
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
        environment_manifest = "crates/alife_game_app/environment_manifest.json"
        production_asset_manifest = "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json"
        production_asset_root = "crates/alife_game_app/assets/production_voxel_v1"
        license_bundle = @("LICENSE", "crates/alife_game_app/assets/production_voxel_v1/production_asset_manifest.json")
        default_profile = $Profile
        minimum_fallback_profile = $FallbackProfile
        default_brain_policy = $BrainPolicy
        default_resolution = "1920x1080"
        save_directory_policy = "Package carries clean fixture saves; user/runtime settings and diagnostics are app-managed and kept out of git."
        gpu_authority_diagnostics = [ordered]@{
            policy = $BrainPolicy
            visible = $true
            require_gpu_supported = $true
            failure_stops_learned_actions = $true
        }
        crash_summary = "diagnostics/fvr08_acceptance/crash_summary.md"
        release_tag_created = $false
        artifacts_must_remain_untracked = $true
        packaged_paths = @(
            "alife_game_app.exe"
            "run_windows_production_voxel_package.ps1"
        ) + $CopyFiles
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
GPU authority diagnostics: `gpu-required` with learned actions stopped on
preflight output and crash summary at `diagnostics/fvr08_acceptance/crash_summary.md`.
Save directory policy: clean package fixture saves ship with the package;
runtime diagnostics and generated receipts stay package-local or under
`target/artifacts/` and must not be committed.

The package includes the production voxel asset pack, production asset manifest,
MIT project license, required GPU shader sources, production environment manifest,
fixture data, and license notices. Editable Blender sources, exporters, and unused
map exports remain in the repository. It creates no release tag.
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
