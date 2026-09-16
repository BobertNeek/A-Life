$ErrorActionPreference = 'Stop'

$sourcePath = Join-Path $PSScriptRoot 'pass2_ei1_ablation.ps1'
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $sourcePath,
    [ref]$tokens,
    [ref]$parseErrors
)
if ($parseErrors.Count -ne 0) { throw "planner script does not parse: $parseErrors" }

$functionAst = $ast.Find(
    {
        param($node)
        $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
            $node.Name -eq 'Write-Utf8JsonNoBom'
    },
    $true
)
if ($null -eq $functionAst) { throw 'Write-Utf8JsonNoBom is missing' }
. ([scriptblock]::Create($functionAst.Extent.Text))

$testPath = [System.IO.Path]::GetTempFileName()
try {
    $value = [ordered]@{
        schema_version = 'transport-test-v1'
        nested = [ordered]@{ value = 1 }
    }
    Write-Utf8JsonNoBom -Path $testPath -Value $value -Depth 4

    $bytes = [System.IO.File]::ReadAllBytes($testPath)
    if ($bytes.Count -eq 0 -or $bytes[0] -ne [byte][char]'{') {
        throw 'planned manifest is empty or starts with a UTF-8 BOM'
    }
    $parsed = Get-Content -Raw -LiteralPath $testPath | ConvertFrom-Json
    if ($parsed.schema_version -ne 'transport-test-v1' -or $parsed.nested.value -ne 1) {
        throw 'planned manifest did not round-trip as JSON'
    }
}
finally {
    Remove-Item -LiteralPath $testPath -Force -ErrorAction SilentlyContinue
}

'PASS pass2 EI1 planned-manifest UTF-8 transport'
