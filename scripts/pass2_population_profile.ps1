[CmdletBinding()]
param(
    [ValidateSet('plan', 'execute')]
    [string]$Mode = 'plan',
    [string]$LadderPath = '',
    [string]$ScenarioContractPath = '',
    [string]$ManifestSchemaPath = '',
    [string]$ArtifactRoot = 'target/artifacts/pass2/population-profile',
    [string]$CacheRoot = 'target/cache/pass2/population-profile',
    [string]$SourceCommit = '',
    [string]$ConfigurationHash = '',
    [string]$ScenarioCorpusVersion = '',
    [string]$SeedSetIdentity = 'pass2-screen-common-v1',
    [string]$SeedSetHash = '',
    [ValidateSet('Nano512', 'N512', 'N1024', 'N2048')]
    [string]$BrainClass = 'N512',
    [string]$HardwareIdentity = 'unconfigured',
    [string]$Toolchain = 'unconfigured',
    [string]$RunnerId = '',
    [string[]]$RunnerCommand = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Stop-PopulationProfile {
    param([Parameter(Mandatory = $true)][string]$Message)
    throw "pass2_population_profile: $Message"
}

function Get-Sha256Text {
    param([Parameter(Mandatory = $true)][string]$Text)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        return (($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join '')
    } finally {
        $sha.Dispose()
    }
}

function Get-AbsolutePath {
    param([Parameter(Mandatory = $true)][string]$Path)
    return [System.IO.Path]::GetFullPath($Path)
}

function Get-SourceCommitValue {
    param([string]$Requested)
    if (-not [string]::IsNullOrWhiteSpace($Requested)) {
        return $Requested.Trim()
    }
    $value = (& git rev-parse HEAD 2>$null | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($value)) {
        Stop-PopulationProfile 'source commit is required and git rev-parse HEAD failed'
    }
    return $value
}

function Assert-Hash {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Value
    )
    if ($Value -notmatch '^[0-9a-f]{64}$') {
        Stop-PopulationProfile "$Name must be a lower-case 64-character SHA-256 value"
    }
}

function New-EmptyMetrics {
    $metric = { param([string[]]$Names) $result = [ordered]@{}; foreach ($name in $Names) { $result[$name] = $null }; $result }
    return [ordered]@{
        timing_ms = & $metric @('total_tick', 'total_frame')
        gpu_time_ms = & $metric @('recurrent', 'dendritic', 'prediction', 'motor', 'learning', 'sleep', 'structural')
        cpu_time_ms = & $metric @('perception', 'attention', 'memory', 'topology', 'lifecycle', 'persistence_staging', 'presentation')
        transfer = & $metric @('upload_bytes', 'readback_bytes', 'upload_stalls', 'readback_stalls')
        work = & $metric @('evaluated_neural_work', 'evaluated_cognitive_work')
        attention = & $metric @('attention_count')
        structural = & $metric @('candidate_count', 'ranking_work', 'edit_work', 'recompaction_work')
        sleep = & $metric @('burst_frequency_hz', 'worst_case_latency_ms')
        residency = & $metric @('memory_bytes', 'buffer_residency_bytes')
        lifecycle = & $metric @('admission_ms', 'retirement_ms', 'save_load_ms')
        budget = & $metric @('throttled_frames', 'missed_budgets', 'causal_failures', 'all_pairs_or_all_branches_regression')
    }
}

