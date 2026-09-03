[CmdletBinding()]
param(
    [ValidateSet('0', '1', '2', '3')]
    [string]$Tier = '0',

    [string]$Receipt,

    [switch]$DryRun,
    [switch]$ResolveCacheKey,
    [switch]$List,
    [switch]$AllowDirty
)

$ErrorActionPreference = 'Stop'

Set-StrictMode -Version Latest

$TaskId = 'pass2-task0a-fast-gates-cache-harness'
$ReceiptSchema = 'alife.pass2.compact_receipt.v1'
$CacheVersion = 'v1'
$Root = Split-Path -Parent $PSScriptRoot

function Get-Sha256Text {
    param([Parameter(Mandatory = $true)][string]$Text)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        return ([System.BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    }
    finally {
        $sha.Dispose()
    }
}

function Get-NativeShell {
    if ($env:OS -eq 'Windows_NT') {
        if (Get-Command pwsh -ErrorAction SilentlyContinue) {
            return 'pwsh'
        }
        return 'powershell'
    }
    return 'bash'
}

function Get-GateCommands {
    $shell = Get-NativeShell
    if ($shell -eq 'bash') {
        return @(
            ,@('git', 'diff', '--check')
            ,@('bash', 'scripts/check_core_boundaries.sh', '--static')
            ,@('bash', 'scripts/docs_check.sh')
        )
    }

    return @(
        ,@('git', 'diff', '--check')
        ,@($shell, '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'scripts/check_core_boundaries.ps1', '--static')
        ,@($shell, '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'scripts/docs_check.ps1')
    )
}

function Get-TierDefinition {
    param([Parameter(Mandatory = $true)][string]$Id)

    $staticCommands = Get-GateCommands
    $hostSmoke = @(
        ,@(
            'cargo', 'test', '-p', 'alife_tools', '--test', 'benchmark_tiers',
            'benchmark_tiers_smoke_runs_tier_1_and_10_without_bevy_or_gpu'
        )
    )
    $ei1Plan = @(
        ,@(
            'cargo', 'run', '-p', 'alife_tools', '--bin', 'era1_promotion', '--',
            '--out', 'target/artifacts/pass2/ei1-promotion.json'
        )
    )
    $populationPlan = @(
        ,@(
            'cargo', 'run', '-p', 'alife_tools', '--bin', 'benchmark_tiers', '--',
            '--backend', 'gpu-closed-loop',
            '--targets', 'configs/gpu_closed_loop_performance_targets_v1.json',
            '--classes', 'n512,n1024,n2048',
            '--sensor-profiles', 'privileged-affordance-v1,grounded-object-slots-v1',
            '--populations', '1,10,30,50,100,250,500',
            '--base-seed', '4404',
            '--output', 'target/artifacts/pass2/population-matrix.json'
        )
    )

    switch ($Id) {
        '0' {
            return [pscustomobject][ordered]@{
                id = 0
                name = 'source-static'
                kind = 'runnable'
                commands = $staticCommands
                note = 'Source, boundary, and documentation checks. No Cargo or GPU.'
            }
        }
        '1' {
            return [pscustomobject][ordered]@{
                id = 1
                name = 'host-smoke'
                kind = 'runnable'
                commands = $hostSmoke
                note = 'Existing deterministic alife_tools smoke for populations 1 and 10.'
            }
        }
        '2' {
            return [pscustomobject][ordered]@{
                id = 2
                name = 'ei1-dormant'
                kind = 'dormant'
                commands = $ei1Plan
                note = 'Dormant EI1 promotion entry point. Dry-run only in this lane.'
            }
        }
        '3' {
            return [pscustomobject][ordered]@{
                id = 3
                name = 'population-dormant'
                kind = 'dormant'
                commands = $populationPlan
                note = 'Dormant population matrix entry point. Requires a later explicit GPU queue turn.'
            }
        }
        default {
            throw "unsupported tier: $Id"
        }
    }
}

function Invoke-GitRead {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $result = @(& git @Arguments 2>$null)
        $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    if ($exitCode -ne 0) {
        throw "git command failed: $($Arguments -join ' ')"
    }
    return $result
}

function Get-SourceIdentity {
    $commit = ((Invoke-GitRead -Arguments @('rev-parse', 'HEAD')) -join "`n").Trim()
    $tree = ((Invoke-GitRead -Arguments @('rev-parse', 'HEAD^{tree}')) -join "`n").Trim()
    $statusLines = @(Invoke-GitRead -Arguments @('status', '--porcelain=v1', '--untracked-files=all'))

    return [pscustomobject][ordered]@{
        commit = $commit
        tree = $tree
        dirty = $statusLines.Count -gt 0
        dirty_path_count = $statusLines.Count
    }
}

function Get-CacheKey {
    param(
        [Parameter(Mandatory = $true)]$Definition,
        [Parameter(Mandatory = $true)]$Source
    )

    $argv = @($Definition.commands | ForEach-Object { (@($_) -join [char]0) }) -join "`n"
    $material = @(
        "task_id=$TaskId"
        "receipt_schema=$ReceiptSchema"
        "cache_version=$CacheVersion"
        "tier=$($Definition.id)"
        "name=$($Definition.name)"
        "source_commit=$($Source.commit)"
        "source_tree=$($Source.tree)"
        "dirty=$($Source.dirty.ToString().ToLowerInvariant())"
        "commands=$argv"
    ) -join "`n"
    $digest = Get-Sha256Text -Text $material
    $cleanState = if ($Source.dirty) { 'dirty' } else { 'clean' }
    return "pass2-$TaskId-$CacheVersion-tier$($Definition.id)-$($Source.commit)-$($Source.tree)-$cleanState-$digest"
}

function Get-ReceiptPath {
    param([Parameter(Mandatory = $true)]$Definition)

    if ([string]::IsNullOrWhiteSpace($Receipt)) {
        return Join-Path $Root ("target/artifacts/pass2/tier{0}/compact-receipt.json" -f $Definition.id)
    }
    if ([System.IO.Path]::IsPathRooted($Receipt)) {
        return [System.IO.Path]::GetFullPath($Receipt)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $Root $Receipt))
}

function Get-RelativeRootPath {
    param([Parameter(Mandatory = $true)][string]$Path)

    $fullPath = [System.IO.Path]::GetFullPath($Path)
    $rootPath = [System.IO.Path]::GetFullPath($Root).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    if ($fullPath.StartsWith($rootPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        return $fullPath.Substring($rootPath.Length).Replace('\', '/')
    }
    return $fullPath.Replace('\', '/')
}

function Write-CompactReceipt {
    param(
        [Parameter(Mandatory = $true)]$ReceiptObject,
        [Parameter(Mandatory = $true)][string]$Path
    )

    $parent = Split-Path -Parent $Path
    [System.IO.Directory]::CreateDirectory($parent) | Out-Null
    $json = $ReceiptObject | ConvertTo-Json -Depth 10 -Compress
    $temporary = "$Path.tmp-$PID"
    [System.IO.File]::WriteAllText(
        $temporary,
        $json + [Environment]::NewLine,
        [System.Text.UTF8Encoding]::new($false)
    )
    Move-Item -LiteralPath $temporary -Destination $Path -Force
    return $json
}

function New-PlannedCommandReceipts {
    param([Parameter(Mandatory = $true)]$Definition)

    return @(
        foreach ($argv in $Definition.commands) {
            [pscustomobject][ordered]@{
                argv = @($argv)
                executed = $false
                exit_code = $null
                log = $null
            }
        }
    )
}

function Invoke-GateCommand {
    param(
        [Parameter(Mandatory = $true)][string[]]$Argv,
        [Parameter(Mandatory = $true)][string]$LogPath
    )

    $program = $Argv[0]
    $arguments = @($Argv | Select-Object -Skip 1)
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = if ($arguments.Count -eq 0) {
            @(& $program 2>&1)
        }
        else {
            @(& $program @arguments 2>&1)
        }
        $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
    }
    finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
    $lines = @($output | ForEach-Object { $_.ToString() })
    [System.IO.File]::WriteAllText(
        $LogPath,
        (($lines -join [Environment]::NewLine) + [Environment]::NewLine),
        [System.Text.UTF8Encoding]::new($false)
    )
    return $exitCode
}

function New-CompactReceipt {
    param(
        [Parameter(Mandatory = $true)]$Definition,
        [Parameter(Mandatory = $true)]$Source,
        [Parameter(Mandatory = $true)][string]$CacheKey,
        [Parameter(Mandatory = $true)][string]$Status,
        [Parameter(Mandatory = $true)][bool]$Executed,
        [Parameter(Mandatory = $true)][object[]]$Commands,
        [Parameter(Mandatory = $true)][string]$Reason
    )

    return [pscustomobject][ordered]@{
        schema = $ReceiptSchema
        task_id = $TaskId
        lane = 'C'
        tier = $Definition.id
        name = $Definition.name
        status = $Status
        executed = $Executed
        claimable = $Status -eq 'Pass' -and -not $Source.dirty
        gpu_evidence = $false
        source_commit = $Source.commit
        source_tree = $Source.tree
        dirty = $Source.dirty
        dirty_path_count = $Source.dirty_path_count
        cache_key = $CacheKey
        commands = $Commands
        reason = $Reason
    }
}

$actionCount = @($DryRun.IsPresent, $ResolveCacheKey.IsPresent, $List.IsPresent | Where-Object { $_ }).Count
if ($actionCount -gt 1) {
    throw 'choose only one of -DryRun, -ResolveCacheKey, or -List'
}

Push-Location $Root
$exitCode = 0
try {
    $definition = Get-TierDefinition -Id $Tier

    if ($List) {
        foreach ($tierId in @('0', '1', '2', '3')) {
            $listed = Get-TierDefinition -Id $tierId
            $commandText = @($listed.commands | ForEach-Object { (@($_) -join ' ') }) -join ' || '
            Write-Output ("Tier {0}: {1} [{2}]`n  {3}`n  {4}" -f $listed.id, $listed.name, $listed.kind, $commandText, $listed.note)
        }
        exit 0
    }

    $source = Get-SourceIdentity
    $cacheKey = Get-CacheKey -Definition $definition -Source $source

    if ($ResolveCacheKey) {
        Write-Output $cacheKey
        exit 0
    }

    $receiptPath = Get-ReceiptPath -Definition $definition
    $commandReceipts = New-PlannedCommandReceipts -Definition $definition

    if ($definition.kind -eq 'dormant' -and -not $DryRun) {
        $receiptObject = New-CompactReceipt `
            -Definition $definition `
            -Source $source `
            -CacheKey $cacheKey `
            -Status 'Unknown' `
            -Executed $false `
            -Commands $commandReceipts `
            -Reason 'Dormant entry point. Re-run only with -DryRun until the queue grants execution.'
        $json = Write-CompactReceipt -ReceiptObject $receiptObject -Path $receiptPath
        Write-Output $json
        exit 2
    }

    if (-not $DryRun -and $source.dirty -and -not $AllowDirty) {
        $receiptObject = New-CompactReceipt `
            -Definition $definition `
            -Source $source `
            -CacheKey $cacheKey `
            -Status 'Blocked' `
            -Executed $false `
            -Commands $commandReceipts `
            -Reason 'Execution requires a clean worktree. Use -AllowDirty only for local diagnostics.'
        $json = Write-CompactReceipt -ReceiptObject $receiptObject -Path $receiptPath
        Write-Output $json
        exit 1
    }

    if ($DryRun) {
        $receiptObject = New-CompactReceipt `
            -Definition $definition `
            -Source $source `
            -CacheKey $cacheKey `
            -Status 'Unknown' `
            -Executed $false `
            -Commands $commandReceipts `
            -Reason 'Dry-run only. No command was executed.'
        $json = Write-CompactReceipt -ReceiptObject $receiptObject -Path $receiptPath
        Write-Output $json
        exit 0
    }

    $logDirectory = Join-Path (Split-Path -Parent $receiptPath) 'logs'
    [System.IO.Directory]::CreateDirectory($logDirectory) | Out-Null
    $failed = $false
    for ($index = 0; $index -lt $commandReceipts.Count; $index++) {
        $logPath = Join-Path $logDirectory ("command-{0:D2}.log" -f ($index + 1))
        $result = Invoke-GateCommand -Argv $commandReceipts[$index].argv -LogPath $logPath
        $commandReceipts[$index].executed = $true
        $commandReceipts[$index].exit_code = $result
        $commandReceipts[$index].log = Get-RelativeRootPath -Path $logPath
        if ($result -ne 0) {
            $failed = $true
            break
        }
    }

    $status = if ($failed) { 'Blocked' } else { 'Pass' }
    $reason = if ($failed) { 'A fast-gate command returned a non-zero exit code.' } else { $definition.note }
    $receiptObject = New-CompactReceipt `
        -Definition $definition `
        -Source $source `
        -CacheKey $cacheKey `
        -Status $status `
        -Executed $true `
        -Commands $commandReceipts `
        -Reason $reason
    $json = Write-CompactReceipt -ReceiptObject $receiptObject -Path $receiptPath
    Write-Output $json
    if ($failed) {
        $exitCode = 1
    }
}
finally {
    Pop-Location
}

exit $exitCode
