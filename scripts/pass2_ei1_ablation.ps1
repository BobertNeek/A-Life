[CmdletBinding()]
param(
    [ValidateSet('plan', 'execute')]
    [string]$Mode = 'plan',

    [ValidateSet('screen', 'confirm', 'certify')]
    [string]$Stage = 'screen',

    [string]$ScenarioContractPath = '',
    [string]$ManifestSchemaPath = '',
    [string]$ArtifactRoot = 'target/artifacts/pass2/ei1-ablation',
    [string]$CacheRoot = 'target/cache/pass2/ei1-ablation',
    [string]$SourceCommit = '',

    [Alias('ScenarioId')]
    [string[]]$ScenarioIds = @(),

    [ValidateRange(1, [int]::MaxValue)]
    [int]$Population = 1,

    [ValidateSet('Nano512', 'N512', 'N1024', 'N2048')]
    [string]$BrainClass = 'N512',

    [string]$HardwareIdentity = '',
    [string]$Toolchain = '',
    [string]$AdapterIdentity = '',
    [string]$DriverIdentity = '',
    [string]$OsIdentity = '',
    [string]$RustcIdentity = '',
    [string]$WgpuIdentity = '',

    [string]$RunnerId = '',
    [string[]]$RunnerCommand = @()
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Root = Split-Path -Parent $PSScriptRoot
$ManifestSchemaVersion = 'pass2-experiment-manifest-v1'
$PlanSchemaVersion = 'pass2-ei1-ablation-plan-v1'
$ScriptName = 'pass2_ei1_ablation'

function Stop-Ei1Ablation {
    param([Parameter(Mandatory = $true)][string]$Message)
    throw "$ScriptName`: $Message"
}

function Get-PropertyValue {
    param(
        [Parameter(Mandatory = $false)]$Object,
        [Parameter(Mandatory = $true)][string]$Name
    )

    if ($null -eq $Object) {
        return $null
    }
    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Get-Sha256Text {
    param([Parameter(Mandatory = $true)][string]$Text)

    $sha = [System.Security.Cryptography.SHA256]::Create()
    try {
        $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
        return (($sha.ComputeHash($bytes) | ForEach-Object { $_.ToString('x2') }) -join '')
    }
    finally {
        $sha.Dispose()
    }
}

function ConvertTo-CanonicalJson {
    param([Parameter(Mandatory = $true)]$Object)
    return ($Object | ConvertTo-Json -Depth 30 -Compress)
}

function Get-AbsolutePath {
    param([Parameter(Mandatory = $true)][string]$Path)

    if ([System.IO.Path]::IsPathRooted($Path)) {
        return [System.IO.Path]::GetFullPath($Path)
    }
    return [System.IO.Path]::GetFullPath((Join-Path $Root $Path))
}

function Get-SourceCommitValue {
    if (-not [string]::IsNullOrWhiteSpace($SourceCommit)) {
        $value = $SourceCommit.Trim()
    }
    else {
        $value = (& git rev-parse HEAD 2>$null | Out-String).Trim()
        if ($LASTEXITCODE -ne 0) {
            Stop-Ei1Ablation 'source commit is required and git rev-parse HEAD failed'
        }
    }

    if ($value -notmatch '^[0-9a-f]{40}$') {
        Stop-Ei1Ablation 'source commit must be a lower-case 40-character commit SHA'
    }
    return $value
}

function Assert-ContractFile {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-Ei1Ablation "required contract is missing: $Path"
    }
    try {
        return (Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json)
    }
    catch {
        Stop-Ei1Ablation "required contract is not valid JSON: $Path"
    }
}

function Assert-ManifestSchema {
    param([Parameter(Mandatory = $true)]$Schema)

    $schemaVersion = [string](Get-PropertyValue $Schema.properties.schema_version 'const')
    if ($schemaVersion -ne $ManifestSchemaVersion) {
        Stop-Ei1Ablation "unexpected manifest schema version: $schemaVersion"
    }

    $required = @($Schema.required | ForEach-Object { [string]$_ })
    if ($required.Count -eq 0) {
        Stop-Ei1Ablation 'manifest schema has no required fields'
    }
    return $required
}

function Assert-ScenarioCatalog {
    param(
        [Parameter(Mandatory = $true)]$Catalog,
        [Parameter(Mandatory = $true)][string]$ScenarioPath,
        [Parameter(Mandatory = $true)][string[]]$MechanismNames
    )

    if ([string]::IsNullOrWhiteSpace([string]$Catalog.catalog_version)) {
        Stop-Ei1Ablation 'scenario catalog has no catalog_version'
    }
    if ([string]$Catalog.experiment_manifest_schema -notmatch 'pass2_experiment_manifest\.schema\.json$') {
        Stop-Ei1Ablation 'scenario catalog points at an unexpected manifest schema'
    }
    if ([string]$Catalog.execution_state -ne 'not_run') {
        Stop-Ei1Ablation 'scenario catalog must remain not_run'
    }
    if (@($Catalog.common_seed_sets).Count -eq 0) {
        Stop-Ei1Ablation 'scenario catalog has no common seed sets'
    }
    if ([string]::IsNullOrWhiteSpace([string]$Catalog.run_contract.runner_id)) {
        Stop-Ei1Ablation 'scenario catalog has no runner identifier'
    }

    $configurationIds = @{}
    foreach ($configuration in @($Catalog.mechanism_configurations)) {
        $configurationId = [string]$configuration.id
        if ([string]::IsNullOrWhiteSpace($configurationId) -or $configurationIds.ContainsKey($configurationId)) {
            Stop-Ei1Ablation 'scenario catalog has duplicate or empty mechanism configuration IDs'
        }
        $configurationIds[$configurationId] = $true
        foreach ($mechanismName in $MechanismNames) {
            $value = Get-PropertyValue $configuration $mechanismName
            if ($null -eq $value -or $value -isnot [bool]) {
                Stop-Ei1Ablation "mechanism configuration $configurationId lacks boolean $mechanismName"
            }
        }
    }

    if (-not $configurationIds.ContainsKey('full_system')) {
        Stop-Ei1Ablation 'scenario catalog has no full_system configuration'
    }

    $scenarioIds = @{}
    foreach ($scenario in @($Catalog.scenarios)) {
        $scenarioId = [string]$scenario.id
        if ([string]::IsNullOrWhiteSpace($scenarioId) -or $scenarioIds.ContainsKey($scenarioId)) {
            Stop-Ei1Ablation 'scenario catalog has duplicate or empty scenario IDs'
        }
        $scenarioIds[$scenarioId] = $true
        if ([string]::IsNullOrWhiteSpace([string]$scenario.corpus_version)) {
            Stop-Ei1Ablation "scenario $scenarioId has no corpus version"
        }
        if ([string]::IsNullOrWhiteSpace([string]$scenario.support)) {
            Stop-Ei1Ablation "scenario $scenarioId has no support state"
        }
        if ([string]$scenario.support -eq 'unavailable' -and [string]::IsNullOrWhiteSpace([string]$scenario.unavailable_reason)) {
            Stop-Ei1Ablation "unsupported scenario $scenarioId has no unavailable_reason"
        }
        foreach ($requiredMechanism in @($scenario.required_mechanisms)) {
            if ($requiredMechanism -notin $MechanismNames) {
                Stop-Ei1Ablation "scenario $scenarioId requires unknown mechanism $requiredMechanism"
            }
        }
    }
}

function Get-StageDefinition {
    param([Parameter(Mandatory = $true)]$Catalog)

    $stageDefinition = Get-PropertyValue $Catalog.stage_policy $Stage
    if ($null -eq $stageDefinition) {
        Stop-Ei1Ablation "scenario catalog has no stage policy for $Stage"
    }
    $seedSetIdentity = [string]$stageDefinition.seed_set_identity
    $seedSets = @($Catalog.common_seed_sets | Where-Object { [string]$_.identity -eq $seedSetIdentity })
    if ($seedSets.Count -ne 1) {
        Stop-Ei1Ablation "stage $Stage does not resolve to exactly one common seed set"
    }
    if ([string]$seedSets[0].intended_stage -ne $Stage) {
        Stop-Ei1Ablation "seed set $seedSetIdentity is not intended for $Stage"
    }
    $seeds = @($seedSets[0].seeds | ForEach-Object { [int]$_ })
    if ($seeds.Count -lt 2 -or (@($seeds | Select-Object -Unique).Count -ne $seeds.Count)) {
        Stop-Ei1Ablation "seed set $seedSetIdentity is not a unique common seed set"
    }

    return [ordered]@{
        seed_set_identity = $seedSetIdentity
        seeds = $seeds
        scenario_scope = [string]$stageDefinition.scenario_scope
        configuration_scope = [string]$stageDefinition.configuration_scope
    }
}

function Get-HardwareToolchain {
    $hardwareFallback = if ([string]::IsNullOrWhiteSpace($HardwareIdentity)) { 'unconfigured' } else { $HardwareIdentity.Trim() }
    $toolchainFallback = if ([string]::IsNullOrWhiteSpace($Toolchain)) { 'unconfigured' } else { $Toolchain.Trim() }
    $resolveHardware = {
        param([string]$Value)
        if ([string]::IsNullOrWhiteSpace($Value)) { return $hardwareFallback }
        return $Value.Trim()
    }
    $resolveToolchain = {
        param([string]$Value)
        if ([string]::IsNullOrWhiteSpace($Value)) { return $toolchainFallback }
        return $Value.Trim()
    }

    return [ordered]@{
        adapter_identity = & $resolveHardware $AdapterIdentity
        driver_identity = & $resolveHardware $DriverIdentity
        os_identity = & $resolveHardware $OsIdentity
        rustc_identity = & $resolveToolchain $RustcIdentity
        wgpu_identity = & $resolveToolchain $WgpuIdentity
    }
}

function Get-MechanismObject {
    param(
        [Parameter(Mandatory = $true)]$Configuration,
        [Parameter(Mandatory = $true)][string[]]$MechanismNames
    )

    $mechanisms = [ordered]@{}
    foreach ($mechanismName in $MechanismNames) {
        $mechanisms[$mechanismName] = [bool](Get-PropertyValue $Configuration $mechanismName)
    }
    return $mechanisms
}

function Get-DisabledMechanism {
    param(
        [Parameter(Mandatory = $true)]$Configuration,
        [Parameter(Mandatory = $true)][string[]]$MechanismNames
    )

    $disabled = @($MechanismNames | Where-Object { -not [bool](Get-PropertyValue $Configuration $_) })
    if ([string]$Configuration.id -eq 'full_system') {
        if ($disabled.Count -ne 0) {
            Stop-Ei1Ablation 'full_system must enable every mechanism'
        }
        return $null
    }
    if ($disabled.Count -ne 1) {
        Stop-Ei1Ablation "mechanism configuration $($Configuration.id) must disable exactly one mechanism"
    }
    return [string]$disabled[0]
}

function Get-ConfigurationCatalog {
    param(
        [Parameter(Mandatory = $true)]$Catalog,
        [Parameter(Mandatory = $true)][string[]]$MechanismNames
    )

    $result = @()
    foreach ($configuration in @($Catalog.mechanism_configurations)) {
        $result += [pscustomobject][ordered]@{
            id = [string]$configuration.id
            mechanisms = Get-MechanismObject -Configuration $configuration -MechanismNames $MechanismNames
            disabled_mechanism = Get-DisabledMechanism -Configuration $configuration -MechanismNames $MechanismNames
        }
    }
    return $result
}

function Get-SelectedScenarios {
    param([Parameter(Mandatory = $true)]$Catalog)

    $requested = @($ScenarioIds | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Select-Object -Unique)
    $allScenarios = @($Catalog.scenarios)
    if ($Stage -eq 'confirm' -and $requested.Count -eq 0) {
        Stop-Ei1Ablation 'confirm requires explicit -ScenarioIds for effects, regressions, instabilities, or ambiguous results'
    }
    if ($requested.Count -eq 0) {
        return $allScenarios
    }

    $known = @($allScenarios | ForEach-Object { [string]$_.id })
    foreach ($scenarioId in $requested) {
        if ($scenarioId -notin $known) {
            Stop-Ei1Ablation "requested scenario is not in the catalog: $scenarioId"
        }
    }
    return @($allScenarios | Where-Object { [string]$_.id -in $requested })
}

function Get-RequiredConfigurations {
    param(
        [Parameter(Mandatory = $true)]$Scenario,
        [Parameter(Mandatory = $true)]$Configurations
    )

    $requiredMechanisms = @($Scenario.required_mechanisms | ForEach-Object { [string]$_ })
    $selected = @($Configurations | Where-Object {
        $_.id -eq 'full_system' -or $_.disabled_mechanism -in $requiredMechanisms
    })
    if (@($selected | Where-Object { $_.id -eq 'full_system' }).Count -ne 1) {
        Stop-Ei1Ablation "no unique full_system match for scenario $($Scenario.id)"
    }
    foreach ($requiredMechanism in $requiredMechanisms) {
        if (@($selected | Where-Object { $_.disabled_mechanism -eq $requiredMechanism }).Count -ne 1) {
            Stop-Ei1Ablation "no unique required ablation match for $requiredMechanism in scenario $($Scenario.id)"
        }
    }
    return $selected
}

function Get-RunnerArguments {
    param(
        [Parameter(Mandatory = $true)]$Catalog,
        [Parameter(Mandatory = $true)]$StageDefinition,
        [Parameter(Mandatory = $true)]$Scenario,
        [Parameter(Mandatory = $true)]$Configuration,
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)]$HardwareToolchain
    )

    $arguments = @($Catalog.run_contract.arguments | ForEach-Object { [string]$_ })
    $arguments += @(
        '--stage', $Stage,
        '--scenario-id', [string]$Scenario.id,
        '--mechanism-id', [string]$Configuration.id,
        '--seed-set-identity', [string]$StageDefinition.seed_set_identity,
        '--seeds', (($StageDefinition.seeds | ForEach-Object { [string]$_ }) -join ','),
        '--brain-class', $BrainClass,
        '--population', [string]$Population,
        '--source-commit', $Source,
        '--scenario-corpus-version', [string]$Scenario.corpus_version,
        '--adapter-identity', [string]$HardwareToolchain.adapter_identity,
        '--driver-identity', [string]$HardwareToolchain.driver_identity,
        '--os-identity', [string]$HardwareToolchain.os_identity,
        '--rustc-identity', [string]$HardwareToolchain.rustc_identity,
        '--wgpu-identity', [string]$HardwareToolchain.wgpu_identity
    )
    foreach ($mechanismName in @($Configuration.mechanisms.PSObject.Properties.Name)) {
        $arguments += @('--mechanism-' + $mechanismName, ([bool]$Configuration.mechanisms.$mechanismName).ToString().ToLowerInvariant())
    }
    return $arguments
}