function New-Identity {
    param(
        [Parameter(Mandatory = $true)][int]$Population,
        [Parameter(Mandatory = $true)][string]$ReceiptSchemaVersion,
        [Parameter(Mandatory = $true)][string]$SourceCommitValue,
        [Parameter(Mandatory = $true)][string]$ConfigurationHashValue,
        [Parameter(Mandatory = $true)][string]$ScenarioVersion,
        [Parameter(Mandatory = $true)][string]$SeedHashValue,
        [Parameter(Mandatory = $true)][string]$BrainClassValue,
        [Parameter(Mandatory = $true)][string]$HardwareValue,
        [Parameter(Mandatory = $true)][string]$ToolchainValue
    )
    $identityMaterial = @(
        "schema_version=$ReceiptSchemaVersion"
        "source_commit=$SourceCommitValue"
        "configuration_hash=$ConfigurationHashValue"
        "scenario_corpus_version=$ScenarioVersion"
        "seed_set_hash=$SeedHashValue"
        "brain_class=$BrainClassValue"
        "population=$Population"
        "hardware_identity=$HardwareValue"
        "toolchain=$ToolchainValue"
    ) -join "`n"
    $cacheKey = Get-Sha256Text $identityMaterial
    return [ordered]@{
        source_commit = $SourceCommitValue
        configuration_hash = $ConfigurationHashValue
        scenario_corpus_version = $ScenarioVersion
        seed_set_hash = $SeedHashValue
        brain_class = $BrainClassValue
        population = $Population
        hardware_identity = $HardwareValue
        toolchain = $ToolchainValue
        schema_version = $ReceiptSchemaVersion
        cache_key = $cacheKey
    }
}

function Get-CacheLookup {
    param(
        [Parameter(Mandatory = $true)]$Identity,
        [Parameter(Mandatory = $true)][string]$CacheReceiptPath
    )
    if (-not (Test-Path -LiteralPath $CacheReceiptPath -PathType Leaf)) {
        return [ordered]@{ lookup = 'miss'; parent_receipt_path = $null; receipt = $null }
    }
    try {
        $candidate = Get-Content -Raw -LiteralPath $CacheReceiptPath | ConvertFrom-Json
        if ($candidate.schema_version -ne 'pass2-population-profile-receipt-v1' -or $candidate.receipt_kind -ne 'population_profile') {
            return [ordered]@{ lookup = 'invalid'; parent_receipt_path = $null; receipt = $null }
        }
        $fields = @(
            'source_commit',
            'configuration_hash',
            'scenario_corpus_version',
            'seed_set_hash',
            'brain_class',
            'population',
            'hardware_identity',
            'toolchain',
            'schema_version',
            'cache_key'
        )
        foreach ($field in $fields) {
            if ([string]$candidate.identity.$field -ne [string]$Identity.$field) {
                return [ordered]@{ lookup = 'invalid'; parent_receipt_path = $null; receipt = $null }
            }
        }
        if ($candidate.execution.state -ne 'executed' -or $candidate.execution.status -eq 'planned') {
            return [ordered]@{ lookup = 'invalid'; parent_receipt_path = $null; receipt = $null }
        }
        return [ordered]@{
            lookup = 'hit'
            parent_receipt_path = $CacheReceiptPath
            receipt = $candidate
        }
    } catch {
        return [ordered]@{ lookup = 'invalid'; parent_receipt_path = $null; receipt = $null }
    }
}

function New-Receipt {
    param(
        [Parameter(Mandatory = $true)]$Entry,
        [Parameter(Mandatory = $true)]$Identity,
        [Parameter(Mandatory = $true)]$Artifacts,
        [Parameter(Mandatory = $true)]$Lookup,
        [Parameter(Mandatory = $true)][string]$ReceiptSchemaVersion,
        [Parameter(Mandatory = $true)][string]$PlannedRunnerId
    )
    $nextPopulation = if ($null -eq $Entry.next_population) { $null } else { [int]$Entry.next_population }
    return [ordered]@{
        schema_version = $ReceiptSchemaVersion
        receipt_kind = 'population_profile'
        execution = [ordered]@{
            mode = 'plan'
            state = 'not_run'
            status = 'planned'
            runner_id = $PlannedRunnerId
        }
        identity = $Identity
        artifacts = $Artifacts
        cache_lineage = [ordered]@{
            cache_key = $Identity.cache_key
            lookup = $Lookup.lookup
            parent_receipt_path = $Lookup.parent_receipt_path
            origin_receipt_path = $null
        }
        metrics = New-EmptyMetrics
        decision = [ordered]@{
            action = 'planned'
            basis = @('planned')
            causal_correctness = 'not_evaluated'
            resource_headroom = 'not_evaluated'
            predicted_value = 'not_evaluated'
            severe_budget_breach = $null
            all_pairs_or_all_branches_regression = 'not_evaluated'
            specific_blocker = $null
            next_population = $nextPopulation
        }
    }
}

