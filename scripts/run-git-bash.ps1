param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$ScriptPath,

    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ScriptArgs
)

$ErrorActionPreference = "Stop"

function Normalize-Candidate {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path)) {
        return $null
    }
    return $Path.Trim().Trim('"')
}

$candidates = @(
    (Normalize-Candidate $env:GIT_BASH),
    "C:\Program Files\Git\bin\bash.exe",
    "C:\Program Files\Git\usr\bin\bash.exe",
    "C:\Program Files (x86)\Git\bin\bash.exe",
    "C:\Program Files (x86)\Git\usr\bin\bash.exe"
) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }

$gitBash = $null
foreach ($candidate in $candidates) {
    if (Test-Path -LiteralPath $candidate -PathType Leaf) {
        $gitBash = $candidate
        break
    }
}

if ($null -eq $gitBash) {
    Write-Error "Git Bash was not found. Set GIT_BASH or install Git for Windows."
    exit 127
}

if (-not (Test-Path -LiteralPath $ScriptPath -PathType Leaf)) {
    Write-Error "Requested script path does not exist: $ScriptPath"
    exit 2
}

$resolvedScript = (Resolve-Path -LiteralPath $ScriptPath).Path
Write-Host "Using Git Bash: $gitBash"

& $gitBash $resolvedScript @ScriptArgs
$exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
exit $exitCode