function Get-RunnerCommandIdentity {
    if ($RunnerCommand.Count -eq 0) {
        return @('runner-not-configured')
    }
    return @($RunnerCommand | ForEach-Object { [string]$_ })
}

function New-CacheIdentity {
    param(
        [Parameter(Mandatory = $true)]$Scenario,
        [Parameter(Mandatory = $true)]$Configuration,
        [Parameter(Mandatory = $true)]$StageDefinition,
        [Parameter(Mandatory = $true)][string]$Source,
        [Parameter(Mandatory = $true)][string]$SourceDigest,
        [Parameter(Mandatory = $true)]$HardwareToolchain,
        [Parameter(Mandatory = $true)][string[]]$RunnerArguments,
        [Parameter(Mandatory = $true)][string[]]$RunnerCommandIdentity,
        [Parameter(Mandatory = $true)][string]$RunnerIdentity
    )

    $configurationMaterial = [ordered]@{
        schema_version = $ManifestSchemaVersion
        mechanisms = $Configuration.mechanisms
        runner_id = $RunnerIdentity
        runner_command = $RunnerCommandIdentity
        runner_arguments = $RunnerArguments
    }
    $configurationCanonical = ConvertTo-CanonicalJson $configurationMaterial
    $configurationHash = Get-Sha256Text $configurationCanonical

    $identityMaterial = [ordered]@{
        schema_version = $ManifestSchemaVersion
        source_commit = $Source
        source_digest = $SourceDigest
        configuration_hash = $configurationHash
        scenario_id = [string]$Scenario.id
        scenario_corpus_version = [string]$Scenario.corpus_version
        seed_set = [ordered]@{
            identity = [string]$StageDefinition.seed_set_identity
            seeds = @($StageDefinition.seeds)
        }
        brain_class = $BrainClass
        population = $Population
        hardware_toolchain = $HardwareToolchain
        runner_id = $RunnerIdentity
        runner_command = $RunnerCommandIdentity
        runner_arguments = $RunnerArguments
    }
    $canonicalIdentity = ConvertTo-CanonicalJson $identityMaterial
    $cacheKey = Get-Sha256Text $canonicalIdentity

    $identityMaterial.cache_key = $cacheKey
    $identityMaterial.configuration_hash = $configurationHash
    return [pscustomobject][ordered]@{
        cache_key = $cacheKey
        source_commit = $Source
        source_digest = $SourceDigest
        configuration_hash = $configurationHash
        scenario_id = [string]$Scenario.id
        scenario_corpus_version = [string]$Scenario.corpus_version
        seed_set = [ordered]@{
            identity = [string]$StageDefinition.seed_set_identity
            seeds = @($StageDefinition.seeds)
        }
        brain_class = $BrainClass
        population = $Population
        hardware_toolchain = $HardwareToolchain
        runner_id = $RunnerIdentity
        runner_command = $RunnerCommandIdentity
        runner_arguments = $RunnerArguments
        canonical = $canonicalIdentity
    }
}