function Assert-ExecutedReceipt {
    param(
        [Parameter(Mandatory = $true)]$Receipt,
        [Parameter(Mandatory = $true)]$Identity,
        [Parameter(Mandatory = $true)]$Artifacts,
        [Parameter(Mandatory = $true)][string]$ReceiptSchemaVersion,
        [Parameter(Mandatory = $true)]$Entry
    )
    if ($Receipt.schema_version -ne $ReceiptSchemaVersion -or $Receipt.receipt_kind -ne 'population_profile') {
        Stop-PopulationProfile "runner receipt for population $($Entry.population) has the wrong schema or kind"
    }
    foreach ($field in @('source_commit', 'configuration_hash', 'scenario_corpus_version', 'seed_set_hash', 'brain_class', 'population', 'hardware_identity', 'toolchain', 'schema_version', 'cache_key')) {
        if ([string]$Receipt.identity.$field -ne [string]$Identity.$field) {
            Stop-PopulationProfile "runner receipt for population $($Entry.population) does not match identity field $field"
        }
    }
    if ($Receipt.execution.state -ne 'executed' -or $Receipt.execution.status -eq 'planned') {
        Stop-PopulationProfile "runner receipt for population $($Entry.population) does not prove execution"
    }
    if ($Receipt.artifacts.raw_output_path -ne $Artifacts.raw_output_path -or $Receipt.artifacts.receipt_path -ne $Artifacts.receipt_path -or $Receipt.artifacts.cache_receipt_path -ne $Artifacts.cache_receipt_path) {
        Stop-PopulationProfile "runner receipt for population $($Entry.population) does not bind the requested artifact paths"
    }
    if ($Receipt.cache_lineage.cache_key -ne $Identity.cache_key -or $Receipt.cache_lineage.lookup -ne 'miss') {
        Stop-PopulationProfile "runner receipt for population $($Entry.population) has invalid cache lineage"
    }
    if ($Receipt.decision.action -notin @('advance', 'stop')) {
        Stop-PopulationProfile "runner receipt for population $($Entry.population) lacks an explicit advance/stop decision"
    }
    if ($Receipt.decision.action -eq 'stop' -and $null -eq $Receipt.decision.specific_blocker -and $Receipt.decision.basis -notcontains 'causal_correctness') {
        Stop-PopulationProfile "runner stop decision for population $($Entry.population) lacks a blocker or causal basis"
    }
}

if ([string]::IsNullOrWhiteSpace($LadderPath)) { $LadderPath = Join-Path $PSScriptRoot 'pass2_population_ladder.json' }
if ([string]::IsNullOrWhiteSpace($ScenarioContractPath)) { $ScenarioContractPath = Join-Path $PSScriptRoot 'pass2_ei1_scenarios.json' }
if ([string]::IsNullOrWhiteSpace($ManifestSchemaPath)) { $ManifestSchemaPath = Join-Path $PSScriptRoot 'pass2_experiment_manifest.schema.json' }

foreach ($contractPath in @($LadderPath, $ScenarioContractPath, $ManifestSchemaPath)) {
    if (-not (Test-Path -LiteralPath $contractPath -PathType Leaf)) {
        Stop-PopulationProfile "required contract is missing: $contractPath"
    }
}

$ladder = Get-Content -Raw -LiteralPath $LadderPath | ConvertFrom-Json
$scenarioContract = Get-Content -Raw -LiteralPath $ScenarioContractPath | ConvertFrom-Json
$manifestSchema = Get-Content -Raw -LiteralPath $ManifestSchemaPath | ConvertFrom-Json
if ($manifestSchema.properties.schema_version.const -ne 'pass2-experiment-manifest-v1') {
    Stop-PopulationProfile 'the existing experiment manifest contract is not pass2-experiment-manifest-v1'
}
if ($ladder.receipt_schema -ne 'scripts/pass2_population_receipt.schema.json') {
    Stop-PopulationProfile 'ladder points at an unexpected receipt schema'
}
$receiptSchemaVersion = 'pass2-population-profile-receipt-v1'
$entries = @($ladder.ladder)
if (($entries.population -join ',') -ne '1,10,30,100,500') {
    Stop-PopulationProfile 'ladder must be exactly 1,10,30,100,500'
}

