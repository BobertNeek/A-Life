#requires -Version 7.0
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Output,
    [ulong]$FounderSeedBase = 539363617
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$exe = Join-Path $repo 'target/release/train_n2048_care.exe'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'Build the release training CLI first.' }
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
    $counts = [ordered]@{ feeding = 0; hazard_avoidance = 0; obstacle_navigation = 0; recovery = 0 }
    $obstacleSides = [Collections.Generic.HashSet[int]]::new()
    $recoveryFatigueLevels = [Collections.Generic.HashSet[string]]::new()
    $lessons = @('feeding', 'hazard_avoidance', 'obstacle_navigation', 'recovery')
    for ($index = 0; $index -lt 32; $index++) {
        $lesson = $lessons[$index % 4]
        $ordinal = [int][Math]::Floor($index / 4)
        # The step of five alternates parity within each lesson category.
        # A +4 schedule made every obstacle route take the same side.
        $seed = [ulong](539364000 + 5 * $ordinal + ($index % 4))
        $name = ('lesson-{0:D2}-{1}' -f $index, $lesson)
        $directory = Join-Path $outputPath $name
        $ticks = switch ($lesson) { feeding { 16 } hazard_avoidance { 32 } obstacle_navigation { 64 } recovery { 40 } }
        $arguments = @('--teacher-pilot', $directory, [string]$ticks, '--seed', [string]$seed,
            '--founder-seed-base', [string]$FounderSeedBase, '--lesson', $lesson)
        if ($lesson -eq 'feeding') {
            $x = (4.2 + 0.4 * ($ordinal % 4)).ToString('R', [Globalization.CultureInfo]::InvariantCulture)
            $z = @('0', '-0.5', '0.5', '1')[$ordinal % 4]
            $arguments += @('--food-position', $x, $z)
        }
        & $exe @arguments 1> (Join-Path $outputPath "$name.stdout.log") 2> (Join-Path $outputPath "$name.stderr.log")
        if ($LASTEXITCODE -ne 0) { throw "Lesson $name failed; see its logs and diagnostic receipt." }
        $receipt = Get-Content -Raw -LiteralPath (Join-Path $directory 'pilot.json') | ConvertFrom-Json
        if ($receipt.lesson -ne $lesson -or $receipt.lesson_completed -ne $true -or
            $receipt.demonstration_replay_records -ne $receipt.ticks -or
            $receipt.founder_seed_base -ne $FounderSeedBase) { throw "Lesson $name receipt mismatch." }
        if ($lesson -eq 'hazard_avoidance' -or $lesson -eq 'obstacle_navigation' -or $lesson -eq 'recovery') {
            $trace = Get-Content -Raw -LiteralPath (Join-Path $directory 'lesson-trace.json') | ConvertFrom-Json
            if ($lesson -eq 'hazard_avoidance') {
                $families = @($trace.steps | ForEach-Object { $_.chosen_family })
                $avoidCount = @($families | Where-Object { $_ -eq 'Avoid' }).Count
                $approachCount = @($families | Where-Object { $_ -eq 'Approach' }).Count
                if ($avoidCount -lt 1 -or $approachCount -lt 1 -or $families[-1] -ne 'Ingest' -or
                    @($families[0..($avoidCount - 1)] | Where-Object { $_ -ne 'Avoid' }).Count -ne 0 -or
                    @($families[$avoidCount..($families.Count - 2)] | Where-Object { $_ -ne 'Approach' }).Count -ne 0 -or
                    @($trace.steps | Where-Object { $_.physical.contact -in @('Blocked', 'Collision') }).Count -ne 0) {
                    throw "Hazard lesson $name did not flee, switch to food, and ingest without contact."
                }
            } elseif ($lesson -eq 'obstacle_navigation') {
                $side = [Math]::Sign([double]$trace.steps[0].physical.displacement.z)
                if ($side -eq 0) { throw "Obstacle lesson $name has no lateral route." }
                [void]$obstacleSides.Add($side)
            } else {
                $before = [double]$trace.steps[0].physiology.before_fatigue
                $after = [double]$trace.steps[-1].physiology.after_fatigue
                $families = @($trace.steps | ForEach-Object { $_.chosen_family })
                $restCount = @($families | Where-Object { $_ -eq 'Rest' }).Count
                if ($before -lt 0.12 -or $after -ge $before - 0.02 -or
                    $restCount -lt 1 -or $families[0] -ne 'Rest' -or $families[-1] -ne 'Ingest' -or
                    @($families[$restCount..($families.Count - 2)] | Where-Object { $_ -ne 'Approach' }).Count -ne 0 -or
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
    if (@($counts.Values | Where-Object { $_ -ne 8 }).Count) { throw 'Lesson corpus is not balanced.' }
    if ($obstacleSides.Count -ne 2 -or $recoveryFatigueLevels.Count -lt 3) {
        throw 'Lesson corpus lacks both obstacle sides or varied biological fatigue.'
    }
    $manifest = [ordered]@{ founder_seed_base = $FounderSeedBase; pilots = @($entries) }
    [IO.File]::WriteAllText((Join-Path $outputPath 'manifest.json'),
        ($manifest | ConvertTo-Json -Depth 4), [Text.UTF8Encoding]::new($false))
    Write-Output "Balanced manifest: $(Join-Path $outputPath 'manifest.json')"
} finally {
    $lock.Dispose()
}
