[CmdletBinding()]
param(
    [ValidateSet('plan', 'execute')]
    [string]$Mode = 'plan',
    [ValidateSet('screen', 'confirm', 'certify')]
    [string]$Stage = 'screen',
    [string]$ScenarioCatalogPath = '',
    [string]$ManifestSchemaPath = '',
    [string[]]$ScenarioId = @(),
    [ValidateSet('N512', 'N1024', 'N2048')]
    [string]$BrainClass = 'N512',
    [ValidateRange(1, [int]::MaxValue)]
    [int]$Population = 1,
    [string]$HardwareToolchain = 'unconfigured',
    [string]$RunnerId = '',
    [string[]]$RunnerCommand = @(),
    [string]$ArtifactRoot = 'target/artifacts/pass2/ei1-ablation',
    [string]$CacheRoot = 'target/cache/pass2/ei1-ablation'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Stop-Ei1Ablation { param([string]$Message) throw "pass2_ei1_ablation: $Message" }
function Get-Sha256Text {
    param([string]$Text)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    try { (($sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($Text)) | ForEach-Object { $_.ToString('x2') }) -join '') }
    finally { $sha.Dispose() }
}
function Get-CanonicalJson { param($Value) ($Value | ConvertTo-Json -Compress -Depth 30) }
function Get-SourceCommit {
    $commit = (& git rev-parse HEAD 2>$null | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $commit -notmatch '^[0-9a-f]{40}$') { Stop-Ei1Ablation 'git rev-parse HEAD did not return a lower-case commit SHA' }
    $commit
}
function Get-Mechanisms {
    param($Configuration)
    [ordered]@{
        attention = [bool]$Configuration.attention; concept_gap_context = [bool]$Configuration.concept_gap_context
        prediction = [bool]$Configuration.prediction; dendritic_conjunctions = [bool]$Configuration.dendritic_conjunctions
        structural_plasticity = [bool]$Configuration.structural_plasticity; weight_learning = [bool]$Configuration.weight_learning
        sleep_consolidation = [bool]$Configuration.sleep_consolidation; episodic_recall = [bool]$Configuration.episodic_recall
    }
}
function Assert-ManifestIdentity {
    param($Actual, $Expected)
    foreach ($field in @('schema_version', 'source_commit', 'configuration_hash', 'scenario_corpus_version', 'brain_class', 'population', 'stage')) {
        if ([string]$Actual.$field -ne [string]$Expected.$field) { Stop-Ei1Ablation "runner output does not match planned manifest field $field" }
    }
    if ((Get-CanonicalJson $Actual.seed_set) -ne (Get-CanonicalJson $Expected.seed_set)) { Stop-Ei1Ablation 'runner output does not match planned seed set' }
    if ((Get-CanonicalJson $Actual.mechanisms) -ne (Get-CanonicalJson $Expected.mechanisms)) { Stop-Ei1Ablation 'runner output does not match planned mechanism configuration' }
    if ((Get-CanonicalJson $Actual.command) -ne (Get-CanonicalJson $Expected.command)) { Stop-Ei1Ablation 'runner output does not match planned runner identity or arguments' }
    if ((Get-CanonicalJson $Actual.artifacts) -ne (Get-CanonicalJson $Expected.artifacts)) { Stop-Ei1Ablation 'runner output does not bind the planned artifact paths' }
    if ((Get-CanonicalJson $Actual.hardware_toolchain) -ne (Get-CanonicalJson $Expected.hardware_toolchain)) { Stop-Ei1Ablation 'runner output does not match planned hardware or toolchain identity' }
    if ((Get-CanonicalJson $Actual.cache_lineage) -ne (Get-CanonicalJson $Expected.cache_lineage)) { Stop-Ei1Ablation 'runner output does not match planned cache lineage' }
}

if ([string]::IsNullOrWhiteSpace($ScenarioCatalogPath)) { $ScenarioCatalogPath = Join-Path $PSScriptRoot 'pass2_ei1_scenarios.json' }
if ([string]::IsNullOrWhiteSpace($ManifestSchemaPath)) { $ManifestSchemaPath = Join-Path $PSScriptRoot 'pass2_experiment_manifest.schema.json' }
foreach ($path in @($ScenarioCatalogPath, $ManifestSchemaPath)) { if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { Stop-Ei1Ablation "required contract is missing: $path" } }
$catalog = Get-Content -Raw -LiteralPath $ScenarioCatalogPath | ConvertFrom-Json
$schema = Get-Content -Raw -LiteralPath $ManifestSchemaPath | ConvertFrom-Json
if ($catalog.execution_state -ne 'not_run' -or $schema.properties.schema_version.const -ne 'pass2-experiment-manifest-v1') { Stop-Ei1Ablation 'unexpected EI1 catalog or manifest schema version' }
$policy = $catalog.stage_policy.$Stage
$seedSet = @($catalog.common_seed_sets | Where-Object { $_.identity -eq $policy.seed_set_identity })
if ($seedSet.Count -ne 1 -or $seedSet[0].intended_stage -ne $Stage) { Stop-Ei1Ablation "stage $Stage does not resolve exactly one matching common seed set" }
if ($Stage -eq 'confirm' -and $ScenarioId.Count -eq 0) { Stop-Ei1Ablation 'confirm requires explicit -ScenarioId values selected from Screen evidence; it cannot expand implicitly' }
$selected = @($catalog.scenarios)
if ($ScenarioId.Count -gt 0) {
    $selected = @($catalog.scenarios | Where-Object { $ScenarioId -contains $_.id })
    if ($selected.Count -ne $ScenarioId.Count) { Stop-Ei1Ablation 'one or more requested scenario IDs are absent from the catalog' }
}