$seedMatches = @($scenarioContract.common_seed_sets | Where-Object { $_.identity -eq $SeedSetIdentity })
if ($seedMatches.Count -ne 1) {
    Stop-PopulationProfile "seed set identity is not present exactly once in the scenario contract: $SeedSetIdentity"
}
$seedSet = $seedMatches[0]
$scenarioVersion = if ([string]::IsNullOrWhiteSpace($ScenarioCorpusVersion)) { [string]$scenarioContract.catalog_version } else { $ScenarioCorpusVersion.Trim() }
$seedText = ($seedSet.seeds | ForEach-Object { [string]$_ }) -join ','
$derivedSeedHash = Get-Sha256Text ("$SeedSetIdentity|$seedText")
$seedHash = if ([string]::IsNullOrWhiteSpace($SeedSetHash)) { $derivedSeedHash } else { $SeedSetHash.Trim() }
Assert-Hash 'seed_set_hash' $seedHash
$source = Get-SourceCommitValue $SourceCommit
if ($source -notmatch '^[0-9a-f]{40}$') {
    Stop-PopulationProfile 'source commit must be a lower-case 40-character commit SHA'
}

$plannedRunnerId = [string]$ladder.runner_contract.required_later_runner.runner_id
$configRunnerId = if ([string]::IsNullOrWhiteSpace($RunnerId)) { $plannedRunnerId } else { $RunnerId.Trim() }
$configCommand = if ($RunnerCommand.Count -eq 0) { 'runner-not-configured' } else { $RunnerCommand -join "`0" }
$configMaterial = @(
    'pass2-population-profile-configuration-v1'
    "ladder_schema=$($ladder.schema_version)"
    "runner_id=$configRunnerId"
    "runner_command=$configCommand"
    "scenario_corpus_version=$scenarioVersion"
    "seed_set_identity=$SeedSetIdentity"
    "brain_class=$BrainClass"
    'populations=1,10,30,100,500'
) -join "`n"
$computedConfigurationHash = Get-Sha256Text $configMaterial
$configuration = if ([string]::IsNullOrWhiteSpace($ConfigurationHash)) { $computedConfigurationHash } else { $ConfigurationHash.Trim() }
Assert-Hash 'configuration_hash' $configuration

$artifactRootAbsolute = Get-AbsolutePath $ArtifactRoot
$cacheRootAbsolute = Get-AbsolutePath $CacheRoot
if ($Mode -eq 'execute') {
    if ([string]::IsNullOrWhiteSpace($RunnerId) -or $RunnerCommand.Count -eq 0) {
        Stop-PopulationProfile 'execute requested without a real configured runner. Supply -RunnerId and -RunnerCommand. No profile was run.'
    }
    $resolvedRunner = Get-Command $RunnerCommand[0] -ErrorAction SilentlyContinue
    if ($null -eq $resolvedRunner -or $resolvedRunner.CommandType -notin @('Application', 'ExternalScript')) {
        Stop-PopulationProfile "configured runner is not an executable or script: $($RunnerCommand[0]). No profile was run."
    }
}

