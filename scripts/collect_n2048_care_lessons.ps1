#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Output,
    [Parameter(Mandatory)][string]$Source,
    [ValidateSet('Care','VisionLanguage')][string]$Curriculum = 'VisionLanguage',
    [ValidateRange(1,2)][int]$ExamplesPerWord = 2
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$exe = Join-Path $repo 'target/release/train_n2048_care.exe'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'Build the release training CLI first.' }
$sourcePath = (Resolve-Path -LiteralPath $Source).Path
$adaptation = Get-Content -Raw -LiteralPath (Join-Path $sourcePath 'adaptation.json') | ConvertFrom-Json
$FounderSeedBase = [ulong]$adaptation.founder_seed_base
if ($FounderSeedBase -eq 0 -or -not $adaptation.optimizer_reset -or
    $adaptation.founder_biology_calibration -ne 2) { throw 'Use a verified terrain adaptation source.' }
$outputPath = [IO.Path]::GetFullPath((Join-Path (Get-Location).Path $Output))
if (Test-Path -LiteralPath $outputPath) { throw "Dataset output already exists: $outputPath" }
$base = Join-Path $repo 'target/founder-training'
[IO.Directory]::CreateDirectory($base) | Out-Null
$lock = [IO.File]::Open((Join-Path $base 'operator.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
try {
    $busy = @(Get-Process | Where-Object { $_.ProcessName -match '^(cargo|rustc|train_n2048_care|alife_.*)$' })
    if ($busy.Count) { throw "GPU/Cargo lane occupied: $($busy.ProcessName -join ', ')" }
    [IO.Directory]::CreateDirectory($outputPath) | Out-Null
    $entries = [Collections.Generic.List[string]]::new()
    $counts = [ordered]@{ feeding = 0; hazard_avoidance = 0; obstacle_navigation = 0; recovery = 0;
        vision_search = 0; maze_navigation = 0; vocabulary_reception = 0; vocabulary_production = 0 }
    $obstacleSides = [Collections.Generic.HashSet[int]]::new()
    $recoveryFatigueLevels = [Collections.Generic.HashSet[string]]::new()
    $languageWords = @{ vocabulary_reception = [Collections.Generic.HashSet[int]]::new();
        vocabulary_production = [Collections.Generic.HashSet[int]]::new() }
    $expected = if ($Curriculum -eq 'Care') { @(8,8,8,8,0,0,0,0) } else { @(2,2,2,0,4,4,(18*$ExamplesPerWord),(18*$ExamplesPerWord)) }
    $lessons = @($counts.Keys)
    $schedule = [Collections.Generic.List[string]]::new()
    for ($ordinal=0; $ordinal -lt ($expected | Measure-Object -Maximum).Maximum; $ordinal++) {
        for ($category=0; $category -lt $lessons.Count; $category++) {
            if ($ordinal -lt $expected[$category]) { $schedule.Add($lessons[$category]) }
        }
    }
    for ($index = 0; $index -lt $schedule.Count; $index++) {
        $lesson = $schedule[$index]
        $ordinal = $counts[$lesson]
        # Thirteen alternates route parity and visits all eighteen lesson vocabulary tokens.
        $seed = [ulong](539364000 + 1000 * $lessons.IndexOf($lesson) + 13 * $ordinal)
        $name = ('lesson-{0:D2}-{1}' -f $index, $lesson)
        $directory = Join-Path $outputPath $name
        # A cap, not a fixed-length lesson: the CLI stops at the measured meal.
        # Scanning and turning can take longer than the old transparent-view scripts.
        $ticks = switch ($lesson) { 'maze_navigation' { 1024 }; 'vocabulary_reception' { 64 }; 'vocabulary_production' { 64 }; default { 128 } }
        $arguments = @('--teacher-adapted', $sourcePath, $directory, [string]$ticks, [string]$seed, $lesson)
        & $exe @arguments 1> (Join-Path $outputPath "$name.stdout.log") 2> (Join-Path $outputPath "$name.stderr.log")
        if ($LASTEXITCODE -ne 0) { throw "Lesson $name failed; see its logs and diagnostic receipt." }
        $receipt = Get-Content -Raw -LiteralPath (Join-Path $directory 'pilot.json') | ConvertFrom-Json
        if ($receipt.lesson -ne $lesson -or $receipt.lesson_completed -ne $true -or
            $receipt.demonstration_replay_records -ne $receipt.ticks -or
            $receipt.founder_seed_base -ne $FounderSeedBase -or
            $receipt.source_asset_digest -ne $adaptation.adapted_asset_digest) { throw "Lesson $name receipt mismatch." }
        if ($lesson -in @('vocabulary_reception','vocabulary_production')) {
            if ($receipt.vocabulary_token -lt 1 -or $receipt.vocabulary_token -gt 18) { throw "Lesson $name has no valid word." }
            [void]$languageWords[$lesson].Add([int]$receipt.vocabulary_token)
            if ($lesson -eq 'vocabulary_production' -and $receipt.speech_opportunities -lt 1) {
                throw "Lesson $name has no grounded production target; physical completion alone is insufficient."
            }
        }
        if ($lesson -eq 'hazard_avoidance' -or $lesson -eq 'obstacle_navigation' -or $lesson -eq 'recovery') {
            $trace = Get-Content -Raw -LiteralPath (Join-Path $directory 'lesson-trace.json') | ConvertFrom-Json
            if ($lesson -eq 'hazard_avoidance') {
                $families = @($trace.steps | ForEach-Object { $_.chosen_family })
                $avoidCount = @($families | Where-Object { $_ -eq 'Avoid' }).Count
                $approachCount = @($families | Where-Object { $_ -eq 'Approach' }).Count
                if ($avoidCount -lt 1 -or $approachCount -lt 1 -or $families[-1] -ne 'Ingest' -or
                    @($families[0..($avoidCount - 1)] | Where-Object { $_ -ne 'Avoid' }).Count -ne 0 -or
                    @($families[$avoidCount..($families.Count - 2)] | Where-Object { $_ -notin @('Approach', 'Inspect') }).Count -ne 0 -or
                    @($trace.steps | Where-Object { $_.physical.contact -in @('Blocked', 'Collision') }).Count -ne 0) {
                    throw "Hazard lesson $name did not flee, switch to food, and ingest without contact."
                }
            } elseif ($lesson -eq 'obstacle_navigation') {
                $firstMovement = $trace.steps | Where-Object { $_.physical.contact -eq 'Moved' } | Select-Object -First 1
                $side = [Math]::Sign([double]$firstMovement.physical.displacement.z)
                if ($side -eq 0) { throw "Obstacle lesson $name has no lateral route." }
                [void]$obstacleSides.Add($side)
            } else {
                $before = [double]$trace.steps[0].physiology.before_fatigue
                $after = [double]$trace.steps[-1].physiology.after_fatigue
                $families = @($trace.steps | ForEach-Object { $_.chosen_family })
                $restCount = @($families | Where-Object { $_ -eq 'Rest' }).Count
                if ($before -lt 0.12 -or $after -ge $before - 0.02 -or
                    $restCount -lt 1 -or $families[0] -ne 'Rest' -or $families[-1] -ne 'Ingest' -or
                    @($families[$restCount..($families.Count - 2)] | Where-Object { $_ -notin @('Approach', 'Inspect') }).Count -ne 0 -or
                    @($trace.steps | Where-Object { $_.physical.contact -eq 'Blocked' }).Count -ne 0) {
                    throw "Recovery lesson $name did not rest, approach food, and ingest."
                }
                [void]$recoveryFatigueLevels.Add(([Math]::Round($before, 2)).ToString('F2', [Globalization.CultureInfo]::InvariantCulture))
            }
        }
        $counts[$lesson]++
        $entries.Add($name)
        Write-Output "$name passed: $($receipt.ticks) sealed records"
    }
    for ($category=0;$category -lt $lessons.Count;$category++) {
        if ($counts[$lessons[$category]] -ne $expected[$category]) { throw 'Lesson corpus differs from its requested category counts.' }
    }
    if ($Curriculum -eq 'Care' -and ($obstacleSides.Count -ne 2 -or $recoveryFatigueLevels.Count -lt 3)) {
        throw 'Lesson corpus lacks both obstacle sides or varied biological fatigue.'
    }
    if ($Curriculum -eq 'VisionLanguage' -and
        ($languageWords.vocabulary_reception.Count -ne 18 -or $languageWords.vocabulary_production.Count -ne 18)) {
        throw 'Lesson corpus must cover all eighteen words in both directions.'
    }
    $manifest = [ordered]@{ founder_seed_base = $FounderSeedBase; pilots = @($entries);
        source_asset = (Join-Path $sourcePath 'trained.alife-foundation'); category_counts = $expected; curriculum = $Curriculum }
    [IO.File]::WriteAllText((Join-Path $outputPath 'manifest.json'),
        ($manifest | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
    Write-Output "Balanced manifest: $(Join-Path $outputPath 'manifest.json')"
} finally {
    $lock.Dispose()
}
