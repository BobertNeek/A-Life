param(
    [string]$BlenderExe = "blender",
    [string]$Manifest = "crates/alife_game_app/assets/alpha_art_v1/blender_pipeline_manifest.json",
    [string]$OutputDir = "target/generated_art/alpha_blender_v1",
    [switch]$CheckOnly
)

$ErrorActionPreference = "Stop"

function Resolve-BlenderExe {
    param([string]$Requested)

    if (![string]::IsNullOrWhiteSpace($env:BLENDER_EXE)) {
        if (Test-Path -LiteralPath $env:BLENDER_EXE) {
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
$scriptPath = Join-Path $root "tools/blender/render_alpha_art_v1.py"
$outputPath = Join-Path $root $OutputDir

if (!(Test-Path -LiteralPath $manifestPath)) {
    throw "Missing Blender asset pipeline manifest: $manifestPath"
}
if (!(Test-Path -LiteralPath $scriptPath)) {
    throw "Missing Blender render script: $scriptPath"
}

$blenderPath = Resolve-BlenderExe -Requested $BlenderExe
if ($null -eq $blenderPath) {
    Write-Output "USER_ACTION_REQUIRED: Blender is not available. Install Blender 4.x or newer, set BLENDER_EXE, or pass -BlenderExe."
    Write-Output "Expected command: blender --background --python `"$scriptPath`" -- --manifest `"$manifestPath`" --out-dir `"$outputPath`""
    if ($CheckOnly) {
        exit 0
    }
    exit 2
}

Write-Output "Blender found: $blenderPath"
Write-Output "Manifest: $manifestPath"
Write-Output "Output: $outputPath"

if ($CheckOnly) {
    & $blenderPath --version
    exit $LASTEXITCODE
}

New-Item -ItemType Directory -Force -Path $outputPath | Out-Null
& $blenderPath --background --python $scriptPath -- --manifest $manifestPath --out-dir $outputPath
exit $LASTEXITCODE
