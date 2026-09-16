param(
    [string]$BlenderExe = "blender",
    [string]$Manifest = "crates/alife_game_app/assets/true_25d_alpha_v1/true_25d_manifest.json",
    [string]$Receipt = "target/artifacts/true25d_blender_normalization/receipt.json",
    [int]$DecimationThresholdTriangles = 512,
    [switch]$CheckOnly
)

$ErrorActionPreference = "Stop"

function Resolve-BlenderExe {
    param([string]$Requested)

    if (![string]::IsNullOrWhiteSpace($env:BLENDER_EXE)) {
        if (Test-Path -LiteralPath $env:BLENDER_EXE -PathType Leaf) {
            return (Resolve-Path -LiteralPath $env:BLENDER_EXE).Path
        }
    }

    if (Test-Path -LiteralPath $Requested -PathType Leaf) {
        return (Resolve-Path -LiteralPath $Requested).Path
    }

    $command = Get-Command $Requested -ErrorAction SilentlyContinue
    if ($null -ne $command) {
        return $command.Source
    }

    $candidateRoots = @(
        (Join-Path $env:ProgramFiles "Blender Foundation"),
        (Join-Path $env:LOCALAPPDATA "Programs\Blender Foundation")
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) }

    $candidates = foreach ($candidateRoot in $candidateRoots) {
        Get-ChildItem -LiteralPath $candidateRoot -Directory -ErrorAction SilentlyContinue |
            ForEach-Object {
                $candidate = Join-Path $_.FullName "blender.exe"
                if (Test-Path -LiteralPath $candidate -PathType Leaf) {
                    Get-Item -LiteralPath $candidate
                }
            }
    }

    $bestCandidate = $candidates |
        Sort-Object -Property @{ Expression = { $_.Directory.Name }; Descending = $true }, LastWriteTime -Descending |
        Select-Object -First 1

    if ($null -ne $bestCandidate) {
        return $bestCandidate.FullName
    }

    return $null
}

$root = Resolve-Path (Join-Path $PSScriptRoot "..")
$manifestPath = Join-Path $root $Manifest
$scriptPath = Join-Path $root "tools/blender/normalize_true25d_assets.py"
$receiptPath = Join-Path $root $Receipt

if (!(Test-Path -LiteralPath $manifestPath)) {
    throw "Missing true 2.5D asset manifest: $manifestPath"
}
if (!(Test-Path -LiteralPath $scriptPath)) {
    throw "Missing Blender normalization script: $scriptPath"
}
if ($DecimationThresholdTriangles -lt 1) {
    throw "DecimationThresholdTriangles must be positive."
}

$blenderPath = Resolve-BlenderExe -Requested $BlenderExe
if ($null -eq $blenderPath) {
    Write-Output "USER_ACTION_REQUIRED: Blender is not available. Install Blender 4.x or newer, set BLENDER_EXE, or pass -BlenderExe."
    Write-Output "Expected command: blender --background --python `"$scriptPath`" -- --root `"$root`" --manifest `"$manifestPath`" --in-place --update-manifest --receipt `"$receiptPath`""
    if ($CheckOnly) {
        exit 0
    }
    exit 2
}

Write-Output "Blender found: $blenderPath"
Write-Output "Manifest: $manifestPath"
Write-Output "Receipt: $receiptPath"
Write-Output "Decimation threshold triangles: $DecimationThresholdTriangles"

if ($CheckOnly) {
    & $blenderPath --version
    exit $LASTEXITCODE
}

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $receiptPath) | Out-Null
& $blenderPath --background --python $scriptPath -- `
    --root $root `
    --manifest $manifestPath `
    --in-place `
    --update-manifest `
    --receipt $receiptPath `
    --decimation-threshold-triangles $DecimationThresholdTriangles
exit $LASTEXITCODE
