param(
    [Parameter(Position = 0)]
    [string]$Command = "update",

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$CommandArgs
)

$ErrorActionPreference = "Stop"
$graphifyCommand = Get-Command graphify -CommandType Application -ErrorAction SilentlyContinue
$graphifyPath = if ($graphifyCommand) { $graphifyCommand.Source } else {
    Join-Path $env:USERPROFILE ".local/bin/graphify.exe"
}

if (-not (Test-Path -LiteralPath $graphifyPath -PathType Leaf)) {
    Write-Error "Graphify is not installed. Run: uv tool install graphifyy"
    exit 127
}

Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    switch ($Command) {
        "update" { & $graphifyPath update . @CommandArgs }
        "extract" { & $graphifyPath extract . @CommandArgs }
        default { & $graphifyPath $Command @CommandArgs }
    }
    $graphifyExitCode = $LASTEXITCODE
} finally {
    Pop-Location
}
exit $graphifyExitCode
