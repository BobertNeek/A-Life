param(
    [string]$LlamaServerPath = "",
    [string]$ModelPath = "",
    [ValidatePattern('^[A-Za-z0-9_.-]+$')]
    [string]$ModelAlias = "alife-qwen3.5-0.8b-prior",
    [ValidateRange(1, 65535)]
    [int]$Port = 18081,
    [int]$ContextSize = 2048,
    [int]$GpuLayers = 999,
    [ValidateRange(0, 128)]
    [int]$Threads = 4,
    [ValidateRange(1, 16)]
    [int]$ParallelSlots = 1,
    [switch]$PrintOnly
)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($ModelPath)) {
    $ModelPath = Join-Path $repoRoot "models\local\qwen3.5-0.8b-gguf\Qwen3.5-0.8B-Q8_0.gguf"
}

function Resolve-LlamaServer {
    param([string]$ExplicitPath)

    if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
        if (-not (Test-Path -LiteralPath $ExplicitPath -PathType Leaf)) {
            throw "USER_ACTION_REQUIRED: explicit llama-server.exe was not found: $ExplicitPath"
        }
        $resolved = (Resolve-Path -LiteralPath $ExplicitPath).Path
        if ($resolved -match "\\Ollama\\") {
            throw "USER_ACTION_REQUIRED: Ollama-bundled llama-server.exe is intentionally rejected."
        }
        return $resolved
    }

    $pathCommand = Get-Command llama-server.exe -ErrorAction SilentlyContinue
    if ($pathCommand -and $pathCommand.Source -notmatch "\\Ollama\\") {
        return (Resolve-Path -LiteralPath $pathCommand.Source).Path
    }

    $candidates = @()
    $wingetRoot = Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Packages"
    if (Test-Path -LiteralPath $wingetRoot -PathType Container) {
        $candidates += Get-ChildItem -LiteralPath $wingetRoot -Recurse -Filter llama-server.exe -File -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -like "*ggml.llamacpp*" } |
            Select-Object -ExpandProperty FullName
    }

    foreach ($candidate in $candidates | Select-Object -Unique) {
        if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
            continue
        }
        if ($candidate -match "\\Ollama\\") {
            continue
        }
        return (Resolve-Path -LiteralPath $candidate).Path
    }

    throw "USER_ACTION_REQUIRED: llama-server.exe from llama.cpp was not found. Install ggml.llamacpp or pass -LlamaServerPath. Ollama-bundled llama-server.exe is intentionally rejected."
}

$server = Resolve-LlamaServer -ExplicitPath $LlamaServerPath
if (-not (Test-Path -LiteralPath $ModelPath -PathType Leaf)) {
    throw "USER_ACTION_REQUIRED: local GGUF SLM prior model file not found: $ModelPath"
}
$model = (Resolve-Path -LiteralPath $ModelPath).Path
$arguments = @(
    "-m", $model,
    "--host", "127.0.0.1",
    "--port", "$Port",
    "--alias", $ModelAlias,
    "-c", "$ContextSize",
    "--reasoning", "off",
    # Parse the template's empty think tags separately so JSON grammar stays valid.
    "--reasoning-format", "deepseek",
    "--reasoning-budget", "0",
    "--n-gpu-layers", "$GpuLayers",
    "--parallel", "$ParallelSlots"
)
if ($Threads -gt 0) {
    $arguments += @("--threads", "$Threads", "--threads-batch", "$Threads")
}

function Format-CommandArgument {
    param([string]$Value)
    if ($Value -match '\s') {
        return '"' + $Value.Replace('"', '\"') + '"'
    }
    return $Value
}

Write-Host "Starting A-Life llama.cpp SLM prior on 127.0.0.1:$Port"
Write-Host "Command: `"$server`" $(($arguments | ForEach-Object { Format-CommandArgument $_ }) -join ' ')"

if ($PrintOnly) {
    exit 0
}

& $server @arguments
$serverExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
exit $serverExitCode