$sourceCommit = Get-SourceCommit
$runnerIdentity = if ([string]::IsNullOrWhiteSpace($RunnerId)) { [string]$catalog.run_contract.runner_id } else { $RunnerId.Trim() }
$runnerArgs = if ($RunnerCommand.Count -gt 1) { @($RunnerCommand[1..($RunnerCommand.Count - 1)]) } else { @() }
$runnerForIdentity = if ($RunnerCommand.Count -eq 0) { 'runner-not-configured' } else { $RunnerCommand -join "`0" }
$artifactRootAbsolute = [IO.Path]::GetFullPath($ArtifactRoot)
$cacheRootAbsolute = [IO.Path]::GetFullPath($CacheRoot)
$manifestSchemaAbsolute = [IO.Path]::GetFullPath($ManifestSchemaPath)
$catalogAbsolute = [IO.Path]::GetFullPath($ScenarioCatalogPath)
$manifestRows = [Collections.Generic.List[object]]::new()

foreach ($scenario in $selected) {
    $required = @($scenario.required_mechanisms)
    $configs = @($catalog.mechanism_configurations | Where-Object {
        $candidate = $_
        $candidate.id -eq 'full_system' -or (@($required | Where-Object { -not [bool]$candidate.PSObject.Properties[$_].Value }).Count -gt 0)
    })
    foreach ($configuration in $configs) {
        $mechanisms = Get-Mechanisms $configuration
        $configurationMaterial = [ordered]@{
            format = 'pass2-ei1-ablation-configuration-v1'; mechanism_configuration = $mechanisms
            scenario_id = [string]$scenario.id; scenario_corpus_version = [string]$scenario.corpus_version
            seed_set_identity = [string]$seedSet[0].identity; seeds = @($seedSet[0].seeds)
            brain_class = $BrainClass; population = $Population; hardware_toolchain = $HardwareToolchain
            runner_id = $runnerIdentity; runner_command = $runnerForIdentity; runner_arguments = $runnerArgs
            manifest_schema_version = 'pass2-experiment-manifest-v1'
        }
        $configurationHash = Get-Sha256Text (Get-CanonicalJson $configurationMaterial)
        $identityMaterial = [ordered]@{ source_commit = $sourceCommit; canonical_mechanism_configuration = $mechanisms; scenario_corpus_version = [string]$scenario.corpus_version; seed_set_identity = [string]$seedSet[0].identity; seeds = @($seedSet[0].seeds); brain_class = $BrainClass; population = $Population; hardware_toolchain = $HardwareToolchain; runner_id = $runnerIdentity; runner_command = $runnerForIdentity; runner_arguments = $runnerArgs; schema_version = 'pass2-experiment-manifest-v1' }
        $cacheKey = Get-Sha256Text (Get-CanonicalJson $identityMaterial)
        $receiptPath = Join-Path $artifactRootAbsolute "$cacheKey.receipt.json"
        $manifest = [ordered]@{
            schema_version = 'pass2-experiment-manifest-v1'; source_commit = $sourceCommit; configuration_hash = $configurationHash
            scenario_corpus_version = [string]$scenario.corpus_version; seed_set = [ordered]@{ identity = [string]$seedSet[0].identity; seeds = @($seedSet[0].seeds) }
            brain_class = $BrainClass; population = $Population; hardware_toolchain = [ordered]@{ identity = $HardwareToolchain }
            mechanisms = $mechanisms; stage = $Stage
            command = [ordered]@{ runner_id = $runnerIdentity; arguments = @($runnerArgs + @('--scenario-id', [string]$scenario.id, '--configuration-id', [string]$configuration.id)) }
            artifacts = [ordered]@{ raw_output_path = (Join-Path $artifactRootAbsolute "$cacheKey.raw.jsonl"); receipt_path = $receiptPath }
            metrics = [ordered]@{ capability = @([string]$scenario.capability_metric); cognitive_work_fields = @($catalog.run_contract.cognitive_work_fields) }
            outcome = [ordered]@{ status = if ($scenario.support -eq 'available') { 'planned' } else { 'unavailable' }; execution_state = 'not_run' }
            cache_lineage = [ordered]@{ source_digest = $sourceCommit; configuration_hash = $configurationHash; parent_receipt_path = $null }
        }
        $manifestRows.Add([ordered]@{ scenario_id = [string]$scenario.id; configuration_id = [string]$configuration.id; support = [string]$scenario.support; unavailable_reason = if ($scenario.support -eq 'available') { $null } else { [string]$scenario.unavailable_reason }; cache_key = $cacheKey; manifest = $manifest })
    }
}