function New-Manifest {
    param(
        [Parameter(Mandatory = $true)]$Scenario,
        [Parameter(Mandatory = $true)]$Configuration,
        [Parameter(Mandatory = $true)]$StageDefinition,
        [Parameter(Mandatory = $true)]$Identity,
        [Parameter(Mandatory = $true)][string]$ManifestPath,
        [Parameter(Mandatory = $true)][string]$ReceiptPath,
        [Parameter(Mandatory = $true)][string]$RawOutputPath,
        [Parameter(Mandatory = $true)][string[]]$RunnerArguments,
        [Parameter(Mandatory = $true)][string]$RunnerIdentity,
        [Parameter(Mandatory = $true)][string[]]$CognitiveWorkFields
    )

    $support = [string]$Scenario.support
    $status = if ($support.ToLowerInvariant() -in @('supported', 'available')) { 'planned' } else { 'unavailable' }
    return [ordered]@{
        schema_version = $ManifestSchemaVersion
        source_commit = [string]$Identity.source_commit
        configuration_hash = [string]$Identity.configuration_hash
        scenario_corpus_version = [string]$Scenario.corpus_version
        seed_set = [ordered]@{
            identity = [string]$StageDefinition.seed_set_identity
            seeds = @($StageDefinition.seeds)
        }
        brain_class = $BrainClass
        population = $Population
        hardware_toolchain = $Identity.hardware_toolchain
        mechanisms = $Configuration.mechanisms
        stage = $Stage
        command = [ordered]@{
            runner_id = $RunnerIdentity
            arguments = @($RunnerArguments)
        }
        artifacts = [ordered]@{
            raw_output_path = $RawOutputPath
            receipt_path = $ReceiptPath
        }
        metrics = [ordered]@{
            capability = @([string]$Scenario.capability_metric)
            cognitive_work_fields = @($CognitiveWorkFields | ForEach-Object { [string]$_ })
        }
        outcome = [ordered]@{
            status = $status
            execution_state = 'not_run'
        }
        cache_lineage = [ordered]@{
            source_digest = [string]$Identity.source_digest
            configuration_hash = [string]$Identity.configuration_hash
            parent_receipt_path = $null
        }
    }
}

