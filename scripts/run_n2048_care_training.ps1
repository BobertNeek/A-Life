#requires -Version 7.0
[CmdletBinding()]
param([ValidateSet('Prepare', 'Status', 'Campaign')][string]$Mode = 'Status')
$PilotTicks = 32
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$gitRoot = & git -C $repo rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0 -or [IO.Path]::GetFullPath($gitRoot) -ne $repo -or (Get-Location).Path -ne $repo) { throw 'Run from the repository root containing this script.' }
$base = Join-Path $repo 'target/founder-training'
$latest = Join-Path $base 'status.json'
if ($Mode -eq 'Campaign') { throw 'Campaign is not implemented. The executable supports only --pilot; no eight-hour run or resume is available.' }
if ($Mode -eq 'Status') {
    if (Test-Path -LiteralPath $latest) { Get-Content -Raw -LiteralPath $latest } else { Write-Output 'No preparation receipt exists.' }
    return
}
[IO.Directory]::CreateDirectory($base) | Out-Null
$lock = $null; $state = $null; $run = $null
function Assert-Idle {
    $busy = @(Get-Process | Where-Object { $_.ProcessName -match '^(cargo|rustc|alife_.*|train_n2048_care|wgsl_foundation_trainer.*|n2048_foundation_abi.*)$' })
    if ($busy.Count) { throw "Cargo/game/GPU lane is occupied: $($busy.ProcessName -join ', '). Nothing was stopped." }
}
function Atomic-Json($Path, $Value) {
    $temporary = "$Path.$([Guid]::NewGuid().ToString('N')).tmp"
    [IO.File]::WriteAllText($temporary, ($Value | ConvertTo-Json -Depth 12), [Text.UTF8Encoding]::new($false))
    [IO.File]::Move($temporary, $Path, $true)
}
function Publish-State {
    $state.updatedUtc = [DateTime]::UtcNow.ToString('o')
    Atomic-Json (Join-Path $run 'status.json') $state
    Atomic-Json $latest $state
}
function Run-Command([string]$Program, [string[]]$Arguments, [string]$Log, [switch]$OneTest) {
    $info = [Diagnostics.ProcessStartInfo]::new($Program)
    $info.WorkingDirectory = $repo; $info.UseShellExecute = $false; $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true; $info.RedirectStandardError = $true
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $info.Environment['CARGO_BUILD_JOBS'] = '2'
    $info.Environment['CARGO_NET_OFFLINE'] = 'true'; $info.Environment['CARGO_TARGET_DIR'] = (Join-Path $repo 'target')
    $stdout = [IO.File]::Open("$Log.stdout.log", 'CreateNew', 'Write', 'Read')
    $stderr = [IO.File]::Open("$Log.stderr.log", 'CreateNew', 'Write', 'Read')
    $process = [Diagnostics.Process]::new(); $process.StartInfo = $info
    try {
        if (-not $process.Start()) { throw "Failed to start $Program" }
        $outTask = $process.StandardOutput.BaseStream.CopyToAsync($stdout)
        $errTask = $process.StandardError.BaseStream.CopyToAsync($stderr)
        $process.WaitForExit(); [Threading.Tasks.Task]::WaitAll(@($outTask, $errTask))
        if ($process.ExitCode -ne 0) { throw "Exit $($process.ExitCode): $Program $($Arguments -join ' '); see $Log.*.log" }
    } finally { $stdout.Dispose(); $stderr.Dispose(); $process.Dispose() }
    if ($OneTest -and [IO.File]::ReadAllText("$Log.stdout.log") -notmatch 'test result: ok\. 1 passed; 0 failed;') { throw "Expected exactly one passing test: $Log.stdout.log" }
}
function Source-Receipt([string]$Directory) {
    [IO.Directory]::CreateDirectory($Directory) | Out-Null
    Run-Command 'git' @('rev-parse', 'HEAD') (Join-Path $Directory 'head')
    Run-Command 'git' @('diff', '--binary', '--no-ext-diff', '--no-textconv', 'HEAD') (Join-Path $Directory 'tracked-diff')
    Run-Command 'git' @('-c', 'core.quotepath=false', 'ls-files', '--others', '--exclude-standard', '--', 'crates', 'scripts', 'docs', 'assets', '.cargo', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml') (Join-Path $Directory 'untracked')
    $files = @([IO.File]::ReadAllLines((Join-Path $Directory 'untracked.stdout.log')) | Sort-Object | Where-Object {
        $_ -and $_ -notmatch '(^|/)(target|artifacts|__pycache__)/|\.blend[0-9]+$|\.pyc$'
    } | ForEach-Object { [ordered]@{ path = $_; sha256 = (Get-FileHash -LiteralPath (Join-Path $repo $_) -Algorithm SHA256).Hash } })
    $identity = [ordered]@{ head = [IO.File]::ReadAllText((Join-Path $Directory 'head.stdout.log')).Trim(); trackedDiffSha256 = (Get-FileHash (Join-Path $Directory 'tracked-diff.stdout.log')).Hash; untracked = $files; pilotTicks = $PilotTicks; cargo = $cargoVersion; rustc = $rustcVersion; profile = 'dev'; features = 'foundation-training' }
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $fingerprint = [BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes(($identity | ConvertTo-Json -Depth 8 -Compress)))).Replace('-', '') } finally { $sha.Dispose() }
    $receipt = [ordered]@{ fingerprint = $fingerprint; identity = $identity }
    Atomic-Json (Join-Path $Directory 'source.json') $receipt
    return $receipt
}
try {
    $lock = [IO.File]::Open((Join-Path $base 'operator.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
    Assert-Idle
    $cargoVersion = (& cargo --version) -join "`n"; if ($LASTEXITCODE) { throw 'Cargo unavailable.' }
    $rustcVersion = (& rustc --version) -join "`n"; if ($LASTEXITCODE) { throw 'Rust compiler unavailable.' }
    $run = Join-Path $base ("prepare-{0}-{1}" -f [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss'), [Guid]::NewGuid().ToString('N').Substring(0, 8))
    [IO.Directory]::CreateDirectory($run) | Out-Null
    $source = Source-Receipt (Join-Path $run 'source')
    if (Test-Path -LiteralPath $latest) {
        $previous = Get-Content -Raw -LiteralPath $latest | ConvertFrom-Json
        if ($previous.fingerprint -eq $source.fingerprint) {
            if ($previous.state -eq 'Failed') { throw 'This exact source already failed. Stop for supervisor diagnosis; do not retry or alter thresholds.' }
            if ($previous.state -eq 'Running') { throw 'Previous run ended without a terminal receipt. Stop for supervisor diagnosis.' }
            if ($previous.state -eq 'Prepared' -and (Test-Path -LiteralPath $previous.executable) -and (Test-Path -LiteralPath $previous.pilotReceipt)) {
                if ((Get-FileHash $previous.executable).Hash -eq $previous.executableSha256 -and (Get-FileHash $previous.pilotReceipt).Hash -eq $previous.pilotReceiptSha256) { Write-Output "Already prepared: $($previous.run). No gates repeated."; return }
            }
        }
    }
    $state = [ordered]@{ schema = 1; state = 'Running'; phase = 'gates'; pid = $PID; run = $run; fingerprint = $source.fingerprint; updatedUtc = ''; completed = @(); error = $null }
    Publish-State
    $gates = @(
        @('asset', '-p', 'alife_core', '--test', 'n2048_foundation_abi', 'explicit_n2048_candidate_preserves_exact_weights_and_compiler_resume_identity'),
        @('sampling', '-p', 'alife_gpu_backend', '--features', 'training-rollout', '--lib', 'training_rollout::tests::training_policy_shader_and_probability_contract'),
        @('trainer-gpu', '-p', 'alife_training', '--features', 'gpu-tests', '--test', 'wgsl_foundation_trainer', 'n2048_exact_graph_adamw_step_changes_only_the_masked_weight_and_exports'),
        @('ppo', '-p', 'alife_training', '--lib', 'ppo::tests::ppo_rollout_keeps_bootstrap_boundaries_masks_and_policy_version'),
        @('ppo-gpu', '-p', 'alife_training', '--features', 'gpu-tests', '--test', 'wgsl_foundation_trainer', 'n2048_ppo_and_imitation_gpu_objective_matches_joint_derivatives_and_partial_batches'),
        @('merge', '-p', 'alife_training', '--lib', 'founder_merge::tests::selective_merge_preserves_unique_changes_averages_overlap_and_rejects_conflicts'),
        @('save', '-p', 'alife_game_app', '--features', 'foundation-training', '--lib', 'new_game_lifecycle::tests::n2048_new_game_save_reload_preserves_exact_candidate_and_class')
    )
    $steps = @([pscustomobject]@{ name = 'build'; argv = @('build', '--offline', '--locked', '-j', '2', '-p', 'alife_game_app', '--features', 'foundation-training', '--bin', 'train_n2048_care'); test = $false })
    $steps += @($gates | ForEach-Object { [pscustomobject]@{ name = $_[0]; argv = @('test', '--offline', '--locked', '-j', '2') + $_[1..($_.Count - 1)] + @('--', '--exact', '--test-threads=1'); test = $true } })
    foreach ($step in $steps) {
        Assert-Idle; $state.phase = $step.name; Publish-State; Write-Host "Running $($step.name); logs: $run"
        if ((Source-Receipt (Join-Path $run "source-$($step.name)")).fingerprint -ne $source.fingerprint) { throw 'Source changed during preparation.' }
        Run-Command 'cargo' $step.argv (Join-Path $run $step.name) -OneTest:$step.test
        $state.completed += [ordered]@{ phase = $step.name; argv = $step.argv }; Publish-State
    }
    Assert-Idle
    if ((Source-Receipt (Join-Path $run 'source-pilot')).fingerprint -ne $source.fingerprint) { throw 'Source changed before pilot.' }
    $exe = Join-Path $repo 'target/debug/train_n2048_care.exe'
    $state.executable = $exe; $state.executableSha256 = (Get-FileHash $exe).Hash
    $state.phase = 'pilot'; Publish-State
    $pilot = Join-Path $run 'pilot' # The executable requires this path NOT to exist.
    Run-Command $exe @('--pilot', $pilot, "$PilotTicks") (Join-Path $run 'pilot')
    $receiptPath = Join-Path $pilot 'pilot.json'; $receipt = Get-Content -Raw -LiteralPath $receiptPath | ConvertFrom-Json
    foreach ($field in @('collection_seconds', 'replay_seconds', 'maximum_logit_error', 'maximum_activation_error', 'maximum_activity_ema_error', 'maximum_metabolic_error')) {
        $value = $receipt.$field
        if ($null -eq $value -or $value -is [string] -or $value -is [bool] -or [double]::IsNaN([double]$value) -or [double]::IsInfinity([double]$value) -or [double]$value -lt 0) { throw "Invalid pilot metric: $field" }
    }
    if ($receipt.ticks -ne $PilotTicks -or $receipt.seed -ne 0x20260921 -or $receipt.source_asset_digest -notmatch '^[0-9a-f]{64}$') { throw 'Pilot identity mismatch.' }
    if ((Source-Receipt (Join-Path $run 'source-final')).fingerprint -ne $source.fingerprint -or (Get-FileHash $exe).Hash -ne $state.executableSha256) { throw 'Source or executable changed during pilot.' }
    $state.pilotReceipt = $receiptPath; $state.pilotReceiptSha256 = (Get-FileHash $receiptPath).Hash
    $state.state = 'Prepared'; $state.phase = 'pilot-verified'; Publish-State
    Write-Output "Pilot verified: $receiptPath. Campaign and resumable training checkpoints are not implemented."
} catch {
    if ($null -ne $state) { $state.state = 'Failed'; $state.error = $_.Exception.Message; Publish-State }
    throw
} finally { if ($null -ne $lock) { $lock.Dispose() } }