if ($Mode -eq 'execute') {
    if ([string]::IsNullOrWhiteSpace($RunnerId) -or $RunnerCommand.Count -eq 0) { Stop-Ei1Ablation 'execute requires an explicit real -RunnerId and -RunnerCommand. The catalog runner ID is a contract label, not an executable evaluator.' }
    $resolvedRunner = Get-Command $RunnerCommand[0] -ErrorAction SilentlyContinue
    if ($null -eq $resolvedRunner -or $resolvedRunner.CommandType -notin @('Application', 'ExternalScript')) { Stop-Ei1Ablation "configured runner is not an executable or script: $($RunnerCommand[0])" }
    if (@($manifestRows | Where-Object { $_.support -ne 'available' }).Count -gt 0) { Stop-Ei1Ablation 'execute is unavailable because the selected catalog scenarios have no real Task 3 evaluator' }
    New-Item -ItemType Directory -Force -Path $artifactRootAbsolute, $cacheRootAbsolute | Out-Null
    foreach ($row in $manifestRows) {
        $plannedPath = Join-Path $artifactRootAbsolute "$($row.cache_key).planned.json"
        $row.manifest | ConvertTo-Json -Depth 30 | Set-Content -LiteralPath $plannedPath -Encoding utf8
        $output = (& $RunnerCommand[0] @($runnerArgs + @('--planned-manifest', $plannedPath, '--manifest-identity', $row.cache_key)) 2>&1 | Out-String)
        if ($LASTEXITCODE -ne 0) { Stop-Ei1Ablation "runner $RunnerId failed for $($row.scenario_id)/$($row.configuration_id): $output" }
        if (-not (Test-Path -LiteralPath $row.manifest.artifacts.receipt_path -PathType Leaf)) { Stop-Ei1Ablation "runner $RunnerId did not produce the requested receipt" }
        $actual = Get-Content -Raw -LiteralPath $row.manifest.artifacts.receipt_path | ConvertFrom-Json
        Assert-ManifestIdentity $actual $row.manifest
        if ($actual.outcome.execution_state -ne 'executed' -or $actual.outcome.status -eq 'planned') { Stop-Ei1Ablation 'runner receipt does not prove execution' }
        if ($actual.outcome.status -in @('causal_failure', 'invalid', 'diverged', 'evaluator_corrupt')) { Stop-Ei1Ablation "runner reported terminal failure: $($actual.outcome.status)" }
        if ($actual.outcome.status -ne 'completed') { Stop-Ei1Ablation "runner returned unsupported executed status: $($actual.outcome.status)" }
        Copy-Item -LiteralPath $row.manifest.artifacts.receipt_path -Destination (Join-Path $cacheRootAbsolute "$($row.cache_key).receipt.json") -Force
    }
}

[ordered]@{
    schema_version = 'pass2-ei1-ablation-plan-v1'; mode = $Mode; execution_state = if ($Mode -eq 'plan') { 'not_run' } else { 'executed' }
    source_contracts = [ordered]@{ scenario_catalog = $catalogAbsolute; manifest_schema = $manifestSchemaAbsolute }
    selection = [ordered]@{ stage = $Stage; seed_set_identity = [string]$seedSet[0].identity; seeds = @($seedSet[0].seeds); screen = 'all catalog scenarios; full_system plus each ablation that disables a scenario-required mechanism'; confirm = 'explicit -ScenarioId only, selected from Screen effects, regressions, instabilities, or ambiguity'; certify = 'all catalog scenarios; full_system plus each ablation that disables a scenario-required mechanism' }
    runner = [ordered]@{ requested_id = if ([string]::IsNullOrWhiteSpace($RunnerId)) { $null } else { $RunnerId }; catalog_contract_id = [string]$catalog.run_contract.runner_id; configured = ($RunnerCommand.Count -gt 0); execution_boundary = 'No real Task 3 evaluator is declared by the current catalog. Execute remains unavailable until a real executable runner is supplied and the catalog marks selected scenarios available.' }
    manifests = @($manifestRows)
} | ConvertTo-Json -Depth 40
