param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ScriptArgs
)

$ErrorActionPreference = "Stop"

$launcher = Join-Path $PSScriptRoot "run-git-bash.ps1"
$script = Join-Path $PSScriptRoot "docs_check.sh"

& powershell -NoProfile -ExecutionPolicy Bypass -File $launcher $script @ScriptArgs
$exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
exit $exitCode