$receipts = [System.Collections.Generic.List[object]]::new()
$stopped = $false
foreach ($entry in $entries) {
    if ($stopped) { break }
    $identity = New-Identity -Population ([int]$entry.population) -ReceiptSchemaVersion $receiptSchemaVersion -SourceCommitValue $source -ConfigurationHashValue $configuration -ScenarioVersion $scenarioVersion -SeedHashValue $seedHash -BrainClassValue $BrainClass -HardwareValue $HardwareIdentity -ToolchainValue $Toolchain
    $receiptPath = Join-Path $artifactRootAbsolute "$($identity.cache_key).receipt.json"
    $rawOutputPath = Join-Path $artifactRootAbsolute "$($identity.cache_key).raw.jsonl"
    $cacheReceiptPath = Join-Path $cacheRootAbsolute "$($identity.cache_key).receipt.json"
    $artifacts = [ordered]@{
        raw_output_path = $rawOutputPath
        receipt_path = $receiptPath
        cache_receipt_path = $cacheReceiptPath
    }
    $lookup = Get-CacheLookup -Identity $identity -CacheReceiptPath $cacheReceiptPath
    if ($lookup.lookup -eq 'hit') {
        $receipts.Add($lookup.receipt)
        if ($lookup.receipt.decision.action -eq 'stop') { $stopped = $true }
        continue
    }
    if ($Mode -eq 'plan') {
        $receipts.Add((New-Receipt -Entry $entry -Identity $identity -Artifacts $artifacts -Lookup $lookup -ReceiptSchemaVersion $receiptSchemaVersion -PlannedRunnerId $plannedRunnerId))
        continue
    }

    New-Item -ItemType Directory -Force -Path $artifactRootAbsolute, $cacheRootAbsolute | Out-Null
    $runnerArgs = @()
    if ($RunnerCommand.Count -gt 1) { $runnerArgs += $RunnerCommand[1..($RunnerCommand.Count - 1)] }
    $runnerArgs += @(
        '--population', [string]$entry.population,
        '--brain-class', $BrainClass,
        '--source-commit', $source,
        '--configuration-hash', $configuration,
        '--scenario-corpus-version', $scenarioVersion,
        '--seed-set-hash', $seedHash,
        '--hardware-identity', $HardwareIdentity,
        '--toolchain', $Toolchain,
        '--receipt-schema', (Get-AbsolutePath (Join-Path $PSScriptRoot 'pass2_population_receipt.schema.json')),
        '--raw-output', $rawOutputPath,
        '--receipt-output', $receiptPath,
        '--cache-key', $identity.cache_key
    )
    $runnerOutput = (& $RunnerCommand[0] @runnerArgs 2>&1 | Out-String)
    $runnerExit = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
    if ($runnerExit -ne 0) {
        Stop-PopulationProfile "runner $RunnerId failed for population $($entry.population) with exit code $runnerExit. No receipt was accepted. Output: $runnerOutput"
    }
    if (-not (Test-Path -LiteralPath $receiptPath -PathType Leaf)) {
        Stop-PopulationProfile "runner $RunnerId completed without the requested receipt for population $($entry.population). No profile was accepted."
    }
    if (-not (Test-Path -LiteralPath $rawOutputPath -PathType Leaf)) {
        [System.IO.File]::WriteAllText($rawOutputPath, $runnerOutput)
    }
    $executedReceipt = Get-Content -Raw -LiteralPath $receiptPath | ConvertFrom-Json
    Assert-ExecutedReceipt -Receipt $executedReceipt -Identity $identity -Artifacts $artifacts -ReceiptSchemaVersion $receiptSchemaVersion -Entry $entry
    Copy-Item -LiteralPath $receiptPath -Destination $cacheReceiptPath -Force
    $receipts.Add($executedReceipt)
    if ($executedReceipt.decision.action -eq 'stop') { $stopped = $true }
}

$report = [ordered]@{
    schema_version = 'pass2-population-profile-run-v1'
    receipt_schema_version = $receiptSchemaVersion
    mode = $Mode
    execution_state = if ($Mode -eq 'plan') { 'not_run' } else { 'executed' }
    source_contracts = [ordered]@{
        ladder = (Get-AbsolutePath $LadderPath)
        scenario_catalog = (Get-AbsolutePath $ScenarioContractPath)
        experiment_manifest_schema = (Get-AbsolutePath $ManifestSchemaPath)
    }
    runner = [ordered]@{
        requested_id = if ([string]::IsNullOrWhiteSpace($RunnerId)) { $null } else { $RunnerId }
        configured = ($Mode -eq 'execute')
        known_existing_entrypoint = $ladder.runner_contract.existing_entrypoint.runner_id
        required_later_runner = $plannedRunnerId
    }
    ladder = @($entries.population)
    profiles = @($receipts)
}
$report | ConvertTo-Json -Depth 30