function Assert-ExactStringArray {
    param(
        [Parameter(Mandatory = $true)]$Actual,
        [Parameter(Mandatory = $true)][string[]]$Expected,
        [Parameter(Mandatory = $true)][string]$Name
    )

    $actualValues = @($Actual | ForEach-Object { [string]$_ })
    if ($actualValues.Count -ne $Expected.Count) {
        Stop-Ei1Ablation "invalid receipt: $Name does not match the planned value"
    }
    for ($index = 0; $index -lt $Expected.Count; $index++) {
        if ($actualValues[$index] -ne $Expected[$index]) {
            Stop-Ei1Ablation "invalid receipt: $Name does not match the planned value"
        }
    }
}

function Assert-NoUnexpectedProperties {
    param(
        [Parameter(Mandatory = $true)]$Object,
        [Parameter(Mandatory = $true)][string[]]$Allowed,
        [Parameter(Mandatory = $true)][string]$Name
    )

    $actual = @($Object.PSObject.Properties.Name)
    $unexpected = @($actual | Where-Object { $_ -notin $Allowed })
    if ($unexpected.Count -ne 0) {
        Stop-Ei1Ablation "invalid receipt: $Name contains unexpected fields $($unexpected -join ', ')"
    }
}

function Assert-ExecutedManifest {
    param(
        [Parameter(Mandatory = $true)]$Receipt,
        [Parameter(Mandatory = $true)]$ExpectedManifest,
        [Parameter(Mandatory = $true)]$Identity,
        [Parameter(Mandatory = $true)][string[]]$MechanismNames
    )

    if ($null -eq $Receipt -or $Receipt -is [array]) {
        Stop-Ei1Ablation 'invalid receipt: output is not one manifest object'
    }
    Assert-NoUnexpectedProperties -Object $Receipt -Allowed @('schema_version', 'source_commit', 'configuration_hash', 'scenario_corpus_version', 'seed_set', 'brain_class', 'population', 'hardware_toolchain', 'mechanisms', 'stage', 'command', 'artifacts', 'metrics', 'outcome', 'cache_lineage') -Name 'manifest'
    foreach ($field in @('schema_version', 'source_commit', 'configuration_hash', 'scenario_corpus_version', 'brain_class', 'population', 'stage')) {
        if ([string](Get-PropertyValue $Receipt $field) -ne [string](Get-PropertyValue $ExpectedManifest $field)) {
            Stop-Ei1Ablation "invalid receipt: $field does not match the planned manifest"
        }
    }
    if ([string]$Receipt.schema_version -ne $ManifestSchemaVersion) {
        Stop-Ei1Ablation 'invalid receipt: schema_version is not pass2-experiment-manifest-v1'
    }
    if ([int]$Receipt.population -ne [int]$ExpectedManifest.population) {
        Stop-Ei1Ablation 'invalid receipt: population does not match the planned manifest'
    }

    Assert-NoUnexpectedProperties -Object $Receipt.seed_set -Allowed @('identity', 'seeds') -Name 'seed_set'
    if ([string]$Receipt.seed_set.identity -ne [string]$ExpectedManifest.seed_set.identity) {
        Stop-Ei1Ablation 'invalid receipt: seed-set identity does not match'
    }
    Assert-ExactStringArray -Actual $Receipt.seed_set.seeds -Expected @($ExpectedManifest.seed_set.seeds | ForEach-Object { [string]$_ }) -Name 'seed_set.seeds'

    Assert-NoUnexpectedProperties -Object $Receipt.hardware_toolchain -Allowed @('adapter_identity', 'driver_identity', 'os_identity', 'rustc_identity', 'wgpu_identity') -Name 'hardware_toolchain'
    foreach ($field in @('adapter_identity', 'driver_identity', 'os_identity', 'rustc_identity', 'wgpu_identity')) {
        if ([string](Get-PropertyValue $Receipt.hardware_toolchain $field) -ne [string](Get-PropertyValue $ExpectedManifest.hardware_toolchain $field)) {
            Stop-Ei1Ablation "invalid receipt: hardware_toolchain.$field does not match"
        }
    }

    Assert-NoUnexpectedProperties -Object $Receipt.mechanisms -Allowed $MechanismNames -Name 'mechanisms'
    foreach ($mechanismName in $MechanismNames) {
        $actual = Get-PropertyValue $Receipt.mechanisms $mechanismName
        $expected = Get-PropertyValue $ExpectedManifest.mechanisms $mechanismName
        if ($actual -isnot [bool] -or [bool]$actual -ne [bool]$expected) {
            Stop-Ei1Ablation "invalid receipt: mechanism $mechanismName does not match"
        }
    }

    Assert-NoUnexpectedProperties -Object $Receipt.command -Allowed @('runner_id', 'arguments') -Name 'command'
    if ([string]$Receipt.command.runner_id -ne [string]$ExpectedManifest.command.runner_id) {
        Stop-Ei1Ablation 'invalid receipt: command.runner_id does not match'
    }
    Assert-ExactStringArray -Actual $Receipt.command.arguments -Expected @($ExpectedManifest.command.arguments | ForEach-Object { [string]$_ }) -Name 'command.arguments'

    Assert-NoUnexpectedProperties -Object $Receipt.artifacts -Allowed @('raw_output_path', 'receipt_path') -Name 'artifacts'
    if ([string]$Receipt.artifacts.raw_output_path -ne [string]$ExpectedManifest.artifacts.raw_output_path -or [string]$Receipt.artifacts.receipt_path -ne [string]$ExpectedManifest.artifacts.receipt_path) {
        Stop-Ei1Ablation 'invalid receipt: artifact paths do not match'
    }

    Assert-NoUnexpectedProperties -Object $Receipt.metrics -Allowed @('capability', 'cognitive_work_fields') -Name 'metrics'
    Assert-ExactStringArray -Actual $Receipt.metrics.capability -Expected @($ExpectedManifest.metrics.capability | ForEach-Object { [string]$_ }) -Name 'metrics.capability'
    Assert-ExactStringArray -Actual $Receipt.metrics.cognitive_work_fields -Expected @($ExpectedManifest.metrics.cognitive_work_fields | ForEach-Object { [string]$_ }) -Name 'metrics.cognitive_work_fields'

    Assert-NoUnexpectedProperties -Object $Receipt.outcome -Allowed @('status', 'execution_state') -Name 'outcome'
    if ([string]$Receipt.outcome.execution_state -ne 'executed') {
        Stop-Ei1Ablation 'invalid receipt: outcome.execution_state does not prove execution'
    }
    $status = [string]$Receipt.outcome.status
    if ($status -in @('causal_failure', 'invalid', 'diverged', 'evaluator_corrupt')) {
        Stop-Ei1Ablation "execution stopped on evaluator status: $status"
    }
    if ($status -ne 'completed') {
        Stop-Ei1Ablation "execution stopped on unsupported evaluator status: $status"
    }

    Assert-NoUnexpectedProperties -Object $Receipt.cache_lineage -Allowed @('source_digest', 'configuration_hash', 'parent_receipt_path') -Name 'cache_lineage'
    if ([string]$Receipt.cache_lineage.source_digest -ne [string]$ExpectedManifest.cache_lineage.source_digest -or [string]$Receipt.cache_lineage.configuration_hash -ne [string]$Identity.configuration_hash) {
        Stop-Ei1Ablation 'invalid receipt: cache lineage does not match the planned identity'
    }
    if ($null -ne $Receipt.cache_lineage.parent_receipt_path) {
        Stop-Ei1Ablation 'invalid receipt: executed output must not claim an unplanned parent receipt'
    }
}

