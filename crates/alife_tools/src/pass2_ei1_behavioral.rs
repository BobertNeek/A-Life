//! Source-bound Pass 2 EI1 behavioural execution through the production Era1 runner.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use alife_core::{
    BrainCapacityClass, CreatureGenome, Era1Ability, Era1Control, Era1EvidencePartition,
    FoundationGeneticIdentity, MetricReading, OrganismId,
};
use alife_training::{Era1TrialRunEvidence, Era1TrialRunRequest, Era1TrialRunner};
use alife_world::{Era1TrialManifest, Era1WorldFamily};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PASS2_EI1_RUNNER_ID: &str = "pass2_ei1_behavioral_harness_v1";
const MANIFEST_SCHEMA_VERSION: &str = "pass2-experiment-manifest-v1";
const CATALOG_VERSION: &str = "pass2-ei1-scenarios-v1";
const N2048_FOUNDATION_ID: u64 = 0x4E32_3034_385F_5631;
const N2048_FOUNDATION_VERSION: u16 = 1;
const N2048_COMPATIBILITY_FAMILY_ID: u64 = 0x4E32_3034_385F_FA11;
const STARTER_TOKEN_ID: u16 = 41;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct SeedSet {
    identity: String,
    seeds: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct HardwareToolchain {
    identity: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct Mechanisms {
    attention: bool,
    concept_gap_context: bool,
    prediction: bool,
    dendritic_conjunctions: bool,
    structural_plasticity: bool,
    weight_learning: bool,
    sleep_consolidation: bool,
    episodic_recall: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct CommandContract {
    runner_id: String,
    arguments: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct ArtifactContract {
    raw_output_path: String,
    receipt_path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct MetricsContract {
    capability: Vec<String>,
    cognitive_work_fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct OutcomeContract {
    status: String,
    execution_state: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct CacheLineage {
    source_digest: String,
    configuration_hash: String,
    parent_receipt_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
struct PlannedManifest {
    schema_version: String,
    source_commit: String,
    configuration_hash: String,
    scenario_corpus_version: String,
    seed_set: SeedSet,
    brain_class: String,
    population: u64,
    hardware_toolchain: HardwareToolchain,
    mechanisms: Mechanisms,
    stage: String,
    command: CommandContract,
    artifacts: ArtifactContract,
    metrics: MetricsContract,
    outcome: OutcomeContract,
    cache_lineage: CacheLineage,
}

#[derive(Debug, Clone, Deserialize)]
struct Catalog {
    catalog_version: String,
    execution_state: String,
    experiment_manifest_schema: String,
    common_seed_sets: Vec<CommonSeedSet>,
    run_contract: RunContract,
    mechanism_configurations: Vec<MechanismConfiguration>,
    scenarios: Vec<Scenario>,
    stage_policy: BTreeMap<String, StagePolicy>,
}

#[derive(Debug, Clone, Deserialize)]
struct CommonSeedSet {
    identity: String,
    seeds: Vec<u64>,
    intended_stage: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RunContract {
    brain_class: String,
    population: u64,
    runner_id: String,
    arguments: Vec<String>,
    cognitive_work_fields: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct MechanismConfiguration {
    id: String,
    attention: bool,
    concept_gap_context: bool,
    prediction: bool,
    dendritic_conjunctions: bool,
    structural_plasticity: bool,
    weight_learning: bool,
    sleep_consolidation: bool,
    episodic_recall: bool,
}

impl MechanismConfiguration {
    fn mechanisms(&self) -> Mechanisms {
        Mechanisms {
            attention: self.attention,
            concept_gap_context: self.concept_gap_context,
            prediction: self.prediction,
            dendritic_conjunctions: self.dendritic_conjunctions,
            structural_plasticity: self.structural_plasticity,
            weight_learning: self.weight_learning,
            sleep_consolidation: self.sleep_consolidation,
            episodic_recall: self.episodic_recall,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Scenario {
    id: String,
    corpus_version: String,
    support: String,
    unavailable_reason: Option<String>,
    required_mechanisms: Vec<String>,
    capability_metric: String,
}

#[derive(Debug, Clone, Deserialize)]
struct StagePolicy {
    seed_set_identity: String,
}

#[derive(Debug, Clone)]
struct RunnerOptions {
    planned_manifest: PathBuf,
    manifest_identity: String,
    scenario_catalog: PathBuf,
    scenario_catalog_argument: String,
    manifest_schema: PathBuf,
    manifest_schema_argument: String,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutedReceipt {
    schema_version: String,
    source_commit: String,
    configuration_hash: String,
    scenario_corpus_version: String,
    seed_set: SeedSet,
    brain_class: String,
    population: u64,
    hardware_toolchain: HardwareToolchain,
    mechanisms: Mechanisms,
    stage: String,
    command: CommandContract,
    artifacts: ArtifactContract,
    metrics: MetricsContract,
    outcome: OutcomeContract,
    cache_lineage: CacheLineage,
    execution: ExecutionReceipt,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutionReceipt {
    scenario_id: String,
    configuration_id: String,
    ability: Era1Ability,
    world_family: Era1WorldFamily,
    partition: Era1EvidencePartition,
    capability_metric: String,
    capability_readings: Vec<MetricReading>,
    capability_summary: MetricReading,
    demonstrated_by_seed: Vec<bool>,
    causal_evidence: CausalEvidenceSummary,
    cognitive_work: CognitiveWorkSummary,
}

#[derive(Debug, Clone, Serialize)]
struct CausalEvidenceSummary {
    runs: u64,
    gpu_dispatches: u64,
    sealed_outcomes: u64,
    memory_context_dispatches: u64,
    learning_commits: u64,
    eligibility_discards: u64,
    memory_updates: u64,
    sleep_commits: u64,
    adapters: Vec<String>,
    backend_apis: Vec<String>,
    source_commit: String,
    source_tree: String,
}

#[derive(Debug, Clone, Serialize)]
struct CognitiveWorkSummary {
    measured: BTreeMap<String, u64>,
    unmeasured_fields: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct ScenarioMapping {
    ability: Era1Ability,
    family: Era1WorldFamily,
    partition: Era1EvidencePartition,
}

pub fn run(args: Vec<String>) -> Result<(), String> {
    let options = parse_args(args)?;
    execute(options)
}

fn parse_args(args: Vec<String>) -> Result<RunnerOptions, String> {
    let mut planned_manifest = None;
    let mut manifest_identity = None;
    let mut scenario_catalog = None;
    let mut scenario_catalog_argument = None;
    let mut manifest_schema = None;
    let mut manifest_schema_argument = None;
    let mut index = 0;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value"))?
            .clone();
        match flag {
            "--planned-manifest" => {
                if planned_manifest.is_some() {
                    return Err("duplicate --planned-manifest".to_owned());
                }
                planned_manifest = Some(PathBuf::from(value));
            }
            "--manifest-identity" => {
                if manifest_identity.is_some() {
                    return Err("duplicate --manifest-identity".to_owned());
                }
                manifest_identity = Some(value);
            }
            "--scenario-catalog" => {
                if scenario_catalog.is_some() {
                    return Err("duplicate --scenario-catalog".to_owned());
                }
                scenario_catalog_argument = Some(value.clone());
                scenario_catalog = Some(PathBuf::from(value));
            }
            "--manifest-schema" => {
                if manifest_schema.is_some() {
                    return Err("duplicate --manifest-schema".to_owned());
                }
                manifest_schema_argument = Some(value.clone());
                manifest_schema = Some(PathBuf::from(value));
            }
            other => return Err(format!("unknown argument {other}")),
        }
        index += 2;
    }
    let planned_manifest = planned_manifest.ok_or("--planned-manifest is required")?;
    let manifest_identity = manifest_identity.ok_or("--manifest-identity is required")?;
    let scenario_catalog_argument =
        scenario_catalog_argument.ok_or("--scenario-catalog is required")?;
    let manifest_schema_argument = manifest_schema_argument.ok_or("--manifest-schema is required")?;
    Ok(RunnerOptions {
        planned_manifest,
        manifest_identity,
        scenario_catalog: resolve_workspace_path(&scenario_catalog.unwrap()),
        scenario_catalog_argument,
        manifest_schema: resolve_workspace_path(&manifest_schema.unwrap()),
        manifest_schema_argument,
    })
}

fn execute(options: RunnerOptions) -> Result<(), String> {
    validate_manifest_identity_argument(&options)?;
    let planned_text = fs::read_to_string(&options.planned_manifest)
        .map_err(|error| format!("read planned manifest: {error}"))?;
    let manifest: PlannedManifest = serde_json::from_str(&planned_text)
        .map_err(|error| format!("decode planned manifest: {error}"))?;
    let catalog: Catalog = read_json(&options.scenario_catalog, "scenario catalog")?;
    let schema: Value = read_json(&options.manifest_schema, "manifest schema")?;
    validate_schema_contract(&schema)?;
    validate_catalog_contract(&catalog, &options.manifest_schema)?;
    validate_planned_manifest(&manifest, &catalog, &schema, &options)?;

    let source_commit = git_output("rev-parse", "HEAD")?;
    let source_tree = git_output("rev-parse", "HEAD^{tree}")?;
    if manifest.source_commit != source_commit {
        return Err("planned manifest source_commit does not match current Git HEAD".to_owned());
    }
    if manifest.cache_lineage.source_digest != source_commit {
        return Err("planned cache lineage source digest does not match Git HEAD".to_owned());
    }

    let (scenario_id, configuration_id) = manifest_scenario_and_configuration(&manifest)?;
    let scenario = catalog
        .scenarios
        .iter()
        .find(|scenario| scenario.id == scenario_id)
        .ok_or_else(|| format!("unknown scenario {scenario_id}"))?;
    let mapping = supported_mapping(&scenario_id)
        .ok_or_else(|| format!("scenario {scenario_id} has no supported Era1 mapping"))?;
    if scenario.support != "available" {
        return Err(format!(
            "scenario {scenario_id} is unavailable: {}",
            scenario
                .unavailable_reason
                .as_deref()
                .unwrap_or("catalog does not provide a truthful unavailable reason")
        ));
    }

    let output_paths = validate_output_paths(&options, &manifest)?;
    let foundation = FoundationGeneticIdentity::new(
        N2048_FOUNDATION_ID,
        N2048_FOUNDATION_VERSION,
        N2048_COMPATIBILITY_FAMILY_ID,
        BrainCapacityClass::N2048_ID,
    )
    .map_err(|error| format!("N2048 foundation identity: {error}"))?;
    let mut runner = Era1TrialRunner::new_required()
        .map_err(|error| format!("production Era1 runner unavailable: {error}"))?;
    let mut evidence = Vec::with_capacity(manifest.seed_set.seeds.len());
    for (seed_index, seed) in manifest.seed_set.seeds.iter().copied().enumerate() {
        let organism_id = deterministic_organism_id(seed, seed_index as u64, 0xA1);
        let familiar_peer = deterministic_organism_id(seed, seed_index as u64, 0xB2);
        let novel_peer = deterministic_organism_id(seed, seed_index as u64, 0xC3);
        let world_variant_id = deterministic_nonzero(seed ^ 0xD4, seed_index as u64);
        let trial_manifest = Era1TrialManifest::new(
            seed,
            mapping.family,
            organism_id,
            familiar_peer,
            novel_peer,
            world_variant_id,
            false,
            STARTER_TOKEN_ID,
        )
        .map_err(|error| format!("construct Era1 trial manifest for seed {seed}: {error}"))?;
        let species_seed = deterministic_nonzero(seed ^ 0xE5, seed_index as u64);
        let genome = CreatureGenome::early_mammal_founder(species_seed, foundation)
            .map_err(|error| format!("construct N2048 organism for seed {seed}: {error}"))?;
        let request = Era1TrialRunRequest::new(
            organism_id,
            0,
            &genome,
            &trial_manifest,
            mapping.ability,
            Era1Control::Intact,
            mapping.partition,
            &source_commit,
            &source_tree,
        )
        .map_err(|error| format!("construct Era1 request for seed {seed}: {error}"))?;
        let run_evidence = runner
            .run(request)
            .map_err(|error| format!("run Era1 trial for seed {seed}: {error}"))?;
        validate_production_evidence(
            &run_evidence,
            &manifest,
            mapping,
            &source_commit,
            &source_tree,
        )?;
        evidence.push(run_evidence);
    }

    let raw = serialize_raw_evidence(&evidence)?;
    let receipt = build_receipt(
        &manifest,
        &scenario_id,
        &configuration_id,
        scenario,
        mapping,
        &evidence,
        &source_commit,
        &source_tree,
    )?;
    let receipt_json = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("serialize executed receipt: {error}"))?;
    fs::write(&output_paths.0, raw).map_err(|error| format!("write raw artifact: {error}"))?;
    fs::write(&output_paths.1, format!("{receipt_json}\n"))
        .map_err(|error| format!("write receipt: {error}"))?;
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("read {label}: {error}"))?;
    serde_json::from_str(&text).map_err(|error| format!("decode {label}: {error}"))
}

fn validate_schema_contract(schema: &Value) -> Result<(), String> {
    if schema
        .get("properties")
        .and_then(|properties| properties.get("schema_version"))
        .and_then(|property| property.get("const"))
        .and_then(Value::as_str)
        != Some(MANIFEST_SCHEMA_VERSION)
    {
        return Err("manifest schema version is not pass2-experiment-manifest-v1".to_owned());
    }
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .ok_or("manifest schema has no required field list")?;
    for field in [
        "schema_version",
        "source_commit",
        "configuration_hash",
        "scenario_corpus_version",
        "seed_set",
        "brain_class",
        "population",
        "hardware_toolchain",
        "mechanisms",
        "stage",
        "command",
        "artifacts",
        "metrics",
        "outcome",
        "cache_lineage",
    ] {
        if !required.iter().any(|value| value.as_str() == Some(field)) {
            return Err(format!("manifest schema is missing required field {field}"));
        }
    }
    Ok(())
}

fn validate_catalog_contract(catalog: &Catalog, schema_path: &Path) -> Result<(), String> {
    if catalog.catalog_version != CATALOG_VERSION || catalog.execution_state != "not_run" {
        return Err("unexpected EI1 scenario catalog version or execution state".to_owned());
    }
    if catalog.run_contract.runner_id != PASS2_EI1_RUNNER_ID
        || catalog.run_contract.brain_class != "N2048"
        || catalog.run_contract.population != 1
        || catalog.run_contract.arguments.len() != 4
        || catalog.run_contract.arguments[0] != "--scenario-catalog"
        || catalog.run_contract.arguments[2] != "--manifest-schema"
    {
        return Err("catalog runner contract does not bind the production N2048 harness".to_owned());
    }
    let catalog_schema = resolve_workspace_path(Path::new(&catalog.experiment_manifest_schema));
    if catalog_schema != *schema_path {
        return Err("manifest schema path does not match the scenario catalog contract".to_owned());
    }
    for seed_set in &catalog.common_seed_sets {
        if seed_set.seeds.len() < 2
            || seed_set.seeds.iter().any(|seed| *seed == 0)
            || seed_set.seeds.windows(2).any(|pair| pair[0] == pair[1])
            || !matches!(seed_set.intended_stage.as_str(), "screen" | "confirm" | "certify")
        {
            return Err(format!("invalid catalog seed set {}", seed_set.identity));
        }
    }
    for scenario in &catalog.scenarios {
        if supported_mapping(&scenario.id).is_none()
            && (scenario.support != "unavailable"
                || scenario
                    .unavailable_reason
                    .as_deref()
                    .is_none_or(str::is_empty))
        {
            return Err(format!(
                "unsupported scenario {} must remain unavailable with a reason",
                scenario.id
            ));
        }
    }
    Ok(())
}

fn validate_planned_manifest(
    manifest: &PlannedManifest,
    catalog: &Catalog,
    schema: &Value,
    options: &RunnerOptions,
) -> Result<(), String> {
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION
        || !valid_lower_hex(&manifest.source_commit, 40)
        || !valid_lower_hex(&manifest.configuration_hash, 64)
        || manifest.cache_lineage.configuration_hash != manifest.configuration_hash
        || manifest.cache_lineage.parent_receipt_path.is_some()
        || manifest.brain_class != "N2048"
        || manifest.population != 1
        || manifest.outcome.status != "planned"
        || manifest.outcome.execution_state != "not_run"
        || manifest.hardware_toolchain.identity.trim().is_empty()
    {
        return Err("planned manifest identity or execution state is invalid".to_owned());
    }
    if manifest.metrics.capability.len() != 1 {
        return Err("planned manifest must declare one capability metric".to_owned());
    }
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .ok_or("manifest schema has no required field list")?;
    if required.len() < 14 {
        return Err("manifest schema required field contract is incomplete".to_owned());
    }
    let (scenario_id, configuration_id) = manifest_scenario_and_configuration(manifest)?;
    let scenario = catalog
        .scenarios
        .iter()
        .find(|scenario| scenario.id == scenario_id)
        .ok_or_else(|| format!("unknown scenario {scenario_id}"))?;
    if scenario.corpus_version != manifest.scenario_corpus_version
        || scenario.capability_metric != manifest.metrics.capability[0]
        || scenario.support != "available"
    {
        return Err(format!("scenario {scenario_id} does not match the planned manifest"));
    }
    let mapping = supported_mapping(&scenario_id)
        .ok_or_else(|| format!("scenario {scenario_id} is not supported by this slice"))?;
    if !scenario.required_mechanisms.is_empty() || configuration_id != "full_system" {
        return Err(format!(
            "unsupported mechanism configuration for {scenario_id}/{configuration_id}"
        ));
    }
    let configuration = catalog
        .mechanism_configurations
        .iter()
        .find(|configuration| configuration.id == configuration_id)
        .ok_or_else(|| format!("unknown configuration {configuration_id}"))?;
    if manifest.mechanisms != configuration.mechanisms()
        || manifest.mechanisms != full_system_mechanisms()
    {
        return Err(format!(
            "mechanism configuration {configuration_id} is not honored by Era1"
        ));
    }
    let stage_policy = catalog
        .stage_policy
        .get(&manifest.stage)
        .ok_or_else(|| format!("unknown stage {}", manifest.stage))?;
    let seed_set = catalog
        .common_seed_sets
        .iter()
        .find(|seed_set| seed_set.identity == stage_policy.seed_set_identity)
        .ok_or_else(|| format!("stage {} has no catalog seed set", manifest.stage))?;
    if manifest.seed_set
        != (SeedSet {
            identity: seed_set.identity.clone(),
            seeds: seed_set.seeds.clone(),
        })
    {
        return Err("planned seed set does not match the catalog stage policy".to_owned());
    }
    let expected_command = vec![
        "--scenario-catalog".to_owned(),
        options.scenario_catalog_argument.clone(),
        "--manifest-schema".to_owned(),
        options.manifest_schema_argument.clone(),
        "--scenario-id".to_owned(),
        scenario_id,
        "--configuration-id".to_owned(),
        configuration_id,
    ];
    if manifest.command.runner_id != PASS2_EI1_RUNNER_ID
        || manifest.command.arguments != expected_command
        || manifest.metrics.cognitive_work_fields != catalog.run_contract.cognitive_work_fields
        || manifest.cache_lineage.source_digest != manifest.source_commit
    {
        return Err("planned runner, metric, or cache identity does not match the catalog".to_owned());
    }
    if mapping.family == Era1WorldFamily::RewardReversal
        && manifest.metrics.capability[0] != "reversal_adaptation_trials"
    {
        return Err("reversal scenario capability metric mismatch".to_owned());
    }
    Ok(())
}

fn manifest_scenario_and_configuration(
    manifest: &PlannedManifest,
) -> Result<(String, String), String> {
    let args = &manifest.command.arguments;
    if args.len() != 8
        || args[0] != "--scenario-catalog"
        || args[2] != "--manifest-schema"
        || args[4] != "--scenario-id"
        || args[6] != "--configuration-id"
        || args[1].is_empty()
        || args[3].is_empty()
    {
        return Err("planned command arguments do not match the EI1 harness contract".to_owned());
    }
    Ok((args[5].clone(), args[7].clone()))
}

fn validate_production_evidence(
    evidence: &Era1TrialRunEvidence,
    planned: &PlannedManifest,
    mapping: ScenarioMapping,
    source_commit: &str,
    source_tree: &str,
) -> Result<(), String> {
    evidence
        .validate_contract()
        .map_err(|error| format!("Era1 evidence contract invalid: {error}"))?;
    if evidence.receipt.ability != mapping.ability
        || evidence.manifest.family != mapping.family
        || evidence.receipt.partition != mapping.partition
        || evidence.receipt.source_commit != source_commit
        || evidence.receipt.source_tree != source_tree
        || evidence.receipt.identity.brain_class_id != BrainCapacityClass::N2048_ID
        || evidence.receipt.backend_api != "vulkan"
        || evidence.gpu_dispatches == 0
        || evidence.sealed_outcomes != evidence.gpu_dispatches
        || evidence.memory_context_dispatches != evidence.gpu_dispatches
        || evidence.steps.is_empty()
        || evidence
            .learning_assessment
            .causal_proof
            .successful_behavior_ticks
            .is_empty()
        || evidence.receipt.score == MetricReading::Unknown
        || planned.brain_class != "N2048"
    {
        return Err(
            "production evidence is missing causal GPU, world, outcome, or learning proof"
                .to_owned(),
        );
    }
    serde_json::to_vec(evidence)
        .map_err(|error| format!("production evidence contains a non-finite metric: {error}"))?;
    Ok(())
}

fn serialize_raw_evidence(evidence: &[Era1TrialRunEvidence]) -> Result<String, String> {
    let mut raw = String::new();
    for (index, item) in evidence.iter().enumerate() {
        if index != 0 {
            raw.push('\n');
        }
        let line = serde_json::to_string(item)
            .map_err(|error| format!("serialize raw causal evidence: {error}"))?;
        raw.push_str(&line);
    }
    raw.push('\n');
    Ok(raw)
}

fn build_receipt(
    manifest: &PlannedManifest,
    scenario_id: &str,
    configuration_id: &str,
    scenario: &Scenario,
    mapping: ScenarioMapping,
    evidence: &[Era1TrialRunEvidence],
    source_commit: &str,
    source_tree: &str,
) -> Result<ExecutedReceipt, String> {
    let capability_readings = evidence
        .iter()
        .map(|item| item.receipt.score)
        .collect::<Vec<_>>();
    let capability_summary = aggregate_metric(&capability_readings)?;
    let adapters = evidence
        .iter()
        .map(|item| item.adapter_name.clone())
        .collect::<Vec<_>>();
    let backend_apis = evidence
        .iter()
        .map(|item| item.backend_api.clone())
        .collect::<Vec<_>>();
    let mut measured = BTreeMap::new();
    measured.insert(
        "neural_updates".to_owned(),
        evidence.iter().map(|item| item.gpu_dispatches).sum(),
    );
    measured.insert(
        "memory_ops".to_owned(),
        evidence
            .iter()
            .map(|item| item.memory_context_dispatches)
            .sum(),
    );
    measured.insert(
        "learning_ops".to_owned(),
        evidence.iter().map(|item| item.learning_commits).sum(),
    );
    measured.insert(
        "sleep_ops".to_owned(),
        evidence
            .iter()
            .map(|item| u64::from(item.sleep_commits))
            .sum(),
    );
    let unmeasured_fields = manifest
        .metrics
        .cognitive_work_fields
        .iter()
        .filter(|field| !measured.contains_key(*field))
        .cloned()
        .collect::<Vec<_>>();
    let causal_evidence = CausalEvidenceSummary {
        runs: evidence.len() as u64,
        gpu_dispatches: evidence.iter().map(|item| item.gpu_dispatches).sum(),
        sealed_outcomes: evidence.iter().map(|item| item.sealed_outcomes).sum(),
        memory_context_dispatches: evidence
            .iter()
            .map(|item| item.memory_context_dispatches)
            .sum(),
        learning_commits: evidence.iter().map(|item| item.learning_commits).sum(),
        eligibility_discards: evidence.iter().map(|item| item.eligibility_discards).sum(),
        memory_updates: evidence.iter().map(|item| item.memory_updates).sum(),
        sleep_commits: evidence
            .iter()
            .map(|item| u64::from(item.sleep_commits))
            .sum(),
        adapters,
        backend_apis,
        source_commit: source_commit.to_owned(),
        source_tree: source_tree.to_owned(),
    };
    Ok(ExecutedReceipt {
        schema_version: manifest.schema_version.clone(),
        source_commit: manifest.source_commit.clone(),
        configuration_hash: manifest.configuration_hash.clone(),
        scenario_corpus_version: manifest.scenario_corpus_version.clone(),
        seed_set: manifest.seed_set.clone(),
        brain_class: manifest.brain_class.clone(),
        population: manifest.population,
        hardware_toolchain: manifest.hardware_toolchain.clone(),
        mechanisms: manifest.mechanisms.clone(),
        stage: manifest.stage.clone(),
        command: manifest.command.clone(),
        artifacts: manifest.artifacts.clone(),
        metrics: manifest.metrics.clone(),
        outcome: OutcomeContract {
            status: "completed".to_owned(),
            execution_state: "executed".to_owned(),
        },
        cache_lineage: manifest.cache_lineage.clone(),
        execution: ExecutionReceipt {
            scenario_id: scenario_id.to_owned(),
            configuration_id: configuration_id.to_owned(),
            ability: mapping.ability,
            world_family: mapping.family,
            partition: mapping.partition,
            capability_metric: scenario.capability_metric.clone(),
            capability_readings,
            capability_summary,
            demonstrated_by_seed: evidence
                .iter()
                .map(|item| item.learning_assessment.demonstrated)
                .collect(),
            causal_evidence,
            cognitive_work: CognitiveWorkSummary {
                measured,
                unmeasured_fields,
            },
        },
    })
}

fn aggregate_metric(readings: &[MetricReading]) -> Result<MetricReading, String> {
    let mut weighted_success = 0_u128;
    let mut exposures = 0_u64;
    for reading in readings {
        let MetricReading::Measured {
            value_q16,
            exposures: reading_exposures,
        } = *reading
        else {
            return Err("capability evidence is unknown".to_owned());
        };
        if value_q16 > 65_535 || reading_exposures == 0 {
            return Err("capability evidence is outside the finite Q16 contract".to_owned());
        }
        weighted_success = weighted_success
            .checked_add(u128::from(value_q16) * u128::from(reading_exposures))
            .ok_or("capability evidence overflow")?;
        exposures = exposures
            .checked_add(reading_exposures)
            .ok_or("capability exposure overflow")?;
    }
    let value_q16 = u32::try_from(
        (weighted_success + u128::from(exposures / 2)) / u128::from(exposures),
    )
    .map_err(|_| "capability summary is outside Q16".to_owned())?;
    Ok(MetricReading::Measured {
        value_q16,
        exposures,
    })
}

fn supported_mapping(scenario_id: &str) -> Option<ScenarioMapping> {
    match scenario_id {
        "few_shot_avoidance" => Some(ScenarioMapping {
            ability: Era1Ability::HazardAvoidance,
            family: Era1WorldFamily::ForagingHazardMaze,
            partition: Era1EvidencePartition::Acquisition,
        }),
        "reversal_learning" => Some(ScenarioMapping {
            ability: Era1Ability::RewardReversal,
            family: Era1WorldFamily::RewardReversal,
            partition: Era1EvidencePartition::ReversalProbe,
        }),
        _ => None,
    }
}

fn full_system_mechanisms() -> Mechanisms {
    Mechanisms {
        attention: true,
        concept_gap_context: true,
        prediction: true,
        dendritic_conjunctions: true,
        structural_plasticity: true,
        weight_learning: true,
        sleep_consolidation: true,
        episodic_recall: true,
    }
}

fn validate_output_paths(
    options: &RunnerOptions,
    manifest: &PlannedManifest,
) -> Result<(PathBuf, PathBuf), String> {
    let root = options
        .planned_manifest
        .parent()
        .ok_or("planned manifest has no artifact root")?;
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("canonicalize planned artifact root: {error}"))?;
    let planned_name = options
        .planned_manifest
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("planned manifest has no valid file name")?;
    let expected_planned = format!("{}.planned.json", options.manifest_identity);
    if planned_name != expected_planned {
        return Err("planned manifest path is substituted or escaped".to_owned());
    }
    let raw = PathBuf::from(&manifest.artifacts.raw_output_path);
    let receipt = PathBuf::from(&manifest.artifacts.receipt_path);
    if !raw.is_absolute() || !receipt.is_absolute() || raw == receipt {
        return Err("artifact paths must be distinct absolute paths".to_owned());
    }
    let expected_raw = format!("{}.raw.jsonl", options.manifest_identity);
    let expected_receipt = format!("{}.receipt.json", options.manifest_identity);
    if raw.file_name().and_then(|name| name.to_str()) != Some(expected_raw.as_str())
        || receipt.file_name().and_then(|name| name.to_str()) != Some(expected_receipt.as_str())
    {
        return Err("artifact path is substituted".to_owned());
    }
    for output in [&raw, &receipt] {
        let parent = output.parent().ok_or("artifact path has no parent")?;
        let canonical_parent = fs::canonicalize(parent)
            .map_err(|error| format!("canonicalize artifact parent: {error}"))?;
        if canonical_parent != canonical_root {
            return Err("artifact path escapes the planned artifact root".to_owned());
        }
        if let Ok(metadata) = fs::symlink_metadata(output) {
            if metadata.file_type().is_symlink() {
                return Err("artifact path is a symlink or reparse target".to_owned());
            }
        }
    }
    Ok((raw, receipt))
}

fn validate_manifest_identity_argument(options: &RunnerOptions) -> Result<(), String> {
    if !valid_lower_hex(&options.manifest_identity, 64) {
        return Err("manifest identity must be a 64-character lower-case hash".to_owned());
    }
    Ok(())
}

fn valid_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn deterministic_organism_id(seed: u64, index: u64, domain: u64) -> OrganismId {
    let mixed = deterministic_nonzero(seed ^ domain, index);
    OrganismId(0x7000_0000_0000_0000 | (mixed & 0x0fff_ffff_ffff_ff00) | (domain & 0xff))
}

fn deterministic_nonzero(seed: u64, salt: u64) -> u64 {
    let mut value = seed ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    value | 1
}

fn resolve_workspace_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root().join(path)
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("alife_tools lives under <workspace>/crates")
        .to_path_buf()
}

fn git_output(command: &str, revision: &str) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(workspace_root())
        .args([command, revision])
        .output()
        .map_err(|error| format!("git {command} {revision}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {command} {revision}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !valid_lower_hex(&value, 40) {
        return Err(format!("git {command} {revision} returned an invalid object ID"));
    }
    Ok(value)
}
