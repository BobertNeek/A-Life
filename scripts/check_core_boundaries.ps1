param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ScriptArgs
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$gpuBackendSource = Join-Path $repoRoot "crates/alife_gpu_backend/src"
$forbiddenClosedLoopSymbols = @("CpuNeuralState", "NeuralProjectionSchema")

if (Test-Path -LiteralPath $gpuBackendSource -PathType Container) {
    $closedLoopDirectory = Join-Path $gpuBackendSource "closed_loop_buffers"
    $closedLoopModules = @(
        Get-ChildItem -LiteralPath $gpuBackendSource -Filter "closed_loop_*.rs" -File
        if (Test-Path -LiteralPath $closedLoopDirectory -PathType Container) {
            Get-ChildItem -LiteralPath $closedLoopDirectory -Filter "*.rs" -File -Recurse
        }
    )

    foreach ($module in $closedLoopModules) {
        foreach ($symbol in $forbiddenClosedLoopSymbols) {
            $matches = Select-String `
                -LiteralPath $module.FullName `
                -Pattern "\b$([regex]::Escape($symbol))\b" `
                -CaseSensitive
            if ($matches) {
                $locations = ($matches | ForEach-Object { "$($_.Path):$($_.LineNumber)" }) -join ", "
                Write-Error "GPU closed-loop production modules must not reference $symbol ($locations)"
            }
        }
    }
}

$launcher = Join-Path $PSScriptRoot "run-git-bash.ps1"
$script = Join-Path $PSScriptRoot "check_core_boundaries.sh"

& powershell -NoProfile -ExecutionPolicy Bypass -File $launcher $script @ScriptArgs
$exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
exit $exitCode