function Assert-RawOutput {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        Stop-Ei1Ablation "evaluator corruption: raw output was not produced at $Path"
    }
    $raw = Get-Content -Raw -LiteralPath $Path
    if ($raw -match '(?i)(?<![A-Za-z_])(?:NaN|Infinity|-Infinity|diverged|divergence)(?![A-Za-z_])') {
        Stop-Ei1Ablation 'execution stopped on NaN, infinity, or divergence in evaluator output'
    }
    return $raw
}

function Write-JsonFile {
    param(
        [Parameter(Mandatory = $true)]$Object,
        [Parameter(Mandatory = $true)][string]$Path
    )

    $parent = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
    $temporary = "$Path.tmp-$PID"
    [System.IO.File]::WriteAllText($temporary, (ConvertTo-CanonicalJson $Object) + [Environment]::NewLine, [System.Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporary -Destination $Path -Force
}

function Get-CacheReceipt {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)]$ExpectedManifest,
        [Parameter(Mandatory = $true)]$Identity,
        [Parameter(Mandatory = $true)][string[]]$MechanismNames
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject][ordered]@{ state = 'miss'; receipt = $null }
    }
    try {
        $receipt = Get-Content -Raw -LiteralPath $Path | ConvertFrom-Json
        Assert-ExecutedManifest -Receipt $receipt -ExpectedManifest $ExpectedManifest -Identity $Identity -MechanismNames $MechanismNames
        Assert-RawOutput -Path ([string]$ExpectedManifest.artifacts.raw_output_path) | Out-Null
        return [pscustomobject][ordered]@{ state = 'hit'; receipt = $receipt }
    }
    catch {
        Stop-Ei1Ablation "invalid receipt: cache entry $Path is not an accepted executed result. $($_.Exception.Message)"
    }
}

