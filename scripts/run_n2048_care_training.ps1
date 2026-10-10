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
function Assert-PriorHealth($Receipt, [string]$Context) {
    if ($env:ALIFE_SLM_PRIOR -eq 'off' -or $env:ALIFE_SLM_PRIOR_BACKEND -eq 'off') { return }
    $prior = $Receipt.semantic_prior
    if ($null -eq $prior -or $prior.failures -gt 0 -or $prior.prime_timeouts -gt 0) {
        Write-Warning "${Context}: optional semantic prior unavailable or failed; continuing with genuine episode receipts."
        return
    }
    $dropout = $prior.dropout_frames -gt 0 -and $prior.requests -eq 0 -and $prior.cache_hits -eq 0
    if ($prior.delivered_frames -lt 1 -and -not $dropout) {
        Write-Warning "${Context}: no grounded language prior hints delivered; continuing unaided."
    }
    if ((-not $env:ALIFE_SLM_PRIOR_BACKEND -or $env:ALIFE_SLM_PRIOR_BACKEND -eq 'local') -and $env:ALIFE_SLM_PRIOR_MODEL -and $prior.model -ne $env:ALIFE_SLM_PRIOR_MODEL) {
        Write-Warning "${Context}: semantic prior model differs from requested local model; inspect provenance."
    }
}
function Run-Command([string]$Program, [string[]]$Arguments, [string]$Log, [switch]$OneTest, [datetime]$DeadlineUtc = [datetime]::MaxValue) {
    if ([DateTime]::UtcNow -ge $DeadlineUtc) { throw [TimeoutException]::new('Wall-time budget exhausted before child launch.') }
    $info = [Diagnostics.ProcessStartInfo]::new($Program)
    $info.WorkingDirectory = $repo; $info.UseShellExecute = $false; $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true; $info.RedirectStandardError = $true
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $info.Environment['CARGO_BUILD_JOBS'] = '2'
    $info.Environment['CARGO_NET_OFFLINE'] = 'true'
    $info.Environment['CARGO_TARGET_DIR'] = if ($env:CARGO_TARGET_DIR) {
        [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
    } else { Join-Path $repo 'target' }
    $stdout = [IO.File]::Open("$Log.stdout.log", 'CreateNew', 'Write', 'Read')
    $stderr = [IO.File]::Open("$Log.stderr.log", 'CreateNew', 'Write', 'Read')
    $process = [Diagnostics.Process]::new(); $process.StartInfo = $info
    try {
        if (-not $process.Start()) { throw "Failed to start $Program" }
        $outTask = $process.StandardOutput.BaseStream.CopyToAsync($stdout)
        $errTask = $process.StandardError.BaseStream.CopyToAsync($stderr)
        while (-not $process.WaitForExit([int][Math]::Clamp(($DeadlineUtc - [DateTime]::UtcNow).TotalMilliseconds, 1, 1000))) {
            if ([DateTime]::UtcNow -ge $DeadlineUtc) {
                $process.Kill($true)
                $process.WaitForExit()
                [Threading.Tasks.Task]::WaitAll(@($outTask, $errTask))
                throw [TimeoutException]::new("Wall-time budget exhausted while running $Program; child process stopped.")
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
        selectedCheckpoint = $null; selectedCheckpointPassed = $false; finalComparison = $null; interruptedCohort = $null
        curriculumStage = 0; gateEveryCycles = $GateEveryCycles; behaviorPanels = @(); bestBehaviorCheckpoint = $null; bestBehaviorByStage = [ordered]@{}
    }
    $deadline = [DateTime]::UtcNow.AddHours($DurationHours)
    $collectionDeadline = $deadline.AddMinutes(-45)
    $state.deadlineUtc = $deadline.ToString('o')
    $state['collectionDeadlineUtc'] = $collectionDeadline.ToString('o')
    $state['finalAssessmentReserveMinutes'] = 45
    Publish-State
    Run-Command 'cargo' @('build', '--release', '--offline', '--locked', '-j', '2', '-p', 'alife_game_app',
        '--features', 'foundation-training', '--bin', 'train_n2048_care') (Join-Path $run 'build') -DeadlineUtc $collectionDeadline
    if ($env:CARGO_TARGET_DIR) {
        $builtExe = Join-Path ([IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)) 'release/train_n2048_care.exe'
        if ([IO.Path]::GetFullPath($builtExe) -ne [IO.Path]::GetFullPath($exe)) {
            [IO.Directory]::CreateDirectory((Split-Path -Parent $exe)) | Out-Null
            Copy-Item -LiteralPath $builtExe -Destination $exe -Force
        }
    }
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'Release trainer was not built.' }
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
    # Measure the starting animal before spending the collection window. This
    # uses the same frozen worlds as later comparisons, without promotion or
    # requiring it to have already mastered every skill the campaign teaches.
    $state['startingBehavior'] = Invoke-BehaviorPanel $exe $sourcePath 0 $collectionDeadline -FinalAssessment -Tag 'starting'
    if (@($state.startingBehavior.scores | Where-Object { $_ -gt 0 }).Count -eq 0) {
        throw 'Starting founder showed no held-out navigation, request contrast, speech, or care competence. Strengthen imitation before a campaign.'
    }
    $previous = $sourcePath
    $index = 0
    while ([DateTime]::UtcNow -lt $collectionDeadline -and $index -lt $MaxCycles) {
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
        try { Run-Command $exe @('--resume-cycle', $previous, $directory, [string]$decisions,
            '--seed', [string]$seed, '--lesson', $lesson) (Join-Path $run ("cycle-{0:D4}" -f $index)) -DeadlineUtc $collectionDeadline
        } catch [TimeoutException] {
            # A partial directory is never a next source. Preserve the last
            # fully verified handoff and spend the reserved time assessing it.
            $state.interruptedCohort = $directory; Publish-State; break
        }
        $receiptPath = Join-Path $directory 'cycle.json'
        $receipt = Get-Content -Raw -LiteralPath $receiptPath | ConvertFrom-Json
        if ($receipt.lesson -ne $lesson -or $receipt.seed -ne $seed -or
            -not $receipt.next_cohort_tick_captured -or -not $receipt.next_cohort_optimizer_rebound -or
            $receipt.training_ticks -lt 1 -or $receipt.new_asset_digest -notmatch '^[0-9a-f]{64}$') {
            throw "Cycle $index did not seal a valid next-cohort handoff."
        }
        Assert-PriorHealth $receipt "Cycle $index"
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
            [DateTime]::UtcNow.AddMinutes(20) -lt $collectionDeadline) {
            try { Invoke-BehaviorPanel $exe $previous $index $collectionDeadline | Out-Null } catch [TimeoutException] { break }
        }
    }
    Invoke-FinalComparison $exe $sourcePath $previous $index $deadline
    $state.state = 'Completed'
    $state.phase = $(if ($index -ge $MaxCycles) { 'cycle-limit-reached' } else { 'deadline-reached' })
    Publish-State
    Write-Output "Campaign completed $index bounded cycles. Latest checkpoint: $previous"
}
function Checkpoint-Identity([string]$Checkpoint) {
    foreach ($name in @('cycle.json','warmup.json','adaptation.json')) {
        $path = Join-Path $Checkpoint $name
        if (Test-Path -LiteralPath $path) {
            $receipt = Get-Content -Raw -LiteralPath $path | ConvertFrom-Json
            $digest = switch ($name) {'cycle.json' {$receipt.new_asset_digest}; 'warmup.json' {$receipt.trained_asset_digest}; default {$receipt.adapted_asset_digest}}
            return [pscustomobject]@{founderSeed=$receipt.founder_seed_base;digest=$digest;asset=(Join-Path $Checkpoint 'trained.alife-foundation')}
        }
    }
    throw 'Candidate has no sealed native checkpoint receipt.'
}
function Test-RequestContrast($Receipts) {
    if ($Receipts.Count -ne 3) {throw 'Contrast requires two requests and silence.'}
    if (@($Receipts.initial_world_signature | Select-Object -Unique).Count -ne 1 -or
        -not $Receipts[0].initial_world_signature -or
        @($Receipts.source_asset_digest | Select-Object -Unique).Count -ne 1) {throw 'Request contrast changed the scene or policy.'}
    if (-not $Receipts[2].silent_request -or $Receipts[2].heard_word_frames -ne 0) {throw 'Silent control heard a request.'}
    $correct = @(); $first = @()
    foreach ($receipt in $Receipts[0..1]) {
        $response = @($receipt.request_responses) | Select-Object -First 1
        $actionMatches = $null -ne $response -and $(switch ($receipt.vocabulary_token) {9 {$response.action -eq 211}; 13 {$response.action -eq 213}; default {$true}})
        $correct += ($receipt.heard_word_frames -gt 0 -and $actionMatches -and $response.target -eq $receipt.request_target)
        $first += $(if ($null -eq $response) {''} else {"$($response.action)/$($response.target)"})
    }
    # Inspecting both objects eventually does not establish understanding.
    return [pscustomobject]@{correct=$correct;passed=($correct -notcontains $false -and $first[0] -ne $first[1]);first=$first}
}
function Invoke-BehaviorPanel([string]$Exe, [string]$Checkpoint, [int]$Index, [datetime]$Deadline,
    [switch]$FinalAssessment, [string]$Tag = 'periodic') {
    $identity = Checkpoint-Identity $Checkpoint
    $panel = Join-Path $run ("behavior-{0:D4}-stage-{1}-{2}" -f $Index,$state.curriculumStage,$Tag)
    [IO.Directory]::CreateDirectory($panel) | Out-Null
    $rows = @(); $priorBefore = $env:ALIFE_SLM_PRIOR; $stageBefore = $env:ALIFE_TRAINING_STAGE; $contrastReceipts = @()
    try {
        $env:ALIFE_TRAINING_STAGE = [string]$state.curriculumStage
        for ($group=0; $group -lt 4; $group++) {
            $lesson = @('maze_navigation','vocabulary_reception','vocabulary_production','feeding')[$group]
            $cases = if ($group -eq 3) {2} else {3}
            for ($case=0; $case -lt $cases; $case++) {
                $seed = [ulong](771900000 + $case * 100)
                $extra = @()
                if ($group -eq 1) {
                    # Same physical scene and random stream in all three cases.
                    $pair = switch ($state.curriculumStage) {0 {@(1,2)}; 1 {@(9,13)}; default {@(15,16)}}
                    $sceneWord = switch ($state.curriculumStage) {0 {1}; 1 {9}; default {15}}
                    $seed = [ulong]771910000
                    $seed += [ulong](($sceneWord-1 + 18 - ($seed % 18)) % 18)
                    $word = $pair[[Math]::Min($case,1)]
                    $extra = @('--request-token',[string]$word)
                    if ($case -eq 2) {$extra += '--silent-request'}
                } elseif ($group -eq 2) {
                    $word = switch ($state.curriculumStage) {0 {@(1,13,11)[$case]}; 1 {@(5,9,12)[$case]}; default {@(15,16,18)[$case]}}
                    $seed += [ulong](($word-1 + 18 - ($seed % 18)) % 18)
                } elseif ($group -eq 3 -and $case -eq 1) {$lesson='hazard_avoidance'}
                $directory = Join-Path $panel "$lesson-$case"
                $ticks = if ($group -eq 0) {2048} elseif ($group -eq 3) {512} else {256}
                $state.phase = "held-out-$lesson-$case"; Publish-State
                Run-Command $Exe (@('--evaluate',$directory,$identity.asset,$lesson,[string]$ticks,
                    [string]$seed,[string]$identity.founderSeed) + $extra) (Join-Path $panel "$lesson-$case") -DeadlineUtc $Deadline
                $receipt = Get-Content -Raw (Join-Path $directory 'pilot.json') | ConvertFrom-Json
                if ($receipt.teacher_mode -or $receipt.source_asset_digest -ne $identity.digest -or
                    $receipt.seed -ne $seed -or $receipt.founder_seed_base -ne $identity.founderSeed -or
                    $receipt.lesson -ne $lesson) {throw 'Frozen panel source/scenario mismatch.'}
                Assert-PriorHealth $receipt "Frozen $lesson case $case"
                if ($group -eq 1) {$contrastReceipts += $receipt}
                $rows += [ordered]@{group=$group;case=$case;seed=$seed;passed=($receipt.lesson_completed -eq $true);
                    meals=$receipt.consumed_events;targetActions=$receipt.vocabulary_target_actions;
                    correctUtterances=$receipt.correct_utterances;unprompted=$receipt.unprompted_correct_utterances;
                    speechOpportunities=$receipt.speech_opportunities;
                    semanticPrior=if ($null -eq $receipt.semantic_prior) {$null} else {[ordered]@{
                        model=$receipt.semantic_prior.model;delivered_frames=$receipt.semantic_prior.delivered_frames;
                        requests=$receipt.semantic_prior.requests;failures=$receipt.semantic_prior.failures;
                        cache_hits=$receipt.semantic_prior.cache_hits;prime_timeouts=$receipt.semantic_prior.prime_timeouts}};
                    receipt=(Join-Path $directory 'pilot.json')}
            }
        }
        $contrast = Test-RequestContrast $contrastReceipts
        $scores = @(0,1,2,3 | ForEach-Object {$group=$_; @($rows | Where-Object {$_.group -eq $group -and $_.passed}).Count})
        # Reception is a contrast, never a tally of eventual target actions.
        $scores[1] = if ($contrast.passed) {2} else {0}
        $assistedPass = $scores[0] -ge 2 -and $scores[1] -eq 2 -and $scores[2] -ge 2 -and $scores[3] -eq 2
        $unaided = @()
        if ($assistedPass) {
            $env:ALIFE_SLM_PRIOR = 'off'
            foreach ($row in @($rows | Where-Object {$_.case -eq 0 -and $_.group -in @(0,2)})) {
                $lesson = @('maze_navigation','vocabulary_reception','vocabulary_production')[$row.group]
                $directory = Join-Path $panel "unaided-$lesson"
                $ticks = if ($row.group -eq 0) {2048} else {256}
                Run-Command $Exe @('--evaluate',$directory,$identity.asset,$lesson,[string]$ticks,
                    [string]$row.seed,[string]$identity.founderSeed) (Join-Path $panel "unaided-$lesson") -DeadlineUtc $Deadline
                $receipt = Get-Content -Raw (Join-Path $directory 'pilot.json') | ConvertFrom-Json
                if ($receipt.teacher_mode -or $receipt.source_asset_digest -ne $identity.digest) {throw 'Unaided source mismatch.'}
                $unaided += ($receipt.lesson_completed -eq $true)
            }
        }
        $passed = $assistedPass -and $unaided.Count -eq 2 -and $unaided -notcontains $false
        $summary = [ordered]@{checkpoint=$Checkpoint;assetDigest=$identity.digest;founderSeed=$identity.founderSeed;
            stage=$state.curriculumStage;scoreOrder=@('navigation','requestContrast','speaking','care');scores=$scores;
            rows=$rows;contrast=$contrast;unaided=$unaided;passed=$passed;finalAssessment=[bool]$FinalAssessment}
        Atomic-Json (Join-Path $panel 'panel.json') $summary
        $state.behaviorPanels += $summary
        if (-not $FinalAssessment) {
            $best = $state.bestBehaviorByStage[[string]$summary.stage]
            if ($null -eq $best -or (Test-ScoreImprovement $summary $best)) {$state.bestBehaviorByStage[[string]$summary.stage]=$summary}
            if ($passed) {$state.bestBehaviorCheckpoint=$summary}
            if ($passed -and $state.curriculumStage -lt 2) {$state.curriculumStage++}
        }
        Publish-State
        return $summary
    } finally { $env:ALIFE_SLM_PRIOR = $priorBefore; $env:ALIFE_TRAINING_STAGE = $stageBefore }
}
function Test-ScoreImprovement($Candidate,$Reference) {
    if ($Candidate.stage -ne $Reference.stage -or $Candidate.founderSeed -ne $Reference.founderSeed) {return $false}
    $better=$false
    for ($i=0;$i -lt 4;$i++) {
        if ($Candidate.scores[$i] -lt $Reference.scores[$i]) {return $false}
        if ($Candidate.scores[$i] -gt $Reference.scores[$i]) {$better=$true}
    }
    if ($Reference.passed -and -not $Candidate.passed) {return $false}
    if ($Candidate.passed -and -not $Reference.passed) {$better=$true}
    return $better
}
function Invoke-FinalComparison([string]$Exe,[string]$Baseline,[string]$Latest,[int]$Index,[datetime]$Deadline) {
    $candidates = @($Baseline,$Latest)
    $best = $state.bestBehaviorByStage[[string]$state.curriculumStage]
    if ($null -eq $best) {$best = $state.bestBehaviorCheckpoint}
    if ($null -ne $best) {$candidates += $best.checkpoint}
    $seen = [Collections.Generic.HashSet[string]]::new()
    $panels = @(); $interrupted = $false; $reference = Checkpoint-Identity $Baseline
    foreach ($checkpoint in $candidates) {
        $identity = Checkpoint-Identity $checkpoint
        if ($identity.founderSeed -ne $reference.founderSeed) {throw 'Incompatible final candidate founder.'}
        if (-not $seen.Add($identity.digest)) {continue}
        try {
            $panels += Invoke-BehaviorPanel $Exe $checkpoint $Index $Deadline -FinalAssessment -Tag ("final-{0}" -f $panels.Count)
        } catch [TimeoutException] {$interrupted=$true; break}
    }
    $winner = if ($panels.Count) {$panels[0]} else {$null}
    foreach ($panel in $panels | Select-Object -Skip 1) {
        if (Test-ScoreImprovement $panel $winner) {$winner=$panel}
    }
    # Baseline wins ties/incomparable scores. A incomplete comparison cannot
    # silently certify a partially evaluated candidate. This never promotes it.
    $state.finalComparison = [ordered]@{panels=$panels;interrupted=$interrupted;completed=($panels.Count -eq $seen.Count -and -not $interrupted);
        selectionRule='same-stage componentwise improvement; baseline wins ties and tradeoffs';winner=$winner}
    if ($null -ne $winner) {$state.selectedCheckpoint=$winner.checkpoint;$state.selectedCheckpointPassed=$winner.passed}
    Atomic-Json (Join-Path $run 'final-comparison.json') $state.finalComparison
    Publish-State
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
} catch [TimeoutException] {
    if ($null -ne $state) { $state.state = 'Completed'; $state.phase = 'budget-exhausted-before-final-comparison'; $state.error = $_.Exception.Message; Publish-State }
    Write-Output 'Wall-time budget exhausted. No unchecked checkpoint selected.'
} catch {
    if ($null -ne $state) { $state.state = 'Failed'; $state.error = $_.Exception.Message; Publish-State }
    throw
} finally { if ($null -ne $lock) { $lock.Dispose() } }
