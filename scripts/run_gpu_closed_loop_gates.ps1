[CmdletBinding()]
param(
    [string]$Output = 'target/artifacts/gpu-closed-loop-gates.json',
    [string]$AdapterEvidence = 'target/artifacts/gpu-closed-loop-slice-a-n512.json'
)

$ErrorActionPreference = 'Stop'
$env:CARGO_INCREMENTAL = '0'
$env:CARGO_BUILD_JOBS = '1'

function Get-MonotonicNanoseconds {
    $ticks = [System.Diagnostics.Stopwatch]::GetTimestamp()
    return [uint64]([decimal]$ticks * 1000000000 / [System.Diagnostics.Stopwatch]::Frequency)
}

function Convert-ArgvToBytes {
    param([string[]]$Argv)
    $joined = [string]::Join([char]0, $Argv)
    return @([System.Text.Encoding]::UTF8.GetBytes($joined) | ForEach-Object { [int]$_ })
}

function ConvertTo-WindowsCommandLineArgument {
    param([string]$Argument)
    if ($Argument.Length -gt 0 -and $Argument -notmatch '[\s"]') {
        return $Argument
    }
    $builder = [System.Text.StringBuilder]::new()
    [void]$builder.Append('"')
    $backslashes = 0
    foreach ($character in $Argument.ToCharArray()) {
        if ($character -eq '\') {
            $backslashes++
        }
        elseif ($character -eq '"') {
            if ($backslashes -gt 0) { [void]$builder.Append(('\' * ($backslashes * 2))) }
            [void]$builder.Append('\"')
            $backslashes = 0
        }
        else {
            if ($backslashes -gt 0) { [void]$builder.Append(('\' * $backslashes)) }
            [void]$builder.Append($character)
            $backslashes = 0
        }
    }
    if ($backslashes -gt 0) { [void]$builder.Append(('\' * ($backslashes * 2))) }
    [void]$builder.Append('"')
    return $builder.ToString()
}

function Invoke-CapturedCommand {
    param(
        [pscustomobject]$Spec,
        [string]$StdoutPath,
        [string]$StderrPath
    )
    $start = Get-MonotonicNanoseconds
    $info = [System.Diagnostics.ProcessStartInfo]::new()
    $info.FileName = $Spec.Argv[0]
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.Arguments = (($Spec.Argv[1..($Spec.Argv.Count - 1)] | ForEach-Object {
        ConvertTo-WindowsCommandLineArgument -Argument $_
    }) -join ' ')
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $info
    $stdout = [System.IO.File]::Open($StdoutPath, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::Read)
    $stderr = [System.IO.File]::Open($StderrPath, [System.IO.FileMode]::CreateNew, [System.IO.FileAccess]::Write, [System.IO.FileShare]::Read)
    try {
        if (-not $process.Start()) { throw "failed to start $($Spec.Label)" }
        $stdoutCopy = $process.StandardOutput.BaseStream.CopyToAsync($stdout)
        $stderrCopy = $process.StandardError.BaseStream.CopyToAsync($stderr)
        $process.WaitForExit()
        [System.Threading.Tasks.Task]::WaitAll(@($stdoutCopy, $stderrCopy))
        $exitCode = $process.ExitCode
    }
    finally {
        $stdout.Dispose()
        $stderr.Dispose()
        $process.Dispose()
    }
    $end = Get-MonotonicNanoseconds
    if ($end -le $start) { $end = $start + 1 }
    return [pscustomobject][ordered]@{
        command_id = [int]$Spec.Id
        argv_utf8 = Convert-ArgvToBytes -Argv $Spec.Argv
        started_monotonic_ns = $start
        ended_monotonic_ns = $end
        exit_code = [int]$exitCode
        stdout_path = $StdoutPath
        stderr_path = $StderrPath
    }
}

function Invoke-AuthorityScan {
    param(
        [pscustomobject]$Spec,
        [string]$StdoutPath,
        [string]$StderrPath
    )
    $start = Get-MonotonicNanoseconds
    $terms = @(
        ('cpu' + '[-_ ]?' + 'shadow'), ('AutoWith' + 'CpuFallback'),
        ('Cpu' + '[-_ ]?' + 'Reference'),
        ('neural' + '[-_ ]?' + 'fallback'),
        ('FullGpu' + 'RuntimeMode'),
        ('parity' + '[-_ ]?' + 'gat(?:e|ed|ing)')
    )
    $pattern = $terms -join '|'
    $raw = @(& rg -n -i $pattern crates/alife_core/src crates/alife_gpu_backend/src crates/alife_world/src crates/alife_game_app/src crates/alife_tools/src scripts 2>&1)
    $rgExit = $LASTEXITCODE
    if ($rgExit -gt 1) {
        [System.IO.File]::WriteAllText($StderrPath, (($raw -join "`n") + "`n"), [System.Text.UTF8Encoding]::new($false))
        [System.IO.File]::WriteAllText($StdoutPath, '', [System.Text.UTF8Encoding]::new($false))
        $exitCode = $rgExit
    }
    else {
        $bad = @($raw | Where-Object { $_ -notmatch 'crates[\\/]alife_world[\\/]src[\\/]legacy_neural_policy_v1.rs:' })
        if ($bad.Count -ne 0) {
            [System.IO.File]::WriteAllText($StderrPath, (($bad -join "`n") + "`n"), [System.Text.UTF8Encoding]::new($false))
            [System.IO.File]::WriteAllText($StdoutPath, '', [System.Text.UTF8Encoding]::new($false))
            $exitCode = 1
        }
        else {
            [System.IO.File]::WriteAllText($StdoutPath, "BROAD_AUTHORITY_SCAN_ZERO_MATCHES`n", [System.Text.UTF8Encoding]::new($false))
            [System.IO.File]::WriteAllText($StderrPath, '', [System.Text.UTF8Encoding]::new($false))
            $exitCode = 0
        }
    }
    $end = Get-MonotonicNanoseconds
    if ($end -le $start) { $end = $start + 1 }
    return [pscustomobject][ordered]@{
        command_id = [int]$Spec.Id
        argv_utf8 = Convert-ArgvToBytes -Argv $Spec.Argv
        started_monotonic_ns = $start
        ended_monotonic_ns = $end
        exit_code = [int]$exitCode
        stdout_path = $StdoutPath
        stderr_path = $StderrPath
    }
}

$commands = @(
    [pscustomobject]@{ Id = 1; Label = '01-fmt'; Argv = @('cargo', 'fmt', '--all', '--', '--check') },
    [pscustomobject]@{ Id = 2; Label = '02-check'; Argv = @('cargo', 'check', '--workspace', '--all-targets', '--all-features', '-j', '1') },
    [pscustomobject]@{ Id = 3; Label = '03-workspace-tests'; Argv = @('cargo', 'test', '--workspace', '--all-features', '-j', '1') },
    [pscustomobject]@{ Id = 4; Label = '04-core-brain'; Argv = @('cargo', 'test', '-p', 'alife_core', '--test', 'production_brain_budgets', '--test', 'phenotype_compiler', '--test', 'brain_topology') },
    [pscustomobject]@{ Id = 5; Label = '05-gpu-brain'; Argv = @('cargo', 'test', '-p', 'alife_gpu_backend', '--features', 'gpu-tests', '--test', 'closed_loop_runtime', '--test', 'closed_loop_admission', '--test', 'closed_loop_activity', '--test', 'closed_loop_gpu_behavior', '--test', 'closed_loop_eligibility', '--test', 'closed_loop_fast_plasticity', '--test', 'closed_loop_sleep', '--test', 'closed_loop_memory_context', '--', '--nocapture') },
    [pscustomobject]@{ Id = 6; Label = '06-world-save'; Argv = @('cargo', 'test', '-p', 'alife_world', '--test', 'gpu_brain_persistence', '--test', 'gpu_brain_vnext_migration', '--test', 'gpu_memory_grounding_persistence') },
    [pscustomobject]@{ Id = 7; Label = '07-app-brain'; Argv = @('cargo', 'test', '-p', 'alife_game_app', '--features', 'gpu-runtime gpu-tests', '--test', 'gpu_closed_loop_acceptance', '--test', 'gpu_learning_sleep_acceptance', '--test', 'gpu_memory_grounding_acceptance', '--test', 'gpu_sleep_restore', '--test', 'gpu_closed_loop_soak', '--test', 'gpu_brain_authority_audit', '--test', 'gpu_closed_loop_promotion', '-j', '1', '--', '--nocapture') },
    [pscustomobject]@{ Id = 8; Label = '08-tools-benchmark'; Argv = @('cargo', 'test', '-p', 'alife_tools', '--test', 'benchmark_tiers') },
    [pscustomobject]@{ Id = 9; Label = '09-docs'; Argv = @('powershell', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'scripts/docs_check.ps1') },
    [pscustomobject]@{ Id = 10; Label = '10-boundaries'; Argv = @('powershell', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', 'scripts/check_core_boundaries.ps1') },
    [pscustomobject]@{ Id = 11; Label = '11-authority-scan'; Argv = @('internal', 'authority-scan-v1') },
    [pscustomobject]@{ Id = 12; Label = '12-diff'; Argv = @('git', 'diff', '--check', 'origin/main...HEAD') }
)

$outputFull = [System.IO.Path]::GetFullPath($Output)
$outputParent = [System.IO.Path]::GetDirectoryName($outputFull)
[System.IO.Directory]::CreateDirectory($outputParent) | Out-Null
$staging = [System.IO.Path]::GetFullPath((Join-Path $outputParent '.gpu-closed-loop-gates.staging'))
$parentPrefix = $outputParent.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
if (-not $staging.StartsWith($parentPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'gate staging path escaped the artifact directory'
}
if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
if (Test-Path -LiteralPath $outputFull) { Remove-Item -LiteralPath $outputFull -Force }
[System.IO.Directory]::CreateDirectory($staging) | Out-Null

$published = $false
try {
    if (& git status --porcelain=v1) { throw 'gate run requires a clean worktree' }
    $head = (& git rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'failed to resolve HEAD' }
    $tree = (& git rev-parse 'HEAD^{tree}').Trim()
    if ($LASTEXITCODE -ne 0) { throw 'failed to resolve HEAD tree' }

    $captures = @()
    foreach ($spec in $commands) {
        $stdoutPath = Join-Path $staging ("{0:D2}.stdout" -f $spec.Id)
        $stderrPath = Join-Path $staging ("{0:D2}.stderr" -f $spec.Id)
        $capture = if ($spec.Id -eq 11) {
            Invoke-AuthorityScan -Spec $spec -StdoutPath $stdoutPath -StderrPath $stderrPath
        }
        else {
            Invoke-CapturedCommand -Spec $spec -StdoutPath $stdoutPath -StderrPath $stderrPath
        }
        $captures += $capture
        if ($capture.exit_code -ne 0) {
            Get-Content -Raw -LiteralPath $stdoutPath -ErrorAction SilentlyContinue | Write-Host
            Get-Content -Raw -LiteralPath $stderrPath -ErrorAction SilentlyContinue | Write-Error
            throw "$($spec.Label) failed with exit $($capture.exit_code)"
        }
    }

    $headAfter = (& git rev-parse HEAD).Trim()
    $treeAfter = (& git rev-parse 'HEAD^{tree}').Trim()
    if ($headAfter -ne $head -or $treeAfter -ne $tree -or (& git status --porcelain=v1)) {
        throw 'source commit, tree, or cleanliness changed during the gate run'
    }

    $captureManifest = [pscustomobject][ordered]@{
        schema_version = 1
        git_commit = $head
        source_tree_digest = $tree
        commands = $captures
    }
    $capturePath = Join-Path $staging 'capture.json'
    $captureJson = $captureManifest | ConvertTo-Json -Depth 8
    [System.IO.File]::WriteAllText($capturePath, $captureJson, [System.Text.UTF8Encoding]::new($false))

    & cargo run -p alife_game_app --features gpu-runtime --bin alife_game_app -- gpu-closed-loop-gate-seal --capture $capturePath --gate-script $PSCommandPath --adapter-evidence $AdapterEvidence --output $outputFull
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $outputFull)) {
        throw 'Rust gate receipt publication failed'
    }
    if ((& git rev-parse HEAD).Trim() -ne $head -or (& git rev-parse 'HEAD^{tree}').Trim() -ne $tree -or (& git status --porcelain=v1)) {
        throw 'source identity changed during gate receipt publication'
    }
    $published = $true
}
finally {
    if (-not $published -and (Test-Path -LiteralPath $outputFull)) {
        Remove-Item -LiteralPath $outputFull -Force
    }
    if (Test-Path -LiteralPath $staging) {
        Remove-Item -LiteralPath $staging -Recurse -Force
    }
}