function Invoke-ConfiguredRunner {
    param(
        [Parameter(Mandatory = $true)]$PlanEntry,
        [Parameter(Mandatory = $true)][string]$ManifestPath
    )

    $program = [string]$RunnerCommand[0]
    $arguments = @()
    if ($RunnerCommand.Count -gt 1) {
        $arguments += @($RunnerCommand[1..($RunnerCommand.Count - 1)])
    }
    $arguments += @(
        '--manifest', $ManifestPath,
        '--cache-key', [string]$PlanEntry.cache_identity.cache_key,
        '--configuration-hash', [string]$PlanEntry.cache_identity.configuration_hash,
        '--source-commit', [string]$PlanEntry.cache_identity.source_commit,
        '--scenario-corpus-version', [string]$PlanEntry.cache_identity.scenario_corpus_version,
        '--seed-set-identity', [string]$PlanEntry.cache_identity.seed_set.identity,
        '--seeds', (($PlanEntry.cache_identity.seed_set.seeds | ForEach-Object { [string]$_ }) -join ','),
        '--brain-class', [string]$PlanEntry.cache_identity.brain_class,
        '--population', [string]$PlanEntry.cache_identity.population,
        '--receipt-output', [string]$PlanEntry.manifest.artifacts.receipt_path,
        '--raw-output', [string]$PlanEntry.manifest.artifacts.raw_output_path
    )

    $runnerOutput = if ($arguments.Count -eq 0) {
        @(& $program 2>&1)
    }
    else {
        @(& $program @arguments 2>&1)
    }
    $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }
    if ($exitCode -ne 0) {
        Stop-Ei1Ablation "configured runner $RunnerId failed with exit code $exitCode. No receipt was accepted. Output: $($runnerOutput -join [Environment]::NewLine)"
    }
}

