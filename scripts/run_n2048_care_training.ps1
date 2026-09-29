#requires -Version 7.0
[CmdletBinding()]
param(
    [ValidateSet('Prepare', 'Status', 'Campaign')][string]$Mode = 'Status',
    [string]$Source,
    [ValidateRange(1, 24)][int]$DurationHours = 8,
    [ValidateRange(16, 4096)][int]$CycleTicks = 256,
    [ValidateSet('Care','VisionLanguage')][string]$Curriculum = 'VisionLanguage',
    [ValidateRange(1, 100000)][int]$MaxCycles = 100000,
    [ValidateRange(10, 1000)][int]$GateEveryCycles = 40
)
$PilotTicks = 32
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$gitRoot = & git -C $repo rev-parse --show-toplevel
if ($LASTEXITCODE -ne 0 -or [IO.Path]::GetFullPath($gitRoot) -ne $repo -or (Get-Location).Path -ne $repo) { throw 'Run from the repository root containing this script.' }
$base = Join-Path $repo 'target/founder-training'
$latest = Join-Path $base 'status.json'
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
function Run-Command([string]$Program, [string[]]$Arguments, [string]$Log, [switch]$OneTest, [datetime]$DeadlineUtc = [datetime]::MaxValue) {
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
        while (-not $process.WaitForExit(1000)) {
            if ([DateTime]::UtcNow -ge $DeadlineUtc) {
                $process.Kill($true)
                $process.WaitForExit()
                throw "Campaign deadline reached while running $Program; child process stopped."
            }
        }
        [Threading.Tasks.Task]::WaitAll(@($outTask, $errTask))
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
    $identity = [ordered]@{ head = [IO.File]::ReadAllText((Join-Path $Directory 'head.stdout.log')).Trim(); trackedDiffSha256 = (Get-FileHash (Join-Path $Directory 'tracked-diff.stdout.log')).Hash; untracked = $files; pilotTicks = $PilotTicks; cargo = $cargoVersion; rustc = $rustcVersion; profile = 'release'; features = 'foundation-training'; curriculum = $Curriculum; cycleTicks = $CycleTicks; semanticPriorMode = $env:ALIFE_SLM_PRIOR; semanticPriorModel = $env:ALIFE_SLM_PRIOR_MODEL; semanticPriorModelSha256 = $env:ALIFE_SLM_PRIOR_MODEL_SHA256 }
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $fingerprint = [BitConverter]::ToString($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes(($identity | ConvertTo-Json -Depth 8 -Compress)))).Replace('-', '') } finally { $sha.Dispose() }
    $receipt = [ordered]@{ fingerprint = $fingerprint; identity = $identity }
    Atomic-Json (Join-Path $Directory 'source.json') $receipt
    return $receipt
}
function Invoke-Campaign {
    if (-not $Source) { throw 'Campaign needs -Source pointing to a sealed adaptation, warm-up, or cycle directory.' }
    $sourcePath = [IO.Path]::GetFullPath($(if ([IO.Path]::IsPathRooted($Source)) { $Source } else { Join-Path $repo $Source }))
    $relative = [IO.Path]::GetRelativePath($base, $sourcePath)
    $bundledRelative = [IO.Path]::GetRelativePath((Join-Path $repo 'assets/founders'), $sourcePath)
    $trainingSource = $relative -ne '.' -and -not $relative.StartsWith('..') -and -not [IO.Path]::IsPathRooted($relative)
    $bundledAdaptation = $bundledRelative -ne '.' -and -not $bundledRelative.StartsWith('..') -and
        -not [IO.Path]::IsPathRooted($bundledRelative) -and
        (Test-Path -LiteralPath (Join-Path $sourcePath 'adaptation.json'))
    if (-not ($trainingSource -or $bundledAdaptation) -or
        -not (Test-Path -LiteralPath $sourcePath -PathType Container) -or
        -not ((Test-Path -LiteralPath (Join-Path $sourcePath 'warmup.json')) -or
              (Test-Path -LiteralPath (Join-Path $sourcePath 'cycle.json')) -or
              (Test-Path -LiteralPath (Join-Path $sourcePath 'adaptation.json')))) {
        throw 'Campaign source must be sealed under target/founder-training, or a bundled adaptation under assets/founders.'
    }
    $script:run = Join-Path $base ("campaign-{0}-{1}" -f [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss'), [Guid]::NewGuid().ToString('N').Substring(0, 8))
    [IO.Directory]::CreateDirectory($run) | Out-Null
    $sourceReceipt = Source-Receipt (Join-Path $run 'source')
    $exe = Join-Path $repo 'target/release/train_n2048_care.exe'
    $script:state = [ordered]@{
        schema = 2; state = 'Running'; phase = 'build'; pid = $PID; run = $run
        fingerprint = $sourceReceipt.fingerprint; executable = $exe; executableSha256 = $null
        source = $sourcePath; latestCheckpoint = $sourcePath; deadlineUtc = $null
        cycleTicks = $CycleTicks; maxCycles = $MaxCycles; updatedUtc = ''; completed = @(); bestByLesson = [ordered]@{}; error = $null
        curriculumStage = 0; gateEveryCycles = $GateEveryCycles; behaviorPanels = @(); bestBehaviorCheckpoint = $null; bestBehaviorByStage = [ordered]@{}
    }
    Publish-State
    Run-Command 'cargo' @('build', '--release', '--offline', '--locked', '-j', '2', '-p', 'alife_game_app',
        '--features', 'foundation-training', '--bin', 'train_n2048_care') (Join-Path $run 'build')
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'Release trainer was not built.' }
    $deadline = [DateTime]::UtcNow.AddHours($DurationHours)
    # Preserve care while adding longer navigation and both language directions.
    $lessons = if ($Curriculum -eq 'Care') {
        @('obstacle_navigation', 'feeding', 'obstacle_navigation', 'hazard_avoidance', 'obstacle_navigation', 'recovery')
    } else {
        @('vision_search','maze_navigation','vision_search','maze_navigation','vocabulary_reception','vocabulary_production','vocabulary_reception','vocabulary_production','feeding','hazard_avoidance')
    }
    $foodPositions = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    $state['lessonSchedule'] = $lessons
    $state.phase = 'campaign'
    $state.executableSha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash
    $state.deadlineUtc = $deadline.ToString('o')
    Publish-State
    $previous = $sourcePath
    $index = 0
    while ([DateTime]::UtcNow -lt $deadline -and $index -lt $MaxCycles) {
        $env:ALIFE_TRAINING_STAGE = [string]$state.curriculumStage
        Assert-Idle
        if ((Source-Receipt (Join-Path $run ("source-cycle-{0:D4}" -f $index))).fingerprint -ne $sourceReceipt.fingerprint -or
            (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash -ne $state.executableSha256) {
            throw 'Campaign source or executable changed between cycles.'
        }
        $lessonIndex = $index % $lessons.Count
        $lesson = $lessons[$lessonIndex]
        # Vary token identity across schedule repetitions, not just scenery.
        $seed = [ulong](539366000 + $index + 3 * [Math]::Floor($index / $lessons.Count))
        $decisions = if ($lesson -eq 'maze_navigation') { [Math]::Max($CycleTicks, 1024) } else { $CycleTicks }
        $directory = Join-Path $run ("cycle-{0:D4}-{1}" -f $index, $lesson)
        $state.phase = "cycle-$index-$lesson"; Publish-State
        Write-Host "Training $lesson cycle $index; logs: $run"
        Run-Command $exe @('--resume-cycle', $previous, $directory, [string]$decisions,
            '--seed', [string]$seed, '--lesson', $lesson) (Join-Path $run ("cycle-{0:D4}" -f $index)) -DeadlineUtc $deadline.AddMinutes(30)
        $receiptPath = Join-Path $directory 'cycle.json'
        $receipt = Get-Content -Raw -LiteralPath $receiptPath | ConvertFrom-Json
        if ($receipt.lesson -ne $lesson -or $receipt.seed -ne $seed -or
            -not $receipt.next_cohort_tick_captured -or -not $receipt.next_cohort_optimizer_rebound -or
            $receipt.training_ticks -lt 1 -or $receipt.new_asset_digest -notmatch '^[0-9a-f]{64}$') {
            throw "Cycle $index did not seal a valid next-cohort handoff."
        }
        $scenario = Get-Content -Raw -LiteralPath (Join-Path $directory 'scenario.json') | ConvertFrom-Json
        if ($scenario.world_seed -ne $seed -or $scenario.lesson -ne $lesson -or
            @($scenario.food_position).Count -ne 3) {
            throw "Cycle $index has an invalid scenario receipt."
        }
        $foodKey = @($scenario.food_position) -join ','
        if (-not $foodPositions.Add($foodKey)) {
            throw "Cycle $index repeated a food position: $foodKey"
        }
        $row = [ordered]@{
            index = $index; lesson = $lesson; seed = $seed; directory = $directory
            foodPosition = $scenario.food_position
            policyVersion = $receipt.policy_version; decisions = $receipt.training_ticks
            meals = $receipt.consumed_events; blocked = $receipt.blocked_actions
            collisions = $receipt.collision_actions; avoid = $receipt.avoid_actions
            recovery = $receipt.rest_recovery_actions; minimumEnergy = $receipt.minimum_energy
            speechTargetRows = $receipt.speech_target_rows
            semanticPrior = if ($null -eq $receipt.semantic_prior) {$null} else {[ordered]@{
                model=$receipt.semantic_prior.model;delivered_frames=$receipt.semantic_prior.delivered_frames;
                requests=$receipt.semantic_prior.requests;failures=$receipt.semantic_prior.failures;
                cache_hits=$receipt.semantic_prior.cache_hits;prime_timeouts=$receipt.semantic_prior.prime_timeouts}}
            receipt = $receiptPath
            finalEnergy = $receipt.final_energy; terminalDeathTick = $receipt.terminal_death_tick
            collectionSeconds = $receipt.collection_seconds; updateSeconds = $receipt.update_seconds
            assetDigest = $receipt.new_asset_digest
        }
        $state.completed += $row
        $previous = $directory; $state.latestCheckpoint = $directory
        # Training-world quality is only a provisional comparison; final
        # acceptance needs separate teacher-free worlds on a frozen policy.
        $score = 10 * [Math]::Min([int]$row.meals, 3) + [double]$row.finalEnergy +
            [Math]::Min([int]$(if ($lesson -eq 'recovery') { $row.recovery } else { 0 }), 4) -
            2 * [int]$row.blocked - 5 * [int]$row.collisions
        $best = $state.bestByLesson[$lesson]
        if ($lesson -notlike 'vocabulary_*' -and ($null -eq $best -or $score -gt $best.score)) {
            $state.bestByLesson[$lesson] = [ordered]@{ score = $score; directory = $directory; receipt = $receiptPath }
        }
        Publish-State
        $index++
        if ($Curriculum -eq 'VisionLanguage' -and $index % $GateEveryCycles -eq 0 -and
            [DateTime]::UtcNow.AddMinutes(20) -lt $deadline) {
            Invoke-BehaviorPanel $exe $previous $index $deadline
        }
    }
    $state.state = 'Completed'
    $state.phase = $(if ($index -ge $MaxCycles) { 'cycle-limit-reached' } else { 'deadline-reached' })
    Publish-State
    Write-Output "Campaign completed $index bounded cycles. Latest checkpoint: $previous"
}
function Invoke-BehaviorPanel([string]$Exe, [string]$Checkpoint, [int]$Index, [datetime]$Deadline) {
    # Frozen held-out worlds. No optimizer, teacher control, or in-place weight changes.
    $adaptation = Get-Content -Raw (Join-Path $Checkpoint 'cycle.json') | ConvertFrom-Json
    $panel = Join-Path $run ("behavior-{0:D4}-stage-{1}" -f $Index,$state.curriculumStage)
    [IO.Directory]::CreateDirectory($panel) | Out-Null
    $rows = @(); $priorBefore = $env:ALIFE_SLM_PRIOR
    try {
        for ($group=0; $group -lt 3; $group++) {
            $lesson = @('maze_navigation','vocabulary_reception','vocabulary_production')[$group]
            for ($case=0; $case -lt 3; $case++) {
                # Fixed disjoint seed band. Food noun, play, and a real hunger need.
                $seed = [ulong](771900000 + $case * 100)
                if ($group -gt 0) {
                    $word = switch ($state.curriculumStage) {0 {@(1,13,11)[$case]}; 1 {@(5,9,12)[$case]}; default {@(15,16,18)[$case]}}
                    $seed += [ulong](($word-1 + 18 - ($seed % 18)) % 18)
                }
                $directory = Join-Path $panel "$lesson-$case"
                $ticks = if ($group -eq 0) {2048} else {256}
                $state.phase = "held-out-$lesson-$case"; Publish-State
                Run-Command $Exe @('--evaluate',$directory,(Join-Path $Checkpoint 'trained.alife-foundation'),
                    $lesson,[string]$ticks,[string]$seed,[string]$adaptation.founder_seed_base) (Join-Path $panel "$lesson-$case") -DeadlineUtc $Deadline
                $receipt = Get-Content -Raw (Join-Path $directory 'pilot.json') | ConvertFrom-Json
                if ($receipt.teacher_mode -or $receipt.source_asset_digest -ne $adaptation.new_asset_digest) {throw 'Frozen panel source mismatch.'}
                $rows += [ordered]@{group=$group;case=$case;seed=$seed;passed=($receipt.lesson_completed -eq $true);
                    meals=$receipt.consumed_events;targetActions=$receipt.vocabulary_target_actions;
                    correctUtterances=$receipt.correct_utterances;unprompted=$receipt.unprompted_correct_utterances;
                    speechOpportunities=$receipt.speech_opportunities;receipt=(Join-Path $directory 'pilot.json')}
            }
        }
        $scores = @(0,1,2 | ForEach-Object {$group=$_; @($rows | Where-Object {$_.group -eq $group -and $_.passed}).Count})
        # Run the expensive unaided probes only after the assisted panel passes.
        $unaided = @()
        if (@($scores | Where-Object {$_ -lt 2}).Count -eq 0) {
            $env:ALIFE_SLM_PRIOR = 'off'
            foreach ($row in @($rows | Where-Object {$_.case -eq 0 -and $_.group -ne 1})) {
                $lesson = @('maze_navigation','vocabulary_reception','vocabulary_production')[$row.group]
                $directory = Join-Path $panel "unaided-$lesson"
                $ticks = if ($row.group -eq 0) {2048} else {256}
                Run-Command $Exe @('--evaluate',$directory,(Join-Path $Checkpoint 'trained.alife-foundation'),
                    $lesson,[string]$ticks,[string]$row.seed,[string]$adaptation.founder_seed_base) (Join-Path $panel "unaided-$lesson") -DeadlineUtc $Deadline
                $receipt = Get-Content -Raw (Join-Path $directory 'pilot.json') | ConvertFrom-Json
                $unaided += ($receipt.lesson_completed -eq $true)
            }
        }
        $passed = @($scores | Where-Object {$_ -lt 2}).Count -eq 0 -and $unaided.Count -eq 2 -and $unaided -notcontains $false
        $summary = [ordered]@{checkpoint=$Checkpoint;stage=$state.curriculumStage;scores=$scores;rows=$rows;unaided=$unaided;passed=$passed}
        Atomic-Json (Join-Path $panel 'panel.json') $summary
        $state.behaviorPanels += $summary
        $best = $state.bestBehaviorByStage[[string]$summary.stage]
        # Keep separate scores: improvements cannot buy a regression in another skill.
        if ($null -eq $best -or ($best.stage -eq $summary.stage -and
            $scores[0] -ge $best.scores[0] -and $scores[1] -ge $best.scores[1] -and $scores[2] -ge $best.scores[2] -and
            ($scores[0]+$scores[1]+$scores[2]) -gt ($best.scores[0]+$best.scores[1]+$best.scores[2]))) {
            $state.bestBehaviorByStage[[string]$summary.stage] = $summary
        }
        if ($passed -and ($null -eq $state.bestBehaviorCheckpoint -or $summary.stage -ge $state.bestBehaviorCheckpoint.stage)) {$state.bestBehaviorCheckpoint=$summary}
        if ($passed -and $state.curriculumStage -lt 2) {$state.curriculumStage++}
        Publish-State
    } finally { $env:ALIFE_SLM_PRIOR = $priorBefore }
}
try {
    $lock = [IO.File]::Open((Join-Path $base 'operator.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
    Assert-Idle
    $cargoVersion = (& cargo --version) -join "`n"; if ($LASTEXITCODE) { throw 'Cargo unavailable.' }
    $rustcVersion = (& rustc --version) -join "`n"; if ($LASTEXITCODE) { throw 'Rust compiler unavailable.' }
    if ($Mode -eq 'Campaign') { Invoke-Campaign; return }
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
        @('speech', '-p', 'alife_training', '--features', 'gpu-tests', '--test', 'wgsl_foundation_trainer', 'n2048_speech_payload_head_trains_on_gpu_and_remains_exportable'),
        @('biology', '-p', 'alife_world', '--test', 'canonical_new_game', 'food_and_toy_variants_have_physical_gene_controlled_and_persistent_effects'),
        @('memory', '-p', 'alife_core', '--test', 'candidate_memory_retrieval', 'memory_bank_roundtrip_rebuilds_indices_and_preserves_recall'),
        @('maze', '-p', 'alife_game_app', '--features', 'foundation-training,gpu-tests', '--lib', 'foundation_training::maze::tests::layouts_change_connectivity_and_keep_routes_connected'),
        @('resume', '-p', 'alife_game_app', '--features', 'foundation-training,gpu-tests', '--lib', 'gpu_live_runtime::checkpoint_manifest_pruning_tests::readiness_resume_preserves_external_actors_toys_and_private_prior')
    )
    $steps = @([pscustomobject]@{ name = 'build'; argv = @('build', '--release', '--offline', '--locked', '-j', '2', '-p', 'alife_game_app', '--features', 'foundation-training', '--bin', 'train_n2048_care'); test = $false })
    $steps += @($gates | ForEach-Object { [pscustomobject]@{ name = $_[0]; argv = @('test', '--offline', '--locked', '-j', '2') + $_[1..($_.Count - 1)] + @('--', '--exact', '--test-threads=1'); test = $true } })
    foreach ($step in $steps) {
        Assert-Idle; $state.phase = $step.name; Publish-State; Write-Host "Running $($step.name); logs: $run"
        if ((Source-Receipt (Join-Path $run "source-$($step.name)")).fingerprint -ne $source.fingerprint) { throw 'Source changed during preparation.' }
        Run-Command 'cargo' $step.argv (Join-Path $run $step.name) -OneTest:$step.test
        $state.completed += [ordered]@{ phase = $step.name; argv = $step.argv }; Publish-State
    }
    Assert-Idle
    if ((Source-Receipt (Join-Path $run 'source-pilot')).fingerprint -ne $source.fingerprint) { throw 'Source changed before pilot.' }
    $exe = Join-Path $repo 'target/release/train_n2048_care.exe'
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
    Write-Output "Pilot verified: $receiptPath. Preparation is complete. Overnight Campaign remains a separate explicit launch."
} catch {
    if ($null -ne $state) { $state.state = 'Failed'; $state.error = $_.Exception.Message; Publish-State }
    throw
} finally { if ($null -ne $lock) { $lock.Dispose() } }