if ([string]::IsNullOrWhiteSpace($ScenarioContractPath)) {
    $ScenarioContractPath = Join-Path $PSScriptRoot 'pass2_ei1_scenarios.json'
}
if ([string]::IsNullOrWhiteSpace($ManifestSchemaPath)) {
    $ManifestSchemaPath = Join-Path $PSScriptRoot 'pass2_experiment_manifest.schema.json'
}

$catalog = Assert-ContractFile -Path $ScenarioContractPath
$manifestSchema = Assert-ContractFile -Path $ManifestSchemaPath
$requiredManifestFields = Assert-ManifestSchema -Schema $manifestSchema
$mechanismProperties = Get-PropertyValue (Get-PropertyValue $manifestSchema.properties 'mechanisms') 'properties'
$mechanismNames = @($mechanismProperties.PSObject.Properties.Name | ForEach-Object { [string]$_ })
if ($mechanismNames.Count -eq 0) {
    Stop-Ei1Ablation 'manifest schema has no mechanism properties'
}
Assert-ScenarioCatalog -Catalog $catalog -ScenarioPath $ScenarioContractPath -MechanismNames $mechanismNames

$source = Get-SourceCommitValue
$sourceDigest = Get-Sha256Text "source_commit=$source"
$stageDefinition = Get-StageDefinition -Catalog $catalog
$hardwareToolchain = Get-HardwareToolchain
$configurations = Get-ConfigurationCatalog -Catalog $catalog -MechanismNames $mechanismNames
$selectedScenarios = Get-SelectedScenarios -Catalog $catalog
$plannedRunnerId = [string]$catalog.run_contract.runner_id
$effectiveRunnerId = if ([string]::IsNullOrWhiteSpace($RunnerId)) { $plannedRunnerId } else { $RunnerId.Trim() }
$runnerCommandIdentity = Get-RunnerCommandIdentity
$cognitiveWorkFields = @($catalog.run_contract.cognitive_work_fields | ForEach-Object { [string]$_ })
if ($cognitiveWorkFields.Count -eq 0) {
    Stop-Ei1Ablation 'scenario catalog has no cognitive work fields'
}

if ($Mode -eq 'execute') {
    if ([string]::IsNullOrWhiteSpace($RunnerId) -or $RunnerCommand.Count -eq 0) {
        Stop-Ei1Ablation 'execute requested without an explicit real runner. Supply both -RunnerId and -RunnerCommand. No experiment was started.'
    }
    $resolvedRunner = Get-Command $RunnerCommand[0] -ErrorAction SilentlyContinue
    if ($null -eq $resolvedRunner -or $resolvedRunner.CommandType -notin @('Application', 'ExternalScript')) {
        Stop-Ei1Ablation "configured runner is not an executable or script: $($RunnerCommand[0]). No experiment was started."
    }
}

$plans = [System.Collections.Generic.List[object]]::new()
$halted = $false
$haltReason = $null

foreach ($scenario in $selectedScenarios) {
    $requiredConfigurations = Get-RequiredConfigurations -Scenario $scenario -Configurations $configurations
    $matchedGroup = "$Stage/$($scenario.id)/$($stageDefinition.seed_set_identity)/$BrainClass/$Population"
    foreach ($configuration in $requiredConfigurations) {
        $runnerArguments = Get-RunnerArguments -Catalog $catalog -StageDefinition $stageDefinition -Scenario $scenario -Configuration $configuration -Source $source -HardwareToolchain $hardwareToolchain
        $identity = New-CacheIdentity -Scenario $scenario -Configuration $configuration -StageDefinition $stageDefinition -Source $source -SourceDigest $sourceDigest -HardwareToolchain $hardwareToolchain -RunnerArguments $runnerArguments -RunnerCommandIdentity $runnerCommandIdentity -RunnerIdentity $effectiveRunnerId
        $manifestPath = Join-Path (Get-AbsolutePath $ArtifactRoot) ($identity.cache_key + '.manifest.json')
        $receiptPath = Join-Path (Get-AbsolutePath $ArtifactRoot) ($identity.cache_key + '.receipt.json')
        $rawOutputPath = Join-Path (Get-AbsolutePath $ArtifactRoot) ($identity.cache_key + '.raw.jsonl')
        $manifest = New-Manifest -Scenario $scenario -Configuration $configuration -StageDefinition $stageDefinition -Identity $identity -ManifestPath $manifestPath -ReceiptPath $receiptPath -RawOutputPath $rawOutputPath -RunnerArguments $runnerArguments -RunnerIdentity $effectiveRunnerId -CognitiveWorkFields $cognitiveWorkFields
        $planEntry = [ordered]@{
            matched_group = $matchedGroup
            scenario_id = [string]$scenario.id
            scenario_support = [string]$scenario.support
            unavailable_reason = if ([string]$scenario.support -in @('supported', 'available')) { $null } else { [string]$scenario.unavailable_reason }
            required_mechanisms = @($scenario.required_mechanisms | ForEach-Object { [string]$_ })
            mechanism_id = [string]$configuration.id
            disabled_mechanism = $configuration.disabled_mechanism
            manifest_path = $manifestPath
            cache_receipt_path = Join-Path (Get-AbsolutePath $CacheRoot) ($identity.cache_key + '.receipt.json')
            cache_identity = $identity
            manifest = $manifest
            execution = [ordered]@{ state = 'not_run'; accepted = $false; cache_lookup = 'not_checked' }
        }

        if ($Mode -eq 'execute' -and [string]$scenario.support -in @('supported', 'available')) {
            $cache = Get-CacheReceipt -Path $planEntry.cache_receipt_path -ExpectedManifest $manifest -Identity $identity -MechanismNames $mechanismNames
            $planEntry.execution.cache_lookup = $cache.state
            if ($cache.state -eq 'hit') {
                $planEntry.manifest = $cache.receipt
                $planEntry.execution.state = 'executed'
                $planEntry.execution.accepted = $true
                $plans.Add([pscustomobject]$planEntry)
                continue
            }

            Write-JsonFile -Object $manifest -Path $manifestPath
            Invoke-ConfiguredRunner -PlanEntry ([pscustomobject]$planEntry) -ManifestPath $manifestPath
            if (-not (Test-Path -LiteralPath $receiptPath -PathType Leaf)) {
                Stop-Ei1Ablation "evaluator corruption: runner completed without receipt $receiptPath"
            }
            Assert-RawOutput -Path $rawOutputPath | Out-Null
            try {
                $executedReceipt = Get-Content -Raw -LiteralPath $receiptPath | ConvertFrom-Json
            }
            catch {
                Stop-Ei1Ablation "invalid receipt: runner output at $receiptPath is not valid JSON"
            }
            Assert-ExecutedManifest -Receipt $executedReceipt -ExpectedManifest $manifest -Identity $identity -MechanismNames $mechanismNames
            Copy-Item -LiteralPath $receiptPath -Destination $planEntry.cache_receipt_path -Force
            $planEntry.manifest = $executedReceipt
            $planEntry.execution.state = 'executed'
            $planEntry.execution.accepted = $true
        }
        elseif ($Mode -eq 'execute') {
            $planEntry.execution.cache_lookup = 'unavailable'
            $planEntry.execution.state = 'not_run'
        }

        $plans.Add([pscustomobject]$planEntry)
    }
}

$unsupported = @($plans | Where-Object { $_.scenario_support -notin @('supported', 'available') } | Select-Object -ExpandProperty scenario_id -Unique)
$executedCount = @($plans | Where-Object { $_.execution.state -eq 'executed' }).Count
$report = [ordered]@{
    schema_version = $PlanSchemaVersion
    manifest_schema_version = $ManifestSchemaVersion
    mode = $Mode
    stage = $Stage
    execution_state = if ($executedCount -eq 0) { 'not_run' } else { 'executed' }
    source_contracts = [ordered]@{
        scenario_catalog = Get-AbsolutePath $ScenarioContractPath
        manifest_schema = Get-AbsolutePath $ManifestSchemaPath
    }
    source = [ordered]@{
        commit = $source
        digest = $sourceDigest
    }
    selection = [ordered]@{
        requested_scenario_ids = @($ScenarioIds | ForEach-Object { [string]$_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
        resolved_scenario_ids = @($selectedScenarios | ForEach-Object { [string]$_.id })
        scenario_scope = $stageDefinition.scenario_scope
        configuration_scope = $stageDefinition.configuration_scope
        seed_set = [ordered]@{
            identity = $stageDefinition.seed_set_identity
            seeds = @($stageDefinition.seeds)
        }
        matched_expansion_rules = @(
            'Every selected scenario expands to full_system plus one catalog ablation for each required_mechanism.',
            'All members of a matched group share stage, scenario, corpus version, common seed identity and seeds, brain class, population, hardware/toolchain, runner identity, and runner arguments.',
            'Confirm requires explicit -ScenarioIds and expands only those selected scenario groups.',
            'Unsupported scenarios are emitted as unavailable/not_run manifests with their catalog unavailable_reason.'
        )
    }
    runner = [ordered]@{
        planned_id = $plannedRunnerId
        requested_id = if ([string]::IsNullOrWhiteSpace($RunnerId)) { $null } else { $RunnerId.Trim() }
        configured = ($RunnerCommand.Count -gt 0 -and -not [string]::IsNullOrWhiteSpace($RunnerId))
        command = $runnerCommandIdentity
        boundary = 'The catalog runner ID is a planned Task 3 identifier only. This source snapshot declares no implemented Task 3 behavioural evaluator; plan output remains unavailable/not_run until an explicit real runner is supplied.'
    }
    stop_policy = @($catalog.stage_policy.early_stop | ForEach-Object { [string]$_ })
    summary = [ordered]@{
        manifest_count = @($plans).Count
        unsupported_scenario_count = $unsupported.Count
        executed_count = $executedCount
        results_or_reward_labels = 'not fabricated'
    }
    manifests = @($plans | ForEach-Object { $_.manifest })
    plans = @($plans)
    halted = $halted
    halt_reason = $haltReason
}

$report | ConvertTo-Json -Depth 40
